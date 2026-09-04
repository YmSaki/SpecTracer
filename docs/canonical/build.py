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
    and report which lines are NOT accounted for by an item source range,
    a dropped-log range, or an auto-excluded line (blank line, ATX
    heading, "---", code-fence delimiter). Per CONVERSION.md SS5 a design
    area's own source.lines is not an item and earns no coverage credit --
    a line that sits inside an area's range but outside every one of its
    items and every dropped range is reported uncovered. Also reports
    (report only, does not affect exit code) overlaps between item-level
    source ranges within the same doc. Exit 1 if any uncovered line
    exists.

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
    duplicates). A bare code is a self-citation -- never harvested -- in
    either of two cases: it is matched at the very start of the statement
    (e.g. "OOS-001仕様書同士の..." or "NFR-001並列性への対応は..."), or the
    item's own source lines (read from --repo-root) open with that code
    after stripping list markers/bold/whitespace -- e.g. a source line
    "- **OOS-001** 仕様書同士の品質監査: ..." makes OOS-001 a self-citation
    for that item even where the statement carries the code only as a
    trailing parenthetical ("...（OOS-001 仕様書同士の品質監査）"). Also drops any
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
    Before matching, a stored `cites` entry is normalised (normalize_cite_
    tokens): surrounding full/half-width parens are stripped ("（R-3）" ->
    "R-3"); a doc name immediately followed by a bare id or code drops the
    doc name ("要件定義 R-2" -> "R-2"); a joined list splits into one
    section per entry on "・"/"、"/"/"/"／"/"および" ("要件定義 §5.2、§28"
    -> two entries), including a section immediately followed by a bare
    code with no separator at all ("基本仕様 §29 OOS-005" -> two entries);
    a cites entry that is nothing but one document name, with no section,
    stays unresolved but is reported as resolution "doc-only" rather than
    silently producing zero entries. This normalization only affects how
    a `cites` string is read here -- it never rewrites the fragment.
    Writes three files under --root:
      derivation-candidates.json -- one entry per (statement, normalised
        atomic cite) pair for every statement carrying a non-empty
        "cites": {id, statement (first 80 chars), cite, candidates:[{id,
        statement (first 60 chars)}], resolution, layer_relation}.
        resolution is "exact" (cite is an id that exists, or a bare
        NFR/OOS/F code whose target statement names itself and there is
        exactly one such statement -- searched in request+require only,
        since a spec-layer statement can restate the same code in its own
        words without being a second definition of it), "section" (cite
        names a document + section/line and at least one statement in
        that document's heading/line range matches), "doc-only" (cite is
        a bare document name), or "unresolved" (cite recognised but zero
        or ambiguous matches, or not recognised at all). layer_relation is
        computed from (citing statement's layer, resolved target's layer)
        over the fixed order request<require<spec<design (every design
        area is layer "design" regardless of which of 本冊/別紙A/別紙C):
        "adjacent-upstream" (target exactly one layer up), "skip-upstream"
        (two or more layers up), "same-layer", "downstream" (target layer
        later than source), or null when the target layer can't be
        determined at all.
      derivation-table-candidates.json -- one entry per data row of the
        付記 導出表 (its two md tables: 第I部 根->要求, 第II部 要求->要件；
        header and separator rows skipped by shape, not by hard-coded
        line numbers): {row_line, target_section, target_ids, source_ids,
        kind, kind_text, scribe, note}. The table has no separate 区分
        column (its columns are 上流ノード/下流ノード/導出理由/状態);
        "kind" carries the 状態 column verbatim (always "ACCEPTED" in the
        current table), "kind_text" carries 導出理由 verbatim, "scribe" is
        true iff 導出理由 contains "書記", and "note" carries all three
        text columns verbatim so nothing is lost regardless of what
        resolved. Each of 上流ノード and 下流ノード is split on "・"/"/"
        (both used as multi-reference separators in this table -- e.g.
        "§12/§19", "R-2 / OOS-005 ...") and every token is resolved as an
        id (R-N/P-00N), a require-layer section/sub-section (§N or §N.N,
        matching the leading number token of a heading like "### 3.2 ...")
        or a require-layer self-naming code (NFR-00N/OOS-00N); F<n> and
        "#11"-style Issue references never resolve to an id and stay as
        plain text in "note" only. "target_section" is set to whatever was
        parsed from 下流ノード regardless of resolution success -- the
        matched id(s), the matched "§N" text(s), or both joined with "/"
        for a compound cell -- so it documents what the cell said even
        when target_ids ends up empty; it is null only when 下流ノード
        parsed as nothing recognisable at all. A large share of
        "source_ids": [] rows are not a parsing failure: 上流ノード often
        names an Issue #11 freeze item (F<n>), an Issue number, or an
        "Owner 裁定"/"U-0N 裁定" label, none of which correspond to any
        statement id in this corpus -- their text is preserved in "note".
      derivation-candidates.md -- human-readable: a summary (counts by
        resolution including doc-only, by layer_relation, table-row counts
        by kind, and a scribe-row count), then per source document a
        bullet list of (statement, cite) pairs and their candidates (top
        5, "+k more" beyond that) restricted to layer_relation =
        adjacent-upstream (the actual approval list), a collapsed
        skip-upstream section as an open question for the Owner, and
        finally the distinct unresolved cite strings with their counts.
    Prints the same summary to stdout. Fully deterministic (iterates
    specification.json's own array order; table rows in line order).

qualifier-check
    A mechanical transcription-fidelity check, independent of derivation.
    Operates on fragments directly (not specification.json), like coverage
    and harvest-cites. For a fixed token set of limiting/qualifying words
    (ただし, のみ, に限り, 限る, 必須, してはならない, しない, 任意, かつ,
    または, すべて, 全て, "1 件以上", ちょうど, 禁止, 推測, §), compares per
    **cluster**, not per line: an item's source.lines ranges are merged
    into maximal runs wherever they overlap or touch (zero-line gap) --
    design areas contribute nothing to this (an area is not an item, same
    principle as the coverage command) and dropped-log ranges play no
    part in cluster formation either. For each cluster, count each
    token's occurrences in the concatenated source lines of the cluster
    versus in the concatenation of statement+description of every item
    whose range is part of that cluster -- except for "§" specifically,
    where the transcribed side also includes each item's `cites` (joined),
    since a dropped section reference legitimately moving into `cites` is
    the whole point of harvest-cites, not a transcription defect. Before
    counting, both sides are normalised: every ASCII/full-width space and
    every Markdown "*" or "`" is stripped (this collapses "1 件以上" and
    "1件以上" into the same string, so the token list is deduped after
    normalisation rather than reporting the same match under two labels).
    A cluster with no item at all (a line no item's range covers) simply
    doesn't exist -- that's a coverage gap, not a transcription-fidelity
    question; see the `coverage` command. Report every (cluster, token)
    pair where the two counts differ, with the cluster's line range and
    the first 100 (unnormalised) characters of its source. Comparing per
    cluster instead of per line is what makes a single real occurrence
    inside a multi-item span stop looking like a mismatch on every other
    line of that span.
    Writes docs/canonical/qualifier-check.md (grouped by doc, each row:
    cluster line range, token, source count, transcribed count, source
    excerpt, plus a per-doc total) and prints the same per-doc totals plus
    the top 15 (token, direction) pairs to stdout, where direction is
    "dropped" (source count > transcribed count) or "added" (the reverse).

source-check
    A mechanical, item-scoped range-sanity check, independent of the
    token-count checks above. Operates on fragments directly (leaf items
    only -- request/require/spec items and design nested items, not
    design areas, since areas have no "statement"). For every item:
      1. Range boundary check: the source md line at source.lines[0] (the
         first line) or source.lines[1] (the last line) is blank, an ATX
         heading, or a code-fence delimiter. A well-formed range should
         start and end on actual content, so either boundary landing on a
         non-content line usually means the range is off by one or
         otherwise mis-scoped.
      2. Backtick-token coverage: every `` `...` `` quoted token inside
         the statement is looked for anywhere across the item's whole
         source range (not just one line); if fewer than half of the
         statement's backtick tokens appear anywhere in the range, flag
         it (statements with no backtick tokens are not checked).
      3. Six-character-substring overlap: strip spaces and backticks from
         the statement and, separately, from each line of the item's
         range; if the statement (once cleaned) is at least 6 characters
         long and none of its 6-character substrings appears in any
         cleaned range line, flag it -- this is a coarse "did this get
         transcribed from the right place at all" signal. A statement
         shorter than 6 characters after cleaning is not checked (nothing
         to slide a 6-character window over).
    Prints per-fragment-file counts (items scanned, and how many tripped
    each of the three checks) and the offending items' location (heading
    + line range -- fragments have no stamped id yet) and detail. Writes
    docs/canonical/source-check.md with the same information. Exit 1 iff
    check 1 or check 3 found anything (check 2 is reported but does not
    gate the exit code).

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

    # doc -> item-level ranges (for overlap check + coverage). Per
    # CONVERSION.md SS5, only an item's source.lines counts as covered --
    # a design area's own source.lines is not an item and earns no
    # coverage credit; it is tracked nowhere here on purpose (a line
    # inside an area but outside every item and every dropped range must
    # surface as uncovered).
    item_ranges: dict[str, list[tuple[int, int, str]]] = {}
    docs_seen: list[str] = []

    def note_doc(doc: str) -> None:
        if doc not in item_ranges:
            item_ranges[doc] = []
            docs_seen.append(doc)

    for path, frag in fragments:
        layer = frag.get("layer")
        if layer == "design":
            for area in frag.get("areas", []):
                asrc = area["source"]
                note_doc(asrc["doc"])
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


# Self-citation rule 2: a bare code is also a self-citation when the
# item's OWN source lines define it -- the code sits at the start of one
# of those raw md lines, once list markers ("- ", "1. "), bold markers
# ("**"), and whitespace are stripped. This catches the shape base.json
# now uses, where the statement carries the code only as a trailing
# parenthetical ("...（OOS-001 仕様書同士の品質監査）") while the source line
# itself still opens with "OOS-001 ...".
_LEADING_MARKUP_RE = re.compile(r"^\s*(?:[-*・•]\s+|[0-9]+[.)]\s+)?\*{0,2}")
_HEAD_BARE_CODE_RE = re.compile(r"^(P-[0-9]{3}|R-[1-5]|F[0-9]+|OOS-[0-9]{3}|NFR-[0-9]{3})")


def self_definition_codes_in_range(md_lines: list[str], s: int, e: int) -> frozenset:
    """Bare codes that open (after markup-stripping) any line in [s, e]
    of this item's own source doc -- i.e. codes this item's source range
    itself defines."""
    codes = set()
    for ln in range(s, e + 1):
        if not (1 <= ln <= len(md_lines)):
            continue
        line = md_lines[ln - 1]
        m_lead = _LEADING_MARKUP_RE.match(line)
        head = line[m_lead.end():] if m_lead else line
        m_code = _HEAD_BARE_CODE_RE.match(head)
        if m_code:
            codes.add(m_code.group(1))
    return frozenset(codes)


def extract_citations(statement: str, self_definition_codes: frozenset = frozenset()) -> list[str]:
    """Citations found in `statement`, in order of appearance, not deduped.

    A bare code (NFR-00N/OOS-00N/P-00N/R-N/F<n>) is a self-citation --
    not harvested as a citation of something else -- in either of two
    cases: (1) it is matched at the very start of the statement (ignoring
    leading whitespace) -- e.g. "OOS-001仕様書同士の品質監査について..." --
    regardless of what follows it (a separator, straight into kanji, or
    nothing at all; only the position matters); or (2) it is one of
    `self_definition_codes` -- codes this item's own source lines define
    -- regardless of where in the statement it sits, since a statement can
    carry the same code only as a trailing parenthetical while still being
    the code's own definition.
    """
    found = []
    lead = len(statement) - len(statement.lstrip())
    for m in CITATION_RE.finditer(statement):
        if m.group("secdoc"):
            found.extend(split_seclist(m.group("secdoc"), m.group("seclist")))
        elif m.group("ldoc"):
            found.append(f"{m.group('ldoc')} L{m.group('lnum')}")
        elif m.group("pdoc"):
            found.append(f"{m.group('pdoc')} {m.group('pnum')}")
        elif m.group("bare"):
            code = m.group("bare")
            if m.start() == lead or code in self_definition_codes:
                continue
            found.append(code)
    return found


# CONVERSION.md SS3: same-document references (no doc name) stay inline in
# the statement and are not citations. A cites entry that is bare -- "§N",
# "§N.N", or a joined list of them ("§4.1・§4.4", "§5.1、§23") -- starts
# with "§" once stripped; a doc-named or R-/P-/F/OOS-/NFR- entry never does.
_BARE_SECTION_ENTRY_RE = re.compile(r"^§")


def is_bare_section_entry(cite: str) -> bool:
    return bool(_BARE_SECTION_ENTRY_RE.match(cite.strip()))


def harvest_item(item: dict, self_definition_codes: frozenset = frozenset()) -> tuple[list[str], int]:
    """Mutate item['cites'] in place: drop bare same-document section
    entries, then append newly-found citations from 'statement' (existing
    surviving entries and their order untouched; no duplicates). Deletes
    'cites' entirely if it would end up empty. Never touches 'statement'
    or 'derived_from'. Returns (added, removed_count)."""
    existing_list = list(item["cites"]) if isinstance(item.get("cites"), list) else []
    kept = [c for c in existing_list if not is_bare_section_entry(c)]
    removed = len(existing_list) - len(kept)

    found = extract_citations(item.get("statement", ""), self_definition_codes)
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
    repo_root = Path(args.repo_root)
    files = list_fragment_files(root)
    if not files:
        print("harvest-cites: no fragments found")
        return 0

    md_cache: dict = {}

    def get_md_lines(doc: str) -> list[str]:
        if doc not in md_cache:
            try:
                md_cache[doc] = (repo_root / doc).read_text(encoding="utf-8").splitlines()
            except OSError:
                md_cache[doc] = []
        return md_cache[doc]

    for path in files:
        frag = read_json(path)
        items = list(iter_leaf_items(frag))
        added_all: list[str] = []
        removed_total = 0
        for it in items:
            s, e = it["source"]["lines"]
            self_codes = self_definition_codes_in_range(get_md_lines(it["source"]["doc"]), s, e)
            added, removed = harvest_item(it, self_codes)
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
    """Statements whose own text starts with `code` (P-00N/R-N/NFR-00N/
    OOS-00N/F<n> are all named -- defined -- in request or require layer
    per CONVERSION.md SS1; a spec-layer statement can restate the same
    code in its own words without being a second definition of it), so
    the search is scoped to request+require to avoid a spurious
    cross-layer ambiguity (e.g. 基本仕様 restating "OOS-005 ..." in its
    own §25-equivalent text must not compete with the require-layer
    statement that actually names OOS-005)."""
    return [
        iid for iid, it in id_index.items()
        if it["layer"] in ("request", "require") and it["statement"].startswith(code)
    ]


# --- Output 1: resolve a stored `cites` string to candidate ids. Fragments
# authored via harvest-cites already store one of four canonical shapes,
# but cites entries can also come from manual/legacy authoring in other
# shapes (parenthesised, doc+bare-id, an un-split joined list, a section
# immediately followed by a bare code, or a bare document name with no
# section at all) -- normalize_cite_tokens() below turns any of those into
# zero or more atomic tokens before resolve_cite() ever sees them.

_CITE_SECTION_RE = re.compile(r"^(要件定義|基本仕様|詳細設計|本冊|別紙[ABC])\s*§\s*([0-9]+(?:\.[0-9]+)*)$")
_CITE_LINE_RE = re.compile(r"^(要件定義|基本仕様|詳細設計|本冊|別紙[ABC])\s*L\s*([0-9]+)$")
_CITE_PDOC_RE = re.compile(r"^(要件定義|基本仕様)\s*(P-[0-9]{3})$")
_CITE_ID_RE = re.compile(r"^(R-[1-5]|P-[0-9]{3})$")
_CITE_SELFNAME_RE = re.compile(r"^(NFR-[0-9]{3}|OOS-[0-9]{3}|F[0-9]+)$")

_CITE_PAREN_RE = re.compile(r"^[（(]\s*(.+?)\s*[）)]$")
_CITE_DOC_ONLY_RE = re.compile(r"^(要件定義|基本仕様|詳細設計|本冊|別紙[ABC])$")
_CITE_JOIN_SEP_RE = re.compile(r"\s*(?:[・、/／]|および)\s*")

# A derivation-candidates-specific scanner, deliberately separate from
# harvest-cites' module-level CITATION_RE: this one's seclist repeat group
# also accepts "および" as a joining word (harvest-cites' own citation
# grammar is unchanged; only cites-field normalization here is more
# permissive), and it must NOT carry harvest-cites' start-of-statement
# self-naming exclusion -- a stored cites entry that is just "R-3" is a
# real citation of R-3, not a statement naming itself.
_DERIV_SCAN_RE = re.compile(
    r"(?P<secdoc>要件定義|基本仕様|詳細設計|本冊|別紙[ABC])\s*"
    r"(?P<seclist>§\s*[0-9]+(?:\.[0-9]+)*(?:\s*(?:[・、/／]|および)\s*§?\s*[0-9]+(?:\.[0-9]+)*)*)"
    r"|(?P<ldoc>要件定義|基本仕様|詳細設計|本冊|別紙[ABC])\s*L\s*(?P<lnum>[0-9]+)"
    r"|(?P<pdoc>要件定義|基本仕様)\s*(?P<pnum>P-[0-9]{3})"
    r"|(?<![A-Za-z0-9])(?P<bare>P-[0-9]{3}|R-[1-5]|F[0-9]+|OOS-[0-9]{3}|NFR-[0-9]{3})(?![A-Za-z0-9])"
)


def strip_cite_parens(s: str) -> str:
    m = _CITE_PAREN_RE.match(s.strip())
    return m.group(1).strip() if m else s.strip()


def split_seclist_for_cite(doc: str, seclist: str) -> list:
    """Same job as harvest-cites' split_seclist, but the separator set
    also includes "および" (manually-authored cites use it too)."""
    out = []
    for part in _CITE_JOIN_SEP_RE.split(seclist):
        part = part.strip()
        if not part:
            continue
        if not part.startswith("§"):
            part = "§" + part
        out.append(f"{doc} §{part[1:].strip()}")
    return out


def normalize_cite_tokens(raw_cite: str) -> list:
    """One stored `cites` entry -> [(atomic_cite, is_doc_only), ...]. A
    joined list, a doc name immediately followed by a bare code, or a
    doc-prefixed bare id all split into the atomic shapes harvest-cites
    would have produced at the point of authoring. A cites entry that is
    nothing but one document name (after stripping surrounding parens) is
    reported as doc-only rather than run through the scanner (which would
    silently find nothing for it)."""
    s = strip_cite_parens(raw_cite)
    if _CITE_DOC_ONLY_RE.match(s):
        return [(s, True)]
    out = []
    for m in _DERIV_SCAN_RE.finditer(s):
        if m.group("secdoc"):
            out.extend((tok, False) for tok in split_seclist_for_cite(m.group("secdoc"), m.group("seclist")))
        elif m.group("ldoc"):
            out.append((f"{m.group('ldoc')} L{m.group('lnum')}", False))
        elif m.group("pdoc"):
            out.append((f"{m.group('pdoc')} {m.group('pnum')}", False))
        elif m.group("bare"):
            out.append((m.group("bare"), False))
    return out


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
    """-> (candidate_ids: list[str], resolution: 'exact'|'section'|
    'doc-only'|'unresolved', target_layer: str|None). Expects an already
    atomic token from normalize_cite_tokens(); doc-only tokens are handled
    by the caller before this is reached."""
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
        for raw_cite in it.get("cites") or []:
            tokens = normalize_cite_tokens(raw_cite) or [(raw_cite, False)]
            for atomic_cite, is_doc_only in tokens:
                if is_doc_only:
                    scope = _DOC_TO_SCOPE.get(atomic_cite)
                    cands, resolution, target_layer = [], "doc-only", (scope_to_layer(scope) if scope else None)
                else:
                    cands, resolution, target_layer = resolve_cite(spec, id_index, atomic_cite)
                entries.append({
                    "id": it["id"],
                    "statement": it["statement"][:80],
                    "cite": atomic_cite,
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
# F<n> (Issue #11 freeze items) and "#11" itself deliberately have no
# pattern here: they name no statement in this corpus and stay as plain
# text in `note`, never attempted as an id.
_TABLE_SELFNAME_RE = re.compile(r"^(NFR-[0-9]{3}|OOS-[0-9]{3})(?![0-9A-Za-z])")
_TABLE_SCRIBE_RE = re.compile("書記")


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
    cell. kind is 'id' | 'section' | 'selfname' | 'unknown'. matched_text
    is the literal id ("R-1", "OOS-005") or the parsed section marker
    ("§3.2") -- whichever the caller should surface as target_section."""
    token = token.strip()
    m = _TABLE_ID_RE.match(token)
    if m:
        return "id", m.group(1), m.group(1)
    m = _TABLE_SECTION_RE.match(token)
    if m:
        return "section", m.group(1), m.group(0)
    m = _TABLE_SELFNAME_RE.match(token)
    if m:
        return "selfname", m.group(1), m.group(1)
    return "unknown", None, None


def self_naming_candidates_scoped(spec: dict, scope: str, code: str) -> list:
    """Like self_naming_candidates, but scoped to one layer via `scope`
    (scope_items' vocabulary) instead of the whole corpus -- this table's
    上流ノード/下流ノード only ever name request/require statements, and a
    global search would wrongly also catch a spec-layer statement merely
    restating the same code in its own words (see self_naming_candidates)."""
    return [it["id"] for it in scope_items(spec, scope) if it["statement"].startswith(code)]


def resolve_table_ref(spec: dict, id_index: dict, token: str):
    """-> (candidate_ids: list[str], matched_text: str|None). matched_text
    is returned whenever the token was recognised as id/section/selfname,
    regardless of whether it actually resolved to any candidate -- it
    documents what was parsed from the cell, not whether resolution
    succeeded (that's what an empty candidate list already says)."""
    kind, key, matched = classify_table_ref(token)
    if kind == "id":
        return ([key] if key in id_index else []), matched
    if kind == "section":
        return section_candidates(spec, "require", key), matched
    if kind == "selfname":
        return self_naming_candidates_scoped(spec, "require", key), matched
    return [], None


def resolve_table_cell(spec: dict, id_index: dict, cell: str):
    """Split a 上流ノード/下流ノード cell on ・ and / (both are used as
    multi-reference separators in this table) and resolve every token.
    -> (ids: list[str] deduped in order, matched_markers: list[str])."""
    ids: list = []
    seen = set()
    markers: list = []
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
            markers.append(matched)
    return ids, markers


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
                    "kind_text": None,
                    "scribe": False,
                    "note": "列数が4でない行（そのまま記録）: " + " | ".join(cells),
                })
                continue
            upstream, downstream, reason, state = cells
            target_ids, target_markers = resolve_table_cell(spec, id_index, downstream)
            source_ids, _src_markers = resolve_table_cell(spec, id_index, upstream)
            rows_out.append({
                "row_line": line_no,
                "target_section": "/".join(target_markers) if target_markers else None,
                "target_ids": target_ids,
                "source_ids": source_ids,
                "kind": state,
                "kind_text": reason,
                "scribe": bool(_TABLE_SCRIBE_RE.search(reason)),
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


_RELATION_ORDER = ("adjacent-upstream", "skip-upstream", "same-layer", "downstream")
_RESOLUTION_ORDER = ("exact", "section", "doc-only", "unresolved")


def summarize_derivation(entries: list, table_rows: list) -> dict:
    resolution_counts = Counter(e["resolution"] for e in entries)
    relation_counts = Counter(e["layer_relation"] for e in entries)
    kind_counts = Counter(r["kind"] for r in table_rows)
    unresolved_cites = Counter(e["cite"] for e in entries if e["resolution"] == "unresolved")
    scribe_rows = sum(1 for r in table_rows if r.get("scribe"))
    return {
        "resolution_counts": resolution_counts,
        "relation_counts": relation_counts,
        "candidate_pairs": len(entries),
        "table_rows": len(table_rows),
        "kind_counts": kind_counts,
        "unresolved_cites": unresolved_cites,
        "scribe_rows": scribe_rows,
    }


def print_derivation_summary(summary: dict) -> None:
    print("--- derivation-candidates summary ---")
    for res in _RESOLUTION_ORDER:
        print(f"  resolution {res}: {summary['resolution_counts'].get(res, 0)}")
    for rel in _RELATION_ORDER:
        print(f"  layer_relation {rel}: {summary['relation_counts'].get(rel, 0)}")
    unknown_rel = summary["relation_counts"].get(None, 0)
    if unknown_rel:
        print(f"  layer_relation (undetermined): {unknown_rel}")
    print(f"  candidate pairs total: {summary['candidate_pairs']}")
    print(f"  distinct unresolved cite strings: {len(summary['unresolved_cites'])}")
    print(f"  derivation-table rows total: {summary['table_rows']}")
    for kind, n in sorted(summary["kind_counts"].items(), key=lambda kv: (kv[0] is None, kv[0] or "")):
        print(f"  table rows kind={kind!r}: {n}")
    print(f"  table rows scribe=True: {summary['scribe_rows']}")


def _format_entry_bullet(e: dict) -> str:
    cands = e["candidates"]
    shown = cands[:5]
    cand_text = ", ".join(f"{c['id']} 「{c['statement']}」" for c in shown)
    if len(cands) > 5:
        cand_text += f" +{len(cands) - 5} more"
    if not cands:
        cand_text = "(候補なし)"
    return f"- {e['id']} 「{e['statement'][:60]}」 ← {e['cite']} → {len(cands)}件: {cand_text}"


def write_derivation_md(path: Path, entries: list, table_rows: list, summary: dict) -> None:
    lines = [
        "<!-- generated from docs/canonical/specification.json and the 付記 導出表 by build.py derivation-candidates; do not edit -->",
        "",
        "# 導出候補（機械生成・Owner 未承認）",
        "",
        "CONVERSION.md SS6 の手順1-2の出力。推論・採点・選定は行っていない。承認して derived_from へ入れるかは Owner の判断。",
        "承認リストに載るのは layer_relation = adjacent-upstream の候補のみ（既定）。skip-upstream は下の折りたたみで別掲（Owner への疑問）。"
        " same-layer / downstream は集計のみで一覧化しない。JSON にはどちらも全件を relation 付きで保持する。",
        "",
        "## 集計",
        "",
        "| resolution | 件数 |",
        "|---|---|",
    ]
    for res in _RESOLUTION_ORDER:
        lines.append(f"| {res} | {summary['resolution_counts'].get(res, 0)} |")
    lines.append(f"| **候補ペア合計** | **{summary['candidate_pairs']}** |")

    lines += ["", "| layer_relation | 件数 |", "|---|---|"]
    for rel in _RELATION_ORDER:
        lines.append(f"| {rel} | {summary['relation_counts'].get(rel, 0)} |")
    unknown_rel = summary["relation_counts"].get(None, 0)
    if unknown_rel:
        lines.append(f"| (未確定) | {unknown_rel} |")

    lines += ["", "| 導出表 状態（kind） | 件数 |", "|---|---|"]
    for kind, n in sorted(summary["kind_counts"].items(), key=lambda kv: (kv[0] is None, kv[0] or "")):
        lines.append(f"| {kind if kind is not None else '(列数異常)'} | {n} |")
    lines.append(f"| **導出表行合計** | **{summary['table_rows']}** |")
    lines.append(f"| うち scribe（導出理由に「書記」を含む） | {summary['scribe_rows']} |")

    by_doc: dict = {}
    for e in entries:
        if e["layer_relation"] == "adjacent-upstream":
            by_doc.setdefault(e["_doc"], []).append(e)

    for doc in sorted(by_doc):
        lines += ["", f"## {doc_display_name(doc)}", ""]
        for e in by_doc[doc]:
            lines.append(_format_entry_bullet(e))

    skip_entries = [e for e in entries if e["layer_relation"] == "skip-upstream"]
    if skip_entries:
        skip_by_doc: dict = {}
        for e in skip_entries:
            skip_by_doc.setdefault(e["_doc"], []).append(e)
        lines += [
            "",
            "## skip-upstream（Owner への疑問。1層より遠くへの飛び越え引用）",
            "",
            "<details>",
            f"<summary>{len(skip_entries)} 件</summary>",
            "",
        ]
        for doc in sorted(skip_by_doc):
            lines += ["", f"### {doc_display_name(doc)}", ""]
            for e in skip_by_doc[doc]:
                lines.append(_format_entry_bullet(e))
        lines += ["", "</details>"]

    unresolved_cites = summary["unresolved_cites"]
    if unresolved_cites:
        lines += [
            "",
            "## 未解決の cite 文字列（異なり数・出現回数付き）",
            "",
            f"{len(unresolved_cites)} 種類、延べ {sum(unresolved_cites.values())} 件。",
            "",
            "| cite | 件数 |",
            "|---|---|",
        ]
        for cite, n in sorted(unresolved_cites.items(), key=lambda kv: (-kv[1], kv[0])):
            lines.append(f"| {cite} | {n} |")

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


# -------------------------------------------------------- qualifier-check --

_QUALIFIER_TOKENS = [
    "ただし", "のみ", "に限り", "限る", "必須", "してはならない", "しない",
    "任意", "かつ", "または", "すべて", "全て", "1 件以上", "1件以上",
    "ちょうど", "禁止", "推測", "§",
]

# For "§" only: a dropped section reference legitimately moves from the
# statement into the item's `cites` list (that is the whole point of
# harvest-cites), so the transcribed side for this one token also counts
# `cites` entries; every other token stays statement+description only.
_QUALIFIER_CITES_TOKEN = "§"

# Both sides are normalised before counting: every ASCII and full-width
# space, and Markdown "*" emphasis and "`" backticks, are removed. This
# collapses "1 件以上" and "1件以上" into the same string, so the token
# list itself is deduped after normalisation (first spelling wins as the
# label) rather than double-reporting the same match under two names.
_QUALIFIER_NORMALIZE_RE = re.compile(r"[ 　*`]")


def normalize_for_qualifier(text: str) -> str:
    return _QUALIFIER_NORMALIZE_RE.sub("", text)


_QUALIFIER_TOKENS_NORMALIZED = list(dict.fromkeys(normalize_for_qualifier(t) for t in _QUALIFIER_TOKENS))


def gather_doc_items_and_dropped(root: Path):
    """doc -> list of leaf items (statement/description/source, straight
    from the fragments -- not specification.json), and doc -> list of
    (start, end) dropped-log ranges."""
    items_by_doc: dict = {}
    for _path, frag in load_fragments(root):
        for it in iter_leaf_items(frag):
            items_by_doc.setdefault(it["source"]["doc"], []).append(it)
    dropped_by_doc: dict = {}
    for _path, entry in load_dropped(root):
        dropped_by_doc.setdefault(entry["doc"], []).append((entry["lines"][0], entry["lines"][1]))
    return items_by_doc, dropped_by_doc


def build_clusters(items: list) -> list:
    """[(cluster_start, cluster_end, [item, ...]), ...], sorted by start.
    Built from item ranges only -- a design area contributes nothing (it
    is not an item, same principle as the coverage fix). Two item ranges
    join the same cluster when they overlap or touch (zero-line gap);
    dropped-log ranges play no role in this at all. Comparison then runs
    per cluster instead of per line, so a real occurrence that sits
    anywhere in a multi-item cluster no longer looks like a mismatch on
    every other line of that cluster."""
    spans = sorted(
        ((it["source"]["lines"][0], it["source"]["lines"][1], it) for it in items),
        key=lambda t: (t[0], t[1]),
    )
    clusters: list = []
    for s, e, it in spans:
        if clusters and s <= clusters[-1][1] + 1:
            cs, ce, citems = clusters[-1]
            clusters[-1] = (cs, max(ce, e), citems + [it])
        else:
            clusters.append((s, e, [it]))
    return clusters


def compact_source_line(line: str) -> str:
    return line if len(line) <= 100 else line[:100]


def write_qualifier_report(path: Path, report_by_doc: dict) -> dict:
    lines_out = [
        "<!-- generated by build.py qualifier-check; do not edit -->",
        "",
        "# 限定語の転記照合（機械照合）",
        "",
        "item の source.lines を重なり・隣接（隙間0行）でまとめたクラスタ単位で比較する"
        "（design の area は item ではないのでクラスタ形成に加わらない。dropped-log もクラスタ形成には関与しない）。"
        "クラスタの生テキスト連結と、そのクラスタを構成する全 item の statement+description 連結とで、"
        "固定トークン集合の出現回数を比較する。件数が一致しないクラスタ×トークンの組のみ列挙する。"
        "比較前に両辺とも正規化する: 半角/全角スペース、Markdown 強調 `*`、バッククォート `` ` `` を除去"
        "（この正規化により「1 件以上」と「1件以上」は同一文字列になるため、トークン集合側もこの2つを1件に統合済み）。"
        "「§」だけは例外で、cites への移動が正当な転記経路（harvest-cites の目的そのもの）であるため、"
        "転記側は statement+description+cites の連結で数える。"
        "どの item にもカバーされていない行（被覆の欠落は `coverage` の領分）はクラスタに含まれない。",
        "",
    ]
    totals: dict = {}
    for doc in sorted(report_by_doc):
        mismatches = report_by_doc[doc]
        totals[doc] = len(mismatches)
        lines_out += [
            f"## {doc}",
            "",
            f"件数: {len(mismatches)}",
            "",
            "| クラスタ範囲 | トークン | source | 転記 | source 先頭100文字 |",
            "|---|---|---|---|---|",
        ]
        for cs, ce, token, sc, tc, src in mismatches:
            lines_out.append(f"| {cs}-{ce} | {token} | {sc} | {tc} | {src.replace('|', chr(92) + '|')} |")
        lines_out.append("")
    text = "\n".join(lines_out).rstrip("\n") + "\n"
    path.write_text(text, encoding="utf-8")
    return totals


def cmd_qualifier_check(args) -> int:
    root = Path(args.root)
    repo_root = Path(args.repo_root)
    items_by_doc, _dropped_by_doc = gather_doc_items_and_dropped(root)

    report_by_doc: dict = {}
    for doc, items in items_by_doc.items():
        md_path = repo_root / doc
        try:
            lines = md_path.read_text(encoding="utf-8").splitlines()
        except OSError as exc:
            print(f"error: cannot read source doc {md_path}: {exc}", file=sys.stderr)
            return 1

        mismatches = []
        for cs, ce, citems in build_clusters(items):
            cluster_lines = lines[max(1, cs) - 1: min(ce, len(lines))]
            source_text_raw = "\n".join(cluster_lines)
            source_text = normalize_for_qualifier(source_text_raw)
            transcribed_text = normalize_for_qualifier("\n".join(
                it["statement"] + ("\n" + it["description"] if it.get("description") else "")
                for it in citems
            ))
            transcribed_text_with_cites = normalize_for_qualifier("\n".join(
                it["statement"]
                + ("\n" + it["description"] if it.get("description") else "")
                + ("\n" + "\n".join(it["cites"]) if it.get("cites") else "")
                for it in citems
            ))
            for token in _QUALIFIER_TOKENS_NORMALIZED:
                tc_text = transcribed_text_with_cites if token == _QUALIFIER_CITES_TOKEN else transcribed_text
                sc = source_text.count(token)
                tc = tc_text.count(token)
                if sc != tc:
                    mismatches.append((cs, ce, token, sc, tc, compact_source_line(source_text_raw)))
        if mismatches:
            report_by_doc[doc] = mismatches

    out_path = root / "qualifier-check.md"
    totals = write_qualifier_report(out_path, report_by_doc)

    print("--- qualifier-check per-doc totals ---")
    grand_total = 0
    for doc in sorted(totals):
        print(f"  {doc}: {totals[doc]}")
        grand_total += totals[doc]
    print(f"  TOTAL: {grand_total}")

    direction_counts = Counter()
    for mismatches in report_by_doc.values():
        for _cs, _ce, token, sc, tc, _src in mismatches:
            direction_counts[(token, "dropped" if sc > tc else "added")] += 1
    print("--- top 15 (token, direction) pairs ---")
    for (token, direction), n in direction_counts.most_common(15):
        print(f"  {token!r} {direction}: {n}")

    print(f"wrote {out_path}")
    return 0


# ------------------------------------------------------------- source-check --

_FENCE_RE = re.compile(r"^\s*```")
_BACKTICK_TOKEN_RE = re.compile(r"`([^`]+)`")
_SPACE_BACKTICK_RE = re.compile(r"[ 　`]")


def is_blank_heading_or_fence(line: str) -> bool:
    stripped = line.strip()
    if stripped == "":
        return True
    if HEADING_RE.match(line):
        return True
    if _FENCE_RE.match(line):
        return True
    return False


def backtick_tokens(text: str) -> list:
    return _BACKTICK_TOKEN_RE.findall(text)


def strip_spaces_and_backticks(text: str) -> str:
    return _SPACE_BACKTICK_RE.sub("", text)


def six_char_substrings(s: str) -> set:
    if len(s) < 6:
        return set()
    return {s[i:i + 6] for i in range(len(s) - 5)}


def item_location(it: dict) -> str:
    s, e = it["source"]["lines"]
    return f"{it['source']['heading']} L{s}-{e}"


def check_source_item(it: dict, md_lines: list) -> dict:
    """-> {'check1': (first_bad_or_None, last_bad_or_None),
           'check2': (total_tokens, present_count),
           'check3': True} -- only for checks that actually tripped."""
    s, e = it["source"]["lines"]
    result: dict = {}

    first_bad = md_lines[s - 1] if 1 <= s <= len(md_lines) and is_blank_heading_or_fence(md_lines[s - 1]) else None
    last_bad = md_lines[e - 1] if 1 <= e <= len(md_lines) and is_blank_heading_or_fence(md_lines[e - 1]) else None
    if first_bad is not None or last_bad is not None:
        result["check1"] = (first_bad, last_bad)

    range_lines = md_lines[max(1, s) - 1: min(e, len(md_lines))]
    tokens = backtick_tokens(it["statement"])
    if tokens:
        range_text = "\n".join(range_lines)
        present = sum(1 for t in tokens if t in range_text)
        if present < len(tokens) / 2:
            result["check2"] = (len(tokens), present)

    stmt_subs = six_char_substrings(strip_spaces_and_backticks(it["statement"]))
    if stmt_subs:
        shared = any(stmt_subs & six_char_substrings(strip_spaces_and_backticks(line)) for line in range_lines)
        if not shared:
            result["check3"] = True

    return result


def write_source_check_report(path: Path, report: dict) -> None:
    lines_out = [
        "<!-- generated by build.py source-check; do not edit -->",
        "",
        "# ソース範囲の機械点検",
        "",
        "check1: 範囲の先頭または末尾行が空行・見出し・コードフェンス。"
        " check2: statement 内のバッククォート付きトークンのうち、範囲内のどこにも現れないものが半数以上。"
        " check3: statement と範囲内のどの行との間にも、空白・バッククォートを除いた6文字部分文字列の一致が無い"
        "（statement が6文字未満になる場合は対象外）。"
        " fragment はまだ id を付番されていないため、見出し+行範囲で位置を示す。",
        "",
    ]
    for path_str in sorted(report):
        n_items, violations = report[path_str]
        lines_out += [
            f"## {path_str}",
            "",
            f"items scanned: {n_items} / check1: {len(violations['check1'])} /"
            f" check2: {len(violations['check2'])} / check3: {len(violations['check3'])}",
            "",
        ]
        if violations["check1"]:
            lines_out.append("check1 違反:")
            for loc, (first_bad, last_bad) in violations["check1"]:
                detail = []
                if first_bad is not None:
                    detail.append(f"先頭行={first_bad!r}")
                if last_bad is not None:
                    detail.append(f"末尾行={last_bad!r}")
                lines_out.append(f"- {loc}: {', '.join(detail)}")
            lines_out.append("")
        if violations["check2"]:
            lines_out.append("check2 違反:")
            for loc, (total, present) in violations["check2"]:
                lines_out.append(f"- {loc}: バッククォートトークン {total} 個中 {present} 個のみ範囲内に出現")
            lines_out.append("")
        if violations["check3"]:
            lines_out.append("check3 違反:")
            for loc in violations["check3"]:
                lines_out.append(f"- {loc}")
            lines_out.append("")

    text = "\n".join(lines_out).rstrip("\n") + "\n"
    path.write_text(text, encoding="utf-8")


def cmd_source_check(args) -> int:
    root = Path(args.root)
    repo_root = Path(args.repo_root)
    md_cache: dict = {}

    def get_md_lines(doc: str):
        if doc not in md_cache:
            md_path = repo_root / doc
            try:
                md_cache[doc] = md_path.read_text(encoding="utf-8").splitlines()
            except OSError as exc:
                print(f"error: cannot read source doc {md_path}: {exc}", file=sys.stderr)
                md_cache[doc] = None
        return md_cache[doc]

    report: dict = {}
    had_read_error = False
    for path in list_fragment_files(root):
        frag = read_json(path)
        items = list(iter_leaf_items(frag))
        violations = {"check1": [], "check2": [], "check3": []}
        for it in items:
            md_lines = get_md_lines(it["source"]["doc"])
            if md_lines is None:
                had_read_error = True
                continue
            res = check_source_item(it, md_lines)
            loc = item_location(it)
            if "check1" in res:
                violations["check1"].append((loc, res["check1"]))
            if "check2" in res:
                violations["check2"].append((loc, res["check2"]))
            if "check3" in res:
                violations["check3"].append(loc)
        if items:
            report[str(path)] = (len(items), violations)

    out_path = root / "source-check.md"
    write_source_check_report(out_path, report)

    print("--- source-check per-fragment summary ---")
    gate_violation = False
    for path_str in sorted(report):
        n_items, violations = report[path_str]
        c1, c2, c3 = len(violations["check1"]), len(violations["check2"]), len(violations["check3"])
        print(f"  {path_str}: items={n_items} check1={c1} check2={c2} check3={c3}")
        if c1 or c3:
            gate_violation = True
    print(f"wrote {out_path}")

    if had_read_error:
        return 1
    return 1 if gate_violation else 0


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
    add_repo_root_arg(p_harvest)
    p_harvest.add_argument("--dry-run", action="store_true", help="print the report without writing any fragment file")
    p_harvest.set_defaults(func=cmd_harvest_cites)

    p_deriv = sub.add_parser("derivation-candidates", help="CONVERSION.md SS6 steps 1-2: mechanical derivation candidate list")
    add_root_arg(p_deriv)
    add_repo_root_arg(p_deriv)
    p_deriv.set_defaults(func=cmd_derivation_candidates)

    p_qual = sub.add_parser("qualifier-check", help="mechanical transcription check for a fixed set of limiting/qualifying tokens")
    add_root_arg(p_qual)
    add_repo_root_arg(p_qual)
    p_qual.set_defaults(func=cmd_qualifier_check)

    p_src = sub.add_parser("source-check", help="item-scoped range-sanity checks (boundary, backtick coverage, substring overlap)")
    add_root_arg(p_src)
    add_repo_root_arg(p_src)
    p_src.set_defaults(func=cmd_source_check)

    args = parser.parse_args(argv)
    return args.func(args)


if __name__ == "__main__":
    sys.exit(main())
