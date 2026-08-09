//! Built-in Rust/Cargo adapter.
//!
//! Rust parser and Cargo source discovery live in this crate.  The scan
//! orchestration crate talks to it only through the language-neutral adapter
//! registry and `DiscoveryResult` contract.

pub mod discovery;
pub mod operations;
mod runner;
mod static_audit;
pub use operations::*;
pub use runner::{observe_test, ExecutionError, RunnerObservation};
pub use static_audit::{
    audit_static, AuditError, AuditOptions, AuditVerdict, RuleResult, StaticAudit,
    StaticAuditSummary,
};

use std::{fs, sync::Arc};

use vtest_adapter_api::{
    AdapterConfig, AdapterDescriptor, AdapterError, AdapterRegistration, AdapterRegistry,
    AuditRuleObservation, Capability, CoverageAdapter, DiscoveredTestDraft, DiscoveryBatch,
    DiscoveryCompleteness, ManagedTestDraft, ManagedTestDraftLink, SourceDiscoveryAdapter,
    SourceFragment, SourceTargetDraft, StaticAnalysisClosureDraft, StaticAuditAdapter,
    StaticAuditConfigDraft, StaticAuditObservation, StructuredTestAdapter, TestRunnerAdapter,
    TestWireCodec,
};
use vtest_model::AdapterId;

pub fn descriptor() -> AdapterDescriptor {
    AdapterDescriptor::new("rust-cargo", ["rust"])
        .with_namespace("rust")
        .with_capabilities([
            Capability::Discovery,
            Capability::TestWireCodec,
            Capability::StaticAudit,
            Capability::StructuredTest,
            Capability::Runner,
            Capability::Coverage,
        ])
}

#[derive(Clone, Copy, Debug, Default)]
pub struct RustCargoAdapter;

impl SourceDiscoveryAdapter for RustCargoAdapter {
    fn descriptor(&self) -> AdapterDescriptor {
        descriptor()
    }

    fn discover(
        &self,
        root: &std::path::Path,
        config: &AdapterConfig,
    ) -> Result<DiscoveryBatch, AdapterError> {
        let result =
            discovery::scan_project_with_adapter_config(root, config).map_err(|error| {
                AdapterError::new("E-ADAPTER-002", error.to_string())
                    .capability(Capability::Discovery)
            })?;
        let mut managed_tests = Vec::new();
        let mut discovered_tests = Vec::new();
        for test in result.tests {
            let test_location = neutral_location(&test.location, &test.location.function);
            let construct = source_construct(root, &test.location).map_err(|error| {
                AdapterError::new("E-ADAPTER-002", error).capability(Capability::Discovery)
            })?;
            let metadata_sources = vec![SourceFragment {
                adapter: vtest_model::AdapterId::from("rust-cargo"),
                project_relative_path: test.location.file.clone(),
                opaque_locator: test.location.function.clone(),
                bytes: construct.clone(),
            }];
            let draft = ManagedTestDraft {
                adapter: vtest_model::AdapterId::from("rust-cargo"),
                id: test.id.clone(),
                covers: test.covers.iter().map(ToString::to_string).collect(),
                targets: test.targets.clone(),
                intent: test.intent.clone(),
                input: test.input.clone(),
                expect: test.expect.clone(),
                kind: test.kind.clone(),
                cases: test.cases.clone(),
                related: test.related.clone(),
                location: test_location.clone(),
                execution: test.execution.clone(),
                metadata_sources: metadata_sources.clone(),
                construct: construct.clone(),
                wire_payload: wire_payload_without_hash(&test)?,
            };
            discovered_tests.push(DiscoveredTestDraft {
                adapter: vtest_model::AdapterId::from("rust-cargo"),
                location: draft.location.clone(),
                construct: draft.construct.clone(),
                metadata_sources,
                managed: ManagedTestDraftLink::One(Box::new(draft.clone())),
            });
            managed_tests.push(draft);
        }
        for location in result.unregistered {
            let construct = source_construct(root, &location).map_err(|error| {
                AdapterError::new("E-ADAPTER-002", error).capability(Capability::Discovery)
            })?;
            discovered_tests.push(DiscoveredTestDraft {
                adapter: vtest_model::AdapterId::from("rust-cargo"),
                location,
                construct,
                metadata_sources: Vec::new(),
                managed: ManagedTestDraftLink::Missing,
            });
        }
        let mut targets = Vec::new();
        for source in result.sources {
            let source_location = neutral_location(&source.location, &source.target.value);
            let construct = source_construct(root, &source.location).map_err(|error| {
                AdapterError::new("E-ADAPTER-002", error).capability(Capability::Discovery)
            })?;
            targets.push(SourceTargetDraft {
                adapter: vtest_model::AdapterId::from("rust-cargo"),
                target: vtest_model::NeutralTargetRef::new(
                    "rust-cargo",
                    source.target.value.clone(),
                ),
                location: source_location,
                construct,
                src_id: source.src_id.map(|id| id.to_string()),
            });
        }
        Ok(DiscoveryBatch {
            adapter: vtest_model::AdapterId::from("rust-cargo"),
            summary: result.summary,
            completeness: DiscoveryCompleteness::Complete,
            discovered_tests,
            managed_tests,
            targets,
            diagnostics: result.diagnostics,
        })
    }
}

impl TestWireCodec for RustCargoAdapter {
    fn descriptor(&self) -> AdapterDescriptor {
        descriptor()
    }

    fn decode_test(&self, bytes: &[u8]) -> Result<Vec<ManagedTestDraft>, AdapterError> {
        let payload = serde_json::from_slice::<serde_json::Value>(bytes).map_err(|error| {
            AdapterError::new("E-ADAPTER-005", error.to_string())
                .capability(Capability::TestWireCodec)
        })?;
        let payload = normalize_wire_payload(payload)?;
        let object = payload.as_object().ok_or_else(|| {
            AdapterError::new(
                "E-ADAPTER-005",
                "Rust test wire value must be a JSON object",
            )
            .capability(Capability::TestWireCodec)
        })?;
        let id = object
            .get("id")
            .and_then(serde_json::Value::as_str)
            .map(vtest_model::TestId::new)
            .ok_or_else(|| {
                AdapterError::new("E-ADAPTER-005", "Rust Test wire value is missing id")
                    .capability(Capability::TestWireCodec)
            })?;
        let location = object
            .get("location")
            .cloned()
            .ok_or_else(|| {
                AdapterError::new("E-ADAPTER-005", "Rust Test wire value is missing location")
                    .capability(Capability::TestWireCodec)
            })
            .and_then(|value| {
                serde_json::from_value::<vtest_model::SourceLocation>(value).map_err(|error| {
                    AdapterError::new("E-ADAPTER-005", error.to_string())
                        .capability(Capability::TestWireCodec)
                })
            })?;
        let location = neutral_location(&location, &location.function);
        let targets = object
            .get("targets")
            .cloned()
            .map(serde_json::from_value)
            .transpose()
            .map_err(|error| {
                AdapterError::new("E-ADAPTER-005", error.to_string())
                    .capability(Capability::TestWireCodec)
            })?
            .unwrap_or_default();
        let execution = object
            .get("execution")
            .cloned()
            .map(serde_json::from_value)
            .transpose()
            .map_err(|error| {
                AdapterError::new("E-ADAPTER-005", error.to_string())
                    .capability(Capability::TestWireCodec)
            })?
            .ok_or_else(|| {
                AdapterError::new(
                    "E-ADAPTER-005",
                    "Rust Test wire value is missing execution descriptor",
                )
                .capability(Capability::TestWireCodec)
            })?;
        let covers = object
            .get("covers")
            .cloned()
            .map(serde_json::from_value)
            .transpose()
            .map_err(|error| {
                AdapterError::new("E-ADAPTER-005", error.to_string())
                    .capability(Capability::TestWireCodec)
            })?
            .unwrap_or_default();
        let related = object
            .get("related")
            .cloned()
            .map(serde_json::from_value)
            .transpose()
            .map_err(|error| {
                AdapterError::new("E-ADAPTER-005", error.to_string())
                    .capability(Capability::TestWireCodec)
            })?
            .unwrap_or_default();
        let metadata_sources = object
            .get("metadata_sources")
            .cloned()
            .map(serde_json::from_value)
            .transpose()
            .map_err(|error| {
                AdapterError::new("E-ADAPTER-005", error.to_string())
                    .capability(Capability::TestWireCodec)
            })?
            .unwrap_or_default();
        Ok(vec![ManagedTestDraft {
            adapter: AdapterId::from("rust-cargo"),
            id,
            covers,
            targets,
            intent: object
                .get("intent")
                .and_then(serde_json::Value::as_str)
                .unwrap_or_default()
                .to_owned(),
            input: object
                .get("input")
                .and_then(serde_json::Value::as_str)
                .map(ToOwned::to_owned),
            expect: object
                .get("expect")
                .and_then(serde_json::Value::as_str)
                .map(ToOwned::to_owned),
            kind: object
                .get("kind")
                .and_then(serde_json::Value::as_str)
                .map(ToOwned::to_owned),
            cases: object
                .get("cases")
                .cloned()
                .map(serde_json::from_value)
                .transpose()
                .map_err(|error| {
                    AdapterError::new("E-ADAPTER-005", error.to_string())
                        .capability(Capability::TestWireCodec)
                })?
                .unwrap_or_default(),
            related,
            location,
            execution,
            metadata_sources,
            construct: Vec::new(),
            wire_payload: payload,
        }])
    }

    fn materialize_test(
        &self,
        draft: &ManagedTestDraft,
    ) -> Result<vtest_model::TestEntity, AdapterError> {
        let mut payload = normalize_wire_payload(draft.wire_payload.clone())?;
        let object = payload.as_object_mut().ok_or_else(|| {
            AdapterError::new(
                "E-ADAPTER-005",
                "Rust test wire payload must be a JSON object",
            )
            .capability(Capability::TestWireCodec)
        })?;
        // content_hash is deliberately absent from the adapter payload.  A
        // placeholder is used only to satisfy the legacy wire model; the
        // language-neutral scan core overwrites it from canonical draft
        // material before the TestEntity is published.
        object.insert(
            "content_hash".to_owned(),
            serde_json::Value::String(vtest_model::ContentHash::from_text("").to_string()),
        );
        serde_json::from_value(payload).map_err(|error| {
            AdapterError::new("E-ADAPTER-005", error.to_string())
                .capability(Capability::TestWireCodec)
        })
    }

    fn encode_test(
        &self,
        test: &vtest_model::TestEntity,
    ) -> Result<serde_json::Value, AdapterError> {
        let mut value = serde_json::to_value(test).map_err(|error| {
            AdapterError::new("E-ADAPTER-005", error.to_string())
                .capability(Capability::TestWireCodec)
        })?;
        let object = value.as_object_mut().ok_or_else(|| {
            AdapterError::new(
                "E-ADAPTER-005",
                "Rust Test wire value must be a JSON object",
            )
            .capability(Capability::TestWireCodec)
        })?;
        object.insert(
            "filter".to_owned(),
            serde_json::Value::String(test.execution.selector.clone()),
        );
        object.insert(
            "package".to_owned(),
            serde_json::Value::String(test.execution.project.clone().unwrap_or_default()),
        );
        let target = test.execution.suite.as_ref();
        let target_value = match target.map(|suite| suite.kind.as_str()) {
            Some("lib") => serde_json::json!({"kind": "lib"}),
            Some("bin") => serde_json::json!({
                "kind": "bin",
                "name": target.and_then(|suite| suite.name.clone()).unwrap_or_default(),
            }),
            Some("integration") => serde_json::json!({
                "kind": "integration_test",
                "name": target.and_then(|suite| suite.name.clone()).unwrap_or_default(),
            }),
            _ => serde_json::json!({"kind": "unknown"}),
        };
        object.insert("test_target".to_owned(), target_value);
        object.remove("target");
        if test.targets.len() == 1 {
            let target = &test.targets[0];
            let legacy_target = if let Some(locator) = vtest_model::Locator::parse(&target.value) {
                serde_json::json!({
                    "kind": "locator",
                    "value": {
                        "path": locator.path,
                        "item_path": locator.item_path,
                    }
                })
            } else {
                serde_json::json!({"kind": "src_id", "value": target.value})
            };
            object.insert("target".to_owned(), legacy_target);
        }
        Ok(value)
    }
}

impl StructuredTestAdapter for RustCargoAdapter {
    fn descriptor(&self) -> AdapterDescriptor {
        descriptor()
    }
}

impl StaticAuditAdapter for RustCargoAdapter {
    fn descriptor(&self) -> AdapterDescriptor {
        descriptor()
    }

    fn audit(
        &self,
        root: &std::path::Path,
        test: &vtest_model::TestEntity,
        config: &AdapterConfig,
    ) -> Result<StaticAuditObservation, AdapterError> {
        let summary = static_audit::audit_static_with_adapter_config(
            root,
            &static_audit::AuditOptions {
                test_id: Some(test.id.to_string()),
            },
            config,
        )
        .map_err(|error| {
            AdapterError::new("E-ADAPTER-002", error.to_string())
                .capability(Capability::StaticAudit)
        })?;
        let audit = summary.audits.into_iter().next().ok_or_else(|| {
            AdapterError::new(
                "E-ADAPTER-002",
                format!("static audit produced no result for {}", test.id),
            )
            .capability(Capability::StaticAudit)
        })?;
        let verdict = match audit.verdict {
            static_audit::AuditVerdict::Pass => vtest_model::CheckValue::Pass,
            static_audit::AuditVerdict::Fail => vtest_model::CheckValue::Fail,
            static_audit::AuditVerdict::Unknown => vtest_model::CheckValue::Unknown,
        };
        let sources = audit
            .subjects
            .iter()
            .filter_map(|subject| subject.locator.as_deref())
            .filter_map(|locator| {
                let locator = locator.strip_prefix("test-source::").unwrap_or(locator);
                let (path, opaque_locator) = locator.split_once("::")?;
                let bytes = fs::read(root.join(path)).ok()?;
                Some(SourceFragment {
                    adapter: vtest_model::AdapterId::from("rust-cargo"),
                    project_relative_path: path.to_owned(),
                    opaque_locator: opaque_locator.to_owned(),
                    bytes,
                })
            })
            .collect();
        let rules = audit
            .rules
            .into_iter()
            .map(|rule| AuditRuleObservation {
                rule: rule.rule,
                verdict: match rule.verdict {
                    static_audit::AuditVerdict::Pass => vtest_model::CheckValue::Pass,
                    static_audit::AuditVerdict::Fail => vtest_model::CheckValue::Fail,
                    static_audit::AuditVerdict::Unknown => vtest_model::CheckValue::Unknown,
                },
                reason: rule.reason,
                location: rule.location,
            })
            .collect();
        Ok(StaticAuditObservation {
            verdict,
            config: StaticAuditConfigDraft {
                rule_set_id: "rust-da".to_owned(),
                rule_set_version: "m3-v1".to_owned(),
                effective_config: serde_json::json!({
                    "assertion_macros": config.get("assertion_macros").unwrap_or_default(),
                }),
            },
            closure: StaticAnalysisClosureDraft {
                complete: true,
                sources,
            },
            rules,
            diagnostics: audit.diagnostics,
        })
    }
}

impl CoverageAdapter for RustCargoAdapter {
    fn descriptor(&self) -> AdapterDescriptor {
        descriptor()
    }
}

impl TestRunnerAdapter for RustCargoAdapter {
    fn descriptor(&self) -> AdapterDescriptor {
        descriptor()
    }

    fn run(
        &self,
        root: &std::path::Path,
        test: &vtest_model::TestEntity,
        config: &AdapterConfig,
        fast: bool,
    ) -> Result<vtest_adapter_api::RunnerObservation, AdapterError> {
        let target_locators = test
            .targets
            .iter()
            .map(|target| vtest_model::Locator::parse(&target.value))
            .collect::<Vec<_>>();
        if target_locators.iter().any(Option::is_none) {
            return Err(AdapterError::new(
                "E-ADAPTER-002",
                format!(
                    "one or more Rust targets for Test {} are not resolvable",
                    test.id
                ),
            )
            .capability(Capability::Runner));
        }
        let coverage_mode = config.get("coverage").unwrap_or("llvm-cov");
        let effective_config = serde_json::to_value(&config.values).unwrap_or_default();
        let observation = runner::observe_test(
            root,
            test,
            &target_locators,
            fast,
            coverage_mode,
            effective_config,
        )
        .map_err(|error| {
            AdapterError::new("E-ADAPTER-002", error.to_string()).capability(Capability::Runner)
        })?
        .ok_or_else(|| {
            AdapterError::new(
                "E-ADAPTER-006",
                format!("Test {} was ignored and produced no observation", test.id),
            )
            .capability(Capability::Runner)
        })?;
        Ok(vtest_adapter_api::RunnerObservation {
            adapter: vtest_model::AdapterId::from("rust-cargo"),
            result: observation.result,
            runner_kind: observation.runner_kind,
            command: observation.command,
            exit_code: observation.exit_code,
            log: observation.log,
            target_execution: vtest_adapter_api::TargetExecutionObservation {
                checked: observation.target_execution.checked,
                method: observation.target_execution.method,
                result: observation.target_execution.result,
                count: observation.target_execution.count,
                targets: test
                    .targets
                    .iter()
                    .cloned()
                    .zip(observation.target_executions)
                    .map(
                        |(target, execution)| vtest_adapter_api::TargetExecutionEntry {
                            target,
                            result: execution.result,
                            count: execution.count,
                        },
                    )
                    .collect(),
            },
            execution_state: observation.execution_state,
            diagnostics: observation.diagnostics,
        })
    }
}

pub fn registration() -> AdapterRegistration {
    let adapter = Arc::new(RustCargoAdapter);
    AdapterRegistration::new(descriptor())
        .with_discovery(adapter.clone())
        .with_wire_codec(adapter.clone())
        .with_static_audit(adapter.clone())
        .with_structured_test(adapter.clone())
        .with_runner(adapter.clone())
        .with_coverage(adapter)
}

pub fn registry() -> Result<AdapterRegistry, vtest_adapter_api::AdapterError> {
    let mut registry = AdapterRegistry::new();
    registry.register(registration())?;
    Ok(registry)
}

pub fn encode_test_wire(test: &vtest_model::TestEntity) -> Result<serde_json::Value, AdapterError> {
    RustCargoAdapter.encode_test(test)
}

fn source_construct(
    root: &std::path::Path,
    location: &vtest_model::SourceLocation,
) -> Result<Vec<u8>, String> {
    let path = root.join(&location.file);
    let source = fs::read_to_string(&path)
        .map_err(|error| format!("failed to read {}: {error}", path.display()))?;
    source
        .get(location.start_byte..location.end_byte)
        .map(str::as_bytes)
        .map(ToOwned::to_owned)
        .ok_or_else(|| format!("invalid source span in {}", path.display()))
}

fn neutral_location(
    location: &vtest_model::SourceLocation,
    opaque_locator: &str,
) -> vtest_model::SourceLocation {
    let mut location = location.clone();
    location.adapter = Some(vtest_model::AdapterId::from("rust-cargo"));
    location.project_relative_path = Some(location.file.clone());
    location.opaque_locator = Some(opaque_locator.to_owned());
    location.byte_range = Some(vtest_model::ByteRange {
        start: location.start_byte,
        end: location.end_byte,
    });
    location
}

fn wire_payload_without_hash(
    test: &vtest_model::TestEntity,
) -> Result<serde_json::Value, AdapterError> {
    let mut payload = serde_json::to_value(test).map_err(|error| {
        AdapterError::new("E-ADAPTER-002", error.to_string()).capability(Capability::TestWireCodec)
    })?;
    payload
        .as_object_mut()
        .ok_or_else(|| {
            AdapterError::new(
                "E-ADAPTER-002",
                "Rust test wire payload must be a JSON object",
            )
            .capability(Capability::TestWireCodec)
        })?
        .remove("content_hash");
    Ok(payload)
}

fn normalize_wire_payload(
    mut payload: serde_json::Value,
) -> Result<serde_json::Value, AdapterError> {
    let object = payload.as_object_mut().ok_or_else(|| {
        AdapterError::new(
            "E-ADAPTER-005",
            "Rust test wire value must be a JSON object",
        )
        .capability(Capability::TestWireCodec)
    })?;
    if !object.contains_key("targets") {
        let targets = object
            .remove("target")
            .and_then(|target| {
                let kind = target.get("kind")?.as_str()?;
                let value = target.get("value")?;
                let value = if kind == "locator" {
                    let path = value.get("path")?.as_str()?;
                    let item_path = value.get("item_path")?.as_str()?;
                    format!("{path}::{item_path}")
                } else if kind == "src_id" {
                    value.as_str()?.to_owned()
                } else {
                    return None;
                };
                Some(serde_json::json!([
                    {"adapter": "rust-cargo", "value": value}
                ]))
            })
            .unwrap_or_else(|| serde_json::json!([]));
        object.insert("targets".to_owned(), targets);
    }
    if !object.contains_key("execution") {
        let selector = object
            .get("filter")
            .and_then(serde_json::Value::as_str)
            .or_else(|| {
                object
                    .get("location")
                    .and_then(|location| location.get("function"))
                    .and_then(serde_json::Value::as_str)
            })
            .unwrap_or_default()
            .to_owned();
        let project = object
            .get("package")
            .and_then(serde_json::Value::as_str)
            .filter(|value| !value.is_empty())
            .map(ToOwned::to_owned);
        let suite = object.get("test_target").and_then(|target| {
            let kind = target.get("kind")?.as_str()?;
            let name = target
                .get("name")
                .and_then(serde_json::Value::as_str)
                .filter(|name| !name.is_empty())
                .map(ToOwned::to_owned);
            let kind = match kind {
                "integration_test" => "integration",
                value => value,
            };
            Some(serde_json::json!({"kind": kind, "name": name}))
        });
        object.insert(
            "execution".to_owned(),
            serde_json::json!({
                "adapter": "rust-cargo",
                "project": project,
                "suite": suite,
                "selector": selector,
                "runner": "cargo-test"
            }),
        );
    }
    object
        .entry("cases".to_owned())
        .or_insert_with(|| serde_json::json!([]));
    object
        .entry("related".to_owned())
        .or_insert_with(|| serde_json::json!([]));
    Ok(payload)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn legacy_wire_codec_reconstructs_neutral_execution_and_target() {
        let legacy = serde_json::json!({
            "id": "TEST-LEGACY-CODEC",
            "covers": ["VO-LEGACY"],
            "target": {
                "kind": "locator",
                "value": {"path": "tests/legacy.rs", "item_path": "fixture::target"}
            },
            "intent": "legacy intent",
            "kind": "unit-normal",
            "location": {
                "file": "tests/legacy.rs",
                "function": "fixture::test_case",
                "start_line": 1,
                "end_line": 3,
                "start_byte": 0,
                "end_byte": 10
            },
            "filter": "fixture::test_case",
            "package": "legacy-package",
            "test_target": {"kind": "integration_test", "name": "legacy"}
        });
        let adapter = RustCargoAdapter;
        let drafts = adapter
            .decode_test(legacy.to_string().as_bytes())
            .expect("legacy payload decodes");
        assert_eq!(drafts.len(), 1);
        assert_eq!(drafts[0].targets[0].adapter.as_str(), "rust-cargo");
        assert_eq!(
            drafts[0].targets[0].value,
            "tests/legacy.rs::fixture::target"
        );
        assert_eq!(drafts[0].execution.selector, "fixture::test_case");
        assert_eq!(
            drafts[0]
                .execution
                .suite
                .as_ref()
                .map(|suite| suite.kind.as_str()),
            Some("integration")
        );
        let entity = adapter
            .materialize_test(&drafts[0])
            .expect("legacy draft materializes");
        assert_eq!(entity.targets, drafts[0].targets);
        assert_eq!(entity.execution.project.as_deref(), Some("legacy-package"));
    }

    #[test]
    fn multi_target_wire_output_omits_legacy_singular_target() {
        let test = vtest_model::TestEntity {
            id: vtest_model::TestId::new("TEST-MULTI-CODEC"),
            covers: vec![vtest_model::VoId::new("VO-MULTI")],
            targets: vec![
                vtest_model::NeutralTargetRef::new("rust-cargo", "src/lib.rs::one"),
                vtest_model::NeutralTargetRef::new("rust-cargo", "src/lib.rs::two"),
            ],
            intent: "multi".to_owned(),
            input: None,
            expect: None,
            kind: Some("integration-normal".to_owned()),
            cases: Vec::new(),
            related: Vec::new(),
            location: vtest_model::SourceLocation::legacy("tests/multi.rs", "multi", 1, 2, 0, 10),
            content_hash: vtest_model::ContentHash::from_text("multi"),
            execution: vtest_model::ExecutionDescriptor {
                adapter: vtest_model::AdapterId::from("rust-cargo"),
                project: Some("multi".to_owned()),
                suite: Some(vtest_model::TestSuite {
                    kind: "integration".to_owned(),
                    name: Some("multi".to_owned()),
                }),
                selector: "multi".to_owned(),
                runner: Some("cargo-test".to_owned()),
                working_root: None,
            },
        };
        let encoded = RustCargoAdapter
            .encode_test(&test)
            .expect("multi-target payload encodes");
        assert!(encoded.get("targets").is_some());
        assert!(encoded.get("target").is_none());
    }
}
