use crate::DocumentId;
use serde::{Deserialize, Serialize};

/// Reference to an upstream document from which a VO record derives.
///
/// This shape (an `anchor` pointing at a location within the referenced
/// node, plus a free-text `note`) is specific to the VO record's
/// `derives_from` (詳細設計 DS-1638). The upstream document node model's own
/// `derives_from` is a bare list of node ids and does not use this type
/// (DS-1594, DS-1595) — see [`RootNode`], [`SentenceNode`], [`SectionNode`].
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct DerivesFrom {
    pub doc: DocumentId,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub anchor: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
}

/// Provenance of a node in the upstream document model: the migration-period
/// record of which markdown file, heading, and line range it was written
/// from (schema `$defs/source`, required on every node kind — DES-568).
///
/// `lines` is a `[start, end]` pair, both 1-based and inclusive, matching
/// the schema's `minItems`/`maxItems` of 2.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NodeSource {
    pub doc: String,
    pub heading: String,
    pub lines: [u32; 2],
}

/// A root-layer node: one of Issue #11's frozen items (F1-F12) or an Owner
/// ruling appearing as upstream in the requirements' derivation table.
///
/// The root layer is the sole layer with no `derives_from` — it is the root
/// of the derivation graph, not derived from anything else in this model
/// (schema `$defs/rootItem`; DES-568).
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RootNode {
    pub id: DocumentId,
    pub statement: String,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    pub source: NodeSource,
}

/// A sentence ("文") node: one normative statement, required to declare its
/// upstream node ids even when that list is empty (schema `$defs/derivedItem`;
/// DES-568, DS-1592, DS-1593).
///
/// `derives_from` holds bare upstream node ids with no `anchor` or `note`
/// per entry (DS-1594, DS-1595) — do not reuse [`DerivesFrom`] here, which is
/// the VO record's distinct, richer shape (DS-1638).
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SentenceNode {
    pub id: DocumentId,
    pub statement: String,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    pub derives_from: Vec<DocumentId>,

    /// Verbatim upstream citations carried over from the source markdown
    /// during the migration period (e.g. `"基本仕様 §3.2"`); resolving these
    /// into `derives_from` entries is a separate, Owner-approved step.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub cites: Vec<String>,

    pub source: NodeSource,
}

/// A section ("節") node: one node of the document > section > subsection >
/// sentence tree, built mechanically from heading numbers (schema
/// `$defs/section`; DES-568, DS-1592).
///
/// Unlike a sentence node, a section's `derives_from` is optional (DS-1593):
/// a section may have children but declare no upstream itself. A section
/// may hold child sections, child sentences, both, or neither (a heading
/// with no direct sentence beneath it is still a valid section).
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SectionNode {
    pub id: DocumentId,
    pub title: String,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    pub source: NodeSource,

    /// Schema-optional (DS-1593: a section may omit this key entirely), but
    /// the canonical bundle's own convention always writes it — even as an
    /// empty array — rather than omitting it, so it is not marked
    /// `skip_serializing_if` here: input tolerates omission (`#[serde(default)]`),
    /// output always states it explicitly, matching observed practice.
    #[serde(default)]
    pub derives_from: Vec<DocumentId>,

    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub sections: Vec<SectionNode>,

    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub items: Vec<SentenceNode>,
}

/// One of the seven specification layers, in descending authority order
/// (root > request > require > spec > detailed_spec > basic_design >
/// design — AGENTS.md "Authority"; BD-318).
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Layer {
    Root,
    Request,
    Require,
    Spec,
    DetailedSpec,
    BasicDesign,
    Design,
}

impl Layer {
    /// Resolves the layer implied by a node id's prefix (BD-318: `ROOT-` /
    /// `R-` / `P-` / `REQ-` / `SPEC-` / `DS-` / `BD-` / `DES-`, with a
    /// section id additionally carrying `-S`). `P-` and `REQ-` both resolve
    /// to [`Layer::Require`] — the require layer's top-level array mixes
    /// `REQ-` section/sentence ids with `P-` principle sentences nested
    /// under them (observed in `specification.json`; e.g. `P-001`..`P-005`
    /// nested under `require[1]`).
    ///
    /// Returns `None` for an id matching no known prefix.
    ///
    /// This function only maps a prefix to its layer. Whether a *specific*
    /// node's id prefix matches the layer of the top-level array it was
    /// placed in (DS-1658) is a reader-side validation, not performed here.
    pub fn from_id_prefix(id: &str) -> Option<Layer> {
        if id.starts_with("ROOT-") {
            Some(Layer::Root)
        } else if id.starts_with("R-") {
            Some(Layer::Request)
        } else if id.starts_with("P-") || id.starts_with("REQ-") {
            Some(Layer::Require)
        } else if id.starts_with("SPEC-") {
            Some(Layer::Spec)
        } else if id.starts_with("DS-") {
            Some(Layer::DetailedSpec)
        } else if id.starts_with("BD-") {
            Some(Layer::BasicDesign)
        } else if id.starts_with("DES-") {
            Some(Layer::Design)
        } else {
            None
        }
    }
}

/// An upstream document file, or the full bundle (`specification.json`)
/// that concatenates every document's same-named layer arrays.
///
/// Both share one shape: `schema_version` plus all seven top-level layer
/// arrays, with the arrays for layers the document does not touch left
/// empty (DES-586). The file carries no field identifying which document it
/// is — the filename itself, chosen by the document's author, is the
/// identity (BD-330, DES-585).
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DocumentFile {
    pub schema_version: String,
    pub root: Vec<RootNode>,
    pub request: Vec<SentenceNode>,
    pub require: Vec<SectionNode>,
    pub spec: Vec<SectionNode>,
    pub detailed_spec: Vec<SectionNode>,
    pub basic_design: Vec<SectionNode>,
    pub design: Vec<SectionNode>,
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

    fn sample_source() -> NodeSource {
        NodeSource {
            doc: "docs/spec.md".to_string(),
            heading: "1".to_string(),
            lines: [1, 1],
        }
    }

    /// @vtest.id TEST-MODEL-DOCUMENT-ROOT-NODE-NO-DERIVES-FROM
    /// @vtest.covers VO-MODEL-DOCUMENT-NODE-SHAPES
    /// @vtest.target crates/vtest-model/src/document.rs::RootNode
    /// @vtest.intent verifies the root layer's node carries no derives_from field
    #[test]
    fn root_node_has_no_derives_from_field() {
        let node = RootNode {
            id: DocumentId::new("ROOT-001"),
            statement: "Frozen ruling.".to_string(),
            description: None,
            source: sample_source(),
        };

        let json = serde_json::to_value(&node).unwrap();
        assert!(json.as_object().unwrap().get("derives_from").is_none());
        assert_eq!(
            serde_json::from_value::<RootNode>(json).unwrap(),
            node,
            "round trip must reproduce the original node"
        );
    }

    /// @vtest.id TEST-MODEL-DOCUMENT-SENTENCE-DERIVES-FROM-REQUIRED
    /// @vtest.covers VO-MODEL-DOCUMENT-NODE-SHAPES
    /// @vtest.target crates/vtest-model/src/document.rs::SentenceNode
    /// @vtest.intent verifies a sentence node's derives_from is present (possibly empty) and holds bare ids
    #[test]
    fn sentence_node_requires_derives_from_key_even_when_empty() {
        let node = SentenceNode {
            id: DocumentId::new("REQ-001"),
            statement: "A requirement.".to_string(),
            description: None,
            derives_from: vec![],
            cites: vec![],
            source: sample_source(),
        };

        let json = serde_json::to_string(&node).unwrap();
        assert!(json.contains(r#""derives_from":[]"#));
        assert_eq!(serde_json::from_str::<SentenceNode>(&json).unwrap(), node);

        // The key must be present on the wire, not merely defaultable on read.
        let missing_key =
            r#"{"id":"REQ-002","statement":"x","source":{"doc":"d","heading":"h","lines":[1,1]}}"#;
        assert!(serde_json::from_str::<SentenceNode>(missing_key).is_err());
    }

    /// @vtest.id TEST-MODEL-DOCUMENT-SECTION-DERIVES-FROM-OPTIONAL
    /// @vtest.covers VO-MODEL-DOCUMENT-NODE-SHAPES
    /// @vtest.target crates/vtest-model/src/document.rs::SectionNode
    /// @vtest.intent verifies a section node may omit derives_from and may nest sections and items
    #[test]
    fn section_node_derives_from_is_optional_and_nests() {
        let json = serde_json::json!({
            "id": "REQ-S001",
            "title": "A section",
            "source": { "doc": "d", "heading": "1", "lines": [1, 2] },
            "sections": [
                {
                    "id": "REQ-S002",
                    "title": "A subsection",
                    "source": { "doc": "d", "heading": "1.1", "lines": [1, 1] },
                    "items": [
                        {
                            "id": "REQ-001",
                            "statement": "nested sentence",
                            "derives_from": [],
                            "source": { "doc": "d", "heading": "1.1", "lines": [1, 1] }
                        }
                    ]
                }
            ]
        });

        let section: SectionNode = serde_json::from_value(json).unwrap();
        assert!(section.derives_from.is_empty());
        assert_eq!(section.sections.len(), 1);
        assert_eq!(section.sections[0].items.len(), 1);
        assert_eq!(section.sections[0].items[0].id, DocumentId::new("REQ-001"));

        // Deserialization tolerates the omitted key (schema-optional,
        // DS-1593), but serialization always states it explicitly — even
        // empty — matching the canonical bundle's own convention (verified
        // against specification.json: every section node carries this key).
        let round_tripped = serde_json::to_value(&section).unwrap();
        assert_eq!(
            round_tripped.as_object().unwrap().get("derives_from"),
            Some(&serde_json::json!([]))
        );
        assert!(round_tripped.as_object().unwrap().get("items").is_none());
    }

    /// @vtest.id TEST-MODEL-DOCUMENT-NODE-REJECTS-UNKNOWN-FIELD
    /// @vtest.covers VO-MODEL-DOCUMENT-SCHEMA-CONFORMANCE
    /// @vtest.target crates/vtest-model/src/document.rs::SentenceNode
    /// @vtest.intent verifies an undeclared field on a sentence node is rejected (DS-1645)
    #[test]
    fn sentence_node_rejects_unknown_field() {
        let json = r#"{
            "id": "REQ-001",
            "statement": "x",
            "derives_from": [],
            "source": { "doc": "d", "heading": "h", "lines": [1, 1] },
            "unexpected": "surplus"
        }"#;
        assert!(serde_json::from_str::<SentenceNode>(json).is_err());
    }

    /// @vtest.id TEST-MODEL-DOCUMENT-NODE-DERIVES-FROM-REJECTS-ANCHOR-NOTE
    /// @vtest.covers VO-MODEL-DOCUMENT-SCHEMA-CONFORMANCE
    /// @vtest.target crates/vtest-model/src/document.rs::SentenceNode
    /// @vtest.intent verifies a document node's derives_from entry cannot carry anchor/note (DS-1594, DS-1595) — that shape is VO-only (DS-1638)
    #[test]
    fn sentence_node_derives_from_entry_rejects_anchor_note_object() {
        let json = r#"{
            "id": "REQ-001",
            "statement": "x",
            "derives_from": [{ "doc": "ROOT-001", "anchor": "x" }],
            "source": { "doc": "d", "heading": "h", "lines": [1, 1] }
        }"#;
        assert!(serde_json::from_str::<SentenceNode>(json).is_err());
    }

    #[test]
    fn layer_from_id_prefix_matches_bd_318() {
        assert_eq!(Layer::from_id_prefix("ROOT-001"), Some(Layer::Root));
        assert_eq!(Layer::from_id_prefix("R-1"), Some(Layer::Request));
        assert_eq!(Layer::from_id_prefix("P-001"), Some(Layer::Require));
        assert_eq!(Layer::from_id_prefix("REQ-001"), Some(Layer::Require));
        assert_eq!(Layer::from_id_prefix("REQ-S001"), Some(Layer::Require));
        assert_eq!(Layer::from_id_prefix("SPEC-001"), Some(Layer::Spec));
        assert_eq!(Layer::from_id_prefix("DS-001"), Some(Layer::DetailedSpec));
        assert_eq!(Layer::from_id_prefix("BD-001"), Some(Layer::BasicDesign));
        assert_eq!(Layer::from_id_prefix("DES-001"), Some(Layer::Design));
        assert_eq!(Layer::from_id_prefix("XYZ-001"), None);
    }

    /// @vtest.id TEST-MODEL-DOCUMENT-FILE-NO-IDENTIFYING-FIELD
    /// @vtest.covers VO-MODEL-DOCUMENT-FILE-SHAPE
    /// @vtest.target crates/vtest-model/src/document.rs::DocumentFile
    /// @vtest.intent verifies the document file shape has no field identifying which document it is (BD-330, DES-585)
    #[test]
    fn document_file_has_no_identifying_field() {
        let file = DocumentFile {
            schema_version: "0.1".to_string(),
            root: vec![],
            request: vec![],
            require: vec![],
            spec: vec![],
            detailed_spec: vec![],
            basic_design: vec![],
            design: vec![],
        };

        let json = serde_json::to_value(&file).unwrap();
        let object = json.as_object().unwrap();
        for key in ["id", "path", "content_hash", "title", "registered_at"] {
            assert!(
                !object.contains_key(key),
                "unexpected identifying field: {key}"
            );
        }
        assert_eq!(serde_json::from_value::<DocumentFile>(json).unwrap(), file);
    }

    /// @vtest.id TEST-MODEL-DOCUMENT-FILE-REJECTS-UNKNOWN-TOP-LEVEL-FIELD
    /// @vtest.covers VO-MODEL-DOCUMENT-SCHEMA-CONFORMANCE
    /// @vtest.target crates/vtest-model/src/document.rs::DocumentFile
    /// @vtest.intent verifies an undeclared top-level field on a document file is rejected (DS-1645)
    #[test]
    fn document_file_rejects_unknown_top_level_field() {
        let json = r#"{
            "schema_version": "0.1",
            "root": [], "request": [], "require": [], "spec": [],
            "detailed_spec": [], "basic_design": [], "design": [],
            "id": "surplus"
        }"#;
        assert!(serde_json::from_str::<DocumentFile>(json).is_err());
    }

    /// @vtest.id TEST-MODEL-DOCUMENT-FILE-ROUND-TRIPS-FROM-FIXTURE
    /// @vtest.covers VO-MODEL-DOCUMENT-FILE-SHAPE
    /// @vtest.target crates/vtest-model/src/document.rs::DocumentFile
    /// @vtest.intent verifies a small representative fixture parses, then re-serializes to a structurally identical JSON value
    #[test]
    fn document_file_round_trips_from_small_fixture() {
        let text = include_str!("../tests/fixtures/document_sample.json");
        let original: serde_json::Value = serde_json::from_str(text).unwrap();
        let parsed: DocumentFile = serde_json::from_str(text).unwrap();
        let round_tripped = serde_json::to_value(&parsed).unwrap();
        assert_eq!(round_tripped, original);
    }

    #[test]
    fn document_file_rejects_unknown_field_fixtures() {
        let top_level = include_str!("../tests/fixtures/document_unknown_field_top_level.json");
        let section = include_str!("../tests/fixtures/document_unknown_field_section.json");
        let sentence = include_str!("../tests/fixtures/document_unknown_field_sentence.json");

        for (name, text) in [
            ("top_level", top_level),
            ("section", section),
            ("sentence", sentence),
        ] {
            assert!(
                serde_json::from_str::<DocumentFile>(text).is_err(),
                "fixture should be rejected: {name}"
            );
        }
    }

    #[test]
    fn document_file_rejects_anchor_note_derives_from_fixture() {
        let text = include_str!("../tests/fixtures/document_derives_from_anchor_rejected.json");
        assert!(serde_json::from_str::<DocumentFile>(text).is_err());
    }

    /// Real-bundle round trip: only runs when `VTEST_CANONICAL_BUNDLE` names
    /// the canonical `specification.json`. Not run by default because it
    /// depends on a file outside this crate's fixtures.
    #[test]
    #[ignore = "requires VTEST_CANONICAL_BUNDLE env var pointing at the canonical specification.json"]
    fn canonical_bundle_round_trips_and_matches_node_counts() {
        let path = std::env::var("VTEST_CANONICAL_BUNDLE")
            .expect("set VTEST_CANONICAL_BUNDLE to the canonical specification.json path");
        let text = std::fs::read_to_string(&path).expect("failed to read canonical bundle");

        let original: serde_json::Value =
            serde_json::from_str(&text).expect("bundle is not valid JSON");
        let parsed: DocumentFile =
            serde_json::from_str(&text).expect("bundle does not parse as DocumentFile");
        let round_tripped = serde_json::to_value(&parsed).expect("failed to reserialize");

        assert_eq!(
            round_tripped, original,
            "parse -> serialize -> parse changed the bundle's structure"
        );

        fn count_sections(sections: &[SectionNode]) -> usize {
            sections
                .iter()
                .map(|s| 1 + s.items.len() + count_sections(&s.sections))
                .sum()
        }

        assert_eq!(parsed.root.len(), 48, "root layer node count");
        assert_eq!(parsed.request.len(), 5, "request layer node count");
        assert_eq!(
            count_sections(&parsed.require),
            395,
            "require layer node count"
        );
        assert_eq!(count_sections(&parsed.spec), 593, "spec layer node count");
        assert_eq!(
            count_sections(&parsed.detailed_spec),
            1734,
            "detailed_spec layer node count"
        );
        assert_eq!(
            count_sections(&parsed.basic_design),
            408,
            "basic_design layer node count"
        );
        assert_eq!(
            count_sections(&parsed.design),
            663,
            "design layer node count"
        );

        let total = parsed.root.len()
            + parsed.request.len()
            + count_sections(&parsed.require)
            + count_sections(&parsed.spec)
            + count_sections(&parsed.detailed_spec)
            + count_sections(&parsed.basic_design)
            + count_sections(&parsed.design);
        assert_eq!(total, 3846, "total node count across all layers");
    }
}
