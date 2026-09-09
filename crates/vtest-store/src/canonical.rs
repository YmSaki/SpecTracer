//! Canonical v0.1 record storage, for the types defined in `vtest-model`.
//! Kept separate from `records.rs`'s predecessor Req/Spec-model types so
//! neither module's exports collide with the other's.
//!
//! The upstream document model (`DocumentFile`) is JSON — BD-319: "上流文書
//! のファイル形式は JSON とし、その他のレコードのファイル形式はすべて YAML
//! とする" (basic_design), echoed at BD-320: "上流文書のレコードは JSON とし、
//! その他のレコードはすべて YAML とする" (detailed_spec) — while VO and
//! Relation stay YAML through `yaml_serde`. `yaml_serde` is used (rather
//! than the hand-rolled scalar/list helpers `records.rs` uses for
//! predecessor types) because those helpers silently mis-parse inline
//! comments and flow-mappings (both of which the canonical VO example —
//! DES-117 — uses).

use crate::{read_text, write_atomic, StoreError, VerifyLayout};
use vtest_model::{
    CombinationEntry, DerivesFrom, Diagnostic, DocumentFile, DocumentId, Layer, NodeSource,
    RootNode, SectionNode, SentenceNode, VoRecord,
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

// `derives_from[]`/`dimensions[]` unknown-field scanning for a VO record
// runs against `LenientValue` instead of this shape — see that type's own
// doc comment, just above `vo_record_from_yaml`, for why (DS-422/DS-902,
// E-SCAN-017 condition 6: a `combinations[]` entry repeating one dimension
// name must reach the scan layer intact, and `yaml_serde::Value` cannot
// represent that). `RelationRecord`/`ApprovalRecord`/Evidence (`records.rs`)
// have no field with that same duplicate-tolerance requirement, so they
// keep using `reject_unknown_fields` directly against `yaml_serde::Value`.

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

/// A duplicate-tolerant mirror of `yaml_serde::Value`'s mapping/sequence
/// shape, used only by `vo_record_from_yaml`'s unknown-field/`status` scan
/// below. `yaml_serde::Value`'s own `Mapping` (`mapping.rs`) rejects *any*
/// duplicate key it finds while deserializing, at *any* depth in the
/// document tree — not only inside `combinations[]`. DS-422: "entryが
/// 宣言済みdimensionのいずれかを欠く、または同じdimension名を2回以上持つ
/// 場合、当該VOレコードは `combinations` 不正とし、`E-SCAN-017` を報告して
/// 当該VOの `chain_integrity` を `MISMATCH` とする（§17.1）。" — a
/// `combinations[]` entry
/// repeating one dimension name must reach `vtest-scan`'s `invalid_vo_
/// combinations` check intact (the VO retained, not dropped), not be
/// rejected here as E-SCAN-010 before that scan-layer judgment ever runs.
/// `VoRecord.combinations`'s element type, `CombinationEntry`
/// (`vtest-model`), already preserves such a duplicate losslessly instead of
/// silently collapsing or rejecting it — but only when built directly from
/// `text`, bypassing `yaml_serde::Value` (see that type's own doc comment).
/// So this record's *unknown-field* scan (DS-1645, run independently of the
/// primary parse below) needs its own value type that likewise never fails
/// to build on a duplicate key.
///
/// Mirrors just the shape the scan needs — mapping (as ordered key/value
/// pairs, keeping every occurrence instead of rejecting a repeat, the same
/// technique `CombinationEntry` uses), sequence, and "anything else" (a bare
/// scalar, kept only far enough to distinguish it from a mapping/sequence) —
/// built directly from `MapAccess`/`SeqAccess` rather than delegating to
/// `yaml_serde::Mapping`/`Sequence`, so *no* level of the tree can fail on a
/// duplicate key.
#[derive(Debug)]
enum LenientValue {
    Mapping(Vec<(LenientValue, LenientValue)>),
    Sequence(Vec<LenientValue>),
    String(String),
    Scalar,
}

impl LenientValue {
    fn as_str(&self) -> Option<&str> {
        match self {
            LenientValue::String(value) => Some(value.as_str()),
            _ => None,
        }
    }

    fn as_mapping(&self) -> Option<&[(LenientValue, LenientValue)]> {
        match self {
            LenientValue::Mapping(pairs) => Some(pairs.as_slice()),
            _ => None,
        }
    }

    fn as_sequence(&self) -> Option<&[LenientValue]> {
        match self {
            LenientValue::Sequence(items) => Some(items.as_slice()),
            _ => None,
        }
    }

    fn get(&self, key: &str) -> Option<&LenientValue> {
        self.as_mapping()?
            .iter()
            .find(|(candidate, _)| candidate.as_str() == Some(key))
            .map(|(_, value)| value)
    }
}

impl<'de> serde::Deserialize<'de> for LenientValue {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct LenientVisitor;

        impl<'de> serde::de::Visitor<'de> for LenientVisitor {
            type Value = LenientValue;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("any YAML value")
            }

            fn visit_bool<E: serde::de::Error>(self, _value: bool) -> Result<LenientValue, E> {
                Ok(LenientValue::Scalar)
            }

            fn visit_i64<E: serde::de::Error>(self, _value: i64) -> Result<LenientValue, E> {
                Ok(LenientValue::Scalar)
            }

            fn visit_u64<E: serde::de::Error>(self, _value: u64) -> Result<LenientValue, E> {
                Ok(LenientValue::Scalar)
            }

            fn visit_f64<E: serde::de::Error>(self, _value: f64) -> Result<LenientValue, E> {
                Ok(LenientValue::Scalar)
            }

            fn visit_i128<E: serde::de::Error>(self, _value: i128) -> Result<LenientValue, E> {
                Ok(LenientValue::Scalar)
            }

            fn visit_u128<E: serde::de::Error>(self, _value: u128) -> Result<LenientValue, E> {
                Ok(LenientValue::Scalar)
            }

            fn visit_str<E: serde::de::Error>(self, value: &str) -> Result<LenientValue, E> {
                Ok(LenientValue::String(value.to_owned()))
            }

            fn visit_string<E: serde::de::Error>(self, value: String) -> Result<LenientValue, E> {
                Ok(LenientValue::String(value))
            }

            fn visit_unit<E: serde::de::Error>(self) -> Result<LenientValue, E> {
                Ok(LenientValue::Scalar)
            }

            fn visit_none<E: serde::de::Error>(self) -> Result<LenientValue, E> {
                Ok(LenientValue::Scalar)
            }

            fn visit_some<D2: serde::Deserializer<'de>>(
                self,
                deserializer: D2,
            ) -> Result<LenientValue, D2::Error> {
                serde::Deserialize::deserialize(deserializer)
            }

            fn visit_seq<A: serde::de::SeqAccess<'de>>(
                self,
                mut sequence: A,
            ) -> Result<LenientValue, A::Error> {
                let mut items = Vec::new();
                while let Some(item) = sequence.next_element::<LenientValue>()? {
                    items.push(item);
                }
                Ok(LenientValue::Sequence(items))
            }

            fn visit_map<A: serde::de::MapAccess<'de>>(
                self,
                mut map: A,
            ) -> Result<LenientValue, A::Error> {
                // Deliberately not `yaml_serde::Mapping::deserialize` here —
                // see this type's own doc comment for why: every `(key,
                // value)` `next_entry` yields is kept, in order, even if a
                // key repeats, exactly like `CombinationEntry` does for the
                // one field that needs it.
                let mut pairs = Vec::new();
                while let Some(pair) = map.next_entry::<LenientValue, LenientValue>()? {
                    pairs.push(pair);
                }
                Ok(LenientValue::Mapping(pairs))
            }

            fn visit_enum<A: serde::de::EnumAccess<'de>>(
                self,
                data: A,
            ) -> Result<LenientValue, A::Error> {
                // A locally-tagged value (`!Foo ...`) — `yaml_serde` routes
                // any scalar/sequence/mapping carrying an unrecognized `!`
                // tag through `deserialize_any`'s `visit_enum`, with the tag
                // itself as the "variant name". DS-1645's schema never
                // assigns meaning to a tag, so the tag is discarded
                // (`IgnoredAny`) and the content underneath it is read as a
                // plain `LenientValue` via `newtype_variant`.
                use serde::de::VariantAccess;
                let (_tag, variant): (serde::de::IgnoredAny, _) = data.variant()?;
                variant.newtype_variant::<LenientValue>()
            }
        }

        deserializer.deserialize_any(LenientVisitor)
    }
}

/// `reject_unknown_fields`'s non-string-key/unknown-key rejection, applied
/// to a `LenientValue` instead of `yaml_serde::Value`. Kept as a separate
/// function (rather than making `reject_unknown_fields` generic) because the
/// two value types are used by disjoint callers for disjoint reasons — see
/// `LenientValue`'s own doc comment.
fn reject_unknown_fields_lenient(
    value: &LenientValue,
    known: &[&str],
    prefix: &str,
) -> Result<(), StoreError> {
    let Some(mapping) = value.as_mapping() else {
        return Ok(());
    };
    for (key, _) in mapping {
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

/// Extends `reject_unknown_fields_lenient` into each `derives_from[]` entry
/// (DS-1638).
fn reject_derives_from_unknown_fields(value: &LenientValue) -> Result<(), StoreError> {
    let Some(sequence) = value
        .get("derives_from")
        .and_then(LenientValue::as_sequence)
    else {
        return Ok(());
    };
    for (index, entry) in sequence.iter().enumerate() {
        reject_unknown_fields_lenient(
            entry,
            DERIVES_FROM_KEYS,
            &format!("derives_from[{index}]."),
        )?;
    }
    Ok(())
}

/// Extends `reject_unknown_fields_lenient` into each VO `dimensions[]`
/// entry. `combinations[]` is deliberately not scanned the same way: each
/// entry's keys are the *dimension names themselves* (a dynamic,
/// record-specific vocabulary), not a fixed schema — whether a combination
/// covers exactly the declared dimensions is E-SCAN-017, a chain_integrity/
/// scan-time concern this record-level reader does not have the dimension
/// set resolved enough to evaluate (the same record-vs-scan layer split
/// `require_at_least_one_derives_from` below documents for E-SCAN-012).
fn reject_dimensions_unknown_fields(value: &LenientValue) -> Result<(), StoreError> {
    let Some(sequence) = value.get("dimensions").and_then(LenientValue::as_sequence) else {
        return Ok(());
    };
    for (index, entry) in sequence.iter().enumerate() {
        reject_unknown_fields_lenient(entry, DIMENSION_KEYS, &format!("dimensions[{index}]."))?;
    }
    Ok(())
}

/// Parses a canonical `VoRecord` from its YAML representation, returning any
/// non-fatal diagnostics alongside it. `yaml_serde::from_str::<VoRecord>`
/// fails closed on a missing required field (`claim`/`created`/`updated`/
/// etc.) or an unrecognized `coverage_policy` value via `VoRecord`'s
/// `Deserialize` derive; this adds the id/file-name check the derive cannot
/// express, the `derives_from` cardinality floor (SPEC-015), plus DS-1645's
/// fail-closed rejection of any field outside the schema.
///
/// **Two independent parses, not one `Value`-first pass.** DS-422/DS-902
/// (E-SCAN-017 condition 6) require a `combinations[]` entry that repeats
/// one dimension name to reach the scan layer intact (VO retained,
/// `chain_integrity = MISMATCH`), not be rejected here as E-SCAN-010 before
/// that check ever runs. Routing the whole record through `yaml_serde::
/// Value` first — as this function used to — makes that record-layer
/// rejection pre-empt the scan-layer judgment for any `combinations[]`-
/// confined duplicate, because `Value`'s own `Mapping` rejects *any*
/// duplicate key anywhere in the document before a `VoRecord` (or anything
/// else) is ever built from it.
///
/// So `VoRecord` is built directly from `text` (`yaml_serde::from_str::
/// <VoRecord>`), bypassing `Value` for the primary parse.
/// `VoRecord.combinations`'s element type, `CombinationEntry` (`vtest-
/// model`), has a hand-written `Deserialize` that preserves a repeated
/// dimension name instead of rejecting or silently collapsing it (see that
/// type's own doc comment) — the one field in `VoRecord`'s tree that needs
/// this. Every other field keeps rejecting a duplicate key on its own, with
/// no help from `Value`: a struct's derived `Deserialize`
/// (`VoRecord`/`DerivesFrom`/`Dimension`) tracks each field and errors the
/// second time it sees one, independent of `Value` (see `vtest_model::
/// vo::CombinationEntry`'s doc comment, which states this empirically-
/// confirmed contract; `vo_record_with_duplicate_top_level_key_is_rejected`/
/// `vo_record_with_duplicate_key_in_derives_from_entry_is_rejected` below
/// lock it in for this crate too).
///
/// The unknown-field/`status` diagnostics this function also computes run
/// against `LenientValue` (above), not `yaml_serde::Value` — built from
/// `text` independently of the primary parse, since that scan walks
/// arbitrary keys `VoRecord`'s own fixed schema does not know about, which a
/// typed struct parse alone cannot surface. `LenientValue` cannot itself
/// fail to build on a duplicate key anywhere in the document (see its own
/// doc comment), so this costs nothing: an unrelated unknown field
/// alongside a `combinations[]` duplicate still fails closed
/// (`vo_record_combinations_duplicate_with_unrelated_unknown_field_is_
/// still_rejected` below).
pub fn vo_record_from_yaml(
    text: &str,
    fallback_id: &str,
) -> Result<(VoRecord, Vec<Diagnostic>), StoreError> {
    let lenient: LenientValue = yaml_serde::from_str(text)
        .map_err(|error| StoreError::InvalidConfig(format!("invalid VO record: {error}")))?;

    let mut diagnostics = Vec::new();
    if lenient.get("status").is_some() {
        diagnostics.push(Diagnostic::warning(
            "W-STORE-001",
            "VO record has the non-canonical read-compat field `status`; its value is ignored — effective state and the VO subject hash are derived from approvals instead",
        ));
    }
    reject_unknown_fields_lenient(&lenient, VO_KEYS, "")?;
    reject_derives_from_unknown_fields(&lenient)?;
    reject_dimensions_unknown_fields(&lenient)?;

    let record: VoRecord = yaml_serde::from_str(text)
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
/// in place (BD-321). Enforces the same two checks a reader would apply
/// after the fact, so a writer that skipped them could not silently
/// persist a record `read_vo_record`/`vtest-scan`'s own validation would
/// then reject or (worse) panic on:
/// - the `derives_from` cardinality floor (`require_at_least_one_derives_
///   from`);
/// - no `combinations[]` entry repeating one dimension name
///   (`require_no_duplicate_combination_dimension_names`).
pub fn write_vo_record(layout: &VerifyLayout, record: &VoRecord) -> Result<(), StoreError> {
    require_at_least_one_derives_from(&record.derives_from)?;
    require_no_duplicate_combination_dimension_names(&record.combinations)?;
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

/// DS-422/DS-902 (E-SCAN-017 condition 6, 詳細設計 v0.1 本冊:283): a
/// `combinations[]` entry naming the same dimension twice is `combinations`
/// 不正. `CombinationEntry`'s own `Serialize` impl (`vtest-model`) already
/// refuses (returns `Err`) to emit such an entry rather than silently
/// writing a YAML mapping with a repeated key — but `vo_record_to_yaml`
/// reaches that via `yaml_serde::to_string(record).expect(...)`, so without
/// this check first, a caller handing `write_vo_record` a malformed record
/// would panic here instead of getting a clean `StoreError`. This mirrors
/// `require_at_least_one_derives_from` immediately above: enforce at write
/// time what the read/scan side would reject anyway, so the failure mode is
/// a `Result`, not a panic.
fn require_no_duplicate_combination_dimension_names(
    combinations: &[CombinationEntry],
) -> Result<(), StoreError> {
    for entry in combinations {
        let duplicates = entry.duplicate_dimension_names();
        if !duplicates.is_empty() {
            return Err(StoreError::InvalidConfig(format!(
                "combinations entry declares dimension `{}` more than once",
                duplicates.join("`, `")
            )));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
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

    /// @vtest.id TEST-STORE-CANONICAL-DOC-ROUND-TRIP-DISK
    /// @vtest.covers VO-STORE-CANONICAL-ROUND-TRIP-FIDELITY
    /// @vtest.target crates/vtest-store/src/canonical.rs::write_document_file,read_document_file
    /// @vtest.intent Writing a document file and reading it back yields the same DocumentFile.
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

    /// @vtest.id TEST-STORE-CANONICAL-DOC-NO-IDENTITY-ON-DISK
    /// @vtest.covers VO-MODEL-DOCUMENT-FILE-SHAPE
    /// @vtest.target crates/vtest-store/src/canonical.rs::write_document_file,read_document_file
    /// @vtest.intent A document file's name, not any field inside it, is its identity — an arbitrary file name round-trips.
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

    /// @vtest.id TEST-STORE-CANONICAL-DOC-REJECTS-SCHEMA-VERSION
    /// @vtest.covers VO-MODEL-DOCUMENT-SCHEMA-CONFORMANCE
    /// @vtest.target crates/vtest-store/src/canonical.rs::document_file_to_json
    /// @vtest.intent An unrecognized schema_version fails closed as E-SCAN-010.
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

    /// @vtest.id TEST-STORE-CANONICAL-DOC-REJECTS-MALFORMED-ID
    /// @vtest.covers VO-MODEL-DOCUMENT-LAYER-ID-PREFIX
    /// @vtest.target crates/vtest-store/src/canonical.rs::document_file_to_json
    /// @vtest.intent A node id that does not conform to its layer's prefix pattern fails closed.
    #[test]
    fn document_file_rejects_malformed_node_id() {
        let mut file = minimal_document_file();
        file.root[0].id = DocumentId::new("ROOT-1"); // fewer than 3 digits
        document_file_to_json(&file).expect_err("a malformed id must fail closed");
    }

    /// @vtest.id TEST-STORE-CANONICAL-DOC-REJECTS-WRONG-LAYER-ARRAY
    /// @vtest.covers VO-MODEL-DOCUMENT-LAYER-ARRAY-PREFIX-AGREEMENT
    /// @vtest.target crates/vtest-store/src/canonical.rs::document_file_to_json
    /// @vtest.intent A node whose id prefix disagrees with the top-level array it is placed in fails closed (DS-1658).
    #[test]
    fn document_file_rejects_node_in_wrong_layer_array() {
        let mut file = minimal_document_file();
        // A design-layer id placed in the request array — DS-1658.
        file.request[0].id = DocumentId::new("DES-001");
        document_file_to_json(&file).expect_err(
            "an id whose prefix disagrees with its layer array must fail closed (DS-1658)",
        );
    }

    /// @vtest.id TEST-STORE-CANONICAL-DOC-REJECTS-EMPTY-STATEMENT
    /// @vtest.covers VO-MODEL-DOCUMENT-SCHEMA-CONFORMANCE
    /// @vtest.target crates/vtest-store/src/canonical.rs::document_file_to_json
    /// @vtest.intent An empty statement fails closed as a schema mismatch.
    #[test]
    fn document_file_rejects_empty_statement() {
        let mut file = minimal_document_file();
        file.request[0].statement = String::new();
        document_file_to_json(&file).expect_err("an empty statement must fail closed");
    }

    /// @vtest.id TEST-STORE-CANONICAL-DOC-REJECTS-EMPTY-SOURCE-DOC
    /// @vtest.covers VO-MODEL-DOCUMENT-SCHEMA-CONFORMANCE
    /// @vtest.target crates/vtest-store/src/canonical.rs::document_file_to_json
    /// @vtest.intent An empty source.doc fails closed as a schema mismatch.
    #[test]
    fn document_file_rejects_empty_source_doc() {
        let mut file = minimal_document_file();
        file.request[0].source.doc = String::new();
        document_file_to_json(&file).expect_err("an empty source.doc must fail closed");
    }

    /// @vtest.id TEST-STORE-CANONICAL-DOC-REJECTS-SOURCE-LINES-BELOW-ONE
    /// @vtest.covers VO-MODEL-DOCUMENT-SCHEMA-CONFORMANCE
    /// @vtest.target crates/vtest-store/src/canonical.rs::document_file_to_json
    /// @vtest.intent A source.lines entry of 0 fails closed as a schema mismatch.
    #[test]
    fn document_file_rejects_source_lines_below_one() {
        let mut file = minimal_document_file();
        file.request[0].source.lines = [0, 1];
        document_file_to_json(&file).expect_err("a source.lines entry of 0 must fail closed");
    }

    /// @vtest.id TEST-STORE-CANONICAL-DOC-REJECTS-EMPTY-SOURCE-HEADING
    /// @vtest.covers VO-MODEL-DOCUMENT-SCHEMA-CONFORMANCE
    /// @vtest.target crates/vtest-store/src/canonical.rs::document_file_to_json
    /// @vtest.intent An empty source.heading fails closed as a schema mismatch.
    /// `validate_node_source`'s `source.heading` check
    /// (specification.schema.json `$defs/source`, `minLength: 1`) — the
    /// module previously implemented this but had no test exercising it.
    #[test]
    fn document_file_rejects_empty_source_heading() {
        let mut file = minimal_document_file();
        file.request[0].source.heading = String::new();
        document_file_to_json(&file).expect_err("an empty source.heading must fail closed");
    }

    /// @vtest.id TEST-STORE-CANONICAL-DOC-REJECTS-EMPTY-CITES-ENTRY
    /// @vtest.covers VO-MODEL-DOCUMENT-SCHEMA-CONFORMANCE
    /// @vtest.target crates/vtest-store/src/canonical.rs::document_file_to_json
    /// @vtest.intent An empty cites[] entry fails closed as a schema mismatch.
    /// `validate_sentence_node`'s `cites` entry check (schema `minLength: 1`
    /// on each `cites[]` string) — implemented but previously untested.
    #[test]
    fn document_file_rejects_empty_cites_entry() {
        let mut file = minimal_document_file();
        file.request[0].cites = Some(vec![String::new()]);
        document_file_to_json(&file).expect_err("an empty cites entry must fail closed");
    }

    /// @vtest.id TEST-STORE-CANONICAL-DOC-REJECTS-EMPTY-SECTION-TITLE
    /// @vtest.covers VO-MODEL-DOCUMENT-SCHEMA-CONFORMANCE
    /// @vtest.target crates/vtest-store/src/canonical.rs::document_file_to_json
    /// @vtest.intent An empty section title fails closed as a schema mismatch.
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

    /// @vtest.id TEST-STORE-CANONICAL-DOC-REJECTS-NESTED-WRONG-LAYER
    /// @vtest.covers VO-MODEL-DOCUMENT-LAYER-ARRAY-PREFIX-AGREEMENT
    /// @vtest.target crates/vtest-store/src/canonical.rs::document_file_to_json,validate_section_node
    /// @vtest.intent DS-1658's layer/prefix agreement also fires on a node nested two levels deep under sections[]/items[].
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

    /// @vtest.id TEST-STORE-CANONICAL-DOC-REJECTS-DUPLICATE-DERIVES-FROM
    /// @vtest.covers VO-MODEL-DOCUMENT-SCHEMA-CONFORMANCE
    /// @vtest.target crates/vtest-store/src/canonical.rs::document_file_to_json
    /// @vtest.intent A duplicated derives_from entry fails closed (schema uniqueItems).
    #[test]
    fn document_file_rejects_duplicate_derives_from() {
        let mut file = minimal_document_file();
        file.request[0].derives_from =
            vec![DocumentId::new("ROOT-001"), DocumentId::new("ROOT-001")];
        document_file_to_json(&file)
            .expect_err("a duplicated derives_from entry must fail closed (schema uniqueItems)");
    }

    /// @vtest.id TEST-STORE-CANONICAL-DOC-REJECTS-MALFORMED-DERIVES-FROM-SENTENCE
    /// @vtest.covers VO-MODEL-DOCUMENT-SCHEMA-CONFORMANCE
    /// @vtest.target crates/vtest-store/src/canonical.rs::document_file_to_json
    /// @vtest.intent A derives_from entry not matching $defs/id fails closed as E-SCAN-010 on a sentence node.
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

    /// @vtest.id TEST-STORE-CANONICAL-DOC-REJECTS-EMPTY-STRING-DERIVES-FROM
    /// @vtest.covers VO-MODEL-DOCUMENT-SCHEMA-CONFORMANCE
    /// @vtest.target crates/vtest-store/src/canonical.rs::document_file_to_json
    /// @vtest.intent An empty-string derives_from entry fails closed as a schema mismatch.
    #[test]
    fn document_file_rejects_empty_string_derives_from_entry() {
        let mut file = minimal_document_file();
        file.request[0].derives_from = vec![DocumentId::new("")];
        document_file_to_json(&file).expect_err("an empty derives_from entry must fail closed");
    }

    /// @vtest.id TEST-STORE-CANONICAL-DOC-REJECTS-MALFORMED-DERIVES-FROM-SECTION
    /// @vtest.covers VO-MODEL-DOCUMENT-SCHEMA-CONFORMANCE
    /// @vtest.target crates/vtest-store/src/canonical.rs::document_file_to_json,validate_section_node
    /// @vtest.intent A section-level derives_from entry not matching $defs/id fails closed as E-SCAN-010.
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

    /// @vtest.id TEST-STORE-CANONICAL-DOC-REJECTS-MALFORMED-DERIVES-FROM-NESTED-ITEM
    /// @vtest.covers VO-MODEL-DOCUMENT-SCHEMA-CONFORMANCE
    /// @vtest.target crates/vtest-store/src/canonical.rs::document_file_to_json,validate_section_node
    /// @vtest.intent A malformed derives_from entry on a nested items[] sentence also fails closed.
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

    /// @vtest.id TEST-STORE-CANONICAL-DOC-ACCEPTS-CROSS-LAYER-DERIVES-FROM
    /// @vtest.covers VO-MODEL-DOCUMENT-DERIVES-FROM-CROSS-LAYER-ALLOWED
    /// @vtest.target crates/vtest-store/src/canonical.rs::document_file_to_json,validate_section_node
    /// @vtest.intent A derives_from entry pointing at a node in a different layer than the node that holds it is accepted.
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

    /// @vtest.id TEST-STORE-CANONICAL-DOC-JSON-REJECTS-NULL-DESCRIPTION
    /// @vtest.covers VO-MODEL-DOCUMENT-SCHEMA-CONFORMANCE
    /// @vtest.target crates/vtest-store/src/canonical.rs::document_file_from_json
    /// @vtest.intent An explicit JSON null on an optional field fails closed as E-SCAN-010.
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

    /// @vtest.id TEST-STORE-CANONICAL-DOC-JSON-ACCEPTS-ABSENT-DESCRIPTION
    /// @vtest.covers VO-MODEL-DOCUMENT-SCHEMA-CONFORMANCE
    /// @vtest.target crates/vtest-store/src/canonical.rs::document_file_from_json
    /// @vtest.intent An absent optional description field (as opposed to an explicit null) is accepted.
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

    /// @vtest.id TEST-STORE-CANONICAL-DOC-REJECTS-UNKNOWN-TOP-LEVEL-FIXTURE
    /// @vtest.covers VO-MODEL-DOCUMENT-SCHEMA-CONFORMANCE
    /// @vtest.target crates/vtest-store/src/canonical.rs::document_file_from_json
    /// @vtest.intent An unknown top-level field fails closed as a schema mismatch.
    #[test]
    fn document_file_rejects_unknown_top_level_field_fixture() {
        let text = include_str!("../tests/fixtures/document_unknown_field_top_level.json");
        let error =
            document_file_from_json(text).expect_err("an unknown top-level field must fail closed");
        assert!(matches!(error, StoreError::SchemaMismatch { .. }));
    }

    /// @vtest.id TEST-STORE-CANONICAL-DOC-REJECTS-UNKNOWN-SECTION-FIXTURE
    /// @vtest.covers VO-MODEL-DOCUMENT-SCHEMA-CONFORMANCE
    /// @vtest.target crates/vtest-store/src/canonical.rs::document_file_from_json
    /// @vtest.intent An unknown section field fails closed as a schema mismatch.
    #[test]
    fn document_file_rejects_unknown_section_field_fixture() {
        let text = include_str!("../tests/fixtures/document_unknown_field_section.json");
        document_file_from_json(text).expect_err("an unknown section field must fail closed");
    }

    /// @vtest.id TEST-STORE-CANONICAL-DOC-REJECTS-UNKNOWN-SENTENCE-FIXTURE
    /// @vtest.covers VO-MODEL-DOCUMENT-SCHEMA-CONFORMANCE
    /// @vtest.target crates/vtest-store/src/canonical.rs::document_file_from_json
    /// @vtest.intent An unknown sentence field fails closed as a schema mismatch.
    #[test]
    fn document_file_rejects_unknown_sentence_field_fixture() {
        let text = include_str!("../tests/fixtures/document_unknown_field_sentence.json");
        document_file_from_json(text).expect_err("an unknown sentence field must fail closed");
    }

    /// @vtest.id TEST-STORE-CANONICAL-DOC-REJECTS-ANCHOR-NOTE-FIXTURE
    /// @vtest.covers VO-MODEL-DOCUMENT-SCHEMA-CONFORMANCE
    /// @vtest.target crates/vtest-store/src/canonical.rs::document_file_from_json
    /// @vtest.intent A document derives_from entry shaped as {doc, anchor} (VO-only shape, DS-1638) fails closed.
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
    /// @vtest.id TEST-STORE-CANONICAL-DOC-SAMPLE-FIXTURE-ROUND-TRIP
    /// @vtest.covers VO-STORE-CANONICAL-ROUND-TRIP-FIDELITY
    /// @vtest.target crates/vtest-store/src/canonical.rs::document_file_from_json,document_file_to_json,write_document_file,read_document_file
    /// @vtest.intent PR #20's populated sample document round-trips through this crate's reader/writer, in memory and through disk.
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
    /// @vtest.id TEST-STORE-CANONICAL-BUNDLE-ROUND-TRIP
    /// @vtest.covers VO-STORE-CANONICAL-ROUND-TRIP-FIDELITY
    /// @vtest.target crates/vtest-store/src/canonical.rs::document_file_from_json,document_file_to_json
    /// @vtest.intent The real canonical specification.json round-trips through this crate's reader/writer with matching node counts.
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

        // Node counts are asserted for self-consistency, not pinned to a
        // literal observed at one canonical-bundle revision: the canonical
        // specification.json moves forward on its own branch, and a count
        // baked in at one commit turns every subsequent growth of the spec
        // into a false failure here, with no relation to whether this
        // crate's reader regressed. Instead, each layer's count is computed
        // two independent ways from the same bundle text — once by walking
        // the typed `DocumentFile` this store's own reader
        // (`document_file_from_json`) produced, via the `count_sections`
        // walk above, and once by walking a `serde_json::Value` parsed
        // independently from the same text — and the two must agree. This
        // is not redundant with the round-trip assertion below: that
        // assertion shows the store's writer/reader pair is idempotent on
        // this bundle, while this one shows the reader's typed output
        // matches the schema's own node shape, independent of how
        // `SectionNode`'s fields happen to be laid out.
        let raw: serde_json::Value =
            serde_json::from_str(&text).expect("bundle must also parse as a plain JSON value");

        fn count_sections_value(sections: &[serde_json::Value]) -> usize {
            sections
                .iter()
                .map(|s| {
                    let items = s
                        .get("items")
                        .and_then(|v| v.as_array())
                        .map(|a| a.len())
                        .unwrap_or(0);
                    let empty = Vec::new();
                    let nested = s
                        .get("sections")
                        .and_then(|v| v.as_array())
                        .unwrap_or(&empty);
                    1 + items + count_sections_value(nested)
                })
                .sum()
        }

        fn layer_array<'a>(bundle: &'a serde_json::Value, key: &str) -> &'a [serde_json::Value] {
            bundle
                .get(key)
                .and_then(|v| v.as_array())
                .unwrap_or_else(|| panic!("bundle `{key}` is not a JSON array"))
                .as_slice()
        }

        let value_root = layer_array(&raw, "root").len();
        let value_request = layer_array(&raw, "request").len();
        let value_require = count_sections_value(layer_array(&raw, "require"));
        let value_spec = count_sections_value(layer_array(&raw, "spec"));
        let value_detailed_spec = count_sections_value(layer_array(&raw, "detailed_spec"));
        let value_basic_design = count_sections_value(layer_array(&raw, "basic_design"));
        let value_design = count_sections_value(layer_array(&raw, "design"));

        assert_eq!(
            file.root.len(),
            value_root,
            "root layer node count: store reader's typed count vs. independent Value-based count"
        );
        assert_eq!(
            file.request.len(),
            value_request,
            "request layer node count: store reader's typed count vs. independent Value-based count"
        );
        assert_eq!(
            count_sections(&file.require),
            value_require,
            "require layer node count: store reader's typed count vs. independent Value-based count"
        );
        assert_eq!(
            count_sections(&file.spec),
            value_spec,
            "spec layer node count: store reader's typed count vs. independent Value-based count"
        );
        assert_eq!(
            count_sections(&file.detailed_spec),
            value_detailed_spec,
            "detailed_spec layer node count: store reader's typed count vs. independent Value-based count"
        );
        assert_eq!(
            count_sections(&file.basic_design),
            value_basic_design,
            "basic_design layer node count: store reader's typed count vs. independent Value-based count"
        );
        assert_eq!(
            count_sections(&file.design),
            value_design,
            "design layer node count: store reader's typed count vs. independent Value-based count"
        );

        let total = file.root.len()
            + file.request.len()
            + count_sections(&file.require)
            + count_sections(&file.spec)
            + count_sections(&file.detailed_spec)
            + count_sections(&file.basic_design)
            + count_sections(&file.design);

        eprintln!(
            "canonical_bundle_round_trips_through_the_store_reader: observed node counts — \
             root={}, request={}, require={}, spec={}, detailed_spec={}, basic_design={}, \
             design={}, total={} (not pinned to a literal — this run's store-reader typed count \
             matched an independent Value-based count for every layer).",
            file.root.len(),
            file.request.len(),
            count_sections(&file.require),
            count_sections(&file.spec),
            count_sections(&file.detailed_spec),
            count_sections(&file.basic_design),
            count_sections(&file.design),
            total,
        );

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

    /// @vtest.id TEST-STORE-CANONICAL-VO-ROUND-TRIP-YAML
    /// @vtest.covers VO-STORE-CANONICAL-ROUND-TRIP-FIDELITY
    /// @vtest.target crates/vtest-store/src/canonical.rs::vo_record_to_yaml,vo_record_from_yaml
    /// @vtest.intent A VO record round-trips through canonical YAML with no diagnostics.
    #[test]
    fn vo_record_round_trips_through_canonical_yaml() {
        let record = sample_vo();
        let yaml = vo_record_to_yaml(&record);
        let (parsed, diagnostics) = vo_record_from_yaml(&yaml, record.id.as_str()).unwrap();
        assert_eq!(parsed, record);
        assert!(diagnostics.is_empty());
    }

    /// @vtest.id TEST-STORE-CANONICAL-VO-OPTIONAL-FIELDS-ABSENT-ROUND-TRIP
    /// @vtest.covers VO-MODEL-VO-RECORD-SHAPE
    /// @vtest.target crates/vtest-store/src/canonical.rs::vo_record_to_yaml,vo_record_from_yaml
    /// @vtest.intent A VO with every optional field absent (parent, dimensions, coverage_policy, combinations, representative_cases) round-trips.
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

    /// @vtest.id TEST-STORE-CANONICAL-VO-EMPTY-DERIVES-FROM-REJECTED
    /// @vtest.covers VO-MODEL-VO-RECORD-SHAPE
    /// @vtest.target crates/vtest-store/src/canonical.rs::vo_record_from_yaml,write_vo_record
    /// @vtest.intent A VO with zero derives_from entries fails closed on both read and write (SPEC-015 requires >= 1).
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

    /// @vtest.id TEST-STORE-CANONICAL-VO-DUPLICATE-COMBINATION-DIMENSION-REJECTED-ON-WRITE
    /// @vtest.covers VO-SCAN-E-SCAN-017-ENTRY-SHAPE-MISMATCH
    /// @vtest.target crates/vtest-store/src/canonical.rs::write_vo_record,require_no_duplicate_combination_dimension_names
    /// @vtest.intent A combinations[] entry repeating a dimension name is rejected cleanly by the writer, not by a panic inside vo_record_to_yaml.
    /// DS-422/DS-902 (E-SCAN-017 condition 6, 詳細設計 v0.1 本冊:283): a
    /// `combinations[]` entry declaring the same dimension twice must not
    /// reach `vo_record_to_yaml`'s `to_string(record).expect(...)` and
    /// panic — `write_vo_record` must reject it with a clean `StoreError`
    /// first, via `require_no_duplicate_combination_dimension_names`.
    #[test]
    fn vo_record_with_duplicate_combination_dimension_name_is_rejected_on_write() {
        let mut record = sample_vo();
        record.coverage_policy = Some(CoveragePolicy::Explicit);
        record.combinations = vec![CombinationEntry::from_iter(vec![
            ("operand-sign".to_owned(), "positive".to_owned()),
            ("operand-sign".to_owned(), "negative".to_owned()),
        ])];

        let root = std::env::temp_dir().join(format!(
            "vtest-store-canonical-vo-duplicate-dimension-{}",
            crate::new_record_id()
        ));
        let layout = crate::init_project(&root, "example").unwrap();
        let error = write_vo_record(&layout, &record).expect_err(
            "the writer must reject a combinations entry that repeats one dimension name, \
             not panic inside vo_record_to_yaml",
        );
        assert!(
            matches!(&error, StoreError::InvalidConfig(message) if message.contains("operand-sign")),
            "error should name the offending dimension: {error:?}"
        );
    }

    /// @vtest.id TEST-STORE-CANONICAL-VO-DUPLICATE-COMBINATION-DIMENSION-READ-INTACT
    /// @vtest.covers VO-SCAN-E-SCAN-017-ENTRY-SHAPE-MISMATCH
    /// @vtest.target crates/vtest-store/src/canonical.rs::vo_record_from_yaml
    /// @vtest.intent A combinations[] entry repeating a dimension name is read successfully and the duplicate survives losslessly for the scan layer's E-SCAN-017 check.
    /// DS-422/DS-902 (E-SCAN-017 condition 6): the record layer must hand a
    /// `combinations[]` entry with a repeated dimension name to the scan
    /// layer intact — reading it must succeed, and the duplicate must
    /// survive on `CombinationEntry`, rather than the read failing closed as
    /// E-SCAN-010 before `vtest-scan`'s `invalid_vo_combinations` ever sees
    /// it. This is the regression this whole fix exists for: before it, the
    /// initial `yaml_serde::Value` parse rejected the duplicate key at
    /// `combinations[0]` before any `VoRecord` was ever built.
    #[test]
    fn vo_record_combinations_entry_with_duplicate_dimension_key_is_read_intact() {
        let yaml = "\
id: VO-ARITH-001
parent: null
derives_from:
  - doc: DOC-BASIC-001
claim: claim
dimensions:
  - name: operand-sign
    partitions: [positive, negative]
coverage_policy: explicit
combinations:
  - { operand-sign: positive, operand-sign: negative }
representative_cases: []
created: 2026-08-08
updated: 2026-08-08
";
        let (record, diagnostics) = vo_record_from_yaml(yaml, "VO-ARITH-001")
            .expect("a combinations[] entry repeating one dimension name must still be read");
        assert!(diagnostics.is_empty());
        assert_eq!(record.combinations.len(), 1);
        assert_eq!(
            record.combinations[0].duplicate_dimension_names(),
            vec!["operand-sign"],
            "the duplicate must survive losslessly for the scan-layer E-SCAN-017 check"
        );
    }

    /// @vtest.id TEST-STORE-CANONICAL-VO-DUPLICATE-TOP-LEVEL-KEY-REJECTED
    /// @vtest.covers VO-MODEL-VO-RECORD-SCHEMA-CONFORMANCE
    /// @vtest.target crates/vtest-store/src/canonical.rs::vo_record_from_yaml
    /// @vtest.intent A duplicate top-level YAML key (two claim: entries) outside combinations[] fails closed.
    /// The record layer's own protections stay intact for every *other*
    /// field once the primary parse bypasses `yaml_serde::Value`: a
    /// duplicate top-level key (here, two `claim:` entries) is still
    /// rejected by `VoRecord`'s own derived `Deserialize`, independent of
    /// `Value` (see `vtest_model::vo::CombinationEntry`'s doc comment).
    #[test]
    fn vo_record_with_duplicate_top_level_key_is_rejected() {
        let yaml = "\
id: VO-PARSER-UTF8-003
parent: null
derives_from:
  - doc: DOC-BASIC-001
claim: first claim
claim: second claim
dimensions: []
coverage_policy: null
combinations: []
representative_cases: []
created: 2026-08-08
updated: 2026-08-08
";
        vo_record_from_yaml(yaml, "VO-PARSER-UTF8-003")
            .expect_err("a duplicate top-level key outside combinations[] must fail closed");
    }

    /// @vtest.id TEST-STORE-CANONICAL-VO-DUPLICATE-KEY-IN-DERIVES-FROM-ENTRY-REJECTED
    /// @vtest.covers VO-MODEL-VO-RECORD-SCHEMA-CONFORMANCE
    /// @vtest.target crates/vtest-store/src/canonical.rs::vo_record_from_yaml
    /// @vtest.intent A duplicate key inside a single derives_from[] entry fails closed.
    /// Same protection, nested one level down: a duplicate key inside a
    /// single `derives_from[]` entry is rejected by `DerivesFrom`'s own
    /// derived `Deserialize`.
    #[test]
    fn vo_record_with_duplicate_key_in_derives_from_entry_is_rejected() {
        let yaml = "\
id: VO-PARSER-UTF8-003
parent: null
derives_from:
  - doc: DOC-BASIC-001
    doc: DOC-OTHER-001
claim: claim
dimensions: []
coverage_policy: null
combinations: []
representative_cases: []
created: 2026-08-08
updated: 2026-08-08
";
        vo_record_from_yaml(yaml, "VO-PARSER-UTF8-003")
            .expect_err("a duplicate key inside one derives_from[] entry must fail closed");
    }

    /// @vtest.id TEST-STORE-CANONICAL-VO-DUPLICATE-KEY-IN-DIMENSIONS-ENTRY-REJECTED
    /// @vtest.covers VO-MODEL-VO-RECORD-SCHEMA-CONFORMANCE
    /// @vtest.target crates/vtest-store/src/canonical.rs::vo_record_from_yaml
    /// @vtest.intent A duplicate key inside one dimensions[] entry fails closed — combinations[] is the only nested shape that tolerates a repeat.
    /// Same protection, on the other nested shape `vtest_model::vo::
    /// CombinationEntry`'s doc comment names: a duplicate key inside one
    /// `dimensions[]` entry is rejected by `Dimension`'s own derived
    /// `Deserialize`, independent of `Value` — `combinations[]` is the *only*
    /// nested shape that tolerates a repeat.
    #[test]
    fn vo_record_with_duplicate_key_in_dimensions_entry_is_rejected() {
        let yaml = "\
id: VO-ARITH-001
parent: null
derives_from:
  - doc: DOC-BASIC-001
claim: claim
dimensions:
  - name: operand-sign
    name: operator
    partitions: [positive, negative]
coverage_policy: null
combinations: []
representative_cases: []
created: 2026-08-08
updated: 2026-08-08
";
        vo_record_from_yaml(yaml, "VO-ARITH-001")
            .expect_err("a duplicate key inside one dimensions[] entry must fail closed");
    }

    /// @vtest.id TEST-STORE-CANONICAL-VO-COMBINATIONS-DUPLICATE-WITH-UNKNOWN-FIELD-REJECTED
    /// @vtest.covers VO-MODEL-VO-RECORD-SCHEMA-CONFORMANCE
    /// @vtest.target crates/vtest-store/src/canonical.rs::vo_record_from_yaml
    /// @vtest.intent An unrelated unknown top-level field still fails closed even alongside a tolerated combinations[] duplicate.
    /// The unknown-field scan (DS-1645) is not skipped merely because a
    /// `combinations[]` entry also has a tolerated duplicate: an unrelated
    /// unknown top-level field must still fail closed, proving `LenientValue`
    /// building successfully on the duplicate did not also swallow this
    /// check.
    #[test]
    fn vo_record_combinations_duplicate_with_unrelated_unknown_field_is_still_rejected() {
        let yaml = "\
id: VO-ARITH-001
parent: null
derives_from:
  - doc: DOC-BASIC-001
claim: claim
dimensions:
  - name: operand-sign
    partitions: [positive, negative]
coverage_policy: explicit
combinations:
  - { operand-sign: positive, operand-sign: negative }
representative_cases: []
created: 2026-08-08
updated: 2026-08-08
nickname: quick-vo
";
        let error = vo_record_from_yaml(yaml, "VO-ARITH-001").expect_err(
            "an unrelated unknown top-level field must still fail closed alongside a tolerated \
             combinations[] duplicate",
        );
        assert!(matches!(error, StoreError::SchemaMismatch { .. }));
    }

    /// @vtest.id TEST-STORE-CANONICAL-VO-PARSES-DES-117-EXAMPLE
    /// @vtest.covers VO-MODEL-VO-PARENT-FIELD-SHAPE
    /// @vtest.target crates/vtest-store/src/canonical.rs::vo_record_from_yaml
    /// @vtest.intent DES-117's own literal VO example, including its parent field as a VO id, parses and round-trips.
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

    /// @vtest.id TEST-STORE-CANONICAL-VO-PARSES-DES-121-EXAMPLE
    /// @vtest.covers VO-MODEL-VO-COMBINATIONS-ENTRY-SHAPE
    /// @vtest.target crates/vtest-store/src/canonical.rs::vo_record_from_yaml
    /// @vtest.intent DES-121's own example parses combinations[] entries as dimension-name-to-partition-value maps, not positional lists.
    /// DES-121's own `description`, verbatim: each `combinations[]` entry is
    /// a dimension-name → partition-value flow-mapping, not a positional
    /// list of bare strings.
    #[test]
    fn vo_record_parses_the_literal_des_121_example() {
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
        // `CombinationEntry::get` already returns `Option<&str>` (`vtest-model`) — no
        // `.map(String::as_str)` needed; that call does not type-check against `&str`.
        assert_eq!(record.combinations[0].get("operand-sign"), Some("positive"));
        assert_eq!(record.combinations[0].get("operator"), Some("div"));
        assert!(diagnostics.is_empty());
        let roundtrip = vo_record_to_yaml(&record);
        assert_eq!(
            vo_record_from_yaml(&roundtrip, "VO-ARITH-001").unwrap().0,
            record
        );
    }

    /// @vtest.id TEST-STORE-CANONICAL-VO-STATUS-VALUE-IGNORED-PRESENCE-WARNS
    /// @vtest.covers VO-MODEL-VO-STATUS-COMPAT-FIELD
    /// @vtest.target crates/vtest-store/src/canonical.rs::vo_record_from_yaml
    /// @vtest.intent A status field is accepted and its value ignored, but its mere presence emits a W-STORE-001 diagnostic.
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

    /// @vtest.id TEST-STORE-CANONICAL-VO-NO-STATUS-FIELD-NO-DIAGNOSTICS
    /// @vtest.covers VO-MODEL-VO-STATUS-COMPAT-FIELD
    /// @vtest.target crates/vtest-store/src/canonical.rs::vo_record_from_yaml,vo_record_to_yaml
    /// @vtest.intent A record with no status field produces no diagnostics.
    #[test]
    fn vo_record_without_status_field_reports_no_diagnostics() {
        let record = sample_vo();
        let yaml = vo_record_to_yaml(&record);
        assert!(!yaml.contains("status"));
        let (_, diagnostics) = vo_record_from_yaml(&yaml, record.id.as_str()).unwrap();
        assert!(diagnostics.is_empty());
    }

    /// @vtest.id TEST-STORE-CANONICAL-VO-STATUS-PLUS-UNKNOWN-FIELD-REJECTED
    /// @vtest.covers VO-MODEL-VO-RECORD-SCHEMA-CONFORMANCE
    /// @vtest.target crates/vtest-store/src/canonical.rs::vo_record_from_yaml
    /// @vtest.intent A genuinely unknown field fails closed even when the legitimate status read-compat field is also present.
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

    /// @vtest.id TEST-STORE-CANONICAL-VO-UNKNOWN-TOP-LEVEL-FIELD-REJECTED
    /// @vtest.covers VO-MODEL-VO-RECORD-SCHEMA-CONFORMANCE
    /// @vtest.target crates/vtest-store/src/canonical.rs::vo_record_from_yaml
    /// @vtest.intent An unknown top-level field on a VO record fails closed.
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

    /// @vtest.id TEST-STORE-CANONICAL-VO-NON-STRING-TOP-LEVEL-KEY-REJECTED
    /// @vtest.covers VO-MODEL-VO-RECORD-SCHEMA-CONFORMANCE
    /// @vtest.target crates/vtest-store/src/canonical.rs::vo_record_from_yaml
    /// @vtest.intent A non-string top-level YAML key fails closed via one of two independent backstops.
    /// Unlike `ApprovalRecord`/`read_evidence` (`records.rs`), a VO record's
    /// unknown-field scan is always followed by `yaml_serde::from_value::<
    /// VoRecord>` (below), which independently rejects a non-string mapping
    /// key with its own type error — a second, independent backstop behind
    /// `reject_unknown_fields`'s own rejection (this test does not isolate
    /// which of the two rejects first; either failing keeps the record
    /// unread). Locks in that at least one of the two continues to reject
    /// this shape (DS-1645), guarding against the non-string-key gap
    /// reopening if either layer regresses.
    #[test]
    fn vo_record_with_non_string_top_level_key_is_rejected() {
        let record = sample_vo();
        let mut yaml = vo_record_to_yaml(&record);
        yaml.push_str("2026: unlimited\n");
        vo_record_from_yaml(&yaml, record.id.as_str())
            .expect_err("a non-string top-level key must fail closed");
    }

    /// @vtest.id TEST-STORE-CANONICAL-VO-UNKNOWN-NESTED-DERIVES-FROM-FIELD-REJECTED
    /// @vtest.covers VO-MODEL-VO-RECORD-SCHEMA-CONFORMANCE
    /// @vtest.target crates/vtest-store/src/canonical.rs::vo_record_from_yaml
    /// @vtest.intent An unknown nested field inside a derives_from[] entry fails closed.
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

    /// @vtest.id TEST-STORE-CANONICAL-VO-UNKNOWN-NESTED-DIMENSIONS-FIELD-REJECTED
    /// @vtest.covers VO-MODEL-VO-RECORD-SCHEMA-CONFORMANCE
    /// @vtest.target crates/vtest-store/src/canonical.rs::vo_record_from_yaml
    /// @vtest.intent An unknown nested field inside a dimensions[] entry fails closed.
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

    /// @vtest.id TEST-STORE-CANONICAL-VO-ID-DISAGREES-WITH-FILE-NAME-REJECTED
    /// @vtest.covers VO-MODEL-VO-RECORD-SCHEMA-CONFORMANCE
    /// @vtest.target crates/vtest-store/src/canonical.rs::vo_record_from_yaml
    /// @vtest.intent A VO id that disagrees with the file name it was read from fails closed as E-SCAN-010.
    #[test]
    fn vo_record_with_id_disagreeing_with_file_name_is_rejected() {
        let yaml = vo_record_to_yaml(&sample_vo());
        let error = vo_record_from_yaml(&yaml, "VO-OTHER-001")
            .expect_err("a VO id that disagrees with the file name must fail closed");
        assert!(error.to_string().contains("does not match file name"));
    }

    /// @vtest.id TEST-STORE-CANONICAL-VO-MISSING-REQUIRED-FIELD-REJECTED
    /// @vtest.covers VO-MODEL-VO-RECORD-SHAPE
    /// @vtest.target crates/vtest-store/src/canonical.rs::vo_record_from_yaml
    /// @vtest.intent Removing any of id, claim, created, or updated (mandatory VO record fields) fails closed.
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

    /// @vtest.id TEST-STORE-CANONICAL-VO-UNRECOGNIZED-COVERAGE-POLICY-REJECTED
    /// @vtest.covers VO-MODEL-VO-COVERAGE-POLICY-VALUES
    /// @vtest.target crates/vtest-store/src/canonical.rs::vo_record_from_yaml
    /// @vtest.intent An unrecognized coverage_policy value fails closed, not silently becoming None.
    #[test]
    fn vo_record_with_unrecognized_coverage_policy_is_rejected() {
        let yaml = vo_record_to_yaml(&sample_vo())
            .replace("coverage_policy: full-product", "coverage_policy: bogus");
        vo_record_from_yaml(&yaml, "VO-PARSER-UTF8-003").expect_err(
            "an unrecognized coverage_policy must fail closed, not silently become None",
        );
    }

    /// @vtest.id TEST-STORE-CANONICAL-VO-READ-WRITE-ROUND-TRIP-DISK
    /// @vtest.covers VO-STORE-CANONICAL-ROUND-TRIP-FIDELITY
    /// @vtest.target crates/vtest-store/src/canonical.rs::write_vo_record,read_vo_record
    /// @vtest.intent Writing a VO record to disk and reading it back yields the same record.
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

    /// @vtest.id TEST-STORE-CANONICAL-VO-COMBINATIONS-ROUND-TRIP-DIMENSION-KEYED-MAPS
    /// @vtest.covers VO-STORE-CANONICAL-ROUND-TRIP-FIDELITY
    /// @vtest.target crates/vtest-store/src/canonical.rs::vo_record_to_yaml,vo_record_from_yaml
    /// @vtest.intent Multiple dimensions with a combinations[] entry round-trip as dimension-keyed maps.
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
        record.combinations = vec![CombinationEntry::from_iter([
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
