//! Acceptance coverage for the v0.2 adapter boundary.
//!
//! W7 requires one scan to merge Rust and synthetic adapter output, while
//! §9.1 requires a duplicate Test ID shared by adapters to be rejected.  These
//! tests use the public registry and scan entry points so both cases exercise
//! the same composition path as the CLI.

use std::{
    env, fs,
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
};

use vtest_adapter_api::AdapterRegistry;
use vtest_adapter_rust::RustCargoAdapter;
use vtest_adapter_synthetic::SyntheticAdapter;
use vtest_model::{
    DerivesFrom, DocumentFile, DocumentId, NodeSource, RootNode, SentenceNode, TestId, VoId,
    VoRecord,
};
use vtest_scan::{scan_project, TestIdLookup};
use vtest_store::{init_project, write_document_file, write_vo_record};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn temp_root(name: &str) -> PathBuf {
    let sequence = NEXT.fetch_add(1, Ordering::Relaxed);
    let root = env::temp_dir().join(format!(
        "vtest-cli-synthetic-mixed-e2e-{name}-{}-{sequence}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(&root).expect("create fixture root");
    root
}

fn mixed_registry() -> AdapterRegistry {
    let mut registry = AdapterRegistry::new();
    // Register in the opposite order from the config below.  scan_project
    // must use the adapter IDs' deterministic order when merging discovery.
    registry
        .register(Box::new(SyntheticAdapter::new()))
        .expect("synthetic adapter must register cleanly");
    registry
        .register(Box::new(RustCargoAdapter::new()))
        .expect("rust-cargo adapter must register cleanly");
    registry
}

fn document_file() -> DocumentFile {
    DocumentFile {
        schema_version: "0.1".to_owned(),
        root: vec![RootNode {
            id: DocumentId::new("ROOT-001"),
            statement: "mixed adapter fixture root".to_owned(),
            description: None,
            source: NodeSource {
                doc: "root.md".to_owned(),
                heading: "fixture".to_owned(),
                lines: [1, 1],
            },
        }],
        request: vec![SentenceNode {
            id: DocumentId::new("R-001"),
            statement: "mixed adapter fixture requirement".to_owned(),
            description: None,
            derives_from: vec![DocumentId::new("ROOT-001")],
            cites: None,
            source: NodeSource {
                doc: "req.md".to_owned(),
                heading: "fixture".to_owned(),
                lines: [1, 1],
            },
        }],
        require: Vec::new(),
        spec: Vec::new(),
        detailed_spec: Vec::new(),
        basic_design: Vec::new(),
        design: Vec::new(),
    }
}

fn vo() -> VoRecord {
    VoRecord {
        id: VoId::new("VO-MIXED"),
        parent: None,
        derives_from: vec![DerivesFrom {
            doc: DocumentId::new("R-001"),
            anchor: None,
            note: None,
        }],
        claim: "mixed adapter fixture claim".to_owned(),
        dimensions: Vec::new(),
        coverage_policy: None,
        combinations: Vec::new(),
        representative_cases: Vec::new(),
        created: "2026-09-20T00:00:00Z".to_owned(),
        updated: "2026-09-20T00:00:00Z".to_owned(),
    }
}

fn write_config(root: &Path) {
    // Keep synthetic first to exercise scan's adapter-ID ordering.  Each
    // adapter gets its own include path, but both roots intentionally overlap
    // to model a polyglot repository.
    let yaml = r#"version: 2
project:
  name: mixed-fixture
adapters:
  - id: synthetic
    roots:
      - '.'
    scan:
      include:
        - '.'
      assertion_macros: []
    run:
      coverage: none
  - id: rust-cargo
    roots:
      - '.'
    scan:
      include:
        - src
        - tests
      assertion_macros: []
    run:
      coverage: llvm-cov
verify:
  full_scope:
    - chain_integrity
    - orphan_detection
    - target_binding
    - oracle_presence
"#;
    fs::write(root.join(".verify").join("config.yaml"), yaml)
        .expect("write mixed adapter config.yaml");
}

fn build_fixture(root: &Path, rust_test_id: &str, synthetic_test_id: &str) {
    fs::create_dir_all(root.join("src")).expect("create src");
    fs::create_dir_all(root.join("tests")).expect("create tests");
    fs::write(
        root.join("Cargo.toml"),
        "[package]\nname = \"mixed-fixture\"\nversion = \"0.1.0\"\nedition = \"2021\"\n\n[workspace]\n",
    )
    .expect("write Cargo.toml");
    fs::write(
        root.join("src/lib.rs"),
        "pub fn implementation() -> bool { true }\n",
    )
    .expect("write Rust source target");
    fs::write(
        root.join("tests/rust_side.rs"),
        format!(
            "/// @vtest.id {rust_test_id}\n\
             /// @vtest.covers VO-MIXED\n\
             /// @vtest.target src/lib.rs::implementation\n\
             /// @vtest.intent exercises the rust-cargo side of the mixed fixture\n\
             #[test]\n\
             fn rust_side() {{ assert!(mixed_fixture::implementation()); }}\n"
        ),
    )
    .expect("write Rust Test construct");
    fs::write(
        root.join("fixture.synthetic"),
        format!(
            "@synthetic.test\n\
             @synthetic.id {synthetic_test_id}\n\
             @synthetic.covers VO-MIXED\n\
             @synthetic.target synthetic-target\n\
             @synthetic.intent exercises the synthetic side of the mixed fixture\n\
             plain synthetic Test construct body\n\
             ---\n\
             @synthetic.source synthetic-target\n\
             plain synthetic Source Target body\n"
        ),
    )
    .expect("write synthetic fixture");

    let layout = init_project(root, "mixed-fixture").expect("init .verify/ layout");
    write_config(root);
    write_document_file(&layout, "fixture", &document_file()).expect("write document file");
    write_vo_record(&layout, &vo()).expect("write VO record");
}

fn adapters_in_order(scan: &vtest_scan::ScanResult) -> Vec<String> {
    scan.tests
        .iter()
        .map(|test| test.execution.adapter.as_str().to_owned())
        .collect()
}

/// @vtest.id TEST-CLI-MIXED-RUST-CARGO-SYNTHETIC-SCAN-MERGE
/// @vtest.covers VO-ADAPTER-MIXED-SCAN-MERGE
/// @vtest.target crates/vtest-scan/src/lib.rs::scan_project_with_config
/// @vtest.intent verifies one registry-driven scan merges a rust-cargo Test and a synthetic Test, resolves both adapter-qualified targets, and keeps output order deterministic by adapter ID
#[test]
fn mixed_rust_cargo_and_synthetic_adapters_merge_in_one_scan() {
    let root = temp_root("merge");
    build_fixture(&root, "TEST-MIXED-RUST", "TEST-MIXED-SYNTHETIC");

    let scan = scan_project(&root, &mixed_registry()).expect("mixed scan must succeed");
    assert!(
        !scan.has_errors(),
        "mixed scan diagnostics: {:?}",
        scan.diagnostics
    );
    assert_eq!(scan.tests.len(), 2, "one Test must come from each adapter");
    assert!(scan
        .sources
        .iter()
        .any(|source| source.locator.adapter.as_str() == "rust-cargo"));
    assert!(scan
        .sources
        .iter()
        .any(|source| source.locator.adapter.as_str() == "synthetic"));
    assert_eq!(adapters_in_order(&scan), ["rust-cargo", "synthetic"]);
    assert_eq!(scan.tests[0].id, TestId::new("TEST-MIXED-RUST"));
    assert_eq!(scan.tests[1].id, TestId::new("TEST-MIXED-SYNTHETIC"));
    assert_eq!(scan.summary.tests, 2);
    assert_eq!(scan.summary.sources as usize, scan.sources.len());
}

/// @vtest.id TEST-CLI-MIXED-DUPLICATE-TEST-ID-REJECTED
/// @vtest.covers VO-ADAPTER-MIXED-DUPLICATE-TEST-ID
/// @vtest.target crates/vtest-scan/src/lib.rs::materialize_tests
/// @vtest.intent verifies a Test ID declared by rust-cargo and synthetic is rejected symmetrically with E-SCAN-002 while both colliding entities remain observable
#[test]
fn duplicate_test_id_across_adapters_is_rejected_symmetrically() {
    let root = temp_root("duplicate");
    build_fixture(&root, "TEST-MIXED-DUPLICATE", "TEST-MIXED-DUPLICATE");

    let scan = scan_project(&root, &mixed_registry()).expect("duplicate IDs are scan diagnostics");
    let duplicate_diagnostics = scan
        .diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.code == "E-SCAN-002")
        .collect::<Vec<_>>();
    assert_eq!(
        duplicate_diagnostics.len(),
        2,
        "both adapter declarations must receive E-SCAN-002: {:?}",
        scan.diagnostics
    );
    assert!(duplicate_diagnostics
        .iter()
        .all(|diagnostic| diagnostic.message.contains("TEST-MIXED-DUPLICATE")));
    assert_eq!(
        scan.tests.len(),
        2,
        "a collision must not drop either Test entity"
    );
    assert_eq!(adapters_in_order(&scan), ["rust-cargo", "synthetic"]);
    match scan.tests_by_id("TEST-MIXED-DUPLICATE") {
        TestIdLookup::Collided(tests) => {
            assert_eq!(tests.len(), 2);
            assert!(tests
                .iter()
                .all(|test| test.id == TestId::new("TEST-MIXED-DUPLICATE")));
        }
        other => panic!("expected a cross-adapter collision, got {other:?}"),
    }
}
