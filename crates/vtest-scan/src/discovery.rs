//! Rust-specific source-location helpers extracted from the scanner. These move
//! to `vtest-adapter-rust` together with the rest of the Rust discovery code
//! (the scanner, Cargo manifest and module resolution, annotation parsing);
//! grouping the language-specific surface into one module isolates it first so
//! the cross-crate move is mechanical.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

use serde::Deserialize;
use syn::{Attribute, Expr, ExprLit, Item, Lit, Meta};
use vtest_model::{AdapterId, ProjectPath, SourceLocation, SourceRange, SrcId, TargetRef};

use crate::RUST_ADAPTER_ID;

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
    let mut had_error = false;
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
            // The caller cannot attach a parser diagnostic without losing the
            // source location, so retain a sentinel that is handled below.
            values.insert("__unknown_key__".to_owned(), key);
            had_error = true;
            continue;
        }
        if matches!(key.as_str(), "case" | "related" | "target") {
            repeated.entry(key).or_default().push(value);
        } else if values.insert(key.clone(), value).is_some() {
            values.insert("__duplicate_key__".to_owned(), key);
            had_error = true;
        }
    }
    if had_error {
        // Preserve parse information in a deterministic diagnostic channel.
        // `parse_annotations` itself stays total and its caller emits the
        // proper location-aware diagnostic.
        if let Some(key) = values.remove("__unknown_key__") {
            values.insert("__parse_error__".to_owned(), format!("unknown:{key}"));
        } else if let Some(key) = values.remove("__duplicate_key__") {
            values.insert("__parse_error__".to_owned(), format!("duplicate:{key}"));
        }
    }
    Some(ParsedAnnotations { values, repeated })
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
