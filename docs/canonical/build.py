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

derivation-candidates
    CONVERSION.md SS6 steps 1-2: produce the mechanical candidate list for
    Owner approval. No inference, no scoring, no picking -- every rule
    below is a fixed lookup, never a guess. Reads specification.json (must
    already be built) and, for the 付記 導出表 source, the md range named
    by the "keep_for_derivation": true entry in fragments/*.dropped.json
    (never hard-coded).
    Writes three files under --root:
      derivation-candidates.json -- one entry per (statement, cite) pair
        for every statement carrying a non-empty "cites": {id, statement
        (first 80 chars), cite, candidates:[{id, statement (first 60
        chars)}], resolution}. resolution is "exact" (cite is an id that
        exists, or a bare NFR/OOS/F code whose target statement names
        itself and there is exactly one such statement), "section" (cite
        names a document + section/line and at least one statement in
        that document's heading/line range matches), or "unresolved"
        (cite recognised but zero matches, or not recognised at all).
      derivation-table-candidates.json -- one entry per data row of the
        付記 導出表 (its two md tables: 第I部 根->要求, 第II部 要求->要件；
        header and separator rows skipped by shape, not by hard-coded
        line numbers): {row_line, target_section, target_ids, source_ids,
        kind, note}. The table has no separate 区分 column (its columns
        are 上流ノード/下流ノード/導出理由/状態); "kind" carries the 状態
        column verbatim (always "ACCEPTED" in the current table) as the
        closest analogue, and "note" carries all three text columns
        verbatim so nothing is lost. Each of 上流ノード and 下流ノード is
        split on "・"/"/" and every token is resolved the same way as a
        cite (id / require-layer section / self-naming code); unresolved
        tokens simply contribute no id (their text survives in "note").
      derivation-candidates.md -- human-readable: a summary (counts by
        resolution, candidate-pair count, table-row counts by kind), then
        per source document a bullet list of (statement, cite) pairs and
        their candidates (top 5, "+k more" beyond that).
    Prints the same summary to stdout. Fully deterministic (iterates
    specification.json's own array order; table rows in line order).

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


# ---------------------------------------------------- derivation-candidates --

def build_id_index(spec: dict) -> dict:
    """Every statement id (request/require/spec/design item) -> its record,
    in specification.json's own deterministic order."""
    idx: dict = {}
    for it in spec["request"]:
        idx[it["id"]] = dict(it, layer="request")
    for it in spec["require"]:
        idx[it["id"]] = dict(it, layer="require")
    for it in spec["spec"]:
        idx[it["id"]] = dict(it, layer="spec")
    for area in spec["design"]:
        for it in area["items"]:
            idx[it["id"]] = dict(it, layer="design", area_id=area["id"], area_title=area["title"])
    return idx


def scope_items(spec: dict, scope: str) -> list:
    """scope: 'require' | 'spec' | 'design:本冊' | 'design:別紙A' | 'design:別紙B' | 'design:別紙C'."""
    if scope == "require":
        return spec["require"]
    if scope == "spec":
        return spec["spec"]
    if scope.startswith("design:"):
        mark = scope.split(":", 1)[1]
        out = []
        for area in spec["design"]:
            doc = area["source"]["doc"]
            if mark == "本冊":
                is_match = "別紙A" not in doc and "別紙B" not in doc and "別紙C" not in doc
            else:
                is_match = mark in doc
            if is_match:
                out.extend(area["items"])
        return out
    return []


_DOC_TO_SCOPE = {
    "要件定義": "require",
    "基本仕様": "spec",
    "本冊": "design:本冊",
    "詳細設計": "design:本冊",
    "別紙A": "design:別紙A",
    "別紙B": "design:別紙B",
    "別紙C": "design:別紙C",
}

_HEADING_NUM_RE = re.compile(r"^#{0,6}\s*([0-9]+(?:\.[0-9]+)*)")


def heading_number_token(heading: str):
    m = _HEADING_NUM_RE.match(heading.strip())
    return m.group(1) if m else None


def section_matches(query: str, token) -> bool:
    if token is None:
        return False
    return token == query or token.startswith(query + ".")


def section_candidates(spec: dict, scope: str, number: str) -> list:
    return [
        it["id"]
        for it in scope_items(spec, scope)
        if section_matches(number, heading_number_token(it["source"]["heading"]))
    ]


def line_candidates(spec: dict, scope: str, line_no: int) -> list:
    out = []
    for it in scope_items(spec, scope):
        s, e = it["source"]["lines"]
        if s <= line_no <= e:
            out.append(it["id"])
    return out


def self_naming_candidates(id_index: dict, code: str) -> list:
    return [iid for iid, it in id_index.items() if it["statement"].startswith(code)]


# --- Output 1: resolve a stored `cites` string (already normalised by
# harvest-cites into one of exactly four shapes) to candidate ids.

_CITE_SECTION_RE = re.compile(r"^(要件定義|基本仕様|詳細設計|本冊|別紙[ABC])\s*§\s*([0-9]+(?:\.[0-9]+)*)$")
_CITE_LINE_RE = re.compile(r"^(要件定義|基本仕様|詳細設計|本冊|別紙[ABC])\s*L\s*([0-9]+)$")
_CITE_PDOC_RE = re.compile(r"^(要件定義|基本仕様)\s*(P-[0-9]{3})$")
_CITE_ID_RE = re.compile(r"^(R-[1-5]|P-[0-9]{3})$")
_CITE_SELFNAME_RE = re.compile(r"^(NFR-[0-9]{3}|OOS-[0-9]{3}|F[0-9]+)$")


def scope_to_layer(scope: str):
    if scope == "require":
        return "require"
    if scope == "spec":
        return "spec"
    if scope and scope.startswith("design:"):
        return "design"
    return None


_LAYER_RANK = {"request": 0, "require": 1, "spec": 2, "design": 3}


def layer_relation(source_layer, target_layer):
    """Fixed order request<require<spec<design (every design area, whichever
    of 本冊/別紙A/別紙C, is layer 'design'). None when either layer is
    undetermined (e.g. an unresolved cite whose ambiguous self-naming
    candidates don't share one layer)."""
    if source_layer is None or target_layer is None:
        return None
    s, t = _LAYER_RANK[source_layer], _LAYER_RANK[target_layer]
    if t == s:
        return "same-layer"
    if t == s - 1:
        return "adjacent-upstream"
    if t < s - 1:
        return "skip-upstream"
    return "downstream"


def resolve_cite(spec: dict, id_index: dict, cite: str):
    """-> (candidate_ids: list[str], resolution: 'exact'|'section'|'unresolved',
    target_layer: str|None)."""
    m = _CITE_SECTION_RE.match(cite)
    if m:
        scope = _DOC_TO_SCOPE.get(m.group(1))
        cands = section_candidates(spec, scope, m.group(2)) if scope else []
        return cands, ("section" if cands else "unresolved"), scope_to_layer(scope)
    m = _CITE_LINE_RE.match(cite)
    if m:
        scope = _DOC_TO_SCOPE.get(m.group(1))
        cands = line_candidates(spec, scope, int(m.group(2))) if scope else []
        return cands, ("section" if cands else "unresolved"), scope_to_layer(scope)
    m = _CITE_PDOC_RE.match(cite)
    if m:
        pid = m.group(2)
        if pid in id_index:
            return [pid], "exact", id_index[pid]["layer"]
        return [], "unresolved", None
    m = _CITE_ID_RE.match(cite)
    if m:
        iid = m.group(1)
        if iid in id_index:
            return [iid], "exact", id_index[iid]["layer"]
        return [], "unresolved", None
    m = _CITE_SELFNAME_RE.match(cite)
    if m:
        cands = self_naming_candidates(id_index, m.group(1))
        if len(cands) == 1:
            return cands, "exact", id_index[cands[0]]["layer"]
        layers = {id_index[c]["layer"] for c in cands}
        return [], "unresolved", (layers.pop() if len(layers) == 1 else None)
    return [], "unresolved", None


def iter_all_statements(spec: dict):
    for it in spec["request"]:
        yield it
    for it in spec["require"]:
        yield it
    for it in spec["spec"]:
        yield it
    for area in spec["design"]:
        for it in area["items"]:
            yield it


def build_derivation_candidates(spec: dict, id_index: dict) -> list:
    entries = []
    for it in iter_all_statements(spec):
        source_layer = id_index[it["id"]]["layer"]
        for cite in it.get("cites") or []:
            cands, resolution, target_layer = resolve_cite(spec, id_index, cite)
            entries.append({
                "id": it["id"],
                "statement": it["statement"][:80],
                "cite": cite,
                "candidates": [{"id": cid, "statement": id_index[cid]["statement"][:60]} for cid in cands],
                "resolution": resolution,
                "layer_relation": layer_relation(source_layer, target_layer),
            })
    return entries


# --- Output 2: parse the 付記 導出表 (要求・要件定義 v0.1.md, the md range
# named by a "keep_for_derivation": true dropped-log entry) into candidates.

_TABLE_ROW_RE = re.compile(r"^\|(.+)\|\s*$")
_TABLE_SEP_CELL_RE = re.compile(r"^:?-{2,}:?$")
_TABLE_TOKEN_SPLIT_RE = re.compile(r"[・/]")

_TABLE_ID_RE = re.compile(r"^(R-[1-5]|P-[0-9]{3})(?![0-9A-Za-z])")
_TABLE_SECTION_RE = re.compile(r"^§\s*([0-9]+(?:\.[0-9]+)*)(?:-[A-Za-z])?")
_TABLE_SELFNAME_RE = re.compile(r"^(NFR-[0-9]{3}|OOS-[0-9]{3}|F[0-9]+)(?![0-9A-Za-z])")


def find_derivation_table_ranges(root: Path) -> list:
    """[(doc, start_line, end_line), ...] from every dropped-log entry
    marked keep_for_derivation: true. Never hard-coded."""
    out = []
    for _path, entry in load_dropped(root):
        if entry.get("keep_for_derivation"):
            out.append((entry["doc"], entry["lines"][0], entry["lines"][1]))
    return out


def parse_md_table_rows(md_lines: list, start: int, end: int) -> list:
    """md_lines: 0-indexed full-file lines. start/end: 1-based inclusive.
    -> [(line_no, [cell, ...]), ...] for data rows only (header/separator
    rows are recognised by shape and skipped)."""
    rows = []
    for line_no in range(start, min(end, len(md_lines)) + 1):
        m = _TABLE_ROW_RE.match(md_lines[line_no - 1].strip())
        if not m:
            continue
        cells = [c.strip() for c in m.group(1).split("|")]
        if all(_TABLE_SEP_CELL_RE.match(c) for c in cells if c):
            continue  # separator row (e.g. |---|---|---|---|)
        if cells and cells[0] == "上流ノード":
            continue  # header row
        rows.append((line_no, cells))
    return rows


def classify_table_ref(token: str):
    """-> (kind, key, matched_text) for one token from a 上流ノード/下流ノード
    cell. kind is 'id' | 'section' | 'selfname' | 'unknown'."""
    token = token.strip()
    m = _TABLE_ID_RE.match(token)
    if m:
        return "id", m.group(1), m.group(0)
    m = _TABLE_SECTION_RE.match(token)
    if m:
        return "section", m.group(1), m.group(0)
    m = _TABLE_SELFNAME_RE.match(token)
    if m:
        return "selfname", m.group(1), m.group(0)
    return "unknown", None, None


def resolve_table_ref(spec: dict, id_index: dict, token: str):
    """-> (candidate_ids: list[str], matched_section_text: str|None)."""
    kind, key, matched = classify_table_ref(token)
    if kind == "id":
        return ([key] if key in id_index else []), None
    if kind == "section":
        return section_candidates(spec, "require", key), matched
    if kind == "selfname":
        return self_naming_candidates(id_index, key), None
    return [], None


def resolve_table_cell(spec: dict, id_index: dict, cell: str):
    """Split a 上流ノード/下流ノード cell on ・ and / (both are used as
    multi-reference separators in this table) and resolve every token.
    -> (ids: list[str] deduped in order, section_texts: list[str])."""
    ids: list = []
    seen = set()
    sections: list = []
    for tok in _TABLE_TOKEN_SPLIT_RE.split(cell):
        tok = tok.strip()
        if not tok:
            continue
        cands, matched = resolve_table_ref(spec, id_index, tok)
        for c in cands:
            if c not in seen:
                seen.add(c)
                ids.append(c)
        if matched:
            sections.append(matched)
    return ids, sections


def build_derivation_table_candidates(spec: dict, id_index: dict, root: Path, repo_root: Path) -> list:
    rows_out = []
    for doc, start, end in find_derivation_table_ranges(root):
        md_path = repo_root / doc
        md_lines = md_path.read_text(encoding="utf-8").splitlines()
        for line_no, cells in parse_md_table_rows(md_lines, start, end):
            if len(cells) != 4:
                rows_out.append({
                    "row_line": line_no,
                    "target_section": None,
                    "target_ids": [],
                    "source_ids": [],
                    "kind": None,
                    "note": "列数が4でない行（そのまま記録）: " + " | ".join(cells),
                })
                continue
            upstream, downstream, reason, state = cells
            target_ids, target_sections = resolve_table_cell(spec, id_index, downstream)
            source_ids, _src_sections = resolve_table_cell(spec, id_index, upstream)
            rows_out.append({
                "row_line": line_no,
                "target_section": "/".join(target_sections) if target_sections else None,
                "target_ids": target_ids,
                "source_ids": source_ids,
                "kind": state,
                "note": f"上流: {upstream} / 下流: {downstream} / 理由: {reason}",
            })
    return rows_out


# --- Output 3: human-readable summary.

def doc_display_name(doc: str) -> str:
    if "別紙A" in doc:
        return "詳細設計 別紙A"
    if "別紙B" in doc:
        return "詳細設計 別紙B"
    if "別紙C" in doc:
        return "詳細設計 別紙C"
    if "詳細設計" in doc:
        return "詳細設計（本冊）"
    if "基本仕様" in doc:
        return "基本仕様"
    if "要求" in doc or "要件定義" in doc:
        return "要求・要件定義"
    return doc


def summarize_derivation(entries: list, table_rows: list) -> dict:
    resolution_counts = Counter(e["resolution"] for e in entries)
    kind_counts = Counter(r["kind"] for r in table_rows)
    return {
        "resolution_counts": resolution_counts,
        "candidate_pairs": len(entries),
        "table_rows": len(table_rows),
        "kind_counts": kind_counts,
    }


def print_derivation_summary(summary: dict) -> None:
    print("--- derivation-candidates summary ---")
    for res in ("exact", "section", "unresolved"):
        print(f"  resolution {res}: {summary['resolution_counts'].get(res, 0)}")
    print(f"  candidate pairs total: {summary['candidate_pairs']}")
    print(f"  derivation-table rows total: {summary['table_rows']}")
    for kind, n in sorted(summary["kind_counts"].items(), key=lambda kv: (kv[0] is None, kv[0] or "")):
        print(f"  table rows kind={kind!r}: {n}")


def write_derivation_md(path: Path, entries: list, table_rows: list, summary: dict) -> None:
    lines = [
        "<!-- generated from docs/canonical/specification.json and the 付記 導出表 by build.py derivation-candidates; do not edit -->",
        "",
        "# 導出候補（機械生成・Owner 未承認）",
        "",
        "CONVERSION.md SS6 の手順1-2の出力。推論・採点・選定は行っていない。承認して derived_from へ入れるかは Owner の判断。",
        "",
        "## 集計",
        "",
        "| resolution | 件数 |",
        "|---|---|",
    ]
    for res in ("exact", "section", "unresolved"):
        lines.append(f"| {res} | {summary['resolution_counts'].get(res, 0)} |")
    lines.append(f"| **候補ペア合計** | **{summary['candidate_pairs']}** |")
    lines += ["", "| 導出表 状態（kind） | 件数 |", "|---|---|"]
    for kind, n in sorted(summary["kind_counts"].items(), key=lambda kv: (kv[0] is None, kv[0] or "")):
        lines.append(f"| {kind if kind is not None else '(列数異常)'} | {n} |")
    lines.append(f"| **導出表行合計** | **{summary['table_rows']}** |")

    by_doc: dict = {}
    for e in entries:
        by_doc.setdefault(e["_doc"], []).append(e)

    for doc in sorted(by_doc):
        lines += ["", f"## {doc_display_name(doc)}", ""]
        for e in by_doc[doc]:
            cands = e["candidates"]
            shown = cands[:5]
            cand_text = ", ".join(f"{c['id']} 「{c['statement']}」" for c in shown)
            if len(cands) > 5:
                cand_text += f" +{len(cands) - 5} more"
            if not cands:
                cand_text = "(候補なし)"
            lines.append(f"- {e['id']} 「{e['statement'][:60]}」 ← {e['cite']} → {len(cands)}件: {cand_text}")

    text = "\n".join(lines).rstrip("\n") + "\n"
    path.write_text(text, encoding="utf-8")


def cmd_derivation_candidates(args) -> int:
    root = Path(args.root)
    repo_root = Path(args.repo_root)
    spec = read_json(spec_json_path(root))
    id_index = build_id_index(spec)

    entries = build_derivation_candidates(spec, id_index)
    # attach the owning statement's source doc for Output 3's per-document
    # grouping only; not part of the written JSON.
    doc_by_id = {it["id"]: it["source"]["doc"] for it in iter_all_statements(spec)}
    for e in entries:
        e["_doc"] = doc_by_id[e["id"]]

    table_rows = build_derivation_table_candidates(spec, id_index, root, repo_root)

    json_entries = [{k: v for k, v in e.items() if k != "_doc"} for e in entries]
    (root / "derivation-candidates.json").write_text(
        json.dumps(json_entries, ensure_ascii=False, indent=2) + "\n", encoding="utf-8"
    )
    (root / "derivation-table-candidates.json").write_text(
        json.dumps(table_rows, ensure_ascii=False, indent=2) + "\n", encoding="utf-8"
    )

    summary = summarize_derivation(entries, table_rows)
    write_derivation_md(root / "derivation-candidates.md", entries, table_rows, summary)
    print_derivation_summary(summary)
    print(f"wrote {root / 'derivation-candidates.json'}")
    print(f"wrote {root / 'derivation-table-candidates.json'}")
    print(f"wrote {root / 'derivation-candidates.md'}")
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

    p_deriv = sub.add_parser("derivation-candidates", help="CONVERSION.md SS6 steps 1-2: mechanical derivation candidate list")
    add_root_arg(p_deriv)
    add_repo_root_arg(p_deriv)
    p_deriv.set_defaults(func=cmd_derivation_candidates)

    args = parser.parse_args(argv)
    return args.func(args)


if __name__ == "__main__":
    sys.exit(main())
