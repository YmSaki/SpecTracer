//! Canonical entity and append-only approval records.
//!
//! M2 keeps the on-disk representation YAML as specified.  The project does
//! not yet depend on a YAML parser, so this module accepts the deliberately
//! small scalar/list subset emitted by vtest and preserves unknown fields by
//! ignoring them (forward-compatible read behavior).

use crate::{StoreError, VerifyLayout};
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeSet,
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

/// One content-addressed subject captured by an append-only audit fact.
/// Exactly one of `id` and `locator` identifies the subject.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct AuditSubjectRecord {
    pub id: Option<String>,
    pub locator: Option<String>,
    pub hash: ContentHash,
}

/// A concrete source consulted to support an audit claim.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct AuditBasisRecord {
    pub kind: String,
    #[serde(rename = "ref")]
    pub reference: String,
}

/// An explained audit conclusion. Static audits may attach a rule and a
/// per-rule verdict; semantic audits normally leave both fields absent.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct AuditReasonRecord {
    pub rule: Option<String>,
    pub verdict: Option<String>,
    pub claim: String,
    pub basis: Vec<AuditBasisRecord>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct AuditExclusionRecord {
    pub item: String,
    pub basis: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct AuditorRecord {
    pub kind: String,
    pub id: String,
    pub model: Option<String>,
}

/// Canonical, append-only audit result stored in `.verify/audits/<ULID>.yaml`.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct AuditRecord {
    pub id: String,
    pub kind: String,
    pub bundle_id: Option<String>,
    pub subjects: Vec<AuditSubjectRecord>,
    pub verdict: String,
    pub reasons: Vec<AuditReasonRecord>,
    pub exclusions: Vec<AuditExclusionRecord>,
    pub auditor: AuditorRecord,
    pub confidence: Option<String>,
    pub audited_at: String,
    pub revision: Revision,
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

impl AuditRecord {
    pub fn to_yaml(&self) -> Result<String, StoreError> {
        self.validate(None)?;
        let mut out = format!(
            "id: {}\nkind: {}\nbundle_id: {}\nsubjects:\n",
            yaml_scalar(&self.id),
            yaml_scalar(&self.kind),
            self.bundle_id
                .as_deref()
                .map(yaml_scalar)
                .unwrap_or_else(|| "null".to_owned()),
        );
        for subject in &self.subjects {
            let identifier = match (&subject.id, &subject.locator) {
                (Some(id), None) => format!("id: {}", yaml_scalar(id)),
                (None, Some(locator)) => format!("locator: {}", yaml_scalar(locator)),
                _ => unreachable!("AuditRecord::validate accepted an invalid subject"),
            };
            out.push_str(&format!(
                "  - {identifier}\n    hash: {}\n",
                yaml_scalar(subject.hash.as_str())
            ));
        }
        out.push_str(&format!(
            "verdict: {}\nreasons:\n",
            yaml_scalar(&self.verdict)
        ));
        for reason in &self.reasons {
            if let Some(rule) = &reason.rule {
                out.push_str(&format!("  - rule: {}\n", yaml_scalar(rule)));
                if let Some(verdict) = &reason.verdict {
                    out.push_str(&format!("    verdict: {}\n", yaml_scalar(verdict)));
                }
            } else {
                out.push_str("  - claim: ");
                out.push_str(&yaml_scalar(&reason.claim));
                out.push('\n');
            }
            if reason.rule.is_some() {
                out.push_str(&format!("    claim: {}\n", yaml_scalar(&reason.claim)));
            }
            out.push_str("    basis:\n");
            for basis in &reason.basis {
                out.push_str(&format!(
                    "      - kind: {}\n        ref: {}\n",
                    yaml_scalar(&basis.kind),
                    yaml_scalar(&basis.reference),
                ));
            }
        }
        if self.exclusions.is_empty() {
            out.push_str("exclusions: []\n");
        } else {
            out.push_str("exclusions:\n");
            for exclusion in &self.exclusions {
                out.push_str(&format!(
                    "  - item: {}\n    basis: {}\n",
                    yaml_scalar(&exclusion.item),
                    yaml_scalar(&exclusion.basis),
                ));
            }
        }
        out.push_str(&format!(
            "auditor:\n  kind: {}\n  id: {}\n",
            yaml_scalar(&self.auditor.kind),
            yaml_scalar(&self.auditor.id),
        ));
        if let Some(model) = &self.auditor.model {
            out.push_str(&format!("  model: {}\n", yaml_scalar(model)));
        }
        if let Some(confidence) = &self.confidence {
            out.push_str(&format!("confidence: {}\n", yaml_scalar(confidence)));
        }
        out.push_str(&format!(
            "audited_at: {}\nrevision: {{ commit: {}, dirty: {} }}\n",
            yaml_scalar(&self.audited_at),
            self.revision
                .commit
                .as_deref()
                .map(yaml_scalar)
                .unwrap_or_else(|| "null".to_owned()),
            self.revision.dirty,
        ));
        Ok(out)
    }

    pub fn from_yaml(text: &str, filename_id: &str) -> Result<Self, StoreError> {
        let fields = audit_top_level_fields(text)?;
        let field = |key: &str| {
            fields.get(key).ok_or_else(|| {
                StoreError::InvalidConfig(format!("audit is missing required field {key}"))
            })
        };
        let id = audit_required_scalar(field("id")?, "id")?;
        let kind = audit_required_scalar(field("kind")?, "kind")?;
        let bundle_id = audit_optional_scalar(field("bundle_id")?, "bundle_id")?;
        let subjects = parse_audit_subjects(field("subjects")?)?;
        let verdict = audit_required_scalar(field("verdict")?, "verdict")?;
        let reasons = parse_audit_reasons(field("reasons")?)?;
        let exclusions = parse_audit_exclusions(field("exclusions")?)?;
        let auditor = parse_auditor(field("auditor")?)?;
        let confidence = fields
            .get("confidence")
            .map(|value| audit_optional_scalar(value, "confidence"))
            .transpose()?
            .flatten();
        let audited_at = audit_required_scalar(field("audited_at")?, "audited_at")?;
        let revision = parse_audit_revision(field("revision")?)?;
        let record = Self {
            id,
            kind,
            bundle_id,
            subjects,
            verdict,
            reasons,
            exclusions,
            auditor,
            confidence,
            audited_at,
            revision,
        };
        record.validate(Some(filename_id))?;
        Ok(record)
    }

    fn validate(&self, filename_id: Option<&str>) -> Result<(), StoreError> {
        if !is_valid_ulid(&self.id) {
            return Err(StoreError::InvalidConfig(
                "audit id must be a valid ULID".to_owned(),
            ));
        }
        if let Some(filename_id) = filename_id {
            if self.id != filename_id {
                return Err(StoreError::InvalidConfig(format!(
                    "audit id {} does not match file name {filename_id}",
                    self.id
                )));
            }
        }
        if !matches!(
            self.kind.as_str(),
            "test-semantic" | "vo-coverage" | "impl-consistency" | "static"
        ) {
            return Err(StoreError::InvalidConfig(
                "audit has an invalid kind".to_owned(),
            ));
        }
        if self.kind == "static" && self.bundle_id.is_some() {
            return Err(StoreError::InvalidConfig(
                "static audit bundle_id must be null".to_owned(),
            ));
        }
        if self.subjects.is_empty() {
            return Err(StoreError::InvalidConfig(
                "audit must contain at least one subject".to_owned(),
            ));
        }
        let mut subject_keys = BTreeSet::new();
        for subject in &self.subjects {
            let id = subject
                .id
                .as_deref()
                .filter(|value| !value.trim().is_empty());
            let locator = subject
                .locator
                .as_deref()
                .filter(|value| !value.trim().is_empty());
            if matches!((id, locator), (Some(_), None) | (None, Some(_))) {
                let key = id
                    .map(|value| format!("id:{value}"))
                    .unwrap_or_else(|| format!("locator:{}", locator.unwrap_or_default()));
                if !subject_keys.insert(key) {
                    return Err(StoreError::InvalidConfig(
                        "audit subjects must not contain duplicate identities".to_owned(),
                    ));
                }
                continue;
            }
            return Err(StoreError::InvalidConfig(
                "each audit subject must contain exactly one non-empty id or locator".to_owned(),
            ));
        }
        if self.kind == "static"
            && self
                .subjects
                .iter()
                .filter(|subject| {
                    subject
                        .id
                        .as_deref()
                        .is_some_and(|id| id.starts_with("TEST-"))
                })
                .count()
                != 1
        {
            return Err(StoreError::InvalidConfig(
                "static audit must bind exactly one TEST-* subject".to_owned(),
            ));
        }
        if !matches!(self.verdict.as_str(), "PASS" | "FAIL" | "UNKNOWN") {
            return Err(StoreError::InvalidConfig(
                "audit verdict must be PASS, FAIL, or UNKNOWN".to_owned(),
            ));
        }
        if self.reasons.is_empty() {
            return Err(StoreError::InvalidConfig(
                "audit must contain at least one reason".to_owned(),
            ));
        }
        for reason in &self.reasons {
            if reason.claim.trim().is_empty() || reason.basis.is_empty() {
                return Err(StoreError::InvalidConfig(
                    "each audit reason must contain a non-empty claim and basis".to_owned(),
                ));
            }
            for basis in &reason.basis {
                if !matches!(
                    basis.kind.as_str(),
                    "spec" | "vo" | "req" | "test-code" | "target-code"
                ) || basis.reference.trim().is_empty()
                {
                    return Err(StoreError::InvalidConfig(
                        "each audit basis must contain an allowed kind and non-empty ref"
                            .to_owned(),
                    ));
                }
            }
        }
        if !matches!(
            self.auditor.kind.as_str(),
            "deterministic" | "agent" | "human"
        ) || self.auditor.id.trim().is_empty()
        {
            return Err(StoreError::InvalidConfig(
                "audit auditor must contain a valid kind and non-empty id".to_owned(),
            ));
        }
        if self.kind == "static" && self.auditor.kind != "deterministic" {
            return Err(StoreError::InvalidConfig(
                "static audit auditor kind must be deterministic".to_owned(),
            ));
        }
        if self.auditor.kind == "deterministic"
            && (self.confidence.is_some() || self.auditor.model.is_some())
        {
            return Err(StoreError::InvalidConfig(
                "deterministic audit model and confidence must be absent".to_owned(),
            ));
        }
        if self.kind == "static" {
            let mut derived = "PASS";
            let mut deterministic_rules = BTreeSet::new();
            let mut warning_seen = false;
            for reason in &self.reasons {
                let Some(rule) = reason.rule.as_deref() else {
                    return Err(StoreError::InvalidConfig(
                        "static audit reasons require a known rule and rule verdict".to_owned(),
                    ));
                };
                if !matches!(reason.verdict.as_deref(), Some("PASS" | "FAIL" | "UNKNOWN")) {
                    return Err(StoreError::InvalidConfig(
                        "static audit reasons require a known rule and rule verdict".to_owned(),
                    ));
                }
                if matches!(
                    rule,
                    "DA-001" | "DA-002" | "DA-003" | "DA-004" | "DA-005" | "DA-006"
                ) {
                    if !deterministic_rules.insert(rule) {
                        return Err(StoreError::InvalidConfig(
                            "static audit must contain each DA-001 through DA-006 rule exactly once"
                                .to_owned(),
                        ));
                    }
                } else if rule == "W-DA-101" {
                    if warning_seen {
                        return Err(StoreError::InvalidConfig(
                            "static audit must not duplicate W-DA-101".to_owned(),
                        ));
                    }
                    warning_seen = true;
                } else {
                    return Err(StoreError::InvalidConfig(
                        "static audit reasons require a known rule and rule verdict".to_owned(),
                    ));
                }
                match reason.verdict.as_deref() {
                    Some("FAIL") => derived = "FAIL",
                    Some("UNKNOWN") if derived != "FAIL" => derived = "UNKNOWN",
                    _ => {}
                }
            }
            if deterministic_rules.len() != 6 {
                return Err(StoreError::InvalidConfig(
                    "static audit must contain each DA-001 through DA-006 rule exactly once"
                        .to_owned(),
                ));
            }
            if self.verdict != derived {
                return Err(StoreError::InvalidConfig(
                    "static audit verdict does not match its rule verdicts".to_owned(),
                ));
            }
        }
        if !is_rfc3339_timestamp(&self.audited_at) {
            return Err(StoreError::InvalidConfig(
                "audit audited_at must be an RFC 3339 timestamp".to_owned(),
            ));
        }
        Ok(())
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
            target_fns: target_hashes,
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

pub fn read_audit(path: &Path) -> Result<AuditRecord, StoreError> {
    let text = read_text(path)?;
    let fallback = path
        .file_stem()
        .and_then(|value| value.to_str())
        .unwrap_or_default();
    AuditRecord::from_yaml(&text, fallback)
}

pub fn read_relation(path: &Path) -> Result<RelationRecord, StoreError> {
    let text = read_text(path)?;
    let fallback = path
        .file_stem()
        .and_then(|value| value.to_str())
        .unwrap_or_default();
    RelationRecord::from_yaml(&text, fallback)
}

/// Creates a new canonical Relation record and writes it to `.verify/rel/`.
/// The id is always generated here as `REL-<ULID>`: 詳細設計 v0.1 §3.3
/// requires the writer to emit only that form, even though `is_valid_relation_id`
/// still accepts a bare ULID for version 1 compatibility on read.
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

fn is_rfc3339_timestamp(value: &str) -> bool {
    let Some((date, time_and_zone)) = value.split_once('T') else {
        return false;
    };
    let date_parts = date.split('-').collect::<Vec<_>>();
    if date_parts.len() != 3
        || date_parts[0].len() != 4
        || date_parts[1].len() != 2
        || date_parts[2].len() != 2
        || !date_parts
            .iter()
            .all(|part| part.chars().all(|ch| ch.is_ascii_digit()))
    {
        return false;
    }
    let year = date_parts[0].parse::<u32>().ok();
    let month = date_parts[1].parse::<u32>().ok();
    let day = date_parts[2].parse::<u32>().ok();
    let (Some(year), Some(month), Some(day)) = (year, month, day) else {
        return false;
    };
    let leap = year % 4 == 0 && (year % 100 != 0 || year % 400 == 0);
    let max_day = match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if leap => 29,
        2 => 28,
        _ => return false,
    };
    if day == 0 || day > max_day {
        return false;
    }

    let (time, zone) = if let Some(time) = time_and_zone.strip_suffix('Z') {
        (time, "Z")
    } else {
        let Some(index) = time_and_zone
            .char_indices()
            .skip(1)
            .find_map(|(index, ch)| matches!(ch, '+' | '-').then_some(index))
        else {
            return false;
        };
        (&time_and_zone[..index], &time_and_zone[index..])
    };
    if zone != "Z" {
        let bytes = zone.as_bytes();
        if bytes.len() != 6
            || !matches!(bytes[0], b'+' | b'-')
            || bytes[3] != b':'
            || !bytes[1..3].iter().all(u8::is_ascii_digit)
            || !bytes[4..6].iter().all(u8::is_ascii_digit)
        {
            return false;
        }
        let offset_hour = zone[1..3].parse::<u32>().ok();
        let offset_minute = zone[4..6].parse::<u32>().ok();
        if offset_hour.is_none_or(|hour| hour > 23)
            || offset_minute.is_none_or(|minute| minute > 59)
        {
            return false;
        }
    }

    let time_parts = time.split(':').collect::<Vec<_>>();
    if time_parts.len() != 3 || time_parts[0].len() != 2 || time_parts[1].len() != 2 {
        return false;
    }
    let second = time_parts[2].split_once('.');
    let (second, fraction) = second
        .map(|(second, fraction)| (second, Some(fraction)))
        .unwrap_or((time_parts[2], None));
    if second.len() != 2
        || !time_parts[0].chars().all(|ch| ch.is_ascii_digit())
        || !time_parts[1].chars().all(|ch| ch.is_ascii_digit())
        || !second.chars().all(|ch| ch.is_ascii_digit())
        || fraction.is_some_and(|fraction| {
            fraction.is_empty() || !fraction.chars().all(|ch| ch.is_ascii_digit())
        })
    {
        return false;
    }
    time_parts[0].parse::<u32>().is_ok_and(|hour| hour <= 23)
        && time_parts[1]
            .parse::<u32>()
            .is_ok_and(|minute| minute <= 59)
        && second.parse::<u32>().is_ok_and(|second| second <= 60)
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

#[derive(Clone, Debug)]
struct AuditYamlField {
    value: String,
    children: Vec<String>,
}

/// Parse the deliberately small YAML subset emitted by `AuditRecord::to_yaml`.
/// Unlike the older entity readers, audit facts are strict: malformed nesting,
/// duplicate keys, and unrecognised top-level fields are rejected rather than
/// being silently accepted as evidence.
fn audit_top_level_fields(
    text: &str,
) -> Result<std::collections::BTreeMap<String, AuditYamlField>, StoreError> {
    const KEYS: &[&str] = &[
        "id",
        "kind",
        "bundle_id",
        "subjects",
        "verdict",
        "reasons",
        "exclusions",
        "auditor",
        "confidence",
        "audited_at",
        "revision",
    ];
    let mut fields: std::collections::BTreeMap<String, AuditYamlField> =
        std::collections::BTreeMap::new();
    let mut current: Option<String> = None;
    for (line_number, raw) in text.lines().enumerate() {
        if raw.trim().is_empty() {
            continue;
        }
        if raw.starts_with([' ', '\t']) {
            let key = current.as_ref().ok_or_else(|| {
                StoreError::InvalidConfig(format!(
                    "audit YAML has an orphan nested field at line {}",
                    line_number + 1
                ))
            })?;
            if raw.starts_with('\t') {
                return Err(StoreError::InvalidConfig(format!(
                    "audit YAML must not use tab indentation at line {}",
                    line_number + 1
                )));
            }
            fields
                .get_mut(key)
                .expect("current audit YAML field must exist")
                .children
                .push(raw.to_owned());
            continue;
        }
        let (key, value) = raw.split_once(':').ok_or_else(|| {
            StoreError::InvalidConfig(format!("invalid audit YAML at line {}", line_number + 1))
        })?;
        let key = key.trim();
        if !KEYS.contains(&key) || fields.contains_key(key) {
            return Err(StoreError::InvalidConfig(format!(
                "invalid or duplicate audit YAML field {key}"
            )));
        }
        fields.insert(
            key.to_owned(),
            AuditYamlField {
                value: value.trim().to_owned(),
                children: Vec::new(),
            },
        );
        current = Some(key.to_owned());
    }
    Ok(fields)
}

fn audit_scalar(value: &str, label: &str) -> Result<Option<String>, StoreError> {
    let value = value.trim();
    if value == "null" {
        return Ok(None);
    }
    if value.is_empty() {
        return Ok(None);
    }
    if value.starts_with('\'') != value.ends_with('\'')
        || value.starts_with('"') != value.ends_with('"')
    {
        return Err(StoreError::InvalidConfig(format!(
            "audit {label} has an unterminated quoted scalar"
        )));
    }
    Ok(Some(unquote(value)))
}

fn audit_required_scalar(field: &AuditYamlField, label: &str) -> Result<String, StoreError> {
    if !field.children.is_empty() {
        return Err(StoreError::InvalidConfig(format!(
            "audit {label} must be a scalar"
        )));
    }
    audit_scalar(&field.value, label)?
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| {
            StoreError::InvalidConfig(format!("audit is missing required field {label}"))
        })
}

fn audit_optional_scalar(
    field: &AuditYamlField,
    label: &str,
) -> Result<Option<String>, StoreError> {
    if !field.children.is_empty() {
        return Err(StoreError::InvalidConfig(format!(
            "audit {label} must be a scalar"
        )));
    }
    audit_scalar(&field.value, label)
}

fn audit_list_entries(
    children: &[String],
    label: &str,
) -> Result<Vec<std::collections::BTreeMap<String, String>>, StoreError> {
    let mut entries = Vec::new();
    let mut current: Option<std::collections::BTreeMap<String, String>> = None;
    for raw in children {
        let (content, first) = if let Some(content) = raw.strip_prefix("  - ") {
            (content, true)
        } else if let Some(content) = raw.strip_prefix("    ") {
            (content, false)
        } else {
            return Err(StoreError::InvalidConfig(format!(
                "audit {label} has invalid list indentation"
            )));
        };
        if first {
            if let Some(entry) = current.take() {
                entries.push(entry);
            }
            current = Some(std::collections::BTreeMap::new());
        }
        let entry = current.as_mut().ok_or_else(|| {
            StoreError::InvalidConfig(format!(
                "audit {label} has a list continuation without an item"
            ))
        })?;
        let (key, value) = content.split_once(':').ok_or_else(|| {
            StoreError::InvalidConfig(format!("audit {label} entry is not a mapping"))
        })?;
        let value = audit_scalar(value, label)?.ok_or_else(|| {
            StoreError::InvalidConfig(format!("audit {label} entry has an empty {key}"))
        })?;
        if entry.insert(key.trim().to_owned(), value).is_some() {
            return Err(StoreError::InvalidConfig(format!(
                "audit {label} entry has duplicate field {}",
                key.trim()
            )));
        }
    }
    if let Some(entry) = current {
        entries.push(entry);
    }
    Ok(entries)
}

fn parse_audit_subjects(field: &AuditYamlField) -> Result<Vec<AuditSubjectRecord>, StoreError> {
    if field.value == "[]" && field.children.is_empty() {
        return Ok(Vec::new());
    }
    if !field.value.is_empty() {
        return Err(StoreError::InvalidConfig(
            "audit subjects must be a YAML list".to_owned(),
        ));
    }
    audit_list_entries(&field.children, "subjects")?
        .into_iter()
        .map(|mut entry| {
            let hash = entry
                .remove("hash")
                .ok_or_else(|| {
                    StoreError::InvalidConfig("audit subject is missing hash".to_owned())
                })?
                .parse()
                .map_err(|error: String| StoreError::InvalidConfig(error))?;
            let id = entry.remove("id");
            let locator = entry.remove("locator");
            if !entry.is_empty() {
                return Err(StoreError::InvalidConfig(
                    "audit subject has an unrecognised field".to_owned(),
                ));
            }
            Ok(AuditSubjectRecord { id, locator, hash })
        })
        .collect()
}

fn parse_audit_reasons(field: &AuditYamlField) -> Result<Vec<AuditReasonRecord>, StoreError> {
    if !field.value.is_empty() {
        return Err(StoreError::InvalidConfig(
            "audit reasons must be a YAML list".to_owned(),
        ));
    }
    let mut reasons = Vec::new();
    let mut index = 0;
    while index < field.children.len() {
        let first = field.children[index].strip_prefix("  - ").ok_or_else(|| {
            StoreError::InvalidConfig("audit reasons has invalid list indentation".to_owned())
        })?;
        let mut values = std::collections::BTreeMap::new();
        let (key, value) = first.split_once(':').ok_or_else(|| {
            StoreError::InvalidConfig("audit reason entry is not a mapping".to_owned())
        })?;
        values.insert(
            key.trim().to_owned(),
            audit_scalar(value, "reason")?.ok_or_else(|| {
                StoreError::InvalidConfig("audit reason has an empty field".to_owned())
            })?,
        );
        index += 1;
        let mut saw_basis = false;
        while index < field.children.len() && !field.children[index].starts_with("  - ") {
            let raw = &field.children[index];
            if raw == "    basis:" {
                saw_basis = true;
                index += 1;
                let mut basis = Vec::new();
                while index < field.children.len() && field.children[index].starts_with("      - ")
                {
                    let kind = field.children[index]
                        .strip_prefix("      - kind:")
                        .map(|value| audit_scalar(value, "basis kind"))
                        .transpose()?
                        .flatten()
                        .filter(|value| !value.trim().is_empty())
                        .ok_or_else(|| {
                            StoreError::InvalidConfig(
                                "audit basis entries must contain kind and ref".to_owned(),
                            )
                        })?;
                    let reference = field
                        .children
                        .get(index + 1)
                        .and_then(|value| value.strip_prefix("        ref:"))
                        .map(|value| audit_scalar(value, "basis ref"))
                        .transpose()?
                        .flatten()
                        .filter(|value| !value.trim().is_empty())
                        .ok_or_else(|| {
                            StoreError::InvalidConfig(
                                "audit basis entries must contain kind and ref".to_owned(),
                            )
                        })?;
                    basis.push(AuditBasisRecord { kind, reference });
                    index += 2;
                }
                let claim = values.remove("claim").unwrap_or_default();
                let rule = values.remove("rule");
                let verdict = values.remove("verdict");
                if !values.is_empty() {
                    return Err(StoreError::InvalidConfig(
                        "audit reason has an unrecognised field".to_owned(),
                    ));
                }
                reasons.push(AuditReasonRecord {
                    rule,
                    verdict,
                    claim,
                    basis,
                });
                break;
            }
            let value = raw.strip_prefix("    ").ok_or_else(|| {
                StoreError::InvalidConfig("audit reason has invalid indentation".to_owned())
            })?;
            let (key, value) = value.split_once(':').ok_or_else(|| {
                StoreError::InvalidConfig("audit reason entry is not a mapping".to_owned())
            })?;
            let value = audit_scalar(value, "reason")?.ok_or_else(|| {
                StoreError::InvalidConfig("audit reason has an empty field".to_owned())
            })?;
            if values.insert(key.trim().to_owned(), value).is_some() {
                return Err(StoreError::InvalidConfig(
                    "audit reason has duplicate fields".to_owned(),
                ));
            }
            index += 1;
        }
        if !saw_basis {
            return Err(StoreError::InvalidConfig(
                "each audit reason must contain a basis list".to_owned(),
            ));
        }
    }
    Ok(reasons)
}

fn parse_audit_exclusions(field: &AuditYamlField) -> Result<Vec<AuditExclusionRecord>, StoreError> {
    if field.value == "[]" && field.children.is_empty() {
        return Ok(Vec::new());
    }
    if !field.value.is_empty() {
        return Err(StoreError::InvalidConfig(
            "audit exclusions must be a YAML list".to_owned(),
        ));
    }
    audit_list_entries(&field.children, "exclusions")?
        .into_iter()
        .map(|mut entry| {
            let item = entry.remove("item").ok_or_else(|| {
                StoreError::InvalidConfig("audit exclusion is missing item".to_owned())
            })?;
            let basis = entry.remove("basis").ok_or_else(|| {
                StoreError::InvalidConfig("audit exclusion is missing basis".to_owned())
            })?;
            if !entry.is_empty() {
                return Err(StoreError::InvalidConfig(
                    "audit exclusion has an unrecognised field".to_owned(),
                ));
            }
            Ok(AuditExclusionRecord { item, basis })
        })
        .collect()
}

fn parse_auditor(field: &AuditYamlField) -> Result<AuditorRecord, StoreError> {
    if !field.value.is_empty() {
        return Err(StoreError::InvalidConfig(
            "audit auditor must be a YAML mapping".to_owned(),
        ));
    }
    let mut kind = None;
    let mut id = None;
    let mut model = None;
    let mut seen = std::collections::BTreeSet::new();
    for raw in &field.children {
        let value = raw.strip_prefix("  ").ok_or_else(|| {
            StoreError::InvalidConfig("audit auditor has invalid indentation".to_owned())
        })?;
        if value.starts_with(' ') {
            return Err(StoreError::InvalidConfig(
                "audit auditor has invalid indentation".to_owned(),
            ));
        }
        let (key, value) = value.split_once(':').ok_or_else(|| {
            StoreError::InvalidConfig("audit auditor must be a mapping".to_owned())
        })?;
        let key = key.trim();
        if !seen.insert(key.to_owned()) {
            return Err(StoreError::InvalidConfig(
                "audit auditor has duplicate fields".to_owned(),
            ));
        }
        match key {
            "kind" => {
                kind = audit_scalar(value, "auditor.kind")?.filter(|value| !value.trim().is_empty())
            }
            "id" => {
                id = audit_scalar(value, "auditor.id")?.filter(|value| !value.trim().is_empty())
            }
            "model" => model = audit_scalar(value, "auditor.model")?,
            _ => {
                return Err(StoreError::InvalidConfig(
                    "audit auditor has an unrecognised field".to_owned(),
                ))
            }
        }
    }
    Ok(AuditorRecord {
        kind: kind.unwrap_or_default(),
        id: id.unwrap_or_default(),
        model,
    })
}

fn parse_audit_revision(field: &AuditYamlField) -> Result<Revision, StoreError> {
    if !field.children.is_empty() {
        return Err(StoreError::InvalidConfig(
            "audit revision must be an inline mapping".to_owned(),
        ));
    }
    let value = field
        .value
        .strip_prefix('{')
        .and_then(|value| value.strip_suffix('}'))
        .ok_or_else(|| StoreError::InvalidConfig("audit revision must be a mapping".to_owned()))?;
    let mut commit = None;
    let mut dirty = None;
    for pair in value.split(',') {
        let (key, value) = pair.split_once(':').ok_or_else(|| {
            StoreError::InvalidConfig("audit revision must contain commit and dirty".to_owned())
        })?;
        match key.trim() {
            "commit" if commit.is_none() => commit = audit_scalar(value, "revision.commit")?,
            "dirty" if dirty.is_none() => {
                dirty = Some(match value.trim() {
                    "true" => true,
                    "false" => false,
                    _ => {
                        return Err(StoreError::InvalidConfig(
                            "audit revision.dirty must be true or false".to_owned(),
                        ))
                    }
                })
            }
            _ => {
                return Err(StoreError::InvalidConfig(
                    "audit revision has an invalid or duplicate field".to_owned(),
                ))
            }
        }
    }
    Ok(Revision {
        commit,
        dirty: dirty.ok_or_else(|| {
            StoreError::InvalidConfig("audit revision is missing dirty".to_owned())
        })?,
    })
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
    fn audit_round_trip_binds_subjects_and_reads_from_a_ulid_file() {
        let id = new_record_id();
        let record = AuditRecord {
            id: id.clone(),
            kind: "static".to_owned(),
            bundle_id: None,
            subjects: vec![AuditSubjectRecord {
                id: Some("TEST-PARSER-044".to_owned()),
                locator: None,
                hash: ContentHash::from_text("test body\n"),
            }],
            verdict: "FAIL".to_owned(),
            reasons: ["DA-001", "DA-002", "DA-003", "DA-004", "DA-005", "DA-006"]
                .into_iter()
                .map(|rule| AuditReasonRecord {
                    rule: Some(rule.to_owned()),
                    verdict: Some(if rule == "DA-001" { "FAIL" } else { "PASS" }.to_owned()),
                    claim: if rule == "DA-001" {
                        "assertion is constant".to_owned()
                    } else {
                        format!("{rule} passed")
                    },
                    basis: vec![AuditBasisRecord {
                        kind: "test-code".to_owned(),
                        reference: "tests/parser_test.rs:12".to_owned(),
                    }],
                })
                .collect(),
            exclusions: vec![AuditExclusionRecord {
                item: "integration-only path".to_owned(),
                basis: "not selected by this test".to_owned(),
            }],
            auditor: AuditorRecord {
                kind: "deterministic".to_owned(),
                id: "vtest-da".to_owned(),
                model: None,
            },
            confidence: None,
            audited_at: "2026-08-08T00:00:00Z".to_owned(),
            revision: Revision {
                commit: Some("abc123".to_owned()),
                dirty: false,
            },
        };
        let yaml = record.to_yaml().unwrap();
        assert_eq!(AuditRecord::from_yaml(&yaml, &id).unwrap(), record);

        let mut partial_rules = record.clone();
        partial_rules.reasons.truncate(1);
        assert!(partial_rules.to_yaml().is_err());

        let mut duplicate_rule = record.clone();
        duplicate_rule.reasons[1].rule = Some("DA-001".to_owned());
        assert!(duplicate_rule.to_yaml().is_err());

        let no_test_subject = AuditRecord {
            subjects: vec![AuditSubjectRecord {
                id: Some("VO-PARSER-044".to_owned()),
                locator: None,
                hash: ContentHash::from_text("vo body\n"),
            }],
            ..record.clone()
        };
        assert!(no_test_subject.to_yaml().is_err());
        assert!(AuditRecord::from_yaml(
            &yaml.replacen("id: 'TEST-PARSER-044'", "id: 'VO-PARSER-044'", 1),
            &id,
        )
        .is_err());

        let multiple_test_subjects = AuditRecord {
            subjects: vec![
                record.subjects[0].clone(),
                AuditSubjectRecord {
                    id: Some("TEST-PARSER-045".to_owned()),
                    locator: None,
                    hash: ContentHash::from_text("another test body\n"),
                },
            ],
            ..record.clone()
        };
        assert!(multiple_test_subjects.to_yaml().is_err());
        let two_subject_yaml = yaml.replacen(
            "verdict:",
            &format!(
                "  - id: 'TEST-PARSER-045'\n    hash: {}\nverdict:",
                yaml_scalar(multiple_test_subjects.subjects[1].hash.as_str())
            ),
            1,
        );
        assert!(AuditRecord::from_yaml(&two_subject_yaml, &id).is_err());

        let root = temporary_directory("audit-read");
        let path = root.join(format!("{id}.yaml"));
        fs::write(&path, &yaml).unwrap();
        assert_eq!(read_audit(&path).unwrap(), record);
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn audit_rejects_malformed_or_untraceable_records() {
        let id = new_record_id();
        let record = AuditRecord {
            id: id.clone(),
            kind: "test-semantic".to_owned(),
            bundle_id: Some(new_record_id()),
            subjects: vec![AuditSubjectRecord {
                id: None,
                locator: Some("src/parser.rs::Parser::parse".to_owned()),
                hash: ContentHash::from_text("target body\n"),
            }],
            verdict: "PASS".to_owned(),
            reasons: vec![AuditReasonRecord {
                rule: None,
                verdict: None,
                claim: "the declared behavior is asserted".to_owned(),
                basis: vec![AuditBasisRecord {
                    kind: "test-code".to_owned(),
                    reference: "tests/parser_test.rs::rejects_invalid_utf8".to_owned(),
                }],
            }],
            exclusions: Vec::new(),
            auditor: AuditorRecord {
                kind: "agent".to_owned(),
                id: "auditor-agent-01".to_owned(),
                model: Some("model-x".to_owned()),
            },
            confidence: Some("high".to_owned()),
            audited_at: "2026-08-08T00:00:00Z".to_owned(),
            revision: Revision {
                commit: None,
                dirty: true,
            },
        };
        let yaml = record.to_yaml().unwrap();
        for malformed in [
            yaml.replacen("kind: 'test-semantic'", "kind: 'unknown'", 1),
            yaml.replacen("    hash:", "    id: 'TEST-X'\n    hash:", 1),
            yaml.replacen("verdict: 'PASS'", "verdict: 'MISSING'", 1),
            yaml.replacen(
                "ref: 'tests/parser_test.rs::rejects_invalid_utf8'",
                "ref: ''",
                1,
            ),
            yaml.replacen("kind: 'test-code'", "kind: 'source-location'", 1),
            yaml.replacen("audited_at: '2026-08-08T00:00:00Z'", "audited_at: ''", 1),
            yaml.replacen(
                "audited_at: '2026-08-08T00:00:00Z'",
                "audited_at: 'not-a-time'",
                1,
            ),
        ] {
            assert!(AuditRecord::from_yaml(&malformed, &id).is_err());
        }

        let static_with_bundle = AuditRecord {
            kind: "static".to_owned(),
            bundle_id: Some(new_record_id()),
            ..record
        };
        assert!(static_with_bundle.to_yaml().is_err());
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
        assert_eq!(read_relation(&path).unwrap(), record);
    }
}
