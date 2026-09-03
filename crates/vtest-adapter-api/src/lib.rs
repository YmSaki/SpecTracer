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
//! この PR では `ManagedTestDraft` / `DiscoveredTestDraft` / `SourceTargetDraft`
//! / `DiscoveryBatch` / `ManagedTestLink` / `DiscoveredTest` という仕様の型名を
//! 導入しない（`ManagedTestLink` / `DiscoveredTest` の新設は Owner 裁定待ち。
//! `pr3-decisions.md`「手を付けてはいけないもの」）。ここで定義する
//! `TestDraft` / `SourceDraft` / `DiscoveryOutcome` は、既存の
//! `vtest_model::TestEntity` / `SourceFunction` 形状を踏襲した、より小さな
//! 中間 DTO である。

use std::path::{Path, PathBuf};

use vtest_model::{
    Diagnostic, Locator, SourceLocation, SrcId, TargetRef, TestId, TestTarget, VoId,
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
/// normalization 済み construct bytes（この adapter では属性・doc comment を
/// 含む関数item全体。本冊:99）であり、core が `ContentHash::from_text` で
/// hash を計算する入力になる。
///
/// 必須 metadata（core 中立: id・`covers ≥ 1`・intent、および adapter 追加
/// 必須: `targets ≥ 1`。本冊 §4.4）を具体化できないTest構文は、adapterが
/// 診断（E-SCAN-005/006/007）だけを返し、`TestDraft` を生成しない
/// （`ManagedTestDraftLink::Missing` 相当）。したがってこの型のフィールドは
/// 必須 metadata について `Option` を持たない。
///
/// Test ID の大域的一意性（E-SCAN-002）と `covers` の VO 参照解決
/// （E-SCAN-003）は、本冊:571「VO参照の解決とTest IDの大局的一意性は
/// adapterではなくcoreが検査する」により、この draft を返す時点では未検査
/// である。同じ Test ID を宣言する複数の draft が返ることを許容する。
pub struct TestDraft {
    pub id: TestId,
    pub covers: Vec<VoId>,
    pub target: TargetRef,
    pub additional_targets: Vec<TargetRef>,
    pub intent: String,
    pub input: Option<String>,
    pub expect: Option<String>,
    pub kind: Option<String>,
    pub cases: Vec<String>,
    pub related: Vec<TestId>,
    pub location: SourceLocation,
    pub construct_text: String,
    pub filter: String,
    pub package: String,
    pub test_target: TestTarget,
}

/// hash 未計算の Source Target draft（本冊 §5.2 の `SourceTargetDraft` に
/// 相当する簡略版。`TestDraft` 同様、hash は core が計算する）。
pub struct SourceDraft {
    pub locator: Locator,
    pub src_id: Option<SrcId>,
    pub location: SourceLocation,
    pub construct_text: String,
}

/// 1 adapter の discovery 結果。Target Reference 解決（§6.1）、Test ID の
/// 大域的一意性、VO 参照解決は含まない — これらは core が複数 adapter の
/// 出力を統合してから行う（本冊 §5.1 手順4・5・7）。
pub struct DiscoveryOutcome {
    pub files_scanned: usize,
    pub tests: Vec<TestDraft>,
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

/// 登録済み adapter を ID で引く registry（本冊 §5.1 手順1「registryとconfig
/// の検証」・§6.1「coreはregistryで解決」）。PR3 時点では `rust-cargo` の
/// みを登録する。未知 adapter ID の扱い（E-CONFIG-001 か E-ADAPTER-001 か）は
/// 仕様の食い違いで Owner 裁定待ち（Issue #24）であり、この registry 自体は
/// 「該当実装が無ければ `None`」を返すだけで、その先の診断判断はしない。
#[derive(Default)]
pub struct AdapterRegistry {
    adapters: Vec<Box<dyn SourceDiscoveryAdapter>>,
}

impl AdapterRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register(&mut self, adapter: Box<dyn SourceDiscoveryAdapter>) {
        self.adapters.push(adapter);
    }

    pub fn get(&self, id: &str) -> Option<&dyn SourceDiscoveryAdapter> {
        self.adapters
            .iter()
            .find(|adapter| adapter.id() == id)
            .map(std::convert::AsRef::as_ref)
    }

    /// Registered adapter IDs, in registration order. Used by core to report
    /// the known-adapter list when it rejects a `config.yaml` adapter ID that
    /// `get` cannot resolve (fail-closed rejection of unregistered IDs; which
    /// diagnostic code that rejection carries is Issue #24's Owner-decision
    /// question, not this method's).
    pub fn ids(&self) -> impl Iterator<Item = &str> {
        self.adapters.iter().map(|adapter| adapter.id())
    }
}
