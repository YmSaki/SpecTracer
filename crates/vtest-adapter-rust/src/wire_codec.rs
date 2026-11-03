//! BD-195: the `rust-cargo` codec owns the version-1 compatibility fields
//! `filter`, `package` and `test_target`. They are derived from, and read
//! back into, the language-neutral `ExecutionDescriptor` only; the core
//! model never holds them (BD-192, DES-561).
//!
//! Version-1 shapes: `package` is the Cargo package (`execution.project`),
//! `filter` is the test filter (`execution.selector`), and `test_target`
//! is `{"kind": "lib"}`, `{"kind": "bin", "name": ...}` or
//! `{"kind": "integration_test", "name": ...}` (`execution.suite`).

use serde_json::{json, Value};
use vtest_adapter_api::{TestWireCodec, WireFields};
use vtest_model::{AdapterId, ExecutionDescriptor, TestSuite};

use crate::ADAPTER_ID;

const FIELDS: &[&str] = &["filter", "package", "test_target"];

pub struct RustCargoWireCodec;

impl TestWireCodec for RustCargoWireCodec {
    fn id(&self) -> &'static str {
        ADAPTER_ID
    }

    fn field_names(&self) -> &'static [&'static str] {
        FIELDS
    }

    /// DS-967/968: all three fields, or none when the descriptor is not a
    /// complete `rust-cargo` coordinate (no empty or dummy value).
    fn encode(&self, execution: &ExecutionDescriptor) -> WireFields {
        let mut fields = WireFields::new();
        if execution.adapter.as_str() != ADAPTER_ID {
            return fields;
        }
        let (Some(package), Some(suite)) = (&execution.project, &execution.suite) else {
            return fields;
        };
        let test_target = match (suite.kind.as_str(), &suite.name) {
            ("lib", None) => json!({"kind": "lib"}),
            ("bin", Some(name)) => json!({"kind": "bin", "name": name}),
            ("integration", Some(name)) => json!({"kind": "integration_test", "name": name}),
            _ => return fields,
        };
        fields.insert("filter".to_owned(), json!(execution.selector));
        fields.insert("package".to_owned(), json!(package));
        fields.insert("test_target".to_owned(), test_target);
        fields
    }

    /// DS-1578/1579: only a complete, consistent field set derives a
    /// descriptor.
    fn decode(&self, fields: &WireFields) -> Result<ExecutionDescriptor, String> {
        let text = |name: &str| match fields.get(name) {
            Some(Value::String(value)) if !value.is_empty() => Ok(value.clone()),
            Some(_) => Err(format!(
                "compatibility field `{name}` must be a non-empty string"
            )),
            None => Err(format!("compatibility field `{name}` is missing")),
        };
        let filter = text("filter")?;
        let package = text("package")?;
        let Some(Value::Object(target)) = fields.get("test_target") else {
            return Err("compatibility field `test_target` must be an object".to_owned());
        };
        let name = target.get("name");
        if target.keys().any(|key| key != "kind" && key != "name") {
            return Err("`test_target` has an undeclared field".to_owned());
        }
        let suite = match (target.get("kind").and_then(Value::as_str), name) {
            (Some("lib"), None) => TestSuite {
                kind: "lib".to_owned(),
                name: None,
            },
            (Some(kind @ ("bin" | "integration_test")), Some(Value::String(name)))
                if !name.is_empty() =>
            {
                TestSuite {
                    kind: if kind == "bin" { "bin" } else { "integration" }.to_owned(),
                    name: Some(name.clone()),
                }
            }
            _ => {
                return Err(
                    "`test_target` is not a runnable lib / bin / integration_test".to_owned(),
                )
            }
        };
        Ok(ExecutionDescriptor {
            adapter: AdapterId::new(ADAPTER_ID),
            project: Some(package),
            suite: Some(suite),
            selector: filter,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use vtest_adapter_api::{decode_test_json, encode_test_json, AdapterRegistry};
    use vtest_model::{
        ContentHash, Locator, ProjectPath, SourceLocation, SourceRange, TargetRef, TestEntity,
        TestId,
    };

    fn registry() -> AdapterRegistry {
        let mut registry = AdapterRegistry::new();
        registry
            .register(Box::new(crate::RustCargoAdapter::new()))
            .expect("register rust-cargo");
        registry
    }

    fn test(adapter: &str, suite: Option<TestSuite>, targets: usize) -> TestEntity {
        TestEntity {
            id: TestId::new("TEST-WIRE"),
            covers: Vec::new(),
            targets: (0..targets)
                .map(|index| {
                    TargetRef::Locator(Locator {
                        adapter: AdapterId::new(adapter),
                        value: format!("src/lib.rs::target{index}"),
                    })
                })
                .collect(),
            intent: "wire".to_owned(),
            input: None,
            expect: None,
            kind: None,
            cases: Vec::new(),
            related: Vec::new(),
            location: SourceLocation {
                adapter: AdapterId::new(adapter),
                path: ProjectPath("tests/wire.rs".to_owned()),
                locator: "wire".to_owned(),
                byte_range: SourceRange { start: 0, end: 1 },
            },
            content_hash: ContentHash::from_text("wire"),
            execution: ExecutionDescriptor {
                adapter: AdapterId::new(adapter),
                project: Some("fixture".to_owned()),
                suite,
                selector: "module::wire".to_owned(),
            },
        }
    }

    fn integration() -> Option<TestSuite> {
        Some(TestSuite {
            kind: "integration".to_owned(),
            name: Some("registered".to_owned()),
        })
    }

    /// DS-1574/DS-967/DS-968/DS-1577: `execution` is always written; only a
    /// rust-cargo Test gets `filter` / `package` / `test_target`, and a
    /// Test of another adapter gets none of them (no empty or dummy value).
    /// The fields round-trip back into the same `TestEntity`.
    /// @vtest.id TEST-ADAPTER-RUST-WIRE-CODEC-COMPAT-FIELDS
    /// @vtest.covers VO-ADAPTER-WIRE-CODEC-COMPAT-FIELDS-RUST-ONLY
    /// @vtest.target crates/vtest-adapter-api/src/wire.rs::encode_test_json
    /// @vtest.intent rust-cargo Tests carry round-tripping compat fields and other adapters carry none
    #[test]
    fn compat_fields_are_written_for_rust_cargo_tests_only_and_round_trip() {
        let registry = registry();
        for suite in [
            integration(),
            Some(TestSuite {
                kind: "lib".to_owned(),
                name: None,
            }),
            Some(TestSuite {
                kind: "bin".to_owned(),
                name: Some("tool".to_owned()),
            }),
        ] {
            let original = test("rust-cargo", suite, 2);
            let json = encode_test_json(&original, &registry);
            assert!(json.get("execution").is_some());
            assert_eq!(json["filter"], "module::wire");
            assert_eq!(json["package"], "fixture");
            assert!(json["test_target"]["kind"].is_string());
            assert_eq!(decode_test_json(&json, &registry), Ok(original));
        }
        assert_eq!(
            encode_test_json(&test("rust-cargo", integration(), 1), &registry)["test_target"],
            json!({"kind": "integration_test", "name": "registered"})
        );

        let other = test("synthetic", integration(), 2);
        let json = encode_test_json(&other, &registry);
        for field in FIELDS {
            assert!(json.get(*field).is_none(), "{field} on a non-Rust Test");
        }
        assert_eq!(decode_test_json(&json, &registry), Ok(other));

        // An incomplete rust-cargo coordinate gets no partial field set.
        let incomplete = test("rust-cargo", None, 2);
        let json = encode_test_json(&incomplete, &registry);
        for field in FIELDS {
            assert!(
                json.get(*field).is_none(),
                "{field} on an incomplete coordinate"
            );
        }
    }

    /// DS-1578: without `execution`, only complete and consistent
    /// compatibility fields derive the rust-cargo descriptor.
    /// @vtest.id TEST-ADAPTER-RUST-WIRE-CODEC-DERIVES-EXECUTION
    /// @vtest.covers VO-ADAPTER-WIRE-CODEC-DERIVES-EXECUTION
    /// @vtest.target crates/vtest-adapter-api/src/wire.rs::decode_test_json
    /// @vtest.intent a Test without execution is derived from complete compat fields and rejected otherwise
    #[test]
    fn execution_is_derived_only_from_complete_compat_fields() {
        let registry = registry();
        let original = test("rust-cargo", integration(), 2);
        let mut json = encode_test_json(&original, &registry);
        json.as_object_mut().unwrap().remove("execution");
        assert_eq!(decode_test_json(&json, &registry), Ok(original));

        for missing in FIELDS {
            let mut partial = json.clone();
            partial.as_object_mut().unwrap().remove(*missing);
            assert!(decode_test_json(&partial, &registry).is_err(), "{missing}");
        }
        let mut unknown = json.clone();
        unknown["test_target"] = json!({"kind": "unknown"});
        assert!(decode_test_json(&unknown, &registry).is_err());
        let mut none = json.clone();
        for field in FIELDS {
            none.as_object_mut().unwrap().remove(*field);
        }
        assert!(decode_test_json(&none, &registry).is_err());
    }

    /// DS-1579/DS-974/DS-1674: contradicting `execution` and compat fields,
    /// compat fields on a non-Rust Test, and a singular `target` that does
    /// not equal the only `targets` entry are all rejected.
    /// @vtest.id TEST-ADAPTER-RUST-WIRE-CODEC-REJECTS-CONTRADICTION
    /// @vtest.covers VO-ADAPTER-WIRE-CODEC-REJECTS-CONTRADICTION, VO-ADAPTER-WIRE-SINGULAR-TARGET
    /// @vtest.target crates/vtest-adapter-api/src/wire.rs::decode_test_json
    /// @vtest.intent contradicting compat fields, compat fields on other adapters and a mismatched singular target are rejected
    #[test]
    fn contradicting_or_misplaced_compat_fields_are_rejected() {
        let registry = registry();
        let json = encode_test_json(&test("rust-cargo", integration(), 1), &registry);
        for (field, value) in [
            ("filter", json!("other::wire")),
            ("package", json!("other")),
            ("test_target", json!({"kind": "lib"})),
        ] {
            let mut contradicted = json.clone();
            contradicted[field] = value;
            assert!(
                decode_test_json(&contradicted, &registry).is_err(),
                "{field}"
            );
        }

        let mut misplaced = encode_test_json(&test("synthetic", integration(), 2), &registry);
        misplaced["filter"] = json!("module::wire");
        assert!(decode_test_json(&misplaced, &registry).is_err());

        assert_eq!(json["target"], json["targets"][0]);
        let mut singular = json.clone();
        singular["target"] =
            json!({"locator": {"adapter": "rust-cargo", "value": "src/lib.rs::x"}});
        assert!(decode_test_json(&singular, &registry).is_err());
        let two = encode_test_json(&test("rust-cargo", integration(), 2), &registry);
        assert!(two.get("target").is_none());
        let mut forced = two.clone();
        forced["target"] = two["targets"][0].clone();
        assert!(decode_test_json(&forced, &registry).is_err());
    }
}
