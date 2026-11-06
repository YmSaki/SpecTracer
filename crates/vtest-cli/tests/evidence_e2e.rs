//! End-to-end demonstration: a fixture on which `vtest verify` reaches
//! `ok: true` with all four canonical checks `PASS`, driven by a real
//! `cargo test` execution through `vtest-exec` and a real Evidence-backed
//! `target_binding` judgment through `vtest-verify` (DS-816-833,
//! DES-097-101/210-212). This is the completion criterion for the
//! Evidence slice this test file was added under.
//!
//! Uses the library entry points directly (`scan_project` / `run_tests` /
//! `verify_project`) rather than the `vtest` CLI binary, because this
//! repository's CLI does not currently expose a `vtest exec` subcommand —
//! disclosed rather than invented: adding new CLI surface is out of this
//! test's scope.

use std::{
    env, fs,
    path::{Path, PathBuf},
    process::Command,
    sync::atomic::{AtomicU64, Ordering},
    time::{SystemTime, UNIX_EPOCH},
};

use vtest_exec::{run_tests, RunnableTest};
use vtest_model::{
    DerivesFrom, DocumentFile, DocumentId, NodeSource, RootNode, SentenceNode, TestId,
    VerificationState, VoId, VoRecord,
};
use vtest_scan::scan_project;
use vtest_store::{init_project, write_document_file, write_vo_record};
use vtest_verify::verify_project;

fn registry() -> vtest_adapter_api::AdapterRegistry {
    vtest_cli::adapters::builtin_registry().expect("builtin registry must register cleanly")
}

static NEXT: AtomicU64 = AtomicU64::new(0);

fn temp_root(name: &str) -> PathBuf {
    let sequence = NEXT.fetch_add(1, Ordering::Relaxed);
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let root = env::temp_dir().join(format!(
        "vtest-cli-evidence-e2e-{name}-{}-{nanos}-{sequence}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(&root).expect("create fixture root");
    root
}

fn git(root: &Path, args: &[&str]) {
    let status = Command::new("git")
        .current_dir(root)
        .args(
            [
                &["-c", "commit.gpgsign=false", "-c", "gpg.format=openpgp"],
                args,
            ]
            .concat(),
        )
        .status()
        .unwrap_or_else(|error| panic!("failed to run git {args:?}: {error}"));
    assert!(status.success(), "git {args:?} failed");
}

fn clear_outer_coverage_environment() {
    // The Execution State binds the execution-affecting environment
    // (DES-097/369), so a variable removed by one test while another test is
    // between its pre- and post-run snapshots would read as an input change
    // (E-EXEC-004). Clear once, before any test reaches its first run.
    static CLEARED: std::sync::Once = std::sync::Once::new();
    CLEARED.call_once(clear_outer_coverage_environment_now);
}

fn clear_outer_coverage_environment_now() {
    // `CARGO_TARGET_DIR` is included alongside the coverage/rustflags
    // variables for a distinct, empirically confirmed reason: every fixture
    // built here declares its own `[workspace]`, so `cargo test`/
    // `cargo llvm-cov` inside it defaults to a fixture-root-local `target/`
    // — but only while `CARGO_TARGET_DIR` is unset. `RustCargoTestRunner`
    // sets no `env` overrides of its own, so an ambient `CARGO_TARGET_DIR`
    // (a common local perf setup) is inherited unchanged into every
    // fixture's inner cargo invocation. Several fixtures in this file share
    // the same package name, integration test target name (`registered`),
    // and version; under `cargo test`'s default parallel test threads their
    // inner cargo invocations then race for the same physical target
    // directory and Cargo's build-unit cache serves one fixture's
    // already-built test binary to a concurrently running, differently
    // sourced fixture. Clearing it here restores each fixture's intended
    // per-root isolation.
    for variable in [
        "RUSTC_WRAPPER",
        "LLVM_PROFILE_FILE",
        "CARGO_LLVM_COV",
        "CARGO_LLVM_COV_TARGET_DIR",
        "RUSTFLAGS",
        "CARGO_ENCODED_RUSTFLAGS",
        "CARGO_INCREMENTAL",
        "CARGO_TARGET_DIR",
    ] {
        // These tests invoke the runner in-process; isolate only the test
        // process from the outer cargo llvm-cov environment.
        std::env::remove_var(variable);
    }
}

fn source(id: &str) -> NodeSource {
    NodeSource {
        doc: format!("{id}.md"),
        heading: "fixture".to_owned(),
        lines: [1, 1],
    }
}

/// A minimal, fully-connected document file: one `root` node and one
/// `request` sentence deriving from it — every non-root node has an
/// effective upstream, so `orphan_detection` is `PASS` (mirrors
/// `vtest-verify`'s own `document_file()` test fixture).
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
        created: "2026-09-08T00:00:00Z".to_owned(),
        updated: "2026-09-08T00:00:00Z".to_owned(),
    }
}

/// Builds a small standalone Cargo project (own `[workspace]`, so `cargo
/// test` inside it never touches this repository's own workspace), commits
/// it to a fresh git repository (Evidence binds HEAD revision, DS-265/DES-097),
/// and registers its one Test's declaration chain (DOC -> VO -> Test) under
/// `.verify/`.
const DEFAULT_TEST_BODY: &str = "/// @vtest.id TEST-E2E-DOUBLE\n\
     /// @vtest.covers VO-E2E-DOUBLE\n\
     /// @vtest.target src/lib.rs::double\n\
     /// @vtest.intent doubles the input\n\
     #[test]\n\
     fn it_doubles() {\n    assert_eq!(vtest_e2e_fixture::double(2), 4);\n}\n";

fn build_fixture_project(root: &Path) {
    build_fixture_project_with_test_body(root, DEFAULT_TEST_BODY);
}

fn build_fixture_project_with_test_body(root: &Path, test_body: &str) {
    fs::create_dir_all(root.join("src")).expect("mkdir src");
    fs::create_dir_all(root.join("tests")).expect("mkdir tests");
    fs::write(
        root.join("Cargo.toml"),
        "[package]\nname = \"vtest-e2e-fixture\"\nversion = \"0.1.0\"\nedition = \"2021\"\n\n\
         [workspace]\n",
    )
    .expect("write Cargo.toml");
    fs::write(
        root.join("src").join("lib.rs"),
        "pub fn double(x: i32) -> i32 { x * 2 }\n",
    )
    .expect("write src/lib.rs");
    fs::write(root.join("tests").join("registered.rs"), test_body)
        .expect("write tests/registered.rs");

    // DES-371: dependency resolution must be settled before the measured
    // execution; creating Cargo.lock during the run is an input change.
    let status = Command::new("cargo")
        .current_dir(root)
        .args(["generate-lockfile", "--offline"])
        .status()
        .expect("generate fixture lockfile");
    assert!(status.success());

    git(root, &["init", "-q"]);
    git(root, &["config", "user.email", "vtest-fixture@example.com"]);
    git(root, &["config", "user.name", "vtest fixture"]);
    git(root, &["add", "."]);
    git(root, &["commit", "-q", "-m", "initial fixture commit"]);

    let layout = init_project(root, "vtest-e2e-fixture").expect("init .verify/ layout");
    write_document_file(&layout, "fixture", &document_file()).expect("write document file");
    write_vo_record(&layout, &vo("VO-E2E-DOUBLE")).expect("write VO record");
}

/// Runs `TEST-E2E-DOUBLE` through `vtest-exec` (writing real Evidence) and
/// returns the resulting `vtest-verify` outcome — the shared drive loop the
/// PASS/non-PASS fixture tests in this file all use.
fn run_and_verify(root: &Path) -> vtest_verify::VerifyOutcome {
    let scan = scan_project(root, &registry()).expect("scan the fixture project");
    assert!(
        !scan.has_errors(),
        "scan reported errors: {:?}",
        scan.diagnostics
    );
    let entity = scan
        .tests
        .iter()
        .find(|test| test.id == TestId::new("TEST-E2E-DOUBLE"))
        .cloned()
        .expect("TEST-E2E-DOUBLE was discovered and materialized");
    let target_locator = match entity.targets.first() {
        Some(vtest_model::TargetRef::Locator(locator)) => locator.clone(),
        other => panic!("expected a single Locator target, got {other:?}"),
    };
    let source_fn = scan
        .sources
        .iter()
        .find(|source| source.locator == target_locator)
        .expect("declared target resolved to a discovered Source Target");
    let runnable = RunnableTest {
        entity: entity.clone(),
        target_hashes: vec![source_fn.content_hash.clone()],
        target_locators: vec![target_locator],
    };
    let layout = vtest_store::VerifyLayout::new(root);
    clear_outer_coverage_environment();
    run_tests(root, &layout, &[runnable], false, &registry())
        .expect("run_tests executes without I/O errors");

    let scan_after = scan_project(root, &registry()).expect("re-scan after execution");
    verify_project(root, &scan_after, None, None, &registry())
}

/// The completion criterion for this Evidence slice: a fixture on which
/// `vtest verify` reaches `ok: true` with all four canonical checks `PASS`.
/// @vtest.id TEST-E2E-FULL-PASS
/// @vtest.covers VO-VERIFY-EVIDENCE-RUNNER-FAIL, VO-VERIFY-EVIDENCE-PASS-TARGETS-REACHED, VO-VERIFY-EVIDENCE-PASS-TARGETS-UNREACHED
/// @vtest.target crates/vtest-verify/src/lib.rs::verify_project
/// @vtest.intent a covered Test with a git-committed environment and valid Evidence reaches ok:true with all four checks PASS
#[test]
fn a_covered_test_with_a_git_committed_environment_reaches_ok_true() {
    let root = temp_root("full-pass");
    build_fixture_project(&root);

    let scan = scan_project(&root, &registry()).expect("scan the fixture project");
    assert!(
        !scan.has_errors(),
        "scan reported errors: {:?}",
        scan.diagnostics
    );
    let entity = scan
        .tests
        .iter()
        .find(|test| test.id == TestId::new("TEST-E2E-DOUBLE"))
        .cloned()
        .expect("TEST-E2E-DOUBLE was discovered and materialized");
    let target_locator = match entity.targets.first() {
        Some(vtest_model::TargetRef::Locator(locator)) => locator.clone(),
        other => panic!("expected a single Locator target, got {other:?}"),
    };
    let source_fn = scan
        .sources
        .iter()
        .find(|source| source.locator == target_locator)
        .expect("declared target resolved to a discovered Source Target");

    let runnable = RunnableTest {
        entity: entity.clone(),
        target_hashes: vec![source_fn.content_hash.clone()],
        target_locators: vec![target_locator],
    };
    let layout = vtest_store::VerifyLayout::new(&root);
    clear_outer_coverage_environment();
    let exec_result = run_tests(&root, &layout, &[runnable], false, &registry())
        .expect("run_tests executes without I/O errors");
    assert!(
        !exec_result.has_errors(),
        "run_tests reported errors: {:?}",
        exec_result.diagnostics
    );
    assert_eq!(
        exec_result.evidence.len(),
        1,
        "expected exactly one Evidence record to be written"
    );
    assert_eq!(
        exec_result.evidence[0].result,
        vtest_model::TestResult::Pass
    );
    assert!(
        exec_result.evidence[0].execution_state.complete,
        "the fixture has no build.rs/include!/target-directory escape risk, so the \
         Execution State subject must reconstruct as complete (DES-212)"
    );

    // Re-scan: `run_tests` wrote a new Evidence record under `.verify/evidence/`,
    // which does not change anything `scan_project` reads, but re-scanning
    // documents that `verify_project` reads Evidence from disk independently
    // of the in-memory `scan` used to drive execution above.
    let scan_after = scan_project(&root, &registry()).expect("re-scan after execution");
    let outcome = verify_project(&root, &scan_after, None, None, &registry());

    assert_eq!(
        outcome.state,
        VerificationState::Pass,
        "expected ok:true (all four checks PASS); got: {}",
        serde_json::to_string_pretty(&outcome).unwrap_or_default()
    );
    assert!(outcome.ok);

    fs::remove_dir_all(&root).ok();
}

/// Stale-Evidence negative case for the same fixture: an Evidence record
/// whose recorded Test subject hash no longer matches (the declared
/// `@vtest.intent` changed after the record was written) must not be
/// reused as `PASS` (DS-819/833) — `target_binding` falls to `NO_EVIDENCE`
/// (`STALE`), so overall `ok` stays `false`.
/// @vtest.id TEST-E2E-STALE-SUBJECT-HASH
/// @vtest.covers VO-VERIFY-STALE-TEST-SUBJECT-HASH-NOT-REUSED
/// @vtest.target crates/vtest-verify/src/lib.rs::verify_project
/// @vtest.intent an Evidence record whose recorded test_subject hash no longer matches must not be reused as PASS; target_binding falls to NO_EVIDENCE (STALE)
#[test]
fn a_stale_test_subject_hash_never_reaches_pass() {
    let root = temp_root("stale-subject-hash");
    build_fixture_project(&root);

    let scan = scan_project(&root, &registry()).expect("scan the fixture project");
    let entity = scan
        .tests
        .iter()
        .find(|test| test.id == TestId::new("TEST-E2E-DOUBLE"))
        .cloned()
        .expect("TEST-E2E-DOUBLE was discovered and materialized");
    let target_locator = match entity.targets.first() {
        Some(vtest_model::TargetRef::Locator(locator)) => locator.clone(),
        other => panic!("expected a single Locator target, got {other:?}"),
    };
    let source_fn = scan
        .sources
        .iter()
        .find(|source| source.locator == target_locator)
        .expect("declared target resolved to a discovered Source Target");
    let runnable = RunnableTest {
        entity: entity.clone(),
        target_hashes: vec![source_fn.content_hash.clone()],
        target_locators: vec![target_locator],
    };
    let layout = vtest_store::VerifyLayout::new(&root);
    clear_outer_coverage_environment();
    run_tests(&root, &layout, &[runnable], false, &registry()).expect("run_tests");

    // Change the declaration (the Test subject hash's own bound `intent`
    // field, per `test_subject_hash`) without producing new Evidence —
    // the recorded Evidence now describes a subject that no longer exists.
    fs::write(
        root.join("tests").join("registered.rs"),
        "/// @vtest.id TEST-E2E-DOUBLE\n\
         /// @vtest.covers VO-E2E-DOUBLE\n\
         /// @vtest.target src/lib.rs::double\n\
         /// @vtest.intent doubles the input, now with a changed intent\n\
         #[test]\n\
         fn it_doubles() {\n    assert_eq!(vtest_e2e_fixture::double(2), 4);\n}\n",
    )
    .expect("rewrite the declaration");

    let scan_after =
        scan_project(&root, &registry()).expect("re-scan after the declaration changed");
    let outcome = verify_project(&root, &scan_after, None, None, &registry());

    assert_ne!(outcome.state, VerificationState::Pass);
    assert!(!outcome.ok);
    let target_binding_outcomes: Vec<_> = outcome
        .all_outcomes()
        .into_iter()
        .filter(|check| check.check == vtest_model::VerificationCheck::TargetBinding)
        .collect();
    assert!(target_binding_outcomes
        .iter()
        .any(|check| check.state == VerificationState::NoEvidence
            && check.labels.contains(&vtest_model::DiagnosticLabel::Stale)));

    fs::remove_dir_all(&root).ok();
}

fn only_check_state(
    outcome: &vtest_verify::VerifyOutcome,
    check: vtest_model::VerificationCheck,
) -> VerificationState {
    let states: Vec<VerificationState> = outcome
        .all_outcomes()
        .into_iter()
        .filter(|outcome| outcome.check == check)
        .map(|outcome| outcome.state)
        .collect();
    assert_eq!(states.len(), 1, "expected exactly one {check:?} outcome");
    states[0]
}

/// Isolated non-`PASS` case 3 of 4: `target_binding` alone breaks. No
/// `vtest-exec` run happens for this Test, so `target_binding` falls to
/// `NO_EVIDENCE` (DS-825) while `chain_integrity`/`orphan_detection` (a
/// complete DOC->VO->Test chain) and `oracle_presence` (a real
/// `assert_eq!` over the target's call, needing no Evidence) all `PASS`.
/// @vtest.id TEST-E2E-ONLY-TARGET-BINDING-BREAKS
/// @vtest.covers VO-VERIFY-NO-EVIDENCE-NOT-EXECUTED
/// @vtest.target crates/vtest-verify/src/lib.rs::verify_project
/// @vtest.intent with no vtest-exec run, target_binding alone falls to NO_EVIDENCE while chain_integrity/orphan_detection/oracle_presence all PASS
#[test]
fn only_target_binding_breaks_when_no_evidence_exists() {
    let root = temp_root("only-target-binding-breaks");
    build_fixture_project(&root);
    let scan = scan_project(&root, &registry()).expect("scan the fixture project");

    let outcome = verify_project(&root, &scan, None, None, &registry());

    assert_eq!(
        only_check_state(&outcome, vtest_model::VerificationCheck::ChainIntegrity),
        VerificationState::Pass
    );
    assert_eq!(
        only_check_state(&outcome, vtest_model::VerificationCheck::OrphanDetection),
        VerificationState::Pass
    );
    assert_eq!(
        only_check_state(&outcome, vtest_model::VerificationCheck::TargetBinding),
        VerificationState::NoEvidence
    );
    assert_eq!(
        only_check_state(&outcome, vtest_model::VerificationCheck::OraclePresence),
        VerificationState::Pass
    );
    assert_ne!(outcome.state, VerificationState::Pass);
    assert!(!outcome.ok);

    fs::remove_dir_all(&root).ok();
}

/// Isolated non-`PASS` case 4 of 4: `oracle_presence` alone breaks. The
/// Test body calls the declared target on both sides of `assert_eq!` with
/// token-identical arguments (`assert_eq!(double(2), double(2))`) — DA-004
/// (DS-624, self comparison) `FAIL`s even though the call is genuinely
/// executed and its result genuinely reaches an assert, so `target_binding`
/// still reaches `PASS` via DS-831 and the structural checks stay `PASS`.
/// @vtest.id TEST-E2E-ONLY-ORACLE-PRESENCE-BREAKS
/// @vtest.covers VO-ORACLE-DA-004-SELF-COMPARISON
/// @vtest.target crates/vtest-verify/src/lib.rs::verify_project
/// @vtest.intent a self-comparison assert_eq!(double(2), double(2)) fails DA-004 oracle_presence even though target_binding still reaches PASS
#[test]
fn only_oracle_presence_breaks_on_a_self_comparison_assertion() {
    let root = temp_root("only-oracle-presence-breaks");
    build_fixture_project_with_test_body(
        &root,
        "/// @vtest.id TEST-E2E-DOUBLE\n\
         /// @vtest.covers VO-E2E-DOUBLE\n\
         /// @vtest.target src/lib.rs::double\n\
         /// @vtest.intent doubles the input\n\
         #[test]\n\
         fn it_doubles() {\n    \
             assert_eq!(vtest_e2e_fixture::double(2), vtest_e2e_fixture::double(2));\n\
         }\n",
    );

    let outcome = run_and_verify(&root);

    assert_eq!(
        only_check_state(&outcome, vtest_model::VerificationCheck::ChainIntegrity),
        VerificationState::Pass
    );
    assert_eq!(
        only_check_state(&outcome, vtest_model::VerificationCheck::OrphanDetection),
        VerificationState::Pass
    );
    assert_eq!(
        only_check_state(&outcome, vtest_model::VerificationCheck::TargetBinding),
        VerificationState::Pass
    );
    assert_eq!(
        only_check_state(&outcome, vtest_model::VerificationCheck::OraclePresence),
        VerificationState::Fail
    );
    assert_ne!(outcome.state, VerificationState::Pass);
    assert!(!outcome.ok);

    fs::remove_dir_all(&root).ok();
}

/// A fixture Test declaring two targets, only one of which the body
/// reaches, run once through the real coverage runner. Returns the project
/// root and the Evidence record the run wrote.
fn run_multi_target_fixture(name: &str) -> (PathBuf, vtest_model::EvidenceRecord) {
    let root = temp_root(name);
    build_fixture_project_with_test_body(
        &root,
        "/// @vtest.id TEST-E2E-DOUBLE\n\
         /// @vtest.covers VO-E2E-DOUBLE\n\
         /// @vtest.target src/lib.rs::double\n\
         /// @vtest.target src/lib.rs::unused\n\
         /// @vtest.intent doubles the input\n\
         #[test]\n\
         fn it_doubles() {\n    assert_eq!(vtest_e2e_fixture::double(2), 4);\n}\n",
    );
    fs::write(
        root.join("src").join("lib.rs"),
        "pub fn double(x: i32) -> i32 { x * 2 }\npub fn unused() -> i32 { 99 }\n",
    )
    .expect("rewrite src/lib.rs");
    git(&root, &["commit", "-q", "-am", "add an unused target"]);

    let scan = scan_project(&root, &registry()).expect("scan");
    assert!(!scan.has_errors(), "scan errors: {:?}", scan.diagnostics);
    let entity = scan
        .tests
        .iter()
        .find(|test| test.id == TestId::new("TEST-E2E-DOUBLE"))
        .cloned()
        .expect("TEST-E2E-DOUBLE discovered");
    let mut target_hashes = Vec::new();
    let mut target_locators = Vec::new();
    for target in &entity.targets {
        let vtest_model::TargetRef::Locator(locator) = target else {
            panic!("expected Locator targets");
        };
        let source = scan
            .sources
            .iter()
            .find(|source| &source.locator == locator)
            .expect("target resolved");
        target_hashes.push(source.content_hash.clone());
        target_locators.push(locator.clone());
    }
    assert_eq!(target_locators.len(), 2);
    let runnable = RunnableTest {
        entity,
        target_hashes,
        target_locators,
    };
    let layout = vtest_store::VerifyLayout::new(&root);
    clear_outer_coverage_environment();
    let result = run_tests(&root, &layout, &[runnable], false, &registry()).expect("run_tests");
    assert!(!result.has_errors(), "run errors: {:?}", result.diagnostics);
    (root, result.evidence[0].clone())
}

/// Issue #122: a Test declaring two targets, only one of which the test
/// body reaches, must not verify as `PASS` — every declared target is
/// measured and recorded, and the unreached one keeps `target_binding`
/// non-`PASS` (DS-831/DS-1410/DS-1411).
/// @vtest.id TEST-E2E-MULTI-TARGET-UNREACHED-NOT-PASS
/// @vtest.covers VO-VERIFY-EVIDENCE-PASS-TARGETS-UNREACHED
/// @vtest.target crates/vtest-exec/src/lib.rs::run_tests_with_runner
/// @vtest.intent 2件のtargetを宣言し片方へ到達しないTestで、Evidenceが2件のtarget entryを持ち、verifyがPASSにならないことを確認する
#[test]
fn a_test_that_reaches_only_one_of_two_declared_targets_is_not_pass() {
    let (root, evidence) = run_multi_target_fixture("multi-target");
    let coverage = &evidence.target_coverage;
    assert!(coverage.checked);
    assert_eq!(
        coverage.targets.len(),
        2,
        "every declared target is recorded"
    );
    assert_ne!(
        coverage.result,
        Some(vtest_model::TargetCoverageResult::Pass)
    );

    let scan_after = scan_project(&root, &registry()).expect("re-scan");
    let outcome = verify_project(&root, &scan_after, None, None, &registry());
    assert_eq!(
        only_check_state(&outcome, vtest_model::VerificationCheck::TargetBinding),
        VerificationState::Fail,
        "{}",
        serde_json::to_string_pretty(&outcome).unwrap_or_default()
    );
    assert!(!outcome.ok);
    fs::remove_dir_all(&root).ok();
}

/// PR #131 review: Evidence whose aggregate `target_coverage.result`
/// contradicts its per-target entries (an UNKNOWN aggregate over a FAIL
/// entry) is E-SCAN-010 (DS-476). `vtest verify` does not use it — the
/// Test's `target_binding` is NO_EVIDENCE (NOT_EXECUTED, DS-1679) — and the
/// E-SCAN-010 diagnostic reaches the JSON envelope's `diagnostics`, not
/// only a basis string.
/// @vtest.id TEST-E2E-EVIDENCE-AGGREGATE-CONTRADICTION-DIAGNOSTIC
/// @vtest.covers VO-VERIFY-EVIDENCE-TARGET-ENTRY-CONTRADICTION-E-SCAN-010
/// @vtest.target crates/vtest-cli/src/ops/verify.rs::execute
/// @vtest.intent aggregateをUNKNOWNへ書き換えたEvidenceで、vtest verifyのJSON envelopeのdiagnosticsにE-SCAN-010が入り、target_bindingがNO_EVIDENCE(NOT_EXECUTED)になることを確認する
#[test]
fn verify_reports_e_scan_010_for_an_aggregate_that_contradicts_its_entries() {
    let (root, evidence) = run_multi_target_fixture("aggregate-contradiction");
    assert_eq!(
        evidence.target_coverage.result,
        Some(vtest_model::TargetCoverageResult::Fail)
    );
    let path = vtest_store::VerifyLayout::new(&root)
        .evidence_dir()
        .join(format!("{}.yaml", evidence.id));
    let text = fs::read_to_string(&path).expect("read Evidence");
    // Only the aggregate line of the `target_coverage` block changes; the
    // per-target entries stay as measured (one PASS, one FAIL with count 0).
    let block = text
        .find("\ntarget_coverage:\n")
        .expect("target_coverage block");
    let (head, tail) = text.split_at(block);
    let rewritten = format!(
        "{head}{}",
        tail.replacen("\n  result: 'FAIL'\n", "\n  result: 'UNKNOWN'\n", 1)
    );
    assert_ne!(rewritten, text, "aggregate result line rewritten:\n{text}");
    fs::write(&path, rewritten).expect("rewrite Evidence");

    let output = Command::new(env!("CARGO_BIN_EXE_vtest"))
        .arg("--project")
        .arg(&root)
        .args(["--format", "json", "verify"])
        .output()
        .expect("run vtest verify");
    assert_eq!(output.status.code(), Some(1));
    let envelope: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("JSON envelope");
    let diagnostics = envelope["diagnostics"].as_array().expect("diagnostics");
    assert!(
        diagnostics
            .iter()
            .any(|diagnostic| diagnostic["code"] == "E-SCAN-010"
                && diagnostic["severity"] == "error"),
        "{envelope:#}"
    );
    let rendered = envelope.to_string();
    assert!(rendered.contains("NOT_EXECUTED"), "{envelope:#}");
    fs::remove_dir_all(&root).ok();
}

/// Issue #123: a newer Evidence record that is not a valid record must not
/// be skipped in favour of the older valid `PASS` (DS-1365/DS-1366/DS-1679).
/// @vtest.id TEST-E2E-INVALID-LATEST-EVIDENCE-NO-FALLBACK
/// @vtest.covers VO-VERIFY-STALE-TEST-SUBJECT-HASH-NOT-REUSED
/// @vtest.target crates/vtest-verify/src/lib.rs::verify_project
/// @vtest.intent 最新Evidenceが不正（未知field・id/ファイル名不一致）のとき古いPASS Evidenceへ戻らずNO_EVIDENCEとし、E-SCAN-010診断をverifyの結果に載せることを確認する
#[test]
fn an_invalid_latest_evidence_record_does_not_fall_back_to_an_older_pass() {
    let root = temp_root("invalid-latest");
    build_fixture_project(&root);
    let outcome = run_and_verify(&root);
    assert_eq!(
        outcome.state,
        VerificationState::Pass,
        "control: valid PASS"
    );

    let evidence_dir = root.join(".verify").join("evidence");
    let original = fs::read_dir(&evidence_dir)
        .expect("evidence dir")
        .flatten()
        .map(|entry| entry.path())
        .find(|path| path.extension().and_then(|e| e.to_str()) == Some("yaml"))
        .expect("one Evidence record");
    let original_id = original
        .file_stem()
        .and_then(|stem| stem.to_str())
        .expect("ULID file name")
        .to_owned();
    let text = fs::read_to_string(&original).expect("read Evidence");
    // A later record for the same Test with the same freshness binding.
    let later_id = "7ZZZZZZZZZZZZZZZZZZZZZZZZZ";
    let later = evidence_dir.join(format!("{later_id}.yaml"));
    let failed = text
        .replace(&original_id, later_id)
        .replacen("result: PASS", "result: FAIL", 1)
        .replacen("result: 'PASS'", "result: 'FAIL'", 1);
    fs::write(&later, &failed).expect("write later Evidence");
    let scan = scan_project(&root, &registry()).expect("scan");
    let valid_latest = verify_project(&root, &scan, None, None, &registry());
    assert_eq!(
        only_check_state(&valid_latest, vtest_model::VerificationCheck::TargetBinding),
        VerificationState::Fail,
        "control: a valid later FAIL record is the latest"
    );

    for (name, broken) in [
        ("unknown field", format!("{failed}unexpected: true\n")),
        (
            "id differs from the file name",
            failed.replace(later_id, "7ZZZZZZZZZZZZZZZZZZZZZZZZY"),
        ),
        (
            // PR #132 review: the broken record's Test cannot be told, so it
            // may be this Test's latest; the older PASS must not stand in.
            "unknown field and test_id: null",
            format!(
                "{}unexpected: true\n",
                failed
                    .lines()
                    .map(|line| if line.starts_with("test_id:") {
                        "test_id: null"
                    } else {
                        line
                    })
                    .collect::<Vec<_>>()
                    .join("\n")
                    + "\n"
            ),
        ),
    ] {
        fs::write(&later, &broken).expect("write broken later Evidence");
        let scan = scan_project(&root, &registry()).expect("scan");
        let outcome = verify_project(&root, &scan, None, None, &registry());
        assert_eq!(
            only_check_state(&outcome, vtest_model::VerificationCheck::TargetBinding),
            VerificationState::NoEvidence,
            "{name}: the older PASS must not stand in for the invalid latest record"
        );
        assert!(!outcome.ok, "{name}");
        assert!(
            outcome
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.code == "E-SCAN-010"),
            "{name}: {:?}",
            outcome.diagnostics
        );
        let (_, _, diagnostics) =
            vtest_cli::ops::verify::execute(&root, &[], None, None, None, None, false, &registry())
                .unwrap_or_else(|_| panic!("{name}: verify operation"));
        assert!(
            diagnostics
                .iter()
                .any(|diagnostic| diagnostic.code == "E-SCAN-010"),
            "{name}: the CLI/MCP envelope carries the diagnostic"
        );
    }
    fs::remove_dir_all(&root).ok();
}
