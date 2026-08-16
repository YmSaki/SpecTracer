//! Rust-specific source-location helpers extracted from the scanner. These move
//! to `vtest-adapter-rust` together with the rest of the Rust discovery code
//! (the scanner, Cargo manifest and module resolution, annotation parsing);
//! grouping the language-specific surface into one module isolates it first so
//! the cross-crate move is mechanical.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

use ignore::WalkBuilder;
use serde::Deserialize;
use syn::spanned::Spanned;
use syn::{Attribute, Expr, ExprLit, ImplItem, Item, ItemFn, ItemImpl, Lit, Meta};
use vtest_adapter_api::{
    AdapterError, DiscoveredTestDraft, DiscoveryBatch, DiscoveryCompleteness, ManagedTestDraft,
    ManagedTestDraftLink, SourceDiscoveryAdapter, SourceFragment, SourceTargetDraft,
};
use vtest_model::{
    AdapterId, CanonicalProjection, Diagnostic, ExecutionDescriptor, ProjectPath, SourceLocation,
    SourceRange, SrcId, TargetRef, TestId, TestSuite, VoId,
};

const RUST_ADAPTER_ID: &str = "rust-cargo";

/// Builds a file-level `SourceLocation` for adapter diagnostics that point at a
/// whole file (read/parse failures) rather than a specific construct.
fn file_location(root: &Path, path: &Path, entity: &str) -> SourceLocation {
    let text = fs::read_to_string(path).unwrap_or_default();
    SourceLocation {
        adapter: AdapterId::new("core-record"),
        path: ProjectPath::new(
            path.strip_prefix(root)
                .unwrap_or(path)
                .to_string_lossy()
                .replace('\\', "/"),
        ),
        locator: entity.to_owned(),
        byte_range: SourceRange {
            start: 0,
            end: text.len(),
            start_line: 1,
            end_line: text.lines().count().max(1),
        },
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Locator {
    pub path: String,
    pub item_path: String,
}

impl Locator {
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

    pub fn as_string(&self) -> String {
        format!("{}::{}", self.path, self.item_path)
    }

    pub fn as_target(&self) -> TargetRef {
        TargetRef::Locator {
            adapter: AdapterId::new(RUST_ADAPTER_ID),
            value: self.as_string(),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum TestTarget {
    Lib,
    Bin(String),
    IntegrationTest(String),
    Unknown,
}

pub(crate) struct ParsedAnnotations {
    pub(crate) values: BTreeMap<String, String>,
    pub(crate) repeated: BTreeMap<String, Vec<String>>,
    /// Every unrecognized-key (E-SCAN-006) and repeated-non-repeatable-key
    /// (E-SCAN-005) defect found while scanning the declaration, in the
    /// order encountered. A declaration can carry more than one of either
    /// kind; each one is its own defect and must survive to a diagnostic
    /// (詳細設計 §5.4 — E-SCAN-005/006 are per-defect, not per-declaration).
    pub(crate) parse_errors: Vec<AnnotationParseError>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum AnnotationParseError {
    UnknownKey(String),
    DuplicateKey(String),
}

pub(crate) fn parse_annotations(attrs: &[Attribute]) -> Option<ParsedAnnotations> {
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
    if !lines.iter().any(|line| line.contains("@vtest.")) {
        return None;
    }
    let mut values = BTreeMap::new();
    let mut repeated = BTreeMap::<String, Vec<String>>::new();
    const KNOWN: &[&str] = &[
        "id", "covers", "target", "intent", "input", "expect", "kind", "case", "related", "src-id",
    ];
    // Every unknown-key and duplicate-key defect is collected, not just the
    // first or the most recent: a declaration carrying more than one such
    // defect (of the same kind or of both kinds) must surface all of them.
    let mut parse_errors = Vec::new();
    for line in lines {
        let Some(annotation) = line.strip_prefix("@vtest.") else {
            continue;
        };
        let (key, value) = if let Some(separator) = annotation.find(char::is_whitespace) {
            annotation.split_at(separator)
        } else {
            (annotation, "")
        };
        let key = key.trim().to_owned();
        let value = value.trim().to_owned();
        if !KNOWN.contains(&key.as_str()) {
            parse_errors.push(AnnotationParseError::UnknownKey(key));
            continue;
        }
        if matches!(key.as_str(), "case" | "related" | "target") {
            repeated.entry(key).or_default().push(value);
        } else if values.insert(key.clone(), value).is_some() {
            parse_errors.push(AnnotationParseError::DuplicateKey(key));
        }
    }
    Some(ParsedAnnotations {
        values,
        repeated,
        parse_errors,
    })
}

pub(crate) fn is_test_function(attrs: &[Attribute]) -> bool {
    attrs.iter().any(|attr| {
        attr.path()
            .segments
            .last()
            .is_some_and(|segment| segment.ident == "test")
    })
}

pub(crate) fn parse_src_id(attrs: &[Attribute]) -> Option<SrcId> {
    parse_annotations(attrs)
        .and_then(|annotations| annotations.values.get("src-id").cloned())
        .map(SrcId::new)
}

pub(crate) fn join_module_path(prefix: &str, item_path: &str) -> String {
    if prefix.is_empty() {
        item_path.to_owned()
    } else {
        format!("{prefix}::{item_path}")
    }
}

pub(crate) fn line_offsets(source: &str) -> Vec<usize> {
    let mut offsets = vec![0];
    for (index, byte) in source.as_bytes().iter().enumerate() {
        if *byte == b'\n' {
            offsets.push(index + 1);
        }
    }
    offsets
}

pub(crate) fn make_location(
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
        adapter: AdapterId::new(RUST_ADAPTER_ID),
        path: ProjectPath::new(relative),
        locator: function.to_owned(),
        byte_range: SourceRange {
            start: start_byte,
            end: end_byte.min(source.len()),
            start_line,
            end_line,
        },
    }
}

pub(crate) fn source_slice<'a>(source: &'a str, location: &SourceLocation) -> &'a str {
    source
        .get(location.byte_range.start..location.byte_range.end)
        .unwrap_or("")
}

pub(crate) struct SourceContext {
    pub(crate) package: String,
    pub(crate) test_target: TestTarget,
    pub(crate) filter_prefix: String,
}

#[derive(Clone, Debug)]
pub(crate) struct CargoTargetRoot {
    path: PathBuf,
    target: TestTarget,
}

#[derive(Debug, Deserialize)]
pub(crate) struct CargoManifest {
    package: Option<CargoPackage>,
    lib: Option<CargoTarget>,
    #[serde(default)]
    bin: Vec<CargoTarget>,
    #[serde(default)]
    test: Vec<CargoTarget>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct CargoPackage {
    name: String,
    autobins: Option<bool>,
    autotests: Option<bool>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct CargoTarget {
    name: Option<String>,
    path: Option<String>,
}

pub(crate) fn cargo_manifest(root: &Path) -> Option<CargoManifest> {
    let text = fs::read_to_string(root.join("Cargo.toml")).ok()?;
    let manifest = toml::from_str::<CargoManifest>(&text).ok()?;
    manifest.package.as_ref()?;
    Some(manifest)
}

pub(crate) fn source_context(root: &Path, path: &Path, fallback_package: &str) -> SourceContext {
    let package_root = package_root_for_path(root, path).unwrap_or_else(|| root.to_owned());
    let manifest = cargo_manifest(&package_root);
    let package = manifest
        .as_ref()
        .and_then(|manifest| manifest.package.as_ref())
        .map(|package| package.name.clone())
        .unwrap_or_else(|| fallback_package.to_owned());

    if let Some(manifest) = &manifest {
        let mut contexts = Vec::new();
        for target_root in cargo_target_roots(&package_root, manifest) {
            for filter_prefix in module_prefixes_for_file(&target_root.path, path) {
                let context = (target_root.target.clone(), filter_prefix);
                if !contexts.contains(&context) {
                    contexts.push(context);
                }
            }
        }
        if contexts.len() == 1 {
            let (test_target, filter_prefix) = contexts.pop().expect("one context exists");
            return SourceContext {
                package,
                test_target,
                filter_prefix,
            };
        }
        return SourceContext {
            package,
            test_target: TestTarget::Unknown,
            filter_prefix: String::new(),
        };
    }
    SourceContext {
        package,
        test_target: TestTarget::Unknown,
        filter_prefix: String::new(),
    }
}

pub(crate) fn cargo_target_name(target: &CargoTarget) -> Option<String> {
    target
        .name
        .clone()
        .or_else(|| target.path.as_deref().and_then(target_name_from_path))
}

pub(crate) fn target_name_from_path(path: &str) -> Option<String> {
    let path = Path::new(path);
    let stem = path.file_stem()?.to_str()?;
    if matches!(stem, "main" | "mod") {
        path.parent()?.file_name()?.to_str().map(str::to_owned)
    } else {
        Some(stem.to_owned())
    }
}

pub(crate) fn normalized_manifest_path(path: &str) -> String {
    path.trim_start_matches("./").replace('\\', "/")
}

pub(crate) fn cargo_target_roots(
    package_root: &Path,
    manifest: &CargoManifest,
) -> Vec<CargoTargetRoot> {
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
        for (path, name) in discovered_target_roots(&package_root.join("src/bin")) {
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
        for (path, name) in discovered_target_roots(&package_root.join("tests")) {
            if !contains_path(&explicit_tests, &path) {
                roots.push(CargoTargetRoot {
                    path,
                    target: TestTarget::IntegrationTest(name),
                });
            }
        }
    }
    roots
}

pub(crate) fn explicit_target_paths(
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

pub(crate) fn discovered_target_roots(directory: &Path) -> Vec<(PathBuf, String)> {
    let Ok(entries) = fs::read_dir(directory) else {
        return Vec::new();
    };
    let mut entries = entries
        .flatten()
        .map(|entry| entry.path())
        .collect::<Vec<_>>();
    entries.sort();
    let mut roots = Vec::new();
    for path in entries {
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
    roots
}

pub(crate) fn contains_path(paths: &[PathBuf], candidate: &Path) -> bool {
    paths.iter().any(|path| same_path(path, candidate))
}

pub(crate) fn same_path(left: &Path, right: &Path) -> bool {
    match (fs::canonicalize(left), fs::canonicalize(right)) {
        (Ok(left), Ok(right)) => left == right,
        _ => left == right,
    }
}

pub(crate) fn module_prefixes_for_file(target_root: &Path, sought: &Path) -> Vec<String> {
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

pub(crate) fn visit_module_file(
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

pub(crate) fn visit_module_items(
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

pub(crate) fn package_root_for_path(root: &Path, path: &Path) -> Option<PathBuf> {
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

pub(crate) fn package_name(root: &Path) -> Option<String> {
    cargo_manifest(root).and_then(|manifest| manifest.package.map(|package| package.name))
}

pub(crate) fn projection_string(projection: &CanonicalProjection, key: &str) -> Option<String> {
    match projection {
        CanonicalProjection::Map(map) => match map.get(key) {
            Some(CanonicalProjection::String(value)) => Some(value.clone()),
            _ => None,
        },
        _ => None,
    }
}

pub(crate) fn projection_strings(projection: &CanonicalProjection, key: &str) -> Vec<String> {
    match projection {
        CanonicalProjection::Map(map) => match map.get(key) {
            Some(CanonicalProjection::List(values)) => values
                .iter()
                .filter_map(|value| match value {
                    CanonicalProjection::String(value) => Some(value.clone()),
                    _ => None,
                })
                .collect(),
            _ => Vec::new(),
        },
        _ => Vec::new(),
    }
}

/// Built-in `rust-cargo` source discovery. Reads the neutral projection and the
/// current filesystem and returns a hash-free `DiscoveryBatch`; core validates,
/// hashes, and materializes it. This is the boundary that moves wholesale to
/// `vtest-adapter-rust` in the next step.
#[derive(Debug, Default)]
pub struct RustCargoDiscovery;

impl SourceDiscoveryAdapter for RustCargoDiscovery {
    fn discover(
        &self,
        root: &Path,
        config: &CanonicalProjection,
    ) -> Result<DiscoveryBatch, AdapterError> {
        let fallback = projection_string(config, "package").unwrap_or_default();
        let package = package_name(root).unwrap_or(fallback);
        let mut paths = Vec::new();
        for include in projection_strings(config, "include") {
            let include_path = root.join(&include);
            collect_rs_files(root, &include_path, &mut paths).map_err(|error| {
                AdapterError::MalformedOutput(format!(
                    "cannot scan `{}`: {error}",
                    include_path.display()
                ))
            })?;
        }
        paths.sort();
        paths.dedup();
        let mut scanner = Scanner::new(root, &package);
        for path in &paths {
            scanner.scan_file(path);
        }
        Ok(scanner.finish())
    }
}

pub(crate) fn collect_rs_files(
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

pub(crate) struct Scanner<'a> {
    root: &'a Path,
    fallback_package: &'a str,
    discovered_tests: Vec<DiscoveredTestDraft>,
    source_targets: Vec<SourceTargetDraft>,
    diagnostics: Vec<Diagnostic>,
    test_ids: BTreeSet<String>,
}

impl<'a> Scanner<'a> {
    fn new(root: &'a Path, fallback_package: &'a str) -> Self {
        Self {
            root,
            fallback_package,
            discovered_tests: Vec::new(),
            source_targets: Vec::new(),
            diagnostics: Vec::new(),
            test_ids: BTreeSet::new(),
        }
    }

    fn scan_file(&mut self, path: &Path) {
        let file_name = path
            .file_name()
            .and_then(|value| value.to_str())
            .unwrap_or_default();
        let file_location = file_location(self.root, path, file_name);
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
                return;
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
                return;
            }
        };
        let relative = path
            .strip_prefix(self.root)
            .unwrap_or(path)
            .to_string_lossy()
            .replace('\\', "/");
        let context = source_context(self.root, path, self.fallback_package);
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
        );
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
    ) {
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
                ),
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
                ),
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
                        );
                    }
                }
                _ => {}
            }
        }
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
    ) {
        let type_name = match item_impl.self_ty.as_ref() {
            syn::Type::Path(value) => value.path.segments.last().map(|v| v.ident.to_string()),
            _ => None,
        };
        let Some(type_name) = type_name else { return };
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
            );
        }
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
    ) {
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
        );
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
        _path: &Path,
    ) {
        let location = make_location(relative, item_path, span, source, line_offsets);
        let content = source_slice(source, &location);
        let construct = SourceFragment {
            location: location.clone(),
            bytes: content.as_bytes().to_vec(),
        };
        self.source_targets.push(SourceTargetDraft {
            target: Locator {
                path: relative.to_owned(),
                item_path: item_path.to_owned(),
            }
            .as_target(),
            src_id: parse_src_id(attrs),
            location: location.clone(),
            construct: construct.clone(),
        });

        if !is_test_function(attrs) {
            return;
        }
        let Some(annotation) = parse_annotations(attrs) else {
            self.diagnostics.push(
                Diagnostic::warning(
                    "W-SCAN-101",
                    format!("test function `{function_name}` has no @vtest annotation"),
                )
                .with_location(location),
            );
            return;
        };
        if !annotation.parse_errors.is_empty() {
            // Every collected defect gets its own diagnostic — a declaration
            // with both an unknown key and a duplicate key (or several of
            // either) must not have any of them swallowed.
            for parse_error in &annotation.parse_errors {
                let (code, message) = match parse_error {
                    AnnotationParseError::DuplicateKey(key) => {
                        ("E-SCAN-005", format!("duplicate annotation key `{key}`"))
                    }
                    AnnotationParseError::UnknownKey(key) => {
                        ("E-SCAN-006", format!("unknown @vtest key `{key}`"))
                    }
                };
                self.diagnostics
                    .push(Diagnostic::error(code, message).with_location(location.clone()));
            }
            return;
        }
        let Some(id) = annotation
            .values
            .get("id")
            .filter(|value| !value.is_empty())
        else {
            self.diagnostics.push(
                Diagnostic::error(
                    "E-SCAN-007",
                    format!("test `{function_name}` is missing required @vtest.id"),
                )
                .with_location(location),
            );
            return;
        };
        let Some(covers) = annotation
            .values
            .get("covers")
            .filter(|value| !value.is_empty())
        else {
            self.diagnostics.push(
                Diagnostic::error(
                    "E-SCAN-007",
                    format!("test `{function_name}` is missing required @vtest.covers"),
                )
                .with_location(location),
            );
            return;
        };
        let Some(target_values) = annotation
            .repeated
            .get("target")
            .filter(|values| !values.is_empty() && values.iter().all(|value| !value.is_empty()))
        else {
            self.diagnostics.push(
                Diagnostic::error(
                    "E-SCAN-007",
                    format!("test `{function_name}` is missing required @vtest.target"),
                )
                .with_location(location),
            );
            return;
        };
        let integration = annotation
            .values
            .get("kind")
            .is_some_and(|kind| kind.starts_with("integration"));
        if target_values.len() > 1 && !integration {
            self.diagnostics.push(
                Diagnostic::error("E-SCAN-005", "duplicate annotation key `target`")
                    .with_location(location),
            );
            return;
        }
        let Some(intent) = annotation
            .values
            .get("intent")
            .filter(|value| !value.is_empty())
        else {
            self.diagnostics.push(
                Diagnostic::error(
                    "E-SCAN-007",
                    format!("test `{function_name}` is missing required @vtest.intent"),
                )
                .with_location(location),
            );
            return;
        };
        let test_id = TestId::new(id);
        if !self.test_ids.insert(id.clone()) {
            self.diagnostics.push(
                Diagnostic::error("E-SCAN-002", format!("duplicate Test ID `{id}`"))
                    .with_location(location.clone()),
            );
            return;
        }
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
            self.diagnostics.push(
                Diagnostic::error(
                    "E-SCAN-007",
                    format!("test `{id}` has no VO in @vtest.covers"),
                )
                .with_location(location.clone()),
            );
        }
        let targets = target_values
            .iter()
            .map(|target_value| {
                if let Some(src_id) = target_value.strip_prefix("SRC-") {
                    TargetRef::SrcId(SrcId::new(format!("SRC-{src_id}")))
                } else if let Some(locator) = Locator::parse(target_value) {
                    locator.as_target()
                } else {
                    self.diagnostics.push(
                        Diagnostic::error(
                            "E-SCAN-004",
                            format!("test `{id}` has an invalid target locator `{target_value}`"),
                        )
                        .with_location(location.clone()),
                    );
                    Locator {
                        path: relative.to_owned(),
                        item_path: item_path.to_owned(),
                    }
                    .as_target()
                }
            })
            .collect::<Vec<_>>();
        let execution = ExecutionDescriptor {
            adapter: AdapterId::new(RUST_ADAPTER_ID),
            project: Some(package.to_owned()),
            suite: Some(match test_target {
                TestTarget::Lib => TestSuite {
                    kind: "lib".to_owned(),
                    name: None,
                },
                TestTarget::Bin(name) => TestSuite {
                    kind: "bin".to_owned(),
                    name: Some(name.clone()),
                },
                TestTarget::IntegrationTest(name) => TestSuite {
                    kind: "integration".to_owned(),
                    name: Some(name.clone()),
                },
                TestTarget::Unknown => TestSuite {
                    kind: "unknown".to_owned(),
                    name: None,
                },
            }),
            selector: join_module_path(filter_prefix, item_path),
        };
        let input = annotation.values.get("input").cloned();
        let expect = annotation.values.get("expect").cloned();
        let kind = annotation.values.get("kind").cloned();
        let cases = annotation.repeated.get("case").cloned().unwrap_or_default();
        let related = annotation
            .repeated
            .get("related")
            .cloned()
            .unwrap_or_default()
            .into_iter()
            .flat_map(|value| {
                value
                    .split(',')
                    .map(str::trim)
                    .filter(|item| !item.is_empty())
                    .map(TestId::new)
                    .collect::<Vec<_>>()
            })
            .collect::<Vec<_>>();
        let managed = ManagedTestDraft {
            id: test_id,
            covers,
            targets,
            intent: intent.clone(),
            input,
            expect,
            kind,
            cases,
            related,
            execution,
        };
        self.discovered_tests.push(DiscoveredTestDraft {
            adapter: AdapterId::new(RUST_ADAPTER_ID),
            location: location.clone(),
            construct: construct.clone(),
            metadata_sources: vec![construct],
            managed: ManagedTestDraftLink::One(managed),
        });
    }

    /// Emit the hash-free discovery batch. The adapter only carries the
    /// per-item diagnostics it can decide locally (read/parse failures,
    /// structural annotation violations). Cross-entity resolution
    /// (E-SCAN-003 covers, E-SCAN-004 target resolution, E-SCAN-011 SRC ID
    /// collision) is owned by core, and canonical subjects are computed by
    /// `materialize_discovery_batch`.
    fn finish(self) -> DiscoveryBatch {
        DiscoveryBatch {
            adapter: AdapterId::new(RUST_ADAPTER_ID),
            completeness: DiscoveryCompleteness::Complete,
            discovered_tests: self.discovered_tests,
            source_targets: self.source_targets,
            diagnostics: self.diagnostics,
        }
    }
}
