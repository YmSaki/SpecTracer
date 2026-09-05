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
        over the fixed order request<require<spec<detailed_spec<basic_design<design (every design
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
    the fixed order request<require<spec<detailed_spec<basic_design<design (derived_from only ever
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
detailed_spec < basic_design < design (detailed_spec 詳細仕様 DS-,
basic_design 基本設計 BD-). A fragment still declares one document-level
"layer", which fixes its on-disk SHAPE only (flat "items" for request/
require/spec; "areas" for detailed_spec/basic_design/design -- see
_FLAT_LAYERS/_AREA_LAYERS). Independently, any item may carry its own
optional "layer" field (spec|detailed_spec|basic_design|design,
_RELAYER_TARGET_LAYERS) that overrides where it lands at build time --
see effective_layer(). This is the relayer mechanism: it never touches a
fragment's shape, only routes individual items.

Section-node model (2026-09-05, Owner): every layer except request is a
recursive section tree
------------------------------------------------------------------------
Superseding the area-per-layer shape above: request/require/spec/
detailed_spec/basic_design/design's "area" concept is replaced by a
文書 > 節 > 小節 > 文 tree, built at build time from the numeric prefix of
each item's source.heading (build_layer_section_tree) -- not from a
fragment's own "areas" grouping, which survives only as authoring shape
and as a source for a matching section's "description" (a native area's
own "title" is never used; a section's title always comes from a real
heading -- its own item's, or, absent one, the true md heading text via
--repo-root, or a bare-number placeholder as a last resort). A section
with numbered children but no item of its own (e.g. "## 2. 基本原則" whose
first content is "### P-001 ...") still exists as a node -- an item with
no numbered heading of its own (that "### P-001 ..." itself) falls back
to the nearest preceding numbered heading in the real md text, never to
"whichever item happens to have a token", so an implied parent isn't
silently skipped over. Section ids are LEAF_PREFIX-S### (REQ-S001,
SPEC-S001, DS-S001, BD-S001, DES-S001), replacing the old per-layer area
prefixes (DA-/BDA-/DSA-) entirely, freshly assigned every build (no
stored-id passthrough for sections yet -- freeze does not persist them).
Statement ids are assigned exactly as before this model (item_sort_key,
one flat per-layer counter) -- the tree only changes how items nest in
the output, never their id.
Edges keep exactly one shape (derived_from, LAYERING.md SS1.1) but now
attach to two kinds of node: an inline `cites` in a statement still
resolves to upstream STATEMENT ids and attaches to that statement (rule
1, unchanged -- section_candidates' fan-out is exactly what it always
was); a row of the 要求→要件 derivation table or of a document's own
trailing traceability appendix, when its column names a section (§N),
resolves to the upstream SECTION node id and attaches to the citing
SECTION node (section_id_candidates -- exact match, never fan-out) --
this is what keeps 136 traceability-table rows from exploding into a
statement-level cross product. A descendant statement still reaches such
an edge through "effective upstream reach" (its own derived_from union
every ancestor section's), computed on demand by cmd_apply_derivation,
never stored.

relayer apply <mapping.json> [<mapping2.json> ...]
    Reads one or more JSON objects {"<id or '<doc>:<start>-<end>'>":
    {"layer": "spec"|"detailed_spec"|"basic_design"|"design", "reason",
    "confidence": "high"|"low", "code_like": bool, "statement_prefix":
    str (optional)}}. Given more than one file, ALL of them are loaded and
    merged FIRST, and every key resolves against ONE baseline build (the
    fragments as they are before this invocation writes anything) -- never
    apply mapping files one invocation at a time: the first write shifts
    statement ids (a moved item leaves its old layer's id sequence), so a
    second invocation's file would resolve its ids against an
    already-renumbered build (found the hard way, 2026-09-05: separate
    per-file invocations left later files 100% unresolved or silently
    misapplied against stale ids). The same key present in more than one
    file with a DIFFERENT entry is a hard error before anything is applied
    (both occurrences are printed); an identical duplicate entry is fine.
    A "statement_prefix" entry (optional), if present, is checked against
    the resolved item's own statement (first 30 chars of each) before
    applying -- a mismatch is skipped and reported separately from a
    plain unresolved key, since it means the mapping was built against
    different content than what's in the fragment now. Resolves each key
    against the baseline build (an id from the most recent build, or a
    literal doc+line-range for content that has no id yet), and writes
    only the "layer" field into that fragment item, in place --
    reason/confidence/code_like/statement_prefix are never persisted to
    the fragment, only echoed in the printed report (with confidence=low
    and code_like=true entries called out separately, so they can be
    routed to the Owner/queued for later cleanup). Unresolved keys are
    reported, not treated as fatal. Only fragment files that actually
    changed are rewritten.

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
Items are sorted by (layer order request<require<spec<detailed_spec<basic_design<design, doc
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
import subprocess
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
# layer split -- detailed_spec (詳細仕様, DS-) then basic_design (基本設計,
# BD-). Two INDEPENDENT axes exist from here on, and must stay independent:
#   - fragment SHAPE (_FLAT_LAYERS / _AREA_LAYERS): what a fragment's own
#     declared "layer" puts on disk -- flat "items" for request/require/
#     spec, "areas" for detailed_spec/basic_design/design. This is authoring
#     shape and does not change with relayering: a base.json item relayered
#     to basic_design still lives inside a flat fragment.
#   - output SHAPE (_SECTIONED_LAYERS): every layer except request is,
#     in specification.json, a recursive section tree (文書 > 節 > 小節 >
#     文), built at build time from source.heading's numbering -- see
#     build_in_memory. A fragment's declared "areas" grouping is consulted
#     only for a matching section's "description" (2026-09-05 section-node
#     model); it no longer determines node identity or membership.
# Everything downstream is written to generalize over however many
# area-layers / sectioned-layers there are, so adding a further one is
# just adding it here.
# 2026-09-05 (root layer): 要件定義's own derivation table declares itself
# 「第I部（根 → 要求）」and names Issue #11's F1-F12 freeze items and Owner
# rulings as its upstream column -- so those are nodes, and request (R-1..
# R-5) has derived_from into them; request itself is not the root, #11 is
# (see LAYERING.md SS1.1 "根の層"). root is flat like request always was
# (no numbered-heading tree -- it's a short, hand-curated ledger, not a
# document section structure) and is the only layer with no derived_from
# (rootItem schema); request moved from rootItem to derivedItem to gain it.
LAYER_ORDER = {"root": 0, "request": 1, "require": 2, "spec": 3, "detailed_spec": 4, "basic_design": 5, "design": 6}
LAYERS = tuple(LAYER_ORDER)
_FLAT_LAYERS = ("root", "request", "require", "spec")     # fragment shape
_AREA_LAYERS = ("detailed_spec", "basic_design", "design")  # fragment shape
_SECTIONED_LAYERS = ("require", "spec", "detailed_spec", "basic_design", "design")  # output shape
# Valid targets for an item's "layer" override (relayer): spec, plus every
# area-layer. root/request/require are never override targets.
_RELAYER_TARGET_LAYERS = ("spec",) + _AREA_LAYERS
LEAF_PREFIX = {"root": "ROOT", "request": "R", "require": "REQ", "spec": "SPEC", "detailed_spec": "DS", "basic_design": "BD", "design": "DES"}
# Section-node id infix (2026-09-05): <LEAF_PREFIX>-S### for every
# sectioned layer, replacing the old per-layer area prefixes (DA-/BDA-/
# DSA-) entirely -- a section is no longer a distinct id family, just the
# leaf prefix with an "S" marker before the number.
ID_PATTERN = re.compile(
    r"^(ROOT-[0-9]{3,}|R-[0-9]+|P-[0-9]{3}|REQ-[0-9]{3,}|REQ-S[0-9]{3,}|SPEC-[0-9]{3,}|SPEC-S[0-9]{3,}"
    r"|DS-[0-9]{3,}|DS-S[0-9]{3,}|BD-[0-9]{3,}|BD-S[0-9]{3,}|DES-[0-9]{3,}|DES-S[0-9]{3,})$"
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
    # design_doc_rank order applies to every sectioned layer (2026-09-05):
    # once section trees exist, a layer's docs are ordered the same way
    # regardless of whether it's request/require/spec (currently
    # single-doc) or an originally-area layer split across multiple docs
    # via relayer.
    if layer in _SECTIONED_LAYERS:
        return (design_doc_rank(doc), doc)
    return (0, doc)


def item_sort_key(layer: str, item: dict):
    src = item["source"]
    return (LAYER_ORDER[layer], doc_key(layer, src["doc"]), src["lines"][0])


_ID_NUM_TRAIL_RE = re.compile(r"([0-9]+)$")


def id_belongs_to_layer(candidate: str, layer: str) -> bool:
    """Whether a stored id's own prefix matches `layer`'s id family --
    used (2026-09-05 fix-ids) to detect a STALE id: one that was assigned
    while the item lived in a different layer, before a later `layer`
    override (relayer) moved it here. P-00N is the one standing exception
    (require's alternate id family, CONVERSION.md SS1) -- never treated
    as stale within require even though it doesn't start with "REQ-"."""
    if layer == "require" and re.fullmatch(r"P-[0-9]{3}", candidate):
        return True
    return candidate.startswith(LEAF_PREFIX[layer] + "-")


def _prefix_matches(candidate: str, prefix: str, layer: str) -> bool:
    if layer == "require" and re.fullmatch(r"P-[0-9]{3}", candidate):
        return True
    return candidate.startswith(prefix + "-")


def assign_ids(items: list[dict], layer: str, reserved_ids: set) -> tuple:
    """Stamp item['_id'] in place, in the given (already sorted) order.

    An item with a stored 'id' (written by `fix-ids`) or a 'keep_id' (the
    pre-fix-ids literal-name mechanism, still honoured for an item that
    hasn't been fixed yet) keeps that value verbatim -- ids are never
    renumbered -- UNLESS that stored id's own prefix no longer matches
    `layer` (id_belongs_to_layer): a relayer "layer" override moved the
    item to a different layer after its id was fixed, so the old id is
    STALE for its new home. A stale id is discarded here (never reused)
    and the item gets a fresh one for `layer`, exactly like an item that
    never had a stored id at all -- 2026-09-05 fix-ids, replacing the old
    freeze's assumption that a stored id is always still valid.

    `reserved_ids` (2026-09-05, fixing a real bug found on the real run):
    every id already known to exist ANYWHERE in the corpus for this run
    (build_reserved_ids -- every fragment's stored ids, every headings[]
    section id, both columns of retired-ids.json), MUTATED IN PLACE as
    this call assigns/retires ids. Both the "highest number in use" scan
    and the fresh-id search must consult it, not just this call's own
    `items` -- an id that just left this layer (its item's "layer"
    override moved it elsewhere in this same run) is invisible to
    `items` (nothing in it claims that id any more), so scanning only
    `items` let a brand-new arrival in the same layer get handed the
    exact number that just left, in the same build. Reserving every id
    ever seen, regardless of who currently claims it, closes that gap
    regardless of which layer's assign_ids call happens to run first.

    Everything without a live stored id gets the next unused number for
    `layer`'s prefix, continuing after the highest number already in use
    for it (stored, keep_id, or merely reserved), so adding or removing
    un-fixed items never shifts an already-fixed id.
    -> (dups: list[str] used by more than one item, retired: list[(old_id,
    new_id)] for every item whose stored id was discarded as stale) --
    the caller decides how to fail on dups and how/whether to log
    retired."""
    prefix = LEAF_PREFIX[layer]
    used: set[str] = set()
    dups: list[str] = []
    max_numbered = 0
    for item in items:
        candidate = item.get("id") or item.get("keep_id")
        if not candidate:
            continue
        if not id_belongs_to_layer(candidate, layer):
            continue
        if candidate in used:
            dups.append(candidate)
        used.add(candidate)

    for rid in used | reserved_ids:
        if _prefix_matches(rid, prefix, layer):
            m = _ID_NUM_TRAIL_RE.search(rid)
            if m:
                max_numbered = max(max_numbered, int(m.group(1)))

    counter = max_numbered + 1
    retired: list[tuple] = []
    for item in items:
        candidate = item.get("id") or item.get("keep_id")
        if candidate and id_belongs_to_layer(candidate, layer):
            item["_id"] = candidate
            continue
        new_id = f"{prefix}-{counter:03d}"
        while new_id in used or new_id in reserved_ids:
            counter += 1
            new_id = f"{prefix}-{counter:03d}"
        item["_id"] = new_id
        used.add(new_id)
        reserved_ids.add(new_id)
        counter += 1
        if candidate:  # it had a stored id, just a stale one -- log the pair
            retired.append((candidate, new_id))
            reserved_ids.add(candidate)
    return dups, retired


# ------------------------------------------------------------- building --

def iter_all_fragment_items(frag: dict):
    """Like iter_leaf_items, but does NOT skip root fragments -- used only
    by build_reserved_ids, which needs every id ever stored anywhere,
    root included (root ids never collide with another layer's prefix,
    but there's no reason to special-case excluding them from the
    reservation registry)."""
    if frag.get("layer") in _AREA_LAYERS:
        for area in frag.get("areas", []):
            for it in area.get("items", []):
                yield it
    else:
        for it in frag.get("items", []):
            yield it


def build_reserved_ids(fragments: list, root: Path) -> set:
    """Every id currently known to exist anywhere in the corpus, whether
    or not it's live in THIS build's routing -- 2026-09-05, fixing a real
    bug found on the real run: an id that just left a layer (a relayer
    "layer" override moved its item elsewhere) is invisible to that
    layer's OWN fresh-id counter (nothing in the layer's current item
    list claims it any more), so the counter could hand the very same
    number to a brand-new arrival in the same run -- observed for real:
    DS-1585 retired to DES-564, then reissued to a different item moving
    into detailed_spec in the same build. Reserving every id ever stored
    anywhere (every fragment item's id/keep_id, every headings[] entry's
    section id map values, and both columns of retired-ids.json) closes
    the gap regardless of which layer's assign_ids call happens to run
    first -- the set is then mutated in place by assign_ids as the run
    proceeds, so a retirement in one layer is visible to every later
    assign_ids call in the same run, including ones for other layers."""
    reserved: set = set()
    for _path, frag in fragments:
        for it in iter_all_fragment_items(frag):
            for key in ("id", "keep_id"):
                v = it.get(key)
                if v:
                    reserved.add(v)
        for h in frag.get("headings", []):
            for v in (h.get("ids") or {}).values():
                reserved.add(v)
    log_path = root / "relations" / "retired-ids.json"
    if log_path.exists():
        for entry in read_json(log_path):
            reserved.add(entry["old_id"])
            reserved.add(entry["new_id"])
    return reserved


def build_source(src: dict) -> dict:
    return {"doc": src["doc"], "heading": src["heading"], "lines": [src["lines"][0], src["lines"][1]]}


def build_leaf_output(item: dict, layer: str) -> dict:
    out = {"id": item["_id"], "statement": item["statement"]}
    if item.get("description"):
        out["description"] = item["description"]
    if layer != "root":
        # Stored derived_from passes through unchanged (2026-09-05 ID
        # freeze): build never computes it -- that is apply-derivation's
        # job, run deliberately with --write, not on every build. root is
        # the only layer without derived_from (2026-09-05 root layer) --
        # request gained it when it moved from rootItem to derivedItem.
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


# ------------------------------------------------------ section-tree build --
# 2026-09-05 (section-node model): every layer except request is a tree,
# 文書 > 節 > 小節 > 文, built purely from the numeric prefix of
# source.heading (heading_number_token, already used by cites/table-row
# resolution) -- not from a fragment's own "areas" grouping, which is
# authoring-time only from here on (see the module-level comment above
# _SECTIONED_LAYERS). A section node's number token is a tuple of ints so
# ("4", "4.2", "4.10") sort numerically, not lexically.

def token_parts(token: str) -> tuple:
    return tuple(int(p) for p in token.split("."))


def parent_token(token: str):
    """'4.2.1' -> '4.2'; '4' -> None (top-level, no parent)."""
    if "." not in token:
        return None
    return token.rsplit(".", 1)[0]


def ancestor_tokens(token: str) -> list:
    """'4.2.1' -> ['4', '4.2'] (nearest-last, excludes token itself)."""
    parts = token.split(".")
    return [".".join(parts[:i]) for i in range(1, len(parts))]


def effective_number_resolver(items_in_doc: list, doc: str, headings_by_doc: dict):
    """-> function(item) -> number token, for one (layer, doc) group.

    An item's own heading usually carries a numeric token (heading_
    number_token); a handful do not (a named sub-heading like "### P-001
    ..." under "## 2. 基本原則", or preamble text before a document's
    first numbered heading) -- CONVERSION.md and the section-node task say
    nothing about these, so the rule here is the smallest one that stays
    inside "derive from source.heading": such an item belongs to the
    nearest PRECEDING numbered heading in the same doc (by line position).
    That "nearest preceding heading" set comes from the fragments' own
    harvested headings[] (headings_by_doc, 2026-09-05 md-independence --
    formerly a live md scan under --repo-root, now `harvest-headings`'
    output, read the same way every other build-time datum is), filtered
    to the digit-dot subset (is_numeric_token) -- a P-00N/R-N entry must
    NOT enter this "known" set, or every item after it would resolve to
    "P-001" instead of "2" and the whole cluster would wrongly become its
    own top-level section (this is exactly the shape of the bug this
    function was written to fix in the first place: "## 2. 基本原則" in
    要求・要件定義 has no item of its own -- its first content is "### P-001
    ..." -- so if "known" were built only from items'/areas' own tokens,
    every one of section 2's fallback items would silently walk past it
    and attach to section 1 instead). When headings_by_doc has nothing for
    this doc (harvest-headings hasn't run for it), this falls back to
    items'/areas' own tokens only -- coarser, but never worse than before
    md-based lookup existed. An item with nothing preceding it at all
    (true document-start preamble) gets the sentinel token "0" -- already
    a real convention in this corpus (詳細設計 本冊 "0. 本書の位置付け")."""
    known = [
        (h["line"], h["number"])
        for h in headings_by_doc.get(doc, [])
        if is_numeric_token(h["number"])
    ]
    if not known:
        seen_area_ids = set()
        for it in items_in_doc:
            tok = heading_number_token(it["source"]["heading"])
            if tok is not None:
                known.append((it["source"]["lines"][0], tok))
            na = it.get("_native_area")
            if na is not None and id(na) not in seen_area_ids:
                seen_area_ids.add(id(na))
                atok = heading_number_token(na["source"]["heading"])
                if atok is not None:
                    known.append((na["source"]["lines"][0], atok))
    known.sort(key=lambda p: p[0])

    def resolve(it: dict) -> str:
        tok = heading_number_token(it["source"]["heading"])
        if tok is not None:
            return tok
        line = it["source"]["lines"][0]
        best = None
        for kline, ktok in known:
            if kline <= line:
                best = ktok
            else:
                break
        return best if best is not None else "0"

    return resolve


def find_heading_in_headings(headings_by_doc: dict, doc: str, token: str):
    """-> (reconstructed_heading_text, line_no) for the harvested heading
    in `doc` whose own number exactly equals `token`, or (None, None) if
    there's no harvested data for `doc` or no such heading. Reconstructs
    an ATX-style string ("#"*level + number + title) from the harvested
    fields so downstream code (heading_number_token, strip_heading_markup)
    keeps working on it exactly as it would on a raw md line -- 2026-09-05
    md-independence replacement for find_heading_in_md; the reconstruction
    drops whatever separator (". " vs " ") the original line used between
    number and title, since harvest-headings' {number, title, line, level}
    shape has nowhere to keep it -- a disclosed, cosmetic-only difference
    for an implied-parent node's title text. Used only for a node that has
    no item and no native area at its own number (an "implied parent" --
    a heading that groups sub-numbered content but was never itself
    transcribed as/near a statement)."""
    for h in headings_by_doc.get(doc, []):
        if h["number"] == token:
            return f"{'#' * h['level']} {h['number']} {h['title']}", h["line"]
    return None, None


def section_id_belongs_to_layer(candidate: str, layer: str) -> bool:
    """Section-id analogue of id_belongs_to_layer (2026-09-05 fix-ids) --
    a stored section id is only still valid for `layer` if its prefix is
    that layer's own LEAF_PREFIX + "-S..."."""
    return candidate.startswith(LEAF_PREFIX[layer] + "-S")


def build_layer_section_tree(layer: str, items_for_layer: list, headings_by_doc: dict, reserved_ids: set) -> tuple:
    """-> (layer_out, item_objects, dups, retired). Builds the 文書 > 節 >
    小節 > 文 tree for one sectioned layer (2026-09-05 section-node
    model): items are grouped by doc (doc order = doc_key, uniform across
    every sectioned layer since any of them can now draw from more than
    one document via relayer); within a doc, a section node exists for
    every number token any item resolves to (effective_number_resolver)
    plus every ancestor of that token (ancestor_tokens) -- so a heading
    with only numbered children and no item of its own still gets a node
    (docstring requirement: "sections with no statements of their own but
    with children still exist"). Item ids are assigned exactly as before
    this model (item_sort_key, one flat counter per layer, global across
    all docs/sections of the layer) -- the tree only changes how items
    nest in the output, never their id.
    2026-09-05 fix-ids: a section's id is looked up from the matching
    headings[] entry's "ids" map (heading number -> {layer: section id},
    written by `fix-ids`) FIRST; only a heading/layer combination with no
    stored id there gets a freshly assigned one, in one global counter per
    layer across every doc, pre-order (doc order, then numeric token
    order within each doc) -- exactly mirroring assign_ids' item-id
    contract, including retiring a stale stored id (wrong layer prefix,
    e.g. a heading that no longer produces a node in a layer it once did)
    rather than reusing it. `retired` is [(old_id, new_id), ...] for
    both items (from assign_ids) and sections (from this function's own
    equivalent logic), for the caller to log."""
    all_items = list(items_for_layer)
    all_items.sort(key=lambda it: item_sort_key(layer, it))
    dups, retired = assign_ids(all_items, layer, reserved_ids)
    item_objects = {it["_id"]: it for it in all_items}

    by_doc: dict[str, list] = {}
    for it in all_items:
        by_doc.setdefault(it["source"]["doc"], []).append(it)
    docs_sorted = sorted(by_doc, key=lambda d: doc_key(layer, d))

    # Pass 1: build every doc's node tree (title/heading/lines/children/
    # items), WITHOUT assigning any section id yet -- id assignment needs
    # to see every doc's preorder token list first (it's one counter
    # shared across the whole layer, not reset per doc).
    nodes_by_doc: dict = {}
    top_tokens_by_doc: dict = {}

    for doc in docs_sorted:
        items_in_doc = by_doc[doc]
        resolve = effective_number_resolver(items_in_doc, doc, headings_by_doc)

        all_tokens: set = set()
        item_tok: dict = {}
        for it in items_in_doc:
            tok = resolve(it)
            item_tok[id(it)] = tok
            all_tokens.add(tok)
            all_tokens.update(ancestor_tokens(tok))
        tokens_sorted = sorted(all_tokens, key=token_parts)

        area_by_token: dict = {}
        seen_area_ids = set()
        for it in items_in_doc:
            na = it.get("_native_area")
            if na is not None and id(na) not in seen_area_ids:
                seen_area_ids.add(id(na))
                atok = heading_number_token(na["source"]["heading"])
                if atok is not None and atok not in area_by_token:
                    area_by_token[atok] = na

        nodes: dict = {}
        for tok in tokens_sorted:
            nodes[tok] = {"children": [], "items": []}
        for tok in tokens_sorted:
            p = parent_token(tok)
            if p is not None:
                nodes[p]["children"].append(tok)
        for it in items_in_doc:
            nodes[item_tok[id(it)]]["items"].append(it)

        for tok in tokens_sorted:
            node = nodes[tok]
            own_items = sorted(
                (it for it in node["items"] if heading_number_token(it["source"]["heading"]) == tok),
                key=lambda it: it["source"]["lines"][0],
            )
            area = area_by_token.get(tok)
            if own_items:
                node["heading"] = own_items[0]["source"]["heading"]
                node["lines"] = [
                    min(it["source"]["lines"][0] for it in own_items),
                    max(it["source"]["lines"][1] for it in own_items),
                ]
                node["title_source"] = "item"
            elif area is not None:
                node["heading"] = area["source"]["heading"]
                node["lines"] = list(area["source"]["lines"])
                node["title_source"] = "area"
            else:
                heading_text, line_no = find_heading_in_headings(headings_by_doc, doc, tok)
                if heading_text is not None:
                    node["heading"] = heading_text
                    node["lines"] = [line_no, line_no]
                    node["title_source"] = "md"
                else:
                    node["heading"] = "#" * min(tok.count(".") + 1, 6) + " " + tok
                    node["lines"] = None  # filled below from subtree bounds
                    node["title_source"] = "placeholder"
            node["title"] = strip_heading_markup(node["heading"])
            if area is not None and area.get("description"):
                node["description"] = area["description"]

        memo: dict = {}

        def subtree_bounds(tok):
            if tok in memo:
                return memo[tok]
            node = nodes[tok]
            los = [it["source"]["lines"][0] for it in node["items"]]
            his = [it["source"]["lines"][1] for it in node["items"]]
            for ctok in node["children"]:
                cs, ce = subtree_bounds(ctok)
                los.append(cs)
                his.append(ce)
            result = (min(los), max(his))
            memo[tok] = result
            return result

        for tok in tokens_sorted:
            if nodes[tok]["lines"] is None:
                nodes[tok]["lines"] = list(subtree_bounds(tok))

        nodes_by_doc[doc] = nodes
        top_tokens_by_doc[doc] = sorted((t for t in tokens_sorted if parent_token(t) is None), key=token_parts)

    # Pass 2: assign section ids -- global preorder across every doc (doc
    # order, then each doc's own top-down, numeric-token-order DFS),
    # mirroring assign_ids' stored-id-first / stale-id-retired / fresh-
    # for-the-rest contract.
    def preorder_keys(doc, toks):
        for tok in toks:
            yield (doc, tok)
            child_toks = sorted(nodes_by_doc[doc][tok]["children"], key=token_parts)
            yield from preorder_keys(doc, child_toks)

    all_keys = [k for doc in docs_sorted for k in preorder_keys(doc, top_tokens_by_doc[doc])]

    section_prefix = LEAF_PREFIX[layer] + "-S"
    used_section_ids: set = set()
    max_numbered = 0
    for doc, tok in all_keys:
        heading_ids = None
        for h in headings_by_doc.get(doc, []):
            if h["number"] == tok:
                heading_ids = h.get("ids")
                break
        stored = heading_ids.get(layer) if heading_ids else None
        if stored is None:
            continue
        if not section_id_belongs_to_layer(stored, layer):
            continue
        used_section_ids.add(stored)

    # 2026-09-05 (fixing the same real-run bug as assign_ids' reserved_ids
    # parameter): the highest-number scan must also cover every reserved
    # section id with this layer's section prefix, not just ids still
    # claimed by a node in THIS build -- a heading's section id can leave
    # a layer (all its statements relayered elsewhere) while the id stays
    # globally reserved.
    for rid in used_section_ids | reserved_ids:
        if rid.startswith(section_prefix):
            m = _ID_NUM_TRAIL_RE.search(rid)
            if m:
                max_numbered = max(max_numbered, int(m.group(1)))

    section_id_by_key: dict = {}
    section_counter = max_numbered + 1
    for doc, tok in all_keys:
        heading_ids = None
        for h in headings_by_doc.get(doc, []):
            if h["number"] == tok:
                heading_ids = h.get("ids")
                break
        stored = heading_ids.get(layer) if heading_ids else None
        if stored is not None and section_id_belongs_to_layer(stored, layer):
            section_id_by_key[(doc, tok)] = stored
            continue
        new_id = f"{section_prefix}{section_counter:03d}"
        while new_id in used_section_ids or new_id in reserved_ids:
            section_counter += 1
            new_id = f"{section_prefix}{section_counter:03d}"
        section_id_by_key[(doc, tok)] = new_id
        used_section_ids.add(new_id)
        reserved_ids.add(new_id)
        section_counter += 1
        if stored is not None:  # it had a stored id, just a stale one
            retired.append((stored, new_id))
            reserved_ids.add(stored)

    # Pass 3: serialize, using the ids assigned above.
    def serialize(doc, tok):
        node = nodes_by_doc[doc][tok]
        out = {"id": section_id_by_key[(doc, tok)], "title": node["title"]}
        if node.get("description"):
            out["description"] = node["description"]
        out["source"] = {"doc": doc, "heading": node["heading"], "lines": node["lines"]}
        out["derived_from"] = []
        child_toks = sorted(node["children"], key=token_parts)
        if child_toks:
            out["sections"] = [serialize(doc, c) for c in child_toks]
        own = sorted(node["items"], key=lambda it: item_sort_key(layer, it))
        if own:
            out["items"] = [build_leaf_output(it, layer) for it in own]
        return out

    layer_out = [serialize(doc, tok) for doc in docs_sorted for tok in top_tokens_by_doc[doc]]

    return layer_out, item_objects, dups, retired


def build_in_memory(root: Path):
    """Core of `build`, factored out so `freeze`/`relayer` can reuse it:
    routes every fragment item to its effective layer (request/require/
    spec/detailed_spec/basic_design/design -- see effective_layer), builds
    request as a flat rootItem list and every other layer as a 文書 > 節 >
    小節 > 文 section tree (build_layer_section_tree, 2026-09-05
    section-node model), and hands back the *original* fragment item dict
    objects (by reference, keyed by their assigned id) so a caller can
    mutate and persist them -- and the (path, frag) pairs `load_fragments`
    returned, so a caller can write fragments back to the same files.

    A fragment's own declared "layer" picks its on-disk SHAPE only (flat
    "items" for request/require/spec, "areas" for detailed_spec/
    basic_design/design) -- this shape is authoring convenience and no
    longer determines section identity or membership; see the
    _SECTIONED_LAYERS comment. An item's own optional "layer" field
    (spec/detailed_spec/basic_design/design -- see effective_layer) can
    still redirect it to a different layer at build time without touching
    the fragment's shape (the relayer mechanism).

    A section node with no item and no native area of its own number (an
    "implied parent" heading) gets its real title from the fragments' own
    harvested headings[] (headings_by_doc_from_fragments -- see
    find_heading_in_headings), not from md -- 2026-09-05 md-independence:
    build no longer touches --repo-root at all; run `harvest-headings`
    first if a doc's headings[] isn't populated yet, or such a node falls
    back to a bare-number placeholder.

    -> (output, item_objects: dict[id, dict], dup_ids: list[str],
        fragments: list[(Path, dict)], retired: list[(old_id, new_id)])
    retired (2026-09-05 fix-ids) is every item/section whose stored id no
    longer matches its current layer (a relayer "layer" override moved it
    since ids were last fixed) -- the caller decides whether/how to log it
    (fix-ids does; a plain `build` still needs the fresh ids to build
    correctly even if nobody's watching the retirement).
    Raises ValueError on an unknown/missing layer; does not touch disk."""
    fragments = load_fragments(root)
    headings_by_doc = headings_by_doc_from_fragments(fragments)
    reserved_ids = build_reserved_ids(fragments, root)

    root_items: list[dict] = []
    request_items: list[dict] = []
    sectioned_items: dict[str, list[dict]] = {layer: [] for layer in _SECTIONED_LAYERS}

    def route(it: dict, target: str) -> None:
        if target == "root":
            root_items.append(it)
        elif target == "request":
            request_items.append(it)
        else:
            sectioned_items[target].append(it)

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
    all_retired: list[tuple] = []
    item_objects: dict[str, dict] = {}

    root_items.sort(key=lambda it: item_sort_key("root", it))
    dups, retired = assign_ids(root_items, "root", reserved_ids)
    all_dups.extend(dups)
    all_retired.extend(retired)
    output["root"] = [build_leaf_output(it, "root") for it in root_items]
    for it in root_items:
        item_objects[it["_id"]] = it

    request_items.sort(key=lambda it: item_sort_key("request", it))
    dups, retired = assign_ids(request_items, "request", reserved_ids)
    all_dups.extend(dups)
    all_retired.extend(retired)
    output["request"] = [build_leaf_output(it, "request") for it in request_items]
    for it in request_items:
        item_objects[it["_id"]] = it

    for layer in _SECTIONED_LAYERS:
        layer_out, layer_item_objects, dups, retired = build_layer_section_tree(
            layer, sectioned_items[layer], headings_by_doc, reserved_ids
        )
        output[layer] = layer_out
        item_objects.update(layer_item_objects)
        all_dups.extend(dups)
        all_retired.extend(retired)

    return output, item_objects, all_dups, fragments, all_retired


def count_sections(nodes: list) -> int:
    return sum(1 + count_sections(n.get("sections", [])) for n in nodes)


def cmd_build(args) -> int:
    root = Path(args.root)
    # 2026-09-05 (md-independence): build no longer reads md at all -- an
    # implied-parent section's title comes from the fragments' own
    # harvested headings[] (harvest-headings), not --repo-root. The flag
    # is still accepted on this subparser (harmless, ignored) so existing
    # invocations/scripts don't need to change.
    try:
        output, _item_objects, dups, _fragments, retired = build_in_memory(root)
    except ValueError as exc:
        print(f"error: {exc}", file=sys.stderr)
        return 1
    if dups:
        for d in sorted(set(dups)):
            print(f"error: duplicate stored id {d!r} used by more than one item/area", file=sys.stderr)
        return 1
    if retired:
        # 2026-09-05 fix-ids: a stored id whose layer override moved it
        # since ids were last fixed gets a fresh id here, every build,
        # whether or not fix-ids runs again -- report it every time,
        # since only `fix-ids` persists the new id (and the retirement)
        # back into fragments/retired-ids.json.
        print(f"note: {len(retired)} stale id(s) retired this build (stored id no longer matches its layer):")
        for old_id, new_id in retired:
            print(f"  {old_id} -> {new_id}")

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

    stmt_counts = {layer: sum(1 for _ in iter_leaf_items_tree(output.get(layer, []))) for layer in _SECTIONED_LAYERS}
    sec_counts = {layer: count_sections(output.get(layer, [])) for layer in _SECTIONED_LAYERS}
    print(f"build: wrote {spec_json_path(root)} ({len(output.get('root', []))} root statements, "
          f"{len(output['request'])} request statements, "
          + ", ".join(
              f"{stmt_counts[layer]} {layer} statements in {sec_counts[layer]} sections"
              for layer in _SECTIONED_LAYERS
          )
          + ") -- schema OK")
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
        if layer == "root":
            # 2026-09-05 root layer: root.json's items come from Issue #11
            # (github, not md) and from derivation-table row lines already
            # inside a keep_for_derivation dropped range -- coverage's job
            # is "did we transcribe every line of the 5 known documents",
            # which root has nothing to do with; skip it entirely rather
            # than trying to md-read a "github:..." doc path.
            continue
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


def fmt_sections(sections: list, depth: int = 2) -> list:
    """2026-09-05 section-node model: replaces fmt_area_blocks, recursive
    over a layer's section tree (every sectioned layer, not just the
    former area-layers) -- one block per node, heading depth increasing
    with nesting (capped at 6, ATX's own limit), children's blocks
    following their parent's in the same flat list write_export_file
    expects. A node's own derived_from renders the same *導出元:...*
    marker an item's does, since a section can carry one too."""
    blocks: list = []
    for sec in sections:
        mark = "#" * min(depth, 6)
        block = [f"{mark} {sec['id']} {sec['title']}"]
        if sec.get("description"):
            block.append("")
            block.append(sec["description"].strip())
        derived = sec.get("derived_from")
        if derived:
            block.append("")
            block.append(f"*導出元: {', '.join(derived)}*")
        for it in sec.get("items", []):
            block.append("")
            block.extend(fmt_item(it))
        blocks.append(block)
        blocks.extend(fmt_sections(sec.get("sections", []), depth + 1))
    return blocks


def cmd_export(args) -> int:
    root = Path(args.root)
    spec = read_json(spec_json_path(root))
    out_dir = export_dir(root)
    out_dir.mkdir(parents=True, exist_ok=True)

    write_export_file(out_dir / "request.md", "要求", [fmt_item(it) for it in spec["request"]])
    write_export_file(out_dir / "require.md", "要件定義", fmt_sections(spec["require"]))
    write_export_file(out_dir / "spec.md", "基本仕様", fmt_sections(spec["spec"]))
    write_export_file(out_dir / "detailed_spec.md", "詳細仕様", fmt_sections(spec["detailed_spec"]))
    write_export_file(out_dir / "basic_design.md", "基本設計", fmt_sections(spec["basic_design"]))
    write_export_file(out_dir / "design.md", "詳細設計", fmt_sections(spec["design"]))

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


def check_leaf_item(item, label: str, problems: list[str], layer: str = None) -> None:
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
    if layer == "root":
        # 2026-09-05 root layer: rootItem has no derived_from field at
        # all (root is the system's own root, nothing to derive from) --
        # unlike every other layer, fragments/root.json must NOT author
        # one either.
        if "derived_from" in item:
            problems.append(f"{label}: root-layer item must not carry 'derived_from' (rootItem has no such field)")
    elif "derived_from" not in item:
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
                check_leaf_item(it, f"items[{i}]", problems, layer)
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
    build time. basic_design and design fragments are both area-shaped.
    2026-09-05: root.json yields nothing here -- every caller of this
    function (harvest-cites, qualifier-check, source-check, relayer
    report/apply) is an md-based migration check or a relayer routing
    tool, and root items are neither md-sourced (an F-item's doc is
    github, not any of the 5 documents) nor ever relayer-eligible."""
    if frag.get("layer") == "root":
        return
    if frag.get("layer") in _AREA_LAYERS:
        for area in frag.get("areas", []):
            for it in area.get("items", []):
                yield it
    else:
        for it in frag.get("items", []):
            yield it


# ------------------------------------------------------- harvest-headings --
# 2026-09-05 (md-independence): build's section tree used to read the real
# md text (via --repo-root) to find the true title of an "implied parent"
# node -- a heading with numbered children but no item of its own. Once md
# retires, that data must live in fragments instead. harvest-headings reads
# each fragment's own "doc" once and writes every numbered ATX heading of
# that document, in order, into a new top-level "headings" array -- build
# then reads THAT instead of md (see headings_by_doc/build_layer_section_
# tree). "Numbered" mirrors what heading_number_token already recognises
# (digit-dot) plus the P-00N/R-N patterns cites/self-naming resolution
# already special-cases -- harvested for completeness even though build's
# own fallback-resolution only consumes the digit-dot subset (a P-00N/R-N
# heading is still reachable by an item's own heading text, never needs
# the "nearest preceding numbered heading" fallback).
_HEADING_HARVEST_RE = re.compile(r"^(#{1,6}) ((?:[0-9]+(?:\.[0-9]+)*)|P-[0-9]{3}|R-[0-9]+)\.?\s*(.*)$")


def is_numeric_token(token: str) -> bool:
    """Whether a harvested heading's "number" is the digit-dot shape
    build's section tree actually nests by (as opposed to a harvested but
    non-nesting P-00N/R-N heading)."""
    return bool(re.fullmatch(r"[0-9]+(?:\.[0-9]+)*", token))


def cmd_harvest_headings(args) -> int:
    root = Path(args.root)
    repo_root = Path(args.repo_root)
    files = list_fragment_files(root)
    if not files:
        print("harvest-headings: no fragments found")
        return 0

    md_cache: dict = {}

    def get_md_lines(doc: str) -> list[str]:
        if doc not in md_cache:
            md_cache[doc] = (repo_root / doc).read_text(encoding="utf-8").splitlines()
        return md_cache[doc]

    counts: dict = {}
    for path in files:
        frag = read_json(path)
        doc = frag.get("doc")
        headings = []
        for i, line in enumerate(get_md_lines(doc), start=1):
            m = _HEADING_HARVEST_RE.match(line)
            if m:
                headings.append({
                    "number": m.group(2),
                    "title": m.group(3).strip(),
                    "line": i,
                    "level": len(m.group(1)),
                })
        frag["headings"] = headings
        counts[str(path)] = len(headings)
        text = json.dumps(frag, ensure_ascii=False, indent=2) + "\n"
        path.write_text(text, encoding="utf-8")

    print("--- harvest-headings ---")
    for p in sorted(counts):
        print(f"  {p}: {counts[p]} headings")
    print(f"  total: {sum(counts.values())}")
    return 0


def headings_by_doc_from_fragments(fragments: list) -> dict:
    """doc -> [{"number","title","line","level"}, ...] sorted by line,
    deduped by line (two fragments sharing a doc -- e.g. det1/det2 both
    from 詳細設計 v0.1.md -- harvest the identical full-document heading
    list into each, so this must not double-count). `fragments` is a
    load_fragments-shaped list of (path, frag dict)."""
    by_doc: dict = {}
    seen: dict = {}
    for _path, frag in fragments:
        doc = frag.get("doc")
        if doc is None:
            continue
        bucket = by_doc.setdefault(doc, [])
        seen_lines = seen.setdefault(doc, set())
        for h in frag.get("headings", []):
            if h["line"] in seen_lines:
                continue
            seen_lines.add(h["line"])
            bucket.append(h)
    for doc in by_doc:
        by_doc[doc].sort(key=lambda h: h["line"])
    return by_doc


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

def iter_leaf_items_tree(sections: list):
    """Recursively yield every leaf item across a section-tree list (a
    layer's own top-level array in specification.json), depth-first,
    section order preserved -- a section's own "items" before its
    children's."""
    for sec in sections:
        yield from sec.get("items", [])
        yield from iter_leaf_items_tree(sec.get("sections", []))


def iter_all_sections(sections: list):
    """Recursively yield every section node (self before children),
    depth-first, section order preserved."""
    for sec in sections:
        yield sec
        yield from iter_all_sections(sec.get("sections", []))


def build_id_index(spec: dict) -> dict:
    """Every statement id (any layer's item) -> its record, in
    specification.json's own deterministic order. Section ids are
    deliberately NOT included here (see build_full_index) -- this index's
    other job, self_naming_candidates' full-corpus scan, would otherwise
    have to guard against matching a section's title text."""
    idx: dict = {}
    for it in spec.get("root", []):
        idx[it["id"]] = dict(it, layer="root")
    for it in spec["request"]:
        idx[it["id"]] = dict(it, layer="request")
    for layer in _SECTIONED_LAYERS:
        for it in iter_leaf_items_tree(spec.get(layer, [])):
            idx[it["id"]] = dict(it, layer=layer)
    return idx


def build_full_index(spec: dict) -> dict:
    """build_id_index plus every section id -> {"layer": ...} (2026-09-05
    section-node model): finalize_derived_from's rank check needs a
    candidate/target's layer regardless of whether it is a statement or a
    section id, since Task A / the 要求→要件 table's §-targets now resolve
    to section ids. Deliberately a separate function from build_id_index
    (not merged into it) so self_naming_candidates' whole-corpus scan never
    sees a section's title text as a candidate statement."""
    idx = build_id_index(spec)
    for layer in _SECTIONED_LAYERS:
        for sec in iter_all_sections(spec.get(layer, [])):
            idx[sec["id"]] = {"layer": layer, "title": sec["title"]}
    return idx


def doc_matches_mark(doc: str, mark: str) -> bool:
    """Whether `doc` (a fragment/section source path) belongs to the
    document `mark` names -- 基本仕様/要件定義/別紙A/別紙B/別紙C match by a
    literal substring; 本冊 (詳細設計 v0.1.md, the only design-family doc
    with no distinguishing substring of its own) matches positively on
    "詳細設計" and negatively excludes every 別紙. The positive "詳細設計"
    check matters once relayering can put an item from any doc into any
    sectioned layer (2026-09-05): a require/spec-layer section's doc never
    contains "詳細設計" or "別紙", so a mark-scoped search across every
    layer (section_id_candidates_cross_layer) can't mistake it for 本冊 the
    way a negative-only check (not 別紙A/B/C) would if it were applied
    outside a single already-known design-family layer."""
    if mark == "本冊":
        return "詳細設計" in doc and "別紙A" not in doc and "別紙B" not in doc and "別紙C" not in doc
    return mark in doc


def scope_items(spec: dict, scope: str) -> list:
    """scope: 'require' | 'spec' | '<sectioned_layer>:本冊' |
    '<sectioned_layer>:別紙A' | '<sectioned_layer>:別紙B' |
    '<sectioned_layer>:別紙C'. Flattens the scope's section tree into its
    leaf items (2026-09-05: require/spec are trees too, so both branches
    below now go through the same doc-filtered tree walk as an
    area-layer's scope always did)."""
    layer = scope_to_layer(scope)
    if layer is None:
        return []
    _, _, mark = (scope or "").partition(":")
    if not mark:
        return list(iter_leaf_items_tree(spec.get(layer, [])))
    out = []
    for sec in spec.get(layer, []):
        if doc_matches_mark(sec["source"]["doc"], mark):
            out.extend(iter_leaf_items_tree([sec]))
    return out


def scoped_sections(spec: dict, scope: str) -> list:
    """Like scope_items, but yields section NODES instead of leaf items --
    used by section_id_candidates (2026-09-05: table-row §-references
    resolve to the section itself, not to every statement under it)."""
    layer = scope_to_layer(scope)
    if layer is None:
        return []
    _, _, mark = (scope or "").partition(":")
    if not mark:
        return list(iter_all_sections(spec.get(layer, [])))
    out = []
    for sec in spec.get(layer, []):
        if doc_matches_mark(sec["source"]["doc"], mark):
            out.extend(iter_all_sections([sec]))
    return out


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
    """Statement ids under `number` (exact node or any descendant),
    unchanged since before the section-node model -- this is rule 1
    (inline cites): 文中の引用は文の辺 (LAYERING.md SS1.1), so a citing
    statement's target still fans out to every statement in the cited
    section, never to the section node itself."""
    return [
        it["id"]
        for it in scope_items(spec, scope)
        if section_matches(number, heading_number_token(it["source"]["heading"]))
    ]


def section_id_candidates(spec: dict, scope: str, number: str) -> list:
    """The section NODE id(s) whose own heading number token exactly
    equals `number` (2026-09-05 section-node model) -- used by the
    table-based rules (要求→要件 導出表 §-targets, and every Task A
    traceability-table §-reference, both source and upstream side): 表を
    文単位に展開しない (LAYERING.md SS1.1), so a §N reference here attaches
    to the one section node for N, not to every statement under it. A
    descendant statement still inherits the edge through "effective
    upstream reach" (own derived_from union ancestors'), computed, not
    stored -- see cmd_apply_derivation. At most one id in the ordinary
    case (each doc has one node per number); a list because the caller
    (resolve_table_ref et al.) already expects one."""
    return [
        sec["id"]
        for sec in scoped_sections(spec, scope)
        if heading_number_token(sec["source"]["heading"]) == number
    ]


def section_id_candidates_cross_layer(spec: dict, doc_mark: str, number: str) -> list:
    """Like section_id_candidates, but searches every sectioned layer
    (not one predetermined layer) for a node matching (doc_mark, number).
    2026-09-05 (post-relayer fix): a traceability-table row names a
    SOURCE-DOCUMENT section (e.g. 基本仕様 §5.2), not a layer -- after
    relayering, that document section's statements can be split across
    more than one layer (spec / detailed_spec / basic_design), so it can
    legitimately have a section node of the same (doc, number) in each of
    them. The edge attaches to EVERY such node, in any layer -- dropping
    it to "the first match" would silently under-realize whichever layer
    lost the coin flip. require never splits (it is never a relayer
    target), so a require-scoped call here degrades to exactly one match,
    same as section_id_candidates."""
    out = []
    for layer in _SECTIONED_LAYERS:
        for sec in spec.get(layer, []):
            if not doc_matches_mark(sec["source"]["doc"], doc_mark):
                continue
            for node in iter_all_sections([sec]):
                if heading_number_token(node["source"]["heading"]) == number:
                    out.append(node["id"])
    return out


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
    """Fixed order request<require<spec<detailed_spec<basic_design<design (every design area, whichever
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
    for it in spec.get("root", []):
        yield it
    for it in spec["request"]:
        yield it
    for layer in _SECTIONED_LAYERS:
        yield from iter_leaf_items_tree(spec.get(layer, []))


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


def split_outside_parens(text: str, sep_chars: str = "・/") -> list:
    """Split `text` on any char in `sep_chars`, but never inside a
    parenthesised span (both full-width （） and half-width ()) -- e.g.
    "F4・F7・Owner 裁定 2026-08-23（8）" -> 3 tokens (the ・ are real
    separators), but "Owner 裁定 2026-08-24（判断/承認分離）" stays ONE
    token (that "/" is a natural-language "and" inside the ruling's own
    parenthetical label, not a second citation) -- found against the real
    root-layer harvest, 2026-09-05: the plain _TABLE_TOKEN_SPLIT_RE.split
    used here corrupted every such ruling into two or three garbage
    tokens ("Owner 裁定 2026-08-24（判断", "承認分離）", ...)."""
    parts = []
    buf = []
    depth = 0
    for ch in text:
        if ch in "（(":
            depth += 1
            buf.append(ch)
        elif ch in "）)":
            depth = max(0, depth - 1)
            buf.append(ch)
        elif ch in sep_chars and depth == 0:
            parts.append("".join(buf))
            buf = []
        else:
            buf.append(ch)
    parts.append("".join(buf))
    return [p.strip() for p in parts if p.strip()]

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
        # 2026-09-05 section-node model: a §-target in the 要求→要件 table
        # attaches to the require-layer SECTION node, not to every
        # statement under it (LAYERING.md SS1.1 -- 表を文単位に展開しない).
        return section_id_candidates(spec, "require", key), matched
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


def collect_cites_edges(spec: dict, id_index: dict) -> dict:
    """target_id -> set of candidate source ids, from cites (rule 1) only,
    before rule 3 filtering/dedup/sort. 2026-09-05 (md-independence):
    formerly also did rule 2 (要求→要件 table) here by re-parsing md; that
    row-based resolution now lives in trace-tables.json (harvest-trace-
    tables) and is merged in by collect_derivation_edges via
    collect_edges_from_trace_tables, uniformly with Task A -- once a row
    is loaded from JSON, "which of the 5 source tables it came from" no
    longer matters to edge-building (see that function's docstring)."""
    edges: dict = {}
    for e in build_derivation_candidates(spec, id_index):
        if e["layer_relation"] in ("adjacent-upstream", "skip-upstream"):
            edges.setdefault(e["id"], set()).update(c["id"] for c in e["candidates"])
    return edges


# ---------------------------------------------------- trace-tables.json --
# 2026-09-05 (md-independence): harvest-trace-tables reads md ONE LAST TIME
# (the 要求→要件 derivation table, kept_for_derivation-marked, and the 4
# document-trailing traceability appendices, reason-marked) and writes
# every row into docs/canonical/relations/trace-tables.json in one uniform
# shape: {"source": ref|[ref,...], "upstream": [ref, ...], "kind", "note"}
# where a ref is {"doc": <fragment-doc-path>|null, "section": "§N"|
# "<bare id/code/unknown token, verbatim>"}. "source" is always the row's
# own/downstream reference (the 要求→要件 table's 下流ノード column, or an
# appendix's own-section column); "upstream" is always its cited upstream
# reference(s) -- this direction is uniform across all 5 source tables, so
# apply-derivation's edge-building (collect_edges_from_trace_tables) needs
# no per-table branching once the JSON exists: "source" resolves to the
# edge's target id(s), each "upstream" entry resolves to a source id. A
# §-ref's doc is resolved to a real fragment doc path AT HARVEST TIME
# (doc_path_for_mark) -- not a layer or a mark -- so the id resolution that
# actually turns (doc, §N) into a section id (section_id_candidates_cross_
# layer) still runs fresh against the CURRENT build every time apply-
# derivation runs (ids are never baked into this file). A bare id/self-
# naming code has "doc": null (it resolves by id, not by doc); an
# unrecognised token (Issue/F-item/裁定 label) is ALSO kept, doc: null,
# section: the token verbatim -- zero loss, even though it will never
# resolve to an edge (classify_table_ref already calls this "unknown").

def doc_path_for_mark(mark: str, all_docs) -> str:
    """The one fragment-declared doc path matching a document mark
    (本冊/基本仕様/要件定義/別紙A/別紙B/別紙C), or None if no fragment names
    that document (shouldn't happen in this corpus, but harvesting must
    not crash on it)."""
    for doc in all_docs:
        if doc_matches_mark(doc, mark):
            return doc
    return None


def doc_mark_for_path(doc: str):
    """Reverse of doc_path_for_mark -- the document mark a fragment's own
    doc path belongs to, for resolve_trace_ref's cross-layer section
    lookup (section_id_candidates_cross_layer takes a mark, not a path)."""
    if doc is None:
        return None
    for mark in ("別紙A", "別紙B", "別紙C", "基本仕様", "要件定義", "本冊"):
        if doc_matches_mark(doc, mark):
            return mark
    return None


def token_to_trace_ref(token: str, doc_for_section) -> dict:
    """One 上流ノード/下流ノード/appendix-cell token -> a trace-tables.json
    ref, reusing classify_table_ref's exact classification (id/section/
    selfname/unknown) so harvesting and the pre-existing derivation-
    candidates command never disagree about what a token means.
    `doc_for_section` is the doc path to attach when the token classifies
    as a section reference (the table's own document for a bare §, or
    whatever a doc-abbreviated §-group resolved to -- see the two harvest
    functions below, which call this differently per case). Uses `key`
    (the bare digit-dot number), not `matched`, for a section reference --
    matched keeps a compound target's item-suffix verbatim (e.g. "§21-a"),
    which resolve_trace_ref's consumer-side lookup (an exact match against
    heading_number_token, always suffix-free) would then never find,
    silently losing every §21-a/-b/-c/-d-style row (caught against the
    real corpus: REQ-S048's derived_from went from [R-2, R-3] to [] before
    this fix, exactly the §21-a..d cluster)."""
    kind, key, matched = classify_table_ref(token)
    if kind == "section":
        return {"doc": doc_for_section, "section": f"§{key}"}
    if kind in ("id", "selfname"):
        return {"doc": None, "section": key}
    return {"doc": None, "section": token.strip()}


def harvest_derivation_table_rows(root: Path, repo_root: Path) -> list:
    """The 要求→要件 derivation table's rows (every dropped-log entry
    marked keep_for_derivation -- CONVERSION.md SS4/SS6 -- which in this
    corpus is 第I部 根→要求 and 第II部 要求→要件 both, since both are marked
    the same way; harvested as-is, not filtered to "just 要求→要件" since
    there is no data-level way to tell them apart here). 下流ノード ->
    "source" (may be a list when the cell is compound, e.g. "§12/§19");
    上流ノード -> "upstream". Both this table's node types share ONE
    document (要求・要件定義), so every section-kind token here uses that
    doc regardless of which column it came from."""
    rows_out = []
    for doc, start, end in find_derivation_table_ranges(root):
        md_lines = (repo_root / doc).read_text(encoding="utf-8").splitlines()
        for line_no, cells in parse_md_table_rows(md_lines, start, end):
            if len(cells) != 4:
                rows_out.append({
                    "source": None, "upstream": [], "kind": None,
                    "note": f"{doc} L{line_no} 列数が4でない行（そのまま記録）: " + " | ".join(cells),
                })
                continue
            upstream, downstream, reason, state = cells
            source_refs = [token_to_trace_ref(t, doc) for t in split_outside_parens(downstream)]
            upstream_refs = [token_to_trace_ref(t, doc) for t in split_outside_parens(upstream)]
            rows_out.append({
                "source": source_refs[0] if len(source_refs) == 1 else source_refs,
                "upstream": upstream_refs,
                "kind": state,
                "note": f"{doc} L{line_no} | 上流: {upstream} / 下流: {downstream} / 理由: {reason}",
            })
    return rows_out


def harvest_traceability_appendix_rows(root: Path, repo_root: Path, all_docs) -> tuple:
    """-> (rows, by_table Counter, unparseable_count). The 4 document-
    trailing traceability appendices (found via dropped-log reason text
    containing トレーサビリティ表): own-section column -> "source" (always
    this table's own doc); upstream column -> "upstream", each §-group's
    doc resolved via its abbreviation (本冊/基本/要件) or, absent one, the
    table's one fixed default (basic_design's own default is 要件定義,
    本冊's is 基本仕様, 別紙A/別紙C have none) -- an unresolvable bare
    §-group (no abbreviation, no default) makes the whole row unparseable,
    same as the pre-harvest md-parsing path did."""
    rows_out = []
    by_table = Counter()
    unparseable = 0
    for doc, start, end in find_traceability_table_ranges(root):
        profile = _trace_table_profile(doc)
        if profile is None:
            continue
        table_name, _source_doc_mark, default_bare_doc_mark = profile
        default_bare_doc = doc_path_for_mark(default_bare_doc_mark, all_docs) if default_bare_doc_mark else None
        md_lines = (repo_root / doc).read_text(encoding="utf-8").splitlines()
        for line_no, cells in parse_generic_table_rows(md_lines, start, end):
            if len(cells) != 3:
                rows_out.append({"source": None, "upstream": [], "kind": None, "note": f"{doc} L{line_no} 列数が3でない行: " + " | ".join(cells)})
                unparseable += 1
                continue
            own_cell, upstream_cell, kind_cell = cells
            m = _TRACE_OWN_SECTION_RE.match(own_cell)
            if not m:
                rows_out.append({"source": None, "upstream": [], "kind": kind_cell, "note": f"{doc} L{line_no} own-section 解析不能: {own_cell}"})
                unparseable += 1
                continue
            source_ref = {"doc": doc, "section": f"§{m.group(1)}"}
            upstream_refs = []
            had_problem = False
            for mm in _TRACE_SCAN_RE.finditer(upstream_cell):
                if mm.group("secdoc"):
                    mark = _TRACE_DOC_ABBR_TO_DOCMARK[mm.group("secdoc")]
                    ref_doc = doc_path_for_mark(mark, all_docs)
                    for num in split_trace_seclist(mm.group("seclist")):
                        upstream_refs.append({"doc": ref_doc, "section": f"§{num}"})
                elif mm.group("bareseclist"):
                    if default_bare_doc is None:
                        had_problem = True
                        continue
                    for num in split_trace_seclist(mm.group("bareseclist")):
                        upstream_refs.append({"doc": default_bare_doc, "section": f"§{num}"})
                elif mm.group("bare"):
                    upstream_refs.append({"doc": None, "section": mm.group("bare")})
            if had_problem:
                rows_out.append({"source": source_ref, "upstream": upstream_refs, "kind": kind_cell, "note": f"{doc} L{line_no} 未解決の裸§グループを含む: {upstream_cell}"})
                unparseable += 1
                continue
            rows_out.append({
                "source": source_ref,
                "upstream": upstream_refs,
                "kind": kind_cell,
                "note": f"{doc} L{line_no} | 本節: {own_cell} / 上流: {upstream_cell} / 理由: {kind_cell}",
            })
            by_table[table_name] += 1
    return rows_out, by_table, unparseable


def trace_tables_path(root: Path) -> Path:
    return root / "relations" / "trace-tables.json"


def cmd_harvest_trace_tables(args) -> int:
    root = Path(args.root)
    repo_root = Path(args.repo_root)
    all_docs = {frag.get("doc") for _path, frag in load_fragments(root) if frag.get("doc")}

    deriv_rows = harvest_derivation_table_rows(root, repo_root)
    appendix_rows, by_table, unparseable = harvest_traceability_appendix_rows(root, repo_root, all_docs)

    all_rows = deriv_rows + appendix_rows
    out_path = trace_tables_path(root)
    out_path.parent.mkdir(parents=True, exist_ok=True)
    out_path.write_text(json.dumps(all_rows, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")

    print("--- harvest-trace-tables ---")
    print(f"  要求→要件 derivation table rows (keep_for_derivation, 第I部+第II部 combined -- see docstring): {len(deriv_rows)}")
    print("  traceability appendix rows, by document:")
    for table_name in ("基本仕様", "本冊", "別紙A", "別紙C"):
        print(f"    {table_name}: {by_table.get(table_name, 0)}")
    print(f"  appendix rows total: {sum(by_table.values())}")
    print(f"  appendix rows unparseable: {unparseable}")
    print(f"  wrote {out_path} ({len(all_rows)} rows total)")
    return 0


_ROOT_ALIAS_RE = re.compile(r"^#11\s+(F[0-9]+)$")


def normalize_root_token(token: str) -> str:
    """Collapse an alternate way of citing an F-item (e.g. "#11 F1",
    found verbatim in the derivation table) to its bare form ("F1") --
    both name the same Issue #11 freeze item, so this must resolve (and,
    at harvest time, count) as the SAME root item, not a second one. A
    bare "#11" (no F-number) passes through unchanged -- CONVERSION.md/
    the root-layer task says it maps to nothing, and it never appeared as
    its own token in this corpus (checked: 0 occurrences) -- so nothing
    else needs a special case here."""
    m = _ROOT_ALIAS_RE.match(token.strip())
    return m.group(1) if m else token.strip()


def build_root_token_index(spec: dict) -> dict:
    """token (verbatim, as it appears in a trace-table cell -- "F1",
    "Owner 裁定 2026-08-21", ...) -> root item id. Built from each root
    item's own source.heading, which harvest-root sets to exactly the
    token that names it (an F-number for an F-item, the token text
    itself otherwise) -- no separate stored field needed."""
    return {it["source"]["heading"]: it["id"] for it in spec.get("root", [])}


def resolve_trace_ref(ref, spec: dict, id_index: dict, root_index: dict) -> list:
    """One harvested {doc, section} ref (or None) -> resolved id(s),
    against the CURRENT build (spec/id_index/root_index) -- ids are never
    baked into trace-tables.json, only the doc+section/code data is, so
    this always reflects whatever the corpus looks like right now, same
    as the old md-parsing path did on every run."""
    if not ref:
        return []
    section = ref.get("section")
    if not section:
        return []
    if section.startswith("§"):
        doc = ref.get("doc")
        mark = doc_mark_for_path(doc) if doc else None
        if mark is None:
            return []
        return section_id_candidates_cross_layer(spec, mark, section[1:])
    if re.match(r"^(R-[1-5]|P-[0-9]{3})$", section):
        return [section] if section in id_index else []
    if re.match(r"^(NFR-[0-9]{3}|OOS-[0-9]{3})$", section):
        return self_naming_candidates(id_index, section)
    # 2026-09-05 (root layer): an F-item or Owner-ruling token (including
    # the "#11 F<n>" alias form) resolves to its root-layer item.
    normalized = normalize_root_token(section)
    if normalized in root_index:
        return [root_index[normalized]]
    return []  # unknown token (a bare "#11", or anything else unrecognised) -- never resolves


def collect_edges_from_trace_tables(rows: list, spec: dict, id_index: dict, root_index: dict) -> dict:
    """target_id -> set(source_id), from every harvested row (要求→要件
    table + all 4 traceability appendices, uniformly -- see the
    trace-tables.json docstring above for why one mechanism suffices for
    both). Replaces the old rule-2 (table_candidates) and rule-3/Task A
    (collect_traceability_table_edges) md-parsing paths. root_index lets
    an F-item/ruling upstream token resolve too (2026-09-05 root layer) --
    this is also how request (R-N) gets derived_from into root, and how a
    require SECTION gets an edge straight to a root item, both via the
    exact same "source" (target) / "upstream" (source) resolution, no
    separate code path per team-lead's task 2/3."""
    edges: dict = {}
    for row in rows:
        src = row.get("source")
        src_refs = src if isinstance(src, list) else ([src] if src else [])
        targets: list = []
        for ref in src_refs:
            targets.extend(resolve_trace_ref(ref, spec, id_index, root_index))
        if not targets:
            continue
        sources: list = []
        for ref in row.get("upstream", []):
            sources.extend(resolve_trace_ref(ref, spec, id_index, root_index))
        if not sources:
            continue
        for t in targets:
            edges.setdefault(t, set()).update(sources)
    return edges


# --- Rule 3 (traceability tables): each document's own trailing 「付記
# （非規範）: トレーサビリティ表」 records, per row, which upstream section(s)
# a given section of THIS document realises. Found via the dropped-log
# reason text (these entries are not marked keep_for_derivation -- that
# flag is reserved for the 要求→要件 table -- so fragments need no edit
# for this to work).

_TRACE_DOC_ABBR_TO_DOCMARK = {"本冊": "本冊", "基本": "基本仕様", "要件": "要件定義"}
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


def resolve_trace_upstream_cell(spec: dict, id_index: dict, cell: str, default_bare_doc_mark):
    """-> (candidate_ids: list[str], had_unresolvable_bare_section: bool).
    default_bare_doc_mark is the document mark a §-group with no doc
    abbreviation at all resolves against (基本仕様's own table has none,
    per its 1 fixed upstream doc 要件定義); None means "no default" -- such
    a group is reported as a parse problem rather than guessed.
    2026-09-05 section-node model: a §-reference here resolves to every
    upstream SECTION node id whose (doc, number) matches, in ANY sectioned
    layer (section_id_candidates_cross_layer -- see its docstring for why
    single-layer resolution stopped being correct once relayering could
    split one document section's statements across layers) -- the
    returned list can therefore mix section ids (from a §-group, possibly
    several per group) and statement ids (from a bare id/self-naming code
    in the same cell); finalize_derived_from's rank check works on either
    via build_full_index."""
    ids = []
    problem = False
    for m in _TRACE_SCAN_RE.finditer(cell):
        if m.group("secdoc"):
            doc_mark = _TRACE_DOC_ABBR_TO_DOCMARK[m.group("secdoc")]
            for num in split_trace_seclist(m.group("seclist")):
                ids.extend(section_id_candidates_cross_layer(spec, doc_mark, num))
        elif m.group("bareseclist"):
            if default_bare_doc_mark is None:
                problem = True
                continue
            for num in split_trace_seclist(m.group("bareseclist")):
                ids.extend(section_id_candidates_cross_layer(spec, default_bare_doc_mark, num))
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
    """-> (table_name, source_doc_mark, default_bare_doc_mark) for a doc
    path, or None if it doesn't match one of the 4 known traceability
    tables. 2026-09-05: doc marks, not layer scopes -- a document's own
    section can now live in more than one sectioned layer after
    relayering (section_id_candidates_cross_layer resolves across all of
    them)."""
    if "基本仕様" in doc:
        return "基本仕様", "基本仕様", "要件定義"
    if "別紙A" in doc:
        return "別紙A", "別紙A", None
    if "別紙C" in doc:
        return "別紙C", "別紙C", None
    if "詳細設計" in doc:
        return "本冊", "本冊", "基本仕様"
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
    by_table ({table_name: rows_parsed})).
    2026-09-05 section-node model: a row's own section (leftmost column)
    is always a §-reference, so `targets` here is always one or more
    section ids (section_id_candidates_cross_layer, not section_
    candidates -- every matching node in every sectioned layer, since
    relayering can split one document section's statements across layers)
    -- every edge this function produces is therefore keyed by a section
    id on the target side (the upstream side can still mix section and
    statement ids, see resolve_trace_upstream_cell)."""
    edges: dict = {}
    rows_parsed = 0
    unparseable = []
    by_table = Counter()

    for doc, start, end in find_traceability_table_ranges(root):
        profile = _trace_table_profile(doc)
        if profile is None:
            continue
        table_name, source_doc_mark, default_bare_doc_mark = profile
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
            targets = section_id_candidates_cross_layer(spec, source_doc_mark, m.group(1))
            upstream_ids, had_problem = resolve_trace_upstream_cell(spec, id_index, upstream_cell, default_bare_doc_mark)
            if had_problem:
                unparseable.append((table_name, line_no, cells))
                continue
            rows_parsed += 1
            by_table[table_name] += 1
            for t in targets:
                edges.setdefault(t, set()).update(upstream_ids)

    return edges, {"rows_parsed": rows_parsed, "unparseable": unparseable, "by_table": by_table}


def collect_derivation_edges(spec: dict, id_index: dict, root: Path):
    """-> (edges, stats). edges is target_id -> set of candidate source
    ids, merged from cites (rule 1) and every harvested trace-table row
    (rules 2+3, collect_edges_from_trace_tables), before rule 3
    filtering/dedup/sort. 2026-09-05 (md-independence): no repo_root --
    reads docs/canonical/relations/trace-tables.json (harvest-trace-
    tables' output), not md; run that command first if it's stale/absent
    (an absent file degrades to rule-1-only edges, not an error)."""
    edges = collect_cites_edges(spec, id_index)
    rows = read_json(trace_tables_path(root)) if trace_tables_path(root).exists() else []
    root_index = build_root_token_index(spec)
    trace_edges = collect_edges_from_trace_tables(rows, spec, id_index, root_index)
    for t, srcs in trace_edges.items():
        edges.setdefault(t, set()).update(srcs)
    return edges, {"rows_loaded": len(rows)}


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
        if layer == "root":  # 2026-09-05: root is the only layer with no derived_from field
            continue
        final_list, _dropped = finalize_derived_from(iid, edges.get(iid, ()), id_index)
        if final_list:
            counts[layer] += 1
    return counts


def build_ancestor_section_map(spec: dict, layer: str) -> dict:
    """item_id -> [ancestor section id, ...] (root-to-immediate-parent
    order) for one sectioned layer -- used by "effective upstream reach"
    (own derived_from union every ancestor section's derived_from,
    computed, not stored -- LAYERING.md SS1.1 "辿るとき")."""
    out: dict = {}

    def walk(sections, chain):
        for sec in sections:
            new_chain = chain + [sec["id"]]
            for it in sec.get("items", []):
                out[it["id"]] = new_chain
            walk(sec.get("sections", []), new_chain)

    walk(spec.get(layer, []), [])
    return out


def cmd_apply_derivation(args) -> int:
    root = Path(args.root)
    spec = read_json(spec_json_path(root))
    # id_index (statement ids only) feeds collect_cites_edges, which calls
    # self_naming_candidates -- every id_index entry must have a
    # "statement" key, so a section entry (title only) must never be in
    # it. full_index adds section ids on top, for finalize_derived_from/
    # summarize_edges: a trace-table row's §-target now keys edges by
    # section id (2026-09-05 section-node model), and the rank check needs
    # a section id's layer too.
    id_index = build_id_index(spec)
    full_index = build_full_index(spec)

    # 2026-09-05 (md-independence): reads docs/canonical/relations/
    # trace-tables.json (harvest-trace-tables' output), not md -- no
    # --repo-root anywhere in this command any more. Run harvest-headings
    # and harvest-trace-tables first if either is stale; an absent
    # trace-tables.json degrades to cites-only edges rather than erroring.
    edges_before = collect_cites_edges(spec, id_index)
    stats_before = summarize_edges(spec, full_index, edges_before)

    tt_path = trace_tables_path(root)
    trace_rows = read_json(tt_path) if tt_path.exists() else []
    root_index = build_root_token_index(spec)
    trace_edges = collect_edges_from_trace_tables(trace_rows, spec, id_index, root_index)
    edges_after = {k: set(v) for k, v in edges_before.items()}
    for t, srcs in trace_edges.items():
        edges_after.setdefault(t, set()).update(srcs)
    stats_after = summarize_edges(spec, full_index, edges_after)

    edges_added_by_trace = sum(
        len(srcs - edges_before.get(t, set())) for t, srcs in trace_edges.items()
    )

    print("--- apply-derivation: trace-tables.json (要求→要件 table + 4 traceability appendices) ---")
    print(f"  rows loaded: {len(trace_rows)}" + (f" (from {tt_path})" if tt_path.exists() else " (file absent -- ran with cites only)"))
    print("  statements with derived_from from cites alone (rule 1):")
    for layer in ("request", "require", "spec", "detailed_spec", "basic_design", "design"):
        print(f"    {layer}: {stats_before.get(layer, 0)}")
    print("  statements with derived_from from cites + trace-tables.json (rules 1+2+3):")
    for layer in ("request", "require", "spec", "detailed_spec", "basic_design", "design"):
        print(f"    {layer}: {stats_after.get(layer, 0)}")
    print(f"  edges added by trace-tables.json (raw candidate pairs, before rule 3): {edges_added_by_trace}")

    cited_ids = {it["id"] for it in iter_all_statements(spec) if it.get("cites")}

    recomputed: dict = {}
    dropped_total = 0
    sizes = []
    empty_after_cite = 0
    for it in iter_all_statements(spec):
        iid = it["id"]
        layer = id_index[iid]["layer"]
        if layer == "root":
            continue  # rootItem schema has no derived_from field (2026-09-05: root, not request)
        final_list, dropped = finalize_derived_from(iid, edges_after.get(iid, ()), full_index)
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

    # 2026-09-05 section-node model: SECTION-targeted edges (rule 2's
    # §-targets, every Task A row's own-section target) never appear in
    # `recomputed` above (iter_all_statements only yields leaf items) --
    # compute and report them in parallel here, over every section node in
    # every sectioned layer.
    section_recomputed: dict = {}
    section_dropped_total = 0
    section_sizes = []
    for layer in _SECTIONED_LAYERS:
        for sec in iter_all_sections(spec.get(layer, [])):
            sid = sec["id"]
            final_list, dropped = finalize_derived_from(sid, edges_after.get(sid, ()), full_index)
            section_dropped_total += dropped
            section_recomputed[sid] = final_list
            if final_list:
                section_sizes.append(len(final_list))

    section_stats_by_layer = Counter()
    for sid, lst in section_recomputed.items():
        if lst:
            section_stats_by_layer[full_index[sid]["layer"]] += 1

    total_section_edges = sum(section_sizes)
    print("--- apply-derivation: section edges (rule 2 §-targets + Task A own-section targets) ---")
    for layer in _SECTIONED_LAYERS:
        print(f"  sections with non-empty derived_from ({layer}): {section_stats_by_layer.get(layer, 0)}")
    print(f"  total section edges: {total_section_edges}")
    print(f"  dropped by rule 3 among section edges (self-id / same-or-later layer): {section_dropped_total}")

    # Effective upstream reach (LAYERING.md SS1.1 "辿るとき"): a statement's
    # own recomputed derived_from union every ancestor section's recomputed
    # derived_from -- computed here for the report only, never stored.
    reach_stats_by_layer = Counter()
    for layer in _SECTIONED_LAYERS:
        ancestor_map = build_ancestor_section_map(spec, layer)
        for it in iter_leaf_items_tree(spec.get(layer, [])):
            iid = it["id"]
            effective = set(recomputed.get(iid, ()))
            for anc in ancestor_map.get(iid, ()):
                effective.update(section_recomputed.get(anc, ()))
            if effective:
                reach_stats_by_layer[layer] += 1

    print("--- apply-derivation: effective upstream reach (own union ancestor sections', computed not stored) ---")
    for layer in _SECTIONED_LAYERS:
        print(f"  statements with non-empty effective reach ({layer}): {reach_stats_by_layer.get(layer, 0)}")

    if not getattr(args, "write", False):
        print("(report-only: pass --write to overwrite specification.json's derived_from with the recomputed set)")
        print("(--write now also persists section-node derived_from -- see the section-edges block above)")
        total_edges = sum(sizes)
        mean_size = (total_edges / len(sizes)) if sizes else 0.0
        max_size = max(sizes) if sizes else 0
        print("--- apply-derivation summary (statement edges) ---")
        for layer in ("request", "require", "spec", "detailed_spec", "basic_design", "design"):
            print(f"  statements with non-empty derived_from ({layer}): {stats_by_layer.get(layer, 0)}")
        print(f"  total edges: {total_edges}")
        print(f"  mean derived_from size (non-empty statements only): {mean_size:.2f}")
        print(f"  max derived_from size: {max_size}")
        print(f"  statements with cites but empty derived_from (unresolved): {empty_after_cite}")
        print(f"  dropped by rule 3 (self-id / same-or-later layer): {dropped_total}")
        return 0

    for it in iter_all_statements(spec):
        iid = it["id"]
        if iid in recomputed:
            it["derived_from"] = recomputed[iid]

    # 2026-09-05: --write now persists section derived_from too (it was
    # computed above into section_recomputed but never written before --
    # Owner-flagged gap: specification.json had 0 sections with
    # derived_from despite the report counting hundreds of section
    # edges). Every section id in section_recomputed is written whether
    # its list ended up empty or not, mirroring how a statement's
    # "derived_from" key is always present (never omitted) once frozen.
    for layer in _SECTIONED_LAYERS:
        for sec in iter_all_sections(spec.get(layer, [])):
            if sec["id"] in section_recomputed:
                sec["derived_from"] = section_recomputed[sec["id"]]

    bad_refs = [
        (it["id"], did)
        for it in iter_all_statements(spec)
        for did in it.get("derived_from", [])
        if did not in full_index
    ] + [
        (sec["id"], did)
        for layer in _SECTIONED_LAYERS
        for sec in iter_all_sections(spec.get(layer, []))
        for did in sec.get("derived_from", [])
        if did not in full_index
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

    print("--- apply-derivation summary (statement edges) ---")
    for layer in ("request", "require", "spec", "detailed_spec", "basic_design", "design"):
        print(f"  statements with non-empty derived_from ({layer}): {stats_by_layer.get(layer, 0)}")
    print(f"  total edges: {total_edges}")
    print(f"  mean derived_from size (non-empty statements only): {mean_size:.2f}")
    print(f"  max derived_from size: {max_size}")
    print(f"  statements with cites but empty derived_from (unresolved): {empty_after_cite}")
    print(f"  dropped by rule 3 (self-id / same-or-later layer): {dropped_total}")
    print(f"wrote {spec_json_path(root)} -- schema OK (statement and section derived_from both persisted)")
    return 0


# ----------------------------------------------------------------- fix-ids --
# 2026-09-05 (section-node model): replaces the old `freeze`. Persists ids
# only -- both statement ids (as before) AND, new here, section ids (one
# per heading per layer it produces a node in, since a document section
# can now be split across layers by a relayer "layer" override).
# derived_from is deliberately NOT written here (team-lead's spec): it
# keeps being computed by `apply-derivation` from cites/trace-tables.json
# and only ever lands in specification.json, never in a fragment -- a
# fragment's job is identity (id), not derivation (that stays a
# recomputed-every-time report/write, not a stored fact about the source).

def collect_section_ids_by_heading(output: dict) -> dict:
    """(doc, heading number) -> {layer: section_id}, from every section
    node in a freshly built `output` -- one entry per (doc, number) that
    produced a node in one or more layers this build. Used by fix-ids to
    write each heading's id(s) back into its fragment's headings[]
    entry -- a heading whose number is split across layers (its
    statements now live in more than one) gets more than one key in its
    map, e.g. {"spec": "SPEC-S001", "detailed_spec": "DS-S010"}."""
    result: dict = {}
    for layer in _SECTIONED_LAYERS:
        for sec in iter_all_sections(output.get(layer, [])):
            doc = sec["source"]["doc"]
            num = heading_number_token(sec["source"]["heading"])
            if num is None:
                continue
            result.setdefault((doc, num), {})[layer] = sec["id"]
    return result


def log_retired_ids(root: Path, retired: list, reason: str = None) -> None:
    """Append (old_id, new_id) pairs to docs/canonical/relations/
    retired-ids.json (created if absent) -- a durable, cumulative ledger
    (not overwritten each run) so a reference to a retired id can still be
    traced to what it became, across every fix-ids/retire-id run, not
    just the latest. `reason`, if given, is stamped on every entry this
    call appends (retire-id's "manual retire"); omitted for fix-ids'
    automatic layer-move retirements, matching their original shape."""
    log_path = root / "relations" / "retired-ids.json"
    existing = read_json(log_path) if log_path.exists() else []
    for old_id, new_id in retired:
        entry = {"old_id": old_id, "new_id": new_id}
        if reason:
            entry["reason"] = reason
        existing.append(entry)
    log_path.parent.mkdir(parents=True, exist_ok=True)
    log_path.write_text(json.dumps(existing, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")


# --------------------------------------------------------------- retire-id --

def next_fresh_id(layer: str, reserved_ids: set) -> str:
    """The next unused statement id for `layer`, consulting reserved_ids
    exactly like assign_ids does (2026-09-05) -- used by retire-id to mint
    a replacement id outside the normal per-layer build pass, for a
    statement id that must be force-retired even though it currently
    matches its own layer's prefix (so id_belongs_to_layer would not have
    flagged it on its own -- e.g. an id that was accidentally assigned
    twice before the reserved-id fix landed)."""
    prefix = LEAF_PREFIX[layer]
    max_numbered = 0
    for rid in reserved_ids:
        if _prefix_matches(rid, prefix, layer):
            m = _ID_NUM_TRAIL_RE.search(rid)
            if m:
                max_numbered = max(max_numbered, int(m.group(1)))
    counter = max_numbered + 1
    new_id = f"{prefix}-{counter:03d}"
    while new_id in reserved_ids:
        counter += 1
        new_id = f"{prefix}-{counter:03d}"
    return new_id


def cmd_retire_id(args) -> int:
    root = Path(args.root)
    try:
        _output, item_objects, dups, fragments, _retired = build_in_memory(root)
    except ValueError as exc:
        print(f"error: {exc}", file=sys.stderr)
        return 1
    if dups:
        for d in sorted(set(dups)):
            print(f"error: duplicate stored id {d!r} used by more than one item/area", file=sys.stderr)
        return 1

    reserved_ids = build_reserved_ids(fragments, root)

    item_to_path: dict = {}
    for path, frag in fragments:
        for it in iter_all_fragment_items(frag):
            item_to_path[id(it)] = path

    retired_pairs = []
    touched_paths = set()
    for old_id in args.id:
        item_obj = item_objects.get(old_id)
        if item_obj is None:
            print(f"error: {old_id!r} is not a current statement id (fix-ids/build it first, or it's a section id -- retire-id only handles statement ids)", file=sys.stderr)
            return 1
        layer = None
        for candidate_layer in LAYERS:
            if id_belongs_to_layer(old_id, candidate_layer):
                layer = candidate_layer
                break
        if layer is None:
            print(f"error: {old_id!r} doesn't match any known layer's id prefix", file=sys.stderr)
            return 1
        new_id = next_fresh_id(layer, reserved_ids)
        reserved_ids.add(new_id)
        reserved_ids.add(old_id)
        item_obj["id"] = new_id
        item_obj.pop("_id", None)
        item_obj.pop("_native_area", None)
        item_obj.pop("keep_id", None)
        retired_pairs.append((old_id, new_id))
        touched_paths.add(item_to_path[id(item_obj)])

    for path, frag in fragments:
        if path in touched_paths:
            text = json.dumps(frag, ensure_ascii=False, indent=2) + "\n"
            path.write_text(text, encoding="utf-8")

    if retired_pairs:
        log_retired_ids(root, retired_pairs, reason="manual retire")

    print("--- retire-id ---")
    for old_id, new_id in retired_pairs:
        print(f"  {old_id} -> {new_id}")
    print(f"  fragment files rewritten: {len(touched_paths)}")
    return 0


def cmd_fix_ids(args) -> int:
    root = Path(args.root)
    try:
        output, item_objects, dups, fragments, retired = build_in_memory(root)
    except ValueError as exc:
        print(f"error: {exc}", file=sys.stderr)
        return 1
    if dups:
        for d in sorted(set(dups)):
            print(f"error: duplicate stored id {d!r} used by more than one item/area", file=sys.stderr)
        return 1

    for item_obj in item_objects.values():
        item_obj["id"] = item_obj["_id"]
        item_obj.pop("_id", None)
        item_obj.pop("_native_area", None)
        item_obj.pop("keep_id", None)  # promoted into "id"; the old mechanism is now redundant
        # NOTE: an item's "layer" override (if any) is NOT removed here --
        # it is a permanent routing directive build_in_memory reads on
        # every run, independent of the assigned id; stripping it would
        # silently un-relayer the item on the next build. derived_from is
        # NOT written here (team-lead's spec) -- it stays apply-
        # derivation's job, computed fresh from cites/trace-tables.json,
        # never a stored fact in a fragment.

    section_ids_by_heading = collect_section_ids_by_heading(output)
    stamped_headings = 0
    for _path, frag in fragments:
        doc = frag.get("doc")
        for h in frag.get("headings", []):
            ids = section_ids_by_heading.get((doc, h["number"]))
            if ids:
                h["ids"] = ids
                stamped_headings += 1

    for path, frag in fragments:
        text = json.dumps(frag, ensure_ascii=False, indent=2) + "\n"
        path.write_text(text, encoding="utf-8")

    if retired:
        log_retired_ids(root, retired)

    print("--- fix-ids ---")
    print(f"  items stamped: {len(item_objects)}")
    print(f"  heading entries stamped with a section id map: {stamped_headings}")
    print(f"  fragment files rewritten: {len(fragments)}")
    if retired:
        print(f"  stale ids retired (logged to {root / 'relations' / 'retired-ids.json'}):")
        for old_id, new_id in retired:
            print(f"    {old_id} -> {new_id}")
    else:
        print("  stale ids retired: 0")
    return 0


# ----------------------------------------------------------------- relayer --

_RELAYER_KEY_DOC_LINES_RE = re.compile(r"^(.*):([0-9]+)-([0-9]+)$")


def strip_build_bookkeeping(item_objects: dict) -> None:
    """Undo build_in_memory's transient mutations (_id/_native_area on
    items) before writing a fragment back to disk from outside `freeze`
    (i.e. from `relayer apply`). 2026-09-05: areas no longer carry any
    build-time bookkeeping of their own (section ids are computed fresh
    every build, never stored on the native area object)."""
    for it in item_objects.values():
        it.pop("_id", None)
        it.pop("_native_area", None)


def cmd_relayer_apply(args) -> int:
    root = Path(args.root)
    mapping_paths = [Path(p) for p in args.mapping]

    # 2026-09-05 (found the hard way): applying several mapping files one
    # invocation at a time is wrong -- each write shifts statement ids
    # (moved items leave a layer, changing that layer's id sequence), so
    # a later file resolves its own ids against an already-renumbered
    # build. Every key across every given file must resolve against ONE
    # baseline (the fragments as they are right now), so all files are
    # loaded and merged BEFORE the single build_in_memory call below.
    merged: dict = {}
    conflicts = []  # (key, [(path, entry), (path, entry), ...])
    for mapping_path in mapping_paths:
        mapping = read_json(mapping_path)
        if not isinstance(mapping, dict):
            print(f"error: {mapping_path} must be a JSON object", file=sys.stderr)
            return 1
        for key, entry in mapping.items():
            if key in merged and merged[key][1] != entry:
                conflicts.append((key, [merged[key], (mapping_path, entry)]))
                continue
            merged[key] = (mapping_path, entry)

    if conflicts:
        print(f"error: {len(conflicts)} key(s) appear in more than one mapping file with different entries -- fix the inputs, nothing was applied:", file=sys.stderr)
        for key, occurrences in conflicts:
            for path, entry in occurrences:
                print(f"    {key} in {path}: {entry}", file=sys.stderr)
        return 1

    try:
        _output, item_objects, dups, fragments, _retired = build_in_memory(root)
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
    prefix_mismatches = []  # (key, expected_prefix, actual_prefix)
    touched_paths = set()

    for key, (mapping_path, entry) in merged.items():
        if not isinstance(entry, dict) or "layer" not in entry:
            unresolved.append((key, f"mapping entry missing 'layer' (from {mapping_path})"))
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

        prefix = entry.get("statement_prefix")
        if prefix and not item_obj.get("statement", "").startswith(prefix[:30]):
            prefix_mismatches.append((key, prefix[:30], item_obj.get("statement", "")[:30]))
            continue

        old_layer = item_obj.get("layer")
        item_obj["layer"] = new_layer
        applied.append((key, old_layer, new_layer))
        touched_paths.add(item_to_path[id(item_obj)])
        if entry.get("confidence") == "low":
            low_confidence.append((key, entry.get("reason", "")))
        if entry.get("code_like"):
            code_like.append((key, entry.get("reason", "")))

    strip_build_bookkeeping(item_objects)

    for path, frag in fragments:
        if path in touched_paths:
            text = json.dumps(frag, ensure_ascii=False, indent=2) + "\n"
            path.write_text(text, encoding="utf-8")

    print("--- relayer apply ---")
    print(f"  mapping files: {len(mapping_paths)}")
    print(f"  entries merged: {len(merged)}")
    print(f"  applied: {len(applied)}")
    print(f"  unresolved: {len(unresolved)}")
    for key, why in unresolved:
        print(f"    {key}: {why}")
    if prefix_mismatches:
        print(f"  skipped -- statement_prefix mismatch ({len(prefix_mismatches)}):")
        for key, expected, actual in prefix_mismatches:
            print(f"    {key}: expected {expected!r}, found {actual!r}")
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


# --------------------------------------------------------- harvest-root --
# 2026-09-05 (root layer): LAYERING.md SS1.1 "根の層" -- 要件定義's own
# derivation table declares itself 「第I部（根 → 要求）」and names Issue
# #11's F1-F12 freeze items and Owner rulings in its upstream column, so
# those are nodes above request, not request itself. harvest-root reads
# Issue #11's body (the freeze ledger) once via `gh issue view`, and the
# derivation table's raw upstream cells once via md, and writes
# fragments/root.json -- reproducible, like every other harvest command.

_F_ITEM_HEADING_RE = re.compile(r"^###\s+(F([0-9]+))\.\s*(.+)$")


def parse_f_items(issue_body: str) -> list:
    """-> [{"n": int, "heading": "F<n>", "text": "<heading line>\\n<its
    bullets>, verbatim"}, ...], one per "### F<n>. ..." section of Issue
    #11's body, closed by the next "### F..." heading or any "## "/"---"
    line (the ledger's own section breaks) -- never by a fixed line
    count, since the ledger's prose is free-form between headings."""
    items = []
    current = None
    for line in issue_body.splitlines():
        m = _F_ITEM_HEADING_RE.match(line)
        if m:
            if current:
                items.append(current)
            current = {"n": int(m.group(2)), "heading": f"F{m.group(2)}", "lines": [line]}
            continue
        if current is not None:
            if line.strip() == "---" or line.startswith("## "):
                items.append(current)
                current = None
            else:
                current["lines"].append(line)
    if current:
        items.append(current)
    for it in items:
        it["text"] = "\n".join(it["lines"]).rstrip()
    return items


def fetch_issue_11_body(issue_body_file) -> str:
    """The freeze ledger's body text -- read from `issue_body_file` if
    given (for synthetic testing, where there is no real Issue #11 to
    query), else fetched live via `gh issue view 11 --json body`."""
    if issue_body_file:
        return Path(issue_body_file).read_text(encoding="utf-8")
    result = subprocess.run(
        ["gh", "issue", "view", "11", "--json", "body", "-q", ".body"],
        capture_output=True, text=True, encoding="utf-8",
    )
    if result.returncode != 0:
        print(f"error: `gh issue view 11` failed: {result.stderr.strip()}", file=sys.stderr)
        sys.exit(1)
    return result.stdout


def collect_root_candidate_tokens(root: Path, repo_root: Path) -> tuple:
    """-> (ruling_tokens: dict[normalized_token -> (doc, first_row_line)],
    bare_11_count: int). Scans the same 要求→要件 derivation table rows
    harvest_derivation_table_rows does (find_derivation_table_ranges,
    keep_for_derivation), but only to find upstream tokens that are
    neither an id/section/selfname reference (classify_table_ref's other
    three kinds -- those are request/require statements or sections, not
    root candidates) nor a bare F<n> (an alias for an Issue #11 item,
    handled from the issue body itself, not duplicated here) nor a bare
    "#11" (maps to nothing, per the task -- counted, not made into an
    item). Each surviving token keeps only its FIRST row/line -- the same
    ruling is commonly cited by many rows."""
    ruling_first: dict = {}
    bare_11 = 0
    for doc, start, end in find_derivation_table_ranges(root):
        md_lines = (repo_root / doc).read_text(encoding="utf-8").splitlines()
        for line_no, cells in parse_md_table_rows(md_lines, start, end):
            if len(cells) != 4:
                continue
            upstream = cells[0]
            for tok in split_outside_parens(upstream):
                kind, _key, _matched = classify_table_ref(tok)
                if kind != "unknown":
                    continue
                norm = normalize_root_token(tok)
                if norm == "#11":
                    bare_11 += 1
                    continue
                if re.fullmatch(r"F[0-9]+", norm):
                    continue
                ruling_first.setdefault(norm, (doc, line_no))
    return ruling_first, bare_11


def cmd_harvest_root(args) -> int:
    root = Path(args.root)
    repo_root = Path(args.repo_root)

    issue_body = fetch_issue_11_body(getattr(args, "issue_body_file", None))
    f_items = parse_f_items(issue_body)
    if len(f_items) != 12:
        print(f"warning: expected 12 F-items (F1-F12) in Issue #11's body, found {len(f_items)}", file=sys.stderr)

    ruling_first, bare_11 = collect_root_candidate_tokens(root, repo_root)

    items = []
    for fi in sorted(f_items, key=lambda x: x["n"]):
        items.append({
            "statement": fi["text"],
            "source": {"doc": "github:YmSaki/SpecTracer/issues/11", "heading": fi["heading"], "lines": [fi["n"], fi["n"]]},
        })
    for tok in sorted(ruling_first):
        doc, line_no = ruling_first[tok]
        items.append({
            "statement": tok,
            "source": {"doc": doc, "heading": tok, "lines": [line_no, line_no]},
        })

    frag = {"doc": "github:YmSaki/SpecTracer/issues/11", "layer": "root", "items": items}
    out_path = fragments_dir(root) / "root.json"
    out_path.write_text(json.dumps(frag, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")

    print("--- harvest-root ---")
    print(f"  F-items: {len(f_items)}")
    print(f"  distinct non-F upstream tokens (ruling/other): {len(ruling_first)}")
    for tok in sorted(ruling_first):
        print(f"    {tok!r}")
    print(f"  bare \"#11\" occurrences (maps to nothing, not an item): {bare_11}")
    print(f"  wrote {out_path} ({len(items)} root items)")
    return 0


# ------------------------------------------------------------------ cli --

def cmd_all(args) -> int:
    """build -> apply-derivation --write -> coverage -> export, stopping
    at the first failure. 2026-09-05: a bare `all` used to leave every
    derived_from empty (build never computes it; apply-derivation wasn't
    part of the chain) -- that silent gap is now closed. `build` alone is
    unchanged (still just structure + ids, no edges) for anyone who wants
    that without also recomputing/writing derived_from."""
    rc = cmd_build(args)
    if rc != 0:
        return rc
    args.write = True
    rc = cmd_apply_derivation(args)
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
    add_repo_root_arg(p_build)
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

    p_hh = sub.add_parser("harvest-headings", help="write every fragment's numbered-heading list into its 'headings' field, from md (run once; build then never touches md)")
    add_root_arg(p_hh)
    add_repo_root_arg(p_hh)
    p_hh.set_defaults(func=cmd_harvest_headings)

    p_htt = sub.add_parser("harvest-trace-tables", help="parse the 要求→要件 table and the 4 traceability appendices from md into docs/canonical/relations/trace-tables.json (run once; apply-derivation then never touches md)")
    add_root_arg(p_htt)
    add_repo_root_arg(p_htt)
    p_htt.set_defaults(func=cmd_harvest_trace_tables)

    p_hr = sub.add_parser("harvest-root", help="write fragments/root.json from Issue #11's freeze ledger (F1-F12) and the derivation table's non-F upstream tokens (Owner rulings)")
    add_root_arg(p_hr)
    add_repo_root_arg(p_hr)
    p_hr.add_argument("--issue-body-file", help="read Issue #11's body from this file instead of `gh issue view 11` (for synthetic testing)")
    p_hr.set_defaults(func=cmd_harvest_root)

    p_deriv = sub.add_parser("derivation-candidates", help="CONVERSION.md SS6 steps 1-2: mechanical derivation candidate list")
    add_root_arg(p_deriv)
    add_repo_root_arg(p_deriv)
    p_deriv.set_defaults(func=cmd_derivation_candidates)

    p_applyderiv = sub.add_parser("apply-derivation", help="CONVERSION.md SS6: report (or, with --write, apply) the recomputed derived_from vs the stored one")
    add_root_arg(p_applyderiv)
    add_repo_root_arg(p_applyderiv)
    p_applyderiv.add_argument("--write", action="store_true", help="overwrite specification.json's derived_from with the recomputed set (default: report only)")
    p_applyderiv.set_defaults(func=cmd_apply_derivation)

    p_fix_ids = sub.add_parser("fix-ids", help="write item ids and heading section-id maps into fragments in place (2026-09-05, replaces the old `freeze` -- does not write derived_from)")
    add_root_arg(p_fix_ids)
    p_fix_ids.set_defaults(func=cmd_fix_ids)

    p_retire = sub.add_parser("retire-id", help="force one or more currently-stored statement ids to a fresh id in the same layer, log the retirement, rewrite the fragment (2026-09-05)")
    add_root_arg(p_retire)
    p_retire.add_argument("id", nargs="+", help="statement id(s) currently stored in a fragment to force-retire")
    p_retire.set_defaults(func=cmd_retire_id)

    p_relayer = sub.add_parser("relayer", help="apply or report per-item layer overrides (spec/basic_design/design split)")
    relayer_sub = p_relayer.add_subparsers(dest="relayer_command", required=True)

    p_relayer_apply = relayer_sub.add_parser("apply", help="write a layer override into fragment items from one or more mapping files")
    add_root_arg(p_relayer_apply)
    p_relayer_apply.add_argument("mapping", nargs="+", help="path(s) to mapping JSON file(s): {'<id or doc:start-end>': {'layer': ..., 'reason': ..., 'confidence': 'high'|'low', 'code_like': bool, 'statement_prefix': ... (optional)}}. Given more than one, ALL keys across ALL files resolve against the SAME baseline build (fragments as they are now) before any write -- passing them separately in multiple invocations is wrong, since the first invocation's write shifts ids for the next (2026-09-05, found the hard way).")
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
