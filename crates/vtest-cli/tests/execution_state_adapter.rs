//! Adapter collection plus core hashing regression coverage for Issue #74.

use std::{
    fs,
    path::{Path, PathBuf},
};
use vtest_adapter_rust::execution_state::{escape_risk, snapshot};
use vtest_model::{AdapterId, ExecutionState};
struct ExecutionStateInputs<'a> {
    adapter: &'a AdapterId,
    schema: &'a str,
    head_commit: Option<&'a str>,
    runner_kind: &'a str,
    invocation: &'a str,
}
fn reconstruct_execution_state(root: &Path, inputs: ExecutionStateInputs<'_>) -> ExecutionState {
    let _ = inputs.schema;
    let draft = snapshot(
        root,
        inputs.head_commit,
        inputs.runner_kind,
        serde_json::json!(inputs.invocation),
    );
    vtest_store::execution_state::materialize_execution_state(inputs.adapter, &draft)
}

fn temp_dir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("vtest-adapter-execstate-{name}"));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).expect("create temp dir");
    dir
}

fn inputs<'a>(adapter: &'a AdapterId, head: &'a str) -> ExecutionStateInputs<'a> {
    ExecutionStateInputs {
        adapter,
        schema: "test-execution-state-v1",
        head_commit: Some(head),
        runner_kind: "cargo-test",
        invocation: "cargo test -p fixture -- --exact fixture::it_works",
    }
}

/// @vtest.id TEST-EXEC-STATE-COMPLETE-RECONSTRUCTION
/// @vtest.covers VO-EXEC-STATE-COMPLETE-WHEN-INPUTS-RESOLVABLE
/// @vtest.target crates/vtest-adapter-rust/src/execution_state.rs::snapshot
/// @vtest.intent Verifies a repository with no escape risk and a readable manifest reconstructs to complete:true with a hash present.
#[test]
fn a_repository_with_no_escape_risk_and_a_readable_manifest_reconstructs_complete() {
    let root = temp_dir("complete");
    fs::write(root.join("Cargo.toml"), "[package]\nname = \"fixture\"\n").expect("write");
    fs::create_dir_all(root.join(".git")).expect("mkdir .git");
    fs::write(root.join(".git").join("HEAD"), "ref: refs/heads/main\n").expect("write");
    let adapter = AdapterId::new("rust-cargo");
    let state = reconstruct_execution_state(&root, inputs(&adapter, "deadbeef"));
    assert!(state.complete, "expected a complete reconstruction");
    assert!(state.hash.is_some());
}

/// @vtest.id TEST-EXEC-STATE-DETERMINISTIC-HASH
/// @vtest.covers VO-EXEC-STATE-HASH-DETERMINISTIC-FOR-IDENTICAL-INPUTS
/// @vtest.target crates/vtest-adapter-rust/src/execution_state.rs::snapshot
/// @vtest.intent Verifies two reconstructions of the identical environment produce the identical hash.
#[test]
fn a_repository_reconstructs_the_identical_hash_across_two_calls() {
    let root = temp_dir("deterministic");
    fs::write(root.join("Cargo.toml"), "[package]\nname = \"fixture\"\n").expect("write");
    let adapter = AdapterId::new("rust-cargo");
    let first = reconstruct_execution_state(&root, inputs(&adapter, "deadbeef"));
    let second = reconstruct_execution_state(&root, inputs(&adapter, "deadbeef"));
    assert_eq!(first.hash, second.hash);
    assert!(first.complete && second.complete);
}

/// @vtest.id TEST-EXEC-STATE-MANIFEST-CHANGE-CHANGES-HASH
/// @vtest.covers VO-EXEC-STATE-MANIFEST-BYTES-BOUND-TO-HASH
/// @vtest.target crates/vtest-adapter-rust/src/execution_state.rs::snapshot
/// @vtest.intent Verifies rewriting a manifest file's bytes changes the reconstructed hash.
#[test]
fn changing_a_manifest_file_changes_the_hash() {
    let root = temp_dir("file-change");
    fs::write(root.join("Cargo.toml"), "[package]\nname = \"fixture\"\n").expect("write");
    let adapter = AdapterId::new("rust-cargo");
    let before = reconstruct_execution_state(&root, inputs(&adapter, "deadbeef"));
    fs::write(
        root.join("Cargo.toml"),
        "[package]\nname = \"fixture-renamed\"\n",
    )
    .expect("rewrite");
    let after = reconstruct_execution_state(&root, inputs(&adapter, "deadbeef"));
    assert_ne!(before.hash, after.hash);
}

/// DES-211 excludes the Cargo *build output* directory at the
/// workspace root, not any directory that happens to share its name —
/// a legitimate source directory nested deeper in the tree and named
/// `target` (e.g. `src/target/mod.rs`) must still be bound into the
/// manifest. A prior version of the exclusion matched by bare
/// directory name at any depth and would have silently dropped it.
/// @vtest.id TEST-EXEC-STATE-NESTED-TARGET-DIR-NOT-EXCLUDED
/// @vtest.covers VO-EXEC-STATE-NESTED-TARGET-DIR-NOT-EXCLUDED
/// @vtest.target crates/vtest-adapter-rust/src/execution_state.rs::walk
/// @vtest.intent Verifies a nested directory literally named `target` (e.g. `src/target`) is not excluded from the manifest.
#[test]
fn a_nested_directory_literally_named_target_is_not_excluded() {
    let root = temp_dir("nested-target-dir");
    fs::write(root.join("Cargo.toml"), "[package]\nname = \"fixture\"\n").expect("write");
    fs::create_dir_all(root.join("src").join("target")).expect("mkdir src/target");
    fs::write(
        root.join("src").join("target").join("mod.rs"),
        "pub fn compile_target() {}\n",
    )
    .expect("write src/target/mod.rs");
    let adapter = AdapterId::new("rust-cargo");
    let with_file = reconstruct_execution_state(&root, inputs(&adapter, "deadbeef"));
    fs::write(
        root.join("src").join("target").join("mod.rs"),
        "pub fn compile_target() { /* changed */ }\n",
    )
    .expect("rewrite src/target/mod.rs");
    let after_change = reconstruct_execution_state(&root, inputs(&adapter, "deadbeef"));
    assert_ne!(
        with_file.hash, after_change.hash,
        "a change inside a nested `target`-named directory must change the manifest hash \
         (it must not have been excluded)"
    );
}

/// DS-819 relies on this via the Test subject hash separately, but
/// DES-097 additionally binds HEAD revision into the Execution State
/// subject itself; a changed revision must change this hash too.
/// @vtest.id TEST-EXEC-STATE-HEAD-REVISION-CHANGE-CHANGES-HASH
/// @vtest.covers VO-EXEC-STATE-HEAD-REVISION-BOUND-TO-HASH
/// @vtest.target crates/vtest-adapter-rust/src/execution_state.rs::snapshot
/// @vtest.intent Verifies changing the HEAD revision input changes the reconstructed hash.
#[test]
fn changing_head_revision_changes_the_hash() {
    let root = temp_dir("revision-change");
    fs::write(root.join("Cargo.toml"), "[package]\nname = \"fixture\"\n").expect("write");
    let adapter = AdapterId::new("rust-cargo");
    let before = reconstruct_execution_state(&root, inputs(&adapter, "deadbeef"));
    let after = reconstruct_execution_state(&root, inputs(&adapter, "cafef00d"));
    assert_ne!(before.hash, after.hash);
}

/// @vtest.id TEST-EXEC-STATE-ABSENT-HEAD-COMMIT-INCOMPLETE
/// @vtest.covers VO-EXEC-STATE-ABSENT-HEAD-COMMIT-INCOMPLETE
/// @vtest.target crates/vtest-adapter-rust/src/execution_state.rs::snapshot
/// @vtest.intent Verifies an absent HEAD commit input yields complete:false and hash:None rather than a fabricated hash.
#[test]
fn an_absent_head_commit_is_incomplete_not_a_fabricated_hash() {
    let root = temp_dir("no-head");
    fs::write(root.join("Cargo.toml"), "[package]\nname = \"fixture\"\n").expect("write");
    let adapter = AdapterId::new("rust-cargo");
    let state = reconstruct_execution_state(
        &root,
        ExecutionStateInputs {
            adapter: &adapter,
            schema: "test-execution-state-v1",
            head_commit: None,
            runner_kind: "cargo-test",
            invocation: "cargo test",
        },
    );
    assert!(!state.complete);
    assert!(state.hash.is_none());
}

/// @vtest.id TEST-EXEC-STATE-BUILD-SCRIPT-FORCES-INCOMPLETE
/// @vtest.covers VO-EXEC-STATE-ESCAPE-RISK-FORCES-INCOMPLETE
/// @vtest.target crates/vtest-adapter-rust/src/execution_state.rs::escape_risk
/// @vtest.intent Verifies a `build.rs` in the tree makes the escape risk unprovable and forces an incomplete reconstruction.
/// DES-212: a `build.rs` in the tree makes the escape risk
/// unprovable, so the reconstruction must not report `complete: true`.
#[test]
fn a_build_script_forces_an_incomplete_reconstruction() {
    let root = temp_dir("build-script");
    fs::write(root.join("Cargo.toml"), "[package]\nname = \"fixture\"\n").expect("write");
    fs::write(root.join("build.rs"), "fn main() {}\n").expect("write build.rs");
    assert!(escape_risk(&root).is_some());
    let adapter = AdapterId::new("rust-cargo");
    let state = reconstruct_execution_state(&root, inputs(&adapter, "deadbeef"));
    assert!(!state.complete);
}

/// @vtest.id TEST-EXEC-STATE-IN-SCOPE-INCLUDE-NOT-INCOMPLETE
/// @vtest.covers VO-EXEC-STATE-IN-SCOPE-INCLUDE-NOT-INCOMPLETE
/// @vtest.target crates/vtest-adapter-rust/src/execution_state.rs::escape_risk
/// @vtest.intent Verifies an `include_str!` literal resolving inside the manifest's included area does not force incompleteness.
/// A literal `include_str!` that stays inside the manifest's own
/// included area (not an excluded directory) does not itself force
/// incompleteness — this is the shape this repository's own
/// `include_str!("../tests/fixtures/...")` calls take.
#[test]
fn an_in_scope_include_str_literal_does_not_force_incompleteness() {
    let root = temp_dir("include-in-scope");
    fs::create_dir_all(root.join("src")).expect("mkdir src");
    fs::create_dir_all(root.join("tests").join("fixtures")).expect("mkdir fixtures");
    fs::write(root.join("Cargo.toml"), "[package]\nname = \"fixture\"\n").expect("write");
    fs::write(
        root.join("src").join("lib.rs"),
        "fn f() { let _ = include_str!(\"../tests/fixtures/sample.json\"); }\n",
    )
    .expect("write lib.rs");
    fs::write(
        root.join("tests").join("fixtures").join("sample.json"),
        "{}",
    )
    .expect("write");
    assert!(escape_risk(&root).is_none());
}

/// @vtest.id TEST-EXEC-STATE-INCLUDE-SPELLING-IN-RUST-LITERAL-NOT-RISK
/// @vtest.covers VO-EXEC-STATE-ESCAPE-RISK-FORCES-INCOMPLETE
/// @vtest.target crates/vtest-adapter-rust/src/execution_state.rs::escape_risk
/// @vtest.intent Rust文字列literal内のinclude_str!表記を実マクロ呼出しと誤認しないことを確認する
#[test]
fn include_spelling_inside_a_string_literal_is_not_escape_risk() {
    let root = temp_dir("include-spelling-literal");
    fs::create_dir_all(root.join("src")).expect("mkdir src");
    fs::write(root.join("Cargo.toml"), "[package]\nname = \"fixture\"\n").expect("write");
    fs::write(
        root.join("src/lib.rs"),
        r##"fn f() { let _ = "include_str!(concat!(\"../outside\"))"; }"##,
    )
    .expect("write lib.rs");
    assert!(escape_risk(&root).is_none());
}

/// @vtest.id TEST-EXEC-STATE-INCLUDE-SPELLING-IN-COMMENT-NOT-RISK
/// @vtest.covers VO-EXEC-STATE-ESCAPE-RISK-FORCES-INCOMPLETE
/// @vtest.target crates/vtest-adapter-rust/src/execution_state.rs::escape_risk
/// @vtest.intent コメント内のinclude_str!表記を実マクロ呼出しと誤認しないことを確認する
#[test]
fn include_spelling_inside_a_comment_is_not_escape_risk() {
    let root = temp_dir("include-spelling-comment");
    fs::create_dir_all(root.join("src")).expect("mkdir src");
    fs::write(root.join("Cargo.toml"), "[package]\nname = \"fixture\"\n").expect("write");
    fs::write(
        root.join("src/lib.rs"),
        "// include_str!(concat!(\"../outside\"))\nfn f() {}\n",
    )
    .expect("write lib.rs");
    assert!(escape_risk(&root).is_none());
}

/// @vtest.id TEST-EXEC-STATE-PARSE-ERROR-FORCES-ESCAPE-RISK
/// @vtest.covers VO-EXEC-STATE-ESCAPE-RISK-FORCES-INCOMPLETE
/// @vtest.target crates/vtest-adapter-rust/src/execution_state.rs::escape_risk
/// @vtest.intent 構文解析不能なRust sourceが理由付きescape_riskになることを確認する
#[test]
fn an_unparseable_rust_file_is_escape_risk_with_a_reason() {
    let root = temp_dir("unparseable-rust");
    fs::create_dir_all(root.join("src")).expect("mkdir src");
    fs::write(root.join("Cargo.toml"), "[package]\nname = \"fixture\"\n").expect("write");
    fs::write(root.join("src/lib.rs"), "fn broken( {\n").expect("write lib.rs");
    let risk = escape_risk(&root).expect("parse error is escape risk");
    assert!(risk.contains("src\\lib.rs") || risk.contains("src/lib.rs"));
    assert!(risk.contains("cannot be parsed as Rust"));
}

/// @vtest.id TEST-EXEC-STATE-OUT-OF-SCOPE-INCLUDE-FORCES-INCOMPLETE
/// @vtest.covers VO-EXEC-STATE-ESCAPE-RISK-FORCES-INCOMPLETE
/// @vtest.target crates/vtest-adapter-rust/src/execution_state.rs::escape_risk
/// @vtest.intent Verifies an `include_str!` literal resolving outside the manifest's included area forces incompleteness.
/// An `include_str!` that resolves outside the manifest's included area
/// (here, above the workspace root entirely) cannot be ruled out and
/// must force incompleteness (DES-212).
#[test]
fn an_out_of_scope_include_str_literal_forces_incompleteness() {
    let root = temp_dir("include-out-of-scope").join("nested");
    fs::create_dir_all(root.join("src")).expect("mkdir src");
    fs::write(root.join("Cargo.toml"), "[package]\nname = \"fixture\"\n").expect("write");
    fs::write(
        root.join("src").join("lib.rs"),
        "fn f() { let _ = include_str!(\"../../outside.json\"); }\n",
    )
    .expect("write lib.rs");
    assert!(escape_risk(&root).is_some());
}

/// DS-212's escape check must not stop after the first `include_str!`
/// occurrence in a file — a *second* call whose argument resolves
/// outside the manifest's included area is exactly as much a risk as a
/// first would be. A prior version of this check only inspected the
/// first occurrence (`find_macro_literal_argument`, singular) and
/// would have missed this.
/// @vtest.id TEST-EXEC-STATE-SECOND-INCLUDE-OCCURRENCE-FORCES-INCOMPLETE
/// @vtest.covers VO-EXEC-STATE-ESCAPE-RISK-FORCES-INCOMPLETE
/// @vtest.target crates/vtest-adapter-rust/src/execution_state.rs::escape_risk
/// @vtest.intent Verifies a second `include_str!` occurrence in a file resolving out of scope forces incompleteness, not only the first occurrence.
#[test]
fn a_second_include_str_occurrence_pointing_out_of_scope_forces_incompleteness() {
    let root = temp_dir("include-second-out-of-scope").join("nested");
    fs::create_dir_all(root.join("src")).expect("mkdir src");
    fs::create_dir_all(root.join("tests").join("fixtures")).expect("mkdir fixtures");
    fs::write(root.join("Cargo.toml"), "[package]\nname = \"fixture\"\n").expect("write");
    fs::write(
        root.join("tests").join("fixtures").join("in-scope.json"),
        "{}",
    )
    .expect("write in-scope fixture");
    fs::write(
        root.join("src").join("lib.rs"),
        "fn f() {\n    \
             let _ = include_str!(\"../tests/fixtures/in-scope.json\");\n    \
             let _ = include_str!(\"../../outside.json\");\n\
         }\n",
    )
    .expect("write lib.rs");
    assert!(escape_risk(&root).is_some());
}

/// @vtest.id TEST-EXEC-STATE-NON-LITERAL-INCLUDE-ARG-FORCES-INCOMPLETE
/// @vtest.covers VO-EXEC-STATE-ESCAPE-RISK-FORCES-INCOMPLETE
/// @vtest.target crates/vtest-adapter-rust/src/execution_state.rs::escape_risk
/// @vtest.intent Verifies a non-literal `include_str!` argument, which cannot be statically resolved, forces incompleteness.
#[test]
fn a_non_literal_include_argument_forces_incompleteness() {
    let root = temp_dir("include-dynamic");
    fs::create_dir_all(root.join("src")).expect("mkdir src");
    fs::write(root.join("Cargo.toml"), "[package]\nname = \"fixture\"\n").expect("write");
    fs::write(
        root.join("src").join("lib.rs"),
        "fn f() { let _ = include_str!(concat!(env!(\"OUT_DIR\"), \"/x.rs\")); }\n",
    )
    .expect("write lib.rs");
    assert!(escape_risk(&root).is_some());
}
/// DES-210/212: an uncollected external dependency is incomplete.
/// @vtest.id TEST-EXEC-STATE-EXTERNAL-DEPENDENCY-INCOMPLETE
/// @vtest.covers VO-EXEC-STATE-LOCAL-INPUT-COMPLETENESS
/// @vtest.target crates/vtest-adapter-rust/src/execution_state.rs::snapshot
/// @vtest.intent external path dependency is incomplete
#[test]
fn external_path_dependency_is_incomplete() {
    let root = temp_dir("external-dependency");
    fs::write(
        root.join("Cargo.toml"),
        "[package]\nname = \"fixture\"\n[dependencies]\nother = { path = \"../outside\" }\n",
    )
    .unwrap();
    let state = snapshot(&root, Some("head"), "cargo-test", serde_json::Value::Null);
    assert!(!state.complete);
}

/// DES-210: local dependency bytes within the workspace stay bound.
/// @vtest.id TEST-EXEC-STATE-LOCAL-DEPENDENCY-INPUTS
/// @vtest.covers VO-EXEC-STATE-LOCAL-INPUT-COMPLETENESS
/// @vtest.target crates/vtest-adapter-rust/src/execution_state.rs::snapshot
/// @vtest.intent local dependency bytes are collected
#[test]
fn local_dependency_bytes_are_collected() {
    let root = temp_dir("local-dependency");
    fs::create_dir_all(root.join("dep/src")).unwrap();
    fs::write(
        root.join("Cargo.toml"),
        "[package]\nname = \"fixture\"\n[dependencies]\nother = { path = \"dep\" }\n",
    )
    .unwrap();
    fs::write(root.join("dep/src/lib.rs"), "pub fn helper() {}\n").unwrap();
    let before = snapshot(&root, Some("head"), "cargo-test", serde_json::Value::Null);
    fs::write(
        root.join("dep/src/lib.rs"),
        "pub fn helper() { panic!() }\n",
    )
    .unwrap();
    let after = snapshot(&root, Some("head"), "cargo-test", serde_json::Value::Null);
    assert!(before.complete && after.complete);
    assert_ne!(before.inputs, after.inputs);
}

/// DES-370: command display paths and per-run coverage output filenames are
/// not execution identity; selector and runner mode remain bound.
/// @vtest.id TEST-EXEC-STATE-CANONICAL-INVOCATION
/// @vtest.covers VO-EXEC-STATE-CANONICAL-INVOCATION
/// @vtest.target crates/vtest-adapter-rust/src/lib.rs::RustCargoTestRunner::execution_state
/// @vtest.intent canonical invocation is relocation independent and binds selector and mode
#[test]
fn canonical_invocation_is_relocation_independent_and_binds_selector_and_mode() {
    use vtest_adapter_api::TestRunnerAdapter;
    use vtest_model::{ExecutionDescriptor, TestSuite};
    let roots = [temp_dir("projection-a"), temp_dir("projection-b")];
    for root in &roots {
        fs::write(root.join("Cargo.toml"), "[package]\nname = \"fixture\"\n").unwrap();
    }
    let mut execution = ExecutionDescriptor {
        adapter: AdapterId::new("rust-cargo"),
        project: Some("fixture".into()),
        suite: Some(TestSuite {
            kind: "lib".into(),
            name: None,
        }),
        selector: "test_one".into(),
    };
    let runner = vtest_adapter_rust::RustCargoTestRunner::new();
    let a = runner
        .execution_state(&roots[0], &execution, Some("head"), "cargo-llvm-cov")
        .unwrap();
    let b = runner
        .execution_state(&roots[1], &execution, Some("head"), "cargo-llvm-cov")
        .unwrap();
    assert_eq!(a, b);
    assert!(!a
        .invocation
        .to_string()
        .contains(&roots[0].to_string_lossy().to_string()));
    execution.selector = "test_two".into();
    let c = runner
        .execution_state(&roots[0], &execution, Some("head"), "cargo-llvm-cov")
        .unwrap();
    assert_ne!(a.invocation, c.invocation);
    let d = runner
        .execution_state(&roots[0], &execution, Some("head"), "cargo-test")
        .unwrap();
    assert_ne!(c.invocation, d.invocation);
}
