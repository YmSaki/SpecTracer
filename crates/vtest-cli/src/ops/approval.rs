//! `vtest approval create|withdraw|show`: the sole canonical entry point for
//! Approval records (BD-304/305/306), per 本冊 §3.5 and DS-1050〜DS-1062 /
//! DS-1461〜DS-1490 (§18.3.7).
//!
//! `--subject-type judgment` (DS-1052) is explicitly rejected rather than
//! silently mishandled: it requires reading a `.verify/decisions/` judgment
//! record, and no judgment-record domain exists anywhere in this codebase
//! (see `vtest-store::approval`'s module doc comment for the full
//! disclosure). This is a real, disclosed scope boundary, not upstream
//! silence — DS-1052 says exactly what a judgment-typed record must contain,
//! this codebase just does not yet have the domain that statement depends on.

use vtest_store::{
    approval::{
        build_document_node_index, document_dependencies, effective_approval_state,
        read_all_approvals, read_all_vos, vo_dependencies, EffectiveApprovalState,
    },
    new_record_id,
    records::{ApprovalBasis, ApprovalRecord, Approver, DependencyRecord},
    write_new_record, StoreError, VerifyLayout,
};

#[derive(Debug, thiserror::Error)]
pub enum ApprovalOpError {
    /// DS-1052 requires a judgment-record domain this codebase does not
    /// have (see module doc comment).
    #[error(
        "--subject-type judgment is not implemented: it requires reading a .verify/decisions/ \
         judgment record, and no judgment-record domain exists in this codebase yet"
    )]
    JudgmentSubjectTypeUnsupported,
    /// DS-1058: the subject (or, for `withdraw`, the superseded record)
    /// cannot be resolved completely/currently.
    #[error("E-APPROVAL-001: {0}")]
    UnresolvedSubject(String),
    /// DS-1059: an out-of-domain value, a subject_type/subject_id mismatch,
    /// or an invalid `supersedes` reference.
    #[error("E-APPROVAL-002: {0}")]
    InvalidRequest(String),
    #[error("store error: {0}")]
    Store(#[from] StoreError),
}

pub struct CreateArgs {
    pub subject_type: String,
    pub subject_id: String,
    pub approved_state: String,
    pub approver_kind: String,
    pub approver_id: String,
    pub approver_model: Option<String>,
    pub basis: Vec<String>,
    pub supersedes: Vec<String>,
}

/// Resolves `subject_type`/`subject_id` to their current `(subject_hash,
/// dependencies)` (DS-1050/1051, DS-1480/1487). Returns
/// `UnresolvedSubject` (E-APPROVAL-001) when the subject does not currently
/// exist, and `JudgmentSubjectTypeUnsupported` for `subject_type ==
/// "judgment"`.
fn resolve_subject(
    layout: &VerifyLayout,
    subject_type: &str,
    subject_id: &str,
) -> Result<(vtest_model::ContentHash, Vec<DependencyRecord>), ApprovalOpError> {
    match subject_type {
        "vo" => {
            let vos = read_all_vos(layout)?;
            let Some(record) = vos.get(subject_id) else {
                return Err(ApprovalOpError::UnresolvedSubject(format!(
                    "no VO with id '{subject_id}' exists"
                )));
            };
            let hash = vtest_model::vo_subject_hash(record);
            let doc_index = build_document_node_index(layout)?;
            let dependencies =
                vo_dependencies(&vos, &doc_index, &vtest_model::VoId::new(subject_id));
            Ok((hash, dependencies))
        }
        "document" => {
            let doc_index = build_document_node_index(layout)?;
            let Some((hash, _)) = doc_index.get(subject_id) else {
                return Err(ApprovalOpError::UnresolvedSubject(format!(
                    "no document node with id '{subject_id}' exists"
                )));
            };
            let dependencies = document_dependencies(&doc_index, subject_id);
            Ok((hash.clone(), dependencies))
        }
        "judgment" => Err(ApprovalOpError::JudgmentSubjectTypeUnsupported),
        other => Err(ApprovalOpError::InvalidRequest(format!(
            "subject_type must be one of vo, document, judgment; got {other}"
        ))),
    }
}

/// DS-1059/1474: each `--supersedes <id>` must name an existing approval
/// record for the *same* subject, and must not self-reference (impossible
/// here since `id` is generated after this check, but kept symmetric with
/// `withdraw`'s equivalent check).
fn validate_supersedes(
    layout: &VerifyLayout,
    subject_type: &str,
    subject_id: &str,
    supersedes: &[String],
) -> Result<(), ApprovalOpError> {
    if supersedes.is_empty() {
        return Ok(());
    }
    let existing = read_all_approvals(layout)?;
    for target_id in supersedes {
        let Some(target) = existing.iter().find(|record| &record.id == target_id) else {
            return Err(ApprovalOpError::InvalidRequest(format!(
                "supersedes target '{target_id}' does not exist"
            )));
        };
        if target.subject_type != subject_type || target.subject != subject_id {
            return Err(ApprovalOpError::InvalidRequest(format!(
                "supersedes target '{target_id}' does not target the same subject"
            )));
        }
    }
    Ok(())
}

pub fn create(layout: &VerifyLayout, args: CreateArgs) -> Result<ApprovalRecord, ApprovalOpError> {
    let (subject_hash, dependencies) =
        resolve_subject(layout, &args.subject_type, &args.subject_id)?;
    validate_supersedes(
        layout,
        &args.subject_type,
        &args.subject_id,
        &args.supersedes,
    )?;
    let record = ApprovalRecord {
        id: new_record_id(),
        subject_type: args.subject_type,
        subject: args.subject_id,
        subject_hash,
        dependencies,
        judgment_ref: None,
        approver: Approver {
            kind: args.approver_kind,
            id: args.approver_id,
            model: args.approver_model,
        },
        approved_state: args.approved_state,
        basis: args
            .basis
            .into_iter()
            .map(|reference| ApprovalBasis {
                kind: "ref".to_owned(),
                reference,
            })
            .collect(),
        supersedes: args.supersedes,
        approved_at: vtest_store::records::now_rfc3339(),
    };
    let yaml = record
        .to_yaml()
        .map_err(|error| ApprovalOpError::InvalidRequest(error.to_string()))?;
    let path = layout.approvals_dir().join(format!("{}.yaml", record.id));
    write_new_record(&path, &yaml)?;
    Ok(record)
}

pub struct WithdrawArgs {
    pub approval_id: String,
    pub approver_kind: String,
    pub approver_id: String,
    pub approver_model: Option<String>,
    pub basis: Vec<String>,
}

/// BD-307/DS-1056: `withdraw` is `create` with `state: withdrawn` and
/// `supersedes: [approval-id]`, copying the target record's subject fields
/// verbatim (not re-resolved against the current subject state — the
/// withdrawal targets the specific prior grant, whatever its binding was).
pub fn withdraw(
    layout: &VerifyLayout,
    args: WithdrawArgs,
) -> Result<ApprovalRecord, ApprovalOpError> {
    let existing = read_all_approvals(layout)?;
    let Some(target) = existing.iter().find(|record| record.id == args.approval_id) else {
        return Err(ApprovalOpError::InvalidRequest(format!(
            "supersedes target '{}' does not exist",
            args.approval_id
        )));
    };
    let record = ApprovalRecord {
        id: new_record_id(),
        subject_type: target.subject_type.clone(),
        subject: target.subject.clone(),
        subject_hash: target.subject_hash.clone(),
        dependencies: target.dependencies.clone(),
        judgment_ref: target.judgment_ref.clone(),
        approver: Approver {
            kind: args.approver_kind,
            id: args.approver_id,
            model: args.approver_model,
        },
        approved_state: "withdrawn".to_owned(),
        basis: args
            .basis
            .into_iter()
            .map(|reference| ApprovalBasis {
                kind: "ref".to_owned(),
                reference,
            })
            .collect(),
        supersedes: vec![args.approval_id],
        approved_at: vtest_store::records::now_rfc3339(),
    };
    let yaml = record
        .to_yaml()
        .map_err(|error| ApprovalOpError::InvalidRequest(error.to_string()))?;
    let path = layout.approvals_dir().join(format!("{}.yaml", record.id));
    write_new_record(&path, &yaml)?;
    Ok(record)
}

pub struct ShowResult {
    pub records: Vec<ApprovalRecord>,
    pub effective_state: EffectiveApprovalState,
}

/// DS-1057: returns the subject's full approval-record history plus its
/// current effective state (`draft` / `approved`). If the subject cannot
/// currently be resolved (DS-1058's condition, at read time rather than
/// write time), no record can validly bind to it (DS-1467's subject_hash
/// match can never hold), so the effective state is `draft` — this follows
/// from DS-1467 rather than inventing new show-specific behavior.
pub fn show(
    layout: &VerifyLayout,
    subject_type: &str,
    subject_id: &str,
) -> Result<ShowResult, ApprovalOpError> {
    let all = read_all_approvals(layout)?;
    let matching: Vec<ApprovalRecord> = all
        .into_iter()
        .filter(|record| record.subject_type == subject_type && record.subject == subject_id)
        .collect();

    let effective_state = match resolve_subject(layout, subject_type, subject_id) {
        Ok((hash, dependencies)) => {
            effective_approval_state(&matching, subject_type, subject_id, &hash, &dependencies)
        }
        Err(ApprovalOpError::UnresolvedSubject(_)) => EffectiveApprovalState::Draft,
        Err(other) => return Err(other),
    };

    Ok(ShowResult {
        records: matching,
        effective_state,
    })
}
