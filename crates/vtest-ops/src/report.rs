//! `vtest report` and MCP `report` (SPEC-401/SPEC-402, BD-082/BD-228/
//! BD-230, DS-1133〜DS-1147, DS-1215, DS-1505, DS-1524).
//!
//! `report` runs the same aggregation as `verify` (the shared
//! `ops::verify::evaluate`), so its state, result, tree, scope, gate and
//! `pending` section are those of `verify` for the same input. It adds:
//!
//! - the full tree regardless of `--summary` (report is the reading /
//!   submission output, verify the judging one, SPEC-402);
//! - `evidence`: the latest Evidence record id per Test in the tree, the
//!   reference behind each Test's execution-derived checks (SPEC-401);
//! - `projection`: a local trace over the declaration graph from any node
//!   (`--from` DOC / VO / TEST / SRC), up, down or both, to `--depth`
//!   steps, filtered by a role preset (`--view`). Its `derives_from` edges
//!   carry the entry's `anchor` / `note` when present and omit them
//!   otherwise (DS-1144/DS-1524); `anchor` is transported as an opaque
//!   string (DS-1146). A parent VO traced downward lists every child VO
//!   and Test with its state, which is the feature-unit breakdown
//!   (DS-1139/DS-1505); there is no Feature field (DS-1140).
//!
//! Role presets (DS-1136; the canon leaves the preset system to this
//! layer, DS-1137) select node kinds: `pm` = documents and VOs, `tester` =
//! VOs, Tests and targets, `coder` = targets, Tests, VOs and documents.

use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::path::Path;

use vtest_adapter_api::AdapterRegistry;
use vtest_model::{Diagnostic, ExitCode, TargetRef, VoRecord};
use vtest_scan::ScanResult;
use vtest_store::VerifyLayout;
use vtest_verify::{NodeKind, TreeNode};

use super::verify::{state_name, VerifyData, VerifyOpError};

#[derive(Clone, Copy, Debug, Eq, PartialEq, serde::Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Direction {
    Up,
    Down,
    Both,
}

impl Direction {
    pub fn parse(value: &str) -> Result<Self, VerifyOpError> {
        match value {
            "up" => Ok(Self::Up),
            "down" => Ok(Self::Down),
            "both" => Ok(Self::Both),
            other => Err(usage(format!(
                "--direction must be up, down or both; got `{other}`"
            ))),
        }
    }
}

pub const VIEWS: &[&str] = &["pm", "tester", "coder"];

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, serde::Serialize)]
#[serde(rename_all = "lowercase")]
pub enum GraphKind {
    Doc,
    Vo,
    Test,
    Src,
}

#[derive(Clone, Debug, serde::Serialize)]
pub struct ProjectionNode {
    pub kind: GraphKind,
    pub id: String,
    /// The node's representative state in this run's tree; `null` for a
    /// target or a node outside the requested entity scope.
    pub state: Option<&'static str>,
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd, serde::Serialize)]
pub struct ProjectionEdge {
    pub from: String,
    pub relation: &'static str,
    pub to: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub anchor: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
}

#[derive(Clone, Debug, serde::Serialize)]
pub struct Projection {
    pub from: Option<String>,
    pub direction: Direction,
    pub depth: Option<usize>,
    pub view: Option<String>,
    pub nodes: Vec<ProjectionNode>,
    pub edges: Vec<ProjectionEdge>,
}

#[derive(Clone, Debug, serde::Serialize)]
pub struct EvidenceRef {
    pub test: String,
    /// The latest record's id; `null` for an invalid record whose file name
    /// could not be read.
    pub evidence: Option<String>,
    /// The record's `result`; `null` when the latest record is invalid.
    pub result: Option<String>,
    pub executed_at: Option<String>,
    /// Set when the latest record is not a valid Evidence record: `verify`
    /// does not use it (E-SCAN-010 / E-INTERNAL-001), and no older valid
    /// record is shown in its place (DS-1365/DS-1366).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub invalid: Option<String>,
}

#[derive(Clone, serde::Serialize)]
pub struct ReportData {
    #[serde(flatten)]
    pub verify: VerifyData,
    pub evidence: Vec<EvidenceRef>,
    pub projection: Projection,
}

/// The projection request: `from` / `view` / `depth` / `direction`.
#[derive(Clone, Debug, Default)]
pub struct ProjectionArgs {
    pub from: Option<String>,
    pub view: Option<String>,
    pub depth: Option<usize>,
    pub direction: Option<String>,
}

fn usage(message: String) -> VerifyOpError {
    VerifyOpError::Usage {
        code: "E-OP-001",
        message,
        candidates: None,
    }
}

struct Graph {
    kinds: BTreeMap<String, GraphKind>,
    /// (edge, upper node, lower node)
    edges: Vec<(ProjectionEdge, String, String)>,
}

/// A Source Target's node id: its canonical Locator *with* its adapter
/// (`<adapter>::<value>`, the identity Evidence records, DS-589/DS-590), so
/// two adapters' Sources that share a locator value stay two nodes with
/// their own edges.
fn src_node_id(locator: &vtest_model::Locator) -> String {
    locator.to_string()
}

/// The Source node a TargetRef resolves to, or `None` unless exactly one
/// Source matches. A collision (e.g. a duplicated `@vtest.src-id`) leaves the
/// reference unresolved (DS-520), so the projection draws no `targets` edge
/// rather than picking whichever Source the scan listed first; the scan's own
/// diagnostic (E-SCAN-011) still reports the collision.
fn target_node(scan: &ScanResult, target: &TargetRef) -> Option<String> {
    let mut matches = scan.sources.iter().filter(|source| match target {
        TargetRef::Locator(locator) => &source.locator == locator,
        TargetRef::SrcId(src_id) => source.src_id.as_ref() == Some(src_id),
    });
    let source = matches.next()?;
    if matches.next().is_some() {
        return None;
    }
    Some(src_node_id(&source.locator))
}

fn build_graph(
    layout: &VerifyLayout,
    scan: &ScanResult,
    vos: &BTreeMap<String, VoRecord>,
) -> Graph {
    let mut kinds = BTreeMap::new();
    let mut edges = Vec::new();
    let edge = |from: &str, relation, to: &str, upper: &str, lower: &str| {
        (
            ProjectionEdge {
                from: from.to_owned(),
                relation,
                to: to.to_owned(),
                anchor: None,
                note: None,
            },
            upper.to_owned(),
            lower.to_owned(),
        )
    };
    if let Ok(index) = vtest_store::approval::build_document_node_index(layout) {
        for (id, parents) in index.iter() {
            kinds.insert(id.to_owned(), GraphKind::Doc);
            for parent in parents {
                edges.push(edge(id, "derives_from", parent, parent, id));
            }
        }
    }
    for (id, vo) in vos {
        kinds.insert(id.clone(), GraphKind::Vo);
        for entry in &vo.derives_from {
            let doc = entry.doc.as_str();
            let mut item = edge(id, "derives_from", doc, doc, id);
            item.0.anchor = entry.anchor.clone();
            item.0.note = entry.note.clone();
            edges.push(item);
        }
        if let Some(parent) = &vo.parent {
            edges.push(edge(id, "parent", parent.as_str(), parent.as_str(), id));
        }
    }
    for source in &scan.sources {
        kinds.insert(src_node_id(&source.locator), GraphKind::Src);
    }
    for test in &scan.tests {
        let id = test.id.as_str();
        kinds.insert(id.to_owned(), GraphKind::Test);
        for vo in &test.covers {
            edges.push(edge(id, "covers", vo.as_str(), vo.as_str(), id));
        }
        for target in &test.targets {
            if let Some(src) = target_node(scan, target) {
                edges.push(edge(id, "targets", &src, id, &src));
            }
        }
    }
    Graph { kinds, edges }
}

/// `--from`'s node: a DOC / VO / TEST id or a SRC node id
/// (`<adapter>::<value>`). A bare locator value is accepted when exactly one
/// adapter's Source has it; when several do, it is ambiguous and rejected.
fn resolve_start(
    graph: &Graph,
    scan: &ScanResult,
    requested: &str,
) -> Result<String, VerifyOpError> {
    if graph.kinds.contains_key(requested) {
        return Ok(requested.to_owned());
    }
    let matches: BTreeSet<String> = scan
        .sources
        .iter()
        .filter(|source| source.locator.value == requested)
        .map(|source| src_node_id(&source.locator))
        .collect();
    match matches.len() {
        1 => Ok(matches.into_iter().next().expect("one match")),
        0 => Err(usage(format!(
            "--from {requested} names no DOC / VO / TEST / SRC node"
        ))),
        _ => Err(usage(format!(
            "--from {requested} matches the Sources of more than one adapter; name one of {}",
            matches.into_iter().collect::<Vec<_>>().join(", ")
        ))),
    }
}

fn view_kinds(view: Option<&str>) -> Result<BTreeSet<GraphKind>, VerifyOpError> {
    use GraphKind::*;
    Ok(match view {
        None | Some("coder") => [Doc, Vo, Test, Src].into_iter().collect(),
        Some("pm") => [Doc, Vo].into_iter().collect(),
        Some("tester") => [Vo, Test, Src].into_iter().collect(),
        Some(other) => {
            return Err(usage(format!(
                "--view must be one of {}; got `{other}`",
                VIEWS.join(", ")
            )))
        }
    })
}

fn states_in(nodes: &[TreeNode], out: &mut BTreeMap<(GraphKind, String), &'static str>) {
    for node in nodes {
        let kind = match node.kind {
            NodeKind::Doc => GraphKind::Doc,
            NodeKind::Vo => GraphKind::Vo,
            NodeKind::Test => GraphKind::Test,
        };
        out.entry((kind, node.id.clone()))
            .or_insert_with(|| state_name(node.state));
        states_in(&node.children, out);
    }
}

/// Builds the projection over the declaration graph (DES-466: VO → Tests,
/// SRC → Tests, DOC → VOs, DOC → DOCs).
pub fn project(
    layout: &VerifyLayout,
    scan: &ScanResult,
    tree: &[TreeNode],
    args: &ProjectionArgs,
) -> Result<Projection, VerifyOpError> {
    let direction = match args.direction.as_deref() {
        Some(value) => Direction::parse(value)?,
        None => Direction::Both,
    };
    let allowed = view_kinds(args.view.as_deref())?;
    let vos = vtest_store::approval::read_all_vos(layout).unwrap_or_default();
    let graph = build_graph(layout, scan, &vos);

    let visited: BTreeSet<String> = match &args.from {
        None => graph.kinds.keys().cloned().collect(),
        Some(requested) => {
            let start = resolve_start(&graph, scan, requested)?;
            let mut seen = BTreeSet::from([start.clone()]);
            let mut queue = VecDeque::from([(start, 0usize)]);
            while let Some((node, steps)) = queue.pop_front() {
                if args.depth.is_some_and(|limit| steps >= limit) {
                    continue;
                }
                for (_, upper, lower) in &graph.edges {
                    let next = match direction {
                        Direction::Down if upper == &node => lower,
                        Direction::Up if lower == &node => upper,
                        Direction::Both if upper == &node => lower,
                        Direction::Both if lower == &node => upper,
                        _ => continue,
                    };
                    if seen.insert(next.clone()) {
                        queue.push_back((next.clone(), steps + 1));
                    }
                }
            }
            seen
        }
    };

    let mut states = BTreeMap::new();
    states_in(tree, &mut states);
    let kept: BTreeSet<&String> = visited
        .iter()
        .filter(|id| {
            graph
                .kinds
                .get(*id)
                .is_some_and(|kind| allowed.contains(kind))
        })
        .collect();
    let mut nodes: Vec<ProjectionNode> = kept
        .iter()
        .map(|id| {
            let kind = graph.kinds[*id];
            ProjectionNode {
                kind,
                id: (*id).clone(),
                state: states.get(&(kind, (*id).clone())).copied(),
            }
        })
        .collect();
    nodes.sort_by(|left, right| (left.kind, &left.id).cmp(&(right.kind, &right.id)));
    let mut edges: Vec<ProjectionEdge> = graph
        .edges
        .into_iter()
        .filter(|(edge, _, _)| kept.contains(&edge.from) && kept.contains(&edge.to))
        .map(|(edge, _, _)| edge)
        .collect();
    edges.sort();
    edges.dedup();
    Ok(Projection {
        from: args.from.clone(),
        direction,
        depth: args.depth,
        view: args.view.clone(),
        nodes,
        edges,
    })
}

fn test_ids(nodes: &[TreeNode], out: &mut BTreeSet<String>) {
    for node in nodes {
        if node.kind == NodeKind::Test {
            out.insert(node.id.clone());
        }
        test_ids(&node.children, out);
    }
}

/// The latest Evidence per Test in the tree, selected by the same rule
/// `verify` uses ([`vtest_verify::latest_evidence`]): a newer invalid record
/// is shown as invalid, never replaced by an older valid one.
fn evidence_refs(root: &Path, registry: &AdapterRegistry, tree: &[TreeNode]) -> Vec<EvidenceRef> {
    let mut wanted = BTreeSet::new();
    test_ids(tree, &mut wanted);
    vtest_verify::latest_evidence(root, registry, wanted.iter().map(String::as_str))
        .into_iter()
        .map(|(test, latest)| match latest {
            vtest_verify::LatestEvidence::Valid(record) => EvidenceRef {
                test,
                evidence: Some(record.id.clone()),
                result: serde_json::to_value(record.result)
                    .ok()
                    .and_then(|value| value.as_str().map(str::to_owned)),
                executed_at: Some(record.executed_at.clone()),
                invalid: None,
            },
            vtest_verify::LatestEvidence::Invalid { id, reason } => EvidenceRef {
                test,
                evidence: id,
                result: None,
                executed_at: None,
                invalid: Some(reason),
            },
        })
        .collect()
}

/// `vtest report` / MCP `report`.
#[allow(clippy::too_many_arguments)]
pub fn execute(
    root: &Path,
    items: &[String],
    doc: Option<String>,
    vo: Option<String>,
    test: Option<String>,
    gate: Option<&str>,
    projection: &ProjectionArgs,
    registry: &AdapterRegistry,
) -> Result<(ExitCode, ReportData, Vec<Diagnostic>), VerifyOpError> {
    // Validate the projection arguments before running anything, so a bad
    // request returns no partial result.
    if let Some(value) = projection.direction.as_deref() {
        Direction::parse(value)?;
    }
    view_kinds(projection.view.as_deref())?;
    let evaluated = super::verify::evaluate(root, items, doc, vo, test, gate, false, registry)?;
    let layout = VerifyLayout::new(root);
    let projection = project(&layout, &evaluated.scan, &evaluated.data.tree, projection)?;
    let evidence = evidence_refs(root, registry, &evaluated.data.tree);
    Ok((
        evaluated.exit,
        ReportData {
            verify: evaluated.data,
            evidence,
            projection,
        },
        evaluated.diagnostics,
    ))
}
