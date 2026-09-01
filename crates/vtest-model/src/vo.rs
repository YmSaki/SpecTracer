use crate::{DerivesFrom, VoId};
use serde::{Deserialize, Serialize};

/// Defines a verification dimension and its allowed partitions.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct Dimension {
    pub name: String,
    pub partitions: Vec<String>,
}

/// Defines how dimension coverage is evaluated for a verification objective.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum CoveragePolicy {
    /// Each dimension is covered independently.
    IndependentAxes,

    /// Every possible combination of dimension partitions must be covered.
    FullProduct,

    /// Only explicitly listed combinations are required.
    Explicit,
}

/// Canonical metadata record for a verification objective.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct VoRecord {
    pub id: VoId,
    pub parent: Option<VoId>,

    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub derives_from: Vec<DerivesFrom>,

    pub claim: String,
    pub dimensions: Vec<Dimension>,
    pub coverage_policy: Option<CoveragePolicy>,
    pub combinations: Vec<Vec<String>>,
    pub representative_cases: Vec<String>,
    pub created: String,
    pub updated: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dimension_serializes_correctly() {
        let dimension = Dimension {
            name: "dimension1".to_string(),
            partitions: vec!["partitionA".to_string(), "partitionB".to_string()],
        };

        assert_eq!(
            serde_json::to_string(&dimension).unwrap(),
            r#"{"name":"dimension1","partitions":["partitionA","partitionB"]}"#
        );
    }

    #[test]
    fn coverage_policy_serializes_correctly() {
        assert_eq!(
            serde_json::to_string(&CoveragePolicy::IndependentAxes).unwrap(),
            "\"independent-axes\""
        );
        assert_eq!(
            serde_json::to_string(&CoveragePolicy::FullProduct).unwrap(),
            "\"full-product\""
        );
        assert_eq!(
            serde_json::to_string(&CoveragePolicy::Explicit).unwrap(),
            "\"explicit\""
        );
    }

    #[test]
    fn vo_record_serializes_correctly() {
        let vo_record = VoRecord {
            id: VoId::new("vo123"),
            parent: Some(VoId::new("parent456")),
            derives_from: vec![],
            claim: "This is a claim.".to_string(),
            dimensions: vec![],
            coverage_policy: Some(CoveragePolicy::IndependentAxes),
            combinations: vec![],
            representative_cases: vec![],
            created: "2026-08-08".to_string(),
            updated: "2026-08-08".to_string(),
        };

        assert_eq!(
            serde_json::to_string(&vo_record).unwrap(),
            r#"{"id":"vo123","parent":"parent456","claim":"This is a claim.","dimensions":[],"coverage_policy":"independent-axes","combinations":[],"representative_cases":[],"created":"2026-08-08","updated":"2026-08-08"}"#
        );
    }

    #[test]
    fn vo_record_serializes_without_parent() {
        let vo_record = VoRecord {
            id: VoId::new("vo789"),
            parent: None,
            derives_from: vec![],
            claim: "Another claim.".to_string(),
            dimensions: vec![],
            coverage_policy: None,
            combinations: vec![],
            representative_cases: vec![],
            created: "2026-08-08".to_string(),
            updated: "2026-08-08".to_string(),
        };

        assert_eq!(
            serde_json::to_string(&vo_record).unwrap(),
            r#"{"id":"vo789","parent":null,"claim":"Another claim.","dimensions":[],"coverage_policy":null,"combinations":[],"representative_cases":[],"created":"2026-08-08","updated":"2026-08-08"}"#
        );
    }

    #[test]
    fn vo_record_carries_representative_cases() {
        let vo_record = VoRecord {
            id: VoId::new("vo321"),
            parent: None,
            derives_from: vec![],
            claim: "A claim with representative cases.".to_string(),
            dimensions: vec![],
            coverage_policy: None,
            combinations: vec![],
            representative_cases: vec!["empty input".to_string(), "max length input".to_string()],
            created: "2026-08-08".to_string(),
            updated: "2026-08-09".to_string(),
        };

        let json = serde_json::to_string(&vo_record).unwrap();
        assert_eq!(serde_json::from_str::<VoRecord>(&json).unwrap(), vo_record);
    }
}
