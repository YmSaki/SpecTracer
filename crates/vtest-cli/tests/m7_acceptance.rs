//! M7 command-line acceptance coverage for measured target execution.

use std::{
    env, fs,
    path::{Path, PathBuf},
    process::{Command, Output},
    sync::atomic::{AtomicU64, Ordering},
    time::{SystemTime, UNIX_EPOCH},
};

use serde_json::Value;

static NEXT_PROJECT: AtomicU64 = AtomicU64::new(0);

struct TempProject {
    root: PathBuf,
}

impl TempProject {
    fn from_m1_base(name: &str) -> Self {
        let sequence = NEXT_PROJECT.fetch_add(1, Ordering::Relaxed);
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        let root = env::temp_dir().join(format!(
            "vtest-cli-m7-{name}-{}-{nanos}-{sequence}",
            std::process::id()
        ));
        copy_tree(&fixture_path("m1/base"), &root);
        fs::create_dir_all(root.join(".verify/approvals"))
            .expect("restore canonical approval directory");
        Self { root }
    }

    fn configure_llvm_cov(&self) {
        let config = self.root.join(".verify/config.yaml");
        let text = fs::read_to_string(&config).expect("read fixture config");
        fs::write(config, text.replace("coverage: off", "coverage: llvm-cov"))
            .expect("enable llvm-cov coverage");
    }

    fn commit_baseline(&self) {
        run_git(
            &self.root,
            ["init", "-q"],
            "initialize temporary git repository",
        );
        run_git(
            &self.root,
            ["config", "user.email", "m7-acceptance@example.invalid"],
            "configure temporary git email",
        );
        run_git(
            &self.root,
            ["config", "user.name", "M7 Acceptance"],
            "configure temporary git name",
        );
        run_git(
            &self.root,
            ["config", "commit.gpgsign", "false"],
            "disable signing for the disposable baseline commit",
        );
        run_git(&self.root, ["add", "."], "stage temporary baseline");
        run_git(
            &self.root,
            ["commit", "-qm", "M7 acceptance baseline"],
            "commit temporary baseline",
        );
    }
}

impl Drop for TempProject {
    fn drop(&mut self) {
        if self.root.exists() {
            let _ = fs::remove_dir_all(&self.root);
        }
    }
}

fn fixture_path(relative: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/fixtures/calc")
        .join(relative)
}

fn copy_tree(source: &Path, destination: &Path) {
    fs::create_dir_all(destination).expect("create fixture destination");
    for entry in fs::read_dir(source).expect("read tracked fixture directory") {
        let entry = entry.expect("read tracked fixture entry");
        let source_path = entry.path();
        let destination_path = destination.join(entry.file_name());
        if entry.file_type().expect("read fixture file type").is_dir() {
            copy_tree(&source_path, &destination_path);
        } else {
            fs::copy(source_path, destination_path).expect("copy tracked fixture file");
        }
    }
}

fn run_git<const N: usize>(root: &Path, args: [&str; N], context: &str) {
    let output = Command::new("git")
        .current_dir(root)
        .args(args)
        .output()
        .expect("run git");
    assert!(
        output.status.success(),
        "{context}: stdout={} stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

fn invoke(project: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_vtest"))
        .arg("--project")
        .arg(project)
        .args(["--format", "json", "run"])
        .args(args)
        .output()
        .expect("run vtest process")
}

fn envelope(output: &Output) -> Value {
    let text = String::from_utf8(output.stdout.clone()).expect("vtest emits UTF-8 JSON");
    serde_json::from_str(&text).unwrap_or_else(|error| {
        panic!(
            "invalid JSON envelope ({error}): stdout={text} stderr={}",
            String::from_utf8_lossy(&output.stderr)
        )
    })
}

fn setup_project(name: &str, test_body: &str) -> TempProject {
    let project = TempProject::from_m1_base(name);
    project.configure_llvm_cov();
    fs::write(
        project.root.join("src/lib.rs"),
        "#[inline(never)] pub fn known() -> i32 { 1 }\n",
    )
    .expect("write target function");
    fs::write(
        project.root.join("tests/registered.rs"),
        format!(
            "/// @vtest.id TEST-M7-{name}\n/// @vtest.covers VO-KNOWN\n/// @vtest.target src/lib.rs::known\n/// @vtest.intent M7 target execution\n#[test]\nfn {name}() {{ {test_body} }}\n"
        ),
    )
    .expect("write registered test");
    project.commit_baseline();
    project
}

#[test]
fn m7_called_target_records_measured_pass() {
    let project = setup_project("CALLED", "assert_eq!(calc_m1_base::known(), 1);");
    let output = invoke(&project.root, &["--all"]);
    assert_eq!(
        output.status.code(),
        Some(0),
        "run should succeed: stdout={} stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let value = envelope(&output);
    let evidence = &value["data"]["evidence"][0];
    assert_eq!(evidence["runner"]["kind"], "cargo-llvm-cov");
    assert_eq!(evidence["target_execution"]["checked"], true);
    assert_eq!(evidence["target_execution"]["result"], "PASS");
    assert!(evidence["target_execution"]["count"].as_u64().unwrap_or(0) >= 1);
}

#[test]
fn m7_passing_test_that_misses_target_records_measured_fail() {
    let project = setup_project(
        "MISSED",
        "if std::hint::black_box(false) { assert_eq!(calc_m1_base::known(), 1); } assert_eq!(1, 1);",
    );
    let output = invoke(&project.root, &["--all"]);
    assert_eq!(
        output.status.code(),
        Some(0),
        "execution itself succeeds: stdout={} stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let value = envelope(&output);
    let evidence = &value["data"]["evidence"][0];
    assert_eq!(evidence["result"], "PASS");
    assert_eq!(evidence["target_execution"]["checked"], true);
    assert_eq!(evidence["target_execution"]["result"], "FAIL");
    assert_eq!(evidence["target_execution"]["count"], 0);
}

#[test]
fn m7_missing_llvm_cov_is_warning_and_not_checked() {
    let project = setup_project("UNAVAILABLE", "assert_eq!(calc_m1_base::known(), 1);");
    let wrapper_dir = project.root.join("fake-bin");
    fs::create_dir_all(&wrapper_dir).expect("create fake cargo directory");
    let cargo = find_cargo();
    let wrapper = wrapper_dir.join(if cfg!(windows) { "cargo.exe" } else { "cargo" });
    fs::write(
        wrapper_dir.join("cargo-wrapper.rs"),
        format!(
            "use std::process::Command; fn main() {{ let args: Vec<String> = std::env::args().collect(); if args.get(1).is_some_and(|arg| arg == \"llvm-cov\") {{ std::process::exit(1); }} let status = Command::new({:?}).args(args.iter().skip(1)).status().expect(\"delegate cargo\"); std::process::exit(status.code().unwrap_or(1)); }}",
            cargo.display().to_string()
        ),
    )
    .expect("write cargo wrapper source");
    let rustc = find_tool("rustc");
    let compiled = Command::new(rustc)
        .args([
            "--edition=2021",
            wrapper_dir.join("cargo-wrapper.rs").to_str().unwrap(),
            "-o",
            wrapper.to_str().unwrap(),
        ])
        .output()
        .expect("compile cargo wrapper");
    assert!(
        compiled.status.success(),
        "compile cargo wrapper: stdout={} stderr={}",
        String::from_utf8_lossy(&compiled.stdout),
        String::from_utf8_lossy(&compiled.stderr)
    );
    let path_key = if cfg!(windows) { "Path" } else { "PATH" };
    let original_path = env::var(path_key).expect("test process has PATH");
    let separator = if cfg!(windows) { ';' } else { ':' };
    let child_path = format!("{}{separator}{original_path}", wrapper_dir.display());
    let output = Command::new(env!("CARGO_BIN_EXE_vtest"))
        .arg("--project")
        .arg(&project.root)
        .args(["--format", "json", "run", "--all"])
        .env(path_key, child_path)
        .output()
        .expect("run vtest with unavailable cargo-llvm-cov");
    assert_eq!(
        output.status.code(),
        Some(0),
        "warning-only run should succeed: stdout={} stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let value = envelope(&output);
    let evidence = &value["data"]["evidence"][0];
    assert_eq!(evidence["runner"]["kind"], "cargo-test");
    assert_eq!(evidence["target_execution"]["checked"], false);
    assert_eq!(evidence["target_execution"]["result"], "NOT_CHECKED");
    assert!(value["diagnostics"]
        .as_array()
        .unwrap()
        .iter()
        .any(|diagnostic| { diagnostic["code"] == "W-EXEC-101" }));
}

fn find_cargo() -> PathBuf {
    find_tool("cargo")
}

fn find_tool(name: &str) -> PathBuf {
    let locator = if cfg!(windows) { "where" } else { "which" };
    let output = Command::new(locator)
        .arg(name)
        .output()
        .unwrap_or_else(|error| panic!("locate {name}: {error}"));
    assert!(output.status.success(), "{locator} {name} failed");
    String::from_utf8_lossy(&output.stdout)
        .lines()
        .map(str::trim)
        .map(PathBuf::from)
        .find(|path| path.is_file())
        .unwrap_or_else(|| panic!("{name} executable exists"))
}
