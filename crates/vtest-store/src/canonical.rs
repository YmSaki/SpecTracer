//! Canonical v0.1 record storage, for the types defined in `vtest-model`.
//! Kept separate from `records.rs`'s predecessor Req/Spec-model types so
//! neither module's exports collide with the other's.
//!
//! The upstream document model (`DocumentFile`) is JSON — BD-319/BD-320:
//! "上流文書のファイル形式は JSON とし、その他のレコードのファイル形式は
//! すべて YAML とする" — while VO and Relation stay YAML through
//! `yaml_serde`. `yaml_serde` is used (rather than the hand-rolled
//! scalar/list helpers `records.rs` uses for predecessor types) because
//! those helpers silently mis-parse inline comments and flow-mappings
//! (both of which the canonical VO example — DES-117 — uses).

use crate::{read_text, write_atomic, StoreError, VerifyLayout};
use vtest_model::{
    DerivesFrom, Diagnostic, DocumentFile, DocumentId, Layer, NodeSource, RootNode, SectionNode,
    SentenceNode, VoRecord,
};

/// Known keys for one `derives_from[]` entry on a VO record (DS-1638).
/// Distinct from the upstream document model's own `derives_from` entries,
/// which are bare ids with no such shape (DS-1594, DS-1595) — see
/// `vtest_model::document`'s module doc comment.
const DERIVES_FROM_KEYS: &[&str] = &["doc", "anchor", "note"];

/// Known top-level keys for a canonical VO record (DES-S037 / DES-117 area).
/// `status` is listed as *known* here even though `VoRecord` has no such
/// field: its presence gets its own, more specific W-STORE-001 diagnostic
/// (DS-405) — listing it here stops the same key from also being rejected as
/// a generic unknown field.
const VO_KEYS: &[&str] = &[
    "id",
    "parent",
    "derives_from",
    "claim",
    "dimensions",
    "coverage_policy",
    "combinations",
    "representative_cases",
    "created",
    "updated",
    "status",
];

/// Known keys for one `dimensions[]` entry.
const DIMENSION_KEYS: &[&str] = &["name", "partitions"];

fn schema_mismatch(location: impl Into<String>, detail: impl Into<String>) -> StoreError {
    StoreError::SchemaMismatch {
        code: "E-SCAN-010",
        location: location.into(),
        detail: detail.into(),
    }
}

/// Scans a YAML mapping for keys outside `known`, failing closed on the
/// first one found. DS-1645 (E-SCAN-010): "schema不一致（宣言されていない
/// 余剰 field を含む）" is an error, not a warning — this replaces the
/// retired `DS-376` "warn and continue" behavior (see
/// `docs/canonical/relations/retired-ids.json`). `prefix` is prepended to
/// the reported key so a nested unknown key (e.g. inside `derives_from[0]`)
/// reads distinctly from a top-level one. Does nothing if `value` isn't a
/// mapping — a type mismatch there is instead caught, fail-closed, by the
/// `from_value` deserialize `VoRecord`'s and `RelationRecord`'s callers run
/// alongside this scan.
///
/// A mapping key that is *not* a YAML string (an integer, bool, null, or
/// nested collection key) is rejected here rather than skipped: every
/// `known` list is entirely strings, so such a key can never be a
/// recognized field, and for `ApprovalRecord::from_yaml` and
/// `read_evidence` — the two callers whose own record type has no
/// `#[serde(deny_unknown_fields)]`/`from_value` pass behind this scan (see
/// their call sites) — this scan is their *only* defense against a surplus
/// field. Silently skipping a non-string key here would let a config such
/// as `2026: unlimited` sit in an Approval record unrejected, undermining
/// exactly the fail-closed guarantee DS-1645 states.
///
/// `pub(crate)` so `records.rs`'s `RelationRecord::from_yaml` — a reader
/// that lives outside this module for historical reasons (see that file's
/// module doc comment) — can apply the same DS-1645 rule instead of
/// re-implementing the scan.
pub(crate) fn reject_unknown_fields(
    value: &yaml_serde::Value,
    known: &[&str],
    prefix: &str,
) -> Result<(), StoreError> {
    let Some(mapping) = value.as_mapping() else {
        return Ok(());
    };
    for (key, _) in mapping.iter() {
        let Some(key_str) = key.as_str() else {
            return Err(schema_mismatch(
                format!("{prefix}{key:?}"),
                "non-string mapping key is not part of the record schema (DS-1645)",
            ));
        };
        if !known.contains(&key_str) {
            return Err(schema_mismatch(
                format!("{prefix}{key_str}"),
                "unknown field is not part of the record schema (DS-1645)",
            ));
        }
    }
    Ok(())
}

/// Extends the unknown-field rejection into each `derives_from[]` entry —
/// the nested shape VO records use (DS-1638).
fn reject_derives_from_unknown_fields(value: &yaml_serde::Value) -> Result<(), StoreError> {
    let Some(sequence) = value
        .get("derives_from")
        .and_then(yaml_serde::Value::as_sequence)
    else {
        return Ok(());
    };
    for (index, entry) in sequence.iter().enumerate() {
        reject_unknown_fields(entry, DERIVES_FROM_KEYS, &format!("derives_from[{index}]."))?;
    }
    Ok(())
}

/// Extends the unknown-field rejection into each VO `dimensions[]` entry.
/// `combinations[]` is deliberately not scanned the same way: each entry's
/// keys are the *dimension names themselves* (a dynamic, record-specific
/// vocabulary), not a fixed schema — whether a combination covers exactly
/// the declared dimensions is E-SCAN-017, a chain_integrity/scan-time
/// concern this record-level reader does not have the dimension set
/// resolved enough to evaluate (the same record-vs-scan layer split
/// `require_at_least_one_derives_from` below documents for E-SCAN-012).
fn reject_dimensions_unknown_fields(value: &yaml_serde::Value) -> Result<(), StoreError> {
    let Some(sequence) = value
        .get("dimensions")
        .and_then(yaml_serde::Value::as_sequence)
    else {
        return Ok(());
    };
    for (index, entry) in sequence.iter().enumerate() {
        reject_unknown_fields(entry, DIMENSION_KEYS, &format!("dimensions[{index}]."))?;
    }
    Ok(())
}

// ---------------------------------------------------------------------
// Upstream document model (`.verify/doc/<name>.json`)
// ---------------------------------------------------------------------
//
// ROOT-047/ROOT-048 (Owner ruling): `specification.json` *is* the
// `.verify/doc/` document model, bundled one file per layer array; on
// registration each document becomes its own file of the same shape
// (BD-319, BD-320, BD-321, BD-323, DES-586). BD-322/BD-330/DES-585: the
// file carries no field identifying which document it is — the filename
// itself (chosen by the document's author, never a machine-generated
// identifier) is the identity, so there is no id/file-name agreement check
// here (contrast the predecessor's per-record `id` field, and VO's
// `VoRecord.id` below).
//
// The checks below are exactly the ones DES-586's own reasoning names as
// following from `specification.schema.json` once a document file is held
// to the same schema as the bundle: `schema_version` is a `const` (schema
// `properties.schema_version`), every node id matches the shared
// `$defs/id` union pattern, every layer's nodes carry that layer's id
// prefix (DS-1658), every `derives_from` entry *also* matches the shared
// `$defs/id` pattern (schema `derivedItem.derives_from`/`section.derives_from`,
// both `"items": { "$ref": "#/$defs/id" }`) and has no duplicates within
// the list (schema `uniqueItems`), `statement`/`title`/`source.doc`/
// `source.heading` are non-empty (schema `minLength: 1`), `source.lines`
// entries are `>= 1` (schema `minimum: 1`), and no optional field is
// present as JSON `null`
// (the schema's `{"type": "string"}"`/`{"type": "array", ...}` on an
// optional property rejects `null` when the key is present — `null` is not
// the same as absent, but `Option<T>: Deserialize` accepts it as `None`
// regardless, so this needs an explicit pre-pass over the raw JSON `Value`
// PR20's `DocumentFile`/`SectionNode`/etc. cannot express on their own).
// Any violation is DS-1645/E-SCAN-010 (schema mismatch) per DES-586's own
// citation of DS-1645 for the surplus-field case; this crate does not
// mint a second diagnostic code for the other schema violations, matching
// DS-1658's own "新しい診断コードは作らない".

/// `specification.schema.json`'s `properties.schema_version` — `{ "type":
/// "string", "const": "0.1" }`.
const DOCUMENT_SCHEMA_VERSION: &str = "0.1";

/// Optional-field keys the upstream document schema types as a required
/// shape (`string`/`array`) whenever the key is present — so JSON `null`
/// must be rejected for these keys specifically, everywhere they occur in a
/// document file. Scoped to this module's document-file parsing only: a VO
/// record's `parent: null` (DES-117) is a different schema entirely and is
/// unaffected.
const DOCUMENT_NULL_DISALLOWED_KEYS: &[&str] =
    &["description", "cites", "derives_from", "sections", "items"];

/// Walks every object in a parsed document-file JSON value and rejects any
/// of `DOCUMENT_NULL_DISALLOWED_KEYS` present with an explicit `null`.
fn reject_document_json_nulls(value: &serde_json::Value) -> Result<(), StoreError> {
    match value {
        serde_json::Value::Object(map) => {
            for (key, nested) in map {
                if DOCUMENT_NULL_DISALLOWED_KEYS.contains(&key.as_str()) && nested.is_null() {
                    return Err(schema_mismatch(
                        key.as_str(),
                        "must not be JSON null when present; omit the key instead (schema types it as string/array when present)",
                    ));
                }
                reject_document_json_nulls(nested)?;
            }
            Ok(())
        }
        serde_json::Value::Array(items) => {
            for item in items {
                reject_document_json_nulls(item)?;
            }
            Ok(())
        }
        _ => Ok(()),
    }
}

/// Matches `specification.schema.json`'s `$defs/id` pattern:
/// `^(ROOT-[0-9]{3,}|R-[0-9]+|P-[0-9]{3}|REQ-[0-9]{3,}|REQ-S[0-9]{3,}|
/// SPEC-[0-9]{3,}|SPEC-S[0-9]{3,}|DS-[0-9]{3,}|DS-S[0-9]{3,}|
/// BD-[0-9]{3,}|BD-S[0-9]{3,}|DES-[0-9]{3,}|DES-S[0-9]{3,})$`. Written by
/// hand rather than pulling in a regex crate; the `-S` variant of each
/// prefix is checked before its plain counterpart so `"REQ-S001"` is not
/// consumed by the `"REQ-"` branch first.
fn matches_document_id_pattern(id: &str) -> bool {
    fn digits(rest: &str, min: usize) -> bool {
        !rest.is_empty() && rest.len() >= min && rest.bytes().all(|b| b.is_ascii_digit())
    }
    fn digits_exact(rest: &str, exact: usize) -> bool {
        rest.len() == exact && rest.bytes().all(|b| b.is_ascii_digit())
    }
    if let Some(rest) = id.strip_prefix("ROOT-") {
        return digits(rest, 3);
    }
    if let Some(rest) = id.strip_prefix("R-") {
        return digits(rest, 1);
    }
    if let Some(rest) = id.strip_prefix("P-") {
        return digits_exact(rest, 3);
    }
    if let Some(rest) = id.strip_prefix("REQ-S") {
        return digits(rest, 3);
    }
    if let Some(rest) = id.strip_prefix("REQ-") {
        return digits(rest, 3);
    }
    if let Some(rest) = id.strip_prefix("SPEC-S") {
        return digits(rest, 3);
    }
    if let Some(rest) = id.strip_prefix("SPEC-") {
        return digits(rest, 3);
    }
    if let Some(rest) = id.strip_prefix("DS-S") {
        return digits(rest, 3);
    }
    if let Some(rest) = id.strip_prefix("DS-") {
        return digits(rest, 3);
    }
    if let Some(rest) = id.strip_prefix("BD-S") {
        return digits(rest, 3);
    }
    if let Some(rest) = id.strip_prefix("BD-") {
        return digits(rest, 3);
    }
    if let Some(rest) = id.strip_prefix("DES-S") {
        return digits(rest, 3);
    }
    if let Some(rest) = id.strip_prefix("DES-") {
        return digits(rest, 3);
    }
    false
}

/// Validates one node id: it must match the schema's `$defs/id` pattern
/// *and* its prefix must resolve to `expected_layer` (DS-1658) — the two
/// are independent checks (a syntactically valid id can still sit in the
/// wrong layer's array).
fn validate_node_id(id: &str, expected_layer: Layer) -> Result<(), StoreError> {
    if !matches_document_id_pattern(id) {
        return Err(schema_mismatch(
            id,
            "id does not match the schema's $defs/id pattern",
        ));
    }
    match Layer::from_id_prefix(id) {
        Some(layer) if layer == expected_layer => Ok(()),
        _ => Err(schema_mismatch(
            id,
            "id's prefix does not match the layer array it was placed in (DS-1658)",
        )),
    }
}

fn validate_node_source(source: &NodeSource, node_id: &str) -> Result<(), StoreError> {
    if source.doc.is_empty() {
        return Err(schema_mismatch(node_id, "source.doc must not be empty"));
    }
    if source.heading.is_empty() {
        return Err(schema_mismatch(node_id, "source.heading must not be empty"));
    }
    if source.lines[0] < 1 || source.lines[1] < 1 {
        return Err(schema_mismatch(
            node_id,
            "source.lines entries must each be >= 1",
        ));
    }
    Ok(())
}

/// Validates one `derives_from[]` list against the schema's `$defs/id`
/// pattern (specification.schema.json:52/:71, `"items": { "$ref":
/// "#/$defs/id" }`) and rejects duplicates (schema `uniqueItems`).
///
/// Deliberately checks the pattern only, *not* `Layer::from_id_prefix`
/// agreement (DS-1658): an upstream document node legitimately derives from
/// a node in a *different* layer (e.g. a `DS-` node deriving from a
/// `SPEC-` node) — DS-1658's layer/prefix agreement applies only to a
/// node's *own* id sitting in its *own* layer's array (`validate_node_id`),
/// never to what that node cites as its upstream. See
/// `document_derives_from_entry_across_layers_is_accepted` (below) for the
/// positive case this scoping preserves.
fn validate_derives_from_unique(ids: &[DocumentId], node_id: &str) -> Result<(), StoreError> {
    let mut seen = std::collections::BTreeSet::new();
    for id in ids {
        if !matches_document_id_pattern(id.as_str()) {
            return Err(schema_mismatch(
                node_id,
                format!(
                    "derives_from entry `{}` does not match the schema's $defs/id pattern",
                    id.as_str()
                ),
            ));
        }
        if !seen.insert(id.as_str()) {
            return Err(schema_mismatch(
                node_id,
                format!("derives_from duplicates `{}`", id.as_str()),
            ));
        }
    }
    Ok(())
}

fn validate_root_node(node: &RootNode) -> Result<(), StoreError> {
    validate_node_id(node.id.as_str(), Layer::Root)?;
    if node.statement.is_empty() {
        return Err(schema_mismatch(
            node.id.as_str(),
            "statement must not be empty",
        ));
    }
    validate_node_source(&node.source, node.id.as_str())
}

fn validate_sentence_node(node: &SentenceNode, layer: Layer) -> Result<(), StoreError> {
    validate_node_id(node.id.as_str(), layer)?;
    if node.statement.is_empty() {
        return Err(schema_mismatch(
            node.id.as_str(),
            "statement must not be empty",
        ));
    }
    validate_node_source(&node.source, node.id.as_str())?;
    validate_derives_from_unique(&node.derives_from, node.id.as_str())?;
    if let Some(cites) = &node.cites {
        for citation in cites {
            if citation.is_empty() {
                return Err(schema_mismatch(
                    node.id.as_str(),
                    "cites entries must not be empty",
                ));
            }
        }
    }
    Ok(())
}

fn validate_section_node(section: &SectionNode, layer: Layer) -> Result<(), StoreError> {
    validate_node_id(section.id.as_str(), layer)?;
    if section.title.is_empty() {
        return Err(schema_mismatch(
            section.id.as_str(),
            "title must not be empty",
        ));
    }
    validate_node_source(&section.source, section.id.as_str())?;
    if let Some(derives_from) = &section.derives_from {
        validate_derives_from_unique(derives_from, section.id.as_str())?;
    }
    if let Some(children) = &section.sections {
        for child in children {
            validate_section_node(child, layer)?;
        }
    }
    if let Some(items) = &section.items {
        for item in items {
            validate_sentence_node(item, layer)?;
        }
    }
    Ok(())
}

/// Runs every schema-intrinsic check this module owns against an already
/// well-typed `DocumentFile` (id pattern, DS-1658 layer/prefix agreement,
/// `derives_from` uniqueness, non-empty statement/title/source fields,
/// `source.lines >= 1`, `schema_version` const). Does *not* re-check for
/// JSON `null` on optional fields — that only makes sense against the raw
/// JSON text (see `reject_document_json_nulls`), and by the time a value is
/// a `DocumentFile` that distinction is already gone. Used by both the
/// reader (after parsing) and the writer (before serializing), so the
/// writer cannot emit a file the reader would then reject.
pub fn validate_document_file(file: &DocumentFile) -> Result<(), StoreError> {
    if file.schema_version != DOCUMENT_SCHEMA_VERSION {
        return Err(schema_mismatch(
            "schema_version",
            format!(
                "must be \"{DOCUMENT_SCHEMA_VERSION}\", got \"{}\"",
                file.schema_version
            ),
        ));
    }
    for node in &file.root {
        validate_root_node(node)?;
    }
    for node in &file.request {
        validate_sentence_node(node, Layer::Request)?;
    }
    for section in &file.require {
        validate_section_node(section, Layer::Require)?;
    }
    for section in &file.spec {
        validate_section_node(section, Layer::Spec)?;
    }
    for section in &file.detailed_spec {
        validate_section_node(section, Layer::DetailedSpec)?;
    }
    for section in &file.basic_design {
        validate_section_node(section, Layer::BasicDesign)?;
    }
    for section in &file.design {
        validate_section_node(section, Layer::Design)?;
    }
    Ok(())
}

/// Parses one upstream document file (BD-319/BD-320: JSON). `text` is
/// parsed twice on purpose: once to a `serde_json::Value` for the
/// null-rejection walk (see `DOCUMENT_NULL_DISALLOWED_KEYS`), and once
/// straight to `DocumentFile` so its `#[serde(deny_unknown_fields)]` and
/// serde-derive's own duplicate-key rejection apply — collapsing this into
/// a single `Value -> from_value` pass would lose both (a `serde_json::Value`
/// object silently keeps only the *last* of two duplicate keys, and
/// `from_value` cannot see whether a key was present-with-null or absent
/// once the intermediate `Value` has already merged them).
pub fn document_file_from_json(text: &str) -> Result<DocumentFile, StoreError> {
    let raw: serde_json::Value = serde_json::from_str(text)
        .map_err(|error| schema_mismatch("<document>", format!("invalid JSON: {error}")))?;
    reject_document_json_nulls(&raw)?;

    let file: DocumentFile = serde_json::from_str(text).map_err(|error| {
        schema_mismatch(
            "<document>",
            format!("does not conform to the document file schema: {error}"),
        )
    })?;
    validate_document_file(&file)?;
    Ok(file)
}

/// Serializes a `DocumentFile` to its canonical JSON text (pretty-printed,
/// trailing newline). Byte-for-byte identity with the canonical bundle's
/// own formatting is not required — see
/// `vtest_model::document::tests::canonical_bundle_round_trips_and_matches_node_counts`
/// for the measured, disclosed divergence (key order on a handful of nodes,
/// trailing newline). Refuses to serialize a `DocumentFile` this module's
/// own reader would then reject, the same defensive symmetry
/// `write_vo_record` already applies below.
pub fn document_file_to_json(file: &DocumentFile) -> Result<String, StoreError> {
    validate_document_file(file)?;
    let mut text =
        serde_json::to_string_pretty(file).expect("DocumentFile always serializes to valid JSON");
    text.push('\n');
    Ok(text)
}

/// Reads the upstream document file `<name>.json` from `.verify/doc/`.
/// `name` is the file's own name (BD-330/DES-585: user-chosen, not a
/// machine-generated identifier, and not compared against any field inside
/// the file — the file carries no such field).
pub fn read_document_file(layout: &VerifyLayout, name: &str) -> Result<DocumentFile, StoreError> {
    let path = layout.doc_dir().join(format!("{name}.json"));
    let text = read_text(&path)?;
    document_file_from_json(&text)
}

/// Writes (or overwrites) the upstream document file `<name>.json` to
/// `.verify/doc/`. Mutable in place: BD-062 lists only Relation / decision /
/// approval / Evidence as append-only-only new-file-per-write; the document
/// model (like VO) follows the general one-record-one-file edit model
/// (BD-321).
pub fn write_document_file(
    layout: &VerifyLayout,
    name: &str,
    file: &DocumentFile,
) -> Result<(), StoreError> {
    let path = layout.doc_dir().join(format!("{name}.json"));
    let text = document_file_to_json(file)?;
    write_atomic(&path, &text)
}

// ---------------------------------------------------------------------
// VO records (`.verify/vo/VO-*.yaml`)
// ---------------------------------------------------------------------

/// Serializes a canonical `VoRecord` to its `.verify/vo/VO-*.yaml` shape via
/// `yaml_serde`. Distinct name from `read_vo`/`VoRecord` (records.rs) on
/// purpose: that pair still serves the predecessor store-side `VoRecord`
/// until PR8 retires it, and the two types are not interchangeable.
pub fn vo_record_to_yaml(record: &VoRecord) -> String {
    yaml_serde::to_string(record).expect("VoRecord always serializes to valid YAML")
}

/// Parses a canonical `VoRecord` from its YAML representation, returning any
/// non-fatal diagnostics alongside it. `yaml_serde::from_value` fails closed
/// on a missing required field (`claim`/`created`/`updated`/etc.) or an
/// unrecognized `coverage_policy` value via `VoRecord`'s `Deserialize`
/// derive; this adds the `derives_from` cardinality floor (SPEC-015), plus
/// DS-1645's fail-closed rejection of any field outside the schema.
///
/// This goes through an explicit two-stage parse (text → `Value` → known-key
/// scan → `VoRecord`) because a direct `from_str::<VoRecord>` would silently
/// drop an unrecognized key with no way to observe it happened: serde's
/// derive ignores fields the target struct does not declare, and
/// `vtest_model::VoRecord`/`DerivesFrom`/`Dimension` carry no
/// `#[serde(deny_unknown_fields)]` of their own (out of this PR's scope to
/// add — that would be a `vtest-model` type change). Most unknown keys are
/// therefore rejected here with DS-1645/E-SCAN-010; `status` is the one
/// exception DS-405 names explicitly: "readerは読取り互換fieldとして
/// `status` を受理するが、実効判定とVO subject hashでは無視し、存在自体を
/// W-STORE-001として通知する" — a warning, not a rejection, and `VoRecord`
/// deliberately has no `status` field to receive it (a canonical writer
/// never persists one; adding one would pollute the canonical model and its
/// JSON shape).
pub fn vo_record_from_yaml(
    text: &str,
    fallback_id: &str,
) -> Result<(VoRecord, Vec<Diagnostic>), StoreError> {
    let value: yaml_serde::Value = yaml_serde::from_str(text)
        .map_err(|error| StoreError::InvalidConfig(format!("invalid VO record: {error}")))?;

    let mut diagnostics = Vec::new();
    if value.get("status").is_some() {
        diagnostics.push(Diagnostic::warning(
            "W-STORE-001",
            "VO record has the non-canonical read-compat field `status`; its value is ignored — effective state and the VO subject hash are derived from approvals instead",
        ));
    }
    reject_unknown_fields(&value, VO_KEYS, "")?;
    reject_derives_from_unknown_fields(&value)?;
    reject_dimensions_unknown_fields(&value)?;

    let record: VoRecord = yaml_serde::from_value(value)
        .map_err(|error| StoreError::InvalidConfig(format!("invalid VO record: {error}")))?;
    if record.id.as_str() != fallback_id {
        return Err(StoreError::InvalidConfig(format!(
            "VO id {} does not match file name {fallback_id}",
            record.id.as_str()
        )));
    }
    require_at_least_one_derives_from(&record.derives_from)?;
    Ok((record, diagnostics))
}

/// Reads the canonical VO record `<id>.yaml` from `.verify/vo/`.
pub fn read_vo_record(
    layout: &VerifyLayout,
    id: &str,
) -> Result<(VoRecord, Vec<Diagnostic>), StoreError> {
    let path = layout.vo_dir().join(format!("{id}.yaml"));
    let text = read_text(&path)?;
    vo_record_from_yaml(&text, id)
}

/// Writes (or overwrites) the canonical VO record to `.verify/vo/`. Mutable
/// in place (BD-321). Enforces the same `derives_from` cardinality floor as
/// the reader: a writer that skipped this check could produce a record
/// `read_vo_record` would then reject, which fail-closed reading alone does
/// not prevent.
pub fn write_vo_record(layout: &VerifyLayout, record: &VoRecord) -> Result<(), StoreError> {
    require_at_least_one_derives_from(&record.derives_from)?;
    let path = layout.vo_dir().join(format!("{}.yaml", record.id.as_str()));
    write_atomic(&path, &vo_record_to_yaml(record))
}

/// SPEC-015: "VOは1件以上のdocumentからderives_fromで導出される" — unlike the
/// upstream document model's own `derives_from` (0 or more; an empty list
/// marks a root candidate), a VO's `derives_from` must be non-empty. This
/// checks only cardinality: whether each entry's `doc` resolves to a
/// document that actually exists is E-SCAN-012, a chain_integrity/scan-time
/// concern this record-level reader/writer does not have the document set
/// to evaluate.
fn require_at_least_one_derives_from(derives_from: &[DerivesFrom]) -> Result<(), StoreError> {
    if derives_from.is_empty() {
        return Err(StoreError::InvalidConfig(
            "VO derives_from must have at least one entry".to_owned(),
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;
    use vtest_model::{CoveragePolicy, Dimension, VoId};

    // -------------------------------------------------------------
    // Document file tests
    // -------------------------------------------------------------

    fn sample_source() -> NodeSource {
        NodeSource {
            doc: "docs/spec.md".to_owned(),
            heading: "1".to_owned(),
            lines: [1, 1],
        }
    }

    fn minimal_document_file() -> DocumentFile {
        DocumentFile {
            schema_version: "0.1".to_owned(),
            root: vec![RootNode {
                id: DocumentId::new("ROOT-001"),
                statement: "A frozen ruling.".to_owned(),
                description: None,
                source: sample_source(),
            }],
            request: vec![SentenceNode {
                id: DocumentId::new("R-1"),
                statement: "A request.".to_owned(),
                description: None,
                derives_from: vec![DocumentId::new("ROOT-001")],
                cites: None,
                source: sample_source(),
            }],
            require: vec![],
            spec: vec![],
            detailed_spec: vec![],
            basic_design: vec![],
            design: vec![],
        }
    }

    #[test]
    fn document_file_round_trips_through_disk() {
        let root = std::env::temp_dir().join(format!(
            "vtest-store-canonical-doc-{}",
            crate::new_record_id()
        ));
        let layout = crate::init_project(&root, "example").unwrap();
        let file = minimal_document_file();

        write_document_file(&layout, "requirements", &file).unwrap();
        assert_eq!(read_document_file(&layout, "requirements").unwrap(), file);
    }

    #[test]
    fn document_file_has_no_identifying_field_on_disk() {
        // BD-330/DES-585: the file's *name* is the identity; a name unlike
        // the document's own content is accepted (nothing inside the file
        // is checked against it), unlike VO's id/file-name agreement.
        let root = std::env::temp_dir().join(format!(
            "vtest-store-canonical-doc-any-name-{}",
            crate::new_record_id()
        ));
        let layout = crate::init_project(&root, "example").unwrap();
        let file = minimal_document_file();
        write_document_file(&layout, "whatever-the-author-called-it", &file).unwrap();
        assert_eq!(
            read_document_file(&layout, "whatever-the-author-called-it").unwrap(),
            file
        );
    }

    #[test]
    fn document_file_rejects_unrecognized_schema_version() {
        let mut file = minimal_document_file();
        file.schema_version = "0.2".to_owned();
        let error = document_file_to_json(&file)
            .expect_err("an unrecognized schema_version must fail closed");
        assert!(matches!(
            error,
            StoreError::SchemaMismatch {
                code: "E-SCAN-010",
                ..
            }
        ));
    }

    #[test]
    fn document_file_rejects_malformed_node_id() {
        let mut file = minimal_document_file();
        file.root[0].id = DocumentId::new("ROOT-1"); // fewer than 3 digits
        document_file_to_json(&file).expect_err("a malformed id must fail closed");
    }

    #[test]
    fn document_file_rejects_node_in_wrong_layer_array() {
        let mut file = minimal_document_file();
        // A design-layer id placed in the request array — DS-1658.
        file.request[0].id = DocumentId::new("DES-001");
        document_file_to_json(&file).expect_err(
            "an id whose prefix disagrees with its layer array must fail closed (DS-1658)",
        );
    }

    #[test]
    fn document_file_rejects_empty_statement() {
        let mut file = minimal_document_file();
        file.request[0].statement = String::new();
        document_file_to_json(&file).expect_err("an empty statement must fail closed");
    }

    #[test]
    fn document_file_rejects_empty_source_doc() {
        let mut file = minimal_document_file();
        file.request[0].source.doc = String::new();
        document_file_to_json(&file).expect_err("an empty source.doc must fail closed");
    }

    #[test]
    fn document_file_rejects_source_lines_below_one() {
        let mut file = minimal_document_file();
        file.request[0].source.lines = [0, 1];
        document_file_to_json(&file).expect_err("a source.lines entry of 0 must fail closed");
    }

    /// `validate_node_source`'s `source.heading` check
    /// (specification.schema.json `$defs/source`, `minLength: 1`) — the
    /// module previously implemented this but had no test exercising it.
    #[test]
    fn document_file_rejects_empty_source_heading() {
        let mut file = minimal_document_file();
        file.request[0].source.heading = String::new();
        document_file_to_json(&file).expect_err("an empty source.heading must fail closed");
    }

    /// `validate_sentence_node`'s `cites` entry check (schema `minLength: 1`
    /// on each `cites[]` string) — implemented but previously untested.
    #[test]
    fn document_file_rejects_empty_cites_entry() {
        let mut file = minimal_document_file();
        file.request[0].cites = Some(vec![String::new()]);
        document_file_to_json(&file).expect_err("an empty cites entry must fail closed");
    }

    /// `validate_section_node`'s `title` check (schema `$defs/section`,
    /// `minLength: 1`) — this path is unreachable through `minimal_document_file`
    /// alone (its `require`/`spec`/etc. arrays are always empty), so this
    /// builds a populated `SectionNode` directly to exercise it.
    #[test]
    fn document_file_rejects_empty_section_title() {
        let mut file = minimal_document_file();
        file.require = vec![SectionNode {
            id: DocumentId::new("REQ-S001"),
            title: String::new(),
            description: None,
            source: sample_source(),
            derives_from: None,
            sections: None,
            items: None,
        }];
        document_file_to_json(&file).expect_err("an empty section title must fail closed");
    }

    /// DS-1658 layer/prefix agreement, checked on a node nested two levels
    /// deep (`require[].sections[].items[]`) — `validate_section_node`
    /// recurses into both `sections[]` and `items[]`, and this is the only
    /// test that reaches a nested `items[]` sentence rather than a
    /// top-level `request[]` one.
    #[test]
    fn document_file_rejects_nested_item_in_wrong_layer() {
        let mut file = minimal_document_file();
        file.require = vec![SectionNode {
            id: DocumentId::new("REQ-S001"),
            title: "A section".to_owned(),
            description: None,
            source: sample_source(),
            derives_from: None,
            sections: Some(vec![SectionNode {
                id: DocumentId::new("REQ-S002"),
                title: "A subsection".to_owned(),
                description: None,
                source: sample_source(),
                derives_from: None,
                sections: None,
                // A design-layer id nested inside the require layer's tree.
                items: Some(vec![SentenceNode {
                    id: DocumentId::new("DES-001"),
                    statement: "Wrong layer.".to_owned(),
                    description: None,
                    derives_from: vec![],
                    cites: None,
                    source: sample_source(),
                }]),
            }]),
            items: None,
        }];
        document_file_to_json(&file).expect_err(
            "a nested item whose id prefix disagrees with its layer must fail closed (DS-1658)",
        );
    }

    #[test]
    fn document_file_rejects_duplicate_derives_from() {
        let mut file = minimal_document_file();
        file.request[0].derives_from =
            vec![DocumentId::new("ROOT-001"), DocumentId::new("ROOT-001")];
        document_file_to_json(&file)
            .expect_err("a duplicated derives_from entry must fail closed (schema uniqueItems)");
    }

    /// specification.schema.json:52 (`derivedItem.derives_from`): each entry
    /// is `{ "$ref": "#/$defs/id" }`, the same pattern checked on a node's
    /// own `id` — a bogus string must fail closed on a sentence node's
    /// `derives_from`, not merely on `id` itself.
    #[test]
    fn document_file_rejects_malformed_derives_from_entry_on_sentence_node() {
        let mut file = minimal_document_file();
        file.request[0].derives_from = vec![DocumentId::new("totally bogus")];
        let error = document_file_to_json(&file)
            .expect_err("a derives_from entry not matching $defs/id must fail closed");
        assert!(matches!(
            error,
            StoreError::SchemaMismatch {
                code: "E-SCAN-010",
                ..
            }
        ));
    }

    #[test]
    fn document_file_rejects_empty_string_derives_from_entry() {
        let mut file = minimal_document_file();
        file.request[0].derives_from = vec![DocumentId::new("")];
        document_file_to_json(&file).expect_err("an empty derives_from entry must fail closed");
    }

    /// specification.schema.json:71 (`section.derives_from`) is the same
    /// `$defs/id`-constrained array as the sentence-node case above, checked
    /// on `SectionNode::derives_from` (`Option<Vec<DocumentId>>`) instead.
    #[test]
    fn document_file_rejects_malformed_derives_from_entry_on_section_node() {
        let mut file = minimal_document_file();
        file.require = vec![SectionNode {
            id: DocumentId::new("REQ-S001"),
            title: "A section".to_owned(),
            description: None,
            source: sample_source(),
            derives_from: Some(vec![DocumentId::new("nope")]),
            sections: None,
            items: None,
        }];
        document_file_to_json(&file).expect_err(
            "a section-level derives_from entry not matching $defs/id must fail closed",
        );
    }

    /// The same check must also fire on a sentence node nested inside a
    /// section's `items[]`, not only on a top-level `request[]` entry —
    /// `validate_section_node` recurses into both `sections[]` and
    /// `items[]` (canonical.rs `validate_section_node`).
    #[test]
    fn document_file_rejects_malformed_derives_from_entry_on_nested_item() {
        let mut file = minimal_document_file();
        file.require = vec![SectionNode {
            id: DocumentId::new("REQ-S001"),
            title: "A section".to_owned(),
            description: None,
            source: sample_source(),
            derives_from: None,
            sections: None,
            items: Some(vec![SentenceNode {
                id: DocumentId::new("REQ-001"),
                statement: "A nested requirement.".to_owned(),
                description: None,
                derives_from: vec![DocumentId::new("not-an-id")],
                cites: None,
                source: sample_source(),
            }]),
        }];
        document_file_to_json(&file)
            .expect_err("a nested item's malformed derives_from entry must fail closed");
    }

    /// Positive case locking in the scoping decision above: a `derives_from`
    /// entry pointing at a node in a *different* layer than the node that
    /// holds it is accepted here — DS-1658's layer/prefix agreement binds
    /// only a node's own id to its own layer's array (`validate_node_id`),
    /// never what that node cites as upstream. Only the id *pattern* is
    /// checked on a `derives_from` entry, matching how `specification.json`
    /// itself is shaped (e.g. a `detailed_spec`-layer `DS-` node deriving
    /// from a `spec`-layer `SPEC-` node throughout the real bundle).
    #[test]
    fn document_file_accepts_derives_from_entry_across_layers() {
        let mut file = minimal_document_file();
        file.require = vec![SectionNode {
            id: DocumentId::new("REQ-S001"),
            title: "A section".to_owned(),
            description: None,
            source: sample_source(),
            derives_from: None,
            sections: None,
            items: Some(vec![SentenceNode {
                id: DocumentId::new("REQ-001"),
                statement: "A requirement deriving from the request layer.".to_owned(),
                description: None,
                derives_from: vec![DocumentId::new("R-1")],
                cites: None,
                source: sample_source(),
            }]),
        }];
        document_file_to_json(&file)
            .expect("a derives_from entry in a different layer than its own node must be accepted");
    }

    #[test]
    fn document_file_json_rejects_explicit_null_description() {
        let json = r#"{
            "schema_version": "0.1",
            "root": [{
                "id": "ROOT-001",
                "statement": "x",
                "description": null,
                "source": { "doc": "d", "heading": "h", "lines": [1, 1] }
            }],
            "request": [], "require": [], "spec": [],
            "detailed_spec": [], "basic_design": [], "design": []
        }"#;
        let error = document_file_from_json(json)
            .expect_err("an explicit JSON null on an optional field must fail closed");
        assert!(matches!(
            error,
            StoreError::SchemaMismatch {
                code: "E-SCAN-010",
                ..
            }
        ));
    }

    #[test]
    fn document_file_json_accepts_absent_description() {
        let json = r#"{
            "schema_version": "0.1",
            "root": [{
                "id": "ROOT-001",
                "statement": "x",
                "source": { "doc": "d", "heading": "h", "lines": [1, 1] }
            }],
            "request": [], "require": [], "spec": [],
            "detailed_spec": [], "basic_design": [], "design": []
        }"#;
        document_file_from_json(json).unwrap();
    }

    #[test]
    fn document_file_rejects_unknown_top_level_field_fixture() {
        let text = include_str!("../tests/fixtures/document_unknown_field_top_level.json");
        let error =
            document_file_from_json(text).expect_err("an unknown top-level field must fail closed");
        assert!(matches!(error, StoreError::SchemaMismatch { .. }));
    }

    #[test]
    fn document_file_rejects_unknown_section_field_fixture() {
        let text = include_str!("../tests/fixtures/document_unknown_field_section.json");
        document_file_from_json(text).expect_err("an unknown section field must fail closed");
    }

    #[test]
    fn document_file_rejects_unknown_sentence_field_fixture() {
        let text = include_str!("../tests/fixtures/document_unknown_field_sentence.json");
        document_file_from_json(text).expect_err("an unknown sentence field must fail closed");
    }

    #[test]
    fn document_file_rejects_derives_from_anchor_note_fixture() {
        let text = include_str!("../tests/fixtures/document_derives_from_anchor_rejected.json");
        document_file_from_json(text).expect_err(
            "a document derives_from entry shaped as {doc, anchor} must fail closed — that shape is VO-only (DS-1638)",
        );
    }

    /// Default-run (not `#[ignore]`d) positive round-trip of PR #20's small,
    /// populated document fixture through *this crate's* reader/writer and
    /// through disk — the exit-gate item the four negative fixtures above
    /// (all `expect_err`) do not exercise: none of them is a document this
    /// store is expected to accept and preserve. Reads the fixture directly
    /// from `vtest-model`'s own `tests/fixtures/` rather than copying it, so
    /// there is exactly one copy of this fixture's content to keep in sync.
    #[test]
    fn document_sample_fixture_round_trips_through_the_store_reader_and_writer() {
        let text = include_str!("../../vtest-model/tests/fixtures/document_sample.json");

        let file = document_file_from_json(text)
            .expect("PR #20's small populated fixture must pass this store's reader checks");
        assert_eq!(file.root.len(), 1);
        assert_eq!(file.request.len(), 1);
        assert_eq!(file.require.len(), 1);
        assert_eq!(file.design.len(), 1);

        // Round-trip through this crate's own writer/reader in memory.
        let rewritten =
            document_file_to_json(&file).expect("re-serializing the fixture must validate too");
        let reparsed = document_file_from_json(&rewritten)
            .expect("the store's own writer output must be readable by its own reader");
        assert_eq!(reparsed, file);

        // Round-trip through disk, the same path `read_document_file`/
        // `write_document_file` exercise for every other document test.
        let root = std::env::temp_dir().join(format!(
            "vtest-store-canonical-doc-sample-{}",
            crate::new_record_id()
        ));
        let layout = crate::init_project(&root, "example").unwrap();
        write_document_file(&layout, "document_sample", &file).unwrap();
        assert_eq!(
            read_document_file(&layout, "document_sample").unwrap(),
            file
        );
    }

    /// Real-bundle round trip through *this crate's* reader/writer, not
    /// `vtest-model`'s own (already covered by
    /// `vtest_model::document::tests::canonical_bundle_round_trips_and_matches_node_counts`).
    /// Only runs when `VTEST_CANONICAL_BUNDLE` names the canonical
    /// `specification.json` — not run by default, since it depends on a
    /// file outside this crate's fixtures.
    #[test]
    #[ignore = "requires VTEST_CANONICAL_BUNDLE env var pointing at the canonical specification.json"]
    fn canonical_bundle_round_trips_through_the_store_reader() {
        let path = std::env::var("VTEST_CANONICAL_BUNDLE")
            .expect("set VTEST_CANONICAL_BUNDLE to the canonical specification.json path");
        let text = std::fs::read_to_string(&path).expect("failed to read canonical bundle");

        let file = document_file_from_json(&text)
            .expect("the canonical bundle must pass every check this store's reader applies");

        fn count_sections(sections: &[SectionNode]) -> usize {
            sections
                .iter()
                .map(|s| {
                    let items = s.items.as_deref().unwrap_or_default().len();
                    let nested = s.sections.as_deref().unwrap_or_default();
                    1 + items + count_sections(nested)
                })
                .sum()
        }

        // Measured against the *pinned* authority this PR is scoped to —
        // the content at commit `fa96642` (blob `92f10cd7`), i.e.
        // `git show fa96642:docs/canonical/specification.json` — not
        // whatever `VTEST_CANONICAL_BUNDLE` happens to point at when this
        // test is run. Verified against that exact blob (hash-object
        // matched `git rev-parse fa96642:docs/canonical/specification.json`)
        // and cross-checked by running vtest-model's own
        // `document::tests::canonical_bundle_round_trips_and_matches_node_counts`
        // against the same snapshot, which passes with these identical
        // per-layer figures — so this is not "vtest-model's own hardcoded,
        // possibly-stale count" as a prior version of this comment claimed;
        // both crates agree at the pin. The main worktree's *live*
        // `docs/canonical/specification.json` has since moved three commits
        // past this pin (`fa96642..fc35e58`: cdab898, 743b6bb, fc35e58) and
        // measures 3848 there (detailed_spec/design each +1) — that drift is
        // real but is a fact about the moving bundle, not a defect in this
        // reader or in vtest-model's count, and is out of this PR's scope
        // (docs/ is not edited here). Whoever next re-pins this test's
        // `VTEST_CANONICAL_BUNDLE` fixture to a newer commit must re-measure
        // and update every assertion below together.
        assert_eq!(file.root.len(), 48, "root layer node count");
        assert_eq!(file.request.len(), 5, "request layer node count");
        assert_eq!(
            count_sections(&file.require),
            395,
            "require layer node count"
        );
        assert_eq!(count_sections(&file.spec), 593, "spec layer node count");
        assert_eq!(
            count_sections(&file.detailed_spec),
            1734,
            "detailed_spec layer node count"
        );
        assert_eq!(
            count_sections(&file.basic_design),
            408,
            "basic_design layer node count"
        );
        assert_eq!(count_sections(&file.design), 663, "design layer node count");

        let total = file.root.len()
            + file.request.len()
            + count_sections(&file.require)
            + count_sections(&file.spec)
            + count_sections(&file.detailed_spec)
            + count_sections(&file.basic_design)
            + count_sections(&file.design);
        assert_eq!(total, 3846, "total node count across all layers");

        // Round-trip back out through this crate's own writer and re-read.
        let rewritten = document_file_to_json(&file).expect("re-serialization must validate too");
        let reparsed = document_file_from_json(&rewritten)
            .expect("the store's own writer output must be readable by its own reader");
        assert_eq!(reparsed, file);
    }

    // -------------------------------------------------------------
    // VO record tests
    // -------------------------------------------------------------

    fn sample_vo() -> VoRecord {
        VoRecord {
            id: VoId::new("VO-PARSER-UTF8-003"),
            parent: Some(VoId::new("VO-PARSER-UTF8")),
            derives_from: vec![DerivesFrom {
                doc: DocumentId::new("DOC-BASIC-001"),
                anchor: Some("§8.2条項2".to_string()),
                note: None,
            }],
            claim: "不正な continuation byte を含む入力を与えた場合、InvalidUtf8 を返す"
                .to_string(),
            dimensions: vec![Dimension {
                name: "operand-sign".to_string(),
                partitions: vec!["positive".to_string(), "negative".to_string()],
            }],
            coverage_policy: Some(CoveragePolicy::FullProduct),
            combinations: vec![],
            representative_cases: vec!["empty input".to_string()],
            created: "2026-08-08".to_string(),
            updated: "2026-08-08".to_string(),
        }
    }

    #[test]
    fn vo_record_round_trips_through_canonical_yaml() {
        let record = sample_vo();
        let yaml = vo_record_to_yaml(&record);
        let (parsed, diagnostics) = vo_record_from_yaml(&yaml, record.id.as_str()).unwrap();
        assert_eq!(parsed, record);
        assert!(diagnostics.is_empty());
    }

    /// `derives_from` itself is mandatory (SPEC-015); this exercises every
    /// *other* optional field being absent instead.
    #[test]
    fn vo_record_without_parent_or_other_optional_fields_round_trips() {
        let record = VoRecord {
            id: VoId::new("VO-ROOT-001"),
            parent: None,
            derives_from: vec![DerivesFrom {
                doc: DocumentId::new("DOC-BASIC-001"),
                anchor: None,
                note: None,
            }],
            claim: "A root VO.".to_string(),
            dimensions: vec![],
            coverage_policy: None,
            combinations: vec![],
            representative_cases: vec![],
            created: "2026-08-08".to_string(),
            updated: "2026-08-08".to_string(),
        };
        let yaml = vo_record_to_yaml(&record);
        assert_eq!(
            vo_record_from_yaml(&yaml, record.id.as_str()).unwrap().0,
            record
        );
    }

    #[test]
    fn vo_record_with_empty_derives_from_is_rejected_on_read_and_write() {
        let mut record = sample_vo();
        record.derives_from = vec![];

        let yaml = vo_record_to_yaml(&record);
        vo_record_from_yaml(&yaml, record.id.as_str()).expect_err(
            "a VO with zero derives_from entries must fail closed, not round-trip as a root",
        );

        let root = std::env::temp_dir().join(format!(
            "vtest-store-canonical-vo-empty-derives-{}",
            crate::new_record_id()
        ));
        let layout = crate::init_project(&root, "example").unwrap();
        write_vo_record(&layout, &record)
            .expect_err("the writer must refuse to persist a VO it could not itself read back");
    }

    /// DES-117's own example, verbatim (including its inline comments), fed
    /// straight to the reader.
    #[test]
    fn vo_record_parses_the_literal_des_117_example() {
        let yaml = "\
id: VO-PARSER-UTF8-003
parent: VO-PARSER-UTF8          # VO ID または null（階層化）
derives_from:                   # 1件以上の document への直結（基本仕様 §3.2）
  - doc: DOC-BASIC-001
    anchor: \"§8.2条項2\"         # 任意の上流該当箇所（節番号等・空可・非 MISMATCH）
    note: \"\"                    # 任意（空可・非 MISMATCH）
claim: 不正な continuation byte を含む入力を与えた場合、ParseError::InvalidUtf8 を返す
dimensions: []                  # 検証軸（任意。§3.2.1）
coverage_policy: null           # independent-axes | full-product | explicit | null
combinations: []                # coverage_policy: explicit のとき実体化する組合せ（§3.2.1）
representative_cases: []        # 代表入力値（任意）
created: 2026-08-08
updated: 2026-08-08
";
        let (record, diagnostics) = vo_record_from_yaml(yaml, "VO-PARSER-UTF8-003").unwrap();
        assert_eq!(
            record.parent.as_ref().map(VoId::as_str),
            Some("VO-PARSER-UTF8")
        );
        assert_eq!(record.derives_from[0].doc.as_str(), "DOC-BASIC-001");
        assert_eq!(record.coverage_policy, None);
        assert!(record.combinations.is_empty());
        assert!(diagnostics.is_empty());
        let roundtrip = vo_record_to_yaml(&record);
        assert_eq!(
            vo_record_from_yaml(&roundtrip, "VO-PARSER-UTF8-003")
                .unwrap()
                .0,
            record
        );
    }

    /// §3.2.1's `explicit` combinations example, verbatim: each entry is a
    /// dimension-name → partition-value flow-mapping, not a positional list
    /// of bare strings.
    #[test]
    fn vo_record_parses_the_literal_combinations_example() {
        let yaml = "\
id: VO-ARITH-001
parent: null
derives_from:
  - doc: DOC-BASIC-001
claim: claim
dimensions:
  - name: operand-sign
    partitions: [positive, negative]
  - name: operator
    partitions: [add, sub, mul, div]
coverage_policy: explicit
combinations:
  - { operand-sign: positive, operator: div }
  - { operand-sign: negative, operator: div }
representative_cases: []
created: 2026-08-08
updated: 2026-08-08
";
        let (record, diagnostics) = vo_record_from_yaml(yaml, "VO-ARITH-001").unwrap();
        assert_eq!(record.combinations.len(), 2);
        assert_eq!(
            record.combinations[0]
                .get("operand-sign")
                .map(String::as_str),
            Some("positive")
        );
        assert_eq!(
            record.combinations[0].get("operator").map(String::as_str),
            Some("div")
        );
        assert!(diagnostics.is_empty());
        let roundtrip = vo_record_to_yaml(&record);
        assert_eq!(
            vo_record_from_yaml(&roundtrip, "VO-ARITH-001").unwrap().0,
            record
        );
    }

    /// DS-405: the reader accepts `status` (does not reject the record) but
    /// ignores its *value* and instead notifies W-STORE-001 on the field's
    /// mere presence — this checks both halves.
    #[test]
    fn vo_record_status_read_compat_field_value_is_ignored_but_presence_warns() {
        let record = sample_vo();
        let mut yaml = vo_record_to_yaml(&record);
        yaml.push_str("status: draft\n");
        let (parsed, diagnostics) = vo_record_from_yaml(&yaml, record.id.as_str()).unwrap();
        assert_eq!(parsed, record);
        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics[0].code, "W-STORE-001");
    }

    #[test]
    fn vo_record_without_status_field_reports_no_diagnostics() {
        let record = sample_vo();
        let yaml = vo_record_to_yaml(&record);
        assert!(!yaml.contains("status"));
        let (_, diagnostics) = vo_record_from_yaml(&yaml, record.id.as_str()).unwrap();
        assert!(diagnostics.is_empty());
    }

    /// DS-1645: a genuinely unknown field fails closed even when `status`
    /// (a distinct, legitimate read-compat field per DS-405) is also
    /// present — the two do not get to coexist as two warnings any more.
    #[test]
    fn vo_record_with_status_and_another_unknown_field_is_rejected() {
        let record = sample_vo();
        let mut yaml = vo_record_to_yaml(&record);
        yaml.push_str("status: draft\nnickname: quick-vo\n");
        let error = vo_record_from_yaml(&yaml, record.id.as_str())
            .expect_err("a genuinely unknown field must fail closed even alongside `status`");
        assert!(matches!(error, StoreError::SchemaMismatch { .. }));
    }

    /// DS-1645 (E-SCAN-010): an unknown top-level field is now rejected,
    /// not merely warned about (this replaces the retired DS-376 behavior —
    /// see `docs/canonical/relations/retired-ids.json`).
    #[test]
    fn vo_record_with_unknown_top_level_field_is_rejected() {
        let record = sample_vo();
        let mut yaml = vo_record_to_yaml(&record);
        yaml.push_str("owner: someone\n");
        let error = vo_record_from_yaml(&yaml, record.id.as_str())
            .expect_err("an unknown top-level field must fail closed");
        assert!(matches!(error, StoreError::SchemaMismatch { .. }));
    }

    /// Unlike `ApprovalRecord`/`read_evidence` (`records.rs`), a VO record's
    /// unknown-field scan is always followed by `yaml_serde::from_value::<
    /// VoRecord>` (below), which itself rejects a non-string mapping key
    /// with its own type error independent of `reject_unknown_fields`. This
    /// locks that in, so a future change to either layer cannot silently
    /// reopen the non-string-key gap `reject_unknown_fields` itself now
    /// closes (DS-1645).
    #[test]
    fn vo_record_with_non_string_top_level_key_is_rejected() {
        let record = sample_vo();
        let mut yaml = vo_record_to_yaml(&record);
        yaml.push_str("2026: unlimited\n");
        vo_record_from_yaml(&yaml, record.id.as_str())
            .expect_err("a non-string top-level key must fail closed");
    }

    #[test]
    fn vo_record_with_unknown_nested_derives_from_field_is_rejected() {
        let yaml = "\
id: VO-PARSER-UTF8-003
parent: null
derives_from:
  - doc: DOC-BASIC-001
    foo: bar
claim: claim
dimensions: []
coverage_policy: null
combinations: []
representative_cases: []
created: 2026-08-08
updated: 2026-08-08
";
        vo_record_from_yaml(yaml, "VO-PARSER-UTF8-003")
            .expect_err("an unknown nested derives_from field must fail closed");
    }

    #[test]
    fn vo_record_with_unknown_nested_dimensions_field_is_rejected() {
        let yaml = "\
id: VO-ARITH-001
parent: null
derives_from:
  - doc: DOC-BASIC-001
claim: claim
dimensions:
  - name: operand-sign
    partitions: [positive, negative]
    bar: baz
coverage_policy: null
combinations: []
representative_cases: []
created: 2026-08-08
updated: 2026-08-08
";
        vo_record_from_yaml(yaml, "VO-ARITH-001")
            .expect_err("an unknown nested dimensions field must fail closed");
    }

    #[test]
    fn vo_record_with_id_disagreeing_with_file_name_is_rejected() {
        let yaml = vo_record_to_yaml(&sample_vo());
        let error = vo_record_from_yaml(&yaml, "VO-OTHER-001")
            .expect_err("a VO id that disagrees with the file name must fail closed");
        assert!(error.to_string().contains("does not match file name"));
    }

    #[test]
    fn vo_record_missing_a_required_field_is_rejected() {
        let yaml = vo_record_to_yaml(&sample_vo());
        for key in ["id", "claim", "created", "updated"] {
            let without_field = yaml
                .lines()
                .filter(|line| !line.starts_with(&format!("{key}:")))
                .collect::<Vec<_>>()
                .join("\n");
            assert!(
                vo_record_from_yaml(&without_field, "VO-PARSER-UTF8-003").is_err(),
                "expected a VO missing `{key}` to fail closed"
            );
        }
    }

    #[test]
    fn vo_record_with_unrecognized_coverage_policy_is_rejected() {
        let yaml = vo_record_to_yaml(&sample_vo())
            .replace("coverage_policy: full-product", "coverage_policy: bogus");
        vo_record_from_yaml(&yaml, "VO-PARSER-UTF8-003").expect_err(
            "an unrecognized coverage_policy must fail closed, not silently become None",
        );
    }

    #[test]
    fn vo_record_read_write_round_trips_through_disk() {
        let root = std::env::temp_dir().join(format!(
            "vtest-store-canonical-vo-{}",
            crate::new_record_id()
        ));
        let layout = crate::init_project(&root, "example").unwrap();
        let record = sample_vo();

        write_vo_record(&layout, &record).unwrap();
        assert_eq!(
            read_vo_record(&layout, record.id.as_str()).unwrap().0,
            record
        );
    }

    #[test]
    fn vo_record_combinations_round_trip_as_dimension_keyed_maps() {
        let mut record = sample_vo();
        record.dimensions = vec![
            Dimension {
                name: "operand-sign".to_string(),
                partitions: vec!["positive".to_string(), "negative".to_string()],
            },
            Dimension {
                name: "operator".to_string(),
                partitions: vec!["div".to_string()],
            },
        ];
        record.coverage_policy = Some(CoveragePolicy::Explicit);
        record.combinations = vec![BTreeMap::from([
            ("operand-sign".to_string(), "positive".to_string()),
            ("operator".to_string(), "div".to_string()),
        ])];
        let yaml = vo_record_to_yaml(&record);
        assert_eq!(
            vo_record_from_yaml(&yaml, record.id.as_str()).unwrap().0,
            record
        );
    }
}
