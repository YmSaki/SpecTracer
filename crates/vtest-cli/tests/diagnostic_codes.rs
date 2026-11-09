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
        "E-INTERNAL-001",
    );
    fs::remove_dir_all(root).unwrap();
}

/// #99: a `.verify/config.yaml` that cannot be read at all (here, it is a
/// directory) is a tool failure (DS-930/DS-1689), not a rejected config
/// (DS-929). Every config-loading entry point reports E-INTERNAL-001 with
/// exit 3 and no partial result, instead of E-CONFIG-001 / exit 2.
/// @vtest.id TEST-CLI-CONFIG-IO-INTERNAL
/// @vtest.covers VO-INTERNAL-ERROR-CODE-CLI-MCP
/// @vtest.target crates/vtest-cli/src/lib.rs::run
/// @vtest.intent an unreadable config.yaml yields E-INTERNAL-001 and exit 3 through scan, doctor, verify and run
#[test]
fn unreadable_config_is_an_internal_error() {
    for args in [
        vec!["scan"],
        vec!["doctor"],
        vec!["verify"],
        vec!["run", "--all"],
    ] {
        let root = fixture("config-io");
        let config = root.join(".verify/config.yaml");
        fs::remove_file(&config).unwrap();
        fs::create_dir(&config).unwrap();
        assert_failure(&root, &args, 3, "E-INTERNAL-001");
        fs::remove_dir_all(root).unwrap();
    }
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

/// DS-744/DS-1563: the CLI rejects combined run target axes as a usage
/// error, the same contract `RunTarget::from_axes` gives the MCP tool.
/// @vtest.id TEST-CLI-RUN-EXCLUSIVE-AXES
/// @vtest.covers VO-MCP-TOOL-CLI-EQUIVALENCE, VO-RUN-UNRESOLVED-SELECTOR-REJECTED
/// @vtest.target crates/vtest-ops/src/run.rs::RunTarget::from_axes
/// @vtest.intent the CLI rejects every run target-axis combination with exit 2 and from_axes accepts each axis alone
#[test]
fn run_rejects_combined_target_axes() {
    let root = fixture("run-exclusive");
    for args in [
        vec!["run", "--fast", "--test", "T", "--all"],
        vec!["run", "--fast", "--test", "T", "--vo", "V"],
        vec!["run", "--fast", "--vo", "V", "--all"],
    ] {
        let output = Command::new(env!("CARGO_BIN_EXE_vtest"))
            .arg("--project")
            .arg(&root)
            .args(&args)
            .output()
            .unwrap();
        assert_eq!(output.status.code(), Some(2), "{args:?}");
    }
    use vtest_ops::run::RunTarget;
    assert!(RunTarget::from_axes(vec!["T".into()], None, true).is_err());
    assert!(RunTarget::from_axes(vec!["T".into()], Some("V".into()), false).is_err());
    assert!(RunTarget::from_axes(Vec::new(), Some("V".into()), true).is_err());
    assert!(matches!(
        RunTarget::from_axes(vec!["T".into()], None, false),
        Ok(RunTarget::Test(ids)) if ids == vec!["T".to_owned()]
    ));
    assert!(matches!(
        RunTarget::from_axes(Vec::new(), Some("V".into()), false),
        Ok(RunTarget::Vo(id)) if id == "V"
    ));
    assert!(matches!(
        RunTarget::from_axes(Vec::new(), None, true),
        Ok(RunTarget::All)
    ));
}
