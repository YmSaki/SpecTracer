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
    io::Write,
    path::{Path, PathBuf},
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
    #[serde(default)]
    pub combinations: Vec<Vec<String>>,
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
pub struct ApprovalBasis {
    pub kind: String,
    #[serde(rename = "ref")]
    pub reference: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ApprovalRecord {
    pub id: String,
    pub subject: VoId,
    pub subject_hash: ContentHash,
    pub approver: Approver,
    pub basis: Vec<ApprovalBasis>,
    pub approved_at: String,
}

/// The only non-derived relationship kinds persisted in `.verify/rel/`.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum RelationType {
    DependsOn,
    Supersedes,
    RegressionFor,
    DerivedFrom,
    SamePartition,
    Complements,
    ConflictsWith,
}

impl RelationType {
    fn as_str(self) -> &'static str {
        match self {
            Self::DependsOn => "depends-on",
            Self::Supersedes => "supersedes",
            Self::RegressionFor => "regression-for",
            Self::DerivedFrom => "derived-from",
            Self::SamePartition => "same-partition",
            Self::Complements => "complements",
            Self::ConflictsWith => "conflicts-with",
        }
    }

    fn parse(value: &str) -> Option<Self> {
        match value {
            "depends-on" => Some(Self::DependsOn),
            "supersedes" => Some(Self::Supersedes),
            "regression-for" => Some(Self::RegressionFor),
            "derived-from" => Some(Self::DerivedFrom),
            "same-partition" => Some(Self::SamePartition),
            "complements" => Some(Self::Complements),
            "conflicts-with" => Some(Self::ConflictsWith),
            _ => None,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct RelationRecord {
    pub id: String,
    #[serde(rename = "type")]
    pub relation_type: RelationType,
    pub from: String,
    pub to: String,
    pub note: Option<String>,
    pub created: String,
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
            "coverage_policy: {}\n",
            self.coverage_policy
                .as_deref()
                .map(yaml_scalar)
                .unwrap_or_else(|| "null".to_owned()),
        ));
        if self.combinations.is_empty() {
            out.push_str("combinations: []\n");
        } else {
            out.push_str("combinations:\n");
            for combination in &self.combinations {
                out.push_str(&format!(
                    "  - {}\n",
                    yaml_list(combination.iter().map(String::as_str))
                ));
            }
        }
        out.push_str(&format!(
            "representative_cases: {}\nstatus: {}\ncreated: {}\nupdated: {}\n",
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
            combinations: parse_combinations(text),
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
        if self.basis.is_empty() {
            out.push_str("basis: []\n");
        } else {
            out.push_str("basis:\n");
        }
        for basis in &self.basis {
            out.push_str(&format!(
                "  - kind: {}\n    ref: {}\n",
                yaml_scalar(&basis.kind),
                yaml_scalar(&basis.reference),
            ));
        }
        out.push_str(&format!(
            "approved_at: {}\n",
            yaml_scalar(&self.approved_at)
        ));
        out
    }

    pub fn from_yaml(text: &str, fallback_id: &str) -> Result<Self, StoreError> {
        let id = required_top_level_scalar(text, "id", "approval")?;
        let subject = required_top_level_scalar(text, "subject", "approval")?;
        let subject_hash = required_top_level_scalar(text, "subject_hash", "approval")?
            .parse()
            .map_err(|error: String| StoreError::InvalidConfig(error))?;
        let approver_kind = nested_scalar(text, "approver", "kind")
            .filter(|value| matches!(value.as_str(), "human" | "agent"))
            .ok_or_else(|| {
                StoreError::InvalidConfig(
                    "approval is missing a valid approver.kind (human or agent)".to_owned(),
                )
            })?;
        let approver_id = nested_scalar(text, "approver", "id")
            .filter(|value| !value.trim().is_empty())
            .ok_or_else(|| {
                StoreError::InvalidConfig("approval is missing approver.id".to_owned())
            })?;
        let approved_at = required_top_level_scalar(text, "approved_at", "approval")?;
        if id != fallback_id {
            return Err(StoreError::InvalidConfig(format!(
                "approval id {id} does not match file name {fallback_id}"
            )));
        }
        if !is_valid_ulid(&id) {
            return Err(StoreError::InvalidConfig(
                "approval id must be a valid ULID".to_owned(),
            ));
        }
        if !subject.starts_with("VO-")
            || subject.len() <= "VO-".len()
            || !subject.chars().all(|character| {
                character.is_ascii_uppercase() || character.is_ascii_digit() || character == '-'
            })
        {
            return Err(StoreError::InvalidConfig(
                "approval subject must be a valid VO ID".to_owned(),
            ));
        }
        Ok(Self {
            id,
            subject: VoId::new(subject),
            subject_hash,
            approver: Approver {
                kind: approver_kind,
                id: approver_id,
                model: nested_scalar(text, "approver", "model"),
            },
            basis: parse_approval_basis(text)?,
            approved_at,
        })
    }
}

impl RelationRecord {
    pub fn to_yaml(&self) -> Result<String, StoreError> {
        self.validate(None)?;
        let mut out = format!(
            "id: {}\ntype: {}\nfrom: {}\nto: {}\n",
            yaml_scalar(&self.id),
            yaml_scalar(self.relation_type.as_str()),
            yaml_scalar(&self.from),
            yaml_scalar(&self.to),
        );
        if let Some(note) = &self.note {
            out.push_str(&format!("note: {}\n", yaml_scalar(note)));
        }
        out.push_str(&format!("created: {}\n", yaml_scalar(&self.created)));
        Ok(out)
    }

    pub fn from_yaml(text: &str, filename_id: &str) -> Result<Self, StoreError> {
        let record = Self {
            id: required_top_level_scalar(text, "id", "relation")?,
            relation_type: required_top_level_scalar(text, "type", "relation")
                .ok()
                .and_then(|value| RelationType::parse(&value))
                .ok_or_else(|| {
                    StoreError::InvalidConfig("relation has an invalid type".to_owned())
                })?,
            from: required_top_level_scalar(text, "from", "relation")?,
            to: required_top_level_scalar(text, "to", "relation")?,
            note: top_level_scalar(text, "note"),
            created: required_top_level_scalar(text, "created", "relation")?,
        };
        record.validate(Some(filename_id))?;
        Ok(record)
    }

    fn validate(&self, filename_id: Option<&str>) -> Result<(), StoreError> {
        if !is_valid_relation_id(&self.id) {
            return Err(StoreError::InvalidConfig(
                "relation id must be a ULID or REL-<ULID>".to_owned(),
            ));
        }
        if let Some(filename_id) = filename_id {
            if self.id != filename_id {
                return Err(StoreError::InvalidConfig(format!(
                    "relation id {} does not match file name {filename_id}",
                    self.id
                )));
            }
        }
        for (field, value) in [
            ("from", self.from.as_str()),
            ("to", self.to.as_str()),
            ("created", self.created.as_str()),
        ] {
            if value.trim().is_empty() {
                return Err(StoreError::InvalidConfig(format!(
                    "relation is missing required field {field}"
                )));
            }
        }
        Ok(())
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
    let temp = create_unique_temp_file(path, text)?;
    if let Err(source) = replace_file(&temp, path) {
        let _ = fs::remove_file(&temp);
        return Err(StoreError::Io {
            path: path.to_owned(),
            source,
        });
    }
    Ok(())
}

/// Create one append-only canonical fact without ever replacing an existing
/// record. A collision is an error so a prior approval/audit/Evidence fact
/// cannot be silently lost.
pub fn write_new_record(path: &Path, text: &str) -> Result<(), StoreError> {
    let temp = create_unique_temp_file(path, text)?;
    let publish = fs::hard_link(&temp, path).map_err(|source| StoreError::Io {
        path: path.to_owned(),
        source,
    });
    match publish {
        Ok(()) => {
            // Publication already succeeded atomically. A transient cleanup
            // failure must not report the committed operation as failed and
            // tempt callers to append a duplicate fact.
            let _ = fs::remove_file(&temp);
            Ok(())
        }
        Err(error) => {
            let _ = fs::remove_file(&temp);
            Err(error)
        }
    }
}

fn create_unique_temp_file(path: &Path, text: &str) -> Result<PathBuf, StoreError> {
    for _ in 0..16 {
        let temp = path.with_extension(format!("{}.tmp", new_record_id()));
        match fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temp)
        {
            Ok(mut file) => {
                if let Err(source) = file
                    .write_all(text.as_bytes())
                    .and_then(|_| file.sync_all())
                {
                    drop(file);
                    let _ = fs::remove_file(&temp);
                    return Err(StoreError::Io { path: temp, source });
                }
                return Ok(temp);
            }
            Err(source) if source.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(source) => return Err(StoreError::Io { path: temp, source }),
        }
    }
    Err(StoreError::Io {
        path: path.to_owned(),
        source: std::io::Error::new(
            std::io::ErrorKind::AlreadyExists,
            "could not allocate a unique atomic-write temporary file",
        ),
    })
}

#[cfg(not(windows))]
fn replace_file(source: &Path, destination: &Path) -> std::io::Result<()> {
    fs::rename(source, destination)
}

#[cfg(windows)]
fn replace_file(source: &Path, destination: &Path) -> std::io::Result<()> {
    use std::os::windows::ffi::OsStrExt;

    #[link(name = "Kernel32")]
    extern "system" {
        fn MoveFileExW(existing: *const u16, replacement: *const u16, flags: u32) -> i32;
    }

    const MOVEFILE_REPLACE_EXISTING: u32 = 0x1;
    const MOVEFILE_WRITE_THROUGH: u32 = 0x8;
    let source = source
        .as_os_str()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect::<Vec<_>>();
    let destination = destination
        .as_os_str()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect::<Vec<_>>();
    // SAFETY: both buffers are NUL-terminated and remain alive for the call.
    let result = unsafe {
        MoveFileExW(
            source.as_ptr(),
            destination.as_ptr(),
            MOVEFILE_REPLACE_EXISTING | MOVEFILE_WRITE_THROUGH,
        )
    };
    if result == 0 {
        Err(std::io::Error::last_os_error())
    } else {
        Ok(())
    }
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
    let entropy = ((elapsed.subsec_nanos() as u128) << 48)
        | (u128::from(std::process::id()) << 16)
        | u128::from(COUNTER.fetch_add(1, Ordering::Relaxed) & 0xffff);
    let mut value = (millis << 80) | (entropy & ((1u128 << 80) - 1));
    let mut output = [b'0'; 26];
    for slot in output.iter_mut().rev() {
        *slot = ALPHABET[(value & 31) as usize];
        value >>= 5;
    }
    String::from_utf8(output.to_vec()).expect("ULID alphabet is ASCII")
}

pub fn is_valid_ulid(value: &str) -> bool {
    const ALPHABET: &str = "0123456789ABCDEFGHJKMNPQRSTVWXYZ";
    value.len() == 26
        && value
            .bytes()
            .next()
            .is_some_and(|first| matches!(first, b'0'..=b'7'))
        && value.chars().all(|character| ALPHABET.contains(character))
}

/// Accept both spellings currently present in the normative documents:
/// detailed design uses a bare ULID, while basic specification §3.1 labels
/// Relation IDs as `REL-` (ULID). The payload is always strictly validated.
pub fn is_valid_relation_id(value: &str) -> bool {
    relation_ulid_payload(value).is_some()
}

pub fn relation_ulid_payload(value: &str) -> Option<&str> {
    if is_valid_ulid(value) {
        Some(value)
    } else {
        value
            .strip_prefix("REL-")
            .filter(|value| is_valid_ulid(value))
    }
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

fn parse_combinations(text: &str) -> Vec<Vec<String>> {
    let lines = text.lines().collect::<Vec<_>>();
    let Some(start) = lines.iter().position(|line| {
        !line.starts_with([' ', '\t'])
            && line
                .trim()
                .strip_prefix("combinations:")
                .is_some_and(|value| value.trim().is_empty())
    }) else {
        return Vec::new();
    };
    lines
        .iter()
        .skip(start + 1)
        .take_while(|line| line.starts_with([' ', '\t']) || line.trim().is_empty())
        .filter_map(|line| line.trim().strip_prefix("- "))
        .map(|value| parse_inline_list(value.trim()))
        .collect()
}

fn parse_approval_basis(text: &str) -> Result<Vec<ApprovalBasis>, StoreError> {
    let lines = text.lines().collect::<Vec<_>>();
    let Some(start) = lines
        .iter()
        .position(|line| !line.starts_with([' ', '\t']) && line.trim() == "basis:")
    else {
        return Ok(Vec::new());
    };
    let mut basis = Vec::new();
    let mut index = start + 1;
    while index < lines.len() {
        let raw = lines[index];
        if !raw.starts_with([' ', '\t']) && !raw.trim().is_empty() {
            break;
        }
        let Some(kind) = raw.trim().strip_prefix("- kind:") else {
            if raw.trim().is_empty() {
                index += 1;
                continue;
            }
            return Err(StoreError::InvalidConfig(
                "approval basis entries must contain kind and ref".to_owned(),
            ));
        };
        let reference = lines
            .get(index + 1)
            .and_then(|line| line.trim().strip_prefix("ref:"))
            .map(str::trim)
            .map(unquote)
            .filter(|value| !value.is_empty())
            .ok_or_else(|| {
                StoreError::InvalidConfig(
                    "approval basis entries must contain kind and ref".to_owned(),
                )
            })?;
        let kind = unquote(kind.trim());
        if kind.is_empty() {
            return Err(StoreError::InvalidConfig(
                "approval basis kind must not be empty".to_owned(),
            ));
        }
        basis.push(ApprovalBasis { kind, reference });
        index += 2;
    }
    Ok(basis)
}

fn required_top_level_scalar(
    text: &str,
    key: &str,
    record_kind: &str,
) -> Result<String, StoreError> {
    top_level_scalar(text, key)
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| {
            StoreError::InvalidConfig(format!("{record_kind} is missing required field {key}"))
        })
}

fn top_level_scalar(text: &str, key: &str) -> Option<String> {
    text.lines().find_map(|raw| {
        if raw.starts_with([' ', '\t']) {
            return None;
        }
        let (candidate, value) = raw.split_once(':')?;
        if candidate.trim() != key {
            return None;
        }
        let value = value.trim();
        if value.is_empty() || value == "null" {
            None
        } else {
            Some(unquote(value))
        }
    })
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;

    fn temporary_directory(name: &str) -> PathBuf {
        let root = std::env::temp_dir().join(format!("vtest-store-{name}-{}", new_record_id()));
        fs::create_dir_all(&root).unwrap();
        root
    }

    #[test]
    fn generated_record_ids_are_valid_and_unique() {
        let ids = (0..4096).map(|_| new_record_id()).collect::<BTreeSet<_>>();
        assert_eq!(ids.len(), 4096);
        assert!(ids.iter().all(|id| is_valid_ulid(id)));
    }

    #[test]
    fn append_only_write_never_replaces_an_existing_fact() {
        let root = temporary_directory("append-only");
        let path = root.join(format!("{}.yaml", new_record_id()));
        write_new_record(&path, "first\n").unwrap();
        assert!(write_new_record(&path, "second\n").is_err());
        assert_eq!(fs::read_to_string(&path).unwrap(), "first\n");
        assert_eq!(fs::read_dir(&root).unwrap().count(), 1);
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn append_only_write_publishes_complete_content() {
        let root = temporary_directory("append-only-complete");
        let path = root.join(format!("{}.yaml", new_record_id()));
        let content = format!(
            "id: {}\npayload: {}\n",
            new_record_id(),
            "x".repeat(1 << 20)
        );
        write_new_record(&path, &content).unwrap();
        assert_eq!(fs::read_to_string(&path).unwrap(), content);
        assert_eq!(fs::read_dir(&root).unwrap().count(), 1);
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn atomic_write_replaces_the_complete_entity_file() {
        let root = temporary_directory("atomic");
        let path = root.join("VO-ONE.yaml");
        fs::write(&path, "old\n").unwrap();
        write_atomic(&path, "new\ncomplete\n").unwrap();
        assert_eq!(fs::read_to_string(&path).unwrap(), "new\ncomplete\n");
        assert_eq!(fs::read_dir(&root).unwrap().count(), 1);
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn approval_round_trip_requires_a_traceable_approver() {
        let id = new_record_id();
        let record = ApprovalRecord {
            id: id.clone(),
            subject: VoId::new("VO-ONE"),
            subject_hash: ContentHash::from_text("vo\n"),
            approver: Approver {
                kind: "human".to_owned(),
                id: "reviewer".to_owned(),
                model: None,
            },
            basis: vec![ApprovalBasis {
                kind: "audit".to_owned(),
                reference: new_record_id(),
            }],
            approved_at: "2026-08-08T00:00:00Z".to_owned(),
        };
        let yaml = record.to_yaml();
        assert_eq!(ApprovalRecord::from_yaml(&yaml, &id).unwrap(), record);

        let malformed = yaml
            .lines()
            .filter(|line| {
                !matches!(
                    line.trim(),
                    "approver:" | "kind: 'human'" | "id: 'reviewer'"
                )
            })
            .collect::<Vec<_>>()
            .join("\n");
        assert!(ApprovalRecord::from_yaml(&malformed, &id).is_err());
    }

    #[test]
    fn relation_round_trip_requires_a_valid_immutable_record() {
        let id = new_record_id();
        let record = RelationRecord {
            id: id.clone(),
            relation_type: RelationType::Complements,
            from: "TEST-PARSER-044".to_owned(),
            to: "TEST-PARSER-012".to_owned(),
            note: Some("boundary cases overlap".to_owned()),
            created: "2026-08-08T00:00:00Z".to_owned(),
        };
        let yaml = record.to_yaml().unwrap();
        assert_eq!(RelationRecord::from_yaml(&yaml, &id).unwrap(), record);

        for malformed in [
            yaml.replacen("type: 'complements'", "type: 'unknown'", 1),
            yaml.replacen("from: 'TEST-PARSER-044'", "from: ''", 1),
            yaml.replacen("to: 'TEST-PARSER-012'", "to: ''", 1),
            yaml.replacen("created: '2026-08-08T00:00:00Z'", "created: ''", 1),
        ] {
            assert!(RelationRecord::from_yaml(&malformed, &id).is_err());
        }
        assert!(RelationRecord::from_yaml(&yaml, &new_record_id()).is_err());

        let prefixed_id = format!("REL-{}", new_record_id());
        let prefixed = RelationRecord {
            id: prefixed_id.clone(),
            ..record.clone()
        };
        let prefixed_yaml = prefixed.to_yaml().unwrap();
        assert_eq!(
            RelationRecord::from_yaml(&prefixed_yaml, &prefixed_id).unwrap(),
            prefixed
        );

        let invalid = RelationRecord {
            id: "not-a-ulid".to_owned(),
            ..record
        };
        assert!(invalid.to_yaml().is_err());
        let invalid = RelationRecord {
            from: String::new(),
            ..RelationRecord::from_yaml(&yaml, &id).unwrap()
        };
        assert!(invalid.to_yaml().is_err());
    }
}
