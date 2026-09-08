//! `vtest run`: execute a Test and record Evidence.
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
use vtest_store::VerifyLayout;

#[derive(Debug, thiserror::Error)]
pub enum RunOpError {
    #[error("no Test with id '{0}' was discovered by scan")]
    UnknownTestId(String),
    #[error(transparent)]
    Execution(#[from] ExecutionError),
}

/// Builds one `RunnableTest` per requested `TestEntity`. `test_ids`, when
/// non-empty, selects a subset by id (E-OP-001 usage error if any id is not
/// found — an unresolved filter must not silently narrow to "whatever
/// matched", mirroring `vtest verify`'s `--items` rejection of unknown
/// names for the same reason). An empty `test_ids` runs every Test the scan
/// materialized.
pub fn resolve_runnables(
    scan: &ScanResult,
    test_ids: &[String],
) -> Result<Vec<TestEntity>, RunOpError> {
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
    test_ids: &[String],
    fast: bool,
) -> Result<ExecutionResult, RunOpError> {
    let entities = resolve_runnables(scan, test_ids)?;
    let runnables = entities
        .iter()
        .map(|entity| build_runnable(entity, scan))
        .collect::<Vec<_>>();
    Ok(run_tests(root, layout, &runnables, fast)?)
}
