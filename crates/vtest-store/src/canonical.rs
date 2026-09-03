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

/// Scans a YAML mapping for keys outside `known`, returning one
/// `W-STORE-007` diagnostic per unknown key, in YAML occurrence order
/// (`yaml_serde::Mapping` preserves insertion order). 詳細設計 v0.1 §3
/// header (L185): "すべてのレコードは YAML とし、未知フィールドはエラーで
/// はなく警告とする" — the record is still read (this function never
/// returns an `Err`), only warned about. `prefix` is prepended to each
/// reported key so a nested unknown key (e.g. inside `derives_from[0]`)
/// reads distinctly from a top-level one. Returns nothing if `value` isn't
/// a mapping — a type mismatch there is instead caught, fail-closed, by
/// the `from_value` deserialize this always runs alongside.
///
/// `pub(crate)` (rather than private to this module) so `records.rs`'s
/// `RelationRecord::from_yaml` — a §3.3 reader that lives outside this
/// module for historical reasons (see that file's module doc comment) —
/// can apply the same §3-header rule instead of re-implementing the scan.
pub(crate) fn unknown_field_diagnostics(
    value: &yaml_serde::Value,
    known: &[&str],
    prefix: &str,
) -> Vec<Diagnostic> {
    let Some(mapping) = value.as_mapping() else {
        return Vec::new();
    };
    mapping
        .iter()
        .filter_map(|(key, _)| key.as_str())
        .filter(|key| !known.contains(key))
        .map(|key| {
            Diagnostic::warning(
                "W-STORE-007",
                format!("unknown field `{prefix}{key}` is not part of the §3 schema; its value is ignored"),
            )
        })
        .collect()
}

/// Extends the unknown-field scan into each `derives_from[]` entry — the
/// nested shape §3.1 (document) and §3.2 (VO) share.
fn derives_from_diagnostics(value: &yaml_serde::Value) -> Vec<Diagnostic> {
    let Some(sequence) = value
        .get("derives_from")
        .and_then(yaml_serde::Value::as_sequence)
    else {
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
fn dimensions_diagnostics(value: &yaml_serde::Value) -> Vec<Diagnostic> {
    let Some(sequence) = value
        .get("dimensions")
        .and_then(yaml_serde::Value::as_sequence)
    else {
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
/// returning any non-fatal diagnostics alongside it. `yaml_serde::from_value`
/// already fails closed on a missing required field or a malformed value via
/// `DocumentRecord`'s `Deserialize` derive; this adds the id/file-name check
/// the derive cannot express (matching the strictness `RelationRecord`/
/// `ApprovalRecord`/`AuditRecord` already apply), plus the unknown-field scan
/// 詳細設計 v0.1 §3 header (L185) requires of every record type.
pub fn document_from_yaml(
    text: &str,
    fallback_id: &str,
) -> Result<(DocumentRecord, Vec<Diagnostic>), StoreError> {
    let value: yaml_serde::Value = yaml_serde::from_str(text)
        .map_err(|error| StoreError::InvalidConfig(format!("invalid document record: {error}")))?;

    let mut diagnostics = unknown_field_diagnostics(&value, DOCUMENT_KEYS, "");
    diagnostics.extend(derives_from_diagnostics(&value));

    let record: DocumentRecord = yaml_serde::from_value(value)
        .map_err(|error| StoreError::InvalidConfig(format!("invalid document record: {error}")))?;
    if record.id.as_str() != fallback_id {
        return Err(StoreError::InvalidConfig(format!(
            "document id {} does not match file name {fallback_id}",
            record.id.as_str()
        )));
    }
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

/// Parses a canonical `VoRecord` from its YAML representation, returning any
/// non-fatal diagnostics alongside it. `yaml_serde::from_value` fails closed
/// on a missing required field (`claim`/`created`/`updated`/etc.) or an
/// unrecognized `coverage_policy` value via `VoRecord`'s `Deserialize`
/// derive; this adds the id/file-name check the derive cannot express, plus
/// the `derives_from` cardinality floor (詳細設計 v0.1 §3.2).
///
/// This goes through an explicit two-stage parse (text → `Value` → known-key
/// scan → `VoRecord`) because a direct `from_str::<VoRecord>` would silently
/// drop an unrecognized key with no way to observe it happened: serde's
/// derive ignores fields the target struct does not declare. Most unknown
/// keys are reported generically as W-STORE-007 (詳細設計 v0.1 §3 header,
/// L185); `status` gets its own more specific diagnostic instead, since
/// `VoRecord` deliberately has no `status` field (canonical writers never
/// persist it — adding one to detect it would pollute the canonical model
/// and change its JSON shape) but 詳細設計 v0.1 §3.2 names the read-compat
/// case explicitly: "readerは読取り互換fieldとして`status`を受理するが、実効
/// 判定とVO subject hashでは無視し、存在自体をW-STORE-001として通知する".
pub fn vo_record_from_yaml(
    text: &str,
    fallback_id: &str,
) -> Result<(VoRecord, Vec<Diagnostic>), StoreError> {
    let (value, combinations_entry_had_a_duplicate_key) =
        parse_vo_value_tolerating_a_combinations_duplicate_key(text)
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

    let mut record: VoRecord = yaml_serde::from_value(value)
        .map_err(|error| StoreError::InvalidConfig(format!("invalid VO record: {error}")))?;
    if combinations_entry_had_a_duplicate_key {
        // 本冊:283（§3.2.1 の受理条件、条件6後半）「entry が...同じdimension
        // 名を2回以上持つ」は E-SCAN-017（`vtest-scan`の`invalid_vo_
        // combinations`、VOを保持したまま`chain_integrity = MISMATCH`）が
        // 扱う条件であり、record層のE-SCAN-010（VOが丸ごと`vos`マップから
        // 消える）ではない（本冊:1625・別紙A:438）。しかし
        // `combinations: Vec<BTreeMap<String, String>>`は重複キーを持つ
        // entryをそもそも表現できない — 上の
        // `parse_vo_value_tolerating_a_combinations_duplicate_key`が
        // `combinations:`ブロック全体を`[]`へ置き換えて初めてパースが
        // 成立している。ここでその置換を「宣言済みdimensionを一切持たない
        // 1件のentry」（空map）で埋め直す。これは`invalid_vo_combinations`
        // の既存の長さ不一致検査（条件6前半「entry が宣言済み dimension の
        // いずれかを欠く」と同じ判定式。本冊:283 は前半・後半を同一箇条で
        // 並列に扱っており、診断コード・chain_integrity・vo expand の帰結は
        // 前半後半で区別されない）に自然に引っかかり、`coverage_policy`が
        // `explicit`以外の場合も「combinations が空でないのに explicit
        // 以外」（条件3）として同じくE-SCAN-017になる。重複していた具体的な
        // dimension名・値はこの型では保持できない（BLOCKER 1 対応の報告
        // 事項）。
        record.combinations = single_combination_entry_with_no_declared_dimensions();
    }
    if record.id.as_str() != fallback_id {
        return Err(StoreError::InvalidConfig(format!(
            "VO id {} does not match file name {fallback_id}",
            record.id.as_str()
        )));
    }
    require_at_least_one_derives_from(&record.derives_from)?;
    Ok((record, diagnostics))
}

fn single_combination_entry_with_no_declared_dimensions(
) -> Vec<std::collections::BTreeMap<String, String>> {
    vec![std::collections::BTreeMap::new()]
}

/// Parses `text` into a `yaml_serde::Value`, the way every other canonical
/// record does — except that a duplicate key confined to inside one
/// `combinations[]` entry's own mapping does not fail the parse the way a
/// duplicate key anywhere else in the document still does.
///
/// `yaml_serde::Value`'s `Mapping` visitor rejects a duplicate key wherever
/// it occurs in the document (`mapping.rs`'s `DuplicateKeyError`), before any
/// typed struct is ever built. 詳細設計 v0.1 本冊:283（§3.2.1）makes "同じ
/// dimension 名を2回以上持つ" one of the `combinations` conditions
/// `E-SCAN-017`（`vtest-scan`, VO retained）owns, but everywhere else in a VO
/// record a duplicate key is still exactly the record-layer schema violation
/// PR2 established it to be (`document_with_a_duplicate_top_level_key_is_
/// rejected`, `vo_record_combination_entry_with_a_duplicate_dimension_key_is_
/// rejected` before this fix, `approval_roles_with_a_duplicate_key_is_
/// rejected`) — this function must not weaken that anywhere except inside
/// `combinations[]`.
///
/// Strategy: parse normally first. Only on a `"duplicate entry"` failure,
/// retry with the entire top-level `combinations:` block replaced by
/// `combinations: []` (`strip_combinations_block`). If that retry succeeds,
/// the duplicate was confined to `combinations[]` — the second return value
/// is `true`, and the caller substitutes a sentinel entry back in (see
/// `vo_record_from_yaml`) so `invalid_vo_combinations` still reports
/// E-SCAN-017 instead of silently accepting an empty `combinations`. If the
/// retry still fails, the duplicate (or some other defect) exists outside
/// `combinations[]` too, and the *original* error is what the caller must
/// see — record-layer rejection stands, unchanged from before this fix.
fn parse_vo_value_tolerating_a_combinations_duplicate_key(
    text: &str,
) -> yaml_serde::Result<(yaml_serde::Value, bool)> {
    match yaml_serde::from_str(text) {
        Ok(value) => Ok((value, false)),
        Err(error) => {
            if !error.to_string().contains("duplicate entry") {
                return Err(error);
            }
            let Some(stripped) = strip_combinations_block(text) else {
                return Err(error);
            };
            match yaml_serde::from_str(&stripped) {
                Ok(value) => Ok((value, true)),
                Err(_) => Err(error),
            }
        }
    }
}

/// Replaces the top-level `combinations:` key and everything that belongs to
/// it (its own line, plus every following more-indented line — the same
/// block-boundary rule `records.rs`'s `nested_scalar` uses for its hand-rolled
/// parsing) with a single `combinations: []` line. Returns `None` if `text`
/// has no top-level `combinations:` key at all, so the caller can tell "there
/// was nothing to strip" apart from "stripping produced this text".
fn strip_combinations_block(text: &str) -> Option<String> {
    let lines = text.lines().collect::<Vec<_>>();
    let start = lines.iter().position(|raw| {
        let line = raw.trim_end();
        !line.starts_with(' ') && (line == "combinations:" || line.starts_with("combinations:"))
    })?;
    let end = lines[start + 1..]
        .iter()
        .position(|raw| {
            let trimmed = raw.trim();
            !trimmed.is_empty() && !raw.starts_with(' ') && !raw.starts_with('\t')
        })
        .map_or(lines.len(), |offset| start + 1 + offset);

    let mut result = String::new();
    for line in &lines[..start] {
        result.push_str(line);
        result.push('\n');
    }
    result.push_str("combinations: []\n");
    for line in &lines[end..] {
        result.push_str(line);
        result.push('\n');
    }
    Some(result)
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
/// reject, which fail-closed reading alone does not prevent.
pub fn write_vo_record(layout: &VerifyLayout, record: &VoRecord) -> Result<(), StoreError> {
    require_at_least_one_derives_from(&record.derives_from)?;
    let path = layout.vo_dir().join(format!("{}.yaml", record.id.as_str()));
    write_atomic(&path, &vo_record_to_yaml(record))
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
    use std::collections::BTreeMap;
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

    /// The two-stage parse (text -> `Value` -> known-key scan -> typed
    /// struct) `document_from_yaml` uses for the unknown-field warning also
    /// rejects a duplicate top-level key outright, as a side effect: parsing
    /// to `Value` first runs `yaml_serde::Mapping`'s own duplicate-key check
    /// on the whole document before the typed struct is ever built.
    #[test]
    fn document_with_a_duplicate_top_level_key_is_rejected() {
        let yaml = document_to_yaml(&sample_document());
        let mut duplicated = yaml.clone();
        duplicated.push_str(&yaml);
        assert!(
            document_from_yaml(&duplicated, "DOC-BASIC-001").is_err(),
            "a document YAML with every top-level key duplicated must fail closed"
        );
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

    /// 詳細設計 v0.1 本冊:283（§3.2.1 の受理条件、条件6後半）"entry が
    /// 宣言済み dimension のいずれかを欠く、または同じ dimension 名を2回以上
    /// 持つ" は E-SCAN-017（`vtest-scan`の`invalid_vo_combinations`が扱う、
    /// VOを保持したまま`chain_integrity = MISMATCH`にする条件）であり、
    /// record層のE-SCAN-010（VOが丸ごと`vos`マップから消える）ではない
    /// （本冊:1625・別紙A:438）。
    ///
    /// `yaml_serde::Value`の`Mapping`visitorは重複キーをドキュメント中どこで
    /// あれ拒否するため、`combinations[]`の1entry内の重複キーだけを特別扱い
    /// せずに読むと、record層が先にVOレコードそのものを拒否してしまい
    /// （BLOCKER 1、PR #26 review round 1 — 旧版のこのテストはその誤った
    /// 挙動を固定していた）、条件1「欠落」で`vo.rs`の`#[serde(default)]`が
    /// 直したのと同型の欠陥になる。`vo_record_from_yaml`は今、重複が
    /// `combinations[]`の内側に閉じている場合に限りレコードを読み進め、
    /// その`combinations`を「宣言済みdimensionを一切持たない1件のentry」へ
    /// 置き換える（重複していた具体的な値は`Vec<BTreeMap<String, String>>`
    /// では表現できない — 置き換え後も`invalid_vo_combinations`の長さ不一致
    /// 検査に確実に引っかかり、E-SCAN-017が保証される）。
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
        assert_eq!(
            record.combinations,
            vec![std::collections::BTreeMap::new()],
            "the unrepresentable duplicate-key entry must be replaced with a sentinel that \
             still trips invalid_vo_combinations's declared-dimension-count check, not silently \
             dropped to an empty combinations list"
        );
        assert!(diagnostics.is_empty());
    }

    /// The general duplicate-key rejection PR2 established for every VO
    /// record field (`document_with_a_duplicate_top_level_key_is_rejected`'s
    /// sibling case) must survive the `combinations[]`-specific carve-out
    /// above unchanged: a duplicate key anywhere *other* than inside a
    /// `combinations[]` entry — here, the top-level `claim` key — is still a
    /// record-layer rejection.
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
            error.to_string().contains("duplicate entry"),
            "expected a duplicate-key parse rejection, got: {error}"
        );
    }

    /// A duplicate dimension key inside a `combinations[]` entry alongside a
    /// *separate* duplicate elsewhere in the record (`derives_from[]` here)
    /// must not have the `combinations[]` carve-out silently swallow the
    /// second, unrelated defect: stripping `combinations:` alone does not
    /// make the record parseable, so the original rejection must stand.
    #[test]
    fn vo_record_with_duplicates_both_inside_and_outside_combinations_is_rejected() {
        let yaml = "\
id: VO-DUP-BOTH
parent: null
derives_from:
  - doc: DOC-BASIC-001
    doc: DOC-OTHER-001
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
        let error = vo_record_from_yaml(yaml, "VO-DUP-BOTH").expect_err(
            "a duplicate outside combinations[] must still fail closed even when combinations[] also has one",
        );
        assert!(
            error.to_string().contains("duplicate entry"),
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
