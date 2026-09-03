//! Deterministic Rust source scanner for M1.
//!
//! 詳細設計 v0.1 §1.1（本冊:30-60）に従い、Rust 固有の source discovery
//! （`syn` によるソース走査・`Cargo.toml` 解析・doc comment 宣言文法の解析・
//! モジュールパス解決）は `vtest-adapter-rust` の `SourceDiscoveryAdapter`
//! 実装（`RustCargoAdapter`）へ委譲する。この crate（core）は adapter が
//! 返した discovery 結果の検証・統合、Test ID の大域的一意性検査
//! （E-SCAN-002）、`covers` の VO 参照解決（E-SCAN-003）、Target Reference
//! 解決（E-SCAN-004/005/011、§6.1）、record 層の参照整合性検査を所有する
//! （本冊:571「VO参照の解決とTest IDの大局的一意性はadapterではなくcoreが
//! 検査する」、本冊 §5.1 手順3-7）。opaque locator の構文は解釈しない
//! （本冊:521-522「coreはpath、module、symbol種別を分解しない」）。

use std::{
    collections::{BTreeMap, BTreeSet},
    fs, io,
    path::{Component, Path, PathBuf},
};

use serde::Serialize;
use thiserror::Error;
use vtest_adapter_api::{AdapterRegistry, AdapterScanConfig};
use vtest_adapter_rust::RustCargoAdapter;
use vtest_model::{
    ContentHash, CoveragePolicy, Diagnostic, DocumentRecord, ScanSummary, SourceFunction,
    SourceLocation, TargetRef, TestEntity, VoRecord,
};
use vtest_store::{
    is_valid_ulid, load_config, read_approval, read_document, read_entity_ids, read_text,
    read_vo_record, relation_ulid_payload, yaml_scalar_value, AdapterConfig, ProjectConfig,
    RelationRecord, StoreError, VerifyLayout,
};

pub mod operations;
pub use operations::*;

#[derive(Debug, Error)]
pub enum ScanError {
    #[error("store error: {0}")]
    Store(StoreError),
    #[error("I/O error at {path}: {source}")]
    Io { path: PathBuf, source: io::Error },
    /// adapterのdiscoveryが確定的に失敗した（Evidenceなし）。本冊:1645
    /// （§17.1）「E-ADAPTER-002 \| error \| adapterのdiscoveryまたはrunnerが
    /// 確定的に失敗（Evidenceなし）」。この variant を生成できる経路は
    /// `vtest_adapter_api::DiscoveryError` からの変換（下記 `From` impl）
    /// だけであり、discoveryの確定的失敗は常にこのコードへ写像されるため、
    /// コードをここで固定する（BLOCKER 4、PR #26 review round 1 — 以前は
    /// コードを一切持たない `ScanError::Discovery { path, message }` だった。
    /// 別紙C:96「`vtest scan` / `doctor`はE-ADAPTER-* / E-CONFIG-*による
    /// 操作拒否をexit 2…にする」を満たすには、対応するコードが必要）。
    #[error("[E-ADAPTER-002] source discovery failed at {path}: {message}")]
    Discovery { path: PathBuf, message: String },
    /// adapter registry 関連の確定的失敗。本冊:1644（§17.1）「E-ADAPTER-001
    /// \| error \| adapterが未登録、重複、またはregistryの宣言と実装が
    /// 不一致」。config.yaml が登録されていない adapter ID を宣言している
    /// 場合がこれに当たる（本冊:1639 の E-CONFIG-001 行は自らの適用範囲を
    /// 括弧書きで明示的に除外している：「未知・重複adapter IDは
    /// E-ADAPTER-001」。別紙C:319「registryは…未登録adapterを拒否する」。
    /// BLOCKER 4、PR #26 review round 1 — 以前はコードを持たない
    /// `ScanError::Config` だった）。
    #[error("[E-ADAPTER-001] {message}")]
    Adapter { message: String },
    #[error("config error: {0}")]
    Config(String),
}

impl ScanError {
    /// The §17.1 diagnostic code this error carries, when construction sites
    /// have committed to one. `Store`/`Io`/`Config` do not name a diagnostic
    /// code of their own at this variant's construction sites (see each,
    /// e.g. `adapter_scan_includes`'s empty-`adapters[]` rejection, which is
    /// a separate, open question — `pr3-review-1.md` §F — not part of
    /// BLOCKER 4's scope).
    pub fn code(&self) -> Option<&'static str> {
        match self {
            Self::Discovery { .. } => Some("E-ADAPTER-002"),
            Self::Adapter { .. } => Some("E-ADAPTER-001"),
            Self::Store(_) | Self::Io { .. } | Self::Config(_) => None,
        }
    }
}

impl From<StoreError> for ScanError {
    fn from(value: StoreError) -> Self {
        Self::Store(value)
    }
}

impl From<vtest_adapter_api::DiscoveryError> for ScanError {
    fn from(value: vtest_adapter_api::DiscoveryError) -> Self {
        Self::Discovery {
            path: value.path,
            message: value.message,
        }
    }
}

#[derive(Clone, Debug, Serialize)]
pub struct ScanResult {
    pub summary: ScanSummary,
    pub tests: Vec<TestEntity>,
    pub sources: Vec<SourceFunction>,
    pub diagnostics: Vec<Diagnostic>,
}

impl ScanResult {
    pub fn has_errors(&self) -> bool {
        self.diagnostics.iter().any(Diagnostic::is_error)
    }
}

pub fn scan_project(root: &Path) -> Result<ScanResult, ScanError> {
    let config = load_config(root)?;
    scan_project_with_config(root, &config)
}

/// registry に登録済みの adapter 一覧を返す（本冊 §5.1 手順1「registryと
/// configの検証」）。v0.1 の唯一の production adapter は `rust-cargo`
/// （基本仕様 §27「組込 production adapter は `rust-cargo` とし...`rust-cargo`
/// 以外の production language adapter は v0.1 の提供範囲に含めない」）。
fn adapter_registry() -> AdapterRegistry {
    let mut registry = AdapterRegistry::new();
    registry.register(Box::new(RustCargoAdapter::new()));
    registry
}

pub fn scan_project_with_config(
    root: &Path,
    config: &ProjectConfig,
) -> Result<ScanResult, ScanError> {
    let entity_ids = read_entity_ids(root)?;
    let vo_ids = entity_ids[1].iter().cloned().collect::<BTreeSet<_>>();
    // 空 adapters[] の fail-closed 拒否は既存挙動を保つ（`adapter_scan_
    // includes`のドキュメント参照。PM 裁定3・pr3-decisions.md）。戻り値は
    // このパスでは使わない — discovery は adapter 毎の `resolve_adapter_
    // includes` を使う（下記）。
    adapter_scan_includes(config).map_err(ScanError::Config)?;

    let registry = adapter_registry();
    let fallback_package = config.project.name.clone();
    let mut files = 0usize;
    let mut test_drafts = Vec::new();
    let mut source_drafts = Vec::new();
    let mut diagnostics = Vec::new();
    for adapter_config in &config.adapters {
        let Some(adapter) = registry.get(adapter_config.id.as_str()) else {
            // BLOCKER 4（PR #26 review round 1）の再裁定: 未知 adapter ID の
            // 診断コードは §2.2（本冊:158「未知adapter…はusage error
            // （E-CONFIG-001）とする」）と §17.1 の間で「食い違っている」
            // ように見えるが、§17.1 の E-CONFIG-001 の行自体（本冊:1639）が
            // 括弧書きで自らの適用範囲からこの条件を明示的に除外している:
            // 「config field型または登録adapterが検証する設定値が現在の
            // config invariantに違反（未知・重複adapter IDはE-ADAPTER-001）」。
            // §17.1 は診断コード表として §2.2 より後段にあり、各コードの
            // 正確な適用範囲を確定する逐語である。E-ADAPTER-001 の行
            // （本冊:1644「adapterが未登録、重複、またはregistryの宣言と
            // 実装が不一致」）・別紙C:319「registryは…未登録adapterを
            // 拒否する」もこの読みと一致する。したがって Issue #24 が
            // 争っていたのはこの条件ではなく（#24 は別の争点だった）、
            // 本冊内で既に自己解決している。E-ADAPTER-001 を用いる。
            //
            // 黙って discovery から除外すること自体が fail-open である点は
            // 変わらない: 走査対象が黙って減り、テスト0件の正常 scan として
            // 報告されうる（別紙C:86-87「adapter discoveryの失敗をTest 0件
            // の正常scanとして扱わない」、基本仕様:719-723「adapterが
            // 未登録...の場合、検証結果を推測でPASSへ昇格してはならない」）。
            let known_ids = registry.ids().collect::<BTreeSet<_>>();
            let known_list = if known_ids.is_empty() {
                "(none registered)".to_owned()
            } else {
                known_ids.into_iter().collect::<Vec<_>>().join(", ")
            };
            return Err(ScanError::Adapter {
                message: format!(
                    "config.yaml declares adapter id `{}` which is not registered; \
                     registered adapter id(s): {known_list}",
                    adapter_config.id
                ),
            });
        };
        let scan_config = AdapterScanConfig {
            include_paths: resolve_adapter_includes(adapter_config),
        };
        let outcome = adapter.discover(root, &fallback_package, &scan_config)?;
        files += outcome.files_scanned;
        diagnostics.extend(outcome.diagnostics);
        test_drafts.extend(outcome.tests);
        source_drafts.extend(outcome.sources);
    }

    let sources = source_drafts
        .into_iter()
        .map(|draft| SourceFunction {
            locator: draft.locator,
            src_id: draft.src_id,
            location: draft.location,
            content_hash: ContentHash::from_text(&draft.construct_text),
        })
        .collect::<Vec<_>>();

    let (tests, dedup_diagnostics) = materialize_tests(test_drafts);
    diagnostics.extend(dedup_diagnostics);
    diagnostics.extend(check_vo_references(&tests, &vo_ids));
    diagnostics.extend(resolve_targets(&tests, &sources));

    let mut result = ScanResult {
        summary: ScanSummary {
            files: files as u64,
            tests: tests.len() as u64,
            sources: sources.len() as u64,
        },
        tests,
        sources,
        diagnostics,
    };
    let doc_roots = config
        .doc
        .roots
        .iter()
        .map(|id| id.as_str().to_owned())
        .collect::<BTreeSet<_>>();
    result.diagnostics.extend(record_diagnostics(
        root,
        &entity_ids,
        &doc_roots,
        &result.tests,
        &result.sources,
    ));
    Ok(result)
}

/// 本冊:571「VO参照の解決とTest IDの大局的一意性はadapterではなくcoreが
/// 検査する」: adapter は同じ Test ID を宣言する複数 draft をそのまま返す
/// ことがある。ここで発見順に先勝ちで採用し、以降の重複には E-SCAN-002 を
/// 発行して当該 draft を落とす（後段の VO 参照解決・Target Reference 解決
/// には先勝ちの entity だけを渡す）。
fn materialize_tests(
    drafts: Vec<vtest_adapter_api::TestDraft>,
) -> (Vec<TestEntity>, Vec<Diagnostic>) {
    let mut diagnostics = Vec::new();
    let mut seen_ids = BTreeSet::new();
    let mut tests = Vec::new();
    for draft in drafts {
        if !seen_ids.insert(draft.id.clone()) {
            diagnostics.push(
                Diagnostic::error("E-SCAN-002", format!("duplicate Test ID `{}`", draft.id))
                    .with_location(draft.location.clone()),
            );
            continue;
        }
        let content_hash = ContentHash::from_text(&draft.construct_text);
        tests.push(TestEntity {
            id: draft.id,
            covers: draft.covers,
            target: draft.target,
            additional_targets: draft.additional_targets,
            intent: draft.intent,
            input: draft.input,
            expect: draft.expect,
            kind: draft.kind,
            cases: draft.cases,
            related: draft.related,
            location: draft.location,
            content_hash,
            filter: draft.filter,
            package: draft.package,
            test_target: draft.test_target,
        });
    }
    (tests, diagnostics)
}

/// 本冊:571「VO参照の解決...はadapterではなくcoreが検査する」（E-SCAN-003）。
fn check_vo_references(tests: &[TestEntity], vo_ids: &BTreeSet<String>) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();
    for test in tests {
        for vo_id in &test.covers {
            if !vo_ids.contains(vo_id.as_str()) {
                diagnostics.push(
                    Diagnostic::error(
                        "E-SCAN-003",
                        format!("test `{}` references missing VO `{vo_id}`", test.id),
                    )
                    .with_location(test.location.clone()),
                );
            }
        }
    }
    diagnostics
}

/// Target Reference 解決（本冊 §6.1・§6.1.1、E-SCAN-004/005/011）。opaque
/// locator の完全一致検索は adapter が構築した source index（`sources`）に
/// 対して行うだけで、構文自体は解釈しない。この解決は core の単一経路が
/// 所有する（本冊:990-1005 §6.3 冒頭「この解決はcoreの単一経路が所有し」）。
fn resolve_targets(tests: &[TestEntity], sources: &[SourceFunction]) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();
    // §6.2 のSRC索引: locatorの完全一致検索に使う。1件だけ一致すれば
    // 解決、0件または複数件は解決失敗（E-SCAN-004。本冊:979-988）。
    let mut locators = BTreeMap::<String, usize>::new();
    // 恒久SRC IDごとの宣言元canonical locator一覧。1件なら`SRC索引`
    // （本冊:571・§6.1）を経て一意にcanonical locatorへ解決する。2件以上は
    // 恒久SRC IDのrepository全体一意性違反であり、E-SCAN-011とする
    // （基本仕様§9.2「同一SRC IDの複数宣言を曖昧参照として受理しない」、
    // 本冊§5.1手順5「adapter間を含む...SRC ID衝突...を検査する」）。この
    // 検査は当該SRC IDを参照するTestの有無に関わらず、索引構築時点で行う。
    let mut src_id_locators = BTreeMap::<String, Vec<String>>::new();
    let mut src_id_first_location = BTreeMap::<String, SourceLocation>::new();
    for source in sources {
        *locators.entry(source.locator.as_string()).or_default() += 1;
        if let Some(src_id) = &source.src_id {
            src_id_locators
                .entry(src_id.as_str().to_owned())
                .or_default()
                .push(source.locator.as_string());
            src_id_first_location
                .entry(src_id.as_str().to_owned())
                .or_insert_with(|| source.location.clone());
        }
    }
    for (src_id, locs) in &src_id_locators {
        if locs.len() > 1 {
            diagnostics.push(
                Diagnostic::error(
                    "E-SCAN-011",
                    format!(
                        "permanent SRC ID `{src_id}` is declared by {} Source Targets",
                        locs.len()
                    ),
                )
                .with_location(src_id_first_location[src_id].clone()),
            );
        }
    }
    for test in tests {
        // §6.1.1: `TestEntity.targets` に宣言された各 `TargetRef` を
        // canonical Source Target（canonical locator）へ解決する。
        // 解決できた宣言だけを (綴り, canonical locator) として集め、
        // 綴りが異なっても同一canonical Source Targetへ到達する宣言が
        // 2件以上あればE-SCAN-005とする（本冊:963-977、本冊:524-546）。
        let mut resolved_canonical = Vec::<(&str, String)>::new();
        for target in std::iter::once(&test.target).chain(&test.additional_targets) {
            match target {
                TargetRef::Locator(locator) => {
                    let key = locator.as_string();
                    if locators.get(&key).copied() == Some(1) {
                        resolved_canonical.push((locator.item_path.as_str(), key));
                    } else {
                        diagnostics.push(
                            Diagnostic::error(
                                "E-SCAN-004",
                                format!("test `{}` target cannot be resolved", test.id),
                            )
                            .with_location(test.location.clone()),
                        );
                    }
                }
                TargetRef::SrcId(src_id) => {
                    match src_id_locators.get(src_id.as_str()).map(Vec::as_slice) {
                        Some([single]) => {
                            resolved_canonical.push((src_id.as_str(), single.clone()));
                        }
                        Some(_) => {
                            // 恒久SRC IDが衝突している。E-SCAN-011は索引
                            // 構築時に既に発行済みであり、いずれのSource
                            // Targetも選択しない（本冊:901「E-SCAN-011が
                            // あるSRC ID参照は曖昧なため、関係するtarget
                            // 解決をMISMATCHとし、いずれのSource Targetも
                            // 選択しない」）。同じ衝突を二重にE-SCAN-004
                            // として報告しない。
                        }
                        None => {
                            diagnostics.push(
                                Diagnostic::error(
                                    "E-SCAN-004",
                                    format!("test `{}` target cannot be resolved", test.id),
                                )
                                .with_location(test.location.clone()),
                            );
                        }
                    }
                }
            }
        }
        let mut seen_canonical = BTreeMap::<String, &str>::new();
        for (spelling, canonical) in resolved_canonical {
            match seen_canonical.get(&canonical) {
                Some(previous_spelling) if *previous_spelling != spelling => {
                    diagnostics.push(
                        Diagnostic::error(
                            "E-SCAN-005",
                            format!(
                                "test `{}` declares multiple targets (`{previous_spelling}`, `{spelling}`) that resolve to the same Source Target `{canonical}`",
                                test.id
                            ),
                        )
                        .with_location(test.location.clone()),
                    );
                }
                _ => {
                    seen_canonical.insert(canonical, spelling);
                }
            }
        }
    }
    diagnostics
}

/// Resolves every registered adapter's `scan.include` patterns to
/// project-relative paths, unioned across all adapters.
///
/// 詳細設計 v0.1 §2.2 (本冊:158-161) requires `vtest-scan` to process every
/// configured adapter, not pick one: "異なるadapterが同じrootを共有することは
/// polyglot repositoryのために許可し、統合したTest IDは全adapterでglobal
/// uniquenessを検査する". 本冊 §5.1 confirms discovery iterates the full
/// registry ("adapter ID順にSourceDiscoveryAdapterを呼び出す... 各adapterは
/// DiscoveryBatchを返す"). This union is also used, unchanged, by
/// `operations.rs` to scope non-scan operations (`E-CONFIG-001`) to the
/// configured include paths.
///
/// Neither 本冊 nor 基本仕様 states what scan should do when `adapters` is
/// empty. `vtest-store`'s config parser deliberately accepts `adapters: []`
/// without backfilling a default (see
/// `v2_config_with_explicitly_empty_adapters_parses_to_no_adapters` in
/// `vtest-store/src/lib.rs`), but that only settles config *parsing*, not
/// what scan does at runtime. 基本:719-723 requires that an unregistered/
/// insufficient adapter never be promoted to a passing result ("adapterが
/// 未登録・能力不足・解析不能の場合、検証結果を推測でPASSへ昇格してはならな
/// い"), and 別紙C:86-87 forbids treating a failed/absent discovery as a
/// trivially-passing zero-Test scan ("adapter discoveryの失敗をTest 0件の
/// 正常scanとして扱わない"). Consistent with that fail-closed posture, and
/// absent an explicit statement either way, an empty `adapters` list is
/// rejected here as a config error (E-CONFIG-001 is 本冊:158's code family
/// for adapter-configuration problems) rather than silently scanning zero
/// files. This is an extrapolation, not a literal spec requirement — flagged
/// for owner review.
pub(crate) fn adapter_scan_includes(config: &ProjectConfig) -> Result<Vec<PathBuf>, String> {
    if config.adapters.is_empty() {
        return Err(
            "no adapters are registered in config.yaml (adapters: []); scan has nothing to discover"
                .to_owned(),
        );
    }
    Ok(config
        .adapters
        .iter()
        .flat_map(resolve_adapter_includes)
        .collect())
}

/// One `AdapterConfig` entry's `roots` × `scan.include`, resolved to
/// project-relative paths. Pure path arithmetic — not Rust-specific — so it
/// stays in core (本冊 §1.1 assigns only `syn`/Cargo-command ownership to
/// `vtest-adapter-rust`, not general path joining). Used both by
/// `adapter_scan_includes` (unioned across every configured adapter, for
/// `operations.rs`) and by `scan_project_with_config`'s per-adapter discovery
/// dispatch (only the matched adapter's own entry).
fn resolve_adapter_includes(adapter: &AdapterConfig) -> Vec<PathBuf> {
    let mut includes = Vec::new();
    for adapter_root in &adapter.roots {
        for include in &adapter.scan.include {
            let joined = Path::new(adapter_root).join(include);
            // A leading "." (from the common `roots: ["."]` default) is
            // preserved as an explicit `CurDir` component by `Path`
            // (docs: normalized away only when *not* the first
            // component), which would make `Path::starts_with` fail
            // against a bare relative path like "src/lib.rs". Strip it
            // so callers can compare against project-relative paths
            // directly.
            let normalized: PathBuf = joined
                .components()
                .filter(|component| !matches!(component, Component::CurDir))
                .collect();
            includes.push(normalized);
        }
    }
    includes
}

fn record_diagnostics(
    root: &Path,
    entity_ids: &[Vec<String>; 2],
    doc_roots: &BTreeSet<String>,
    tests: &[TestEntity],
    sources: &[SourceFunction],
) -> Vec<Diagnostic> {
    let layout = VerifyLayout::new(root);
    let mut diagnostics = Vec::new();
    let mut known_ids = BTreeSet::new();
    for ids in entity_ids {
        known_ids.extend(ids.iter().cloned());
    }
    known_ids.extend(tests.iter().map(|test| test.id.as_str().to_owned()));
    for source in sources {
        known_ids.insert(source.locator.as_string());
        if let Some(src_id) = &source.src_id {
            known_ids.insert(src_id.as_str().to_owned());
        }
    }

    // 詳細設計 v0.1 §2.1 replaces the predecessor spec/req layers with doc/
    // (本冊:30-60, vtest-store's `init_project` no longer creates
    // `.verify/spec` or `.verify/req`). `entity_ids` therefore carries only
    // [doc, vo] (`vtest_store::read_entity_ids`) — there is no third (REQ)
    // slot to validate, and REQ has no canonical counterpart at all, so its
    // validation is removed outright rather than repointed.
    let mut docs = BTreeMap::new();
    for id in &entity_ids[0] {
        if let Some(record) = validate_document_record(&layout, id, &mut diagnostics) {
            docs.insert(id.clone(), record);
        }
    }
    validate_document_graph(&layout, &docs, doc_roots, &mut diagnostics);

    let mut vos = BTreeMap::new();
    for id in &entity_ids[1] {
        if let Some(record) = validate_vo_record(&layout, id, &mut diagnostics) {
            vos.insert(id.clone(), record);
        }
    }
    validate_vo_document_references(&layout, &vos, &docs, &mut diagnostics);

    let vo_parents = vos
        .iter()
        .map(|(id, record)| {
            (
                id.clone(),
                record
                    .parent
                    .as_ref()
                    .map(|parent| parent.as_str().to_owned()),
            )
        })
        .collect::<BTreeMap<_, _>>();
    validate_parent_graph(root, &layout.vo_dir(), &vo_parents, "VO", &mut diagnostics);

    validate_relations(&layout, &known_ids, &mut diagnostics);
    validate_vo_warnings(&layout, &vos, tests, &mut diagnostics);
    validate_approval_status(&layout, &vos, &mut diagnostics);
    diagnostics
}

/// Reads and validates the canonical document record `.verify/doc/<id>.yaml`
/// (詳細設計 v0.1 §3.1), delegating every schema-intrinsic check (required
/// fields, id/file-name match, unknown fields) to `vtest_store::read_document`
/// — the record-layer reader — rather than re-checking them here (record vs.
/// scan layer split; `pr3-spec-extract.md` §7, mirrors `validate_vo_record`
/// below). This function adds the one check that reader deliberately leaves
/// to the scan layer: `content_hash` staleness against the file at `path`
/// (W-SCAN-104, 本冊:1626 §17.1) — the reader only sees the record text,
/// never the working tree, so it cannot compare against the file `path`
/// names. It deliberately does not police the `DOC-<NAME>.yaml` id/file-name
/// *shape*: 基本仕様:126-134 states the ID prefix/charset is a convention the
/// tool must not enforce, only uniqueness (PM 裁定7); the reader's
/// id-matches-file-name check above is a different, permitted rule (ファイル
/// 名を ID とする, 本冊:644), not a format constraint on what that ID may
/// contain.
fn validate_document_record(
    layout: &VerifyLayout,
    id: &str,
    diagnostics: &mut Vec<Diagnostic>,
) -> Option<DocumentRecord> {
    let path = layout.doc_dir().join(format!("{id}.yaml"));
    let location = record_location(&layout.root, &path, id);
    let (record, record_diagnostics) = match read_document(layout, id) {
        Ok(result) => result,
        Err(error) => {
            // Same E-SCAN-010 precedent `validate_vo_record` documents: every
            // failure `read_document` reports (invalid YAML, a missing
            // required field, id/file-name mismatch, or a raw I/O failure) is
            // schema non-conformance for scan's purposes.
            diagnostics.push(
                Diagnostic::error(
                    "E-SCAN-010",
                    format!("document {id} has an invalid record: {error}"),
                )
                .with_location(location),
            );
            return None;
        }
    };
    diagnostics.extend(
        record_diagnostics
            .into_iter()
            .map(|diagnostic| diagnostic.with_location(location.clone())),
    );
    if let Some(diagnostic) = document_staleness_diagnostic(&layout.root, &record, &location) {
        diagnostics.push(diagnostic);
    }
    Some(record)
}

/// W-SCAN-104 (本冊:1626 §17.1: "document レコードの content_hash と実ファイル
/// の不一致"). Recomputes the referenced file's hash the same way the
/// canonical model does (`ContentHash::from_text`, 詳細設計 v0.1 §1.3
/// normalization — the same function `validate_approval_status` already uses
/// for VO subject hashing) and compares it against the record's stored
/// `content_hash`. `path` failing to read (deleted/moved file) is treated the
/// same as a mismatch: there is no way to confirm the record is still
/// current, so this stays fail-closed rather than silently passing when the
/// file is simply gone.
fn document_staleness_diagnostic(
    root: &Path,
    record: &DocumentRecord,
    location: &SourceLocation,
) -> Option<Diagnostic> {
    let current = match fs::read_to_string(root.join(&record.path)) {
        Ok(text) => ContentHash::from_text(&text),
        Err(error) => {
            return Some(
                Diagnostic::warning(
                    "W-SCAN-104",
                    format!(
                        "document {} content_hash cannot be verified: {} is unreadable ({error})",
                        record.id, record.path
                    ),
                )
                .with_location(location.clone()),
            );
        }
    };
    if current == record.content_hash {
        return None;
    }
    Some(
        Diagnostic::warning(
            "W-SCAN-104",
            format!(
                "document {} content_hash does not match {}",
                record.id, record.path
            ),
        )
        .with_location(location.clone()),
    )
}

/// chain_integrity（文書層）と orphan_detection（別紙C §18.3.1 L76-95・§18.3.2
/// L119-125 逐語）:
/// - E-SCAN-012（本冊:878）: 各 document の `derives_from` 参照先が document
///   として存在すること。参照先集合は VO 同様、成功裏に読めた `docs`（本関数
///   の呼び出し元が構築）のみとする — `validate_parent_graph`（VO parent）が
///   既に確立した「解決先は正常にparseできたrecordの集合」という前例と揃える。
/// - E-SCAN-016（本冊:879）: 「`derives_from` が空、かつ他のどの document か
///   らも `derives_from` で参照されず、`doc.roots` にも列挙されない」の3条件
///   すべてを満たす document を孤児とする。3条件目だけを見て「根に列挙されて
///   いない」を孤児と判定しない — 別紙C:119-125 は3条件の連言である。
fn validate_document_graph(
    layout: &VerifyLayout,
    docs: &BTreeMap<String, DocumentRecord>,
    doc_roots: &BTreeSet<String>,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let referenced = docs
        .values()
        .flat_map(|record| record.derives_from.iter())
        .map(|entry| entry.doc.as_str().to_owned())
        .collect::<BTreeSet<_>>();

    for (id, record) in docs {
        let location = record_location(
            &layout.root,
            &layout.doc_dir().join(format!("{id}.yaml")),
            id,
        );
        for entry in &record.derives_from {
            let target = entry.doc.as_str();
            if !docs.contains_key(target) {
                diagnostics.push(
                    Diagnostic::error(
                        "E-SCAN-012",
                        format!("document {id} derives_from missing document {target}"),
                    )
                    .with_location(location.clone()),
                );
            }
        }
        if record.derives_from.is_empty()
            && !referenced.contains(id.as_str())
            && !doc_roots.contains(id.as_str())
        {
            diagnostics.push(
                Diagnostic::error(
                    "E-SCAN-016",
                    format!(
                        "document {id} is orphaned: empty derives_from, not referenced by \
                         another document, and not listed in config.yaml's doc.roots"
                    ),
                )
                .with_location(location),
            );
        }
    }
}

/// chain_integrity（VO 層、本冊:878/別紙C:80）: 各 VO の `derives_from` は
/// document へ解決できなければならない。カーディナリティ（1件以上）は
/// `vtest_store::vo_record_from_yaml` が既に record 層で強制しているので
/// (`require_at_least_one_derives_from`)、ここでは各 entry の参照先が実在す
/// る document かどうかだけを検査する — `validate_document_graph`の
/// E-SCAN-012チェックと対になる、VO側の半分。
fn validate_vo_document_references(
    layout: &VerifyLayout,
    vos: &BTreeMap<String, VoRecord>,
    docs: &BTreeMap<String, DocumentRecord>,
    diagnostics: &mut Vec<Diagnostic>,
) {
    for (id, record) in vos {
        let location = record_location(
            &layout.root,
            &layout.vo_dir().join(format!("{id}.yaml")),
            id,
        );
        for entry in &record.derives_from {
            let target = entry.doc.as_str();
            if !docs.contains_key(target) {
                diagnostics.push(
                    Diagnostic::error(
                        "E-SCAN-012",
                        format!("VO {id} derives_from missing document {target}"),
                    )
                    .with_location(location.clone()),
                );
            }
        }
    }
}

/// Reads and validates the canonical VO record `.verify/vo/<id>.yaml` (詳細設計
/// v0.1 §3.2), delegating every schema-intrinsic check (required fields,
/// `derives_from` cardinality, `coverage_policy` value domain, id/file-name
/// match, unknown fields) to `vtest_store::read_vo_record` — the record-layer
/// reader implemented in PR2 — rather than re-checking them here (record vs.
/// scan layer split; `pr3-spec-extract.md` §7). This function only adds the
/// one check that reader deliberately leaves to the scan layer:
/// `combinations` validity against the declared `dimensions` (E-SCAN-017,
/// 本冊:1625/別紙C:97-104) — combinations resolution needs the VO's own
/// dimension set, which canonical.rs's own doc comment says is a scan-time
/// concern, not the reader's. It deliberately does not police the
/// `VO-<NAME>.yaml` id/file-name *shape*: 基本仕様:126-134 states the ID
/// prefix/charset is a convention the tool must not enforce, only uniqueness
/// (PM 裁定7); the reader's id-matches-file-name check is a different,
/// permitted rule (ファイル名を ID とする, 本冊:644), not a format constraint
/// on what that ID may contain.
fn validate_vo_record(
    layout: &VerifyLayout,
    id: &str,
    diagnostics: &mut Vec<Diagnostic>,
) -> Option<VoRecord> {
    let path = layout.vo_dir().join(format!("{id}.yaml"));
    let location = record_location(&layout.root, &path, id);
    let (record, record_diagnostics) = match read_vo_record(layout, id) {
        Ok(result) => result,
        Err(error) => {
            // 本冊:876 E-SCAN-010「レコードのid / ファイル名 / schema不一致」
            // covers every failure mode `read_vo_record` reports (invalid
            // YAML, a missing required field, an out-of-domain
            // `coverage_policy`, an empty `derives_from`, or id/file-name
            // mismatch) — all are schema non-conformance. The one case that
            // reads less like "schema" is a raw I/O failure surfaced through
            // the same `StoreError`; the predecessor code already folded
            // that into E-SCAN-010 too, so this keeps that precedent rather
            // than inventing a new code.
            diagnostics.push(
                Diagnostic::error(
                    "E-SCAN-010",
                    format!("VO {id} has an invalid record: {error}"),
                )
                .with_location(location),
            );
            return None;
        }
    };
    diagnostics.extend(
        record_diagnostics
            .into_iter()
            .map(|diagnostic| diagnostic.with_location(location.clone())),
    );
    if let Some(message) = invalid_vo_combinations(&record) {
        diagnostics.push(
            Diagnostic::error("E-SCAN-017", format!("VO {id} {message}")).with_location(location),
        );
    }
    Some(record)
}

/// Checks `combinations` against the declared `dimensions` per the E-SCAN-017
/// condition list (本冊:1625 §17.1, 別紙C:97-104 逐語). Both are copied here
/// verbatim from the spec, in the same order:
///
/// - `coverage_policy: explicit` かつ `combinations` が欠落・null・空 list。
/// - `coverage_policy: explicit` かつ `dimensions` が空。
/// - `combinations` が空でないのに `coverage_policy` が `explicit` 以外。
/// - entry が未宣言の dimension 名を含む。
/// - entry の partition 値が当該 dimension の `partitions` にない。
/// - entry が宣言済み dimension を欠く、または同じ dimension 名を2回以上持つ。
/// - 同一 tuple を持つ entry が2件以上（重複 tuple）。
///
/// First bullet's three sub-cases (欠落・null・空 list) all reach this
/// function as `record.combinations.is_empty()`: a missing key, an explicit
/// `null`, and an explicit `[]` all normalize to `vec![]` in
/// `VoRecord.combinations` (`#[serde(default, deserialize_with = ...)]` —
/// see that field's own doc comment; empirically verified for all three,
/// not just the pre-existing missing-key case — see `vtest_store::
/// canonical::vo_record_combinations_missing_or_null_parses_as_empty_vec`),
/// so none of them fails at the record layer before reaching here.
///
/// The sixth bullet's "同じ dimension 名を2回以上持つ" half IS checked here
/// now (BLOCKER 1, PR #26 review round 2), as its own explicit branch
/// before the length-mismatch check the first half uses — not folded into
/// that check via a fabricated stand-in. This is possible because
/// `combinations`' element type, `CombinationEntry` (`vtest_model`),
/// preserves a repeated dimension name losslessly instead of a bare
/// `BTreeMap<String, String>` colliding it away or `yaml_serde::Value`
/// rejecting the whole record before a `VoRecord` is ever built (see
/// `CombinationEntry`'s own doc comment, and `vtest_store::canonical::
/// vo_record_from_yaml`'s, for how the record layer now hands this case
/// here intact — `vo_record_combination_entry_with_a_duplicate_dimension_
/// key_reaches_scan_as_e_scan_017` locks that in).
fn invalid_vo_combinations(record: &VoRecord) -> Option<String> {
    if !matches!(record.coverage_policy, Some(CoveragePolicy::Explicit)) {
        if !record.combinations.is_empty() {
            return Some("has combinations but coverage_policy is not `explicit`".to_owned());
        }
        return None;
    }
    if record.combinations.is_empty() {
        return Some("explicit coverage_policy requires at least one combination".to_owned());
    }
    if record.dimensions.is_empty() {
        return Some("explicit coverage_policy requires at least one dimension".to_owned());
    }
    let dimensions = record
        .dimensions
        .iter()
        .map(|dimension| (dimension.name.as_str(), &dimension.partitions))
        .collect::<BTreeMap<_, _>>();
    let mut unique = BTreeSet::new();
    for combination in &record.combinations {
        let duplicate_names = combination.duplicate_dimension_names();
        if !duplicate_names.is_empty() {
            return Some(format!(
                "has a combination that declares dimension `{}` more than once",
                duplicate_names.join("`, `")
            ));
        }
        if combination.len() != dimensions.len() {
            return Some("has a combination missing a declared dimension".to_owned());
        }
        for (name, value) in combination {
            let Some(partitions) = dimensions.get(name) else {
                return Some(format!(
                    "has a combination with undeclared dimension `{name}`"
                ));
            };
            if !partitions.iter().any(|partition| partition == value) {
                return Some(format!(
                    "has a combination with undeclared partition `{value}` for dimension `{name}`"
                ));
            }
        }
        if !unique.insert(combination) {
            return Some("has duplicate explicit combinations".to_owned());
        }
    }
    None
}

fn missing_fields(text: &str, fields: &[&str]) -> Option<String> {
    let missing = fields
        .iter()
        .copied()
        .filter(|field| {
            yaml_scalar_value(text, field)
                .map(|value| value.trim().is_empty())
                .unwrap_or(true)
        })
        .collect::<Vec<_>>();
    (!missing.is_empty()).then(|| missing.join(", "))
}

fn record_location(root: &Path, path: &Path, entity: &str) -> SourceLocation {
    let text = fs::read_to_string(path).unwrap_or_default();
    SourceLocation {
        file: path
            .strip_prefix(root)
            .unwrap_or(path)
            .to_string_lossy()
            .replace('\\', "/"),
        function: entity.to_owned(),
        start_line: 1,
        end_line: text.lines().count().max(1) as u64,
        start_byte: 0,
        end_byte: text.len() as u64,
    }
}

fn validate_parent_graph(
    root: &Path,
    directory: &Path,
    parents: &BTreeMap<String, Option<String>>,
    kind: &str,
    diagnostics: &mut Vec<Diagnostic>,
) {
    for (id, parent) in parents {
        if let Some(parent) = parent {
            if !parents.contains_key(parent) {
                diagnostics.push(
                    Diagnostic::error(
                        "E-SCAN-008",
                        format!("{kind} {id} references missing parent {parent}"),
                    )
                    .with_location(record_location(
                        root,
                        &directory.join(format!("{id}.yaml")),
                        id,
                    )),
                );
            }
        }
    }

    let mut reported = BTreeSet::new();
    for start in parents.keys() {
        let mut path = Vec::new();
        let mut positions = BTreeMap::new();
        let mut current = start.clone();
        loop {
            if let Some(index) = positions.get(&current) {
                let cycle = path[*index..].to_vec();
                let mut key_parts = cycle.clone();
                key_parts.sort();
                let key = key_parts.join("|");
                if reported.insert(key) {
                    diagnostics.push(
                        Diagnostic::error(
                            "E-SCAN-008",
                            format!("{kind} parent cycle: {}", cycle.join(" -> ")),
                        )
                        .with_location(record_location(
                            root,
                            &directory.join(format!("{current}.yaml")),
                            &current,
                        )),
                    );
                }
                break;
            }
            positions.insert(current.clone(), path.len());
            path.push(current.clone());
            let Some(Some(parent)) = parents.get(&current) else {
                break;
            };
            if !parents.contains_key(parent) {
                break;
            }
            current = parent.clone();
        }
    }
}

fn validate_relations(
    layout: &VerifyLayout,
    known_ids: &BTreeSet<String>,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let entries = match fs::read_dir(layout.relation_dir()) {
        Ok(entries) => entries,
        Err(_) => return,
    };
    let mut paths = entries
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| path.extension().and_then(|value| value.to_str()) == Some("yaml"))
        .collect::<Vec<_>>();
    paths.sort();
    let mut relation_payloads = BTreeMap::<String, String>::new();
    for path in paths {
        let file_id = path
            .file_stem()
            .and_then(|value| value.to_str())
            .unwrap_or_default()
            .to_owned();
        let location = record_location(&layout.root, &path, &file_id);
        if let Some(payload) = relation_ulid_payload(&file_id) {
            if let Some(first) = relation_payloads.insert(payload.to_owned(), file_id.clone()) {
                diagnostics.push(
                    Diagnostic::error(
                        "E-SCAN-010",
                        format!(
                            "relation IDs {first} and {file_id} use the same ULID payload {payload}"
                        ),
                    )
                    .with_location(location.clone()),
                );
                continue;
            }
        }
        let text = match read_text(&path) {
            Ok(text) => text,
            Err(error) => {
                diagnostics.push(
                    Diagnostic::error(
                        "E-SCAN-010",
                        format!("relation {file_id} cannot be read: {error}"),
                    )
                    .with_location(location.clone()),
                );
                continue;
            }
        };
        let (relation, relation_diagnostics) = match RelationRecord::from_yaml(&text, &file_id) {
            Ok(parsed) => parsed,
            Err(error) => {
                diagnostics.push(
                    Diagnostic::error(
                        "E-SCAN-010",
                        format!("relation {file_id} has an invalid schema: {error}"),
                    )
                    .with_location(location.clone()),
                );
                continue;
            }
        };
        diagnostics.extend(relation_diagnostics);
        for (field, value) in [("from", relation.from), ("to", relation.to)] {
            if !known_ids.contains(&value) {
                diagnostics.push(
                    Diagnostic::error(
                        "E-SCAN-009",
                        format!("relation {file_id} {field} references missing entity {value}"),
                    )
                    .with_location(location.clone()),
                );
            }
        }
    }
}

fn validate_vo_warnings(
    layout: &VerifyLayout,
    vos: &BTreeMap<String, VoRecord>,
    tests: &[TestEntity],
    diagnostics: &mut Vec<Diagnostic>,
) {
    let child_ids = vos
        .values()
        .filter_map(|vo| vo.parent.as_ref().map(|parent| parent.as_str().to_owned()))
        .collect::<BTreeSet<_>>();
    let covered_ids = tests
        .iter()
        .flat_map(|test| test.covers.iter().map(|vo| vo.as_str().to_owned()))
        .collect::<BTreeSet<_>>();
    for id in vos.keys() {
        if !child_ids.contains(id) && !covered_ids.contains(id) {
            diagnostics.push(
                Diagnostic::warning(
                    "W-SCAN-102",
                    format!("VO {id} is isolated and has no covering test"),
                )
                .with_location(record_location(
                    &layout.root,
                    &layout.vo_dir().join(format!("{id}.yaml")),
                    id,
                )),
            );
        }
    }
    for test in tests {
        for vo_id in &test.covers {
            if child_ids.contains(vo_id.as_str()) {
                diagnostics.push(
                    Diagnostic::warning(
                        "W-SCAN-103",
                        format!("test {} covers non-leaf VO {}", test.id, vo_id),
                    )
                    .with_location(test.location.clone()),
                );
            }
        }
    }
}

/// Validates approval record schema (this record type is not yet migrated to
/// a canonical `vtest-store` reader; out of this PR's scope). The VO layer's
/// own `status`-vs-approval mismatch diagnostic (W-STORE-001) is no longer
/// computed here: 詳細設計 v0.1 §3.2 defines W-STORE-001 as firing on the
/// read-compat `status` field's mere *presence* ("存在自体をW-STORE-001として
/// 通知する"), not on a mismatch against an approval-derived value, and
/// `read_vo_record` (via `validate_vo_record`) already emits it on that
/// condition — recomputing an approval-derived status here and diffing it
/// against `status` would both duplicate that check and apply the wrong
/// condition.
fn validate_approval_status(
    layout: &VerifyLayout,
    vos: &BTreeMap<String, VoRecord>,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let mut current_hashes = BTreeMap::new();
    for id in vos.keys() {
        let path = layout.vo_dir().join(format!("{id}.yaml"));
        if let Ok(text) = read_text(&path) {
            current_hashes.insert(id.clone(), ContentHash::from_text(&text));
        }
    }
    let entries = match fs::read_dir(layout.approvals_dir()) {
        Ok(entries) => entries,
        Err(_) => return,
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|value| value.to_str()) != Some("yaml") {
            continue;
        }
        let file_id = path
            .file_stem()
            .and_then(|value| value.to_str())
            .unwrap_or_default()
            .to_owned();
        let location = record_location(&layout.root, &path, &file_id);
        let text = match read_text(&path) {
            Ok(text) => text,
            Err(error) => {
                diagnostics.push(
                    Diagnostic::error(
                        "E-SCAN-010",
                        format!("approval {file_id} cannot be read: {error}"),
                    )
                    .with_location(location.clone()),
                );
                continue;
            }
        };
        let mut invalid = false;
        if !is_valid_ulid(&file_id) {
            diagnostics.push(
                Diagnostic::error(
                    "E-SCAN-010",
                    format!("approval file name {file_id} is not a valid ULID"),
                )
                .with_location(location.clone()),
            );
            invalid = true;
        }
        if let Some(missing) =
            missing_fields(&text, &["id", "subject", "subject_hash", "approved_at"])
        {
            diagnostics.push(
                Diagnostic::error(
                    "E-SCAN-010",
                    format!("approval {file_id} is missing required fields: {missing}"),
                )
                .with_location(location.clone()),
            );
            invalid = true;
        }
        let approval = match read_approval(&path) {
            Ok(approval) => approval,
            Err(error) => {
                diagnostics.push(
                    Diagnostic::error(
                        "E-SCAN-010",
                        format!("approval {file_id} has an invalid schema: {error}"),
                    )
                    .with_location(location.clone()),
                );
                continue;
            }
        };
        if approval.id != file_id {
            diagnostics.push(
                Diagnostic::error(
                    "E-SCAN-010",
                    format!(
                        "approval file name {file_id} does not match record id {}",
                        approval.id
                    ),
                )
                .with_location(location.clone()),
            );
            invalid = true;
        }
        if invalid {
            continue;
        }
        let subject = approval.subject.as_str();
        if !current_hashes.contains_key(subject) {
            diagnostics.push(
                Diagnostic::error(
                    "E-SCAN-010",
                    format!("approval {file_id} references missing VO {subject}"),
                )
                .with_location(location),
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};
    use vtest_model::{DerivesFrom, DocumentId, DocumentRecord, SrcId, TestId, TestTarget, VoId};
    use vtest_store::{init_project, new_record_id, write_document, FormValue};

    fn valid_vo(id: &str, parent: &str) -> String {
        format!(
            "id: {id}\nparent: {parent}\nderives_from:\n  - doc: DOC-TEST\nclaim: claim\ndimensions: []\ncoverage_policy: null\ncombinations: []\nrepresentative_cases: []\ncreated: '2026-01-01'\nupdated: '2026-01-01'\n"
        )
    }

    /// Registers `DOC-TEST` — the document every `valid_vo` fixture record
    /// declares as its `derives_from` target — as a resolvable, non-stale,
    /// root document. Adding E-SCAN-012/E-SCAN-016 (this PR) would otherwise
    /// make every existing VO fixture that uses `valid_vo` dangling
    /// (`derives_from: [{doc: DOC-TEST}]` pointing at a document that never
    /// existed before this PR) and every such VO fixture unresolvable, since
    /// nothing wrote a `.verify/doc/DOC-TEST.yaml`.
    fn write_doc_test_fixture(root: &Path) {
        let layout = VerifyLayout::new(root);
        fs::create_dir_all(root.join("docs")).unwrap();
        let text = "fixture document\n";
        fs::write(root.join("docs/test.md"), text).unwrap();
        write_document(
            &layout,
            &DocumentRecord {
                id: DocumentId::new("DOC-TEST"),
                path: "docs/test.md".to_owned(),
                content_hash: ContentHash::from_text(text),
                title: None,
                derives_from: Vec::new(),
                registered_at: "2026-01-01T00:00:00Z".to_owned(),
            },
        )
        .unwrap();
        let mut config = load_config(root).unwrap();
        config.doc.roots = vec![DocumentId::new("DOC-TEST")];
        fs::write(layout.config(), config.to_yaml()).unwrap();
    }

    fn fixture() -> PathBuf {
        let suffix = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = std::env::temp_dir().join(format!("vtest-scan-{suffix}"));
        fs::create_dir_all(root.join("src")).unwrap();
        fs::create_dir_all(root.join("tests")).unwrap();
        fs::write(
            root.join("Cargo.toml"),
            "[package]\nname = \"fixture\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
        )
        .unwrap();
        init_project(&root, "fixture").unwrap();
        write_doc_test_fixture(&root);
        fs::write(
            root.join("src/lib.rs"),
            "pub fn add(a: i32, b: i32) -> i32 { a + b }\n",
        )
        .unwrap();
        fs::write(
            root.join("tests/calc.rs"),
            r#"
/// @vtest.id TEST-ADD
/// @vtest.covers VO-ADD
/// @vtest.target src/lib.rs::add
/// @vtest.intent adds values
#[test]
fn adds() { assert_eq!(2, crate::missing()); }
"#,
        )
        .unwrap();
        fs::write(
            root.join(".verify/vo/VO-ADD.yaml"),
            valid_vo("VO-ADD", "null"),
        )
        .unwrap();
        root
    }

    #[test]
    fn extracts_annotated_test_and_source() {
        let root = fixture();
        let result = scan_project(&root).unwrap();
        assert_eq!(result.summary.tests, 1);
        assert_eq!(result.summary.sources, 2);
        assert!(
            !result.has_errors(),
            "diagnostics: {:?}",
            result.diagnostics
        );
        assert_eq!(result.tests[0].id.as_str(), "TEST-ADD");
        assert_eq!(result.tests[0].filter, "adds");
        assert_eq!(result.tests[0].package, "fixture");
        assert_eq!(
            result.tests[0].test_target,
            TestTarget::IntegrationTest("calc".to_owned())
        );
    }

    /// 未知 adapter ID の fail-closed 拒否。拒否すること自体は別紙C:86-87・
    /// 基本仕様:719-723 により確定しており、診断コードは本冊:1639 の
    /// E-CONFIG-001 行が自らの適用範囲を括弧書きで除外した先の
    /// E-ADAPTER-001（本冊:1644・別紙C:319）で確定する（BLOCKER 4、PR #26
    /// review round 1 — 旧版はコードを持たない `ScanError::Config` を返し、
    /// Issue #24 のコード選択を保留扱いにしていた）。config.yaml の唯一の
    /// adapter エントリを未登録 ID へ書き換えると、discovery からの黙った
    /// 除外（旧挙動: テスト0件の正常 scan）ではなく `ScanError::Adapter`
    /// （`.code() == Some("E-ADAPTER-001")`）を返すこと、かつそのメッセージ
    /// が未登録だった ID と登録済み ID 一覧の両方を含むことを確認する。
    #[test]
    fn unknown_adapter_id_is_rejected_fail_closed() {
        let root = fixture();
        let layout = VerifyLayout::new(&root);
        let mut config = load_config(&root).unwrap();
        assert_eq!(config.adapters.len(), 1, "fixture registers one adapter");
        config.adapters[0].id = "unknown-lang".to_owned();
        fs::write(layout.config(), config.to_yaml()).unwrap();

        let error = match scan_project(&root) {
            Err(err @ ScanError::Adapter { .. }) => {
                assert_eq!(err.code(), Some("E-ADAPTER-001"));
                err.to_string()
            }
            other => {
                panic!("expected ScanError::Adapter for an unregistered adapter id, got {other:?}")
            }
        };
        assert!(
            error.contains("unknown-lang"),
            "error should name the unregistered id: {error}"
        );
        assert!(
            error.contains("rust-cargo"),
            "error should list the registered id(s): {error}"
        );
    }

    /// 本冊:1645（§17.1）「E-ADAPTER-002 \| error \| adapterのdiscoveryまたは
    /// runnerが確定的に失敗（Evidenceなし）」。`vtest_adapter_api::
    /// DiscoveryError` から変換された `ScanError::Discovery` は、adapter
    /// discovery が確定的に失敗した経路（`vtest-adapter-rust`の
    /// `collect_rs_files`失敗・byte range逸脱の2箇所。BLOCKER 4、PR #26
    /// review round 1）のいずれからでも常にこのコードへ写像されることを、
    /// 変換経路を単体で断言してロックインする（filesystem 権限操作に頼らず
    /// 決定論的に検証するため、`scan_project`の全体経路ではなく`From`
    /// 変換自体を対象にする）。
    #[test]
    fn discovery_error_conversion_carries_e_adapter_002() {
        let error: ScanError = vtest_adapter_api::DiscoveryError {
            path: PathBuf::from("src/lib.rs"),
            message: "boom".to_owned(),
        }
        .into();
        assert_eq!(error.code(), Some("E-ADAPTER-002"));
        assert!(
            error.to_string().starts_with("[E-ADAPTER-002]"),
            "error: {error}"
        );
    }

    #[test]
    fn missing_or_invalid_cargo_metadata_is_fail_closed() {
        let root = fixture();
        fs::write(root.join("Cargo.toml"), "[package\ninvalid = true\n").unwrap();

        let result = scan_project(&root).unwrap();
        assert!(result.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "E-SCAN-004"
                && diagnostic.location.as_ref().is_some_and(|location| {
                    location.file == "tests/calc.rs" && location.function == "adds"
                })
        }));
        assert!(matches!(result.tests[0].test_target, TestTarget::Unknown));

        let root = fixture();
        fs::remove_file(root.join("Cargo.toml")).unwrap();
        let result = scan_project(&root).unwrap();
        assert!(result.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "E-SCAN-004"
                && diagnostic.location.as_ref().is_some_and(|location| {
                    location.file == "tests/calc.rs" && location.function == "adds"
                })
        }));
        assert!(matches!(result.tests[0].test_target, TestTarget::Unknown));
    }

    #[test]
    fn resolves_workspace_packages_targets_and_external_module_filters() {
        let root = fixture();
        fs::create_dir_all(root.join("crates/parser/src/runtime")).unwrap();
        fs::create_dir_all(root.join("crates/parser/src/bin")).unwrap();
        fs::create_dir_all(root.join("crates/parser/tests/suite")).unwrap();
        fs::write(
            root.join("crates/parser/Cargo.toml"),
            "[package]\nname = \"parser-crate\" # an inline TOML comment\nversion = \"0.1.0\"\nedition = \"2021\"\nautobins = false\nautotests = false\n\n[lib]\npath = \"src/runtime/mod.rs\"\n\n[[bin]]\nname = \"parser-check\"\npath = \"src/bin/check.rs\"\n\n[[test]]\nname = \"parser-suite\"\npath = \"tests/suite/main.rs\"\n",
        )
        .unwrap();
        fs::write(
            root.join("crates/parser/src/runtime/mod.rs"),
            "pub mod parser;\n",
        )
        .unwrap();
        fs::write(
            root.join("crates/parser/src/runtime/parser.rs"),
            r#"
pub fn parse() {}

#[cfg(test)]
mod tests {
    /// @vtest.id TEST-PARSER-MODULE
    /// @vtest.covers VO-ADD
    /// @vtest.target crates/parser/src/runtime/parser.rs::parse
    /// @vtest.intent parses from an external module
    #[test]
    fn parses_external_module() { super::parse(); }
}
"#,
        )
        .unwrap();
        fs::write(
            root.join("crates/parser/src/bin/check.rs"),
            r#"
fn main() {}

/// @vtest.id TEST-PARSER-BIN
/// @vtest.covers VO-ADD
/// @vtest.target crates/parser/src/runtime/parser.rs::parse
/// @vtest.intent checks the parser binary
#[test]
fn checks_binary() {}
"#,
        )
        .unwrap();
        fs::write(
            root.join("crates/parser/tests/suite/main.rs"),
            "mod support;\n",
        )
        .unwrap();
        fs::write(
            root.join("crates/parser/tests/suite/support.rs"),
            r#"
pub fn exercise() {}

/// @vtest.id TEST-PARSER-INTEGRATION
/// @vtest.covers VO-ADD
/// @vtest.target crates/parser/src/runtime/parser.rs::parse
/// @vtest.intent parses through an integration target
#[test]
fn parses_integration() { exercise(); }
"#,
        )
        .unwrap();

        let result = scan_project(&root).unwrap();
        let module_test = result
            .tests
            .iter()
            .find(|test| test.id.as_str() == "TEST-PARSER-MODULE")
            .unwrap();
        assert_eq!(module_test.package, "parser-crate");
        assert_eq!(module_test.test_target, TestTarget::Lib);
        assert_eq!(module_test.filter, "parser::tests::parses_external_module");

        let integration = result
            .tests
            .iter()
            .find(|test| test.id.as_str() == "TEST-PARSER-INTEGRATION")
            .unwrap();
        assert_eq!(integration.package, "parser-crate");
        assert_eq!(
            integration.test_target,
            TestTarget::IntegrationTest("parser-suite".to_owned())
        );
        assert_eq!(integration.filter, "support::parses_integration");

        let binary = result
            .tests
            .iter()
            .find(|test| test.id.as_str() == "TEST-PARSER-BIN")
            .unwrap();
        assert_eq!(binary.package, "parser-crate");
        assert_eq!(
            binary.test_target,
            TestTarget::Bin("parser-check".to_owned())
        );
        assert_eq!(binary.filter, "checks_binary");
    }

    #[test]
    fn ignored_rust_files_are_not_scanned() {
        let root = fixture();
        fs::write(root.join(".gitignore"), "src/ignored.rs\n").unwrap();
        fs::write(root.join(".ignore"), "src/kept.rs\n").unwrap();
        fs::write(root.join("src/ignored.rs"), "this is not rust\n").unwrap();
        fs::write(root.join("src/kept.rs"), "pub fn kept() {}\n").unwrap();

        let result = scan_project(&root).unwrap();
        assert!(!result.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "E-SCAN-001" && diagnostic.message.contains("ignored.rs")
        }));
        assert!(!result
            .sources
            .iter()
            .any(|source| source.location.file == "src/ignored.rs"));
        assert!(result
            .sources
            .iter()
            .any(|source| source.location.file == "src/kept.rs"));
    }

    #[test]
    fn ambiguous_target_locator_is_not_resolved() {
        let root = fixture();
        fs::write(
            root.join("src/ambiguous.rs"),
            r#"
#[cfg(feature = "left")]
pub fn duplicate() {}
#[cfg(not(feature = "left"))]
pub fn duplicate() {}
"#,
        )
        .unwrap();
        fs::write(
            root.join("tests/ambiguous.rs"),
            r#"
/// @vtest.id TEST-AMBIGUOUS
/// @vtest.covers VO-ADD
/// @vtest.target src/ambiguous.rs::duplicate
/// @vtest.intent rejects an ambiguous source locator
#[test]
fn ambiguous() {}
"#,
        )
        .unwrap();

        let result = scan_project(&root).unwrap();
        assert!(result.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "E-SCAN-004"
                && diagnostic
                    .location
                    .as_ref()
                    .is_some_and(|location| location.function == "ambiguous")
        }));
    }

    #[test]
    fn reports_unregistered_tests() {
        let root = fixture();
        fs::write(root.join("tests/unregistered.rs"), "#[test]\nfn x() {}\n").unwrap();
        let result = scan_project(&root).unwrap();
        assert!(result.diagnostics.iter().any(|d| d.code == "W-SCAN-101"));
    }

    #[test]
    fn rejects_unknown_and_duplicate_annotation_keys() {
        let root = fixture();
        fs::write(
            root.join("tests/invalid.rs"),
            r#"
/// @vtest.id TEST-UNKNOWN
/// @vtest.covers VO-ADD
/// @vtest.target src/lib.rs::add
/// @vtest.intent invalid
/// @vtest.typo value
#[test]
fn unknown_key() {}

/// @vtest.id TEST-DUPLICATE
/// @vtest.id TEST-DUPLICATE-2
/// @vtest.covers VO-ADD
/// @vtest.target src/lib.rs::add
/// @vtest.intent invalid
#[test]
fn duplicate_key() {}
"#,
        )
        .unwrap();
        let result = scan_project(&root).unwrap();
        assert!(result.diagnostics.iter().any(|d| d.code == "E-SCAN-005"));
        assert!(result.diagnostics.iter().any(|d| d.code == "E-SCAN-006"));
    }

    #[test]
    fn rejects_missing_required_annotation() {
        let root = fixture();
        fs::write(
            root.join("tests/invalid.rs"),
            r#"
/// @vtest.id TEST-MISSING
/// @vtest.covers VO-ADD
/// @vtest.target src/lib.rs::does_not_exist
#[test]
fn missing_intent() {}
"#,
        )
        .unwrap();
        let result = scan_project(&root).unwrap();
        assert!(result.diagnostics.iter().any(|d| d.code == "E-SCAN-007"));
    }

    #[test]
    fn integration_tests_allow_multiple_targets_only() {
        let root = fixture();
        fs::write(
            root.join("src/lib.rs"),
            "pub fn add(a: i32, b: i32) -> i32 { a + b }\npub fn subtract(a: i32, b: i32) -> i32 { a - b }\n",
        )
        .unwrap();
        fs::write(
            root.join("tests/multiple.rs"),
            r#"
/// @vtest.id TEST-INTEGRATION
/// @vtest.covers VO-ADD
/// @vtest.target src/lib.rs::add
/// @vtest.target src/lib.rs::subtract
/// @vtest.intent combines operations
/// @vtest.kind integration-normal
#[test]
fn combines() {}

/// @vtest.id TEST-UNIT-DUPLICATE
/// @vtest.covers VO-ADD
/// @vtest.target src/lib.rs::add
/// @vtest.target src/lib.rs::subtract
/// @vtest.intent invalid duplicate
/// @vtest.kind unit-normal
#[test]
fn duplicate_target() {}
"#,
        )
        .unwrap();
        let result = scan_project(&root).unwrap();
        let integration = result
            .tests
            .iter()
            .find(|test| test.id.as_str() == "TEST-INTEGRATION")
            .unwrap();
        assert_eq!(integration.additional_targets.len(), 1);
        assert!(result.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "E-SCAN-005"
                && diagnostic
                    .location
                    .as_ref()
                    .is_some_and(|location| location.function == "duplicate_target")
        }));
    }

    /// 本冊 §4.2「許容された複数 `target` 内でも同じ TargetRef の重複は
    /// E-SCAN-005 とする」— integration kind でも同一 target の重複宣言は
    /// 許容しない。
    #[test]
    fn integration_test_duplicate_target_value_is_rejected() {
        let root = fixture();
        fs::write(
            root.join("tests/multiple_same.rs"),
            r#"
/// @vtest.id TEST-INTEGRATION-DUPLICATE
/// @vtest.covers VO-ADD
/// @vtest.target src/lib.rs::add
/// @vtest.target src/lib.rs::add
/// @vtest.intent rejects the same target declared twice
/// @vtest.kind integration-normal
#[test]
fn same_target_twice() {}
"#,
        )
        .unwrap();
        let result = scan_project(&root).unwrap();
        assert!(result.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "E-SCAN-005"
                && diagnostic
                    .location
                    .as_ref()
                    .is_some_and(|location| location.function == "same_target_twice")
        }));
        assert!(!result
            .tests
            .iter()
            .any(|test| test.id.as_str() == "TEST-INTEGRATION-DUPLICATE"));
    }

    /// 本冊 §4.2「1行1キー。`covers` と `related` の値はカンマ区切りで
    /// 複数指定できる」。
    #[test]
    fn covers_and_related_accept_comma_separated_values() {
        let root = fixture();
        fs::write(
            root.join(".verify/vo/VO-SECOND.yaml"),
            valid_vo("VO-SECOND", "null"),
        )
        .unwrap();
        fs::write(
            root.join("tests/comma_separated.rs"),
            r#"
/// @vtest.id TEST-COMMA
/// @vtest.covers VO-ADD, VO-SECOND
/// @vtest.target src/lib.rs::add
/// @vtest.intent accepts a comma-separated covers and related list
/// @vtest.related TEST-ADD, TEST-COMMA-OTHER
#[test]
fn comma_separated() {}
"#,
        )
        .unwrap();
        let result = scan_project(&root).unwrap();
        assert!(
            !result.has_errors(),
            "diagnostics: {:?}",
            result.diagnostics
        );
        let test = result
            .tests
            .iter()
            .find(|test| test.id.as_str() == "TEST-COMMA")
            .unwrap();
        assert_eq!(
            test.covers.iter().map(VoId::as_str).collect::<Vec<_>>(),
            vec!["VO-ADD", "VO-SECOND"]
        );
        assert_eq!(
            test.related.iter().map(TestId::as_str).collect::<Vec<_>>(),
            vec!["TEST-ADD", "TEST-COMMA-OTHER"]
        );
    }

    /// 本冊 §4.2「`case` と `related` はキー自体を複数行書ける」。
    #[test]
    fn case_and_related_allow_repeated_annotation_lines() {
        let root = fixture();
        fs::write(
            root.join("tests/repeated.rs"),
            r#"
/// @vtest.id TEST-REPEATED
/// @vtest.covers VO-ADD
/// @vtest.target src/lib.rs::add
/// @vtest.intent allows case and related to repeat as separate lines
/// @vtest.case zero
/// @vtest.case negative
/// @vtest.related TEST-ADD
/// @vtest.related TEST-REPEATED-OTHER
#[test]
fn repeated() {}
"#,
        )
        .unwrap();
        let result = scan_project(&root).unwrap();
        assert!(
            !result.has_errors(),
            "diagnostics: {:?}",
            result.diagnostics
        );
        let test = result
            .tests
            .iter()
            .find(|test| test.id.as_str() == "TEST-REPEATED")
            .unwrap();
        assert_eq!(test.cases, vec!["zero".to_owned(), "negative".to_owned()]);
        assert_eq!(
            test.related.iter().map(TestId::as_str).collect::<Vec<_>>(),
            vec!["TEST-ADD", "TEST-REPEATED-OTHER"]
        );
    }

    /// 本冊 §4.2「表面1で、`@vtest.` で始まるが test-key を持たない行は
    /// エラー E-SCAN-006... 未知キーに加え、source-target-key（`src-id`）の
    /// 誤配置も含む」。
    #[test]
    fn src_id_annotation_on_a_test_construct_is_rejected_as_an_unknown_key() {
        let root = fixture();
        fs::write(
            root.join("tests/misplaced_src_id.rs"),
            r#"
/// @vtest.id TEST-MISPLACED-SRC-ID
/// @vtest.covers VO-ADD
/// @vtest.target src/lib.rs::add
/// @vtest.intent rejects a src-id declared on a Test construct
/// @vtest.src-id SRC-MISPLACED
#[test]
fn misplaced() {}
"#,
        )
        .unwrap();
        let result = scan_project(&root).unwrap();
        assert!(result.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "E-SCAN-006"
                && diagnostic
                    .location
                    .as_ref()
                    .is_some_and(|location| location.function == "misplaced")
        }));
        assert!(!result
            .tests
            .iter()
            .any(|test| test.id.as_str() == "TEST-MISPLACED-SRC-ID"));
    }

    /// 本冊 §4.2「表面2で、`@vtest.` で始まるが source-target-key を
    /// 持たない行（test-key を含む）は警告 W-SCAN-105 とする」。
    #[test]
    fn non_test_item_with_a_test_key_annotation_only_warns() {
        let root = fixture();
        fs::write(
            root.join("src/lib.rs"),
            "pub fn add(a: i32, b: i32) -> i32 { a + b }\n\n\
             /// @vtest.id TEST-MISPLACED-ON-HELPER\n\
             pub fn helper() -> i32 { 0 }\n",
        )
        .unwrap();
        let result = scan_project(&root).unwrap();
        assert!(result.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "W-SCAN-105"
                && diagnostic
                    .location
                    .as_ref()
                    .is_some_and(|location| location.function == "helper")
        }));
        assert!(!result.diagnostics.iter().any(|diagnostic| {
            diagnostic.is_error()
                && diagnostic
                    .location
                    .as_ref()
                    .is_some_and(|location| location.function == "helper")
        }));
    }

    /// 本冊 §4.2「`src-id` は表面2でも反復不可であり...このときいずれの
    /// 宣言値も採用せず、当該Source TargetのSRC IDは無しとして扱う
    /// （どちらかを推測で選ばない）」。
    #[test]
    fn duplicate_src_id_on_a_source_target_is_rejected_and_neither_value_is_adopted() {
        let root = fixture();
        fs::write(
            root.join("src/lib.rs"),
            "pub fn add(a: i32, b: i32) -> i32 { a + b }\n\n\
             /// @vtest.src-id SRC-FIRST\n\
             /// @vtest.src-id SRC-SECOND\n\
             pub fn helper() -> i32 { 0 }\n",
        )
        .unwrap();
        let result = scan_project(&root).unwrap();
        assert!(result.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "E-SCAN-005"
                && diagnostic
                    .location
                    .as_ref()
                    .is_some_and(|location| location.function == "helper")
        }));
        let helper = result
            .sources
            .iter()
            .find(|source| source.locator.item_path == "helper")
            .unwrap();
        assert!(helper.src_id.is_none());
    }

    /// 表面2の正常経路: 反復のない単一の `@vtest.src-id` は認識され、
    /// 診断を生じない。
    #[test]
    fn non_test_item_declares_a_permanent_src_id() {
        let root = fixture();
        fs::write(
            root.join("src/lib.rs"),
            "pub fn add(a: i32, b: i32) -> i32 { a + b }\n\n\
             /// @vtest.src-id SRC-HELPER\n\
             pub fn helper() -> i32 { 0 }\n",
        )
        .unwrap();
        let result = scan_project(&root).unwrap();
        let helper = result
            .sources
            .iter()
            .find(|source| source.locator.item_path == "helper")
            .unwrap();
        assert_eq!(
            helper.src_id.as_ref().map(SrcId::as_str),
            Some("SRC-HELPER")
        );
        assert!(!result.diagnostics.iter().any(|diagnostic| {
            diagnostic
                .location
                .as_ref()
                .is_some_and(|location| location.function == "helper")
        }));
    }

    /// 本冊 §5.1手順5・基本仕様§9.2「恒久SRC IDを使用する場合、adapter境界を
    /// 越えてrepository全体で一意でなければならない。同一SRC IDの複数宣言を
    /// 曖昧参照として受理しない」。2件の異なるSource Targetが同じ恒久SRC ID
    /// を宣言した場合はE-SCAN-011とし（本冊:877・901）、どのTestからも
    /// 参照されていなくても索引構築時点で検出する。
    #[test]
    fn colliding_permanent_src_id_across_two_source_targets_is_rejected() {
        let root = fixture();
        fs::write(
            root.join("src/lib.rs"),
            "pub fn add(a: i32, b: i32) -> i32 { a + b }\n\n\
             /// @vtest.src-id SRC-SHARED\n\
             pub fn helper_one() -> i32 { 0 }\n\n\
             /// @vtest.src-id SRC-SHARED\n\
             pub fn helper_two() -> i32 { 1 }\n",
        )
        .unwrap();
        let result = scan_project(&root).unwrap();
        assert!(
            result
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.code == "E-SCAN-011"),
            "diagnostics: {:?}",
            result.diagnostics
        );
    }

    /// 本冊 §6.1.1「Testの宣言target集合は解決後のcanonical Source Target
    /// 単位で一意でなければならない。綴りの異なる複数の宣言が同一の
    /// canonical Source Targetへ解決する場合は重複targetとしてE-SCAN-005と
    /// する」。locator形式の宣言と、同じSource Targetを指すSRC ID形式の
    /// 宣言は綴りが異なるが、解決後は同一canonical Source Targetになる。
    #[test]
    fn locator_and_src_id_target_declarations_resolving_to_the_same_source_target_collide() {
        let root = fixture();
        fs::write(
            root.join("src/lib.rs"),
            "pub fn add(a: i32, b: i32) -> i32 { a + b }\n\n\
             /// @vtest.src-id SRC-HELPER\n\
             pub fn helper() -> i32 { 0 }\n",
        )
        .unwrap();
        fs::write(
            root.join("tests/aliased_target.rs"),
            r#"
/// @vtest.id TEST-ALIASED-TARGET
/// @vtest.covers VO-ADD
/// @vtest.target src/lib.rs::helper
/// @vtest.target SRC-HELPER
/// @vtest.intent declares the same Source Target twice under different spellings
/// @vtest.kind integration-normal
#[test]
fn aliased_target() {}
"#,
        )
        .unwrap();
        let result = scan_project(&root).unwrap();
        assert!(
            result.diagnostics.iter().any(|diagnostic| {
                diagnostic.code == "E-SCAN-005"
                    && diagnostic
                        .location
                        .as_ref()
                        .is_some_and(|location| location.function == "aliased_target")
            }),
            "diagnostics: {:?}",
            result.diagnostics
        );
    }

    /// 本冊:955/959/961（§6.1）: 解決できない target は「対象なし」の
    /// fail-closed 終端状態であり、後段が任意の候補で埋めて「解決済み」を
    /// 偽装してはならない。`@vtest.target` に `path::item-path` へ構文解析
    /// できない値（`::` を含まない自由記述）を与えると、この Test 自身の
    /// locator（`tests/unresolvable_target.rs::declares_unparseable_target`）
    /// で肩代わりして解決済みにする旧挙動があった（fail-open。BLOCKER 3）。
    /// 現在は adapter が sentinel Locator を返し、core の `resolve_targets`
    /// が通常の「0件ヒット」経路として E-SCAN-004 を発行することを断言する。
    #[test]
    fn unparseable_target_locator_is_not_silently_resolved_to_the_test_itself() {
        let root = fixture();
        fs::write(
            root.join("tests/unresolvable_target.rs"),
            r#"
/// @vtest.id TEST-UNRESOLVABLE-TARGET
/// @vtest.covers VO-ADD
/// @vtest.target this is not a locator
/// @vtest.intent declares a target value that cannot be parsed as a locator
#[test]
fn declares_unparseable_target() {}
"#,
        )
        .unwrap();
        let result = scan_project(&root).unwrap();
        let test = result
            .tests
            .iter()
            .find(|test| test.id.as_str() == "TEST-UNRESOLVABLE-TARGET")
            .expect("the Test Entity is still materialized; only its target fails to resolve");
        let TargetRef::Locator(locator) = &test.target else {
            panic!("expected a Locator target, got {:?}", test.target);
        };
        assert_ne!(
            (locator.path.as_str(), locator.item_path.as_str()),
            (
                "tests/unresolvable_target.rs",
                "declares_unparseable_target"
            ),
            "an unresolvable target must not be silently filled in with the Test's own \
             self-referencing locator: {locator:?}"
        );
        assert!(
            result.diagnostics.iter().any(|diagnostic| {
                diagnostic.code == "E-SCAN-004"
                    && diagnostic
                        .location
                        .as_ref()
                        .is_some_and(|location| location.function == "declares_unparseable_target")
            }),
            "diagnostics: {:?}",
            result.diagnostics
        );
    }

    /// 本冊 §4.2「doc comment 内の `@vtest.` を含まない行は自由記述として
    /// 無視する」。
    #[test]
    fn free_text_lines_in_a_doc_comment_are_ignored() {
        let root = fixture();
        fs::write(
            root.join("tests/free_text.rs"),
            r#"
/// This test exercises addition end to end.
/// @vtest.id TEST-FREE-TEXT
/// @vtest.covers VO-ADD
/// See also the design notes in docs/plans.
/// @vtest.target src/lib.rs::add
/// @vtest.intent ignores free-form prose lines interleaved with declarations
#[test]
fn free_text() {}
"#,
        )
        .unwrap();
        let result = scan_project(&root).unwrap();
        assert!(
            !result.has_errors(),
            "diagnostics: {:?}",
            result.diagnostics
        );
        assert!(result
            .tests
            .iter()
            .any(|test| test.id.as_str() == "TEST-FREE-TEXT"));
    }

    /// 本冊 §4.4 / §5.5: `rust-cargo` は追加必須 metadata として
    /// `targets ≥ 1` を要求する。`@vtest.target` を1件も宣言しない Test は
    /// E-SCAN-007 になる。
    #[test]
    fn missing_target_annotation_is_rejected() {
        let root = fixture();
        fs::write(
            root.join("tests/no_target.rs"),
            r#"
/// @vtest.id TEST-NO-TARGET
/// @vtest.covers VO-ADD
/// @vtest.intent requires at least one target
#[test]
fn no_target() {}
"#,
        )
        .unwrap();
        let result = scan_project(&root).unwrap();
        assert!(result.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "E-SCAN-007"
                && diagnostic
                    .location
                    .as_ref()
                    .is_some_and(|location| location.function == "no_target")
        }));
    }

    /// 本冊 §4.4 / §11.1.1: core が中立に要求する必須 metadata（`id` /
    /// `covers ≥ 1`）の欠落も E-SCAN-007 になる。
    #[test]
    fn missing_id_and_covers_annotations_are_rejected() {
        let root = fixture();
        fs::write(
            root.join("tests/no_id_or_covers.rs"),
            r#"
/// @vtest.target src/lib.rs::add
/// @vtest.intent requires an id
#[test]
fn no_id() {}

/// @vtest.id TEST-NO-COVERS
/// @vtest.target src/lib.rs::add
/// @vtest.intent requires at least one covers VO
#[test]
fn no_covers() {}
"#,
        )
        .unwrap();
        let result = scan_project(&root).unwrap();
        assert!(result.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "E-SCAN-007"
                && diagnostic
                    .location
                    .as_ref()
                    .is_some_and(|location| location.function == "no_id")
        }));
        assert!(result.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "E-SCAN-007"
                && diagnostic
                    .location
                    .as_ref()
                    .is_some_and(|location| location.function == "no_covers")
        }));
    }

    /// 本冊:567（§4.4）・基本:412（§12）・別紙C:81（§18.3.1）: `covers` を
    /// 持たない（0 件の）Test は E-SCAN-007 とし `TestDraft` を生成しない
    /// （BLOCKER 2、PR #26 review round 1）。`@vtest.covers` の値自体は
    /// 非空文字列（`,`）だが、カンマ区切りで分割すると VO ID が1件も
    /// 残らない — 旧挙動は E-SCAN-007 を出しつつ `covers: []` の
    /// `TestEntity` を管理対象集合へ混入させていた（fail-open）。
    #[test]
    fn covers_that_reduces_to_zero_vo_ids_is_rejected_and_produces_no_test_entity() {
        let root = fixture();
        fs::write(
            root.join("tests/empty_covers.rs"),
            r#"
/// @vtest.id TEST-EMPTY-COVERS
/// @vtest.covers ,
/// @vtest.target src/lib.rs::add
/// @vtest.intent declares a covers value that is non-empty text but zero VO ids
#[test]
fn empty_covers() {}
"#,
        )
        .unwrap();
        let result = scan_project(&root).unwrap();
        assert!(
            result.diagnostics.iter().any(|diagnostic| {
                diagnostic.code == "E-SCAN-007"
                    && diagnostic
                        .location
                        .as_ref()
                        .is_some_and(|location| location.function == "empty_covers")
            }),
            "diagnostics: {:?}",
            result.diagnostics
        );
        assert!(
            !result
                .tests
                .iter()
                .any(|test| test.id.as_str() == "TEST-EMPTY-COVERS"),
            "a Test with 0 resolved covers VOs must not become a managed Test Entity: {:?}",
            result.tests
        );
    }

    /// 基本仕様:126-134「ツールは形式を強制せず一意性のみを強制する」。
    /// `covers` に `VO-` 接頭辞を持たない ID を指定しても、その ID が実在
    /// する VO を参照していれば拒否されない
    /// （PM 裁定・pr3-decisions.md 裁定7）。
    #[test]
    fn edit_test_covers_does_not_enforce_a_vo_id_prefix() {
        let root = fixture();
        fs::write(
            root.join(".verify/vo/WIDGET-ADD.yaml"),
            valid_vo("WIDGET-ADD", "null"),
        )
        .unwrap();
        let mut set = BTreeMap::new();
        set.insert(
            "covers".to_owned(),
            FormValue::List(vec!["WIDGET-ADD".to_owned()]),
        );
        let result = edit_test(&root, "TEST-ADD", None, &set, None, true);
        assert!(result.is_ok(), "unexpected error: {:?}", result.err());
    }

    #[test]
    fn relation_id_aliases_cannot_duplicate_one_ulid_payload() {
        let root = fixture();
        let payload = new_record_id();
        for id in [payload.clone(), format!("REL-{payload}")] {
            fs::write(
                root.join(format!(".verify/rel/{id}.yaml")),
                format!(
                    "id: {id}\ntype: complements\nfrom: VO-ADD\nto: VO-ADD\ncreated: '2026-01-01'\n"
                ),
            )
            .unwrap();
        }

        let result = scan_project(&root).unwrap();
        let duplicates = result
            .diagnostics
            .iter()
            .filter(|diagnostic| {
                diagnostic.code == "E-SCAN-010" && diagnostic.message.contains("same ULID payload")
            })
            .collect::<Vec<_>>();
        assert_eq!(duplicates.len(), 1, "diagnostics: {:?}", result.diagnostics);
        assert!(duplicates[0].location.is_some());
    }

    #[test]
    fn reports_vo_and_relation_integrity_diagnostics() {
        // 詳細設計 v0.1 §2.1 replaced the predecessor REQ/SPEC layers with
        // canonical doc/VO (本冊:30-60); this test used to also exercise the
        // predecessor SPEC-layer staleness check (`.verify/spec/`,
        // W-SCAN-104) here. That subject survives canonically as DOC +
        // `derives_from` (本冊:1626 §17.1), but no DOC-layer validator exists
        // in this crate yet — its assertion moved, unweakened, to
        // `reports_document_content_hash_staleness` below (`#[ignore]`d
        // until that validator lands). What remains here is the VO-layer
        // and Relation-layer integrity checks (E-SCAN-008/009/010,
        // W-SCAN-102/103, W-STORE-001), which are unaffected by the doc/
        // REQ/SPEC migration.
        let root = fixture();
        fs::write(
            root.join(".verify/vo/VO-MISSING-PARENT.yaml"),
            valid_vo("VO-MISSING-PARENT", "VO-NOT-FOUND"),
        )
        .unwrap();
        fs::write(
            root.join(".verify/vo/VO-PARENT.yaml"),
            valid_vo("VO-PARENT", "null"),
        )
        .unwrap();
        fs::write(
            root.join(".verify/vo/VO-CHILD.yaml"),
            valid_vo("VO-CHILD", "VO-PARENT"),
        )
        .unwrap();
        fs::write(
            root.join(".verify/vo/VO-RENAMED.yaml"),
            valid_vo("VO-DIFFERENT", "null"),
        )
        .unwrap();
        fs::write(
            root.join("tests/parent.rs"),
            r#"
/// @vtest.id TEST-PARENT
/// @vtest.covers VO-PARENT
/// @vtest.target src/lib.rs::add
/// @vtest.intent covers a parent VO
#[test]
fn covers_parent() {}
"#,
        )
        .unwrap();

        let relation_id = new_record_id();
        let relation = root.join(format!(".verify/rel/{relation_id}.yaml"));
        fs::write(
            relation,
            format!(
                "id: {relation_id}\ntype: depends-on\nfrom: ENTITY-NOT-FOUND\nto: VO-ADD\ncreated: '2026-01-01'\n"
            ),
        )
        .unwrap();

        // 詳細設計 v0.1 §3.2: the read-compat `status` field triggers
        // W-STORE-001 by its mere presence (regardless of value), so append
        // it to VO-ADD directly rather than reconstructing approval-derived
        // status comparison in the scanner (removed; see
        // `validate_approval_status`'s doc comment).
        let vo_add_path = root.join(".verify/vo/VO-ADD.yaml");
        let vo_add_text = format!("{}status: draft\n", valid_vo("VO-ADD", "null"));
        fs::write(&vo_add_path, &vo_add_text).unwrap();
        let vo_hash = ContentHash::from_text(&vo_add_text);
        let approval_id = new_record_id();
        fs::write(
            root.join(format!(".verify/approvals/{approval_id}.yaml")),
            format!(
                "id: {approval_id}\nsubject: VO-ADD\nsubject_hash: {vo_hash}\napprover:\n  kind: human\n  id: reviewer\nbasis: []\napproved_at: '2026-01-01'\n"
            ),
        )
        .unwrap();

        let result = scan_project(&root).unwrap();
        let codes = result
            .diagnostics
            .iter()
            .map(|diagnostic| diagnostic.code.as_str())
            .collect::<BTreeSet<_>>();
        assert!(
            codes.contains("E-SCAN-008"),
            "diagnostics: {:?}",
            result.diagnostics
        );
        assert!(
            codes.contains("E-SCAN-009"),
            "diagnostics: {:?}",
            result.diagnostics
        );
        assert!(
            codes.contains("E-SCAN-010"),
            "diagnostics: {:?}",
            result.diagnostics
        );
        assert!(
            codes.contains("W-SCAN-102"),
            "diagnostics: {:?}",
            result.diagnostics
        );
        assert!(
            codes.contains("W-SCAN-103"),
            "diagnostics: {:?}",
            result.diagnostics
        );
        assert!(
            codes.contains("W-STORE-001"),
            "diagnostics: {:?}",
            result.diagnostics
        );
        assert!(
            result
                .diagnostics
                .iter()
                .all(|diagnostic| diagnostic.location.is_some()),
            "every scanner diagnostic must identify its canonical source: {:?}",
            result.diagnostics
        );
    }

    #[test]
    fn reports_document_content_hash_staleness() {
        let root = fixture();
        fs::create_dir_all(root.join("docs")).unwrap();
        fs::write(root.join("docs/spec.md"), "original\n").unwrap();
        let layout = VerifyLayout::new(&root);
        let document = DocumentRecord {
            id: DocumentId::new("DOC-ONE"),
            path: "docs/spec.md".to_owned(),
            content_hash: ContentHash::from_text("original\n"),
            title: None,
            derives_from: Vec::new(),
            registered_at: "2026-01-01T00:00:00Z".to_owned(),
        };
        write_document(&layout, &document).unwrap();
        // Declare DOC-ONE a root so this test observes W-SCAN-104 in
        // isolation, without also tripping E-SCAN-016 (orphan) on a document
        // this test never gave a `derives_from`.
        add_doc_root(&root, "DOC-ONE");
        fs::write(root.join("docs/spec.md"), "changed\n").unwrap();

        let result = scan_project(&root).unwrap();
        assert!(
            result
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.code == "W-SCAN-104"),
            "diagnostics: {:?}",
            result.diagnostics
        );
    }

    /// Appends `id` to `config.yaml`'s `doc.roots` (詳細設計 v0.1 §2.2/§5.6),
    /// keeping every root the fixture already declared (e.g. `DOC-TEST`, see
    /// `write_doc_test_fixture`).
    fn add_doc_root(root: &Path, id: &str) {
        let layout = VerifyLayout::new(root);
        let mut config = load_config(root).unwrap();
        config.doc.roots.push(DocumentId::new(id));
        fs::write(layout.config(), config.to_yaml()).unwrap();
    }

    #[test]
    fn reports_document_derives_from_dangling_reference() {
        let root = fixture();
        let layout = VerifyLayout::new(&root);
        write_document(
            &layout,
            &DocumentRecord {
                id: DocumentId::new("DOC-DANGLING"),
                path: "docs/test.md".to_owned(),
                content_hash: ContentHash::from_text("fixture document\n"),
                title: None,
                derives_from: vec![DerivesFrom {
                    doc: DocumentId::new("DOC-MISSING"),
                    anchor: None,
                    note: None,
                }],
                registered_at: "2026-01-01T00:00:00Z".to_owned(),
            },
        )
        .unwrap();

        let result = scan_project(&root).unwrap();
        let dangling = result
            .diagnostics
            .iter()
            .filter(|diagnostic| {
                diagnostic.code == "E-SCAN-012" && diagnostic.message.contains("DOC-DANGLING")
            })
            .collect::<Vec<_>>();
        assert_eq!(dangling.len(), 1, "diagnostics: {:?}", result.diagnostics);
        assert!(dangling[0].message.contains("DOC-MISSING"));
        assert!(
            !result
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.code == "E-SCAN-016"
                    && diagnostic.message.contains("DOC-DANGLING")),
            "a document with a (dangling) derives_from entry is not orphaned; \
             E-SCAN-012 and E-SCAN-016 must not both fire for it: {:?}",
            result.diagnostics
        );
    }

    #[test]
    fn reports_orphan_document_not_listed_as_root() {
        let root = fixture();
        let layout = VerifyLayout::new(&root);
        write_document(
            &layout,
            &DocumentRecord {
                id: DocumentId::new("DOC-ORPHAN"),
                path: "docs/test.md".to_owned(),
                content_hash: ContentHash::from_text("fixture document\n"),
                title: None,
                derives_from: Vec::new(),
                registered_at: "2026-01-01T00:00:00Z".to_owned(),
            },
        )
        .unwrap();

        let result = scan_project(&root).unwrap();
        let orphan = result
            .diagnostics
            .iter()
            .filter(|diagnostic| {
                diagnostic.code == "E-SCAN-016" && diagnostic.message.contains("DOC-ORPHAN")
            })
            .collect::<Vec<_>>();
        assert_eq!(orphan.len(), 1, "diagnostics: {:?}", result.diagnostics);
        assert!(
            !result.diagnostics.iter().any(|diagnostic| {
                diagnostic.code == "E-SCAN-012" && diagnostic.message.contains("DOC-ORPHAN")
            }),
            "an orphan document with no derives_from entries has nothing to \
             dangle; E-SCAN-012 must not fire for it: {:?}",
            result.diagnostics
        );
    }

    #[test]
    fn document_referenced_by_another_documents_derives_from_is_not_orphaned() {
        let root = fixture();
        let layout = VerifyLayout::new(&root);
        // DOC-UPSTREAM has an empty derives_from and is not in doc.roots, but
        // DOC-DOWNSTREAM derives from it — 別紙C:119-125's second orphan
        // condition ("他のどの document からも derives_from で参照されず")
        // means that incoming reference alone keeps DOC-UPSTREAM connected.
        write_document(
            &layout,
            &DocumentRecord {
                id: DocumentId::new("DOC-UPSTREAM"),
                path: "docs/test.md".to_owned(),
                content_hash: ContentHash::from_text("fixture document\n"),
                title: None,
                derives_from: Vec::new(),
                registered_at: "2026-01-01T00:00:00Z".to_owned(),
            },
        )
        .unwrap();
        write_document(
            &layout,
            &DocumentRecord {
                id: DocumentId::new("DOC-DOWNSTREAM"),
                path: "docs/test.md".to_owned(),
                content_hash: ContentHash::from_text("fixture document\n"),
                title: None,
                derives_from: vec![DerivesFrom {
                    doc: DocumentId::new("DOC-UPSTREAM"),
                    anchor: None,
                    note: None,
                }],
                registered_at: "2026-01-01T00:00:00Z".to_owned(),
            },
        )
        .unwrap();

        let result = scan_project(&root).unwrap();
        assert!(
            !result.diagnostics.iter().any(|diagnostic| {
                diagnostic.code == "E-SCAN-016" && diagnostic.message.contains("DOC-UPSTREAM")
            }),
            "diagnostics: {:?}",
            result.diagnostics
        );
        assert!(
            !result
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.code == "E-SCAN-012"),
            "diagnostics: {:?}",
            result.diagnostics
        );
    }

    #[test]
    fn well_formed_documents_report_no_document_layer_diagnostics() {
        // fixture() already registers DOC-TEST (empty derives_from, listed
        // in doc.roots, content_hash matching docs/test.md) — the baseline
        // this test asserts stays silent under E-SCAN-012/E-SCAN-016/
        // W-SCAN-104. Add one more well-formed, non-root document that
        // derives from it to also exercise a resolving `derives_from`.
        let root = fixture();
        let layout = VerifyLayout::new(&root);
        write_document(
            &layout,
            &DocumentRecord {
                id: DocumentId::new("DOC-CHILD"),
                path: "docs/test.md".to_owned(),
                content_hash: ContentHash::from_text("fixture document\n"),
                title: None,
                derives_from: vec![DerivesFrom {
                    doc: DocumentId::new("DOC-TEST"),
                    anchor: None,
                    note: None,
                }],
                registered_at: "2026-01-01T00:00:00Z".to_owned(),
            },
        )
        .unwrap();

        let result = scan_project(&root).unwrap();
        let document_layer_codes = ["E-SCAN-012", "E-SCAN-016", "W-SCAN-104"];
        assert!(
            !result
                .diagnostics
                .iter()
                .any(|diagnostic| document_layer_codes.contains(&diagnostic.code.as_str())),
            "diagnostics: {:?}",
            result.diagnostics
        );
    }

    #[test]
    fn spec_example_document_yaml_parses_verbatim() {
        // 詳細設計 v0.1 §3.1 (docs/AI並列開発向けテスト検証システム 詳細設計
        // v0.1.md:192-202), verbatim including inline comments. The example
        // is illustrative, not a fixture: DOC-REQ-001 is never registered and
        // "sha256:..." is a placeholder, not a real hash — this asserts scan
        // accepts the shape (no E-SCAN-010) while still, correctly, flagging
        // both as chain-integrity problems rather than silently passing them.
        let yaml = r#"id: DOC-BASIC-001
path: docs/basic-spec.md        # プロジェクト相対パス
content_hash: "sha256:..."      # 登録時の内容ハッシュ（§1.3 document subject）
title: 基本仕様書               # 任意の表示名
derives_from:                   # 上流 document への導出リンク（0件可＝根候補）
  - doc: DOC-REQ-001
    anchor: "§12.3"             # 任意の上流該当箇所（節番号等・空可・非 MISMATCH）
    note: ""                    # 任意の導出理由（空可・非 MISMATCH。基本仕様 §3.4）
registered_at: 2026-08-08T00:00:00Z
"#;

        let root = fixture();
        fs::create_dir_all(root.join("docs")).unwrap();
        fs::write(root.join("docs/basic-spec.md"), "basic spec body\n").unwrap();
        let layout = VerifyLayout::new(&root);
        fs::write(layout.doc_dir().join("DOC-BASIC-001.yaml"), yaml).unwrap();

        let result = scan_project(&root).unwrap();
        assert!(
            !result.diagnostics.iter().any(|diagnostic| {
                diagnostic.code == "E-SCAN-010" && diagnostic.message.contains("DOC-BASIC-001")
            }),
            "the spec's own example record must parse as schema-valid: {:?}",
            result.diagnostics
        );
        assert!(
            result.diagnostics.iter().any(|diagnostic| {
                diagnostic.code == "E-SCAN-012" && diagnostic.message.contains("DOC-BASIC-001")
            }),
            "diagnostics: {:?}",
            result.diagnostics
        );
        assert!(
            result.diagnostics.iter().any(|diagnostic| {
                diagnostic.code == "W-SCAN-104" && diagnostic.message.contains("DOC-BASIC-001")
            }),
            "diagnostics: {:?}",
            result.diagnostics
        );
    }

    /// Overwrites `.verify/vo/VO-ADD.yaml` (the VO `fixture()` already wires
    /// `TEST-ADD` to `covers`) with a custom record, so each E-SCAN-017 test
    /// below only has to vary the `combinations`/`dimensions`/
    /// `coverage_policy` shape under test.
    fn write_vo_add(root: &Path, yaml: &str) {
        fs::write(root.join(".verify/vo/VO-ADD.yaml"), yaml).unwrap();
    }

    /// One diagnostic with `code` whose `message` names `VO-ADD`, i.e. the
    /// diagnostic this test's own mutated record produced (not some other
    /// VO's).
    fn has_diagnostic_for_vo_add(result: &ScanResult, code: &str) -> bool {
        result
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == code && diagnostic.message.contains("VO-ADD"))
    }

    /// Base for every E-SCAN-017 fixture below: `derives_from`/`claim`/
    /// `created`/`updated` never vary across the 別紙C:97-104 conditions, so
    /// each test only supplies the `dimensions:`/`coverage_policy:`/
    /// `combinations:` block that condition exercises.
    fn vo_add_header() -> &'static str {
        "id: VO-ADD\nparent: null\nderives_from:\n  - doc: DOC-TEST\nclaim: claim\nrepresentative_cases: []\ncreated: '2026-01-01'\nupdated: '2026-01-01'\n"
    }

    /// 別紙C:97-104 condition 1a: `explicit` かつ `combinations` 欠落
    /// (missing key entirely, not `null` or `[]` — those are 1b/1c below).
    #[test]
    fn e_scan_017_condition_1a_missing_combinations_under_explicit_policy() {
        let root = fixture();
        write_vo_add(
            &root,
            &format!(
                "{}dimensions:\n  - name: d1\n    partitions: [a, b]\ncoverage_policy: explicit\n",
                vo_add_header()
            ),
        );
        let result = scan_project(&root).unwrap();
        assert!(
            has_diagnostic_for_vo_add(&result, "E-SCAN-017"),
            "diagnostics: {:?}",
            result.diagnostics
        );
    }

    /// 別紙C:97-104 condition 1b: `explicit` かつ `combinations` が `null`.
    #[test]
    fn e_scan_017_condition_1b_null_combinations_under_explicit_policy() {
        let root = fixture();
        write_vo_add(
            &root,
            &format!(
                "{}dimensions:\n  - name: d1\n    partitions: [a, b]\ncoverage_policy: explicit\ncombinations: null\n",
                vo_add_header()
            ),
        );
        let result = scan_project(&root).unwrap();
        assert!(
            has_diagnostic_for_vo_add(&result, "E-SCAN-017"),
            "diagnostics: {:?}",
            result.diagnostics
        );
    }

    /// 別紙C:97-104 condition 1c: `explicit` かつ `combinations` が空 list.
    #[test]
    fn e_scan_017_condition_1c_empty_combinations_under_explicit_policy() {
        let root = fixture();
        write_vo_add(
            &root,
            &format!(
                "{}dimensions:\n  - name: d1\n    partitions: [a, b]\ncoverage_policy: explicit\ncombinations: []\n",
                vo_add_header()
            ),
        );
        let result = scan_project(&root).unwrap();
        assert!(
            has_diagnostic_for_vo_add(&result, "E-SCAN-017"),
            "diagnostics: {:?}",
            result.diagnostics
        );
    }

    /// 別紙C:97-104 condition 2: `explicit` かつ `dimensions` が空.
    #[test]
    fn e_scan_017_condition_2_empty_dimensions_under_explicit_policy() {
        let root = fixture();
        write_vo_add(
            &root,
            &format!(
                "{}dimensions: []\ncoverage_policy: explicit\ncombinations:\n  - d1: a\n",
                vo_add_header()
            ),
        );
        let result = scan_project(&root).unwrap();
        assert!(
            has_diagnostic_for_vo_add(&result, "E-SCAN-017"),
            "diagnostics: {:?}",
            result.diagnostics
        );
    }

    /// 別紙C:97-104 condition 3: `combinations` が空でないのに
    /// `coverage_policy` が `explicit` 以外（ここでは `independent-axes`）.
    #[test]
    fn e_scan_017_condition_3_nonempty_combinations_under_non_explicit_policy() {
        let root = fixture();
        write_vo_add(
            &root,
            &format!(
                "{}dimensions:\n  - name: d1\n    partitions: [a, b]\ncoverage_policy: independent-axes\ncombinations:\n  - d1: a\n",
                vo_add_header()
            ),
        );
        let result = scan_project(&root).unwrap();
        assert!(
            has_diagnostic_for_vo_add(&result, "E-SCAN-017"),
            "diagnostics: {:?}",
            result.diagnostics
        );
    }

    /// 別紙C:97-104 condition 4: entry が未宣言の dimension 名を含む. Two
    /// dimensions are declared (`d1`/`d2`) so the entry's length matches
    /// `dimensions.len()` and this exercises the undeclared-name check
    /// specifically, not the length-mismatch branch condition 6 exercises.
    #[test]
    fn e_scan_017_condition_4_entry_references_an_undeclared_dimension() {
        let root = fixture();
        write_vo_add(
            &root,
            &format!(
                "{}dimensions:\n  - name: d1\n    partitions: [a, b]\n  - name: d2\n    partitions: [x, y]\ncoverage_policy: explicit\ncombinations:\n  - d1: a\n    d3: x\n",
                vo_add_header()
            ),
        );
        let result = scan_project(&root).unwrap();
        assert!(
            has_diagnostic_for_vo_add(&result, "E-SCAN-017"),
            "diagnostics: {:?}",
            result.diagnostics
        );
    }

    /// 別紙C:97-104 condition 5: entry の partition 値が当該 dimension の
    /// `partitions` に無い.
    #[test]
    fn e_scan_017_condition_5_entry_uses_an_undeclared_partition_value() {
        let root = fixture();
        write_vo_add(
            &root,
            &format!(
                "{}dimensions:\n  - name: d1\n    partitions: [a, b]\ncoverage_policy: explicit\ncombinations:\n  - d1: c\n",
                vo_add_header()
            ),
        );
        let result = scan_project(&root).unwrap();
        assert!(
            has_diagnostic_for_vo_add(&result, "E-SCAN-017"),
            "diagnostics: {:?}",
            result.diagnostics
        );
    }

    /// 別紙C:97-104 condition 6 (first half): entry が宣言済み dimension を
    /// 欠く（ここでは `d2` を欠いた1件だけの entry）.
    #[test]
    fn e_scan_017_condition_6_entry_is_missing_a_declared_dimension() {
        let root = fixture();
        write_vo_add(
            &root,
            &format!(
                "{}dimensions:\n  - name: d1\n    partitions: [a, b]\n  - name: d2\n    partitions: [x, y]\ncoverage_policy: explicit\ncombinations:\n  - d1: a\n",
                vo_add_header()
            ),
        );
        let result = scan_project(&root).unwrap();
        assert!(
            has_diagnostic_for_vo_add(&result, "E-SCAN-017"),
            "diagnostics: {:?}",
            result.diagnostics
        );
    }

    /// 別紙C:97-104 condition 6 (second half): entry が同じ dimension 名を
    /// 2回以上持つ。本冊:283（§3.2.1）は前半「宣言済みdimensionを欠く」と
    /// 後半「同じdimension名を2回以上持つ」を同一箇条で並列に扱い、いずれも
    /// E-SCAN-017（VOを保持したままchain_integrity = MISMATCH）に帰着する
    /// （本冊:1625・別紙A:438）。record層（`vtest-store::vo_record_from_
    /// yaml`）は重複が`combinations[]`の内側に閉じている場合、レコードを
    /// 拒否せずそのまま読み取る（`CombinationEntry`が重複キーを losslessly
    /// 保持できるため。BLOCKER 1、PR #26 review round 1/2 — 旧版は
    /// E-SCAN-010が出て`vos`マップからVOが丸ごと消える誤った挙動を
    /// 固定していた）。この scan 層のテストはその結果として E-SCAN-017 が
    /// `VO-ADD` の位置で発行され、record層のE-SCAN-010は発行されないことを
    /// 断言する。record層側の正確な挙動は`vtest_store::canonical::
    /// vo_record_combination_entry_with_a_duplicate_dimension_key_reaches_
    /// scan_as_e_scan_017`が固定する。
    #[test]
    fn e_scan_017_condition_6_duplicate_dimension_key_in_one_entry_is_rejected() {
        let root = fixture();
        write_vo_add(
            &root,
            &format!(
                "{}dimensions:\n  - name: d1\n    partitions: [a, b]\n  - name: d2\n    partitions: [x, y]\ncoverage_policy: explicit\ncombinations:\n  - d1: a\n    d1: b\n    d2: x\n",
                vo_add_header()
            ),
        );
        let result = scan_project(&root).unwrap();
        assert!(
            has_diagnostic_for_vo_add(&result, "E-SCAN-017"),
            "diagnostics: {:?}",
            result.diagnostics
        );
        assert!(
            !has_diagnostic_for_vo_add(&result, "E-SCAN-010"),
            "a duplicate dimension key confined to inside one combinations[] entry must not \
             make the record layer reject the whole VO (BLOCKER 1): {:?}",
            result.diagnostics
        );
    }

    /// 別紙C:97-104 condition 7: 同一の（dimension 名→partition 値）対応を
    /// 持つ entry が2件以上（重複 tuple）。The two entries below both
    /// resolve to the same tuple even though key order differs, matching
    /// §3.2.1's "記述順・map key 順には依存しない" — `BTreeMap`'s `Eq`/`Ord`
    /// already normalize key order, so no special-casing is needed to catch
    /// this as a duplicate.
    #[test]
    fn e_scan_017_condition_7_duplicate_tuple() {
        let root = fixture();
        write_vo_add(
            &root,
            &format!(
                "{}dimensions:\n  - name: d1\n    partitions: [a, b]\n  - name: d2\n    partitions: [x, y]\ncoverage_policy: explicit\ncombinations:\n  - d1: a\n    d2: x\n  - d2: x\n    d1: a\n",
                vo_add_header()
            ),
        );
        let result = scan_project(&root).unwrap();
        assert!(
            has_diagnostic_for_vo_add(&result, "E-SCAN-017"),
            "diagnostics: {:?}",
            result.diagnostics
        );
    }

    /// Positive control: a well-formed `explicit`-policy VO (別紙C:97-104's
    /// own literal example, id/derives_from swapped for this fixture) must
    /// not raise E-SCAN-017. Without this, the eight tests above could all
    /// be trivially satisfied by an `invalid_vo_combinations` that always
    /// returns `Some(..)`.
    #[test]
    fn e_scan_017_well_formed_explicit_combinations_report_no_diagnostic() {
        let root = fixture();
        write_vo_add(
            &root,
            &format!(
                "{}dimensions:\n  - name: operand-sign\n    partitions: [positive, negative]\n  - name: operator\n    partitions: [add, sub, mul, div]\ncoverage_policy: explicit\ncombinations:\n  - operand-sign: positive\n    operator: div\n  - operand-sign: negative\n    operator: div\n",
                vo_add_header()
            ),
        );
        let result = scan_project(&root).unwrap();
        assert!(
            !has_diagnostic_for_vo_add(&result, "E-SCAN-017"),
            "diagnostics: {:?}",
            result.diagnostics
        );
    }
}
