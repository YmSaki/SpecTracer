//! Canonical v0.1 record storage (詳細設計 v0.1 §3), for the types defined in
//! `vtest-model`. Kept separate from `records.rs`'s predecessor Req/Spec-model
//! types so neither module's exports collide with the other's.

use crate::records::{
    list, parse_combinations, parse_inline_list, scalar, unquote, yaml_list, yaml_scalar,
};
use crate::{read_text, write_atomic, StoreError, VerifyLayout};
use vtest_model::{ContentHash, DerivesFrom, DocumentId, DocumentRecord, VoId, VoRecord};

/// Serializes a `DocumentRecord` to its canonical `.verify/doc/DOC-*.yaml`
/// shape (詳細設計 v0.1 §3.1).
pub fn document_to_yaml(record: &DocumentRecord) -> String {
    let mut out = format!(
        "id: {}\npath: {}\ncontent_hash: {}\n",
        yaml_scalar(record.id.as_str()),
        yaml_scalar(&record.path),
        yaml_scalar(record.content_hash.as_str()),
    );
    if let Some(title) = &record.title {
        out.push_str(&format!("title: {}\n", yaml_scalar(title)));
    }
    out.push_str(&derives_from_yaml(&record.derives_from));
    out.push_str(&format!(
        "registered_at: {}\n",
        yaml_scalar(&record.registered_at)
    ));
    out
}

/// Parses a `DocumentRecord` from its canonical YAML representation.
/// `id`/`path`/`content_hash`/`registered_at` are required (詳細設計 v0.1
/// §3.1's example has no `?` marker on any of them); a missing field or an
/// `id` that disagrees with the file name fails closed rather than
/// synthesizing an empty-string or fallback-derived record, matching the
/// strictness `RelationRecord`/`ApprovalRecord`/`AuditRecord` already apply.
pub fn document_from_yaml(text: &str, fallback_id: &str) -> Result<DocumentRecord, StoreError> {
    let id = scalar(text, "id")
        .ok_or_else(|| StoreError::InvalidConfig("document is missing id".to_owned()))?;
    if id != fallback_id {
        return Err(StoreError::InvalidConfig(format!(
            "document id {id} does not match file name {fallback_id}"
        )));
    }
    let path = scalar(text, "path")
        .ok_or_else(|| StoreError::InvalidConfig("document is missing path".to_owned()))?;
    let content_hash: ContentHash = scalar(text, "content_hash")
        .ok_or_else(|| StoreError::InvalidConfig("document is missing content_hash".to_owned()))?
        .parse()
        .map_err(|error: String| StoreError::InvalidConfig(error))?;
    let registered_at = scalar(text, "registered_at")
        .ok_or_else(|| StoreError::InvalidConfig("document is missing registered_at".to_owned()))?;
    Ok(DocumentRecord {
        id: DocumentId::new(id),
        path,
        content_hash,
        title: scalar(text, "title"),
        derives_from: parse_derives_from(text),
        registered_at,
    })
}

/// Reads the canonical document record `<id>.yaml` from `.verify/doc/`.
pub fn read_document(layout: &VerifyLayout, id: &str) -> Result<DocumentRecord, StoreError> {
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

/// Serializes the `derives_from:` block list shared by document and VO
/// records (詳細設計 v0.1 §3.1, §3.2).
fn derives_from_yaml(entries: &[DerivesFrom]) -> String {
    if entries.is_empty() {
        return "derives_from: []\n".to_owned();
    }
    let mut out = String::from("derives_from:\n");
    for entry in entries {
        out.push_str(&format!("  - doc: {}\n", yaml_scalar(entry.doc.as_str())));
        if let Some(anchor) = &entry.anchor {
            out.push_str(&format!("    anchor: {}\n", yaml_scalar(anchor)));
        }
        if let Some(note) = &entry.note {
            out.push_str(&format!("    note: {}\n", yaml_scalar(note)));
        }
    }
    out
}

/// Parses the `derives_from:` block list. Each entry's `anchor`/`note` are
/// optional and read in the fixed order the writer emits them.
fn parse_derives_from(text: &str) -> Vec<DerivesFrom> {
    let lines: Vec<&str> = text.lines().collect();
    let Some(start) = lines.iter().position(|line| {
        !line.starts_with([' ', '\t'])
            && line
                .trim()
                .strip_prefix("derives_from:")
                .is_some_and(|value| value.trim().is_empty())
    }) else {
        return Vec::new();
    };
    let block: Vec<&str> = lines[start + 1..]
        .iter()
        .take_while(|line| line.starts_with([' ', '\t']) || line.trim().is_empty())
        .copied()
        .collect();

    let mut entries = Vec::new();
    let mut index = 0;
    while index < block.len() {
        let line = block[index].trim();
        let Some(doc) = line.strip_prefix("- doc:") else {
            index += 1;
            continue;
        };
        let doc = DocumentId::new(unquote(doc.trim()));
        let mut anchor = None;
        let mut note = None;
        index += 1;
        while index < block.len() {
            let next = block[index].trim();
            if let Some(value) = next.strip_prefix("anchor:") {
                anchor = Some(unquote(value.trim()));
                index += 1;
            } else if let Some(value) = next.strip_prefix("note:") {
                note = Some(unquote(value.trim()));
                index += 1;
            } else {
                break;
            }
        }
        entries.push(DerivesFrom { doc, anchor, note });
    }
    entries
}

/// Serializes a canonical `VoRecord` to its `.verify/vo/VO-*.yaml` shape
/// (詳細設計 v0.1 §3.2). Distinct name from `read_vo`/`VoRecord` (records.rs)
/// on purpose: that pair still serves the predecessor store-side `VoRecord`
/// until PR8 retires it, and the two types are not interchangeable.
pub fn vo_record_to_yaml(record: &VoRecord) -> String {
    let mut out = format!(
        "id: {}\nparent: {}\n",
        yaml_scalar(record.id.as_str()),
        record
            .parent
            .as_ref()
            .map(|value| yaml_scalar(value.as_str()))
            .unwrap_or_else(|| "null".to_owned()),
    );
    out.push_str(&derives_from_yaml(&record.derives_from));
    out.push_str(&format!(
        "claim: {}\ndimensions:\n",
        yaml_scalar(&record.claim)
    ));
    for dimension in &record.dimensions {
        out.push_str(&format!(
            "  - name: {}\n    partitions: {}\n",
            yaml_scalar(&dimension.name),
            yaml_list(dimension.partitions.iter().map(String::as_str)),
        ));
    }
    out.push_str(&format!(
        "coverage_policy: {}\n",
        record
            .coverage_policy
            .map(|policy| yaml_scalar(coverage_policy_str(policy)))
            .unwrap_or_else(|| "null".to_owned()),
    ));
    if record.combinations.is_empty() {
        out.push_str("combinations: []\n");
    } else {
        out.push_str("combinations:\n");
        for combination in &record.combinations {
            out.push_str(&format!(
                "  - {}\n",
                yaml_list(combination.iter().map(String::as_str))
            ));
        }
    }
    out.push_str(&format!(
        "representative_cases: {}\ncreated: {}\nupdated: {}\n",
        yaml_list(record.representative_cases.iter().map(String::as_str)),
        yaml_scalar(&record.created),
        yaml_scalar(&record.updated),
    ));
    out
}

/// Parses a canonical `VoRecord` from its YAML representation. `id`/`claim`/
/// `created`/`updated` are required, `id` must match the file name, and a
/// `coverage_policy` value that is present but not one of the three known
/// variants fails closed rather than silently collapsing to `None` (which
/// would make it indistinguishable from an intentional `coverage_policy:
/// null`). The `status` read-compat field (詳細設計 v0.1 §3.2) is parsed
/// nowhere: canonical writers never persist it, and readers must not derive
/// VO state from it.
pub fn vo_record_from_yaml(text: &str, fallback_id: &str) -> Result<VoRecord, StoreError> {
    let id = scalar(text, "id")
        .ok_or_else(|| StoreError::InvalidConfig("VO is missing id".to_owned()))?;
    if id != fallback_id {
        return Err(StoreError::InvalidConfig(format!(
            "VO id {id} does not match file name {fallback_id}"
        )));
    }
    let claim = scalar(text, "claim")
        .ok_or_else(|| StoreError::InvalidConfig("VO is missing claim".to_owned()))?;
    let coverage_policy = match scalar(text, "coverage_policy") {
        None => None,
        Some(value) => Some(coverage_policy_from_str(&value).ok_or_else(|| {
            StoreError::InvalidConfig(format!("VO has an unrecognized coverage_policy `{value}`"))
        })?),
    };
    let created = scalar(text, "created")
        .ok_or_else(|| StoreError::InvalidConfig("VO is missing created".to_owned()))?;
    let updated = scalar(text, "updated")
        .ok_or_else(|| StoreError::InvalidConfig("VO is missing updated".to_owned()))?;
    Ok(VoRecord {
        id: VoId::new(id),
        parent: scalar(text, "parent")
            .and_then(|value| (!value.is_empty()).then_some(VoId::new(value))),
        derives_from: parse_derives_from(text),
        claim,
        dimensions: parse_dimensions(text),
        coverage_policy,
        combinations: parse_combinations(text),
        representative_cases: list(text, "representative_cases"),
        created,
        updated,
    })
}

/// Reads the canonical VO record `<id>.yaml` from `.verify/vo/`.
pub fn read_vo_record(layout: &VerifyLayout, id: &str) -> Result<VoRecord, StoreError> {
    let path = layout.vo_dir().join(format!("{id}.yaml"));
    let text = read_text(&path)?;
    vo_record_from_yaml(&text, id)
}

/// Writes (or overwrites) the canonical VO record to `.verify/vo/`. Mutable
/// in place, for the same reason as `write_document` above.
pub fn write_vo_record(layout: &VerifyLayout, record: &VoRecord) -> Result<(), StoreError> {
    let path = layout.vo_dir().join(format!("{}.yaml", record.id.as_str()));
    write_atomic(&path, &vo_record_to_yaml(record))
}

fn coverage_policy_str(policy: vtest_model::CoveragePolicy) -> &'static str {
    match policy {
        vtest_model::CoveragePolicy::IndependentAxes => "independent-axes",
        vtest_model::CoveragePolicy::FullProduct => "full-product",
        vtest_model::CoveragePolicy::Explicit => "explicit",
    }
}

fn coverage_policy_from_str(value: &str) -> Option<vtest_model::CoveragePolicy> {
    match value {
        "independent-axes" => Some(vtest_model::CoveragePolicy::IndependentAxes),
        "full-product" => Some(vtest_model::CoveragePolicy::FullProduct),
        "explicit" => Some(vtest_model::CoveragePolicy::Explicit),
        _ => None,
    }
}

fn parse_dimensions(text: &str) -> Vec<vtest_model::Dimension> {
    let lines = text.lines().collect::<Vec<_>>();
    let mut dimensions = Vec::new();
    for (index, raw) in lines.iter().enumerate() {
        let line = raw.trim();
        let Some(name) = line.strip_prefix("- name:") else {
            continue;
        };
        let partitions = lines
            .get(index + 1)
            .and_then(|next| next.trim().strip_prefix("partitions:"))
            .map(|value| parse_inline_list(value.trim()))
            .unwrap_or_default();
        dimensions.push(vtest_model::Dimension {
            name: unquote(name.trim()),
            partitions,
        });
    }
    dimensions
}

#[cfg(test)]
mod tests {
    use super::*;
    use vtest_model::{CoveragePolicy, Dimension};

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
        assert_eq!(
            document_from_yaml(&yaml, record.id.as_str()).unwrap(),
            record
        );
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
            document_from_yaml(&yaml, record.id.as_str()).unwrap(),
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

    #[test]
    fn document_read_write_round_trips_through_disk() {
        let root = std::env::temp_dir().join(format!(
            "vtest-store-canonical-doc-{}",
            crate::new_record_id()
        ));
        let layout = crate::init_project(&root, "example").unwrap();
        let record = sample_document();

        write_document(&layout, &record).unwrap();
        assert_eq!(read_document(&layout, record.id.as_str()).unwrap(), record);
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
        assert_eq!(
            vo_record_from_yaml(&yaml, record.id.as_str()).unwrap(),
            record
        );
    }

    #[test]
    fn vo_record_without_parent_or_optional_fields_round_trips() {
        let record = VoRecord {
            id: VoId::new("VO-ROOT-001"),
            parent: None,
            derives_from: vec![],
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
            vo_record_from_yaml(&yaml, record.id.as_str()).unwrap(),
            record
        );
    }

    #[test]
    fn vo_record_status_read_compat_field_is_ignored() {
        let record = sample_vo();
        let mut yaml = vo_record_to_yaml(&record);
        yaml.push_str("status: draft\n");
        assert_eq!(
            vo_record_from_yaml(&yaml, record.id.as_str()).unwrap(),
            record
        );
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
        let yaml = vo_record_to_yaml(&sample_vo()).replace(
            "coverage_policy: 'full-product'",
            "coverage_policy: 'bogus'",
        );
        let error = vo_record_from_yaml(&yaml, "VO-PARSER-UTF8-003").expect_err(
            "an unrecognized coverage_policy must fail closed, not silently become None",
        );
        assert!(error.to_string().contains("coverage_policy"));
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
        assert_eq!(read_vo_record(&layout, record.id.as_str()).unwrap(), record);
    }
}
