//! Canonical v0.1 verification: exactly four checks, exactly five states.
//!
//! SPEC-053「検証は `chain_integrity` / `orphan_detection` / `target_binding` /
//! `oracle_presence` の4検査のみで行う」、REQ-085「検証状態は `PASS` / `FAIL` /
//! `MISMATCH` / `NO_EVIDENCE` / `UNKNOWN` の 5 つのみとする」。
//!
//! 診断ラベルは状態とは別の field である（REQ-092「`MISSING` /
//! `NOT_EXECUTED` / `NOT_CHECKED` / `STALE` などは、状態に付随して原因を
//! 説明する診断ラベルであり、検証状態ではない」、SPEC-373「`NO_EVIDENCE` は
//! 状態であって診断ラベルではない」）。この crate は両者を
//! [`CheckOutcome::state`] と [`CheckOutcome::labels`] に分けて保持し、
//! どちらか一方へ畳み込む経路を持たない。
//!
//! 旧 12 項目モデル（`CheckValue` / `CheckItem` / `audits/`）はこの crate から
//! 完全に排除した。SPEC-400「旧モデルの12項目…は検査として存在しない」。

use std::{
    collections::{BTreeMap, BTreeSet},
    path::Path,
    process::Command,
};

use serde::Serialize;
use vtest_model::{
    AdapterId, CheckValue as EvidenceCheckValue, ContentHash, DiagnosticLabel, DocumentFile,
    EvidenceRecord, ManagedTestLink, SectionNode, SentenceNode, TargetRef, TestEntity,
    VerificationCheck, VerificationState, VoRecord,
};
use vtest_scan::ScanResult;
use vtest_store::{
    execution_state::{escape_risk, reconstruct_execution_state, ExecutionStateInputs},
    read_document_file, read_document_names, read_evidence, read_record_ids, read_vo_record,
    VerifyLayout,
};

/// 固定4検査。DS-1106「`--items` 省略時は常に固定4検査による完全検証を行う」、
/// DS-836「項目scopeが省略された場合、aggregatorはconfig値から部分集合を
/// 組み立てず、基本仕様 §5の固定4検査を選択する」。`config.verify.full_scope`
/// は項目選択 knob として使用しない（DS-1107 / DS-1109）。
pub const ALL_CHECKS: [VerificationCheck; 4] = [
    VerificationCheck::ChainIntegrity,
    VerificationCheck::OrphanDetection,
    VerificationCheck::TargetBinding,
    VerificationCheck::OraclePresence,
];

/// 構造検査。DS-838「aggregateは、chain_integrity / orphan_detectionを
/// repository / DOC / VO / TEST構造に対して評価する」。
pub const STRUCTURAL_CHECKS: [VerificationCheck; 2] = [
    VerificationCheck::ChainIntegrity,
    VerificationCheck::OrphanDetection,
];

/// Test 単位で評価する検査。DS-840「aggregateは、各TESTについて、scopeの
/// 検査軸に含まれるtarget_binding / oracle_presenceを評価する」。
pub const PER_TEST_CHECKS: [VerificationCheck; 2] = [
    VerificationCheck::TargetBinding,
    VerificationCheck::OraclePresence,
];

/// Canonical check name, as it appears in `--items` and in the JSON wire form.
pub fn check_name(check: VerificationCheck) -> &'static str {
    match check {
        VerificationCheck::ChainIntegrity => "chain_integrity",
        VerificationCheck::OrphanDetection => "orphan_detection",
        VerificationCheck::TargetBinding => "target_binding",
        VerificationCheck::OraclePresence => "oracle_presence",
    }
}

/// Parse an `--items` value. Unknown names are rejected rather than ignored:
/// silently dropping an unrecognised item would narrow the requested scope
/// without saying so, which DS-1113 forbids.
pub fn parse_check(name: &str) -> Option<VerificationCheck> {
    ALL_CHECKS
        .into_iter()
        .find(|check| check_name(*check) == name)
}

/// Entity-axis scope. DS-1104「scope は2軸であり、`--items` が検査軸（4検査の
/// 部分集合）、`--doc` / `--vo` / `--test` がエンティティ軸（部分木）である」。
///
/// 旧モデルの `--req` は持たない。DS-1105「旧モデルの `--spec` / `--req` は
/// 廃止し、`--req` は除去する」。
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(tag = "kind", content = "id", rename_all = "lowercase")]
pub enum EntityScope {
    Doc(String),
    Vo(String),
    Test(String),
}

impl EntityScope {
    pub fn id(&self) -> &str {
        match self {
            Self::Doc(id) | Self::Vo(id) | Self::Test(id) => id,
        }
    }
}

/// 最上位 field `scope`。DS-1114「`--format json` では同じ内容を最上位 field
/// `scope`（§12.1）として返し、完全検証の場合も省略しない」、DS-947。
#[derive(Clone, Debug, Serialize)]
pub struct ScopeReport {
    pub requested_checks: Vec<VerificationCheck>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub entity: Option<EntityScope>,
    /// DS-1111「限定 scope の結果を完全検証 OK と表示しない」。
    pub limited: bool,
    /// DS-1113「scope を限定した場合、出力冒頭に要求 scope と「scope 外は
    /// 未検証」の旨を必ず表示する」。
    pub outside_scope_is_unverified: bool,
}

/// One check's result at one evaluation point.
///
/// `state` と `labels` は別 field である。`basis` は根拠テキストで、DS-1513
/// 「各非PASSの根拠…を text / JSON で返す」に対応する。
#[derive(Clone, Debug, Serialize)]
pub struct CheckOutcome {
    pub check: VerificationCheck,
    pub state: VerificationState,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub labels: Vec<DiagnosticLabel>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub basis: Vec<String>,
}

impl CheckOutcome {
    fn new(
        check: VerificationCheck,
        state: VerificationState,
        labels: Vec<DiagnosticLabel>,
        basis: Vec<String>,
    ) -> Self {
        Self {
            check,
            state,
            labels,
            basis,
        }
    }

    fn pass(check: VerificationCheck, basis: impl Into<String>) -> Self {
        Self::new(
            check,
            VerificationState::Pass,
            Vec::new(),
            vec![basis.into()],
        )
    }

    /// scope 外の検査。DS-840「含まれない検査はNO_EVIDENCE、診断NOT_CHECKED」、
    /// DS-1110「scope 外・未実施の検査は `NO_EVIDENCE`（診断 `NOT_CHECKED`）と
    /// して保持し、`PASS` へ変換しない」。
    fn out_of_scope(check: VerificationCheck) -> Self {
        Self::new(
            check,
            VerificationState::NoEvidence,
            vec![DiagnosticLabel::NotChecked],
            vec!["outside the requested check scope (--items)".to_owned()],
        )
    }
}

/// A node of the DOC → VO → Test aggregation tree.
/// DS-1513「report は DOC → VO → Test の構造…を text / JSON で返す」。
#[derive(Clone, Debug, Serialize)]
pub struct TreeNode {
    pub kind: NodeKind,
    pub id: String,
    /// この部分木の代表値（DS-871 の優先順位による fail-closed 合成）。
    pub state: VerificationState,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub checks: Vec<CheckOutcome>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub children: Vec<TreeNode>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum NodeKind {
    Doc,
    Vo,
    Test,
}

/// The complete outcome of one `vtest verify` run.
#[derive(Clone, Debug, Serialize)]
pub struct VerifyOutcome {
    pub scope: ScopeReport,
    /// 構造検査。宣言鎖全体に対する検査であり、エンティティ軸の限定では
    /// 縮まない（REQ-295）。
    pub structural: Vec<CheckOutcome>,
    pub tree: Vec<TreeNode>,
    /// 評価地点が1件も無かった検査。
    ///
    /// DS-840 は `target_binding` / `oracle_presence` を「各TESTについて」
    /// 評価すると定める。TEST が 1 件も無ければ、これらの検査は実施されて
    /// いない — 空虚に `PASS` なのではない。DS-252「`vtest verify` は正典
    /// または検証事実の欠落を対応する非 `PASS` 値として表示する」、DS-253
    /// 「`vtest verify` は部分的な登録・判断・実行状態を総合 `OK` として
    /// 扱わない」に従い、`NO_EVIDENCE`（診断 `NOT_CHECKED`）として明示的に
    /// 持ち、代表値へ算入する。
    ///
    /// これが空でない結果は、定義上、完全検証 OK ではない。
    pub unevaluated: Vec<CheckOutcome>,
    /// 集約代表値。DS-870「集約代表値は、要求scope内で評価した全値…を
    /// §11.3のfail-closed規則で合成した1値とする」。
    pub state: VerificationState,
    /// 総合判定。DS-844「総合判定は、構造検査…とentity treeのscope内評価が
    /// すべてPASSならOK、それ以外ならNGとする」。SPEC-333「利用者向け簡易出力は
    /// `OK` / `NG`の二値とする」。検証状態とは別軸の field である。
    pub ok: bool,
}

impl VerifyOutcome {
    /// Every check outcome in the whole result, structural and per-Test.
    pub fn all_outcomes(&self) -> Vec<&CheckOutcome> {
        fn walk<'a>(node: &'a TreeNode, sink: &mut Vec<&'a CheckOutcome>) {
            sink.extend(node.checks.iter());
            for child in &node.children {
                walk(child, sink);
            }
        }
        let mut sink = self.structural.iter().collect::<Vec<_>>();
        sink.extend(self.unevaluated.iter());
        for node in &self.tree {
            walk(node, &mut sink);
        }
        sink
    }
}

/// fail-closed 合成と代表値選択。
///
/// DS-1511「集約は fail-closed とし、子に 1 つでも非 `PASS` があれば親は
/// 非 `PASS`」。DS-871「全値が`PASS`なら代表値は`PASS`…非`PASS`が混在する
/// 場合は…優先順位`FAIL > MISMATCH > NO_EVIDENCE > UNKNOWN`で選ぶ」。
///
/// この関数は `VerificationState` に `Ord` を実装せず、局所的な match で
/// 優先順位を与える — DS-873「5状態に順序・優劣・包含関係を設けない」を
/// 型の上で守るため。
///
/// # 空入力の危険
///
/// 空の iterator を渡すと `PASS` を返す（`fold` の初期値）。これは
/// 「何も確かめていない」を「合格」と読み替える経路であり、このツールが
/// 防ぐべき偽 `PASS` そのものの形をしている。**呼び出し側は、集約点が空に
/// なりうる場合、必ず先に `NO_EVIDENCE`（診断 `NOT_CHECKED`）を注入して
/// から呼ぶこと。** この crate 内の呼び出しは 3 箇所とも注入済みである
/// （`node_from_children` の空検査・空子ノード、`verify_project` の
/// `unevaluated`）。
pub fn representative(states: impl IntoIterator<Item = VerificationState>) -> VerificationState {
    fn rank(state: VerificationState) -> u8 {
        match state {
            VerificationState::Fail => 4,
            VerificationState::Mismatch => 3,
            VerificationState::NoEvidence => 2,
            VerificationState::Unknown => 1,
            VerificationState::Pass => 0,
        }
    }
    states
        .into_iter()
        .fold(VerificationState::Pass, |left, right| {
            if rank(right) > rank(left) {
                right
            } else {
                left
            }
        })
}

/// Verify the project.
///
/// `requested_checks` が `None` のときは固定4検査（DS-1106）。`Some` のときは
/// その明示的部分集合だけを限定 scope とする（DS-1110）。
/// Bundles the DS-265/DS-816-825 validity inputs `evaluate_target_binding`
/// needs beyond the current `TestEntity`/`ScanResult`: the latest recorded
/// Evidence per Test, the current adapter identity, and the current HEAD
/// revision.
///
/// **Disclosed scope limit**: `current_adapter` is a single fixed value
/// (`rust-cargo`) rather than resolved per-Test, because this repository and
/// this slice's only Evidence writer (`vtest-exec`) both use exactly one
/// adapter today. DS-817/DS-1628/DS-824 (per-Test adapter identity match)
/// are evaluated against this fixed value; a multi-adapter repository is
/// out of this slice's scope and not represented here.
struct EvidenceContext {
    root: std::path::PathBuf,
    latest_by_test: BTreeMap<String, EvidenceRecord>,
    current_adapter: AdapterId,
    head_commit: Option<String>,
}

impl EvidenceContext {
    fn load(root: &Path, layout: &VerifyLayout) -> Self {
        let mut latest_by_test: BTreeMap<String, EvidenceRecord> = BTreeMap::new();
        if let Ok(entries) = std::fs::read_dir(layout.evidence_dir()) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().and_then(|ext| ext.to_str()) != Some("yaml") {
                    continue;
                }
                let Ok(record) = read_evidence(&path) else {
                    continue;
                };
                let test_id = record.test_id.as_str().to_owned();
                // Evidence record ids are ULIDs (DES-032), which sort
                // lexicographically in creation-time order, so the greatest
                // `id` string among a Test's records is the latest one.
                match latest_by_test.get(&test_id) {
                    Some(existing) if existing.id >= record.id => {}
                    _ => {
                        latest_by_test.insert(test_id, record);
                    }
                }
            }
        }
        Self {
            root: root.to_owned(),
            latest_by_test,
            current_adapter: AdapterId::new("rust-cargo"),
            head_commit: git_head_commit(root),
        }
    }
}

fn git_head_commit(root: &Path) -> Option<String> {
    Command::new("git")
        .current_dir(root)
        .args(["rev-parse", "HEAD"])
        .output()
        .ok()
        .filter(|output| output.status.success())
        .map(|output| String::from_utf8_lossy(&output.stdout).trim().to_owned())
        .filter(|value| !value.is_empty())
}

pub fn verify_project(
    root: &std::path::Path,
    scan: &ScanResult,
    requested_checks: Option<&[VerificationCheck]>,
    entity_scope: Option<EntityScope>,
) -> VerifyOutcome {
    let layout = VerifyLayout::new(root);
    let evidence = EvidenceContext::load(root, &layout);
    let selected: BTreeSet<VerificationCheck> = match requested_checks {
        None => ALL_CHECKS.into_iter().collect(),
        Some(checks) => checks.iter().copied().collect(),
    };
    let limited = selected.len() < ALL_CHECKS.len() || entity_scope.is_some();

    let vos = read_vo_records(&layout);
    let document_node_ids = read_document_node_ids(&layout);

    // 構造検査は宣言鎖全体に対する検査であり、エンティティ軸の限定では
    // 縮まない。REQ-295「完全検証における OK は、宣言鎖全体に対する検査
    // （chain_integrity / orphan_detection）と、scope に含まれる各「宣言 +
    // コード + 証拠」の組に対する検査（target_binding / oracle_presence）が
    // すべて `PASS` であり…」。
    let structural = STRUCTURAL_CHECKS
        .into_iter()
        .map(|check| {
            if !selected.contains(&check) {
                return CheckOutcome::out_of_scope(check);
            }
            match check {
                VerificationCheck::ChainIntegrity => {
                    evaluate_chain_integrity(scan, &vos, &document_node_ids)
                }
                VerificationCheck::OrphanDetection => evaluate_orphan_detection(scan),
                _ => unreachable!("STRUCTURAL_CHECKS holds only the two structural checks"),
            }
        })
        .collect::<Vec<_>>();

    let selection = EntitySelection::new(&vos, scan, entity_scope.as_ref());
    let tree = build_tree(&vos, scan, &selection, &selected, &evidence);

    // 評価地点が1件も無い per-Test 検査を、空虚な `PASS` にせず明示する。
    // TEST が 1 件も無い repository（初期化直後など）で総合 OK を返すことは、
    // このツールが防ぐべき偽 `PASS` そのものである（DS-252 / DS-253）。
    let has_test_node = tree.iter().any(contains_test_node);
    let unevaluated = if has_test_node {
        Vec::new()
    } else {
        PER_TEST_CHECKS
            .into_iter()
            .filter(|check| selected.contains(check))
            .map(|check| {
                CheckOutcome::new(
                    check,
                    VerificationState::NoEvidence,
                    vec![DiagnosticLabel::NotChecked],
                    vec![
                        "no Test exists in the requested entity scope, so this check has \
                         no evaluation point (DS-840)"
                            .to_owned(),
                    ],
                )
            })
            .collect()
    };

    let state = representative(
        structural
            .iter()
            .map(|outcome| outcome.state)
            .chain(unevaluated.iter().map(|outcome| outcome.state))
            .chain(tree.iter().map(|node| node.state)),
    );

    VerifyOutcome {
        scope: ScopeReport {
            requested_checks: ALL_CHECKS
                .into_iter()
                .filter(|check| selected.contains(check))
                .collect(),
            entity: entity_scope,
            limited,
            outside_scope_is_unverified: limited,
        },
        structural,
        tree,
        unevaluated,
        state,
        ok: state == VerificationState::Pass,
    }
}

fn contains_test_node(node: &TreeNode) -> bool {
    node.kind == NodeKind::Test || node.children.iter().any(contains_test_node)
}

// ---------------------------------------------------------------------------
// chain_integrity
// ---------------------------------------------------------------------------

/// scan が報告する整合性診断のうち `chain_integrity = MISMATCH` へ写像される
/// コード。
///
/// - DS-561「`ManagedTestLink::Multiple`、E-SCAN-002（Test ID衝突）、
///   E-SCAN-003（解決不能なVO参照）は `chain_integrity = MISMATCH` に写像する」
/// - DS-562「E-SCAN-008（VO parent不在・循環）、E-SCAN-009（Relation dangling）、
///   E-SCAN-012（文書鎖・VO derives_fromのリンク切れ）は
///   `chain_integrity = MISMATCH` に写像する」
/// - DS-902（E-SCAN-017）「当該VOの`chain_integrity`を`MISMATCH`とし…」
const CHAIN_INTEGRITY_CODES: [&str; 6] = [
    "E-SCAN-002",
    "E-SCAN-003",
    "E-SCAN-008",
    "E-SCAN-009",
    "E-SCAN-012",
    "E-SCAN-017",
];

/// `chain_integrity = MISMATCH`（診断 `MISSING`）へ写像されるコード。
/// DS-560「管理宣言の欠落・E-SCAN-007（必須metadata欠落）が示す
/// `ManagedTestLink::Missing` は `chain_integrity = MISMATCH`（診断 `MISSING`）
/// に写像する」。
const CHAIN_INTEGRITY_MISSING_CODES: [&str; 1] = ["E-SCAN-007"];

fn evaluate_chain_integrity(
    scan: &ScanResult,
    vos: &BTreeMap<String, VoRecord>,
    document_node_ids: &BTreeSet<String>,
) -> CheckOutcome {
    let mut basis = Vec::new();
    let mut labels = BTreeSet::new();
    let mut mismatch = false;

    for diagnostic in &scan.diagnostics {
        if !diagnostic.is_error() {
            continue;
        }
        let code = diagnostic.code.as_str();
        if CHAIN_INTEGRITY_MISSING_CODES.contains(&code) {
            mismatch = true;
            labels.insert(DiagnosticLabel::Missing);
            basis.push(format!("[{code}] {}", diagnostic.message));
        } else if CHAIN_INTEGRITY_CODES.contains(&code) || code == "E-SCAN-010" {
            // E-SCAN-010 → `chain_integrity = MISMATCH`.
            //
            // DS-1679「`E-SCAN-010` により受理されなかった宣言鎖レコード
            // （VO・Test・上流文書ノード）は、存在しないものとして集約から
            // 取り除かず、当該レコードの`chain_integrity`を`MISMATCH`と
            // する」。これで前身の blanket 写像（本 slice の stopped_on S4）
            // は解消した — レコードを「存在しない」と読むと、覆う Test を
            // 要求する義務ごと集約木から消え、REQ-056 / ROOT-034 の双方向
            // 完全性が空振りする。
            //
            // DS-1679 の後半は Evidence レコードの `E-SCAN-010` を除外し、
            // DS-476（当該Evidenceを有効な結果に使用しない）＋ DS-278
            // （`NO_EVIDENCE`／診断 `NOT_EXECUTED`）へ送る。この分岐は本
            // slice では到達不能である: `scan_project` は Evidence レコードを
            // 検証せず（`read_evidence` は `operations::show_test` 専用）、
            // したがって scan 診断に Evidence 由来の `E-SCAN-010` は現れない。
            // 到達不能な分岐をメッセージ文字列の照合で書くと、後で Evidence
            // 検証が入ったときに黙って誤分類するので、ここでは書かない。
            // DS-054 / DS-1677（上流文書ノード id 衝突）も同じ `MISMATCH`。
            mismatch = true;
            basis.push(format!("[{code}] {}", diagnostic.message));
        }
    }

    // DS-560「管理宣言の欠落…が示す `ManagedTestLink::Missing` は
    // `chain_integrity = MISMATCH`（診断 `MISSING`）に写像する」。
    // `discovered` を直接読む: 管理宣言を欠く construct は警告
    // （W-SCAN-101）としてしか診断に現れない場合があり、error 診断の
    // フィルタでは取りこぼす。DS-1510「管理済みgraph側の他検査がすべて
    // PASSでも、未登録Testが1件あれば`chain_integrity`により総合NGになる」。
    let unmanaged = scan
        .discovered
        .iter()
        .filter(|discovered| matches!(discovered.managed, ManagedTestLink::Missing))
        .map(|discovered| discovered.location.locator.clone())
        .collect::<BTreeSet<_>>();
    if !unmanaged.is_empty() {
        mismatch = true;
        labels.insert(DiagnosticLabel::Missing);
        basis.push(format!(
            "{} discovered Test construct(s) have no management declaration: {}",
            unmanaged.len(),
            join_ids(&unmanaged)
        ));
    }

    // DS-561「`ManagedTestLink::Multiple`、E-SCAN-002（Test ID衝突）、
    // E-SCAN-003（解決不能なVO参照）は `chain_integrity = MISMATCH` に
    // 写像する」。`Multiple` は「同一 Test construct から複数 draft が生じた」
    // 状態であって Test ID の大域的衝突ではない（後者は E-SCAN-002）。
    // DS-561 が両者を並列に列挙しているとおり、別個の違反として数える。
    //
    // 診断ラベルは付けない: 正典は `Multiple` にラベルを割り当てておらず、
    // 発明しない。
    let multiple = scan
        .discovered
        .iter()
        .filter(|discovered| matches!(discovered.managed, ManagedTestLink::Multiple(_)))
        .map(|discovered| discovered.location.locator.clone())
        .collect::<BTreeSet<_>>();
    if !multiple.is_empty() {
        mismatch = true;
        basis.push(format!(
            "{} discovered Test construct(s) produced multiple management declarations \
             (ManagedTestLink::Multiple, DS-561): {}",
            multiple.len(),
            join_ids(&multiple)
        ));
    }

    // DS-812「`covers`を持たない（0件の）Testは管理宣言不整合として
    // `chain_integrity = MISMATCH`であり、特別扱いの分岐を設けない」。
    let uncovered = scan
        .tests
        .iter()
        .filter(|test| test.covers.is_empty())
        .map(|test| test.id.as_str().to_owned())
        .collect::<BTreeSet<_>>();
    if !uncovered.is_empty() {
        mismatch = true;
        basis.push(format!(
            "{} Test(s) declare no covers: {}",
            uncovered.len(),
            join_ids(&uncovered)
        ));
    }

    // REQ-054「VO 層では、各 VO が 1 件以上の `document` への解決可能な
    // derives_from を持つことを要求する」。参照解決自体は scan が
    // E-SCAN-012 で報告するが、`.verify/doc/` が読めず document ノードが
    // 1件も無い場合も VO の derives_from は解決できない。
    let unresolved_vos = vos
        .iter()
        .filter(|(_, record)| {
            record.derives_from.is_empty()
                || record
                    .derives_from
                    .iter()
                    .any(|entry| !document_node_ids.contains(entry.doc.as_str()))
        })
        .map(|(id, _)| id.clone())
        .collect::<BTreeSet<_>>();
    if !unresolved_vos.is_empty() {
        mismatch = true;
        basis.push(format!(
            "{} VO(s) have a derives_from that does not resolve to a document node: {}",
            unresolved_vos.len(),
            join_ids(&unresolved_vos)
        ));
    }

    // REQ-056「leaf VO から Test への方向（検証実装の存在）と、発見された
    // Test から宣言への方向（管理宣言の解決）の両方が成立して初めて、
    // 宣言鎖の双方向完全性が成立する」。ROOT-034 が旧 `test_existence` を
    // `chain_integrity` へ統合した以上、覆う Test を持たない leaf VO は
    // この検査で捕まえなければ、どの検査でも捕まらない。
    //
    // この事象に対応する診断ラベルは正典に明記が無い（stopped_on）。
    // ラベルを発明せず、根拠テキストだけを添えて `MISMATCH` とする。
    let parents = vos
        .values()
        .filter_map(|record| record.parent.as_ref().map(|p| p.as_str().to_owned()))
        .collect::<BTreeSet<_>>();
    let covered = scan
        .tests
        .iter()
        .flat_map(|test| test.covers.iter().map(|vo| vo.as_str().to_owned()))
        .collect::<BTreeSet<_>>();
    let uncovered_leaves = vos
        .keys()
        .filter(|id| !parents.contains(*id) && !covered.contains(*id))
        .cloned()
        .collect::<BTreeSet<_>>();
    if !uncovered_leaves.is_empty() {
        mismatch = true;
        basis.push(format!(
            "{} leaf VO(s) have no covering Test: {}",
            uncovered_leaves.len(),
            join_ids(&uncovered_leaves)
        ));
    }

    if mismatch {
        CheckOutcome::new(
            VerificationCheck::ChainIntegrity,
            VerificationState::Mismatch,
            labels.into_iter().collect(),
            basis,
        )
    } else {
        CheckOutcome::pass(
            VerificationCheck::ChainIntegrity,
            format!(
                "declaration chain complete over {} document node(s), {} VO(s), {} Test(s)",
                document_node_ids.len(),
                vos.len(),
                scan.tests.len()
            ),
        )
    }
}

// ---------------------------------------------------------------------------
// orphan_detection
// ---------------------------------------------------------------------------

/// DS-1651「`orphan_detection`は評価地点をノードとし、`root` 層のノードを除き
/// 実効的な上流（自分の辺 ∪ 先祖の辺）を持たないノードが無ければ`PASS`、
/// あれば`MISMATCH`とする」。DS-1647 / DS-1650 は同条件を E-SCAN-016 として
/// 報告することを定める。孤児判定そのものは scan が所有する。
fn evaluate_orphan_detection(scan: &ScanResult) -> CheckOutcome {
    let orphans = scan
        .diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.is_error() && diagnostic.code == "E-SCAN-016")
        .map(|diagnostic| diagnostic.message.clone())
        .collect::<Vec<_>>();
    if orphans.is_empty() {
        // REQ-062「対象は文書層のみとする」。文書ノードが1件も無い
        // repository ではこの `PASS` は空虚に成立する（DS-1651 の字義）。
        // 根拠テキストにその事実を残す。
        CheckOutcome::pass(
            VerificationCheck::OrphanDetection,
            "no document node lacks an effective upstream (E-SCAN-016 not reported)",
        )
    } else {
        CheckOutcome::new(
            VerificationCheck::OrphanDetection,
            VerificationState::Mismatch,
            Vec::new(),
            orphans
                .into_iter()
                .map(|message| format!("[E-SCAN-016] {message}"))
                .collect(),
        )
    }
}

// ---------------------------------------------------------------------------
// target_binding / oracle_presence (per Test)
// ---------------------------------------------------------------------------

/// One Test construct's unresolvable-target diagnostics.
///
/// DS-1678「全宣言targetのうち1件でも解決できない場合、当該targetの
/// `target_binding`は、対象が存在しない場合（E-SCAN-004）は`MISMATCH`
/// （診断`MISSING`）、複数候補により曖昧な場合（E-SCAN-011）は`MISMATCH`とし、
/// 当該Testの`target_binding`もfail-closed合成により`MISMATCH`とする。
/// Evidenceが生成されないこと（DS-755）を理由に`NO_EVIDENCE`
/// （診断`NOT_EXECUTED`）へ倒してはならない」。
///
/// DS-1678 retires the predecessor DS-756, which had held this case at
/// `NO_EVIDENCE` / `NOT_EXECUTED`; DS-756 no longer exists in the canon.
#[derive(Default)]
struct TargetResolution {
    /// E-SCAN-004: the declared target does not exist.
    missing: Vec<String>,
    /// E-SCAN-011: the declared target is ambiguous across candidates.
    ambiguous: Vec<String>,
}

impl TargetResolution {
    fn is_unresolved(&self) -> bool {
        !self.missing.is_empty() || !self.ambiguous.is_empty()
    }

    fn basis(&self) -> Vec<String> {
        self.missing
            .iter()
            .chain(self.ambiguous.iter())
            .cloned()
            .collect()
    }
}

fn target_resolution_diagnostics(scan: &ScanResult) -> BTreeMap<String, TargetResolution> {
    let mut by_locator: BTreeMap<String, TargetResolution> = BTreeMap::new();
    for diagnostic in &scan.diagnostics {
        let Some(location) = &diagnostic.location else {
            continue;
        };
        let entry = by_locator.entry(location.locator.clone()).or_default();
        let text = format!("[{}] {}", diagnostic.code, diagnostic.message);
        match diagnostic.code.as_str() {
            "E-SCAN-004" => entry.missing.push(text),
            "E-SCAN-011" => entry.ambiguous.push(text),
            _ => {}
        }
    }
    by_locator
}

/// Resolves the current implementation-construct hash for every
/// `TargetRef::Locator` target `test` declares, as a set (DS-265/818's
/// "target参照集合").
///
/// **Disclosed scope limit**: `TargetRef::SrcId` targets are not resolved
/// here — doing so needs a SRC-record lookup this crate does not yet wire
/// in for this purpose. Returns `None` if any declared target cannot be
/// hash-resolved this way (a `SrcId` target, or a `Locator` target not
/// present in `scan.sources`), which the caller folds into DS-819/827's
/// "does not match the current canonical set" outcome rather than silently
/// skipping the check.
fn current_target_hashes(test: &TestEntity, scan: &ScanResult) -> Option<BTreeSet<ContentHash>> {
    let mut hashes = BTreeSet::new();
    for target in &test.targets {
        let TargetRef::Locator(locator) = target else {
            return None;
        };
        let found = scan
            .sources
            .iter()
            .find(|source| &source.locator == locator)?;
        hashes.insert(found.content_hash.clone());
    }
    Some(hashes)
}

/// DS-818's full AND-condition, decomposed into DS-816/817/819/820/821/822/
/// 1628/824's individual named failure branches. Returns `Ok(())` only when
/// every condition holds (the Evidence is current); otherwise returns the
/// `CheckOutcome` DS-833 says to hold instead of reusing the Evidence.
///
/// Branch order is this crate's own tie-break where more than one condition
/// could independently fail at once (the canon states each condition as an
/// independent AND-term, not a priority order): adapter identity first,
/// then subject/target hashes, then revision, then Execution State. This is
/// implementation discretion, not a normative claim.
fn evidence_validity_failure(
    test: &TestEntity,
    record: &EvidenceRecord,
    scan: &ScanResult,
    evidence: &EvidenceContext,
) -> Option<CheckOutcome> {
    // DS-1628: adapter explicitly does not match.
    if record.adapter != evidence.current_adapter {
        return Some(CheckOutcome::new(
            VerificationCheck::TargetBinding,
            VerificationState::NoEvidence,
            vec![DiagnosticLabel::Stale],
            vec![format!(
                "Evidence adapter {:?} does not match the current adapter {:?} (DS-1628)",
                record.adapter, evidence.current_adapter
            )],
        ));
    }

    // DS-819/DS-827: current Test subject hash or current target hash set
    // does not match the recorded one (including a target that no longer
    // resolves to exactly the recorded canonical Locator set, or that this
    // crate cannot resolve at all — see `current_target_hashes`'s disclosed
    // `SrcId` gap).
    let current_targets = current_target_hashes(test, scan);
    let recorded_targets: BTreeSet<ContentHash> =
        record.hashes.target_fns.iter().cloned().collect();
    let subject_matches = record.hashes.test_fn == test.content_hash;
    let targets_match = current_targets.as_ref() == Some(&recorded_targets);
    if !subject_matches || !targets_match {
        return Some(CheckOutcome::new(
            VerificationCheck::TargetBinding,
            VerificationState::NoEvidence,
            vec![DiagnosticLabel::Stale],
            vec![
                "Evidence test subject or target reference set does not match \
                  the current canonical set (DS-819/DS-827)"
                    .to_owned(),
            ],
        ));
    }

    // DS-820: revision must be present and match the current HEAD.
    let revision_matches = match (&record.revision.commit, &evidence.head_commit) {
        (Some(recorded), Some(current)) => recorded == current,
        _ => false,
    };
    if !revision_matches {
        return Some(CheckOutcome::new(
            VerificationCheck::TargetBinding,
            VerificationState::NoEvidence,
            vec![DiagnosticLabel::Stale],
            vec!["Evidence revision is absent or does not match the current \
                  HEAD revision (DS-820)"
                .to_owned()],
        ));
    }

    // DS-822: `execution_state.complete` must be `true`, AND the current
    // Execution State subject must be fully reconstructible for comparison
    // (`vtest_store::execution_state::reconstruct_execution_state` — the
    // same function `vtest-exec` uses to write the record in the first
    // place, so an unchanged environment reconstructs an identical hash).
    // The recorded runner kind/invocation are reused rather than
    // independently re-derived: any *declared* difference (a changed
    // `execution.selector`, a changed target) already fails the DS-819
    // subject-hash check above, so re-deriving them here would only ever
    // reproduce the record's own values in the non-drift case this branch
    // exists to confirm.
    if !record.execution_state.complete {
        return Some(CheckOutcome::new(
            VerificationCheck::TargetBinding,
            VerificationState::Unknown,
            Vec::new(),
            vec!["recorded execution_state.complete is not true (DS-822)".to_owned()],
        ));
    }
    let current_state = reconstruct_execution_state(
        &evidence.root,
        ExecutionStateInputs {
            adapter: &evidence.current_adapter,
            schema: &record.execution_state.schema,
            head_commit: evidence.head_commit.as_deref(),
            runner_kind: &record.runner.kind,
            invocation: &record.runner.command,
        },
    );
    if !current_state.complete {
        let reason = escape_risk(&evidence.root)
            .unwrap_or_else(|| "the current environment could not be fully snapshotted".to_owned());
        return Some(CheckOutcome::new(
            VerificationCheck::TargetBinding,
            VerificationState::Unknown,
            Vec::new(),
            vec![format!(
                "current Execution State subject could not be reconstructed for \
                 comparison (DS-822): {reason}"
            )],
        ));
    }

    // DS-821: a present-but-different hash is a genuine mismatch, not an
    // inability to tell — `NO_EVIDENCE` (STALE), never `UNKNOWN`.
    if record.execution_state.hash != current_state.hash {
        return Some(CheckOutcome::new(
            VerificationCheck::TargetBinding,
            VerificationState::NoEvidence,
            vec![DiagnosticLabel::Stale],
            vec!["recorded Execution State subject hash does not match the \
                  current one (DS-821)"
                .to_owned()],
        ));
    }

    None
}

fn evaluate_target_binding(
    test: &TestEntity,
    resolution: &TargetResolution,
    scan: &ScanResult,
    evidence: &EvidenceContext,
) -> CheckOutcome {
    // DS-1664「targetを持たないTestの`target_binding`は`NO_EVIDENCE`
    // （診断`NOT_CHECKED`）とする」。
    if test.targets.is_empty() {
        return CheckOutcome::new(
            VerificationCheck::TargetBinding,
            VerificationState::NoEvidence,
            vec![DiagnosticLabel::NotChecked],
            vec!["Test declares no target (DS-1664)".to_owned()],
        );
    }

    // DS-1678: an unresolvable declared target is `MISMATCH`, and must NOT be
    // folded to `NO_EVIDENCE` / `NOT_EXECUTED` on the grounds that no Evidence
    // could be produced. This is evaluated BEFORE the Evidence-absence rule
    // below, because DS-1678 names that fallback as the specific error to
    // avoid: no Evidence can exist for a target that does not resolve
    // (DS-755 forbids generating it), so telling the reader to "create
    // evidence" would direct them at work that cannot be done and would hide
    // the break on the declaration side.
    //
    // Diagnostic labels: `MISSING` accompanies the target-absent case
    // (E-SCAN-004) per DS-1678. For the ambiguous case (E-SCAN-011) the canon
    // states no label and DS-1678 explicitly declines to forbid one
    // (「E-SCAN-011 について診断ラベルを禁じてはいない（正典は沈黙しており、
    // ここで禁止を新設しない）」), so none is invented here.
    if resolution.is_unresolved() {
        let mut labels = Vec::new();
        if !resolution.missing.is_empty() {
            labels.push(DiagnosticLabel::Missing);
        }
        return CheckOutcome::new(
            VerificationCheck::TargetBinding,
            VerificationState::Mismatch,
            labels,
            resolution.basis(),
        );
    }

    // DS-825「Evidenceなしの場合、`NO_EVIDENCE`（診断NOT_EXECUTED）とする」。
    let Some(record) = evidence.latest_by_test.get(test.id.as_str()) else {
        return CheckOutcome::new(
            VerificationCheck::TargetBinding,
            VerificationState::NoEvidence,
            vec![DiagnosticLabel::NotExecuted],
            vec![format!(
                "no Evidence for {} declared target(s) (DS-825)",
                test.targets.len()
            )],
        );
    };

    // DS-833: an Evidence record that exists but is not valid per DS-818 is
    // never reused; the caller keeps the specific MISMATCH/NO_EVIDENCE
    // (STALE)/UNKNOWN this returns instead.
    if let Some(outcome) = evidence_validity_failure(test, record, scan, evidence) {
        return outcome;
    }

    // DS-830/831/832: the Evidence is valid.
    dynamic_result_from_evidence(record)
}

/// DS-830/831/832: judgment applied once an Evidence record is confirmed
/// valid (DS-818). Kept as its own function so it is unit-testable without
/// first achieving end-to-end validity (see `evaluate_target_binding`'s
/// doc comment on why that path is not reachable yet in this crate).
fn dynamic_result_from_evidence(record: &EvidenceRecord) -> CheckOutcome {
    // DS-830: the runner itself reported failure.
    if record.result == vtest_model::TestResult::Fail {
        return CheckOutcome::new(
            VerificationCheck::TargetBinding,
            VerificationState::Fail,
            Vec::new(),
            vec!["test runner reported FAIL (DS-830)".to_owned()],
        );
    }

    // DS-831/832: the runner passed; the coverage measurement decides.
    let coverage = &record.target_execution;
    if !coverage.checked {
        // DS-832: uncomputed/unmeasured reachability -> NO_EVIDENCE (NOT_CHECKED).
        return CheckOutcome::new(
            VerificationCheck::TargetBinding,
            VerificationState::NoEvidence,
            vec![DiagnosticLabel::NotChecked],
            vec!["target reachability was not measured (DS-832)".to_owned()],
        );
    }
    match coverage.result {
        EvidenceCheckValue::Pass => CheckOutcome::new(
            VerificationCheck::TargetBinding,
            VerificationState::Pass,
            Vec::new(),
            vec!["all declared targets reached §7.3 coverage (DS-831)".to_owned()],
        ),
        EvidenceCheckValue::Fail => CheckOutcome::new(
            VerificationCheck::TargetBinding,
            VerificationState::Fail,
            vec![DiagnosticLabel::NotExecuted],
            vec!["measured target coverage count is 0 (DS-832)".to_owned()],
        ),
        // DS-832「関数不見当はUNKNOWNとする」— this aggregate-level
        // `TargetExecution` (predecessor single-field shape; see DES-185's
        // disclosed `target_coverage` rename this crate has not carried out)
        // cannot name which declared target went unfound, only that the
        // aggregate measurement could not identify one.
        _ => CheckOutcome::new(
            VerificationCheck::TargetBinding,
            VerificationState::Unknown,
            Vec::new(),
            vec!["measured coverage could not identify the declared target \
                  function (DS-832)"
                .to_owned()],
        ),
    }
}

/// DS-605「`oracle_presence`はDA-001 / DA-003 / DA-004 / DA-005 / DA-006の
/// 合成とする」— delegated to `vtest_adapter_rust::oracle_presence`, the
/// `rust-cargo` adapter's own "Static Analysis capability" (DS-614). Core
/// (this function) does not interpret Rust syntax itself: it reads the raw
/// construct bytes by the byte range `vtest-scan` already located, hands
/// them to the adapter capability, and only composes the returned verdicts
/// into `VerificationState`/`DiagnosticLabel` (DS-606/607/608).
///
/// DS-614「Static Analysis capabilityがない場合は`NO_EVIDENCE`（診断
/// `NOT_CHECKED`）とする」: this slice's capability is unavailable exactly
/// when the construct's source bytes cannot be read back (the file moved,
/// or its byte range no longer resolves) — a real, if narrow, instance of
/// "capability absent for this Test", not a blanket placeholder.
fn evaluate_oracle_presence(test: &TestEntity, root: &Path) -> CheckOutcome {
    let Some(construct_text) = read_construct_text(root, &test.location) else {
        return CheckOutcome::new(
            VerificationCheck::OraclePresence,
            VerificationState::NoEvidence,
            vec![DiagnosticLabel::NotChecked],
            vec![
                "the Test construct's source bytes could not be read for static \
                  analysis (DS-614)"
                    .to_owned(),
            ],
        );
    };
    // DS-750-shaped disclosed narrowing (see `vtest_adapter_rust::oracle_presence`'s
    // module doc): only `TargetRef::Locator` targets contribute a symbol
    // name to DA-003. A `SrcId` target is silently excluded from DA-003's
    // per-target check rather than treated as an unverified call — DA-003
    // only evaluates calls it can name, so this narrows DA-003's coverage,
    // it does not fabricate a violation.
    let target_symbols: Vec<String> = test
        .targets
        .iter()
        .filter_map(|target| match target {
            TargetRef::Locator(locator) => {
                Some(vtest_adapter_rust::oracle_presence::target_symbol(locator))
            }
            TargetRef::SrcId(_) => None,
        })
        .collect();
    // DS-617's `assertion_macros` config projection is not read from
    // `config.yaml` in this slice (disclosed): only the standard macro set
    // is used. A project that only verifies through a configured custom
    // macro would see DA-006 report `Fail` where a config-aware analysis
    // would not.
    let analysis =
        vtest_adapter_rust::oracle_presence::analyze(&construct_text, &target_symbols, &[]);
    let (is_fail, is_unknown, basis) = vtest_adapter_rust::oracle_presence::compose(&analysis);
    // DS-609「`oracle_presence`に動的な昇格経路は無い」: composed purely
    // from the five static verdicts, nothing else can move this outcome.
    if is_fail {
        CheckOutcome::new(
            VerificationCheck::OraclePresence,
            VerificationState::Fail,
            Vec::new(),
            basis,
        )
    } else if is_unknown {
        CheckOutcome::new(
            VerificationCheck::OraclePresence,
            VerificationState::Unknown,
            Vec::new(),
            basis,
        )
    } else {
        CheckOutcome::new(
            VerificationCheck::OraclePresence,
            VerificationState::Pass,
            Vec::new(),
            basis,
        )
    }
}

/// Reads a Test construct's raw source bytes back from disk by the byte
/// range `vtest-scan` already located (`location.byte_range`), the same
/// bytes `source_target_subject_hash`/`test_subject_hash` bind — this
/// function never re-parses or re-locates anything, only re-reads.
fn read_construct_text(root: &Path, location: &vtest_model::SourceLocation) -> Option<String> {
    let path = root.join(location.path.as_str());
    let bytes = std::fs::read(path).ok()?;
    let start = usize::try_from(location.byte_range.start).ok()?;
    let end = usize::try_from(location.byte_range.end).ok()?;
    if end > bytes.len() || start > end {
        return None;
    }
    String::from_utf8(bytes[start..end].to_vec()).ok()
}

// ---------------------------------------------------------------------------
// Tree
// ---------------------------------------------------------------------------

/// Entity-axis selection: which DOC nodes, VOs and Tests the tree shows.
struct EntitySelection {
    docs: BTreeSet<String>,
    vos: BTreeSet<String>,
    tests: BTreeSet<String>,
}

impl EntitySelection {
    fn new(
        vos: &BTreeMap<String, VoRecord>,
        scan: &ScanResult,
        entity_scope: Option<&EntityScope>,
    ) -> Self {
        let all_docs = vos
            .values()
            .flat_map(|record| {
                record
                    .derives_from
                    .iter()
                    .map(|entry| entry.doc.as_str().to_owned())
            })
            .collect::<BTreeSet<_>>();

        let Some(scope) = entity_scope else {
            return Self {
                docs: all_docs,
                vos: vos.keys().cloned().collect(),
                tests: scan
                    .tests
                    .iter()
                    .map(|test| test.id.as_str().to_owned())
                    .collect(),
            };
        };

        let mut selected_vos = BTreeSet::new();
        let mut selected_docs = BTreeSet::new();
        let mut selected_tests = BTreeSet::new();
        match scope {
            EntityScope::Doc(id) => {
                selected_docs.insert(id.clone());
                selected_vos.extend(
                    vos.iter()
                        .filter(|(_, record)| {
                            record
                                .derives_from
                                .iter()
                                .any(|entry| entry.doc.as_str() == id)
                        })
                        .map(|(vo_id, _)| vo_id.clone()),
                );
                extend_with_descendants(vos, &mut selected_vos);
            }
            EntityScope::Vo(id) => {
                selected_vos.insert(id.clone());
                extend_with_descendants(vos, &mut selected_vos);
            }
            EntityScope::Test(id) => {
                selected_tests.insert(id.clone());
                if let Some(test) = scan.tests.iter().find(|test| test.id.as_str() == id) {
                    selected_vos.extend(test.covers.iter().map(|vo| vo.as_str().to_owned()));
                }
            }
        }
        if !matches!(scope, EntityScope::Test(_)) {
            selected_tests.extend(
                scan.tests
                    .iter()
                    .filter(|test| {
                        test.covers
                            .iter()
                            .any(|vo| selected_vos.contains(vo.as_str()))
                    })
                    .map(|test| test.id.as_str().to_owned()),
            );
        }
        for vo_id in &selected_vos {
            if let Some(record) = vos.get(vo_id) {
                selected_docs.extend(
                    record
                        .derives_from
                        .iter()
                        .map(|e| e.doc.as_str().to_owned()),
                );
            }
        }
        Self {
            docs: selected_docs,
            vos: selected_vos,
            tests: selected_tests,
        }
    }
}

fn extend_with_descendants(vos: &BTreeMap<String, VoRecord>, selected: &mut BTreeSet<String>) {
    loop {
        let additions = vos
            .iter()
            .filter(|(id, record)| {
                !selected.contains(*id)
                    && record
                        .parent
                        .as_ref()
                        .is_some_and(|parent| selected.contains(parent.as_str()))
            })
            .map(|(id, _)| id.clone())
            .collect::<Vec<_>>();
        if additions.is_empty() {
            break;
        }
        selected.extend(additions);
    }
}

/// Build the DOC → VO → Test tree.
///
/// DS-841「aggregateは、各leaf VOについてcoversするTEST群の結果を fail-closed で
/// 合成する」、DS-842（親VO = 子VO ∪ 直接 covers する TEST）、DS-843（DOC =
/// 下流VO部分木の合成）。DS-857「covers宣言を経由しない「機能名による束ね」
/// …を設けない」ため、束ねの経路は `derives_from` / `parent` / `covers` だけ。
fn build_tree(
    vos: &BTreeMap<String, VoRecord>,
    scan: &ScanResult,
    selection: &EntitySelection,
    selected_checks: &BTreeSet<VerificationCheck>,
    evidence: &EvidenceContext,
) -> Vec<TreeNode> {
    let resolution = target_resolution_diagnostics(scan);
    let mut roots = Vec::new();
    let mut placed_vos = BTreeSet::new();

    // DOC 層: VO の derives_from が指す上流ノード id（DS-1660）。
    for doc in &selection.docs {
        let children = vos
            .iter()
            .filter(|(id, record)| {
                selection.vos.contains(*id)
                    && record.parent.is_none()
                    && record
                        .derives_from
                        .iter()
                        .any(|entry| entry.doc.as_str() == doc)
            })
            .map(|(id, _)| {
                build_vo_node(
                    id,
                    vos,
                    scan,
                    selection,
                    selected_checks,
                    &resolution,
                    &mut placed_vos,
                    evidence,
                )
            })
            .collect::<Vec<_>>();
        roots.push(node_from_children(NodeKind::Doc, doc, Vec::new(), children));
    }

    // 上流ノードへ結び付かない VO も部分木から消さない。消すと、限定 report が
    // 「見えないから PASS」を生む。
    for id in &selection.vos {
        if placed_vos.contains(id) {
            continue;
        }
        if vos.get(id).is_some_and(|record| record.parent.is_some()) {
            continue;
        }
        roots.push(build_vo_node(
            id,
            vos,
            scan,
            selection,
            selected_checks,
            &resolution,
            &mut placed_vos,
            evidence,
        ));
    }

    // どの選択済み VO も covers しない Test も同様に残す。
    for test in &scan.tests {
        if !selection.tests.contains(test.id.as_str()) {
            continue;
        }
        if test
            .covers
            .iter()
            .any(|vo| selection.vos.contains(vo.as_str()))
        {
            continue;
        }
        roots.push(test_node(
            test,
            selected_checks,
            &resolution,
            scan,
            evidence,
        ));
    }

    roots
}

#[allow(clippy::too_many_arguments)]
fn build_vo_node(
    id: &str,
    vos: &BTreeMap<String, VoRecord>,
    scan: &ScanResult,
    selection: &EntitySelection,
    selected_checks: &BTreeSet<VerificationCheck>,
    resolution: &BTreeMap<String, TargetResolution>,
    placed: &mut BTreeSet<String>,
    evidence: &EvidenceContext,
) -> TreeNode {
    if !placed.insert(id.to_owned()) {
        // 循環・重複配置。値を発明せず、未検査として保持する。
        return TreeNode {
            kind: NodeKind::Vo,
            id: id.to_owned(),
            state: VerificationState::NoEvidence,
            checks: vec![CheckOutcome::new(
                VerificationCheck::ChainIntegrity,
                VerificationState::NoEvidence,
                vec![DiagnosticLabel::NotChecked],
                vec!["VO already placed in the tree (cycle or shared parent)".to_owned()],
            )],
            children: Vec::new(),
        };
    }
    let mut children = vos
        .iter()
        .filter(|(child_id, record)| {
            selection.vos.contains(*child_id)
                && record
                    .parent
                    .as_ref()
                    .is_some_and(|parent| parent.as_str() == id)
        })
        .map(|(child_id, _)| {
            build_vo_node(
                child_id,
                vos,
                scan,
                selection,
                selected_checks,
                resolution,
                placed,
                evidence,
            )
        })
        .collect::<Vec<_>>();
    children.extend(
        scan.tests
            .iter()
            .filter(|test| {
                selection.tests.contains(test.id.as_str())
                    && test.covers.iter().any(|vo| vo.as_str() == id)
            })
            .map(|test| test_node(test, selected_checks, resolution, scan, evidence)),
    );
    node_from_children(NodeKind::Vo, id, Vec::new(), children)
}

fn test_node(
    test: &TestEntity,
    selected_checks: &BTreeSet<VerificationCheck>,
    resolution: &BTreeMap<String, TargetResolution>,
    scan: &ScanResult,
    evidence: &EvidenceContext,
) -> TreeNode {
    let empty = TargetResolution::default();
    let test_resolution = resolution.get(&test.location.locator).unwrap_or(&empty);
    let checks = PER_TEST_CHECKS
        .into_iter()
        .map(|check| {
            if !selected_checks.contains(&check) {
                return CheckOutcome::out_of_scope(check);
            }
            match check {
                VerificationCheck::TargetBinding => {
                    evaluate_target_binding(test, test_resolution, scan, evidence)
                }
                VerificationCheck::OraclePresence => evaluate_oracle_presence(test, &evidence.root),
                _ => unreachable!("PER_TEST_CHECKS holds only the two per-Test checks"),
            }
        })
        .collect::<Vec<_>>();
    node_from_children(NodeKind::Test, test.id.as_str(), checks, Vec::new())
}

/// 集約点を1つ作る。子が空でも `PASS` へ畳まない: 検査も子も持たない集約点は
/// 「まだ何も確かめていない」ので `NO_EVIDENCE` / `NOT_CHECKED` とする。
/// REQ-100 / DS-1110 の「未実施を `PASS` へ変換しない」を空集合へ拡張する
/// 唯一の安全な読み。
fn node_from_children(
    kind: NodeKind,
    id: &str,
    mut checks: Vec<CheckOutcome>,
    children: Vec<TreeNode>,
) -> TreeNode {
    if checks.is_empty() && children.is_empty() {
        checks.push(CheckOutcome::new(
            VerificationCheck::ChainIntegrity,
            VerificationState::NoEvidence,
            vec![DiagnosticLabel::NotChecked],
            vec!["aggregation point has no child and no evaluated check".to_owned()],
        ));
    }
    let state = representative(
        checks
            .iter()
            .map(|outcome| outcome.state)
            .chain(children.iter().map(|child| child.state)),
    );
    TreeNode {
        kind,
        id: id.to_owned(),
        state,
        checks,
        children,
    }
}

// ---------------------------------------------------------------------------
// Record reading
// ---------------------------------------------------------------------------

fn read_vo_records(layout: &VerifyLayout) -> BTreeMap<String, VoRecord> {
    // 正典の VO reader を使う（前身の `read_vo` は旧 `ReqRecord` 系の
    // レコード型を返すため使わない）。読めなかったレコードはここでは
    // 黙って落とすのではなく、scan が E-SCAN-010 として報告している
    // ものが `chain_integrity` で `MISMATCH` になる経路に委ねる。
    read_record_ids(&layout.vo_dir())
        .unwrap_or_default()
        .into_iter()
        .filter_map(|id| {
            read_vo_record(layout, &id)
                .ok()
                .map(|(record, _)| (id, record))
        })
        .collect()
}

/// Every upstream document node id declared under `.verify/doc/`, across all
/// seven layers. DS-1660（`doc` field の値は上流ノード id であり、上流文書の
/// ファイル名ではない）に従い、VO の `derives_from` はこの集合に対して解決する。
fn read_document_node_ids(layout: &VerifyLayout) -> BTreeSet<String> {
    let mut ids = BTreeSet::new();
    for name in read_document_names(&layout.doc_dir()).unwrap_or_default() {
        let Ok(file) = read_document_file(layout, &name) else {
            continue;
        };
        collect_document_node_ids(&file, &mut ids);
    }
    ids
}

fn collect_document_node_ids(file: &DocumentFile, ids: &mut BTreeSet<String>) {
    for node in &file.root {
        ids.insert(node.id.as_str().to_owned());
    }
    for node in &file.request {
        collect_sentence_ids(node, ids);
    }
    for sections in [
        &file.require,
        &file.spec,
        &file.detailed_spec,
        &file.basic_design,
        &file.design,
    ] {
        for section in sections {
            collect_section_ids(section, ids);
        }
    }
}

fn collect_section_ids(section: &SectionNode, ids: &mut BTreeSet<String>) {
    ids.insert(section.id.as_str().to_owned());
    if let Some(items) = &section.items {
        for item in items {
            collect_sentence_ids(item, ids);
        }
    }
    if let Some(children) = &section.sections {
        for child in children {
            collect_section_ids(child, ids);
        }
    }
}

fn collect_sentence_ids(sentence: &SentenceNode, ids: &mut BTreeSet<String>) {
    ids.insert(sentence.id.as_str().to_owned());
}

fn join_ids(ids: &BTreeSet<String>) -> String {
    ids.iter().cloned().collect::<Vec<_>>().join(", ")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};
    use vtest_model::{
        AdapterId, ContentHash, DerivesFrom, Diagnostic, DiscoveredTest, DocumentId,
        ExecutionDescriptor, NodeSource, ProjectPath, RootNode, SentenceNode, SourceLocation,
        SourceRange, TargetRef, TestId, VoId,
    };
    use vtest_store::{init_project, write_document_file, write_vo_record, VerifyLayout};

    // -----------------------------------------------------------------
    // Fixture construction
    // -----------------------------------------------------------------

    fn temp_root(name: &str) -> std::path::PathBuf {
        let suffix = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock")
            .as_nanos();
        let root = std::env::temp_dir().join(format!("vtest-verify-{name}-{suffix}"));
        std::fs::create_dir_all(&root).expect("create fixture root");
        root
    }

    fn source(id: &str) -> NodeSource {
        NodeSource {
            doc: format!("{id}.md"),
            heading: "fixture".to_owned(),
            lines: [1, 1],
        }
    }

    /// A minimal, fully-connected document file: one `root` node and one
    /// `request` sentence deriving from it. Every non-root node therefore has
    /// an effective upstream, so a correct `orphan_detection` is `PASS`.
    fn document_file() -> DocumentFile {
        DocumentFile {
            schema_version: "0.1".to_owned(),
            root: vec![RootNode {
                id: DocumentId::new("ROOT-001"),
                statement: "fixture root".to_owned(),
                description: None,
                source: source("root"),
            }],
            request: vec![SentenceNode {
                id: DocumentId::new("R-001"),
                statement: "fixture requirement".to_owned(),
                description: None,
                derives_from: vec![DocumentId::new("ROOT-001")],
                cites: None,
                source: source("req"),
            }],
            require: Vec::new(),
            spec: Vec::new(),
            detailed_spec: Vec::new(),
            basic_design: Vec::new(),
            design: Vec::new(),
        }
    }

    fn vo(id: &str, parent: Option<&str>) -> VoRecord {
        VoRecord {
            id: VoId::new(id),
            parent: parent.map(VoId::new),
            derives_from: vec![DerivesFrom {
                doc: DocumentId::new("R-001"),
                anchor: None,
                note: None,
            }],
            claim: "fixture claim".to_owned(),
            dimensions: Vec::new(),
            coverage_policy: None,
            combinations: Vec::new(),
            representative_cases: Vec::new(),
            created: "2026-09-08T00:00:00Z".to_owned(),
            updated: "2026-09-08T00:00:00Z".to_owned(),
        }
    }

    fn location(function: &str) -> SourceLocation {
        SourceLocation {
            adapter: AdapterId::new("rust-cargo"),
            path: ProjectPath("tests/fixture.rs".to_owned()),
            locator: format!("tests/fixture.rs::{function}"),
            byte_range: SourceRange { start: 0, end: 1 },
        }
    }

    /// A Test with one declared target, covering `covers`.
    fn test_entity(id: &str, covers: &[&str], targets: usize) -> TestEntity {
        TestEntity {
            id: TestId::new(id),
            covers: covers.iter().map(|vo| VoId::new(*vo)).collect(),
            targets: (0..targets)
                .map(|index| {
                    TargetRef::Locator(vtest_model::Locator {
                        adapter: AdapterId::new("rust-cargo"),
                        value: format!("src/lib.rs::target{index}"),
                    })
                })
                .collect(),
            intent: "fixture".to_owned(),
            input: None,
            expect: None,
            kind: None,
            cases: Vec::new(),
            related: Vec::new(),
            location: location(id),
            content_hash: ContentHash::from_text(id),
            execution: ExecutionDescriptor {
                adapter: AdapterId::new("rust-cargo"),
                project: None,
                suite: None,
                selector: id.to_owned(),
            },
        }
    }

    fn scan_result(tests: Vec<TestEntity>, diagnostics: Vec<Diagnostic>) -> ScanResult {
        ScanResult {
            summary: vtest_model::ScanSummary {
                files: 1,
                tests: tests.len() as u64,
                sources: 1,
            },
            discovered: tests
                .iter()
                .map(|test| DiscoveredTest {
                    adapter: AdapterId::new("rust-cargo"),
                    location: test.location.clone(),
                    content_hash: test.content_hash.clone(),
                    managed: ManagedTestLink::One(test.id.clone()),
                })
                .collect(),
            tests,
            sources: Vec::new(),
            diagnostics,
        }
    }

    /// A project whose declaration chain is complete: DOC → VO → Test.
    fn complete_project(name: &str) -> (std::path::PathBuf, ScanResult) {
        let root = temp_root(name);
        let layout: VerifyLayout = init_project(&root, "fixture").expect("init fixture project");
        write_document_file(&layout, "fixture", &document_file()).expect("write document file");
        write_vo_record(&layout, &vo("VO-ONE", None)).expect("write VO");
        let scan = scan_result(vec![test_entity("TEST-ONE", &["VO-ONE"], 1)], Vec::new());
        (root, scan)
    }

    fn outcome_for(root: &std::path::Path, scan: &ScanResult) -> VerifyOutcome {
        verify_project(root, scan, None, None)
    }

    fn state_of(outcome: &VerifyOutcome, check: VerificationCheck) -> VerificationState {
        representative(
            outcome
                .all_outcomes()
                .iter()
                .filter(|candidate| candidate.check == check)
                .map(|candidate| candidate.state),
        )
    }

    fn labels_of(outcome: &VerifyOutcome, check: VerificationCheck) -> Vec<DiagnosticLabel> {
        outcome
            .all_outcomes()
            .iter()
            .filter(|candidate| candidate.check == check)
            .flat_map(|candidate| candidate.labels.clone())
            .collect()
    }

    // -----------------------------------------------------------------
    // The two checks that can reach PASS in this slice
    // -----------------------------------------------------------------

    /// SPEC-053 / REQ-085: exactly four checks, and each lands in one of the
    /// five states. Nothing else appears in a result.
    #[test]
    fn a_result_holds_exactly_the_four_canonical_checks() {
        let (root, scan) = complete_project("four-checks");
        let outcome = outcome_for(&root, &scan);
        let checks = outcome
            .all_outcomes()
            .iter()
            .map(|check| check.check)
            .collect::<BTreeSet<_>>();
        assert_eq!(checks, ALL_CHECKS.into_iter().collect::<BTreeSet<_>>());
    }

    /// The complete-chain fixture: both structural checks are `PASS`.
    /// This is the closest this slice gets to an "all PASS" fixture —
    /// `target_binding` and `oracle_presence` cannot reach `PASS` here
    /// (no Evidence reader, no DA static analysis), which the two tests
    /// below assert explicitly rather than leave implied.
    #[test]
    fn a_complete_declaration_chain_passes_both_structural_checks() {
        let (root, scan) = complete_project("complete");
        let outcome = outcome_for(&root, &scan);
        assert_eq!(
            state_of(&outcome, VerificationCheck::ChainIntegrity),
            VerificationState::Pass
        );
        assert_eq!(
            state_of(&outcome, VerificationCheck::OrphanDetection),
            VerificationState::Pass
        );
    }

    // -----------------------------------------------------------------
    // chain_integrity in isolation — orphan_detection stays PASS
    // -----------------------------------------------------------------

    /// DS-561: E-SCAN-002 (Test ID collision) → `chain_integrity = MISMATCH`.
    /// `orphan_detection` is untouched: the two checks answer different
    /// questions (SPEC-054) and must not damage each other.
    #[test]
    fn only_chain_integrity_breaks_on_a_test_id_collision() {
        let (root, mut scan) = complete_project("collision");
        scan.diagnostics.push(Diagnostic::error(
            "E-SCAN-002",
            "duplicate Test ID TEST-ONE",
        ));
        let outcome = outcome_for(&root, &scan);
        assert_eq!(
            state_of(&outcome, VerificationCheck::ChainIntegrity),
            VerificationState::Mismatch
        );
        assert_eq!(
            state_of(&outcome, VerificationCheck::OrphanDetection),
            VerificationState::Pass
        );
    }

    /// DS-812: a Test declaring no `covers` is a management-declaration
    /// inconsistency, `chain_integrity = MISMATCH`.
    #[test]
    fn only_chain_integrity_breaks_on_a_test_without_covers() {
        let root = temp_root("no-covers");
        let layout = init_project(&root, "fixture").expect("init");
        write_document_file(&layout, "fixture", &document_file()).expect("doc");
        write_vo_record(&layout, &vo("VO-ONE", None)).expect("vo");
        // VO-ONE keeps a covering Test so the leaf-VO rule below does not
        // also fire; the covers-less Test is the only defect.
        let scan = scan_result(
            vec![
                test_entity("TEST-ONE", &["VO-ONE"], 1),
                test_entity("TEST-TWO", &[], 1),
            ],
            Vec::new(),
        );
        let outcome = outcome_for(&root, &scan);
        assert_eq!(
            state_of(&outcome, VerificationCheck::ChainIntegrity),
            VerificationState::Mismatch
        );
        assert_eq!(
            state_of(&outcome, VerificationCheck::OrphanDetection),
            VerificationState::Pass
        );
    }

    /// REQ-056 / ROOT-034: the retired `test_existence` was folded into
    /// `chain_integrity`, so a leaf VO with no covering Test must be caught
    /// here or nowhere.
    #[test]
    fn only_chain_integrity_breaks_on_a_leaf_vo_with_no_covering_test() {
        let root = temp_root("uncovered-leaf");
        let layout = init_project(&root, "fixture").expect("init");
        write_document_file(&layout, "fixture", &document_file()).expect("doc");
        write_vo_record(&layout, &vo("VO-ONE", None)).expect("vo");
        write_vo_record(&layout, &vo("VO-TWO", None)).expect("vo two");
        let scan = scan_result(vec![test_entity("TEST-ONE", &["VO-ONE"], 1)], Vec::new());
        let outcome = outcome_for(&root, &scan);
        assert_eq!(
            state_of(&outcome, VerificationCheck::ChainIntegrity),
            VerificationState::Mismatch
        );
        assert_eq!(
            state_of(&outcome, VerificationCheck::OrphanDetection),
            VerificationState::Pass
        );
    }

    /// DS-560 / DS-1510: a discovered Test construct with no management
    /// declaration is `MISMATCH` + diagnostic `MISSING`. It is read from
    /// `discovered`, not from error diagnostics, because an unregistered
    /// `#[test]` may only be reported as a warning.
    #[test]
    fn an_unmanaged_discovered_test_is_mismatch_with_the_missing_label() {
        let (root, mut scan) = complete_project("unmanaged");
        scan.discovered.push(DiscoveredTest {
            adapter: AdapterId::new("rust-cargo"),
            location: location("unregistered"),
            content_hash: ContentHash::from_text("unregistered"),
            managed: ManagedTestLink::Missing,
        });
        let outcome = outcome_for(&root, &scan);
        assert_eq!(
            state_of(&outcome, VerificationCheck::ChainIntegrity),
            VerificationState::Mismatch
        );
        assert!(labels_of(&outcome, VerificationCheck::ChainIntegrity)
            .contains(&DiagnosticLabel::Missing));
        assert_eq!(
            state_of(&outcome, VerificationCheck::OrphanDetection),
            VerificationState::Pass
        );
    }

    // -----------------------------------------------------------------
    // orphan_detection in isolation — chain_integrity stays PASS
    // -----------------------------------------------------------------

    /// DS-1647 / DS-1650 / DS-1651: E-SCAN-016 → `orphan_detection =
    /// MISMATCH`, and it must not spill into `chain_integrity`.
    #[test]
    fn only_orphan_detection_breaks_on_an_orphaned_document_node() {
        let (root, mut scan) = complete_project("orphan");
        scan.diagnostics.push(Diagnostic::error(
            "E-SCAN-016",
            "document node SPEC-999 is orphaned",
        ));
        let outcome = outcome_for(&root, &scan);
        assert_eq!(
            state_of(&outcome, VerificationCheck::OrphanDetection),
            VerificationState::Mismatch
        );
        assert_eq!(
            state_of(&outcome, VerificationCheck::ChainIntegrity),
            VerificationState::Pass
        );
    }

    // -----------------------------------------------------------------
    // target_binding / oracle_presence
    // -----------------------------------------------------------------

    /// DS-1664: a Test with no target is `NO_EVIDENCE` + `NOT_CHECKED`.
    /// DS-278: a Test with targets but no valid Evidence is `NO_EVIDENCE` +
    /// `NOT_EXECUTED`. The state is the same; the diagnostic label is what
    /// distinguishes the two causes — which is exactly why REQ-092 keeps the
    /// label in a separate field.
    #[test]
    fn target_binding_distinguishes_its_two_causes_by_diagnostic_label() {
        let (root, scan) = complete_project("tb-executed");
        let outcome = outcome_for(&root, &scan);
        assert_eq!(
            state_of(&outcome, VerificationCheck::TargetBinding),
            VerificationState::NoEvidence
        );
        assert_eq!(
            labels_of(&outcome, VerificationCheck::TargetBinding),
            vec![DiagnosticLabel::NotExecuted]
        );

        let root = temp_root("tb-not-checked");
        let layout = init_project(&root, "fixture").expect("init");
        write_document_file(&layout, "fixture", &document_file()).expect("doc");
        write_vo_record(&layout, &vo("VO-ONE", None)).expect("vo");
        let scan = scan_result(vec![test_entity("TEST-ONE", &["VO-ONE"], 0)], Vec::new());
        let outcome = outcome_for(&root, &scan);
        assert_eq!(
            labels_of(&outcome, VerificationCheck::TargetBinding),
            vec![DiagnosticLabel::NotChecked]
        );
    }

    /// DS-1678: an unresolvable declared target is `MISMATCH`, never folded to
    /// `NO_EVIDENCE` / `NOT_EXECUTED` on the grounds that no Evidence could be
    /// produced. E-SCAN-004 (target absent) additionally carries `MISSING`;
    /// E-SCAN-011 (ambiguous) carries no label, because DS-1678 leaves the
    /// canon silent there and declines to forbid one.
    ///
    /// This supersedes the retired DS-756, which had held the same event at
    /// `NO_EVIDENCE` / `NOT_EXECUTED`.
    #[test]
    fn an_unresolvable_target_is_mismatch_not_no_evidence() {
        let (root, mut scan) = complete_project("unresolvable-target");
        let at = scan.tests[0].location.clone();
        scan.diagnostics.push(
            Diagnostic::error("E-SCAN-004", "target `src/lib.rs::gone` cannot be resolved")
                .with_location(at),
        );
        let outcome = outcome_for(&root, &scan);
        assert_eq!(
            state_of(&outcome, VerificationCheck::TargetBinding),
            VerificationState::Mismatch,
            "DS-1678 forbids falling back to NO_EVIDENCE here"
        );
        assert!(labels_of(&outcome, VerificationCheck::TargetBinding)
            .contains(&DiagnosticLabel::Missing));
        assert!(!labels_of(&outcome, VerificationCheck::TargetBinding)
            .contains(&DiagnosticLabel::NotExecuted));

        // E-SCAN-011: MISMATCH, but no invented diagnostic label.
        let (root, mut scan) = complete_project("ambiguous-target");
        let at = scan.tests[0].location.clone();
        scan.diagnostics.push(
            Diagnostic::error("E-SCAN-011", "target matches multiple candidates").with_location(at),
        );
        let outcome = outcome_for(&root, &scan);
        assert_eq!(
            state_of(&outcome, VerificationCheck::TargetBinding),
            VerificationState::Mismatch
        );
        assert!(labels_of(&outcome, VerificationCheck::TargetBinding).is_empty());
    }

    /// DS-561: `ManagedTestLink::Multiple` maps to `chain_integrity = MISMATCH`,
    /// as a violation distinct from the E-SCAN-002 Test-ID collision it is
    /// listed alongside. The other three checks must not be collaterally
    /// damaged.
    ///
    /// Disclosure: `vtest-scan` does not currently produce `Multiple` from any
    /// live scan — `rust-cargo` emits at most one draft per function item — so
    /// this mapping is exercised only by this constructed fixture.
    #[test]
    fn a_multiple_management_declaration_is_chain_integrity_mismatch() {
        let (root, mut scan) = complete_project("multiple-link");
        scan.discovered.push(DiscoveredTest {
            adapter: AdapterId::new("rust-cargo"),
            location: location("ambiguous-construct"),
            content_hash: ContentHash::from_text("ambiguous-construct"),
            managed: ManagedTestLink::Multiple(vec![TestId::new("TEST-A"), TestId::new("TEST-B")]),
        });
        let outcome = outcome_for(&root, &scan);
        assert_eq!(
            state_of(&outcome, VerificationCheck::ChainIntegrity),
            VerificationState::Mismatch
        );
        // No collateral damage to the other three.
        assert_eq!(
            state_of(&outcome, VerificationCheck::OrphanDetection),
            VerificationState::Pass
        );
        assert_eq!(
            state_of(&outcome, VerificationCheck::TargetBinding),
            VerificationState::NoEvidence
        );
        assert_eq!(
            state_of(&outcome, VerificationCheck::OraclePresence),
            VerificationState::NoEvidence
        );
    }

    /// DS-1107「`config.yaml` の `verify.full_scope` は…項目選択 knob として
    /// 使用しない」 and DS-1109「`verify.full_scope` の in-memory の項目補完は
    /// 行わない」.
    ///
    /// Written as a behavioural assertion rather than a statement about the
    /// call signature: a `config.yaml` carrying a one-item `full_scope` sits on
    /// disk, and the run still evaluates all four checks. If the config were
    /// ever consulted for item selection, three checks would come back
    /// NOT_CHECKED instead.
    #[test]
    fn a_config_full_scope_subset_never_narrows_the_checks_that_run() {
        let (root, scan) = complete_project("config-not-a-knob");
        std::fs::write(
            root.join(".verify").join("config.yaml"),
            "version: 2\nproject:\n  name: fixture\nadapters: []\nverify:\n  \
             full_scope:\n    - chain_integrity\n",
        )
        .expect("write a subset full_scope config");

        let outcome = verify_project(&root, &scan, None, None);
        assert_eq!(
            outcome.scope.requested_checks,
            ALL_CHECKS.to_vec(),
            "omitting --items must mean the fixed four, not the config's list"
        );
        assert!(
            !outcome.scope.limited,
            "a config value must not make the run a limited scope"
        );
        for check in ALL_CHECKS {
            assert!(
                !labels_of(&outcome, check).contains(&DiagnosticLabel::NotChecked)
                    || check == VerificationCheck::OraclePresence
                    || check == VerificationCheck::TargetBinding,
                "{} was skipped as if the config had selected items",
                check_name(check)
            );
        }
    }

    /// REQ-079 (static analysis never proves the positive) and REQ-108
    /// (`UNKNOWN` is not an error fallback): with no DA analysis available,
    /// `oracle_presence` is held as `NO_EVIDENCE` + `NOT_CHECKED` — never
    /// `PASS`, never `UNKNOWN`.
    #[test]
    fn oracle_presence_is_never_pass_and_never_unknown_without_da_analysis() {
        let (root, scan) = complete_project("oracle");
        let outcome = outcome_for(&root, &scan);
        let state = state_of(&outcome, VerificationCheck::OraclePresence);
        assert_ne!(state, VerificationState::Pass);
        assert_ne!(state, VerificationState::Unknown);
        assert_eq!(state, VerificationState::NoEvidence);
        assert_eq!(
            labels_of(&outcome, VerificationCheck::OraclePresence),
            vec![DiagnosticLabel::NotChecked]
        );
    }

    // -----------------------------------------------------------------
    // Scope
    // -----------------------------------------------------------------

    /// DS-840 / DS-1110: a check outside the requested item scope is retained
    /// as `NO_EVIDENCE` + `NOT_CHECKED`; it is never converted to `PASS` and
    /// never silently dropped from the result.
    #[test]
    fn a_check_outside_the_item_scope_is_no_evidence_not_checked() {
        let (root, scan) = complete_project("item-scope");
        let outcome = verify_project(
            &root,
            &scan,
            Some(&[VerificationCheck::ChainIntegrity]),
            None,
        );
        assert!(outcome.scope.limited);
        assert!(outcome.scope.outside_scope_is_unverified);
        assert_eq!(
            state_of(&outcome, VerificationCheck::ChainIntegrity),
            VerificationState::Pass
        );
        for check in [
            VerificationCheck::OrphanDetection,
            VerificationCheck::TargetBinding,
            VerificationCheck::OraclePresence,
        ] {
            assert_eq!(
                state_of(&outcome, check),
                VerificationState::NoEvidence,
                "{} must not be PASS when out of scope",
                check_name(check)
            );
            assert!(labels_of(&outcome, check).contains(&DiagnosticLabel::NotChecked));
        }
        // DS-1111: a limited scope is never reported as complete-verification OK.
        assert!(!outcome.ok);
    }

    /// REQ-295: the structural checks are checks over the whole declaration
    /// chain, so limiting the entity axis must not shrink them into a PASS.
    #[test]
    fn an_entity_scope_does_not_shrink_the_structural_checks() {
        let root = temp_root("entity-scope");
        let layout = init_project(&root, "fixture").expect("init");
        write_document_file(&layout, "fixture", &document_file()).expect("doc");
        write_vo_record(&layout, &vo("VO-ONE", None)).expect("vo");
        write_vo_record(&layout, &vo("VO-TWO", None)).expect("vo two");
        // VO-TWO has no covering Test, so chain_integrity is MISMATCH.
        let scan = scan_result(vec![test_entity("TEST-ONE", &["VO-ONE"], 1)], Vec::new());

        // Scoping to VO-ONE (which is itself fine) must not hide VO-TWO's
        // break: the structural check stays whole-chain.
        let outcome = verify_project(
            &root,
            &scan,
            None,
            Some(EntityScope::Vo("VO-ONE".to_owned())),
        );
        assert_eq!(
            state_of(&outcome, VerificationCheck::ChainIntegrity),
            VerificationState::Mismatch
        );
        assert!(outcome.scope.limited);
    }

    // -----------------------------------------------------------------
    // Aggregation
    // -----------------------------------------------------------------

    /// DS-871: `FAIL > MISMATCH > NO_EVIDENCE > UNKNOWN`, and all-`PASS`
    /// yields `PASS`.
    #[test]
    fn representative_selection_follows_the_canonical_priority() {
        use VerificationState::{Fail, Mismatch, NoEvidence, Pass, Unknown};
        assert_eq!(representative([Pass, Pass]), Pass);
        assert_eq!(representative([Pass, Unknown]), Unknown);
        assert_eq!(representative([Unknown, NoEvidence]), NoEvidence);
        assert_eq!(representative([NoEvidence, Mismatch]), Mismatch);
        assert_eq!(representative([Mismatch, Fail]), Fail);
        assert_eq!(representative([Fail, Mismatch, NoEvidence, Unknown]), Fail);
        // DS-1511: one non-PASS child makes the parent non-PASS.
        assert_ne!(representative([Pass, Pass, Unknown]), Pass);
    }

    /// An aggregation point with no child and no evaluated check must not
    /// fold to `PASS` — that is the precise shape of a false PASS.
    #[test]
    fn an_empty_aggregation_point_is_not_pass() {
        let node = node_from_children(NodeKind::Vo, "VO-EMPTY", Vec::new(), Vec::new());
        assert_eq!(node.state, VerificationState::NoEvidence);
        assert_eq!(node.checks[0].labels, vec![DiagnosticLabel::NotChecked]);
    }

    /// A repository with no VO and no Test must never verify as OK.
    ///
    /// Both structural checks are vacuously satisfiable over an empty chain,
    /// so without this rule an empty (or freshly initialised) project would
    /// report complete-verification OK — the exact false PASS this tool
    /// exists to stop. DS-252「`vtest verify` は正典または検証事実の欠落を
    /// 対応する非 `PASS` 値として表示する」、DS-253。
    #[test]
    fn an_empty_repository_is_not_a_complete_verification_ok() {
        let root = temp_root("empty-repo");
        init_project(&root, "fixture").expect("init");
        let scan = scan_result(Vec::new(), Vec::new());
        let outcome = outcome_for(&root, &scan);
        assert!(!outcome.ok, "an empty repository must not be OK");
        assert_eq!(outcome.state, VerificationState::NoEvidence);
        // 4検査はすべて結果の中に現れ続ける（SPEC-053）。
        assert_eq!(
            outcome
                .all_outcomes()
                .iter()
                .map(|check| check.check)
                .collect::<BTreeSet<_>>(),
            ALL_CHECKS.into_iter().collect::<BTreeSet<_>>()
        );
        for check in PER_TEST_CHECKS {
            assert_eq!(state_of(&outcome, check), VerificationState::NoEvidence);
            assert!(labels_of(&outcome, check).contains(&DiagnosticLabel::NotChecked));
        }
    }

    /// DS-789: identical evaluation inputs must produce an identical result.
    #[test]
    fn verification_is_deterministic() {
        let (root, scan) = complete_project("determinism");
        let first = serde_json::to_string(&outcome_for(&root, &scan)).expect("serialise");
        let second = serde_json::to_string(&outcome_for(&root, &scan)).expect("serialise");
        assert_eq!(first, second);
    }

    /// REQ-092 / SPEC-373: state and diagnostic label are separate fields.
    /// A label must never be serialised in the state position.
    #[test]
    fn state_and_diagnostic_label_are_separate_fields() {
        let outcome = CheckOutcome::new(
            VerificationCheck::TargetBinding,
            VerificationState::NoEvidence,
            vec![DiagnosticLabel::NotExecuted],
            Vec::new(),
        );
        let json = serde_json::to_value(&outcome).expect("serialise");
        assert_eq!(json["state"], "NO_EVIDENCE");
        assert_eq!(json["labels"][0], "NOT_EXECUTED");
    }

    // -------------------------------------------------------------------
    // target_binding: Evidence-backed judgment (DS-816-833)
    // -------------------------------------------------------------------

    fn sample_evidence(test_id: &str, adapter: &str, commit: Option<&str>) -> EvidenceRecord {
        EvidenceRecord {
            id: "01ARZ3NDEKTSV4RRFFQ69G5FAV".to_owned(),
            test_id: TestId::new(test_id),
            adapter: AdapterId::new(adapter),
            result: vtest_model::TestResult::Pass,
            executed_at: "2026-09-08T00:00:00Z".to_owned(),
            revision: vtest_model::Revision {
                commit: commit.map(str::to_owned),
                dirty: false,
            },
            execution_state: vtest_model::ExecutionState {
                schema: "rust-cargo-execution-state-v1".to_owned(),
                complete: false,
                hash: None,
            },
            hashes: vtest_model::EvidenceHashes {
                test_fn: ContentHash::from_text("TEST-ONE"),
                target_fn: ContentHash::from_text("TEST-ONE::target0"),
                target_fns: vec![ContentHash::from_text("TEST-ONE::target0")],
            },
            runner: vtest_model::RunnerInfo {
                kind: "cargo-test".to_owned(),
                command: "cargo test".to_owned(),
                exit_code: 0,
            },
            target_execution: vtest_model::TargetExecution {
                checked: false,
                method: None,
                result: EvidenceCheckValue::NotChecked,
                count: None,
            },
            log_ref: "cache/logs/01ARZ3NDEKTSV4RRFFQ69G5FAV.log".to_owned(),
        }
    }

    fn write_sample_evidence(layout: &VerifyLayout, record: &EvidenceRecord) {
        std::fs::create_dir_all(layout.evidence_dir()).expect("evidence dir");
        let path = layout.evidence_dir().join(format!("{}.yaml", record.id));
        let yaml = format!(
            "id: '{id}'\ntest_id: '{test_id}'\nadapter: '{adapter}'\nresult: PASS\n\
             executed_at: '{executed_at}'\nrevision:\n  commit: {commit}\n  dirty: false\n\
             execution_state:\n  schema: '{schema}'\n  complete: {complete}\n  hash: null\n\
             hashes:\n  test_fn: '{test_fn}'\n  target_fn: '{target_fn}'\n  target_fns:\n    - '{target_fn}'\n\
             runner:\n  kind: 'cargo-test'\n  command: 'cargo test'\n  exit_code: 0\n\
             target_execution:\n  checked: false\n  method: null\n  result: NOT_CHECKED\n  count: null\n\
             log_ref: '{log_ref}'\n",
            id = record.id,
            test_id = record.test_id.as_str(),
            adapter = record.adapter.as_str(),
            executed_at = record.executed_at,
            commit = record
                .revision
                .commit
                .as_deref()
                .map(|value| format!("'{value}'"))
                .unwrap_or_else(|| "null".to_owned()),
            schema = record.execution_state.schema,
            complete = record.execution_state.complete,
            test_fn = record.hashes.test_fn.as_str(),
            target_fn = record.hashes.target_fn.as_str(),
            log_ref = record.log_ref,
        );
        std::fs::write(path, yaml).expect("write evidence fixture");
    }

    /// DS-825: no Evidence at all is `NO_EVIDENCE` / `NOT_EXECUTED` — already
    /// covered above (`target_binding_distinguishes_its_two_causes_by_diagnostic_label`'s
    /// `tb-executed` case, which has no Evidence on disk). This test instead
    /// covers DS-1628/DS-819/DS-820: an Evidence record that *exists* but
    /// fails validity is `NO_EVIDENCE` / `STALE`, never reused as `PASS`.
    #[test]
    fn stale_evidence_with_a_mismatched_adapter_is_no_evidence_stale() {
        let root = temp_root("tb-stale-adapter");
        let layout = init_project(&root, "fixture").expect("init");
        write_document_file(&layout, "fixture", &document_file()).expect("doc");
        write_vo_record(&layout, &vo("VO-ONE", None)).expect("vo");
        let scan = scan_result(vec![test_entity("TEST-ONE", &["VO-ONE"], 1)], Vec::new());
        write_sample_evidence(
            &layout,
            &sample_evidence("TEST-ONE", "other-adapter", Some("deadbeef")),
        );

        let outcome = outcome_for(&root, &scan);
        assert_eq!(
            state_of(&outcome, VerificationCheck::TargetBinding),
            VerificationState::NoEvidence
        );
        assert_eq!(
            labels_of(&outcome, VerificationCheck::TargetBinding),
            vec![DiagnosticLabel::Stale]
        );
    }

    /// DS-819/DS-827: a matching adapter is not enough — a target set /
    /// subject hash that no longer matches the current canonical set is
    /// also `NO_EVIDENCE` / `STALE`, never reused as `PASS`. `scan_result`'s
    /// test fixtures always leave `sources` empty, so the target set can
    /// never resolve to a match; this exercises that path directly.
    #[test]
    fn stale_evidence_with_an_unresolvable_target_set_is_no_evidence_stale() {
        let root = temp_root("tb-stale-targets");
        let layout = init_project(&root, "fixture").expect("init");
        write_document_file(&layout, "fixture", &document_file()).expect("doc");
        write_vo_record(&layout, &vo("VO-ONE", None)).expect("vo");
        let scan = scan_result(vec![test_entity("TEST-ONE", &["VO-ONE"], 1)], Vec::new());
        write_sample_evidence(
            &layout,
            &sample_evidence("TEST-ONE", "rust-cargo", Some("deadbeef")),
        );

        let outcome = outcome_for(&root, &scan);
        assert_eq!(
            state_of(&outcome, VerificationCheck::TargetBinding),
            VerificationState::NoEvidence
        );
        assert_eq!(
            labels_of(&outcome, VerificationCheck::TargetBinding),
            vec![DiagnosticLabel::Stale]
        );
    }

    /// DS-822: an Evidence record whose `execution_state.complete` is not
    /// `true` — every record this repository's `vtest-exec` writes today,
    /// disclosed at the write site — is `UNKNOWN`, never `PASS` and never
    /// silently treated as stale. This is the disclosed stopped_on
    /// (`current Execution State subject reconstruction is not
    /// implemented`) surfacing as a real, deterministic outcome rather than
    /// an internal-error fallback (REQ-108/ROOT-033).
    ///
    /// This test cannot reach the DS-822 branch through the public
    /// `verify_project` entry point today, because DS-819's target-set
    /// check (see the previous test) fails first for every fixture this
    /// crate's test helpers can construct (`scan_result` always leaves
    /// `sources` empty). It instead calls `evidence_validity_failure`
    /// directly with a `scan` that *does* resolve the declared target, to
    /// isolate the DS-822 branch specifically.
    #[test]
    fn incomplete_execution_state_is_unknown_not_stale_or_pass() {
        let test = test_entity("TEST-ONE", &["VO-ONE"], 1);
        let target_hash = ContentHash::from_text("TEST-ONE::target0");
        let scan = ScanResult {
            summary: vtest_model::ScanSummary {
                files: 1,
                tests: 1,
                sources: 1,
            },
            discovered: Vec::new(),
            tests: vec![test.clone()],
            sources: vec![vtest_model::SourceFunction {
                locator: vtest_model::Locator {
                    adapter: AdapterId::new("rust-cargo"),
                    value: "src/lib.rs::target0".to_owned(),
                },
                src_id: None,
                location: location("SRC-target0"),
                content_hash: target_hash.clone(),
            }],
            diagnostics: Vec::new(),
        };
        let mut record = sample_evidence("TEST-ONE", "rust-cargo", Some("deadbeef"));
        record.hashes.test_fn = test.content_hash.clone();
        record.hashes.target_fn = target_hash.clone();
        record.hashes.target_fns = vec![target_hash];
        let evidence = EvidenceContext {
            root: temp_root("tb-incomplete-execution-state"),
            latest_by_test: BTreeMap::new(),
            current_adapter: AdapterId::new("rust-cargo"),
            head_commit: Some("deadbeef".to_owned()),
        };

        let outcome = evidence_validity_failure(&test, &record, &scan, &evidence)
            .expect("an incomplete Execution State must not validate as reusable");
        assert_eq!(outcome.state, VerificationState::Unknown);
        assert!(outcome.labels.is_empty());
    }

    /// The DS-822 branch also fires when the *recorded* `complete` is
    /// `true` but this crate's own current-side reconstruction cannot
    /// confirm it (e.g. HEAD is unknown) — not just when the record itself
    /// says `complete: false`.
    #[test]
    fn a_recorded_complete_state_still_falls_to_unknown_if_current_reconstruction_fails() {
        let test = test_entity("TEST-ONE", &["VO-ONE"], 1);
        let target_hash = ContentHash::from_text("TEST-ONE::target0");
        let scan = ScanResult {
            summary: vtest_model::ScanSummary {
                files: 1,
                tests: 1,
                sources: 1,
            },
            discovered: Vec::new(),
            tests: vec![test.clone()],
            sources: vec![vtest_model::SourceFunction {
                locator: vtest_model::Locator {
                    adapter: AdapterId::new("rust-cargo"),
                    value: "src/lib.rs::target0".to_owned(),
                },
                src_id: None,
                location: location("SRC-target0"),
                content_hash: target_hash.clone(),
            }],
            diagnostics: Vec::new(),
        };
        let mut record = sample_evidence("TEST-ONE", "rust-cargo", Some("deadbeef"));
        record.hashes.test_fn = test.content_hash.clone();
        record.hashes.target_fn = target_hash.clone();
        record.hashes.target_fns = vec![target_hash];
        record.execution_state.complete = true;
        record.execution_state.hash = Some(ContentHash::from_text("anything"));
        let evidence = EvidenceContext {
            // A `root` that does not exist: the manifest walk itself fails,
            // so `reconstruct_execution_state` reports `complete: false`
            // (DS-822's second disjunct), independent of DS-820's revision
            // check (`head_commit` here still matches the record's, so
            // DS-820 passes and does not mask this branch).
            root: temp_root("tb-current-reconstruction-fails").join("does-not-exist"),
            latest_by_test: BTreeMap::new(),
            current_adapter: AdapterId::new("rust-cargo"),
            head_commit: Some("deadbeef".to_owned()),
        };

        let outcome = evidence_validity_failure(&test, &record, &scan, &evidence)
            .expect("an unreconstructable current state must not validate as reusable");
        assert_eq!(outcome.state, VerificationState::Unknown);
    }

    /// DS-830/831/832, isolated from the (currently unreachable — see
    /// `evaluate_target_binding`'s doc comment) end-to-end validity path.
    #[test]
    fn dynamic_result_from_evidence_covers_ds_830_831_832() {
        let mut record = sample_evidence("TEST-ONE", "rust-cargo", Some("deadbeef"));

        // DS-830: runner FAIL.
        record.result = vtest_model::TestResult::Fail;
        assert_eq!(
            dynamic_result_from_evidence(&record).state,
            VerificationState::Fail
        );

        // DS-832: runner PASS, coverage not measured -> NO_EVIDENCE (NOT_CHECKED).
        record.result = vtest_model::TestResult::Pass;
        record.target_execution.checked = false;
        let outcome = dynamic_result_from_evidence(&record);
        assert_eq!(outcome.state, VerificationState::NoEvidence);
        assert_eq!(outcome.labels, vec![DiagnosticLabel::NotChecked]);

        // DS-832: measured count 0 -> FAIL (NOT_EXECUTED).
        record.target_execution.checked = true;
        record.target_execution.result = EvidenceCheckValue::Fail;
        record.target_execution.count = Some(0);
        let outcome = dynamic_result_from_evidence(&record);
        assert_eq!(outcome.state, VerificationState::Fail);
        assert_eq!(outcome.labels, vec![DiagnosticLabel::NotExecuted]);

        // DS-832: function not found (aggregate UNKNOWN) -> UNKNOWN.
        record.target_execution.result = EvidenceCheckValue::Unknown;
        record.target_execution.count = None;
        assert_eq!(
            dynamic_result_from_evidence(&record).state,
            VerificationState::Unknown
        );

        // DS-831: measured and reached -> PASS.
        record.target_execution.result = EvidenceCheckValue::Pass;
        record.target_execution.count = Some(3);
        let outcome = dynamic_result_from_evidence(&record);
        assert_eq!(outcome.state, VerificationState::Pass);
        assert!(outcome.labels.is_empty());
    }
}
