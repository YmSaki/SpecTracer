//! Shared fixture for the end-to-end tests of the `vtest-adapter-lexical`
//! language adapters: each language test writes a few source files, declares
//! its adapter as the only one in `config.yaml`, and scans and verifies them
//! through the CLI's composition root (`builtin_registry`) and the
//! unmodified `vtest-scan` / `vtest-verify` core.
#![allow(dead_code)]

use std::{
    env, fs,
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
};

use vtest_model::{
    DerivesFrom, DiagnosticLabel, DocumentFile, DocumentId, NodeSource, RootNode, SentenceNode,
    VerificationCheck, VerificationState, VoId, VoRecord,
};
use vtest_scan::{scan_project, ScanResult};
use vtest_store::{init_project, write_document_file, write_vo_record};
use vtest_verify::{verify_project, VerifyOutcome};

static NEXT: AtomicU64 = AtomicU64::new(0);

pub struct Fixture {
    pub root: PathBuf,
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

/// Writes a project whose only adapter is `adapter_id`, with one document,
/// the VOs `vo_ids`, and the source `files`.
pub fn fixture(adapter_id: &str, files: &[(&str, &str)], vo_ids: &[&str]) -> Fixture {
    let root = env::temp_dir().join(format!(
        "vtest-cli-lexical-e2e-{adapter_id}-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(&root).expect("create fixture root");
    let layout = init_project(&root, "lexical-fixture").expect("init .verify/ layout");
    write_config(&root, adapter_id);
    write_document_file(&layout, "fixture", &document_file()).expect("write document file");
    for vo_id in vo_ids {
        write_vo_record(&layout, &vo(vo_id)).expect("write VO record");
    }
    for (path, text) in files {
        let full = root.join(path);
        fs::create_dir_all(full.parent().expect("parent")).expect("create source dir");
        fs::write(full, text).expect("write source file");
    }
    Fixture { root }
}

impl Fixture {
    /// Scans and verifies through the CLI's built-in registry.
    pub fn scan_and_verify(&self) -> (ScanResult, VerifyOutcome) {
        let registry = vtest_ops::registry::builtin_registry().expect("builtin registry");
        let scan = scan_project(&self.root, &registry).expect("scan succeeds");
        let outcome = verify_project(&self.root, &scan, None, None, &registry);
        (scan, outcome)
    }

    pub fn path(&self) -> &Path {
        &self.root
    }
}

fn write_config(root: &Path, adapter_id: &str) {
    let yaml = format!(
        "version: 2\n\
         project:\n  name: lexical-fixture\n\
         adapters:\n  \
           - id: {adapter_id}\n    \
             roots:\n      - '.'\n    \
             scan:\n      include:\n        - '.'\n      assertion_macros: []\n    \
             run:\n      coverage: none\n\
         verify:\n  full_scope:\n    - chain_integrity\n    - orphan_detection\n    - target_binding\n    - oracle_presence\n"
    );
    fs::write(root.join(".verify").join("config.yaml"), yaml).expect("write config.yaml");
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

fn vo(id: &str) -> VoRecord {
    VoRecord {
        id: VoId::new(id),
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
        created: "2026-11-07T00:00:00Z".to_owned(),
        updated: "2026-11-07T00:00:00Z".to_owned(),
    }
}

/// The state and labels of `check` in a fixture with exactly one Test
/// (`test_id` names it in failure messages).
pub fn state_of(
    outcome: &VerifyOutcome,
    check: VerificationCheck,
    test_id: &str,
) -> (VerificationState, Vec<DiagnosticLabel>) {
    let matching = outcome
        .all_outcomes()
        .into_iter()
        .filter(|candidate| candidate.check == check)
        .collect::<Vec<_>>();
    assert_eq!(
        matching.len(),
        1,
        "expected exactly one {check:?} outcome for the single Test {test_id}"
    );
    (matching[0].state, matching[0].labels.clone())
}

/// Asserts the verdict every discovery-only adapter must give a declared,
/// fully linked Test: the trace checks pass, and the checks that need a
/// runner, coverage, or static analysis stay `NO_EVIDENCE` instead of a
/// framework-specific default.
pub fn assert_discovery_only_verdict(outcome: &VerifyOutcome, test_id: &str) {
    let (chain, _) = state_of(outcome, VerificationCheck::ChainIntegrity, test_id);
    assert_eq!(chain, VerificationState::Pass, "chain_integrity");
    let (orphan, _) = state_of(outcome, VerificationCheck::OrphanDetection, test_id);
    assert_eq!(orphan, VerificationState::Pass, "orphan_detection");
    let (oracle, oracle_labels) = state_of(outcome, VerificationCheck::OraclePresence, test_id);
    assert_eq!(oracle, VerificationState::NoEvidence);
    assert_eq!(oracle_labels, vec![DiagnosticLabel::NotChecked]);
    let (binding, binding_labels) = state_of(outcome, VerificationCheck::TargetBinding, test_id);
    assert_eq!(binding, VerificationState::NoEvidence);
    assert_eq!(
        binding_labels,
        vec![DiagnosticLabel::NotExecuted, DiagnosticLabel::NotChecked]
    );
}
