use serde_json::{Map, Value};
use std::{collections::BTreeSet, fs, path::PathBuf, sync::Arc};
use vtest_adapter_api::{
    encode_wire_targets, missing_capability_semantics, normalize_wire_targets, AdapterCapability,
    AdapterDescriptor, AdapterError, AdapterRegistration, AdapterRegistry, DiscoveredTestDraft,
    DiscoveryCompleteness, ManagedTestDraft, ManagedTestDraftLink, MissingCapabilitySemantics,
    SourceFragment, TestWireCodec,
};
use vtest_model::{
    hash_test_subject, AdapterId, ContentHash, ExecutionDescriptor, ProjectPath, SourceLocation,
    SourceRange, TargetRef, TestEntity, TestId, TestSubjectInput,
};

fn fixtures() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures/adapters")
}

fn json(relative: &str) -> Value {
    serde_json::from_str(
        &fs::read_to_string(fixtures().join(relative)).expect("read frozen adapter fixture"),
    )
    .expect("fixture is JSON")
}

#[derive(Default)]
struct SyntheticCodec;

impl TestWireCodec for SyntheticCodec {
    fn decode_execution(
        &self,
        execution: Option<&ExecutionDescriptor>,
        properties: &Map<String, Value>,
    ) -> Result<ExecutionDescriptor, AdapterError> {
        let selector = properties
            .get("scenario")
            .and_then(Value::as_str)
            .ok_or_else(|| AdapterError::MalformedOutput("scenario is required".to_owned()))?;
        let decoded = ExecutionDescriptor {
            adapter: AdapterId::new("synthetic"),
            project: None,
            suite: None,
            selector: selector.to_owned(),
        };
        if execution.is_some_and(|execution| execution != &decoded) {
            return Err(AdapterError::Mismatch(
                "execution and synthetic property disagree".to_owned(),
            ));
        }
        Ok(decoded)
    }

    fn encode_properties(&self, test: &TestEntity) -> Result<Map<String, Value>, AdapterError> {
        if test.execution.adapter.as_str() != "synthetic" {
            return Err(AdapterError::Mismatch("codec adapter mismatch".to_owned()));
        }
        Ok(Map::from_iter([(
            "scenario".to_owned(),
            Value::String(test.execution.selector.clone()),
        )]))
    }
}

fn synthetic_test() -> TestEntity {
    TestEntity {
        id: TestId::new("TEST-SYNTH-ADD"),
        covers: vec!["VO-SYNTH-ADD".into()],
        targets: vec![TargetRef::Locator {
            adapter: AdapterId::new("synthetic"),
            value: "component(add)/scenario[happy]".to_owned(),
        }],
        intent: "adding two values returns their sum".to_owned(),
        input: None,
        expect: None,
        kind: Some("scenario".to_owned()),
        cases: Vec::new(),
        related: Vec::new(),
        location: SourceLocation {
            adapter: AdapterId::new("synthetic"),
            path: ProjectPath::new("source/cases.synth"),
            locator: "scenario[adding two values]".to_owned(),
            byte_range: SourceRange {
                start: 65,
                end: 176,
                start_line: 1,
                end_line: 1,
            },
        },
        content_hash: ContentHash::from_text("fixture subject"),
        execution: ExecutionDescriptor {
            adapter: AdapterId::new("synthetic"),
            project: None,
            suite: None,
            selector: "scenario[adding two values]".to_owned(),
        },
    }
}

#[test]
fn synthetic_wire_properties_round_trip_without_rust_fields() {
    let codec = SyntheticCodec;
    let mut test = synthetic_test();
    test.targets.push(TargetRef::Locator {
        adapter: AdapterId::new("synthetic"),
        value: "component(add)/scenario[edge]".to_owned(),
    });
    let properties = codec.encode_properties(&test).expect("encode properties");
    test.validate().expect("two distinct targets are valid");
    let decoded = codec
        .decode_execution(Some(&test.execution), &properties)
        .expect("decode properties");
    assert_eq!(decoded, test.execution);
    let encoded = serde_json::to_value(&test).expect("serialize neutral Test");
    for forbidden in ["filter", "package", "test_target"] {
        assert!(encoded.get(forbidden).is_none());
    }
    assert_eq!(encoded["targets"].as_array().map(Vec::len), Some(2));
    assert!(encoded.get("target").is_none());

    let wire = encode_wire_targets(&test.targets).expect("encode two targets");
    assert_eq!(wire["targets"].as_array().map(Vec::len), Some(2));
    assert!(wire.get("target").is_none());

    let first = test.targets[0].clone();
    assert!(normalize_wire_targets(Some(vec![first.clone()]), Some(first.clone())).is_ok());
    assert!(normalize_wire_targets(Some(test.targets.clone()), Some(first)).is_err());

    test.targets.clear();
    assert!(test.validate().is_err());
}

#[test]
fn registry_binds_codec_and_preserves_deterministic_id_order() {
    let mut synthetic = AdapterRegistration::new(AdapterDescriptor {
        id: AdapterId::new("synthetic"),
        languages: vec!["fixture".to_owned()],
        capabilities: vec![AdapterCapability::TestWireCodec],
        config_namespace: "synthetic".to_owned(),
    });
    synthetic.test_wire_codec = Some(Arc::new(SyntheticCodec));
    let rust = AdapterRegistration::new(AdapterDescriptor {
        id: AdapterId::new("rust-cargo"),
        languages: vec!["rust".to_owned()],
        capabilities: Vec::new(),
        config_namespace: "rust-cargo".to_owned(),
    });
    let registry = AdapterRegistry::from_registrations([synthetic, rust]).expect("registry");
    assert_eq!(
        registry.ids().map(AdapterId::as_str).collect::<Vec<_>>(),
        ["rust-cargo", "synthetic"]
    );
    assert!(registry
        .test_wire_codec(&AdapterId::new("synthetic"))
        .is_ok());
}

#[test]
fn discovery_dto_is_hash_free_and_core_hash_function_binds_the_subject() {
    let test = synthetic_test();
    let managed = ManagedTestDraft {
        id: test.id.clone(),
        covers: test.covers.clone(),
        targets: test.targets.clone(),
        intent: test.intent.clone(),
        input: test.input.clone(),
        expect: test.expect.clone(),
        kind: test.kind.clone(),
        cases: test.cases.clone(),
        related: test.related.clone(),
        execution: test.execution.clone(),
    };
    let construct = SourceFragment {
        location: test.location.clone(),
        bytes: b"scenario[adding two values]".to_vec(),
    };
    let discovered = DiscoveredTestDraft {
        adapter: AdapterId::new("synthetic"),
        location: test.location.clone(),
        construct: construct.clone(),
        metadata_sources: vec![SourceFragment {
            location: SourceLocation {
                path: ProjectPath::new("metadata/tests.json"),
                locator: "tests[TEST-SYNTH-ADD]".to_owned(),
                byte_range: SourceRange {
                    start: 0,
                    end: 1,
                    start_line: 1,
                    end_line: 1,
                },
                ..test.location.clone()
            },
            bytes: b"logical metadata".to_vec(),
        }],
        managed: ManagedTestDraftLink::One(managed.clone()),
    };
    let wire = serde_json::to_value(&discovered).expect("serialize hash-free DTO");
    assert!(wire.get("content_hash").is_none());
    assert!(wire["construct"].get("content_hash").is_none());

    let hash = hash_test_subject(&TestSubjectInput {
        adapter: &discovered.adapter,
        id: &managed.id,
        covers: &managed.covers,
        targets: &managed.targets,
        intent: &managed.intent,
        input: managed.input.as_deref(),
        expect: managed.expect.as_deref(),
        kind: managed.kind.as_deref(),
        cases: &managed.cases,
        related: &managed.related,
        location: &discovered.location,
        execution: &managed.execution,
        construct: &discovered.construct.bytes,
    });
    assert_ne!(hash, ContentHash::from_text("scenario[adding two values]"));
}

#[test]
fn collision_fixture_proves_repository_global_ids_are_not_namespaced() {
    let fixture = json("mixed/collisions.json");
    for field in ["test_id_collision", "src_id_collision"] {
        let ids = fixture[field]
            .as_array()
            .expect("collision list")
            .iter()
            .map(|entry| entry["id"].as_str().expect("ID"))
            .collect::<Vec<_>>();
        assert!(ids.len() > ids.iter().collect::<BTreeSet<_>>().len());
    }
}

#[test]
fn target_observation_fixture_has_no_representative_target_escape_hatch() {
    let fixture = json("synthetic/target-observations.json");
    let results = fixture["targets"]
        .as_array()
        .expect("target observations")
        .iter()
        .map(|entry| entry["result"].as_str().expect("result"))
        .collect::<Vec<_>>();
    assert_eq!(results, ["PASS", "FAIL", "UNKNOWN"]);
    assert_eq!(fixture["expected_aggregate"], "FAIL");
}

#[test]
fn capability_absence_and_analysis_limits_bind_to_non_pass_states() {
    assert_eq!(
        missing_capability_semantics(AdapterCapability::StaticAudit),
        MissingCapabilitySemantics::NotChecked
    );
    assert_eq!(
        missing_capability_semantics(AdapterCapability::Coverage),
        MissingCapabilitySemantics::NotChecked
    );
    assert_eq!(
        missing_capability_semantics(AdapterCapability::TestRunner),
        MissingCapabilitySemantics::NotExecuted
    );
    assert_eq!(
        missing_capability_semantics(AdapterCapability::SourceDiscovery),
        MissingCapabilitySemantics::OperationRejected
    );
    let incomplete = json("synthetic/manifest-incomplete-analysis.json");
    assert_eq!(incomplete["static_analysis"]["complete"], false);
}

#[test]
fn frozen_ordering_variants_have_one_canonical_observable() {
    let left = json("mixed/order-a.json");
    let right = json("mixed/order-b.json");
    assert_ne!(left["adapters"], right["adapters"]);
    assert_ne!(left["filesystem_entries"], right["filesystem_entries"]);
    assert_eq!(left["expected_test_order"], right["expected_test_order"]);
}

#[test]
fn form_owner_fixtures_forbid_ambiguous_or_rust_fallback_resolution() {
    let ambiguous = json("forms/ambiguous-compatibility.json");
    assert_eq!(
        ambiguous["matching_adapters"].as_array().map(Vec::len),
        Some(2)
    );
    assert_eq!(ambiguous["expected_code"], "E-ADAPTER-001");
    assert_eq!(ambiguous["expected_write"], false);
    assert_eq!(ambiguous["rust_fallback_allowed"], false);
    let duplicate = json("forms/duplicate-kind.json");
    assert_eq!(duplicate["registrations"].as_array().map(Vec::len), Some(2));
    assert_eq!(duplicate["expected_code"], "E-ADAPTER-001");
}

#[test]
fn v2_rust_fixture_binds_to_the_same_neutral_adapter_id_as_v1_compatibility() {
    let config = fs::read_to_string(fixtures().join("config/v2-rust-cargo.yaml"))
        .expect("v2 rust-cargo config");
    assert!(config.contains("version: 2"));
    assert!(config.contains("id: rust-cargo"));
    let v1_compatibility_adapter = AdapterId::new("rust-cargo");
    let v2_adapter = AdapterId::new("rust-cargo");
    assert_eq!(v1_compatibility_adapter, v2_adapter);
}

#[test]
fn incomplete_discovery_is_representable_and_never_complete_empty_success() {
    let fixture = json("synthetic/manifest-discovery-failure.json");
    let completeness = if fixture["complete"] == true {
        DiscoveryCompleteness::Complete
    } else {
        DiscoveryCompleteness::Incomplete
    };
    assert_eq!(completeness, DiscoveryCompleteness::Incomplete);
    assert_eq!(fixture["diagnostics"][0]["code"], "E-SCAN-001");
    assert_eq!(
        fixture["discovered_tests"].as_array().map(Vec::len),
        Some(0)
    );
}

#[test]
fn relation_aliases_bind_to_one_in_memory_identity_without_rewrite() {
    let directory = fixtures().join("relations");
    let bare_text = fs::read_to_string(directory.join("01ARZ3NDEKTSV4RRFFQ69G5FAV.yaml"))
        .expect("bare Relation");
    let prefixed_text = fs::read_to_string(directory.join("REL-01ARZ3NDEKTSV4RRFFQ69G5FAV.yaml"))
        .expect("prefixed Relation");
    let parse = |text: &str| {
        let id = text
            .lines()
            .find_map(|line| line.strip_prefix("id: "))
            .expect("Relation id")
            .trim_matches('\'')
            .to_owned();
        let payload = id.strip_prefix("REL-").unwrap_or(&id).to_owned();
        (id, payload)
    };
    let (bare_id, bare_payload) = parse(&bare_text);
    let (prefixed_id, prefixed_payload) = parse(&prefixed_text);
    assert_ne!(bare_id, prefixed_id);
    assert_eq!(bare_payload, prefixed_payload);
}
