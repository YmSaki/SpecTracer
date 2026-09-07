//! §1.3 subject hash composition functions: document, VO, and Source Target.
//!
//! These bind the canonical inputs each subject hash requires (DES-070–076
//! for the shared encoding rules; DES-572/DES-575/DES-587 for the document
//! node rules; DES-093/DES-094/DES-096/DES-590/DES-122 for the VO rules;
//! DES-083/DES-085/DES-090 for the Source Target rules), using the encoding
//! primitives in `hash.rs` (`SubjectHashInput` / `FieldValue` /
//! `SubjectDomain` / `encode_nested_fields`). They only compute hash values
//! — no existing `ContentHash::from_text` call site is rewired to use them
//! here.
//!
//! Every scalar text field these functions bind uses
//! [`FieldValue::text_fragment`] (normalized), not
//! [`FieldValue::exact_bytes`] — DES-067 ("subject固有規則でbyte-exactを要
//! 求しないテキストfragmentは、改行をLFへ統一する。") and DES-068
//! ("subject固有規則でbyte-exactを要求しないテキストfragmentは、各行の末
//! 尾空白を除去する。") make normalization the default for any field a
//! subject-specific rule does not explicitly require byte-exact; DES-069
//! then confines normalization to exactly those two operations ("改行の統
//! 一と各行末尾空白の除去以外の空白は正規化しない。"). DES-098's manifest
//! entry rule ("…byte-exact file bytesからなる") is the only place §1.3
//! states a byte-exact requirement, and it names the Execution State
//! manifest's file bytes (DES-097); no document/VO/Source-Target field
//! carries such a requirement. DS-348 ("format変更を構文上の意味だけから
//! 同値とみなさず、正規化後のsource bytesが変化した場合は安全側でSTALE
//! にする。") speaks of Source Target construct bytes going through "正規
//! 化" without itself naming which rule that is — a derivation, not a
//! literal cross-reference: since DES-098 names the only byte-exact
//! requirement §1.3 states, and construct bytes are not that requirement's
//! subject, the "正規化" DS-348 refers to is DES-067–069 by elimination.
//! See [`optional_text_fragment`]'s doc comment for the same citation
//! chain.
//!
//! # Document node subject hash (root / sentence / section)
//!
//! DES-572 states one rule, domain `vtest:document-subject:v1`
//! (`SubjectDomain::DocumentSubject`), applied without distinguishing layer
//! (DES-575: "document subject hashは、層を区別せず、すべてのノードについて
//! §1.3 の同一の規則で計算する"): a sentence node's subject hash binds its
//! own `id` and `statement` (`description` excluded); a section node's
//! subject hash binds its children's subject hashes as two named ordered
//! sequences, `items` and `sections`, each encoded in declaration (array)
//! order (DES-587), with an absent or empty sequence bound as an explicit
//! empty list either way, never omitted.
//!
//! Neither case binds `derives_from`, `cites`, `title`, or `source` — DES-572
//! and DES-587 each enumerate exactly what that node kind's subject hash
//! binds, and DS-1601 / DS-1610 each gloss a sentence node's "規範内容"
//! (normative content — what the subject hash must capture) as
//! parenthetically `id` と `statement`. That parenthetical is a sufficient-
//! condition statement about sentence nodes, not an exhaustive definition
//! of "normative content" in general — DS-1601 and DS-1610's own
//! `description` field each say exactly this, in identical wording:
//! "「規範内容（`id` と `statement`）」は文ノードについての十分条件であ
//! り、規範内容の網羅的な定義ではない". DS-1612 makes the same
//! non-exhaustiveness point about a document node's normative content, in
//! its own (different) wording, without repeating the `id`/`statement`
//! parenthetical: its `description` reads "「規範内容」は文ノードの `id`
//! と `statement` に限られない". DS-1659 extends a *section*'s normative
//! content to include its children's declared order, which is exactly
//! `section_node_subject_hash`'s `items`/`sections` binding below.
//!
//! `root_node_subject_hash` is a derivation, not a literal citation: DES-572
//! names its two cases "文ノード" (sentence node) and "節ノード" (section
//! node) and does not mention `RootNode` in so many words. Schema
//! `$defs/rootItem` gives a root node the same leaf shape as
//! `$defs/derivedItem`'s sentence node — `id`, `statement`, optional
//! `description`, `source` — minus `derives_from`/`cites`, and DES-575
//! states the *same* §1.3 rule applies to every node "層を区別せず"
//! (regardless of layer). A root node is that leaf shape (it carries
//! `statement`, not `items`/`sections`), so this module folds it into
//! DES-572's sentence-node case. No specification.json node states this for
//! `RootNode` by name; see the PR report for this call-out.
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
//!
//! [`test_subject_hash`] below (this PR / #34) implements Test subject hash
//! per DES-077, binding an adapter ID and a Source Location built from
//! `adapter` / project-relative `path` / opaque `locator`, plus an
//! `ExecutionDescriptor` — see that function's own doc comment for the
//! field-by-field citation.

use std::collections::BTreeSet;

use crate::{
    encode_nested_fields, normalize_hashed_text, AdapterId, CombinationEntry, ContentHash,
    Dimension, DocumentId, ExecutionDescriptor, FieldValue, Locator, RootNode, SectionNode,
    SentenceNode, SourceLocation, SubjectDomain, SubjectHashInput, TargetRef, TestRecord,
    TestSuite, VoRecord,
};

/// A scalar field whose declaration may be entirely absent (`None`,
/// encoded as [`FieldValue::Null`]) versus present-with-a-value (`Some`,
/// encoded as a normalized text fragment). Used for optional
/// identifier/metadata fields (`title`, `parent`, `anchor`, `note`).
///
/// Every scalar text field in this module goes through
/// [`FieldValue::text_fragment`], not [`FieldValue::exact_bytes`]: DES-067
/// and DES-068 make normalization (LF unification, per-line trailing
/// whitespace removal) the default for any field a subject-specific rule
/// does not explicitly require byte-exact, and DES-098's "…byte-exact file
/// bytesからなる" is the only place §1.3 states a byte-exact requirement,
/// naming the Execution State manifest's file bytes (DES-097) —
/// document/VO/Source-Target fields carry no such requirement. DS-348
/// "正規化後のsource bytesが変化した場合は安全側でSTALEにする" names
/// construct bytes as going through "正規化" without itself saying which
/// rule that is — by elimination against DES-098's byte-exact requirement
/// (not construct bytes' subject), that normalization is DES-067/DES-068
/// (a derivation, not a literal cross-reference).
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

/// A leaf document node's subject hash: binds `id` and `statement`,
/// excludes `description` (DES-572). Shared by [`root_node_subject_hash`]
/// and [`sentence_node_subject_hash`] — see this module's doc comment for
/// why a root node is folded into DES-572's sentence-node case.
fn leaf_node_subject_hash(id: &DocumentId, statement: &str) -> ContentHash {
    SubjectHashInput::new(SubjectDomain::DocumentSubject)
        .field("id", FieldValue::text_fragment(id.as_str()))
        .field("statement", FieldValue::text_fragment(statement))
        .finish()
}

/// Root node subject hash (DES-572, DES-575). A derivation, not a literal
/// citation — see this module's doc comment for why `RootNode` is folded
/// into DES-572's sentence-node case.
pub fn root_node_subject_hash(node: &RootNode) -> ContentHash {
    leaf_node_subject_hash(&node.id, &node.statement)
}

/// Sentence node subject hash (DES-572): "文ノードでは当該ノードの `id` と
/// `statement` を…束縛し、`description` を束縛しない".
pub fn sentence_node_subject_hash(node: &SentenceNode) -> ContentHash {
    leaf_node_subject_hash(&node.id, &node.statement)
}

/// Section node subject hash (DES-572, DES-587): "節ノードでは子ノードの
/// subject hash を束縛し" — DES-587 elaborates this as two named ordered
/// sequences, `items` (sentence nodes) and `sections` (child section
/// nodes), each encoded in the array's declaration order ("当該配列の宣言
/// 順で encode する"). An absent (`None`) or explicitly empty
/// (`Some(vec![])`) sequence encodes identically — an explicit empty list
/// either way (DES-587 "空の列も空 list として明示し、省略しない") —
/// matching [`SectionNode`]'s own doc comment that this crate does not treat
/// the author's absent-vs-explicit-empty choice as meaningful.
///
/// This recurses into nested sections, so a section's subject hash is a
/// Merkle-style fold over its entire subtree: reordering, adding, or
/// removing any descendant sentence or section changes every ancestor
/// section's subject hash up to the root of that subtree (DS-1659). Neither
/// this function nor DES-572/DES-587 binds the section's own `id`, `title`,
/// `description`, `derives_from`, or `source` — only its children's hashes.
pub fn section_node_subject_hash(node: &SectionNode) -> ContentHash {
    let item_hashes: Vec<Vec<u8>> = node
        .items
        .as_deref()
        .unwrap_or(&[])
        .iter()
        .map(|item| {
            sentence_node_subject_hash(item)
                .as_str()
                .as_bytes()
                .to_vec()
        })
        .collect();
    let section_hashes: Vec<Vec<u8>> = node
        .sections
        .as_deref()
        .unwrap_or(&[])
        .iter()
        .map(|section| {
            section_node_subject_hash(section)
                .as_str()
                .as_bytes()
                .to_vec()
        })
        .collect();

    SubjectHashInput::new(SubjectDomain::DocumentSubject)
        .field("items", FieldValue::Ordered(item_hashes))
        .field("sections", FieldValue::Ordered(section_hashes))
        .finish()
}

/// VO subject hash (domain `vtest:record-subject:v1`, DES-093):
/// "readerが具体化したcanonical VO recordをfield規則に従って encodeする".
///
/// The canonical `vtest_model::VoRecord` this function reads has no `status`
/// or `covers` field — DES-094 ("VOの読取り互換field `status` を正典では
/// ないため含めない") and DES-096 ("`covers` の増減をTest側subjectで捕捉す
/// るため含めない") are therefore structurally satisfied by this function's
/// parameter type, not by an explicit runtime exclusion (there is nothing
/// to exclude): DS-405 confirms `status` is a read-compat-only field the
/// canonical writer never persists ("readerは読取り互換fieldとして
/// `status` を受理するが、実効判定とVO subject hashでは無視し"), and no
/// `VoRecord` field is named `covers`.
///
/// `derives_from` is reduced to the **set** of referenced upstream node ids,
/// dropping each entry's `anchor`/`note` — DES-590 ("VO subject hashは
/// `derives_from`（参照先ノード id 集合）と `parent` を束縛する") and
/// DS-1661 ("`anchor` と `note` はVO subject hashの入力に含まれない（VO
/// subject hash は `derives_from` の参照先ノード id 集合を束縛する）"), and
/// DS-401 ("同一 `doc` を `anchor` 違いで複数entryとして持つことを許容し、
/// 重複としない") — two entries differing only by `anchor` reference one
/// upstream node, so this reduction step deduplicates by that node's id.
/// `DerivesFrom::doc` names an upstream node — a section or a sentence node
/// (DS-1660: "VO レコードの `derives_from` entry の `doc` field の値は上流
/// ノード id（節ノードまたは文ノード）であり、上流文書のファイル名では
/// ない"), never a document *file*. This is narrower than a document node's
/// own subject hash ([`sentence_node_subject_hash`] /
/// [`section_node_subject_hash`]), which does not bind `derives_from` at all
/// — see this module's document-node doc comment.
///
/// Every other canonical field is bound as part of the whole record —
/// DES-122 confirms this explicitly for `combinations`: "`combinations` は
/// canonical VO recordの一部であり、VO subject hashに束縛される".
/// `combinations` entries are encoded via [`CombinationEntry::iter`], sorted
/// by (dimension name, partition value) — the same canonical, declaration-
/// order-independent form the type's own `Eq`/`Ord` use (DS-414:
/// "`combinations` の各entryはdimension名→partition値のmapとし" — a map, so
/// order-independent by construction), which also preserves a malformed
/// entry with a repeated dimension name losslessly rather than collapsing
/// it.
pub fn vo_subject_hash(record: &VoRecord) -> ContentHash {
    let referenced_node_ids: BTreeSet<Vec<u8>> = record
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
            FieldValue::Set(referenced_node_ids.into_iter().collect()),
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

/// Source Target hash (domain `vtest:target-subject:v1`, DES-083):
/// "canonical Target Referenceとadapterが返すimplementation construct
/// bytesを束縛する".
///
/// `locator` is the Source Target's own **canonical Target Reference** —
/// DES-085 requires this to always be a `TargetRef::Locator`, never a
/// `TargetRef::SrcId` ("canonical Target Referenceは常に `TargetRef::Locator`
/// …であり、`TargetRef::SrcId` をcanonical Target Referenceにしない").
/// Taking a plain [`Locator`] here (not a `TargetRef`) makes that
/// structural: there is no `TargetRef::SrcId` value this parameter could
/// hold. This also means the hash is computed only from the Source
/// Target's own canonical Locator, never from a referencing Test's
/// `TargetRef` spelling (DES-090 "Source Target hashはSource Target自身の
/// canonical Locatorから一度だけ計算し、当該Source Targetを参照するTest側
/// の `TargetRef` 綴りからは計算しない") — this function has no parameter
/// a Test's `TargetRef` could even be passed through.
///
/// `construct_text` is the adapter-returned implementation construct bytes,
/// decoded to text by the caller (`vtest-model` does no adapter I/O). It is
/// bound as a normalized text fragment: DS-348 "正規化後のsource bytesが
/// 変化した場合は安全側でSTALEにする" names construct bytes as going
/// through "正規化" without itself saying which rule that is; DES-098's
/// manifest-entry rule is the only place §1.3 requires byte-exactness (the
/// Execution State manifest's file bytes), and Source Target construct
/// bytes are not that requirement's subject, so by elimination the
/// "正規化" DS-348 refers to is DES-067/DES-068 (a derivation, not a
/// literal cross-reference).
///
/// The Source Target's permanent SRC ID is **not** a parameter of this
/// function and so cannot be bound as an independent field (DS-344 "恒久
/// SRC IDはhash inputの独立fieldとして束縛せず、canonical Target Reference
/// 経由でもhash inputへ入らない"). Declaring, changing, or deleting a SRC ID
/// therefore cannot change this hash by itself — except through
/// `construct_text`, for an adapter (`rust-cargo`'s `@vtest.src-id` doc
/// comment) that places the SRC ID declaration inside the construct bytes
/// themselves (DES-088); DES-089 states this construct-bytes-mediated
/// change is correct behavior, not evidence that SRC ID is an independent
/// hash field.
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
        AdapterId, CoveragePolicy, DerivesFrom, DocumentId, NodeSource, ProjectPath, SourceRange,
        SrcId, TestId, VoId,
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

    /// @vtest.id TEST-MODEL-TEST-SUBJECT-HASH-COMPUTES-WITH-EMPTY-TARGETS
    /// @vtest.covers VO-MODEL-TEST-SUBJECT-HASH
    /// @vtest.target crates/vtest-model/src/subject_hash.rs::test_subject_hash
    /// @vtest.intent verifies the hash computes for a Test declaring zero targets and differs from one declaring a target (ROOT-049/DS-1666: Source Target cardinality is N >= 0, not N >= 1 — `TestRecord.targets` is a plain `Vec`, not `Option<Vec>`, so an empty declaration and a present-but-empty declaration are the same value; there is no null/empty distinction to encode for this field, unlike the `Option` metadata fields covered elsewhere in this module)
    #[test]
    fn test_subject_hash_computes_with_empty_targets() {
        let mut no_targets = base_test_record();
        no_targets.targets = vec![];
        let empty_hash = test_subject_hash(
            &AdapterId::new("rust-cargo"),
            &no_targets,
            &base_test_location(),
            &base_test_execution(),
            base_test_construct(),
        );

        assert_ne!(
            base_test_hash(),
            empty_hash,
            "a Test declaring zero targets must hash differently from base_test_record's \
             single declared target, and must not panic or otherwise fail to compute"
        );
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
        let target_a = TargetRef::Locator(Locator {
            adapter: AdapterId::new("rust-cargo"),
            value: "src/parser.rs::Parser::a".to_string(),
        });
        let target_b = TargetRef::Locator(Locator {
            adapter: AdapterId::new("rust-cargo"),
            value: "src/parser.rs::Parser::b".to_string(),
        });

        let mut forward = base_test_record();
        forward.covers = vec![VoId::new("VO-A"), VoId::new("VO-B")];
        forward.targets = vec![target_a.clone(), target_b.clone()];
        forward.related = vec![TestId::new("TEST-A"), TestId::new("TEST-B")];

        let mut reversed = base_test_record();
        reversed.covers = vec![VoId::new("VO-B"), VoId::new("VO-A")];
        reversed.targets = vec![target_b, target_a];
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

        // Duplicate covers/targets/related entries must not change the hash
        // relative to the deduplicated set.
        let mut duplicated = base_test_record();
        duplicated.covers = vec![
            VoId::new("VO-PARSER-UTF8-003"),
            VoId::new("VO-PARSER-UTF8-003"),
        ];
        duplicated.targets = vec![
            TargetRef::Locator(Locator {
                adapter: AdapterId::new("rust-cargo"),
                value: "src/parser.rs::Parser::parse".to_string(),
            }),
            TargetRef::Locator(Locator {
                adapter: AdapterId::new("rust-cargo"),
                value: "src/parser.rs::Parser::parse".to_string(),
            }),
        ];
        duplicated.related = vec![
            TestId::new("TEST-PARSER-UTF8-004"),
            TestId::new("TEST-PARSER-UTF8-004"),
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

    fn sample_source() -> NodeSource {
        NodeSource {
            doc: "docs/spec.md".to_string(),
            heading: "1".to_string(),
            lines: [1, 1],
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

    // ---- document node subject hash (root / sentence / section) ----

    fn sample_sentence(id: &str, statement: &str) -> SentenceNode {
        SentenceNode {
            id: DocumentId::new(id),
            statement: statement.to_string(),
            description: None,
            derives_from: vec![],
            cites: None,
            source: sample_source(),
        }
    }

    fn sample_root(id: &str, statement: &str) -> RootNode {
        RootNode {
            id: DocumentId::new(id),
            statement: statement.to_string(),
            description: None,
            source: sample_source(),
        }
    }

    fn sample_section(
        id: &str,
        items: Vec<SentenceNode>,
        sections: Vec<SectionNode>,
    ) -> SectionNode {
        SectionNode {
            id: DocumentId::new(id),
            title: format!("title for {id}"),
            description: None,
            source: sample_source(),
            derives_from: None,
            sections: if sections.is_empty() {
                None
            } else {
                Some(sections)
            },
            items: if items.is_empty() { None } else { Some(items) },
        }
    }

    /// @vtest.id TEST-MODEL-DOCUMENT-NODE-SUBJECT-HASH-IS-DETERMINISTIC
    /// @vtest.covers VO-MODEL-DOCUMENT-NODE-SUBJECT-HASH
    /// @vtest.target crates/vtest-model/src/subject_hash.rs::sentence_node_subject_hash
    /// @vtest.intent verifies computing the same node's subject hash twice yields the same value (DES-572)
    #[test]
    fn document_node_subject_hash_is_deterministic() {
        let node = sample_sentence("REQ-001", "a requirement");
        assert_eq!(
            sentence_node_subject_hash(&node),
            sentence_node_subject_hash(&node)
        );

        let section = sample_section("REQ-S001", vec![sample_sentence("REQ-002", "x")], vec![]);
        assert_eq!(
            section_node_subject_hash(&section),
            section_node_subject_hash(&section)
        );
    }

    /// @vtest.id TEST-MODEL-SENTENCE-NODE-SUBJECT-HASH-BINDS-ID-AND-STATEMENT
    /// @vtest.covers VO-MODEL-DOCUMENT-NODE-SUBJECT-HASH
    /// @vtest.target crates/vtest-model/src/subject_hash.rs::sentence_node_subject_hash
    /// @vtest.intent verifies id and statement each change the hash (DES-572 "文ノードでは当該ノードの `id` と `statement` を…束縛し")
    #[test]
    fn sentence_node_subject_hash_changes_when_id_or_statement_changes() {
        let base = sentence_node_subject_hash(&sample_sentence("REQ-001", "a requirement"));

        let id_changed = sample_sentence("REQ-002", "a requirement");
        assert_ne!(base, sentence_node_subject_hash(&id_changed));

        let statement_changed = sample_sentence("REQ-001", "a different requirement");
        assert_ne!(base, sentence_node_subject_hash(&statement_changed));
    }

    /// @vtest.id TEST-MODEL-SENTENCE-NODE-SUBJECT-HASH-EXCLUDES-DESCRIPTION
    /// @vtest.covers VO-MODEL-DOCUMENT-NODE-SUBJECT-HASH
    /// @vtest.target crates/vtest-model/src/subject_hash.rs::sentence_node_subject_hash
    /// @vtest.intent verifies description does not change the hash (DES-572 "`description` を束縛しない"; DS-1599/DS-1609)
    #[test]
    fn sentence_node_subject_hash_ignores_description() {
        let mut without_description = sample_sentence("REQ-001", "a requirement");
        without_description.description = None;
        let mut with_description = sample_sentence("REQ-001", "a requirement");
        with_description.description = Some("an explanatory note".to_string());
        assert_eq!(
            sentence_node_subject_hash(&without_description),
            sentence_node_subject_hash(&with_description)
        );
    }

    /// @vtest.id TEST-MODEL-SENTENCE-NODE-SUBJECT-HASH-EXCLUDES-DERIVES-FROM-AND-CITES
    /// @vtest.covers VO-MODEL-DOCUMENT-NODE-SUBJECT-HASH
    /// @vtest.target crates/vtest-model/src/subject_hash.rs::sentence_node_subject_hash
    /// @vtest.intent verifies derives_from/cites do not change the hash — DES-572 enumerates only id and statement as bound for a sentence node
    #[test]
    fn sentence_node_subject_hash_ignores_derives_from_and_cites() {
        let base = sentence_node_subject_hash(&sample_sentence("REQ-001", "a requirement"));

        let mut derives_from_changed = sample_sentence("REQ-001", "a requirement");
        derives_from_changed.derives_from = vec![DocumentId::new("ROOT-001")];
        assert_eq!(base, sentence_node_subject_hash(&derives_from_changed));

        let mut cites_changed = sample_sentence("REQ-001", "a requirement");
        cites_changed.cites = Some(vec!["基本仕様 §3.2".to_string()]);
        assert_eq!(base, sentence_node_subject_hash(&cites_changed));
    }

    /// @vtest.id TEST-MODEL-ROOT-NODE-SUBJECT-HASH-MATCHES-SENTENCE-NODE-RULE
    /// @vtest.covers VO-MODEL-DOCUMENT-NODE-SUBJECT-HASH
    /// @vtest.target crates/vtest-model/src/subject_hash.rs::root_node_subject_hash
    /// @vtest.intent verifies a root node's subject hash follows the same id+statement rule as a sentence node (derivation — see this module's doc comment; DES-575 "層を区別せず…同一の規則で計算する"), and that id/statement each change it while description does not
    #[test]
    fn root_node_subject_hash_follows_the_sentence_node_rule() {
        let root = sample_root("ROOT-001", "Frozen ruling.");
        let sentence_with_same_id_and_statement = sample_sentence("ROOT-001", "Frozen ruling.");
        assert_eq!(
            root_node_subject_hash(&root),
            sentence_node_subject_hash(&sentence_with_same_id_and_statement),
            "DES-575 states one rule applied regardless of layer; a root node is DES-572's leaf shape"
        );

        let base = root_node_subject_hash(&sample_root("ROOT-001", "Frozen ruling."));
        assert_ne!(
            base,
            root_node_subject_hash(&sample_root("ROOT-002", "Frozen ruling."))
        );
        assert_ne!(
            base,
            root_node_subject_hash(&sample_root("ROOT-001", "A different ruling."))
        );

        let mut with_description = sample_root("ROOT-001", "Frozen ruling.");
        with_description.description = Some("context".to_string());
        assert_eq!(base, root_node_subject_hash(&with_description));
    }

    /// @vtest.id TEST-MODEL-SECTION-NODE-SUBJECT-HASH-CHANGES-WHEN-CHILDREN-REORDERED
    /// @vtest.covers VO-MODEL-DOCUMENT-NODE-SUBJECT-HASH
    /// @vtest.target crates/vtest-model/src/subject_hash.rs::section_node_subject_hash
    /// @vtest.intent verifies reordering items, and reordering sections, each change the parent section's hash even though the child set is unchanged (DS-1659, DES-587 "当該配列の宣言順で encode する")
    #[test]
    fn section_node_subject_hash_changes_when_children_are_reordered() {
        let forward_items = sample_section(
            "REQ-S001",
            vec![
                sample_sentence("REQ-001", "first"),
                sample_sentence("REQ-002", "second"),
            ],
            vec![],
        );
        let reversed_items = sample_section(
            "REQ-S001",
            vec![
                sample_sentence("REQ-002", "second"),
                sample_sentence("REQ-001", "first"),
            ],
            vec![],
        );
        assert_ne!(
            section_node_subject_hash(&forward_items),
            section_node_subject_hash(&reversed_items)
        );

        let forward_sections = sample_section(
            "REQ-S002",
            vec![],
            vec![
                sample_section("REQ-S003", vec![sample_sentence("REQ-003", "a")], vec![]),
                sample_section("REQ-S004", vec![sample_sentence("REQ-004", "b")], vec![]),
            ],
        );
        let reversed_sections = sample_section(
            "REQ-S002",
            vec![],
            vec![
                sample_section("REQ-S004", vec![sample_sentence("REQ-004", "b")], vec![]),
                sample_section("REQ-S003", vec![sample_sentence("REQ-003", "a")], vec![]),
            ],
        );
        assert_ne!(
            section_node_subject_hash(&forward_sections),
            section_node_subject_hash(&reversed_sections)
        );
    }

    /// @vtest.id TEST-MODEL-SUBJECT-HASH-INPUT-SAME-VALUE-DIFFERS-BY-FIELD-NAME-ALONE
    /// @vtest.covers VO-MODEL-DOCUMENT-NODE-SUBJECT-HASH
    /// @vtest.target crates/vtest-model/src/hash.rs::SubjectHashInput
    /// @vtest.intent verifies, at the encoder level, that binding the identical `Ordered([X])` value under field name `items` (with no `sections` field present at all) produces a different hash than binding that same value under field name `sections` (with no `items` field present). Field name is the only variable that differs between the two encoder calls — the payload occupies the same relative position (the sole field) in both encoded buffers — so this isolates the field-name binding as the sole variable, which the two-field construction in the next test does not (there the payload's position in the buffer is confounded with which field carries it). This is what DES-587's "`items`（文ノードの列）と `sections`（下位節ノードの列）の 2 つの名前付き順序列として束縛し" actually rests on.
    #[test]
    fn subject_hash_input_same_value_differs_by_field_name_alone() {
        let same_child_hash_bytes = b"same-child-subject-hash-bytes".to_vec();
        let as_items = SubjectHashInput::new(SubjectDomain::DocumentSubject)
            .field(
                "items",
                FieldValue::Ordered(vec![same_child_hash_bytes.clone()]),
            )
            .finish();
        let as_sections = SubjectHashInput::new(SubjectDomain::DocumentSubject)
            .field("sections", FieldValue::Ordered(vec![same_child_hash_bytes]))
            .finish();
        assert_ne!(as_items, as_sections);
    }

    /// @vtest.id TEST-MODEL-SUBJECT-HASH-INPUT-ITEMS-FIELD-DIFFERS-FROM-SECTIONS-FIELD
    /// @vtest.covers VO-MODEL-DOCUMENT-NODE-SUBJECT-HASH
    /// @vtest.target crates/vtest-model/src/hash.rs::SubjectHashInput
    /// @vtest.intent verifies, at the encoder level, that placing the same child-hash byte string under `items` (with `sections` bound empty) hashes differently than placing it under `sections` (with `items` bound empty) — the two-field shape `section_node_subject_hash` actually produces. This does NOT isolate field name as the sole variable: the payload also occupies a different position in the encoded buffer in each case (first field vs. second field), so a byte-string comparison alone cannot attribute the difference to the field name rather than to that position. The preceding test isolates field name directly by comparing the identical single-field input under each name with no other field present in either input.
    #[test]
    fn subject_hash_input_items_field_differs_from_sections_field_for_the_same_content() {
        let same_child_hash_bytes = b"same-child-subject-hash-bytes".to_vec();
        let as_items = SubjectHashInput::new(SubjectDomain::DocumentSubject)
            .field(
                "items",
                FieldValue::Ordered(vec![same_child_hash_bytes.clone()]),
            )
            .field("sections", FieldValue::Ordered(vec![]))
            .finish();
        let as_sections = SubjectHashInput::new(SubjectDomain::DocumentSubject)
            .field("items", FieldValue::Ordered(vec![]))
            .field("sections", FieldValue::Ordered(vec![same_child_hash_bytes]))
            .finish();
        assert_ne!(as_items, as_sections);
    }

    /// @vtest.id TEST-MODEL-SECTION-NODE-SUBJECT-HASH-DIFFERS-WHEN-A-CHILD-MOVES-TO-A-NESTED-SECTION
    /// @vtest.covers VO-MODEL-DOCUMENT-NODE-SUBJECT-HASH
    /// @vtest.target crates/vtest-model/src/subject_hash.rs::section_node_subject_hash
    /// @vtest.intent verifies a real structural edit — moving a sentence out of `items` and into a new child section's own `items` — changes the parent's hash. This is a meaningful regression test on the typed API, but (unlike the encoder-level test above) it does not isolate "which named field" as the sole variable: the wrapping child section also adds one level of nesting, so a hypothetical implementation that merged items+sections into one unordered list could also fail this comparison for the nesting-depth reason alone, not necessarily by distinguishing the two field names.
    #[test]
    fn section_node_subject_hash_changes_when_a_child_moves_into_a_nested_section() {
        let leaf_sentence = sample_sentence("REQ-001", "shared content");
        let as_item = sample_section("REQ-S001", vec![leaf_sentence.clone()], vec![]);
        let as_section = sample_section(
            "REQ-S001",
            vec![],
            vec![sample_section("REQ-S002", vec![leaf_sentence], vec![])],
        );
        assert_ne!(
            section_node_subject_hash(&as_item),
            section_node_subject_hash(&as_section)
        );
    }

    /// @vtest.id TEST-MODEL-SECTION-NODE-SUBJECT-HASH-ABSENT-EQUALS-EXPLICIT-EMPTY
    /// @vtest.covers VO-MODEL-DOCUMENT-NODE-SUBJECT-HASH
    /// @vtest.target crates/vtest-model/src/subject_hash.rs::section_node_subject_hash
    /// @vtest.intent verifies items:None and items:Some(vec![]) (likewise sections) hash identically — DES-587 "空の列も空 list として明示し、省略しない"
    #[test]
    fn section_node_subject_hash_treats_absent_and_explicit_empty_the_same() {
        let mut none_children = sample_section("REQ-S001", vec![], vec![]);
        none_children.items = None;
        none_children.sections = None;

        let mut explicit_empty_children = sample_section("REQ-S001", vec![], vec![]);
        explicit_empty_children.items = Some(vec![]);
        explicit_empty_children.sections = Some(vec![]);

        assert_eq!(
            section_node_subject_hash(&none_children),
            section_node_subject_hash(&explicit_empty_children)
        );
    }

    /// @vtest.id TEST-MODEL-SECTION-NODE-SUBJECT-HASH-IGNORES-OWN-IDENTITY-FIELDS
    /// @vtest.covers VO-MODEL-DOCUMENT-NODE-SUBJECT-HASH
    /// @vtest.target crates/vtest-model/src/subject_hash.rs::section_node_subject_hash
    /// @vtest.intent verifies a section's own id/title/description/derives_from do not change its hash — DES-572 binds only the children's subject hashes for a section node
    #[test]
    fn section_node_subject_hash_ignores_its_own_identity_fields() {
        let base = sample_section("REQ-S001", vec![sample_sentence("REQ-001", "x")], vec![]);
        let base_hash = section_node_subject_hash(&base);

        let mut id_changed = base.clone();
        id_changed.id = DocumentId::new("REQ-S999");
        assert_eq!(base_hash, section_node_subject_hash(&id_changed));

        let mut title_changed = base.clone();
        title_changed.title = "a different title".to_string();
        assert_eq!(base_hash, section_node_subject_hash(&title_changed));

        let mut description_changed = base.clone();
        description_changed.description = Some("context".to_string());
        assert_eq!(base_hash, section_node_subject_hash(&description_changed));

        let mut derives_from_changed = base;
        derives_from_changed.derives_from = Some(vec![DocumentId::new("ROOT-001")]);
        assert_eq!(base_hash, section_node_subject_hash(&derives_from_changed));
    }

    /// @vtest.id TEST-MODEL-SECTION-NODE-SUBJECT-HASH-PROPAGATES-FROM-NESTED-DESCENDANT
    /// @vtest.covers VO-MODEL-DOCUMENT-NODE-SUBJECT-HASH
    /// @vtest.target crates/vtest-model/src/subject_hash.rs::section_node_subject_hash
    /// @vtest.intent verifies changing a deeply nested sentence's statement changes every ancestor section's hash (DS-1659's Merkle-style propagation)
    #[test]
    fn section_node_subject_hash_propagates_changes_from_nested_descendants() {
        let build = |leaf_statement: &str| {
            sample_section(
                "REQ-S001",
                vec![],
                vec![sample_section(
                    "REQ-S002",
                    vec![sample_sentence("REQ-001", leaf_statement)],
                    vec![],
                )],
            )
        };
        assert_ne!(
            section_node_subject_hash(&build("original")),
            section_node_subject_hash(&build("changed"))
        );
    }

    /// Real-bundle sweep: only runs when `VTEST_CANONICAL_BUNDLE` names the
    /// canonical `specification.json`. Not run by default (see
    /// `document.rs`'s analogous `#[ignore]`d round-trip test for the same
    /// convention).
    #[test]
    #[ignore = "requires VTEST_CANONICAL_BUNDLE env var pointing at the canonical specification.json"]
    fn canonical_bundle_document_node_subject_hashes_compute_without_panicking() {
        let path = std::env::var("VTEST_CANONICAL_BUNDLE")
            .expect("set VTEST_CANONICAL_BUNDLE to the canonical specification.json path");
        let text = std::fs::read_to_string(&path).expect("failed to read canonical bundle");
        let bundle: crate::DocumentFile =
            serde_json::from_str(&text).expect("bundle does not parse as DocumentFile");

        let mut hashes: Vec<String> = Vec::new();

        for root in &bundle.root {
            hashes.push(root_node_subject_hash(root).as_str().to_owned());
        }
        for sentence in &bundle.request {
            hashes.push(sentence_node_subject_hash(sentence).as_str().to_owned());
        }

        fn walk_sections(sections: &[SectionNode], hashes: &mut Vec<String>) {
            for section in sections {
                hashes.push(section_node_subject_hash(section).as_str().to_owned());
                if let Some(items) = &section.items {
                    for item in items {
                        hashes.push(sentence_node_subject_hash(item).as_str().to_owned());
                    }
                }
                if let Some(nested) = &section.sections {
                    walk_sections(nested, hashes);
                }
            }
        }
        walk_sections(&bundle.require, &mut hashes);
        walk_sections(&bundle.spec, &mut hashes);
        walk_sections(&bundle.detailed_spec, &mut hashes);
        walk_sections(&bundle.basic_design, &mut hashes);
        walk_sections(&bundle.design, &mut hashes);

        let total = hashes.len();
        let distinct: std::collections::HashSet<&String> = hashes.iter().collect();
        eprintln!(
            "canonical_bundle_document_node_subject_hashes_compute_without_panicking: computed \
             {total} node subject hashes with 0 panics; {} distinct values ({} colliding). Not \
             asserted: the canonical bundle moves forward on its own branch, and a section-level \
             collision is not by itself a defect — DES-572 binds a section's subject hash purely \
             to its children's hashes, never its own id/title, so two structurally identical \
             subtrees (most commonly two sections with neither items nor sections) hash \
             identically by design.",
            distinct.len(),
            total - distinct.len(),
        );
    }

    // ---- VO subject hash ----

    /// @vtest.id TEST-MODEL-VO-RECORD-HAS-NO-STATUS-OR-COVERS-FIELD
    /// @vtest.covers VO-MODEL-VO-SUBJECT-HASH
    /// @vtest.target crates/vtest-model/src/subject_hash.rs::vo_subject_hash
    /// @vtest.intent verifies the canonical VoRecord this function reads structurally excludes `status`/`covers` (DES-094 "VOの読取り互換field `status` を正典ではないため含めない"; DES-096 "`covers` の増減をTest側subjectで捕捉するため含めない"; DS-405 confirms `status` is a read-compat-only field the canonical writer never persists)
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
    /// @vtest.intent verifies derives_from and parent are bound (DES-590 "VO subject hashは `derives_from`（参照先ノード id 集合）と `parent` を束縛する")
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
    /// @vtest.intent verifies derives_from[].anchor/note are excluded from VO subject hash (DS-1661 "`anchor` と `note` はVO subject hashの入力に含まれない")
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
    /// @vtest.intent verifies a second derives_from entry for the same document (different anchor) does not change the hash — DS-401 "同一 `doc` を `anchor` 違いで複数entryとして持つことを許容し、重複としない"
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
    /// @vtest.intent verifies combinations is bound (DES-122 "`combinations` はcanonical VO recordの一部であり、VO subject hashに束縛される")
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
    /// @vtest.intent verifies a combinations entry's internal (dimension, partition) pair order does not change the hash (DES-073 "hash inputにおいて、mapはkey昇順でencodeする"; DS-414 confirms a `combinations` entry is itself a dimension-name→partition-value map)
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
    /// @vtest.intent verifies claim/dimensions/coverage_policy/representative_cases/created/updated are each bound, as part of the whole canonical record (DES-093 "readerが具体化したcanonical VO recordをfield規則に従ってencodeする")
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
    /// @vtest.intent verifies record scalar fields (claim) are normalized text fragments, not byte-exact (DES-067/DES-068 make normalization the default; DES-093 "readerが具体化したcanonical VO recordをfield規則に従ってencodeする" states no byte-exact requirement for `claim`, and DES-098's manifest-entry rule is the only place §1.3 requires byte-exactness, naming the Execution State manifest's file bytes)
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
    /// @vtest.intent verifies canonical Target Reference (adapter, locator value) and construct bytes are each bound (DES-083 "canonical Target Referenceとadapterが返すimplementation construct bytesを束縛する")
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
    /// @vtest.intent verifies construct bytes are a normalized text fragment, not byte-exact (DS-348 names construct bytes as going through "正規化"; DES-098's manifest-entry rule is the only place §1.3 requires byte-exactness, naming the Execution State manifest's file bytes — construct bytes are not that requirement's subject, so by elimination the applicable normalization is DES-067/DES-068)
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
    /// @vtest.intent verifies construct bytes preserve leading whitespace and trailing-newline presence — normalization only unifies line endings and strips trailing per-line whitespace (DES-069 "改行の統一と各行末尾空白の除去以外の空白は正規化しない")
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
    /// @vtest.intent verifies a SRC ID declaration placed inside construct bytes (rust-cargo's `@vtest.src-id` doc comment) changes the hash via construct_text, which DES-089 states is correct — distinct from binding SRC ID as an independent field
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
             which DES-089 says is correct — it is not evidence of an independent SRC ID field"
        );
    }
}
