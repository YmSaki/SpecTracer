use crate::{ContentHash, DocumentId};
use serde::{Deserialize, Serialize};

/// Reference to an upstream document from which a record derives.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct DerivesFrom {
    pub doc: DocumentId,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub anchor: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
}

/// Canonical metadata record for a registered upstream document.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct DocumentRecord {
    pub id: DocumentId,
    pub path: String,

    pub content_hash: ContentHash,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,

    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub derives_from: Vec<DerivesFrom>,
    pub registered_at: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn derives_from_serializes_correctly() {
        let derives_from = DerivesFrom {
            doc: DocumentId::new("doc123"),
            anchor: Some("anchor456".to_string()),
            note: Some("This is a note.".to_string()),
        };

        assert_eq!(
            serde_json::to_string(&derives_from).unwrap(),
            r#"{"doc":"doc123","anchor":"anchor456","note":"This is a note."}"#
        );

        let derives_from_no_anchor = DerivesFrom {
            doc: DocumentId::new("doc789"),
            anchor: None,
            note: Some("Another note.".to_string()),
        };

        assert_eq!(
            serde_json::to_string(&derives_from_no_anchor).unwrap(),
            r#"{"doc":"doc789","note":"Another note."}"#
        );

        let derives_from_no_note = DerivesFrom {
            doc: DocumentId::new("doc101"),
            anchor: Some("anchor202".to_string()),
            note: None,
        };

        assert_eq!(
            serde_json::to_string(&derives_from_no_note).unwrap(),
            r#"{"doc":"doc101","anchor":"anchor202"}"#
        );

        let derives_only_doc = DerivesFrom {
            doc: DocumentId::new("doc303"),
            anchor: None,
            note: None,
        };

        assert_eq!(
            serde_json::to_string(&derives_only_doc).unwrap(),
            r#"{"doc":"doc303"}"#
        );
    }

    #[test]
    fn document_record_serializes_correctly() {
        let hash = ContentHash::from_text("hash123");

        let record = DocumentRecord {
            id: DocumentId::new("doc123"),
            path: "/path/to/document".into(),
            content_hash: hash,
            title: Some("Document Title".into()),
            derives_from: vec![],
            registered_at: "2023-01-01T00:00:00Z".into(),
        };

        let serialized = serde_json::to_string(&record).unwrap();
        let hash_text = record.content_hash.to_string();
        let expected = format!(
            r#"{{"id":"doc123","path":"/path/to/document","content_hash":"{hash_text}","title":"Document Title","registered_at":"2023-01-01T00:00:00Z"}}"#,
        );
        assert_eq!(serialized, expected);
    }
}
