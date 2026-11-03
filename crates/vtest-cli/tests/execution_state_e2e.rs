//! End-to-end regressions for Execution State completeness (Issues #117,
//! #118, #119, #121): a real `cargo llvm-cov` run records Evidence, an input
//! that the run depended on changes, and `verify_project` must then not
//! reuse that Evidence as current `PASS` (DES-208/212/372). The control case
//! (nothing changes) still reaches `PASS`.
//!
//! These tests change the process environment (`RUSTFLAGS`,
//! `CARGO_TARGET_DIR`), which every Cargo invocation in this process
//! inherits, so they run one at a time under `ENVIRONMENT_LOCK` in their own
//! test binary.

use std::{
    env, fs,
    path::{Path, PathBuf},
    process::Command,
    sync::{
        atomic::{AtomicU64, Ordering},
        Mutex,
    },
    time::{SystemTime, UNIX_EPOCH},
};

use vtest_exec::{run_tests, ExecutionResult, RunnableTest};
use vtest_model::{
    DerivesFrom, DocumentFile, DocumentId, NodeSource, RootNode, SentenceNode, TestId,
    VerificationState, VoId, VoRecord,
};
use vtest_scan::scan_project;
use vtest_store::{init_project, write_document_file, write_vo_record};
use vtest_verify::{verify_project, VerifyOutcome};

static ENVIRONMENT_LOCK: Mutex<()> = Mutex::new(());
static NEXT: AtomicU64 = AtomicU64::new(0);

fn registry() -> vtest_adapter_api::AdapterRegistry {
    vtest_cli::adapters::builtin_registry().expect("builtin registry must register cleanly")
}

fn temp_root(name: &str) -> PathBuf {
    let sequence = NEXT.fetch_add(1, Ordering::Relaxed);
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let root = env::temp_dir().join(format!(
        "vtest-cli-execstate-e2e-{name}-{}-{nanos}-{sequence}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(&root).expect("create fixture root");
    root
}

fn git(root: &Path, args: &[&str]) {
    let status = Command::new("git")
        .current_dir(root)
        .args([&["-c", "commit.gpgsign=false"], args].concat())
        .status()
        .expect("run git");
    assert!(status.success(), "git {args:?} failed");
}

/// The same outer-environment isolation as `evidence_e2e.rs`: the outer
/// `cargo llvm-cov` / user settings must not leak into the fixture build.
fn clear_outer_environment() {
    for variable in [
        "RUSTC_WRAPPER",
        "LLVM_PROFILE_FILE",
        "CARGO_LLVM_COV",
        "CARGO_LLVM_COV_TARGET_DIR",
        "RUSTFLAGS",
        "CARGO_ENCODED_RUSTFLAGS",
        "CARGO_INCREMENTAL",
        "CARGO_TARGET_DIR",
        "CARGO_BUILD_TARGET_DIR",
    ] {
        env::remove_var(variable);
    }
}

fn source(id: &str) -> NodeSource {
    NodeSource {
        doc: format!("{id}.md"),
        heading: "fixture".to_owned(),
        lines: [1, 1],
    }
}

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
        created: "2026-11-04T00:00:00Z".to_owned(),
        updated: "2026-11-04T00:00:00Z".to_owned(),
    }
}

struct Fixture<'a> {
    manifest_extra: &'a str,
    lib: &'a str,
    test_extra: &'a str,
    files: &'a [(&'a str, &'a str)],
}

const DEFAULT_LIB: &str = "pub fn double(x: i32) -> i32 { x * 2 }\n";

fn build_fixture(root: &Path, fixture: &Fixture<'_>) {
    fs::create_dir_all(root.join("src")).unwrap();
    fs::create_dir_all(root.join("tests")).unwrap();
    fs::write(
        root.join("Cargo.toml"),
        format!(
            "[package]\nname = \"vtest-e2e-fixture\"\nversion = \"0.1.0\"\nedition = \"2021\"\n{}\n\
             [workspace]\n",
            fixture.manifest_extra
        ),
    )
    .unwrap();
    fs::write(root.join("src/lib.rs"), fixture.lib).unwrap();
    fs::write(
        root.join("tests/registered.rs"),
        format!(
            "/// @vtest.id TEST-E2E-DOUBLE\n\
             /// @vtest.covers VO-E2E-DOUBLE\n\
             /// @vtest.target src/lib.rs::double\n\
             /// @vtest.intent doubles the input\n\
             #[test]\n\
             fn it_doubles() {{\n    {}assert_eq!(vtest_e2e_fixture::double(2), 4);\n}}\n",
            fixture.test_extra
        ),
    )
    .unwrap();
    for (path, contents) in fixture.files {
        let path = root.join(path);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, contents).unwrap();
    }
    let status = Command::new("cargo")
        .current_dir(root)
        .args(["generate-lockfile", "--offline"])
        .status()
        .expect("generate fixture lockfile");
    assert!(status.success());
    fs::write(root.join(".gitignore"), "target/\nout/\n").unwrap();
    git(root, &["init", "-q"]);
    git(root, &["config", "user.email", "vtest-fixture@example.com"]);
    git(root, &["config", "user.name", "vtest fixture"]);
    git(root, &["add", "."]);
    git(root, &["commit", "-q", "-m", "initial fixture commit"]);
    let layout = init_project(root, "vtest-e2e-fixture").expect("init .verify/ layout");
    write_document_file(&layout, "fixture", &document_file()).unwrap();
    write_vo_record(&layout, &vo("VO-E2E-DOUBLE")).unwrap();
}

fn run(root: &Path) -> ExecutionResult {
    let scan = scan_project(root, &registry()).expect("scan the fixture project");
    assert!(!scan.has_errors(), "scan errors: {:?}", scan.diagnostics);
    let entity = scan
        .tests
        .iter()
        .find(|test| test.id == TestId::new("TEST-E2E-DOUBLE"))
        .cloned()
        .expect("TEST-E2E-DOUBLE discovered");
    let target_locator = match entity.targets.first() {
        Some(vtest_model::TargetRef::Locator(locator)) => locator.clone(),
        other => panic!("expected a single Locator target, got {other:?}"),
    };
    let source_fn = scan
        .sources
        .iter()
        .find(|source| source.locator == target_locator)
        .expect("declared target resolved");
    let runnable = RunnableTest {
        entity,
        target_hashes: vec![source_fn.content_hash.clone()],
        target_locator: Some(target_locator),
    };
    let layout = vtest_store::VerifyLayout::new(root);
    run_tests(root, &layout, &[runnable], false, &registry()).expect("run_tests")
}

fn verify(root: &Path) -> VerifyOutcome {
    let scan = scan_project(root, &registry()).expect("re-scan");
    verify_project(root, &scan, None, None, &registry())
}

/// Whether plain `cargo test` of the fixture passes in the current
/// environment — the ground truth the stored Evidence claims.
fn cargo_test_passes(root: &Path) -> bool {
    Command::new("cargo")
        .current_dir(root)
        .args(["test", "--quiet", "--offline"])
        .output()
        .expect("run cargo test")
        .status
        .success()
}

/// Records PASS Evidence, then applies `change` and asserts that the actual
/// test now fails while `verify` does not report `PASS` from the old record.
fn assert_change_is_not_reused_as_pass(root: &Path, change: impl FnOnce(&Path)) {
    let result = run(root);
    assert!(!result.has_errors(), "run errors: {:?}", result.diagnostics);
    assert_eq!(result.evidence.len(), 1);
    assert_eq!(result.evidence[0].result, vtest_model::TestResult::Pass);
    let before = verify(root);
    change(root);
    assert!(
        !cargo_test_passes(root),
        "the change makes the real test fail; otherwise this fixture proves nothing"
    );
    let after = verify(root);
    assert_ne!(
        after.state,
        VerificationState::Pass,
        "stale Evidence reused as PASS (before: {:?}); outcome: {}",
        before.state,
        serde_json::to_string_pretty(&after).unwrap_or_default()
    );
    assert!(!after.ok);
}

/// @vtest.id TEST-E2E-EXEC-STATE-RUSTFLAGS-CHANGE-NOT-PASS
/// @vtest.covers VO-EXEC-STATE-REEVALUATION-REBUILDS-CURRENT-DTO, VO-EXEC-STATE-EFFECTIVE-CONFIG-BOUND
/// @vtest.target crates/vtest-adapter-rust/src/execution_state.rs::snapshot_in
/// @vtest.intent RUSTFLAGS付きで記録したPASS Evidenceを、RUSTFLAGSを外して実テストが失敗する環境でPASSとして再利用しないことを確認する
#[test]
fn evidence_recorded_under_rustflags_is_not_pass_without_them() {
    let _guard = ENVIRONMENT_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    clear_outer_environment();
    let root = temp_root("rustflags");
    build_fixture(
        &root,
        &Fixture {
            manifest_extra: "[lints.rust]\nunexpected_cfgs = \"allow\"\n",
            lib: DEFAULT_LIB,
            test_extra: "assert!(cfg!(review_flag));\n    ",
            files: &[],
        },
    );
    env::set_var("RUSTFLAGS", "--cfg review_flag");
    let result = run(&root);
    env::remove_var("RUSTFLAGS");
    assert!(!result.has_errors(), "run errors: {:?}", result.diagnostics);
    assert_eq!(result.evidence[0].result, vtest_model::TestResult::Pass);
    assert!(result.evidence[0].execution_state.complete);
    assert!(!cargo_test_passes(&root), "without the flag the test fails");
    let outcome = verify(&root);
    assert_ne!(outcome.state, VerificationState::Pass);
    assert!(!outcome.ok);
    fs::remove_dir_all(&root).ok();
}

/// @vtest.id TEST-E2E-EXEC-STATE-NESTED-INCLUDE-CHANGE-NOT-PASS
/// @vtest.covers VO-EXEC-STATE-REEVALUATION-REBUILDS-CURRENT-DTO, VO-EXEC-STATE-ESCAPE-RISK-FORCES-INCOMPLETE
/// @vtest.target crates/vtest-adapter-rust/src/execution_state.rs::escape_risk_in
/// @vtest.intent concat!内のinclude_str!で除外領域を読むcrateで、その領域の変更後にverifyがPASSを返さないことを確認する
#[test]
fn a_nested_include_of_an_excluded_area_is_not_pass_after_it_changes() {
    let _guard = ENVIRONMENT_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    clear_outer_environment();
    let root = temp_root("nested-include");
    build_fixture(
        &root,
        &Fixture {
            manifest_extra: "",
            lib: "pub fn double(x: i32) -> i32 {\n    \
                  x * concat!(include_str!(\"../target/factor\")).trim().parse::<i32>().unwrap()\n}\n",
            test_extra: "",
            files: &[("target/factor", "2")],
        },
    );
    assert_change_is_not_reused_as_pass(&root, |root| {
        fs::write(root.join("target/factor"), "3").unwrap();
    });
    fs::remove_dir_all(&root).ok();
}

/// @vtest.id TEST-E2E-EXEC-STATE-RENAMED-BUILD-SCRIPT-CHANGE-NOT-PASS
/// @vtest.covers VO-EXEC-STATE-REEVALUATION-REBUILDS-CURRENT-DTO, VO-EXEC-STATE-ESCAPE-RISK-FORCES-INCOMPLETE
/// @vtest.target crates/vtest-adapter-rust/src/execution_state.rs::escape_risk_in
/// @vtest.intent package.buildの別名build scriptが除外領域を読むcrateで、その領域の変更後にverifyがPASSを返さないことを確認する
#[test]
fn a_renamed_build_script_input_change_is_not_pass() {
    let _guard = ENVIRONMENT_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    clear_outer_environment();
    let root = temp_root("renamed-build-script");
    build_fixture(
        &root,
        &Fixture {
            manifest_extra: "build = \"builder.rs\"\n",
            lib: "pub fn double(x: i32) -> i32 { x * env!(\"FACTOR\").trim().parse::<i32>().unwrap() }\n",
            test_extra: "",
            files: &[
                ("target/factor", "2"),
                (
                    "builder.rs",
                    "fn main() {\n    \
                     println!(\"cargo:rerun-if-changed=target/factor\");\n    \
                     println!(\"cargo:rustc-env=FACTOR={}\", \
                     std::fs::read_to_string(\"target/factor\").unwrap());\n}\n",
                ),
            ],
        },
    );
    assert_change_is_not_reused_as_pass(&root, |root| {
        fs::write(root.join("target/factor"), "3").unwrap();
    });
    fs::remove_dir_all(&root).ok();
}

/// @vtest.id TEST-E2E-EXEC-STATE-RELOCATED-TARGET-DIR-RECORDS-EVIDENCE
/// @vtest.covers VO-EXEC-STATE-NESTED-TARGET-DIR-NOT-EXCLUDED, VO-EXEC-STATE-PREPOST-STABILITY
/// @vtest.target crates/vtest-adapter-rust/src/execution_state.rs::ExecutionContext::resolve
/// @vtest.intent CARGO_TARGET_DIRをroot内の別名out/にしても生成物を入力扱いせず、E-EXEC-004なしでEvidenceを記録しverifyがPASSになることを確認する
#[test]
fn a_relocated_target_directory_inside_the_root_still_records_evidence() {
    let _guard = ENVIRONMENT_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    clear_outer_environment();
    let root = temp_root("relocated-target-dir");
    build_fixture(
        &root,
        &Fixture {
            manifest_extra: "",
            lib: DEFAULT_LIB,
            test_extra: "",
            files: &[],
        },
    );
    env::set_var("CARGO_TARGET_DIR", root.join("out"));
    let result = run(&root);
    let outcome = verify(&root);
    env::remove_var("CARGO_TARGET_DIR");
    assert!(
        !result.has_errors(),
        "generated output must not be an input change: {:?}",
        result.diagnostics
    );
    assert_eq!(result.evidence.len(), 1);
    assert!(result.evidence[0].execution_state.complete);
    assert!(root.join("out").is_dir(), "Cargo built into out/");
    assert_eq!(
        outcome.state,
        VerificationState::Pass,
        "{}",
        serde_json::to_string_pretty(&outcome).unwrap_or_default()
    );
    fs::remove_dir_all(&root).ok();
}

/// The control case for the regressions above: with no change between the
/// run and the re-evaluation, the recorded Evidence is still current `PASS`.
/// @vtest.id TEST-E2E-EXEC-STATE-UNCHANGED-CONTROL-PASS
/// @vtest.covers VO-EXEC-STATE-REEVALUATION-REBUILDS-CURRENT-DTO
/// @vtest.target crates/vtest-adapter-rust/src/execution_state.rs::snapshot_in
/// @vtest.intent 環境・入力を変えない対照ではverifyが引き続きPASSになることを確認する
#[test]
fn an_unchanged_environment_still_reaches_pass() {
    let _guard = ENVIRONMENT_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    clear_outer_environment();
    let root = temp_root("control");
    build_fixture(
        &root,
        &Fixture {
            manifest_extra: "",
            lib: DEFAULT_LIB,
            test_extra: "",
            files: &[],
        },
    );
    let result = run(&root);
    assert!(!result.has_errors(), "run errors: {:?}", result.diagnostics);
    let outcome = verify(&root);
    assert_eq!(
        outcome.state,
        VerificationState::Pass,
        "{}",
        serde_json::to_string_pretty(&outcome).unwrap_or_default()
    );
    fs::remove_dir_all(&root).ok();
}
