//! What the end-to-end tests of the `go` adapter share: a real Go project in a
//! fresh git repository (Evidence binds the HEAD revision, DS-265), `vtest run`
//! and `vtest verify` through the CLI's composition root, and the reading of
//! their results. `go` and `git` must be on `PATH`: a missing one fails the test
//! with a message instead of passing without having run anything.

use std::{path::Path, process::Command};

use vtest_exec::ExecutionResult;
use vtest_model::{DiagnosticLabel, EvidenceRecord, VerificationCheck, VerificationState};
use vtest_ops::run::RunTarget;
use vtest_scan::scan_project;
use vtest_store::VerifyLayout;
use vtest_verify::{verify_project, NodeKind, TreeNode, VerifyOutcome};

pub fn require_tool(tool: &str, args: &[&str]) {
    let ran = Command::new(tool)
        .args(args)
        .output()
        .ok()
        .filter(|output| output.status.success());
    assert!(
        ran.is_some(),
        "`{tool}` must be on PATH: the go tests run the real toolchain"
    );
}

pub fn git(root: &Path, args: &[&str]) {
    let output = Command::new("git")
        .current_dir(root)
        .args(["-c", "commit.gpgsign=false", "-c", "gpg.format=openpgp"])
        .args(args)
        .output()
        .unwrap_or_else(|error| panic!("failed to run git {args:?}: {error}"));
    assert!(
        output.status.success(),
        "git {args:?} failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

/// A Go project whose only adapter is `go`, committed to a fresh git
/// repository (Evidence binds the HEAD revision, DS-265).
pub fn project(files: &[(&str, &str)]) -> super::Fixture {
    require_tool("go", &["version"]);
    require_tool("git", &["--version"]);
    let fixture = super::fixture("go", files, &["VO-FIXTURE-ONE"]);
    commit(&fixture);
    fixture
}

pub fn commit(fixture: &super::Fixture) {
    git(&fixture.root, &["init", "-q"]);
    git(
        &fixture.root,
        &["config", "user.email", "vtest@example.com"],
    );
    git(&fixture.root, &["config", "user.name", "vtest fixture"]);
    git(&fixture.root, &["add", "."]);
    git(&fixture.root, &["commit", "-q", "-m", "fixture"]);
}

/// `vtest run --all` (or `--fast`) and then `vtest verify`.
pub fn run_and_verify(fixture: &super::Fixture, fast: bool) -> (ExecutionResult, VerifyOutcome) {
    let registry = vtest_ops::registry::builtin_registry().expect("builtin registry");
    let scan = scan_project(&fixture.root, &registry).expect("scan");
    assert!(!scan.has_errors(), "{:?}", scan.diagnostics);
    let layout = VerifyLayout::new(&fixture.root);
    let result = vtest_ops::run::run(
        &fixture.root,
        &layout,
        &scan,
        &RunTarget::All,
        fast,
        &registry,
    )
    .expect("run");
    let after = scan_project(&fixture.root, &registry).expect("rescan");
    let outcome = verify_project(&fixture.root, &after, None, None, &registry);
    (result, outcome)
}

pub fn codes(result: &ExecutionResult) -> Vec<&str> {
    result
        .diagnostics
        .iter()
        .map(|diagnostic| diagnostic.code.as_str())
        .collect()
}

pub fn only_evidence(result: &ExecutionResult) -> &EvidenceRecord {
    assert_eq!(result.evidence.len(), 1, "{:?}", result.diagnostics);
    &result.evidence[0]
}

/// The state and labels of `check` for the Tests of the project: every
/// outcome of that check must have the same state, which is returned.
pub fn state_of(
    outcome: &VerifyOutcome,
    check: VerificationCheck,
) -> (VerificationState, Vec<DiagnosticLabel>) {
    let matching: Vec<_> = outcome
        .all_outcomes()
        .into_iter()
        .filter(|candidate| candidate.check == check)
        .collect();
    assert!(!matching.is_empty(), "no {check:?} outcome");
    assert!(
        matching
            .iter()
            .all(|candidate| (candidate.state, &candidate.labels)
                == (matching[0].state, &matching[0].labels)),
        "{matching:?}"
    );
    (matching[0].state, matching[0].labels.clone())
}

/// The state, the labels and the basis of `check` for the Test `test_id`.
pub fn outcome_of_test(
    outcome: &VerifyOutcome,
    test_id: &str,
    check: VerificationCheck,
) -> (VerificationState, Vec<DiagnosticLabel>, Vec<String>) {
    fn find<'a>(nodes: &'a [TreeNode], test_id: &str) -> Option<&'a TreeNode> {
        nodes.iter().find_map(|node| {
            if node.kind == NodeKind::Test && node.id == test_id {
                Some(node)
            } else {
                find(&node.children, test_id)
            }
        })
    }
    let node = find(&outcome.tree, test_id).unwrap_or_else(|| panic!("no node of {test_id}"));
    let outcome = node
        .checks
        .iter()
        .find(|candidate| candidate.check == check)
        .unwrap_or_else(|| panic!("no {check:?} outcome of {test_id}"));
    (outcome.state, outcome.labels.clone(), outcome.basis.clone())
}

/// The state and labels of `check` for the Test `test_id`.
pub fn state_of_test(
    outcome: &VerifyOutcome,
    test_id: &str,
    check: VerificationCheck,
) -> (VerificationState, Vec<DiagnosticLabel>) {
    let (state, labels, _) = outcome_of_test(outcome, test_id, check);
    (state, labels)
}
