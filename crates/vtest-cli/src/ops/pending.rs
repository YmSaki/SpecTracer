//! The `pending` section of `verify` / `report` (本冊 §12.4 / 別紙C §11.7,
//! DS-884〜DS-895, DS-1147, DS-1214/DS-1215, DS-1439〜DS-1443,
//! DES-468〜DES-473).
//!
//! It gathers, across the whole result, the facts that are not settled and
//! the matters that need an outside judgment. Each item carries `subject`,
//! `kind`, `check`, `judgment_kind`, `basis` and `bundle_ref`. It is a
//! projection of the verification result: building it changes no check
//! value (DS-886) and the same inputs always yield the same items in the
//! same order (DS-789/DS-1440).
//!
//! Kinds produced here:
//!
//! - `unknown` from a check whose state is `UNKNOWN` (DS-894), and the
//!   per-Test `case-coverage` item (DS-888〜DS-893 / DS-1441). The item is
//!   dropped only when the `(Test, case-coverage)` effective judgment
//!   (DS-724〜DS-729) is `accepted`; an undetermined, `rejected`,
//!   `deferred` or conflicting one keeps it, with the judgment records it
//!   looked at in `basis` (DS-891/DS-892). Judgment records never touch the
//!   check-derived `unknown` items (DS-1443).
//! - `unregistered` from a discovered Test construct with no management
//!   declaration (`ManagedTestLink::Missing`, DS-560).
//! - `unresolved` from a `MISMATCH` whose label is `MISSING` on a Test's
//!   `target_binding` (a declared target that does not resolve).
//! - `pending_approval` from a requested gate's approval role that is not
//!   satisfied.
//!
//! Not produced: `undecided` (VO 未確定). The canon gives no machine
//! criterion for an undecided VO on this branch, and inventing one would
//! fabricate a pending fact.

use std::collections::BTreeMap;

use vtest_model::{DiagnosticLabel, ManagedTestLink, VerificationState, VoRecord};
use vtest_scan::ScanResult;
use vtest_store::{decisions::DecisionRecord, VerifyLayout};
use vtest_verify::{check_name, CheckOutcome, NodeKind, TreeNode, VerifyOutcome};

use super::verify::{state_name, GateEvaluation};

/// DES-470: the related check, its current state and labels.
#[derive(Clone, Debug, Eq, PartialEq, serde::Serialize)]
pub struct PendingCheck {
    pub name: &'static str,
    pub state: &'static str,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub labels: Vec<&'static str>,
}

/// One pending item (DES-468〜DES-473).
#[derive(Clone, Debug, Eq, PartialEq, serde::Serialize)]
pub struct PendingItem {
    pub subject: String,
    pub kind: &'static str,
    /// `null` for an item that comes from no check (DS-885).
    pub check: Option<PendingCheck>,
    /// `null` when no outside judgment is needed (DS-887).
    pub judgment_kind: Option<&'static str>,
    pub basis: Vec<String>,
    /// DES-473: optional reference to a judgment bundle. No bundle exists
    /// until one is requested, so this is always `null` here.
    pub bundle_ref: Option<String>,
}

pub fn label_name(label: DiagnosticLabel) -> &'static str {
    match label {
        DiagnosticLabel::Missing => "MISSING",
        DiagnosticLabel::NotChecked => "NOT_CHECKED",
        DiagnosticLabel::NotExecuted => "NOT_EXECUTED",
        DiagnosticLabel::Stale => "STALE",
    }
}

fn pending_check(outcome: &CheckOutcome) -> PendingCheck {
    PendingCheck {
        name: check_name(outcome.check),
        state: state_name(outcome.state),
        labels: outcome.labels.iter().copied().map(label_name).collect(),
    }
}

fn from_check(subject: &str, outcome: &CheckOutcome) -> Option<PendingItem> {
    let kind = match outcome.state {
        VerificationState::Unknown => "unknown",
        VerificationState::Mismatch
            if outcome.check == vtest_model::VerificationCheck::TargetBinding
                && outcome.labels.contains(&DiagnosticLabel::Missing) =>
        {
            "unresolved"
        }
        _ => return None,
    };
    Some(PendingItem {
        subject: subject.to_owned(),
        kind,
        check: Some(pending_check(outcome)),
        judgment_kind: None,
        basis: outcome.basis.clone(),
        bundle_ref: None,
    })
}

fn walk(node: &TreeNode, out: &mut Vec<PendingItem>) {
    if node.kind == NodeKind::Test {
        out.extend(
            node.checks
                .iter()
                .filter_map(|outcome| from_check(&node.id, outcome)),
        );
    }
    for child in &node.children {
        walk(child, out);
    }
}

fn test_ids_in(nodes: &[TreeNode], out: &mut std::collections::BTreeSet<String>) {
    for node in nodes {
        if node.kind == NodeKind::Test {
            out.insert(node.id.clone());
        }
        test_ids_in(&node.children, out);
    }
}

/// The `(Test, case-coverage)` effective judgment (本冊 §8.5).
#[derive(Debug, Eq, PartialEq)]
enum CaseCoverage {
    /// DS-729: every record of the effective set says `accepted`.
    Accepted,
    /// DS-728 (empty effective set), DS-725 (conflicting values), or a
    /// single `rejected` / `deferred` value. `basis` holds the ids of the
    /// records looked at (DS-892/DS-1442).
    NotAccepted { basis: Vec<String> },
}

/// Derives the effective case-coverage judgment of `test_id`.
///
/// - V (DS-726): the records for this subject and kind that are valid
///   (DS-724): the subject hash and the full dependency closure equal the
///   ones computed now, the same binding `audit bundle` records.
/// - E (DS-727): V minus the records another record of V supersedes.
/// - Empty E is undetermined (DS-728); one shared value is that value
///   (DS-729); differing values select none (DS-725).
fn case_coverage_judgment(
    layout: &VerifyLayout,
    scan: &ScanResult,
    test_id: &str,
    decisions: &[DecisionRecord],
) -> CaseCoverage {
    let records: Vec<&DecisionRecord> = decisions
        .iter()
        .filter(|record| record.subject == test_id && record.judgment_kind == "case-coverage")
        .collect();
    if records.is_empty() {
        return CaseCoverage::NotAccepted { basis: Vec::new() };
    }
    let current = super::judgment::current_binding(
        layout,
        scan,
        &super::judgment::Subject::Test(test_id.to_owned()),
    )
    .ok();
    let valid: Vec<&DecisionRecord> = records
        .iter()
        .copied()
        .filter(|record| {
            current.as_ref().is_some_and(|binding| {
                binding.subject_hash == record.subject_hash
                    && binding.dependencies == record.dependencies
            })
        })
        .collect();
    let superseded = |record: &DecisionRecord| {
        valid
            .iter()
            .any(|other| other.id != record.id && other.supersedes.contains(&record.id))
    };
    let effective: Vec<&DecisionRecord> = valid
        .iter()
        .copied()
        .filter(|record| !superseded(record))
        .collect();
    if !effective.is_empty() && effective.iter().all(|record| record.decision == "accepted") {
        return CaseCoverage::Accepted;
    }
    // DS-892/DS-1442: `basis` carries the ids of the judgment records
    // looked at, valid or not, in id order.
    let basis = records.iter().map(|record| record.id.clone()).collect();
    CaseCoverage::NotAccepted { basis }
}

/// Builds the `pending` section for one verification result.
pub fn pending_items(
    layout: &VerifyLayout,
    scan: &ScanResult,
    vos: &BTreeMap<String, VoRecord>,
    decisions: &[DecisionRecord],
    outcome: &VerifyOutcome,
    gate: Option<&GateEvaluation>,
) -> Vec<PendingItem> {
    let mut items = Vec::new();
    for check in &outcome.structural {
        if check.state == VerificationState::Unknown {
            items.extend(from_check(check_name(check.check), check));
        }
    }
    for node in &outcome.tree {
        walk(node, &mut items);
    }

    // DS-1441: one case-coverage item per managed Test in scope with
    // covers >= 1 and (cases >= 1 or a resolved covered VO with
    // dimensions >= 1).
    let mut in_scope = std::collections::BTreeSet::new();
    test_ids_in(&outcome.tree, &mut in_scope);
    for test in scan
        .tests
        .iter()
        .filter(|test| in_scope.contains(test.id.as_str()))
    {
        let has_dimensions = test
            .covers
            .iter()
            .filter_map(|vo| vos.get(vo.as_str()))
            .any(|vo| !vo.dimensions.is_empty());
        if !test.covers.is_empty() && (!test.cases.is_empty() || has_dimensions) {
            // DS-1441: the item disappears only for an `accepted`
            // effective judgment.
            if let CaseCoverage::NotAccepted { basis } =
                case_coverage_judgment(layout, scan, test.id.as_str(), decisions)
            {
                items.push(PendingItem {
                    subject: test.id.as_str().to_owned(),
                    kind: "unknown",
                    check: None,
                    judgment_kind: Some("case-coverage"),
                    basis,
                    bundle_ref: None,
                });
            }
        }
    }

    // Test constructs with no management declaration. They are outside
    // any entity subtree, so they are listed only for an unscoped run.
    if outcome.scope.entity.is_none() {
        // DS-560: a missing management declaration is chain_integrity's
        // MISMATCH (MISSING); report that check's current value with it.
        let chain = outcome
            .structural
            .iter()
            .find(|check| check.check == vtest_model::VerificationCheck::ChainIntegrity)
            .map(pending_check);
        for discovered in &scan.discovered {
            if discovered.managed == ManagedTestLink::Missing {
                items.push(PendingItem {
                    subject: format!(
                        "{}::{}",
                        discovered.location.path.as_str(),
                        discovered.location.locator
                    ),
                    kind: "unregistered",
                    check: chain.clone(),
                    judgment_kind: None,
                    basis: vec![format!(
                        "{} bytes {}..{} has no management declaration",
                        discovered.location.path.as_str(),
                        discovered.location.byte_range.start,
                        discovered.location.byte_range.end
                    )],
                    bundle_ref: None,
                });
            }
        }
    }

    if let Some(gate) = gate {
        for approval in gate.approvals.iter().filter(|approval| !approval.satisfied) {
            items.push(PendingItem {
                subject: gate.name.clone(),
                kind: "pending_approval",
                check: None,
                judgment_kind: None,
                basis: vec![format!(
                    "approval role `{}` is not satisfied",
                    approval.role
                )],
                bundle_ref: None,
            });
        }
    }

    items.sort_by(|left, right| {
        (
            left.kind,
            &left.subject,
            left.check.as_ref().map(|c| c.name),
        )
            .cmp(&(
                right.kind,
                &right.subject,
                right.check.as_ref().map(|c| c.name),
            ))
    });
    items.dedup();
    items
}
