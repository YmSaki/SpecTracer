//! #75: architecture tests that pin the adapter boundary (計画 v0.2 §9.2).
//!
//! - core crates do not depend directly on Rust syntax crates (BD-114, BD-311)
//! - `vtest-adapter-api` depends on no language or Cargo implementation
//!   (BD-112)
//! - a scan with no usable adapter fails closed instead of reporting an empty
//!   successful scan (DS-1292)
//! - scan output depends neither on adapter registration order (DES-231) nor
//!   on the order files were created in

use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
    sync::atomic::{AtomicU64, Ordering},
};

use vtest_adapter_api::AdapterRegistry;
use vtest_adapter_rust::RustCargoAdapter;
use vtest_adapter_synthetic::SyntheticAdapter;
use vtest_scan::scan_project;

/// Crates that parse Rust or demangle Rust symbols. Only `vtest-adapter-rust`
/// may own them.
const RUST_SYNTAX_CRATES: &[&str] = &["syn", "quote", "proc-macro2", "rustc-demangle"];

/// Core crates that must stay language-independent.
const CORE_CRATES: &[&str] = &[
    "vtest-model",
    "vtest-adapter-api",
    "vtest-store",
    "vtest-scan",
    "vtest-exec",
    "vtest-verify",
];

static NEXT: AtomicU64 = AtomicU64::new(0);

fn temp_root(name: &str) -> PathBuf {
    let root = std::env::temp_dir().join(format!(
        "vtest-architecture-{name}-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(&root).unwrap();
    root
}

/// Every workspace package with the names of its direct dependencies (all
/// kinds: normal, dev and build), from `cargo metadata`.
fn workspace_direct_dependencies() -> Vec<(String, Vec<String>)> {
    workspace_dependencies(true)
}

/// Every workspace package with the names of its direct dependencies, from
/// `cargo metadata`. `every_kind: false` keeps only normal dependencies, the
/// ones a release build links.
fn workspace_dependencies(every_kind: bool) -> Vec<(String, Vec<String>)> {
    let manifest = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../Cargo.toml");
    let output = Command::new(env!("CARGO"))
        .args([
            "metadata",
            "--format-version",
            "1",
            "--no-deps",
            "--offline",
        ])
        .arg("--manifest-path")
        .arg(&manifest)
        .output()
        .expect("cargo metadata must run");
    assert!(
        output.status.success(),
        "cargo metadata failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let metadata: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("cargo metadata emits JSON");
    metadata["packages"]
        .as_array()
        .expect("metadata lists packages")
        .iter()
        .map(|package| {
            let dependencies = package["dependencies"]
                .as_array()
                .expect("package lists dependencies")
                .iter()
                .filter(|dependency| every_kind || dependency["kind"].is_null())
                .map(|dependency| dependency["name"].as_str().unwrap().to_owned())
                .collect();
            (package["name"].as_str().unwrap().to_owned(), dependencies)
        })
        .collect()
}

/// @vtest.id TEST-ARCH-CORE-HAS-NO-RUST-SYNTAX-DEPENDENCY
/// @vtest.covers VO-ARCH-CORE-NO-RUST-SYNTAX-OWNERSHIP
/// @vtest.intent cargo metadata shows no core crate (model, adapter-api, store, scan, exec, verify) with a direct syn, quote, proc-macro2 or rustc-demangle dependency, while vtest-adapter-rust still owns syn
// No `@vtest.target`: the subject is the workspace's Cargo manifests, which
// are not a Rust source item an adapter can resolve as a Source Target.
#[test]
fn core_crates_have_no_direct_rust_syntax_dependency() {
    let packages = workspace_direct_dependencies();
    for core in CORE_CRATES {
        let (_, dependencies) = packages
            .iter()
            .find(|(name, _)| name == core)
            .unwrap_or_else(|| panic!("workspace must contain {core}"));
        let forbidden: Vec<_> = dependencies
            .iter()
            .filter(|dependency| RUST_SYNTAX_CRATES.contains(&dependency.as_str()))
            .collect();
        assert!(
            forbidden.is_empty(),
            "{core} depends directly on Rust syntax crates {forbidden:?}"
        );
    }
    // The check must be able to see a violation: the Rust adapter is the one
    // crate that is supposed to own `syn`.
    let (_, rust_adapter) = packages
        .iter()
        .find(|(name, _)| name == "vtest-adapter-rust")
        .expect("workspace must contain vtest-adapter-rust");
    assert!(rust_adapter.iter().any(|dependency| dependency == "syn"));
}

/// @vtest.id TEST-ARCH-CORE-HAS-NO-LANGUAGE-ADAPTER-DEPENDENCY
/// @vtest.covers VO-ARCH-CORE-NO-RUST-SYNTAX-OWNERSHIP
/// @vtest.intent cargo metadata shows no core crate (model, adapter-api, store, scan, exec, verify) with a normal dependency on a language adapter crate, so core cannot reach Rust syntax through vtest-adapter-rust either, while vtest-cli still registers the adapters (#209)
// No `@vtest.target`, for the same reason as the test above.
#[test]
fn core_crates_have_no_normal_dependency_on_a_language_adapter() {
    let is_language_adapter =
        |name: &str| name.starts_with("vtest-adapter-") && name != "vtest-adapter-api";
    let packages = workspace_dependencies(false);
    for core in CORE_CRATES {
        let (_, dependencies) = packages
            .iter()
            .find(|(name, _)| name == core)
            .unwrap_or_else(|| panic!("workspace must contain {core}"));
        let forbidden: Vec<_> = dependencies
            .iter()
            .filter(|dependency| is_language_adapter(dependency))
            .collect();
        assert!(
            forbidden.is_empty(),
            "{core} depends on language adapters {forbidden:?}"
        );
    }
    // The check must be able to see a violation: the CLI is the composition
    // root that links the adapters.
    let (_, cli) = packages
        .iter()
        .find(|(name, _)| name == "vtest-cli")
        .expect("workspace must contain vtest-cli");
    assert!(cli.iter().any(|dependency| is_language_adapter(dependency)));
}

/// @vtest.id TEST-ARCH-ADAPTER-API-DEPENDS-ONLY-ON-MODEL
/// @vtest.covers VO-ARCH-ADAPTER-API-LANGUAGE-INDEPENDENT
/// @vtest.intent cargo metadata shows vtest-adapter-api depends on no workspace crate other than vtest-model and on no Rust syntax crate
// No `@vtest.target`, for the same reason as the test above.
#[test]
fn adapter_api_depends_on_no_language_implementation() {
    let packages = workspace_direct_dependencies();
    let workspace: Vec<&str> = packages.iter().map(|(name, _)| name.as_str()).collect();
    let (_, dependencies) = packages
        .iter()
        .find(|(name, _)| name == "vtest-adapter-api")
        .expect("workspace must contain vtest-adapter-api");
    for dependency in dependencies {
        assert!(
            !workspace.contains(&dependency.as_str()) || dependency == "vtest-model",
            "vtest-adapter-api depends on workspace crate {dependency}"
        );
        assert!(
            !RUST_SYNTAX_CRATES.contains(&dependency.as_str()),
            "vtest-adapter-api depends on {dependency}"
        );
    }
}

fn rust_fixture(root: &Path, config: &str) {
    vtest_store::init_project(root, "architecture").unwrap();
    fs::write(root.join(".verify/config.yaml"), config).unwrap();
    fs::write(
        root.join("Cargo.toml"),
        "[package]\nname = \"architecture\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    )
    .unwrap();
    fs::create_dir_all(root.join("src")).unwrap();
    fs::write(root.join("src/lib.rs"), "pub fn target() {}\n").unwrap();
}

/// @vtest.id TEST-ARCH-ZERO-ADAPTER-SCAN-FAILS-CLOSED
/// @vtest.covers VO-SCAN-UNKNOWN-ADAPTER-ID-REJECTED-E-CONFIG-001
/// @vtest.target crates/vtest-scan/src/lib.rs::scan_project
/// @vtest.intent a scan with an empty registry, or with a config that lists no adapter, is rejected and never an empty successful scan
/// @vtest.case registry with no adapter registered
/// @vtest.case config with `adapters: []`
#[test]
fn a_scan_without_adapters_fails_closed() {
    let root = temp_root("empty-registry");
    rust_fixture(
        &root,
        "version: 2\nproject:\n  name: architecture\nadapters:\n  - id: rust-cargo\n    \
         roots: [\".\"]\nverify:\n  full_scope: [chain_integrity, orphan_detection, \
         target_binding, oracle_presence]\n",
    );
    let empty_registry = scan_project(&root, &AdapterRegistry::new());
    assert!(
        empty_registry.is_err(),
        "a scan with no registered adapter must be rejected: {:?}",
        empty_registry.map(|scan| scan.summary)
    );
    fs::remove_dir_all(root).unwrap();

    let root = temp_root("empty-config");
    rust_fixture(
        &root,
        "version: 2\nproject:\n  name: architecture\nadapters: []\nverify:\n  full_scope: \
         [chain_integrity, orphan_detection, target_binding, oracle_presence]\n",
    );
    let mut registry = AdapterRegistry::new();
    registry
        .register(Box::new(RustCargoAdapter::new()))
        .unwrap();
    let empty_config = scan_project(&root, &registry);
    assert!(
        empty_config.is_err(),
        "a config with no adapter must be rejected: {:?}",
        empty_config.map(|scan| scan.summary)
    );
    fs::remove_dir_all(root).unwrap();
}

const MIXED_CONFIG: &str = "version: 2\nproject:\n  name: architecture\nadapters:\n  \
- id: synthetic\n    roots: ['.']\n    scan:\n      include: ['.']\n      assertion_macros: []\n    \
run:\n      coverage: none\n  - id: rust-cargo\n    roots: ['.']\n    scan:\n      include: [src, tests]\n      \
assertion_macros: []\n    run:\n      coverage: llvm-cov\nverify:\n  full_scope: [chain_integrity, \
orphan_detection, target_binding, oracle_presence]\n";

/// Writes the mixed fixture's files in `order` (file creation order is the
/// only thing that differs between two fixtures built with different
/// orders).
fn mixed_fixture(root: &Path, reverse: bool) {
    let mut files: Vec<(&str, String)> = vec![
        (
            "Cargo.toml",
            "[package]\nname = \"architecture\"\nversion = \"0.1.0\"\nedition = \"2021\"\n\n[workspace]\n"
                .to_owned(),
        ),
        ("src/lib.rs", "pub fn first() {}\npub fn second() {}\n".to_owned()),
    ];
    for (file, id, target) in [
        ("tests/a_side.rs", "TEST-ARCH-A", "first"),
        ("tests/b_side.rs", "TEST-ARCH-B", "second"),
        ("tests/c_side.rs", "TEST-ARCH-C", "first"),
    ] {
        files.push((
            file,
            format!(
                "/// @vtest.id {id}\n/// @vtest.covers VO-ARCH\n/// @vtest.target src/lib.rs::{target}\n\
                 /// @vtest.intent fixture\n#[test]\nfn side() {{ assert!(true); }}\n"
            ),
        ));
    }
    for (file, id) in [
        ("one.synthetic", "TEST-ARCH-S1"),
        ("two.synthetic", "TEST-ARCH-S2"),
    ] {
        files.push((
            file,
            format!(
                "@synthetic.test\n@synthetic.id {id}\n@synthetic.covers VO-ARCH\n\
                 @synthetic.target {id}-target\n@synthetic.intent fixture\nbody\n---\n\
                 @synthetic.source {id}-target\nsource body\n"
            ),
        ));
    }
    if reverse {
        files.reverse();
    }
    fs::create_dir_all(root.join("src")).unwrap();
    fs::create_dir_all(root.join("tests")).unwrap();
    for (file, text) in files {
        fs::write(root.join(file), text).unwrap();
    }
    vtest_store::init_project(root, "architecture").unwrap();
    fs::write(root.join(".verify/config.yaml"), MIXED_CONFIG).unwrap();
}

fn registry(synthetic_first: bool) -> AdapterRegistry {
    let mut registry = AdapterRegistry::new();
    let rust: Box<dyn vtest_adapter_api::Adapter> = Box::new(RustCargoAdapter::new());
    let synthetic: Box<dyn vtest_adapter_api::Adapter> = Box::new(SyntheticAdapter::new());
    let (first, second) = if synthetic_first {
        (synthetic, rust)
    } else {
        (rust, synthetic)
    };
    registry.register(first).unwrap();
    registry.register(second).unwrap();
    registry
}

fn scan_json(root: &Path, synthetic_first: bool) -> serde_json::Value {
    let scan = scan_project(root, &registry(synthetic_first)).expect("mixed scan must run");
    serde_json::to_value(scan).unwrap()
}

/// @vtest.id TEST-ARCH-SCAN-ORDER-INDEPENDENT
/// @vtest.covers VO-ARCH-DISCOVERY-ADAPTER-ID-ORDER
/// @vtest.target crates/vtest-scan/src/lib.rs::scan_project
/// @vtest.intent the same mixed rust-cargo and synthetic project scans to identical output whatever the adapter registration order or the order its files were created in
/// @vtest.case adapter registration order reversed
/// @vtest.case file creation order reversed
#[test]
fn scan_output_does_not_depend_on_registration_or_file_order() {
    let forward = temp_root("order-forward");
    mixed_fixture(&forward, false);
    let reverse = temp_root("order-reverse");
    mixed_fixture(&reverse, true);

    let baseline = scan_json(&forward, false);
    assert_eq!(
        baseline["summary"]["tests"], 5,
        "every fixture Test must be discovered: {baseline}"
    );
    assert_eq!(
        scan_json(&forward, true),
        baseline,
        "adapter registration order changed the scan"
    );
    assert_eq!(
        scan_json(&reverse, false),
        baseline,
        "file creation order changed the scan"
    );
    assert_eq!(scan_json(&reverse, true), baseline);
    fs::remove_dir_all(forward).unwrap();
    fs::remove_dir_all(reverse).unwrap();
}
