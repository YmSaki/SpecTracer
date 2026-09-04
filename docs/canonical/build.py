#!/usr/bin/env python3
"""build.py -- fragment -> canonical specification.json -> md export.

Rules implemented here come from docs/canonical/CONVERSION.md and
docs/canonical/specification.schema.json. This script does not decide
content; it only stamps IDs, checks coverage, validates against the
schema, and exports md. See CONVERSION.md before changing behaviour.

Commands
--------
build
    Read docs/canonical/fragments/*.json (excluding *.dropped.json),
    assign deterministic IDs (CONVERSION.md SS7), write
    docs/canonical/specification.json, then validate it against
    specification.schema.json. Exit 1 on any schema error.

coverage
    For every source doc referenced by the fragments, read the md file
    and report which lines are NOT accounted for by an item/area source
    range, a dropped-log range, or an auto-excluded line (blank line,
    ATX heading, "---", code-fence delimiter). Also reports (report
    only, does not affect exit code) overlaps between item-level source
    ranges within the same doc; area/item overlap is expected and is
    never reported. Exit 1 if any uncovered line exists.

export
    Read specification.json and write docs/canonical/export/{request,
    require,spec,design}.md.

all
    build, then coverage, then export. Stops at the first failure.

check-fragment <path>
    Validate one fragment file's structural shape (not the final
    schema): required keys present, derived_from == [], no stray "id"
    key, source.lines is a valid [start, end] pair. Exit 1 on any
    problem.

harvest-cites
    Scan every item's "statement" (including design area items) for
    citations that name a document (要件定義/基本仕様/詳細設計/本冊/別紙A/B/C
    section refs, 別紙A/B/C included symmetrically; L-line refs likewise;
    P-00N refs for 要件定義/基本仕様) or a bare cross-cutting code (P-00N,
    R-1..R-5, F<n>, OOS-00N, NFR-00N), and append any not already present
    to that item's "cites" list (surviving entries and their order are
    untouched; new ones are appended in order of first appearance; no
    duplicates). A bare code matched at the very start of the statement
    (e.g. "OOS-001仕様書同士の..." or "NFR-001並列性への対応は...") is the
    statement naming itself and is never harvested. Also drops any
    existing "cites" entry that is a bare same-document section reference
    ("§N", "§N.N", or a joined list of them such as "§4.1・§4.4") per
    CONVERSION.md SS3 -- those stay inline in the statement and are not
    citations; if "cites" would end up empty it is deleted. Never touches
    "statement" or "derived_from". Prints, per fragment file, items
    scanned, citations added, citations removed, and the 10 most common
    newly-added citation strings. A fragment file is rewritten (same
    formatting as the converters: UTF-8, ensure_ascii=False, indent 2,
    trailing newline) whenever anything was added or removed; pass
    --dry-run to only print the report. Bare "§N"/"§N.N" without a
    preceding document name are never captured as new citations either.

ID stamping (CONVERSION.md SS7)
--------------------------------
Items are sorted by (layer order request<require<spec<design, doc
key, source.lines[0]) and stamped in that order with a per-prefix
counter (R for request, REQ for require, SPEC for spec, DES for design
items, DA for design areas), zero-padded to 3 digits (more if needed,
via Python's ":03d" format which never truncates). An item carrying a
"keep_id" field keeps that id verbatim and does not consume a counter
slot; "keep_id" is stripped from the output either way.

For the design layer, "doc key" is not a literal path sort: CONVERSION.md
SS1 fixes an explicit document order, 本冊 -> 別紙A -> 別紙C. This script
implements that as a small rank function (design_doc_rank) rather than
sorting doc path strings. For the other three layers no such override is
specified, so "doc key" is the literal doc path string.

Design items are stamped and ordered with the same explicit rank, in a
single global pass across all areas (not restarted per area), and are
then grouped back under their owning area for output; areas are ordered
and stamped the same way, independently of item stamping.

The rootItem schema (request layer) has no derived_from/cites fields,
so those two keys are dropped from request-layer output even if a
fragment supplies them (fragments use one shared item shape across all
four layers for authoring convenience).
"""

from __future__ import annotations

import argparse
import json
import re
import sys
from collections import Counter
from pathlib import Path

try:
    from jsonschema import Draft202012Validator
except ImportError:  # pragma: no cover - environment problem, not a code path
    print("error: the 'jsonschema' package is required (pip install jsonschema)", file=sys.stderr)
    sys.exit(1)

# Windows consoles often default stdout/stderr to a legacy codepage (e.g.
# cp932), which mangles the Japanese text this script prints. Force UTF-8
# where the stream supports reconfiguration; harmless elsewhere.
for _stream in (sys.stdout, sys.stderr):
    if hasattr(_stream, "reconfigure"):
        try:
            _stream.reconfigure(encoding="utf-8")
        except Exception:
            pass

LAYER_ORDER = {"request": 0, "require": 1, "spec": 2, "design": 3}
LAYERS = tuple(LAYER_ORDER)
LEAF_PREFIX = {"request": "R", "require": "REQ", "spec": "SPEC"}
ID_PATTERN = re.compile(r"^(R-[0-9]+|P-[0-9]{3}|REQ-[0-9]{3,}|SPEC-[0-9]{3,}|DES-[0-9]{3,}|DA-[0-9]{3,})$")
HEADING_RE = re.compile(r"^#{1,6} ")
SCHEMA_VERSION = "0.1"


# ------------------------------------------------------------------ paths --

def fragments_dir(root: Path) -> Path:
    return root / "fragments"


def spec_json_path(root: Path) -> Path:
    return root / "specification.json"


def schema_path(root: Path) -> Path:
    return root / "specification.schema.json"


def export_dir(root: Path) -> Path:
    return root / "export"


# --------------------------------------------------------------- loading --

def read_json(path: Path):
    try:
        text = path.read_text(encoding="utf-8")
    except OSError as exc:
        print(f"error: cannot read {path}: {exc}", file=sys.stderr)
        sys.exit(1)
    try:
        return json.loads(text)
    except json.JSONDecodeError as exc:
        print(f"error: {path} is not valid JSON: {exc}", file=sys.stderr)
        sys.exit(1)


def list_fragment_files(root: Path) -> list[Path]:
    d = fragments_dir(root)
    if not d.is_dir():
        return []
    return sorted(p for p in d.glob("*.json") if not p.name.endswith(".dropped.json"))


def list_dropped_files(root: Path) -> list[Path]:
    d = fragments_dir(root)
    if not d.is_dir():
        return []
    return sorted(d.glob("*.dropped.json"))


def load_fragments(root: Path) -> list[tuple[Path, dict]]:
    out = []
    for p in list_fragment_files(root):
        data = read_json(p)
        if not isinstance(data, dict):
            print(f"error: {p} is not a JSON object", file=sys.stderr)
            sys.exit(1)
        out.append((p, data))
    return out


def load_dropped(root: Path) -> list[tuple[Path, dict]]:
    out = []
    for p in list_dropped_files(root):
        data = read_json(p)
        if not isinstance(data, list):
            print(f"error: {p} must be a JSON array", file=sys.stderr)
            sys.exit(1)
        for entry in data:
            out.append((p, entry))
    return out


# ------------------------------------------------------------- ordering --

def design_doc_rank(doc: str) -> int:
    """CONVERSION.md SS1: 本冊 -> 別紙A -> 別紙C, fixed explicitly."""
    if "別紙A" in doc:
        return 1
    if "別紙C" in doc:
        return 2
    return 0


def doc_key(layer: str, doc: str):
    if layer == "design":
        return (design_doc_rank(doc), doc)
    return (0, doc)


def item_sort_key(layer: str, item: dict):
    src = item["source"]
    return (LAYER_ORDER[layer], doc_key(layer, src["doc"]), src["lines"][0])


def assign_ids(items: list[dict], prefix: str) -> None:
    """Stamp item['_id'] in place, in the given (already sorted) order."""
    counter = 1
    for item in items:
        keep = item.get("keep_id")
        if keep:
            item["_id"] = keep
        else:
            item["_id"] = f"{prefix}-{counter:03d}"
            counter += 1


# ------------------------------------------------------------- building --

def build_source(src: dict) -> dict:
    return {"doc": src["doc"], "heading": src["heading"], "lines": [src["lines"][0], src["lines"][1]]}


def build_leaf_output(item: dict, layer: str) -> dict:
    out = {"id": item["_id"], "statement": item["statement"]}
    if item.get("description"):
        out["description"] = item["description"]
    if layer != "request":
        out["derived_from"] = list(item.get("derived_from") or [])
        cites = item.get("cites")
        if cites:
            out["cites"] = list(cites)
    out["source"] = build_source(item["source"])
    return out


def cmd_build(args) -> int:
    root = Path(args.root)
    fragments = load_fragments(root)

    per_layer_items: dict[str, list[dict]] = {"request": [], "require": [], "spec": [], "design": []}
    design_areas: list[dict] = []

    for path, frag in fragments:
        layer = frag.get("layer")
        if layer not in LAYERS:
            print(f"error: {path}: unknown or missing layer {layer!r}", file=sys.stderr)
            return 1
        if layer == "design":
            for area in frag.get("areas", []):
                design_areas.append(area)
                per_layer_items["design"].extend(area.get("items", []))
                for it in area.get("items", []):
                    it["_area"] = area
        else:
            per_layer_items[layer].extend(frag.get("items", []))

    # request / require / spec: sort, stamp, build output arrays.
    output = {"schema_version": SCHEMA_VERSION}
    for layer in ("request", "require", "spec"):
        items = per_layer_items[layer]
        items.sort(key=lambda it, layer=layer: item_sort_key(layer, it))
        assign_ids(items, LEAF_PREFIX[layer])
        output[layer] = [build_leaf_output(it, layer) for it in items]

    # design: areas ordered/stamped independently of items; items ordered
    # and stamped in one global pass, then grouped back under their area.
    design_areas.sort(key=lambda a: (design_doc_rank(a["source"]["doc"]), a["source"]["lines"][0]))
    assign_ids(design_areas, "DA")

    all_design_items = per_layer_items["design"]
    all_design_items.sort(key=lambda it: item_sort_key("design", it))
    assign_ids(all_design_items, "DES")

    area_out_by_id = {}
    design_out = []
    for area in design_areas:
        area_out = {"id": area["_id"], "title": area["title"]}
        if area.get("description"):
            area_out["description"] = area["description"]
        area_out["items"] = []
        area_out["source"] = build_source(area["source"])
        design_out.append(area_out)
        area_out_by_id[id(area)] = area_out

    for it in all_design_items:
        area_out = area_out_by_id[id(it["_area"])]
        area_out["items"].append(build_leaf_output(it, "design"))

    output["design"] = design_out

    text = json.dumps(output, ensure_ascii=False, indent=2) + "\n"
    spec_json_path(root).write_text(text, encoding="utf-8")

    schema = read_json(schema_path(root))
    validator = Draft202012Validator(schema)
    errors = sorted(validator.iter_errors(output), key=lambda e: list(e.absolute_path))
    if errors:
        for err in errors:
            pointer = "/" + "/".join(str(p) for p in err.absolute_path)
            print(f"SCHEMA ERROR {pointer}: {err.message}", file=sys.stderr)
        print(f"build: wrote {spec_json_path(root)} but it failed schema validation ({len(errors)} error(s))", file=sys.stderr)
        return 1

    print(f"build: wrote {spec_json_path(root)} ({len(output['request'])} request, "
          f"{len(output['require'])} require, {len(output['spec'])} spec, "
          f"{len(output['design'])} design areas) -- schema OK")
    return 0


# ------------------------------------------------------------- coverage --

def compact_ranges(nums: list[int]) -> str:
    if not nums:
        return "(none)"
    nums = sorted(set(nums))
    parts = []
    start = prev = nums[0]
    for n in nums[1:]:
        if n == prev + 1:
            prev = n
            continue
        parts.append((start, prev))
        start = prev = n
    parts.append((start, prev))
    return ", ".join(f"{s}-{e}" if s != e else f"{s}" for s, e in parts)


def is_auto_excluded(line: str) -> bool:
    stripped = line.strip()
    if stripped == "" or stripped == "---":
        return True
    if stripped.startswith("```"):
        return True
    if HEADING_RE.match(line):
        return True
    return False


def cmd_coverage(args) -> int:
    root = Path(args.root)
    repo_root = Path(args.repo_root)
    fragments = load_fragments(root)
    dropped = load_dropped(root)

    # doc -> item-level ranges (for overlap check + coverage)
    item_ranges: dict[str, list[tuple[int, int, str]]] = {}
    # doc -> area ranges (coverage only, never overlap-checked)
    area_ranges: dict[str, list[tuple[int, int]]] = {}
    docs_seen: list[str] = []

    def note_doc(doc: str) -> None:
        if doc not in item_ranges:
            item_ranges[doc] = []
            area_ranges[doc] = []
            docs_seen.append(doc)

    for path, frag in fragments:
        layer = frag.get("layer")
        if layer == "design":
            for area in frag.get("areas", []):
                asrc = area["source"]
                note_doc(asrc["doc"])
                area_ranges[asrc["doc"]].append((asrc["lines"][0], asrc["lines"][1]))
                for it in area.get("items", []):
                    isrc = it["source"]
                    note_doc(isrc["doc"])
                    item_ranges[isrc["doc"]].append((isrc["lines"][0], isrc["lines"][1], area.get("title", "?")))
        else:
            for it in frag.get("items", []):
                isrc = it["source"]
                note_doc(isrc["doc"])
                item_ranges[isrc["doc"]].append((isrc["lines"][0], isrc["lines"][1], it.get("statement", "")[:30]))

    dropped_ranges: dict[str, list[tuple[int, int]]] = {}
    for _path, entry in dropped:
        doc = entry["doc"]
        note_doc(doc)
        dropped_ranges.setdefault(doc, []).append((entry["lines"][0], entry["lines"][1]))

    had_uncovered = False
    for doc in docs_seen:
        md_path = repo_root / doc
        try:
            md_text = md_path.read_text(encoding="utf-8")
        except OSError as exc:
            print(f"error: cannot read source doc {md_path}: {exc}", file=sys.stderr)
            return 1
        lines = md_text.splitlines()
        total = len(lines)

        covered: set[int] = set()
        for s, e, _label in item_ranges[doc]:
            covered.update(range(s, e + 1))
        for s, e in area_ranges[doc]:
            covered.update(range(s, e + 1))
        for s, e in dropped_ranges.get(doc, []):
            covered.update(range(s, e + 1))

        uncovered = []
        for i in range(1, total + 1):
            if i in covered:
                continue
            if is_auto_excluded(lines[i - 1]):
                continue
            uncovered.append(i)

        print(f"--- {doc} ---")
        print(f"  total lines: {total}")
        print(f"  covered (explicit+dropped): {len(covered)}")
        print(f"  uncovered: {compact_ranges(uncovered)}")
        if uncovered:
            had_uncovered = True

        # overlap check: item-level ranges only, within this doc.
        ranges = item_ranges[doc]
        overlaps = []
        for a in range(len(ranges)):
            for b in range(a + 1, len(ranges)):
                s1, e1, l1 = ranges[a]
                s2, e2, l2 = ranges[b]
                if s1 <= e2 and s2 <= e1:
                    overlaps.append((s1, e1, l1, s2, e2, l2))
        if overlaps:
            print(f"  item/item overlaps ({len(overlaps)}):")
            for s1, e1, l1, s2, e2, l2 in overlaps:
                print(f"    [{s1}-{e1}] {l1!r} overlaps [{s2}-{e2}] {l2!r}")

    if not docs_seen:
        print("coverage: no fragments found (nothing to check)")

    return 1 if had_uncovered else 0


# --------------------------------------------------------------- export --

def fmt_item(item: dict) -> list[str]:
    out = [f"### {item['id']}", "", item["statement"].strip()]
    desc = item.get("description")
    if desc:
        out.append("")
        for dl in desc.strip("\n").split("\n"):
            out.append(f"> {dl}".rstrip() if dl else ">")
    derived = item.get("derived_from")
    if derived:
        out.append("")
        out.append(f"*導出元: {', '.join(derived)}*")
    cites = item.get("cites")
    if cites:
        out.append("")
        out.append(f"*引用: {', '.join(cites)}*")
    return out


BANNER = "<!-- generated from docs/canonical/specification.json by build.py; do not edit -->"


def write_export_file(path: Path, title: str, blocks: list[list[str]]) -> None:
    lines = [BANNER, "", f"# {title}"]
    for block in blocks:
        lines.append("")
        lines.extend(block)
    text = "\n".join(lines).rstrip("\n") + "\n"
    path.write_text(text, encoding="utf-8")


def cmd_export(args) -> int:
    root = Path(args.root)
    spec = read_json(spec_json_path(root))
    out_dir = export_dir(root)
    out_dir.mkdir(parents=True, exist_ok=True)

    write_export_file(out_dir / "request.md", "要求", [fmt_item(it) for it in spec["request"]])
    write_export_file(out_dir / "require.md", "要件定義", [fmt_item(it) for it in spec["require"]])
    write_export_file(out_dir / "spec.md", "基本仕様", [fmt_item(it) for it in spec["spec"]])

    design_blocks: list[list[str]] = []
    for area in spec["design"]:
        block = [f"## {area['id']} {area['title']}"]
        if area.get("description"):
            block.append("")
            block.append(area["description"].strip())
        for it in area["items"]:
            block.append("")
            block.extend(fmt_item(it))
        design_blocks.append(block)
    write_export_file(out_dir / "design.md", "詳細設計", design_blocks)

    print(f"export: wrote {out_dir / 'request.md'}, require.md, spec.md, design.md")
    return 0


# --------------------------------------------------------- check-fragment --

def check_source(src, label: str, problems: list[str]) -> None:
    if not isinstance(src, dict):
        problems.append(f"{label}: source must be an object")
        return
    for key in ("doc", "heading"):
        if not isinstance(src.get(key), str) or not src.get(key):
            problems.append(f"{label}: source.{key} must be a non-empty string")
    lines = src.get("lines")
    if not (isinstance(lines, list) and len(lines) == 2 and all(isinstance(n, int) for n in lines)):
        problems.append(f"{label}: source.lines must be [start, end] integers")
        return
    s, e = lines
    if s < 1 or e < 1 or s > e:
        problems.append(f"{label}: source.lines={lines} is not a valid 1-based [start, end] with start<=end")


def check_leaf_item(item, label: str, problems: list[str]) -> None:
    if not isinstance(item, dict):
        problems.append(f"{label}: item must be an object")
        return
    if not isinstance(item.get("statement"), str) or not item.get("statement"):
        problems.append(f"{label}: statement must be a non-empty string")
    if "id" in item:
        problems.append(f"{label}: fragments must not carry an 'id'; ids are stamped by 'build'")
    if "derived_from" not in item:
        problems.append(f"{label}: derived_from is required (must be []) in fragments")
    elif item["derived_from"] != []:
        problems.append(f"{label}: derived_from must be [] in fragments; got {item['derived_from']!r}")
    if "description" in item and not isinstance(item["description"], str):
        problems.append(f"{label}: description must be a string")
    if "cites" in item:
        cites = item["cites"]
        if not (isinstance(cites, list) and all(isinstance(c, str) and c for c in cites)):
            problems.append(f"{label}: cites must be a list of non-empty strings")
    if "keep_id" in item:
        keep = item["keep_id"]
        if not isinstance(keep, str) or not ID_PATTERN.match(keep):
            problems.append(f"{label}: keep_id={keep!r} does not match the id pattern")
    if "source" not in item:
        problems.append(f"{label}: source is required")
    else:
        check_source(item["source"], label, problems)


def cmd_check_fragment(args) -> int:
    path = Path(args.path)
    frag = read_json(path)
    problems: list[str] = []

    if not isinstance(frag, dict):
        print(f"{path}: fragment must be a JSON object")
        return 1

    doc = frag.get("doc")
    if not isinstance(doc, str) or not doc:
        problems.append("doc must be a non-empty string")
    layer = frag.get("layer")
    if layer not in LAYERS:
        problems.append(f"layer must be one of {LAYERS}; got {layer!r}")

    if layer in ("request", "require", "spec"):
        if "areas" in frag:
            problems.append(f"layer {layer!r} must not carry 'areas'")
        items = frag.get("items")
        if not isinstance(items, list):
            problems.append("items must be a list")
        else:
            for i, it in enumerate(items):
                check_leaf_item(it, f"items[{i}]", problems)
    elif layer == "design":
        if "items" in frag:
            problems.append("layer 'design' must not carry top-level 'items' (use 'areas[].items')")
        areas = frag.get("areas")
        if not isinstance(areas, list):
            problems.append("areas must be a list")
        else:
            for ai, area in enumerate(areas):
                alabel = f"areas[{ai}]"
                if not isinstance(area, dict):
                    problems.append(f"{alabel}: area must be an object")
                    continue
                if "id" in area:
                    problems.append(f"{alabel}: fragments must not carry an 'id'; ids are stamped by 'build'")
                if not isinstance(area.get("title"), str) or not area.get("title"):
                    problems.append(f"{alabel}: title must be a non-empty string")
                if "description" in area and not isinstance(area["description"], str):
                    problems.append(f"{alabel}: description must be a string")
                if "source" not in area:
                    problems.append(f"{alabel}: source is required")
                else:
                    check_source(area["source"], alabel, problems)
                aitems = area.get("items")
                if not isinstance(aitems, list):
                    problems.append(f"{alabel}: items must be a list")
                else:
                    for ii, it in enumerate(aitems):
                        check_leaf_item(it, f"{alabel}.items[{ii}]", problems)

    if problems:
        print(f"{path}: {len(problems)} problem(s):")
        for p in problems:
            print(f"  - {p}")
        return 1

    print(f"{path}: OK")
    return 0


# --------------------------------------------------------- harvest-cites --

# Document-name alternations, one per pattern below. The section-list and
# L-line patterns both recognise 別紙A/別紙B/別紙C symmetrically; the
# P-code pattern is narrower by design (only 要件定義/基本仕様 carry P-00N
# principles).
_DOC_SECTION = "要件定義|基本仕様|詳細設計|本冊|別紙[ABC]"
_DOC_LINE = "要件定義|基本仕様|詳細設計|本冊|別紙[ABC]"
_DOC_P = "要件定義|基本仕様"

_CITATION_SEP_RE = re.compile(r"\s*[・、/／]\s*")

CITATION_RE = re.compile(
    r"(?P<secdoc>" + _DOC_SECTION + r")\s*"
    r"(?P<seclist>§\s*[0-9]+(?:\.[0-9]+)*(?:\s*[・、/／]\s*§?\s*[0-9]+(?:\.[0-9]+)*)*)"
    r"|(?P<ldoc>" + _DOC_LINE + r")\s*L\s*(?P<lnum>[0-9]+)"
    r"|(?P<pdoc>" + _DOC_P + r")\s*(?P<pnum>P-[0-9]{3})"
    r"|(?<![A-Za-z0-9])(?P<bare>P-[0-9]{3}|R-[1-5]|F[0-9]+|OOS-[0-9]{3}|NFR-[0-9]{3})(?![A-Za-z0-9])"
)


def split_seclist(doc: str, seclist: str) -> list[str]:
    """'§5.1・§23' (doc-prefixed) -> ['<doc> §5.1', '<doc> §23']."""
    out = []
    for part in _CITATION_SEP_RE.split(seclist):
        part = part.strip()
        if not part:
            continue
        if not part.startswith("§"):
            part = "§" + part
        num = part[1:].strip()
        out.append(f"{doc} §{num}")
    return out


def extract_citations(statement: str) -> list[str]:
    """Citations found in `statement`, in order of appearance, not deduped.

    A bare code (NFR-00N/OOS-00N/P-00N/R-N/F<n>) matched at the very start
    of the statement (ignoring leading whitespace) is the statement naming
    itself -- e.g. "OOS-001仕様書同士の品質監査について..." or "NFR-001並列性
    への対応は..." -- and is not harvested as a citation of something else.
    Whatever follows it (a separator like " ", ".", ":", "：", a full-width
    space, straight into kanji, or nothing at all) does not change this;
    only the position matters.
    """
    found = []
    lead = len(statement) - len(statement.lstrip())
    for m in CITATION_RE.finditer(statement):
        if m.group("bare") and m.start() == lead:
            continue
        if m.group("secdoc"):
            found.extend(split_seclist(m.group("secdoc"), m.group("seclist")))
        elif m.group("ldoc"):
            found.append(f"{m.group('ldoc')} L{m.group('lnum')}")
        elif m.group("pdoc"):
            found.append(f"{m.group('pdoc')} {m.group('pnum')}")
        elif m.group("bare"):
            found.append(m.group("bare"))
    return found


# CONVERSION.md SS3: same-document references (no doc name) stay inline in
# the statement and are not citations. A cites entry that is bare -- "§N",
# "§N.N", or a joined list of them ("§4.1・§4.4", "§5.1、§23") -- starts
# with "§" once stripped; a doc-named or R-/P-/F/OOS-/NFR- entry never does.
_BARE_SECTION_ENTRY_RE = re.compile(r"^§")


def is_bare_section_entry(cite: str) -> bool:
    return bool(_BARE_SECTION_ENTRY_RE.match(cite.strip()))


def harvest_item(item: dict) -> tuple[list[str], int]:
    """Mutate item['cites'] in place: drop bare same-document section
    entries, then append newly-found citations from 'statement' (existing
    surviving entries and their order untouched; no duplicates). Deletes
    'cites' entirely if it would end up empty. Never touches 'statement'
    or 'derived_from'. Returns (added, removed_count)."""
    existing_list = list(item["cites"]) if isinstance(item.get("cites"), list) else []
    kept = [c for c in existing_list if not is_bare_section_entry(c)]
    removed = len(existing_list) - len(kept)

    found = extract_citations(item.get("statement", ""))
    seen = set(kept)
    added = []
    for c in found:
        if c in seen:
            continue
        seen.add(c)
        added.append(c)

    new_list = kept + added
    if new_list:
        item["cites"] = new_list
    elif "cites" in item:
        del item["cites"]

    return added, removed


def iter_leaf_items(frag: dict):
    if frag.get("layer") == "design":
        for area in frag.get("areas", []):
            for it in area.get("items", []):
                yield it
    else:
        for it in frag.get("items", []):
            yield it


def cmd_harvest_cites(args) -> int:
    root = Path(args.root)
    files = list_fragment_files(root)
    if not files:
        print("harvest-cites: no fragments found")
        return 0

    for path in files:
        frag = read_json(path)
        items = list(iter_leaf_items(frag))
        added_all: list[str] = []
        removed_total = 0
        for it in items:
            added, removed = harvest_item(it)
            added_all.extend(added)
            removed_total += removed

        top = Counter(added_all).most_common(10)
        print(f"--- {path} ---")
        print(f"  items scanned: {len(items)}")
        print(f"  citations added: {len(added_all)}")
        print(f"  citations removed (bare same-document §): {removed_total}")
        if top:
            print("  top citations: " + ", ".join(f"{c!r} ({n})" for c, n in top))
        else:
            print("  top citations: (none)")

        if (added_all or removed_total) and not args.dry_run:
            text = json.dumps(frag, ensure_ascii=False, indent=2) + "\n"
            path.write_text(text, encoding="utf-8")

    return 0


# ------------------------------------------------------------------ cli --

def cmd_all(args) -> int:
    rc = cmd_build(args)
    if rc != 0:
        return rc
    rc = cmd_coverage(args)
    if rc != 0:
        return rc
    return cmd_export(args)


def main(argv=None) -> int:
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    sub = parser.add_subparsers(dest="command", required=True)

    def add_root_arg(p):
        p.add_argument("--root", default="docs/canonical", help="directory holding fragments/, specification.json, export/ (default: docs/canonical)")

    def add_repo_root_arg(p):
        p.add_argument("--repo-root", default=".", help="base directory for resolving fragment 'doc' paths (default: .)")

    p_build = sub.add_parser("build", help="assign ids, write specification.json, validate against schema")
    add_root_arg(p_build)
    p_build.set_defaults(func=cmd_build)

    p_cov = sub.add_parser("coverage", help="report md lines not accounted for by any fragment/dropped-log entry")
    add_root_arg(p_cov)
    add_repo_root_arg(p_cov)
    p_cov.set_defaults(func=cmd_coverage)

    p_exp = sub.add_parser("export", help="write export/{request,require,spec,design}.md from specification.json")
    add_root_arg(p_exp)
    p_exp.set_defaults(func=cmd_export)

    p_all = sub.add_parser("all", help="build, then coverage, then export; stop at first failure")
    add_root_arg(p_all)
    add_repo_root_arg(p_all)
    p_all.set_defaults(func=cmd_all)

    p_check = sub.add_parser("check-fragment", help="validate one fragment file's structural shape")
    p_check.add_argument("path", help="path to the fragment JSON file")
    p_check.set_defaults(func=cmd_check_fragment)

    p_harvest = sub.add_parser("harvest-cites", help="scan item statements for citations and append them to cites")
    add_root_arg(p_harvest)
    p_harvest.add_argument("--dry-run", action="store_true", help="print the report without writing any fragment file")
    p_harvest.set_defaults(func=cmd_harvest_cites)

    args = parser.parse_args(argv)
    return args.func(args)


if __name__ == "__main__":
    sys.exit(main())
