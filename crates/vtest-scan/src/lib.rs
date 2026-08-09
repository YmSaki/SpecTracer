//! Language-neutral discovery orchestration.
//!
//! The crate owns adapter selection, result merging, and the public scan
//! envelope.  Language syntax, source walking, annotation parsing, and Cargo
//! target resolution are implemented by the selected adapter.

use std::{
    collections::BTreeSet,
    io,
    path::{Path, PathBuf},
};

use thiserror::Error;
use vtest_adapter_api::{
    AdapterConfig, Capability, DiscoveredTestDraft, DiscoveryBatch, TestWireCodec,
};
use vtest_model::{
    target_subject_hash, test_subject_hash, AdapterId, Diagnostic, ScanSummary, SourceFunction,
    SrcId, TestEntity,
};
use vtest_store::{load_config, ProjectConfig, StoreError};

#[derive(Debug, Error)]
pub enum ScanError {
    #[error("store error: {0}")]
    Store(StoreError),
    #[error("I/O error at {path}: {source}")]
    Io { path: PathBuf, source: io::Error },
    #[error("source discovery failed at {path}: {message}")]
    Discovery { path: PathBuf, message: String },
    #[error("adapter {code}: {message}")]
    Adapter { code: String, message: String },
}

impl From<StoreError> for ScanError {
    fn from(value: StoreError) -> Self {
        Self::Store(value)
    }
}

#[derive(Clone, Debug)]
pub struct ScanResult {
    pub summary: ScanSummary,
    pub tests: Vec<TestEntity>,
    pub sources: Vec<SourceFunction>,
    pub discovered_tests: Vec<DiscoveredTestDraft>,
    pub diagnostics: Vec<Diagnostic>,
}

impl ScanResult {
    pub fn has_errors(&self) -> bool {
        self.diagnostics.iter().any(Diagnostic::is_error)
    }
}

pub fn scan_project(root: &Path) -> Result<ScanResult, ScanError> {
    let config = load_config(root)?;
    scan_project_with_config(root, &config)
}

/// Select configured adapters and merge their typed discovery results.  The
/// core never parses a source file or interprets an opaque locator.
pub fn scan_project_with_config(
    root: &Path,
    config: &ProjectConfig,
) -> Result<ScanResult, ScanError> {
    let registry = vtest_adapter_rust::registry().map_err(|error| ScanError::Adapter {
        code: error.code,
        message: error.message,
    })?;
    scan_project_with_registry(root, config, &registry)
}

/// Scan with an explicitly composed registry.  Product callers use the
/// built-in registry above; tests and future hosts can register a synthetic
/// or third-party adapter without changing this orchestration crate.
pub fn scan_project_with_registry(
    root: &Path,
    config: &ProjectConfig,
    registry: &vtest_adapter_api::AdapterRegistry,
) -> Result<ScanResult, ScanError> {
    let adapters = if config.adapters.is_empty() {
        vec![("rust-cargo".to_owned(), AdapterConfig::default())]
    } else {
        config
            .adapters
            .iter()
            .map(|entry| {
                let mut adapter_config = AdapterConfig::default();
                adapter_config.insert("roots", entry.roots.join(","));
                adapter_config.insert("include", entry.scan.include.join(","));
                adapter_config.insert("assertion_macros", entry.scan.assertion_macros.join(","));
                adapter_config.insert("coverage", entry.run.coverage.clone());
                (entry.id.clone(), adapter_config)
            })
            .collect::<Vec<_>>()
    };

    let mut tests = Vec::new();
    let mut sources = Vec::new();
    let mut discovered_tests = Vec::new();
    let mut diagnostics = Vec::new();
    let mut files = 0usize;
    let mut test_ids = BTreeSet::new();
    let mut source_ids = BTreeSet::new();

    for (adapter_name, adapter_config) in adapters {
        let adapter_id = AdapterId::from(adapter_name);
        let registration = registry
            .require(&adapter_id, Capability::Discovery)
            .map_err(|error| ScanError::Adapter {
                code: error.code,
                message: error.message,
            })?;
        let discovery = registration
            .discovery
            .as_ref()
            .ok_or_else(|| ScanError::Adapter {
                code: "E-ADAPTER-004".to_owned(),
                message: format!("adapter `{adapter_id}` does not implement discovery"),
            })?
            .discover(root, &adapter_config)
            .map_err(|error| ScanError::Adapter {
                code: error.code,
                message: error.message,
            })?;
        let wire_codec = registration
            .wire_codec
            .as_ref()
            .ok_or_else(|| ScanError::Adapter {
                code: "E-ADAPTER-004".to_owned(),
                message: format!("adapter `{adapter_id}` does not implement test wire codec"),
            })?;
        merge_discovery(
            &adapter_id,
            wire_codec.as_ref(),
            discovery,
            &mut files,
            &mut tests,
            &mut sources,
            &mut discovered_tests,
            &mut diagnostics,
            &mut test_ids,
            &mut source_ids,
        )?;
    }

    Ok(ScanResult {
        summary: ScanSummary {
            files,
            tests: tests.len(),
            sources: sources.len(),
        },
        tests,
        sources,
        discovered_tests,
        diagnostics,
    })
}

#[allow(clippy::too_many_arguments)]
fn merge_discovery(
    adapter_id: &AdapterId,
    wire_codec: &dyn TestWireCodec,
    discovery: DiscoveryBatch,
    files: &mut usize,
    tests: &mut Vec<TestEntity>,
    sources: &mut Vec<SourceFunction>,
    discovered_tests: &mut Vec<DiscoveredTestDraft>,
    diagnostics: &mut Vec<Diagnostic>,
    test_ids: &mut BTreeSet<String>,
    source_ids: &mut BTreeSet<String>,
) -> Result<(), ScanError> {
    *files = files.saturating_add(discovery.summary.files);
    diagnostics.extend(discovery.diagnostics);
    discovered_tests.extend(discovery.discovered_tests);
    let mut discovered_sources = Vec::new();
    for target in discovery.targets {
        let source = SourceFunction {
            target: target.target.clone(),
            src_id: target.src_id.map(SrcId::new),
            location: target.location,
            content_hash: target_subject_hash(&target.target, &target.construct),
        };
        discovered_sources.push(source);
    }
    for draft in discovery.managed_tests {
        let mut test = wire_codec
            .materialize_test(&draft)
            .map_err(|error| ScanError::Adapter {
                code: error.code,
                message: error.message,
            })?;
        let metadata = serde_json::to_string(&draft.metadata_sources).unwrap_or_default();
        test.content_hash = test_subject_hash(
            adapter_id,
            &draft.id,
            &metadata,
            &draft.location,
            &draft.execution,
            &draft.construct,
        );
        let id = test.id.as_str().to_owned();
        if !test_ids.insert(id.clone()) {
            return Err(ScanError::Adapter {
                code: "E-ADAPTER-003".to_owned(),
                message: format!("duplicate Test ID `{id}` across adapters"),
            });
        }
        tests.push(test);
    }
    for source in discovered_sources {
        let key = source
            .src_id
            .as_ref()
            .map(|id| id.as_str().to_owned())
            .unwrap_or_else(|| format!("{}:{}", source.target.adapter, source.target.value));
        let inserted = source_ids.insert(key.clone());
        if !inserted && source.src_id.is_some() {
            return Err(ScanError::Adapter {
                code: "E-SCAN-011".to_owned(),
                message: format!("duplicate SRC ID `{key}` across adapters"),
            });
        }
        if !inserted && source.src_id.is_none() {
            diagnostics.push(Diagnostic::warning(
                "W-ADAPTER-102",
                format!("duplicate source identity `{key}` was retained with adapter provenance"),
            ));
        }
        sources.push(source);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::{fs, sync::Arc, time::SystemTime};

    use super::scan_project_with_registry;

    use vtest_adapter_api::{
        AdapterDescriptor, AdapterRegistration, AdapterRegistry, Capability, DiscoveryBatch,
        DiscoveryCompleteness, ManagedTestDraft, ManagedTestDraftLink, SourceDiscoveryAdapter,
        SourceFragment, SourceTargetDraft, TestWireCodec,
    };
    use vtest_model::{
        AdapterId, ContentHash, ExecutionDescriptor, NeutralTargetRef, ScanSummary, SourceLocation,
        TestEntity, TestId, TestSuite,
    };
    use vtest_store::{init_project, AdapterConfigEntry, ProjectConfig};

    #[derive(Clone, Copy)]
    struct SyntheticAdapter {
        id: &'static str,
        test_id: &'static str,
        vo_id: &'static str,
        target_value: &'static str,
    }

    fn descriptor(id: &str) -> AdapterDescriptor {
        AdapterDescriptor::new(id, ["fixture"])
            .with_namespace(id)
            .with_capabilities([Capability::Discovery, Capability::TestWireCodec])
    }

    fn location(adapter: &str, path: &str, locator: &str) -> SourceLocation {
        SourceLocation::neutral(
            adapter,
            path,
            locator,
            vtest_model::ByteRange { start: 0, end: 1 },
        )
    }

    impl SourceDiscoveryAdapter for SyntheticAdapter {
        fn descriptor(&self) -> AdapterDescriptor {
            descriptor(self.id)
        }

        fn discover(
            &self,
            _root: &std::path::Path,
            _config: &vtest_adapter_api::AdapterConfig,
        ) -> Result<DiscoveryBatch, vtest_adapter_api::AdapterError> {
            let target = NeutralTargetRef::new(self.id, self.target_value);
            let test_location = location(self.id, "fixture.spec", "case");
            let target_location = location(self.id, "fixture.spec", "target");
            let draft = ManagedTestDraft {
                adapter: AdapterId::from(self.id),
                id: TestId::new(self.test_id),
                covers: vec![self.vo_id.to_owned()],
                targets: vec![target.clone()],
                intent: "synthetic behavior".to_owned(),
                input: None,
                expect: None,
                kind: Some("fixture".to_owned()),
                cases: Vec::new(),
                related: Vec::new(),
                location: test_location.clone(),
                execution: ExecutionDescriptor {
                    adapter: AdapterId::from(self.id),
                    project: Some("fixture".to_owned()),
                    suite: Some(TestSuite {
                        kind: "fixture".to_owned(),
                        name: None,
                    }),
                    selector: "case".to_owned(),
                    runner: None,
                    working_root: None,
                },
                metadata_sources: vec![SourceFragment {
                    adapter: AdapterId::from(self.id),
                    project_relative_path: "fixture.meta".to_owned(),
                    opaque_locator: "case".to_owned(),
                    bytes: b"intent: synthetic behavior".to_vec(),
                }],
                construct: b"case construct".to_vec(),
                wire_payload: serde_json::json!({"kind": "fixture"}),
            };
            Ok(DiscoveryBatch {
                adapter: AdapterId::from(self.id),
                summary: ScanSummary {
                    files: 1,
                    tests: 1,
                    sources: 1,
                },
                completeness: DiscoveryCompleteness::Complete,
                discovered_tests: vec![vtest_adapter_api::DiscoveredTestDraft {
                    adapter: AdapterId::from(self.id),
                    location: test_location,
                    construct: b"case construct".to_vec(),
                    metadata_sources: draft.metadata_sources.clone(),
                    managed: ManagedTestDraftLink::One(Box::new(draft.clone())),
                }],
                managed_tests: vec![draft],
                targets: vec![SourceTargetDraft {
                    adapter: AdapterId::from(self.id),
                    target,
                    location: target_location,
                    construct: b"target construct".to_vec(),
                    src_id: None,
                }],
                diagnostics: Vec::new(),
            })
        }
    }

    impl TestWireCodec for SyntheticAdapter {
        fn descriptor(&self) -> AdapterDescriptor {
            descriptor(self.id)
        }

        fn decode_test(
            &self,
            _bytes: &[u8],
        ) -> Result<Vec<ManagedTestDraft>, vtest_adapter_api::AdapterError> {
            Ok(Vec::new())
        }

        fn materialize_test(
            &self,
            draft: &ManagedTestDraft,
        ) -> Result<TestEntity, vtest_adapter_api::AdapterError> {
            Ok(TestEntity {
                id: draft.id.clone(),
                covers: draft.covers.iter().map(|id| id.as_str().into()).collect(),
                targets: draft.targets.clone(),
                intent: draft.intent.clone(),
                input: draft.input.clone(),
                expect: draft.expect.clone(),
                kind: draft.kind.clone(),
                cases: draft.cases.clone(),
                related: draft.related.clone(),
                location: draft.location.clone(),
                content_hash: ContentHash::from_text("placeholder"),
                execution: draft.execution.clone(),
            })
        }

        fn encode_test(
            &self,
            test: &TestEntity,
        ) -> Result<serde_json::Value, vtest_adapter_api::AdapterError> {
            Ok(serde_json::json!({
                "id": test.id,
                "adapter": test.execution.adapter,
                "selector": test.execution.selector,
                "targets": test.targets,
            }))
        }
    }

    #[test]
    fn synthetic_adapter_scans_without_rust_specific_core_fields() {
        let mut registry = AdapterRegistry::new();
        let adapter = Arc::new(SyntheticAdapter {
            id: "synthetic",
            test_id: "TEST-SYNTHETIC",
            vo_id: "VO-SYNTHETIC",
            target_value: "fixture::target",
        });
        registry
            .register(
                AdapterRegistration::new(descriptor(adapter.id))
                    .with_discovery(adapter.clone())
                    .with_wire_codec(adapter.clone()),
            )
            .expect("register synthetic adapter");
        let mut config = ProjectConfig::default_for("fixture");
        config.adapters = vec![AdapterConfigEntry {
            id: "synthetic".to_owned(),
            roots: vec![".".to_owned()],
            scan: config.scan.clone(),
            run: config.run.clone(),
        }];
        let result = scan_project_with_registry(std::path::Path::new("."), &config, &registry)
            .expect("synthetic scan succeeds");
        assert_eq!(result.tests.len(), 1);
        assert_eq!(
            result.sources[0].target.adapter,
            AdapterId::from("synthetic")
        );
        let value = adapter
            .encode_test(&result.tests[0])
            .expect("synthetic wire encoding");
        assert!(value.get("filter").is_none());
        assert!(value.get("package").is_none());
        assert!(value.get("test_target").is_none());
    }

    #[test]
    fn multiple_opaque_adapters_merge_and_duplicate_test_ids_fail_closed() {
        let first = Arc::new(SyntheticAdapter {
            id: "synthetic-one",
            test_id: "TEST-SYNTHETIC-ONE",
            vo_id: "VO-SYNTHETIC-ONE",
            target_value: "fixture::one",
        });
        let second = Arc::new(SyntheticAdapter {
            id: "synthetic-two",
            test_id: "TEST-SYNTHETIC-TWO",
            vo_id: "VO-SYNTHETIC-TWO",
            target_value: "fixture::two",
        });
        let mut registry = AdapterRegistry::new();
        for adapter in [first.clone(), second.clone()] {
            registry
                .register(
                    AdapterRegistration::new(descriptor(adapter.id))
                        .with_discovery(adapter.clone())
                        .with_wire_codec(adapter),
                )
                .expect("register distinct opaque adapter");
        }
        let mut config = ProjectConfig::default_for("fixture");
        config.adapters = [first.id, second.id]
            .into_iter()
            .map(|id| AdapterConfigEntry {
                id: id.to_owned(),
                roots: vec![".".to_owned()],
                scan: config.scan.clone(),
                run: config.run.clone(),
            })
            .collect();
        let merged = scan_project_with_registry(std::path::Path::new("."), &config, &registry)
            .expect("distinct opaque adapters merge");
        assert_eq!(merged.tests.len(), 2);
        assert_eq!(merged.sources.len(), 2);
        assert!(merged.tests.iter().all(|test| test
            .location
            .adapter
            .as_ref()
            .is_some_and(|id| id.as_str().starts_with("synthetic"))));

        let duplicate = Arc::new(SyntheticAdapter {
            id: "synthetic-duplicate",
            test_id: "TEST-SYNTHETIC-ONE",
            vo_id: "VO-SYNTHETIC-DUPLICATE",
            target_value: "fixture::duplicate",
        });
        registry
            .register(
                AdapterRegistration::new(descriptor(duplicate.id))
                    .with_discovery(duplicate.clone())
                    .with_wire_codec(duplicate),
            )
            .expect("register duplicate-id adapter");
        config.adapters.push(AdapterConfigEntry {
            id: "synthetic-duplicate".to_owned(),
            roots: vec![".".to_owned()],
            scan: config.scan.clone(),
            run: config.run.clone(),
        });
        let error = scan_project_with_registry(std::path::Path::new("."), &config, &registry)
            .expect_err("duplicate Test ID must fail closed");
        assert!(error.to_string().contains("duplicate Test ID"));
    }

    #[test]
    fn rust_and_synthetic_adapters_merge_in_one_scan() {
        let suffix = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .expect("system clock")
            .as_nanos();
        let root = std::env::temp_dir().join(format!("vtest-scan-mixed-{suffix}"));
        let _ = fs::remove_dir_all(&root);
        init_project(&root, "fixture").expect("initialise mixed fixture");
        fs::create_dir_all(root.join("src")).expect("create source directory");
        fs::create_dir_all(root.join("tests")).expect("create test directory");
        fs::write(
            root.join("Cargo.toml"),
            "[package]\nname = \"fixture\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
        )
        .expect("write mixed Cargo manifest");
        fs::write(root.join("src/lib.rs"), "pub fn target() {}\n")
            .expect("write mixed Rust source");
        fs::write(
            root.join("tests/registered.rs"),
            "/// @vtest.id TEST-RUST-MIXED\n/// @vtest.covers VO-RUST-MIXED\n/// @vtest.target src/lib.rs::target\n/// @vtest.intent mixed Rust test\n#[test]\nfn rust_mixed() { target(); }\n",
        )
        .expect("write mixed Rust test");

        let synthetic = Arc::new(SyntheticAdapter {
            id: "synthetic",
            test_id: "TEST-SYNTHETIC-MIXED",
            vo_id: "VO-SYNTHETIC-MIXED",
            target_value: "fixture::synthetic",
        });
        let mut registry = AdapterRegistry::new();
        registry
            .register(vtest_adapter_rust::registration())
            .expect("register Rust adapter");
        registry
            .register(
                AdapterRegistration::new(descriptor(synthetic.id))
                    .with_discovery(synthetic.clone())
                    .with_wire_codec(synthetic),
            )
            .expect("register synthetic adapter");
        let mut config = ProjectConfig::default_for("fixture");
        config.adapters = vec![
            AdapterConfigEntry {
                id: "rust-cargo".to_owned(),
                roots: vec![".".to_owned()],
                scan: config.scan.clone(),
                run: config.run.clone(),
            },
            AdapterConfigEntry {
                id: "synthetic".to_owned(),
                roots: vec![".".to_owned()],
                scan: config.scan.clone(),
                run: config.run.clone(),
            },
        ];
        let result = scan_project_with_registry(&root, &config, &registry)
            .expect("mixed adapter scan succeeds");
        assert!(result
            .tests
            .iter()
            .any(|test| test.id.as_str() == "TEST-RUST-MIXED"));
        assert!(result
            .tests
            .iter()
            .any(|test| test.id.as_str() == "TEST-SYNTHETIC-MIXED"));
        assert!(result
            .sources
            .iter()
            .any(|source| source.target.adapter.as_str() == "rust-cargo"));
        assert!(result
            .sources
            .iter()
            .any(|source| source.target.adapter.as_str() == "synthetic"));
        let _ = fs::remove_dir_all(root);
    }
}
