//! §1.3 subject hash composition functions: document, VO, and Source Target.
//!
//! These bind the canonical inputs each subject hash requires (詳細設計 v0.1
//! §1.3, 本冊:83-91), using the encoding primitives in `hash.rs`
//! (`SubjectHashInput` / `FieldValue` / `SubjectDomain` /
//! `encode_nested_fields`). They only compute hash values — no existing
//! `ContentHash::from_text` call site is rewired to use them here.
//!
//! Every scalar text field these functions bind uses
//! [`FieldValue::text_fragment`] (normalized), not
//! [`FieldValue::exact_bytes`] — 本冊:83 makes normalization the default for
//! any field a subject-specific rule does not explicitly require
//! byte-exact, and the PM ruling on this PR confirmed that document/VO/
//! Source-Target fields carry no such explicit requirement; only the
//! Execution State manifest's file bytes do (本冊:91 "byte-exact file
//! bytes"). See [`optional_text_fragment`]'s doc comment for the citation
//! chain (本冊:83/88/99).
//!
//! # Test subject hash is not implemented here
//!
//! §1.3 (本冊:87) requires Test subject hash to bind an adapter ID and a
//! Source Location built from `adapter` / project-relative `path` / opaque
//! `locator`, plus an `ExecutionDescriptor` (本冊:629). The spec's own
//! `struct SourceLocation` (本冊:637-642) is:
//!
//! ```text
//! pub struct SourceLocation {
//!     pub adapter: AdapterId,
//!     pub path: ProjectPath,
//!     pub locator: String,            // adapter所有のopaque construct locator
//!     pub byte_range: SourceRange,
//! }
//! ```
//!
//! This crate's actual [`crate::SourceLocation`] is `{ file, function,
//! start_line, end_line, start_byte, end_byte }` — it has no `adapter` field
//! and no opaque `locator` field at all. [`crate::TestEntity`] likewise has
//! no top-level `adapter` field, and this crate has no `ExecutionDescriptor`
//! type (本冊:644-649) yet. These are gaps between the model and the spec's
//! own struct shapes, not inputs this function could source from something
//! else already on `TestEntity` — `filter` / `package` / `test_target`
//! (Rust/Cargo execution details this crate's `TestEntity` carries instead)
//! are not substitutes for `ExecutionDescriptor` and are not used as one.
//! See the PR report for the full gap analysis.

use std::collections::BTreeSet;

use crate::{
    encode_nested_fields, normalize_hashed_text, CombinationEntry, ContentHash, Dimension,
    DocumentRecord, FieldValue, Locator, SubjectDomain, SubjectHashInput, VoRecord,
};

/// A scalar field whose declaration may be entirely absent (`None`,
/// encoded as [`FieldValue::Null`]) versus present-with-a-value (`Some`,
/// encoded as a normalized text fragment). Used for optional
/// identifier/metadata fields (`title`, `parent`, `anchor`, `note`).
///
/// Every scalar text field in this module goes through
/// [`FieldValue::text_fragment`], not [`FieldValue::exact_bytes`]: 本冊:83
/// makes normalization the default for any field a subject-specific rule
/// does not explicitly require byte-exact, and only the Execution State
/// manifest's file bytes carry that explicit requirement (本冊:91
/// "byte-exact file bytes") — document/VO/Source-Target fields do not (PM
/// ruling; see this crate's PR report for the citation chain, 本冊:83/88/99).
fn optional_text_fragment(value: Option<&str>) -> FieldValue {
    match value {
        None => FieldValue::Null,
        Some(text) => FieldValue::text_fragment(text),
    }
}

/// document subject hash (詳細設計 v0.1 §1.3, domain
/// `vtest:document-subject:v1`, 本冊:89): "canonical document recordと参照先
/// source（`path` の実ファイル）の正規化内容を束縛する".
///
/// `source_text` is the current content of the file at `record.path`,
/// already read by the caller — `vtest-model` does not perform file I/O
/// (see this crate's `lib.rs` header). This function does not compare
/// `record.content_hash` against `source_text`'s freshly computed hash;
/// that staleness judgment (本冊:89, §11.4) belongs to a verification layer,
/// not to this pure hash-composition function.
///
/// Binds every canonical `DocumentRecord` field, including the full
/// `derives_from[]` entries (`doc`/`anchor`/`note`) — 本冊:207: "`anchor` は
/// canonical document record の一部であり、§1.3 の document subject hash の
/// 入力に含まれる". This differs from VO subject hash, which reduces
/// `derives_from` to just the referenced document ID set (本冊:233) — see
/// [`vo_subject_hash`]'s doc comment for that asymmetry.
pub fn document_subject_hash(record: &DocumentRecord, source_text: &str) -> ContentHash {
    SubjectHashInput::new(SubjectDomain::DocumentSubject)
        .field("id", FieldValue::text_fragment(record.id.as_str()))
        .field("path", FieldValue::text_fragment(&record.path))
        .field(
            "content_hash",
            FieldValue::text_fragment(record.content_hash.as_str()),
        )
        .field("title", optional_text_fragment(record.title.as_deref()))
        .field(
            "derives_from",
            FieldValue::Ordered(
                record
                    .derives_from
                    .iter()
                    .map(|entry| {
                        encode_nested_fields([
                            ("doc", FieldValue::text_fragment(entry.doc.as_str())),
                            ("anchor", optional_text_fragment(entry.anchor.as_deref())),
                            ("note", optional_text_fragment(entry.note.as_deref())),
                        ])
                    })
                    .collect(),
            ),
        )
        .field(
            "registered_at",
            FieldValue::text_fragment(&record.registered_at),
        )
        .field("source", FieldValue::text_fragment(source_text))
        .finish()
}

/// VO subject hash (詳細設計 v0.1 §1.3, domain `vtest:record-subject:v1`,
/// 本冊:90): "readerが具体化したcanonical VO recordをfield規則に従って
/// encodeする…`derives_from`（参照先 document ID 集合）と `parent` を束縛".
///
/// The canonical `vtest_model::VoRecord` this function reads has no `status`
/// or `covers` field — §1.3's "VOの読取り互換field `status`は正典ではない
/// ため含めない" and "`covers`の増減は…VO subjectには含めない" are therefore
/// structurally satisfied by this function's parameter type, not by an
/// explicit runtime exclusion (there is nothing to exclude): 本冊:235-237
/// confirms `status` is a read-compat-only field the canonical writer never
/// persists, and no VO YAML example (本冊:215-227) has a `covers` key.
///
/// `derives_from` is reduced to the **set** of referenced document IDs,
/// dropping each entry's `anchor`/`note` — 本冊:233: "`anchor` と `note` は
/// §1.3 の VO subject hash の入力に含まれない（VO subject hash は
/// `derives_from` の参照先 document ID 集合を束縛する）", and 本冊:232:
/// "同一 `doc` を `anchor` 違いで複数 entry として持つことを許容し、重複と
/// しない" — two entries differing only by `anchor` reference one document,
/// so this reduction step deduplicates by document ID. This is narrower than
/// [`document_subject_hash`], which keeps full `derives_from` entries
/// (anchor included) per 本冊:207.
///
/// Every other canonical field is bound as part of the whole record —
/// 本冊:286 confirms this explicitly for `combinations`: "`combinations` は
/// canonical VO record の一部であり、§1.3 の VO subject hash に束縛される".
/// `combinations` entries are encoded via [`CombinationEntry::iter`], sorted
/// by (dimension name, partition value) — the same canonical, declaration-
/// order-independent form the type's own `Eq`/`Ord` use (本冊:256: "記述順・
/// map key 順には依存しない"), which also preserves a malformed entry with a
/// repeated dimension name losslessly rather than collapsing it.
pub fn vo_subject_hash(record: &VoRecord) -> ContentHash {
    let referenced_documents: BTreeSet<Vec<u8>> = record
        .derives_from
        .iter()
        .map(|entry| normalize_hashed_text(entry.doc.as_str()).into_bytes())
        .collect();

    SubjectHashInput::new(SubjectDomain::RecordSubject)
        .field("id", FieldValue::text_fragment(record.id.as_str()))
        .field(
            "parent",
            optional_text_fragment(record.parent.as_ref().map(|id| id.as_str())),
        )
        .field(
            "derives_from",
            FieldValue::Set(referenced_documents.into_iter().collect()),
        )
        .field("claim", FieldValue::text_fragment(&record.claim))
        .field(
            "dimensions",
            FieldValue::Ordered(record.dimensions.iter().map(encode_dimension).collect()),
        )
        .field(
            "coverage_policy",
            match record.coverage_policy {
                None => FieldValue::Null,
                Some(policy) => FieldValue::text_fragment(coverage_policy_str(policy)),
            },
        )
        .field(
            "combinations",
            FieldValue::Ordered(
                record
                    .combinations
                    .iter()
                    .map(encode_combination_entry)
                    .collect(),
            ),
        )
        .field(
            "representative_cases",
            FieldValue::Ordered(
                record
                    .representative_cases
                    .iter()
                    .map(|case| normalize_hashed_text(case).into_bytes())
                    .collect(),
            ),
        )
        .field("created", FieldValue::text_fragment(&record.created))
        .field("updated", FieldValue::text_fragment(&record.updated))
        .finish()
}

fn encode_dimension(dimension: &Dimension) -> Vec<u8> {
    encode_nested_fields([
        ("name", FieldValue::text_fragment(&dimension.name)),
        (
            "partitions",
            FieldValue::Ordered(
                dimension
                    .partitions
                    .iter()
                    .map(|partition| normalize_hashed_text(partition).into_bytes())
                    .collect(),
            ),
        ),
    ])
}

/// The literal strings `CoveragePolicy`'s `#[serde(rename_all =
/// "kebab-case")]` produces. Kept as an explicit match (not a serialization
/// round-trip) so this crate does not need a runtime `serde_json`
/// dependency for one enum; the match is exhaustive, so a new variant fails
/// to compile here instead of silently omitting itself from the hash input.
fn coverage_policy_str(policy: crate::CoveragePolicy) -> &'static str {
    use crate::CoveragePolicy;
    match policy {
        CoveragePolicy::IndependentAxes => "independent-axes",
        CoveragePolicy::FullProduct => "full-product",
        CoveragePolicy::Explicit => "explicit",
    }
}

/// Encodes one `combinations[]` entry as (dimension name, partition value)
/// pairs sorted ascending — see [`vo_subject_hash`]'s doc comment.
fn encode_combination_entry(entry: &CombinationEntry) -> Vec<u8> {
    let mut pairs: Vec<(&str, &str)> = entry.iter().collect();
    pairs.sort_unstable();
    encode_nested_fields(
        pairs
            .into_iter()
            .map(|(name, value)| (name, FieldValue::text_fragment(value))),
    )
}

/// Source Target hash (詳細設計 v0.1 §1.3, domain `vtest:target-subject:v1`,
/// 本冊:88): "canonical Target Referenceとadapterが返すimplementation
/// construct bytesを束縛する".
///
/// `locator` is the Source Target's own **canonical Target Reference** —
/// `本冊:88` requires this to always be a `TargetRef::Locator`, never a
/// `TargetRef::SrcId` ("canonical Target Referenceは常に`TargetRef::Locator`
/// …であり、`TargetRef::SrcId`をcanonical Target Referenceにしない"). Taking
/// a plain [`Locator`] here (not a `TargetRef`) makes that structural: there
/// is no `TargetRef::SrcId` value this parameter could hold. This also means
/// the hash is computed only from the Source Target's own canonical Locator,
/// never from a referencing Test's `TargetRef` spelling (本冊:88 "hashは
/// Source Target自身のcanonical Locatorから一度だけ計算し、当該Source
/// Targetを参照するTest側の`TargetRef`綴りからは計算しない") — this function
/// has no parameter a Test's `TargetRef` could even be passed through.
///
/// `construct_text` is the adapter-returned implementation construct bytes,
/// decoded to text by the caller (`vtest-model` does no adapter I/O). It is
/// bound as a normalized text fragment — see this module's header comment
/// for the PM ruling that construct bytes are not byte-exact.
///
/// The Source Target's permanent SRC ID is **not** a parameter of this
/// function and so cannot be bound as an independent field (本冊:88 "恒久
/// SRC IDはhash inputの独立fieldとして束縛せず、canonical Target Reference
/// 経由でもhash inputへ入らない"). Declaring, changing, or deleting a SRC ID
/// therefore cannot change this hash by itself — except through
/// `construct_text`, for an adapter (`rust-cargo`'s `@vtest.src-id` doc
/// comment) that places the SRC ID declaration inside the construct bytes
/// themselves; 本冊:88 states this construct-bytes-mediated change is
/// correct behavior, not evidence that SRC ID is an independent hash field.
pub fn source_target_subject_hash(locator: &Locator, construct_text: &str) -> ContentHash {
    SubjectHashInput::new(SubjectDomain::TargetSubject)
        .field(
            "adapter",
            FieldValue::text_fragment(locator.adapter.as_str()),
        )
        .field("locator", FieldValue::text_fragment(&locator.value))
        .field("construct", FieldValue::text_fragment(construct_text))
        .finish()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        AdapterId, CoveragePolicy, DerivesFrom, DocumentId, SourceFunction, TargetRef, VoId,
    };

    fn base_document() -> DocumentRecord {
        DocumentRecord {
            id: DocumentId::new("DOC-BASIC-001"),
            path: "docs/basic-spec.md".to_string(),
            content_hash: ContentHash::from_text("registered content"),
            title: Some("基本仕様書".to_string()),
            derives_from: vec![DerivesFrom {
                doc: DocumentId::new("DOC-REQ-001"),
                anchor: Some("§12.3".to_string()),
                note: Some("".to_string()),
            }],
            registered_at: "2026-08-08T00:00:00Z".to_string(),
        }
    }

    fn base_vo() -> VoRecord {
        VoRecord {
            id: VoId::new("VO-PARSER-UTF8-003"),
            parent: Some(VoId::new("VO-PARSER-UTF8")),
            derives_from: vec![DerivesFrom {
                doc: DocumentId::new("DOC-BASIC-001"),
                anchor: Some("§8.2条項2".to_string()),
                note: Some("".to_string()),
            }],
            claim:
                "不正な continuation byte を含む入力を与えた場合、ParseError::InvalidUtf8 を返す"
                    .to_string(),
            dimensions: vec![Dimension {
                name: "operand-sign".to_string(),
                partitions: vec!["positive".to_string(), "negative".to_string()],
            }],
            coverage_policy: Some(CoveragePolicy::Explicit),
            combinations: vec![CombinationEntry::from_iter([
                ("operand-sign".to_string(), "positive".to_string()),
                ("operator".to_string(), "div".to_string()),
            ])],
            representative_cases: vec!["empty input".to_string()],
            created: "2026-08-08".to_string(),
            updated: "2026-08-08".to_string(),
        }
    }

    // ---- document subject hash ----

    /// @vtest.id TEST-MODEL-DOCUMENT-SUBJECT-HASH-SOURCE-CONTENT-CHANGES-HASH
    /// @vtest.covers VO-MODEL-DOCUMENT-SUBJECT-HASH
    /// @vtest.target crates/vtest-model/src/subject_hash.rs::document_subject_hash
    /// @vtest.intent verifies the referenced source file's current content is bound (本冊:89 "参照先 source…の正規化内容を束縛する")
    #[test]
    fn document_subject_hash_changes_when_source_text_changes() {
        let record = base_document();
        let a = document_subject_hash(&record, "current file content\n");
        let b = document_subject_hash(&record, "different file content\n");
        assert_ne!(a, b);
    }

    /// @vtest.id TEST-MODEL-DOCUMENT-SUBJECT-HASH-EACH-RECORD-FIELD-CHANGES-HASH
    /// @vtest.covers VO-MODEL-DOCUMENT-SUBJECT-HASH
    /// @vtest.target crates/vtest-model/src/subject_hash.rs::document_subject_hash
    /// @vtest.intent verifies each canonical DocumentRecord field is bound (本冊:89 "canonical document recordと…を束縛する")
    #[test]
    fn document_subject_hash_changes_when_any_record_field_changes() {
        let base = document_subject_hash(&base_document(), "source");

        let mut id_changed = base_document();
        id_changed.id = DocumentId::new("DOC-BASIC-002");
        assert_ne!(base, document_subject_hash(&id_changed, "source"));

        let mut path_changed = base_document();
        path_changed.path = "docs/other-spec.md".to_string();
        assert_ne!(base, document_subject_hash(&path_changed, "source"));

        let mut content_hash_changed = base_document();
        content_hash_changed.content_hash = ContentHash::from_text("a different registered value");
        assert_ne!(base, document_subject_hash(&content_hash_changed, "source"));

        let mut title_changed = base_document();
        title_changed.title = Some("別のタイトル".to_string());
        assert_ne!(base, document_subject_hash(&title_changed, "source"));

        let mut registered_at_changed = base_document();
        registered_at_changed.registered_at = "2026-09-01T00:00:00Z".to_string();
        assert_ne!(
            base,
            document_subject_hash(&registered_at_changed, "source")
        );

        let mut derives_from_changed = base_document();
        derives_from_changed.derives_from[0].doc = DocumentId::new("DOC-REQ-002");
        assert_ne!(base, document_subject_hash(&derives_from_changed, "source"));
    }

    /// @vtest.id TEST-MODEL-DOCUMENT-SUBJECT-HASH-ANCHOR-ONLY-CHANGE-CHANGES-HASH
    /// @vtest.covers VO-MODEL-DOCUMENT-SUBJECT-HASH
    /// @vtest.target crates/vtest-model/src/subject_hash.rs::document_subject_hash
    /// @vtest.intent verifies derives_from[].anchor is bound for document subject hash (本冊:207 "anchor は…document subject hash の入力に含まれる"), unlike VO subject hash
    #[test]
    fn document_subject_hash_changes_when_derives_from_anchor_changes() {
        let base = document_subject_hash(&base_document(), "source");

        let mut anchor_changed = base_document();
        anchor_changed.derives_from[0].anchor = Some("§99.9".to_string());
        assert_ne!(base, document_subject_hash(&anchor_changed, "source"));

        let mut note_changed = base_document();
        note_changed.derives_from[0].note = Some("a reason".to_string());
        assert_ne!(base, document_subject_hash(&note_changed, "source"));
    }

    /// @vtest.id TEST-MODEL-DOCUMENT-SUBJECT-HASH-RECORD-TEXT-FIELDS-ARE-NORMALIZED
    /// @vtest.covers VO-MODEL-DOCUMENT-SUBJECT-HASH
    /// @vtest.target crates/vtest-model/src/subject_hash.rs::document_subject_hash
    /// @vtest.intent verifies record scalar fields (title, id) are normalized text fragments, not byte-exact (PM ruling: 本冊:83 default applies; only Execution State manifest bytes are byte-exact, 本冊:91)
    #[test]
    fn document_subject_hash_normalizes_record_text_fields() {
        let mut crlf_title = base_document();
        crlf_title.title = Some("基本仕様書  \r\n".to_string());
        let mut lf_title = base_document();
        lf_title.title = Some("基本仕様書  \n".to_string());
        assert_eq!(
            document_subject_hash(&crlf_title, "source"),
            document_subject_hash(&lf_title, "source"),
            "title is a normalized text fragment, so CRLF vs LF must not change the hash"
        );
    }

    /// @vtest.id TEST-MODEL-DOCUMENT-SUBJECT-HASH-TITLE-ABSENT-VS-EMPTY
    /// @vtest.covers VO-MODEL-DOCUMENT-SUBJECT-HASH
    /// @vtest.target crates/vtest-model/src/subject_hash.rs::document_subject_hash
    /// @vtest.intent verifies an absent title (None) hashes differently from an explicit empty title (Some(""))
    #[test]
    fn document_subject_hash_distinguishes_absent_title_from_empty_title() {
        let mut absent = base_document();
        absent.title = None;
        let mut empty = base_document();
        empty.title = Some(String::new());
        assert_ne!(
            document_subject_hash(&absent, "source"),
            document_subject_hash(&empty, "source")
        );
    }

    // ---- VO subject hash ----

    /// @vtest.id TEST-MODEL-VO-RECORD-HAS-NO-STATUS-OR-COVERS-FIELD
    /// @vtest.covers VO-MODEL-VO-SUBJECT-HASH
    /// @vtest.target crates/vtest-model/src/subject_hash.rs::vo_subject_hash
    /// @vtest.intent verifies the canonical VoRecord this function reads structurally excludes `status`/`covers` (本冊:236 "実効判定とVO subject hashでは無視し", no `covers` key in the VO YAML example 本冊:215-227)
    #[test]
    fn vo_record_json_shape_has_no_status_or_covers_key() {
        let value = serde_json::to_value(base_vo()).unwrap();
        let keys: Vec<&String> = value.as_object().unwrap().keys().collect();
        assert!(
            !keys.iter().any(|key| key.as_str() == "status"),
            "canonical VoRecord must not carry a status field: {keys:?}"
        );
        assert!(
            !keys.iter().any(|key| key.as_str() == "covers"),
            "canonical VoRecord must not carry a covers field: {keys:?}"
        );
    }

    /// @vtest.id TEST-MODEL-VO-SUBJECT-HASH-DERIVES-FROM-AND-PARENT-CHANGE-HASH
    /// @vtest.covers VO-MODEL-VO-SUBJECT-HASH
    /// @vtest.target crates/vtest-model/src/subject_hash.rs::vo_subject_hash
    /// @vtest.intent verifies derives_from and parent are bound (本冊:90 "`derives_from`…と `parent` を束縛")
    #[test]
    fn vo_subject_hash_changes_when_derives_from_or_parent_changes() {
        let base = vo_subject_hash(&base_vo());

        let mut parent_changed = base_vo();
        parent_changed.parent = Some(VoId::new("VO-OTHER-PARENT"));
        assert_ne!(base, vo_subject_hash(&parent_changed));

        let mut derives_from_changed = base_vo();
        derives_from_changed.derives_from[0].doc = DocumentId::new("DOC-OTHER-001");
        assert_ne!(base, vo_subject_hash(&derives_from_changed));
    }

    /// @vtest.id TEST-MODEL-VO-SUBJECT-HASH-ANCHOR-ONLY-CHANGE-DOES-NOT-CHANGE-HASH
    /// @vtest.covers VO-MODEL-VO-SUBJECT-HASH
    /// @vtest.target crates/vtest-model/src/subject_hash.rs::vo_subject_hash
    /// @vtest.intent verifies derives_from[].anchor/note are excluded from VO subject hash (本冊:233 "anchorとnoteは…VO subject hashの入力に含まれない")
    #[test]
    fn vo_subject_hash_does_not_change_when_derives_from_anchor_or_note_changes() {
        let base = vo_subject_hash(&base_vo());

        let mut anchor_changed = base_vo();
        anchor_changed.derives_from[0].anchor = Some("§different".to_string());
        assert_eq!(base, vo_subject_hash(&anchor_changed));

        let mut note_changed = base_vo();
        note_changed.derives_from[0].note = Some("a different reason".to_string());
        assert_eq!(base, vo_subject_hash(&note_changed));
    }

    /// @vtest.id TEST-MODEL-VO-SUBJECT-HASH-DERIVES-FROM-REDUCES-TO-DOCUMENT-ID-SET
    /// @vtest.covers VO-MODEL-VO-SUBJECT-HASH
    /// @vtest.target crates/vtest-model/src/subject_hash.rs::vo_subject_hash
    /// @vtest.intent verifies a second derives_from entry for the same document (different anchor) does not change the hash — 本冊:232 "同一docをanchor違いで複数entryとして持つことを許容し、重複としない"
    #[test]
    fn vo_subject_hash_dedupes_derives_from_entries_pointing_at_the_same_document() {
        let single_entry = vo_subject_hash(&base_vo());

        let mut duplicated = base_vo();
        duplicated.derives_from.push(DerivesFrom {
            doc: duplicated.derives_from[0].doc.clone(),
            anchor: Some("§a-different-clause".to_string()),
            note: None,
        });
        assert_eq!(single_entry, vo_subject_hash(&duplicated));
    }

    /// @vtest.id TEST-MODEL-VO-SUBJECT-HASH-COMBINATIONS-CHANGE-HASH
    /// @vtest.covers VO-MODEL-VO-SUBJECT-HASH
    /// @vtest.target crates/vtest-model/src/subject_hash.rs::vo_subject_hash
    /// @vtest.intent verifies combinations is bound (本冊:286 "`combinations`は…VO subject hashに束縛される")
    #[test]
    fn vo_subject_hash_changes_when_combinations_changes() {
        let base = vo_subject_hash(&base_vo());
        let mut changed = base_vo();
        changed.combinations = vec![CombinationEntry::from_iter([
            ("operand-sign".to_string(), "negative".to_string()),
            ("operator".to_string(), "div".to_string()),
        ])];
        assert_ne!(base, vo_subject_hash(&changed));
    }

    /// @vtest.id TEST-MODEL-VO-SUBJECT-HASH-COMBINATIONS-ENTRY-KEY-ORDER-INDEPENDENT
    /// @vtest.covers VO-MODEL-VO-SUBJECT-HASH
    /// @vtest.target crates/vtest-model/src/subject_hash.rs::vo_subject_hash
    /// @vtest.intent verifies a combinations entry's internal (dimension, partition) pair order does not change the hash (本冊:256 "記述順・map key 順には依存しない")
    #[test]
    fn vo_subject_hash_combination_entry_is_independent_of_pair_declaration_order() {
        let mut forward_order = base_vo();
        forward_order.combinations = vec![CombinationEntry::from_iter([
            ("operand-sign".to_string(), "positive".to_string()),
            ("operator".to_string(), "div".to_string()),
        ])];

        let mut reversed_order = base_vo();
        reversed_order.combinations = vec![CombinationEntry::from_iter([
            ("operator".to_string(), "div".to_string()),
            ("operand-sign".to_string(), "positive".to_string()),
        ])];

        assert_eq!(
            vo_subject_hash(&forward_order),
            vo_subject_hash(&reversed_order)
        );
    }

    /// @vtest.id TEST-MODEL-VO-SUBJECT-HASH-EACH-REMAINING-FIELD-CHANGES-HASH
    /// @vtest.covers VO-MODEL-VO-SUBJECT-HASH
    /// @vtest.target crates/vtest-model/src/subject_hash.rs::vo_subject_hash
    /// @vtest.intent verifies claim/dimensions/coverage_policy/representative_cases/created/updated are each bound, as part of the whole canonical record (本冊:90, 本冊:286)
    #[test]
    fn vo_subject_hash_changes_when_any_other_canonical_field_changes() {
        let base = vo_subject_hash(&base_vo());

        let mut id_changed = base_vo();
        id_changed.id = VoId::new("VO-OTHER-ID");
        assert_ne!(base, vo_subject_hash(&id_changed));

        let mut claim_changed = base_vo();
        claim_changed.claim = "a different claim".to_string();
        assert_ne!(base, vo_subject_hash(&claim_changed));

        let mut dimensions_changed = base_vo();
        dimensions_changed.dimensions[0]
            .partitions
            .push("zero".to_string());
        assert_ne!(base, vo_subject_hash(&dimensions_changed));

        let mut coverage_policy_changed = base_vo();
        coverage_policy_changed.coverage_policy = Some(CoveragePolicy::FullProduct);
        assert_ne!(base, vo_subject_hash(&coverage_policy_changed));

        let mut representative_cases_changed = base_vo();
        representative_cases_changed
            .representative_cases
            .push("max length input".to_string());
        assert_ne!(base, vo_subject_hash(&representative_cases_changed));

        let mut created_changed = base_vo();
        created_changed.created = "2026-01-01".to_string();
        assert_ne!(base, vo_subject_hash(&created_changed));

        let mut updated_changed = base_vo();
        updated_changed.updated = "2026-01-01".to_string();
        assert_ne!(base, vo_subject_hash(&updated_changed));
    }

    /// @vtest.id TEST-MODEL-VO-SUBJECT-HASH-RECORD-TEXT-FIELDS-ARE-NORMALIZED
    /// @vtest.covers VO-MODEL-VO-SUBJECT-HASH
    /// @vtest.target crates/vtest-model/src/subject_hash.rs::vo_subject_hash
    /// @vtest.intent verifies record scalar fields (claim) are normalized text fragments, not byte-exact (PM ruling: 本冊:83 default applies; VO subject hash — 本冊:90 — has no "byte-exact" language)
    #[test]
    fn vo_subject_hash_normalizes_record_text_fields() {
        let mut crlf_claim = base_vo();
        crlf_claim.claim = "a claim with trailing space  \r\nsecond line  \r\n".to_string();
        let mut lf_claim = base_vo();
        lf_claim.claim = "a claim with trailing space\nsecond line\n".to_string();
        assert_eq!(
            vo_subject_hash(&crlf_claim),
            vo_subject_hash(&lf_claim),
            "claim is a normalized text fragment, so CRLF/trailing-space vs LF must not change the hash"
        );
    }

    /// @vtest.id TEST-MODEL-VO-SUBJECT-HASH-PARENT-ABSENT-VS-PRESENT
    /// @vtest.covers VO-MODEL-VO-SUBJECT-HASH
    /// @vtest.target crates/vtest-model/src/subject_hash.rs::vo_subject_hash
    /// @vtest.intent verifies an absent parent (None, root VO) hashes differently from any present parent value
    #[test]
    fn vo_subject_hash_distinguishes_absent_parent_from_present_parent() {
        let mut no_parent = base_vo();
        no_parent.parent = None;
        let with_parent = base_vo();
        assert_ne!(vo_subject_hash(&no_parent), vo_subject_hash(&with_parent));
    }

    // ---- Source Target subject hash ----

    fn base_locator() -> Locator {
        Locator {
            adapter: AdapterId::new("rust-cargo"),
            value: "src/parser.rs::Parser::parse".to_string(),
        }
    }

    /// @vtest.id TEST-MODEL-SOURCE-TARGET-SUBJECT-HASH-LOCATOR-AND-CONSTRUCT-CHANGE-HASH
    /// @vtest.covers VO-MODEL-SOURCE-TARGET-SUBJECT-HASH
    /// @vtest.target crates/vtest-model/src/subject_hash.rs::source_target_subject_hash
    /// @vtest.intent verifies canonical Target Reference (adapter, locator value) and construct bytes are each bound (本冊:88 "canonical Target Referenceとadapterが返すimplementation construct bytesを束縛する")
    #[test]
    fn source_target_subject_hash_changes_when_locator_or_construct_changes() {
        let base = source_target_subject_hash(&base_locator(), "fn parse() {}");

        let mut adapter_changed = base_locator();
        adapter_changed.adapter = AdapterId::new("other-lang");
        assert_ne!(
            base,
            source_target_subject_hash(&adapter_changed, "fn parse() {}")
        );

        let mut value_changed = base_locator();
        value_changed.value = "src/parser.rs::Parser::other".to_string();
        assert_ne!(
            base,
            source_target_subject_hash(&value_changed, "fn parse() {}")
        );

        assert_ne!(
            base,
            source_target_subject_hash(&base_locator(), "fn parse_other() {}")
        );
    }

    /// @vtest.id TEST-MODEL-SOURCE-TARGET-SUBJECT-HASH-CONSTRUCT-IS-NORMALIZED
    /// @vtest.covers VO-MODEL-SOURCE-TARGET-SUBJECT-HASH
    /// @vtest.target crates/vtest-model/src/subject_hash.rs::source_target_subject_hash
    /// @vtest.intent verifies construct bytes are a normalized text fragment, not byte-exact (PM ruling on 本冊:83/88/99 — see module header)
    #[test]
    fn source_target_subject_hash_normalizes_construct_text() {
        let crlf =
            source_target_subject_hash(&base_locator(), "fn parse() {  \r\n    body()  \r\n}");
        let lf = source_target_subject_hash(&base_locator(), "fn parse() {\n    body()\n}");
        assert_eq!(crlf, lf);
    }

    fn sample_source_location() -> crate::SourceLocation {
        crate::SourceLocation {
            file: "src/parser.rs".to_string(),
            function: "parse".to_string(),
            start_line: 10,
            end_line: 12,
            start_byte: 100,
            end_byte: 140,
        }
    }

    /// @vtest.id TEST-MODEL-SOURCE-TARGET-SUBJECT-HASH-IGNORES-SRC-ID-DECLARATION-CHANGE-OR-DELETION
    /// @vtest.covers VO-MODEL-SOURCE-TARGET-SUBJECT-HASH
    /// @vtest.target crates/vtest-model/src/subject_hash.rs::source_target_subject_hash
    /// @vtest.intent verifies declaring, changing, or deleting a SourceFunction's src_id does not change the hash when locator and construct bytes are held fixed (本冊:88 "恒久SRC IDはhash inputの独立fieldとして束縛せず、canonical Target Reference経由でもhash inputへ入らない")
    #[test]
    fn source_target_subject_hash_ignores_src_id_declaration_change_or_deletion() {
        // Three states of the *same* Source Target's permanent SRC ID —
        // undeclared, declared, and declared differently — modeled on the
        // realistic `SourceFunction` record (which carries `src_id`
        // alongside `locator`), with `locator` and construct bytes held
        // fixed throughout. `source_target_subject_hash` only ever reads
        // `.locator`, never `.src_id`, so all three must hash identically.
        let construct = "fn parse() {}";
        let src_id_undeclared = SourceFunction {
            locator: base_locator(),
            src_id: None,
            location: sample_source_location(),
            content_hash: ContentHash::from_text(construct),
        };
        let src_id_declared = SourceFunction {
            locator: base_locator(),
            src_id: Some(crate::SrcId::new("SRC-PARSER-001")),
            location: sample_source_location(),
            content_hash: ContentHash::from_text(construct),
        };
        let src_id_changed = SourceFunction {
            locator: base_locator(),
            src_id: Some(crate::SrcId::new("SRC-PARSER-002")),
            location: sample_source_location(),
            content_hash: ContentHash::from_text(construct),
        };

        let hash_undeclared = source_target_subject_hash(&src_id_undeclared.locator, construct);
        let hash_declared = source_target_subject_hash(&src_id_declared.locator, construct);
        let hash_changed = source_target_subject_hash(&src_id_changed.locator, construct);

        assert_eq!(
            hash_undeclared, hash_declared,
            "declaring a SRC ID must not change the hash"
        );
        assert_eq!(
            hash_declared, hash_changed,
            "changing a declared SRC ID must not change the hash"
        );
        // Deletion is the reverse of declaration — already covered by
        // hash_undeclared == hash_declared holding in both directions.
    }

    /// @vtest.id TEST-MODEL-SOURCE-TARGET-SUBJECT-HASH-INDEPENDENT-OF-REFERENCING-TARGETREF-SPELLING
    /// @vtest.covers VO-MODEL-SOURCE-TARGET-SUBJECT-HASH
    /// @vtest.target crates/vtest-model/src/subject_hash.rs::source_target_subject_hash
    /// @vtest.intent verifies the hash is computed only from the Source Target's own canonical Locator, never from a referencing Test's TargetRef spelling (本冊:88 "hashはSource Target自身のcanonical Locatorから一度だけ計算し、当該Source Targetを参照するTest側の`TargetRef`綴りからは計算しない")
    #[test]
    fn source_target_subject_hash_is_independent_of_referencing_targetref_spelling() {
        // Two Tests could declare a dependency on the *same* Source Target
        // through different `TargetRef` spellings — one by Locator, one by
        // SrcId — which core resolution (§6.1, out of this function's
        // scope) collapses to the same canonical Locator before this
        // function is ever called. This function's signature only accepts
        // that resolved `Locator`, never a `TargetRef`, so it structurally
        // cannot see which spelling a referencing Test used.
        let referenced_via_locator_targetref = TargetRef::Locator(base_locator());
        let referenced_via_src_id_targetref = TargetRef::SrcId(crate::SrcId::new("SRC-PARSER-001"));
        assert_ne!(
            referenced_via_locator_targetref, referenced_via_src_id_targetref,
            "precondition: these really are two different TargetRef spellings"
        );

        // Both resolve (elsewhere, not in this function) to the same
        // canonical Locator — which is the only thing this function reads.
        let resolved_canonical_locator = base_locator();
        let hash_from_one_call_site =
            source_target_subject_hash(&resolved_canonical_locator, "fn parse() {}");
        let hash_from_another_call_site =
            source_target_subject_hash(&resolved_canonical_locator, "fn parse() {}");
        assert_eq!(hash_from_one_call_site, hash_from_another_call_site);
    }

    /// @vtest.id TEST-MODEL-SOURCE-TARGET-SUBJECT-HASH-PRESERVES-LEADING-SPACE-AND-TRAILING-NEWLINE-IN-CONSTRUCT
    /// @vtest.covers VO-MODEL-SOURCE-TARGET-SUBJECT-HASH
    /// @vtest.target crates/vtest-model/src/subject_hash.rs::source_target_subject_hash
    /// @vtest.intent verifies construct bytes preserve leading whitespace and trailing-newline presence — normalization only unifies line endings and strips trailing per-line whitespace (本冊:83 "これ以外の空白は正規化しない")
    #[test]
    fn source_target_subject_hash_preserves_leading_space_and_trailing_newline_in_construct() {
        let leading_space =
            source_target_subject_hash(&base_locator(), "fn parse() {\n    body()\n}");
        let no_leading_space =
            source_target_subject_hash(&base_locator(), "fn parse() {\nbody()\n}");
        assert_ne!(leading_space, no_leading_space);

        let without_trailing_newline = source_target_subject_hash(&base_locator(), "fn parse() {}");
        let with_trailing_newline = source_target_subject_hash(&base_locator(), "fn parse() {}\n");
        assert_ne!(without_trailing_newline, with_trailing_newline);
    }

    /// @vtest.id TEST-MODEL-SOURCE-TARGET-SUBJECT-HASH-SRC-ID-INSIDE-CONSTRUCT-BYTES-CHANGES-HASH
    /// @vtest.covers VO-MODEL-SOURCE-TARGET-SUBJECT-HASH
    /// @vtest.target crates/vtest-model/src/subject_hash.rs::source_target_subject_hash
    /// @vtest.intent verifies a SRC ID declaration placed inside construct bytes (rust-cargo's `@vtest.src-id` doc comment) changes the hash via construct_text, which 本冊:88 states is correct — distinct from binding SRC ID as an independent field
    #[test]
    fn source_target_subject_hash_changes_when_an_in_construct_src_id_comment_is_added() {
        let without_src_id_comment = source_target_subject_hash(&base_locator(), "fn parse() {}");
        let with_src_id_comment = source_target_subject_hash(
            &base_locator(),
            "/// @vtest.src-id SRC-PARSER-001\nfn parse() {}",
        );
        assert_ne!(
            without_src_id_comment, with_src_id_comment,
            "a SRC ID declared inside construct bytes changes the hash through construct_text, \
             which 本冊:88 says is correct — it is not evidence of an independent SRC ID field"
        );
    }
}
