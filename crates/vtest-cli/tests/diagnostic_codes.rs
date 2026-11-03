//! #25: DS-1676 / DS-929 / DS-930 require record rejection and internal
//! failure to remain distinguishable in the public JSON and process status.
use std::{fs, path::Path, process::Command};

fn fixture(name: &str) -> std::path::PathBuf {
    static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let root = std::env::temp_dir().join(format!(
        "vtest-diagnostic-{name}-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
    ));
    fs::create_dir_all(&root).unwrap();
    vtest_store::init_project(&root, "diagnostics").unwrap();
    fs::write(
        root.join("Cargo.toml"),
        "[package]\nname = \"diagnostics\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    )
    .unwrap();
    fs::create_dir_all(root.join("src")).unwrap();
    fs::write(root.join("src/lib.rs"), "pub fn target() {}\n").unwrap();
    root
}

fn assert_failure(root: &Path, args: &[&str], exit: i32, code: &str) {
    let output = Command::new(env!("CARGO_BIN_EXE_vtest"))
        .arg("--project")
        .arg(root)
        .args(["--format", "json"])
        .args(args)
        .output()
        .unwrap();
    assert_eq!(
        output.status.code(),
        Some(exit),
        "{}",
        String::from_utf8_lossy(&output.stdout)
    );
    let envelope: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(envelope["ok"], false);
    assert!(envelope["data"].is_null());
    assert_eq!(envelope["diagnostics"][0]["code"], code);
}

/// @vtest.id TEST-CLI-RECORD-ERROR-CODES
/// @vtest.covers VO-SCAN-RECORD-INTEGRITY-E-SCAN-010
/// @vtest.target crates/vtest-cli/src/lib.rs::run
/// @vtest.intent malformed records return E-SCAN-010 and exit 2 through doc, approval and run instead of internal failure
#[test]
fn malformed_records_reject_operations_with_schema_code_and_exit_two() {
    for (name, path, args) in [
        ("doc", ".verify/doc/bad.json", vec!["doc", "list"]),
        (
            "approval",
            ".verify/vo/VO-X.yaml",
            vec![
                "approval",
                "show",
                "--subject-type",
                "vo",
                "--subject-id",
                "VO-X",
            ],
        ),
        ("run", ".verify/vo/VO-X.yaml", vec!["run", "--vo", "VO-X"]),
    ] {
        let root = fixture(name);
        fs::write(root.join(path), "{\n").unwrap();
        assert_failure(&root, &args, 2, "E-SCAN-010");
        fs::remove_dir_all(root).unwrap();
    }
}

/// @vtest.id TEST-CLI-DOC-IO-INTERNAL
/// @vtest.covers VO-ISSUE25-INTERNAL-FAILURE-EXIT
/// @vtest.target crates/vtest-cli/src/lib.rs::run
/// @vtest.intent an unreadable document file remains an internal failure, distinct from a malformed record
#[test]
fn document_io_failure_remains_internal() {
    let root = fixture("io");
    fs::create_dir(root.join("source.json")).unwrap();
    assert_failure(
        &root,
        &["doc", "add", "--id", "bad", "--path", "source.json"],
        3,
        "E-CORE-001",
    );
    fs::remove_dir_all(root).unwrap();
}

/// @vtest.id TEST-CLI-CONFIG-ERROR-CODE
/// @vtest.covers VO-STORE-CONFIG-VERSION-RECOGNIZED-VALUE-REQUIRED
/// @vtest.target crates/vtest-cli/src/lib.rs::run
/// @vtest.intent malformed config is rejected with E-CONFIG-001 and exit 2 before run can execute
#[test]
fn invalid_config_is_a_usage_error() {
    let root = fixture("config");
    fs::write(root.join(".verify/config.yaml"), "version: 999\n").unwrap();
    assert_failure(&root, &["run", "--all"], 2, "E-CONFIG-001");
    fs::remove_dir_all(root).unwrap();
}
