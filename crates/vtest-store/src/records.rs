//! Predecessor entity records (Spec/Req/VO) plus the canonical Approval,
//! Evidence, and Relation records that also happen to live in this file.
//! `SpecRecord`, `ReqRecord`, and the local `VoRecord` accept the
//! deliberately small hand-rolled scalar/list YAML subset vtest itself
//! emits and are out of this PR's scope (PR8 retires the predecessor
//! Spec/Req model these serve — see AGENTS.md's predecessor/canonical
//! split). The predecessor `AuditRecord` (`.verify/audits/`) was removed
//! by the Owner's decision in issue #23 (BD-150: no canonical directory
//! for deterministic-analysis results).
//!
//! `ApprovalRecord`, `read_evidence`/`EvidenceRecord`, and `RelationRecord`
//! are canonical entities on that same chain (`Document` → `VO` → `Test` →
//! `Target` → `Evidence` → `VerificationResult`, with Approval an
//! independent domain per AGENTS.md) and are not PR8 predecessors, even
//! though `ApprovalRecord`/`read_evidence` still parse via this file's
//! hand-rolled scalar/list helpers rather than `yaml_serde`'s typed
//! `Deserialize` the way `RelationRecord` does below. All three reject an
//! unrecognized field fail-closed (DS-1645/E-SCAN-010: schema不一致
//! （宣言されていない余剰 field を含む）is an error, not a warning) rather
//! than preserving it by silently ignoring it — `RelationRecord::from_yaml`
//! does this via a `yaml_serde::Value` → known-key scan → typed struct
//! parse; `ApprovalRecord::from_yaml` and `read_evidence` do the same
//! known-key scan over a `yaml_serde::Value` first, then keep their
//! existing hand-rolled extraction (and its existing, more specific
//! validations — ULID format, `human`/`agent` whitelist, and so on)
//! unchanged over the original text. Deliberately *not* listed among those
//! validations: a format/prefix check on `ApprovalRecord.subject` — DS-052
//! ("ツールはID形式を強制せず一意性のみを強制する") forbids exactly that,
//! see `ApprovalRecord::from_yaml`'s own doc comment at that field.

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
    ContentHash, Diagnostic, EvidenceHashes, EvidenceRecord, ReqId, Revision, RunnerInfo, SpecId,
    TargetCoverage, TargetCoverageResult, TargetCoverageTarget, TestId, TestResult, VoId,
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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
}

/// One entry of an Approval record's upstream dependency closure (DS-1467:
/// `dependencies` は現在の上流依存closureと entity・hash とも完全一致で
/// 有効性を判定する). `entity` is the dependency node's own id (a VO id or a
/// document node id, per which closure DS-1480/DS-1487 built it from); `hash`
/// is that entity's subject hash at the time this Approval record was
/// written.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct DependencyRecord {
    pub entity: String,
    pub hash: ContentHash,
}

/// A canonical Approval record (`.verify/approvals/<ULID>.yaml`), per 本冊
/// §3.5 and DS-1050〜DS-1062 / DS-1461〜DS-1490.
///
/// `subject_type` is `"vo"` (DS-1050), `"document"` (DS-1051), or
/// `"judgment"` (DS-1052) — the value domain DS-1475 restricts *effective*
/// approval subjects to (VO id, document id); a `judgment`-typed record does
/// not put the judgment record's own ULID into `subject` (DS-1483 forbids
/// that) — it resolves the referenced judgment record's own `subject` and
/// writes that here instead (DS-1052), with `judgment_ref` carrying the
/// judgment record's ULID (DS-1476/DS-1482).
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ApprovalRecord {
    pub id: String,
    pub subject_type: String,
    pub subject: String,
    pub subject_hash: ContentHash,
    #[serde(default)]
    pub dependencies: Vec<DependencyRecord>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub judgment_ref: Option<String>,
    pub approver: Approver,
    pub approved_state: String,
    /// DS-1055/DS-1196: "`--basis` は根拠参照（任意）である" — a bare list
    /// of free-form reference strings, no `{kind, ref}` pair or `kind`
    /// value domain (that structure was a prior, unfounded downstream
    /// invention on this field; team-lead ruling 2026-09-10, see
    /// `reports/closure-trace.md`'s stopped_on history).
    #[serde(default)]
    pub basis: Vec<String>,
    #[serde(default)]
    pub supersedes: Vec<String>,
    pub approved_at: String,
}

/// The three `subject_type` values DS-1050/DS-1051/DS-1052 define. No other
/// value is valid (DS-1059/DS-1464: an out-of-domain `--subject-type` is
/// E-APPROVAL-002, exit 2, on write).
pub const APPROVAL_SUBJECT_TYPES: &[&str] = &["vo", "document", "judgment"];

/// The three `approved_state` values DS-1054/DS-1463 define (本冊 §3.5's
/// `approved_state`). No other value is valid (DS-1059/DS-1464).
pub const APPROVAL_STATES: &[&str] = &["approved", "rejected", "withdrawn"];

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

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RelationRecord {
    pub id: String,
    #[serde(rename = "type")]
    pub relation_type: RelationType,
    pub from: String,
    pub to: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
    pub created: String,
}

/// Known top-level keys for a canonical Relation record (DS-425/DS-426,
/// DS-S074). Kept in sync with `RelationRecord`'s own fields by
/// `relation_known_keys_match_the_record_shape` (`#[cfg(test)]`, below) —
/// also enforced independently by `#[serde(deny_unknown_fields)]` above
/// (DS-1645/E-SCAN-010's generic "schema不一致（宣言されていない余剰 field
/// を含む）").
const RELATION_KEYS: &[&str] = &["id", "type", "from", "to", "note", "created"];

/// Known top-level keys for a canonical Approval record, matching
/// `ApprovalRecord`'s own fields (DS-1645/E-SCAN-010).
const APPROVAL_KEYS: &[&str] = &[
    "id",
    "subject_type",
    "subject",
    "subject_hash",
    "dependencies",
    "judgment_ref",
    "approver",
    "approved_state",
    "basis",
    "supersedes",
    "approved_at",
];

/// Known keys for an Approval record's nested `approver` mapping, matching
/// `Approver`'s own fields.
const APPROVER_KEYS: &[&str] = &["kind", "id", "model"];

/// Known keys for one entry of an Approval record's `dependencies[]` list,
/// matching `DependencyRecord`'s own fields.
const APPROVAL_DEPENDENCY_KEYS: &[&str] = &["entity", "hash"];

/// Known top-level keys for a canonical Evidence record, matching
/// `EvidenceRecord`'s own fields (`vtest-model`) as `read_evidence` below
/// actually populates them, and the shape `write_evidence` below emits.
const EVIDENCE_KEYS: &[&str] = &[
    "id",
    "test_id",
    "adapter",
    "result",
    "executed_at",
    "revision",
    "execution_state",
    "hashes",
    "runner",
    "target_coverage",
    "log_ref",
];

/// Known keys for an Evidence record's nested `execution_state` mapping,
/// matching `ExecutionState`'s own fields (DES-097/DES-184).
const EVIDENCE_EXECUTION_STATE_KEYS: &[&str] = &["schema", "complete", "hash"];

/// Known keys for an Evidence record's nested `revision` mapping.
const EVIDENCE_REVISION_KEYS: &[&str] = &["commit", "dirty"];

/// Known keys for an Evidence record's nested `hashes` mapping. `target_fns`
/// is optional (`read_evidence` treats an absent `target_fns:` key as an
/// empty list), matching `EvidenceHashes`'s own fields.
const EVIDENCE_HASHES_KEYS: &[&str] = &["test_fn", "target_fn", "target_fns"];

/// Known keys for an Evidence record's nested `runner` mapping, matching
/// `RunnerInfo`'s own fields.
const EVIDENCE_RUNNER_KEYS: &[&str] = &["kind", "command", "exit_code"];

/// Known keys for an Evidence record's nested `target_coverage` mapping,
/// matching `TargetCoverage`'s own fields (DES-183: no `diagnostic` key).
const EVIDENCE_TARGET_COVERAGE_KEYS: &[&str] = &["checked", "method", "result", "targets", "count"];

/// Known keys for each entry in `target_coverage.targets` (DES-188/189).
const EVIDENCE_TARGET_COVERAGE_TARGET_KEYS: &[&str] = &["target", "result", "count"];

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
    pub fn to_yaml(&self) -> Result<String, StoreError> {
        self.validate(None)?;
        yaml_serde::to_string(self).map_err(|error| {
            StoreError::InvalidConfig(format!("could not serialize approval: {error}"))
        })
    }

    /// DS-1645/E-SCAN-010: a field outside `APPROVAL_KEYS`/`APPROVER_KEYS`/
    /// `APPROVAL_DEPENDENCY_KEYS` fails closed rather than being silently
    /// ignored — same pattern as `RelationRecord`'s own `from_yaml`: a
    /// text -> `Value` -> known-key scan (for a named error) -> typed
    /// struct parse. `basis[]` is a bare string list (DS-1055/1196), so it
    /// has no nested keys of its own to scan.
    pub fn from_yaml(text: &str, filename_id: &str) -> Result<Self, StoreError> {
        let value: yaml_serde::Value = yaml_serde::from_str(text).map_err(|error| {
            StoreError::InvalidConfig(format!("invalid approval record: {error}"))
        })?;
        crate::canonical::reject_unknown_fields(&value, APPROVAL_KEYS, "")?;
        if let Some(approver) = value.get("approver") {
            crate::canonical::reject_unknown_fields(approver, APPROVER_KEYS, "approver.")?;
        }
        if let Some(dependencies) = value
            .get("dependencies")
            .and_then(yaml_serde::Value::as_sequence)
        {
            for (index, entry) in dependencies.iter().enumerate() {
                crate::canonical::reject_unknown_fields(
                    entry,
                    APPROVAL_DEPENDENCY_KEYS,
                    &format!("dependencies[{index}]."),
                )?;
            }
        }

        let record: Self = yaml_serde::from_value(value).map_err(|error| {
            StoreError::InvalidConfig(format!("invalid approval record: {error}"))
        })?;
        record.validate(Some(filename_id))?;
        Ok(record)
    }

    /// DS-1050/1051/1052 (subject_type domain), DS-1054/1463 (approved_state
    /// domain), DS-1059/1464 (out-of-domain `--subject-type`/`--state` or a
    /// `subject_type`↔`subject`/`judgment_ref` mismatch is E-APPROVAL-002 on
    /// write), DS-1476/1483 (`judgment_ref` set iff `subject_type ==
    /// "judgment"`, and a judgment ULID never appears in `subject` itself).
    fn validate(&self, filename_id: Option<&str>) -> Result<(), StoreError> {
        if !is_valid_ulid(&self.id) {
            return Err(StoreError::InvalidConfig(
                "approval id must be a valid ULID".to_owned(),
            ));
        }
        if let Some(filename_id) = filename_id {
            if self.id != filename_id {
                return Err(StoreError::InvalidConfig(format!(
                    "approval id {} does not match file name {filename_id}",
                    self.id
                )));
            }
        }
        if !APPROVAL_SUBJECT_TYPES.contains(&self.subject_type.as_str()) {
            return Err(StoreError::InvalidConfig(format!(
                "approval subject_type must be one of {APPROVAL_SUBJECT_TYPES:?}, got {}",
                self.subject_type
            )));
        }
        if !APPROVAL_STATES.contains(&self.approved_state.as_str()) {
            return Err(StoreError::InvalidConfig(format!(
                "approval approved_state must be one of {APPROVAL_STATES:?}, got {}",
                self.approved_state
            )));
        }
        if !matches!(self.approver.kind.as_str(), "human" | "agent") {
            return Err(StoreError::InvalidConfig(
                "approval is missing a valid approver.kind (human or agent)".to_owned(),
            ));
        }
        // DS-052 ("ツールはID形式を強制せず一意性のみを強制する") — no
        // format/prefix check on `subject`, `approver.id`, or `basis[].ref`
        // beyond non-emptiness, matching the predecessor `ApprovalRecord`'s
        // own documented choice at this same field.
        for (field, value) in [
            ("subject", self.subject.as_str()),
            ("approver.id", self.approver.id.as_str()),
            ("approved_at", self.approved_at.as_str()),
        ] {
            if value.trim().is_empty() {
                return Err(StoreError::InvalidConfig(format!(
                    "approval is missing required field {field}"
                )));
            }
        }
        let judgment_typed = self.subject_type == "judgment";
        if judgment_typed
            && self
                .judgment_ref
                .as_deref()
                .unwrap_or_default()
                .trim()
                .is_empty()
        {
            return Err(StoreError::InvalidConfig(
                "approval subject_type judgment requires judgment_ref".to_owned(),
            ));
        }
        if !judgment_typed && self.judgment_ref.is_some() {
            return Err(StoreError::InvalidConfig(
                "approval judgment_ref is only valid when subject_type is judgment".to_owned(),
            ));
        }
        if let Some(judgment_ref) = &self.judgment_ref {
            if !is_valid_ulid(judgment_ref) {
                return Err(StoreError::InvalidConfig(
                    "approval judgment_ref must be a valid ULID".to_owned(),
                ));
            }
        }
        for entry in &self.supersedes {
            if !is_valid_ulid(entry) {
                return Err(StoreError::InvalidConfig(format!(
                    "approval supersedes entry {entry} must be a valid ULID"
                )));
            }
            if entry == &self.id {
                return Err(StoreError::InvalidConfig(
                    "approval supersedes must not reference its own id".to_owned(),
                ));
            }
        }
        Ok(())
    }
}

impl RelationRecord {
    pub fn to_yaml(&self) -> Result<String, StoreError> {
        self.validate(None)?;
        yaml_serde::to_string(self).map_err(|error| {
            StoreError::InvalidConfig(format!("could not serialize relation: {error}"))
        })
    }

    /// Parses a `RelationRecord`. Returns `(Self, Vec<Diagnostic>)` to match
    /// the shape `vo_record_from_yaml` (`canonical.rs`) uses, though the
    /// vector here is always empty: Relation has no read-compat field like
    /// VO's `status` (DS-405), so every field outside `RELATION_KEYS` is a
    /// straightforward DS-1645/E-SCAN-010 rejection, not a warning — this
    /// goes through the same text -> `Value` -> known-key check -> typed
    /// struct shape `document_file_from_json`/`vo_record_from_yaml` use, so
    /// the unknown field is named in the error rather than silently
    /// dropped. `#[serde(deny_unknown_fields)]` on `RelationRecord` itself
    /// (above) is a second, independent enforcement of the same rule.
    pub fn from_yaml(text: &str, filename_id: &str) -> Result<(Self, Vec<Diagnostic>), StoreError> {
        let value: yaml_serde::Value = yaml_serde::from_str(text).map_err(|error| {
            StoreError::InvalidConfig(format!("invalid relation record: {error}"))
        })?;
        crate::canonical::reject_unknown_fields(&value, RELATION_KEYS, "")?;
        let record: Self = yaml_serde::from_value(value).map_err(|error| {
            StoreError::InvalidConfig(format!("invalid relation record: {error}"))
        })?;
        record.validate(Some(filename_id))?;
        Ok((record, Vec::new()))
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

/// DS-1645/E-SCAN-010: same fail-closed known-key scan as
/// `ApprovalRecord::from_yaml` above, applied to Evidence's own shape. The
/// existing hand-rolled extraction below (driven by `text`, unchanged) keeps
/// owning every other validation.
fn reject_unknown_evidence_fields(text: &str) -> Result<(), StoreError> {
    let value: yaml_serde::Value = yaml_serde::from_str(text)
        .map_err(|error| StoreError::InvalidConfig(format!("invalid Evidence record: {error}")))?;
    crate::canonical::reject_unknown_fields(&value, EVIDENCE_KEYS, "")?;
    if let Some(revision) = value.get("revision") {
        crate::canonical::reject_unknown_fields(revision, EVIDENCE_REVISION_KEYS, "revision.")?;
    }
    if let Some(hashes) = value.get("hashes") {
        crate::canonical::reject_unknown_fields(hashes, EVIDENCE_HASHES_KEYS, "hashes.")?;
    }
    if let Some(runner) = value.get("runner") {
        crate::canonical::reject_unknown_fields(runner, EVIDENCE_RUNNER_KEYS, "runner.")?;
    }
    if let Some(execution_state) = value.get("execution_state") {
        crate::canonical::reject_unknown_fields(
            execution_state,
            EVIDENCE_EXECUTION_STATE_KEYS,
            "execution_state.",
        )?;
    }
    if let Some(target_coverage) = value.get("target_coverage") {
        crate::canonical::reject_unknown_fields(
            target_coverage,
            EVIDENCE_TARGET_COVERAGE_KEYS,
            "target_coverage.",
        )?;
        if let Some(targets) = target_coverage
            .get("targets")
            .and_then(yaml_serde::Value::as_sequence)
        {
            for (index, target) in targets.iter().enumerate() {
                crate::canonical::reject_unknown_fields(
                    target,
                    EVIDENCE_TARGET_COVERAGE_TARGET_KEYS,
                    &format!("target_coverage.targets[{index}]."),
                )?;
            }
        }
    }
    Ok(())
}

fn read_target_coverage(text: &str) -> Result<TargetCoverage, StoreError> {
    let value: yaml_serde::Value = yaml_serde::from_str(text)
        .map_err(|error| StoreError::InvalidConfig(format!("invalid Evidence record: {error}")))?;
    let Some(value) = value.get("target_coverage") else {
        return Ok(TargetCoverage {
            checked: false,
            method: None,
            result: None,
            targets: Vec::new(),
            count: None,
        });
    };
    let mut coverage: TargetCoverage = yaml_serde::from_value(value.clone()).map_err(|error| {
        StoreError::InvalidConfig(format!("invalid Evidence target_coverage: {error}"))
    })?;

    if coverage.checked {
        if coverage.result.is_none() {
            return Err(StoreError::InvalidConfig(
                "checked Evidence target_coverage is missing result".to_owned(),
            ));
        }
    } else {
        match coverage.result {
            None | Some(vtest_model::TargetCoverageResult::Unknown) => {}
            Some(vtest_model::TargetCoverageResult::Pass)
            | Some(vtest_model::TargetCoverageResult::Fail) => {
                return Err(StoreError::InvalidConfig(
                    "unchecked Evidence target_coverage result must be null".to_owned(),
                ))
            }
        }
        // Compatibility normalization for the predecessor writer, which
        // encoded unchecked coverage as `result: UNKNOWN` (and could retain
        // the measurement method). The current canonical shape has no
        // measurement result, method, count, or per-target entries here.
        coverage.method = None;
        coverage.result = None;
        coverage.targets.clear();
        coverage.count = None;
    }
    Ok(coverage)
}

pub fn read_evidence(path: &Path) -> Result<EvidenceRecord, StoreError> {
    let text = read_text(path)?;
    reject_unknown_evidence_fields(&text)?;
    let fallback = path
        .file_stem()
        .and_then(|value| value.to_str())
        .unwrap_or_default();
    // DS-1657: "上流文書のレコードを除き、`id` とファイル名（拡張子除く）は
    // 一致しなければならない" applies schema-independently to every record
    // type Evidence is not excluded from; DES-032: "判断・承認・Evidenceの
    // IDはbare ULIDとする". A present-but-different `id` used to be silently
    // accepted (only an *absent* id fell back to the file name), the same
    // fail-open shape `read_approval` already closes for approvals.
    let id = scalar(&text, "id")
        .ok_or_else(|| StoreError::InvalidConfig("Evidence is missing id".to_owned()))?;
    if id != fallback {
        return Err(StoreError::InvalidConfig(format!(
            "Evidence id {id} does not match file name {fallback}"
        )));
    }
    if !is_valid_ulid(&id) {
        return Err(StoreError::InvalidConfig(format!(
            "Evidence id {id} is not a valid ULID"
        )));
    }
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
    let target_hashes = if text.lines().any(|line| line.trim() == "target_fns:") {
        let values = list(&text, "target_fns");
        if values.is_empty() {
            return Err(StoreError::InvalidConfig(
                "Evidence has an empty hashes.target_fns list".to_owned(),
            ));
        }
        let parsed = values
            .into_iter()
            .map(|value| {
                value
                    .parse()
                    .map_err(|error: String| StoreError::InvalidConfig(error))
            })
            .collect::<Result<Vec<ContentHash>, StoreError>>()?;
        if parsed.first() != Some(&target_hash) {
            return Err(StoreError::InvalidConfig(
                "Evidence hashes.target_fn must equal the first hashes.target_fns entry".to_owned(),
            ));
        }
        parsed
    } else {
        Vec::new()
    };
    let result = match scalar(&text, "result").as_deref() {
        Some("PASS") => TestResult::Pass,
        Some("FAIL") => TestResult::Fail,
        _ => {
            return Err(StoreError::InvalidConfig(
                "Evidence has invalid result".to_owned(),
            ))
        }
    };
    let target_coverage = read_target_coverage(&text)?;
    // DES-097/184: `execution_state.hash` is only meaningful once `complete`
    // is `true`. An absent `execution_state` mapping (a predecessor-shape
    // record written before this field existed) is read as an explicitly
    // incomplete state, not as a silently-matching one — DS-265's validity
    // AND-condition on Execution State subject then fails closed rather
    // than being skipped.
    let execution_state_complete =
        nested_scalar(&text, "execution_state", "complete").is_some_and(|value| value == "true");
    let execution_state_hash = nested_scalar(&text, "execution_state", "hash")
        .filter(|value| value != "null")
        .map(|value| value.parse::<ContentHash>())
        .transpose()
        .map_err(|error: String| StoreError::InvalidConfig(error))?;
    let execution_state = vtest_model::ExecutionState {
        schema: nested_scalar(&text, "execution_state", "schema").unwrap_or_default(),
        complete: execution_state_complete,
        hash: execution_state_hash,
    };
    Ok(EvidenceRecord {
        id,
        test_id: TestId::new(scalar(&text, "test_id").unwrap_or_default()),
        adapter: vtest_model::AdapterId::new(scalar(&text, "adapter").unwrap_or_default()),
        result,
        executed_at: scalar(&text, "executed_at").unwrap_or_default(),
        revision: Revision {
            commit: nested_scalar(&text, "revision", "commit").filter(|value| value != "null"),
            dirty: nested_scalar(&text, "revision", "dirty").is_some_and(|value| value == "true"),
        },
        execution_state,
        hashes: EvidenceHashes {
            test_fn: test_hash,
            target_fn: target_hash,
            target_fns: target_hashes,
        },
        runner: RunnerInfo {
            kind: nested_scalar(&text, "runner", "kind").unwrap_or_default(),
            command: nested_scalar(&text, "runner", "command").unwrap_or_default(),
            exit_code: nested_scalar(&text, "runner", "exit_code")
                .and_then(|value| value.parse().ok())
                .unwrap_or(-1),
        },
        target_coverage,
        log_ref: scalar(&text, "log_ref").unwrap_or_default(),
    })
}

/// Writes one Evidence record as a new append-only fact at `path`
/// (`write_new_record`: an existing file is an error, never replaced). The
/// text is exactly what `read_evidence` above accepts, so the two stay in
/// one file.
pub fn write_evidence(path: &Path, record: &EvidenceRecord) -> Result<(), StoreError> {
    write_new_record(path, &evidence_yaml(record))
}

fn evidence_yaml(record: &EvidenceRecord) -> String {
    let target = &record.target_coverage;
    format!(
        "id: {id}\ntest_id: {test_id}\nadapter: {adapter}\nresult: {result}\nexecuted_at: {executed_at}\nrevision:\n  commit: {commit}\n  dirty: {dirty}\nexecution_state:\n  schema: {es_schema}\n  complete: {es_complete}\n  hash: {es_hash}\nhashes:\n  test_fn: {test_fn}\n  target_fn: {target_fn}\n  target_fns:\n{target_fns}runner:\n  kind: {kind}\n  command: {command}\n  exit_code: {exit_code}\ntarget_coverage:\n  checked: {checked}\n  method: {method}\n  result: {target_result}\n{targets}  count: {count}\nlog_ref: {log_ref}\n",
        id = yaml_scalar(&record.id),
        test_id = yaml_scalar(record.test_id.as_str()),
        adapter = yaml_scalar(record.adapter.as_str()),
        result = yaml_scalar(match record.result { TestResult::Pass => "PASS", TestResult::Fail => "FAIL" }),
        executed_at = yaml_scalar(&record.executed_at),
        commit = record.revision.commit.as_deref().map(yaml_scalar).unwrap_or_else(|| "null".to_owned()),
        dirty = record.revision.dirty,
        es_schema = yaml_scalar(&record.execution_state.schema),
        es_complete = record.execution_state.complete,
        es_hash = record.execution_state.hash.as_ref().map(|hash| yaml_scalar(hash.as_str())).unwrap_or_else(|| "null".to_owned()),
        test_fn = yaml_scalar(record.hashes.test_fn.as_str()),
        target_fn = yaml_scalar(record.hashes.target_fn.as_str()),
        target_fns = if record.hashes.target_fns.is_empty() {
            format!("    - {}\n", yaml_scalar(record.hashes.target_fn.as_str()))
        } else {
            record
                .hashes
                .target_fns
                .iter()
                .map(|hash| format!("    - {}\n", yaml_scalar(hash.as_str())))
                .collect::<String>()
        },
        kind = yaml_scalar(&record.runner.kind),
        command = yaml_scalar(&record.runner.command),
        exit_code = record.runner.exit_code,
        checked = target.checked,
        method = target.method.as_deref().map(yaml_scalar).unwrap_or_else(|| "null".to_owned()),
        target_result = target
            .result
            .map(target_coverage_result_name)
            .map(yaml_scalar)
            .unwrap_or_else(|| "null".to_owned()),
        targets = target_coverage_targets_yaml(&target.targets),
        count = target.count.map(|value| value.to_string()).unwrap_or_else(|| "null".to_owned()),
        log_ref = yaml_scalar(&record.log_ref),
    )
}

fn target_coverage_result_name(result: TargetCoverageResult) -> &'static str {
    match result {
        TargetCoverageResult::Pass => "PASS",
        TargetCoverageResult::Fail => "FAIL",
        TargetCoverageResult::Unknown => "UNKNOWN",
    }
}

fn target_coverage_targets_yaml(targets: &[TargetCoverageTarget]) -> String {
    if targets.is_empty() {
        return "  targets: []\n".to_owned();
    }
    let mut yaml = String::from("  targets:\n");
    for target in targets {
        yaml.push_str(&format!(
            "    - target: {}\n      result: {}\n      count: {}\n",
            yaml_scalar(&target.target),
            yaml_scalar(target_coverage_result_name(target.result)),
            target
                .count
                .map(|value| value.to_string())
                .unwrap_or_else(|| "null".to_owned()),
        ));
    }
    yaml
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
    if !crate::canonical::is_valid_vo_id(id) {
        return Err(StoreError::InvalidVoId(id.to_owned()));
    }
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

pub fn read_relation(path: &Path) -> Result<(RelationRecord, Vec<Diagnostic>), StoreError> {
    let text = read_text(path)?;
    let fallback = path
        .file_stem()
        .and_then(|value| value.to_str())
        .unwrap_or_default();
    RelationRecord::from_yaml(&text, fallback)
}

/// Creates a new canonical Relation record and writes it to `.verify/rel/`.
/// The id is always generated here as `REL-<ULID>`: DES-124 ("writerは
/// `.verify/rel/REL-<ULID>.yaml` と同値の `id` だけを生成する") requires the
/// writer to emit only that form, even though `is_valid_relation_id` still
/// accepts a bare ULID for version 1 compatibility on read (DES-125).
pub fn write_relation(
    layout: &VerifyLayout,
    relation_type: RelationType,
    from: impl Into<String>,
    to: impl Into<String>,
    note: Option<String>,
    created: impl Into<String>,
) -> Result<RelationRecord, StoreError> {
    let record = RelationRecord {
        id: format!("REL-{}", new_record_id()),
        relation_type,
        from: from.into(),
        to: to.into(),
        note,
        created: created.into(),
    };
    let path = layout.relation_dir().join(format!("{}.yaml", record.id));
    write_new_record(&path, &record.to_yaml()?)?;
    Ok(record)
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

/// The canonical Relation ID form is `REL-` prefixed only (BD-023: "Relation
/// のIDは `REL-`（ULID）とし..."; DS-427: "canonical Relation IDは `REL-` と
/// 26文字のULID payloadからなる"). A bare ULID is accepted here only as
/// version 1 compatibility input on read (DES-125: "readerはversion 1
/// 互換入力として...bare `id` を受理し、`REL-<ULID>` へin-memoryで正規化する
/// が、読み取りだけでファイルを書き換えない"); `relation_ulid_payload` below
/// normalizes either shape to its ULID payload, which is always strictly
/// validated.
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

pub(crate) fn parse_combinations(text: &str) -> Vec<Vec<String>> {
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

pub(crate) fn scalar(text: &str, key: &str) -> Option<String> {
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

pub(crate) fn list(text: &str, key: &str) -> Vec<String> {
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

pub(crate) fn parse_inline_list(value: &str) -> Vec<String> {
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

pub(crate) fn yaml_list<'a>(values: impl Iterator<Item = &'a str>) -> String {
    let values = values.map(yaml_scalar).collect::<Vec<_>>();
    format!("[{}]", values.join(", "))
}

pub(crate) fn yaml_scalar(value: &str) -> String {
    format!("'{}'", value.replace('\'', "''"))
}

pub(crate) fn unquote(value: &str) -> String {
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

    /// @vtest.id TEST-STORE-RECORD-ID-BARE-ULID
    /// @vtest.covers VO-STORE-RECORD-ID-BARE-ULID
    /// @vtest.target crates/vtest-store/src/records.rs::new_record_id
    /// @vtest.intent verifies new_record_id generates unique, valid bare ULIDs (DES-032)
    #[test]
    fn generated_record_ids_are_valid_and_unique() {
        let ids = (0..4096).map(|_| new_record_id()).collect::<BTreeSet<_>>();
        assert_eq!(ids.len(), 4096);
        assert!(ids.iter().all(|id| is_valid_ulid(id)));
    }

    /// @vtest.id TEST-STORE-APPEND-ONLY-NO-OVERWRITE
    /// @vtest.covers VO-STORE-NEW-RECORD-CREATE-ONLY-NO-OVERWRITE
    /// @vtest.target crates/vtest-store/src/records.rs::write_new_record
    /// @vtest.intent verifies write_new_record refuses to replace an existing fact at the same path (BD-232)
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

    /// @vtest.id TEST-STORE-APPEND-ONLY-PUBLISH-COMPLETE
    /// @vtest.covers VO-STORE-ATOMIC-PUBLISH-COMPLETE-CONTENT
    /// @vtest.target crates/vtest-store/src/records.rs::write_new_record
    /// @vtest.intent verifies write_new_record publishes the complete content atomically, with no partial file (DES-474)
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

    /// @vtest.id TEST-STORE-ATOMIC-WRITE-ENTITY-REPLACE
    /// @vtest.covers VO-STORE-ATOMIC-PUBLISH-COMPLETE-CONTENT
    /// @vtest.target crates/vtest-store/src/records.rs::write_atomic
    /// @vtest.intent verifies write_atomic replaces an entity file with complete new content, no partial state (DES-474)
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

    fn sample_approval(id: &str) -> ApprovalRecord {
        ApprovalRecord {
            id: id.to_owned(),
            subject_type: "vo".to_owned(),
            subject: "VO-ONE".to_owned(),
            subject_hash: ContentHash::from_text("vo\n"),
            dependencies: vec![DependencyRecord {
                entity: "DOC-ROOT-001".to_owned(),
                hash: ContentHash::from_text("root\n"),
            }],
            judgment_ref: None,
            approver: Approver {
                kind: "human".to_owned(),
                id: "reviewer".to_owned(),
                model: None,
            },
            approved_state: "approved".to_owned(),
            basis: vec![new_record_id()],
            supersedes: Vec::new(),
            approved_at: "2026-08-08T00:00:00Z".to_owned(),
        }
    }

    /// @vtest.id TEST-STORE-APPROVAL-APPROVER-TRACEABLE
    /// @vtest.covers VO-STORE-APPROVAL-APPROVER-TRACEABLE
    /// @vtest.target crates/vtest-store/src/records.rs::ApprovalRecord::from_yaml
    /// @vtest.intent verifies approval round-trips and rejects a record whose approver fields are stripped (REQ-245)
    #[test]
    fn approval_round_trip_requires_a_traceable_approver() {
        let id = new_record_id();
        let record = sample_approval(&id);
        let yaml = record.to_yaml().unwrap();
        assert_eq!(ApprovalRecord::from_yaml(&yaml, &id).unwrap(), record);

        let malformed = yaml
            .lines()
            .filter(|line| !matches!(line.trim(), "approver:" | "kind: human" | "id: reviewer"))
            .collect::<Vec<_>>()
            .join("\n");
        assert!(ApprovalRecord::from_yaml(&malformed, &id).is_err());
    }

    /// DS-052 ("ツールはID形式を強制せず一意性のみを強制する") / DS-048
    /// (DOC/VO/TEST ids are grouped under the same rule): the record layer
    /// must not enforce a `subject` format or `"VO-"` prefix — only a
    /// scan-layer existence/uniqueness check is permitted, and this
    /// record-level reader does not have the VO set to run one. A subject
    /// that would fail the retired prefix/charset gate (lowercase letters,
    /// no `VO-` prefix) must still parse.
    ///
    /// @vtest.id TEST-STORE-APPROVAL-SUBJECT-NO-FORMAT-GATE
    /// @vtest.covers VO-STORE-APPROVAL-RECORD-SHAPE
    /// @vtest.target crates/vtest-store/src/records.rs::ApprovalRecord::from_yaml
    /// @vtest.intent verifies ApprovalRecord.subject is not format/prefix-checked (DS-052)
    #[test]
    fn approval_subject_is_not_format_or_prefix_checked() {
        let id = new_record_id();
        let mut record = sample_approval(&id);
        record.subject = "not-a-vo-shaped-id".to_owned();
        let yaml = record.to_yaml().unwrap();
        let parsed = ApprovalRecord::from_yaml(&yaml, &id)
            .expect("a subject with no VO- prefix and lowercase characters must still parse");
        assert_eq!(parsed.subject, "not-a-vo-shaped-id");
    }

    /// DS-1645/E-SCAN-010: an approval record carrying a scope-limiting
    /// field the reader does not recognize (e.g. an expiry or scope
    /// restriction) must fail closed rather than being silently discarded —
    /// discarding it would let a writer record "approved with a
    /// restriction" while a machine reads "approved unconditionally", the
    /// exact false-open path DES-586's own reasoning warns against for the
    /// document model's equivalent case.
    /// @vtest.id TEST-STORE-APPROVAL-UNKNOWN-TOP-LEVEL-FIELD
    /// @vtest.covers VO-STORE-APPROVAL-UNKNOWN-FIELD-REJECTED
    /// @vtest.target crates/vtest-store/src/records.rs::ApprovalRecord::from_yaml
    /// @vtest.intent verifies an unrecognized top-level approval field fails closed (E-SCAN-010 / DS-1676)
    #[test]
    fn approval_with_unknown_top_level_field_is_rejected() {
        let id = new_record_id();
        let record = sample_approval(&id);
        let mut yaml = record.to_yaml().unwrap();
        yaml.push_str("scope: read-only\n");
        let error = ApprovalRecord::from_yaml(&yaml, &id)
            .expect_err("an unrecognized top-level approval field must fail closed");
        assert!(matches!(error, StoreError::SchemaMismatch { .. }));
    }

    /// @vtest.id TEST-STORE-APPROVAL-UNKNOWN-NESTED-APPROVER-FIELD
    /// @vtest.covers VO-STORE-APPROVAL-UNKNOWN-FIELD-REJECTED
    /// @vtest.target crates/vtest-store/src/records.rs::ApprovalRecord::from_yaml
    /// @vtest.intent verifies an unrecognized nested approver field fails closed (E-SCAN-010 / DS-1676)
    #[test]
    fn approval_with_unknown_nested_approver_field_is_rejected() {
        let id = new_record_id();
        let yaml = format!(
            "id: {id}\nsubject_type: vo\nsubject: VO-ONE\nsubject_hash: {}\napprover:\n  kind: human\n  id: reviewer\n  weight: 2\napproved_state: approved\nbasis: []\nsupersedes: []\napproved_at: '2026-08-08T00:00:00Z'\n",
            ContentHash::from_text("vo\n"),
        );
        let error = ApprovalRecord::from_yaml(&yaml, &id)
            .expect_err("an unrecognized nested approver field must fail closed");
        assert!(matches!(error, StoreError::SchemaMismatch { .. }));
    }

    /// @vtest.id TEST-STORE-APPROVAL-BASIS-NON-STRING-ENTRY-REJECTED
    /// @vtest.covers VO-STORE-APPROVAL-BASIS-STRING-LIST
    /// @vtest.target crates/vtest-store/src/records.rs::ApprovalRecord::from_yaml
    /// @vtest.intent verifies a basis[] entry that is a mapping, not a bare string, is rejected (DS-1055)
    /// DS-1055/1196: `basis[]` is a bare list of free-form reference
    /// strings ("根拠参照", no `{kind, ref}` pair or `kind` value domain —
    /// team-lead ruling 2026-09-10). A `basis[]` entry that is a mapping
    /// rather than a plain string is rejected (a type mismatch against
    /// `Vec<String>`, not a per-entry unknown-field scan, since there is no
    /// longer a nested shape to scan).
    #[test]
    fn approval_with_a_non_string_basis_entry_is_rejected() {
        let id = new_record_id();
        let yaml = format!(
            "id: {id}\nsubject_type: vo\nsubject: VO-ONE\nsubject_hash: {}\napprover:\n  kind: human\n  id: reviewer\napproved_state: approved\nbasis:\n  - kind: audit\n    ref: {}\nsupersedes: []\napproved_at: '2026-08-08T00:00:00Z'\n",
            ContentHash::from_text("vo\n"),
            new_record_id(),
        );
        assert!(
            ApprovalRecord::from_yaml(&yaml, &id).is_err(),
            "a basis[] entry that is a mapping, not a bare string, must be rejected"
        );
    }

    /// @vtest.id TEST-STORE-APPROVAL-BASIS-BARE-STRING-LIST
    /// @vtest.covers VO-STORE-APPROVAL-BASIS-STRING-LIST
    /// @vtest.target crates/vtest-store/src/records.rs::ApprovalRecord::to_yaml
    /// @vtest.intent verifies a plain basis[] of free-form reference strings round-trips (DS-1055)
    /// DS-1055/1196: a plain `basis[]` of free-form reference strings
    /// round-trips.
    #[test]
    fn approval_basis_is_a_bare_string_list() {
        let mut record = sample_approval(&new_record_id());
        record.basis = vec!["some free-form reference".to_owned()];
        let yaml = record.to_yaml().expect("valid basis must serialize");
        let read = ApprovalRecord::from_yaml(&yaml, &record.id).expect("must round-trip");
        assert_eq!(read.basis, vec!["some free-form reference".to_owned()]);
    }

    /// @vtest.id TEST-STORE-APPROVAL-NON-STRING-TOP-LEVEL-KEY-REJECTED
    /// @vtest.covers VO-STORE-APPROVAL-UNKNOWN-FIELD-REJECTED
    /// @vtest.target crates/vtest-store/src/records.rs::ApprovalRecord::from_yaml
    /// @vtest.intent verifies a non-string top-level key fails closed rather than being silently skipped (E-SCAN-010 / DS-1676)
    /// DS-1645: `reject_unknown_fields` used to silently skip any mapping
    /// key that was not a YAML string (`key.as_str()` returning `None`),
    /// relying on a `from_value` deserialize elsewhere to reject the type
    /// mismatch. A surplus field written with an integer/bool/null key
    /// (e.g. a numeric-looking scope-limiting field like `2026: unlimited`)
    /// used to pass through unrejected.
    #[test]
    fn approval_with_non_string_top_level_key_is_rejected() {
        let id = new_record_id();
        let record = sample_approval(&id);
        for extra in ["2026: unlimited\n", "true: unlimited\n", "~: unlimited\n"] {
            let mut yaml = record.to_yaml().unwrap();
            yaml.push_str(extra);
            let error = ApprovalRecord::from_yaml(&yaml, &id).expect_err(&format!(
                "a non-string top-level key ({extra:?}) must fail closed, not be silently skipped"
            ));
            assert!(matches!(error, StoreError::SchemaMismatch { .. }));
        }
    }

    /// @vtest.id TEST-STORE-APPROVAL-NON-STRING-NESTED-APPROVER-KEY-REJECTED
    /// @vtest.covers VO-STORE-APPROVAL-UNKNOWN-FIELD-REJECTED
    /// @vtest.target crates/vtest-store/src/records.rs::ApprovalRecord::from_yaml
    /// @vtest.intent verifies a non-string nested approver key fails closed (E-SCAN-010 / DS-1676)
    /// Same gap as above, on the nested `approver` mapping.
    #[test]
    fn approval_with_non_string_nested_approver_key_is_rejected() {
        let id = new_record_id();
        let yaml = format!(
            "id: {id}\nsubject_type: vo\nsubject: VO-ONE\nsubject_hash: {}\napprover:\n  kind: human\n  id: reviewer\n  7: extra\napproved_state: approved\nbasis: []\nsupersedes: []\napproved_at: '2026-08-08T00:00:00Z'\n",
            ContentHash::from_text("vo\n"),
        );
        let error = ApprovalRecord::from_yaml(&yaml, &id)
            .expect_err("a non-string nested approver key must fail closed");
        assert!(matches!(error, StoreError::SchemaMismatch { .. }));
    }

    /// @vtest.id TEST-STORE-APPROVAL-JUDGMENT-SUBJECT-REQUIRES-REF
    /// @vtest.covers VO-STORE-APPROVAL-JUDGMENT-SUBJECT-REQUIRES-REF
    /// @vtest.target crates/vtest-store/src/records.rs::ApprovalRecord::to_yaml
    /// @vtest.intent verifies subject_type judgment without judgment_ref is rejected (DES-150)
    #[test]
    fn approval_subject_type_judgment_requires_judgment_ref() {
        let id = new_record_id();
        let mut record = sample_approval(&id);
        record.subject_type = "judgment".to_owned();
        // judgment_ref left None: DS-1052/1476/1483 require it whenever
        // subject_type is judgment.
        assert!(record.to_yaml().is_err());
    }

    /// @vtest.id TEST-STORE-APPROVAL-OUT-OF-DOMAIN-SUBJECT-TYPE-REJECTED
    /// @vtest.covers VO-STORE-APPROVAL-SUBJECT-TYPE-DOMAIN
    /// @vtest.target crates/vtest-store/src/records.rs::ApprovalRecord::to_yaml
    /// @vtest.intent verifies a subject_type outside {vo, document, judgment} is rejected (DS-1196)
    #[test]
    fn approval_out_of_domain_subject_type_is_rejected() {
        let id = new_record_id();
        let mut record = sample_approval(&id);
        record.subject_type = "test".to_owned();
        assert!(record.to_yaml().is_err());
    }

    /// @vtest.id TEST-STORE-APPROVAL-OUT-OF-DOMAIN-STATE-REJECTED
    /// @vtest.covers VO-STORE-APPROVAL-STATE-DOMAIN
    /// @vtest.target crates/vtest-store/src/records.rs::ApprovalRecord::to_yaml
    /// @vtest.intent verifies an approved_state outside {approved, rejected, withdrawn} is rejected (DS-458)
    #[test]
    fn approval_out_of_domain_approved_state_is_rejected() {
        let id = new_record_id();
        let mut record = sample_approval(&id);
        record.approved_state = "maybe".to_owned();
        assert!(record.to_yaml().is_err());
    }

    /// @vtest.id TEST-STORE-APPROVAL-SELF-REFERENCING-SUPERSEDES-REJECTED
    /// @vtest.covers VO-STORE-APPROVAL-SUPERSEDES-NO-SELF-REFERENCE
    /// @vtest.target crates/vtest-store/src/records.rs::ApprovalRecord::to_yaml
    /// @vtest.intent verifies a supersedes entry referencing the record's own id is rejected (DS-462)
    #[test]
    fn approval_self_referencing_supersedes_is_rejected() {
        let id = new_record_id();
        let mut record = sample_approval(&id);
        record.supersedes = vec![id.clone()];
        assert!(record.to_yaml().is_err());
    }

    /// @vtest.id TEST-STORE-EVIDENCE-ID-FILENAME-ULID-INVARIANTS
    /// @vtest.covers VO-STORE-EVIDENCE-ID-FILENAME-MATCH, VO-STORE-EVIDENCE-ID-BARE-ULID
    /// @vtest.target crates/vtest-store/src/records.rs::read_evidence
    /// @vtest.intent verifies read_evidence enforces id/file-name match and bare-ULID form (DS-1657, DES-032)
    /// DS-1657 ("上流文書のレコードを除き、`id` とファイル名は一致しなければ
    /// ならない") and DES-032 ("判断・承認・EvidenceのIDはbare ULIDとする")
    /// both apply to Evidence the same as any other record type;
    /// `read_evidence` used to accept a present-but-different `id` silently
    /// (only an absent one fell back to the file name).
    #[test]
    fn read_evidence_enforces_id_file_name_and_ulid_invariants() {
        let root = temporary_directory("read-evidence");
        let id = new_record_id();
        let base = format!(
            "id: {id}\ntest_id: TEST-X\nresult: PASS\nexecuted_at: '2026-08-08T00:00:00Z'\nhashes:\n  test_fn: {}\n  target_fn: {}\nrunner:\n  kind: cargo\n  command: 'cargo test'\n  exit_code: 0\nlog_ref: ''\n",
            ContentHash::from_text("test body\n"),
            ContentHash::from_text("target body\n"),
        );

        let matching_path = root.join(format!("{id}.yaml"));
        fs::write(&matching_path, &base).unwrap();
        assert!(
            read_evidence(&matching_path).is_ok(),
            "a well-formed Evidence record with a matching bare-ULID id must parse"
        );

        let mismatched_path = root.join(format!("{}.yaml", new_record_id()));
        fs::write(&mismatched_path, &base).unwrap();
        let error = read_evidence(&mismatched_path)
            .expect_err("an Evidence id that disagrees with the file name must fail closed");
        assert!(error.to_string().contains("does not match file name"));

        let non_ulid_id = "not-a-ulid";
        let non_ulid_yaml = base.replacen(&id, non_ulid_id, 1);
        let non_ulid_path = root.join(format!("{non_ulid_id}.yaml"));
        fs::write(&non_ulid_path, &non_ulid_yaml).unwrap();
        let error = read_evidence(&non_ulid_path)
            .expect_err("a non-ULID Evidence id must fail closed even if it matches the file name");
        assert!(error.to_string().contains("not a valid ULID"));

        let missing_id_yaml = base
            .lines()
            .filter(|line| !line.starts_with("id:"))
            .collect::<Vec<_>>()
            .join("\n");
        fs::write(&matching_path, &missing_id_yaml).unwrap();
        assert!(
            read_evidence(&matching_path).is_err(),
            "an Evidence record missing id entirely must fail closed, not fall back to the file name"
        );
    }

    fn full_evidence_record() -> EvidenceRecord {
        EvidenceRecord {
            id: "01J8XW1B2C3D4E5F6G7H8J9K0M".to_owned(),
            test_id: TestId::new("TEST-PARSER-044"),
            adapter: vtest_model::AdapterId::new("rust-cargo"),
            result: TestResult::Pass,
            executed_at: "2026-08-08T00:00:00Z".to_owned(),
            revision: Revision {
                commit: Some("0123456789abcdef0123456789abcdef01234567".to_owned()),
                dirty: false,
            },
            execution_state: vtest_model::ExecutionState {
                schema: "rust-cargo-execution-state-v1".to_owned(),
                complete: true,
                hash: Some(ContentHash::from_text("execution state\n")),
            },
            hashes: EvidenceHashes {
                test_fn: ContentHash::from_text("test body\n"),
                target_fn: ContentHash::from_text("first target\n"),
                target_fns: vec![
                    ContentHash::from_text("first target\n"),
                    ContentHash::from_text("second target\n"),
                ],
            },
            runner: RunnerInfo {
                kind: "cargo".to_owned(),
                command: "cargo test -- 'parser::parses' --exact".to_owned(),
                exit_code: 0,
            },
            target_coverage: TargetCoverage {
                checked: true,
                method: Some("llvm-cov".to_owned()),
                result: Some(TargetCoverageResult::Unknown),
                targets: vec![
                    TargetCoverageTarget {
                        target: "rust-cargo::src/parser.rs::Parser::parse".to_owned(),
                        result: TargetCoverageResult::Pass,
                        count: Some(3),
                    },
                    TargetCoverageTarget {
                        target: "rust-cargo::src/parser.rs::Parser::reset".to_owned(),
                        result: TargetCoverageResult::Unknown,
                        count: None,
                    },
                ],
                count: None,
            },
            log_ref: "cache/logs/01J8XW1B2C3D4E5F6G7H8J9K0M.log".to_owned(),
        }
    }

    fn minimal_evidence_record() -> EvidenceRecord {
        EvidenceRecord {
            id: "01J8XW1B2C3D4E5F6G7H8J9K0N".to_owned(),
            test_id: TestId::new("TEST-X"),
            adapter: vtest_model::AdapterId::new("ruby-minitest"),
            result: TestResult::Fail,
            executed_at: "2026-08-08T00:00:00Z".to_owned(),
            revision: Revision {
                commit: None,
                dirty: true,
            },
            execution_state: vtest_model::ExecutionState {
                schema: "ruby-minitest-execution-state-v1".to_owned(),
                complete: false,
                hash: None,
            },
            hashes: EvidenceHashes {
                test_fn: ContentHash::from_text("test body\n"),
                target_fn: ContentHash::from_text(""),
                target_fns: Vec::new(),
            },
            runner: RunnerInfo {
                kind: "minitest".to_owned(),
                command: "ruby -Itest test/x_test.rb".to_owned(),
                exit_code: 1,
            },
            target_coverage: TargetCoverage {
                checked: false,
                method: None,
                result: None,
                targets: Vec::new(),
                count: None,
            },
            log_ref: "cache/logs/01J8XW1B2C3D4E5F6G7H8J9K0N.log".to_owned(),
        }
    }

    /// @vtest.id TEST-STORE-EVIDENCE-WRITER-BYTES-AND-ROUND-TRIP
    /// @vtest.covers VO-STORE-EVIDENCE-UNKNOWN-FIELD-REJECTED
    /// @vtest.target crates/vtest-store/src/records.rs::write_evidence
    /// @vtest.intent verifies write_evidence emits byte-for-byte the record text captured from the writer before it moved here from vtest-exec, and that read_evidence accepts it without a schema mismatch
    #[test]
    fn write_evidence_emits_the_fixed_text_and_read_evidence_accepts_it() {
        let full_text = concat!(
            "id: '01J8XW1B2C3D4E5F6G7H8J9K0M'\n",
            "test_id: 'TEST-PARSER-044'\n",
            "adapter: 'rust-cargo'\n",
            "result: 'PASS'\n",
            "executed_at: '2026-08-08T00:00:00Z'\n",
            "revision:\n",
            "  commit: '0123456789abcdef0123456789abcdef01234567'\n",
            "  dirty: false\n",
            "execution_state:\n",
            "  schema: 'rust-cargo-execution-state-v1'\n",
            "  complete: true\n",
            "  hash: 'sha256:526676ad8c6dbe6bfe5c92eca4382c4b5a353eabce5e75c8571f9e2fc9a1d3f8'\n",
            "hashes:\n",
            "  test_fn: 'sha256:94ceebc543d66991bc51e0a0261972e2e4b9d0b29693eddf000bcc5871798727'\n",
            "  target_fn: 'sha256:a64b42ee65bc60b078a457c773acdbde3aa0252c2573771e0a73ec127919997c'\n",
            "  target_fns:\n",
            "    - 'sha256:a64b42ee65bc60b078a457c773acdbde3aa0252c2573771e0a73ec127919997c'\n",
            "    - 'sha256:045beed3260c795fd4bf80bb3fbbacef325ca559c25615ec067ddd67ea91dba6'\n",
            "runner:\n",
            "  kind: 'cargo'\n",
            "  command: 'cargo test -- ''parser::parses'' --exact'\n",
            "  exit_code: 0\n",
            "target_coverage:\n",
            "  checked: true\n",
            "  method: 'llvm-cov'\n",
            "  result: 'UNKNOWN'\n",
            "  targets:\n",
            "    - target: 'rust-cargo::src/parser.rs::Parser::parse'\n",
            "      result: 'PASS'\n",
            "      count: 3\n",
            "    - target: 'rust-cargo::src/parser.rs::Parser::reset'\n",
            "      result: 'UNKNOWN'\n",
            "      count: null\n",
            "  count: null\n",
            "log_ref: 'cache/logs/01J8XW1B2C3D4E5F6G7H8J9K0M.log'\n",
        );
        let minimal_text = concat!(
            "id: '01J8XW1B2C3D4E5F6G7H8J9K0N'\n",
            "test_id: 'TEST-X'\n",
            "adapter: 'ruby-minitest'\n",
            "result: 'FAIL'\n",
            "executed_at: '2026-08-08T00:00:00Z'\n",
            "revision:\n",
            "  commit: null\n",
            "  dirty: true\n",
            "execution_state:\n",
            "  schema: 'ruby-minitest-execution-state-v1'\n",
            "  complete: false\n",
            "  hash: null\n",
            "hashes:\n",
            "  test_fn: 'sha256:94ceebc543d66991bc51e0a0261972e2e4b9d0b29693eddf000bcc5871798727'\n",
            "  target_fn: 'sha256:e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855'\n",
            "  target_fns:\n",
            "    - 'sha256:e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855'\n",
            "runner:\n",
            "  kind: 'minitest'\n",
            "  command: 'ruby -Itest test/x_test.rb'\n",
            "  exit_code: 1\n",
            "target_coverage:\n",
            "  checked: false\n",
            "  method: null\n",
            "  result: null\n",
            "  targets: []\n",
            "  count: null\n",
            "log_ref: 'cache/logs/01J8XW1B2C3D4E5F6G7H8J9K0N.log'\n",
        );
        let root = temporary_directory("write-evidence");

        let full = full_evidence_record();
        let full_path = root.join(format!("{}.yaml", full.id));
        write_evidence(&full_path, &full).unwrap();
        assert_eq!(fs::read_to_string(&full_path).unwrap(), full_text);
        assert_eq!(read_evidence(&full_path).unwrap(), full);
        assert!(
            write_evidence(&full_path, &full).is_err(),
            "an existing Evidence record must never be replaced"
        );

        let minimal = minimal_evidence_record();
        let minimal_path = root.join(format!("{}.yaml", minimal.id));
        write_evidence(&minimal_path, &minimal).unwrap();
        assert_eq!(fs::read_to_string(&minimal_path).unwrap(), minimal_text);
        // An empty `target_fns` is written as the single `target_fn`, so that
        // is what comes back.
        let mut read_back = minimal.clone();
        read_back.hashes.target_fns = vec![minimal.hashes.target_fn.clone()];
        assert_eq!(read_evidence(&minimal_path).unwrap(), read_back);
    }

    /// @vtest.id TEST-STORE-EVIDENCE-UNCHECKED-COVERAGE-LEGACY-UNKNOWN-NORMALIZED
    /// @vtest.covers VO-EXEC-COVERAGE-UNAVAILABLE-NOT-CHECKED
    /// @vtest.target crates/vtest-store/src/records.rs::read_evidence
    /// @vtest.intent verifies the predecessor checked:false/result:UNKNOWN wire shape is normalized to the canonical null/null/empty-list shape
    #[test]
    fn read_evidence_normalizes_legacy_unchecked_coverage() {
        let root = temporary_directory("read-evidence-legacy-unchecked-coverage");
        let id = new_record_id();
        let path = root.join(format!("{id}.yaml"));
        let yaml = format!(
            "id: {id}\ntest_id: TEST-X\nresult: PASS\nexecuted_at: '2026-08-08T00:00:00Z'\nhashes:\n  test_fn: {test_hash}\n  target_fn: {target_hash}\ntarget_coverage:\n  checked: false\n  method: llvm-cov\n  result: UNKNOWN\n  targets: []\n  count: 0\nrunner:\n  kind: cargo\n  command: 'cargo test'\n  exit_code: 0\nlog_ref: ''\n",
            test_hash = ContentHash::from_text("test body\n"),
            target_hash = ContentHash::from_text("target body\n"),
        );
        fs::write(&path, yaml).unwrap();

        let record = read_evidence(&path).expect("legacy unchecked Evidence must remain readable");
        assert!(!record.target_coverage.checked);
        assert_eq!(record.target_coverage.method, None);
        assert_eq!(record.target_coverage.result, None);
        assert!(record.target_coverage.targets.is_empty());
        assert_eq!(record.target_coverage.count, None);
    }

    /// @vtest.id TEST-STORE-EVIDENCE-CHECKED-COVERAGE-RESULT-DOMAIN-REJECTED
    /// @vtest.covers VO-STORE-EVIDENCE-UNKNOWN-FIELD-REJECTED
    /// @vtest.target crates/vtest-store/src/records.rs::read_evidence
    /// @vtest.intent verifies checked:true target coverage rejects a result outside PASS/FAIL/UNKNOWN instead of coercing it to UNKNOWN
    #[test]
    fn read_evidence_rejects_checked_coverage_result_outside_domain() {
        let root = temporary_directory("read-evidence-invalid-coverage-result");
        let id = new_record_id();
        let path = root.join(format!("{id}.yaml"));
        let yaml = format!(
            "id: {id}\ntest_id: TEST-X\nresult: PASS\nexecuted_at: '2026-08-08T00:00:00Z'\nhashes:\n  test_fn: {test_hash}\n  target_fn: {target_hash}\ntarget_coverage:\n  checked: true\n  method: llvm-cov\n  result: BOGUS\n  targets: []\n  count: 1\nrunner:\n  kind: cargo\n  command: 'cargo test'\n  exit_code: 0\nlog_ref: ''\n",
            test_hash = ContentHash::from_text("test body\n"),
            target_hash = ContentHash::from_text("target body\n"),
        );
        fs::write(&path, yaml).unwrap();

        let error = read_evidence(&path)
            .expect_err("checked coverage with an out-of-domain result must fail closed");
        assert!(error.to_string().contains("target_coverage"));
    }

    /// @vtest.id TEST-STORE-EVIDENCE-UNKNOWN-TOP-LEVEL-FIELD-REJECTED
    /// @vtest.covers VO-STORE-EVIDENCE-UNKNOWN-FIELD-REJECTED
    /// @vtest.target crates/vtest-store/src/records.rs::read_evidence
    /// @vtest.intent verifies an unrecognized top-level Evidence field fails closed (E-SCAN-010 / DS-1676)
    /// DS-1645/E-SCAN-010: an Evidence record carrying a field this reader
    /// does not recognize must fail closed rather than being silently
    /// discarded — the same rule already applied to Document/VO/Relation,
    /// now closed for Evidence too (it is a canonical entity on the
    /// `Document` → `VO` → `Test` → `Target` → `Evidence` →
    /// `VerificationResult` chain, not a PR8 predecessor).
    #[test]
    fn read_evidence_rejects_unknown_top_level_field() {
        let root = temporary_directory("read-evidence-unknown-top-level");
        let id = new_record_id();
        let path = root.join(format!("{id}.yaml"));
        let yaml = format!(
            "id: {id}\ntest_id: TEST-X\nresult: PASS\nexecuted_at: '2026-08-08T00:00:00Z'\nhashes:\n  test_fn: {}\n  target_fn: {}\nrunner:\n  kind: cargo\n  command: 'cargo test'\n  exit_code: 0\nlog_ref: ''\nnotes: extra\n",
            ContentHash::from_text("test body\n"),
            ContentHash::from_text("target body\n"),
        );
        fs::write(&path, &yaml).unwrap();
        let error = read_evidence(&path)
            .expect_err("an unrecognized top-level Evidence field must fail closed");
        assert!(matches!(error, StoreError::SchemaMismatch { .. }));
    }

    /// @vtest.id TEST-STORE-EVIDENCE-UNKNOWN-NESTED-HASHES-FIELD-REJECTED
    /// @vtest.covers VO-STORE-EVIDENCE-UNKNOWN-FIELD-REJECTED
    /// @vtest.target crates/vtest-store/src/records.rs::read_evidence
    /// @vtest.intent verifies an unrecognized nested hashes field fails closed (E-SCAN-010 / DS-1676)
    #[test]
    fn read_evidence_rejects_unknown_nested_hashes_field() {
        let root = temporary_directory("read-evidence-unknown-hashes");
        let id = new_record_id();
        let path = root.join(format!("{id}.yaml"));
        let yaml = format!(
            "id: {id}\ntest_id: TEST-X\nresult: PASS\nexecuted_at: '2026-08-08T00:00:00Z'\nhashes:\n  test_fn: {}\n  target_fn: {}\n  algorithm: sha256\nrunner:\n  kind: cargo\n  command: 'cargo test'\n  exit_code: 0\nlog_ref: ''\n",
            ContentHash::from_text("test body\n"),
            ContentHash::from_text("target body\n"),
        );
        fs::write(&path, &yaml).unwrap();
        let error =
            read_evidence(&path).expect_err("an unrecognized nested hashes field must fail closed");
        assert!(matches!(error, StoreError::SchemaMismatch { .. }));
    }

    /// @vtest.id TEST-STORE-EVIDENCE-NON-STRING-TOP-LEVEL-KEY-REJECTED
    /// @vtest.covers VO-STORE-EVIDENCE-UNKNOWN-FIELD-REJECTED
    /// @vtest.target crates/vtest-store/src/records.rs::read_evidence
    /// @vtest.intent verifies a non-string top-level key fails closed rather than being silently skipped (E-SCAN-010 / DS-1676)
    /// DS-1645: same non-string-key gap as `ApprovalRecord::from_yaml` (see
    /// `approval_with_non_string_top_level_key_is_rejected`) — `read_evidence`
    /// also never builds a typed struct from the `Value` this scan runs
    /// over, so a surplus field written with a non-string key used to be
    /// silently skipped instead of rejected.
    #[test]
    fn read_evidence_rejects_non_string_top_level_key() {
        let root = temporary_directory("read-evidence-non-string-key");
        let id = new_record_id();
        let path = root.join(format!("{id}.yaml"));
        let yaml = format!(
            "id: {id}\ntest_id: TEST-X\nresult: PASS\nexecuted_at: '2026-08-08T00:00:00Z'\nhashes:\n  test_fn: {}\n  target_fn: {}\nrunner:\n  kind: cargo\n  command: 'cargo test'\n  exit_code: 0\nlog_ref: ''\n2026: unlimited\n",
            ContentHash::from_text("test body\n"),
            ContentHash::from_text("target body\n"),
        );
        fs::write(&path, &yaml).unwrap();
        let error = read_evidence(&path)
            .expect_err("a non-string top-level key must fail closed, not be silently skipped");
        assert!(matches!(error, StoreError::SchemaMismatch { .. }));
    }

    /// @vtest.id TEST-STORE-EVIDENCE-NON-STRING-NESTED-HASHES-KEY-REJECTED
    /// @vtest.covers VO-STORE-EVIDENCE-UNKNOWN-FIELD-REJECTED
    /// @vtest.target crates/vtest-store/src/records.rs::read_evidence
    /// @vtest.intent verifies a non-string nested hashes key fails closed (E-SCAN-010 / DS-1676)
    #[test]
    fn read_evidence_rejects_non_string_nested_hashes_key() {
        let root = temporary_directory("read-evidence-non-string-nested-key");
        let id = new_record_id();
        let path = root.join(format!("{id}.yaml"));
        let yaml = format!(
            "id: {id}\ntest_id: TEST-X\nresult: PASS\nexecuted_at: '2026-08-08T00:00:00Z'\nhashes:\n  test_fn: {}\n  target_fn: {}\n  9: extra\nrunner:\n  kind: cargo\n  command: 'cargo test'\n  exit_code: 0\nlog_ref: ''\n",
            ContentHash::from_text("test body\n"),
            ContentHash::from_text("target body\n"),
        );
        fs::write(&path, &yaml).unwrap();
        let error = read_evidence(&path)
            .expect_err("a non-string nested hashes key must fail closed, not be silently skipped");
        assert!(matches!(error, StoreError::SchemaMismatch { .. }));
    }

    /// @vtest.id TEST-STORE-RELATION-ROUND-TRIP-VALID-RECORD
    /// @vtest.covers VO-STORE-RELATION-RECORD-SHAPE
    /// @vtest.target crates/vtest-store/src/records.rs::RelationRecord::from_yaml
    /// @vtest.intent verifies Relation round-trips and rejects an out-of-domain type or empty from/to/created (DES-123)
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
        let (parsed, diagnostics) = RelationRecord::from_yaml(&yaml, &id).unwrap();
        assert_eq!(parsed, record);
        assert!(diagnostics.is_empty());

        for malformed in [
            yaml.replacen("type: complements", "type: unknown", 1),
            yaml.replacen("from: TEST-PARSER-044", "from: ''", 1),
            yaml.replacen("to: TEST-PARSER-012", "to: ''", 1),
            yaml.replacen("created: 2026-08-08T00:00:00Z", "created: ''", 1),
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
            RelationRecord::from_yaml(&prefixed_yaml, &prefixed_id)
                .unwrap()
                .0,
            prefixed
        );

        let invalid = RelationRecord {
            id: "not-a-ulid".to_owned(),
            ..record
        };
        assert!(invalid.to_yaml().is_err());
        let invalid = RelationRecord {
            from: String::new(),
            ..RelationRecord::from_yaml(&yaml, &id).unwrap().0
        };
        assert!(invalid.to_yaml().is_err());
    }

    /// @vtest.id TEST-STORE-WRITE-RELATION-REL-PREFIXED-ID
    /// @vtest.covers VO-STORE-RELATION-WRITER-REL-PREFIXED-ID
    /// @vtest.target crates/vtest-store/src/records.rs::write_relation
    /// @vtest.intent verifies write_relation always generates an id starting with REL- (DES-124)
    #[test]
    fn write_relation_always_generates_a_rel_prefixed_id() {
        let root = temporary_directory("write-relation");
        let layout = crate::init_project(&root, "example").unwrap();

        let record = write_relation(
            &layout,
            RelationType::DependsOn,
            "TEST-PARSER-044",
            "TEST-PARSER-012",
            None,
            "2026-08-08T00:00:00Z",
        )
        .unwrap();

        assert!(
            record.id.starts_with("REL-"),
            "expected a REL-prefixed id, got {}",
            record.id
        );

        let path = layout.relation_dir().join(format!("{}.yaml", record.id));
        assert_eq!(read_relation(&path).unwrap().0, record);
    }

    /// @vtest.id TEST-STORE-RELATION-UNKNOWN-TOP-LEVEL-FIELD-REJECTED
    /// @vtest.covers VO-STORE-RELATION-UNKNOWN-FIELD-REJECTED
    /// @vtest.target crates/vtest-store/src/records.rs::RelationRecord::from_yaml
    /// @vtest.intent verifies an unknown top-level Relation field fails closed (E-SCAN-010 / DS-1676)
    /// DS-1645/E-SCAN-010: an unknown field is rejected, not merely warned
    /// about — this replaces the retired DS-376 "warn and continue"
    /// behavior (`docs/canonical/relations/retired-ids.json`), exercised
    /// here for the Relation reader that lives in this module instead of
    /// `canonical.rs`.
    #[test]
    fn relation_with_unknown_top_level_field_is_rejected() {
        let id = new_record_id();
        let record = RelationRecord {
            id: id.clone(),
            relation_type: RelationType::Complements,
            from: "TEST-PARSER-044".to_owned(),
            to: "TEST-PARSER-012".to_owned(),
            note: None,
            created: "2026-08-08T00:00:00Z".to_owned(),
        };
        let mut yaml = record.to_yaml().unwrap();
        yaml.push_str("owner: someone\n");
        let error = RelationRecord::from_yaml(&yaml, &id)
            .expect_err("an unknown top-level field must fail closed");
        assert!(matches!(error, StoreError::SchemaMismatch { .. }));
    }

    /// @vtest.id TEST-STORE-RELATION-NON-STRING-TOP-LEVEL-KEY-REJECTED
    /// @vtest.covers VO-STORE-RELATION-UNKNOWN-FIELD-REJECTED
    /// @vtest.target crates/vtest-store/src/records.rs::RelationRecord::from_yaml
    /// @vtest.intent verifies a non-string top-level Relation key fails closed (E-SCAN-010 / DS-1676)
    /// Unlike `ApprovalRecord`/`read_evidence` above, `RelationRecord` is
    /// always backstopped by both `#[serde(deny_unknown_fields)]` and a
    /// `yaml_serde::from_value` pass after `reject_unknown_fields` runs —
    /// independently caught there even before `reject_unknown_fields`
    /// itself started rejecting non-string keys (DS-1645; this test does
    /// not isolate which of the two layers rejects first). Locks in that at
    /// least one of them continues to reject this shape, guarding against
    /// regression in either.
    #[test]
    fn relation_with_non_string_top_level_key_is_rejected() {
        let id = new_record_id();
        let record = RelationRecord {
            id: id.clone(),
            relation_type: RelationType::Complements,
            from: "TEST-PARSER-044".to_owned(),
            to: "TEST-PARSER-012".to_owned(),
            note: None,
            created: "2026-08-08T00:00:00Z".to_owned(),
        };
        let mut yaml = record.to_yaml().unwrap();
        yaml.push_str("2026: unlimited\n");
        RelationRecord::from_yaml(&yaml, &id)
            .expect_err("a non-string top-level key must fail closed");
    }

    /// Guards `RELATION_KEYS` against drifting out of sync with
    /// `RelationRecord`'s actual fields: `note` is populated (`Some`), so
    /// its `skip_serializing_if` does not omit the key from the shape.
    /// @vtest.id TEST-STORE-RELATION-KNOWN-KEYS-MATCH-SHAPE
    /// @vtest.covers VO-STORE-RELATION-UNKNOWN-FIELD-REJECTED
    /// @vtest.target crates/vtest-store/src/records.rs::RelationRecord::from_yaml
    /// @vtest.intent the known-key list used to reject undeclared Relation fields matches the record shape
    #[test]
    fn relation_known_keys_match_the_record_shape() {
        let record = RelationRecord {
            id: new_record_id(),
            relation_type: RelationType::Complements,
            from: "TEST-PARSER-044".to_owned(),
            to: "TEST-PARSER-012".to_owned(),
            note: Some("boundary cases overlap".to_owned()),
            created: "2026-08-08T00:00:00Z".to_owned(),
        };
        let value = yaml_serde::to_value(&record).unwrap();
        let mut keys: Vec<&str> = value
            .as_mapping()
            .unwrap()
            .iter()
            .filter_map(|(key, _)| key.as_str())
            .collect();
        keys.sort_unstable();
        let mut expected = RELATION_KEYS.to_vec();
        expected.sort_unstable();
        assert_eq!(keys, expected);
    }
}
