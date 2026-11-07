//! `vtest vo add|edit|list|show|expand|approve` and the MCP `vo_*` tools
//! (別紙A §12.2 DS-1021〜DS-1048, §13.2 DS-1199〜DS-1202 / DS-1218〜DS-1226).
//!
//! Every function here is the single shared operation that both the CLI
//! wrapper and the MCP tool handler call (DS-1563), so the two interfaces
//! cannot drift apart.
//!
//! Scope boundaries (stated, not hidden):
//! - `vo approve` / `vo_approve` are aliases of `approval create` with
//!   `subject_type: vo` (DS-1045/DS-1046, SPEC-383/SPEC-410). They call
//!   `ops::approval::create` and add no rule of their own.
//! - `vo show`'s judgment-record status (DS-1021's usage note) is not
//!   reported: no judgment-record domain exists in this codebase yet (see
//!   `ops::approval`'s module doc comment).
//! - Child VO field contents for `vo expand` are not fixed by the canon
//!   beyond the id (DS-410/DS-411) and parent. Children copy the parent's
//!   `derives_from`, carry the parent's claim suffixed with the
//!   `dimension=partition` tuple they materialize, and declare no
//!   dimensions of their own. See `expand`.

use std::collections::{BTreeMap, BTreeSet};

use vtest_model::{
    CombinationEntry, CoveragePolicy, DerivesFrom, Dimension, DocumentId, VoId, VoRecord,
};
use vtest_store::{
    approval::{build_document_node_index, read_all_vos, EffectiveApprovalState},
    canonical::write_vo_record,
    StoreError, VerifyLayout,
};

use super::approval::{self, ApprovalOpError};

#[derive(Debug, thiserror::Error)]
pub enum VoOpError {
    /// E-OP-001 (DS-918): input validation failure, including an unknown or
    /// already-existing VO id and malformed `--dimension` / `--combination`
    /// / `--anchor` arguments (DS-1029/DS-1040).
    #[error("E-OP-001: {0}")]
    Usage(String),
    /// E-SCAN-017 (DS-902/DS-1038/DS-1044/DS-1224/DS-1226/DS-1318):
    /// `combinations` invalid for the declared dimensions / policy. Nothing
    /// is written.
    #[error("E-SCAN-017: {0}")]
    InvalidCombinations(String),
    #[error(transparent)]
    Approval(#[from] ApprovalOpError),
    #[error("store error: {0}")]
    Store(#[from] StoreError),
}

impl VoOpError {
    /// The diagnostic code for an operation rejection (exit 2). `None` for
    /// a store failure that is not a record-schema rejection, which callers
    /// report as an internal error.
    pub fn code(&self) -> Option<&'static str> {
        match self {
            Self::Usage(_) => Some("E-OP-001"),
            Self::InvalidCombinations(_) => Some("E-SCAN-017"),
            Self::Approval(ApprovalOpError::JudgmentSubjectTypeUnsupported)
            | Self::Approval(ApprovalOpError::UnresolvedSubject(_)) => Some("E-APPROVAL-001"),
            Self::Approval(ApprovalOpError::InvalidRequest(_)) => Some("E-APPROVAL-002"),
            Self::Approval(ApprovalOpError::Store(error)) | Self::Store(error) => {
                error.record_read_code()
            }
        }
    }

    /// The message without the `E-xxx: ` prefix the `Display` impl adds.
    pub fn message(&self) -> String {
        match self {
            Self::Usage(message) | Self::InvalidCombinations(message) => message.clone(),
            other => other.to_string(),
        }
    }
}

// ---------------------------------------------------------------------------
// Argument parsing shared by CLI and MCP
// ---------------------------------------------------------------------------

/// DS-1021: `--dimension <name>=<p1>,<p2>...`.
pub fn parse_dimension(value: &str) -> Result<Dimension, VoOpError> {
    let Some((name, partitions)) = value.split_once('=') else {
        return Err(VoOpError::Usage(format!(
            "--dimension must be <name>=<p1>,<p2>...; got `{value}`"
        )));
    };
    let partitions: Vec<String> = partitions.split(',').map(str::to_owned).collect();
    if name.is_empty() || partitions.iter().any(String::is_empty) {
        return Err(VoOpError::Usage(format!(
            "--dimension must name a dimension and at least one non-empty partition; got `{value}`"
        )));
    }
    Ok(Dimension {
        name: name.to_owned(),
        partitions,
    })
}

/// DS-1033/DS-1040: one `--combination` is one tuple, `<dim>=<part>`
/// pairs separated by commas. A value that is not of that form is an
/// argument error (exit 2). Repeated dimension names are kept so the
/// E-SCAN-017 check can report them (DS-422).
pub fn parse_combination(value: &str) -> Result<CombinationEntry, VoOpError> {
    let mut pairs = Vec::new();
    for pair in value.split(',') {
        match pair.split_once('=') {
            Some((dim, part)) if !dim.is_empty() && !part.is_empty() => {
                pairs.push((dim.to_owned(), part.to_owned()));
            }
            _ => {
                return Err(VoOpError::Usage(format!(
                    "--combination must be <dim>=<part>[,<dim>=<part>]...; got `{value}`"
                )))
            }
        }
    }
    Ok(pairs.into_iter().collect())
}

/// `--policy` / `coverage_policy`: the §3.2.1 domain plus `null`.
pub fn parse_policy(value: &str) -> Result<Option<CoveragePolicy>, VoOpError> {
    match value {
        "independent-axes" => Ok(Some(CoveragePolicy::IndependentAxes)),
        "full-product" => Ok(Some(CoveragePolicy::FullProduct)),
        "explicit" => Ok(Some(CoveragePolicy::Explicit)),
        "null" => Ok(None),
        other => Err(VoOpError::Usage(format!(
            "coverage policy must be independent-axes, full-product, explicit or null; got `{other}`"
        ))),
    }
}

/// The fields `vo add` / `vo edit` take, after the order-sensitive
/// `--derives-from` / `--anchor` / `--note` binding has been resolved.
#[derive(Debug, Default)]
pub struct VoArgs {
    pub id: Option<String>,
    pub claim: Option<String>,
    /// `None` = not given. `Some` = desired state (replaces the record's
    /// `derives_from`).
    pub derives_from: Option<Vec<DerivesFrom>>,
    pub parent: Option<String>,
    /// `None` = not given; `Some` replaces `dimensions`.
    pub dimensions: Option<Vec<Dimension>>,
    /// `None` = not given; `Some(None)` sets `coverage_policy: null`.
    pub policy: Option<Option<CoveragePolicy>>,
    pub combinations: CombinationsUpdate,
}

/// DS-1035〜DS-1037 / DS-1221 / DS-1225: `combinations` is desired state.
#[derive(Debug, Default)]
pub enum CombinationsUpdate {
    /// Neither `--combination` nor `--clear-combinations` (MCP: field
    /// omitted) — keep the existing value.
    #[default]
    Keep,
    /// `--combination` given at least once (MCP: a non-empty list) —
    /// replace with exactly these tuples.
    Replace(Vec<CombinationEntry>),
    /// `--clear-combinations` (MCP: an explicit empty list).
    Clear,
}

/// Parses the raw `vo add` / `vo edit` argument list. Hand-parsed rather
/// than declared to clap because DS-1027〜DS-1029 bind each `--anchor` /
/// `--note` to the `--derives-from` immediately before it, which a
/// declarative parser cannot express. A positional argument is the VO id
/// (`vo edit VO-X`).
pub fn parse_vo_args(raw: &[String]) -> Result<VoArgs, VoOpError> {
    let mut args = VoArgs::default();
    let mut derives: Vec<DerivesFrom> = Vec::new();
    let mut anchor_bound = false;
    let mut note_bound = false;
    let mut combinations = Vec::new();
    let mut clear = false;
    let mut dimensions = Vec::new();
    let mut iter = raw.iter();
    while let Some(arg) = iter.next() {
        let (flag, inline) = match arg.split_once('=') {
            Some((flag, value)) if flag.starts_with("--") => (flag, Some(value.to_owned())),
            _ => (arg.as_str(), None),
        };
        let mut value = |name: &str| -> Result<String, VoOpError> {
            match inline.clone() {
                Some(value) => Ok(value),
                None => iter
                    .next()
                    .cloned()
                    .ok_or_else(|| VoOpError::Usage(format!("{name} requires a value"))),
            }
        };
        match flag {
            "--id" => args.id = Some(value("--id")?),
            "--claim" => args.claim = Some(value("--claim")?),
            "--derives-from" => {
                derives.push(DerivesFrom {
                    doc: DocumentId::new(value("--derives-from")?),
                    anchor: None,
                    note: None,
                });
                anchor_bound = false;
                note_bound = false;
            }
            "--anchor" | "--note" => {
                let text = value(flag)?;
                let Some(last) = derives.last_mut() else {
                    return Err(VoOpError::Usage(format!(
                        "{flag} must follow a --derives-from (DS-1029)"
                    )));
                };
                let bound = if flag == "--anchor" {
                    &mut anchor_bound
                } else {
                    &mut note_bound
                };
                if *bound {
                    return Err(VoOpError::Usage(format!(
                        "only one {flag} may follow each --derives-from (DS-1029)"
                    )));
                }
                *bound = true;
                if flag == "--anchor" {
                    last.anchor = Some(text);
                } else {
                    last.note = Some(text);
                }
            }
            "--parent" => args.parent = Some(value("--parent")?),
            "--dimension" => dimensions.push(parse_dimension(&value("--dimension")?)?),
            "--policy" => args.policy = Some(parse_policy(&value("--policy")?)?),
            "--combination" => combinations.push(parse_combination(&value("--combination")?)?),
            "--clear-combinations" if inline.is_none() => clear = true,
            positional if !positional.starts_with('-') && args.id.is_none() => {
                args.id = Some(positional.to_owned());
            }
            other => {
                return Err(VoOpError::Usage(format!("unexpected argument `{other}`")));
            }
        }
    }
    if clear && !combinations.is_empty() {
        return Err(VoOpError::Usage(
            "--combination and --clear-combinations cannot be combined".to_owned(),
        ));
    }
    if !derives.is_empty() {
        args.derives_from = Some(derives);
    }
    if !dimensions.is_empty() {
        args.dimensions = Some(dimensions);
    }
    args.combinations = if clear {
        CombinationsUpdate::Clear
    } else if combinations.is_empty() {
        CombinationsUpdate::Keep
    } else {
        CombinationsUpdate::Replace(combinations)
    };
    Ok(args)
}

// ---------------------------------------------------------------------------
// add / edit / upsert
// ---------------------------------------------------------------------------

/// Result of a create or update.
#[derive(Debug, serde::Serialize)]
pub struct VoWriteResult {
    pub created: bool,
    pub record: VoRecord,
    /// DS-1048/DS-1200: the VO's effective approval was `approved` before
    /// this edit. The edit is still applied.
    pub was_approved: bool,
    /// The edit changed the VO subject hash, so any prior approval no
    /// longer binds (§3.5). An `anchor`-only edit keeps it (DS-1031).
    pub approval_invalidated: bool,
    /// Human-readable warnings (DS-1048). The canon assigns no diagnostic
    /// code to this warning, so it is reported here rather than as a coded
    /// diagnostic.
    pub warnings: Vec<String>,
}

fn validate_record(record: &VoRecord) -> Result<(), VoOpError> {
    if record.derives_from.is_empty() {
        return Err(VoOpError::Usage(
            "a VO needs at least one --derives-from (DS-1021)".to_owned(),
        ));
    }
    if record.claim.trim().is_empty() {
        return Err(VoOpError::Usage("a VO needs a non-empty claim".to_owned()));
    }
    if let Some(message) = vtest_scan::invalid_vo_combinations(record) {
        return Err(VoOpError::InvalidCombinations(format!(
            "VO {} {message}",
            record.id.as_str()
        )));
    }
    Ok(())
}

fn require_id(id: Option<&str>) -> Result<&str, VoOpError> {
    match id {
        Some(id) if !id.trim().is_empty() => {
            require_valid_id(id)?;
            Ok(id)
        }
        _ => Err(VoOpError::Usage("a VO id is required".to_owned())),
    }
}

/// A VO id must name a file inside `.verify/vo/` (no `..`, separator or
/// absolute path). The store rejects such an id too; checking it here
/// reports it as a usage error (E-OP-001) before anything is read or
/// written, for the CLI and MCP add/upsert/edit/expand alike.
fn require_valid_id(id: &str) -> Result<(), VoOpError> {
    if vtest_store::canonical::is_valid_vo_id(id) {
        Ok(())
    } else {
        Err(VoOpError::Usage(
            vtest_store::StoreError::InvalidVoId(id.to_owned()).to_string(),
        ))
    }
}

/// A new VO id must not differ from an existing one only in ASCII case.
/// On a case-insensitive file system (macOS, Windows) `vo-x.yaml` and
/// `VO-X.yaml` are one file, so creating `vo-x` would overwrite `VO-X`.
/// Rejected for every file system, so the result does not depend on where
/// the project lives.
fn reject_case_variant(vos: &BTreeMap<String, VoRecord>, id: &str) -> Result<(), VoOpError> {
    match vos
        .keys()
        .find(|existing| existing.as_str() != id && existing.eq_ignore_ascii_case(id))
    {
        Some(existing) => Err(VoOpError::Usage(format!(
            "VO id {id} differs from the existing VO {existing} only in case and would share \
             its record file on a case-insensitive file system; nothing was written"
        ))),
        None => Ok(()),
    }
}

/// `vo add` (DS-1021): creates a new VO record. Rejects an existing id, a
/// missing `--derives-from` or `--claim`, and invalid `combinations`
/// (E-SCAN-017) without writing anything (DS-1039/DS-1318).
pub fn add(layout: &VerifyLayout, args: VoArgs) -> Result<VoWriteResult, VoOpError> {
    let id = require_id(args.id.as_deref())?.to_owned();
    let vos = read_all_vos(layout)?;
    if vos.contains_key(&id) {
        return Err(VoOpError::Usage(format!(
            "VO {id} already exists; use `vo edit` to change it"
        )));
    }
    reject_case_variant(&vos, &id)?;
    let combinations = match args.combinations {
        CombinationsUpdate::Replace(entries) => entries,
        CombinationsUpdate::Keep | CombinationsUpdate::Clear => Vec::new(),
    };
    let now = vtest_store::records::now_rfc3339();
    let record = VoRecord {
        id: VoId::new(id),
        parent: args.parent.map(VoId::new),
        derives_from: args.derives_from.unwrap_or_default(),
        claim: args
            .claim
            .ok_or_else(|| VoOpError::Usage("--claim is required".to_owned()))?,
        dimensions: args.dimensions.unwrap_or_default(),
        coverage_policy: args.policy.flatten(),
        combinations,
        representative_cases: Vec::new(),
        created: now.clone(),
        updated: now,
    };
    validate_record(&record)?;
    write_vo_record(layout, &record)?;
    Ok(VoWriteResult {
        created: true,
        record,
        was_approved: false,
        approval_invalidated: false,
        warnings: Vec::new(),
    })
}

fn effective_state(layout: &VerifyLayout, id: &str) -> Result<&'static str, VoOpError> {
    let shown = approval::show(layout, "vo", id)?;
    Ok(match shown.effective_state {
        EffectiveApprovalState::Approved => "approved",
        EffectiveApprovalState::Draft => "draft",
    })
}

/// `vo edit VO-X` (DS-1021/DS-1035〜DS-1037/DS-1048): applies only the
/// given fields. `combinations` follows `CombinationsUpdate`. Editing an
/// approved VO is allowed and warned about; the approval lapses through the
/// subject-hash mismatch, not through any deletion here.
pub fn edit(layout: &VerifyLayout, args: VoArgs) -> Result<VoWriteResult, VoOpError> {
    let id = require_id(args.id.as_deref())?.to_owned();
    let vos = read_all_vos(layout)?;
    let Some(existing) = vos.get(&id).cloned() else {
        return Err(VoOpError::Usage(format!("no VO with id '{id}' exists")));
    };
    let was_approved = effective_state(layout, &id)? == "approved";
    let before = vtest_model::vo_subject_hash(&existing);

    let mut record = existing;
    if let Some(claim) = args.claim {
        record.claim = claim;
    }
    if let Some(derives_from) = args.derives_from {
        record.derives_from = derives_from;
    }
    if let Some(parent) = args.parent {
        record.parent = Some(VoId::new(parent));
    }
    if let Some(dimensions) = args.dimensions {
        record.dimensions = dimensions;
    }
    if let Some(policy) = args.policy {
        record.coverage_policy = policy;
    }
    match args.combinations {
        CombinationsUpdate::Keep => {}
        CombinationsUpdate::Replace(entries) => record.combinations = entries,
        CombinationsUpdate::Clear => record.combinations = Vec::new(),
    }
    record.updated = vtest_store::records::now_rfc3339();
    validate_record(&record)?;
    let approval_invalidated = was_approved && vtest_model::vo_subject_hash(&record) != before;
    write_vo_record(layout, &record)?;

    let mut warnings = Vec::new();
    if was_approved {
        warnings.push(if approval_invalidated {
            format!(
                "VO {id} was approved; this edit changes its subject hash, so the approval no \
                 longer applies and its effective state is now draft"
            )
        } else {
            format!(
                "VO {id} is approved; this edit does not change its subject hash (e.g. an \
                 anchor-only change, DS-1031), so the approval still applies"
            )
        });
    }
    Ok(VoWriteResult {
        created: false,
        record,
        was_approved,
        approval_invalidated,
        warnings,
    })
}

/// MCP `vo_upsert` (DS-1200/DS-1218〜DS-1225): creates the VO when absent,
/// otherwise updates it. `derives_from[]` is required with at least one
/// entry on both paths.
pub fn upsert(layout: &VerifyLayout, args: VoArgs) -> Result<VoWriteResult, VoOpError> {
    if args.derives_from.as_ref().is_none_or(Vec::is_empty) {
        return Err(VoOpError::Usage(
            "derives_from[] must contain at least one entry (DS-1200)".to_owned(),
        ));
    }
    let id = require_id(args.id.as_deref())?.to_owned();
    if read_all_vos(layout)?.contains_key(&id) {
        edit(layout, args)
    } else {
        add(layout, args)
    }
}

// ---------------------------------------------------------------------------
// list / show
// ---------------------------------------------------------------------------

#[derive(Debug, serde::Serialize)]
pub struct VoSummary {
    pub id: String,
    pub parent: Option<String>,
    pub claim: String,
    /// DS-1023/DS-1047: derived from approval records, never stored.
    pub status: &'static str,
    pub derives_from: Vec<DerivesFrom>,
}

/// Every document node reachable upstream from `start` (inclusive).
fn document_closure(
    index: &vtest_store::approval::DocumentNodeIndex,
    start: &str,
) -> BTreeSet<String> {
    let mut seen = BTreeSet::new();
    let mut stack = vec![start.to_owned()];
    while let Some(node) = stack.pop() {
        if !seen.insert(node.clone()) {
            continue;
        }
        if let Some((_, parents)) = index.get(&node) {
            stack.extend(parents.iter().cloned());
        }
    }
    seen
}

/// `vo list [--doc DOC-X] [--status draft|approved]` and MCP `vo_list`
/// (DS-1026/DS-1047/DS-1199). `--doc` keeps the VOs downstream of that
/// document node: a VO matches when DOC-X is one of its `derives_from`
/// targets or upstream of one, and the descendants (by `parent`) of a
/// matching VO match too.
pub fn list(
    layout: &VerifyLayout,
    doc: Option<&str>,
    status: Option<&str>,
) -> Result<Vec<VoSummary>, VoOpError> {
    if let Some(status) = status {
        if status != "draft" && status != "approved" {
            return Err(VoOpError::Usage(format!(
                "--status must be draft or approved; got `{status}`"
            )));
        }
    }
    let vos = read_all_vos(layout)?;
    let selected: BTreeSet<String> = match doc {
        None => vos.keys().cloned().collect(),
        Some(doc) => {
            let index = build_document_node_index(layout)?;
            if index.get(doc).is_none() {
                return Err(VoOpError::Usage(format!(
                    "no document node with id '{doc}' exists"
                )));
            }
            let direct: BTreeSet<String> = vos
                .values()
                .filter(|vo| {
                    vo.derives_from
                        .iter()
                        .any(|entry| document_closure(&index, entry.doc.as_str()).contains(doc))
                })
                .map(|vo| vo.id.as_str().to_owned())
                .collect();
            vos.keys()
                .filter(|id| has_selected_ancestor(&vos, id, &direct))
                .cloned()
                .collect()
        }
    };
    let mut out = Vec::new();
    for id in selected {
        let vo = &vos[&id];
        let state = effective_state(layout, &id)?;
        if status.is_some_and(|wanted| wanted != state) {
            continue;
        }
        out.push(VoSummary {
            id: id.clone(),
            parent: vo.parent.as_ref().map(|parent| parent.as_str().to_owned()),
            claim: vo.claim.clone(),
            status: state,
            derives_from: vo.derives_from.clone(),
        });
    }
    Ok(out)
}

fn has_selected_ancestor(
    vos: &BTreeMap<String, VoRecord>,
    id: &str,
    selected: &BTreeSet<String>,
) -> bool {
    let mut seen = BTreeSet::new();
    let mut cursor = Some(id.to_owned());
    while let Some(current) = cursor {
        if selected.contains(&current) {
            return true;
        }
        if !seen.insert(current.clone()) {
            return false;
        }
        cursor = vos
            .get(&current)
            .and_then(|vo| vo.parent.as_ref())
            .map(|parent| parent.as_str().to_owned());
    }
    false
}

#[derive(Debug, serde::Serialize)]
pub struct VoView {
    pub record: VoRecord,
    pub status: &'static str,
    /// Ids of the Tests whose `covers` name this VO (DS-1199 "covers 状況").
    pub covered_by: Vec<String>,
}

/// `vo show VO-X` / MCP `vo_get` (DS-1030/DS-1047/DS-1199).
pub fn show(
    layout: &VerifyLayout,
    scan: &vtest_scan::ScanResult,
    id: &str,
) -> Result<VoView, VoOpError> {
    let vos = read_all_vos(layout)?;
    let Some(record) = vos.get(id).cloned() else {
        return Err(VoOpError::Usage(format!("no VO with id '{id}' exists")));
    };
    let covered_by = scan
        .tests
        .iter()
        .filter(|test| test.covers.iter().any(|vo| vo.as_str() == id))
        .map(|test| test.id.as_str().to_owned())
        .collect();
    Ok(VoView {
        status: effective_state(layout, id)?,
        record,
        covered_by,
    })
}

// ---------------------------------------------------------------------------
// expand
// ---------------------------------------------------------------------------

#[derive(Debug, serde::Serialize)]
pub struct ExpandResult {
    pub dry_run: bool,
    pub children: Vec<VoRecord>,
}

/// `vo expand VO-X [--dry-run]` / MCP `vo_expand` (DS-407〜DS-416,
/// DS-1041〜DS-1044, DS-1201, DS-1226).
///
/// - `independent-axes`: one child per partition of each dimension.
/// - `full-product`: one child per element of the Cartesian product.
/// - `explicit`: one child per `combinations` tuple.
///
/// Child ids are `VO-X-<SUFFIX>`, where the suffix is the upper-cased
/// partition values joined in `dimensions` declaration order (DS-410〜
/// DS-412). Invalid `combinations` is E-SCAN-017. Nothing is written when
/// any check fails, including when a child id already exists, so a run
/// never generates part of the set (DS-416/DS-1044).
pub fn expand(layout: &VerifyLayout, id: &str, dry_run: bool) -> Result<ExpandResult, VoOpError> {
    require_valid_id(id)?;
    let vos = read_all_vos(layout)?;
    let Some(parent) = vos.get(id) else {
        return Err(VoOpError::Usage(format!("no VO with id '{id}' exists")));
    };
    if let Some(message) = vtest_scan::invalid_vo_combinations(parent) {
        return Err(VoOpError::InvalidCombinations(format!("VO {id} {message}")));
    }
    let Some(policy) = parent.coverage_policy else {
        return Err(VoOpError::Usage(format!(
            "VO {id} has coverage_policy null; there is nothing to expand"
        )));
    };
    if parent.dimensions.is_empty() {
        return Err(VoOpError::Usage(format!(
            "VO {id} declares no dimensions; there is nothing to expand"
        )));
    }

    let tuples: Vec<Vec<(String, String)>> = match policy {
        CoveragePolicy::IndependentAxes => parent
            .dimensions
            .iter()
            .flat_map(|dimension| {
                dimension
                    .partitions
                    .iter()
                    .map(|partition| vec![(dimension.name.clone(), partition.clone())])
            })
            .collect(),
        CoveragePolicy::FullProduct => {
            let mut product: Vec<Vec<(String, String)>> = vec![Vec::new()];
            for dimension in &parent.dimensions {
                product = product
                    .into_iter()
                    .flat_map(|prefix| {
                        dimension.partitions.iter().map(move |partition| {
                            let mut tuple = prefix.clone();
                            tuple.push((dimension.name.clone(), partition.clone()));
                            tuple
                        })
                    })
                    .collect();
            }
            product
        }
        CoveragePolicy::Explicit => parent
            .combinations
            .iter()
            .map(|entry| {
                // DS-411/DS-412: declaration order, independent of the
                // entry's own key order.
                parent
                    .dimensions
                    .iter()
                    .filter_map(|dimension| {
                        entry
                            .get(&dimension.name)
                            .map(|value| (dimension.name.clone(), value.to_owned()))
                    })
                    .collect()
            })
            .collect(),
    };

    let now = vtest_store::records::now_rfc3339();
    let mut seen = BTreeSet::new();
    let mut children = Vec::new();
    for tuple in tuples {
        let suffix = tuple
            .iter()
            .map(|(_, partition)| partition.to_uppercase())
            .collect::<Vec<_>>()
            .join("-");
        let child_id = format!("{id}-{suffix}");
        // A partition value is part of the child id, so `../x` or `a/b`
        // would name a file outside `.verify/vo/`. Checked for every child
        // before any is written, so nothing is generated (DS-416).
        if !vtest_store::canonical::is_valid_vo_id(&child_id) {
            return Err(VoOpError::Usage(format!(
                "expanding VO {id} yields the child id {child_id:?}, which cannot name a VO \
                 record file; nothing was generated"
            )));
        }
        if !seen.insert(child_id.clone()) {
            return Err(VoOpError::Usage(format!(
                "expanding VO {id} yields the child id {child_id} more than once (partition \
                 values that differ only in case); nothing was generated"
            )));
        }
        if vos.contains_key(&child_id) {
            return Err(VoOpError::Usage(format!(
                "child VO {child_id} already exists; nothing was generated"
            )));
        }
        reject_case_variant(&vos, &child_id)?;
        let label = tuple
            .iter()
            .map(|(dimension, partition)| format!("{dimension}={partition}"))
            .collect::<Vec<_>>()
            .join(", ");
        children.push(VoRecord {
            id: VoId::new(child_id),
            parent: Some(parent.id.clone()),
            derives_from: parent.derives_from.clone(),
            claim: format!("{} [{label}]", parent.claim),
            dimensions: Vec::new(),
            coverage_policy: None,
            combinations: Vec::new(),
            representative_cases: Vec::new(),
            created: now.clone(),
            updated: now.clone(),
        });
    }
    if !dry_run {
        for child in &children {
            write_vo_record(layout, child)?;
        }
    }
    Ok(ExpandResult { dry_run, children })
}

// ---------------------------------------------------------------------------
// approve
// ---------------------------------------------------------------------------

/// `vo approve VO-X` / MCP `vo_approve`: exactly `approval create
/// --subject-type vo --subject-id VO-X` (DS-1045/DS-1202, SPEC-383/SPEC-410).
pub fn approve(
    layout: &VerifyLayout,
    id: &str,
    args: approval::CreateArgs,
) -> Result<vtest_store::records::ApprovalRecord, VoOpError> {
    Ok(approval::create(
        layout,
        approval::CreateArgs {
            subject_type: "vo".to_owned(),
            subject_id: id.to_owned(),
            ..args
        },
    )?)
}
