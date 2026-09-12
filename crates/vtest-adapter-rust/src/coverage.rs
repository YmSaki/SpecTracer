//! `rust-cargo` `CoverageAdapter`（詳細設計 v0.1 本冊 §5.2「adapter capability
//! は…`CoverageAdapter`に分割する」BD-197、BD-220「`rust-cargo`
//! CoverageAdapterは`cargo-llvm-cov`を使用する」、BD-222「coverageは独立した
//! `CoverageAdapter` capabilityとして扱う」）。
//!
//! `cargo llvm-cov --version` の直接起動、llvm-cov JSON（`data[].functions[]`
//! の`name`/`filenames`/`count`/`regions`）の解析、Rust symbol の demangle、
//! opaque locator value の `path`/`item_path` への分解は、この crate が
//! `syn`、`quote`、`rustc-demangle`、Cargo commandを直接所有する側であり
//! （crate冒頭コメント、BD-114）、`vtest-exec`（core）はこの module の
//! `RustCargoCoverageAdapter` が返す [`vtest_adapter_api::
//! CoverageTargetMeasurement`]（DS-832の3値判定済み）を`target_coverage`へ
//! 写すだけになる。

use std::{path::Path, process::Command};

use vtest_adapter_api::{CoverageAdapter, CoverageTargetMeasurement};
use vtest_model::{Locator, TargetCoverageResult};

use crate::ADAPTER_ID;

/// The rust-cargo `CoverageAdapter`. Availability probing, llvm-cov JSON
/// parsing, and Rust symbol demangling all live here; `vtest-exec` only
/// records the method name and per-target measurements this returns.
#[derive(Clone, Copy, Debug, Default)]
pub struct RustCargoCoverageAdapter;

impl RustCargoCoverageAdapter {
    pub const fn new() -> Self {
        Self
    }
}

/// BD-220 の `cargo-llvm-cov` を指す method 名。`vtest-exec` はこの文字列を
/// `target_coverage.method` へそのまま書き込むだけで、自身では選ばない。
const COVERAGE_METHOD: &str = "llvm-cov";

impl CoverageAdapter for RustCargoCoverageAdapter {
    fn id(&self) -> &'static str {
        ADAPTER_ID
    }

    fn method(&self) -> &'static str {
        COVERAGE_METHOD
    }

    fn availability(&self, root: &Path) -> Result<(), String> {
        Command::new("cargo")
            .current_dir(root)
            .args(["llvm-cov", "--version"])
            .output()
            .ok()
            .filter(|output| output.status.success())
            .map(|_| ())
            .ok_or_else(|| {
                "cargo-llvm-cov is unavailable; target_coverage is NOT_CHECKED".to_owned()
            })
    }

    fn measure(
        &self,
        coverage_output_path: &Path,
        targets: &[Locator],
    ) -> Vec<CoverageTargetMeasurement> {
        let output = std::fs::read_to_string(coverage_output_path).ok();
        targets
            .iter()
            .map(|target| {
                let count = output
                    .as_deref()
                    .and_then(|output| llvm_cov_function_count(output, target));
                match count {
                    Some(count) => CoverageTargetMeasurement {
                        target: target.clone(),
                        result: if count > 0 {
                            TargetCoverageResult::Pass
                        } else {
                            TargetCoverageResult::Fail
                        },
                        count: Some(count),
                    },
                    // DS-832「関数不見当はUNKNOWNとする」: covers both an
                    // unreadable/unparsable coverage output file and an
                    // output that simply never mentions this target
                    // function — both are "could not locate the target
                    // function", not a measured absence of coverage.
                    None => CoverageTargetMeasurement {
                        target: target.clone(),
                        result: TargetCoverageResult::Unknown,
                        count: None,
                    },
                }
            })
            .collect()
    }
}

/// `target.value` は `rust-cargo` adapter が所有する opaque locator 文字列
/// （`<path>.rs::<item_path>`）。llvm-cov 出力との突き合わせに `path`/
/// `item_path` への分解が要る。分解は最初の `::` で区切るだけで、
/// `RustLocator::parse` の妥当性検査（`.rs`拡張子など）は行わない — この値
/// は常にこの adapter 自身の scanner が構築したものであり、構文は保証され
/// ている。
fn locator_parts(locator: &Locator) -> (&str, &str) {
    locator
        .value
        .split_once("::")
        .unwrap_or((locator.value.as_str(), ""))
}

fn llvm_cov_function_count(output: &str, target: &Locator) -> Option<u64> {
    let value = serde_json::from_str::<serde_json::Value>(output).ok()?;
    let data = value.get("data")?.as_array()?;
    let mut total = 0_u64;
    let mut matched = false;
    let (target_path, target_item_path) = locator_parts(target);
    for item in data {
        let Some(functions) = item.get("functions").and_then(serde_json::Value::as_array) else {
            continue;
        };
        for function in functions {
            let Some(name) = function.get("name").and_then(serde_json::Value::as_str) else {
                continue;
            };
            if !llvm_name_matches(name, target_item_path)
                || !llvm_filenames_match(function, target_path)
            {
                continue;
            }
            let function_count = function
                .get("count")
                .and_then(serde_json::Value::as_u64)
                .or_else(|| {
                    function
                        .get("regions")
                        .and_then(serde_json::Value::as_array)
                        .map(|regions| {
                            regions
                                .iter()
                                .filter_map(|region| region.as_array()?.get(4))
                                .filter_map(serde_json::Value::as_u64)
                                .max()
                                .unwrap_or(0)
                        })
                })?;
            matched = true;
            total = total.saturating_add(function_count);
        }
    }
    matched.then_some(total)
}

fn llvm_name_matches(name: &str, item_path: &str) -> bool {
    let demangled = format!("{:#}", rustc_demangle::demangle(name));
    if demangled == item_path || demangled.ends_with(&format!("::{item_path}")) {
        return true;
    }

    let generic_path = format!("{item_path}::<");
    demangled
        .strip_prefix(&generic_path)
        .or_else(|| {
            demangled
                .rsplit_once(&format!("::{generic_path}"))
                .map(|(_, arguments)| arguments)
        })
        .is_some_and(|arguments| !arguments.is_empty() && arguments.ends_with('>'))
}

fn llvm_filenames_match(function: &serde_json::Value, target_path: &str) -> bool {
    function
        .get("filenames")
        .and_then(serde_json::Value::as_array)
        .is_some_and(|filenames| {
            filenames.iter().any(|filename| {
                filename
                    .as_str()
                    .is_some_and(|filename| path_suffix_matches(filename, target_path))
            })
        })
}

fn path_suffix_matches(candidate: &str, expected: &str) -> bool {
    candidate
        .replace('\\', "/")
        .ends_with(&expected.replace('\\', "/"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use vtest_model::AdapterId;

    fn rust_locator(path: &str, item_path: &str) -> Locator {
        Locator {
            adapter: AdapterId::new(ADAPTER_ID),
            value: format!("{path}::{item_path}"),
        }
    }

    fn write_fixture(name: &str, contents: &str) -> std::path::PathBuf {
        let path = std::env::temp_dir().join(format!(
            "vtest-adapter-rust-coverage-{name}-{}.json",
            std::process::id()
        ));
        let mut file = std::fs::File::create(&path).expect("create coverage fixture");
        file.write_all(contents.as_bytes())
            .expect("write coverage fixture");
        path
    }

    /// @vtest.id TEST-ADAPTER-RUST-LLVM-COV-FUNCTION-COUNT-MATCH-AND-SUM
    /// @vtest.covers VO-EXEC-LLVM-COV-FUNCTIONS-LOOKUP, VO-EXEC-LLVM-COV-LOCATOR-SUFFIX-MATCH, VO-EXEC-LLVM-COV-GENERIC-COUNTS-SUM
    /// @vtest.target crates/vtest-adapter-rust/src/coverage.rs::llvm_cov_function_count
    /// @vtest.intent llvm-cov export JSONからlocatorに一致する関数（複数ジェネリックインスタンス含む）のcountを合算し、一致しないtargetはNoneを返すことを検証する
    #[test]
    fn llvm_cov_parser_extracts_target_function_count() {
        let target = rust_locator("src/lib.rs", "add");
        let output = r#"{
            "data": [{
                "functions": [
                    {
                        "name": "calc::add::<i32>",
                        "filenames": ["C:/workspace/calc/src/lib.rs"],
                        "count": 2
                    },
                    {
                        "name": "calc::add::<u64>",
                        "filenames": ["C:/workspace/calc/src/lib.rs"],
                        "regions": [[1, 0, 1, 10, 3]]
                    },
                    {
                        "name": "other::add",
                        "filenames": ["C:/workspace/calc/src/other.rs"],
                        "count": 99
                    }
                ]
            }]
        }"#;
        assert_eq!(llvm_cov_function_count(output, &target), Some(5));

        let absent = rust_locator("src/lib.rs", "subtract");
        assert_eq!(llvm_cov_function_count(output, &absent), None);
    }

    /// @vtest.id TEST-ADAPTER-RUST-LLVM-COV-ZERO-COUNT-NOT-CONFUSED-WITH-UNKNOWN
    /// @vtest.covers VO-EXEC-LLVM-COV-ZERO-COUNT-DISTINCT-FROM-UNKNOWN
    /// @vtest.target crates/vtest-adapter-rust/src/coverage.rs::llvm_cov_function_count
    /// @vtest.intent 対象関数が発見されcountが0のときSome(0)を返し、対象関数自体が見つからない場合のNoneと区別されることを検証する
    #[test]
    fn llvm_cov_zero_count_is_preserved_as_a_measured_failure() {
        let target = rust_locator("src/lib.rs", "add");
        let output = r#"{
            "data": [{
                "functions": [{
                    "name": "calc::add",
                    "filenames": ["src/lib.rs"],
                    "count": 0
                }]
            }]
        }"#;
        assert_eq!(llvm_cov_function_count(output, &target), Some(0));
    }

    /// @vtest.id TEST-ADAPTER-RUST-LLVM-COV-DEMANGLE-RUST-V0-NAME-MATCH
    /// @vtest.covers VO-EXEC-LLVM-COV-DEMANGLE-MATCH
    /// @vtest.target crates/vtest-adapter-rust/src/coverage.rs::llvm_name_matches
    /// @vtest.intent Rust v0 mangled関数名をdemangleした末尾がlocatorのitem-pathと一致するときだけ真を返すことを検証する
    #[test]
    fn llvm_cov_parser_demangles_rust_v0_symbols() {
        assert!(llvm_name_matches(
            "_RNvCs119z72hoDxF_12calc_fixture3add",
            "add"
        ));
        assert!(!llvm_name_matches(
            "_RNvCs119z72hoDxF_12calc_fixture8evaluate",
            "add"
        ));
    }

    /// impl 内関数（`型名::関数名`）の item path に対する一致を、トップ
    /// レベル関数（上のテスト）と区別して検証する。`name` はここでは
    /// (上の demangle テストとは異なり) 実 mangled シンボルではなく、
    /// `rustc_demangle::demangle` が対象としない形の文字列（元テストの
    /// `"calc::add::<i32>"` と同じ前例）で、`demangle` はマングル済みと
    /// 判定できない入力をそのまま返すため、`llvm_name_matches` の末尾一致
    /// 分岐（`ends_with("::{item_path}")`）を直接検証できる。
    /// @vtest.id TEST-ADAPTER-RUST-LLVM-COV-IMPL-METHOD-NAME-MATCH
    /// @vtest.covers VO-EXEC-LLVM-COV-LOCATOR-SUFFIX-MATCH
    /// @vtest.target crates/vtest-adapter-rust/src/coverage.rs::llvm_name_matches
    /// @vtest.intent impl内関数（型名::関数名）のitem pathがsymbol名の末尾と一致するときだけ真を返すことを検証する
    #[test]
    fn llvm_cov_parser_matches_impl_method_item_paths() {
        assert!(llvm_name_matches(
            "calc_fixture::Calcer::add",
            "Calcer::add"
        ));
        assert!(!llvm_name_matches(
            "calc_fixture::Calcer::subtract",
            "Calcer::add"
        ));
    }

    /// generic instantiation（`item_path::<Args>`）に対する一致を、
    /// `llvm_cov_function_count`の合算経路とは別に`llvm_name_matches`単体で
    /// 確認する。
    /// @vtest.id TEST-ADAPTER-RUST-LLVM-COV-GENERIC-INSTANTIATION-NAME-MATCH
    /// @vtest.covers VO-EXEC-LLVM-COV-GENERIC-COUNTS-SUM
    /// @vtest.target crates/vtest-adapter-rust/src/coverage.rs::llvm_name_matches
    /// @vtest.intent generic instantiation（item_path::<Args>）の demangle 名が一致し、無関係な名前が一致しないことを検証する
    #[test]
    fn llvm_cov_parser_matches_generic_instantiations() {
        assert!(llvm_name_matches("calc::add::<i32>", "add"));
        assert!(!llvm_name_matches("calc::subtract::<i32>", "add"));
    }

    /// `VO-EXEC-TARGET-COVERAGE-COUNT-JUDGEMENT`（DS-766/DS-767「計測された
    /// target別countが正のときはchecked:true・result:PASSとし、countが0の
    /// ときはchecked:true・result:FAILとする」）を、旧
    /// `measured_target_coverage_requires_a_positive_count`（`vtest-exec`、
    /// PR Bで除去）から引き継いで観測する。対象関数が出力に見当たらない
    /// 場合（UNKNOWN、DS-832）も同じ `measure` 呼び出しで検証するが、この
    /// 3件目のケースをclaimとするVOは正本に見当たらない（候補:
    /// DS-832、未確定 — count 0の`FAIL`とUNKNOWN一見当たらずの判定基準は
    /// 明文だが、それをclaimとする独立VOは無い）。
    /// @vtest.id TEST-ADAPTER-RUST-COVERAGE-TARGET-NOT-LOCATED-IS-UNKNOWN
    /// @vtest.covers VO-EXEC-TARGET-COVERAGE-COUNT-JUDGEMENT
    /// @vtest.target crates/vtest-adapter-rust/src/coverage.rs::RustCargoCoverageAdapter::measure
    /// @vtest.intent 到達（count>0でPASS）・未到達（count 0でFAIL）・対象関数が出力に見当たらない場合（UNKNOWN、DS-832）の3ケースをmeasureが正しく判定することを検証する
    #[test]
    fn measure_distinguishes_reached_unreached_and_not_located() {
        let output = r#"{
            "data": [{
                "functions": [
                    { "name": "calc::add", "filenames": ["src/lib.rs"], "count": 4 },
                    { "name": "calc::subtract", "filenames": ["src/lib.rs"], "count": 0 }
                ]
            }]
        }"#;
        let path = write_fixture("reachability", output);
        let adapter = RustCargoCoverageAdapter::new();
        let targets = vec![
            rust_locator("src/lib.rs", "add"),
            rust_locator("src/lib.rs", "subtract"),
            rust_locator("src/lib.rs", "multiply"),
        ];
        let measurements = adapter.measure(&path, &targets);
        assert_eq!(measurements.len(), 3);
        assert_eq!(measurements[0].result, TargetCoverageResult::Pass);
        assert_eq!(measurements[0].count, Some(4));
        assert_eq!(measurements[1].result, TargetCoverageResult::Fail);
        assert_eq!(measurements[1].count, Some(0));
        assert_eq!(measurements[2].result, TargetCoverageResult::Unknown);
        assert_eq!(measurements[2].count, None);
        std::fs::remove_file(path).expect("remove coverage fixture");
    }

    /// @vtest.id TEST-ADAPTER-RUST-COVERAGE-AVAILABILITY-REASON-IS-ADAPTER-OWNED
    /// @vtest.covers VO-EXEC-COVERAGE-UNAVAILABLE-NOT-CHECKED
    /// @vtest.target crates/vtest-adapter-rust/src/coverage.rs::RustCargoCoverageAdapter::availability
    /// @vtest.intent cargo-llvm-covが利用できない環境でavailabilityがErrを返し、その理由文言がこのadapter自身の文言であることを検証する
    #[test]
    fn availability_reports_the_adapters_own_unavailable_reason() {
        let missing_root = std::env::temp_dir().join(format!(
            "vtest-adapter-rust-coverage-missing-tool-{}",
            std::process::id()
        ));
        // A directory with no cargo-llvm-cov subcommand installed reachably
        // still runs `cargo llvm-cov --version` (cargo itself resolves from
        // PATH regardless of cwd); on a machine without the llvm-cov
        // subcommand this reliably fails. This crate's CI/dev environment is
        // assumed to have cargo on PATH but is not assumed to have
        // cargo-llvm-cov installed — if it does, this test is skipped rather
        // than asserting a specific environment's tool availability.
        std::fs::create_dir_all(&missing_root).expect("create fixture root");
        let adapter = RustCargoCoverageAdapter::new();
        if let Err(reason) = adapter.availability(&missing_root) {
            assert_eq!(
                reason,
                "cargo-llvm-cov is unavailable; target_coverage is NOT_CHECKED"
            );
        }
        std::fs::remove_dir_all(&missing_root).expect("remove fixture root");
    }
}
