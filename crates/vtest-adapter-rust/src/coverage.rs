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
        probe_availability("cargo", root)
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

/// `availability`の実体。`cargo_binary`をtest側から差し替え可能にすることで
/// （production側は常に`"cargo"`を渡し、挙動は変わらない）、レビュー指摘
/// 「`if let Err`の内側でしかassertせず、cargo-llvm-covが入った環境では
/// 何も検証しない」を、実環境のtool有無に依存しない決定論的なテストへ直す
/// ための足場。存在しないbinary名を渡せば`Command::new(...).output()`は
/// 環境を問わず必ず失敗するため、Err分岐を無条件に踏める。
fn probe_availability(cargo_binary: &str, root: &Path) -> Result<(), String> {
    Command::new(cargo_binary)
        .current_dir(root)
        .args(["llvm-cov", "--version"])
        .output()
        .ok()
        .filter(|output| output.status.success())
        .map(|_| ())
        .ok_or_else(|| "cargo-llvm-cov is unavailable; target_coverage is NOT_CHECKED".to_owned())
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

/// How a target Locator is matched against llvm-cov's function records.
enum TargetMatch {
    /// The Locator resolved to one source construct in the measured file:
    /// a function record matches when its name ends with the construct's
    /// own name and the innermost discovered construct that contains the
    /// record's first region (line and column) is this one. This keeps
    /// cfg branches, same-named trait impl methods and same-named functions
    /// of other modules apart even when they share a line, and even though
    /// their Locators (`f[cfg(unix)]`, `<Widget as A>::f`) are not LLVM
    /// symbol names (#35, DS-510).
    Span {
        name: String,
        construct: ResolvedConstruct,
    },
    /// The measured file cannot be read (e.g. coverage output from another
    /// machine): the plain item path is compared with the symbol name. A
    /// disambiguated Locator is never matched this way, so it stays
    /// UNKNOWN rather than borrowing another branch's count.
    ItemPath(String),
}

/// The byte range of the target's source construct, the byte ranges of
/// every other construct discovered in the same file, and the byte offset
/// at which each line starts (to place llvm-cov's 1-based line/column).
struct ResolvedConstruct {
    range: (u64, u64),
    others: Vec<(u64, u64)>,
    line_starts: Vec<u64>,
}

impl ResolvedConstruct {
    /// Whether a function record starting at `(line, column)` belongs to
    /// this construct: the position lies in its range and in no smaller
    /// other construct inside it (e.g. a nested `fn` or another `mod`'s
    /// function on the same line).
    fn owns(&self, (line, column): (u64, u64)) -> bool {
        let Some(line_start) = usize::try_from(line)
            .ok()
            .and_then(|line| line.checked_sub(1))
            .and_then(|index| self.line_starts.get(index))
        else {
            return false;
        };
        let offset = line_start + column.saturating_sub(1);
        let contains = |(start, end): (u64, u64)| start <= offset && offset < end;
        contains(self.range)
            && !self.others.iter().any(|&other| {
                contains(other)
                    && self.range.0 <= other.0
                    && other.1 <= self.range.1
                    && other != self.range
            })
    }
}

/// The function name an item path ends with: `f` for `m::Widget::f`,
/// `<Widget as A>::f`, `<W as A<[u8; 1]>>::f` or `f[cfg(unix)]`. Only the
/// `[cfg(...)]` suffix that disambiguation appends is removed; square
/// brackets of type syntax are kept.
fn construct_name(item_path: &str) -> &str {
    let base = match item_path.rfind("[cfg(") {
        Some(index) if item_path.ends_with(']') => &item_path[..index],
        _ => item_path,
    };
    base.rsplit("::").next().unwrap_or(base)
}

fn is_disambiguated(item_path: &str) -> bool {
    item_path.contains('[') || item_path.contains('<')
}

/// Resolves `target` to the one source construct the `rust-cargo` scanner
/// gives that Locator, using the project root implied by an absolute
/// llvm-cov filename that ends with the Locator's path.
fn resolve_target(data: &[serde_json::Value], target: &Locator) -> Option<ResolvedConstruct> {
    let (target_path, _) = locator_parts(target);
    let suffix = target_path.replace('\\', "/");
    let filenames = data
        .iter()
        .filter_map(|item| item.get("functions")?.as_array())
        .flatten()
        .filter_map(|function| function.get("filenames")?.as_array())
        .flatten()
        .filter_map(serde_json::Value::as_str);
    for filename in filenames {
        let normalized = filename.replace('\\', "/");
        let Some(root) = normalized.strip_suffix(&suffix) else {
            continue;
        };
        if !root.ends_with('/') || !Path::new(root).is_absolute() {
            continue;
        }
        let root = Path::new(root);
        let file = root.join(target_path);
        let Ok(text) = std::fs::read_to_string(&file) else {
            continue;
        };
        let mut scanner = crate::Scanner::new(root, "");
        if scanner.scan_file(&file).is_err() {
            continue;
        }
        let outcome = scanner.finish(1);
        let mut found = outcome
            .sources
            .iter()
            .filter(|source| source.locator.value == target.value);
        let (Some(source), None) = (found.next(), found.next()) else {
            return None;
        };
        let range_of = |range: &vtest_model::SourceRange| (range.start, range.end);
        let others = outcome
            .sources
            .iter()
            .filter(|other| other.locator.value != target.value)
            .map(|other| range_of(&other.location.byte_range))
            .collect();
        let line_starts = std::iter::once(0)
            .chain(
                text.bytes()
                    .enumerate()
                    .filter(|&(_, byte)| byte == b'\n')
                    .map(|(index, _)| index as u64 + 1),
            )
            .collect();
        return Some(ResolvedConstruct {
            range: range_of(&source.location.byte_range),
            others,
            line_starts,
        });
    }
    None
}

/// The earliest `(line, column)` among a function record's regions.
fn function_start(function: &serde_json::Value) -> Option<(u64, u64)> {
    function
        .get("regions")?
        .as_array()?
        .iter()
        .filter_map(|region| {
            let region = region.as_array()?;
            Some((region.first()?.as_u64()?, region.get(1)?.as_u64()?))
        })
        .min()
}

fn llvm_cov_function_count(output: &str, target: &Locator) -> Option<u64> {
    let value = serde_json::from_str::<serde_json::Value>(output).ok()?;
    let data = value.get("data")?.as_array()?;
    let mut total = 0_u64;
    let mut matched = false;
    let (target_path, target_item_path) = locator_parts(target);
    let rule = match resolve_target(data, target) {
        Some(construct) => TargetMatch::Span {
            name: construct_name(target_item_path).to_owned(),
            construct,
        },
        None if is_disambiguated(target_item_path) => return None,
        None => TargetMatch::ItemPath(target_item_path.to_owned()),
    };
    for item in data {
        let Some(functions) = item.get("functions").and_then(serde_json::Value::as_array) else {
            continue;
        };
        for function in functions {
            let Some(name) = function.get("name").and_then(serde_json::Value::as_str) else {
                continue;
            };
            let name_matches = match &rule {
                TargetMatch::Span {
                    name: own,
                    construct,
                } => {
                    llvm_name_matches(name, own)
                        && function_start(function).is_some_and(|start| construct.owns(start))
                }
                TargetMatch::ItemPath(item_path) => llvm_name_matches(name, item_path),
            };
            if !name_matches || !llvm_filenames_match(function, target_path) {
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

    /// count>0/==0の2ケースは`VO-EXEC-TARGET-COVERAGE-COUNT-JUDGEMENT`
    /// （DS-766/DS-767「計測されたtarget別countが正のときはchecked:true・
    /// result:PASSとし、countが0のときはchecked:true・result:FAILとする」）
    /// を、旧`measured_target_coverage_requires_a_positive_count`
    /// （`vtest-exec`、PR Bで除去）から引き継いで観測する。対象関数が出力に
    /// 見当たらない3件目のケース（UNKNOWN）は、round 2レビュー指摘
    /// （欠陥E-3）で見つかった見落としを修正し、`VO-EXEC-COVERAGE-
    /// TARGET-NOT-LOCATED-IS-UNKNOWN`（DS-768「target別判定は、関数が
    /// 見つからなければUNKNOWNとする」）で観測する。DS-768はこのcomment
    /// が引くDS-766/DS-767の2項目先にあり、かつ本crateへ移設済みの
    /// `VO-EXEC-LLVM-COV-ZERO-COUNT-DISTINCT-FROM-UNKNOWN`の`derives_from`
    /// に既に含まれている条文であり、「それをclaimとする独立VOは無い」と
    /// 断定していた旧commentは誤りだった。
    /// @vtest.id TEST-ADAPTER-RUST-COVERAGE-TARGET-NOT-LOCATED-IS-UNKNOWN
    /// @vtest.covers VO-EXEC-TARGET-COVERAGE-COUNT-JUDGEMENT, VO-EXEC-COVERAGE-TARGET-NOT-LOCATED-IS-UNKNOWN
    /// @vtest.target crates/vtest-adapter-rust/src/coverage.rs::RustCargoCoverageAdapter::measure
    /// @vtest.intent 到達（count>0でPASS）・未到達（count 0でFAIL）・対象関数が出力に見当たらない場合（UNKNOWN、DS-768）の3ケースをmeasureが正しく判定することを検証する
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

    // round 2 レビュー指摘（欠陥E-1、2件）:
    // (1) 旧テストはassertを`if let Err`の内側だけに置いており、
    //     cargo-llvm-covが導入済みの環境（このリポジトリの作業ツリー自身が
    //     そう — `.verify/cache/cov/`に実出力が50件超存在する）ではErr分岐
    //     に一度も入らず、何も検証していなかった。存在しないbinary名を
    //     `probe_availability`へ渡すことで、実行環境のtool有無に依存せず
    //     Err分岐を無条件に踏み、`RustCargoCoverageAdapter::availability`が
    //     最終的に委譲する同じロジック・同じ文言を決定論的に検証する
    //     （productionコードは常に`"cargo"`を渡すので挙動は不変）。
    // (2) このテストが観測するのはadapterが返すErr文字列そのものだけで、
    //     `VO-EXEC-COVERAGE-UNAVAILABLE-NOT-CHECKED`のclaim（checked:false・
    //     method null・result null・targets空・NO_EVIDENCE）が挙げる
    //     target_coverage側のfieldを1つも観測しない（そちらは
    //     vtest-exec::tests::coverage_unavailable_produces_not_checked_
    //     target_coverage_and_w_exec_101が観測する）。この文言自体を
    //     claimとする正本条文も見当たらない — 候補: DS-760（L29657、
    //     W-EXEC-101の発行のみを定め文言は定めない）・BD-220、未確定。
    //     covers を持たない無印の #[test]（W-SCAN-101）のまま残す。
    #[test]
    fn availability_reports_the_adapters_own_unavailable_reason() {
        let missing_root = std::env::temp_dir().join(format!(
            "vtest-adapter-rust-coverage-missing-tool-{}",
            std::process::id()
        ));
        std::fs::create_dir_all(&missing_root).expect("create fixture root");
        let reason =
            probe_availability("vtest-nonexistent-cargo-binary-for-testing", &missing_root)
                .expect_err("a nonexistent binary must always fail to launch");
        assert_eq!(
            reason,
            "cargo-llvm-cov is unavailable; target_coverage is NOT_CHECKED"
        );
        std::fs::remove_dir_all(&missing_root).expect("remove fixture root");
    }

    /// Runs real `cargo llvm-cov --lib --json` in `root` (a fixture crate)
    /// with the outer cargo-llvm-cov environment removed, and returns the
    /// path of the JSON it wrote.
    fn run_real_llvm_cov(root: &std::path::Path) -> std::path::PathBuf {
        let output_path = root.join("coverage.json");
        let mut command = Command::new("cargo");
        command.current_dir(root).args([
            "llvm-cov",
            "--lib",
            "--json",
            "--output-path",
            output_path.to_str().expect("utf-8 path"),
        ]);
        for variable in [
            "RUSTC_WRAPPER",
            "LLVM_PROFILE_FILE",
            "CARGO_LLVM_COV",
            "CARGO_LLVM_COV_TARGET_DIR",
            "RUSTFLAGS",
            "CARGO_ENCODED_RUSTFLAGS",
            "CARGO_INCREMENTAL",
            "CARGO_TARGET_DIR",
            "CARGO_LLVM_COV_SHOW_ENV",
        ] {
            command.env_remove(variable);
        }
        let status = command.output().expect("run cargo llvm-cov");
        assert!(
            status.status.success(),
            "{}",
            String::from_utf8_lossy(&status.stderr)
        );

        output_path
    }

    /// Discovered Locators of `src/lib.rs` in the fixture crate `root`.
    fn discovered_locators(root: &std::path::Path) -> Vec<String> {
        use vtest_adapter_api::SourceDiscoveryAdapter;
        crate::RustCargoAdapter::new()
            .discover(
                root,
                "demo",
                &vtest_adapter_api::AdapterScanConfig {
                    include_paths: vec![std::path::PathBuf::from("src")],
                },
            )
            .expect("discover")
            .sources
            .into_iter()
            .map(|source| source.locator.value)
            .collect()
    }

    /// #139 review: two constructs on one line are told apart by the column
    /// of their LLVM region, so an uncalled one never borrows the called
    /// one's count — for trait impls, for same-named functions of two
    /// modules, and for exclusive cfg branches. A trait impl whose type
    /// argument has square brackets (`T<[u8; 1]>`) is still measured.
    /// @vtest.id TEST-ADAPTER-RUST-COVERAGE-SAME-LINE-AND-TYPE-SYNTAX-LOCATORS
    /// @vtest.covers VO-ADAPTER-RUST-COVERAGE-DISAMBIGUATED-LOCATORS
    /// @vtest.target crates/vtest-adapter-rust/src/coverage.rs::RustCargoCoverageAdapter::measure
    /// @vtest.target crates/vtest-adapter-rust/src/coverage.rs::llvm_cov_function_count
    /// @vtest.intent 同じ行に並ぶ別trait・別module・別cfgの関数と、配列型を型引数に持つtrait実装の各Locatorが、自分のconstructのcountだけで判定されることを実cargo-llvm-covで確認する
    #[test]
    fn real_llvm_cov_keeps_same_line_constructs_and_type_syntax_apart() {
        let root = std::env::temp_dir().join(format!(
            "vtest-adapter-rust-coverage-same-line-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(root.join("src")).expect("create src");
        std::fs::write(
            root.join("Cargo.toml"),
            "[package]\nname = \"demo\"\nversion = \"0.1.0\"\nedition = \"2021\"\n\n[workspace]\n",
        )
        .expect("write manifest");
        std::fs::write(
            root.join("src/lib.rs"),
            "pub struct W;\n\
pub trait A { fn f(&self) -> i32; }\n\
pub trait B { fn f(&self) -> i32; }\n\
pub trait T<X> { fn h(&self) -> i32; }\n\
impl A for W { fn f(&self) -> i32 { 1 } } impl B for W { fn f(&self) -> i32 { 2 } }\n\
pub mod a { pub fn f() -> i32 { 1 } } pub mod b { pub fn f() -> i32 { 2 } }\n\
#[cfg(unix)] pub fn g() -> i32 { 3 } #[cfg(not(unix))] pub fn g() -> i32 { 4 }\n\
impl T<[u8; 1]> for W { fn h(&self) -> i32 { 5 } }\n\
impl T<[u8; 2]> for W { fn h(&self) -> i32 { 6 } }\n\
#[cfg(test)]\nmod tests {\n    #[test]\n    fn calls_one_of_each() {\n        \
let w = super::W;\n        \
assert!(super::A::f(&w) + super::a::f() + super::g() + <super::W as super::T<[u8; 1]>>::h(&w) > 0);\n    }\n}\n",
        )
        .expect("write lib.rs");

        let output_path = run_real_llvm_cov(&root);

        let (own_cfg, other_cfg) = if cfg!(unix) {
            ("g[cfg(unix)]", "g[cfg(not(unix))]")
        } else {
            ("g[cfg(not(unix))]", "g[cfg(unix)]")
        };
        let item_paths = [
            "<W as A>::f",
            "<W as B>::f",
            "a::f",
            "b::f",
            own_cfg,
            other_cfg,
            "<W as T<[u8;1]>>::h",
            "<W as T<[u8;2]>>::h",
        ];
        let targets: Vec<_> = item_paths
            .iter()
            .map(|item_path| rust_locator("src/lib.rs", item_path))
            .collect();
        let discovered = discovered_locators(&root);
        for target in &targets {
            assert!(
                discovered.contains(&target.value),
                "{} is not a discovered Locator: {discovered:?}",
                target.value
            );
        }

        let measured = RustCargoCoverageAdapter::new().measure(&output_path, &targets);
        let results: Vec<_> = measured
            .iter()
            .map(|measurement| (measurement.result, measurement.count.map(|count| count > 0)))
            .collect();
        assert_eq!(
            results,
            [
                (TargetCoverageResult::Pass, Some(true)),
                (TargetCoverageResult::Fail, Some(false)),
                (TargetCoverageResult::Pass, Some(true)),
                (TargetCoverageResult::Fail, Some(false)),
                (TargetCoverageResult::Pass, Some(true)),
                (TargetCoverageResult::Unknown, None),
                (TargetCoverageResult::Pass, Some(true)),
                (TargetCoverageResult::Fail, Some(false)),
            ],
            "{measured:?}"
        );
        std::fs::remove_dir_all(&root).ok();
    }

    /// #35 / DS-510: a disambiguated Locator (`<Widget as A>::f`,
    /// `g[cfg(unix)]`) is not an LLVM symbol name. Measured with real
    /// cargo-llvm-cov, each Locator gets the count of its own source
    /// construct: the called trait impl PASSes and the same-named uncalled
    /// one FAILs, the compiled cfg branch PASSes and the branch that is not
    /// compiled stays UNKNOWN instead of borrowing the other's count.
    /// @vtest.id TEST-ADAPTER-RUST-COVERAGE-DISAMBIGUATED-LOCATORS
    /// @vtest.covers VO-ADAPTER-RUST-COVERAGE-DISAMBIGUATED-LOCATORS
    /// @vtest.target crates/vtest-adapter-rust/src/coverage.rs::RustCargoCoverageAdapter::measure
    /// @vtest.target crates/vtest-adapter-rust/src/coverage.rs::llvm_cov_function_count
    /// @vtest.intent 実cargo-llvm-covで、同名trait実装とcfg分岐の各Locatorが自分のconstructのcountだけで判定されることを確認する
    #[test]
    fn real_llvm_cov_measures_trait_impl_and_cfg_locators_separately() {
        let root = std::env::temp_dir().join(format!(
            "vtest-adapter-rust-coverage-disambiguated-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(root.join("src")).expect("create src");
        std::fs::write(
            root.join("Cargo.toml"),
            "[package]\nname = \"demo\"\nversion = \"0.1.0\"\nedition = \"2021\"\n\n[workspace]\n",
        )
        .expect("write manifest");
        std::fs::write(
            root.join("src/lib.rs"),
            "pub struct Widget;\n\
pub trait A {\n    fn f(&self) -> i32;\n}\n\
pub trait B {\n    fn f(&self) -> i32;\n}\n\
impl A for Widget {\n    fn f(&self) -> i32 {\n        1\n    }\n}\n\
impl B for Widget {\n    fn f(&self) -> i32 {\n        2\n    }\n}\n\
#[cfg(unix)]\npub fn g() -> i32 {\n    3\n}\n\
#[cfg(not(unix))]\npub fn g() -> i32 {\n    4\n}\n\
#[cfg(test)]\nmod tests {\n    #[test]\n    fn calls_a_and_g() {\n        \
assert_eq!(super::A::f(&super::Widget) + super::g(), if cfg!(unix) { 4 } else { 5 });\n    }\n}\n",
        )
        .expect("write lib.rs");

        let output_path = run_real_llvm_cov(&root);

        let locator = |item_path: &str| rust_locator("src/lib.rs", item_path);
        let (own_cfg, other_cfg) = if cfg!(unix) {
            ("g[cfg(unix)]", "g[cfg(not(unix))]")
        } else {
            ("g[cfg(not(unix))]", "g[cfg(unix)]")
        };
        let targets = vec![
            locator("<Widget as A>::f"),
            locator("<Widget as B>::f"),
            locator(own_cfg),
            locator(other_cfg),
        ];
        // The Locators are the ones discovery itself produces.
        use vtest_adapter_api::SourceDiscoveryAdapter;
        let outcome = crate::RustCargoAdapter::new()
            .discover(
                &root,
                "demo",
                &vtest_adapter_api::AdapterScanConfig {
                    include_paths: vec![std::path::PathBuf::from("src")],
                },
            )
            .expect("discover");
        for target in &targets {
            assert!(
                outcome
                    .sources
                    .iter()
                    .any(|source| source.locator.value == target.value),
                "{} is not a discovered Locator",
                target.value
            );
        }

        let measured = RustCargoCoverageAdapter::new().measure(&output_path, &targets);
        let results: Vec<_> = measured
            .iter()
            .map(|measurement| (measurement.result, measurement.count.map(|count| count > 0)))
            .collect();
        assert_eq!(
            results,
            [
                (TargetCoverageResult::Pass, Some(true)),
                (TargetCoverageResult::Fail, Some(false)),
                (TargetCoverageResult::Pass, Some(true)),
                (TargetCoverageResult::Unknown, None),
            ],
            "{measured:?}"
        );

        // Without the source file the disambiguated Locators cannot be tied
        // to one construct, so they are UNKNOWN, never another one's count.
        std::fs::remove_file(root.join("src/lib.rs")).expect("remove lib.rs");
        let measured = RustCargoCoverageAdapter::new().measure(&output_path, &targets[..1]);
        assert_eq!(measured[0].result, TargetCoverageResult::Unknown);
        std::fs::remove_dir_all(&root).ok();
    }
}
