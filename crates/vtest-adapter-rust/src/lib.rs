//! `rust-cargo` `SourceDiscoveryAdapter`（詳細設計 v0.1 §5.5・§4.2・§4.3）。
//!
//! 詳細設計 v0.1 §1.1（本冊:30-60）: "`vtest-adapter-rust`/ # rust-cargo
//! discovery/static-analysis/operations/runner/coverage" および
//! "`vtest-scan`、`vtest-audit`、`vtest-exec` はadapterを選択・委譲する
//! orchestrationであり、それぞれが`syn`、`quote`、`rustc-demangle`、Cargo
//! commandを直接所有しない"。この crate が `syn` によるソース走査・
//! `Cargo.toml` 解析・Rust パス解決を所有し、`vtest-scan`（core）はこの
//! crate の出力（`vtest_adapter_api::DiscoveryOutcome`）だけを消費する。
//!
//! PR3 時点で本冊 §5.5 の手順のうち、この crate が実行するのは
//! 1（ファイル探索）・2（構文解析）・3（モジュールパス構築）・4（Test
//! construct抽出）・5（metadata宣言抽出）・6（Source Target抽出・
//! locator/`@vtest.src-id`認識）・7（draft生成）である。Test ID の大域的
//! 一意性・`covers` の VO 参照解決・Target Reference 解決（§6.1）は
//! adapter の責務ではなく（本冊:571）、`vtest-scan` 側に残る。

use std::{
    collections::BTreeSet,
    fs, io,
    path::{Path, PathBuf},
};

use ignore::WalkBuilder;
use serde::Deserialize;
use syn::spanned::Spanned;
use syn::{Attribute, Expr, ExprLit, ImplItem, Item, ItemFn, ItemImpl, Lit, Meta};
use vtest_adapter_api::{
    AdapterScanConfig, DiscoveryError, DiscoveryOutcome, SourceDiscoveryAdapter, SourceDraft,
    TestDraft,
};
use vtest_model::{
    AdapterId, Diagnostic, Locator, SourceLocation, SrcId, TargetRef, TestId, TestTarget, VoId,
};

/// 本冊 §4.3「`rust-cargo` adapterはこの値を`TargetRef::Locator { adapter:
/// "rust-cargo", value: locator }`へ正規化する」。`config.yaml` の
/// `adapters[].id`、`AdapterRegistry` のキー、`TargetRef::Locator.adapter`
/// のいずれもこの文字列で揃える。
pub const ADAPTER_ID: &str = "rust-cargo";

/// `rust-cargo` が所有する opaque locator value の内部構文
/// （`<project-relative path>.rs::<item path>`）。この構文の定義・
/// parse・正規化は本冊:522「coreがpath、module、symbol種別を分解しない」
/// により core（`vtest-model`・`vtest-scan`）に置かない。core が保持する
/// のは常に `vtest_model::Locator { adapter, value }` の opaque な
/// `value` 文字列だけであり、`RustLocator` はこの adapter とその呼び出し元
/// （`vtest-scan::operations` の symbol 検証・編集操作。canonical化は
/// PR3 の範囲外、`pr3-decisions.md`「保留中の論点」）だけが使う。
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RustLocator {
    pub path: String,
    pub item_path: String,
}

impl RustLocator {
    /// `<path>.rs::<item_path>` 形式として構文解析する。`path` は
    /// project-relative で `.rs` へ末尾一致し、`item_path` は空でない
    /// ことを要求する。この検証は adapter 自身の value 構文の妥当性
    /// チェックであり、core の resolution（§6.1、`vtest-scan::
    /// resolve_targets`）が行う「実在する Source Target と一致するか」
    /// の判定とは別物 — parse に失敗した値も、そのまま opaque `value`
    /// として core へ渡してよい（本冊:955/959「構文解析できない
    /// `@vtest.target` 値を adapter が独自に埋め合わせてはならない」。
    /// `pr3-decisions.md` Owner裁定2「架空locatorを作らない」）。
    pub fn parse(value: &str) -> Option<Self> {
        let separator = value.find("::")?;
        let (path, item_path) = value.split_at(separator);
        let item_path = item_path.strip_prefix("::")?;
        if path.is_empty() || item_path.is_empty() || !path.ends_with(".rs") {
            return None;
        }
        Some(Self {
            path: path.replace('\\', "/"),
            item_path: item_path.to_owned(),
        })
    }

    /// この adapter の正規化された opaque locator value（`path::item_path`,
    /// path は forward slash 済み）。
    pub fn to_value(&self) -> String {
        format!("{}::{}", self.path, self.item_path)
    }

    /// この adapter が所有する `vtest_model::Locator` へ包む。
    pub fn to_locator(&self) -> Locator {
        Locator {
            adapter: AdapterId::new(ADAPTER_ID),
            value: self.to_value(),
        }
    }
}

/// `raw` を `rust-cargo` の locator 構文として解釈できれば正規化した
/// `vtest_model::Locator` を返す。解釈できない場合は `raw` をそのまま
/// opaque value として包む — 構文解析できないことは「解決できない」
/// （core の §6.1 resolution が実在する Source Target と0件一致で
/// 判定する）だけであり、adapter が代替値を捏造してはならない
/// （`pr3-decisions.md` Owner裁定2）。
pub fn locator_from_declared_value(raw: &str) -> Locator {
    match RustLocator::parse(raw) {
        Some(parsed) => parsed.to_locator(),
        None => Locator {
            adapter: AdapterId::new(ADAPTER_ID),
            value: raw.to_owned(),
        },
    }
}

/// 本冊 §5.5 の `SourceDiscoveryAdapter` 実装。ID `"rust-cargo"` は
/// `config.yaml` の `adapters[].id` および `TargetRef::Locator.adapter` と
/// 一致する（本冊 §4.3「`rust-cargo` adapterはこの値を`TargetRef::Locator {
/// adapter: "rust-cargo", value: locator }`へ正規化する」）。
#[derive(Default)]
pub struct RustCargoAdapter;

impl RustCargoAdapter {
    pub fn new() -> Self {
        Self
    }
}

impl SourceDiscoveryAdapter for RustCargoAdapter {
    fn id(&self) -> &'static str {
        ADAPTER_ID
    }

    fn discover(
        &self,
        root: &Path,
        fallback_package: &str,
        config: &AdapterScanConfig,
    ) -> Result<DiscoveryOutcome, DiscoveryError> {
        // 本冊 §5.5 手順1: adapter configのinclude配下の*.rsをignoreクレート
        // で列挙する（.gitignore準拠、target/は除外）。root × scan.include の
        // path 解決自体は core が行い（Rust 固有処理ではない）、この adapter
        // は解決済みの project-relative path だけを受け取る。
        let mut paths = Vec::new();
        for include_path in &config.include_paths {
            let full = root.join(include_path);
            collect_rs_files(root, &full, &mut paths).map_err(|error| DiscoveryError {
                path: full.clone(),
                message: error.to_string(),
            })?;
        }
        paths.sort();
        paths.dedup();

        let package = package_name(root).unwrap_or_else(|| fallback_package.to_owned());
        let mut scanner = Scanner::new(root, &package);
        for path in &paths {
            scanner.scan_file(path)?;
        }
        Ok(scanner.finish(paths.len()))
    }
}

fn package_name(root: &Path) -> Option<String> {
    cargo_manifest(root).and_then(|manifest| manifest.package.map(|package| package.name))
}

fn collect_rs_files(
    project_root: &Path,
    path: &Path,
    output: &mut Vec<PathBuf>,
) -> Result<(), ignore::Error> {
    if !path.exists() {
        return Ok(());
    }
    if path.is_file() {
        if path.extension().and_then(|v| v.to_str()) == Some("rs") {
            output.push(path.to_owned());
        }
        return Ok(());
    }
    let include_root = path.to_owned();
    let project_root = project_root.to_owned();
    let mut builder = WalkBuilder::new(&project_root);
    builder
        .standard_filters(false)
        .hidden(false)
        .parents(false)
        .ignore(false)
        .git_ignore(true)
        .git_global(false)
        .git_exclude(false)
        .require_git(false)
        .follow_links(false)
        .filter_entry(move |entry| {
            entry.file_name().to_str() != Some("target")
                && (include_root.starts_with(entry.path())
                    || entry.path().starts_with(&include_root))
        });
    for entry in builder.build() {
        let entry = entry?;
        if !entry.file_type().is_some_and(|kind| kind.is_file()) {
            continue;
        }
        if entry.path().extension().and_then(|value| value.to_str()) == Some("rs") {
            output.push(entry.into_path());
        }
    }
    Ok(())
}

struct Scanner<'a> {
    root: &'a Path,
    fallback_package: &'a str,
    tests: Vec<TestDraft>,
    sources: Vec<SourceDraft>,
    diagnostics: Vec<Diagnostic>,
}

impl<'a> Scanner<'a> {
    fn new(root: &'a Path, fallback_package: &'a str) -> Self {
        Self {
            root,
            fallback_package,
            tests: Vec::new(),
            sources: Vec::new(),
            diagnostics: Vec::new(),
        }
    }

    fn scan_file(&mut self, path: &Path) -> Result<(), DiscoveryError> {
        let file_name = path
            .file_name()
            .and_then(|value| value.to_str())
            .unwrap_or_default();
        let file_location = record_location(self.root, path, file_name);
        let source = match fs::read_to_string(path) {
            Ok(source) => source,
            Err(source) => {
                self.diagnostics.push(
                    Diagnostic::error(
                        "E-SCAN-001",
                        format!("failed to read {}: {source}", path.display()),
                    )
                    .with_location(file_location.clone()),
                );
                return Ok(());
            }
        };
        let syntax = match syn::parse_file(&source) {
            Ok(syntax) => syntax,
            Err(error) => {
                self.diagnostics.push(
                    Diagnostic::error(
                        "E-SCAN-001",
                        format!("failed to parse {}: {error}", path.display()),
                    )
                    .with_location(file_location),
                );
                return Ok(());
            }
        };
        let relative = path
            .strip_prefix(self.root)
            .unwrap_or(path)
            .to_string_lossy()
            .replace('\\', "/");
        let context = source_context(self.root, path, self.fallback_package)?;
        let line_offsets = line_offsets(&source);
        self.collect_items(
            &syntax.items,
            &relative,
            &context.test_target,
            &context.package,
            &context.filter_prefix,
            &source,
            &line_offsets,
            "",
            path,
        )
    }

    #[allow(clippy::too_many_arguments)]
    fn collect_items(
        &mut self,
        items: &[Item],
        relative: &str,
        test_target: &TestTarget,
        package: &str,
        filter_prefix: &str,
        source: &str,
        line_offsets: &[usize],
        module: &str,
        path: &Path,
    ) -> Result<(), DiscoveryError> {
        for item in items {
            match item {
                Item::Fn(item_fn) => self.collect_fn(
                    item_fn,
                    relative,
                    test_target,
                    package,
                    filter_prefix,
                    source,
                    line_offsets,
                    module,
                    path,
                )?,
                Item::Impl(item_impl) => self.collect_impl(
                    item_impl,
                    relative,
                    test_target,
                    package,
                    filter_prefix,
                    source,
                    line_offsets,
                    module,
                    path,
                )?,
                Item::Mod(item_mod) => {
                    if let Some((_, nested)) = &item_mod.content {
                        let nested_module = if module.is_empty() {
                            item_mod.ident.to_string()
                        } else {
                            format!("{module}::{}", item_mod.ident)
                        };
                        self.collect_items(
                            nested,
                            relative,
                            test_target,
                            package,
                            filter_prefix,
                            source,
                            line_offsets,
                            &nested_module,
                            path,
                        )?;
                    }
                }
                _ => {}
            }
        }
        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    fn collect_impl(
        &mut self,
        item_impl: &ItemImpl,
        relative: &str,
        test_target: &TestTarget,
        package: &str,
        filter_prefix: &str,
        source: &str,
        line_offsets: &[usize],
        module: &str,
        path: &Path,
    ) -> Result<(), DiscoveryError> {
        let type_name = match item_impl.self_ty.as_ref() {
            syn::Type::Path(value) => value.path.segments.last().map(|v| v.ident.to_string()),
            _ => None,
        };
        let Some(type_name) = type_name else {
            return Ok(());
        };
        for item in &item_impl.items {
            let ImplItem::Fn(item_fn) = item else {
                continue;
            };
            let item_path = if module.is_empty() {
                format!("{type_name}::{}", item_fn.sig.ident)
            } else {
                format!("{module}::{type_name}::{}", item_fn.sig.ident)
            };
            self.collect_function_parts(
                &item_fn.attrs,
                &item_fn.sig.ident.to_string(),
                &item_path,
                item_fn.span(),
                relative,
                test_target,
                package,
                filter_prefix,
                source,
                line_offsets,
                path,
            )?;
        }
        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    fn collect_fn(
        &mut self,
        item_fn: &ItemFn,
        relative: &str,
        test_target: &TestTarget,
        package: &str,
        filter_prefix: &str,
        source: &str,
        line_offsets: &[usize],
        module: &str,
        path: &Path,
    ) -> Result<(), DiscoveryError> {
        let item_path = if module.is_empty() {
            item_fn.sig.ident.to_string()
        } else {
            format!("{module}::{}", item_fn.sig.ident)
        };
        self.collect_function_parts(
            &item_fn.attrs,
            &item_fn.sig.ident.to_string(),
            &item_path,
            item_fn.span(),
            relative,
            test_target,
            package,
            filter_prefix,
            source,
            line_offsets,
            path,
        )
    }

    #[allow(clippy::too_many_arguments)]
    fn collect_function_parts(
        &mut self,
        attrs: &[Attribute],
        function_name: &str,
        item_path: &str,
        span: proc_macro2::Span,
        relative: &str,
        test_target: &TestTarget,
        package: &str,
        filter_prefix: &str,
        source: &str,
        line_offsets: &[usize],
        path: &Path,
    ) -> Result<(), DiscoveryError> {
        let location = make_location(relative, item_path, span, source, line_offsets);
        // 詳細設計 v0.1 のどの診断コード表にも `E-CORE-001` は存在せず、コード
        // 自体を実装が発明することは禁止（PM 裁定・pr3-decisions.md 裁定2）。
        // byte range の out-of-bounds は「壊れた発見結果を診断として黙って
        // 通す」ことになり fail-open なので、診断を出さず discovery 全体の
        // 失敗（`DiscoveryError`。core はこれを `ScanError::Discovery` へ
        // 変換する）として扱う。
        let Some(content) = source_slice(source, &location) else {
            return Err(DiscoveryError {
                path: path.to_owned(),
                message: format!("function `{item_path}` source range is out of bounds"),
            });
        };

        // 本冊 §4.2: `@vtest.` 宣言は表面ごとに異なるキー集合を認識する。
        // 表面1 = Test construct の doc comment（test-key）。
        // 表面2 = Test construct ではない関数 item の doc comment
        //         （source-target-key = `src-id`）。
        let is_test = is_test_function(attrs);

        // 本冊 §5.5 手順6: すべての fn / impl fn を SRC 候補として索引化する。
        // ただし恒久 SRC ID（`@vtest.src-id`）の認識は非 Test construct の
        // 宣言に限る（§4.2）。Test construct 自身の doc comment にある
        // `src-id` は誤配置であり、表面1側で未知キーとして E-SCAN-006 になる
        // （下の `parse_test_annotations` が処理する）。
        let src_id = if is_test {
            None
        } else {
            let outcome = parse_source_target_annotations(attrs);
            for (code, message) in outcome.diagnostics {
                let diagnostic = if code.starts_with('E') {
                    Diagnostic::error(code, message)
                } else {
                    Diagnostic::warning(code, message)
                };
                self.diagnostics
                    .push(diagnostic.with_location(location.clone()));
            }
            outcome.src_id
        };
        self.sources.push(SourceDraft {
            locator: RustLocator {
                path: relative.to_owned(),
                item_path: item_path.to_owned(),
            }
            .to_locator(),
            src_id,
            location: location.clone(),
            construct_text: content.to_owned(),
        });

        if !is_test {
            return Ok(());
        }
        let Some(annotation) = parse_test_annotations(attrs) else {
            self.diagnostics.push(
                Diagnostic::warning(
                    "W-SCAN-101",
                    format!("test function `{function_name}` has no @vtest annotation"),
                )
                .with_location(location),
            );
            return Ok(());
        };
        if !annotation.diagnostics.is_empty() {
            // 本冊 §4.4: adapter固有のsource declarationを構文解析できない
            // 場合、adapterは当該Test constructを管理宣言欠落として扱い、
            // Test Entityを具体化しない（診断だけを付与する）。
            for (code, message) in annotation.diagnostics {
                self.diagnostics
                    .push(Diagnostic::error(code, message).with_location(location.clone()));
            }
            return Ok(());
        }
        let Some(id) = annotation.id.filter(|value| !value.is_empty()) else {
            self.diagnostics.push(
                Diagnostic::error(
                    "E-SCAN-007",
                    format!("test `{function_name}` is missing required @vtest.id"),
                )
                .with_location(location),
            );
            return Ok(());
        };
        let Some(covers) = annotation.covers.filter(|value| !value.is_empty()) else {
            self.diagnostics.push(
                Diagnostic::error(
                    "E-SCAN-007",
                    format!("test `{function_name}` is missing required @vtest.covers"),
                )
                .with_location(location),
            );
            return Ok(());
        };
        let target_values = annotation.targets;
        if target_values.is_empty() || target_values.iter().any(|value| value.is_empty()) {
            self.diagnostics.push(
                Diagnostic::error(
                    "E-SCAN-007",
                    format!("test `{function_name}` is missing required @vtest.target"),
                )
                .with_location(location),
            );
            return Ok(());
        }
        let Some(intent) = annotation.intent.filter(|value| !value.is_empty()) else {
            self.diagnostics.push(
                Diagnostic::error(
                    "E-SCAN-007",
                    format!("test `{function_name}` is missing required @vtest.intent"),
                )
                .with_location(location),
            );
            return Ok(());
        };
        let test_id = TestId::new(id.clone());
        // 本冊:571「VO参照の解決とTest IDの大局的一意性はadapterではなく
        // coreが検査する」。ここで duplicate id を落とさず、core（全adapter
        // 統合後）に判定を委ねる。同じ理由で `covers` の VO 参照解決
        // （E-SCAN-003）もここでは行わない。
        if matches!(test_target, TestTarget::Unknown) {
            self.diagnostics.push(
                Diagnostic::error(
                    "E-SCAN-004",
                    format!("test `{id}` Cargo test target cannot be resolved"),
                )
                .with_location(location.clone()),
            );
        }
        let covers = covers
            .split(',')
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(VoId::new)
            .collect::<Vec<_>>();
        if covers.is_empty() {
            // 本冊:567（§4.4）「これらの必須metadata（core の `id` /
            // `covers ≥ 1` / `intent`...）を欠く場合は E-SCAN-007 とし、
            // `ManagedTestLink::Missing`（`chain_integrity` の `MISMATCH`、
            // 診断 `MISSING`）とする」。基本:412（§12）「構造上完全とは...
            // 1 件以上の `covers`...を Test Entity として具体化できること」。
            // `@vtest.covers ,` のように非空文字列だが実質0件の場合も同じ
            // Missing 扱いとし、`TestDraft` を生成しない（他の必須
            // metadata 欠落と同じ早期 return）。
            self.diagnostics.push(
                Diagnostic::error(
                    "E-SCAN-007",
                    format!("test `{id}` has no VO in @vtest.covers"),
                )
                .with_location(location.clone()),
            );
            return Ok(());
        }
        let mut targets = target_values
            .iter()
            .map(|target_value| {
                if let Some(src_id) = target_value.strip_prefix("SRC-") {
                    TargetRef::SrcId(SrcId::new(format!("SRC-{src_id}")))
                } else {
                    // 本冊:955/959/961（§6.1）・pr3-decisions.md Owner裁定2:
                    // Target Reference の解決（0件／複数件／曖昧の判定と
                    // 診断発行）はcoreの単一経路が所有し、adapterが独自に
                    // 候補を選んで「解決済み」を偽装したり、解決できない
                    // ことを表すために架空の locator を作ってはならない。
                    // ここでは宣言された文字列をそのまま opaque
                    // `TargetRef::Locator.value` として core へ渡すだけで
                    // ある — `rust-cargo` の構文として解釈できればその
                    // 正規化値（`RustLocator::to_value`）を、できなければ
                    // 宣言値をそのまま使う。どちらの場合も捏造した値は
                    // 含まない。実在する Source Target と一致するかどうか
                    // は core の SRC 索引（`vtest-scan::resolve_targets`）
                    // が完全一致で判定し、0件ヒットならE-SCAN-004とする。
                    TargetRef::Locator(locator_from_declared_value(target_value))
                }
            })
            .collect::<Vec<_>>();
        let target = targets.remove(0);
        self.tests.push(TestDraft {
            id: test_id,
            covers,
            target,
            additional_targets: targets,
            intent: intent.clone(),
            input: annotation.input,
            expect: annotation.expect,
            kind: annotation.kind,
            cases: annotation.cases,
            related: annotation
                .related
                .into_iter()
                .flat_map(|value| {
                    value
                        .split(',')
                        .map(str::trim)
                        .filter(|item| !item.is_empty())
                        .map(TestId::new)
                        .collect::<Vec<_>>()
                })
                .collect(),
            location,
            construct_text: content.to_owned(),
            filter: join_module_path(filter_prefix, item_path),
            package: package.to_owned(),
            test_target: test_target.clone(),
        });
        Ok(())
    }

    fn finish(self, files: usize) -> DiscoveryOutcome {
        DiscoveryOutcome {
            files_scanned: files,
            tests: self.tests,
            sources: self.sources,
            diagnostics: self.diagnostics,
        }
    }
}

/// 本冊 §4.2 test-annotation-line 文法が認識する test-key の全集合。
const TEST_KEYS: &[&str] = &[
    "id", "covers", "target", "intent", "input", "expect", "kind", "case", "related",
];

/// Test construct（表面1）の doc comment を正規化した結果。
///
/// `diagnostics` が空でない場合、この宣言は構文上有効な Test Entity へ
/// 正規化できない（本冊 §4.4）。呼び出し側は診断だけを記録し、
/// Test Entity を具体化してはならない。
struct TestAnnotationOutcome {
    id: Option<String>,
    covers: Option<String>,
    targets: Vec<String>,
    intent: Option<String>,
    input: Option<String>,
    expect: Option<String>,
    kind: Option<String>,
    cases: Vec<String>,
    related: Vec<String>,
    diagnostics: Vec<(String, String)>,
}

/// doc comment（`///` / `/** */`）から `@vtest.` 行を出現順に抽出する。
/// キーの妥当性は判定しない — 表面1・表面2どちらの文法にも共通する
/// 字句段階の処理であり、`@vtest.` を含まない行は自由記述として捨てる
/// （本冊 §4.2「doc comment 内の `@vtest.` を含まない行は自由記述として
/// 無視する」）。
fn vtest_annotation_lines(attrs: &[Attribute]) -> Vec<(String, String)> {
    let mut lines = Vec::new();
    for attr in attrs {
        if !attr.path().is_ident("doc") {
            continue;
        }
        let Meta::NameValue(value) = &attr.meta else {
            continue;
        };
        let Expr::Lit(ExprLit {
            lit: Lit::Str(text),
            ..
        }) = &value.value
        else {
            continue;
        };
        lines.extend(text.value().lines().map(|line| line.trim().to_owned()));
    }
    lines
        .into_iter()
        .filter_map(|line| {
            let annotation = line.strip_prefix("@vtest.")?;
            let (key, value) = if let Some(separator) = annotation.find(char::is_whitespace) {
                annotation.split_at(separator)
            } else {
                (annotation, "")
            };
            Some((key.trim().to_owned(), value.trim().to_owned()))
        })
        .collect()
}

/// 表面1（Test construct の doc comment）の `@vtest.` 宣言を本冊 §4.2 の
/// test-annotation-line 文法で解析する。`@vtest.` 行が1件も無ければ
/// `None`（呼び出し側は W-SCAN-101 の判定に使う）。
fn parse_test_annotations(attrs: &[Attribute]) -> Option<TestAnnotationOutcome> {
    let lines = vtest_annotation_lines(attrs);
    if lines.is_empty() {
        return None;
    }
    let mut diagnostics = Vec::new();
    let mut single = std::collections::BTreeMap::<String, String>::new();
    let mut cases = Vec::new();
    let mut related = Vec::new();
    let mut targets = Vec::new();
    for (key, value) in lines {
        if !TEST_KEYS.contains(&key.as_str()) {
            // 本冊 §4.2: 表面1で test-key を持たない行は未知キーとする
            // （打鍵ミス検出を優先し、警告ではなくエラーとする）。
            // source-target-key（`src-id`）の誤配置もここに含む。
            diagnostics.push((
                "E-SCAN-006".to_owned(),
                format!("unrecognized @vtest key `{key}` on a Test construct"),
            ));
            continue;
        }
        match key.as_str() {
            "case" => cases.push(value),
            "related" => related.push(value),
            "target" => targets.push(value),
            _ => {
                if single.insert(key.clone(), value).is_some() {
                    diagnostics.push((
                        "E-SCAN-005".to_owned(),
                        format!("duplicate annotation key `{key}`"),
                    ));
                }
            }
        }
    }
    // 本冊 §4.2: `kind` が integration 系の Test に限り `target` の複数行を
    // 許容する。それ以外のキーの重複は常にエラー。
    let integration = single
        .get("kind")
        .is_some_and(|kind| kind.starts_with("integration"));
    if targets.len() > 1 && !integration {
        diagnostics.push((
            "E-SCAN-005".to_owned(),
            "duplicate annotation key `target`".to_owned(),
        ));
    } else if targets.len() > 1 {
        // 許容された複数 `target` 内でも同じ値の重複は E-SCAN-005 とする。
        // 綴りが異なるが解決後に同一 canonical Source Target へ到達する
        // 場合の検出は core の Target Reference 解決（§6.1）が担い、この
        // 段階（宣言表面の解析）では扱わない。
        let mut seen = BTreeSet::new();
        for value in &targets {
            if !seen.insert(value.as_str()) {
                diagnostics.push((
                    "E-SCAN-005".to_owned(),
                    format!("duplicate target `{value}`"),
                ));
            }
        }
    }
    Some(TestAnnotationOutcome {
        id: single.remove("id"),
        covers: single.remove("covers"),
        targets,
        intent: single.remove("intent"),
        input: single.remove("input"),
        expect: single.remove("expect"),
        kind: single.remove("kind"),
        cases,
        related,
        diagnostics,
    })
}

fn is_test_function(attrs: &[Attribute]) -> bool {
    attrs.iter().any(|attr| {
        attr.path()
            .segments
            .last()
            .is_some_and(|segment| segment.ident == "test")
    })
}

/// 表面2（Test construct ではない関数 item の doc comment）の解析結果。
struct SourceTargetAnnotationOutcome {
    src_id: Option<SrcId>,
    diagnostics: Vec<(String, String)>,
}

/// 表面2の `@vtest.` 宣言を本冊 §4.2 の source-target-annotation-line 文法
/// で解析する。認識するキーは `src-id` のみ。
fn parse_source_target_annotations(attrs: &[Attribute]) -> SourceTargetAnnotationOutcome {
    let mut diagnostics = Vec::new();
    let mut declared = Vec::new();
    for (key, value) in vtest_annotation_lines(attrs) {
        if key == "src-id" {
            declared.push(value);
            continue;
        }
        // 本冊 §4.2: 表面2で `@vtest.` 行が source-target-key を持たない
        // （test-key を含む）場合は警告とする。表面2の宣言は Test metadata
        // を破損させず採用値の曖昧さも生まないため、error ではなく
        // warning とする。
        diagnostics.push((
            "W-SCAN-105".to_owned(),
            format!("unrecognized @vtest key `{key}` on a non-test item"),
        ));
    }
    let src_id = match declared.len() {
        0 => None,
        1 => declared.into_iter().next().map(SrcId::new),
        _ => {
            // 本冊 §4.2: `src-id` は表面2でも反復不可。同一関数 item での
            // 重複は採用すべき ID を決定できないため、いずれの宣言値も
            // 採用せず SRC ID は無しとして扱う（どちらかを推測で選ばない）。
            diagnostics.push((
                "E-SCAN-005".to_owned(),
                "duplicate annotation key `src-id`".to_owned(),
            ));
            None
        }
    };
    SourceTargetAnnotationOutcome {
        src_id,
        diagnostics,
    }
}

struct SourceContext {
    package: String,
    test_target: TestTarget,
    filter_prefix: String,
}

#[derive(Clone, Debug)]
struct CargoTargetRoot {
    path: PathBuf,
    target: TestTarget,
}

#[derive(Debug, Deserialize)]
struct CargoManifest {
    package: Option<CargoPackage>,
    lib: Option<CargoTarget>,
    #[serde(default)]
    bin: Vec<CargoTarget>,
    #[serde(default)]
    test: Vec<CargoTarget>,
}

#[derive(Debug, Deserialize)]
struct CargoPackage {
    name: String,
    autobins: Option<bool>,
    autotests: Option<bool>,
}

#[derive(Debug, Deserialize)]
struct CargoTarget {
    name: Option<String>,
    path: Option<String>,
}

// レビュー round 2 項目【L】掃引: このサイトは PR3 round 2 の対象外だが、
// 失われる情報を明記する — 二連の `.ok()?` は「`Cargo.toml` が存在しない
// （package rootの外、または workspace root自体にpackageが無い正当な
// ケース）」と「`Cargo.toml` は存在するが壊れている（TOML構文エラー・
// `[package]` 欠落）」を区別しない。呼び出し元 `source_context` は
// どちらの場合も同じ `None` を受け取り、`fallback_package` へ黙って
// フォールバックする（package名・target種別 `TestTarget` の解決精度が
// 静かに低下する。診断は一切出ない）。
fn cargo_manifest(root: &Path) -> Option<CargoManifest> {
    let text = fs::read_to_string(root.join("Cargo.toml")).ok()?;
    let manifest = toml::from_str::<CargoManifest>(&text).ok()?;
    manifest.package.as_ref()?;
    Some(manifest)
}

fn source_context(
    root: &Path,
    path: &Path,
    fallback_package: &str,
) -> Result<SourceContext, DiscoveryError> {
    let package_root = package_root_for_path(root, path).unwrap_or_else(|| root.to_owned());
    let manifest = cargo_manifest(&package_root);
    let package = manifest
        .as_ref()
        .and_then(|manifest| manifest.package.as_ref())
        .map(|package| package.name.clone())
        .unwrap_or_else(|| fallback_package.to_owned());

    if let Some(manifest) = &manifest {
        let mut contexts = Vec::new();
        for target_root in cargo_target_roots(&package_root, manifest)? {
            for filter_prefix in module_prefixes_for_file(&target_root.path, path) {
                let context = (target_root.target.clone(), filter_prefix);
                if !contexts.contains(&context) {
                    contexts.push(context);
                }
            }
        }
        if contexts.len() == 1 {
            let (test_target, filter_prefix) = contexts.pop().expect("one context exists");
            return Ok(SourceContext {
                package,
                test_target,
                filter_prefix,
            });
        }
        return Ok(SourceContext {
            package,
            test_target: TestTarget::Unknown,
            filter_prefix: String::new(),
        });
    }
    Ok(SourceContext {
        package,
        test_target: TestTarget::Unknown,
        filter_prefix: String::new(),
    })
}

fn cargo_target_name(target: &CargoTarget) -> Option<String> {
    target
        .name
        .clone()
        .or_else(|| target.path.as_deref().and_then(target_name_from_path))
}

fn target_name_from_path(path: &str) -> Option<String> {
    let path = Path::new(path);
    let stem = path.file_stem()?.to_str()?;
    if matches!(stem, "main" | "mod") {
        path.parent()?.file_name()?.to_str().map(str::to_owned)
    } else {
        Some(stem.to_owned())
    }
}

fn normalized_manifest_path(path: &str) -> String {
    path.trim_start_matches("./").replace('\\', "/")
}

fn cargo_target_roots(
    package_root: &Path,
    manifest: &CargoManifest,
) -> Result<Vec<CargoTargetRoot>, DiscoveryError> {
    let mut roots = Vec::new();
    let lib_path = manifest
        .lib
        .as_ref()
        .map(|target| target.path.as_deref().unwrap_or("src/lib.rs"))
        .or_else(|| {
            package_root
                .join("src/lib.rs")
                .exists()
                .then_some("src/lib.rs")
        });
    if let Some(path) = lib_path {
        roots.push(CargoTargetRoot {
            path: package_root.join(normalized_manifest_path(path)),
            target: TestTarget::Lib,
        });
    }

    let mut explicit_bins = Vec::new();
    for binary in &manifest.bin {
        let Some(name) = cargo_target_name(binary) else {
            continue;
        };
        for path in explicit_target_paths(package_root, binary, "src/bin", &name, true) {
            explicit_bins.push(path.clone());
            roots.push(CargoTargetRoot {
                path,
                target: TestTarget::Bin(name.clone()),
            });
        }
    }

    let autobins = manifest
        .package
        .as_ref()
        .and_then(|package| package.autobins)
        .unwrap_or(true);
    if autobins {
        let main = package_root.join("src/main.rs");
        if main.exists() && !contains_path(&explicit_bins, &main) {
            let name = manifest
                .package
                .as_ref()
                .map(|package| package.name.clone())
                .unwrap_or_default();
            roots.push(CargoTargetRoot {
                path: main,
                target: TestTarget::Bin(name),
            });
        }
        for (path, name) in discovered_target_roots(&package_root.join("src/bin"))? {
            if !contains_path(&explicit_bins, &path) {
                roots.push(CargoTargetRoot {
                    path,
                    target: TestTarget::Bin(name),
                });
            }
        }
    }

    let mut explicit_tests = Vec::new();
    for test in &manifest.test {
        let Some(name) = cargo_target_name(test) else {
            continue;
        };
        for path in explicit_target_paths(package_root, test, "tests", &name, true) {
            explicit_tests.push(path.clone());
            roots.push(CargoTargetRoot {
                path,
                target: TestTarget::IntegrationTest(name.clone()),
            });
        }
    }

    let autotests = manifest
        .package
        .as_ref()
        .and_then(|package| package.autotests)
        .unwrap_or(true);
    if autotests {
        for (path, name) in discovered_target_roots(&package_root.join("tests"))? {
            if !contains_path(&explicit_tests, &path) {
                roots.push(CargoTargetRoot {
                    path,
                    target: TestTarget::IntegrationTest(name),
                });
            }
        }
    }
    Ok(roots)
}

fn explicit_target_paths(
    package_root: &Path,
    target: &CargoTarget,
    default_directory: &str,
    name: &str,
    allow_directory_main: bool,
) -> Vec<PathBuf> {
    if let Some(path) = &target.path {
        return vec![package_root.join(normalized_manifest_path(path))];
    }
    let mut candidates = vec![package_root.join(format!("{default_directory}/{name}.rs"))];
    if allow_directory_main {
        candidates.push(package_root.join(format!("{default_directory}/{name}/main.rs")));
    }
    let existing = candidates
        .iter()
        .filter(|path| path.exists())
        .cloned()
        .collect::<Vec<_>>();
    if existing.is_empty() {
        candidates.truncate(1);
        candidates
    } else {
        existing
    }
}

// PR #26 review round 3 要確認C-1: `entries.flatten()` silently dropped both
// "directory doesn't exist" (the common, legitimate case — most packages
// have no `src/bin` or `tests`) and "directory exists but a `DirEntry`
// failed mid-enumeration" (an I/O error) into the same empty result. The
// latter is the same fail-open class `d423f8a` closed for
// `validate_relations`/`validate_approval_status` in `vtest-scan`: a root
// that silently drops out here degrades `source_context`'s `TestTarget`/
// `filter_prefix` attribution for files under it without any diagnostic.
// Distinguish the two the same way: `NotFound` still reads as "no such
// directory" (empty, not an error); every other `read_dir`/`DirEntry`
// error now fails discovery closed via `DiscoveryError` (core turns this
// into `ScanError::Discovery` / `E-ADAPTER-002`), instead of guessing.
fn discovered_target_roots(directory: &Path) -> Result<Vec<(PathBuf, String)>, DiscoveryError> {
    let entries = match fs::read_dir(directory) {
        Ok(entries) => entries,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(source) => {
            return Err(DiscoveryError {
                path: directory.to_owned(),
                message: source.to_string(),
            })
        }
    };
    let mut paths = Vec::new();
    for entry in entries {
        let entry = entry.map_err(|source| DiscoveryError {
            path: directory.to_owned(),
            message: source.to_string(),
        })?;
        paths.push(entry.path());
    }
    paths.sort();
    let mut roots = Vec::new();
    for path in paths {
        if path.is_file() && path.extension().and_then(|value| value.to_str()) == Some("rs") {
            if let Some(name) = path
                .file_stem()
                .and_then(|value| value.to_str())
                .map(str::to_owned)
            {
                roots.push((path, name));
            }
        } else if path.is_dir() {
            let main = path.join("main.rs");
            if main.exists() {
                if let Some(name) = path
                    .file_name()
                    .and_then(|value| value.to_str())
                    .map(str::to_owned)
                {
                    roots.push((main, name));
                }
            }
        }
    }
    Ok(roots)
}

fn contains_path(paths: &[PathBuf], candidate: &Path) -> bool {
    paths.iter().any(|path| same_path(path, candidate))
}

fn same_path(left: &Path, right: &Path) -> bool {
    match (fs::canonicalize(left), fs::canonicalize(right)) {
        (Ok(left), Ok(right)) => left == right,
        _ => left == right,
    }
}

fn module_prefixes_for_file(target_root: &Path, sought: &Path) -> Vec<String> {
    let Some(module_directory) = target_root.parent() else {
        return Vec::new();
    };
    let mut prefixes = Vec::new();
    let mut visiting = BTreeSet::new();
    visit_module_file(
        target_root,
        module_directory,
        "",
        sought,
        &mut visiting,
        &mut prefixes,
    );
    prefixes.sort();
    prefixes.dedup();
    prefixes
}

fn visit_module_file(
    file: &Path,
    module_directory: &Path,
    prefix: &str,
    sought: &Path,
    visiting: &mut BTreeSet<PathBuf>,
    prefixes: &mut Vec<String>,
) {
    if same_path(file, sought) {
        prefixes.push(prefix.to_owned());
    }
    let identity = fs::canonicalize(file).unwrap_or_else(|_| file.to_owned());
    if !visiting.insert(identity.clone()) {
        return;
    }
    // レビュー round 2 項目【L】掃引: このサイトは PR3 round 2 の対象外
    // だが、失われる情報を明記する — read 失敗（権限エラー等）と構文
    // エラーはどちらも `None` に潰れ、区別されない。結果として、この
    // module ファイル配下の部分木がモジュールパス解決用の prefix 列挙
    // （`prefixes`）から黙って欠落する。呼び出し元は「この module に
    // `sought` は存在しない」と「この module は読めなかった／構文が
    // 壊れている」を区別できない。
    let syntax = fs::read_to_string(file)
        .ok()
        .and_then(|source| syn::parse_file(&source).ok());
    if let Some(syntax) = syntax {
        visit_module_items(
            &syntax.items,
            module_directory,
            prefix,
            sought,
            visiting,
            prefixes,
        );
    }
    visiting.remove(&identity);
}

fn visit_module_items(
    items: &[Item],
    module_directory: &Path,
    prefix: &str,
    sought: &Path,
    visiting: &mut BTreeSet<PathBuf>,
    prefixes: &mut Vec<String>,
) {
    for item in items {
        let Item::Mod(module) = item else {
            continue;
        };
        let name = module.ident.to_string();
        let child_prefix = join_module_path(prefix, &name);
        let child_directory = module_directory.join(&name);
        if let Some((_, items)) = &module.content {
            visit_module_items(
                items,
                &child_directory,
                &child_prefix,
                sought,
                visiting,
                prefixes,
            );
            continue;
        }
        let candidates = [
            module_directory.join(format!("{name}.rs")),
            child_directory.join("mod.rs"),
        ];
        let existing = candidates
            .iter()
            .filter(|path| path.exists())
            .collect::<Vec<_>>();
        if existing.len() == 1 {
            visit_module_file(
                existing[0],
                &child_directory,
                &child_prefix,
                sought,
                visiting,
                prefixes,
            );
        }
    }
}

fn package_root_for_path(root: &Path, path: &Path) -> Option<PathBuf> {
    let mut current = path.parent();
    while let Some(directory) = current {
        if directory.join("Cargo.toml").exists() {
            return Some(directory.to_owned());
        }
        if directory == root {
            break;
        }
        current = directory.parent();
    }
    None
}

fn join_module_path(prefix: &str, item_path: &str) -> String {
    if prefix.is_empty() {
        item_path.to_owned()
    } else {
        format!("{prefix}::{item_path}")
    }
}

fn line_offsets(source: &str) -> Vec<usize> {
    let mut offsets = vec![0];
    for (index, byte) in source.as_bytes().iter().enumerate() {
        if *byte == b'\n' {
            offsets.push(index + 1);
        }
    }
    offsets
}

fn make_location(
    relative: &str,
    function: &str,
    span: proc_macro2::Span,
    source: &str,
    offsets: &[usize],
) -> SourceLocation {
    let start = span.start();
    let end = span.end();
    let start_line = start.line.max(1);
    let end_line = end.line.max(start_line);
    let start_byte = offsets.get(start_line - 1).copied().unwrap_or(0) + start.column;
    let end_byte = offsets.get(end_line - 1).copied().unwrap_or(source.len()) + end.column;
    SourceLocation {
        file: relative.to_owned(),
        function: function.to_owned(),
        start_line: start_line as u64,
        end_line: end_line as u64,
        start_byte: start_byte as u64,
        end_byte: end_byte.min(source.len()) as u64,
    }
}

fn source_slice<'a>(source: &'a str, location: &SourceLocation) -> Option<&'a str> {
    let start: usize = location.start_byte.try_into().ok()?;
    let end: usize = location.end_byte.try_into().ok()?;
    source.get(start..end)
}

fn record_location(root: &Path, path: &Path, entity: &str) -> SourceLocation {
    // レビュー round 2 項目【L】掃引: `unwrap_or_default()` は read 失敗
    // （権限エラー等）を空文字列と同じに扱う。このサイトは PR3 round 2 の
    // 対象外だが、失われる情報を明記する — read が失敗すると呼び出し元の
    // 診断に付く `SourceLocation.end_line` / `end_byte` が実ファイルの
    // 実測値ではなく `1` / `0` へ退化し、read 失敗そのものは診断として
    // 一切報告されない（`vtest-scan::record_location` の同型サイトと同じ）。
    let text = fs::read_to_string(path).unwrap_or_default();
    SourceLocation {
        file: path
            .strip_prefix(root)
            .unwrap_or(path)
            .to_string_lossy()
            .replace('\\', "/"),
        function: entity.to_owned(),
        start_line: 1,
        end_line: text.lines().count().max(1) as u64,
        start_byte: 0,
        end_byte: text.len() as u64,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use vtest_adapter_api::AdapterRegistry;

    /// `AdapterRegistry` は `vtest-adapter-api` が所有する契約だが、
    /// registry の「登録済み ID は解決でき、未登録 ID は解決できない」動作
    /// を production adapter の実 ID（`"rust-cargo"`）で確認するにはこの
    /// crate が必要になる。`vtest-scan`（core）はこの解決結果を使って未知
    /// adapter ID を fail-closed で拒否する（`vtest-scan::lib::tests::
    /// unknown_adapter_id_is_rejected_fail_closed`）。
    fn registry_with_rust_cargo() -> AdapterRegistry {
        let mut registry = AdapterRegistry::new();
        registry.register(Box::new(RustCargoAdapter::new()));
        registry
    }

    #[test]
    fn registered_adapter_id_resolves() {
        let registry = registry_with_rust_cargo();
        assert!(registry.get("rust-cargo").is_some());
    }

    #[test]
    fn unregistered_adapter_id_does_not_resolve() {
        let registry = registry_with_rust_cargo();
        assert!(registry.get("unknown-lang").is_none());
    }

    #[test]
    fn ids_lists_every_registered_adapter() {
        let registry = registry_with_rust_cargo();
        assert_eq!(registry.ids().collect::<Vec<_>>(), vec!["rust-cargo"]);
    }

    #[test]
    fn rust_locator_splits_at_the_first_separator() {
        let locator = RustLocator::parse("src/lib.rs::module::function").expect("valid locator");
        assert_eq!(locator.path, "src/lib.rs");
        assert_eq!(locator.item_path, "module::function");
    }

    #[test]
    fn rust_locator_rejects_values_without_an_rs_path() {
        assert!(RustLocator::parse("not-a-path::item").is_none());
        assert!(RustLocator::parse("src/lib.rs").is_none());
    }

    #[test]
    fn locator_from_declared_value_normalizes_a_parseable_value() {
        let locator = locator_from_declared_value(r"src\lib.rs::module::function");
        assert_eq!(locator.adapter.as_str(), ADAPTER_ID);
        assert_eq!(locator.value, "src/lib.rs::module::function");
    }

    /// pr3-decisions.md Owner裁定2「架空locatorを作らない」: 構文解析できな
    /// い宣言値は、捏造した代替値ではなく宣言値そのものを opaque `value` と
    /// して運ぶ。
    #[test]
    fn locator_from_declared_value_passes_through_an_unparseable_value_verbatim() {
        let locator = locator_from_declared_value("this is not a locator");
        assert_eq!(locator.adapter.as_str(), ADAPTER_ID);
        assert_eq!(locator.value, "this is not a locator");
    }
}
