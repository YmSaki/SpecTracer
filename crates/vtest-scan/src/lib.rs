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
    source_target_subject_hash, test_subject_hash, AdapterId, ContentHash, CoveragePolicy,
    Diagnostic, DiscoveredTest, DocumentFile, ManagedTestLink, ScanSummary, SectionNode,
    SentenceNode, SourceFunction, SourceLocation, TargetRef, TestEntity, TestRecord, VoRecord,
};
use vtest_store::{
    is_valid_ulid, load_config, read_approval, read_document_dir, read_document_file,
    read_entity_ids, read_text, read_vo_record, relation_ulid_payload, yaml_scalar_value,
    AdapterConfig, ProjectConfig, RelationRecord, StoreError, VerifyLayout,
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
    /// `config.yaml`'s `adapters[].id` names an adapter the registry cannot
    /// resolve. DS-352（specification.json, statement）「adapter IDの重複、
    /// 同一adapter内のroot重複、未知adapter、無効なadapter設定はusage
    /// error（E-CONFIG-001）とする」、その description が明示的に定義する
    /// 「未知adapter」＝「`config.yaml` の `adapters` が指すadapter IDを
    /// registryで解決できないこと」——これはこの variant が唯一構成される
    /// 条件そのものである。DS-1663（同ファイル、statement）はこの条件を
    /// `E-ADAPTER-001` の「未登録」から明示的に除外する：「`config.yaml`
    /// の `adapters` におけるadapter IDの重複・未知adapterはE-CONFIG-001」。
    /// `E-ADAPTER-001` はDS-1663が残す3条件（registry内部で判明する
    /// adapterの未登録・registryのadapter ID重複・registryの宣言と実装の
    /// 不一致）専用であり、この構成サイトにはいずれも当たらない —
    /// この variant 名は以前 `Adapter`（コード `E-ADAPTER-001`）だったが、
    /// 正本監査（主題H・診断コード全数照合）が上の逐語で誤りと確認したため
    /// 改名した。
    ///
    /// 黙って discovery から除外すること自体が fail-open である点は
    /// 変わらない: 走査対象が黙って減り、テスト0件の正常 scan として
    /// 報告されうる（DS-1292「adapter discoveryの失敗をTest 0件の
    /// 正常scanとして扱わない」、DS-333/REQ-265「adapterが未登録・
    /// 能力不足・解析不能の場合、検証結果を推測で `PASS` へ昇格しては
    /// ならない」——このscan呼び出しはfail-closedにErrを返すので、この
    /// variantの構成自体はその要求を満たす）。
    #[error("[E-CONFIG-001] {message}")]
    UnknownAdapterId { message: String },
    #[error("config error: {0}")]
    Config(String),
}

impl ScanError {
    /// The §17.1 diagnostic code this error carries, when construction sites
    /// have committed to one. `Store`/`Io`/`Config` do not name a diagnostic
    /// code of their own at this variant's construction sites. `Config` in
    /// particular is `adapter_scan_includes`'s empty-`adapters[]` rejection —
    /// レビュー round 2 項目【F】で裁定済み: 仕様上このケースに割り当てられる
    /// コードは無いと判断し、意図的にコード無しのまま残す（`adapter_scan_
    /// includes` のdoc commentを参照）。これは未決の空欄ではなく、根拠に
    /// 基づく確定した設計である。
    pub fn code(&self) -> Option<&'static str> {
        match self {
            Self::Discovery { .. } => Some("E-ADAPTER-002"),
            Self::UnknownAdapterId { .. } => Some("E-CONFIG-001"),
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
    /// 発見された Test construct 集合 `D`（基本仕様§12、基本:408-427）
    /// **全体**。登録 adapter が Test として認識した construct はすべて
    /// ここへ現れる — 管理宣言自体を欠く construct（`rust-cargo` では
    /// `@vtest` annotation を持たない `#[test]`。W-SCAN-101）や必須
    /// metadata を欠く construct（E-SCAN-005/006/007）も、対応する
    /// `DiscoveredTest.managed` が `ManagedTestLink::Missing` を持つ要素
    /// として含まれる（本冊:565「adapter固有のsource declarationを構文
    /// 解析できない場合、adapterは該当Test constructをDiscovered Testと
    /// して返し、対応を`ManagedTestLink::Missing`として診断を付与する」。
    /// Owner裁定1、`pr3-decisions.md`「scanner が観測した Test construct
    /// はすべて保持する」）。
    ///
    /// `tests`（構造上完全な managed Test Entity 集合 `M`）との対応:
    /// `discovered` の要素のうち `managed: ManagedTestLink::One(id)` を
    /// 持つものが、`tests` の中の Test ID `id` を持つ Entity と1件ずつ
    /// 対応する。基本:412「発見された Test 集合を `D`、構造上完全な
    /// managed Test Entity 集合を `M` とする」の通り `M ⊆ D` であり、
    /// `tests.len() <= discovered.len()` が成り立つ（`Missing` construct
    /// の分だけ `discovered` が大きくなる）。
    ///
    /// Test ID の大域的一意性（E-SCAN-002）はこの対応数とは独立した
    /// 整合性条件であり（基本:412「Discovered Test と entity の対応数は
    /// 構造完全性に含めず、独立した整合性条件とする」）、`discovered` の
    /// `One` 要素は衝突していても個別に `managed: ManagedTestLink::
    /// One(自分のid)` を持つ。
    pub discovered: Vec<DiscoveredTest>,
    pub sources: Vec<SourceFunction>,
    pub diagnostics: Vec<Diagnostic>,
}

impl ScanResult {
    pub fn has_errors(&self) -> bool {
        self.diagnostics.iter().any(Diagnostic::is_error)
    }

    /// Test ID で `tests` を引く API。Owner裁定1（pr3-decisions.md）「後段が
    /// 代表1件を推測選択してはならない」を、このメソッドを経由する限り強制
    /// する: この ID を宣言する構文上完全な Test Entity が複数件あれば
    /// （E-SCAN-002、Test ID衝突）`Collided` として全件を返し、`Unique`
    /// （1件）と型として区別する。`Option<&TestEntity>` を返す経路（1件
    /// だけ返して衝突を握り潰す）は提供しない — `TargetResolution`
    /// （`resolve_targets`）と同じ考え方（曖昧な状態から代表候補を選ばず、
    /// この型を経由する限り代表を取り出せない）。
    ///
    /// この強制は API 契約であって型システムによる封じ込めではない:
    /// `tests` フィールド自体は公開されており、呼び出し側が
    /// `scan.tests.iter().find(|t| t.id == id)` のように直接舐めれば
    /// `Collided` の場合でも代表1件を選べてしまう。後段は必ずこの
    /// メソッド（または `TestIdLookup` を経由する同等の経路）を使うこと。
    pub fn tests_by_id<'a>(&'a self, id: &str) -> TestIdLookup<'a> {
        let mut matches = self.tests.iter().filter(|test| test.id.as_str() == id);
        let Some(first) = matches.next() else {
            return TestIdLookup::NotFound;
        };
        let rest: Vec<&TestEntity> = matches.collect();
        if rest.is_empty() {
            TestIdLookup::Unique(first)
        } else {
            let mut all = Vec::with_capacity(rest.len() + 1);
            all.push(first);
            all.extend(rest);
            TestIdLookup::Collided(all)
        }
    }
}

/// [`ScanResult::tests_by_id`] の戻り値。
#[derive(Clone, Debug)]
pub enum TestIdLookup<'a> {
    /// この Test ID に対応する構造上完全な Test Entity が存在しない。
    NotFound,
    /// この Test ID に対応する Test Entity がちょうど1件。
    Unique(&'a TestEntity),
    /// この Test ID が複数の Test construct から宣言されている
    /// （E-SCAN-002、基本:412「Test ID が衝突する entity」）。衝突した
    /// 全件を返す — どの1件を代表とするかはこの型を経由しても選べない。
    Collided(Vec<&'a TestEntity>),
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
    let mut test_drafts: Vec<(AdapterId, vtest_adapter_api::TestDraft)> = Vec::new();
    let mut missing_test_drafts: Vec<(AdapterId, vtest_adapter_api::MissingTestConstruct)> =
        Vec::new();
    let mut source_drafts = Vec::new();
    let mut diagnostics = Vec::new();
    // レビュー round 2 項目【G】: 本冊:584（§5.1 手順2）「登録順ではなく
    // adapter ID順にSourceDiscoveryAdapterを呼び出す」。`config.adapters`
    // の記述順（config.yaml 上の順序）ではなく adapter ID の辞書順で
    // discovery を委譲する。config load 時点で adapter ID の重複は
    // 既に E-CONFIG-001 で拒否されている（`vtest-store`
    // `duplicate_adapter_id_is_rejected`）ため、ここでの並べ替えは
    // 常に一意な全順序になる。Test ID 衝突時も全 draft を保持する
    // （Owner裁定1、pr3-decisions.md）ため「どの draft が生き残るか」は
    // 問題にならないが、`tests` / `discovered` の出力順序自体が
    // config.yaml の記述順に依存しないことは、それ自体が決定性の要件
    // である。
    let mut sorted_adapters = config.adapters.iter().collect::<Vec<_>>();
    sorted_adapters.sort_by(|left, right| left.id.cmp(&right.id));
    for adapter_config in sorted_adapters {
        let Some(adapter) = registry.get(adapter_config.id.as_str()) else {
            // 正本監査（診断コード全数照合、主題H）: 本条件（config.yaml の
            // `adapters[].id` がregistryで解決できない）はDS-352の
            // statementそのもの（「adapter IDの重複、同一adapter内の
            // root重複、未知adapter、無効なadapter設定はusage error
            // （E-CONFIG-001）とする」）で、そのdescriptionが「未知adapter」
            // を明示的にこの条件と定義し、`DS-1663`の「未登録」から除く旨も
            // 明記する。DS-1663のstatement自体も同じ除外を逆方向から明記する
            // （「`config.yaml` の `adapters` におけるadapter IDの重複・
            // 未知adapterはE-CONFIG-001」）。E-ADAPTER-001はDS-1663が残す
            // 3条件（registry内部で判明するadapterの未登録・registryの
            // adapter ID重複・registryの宣言と実装の不一致）専用であり、
            // ここには当たらない（以前はE-ADAPTER-001を返していたが、
            // 上記の逐語で誤りと確認して修正した — ScanError::UnknownAdapterId
            // 自身のdoc commentも参照）。
            let known_ids = registry.ids().collect::<BTreeSet<_>>();
            let known_list = if known_ids.is_empty() {
                "(none registered)".to_owned()
            } else {
                known_ids.into_iter().collect::<Vec<_>>().join(", ")
            };
            return Err(ScanError::UnknownAdapterId {
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
        // `DiscoveredTest.adapter`（本冊:788-801）のために、この
        // discovery batch がどの adapter から返ったかを draft へ束ねる。
        // `TestDraft` 自体は adapter を名乗らない（`vtest-adapter-api`
        // のdoc comment参照）ため、呼び出し元であるここが唯一の出所を
        // 知っている。
        let adapter_id = AdapterId::new(adapter_config.id.clone());
        test_drafts.extend(
            outcome
                .tests
                .into_iter()
                .map(|draft| (adapter_id.clone(), draft)),
        );
        // `DiscoveredTest.adapter` は `Missing` construct についても必須
        // field（本冊:788-801）。理由は上と同じ — `MissingTestConstruct`
        // 自体は adapter を名乗らない（`vtest-adapter-api` のdoc comment
        // 参照）。
        missing_test_drafts.extend(
            outcome
                .missing_tests
                .into_iter()
                .map(|draft| (adapter_id.clone(), draft)),
        );
        source_drafts.extend(outcome.sources);
    }

    let sources = source_drafts
        .into_iter()
        .map(|draft| SourceFunction {
            // DES-083（本冊:88）: Source Target hash は canonical Target
            // Reference（`draft.locator`）と adapter が返す implementation
            // construct bytes の両方を束縛する。`draft.locator` を先に
            // borrow してから同じ式内で move するため、struct literal の
            // field 順は宣言順ではなく borrow が先に来る順にしている。
            content_hash: source_target_subject_hash(&draft.locator, &draft.construct_text),
            locator: draft.locator,
            src_id: draft.src_id,
            location: draft.location,
        })
        .collect::<Vec<_>>();

    let (tests, discovered, collision_diagnostics) =
        materialize_tests(test_drafts, missing_test_drafts);
    diagnostics.extend(collision_diagnostics);
    diagnostics.extend(check_vo_references(&tests, &vo_ids));
    diagnostics.extend(resolve_targets(&tests, &sources));

    let mut result = ScanResult {
        summary: ScanSummary {
            files: files as u64,
            tests: tests.len() as u64,
            sources: sources.len() as u64,
        },
        tests,
        discovered,
        sources,
        diagnostics,
    };
    result.diagnostics.extend(record_diagnostics(
        root,
        &entity_ids,
        &result.tests,
        &result.sources,
    )?);
    Ok(result)
}

/// 本冊:571「VO参照の解決とTest IDの大局的一意性はadapterではなくcoreが
/// 検査する」: adapter は同じ Test ID を宣言する複数 draft をそのまま返す
/// ことがある。
///
/// Owner裁定1（pr3-decisions.md）「Test ID が衝突した場合、先勝ちで1件を
/// 残して他を捨てることを禁止する」: scanner が観測した Test construct は
/// すべて `TestEntity` へ具体化して保持する（基本:412「`M` は…Test ID が
/// 衝突する entity も含む」）。Test ID 衝突は診断（E-SCAN-002）として
/// **衝突した全 construct へ対称に**発行するだけで、entity 集合からは
/// 何も落とさない — 以前の実装は2件目以降を `continue` で捨てており、
/// `ScanResult.tests` / `summary.tests` が実際に発見した件数と一致しない
/// fail-open だった。
///
/// 同時に `DiscoveredTest`（本冊:788-801）を1 draft あたり1件生成する
/// （`ScanResult::discovered`。集合 `D` 全体になった旨は `discovered` の
/// doc comment参照）。各 `DiscoveredTest.managed` は衝突の有無に関わらず
/// 個別に `ManagedTestLink::One(自分のid)` を持つ（本冊:804「解決不能な
/// coversを持つdraftもcore materialization後のmanaged entity集合に保持
/// され、対応するobservationはManagedTestLink::One(id)を持つ」と同じ理由 —
/// `ManagedTestLink::Multiple` は同一 construct から複数 draft が生じる
/// 別の状態を表し、Test ID の大域的衝突を表す variant ではない。
/// `ManagedTestLink` のdoc comment参照）。
///
/// `missing_drafts`（adapterが管理宣言または必須metadataの欠落により
/// `TestDraft` へ具体化できなかったTest construct。`vtest_adapter_api::
/// MissingTestConstruct` のdoc comment参照）は `TestEntity` を生成せず
/// （`M` に属さない）、`ManagedTestLink::Missing` を持つ `DiscoveredTest`
/// だけを生成する（本冊:565「adapter固有のsource declarationを構文解析
/// できない場合、adapterは該当Test constructをDiscovered Testとして返し、
/// 対応を`ManagedTestLink::Missing`として診断を付与する」）。診断そのもの
/// （W-SCAN-101 / E-SCAN-005/006/007）は adapter が既に発行済みであり、
/// ここでは追加の診断を発行しない — construct を `D` に反映するだけである。
fn materialize_tests(
    drafts: Vec<(AdapterId, vtest_adapter_api::TestDraft)>,
    missing_drafts: Vec<(AdapterId, vtest_adapter_api::MissingTestConstruct)>,
) -> (Vec<TestEntity>, Vec<DiscoveredTest>, Vec<Diagnostic>) {
    let mut tests = Vec::with_capacity(drafts.len());
    let mut discovered = Vec::with_capacity(drafts.len() + missing_drafts.len());
    let mut locations_by_id: BTreeMap<String, Vec<SourceLocation>> = BTreeMap::new();

    for (adapter, draft) in drafts {
        locations_by_id
            .entry(draft.id.as_str().to_owned())
            .or_default()
            .push(draft.location.clone());

        // `DiscoveredTest.content_hash` stays a construct-only hash — it
        // exists for every observed construct, including `missing_drafts`
        // below (which have no `TestRecord`), so it cannot depend on
        // metadata. Only `TestEntity.content_hash` is the Test subject hash
        // (§1.3, 本冊:87): it binds adapter ID, canonical metadata, Source
        // Location (excluding byte_range), ExecutionDescriptor, and the
        // normalized construct bytes, so a metadata-only edit (e.g.
        // `@vtest.covers`) changes it even though the construct bytes do
        // not (別紙C:35).
        //
        // 開示（PR34 の ContentHash::from_text 全数調査より）: specification.json
        // には `DiscoveredTest.content_hash: ContentHash` という field 型の
        // 宣言（`DES-339`）はあるが、この field 自体が何を束縛するかを
        // 定める subject hash 規則は見つからない — §1.3 が名付ける
        // domain-separated hash は `vtest:test-subject:v1` /
        // `vtest:target-subject:v1` / `vtest:document-subject:v1` /
        // `vtest:record-subject:v1`（VO）/ `vtest:execution-state:v1` の
        // 5つで、"DiscoveredTest" 用の domain は無い（`DES-335`「coreは
        // range・bytes対応を検証し、§1.3でhashを計算してから `TestEntity`、
        // `SourceTarget` および `DiscoveredTest` を具体化する」は3型まとめて
        // 言うだけで、DiscoveredTest固有の束縛対象までは述べない）。したがって
        // 上の「construct-only」という設計は上記の構造的理由（`missing_drafts`
        // に `TestRecord` が無い）からの導出であり、明文の引用ではない。
        let construct_hash = ContentHash::from_text(&draft.construct_text);

        let metadata = TestRecord {
            id: draft.id.clone(),
            covers: draft.covers.clone(),
            targets: draft.targets.clone(),
            intent: draft.intent.clone(),
            input: draft.input.clone(),
            expect: draft.expect.clone(),
            kind: draft.kind.clone(),
            cases: draft.cases.clone(),
            related: draft.related.clone(),
        };
        let subject_hash = test_subject_hash(
            &adapter,
            &metadata,
            &draft.location,
            &draft.execution,
            &draft.construct_text,
        );

        discovered.push(DiscoveredTest {
            adapter,
            location: draft.location.clone(),
            content_hash: construct_hash,
            managed: ManagedTestLink::One(draft.id.clone()),
        });
        tests.push(TestEntity {
            id: draft.id,
            covers: draft.covers,
            targets: draft.targets,
            intent: draft.intent,
            input: draft.input,
            expect: draft.expect,
            kind: draft.kind,
            cases: draft.cases,
            related: draft.related,
            location: draft.location,
            content_hash: subject_hash,
            execution: draft.execution,
        });
    }

    for (adapter, draft) in missing_drafts {
        discovered.push(DiscoveredTest {
            adapter,
            location: draft.location,
            content_hash: ContentHash::from_text(&draft.construct_text),
            managed: ManagedTestLink::Missing,
        });
    }

    // Test ID の大域的一意性検査（E-SCAN-002）。基本:412「M は…Test ID が
    // 衝突する entity も含む」の通り、上のループで既に全 entity/discovered
    // を保持済み — ここでは診断を発行するだけで、集合には何も触れない。
    // 衝突した construct 全件（先発・後発の区別なく対称に）へ、その
    // construct 自身の location で1件ずつ発行する。`locations_by_id` は
    // `drafts`（構文上有効な Test ID を持つ construct）だけから構築して
    // おり、`missing_drafts` はここに加えない — `ManagedTestLink::Missing`
    // は `Missing` variant 自体が `TestId` を運ばない型（本冊:796-800）
    // であり、E-SCAN-007 の4経路（id・covers・intent 欠落、covers split後
    // 0件。target 欠落は含まない — DS-1666 / ROOT-049 により `targets` の
    // 宣言は Test 成立性の必須条件ではなく、空値も core の target 解決
    // （E-SCAN-004）へ素通しする）のうち id 以外の3経路は構文上有効な
    // `@vtest.id` を持つ construct でも起こりうる。したがって「`Missing` は Test ID
    // を持たない」は誤りで、正しくは「`ManagedTestLink::Missing` という
    // 型が Test ID を運ばないため、たとえ元の宣言に `@vtest.id` の文字列
    // があっても core 側にはこの検査で比較できる `TestId` が存在しない」
    // という型レベルの理由である。**注意（未検査のまま残る論点）**:
    // `Missing` construct の宣言 `@vtest.id`（例えば covers 欠落で
    // Missing になった construct の `@vtest.id TEST-X`）が、別の構造上
    // 完全な construct が宣言する `TestId("TEST-X")` と衝突していても、
    // この検査は検出しない — `Missing` は id 文字列を保持しない型なので
    // 比較しようがない。仕様（基本:412 の `M`、本冊:898 の E-SCAN-002
    // 列挙）はいずれも `M`（構造上完全な managed Test Entity 集合）内の
    // 衝突を対象にしており、`M` に属さない `Missing` construct の宣言
    // 文字列同士・`Missing` と `M` の間の衝突までは述べていない。仕様
    // 沈黙であり、本 PR ではこの外挿を行わない。
    let mut diagnostics = Vec::new();
    for (id, locations) in &locations_by_id {
        if locations.len() > 1 {
            for location in locations {
                diagnostics.push(
                    Diagnostic::error(
                        "E-SCAN-002",
                        format!(
                            "Test ID `{id}` is declared by {} Test constructs",
                            locations.len()
                        ),
                    )
                    .with_location(location.clone()),
                );
            }
        }
    }

    (tests, discovered, diagnostics)
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

/// canonical locator の索引キー。`(adapter, opaque value)` の完全一致だけ
/// で比較し、`value`の内部構文は解釈しない（本冊:522「coreがpath、module、
/// symbol種別を分解しない」）。
type LocatorKey = (AdapterId, String);

fn locator_key(locator: &vtest_model::Locator) -> LocatorKey {
    (locator.adapter.clone(), locator.value.clone())
}

/// 1件の宣言 `TargetRef` を canonical Source Target へ解決した結果。
///
/// 本冊 §6.1「解決結果は「解決済み」「対象なし」「曖昧」の3状態を区別し、
/// 曖昧はfail-closedな終端状態とする」の3語に対応する。本冊にはこの状態を
/// 表すRust型定義そのものは無く（`pr3-ruling-spec.md` §2.4・2.7）、型の形は
/// 実装裁量。ここでは仕様が定める3状態の意味だけを表し、それ以上の情報
/// （候補一覧・canonical Source Targetの型など）は持ち込まない —
/// `Ambiguous`・`NotFound`はどんな値も運ばず、`Resolved`が運ぶのは一致した
/// 1件のcanonical locator keyだけである。この型を経由しない限り、呼び出し
/// 側は「曖昧」「対象なし」の宣言から代表候補を取り出せない（本冊:959
/// 「曖昧な解決から代表候補を選ばず…後段へ候補を1件も引き渡さない」を型で
/// 強制する）。
#[derive(Clone, Debug, Eq, PartialEq)]
enum TargetResolution {
    /// 解決済み。
    Resolved(LocatorKey),
    /// 対象なし（一致するSource Targetが0件）。
    NotFound,
    /// 曖昧（一致するSource Targetが2件以上）。fail-closedな終端状態。
    Ambiguous,
}

/// Target Reference 解決（本冊 §6.1・§6.1.1、E-SCAN-004/005/011）。opaque
/// locator の完全一致検索は adapter が構築した source index（`sources`）に
/// 対して行うだけで、構文自体は解釈しない。この解決は core の単一経路が
/// 所有する（本冊:990-1005 §6.3 冒頭「この解決はcoreの単一経路が所有し」）。
fn resolve_targets(tests: &[TestEntity], sources: &[SourceFunction]) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();
    // §6.2 のSRC索引: locatorの完全一致検索に使う。1件だけ一致すれば
    // 解決、0件または複数件は解決失敗（E-SCAN-004。本冊:979-988）。
    let mut locators = BTreeMap::<LocatorKey, usize>::new();
    // 恒久SRC IDごとの宣言元canonical locator一覧。1件なら`SRC索引`
    // （本冊:571・§6.1）を経て一意にcanonical locatorへ解決する。2件以上は
    // 恒久SRC IDのrepository全体一意性違反であり、E-SCAN-011とする
    // （基本仕様§9.2「同一SRC IDの複数宣言を曖昧参照として受理しない」、
    // 本冊§5.1手順5「adapter間を含む...SRC ID衝突...を検査する」）。この
    // 検査は当該SRC IDを参照するTestの有無に関わらず、索引構築時点で行う。
    let mut src_id_locators = BTreeMap::<String, Vec<LocatorKey>>::new();
    let mut src_id_first_location = BTreeMap::<String, SourceLocation>::new();
    for source in sources {
        let key = locator_key(&source.locator);
        *locators.entry(key.clone()).or_default() += 1;
        if let Some(src_id) = &source.src_id {
            src_id_locators
                .entry(src_id.as_str().to_owned())
                .or_default()
                .push(key);
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
        let mut resolved_canonical = Vec::<(String, LocatorKey)>::new();
        for target in &test.targets {
            match target {
                TargetRef::Locator(locator) => {
                    let key = locator_key(locator);
                    let resolution = match locators.get(&key).copied().unwrap_or(0) {
                        0 => TargetResolution::NotFound,
                        1 => TargetResolution::Resolved(key),
                        _ => TargetResolution::Ambiguous,
                    };
                    match resolution {
                        TargetResolution::Resolved(canonical) => {
                            resolved_canonical.push((locator.value.clone(), canonical));
                        }
                        TargetResolution::NotFound | TargetResolution::Ambiguous => {
                            // 要確認C（PR #26 review round 2）: メッセージに
                            // 不正だった宣言値（`locator.value`）自体を含め
                            // る — d4c1522でadapter側の発行をやめてcoreへ
                            // 一本化した際、core側のメッセージから値が落ち
                            // ていた。判定ロジック（`resolved_canonical`へ
                            // の非採用）は変えない。「対象なし」と「曖昧」
                            // はどちらもE-SCAN-004（本冊:954-961「解決が
                            // 0件または複数候補で一意に定まらない場合は
                            // E-SCAN-004とし」）。
                            diagnostics.push(
                                Diagnostic::error(
                                    "E-SCAN-004",
                                    format!(
                                        "test `{}` target `{}` cannot be resolved",
                                        test.id, locator.value
                                    ),
                                )
                                .with_location(test.location.clone()),
                            );
                        }
                    }
                }
                TargetRef::SrcId(src_id) => {
                    let resolution = match src_id_locators.get(src_id.as_str()).map(Vec::as_slice) {
                        Some([single]) => TargetResolution::Resolved(single.clone()),
                        Some(_) => TargetResolution::Ambiguous,
                        None => TargetResolution::NotFound,
                    };
                    match resolution {
                        TargetResolution::Resolved(canonical) => {
                            resolved_canonical.push((src_id.as_str().to_owned(), canonical));
                        }
                        TargetResolution::Ambiguous => {
                            // 恒久SRC IDが衝突している。E-SCAN-011は索引
                            // 構築時に既に発行済みであり、いずれのSource
                            // Targetも選択しない（本冊:901「E-SCAN-011が
                            // あるSRC ID参照は曖昧なため、関係するtarget
                            // 解決をMISMATCHとし、いずれのSource Targetも
                            // 選択しない」）。同じ衝突を二重にE-SCAN-004
                            // として報告しない。
                        }
                        TargetResolution::NotFound => {
                            // 要確認C: 同上、SRC ID参照側にも同じ修正を
                            // 揃える。
                            diagnostics.push(
                                Diagnostic::error(
                                    "E-SCAN-004",
                                    format!(
                                        "test `{}` target `{}` cannot be resolved",
                                        test.id,
                                        src_id.as_str()
                                    ),
                                )
                                .with_location(test.location.clone()),
                            );
                        }
                    }
                }
            }
        }
        let mut seen_canonical = BTreeMap::<LocatorKey, String>::new();
        for (spelling, canonical) in resolved_canonical {
            match seen_canonical.get(&canonical) {
                Some(previous_spelling) if *previous_spelling != spelling => {
                    diagnostics.push(
                        Diagnostic::error(
                            "E-SCAN-005",
                            format!(
                                "test `{}` declares multiple targets (`{previous_spelling}`, `{spelling}`) that resolve to the same Source Target `{}::{}`",
                                test.id, canonical.0, canonical.1
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
/// `operations.rs` to scope non-scan operations to the configured include
/// paths.
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
/// rejected here as a config error rather than silently scanning zero files.
///
/// レビュー round 2 項目【F】の裁定: この拒否自体に割り当てられる仕様上の
/// 診断コードは無い。本冊:158「adapter IDの重複、同一adapter内のroot重複、
/// 未知adapter、無効なadapter設定はusage error（E-CONFIG-001）とする」の
/// 「無効なadapter設定」は、この一文の他3項目と同じく非空 `adapters` list
/// 内の1 entryの妥当性を問う並列項目であり（重複・root重複・未知adapterは
/// いずれも「listに何が入っているか」の話）、「listが空である」という
/// list全体の不在は同じ読みに当てはまらない。したがってこの関数の戻り値
/// （`Err(String)`）はどのコードにも割り当てず、呼び出し元に委ねる:
/// `scan_project_with_config`（コードなしの `ScanError::Config`）と、
/// `operations.rs` の `validate_rust_file` / `validate_enum_variant`
/// （それぞれが属する Structured Operation 候補検証の失敗コード
/// E-OP-001、本冊:1641・別紙A:539/541）。両者のコードの違いは呼び出し
/// 文脈（scan全体の起動拒否 vs. Structured Operationの入力検証）が異なる
/// ためであり、レビューが指摘した「同じ条件に不整合な2つのコード」では
/// なくなった — 一方は意図的にコード無し、他方はその文脈の既存コード。
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
    // A leading "." (from the common `roots: ["."]` default) is preserved
    // as an explicit `CurDir` component by `Path` (docs: normalized away
    // only when *not* the first component), which would make
    // `Path::starts_with` fail against a bare relative path like
    // "src/lib.rs". Strip it so callers can compare against
    // project-relative paths directly.
    fn strip_curdir(path: PathBuf) -> PathBuf {
        path.components()
            .filter(|component| !matches!(component, Component::CurDir))
            .collect()
    }

    let mut includes = Vec::new();
    for adapter_root in &adapter.roots {
        match &adapter.scan.include {
            Some(patterns) => {
                for include in patterns {
                    includes.push(strip_curdir(Path::new(adapter_root).join(include)));
                }
            }
            None => {
                // DS-349: "`config.yaml` の各adapterの `scan` 設定の
                // `include` はテストコード走査パスであり、省略時は
                // ワークスペース全体を対象とする". An omitted `include`
                // is not "scan nothing" or "scan the `default_for`
                // literal (`src`/`tests`/`crates`)" — it is this
                // adapter's own root, walked whole, so every file under
                // it is in scope.
                includes.push(strip_curdir(Path::new(adapter_root).to_path_buf()));
            }
        }
    }
    includes
}

fn record_diagnostics(
    root: &Path,
    entity_ids: &[Vec<String>; 2],
    tests: &[TestEntity],
    sources: &[SourceFunction],
) -> Result<Vec<Diagnostic>, ScanError> {
    let layout = VerifyLayout::new(root);
    let mut diagnostics = Vec::new();
    let mut known_ids = BTreeSet::new();
    // `entity_ids[0]` is `.verify/doc/*.json` file *names* (DES-585/586), not
    // an entity id — BD-330/DES-585 state the upstream document *file* carries
    // no field that identifies it ("上流文書のファイルは、当該文書を識別する
    // fieldを持たない"); the entity id a `derives_from`/relation edge resolves
    // to is the *node* id inside that file (BD-318, DS-1660). So `known_ids`
    // is seeded here from `entity_ids[1]` (VO ids) only; the file-name slot is
    // not folded in, and the corpus-wide node-id index computed below by
    // `validate_document_nodes` is merged in afterward instead.
    known_ids.extend(entity_ids[1].iter().cloned());
    known_ids.extend(tests.iter().map(|test| test.id.as_str().to_owned()));
    for source in sources {
        known_ids.insert(source.locator.value.clone());
        if let Some(src_id) = &source.src_id {
            known_ids.insert(src_id.as_str().to_owned());
        }
    }

    // 詳細設計 v0.1 §2.1 replaces the predecessor spec/req layers with doc/
    // (本冊:30-60, vtest-store's `init_project` no longer creates
    // `.verify/spec` or `.verify/req`). `entity_ids` therefore carries only
    // [doc, vo] (`vtest_store::read_entity_ids`) — there is no third (REQ)
    // slot to validate, and REQ has no canonical counterpart at all, so its
    // validation is removed outright rather than repointed. `entity_ids[0]`
    // is now `.verify/doc/*.json` file *names* (DES-585/586), each a full
    // upstream `DocumentFile` tree of nodes, not a single flat record.
    // DS-784 / DS-786 make the evaluation input「`.verify/`配下の正典ファイル
    // 集合」— the file *set* under the directory, not the subset a reader
    // happens to recognise. An entry in `.verify/doc/` that is not a document
    // file must therefore be reported, not silently filtered out: dropping it
    // makes it invisible to all four checks, so a document left behind in an
    // unread format would simply not exist as far as verification is
    // concerned. DS-1676 covers this as schema non-conformance
    // （「レコードのid / ファイル名 / schema不一致」）= E-SCAN-010, error.
    //
    // The `.gitkeep` placeholder `init_project` writes into every record
    // directory is part of the layout, not a document, and is excluded by
    // `read_document_dir` before this point.
    for unaccounted in read_document_dir(&layout.doc_dir())
        .map(|listing| listing.unaccounted)
        .unwrap_or_default()
    {
        let record_path = record_relative_path(&layout.root, &layout.doc_dir().join(&unaccounted));
        diagnostics.push(Diagnostic::error(
            "E-SCAN-010",
            format!(
                "{unaccounted} in .verify/doc/ is not an upstream document file and is not \
                 accounted for by any check ({record_path})"
            ),
        ));
    }
    let document_node_ids = validate_document_nodes(&layout, &entity_ids[0], &mut diagnostics);
    // DS-425/DS-429/DS-543: a relation's `from`/`to` is an arbitrary entity
    // id, and its existence is what E-SCAN-009 checks — the entity id space
    // for the document layer is the node-id index just built, not the file
    // names `entity_ids[0]` holds. Merge it in before `validate_relations`
    // resolves anything against `known_ids`.
    known_ids.extend(document_node_ids.iter().cloned());

    let mut vos = BTreeMap::new();
    for id in &entity_ids[1] {
        if let Some(record) = validate_vo_record(&layout, id, &mut diagnostics) {
            vos.insert(id.clone(), record);
        }
    }
    validate_vo_document_references(&layout, &vos, &document_node_ids, &mut diagnostics);

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

    validate_relations(&layout, &known_ids, &mut diagnostics)?;
    validate_vo_warnings(&layout, &vos, tests, &mut diagnostics);
    validate_approval_status(&layout, &vos, &mut diagnostics)?;
    Ok(diagnostics)
}

/// Reads every upstream document file `.verify/doc/<name>.json` (DES-585/586)
/// and checks the document-node layer of chain_integrity/orphan_detection:
///
/// - E-SCAN-012 (DS-546/本冊:878): each node's own `derives_from` entries
///   must resolve to a node id that exists somewhere in the corpus (any
///   document — DS-546 does not confine resolution to the same file).
/// - E-SCAN-016 (DS-1647/DS-1650, §5.6): every node except a `root`-layer
///   one must have a non-empty *effective* upstream — its own `derives_from`
///   edges, unioned with every ancestor section's edges within the same
///   document file ("自分の辺 ∪ 先祖の辺"). DS-1646 confirms `root`-layer
///   exclusion is structural (RootNode carries no `derives_from` field at
///   all) and that there is no config-based exclusion to honor instead
///   (`doc.roots` no longer exists — DS-1646: "設定による除外指定は
///   持たない").
///
/// A whole document file failing to parse (`read_document_file`'s
/// `StoreError`) is reported as `E-SCAN-010` and that file is skipped,
/// exactly like `validate_vo_record` (below) already does for a malformed
/// VO record — it is not propagated via `?` to abort the whole scan.
/// BD-320 calls the upstream document file itself "上流文書のレコード", so
/// DS-1645's "レコードのid / ファイル名 / schema不一致…を意味する" applies
/// to it exactly as it does to a VO record; `document_file_from_json`
/// already runs every schema-intrinsic check DS-1645 requires (a prior
/// version of this comment claimed no diagnostic code covers this — that
/// was wrong: DS-1645/E-SCAN-010 does, via BD-320's own vocabulary). Unlike
/// the predecessor per-record reader, a `DocumentFile` carries no internal
/// id/filename pair for an "id / ファイル名不一致" condition to apply to
/// (BD-330/DES-585: the file itself carries no identifying field) — but
/// DS-1645's other two branches (schema不一致, logical record ID重複) still
/// apply, and this crate has no third code to reach for, so every
/// `read_document_file` failure reported here uses E-SCAN-010, matching
/// `validate_vo_record`'s own precedent for folding a record read failure
/// (including a raw I/O failure) into that same code rather than aborting.
///
/// Aborting the whole scan on one malformed document file was a real
/// defect, not only a wrong code: `record_diagnostics` calls this function
/// first, before the VO/relation/parent-graph/approval checks, so an `Err`
/// here discarded every other diagnostic the scan would otherwise have
/// produced, and the resulting `ScanError::Store` carries no diagnostic
/// code at all (`ScanError::code`'s doc comment documents that as
/// deliberate only for `Store`/`Io`/`Config`'s *other* uses, not this one).
///
/// Returns every node id found, across every document that parsed, for
/// `validate_vo_document_references`'s DS-1660 check (a VO's own
/// `derives_from[].doc` names an upstream *node* id, not a document file
/// name). A node defined only inside a skipped (malformed) file is absent
/// from that set — a `derives_from` entry elsewhere pointing at such a node
/// will report E-SCAN-012 as "missing", a disclosed consequence of skipping
/// the file rather than aborting, not a separate defect.
fn validate_document_nodes(
    layout: &VerifyLayout,
    document_names: &[String],
    diagnostics: &mut Vec<Diagnostic>,
) -> BTreeSet<String> {
    let mut files = Vec::with_capacity(document_names.len());
    for name in document_names {
        match read_document_file(layout, name) {
            Ok(file) => files.push((name.clone(), file)),
            Err(error) => {
                let record_path = document_node_record_path(layout, name);
                diagnostics.push(Diagnostic::error(
                    "E-SCAN-010",
                    format!("document {name} has an invalid record: {error} ({record_path})"),
                ));
            }
        }
    }

    // First pass: index every node id across every document. E-SCAN-012
    // resolution must see the whole corpus before any single-file walk
    // checks an edge, since a node in document A may cite a node defined
    // in document B.
    //
    // A node id occurring more than once (within one file, or across two
    // files) is a real DS-053 condition ("IDの一意性はスキャン時に全数検査
    // する", derives_from REQ-055/REQ-155/REQ-156) and DS-054 assigns it a
    // state ("ID衝突は `chain_integrity` の非 `PASS`（`MISMATCH`）とする").
    // The diagnostic code has since been settled upstream (superseding the
    // account this comment previously gave of an unresolved DS-897/DS-536
    // conflict): DS-1675 confines E-SCAN-002 to Test ID collisions and
    // assigns every other `.verify/` record id collision — including an
    // upstream document node's own `id` — to E-SCAN-010, and DS-1677/
    // DS-1676 spell out the document-node case by name. A colliding id is
    // excluded from `known_ids` below rather than resolved: per DS-1677,
    // "当該idを参照するderives_fromはいずれの候補も解決先として選ばず" — no
    // candidate wins, so a `derives_from` edge pointing at a colliding id
    // reports E-SCAN-012 (unresolved) instead of appearing to resolve to an
    // arbitrary one of the colliding nodes.
    let mut occurrences: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for (name, file) in &files {
        index_document_ids(file, name, &mut occurrences);
    }

    let mut known_ids = BTreeSet::new();
    for (id, occurring_in) in &occurrences {
        if occurring_in.len() > 1 {
            let mut locations = occurring_in.clone();
            locations.sort();
            locations.dedup();
            let record_path = document_node_record_path(layout, &occurring_in[0]);
            diagnostics.push(Diagnostic::error(
                "E-SCAN-010",
                format!(
                    "document node id {id} occurs more than once, in: {} ({record_path})",
                    locations.join(", ")
                ),
            ));
        } else {
            known_ids.insert(id.clone());
        }
    }

    for (name, file) in &files {
        for node in &file.request {
            check_sentence_node(layout, name, node, false, &known_ids, diagnostics);
        }
        for section in &file.require {
            check_section_node(layout, name, section, false, &known_ids, diagnostics);
        }
        for section in &file.spec {
            check_section_node(layout, name, section, false, &known_ids, diagnostics);
        }
        for section in &file.detailed_spec {
            check_section_node(layout, name, section, false, &known_ids, diagnostics);
        }
        for section in &file.basic_design {
            check_section_node(layout, name, section, false, &known_ids, diagnostics);
        }
        for section in &file.design {
            check_section_node(layout, name, section, false, &known_ids, diagnostics);
        }
        // `file.root` nodes are excluded from both checks by construction:
        // `RootNode` has no `derives_from` field (nothing for E-SCAN-012 to
        // resolve) and DS-1647 excludes the `root` layer from
        // orphan_detection outright.
    }

    known_ids
}

fn index_document_ids(
    file: &DocumentFile,
    name: &str,
    occurrences: &mut BTreeMap<String, Vec<String>>,
) {
    for node in &file.root {
        occurrences
            .entry(node.id.as_str().to_owned())
            .or_default()
            .push(name.to_owned());
    }
    for node in &file.request {
        occurrences
            .entry(node.id.as_str().to_owned())
            .or_default()
            .push(name.to_owned());
    }
    for section in &file.require {
        index_section_ids(section, name, occurrences);
    }
    for section in &file.spec {
        index_section_ids(section, name, occurrences);
    }
    for section in &file.detailed_spec {
        index_section_ids(section, name, occurrences);
    }
    for section in &file.basic_design {
        index_section_ids(section, name, occurrences);
    }
    for section in &file.design {
        index_section_ids(section, name, occurrences);
    }
}

fn index_section_ids(
    section: &SectionNode,
    name: &str,
    occurrences: &mut BTreeMap<String, Vec<String>>,
) {
    occurrences
        .entry(section.id.as_str().to_owned())
        .or_default()
        .push(name.to_owned());
    if let Some(items) = &section.items {
        for item in items {
            occurrences
                .entry(item.id.as_str().to_owned())
                .or_default()
                .push(name.to_owned());
        }
    }
    if let Some(children) = &section.sections {
        for child in children {
            index_section_ids(child, name, occurrences);
        }
    }
}

/// A document-node diagnostic's identifying record path: the repo-relative
/// `.verify/doc/<name>.json` path, embedded in the diagnostic's message text
/// rather than attached as a `SourceLocation` — document nodes are a record
/// layer, not an adapter-discovered construct, so (see `record_relative_path`'s
/// own doc comment) they must not fabricate one. Mirrors `record_relative_path`'s
/// use for every other record-layer diagnostic in this file (E-SCAN-008/009/
/// 010, and `validate_vo_document_references` below).
fn document_node_record_path(layout: &VerifyLayout, name: &str) -> String {
    record_relative_path(&layout.root, &layout.doc_dir().join(format!("{name}.json")))
}

/// Walks one `SectionNode` and its descendants, checking both E-SCAN-012
/// (this section's own `derives_from` entries resolve) and E-SCAN-016 (this
/// section's *effective* upstream — own edges ∪ `ancestor_has_upstream` —
/// is non-empty). `ancestor_has_upstream` carries whether any strict
/// ancestor section already had a non-empty `derives_from` (DS-1647's
/// "先祖の辺"); a section with its own edge also becomes an upstream-bearing
/// ancestor for its own children.
fn check_section_node(
    layout: &VerifyLayout,
    document: &str,
    section: &SectionNode,
    ancestor_has_upstream: bool,
    known_ids: &BTreeSet<String>,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let own_edges = section.derives_from.as_deref().unwrap_or(&[]);
    let record_path = document_node_record_path(layout, document);
    for target in own_edges {
        if !known_ids.contains(target.as_str()) {
            diagnostics.push(Diagnostic::error(
                "E-SCAN-012",
                format!(
                    "document node {} derives_from missing node {} ({record_path})",
                    section.id.as_str(),
                    target.as_str()
                ),
            ));
        }
    }
    let has_upstream = ancestor_has_upstream || !own_edges.is_empty();
    if !has_upstream {
        diagnostics.push(Diagnostic::error(
            "E-SCAN-016",
            format!(
                "document node {} is orphaned: no effective upstream (own or ancestor \
                 derives_from edges) ({record_path})",
                section.id.as_str()
            ),
        ));
    }

    if let Some(items) = &section.items {
        for item in items {
            check_sentence_node(layout, document, item, has_upstream, known_ids, diagnostics);
        }
    }
    if let Some(children) = &section.sections {
        for child in children {
            check_section_node(
                layout,
                document,
                child,
                has_upstream,
                known_ids,
                diagnostics,
            );
        }
    }
}

/// Same checks as `check_section_node`, for a `SentenceNode`. A sentence
/// node never has children, so it only ever reports for itself.
fn check_sentence_node(
    layout: &VerifyLayout,
    document: &str,
    sentence: &SentenceNode,
    ancestor_has_upstream: bool,
    known_ids: &BTreeSet<String>,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let record_path = document_node_record_path(layout, document);
    for target in &sentence.derives_from {
        if !known_ids.contains(target.as_str()) {
            diagnostics.push(Diagnostic::error(
                "E-SCAN-012",
                format!(
                    "document node {} derives_from missing node {} ({record_path})",
                    sentence.id.as_str(),
                    target.as_str()
                ),
            ));
        }
    }
    let has_upstream = ancestor_has_upstream || !sentence.derives_from.is_empty();
    if !has_upstream {
        diagnostics.push(Diagnostic::error(
            "E-SCAN-016",
            format!(
                "document node {} is orphaned: no effective upstream (own or ancestor \
                 derives_from edges) ({record_path})",
                sentence.id.as_str()
            ),
        ));
    }
}

/// chain_integrity（VO 層、本冊:878/別紙C:80）: 各 VO の `derives_from` は
/// upstream node へ解決できなければならない（DS-1660: `doc` field の値は
/// 上流ノード id であり、上流文書のファイル名ではない）。カーディナリティ
/// （1件以上）は `vtest_store::vo_record_from_yaml` が既に record 層で強制
/// しているので (`require_at_least_one_derives_from`)、ここでは各 entry の
/// 参照先が実在する node かどうかだけを検査する — `validate_document_nodes`
/// の E-SCAN-012 チェックと対になる、VO側の半分。
fn validate_vo_document_references(
    layout: &VerifyLayout,
    vos: &BTreeMap<String, VoRecord>,
    document_node_ids: &BTreeSet<String>,
    diagnostics: &mut Vec<Diagnostic>,
) {
    for (id, record) in vos {
        let record_path =
            record_relative_path(&layout.root, &layout.vo_dir().join(format!("{id}.yaml")));
        for entry in &record.derives_from {
            let target = entry.doc.as_str();
            if !document_node_ids.contains(target) {
                diagnostics.push(Diagnostic::error(
                    "E-SCAN-012",
                    format!("VO {id} derives_from missing node {target} ({record_path})"),
                ));
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
    let record_path = record_relative_path(&layout.root, &path);
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
            diagnostics.push(Diagnostic::error(
                "E-SCAN-010",
                format!("VO {id} has an invalid record: {error} ({record_path})"),
            ));
            return None;
        }
    };
    let context = format!("VO {id} ({record_path})");
    diagnostics.extend(
        record_diagnostics
            .into_iter()
            .map(|diagnostic| annotate_record_diagnostic(diagnostic, &context)),
    );
    if let Some(message) = invalid_vo_combinations(&record) {
        diagnostics.push(Diagnostic::error(
            "E-SCAN-017",
            format!("VO {id} {message} ({record_path})"),
        ));
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

/// document/VO/relation/approval の record 層診断（`.verify/*.yaml`）が
/// 対象ファイルを特定できるよう、リポジトリ相対パスの文字列を組み立てる。
///
/// **経緯（`SourceLocation` reshape で表面化した不一致、上流未報告）**:
/// 本冊:637-642 が定める `SourceLocation` は `adapter: AdapterId` を必須
/// field とする — adapter が discovery した source construct（Test / Source
/// Target）の location を表すための型である（本冊 §5.2）。この関数が
/// パスを組み立てる対象（document / VO / relation / approval の canonical
/// YAML レコード）は、どの `SourceDiscoveryAdapter` にも属さない —
/// `vtest-store` が読む record ファイルであって、adapter が発見した source
/// construct ではない。
///
/// reshape 直後の実装は、型を満たすために実在しない adapter id
/// （`AdapterId::new("vtest-store")`）を捏造して `SourceLocation` を組み
/// 立てていた。Owner は同種の実装（解決不能な対象に架空の locator を入れる
/// こと）を明示的に否定しているため、これを取りやめた。record 層の診断は
/// そもそも adapter 起源ではないので `SourceLocation` を持たない
/// （`Diagnostic.location` は `None` のまま）。失われる情報（対象ファイルの
/// パス）は、この関数が返す文字列を呼び出し元がメッセージ本文へ埋め込むこと
/// で補う。
fn record_relative_path(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .to_string_lossy()
        .replace('\\', "/")
}

/// `vtest-store` の record reader のうち `(record, Vec<Diagnostic>)` を返す
/// もの（`read_vo_record` / `RelationRecord::from_yaml`）が生成する
/// diagnostics（W-STORE-007 等）は、どの VO/relation/approval レコードから
/// 来たかを知らずに生成される — メッセージ本文に id もパスも含まない
/// （`vtest-store` 側は `VerifyLayout`/ファイルパスの文脈を持たない）。
/// 呼び出し元（この crate）は id とパスを知っているので、
/// `record_relative_path` で失われた `SourceLocation` の代わりに、対象を
/// 識別できる文脈をメッセージ先頭へ前置する。（`read_document_file` は
/// この形に当てはまらない — `Vec<Diagnostic>` を返さず、失敗は
/// `validate_document_nodes` がこの crate 自身で E-SCAN-010 へ畳み込む。
/// ここでの annotate 対象ではない。）
fn annotate_record_diagnostic(diagnostic: Diagnostic, context: &str) -> Diagnostic {
    Diagnostic {
        message: format!("{context}: {}", diagnostic.message),
        ..diagnostic
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
                let record_path = record_relative_path(root, &directory.join(format!("{id}.yaml")));
                diagnostics.push(Diagnostic::error(
                    "E-SCAN-008",
                    format!("{kind} {id} references missing parent {parent} ({record_path})"),
                ));
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
                    let record_path =
                        record_relative_path(root, &directory.join(format!("{current}.yaml")));
                    diagnostics.push(Diagnostic::error(
                        "E-SCAN-008",
                        format!(
                            "{kind} parent cycle: {} ({record_path})",
                            cycle.join(" -> ")
                        ),
                    ));
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

/// レビュー round 2 項目【L】: `layout.relation_dir()` が存在しないこと
/// （まだ Relation レコードが1件も無い正常なリポジトリ）と、存在するが
/// 読めないこと（権限エラー等）を区別する。前者は空の Relation 集合として
/// 扱い（診断ゼロで正しい）、後者は Relation 検査全体が診断ゼロで黙って
/// スキップされる fail-open を避けるため、`ScanError::Io` で scan 全体を
/// fail-closed に中断する。E-SCAN-010（本冊:876「レコードのid / ファイル名
/// / schema不一致」）はこの条件（ディレクトリを列挙できない）に適合しない
/// ため、当てはめず既存の `ScanError` 経路を使う。
///
/// 要確認D（PR #26 review round 2）: ディレクトリ自体が開けても、列挙中に
/// 個々の `DirEntry` が `Err` を返すこと（列挙中の I/O エラー等）はなお
/// ありうる。`Iterator::flatten` で読み飛ばすと、その1レコードだけが
/// 検査対象から静かに脱落する — ディレクトリ自体を区別した理由と同じ
/// fail-open を、粒度を変えて再導入してしまう。ここでも同じ扱い
/// （`ScanError::Io` で scan 全体を中断）に揃える。
fn validate_relations(
    layout: &VerifyLayout,
    known_ids: &BTreeSet<String>,
    diagnostics: &mut Vec<Diagnostic>,
) -> Result<(), ScanError> {
    let relation_dir = layout.relation_dir();
    let entries = match fs::read_dir(&relation_dir) {
        Ok(entries) => entries,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(()),
        Err(source) => {
            return Err(ScanError::Io {
                path: relation_dir,
                source,
            })
        }
    };
    let mut paths = Vec::new();
    for entry in entries {
        let entry = entry.map_err(|source| ScanError::Io {
            path: relation_dir.clone(),
            source,
        })?;
        let path = entry.path();
        if path.extension().and_then(|value| value.to_str()) == Some("yaml") {
            paths.push(path);
        }
    }
    paths.sort();
    let mut relation_payloads = BTreeMap::<String, String>::new();
    for path in paths {
        let file_id = path
            .file_stem()
            .and_then(|value| value.to_str())
            .unwrap_or_default()
            .to_owned();
        let record_path = record_relative_path(&layout.root, &path);
        if let Some(payload) = relation_ulid_payload(&file_id) {
            if let Some(first) = relation_payloads.insert(payload.to_owned(), file_id.clone()) {
                diagnostics.push(Diagnostic::error(
                    "E-SCAN-010",
                    format!(
                        "relation IDs {first} and {file_id} use the same ULID payload {payload} \
                         ({record_path})"
                    ),
                ));
                continue;
            }
        }
        let text = match read_text(&path) {
            Ok(text) => text,
            Err(error) => {
                diagnostics.push(Diagnostic::error(
                    "E-SCAN-010",
                    format!("relation {file_id} cannot be read: {error} ({record_path})"),
                ));
                continue;
            }
        };
        let (relation, relation_diagnostics) = match RelationRecord::from_yaml(&text, &file_id) {
            Ok(parsed) => parsed,
            Err(error) => {
                diagnostics.push(Diagnostic::error(
                    "E-SCAN-010",
                    format!("relation {file_id} has an invalid schema: {error} ({record_path})"),
                ));
                continue;
            }
        };
        // NOTE (未報告の既存ギャップ、本 PR の対象外): `relation_diagnostics`
        // （`RelationRecord::from_yaml` が返す W-STORE-007 等）はこの reshape
        // 以前から location もメッセージ内の id/path も持たない —
        // `vtest-store::unknown_field_diagnostics` のメッセージ自体に
        // relation の識別情報が無いため。document/VO と揃えて
        // `annotate_record_diagnostic` を通すべきだが、これは今回の sentinel
        // 修正が生んだ欠陥ではなく独立の既存ギャップなので、ここでは直さず
        // 報告のみに留める。
        diagnostics.extend(relation_diagnostics);
        for (field, value) in [("from", relation.from), ("to", relation.to)] {
            if !known_ids.contains(&value) {
                diagnostics.push(Diagnostic::error(
                    "E-SCAN-009",
                    format!(
                        "relation {file_id} {field} references missing entity {value} \
                         ({record_path})"
                    ),
                ));
            }
        }
    }
    Ok(())
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
            let record_path =
                record_relative_path(&layout.root, &layout.vo_dir().join(format!("{id}.yaml")));
            diagnostics.push(Diagnostic::warning(
                "W-SCAN-102",
                format!("VO {id} is isolated and has no covering test ({record_path})"),
            ));
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
/// レビュー round 2 項目【L】: `validate_relations` と同じ理由で
/// `layout.approvals_dir()` が存在しないこと（承認レコードがまだ1件も無い
/// 正常なリポジトリ）と、存在するが読めないこと（権限エラー等）を区別する。
/// 前者は空の承認集合として扱い、後者は承認レコード検査全体が診断ゼロで
/// 黙ってスキップされる fail-open を避けるため `ScanError::Io` で中断する。
///
/// 要確認D（PR #26 review round 2）: `validate_relations` と同じ理由で、
/// 列挙中の個々の `DirEntry` の `Err`（`Iterator::flatten` が読み飛ばして
/// いた）もディレクトリ自体の `Err` と同じ扱い（`ScanError::Io` で中断）
/// に揃える。
fn validate_approval_status(
    layout: &VerifyLayout,
    vos: &BTreeMap<String, VoRecord>,
    diagnostics: &mut Vec<Diagnostic>,
) -> Result<(), ScanError> {
    // 開示（PR34 の ContentHash::from_text 全数調査より）: `current_hashes`'s `ContentHash` values are
    // computed here but never read below — the only consumer is
    // `.contains_key(subject)` at this function's tail, a presence check
    // that would work identically over a `BTreeSet<String>` of VO ids. No
    // comparison against `approval.subject_hash` exists anywhere in this
    // function (confirmed: `approval.subject_hash` is never referenced after
    // `read_approval` below), so this is not currently a VO-content
    // staleness check despite computing a per-VO hash. Left unsimplified and
    // the `ContentHash::from_text` call left unreplaced (rather than
    // collapsing to a `BTreeSet` or wiring in a real `vo_subject_hash`
    // comparison) because either change reaches into Approval-record
    // validity semantics, which this function's own doc comment already
    // marks out of this PR's scope pending the canonical Approval migration
    // (item 5 / PR4+; DES-093 would be the binding rule for that eventual
    // comparison).
    let mut current_hashes = BTreeMap::new();
    for id in vos.keys() {
        let path = layout.vo_dir().join(format!("{id}.yaml"));
        if let Ok(text) = read_text(&path) {
            current_hashes.insert(id.clone(), ContentHash::from_text(&text));
        }
    }
    let approvals_dir = layout.approvals_dir();
    let entries = match fs::read_dir(&approvals_dir) {
        Ok(entries) => entries,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(()),
        Err(source) => {
            return Err(ScanError::Io {
                path: approvals_dir,
                source,
            })
        }
    };
    for entry in entries {
        let entry = entry.map_err(|source| ScanError::Io {
            path: approvals_dir.clone(),
            source,
        })?;
        let path = entry.path();
        if path.extension().and_then(|value| value.to_str()) != Some("yaml") {
            continue;
        }
        let file_id = path
            .file_stem()
            .and_then(|value| value.to_str())
            .unwrap_or_default()
            .to_owned();
        let record_path = record_relative_path(&layout.root, &path);
        let text = match read_text(&path) {
            Ok(text) => text,
            Err(error) => {
                diagnostics.push(Diagnostic::error(
                    "E-SCAN-010",
                    format!("approval {file_id} cannot be read: {error} ({record_path})"),
                ));
                continue;
            }
        };
        let mut invalid = false;
        if !is_valid_ulid(&file_id) {
            diagnostics.push(Diagnostic::error(
                "E-SCAN-010",
                format!("approval file name {file_id} is not a valid ULID ({record_path})"),
            ));
            invalid = true;
        }
        if let Some(missing) = missing_fields(
            &text,
            &[
                "id",
                "subject_type",
                "subject",
                "subject_hash",
                "approved_state",
                "approved_at",
            ],
        ) {
            diagnostics.push(Diagnostic::error(
                "E-SCAN-010",
                format!("approval {file_id} is missing required fields: {missing} ({record_path})"),
            ));
            invalid = true;
        }
        let approval = match read_approval(&path) {
            Ok(approval) => approval,
            Err(error) => {
                diagnostics.push(Diagnostic::error(
                    "E-SCAN-010",
                    format!("approval {file_id} has an invalid schema: {error} ({record_path})"),
                ));
                continue;
            }
        };
        if approval.id != file_id {
            diagnostics.push(Diagnostic::error(
                "E-SCAN-010",
                format!(
                    "approval file name {file_id} does not match record id {} ({record_path})",
                    approval.id
                ),
            ));
            invalid = true;
        }
        if invalid {
            continue;
        }
        // DS-1475: the effective承認対象の値域は VO ID と document ID の
        // 2 種のみ。`subject_type == "vo"` の場合だけ、この関数がすでに
        // 持っている VO ハッシュ集合と突き合わせる — `document`/`judgment`
        // の存在確認は現時点でこの関数の対象外（document ノードの存在は
        // orphan_detection/chain_integrity 側が別途扱う; judgment 参照の
        // 存在確認は DS-1052 の判断記録ドメインが未実装のため、この
        // closure-slice の範囲外）。
        if approval.subject_type == "vo" {
            let subject = approval.subject.as_str();
            if !current_hashes.contains_key(subject) {
                diagnostics.push(Diagnostic::error(
                    "E-SCAN-010",
                    format!("approval {file_id} references missing VO {subject} ({record_path})"),
                ));
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use vtest_model::{DocumentId, NodeSource, RootNode, SrcId, TestId, TestSuite, VoId};
    use vtest_store::{init_project, new_record_id, write_document_file, FormAnswers, FormValue};

    fn valid_vo(id: &str, parent: &str) -> String {
        format!(
            "id: {id}\nparent: {parent}\nderives_from:\n  - doc: ROOT-001\nclaim: claim\ndimensions: []\ncoverage_policy: null\ncombinations: []\nrepresentative_cases: []\ncreated: '2026-01-01'\nupdated: '2026-01-01'\n"
        )
    }

    fn fixture_node_source() -> NodeSource {
        NodeSource {
            doc: "docs/test.md".to_owned(),
            heading: "1".to_owned(),
            lines: [1, 1],
        }
    }

    /// Writes an empty `DocumentFile` (DES-586) with a single `root`-layer
    /// node, `ROOT-001` — the node every `valid_vo` fixture record declares
    /// as its `derives_from` target (DS-1660: a VO's `derives_from[].doc` is
    /// an upstream *node* id, not a document file name). `root`-layer nodes
    /// are excluded from orphan_detection outright (DS-1646/DS-1647) and
    /// resolve trivially for E-SCAN-012, so this fixture never itself trips
    /// either check.
    fn write_doc_test_fixture(root: &Path) {
        let layout = VerifyLayout::new(root);
        let file = DocumentFile {
            schema_version: "0.1".to_owned(),
            root: vec![RootNode {
                id: DocumentId::new("ROOT-001"),
                statement: "fixture root ruling".to_owned(),
                description: None,
                source: fixture_node_source(),
            }],
            request: Vec::new(),
            require: Vec::new(),
            spec: Vec::new(),
            detailed_spec: Vec::new(),
            basic_design: Vec::new(),
            design: Vec::new(),
        };
        write_document_file(&layout, "DOC-TEST", &file).unwrap();
    }

    fn fixture() -> PathBuf {
        // A nanosecond-timestamp suffix alone collides under parallel test
        // execution on Windows' coarser clock resolution -- two tests can
        // get the same value, and the second `init_project` call then
        // fails with `AlreadyInitialized` (the confirmed root cause of a
        // previously-unconfirmed flaky failure; team-lead reproduced it
        // via `covers_and_related_accept_comma_separated_values`). Use
        // process id + a per-process atomic counter instead, matching
        // `operations.rs`'s own `temp_root` helper.
        static COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let sequence = COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let root =
            std::env::temp_dir().join(format!("vtest-scan-{}-{sequence}", std::process::id()));
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

    /// @vtest.id TEST-SCAN-EXTRACTS-ANNOTATED-TEST-AND-SOURCE
    /// @vtest.covers VO-SCAN-RUST-CARGO-EXECUTION-COORDINATES
    /// @vtest.target crates/vtest-scan/src/lib.rs::scan_project
    /// @vtest.intent verifies scan_project extracts an annotated Test construct with correct id, execution.selector, project, and suite.kind/name
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
        assert_eq!(result.tests[0].execution.selector, "adds");
        assert_eq!(
            result.tests[0].execution.project.as_deref(),
            Some("fixture")
        );
        assert_eq!(
            result.tests[0].execution.suite,
            Some(vtest_model::TestSuite {
                kind: "integration".to_owned(),
                name: Some("calc".to_owned()),
            })
        );
    }

    /// DES-083（本冊:88）: Source Target hash は canonical Target Reference
    /// と construct bytes の両方を束縛する。construct bytes だけを hash して
    /// いた旧実装（Issue #27）では、byte列が同一の関数が別の場所にあると
    /// 同一 hash になっていた。二つのファイルへ byte-identical な関数を
    /// 置き、`content_hash` が異なることを確認する — この配線が固定する
    /// まさにその性質。
    ///
    /// 最小構成の専用 fixture で配線箇所だけを狭く確認する版。既存の
    /// `fixture()`（Test・VO・doc を含む現実的な構成）を通した確認は
    /// `source_target_hash_differs_for_identical_construct_text_at_different_locations`
    /// が別に持つ。両方に価値があるため両方残す。
    /// @vtest.id TEST-SCAN-SOURCE-TARGET-HASH-BINDS-LOCATION-DUP-CONTENT
    /// @vtest.covers VO-MODEL-SOURCE-TARGET-SUBJECT-HASH
    /// @vtest.target crates/vtest-scan/src/lib.rs::scan_project
    /// @vtest.intent verifies two Source Targets with byte-identical construct bytes at different canonical Locators get different content hashes
    #[test]
    fn source_targets_with_identical_construct_bytes_at_different_locations_get_different_hashes() {
        // See `fixture()`'s doc comment: a nanosecond-timestamp suffix
        // alone collides under parallel test execution on Windows.
        static COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let sequence = COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let root = std::env::temp_dir().join(format!(
            "vtest-scan-dup-content-{}-{sequence}",
            std::process::id()
        ));
        fs::create_dir_all(root.join("src")).unwrap();
        fs::write(
            root.join("Cargo.toml"),
            "[package]\nname = \"fixture\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
        )
        .unwrap();
        init_project(&root, "fixture").unwrap();
        write_doc_test_fixture(&root);
        fs::write(root.join("src/lib.rs"), "pub fn root_helper() {}\n").unwrap();
        // Byte-identical construct text at two different paths.
        fs::write(root.join("src/a.rs"), "pub fn helper() -> i32 { 42 }\n").unwrap();
        fs::write(root.join("src/b.rs"), "pub fn helper() -> i32 { 42 }\n").unwrap();

        let result = scan_project(&root).unwrap();
        let helper_a = result
            .sources
            .iter()
            .find(|source| source.locator.value.contains("a.rs"))
            .expect("src/a.rs::helper must be discovered as a source");
        let helper_b = result
            .sources
            .iter()
            .find(|source| source.locator.value.contains("b.rs"))
            .expect("src/b.rs::helper must be discovered as a source");

        assert_ne!(
            helper_a.content_hash, helper_b.content_hash,
            "two Source Targets with byte-identical construct bytes at different \
             canonical Locators must not collide (本冊:88, Issue #27)"
        );
    }

    /// Locks in the `test_subject_hash` → `TestEntity.content_hash` wiring
    /// in `materialize_tests`. §1.3 (本冊:87) makes the Test subject hash
    /// bind canonical metadata (including `covers`) in addition to
    /// construct bytes, and 別紙C:35 requires this specific consequence: a
    /// metadata-only change must change `TestEntity.content_hash` even when
    /// the Test construct bytes are byte-for-byte identical. Before this
    /// wiring, `content_hash` was `ContentHash::from_text(&draft.
    /// construct_text)` — construct bytes only — so a metadata-only change
    /// would not have changed it.
    ///
    /// This drives `materialize_tests` directly with two hand-built
    /// `TestDraft`s built from the exact same `construct_text` `String`
    /// (cloned, never mutated between the two calls) and differing only in
    /// `covers`, rather than going through `rust-cargo::discover` on a
    /// `@vtest.covers` source edit. At the time this test was written,
    /// `rust-cargo` sliced a Test construct as the whole item span including
    /// its metadata doc comment, so a source-edit-based version of this test
    /// would have changed `construct_text` too and no longer isolated a
    /// metadata-only change — driving `materialize_tests` directly sidestepped
    /// that gap. That gap is now closed (`vtest_adapter_rust::
    /// test_construct_start_span` excludes the leading metadata doc comment
    /// per 本冊:99), and `scan_project_content_hash_changes_when_only_covers_
    /// metadata_changes_via_source_edit` below covers the same property
    /// through a real `@vtest.covers` source edit end-to-end. This test is
    /// kept alongside it because it isolates the `materialize_tests` wiring
    /// itself (independent of adapter discovery) with no other moving parts.
    /// @vtest.id TEST-SCAN-MATERIALIZE-TESTS-HASH-CHANGES-ON-COVERS-EDIT
    /// @vtest.covers VO-MODEL-TEST-SUBJECT-HASH
    /// @vtest.target crates/vtest-scan/src/lib.rs::materialize_tests
    /// @vtest.intent verifies materialize_tests' content_hash changes when only the covers metadata differs, construct bytes held identical
    #[test]
    fn materialize_tests_content_hash_changes_when_only_covers_metadata_changes() {
        let construct_text = "fn adds() { assert_eq!(2, add(1, 1)); }".to_owned();
        let location = SourceLocation {
            adapter: AdapterId::new("rust-cargo"),
            path: vtest_model::ProjectPath::new("tests/calc.rs"),
            locator: "adds".to_owned(),
            byte_range: vtest_model::SourceRange {
                start: 0,
                end: construct_text.len() as u64,
            },
        };
        let execution = vtest_model::ExecutionDescriptor {
            adapter: AdapterId::new("rust-cargo"),
            project: Some("fixture".to_owned()),
            suite: Some(vtest_model::TestSuite {
                kind: "integration".to_owned(),
                name: Some("calc".to_owned()),
            }),
            selector: "adds".to_owned(),
        };
        let draft_with_covers = |covers: Vec<vtest_model::VoId>| vtest_adapter_api::TestDraft {
            id: vtest_model::TestId::new("TEST-ADD"),
            covers,
            targets: vec![TargetRef::Locator(vtest_model::Locator {
                adapter: AdapterId::new("rust-cargo"),
                value: "src/lib.rs::add".to_owned(),
            })],
            intent: "adds values".to_owned(),
            input: None,
            expect: None,
            kind: None,
            cases: Vec::new(),
            related: Vec::new(),
            location: location.clone(),
            construct_text: construct_text.clone(),
            execution: execution.clone(),
        };

        let (before_tests, _, _) = materialize_tests(
            vec![(
                AdapterId::new("rust-cargo"),
                draft_with_covers(vec![vtest_model::VoId::new("VO-ADD")]),
            )],
            Vec::new(),
        );
        let (after_tests, _, _) = materialize_tests(
            vec![(
                AdapterId::new("rust-cargo"),
                draft_with_covers(vec![vtest_model::VoId::new("VO-ADD-2")]),
            )],
            Vec::new(),
        );

        assert_eq!(before_tests.len(), 1);
        assert_eq!(after_tests.len(), 1);
        assert_ne!(
            before_tests[0].covers, after_tests[0].covers,
            "the two drafts must actually declare different covers"
        );

        assert_ne!(
            before_tests[0].content_hash, after_tests[0].content_hash,
            "TestEntity.content_hash must change when only covers metadata \
             changes, even though construct_text is byte-for-byte identical \
             (別紙C:35)"
        );
    }

    /// 本冊:99「`rust-cargo` adapterはTest constructとしてmetadata doc
    /// commentを除き、実行に影響する属性、signature、bodyを含む関数itemの
    /// bytesを返す」を実ファイル経由で確認する。`@vtest.covers`はmetadata
    /// doc commentの内側にあるため、その書き換えはTest construct bytes
    /// （`DiscoveredTest.content_hash` — construct-onlyのhash。
    /// `materialize_tests`のdoc comment参照）を変えないが、`TestEntity.
    /// content_hash`（Test subject hash。§1.3, 本冊:87はcanonical metadata
    /// も束縛する）は変える（別紙C:35）。上の
    /// `materialize_tests_content_hash_changes_when_only_covers_metadata_
    /// changes`が`materialize_tests`の配線だけを直接駆動して確認するのに
    /// 対し、この版は`scan_project`を通してadapter discoveryから通し、
    /// 「metadataだけ変えてconstructは不変」という状態が実ファイル編集
    /// からも作れることそのものを確認する。
    /// @vtest.id TEST-SCAN-PROJECT-HASH-CHANGES-ON-COVERS-SOURCE-EDIT
    /// @vtest.covers VO-MODEL-TEST-SUBJECT-HASH
    /// @vtest.target crates/vtest-scan/src/lib.rs::scan_project
    /// @vtest.intent verifies scan_project's TestEntity.content_hash changes end-to-end when a real @vtest.covers source edit changes only metadata, not construct bytes
    #[test]
    fn scan_project_content_hash_changes_when_only_covers_metadata_changes_via_source_edit() {
        let root = fixture();
        fs::write(
            root.join(".verify/vo/VO-ADD-2.yaml"),
            valid_vo("VO-ADD-2", "null"),
        )
        .unwrap();

        let before = scan_project(&root).unwrap();
        assert!(
            !before.has_errors(),
            "diagnostics: {:?}",
            before.diagnostics
        );
        let before_test = before
            .tests
            .iter()
            .find(|test| test.id.as_str() == "TEST-ADD")
            .expect("fixture registers TEST-ADD");
        let before_construct_hash = before
            .discovered
            .iter()
            .find(|discovered| {
                matches!(&discovered.managed, ManagedTestLink::One(id) if id.as_str() == "TEST-ADD")
            })
            .expect("TEST-ADD must have a DiscoveredTest observation")
            .content_hash
            .clone();
        // `adds`自身もSource Target（属性とdoc commentを含む関数item全体。
        // 本冊:99）として登録される（`collect_function_parts`は`is_test`
        // 判定より前に無条件で`self.sources`へpushする）。この
        // `SourceFunction.content_hash`はdoc commentを含む範囲から計算する
        // ため、`@vtest.covers`の書き換えで変わるはずである — Test
        // construct側が不変であることの非対称な裏付け（Source Targetの
        // 範囲は変えていないことの実測）。
        let before_source_hash = before
            .sources
            .iter()
            .find(|source| source.locator.value == "tests/calc.rs::adds")
            .expect("fixture's own test function must be registered as a Source Target")
            .content_hash
            .clone();

        // `@vtest.covers`だけを書き換える。属性・signature・bodyは不変。
        fs::write(
            root.join("tests/calc.rs"),
            r#"
/// @vtest.id TEST-ADD
/// @vtest.covers VO-ADD-2
/// @vtest.target src/lib.rs::add
/// @vtest.intent adds values
#[test]
fn adds() { assert_eq!(2, crate::missing()); }
"#,
        )
        .unwrap();

        let after = scan_project(&root).unwrap();
        assert!(!after.has_errors(), "diagnostics: {:?}", after.diagnostics);
        let after_test = after
            .tests
            .iter()
            .find(|test| test.id.as_str() == "TEST-ADD")
            .expect("fixture registers TEST-ADD");
        let after_construct_hash = after
            .discovered
            .iter()
            .find(|discovered| {
                matches!(&discovered.managed, ManagedTestLink::One(id) if id.as_str() == "TEST-ADD")
            })
            .expect("TEST-ADD must have a DiscoveredTest observation")
            .content_hash
            .clone();
        let after_source_hash = after
            .sources
            .iter()
            .find(|source| source.locator.value == "tests/calc.rs::adds")
            .expect("fixture's own test function must be registered as a Source Target")
            .content_hash
            .clone();

        assert_ne!(
            before_test.covers, after_test.covers,
            "the source edit must actually declare different covers"
        );
        assert_eq!(
            before_construct_hash, after_construct_hash,
            "Test construct bytes (metadata doc comment excluded, 本冊:99) \
             must stay unchanged when only @vtest.covers changes"
        );
        assert_ne!(
            before_test.content_hash, after_test.content_hash,
            "TestEntity.content_hash must change when only covers metadata \
             changes, even though Test construct bytes are unchanged (別紙C:35)"
        );
        assert_ne!(
            before_source_hash, after_source_hash,
            "Source Target hash for `adds` itself must change — its \
             construct bytes still include the metadata doc comment \
             (本冊:99 \"Source Targetには属性とdoc commentを含む関数item \
             全体を返す\"), so rewriting @vtest.covers changes those bytes \
             even though it leaves the Test construct (attrs/signature/body \
             only) unchanged. This is the asymmetry that proves the two \
             ranges were actually split, not both silently narrowed."
        );
    }

    /// DES-083（本冊:88）: Source Target hash は canonical Target Reference
    /// と adapterが返すimplementation construct bytesの両方を束縛する。同一の
    /// construct bytesを持つ2つのSource Targetが異なる場所（＝異なる
    /// canonical Locator）にある場合、hashは異なる値になるべきである
    /// （配線前はconstruct bytesのみをhashしていたため、同一内容・異なる
    /// 場所の関数が同一ハッシュになっていた。Issue #27）。
    ///
    /// 既存の `fixture()`（Test・VO・doc 登録済みの現実的な構成、`pub mod`
    /// によるモジュール分割）を通した確認版。最小構成での確認は
    /// `source_targets_with_identical_construct_bytes_at_different_locations_get_different_hashes`
    /// が別に持つ。両方に価値があるため両方残す。
    /// @vtest.id TEST-SCAN-SOURCE-TARGET-HASH-DIFFERS-BY-LOCATION
    /// @vtest.covers VO-MODEL-SOURCE-TARGET-SUBJECT-HASH
    /// @vtest.target crates/vtest-scan/src/lib.rs::scan_project
    /// @vtest.intent verifies, through a realistic multi-module fixture, that identical construct bytes at different canonical Locators hash differently
    #[test]
    fn source_target_hash_differs_for_identical_construct_text_at_different_locations() {
        let root = fixture();
        fs::write(
            root.join("src/lib.rs"),
            "pub mod second;\n\npub fn helper() -> i32 { 0 }\n",
        )
        .unwrap();
        fs::write(root.join("src/second.rs"), "pub fn helper() -> i32 { 0 }\n").unwrap();
        let result = scan_project(&root).unwrap();
        let lib_helper = result
            .sources
            .iter()
            .find(|source| source.locator.value == "src/lib.rs::helper")
            .unwrap();
        let second_helper = result
            .sources
            .iter()
            .find(|source| source.locator.value == "src/second.rs::helper")
            .unwrap();
        assert_ne!(
            lib_helper.content_hash, second_helper.content_hash,
            "identical construct bytes at different canonical locators must hash \
             differently once the canonical Target Reference is bound (本冊:88)"
        );
    }

    /// 未知 adapter ID の fail-closed 拒否。拒否すること自体は DS-1292・
    /// DS-333/REQ-265 により確定しており、診断コードは DS-352 の
    /// statement/descriptionと DS-1663 の statement/description が
    /// 揃って明示する E-CONFIG-001 で確定する（正本監査、診断コード全数
    /// 照合、主題H — 旧版は E-ADAPTER-001 を返しており、退役 md
    /// （本冊:1639/1644、別紙C:319）とIssue #24を根拠に挙げていたが、
    /// 正本の逐語と食い違っていたため修正した）。config.yaml の唯一の
    /// adapter エントリを未登録 ID へ書き換えると、discovery からの黙った
    /// 除外（旧挙動: テスト0件の正常 scan）ではなく
    /// `ScanError::UnknownAdapterId`（`.code() == Some("E-CONFIG-001")`）を
    /// 返すこと、かつそのメッセージが未登録だった ID と登録済み ID 一覧の
    /// 両方を含むことを確認する。
    /// @vtest.id TEST-SCAN-UNKNOWN-ADAPTER-ID-REJECTED
    /// @vtest.covers VO-SCAN-UNKNOWN-ADAPTER-ID-REJECTED-E-CONFIG-001
    /// @vtest.target crates/vtest-scan/src/lib.rs::scan_project
    /// @vtest.intent verifies scan_project rejects an unregistered adapter id with fail-closed ScanError::UnknownAdapterId (E-CONFIG-001) naming both ids
    #[test]
    fn unknown_adapter_id_is_rejected_fail_closed() {
        let root = fixture();
        let layout = VerifyLayout::new(&root);
        let mut config = load_config(&root).unwrap();
        assert_eq!(config.adapters.len(), 1, "fixture registers one adapter");
        config.adapters[0].id = "unknown-lang".to_owned();
        fs::write(layout.config(), config.to_yaml()).unwrap();

        let error = match scan_project(&root) {
            Err(err @ ScanError::UnknownAdapterId { .. }) => {
                assert_eq!(err.code(), Some("E-CONFIG-001"));
                err.to_string()
            }
            other => {
                panic!(
                    "expected ScanError::UnknownAdapterId for an unregistered adapter id, got {other:?}"
                )
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
    /// @vtest.id TEST-SCAN-DISCOVERY-ERROR-CARRIES-E-ADAPTER-002
    /// @vtest.covers VO-SCAN-DISCOVERY-FAILURE-E-ADAPTER-002
    /// @vtest.target crates/vtest-scan/src/lib.rs::ScanError
    /// @vtest.intent verifies vtest_adapter_api::DiscoveryError converts into ScanError carrying code E-ADAPTER-002
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

    /// @vtest.id TEST-SCAN-MISSING-CARGO-METADATA-FAIL-CLOSED
    /// @vtest.covers VO-SCAN-TARGET-UNRESOLVABLE-E-SCAN-004
    /// @vtest.target crates/vtest-scan/src/lib.rs::scan_project
    /// @vtest.intent verifies invalid or missing Cargo.toml leaves target resolution unresolved (E-SCAN-004, suite None) instead of silently succeeding
    #[test]
    fn missing_or_invalid_cargo_metadata_is_fail_closed() {
        let root = fixture();
        fs::write(root.join("Cargo.toml"), "[package\ninvalid = true\n").unwrap();

        let result = scan_project(&root).unwrap();
        assert!(result.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "E-SCAN-004"
                && diagnostic.location.as_ref().is_some_and(|location| {
                    location.path.as_str() == "tests/calc.rs" && location.locator == "adds"
                })
        }));
        assert_eq!(result.tests[0].execution.suite, None);

        let root = fixture();
        fs::remove_file(root.join("Cargo.toml")).unwrap();
        let result = scan_project(&root).unwrap();
        assert!(result.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "E-SCAN-004"
                && diagnostic.location.as_ref().is_some_and(|location| {
                    location.path.as_str() == "tests/calc.rs" && location.locator == "adds"
                })
        }));
        assert_eq!(result.tests[0].execution.suite, None);
    }

    /// DS-349: "`config.yaml` の各adapterの `scan` 設定の `include` は
    /// テストコード走査パスであり、省略時はワークスペース全体を対象とする".
    /// An omitted `include` must resolve to this adapter's own root — not an
    /// empty list, and not `default_for`'s concrete `src`/`tests`/`crates`
    /// literal (that is one adapter's chosen default value, not DS-349's
    /// stated default).
    /// @vtest.id TEST-SCAN-RESOLVE-INCLUDES-NONE-TARGETS-WHOLE-ROOT
    /// @vtest.covers VO-SCAN-OMITTED-INCLUDE-SCANS-WHOLE-ROOT
    /// @vtest.target crates/vtest-scan/src/lib.rs::resolve_adapter_includes
    /// @vtest.intent verifies an omitted scan.include resolves to the adapter root itself (empty relative path), not an empty include list
    #[test]
    fn resolve_adapter_includes_none_targets_the_whole_adapter_root() {
        let mut adapter = ProjectConfig::default_for("fixture")
            .adapters
            .into_iter()
            .next()
            .unwrap();
        assert!(adapter.scan.include.is_some(), "precondition: default_for sets an explicit include list, not None — this test is exercising the *other* case");
        adapter.scan.include = None;

        let includes = resolve_adapter_includes(&adapter);
        assert_eq!(
            includes,
            vec![PathBuf::new()],
            "an omitted include must resolve to the adapter root itself (joined with an \
             empty relative path), not an empty include list: {includes:?}"
        );
    }

    /// @vtest.id TEST-SCAN-OMITTED-INCLUDE-SCANS-WHOLE-WORKSPACE
    /// @vtest.covers VO-SCAN-OMITTED-INCLUDE-SCANS-WHOLE-ROOT
    /// @vtest.target crates/vtest-scan/src/lib.rs::scan_project
    /// @vtest.intent verifies scan_project discovers a Test construct outside the adapter's default include literal when scan.include is omitted
    #[test]
    fn omitted_scan_include_scans_the_whole_workspace() {
        let root = fixture();
        // DS-349: place the adapter's `scan.include` at `None` and put a
        // Test construct outside `default_for`'s own `src`/`tests`/`crates`
        // literal to prove the whole workspace is in scope, not silently
        // narrowed back to that literal or to nothing.
        let layout = VerifyLayout::new(&root);
        let mut config = load_config(&root).unwrap();
        config.adapters[0].scan.include = None;
        fs::write(layout.config(), config.to_yaml()).unwrap();

        fs::create_dir_all(root.join("other")).unwrap();
        fs::write(
            root.join("other/extra.rs"),
            r#"
/// @vtest.id TEST-OUTSIDE-DEFAULT-INCLUDE
/// @vtest.covers VO-ADD
/// @vtest.target src/lib.rs::add
/// @vtest.intent lives outside src/tests/crates
#[test]
fn outside_default() {}
"#,
        )
        .unwrap();

        let result = scan_project(&root).unwrap();
        assert!(
            result
                .tests
                .iter()
                .any(|test| test.id.as_str() == "TEST-OUTSIDE-DEFAULT-INCLUDE"),
            "an omitted scan.include must scan the whole workspace, not just \
             default_for's src/tests/crates literal: {:?}",
            result.tests
        );
    }

    /// @vtest.id TEST-SCAN-RESOLVES-WORKSPACE-PACKAGES-AND-SUITES
    /// @vtest.covers VO-SCAN-RUST-CARGO-EXECUTION-COORDINATES
    /// @vtest.target crates/vtest-scan/src/lib.rs::scan_project
    /// @vtest.intent verifies scan_project resolves per-workspace-package project name and lib/bin/integration suite.kind/name/selector across module filters
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
        assert_eq!(
            module_test.execution.project.as_deref(),
            Some("parser-crate")
        );
        assert_eq!(
            module_test.execution.suite,
            Some(vtest_model::TestSuite {
                kind: "lib".to_owned(),
                name: None,
            })
        );
        assert_eq!(
            module_test.execution.selector,
            "parser::tests::parses_external_module"
        );

        let integration = result
            .tests
            .iter()
            .find(|test| test.id.as_str() == "TEST-PARSER-INTEGRATION")
            .unwrap();
        assert_eq!(
            integration.execution.project.as_deref(),
            Some("parser-crate")
        );
        assert_eq!(
            integration.execution.suite,
            Some(vtest_model::TestSuite {
                kind: "integration".to_owned(),
                name: Some("parser-suite".to_owned()),
            })
        );
        assert_eq!(
            integration.execution.selector,
            "support::parses_integration"
        );

        let binary = result
            .tests
            .iter()
            .find(|test| test.id.as_str() == "TEST-PARSER-BIN")
            .unwrap();
        assert_eq!(binary.execution.project.as_deref(), Some("parser-crate"));
        assert_eq!(
            binary.execution.suite,
            Some(vtest_model::TestSuite {
                kind: "bin".to_owned(),
                name: Some("parser-check".to_owned()),
            })
        );
        assert_eq!(binary.execution.selector, "checks_binary");
    }

    /// @vtest.id TEST-SCAN-IGNORED-RUST-FILES-NOT-SCANNED
    /// @vtest.covers VO-SCAN-RUST-DISCOVERY-RESPECTS-GITIGNORE
    /// @vtest.target crates/vtest-scan/src/lib.rs::scan_project
    /// @vtest.intent verifies a .gitignore/.ignore-excluded Rust file is not scanned and produces no E-SCAN-001 for it
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
            .any(|source| source.location.path.as_str() == "src/ignored.rs"));
        assert!(result
            .sources
            .iter()
            .any(|source| source.location.path.as_str() == "src/kept.rs"));
    }

    /// @vtest.id TEST-SCAN-AMBIGUOUS-TARGET-LOCATOR-NOT-RESOLVED
    /// @vtest.covers VO-SCAN-TARGET-UNRESOLVABLE-E-SCAN-004
    /// @vtest.target crates/vtest-scan/src/lib.rs::scan_project
    /// @vtest.intent verifies an ambiguous source locator (two cfg-gated definitions of the same symbol) is reported as unresolved (E-SCAN-004) with the declared value in the message
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
                    .is_some_and(|location| location.locator == "ambiguous")
                // 要確認C（PR #26 review round 2）: d4c1522でadapter側の
                // E-SCAN-004発行をやめてcoreの`resolve_targets`へ一本化した
                // 際、coreのメッセージから不正だった宣言値が落ちていた
                // （判定ロジックとは無関係の可読性の退行）。ここで宣言値が
                // メッセージに含まれることを断言してロックインする。
                && diagnostic.message.contains("src/ambiguous.rs::duplicate")
        }));
    }

    /// @vtest.id TEST-SCAN-REPORTS-UNREGISTERED-TESTS
    /// @vtest.covers VO-SCAN-UNREGISTERED-TEST-W-SCAN-101
    /// @vtest.target crates/vtest-scan/src/lib.rs::scan_project
    /// @vtest.intent verifies an undecorated #[test] function produces a W-SCAN-101 diagnostic
    #[test]
    fn reports_unregistered_tests() {
        let root = fixture();
        fs::write(root.join("tests/unregistered.rs"), "#[test]\nfn x() {}\n").unwrap();
        let result = scan_project(&root).unwrap();
        assert!(result.diagnostics.iter().any(|d| d.code == "W-SCAN-101"));
    }

    /// Owner裁定1（pr3-decisions.md）「scanner が観測した Test construct
    /// はすべて保持する」の直接ロックイン: `@vtest` annotation を持たない
    /// `#[test]` 関数（W-SCAN-101 の対象そのもの）が、`ScanResult.discovered`
    /// （集合 `D`。doc comment参照）に `ManagedTestLink::Missing` を持つ
    /// `DiscoveredTest` として**現れる**こと自体を断言する。以前は adapter
    /// 契約（`TestDraft`/`DiscoveryOutcome`）がこの construct について診断
    /// だけを返し、`vtest_adapter_api::MissingTestConstruct` に相当する
    /// 型が無かったため、この construct はモデルへ一切現れなかった
    /// （`result.tests` にも `result.discovered` にも痕跡が残らなかった）。
    /// @vtest.id TEST-SCAN-UNDECORATED-FN-APPEARS-IN-DISCOVERED-AS-MISSING
    /// @vtest.covers VO-SCAN-DISCOVERED-SET-RETAINS-UNMANAGED-CONSTRUCTS
    /// @vtest.target crates/vtest-scan/src/lib.rs::scan_project
    /// @vtest.intent verifies an undecorated #[test] function appears in ScanResult.discovered as ManagedTestLink::Missing rather than being dropped, and is absent from result.tests
    #[test]
    fn undecorated_test_functions_appear_in_discovered_as_missing() {
        let root = fixture();
        fs::write(root.join("tests/unregistered.rs"), "#[test]\nfn x() {}\n").unwrap();
        let result = scan_project(&root).unwrap();

        let missing: Vec<&DiscoveredTest> = result
            .discovered
            .iter()
            .filter(|entry| entry.location.locator == "x")
            .collect();
        assert_eq!(missing.len(), 1, "expected exactly one D entry for `x`");
        assert!(matches!(missing[0].managed, ManagedTestLink::Missing));
        assert_eq!(missing[0].adapter.as_str(), "rust-cargo");

        // `M`（構造上完全な managed Test Entity 集合）には現れない
        // （基本:412「構造上完全とは…Test Entity として具体化できること
        // をいう」— 管理宣言自体が無いため具体化できていない）。
        assert!(!result.tests.iter().any(|test| test.location.locator == "x"));
    }

    /// 上のテストは「annotation が無い」経路（W-SCAN-101）だけを断言する。
    /// この construct は `@vtest.id` を宣言しているのに `@vtest.covers` を
    /// 欠く別経路（E-SCAN-007、`Scanner::collect_function_parts` の
    /// `annotation.covers` 分岐）で、同じ `push_missing_test` 呼び出しが
    /// 通ることを別途断言する — 既存の診断（E-SCAN-007 の発行条件・
    /// メッセージ）が変わっていないことも同じテストで確認する。
    /// @vtest.id TEST-SCAN-MISSING-COVERS-APPEARS-IN-DISCOVERED-AS-MISSING
    /// @vtest.covers VO-SCAN-MISSING-REQUIRED-METADATA-E-SCAN-007
    /// @vtest.target crates/vtest-scan/src/lib.rs::scan_project
    /// @vtest.intent verifies a Test construct declaring @vtest.id but no @vtest.covers is rejected as E-SCAN-007 and still appears in discovered as Missing, not silently dropped
    #[test]
    fn test_construct_missing_required_covers_appears_in_discovered_as_missing() {
        let root = fixture();
        fs::write(
            root.join("tests/missing_covers.rs"),
            r#"
/// @vtest.id TEST-MISSING-COVERS
/// @vtest.target src/lib.rs::add
/// @vtest.intent declares no VO in covers
#[test]
fn missing_covers() {}
"#,
        )
        .unwrap();
        let result = scan_project(&root).unwrap();

        assert!(result.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "E-SCAN-007"
                && diagnostic
                    .location
                    .as_ref()
                    .is_some_and(|location| location.locator == "missing_covers")
                && diagnostic.message.contains("@vtest.covers")
        }));

        let missing: Vec<&DiscoveredTest> = result
            .discovered
            .iter()
            .filter(|entry| entry.location.locator == "missing_covers")
            .collect();
        assert_eq!(
            missing.len(),
            1,
            "expected exactly one D entry for `missing_covers`"
        );
        assert!(matches!(missing[0].managed, ManagedTestLink::Missing));

        assert!(!result
            .tests
            .iter()
            .any(|test| test.location.locator == "missing_covers"));
    }

    /// Owner裁定1（pr3-decisions.md）「Test ID が衝突した場合、先勝ちで1件
    /// を残して他を捨てることを禁止する」のロックイン。以前の実装は
    /// `materialize_tests` が2件目以降を `continue` で捨てていたため、
    /// この fixture では `TEST-COLLISION` を宣言する2 construct のうち
    /// `collision_second`（後発）が `result.tests` から消え、その
    /// `@vtest.covers VO-MISSING`（存在しない VO）も検証されないまま
    /// 素通りしていた。
    /// @vtest.id TEST-SCAN-COLLIDING-TEST-IDS-ALL-PRESERVED
    /// @vtest.covers VO-SCAN-COLLIDING-TEST-IDS-ALL-PRESERVED
    /// @vtest.target crates/vtest-scan/src/lib.rs::scan_project
    /// @vtest.intent verifies both constructs declaring a colliding Test ID are preserved as separate TestEntity records, reported symmetrically as E-SCAN-002, and reach downstream VO-reference checks, rather than the first-wins construct dropping the second
    #[test]
    fn colliding_test_ids_are_all_preserved_and_reach_downstream_checks() {
        let root = fixture();
        fs::write(
            root.join("tests/collision.rs"),
            r#"
/// @vtest.id TEST-COLLISION
/// @vtest.covers VO-ADD
/// @vtest.target src/lib.rs::add
/// @vtest.intent first construct declaring a colliding Test ID
#[test]
fn collision_first() {}

/// @vtest.id TEST-COLLISION
/// @vtest.covers VO-MISSING
/// @vtest.target src/lib.rs::add
/// @vtest.intent second construct declaring the same colliding Test ID, with an unresolvable VO reference
#[test]
fn collision_second() {}
"#,
        )
        .unwrap();

        let result = scan_project(&root).unwrap();

        // 1. 衝突した両方の construct が Test Entity として保持されている
        //    （先勝ちで2件目を落としていない）。
        let colliding: Vec<&TestEntity> = result
            .tests
            .iter()
            .filter(|test| test.id.as_str() == "TEST-COLLISION")
            .collect();
        assert_eq!(
            colliding.len(),
            2,
            "both constructs declaring the colliding Test ID must be preserved, not just the first"
        );
        let functions: BTreeSet<&str> = colliding
            .iter()
            .map(|test| test.location.locator.as_str())
            .collect();
        assert_eq!(
            functions,
            BTreeSet::from(["collision_first", "collision_second"])
        );

        // 2. summary.tests は実際に保持された件数と一致する（先勝ちで
        //    落とされていた分だけ実数とずれていた回帰の防止）。
        assert_eq!(result.summary.tests, result.tests.len() as u64);

        // 3. E-SCAN-002 は衝突した construct 全件へ対称に発行される
        //    （先発だけ／後発だけに偏らない）。
        let scan_002_functions: BTreeSet<&str> = result
            .diagnostics
            .iter()
            .filter(|d| d.code == "E-SCAN-002")
            .filter_map(|d| d.location.as_ref().map(|l| l.locator.as_str()))
            .collect();
        assert_eq!(
            scan_002_functions,
            BTreeSet::from(["collision_first", "collision_second"]),
            "E-SCAN-002 must be reported symmetrically for every colliding construct"
        );

        // 4. 後発 construct（collision_second）の不正な VO 参照
        //    （E-SCAN-003）が後段検査に届いている。捨てられていれば
        //    check_vo_references はこの construct を一切見ない。
        assert!(result.diagnostics.iter().any(|d| {
            d.code == "E-SCAN-003"
                && d.location
                    .as_ref()
                    .is_some_and(|l| l.locator == "collision_second")
        }));

        // 5. `tests_by_id` は代表1件を選ばず `Collided` を返す。
        match result.tests_by_id("TEST-COLLISION") {
            TestIdLookup::Collided(entities) => assert_eq!(entities.len(), 2),
            other => panic!("expected Collided, got {other:?}"),
        }

        // 6. 衝突していない Test ID（fixture が最初から持つ TEST-ADD）は
        //    `Unique` を返す — `tests_by_id` 自体が全件を衝突扱いにする
        //    退化をしていないことの確認。
        match result.tests_by_id("TEST-ADD") {
            TestIdLookup::Unique(test) => assert_eq!(test.id.as_str(), "TEST-ADD"),
            other => panic!("expected Unique, got {other:?}"),
        }

        // 7. `discovered`（集合 D）にも両 construct が1件ずつ現れ、それぞれ
        //    個別に `ManagedTestLink::One(自分のid)` を持つ（本冊:804）。
        //    Test ID の大域的衝突を `ManagedTestLink::Multiple` へ写像して
        //    いない（`ManagedTestLink` のdoc comment参照）。
        let discovered_for_collision: Vec<&DiscoveredTest> = result
            .discovered
            .iter()
            .filter(|d| {
                d.location.locator == "collision_first" || d.location.locator == "collision_second"
            })
            .collect();
        assert_eq!(discovered_for_collision.len(), 2);
        for entry in &discovered_for_collision {
            assert!(matches!(
                &entry.managed,
                ManagedTestLink::One(id) if id.as_str() == "TEST-COLLISION"
            ));
        }
    }

    /// @vtest.id TEST-SCAN-REJECTS-UNKNOWN-AND-DUPLICATE-ANNOTATION-KEYS
    /// @vtest.covers VO-SCAN-UNKNOWN-ANNOTATION-KEY-E-SCAN-006, VO-SCAN-DUPLICATE-KEY-E-SCAN-005
    /// @vtest.target crates/vtest-scan/src/lib.rs::scan_project
    /// @vtest.intent verifies an unrecognized annotation key produces E-SCAN-006 and a duplicated single-valued key (@vtest.id twice) produces E-SCAN-005
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

    /// @vtest.id TEST-SCAN-REJECTS-MISSING-REQUIRED-ANNOTATION
    /// @vtest.covers VO-SCAN-MISSING-REQUIRED-METADATA-E-SCAN-007
    /// @vtest.target crates/vtest-scan/src/lib.rs::scan_project
    /// @vtest.intent verifies a Test construct missing the required @vtest.intent annotation is rejected as E-SCAN-007
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

    /// REQ-150「1 つの Test は 1 件以上の Source Target を宣言できる」・
    /// SPEC-085（同文）・DS-1618「`case`・`related`・`target` はキー自体を
    /// 複数行書ける」: cardinality is N >= 1 with no upper bound and no
    /// execution-form or `@vtest.kind` condition. This crate used to allow
    /// more than one `target` only for a Cargo integration test
    /// (pr3-decisions.md Owner裁定3, PR #26 review round 5); the canonical
    /// audit found no upstream node granting that restriction, so it was
    /// removed rather than re-derived (`vtest-adapter-rust`'s
    /// `parse_test_annotations` and `vtest-scan::operations`'s
    /// `validate_desired_test`). This test asserts N=1, N=2, and N=3 all
    /// work, for a lib test (`src/`, not a Cargo integration test) and for
    /// a Cargo integration test (`tests/`) alike, with `@vtest.kind` values
    /// chosen to also show the decision is not kind-dependent.
    /// @vtest.id TEST-SCAN-N-TARGETS-KIND-INDEPENDENT
    /// @vtest.covers VO-SCAN-N-TARGETS-UNBOUNDED-KIND-INDEPENDENT
    /// @vtest.target crates/vtest-scan/src/lib.rs::scan_project
    /// @vtest.intent verifies a Test may declare 1, 2, or 3 distinct targets for a lib test and a Cargo integration test alike, regardless of @vtest.kind, with no E-SCAN-005
    #[test]
    fn tests_declare_any_number_of_targets_regardless_of_kind_or_physical_location() {
        let root = fixture();
        fs::write(
            root.join("src/lib.rs"),
            r#"pub fn add(a: i32, b: i32) -> i32 { a + b }
pub fn subtract(a: i32, b: i32) -> i32 { a - b }
pub fn multiply(a: i32, b: i32) -> i32 { a * b }

#[cfg(test)]
mod tests {
    /// @vtest.id TEST-LIB-ONE-TARGET
    /// @vtest.covers VO-ADD
    /// @vtest.target src/lib.rs::add
    /// @vtest.intent a lib test declaring exactly one target (N=1)
    /// @vtest.kind integration-normal
    #[test]
    fn one_target() {}

    /// @vtest.id TEST-LIB-TWO-TARGETS
    /// @vtest.covers VO-ADD
    /// @vtest.target src/lib.rs::add
    /// @vtest.target src/lib.rs::subtract
    /// @vtest.intent a lib test may declare more than one target (N=2)
    /// @vtest.kind integration-normal
    #[test]
    fn two_targets() {}
}
"#,
        )
        .unwrap();
        fs::write(
            root.join("tests/three_targets.rs"),
            r#"
/// @vtest.id TEST-INTEGRATION-THREE-TARGETS
/// @vtest.covers VO-ADD
/// @vtest.target src/lib.rs::add
/// @vtest.target src/lib.rs::subtract
/// @vtest.target src/lib.rs::multiply
/// @vtest.intent a Cargo integration test may declare three targets (N=3)
/// @vtest.kind unit-normal
#[test]
fn combines() {}
"#,
        )
        .unwrap();
        let result = scan_project(&root).unwrap();

        let one = result
            .tests
            .iter()
            .find(|test| test.id.as_str() == "TEST-LIB-ONE-TARGET")
            .unwrap();
        assert_eq!(one.targets.len(), 1);
        assert_eq!(
            one.execution.suite,
            Some(TestSuite {
                kind: "lib".to_owned(),
                name: None,
            })
        );

        let two = result
            .tests
            .iter()
            .find(|test| test.id.as_str() == "TEST-LIB-TWO-TARGETS")
            .unwrap();
        assert_eq!(two.targets.len(), 2);
        assert_eq!(
            two.execution.suite,
            Some(TestSuite {
                kind: "lib".to_owned(),
                name: None,
            })
        );

        let three = result
            .tests
            .iter()
            .find(|test| test.id.as_str() == "TEST-INTEGRATION-THREE-TARGETS")
            .unwrap();
        assert_eq!(three.targets.len(), 3);
        assert_eq!(
            three.execution.suite,
            Some(TestSuite {
                kind: "integration".to_owned(),
                name: Some("three_targets".to_owned()),
            })
        );

        assert!(
            !result
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.code == "E-SCAN-005"),
            "declaring 1, 2, or 3 distinct targets must never be reported as a duplicate, \
             for a lib test or a Cargo integration test alike, regardless of `@vtest.kind`: {:?}",
            result.diagnostics
        );
    }

    /// 本冊 §4.2「許容された複数 `target` 内でも同じ TargetRef の重複は
    /// E-SCAN-005 とする」(DS-497) — a literal duplicate spelling within a
    /// Test's declared targets is still rejected regardless of how many
    /// targets are declared or what execution form the Test has.
    /// `@vtest.kind` is `unit-normal` (the value the built-in §14.1/§14.3
    /// Form actually outputs) to also show the rejection is not
    /// kind-dependent.
    /// @vtest.id TEST-SCAN-DUPLICATE-TARGET-VALUE-REJECTED
    /// @vtest.covers VO-SCAN-DUPLICATE-TARGETREF-WITHIN-MULTIPLE
    /// @vtest.target crates/vtest-scan/src/lib.rs::scan_project
    /// @vtest.intent verifies a literal duplicate target declaration within a Test's declared targets is rejected as E-SCAN-005, regardless of @vtest.kind
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
/// @vtest.kind unit-normal
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
                    .is_some_and(|location| location.locator == "same_target_twice")
        }));
        assert!(!result
            .tests
            .iter()
            .any(|test| test.id.as_str() == "TEST-INTEGRATION-DUPLICATE"));
    }

    /// 本冊 §4.2「1行1キー。`covers` と `related` の値はカンマ区切りで
    /// 複数指定できる」。
    /// @vtest.id TEST-SCAN-COVERS-RELATED-COMMA-SEPARATED
    /// @vtest.covers VO-SCAN-COVERS-RELATED-COMMA-SEPARATED
    /// @vtest.target crates/vtest-scan/src/lib.rs::scan_project
    /// @vtest.intent verifies @vtest.covers and @vtest.related accept a comma-separated list of multiple values
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
    /// @vtest.id TEST-SCAN-CASE-RELATED-REPEATED-LINES
    /// @vtest.covers VO-SCAN-CASE-RELATED-REPEATED-KEY-LINES
    /// @vtest.target crates/vtest-scan/src/lib.rs::scan_project
    /// @vtest.intent verifies @vtest.case and @vtest.related accept repeated key lines, each contributing one value, in declaration order
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
    /// @vtest.id TEST-SCAN-SRC-ID-ON-TEST-CONSTRUCT-REJECTED
    /// @vtest.covers VO-SCAN-UNKNOWN-ANNOTATION-KEY-E-SCAN-006
    /// @vtest.target crates/vtest-scan/src/lib.rs::scan_project
    /// @vtest.intent verifies a src-id (source-target-key) declared on a Test construct is rejected as E-SCAN-006 and the Test does not materialize
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
                    .is_some_and(|location| location.locator == "misplaced")
        }));
        assert!(!result
            .tests
            .iter()
            .any(|test| test.id.as_str() == "TEST-MISPLACED-SRC-ID"));
    }

    /// 本冊 §4.2「表面2で、`@vtest.` で始まるが source-target-key を
    /// 持たない行（test-key を含む）は警告 W-SCAN-105 とする」。
    /// @vtest.id TEST-SCAN-NON-TEST-ITEM-TEST-KEY-WARNS
    /// @vtest.covers VO-SCAN-NON-TEST-UNKNOWN-KEY-W-SCAN-105
    /// @vtest.target crates/vtest-scan/src/lib.rs::scan_project
    /// @vtest.intent verifies a non-Test item's doc comment carrying a test-key (@vtest.id) produces only a W-SCAN-105 warning, no error
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
                    .is_some_and(|location| location.locator == "helper")
        }));
        assert!(!result.diagnostics.iter().any(|diagnostic| {
            diagnostic.is_error()
                && diagnostic
                    .location
                    .as_ref()
                    .is_some_and(|location| location.locator == "helper")
        }));
    }

    /// 本冊 §4.2「`src-id` は表面2でも反復不可であり...このときいずれの
    /// 宣言値も採用せず、当該Source TargetのSRC IDは無しとして扱う
    /// （どちらかを推測で選ばない）」。
    /// @vtest.id TEST-SCAN-DUPLICATE-SRC-ID-NEITHER-VALUE-ADOPTED
    /// @vtest.covers VO-SCAN-DUPLICATE-KEY-E-SCAN-005
    /// @vtest.target crates/vtest-scan/src/lib.rs::scan_project
    /// @vtest.intent verifies a repeated @vtest.src-id on one Source Target is rejected as E-SCAN-005 and neither declared value is adopted (src_id is None)
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
                    .is_some_and(|location| location.locator == "helper")
        }));
        let helper = result
            .sources
            .iter()
            .find(|source| source.locator.value == "src/lib.rs::helper")
            .unwrap();
        assert!(helper.src_id.is_none());
    }

    /// 表面2の正常経路: 反復のない単一の `@vtest.src-id` は認識され、
    /// 診断を生じない。
    /// @vtest.id TEST-SCAN-NON-TEST-ITEM-DECLARES-PERMANENT-SRC-ID
    /// @vtest.covers VO-SCAN-SRC-ID-DECLARED-ON-IMPLEMENTATION
    /// @vtest.target crates/vtest-scan/src/lib.rs::scan_project
    /// @vtest.intent verifies a single, non-repeated @vtest.src-id on an implementation function is recognized as that Source Target's src_id with no diagnostic
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
            .find(|source| source.locator.value == "src/lib.rs::helper")
            .unwrap();
        assert_eq!(
            helper.src_id.as_ref().map(SrcId::as_str),
            Some("SRC-HELPER")
        );
        assert!(!result.diagnostics.iter().any(|diagnostic| {
            diagnostic
                .location
                .as_ref()
                .is_some_and(|location| location.locator == "helper")
        }));
    }

    /// 本冊 §5.1手順5・基本仕様§9.2「恒久SRC IDを使用する場合、adapter境界を
    /// 越えてrepository全体で一意でなければならない。同一SRC IDの複数宣言を
    /// 曖昧参照として受理しない」。2件の異なるSource Targetが同じ恒久SRC ID
    /// を宣言した場合はE-SCAN-011とし（本冊:877・901）、どのTestからも
    /// 参照されていなくても索引構築時点で検出する。
    /// @vtest.id TEST-SCAN-COLLIDING-PERMANENT-SRC-ID-REJECTED
    /// @vtest.covers VO-SCAN-COLLIDING-PERMANENT-SRC-ID-E-SCAN-011
    /// @vtest.target crates/vtest-scan/src/lib.rs::scan_project
    /// @vtest.intent verifies two Source Targets declaring the same permanent src-id produce E-SCAN-011 naming the colliding id and pointing at a declaring Source Target
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
        // レビュー round 2 項目【K-1】: コードだけでなく、衝突している
        // 恒久SRC ID自体（`SRC-SHARED`）と、診断が宣言側のSource Target
        // （2件のうち索引構築順で先に見つかったもの）を指していることを
        // 断言する。単に E-SCAN-011 が"どこかで"出たことだけでは、
        // 別の恒久SRC IDや無関係な条件と取り違えていないことを保証しない。
        let diagnostic = result
            .diagnostics
            .iter()
            .find(|diagnostic| diagnostic.code == "E-SCAN-011")
            .unwrap_or_else(|| panic!("diagnostics: {:?}", result.diagnostics));
        assert!(
            diagnostic.message.contains("SRC-SHARED"),
            "diagnostic must name the colliding SRC ID: {diagnostic:?}"
        );
        assert_eq!(
            diagnostic
                .location
                .as_ref()
                .map(|location| location.locator.as_str()),
            Some("helper_one"),
            "diagnostic must point at a declaring Source Target: {diagnostic:?}"
        );
    }

    /// 本冊 §6.1.1「Testの宣言target集合は解決後のcanonical Source Target
    /// 単位で一意でなければならない。綴りの異なる複数の宣言が同一の
    /// canonical Source Targetへ解決する場合は重複targetとしてE-SCAN-005と
    /// する」。locator形式の宣言と、同じSource Targetを指すSRC ID形式の
    /// 宣言は綴りが異なるが、解決後は同一canonical Source Targetになる。
    /// @vtest.id TEST-SCAN-LOCATOR-AND-SRC-ID-ALIAS-COLLIDE
    /// @vtest.covers VO-SCAN-DUPLICATE-KEY-E-SCAN-005
    /// @vtest.target crates/vtest-scan/src/lib.rs::scan_project
    /// @vtest.intent verifies a locator-form target declaration and a src-id-form declaration resolving to the same canonical Source Target under different spellings are rejected as E-SCAN-005
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
/// @vtest.kind unit-normal
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
                        .is_some_and(|location| location.locator == "aliased_target")
            }),
            "diagnostics: {:?}",
            result.diagnostics
        );
    }

    /// 本冊:955/959/961（§6.1）・pr3-decisions.md Owner裁定2「架空locator
    /// を作らない」: 解決できない target は「対象なし」の fail-closed 終端
    /// 状態であり、後段が任意の候補で埋めて「解決済み」を偽装してはならな
    /// い。`@vtest.target` に `path::item-path` へ構文解析できない値
    /// （`::` を含まない自由記述）を与えると、この Test 自身の locator
    /// （`tests/unresolvable_target.rs::declares_unparseable_target`）で
    /// 肩代わりして解決済みにする旧挙動があった（fail-open。BLOCKER 3）。
    /// その後、実在しない `<unresolvable @vtest.target ...>` という架空
    /// locatorをcanonical modelに作る形で「直した」ものも Owner裁定2で
    /// 否定された。現在は adapter が宣言値をそのまま opaque
    /// `TargetRef::Locator.value` として運ぶだけであり（捏造なし）、core の
    /// `resolve_targets` が実在するSource Targetとの完全一致を求める通常の
    /// 「0件ヒット」経路として E-SCAN-004 を発行することを断言する。
    /// @vtest.id TEST-SCAN-UNPARSEABLE-TARGET-NOT-SELF-RESOLVED
    /// @vtest.covers VO-SCAN-TARGET-UNRESOLVABLE-E-SCAN-004
    /// @vtest.target crates/vtest-scan/src/lib.rs::scan_project
    /// @vtest.intent verifies an unparseable @vtest.target value is carried verbatim as an opaque locator (not fabricated or self-referenced) and reported as E-SCAN-004
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
        assert_eq!(test.targets.len(), 1, "single-target declaration");
        let TargetRef::Locator(locator) = &test.targets[0] else {
            panic!("expected a Locator target, got {:?}", test.targets[0]);
        };
        // adapterは捏造しない: opaque valueは宣言値そのもの、この Test 自身
        // を指す自己参照locatorへ肩代わりされていない。
        assert_eq!(locator.value, "this is not a locator");
        assert_ne!(
            locator.value, "tests/unresolvable_target.rs::declares_unparseable_target",
            "an unresolvable target must not be silently filled in with the Test's own \
             self-referencing locator: {locator:?}"
        );
        assert!(
            result.diagnostics.iter().any(|diagnostic| {
                diagnostic.code == "E-SCAN-004"
                    && diagnostic
                        .location
                        .as_ref()
                        .is_some_and(|location| location.locator == "declares_unparseable_target")
                    // 要確認C（PR #26 review round 2）: メッセージに元の
                    // 宣言値が残っていることを断言する — 利用者が「何が
                    // 不正だったか」を診断から追える最も分かりやすい例。
                    && diagnostic.message.contains("this is not a locator")
            }),
            "diagnostics: {:?}",
            result.diagnostics
        );
    }

    /// 本冊 §4.2「doc comment 内の `@vtest.` を含まない行は自由記述として
    /// 無視する」。
    /// @vtest.id TEST-SCAN-FREE-TEXT-LINES-IGNORED
    /// @vtest.covers VO-SCAN-FREE-TEXT-LINES-IGNORED
    /// @vtest.target crates/vtest-scan/src/lib.rs::scan_project
    /// @vtest.intent verifies doc comment lines not starting with @vtest. are ignored as free-form prose interleaved with declarations
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

    /// ROOT-049 / DS-1666: `targets` の宣言は Test 成立性の必須条件では
    /// ない（旧 DS-1621 の `targets ≥ 1` 条項は撤去された）。
    /// `@vtest.target` を1件も宣言しない Test は E-SCAN-007 にならず、
    /// core 中立の必須 metadata（id・covers ≥ 1・intent）さえ揃えば
    /// `TestEntity` として具体化される。その `target_binding` を
    /// `NO_EVIDENCE`（DS-1664）にする判定は verify 側の責務であり、
    /// scan/adapter 層の観測範囲ではない。
    /// @vtest.id TEST-SCAN-MISSING-TARGET-ANNOTATION-ACCEPTED
    /// @vtest.covers VO-SCAN-N-TARGETS-UNBOUNDED-KIND-INDEPENDENT
    /// @vtest.target crates/vtest-scan/src/lib.rs::scan_project
    /// @vtest.intent verifies a Test declaring zero @vtest.target lines materializes as a TestEntity (not E-SCAN-007) once core-neutral required metadata is present
    #[test]
    fn missing_target_annotation_is_accepted() {
        let root = fixture();
        fs::write(
            root.join("tests/no_target.rs"),
            r#"
/// @vtest.id TEST-NO-TARGET
/// @vtest.covers VO-ADD
/// @vtest.intent target declaration is optional
#[test]
fn no_target() {}
"#,
        )
        .unwrap();
        let result = scan_project(&root).unwrap();
        assert!(
            !result.diagnostics.iter().any(|diagnostic| {
                diagnostic.code == "E-SCAN-007"
                    && diagnostic
                        .location
                        .as_ref()
                        .is_some_and(|location| location.locator == "no_target")
            }),
            "diagnostics: {:?}",
            result.diagnostics
        );
        assert!(result
            .tests
            .iter()
            .any(|test| test.id.as_str() == "TEST-NO-TARGET"));
    }

    /// DS-1666/DES-229「E-SCAN-007は必須metadata（core中立: id / covers ≥ 1
    /// / intent）の欠落を意味し、targetは必須キーではない」。DS-538・
    /// 本冊:990-1005「targetロケータ／SRC IDの解決失敗（E-SCAN-004）は
    /// coreの単一経路（`resolve_targets`）が所有する」。`@vtest.target` に
    /// 空文字列を宣言した場合、それは「宣言が無い」ことにはならない
    /// （`missing_target_annotation_is_accepted` とは異なる経路）ため
    /// adapterはE-SCAN-007で早期returnせず、core側のtarget解決へ素通し
    /// する。空文字列はどのSource Targetロケータとも一致しないため、
    /// core の「0件ヒット」経路がE-SCAN-004を発行する。
    /// @vtest.id TEST-SCAN-EMPTY-TARGET-VALUE-RESOLVES-TO-E-SCAN-004
    /// @vtest.covers VO-SCAN-TARGET-UNRESOLVABLE-E-SCAN-004
    /// @vtest.target crates/vtest-scan/src/lib.rs::scan_project
    /// @vtest.intent verifies an empty @vtest.target value is treated as a declared-but-unresolvable target (E-SCAN-004), not a missing declaration (E-SCAN-007)
    #[test]
    fn empty_string_target_value_resolves_through_core_to_e_scan_004_not_e_scan_007() {
        let root = fixture();
        fs::write(
            root.join("tests/empty_target.rs"),
            r#"
/// @vtest.id TEST-EMPTY-TARGET
/// @vtest.covers VO-ADD
/// @vtest.target
/// @vtest.intent an empty target value is not a missing declaration
#[test]
fn empty_target() {}
"#,
        )
        .unwrap();
        let result = scan_project(&root).unwrap();
        let at_empty_target = |diagnostic: &&Diagnostic| {
            diagnostic
                .location
                .as_ref()
                .is_some_and(|location| location.locator == "empty_target")
        };
        assert!(
            !result
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.code == "E-SCAN-007" && at_empty_target(&diagnostic)),
            "an empty @vtest.target value must not be reported as E-SCAN-007: {:?}",
            result.diagnostics
        );
        let e_scan_004 = result
            .diagnostics
            .iter()
            .filter(|diagnostic| diagnostic.code == "E-SCAN-004" && at_empty_target(diagnostic))
            .collect::<Vec<_>>();
        assert_eq!(
            e_scan_004.len(),
            1,
            "an empty @vtest.target value must resolve through core to exactly one \
             E-SCAN-004: {:?}",
            result.diagnostics
        );
        assert!(result
            .tests
            .iter()
            .any(|test| test.id.as_str() == "TEST-EMPTY-TARGET"));
    }

    /// 本冊 §4.4 / §11.1.1: core が中立に要求する必須 metadata（`id` /
    /// `covers ≥ 1`）の欠落も E-SCAN-007 になる。
    /// @vtest.id TEST-SCAN-MISSING-ID-AND-COVERS-REJECTED
    /// @vtest.covers VO-SCAN-MISSING-REQUIRED-METADATA-E-SCAN-007
    /// @vtest.target crates/vtest-scan/src/lib.rs::scan_project
    /// @vtest.intent verifies a Test construct missing @vtest.id, and one missing @vtest.covers, are each rejected as E-SCAN-007
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
                    .is_some_and(|location| location.locator == "no_id")
        }));
        assert!(result.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "E-SCAN-007"
                && diagnostic
                    .location
                    .as_ref()
                    .is_some_and(|location| location.locator == "no_covers")
        }));
    }

    /// 本冊:567（§4.4）・基本:412（§12）・別紙C:81（§18.3.1）: `covers` を
    /// 持たない（0 件の）Test は E-SCAN-007 とし `TestDraft` を生成しない
    /// （BLOCKER 2、PR #26 review round 1）。`@vtest.covers` の値自体は
    /// 非空文字列（`,`）だが、カンマ区切りで分割すると VO ID が1件も
    /// 残らない — 旧挙動は E-SCAN-007 を出しつつ `covers: []` の
    /// `TestEntity` を管理対象集合へ混入させていた（fail-open）。
    /// @vtest.id TEST-SCAN-COVERS-ZERO-VO-IDS-REJECTED
    /// @vtest.covers VO-SCAN-MISSING-REQUIRED-METADATA-E-SCAN-007
    /// @vtest.target crates/vtest-scan/src/lib.rs::scan_project
    /// @vtest.intent verifies a non-empty @vtest.covers value that splits to zero VO ids is rejected as E-SCAN-007 and produces no TestEntity
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
                        .is_some_and(|location| location.locator == "empty_covers")
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
    /// @vtest.id TEST-SCAN-EDIT-COVERS-NO-VO-PREFIX-ENFORCED
    /// @vtest.covers VO-SCAN-ID-FORMAT-NOT-ENFORCED
    /// @vtest.target crates/vtest-scan/src/operations.rs::edit_test
    /// @vtest.intent verifies edit_test accepts a covers value referencing a real VO whose id lacks a VO- prefix
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

    /// Owner裁定1（pr3-decisions.md）「後段が代表1件を推測選択しては
    /// ならない」: Test ID が衝突している状態で `edit_test` を呼んでも、
    /// 衝突した construct のどれかを黙って編集対象に選ばない。
    /// @vtest.id TEST-SCAN-EDIT-REJECTS-COLLIDING-TEST-ID
    /// @vtest.covers VO-SCAN-EDIT-COLLIDING-TEST-ID-NO-REPRESENTATIVE
    /// @vtest.target crates/vtest-scan/src/operations.rs::edit_test
    /// @vtest.intent verifies edit_test fails closed with E-OP-002 (naming the collision count) rather than silently picking one of the colliding constructs to edit
    #[test]
    fn edit_test_rejects_a_colliding_test_id() {
        let root = fixture();
        fs::write(
            root.join("tests/collision_edit.rs"),
            r#"
/// @vtest.id TEST-EDIT-COLLISION
/// @vtest.covers VO-ADD
/// @vtest.target src/lib.rs::add
/// @vtest.intent first construct declaring a colliding Test ID
#[test]
fn edit_collision_first() {}

/// @vtest.id TEST-EDIT-COLLISION
/// @vtest.covers VO-ADD
/// @vtest.target src/lib.rs::add
/// @vtest.intent second construct declaring the same colliding Test ID
#[test]
fn edit_collision_second() {}
"#,
        )
        .unwrap();
        let mut set = BTreeMap::new();
        set.insert("intent".to_owned(), FormValue::Scalar("changed".to_owned()));
        let result = edit_test(&root, "TEST-EDIT-COLLISION", None, &set, None, true);
        let error = result.expect_err("edit of a colliding Test ID must fail closed");
        assert_eq!(error.code, "E-OP-002");
        assert!(
            error.message.contains("2 Test constructs"),
            "error should surface how many constructs collide: {}",
            error.message
        );
    }

    /// 複数target許容のカーディナリティ検査はもはや実行形態に条件付けられて
    /// いない — `operations.rs`の`validate_desired_test`は`current.
    /// test_target`/`desired.targets.len()`を一切見ない無条件の1件以上検査
    /// になった（正本監査がREQ-150/SPEC-085に上位の根拠を見つけられず、旧
    /// `TestTarget::IntegrationTest`限定条件を撤去した — 下記
    /// `edit_test_allows_multiple_targets_for_a_lib_test_regardless_of_
    /// kind_string`を参照）。このテストは、その無条件の許容がCargo
    /// integration test（`fixture()`のTEST-ADDが置かれる`tests/calc.rs`）
    /// についても成り立つことを確認する — `kind`を`integration`を含まない
    /// 値へ`--set`しても複数targetへの編集が通る。
    /// @vtest.id TEST-SCAN-EDIT-MULTI-TARGET-CARGO-INTEGRATION-KIND-INDEPENDENT
    /// @vtest.covers VO-SCAN-N-TARGETS-UNBOUNDED-KIND-INDEPENDENT
    /// @vtest.target crates/vtest-scan/src/operations.rs::edit_test
    /// @vtest.intent verifies edit_test allows a Cargo integration test to declare multiple targets even when @vtest.kind is set to a value not starting with integration
    #[test]
    fn edit_test_allows_multiple_targets_for_a_cargo_integration_test_regardless_of_kind_string() {
        let root = fixture();
        fs::write(
            root.join("src/lib.rs"),
            "pub fn add(a: i32, b: i32) -> i32 { a + b }\npub fn subtract(a: i32, b: i32) -> i32 { a - b }\n",
        )
        .unwrap();
        let mut set = BTreeMap::new();
        set.insert(
            "targets".to_owned(),
            FormValue::List(vec![
                "src/lib.rs::add".to_owned(),
                "src/lib.rs::subtract".to_owned(),
            ]),
        );
        set.insert(
            "kind".to_owned(),
            FormValue::Scalar("unit-normal".to_owned()),
        );
        let result = edit_test(&root, "TEST-ADD", None, &set, None, true);
        assert!(
            result.is_ok(),
            "a Cargo integration test must accept multiple targets even when `kind` does not \
             start with `integration`: {:?}",
            result.err()
        );
    }

    /// 上記の裏側: REQ-150/SPEC-085/DS-1618 はカーディナリティに執行形態
    /// 条件を課さないため、Cargo Integration Test ではない（lib test の）
    /// Test への Structured Edit も複数 target への編集を許容する。旧実装は
    /// これを `current.test_target` で拒否していた（Owner裁定3、PR #26
    /// review round 5）— 正本監査が上位の根拠を見つけられず撤去した後の
    /// 挙動をロックインする回帰テスト。
    /// @vtest.id TEST-SCAN-EDIT-MULTI-TARGET-LIB-KIND-INDEPENDENT
    /// @vtest.covers VO-SCAN-N-TARGETS-UNBOUNDED-KIND-INDEPENDENT
    /// @vtest.target crates/vtest-scan/src/operations.rs::edit_test
    /// @vtest.intent verifies edit_test allows a lib test (not a Cargo integration test) to declare multiple targets, with no execution-form condition on cardinality
    #[test]
    fn edit_test_allows_multiple_targets_for_a_lib_test_regardless_of_kind_string() {
        let root = fixture();
        fs::write(
            root.join("src/lib.rs"),
            r#"pub fn add(a: i32, b: i32) -> i32 { a + b }
pub fn subtract(a: i32, b: i32) -> i32 { a - b }

/// @vtest.id TEST-LIB-FAKE-INTEGRATION-KIND
/// @vtest.covers VO-ADD
/// @vtest.target src/lib.rs::add
/// @vtest.intent editing this lib test to declare a second target must succeed
/// @vtest.kind integration-normal
#[test]
fn lib_test() {}
"#,
        )
        .unwrap();
        let mut set = BTreeMap::new();
        set.insert(
            "targets".to_owned(),
            FormValue::List(vec![
                "src/lib.rs::add".to_owned(),
                "src/lib.rs::subtract".to_owned(),
            ]),
        );
        let result = edit_test(
            &root,
            "TEST-LIB-FAKE-INTEGRATION-KIND",
            None,
            &set,
            None,
            true,
        );
        assert!(
            result.is_ok(),
            "a lib test must be allowed multiple targets — cardinality has no execution-form \
             condition: {:?}",
            result.err()
        );
    }

    /// 別紙A §14.3「§14.1との差分はこの2点であり、他は同一」: `--set
    /// test_kind=...`経由のTest編集（`operations.rs`の`DesiredTest::
    /// apply_sets`）は、targetsの件数にかかわらず常に`unit-{test_kind}`を
    /// 生成しなければならない（前コミットで修正済み）。本テストは、その
    /// 生成修正と本コミットの判定基準修正を組み合わせた edit_test
    /// 経路全体で、複数targetを持つCargo integration testの編集が
    /// 通り、かつ生成される`@vtest.kind`が仕様どおりであることを確認する。
    #[test]
    fn edit_test_set_test_kind_always_generates_the_unit_prefix_regardless_of_target_count() {
        let root = fixture();
        fs::write(
            root.join("src/lib.rs"),
            "pub fn add(a: i32, b: i32) -> i32 { a + b }\npub fn subtract(a: i32, b: i32) -> i32 { a - b }\n",
        )
        .unwrap();
        let mut set = BTreeMap::new();
        set.insert(
            "targets".to_owned(),
            FormValue::List(vec![
                "src/lib.rs::add".to_owned(),
                "src/lib.rs::subtract".to_owned(),
            ]),
        );
        set.insert(
            "test_kind".to_owned(),
            FormValue::Scalar("normal".to_owned()),
        );
        let result = edit_test(&root, "TEST-ADD", None, &set, None, true)
            .expect("editing an integration test's targets and test_kind together must succeed");
        assert!(
            result.rendered.contains("@vtest.kind unit-normal"),
            "expected `unit-normal`, got rendered output: {}",
            result.rendered
        );
        assert!(
            !result.rendered.contains("@vtest.kind integration-"),
            "must not generate an `integration-` prefix (rejected criterion, \
             pr3-decisions.md Owner裁定3): {}",
            result.rendered
        );
    }

    /// 同上の`apply_complete_answers`（`--answers`経由の編集）版。
    /// `rust-integration` built-in Formで編集しても、生成される
    /// `@vtest.kind`は`rust-unit-function`と同じ`unit-{test_kind}`で
    /// なければならない（別紙A §14.1/§14.3）。
    #[test]
    fn edit_test_with_rust_integration_answers_generates_the_unit_prefix() {
        let root = fixture();
        fs::write(
            root.join("src/lib.rs"),
            "pub fn add(a: i32, b: i32) -> i32 { a + b }\npub fn subtract(a: i32, b: i32) -> i32 { a - b }\n",
        )
        .unwrap();
        let mut answers = BTreeMap::new();
        answers.insert(
            "targets".to_owned(),
            FormValue::List(vec![
                "src/lib.rs::add".to_owned(),
                "src/lib.rs::subtract".to_owned(),
            ]),
        );
        answers.insert(
            "covers".to_owned(),
            FormValue::List(vec!["VO-ADD".to_owned()]),
        );
        answers.insert(
            "behavior".to_owned(),
            FormValue::Scalar("combines calculator operations".to_owned()),
        );
        answers.insert(
            "test_kind".to_owned(),
            FormValue::Scalar("normal".to_owned()),
        );
        answers.insert(
            "input".to_owned(),
            FormValue::Scalar("two integers".to_owned()),
        );
        answers.insert(
            "expect".to_owned(),
            FormValue::Scalar("consistent arithmetic".to_owned()),
        );
        answers.insert("fn_name".to_owned(), FormValue::Scalar("adds".to_owned()));
        answers.insert(
            "file".to_owned(),
            FormValue::Scalar("tests/calc.rs".to_owned()),
        );
        let supplied = FormAnswers {
            form: "rust-integration".to_owned(),
            answers,
        };
        let result = edit_test(
            &root,
            "TEST-ADD",
            Some(&supplied),
            &BTreeMap::new(),
            None,
            true,
        )
        .expect("editing via the rust-integration built-in Form must succeed");
        assert!(
            result.rendered.contains("@vtest.kind unit-normal"),
            "expected `unit-normal` (the value the built-in §14.1/§14.3 Form template \
             actually declares), got rendered output: {}",
            result.rendered
        );
        assert!(
            !result.rendered.contains("@vtest.kind integration-"),
            "must not generate an `integration-` prefix (rejected criterion, \
             pr3-decisions.md Owner裁定3): {}",
            result.rendered
        );
    }

    /// @vtest.id TEST-SCAN-RELATION-ID-ALIASES-CANNOT-DUPLICATE-ULID
    /// @vtest.covers VO-SCAN-RECORD-LOGICAL-ID-DUPLICATE-E-SCAN-010
    /// @vtest.target crates/vtest-scan/src/lib.rs::scan_project
    /// @vtest.intent verifies two relation records whose ids are the same ULID payload under bare and REL-prefixed spellings collide as a logical id duplicate (E-SCAN-010), with no adapter-attributed location, and the message names both ids and the file
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
        // record 層診断（document/VO/relation/approval）はどの adapter にも
        // 属さないため `SourceLocation` を持たない（修正1、CLAUDE.local.md
        // 「実在しない adapter id を捏造しない」原則）— `location` は
        // `None` のままで、対象識別性はメッセージ本文（両方の relation id と
        // レコードファイルの相対パス）で維持する。
        assert!(duplicates[0].location.is_none());
        assert!(
            duplicates[0].message.contains(".verify/rel/")
                && duplicates[0].message.contains(".yaml"),
            "message should identify the record file: {}",
            duplicates[0].message
        );
    }

    /// Regression test for the `known_ids`/`document_node_ids` merge bug: a
    /// relation's `from`/`to` must resolve against the corpus-wide upstream
    /// *node*-id index (DS-425/DS-429/DS-543), never against
    /// `.verify/doc/*.json` file *names* (BD-330/DES-585 — the upstream
    /// document file itself carries no field that identifies it; BD-318/
    /// DS-1660 place the entity id on the node, not the file). The fixture's
    /// document file is named `DOC-TEST` and declares one real node,
    /// `ROOT-001` (see `write_doc_test_fixture`): a relation naming the real
    /// node id must resolve, and one naming the file name must not — before
    /// this fix, `known_ids` held the file name and not the node id, so both
    /// directions were backwards (the file name resolved, the real node id
    /// did not).
    /// @vtest.id TEST-SCAN-RELATION-ENDPOINTS-RESOLVE-BY-NODE-ID-NOT-FILENAME
    /// @vtest.covers VO-SCAN-RELATION-ENDPOINTS-RESOLVE-BY-NODE-ID
    /// @vtest.target crates/vtest-scan/src/lib.rs::scan_project
    /// @vtest.intent verifies a relation naming a real upstream document node id resolves, and one naming the document file's name (not a declared node id) does not
    #[test]
    fn relation_endpoints_resolve_against_document_node_ids_not_file_names() {
        let root = fixture();

        let resolves_id = new_record_id();
        fs::write(
            root.join(format!(".verify/rel/{resolves_id}.yaml")),
            "id: RESOLVES\ntype: depends-on\nfrom: ROOT-001\nto: VO-ADD\ncreated: '2026-01-01'\n"
                .replace("RESOLVES", &resolves_id),
        )
        .unwrap();

        let dangling_id = new_record_id();
        fs::write(
            root.join(format!(".verify/rel/{dangling_id}.yaml")),
            "id: DANGLING\ntype: depends-on\nfrom: DOC-TEST\nto: VO-ADD\ncreated: '2026-01-01'\n"
                .replace("DANGLING", &dangling_id),
        )
        .unwrap();

        let result = scan_project(&root).unwrap();
        let e_scan_009_messages = result
            .diagnostics
            .iter()
            .filter(|diagnostic| diagnostic.code == "E-SCAN-009")
            .map(|diagnostic| diagnostic.message.as_str())
            .collect::<Vec<_>>();

        assert!(
            !e_scan_009_messages
                .iter()
                .any(|message| message.contains("ROOT-001")),
            "a relation `from: ROOT-001` names a real upstream node id and must resolve: {:?}",
            result.diagnostics
        );
        assert!(
            e_scan_009_messages
                .iter()
                .any(|message| message.contains("DOC-TEST")),
            "a relation `from: DOC-TEST` names a document *file name*, not a node id, \
             and must NOT resolve: {:?}",
            result.diagnostics
        );
    }

    /// @vtest.id TEST-SCAN-REPORTS-VO-AND-RELATION-INTEGRITY-DIAGNOSTICS
    /// @vtest.covers VO-SCAN-VO-RELATION-RECORD-INTEGRITY-DIAGNOSTICS
    /// @vtest.target crates/vtest-scan/src/lib.rs::scan_project
    /// @vtest.intent verifies scan_project reports E-SCAN-008/009/010, W-SCAN-102/103, and W-STORE-001 for the respective malformed VO/relation/approval fixtures, each diagnostic identifying its source via location or an embedded record path
    #[test]
    fn reports_vo_and_relation_integrity_diagnostics() {
        // 詳細設計 v0.1 §2.1 replaced the predecessor REQ/SPEC layers with
        // canonical doc/VO; this test used to also exercise the predecessor
        // SPEC-layer staleness check (`.verify/spec/`, W-SCAN-104) here.
        // That predecessor diagnostic has no current canonical basis (see
        // the W-SCAN-104 removal note further down this file) and was
        // removed outright rather than ported to a DOC-layer equivalent —
        // there is no `reports_document_content_hash_staleness` test to
        // move its assertion to. What remains here is the VO-layer and
        // Relation-layer integrity checks (E-SCAN-008/009/010, W-SCAN-102/
        // 103, W-STORE-001), which are unaffected by the doc/REQ/SPEC
        // migration.
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
        // record 層診断（document/VO/relation/approval、E-SCAN-008/009/010,
        // W-SCAN-102, W-STORE-001 等）はどの adapter にも属さないため
        // `SourceLocation` を持たない（修正1、CLAUDE.local.md「実在しない
        // adapter id を捏造しない」原則）。以前はここで全診断に
        // `location.is_some()` を求めていたが、それは reshape 直後の実装が
        // 架空の adapter id で `SourceLocation` を捏造していたことへの
        // 誤った断言だった。対象識別性は維持されるべき性質なので、
        // location か、メッセージへ埋め込まれた record ファイルパス
        // （`.yaml`）のどちらかを持つことを断言する。
        assert!(
            result.diagnostics.iter().all(|diagnostic| {
                diagnostic.location.is_some() || diagnostic.message.contains(".yaml")
            }),
            "every scanner diagnostic must identify its canonical source, either via \
             `location` (adapter-discovered constructs) or an embedded record-file \
             path in the message (record-layer diagnostics): {:?}",
            result.diagnostics
        );
    }

    // レビュー round 2 項目【L】の回帰テスト群。「存在しない」（正常な
    // 空リポジトリ）と「存在するが読めない」（fail-closed で中断すべき）を
    // 区別する。ディレクトリを通常ファイルへ差し替えることで、権限エラー
    // と同じ `io::ErrorKind` 非 `NotFound` 経路を移植性のある形で再現する。

    /// @vtest.id TEST-SCAN-MISSING-RELATION-DIR-IS-NO-RELATIONS
    /// @vtest.covers VO-SCAN-RECORD-DIR-MISSING-VS-UNREADABLE
    /// @vtest.target crates/vtest-scan/src/lib.rs::scan_project
    /// @vtest.intent verifies a missing .verify/rel directory is treated as zero relations, producing no relation diagnostics, rather than aborting the scan
    #[test]
    fn missing_relation_dir_is_treated_as_no_relations() {
        let root = fixture();
        fs::remove_dir_all(root.join(".verify/rel")).unwrap();
        let result = scan_project(&root).expect("a missing relation dir must not abort the scan");
        assert!(
            !result.diagnostics.iter().any(
                |diagnostic| diagnostic.code == "E-SCAN-009" || diagnostic.code == "E-SCAN-010"
            ),
            "a missing relation dir must not itself produce relation diagnostics: {:?}",
            result.diagnostics
        );
    }

    /// @vtest.id TEST-SCAN-UNREADABLE-RELATION-DIR-ABORTS-FAIL-CLOSED
    /// @vtest.covers VO-SCAN-RECORD-DIR-MISSING-VS-UNREADABLE
    /// @vtest.target crates/vtest-scan/src/lib.rs::scan_project
    /// @vtest.intent verifies a .verify/rel path that exists but cannot be read as a directory aborts scan_project with ScanError::Io rather than being treated as empty
    #[test]
    fn unreadable_relation_dir_aborts_scan_fail_closed() {
        let root = fixture();
        let relation_dir = root.join(".verify/rel");
        fs::remove_dir_all(&relation_dir).unwrap();
        fs::write(&relation_dir, b"not a directory").unwrap();
        let error =
            scan_project(&root).expect_err("an unreadable relation dir must abort the scan");
        assert!(
            matches!(error, ScanError::Io { .. }),
            "expected ScanError::Io, got: {error:?}"
        );
    }

    /// @vtest.id TEST-SCAN-MISSING-APPROVALS-DIR-IS-NO-APPROVALS
    /// @vtest.covers VO-SCAN-RECORD-DIR-MISSING-VS-UNREADABLE
    /// @vtest.target crates/vtest-scan/src/lib.rs::scan_project
    /// @vtest.intent verifies a missing .verify/approvals directory is treated as zero approvals, producing no approval diagnostics
    #[test]
    fn missing_approvals_dir_is_treated_as_no_approvals() {
        let root = fixture();
        fs::remove_dir_all(root.join(".verify/approvals")).unwrap();
        let result = scan_project(&root).expect("a missing approvals dir must not abort the scan");
        assert!(
            !result
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.code == "E-SCAN-010"
                    && diagnostic.message.contains("approval")),
            "a missing approvals dir must not itself produce approval diagnostics: {:?}",
            result.diagnostics
        );
    }

    /// @vtest.id TEST-SCAN-UNREADABLE-APPROVALS-DIR-ABORTS-FAIL-CLOSED
    /// @vtest.covers VO-SCAN-RECORD-DIR-MISSING-VS-UNREADABLE
    /// @vtest.target crates/vtest-scan/src/lib.rs::scan_project
    /// @vtest.intent verifies a .verify/approvals path that exists but cannot be read as a directory aborts scan_project with ScanError::Io rather than being treated as empty
    #[test]
    fn unreadable_approvals_dir_aborts_scan_fail_closed() {
        let root = fixture();
        let approvals_dir = root.join(".verify/approvals");
        fs::remove_dir_all(&approvals_dir).unwrap();
        fs::write(&approvals_dir, b"not a directory").unwrap();
        let error =
            scan_project(&root).expect_err("an unreadable approvals dir must abort the scan");
        assert!(
            matches!(error, ScanError::Io { .. }),
            "expected ScanError::Io, got: {error:?}"
        );
    }

    // W-SCAN-104 (predecessor content_hash-vs-file staleness diagnostic) and
    // `doc.roots`-based root declaration no longer exist in this crate:
    // neither the detailed_spec §5.4 diagnostic table (DS-535..DS-1650) nor
    // any other node in specification.json assigns `W-SCAN-104` to
    // anything, and DS-1646 replaces config-declared roots with structural
    // `root`-layer membership ("設定による除外指定は持たない"). The
    // predecessor tests exercising both (`reports_document_content_hash_
    // staleness`, `add_doc_root`) are removed outright rather than ported —
    // there is no current canonical condition left for them to assert.

    /// DS-546 (E-SCAN-012, VO half): "VOの `derives_from` が存在しない
    /// documentを参照…" (`validate_vo_document_references`, above). Every
    /// other VO fixture in this crate (`valid_vo`) resolves its
    /// `derives_from` target against `ROOT-001` (written into every
    /// `fixture()` by `write_doc_test_fixture`), so this branch had no test
    /// making it fire — this locks it in, mirroring the document-side
    /// dangling-reference test immediately below.
    /// @vtest.id TEST-SCAN-REPORTS-VO-DERIVES-FROM-DANGLING-REFERENCE
    /// @vtest.covers VO-SCAN-DERIVES-FROM-DANGLING-E-SCAN-012
    /// @vtest.target crates/vtest-scan/src/lib.rs::scan_project
    /// @vtest.intent verifies a VO's derives_from entry naming a non-existent document node produces E-SCAN-012 naming both the VO and the missing node
    #[test]
    fn reports_vo_derives_from_dangling_reference() {
        let root = fixture();
        let vo_text =
            valid_vo("VO-DANGLING", "null").replace("doc: ROOT-001", "doc: DOC-NODE-NOT-FOUND");
        assert!(
            vo_text.contains("doc: DOC-NODE-NOT-FOUND"),
            "fixture template must actually contain the substring being replaced"
        );
        fs::write(root.join(".verify/vo/VO-DANGLING.yaml"), vo_text).unwrap();

        let result = scan_project(&root).unwrap();
        assert!(
            result.diagnostics.iter().any(|diagnostic| {
                diagnostic.code == "E-SCAN-012"
                    && diagnostic.message.contains("VO-DANGLING")
                    && diagnostic.message.contains("DOC-NODE-NOT-FOUND")
            }),
            "diagnostics: {:?}",
            result.diagnostics
        );
    }

    /// @vtest.id TEST-SCAN-REPORTS-DOCUMENT-DERIVES-FROM-DANGLING-REFERENCE
    /// @vtest.covers VO-SCAN-DERIVES-FROM-DANGLING-E-SCAN-012
    /// @vtest.target crates/vtest-scan/src/lib.rs::scan_project
    /// @vtest.intent verifies an upstream document node's derives_from entry naming a non-existent document node produces E-SCAN-012, without also firing E-SCAN-016 (the node itself is not orphaned by a dangling edge)
    #[test]
    fn reports_document_derives_from_dangling_reference() {
        let root = fixture();
        let layout = VerifyLayout::new(&root);
        // DS-1660's own illustrative non-existent id: `SPEC-999` never
        // appears in any document this fixture writes. A dangling
        // `derives_from` entry is still an edge — the node it belongs to is
        // not orphaned — so E-SCAN-012 and E-SCAN-016 must not both fire.
        let file = DocumentFile {
            schema_version: "0.1".to_owned(),
            root: Vec::new(),
            request: vec![SentenceNode {
                id: DocumentId::new("R-901"),
                statement: "dangling reference fixture".to_owned(),
                description: None,
                derives_from: vec![DocumentId::new("SPEC-999")],
                cites: None,
                source: fixture_node_source(),
            }],
            require: Vec::new(),
            spec: Vec::new(),
            detailed_spec: Vec::new(),
            basic_design: Vec::new(),
            design: Vec::new(),
        };
        write_document_file(&layout, "DOC-DANGLING", &file).unwrap();

        let result = scan_project(&root).unwrap();
        let dangling = result
            .diagnostics
            .iter()
            .filter(|diagnostic| {
                diagnostic.code == "E-SCAN-012" && diagnostic.message.contains("R-901")
            })
            .collect::<Vec<_>>();
        assert_eq!(dangling.len(), 1, "diagnostics: {:?}", result.diagnostics);
        assert!(dangling[0].message.contains("SPEC-999"));
        assert!(
            !result
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.code == "E-SCAN-016"
                    && diagnostic.message.contains("R-901")),
            "a node with a (dangling) derives_from entry is not orphaned; \
             E-SCAN-012 and E-SCAN-016 must not both fire for it: {:?}",
            result.diagnostics
        );
    }

    /// DS-1677/DS-1676 (E-SCAN-010): the same node id defined across two
    /// different document files must be reported as a collision, and a
    /// `derives_from` edge naming that id must not resolve — DS-1677: "当該
    /// idを参照するderives_fromはいずれの候補も解決先として選ばず". Two
    /// documents each declare `R-908` and a third node cites it.
    /// @vtest.id TEST-SCAN-REPORTS-DOCUMENT-NODE-ID-COLLISION-ACROSS-FILES
    /// @vtest.covers VO-SCAN-RECORD-LOGICAL-ID-DUPLICATE-E-SCAN-010
    /// @vtest.target crates/vtest-scan/src/lib.rs::scan_project
    /// @vtest.intent verifies the same document node id defined in two different document files is reported as E-SCAN-010, and a derives_from edge naming that id resolves to no candidate (E-SCAN-012) rather than picking one
    #[test]
    fn reports_document_node_id_collision_across_files() {
        let root = fixture();
        let layout = VerifyLayout::new(&root);
        let file_a = DocumentFile {
            schema_version: "0.1".to_owned(),
            root: Vec::new(),
            request: vec![SentenceNode {
                id: DocumentId::new("R-908"),
                statement: "first definition".to_owned(),
                description: None,
                derives_from: Vec::new(),
                cites: None,
                source: fixture_node_source(),
            }],
            require: Vec::new(),
            spec: Vec::new(),
            detailed_spec: Vec::new(),
            basic_design: Vec::new(),
            design: Vec::new(),
        };
        let file_b = DocumentFile {
            schema_version: "0.1".to_owned(),
            root: Vec::new(),
            request: vec![
                SentenceNode {
                    id: DocumentId::new("R-908"),
                    statement: "second definition".to_owned(),
                    description: None,
                    derives_from: Vec::new(),
                    cites: None,
                    source: fixture_node_source(),
                },
                SentenceNode {
                    id: DocumentId::new("R-905"),
                    statement: "cites the colliding id".to_owned(),
                    description: None,
                    derives_from: vec![DocumentId::new("R-908")],
                    cites: None,
                    source: fixture_node_source(),
                },
            ],
            require: Vec::new(),
            spec: Vec::new(),
            detailed_spec: Vec::new(),
            basic_design: Vec::new(),
            design: Vec::new(),
        };
        write_document_file(&layout, "DOC-DUP-A", &file_a).unwrap();
        write_document_file(&layout, "DOC-DUP-B", &file_b).unwrap();

        let result = scan_project(&root).unwrap();
        let collisions = result
            .diagnostics
            .iter()
            .filter(|diagnostic| {
                diagnostic.code == "E-SCAN-010" && diagnostic.message.contains("R-908")
            })
            .collect::<Vec<_>>();
        assert!(
            !collisions.is_empty(),
            "expected an E-SCAN-010 for the R-908 collision, diagnostics: {:?}",
            result.diagnostics
        );

        // Per DS-1677, no candidate resolves: the reference must surface as
        // an unresolved E-SCAN-012, not a silent (arbitrary) resolution.
        assert!(
            result.diagnostics.iter().any(|diagnostic| {
                diagnostic.code == "E-SCAN-012"
                    && diagnostic.message.contains("R-905")
                    && diagnostic.message.contains("R-908")
            }),
            "a derives_from edge naming a colliding id must not resolve, diagnostics: {:?}",
            result.diagnostics
        );
    }

    /// Same as above but the collision is within one document file — DS-1677
    /// draws no distinction ("同一ファイル内・ファイル間を問わない").
    /// @vtest.id TEST-SCAN-REPORTS-DOCUMENT-NODE-ID-COLLISION-WITHIN-FILE
    /// @vtest.covers VO-SCAN-RECORD-LOGICAL-ID-DUPLICATE-E-SCAN-010
    /// @vtest.target crates/vtest-scan/src/lib.rs::scan_project
    /// @vtest.intent verifies the same document node id defined twice within one document file is reported as E-SCAN-010, matching the across-files case
    #[test]
    fn reports_document_node_id_collision_within_one_file() {
        let root = fixture();
        let layout = VerifyLayout::new(&root);
        let file = DocumentFile {
            schema_version: "0.1".to_owned(),
            root: Vec::new(),
            request: vec![
                SentenceNode {
                    id: DocumentId::new("R-909"),
                    statement: "first definition".to_owned(),
                    description: None,
                    derives_from: Vec::new(),
                    cites: None,
                    source: fixture_node_source(),
                },
                SentenceNode {
                    id: DocumentId::new("R-909"),
                    statement: "second definition, same file".to_owned(),
                    description: None,
                    derives_from: Vec::new(),
                    cites: None,
                    source: fixture_node_source(),
                },
            ],
            require: Vec::new(),
            spec: Vec::new(),
            detailed_spec: Vec::new(),
            basic_design: Vec::new(),
            design: Vec::new(),
        };
        write_document_file(&layout, "DOC-SAME-FILE-DUP", &file).unwrap();

        let result = scan_project(&root).unwrap();
        assert!(
            result.diagnostics.iter().any(|diagnostic| {
                diagnostic.code == "E-SCAN-010" && diagnostic.message.contains("R-909")
            }),
            "expected an E-SCAN-010 for the same-file collision, diagnostics: {:?}",
            result.diagnostics
        );
    }

    /// DS-1675/DS-536: a Test ID duplicate stays E-SCAN-002 and must not be
    /// reported as E-SCAN-010 — the two codes partition by id kind, they do
    /// not both fire for the same collision. Reuses the same colliding-Test-
    /// ID fixture as `colliding_test_ids_are_all_preserved_and_reach_
    /// downstream_checks` above.
    /// @vtest.id TEST-SCAN-TEST-ID-COLLISION-STAYS-E-SCAN-002
    /// @vtest.covers VO-SCAN-COLLIDING-TEST-IDS-ALL-PRESERVED
    /// @vtest.target crates/vtest-scan/src/lib.rs::scan_project
    /// @vtest.intent verifies a Test ID collision is reported as E-SCAN-002 and never also as E-SCAN-010, since the two codes partition by id kind
    #[test]
    fn test_id_collision_stays_e_scan_002_not_e_scan_010() {
        let root = fixture();
        fs::write(
            root.join("tests/collision.rs"),
            r#"
/// @vtest.id TEST-COLLISION
/// @vtest.covers VO-ADD
/// @vtest.target src/lib.rs::add
/// @vtest.intent first construct declaring a colliding Test ID
#[test]
fn collision_first() {}

/// @vtest.id TEST-COLLISION
/// @vtest.covers VO-ADD
/// @vtest.target src/lib.rs::add
/// @vtest.intent second construct declaring the same colliding Test ID
#[test]
fn collision_second() {}
"#,
        )
        .unwrap();

        let result = scan_project(&root).unwrap();
        assert!(
            result
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.code == "E-SCAN-002"
                    && diagnostic.message.contains("TEST-COLLISION")),
            "expected the Test ID collision to be reported as E-SCAN-002, diagnostics: {:?}",
            result.diagnostics
        );
        assert!(
            !result
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.code == "E-SCAN-010"
                    && diagnostic.message.contains("TEST-COLLISION")),
            "a Test ID collision must not be reported as E-SCAN-010, diagnostics: {:?}",
            result.diagnostics
        );
    }

    /// A corpus with no colliding ids at all must report zero E-SCAN-010
    /// diagnostics for document nodes — two distinct ids across two files,
    /// each referencing the other with no dangling or colliding entry.
    /// @vtest.id TEST-SCAN-NO-DOCUMENT-NODE-COLLISION-REPORTS-NONE
    /// @vtest.covers VO-SCAN-RECORD-LOGICAL-ID-DUPLICATE-E-SCAN-010
    /// @vtest.target crates/vtest-scan/src/lib.rs::scan_project
    /// @vtest.intent verifies a corpus with two distinct document node ids and no dangling/colliding entry reports zero E-SCAN-010 diagnostics
    #[test]
    fn no_document_node_collision_reports_no_e_scan_010() {
        let root = fixture();
        let layout = VerifyLayout::new(&root);
        let file_a = DocumentFile {
            schema_version: "0.1".to_owned(),
            root: Vec::new(),
            request: vec![SentenceNode {
                id: DocumentId::new("R-906"),
                statement: "no collision fixture A".to_owned(),
                description: None,
                derives_from: Vec::new(),
                cites: None,
                source: fixture_node_source(),
            }],
            require: Vec::new(),
            spec: Vec::new(),
            detailed_spec: Vec::new(),
            basic_design: Vec::new(),
            design: Vec::new(),
        };
        let file_b = DocumentFile {
            schema_version: "0.1".to_owned(),
            root: Vec::new(),
            request: vec![SentenceNode {
                id: DocumentId::new("R-907"),
                statement: "no collision fixture B".to_owned(),
                description: None,
                derives_from: vec![DocumentId::new("R-906")],
                cites: None,
                source: fixture_node_source(),
            }],
            require: Vec::new(),
            spec: Vec::new(),
            detailed_spec: Vec::new(),
            basic_design: Vec::new(),
            design: Vec::new(),
        };
        write_document_file(&layout, "DOC-NO-COLLISION-A", &file_a).unwrap();
        write_document_file(&layout, "DOC-NO-COLLISION-B", &file_b).unwrap();

        let result = scan_project(&root).unwrap();
        assert!(
            !result
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.code == "E-SCAN-010"),
            "no collision should report zero E-SCAN-010, diagnostics: {:?}",
            result.diagnostics
        );
    }

    /// DS-1645/BD-320 (E-SCAN-010): a `.verify/doc/*.json` file that fails
    /// `document_file_from_json`'s schema checks must be reported the same
    /// way a malformed VO record is (`validate_vo_record`) — an E-SCAN-010
    /// diagnostic, the file skipped — rather than aborting the whole scan
    /// with a code-less `ScanError` and no `ScanResult` at all. This also
    /// proves the abort does not silently swallow every diagnostic
    /// `record_diagnostics` would otherwise produce: a second, well-formed
    /// document with an orphaned node must still report its own E-SCAN-016.
    /// @vtest.id TEST-SCAN-MALFORMED-DOCUMENT-FILE-E-SCAN-010-CONTINUES
    /// @vtest.covers VO-SCAN-RECORD-LOGICAL-ID-DUPLICATE-E-SCAN-010, VO-SCAN-ORPHAN-NODE-E-SCAN-016
    /// @vtest.target crates/vtest-scan/src/lib.rs::scan_project
    /// @vtest.intent verifies a document file that fails schema parsing is reported as E-SCAN-010 with the file skipped, and the scan still evaluates the remaining well-formed document (reporting its own E-SCAN-016), rather than aborting with a code-less error
    #[test]
    fn malformed_document_file_reports_e_scan_010_and_scan_continues() {
        let root = fixture();
        let layout = VerifyLayout::new(&root);
        write_document_file(
            &layout,
            "DOC-OK",
            &DocumentFile {
                schema_version: "0.1".to_owned(),
                root: Vec::new(),
                request: vec![SentenceNode {
                    id: DocumentId::new("R-907"),
                    statement: "an orphaned request in a document alongside a broken one"
                        .to_owned(),
                    description: None,
                    derives_from: Vec::new(),
                    cites: None,
                    source: fixture_node_source(),
                }],
                require: Vec::new(),
                spec: Vec::new(),
                detailed_spec: Vec::new(),
                basic_design: Vec::new(),
                design: Vec::new(),
            },
        )
        .unwrap();
        fs::write(layout.doc_dir().join("DOC-BROKEN.json"), "{ not valid json").unwrap();

        let result = scan_project(&root).expect(
            "a malformed document file must not abort the whole scan with a code-less error",
        );
        assert!(
            result
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.code == "E-SCAN-010"
                    && diagnostic.message.contains("DOC-BROKEN")),
            "diagnostics: {:?}",
            result.diagnostics
        );
        assert!(
            result
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.code == "E-SCAN-016"
                    && diagnostic.message.contains("R-907")),
            "the scan must still evaluate documents after the broken one: {:?}",
            result.diagnostics
        );
    }

    /// @vtest.id TEST-SCAN-REPORTS-ORPHAN-NODE-NO-EFFECTIVE-UPSTREAM
    /// @vtest.covers VO-SCAN-ORPHAN-NODE-E-SCAN-016
    /// @vtest.target crates/vtest-scan/src/lib.rs::scan_project
    /// @vtest.intent verifies a request-layer sentence with an empty derives_from and no ancestor section is reported as E-SCAN-016, and not also as E-SCAN-012
    #[test]
    fn reports_orphan_node_with_no_effective_upstream() {
        let root = fixture();
        let layout = VerifyLayout::new(&root);
        // DS-1647: a `request`-layer sentence has no ancestor section to be
        // rescued by, so an empty `derives_from` here is orphan outright —
        // this is the one node shape where "own edges" and "effective
        // upstream" coincide exactly.
        let file = DocumentFile {
            schema_version: "0.1".to_owned(),
            root: Vec::new(),
            request: vec![SentenceNode {
                id: DocumentId::new("R-902"),
                statement: "orphan fixture".to_owned(),
                description: None,
                derives_from: Vec::new(),
                cites: None,
                source: fixture_node_source(),
            }],
            require: Vec::new(),
            spec: Vec::new(),
            detailed_spec: Vec::new(),
            basic_design: Vec::new(),
            design: Vec::new(),
        };
        write_document_file(&layout, "DOC-ORPHAN", &file).unwrap();

        let result = scan_project(&root).unwrap();
        let orphan = result
            .diagnostics
            .iter()
            .filter(|diagnostic| {
                diagnostic.code == "E-SCAN-016" && diagnostic.message.contains("R-902")
            })
            .collect::<Vec<_>>();
        assert_eq!(orphan.len(), 1, "diagnostics: {:?}", result.diagnostics);
        assert!(
            !result.diagnostics.iter().any(|diagnostic| {
                diagnostic.code == "E-SCAN-012" && diagnostic.message.contains("R-902")
            }),
            "an orphan node with no derives_from entries has nothing to \
             dangle; E-SCAN-012 must not fire for it: {:?}",
            result.diagnostics
        );
    }

    /// @vtest.id TEST-SCAN-SENTENCE-EMPTY-DERIVES-FROM-RESCUED-BY-ANCESTOR
    /// @vtest.covers VO-SCAN-ORPHAN-NODE-E-SCAN-016
    /// @vtest.target crates/vtest-scan/src/lib.rs::scan_project
    /// @vtest.intent verifies a sentence with an empty derives_from is not orphaned when its containing section already carries an edge (effective upstream = own edges union ancestor edges)
    #[test]
    fn sentence_with_empty_derives_from_is_rescued_by_ancestor_section_edge() {
        // DS-1647: 実効的な上流 = 自分の辺 ∪ 先祖の辺. A sentence whose own
        // `derives_from` is empty is not orphaned when the section
        // containing it already carries an edge.
        let root = fixture();
        let layout = VerifyLayout::new(&root);
        let file = DocumentFile {
            schema_version: "0.1".to_owned(),
            root: Vec::new(),
            request: Vec::new(),
            require: vec![SectionNode {
                id: DocumentId::new("REQ-S901"),
                title: "ancestor with an edge".to_owned(),
                description: None,
                source: fixture_node_source(),
                derives_from: Some(vec![DocumentId::new("ROOT-001")]),
                sections: None,
                items: Some(vec![SentenceNode {
                    id: DocumentId::new("REQ-903"),
                    statement: "rescued by ancestor".to_owned(),
                    description: None,
                    derives_from: Vec::new(),
                    cites: None,
                    source: fixture_node_source(),
                }]),
            }],
            spec: Vec::new(),
            detailed_spec: Vec::new(),
            basic_design: Vec::new(),
            design: Vec::new(),
        };
        write_document_file(&layout, "DOC-ANCESTOR-RESCUE", &file).unwrap();

        let result = scan_project(&root).unwrap();
        assert!(
            !result.diagnostics.iter().any(|diagnostic| {
                diagnostic.code == "E-SCAN-016"
                    && (diagnostic.message.contains("REQ-903")
                        || diagnostic.message.contains("REQ-S901"))
            }),
            "diagnostics: {:?}",
            result.diagnostics
        );
    }

    /// `check_section_node`'s own E-SCAN-016 branch (a section with neither
    /// its own edge nor an ancestor one) was never reached by any fixture in
    /// this suite: every other `SectionNode` fixture carries its own
    /// `derives_from`. A top-level `require`-layer section with no edge and
    /// no ancestor (unlike a `request`-layer sentence, which cannot even
    /// have a section ancestor) must still report E-SCAN-016 for itself.
    /// @vtest.id TEST-SCAN-REPORTS-ORPHANED-SECTION-NO-OWN-OR-ANCESTOR-EDGE
    /// @vtest.covers VO-SCAN-ORPHAN-NODE-E-SCAN-016
    /// @vtest.target crates/vtest-scan/src/lib.rs::scan_project
    /// @vtest.intent verifies a top-level section with neither its own derives_from edge nor an ancestor one reports E-SCAN-016 for itself
    #[test]
    fn reports_orphaned_section_node_with_no_own_or_ancestor_edge() {
        let root = fixture();
        let layout = VerifyLayout::new(&root);
        let file = DocumentFile {
            schema_version: "0.1".to_owned(),
            root: Vec::new(),
            request: Vec::new(),
            require: vec![SectionNode {
                id: DocumentId::new("REQ-S902"),
                title: "a section with no edge of its own and no ancestor".to_owned(),
                description: None,
                source: fixture_node_source(),
                derives_from: None,
                sections: None,
                items: None,
            }],
            spec: Vec::new(),
            detailed_spec: Vec::new(),
            basic_design: Vec::new(),
            design: Vec::new(),
        };
        write_document_file(&layout, "DOC-ORPHAN-SECTION", &file).unwrap();

        let result = scan_project(&root).unwrap();
        assert!(
            result.diagnostics.iter().any(|diagnostic| {
                diagnostic.code == "E-SCAN-016" && diagnostic.message.contains("REQ-S902")
            }),
            "diagnostics: {:?}",
            result.diagnostics
        );
    }

    /// `check_section_node`'s own E-SCAN-012 branch (a section's *own*
    /// `derives_from` dangling) was never reached by any fixture in this
    /// suite either — the sibling dangling-reference test above uses a
    /// `SentenceNode`. A dangling edge on a `SectionNode` itself must report
    /// E-SCAN-012, and — since it still counts as "having an edge" for
    /// orphan_detection purposes, resolving or not — must not also report
    /// E-SCAN-016 for that same section.
    /// @vtest.id TEST-SCAN-REPORTS-DANGLING-DERIVES-FROM-ON-SECTION-NOT-ORPHAN
    /// @vtest.covers VO-SCAN-DERIVES-FROM-DANGLING-E-SCAN-012, VO-SCAN-ORPHAN-NODE-E-SCAN-016
    /// @vtest.target crates/vtest-scan/src/lib.rs::scan_project
    /// @vtest.intent verifies a SectionNode's own dangling derives_from entry reports E-SCAN-012, and the section — since it still carries an edge, dangling or not — is not also reported as orphaned (E-SCAN-016)
    #[test]
    fn reports_dangling_derives_from_on_section_node_without_orphan() {
        let root = fixture();
        let layout = VerifyLayout::new(&root);
        let file = DocumentFile {
            schema_version: "0.1".to_owned(),
            root: Vec::new(),
            request: Vec::new(),
            require: vec![SectionNode {
                id: DocumentId::new("REQ-S903"),
                title: "a section with a dangling edge of its own".to_owned(),
                description: None,
                source: fixture_node_source(),
                derives_from: Some(vec![DocumentId::new("SPEC-999")]),
                sections: None,
                items: None,
            }],
            spec: Vec::new(),
            detailed_spec: Vec::new(),
            basic_design: Vec::new(),
            design: Vec::new(),
        };
        write_document_file(&layout, "DOC-SECTION-DANGLING", &file).unwrap();

        let result = scan_project(&root).unwrap();
        assert!(
            result.diagnostics.iter().any(|diagnostic| {
                diagnostic.code == "E-SCAN-012"
                    && diagnostic.message.contains("REQ-S903")
                    && diagnostic.message.contains("SPEC-999")
            }),
            "diagnostics: {:?}",
            result.diagnostics
        );
        assert!(
            !result.diagnostics.iter().any(|diagnostic| {
                diagnostic.code == "E-SCAN-016" && diagnostic.message.contains("REQ-S903")
            }),
            "a section with a (dangling) derives_from entry is not orphaned: {:?}",
            result.diagnostics
        );
    }

    /// DS-1647's ancestor-union rescue was only ever exercised at depth 1
    /// (a section's direct `items[]`) — no fixture in this suite nests
    /// `sections[]`, so the recursive `has_upstream` propagation through
    /// `check_section_node`'s own `sections[]` branch was never reached.
    /// Here, an outer section carries the only edge; its nested child
    /// section has none of its own, and that child's own sentence item also
    /// has none — both the child section and the sentence must be rescued by
    /// the outer section's edge propagating two levels down.
    /// @vtest.id TEST-SCAN-ORPHAN-RESCUE-PROPAGATES-NESTED-DEPTH-TWO
    /// @vtest.covers VO-SCAN-ORPHAN-NODE-E-SCAN-016
    /// @vtest.target crates/vtest-scan/src/lib.rs::scan_project
    /// @vtest.intent verifies an outer section's derives_from edge rescues both a nested child section with no edge of its own and that child's own sentence item, two levels down
    #[test]
    fn orphan_rescue_propagates_through_nested_sections_at_depth_two() {
        let root = fixture();
        let layout = VerifyLayout::new(&root);
        let file = DocumentFile {
            schema_version: "0.1".to_owned(),
            root: Vec::new(),
            request: Vec::new(),
            require: vec![SectionNode {
                id: DocumentId::new("REQ-S904"),
                title: "outer section with the only edge".to_owned(),
                description: None,
                source: fixture_node_source(),
                derives_from: Some(vec![DocumentId::new("ROOT-001")]),
                sections: Some(vec![SectionNode {
                    id: DocumentId::new("REQ-S905"),
                    title: "nested child section with no edge of its own".to_owned(),
                    description: None,
                    source: fixture_node_source(),
                    derives_from: None,
                    sections: None,
                    items: Some(vec![SentenceNode {
                        id: DocumentId::new("REQ-906"),
                        statement: "rescued through two levels of ancestor sections".to_owned(),
                        description: None,
                        derives_from: Vec::new(),
                        cites: None,
                        source: fixture_node_source(),
                    }]),
                }]),
                items: None,
            }],
            spec: Vec::new(),
            detailed_spec: Vec::new(),
            basic_design: Vec::new(),
            design: Vec::new(),
        };
        write_document_file(&layout, "DOC-NESTED-RESCUE", &file).unwrap();

        let result = scan_project(&root).unwrap();
        assert!(
            !result.diagnostics.iter().any(|diagnostic| {
                diagnostic.code == "E-SCAN-016"
                    && (diagnostic.message.contains("REQ-S905")
                        || diagnostic.message.contains("REQ-906"))
            }),
            "diagnostics: {:?}",
            result.diagnostics
        );
    }

    /// @vtest.id TEST-SCAN-REFERENCED-NODE-IS-STILL-ORPHAN
    /// @vtest.covers VO-SCAN-ORPHAN-NODE-E-SCAN-016
    /// @vtest.target crates/vtest-scan/src/lib.rs::scan_project
    /// @vtest.intent verifies a node with an empty derives_from stays orphan even when another node's derives_from cites it — an incoming reference is not part of effective upstream (own edges union ancestor edges only)
    #[test]
    fn node_referenced_by_another_nodes_derives_from_is_still_orphan() {
        // DS-1647 drops the predecessor document model's "referenced by
        // another document" rescue condition entirely: effective upstream
        // is own edges ∪ ancestor edges only, never an incoming reference.
        // A node with an empty `derives_from` stays orphan even when
        // another node cites it — this is a deliberate behavior change from
        // the predecessor model, asserted explicitly so a future change
        // does not silently reintroduce the dropped condition.
        let root = fixture();
        let layout = VerifyLayout::new(&root);
        let file = DocumentFile {
            schema_version: "0.1".to_owned(),
            root: Vec::new(),
            request: vec![
                SentenceNode {
                    id: DocumentId::new("R-904"),
                    statement: "cited by R-905 but has no upstream edge itself".to_owned(),
                    description: None,
                    derives_from: Vec::new(),
                    cites: None,
                    source: fixture_node_source(),
                },
                SentenceNode {
                    id: DocumentId::new("R-905"),
                    statement: "cites R-904".to_owned(),
                    description: None,
                    derives_from: vec![DocumentId::new("R-904")],
                    cites: None,
                    source: fixture_node_source(),
                },
            ],
            require: Vec::new(),
            spec: Vec::new(),
            detailed_spec: Vec::new(),
            basic_design: Vec::new(),
            design: Vec::new(),
        };
        write_document_file(&layout, "DOC-INCOMING-REF-ONLY", &file).unwrap();

        let result = scan_project(&root).unwrap();
        assert!(
            result.diagnostics.iter().any(|diagnostic| {
                diagnostic.code == "E-SCAN-016" && diagnostic.message.contains("R-904")
            }),
            "an incoming derives_from reference from another node must not rescue an \
             otherwise-empty node from orphan_detection (DS-1647 has no such condition): {:?}",
            result.diagnostics
        );
    }

    /// @vtest.id TEST-SCAN-WELL-FORMED-DOCUMENTS-NO-DOCUMENT-LAYER-DIAGNOSTICS
    /// @vtest.covers VO-SCAN-ORPHAN-NODE-E-SCAN-016, VO-SCAN-DERIVES-FROM-DANGLING-E-SCAN-012
    /// @vtest.target crates/vtest-scan/src/lib.rs::scan_project
    /// @vtest.intent verifies a well-formed document tree, including a non-root node whose derives_from resolves, reports neither E-SCAN-012 nor E-SCAN-016
    #[test]
    fn well_formed_documents_report_no_document_layer_diagnostics() {
        // fixture() already registers DOC-TEST's ROOT-001 (root layer,
        // excluded from orphan_detection outright). Add one more
        // well-formed, non-root node that derives from it to also exercise
        // a resolving `derives_from`.
        let root = fixture();
        let layout = VerifyLayout::new(&root);
        let file = DocumentFile {
            schema_version: "0.1".to_owned(),
            root: Vec::new(),
            request: vec![SentenceNode {
                id: DocumentId::new("R-906"),
                statement: "derives from the fixture root".to_owned(),
                description: None,
                derives_from: vec![DocumentId::new("ROOT-001")],
                cites: None,
                source: fixture_node_source(),
            }],
            require: Vec::new(),
            spec: Vec::new(),
            detailed_spec: Vec::new(),
            basic_design: Vec::new(),
            design: Vec::new(),
        };
        write_document_file(&layout, "DOC-CHILD", &file).unwrap();

        let result = scan_project(&root).unwrap();
        let document_layer_codes = ["E-SCAN-012", "E-SCAN-016"];
        assert!(
            !result
                .diagnostics
                .iter()
                .any(|diagnostic| document_layer_codes.contains(&diagnostic.code.as_str())),
            "diagnostics: {:?}",
            result.diagnostics
        );
    }

    /// Real-bundle orphan count: only runs when `VTEST_CANONICAL_BUNDLE`
    /// names the canonical `specification.json`. The bundle *is* a
    /// `DocumentFile` shape (DES-586: every document's same-named layer
    /// arrays concatenated — see `vtest_model::document`'s and
    /// `vtest_store::canonical`'s own bundle round-trip tests), so it is
    /// written as one `.verify/doc/BUNDLE.json` and run straight through
    /// `validate_document_nodes`. Reports the E-SCAN-016 (orphan_detection)
    /// count via `eprintln!` for the PR to cite — not asserted, per this
    /// task's own instruction (the number moves as the canonical bundle
    /// DS-784 / DS-786 make the evaluation input the canonical file *set*
    /// under `.verify/`, not the subset a reader recognises. A stray file in
    /// `.verify/doc/` must therefore surface as E-SCAN-010 (DS-1676, schema
    /// non-conformance) rather than being silently filtered out — silent
    /// filtering would let a document in an unread format vanish from all
    /// four checks.
    ///
    /// The `.gitkeep` that `init_project` writes into the same directory must
    /// NOT be reported: it is part of the layout this tool creates, so a
    /// freshly initialised project stays clean.
    /// @vtest.id TEST-SCAN-STRAY-DOC-FILE-REPORTED-NOT-SKIPPED
    /// @vtest.covers VO-SCAN-RECORD-LOGICAL-ID-DUPLICATE-E-SCAN-010
    /// @vtest.target crates/vtest-scan/src/lib.rs::scan_project
    /// @vtest.intent verifies a stray unrecognized file placed in .verify/doc/ is reported once as E-SCAN-010 rather than silently filtered, while the layout's own .gitkeep is not reported
    #[test]
    fn a_stray_file_in_the_doc_directory_is_reported_not_silently_skipped() {
        let root = std::env::temp_dir().join(format!("vtest-scan-doc-stray-{}", new_record_id()));
        let layout = init_project(&root, "stray").unwrap();

        // A freshly initialised project has only `.gitkeep` in doc/ and must
        // report nothing.
        let listing = read_document_dir(&layout.doc_dir()).unwrap();
        assert!(
            listing.unaccounted.is_empty(),
            "the layout's own .gitkeep must not be reported: {:?}",
            listing.unaccounted
        );

        fs::write(layout.doc_dir().join("legacy-document.yaml"), "id: DOC-1\n").unwrap();
        let listing = read_document_dir(&layout.doc_dir()).unwrap();
        assert_eq!(listing.unaccounted, vec!["legacy-document.yaml".to_owned()]);
        assert!(
            listing.names.is_empty(),
            "a stray file is not a document name"
        );

        let result = scan_project(&root).expect("scan should complete");
        let reported = result
            .diagnostics
            .iter()
            .filter(|diagnostic| {
                diagnostic.code == "E-SCAN-010"
                    && diagnostic.message.contains("legacy-document.yaml")
            })
            .count();
        assert_eq!(
            reported, 1,
            "a stray file in .verify/doc/ must be reported once as E-SCAN-010: {:?}",
            result.diagnostics
        );
        assert!(result.has_errors());
    }

    /// grows on its own branch).
    /// @vtest.id TEST-SCAN-CANONICAL-BUNDLE-ORPHAN-COUNT
    /// @vtest.covers VO-SCAN-ORPHAN-NODE-E-SCAN-016
    /// @vtest.target crates/vtest-scan/src/lib.rs::validate_document_nodes
    /// @vtest.intent reports (without asserting) the E-SCAN-016 orphan count against the real canonical specification.json bundle, and asserts the bundle itself parses without E-SCAN-010
    #[test]
    #[ignore = "requires VTEST_CANONICAL_BUNDLE env var pointing at the canonical specification.json"]
    fn canonical_bundle_orphan_count() {
        let path = std::env::var("VTEST_CANONICAL_BUNDLE")
            .expect("set VTEST_CANONICAL_BUNDLE to the canonical specification.json path");
        let text = std::fs::read_to_string(&path).expect("failed to read canonical bundle");

        let root =
            std::env::temp_dir().join(format!("vtest-scan-bundle-orphan-{}", new_record_id()));
        let layout = init_project(&root, "bundle").unwrap();
        fs::write(layout.doc_dir().join("BUNDLE.json"), &text).unwrap();

        let mut diagnostics = Vec::new();
        let document_names = vec!["BUNDLE".to_owned()];
        validate_document_nodes(&layout, &document_names, &mut diagnostics);
        assert!(
            !diagnostics
                .iter()
                .any(|diagnostic| diagnostic.code == "E-SCAN-010"),
            "the canonical bundle must parse as a well-formed DocumentFile: {:?}",
            diagnostics
        );
        // Document-node diagnostics carry no `SourceLocation` (record-layer,
        // not adapter-discovered — see `document_node_record_path`'s doc
        // comment), so the orphan count is taken directly from the
        // diagnostic count rather than from distinct `.location` values.
        let orphan_count = diagnostics
            .iter()
            .filter(|diagnostic| diagnostic.code == "E-SCAN-016")
            .count();
        eprintln!(
            "canonical_bundle_orphan_count: {} node(s) report E-SCAN-016 (orphan_detection) \
             against the real canonical bundle specification.json — not asserted (reported for \
             the PR to cite; DS-1647 scope: root-layer nodes excluded, effective upstream = own \
             derives_from ∪ ancestor section derives_from within the same file).",
            orphan_count
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
        "id: VO-ADD\nparent: null\nderives_from:\n  - doc: ROOT-001\nclaim: claim\nrepresentative_cases: []\ncreated: '2026-01-01'\nupdated: '2026-01-01'\n"
    }

    /// 別紙C:97-104 condition 1a: `explicit` かつ `combinations` 欠落
    /// (missing key entirely, not `null` or `[]` — those are 1b/1c below).
    /// @vtest.id TEST-SCAN-E-SCAN-017-CONDITION-1A-MISSING-COMBINATIONS
    /// @vtest.covers VO-SCAN-E-SCAN-017-MISSING-NULL-EMPTY-COMBINATIONS
    /// @vtest.target crates/vtest-scan/src/lib.rs::scan_project
    /// @vtest.intent verifies an explicit-policy VO with the combinations key entirely absent reports E-SCAN-017
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
    /// @vtest.id TEST-SCAN-E-SCAN-017-CONDITION-1B-NULL-COMBINATIONS
    /// @vtest.covers VO-SCAN-E-SCAN-017-MISSING-NULL-EMPTY-COMBINATIONS
    /// @vtest.target crates/vtest-scan/src/lib.rs::scan_project
    /// @vtest.intent verifies an explicit-policy VO with combinations: null reports E-SCAN-017
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
    /// @vtest.id TEST-SCAN-E-SCAN-017-CONDITION-1C-EMPTY-COMBINATIONS
    /// @vtest.covers VO-SCAN-E-SCAN-017-MISSING-NULL-EMPTY-COMBINATIONS
    /// @vtest.target crates/vtest-scan/src/lib.rs::scan_project
    /// @vtest.intent verifies an explicit-policy VO with combinations: [] reports E-SCAN-017
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
    /// @vtest.id TEST-SCAN-E-SCAN-017-CONDITION-2-EMPTY-DIMENSIONS
    /// @vtest.covers VO-SCAN-E-SCAN-017-EMPTY-DIMENSIONS
    /// @vtest.target crates/vtest-scan/src/lib.rs::scan_project
    /// @vtest.intent verifies an explicit-policy VO with an empty dimensions list reports E-SCAN-017
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
    /// @vtest.id TEST-SCAN-E-SCAN-017-CONDITION-3-NONEXPLICIT-POLICY
    /// @vtest.covers VO-SCAN-E-SCAN-017-NONEXPLICIT-POLICY-WITH-COMBINATIONS
    /// @vtest.target crates/vtest-scan/src/lib.rs::scan_project
    /// @vtest.intent verifies a non-empty combinations list under a non-explicit coverage_policy (independent-axes) reports E-SCAN-017
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
    /// @vtest.id TEST-SCAN-E-SCAN-017-CONDITION-4-UNDECLARED-DIMENSION
    /// @vtest.covers VO-SCAN-E-SCAN-017-UNDECLARED-DIMENSION-NAME
    /// @vtest.target crates/vtest-scan/src/lib.rs::scan_project
    /// @vtest.intent verifies a combinations entry naming a dimension not declared in dimensions[] reports E-SCAN-017
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
    /// @vtest.id TEST-SCAN-E-SCAN-017-CONDITION-5-UNDECLARED-PARTITION
    /// @vtest.covers VO-SCAN-E-SCAN-017-UNDECLARED-PARTITION-VALUE
    /// @vtest.target crates/vtest-scan/src/lib.rs::scan_project
    /// @vtest.intent verifies a combinations entry using a partition value not listed for its dimension reports E-SCAN-017
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
    /// @vtest.id TEST-SCAN-E-SCAN-017-CONDITION-6-MISSING-DECLARED-DIMENSION
    /// @vtest.covers VO-SCAN-E-SCAN-017-ENTRY-SHAPE-MISMATCH
    /// @vtest.target crates/vtest-scan/src/lib.rs::scan_project
    /// @vtest.intent verifies a combinations entry that omits one of the declared dimensions reports E-SCAN-017
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
    /// @vtest.id TEST-SCAN-E-SCAN-017-CONDITION-6-DUPLICATE-DIMENSION-KEY
    /// @vtest.covers VO-SCAN-E-SCAN-017-ENTRY-SHAPE-MISMATCH
    /// @vtest.target crates/vtest-scan/src/lib.rs::scan_project
    /// @vtest.intent verifies a combinations entry that declares the same dimension name twice reports E-SCAN-017 at the scan layer without the record layer rejecting the VO as E-SCAN-010
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
    /// §3.2.1's "記述順・map key 順には依存しない" — 要確認I（PR #26 review
    /// round 2）: `combinations[]`'s element type is `CombinationEntry`
    /// (`vtest-model`, since `61ba379`), not `BTreeMap`; this comment used
    /// to credit `BTreeMap`'s own `Eq`/`Ord` for the order-independence,
    /// which stopped being true once that migration landed.
    /// `CombinationEntry` recovers the same property explicitly — its
    /// `PartialEq`/`Ord` compare a sorted (name, value) view
    /// (`canonical_pairs`, `vtest-model/src/vo.rs`) instead of the
    /// declaration-order `Vec` it stores — so no special-casing is needed
    /// here to catch this as a duplicate.
    /// @vtest.id TEST-SCAN-E-SCAN-017-CONDITION-7-DUPLICATE-TUPLE
    /// @vtest.covers VO-SCAN-E-SCAN-017-DUPLICATE-TUPLE
    /// @vtest.target crates/vtest-scan/src/lib.rs::scan_project
    /// @vtest.intent verifies two combinations entries resolving to the same (dimension, partition) tuple, regardless of key declaration order, report E-SCAN-017 as a duplicate
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

    /// Positive control: a well-formed `explicit`-policy VO (本冊:263-271
    /// §3.2.1's own literal example, id/derives_from swapped for this
    /// fixture; kept verbatim including its flow-style `combinations`
    /// entries — レビュー round 2 項目【K-2】の訂正。旧コメントは出典を
    /// 別紙C:97-104 としていたが、別紙C:97-104 は散文の箇条書きのみで
    /// literal example を含まない。`operand-sign` / `operator` の例は
    /// 本冊 §3.2.1 にあり、そこでは `combinations` の各 entry が flow
    /// style（`- { operand-sign: positive, operator: div }`）で書かれて
    /// いる。以前はこれを block style へ書き換えており verbatim ではな
    /// かった) must not raise E-SCAN-017. Without this, the eight tests
    /// above could all be trivially satisfied by an
    /// `invalid_vo_combinations` that always returns `Some(..)`.
    /// @vtest.id TEST-SCAN-E-SCAN-017-WELL-FORMED-REPORTS-NONE
    /// @vtest.covers VO-MODEL-VO-COMBINATIONS-ENTRY-SHAPE
    /// @vtest.target crates/vtest-scan/src/lib.rs::scan_project
    /// @vtest.intent verifies a well-formed explicit-policy VO, using 本冊 §3.2.1's own literal flow-style example, raises no E-SCAN-017 — the positive control for the condition 1-7 negative tests
    #[test]
    fn e_scan_017_well_formed_explicit_combinations_report_no_diagnostic() {
        let root = fixture();
        // `{{` / `}}` below are `format!`'s brace-escape for a literal `{`
        // / `}` — the rendered YAML is the flow-style
        // `{ operand-sign: positive, operator: div }` verbatim from
        // 本冊:270-271, not doubled braces.
        write_vo_add(
            &root,
            &format!(
                "{}dimensions:\n  - name: operand-sign\n    partitions: [positive, negative]\n  - name: operator\n    partitions: [add, sub, mul, div]\ncoverage_policy: explicit\ncombinations:\n  - {{ operand-sign: positive, operator: div }}\n  - {{ operand-sign: negative, operator: div }}\n",
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
