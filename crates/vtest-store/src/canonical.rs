//! Canonical v0.1 record storage (詳細設計 v0.1 §3), for the types defined in
//! `vtest-model`. Kept separate from `records.rs`'s predecessor Req/Spec-model
//! types so neither module's exports collide with the other's.
//!
//! Serialization goes through `yaml_serde` rather than the hand-rolled
//! scalar/list helpers `records.rs` uses for predecessor types: those
//! helpers silently mis-parse inline comments and flow-mappings (both of
//! which 詳細設計 v0.1's own YAML examples use), so canonical types rely on
//! their existing `Serialize`/`Deserialize` derives (shared with their JSON
//! representation) instead.

use crate::{read_text, write_atomic, StoreError, VerifyLayout};
use vtest_model::{DerivesFrom, Diagnostic, DocumentRecord, VoRecord};

/// Known top-level keys for a canonical document record (詳細設計 v0.1
/// §3.1). Kept in sync with `DocumentRecord`'s own fields by
/// `document_known_keys_match_the_record_shape` (below, in `#[cfg(test)]`):
/// a field added to the struct without updating this list would otherwise
/// go unwarned rather than loudly wrong.
const DOCUMENT_KEYS: &[&str] = &[
    "id",
    "path",
    "content_hash",
    "title",
    "derives_from",
    "registered_at",
];

/// Known keys for one `derives_from[]` entry (詳細設計 v0.1 §3.1/§3.2),
/// shared by document and VO records.
const DERIVES_FROM_KEYS: &[&str] = &["doc", "anchor", "note"];

/// Known top-level keys for a canonical VO record (詳細設計 v0.1 §3.2).
/// `status` is listed as *known* here even though `VoRecord` has no such
/// field: its presence already gets its own, more specific W-STORE-001
/// diagnostic (詳細設計 v0.1 §3.2 L235-237); listing it here stops the same
/// key from also being reported as a generic W-STORE-007.
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

/// Known keys for one `dimensions[]` entry (詳細設計 v0.1 §3.2).
const DIMENSION_KEYS: &[&str] = &["name", "partitions"];

/// Minimal shape a diagnostics-scan value must provide: its string-keyed
/// mapping entries, its elements if it is a sequence, and lookup by key.
/// Implemented by `LenientValue` (below), which every §3 record reader
/// (`document_from_yaml`, `vo_record_from_yaml`, `RelationRecord::from_yaml`)
/// now scans through — see `LenientValue`'s own doc comment for why none of
/// them use `yaml_serde::Value` for this anymore. Kept as a trait, rather
/// than each scan function naming `LenientValue` directly, so a future value
/// shape could plug into the same scan without duplicating it; factoring the
/// scan functions below over this trait, instead of duplicating each one per
/// value type, keeps the W-STORE-007/derives_from[]/dimensions[] scan logic
/// in one place — the diagnostic message text and traversal order only need
/// to be right once.
pub(crate) trait DiagnosticValue: Sized {
    /// This value's string-keyed mapping entries, in occurrence order,
    /// skipping any non-string key. Empty (not an error) if this value
    /// isn't a mapping at all — a type mismatch there is instead caught,
    /// fail-closed, by the typed struct deserialize each reader runs
    /// alongside this scan.
    fn mapping_entries(&self) -> Vec<(&str, &Self)>;
    /// This value's elements, if it is a sequence.
    fn as_sequence(&self) -> Option<&[Self]>;
    /// The value under `key`, if this is a mapping containing it.
    fn get(&self, key: &str) -> Option<&Self>;
}

/// Scans a YAML mapping for keys outside `known`, returning one
/// `W-STORE-007` diagnostic per unknown key, in YAML occurrence order.
/// 詳細設計 v0.1 §3 header (L185): "すべてのレコードは YAML とし、未知
/// フィールドはエラーではなく警告とする" — the record is still read (this
/// function never returns an `Err`), only warned about. `prefix` is
/// prepended to each reported key so a nested unknown key (e.g. inside
/// `derives_from[0]`) reads distinctly from a top-level one.
///
/// `pub(crate)` (rather than private to this module) so `records.rs`'s
/// `RelationRecord::from_yaml` — a §3.3 reader that lives outside this
/// module for historical reasons (see that file's module doc comment) —
/// can apply the same §3-header rule instead of re-implementing the scan.
pub(crate) fn unknown_field_diagnostics<V: DiagnosticValue>(
    value: &V,
    known: &[&str],
    prefix: &str,
) -> Vec<Diagnostic> {
    value
        .mapping_entries()
        .into_iter()
        .filter(|(key, _)| !known.contains(key))
        .map(|(key, _)| {
            Diagnostic::warning(
                "W-STORE-007",
                format!("unknown field `{prefix}{key}` is not part of the §3 schema; its value is ignored"),
            )
        })
        .collect()
}

/// Extends the unknown-field scan into each `derives_from[]` entry — the
/// nested shape §3.1 (document) and §3.2 (VO) share.
fn derives_from_diagnostics<V: DiagnosticValue>(value: &V) -> Vec<Diagnostic> {
    let Some(sequence) = value.get("derives_from").and_then(V::as_sequence) else {
        return Vec::new();
    };
    sequence
        .iter()
        .enumerate()
        .flat_map(|(index, entry)| {
            unknown_field_diagnostics(entry, DERIVES_FROM_KEYS, &format!("derives_from[{index}]."))
        })
        .collect()
}

/// Extends the unknown-field scan into each VO `dimensions[]` entry (詳細設計
/// v0.1 §3.2). `combinations[]` is deliberately not scanned the same way:
/// each entry's keys are the *dimension names themselves* (a dynamic,
/// record-specific vocabulary), not a fixed schema — whether a combination
/// covers exactly the declared dimensions is E-SCAN-017, a chain_integrity/
/// scan-time concern this record-level reader does not have the dimension
/// set resolved enough to evaluate (the same record-vs-scan layer split
/// `require_at_least_one_derives_from` below documents for E-SCAN-012).
fn dimensions_diagnostics<V: DiagnosticValue>(value: &V) -> Vec<Diagnostic> {
    let Some(sequence) = value.get("dimensions").and_then(V::as_sequence) else {
        return Vec::new();
    };
    sequence
        .iter()
        .enumerate()
        .flat_map(|(index, entry)| {
            unknown_field_diagnostics(entry, DIMENSION_KEYS, &format!("dimensions[{index}]."))
        })
        .collect()
}

/// Serializes a `DocumentRecord` to its canonical `.verify/doc/DOC-*.yaml`
/// shape (詳細設計 v0.1 §3.1) via `yaml_serde`, using `DocumentRecord`'s own
/// `Serialize` derive.
pub fn document_to_yaml(record: &DocumentRecord) -> String {
    yaml_serde::to_string(record).expect("DocumentRecord always serializes to valid YAML")
}

/// Parses a `DocumentRecord` from its canonical YAML representation,
/// returning any non-fatal diagnostics alongside it. `yaml_serde::from_str`
/// already fails closed on a missing required field, a malformed value, or a
/// duplicate key on one of `DocumentRecord`'s own (nested) fields, via its
/// `Deserialize` derive; this adds the id/file-name check the derive cannot
/// express (matching the strictness `RelationRecord`/`ApprovalRecord`/
/// `AuditRecord` already apply), plus the unknown-field scan 詳細設計 v0.1
/// §3 header (L185) requires of every record type.
///
/// **Parses straight from `text` into `DocumentRecord`, not through
/// `yaml_serde::Value` first**, for the same reason `vo_record_from_yaml`
/// does (see that function's own doc comment for the fuller rationale,
/// established by BLOCKER A / PR #26 review round 2 for VO, then extended
/// here to document/Relation by PR #26 round 3): `yaml_serde::Value`'s
/// `Mapping` rejects a duplicate key *anywhere* in the document tree, not
/// only inside a field this reader actually parses into typed structure. A
/// duplicate key confined to the value of a field §3.1 does not recognize —
/// which this reader would otherwise warn about (W-STORE-007) and still read
/// — used to reject the whole record instead, the same fail-closed-too-far
/// shape `vo_record_from_yaml` closed for `combinations[]`. The unknown-field
/// diagnostics scan below runs against a separately-built `LenientValue`
/// (`yaml_serde::from_str::<LenientValue>(text)`, `?`-propagated — not an
/// `if let Ok(...)` catch-all, so an unexpected failure there still fails the
/// whole read closed) instead, which cannot itself fail to build on a
/// duplicate key anywhere.
pub fn document_from_yaml(
    text: &str,
    fallback_id: &str,
) -> Result<(DocumentRecord, Vec<Diagnostic>), StoreError> {
    let record: DocumentRecord = yaml_serde::from_str(text)
        .map_err(|error| StoreError::InvalidConfig(format!("invalid document record: {error}")))?;
    if record.id.as_str() != fallback_id {
        return Err(StoreError::InvalidConfig(format!(
            "document id {} does not match file name {fallback_id}",
            record.id.as_str()
        )));
    }

    let value: LenientValue = yaml_serde::from_str(text)
        .map_err(|error| StoreError::InvalidConfig(format!("invalid document record: {error}")))?;
    let mut diagnostics = unknown_field_diagnostics(&value, DOCUMENT_KEYS, "");
    diagnostics.extend(derives_from_diagnostics(&value));

    Ok((record, diagnostics))
}

/// Reads the canonical document record `<id>.yaml` from `.verify/doc/`.
pub fn read_document(
    layout: &VerifyLayout,
    id: &str,
) -> Result<(DocumentRecord, Vec<Diagnostic>), StoreError> {
    let path = layout.doc_dir().join(format!("{id}.yaml"));
    let text = read_text(&path)?;
    document_from_yaml(&text, id)
}

/// Writes (or overwrites) the canonical document record to `.verify/doc/`.
/// Documents are mutable-in-place: 基本仕様 §24.2 lists only Relation /
/// decision / approval / Evidence as append-only-only, so document (like VO)
/// follows the general one-record-one-file edit model.
pub fn write_document(layout: &VerifyLayout, record: &DocumentRecord) -> Result<(), StoreError> {
    let path = layout
        .doc_dir()
        .join(format!("{}.yaml", record.id.as_str()));
    write_atomic(&path, &document_to_yaml(record))
}

/// Serializes a canonical `VoRecord` to its `.verify/vo/VO-*.yaml` shape
/// (詳細設計 v0.1 §3.2) via `yaml_serde`. Distinct name from `read_vo`/
/// `VoRecord` (records.rs) on purpose: that pair still serves the
/// predecessor store-side `VoRecord` until PR8 retires it, and the two types
/// are not interchangeable.
pub fn vo_record_to_yaml(record: &VoRecord) -> String {
    yaml_serde::to_string(record).expect("VoRecord always serializes to valid YAML")
}

/// A duplicate-tolerant mirror of `yaml_serde::Value`'s mapping/sequence
/// shape, used by every §3 record reader's diagnostics scan —
/// `vo_record_from_yaml` (below), `document_from_yaml`, and
/// `RelationRecord::from_yaml` (`records.rs`) all build one from `text`
/// independently of their primary typed-struct parse. Originally added for
/// VO alone; PR #26 round 3 found the same asymmetry document/Relation had
/// not yet closed (see the "Extended to document/Relation" note at the end
/// of this comment) and switched all three readers to it, so they now agree
/// on this input shape instead of only VO doing so.
///
/// **Corrects `61ba379`** (PR #26 round 1's BLOCKER 1 fix): that commit's
/// message claimed the diagnostics scan below could fail to run "only
/// possible now when combinations\[\] has a duplicate key, since Value
/// structurally cannot represent that". PR #26 round 2 review measured this
/// empirically against a scratch harness calling `vo_record_from_yaml`
/// directly and found it false: `yaml_serde::Value`'s own `Mapping`
/// (`mapping.rs`) rejects *any* duplicate key it finds while deserializing,
/// at *any* depth in the document tree — not only inside `combinations[]`.
/// Building a `yaml_serde::Value` for the whole VO record text therefore
/// failed not only for the one shape `VoRecord`'s own typed parse cannot
/// itself reject (a repeated dimension name confined to one
/// `combinations[]` entry — see `CombinationEntry`'s doc comment), but for a
/// duplicate key *anywhere else* in the document too — including inside the
/// value of a field the §3.2 schema does not even recognize (an unrelated
/// unknown top-level key whose own value happens to be a YAML mapping with
/// a repeated key, or the read-compat `status` field's value, or an unknown
/// key nested inside a `dimensions[]` entry). Round 1's fix routed every one
/// of those cases through the same `if let Ok(value) = ... else { skip }`,
/// silently dropping W-STORE-007/W-STORE-001/the `derives_from[]`/
/// `dimensions[]` nested scans for a record that otherwise reads (and
/// should warn) fine — a broader loss than the round-1 commit message or
/// the PR's "Known gaps" section described (詳細設計 v0.1 本冊:185, :890,
/// :884, :237).
///
/// `LenientValue` mirrors just the shape the scan below needs — mapping (as
/// ordered key/value pairs, keeping every occurrence instead of rejecting a
/// repeat, the same technique `CombinationEntry` already uses for the same
/// reason), sequence, and "anything else" (a bare scalar, kept only far
/// enough to distinguish it from a mapping/sequence) — built directly from
/// `MapAccess`/`SeqAccess` rather than delegating to `yaml_serde::Mapping`/
/// `Sequence`, so *no* level of the tree can fail on a duplicate key. This
/// is not a general-purpose YAML value type (a tagged scalar, e.g.
/// `!!binary ...`, falls back to `deserialize_any`'s default `visit_enum`
/// error — nothing in §3.2's schema uses YAML tags, and failing closed on
/// one is preferable to guessing at its shape) — it only needs to answer
/// "is this a mapping/sequence, and what does it contain", which is all
/// `unknown_field_diagnostics`/`derives_from_diagnostics`/
/// `dimensions_diagnostics` (via `DiagnosticValue`, above) ask of it.
///
/// Because this never fails on a duplicate key anywhere, `vo_record_from_
/// yaml` below no longer needs to distinguish "the one tolerated shape" from
/// "every other reason `Value` could fail to build" — there is no longer a
/// second reason. Its own `yaml_serde::from_str::<LenientValue>(text)` call
/// is `?`-propagated, not swallowed by an `if let Ok(...)`: if it ever did
/// fail (it should not, for any text the primary `VoRecord` parse already
/// accepted — the two parses see the same event stream, and this type
/// handles every scalar/map/seq shape that stream can produce except a
/// tagged value), the whole read now fails closed instead of silently
/// returning zero diagnostics.
///
/// **Extended to document/Relation (PR #26 round 3).** `document_from_yaml`
/// and `RelationRecord::from_yaml` had no `combinations[]`-shaped field that
/// forced them off a `Value`-first parse the way VO's did, so round 1/2 left
/// them on the original `text -> yaml_serde::Value -> known-key scan -> typed
/// struct` shape (see each function's own doc comment history). But that
/// shape has the exact defect this type was built to fix, independent of
/// `combinations[]`: an unrelated *unknown* field's value containing a
/// duplicate key made the whole `Value` build fail before `from_value` ever
/// ran, rejecting a document/Relation record that would otherwise read fine
/// and only warrant a W-STORE-007 for the unknown field itself — the same
/// asymmetry `vo_record_from_yaml` no longer has, and every §3 record type
/// is bound by the same §3 header rule (詳細設計 v0.1 本冊:185: "すべての
/// レコードは...未知フィールドはエラーではなく警告とする", worded without a
/// per-record-type carve-out). Both readers now parse straight from `text`
/// into their typed struct first (so a duplicate key on a field they
/// actually recognize still rejects, via that struct's own derived
/// `Deserialize` — unchanged), then build a `LenientValue` independently for
/// the diagnostics-only scan, exactly like `vo_record_from_yaml` below.
pub(crate) enum LenientValue {
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
}

impl DiagnosticValue for LenientValue {
    fn mapping_entries(&self) -> Vec<(&str, &Self)> {
        self.as_mapping()
            .into_iter()
            .flatten()
            .filter_map(|(key, value)| key.as_str().map(|key| (key, value)))
            .collect()
    }

    fn as_sequence(&self) -> Option<&[Self]> {
        match self {
            LenientValue::Sequence(items) => Some(items.as_slice()),
            _ => None,
        }
    }

    fn get(&self, key: &str) -> Option<&Self> {
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
        }

        deserializer.deserialize_any(LenientVisitor)
    }
}

/// Parses a canonical `VoRecord` from its YAML representation, returning any
/// non-fatal diagnostics alongside it. `yaml_serde::from_str::<VoRecord>`
/// fails closed on a missing required field (`claim`/`created`/`updated`/
/// etc.) or an unrecognized `coverage_policy` value via `VoRecord`'s
/// `Deserialize` derive; this adds the id/file-name check the derive cannot
/// express, plus the `derives_from` cardinality floor (詳細設計 v0.1 §3.2).
///
/// **Two independent parses, not one `Value`-first pass** (a shape every
/// other canonical record reader also uses now — see `document_from_yaml`/
/// `RelationRecord::from_yaml`'s own doc comments; VO moved off `Value`-first
/// first, for the combinations[]-specific reason below, and PR #26 round 3
/// then found document/Relation needed the same move for a more general
/// reason). BLOCKER 1 (PR #26 review round 1):
/// 詳細設計 v0.1 本冊:283（§3.2.1 の受理条件、条件6）"entry が宣言済み
/// dimension のいずれかを欠く、または同じ dimension 名を2回以上持つ" is one
/// `E-SCAN-017` condition (`vtest-scan`'s `invalid_vo_combinations`, VO
/// retained with `chain_integrity = MISMATCH` — 本冊:1625・別紙A:438), a
/// scan-layer judgment. But a `combinations[]` entry with a repeated
/// dimension name is exactly the kind of YAML mapping
/// `yaml_serde::Value`'s own `Mapping` type structurally cannot hold — its
/// `Deserialize` rejects *any* duplicate key anywhere in the document
/// before a `VoRecord` (or anything else) is ever built from it. Routing
/// every VO record through `Value` first — as this function used to —
/// would make that record-layer rejection (E-SCAN-010, the VO dropped from
/// `vtest-scan`'s `vos` map entirely) pre-empt the scan-layer judgment the
/// spec assigns this condition to, for a `combinations[]`-confined
/// duplicate.
///
/// So `VoRecord` is now built directly from `text`
/// (`yaml_serde::from_str::<VoRecord>`), bypassing `Value` for the primary
/// parse. `VoRecord.combinations`'s element type, `CombinationEntry`
/// (`vtest-model`), has a hand-written `Deserialize` that preserves a
/// repeated dimension name instead of rejecting or silently collapsing it
/// (see that type's own doc comment) — the one field in `VoRecord`'s tree
/// that needs this. Every other field keeps rejecting a duplicate key on
/// its own, with no help from `Value`: a `struct`'s derived `Deserialize`
/// (`VoRecord` itself, `DerivesFrom`, `Dimension`) tracks each field and
/// errors the second time it sees one, independent of `Value` — confirmed
/// empirically, not just inferred (see `vo_record_with_a_duplicate_top_
/// level_key_outside_combinations_is_still_rejected`, `vo_record_with_a_
/// duplicate_key_in_a_derives_from_entry_is_still_rejected`, and
/// `vo_record_with_a_duplicate_key_in_a_dimensions_entry_is_still_rejected`
/// below). `combinations`'s old element type, a bare `BTreeMap<String,
/// String>`, was the one field that fell through that protection — a plain
/// map's `Deserialize` has no duplicate-key check of its own, so read
/// directly (bypassing `Value`) it used to silently keep only the last
/// value for a repeated key.
///
/// **The unknown-field diagnostics this function also returns** (W-STORE-
/// 007, the `status` read-compat warning, `derives_from[]`/`dimensions[]`
/// nested unknown-field scans) run against `LenientValue` (above), not
/// `yaml_serde::Value` — built from `text` independently of the primary
/// parse, since that scan walks arbitrary keys `VoRecord`'s own fixed
/// schema does not know about, which a typed struct parse alone cannot
/// surface. Unlike PR #26 round 1's `yaml_serde::Value` attempt,
/// `LenientValue` cannot itself fail to build on a duplicate key anywhere
/// in the document (see its own doc comment for why, and for the round-1
/// commit-message correction) — so this no longer costs anything: every
/// case round 2 review measured empirically (an unrelated unknown top-level
/// field, `status`, or an unknown nested `dimensions[]` key, each paired
/// with a `combinations[]` duplicate or not) now reaches its diagnostic.
/// `vo_record_combination_entry_with_a_duplicate_dimension_key_reaches_
/// scan_as_e_scan_017` and `vo_record_with_a_combinations_duplicate_and_an_
/// unrelated_unknown_field_still_warns` below lock this in.
pub fn vo_record_from_yaml(
    text: &str,
    fallback_id: &str,
) -> Result<(VoRecord, Vec<Diagnostic>), StoreError> {
    let record: VoRecord = yaml_serde::from_str(text)
        .map_err(|error| StoreError::InvalidConfig(format!("invalid VO record: {error}")))?;
    if record.id.as_str() != fallback_id {
        return Err(StoreError::InvalidConfig(format!(
            "VO id {} does not match file name {fallback_id}",
            record.id.as_str()
        )));
    }
    require_at_least_one_derives_from(&record.derives_from)?;

    let value: LenientValue = yaml_serde::from_str(text)
        .map_err(|error| StoreError::InvalidConfig(format!("invalid VO record: {error}")))?;
    let mut diagnostics = Vec::new();
    if value.get("status").is_some() {
        diagnostics.push(Diagnostic::warning(
            "W-STORE-001",
            "VO record has the non-canonical read-compat field `status`; its value is ignored — effective state and the VO subject hash are derived from approvals instead",
        ));
    }
    diagnostics.extend(unknown_field_diagnostics(&value, VO_KEYS, ""));
    diagnostics.extend(derives_from_diagnostics(&value));
    diagnostics.extend(dimensions_diagnostics(&value));
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
/// in place, for the same reason as `write_document` above. Enforces the
/// same `derives_from` cardinality floor as the reader: a writer that
/// skipped this check could produce a record `read_vo_record` would then
/// reject, which fail-closed reading alone does not prevent. Also enforces
/// `require_no_duplicate_combination_dimension_names` for the same reason,
/// applied to `combinations` (BLOCKER 1 condition 2, PR #26 review round 2):
/// `CombinationEntry` can *hold* a repeated dimension name (so the reader
/// can hand a malformed record to the scan layer losslessly), but the
/// writer must not be able to freshly persist one — a reader that tolerates
/// what a writer can also produce is not a read-only accommodation anymore,
/// it is a normalized shape.
pub fn write_vo_record(layout: &VerifyLayout, record: &VoRecord) -> Result<(), StoreError> {
    require_at_least_one_derives_from(&record.derives_from)?;
    require_no_duplicate_combination_dimension_names(&record.combinations)?;
    let path = layout.vo_dir().join(format!("{}.yaml", record.id.as_str()));
    write_atomic(&path, &vo_record_to_yaml(record))
}

/// 詳細設計 v0.1 本冊:283（§3.2.1 条件6後半）"同じ dimension 名を2回以上
/// 持つ" entry を持つ `combinations` は、reader が読み取れる不正な状態
/// （`vtest-scan`の E-SCAN-017 が扱う）であって、writer が新たに書き出して
/// よい状態ではない。`CombinationEntry`はこの状態を読み取り時に losslessly
/// 保持できるよう意図的に許容する型だが（`vo_record_from_yaml`参照）、
/// 書き込み時にまで許すと「reader だけ直して writer が緩い」という非対称
/// になる（このリポジトリで繰り返し指摘されてきた欠陥クラス）。
/// `CombinationEntry`自身の`Serialize`も同じ状態を拒否する
/// （`yaml_serde::to_string`が`Err`を返す）ため、この関数を経由しない
/// `vo_record_to_yaml`の直接呼び出しも（`.expect(...)`のpanicという形で
/// はあるが）無防備ではない。ここでは`write_vo_record`という実際の
/// ディスク書き込み経路で、panicではなくきれいな`StoreError`として
/// 早期に拒否する。
fn require_no_duplicate_combination_dimension_names(
    combinations: &[vtest_model::CombinationEntry],
) -> Result<(), StoreError> {
    for combination in combinations {
        let duplicates = combination.duplicate_dimension_names();
        if !duplicates.is_empty() {
            return Err(StoreError::InvalidConfig(format!(
                "VO combinations entry declares dimension `{}` more than once; \
                 the writer refuses to persist a record it cannot read back losslessly",
                duplicates.join("`, `")
            )));
        }
    }
    Ok(())
}

/// 詳細設計 v0.1 §3.2: "VO は 1 件以上の `document` から `derives_from` で
/// 導出される" — unlike document's `derives_from` (0 or more, an empty list
/// marks a root candidate), a VO's `derives_from` must be non-empty. This
/// checks only cardinality: whether each entry's `doc` resolves to a document
/// that actually exists is E-SCAN-012 (§3.2 L230), a chain_integrity/scan-time
/// concern this record-level reader/writer does not have the document set to
/// evaluate.
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
    use vtest_model::{ContentHash, CoveragePolicy, Dimension, DocumentId, VoId};

    fn sample_document() -> DocumentRecord {
        DocumentRecord {
            id: DocumentId::new("DOC-BASIC-001"),
            path: "docs/basic-spec.md".to_string(),
            content_hash: ContentHash::from_text("basic spec contents"),
            title: Some("基本仕様書".to_string()),
            derives_from: vec![DerivesFrom {
                doc: DocumentId::new("DOC-REQ-001"),
                anchor: Some("§12.3".to_string()),
                note: None,
            }],
            registered_at: "2026-08-08T00:00:00Z".to_string(),
        }
    }

    #[test]
    fn document_round_trips_through_canonical_yaml() {
        let record = sample_document();
        let yaml = document_to_yaml(&record);
        let (parsed, diagnostics) = document_from_yaml(&yaml, record.id.as_str()).unwrap();
        assert_eq!(parsed, record);
        assert!(diagnostics.is_empty());
    }

    #[test]
    fn document_without_title_or_derives_from_round_trips() {
        let record = DocumentRecord {
            id: DocumentId::new("DOC-ROOT-001"),
            path: "docs/root.md".to_string(),
            content_hash: ContentHash::from_text("root contents"),
            title: None,
            derives_from: vec![],
            registered_at: "2026-08-08T00:00:00Z".to_string(),
        };
        let yaml = document_to_yaml(&record);
        assert_eq!(
            document_from_yaml(&yaml, record.id.as_str()).unwrap().0,
            record
        );
    }

    /// 詳細設計 v0.1 §3.1's own example (L193-201), verbatim including its
    /// inline comments — the fixture the hand-rolled parser used to
    /// silently corrupt. Only `content_hash`'s value is substituted: the
    /// spec itself writes it as the documentation placeholder
    /// `"sha256:..."`, which is not a parseable hash.
    #[test]
    fn document_parses_the_literal_spec_example() {
        let yaml = "\
id: DOC-BASIC-001
path: docs/basic-spec.md        # プロジェクト相対パス
content_hash: \"sha256:9f2c1a4e5b6d7c8f9a0b1c2d3e4f5061728394a5b6c7d8e9f0a1b2c3d4e5f60\"      # 登録時の内容ハッシュ（§1.3 document subject）
title: 基本仕様書               # 任意の表示名
derives_from:                   # 上流 document への導出リンク（0件可＝根候補）
  - doc: DOC-REQ-001
    anchor: \"§12.3\"             # 任意の上流該当箇所（節番号等・空可・非 MISMATCH）
    note: \"\"                    # 任意の導出理由（空可・非 MISMATCH。基本仕様 §3.4）
registered_at: 2026-08-08T00:00:00Z
";
        let (record, diagnostics) = document_from_yaml(yaml, "DOC-BASIC-001").unwrap();
        assert_eq!(record.id.as_str(), "DOC-BASIC-001");
        assert_eq!(record.path, "docs/basic-spec.md");
        assert_eq!(record.title.as_deref(), Some("基本仕様書"));
        assert_eq!(record.derives_from[0].doc.as_str(), "DOC-REQ-001");
        assert_eq!(record.derives_from[0].anchor.as_deref(), Some("§12.3"));
        assert_eq!(record.derives_from[0].note.as_deref(), Some(""));
        assert!(diagnostics.is_empty());
        let roundtrip = document_to_yaml(&record);
        assert_eq!(
            document_from_yaml(&roundtrip, "DOC-BASIC-001").unwrap().0,
            record
        );
    }

    #[test]
    fn document_with_id_disagreeing_with_file_name_is_rejected() {
        let yaml = document_to_yaml(&sample_document());
        let error = document_from_yaml(&yaml, "DOC-OTHER-001")
            .expect_err("a document id that disagrees with the file name must fail closed");
        assert!(error.to_string().contains("does not match file name"));
    }

    #[test]
    fn document_missing_a_required_field_is_rejected() {
        let yaml = document_to_yaml(&sample_document());
        for key in ["id", "path", "content_hash", "registered_at"] {
            let without_field = yaml
                .lines()
                .filter(|line| !line.starts_with(&format!("{key}:")))
                .collect::<Vec<_>>()
                .join("\n");
            assert!(
                document_from_yaml(&without_field, "DOC-BASIC-001").is_err(),
                "expected a document missing `{key}` to fail closed"
            );
        }
    }

    /// `document_from_yaml`'s primary parse builds `DocumentRecord` directly
    /// from `text` (PR #26 round 3), not through `yaml_serde::Value` first —
    /// but a duplicate top-level key (every key here is one `DocumentRecord`
    /// actually has) is still rejected, via `DocumentRecord`'s own derived
    /// `Deserialize`: a `struct` visitor tracks each field and errors the
    /// second time it sees one, independent of `Value`.
    #[test]
    fn document_with_a_duplicate_top_level_key_is_rejected() {
        let yaml = document_to_yaml(&sample_document());
        let mut duplicated = yaml.clone();
        duplicated.push_str(&yaml);
        let error = document_from_yaml(&duplicated, "DOC-BASIC-001")
            .expect_err("a document YAML with every top-level key duplicated must fail closed");
        assert!(
            error.to_string().contains("duplicate field"),
            "expected a duplicate-key parse rejection, got: {error}"
        );
    }

    /// Same carve-out boundary, one level deeper: a duplicate key inside a
    /// `derives_from[]` entry (a nested `struct`, `DerivesFrom`) is also
    /// still rejected, via that struct's own derived `Deserialize`.
    #[test]
    fn document_with_a_duplicate_key_in_a_derives_from_entry_is_rejected() {
        let yaml = "\
id: DOC-BASIC-001
path: docs/basic-spec.md
content_hash: \"sha256:9f2c1a4e5b6d7c8f9a0b1c2d3e4f5061728394a5b6c7d8e9f0a1b2c3d4e5f60\"
derives_from:
  - doc: DOC-REQ-001
    doc: DOC-OTHER-001
registered_at: 2026-08-08T00:00:00Z
";
        let error = document_from_yaml(yaml, "DOC-BASIC-001")
            .expect_err("a duplicate key in a derives_from[] entry must fail closed");
        assert!(
            error.to_string().contains("duplicate field"),
            "expected a duplicate-key parse rejection, got: {error}"
        );
    }

    /// PR #26 round 3: the defect the earlier `Value`-first parse had for
    /// `vo_record_from_yaml` (BLOCKER A, PR #26 review round 2) applied to
    /// `document_from_yaml` too, for the same reason — an unrelated *unknown*
    /// top-level key whose own value is a mapping with an internally
    /// duplicated key used to reject the whole document, even though the
    /// document itself (and `owner`'s presence) reads fine and should only
    /// warn W-STORE-007. `document_from_yaml` now scans with `LenientValue`
    /// (see its own doc comment), which cannot fail to build on a duplicate
    /// key anywhere, so the warning is no longer lost.
    #[test]
    fn document_with_unknown_top_level_field_whose_value_has_a_duplicate_key_still_warns() {
        let record = sample_document();
        let mut yaml = document_to_yaml(&record);
        yaml.push_str("owner:\n  x: 1\n  x: 2\n");
        let (parsed, diagnostics) = document_from_yaml(&yaml, record.id.as_str()).expect(
            "an unrelated unknown field's internally-duplicated value must not fail the read",
        );
        assert_eq!(parsed, record);
        assert_eq!(diagnostics.len(), 1, "{diagnostics:?}");
        assert_eq!(diagnostics[0].code, "W-STORE-007");
        assert!(diagnostics[0].message.contains("owner"));
    }

    #[test]
    fn document_read_write_round_trips_through_disk() {
        let root = std::env::temp_dir().join(format!(
            "vtest-store-canonical-doc-{}",
            crate::new_record_id()
        ));
        let layout = crate::init_project(&root, "example").unwrap();
        let record = sample_document();

        write_document(&layout, &record).unwrap();
        assert_eq!(
            read_document(&layout, record.id.as_str()).unwrap().0,
            record
        );
    }

    /// 詳細設計 v0.1 §3 header (L185): an unknown field warns, it does not
    /// stop the record from being read.
    #[test]
    fn document_with_unknown_top_level_field_warns_and_still_reads() {
        let record = sample_document();
        let mut yaml = document_to_yaml(&record);
        yaml.push_str("owner: someone\n");
        let (parsed, diagnostics) = document_from_yaml(&yaml, record.id.as_str()).unwrap();
        assert_eq!(parsed, record);
        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics[0].code, "W-STORE-007");
        assert!(diagnostics[0].message.contains("owner"));
    }

    #[test]
    fn document_with_unknown_nested_derives_from_field_warns_with_path() {
        let yaml = "\
id: DOC-BASIC-001
path: docs/basic-spec.md
content_hash: \"sha256:9f2c1a4e5b6d7c8f9a0b1c2d3e4f5061728394a5b6c7d8e9f0a1b2c3d4e5f60\"
derives_from:
  - doc: DOC-REQ-001
    foo: bar
registered_at: 2026-08-08T00:00:00Z
";
        let (_record, diagnostics) = document_from_yaml(yaml, "DOC-BASIC-001").unwrap();
        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics[0].code, "W-STORE-007");
        assert!(diagnostics[0].message.contains("derives_from[0].foo"));
    }

    /// Two unknown keys appended out of alphabetical order — proves
    /// diagnostics follow YAML occurrence order (the `Mapping`'s insertion
    /// order), not e.g. a sorted-key iteration.
    #[test]
    fn document_with_multiple_unknown_fields_warns_in_yaml_order() {
        let record = sample_document();
        let mut yaml = document_to_yaml(&record);
        yaml.push_str("zeta_unknown: 1\nalpha_unknown: 2\n");
        let (_record, diagnostics) = document_from_yaml(&yaml, record.id.as_str()).unwrap();
        assert_eq!(diagnostics.len(), 2);
        assert!(diagnostics[0].message.contains("zeta_unknown"));
        assert!(diagnostics[1].message.contains("alpha_unknown"));
    }

    /// Guards `DOCUMENT_KEYS`/`DERIVES_FROM_KEYS` against drifting out of
    /// sync with `DocumentRecord`/`DerivesFrom`'s actual fields: every
    /// optional field here is populated (`Some`, non-empty), so nothing is
    /// omitted from the serialized shape by a `skip_serializing_if`.
    #[test]
    fn document_known_keys_match_the_record_shape() {
        let record = DocumentRecord {
            id: DocumentId::new("DOC-MAX-001"),
            path: "docs/max.md".to_string(),
            content_hash: ContentHash::from_text("max"),
            title: Some("title".to_string()),
            derives_from: vec![DerivesFrom {
                doc: DocumentId::new("DOC-REQ-001"),
                anchor: Some("anchor".to_string()),
                note: Some("note".to_string()),
            }],
            registered_at: "2026-08-08T00:00:00Z".to_string(),
        };
        let value = yaml_serde::to_value(&record).unwrap();

        let mut keys: Vec<&str> = value
            .as_mapping()
            .unwrap()
            .iter()
            .filter_map(|(key, _)| key.as_str())
            .collect();
        keys.sort_unstable();
        let mut expected = DOCUMENT_KEYS.to_vec();
        expected.sort_unstable();
        assert_eq!(keys, expected);

        let mut derives_from_keys: Vec<&str> = value
            .get("derives_from")
            .and_then(|list| list.get(0))
            .and_then(yaml_serde::Value::as_mapping)
            .unwrap()
            .iter()
            .filter_map(|(key, _)| key.as_str())
            .collect();
        derives_from_keys.sort_unstable();
        let mut expected_derives_from = DERIVES_FROM_KEYS.to_vec();
        expected_derives_from.sort_unstable();
        assert_eq!(derives_from_keys, expected_derives_from);
    }

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

    /// `derives_from` itself is mandatory (詳細設計 v0.1 §3.2: "1 件以上");
    /// this exercises every *other* optional field being absent instead.
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

    /// 詳細設計 v0.1 §3.2's own example, verbatim (including its inline
    /// comments), fed straight to the reader.
    #[test]
    fn vo_record_parses_the_literal_spec_example() {
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

    /// 詳細設計 v0.1 §3.2.1's `explicit` combinations example, verbatim: each
    /// entry is a dimension-name → partition-value flow-mapping, not a
    /// positional list of bare strings.
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
        assert_eq!(record.combinations[0].get("operand-sign"), Some("positive"));
        assert_eq!(record.combinations[0].get("operator"), Some("div"));
        // `combinations[]` entries are keyed by the record's own declared
        // dimension names (a dynamic vocabulary), not a fixed schema — this
        // locks in that the unknown-field scan does not walk into them and
        // misreport every dimension name as an unknown key.
        assert!(diagnostics.is_empty());
        let roundtrip = vo_record_to_yaml(&record);
        assert_eq!(
            vo_record_from_yaml(&roundtrip, "VO-ARITH-001").unwrap().0,
            record
        );
    }

    /// 別紙C:97-104's E-SCAN-017 condition 1 names three distinct `explicit`-
    /// policy inputs — `combinations` **欠落**（missing key）, `null`, and an
    /// empty list — as all requiring the same E-SCAN-017 diagnostic from the
    /// scan layer (`vtest-scan`'s `invalid_vo_combinations`), not a record-
    /// layer schema rejection. `null` and an omitted key must therefore both
    /// reach the typed `VoRecord` as `combinations: vec![]`, exactly like an
    /// explicit `combinations: []`, rather than failing `Deserialize`
    /// outright — this locks in both empirically (`VoRecord.combinations`
    /// carries `#[serde(default)]` for this reason).
    #[test]
    fn vo_record_combinations_missing_or_null_parses_as_empty_vec() {
        let base = "\
id: VO-COMBOS
parent: null
derives_from:
  - doc: DOC-BASIC-001
claim: claim
dimensions: []
coverage_policy: null
representative_cases: []
created: 2026-08-08
updated: 2026-08-08
";
        let (missing, diagnostics) = vo_record_from_yaml(base, "VO-COMBOS").unwrap();
        assert!(missing.combinations.is_empty());
        assert!(diagnostics.is_empty());

        let with_null = format!("{base}combinations: null\n");
        let (null_record, diagnostics) = vo_record_from_yaml(&with_null, "VO-COMBOS").unwrap();
        assert!(null_record.combinations.is_empty());
        assert!(diagnostics.is_empty());
        assert_eq!(missing, null_record);
    }

    /// 詳細設計 v0.1 本冊:283（§3.2.1 の受理条件、条件6）"entry が宣言済み
    /// dimension のいずれかを欠く、**または同じ dimension 名を2回以上持つ**"
    /// is one `E-SCAN-017` condition (VO retained, `chain_integrity =
    /// MISMATCH` — 本冊:1625・別紙A:438), not a record-layer rejection.
    /// BLOCKER 1 (PR #26 review round 1/2): this test used to assert the
    /// opposite — that reading such a record fails outright
    /// (`unwrap_err()`, "duplicate entry") — which routed this condition
    /// through E-SCAN-010 instead, dropping the VO from `vtest-scan`'s
    /// `vos` map entirely. `vo_record_from_yaml`'s own doc comment explains
    /// how this is now avoided (direct `VoRecord` deserialize,
    /// `CombinationEntry` preserving the duplicate losslessly instead of a
    /// bare `BTreeMap` colliding or `yaml_serde::Value` rejecting it).
    #[test]
    fn vo_record_combination_entry_with_a_duplicate_dimension_key_reaches_scan_as_e_scan_017() {
        let yaml = "\
id: VO-COMBOS
parent: null
derives_from:
  - doc: DOC-BASIC-001
claim: claim
dimensions:
  - name: d1
    partitions: [a, b]
  - name: d2
    partitions: [x, y]
coverage_policy: explicit
combinations:
  - d1: a
    d1: b
    d2: x
representative_cases: []
created: 2026-08-08
updated: 2026-08-08
";
        let (record, diagnostics) = vo_record_from_yaml(yaml, "VO-COMBOS")
            .expect("a duplicate key confined to inside one combinations[] entry must not fail the whole VO record read (BLOCKER 1) — the defect belongs to the scan layer's E-SCAN-017, not a record-layer rejection");
        assert_eq!(record.combinations.len(), 1);
        assert_eq!(
            record.combinations[0].iter().collect::<Vec<_>>(),
            vec![("d1", "a"), ("d1", "b"), ("d2", "x")],
            "every declared pair must survive, in declaration order — nothing fabricated, \
             nothing dropped"
        );
        assert_eq!(
            record.combinations[0].duplicate_dimension_names(),
            vec!["d1"]
        );
        // BLOCKER A (PR #26 review round 2): this record has no other
        // unknown field, so the empty result here is "nothing to warn
        // about", not "the scan was skipped" — `vo_record_with_a_
        // combinations_duplicate_and_an_unrelated_unknown_field_still_warns`
        // (below) is the test that actually distinguishes the two, by
        // adding an unrelated unknown field to the same combinations[]
        // duplicate and asserting its W-STORE-007 still fires.
        assert!(
            diagnostics.is_empty(),
            "no unrelated unknown field is present on this record, so there is nothing for the \
             scan to warn about: {diagnostics:?}"
        );
    }

    /// BLOCKER A (PR #26 review round 2): round 1's fix
    /// (`vo_record_combination_entry_with_a_duplicate_dimension_key_
    /// reaches_scan_as_e_scan_017`, above) built its diagnostics-only value
    /// with `yaml_serde::Value`, which — round 2 review measured — fails to
    /// build for *any* duplicate key anywhere in the document, not only one
    /// confined to `combinations[]`. Because the two failure reasons were
    /// indistinguishable through an `if let Ok(value) = ... else { skip }`,
    /// round 1 silently dropped every diagnostic on a record like this one
    /// — an unrelated unknown top-level field, on a record that also has a
    /// `combinations[]` duplicate — even though the record itself reads
    /// fine and the unknown field has nothing to do with `combinations[]`.
    /// This test used to lock that loss in as "a known, tested trade-off";
    /// `vo_record_from_yaml` now scans with `LenientValue` (see its own doc
    /// comment), which cannot fail to build on a duplicate key anywhere, so
    /// the warning is no longer lost.
    #[test]
    fn vo_record_with_a_combinations_duplicate_and_an_unrelated_unknown_field_still_warns() {
        let yaml = "\
id: VO-COMBOS
parent: null
derives_from:
  - doc: DOC-BASIC-001
claim: claim
dimensions:
  - name: d1
    partitions: [a, b]
coverage_policy: explicit
combinations:
  - d1: a
    d1: b
unknown_field: surprise
representative_cases: []
created: 2026-08-08
updated: 2026-08-08
";
        let (record, diagnostics) =
            vo_record_from_yaml(yaml, "VO-COMBOS").expect("the record itself still reads fine");
        assert_eq!(
            record.combinations[0].duplicate_dimension_names(),
            vec!["d1"],
            "the combinations[] duplicate must still reach the record, unaffected by the \
             unrelated unknown field"
        );
        assert_eq!(
            diagnostics.len(),
            1,
            "unknown_field must still warn as W-STORE-007, even though the same record's \
             combinations[] entry has an (unrelated) duplicate dimension key: {diagnostics:?}"
        );
        assert_eq!(diagnostics[0].code, "W-STORE-007");
        assert!(diagnostics[0].message.contains("unknown_field"));
    }

    /// BLOCKER A (PR #26 review round 2), probe row 6: an unrelated unknown
    /// top-level key whose *own value* is a mapping with an internally
    /// duplicated key. Round 1's `yaml_serde::Value`-based scan failed to
    /// build for this input too (the duplicate is not inside
    /// `combinations[]`, but `Value`'s duplicate-key rejection applies
    /// recursively to the whole document, not just the fields a schema
    /// recognizes) — so `extra` itself went unwarned, silently. `VoRecord`'s
    /// own typed parse never saw a problem here: `extra` is not one of its
    /// fields, so serde's generated struct visitor discards its value via
    /// `IgnoredAny` without ever checking that value's own keys for
    /// duplicates.
    #[test]
    fn vo_record_with_unknown_top_level_field_whose_value_has_a_duplicate_key_still_warns() {
        let record = sample_vo();
        let mut yaml = vo_record_to_yaml(&record);
        yaml.push_str("extra:\n  x: 1\n  x: 2\n");
        let (parsed, diagnostics) = vo_record_from_yaml(&yaml, record.id.as_str()).expect(
            "an unrelated unknown field's internally-duplicated value must not fail the read",
        );
        assert_eq!(parsed, record);
        assert_eq!(diagnostics.len(), 1, "{diagnostics:?}");
        assert_eq!(diagnostics[0].code, "W-STORE-007");
        assert!(diagnostics[0].message.contains("extra"));
    }

    /// BLOCKER A (PR #26 review round 2), probe row 8: `status`'s own value
    /// is a mapping with an internally duplicated key (rather than the
    /// ordinary scalar `status: draft`/`status: approved`). `status` is not
    /// a `VoRecord` field either, so the same `IgnoredAny` reasoning as the
    /// previous test applies to the primary parse; the presence check below
    /// only needs to see the *key*, not validate its value.
    #[test]
    fn vo_record_with_status_field_whose_value_has_a_duplicate_key_still_warns() {
        let record = sample_vo();
        let mut yaml = vo_record_to_yaml(&record);
        yaml.push_str("status:\n  x: 1\n  x: 2\n");
        let (parsed, diagnostics) = vo_record_from_yaml(&yaml, record.id.as_str())
            .expect("status's internally-duplicated value must not fail the read");
        assert_eq!(parsed, record);
        assert_eq!(diagnostics.len(), 1, "{diagnostics:?}");
        assert_eq!(diagnostics[0].code, "W-STORE-001");
    }

    /// BLOCKER A (PR #26 review round 2), probe row 10: an unknown key
    /// nested inside a `dimensions[]` entry, whose own value is a mapping
    /// with an internally duplicated key. `Dimension`'s derived
    /// `Deserialize` ignores `bar` the same way `VoRecord` ignores an
    /// unrecognized top-level key (ミラー of the two tests above, one level
    /// deeper — the same defect class, at the nesting the round-2 probe
    /// found it missing from).
    #[test]
    fn vo_record_with_unknown_nested_dimensions_field_whose_value_has_a_duplicate_key_still_warns()
    {
        let yaml = "\
id: VO-ARITH-001
parent: null
derives_from:
  - doc: DOC-BASIC-001
claim: claim
dimensions:
  - name: operand-sign
    partitions: [positive, negative]
    bar:
      x: 1
      x: 2
coverage_policy: null
combinations: []
representative_cases: []
created: 2026-08-08
updated: 2026-08-08
";
        let (_record, diagnostics) = vo_record_from_yaml(yaml, "VO-ARITH-001")
            .expect("an unknown nested field's internally-duplicated value must not fail the read");
        assert_eq!(diagnostics.len(), 1, "{diagnostics:?}");
        assert_eq!(diagnostics[0].code, "W-STORE-007");
        assert!(diagnostics[0].message.contains("dimensions[0].bar"));
    }

    /// The general duplicate-key rejection PR2 established for every VO
    /// record field — outside `combinations[]` — must survive the carve-out
    /// above unchanged. A duplicate top-level `claim:` key is still a
    /// record-layer rejection, and it no longer depends on `yaml_serde::
    /// Value` to get there: `VoRecord`'s own `#[derive(Deserialize)]`
    /// rejects it directly (confirmed empirically — `serde`'s generated
    /// struct visitor tracks each field and errors the second time it sees
    /// one, independent of whichever `Deserializer` drives it).
    #[test]
    fn vo_record_with_a_duplicate_top_level_key_outside_combinations_is_still_rejected() {
        let yaml = "\
id: VO-DUP-TOP
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
        let error = vo_record_from_yaml(yaml, "VO-DUP-TOP")
            .expect_err("a duplicate top-level key outside combinations[] must fail closed");
        assert!(
            error.to_string().contains("duplicate field"),
            "expected a duplicate-key parse rejection, got: {error}"
        );
    }

    /// Same carve-out boundary, one level deeper: a duplicate key inside a
    /// `derives_from[]` entry (a nested `struct`, `DerivesFrom`) is also
    /// still rejected, via that struct's own derived `Deserialize` — not
    /// `combinations[]`, so `CombinationEntry`'s tolerance does not apply.
    #[test]
    fn vo_record_with_a_duplicate_key_in_a_derives_from_entry_is_still_rejected() {
        let yaml = "\
id: VO-DUP-DERIVES
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
        let error = vo_record_from_yaml(yaml, "VO-DUP-DERIVES")
            .expect_err("a duplicate key in a derives_from[] entry must fail closed");
        assert!(
            error.to_string().contains("duplicate field"),
            "expected a duplicate-key parse rejection, got: {error}"
        );
    }

    /// Same carve-out boundary, checked for the other nested `struct`
    /// (`Dimension`, inside `dimensions[]`) that `combinations[]` sits
    /// alongside in `VoRecord`.
    #[test]
    fn vo_record_with_a_duplicate_key_in_a_dimensions_entry_is_still_rejected() {
        let yaml = "\
id: VO-DUP-DIMENSIONS
parent: null
derives_from:
  - doc: DOC-BASIC-001
claim: claim
dimensions:
  - name: d1
    name: d2
    partitions: [a, b]
coverage_policy: null
combinations: []
representative_cases: []
created: 2026-08-08
updated: 2026-08-08
";
        let error = vo_record_from_yaml(yaml, "VO-DUP-DIMENSIONS")
            .expect_err("a duplicate key in a dimensions[] entry must fail closed");
        assert!(
            error.to_string().contains("duplicate field"),
            "expected a duplicate-key parse rejection, got: {error}"
        );
    }

    /// 詳細設計 v0.1 §3.2: the reader accepts `status` (does not reject the
    /// record) but ignores its *value* and instead notifies W-STORE-001 on
    /// the field's mere presence — this checks both halves.
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

    /// `status` keeps its own specific W-STORE-001 diagnostic even when a
    /// second, genuinely unknown field is also present — the two do not
    /// collapse into one, and `status` (checked first) is reported first.
    #[test]
    fn vo_record_with_status_and_another_unknown_field_reports_both() {
        let record = sample_vo();
        let mut yaml = vo_record_to_yaml(&record);
        yaml.push_str("status: draft\nnickname: quick-vo\n");
        let (parsed, diagnostics) = vo_record_from_yaml(&yaml, record.id.as_str()).unwrap();
        assert_eq!(parsed, record);
        assert_eq!(diagnostics.len(), 2);
        assert_eq!(diagnostics[0].code, "W-STORE-001");
        assert_eq!(diagnostics[1].code, "W-STORE-007");
        assert!(diagnostics[1].message.contains("nickname"));
    }

    /// 詳細設計 v0.1 §3 header (L185): same generic warn-and-continue
    /// behavior as document, for a field with no dedicated code.
    #[test]
    fn vo_record_with_unknown_top_level_field_warns_and_still_reads() {
        let record = sample_vo();
        let mut yaml = vo_record_to_yaml(&record);
        yaml.push_str("owner: someone\n");
        let (parsed, diagnostics) = vo_record_from_yaml(&yaml, record.id.as_str()).unwrap();
        assert_eq!(parsed, record);
        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics[0].code, "W-STORE-007");
        assert!(diagnostics[0].message.contains("owner"));
    }

    #[test]
    fn vo_record_with_unknown_nested_derives_from_field_warns_with_path() {
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
        let (_record, diagnostics) = vo_record_from_yaml(yaml, "VO-PARSER-UTF8-003").unwrap();
        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics[0].code, "W-STORE-007");
        assert!(diagnostics[0].message.contains("derives_from[0].foo"));
    }

    #[test]
    fn vo_record_with_unknown_nested_dimensions_field_warns_with_path() {
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
        let (_record, diagnostics) = vo_record_from_yaml(yaml, "VO-ARITH-001").unwrap();
        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics[0].code, "W-STORE-007");
        assert!(diagnostics[0].message.contains("dimensions[0].bar"));
    }

    /// Guards `VO_KEYS`/`DIMENSION_KEYS` against drifting out of sync with
    /// `VoRecord`/`Dimension`'s actual fields. `sample_vo()` already
    /// populates every optional field (`parent`, `coverage_policy`), and no
    /// `VoRecord` field carries `skip_serializing_if` other than
    /// `derives_from` (always non-empty for a valid VO), so its serialized
    /// shape already has every key present.
    #[test]
    fn vo_known_keys_match_the_record_shape() {
        let value = yaml_serde::to_value(sample_vo()).unwrap();

        let mut keys: Vec<&str> = value
            .as_mapping()
            .unwrap()
            .iter()
            .filter_map(|(key, _)| key.as_str())
            .collect();
        keys.sort_unstable();
        let mut expected: Vec<&str> = VO_KEYS
            .iter()
            .copied()
            .filter(|key| *key != "status")
            .collect();
        expected.sort_unstable();
        assert_eq!(
            keys, expected,
            "VO_KEYS (minus the status read-compat key) must list exactly VoRecord's fields"
        );

        let mut dimension_keys: Vec<&str> = value
            .get("dimensions")
            .and_then(|list| list.get(0))
            .and_then(yaml_serde::Value::as_mapping)
            .unwrap()
            .iter()
            .filter_map(|(key, _)| key.as_str())
            .collect();
        dimension_keys.sort_unstable();
        let mut expected_dimension = DIMENSION_KEYS.to_vec();
        expected_dimension.sort_unstable();
        assert_eq!(dimension_keys, expected_dimension);
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
        record.combinations = vec![vtest_model::CombinationEntry::from_iter([
            ("operand-sign".to_string(), "positive".to_string()),
            ("operator".to_string(), "div".to_string()),
        ])];
        let yaml = vo_record_to_yaml(&record);
        assert_eq!(
            vo_record_from_yaml(&yaml, record.id.as_str()).unwrap().0,
            record
        );
    }

    /// BLOCKER 1 condition 2 (PR #26 review round 2): the writer must not be
    /// able to persist a `combinations[]` entry with a duplicate dimension
    /// name, even though the reader can tolerate reading one back
    /// (`vo_record_combination_entry_with_a_duplicate_dimension_key_reaches_
    /// scan_as_e_scan_017` below). A `BTreeMap`-backed entry could not even
    /// construct this state; `CombinationEntry` can (that is the whole point
    /// — see its doc comment), so the writer needs its own explicit guard.
    #[test]
    fn write_vo_record_refuses_a_combinations_entry_with_a_duplicate_dimension_name() {
        let root = std::env::temp_dir().join(format!(
            "vtest-store-canonical-dup-combo-{}",
            crate::new_record_id()
        ));
        let layout = crate::init_project(&root, "example").unwrap();
        let mut record = sample_vo();
        record.dimensions = vec![Dimension {
            name: "d1".to_string(),
            partitions: vec!["a".to_string(), "b".to_string()],
        }];
        record.coverage_policy = Some(CoveragePolicy::Explicit);
        record.combinations = vec![vtest_model::CombinationEntry::from_iter([
            ("d1".to_string(), "a".to_string()),
            ("d1".to_string(), "b".to_string()),
        ])];
        write_vo_record(&layout, &record)
            .expect_err("a duplicate dimension name in combinations must not be writable");
        assert!(
            !layout
                .vo_dir()
                .join(format!("{}.yaml", record.id.as_str()))
                .exists(),
            "the writer must refuse before touching disk"
        );
    }

    /// PR #26 round 3: the three §3 record readers used to disagree on a
    /// record whose only defect was a duplicate key confined to the value of
    /// a field none of them recognize — document/Relation rejected the whole
    /// record, VO warned and read it. All three now agree on every row of
    /// this table (each row exercised for document, then VO, then Relation,
    /// in that order):
    ///
    /// | input shape                                       | document    | VO          | Relation    |
    /// |----------------------------------------------------|-------------|-------------|-------------|
    /// | unknown field's *value* has an internal duplicate   | warn, reads | warn, reads | warn, reads |
    /// | duplicate key on a field the schema *does* recognize | reject      | reject      | reject      |
    /// | an ordinary (non-duplicated) unknown top-level field | warn, reads | warn, reads | warn, reads |
    ///
    /// The individual per-reader tests above (`document_with_unknown_top_
    /// level_field_whose_value_has_a_duplicate_key_still_warns`,
    /// `vo_record_with_unknown_top_level_field_whose_value_has_a_duplicate_
    /// key_still_warns`, and `records.rs`'s
    /// `relation_with_unknown_top_level_field_whose_value_has_a_duplicate_
    /// key_still_warns`/`relation_with_a_duplicate_top_level_key_is_still_
    /// rejected`) already lock in each cell on its own; this test exists so
    /// the three readers' agreement is visible in one place instead of
    /// inferred by reading three files.
    #[test]
    fn document_vo_relation_readers_agree_on_the_same_three_input_shapes() {
        fn relation_fixture() -> crate::RelationRecord {
            crate::RelationRecord {
                id: crate::new_record_id(),
                relation_type: crate::RelationType::Complements,
                from: "TEST-PARSER-044".to_owned(),
                to: "TEST-PARSER-012".to_owned(),
                note: None,
                created: "2026-08-08T00:00:00Z".to_owned(),
            }
        }

        // Row 1: an unknown field's value has an internally duplicated key.
        {
            let record = sample_document();
            let mut yaml = document_to_yaml(&record);
            yaml.push_str("owner:\n  x: 1\n  x: 2\n");
            let (parsed, diagnostics) =
                document_from_yaml(&yaml, record.id.as_str()).expect("document, row 1: must read");
            assert_eq!(parsed, record);
            assert_eq!(diagnostics.len(), 1, "document, row 1: {diagnostics:?}");
            assert_eq!(diagnostics[0].code, "W-STORE-007");
        }
        {
            let record = sample_vo();
            let mut yaml = vo_record_to_yaml(&record);
            yaml.push_str("owner:\n  x: 1\n  x: 2\n");
            let (parsed, diagnostics) =
                vo_record_from_yaml(&yaml, record.id.as_str()).expect("VO, row 1: must read");
            assert_eq!(parsed, record);
            assert_eq!(diagnostics.len(), 1, "VO, row 1: {diagnostics:?}");
            assert_eq!(diagnostics[0].code, "W-STORE-007");
        }
        {
            let record = relation_fixture();
            let mut yaml = record.to_yaml().unwrap();
            yaml.push_str("owner:\n  x: 1\n  x: 2\n");
            let (parsed, diagnostics) = crate::RelationRecord::from_yaml(&yaml, &record.id)
                .expect("Relation, row 1: must read");
            assert_eq!(parsed, record);
            assert_eq!(diagnostics.len(), 1, "Relation, row 1: {diagnostics:?}");
            assert_eq!(diagnostics[0].code, "W-STORE-007");
        }

        // Row 2: a duplicate key on a field the schema does recognize.
        {
            let yaml = document_to_yaml(&sample_document());
            let mut duplicated = yaml.clone();
            duplicated.push_str(&yaml);
            assert!(
                document_from_yaml(&duplicated, "DOC-BASIC-001").is_err(),
                "document, row 2: must reject"
            );
        }
        {
            let yaml = vo_record_to_yaml(&sample_vo());
            let mut duplicated = yaml.clone();
            duplicated.push_str(&yaml);
            assert!(
                vo_record_from_yaml(&duplicated, "VO-PARSER-UTF8-003").is_err(),
                "VO, row 2: must reject"
            );
        }
        {
            let record = relation_fixture();
            let yaml = record.to_yaml().unwrap();
            let mut duplicated = yaml.clone();
            duplicated.push_str(&yaml);
            assert!(
                crate::RelationRecord::from_yaml(&duplicated, &record.id).is_err(),
                "Relation, row 2: must reject"
            );
        }

        // Row 3: an ordinary unknown top-level field, with no duplicate
        // anywhere.
        {
            let record = sample_document();
            let mut yaml = document_to_yaml(&record);
            yaml.push_str("nickname: quick-doc\n");
            let (parsed, diagnostics) =
                document_from_yaml(&yaml, record.id.as_str()).expect("document, row 3: must read");
            assert_eq!(parsed, record);
            assert_eq!(diagnostics.len(), 1, "document, row 3: {diagnostics:?}");
            assert_eq!(diagnostics[0].code, "W-STORE-007");
        }
        {
            let record = sample_vo();
            let mut yaml = vo_record_to_yaml(&record);
            yaml.push_str("nickname: quick-vo\n");
            let (parsed, diagnostics) =
                vo_record_from_yaml(&yaml, record.id.as_str()).expect("VO, row 3: must read");
            assert_eq!(parsed, record);
            assert_eq!(diagnostics.len(), 1, "VO, row 3: {diagnostics:?}");
            assert_eq!(diagnostics[0].code, "W-STORE-007");
        }
        {
            let record = relation_fixture();
            let mut yaml = record.to_yaml().unwrap();
            yaml.push_str("nickname: quick-rel\n");
            let (parsed, diagnostics) = crate::RelationRecord::from_yaml(&yaml, &record.id)
                .expect("Relation, row 3: must read");
            assert_eq!(parsed, record);
            assert_eq!(diagnostics.len(), 1, "Relation, row 3: {diagnostics:?}");
            assert_eq!(diagnostics[0].code, "W-STORE-007");
        }
    }
}
