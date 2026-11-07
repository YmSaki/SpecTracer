//! Judgment records (`.verify/decisions/<ULID>.yaml`, 本冊 §3.4 / §8,
//! DES-128〜DES-133, DS-430〜DS-448).
//!
//! A judgment record is append-only history of what an external judge
//! (human or agent) decided about a subject that the machine left
//! `UNKNOWN`. It never changes a verification state (DS-693/DS-708). The
//! reason, exclusions and supersedes are optional, and an empty reason is
//! never a ground for treating a record as invalid (DS-431/DS-439/DS-441).

use std::path::Path;

use serde::{Deserialize, Serialize};
use vtest_model::{ContentHash, Revision};

use crate::{StoreError, VerifyLayout};

/// DS-694: the only judgment kinds a bundle is generated for.
pub const JUDGMENT_KINDS: &[&str] = &["test-semantic", "impl-consistency", "case-coverage"];

/// DS-707: the decision values this tool accepts.
pub const DECISION_VALUES: &[&str] = &["accepted", "rejected", "deferred"];

/// DS-713: the `basis.kind` domain inside `reason[]`.
pub const BASIS_KINDS: &[&str] = &["document", "vo", "test-code", "target-code"];

/// One entry of the upstream dependency closure bound at judgment time
/// (DS-435, DES-128's `{kind, id, hash}`).
#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DecisionDependency {
    pub kind: String,
    pub id: String,
    pub hash: ContentHash,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Actor {
    pub kind: String,
    pub id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Basis {
    pub kind: String,
    #[serde(rename = "ref")]
    pub reference: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReasonEntry {
    pub claim: String,
    #[serde(default)]
    pub basis: Vec<Basis>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DecisionRecord {
    pub id: String,
    pub subject: String,
    pub judgment_kind: String,
    #[serde(default)]
    pub supersedes: Vec<String>,
    pub subject_hash: ContentHash,
    #[serde(default)]
    pub dependencies: Vec<DecisionDependency>,
    pub actor: Actor,
    pub decision: String,
    #[serde(default)]
    pub reason: Vec<ReasonEntry>,
    #[serde(default)]
    pub exclusions: Vec<String>,
    pub decided_at: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub revision: Option<Revision>,
}

impl DecisionRecord {
    pub fn to_yaml(&self) -> Result<String, StoreError> {
        yaml_serde::to_string(self).map_err(|error| {
            StoreError::InvalidConfig(format!("could not serialize judgment record: {error}"))
        })
    }

    pub fn from_yaml(text: &str, file_id: &str) -> Result<Self, StoreError> {
        let record: Self =
            yaml_serde::from_str(text).map_err(|error| StoreError::SchemaMismatch {
                code: "E-SCAN-010",
                location: format!(".verify/decisions/{file_id}.yaml"),
                detail: error.to_string(),
            })?;
        if record.id != file_id {
            return Err(StoreError::SchemaMismatch {
                code: "E-SCAN-010",
                location: format!(".verify/decisions/{file_id}.yaml"),
                detail: format!("id `{}` does not match the file name", record.id),
            });
        }
        Ok(record)
    }
}

/// Writes a new judgment record; never overwrites (records are
/// append-only).
pub fn write_decision(layout: &VerifyLayout, record: &DecisionRecord) -> Result<(), StoreError> {
    let path = layout.decisions_dir().join(format!("{}.yaml", record.id));
    crate::records::write_new_record(&path, &record.to_yaml()?)
}

fn read_decision(path: &Path) -> Result<DecisionRecord, StoreError> {
    let text = std::fs::read_to_string(path).map_err(|source| StoreError::Io {
        path: path.to_owned(),
        source,
    })?;
    let file_id = path
        .file_stem()
        .and_then(|stem| stem.to_str())
        .unwrap_or_default();
    DecisionRecord::from_yaml(&text, file_id)
}

/// Every judgment record under `.verify/decisions/`, sorted by id. A record
/// that does not parse is an error (E-SCAN-010), not skipped.
pub fn read_all_decisions(layout: &VerifyLayout) -> Result<Vec<DecisionRecord>, StoreError> {
    let dir = layout.decisions_dir();
    let entries = match std::fs::read_dir(&dir) {
        Ok(entries) => entries,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(source) => return Err(StoreError::Io { path: dir, source }),
    };
    let mut records = Vec::new();
    for entry in entries {
        let entry = entry.map_err(|source| StoreError::Io {
            path: dir.clone(),
            source,
        })?;
        let path = entry.path();
        if path.is_file() && path.extension().and_then(|value| value.to_str()) == Some("yaml") {
            records.push(read_decision(&path)?);
        }
    }
    records.sort_by(|left, right| left.id.cmp(&right.id));
    Ok(records)
}
