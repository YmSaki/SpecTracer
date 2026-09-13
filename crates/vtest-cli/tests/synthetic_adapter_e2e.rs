//! DS-1584「synthetic adapterは.rs以外のsource、関数ではないTest
//! construct、doc commentではないmetadata宣言、Rust item pathではない
//! opaque locatorを、vtest-model、vtest-scan、vtest-verifyの変更なしで
//! 登録・scan・verifyできる」の end-to-end 確認。
//!
//! `vtest-adapter-synthetic`（本 PR で新設。`vtest-adapter-api` と
//! `vtest-model` にしか依存しない）を registry に単独登録し、
//! `vtest-scan::scan_project` / `vtest_verify::verify_project` を、core
//! 側のコード変更なしにそのまま通す。static_analysis / test_runner /
//! coverage capability を宣言しない adapter なので、verify の新しい
//! capability gate（本 PR）により `oracle_presence` は
//! `NO_EVIDENCE`(`NOT_CHECKED`)、`target_binding` は
//! `NO_EVIDENCE`(`NOT_EXECUTED`) に落ちる一方、`chain_integrity` /
//! `orphan_detection` は宣言鎖が完全なので通常どおり `PASS` になる。

use std::{
    env, fs,
    path::PathBuf,
    sync::atomic::{AtomicU64, Ordering},
};

use vtest_adapter_api::AdapterRegistry;
use vtest_adapter_synthetic::SyntheticAdapter;
use vtest_model::{
    DerivesFrom, DiagnosticLabel, DocumentFile, DocumentId, NodeSource, RootNode, SentenceNode,
    VerificationCheck, VerificationState, VoId, VoRecord,
};
use vtest_scan::scan_project;
use vtest_store::{init_project, write_document_file, write_vo_record};
use vtest_verify::verify_project;

static NEXT: AtomicU64 = AtomicU64::new(0);

fn temp_root(name: &str) -> PathBuf {
    let sequence = NEXT.fetch_add(1, Ordering::Relaxed);
    let root = env::temp_dir().join(format!(
        "vtest-cli-synthetic-e2e-{name}-{}-{sequence}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(&root).expect("create fixture root");
    root
}

fn synthetic_only_registry() -> AdapterRegistry {
    let mut registry = AdapterRegistry::new();
    registry
        .register(Box::new(SyntheticAdapter::new()))
        .expect("synthetic adapter must register cleanly (discovery-only, no capability mismatch)");
    registry
}

fn document_file() -> DocumentFile {
    DocumentFile {
        schema_version: "0.1".to_owned(),
        root: vec![RootNode {
            id: DocumentId::new("ROOT-001"),
            statement: "fixture root".to_owned(),
            description: None,
            source: NodeSource {
                doc: "root.md".to_owned(),
                heading: "fixture".to_owned(),
                lines: [1, 1],
            },
        }],
        request: vec![SentenceNode {
            id: DocumentId::new("R-001"),
            statement: "fixture requirement".to_owned(),
            description: None,
            derives_from: vec![DocumentId::new("ROOT-001")],
            cites: None,
            source: NodeSource {
                doc: "req.md".to_owned(),
                heading: "fixture".to_owned(),
                lines: [1, 1],
            },
        }],
        require: Vec::new(),
        spec: Vec::new(),
        detailed_spec: Vec::new(),
        basic_design: Vec::new(),
        design: Vec::new(),
    }
}

fn vo() -> VoRecord {
    VoRecord {
        id: VoId::new("VO-SYN-ONE"),
        parent: None,
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

/// `synthetic` を唯一の adapter として宣言した `config.yaml`。
/// `ScanSection.assertion_macros` / `RunSection.coverage` はこの adapter に
/// 意味を持たないが、`vtest-store::AdapterConfig` の固定 schema（本 PR
/// 範囲外、`AGENTS.md`「config namespace の型」）が要求する必須 field
/// なので、構文上有効な値を埋めるだけで満たす。
fn write_synthetic_config(root: &std::path::Path) {
    let config_path = root.join(".verify").join("config.yaml");
    let yaml = "version: 2\n\
                project:\n  name: synthetic-fixture\n\
                adapters:\n  \
                  - id: synthetic\n    \
                    roots:\n      - '.'\n    \
                    scan:\n      include:\n        - '.'\n      assertion_macros: []\n    \
                    run:\n      coverage: none\n\
                verify:\n  full_scope:\n    - chain_integrity\n    - orphan_detection\n    - target_binding\n    - oracle_presence\n";
    fs::write(config_path, yaml).expect("write synthetic-only config.yaml");
}

fn build_fixture(root: &std::path::Path) {
    let layout = init_project(root, "synthetic-fixture").expect("init .verify/ layout");
    write_synthetic_config(root);
    write_document_file(&layout, "fixture", &document_file()).expect("write document file");
    write_vo_record(&layout, &vo()).expect("write VO record");
    fs::write(
        root.join("fixture.synthetic"),
        "@synthetic.test\n\
         @synthetic.id TEST-SYN-ONE\n\
         @synthetic.covers VO-SYN-ONE\n\
         @synthetic.target opaque-target-one\n\
         @synthetic.intent a synthetic test construct outside any Rust syntax\n\
         this construct body is plain text, not a Rust function\n\
         ---\n\
         @synthetic.source opaque-target-one\n\
         this Source Target's body is plain text, not a Rust function either\n",
    )
    .expect("write synthetic fixture file");
}

fn state_and_labels(
    outcome: &vtest_verify::VerifyOutcome,
    check: VerificationCheck,
) -> (VerificationState, Vec<DiagnosticLabel>) {
    let matching = outcome
        .all_outcomes()
        .into_iter()
        .filter(|candidate| candidate.check == check)
        .collect::<Vec<_>>();
    assert_eq!(
        matching.len(),
        1,
        "expected exactly one {check:?} outcome for the single synthetic Test"
    );
    (matching[0].state, matching[0].labels.clone())
}

/// @vtest.id TEST-CLI-SYNTHETIC-ADAPTER-SCAN-AND-VERIFY-WITHOUT-CORE-CHANGES
/// @vtest.covers VO-ADAPTER-SYNTHETIC-SCAN-VERIFY-WITHOUT-CORE-CHANGES
/// @vtest.target crates/vtest-adapter-synthetic/src/lib.rs::SyntheticAdapter
/// @vtest.intent verifies a synthetic (non-.rs, non-function, non-doc-comment, colon-free-locator) adapter registers, scans one Test, and verifies through unmodified vtest-scan/vtest-verify, with oracle_presence NO_EVIDENCE(NOT_CHECKED), target_binding NO_EVIDENCE(NOT_EXECUTED), and chain_integrity/orphan_detection evaluated normally as PASS
#[test]
fn synthetic_adapter_scans_and_verifies_through_unmodified_core() {
    let root = temp_root("basic");
    build_fixture(&root);
    let registry = synthetic_only_registry();

    let scan =
        scan_project(&root, &registry).expect("scan must succeed with the synthetic adapter");
    assert_eq!(scan.tests.len(), 1, "the synthetic Test must be discovered");
    assert_eq!(scan.tests[0].id.as_str(), "TEST-SYN-ONE");
    assert_eq!(scan.tests[0].execution.adapter.as_str(), "synthetic");
    assert!(
        !scan.tests[0].location.path.as_str().ends_with(".rs"),
        "the synthetic Test's source must not be a .rs file (DS-1584)"
    );
    assert!(
        !scan.tests[0].targets.is_empty(),
        "the fixture declares one @synthetic.target"
    );

    let outcome = verify_project(&root, &scan, None, None, &registry);

    let (oracle_state, oracle_labels) =
        state_and_labels(&outcome, VerificationCheck::OraclePresence);
    assert_eq!(oracle_state, VerificationState::NoEvidence);
    assert_eq!(oracle_labels, vec![DiagnosticLabel::NotChecked]);

    let (binding_state, binding_labels) =
        state_and_labels(&outcome, VerificationCheck::TargetBinding);
    assert_eq!(binding_state, VerificationState::NoEvidence);
    assert_eq!(binding_labels, vec![DiagnosticLabel::NotExecuted]);

    // chain_integrity / orphan_detection are whole-chain structural checks
    // (not per-Test), and this fixture's declaration chain is complete
    // (DOC -> VO -> Test, no cycles, no orphans), so both must reach PASS —
    // "通常評価" — untouched by the capability gate added above.
    let (chain_state, _) = state_and_labels(&outcome, VerificationCheck::ChainIntegrity);
    assert_eq!(chain_state, VerificationState::Pass);
    let (orphan_state, _) = state_and_labels(&outcome, VerificationCheck::OrphanDetection);
    assert_eq!(orphan_state, VerificationState::Pass);
}
