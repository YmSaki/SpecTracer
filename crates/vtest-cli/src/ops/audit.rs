//! `vtest audit static` and MCP `audit_static` (SPEC-297/SPEC-298/SPEC-387,
//! DS-1208/DS-1209/DS-1227).
//!
//! The analysis runs on request through each Test's adapter Static Analysis
//! capability (resolved via the registry, BD-007) and returns rule-by-rule
//! verdicts (`FAIL` / `UNKNOWN` / `NO_VIOLATION`) with the evidence span.
//! The result is written to stdout (by the caller) and to
//! `.verify/cache/audit-static.json`. It is a recomputed derivation: no
//! canonical record is created and no audit record id is returned
//! (DS-1209/DS-1227), and it is not a judgment record (SPEC-298).

use std::path::Path;

use vtest_adapter_api::{AdapterRegistry, StaticAnalysisInput};
use vtest_model::{Diagnostic, TestEntity};
use vtest_scan::ScanResult;
use vtest_store::VerifyLayout;

#[derive(Debug, thiserror::Error)]
pub enum AuditOpError {
    /// E-OP-001: neither or both of `--test` / `--all`, or an unknown Test.
    #[error("{0}")]
    Usage(String),
    /// E-ADAPTER-004 (DS-528/DS-979/DS-1580): a selected Test's adapter has
    /// no Static Analysis capability, which `audit static` requires. The
    /// operation stops before analyzing anything and writes no cache.
    #[error("{0}")]
    MissingCapability(String),
    /// Writing the cache file failed: an internal error.
    #[error("cannot write the audit cache at {path}: {source}")]
    Cache {
        path: String,
        #[source]
        source: std::io::Error,
    },
}

impl AuditOpError {
    /// The diagnostic code the CLI and MCP report for this rejection.
    pub fn code(&self) -> &'static str {
        match self {
            Self::Usage(_) => "E-OP-001",
            Self::MissingCapability(_) => "E-ADAPTER-004",
            Self::Cache { .. } => vtest_model::INTERNAL_ERROR_CODE,
        }
    }

    /// The exit code: 2 for a rejected request (DS-979), 3 for a failure of
    /// the tool itself.
    pub fn exit_code(&self) -> vtest_model::ExitCode {
        match self {
            Self::Usage(_) | Self::MissingCapability(_) => vtest_model::ExitCode::Usage,
            Self::Cache { .. } => vtest_model::ExitCode::Internal,
        }
    }
}

#[derive(Debug, serde::Serialize)]
pub struct Span {
    pub path: String,
    pub byte_start: u64,
    pub byte_end: u64,
}

#[derive(Debug, serde::Serialize)]
pub struct RuleResult {
    pub rule: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target: Option<String>,
    pub verdict: &'static str,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub reason: String,
    pub span: Span,
}

#[derive(Debug, serde::Serialize)]
pub struct TestAudit {
    pub test: String,
    pub adapter: String,
    pub rules: Vec<RuleResult>,
    /// Why no rule verdicts are listed, when that is the case.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub not_analyzed: Option<String>,
}

#[derive(Debug, serde::Serialize)]
pub struct StaticAuditData {
    pub tests: Vec<TestAudit>,
    /// Where the derived copy was written (relative to the project root).
    pub cache: String,
}

pub const CACHE_FILE: &str = "audit-static.json";

fn select<'a>(
    scan: &'a ScanResult,
    test: Option<&str>,
    all: bool,
) -> Result<Vec<&'a TestEntity>, AuditOpError> {
    match (test, all) {
        (Some(_), true) | (None, false) => Err(AuditOpError::Usage(
            "audit static takes exactly one of --test <ID> or --all".to_owned(),
        )),
        (None, true) => Ok(scan.tests.iter().collect()),
        (Some(id), false) => scan
            .tests
            .iter()
            .find(|candidate| candidate.id.as_str() == id)
            .map(|found| vec![found])
            .ok_or_else(|| AuditOpError::Usage(format!("no Test with id '{id}' exists"))),
    }
}

/// DS-528/DS-979/DS-1580: `audit static` is an explicit operation whose
/// required capability is Static Analysis. Every selected Test's adapter is
/// checked before any Test is analyzed, so a missing capability stops the
/// whole operation (E-ADAPTER-004) instead of being reported per Test as
/// `verify` does (W-ADAPTER-101, DS-925).
fn require_static_analysis(
    selected: &[&TestEntity],
    registry: &AdapterRegistry,
) -> Result<(), AuditOpError> {
    let missing: std::collections::BTreeSet<&str> = selected
        .iter()
        .map(|test| test.execution.adapter.as_str())
        .filter(|adapter| registry.static_analysis(adapter).is_none())
        .collect();
    if missing.is_empty() {
        return Ok(());
    }
    let tests: Vec<&str> = selected
        .iter()
        .filter(|test| missing.contains(test.execution.adapter.as_str()))
        .map(|test| test.id.as_str())
        .collect();
    Err(AuditOpError::MissingCapability(format!(
        "audit static requires the Static Analysis capability, which adapter(s) {} do not \
         provide (Tests: {}); nothing was analyzed or written",
        missing.into_iter().collect::<Vec<_>>().join(", "),
        tests.join(", ")
    )))
}

fn audit_one(
    root: &Path,
    test: &TestEntity,
    analyzer: &dyn vtest_adapter_api::StaticAnalysisAdapter,
    assertion_macros: &std::collections::BTreeMap<String, Vec<String>>,
) -> TestAudit {
    let adapter_id = test.execution.adapter.as_str();
    let mut audit = TestAudit {
        test: test.id.as_str().to_owned(),
        adapter: adapter_id.to_owned(),
        rules: Vec::new(),
        not_analyzed: None,
    };
    let Some(text) = vtest_verify::read_construct_text(root, &test.location) else {
        audit.not_analyzed = Some("the Test construct's source bytes could not be read".to_owned());
        return audit;
    };
    let extra = assertion_macros
        .get(adapter_id)
        .cloned()
        .unwrap_or_default();
    let verdicts = analyzer.audit_rules(StaticAnalysisInput {
        test,
        construct_text: &text,
        content_hash: &test.content_hash,
        extra_assertion_macros: &extra,
    });
    if verdicts.is_empty() {
        audit.not_analyzed = Some("adapter reports no rule-level verdicts".to_owned());
    }
    audit.rules = verdicts
        .into_iter()
        .map(|verdict| RuleResult {
            rule: verdict.rule,
            target: verdict.target,
            verdict: verdict.outcome.as_str(),
            reason: verdict.reason,
            span: Span {
                path: test.location.path.as_str().to_owned(),
                byte_start: test.location.byte_range.start,
                byte_end: test.location.byte_range.end,
            },
        })
        .collect();
    audit
}

/// Runs the static audit for the selected Tests and writes the derived copy
/// to `.verify/cache/`.
pub fn audit_static(
    root: &Path,
    scan: &ScanResult,
    registry: &AdapterRegistry,
    test: Option<&str>,
    all: bool,
) -> Result<(StaticAuditData, Vec<Diagnostic>), AuditOpError> {
    let selected = select(scan, test, all)?;
    require_static_analysis(&selected, registry)?;
    let assertion_macros = vtest_store::load_config(root)
        .map(|config| {
            config
                .adapters
                .into_iter()
                .map(|adapter| (adapter.id, adapter.scan.assertion_macros))
                .collect()
        })
        .unwrap_or_default();
    let diagnostics = Vec::new();
    let tests = selected
        .into_iter()
        .map(|test| {
            let analyzer = registry
                .static_analysis(test.execution.adapter.as_str())
                .expect("checked by require_static_analysis");
            audit_one(root, test, analyzer, &assertion_macros)
        })
        .collect();
    let layout = VerifyLayout::new(root);
    let cache_path = layout.cache_dir().join(CACHE_FILE);
    let data = StaticAuditData {
        tests,
        cache: format!(".verify/cache/{CACHE_FILE}"),
    };
    let json = serde_json::to_string_pretty(&data).unwrap_or_default();
    let write = std::fs::create_dir_all(layout.cache_dir())
        .and_then(|()| std::fs::write(&cache_path, json));
    if let Err(source) = write {
        return Err(AuditOpError::Cache {
            path: cache_path.display().to_string(),
            source,
        });
    }
    Ok((data, diagnostics))
}
