//! Judgment records (`.verify/decisions/<ULID>.yaml`, 本冊 §3.4 / §8,
//! DES-128〜DES-133, DS-430〜DS-448).
//!
//! A judgment record is append-only history of what an external judge
//! (human or agent) decided about a subject that the machine left
//! `UNKNOWN`. It never changes a verification state (DS-693/DS-708). The
//! reason, exclusions and supersedes are optional, and an empty reason is
//! never a ground for treating a record as invalid (DS-431/DS-439/DS-441).

use std::{
    collections::{BTreeMap, BTreeSet},
    path::Path,
};

use serde::{Deserialize, Serialize};
use vtest_model::{ContentHash, Diagnostic, Revision};

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
    /// DS-446/DS-1455: a compatible record may lack it. Such a record (or
    /// one with a value outside [`JUDGMENT_KINDS`]) is kept as history and
    /// contributes to no effective judgment (W-STORE-003).
    #[serde(default)]
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

/// DS-446/DS-1455/DS-554: W-STORE-003 for each record whose
/// `judgment_kind` is missing or outside [`JUDGMENT_KINDS`]. Such a record
/// stays readable as history but contributes to no effective judgment.
pub fn kindless_record_warnings(records: &[DecisionRecord]) -> Vec<Diagnostic> {
    records
        .iter()
        .filter(|record| !JUDGMENT_KINDS.contains(&record.judgment_kind.as_str()))
        .map(|record| {
            Diagnostic::warning(
                "W-STORE-003",
                format!(
                    "judgment record {} has {}; it is kept as history and contributes to no \
                     effective judgment (DS-446)",
                    record.id,
                    if record.judgment_kind.is_empty() {
                        "no judgment_kind".to_owned()
                    } else {
                        format!(
                            "judgment_kind `{}` outside the domain",
                            record.judgment_kind
                        )
                    }
                ),
            )
        })
        .collect()
}

/// The effective judgment of one `(subject, judgment_kind)` (本冊 §8.5).
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum EffectiveJudgment {
    /// DS-728/DS-730/DS-735: empty effective set, conflicting values, or a
    /// cycle that empties it.
    Unknown,
    /// DS-729: every record of the effective set carries this decision.
    Decided(String),
}

/// Derives the effective judgment of `(subject, judgment_kind)` against the
/// current subject hash and dependency closure, with its W-STORE-004 /
/// W-STORE-005 warnings.
///
/// - A record whose `supersedes` names itself, a missing id, or a record of
///   another subject or kind, or that lies on a `supersedes` cycle, is
///   ill-formed (DS-447/DS-556/DS-735): it contributes to the effective set
///   neither as a member nor by removing what it names, and gives
///   W-STORE-005. The check runs on every read, since a stored record can be
///   edited or its target removed after submission (DS-1451 checks only the
///   submission).
/// - V: the well-formed records whose subject hash and dependency closure
///   equal the current ones (DS-724/DS-726).
/// - E: V minus the records another record of V supersedes (DS-727).
/// - Empty E is UNKNOWN (DS-728); one shared value is that value (DS-729);
///   several values are UNKNOWN with W-STORE-004 (DS-725/DS-730). No
///   record is preferred by age, value or count (DS-734).
pub fn effective_judgment(
    records: &[DecisionRecord],
    subject: &str,
    judgment_kind: &str,
    current_hash: &ContentHash,
    current_dependencies: &[DecisionDependency],
) -> (EffectiveJudgment, Vec<Diagnostic>) {
    let own: Vec<&DecisionRecord> = records
        .iter()
        .filter(|record| record.subject == subject && record.judgment_kind == judgment_kind)
        .collect();
    let by_id: BTreeMap<&str, &DecisionRecord> = own
        .iter()
        .map(|record| (record.id.as_str(), *record))
        .collect();

    let mut diagnostics = Vec::new();
    let mut ill_formed: BTreeSet<&str> = BTreeSet::new();
    for record in &own {
        for target in &record.supersedes {
            let reason = if target == &record.id {
                Some("names itself")
            } else if !by_id.contains_key(target.as_str()) {
                Some("names no judgment record of the same subject and judgment_kind")
            } else {
                None
            };
            if let Some(reason) = reason {
                ill_formed.insert(record.id.as_str());
                diagnostics.push(Diagnostic::warning(
                    "W-STORE-005",
                    format!(
                        "judgment record {} supersedes '{target}', which {reason}; the record \
                         does not contribute to the effective set (DS-447/DS-556)",
                        record.id
                    ),
                ));
            }
        }
    }
    for id in cycle_members(&by_id) {
        if ill_formed.insert(id) {
            diagnostics.push(Diagnostic::warning(
                "W-STORE-005",
                format!(
                    "judgment record {id} is on a supersedes cycle; the record does not \
                     contribute to the effective set (DS-556/DS-735)"
                ),
            ));
        }
    }

    let valid: Vec<&DecisionRecord> = own
        .iter()
        .copied()
        .filter(|record| !ill_formed.contains(record.id.as_str()))
        .filter(|record| {
            &record.subject_hash == current_hash && record.dependencies == current_dependencies
        })
        .collect();
    let superseded: BTreeSet<&str> = valid
        .iter()
        .flat_map(|record| record.supersedes.iter().map(String::as_str))
        .collect();
    let values: BTreeSet<&str> = valid
        .iter()
        .filter(|record| !superseded.contains(record.id.as_str()))
        .map(|record| record.decision.as_str())
        .collect();
    let judgment = match values.len() {
        0 => EffectiveJudgment::Unknown,
        1 => EffectiveJudgment::Decided(values.into_iter().next().unwrap_or_default().to_owned()),
        _ => {
            diagnostics.push(Diagnostic::warning(
                "W-STORE-004",
                format!(
                    "{subject} has valid {judgment_kind} judgments with differing decisions \
                     ({}); the effective judgment is UNKNOWN (DS-725/DS-730)",
                    values.into_iter().collect::<Vec<_>>().join(", ")
                ),
            ));
            EffectiveJudgment::Unknown
        }
    };
    (judgment, diagnostics)
}

/// Ids of records on a `supersedes` cycle, following only edges between
/// records of the same `(subject, judgment_kind)`.
fn cycle_members<'a>(by_id: &BTreeMap<&'a str, &'a DecisionRecord>) -> BTreeSet<&'a str> {
    let mut members = BTreeSet::new();
    for &start in by_id.keys() {
        let mut stack: Vec<&str> = by_id[start]
            .supersedes
            .iter()
            .map(String::as_str)
            .filter(|target| *target != start)
            .collect();
        let mut seen = BTreeSet::new();
        while let Some(id) = stack.pop() {
            if id == start {
                members.insert(start);
                break;
            }
            if !seen.insert(id) {
                continue;
            }
            if let Some(record) = by_id.get(id) {
                stack.extend(record.supersedes.iter().map(String::as_str));
            }
        }
    }
    members
}
