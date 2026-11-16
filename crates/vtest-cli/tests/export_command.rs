//! `vtest export` through the `vtest` binary (#69): the accepted formats,
//! what goes to stdout and stderr, the exit codes, `--quiet`, and rejected
//! arguments. The content of the test specification itself is covered by
//! `test_spec_core.rs`; the comparison with the MCP tool `export` is in
//! `mcp_cli_equivalence.rs`.

use std::{
    fs,
    path::{Path, PathBuf},
    process::{Command, Output},
    sync::atomic::{AtomicU64, Ordering},
};

use serde_json::Value;

static NEXT: AtomicU64 = AtomicU64::new(0);

const CARGO_TOML: &str =
    "[package]\nname = \"export_fixture\"\nversion = \"0.1.0\"\nedition = \"2021\"\n";

/// One managed Test whose `covers` resolves to nothing unless `VO-X` is
/// replaced, and one test function without annotations (W-SCAN-101).
const LIB_RS: &str = "pub fn target() {}\n\n#[cfg(test)]\nmod tests {\n    \
     /// @vtest.id TEST-X\n    /// @vtest.covers VO-GONE\n    \
     /// @vtest.target src/lib.rs::target\n    /// @vtest.intent fixture\n    \
     #[test]\n    fn calls_target() {\n        super::target();\n    }\n\n    \
     #[test]\n    fn bare() {\n        super::target();\n    }\n}\n";

/// The fixture without the managed Test: its only diagnostic is the warning.
const WARNING_ONLY_LIB_RS: &str = "pub fn target() {}\n\n#[cfg(test)]\nmod tests {\n    \
     #[test]\n    fn bare() {\n        super::target();\n    }\n}\n";

fn project(name: &str, lib: &str) -> PathBuf {
    let root = std::env::temp_dir().join(format!(
        "vtest-export-command-{name}-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(root.join("src")).unwrap();
    vtest_store::init_project(&root, "export-command").unwrap();
    fs::write(root.join("Cargo.toml"), CARGO_TOML).unwrap();
    fs::write(root.join("src/lib.rs"), lib).unwrap();
    root
}

/// `vtest --project <root> <args>`.
fn vtest(root: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_vtest"))
        .arg("--project")
        .arg(root)
        .args(args)
        .output()
        .unwrap()
}

fn exit(output: &Output) -> i32 {
    output.status.code().expect("exit code")
}

fn stdout(output: &Output) -> String {
    String::from_utf8(output.stdout.clone()).unwrap()
}

fn stderr(output: &Output) -> String {
    String::from_utf8(output.stderr.clone()).unwrap()
}

fn envelope(output: &Output) -> Value {
    serde_json::from_slice(&output.stdout)
        .unwrap_or_else(|error| panic!("non-JSON stdout ({error}): {}", stdout(output)))
}

/// Every file below `root` with its bytes, and every directory.
fn tree(root: &Path) -> Vec<(String, Option<Vec<u8>>)> {
    fn visit(root: &Path, directory: &Path, entries: &mut Vec<(String, Option<Vec<u8>>)>) {
        for entry in fs::read_dir(directory).unwrap() {
            let path = entry.unwrap().path();
            let relative = path
                .strip_prefix(root)
                .unwrap()
                .to_string_lossy()
                .into_owned();
            if path.is_dir() {
                entries.push((relative, None));
                visit(root, &path, entries);
            } else {
                entries.push((relative, Some(fs::read(&path).unwrap())));
            }
        }
    }
    let mut entries = Vec::new();
    visit(root, root, &mut entries);
    entries.sort();
    entries
}

/// @vtest.id TEST-CLI-EXPORT-FORMATS-AND-STREAMS
/// @vtest.covers VO-CLI-EXPORT-FORMAT-MD-JSON, VO-CLI-EXPORT-MD-STDOUT-DOCUMENT-ONLY, VO-CLI-EXPORT-MD-DIAGNOSTICS-ON-STDERR, VO-CLI-EXPORT-JSON-OK-FROM-EXIT-CODE
/// @vtest.target crates/vtest-cli/src/lib.rs::run_export
/// @vtest.intent `vtest export` writes the Markdown document alone to stdout by default and with `--format md`, writes each diagnostic as `[code] message` to stderr in the order of the JSON `diagnostics`, and with `--format json` writes the envelope whose `ok` is true for exit code 0 and false for exit code 1 while `data` stays present
/// @vtest.case warning only, exit code 0
/// @vtest.case unresolved covers, exit code 1
#[test]
fn export_writes_the_document_to_stdout_and_diagnostics_to_stderr() {
    for (name, lib, expected_exit) in [
        ("warning", WARNING_ONLY_LIB_RS, 0),
        ("unresolved", LIB_RS, 1),
    ] {
        let root = project(name, lib);

        let json = vtest(&root, &["export", "--format", "json"]);
        assert_eq!(exit(&json), expected_exit, "{name}");
        assert_eq!(stderr(&json), "", "{name}");
        let json = envelope(&json);
        assert_eq!(json["ok"], expected_exit == 0, "{name}");
        let keys: Vec<&String> = json["data"].as_object().expect("data").keys().collect();
        assert_eq!(
            keys,
            ["by_path", "by_vo", "tests", "unmanaged", "unresolved"],
            "{name}"
        );
        let diagnostics = json["diagnostics"].as_array().unwrap();
        assert!(!diagnostics.is_empty(), "{name}");
        let expected_stderr: String = diagnostics
            .iter()
            .map(|diagnostic| {
                format!(
                    "[{}] {}\n",
                    diagnostic["code"].as_str().unwrap(),
                    diagnostic["message"].as_str().unwrap()
                )
            })
            .collect();

        let default = vtest(&root, &["export"]);
        let md = vtest(&root, &["export", "--format", "md"]);
        for output in [&default, &md] {
            assert_eq!(exit(output), expected_exit, "{name}");
            let document = stdout(output);
            assert!(
                document.starts_with("# Test Specification\n"),
                "{name}: {document}"
            );
            for diagnostic in diagnostics {
                assert!(
                    !document.contains(&format!("[{}]", diagnostic["code"].as_str().unwrap())),
                    "{name}: the document must not carry diagnostics"
                );
            }
            assert_eq!(stderr(output), expected_stderr, "{name}");
        }
        assert_eq!(stdout(&default), stdout(&md), "{name}");
        fs::remove_dir_all(&root).ok();
    }
}

/// @vtest.id TEST-CLI-EXPORT-REJECTED-OUTPUT
/// @vtest.covers VO-CLI-EXPORT-REJECTED-OUTPUT
/// @vtest.target crates/vtest-cli/src/lib.rs::run_export
/// @vtest.intent an export that ends with exit code 2 prints no test specification: `--format json` prints the `ok: false`, `data: null` envelope with the diagnostic on stdout, and `--format md` prints nothing on stdout and `[code] message` on stderr
/// @vtest.case rejected config (E-CONFIG-001)
/// @vtest.case no project (E-OP-001)
#[test]
fn a_rejected_export_prints_no_test_specification() {
    let root = project("rejected", LIB_RS);
    fs::write(root.join(".verify/config.yaml"), "version: 2\nproject: [\n").unwrap();
    let outside = std::env::temp_dir().join(format!(
        "vtest-export-command-outside-{}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&outside);
    fs::create_dir_all(&outside).unwrap();

    for (directory, code) in [(&root, "E-CONFIG-001"), (&outside, "E-OP-001")] {
        let json = vtest(directory, &["export", "--format", "json"]);
        assert_eq!(exit(&json), 2, "{code}");
        assert_eq!(stderr(&json), "", "{code}");
        let json = envelope(&json);
        assert_eq!(json["ok"], false, "{code}");
        assert_eq!(json["data"], Value::Null, "{code}");
        assert_eq!(json["diagnostics"].as_array().unwrap().len(), 1, "{code}");
        assert_eq!(json["diagnostics"][0]["code"], code);
        let message = json["diagnostics"][0]["message"].as_str().unwrap();

        let md = vtest(directory, &["export", "--format", "md"]);
        assert_eq!(exit(&md), 2, "{code}");
        assert_eq!(stdout(&md), "", "{code}");
        assert_eq!(stderr(&md), format!("[{code}] {message}\n"));
    }
    fs::remove_dir_all(&root).ok();
    fs::remove_dir_all(&outside).ok();
}

/// @vtest.id TEST-CLI-EXPORT-REJECTED-ARGUMENTS
/// @vtest.covers VO-CLI-EXPORT-FORMAT-TEXT-REJECTED, VO-CLI-FORMAT-MD-EXPORT-ONLY, VO-CLI-EXPORT-EXTRA-ARGUMENTS-REJECTED
/// @vtest.target crates/vtest-cli/src/lib.rs::run
/// @vtest.intent `vtest export` rejects `--format text`, an unknown format value, a positional argument and an option other than the global ones with exit code 2, nothing on stdout and the reason on stderr, whatever `--format` is given, and another command rejects `--format md` with exit code 2
/// @vtest.case export --format text
/// @vtest.case export with a positional argument or an unknown option
/// @vtest.case scan and verify with --format md
#[test]
fn export_rejects_arguments_it_does_not_take() {
    let root = project("arguments", WARNING_ONLY_LIB_RS);
    for args in [
        &["export", "--format", "text"][..],
        &["export", "--format", "xml"],
        &["export", "extra"],
        &["export", "--format", "json", "extra"],
        &["export", "--format", "md", "extra"],
        &["export", "--output", "spec.md"],
        &["export", "--format", "json", "--output", "spec.md"],
        &["scan", "--format", "md"],
        &["verify", "--format", "md"],
    ] {
        let output = vtest(&root, args);
        assert_eq!(exit(&output), 2, "{args:?}");
        assert_eq!(stdout(&output), "", "{args:?}");
        assert_ne!(stderr(&output), "", "{args:?}");
    }
    // The accepted forms are not caught by the same checks.
    assert_eq!(exit(&vtest(&root, &["export", "--format", "md"])), 0);
    assert_eq!(exit(&vtest(&root, &["scan", "--format", "json"])), 0);
    fs::remove_dir_all(&root).ok();
}

/// @vtest.id TEST-CLI-EXPORT-QUIET
/// @vtest.covers VO-CLI-EXPORT-QUIET-PRINTS-NOTHING
/// @vtest.target crates/vtest-cli/src/lib.rs::is_quiet_export
/// @vtest.intent `vtest export --quiet` prints nothing on stdout and stderr and returns the exit code it returns without `--quiet`, for exit codes 0, 1 and 2, for every `--format` value accepted or not, and for rejected arguments
/// @vtest.case exit code 0, 1 and 2 with md and json
/// @vtest.case --format text, an unknown format value, a positional argument, an unknown option
#[test]
fn quiet_export_prints_nothing_and_keeps_the_exit_code() {
    let ok = project("quiet-ok", WARNING_ONLY_LIB_RS);
    let failed = project("quiet-failed", LIB_RS);
    let rejected = project("quiet-rejected", LIB_RS);
    fs::write(
        rejected.join(".verify/config.yaml"),
        "version: 2\nproject: [\n",
    )
    .unwrap();

    let cases: [(&Path, &[&str]); 12] = [
        (&ok, &["export"]),
        (&ok, &["export", "--format", "json"]),
        (&failed, &["export"]),
        (&failed, &["export", "--format", "json"]),
        (&rejected, &["export"]),
        (&rejected, &["export", "--format", "json"]),
        (&ok, &["export", "--format", "text"]),
        (&ok, &["export", "--format", "xml"]),
        (&ok, &["export", "extra"]),
        (&ok, &["export", "--output", "spec.md"]),
        (&ok, &["export", "--format", "json", "extra"]),
        (&failed, &["export", "--format", "md"]),
    ];
    for (root, args) in cases {
        let loud = vtest(root, args);
        let mut quiet_args = args.to_vec();
        quiet_args.push("--quiet");
        let quiet = vtest(root, &quiet_args);
        assert_eq!(exit(&quiet), exit(&loud), "{args:?}");
        assert_eq!(stdout(&quiet), "", "{args:?}");
        assert_eq!(stderr(&quiet), "", "{args:?}");
        // `--quiet` before the subcommand is the same global option.
        let mut leading = vec!["--quiet"];
        leading.extend_from_slice(args);
        let quiet = vtest(root, &leading);
        assert_eq!(exit(&quiet), exit(&loud), "{args:?}");
        assert_eq!(stdout(&quiet), "", "{args:?}");
        assert_eq!(stderr(&quiet), "", "{args:?}");
    }
    assert_eq!(exit(&vtest(&ok, &["export"])), 0);
    assert_eq!(exit(&vtest(&failed, &["export"])), 1);
    assert_eq!(exit(&vtest(&rejected, &["export"])), 2);
    assert_eq!(exit(&vtest(&ok, &["export", "extra"])), 2);

    // Another command's argument error is still reported with `--quiet`.
    assert_ne!(stderr(&vtest(&ok, &["scan", "extra", "--quiet"])), "");
    for root in [ok, failed, rejected] {
        fs::remove_dir_all(root).ok();
    }
}

/// @vtest.id TEST-CLI-EXPORT-QUIET-NON-UTF8-PROJECT
/// @vtest.covers VO-CLI-EXPORT-QUIET-PRINTS-NOTHING
/// @vtest.target crates/vtest-cli/src/lib.rs::is_quiet_export
/// @vtest.intent `vtest export --quiet` with a rejected argument prints nothing and returns exit code 2 when the `--project` value is a path that is not UTF-8, in both positions of `--quiet`, and the same call without `--quiet` reports the argument error on stderr
/// @vtest.case --project <non UTF-8> export --unknown --quiet
/// @vtest.case --quiet --project <non UTF-8> export extra
#[cfg(unix)]
#[test]
fn quiet_export_prints_nothing_with_a_project_path_that_is_not_utf8() {
    use std::{ffi::OsString, os::unix::ffi::OsStringExt};

    let project = OsString::from_vec(b"/tmp/vtest-export-\xff\xfe-project".to_vec());
    assert!(project.to_str().is_none(), "the path must not be UTF-8");
    let run = |before: &[&str], after: &[&str]| {
        Command::new(env!("CARGO_BIN_EXE_vtest"))
            .args(before)
            .arg("--project")
            .arg(&project)
            .args(after)
            .output()
            .unwrap()
    };

    for (before, after) in [
        (&[][..], &["export", "--unknown", "--quiet"][..]),
        (&[], &["export", "--quiet", "extra"]),
        (&["--quiet"], &["export", "extra"]),
        (&["--quiet"], &["export", "--format", "json", "--unknown"]),
    ] {
        let output = run(before, after);
        assert_eq!(exit(&output), 2, "{before:?} {after:?}");
        assert!(output.stdout.is_empty(), "{before:?} {after:?}");
        assert!(
            output.stderr.is_empty(),
            "{before:?} {after:?}: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    let loud = run(&[], &["export", "--unknown"]);
    assert_eq!(exit(&loud), 2);
    assert!(loud.stdout.is_empty());
    assert!(!loud.stderr.is_empty());

    // The value of `--project` is skipped, so `export` as that value is not
    // the subcommand: `scan --quiet` with a bad argument still reports.
    assert!(!vtest_cli::is_quiet_export(&[
        "--project",
        "export",
        "scan",
        "--quiet"
    ]));
    assert!(vtest_cli::is_quiet_export(&[
        OsString::from("--project"),
        project.clone(),
        OsString::from("export"),
        OsString::from("--quiet"),
    ]));
}

/// @vtest.id TEST-CLI-EXPORT-WRITES-NOTHING
/// @vtest.covers VO-CLI-EXPORT-WRITES-NOTHING
/// @vtest.target crates/vtest-cli/src/lib.rs::run_export
/// @vtest.intent the files and directories below the project root, and the bytes of every file, are the same before and after `vtest export` in both formats, with exit code 0 and with exit code 1
/// @vtest.case warning only
/// @vtest.case unresolved covers
#[test]
fn export_creates_changes_and_deletes_nothing() {
    for (name, lib) in [
        ("writes-ok", WARNING_ONLY_LIB_RS),
        ("writes-failed", LIB_RS),
    ] {
        let root = project(name, lib);
        let before = tree(&root);
        for args in [
            &["export"][..],
            &["export", "--format", "json"],
            &["export", "--quiet"],
        ] {
            vtest(&root, args);
            assert!(
                tree(&root) == before,
                "{name} {args:?}: the project changed"
            );
        }
        fs::remove_dir_all(&root).ok();
    }
}
