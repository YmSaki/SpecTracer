//! Canonical Test query operations: `list`, `query` and `show`, shared
//! between `vtest test list / query / show` and the MCP tools `test_query`
//! and `test_get` (BD-344).
//!
//! `list` and `query` read the managed Test Entities in the order of the
//! test specification from `vtest-scan` and only filter them (BD-343,
//! DES-613). `show` takes its verification from the `verify` operation, its
//! judgment state from the effective-judgment derivation `verify` uses for
//! the pending section, and its Evidence state from the latest-Evidence
//! selection `verify` uses for `target_binding` (BD-345, BD-347, BD-348).
//! None of them writes anything (DS-1812).

use std::path::Path;

use serde_json::{json, Value};
use vtest_adapter_api::{wire::encode_test_json, AdapterRegistry};
use vtest_model::{AdapterId, ExitCode, Locator, SrcId, TargetRef, TestEntity};
use vtest_scan::{
    scan_project,
    test_spec::{ordered_tests, test_specification},
    LocatorKey, ScanResult, SourceIndex, TargetResolution,
};
use vtest_store::{
    decisions::{effective_set, EffectiveJudgment, JUDGMENT_KINDS},
    load_config, VerifyLayout,
};
use vtest_verify::{EntityScope, LatestEvidence};

use crate::envelope::{
    config_failure, envelope_json, failure_envelope_with_candidates, scan_failure, Failure,
};

/// Which Tests `list` returns (DS-1816).
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ListFilter {
    /// Every managed Test Entity (DS-1817).
    All,
    /// The managed Test Entities that declare this VO ID in `covers`
    /// (DS-1818). The VO record does not have to exist (DS-1819).
    Vo(String),
    /// The Test constructs without a management declaration (DS-1820).
    Unregistered,
}

fn scanned(root: &Path, registry: &AdapterRegistry) -> Result<ScanResult, Failure> {
    load_config(root).map_err(|error| config_failure(&error))?;
    scan_project(root, registry).map_err(|error| scan_failure(&error))
}

/// DS-1815: the result with every scan diagnostic; exit code 1 and
/// `ok: false` when one of them is an error.
fn completed(scan: &ScanResult, data: Value) -> (ExitCode, Value) {
    let has_errors = scan.has_errors();
    let exit = if has_errors {
        ExitCode::VerificationFailed
    } else {
        ExitCode::Ok
    };
    (exit, envelope_json(!has_errors, data, &scan.diagnostics))
}

fn tests_json<'a>(
    tests: impl IntoIterator<Item = &'a TestEntity>,
    registry: &AdapterRegistry,
) -> Value {
    Value::Array(
        tests
            .into_iter()
            .map(|test| encode_test_json(test, registry))
            .collect(),
    )
}

/// `vtest test list` and the `vo` / `unregistered` forms of `test_query`.
pub fn list(root: &Path, registry: &AdapterRegistry, filter: &ListFilter) -> (ExitCode, Value) {
    let scan = match scanned(root, registry) {
        Ok(scan) => scan,
        Err(failure) => return failure.result(),
    };
    let data = match filter {
        ListFilter::All => json!({"tests": tests_json(ordered_tests(&scan), registry)}),
        ListFilter::Vo(vo) => json!({
            "tests": tests_json(
                ordered_tests(&scan)
                    .into_iter()
                    .filter(|test| test.covers.iter().any(|covered| covered.as_str() == vo)),
                registry,
            )
        }),
        ListFilter::Unregistered => json!({"unmanaged": test_specification(&scan).unmanaged}),
    };
    completed(&scan, data)
}

/// DS-1822: `SRC-…` is a SRC ID reference; anything else is
/// `<adapter ID>::<locator value>`, split at the first `::`.
fn parse_source(source: &str) -> Result<TargetRef, String> {
    if source.starts_with("SRC-") {
        return Ok(TargetRef::SrcId(SrcId::new(source)));
    }
    match source.split_once("::") {
        Some((adapter, value)) => Ok(TargetRef::Locator(Locator {
            adapter: AdapterId::new(adapter),
            value: value.to_owned(),
        })),
        None => Err(format!(
            "--source `{source}` is neither a SRC ID (`SRC-…`) nor `<adapter ID>::<locator>`"
        )),
    }
}

fn canonical(key: &LocatorKey) -> String {
    format!("{}::{}", key.0, key.1)
}

fn source_rejection(message: String, candidates: &[LocatorKey]) -> (ExitCode, Value) {
    let candidates: Vec<String> = candidates.iter().map(canonical).collect();
    (
        ExitCode::Usage,
        failure_envelope_with_candidates(
            "E-OP-001",
            message,
            (!candidates.is_empty()).then_some(candidates.as_slice()),
        ),
    )
}

/// `vtest test query --source` and the `source` form of `test_query`.
pub fn query(root: &Path, registry: &AdapterRegistry, source: &str) -> (ExitCode, Value) {
    let reference = match parse_source(source) {
        Ok(reference) => reference,
        Err(message) => return source_rejection(message, &[]),
    };
    let scan = match scanned(root, registry) {
        Ok(scan) => scan,
        Err(failure) => return failure.result(),
    };
    if let TargetRef::Locator(locator) = &reference {
        if registry.get(locator.adapter.as_str()).is_none() {
            return source_rejection(
                format!(
                    "--source `{source}` names adapter `{}`, which is not registered",
                    locator.adapter
                ),
                &[],
            );
        }
    }
    // DES-614: the reference and each Test's targets are resolved by the
    // index the scan resolves targets with, and compared as canonical
    // Source Targets, not as spellings.
    let index = SourceIndex::new(&scan.sources);
    let wanted = match index.resolve(&reference) {
        TargetResolution::Resolved(key) => key,
        TargetResolution::NotFound => {
            return source_rejection(
                format!("--source `{source}` resolves to no Source Target"),
                &[],
            )
        }
        TargetResolution::Ambiguous(candidates) => {
            return source_rejection(
                format!("--source `{source}` does not resolve to exactly one Source Target"),
                &candidates,
            )
        }
    };
    let tests = ordered_tests(&scan).into_iter().filter(|test| {
        test.targets.iter().any(|target| {
            matches!(index.resolve(target), TargetResolution::Resolved(key) if key == wanted)
        })
    });
    let data = json!({"tests": tests_json(tests, registry)});
    completed(&scan, data)
}

/// DS-1849 to DS-1853: one element per judgment kind that has a record of
/// this Test, in ascending kind order.
fn judgments(
    layout: &VerifyLayout,
    scan: &ScanResult,
    test_id: &str,
    decisions: &[vtest_store::decisions::DecisionRecord],
) -> Vec<Value> {
    let mut kinds: Vec<&str> = JUDGMENT_KINDS
        .iter()
        .copied()
        .filter(|kind| {
            decisions
                .iter()
                .any(|record| record.subject == test_id && record.judgment_kind == *kind)
        })
        .collect();
    kinds.sort_unstable();
    if kinds.is_empty() {
        return Vec::new();
    }
    // A Test whose binding cannot be computed now has no valid record
    // (DS-724), exactly as the pending section treats it.
    let binding = crate::judgment::current_binding(
        layout,
        scan,
        &crate::judgment::Subject::Test(test_id.to_owned()),
    )
    .ok();
    kinds
        .into_iter()
        .map(|kind| {
            let set = effective_set(
                decisions,
                test_id,
                kind,
                binding
                    .as_ref()
                    .map(|binding| (&binding.subject_hash, binding.dependencies.as_slice())),
            );
            let decision = match set.judgment {
                EffectiveJudgment::Decided(value) => Value::String(value),
                EffectiveJudgment::Unknown => Value::Null,
            };
            json!({
                "judgment_kind": kind,
                "decision": decision,
                "effective": set.effective,
                "superseded": set.superseded,
                "invalid": set.invalid,
            })
        })
        .collect()
}

/// DS-1854 / DS-1855: the latest Evidence record `verify` selects, valid or
/// not; an older record never stands in.
fn evidence(root: &Path, registry: &AdapterRegistry, test_id: &str) -> Value {
    match vtest_verify::latest_evidence(root, registry, [test_id]).remove(test_id) {
        None => Value::Null,
        Some(LatestEvidence::Valid(record)) => json!({
            "id": record.id,
            "result": record.result,
            "executed_at": record.executed_at,
            "valid": true,
        }),
        Some(LatestEvidence::Invalid { id, .. }) => json!({
            "id": id,
            "result": null,
            "executed_at": null,
            "valid": false,
        }),
    }
}

/// `vtest test show <Test ID>` and `test_get`.
pub fn show(root: &Path, registry: &AdapterRegistry, test_id: &str) -> (ExitCode, Value) {
    let config = match crate::verify::load_verify_config(root) {
        Ok(config) => config,
        Err(error) => return error.failure().result(),
    };
    let scan = match scan_project(root, registry) {
        Ok(scan) => scan,
        Err(error) => return scan_failure(&error).result(),
    };
    // DS-1826: the Test ID must name exactly one managed Test Entity.
    let holders = scan
        .tests
        .iter()
        .filter(|test| test.id.as_str() == test_id)
        .count();
    if holders == 0 {
        // DS-1192: the near Test IDs, by the rule every rejection of an
        // unknown Test ID uses (DES-405), each once.
        let mut candidates = vtest_scan::test_id_candidates(&scan, test_id);
        candidates.dedup();
        return (
            ExitCode::Usage,
            failure_envelope_with_candidates(
                "E-OP-001",
                format!("no Test with id '{test_id}' exists"),
                (!candidates.is_empty()).then_some(candidates.as_slice()),
            ),
        );
    }
    if holders > 1 {
        return Failure::usage(
            "E-SCAN-002",
            format!("Test ID '{test_id}' is declared by {holders} Tests"),
        )
        .result();
    }
    // DES-616: the verification, `ok`, the diagnostics and the exit code are
    // the ones the verify operation returns for `--test <Test ID>`.
    let evaluated = match crate::verify::evaluate_scanned(
        root,
        &config,
        scan,
        None,
        Some(EntityScope::Test(test_id.to_owned())),
        None,
        false,
        registry,
    ) {
        Ok(evaluated) => evaluated,
        Err(error) => return error.failure().result(),
    };
    let Some(test) = evaluated
        .scan
        .tests
        .iter()
        .find(|test| test.id.as_str() == test_id)
    else {
        return Failure::internal(format!("Test '{test_id}' left the scan result")).result();
    };
    let layout = VerifyLayout::new(root);
    // The verify operation has already reported a store it cannot read; a
    // record that breaks the schema leaves no judgment applied there, and
    // none shown here.
    let decisions = vtest_store::decisions::read_all_decisions(&layout).unwrap_or_default();
    let data = json!({
        "test": encode_test_json(test, registry),
        "verification": evaluated.data,
        "judgments": judgments(&layout, &evaluated.scan, test_id, &decisions),
        "evidence": evidence(root, registry, test_id),
    });
    (
        evaluated.exit,
        envelope_json(evaluated.exit == ExitCode::Ok, data, &evaluated.diagnostics),
    )
}
