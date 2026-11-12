//! `vtest run`: execute a Test and record Evidence.
//!
//! DS-744「`vtest run`は`--test` / `--vo` / `--all`で対象を受け取り、検証
//! グラフからTest集合へ展開する（VO指定は部分木のcoversを辿る）」— all
//! three axes are implemented: `--test` selects by explicit Test id,
//! `--vo` expands the VO subtree (`vtest_verify::vo_subtree_ids`, the same
//! parent-chain closure `ops::verify`'s own `--vo` entity scope uses) and
//! selects every Test whose `covers` intersects it, `--all` runs every Test
//! the scan materialized.
//!
//! DS-1101 (旧 `--req` は document 層の総称化により廃止し、VO 部分木経由で
//! 指定する), DS-1102/DS-1103 (`--fast` は cargo test のみで実行し、
//! `target_coverage` を `checked: false` として記録、`target_binding` の
//! 動的証拠を採らない). The base per-Test execution/Evidence-write behavior
//! itself is `vtest-exec::run_tests`'s existing contract (unchanged here);
//! this module only resolves which `TestEntity`s to hand it and how to
//! build each `RunnableTest`'s `target_hashes`/`target_locators`, following
//! the pattern `crates/vtest-cli/tests/evidence_e2e.rs` already exercises
//! against the library entry points directly (that test file's own comment
//! disclosed no `vtest run`/`vtest exec` CLI surface existed yet).

use std::path::Path;

use vtest_adapter_api::AdapterRegistry;
use vtest_exec::{run_tests, ExecutionError, ExecutionResult, RunnableTest};
use vtest_model::{Diagnostic, SourceFunction, TargetRef, TestEntity, TestId};
use vtest_scan::ScanResult;
use vtest_store::{approval::read_all_vos, StoreError, VerifyLayout};
use vtest_verify::vo_subtree_ids;

use crate::envelope::{record_failure, Failure};

#[derive(Debug, thiserror::Error)]
pub enum RunOpError {
    /// DS-744/DS-1563: `test`, `vo` and `all` are mutually exclusive target
    /// axes; a combination must be rejected (E-OP-001), never narrowed or
    /// widened to one axis.
    #[error("{0}")]
    ConflictingTargets(String),
    #[error("no Test with id '{0}' was discovered by scan")]
    UnknownTestId(String),
    #[error("no VO with id '{0}' exists")]
    UnknownVoId(String),
    #[error(transparent)]
    Execution(#[from] ExecutionError),
    #[error("store error: {0}")]
    Store(#[from] StoreError),
}

impl RunOpError {
    /// The exit code and diagnostic both entrypoints report.
    ///
    /// DS-745/DS-923: a registered adapter whose runner cannot interpret the
    /// Test's execution descriptor is a rejection with the code
    /// `ExecutionError::code` assigns (E-ADAPTER-003); an execution failure
    /// with no assigned code is an internal error (DS-1689/DS-1690).
    pub fn failure(&self) -> Failure {
        match self {
            Self::UnknownTestId(_) | Self::UnknownVoId(_) | Self::ConflictingTargets(_) => {
                Failure::usage("E-OP-001", self.to_string())
            }
            Self::Execution(error) => match error.code() {
                Some(code) => Failure::usage(code, self.to_string()),
                None => Failure::internal(self.to_string()),
            },
            Self::Store(error) => record_failure(error),
        }
    }
}

/// DS-744's three target axes. Exactly one applies at a time — the CLI/MCP
/// layer enforces mutual exclusivity before constructing this (mirroring
/// `ops::verify`'s `EntityScope` exclusivity for the same reason: an
/// ambiguous combination must not silently narrow to one axis's guess).
pub enum RunTarget {
    Test(Vec<String>),
    Vo(String),
    All,
}

impl RunTarget {
    /// Builds the single target axis from the raw `test` / `vo` / `all`
    /// inputs shared by the CLI and MCP. More than one axis given is
    /// `ConflictingTargets` (E-OP-001), so a request cannot silently drop
    /// the Test ids it named or widen to every Test (DS-744/DS-1563).
    pub fn from_axes(
        test_ids: Vec<String>,
        vo: Option<String>,
        all: bool,
    ) -> Result<Self, RunOpError> {
        let mut given = Vec::new();
        if !test_ids.is_empty() {
            given.push("test");
        }
        if vo.is_some() {
            given.push("vo");
        }
        if all {
            given.push("all");
        }
        if given.len() > 1 {
            return Err(RunOpError::ConflictingTargets(format!(
                "run target axes are mutually exclusive; got {}",
                given.join(" + ")
            )));
        }
        Ok(match (vo, all) {
            (Some(vo_id), _) => Self::Vo(vo_id),
            (None, true) => Self::All,
            (None, false) => Self::Test(test_ids),
        })
    }
}

/// Resolves the requested target to the `TestEntity` set to run.
///
/// `RunTarget::Test`: an id not discovered by scan is `UnknownTestId` (E-OP-001
/// usage error) — an unresolved filter must not silently narrow to "whatever
/// matched", mirroring `vtest verify`'s `--items` rejection of unknown names
/// for the same reason. An empty id list runs every Test the scan
/// materialized (kept for the CLI's pre-DS-744 default-target behavior;
/// `RunTarget::All` is the same behavior named explicitly).
///
/// `RunTarget::Vo`: DS-744 "VO指定は部分木のcoversを辿る" — the VO subtree
/// (`vo_subtree_ids`, root VO plus every descendant reachable through
/// `parent`) union-selects every Test whose `covers` names any subtree
/// member. An unresolved VO id is `UnknownVoId` (E-OP-001).
pub fn resolve_runnables(
    scan: &ScanResult,
    layout: &VerifyLayout,
    target: &RunTarget,
) -> Result<Vec<TestEntity>, RunOpError> {
    match target {
        RunTarget::All => Ok(scan.tests.clone()),
        RunTarget::Test(test_ids) => {
            if test_ids.is_empty() {
                return Ok(scan.tests.clone());
            }
            let mut selected = Vec::with_capacity(test_ids.len());
            for id in test_ids {
                let test_id = TestId::new(id.as_str());
                let Some(entity) = scan.tests.iter().find(|test| test.id == test_id) else {
                    return Err(RunOpError::UnknownTestId(id.clone()));
                };
                selected.push(entity.clone());
            }
            Ok(selected)
        }
        RunTarget::Vo(vo_id) => {
            let vos = read_all_vos(layout)?;
            if !vos.contains_key(vo_id) {
                return Err(RunOpError::UnknownVoId(vo_id.clone()));
            }
            let subtree = vo_subtree_ids(&vos, vo_id);
            Ok(scan
                .tests
                .iter()
                .filter(|test| {
                    test.covers
                        .iter()
                        .any(|covered| subtree.contains(covered.as_str()))
                })
                .cloned()
                .collect())
        }
    }
}

/// Builds the `RunnableTest` for one `TestEntity`: resolves its declared
/// targets (Locator or SRC ID, if any) against the scan's discovered Source
/// Functions to obtain their current content hashes and canonical
/// Locators. Every resolved target is handed to the runner, so each one is
/// measured and recorded (DS-831/DS-1411), not only the first. A Test with
/// zero targets (DS-1673 permits this) yields empty `target_hashes` and
/// `target_locators` — `run_tests` already handles that shape; this
/// function does not itself decide what that implies for verification.
fn build_runnable(entity: &TestEntity, scan: &ScanResult) -> RunnableTest {
    let mut target_hashes = Vec::new();
    let mut target_locators = Vec::new();
    for target in &entity.targets {
        // `run` only builds a Test whose every target matched exactly one
        // Source Target (`unresolved_targets`), so a Locator and a SrcId
        // target are both recorded here; none is silently dropped.
        if let [source_fn] = matching_sources(target, scan).as_slice() {
            target_hashes.push(source_fn.content_hash.clone());
            target_locators.push(source_fn.locator.clone());
        }
    }
    RunnableTest {
        entity: entity.clone(),
        target_hashes,
        target_locators,
    }
}

pub fn run(
    root: &Path,
    layout: &VerifyLayout,
    scan: &ScanResult,
    target: &RunTarget,
    fast: bool,
    registry: &AdapterRegistry,
) -> Result<ExecutionResult, RunOpError> {
    let entities = resolve_runnables(scan, layout, target)?;
    let mut unresolved = Vec::new();
    let runnables = entities
        .iter()
        .filter(|entity| {
            let failures = unresolved_targets(entity, scan);
            let runnable = failures.is_empty();
            unresolved.extend(failures);
            runnable
        })
        .map(|entity| build_runnable(entity, scan))
        .collect::<Vec<_>>();
    let mut result = run_tests(root, layout, &runnables, fast, registry)?;
    // The skipped Tests' diagnostics stay in the result, so the run is
    // reported as failed rather than as a successful run of fewer Tests.
    result.diagnostics.splice(0..0, unresolved);
    Ok(result)
}

/// DS-1374/DS-1382/DS-1383: Evidence is generated only when every declared
/// target resolves to exactly one Source Target. A Test with a target that
/// resolves to none or to several is not executed, and one error per
/// failing target is returned instead (E-SCAN-004, or E-SCAN-011 for a
/// SRC ID declared by several Source Targets), so `run` never writes
/// Evidence with a partial `hashes.targets` (#127).
fn unresolved_targets(entity: &TestEntity, scan: &ScanResult) -> Vec<Diagnostic> {
    entity
        .targets
        .iter()
        .filter_map(|target| {
            let matches = matching_sources(target, scan).len();
            if matches == 1 {
                return None;
            }
            let (code, spelling) = match target {
                TargetRef::Locator(locator) => ("E-SCAN-004", locator.value.as_str()),
                // A SRC ID declared by several Source Targets is a SRC ID
                // collision, which scan already reports as E-SCAN-011 on
                // the referring Test; it is not re-labelled E-SCAN-004.
                TargetRef::SrcId(src_id) if matches > 1 => ("E-SCAN-011", src_id.as_str()),
                TargetRef::SrcId(src_id) => ("E-SCAN-004", src_id.as_str()),
            };
            let reason = if matches == 0 { "no" } else { "more than one" };
            Some(
                Diagnostic::error(
                    code,
                    format!(
                        "test `{}` target `{spelling}` resolves to {reason} Source Target; the \
                         Test is not run and no Evidence is written",
                        entity.id
                    ),
                )
                .with_location(entity.location.clone()),
            )
        })
        .collect()
}

/// The Source Targets one declared target matches: by exact Locator, or by
/// the permanent SRC ID a Source Target declares.
fn matching_sources<'a>(target: &TargetRef, scan: &'a ScanResult) -> Vec<&'a SourceFunction> {
    scan.sources
        .iter()
        .filter(|source| match target {
            TargetRef::Locator(locator) => &source.locator == locator,
            TargetRef::SrcId(src_id) => source.src_id.as_ref() == Some(src_id),
        })
        .collect()
}

/// The `data` of `run` / `run_tests`.
pub fn data_json(result: &ExecutionResult, fast: bool) -> serde_json::Value {
    serde_json::json!({
        "evidence": result.evidence.len(),
        "evidence_ids": result.evidence.iter().map(|record| record.id.clone()).collect::<Vec<_>>(),
        "fast": fast,
    })
}
