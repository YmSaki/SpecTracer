//! Adapter collection plus core hashing regression coverage for Issue #74.

use std::{
    fs,
    path::{Path, PathBuf},
};
use vtest_adapter_rust::execution_state::{escape_risk, snapshot, snapshot_in, Environment};
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

// ---------------------------------------------------------------------------
// Issue #117: the execution-affecting environment and configuration.
// ---------------------------------------------------------------------------

fn cargo_root(name: &str) -> PathBuf {
    let root = temp_dir(name);
    fs::write(root.join("Cargo.toml"), "[package]\nname = \"fixture\"\n").expect("write");
    root
}

/// A fixed environment; HOME points into the fixture so no developer
/// `~/.cargo/config.toml` leaks into the projection.
fn fixed_environment(root: &Path, extra: &[(&str, &str)]) -> Environment {
    let mut vars: Vec<(String, String)> = vec![
        ("PATH".to_owned(), std::env::var("PATH").unwrap_or_default()),
        ("HOME".to_owned(), root.join("home").display().to_string()),
    ];
    for key in ["RUSTUP_HOME", "RUSTUP_TOOLCHAIN"] {
        if let Ok(value) = std::env::var(key) {
            vars.push((key.to_owned(), value));
        }
    }
    if let Ok(home) = std::env::var("CARGO_HOME") {
        // rustup proxies locate toolchains through CARGO_HOME/bin; keep the
        // real one so `rustc -Vv` resolves, unless a test overrides it.
        vars.push(("CARGO_HOME".to_owned(), home));
    }
    for (key, value) in extra {
        vars.retain(|(k, _)| k != key);
        vars.push(((*key).to_owned(), (*value).to_owned()));
    }
    Environment::from_vars(vars)
}

fn snapshot_with(root: &Path, extra: &[(&str, &str)]) -> vtest_model::ExecutionStateDraft {
    snapshot_in(
        root,
        Some("head"),
        "cargo-test",
        serde_json::Value::Null,
        fixed_environment(root, extra),
    )
}

fn hash_of(draft: &vtest_model::ExecutionStateDraft) -> Option<String> {
    let adapter = AdapterId::new("rust-cargo");
    vtest_store::execution_state::materialize_execution_state(&adapter, draft)
        .hash
        .map(|hash| hash.as_str().to_owned())
}

/// @vtest.id TEST-EXEC-STATE-RUSTFLAGS-BOUND
/// @vtest.covers VO-EXEC-STATE-EFFECTIVE-CONFIG-BOUND
/// @vtest.target crates/vtest-adapter-rust/src/execution_state.rs::snapshot_in
/// @vtest.intent RUSTFLAGS・CARGO_ENCODED_RUSTFLAGS・RUSTC_WRAPPER などCargo/rustcの設定環境変数の変更がExecution State hashを変えることを確認する
#[test]
fn execution_affecting_environment_variables_change_the_hash() {
    let root = cargo_root("env-rustflags");
    let baseline = snapshot_with(&root, &[]);
    assert!(baseline.complete, "a plain fixture is complete");
    for (name, value) in [
        ("RUSTFLAGS", "--cfg review_flag"),
        ("CARGO_ENCODED_RUSTFLAGS", "--cfg\u{1f}review_flag"),
        ("RUSTC_WRAPPER", "sccache"),
        ("CARGO_PROFILE_DEV_OPT_LEVEL", "3"),
        ("RUST_TEST_THREADS", "1"),
    ] {
        let changed = snapshot_with(&root, &[(name, value)]);
        assert!(changed.complete, "{name} is bound, not a reason to refuse");
        assert_ne!(
            hash_of(&baseline),
            hash_of(&changed),
            "{name} changes what Cargo builds or runs and must be bound"
        );
    }
}

/// @vtest.id TEST-EXEC-STATE-UNRELATED-ENVIRONMENT-NOT-BOUND
/// @vtest.covers VO-EXEC-STATE-EFFECTIVE-CONFIG-BOUND
/// @vtest.target crates/vtest-adapter-rust/src/execution_state.rs::snapshot_in
/// @vtest.intent 無関係な環境変数、cargoが上書きするCARGO_PKG_*、資格情報はhashを変えないことを確認する
#[test]
fn unrelated_overwritten_and_credential_variables_are_not_bound() {
    let root = cargo_root("env-unbound");
    let baseline = snapshot_with(&root, &[]);
    for (name, value) in [
        ("TERM", "dumb"),
        ("CARGO_PKG_NAME", "outer"),
        ("CARGO_MANIFEST_DIR", "/elsewhere"),
        ("CARGO_REGISTRY_TOKEN", "secret"),
    ] {
        let changed = snapshot_with(&root, &[(name, value)]);
        assert_eq!(hash_of(&baseline), hash_of(&changed), "{name} is not bound");
    }
}

/// @vtest.id TEST-EXEC-STATE-OUT-OF-ROOT-CARGO-CONFIG-BOUND
/// @vtest.covers VO-EXEC-STATE-EFFECTIVE-CONFIG-BOUND
/// @vtest.target crates/vtest-adapter-rust/src/execution_state.rs::snapshot_in
/// @vtest.intent root外の祖先ディレクトリとCARGO_HOMEの.cargo設定の変更がhashを変えることを確認する
#[test]
fn cargo_configuration_outside_the_root_is_bound() {
    let parent = temp_dir("env-ancestor-config");
    let root = parent.join("project");
    fs::create_dir_all(&root).unwrap();
    fs::write(root.join("Cargo.toml"), "[package]\nname = \"fixture\"\n").unwrap();
    let cargo_home = parent.join("cargo-home");
    fs::create_dir_all(&cargo_home).unwrap();
    let home = cargo_home.display().to_string();
    let env: &[(&str, &str)] = &[("CARGO_HOME", &home)];
    // Point toolchain lookup at the real CARGO_HOME only through PATH;
    // `rustc` stays resolvable through the rustup proxy on PATH.
    let baseline = snapshot_with(&root, env);
    fs::create_dir_all(parent.join(".cargo")).unwrap();
    fs::write(
        parent.join(".cargo/config.toml"),
        "[build]\nrustflags = [\"--cfg\", \"review_flag\"]\n",
    )
    .unwrap();
    let with_ancestor = snapshot_with(&root, env);
    assert_ne!(hash_of(&baseline), hash_of(&with_ancestor));
    fs::write(cargo_home.join("config.toml"), "[term]\nverbose = true\n").unwrap();
    let with_home = snapshot_with(&root, env);
    assert_ne!(hash_of(&with_ancestor), hash_of(&with_home));
}

/// @vtest.id TEST-EXEC-STATE-RUNNER-ENVIRONMENT-OVERLAY
/// @vtest.covers VO-EXEC-STATE-RUNNER-ENVIRONMENT-MATCHES
/// @vtest.target crates/vtest-adapter-rust/src/execution_state.rs::Environment::with_overlay
/// @vtest.intent runner commandが継承する環境（プロセス環境にcommand.envを重ねたもの）をsnapshotが使うことを確認する
#[test]
fn the_snapshot_environment_is_the_inherited_environment_with_the_command_overlay() {
    let inherited = Environment::from_vars([("RUSTFLAGS", "-Dwarnings"), ("PATH", "/bin")]);
    let overlay = std::collections::BTreeMap::from([(
        "RUSTFLAGS".to_owned(),
        "--cfg review_flag".to_owned(),
    )]);
    let effective = inherited.with_overlay(&overlay);
    assert_eq!(
        effective.vars.get("RUSTFLAGS").map(String::as_str),
        Some("--cfg review_flag")
    );
    assert_eq!(effective.vars.get("PATH").map(String::as_str), Some("/bin"));
}

// ---------------------------------------------------------------------------
// Issue #118: include-family calls nested in unparsed macro tokens.
// ---------------------------------------------------------------------------

fn source_root(name: &str, lib: &str) -> PathBuf {
    let root = cargo_root(name);
    fs::create_dir_all(root.join("src")).unwrap();
    fs::create_dir_all(root.join("tests/fixtures")).unwrap();
    fs::write(root.join("tests/fixtures/in-scope.txt"), "2").unwrap();
    fs::write(root.join("src/lib.rs"), lib).unwrap();
    root
}

/// @vtest.id TEST-EXEC-STATE-NESTED-MACRO-INCLUDE-FORCES-INCOMPLETE
/// @vtest.covers VO-EXEC-STATE-ESCAPE-RISK-FORCES-INCOMPLETE
/// @vtest.target crates/vtest-adapter-rust/src/execution_state.rs::escape_risk_in
/// @vtest.intent concat!・assert_eq!・cfg_attr・macro_rules!の未解析token内から除外領域を読むinclude系呼出しと#[path]をescape riskとすることを確認する
#[test]
fn include_calls_nested_in_macro_tokens_that_reach_an_excluded_area_force_incompleteness() {
    for (name, lib) in [
        (
            "nested-concat",
            "pub fn f() -> &'static str { concat!(include_str!(\"../target/factor\")) }\n",
        ),
        (
            "nested-assert",
            "pub fn f() { assert_eq!(include_str!(\"../target/factor\"), \"2\"); }\n",
        ),
        (
            "nested-twice",
            "pub fn f() -> String { format!(\"{}\", concat!(include_str!(\"../../outside\"))) }\n",
        ),
        (
            "nested-cfg-attr",
            "#![cfg_attr(all(), doc = include_str!(\"../target/README.md\"))]\npub fn f() {}\n",
        ),
        (
            "nested-macro-rules-literal",
            "macro_rules! m { () => { include_str!(\"../target/factor\") }; }\npub fn f() -> &'static str { m!() }\n",
        ),
        (
            "nested-macro-rules-argument",
            "macro_rules! m { ($p:literal) => { include_str!($p) }; }\npub fn f() -> &'static str { m!(\"../target/factor\") }\n",
        ),
        (
            "nested-metavariable-call",
            "macro_rules! m { ($m:ident) => { $m!(\"../target/factor\") }; }\npub fn f() -> &'static str { m!(include_str) }\n",
        ),
        (
            "path-attribute",
            "#[path = \"../target/generated.rs\"]\nmod generated;\n",
        ),
    ] {
        let root = source_root(name, lib);
        let risk = escape_risk(&root);
        assert!(risk.is_some(), "{name}: expected an escape risk for {lib}");
        assert!(!snapshot_with(&root, &[]).complete, "{name}: incomplete");
    }
}

/// @vtest.id TEST-EXEC-STATE-CFG-ATTR-AND-MACRO-PATH-FORCES-INCOMPLETE
/// @vtest.covers VO-EXEC-STATE-ESCAPE-RISK-FORCES-INCOMPLETE
/// @vtest.target crates/vtest-adapter-rust/src/execution_state.rs::escape_risk_in
/// @vtest.intent cfg_attr内（入れ子・2番目以降の属性を含む）とmacro_rules!内の#[path]が除外領域を指すとき、およびmacro_rules!のmetavariableでpathを渡すときにescape riskとし、snapshotを不完全にすることを確認する
#[test]
fn path_attributes_in_cfg_attr_or_macro_tokens_force_incompleteness() {
    for (name, lib) in [
        (
            "cfg-attr-path",
            "#[cfg_attr(unix, path = \"../target/hidden.rs\")]\nmod hidden;\n",
        ),
        (
            "cfg-attr-path-not-first",
            "#[cfg_attr(all(), allow(dead_code), path = \"../target/hidden.rs\")]\nmod hidden;\n",
        ),
        (
            "nested-cfg-attr-path",
            "#[cfg_attr(all(), cfg_attr(unix, path = \"../target/hidden.rs\"))]\nmod hidden;\n",
        ),
        (
            "cfg-attr-path-outside-root",
            "#[cfg_attr(windows, path = \"../../outside.rs\")]\nmod hidden;\n",
        ),
        (
            "macro-rules-path",
            "macro_rules! m { () => { #[path = \"../target/hidden.rs\"] mod hidden; }; }\nm!();\n",
        ),
        (
            "macro-rules-cfg-attr-path",
            "macro_rules! m { () => { #[cfg_attr(all(), path = \"../target/hidden.rs\")] mod hidden; }; }\nm!();\n",
        ),
        (
            "macro-rules-path-metavariable",
            "macro_rules! m { ($p:literal) => { #[path = $p] mod hidden; }; }\nm!(\"../target/hidden.rs\");\n",
        ),
        (
            "macro-rules-attribute-metavariable",
            "macro_rules! m { ($a:meta) => { #[$a] mod hidden; }; }\nm!(path = \"../target/hidden.rs\");\n",
        ),
        (
            "macro-rules-attribute-metavariable-named-mod",
            "macro_rules! m { ($a:meta, $n:ident) => { #[$a] mod $n; }; }\nm!(path = \"../target/hidden.rs\", hidden);\n",
        ),
        (
            "cfg-attr-path-non-literal",
            "#[cfg_attr(all(), path = concat!(\"../target/\", \"hidden.rs\"))]\nmod hidden;\n",
        ),
    ] {
        let root = source_root(name, lib);
        assert!(escape_risk(&root).is_some(), "{name}: expected an escape risk for {lib}");
        assert!(!snapshot_with(&root, &[]).complete, "{name}: incomplete");
    }
}

/// @vtest.id TEST-EXEC-STATE-IN-SCOPE-CFG-ATTR-PATH-NOT-RISK
/// @vtest.covers VO-EXEC-STATE-IN-SCOPE-INCLUDE-NOT-INCOMPLETE
/// @vtest.target crates/vtest-adapter-rust/src/execution_state.rs::escape_risk_in
/// @vtest.intent 包含領域内を指すcfg_attr内の#[path]と、file moduleを宣言しないmacro_rules!の$(#[$meta])*はescape riskにしないことを確認する
#[test]
fn in_scope_cfg_attr_paths_and_doc_pass_through_attributes_are_not_escape_risk() {
    for (name, lib) in [
        (
            "cfg-attr-path-in-scope",
            "#[cfg_attr(all(), path = \"inner/hidden.rs\")]\nmod hidden;\n",
        ),
        (
            "meta-pass-through",
            "macro_rules! id { ($(#[$meta:meta])* $name:ident) => { $(#[$meta])* pub struct $name; }; }\nid!(#[derive(Debug)] Foo);\n",
        ),
        (
            "meta-on-inline-mod",
            "macro_rules! m { ($a:meta) => { #[$a] mod inline {} }; }\nm!(allow(dead_code));\n",
        ),
    ] {
        let root = source_root(name, lib);
        assert!(escape_risk(&root).is_none(), "{name}: {:?}", escape_risk(&root));
    }
}

/// @vtest.id TEST-EXEC-STATE-NESTED-IN-SCOPE-INCLUDE-NOT-RISK
/// @vtest.covers VO-EXEC-STATE-IN-SCOPE-INCLUDE-NOT-INCOMPLETE
/// @vtest.target crates/vtest-adapter-rust/src/execution_state.rs::escape_risk_in
/// @vtest.intent 外側macro内でも包含領域内を指すinclude_str!と、includeという名前の変数はescape riskにしないことを確認する
#[test]
fn nested_in_scope_includes_and_identifiers_named_include_are_not_escape_risk() {
    for (name, lib) in [
        (
            "nested-in-scope",
            "pub fn f() -> &'static str { concat!(include_str!(\"../tests/fixtures/in-scope.txt\")) }\n",
        ),
        (
            "identifier-named-include",
            "pub fn f(include: bool) -> String { format!(\"{}\", include) }\n",
        ),
    ] {
        let root = source_root(name, lib);
        assert!(escape_risk(&root).is_none(), "{name}: {:?}", escape_risk(&root));
    }
}

// ---------------------------------------------------------------------------
// Issue #119: the build script Cargo actually selects.
// ---------------------------------------------------------------------------

/// @vtest.id TEST-EXEC-STATE-RENAMED-BUILD-SCRIPT-FORCES-INCOMPLETE
/// @vtest.covers VO-EXEC-STATE-ESCAPE-RISK-FORCES-INCOMPLETE
/// @vtest.target crates/vtest-adapter-rust/src/execution_state.rs::escape_risk_in
/// @vtest.intent package.buildで指定した別名build scriptとproc-macro crateをescape riskとし、build=falseの通常crateはriskにしないことを確認する
#[test]
fn a_manifest_selected_build_script_or_proc_macro_forces_incompleteness() {
    let renamed = temp_dir("renamed-build-script");
    fs::write(
        renamed.join("Cargo.toml"),
        "[package]\nname = \"fixture\"\nbuild = \"builder.rs\"\n",
    )
    .unwrap();
    fs::write(renamed.join("builder.rs"), "fn main() {}\n").unwrap();
    assert!(escape_risk(&renamed).is_some());
    assert!(!snapshot_with(&renamed, &[]).complete);

    let nested = temp_dir("nested-member-build-script");
    fs::create_dir_all(nested.join("member")).unwrap();
    fs::write(
        nested.join("Cargo.toml"),
        "[workspace]\nmembers = [\"member\"]\n",
    )
    .unwrap();
    fs::write(
        nested.join("member/Cargo.toml"),
        "[package]\nname = \"member\"\nbuild = \"scripts/gen.rs\"\n",
    )
    .unwrap();
    assert!(escape_risk(&nested).is_some());

    let proc_macro = temp_dir("proc-macro-crate");
    fs::write(
        proc_macro.join("Cargo.toml"),
        "[package]\nname = \"fixture\"\n[lib]\nproc-macro = true\n",
    )
    .unwrap();
    assert!(escape_risk(&proc_macro).is_some());

    let disabled = temp_dir("build-disabled");
    fs::write(
        disabled.join("Cargo.toml"),
        "[package]\nname = \"fixture\"\nbuild = false\n",
    )
    .unwrap();
    assert!(escape_risk(&disabled).is_none());
    assert!(snapshot_with(&disabled, &[]).complete);
}

// ---------------------------------------------------------------------------
// Issue #121: the Cargo target directory Cargo actually uses.
// ---------------------------------------------------------------------------

fn manifest_paths(draft: &vtest_model::ExecutionStateDraft) -> Vec<String> {
    draft
        .inputs
        .iter()
        .map(|input| input.root_relative_path.clone())
        .collect()
}

/// @vtest.id TEST-EXEC-STATE-RESOLVED-TARGET-DIR-EXCLUDED
/// @vtest.covers VO-EXEC-STATE-NESTED-TARGET-DIR-NOT-EXCLUDED
/// @vtest.target crates/vtest-adapter-rust/src/execution_state.rs::ExecutionContext::resolve
/// @vtest.intent 既定target/、CARGO_TARGET_DIRによるroot内の別名out/、.cargo/configのbuild.target-dirを生成物として除外し、root外targetではroot内のtarget/を入力のままにすることを確認する
#[test]
fn the_resolved_cargo_target_directory_is_excluded_and_only_it() {
    let root = cargo_root("resolved-target-dir");
    for dir in ["target", "out", "build-out"] {
        fs::create_dir_all(root.join(dir)).unwrap();
        fs::write(root.join(dir).join("artifact"), "1").unwrap();
    }
    let has = |draft: &vtest_model::ExecutionStateDraft, path: &str| {
        manifest_paths(draft).iter().any(|p| p == path)
    };

    let default = snapshot_with(&root, &[]);
    assert!(!has(&default, "target/artifact"));
    assert!(has(&default, "out/artifact"));

    let relocated = snapshot_with(&root, &[("CARGO_TARGET_DIR", "out")]);
    assert!(relocated.complete);
    assert!(!has(&relocated, "out/artifact"));
    assert!(has(&relocated, "target/artifact"));
    fs::write(root.join("out/artifact"), "2").unwrap();
    let rebuilt = snapshot_with(&root, &[("CARGO_TARGET_DIR", "out")]);
    assert_eq!(
        hash_of(&relocated),
        hash_of(&rebuilt),
        "generated output in the relocated target directory is not an input"
    );

    let outside = temp_dir("resolved-target-dir-outside");
    let outside_dir = outside.display().to_string();
    let external = snapshot_with(&root, &[("CARGO_TARGET_DIR", &outside_dir)]);
    assert!(external.complete);
    assert!(has(&external, "target/artifact"));
    assert!(has(&external, "out/artifact"));

    fs::create_dir_all(root.join(".cargo")).unwrap();
    fs::write(
        root.join(".cargo/config.toml"),
        "[build]\ntarget-dir = \"build-out\"\n",
    )
    .unwrap();
    let configured = snapshot_with(&root, &[]);
    assert!(configured.complete);
    assert!(!has(&configured, "build-out/artifact"));
    assert!(has(&configured, "target/artifact"));
}

/// @vtest.id TEST-EXEC-STATE-READ-OF-RELOCATED-OUTPUT-INCOMPLETE
/// @vtest.covers VO-EXEC-STATE-ESCAPE-RISK-FORCES-INCOMPLETE
/// @vtest.target crates/vtest-adapter-rust/src/execution_state.rs::escape_risk_in
/// @vtest.intent 別名target directoryをinclude_str!で読むsourceと、target directoryがroot自身を含む設定は不完全とすることを確認する
#[test]
fn reading_the_relocated_output_or_overlapping_the_root_is_incomplete() {
    let root = source_root(
        "relocated-output-read",
        "pub fn f() -> &'static str { include_str!(\"../out/factor\") }\n",
    );
    assert!(
        snapshot_with(&root, &[]).complete,
        "out/ is an input by default"
    );
    assert!(!snapshot_with(&root, &[("CARGO_TARGET_DIR", "out")]).complete);
    assert!(!snapshot_with(&root, &[("CARGO_TARGET_DIR", ".")]).complete);
}
