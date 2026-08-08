//! Canonical entity and append-only approval records.
//!
//! M2 keeps the on-disk representation YAML as specified.  The project does
//! not yet depend on a YAML parser, so this module accepts the deliberately
//! small scalar/list subset emitted by vtest and preserves unknown fields by
//! ignoring them (forward-compatible read behavior).

use crate::{StoreError, VerifyLayout};
use serde::{Deserialize, Serialize};
use std::{
    fs,
    path::Path,
    sync::atomic::{AtomicU64, Ordering},
    time::{SystemTime, UNIX_EPOCH},
};
use vtest_model::{
    CheckValue, ContentHash, EvidenceHashes, EvidenceRecord, ReqId, Revision, RunnerInfo, SpecId,
    TargetExecution, TestId, TestResult, VoId,
};

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SpecRef {
    pub spec: SpecId,
    pub section: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct Dimension {
    pub name: String,
    pub partitions: Vec<String>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SpecRecord {
    pub id: SpecId,
    pub kind: String,
    pub path: String,
    pub sha256: ContentHash,
    pub title: Option<String>,
    pub note: Option<String>,
    pub registered_at: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ReqRecord {
    pub id: ReqId,
    pub parent: Option<ReqId>,
    pub spec_refs: Vec<SpecRef>,
    pub summary: String,
    pub status: String,
    pub created: String,
    pub updated: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct VoRecord {
    pub id: VoId,
    pub parent: Option<VoId>,
    pub requirements: Vec<ReqId>,
    pub spec_refs: Vec<SpecRef>,
    pub claim: String,
    pub dimensions: Vec<Dimension>,
    pub coverage_policy: Option<String>,
    pub representative_cases: Vec<String>,
    pub status: String,
    pub created: String,
    pub updated: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct Approver {
    pub kind: String,
    pub id: String,
    pub model: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ApprovalRecord {
    pub id: String,
    pub subject: VoId,
    pub subject_hash: ContentHash,
    pub approver: Approver,
    pub basis: Vec<String>,
    pub approved_at: String,
}

impl SpecRecord {
    pub fn to_yaml(&self) -> String {
        let mut out = format!(
            "id: {}\nkind: {}\npath: {}\nsha256: {}\n",
            yaml_scalar(self.id.as_str()),
            yaml_scalar(&self.kind),
            yaml_scalar(&self.path),
            yaml_scalar(self.sha256.as_str()),
        );
        if let Some(title) = &self.title {
            out.push_str(&format!("title: {}\n", yaml_scalar(title)));
        }
        if let Some(note) = &self.note {
            out.push_str(&format!("note: {}\n", yaml_scalar(note)));
        }
        out.push_str(&format!(
            "registered_at: {}\n",
            yaml_scalar(&self.registered_at)
        ));
        out
    }

    pub fn from_yaml(text: &str, fallback_id: &str) -> Result<Self, StoreError> {
        let id = scalar(text, "id").unwrap_or_else(|| fallback_id.to_owned());
        let sha256 = scalar(text, "sha256")
            .ok_or_else(|| StoreError::InvalidConfig("SPEC is missing sha256".to_owned()))?
            .parse()
            .map_err(|error: String| StoreError::InvalidConfig(error))?;
        Ok(Self {
            id: SpecId::new(id),
            kind: scalar(text, "kind").unwrap_or_else(|| "other".to_owned()),
            path: scalar(text, "path").unwrap_or_default(),
            sha256,
            title: scalar(text, "title"),
            note: scalar(text, "note"),
            registered_at: scalar(text, "registered_at").unwrap_or_default(),
        })
    }
}

impl ReqRecord {
    pub fn to_yaml(&self) -> String {
        let mut out = format!(
            "id: {}\nparent: {}\nspec_refs:\n",
            yaml_scalar(self.id.as_str()),
            self.parent
                .as_ref()
                .map(|value| yaml_scalar(value.as_str()))
                .unwrap_or_else(|| "null".to_owned()),
        );
        out.push_str(&spec_refs_yaml(&self.spec_refs));
        out.push_str(&format!(
            "summary: {}\nstatus: {}\ncreated: {}\nupdated: {}\n",
            yaml_scalar(&self.summary),
            yaml_scalar(&self.status),
            yaml_scalar(&self.created),
            yaml_scalar(&self.updated),
        ));
        out
    }

    pub fn from_yaml(text: &str, fallback_id: &str) -> Self {
        Self {
            id: ReqId::new(scalar(text, "id").unwrap_or_else(|| fallback_id.to_owned())),
            parent: scalar(text, "parent")
                .and_then(|value| (!value.is_empty()).then_some(ReqId::new(value))),
            spec_refs: parse_spec_refs(text),
            summary: scalar(text, "summary").unwrap_or_default(),
            status: scalar(text, "status").unwrap_or_else(|| "active".to_owned()),
            created: scalar(text, "created").unwrap_or_default(),
            updated: scalar(text, "updated").unwrap_or_default(),
        }
    }
}

impl VoRecord {
    pub fn to_yaml(&self) -> String {
        let mut out = format!(
            "id: {}\nparent: {}\nrequirements: {}\nspec_refs:\n",
            yaml_scalar(self.id.as_str()),
            self.parent
                .as_ref()
                .map(|value| yaml_scalar(value.as_str()))
                .unwrap_or_else(|| "null".to_owned()),
            yaml_list(self.requirements.iter().map(ReqId::as_str)),
        );
        out.push_str(&spec_refs_yaml(&self.spec_refs));
        out.push_str(&format!(
            "claim: {}\ndimensions:\n",
            yaml_scalar(&self.claim)
        ));
        for dimension in &self.dimensions {
            out.push_str(&format!(
                "  - name: {}\n    partitions: {}\n",
                yaml_scalar(&dimension.name),
                yaml_list(dimension.partitions.iter().map(String::as_str)),
            ));
        }
        out.push_str(&format!(
            "coverage_policy: {}\nrepresentative_cases: {}\nstatus: {}\ncreated: {}\nupdated: {}\n",
            self.coverage_policy
                .as_deref()
                .map(yaml_scalar)
                .unwrap_or_else(|| "null".to_owned()),
            yaml_list(self.representative_cases.iter().map(String::as_str)),
            yaml_scalar(&self.status),
            yaml_scalar(&self.created),
            yaml_scalar(&self.updated),
        ));
        out
    }

    pub fn from_yaml(text: &str, fallback_id: &str) -> Self {
        Self {
            id: VoId::new(scalar(text, "id").unwrap_or_else(|| fallback_id.to_owned())),
            parent: scalar(text, "parent")
                .and_then(|value| (!value.is_empty()).then_some(VoId::new(value))),
            requirements: list(text, "requirements")
                .into_iter()
                .map(ReqId::new)
                .collect(),
            spec_refs: parse_spec_refs(text),
            claim: scalar(text, "claim").unwrap_or_default(),
            dimensions: parse_dimensions(text),
            coverage_policy: scalar(text, "coverage_policy").filter(|value| value != "null"),
            representative_cases: list(text, "representative_cases"),
            status: scalar(text, "status").unwrap_or_else(|| "draft".to_owned()),
            created: scalar(text, "created").unwrap_or_default(),
            updated: scalar(text, "updated").unwrap_or_default(),
        }
    }
}

impl ApprovalRecord {
    pub fn to_yaml(&self) -> String {
        let mut out = format!(
            "id: {}\nsubject: {}\nsubject_hash: {}\napprover:\n  kind: {}\n  id: {}\n",
            yaml_scalar(&self.id),
            yaml_scalar(self.subject.as_str()),
            yaml_scalar(self.subject_hash.as_str()),
            yaml_scalar(&self.approver.kind),
            yaml_scalar(&self.approver.id),
        );
        if let Some(model) = &self.approver.model {
            out.push_str(&format!("  model: {}\n", yaml_scalar(model)));
        }
        out.push_str("basis:\n");
        for basis in &self.basis {
            out.push_str(&format!("  - {}\n", yaml_scalar(basis)));
        }
        out.push_str(&format!(
            "approved_at: {}\n",
            yaml_scalar(&self.approved_at)
        ));
        out
    }

    pub fn from_yaml(text: &str, fallback_id: &str) -> Result<Self, StoreError> {
        let subject_hash = scalar(text, "subject_hash")
            .ok_or_else(|| {
                StoreError::InvalidConfig("approval is missing subject_hash".to_owned())
            })?
            .parse()
            .map_err(|error: String| StoreError::InvalidConfig(error))?;
        Ok(Self {
            id: scalar(text, "id").unwrap_or_else(|| fallback_id.to_owned()),
            subject: VoId::new(scalar(text, "subject").unwrap_or_default()),
            subject_hash,
            approver: Approver {
                kind: nested_scalar(text, "approver", "kind").unwrap_or_else(|| "agent".to_owned()),
                id: nested_scalar(text, "approver", "id").unwrap_or_default(),
                model: nested_scalar(text, "approver", "model"),
            },
            basis: list(text, "basis"),
            approved_at: scalar(text, "approved_at").unwrap_or_default(),
        })
    }
}

pub fn read_evidence(path: &Path) -> Result<EvidenceRecord, StoreError> {
    let text = read_text(path)?;
    let fallback = path
        .file_stem()
        .and_then(|value| value.to_str())
        .unwrap_or_default();
    let test_hash = nested_scalar(&text, "hashes", "test_fn")
        .ok_or_else(|| StoreError::InvalidConfig("Evidence is missing hashes.test_fn".to_owned()))?
        .parse()
        .map_err(|error: String| StoreError::InvalidConfig(error))?;
    let target_hash = nested_scalar(&text, "hashes", "target_fn")
        .ok_or_else(|| {
            StoreError::InvalidConfig("Evidence is missing hashes.target_fn".to_owned())
        })?
        .parse()
        .map_err(|error: String| StoreError::InvalidConfig(error))?;
    let result = match scalar(&text, "result").as_deref() {
        Some("PASS") => TestResult::Pass,
        Some("FAIL") => TestResult::Fail,
        _ => {
            return Err(StoreError::InvalidConfig(
                "Evidence has invalid result".to_owned(),
            ))
        }
    };
    let target_result = match nested_scalar(&text, "target_execution", "result").as_deref() {
        Some("PASS") => CheckValue::Pass,
        Some("FAIL") => CheckValue::Fail,
        Some("NOT_CHECKED") => CheckValue::NotChecked,
        Some("NOT_EXECUTED") => CheckValue::NotExecuted,
        Some("UNKNOWN") => CheckValue::Unknown,
        Some("STALE") => CheckValue::Stale,
        Some("MISMATCH") => CheckValue::Mismatch,
        Some("MISSING") => CheckValue::Missing,
        _ => CheckValue::Unknown,
    };
    Ok(EvidenceRecord {
        id: scalar(&text, "id").unwrap_or_else(|| fallback.to_owned()),
        test_id: TestId::new(scalar(&text, "test_id").unwrap_or_default()),
        result,
        executed_at: scalar(&text, "executed_at").unwrap_or_default(),
        revision: Revision {
            commit: nested_scalar(&text, "revision", "commit").filter(|value| value != "null"),
            dirty: nested_scalar(&text, "revision", "dirty").is_some_and(|value| value == "true"),
        },
        hashes: EvidenceHashes {
            test_fn: test_hash,
            target_fn: target_hash,
        },
        runner: RunnerInfo {
            kind: nested_scalar(&text, "runner", "kind").unwrap_or_default(),
            command: nested_scalar(&text, "runner", "command").unwrap_or_default(),
            exit_code: nested_scalar(&text, "runner", "exit_code")
                .and_then(|value| value.parse().ok())
                .unwrap_or(-1),
        },
        target_execution: TargetExecution {
            checked: nested_scalar(&text, "target_execution", "checked")
                .is_some_and(|value| value == "true"),
            method: nested_scalar(&text, "target_execution", "method")
                .filter(|value| value != "null"),
            result: target_result,
            count: nested_scalar(&text, "target_execution", "count")
                .filter(|value| value != "null")
                .and_then(|value| value.parse().ok()),
        },
        log_ref: scalar(&text, "log_ref").unwrap_or_default(),
    })
}

pub fn read_spec(layout: &VerifyLayout, id: &str) -> Result<SpecRecord, StoreError> {
    let path = layout.spec_dir().join(format!("{id}.yaml"));
    let text = read_text(&path)?;
    SpecRecord::from_yaml(&text, id)
}

pub fn read_req(layout: &VerifyLayout, id: &str) -> Result<ReqRecord, StoreError> {
    let path = layout.req_dir().join(format!("{id}.yaml"));
    let text = read_text(&path)?;
    Ok(ReqRecord::from_yaml(&text, id))
}

pub fn read_vo(layout: &VerifyLayout, id: &str) -> Result<VoRecord, StoreError> {
    let path = layout.vo_dir().join(format!("{id}.yaml"));
    let text = read_text(&path)?;
    Ok(VoRecord::from_yaml(&text, id))
}

pub fn read_approval(path: &Path) -> Result<ApprovalRecord, StoreError> {
    let text = read_text(path)?;
    let fallback = path
        .file_stem()
        .and_then(|value| value.to_str())
        .unwrap_or_default();
    ApprovalRecord::from_yaml(&text, fallback)
}

pub fn read_text(path: &Path) -> Result<String, StoreError> {
    fs::read_to_string(path).map_err(|source| StoreError::Io {
        path: path.to_owned(),
        source,
    })
}

pub fn write_atomic(path: &Path, text: &str) -> Result<(), StoreError> {
    let temp = path.with_extension("yaml.tmp");
    fs::write(&temp, text).map_err(|source| StoreError::Io {
        path: temp.clone(),
        source,
    })?;
    if let Err(source) = fs::rename(&temp, path) {
        // Windows does not replace an existing file with rename.  Remove only
        // the exact canonical target, then complete the same-directory swap.
        if path.exists() {
            fs::remove_file(path).map_err(|remove_error| StoreError::Io {
                path: path.to_owned(),
                source: remove_error,
            })?;
            fs::rename(&temp, path).map_err(|rename_error| StoreError::Io {
                path: path.to_owned(),
                source: rename_error,
            })?;
        } else {
            return Err(StoreError::Io {
                path: path.to_owned(),
                source,
            });
        }
    }
    Ok(())
}

/// Generate a monotonic-enough ULID-shaped record name without making the
/// model depend on a particular ULID crate.  The timestamp occupies the
/// standard 48-bit prefix; process-local entropy and a counter fill the rest.
pub fn new_record_id() -> String {
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    const ALPHABET: &[u8; 32] = b"0123456789ABCDEFGHJKMNPQRSTVWXYZ";
    let elapsed = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();
    let millis = elapsed.as_millis() & ((1u128 << 48) - 1);
    let entropy = ((elapsed.subsec_nanos() as u128) << 32)
        | u128::from(COUNTER.fetch_add(1, Ordering::Relaxed));
    let mut value = (millis << 80) | (entropy & ((1u128 << 80) - 1));
    let mut output = [b'0'; 26];
    for slot in output.iter_mut().rev() {
        *slot = ALPHABET[(value & 31) as usize];
        value >>= 5;
    }
    String::from_utf8(output.to_vec()).expect("ULID alphabet is ASCII")
}

/// RFC 3339 UTC timestamp used by append-only records.
pub fn now_rfc3339() -> String {
    let seconds = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let days = seconds / 86_400;
    let day_seconds = seconds % 86_400;
    let (year, month, day) = civil_from_days(days as i64);
    let hour = day_seconds / 3_600;
    let minute = (day_seconds % 3_600) / 60;
    let second = day_seconds % 60;
    format!("{year:04}-{month:02}-{day:02}T{hour:02}:{minute:02}:{second:02}Z")
}

fn civil_from_days(days_since_epoch: i64) -> (i64, i64, i64) {
    let z = days_since_epoch + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let year = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = mp + if mp < 10 { 3 } else { -9 };
    let year = year + if month <= 2 { 1 } else { 0 };
    (year, month, day)
}

fn spec_refs_yaml(refs: &[SpecRef]) -> String {
    if refs.is_empty() {
        return "  []\n".to_owned();
    }
    let mut out = String::new();
    for reference in refs {
        out.push_str(&format!(
            "  - spec: {}\n    section: {}\n",
            yaml_scalar(reference.spec.as_str()),
            yaml_scalar(&reference.section),
        ));
    }
    out
}

fn parse_spec_refs(text: &str) -> Vec<SpecRef> {
    let mut refs = Vec::new();
    let lines = text.lines().collect::<Vec<_>>();
    for (index, raw) in lines.iter().enumerate() {
        let line = raw.trim();
        if let Some(value) = line.strip_prefix("- spec:") {
            let section = lines
                .get(index + 1)
                .and_then(|next| next.trim().strip_prefix("section:"))
                .map(str::trim)
                .map(unquote)
                .unwrap_or_default();
            refs.push(SpecRef {
                spec: SpecId::new(unquote(value.trim())),
                section,
            });
        }
    }
    refs
}

fn parse_dimensions(text: &str) -> Vec<Dimension> {
    let lines = text.lines().collect::<Vec<_>>();
    let mut dimensions = Vec::new();
    for (index, raw) in lines.iter().enumerate() {
        let line = raw.trim();
        let Some(name) = line.strip_prefix("- name:") else {
            continue;
        };
        let partitions = lines
            .get(index + 1)
            .and_then(|next| next.trim().strip_prefix("partitions:"))
            .map(|value| parse_inline_list(value.trim()))
            .unwrap_or_default();
        dimensions.push(Dimension {
            name: unquote(name.trim()),
            partitions,
        });
    }
    dimensions
}

fn scalar(text: &str, key: &str) -> Option<String> {
    text.lines().find_map(|raw| {
        let line = raw.trim();
        let (candidate, value) = line.split_once(':')?;
        if candidate.trim() != key {
            return None;
        }
        let value = value.trim();
        if value == "null" {
            None
        } else {
            Some(unquote(value))
        }
    })
}

pub fn yaml_scalar_value(text: &str, key: &str) -> Option<String> {
    scalar(text, key)
}

fn nested_scalar(text: &str, parent: &str, key: &str) -> Option<String> {
    let mut in_parent = false;
    for raw in text.lines() {
        let line = raw.trim_end();
        if !line.starts_with(' ') && line.trim_end() == format!("{parent}:") {
            in_parent = true;
            continue;
        }
        if in_parent && !line.starts_with(' ') {
            in_parent = false;
        }
        if in_parent {
            let trimmed = line.trim();
            if let Some(value) = trimmed.strip_prefix(&format!("{key}:")) {
                return Some(unquote(value.trim()));
            }
        }
    }
    None
}

fn list(text: &str, key: &str) -> Vec<String> {
    let lines = text.lines().collect::<Vec<_>>();
    for (index, raw) in lines.iter().enumerate() {
        let line = raw.trim();
        let Some(value) = line.strip_prefix(&format!("{key}:")) else {
            continue;
        };
        let value = value.trim();
        if value.starts_with('[') {
            return parse_inline_list(value);
        }
        let mut values = Vec::new();
        for next in lines.iter().skip(index + 1) {
            let next = next.trim();
            if let Some(item) = next.strip_prefix('-') {
                values.push(unquote(item.trim()));
            } else if !next.is_empty() {
                break;
            }
        }
        return values;
    }
    Vec::new()
}

fn parse_inline_list(value: &str) -> Vec<String> {
    let value = value.trim();
    let value = value
        .strip_prefix('[')
        .and_then(|v| v.strip_suffix(']'))
        .unwrap_or(value);
    value
        .split(',')
        .map(str::trim)
        .filter(|item| !item.is_empty())
        .map(unquote)
        .collect()
}

fn yaml_list<'a>(values: impl Iterator<Item = &'a str>) -> String {
    let values = values.map(yaml_scalar).collect::<Vec<_>>();
    format!("[{}]", values.join(", "))
}

fn yaml_scalar(value: &str) -> String {
    format!("'{}'", value.replace('\'', "''"))
}

fn unquote(value: &str) -> String {
    if value.starts_with('\'') && value.ends_with('\'') && value.len() >= 2 {
        value[1..value.len() - 1].replace("''", "'")
    } else if value.starts_with('"') && value.ends_with('"') && value.len() >= 2 {
        value[1..value.len() - 1].to_owned()
    } else {
        value.to_owned()
    }
}
