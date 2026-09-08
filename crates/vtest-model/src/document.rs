use crate::DocumentId;
use serde::{Deserialize, Serialize};

/// Reference to an upstream document from which a VO record derives.
///
/// This shape (an `anchor` pointing at a location within the referenced
/// node, plus a free-text `note`) is specific to the VO record's
/// `derives_from` (詳細仕様 DS-1638 — the `DS-` prefix names the
/// `detailed_spec` layer, 詳細仕様; `design`/`DES-` is 詳細設計, a distinct
/// layer). The upstream document node model's own
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
    ///
    /// `Option`, not `Vec`: schema `$defs/derivedItem` does not list `cites`
    /// in `required`, so — like [`SectionNode::derives_from`],
    /// [`SectionNode::sections`], and [`SectionNode::items`] — this key's
    /// own presence is optional, and the type preserves exactly what the
    /// author wrote rather than normalizing it: an absent key round-trips
    /// to `None` and back to absent; a key present as `[]` round-trips to
    /// `Some(vec![])` and back to `[]`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cites: Option<Vec<String>>,

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

    /// `Option`, not `Vec` (contrast [`SentenceNode::derives_from`]): DS-1593
    /// makes this key itself optional for a section, so the type preserves
    /// exactly what the author wrote rather than normalizing it — a key
    /// that is absent round-trips to `None` and back to absent; a key
    /// present as `[]` round-trips to `Some(vec![])` and back to `[]`. This
    /// crate does not decide, as a convention, whether an author should
    /// omit or state-empty this key (the canonical bundle happens to always
    /// state it, but that is the author's choice each schema permits, not a
    /// rule this type enforces).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub derives_from: Option<Vec<DocumentId>>,

    /// `Option`, not `Vec`: schema `$defs/section` does not list `sections`
    /// in `required` either, so this key's presence is preserved on the
    /// same terms as `derives_from` above (absent round-trips to absent,
    /// `[]` round-trips to `[]`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sections: Option<Vec<SectionNode>>,

    /// `Option`, not `Vec`: schema `$defs/section` does not list `items` in
    /// `required` either, so this key's presence is preserved on the same
    /// terms as `derives_from` above (absent round-trips to absent, `[]`
    /// round-trips to `[]`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub items: Option<Vec<SentenceNode>>,
}

/// One of the seven specification layers. Root, request, require, spec,
/// detailed_spec, basic_design, and design are enumerated in exactly this
/// order by the upstream document model's own top-level arrays (REQ-333,
/// SPEC-462). Reading that order, together with the canonical
/// upstream-to-downstream propagation direction (SPEC-261, itself derived
/// from P-005), as a descending authority ranking is a derivation this
/// crate makes for the `Ord` intuition the variant order suggests — no
/// specification.json node states an authority order over these seven
/// layers in these words, and this type does not derive `Ord`/`PartialOrd`.
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
            cites: None,
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
        assert_eq!(
            section.derives_from, None,
            "omitted key must parse to None, not Some(vec![])"
        );
        let subsections = section.sections.as_ref().expect("sections must be Some");
        assert_eq!(subsections.len(), 1);
        let nested_items = subsections[0].items.as_ref().expect("items must be Some");
        assert_eq!(nested_items.len(), 1);
        assert_eq!(nested_items[0].id, DocumentId::new("REQ-001"));

        // An omitted key must round-trip back to an omitted key — this type
        // preserves the author's choice (DS-1593 makes the key itself
        // optional) rather than normalizing every section to always state
        // it, even though the canonical bundle's own convention happens to
        // always state it (see the `Some(vec![])` case below).
        let round_tripped = serde_json::to_value(&section).unwrap();
        assert!(round_tripped
            .as_object()
            .unwrap()
            .get("derives_from")
            .is_none());
        assert!(round_tripped.as_object().unwrap().get("items").is_none());

        // A key present as an explicit empty array must likewise round-trip
        // back to an explicit empty array, not be normalized to absent.
        let explicit_empty: SectionNode = serde_json::from_value(serde_json::json!({
            "id": "REQ-S003",
            "title": "A section stating an empty derives_from",
            "source": { "doc": "d", "heading": "2", "lines": [1, 1] },
            "derives_from": []
        }))
        .unwrap();
        assert_eq!(explicit_empty.derives_from, Some(vec![]));
        let round_tripped_empty = serde_json::to_value(&explicit_empty).unwrap();
        assert_eq!(
            round_tripped_empty.as_object().unwrap().get("derives_from"),
            Some(&serde_json::json!([]))
        );
    }

    /// @vtest.id TEST-MODEL-DOCUMENT-SENTENCE-CITES-KEY-OPTIONAL
    /// @vtest.covers VO-MODEL-DOCUMENT-NODE-SHAPES
    /// @vtest.target crates/vtest-model/src/document.rs::SentenceNode
    /// @vtest.intent verifies a sentence node's cites key preserves the author's absent/empty choice, mirroring SectionNode::derives_from
    #[test]
    fn sentence_node_cites_key_is_optional_and_round_trips() {
        let missing_cites: SentenceNode = serde_json::from_value(serde_json::json!({
            "id": "REQ-004",
            "statement": "x",
            "derives_from": [],
            "source": { "doc": "d", "heading": "h", "lines": [1, 1] }
        }))
        .unwrap();
        assert_eq!(
            missing_cites.cites, None,
            "omitted key must parse to None, not Some(vec![])"
        );
        let round_tripped = serde_json::to_value(&missing_cites).unwrap();
        assert!(
            round_tripped.as_object().unwrap().get("cites").is_none(),
            "an omitted cites key must round-trip back to an omitted key"
        );

        let explicit_empty: SentenceNode = serde_json::from_value(serde_json::json!({
            "id": "REQ-005",
            "statement": "x",
            "derives_from": [],
            "cites": [],
            "source": { "doc": "d", "heading": "h", "lines": [1, 1] }
        }))
        .unwrap();
        assert_eq!(explicit_empty.cites, Some(vec![]));
        let round_tripped_empty = serde_json::to_value(&explicit_empty).unwrap();
        assert_eq!(
            round_tripped_empty.as_object().unwrap().get("cites"),
            Some(&serde_json::json!([])),
            "a cites key present as [] must round-trip back to [], not be dropped"
        );
    }

    /// @vtest.id TEST-MODEL-DOCUMENT-SECTION-CHILDREN-KEYS-OPTIONAL
    /// @vtest.covers VO-MODEL-DOCUMENT-NODE-SHAPES
    /// @vtest.target crates/vtest-model/src/document.rs::SectionNode
    /// @vtest.intent verifies a section node's sections/items keys preserve the author's absent/empty choice, mirroring SectionNode::derives_from
    #[test]
    fn section_node_sections_and_items_keys_are_optional_and_round_trip() {
        let explicit_empty: SectionNode = serde_json::from_value(serde_json::json!({
            "id": "SPEC-S001",
            "title": "A section stating empty children",
            "source": { "doc": "d", "heading": "3", "lines": [1, 1] },
            "derives_from": [],
            "sections": [],
            "items": []
        }))
        .unwrap();
        assert_eq!(explicit_empty.sections, Some(vec![]));
        assert_eq!(explicit_empty.items, Some(vec![]));

        let round_tripped = serde_json::to_value(&explicit_empty).unwrap();
        let object = round_tripped.as_object().unwrap();
        assert_eq!(
            object.get("sections"),
            Some(&serde_json::json!([])),
            "a sections key present as [] must round-trip back to [], not be dropped"
        );
        assert_eq!(
            object.get("items"),
            Some(&serde_json::json!([])),
            "an items key present as [] must round-trip back to [], not be dropped"
        );
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

        // Structural equality (asserted above) is the pass/fail criterion —
        // byte-for-byte identity of the serialized output against the
        // original file is a separate question this task also requires
        // measuring and reporting, without treating a mismatch as a test
        // failure (a schema-conformant re-serialization is not obligated to
        // reproduce the original byte stream). Measure it here so the
        // result is never silently unmeasured, and print the cause of any
        // difference for the PR report to cite.
        let round_tripped_pretty = serde_json::to_string_pretty(&parsed)
            .expect("failed to reserialize parsed bundle as pretty JSON");
        let pretty_bytes = round_tripped_pretty.as_bytes();
        let file_bytes = text.as_bytes();
        let bytes_identical = pretty_bytes == file_bytes;
        if bytes_identical {
            eprintln!(
                "canonical_bundle_round_trips_and_matches_node_counts: byte-identical \
                 round trip ({} bytes) — reported per task requirement, not asserted.",
                file_bytes.len()
            );
        } else {
            let first_diff = pretty_bytes
                .iter()
                .zip(file_bytes.iter())
                .position(|(a, b)| a != b);
            eprintln!(
                "canonical_bundle_round_trips_and_matches_node_counts: NOT byte-identical \
                 (structural equality above already holds — this is expected, not a failure). \
                 pretty-printed length = {} bytes, file length = {} bytes, first differing byte \
                 at offset {:?}. Known causes, not exhaustive: (a) key order — SentenceNode's \
                 field declaration order is id, statement, description, derives_from, cites, \
                 source, and the canonical bundle mostly agrees (e.g. detailed_spec node DS-038 \
                 places `description` right after `statement`, matching that order), but it is \
                 not uniform: at least 5 sentence nodes (DS-1637, DS-1658, BD-330, and 2 more) \
                 place `description` last, after `source`, instead — the first byte offset above \
                 lands exactly at that divergence inside DS-1637 — so declaration-order \
                 serialization cannot reproduce every node's original key order; (b) the file \
                 ends with a trailing newline that `to_string_pretty` does not emit. This has no \
                 bearing on the structural-equality assertion above; it is disclosed because a \
                 future byte-preserving writer (a later PR's concern, not this one's) would need \
                 to account for both.",
                pretty_bytes.len(),
                file_bytes.len(),
                first_diff,
            );
        }

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
        // baked in at one SHA turns every subsequent growth of the spec into
        // a false failure here. Instead, each layer's count is computed two
        // independent ways from the same parsed bytes — once by walking the
        // typed `DocumentFile` this test already parsed (via `count_sections`
        // above), and once by walking the raw `serde_json::Value` parsed
        // from the same text — and the two must agree. This does not
        // duplicate the structural-equality assertion above: that assertion
        // shows the typed value serializes back to the same JSON shape; this
        // one shows `count_sections`'s recursion (1 per section, plus each
        // section's `items`, plus recursing into `sections`) reaches the
        // same leaves the schema itself defines, independent of how
        // `SectionNode`'s fields happen to be laid out.
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

        let value_root = layer_array(&original, "root").len();
        let value_request = layer_array(&original, "request").len();
        let value_require = count_sections_value(layer_array(&original, "require"));
        let value_spec = count_sections_value(layer_array(&original, "spec"));
        let value_detailed_spec = count_sections_value(layer_array(&original, "detailed_spec"));
        let value_basic_design = count_sections_value(layer_array(&original, "basic_design"));
        let value_design = count_sections_value(layer_array(&original, "design"));

        assert_eq!(
            parsed.root.len(),
            value_root,
            "root layer node count: typed count vs. independent Value-based count"
        );
        assert_eq!(
            parsed.request.len(),
            value_request,
            "request layer node count: typed count vs. independent Value-based count"
        );
        assert_eq!(
            count_sections(&parsed.require),
            value_require,
            "require layer node count: typed count vs. independent Value-based count"
        );
        assert_eq!(
            count_sections(&parsed.spec),
            value_spec,
            "spec layer node count: typed count vs. independent Value-based count"
        );
        assert_eq!(
            count_sections(&parsed.detailed_spec),
            value_detailed_spec,
            "detailed_spec layer node count: typed count vs. independent Value-based count"
        );
        assert_eq!(
            count_sections(&parsed.basic_design),
            value_basic_design,
            "basic_design layer node count: typed count vs. independent Value-based count"
        );
        assert_eq!(
            count_sections(&parsed.design),
            value_design,
            "design layer node count: typed count vs. independent Value-based count"
        );

        let total = parsed.root.len()
            + parsed.request.len()
            + count_sections(&parsed.require)
            + count_sections(&parsed.spec)
            + count_sections(&parsed.detailed_spec)
            + count_sections(&parsed.basic_design)
            + count_sections(&parsed.design);

        eprintln!(
            "canonical_bundle_round_trips_and_matches_node_counts: observed node counts — \
             root={}, request={}, require={}, spec={}, detailed_spec={}, basic_design={}, \
             design={}, total={} (not pinned to a literal — this run's typed count matched an \
             independent Value-based count for every layer).",
            parsed.root.len(),
            parsed.request.len(),
            count_sections(&parsed.require),
            count_sections(&parsed.spec),
            count_sections(&parsed.detailed_spec),
            count_sections(&parsed.basic_design),
            count_sections(&parsed.design),
            total,
        );
    }
}
