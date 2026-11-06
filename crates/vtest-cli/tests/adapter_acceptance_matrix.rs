//! #76: rows of the adapter acceptance matrix (計画 v0.2 §9.1) that had no
//! test, asserted with the *current* canonical expectations. The plan's table
//! is process material; `docs/reports/adapter-acceptance-matrix.md` maps
//! each row to its current clause and records where the plan's expected value
//! is out of date (for example the predecessor "fixed 12 items", which is now
//! the fixed four checks, DS-1494 / SPEC-399).
use std::{
    fs,
    path::{Path, PathBuf},
    process::{Command, Output},
};

const FIXED_FOUR: [&str; 4] = [
    "chain_integrity",
    "orphan_detection",
    "target_binding",
    "oracle_presence",
];

fn fixture(name: &str) -> PathBuf {
    static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let root = std::env::temp_dir().join(format!(
        "vtest-matrix-{name}-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
    ));
    fs::create_dir_all(&root).unwrap();
    vtest_store::init_project(&root, "matrix").unwrap();
    fs::write(
        root.join("Cargo.toml"),
        "[package]\nname = \"matrix\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    )
    .unwrap();
    fs::create_dir_all(root.join("src")).unwrap();
    fs::write(
        root.join("src/lib.rs"),
        "pub fn target() -> u32 { 1 }\n\n\
         #[cfg(test)]\n\
         mod tests {\n\
         \x20   /// @vtest.id TEST-MATRIX-ONE\n\
         \x20   /// @vtest.covers VO-MATRIX-ONE\n\
         \x20   /// @vtest.target src/lib.rs::target\n\
         \x20   /// @vtest.intent target returns one\n\
         \x20   #[test]\n\
         \x20   fn target_returns_one() {\n\
         \x20       assert_eq!(super::target(), 1);\n\
         \x20   }\n\
         }\n",
    )
    .unwrap();
    root
}

fn vtest(root: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_vtest"))
        .arg("--project")
        .arg(root)
        .args(["--format", "json"])
        .args(args)
        .output()
        .unwrap()
}

fn envelope(output: &Output) -> serde_json::Value {
    serde_json::from_slice(&output.stdout).unwrap_or_else(|error| {
        panic!(
            "stdout is not a JSON envelope ({error}): {}",
            String::from_utf8_lossy(&output.stdout)
        )
    })
}

fn config_path(root: &Path) -> PathBuf {
    root.join(".verify").join("config.yaml")
}

fn requested_items(envelope: &serde_json::Value) -> Vec<String> {
    envelope["data"]["scope"]["requested"]["items"]
        .as_array()
        .unwrap_or_else(|| panic!("verify data has no requested items: {envelope}"))
        .iter()
        .map(|item| item.as_str().unwrap().to_owned())
        .collect()
}

fn assert_rejected(output: &Output, code: &str) {
    assert_eq!(
        output.status.code(),
        Some(2),
        "{}",
        String::from_utf8_lossy(&output.stdout)
    );
    let envelope = envelope(output);
    assert_eq!(envelope["ok"], false);
    assert!(
        envelope["data"].is_null(),
        "a rejected run must produce no result data: {envelope}"
    );
    assert_eq!(envelope["diagnostics"][0]["code"], code, "{envelope}");
}

const V1_CONFIG: &str = "version: 1\nproject:\n  name: matrix\nscan:\n  include:\n    - src\n";

/// 計画 §9.1「v1 config + Rust fixture」「v1 config without full_scope」.
/// The plan's "fixed 12 items" is out of date: an omitted v1 `full_scope`
/// becomes the fixed four checks (DS-357), and reading never rewrites the
/// canonical config (DES-109).
/// @vtest.id TEST-MATRIX-V1-CONFIG-FIXED-FOUR-NO-REWRITE
/// @vtest.covers VO-STORE-CONFIG-V1-UPCONVERTS-RUST-CARGO, VO-SCAN-LEGACY-TWELVE-FIELDS-E-CONFIG-001
/// @vtest.target crates/vtest-cli/src/lib.rs::run
/// @vtest.intent a v1 config without full_scope scans and verifies the Rust fixture over exactly the fixed four checks and leaves config.yaml byte-identical
#[test]
fn v1_config_without_full_scope_verifies_the_fixed_four_without_rewriting() {
    let root = fixture("v1");
    fs::write(config_path(&root), V1_CONFIG).unwrap();

    // The fixture's VO is deliberately absent, so scan reports E-SCAN-003
    // (exit 1); what matters here is that the v1 config is accepted and the
    // Rust Test is discovered, not rejected (exit 2).
    let scan = vtest(&root, &["scan"]);
    assert_ne!(
        scan.status.code(),
        Some(2),
        "{}",
        String::from_utf8_lossy(&scan.stdout)
    );
    assert_eq!(
        envelope(&scan)["data"]["tests"],
        1,
        "the Rust Test must be discovered"
    );

    let verify = vtest(&root, &["verify"]);
    let verify_envelope = envelope(&verify);
    assert_ne!(
        verify.status.code(),
        Some(2),
        "a v1 config must be evaluated, not rejected: {verify_envelope}"
    );
    assert_eq!(requested_items(&verify_envelope), FIXED_FOUR);

    assert_eq!(
        fs::read_to_string(config_path(&root)).unwrap(),
        V1_CONFIG,
        "reading a v1 config must not rewrite it"
    );
    fs::remove_dir_all(root).unwrap();
}

/// 計画 §9.1「v2 config + rust-cargo」: Rust behaviour under v2 must be the
/// same as under the v1 compatibility reading (DES-109 reads v1 as exactly
/// one `rust-cargo` adapter), and full_scope is the fixed four checks.
/// @vtest.id TEST-MATRIX-V1-V2-SAME-RUST-RESULT
/// @vtest.covers VO-STORE-CONFIG-V1-UPCONVERTS-RUST-CARGO
/// @vtest.target crates/vtest-cli/src/lib.rs::run
/// @vtest.intent the same Rust fixture yields identical scan and verify JSON under a v1 config and under its v2 rust-cargo equivalent
#[test]
fn v1_and_equivalent_v2_configs_give_the_same_rust_result() {
    let v1 = fixture("same-v1");
    fs::write(config_path(&v1), V1_CONFIG).unwrap();
    let v2 = fixture("same-v2");
    fs::write(
        config_path(&v2),
        "version: 2\nproject:\n  name: matrix\nadapters:\n  - id: rust-cargo\n    roots: [\".\"]\n    \
         scan:\n      include: [src]\n      assertion_macros: []\n    run:\n      coverage: llvm-cov\n\
         verify:\n  full_scope: [chain_integrity, orphan_detection, target_binding, oracle_presence]\n",
    )
    .unwrap();

    for args in [&["scan"][..], &["verify"][..]] {
        let left = vtest(&v1, args);
        let right = vtest(&v2, args);
        assert_eq!(left.status.code(), right.status.code(), "{args:?}");
        assert_eq!(envelope(&left), envelope(&right), "{args:?}");
    }
    assert_eq!(
        requested_items(&envelope(&vtest(&v2, &["verify"]))),
        FIXED_FOUR
    );
    fs::remove_dir_all(v1).unwrap();
    fs::remove_dir_all(v2).unwrap();
}

/// 計画 §9.1「v1 11-item full_scope」「v2 incomplete / duplicate
/// full_scope」. The plan's "complete it in memory to 12 items" is out of
/// date: a predecessor enumeration is E-CONFIG-001 under either version and
/// is never completed (DS-1108, DS-1109); an incomplete or duplicated v2
/// list is E-CONFIG-001 too (DS-1662). No verification result is produced.
/// @vtest.id TEST-MATRIX-FULL-SCOPE-REJECTED-NO-RESULT
/// @vtest.covers VO-SCAN-LEGACY-TWELVE-FIELDS-E-CONFIG-001, VO-SCAN-REJECTED-CONFIG-EXIT-TWO
/// @vtest.target crates/vtest-cli/src/lib.rs::run
/// @vtest.intent predecessor, incomplete and duplicate full_scope lists are rejected with E-CONFIG-001, exit 2 and no verify data under v1 and v2
/// @vtest.case v1 with the predecessor 11-item list
/// @vtest.case v2 with the predecessor 11-item list
/// @vtest.case v2 with an incomplete list
/// @vtest.case v2 with a duplicated entry
#[test]
fn rejected_full_scope_lists_produce_no_verification_result() {
    let legacy = "[spec_coverage, vo_decomposition, vo_coverage, test_existence, \
                  static_audit, semantic_audit, impl_consistency, test_execution, \
                  runtime_result, target_execution, evidence_validity]";
    let v2 = |full_scope: &str| {
        format!(
            "version: 2\nproject:\n  name: matrix\nadapters:\n  - id: rust-cargo\n    \
             roots: [\".\"]\nverify:\n  full_scope: {full_scope}\n"
        )
    };
    for (name, config) in [
        (
            "v1-legacy",
            format!("version: 1\nproject:\n  name: matrix\nverify:\n  full_scope: {legacy}\n"),
        ),
        ("v2-legacy", v2(legacy)),
        (
            "v2-incomplete",
            v2("[chain_integrity, orphan_detection, target_binding]"),
        ),
        (
            "v2-duplicate",
            v2("[chain_integrity, chain_integrity, orphan_detection, target_binding, oracle_presence]"),
        ),
    ] {
        let root = fixture(name);
        fs::write(config_path(&root), &config).unwrap();
        assert_rejected(&vtest(&root, &["verify"]), "E-CONFIG-001");
        assert_eq!(fs::read_to_string(config_path(&root)).unwrap(), config);
        fs::remove_dir_all(root).unwrap();
    }
}

/// 計画 §9.1「adapter discovery failure」「scan rejected by adapter」: a
/// deterministic discovery failure is E-ADAPTER-002 (DS-922), is never an
/// empty successful scan (DS-1292), and `vtest scan` exits 2 with no scan
/// result (DS-935). Here the package's `tests` path is a file, so the
/// `rust-cargo` adapter cannot list its integration-test roots.
/// @vtest.id TEST-MATRIX-DISCOVERY-FAILURE-EXIT-TWO
/// @vtest.covers VO-SCAN-DISCOVERY-FAILURE-E-ADAPTER-002, VO-SCAN-UNKNOWN-ADAPTER-ID-REJECTED-E-CONFIG-001, VO-SCAN-REJECTED-CONFIG-EXIT-TWO
/// @vtest.target crates/vtest-cli/src/lib.rs::run
/// @vtest.intent an adapter discovery failure makes scan and verify exit 2 with E-ADAPTER-002 and no result, never an empty successful scan
#[test]
fn adapter_discovery_failure_is_rejected_not_an_empty_scan() {
    let root = fixture("discovery");
    fs::write(root.join("tests"), "not a directory\n").unwrap();
    for args in [&["scan"][..], &["verify"][..]] {
        assert_rejected(&vtest(&root, args), "E-ADAPTER-002");
    }
    fs::remove_dir_all(root).unwrap();
}
