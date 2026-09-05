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
    apply-derivation is not part of this chain (2026-09-05 ID freeze
    revision): once ids are frozen, `build` passes each fragment item's
    stored derived_from through unchanged, and apply-derivation is a
    separate, explicit command (report-only by default; --write to
    apply) run deliberately, not on every build.

check-fragment <path>
    Validate one fragment file's structural shape (not the final
    schema): required keys present, source.lines is a valid [start, end]
    pair. Since the 2026-09-05 ID freeze, a stored "id" (item or design
    area) and a non-empty "derived_from" are legitimate -- both are
    checked against the schema's id pattern rather than rejected
    outright. Exit 1 on any problem.

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
        over the fixed order request<require<spec<basic_design<design (every design
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

apply-derivation
    CONVERSION.md SS6: Owner decision that derived_from is a reference,
    not a proof -- it is computed mechanically, no pairwise approval.
    Reads specification.json; a plain run only REPORTS what it would
    change (report-only by default, since 2026-09-05's ID-freeze
    revision -- see `freeze` below); pass --write to actually overwrite
    specification.json's derived_from with the recomputed set (still
    never touches fragments -- that is `freeze`'s job).
    Edges come from three sources, reusing the exact same candidate logic
    as `derivation-candidates` for sources 1-2 rather than a separate
    resolver:
    Source 1 (cites): for every statement with cites, every candidate
    whose layer_relation is "adjacent-upstream" or "skip-upstream" is
    added; "same-layer", "downstream", "unresolved" and "doc-only" add
    nothing.
    Source 2 (要求→要件 derivation table): for every row with both
    non-empty source_ids and target_ids, every source_id is added to
    every target_id in that row; a row whose 上流 is only an Issue/F-item
    (no ids) contributes nothing.
    Source 3 -- Task A, 2026-09-05 (each document's own trailing 「付記
    （非規範）: トレーサビリティ表」, found via the dropped-log reason text
    containing "トレーサビリティ表", not a hard-coded location): a row's
    own section (leftmost column, "§N"/"§N.N") names a source section
    within THAT document; every statement whose heading begins with that
    number gets every candidate from the row's upstream column (中列)
    added. Upstream tokens can be a bare id (P-00N/R-N/NFR-00N/OOS-00N),
    or a §-list, and for 本冊/別紙A/別紙C tables every §-list has a leading
    abbreviated doc name (本冊/基本/要件) that can change mid-cell without
    a separator (inspected: neither table ever actually has a bare §-list
    with no doc name); 基本仕様's own table has no doc names at all and
    every bare §-list defaults to 要件定義 (its one upstream doc, again
    confirmed empirically: no row names one explicitly). A trailing
    "itemN" or "itemN-M" qualifier (e.g. "基本§30 item18") is recognised
    and dropped -- resolution stays at the section, not the sub-item,
    level, since there is no per-item heading data to match against.
    Rule 3 (applied to the union of all three sources' contributions, per
    statement): dedup; drop the statement's own id; drop any id whose
    layer is the same as or later than (>=) the statement's own layer in
    the fixed order request<require<spec<basic_design<design (derived_from only ever
    points strictly upstream -- this is also what makes a 別紙 citing 本冊
    produce no edge, since both are layer "design") -- both kinds of drop
    are counted and reported. What survives is sorted by (layer order,
    then the id's trailing number).
    Request-layer statements are skipped entirely (the rootItem schema
    has no derived_from field).
    In --write mode: verifies every remaining derived_from id actually
    exists in specification.json (exit 1 otherwise, before writing) and
    re-validates the whole document against specification.schema.json
    (exit 1 on any schema error, before writing) -- only then writes.
    Prints, always: the Task A row-parse report (rows parsed per table,
    any unparseable row verbatim, statements-with-derived_from per layer
    before vs after Task A, raw edge count Task A added), then a
    stored-vs-recomputed diff count; in --write mode, also the usual
    summary (statements with non-empty derived_from per layer, total
    edges, mean/max size, statements with cites but empty derived_from,
    count dropped by rule 3).
    Deliberately not part of `all` (see `all` above and `freeze` below).

freeze
    CONVERSION.md SS6/SS7 (2026-09-05 ID freeze). A one-way operation:
    builds specification.json in memory exactly as `build` would (same
    sort, same id assignment -- a fragment item/area that already has a
    stored "id" keeps it; a new one gets the next unused number for its
    prefix; a duplicate stored id is a hard error, same as `build`), then
    writes each item/area's assigned id, and its derived_from computed
    the same way apply-derivation --write would (all three edge sources,
    rule 3 filtering), directly into the *fragment* JSON files (same
    formatting: UTF-8, ensure_ascii=False, indent 2, trailing newline).
    A "keep_id" is removed once its value is promoted into "id" (the two
    would otherwise be redundant). Every fragment file that contributed
    at least one item or area is rewritten, even if nothing in it
    actually changed. Prints, per fragment file, how many items+areas
    were stamped, and the totals.
    After this runs, ids in fragments are load-bearing: `build` will keep
    reusing them, and re-running `freeze` mid-stream should be a no-op
    for any item that was already frozen (its stored id and, unless
    apply-derivation has re-run, its derived_from do not change) --
    still, only run it deliberately, not as part of `all`.

Layer split (2026-09-05, Owner): detailed_spec, basic_design
--------------------------------------------------------------
Two layers inserted between spec and design: request < require < spec <
detailed_spec < basic_design < design. detailed_spec (詳細仕様, prefix
DS-, area prefix DSA-) and basic_design (基本設計, prefix BD-, area prefix
BDA-) both have the same area-based shape as design (schema_defs
designArea, reused verbatim for all three) and their own top-level
specification.json array, export file (detailed_spec.md 「詳細仕様」,
basic_design.md 「基本設計」), and id-stamping pass. Everything that
touches "the area-layers" (_AREA_LAYERS) or "the flat layers"
(_FLAT_LAYERS) is written generically over those tuples, so a further
layer is a small, mechanical addition, not a rewrite.
A fragment still declares one document-level "layer", which fixes its
on-disk shape (flat items for request/require/spec; areas for
detailed_spec/basic_design/design). Independently, any item may carry its
own optional "layer" field (spec|detailed_spec|basic_design|design,
_RELAYER_TARGET_LAYERS) that overrides where it lands at build time --
see effective_layer(). This is the relayer mechanism: it never touches a
fragment's shape, only routes individual items.
An item promoted out of a flat layer (e.g. spec) into an area-based one
gets a synthetic one-item area, titled from its own heading text. An item
whose fragment already puts it in an area keeps that area as its grouping
key when redirected to a *different* area-layer; if only some of an
area's items move, the area is represented once per layer that still has
items in it (same title/description/source, disjoint item sets) -- so the
same original area can appear in more than one of detailed_spec/
basic_design/design at once. See build_in_memory's docstring for the
id-stability caveat this creates for a split area (not a live concern
until freeze and relayer are both in use at once).

relayer apply <mapping.json>
    Reads a JSON object {"<id or '<doc>:<start>-<end>'>": {"layer":
    "spec"|"detailed_spec"|"basic_design"|"design", "reason", "confidence":
    "high"|"low", "code_like": bool}}, resolves each key against the
    *current* build (an id from the most recent build, or a literal
    doc+line-range for content that has no id yet), and writes only the
    "layer" field into that fragment item, in place -- reason/confidence/
    code_like are never persisted to the fragment, only echoed in the
    printed report (with confidence=low and code_like=true entries called
    out separately, so they can be routed to the Owner/queued for later
    cleanup). Unresolved keys are reported, not treated as fatal. Only
    fragment files that actually changed are rewritten.

relayer report
    Prints, for every leaf item across all fragments, a count grouped by
    (its source doc, its effective layer) -- i.e. what the *next* build
    would actually produce, without running one.

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
Items are sorted by (layer order request<require<spec<basic_design<design, doc
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

# 2026-09-05: two layers inserted between spec and design, per the Owner's
# layer split -- detailed_spec (詳細仕様, DS-/area DSA-) then basic_design
# (基本設計, BD-/area BDA-). request<require<spec are always flat (no
# areas); detailed_spec/basic_design/design are always area-based -- an
# item's area is keyed by (target layer, doc, top-level heading), so the
# same original area can end up represented in more than one of them (see
# relayer below). Everything downstream of these two tuples and two dicts
# is written to generalize over however many area-layers there are, so
# adding a further one is just adding it here.
LAYER_ORDER = {"request": 0, "require": 1, "spec": 2, "detailed_spec": 3, "basic_design": 4, "design": 5}
LAYERS = tuple(LAYER_ORDER)
_FLAT_LAYERS = ("request", "require", "spec")
_AREA_LAYERS = ("detailed_spec", "basic_design", "design")
# Valid targets for an item's "layer" override (relayer): spec, plus every
# area-layer. request/require are never override targets.
_RELAYER_TARGET_LAYERS = ("spec",) + _AREA_LAYERS
LEAF_PREFIX = {"request": "R", "require": "REQ", "spec": "SPEC", "detailed_spec": "DS", "basic_design": "BD", "design": "DES"}
AREA_PREFIX = {"detailed_spec": "DSA", "basic_design": "BDA", "design": "DA"}
ID_PATTERN = re.compile(
    r"^(R-[0-9]+|P-[0-9]{3}|REQ-[0-9]{3,}|SPEC-[0-9]{3,}|DS-[0-9]{3,}|DSA-[0-9]{3,}"
    r"|BD-[0-9]{3,}|BDA-[0-9]{3,}|DES-[0-9]{3,}|DA-[0-9]{3,})$"
)
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
    if layer in _AREA_LAYERS:
        return (design_doc_rank(doc), doc)
    return (0, doc)


def item_sort_key(layer: str, item: dict):
    src = item["source"]
    return (LAYER_ORDER[layer], doc_key(layer, src["doc"]), src["lines"][0])


_ID_NUM_TRAIL_RE = re.compile(r"([0-9]+)$")


def assign_ids(items: list[dict], prefix: str) -> list[str]:
    """Stamp item['_id'] in place, in the given (already sorted) order.

    Since the 2026-09-05 ID freeze: an item with a stored 'id' (frozen by
    `freeze`) or a 'keep_id' (the pre-freeze literal-name mechanism, still
    honoured for an item that has not been frozen yet) keeps that value
    verbatim -- ids are never renumbered. Everything else gets the next
    unused number for `prefix`, continuing after the highest number
    already in use for it (stored or keep_id), so adding or removing
    unfrozen items never shifts an already-frozen id.
    Returns the list of ids used by more than one item (empty if none);
    the caller decides how to fail on that."""
    used: set[str] = set()
    dups: list[str] = []
    max_numbered = 0
    for item in items:
        candidate = item.get("id") or item.get("keep_id")
        if not candidate:
            continue
        if candidate in used:
            dups.append(candidate)
        used.add(candidate)
        if candidate.startswith(prefix + "-"):
            m = _ID_NUM_TRAIL_RE.search(candidate)
            if m:
                max_numbered = max(max_numbered, int(m.group(1)))

    counter = max_numbered + 1
    for item in items:
        candidate = item.get("id") or item.get("keep_id")
        if candidate:
            item["_id"] = candidate
            continue
        new_id = f"{prefix}-{counter:03d}"
        while new_id in used:
            counter += 1
            new_id = f"{prefix}-{counter:03d}"
        item["_id"] = new_id
        used.add(new_id)
        counter += 1
    return dups


# ------------------------------------------------------------- building --

def build_source(src: dict) -> dict:
    return {"doc": src["doc"], "heading": src["heading"], "lines": [src["lines"][0], src["lines"][1]]}


def build_leaf_output(item: dict, layer: str) -> dict:
    out = {"id": item["_id"], "statement": item["statement"]}
    if item.get("description"):
        out["description"] = item["description"]
    if layer != "request":
        # Stored derived_from passes through unchanged (2026-09-05 ID
        # freeze): build never computes it -- that is apply-derivation's
        # job, run deliberately with --write, not on every build.
        out["derived_from"] = list(item.get("derived_from") or [])
        cites = item.get("cites")
        if cites:
            out["cites"] = list(cites)
    out["source"] = build_source(item["source"])
    return out


_HEADING_MARKUP_STRIP_RE = re.compile(r"^#{1,6}\s*")


def strip_heading_markup(heading: str) -> str:
    return _HEADING_MARKUP_STRIP_RE.sub("", heading).strip()


def effective_layer(item: dict, frag_layer: str) -> str:
    """An item's actual (post-relayer) layer: its own "layer" override if
    present and a real layer name, else the layer its fragment declares."""
    override = item.get("layer")
    return override if override in LAYER_ORDER else frag_layer


def build_in_memory(root: Path):
    """Core of `build`, factored out so `freeze` can reuse it: sorts and
    id-stamps every fragment item/area exactly as `build` would, but also
    hands back the *original* fragment item/area dict objects (by
    reference, keyed by their assigned id) so a caller can mutate and
    persist them -- and the (path, frag) pairs `load_fragments` returned,
    so a caller can write fragments back to the same files.

    2026-09-05 layer split: a fragment's own declared "layer" (request/
    require/spec/basic_design/design) picks its on-disk shape (flat items,
    or areas); an item's own optional "layer" field (spec/basic_design/
    design only -- see effective_layer) can redirect it elsewhere at build
    time without touching the fragment's shape. A flat-declared item
    (request/require/spec) redirected into an area-layer gets a synthetic
    one-item area (title = its own heading, text stripped of "#"/
    whitespace); several such items sharing the same (doc, heading)
    collapse into the same synthetic area. An area-declared item
    redirected to a *different* area-layer keeps its original area's
    title/description/source as the grouping key, so if only some of an
    area's items move, the area is represented once per layer it now has
    items in (a stored area id is only reused for the layer that ends up
    with the area's full original item set; the other gets a fresh one --
    freeze does not run concurrently with an unresolved split, so this is
    not a live concern yet).

    -> (output, item_objects: dict[id, dict], area_objects: dict[id, dict],
        dup_ids: list[str], fragments: list[(Path, dict)])
    Raises ValueError on an unknown/missing layer; does not touch disk."""
    fragments = load_fragments(root)

    flat_items: dict[str, list[dict]] = {layer: [] for layer in _FLAT_LAYERS}
    # (doc, heading) -> group record, per area-layer
    area_groups: dict[str, dict] = {layer: {} for layer in _AREA_LAYERS}

    def route(it: dict, target: str) -> None:
        if target in _AREA_LAYERS:
            native_area = it.get("_native_area")
            group_source = native_area["source"] if native_area is not None else it["source"]
            key = (group_source["doc"], group_source["heading"])
            grp = area_groups[target].get(key)
            if grp is None:
                if native_area is not None:
                    grp = {
                        "title": native_area["title"],
                        "description": native_area.get("description"),
                        "source": native_area["source"],
                        "native_area": native_area,
                        "items": [],
                    }
                else:
                    grp = {
                        "title": strip_heading_markup(it["source"]["heading"]),
                        "description": None,
                        "source": it["source"],
                        "native_area": None,
                        "items": [],
                    }
                area_groups[target][key] = grp
            grp["items"].append(it)
            it["_group"] = grp
        else:
            flat_items[target].append(it)

    for path, frag in fragments:
        frag_layer = frag.get("layer")
        if frag_layer not in LAYERS:
            raise ValueError(f"{path}: unknown or missing layer {frag_layer!r}")
        if frag_layer in _AREA_LAYERS:
            for area in frag.get("areas", []):
                for it in area.get("items", []):
                    it["_native_area"] = area
                    route(it, effective_layer(it, frag_layer))
        else:
            for it in frag.get("items", []):
                it["_native_area"] = None
                route(it, effective_layer(it, frag_layer))

    output = {"schema_version": SCHEMA_VERSION}
    all_dups: list[str] = []
    item_objects: dict[str, dict] = {}

    for layer in _FLAT_LAYERS:
        items = flat_items[layer]
        items.sort(key=lambda it, layer=layer: item_sort_key(layer, it))
        all_dups.extend(assign_ids(items, LEAF_PREFIX[layer]))
        output[layer] = [build_leaf_output(it, layer) for it in items]
        for it in items:
            item_objects[it["_id"]] = it

    area_objects: dict[str, dict] = {}
    for layer in _AREA_LAYERS:
        groups = list(area_groups[layer].values())
        groups.sort(key=lambda g: (design_doc_rank(g["source"]["doc"]), g["source"]["lines"][0]))

        # An area-group's own "id" carrier: only when a native area's WHOLE
        # original item set landed in this one layer (not split) does its
        # stored id (if any) belong here unambiguously.
        area_id_carriers = []
        for g in groups:
            na = g["native_area"]
            carrier = {}
            if na is not None and len(na.get("items", [])) == len(g["items"]) and na.get("id"):
                carrier["id"] = na["id"]
            area_id_carriers.append(carrier)
        all_dups.extend(assign_ids(area_id_carriers, AREA_PREFIX[layer]))
        for g, carrier in zip(groups, area_id_carriers):
            g["_area_id"] = carrier["_id"]

        all_items_this_layer = [it for g in groups for it in g["items"]]
        all_items_this_layer.sort(key=lambda it: item_sort_key(layer, it))
        all_dups.extend(assign_ids(all_items_this_layer, LEAF_PREFIX[layer]))

        area_out_by_group_id = {}
        layer_out = []
        for g in groups:
            area_out = {"id": g["_area_id"], "title": g["title"]}
            if g.get("description"):
                area_out["description"] = g["description"]
            area_out["items"] = []
            area_out["source"] = build_source(g["source"])
            layer_out.append(area_out)
            area_out_by_group_id[id(g)] = area_out
            na = g["native_area"]
            area_objects[g["_area_id"]] = na if (na is not None and len(na.get("items", [])) == len(g["items"])) else g

        for it in all_items_this_layer:
            area_out = area_out_by_group_id[id(it["_group"])]
            area_out["items"].append(build_leaf_output(it, layer))
            item_objects[it["_id"]] = it

        output[layer] = layer_out

    return output, item_objects, area_objects, all_dups, fragments


def cmd_build(args) -> int:
    root = Path(args.root)
    try:
        output, _item_objects, _area_objects, dups, _fragments = build_in_memory(root)
    except ValueError as exc:
        print(f"error: {exc}", file=sys.stderr)
        return 1
    if dups:
        for d in sorted(set(dups)):
            print(f"error: duplicate stored id {d!r} used by more than one item/area", file=sys.stderr)
        return 1

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
          f"{len(output['detailed_spec'])} detailed_spec areas, "
          f"{len(output['basic_design'])} basic_design areas, "
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
        if layer in _AREA_LAYERS:
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


def fmt_area_blocks(areas: list) -> list:
    blocks: list = []
    for area in areas:
        block = [f"## {area['id']} {area['title']}"]
        if area.get("description"):
            block.append("")
            block.append(area["description"].strip())
        for it in area["items"]:
            block.append("")
            block.extend(fmt_item(it))
        blocks.append(block)
    return blocks


def cmd_export(args) -> int:
    root = Path(args.root)
    spec = read_json(spec_json_path(root))
    out_dir = export_dir(root)
    out_dir.mkdir(parents=True, exist_ok=True)

    write_export_file(out_dir / "request.md", "要求", [fmt_item(it) for it in spec["request"]])
    write_export_file(out_dir / "require.md", "要件定義", [fmt_item(it) for it in spec["require"]])
    write_export_file(out_dir / "spec.md", "基本仕様", [fmt_item(it) for it in spec["spec"]])
    write_export_file(out_dir / "detailed_spec.md", "詳細仕様", fmt_area_blocks(spec["detailed_spec"]))
    write_export_file(out_dir / "basic_design.md", "基本設計", fmt_area_blocks(spec["basic_design"]))
    write_export_file(out_dir / "design.md", "詳細設計", fmt_area_blocks(spec["design"]))

    print(f"export: wrote {out_dir / 'request.md'}, require.md, spec.md, detailed_spec.md, basic_design.md, design.md")
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
        # 2026-09-05 ID freeze: a stored 'id' is now legitimate (written by
        # `freeze`); build keeps it verbatim rather than renumbering. Still
        # validate its shape against the schema's id pattern.
        iid = item["id"]
        if not isinstance(iid, str) or not ID_PATTERN.match(iid):
            problems.append(f"{label}: id={iid!r} does not match the id pattern")
    if "derived_from" not in item:
        problems.append(f"{label}: derived_from is required in fragments")
    else:
        derived = item["derived_from"]
        if not (isinstance(derived, list) and all(isinstance(d, str) and ID_PATTERN.match(d) for d in derived)):
            problems.append(f"{label}: derived_from must be a list of ids matching the id pattern; got {derived!r}")
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
    if "layer" in item:
        # 2026-09-05 relayer: an item may override its effective layer to
        # spec or any area-layer, independent of its fragment's own
        # declared layer (see effective_layer / build_in_memory).
        override = item["layer"]
        if override not in _RELAYER_TARGET_LAYERS:
            problems.append(f"{label}: layer override {override!r} must be one of {_RELAYER_TARGET_LAYERS}")
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

    if layer in _FLAT_LAYERS:
        if "areas" in frag:
            problems.append(f"layer {layer!r} must not carry 'areas'")
        items = frag.get("items")
        if not isinstance(items, list):
            problems.append("items must be a list")
        else:
            for i, it in enumerate(items):
                check_leaf_item(it, f"items[{i}]", problems)
    elif layer in _AREA_LAYERS:
        if "items" in frag:
            problems.append(f"layer {layer!r} must not carry top-level 'items' (use 'areas[].items')")
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
                    aid = area["id"]
                    if not isinstance(aid, str) or not ID_PATTERN.match(aid):
                        problems.append(f"{alabel}: id={aid!r} does not match the id pattern")
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
    """Iterates a fragment's own declared structure (areas vs flat items) --
    NOT the effective/overridden layer of each item, which only matters at
    build time. basic_design and design fragments are both area-shaped."""
    if frag.get("layer") in _AREA_LAYERS:
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
    """Every statement id (any layer's item) -> its record, in
    specification.json's own deterministic order."""
    idx: dict = {}
    for it in spec["request"]:
        idx[it["id"]] = dict(it, layer="request")
    for it in spec["require"]:
        idx[it["id"]] = dict(it, layer="require")
    for it in spec["spec"]:
        idx[it["id"]] = dict(it, layer="spec")
    for layer in _AREA_LAYERS:
        for area in spec.get(layer, []):
            for it in area["items"]:
                idx[it["id"]] = dict(it, layer=layer, area_id=area["id"], area_title=area["title"])
    return idx


def scope_items(spec: dict, scope: str) -> list:
    """scope: 'require' | 'spec' | '<area_layer>:本冊' | '<area_layer>:別紙A' |
    '<area_layer>:別紙B' | '<area_layer>:別紙C', area_layer in
    {basic_design, design} (both draw areas from the same document set)."""
    if scope == "require":
        return spec["require"]
    if scope == "spec":
        return spec["spec"]
    area_layer, _, mark = scope.partition(":")
    if area_layer in _AREA_LAYERS and mark:
        out = []
        for area in spec.get(area_layer, []):
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
    area_layer, _, mark = (scope or "").partition(":")
    if area_layer in _AREA_LAYERS and mark:
        return area_layer
    return None


_LAYER_RANK = LAYER_ORDER


def layer_relation(source_layer, target_layer):
    """Fixed order request<require<spec<basic_design<design (every design area, whichever
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
    for layer in _AREA_LAYERS:
        for area in spec.get(layer, []):
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


# ---------------------------------------------------------- apply-derivation --

_ID_TRAILING_NUM_RE = re.compile(r"([0-9]+)$")


def id_sort_number(iid: str) -> int:
    m = _ID_TRAILING_NUM_RE.search(iid)
    return int(m.group(1)) if m else 0


def collect_rules_1_2_edges(spec: dict, id_index: dict, root: Path, repo_root: Path) -> dict:
    """target_id -> set of candidate source ids, from cites (rule 1) and
    the 要求-要件 derivation table (rule 2) only, before rule 3
    filtering/dedup/sort."""
    edges: dict = {}

    # Rule 1: reuse derivation-candidates' own cite resolution + layer_relation.
    for e in build_derivation_candidates(spec, id_index):
        if e["layer_relation"] in ("adjacent-upstream", "skip-upstream"):
            edges.setdefault(e["id"], set()).update(c["id"] for c in e["candidates"])

    # Rule 2: reuse the same table-row resolution.
    for r in build_derivation_table_candidates(spec, id_index, root, repo_root):
        if r["source_ids"] and r["target_ids"]:
            for t in r["target_ids"]:
                edges.setdefault(t, set()).update(r["source_ids"])

    return edges


# --- Rule 3 (traceability tables): each document's own trailing 「付記
# （非規範）: トレーサビリティ表」 records, per row, which upstream section(s)
# a given section of THIS document realises. Found via the dropped-log
# reason text (these entries are not marked keep_for_derivation -- that
# flag is reserved for the 要求→要件 table -- so fragments need no edit
# for this to work).

_TRACE_DOC_ABBR_TO_SCOPE = {"本冊": "design:本冊", "基本": "spec", "要件": "require"}
_TRACE_DOC_ABBR_RE = "本冊|基本|要件"
_TRACE_ITEM_SUFFIX = r"(?:\s*item[0-9]+(?:-[0-9]+)?)?"
_TRACE_SECLIST_BODY = (
    r"§\s*[0-9]+(?:\.[0-9]+)*" + _TRACE_ITEM_SUFFIX
    + r"(?:\s*[・、/／]\s*§?\s*[0-9]+(?:\.[0-9]+)*" + _TRACE_ITEM_SUFFIX + r")*"
)
_TRACE_SCAN_RE = re.compile(
    r"(?P<secdoc>" + _TRACE_DOC_ABBR_RE + r")\s*(?P<seclist>" + _TRACE_SECLIST_BODY + r")"
    r"|(?P<bareseclist>" + _TRACE_SECLIST_BODY + r")"
    r"|(?<![A-Za-z0-9])(?P<bare>P-[0-9]{3}|R-[1-5]|F[0-9]+|OOS-[0-9]{3}|NFR-[0-9]{3})(?![A-Za-z0-9])"
)
_TRACE_TOKEN_SEP_RE = re.compile(r"\s*[・、/／]\s*")
_TRACE_ITEM_SUFFIX_TRIM_RE = re.compile(r"\s*item[0-9]+(?:-[0-9]+)?\s*$")
_TRACE_OWN_SECTION_RE = re.compile(r"^§\s*([0-9]+(?:\.[0-9]+)*)")
_TRACE_TABLE_ROW_RE = re.compile(r"^\|(.+)\|\s*$")


def split_trace_seclist(seclist: str) -> list:
    nums = []
    for raw in _TRACE_TOKEN_SEP_RE.split(seclist):
        raw = raw.strip()
        if not raw:
            continue
        raw = _TRACE_ITEM_SUFFIX_TRIM_RE.sub("", raw).strip()
        if not raw.startswith("§"):
            raw = "§" + raw
        nums.append(raw[1:].strip())
    return nums


def resolve_trace_upstream_cell(spec: dict, id_index: dict, cell: str, default_bare_scope):
    """-> (candidate_ids: list[str], had_unresolvable_bare_section: bool).
    default_bare_scope is the scope a §-group with no doc abbreviation at
    all resolves against (基本仕様's own table has none, per its 1 fixed
    upstream doc 要件定義); None means "no default" -- such a group is
    reported as a parse problem rather than guessed."""
    ids = []
    problem = False
    for m in _TRACE_SCAN_RE.finditer(cell):
        if m.group("secdoc"):
            scope = _TRACE_DOC_ABBR_TO_SCOPE[m.group("secdoc")]
            for num in split_trace_seclist(m.group("seclist")):
                ids.extend(section_candidates(spec, scope, num))
        elif m.group("bareseclist"):
            if default_bare_scope is None:
                problem = True
                continue
            for num in split_trace_seclist(m.group("bareseclist")):
                ids.extend(section_candidates(spec, default_bare_scope, num))
        elif m.group("bare"):
            code = m.group("bare")
            if re.match(r"^(R-[1-5]|P-[0-9]{3})$", code):
                if code in id_index:
                    ids.append(code)
            else:
                ids.extend(self_naming_candidates(id_index, code))
    return ids, problem


def find_traceability_table_ranges(root: Path) -> list:
    """[(doc, start, end), ...] from dropped-log entries whose reason
    names the trailing traceability appendix."""
    out = []
    for _path, entry in load_dropped(root):
        if "トレーサビリティ表" in entry.get("reason", ""):
            out.append((entry["doc"], entry["lines"][0], entry["lines"][1]))
    return out


def _trace_table_profile(doc: str):
    """-> (table_name, source_scope, default_bare_scope) for a doc path,
    or None if it doesn't match one of the 4 known traceability tables."""
    if "基本仕様" in doc:
        return "基本仕様", "spec", "require"
    if "別紙A" in doc:
        return "別紙A", "design:別紙A", None
    if "別紙C" in doc:
        return "別紙C", "design:別紙C", None
    if "詳細設計" in doc:
        return "本冊", "design:本冊", "spec"
    return None


def parse_generic_table_rows(md_lines: list, start: int, end: int) -> list:
    """Like parse_md_table_rows, but detects the header row positionally
    (the row immediately before a separator row) instead of matching a
    specific header text, so it works for any table shape."""
    raw_rows = []
    for line_no in range(start, min(end, len(md_lines)) + 1):
        m = _TRACE_TABLE_ROW_RE.match(md_lines[line_no - 1].strip())
        if not m:
            continue
        raw_rows.append((line_no, [c.strip() for c in m.group(1).split("|")]))
    rows = []
    for i, (line_no, cells) in enumerate(raw_rows):
        if all(_TABLE_SEP_CELL_RE.match(c) for c in cells if c):
            continue
        if i + 1 < len(raw_rows):
            next_cells = raw_rows[i + 1][1]
            if all(_TABLE_SEP_CELL_RE.match(c) for c in next_cells if c):
                continue  # this row is a header (the one right before the separator)
        rows.append((line_no, cells))
    return rows


def collect_traceability_table_edges(spec: dict, id_index: dict, root: Path, repo_root: Path):
    """-> (edges: dict[target_id -> set(source_id)], stats: dict with
    rows_parsed, unparseable ([(table_name, line_no, cells), ...]),
    by_table ({table_name: rows_parsed}))."""
    edges: dict = {}
    rows_parsed = 0
    unparseable = []
    by_table = Counter()

    for doc, start, end in find_traceability_table_ranges(root):
        profile = _trace_table_profile(doc)
        if profile is None:
            continue
        table_name, source_scope, default_bare_scope = profile
        md_lines = (repo_root / doc).read_text(encoding="utf-8").splitlines()
        for line_no, cells in parse_generic_table_rows(md_lines, start, end):
            if len(cells) != 3:
                unparseable.append((table_name, line_no, cells))
                continue
            own_cell, upstream_cell, _kind_cell = cells
            m = _TRACE_OWN_SECTION_RE.match(own_cell)
            if not m:
                unparseable.append((table_name, line_no, cells))
                continue
            targets = section_candidates(spec, source_scope, m.group(1))
            upstream_ids, had_problem = resolve_trace_upstream_cell(spec, id_index, upstream_cell, default_bare_scope)
            if had_problem:
                unparseable.append((table_name, line_no, cells))
                continue
            rows_parsed += 1
            by_table[table_name] += 1
            for t in targets:
                edges.setdefault(t, set()).update(upstream_ids)

    return edges, {"rows_parsed": rows_parsed, "unparseable": unparseable, "by_table": by_table}


def collect_derivation_edges(spec: dict, id_index: dict, root: Path, repo_root: Path):
    """-> (edges, trace_stats). edges is target_id -> set of candidate
    source ids, merged from all three rules, before rule 3
    filtering/dedup/sort."""
    edges = collect_rules_1_2_edges(spec, id_index, root, repo_root)
    trace_edges, trace_stats = collect_traceability_table_edges(spec, id_index, root, repo_root)
    for t, srcs in trace_edges.items():
        edges.setdefault(t, set()).update(srcs)
    return edges, trace_stats


def finalize_derived_from(target_id: str, candidate_ids, id_index: dict):
    """Rule 3 -> (sorted deduped list, dropped_count). Dropped counts the
    statement's own id and any candidate whose layer is the same as or
    later than the target's own layer; a plain duplicate is silently
    deduped and not counted as a drop."""
    target_rank = _LAYER_RANK[id_index[target_id]["layer"]]
    kept = []
    seen = set()
    dropped = 0
    for cid in candidate_ids:
        if cid == target_id:
            dropped += 1
            continue
        cand = id_index.get(cid)
        if cand is None or _LAYER_RANK[cand["layer"]] >= target_rank:
            dropped += 1
            continue
        if cid in seen:
            continue
        seen.add(cid)
        kept.append(cid)
    kept.sort(key=lambda i: (_LAYER_RANK[id_index[i]["layer"]], id_sort_number(i)))
    return kept, dropped


def summarize_edges(spec: dict, id_index: dict, edges: dict) -> dict:
    """layer -> count of statements that would have a non-empty
    derived_from under `edges`, after rule 3. Read-only (does not touch
    the statements)."""
    counts = Counter()
    for it in iter_all_statements(spec):
        iid = it["id"]
        layer = id_index[iid]["layer"]
        if layer == "request":
            continue
        final_list, _dropped = finalize_derived_from(iid, edges.get(iid, ()), id_index)
        if final_list:
            counts[layer] += 1
    return counts


def cmd_apply_derivation(args) -> int:
    root = Path(args.root)
    repo_root = Path(args.repo_root)
    spec = read_json(spec_json_path(root))
    id_index = build_id_index(spec)

    edges_before = collect_rules_1_2_edges(spec, id_index, root, repo_root)
    stats_before = summarize_edges(spec, id_index, edges_before)

    trace_edges, trace_stats = collect_traceability_table_edges(spec, id_index, root, repo_root)
    edges_after = {k: set(v) for k, v in edges_before.items()}
    for t, srcs in trace_edges.items():
        edges_after.setdefault(t, set()).update(srcs)
    stats_after = summarize_edges(spec, id_index, edges_after)

    edges_added_by_trace = sum(
        len(srcs - edges_before.get(t, set())) for t, srcs in trace_edges.items()
    )

    print("--- apply-derivation: Task A (traceability tables) ---")
    print(f"  rows parsed: {trace_stats['rows_parsed']}")
    for table_name in ("基本仕様", "本冊", "別紙A", "別紙C"):
        print(f"    {table_name}: {trace_stats['by_table'].get(table_name, 0)}")
    print(f"  rows unparseable: {len(trace_stats['unparseable'])}")
    for table_name, line_no, cells in trace_stats["unparseable"]:
        print(f"    {table_name} L{line_no}: {cells!r}")
    print("  statements with derived_from BEFORE Task A (rules 1+2 only):")
    for layer in ("require", "spec", "detailed_spec", "basic_design", "design"):
        print(f"    {layer}: {stats_before.get(layer, 0)}")
    print("  statements with derived_from AFTER Task A (rules 1+2+3):")
    for layer in ("require", "spec", "detailed_spec", "basic_design", "design"):
        print(f"    {layer}: {stats_after.get(layer, 0)}")
    print(f"  edges added by Task A (raw candidate pairs, before rule 3): {edges_added_by_trace}")

    cited_ids = {it["id"] for it in iter_all_statements(spec) if it.get("cites")}

    recomputed: dict = {}
    dropped_total = 0
    sizes = []
    empty_after_cite = 0
    for it in iter_all_statements(spec):
        iid = it["id"]
        layer = id_index[iid]["layer"]
        if layer == "request":
            continue  # rootItem schema has no derived_from field
        final_list, dropped = finalize_derived_from(iid, edges_after.get(iid, ()), id_index)
        dropped_total += dropped
        recomputed[iid] = final_list
        if final_list:
            sizes.append(len(final_list))
        elif iid in cited_ids:
            empty_after_cite += 1

    stats_by_layer = Counter()
    for iid, lst in recomputed.items():
        if lst:
            stats_by_layer[id_index[iid]["layer"]] += 1

    diffs = []
    for it in iter_all_statements(spec):
        iid = it["id"]
        if iid not in recomputed:
            continue
        stored = it.get("derived_from", [])
        new = recomputed[iid]
        if stored != new:
            diffs.append((iid, sorted(set(new) - set(stored)), sorted(set(stored) - set(new))))

    print(f"--- apply-derivation diff (stored vs recomputed): {len(diffs)} statement(s) differ ---")
    added_total = sum(len(a) for _i, a, _r in diffs)
    removed_total_diff = sum(len(r) for _i, _a, r in diffs)
    print(f"  ids that would be added across those statements: {added_total}")
    print(f"  ids that would be removed across those statements: {removed_total_diff}")

    if not getattr(args, "write", False):
        print("(report-only: pass --write to overwrite specification.json's derived_from with the recomputed set)")
        return 0

    for it in iter_all_statements(spec):
        iid = it["id"]
        if iid in recomputed:
            it["derived_from"] = recomputed[iid]

    bad_refs = [
        (it["id"], did)
        for it in iter_all_statements(spec)
        for did in it.get("derived_from", [])
        if did not in id_index
    ]
    if bad_refs:
        for sid, did in bad_refs:
            print(f"error: {sid} derived_from references nonexistent id {did!r}", file=sys.stderr)
        return 1

    schema = read_json(schema_path(root))
    validator = Draft202012Validator(schema)
    errors = sorted(validator.iter_errors(spec), key=lambda e: list(e.absolute_path))
    if errors:
        for err in errors:
            pointer = "/" + "/".join(str(p) for p in err.absolute_path)
            print(f"SCHEMA ERROR {pointer}: {err.message}", file=sys.stderr)
        return 1

    text = json.dumps(spec, ensure_ascii=False, indent=2) + "\n"
    spec_json_path(root).write_text(text, encoding="utf-8")

    total_edges = sum(sizes)
    mean_size = (total_edges / len(sizes)) if sizes else 0.0
    max_size = max(sizes) if sizes else 0

    print("--- apply-derivation summary ---")
    for layer in ("require", "spec", "detailed_spec", "basic_design", "design"):
        print(f"  statements with non-empty derived_from ({layer}): {stats_by_layer.get(layer, 0)}")
    print(f"  total edges: {total_edges}")
    print(f"  mean derived_from size (non-empty statements only): {mean_size:.2f}")
    print(f"  max derived_from size: {max_size}")
    print(f"  statements with cites but empty derived_from (unresolved): {empty_after_cite}")
    print(f"  dropped by rule 3 (self-id / same-or-later layer): {dropped_total}")
    print(f"wrote {spec_json_path(root)} -- schema OK")
    return 0


# ------------------------------------------------------------------ freeze --

def cmd_freeze(args) -> int:
    root = Path(args.root)
    repo_root = Path(args.repo_root)

    try:
        output, item_objects, area_objects, dups, fragments = build_in_memory(root)
    except ValueError as exc:
        print(f"error: {exc}", file=sys.stderr)
        return 1
    if dups:
        for d in sorted(set(dups)):
            print(f"error: duplicate stored id {d!r} used by more than one item/area", file=sys.stderr)
        return 1

    id_index = build_id_index(output)
    edges, _trace_stats = collect_derivation_edges(output, id_index, root, repo_root)

    for iid, item_obj in item_objects.items():
        item_obj["id"] = iid
        item_obj.pop("_id", None)
        item_obj.pop("_native_area", None)
        item_obj.pop("_group", None)
        item_obj.pop("keep_id", None)  # promoted into "id"; the old mechanism is now redundant
        # NOTE: an item's "layer" override (if any) is NOT removed here --
        # it is a permanent routing directive build_in_memory reads on
        # every run, independent of the assigned id; stripping it would
        # silently un-relayer the item on the next build.
        layer = id_index[iid]["layer"]
        if layer != "request":
            final_list, _dropped = finalize_derived_from(iid, edges.get(iid, ()), id_index)
            item_obj["derived_from"] = final_list

    for aid, area_obj in area_objects.items():
        area_obj["id"] = aid
        area_obj.pop("_id", None)
        area_obj.pop("_area_id", None)

    stamped_by_file: dict = {}
    for path, frag in fragments:
        n = 0
        if frag.get("layer") in _AREA_LAYERS:
            for area in frag.get("areas", []):
                n += 1
                n += len(area.get("items", []))
        else:
            n += len(frag.get("items", []))
        stamped_by_file[str(path)] = n

    for path, frag in fragments:
        text = json.dumps(frag, ensure_ascii=False, indent=2) + "\n"
        path.write_text(text, encoding="utf-8")

    print("--- freeze summary ---")
    for path_str in sorted(stamped_by_file):
        print(f"  {path_str}: {stamped_by_file[path_str]} stamped")
    print(f"  total items+areas stamped: {sum(stamped_by_file.values())}")
    print(f"  fragment files rewritten: {len(fragments)}")
    return 0


# ----------------------------------------------------------------- relayer --

_RELAYER_KEY_DOC_LINES_RE = re.compile(r"^(.*):([0-9]+)-([0-9]+)$")


def strip_build_bookkeeping(item_objects: dict, area_objects: dict) -> None:
    """Undo build_in_memory's transient mutations (_id/_native_area/_group
    on items, _id/_area_id on areas) before writing a fragment back to
    disk from outside `freeze` (i.e. from `relayer apply`)."""
    for it in item_objects.values():
        it.pop("_id", None)
        it.pop("_native_area", None)
        it.pop("_group", None)
    for area in area_objects.values():
        area.pop("_id", None)
        area.pop("_area_id", None)


def cmd_relayer_apply(args) -> int:
    root = Path(args.root)
    mapping_path = Path(args.mapping)
    mapping = read_json(mapping_path)
    if not isinstance(mapping, dict):
        print(f"error: {mapping_path} must be a JSON object", file=sys.stderr)
        return 1

    try:
        _output, item_objects, area_objects, dups, fragments = build_in_memory(root)
    except ValueError as exc:
        print(f"error: {exc}", file=sys.stderr)
        return 1
    if dups:
        for d in sorted(set(dups)):
            print(f"error: duplicate stored id {d!r} used by more than one item/area", file=sys.stderr)
        return 1

    by_doc_lines = {}
    for it in item_objects.values():
        s, e = it["source"]["lines"]
        by_doc_lines[(it["source"]["doc"], s, e)] = it

    item_to_path = {}
    for path, frag in fragments:
        for it in iter_leaf_items(frag):
            item_to_path[id(it)] = path

    applied = []      # (key, old_layer_or_none, new_layer)
    low_confidence = []
    code_like = []
    unresolved = []
    touched_paths = set()

    for key, entry in mapping.items():
        if not isinstance(entry, dict) or "layer" not in entry:
            unresolved.append((key, "mapping entry missing 'layer'"))
            continue
        new_layer = entry["layer"]
        if new_layer not in _RELAYER_TARGET_LAYERS:
            unresolved.append((key, f"invalid layer {new_layer!r} (must be one of {_RELAYER_TARGET_LAYERS})"))
            continue

        item_obj = item_objects.get(key)
        if item_obj is None:
            m = _RELAYER_KEY_DOC_LINES_RE.match(key)
            if m:
                doc, s, e = m.group(1), int(m.group(2)), int(m.group(3))
                item_obj = by_doc_lines.get((doc, s, e))
        if item_obj is None:
            unresolved.append((key, "id/'<doc>:<start>-<end>' not found in the current build"))
            continue

        old_layer = item_obj.get("layer")
        item_obj["layer"] = new_layer
        applied.append((key, old_layer, new_layer))
        touched_paths.add(item_to_path[id(item_obj)])
        if entry.get("confidence") == "low":
            low_confidence.append((key, entry.get("reason", "")))
        if entry.get("code_like"):
            code_like.append((key, entry.get("reason", "")))

    strip_build_bookkeeping(item_objects, area_objects)

    for path, frag in fragments:
        if path in touched_paths:
            text = json.dumps(frag, ensure_ascii=False, indent=2) + "\n"
            path.write_text(text, encoding="utf-8")

    print("--- relayer apply ---")
    print(f"  entries in mapping: {len(mapping)}")
    print(f"  applied: {len(applied)}")
    print(f"  unresolved: {len(unresolved)}")
    for key, why in unresolved:
        print(f"    {key}: {why}")
    print(f"  fragment files rewritten: {len(touched_paths)}")
    if low_confidence:
        print(f"  confidence=low ({len(low_confidence)}):")
        for key, reason in low_confidence:
            print(f"    {key}: {reason}")
    if code_like:
        print(f"  code_like=true ({len(code_like)}):")
        for key, reason in code_like:
            print(f"    {key}: {reason}")
    return 0


def cmd_relayer_report(args) -> int:
    root = Path(args.root)
    counts: dict = {}
    for _path, frag in load_fragments(root):
        frag_layer = frag.get("layer")
        for it in iter_leaf_items(frag):
            key = (it["source"]["doc"], effective_layer(it, frag_layer))
            counts[key] = counts.get(key, 0) + 1

    print("--- relayer report: counts per (source doc -> target layer) ---")
    for (doc, layer), n in sorted(counts.items()):
        print(f"  {doc} -> {layer}: {n}")
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

    p_applyderiv = sub.add_parser("apply-derivation", help="CONVERSION.md SS6: report (or, with --write, apply) the recomputed derived_from vs the stored one")
    add_root_arg(p_applyderiv)
    add_repo_root_arg(p_applyderiv)
    p_applyderiv.add_argument("--write", action="store_true", help="overwrite specification.json's derived_from with the recomputed set (default: report only)")
    p_applyderiv.set_defaults(func=cmd_apply_derivation)

    p_freeze = sub.add_parser("freeze", help="write id and computed derived_from into fragments in place (2026-09-05 ID freeze)")
    add_root_arg(p_freeze)
    add_repo_root_arg(p_freeze)
    p_freeze.set_defaults(func=cmd_freeze)

    p_relayer = sub.add_parser("relayer", help="apply or report per-item layer overrides (spec/basic_design/design split)")
    relayer_sub = p_relayer.add_subparsers(dest="relayer_command", required=True)

    p_relayer_apply = relayer_sub.add_parser("apply", help="write a layer override into fragment items from a mapping file")
    add_root_arg(p_relayer_apply)
    p_relayer_apply.add_argument("mapping", help="path to the mapping JSON: {'<id or doc:start-end>': {'layer': ..., 'reason': ..., 'confidence': 'high'|'low', 'code_like': bool}}")
    p_relayer_apply.set_defaults(func=cmd_relayer_apply)

    p_relayer_report = relayer_sub.add_parser("report", help="print counts per (source doc -> effective layer)")
    add_root_arg(p_relayer_report)
    p_relayer_report.set_defaults(func=cmd_relayer_report)

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
