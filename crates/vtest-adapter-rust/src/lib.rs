//! Built-in `rust-cargo` adapter contracts.
//!
//! W2 owns the descriptor and version 1 Test JSON compatibility codec. Rust
//! parsing, static analysis, execution, and coverage capabilities are added by
//! their owning waves without leaking Cargo-specific values into the model.

use serde_json::{Map, Value};
use std::sync::Arc;
use vtest_adapter_api::{
    AdapterCapability, AdapterDescriptor, AdapterError, AdapterRegistration, TestWireCodec,
};
use vtest_model::{AdapterId, ExecutionDescriptor, TestEntity, TestSuite};

mod discovery;
pub use discovery::{Locator, RustCargoDiscovery};

pub const RUST_CARGO_ADAPTER_ID: &str = "rust-cargo";

#[derive(Default)]
pub struct RustCargoCodec;

impl TestWireCodec for RustCargoCodec {
    fn decode_execution(
        &self,
        execution: Option<&ExecutionDescriptor>,
        properties: &Map<String, Value>,
    ) -> Result<ExecutionDescriptor, AdapterError> {
        let has_compatibility = ["filter", "package", "test_target"]
            .iter()
            .any(|key| properties.contains_key(*key));
        let decoded = if has_compatibility {
            Some(decode_compatibility_execution(properties)?)
        } else {
            None
        };
        match (execution, decoded) {
            (Some(execution), Some(decoded)) if execution != &decoded => {
                Err(AdapterError::Mismatch(
                    "execution and rust-cargo compatibility fields disagree".to_owned(),
                ))
            }
            (Some(execution), _) if execution.adapter.as_str() != RUST_CARGO_ADAPTER_ID => {
                Err(AdapterError::Mismatch(
                    "rust-cargo codec received another adapter's execution descriptor".to_owned(),
                ))
            }
            (Some(execution), _) => Ok(execution.clone()),
            (None, Some(decoded)) => Ok(decoded),
            (None, None) => Err(AdapterError::MalformedOutput(
                "Rust Test wire input has neither execution nor complete compatibility fields"
                    .to_owned(),
            )),
        }
    }

    fn encode_properties(&self, test: &TestEntity) -> Result<Map<String, Value>, AdapterError> {
        if test.execution.adapter.as_str() != RUST_CARGO_ADAPTER_ID {
            return Err(AdapterError::Mismatch(
                "rust-cargo codec received another adapter's Test".to_owned(),
            ));
        }
        let mut properties = Map::new();
        properties.insert(
            "filter".to_owned(),
            Value::String(test.execution.selector.clone()),
        );
        properties.insert(
            "package".to_owned(),
            test.execution
                .project
                .as_ref()
                .map(|value| Value::String(value.clone()))
                .unwrap_or(Value::Null),
        );
        properties.insert(
            "test_target".to_owned(),
            suite_to_compatibility(test.execution.suite.as_ref())?,
        );
        Ok(properties)
    }
}

pub fn rust_cargo_registration() -> AdapterRegistration {
    let mut registration = AdapterRegistration::new(AdapterDescriptor {
        id: AdapterId::new(RUST_CARGO_ADAPTER_ID),
        languages: vec!["rust".to_owned()],
        capabilities: vec![AdapterCapability::TestWireCodec],
        config_namespace: RUST_CARGO_ADAPTER_ID.to_owned(),
    });
    registration.test_wire_codec = Some(Arc::new(RustCargoCodec));
    registration
}

fn decode_compatibility_execution(
    properties: &Map<String, Value>,
) -> Result<ExecutionDescriptor, AdapterError> {
    let filter = required_string(properties, "filter")?;
    let project = match properties.get("package") {
        Some(Value::String(value)) if !value.is_empty() => Some(value.clone()),
        Some(Value::Null) => None,
        _ => {
            return Err(AdapterError::MalformedOutput(
                "rust-cargo compatibility package must be a non-empty string or null".to_owned(),
            ))
        }
    };
    let suite = compatibility_to_suite(properties.get("test_target").ok_or_else(|| {
        AdapterError::MalformedOutput("rust-cargo compatibility test_target is required".to_owned())
    })?)?;
    Ok(ExecutionDescriptor {
        adapter: AdapterId::new(RUST_CARGO_ADAPTER_ID),
        project,
        suite: Some(suite),
        selector: filter,
    })
}

fn required_string(properties: &Map<String, Value>, key: &str) -> Result<String, AdapterError> {
    properties
        .get(key)
        .and_then(Value::as_str)
        .filter(|value| !value.is_empty())
        .map(str::to_owned)
        .ok_or_else(|| {
            AdapterError::MalformedOutput(format!(
                "rust-cargo compatibility {key} must be a non-empty string"
            ))
        })
}

fn compatibility_to_suite(value: &Value) -> Result<TestSuite, AdapterError> {
    let object = value.as_object().ok_or_else(|| {
        AdapterError::MalformedOutput(
            "rust-cargo compatibility test_target must be an object".to_owned(),
        )
    })?;
    let kind = object.get("kind").and_then(Value::as_str).ok_or_else(|| {
        AdapterError::MalformedOutput(
            "rust-cargo compatibility test_target.kind is required".to_owned(),
        )
    })?;
    let name = object
        .get("name")
        .and_then(Value::as_str)
        .map(str::to_owned);
    let (kind, name) = match (kind, name) {
        ("lib", None) => ("lib".to_owned(), None),
        ("bin", Some(name)) if !name.is_empty() => ("bin".to_owned(), Some(name)),
        ("integration_test", Some(name)) if !name.is_empty() => {
            ("integration".to_owned(), Some(name))
        }
        _ => {
            return Err(AdapterError::MalformedOutput(
                "rust-cargo compatibility test_target kind/name combination is invalid".to_owned(),
            ))
        }
    };
    Ok(TestSuite { kind, name })
}

fn suite_to_compatibility(suite: Option<&TestSuite>) -> Result<Value, AdapterError> {
    let suite = suite.ok_or_else(|| {
        AdapterError::MalformedOutput("rust-cargo execution suite is required".to_owned())
    })?;
    let (kind, name) = match (suite.kind.as_str(), suite.name.as_ref()) {
        ("lib", None) => ("lib", None),
        ("bin", Some(name)) if !name.is_empty() => ("bin", Some(name.clone())),
        ("integration", Some(name)) if !name.is_empty() => ("integration_test", Some(name.clone())),
        _ => {
            return Err(AdapterError::MalformedOutput(
                "rust-cargo execution suite kind/name combination is invalid".to_owned(),
            ))
        }
    };
    let mut target = Map::new();
    target.insert("kind".to_owned(), Value::String(kind.to_owned()));
    if let Some(name) = name {
        target.insert("name".to_owned(), Value::String(name));
    }
    Ok(Value::Object(target))
}

#[cfg(test)]
mod tests {
    use super::*;
    use vtest_model::{ContentHash, ProjectPath, SourceLocation, SourceRange, TargetRef, TestId};

    fn test_entity() -> TestEntity {
        TestEntity {
            id: TestId::new("TEST-RUST-CODEC"),
            covers: vec!["VO-RUST-CODEC".into()],
            targets: vec![TargetRef::Locator {
                adapter: AdapterId::new(RUST_CARGO_ADAPTER_ID),
                value: "src/lib.rs::parse".to_owned(),
            }],
            intent: "codec compatibility".to_owned(),
            input: None,
            expect: None,
            kind: None,
            cases: Vec::new(),
            related: Vec::new(),
            location: SourceLocation {
                adapter: AdapterId::new(RUST_CARGO_ADAPTER_ID),
                path: ProjectPath::new("tests/parser.rs"),
                locator: "tests::parses".to_owned(),
                byte_range: SourceRange {
                    start: 0,
                    end: 1,
                    start_line: 1,
                    end_line: 1,
                },
            },
            content_hash: ContentHash::from_text("test"),
            execution: ExecutionDescriptor {
                adapter: AdapterId::new(RUST_CARGO_ADAPTER_ID),
                project: Some("parser".to_owned()),
                suite: Some(TestSuite {
                    kind: "integration".to_owned(),
                    name: Some("parser".to_owned()),
                }),
                selector: "tests::parses".to_owned(),
            },
        }
    }

    #[test]
    fn version_one_compatibility_round_trips_losslessly() {
        let codec = RustCargoCodec;
        let test = test_entity();
        let properties = codec.encode_properties(&test).unwrap();
        assert_eq!(properties["filter"], "tests::parses");
        assert_eq!(properties["package"], "parser");
        assert_eq!(properties["test_target"]["kind"], "integration_test");
        assert_eq!(
            codec.decode_execution(None, &properties).unwrap(),
            test.execution
        );
        assert_eq!(
            codec
                .decode_execution(Some(&test.execution), &properties)
                .unwrap(),
            test.execution
        );
    }

    #[test]
    fn compatibility_contradictions_and_partial_fields_are_rejected() {
        let codec = RustCargoCodec;
        let test = test_entity();
        let mut properties = codec.encode_properties(&test).unwrap();
        properties.insert("filter".to_owned(), Value::String("other".to_owned()));
        assert!(matches!(
            codec.decode_execution(Some(&test.execution), &properties),
            Err(AdapterError::Mismatch(_))
        ));
        properties.remove("package");
        assert!(matches!(
            codec.decode_execution(None, &properties),
            Err(AdapterError::MalformedOutput(_))
        ));
    }
}
