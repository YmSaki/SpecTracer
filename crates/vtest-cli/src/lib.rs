//! Minimal canonical v0.1 CLI: `init`, `scan`, `verify`, `doctor`.
//!
//! この slice の CLI は正典 v0.1 モデルの上でだけ動く。前身モデルの
//! サブコマンド（`spec` / `req` / `audit` / `vo` / `test` / `run` /
//! `approval` / `mcp` / `report`）はこの版から外してある — それらは
//! `CheckValue` / `CheckItem` / `audits/` を前提としており、正典モデル上で
//! 意味を持たない（SPEC-400「旧モデルの12項目…は検査として存在しない」）。
//! 撤去そのものは移行チェーンの最終段（旧系撤去）の仕事であり、ここでは
//! `verify` を動かすために必要な範囲だけを先行して落としている。

use std::path::{Path, PathBuf};

use clap::{Parser, Subcommand};
use vtest_model::{Diagnostic, ExitCode, JsonEnvelope, VerificationCheck, VerificationState};
use vtest_scan::{scan_project, ScanResult};
use vtest_store::{init_project, load_config, GateConfig, ProjectConfig};
use vtest_verify::{
    check_name, parse_check, verify_project, CheckOutcome, EntityScope, TreeNode, VerifyOutcome,
};

#[derive(Parser, Debug)]
#[command(name = "vtest", version, about = "Specification traceability verifier")]
pub struct Cli {
    /// Project root, or a path below a project containing `.verify/`.
    #[arg(long, global = true, default_value = ".")]
    pub project: PathBuf,

    /// Emit machine-readable JSON instead of human-readable text.
    #[arg(long, global = true, value_enum, default_value_t = OutputFormat::Text)]
    pub format: OutputFormat,

    /// Suppress all output; communicate through the exit code only.
    #[arg(long, global = true)]
    pub quiet: bool,

    #[command(subcommand)]
    pub command: Command,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, clap::ValueEnum)]
pub enum OutputFormat {
    Text,
    Json,
}

#[derive(Subcommand, Debug)]
pub enum Command {
    /// Create the `.verify/` canonical directory layout.
    Init {
        #[arg(long)]
        name: Option<String>,
    },
    /// Scan the repository and report consistency diagnostics.
    Scan,
    /// Validate configuration and adapter preconditions.
    Doctor,
    /// Aggregate the four canonical checks and return OK / NG.
    ///
    /// SPEC-398「`vtest verify` は集約を実行し、`OK` / `NG` を返す」。
    Verify {
        /// Check axis: a subset of the fixed four checks (DS-1104).
        /// 省略時は固定4検査（DS-1106）。
        #[arg(long, value_delimiter = ',')]
        items: Vec<String>,

        /// Entity axis: a DOC / VO / Test subtree (DS-1104). At most one.
        #[arg(long)]
        doc: Option<String>,
        #[arg(long)]
        vo: Option<String>,
        #[arg(long)]
        test: Option<String>,

        /// Phase-gate evaluation (DS-1115).
        #[arg(long)]
        gate: Option<String>,

        /// DS-1117「`--summary` は総合 `OK` / `NG` と非 `PASS` 件数のみを
        /// 出力する」。
        #[arg(long)]
        summary: bool,
    },
}

pub fn run(cli: Cli) -> ExitCode {
    match cli.command {
        Command::Init { name } => run_init(&cli.project, name.as_deref(), cli.format, cli.quiet),
        Command::Scan => run_scan(&cli.project, cli.format, cli.quiet),
        Command::Doctor => run_doctor(&cli.project, cli.format, cli.quiet),
        Command::Verify {
            items,
            doc,
            vo,
            test,
            gate,
            summary,
        } => run_verify(
            &cli.project,
            &items,
            doc,
            vo,
            test,
            gate.as_deref(),
            summary,
            cli.format,
            cli.quiet,
        ),
    }
}

// ---------------------------------------------------------------------------
// init / scan / doctor
// ---------------------------------------------------------------------------

fn run_init(project: &Path, name: Option<&str>, format: OutputFormat, quiet: bool) -> ExitCode {
    let root = absolute_path(project);
    let project_name = name
        .map(str::to_owned)
        .or_else(|| {
            root.file_name()
                .and_then(|value| value.to_str())
                .map(str::to_owned)
        })
        .unwrap_or_else(|| "project".to_owned());
    match init_project(&root, &project_name) {
        Ok(_) => {
            let envelope = JsonEnvelope::new(
                true,
                serde_json::json!({ "project": root, "initialized": true }),
                Vec::new(),
            );
            emit(format, quiet, &envelope, |_| {
                format!("initialised {}\n", root.display())
            });
            ExitCode::Ok
        }
        Err(error) => usage_failure(format, quiet, "E-CONFIG-001", &error.to_string()),
    }
}

fn run_scan(project: &Path, format: OutputFormat, quiet: bool) -> ExitCode {
    let root = match resolve_root(project, format, quiet) {
        Ok(root) => root,
        Err(code) => return code,
    };
    // config の拒否は操作拒否であって内部エラーではない。DS-935「`vtest scan`
    // / `vtest doctor`では、registry・config・adapter契約の検証…が
    // E-ADAPTER-* / E-CONFIG-*で拒否された場合は2とする」。`scan_project` は
    // config 読み込み失敗を `ScanError::Store`（診断コードなし）へ畳むため、
    // ここで先に読んでおかないと終了コード 3（内部エラー）になってしまう。
    if let Err(error) = load_config(&root) {
        return usage_failure(format, quiet, "E-CONFIG-001", &error.to_string());
    }
    match scan_project(&root) {
        Ok(result) => {
            // DS-1304「`vtest scan` / `doctor`はerrorなしをexit 0にする」、
            // DS-936「scanが完了してrepository整合性のE-SCAN-*を報告した
            // 場合は1とする」。
            let has_errors = result.has_errors();
            let data = serde_json::json!({
                "files": result.summary.files,
                "tests": result.summary.tests,
                "sources": result.summary.sources,
                "discovered": result.discovered.len(),
            });
            let envelope = JsonEnvelope::new(!has_errors, data, result.diagnostics.clone());
            emit(format, quiet, &envelope, |envelope| {
                format!(
                    "scan: {} test(s), {} source(s), {} diagnostic(s)\n",
                    result.summary.tests,
                    result.summary.sources,
                    envelope.diagnostics.len()
                )
            });
            if has_errors {
                ExitCode::VerificationFailed
            } else {
                ExitCode::Ok
            }
        }
        // DS-935「`vtest scan` / `vtest doctor`では、registry・config・adapter
        // 契約の検証またはadapter呼出しがE-ADAPTER-* / E-CONFIG-*で拒否された
        // 場合は2とする」。
        Err(error) => scan_error_exit(&error, format, quiet),
    }
}

fn run_doctor(project: &Path, format: OutputFormat, quiet: bool) -> ExitCode {
    let root = match resolve_root(project, format, quiet) {
        Ok(root) => root,
        Err(code) => return code,
    };
    // `doctor` は config と adapter 前提の検証（DS-935）。config の読み込みが
    // E-CONFIG-* で拒否されれば 2、scan が整合性 error を報告すれば 1。
    let config = match load_config(&root) {
        Ok(config) => config,
        Err(error) => return usage_failure(format, quiet, "E-CONFIG-001", &error.to_string()),
    };
    match scan_project(&root) {
        Ok(result) => {
            let has_errors = result.has_errors();
            let data = serde_json::json!({
                "project": config.project.name,
                "adapters": config.adapters.iter().map(|a| a.id.clone()).collect::<Vec<_>>(),
                "gates": config.gates.iter().map(|g| g.name.clone()).collect::<Vec<_>>(),
            });
            let envelope = JsonEnvelope::new(!has_errors, data, result.diagnostics.clone());
            emit(format, quiet, &envelope, |envelope| {
                format!(
                    "doctor: project {}, {} adapter(s), {} diagnostic(s)\n",
                    config.project.name,
                    config.adapters.len(),
                    envelope.diagnostics.len()
                )
            });
            if has_errors {
                ExitCode::VerificationFailed
            } else {
                ExitCode::Ok
            }
        }
        Err(error) => scan_error_exit(&error, format, quiet),
    }
}

fn scan_error_exit(error: &vtest_scan::ScanError, format: OutputFormat, quiet: bool) -> ExitCode {
    let code = error.code().unwrap_or("E-CORE-001");
    let exit = if error.code().is_some() {
        ExitCode::Usage
    } else {
        ExitCode::Internal
    };
    emit_failure(format, quiet, code, &error.to_string());
    exit
}

// ---------------------------------------------------------------------------
// verify
// ---------------------------------------------------------------------------

/// The `verify` JSON payload.
///
/// 正典が逐語で名指しする最上位 field は `scope` だけである（DS-947 /
/// DS-1114）。それ以外の envelope 形は正典に定義が無く（stopped_on として
/// 開示する）、ここでは既存の [`JsonEnvelope`] の `ok` / `data` /
/// `diagnostics` を踏襲し、`data` の中に `scope` を最上位 field として置く。
#[derive(serde::Serialize)]
struct VerifyData<'a> {
    scope: &'a vtest_verify::ScopeReport,
    /// 集約代表値（DS-870）。総合 OK/NG やゲート充足とは別の field として
    /// 必ず出す — 検証状態とゲート充足は別軸である。
    state: &'static str,
    result: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    gate: Option<GateEvaluation>,
    #[serde(skip_serializing_if = "<[_]>::is_empty")]
    structural: &'a [CheckOutcome],
    /// 評価地点を1件も持たなかった検査（DS-840 / DS-252 / DS-253）。
    /// 空でなければ、この結果は完全検証 OK ではない。
    #[serde(skip_serializing_if = "<[_]>::is_empty")]
    unevaluated: &'a [CheckOutcome],
    #[serde(skip_serializing_if = "<[_]>::is_empty")]
    tree: &'a [TreeNode],
    non_pass: usize,
}

#[derive(serde::Serialize)]
struct GateEvaluation {
    name: String,
    /// DS-869「検証条件の充足判定は、`require.verification`の値と、要求scopeの
    /// 集約代表値との完全一致でのみ充足する」。
    required_verification: String,
    verification_satisfied: bool,
    approvals_satisfied: bool,
    satisfied: bool,
    reasons: Vec<String>,
}

#[allow(clippy::too_many_arguments)]
fn run_verify(
    project: &Path,
    items: &[String],
    doc: Option<String>,
    vo: Option<String>,
    test: Option<String>,
    gate: Option<&str>,
    summary: bool,
    format: OutputFormat,
    quiet: bool,
) -> ExitCode {
    let root = match resolve_root(project, format, quiet) {
        Ok(root) => root,
        Err(code) => return code,
    };

    // DS-1104 のエンティティ軸は排他。複数指定は操作拒否（exit 2）。
    let entity = match entity_scope(doc, vo, test) {
        Ok(entity) => entity,
        Err(message) => return usage_failure(format, quiet, "E-OP-001", &message),
    };

    let requested = match parse_items(items) {
        Ok(requested) => requested,
        Err(message) => return usage_failure(format, quiet, "E-OP-001", &message),
    };

    let config = match load_config(&root) {
        Ok(config) => config,
        Err(error) => return usage_failure(format, quiet, "E-CONFIG-001", &error.to_string()),
    };

    // DS-1116「config の `gates` に同名の定義が無ければ E-CONFIG-002・
    // 終了コード 2 で拒否し、検証を実行しない」。scan より前に解決する。
    let gate_config = match resolve_gate(&config, gate) {
        Ok(gate_config) => gate_config,
        Err(message) => return usage_failure(format, quiet, "E-CONFIG-002", &message),
    };

    let scan: ScanResult = match scan_project(&root) {
        Ok(scan) => scan,
        Err(error) => return scan_error_exit(&error, format, quiet),
    };

    let outcome = verify_project(&root, &scan, requested.as_deref(), entity);
    let gate_evaluation = gate_config.map(|config| evaluate_gate(config, &outcome));
    let non_pass = outcome
        .all_outcomes()
        .iter()
        .filter(|check| check.state != VerificationState::Pass)
        .count();

    // DS-1117「`--summary` は総合 `OK` / `NG` と非 `PASS` 件数のみを出力する」。
    // 逐語どおり、per-check の内訳（構造検査・未評価検査・ツリー）はすべて
    // 落とす。`scope` だけは残す — DS-1114「`--format json` では同じ内容を
    // 最上位 field `scope` として返し、完全検証の場合も省略しない」が、
    // 出力形態を問わない無条件の義務として課している。
    let data = VerifyData {
        scope: &outcome.scope,
        state: state_name(outcome.state),
        result: if outcome.ok { "OK" } else { "NG" },
        gate: gate_evaluation,
        structural: if summary { &[] } else { &outcome.structural },
        unevaluated: if summary { &[] } else { &outcome.unevaluated },
        tree: if summary { &[] } else { &outcome.tree },
        non_pass,
    };

    // 終了コード。DS-931「`--gate <name>`を指定した`vtest verify` /
    // `vtest report`では、0と1をゲート充足で決める」、DS-932。
    // DS-933「`require.verification`に`PASS`以外を定義したゲートでは、集約
    // 代表値が要求値と一致して充足した実行が0になり、この場合に総合がNGで
    // あることは0を妨げない」。
    let exit = match &data.gate {
        Some(evaluation) if evaluation.satisfied => ExitCode::Ok,
        Some(_) => ExitCode::VerificationFailed,
        None if outcome.ok => ExitCode::Ok,
        None => ExitCode::VerificationFailed,
    };

    // `ok` はこの実行が返す 0/1 と同義にする — ゲート指定時はゲート充足、
    // 非指定時は総合 OK。検証状態そのものは `state` field に常に別途出す。
    let envelope = JsonEnvelope::new(exit == ExitCode::Ok, data, scan.diagnostics.clone());
    emit(format, quiet, &envelope, render_verify_text);
    exit
}

fn entity_scope(
    doc: Option<String>,
    vo: Option<String>,
    test: Option<String>,
) -> Result<Option<EntityScope>, String> {
    let selected = [
        doc.map(EntityScope::Doc),
        vo.map(EntityScope::Vo),
        test.map(EntityScope::Test),
    ]
    .into_iter()
    .flatten()
    .collect::<Vec<_>>();
    match selected.len() {
        0 => Ok(None),
        1 => Ok(selected.into_iter().next()),
        _ => Err("verify accepts at most one of --doc, --vo, or --test".to_owned()),
    }
}

/// `--items` を検査軸へ解決する。未知の名前は黙って捨てず拒否する:
/// 捨てると要求 scope が黙って狭まり、DS-1113 の開示義務を破る。
fn parse_items(items: &[String]) -> Result<Option<Vec<VerificationCheck>>, String> {
    if items.is_empty() {
        return Ok(None);
    }
    let mut checks = Vec::new();
    for item in items {
        let name = item.trim();
        if name.is_empty() {
            continue;
        }
        let Some(check) = parse_check(name) else {
            return Err(format!(
                "unknown check '{name}'; the fixed four are chain_integrity, \
                 orphan_detection, target_binding, oracle_presence"
            ));
        };
        if !checks.contains(&check) {
            checks.push(check);
        }
    }
    if checks.is_empty() {
        return Ok(None);
    }
    Ok(Some(checks))
}

/// DS-865「`--gate <name>`は`gates[].name`との大文字小文字を区別した完全一致で
/// 解決する」。DS-363「`--gate` を指定しない実行は、`gates` field自体の欠落と
/// 空listの影響を受けない」。
fn resolve_gate<'a>(
    config: &'a ProjectConfig,
    gate: Option<&str>,
) -> Result<Option<&'a GateConfig>, String> {
    let Some(name) = gate else {
        return Ok(None);
    };
    config
        .gates
        .iter()
        .find(|candidate| candidate.name == name)
        .map(Some)
        .ok_or_else(|| format!("no gate named '{name}' is defined in config.yaml"))
}

/// DS-864「`vtest verify --gate <name>`は、指定ゲートの対象scopeについて検証を
/// 実行し、(1) 検証結果が`require.verification`を満たすか、(2)
/// `require.approvals`の各ロールについて対象の実効承認状態が`approved`である
/// か、を評価して満否と根拠を提示する」。
fn evaluate_gate(config: &GateConfig, outcome: &VerifyOutcome) -> GateEvaluation {
    // DS-869「…`require.verification`の値と、要求scopeの集約代表値との完全
    // 一致でのみ充足する」。DS-874「「要求値以上」「要求値より良い」といった
    // 比較解釈を採らず…」— したがって完全一致だけで判定する。
    let actual = state_name(outcome.state);
    let verification_satisfied = config.require.verification == actual;

    let mut reasons = Vec::new();
    if !verification_satisfied {
        reasons.push(format!(
            "aggregate representative state is {actual}, gate requires {}",
            config.require.verification
        ));
    }

    // 承認側。この slice には正典の実効承認状態（§3.5）の読み手が無い。
    // 「読めないから充足」は fail-open なので、要求ロールが1件でもあれば
    // 未充足として扱い、その旨を根拠に明示する。
    let approvals_satisfied = config.require.approvals.is_empty();
    if !approvals_satisfied {
        reasons.push(format!(
            "approval roles {:?} cannot be evaluated in this slice; treated as unsatisfied \
             (fail-closed)",
            config.require.approvals
        ));
    }

    GateEvaluation {
        name: config.name.clone(),
        required_verification: config.require.verification.clone(),
        verification_satisfied,
        approvals_satisfied,
        satisfied: verification_satisfied && approvals_satisfied,
        reasons,
    }
}

fn state_name(state: VerificationState) -> &'static str {
    match state {
        VerificationState::Pass => "PASS",
        VerificationState::Fail => "FAIL",
        VerificationState::Mismatch => "MISMATCH",
        VerificationState::NoEvidence => "NO_EVIDENCE",
        VerificationState::Unknown => "UNKNOWN",
    }
}

fn label_name(label: vtest_model::DiagnosticLabel) -> &'static str {
    match label {
        vtest_model::DiagnosticLabel::Missing => "MISSING",
        vtest_model::DiagnosticLabel::NotChecked => "NOT_CHECKED",
        vtest_model::DiagnosticLabel::NotExecuted => "NOT_EXECUTED",
        vtest_model::DiagnosticLabel::Stale => "STALE",
    }
}

/// DS-1118「`vtest verify` は状態列…と診断ラベル列…を分離して表示する」。
fn render_verify_text(envelope: &JsonEnvelope<VerifyData<'_>>) -> String {
    let data = &envelope.data;
    let mut out = String::new();

    // DS-1113「scope を限定した場合、出力冒頭に要求 scope と「scope 外は
    // 未検証」の旨を必ず表示する」。
    out.push_str(&format!(
        "Requested scope: {}\n",
        data.scope
            .requested_checks
            .iter()
            .map(|check| check_name(*check))
            .collect::<Vec<_>>()
            .join(", ")
    ));
    if let Some(entity) = &data.scope.entity {
        out.push_str(&format!("Entity scope: {}\n", entity.id()));
    }
    if data.scope.outside_scope_is_unverified {
        out.push_str("Anything outside the requested scope is UNVERIFIED, not PASS.\n");
    }

    if !data.structural.is_empty() {
        out.push_str("\nStructural checks:\n");
        for check in data.structural {
            out.push_str(&render_check(check, 2));
        }
    }

    if !data.unevaluated.is_empty() {
        out.push_str("\nChecks with no evaluation point (NOT verified, not PASS):\n");
        for check in data.unevaluated {
            out.push_str(&render_check(check, 2));
        }
    }

    if !data.tree.is_empty() {
        out.push('\n');
        for node in data.tree {
            render_node(node, 0, &mut out);
        }
    }

    if let Some(gate) = &data.gate {
        out.push_str(&format!(
            "\nGate {}: {} (verification {}, approvals {})\n",
            gate.name,
            if gate.satisfied {
                "SATISFIED"
            } else {
                "NOT SATISFIED"
            },
            if gate.verification_satisfied {
                "ok"
            } else {
                "no"
            },
            if gate.approvals_satisfied { "ok" } else { "no" },
        ));
        for reason in &gate.reasons {
            out.push_str(&format!("  - {reason}\n"));
        }
    }

    out.push_str(&format!(
        "\nAggregate state: {}\nNon-PASS checks: {}\nResult: {}\n",
        data.state, data.non_pass, data.result
    ));
    out
}

fn render_node(node: &TreeNode, depth: usize, out: &mut String) {
    let pad = "  ".repeat(depth);
    out.push_str(&format!(
        "{pad}{:?} {} [{}]\n",
        node.kind,
        node.id,
        state_name(node.state)
    ));
    for check in &node.checks {
        out.push_str(&render_check(check, (depth + 1) * 2));
    }
    for child in &node.children {
        render_node(child, depth + 1, out);
    }
}

/// 状態列と診断ラベル列を分けて描画する（DS-1118）。診断ラベルは代表値の
/// 順位に用いず併記するだけである（DS-1119）。
fn render_check(check: &CheckOutcome, indent: usize) -> String {
    let pad = " ".repeat(indent);
    let labels = if check.labels.is_empty() {
        String::new()
    } else {
        format!(
            "  {}",
            check
                .labels
                .iter()
                .map(|label| format!("[{}]", label_name(*label)))
                .collect::<Vec<_>>()
                .join(" ")
        )
    };
    let mut line = format!(
        "{pad}{:<18} {:<12}{labels}\n",
        check_name(check.check),
        state_name(check.state)
    );
    for basis in &check.basis {
        line.push_str(&format!("{pad}    {basis}\n"));
    }
    line
}

// ---------------------------------------------------------------------------
// Shared plumbing
// ---------------------------------------------------------------------------

fn emit<T: serde::Serialize>(
    format: OutputFormat,
    quiet: bool,
    envelope: &JsonEnvelope<T>,
    render_text: impl FnOnce(&JsonEnvelope<T>) -> String,
) {
    if quiet {
        return;
    }
    match format {
        OutputFormat::Json => println!(
            "{}",
            serde_json::to_string_pretty(envelope).unwrap_or_else(|error| format!(
                "{{\"ok\":false,\"data\":null,\"diagnostics\":[\"{error}\"]}}"
            ))
        ),
        OutputFormat::Text => {
            print!("{}", render_text(envelope));
            for diagnostic in &envelope.diagnostics {
                println!("[{}] {}", diagnostic.code, diagnostic.message);
            }
        }
    }
}

fn emit_failure(format: OutputFormat, quiet: bool, code: &str, message: &str) {
    let envelope = JsonEnvelope::new(
        false,
        serde_json::Value::Null,
        vec![Diagnostic::error(code, message.to_owned())],
    );
    emit(format, quiet, &envelope, |_| String::new());
}

fn usage_failure(format: OutputFormat, quiet: bool, code: &str, message: &str) -> ExitCode {
    emit_failure(format, quiet, code, message);
    ExitCode::Usage
}

fn absolute_path(path: &Path) -> PathBuf {
    if path.is_absolute() {
        path.to_owned()
    } else {
        std::env::current_dir()
            .map(|cwd| cwd.join(path))
            .unwrap_or_else(|_| path.to_owned())
    }
}

/// Walk upwards looking for a `.verify/` directory, so the CLI works from a
/// subdirectory of the project.
fn resolve_root(project: &Path, format: OutputFormat, quiet: bool) -> Result<PathBuf, ExitCode> {
    let start = absolute_path(project);
    let mut current = start.as_path();
    loop {
        if current.join(".verify").is_dir() {
            return Ok(current.to_owned());
        }
        match current.parent() {
            Some(parent) => current = parent,
            None => {
                return Err(usage_failure(
                    format,
                    quiet,
                    "E-OP-001",
                    &format!(
                        "could not find a .verify directory at or above {}",
                        start.display()
                    ),
                ));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn items_reject_an_unknown_check_rather_than_narrowing_the_scope() {
        // 旧12項目名は検査として存在しない（SPEC-400）。黙って捨てると
        // 要求 scope が狭まったことが開示されない（DS-1113）。
        assert!(parse_items(&["spec_coverage".to_owned()]).is_err());
        assert!(parse_items(&["chain_integrity".to_owned()]).is_ok());
    }

    #[test]
    fn omitted_items_mean_the_fixed_four_not_a_config_subset() {
        // DS-1106 / DS-1107: 省略は「固定4検査」であり、config 値の部分集合
        // ではない。`None` が verify 側の「固定4検査」を意味する。
        assert!(parse_items(&[]).expect("empty is valid").is_none());
    }

    #[test]
    fn entity_axis_is_exclusive() {
        // DS-1104: エンティティ軸は DOC / VO / Test のいずれか一つ。
        assert!(entity_scope(Some("D".to_owned()), Some("V".to_owned()), None).is_err());
        assert!(entity_scope(None, Some("V".to_owned()), None).is_ok());
        assert!(entity_scope(None, None, None)
            .expect("none is valid")
            .is_none());
    }
}
