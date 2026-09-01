//! Canonical v0.1 record storage (詳細設計 v0.1 §3), for the types defined in
//! `vtest-model`. Kept separate from `records.rs`'s predecessor Req/Spec-model
//! types so neither module's exports collide with the other's.

use crate::records::{scalar, unquote, yaml_scalar};
use crate::{read_text, write_atomic, StoreError, VerifyLayout};
use vtest_model::{ContentHash, DerivesFrom, DocumentId, DocumentRecord};

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
pub fn document_from_yaml(text: &str, fallback_id: &str) -> Result<DocumentRecord, StoreError> {
    let id = scalar(text, "id").unwrap_or_else(|| fallback_id.to_owned());
    let content_hash: ContentHash = scalar(text, "content_hash")
        .ok_or_else(|| StoreError::InvalidConfig("document is missing content_hash".to_owned()))?
        .parse()
        .map_err(|error: String| StoreError::InvalidConfig(error))?;
    Ok(DocumentRecord {
        id: DocumentId::new(id),
        path: scalar(text, "path").unwrap_or_default(),
        content_hash,
        title: scalar(text, "title"),
        derives_from: parse_derives_from(text),
        registered_at: scalar(text, "registered_at").unwrap_or_default(),
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

#[cfg(test)]
mod tests {
    use super::*;

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
}
