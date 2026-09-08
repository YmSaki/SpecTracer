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
//! build each `RunnableTest`'s `target_hashes`/`target_locator`, following
//! the pattern `crates/vtest-cli/tests/evidence_e2e.rs` already exercises
//! against the library entry points directly (that test file's own comment
//! disclosed no `vtest run`/`vtest exec` CLI surface existed yet).

use std::path::Path;

use vtest_exec::{run_tests, ExecutionError, ExecutionResult, RunnableTest};
use vtest_model::{TargetRef, TestEntity, TestId};
use vtest_scan::ScanResult;
use vtest_store::{approval::read_all_vos, StoreError, VerifyLayout};
use vtest_verify::vo_subtree_ids;

#[derive(Debug, thiserror::Error)]
pub enum RunOpError {
    #[error("no Test with id '{0}' was discovered by scan")]
    UnknownTestId(String),
    #[error("no VO with id '{0}' exists")]
    UnknownVoId(String),
    #[error(transparent)]
    Execution(#[from] ExecutionError),
    #[error("store error: {0}")]
    Store(#[from] StoreError),
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
/// `Locator` targets (if any) against the scan's discovered Source
/// Functions to obtain their current content hashes. A Test with zero
/// targets (DS-1673 permits this) yields empty `target_hashes` and
/// `target_locator: None` — `run_tests` already handles that shape (see its
/// `unavailable_target_coverage`/`unknown_target_coverage` fallbacks); this
/// function does not itself decide what that implies for verification.
fn build_runnable(entity: &TestEntity, scan: &ScanResult) -> RunnableTest {
    let mut target_hashes = Vec::new();
    let mut target_locator = None;
    for target in &entity.targets {
        let TargetRef::Locator(locator) = target else {
            continue;
        };
        if let Some(source_fn) = scan
            .sources
            .iter()
            .find(|source| &source.locator == locator)
        {
            target_hashes.push(source_fn.content_hash.clone());
            if target_locator.is_none() {
                target_locator = Some(locator.clone());
            }
        }
    }
    RunnableTest {
        entity: entity.clone(),
        target_hashes,
        target_locator,
    }
}

pub fn run(
    root: &Path,
    layout: &VerifyLayout,
    scan: &ScanResult,
    target: &RunTarget,
    fast: bool,
) -> Result<ExecutionResult, RunOpError> {
    let entities = resolve_runnables(scan, layout, target)?;
    let runnables = entities
        .iter()
        .map(|entity| build_runnable(entity, scan))
        .collect::<Vec<_>>();
    Ok(run_tests(root, layout, &runnables, fast)?)
}
