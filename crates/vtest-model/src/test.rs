use crate::{ContentHash, SourceLocation, TargetRef, TestId, VoId};
use serde::{Deserialize, Serialize};

// TODO: Review fail-closed handling of `Unknown`.
// Execution must not silently fall back to an unscoped Cargo target.

/// Identifies the Rust/Cargo target that contains a test.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "name", rename_all = "snake_case")]
pub enum TestTarget {
    Lib,
    Bin(String),
    IntegrationTest(String),
    Unknown,
}

// TODO: Split canonical test data from discovery and Rust/Cargo execution metadata.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct TestEntity {
    pub id: TestId,
    pub covers: Vec<VoId>,
    pub target: TargetRef,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub additional_targets: Vec<TargetRef>,
    pub intent: String,
    pub input: Option<String>,
    pub expect: Option<String>,
    pub kind: Option<String>,
    pub cases: Vec<String>,
    pub related: Vec<TestId>,
    pub location: SourceLocation,
    pub content_hash: ContentHash,
    pub filter: String,
    pub package: String,
    pub test_target: TestTarget,
}

/// Canonical, adapter-neutral metadata record for a test.
///
/// The fields are the normalized logical fields a source discovery adapter
/// produces for a test. Cardinality requirements on `targets` belong to the
/// adapter, not to this record.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct TestRecord {
    pub id: TestId,
    pub covers: Vec<VoId>,
    pub targets: Vec<TargetRef>,
    pub intent: String,
    pub input: Option<String>,
    pub expect: Option<String>,
    pub kind: Option<String>,
    pub cases: Vec<String>,
    pub related: Vec<TestId>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{AdapterId, Locator};

    fn populated_record() -> TestRecord {
        TestRecord {
            id: TestId::new("TEST-001"),
            covers: vec![VoId::new("VO-001")],
            targets: vec![TargetRef::Locator(Locator {
                adapter: AdapterId::new("rust-cargo"),
                value: "src/lib.rs::module::function".to_string(),
            })],
            intent: "The parser rejects an unknown key.".to_string(),
            input: Some("an unknown key".to_string()),
            expect: Some("E-SCAN-006".to_string()),
            kind: Some("unit-normal".to_string()),
            cases: vec!["unknown key".to_string()],
            related: vec![TestId::new("TEST-002")],
        }
    }

    #[test]
    fn test_record_carries_the_normalized_logical_fields() {
        let value = serde_json::to_value(populated_record()).unwrap();
        let mut keys: Vec<&String> = value.as_object().unwrap().keys().collect();
        keys.sort();

        let mut expected = vec![
            "id", "covers", "targets", "intent", "input", "expect", "kind", "cases", "related",
        ];
        expected.sort();

        assert_eq!(keys, expected);
    }

    #[test]
    fn test_record_round_trips_without_the_optional_fields() {
        let record = TestRecord {
            input: None,
            expect: None,
            kind: None,
            ..populated_record()
        };

        let json = serde_json::to_string(&record).unwrap();
        assert_eq!(serde_json::from_str::<TestRecord>(&json).unwrap(), record);
    }

    #[test]
    fn test_record_round_trips_with_empty_lists() {
        let record = TestRecord {
            covers: vec![],
            targets: vec![],
            cases: vec![],
            related: vec![],
            ..populated_record()
        };

        let json = serde_json::to_string(&record).unwrap();
        assert_eq!(serde_json::from_str::<TestRecord>(&json).unwrap(), record);
    }
}
