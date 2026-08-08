//! Deterministic Rust source scanner for M1.

use std::{
    collections::{BTreeMap, BTreeSet},
    fs, io,
    path::{Path, PathBuf},
};

use serde::Serialize;
use syn::spanned::Spanned;
use syn::{Attribute, Expr, ExprLit, ImplItem, Item, ItemFn, ItemImpl, Lit, Meta};
use thiserror::Error;
use vtest_model::{
    ContentHash, Diagnostic, Locator, ScanSummary, SourceFunction, SourceLocation, SrcId,
    TargetRef, TestEntity, TestId, TestTarget, VoId,
};
use vtest_store::{
    load_config, read_approval, read_entity_ids, read_req, read_spec, read_text, read_vo,
    yaml_scalar_value, ProjectConfig, ReqRecord, StoreError, VerifyLayout, VoRecord,
};

pub mod operations;
pub use operations::*;

#[derive(Debug, Error)]
pub enum ScanError {
    #[error("store error: {0}")]
    Store(StoreError),
    #[error("I/O error at {path}: {source}")]
    Io { path: PathBuf, source: io::Error },
}

impl From<StoreError> for ScanError {
    fn from(value: StoreError) -> Self {
        Self::Store(value)
    }
}

#[derive(Clone, Debug, Serialize)]
pub struct ScanResult {
    pub summary: ScanSummary,
    pub tests: Vec<TestEntity>,
    pub sources: Vec<SourceFunction>,
    pub diagnostics: Vec<Diagnostic>,
}

impl ScanResult {
    pub fn has_errors(&self) -> bool {
        self.diagnostics.iter().any(Diagnostic::is_error)
    }
}

pub fn scan_project(root: &Path) -> Result<ScanResult, ScanError> {
    let config = load_config(root)?;
    scan_project_with_config(root, &config)
}

pub fn scan_project_with_config(
    root: &Path,
    config: &ProjectConfig,
) -> Result<ScanResult, ScanError> {
    let entity_ids = read_entity_ids(root)?;
    let vo_ids = entity_ids[2].iter().cloned().collect::<BTreeSet<_>>();
    let package = package_name(root).unwrap_or_else(|| config.project.name.clone());
    let mut paths = Vec::new();
    for include in &config.scan.include {
        collect_rs_files(&root.join(include), &mut paths).map_err(|source| ScanError::Io {
            path: root.join(include),
            source,
        })?;
    }
    paths.sort();
    paths.dedup();

    let mut scanner = Scanner::new(root, &package, vo_ids);
    for path in &paths {
        scanner.scan_file(path);
    }
    let mut result = scanner.finish(paths.len())?;
    result.diagnostics.extend(record_diagnostics(
        root,
        &entity_ids,
        &result.tests,
        &result.sources,
    ));
    Ok(result)
}

fn package_name(root: &Path) -> Option<String> {
    let manifest = fs::read_to_string(root.join("Cargo.toml")).ok()?;
    let mut in_package = false;
    for raw in manifest.lines() {
        let line = raw.trim();
        if line.starts_with('[') {
            in_package = line == "[package]";
            continue;
        }
        if in_package && line.starts_with("name") {
            let (_, value) = line.split_once('=')?;
            let name = value.trim().trim_matches(['\'', '"']);
            if !name.is_empty() {
                return Some(name.to_owned());
            }
        }
    }
    None
}

fn record_diagnostics(
    root: &Path,
    entity_ids: &[Vec<String>; 3],
    tests: &[TestEntity],
    sources: &[SourceFunction],
) -> Vec<Diagnostic> {
    let layout = VerifyLayout::new(root);
    let mut diagnostics = Vec::new();
    let mut known_ids = BTreeSet::new();
    for ids in entity_ids {
        known_ids.extend(ids.iter().cloned());
    }
    known_ids.extend(tests.iter().map(|test| test.id.as_str().to_owned()));
    for source in sources {
        known_ids.insert(source.locator.as_string());
        if let Some(src_id) = &source.src_id {
            known_ids.insert(src_id.as_str().to_owned());
        }
    }

    for id in &entity_ids[0] {
        validate_spec_record(root, &layout, id, &mut diagnostics);
    }

    let mut reqs = BTreeMap::new();
    for id in &entity_ids[1] {
        if let Some(record) = validate_req_record(&layout, id, &mut diagnostics) {
            reqs.insert(id.clone(), record);
        }
    }
    let mut vos = BTreeMap::new();
    for id in &entity_ids[2] {
        if let Some(record) = validate_vo_record(&layout, id, &mut diagnostics) {
            vos.insert(id.clone(), record);
        }
    }

    let req_parents = reqs
        .iter()
        .map(|(id, record)| {
            (
                id.clone(),
                record
                    .parent
                    .as_ref()
                    .map(|parent| parent.as_str().to_owned()),
            )
        })
        .collect::<BTreeMap<_, _>>();
    validate_parent_graph(&req_parents, "REQ", &mut diagnostics);
    let vo_parents = vos
        .iter()
        .map(|(id, record)| {
            (
                id.clone(),
                record
                    .parent
                    .as_ref()
                    .map(|parent| parent.as_str().to_owned()),
            )
        })
        .collect::<BTreeMap<_, _>>();
    validate_parent_graph(&vo_parents, "VO", &mut diagnostics);

    validate_relations(&layout, &known_ids, &mut diagnostics);
    validate_vo_warnings(&vos, tests, &mut diagnostics);
    validate_approval_status(&layout, &vos, &mut diagnostics);
    diagnostics
}

fn validate_spec_record(
    root: &Path,
    layout: &VerifyLayout,
    id: &str,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let path = layout.spec_dir().join(format!("{id}.yaml"));
    let text = match read_text(&path) {
        Ok(text) => text,
        Err(error) => {
            diagnostics.push(Diagnostic::error(
                "E-SCAN-010",
                format!("SPEC {id} cannot be read: {error}"),
            ));
            return;
        }
    };
    if let Some(missing) = missing_fields(&text, &["id", "kind", "path", "sha256", "registered_at"])
    {
        diagnostics.push(Diagnostic::error(
            "E-SCAN-010",
            format!("SPEC {id} is missing required fields: {missing}"),
        ));
    }
    if !matches!(
        yaml_scalar_value(&text, "kind").as_deref(),
        Some("document" | "api-schema" | "type-spec" | "db-schema" | "other")
    ) {
        diagnostics.push(Diagnostic::error(
            "E-SCAN-010",
            format!("SPEC {id} has an invalid kind"),
        ));
    }
    let record = match read_spec(layout, id) {
        Ok(record) => record,
        Err(error) => {
            diagnostics.push(Diagnostic::error(
                "E-SCAN-010",
                format!("SPEC {id} has an invalid schema: {error}"),
            ));
            return;
        }
    };
    if record.id.as_str() != id {
        diagnostics.push(Diagnostic::error(
            "E-SCAN-010",
            format!("SPEC file name {id} does not match record id {}", record.id),
        ));
    }
    let relative_path = Path::new(&record.path);
    if !is_safe_relative_path(relative_path) {
        diagnostics.push(Diagnostic::error(
            "E-SCAN-010",
            format!("SPEC {id} path must be project-relative: {}", record.path),
        ));
        return;
    }
    let source_path = root.join(relative_path);
    let bytes = match fs::read(&source_path) {
        Ok(bytes) => bytes,
        Err(error) => {
            diagnostics.push(Diagnostic::error(
                "E-SCAN-010",
                format!("SPEC {id} path {} cannot be read: {error}", record.path),
            ));
            return;
        }
    };
    let actual_hash = ContentHash::from_bytes(&bytes);
    if actual_hash != record.sha256 {
        diagnostics.push(Diagnostic::warning(
            "W-SCAN-104",
            format!(
                "SPEC {id} hash is stale: recorded {}, actual {}",
                record.sha256, actual_hash
            ),
        ));
    }
}

fn validate_req_record(
    layout: &VerifyLayout,
    id: &str,
    diagnostics: &mut Vec<Diagnostic>,
) -> Option<ReqRecord> {
    let path = layout.req_dir().join(format!("{id}.yaml"));
    let text = match read_text(&path) {
        Ok(text) => text,
        Err(error) => {
            diagnostics.push(Diagnostic::error(
                "E-SCAN-010",
                format!("REQ {id} cannot be read: {error}"),
            ));
            return None;
        }
    };
    if let Some(missing) = missing_fields(&text, &["id", "summary", "status", "created", "updated"])
    {
        diagnostics.push(Diagnostic::error(
            "E-SCAN-010",
            format!("REQ {id} is missing required fields: {missing}"),
        ));
    }
    let record = read_req(layout, id).ok()?;
    if record.id.as_str() != id {
        diagnostics.push(Diagnostic::error(
            "E-SCAN-010",
            format!("REQ file name {id} does not match record id {}", record.id),
        ));
    }
    if !matches!(record.status.as_str(), "active" | "withdrawn") {
        diagnostics.push(Diagnostic::error(
            "E-SCAN-010",
            format!("REQ {id} has invalid status {}", record.status),
        ));
    }
    Some(record)
}

fn validate_vo_record(
    layout: &VerifyLayout,
    id: &str,
    diagnostics: &mut Vec<Diagnostic>,
) -> Option<VoRecord> {
    let path = layout.vo_dir().join(format!("{id}.yaml"));
    let text = match read_text(&path) {
        Ok(text) => text,
        Err(error) => {
            diagnostics.push(Diagnostic::error(
                "E-SCAN-010",
                format!("VO {id} cannot be read: {error}"),
            ));
            return None;
        }
    };
    if let Some(missing) = missing_fields(&text, &["id", "claim", "status", "created", "updated"]) {
        diagnostics.push(Diagnostic::error(
            "E-SCAN-010",
            format!("VO {id} is missing required fields: {missing}"),
        ));
    }
    let record = read_vo(layout, id).ok()?;
    if record.id.as_str() != id {
        diagnostics.push(Diagnostic::error(
            "E-SCAN-010",
            format!("VO file name {id} does not match record id {}", record.id),
        ));
    }
    if !matches!(record.status.as_str(), "draft" | "approved") {
        diagnostics.push(Diagnostic::error(
            "E-SCAN-010",
            format!("VO {id} has invalid status {}", record.status),
        ));
    }
    if let Some(policy) = &record.coverage_policy {
        if !matches!(
            policy.as_str(),
            "independent-axes" | "full-product" | "explicit"
        ) {
            diagnostics.push(Diagnostic::error(
                "E-SCAN-010",
                format!("VO {id} has invalid coverage_policy {policy}"),
            ));
        }
    }
    Some(record)
}

fn missing_fields(text: &str, fields: &[&str]) -> Option<String> {
    let missing = fields
        .iter()
        .copied()
        .filter(|field| {
            yaml_scalar_value(text, field)
                .map(|value| value.trim().is_empty())
                .unwrap_or(true)
        })
        .collect::<Vec<_>>();
    (!missing.is_empty()).then(|| missing.join(", "))
}

fn validate_parent_graph(
    parents: &BTreeMap<String, Option<String>>,
    kind: &str,
    diagnostics: &mut Vec<Diagnostic>,
) {
    for (id, parent) in parents {
        if let Some(parent) = parent {
            if !parents.contains_key(parent) {
                diagnostics.push(Diagnostic::error(
                    "E-SCAN-008",
                    format!("{kind} {id} references missing parent {parent}"),
                ));
            }
        }
    }

    let mut reported = BTreeSet::new();
    for start in parents.keys() {
        let mut path = Vec::new();
        let mut positions = BTreeMap::new();
        let mut current = start.clone();
        loop {
            if let Some(index) = positions.get(&current) {
                let cycle = path[*index..].to_vec();
                let mut key_parts = cycle.clone();
                key_parts.sort();
                let key = key_parts.join("|");
                if reported.insert(key) {
                    diagnostics.push(Diagnostic::error(
                        "E-SCAN-008",
                        format!("{kind} parent cycle: {}", cycle.join(" -> ")),
                    ));
                }
                break;
            }
            positions.insert(current.clone(), path.len());
            path.push(current.clone());
            let Some(Some(parent)) = parents.get(&current) else {
                break;
            };
            if !parents.contains_key(parent) {
                break;
            }
            current = parent.clone();
        }
    }
}

fn validate_relations(
    layout: &VerifyLayout,
    known_ids: &BTreeSet<String>,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let allowed_types = [
        "depends-on",
        "supersedes",
        "regression-for",
        "derived-from",
        "same-partition",
        "complements",
        "conflicts-with",
    ];
    let entries = match fs::read_dir(layout.relation_dir()) {
        Ok(entries) => entries,
        Err(_) => return,
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|value| value.to_str()) != Some("yaml") {
            continue;
        }
        let file_id = path
            .file_stem()
            .and_then(|value| value.to_str())
            .unwrap_or_default()
            .to_owned();
        let text = match read_text(&path) {
            Ok(text) => text,
            Err(error) => {
                diagnostics.push(Diagnostic::error(
                    "E-SCAN-010",
                    format!("relation {file_id} cannot be read: {error}"),
                ));
                continue;
            }
        };
        let mut invalid = false;
        if yaml_scalar_value(&text, "id").as_deref() != Some(file_id.as_str()) {
            diagnostics.push(Diagnostic::error(
                "E-SCAN-010",
                format!("relation file name {file_id} does not match record id"),
            ));
            invalid = true;
        }
        if let Some(missing) = missing_fields(&text, &["id", "type", "from", "to", "created"]) {
            diagnostics.push(Diagnostic::error(
                "E-SCAN-010",
                format!("relation {file_id} is missing required fields: {missing}"),
            ));
            invalid = true;
        }
        let relation_type = yaml_scalar_value(&text, "type");
        if relation_type
            .as_deref()
            .is_some_and(|value| !allowed_types.contains(&value))
        {
            diagnostics.push(Diagnostic::error(
                "E-SCAN-010",
                format!(
                    "relation {file_id} has invalid type {}",
                    relation_type.unwrap_or_default()
                ),
            ));
            invalid = true;
        }
        if invalid {
            continue;
        }
        for field in ["from", "to"] {
            if let Some(value) = yaml_scalar_value(&text, field) {
                if !known_ids.contains(&value) {
                    diagnostics.push(Diagnostic::error(
                        "E-SCAN-009",
                        format!("relation {file_id} {field} references missing entity {value}"),
                    ));
                }
            }
        }
    }
}

fn validate_vo_warnings(
    vos: &BTreeMap<String, VoRecord>,
    tests: &[TestEntity],
    diagnostics: &mut Vec<Diagnostic>,
) {
    let child_ids = vos
        .values()
        .filter_map(|vo| vo.parent.as_ref().map(|parent| parent.as_str().to_owned()))
        .collect::<BTreeSet<_>>();
    let covered_ids = tests
        .iter()
        .flat_map(|test| test.covers.iter().map(|vo| vo.as_str().to_owned()))
        .collect::<BTreeSet<_>>();
    for id in vos.keys() {
        if !child_ids.contains(id) && !covered_ids.contains(id) {
            diagnostics.push(Diagnostic::warning(
                "W-SCAN-102",
                format!("VO {id} is isolated and has no covering test"),
            ));
        }
    }
    for test in tests {
        for vo_id in &test.covers {
            if child_ids.contains(vo_id.as_str()) {
                diagnostics.push(Diagnostic::warning(
                    "W-SCAN-103",
                    format!("test {} covers non-leaf VO {}", test.id, vo_id),
                ));
            }
        }
    }
}

fn validate_approval_status(
    layout: &VerifyLayout,
    vos: &BTreeMap<String, VoRecord>,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let mut current_hashes = BTreeMap::new();
    for id in vos.keys() {
        let path = layout.vo_dir().join(format!("{id}.yaml"));
        if let Ok(text) = read_text(&path) {
            current_hashes.insert(id.clone(), ContentHash::from_text(&text));
        }
    }
    let mut approved = BTreeSet::new();
    let entries = match fs::read_dir(layout.approvals_dir()) {
        Ok(entries) => entries,
        Err(_) => return,
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|value| value.to_str()) != Some("yaml") {
            continue;
        }
        let file_id = path
            .file_stem()
            .and_then(|value| value.to_str())
            .unwrap_or_default()
            .to_owned();
        let text = match read_text(&path) {
            Ok(text) => text,
            Err(error) => {
                diagnostics.push(Diagnostic::error(
                    "E-SCAN-010",
                    format!("approval {file_id} cannot be read: {error}"),
                ));
                continue;
            }
        };
        let mut invalid = false;
        if let Some(missing) =
            missing_fields(&text, &["id", "subject", "subject_hash", "approved_at"])
        {
            diagnostics.push(Diagnostic::error(
                "E-SCAN-010",
                format!("approval {file_id} is missing required fields: {missing}"),
            ));
            invalid = true;
        }
        let approval = match read_approval(&path) {
            Ok(approval) => approval,
            Err(error) => {
                diagnostics.push(Diagnostic::error(
                    "E-SCAN-010",
                    format!("approval {file_id} has an invalid schema: {error}"),
                ));
                continue;
            }
        };
        if approval.id != file_id {
            diagnostics.push(Diagnostic::error(
                "E-SCAN-010",
                format!(
                    "approval file name {file_id} does not match record id {}",
                    approval.id
                ),
            ));
            invalid = true;
        }
        if invalid {
            continue;
        }
        let subject = approval.subject.as_str();
        let Some(current_hash) = current_hashes.get(subject) else {
            diagnostics.push(Diagnostic::error(
                "E-SCAN-010",
                format!("approval {file_id} references missing VO {subject}"),
            ));
            continue;
        };
        if current_hash == &approval.subject_hash {
            approved.insert(subject.to_owned());
        }
    }
    for (id, vo) in vos {
        let effective = approved.contains(id);
        if (vo.status == "approved") != effective {
            diagnostics.push(Diagnostic::warning(
                "W-STORE-001",
                format!(
                    "VO {id} status {} differs from approval-derived status {}",
                    vo.status,
                    if effective { "approved" } else { "draft" }
                ),
            ));
        }
    }
}

fn is_safe_relative_path(path: &Path) -> bool {
    !path.is_absolute()
        && path.components().all(|component| {
            !matches!(
                component,
                std::path::Component::ParentDir
                    | std::path::Component::RootDir
                    | std::path::Component::Prefix(_)
            )
        })
}

fn collect_rs_files(path: &Path, output: &mut Vec<PathBuf>) -> io::Result<()> {
    if !path.exists() {
        return Ok(());
    }
    let metadata = fs::metadata(path)?;
    if metadata.is_file() {
        if path.extension().and_then(|v| v.to_str()) == Some("rs") {
            output.push(path.to_owned());
        }
        return Ok(());
    }
    for entry in fs::read_dir(path)? {
        let entry = entry?;
        let child = entry.path();
        if child.file_name().and_then(|v| v.to_str()) == Some("target") {
            continue;
        }
        if child.is_dir() {
            collect_rs_files(&child, output)?;
        } else if child.extension().and_then(|v| v.to_str()) == Some("rs") {
            output.push(child);
        }
    }
    Ok(())
}

struct Scanner<'a> {
    root: &'a Path,
    package: &'a str,
    vo_ids: BTreeSet<String>,
    tests: Vec<TestEntity>,
    sources: Vec<SourceFunction>,
    diagnostics: Vec<Diagnostic>,
    test_ids: BTreeSet<String>,
}

impl<'a> Scanner<'a> {
    fn new(root: &'a Path, package: &'a str, vo_ids: BTreeSet<String>) -> Self {
        Self {
            root,
            package,
            vo_ids,
            tests: Vec::new(),
            sources: Vec::new(),
            diagnostics: Vec::new(),
            test_ids: BTreeSet::new(),
        }
    }

    fn scan_file(&mut self, path: &Path) {
        let source = match fs::read_to_string(path) {
            Ok(source) => source,
            Err(source) => {
                self.diagnostics.push(Diagnostic::error(
                    "E-SCAN-001",
                    format!("failed to read {}: {source}", path.display()),
                ));
                return;
            }
        };
        let syntax = match syn::parse_file(&source) {
            Ok(syntax) => syntax,
            Err(error) => {
                self.diagnostics.push(Diagnostic::error(
                    "E-SCAN-001",
                    format!("failed to parse {}: {error}", path.display()),
                ));
                return;
            }
        };
        let relative = path
            .strip_prefix(self.root)
            .unwrap_or(path)
            .to_string_lossy()
            .replace('\\', "/");
        let target = test_target_for_path(&relative);
        let line_offsets = line_offsets(&source);
        self.collect_items(
            &syntax.items,
            &relative,
            &target,
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
                    source,
                    line_offsets,
                    module,
                    path,
                ),
                Item::Impl(item_impl) => self.collect_impl(
                    item_impl,
                    relative,
                    test_target,
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
        source: &str,
        line_offsets: &[usize],
        _path: &Path,
    ) {
        let location = make_location(relative, item_path, span, source, line_offsets);
        let content = source_slice(source, &location);
        let source_function = SourceFunction {
            locator: Locator {
                path: relative.to_owned(),
                item_path: item_path.to_owned(),
            },
            src_id: parse_src_id(attrs),
            location: location.clone(),
            content_hash: ContentHash::from_text(content),
        };
        self.sources.push(source_function);

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
        if let Some(parse_error) = annotation.values.get("__parse_error__") {
            let (kind, key) = parse_error
                .split_once(':')
                .unwrap_or(("unknown", parse_error));
            let (code, message) = if kind == "duplicate" {
                ("E-SCAN-005", format!("duplicate annotation key `{key}`"))
            } else {
                ("E-SCAN-006", format!("unknown @vtest key `{key}`"))
            };
            self.diagnostics
                .push(Diagnostic::error(code, message).with_location(location));
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
        for vo_id in &covers {
            if !self.vo_ids.contains(vo_id.as_str()) {
                self.diagnostics.push(
                    Diagnostic::error(
                        "E-SCAN-003",
                        format!("test `{id}` references missing VO `{vo_id}`"),
                    )
                    .with_location(location.clone()),
                );
            }
        }
        let mut targets = target_values
            .iter()
            .map(|target_value| {
                if let Some(src_id) = target_value.strip_prefix("SRC-") {
                    TargetRef::SrcId(SrcId::new(format!("SRC-{src_id}")))
                } else if let Some(locator) = Locator::parse(target_value) {
                    TargetRef::Locator(locator)
                } else {
                    self.diagnostics.push(
                        Diagnostic::error(
                            "E-SCAN-004",
                            format!("test `{id}` has an invalid target locator `{target_value}`"),
                        )
                        .with_location(location.clone()),
                    );
                    TargetRef::Locator(Locator {
                        path: relative.to_owned(),
                        item_path: item_path.to_owned(),
                    })
                }
            })
            .collect::<Vec<_>>();
        let target = targets.remove(0);
        let source_hash = ContentHash::from_text(content);
        let entity = TestEntity {
            id: test_id,
            covers,
            target,
            additional_targets: targets,
            intent: intent.clone(),
            input: annotation.values.get("input").cloned(),
            expect: annotation.values.get("expect").cloned(),
            kind: annotation.values.get("kind").cloned(),
            cases: annotation.repeated.get("case").cloned().unwrap_or_default(),
            related: annotation
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
                .collect(),
            location,
            content_hash: source_hash,
            filter: function_name.to_owned(),
            package: self.package.to_owned(),
            test_target: test_target.clone(),
        };
        self.tests.push(entity);
    }

    fn finish(self, files: usize) -> Result<ScanResult, ScanError> {
        let mut diagnostics = self.diagnostics;
        let mut locators = BTreeMap::new();
        let mut src_ids = BTreeMap::new();
        for source in &self.sources {
            locators
                .entry(source.locator.as_string())
                .or_insert_with(|| source.locator.clone());
            if let Some(src_id) = &source.src_id {
                src_ids
                    .entry(src_id.as_str().to_owned())
                    .or_insert_with(|| source.locator.clone());
            }
        }
        for test in &self.tests {
            for target in std::iter::once(&test.target).chain(&test.additional_targets) {
                let resolved = match target {
                    TargetRef::Locator(locator) => locators.contains_key(&locator.as_string()),
                    TargetRef::SrcId(src_id) => src_ids.contains_key(src_id.as_str()),
                };
                if !resolved {
                    diagnostics.push(
                        Diagnostic::error(
                            "E-SCAN-004",
                            format!("test `{}` target cannot be resolved", test.id),
                        )
                        .with_location(test.location.clone()),
                    );
                }
            }
        }
        Ok(ScanResult {
            summary: ScanSummary {
                files,
                tests: self.tests.len(),
                sources: self.sources.len(),
            },
            tests: self.tests,
            sources: self.sources,
            diagnostics,
        })
    }
}

struct ParsedAnnotations {
    values: BTreeMap<String, String>,
    repeated: BTreeMap<String, Vec<String>>,
}

fn parse_annotations(attrs: &[Attribute]) -> Option<ParsedAnnotations> {
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

fn is_test_function(attrs: &[Attribute]) -> bool {
    attrs.iter().any(|attr| {
        attr.path()
            .segments
            .last()
            .is_some_and(|segment| segment.ident == "test")
    })
}

fn parse_src_id(attrs: &[Attribute]) -> Option<SrcId> {
    parse_annotations(attrs)
        .and_then(|annotations| annotations.values.get("src-id").cloned())
        .map(SrcId::new)
}

fn test_target_for_path(path: &str) -> TestTarget {
    let components = path.split('/').collect::<Vec<_>>();
    if let Some(index) = components
        .iter()
        .position(|component| *component == "tests")
    {
        if let Some(file) = components
            .get(index + 1)
            .and_then(|value| value.strip_suffix(".rs"))
        {
            return TestTarget::IntegrationTest((*file).to_owned());
        }
    }
    TestTarget::Lib
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
        start_line,
        end_line,
        start_byte,
        end_byte: end_byte.min(source.len()),
    }
}

fn source_slice<'a>(source: &'a str, location: &SourceLocation) -> &'a str {
    source
        .get(location.start_byte..location.end_byte)
        .unwrap_or("")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};
    use vtest_store::init_project;

    fn valid_vo(id: &str, parent: &str) -> String {
        format!(
            "id: {id}\nparent: {parent}\nrequirements: []\nspec_refs: []\nclaim: claim\ndimensions: []\ncoverage_policy: null\nrepresentative_cases: []\nstatus: draft\ncreated: '2026-01-01'\nupdated: '2026-01-01'\n"
        )
    }

    fn fixture() -> PathBuf {
        let suffix = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = std::env::temp_dir().join(format!("vtest-scan-{suffix}"));
        fs::create_dir_all(root.join("src")).unwrap();
        fs::create_dir_all(root.join("tests")).unwrap();
        init_project(&root, "fixture").unwrap();
        fs::write(
            root.join("src/lib.rs"),
            "pub fn add(a: i32, b: i32) -> i32 { a + b }\n",
        )
        .unwrap();
        fs::write(
            root.join("tests/calc.rs"),
            r#"
/// @vtest.id TEST-ADD
/// @vtest.covers VO-ADD
/// @vtest.target src/lib.rs::add
/// @vtest.intent adds values
#[test]
fn adds() { assert_eq!(2, crate::missing()); }
"#,
        )
        .unwrap();
        fs::write(
            root.join(".verify/vo/VO-ADD.yaml"),
            "id: VO-ADD\nparent: null\nrequirements: []\nspec_refs: []\nclaim: adds values\ndimensions: []\ncoverage_policy: null\nrepresentative_cases: []\nstatus: draft\ncreated: '2026-01-01'\nupdated: '2026-01-01'\n",
        )
        .unwrap();
        root
    }

    #[test]
    fn extracts_annotated_test_and_source() {
        let root = fixture();
        let result = scan_project(&root).unwrap();
        assert_eq!(result.summary.tests, 1);
        assert_eq!(result.summary.sources, 2);
        assert!(
            !result.has_errors(),
            "diagnostics: {:?}",
            result.diagnostics
        );
        assert_eq!(result.tests[0].id.as_str(), "TEST-ADD");
        assert_eq!(result.tests[0].package, "fixture");
        assert_eq!(
            result.tests[0].test_target,
            TestTarget::IntegrationTest("calc".to_owned())
        );
    }

    #[test]
    fn reports_unregistered_tests() {
        let root = fixture();
        fs::write(root.join("tests/unregistered.rs"), "#[test]\nfn x() {}\n").unwrap();
        let result = scan_project(&root).unwrap();
        assert!(result.diagnostics.iter().any(|d| d.code == "W-SCAN-101"));
    }

    #[test]
    fn rejects_unknown_and_duplicate_annotation_keys() {
        let root = fixture();
        fs::write(
            root.join("tests/invalid.rs"),
            r#"
/// @vtest.id TEST-UNKNOWN
/// @vtest.covers VO-ADD
/// @vtest.target src/lib.rs::add
/// @vtest.intent invalid
/// @vtest.typo value
#[test]
fn unknown_key() {}

/// @vtest.id TEST-DUPLICATE
/// @vtest.id TEST-DUPLICATE-2
/// @vtest.covers VO-ADD
/// @vtest.target src/lib.rs::add
/// @vtest.intent invalid
#[test]
fn duplicate_key() {}
"#,
        )
        .unwrap();
        let result = scan_project(&root).unwrap();
        assert!(result.diagnostics.iter().any(|d| d.code == "E-SCAN-005"));
        assert!(result.diagnostics.iter().any(|d| d.code == "E-SCAN-006"));
    }

    #[test]
    fn rejects_missing_required_annotation() {
        let root = fixture();
        fs::write(
            root.join("tests/invalid.rs"),
            r#"
/// @vtest.id TEST-MISSING
/// @vtest.covers VO-ADD
/// @vtest.target src/lib.rs::does_not_exist
#[test]
fn missing_intent() {}
"#,
        )
        .unwrap();
        let result = scan_project(&root).unwrap();
        assert!(result.diagnostics.iter().any(|d| d.code == "E-SCAN-007"));
    }

    #[test]
    fn integration_tests_allow_multiple_targets_only() {
        let root = fixture();
        fs::write(
            root.join("src/lib.rs"),
            "pub fn add(a: i32, b: i32) -> i32 { a + b }\npub fn subtract(a: i32, b: i32) -> i32 { a - b }\n",
        )
        .unwrap();
        fs::write(
            root.join("tests/multiple.rs"),
            r#"
/// @vtest.id TEST-INTEGRATION
/// @vtest.covers VO-ADD
/// @vtest.target src/lib.rs::add
/// @vtest.target src/lib.rs::subtract
/// @vtest.intent combines operations
/// @vtest.kind integration-normal
#[test]
fn combines() {}

/// @vtest.id TEST-UNIT-DUPLICATE
/// @vtest.covers VO-ADD
/// @vtest.target src/lib.rs::add
/// @vtest.target src/lib.rs::subtract
/// @vtest.intent invalid duplicate
/// @vtest.kind unit-normal
#[test]
fn duplicate_target() {}
"#,
        )
        .unwrap();
        let result = scan_project(&root).unwrap();
        let integration = result
            .tests
            .iter()
            .find(|test| test.id.as_str() == "TEST-INTEGRATION")
            .unwrap();
        assert_eq!(integration.additional_targets.len(), 1);
        assert!(result.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "E-SCAN-005"
                && diagnostic
                    .location
                    .as_ref()
                    .is_some_and(|location| location.function == "duplicate_target")
        }));
    }

    #[test]
    fn reports_record_integrity_and_staleness_diagnostics() {
        let root = fixture();
        fs::write(
            root.join(".verify/req/REQ-A.yaml"),
            "id: REQ-A\nparent: REQ-B\nsummary: A\nstatus: active\ncreated: '2026-01-01'\nupdated: '2026-01-01'\n",
        )
        .unwrap();
        fs::write(
            root.join(".verify/req/REQ-B.yaml"),
            "id: REQ-B\nparent: REQ-A\nsummary: B\nstatus: active\ncreated: '2026-01-01'\nupdated: '2026-01-01'\n",
        )
        .unwrap();
        fs::write(
            root.join(".verify/vo/VO-MISSING-PARENT.yaml"),
            valid_vo("VO-MISSING-PARENT", "VO-NOT-FOUND"),
        )
        .unwrap();
        fs::write(
            root.join(".verify/vo/VO-PARENT.yaml"),
            valid_vo("VO-PARENT", "null"),
        )
        .unwrap();
        fs::write(
            root.join(".verify/vo/VO-CHILD.yaml"),
            valid_vo("VO-CHILD", "VO-PARENT"),
        )
        .unwrap();
        fs::write(
            root.join(".verify/vo/VO-RENAMED.yaml"),
            valid_vo("VO-DIFFERENT", "null"),
        )
        .unwrap();
        fs::write(
            root.join("tests/parent.rs"),
            r#"
/// @vtest.id TEST-PARENT
/// @vtest.covers VO-PARENT
/// @vtest.target src/lib.rs::add
/// @vtest.intent covers a parent VO
#[test]
fn covers_parent() {}
"#,
        )
        .unwrap();

        fs::create_dir_all(root.join("docs")).unwrap();
        fs::write(root.join("docs/spec.md"), "original\n").unwrap();
        let spec_hash = ContentHash::from_text("original\n");
        fs::write(
            root.join(".verify/spec/SPEC-ONE.yaml"),
            format!(
                "id: SPEC-ONE\nkind: document\npath: docs/spec.md\nsha256: {spec_hash}\nregistered_at: '2026-01-01'\n"
            ),
        )
        .unwrap();
        fs::write(root.join("docs/spec.md"), "changed\n").unwrap();

        let relation = root.join(".verify/rel/REL-ONE.yaml");
        fs::write(
            relation,
            "id: REL-ONE\ntype: depends-on\nfrom: ENTITY-NOT-FOUND\nto: VO-ADD\ncreated: '2026-01-01'\n",
        )
        .unwrap();

        let vo_text = fs::read_to_string(root.join(".verify/vo/VO-ADD.yaml")).unwrap();
        let vo_hash = ContentHash::from_text(&vo_text);
        fs::write(
            root.join(".verify/approvals/APPROVAL-ONE.yaml"),
            format!(
                "id: APPROVAL-ONE\nsubject: VO-ADD\nsubject_hash: {vo_hash}\napprover:\n  kind: human\n  id: reviewer\napproved_at: '2026-01-01'\n"
            ),
        )
        .unwrap();

        let result = scan_project(&root).unwrap();
        let codes = result
            .diagnostics
            .iter()
            .map(|diagnostic| diagnostic.code.as_str())
            .collect::<BTreeSet<_>>();
        assert!(
            codes.contains("E-SCAN-008"),
            "diagnostics: {:?}",
            result.diagnostics
        );
        assert!(
            codes.contains("E-SCAN-009"),
            "diagnostics: {:?}",
            result.diagnostics
        );
        assert!(
            codes.contains("E-SCAN-010"),
            "diagnostics: {:?}",
            result.diagnostics
        );
        assert!(
            codes.contains("W-SCAN-102"),
            "diagnostics: {:?}",
            result.diagnostics
        );
        assert!(
            codes.contains("W-SCAN-103"),
            "diagnostics: {:?}",
            result.diagnostics
        );
        assert!(
            codes.contains("W-SCAN-104"),
            "diagnostics: {:?}",
            result.diagnostics
        );
        assert!(
            codes.contains("W-STORE-001"),
            "diagnostics: {:?}",
            result.diagnostics
        );
    }
}
