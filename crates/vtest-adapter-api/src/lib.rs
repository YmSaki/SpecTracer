//! 言語・runner非依存の source discovery adapter 契約（詳細設計 v0.1 §1.1・§4・
//! §5.2）。
//!
//! 詳細設計 v0.1 §1.1（本冊:30-60）はワークスペース構成として
//! `vtest-adapter-api` を独立 crate と定め、"`vtest-adapter-api` は
//! `vtest-model` 以外の言語実装・Cargo実装へ依存しない" と明言する。この
//! crate はその境界そのものであり、`vtest-model` 以外へ依存してはならない。
//!
//! capability の型は本冊 §5.2（704-758行）が hash 未計算 DTO として定義する
//! `SourceDiscoveryAdapter` 系を指すが、trait メソッドシグネチャ自体は仕様の
//! どこにも書かれていない（`pub trait` の記述は本冊・別紙A・別紙Cいずれにも
//! 存在しない。`pr3-spec-extract.md` §8）。したがってこの trait の「形」は
//! 実装の裁量であり、本 crate が PR3 の裁量で定義する。契約の中身（何を
//! adapter の責務とし、何を core に残すか）は仕様が正典。
//!
//! この crate では `ManagedTestDraft` / `DiscoveredTestDraft` /
//! `SourceTargetDraft` / `DiscoveryBatch` という仕様の型名を導入しない。ここで
//! 定義する `TestDraft` / `SourceDraft` / `DiscoveryOutcome` は、既存の
//! `vtest_model::TestEntity` / `SourceFunction` 形状を踏襲した、より小さな
//! 中間 DTO である。
//!
//! `ManagedTestLink` / `DiscoveredTest`（本冊:788-801、Test ID 衝突時に
//! observation を保持したまま後段へ伝達する語彙。Owner 裁定1、
//! `pr3-decisions.md`）は core materialization 後の型であり、adapter が
//! 返す draft 側の型ではないため `vtest_model` 側に置く。この crate は
//! 生成しない（core 側の `vtest-scan` が生成する）。
//!
//! `MissingTestConstruct`（下記）も同じ理由で `ManagedTestDraftLink` という
//! 仕様の型名を導入しない。本冊:565（§4.4）「adapter固有のsource
//! declarationを構文解析できない場合、adapterは該当Test constructを
//! Discovered Testとして返し、対応を`ManagedTestLink::Missing`として診断を
//! 付与する」が要求するのは、adapter が「観測した Test construct 集合
//! `D`」（基本:408-427、§12）に、管理宣言または必須 metadata を欠く
//! construct も含めて返すことだけである。`D` から `M`（構造上完全な
//! managed Test Entity 集合）を導く判定・`ManagedTestLink` そのものの
//! 組み立ては core（`vtest-scan::materialize_tests`）が行う。

use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
};

use vtest_model::{
    ContentHash, Diagnostic, ExecutionDescriptor, Locator, SourceLocation, SrcId,
    TargetCoverageResult, TargetRef, TestEntity, TestId, VoId,
};

/// core が `config.yaml` の1 adapter エントリ（`AdapterConfig`）から解決した、
/// project-relative な include root の一覧。root × `scan.include` の path
/// 演算は Rust 固有の処理ではないため core（`vtest-scan::resolve_adapter_
/// includes`）が行い、adapter へは解決済みの path だけを渡す。
pub struct AdapterScanConfig {
    pub include_paths: Vec<PathBuf>,
}

/// hash 未計算の Test draft。本冊:93「adapterが最終的な`TestEntity.content_
/// hash`または`SourceTarget.content_hash`を返して自己確定してはならない」に
/// 従い、adapter は最終 hash を計算しない。`construct_text` は§1.3の
/// normalization 済み construct bytes（本冊:99「Test constructとして
/// metadata doc commentを除き、実行に影響する属性、signature、bodyを含む
/// 関数itemのbytesを返す」— `rust-cargo` adapterではmetadata doc comment
/// だけを除いた範囲であり、`SourceDraft::construct_text`（属性とdoc
/// commentを含む関数item全体。本冊:99「Source Targetには属性とdoc comment
/// を含む関数item全体を返す」）とは異なる範囲を指す）であり、core が2つの
/// 独立したhashへの入力として使う（`vtest-scan::materialize_tests`）:
/// `vtest_model::DiscoveredTest.content_hash`（`ContentHash::from_text` に
/// よる、construct bytesだけを束縛する限定hash）と、
/// `vtest_model::TestEntity.content_hash`（`test_subject_hash`、DES-077の
/// Test subject hash — adapter ID・全canonical metadata・`location`の
/// adapter/path/locator・`execution`と合わせて束縛する）の両方。
///
/// 必須 metadata（core 中立: id・`covers ≥ 1`・intent。本冊 §4.4、DS-1666）を
/// 具体化できないTest構文は、adapterが
/// 診断（W-SCAN-101/E-SCAN-005/006/007）を返した上で、`TestDraft` の代わりに
/// `MissingTestConstruct`（下記）を返す（`ManagedTestDraftLink::Missing`
/// 相当）。したがってこの型のフィールドは必須 metadata について `Option`
/// を持たない — `Option` で表現される曖昧さは `TestDraft` を作らない
/// （`MissingTestConstruct` 側へ回る）ことで型から追い出す。
///
/// Test ID の大域的一意性（E-SCAN-002）と `covers` の VO 参照解決
/// （E-SCAN-003）は、本冊:571「VO参照の解決とTest IDの大局的一意性は
/// adapterではなくcoreが検査する」により、この draft を返す時点では未検査
/// である。同じ Test ID を宣言する複数の draft が返ることを許容する。
pub struct TestDraft {
    pub id: TestId,
    pub covers: Vec<VoId>,
    /// 基本仕様:143・本冊:619。`vtest_model::TestEntity::targets` と同じ
    /// 単一 field 形状（代表 1 件へ縮約しない）。
    pub targets: Vec<TargetRef>,
    pub intent: String,
    pub input: Option<String>,
    pub expect: Option<String>,
    pub kind: Option<String>,
    pub cases: Vec<String>,
    pub related: Vec<TestId>,
    pub location: SourceLocation,
    pub construct_text: String,
    /// 本冊:685-703「`TestEntity.execution` はadapter、project、suite、
    /// opaque selectorからなる中立な実行座標である」「`filter`、
    /// `package`、`test_target`および`TestTarget`型を`vtest-model`へ置か
    /// ない」。この crate は core（`vtest-model`）と同じ中立形状を保つため、
    /// Rust/Cargo 固有の `filter` / `package` / `test_target` フィールドを
    /// 持たない — `rust-cargo` adapter（`vtest-adapter-rust`）がそれらを
    /// `ExecutionDescriptor` へ解釈してから積む（本冊 §9.2）。
    pub execution: ExecutionDescriptor,
}

/// hash 未計算の Source Target draft（本冊 §5.2 の `SourceTargetDraft` に
/// 相当する簡略版。`TestDraft` 同様、hash は core が計算する）。
pub struct SourceDraft {
    pub locator: Locator,
    pub src_id: Option<SrcId>,
    pub location: SourceLocation,
    pub construct_text: String,
}

/// adapter が Test construct として認識したが、管理宣言または必須
/// metadata の欠落により `TestDraft` へ具体化できなかった construct
/// （本冊:565「adapter固有のsource declarationを構文解析できない場合、
/// adapterは該当Test constructをDiscovered Testとして返し、対応を
/// `ManagedTestLink::Missing`として診断を付与する」。Owner裁定1、
/// `pr3-decisions.md`「scanner が観測した Test construct はすべて保持
/// する」）。
///
/// 何が欠落していたか（`@vtest` annotation が無い・必須 metadata が無い・
/// test-annotation-line 文法エラー等）は `DiscoveryOutcome.diagnostics`
/// （W-SCAN-101 / E-SCAN-005/006/007 等）が既に運ぶ。この型はそれらの
/// 診断内容を重複させず、core が `ManagedTestLink::Missing` を持つ
/// `vtest_model::DiscoveredTest` を組み立てるための construct identity
/// （`location` と、`content_hash` 計算用の normalization 済み construct
/// bytes）だけを渡す。`TestDraft::location` / `construct_text` と同じ
/// 意味を持つ2 fieldのみで、hash 未計算・adapter 名なし（core 側で
/// discovery batch から adapter を束ねる。`vtest-scan::scan_project_with_
/// config` の `test_drafts` 同様の扱い）である点も `TestDraft` と揃える。
pub struct MissingTestConstruct {
    pub location: SourceLocation,
    pub construct_text: String,
}

/// 1 adapter の discovery 結果。Target Reference 解決（§6.1）、Test ID の
/// 大域的一意性、VO 参照解決は含まない — これらは core が複数 adapter の
/// 出力を統合してから行う（本冊 §5.1 手順4・5・7）。
pub struct DiscoveryOutcome {
    pub files_scanned: usize,
    pub tests: Vec<TestDraft>,
    /// 管理宣言または必須 metadata を欠くために `TestDraft` へ具体化
    /// できなかった Test construct（上記 `MissingTestConstruct` 参照）。
    /// `tests` と合わせて、この adapter が観測した Test construct 全体
    /// （基本:408-427・§12 の集合 `D` の、この adapter に属する部分）を
    /// 成す。
    pub missing_tests: Vec<MissingTestConstruct>,
    pub sources: Vec<SourceDraft>,
    pub diagnostics: Vec<Diagnostic>,
}

/// discovery を継続できない確定的な失敗（ファイル走査そのものの失敗、
/// byte range 逸脱等。Evidence なし）。本冊:1645（§17.1）
/// 「E-ADAPTER-002 | error | adapterのdiscoveryまたはrunnerが確定的に失敗
/// （Evidenceなし）」が割り当てる条件そのものであり、core はこれを scan
/// 全体の失敗として扱う（`vtest-scan::ScanError::Discovery` へ変換し、
/// `.code()` は常に `"E-ADAPTER-002"` を返す。BLOCKER 4、PR #26 review
/// round 1 — 以前はこの型もその変換先の`ScanError`もコードを一切持たず、
/// 別紙C:96「`vtest scan` / `doctor`はE-ADAPTER-* / E-CONFIG-*による操作
/// 拒否をexit 2…にする」を満たせなかった）。診断として記録して scan を
/// 継続させると、壊れた発見結果を正常な結果として黙って通すことになり
/// fail-open になる（`pr3-decisions.md` 裁定2 と同じ理由）。
#[derive(Debug)]
pub struct DiscoveryError {
    pub path: PathBuf,
    pub message: String,
}

impl std::fmt::Display for DiscoveryError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "discovery failed at {}: {}",
            self.path.display(),
            self.message
        )
    }
}

impl std::error::Error for DiscoveryError {}

/// 本冊 §5.2「adapter capabilityは `SourceDiscoveryAdapter`、…に分割する。
/// 各adapterは一意なID、languages、capabilities、config namespaceを宣言し」
/// のうち、PR3 が扱う discovery capability だけを表す。`languages` /
/// `capabilities` 宣言と registry 側の不一致検査（E-ADAPTER-001系）は本 PR の
/// 範囲外（PR3 は単一 adapter `rust-cargo` のみを登録する）。
pub trait SourceDiscoveryAdapter {
    /// registry がこの adapter を引くための ID（`config.yaml` の
    /// `adapters[].id` と照合する。本冊 §6.1「coreは`TargetRef::Locator.
    /// adapter`をregistryで解決し」）。
    fn id(&self) -> &'static str;

    /// 本冊 §5.5 の `DiscoveryBatch` 構築手順に相当する discovery を実行する。
    /// `root` はプロジェクトルート、`fallback_package` は adapter 固有の
    /// package 名解決が失敗した場合に使う core 側の既定パッケージ名
    /// （`config.yaml` の `project.name`）。
    fn discover(
        &self,
        root: &Path,
        fallback_package: &str,
        config: &AdapterScanConfig,
    ) -> Result<DiscoveryOutcome, DiscoveryError>;
}

/// The command and identity selected by a [`TestRunnerAdapter`].  The process
/// itself is intentionally not owned by the adapter: orchestration launches
/// this language-neutral description and records the returned runner kind.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RunnerCommand {
    pub program: String,
    pub args: Vec<String>,
    pub current_dir: PathBuf,
    pub env: BTreeMap<String, String>,
    pub runner_kind: String,
    pub command_line: String,
}

/// Raw process output passed back to a [`TestRunnerAdapter`] for parsing.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RunnerOutput<'a> {
    pub stdout: &'a str,
    pub stderr: &'a str,
    pub exit_code: Option<i32>,
}

/// A runner's per-Test observation.  `Unknown` is deliberately distinct from
/// a successful or failed execution so that orchestration can reject a
/// missing or unparseable result without creating Evidence.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RunnerTestResult {
    Pass,
    Fail,
    Ignored,
    Unknown,
}

/// Deterministic failures while interpreting an opaque execution descriptor.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum TestRunnerError {
    MissingProject,
    MissingSuite,
    MissingSuiteName { kind: String },
    MissingCoverageOutputPath,
    UnsupportedSuiteKind { kind: String },
}

impl std::fmt::Display for TestRunnerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MissingProject => write!(f, "execution project is missing"),
            Self::MissingSuite => write!(f, "execution suite is missing"),
            Self::MissingSuiteName { kind } => {
                write!(f, "execution suite {kind:?} requires a name")
            }
            Self::MissingCoverageOutputPath => {
                write!(f, "coverage execution requires an output path")
            }
            Self::UnsupportedSuiteKind { kind } => {
                write!(f, "unsupported execution suite kind {kind:?}")
            }
        }
    }
}

impl std::error::Error for TestRunnerError {}

/// Language- and runner-specific interpretation of an opaque execution
/// descriptor.  The core passes the descriptor's strings through unchanged;
/// the selected adapter owns command construction and result parsing.
pub trait TestRunnerAdapter {
    fn id(&self) -> &'static str;

    fn command(
        &self,
        root: &Path,
        execution: &ExecutionDescriptor,
        coverage: bool,
        coverage_output_path: Option<&Path>,
    ) -> Result<RunnerCommand, TestRunnerError>;

    fn parse(&self, execution: &ExecutionDescriptor, output: RunnerOutput<'_>) -> RunnerTestResult;
}

/// Per-target reachability measurement returned by [`CoverageAdapter::
/// measure`].  `result`/`count` reuse `vtest_model::TargetCoverageResult`
/// (the Evidence wire's own PASS/FAIL/UNKNOWN domain, DES-187/DES-188)
/// directly rather than a parallel adapter-owned enum: `vtest-exec` writes
/// these two fields into `EvidenceRecord.target_coverage` unchanged (本冊
/// §0「基本仕様に無い義務・検査・状態…を新設しない」— inventing a second
/// three-value domain only to convert it back at the call site would do
/// exactly that). `target` is returned (not just re-used from the caller's
/// input) so `vtest-exec` never has to zip its own target list back onto the
/// adapter's per-target results by position.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CoverageTargetMeasurement {
    pub target: Locator,
    pub result: TargetCoverageResult,
    pub count: Option<u64>,
}

/// Language- and tool-specific coverage measurement (本冊 §5.2 `CoverageAdapter`,
/// BD-197/BD-220/BD-222). The core (`vtest-exec`) launches the runner and
/// hands this adapter the coverage output path it wrote to; the adapter owns
/// parsing that output and matching it against each declared Target
/// Reference's opaque `Locator.value` (BD-206「coreはopaque locatorの内部
/// 構文は解釈しない」, REQ-154).
///
/// Method-name and availability-reason strings are the adapter's own wording
/// (DS-760's `W-EXEC-101` diagnostic and `target_coverage.method`,
/// BD-220「`rust-cargo` CoverageAdapterは`cargo-llvm-cov`を使用する」) —
/// `vtest-exec` records them verbatim and does not hardcode a tool name or
/// unavailability message itself.
///
/// The four method names and their argument/return shapes below are not
/// specified anywhere in the four canonical spec files (same silence noted
/// for `SourceDiscoveryAdapter` and `TestRunnerAdapter` above and in
/// `TestRunnerAdapter`'s own doc comment) — DES-352 fixes only that a
/// capability adapter returns hash-uncomputed DTOs for core to finalize.
/// This trait's concrete shape (four methods named `id`/`method`/
/// `availability`/`measure`, `availability` returning `Result<(), String>`,
/// `measure` taking a target slice and returning a `Vec` in the same order)
/// is this PR's derivation from that responsibility split, not a literal
/// spec requirement.
pub trait CoverageAdapter {
    fn id(&self) -> &'static str;

    /// The coverage method name this adapter's Evidence records in
    /// `target_coverage.method` (currently always `"llvm-cov"` for
    /// `rust-cargo`, BD-220).
    fn method(&self) -> &'static str;

    /// Whether this environment can measure coverage right now (e.g.
    /// whether the required tool binary is installed and runs). `Err`
    /// carries the adapter's own diagnostic text for why not; `vtest-exec`
    /// reports it as `W-EXEC-101` and records `target_coverage` as
    /// `checked: false` (DS-473/DS-760/DS-1581).
    fn availability(&self, root: &Path) -> Result<(), String>;

    /// Parses the coverage output written to `coverage_output_path` and
    /// reports, for each of `targets` (in the same order), whether it was
    /// reached. `result` is `Unknown` when the target function cannot be
    /// located in the output (DS-832「関数不見当はUNKNOWNとする」) — this
    /// covers both an unreadable/unparsable output file and a target the
    /// output simply does not mention; `vtest-exec` does not need to
    /// distinguish those cases further.
    fn measure(
        &self,
        coverage_output_path: &Path,
        targets: &[Locator],
    ) -> Vec<CoverageTargetMeasurement>;
}

/// 本冊 §5.2「adapter capabilityは `SourceDiscoveryAdapter`、
/// `TestWireCodec`、`StaticAnalysisAdapter`、`StructuredTestAdapter`、
/// `TestRunnerAdapter`、`CoverageAdapter`に分割する」（BD-197）が列挙する
/// 6 capability。`TestWireCodec` / `StructuredTestAdapter` は本 PR の
/// 時点で対応する trait を持たない — 宣言はできるが対応する `as_*` 検査
/// (下記 `Adapter::register` の対象)は無い slot のまま残す（trait 自体は
/// PR E / G が追加する）。
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum Capability {
    SourceDiscovery,
    TestWireCodec,
    StaticAnalysis,
    StructuredTest,
    TestRunner,
    Coverage,
}

impl Capability {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::SourceDiscovery => "source_discovery",
            Self::TestWireCodec => "test_wire_codec",
            Self::StaticAnalysis => "static_analysis",
            Self::StructuredTest => "structured_test",
            Self::TestRunner => "test_runner",
            Self::Coverage => "coverage",
        }
    }
}

impl std::fmt::Display for Capability {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// DES-350「各adapterは一意なID、languages、capabilities、config namespace
/// を宣言する」の自己宣言 DTO。`AdapterRegistry::register` はこの宣言と、
/// 渡された `Adapter` の `as_*` 実装との対応（双方向）を検査する
/// （DES-351/DS-1569/DS-1663）。
#[derive(Clone, Debug)]
pub struct AdapterDescriptor {
    pub id: String,
    pub languages: Vec<String>,
    pub capabilities: Vec<Capability>,
    pub config_namespace: String,
}

/// DES-408「`vtest-audit`は、Test、全Target Reference、各source range、
/// content hash、および選択adapterの現在configを`StaticAnalysisAdapter`へ
/// 渡す」の入力 DTO。
///
/// 「全Target Reference」は `test.targets` として運ぶ（`TestEntity` 自体が
/// 保持する）。「各source range」はこの capability の唯一の実装
/// （`vtest_adapter_rust::oracle_presence`）が実際に読むのが Test 自身の
/// construct 範囲だけであるため、その construct bytes（`construct_text`）
/// として運ぶ — 個々の target 自身の source range は、この解析が対象の
/// *名前*（`Locator` の末尾 segment、`oracle_presence::target_symbol`）
/// 以外を消費しない既存の開示済み限定（本 PR で変更しない）により、別途
/// 引き回さない。「選択adapterの現在config」は config namespace の型
/// （本 PR 範囲外）を新設せず、現在の唯一の消費対象（DS-617
/// `assertion_macros`）だけをプレーンな `Vec<String>` として渡す。
pub struct StaticAnalysisInput<'a> {
    pub test: &'a TestEntity,
    pub construct_text: &'a str,
    pub content_hash: &'a ContentHash,
    pub extra_assertion_macros: &'a [String],
}

/// DS-606/607/608 が合成する `oracle_presence` の3値判定。
/// `CoverageAdapter::measure` が `CoverageTargetMeasurement` で
/// `vtest_model::TargetCoverageResult`（coverage capability専用の意味を
/// 持つ既存3値）を再利用したのと同じ理由で、ここでは意味の異なる
/// capability 専用の3値ドメインを `RunnerTestResult` と同様に新設する
/// （本冊 §0「基本仕様に無い義務・検査・状態を新設しない」は verify が
/// 消費する `VerificationState`（5値）の話であり、adapter capability の
/// 戻り値型はその対象ではない）。`basis` は DS-606/607/608 の根拠文字列
/// （DA-001/003/004/005/006 のうちどれがどう判定したか）。
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum StaticAnalysisVerdict {
    Pass(Vec<String>),
    Fail(Vec<String>),
    Unknown(Vec<String>),
}

/// 本冊 §5.2 `StaticAnalysisAdapter`（DES-408、`oracle_presence`の
/// dispatch先）。
pub trait StaticAnalysisAdapter {
    fn id(&self) -> &'static str;

    fn analyze(&self, input: StaticAnalysisInput<'_>) -> StaticAnalysisVerdict;
}

/// DES-350/351「各adapterは一意なID、languages、capabilities、config
/// namespaceを宣言する」「registryは宣言と実装の不一致および重複IDを拒否
/// する」の宣言側。1 adapter 実装がこの trait を実装し、`descriptor()` で
/// 宣言する capability を返し、`as_*` で実際に実装している capability の
/// trait object を返す。宣言したのに対応する `as_*` が `None`、または
/// 宣言していないのに `as_*` が `Some` を返す組は `AdapterRegistry::
/// register` が拒否する（下記）。
pub trait Adapter {
    fn descriptor(&self) -> AdapterDescriptor;

    fn as_source_discovery(&self) -> Option<&dyn SourceDiscoveryAdapter> {
        None
    }

    fn as_static_analysis(&self) -> Option<&dyn StaticAnalysisAdapter> {
        None
    }

    fn as_test_runner(&self) -> Option<&dyn TestRunnerAdapter> {
        None
    }

    fn as_coverage(&self) -> Option<&dyn CoverageAdapter> {
        None
    }
}

/// DS-1569/DES-351「registryはadapter IDの重複、宣言capabilityと実装の
/// 不一致、未登録adapterを拒否する」のうち、登録時に判明する2条件
/// （id重複・宣言と実装の不一致、双方向）。「未登録」は登録時ではなく
/// 解決時（`AdapterRegistry::get`等が`None`を返す先）の呼び出し元の責務
/// であり、この型の構成対象ではない。DS-1663はこれら3条件をまとめて
/// `E-ADAPTER-001` と定めるが、コードの割り当ては `DiscoveryError` と同じ
/// 方針でこの crate の関心事とせず、呼び出し元が割り当てる。
#[derive(Debug, Eq, PartialEq)]
pub enum AdapterRegistrationError {
    DuplicateId { id: String },
    DeclaredWithoutImplementation { id: String, capability: Capability },
    ImplementedWithoutDeclaration { id: String, capability: Capability },
}

impl std::fmt::Display for AdapterRegistrationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::DuplicateId { id } => write!(f, "adapter id {id:?} is already registered"),
            Self::DeclaredWithoutImplementation { id, capability } => write!(
                f,
                "adapter {id:?} declares capability `{capability}` but does not implement it"
            ),
            Self::ImplementedWithoutDeclaration { id, capability } => write!(
                f,
                "adapter {id:?} implements capability `{capability}` without declaring it"
            ),
        }
    }
}

impl std::error::Error for AdapterRegistrationError {}

struct RegisteredAdapter {
    descriptor: AdapterDescriptor,
    adapter: Box<dyn Adapter>,
}

/// 登録済み adapter を ID で引く registry（本冊 §5.1 手順1「registryとconfig
/// の検証」・§6.1「coreはregistryで解決」、BD-007「CLI・MCP・検証coreは
/// adapter registryを介して能力を選択する」）。「該当実装が無ければ
/// `None`」を返すだけで、その先の診断コード割り当ては呼び出し元が行う
/// （`DiscoveryError`/`ScanError`と同じ方針）。
#[derive(Default)]
pub struct AdapterRegistry {
    adapters: Vec<RegisteredAdapter>,
}

impl AdapterRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    /// DES-351/DS-1569/DS-1663: id重複、または宣言capabilityと実装
    /// （`as_*`）の不一致（双方向）があれば拒否する。
    pub fn register(&mut self, adapter: Box<dyn Adapter>) -> Result<(), AdapterRegistrationError> {
        let descriptor = adapter.descriptor();
        if self
            .adapters
            .iter()
            .any(|entry| entry.descriptor.id == descriptor.id)
        {
            return Err(AdapterRegistrationError::DuplicateId { id: descriptor.id });
        }
        let checks: [(Capability, bool); 4] = [
            (
                Capability::SourceDiscovery,
                adapter.as_source_discovery().is_some(),
            ),
            (
                Capability::StaticAnalysis,
                adapter.as_static_analysis().is_some(),
            ),
            (Capability::TestRunner, adapter.as_test_runner().is_some()),
            (Capability::Coverage, adapter.as_coverage().is_some()),
        ];
        for (capability, implemented) in checks {
            let declared = descriptor.capabilities.contains(&capability);
            if declared && !implemented {
                return Err(AdapterRegistrationError::DeclaredWithoutImplementation {
                    id: descriptor.id,
                    capability,
                });
            }
            if implemented && !declared {
                return Err(AdapterRegistrationError::ImplementedWithoutDeclaration {
                    id: descriptor.id,
                    capability,
                });
            }
        }
        self.adapters.push(RegisteredAdapter {
            descriptor,
            adapter,
        });
        Ok(())
    }

    pub fn get(&self, id: &str) -> Option<&dyn Adapter> {
        self.adapters
            .iter()
            .find(|entry| entry.descriptor.id == id)
            .map(|entry| entry.adapter.as_ref())
    }

    pub fn descriptor(&self, id: &str) -> Option<&AdapterDescriptor> {
        self.adapters
            .iter()
            .find(|entry| entry.descriptor.id == id)
            .map(|entry| &entry.descriptor)
    }

    pub fn source_discovery(&self, id: &str) -> Option<&dyn SourceDiscoveryAdapter> {
        self.get(id).and_then(Adapter::as_source_discovery)
    }

    pub fn static_analysis(&self, id: &str) -> Option<&dyn StaticAnalysisAdapter> {
        self.get(id).and_then(Adapter::as_static_analysis)
    }

    pub fn test_runner(&self, id: &str) -> Option<&dyn TestRunnerAdapter> {
        self.get(id).and_then(Adapter::as_test_runner)
    }

    pub fn coverage(&self, id: &str) -> Option<&dyn CoverageAdapter> {
        self.get(id).and_then(Adapter::as_coverage)
    }

    /// Registered adapter IDs, in registration order. Used by core to report
    /// the known-adapter list when it rejects a `config.yaml` adapter ID that
    /// `get` cannot resolve (fail-closed rejection of unregistered IDs). Which
    /// diagnostic code that rejection carries is settled, not this method's
    /// concern to restate: DS-352/DS-1663 fix it as E-CONFIG-001 (see
    /// `vtest-scan::ScanError::UnknownAdapterId`'s doc comment).
    pub fn ids(&self) -> impl Iterator<Item = &str> {
        self.adapters
            .iter()
            .map(|entry| entry.descriptor.id.as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    /// 宣言どおりの capability だけを実装する fake（正常系）。
    #[derive(Default)]
    struct HonestAdapter;

    impl StaticAnalysisAdapter for HonestAdapter {
        fn id(&self) -> &'static str {
            "honest"
        }
        fn analyze(&self, _input: StaticAnalysisInput<'_>) -> StaticAnalysisVerdict {
            StaticAnalysisVerdict::Pass(Vec::new())
        }
    }

    impl Adapter for HonestAdapter {
        fn descriptor(&self) -> AdapterDescriptor {
            AdapterDescriptor {
                id: "honest".to_owned(),
                languages: vec!["fake".to_owned()],
                capabilities: vec![Capability::StaticAnalysis],
                config_namespace: "honest".to_owned(),
            }
        }
        fn as_static_analysis(&self) -> Option<&dyn StaticAnalysisAdapter> {
            Some(self)
        }
    }

    /// DES-350/351/DS-1569「宣言capabilityと実装の不一致」片方向 —
    /// 宣言したのに実装が無い（`as_static_analysis` が `None`）。
    #[derive(Default)]
    struct DeclaresWithoutImplementing;

    impl Adapter for DeclaresWithoutImplementing {
        fn descriptor(&self) -> AdapterDescriptor {
            AdapterDescriptor {
                id: "declares-without-implementing".to_owned(),
                languages: vec!["fake".to_owned()],
                capabilities: vec![Capability::StaticAnalysis],
                config_namespace: "declares-without-implementing".to_owned(),
            }
        }
        // `as_static_analysis` の既定実装（`None`）をそのまま使う — 宣言と
        // 食い違う。
    }

    /// 不一致のもう片方向 — 実装しているのに宣言していない
    /// （`as_test_runner` が `Some` だが descriptor に `TestRunner` が無い）。
    #[derive(Default)]
    struct ImplementsWithoutDeclaring;

    impl TestRunnerAdapter for ImplementsWithoutDeclaring {
        fn id(&self) -> &'static str {
            "implements-without-declaring"
        }
        fn command(
            &self,
            _root: &Path,
            _execution: &ExecutionDescriptor,
            _coverage: bool,
            _coverage_output_path: Option<&Path>,
        ) -> Result<RunnerCommand, TestRunnerError> {
            unimplemented!("never invoked: registration is rejected before use")
        }
        fn parse(
            &self,
            _execution: &ExecutionDescriptor,
            _output: RunnerOutput<'_>,
        ) -> RunnerTestResult {
            unimplemented!("never invoked: registration is rejected before use")
        }
    }

    impl Adapter for ImplementsWithoutDeclaring {
        fn descriptor(&self) -> AdapterDescriptor {
            AdapterDescriptor {
                id: "implements-without-declaring".to_owned(),
                languages: vec!["fake".to_owned()],
                capabilities: Vec::new(),
                config_namespace: "implements-without-declaring".to_owned(),
            }
        }
        fn as_test_runner(&self) -> Option<&dyn TestRunnerAdapter> {
            Some(self)
        }
    }

    // 正本を検索したが、「宣言と実装が一致する adapter を registry が
    // *受理する*」ことだけを直接述べる条文は見当たらなかった — DES-351/
    // DS-1569/DS-1663 はいずれも拒否条件（不一致・重複・未登録）だけを
    // 定める。この受理側は拒否規則の論理的補集合であり嘘ではないが、
    // 「該当規範なし」と断定するのも早計なため、`@vtest` は付けず
    // 候補（DES-350: 宣言の存在、BD-007: registry経由の能力選択）を
    // ここに記す未確定のまま残す。W-SCAN-101 が出る無印の #[test] とする。
    #[test]
    fn register_accepts_a_consistent_declaration_and_resolves_the_typed_accessor() {
        let mut registry = AdapterRegistry::new();
        registry
            .register(Box::new(HonestAdapter))
            .expect("a consistent declaration must register");
        assert!(registry.static_analysis("honest").is_some());
        assert!(registry.test_runner("honest").is_none());
        assert!(registry.coverage("honest").is_none());
        assert_eq!(registry.ids().collect::<Vec<_>>(), vec!["honest"]);
    }

    /// DES-351/DS-1569/DS-1663「registryはadapter IDの重複…を拒否する」。
    /// @vtest.id TEST-ADAPTER-API-REGISTRY-REJECTS-DUPLICATE-ID
    /// @vtest.covers VO-ADAPTER-REGISTRY-REJECTS-DUPLICATE-ID
    /// @vtest.target crates/vtest-adapter-api/src/lib.rs::AdapterRegistry::register
    /// @vtest.intent verifies AdapterRegistry::register rejects a second adapter registered under an id already present
    #[test]
    fn register_rejects_a_duplicate_id() {
        let mut registry = AdapterRegistry::new();
        registry
            .register(Box::new(HonestAdapter))
            .expect("first registration must succeed");
        let error = registry
            .register(Box::new(HonestAdapter))
            .expect_err("a duplicate id must be rejected");
        assert_eq!(
            error,
            AdapterRegistrationError::DuplicateId {
                id: "honest".to_owned()
            }
        );
    }

    /// DES-351/DS-1569/DS-1663「宣言capabilityと実装の不一致」— 宣言した
    /// のに実装が無い方向。
    /// @vtest.id TEST-ADAPTER-API-REGISTRY-REJECTS-DECLARED-WITHOUT-IMPL
    /// @vtest.covers VO-ADAPTER-REGISTRY-REJECTS-CAPABILITY-MISMATCH
    /// @vtest.target crates/vtest-adapter-api/src/lib.rs::AdapterRegistry::register
    /// @vtest.intent verifies AdapterRegistry::register rejects an adapter that declares a capability it does not implement
    #[test]
    fn register_rejects_a_declared_but_unimplemented_capability() {
        let mut registry = AdapterRegistry::new();
        let error = registry
            .register(Box::new(DeclaresWithoutImplementing))
            .expect_err("declaring StaticAnalysis without implementing it must be rejected");
        assert_eq!(
            error,
            AdapterRegistrationError::DeclaredWithoutImplementation {
                id: "declares-without-implementing".to_owned(),
                capability: Capability::StaticAnalysis,
            }
        );
    }

    /// 不一致のもう片方向 — 実装しているのに宣言していない。
    /// @vtest.id TEST-ADAPTER-API-REGISTRY-REJECTS-IMPLEMENTED-WITHOUT-DECLARATION
    /// @vtest.covers VO-ADAPTER-REGISTRY-REJECTS-CAPABILITY-MISMATCH
    /// @vtest.target crates/vtest-adapter-api/src/lib.rs::AdapterRegistry::register
    /// @vtest.intent verifies AdapterRegistry::register rejects an adapter that implements a capability it does not declare
    #[test]
    fn register_rejects_an_implemented_but_undeclared_capability() {
        let mut registry = AdapterRegistry::new();
        let error = registry
            .register(Box::new(ImplementsWithoutDeclaring))
            .expect_err("implementing TestRunner without declaring it must be rejected");
        assert_eq!(
            error,
            AdapterRegistrationError::ImplementedWithoutDeclaration {
                id: "implements-without-declaring".to_owned(),
                capability: Capability::TestRunner,
            }
        );
    }

    /// 既存 VO `VO-ADAPTER-REGISTRY-REJECTS-UNREGISTERED`（DS-1569「registry
    /// は未登録のadapter IDに対する解決要求を拒否する（解決結果を返さない）」、
    /// 元は `vtest-adapter-rust::tests::unregistered_adapter_id_does_not_
    /// resolve` が `.get()` だけを観測）を、本 PR が追加した5つの型付き
    /// accessor 全体へ一般化して観測する。covers 先の VO・claim は移動して
    /// いない — 同じ VO を追加の観測者として厚くする。
    /// @vtest.id TEST-ADAPTER-API-REGISTRY-UNREGISTERED-ID-RESOLVES-NONE
    /// @vtest.covers VO-ADAPTER-REGISTRY-REJECTS-UNREGISTERED
    /// @vtest.target crates/vtest-adapter-api/src/lib.rs::AdapterRegistry::get
    /// @vtest.intent verifies an unregistered adapter id resolves to None through every typed accessor, not a panic or a fabricated adapter
    #[test]
    fn unregistered_id_resolves_to_none_everywhere() {
        let registry = AdapterRegistry::new();
        assert!(registry.get("nothing-registered").is_none());
        assert!(registry.source_discovery("nothing-registered").is_none());
        assert!(registry.static_analysis("nothing-registered").is_none());
        assert!(registry.test_runner("nothing-registered").is_none());
        assert!(registry.coverage("nothing-registered").is_none());
    }
}
