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
//! byte-exact ("subject固有規則でbyte-exactを要求しないテキストfragmentは
//! 改行をLFへ統一し、各行の末尾空白を除去する。これ以外の空白は正規化しな
//! い"). 本冊:91 ("byte-exact file bytes") is the only place §1.3 states a
//! byte-exact requirement, and it names the Execution State manifest's file
//! bytes; no document/VO/Source-Target field carries such a requirement.
//! Source Target construct bytes are explicitly subject to the 本冊:83
//! normalization: 本冊:99 "上記正規化後のsource bytesが変化した場合は安全
//! 側でSTALEにする" — "上記正規化" refers back to 本冊:83 (the word
//! 「正規化」 does not otherwise occur in 本冊:86-98). See
//! [`optional_text_fragment`]'s doc comment for the same citation chain.
//!
//! # Execution State subject hash is not implemented here
//!
//! §1.3 (本冊:91) requires Execution State subject hash to bind adapter ID,
//! snapshot schema ID / version, HEAD revision, runner kind and its canonical
//! invocation projection, toolchain identity, an adapter config projection,
//! and a full manifest of repository / local dependency inputs (each entry:
//! stable root identity, root-relative path, input kind, byte-exact file
//! bytes). None of these has a matching type anywhere in `vtest-model` or
//! `vtest-adapter-api` as of this module's writing (no `ExecutionState`,
//! `Snapshot`, `Manifest`/`ManifestEntry`, `ToolchainIdentity`, or `Revision`
//! type exists in either crate — confirmed by grep). Composing this hash
//! would require inventing the DTO shapes those inputs would arrive in,
//! which is not this module's place to do (this crate's own header: "この
//! crate は…filesystem access and derived indexes intentionally live in…
//! higher-level crates" — the same reasoning that keeps `document_subject_hash`
//! and `source_target_subject_hash` taking pre-read `&str`/bytes instead of
//! doing I/O applies more sharply here, where no receiving type exists at
//! all yet). Implementing `execution_state_subject_hash` is out of scope
//! until those upstream types exist; it is tracked as separate, follow-up
//! work.

use std::collections::BTreeSet;

use crate::{
    encode_nested_fields, normalize_hashed_text, AdapterId, CombinationEntry, ContentHash,
    Dimension, DocumentRecord, ExecutionDescriptor, FieldValue, Locator, SourceLocation,
    SubjectDomain, SubjectHashInput, TargetRef, TestRecord, TestSuite, VoRecord,
};

/// A scalar field whose declaration may be entirely absent (`None`,
/// encoded as [`FieldValue::Null`]) versus present-with-a-value (`Some`,
/// encoded as a normalized text fragment). Used for optional
/// identifier/metadata fields (`title`, `parent`, `anchor`, `note`).
///
/// Every scalar text field in this module goes through
/// [`FieldValue::text_fragment`], not [`FieldValue::exact_bytes`]: 本冊:83
/// makes normalization the default for any field a subject-specific rule
/// does not explicitly require byte-exact, and 本冊:91 ("byte-exact file
/// bytes") is the only place §1.3 states a byte-exact requirement, naming
/// the Execution State manifest's file bytes — document/VO/Source-Target
/// fields carry no such requirement. Source Target construct bytes fall
/// under the 本冊:83 default explicitly: 本冊:99 "上記正規化後のsource
/// bytesが変化した場合は安全側でSTALEにする" ("上記正規化" refers to
/// 本冊:83; 「正規化」 does not otherwise occur in 本冊:86-98).
fn optional_text_fragment(value: Option<&str>) -> FieldValue {
    match value {
        None => FieldValue::Null,
        Some(text) => FieldValue::text_fragment(text),
    }
}

/// Test subject hash (詳細設計 v0.1 §1.3, domain `vtest:test-subject:v1`,
/// 本冊:87): "adapter ID、Test ID、全canonical metadata、Source Locationの
/// adapter・project-relative path・opaque locator、ExecutionDescriptor、
/// および正規化したTest construct bytesを束縛する。byte range自体は前方の
/// 無関係な編集で変化するためhash inputにしない。"
///
/// # Inputs, matched against 本冊:87 one by one
///
/// - `adapter`: the discovering adapter's ID ("adapter ID"). This is a
///   separate parameter from `location.adapter` because 本冊:87 lists them
///   as two distinct bound items ("adapter ID、…、Source Locationのadapter
///   …"); in every adapter this crate knows about the two values coincide,
///   but the spec text binds both positions, so both are taken and both are
///   encoded under distinct field names (`adapter` and inside `location`).
/// - `metadata`: `id`/`covers`/`targets`/`intent`/`input`/`expect`/`kind`/
///   `cases`/`related` — exactly 本冊:87's "canonical metadataは `id` /
///   `covers` / `targets` / `intent` / `input` / `expect` / `kind` /
///   `cases` / `related` からなる" list, which is also this function's only
///   source for "Test ID" (`metadata.id` — the spec mentions "Test ID" and
///   metadata's `id` field as the same field, not two).
/// - `location`: only `adapter`/`path`/`locator` are read — `byte_range` is
///   deliberately never touched by this function (there is no parameter
///   position it could reach hash input through), which is what 本冊:87's
///   "byte range自体は…hash inputにしない" requires.
/// - `execution`: bound as a whole (adapter/project/suite/selector) — 本冊:87
///   names `ExecutionDescriptor` as a single bound input.
/// - `construct_text`: the normalized Test construct bytes ("正規化した
///   Test construct bytes") — a normalized [`FieldValue::text_fragment`],
///   not [`FieldValue::exact_bytes`], per the same 本冊:83-default reasoning
///   this module's header documents (本冊:91's manifest file bytes are the
///   only byte-exact requirement in §1.3).
///
/// `metadata.targets` binds the **declared** `TargetRef` values verbatim
/// (via [`encode_target_ref`]), never a resolved canonical Source Target
/// Locator — 本冊:87: "canonical metadataの`targets`は**宣言された**
/// `TargetRef`の正規化値を束縛し、解決後のcanonical Locatorへ置換しない".
/// This function has no parameter a *resolved* Source Target could reach —
/// only the caller's own `metadata.targets: Vec<TargetRef>` — so a spelling
/// change in a Test's declared target reference (e.g. rewriting a `Locator`
/// reference to a `SrcId` reference for the same underlying Source Target)
/// changes this hash, which is the effect 本冊:87 requires ("これにより
/// Testの参照方法の変更…はTest subject hashで捕捉される").
///
/// `covers`/`targets`/`related` are bound as sets (normalized-value
/// ascending, deduplicated — 本冊:85), `cases` as a declaration-ordered
/// list (本冊:85), matching [`FieldValue::Set`]/[`FieldValue::Ordered`]
/// respectively. `input`/`expect`/`kind` go through
/// [`optional_text_fragment`] so an absent declaration (`None`) hashes
/// differently from an explicit empty string (`Some(String::new())`) —
/// 本冊:87 "宣言の不在と空値の明示は異なる値としてencodeする".
pub fn test_subject_hash(
    adapter: &AdapterId,
    metadata: &TestRecord,
    location: &SourceLocation,
    execution: &ExecutionDescriptor,
    construct_text: &str,
) -> ContentHash {
    let covers: BTreeSet<Vec<u8>> = metadata
        .covers
        .iter()
        .map(|id| normalize_hashed_text(id.as_str()).into_bytes())
        .collect();
    let related: BTreeSet<Vec<u8>> = metadata
        .related
        .iter()
        .map(|id| normalize_hashed_text(id.as_str()).into_bytes())
        .collect();

    SubjectHashInput::new(SubjectDomain::TestSubject)
        .field("adapter", FieldValue::text_fragment(adapter.as_str()))
        .field("id", FieldValue::text_fragment(metadata.id.as_str()))
        .field("covers", FieldValue::Set(covers.into_iter().collect()))
        .field(
            "targets",
            FieldValue::Set(metadata.targets.iter().map(encode_target_ref).collect()),
        )
        .field("intent", FieldValue::text_fragment(&metadata.intent))
        .field("input", optional_text_fragment(metadata.input.as_deref()))
        .field("expect", optional_text_fragment(metadata.expect.as_deref()))
        .field("kind", optional_text_fragment(metadata.kind.as_deref()))
        .field(
            "cases",
            FieldValue::Ordered(
                metadata
                    .cases
                    .iter()
                    .map(|case| normalize_hashed_text(case).into_bytes())
                    .collect(),
            ),
        )
        .field("related", FieldValue::Set(related.into_iter().collect()))
        .field(
            "location",
            FieldValue::exact_bytes(encode_nested_fields([
                (
                    "adapter",
                    FieldValue::text_fragment(location.adapter.as_str()),
                ),
                ("path", FieldValue::text_fragment(location.path.as_str())),
                ("locator", FieldValue::text_fragment(&location.locator)),
            ])),
        )
        .field(
            "execution",
            FieldValue::exact_bytes(encode_nested_fields([
                (
                    "adapter",
                    FieldValue::text_fragment(execution.adapter.as_str()),
                ),
                (
                    "project",
                    optional_text_fragment(execution.project.as_deref()),
                ),
                ("suite", encode_optional_suite(execution.suite.as_ref())),
                ("selector", FieldValue::text_fragment(&execution.selector)),
            ])),
        )
        .field("construct", FieldValue::text_fragment(construct_text))
        .finish()
}

/// Encodes one declared `TargetRef` losslessly as a set element for
/// [`test_subject_hash`]'s `targets` field. Distinguishes the `Locator`
/// and `SrcId` variants by an explicit `kind` field so a `Locator` and a
/// `SrcId` that happen to carry the same text value never collide, and
/// binds `Locator`'s `adapter`/`value` or `SrcId`'s identifier verbatim —
/// this is the **declared** spelling, never resolved against a registry.
fn encode_target_ref(target: &TargetRef) -> Vec<u8> {
    match target {
        TargetRef::Locator(locator) => encode_nested_fields([
            ("kind", FieldValue::text_fragment("locator")),
            (
                "adapter",
                FieldValue::text_fragment(locator.adapter.as_str()),
            ),
            ("value", FieldValue::text_fragment(&locator.value)),
        ]),
        TargetRef::SrcId(src_id) => encode_nested_fields([
            ("kind", FieldValue::text_fragment("src-id")),
            ("id", FieldValue::text_fragment(src_id.as_str())),
        ]),
    }
}

/// Encodes `ExecutionDescriptor.suite` for [`test_subject_hash`]. `None`
/// (no suite declared) encodes as [`FieldValue::Null`]; `Some(suite)`
/// encodes as a nested `kind`/`name` record — `kind` is a required
/// (non-`Option`) `String`, so it is always a [`FieldValue::Scalar`] (even
/// when empty) whenever `suite` is `Some`, which is what keeps
/// `suite: None` from ever colliding with a present-but-degenerate `Some`.
fn encode_optional_suite(suite: Option<&TestSuite>) -> FieldValue {
    match suite {
        None => FieldValue::Null,
        Some(suite) => FieldValue::exact_bytes(encode_nested_fields([
            ("kind", FieldValue::text_fragment(&suite.kind)),
            ("name", optional_text_fragment(suite.name.as_deref())),
        ])),
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
/// bound as a normalized text fragment: 本冊:99 "上記正規化後のsource bytes
/// が変化した場合は安全側でSTALEにする" states construct bytes are subject
/// to the 本冊:83 normalization ("上記正規化"), and 本冊:91 is the only
/// place §1.3 requires byte-exactness (the Execution State manifest's file
/// bytes) — Source Target construct bytes carry no such requirement.
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
        AdapterId, CoveragePolicy, DerivesFrom, DocumentId, ProjectPath, SourceRange, SrcId,
        TestId, VoId,
    };

    // ---- Test subject hash fixtures ----

    fn base_test_record() -> TestRecord {
        TestRecord {
            id: TestId::new("TEST-PARSER-UTF8-003"),
            covers: vec![VoId::new("VO-PARSER-UTF8-003")],
            targets: vec![TargetRef::Locator(Locator {
                adapter: AdapterId::new("rust-cargo"),
                value: "src/parser.rs::Parser::parse".to_string(),
            })],
            intent: "The parser rejects an invalid continuation byte.".to_string(),
            input: Some("an invalid continuation byte".to_string()),
            expect: Some("ParseError::InvalidUtf8".to_string()),
            kind: Some("unit-error".to_string()),
            cases: vec!["empty input".to_string(), "max length input".to_string()],
            related: vec![TestId::new("TEST-PARSER-UTF8-004")],
        }
    }

    fn base_test_location() -> SourceLocation {
        SourceLocation {
            adapter: AdapterId::new("rust-cargo"),
            path: ProjectPath::new("src/parser.rs"),
            locator: "Parser::parse".to_string(),
            byte_range: SourceRange {
                start: 100,
                end: 240,
            },
        }
    }

    fn base_test_execution() -> ExecutionDescriptor {
        ExecutionDescriptor {
            adapter: AdapterId::new("rust-cargo"),
            project: Some("my-pkg".to_string()),
            suite: Some(TestSuite {
                kind: "lib".to_string(),
                name: None,
            }),
            selector: "parser::tests::rejects_invalid_utf8".to_string(),
        }
    }

    fn base_test_construct() -> &'static str {
        "fn rejects_invalid_utf8() { /* ... */ }"
    }

    fn base_test_hash() -> ContentHash {
        test_subject_hash(
            &AdapterId::new("rust-cargo"),
            &base_test_record(),
            &base_test_location(),
            &base_test_execution(),
            base_test_construct(),
        )
    }

    /// @vtest.id TEST-MODEL-TEST-SUBJECT-HASH-EACH-METADATA-FIELD-CHANGES-HASH
    /// @vtest.covers VO-MODEL-TEST-SUBJECT-HASH
    /// @vtest.target crates/vtest-model/src/subject_hash.rs::test_subject_hash
    /// @vtest.intent verifies each canonical metadata field (id/covers/targets/intent/input/expect/kind/cases/related) changes the hash when changed individually (本冊:87 "全canonical metadataを束縛する…canonical metadataは id/covers/targets/intent/input/expect/kind/cases/related からなる")
    #[test]
    fn test_subject_hash_changes_when_any_metadata_field_changes() {
        let base = base_test_hash();
        let hash_of = |record: TestRecord| {
            test_subject_hash(
                &AdapterId::new("rust-cargo"),
                &record,
                &base_test_location(),
                &base_test_execution(),
                base_test_construct(),
            )
        };

        let mut id_changed = base_test_record();
        id_changed.id = TestId::new("TEST-OTHER-ID");
        assert_ne!(base, hash_of(id_changed), "id");

        let mut covers_changed = base_test_record();
        covers_changed.covers.push(VoId::new("VO-OTHER"));
        assert_ne!(base, hash_of(covers_changed), "covers");

        let mut targets_changed = base_test_record();
        targets_changed.targets.push(TargetRef::Locator(Locator {
            adapter: AdapterId::new("rust-cargo"),
            value: "src/parser.rs::Parser::other".to_string(),
        }));
        assert_ne!(base, hash_of(targets_changed), "targets");

        let mut intent_changed = base_test_record();
        intent_changed.intent = "a different intent".to_string();
        assert_ne!(base, hash_of(intent_changed), "intent");

        let mut input_changed = base_test_record();
        input_changed.input = Some("a different input".to_string());
        assert_ne!(base, hash_of(input_changed), "input");

        let mut expect_changed = base_test_record();
        expect_changed.expect = Some("a different expectation".to_string());
        assert_ne!(base, hash_of(expect_changed), "expect");

        let mut kind_changed = base_test_record();
        kind_changed.kind = Some("unit-normal".to_string());
        assert_ne!(base, hash_of(kind_changed), "kind");

        let mut cases_changed = base_test_record();
        cases_changed.cases.push("a new case".to_string());
        assert_ne!(base, hash_of(cases_changed), "cases");

        let mut related_changed = base_test_record();
        related_changed.related.push(TestId::new("TEST-OTHER"));
        assert_ne!(base, hash_of(related_changed), "related");
    }

    /// @vtest.id TEST-MODEL-TEST-SUBJECT-HASH-BYTE-RANGE-DOES-NOT-CHANGE-HASH
    /// @vtest.covers VO-MODEL-TEST-SUBJECT-HASH
    /// @vtest.target crates/vtest-model/src/subject_hash.rs::test_subject_hash
    /// @vtest.intent verifies SourceLocation.byte_range is excluded from the hash input (本冊:87 "byte range自体は前方の無関係な編集で変化するためhash inputにしない")
    #[test]
    fn test_subject_hash_is_unchanged_when_only_byte_range_changes() {
        let base = base_test_hash();

        let mut moved = base_test_location();
        moved.byte_range = SourceRange {
            start: 9000,
            end: 9140,
        };
        let moved_hash = test_subject_hash(
            &AdapterId::new("rust-cargo"),
            &base_test_record(),
            &moved,
            &base_test_execution(),
            base_test_construct(),
        );

        assert_eq!(base, moved_hash);
    }

    /// @vtest.id TEST-MODEL-TEST-SUBJECT-HASH-TARGETS-BIND-DECLARED-SPELLING-NOT-RESOLVED-LOCATOR
    /// @vtest.covers VO-MODEL-TEST-SUBJECT-HASH
    /// @vtest.target crates/vtest-model/src/subject_hash.rs::test_subject_hash
    /// @vtest.intent verifies targets binds the declared TargetRef spelling — a Locator-vs-SrcId rewrite for the same underlying Source Target changes the hash, and it is never replaced with a resolved canonical Locator (本冊:87 "targetsは宣言されたTargetRefの正規化値を束縛し、解決後のcanonical Locatorへ置換しない")
    #[test]
    fn test_subject_hash_binds_declared_target_ref_spelling() {
        let via_locator = base_test_record();
        let base = test_subject_hash(
            &AdapterId::new("rust-cargo"),
            &via_locator,
            &base_test_location(),
            &base_test_execution(),
            base_test_construct(),
        );

        let mut via_src_id = base_test_record();
        via_src_id.targets = vec![TargetRef::SrcId(SrcId::new("SRC-PARSER-001"))];
        let src_id_hash = test_subject_hash(
            &AdapterId::new("rust-cargo"),
            &via_src_id,
            &base_test_location(),
            &base_test_execution(),
            base_test_construct(),
        );

        assert_ne!(
            base, src_id_hash,
            "rewriting a Test's declared target reference (Locator -> SrcId for the \
             same Source Target) must change the Test subject hash"
        );

        // Changing only the declared locator's spelling (not what it
        // resolves to) must also change the hash — the function has no
        // resolution step to consult in the first place.
        let mut different_locator_spelling = base_test_record();
        different_locator_spelling.targets = vec![TargetRef::Locator(Locator {
            adapter: AdapterId::new("rust-cargo"),
            value: "src/parser.rs::Parser::completely_different_symbol".to_string(),
        })];
        let different_spelling_hash = test_subject_hash(
            &AdapterId::new("rust-cargo"),
            &different_locator_spelling,
            &base_test_location(),
            &base_test_execution(),
            base_test_construct(),
        );
        assert_ne!(base, different_spelling_hash);
    }

    /// @vtest.id TEST-MODEL-TEST-SUBJECT-HASH-LOCATION-FIELDS-CHANGE-HASH
    /// @vtest.covers VO-MODEL-TEST-SUBJECT-HASH
    /// @vtest.target crates/vtest-model/src/subject_hash.rs::test_subject_hash
    /// @vtest.intent verifies SourceLocation's adapter/path/locator are each bound (本冊:87 "Source Locationのadapter・project-relative path・opaque locatorを束縛する")
    #[test]
    fn test_subject_hash_changes_when_location_adapter_path_or_locator_changes() {
        let base = base_test_hash();
        let hash_with_location = |location: SourceLocation| {
            test_subject_hash(
                &AdapterId::new("rust-cargo"),
                &base_test_record(),
                &location,
                &base_test_execution(),
                base_test_construct(),
            )
        };

        let mut adapter_changed = base_test_location();
        adapter_changed.adapter = AdapterId::new("other-lang");
        assert_ne!(
            base,
            hash_with_location(adapter_changed),
            "location.adapter"
        );

        let mut path_changed = base_test_location();
        path_changed.path = ProjectPath::new("src/other.rs");
        assert_ne!(base, hash_with_location(path_changed), "location.path");

        let mut locator_changed = base_test_location();
        locator_changed.locator = "Parser::other".to_string();
        assert_ne!(
            base,
            hash_with_location(locator_changed),
            "location.locator"
        );
    }

    /// @vtest.id TEST-MODEL-TEST-SUBJECT-HASH-EXECUTION-DESCRIPTOR-FIELDS-CHANGE-HASH
    /// @vtest.covers VO-MODEL-TEST-SUBJECT-HASH
    /// @vtest.target crates/vtest-model/src/subject_hash.rs::test_subject_hash
    /// @vtest.intent verifies each ExecutionDescriptor field (adapter/project/suite.kind/suite.name/selector) is bound (本冊:87 "ExecutionDescriptorを束縛する")
    #[test]
    fn test_subject_hash_changes_when_any_execution_descriptor_field_changes() {
        let base = base_test_hash();
        let hash_with_execution = |execution: ExecutionDescriptor| {
            test_subject_hash(
                &AdapterId::new("rust-cargo"),
                &base_test_record(),
                &base_test_location(),
                &execution,
                base_test_construct(),
            )
        };

        let mut adapter_changed = base_test_execution();
        adapter_changed.adapter = AdapterId::new("other-lang");
        assert_ne!(
            base,
            hash_with_execution(adapter_changed),
            "execution.adapter"
        );

        let mut project_changed = base_test_execution();
        project_changed.project = Some("other-pkg".to_string());
        assert_ne!(
            base,
            hash_with_execution(project_changed),
            "execution.project"
        );

        let mut suite_kind_changed = base_test_execution();
        suite_kind_changed.suite = Some(TestSuite {
            kind: "integration".to_string(),
            name: None,
        });
        assert_ne!(
            base,
            hash_with_execution(suite_kind_changed),
            "execution.suite.kind"
        );

        let mut suite_name_changed = base_test_execution();
        suite_name_changed.suite = Some(TestSuite {
            kind: "lib".to_string(),
            name: Some("some_integration_target".to_string()),
        });
        assert_ne!(
            base,
            hash_with_execution(suite_name_changed),
            "execution.suite.name"
        );

        let mut selector_changed = base_test_execution();
        selector_changed.selector = "parser::tests::other".to_string();
        assert_ne!(
            base,
            hash_with_execution(selector_changed),
            "execution.selector"
        );
    }

    /// @vtest.id TEST-MODEL-TEST-SUBJECT-HASH-SUITE-ABSENT-VS-PRESENT
    /// @vtest.covers VO-MODEL-TEST-SUBJECT-HASH
    /// @vtest.target crates/vtest-model/src/subject_hash.rs::test_subject_hash
    /// @vtest.intent verifies ExecutionDescriptor.suite being entirely absent (None) hashes differently from Some(suite) — a structural absent-vs-present distinction alongside 本冊:87's declaration-absence rule
    #[test]
    fn test_subject_hash_distinguishes_absent_suite_from_present_suite() {
        let mut no_suite = base_test_execution();
        no_suite.suite = None;
        let with_suite = base_test_execution();

        let no_suite_hash = test_subject_hash(
            &AdapterId::new("rust-cargo"),
            &base_test_record(),
            &base_test_location(),
            &no_suite,
            base_test_construct(),
        );
        let with_suite_hash = test_subject_hash(
            &AdapterId::new("rust-cargo"),
            &base_test_record(),
            &base_test_location(),
            &with_suite,
            base_test_construct(),
        );
        assert_ne!(no_suite_hash, with_suite_hash);
    }

    /// @vtest.id TEST-MODEL-TEST-SUBJECT-HASH-CONSTRUCT-TEXT-CHANGES-HASH
    /// @vtest.covers VO-MODEL-TEST-SUBJECT-HASH
    /// @vtest.target crates/vtest-model/src/subject_hash.rs::test_subject_hash
    /// @vtest.intent verifies normalized Test construct bytes are bound (本冊:87 "正規化したTest construct bytesを束縛する")
    #[test]
    fn test_subject_hash_changes_when_construct_text_changes() {
        let base = base_test_hash();
        let changed = test_subject_hash(
            &AdapterId::new("rust-cargo"),
            &base_test_record(),
            &base_test_location(),
            &base_test_execution(),
            "fn a_completely_different_function() {}",
        );
        assert_ne!(base, changed);
    }

    /// @vtest.id TEST-MODEL-TEST-SUBJECT-HASH-CONSTRUCT-TEXT-IS-NORMALIZED
    /// @vtest.covers VO-MODEL-TEST-SUBJECT-HASH
    /// @vtest.target crates/vtest-model/src/subject_hash.rs::test_subject_hash
    /// @vtest.intent verifies construct bytes are a normalized text fragment (CRLF/trailing-space insensitive), not byte-exact (本冊:83 default; 本冊:91's manifest file bytes are the only byte-exact requirement in §1.3)
    #[test]
    fn test_subject_hash_normalizes_construct_text() {
        let crlf = test_subject_hash(
            &AdapterId::new("rust-cargo"),
            &base_test_record(),
            &base_test_location(),
            &base_test_execution(),
            "fn f() {  \r\n    body()  \r\n}",
        );
        let lf = test_subject_hash(
            &AdapterId::new("rust-cargo"),
            &base_test_record(),
            &base_test_location(),
            &base_test_execution(),
            "fn f() {\n    body()\n}",
        );
        assert_eq!(crlf, lf);
    }

    /// @vtest.id TEST-MODEL-TEST-SUBJECT-HASH-ADAPTER-ID-CHANGES-HASH
    /// @vtest.covers VO-MODEL-TEST-SUBJECT-HASH
    /// @vtest.target crates/vtest-model/src/subject_hash.rs::test_subject_hash
    /// @vtest.intent verifies the top-level adapter ID parameter is bound (本冊:87 "adapter ID…を束縛する")
    #[test]
    fn test_subject_hash_changes_when_adapter_id_changes() {
        let base = base_test_hash();
        let changed = test_subject_hash(
            &AdapterId::new("other-lang"),
            &base_test_record(),
            &base_test_location(),
            &base_test_execution(),
            base_test_construct(),
        );
        assert_ne!(base, changed);
    }

    /// @vtest.id TEST-MODEL-TEST-SUBJECT-HASH-INPUT-EXPECT-KIND-ABSENT-VS-EMPTY
    /// @vtest.covers VO-MODEL-TEST-SUBJECT-HASH
    /// @vtest.target crates/vtest-model/src/subject_hash.rs::test_subject_hash
    /// @vtest.intent verifies input/expect/kind each distinguish an absent declaration (None) from an explicit empty value (Some("")) — 本冊:87 "宣言の不在と空値の明示は異なる値としてencodeする"
    #[test]
    fn test_subject_hash_distinguishes_absent_declaration_from_empty_value() {
        let hash_of = |record: TestRecord| {
            test_subject_hash(
                &AdapterId::new("rust-cargo"),
                &record,
                &base_test_location(),
                &base_test_execution(),
                base_test_construct(),
            )
        };

        let mut input_absent = base_test_record();
        input_absent.input = None;
        let mut input_empty = base_test_record();
        input_empty.input = Some(String::new());
        assert_ne!(hash_of(input_absent), hash_of(input_empty), "input");

        let mut expect_absent = base_test_record();
        expect_absent.expect = None;
        let mut expect_empty = base_test_record();
        expect_empty.expect = Some(String::new());
        assert_ne!(hash_of(expect_absent), hash_of(expect_empty), "expect");

        let mut kind_absent = base_test_record();
        kind_absent.kind = None;
        let mut kind_empty = base_test_record();
        kind_empty.kind = Some(String::new());
        assert_ne!(hash_of(kind_absent), hash_of(kind_empty), "kind");
    }

    /// @vtest.id TEST-MODEL-TEST-SUBJECT-HASH-CASES-IS-DECLARATION-ORDERED
    /// @vtest.covers VO-MODEL-TEST-SUBJECT-HASH
    /// @vtest.target crates/vtest-model/src/subject_hash.rs::test_subject_hash
    /// @vtest.intent verifies cases order is significant (declaration order), unlike covers/targets/related (本冊:85 "順序に意味があるcasesは宣言順とする")
    #[test]
    fn test_subject_hash_cases_order_is_significant() {
        let hash_of = |cases: Vec<String>| {
            let mut record = base_test_record();
            record.cases = cases;
            test_subject_hash(
                &AdapterId::new("rust-cargo"),
                &record,
                &base_test_location(),
                &base_test_execution(),
                base_test_construct(),
            )
        };

        let forward = hash_of(vec!["case-a".to_string(), "case-b".to_string()]);
        let reversed = hash_of(vec!["case-b".to_string(), "case-a".to_string()]);
        assert_ne!(forward, reversed);
    }

    /// @vtest.id TEST-MODEL-TEST-SUBJECT-HASH-COVERS-TARGETS-RELATED-ARE-ORDER-INDEPENDENT-SETS
    /// @vtest.covers VO-MODEL-TEST-SUBJECT-HASH
    /// @vtest.target crates/vtest-model/src/subject_hash.rs::test_subject_hash
    /// @vtest.intent verifies covers/targets/related are order-independent, deduplicated sets, unlike cases (本冊:85 "集合として扱うcovers・targets・relatedは正規化値の昇順")
    #[test]
    fn test_subject_hash_covers_targets_related_are_order_independent_sets() {
        let mut forward = base_test_record();
        forward.covers = vec![VoId::new("VO-A"), VoId::new("VO-B")];
        forward.related = vec![TestId::new("TEST-A"), TestId::new("TEST-B")];

        let mut reversed = base_test_record();
        reversed.covers = vec![VoId::new("VO-B"), VoId::new("VO-A")];
        reversed.related = vec![TestId::new("TEST-B"), TestId::new("TEST-A")];

        let hash_forward = test_subject_hash(
            &AdapterId::new("rust-cargo"),
            &forward,
            &base_test_location(),
            &base_test_execution(),
            base_test_construct(),
        );
        let hash_reversed = test_subject_hash(
            &AdapterId::new("rust-cargo"),
            &reversed,
            &base_test_location(),
            &base_test_execution(),
            base_test_construct(),
        );
        assert_eq!(hash_forward, hash_reversed);

        // Duplicate covers entries must not change the hash relative to the
        // deduplicated set.
        let mut duplicated = base_test_record();
        duplicated.covers = vec![
            VoId::new("VO-PARSER-UTF8-003"),
            VoId::new("VO-PARSER-UTF8-003"),
        ];
        let hash_duplicated = test_subject_hash(
            &AdapterId::new("rust-cargo"),
            &duplicated,
            &base_test_location(),
            &base_test_execution(),
            base_test_construct(),
        );
        assert_eq!(base_test_hash(), hash_duplicated);
    }

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
    /// @vtest.intent verifies record scalar fields (title, id) are normalized text fragments, not byte-exact (本冊:83 makes normalization the default; 本冊:91 is the only place §1.3 requires byte-exactness, naming the Execution State manifest's file bytes — document fields carry no such requirement)
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
    /// @vtest.intent verifies record scalar fields (claim) are normalized text fragments, not byte-exact (本冊:83 makes normalization the default; VO subject hash — 本冊:90 — states no byte-exact requirement, and 本冊:91 is the only place §1.3 requires byte-exactness, naming the Execution State manifest's file bytes)
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
    /// @vtest.intent verifies construct bytes are a normalized text fragment, not byte-exact (本冊:99 "上記正規化後のsource bytesが変化した場合は安全側でSTALEにする" subjects construct bytes to the 本冊:83 normalization; 本冊:91 is the only place §1.3 requires byte-exactness, naming the Execution State manifest's file bytes)
    #[test]
    fn source_target_subject_hash_normalizes_construct_text() {
        let crlf =
            source_target_subject_hash(&base_locator(), "fn parse() {  \r\n    body()  \r\n}");
        let lf = source_target_subject_hash(&base_locator(), "fn parse() {\n    body()\n}");
        assert_eq!(crlf, lf);
    }

    // No test exercises "changing/deleting a SourceFunction's src_id leaves
    // the hash unchanged" or "the hash is independent of a referencing
    // Test's TargetRef spelling" as a runtime assertion. Both were removed
    // (previously `source_target_subject_hash_ignores_src_id_declaration_
    // change_or_deletion` and
    // `source_target_subject_hash_is_independent_of_referencing_targetref_
    // spelling`): each called `source_target_subject_hash` with the *same*
    // `&Locator` and construct text every time and asserted `assert_eq!` on
    // the results, so they passed unconditionally regardless of what the
    // function's implementation did — `SrcId` and `TargetRef` values were
    // constructed in the test body but never reached the function under
    // test. That is not a gap this encoder could regress into: its
    // signature is `fn source_target_subject_hash(locator: &Locator,
    // construct_text: &str)` — there is no parameter position a `SrcId` or a
    // `TargetRef` could be passed through, so "ignores SRC ID" and
    // "ignores TargetRef spelling" are guaranteed by the type signature
    // itself, not by anything a unit test could falsify. The properties are
    // documented at the definition site instead (see
    // [`source_target_subject_hash`]'s doc comment, "The Source Target's
    // permanent SRC ID is **not** a parameter of this function..." and
    // "this function has no parameter a Test's `TargetRef` could even be
    // passed through").

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
