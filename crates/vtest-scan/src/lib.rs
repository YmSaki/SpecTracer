//! Deterministic Rust source scanner for M1.

use std::{
    collections::{BTreeMap, BTreeSet},
    fs, io,
    path::{Path, PathBuf},
};

use ignore::WalkBuilder;
use serde::{Deserialize, Serialize};
use syn::spanned::Spanned;
use syn::{Attribute, Expr, ExprLit, ImplItem, Item, ItemFn, ItemImpl, Lit, Meta};
use thiserror::Error;
use vtest_adapter_api::{
    AdapterError, DiscoveredTestDraft, DiscoveryBatch, DiscoveryCompleteness, ManagedTestDraft,
    ManagedTestDraftLink, SourceDiscoveryAdapter, SourceFragment, SourceTargetDraft,
};
use vtest_model::{
    hash_specification_source, hash_target_subject, hash_test_subject, AdapterId, CanonicalProjection,
    ContentHash, Diagnostic, DiscoveredTest, ExecutionDescriptor, ManagedTestLink, ProjectPath,
    ScanSummary, SourceFunction, SourceLocation, SourceRange, SourceTarget, SrcId, TargetRef,
    TestEntity, TestId, TestSubjectInput, TestSuite, VoId,
};
use vtest_store::{
    current_approval_subject, derive_vo_status, is_valid_ulid, load_config, read_approval,
    read_entity_ids, read_req, read_spec, read_text, read_vo, relation_ulid_payload,
    yaml_scalar_value, ProjectConfig, RelationRecord, ReqRecord, StoreError, VerifyLayout,
    VoRecord,
};

pub mod operations;
pub use operations::*;

const RUST_ADAPTER_ID: &str = "rust-cargo";

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
enum TestTarget {
    Lib,
    Bin(String),
    IntegrationTest(String),
    Unknown,
}

#[derive(Debug, Error)]
pub enum ScanError {
    #[error("store error: {0}")]
    Store(StoreError),
    #[error("I/O error at {path}: {source}")]
    Io { path: PathBuf, source: io::Error },
    #[error("source discovery failed at {path}: {message}")]
    Discovery { path: PathBuf, message: String },
    #[error("{0}")]
    Adapter(#[from] AdapterError),
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

/// Core-owned result of validating and materializing one adapter discovery batch.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MaterializedDiscovery {
    pub adapter: AdapterId,
    pub discovered_tests: Vec<DiscoveredTest>,
    pub managed_tests: Vec<TestEntity>,
    pub source_targets: Vec<SourceTarget>,
    pub diagnostics: Vec<Diagnostic>,
}

/// Validate an adapter's hash-free observations against current filesystem bytes,
/// then compute canonical subjects and materialize neutral domain entities.
pub fn materialize_discovery_batch(
    root: &Path,
    batch: DiscoveryBatch,
) -> Result<MaterializedDiscovery, ScanError> {
    if batch.completeness != DiscoveryCompleteness::Complete {
        return Err(malformed_adapter_output(
            "incomplete discovery cannot be materialized as a successful scan",
        ));
    }

    let adapter = batch.adapter.clone();
    if adapter.as_str().is_empty() {
        return Err(malformed_adapter_output("batch adapter ID is empty"));
    }

    let mut managed_tests = Vec::new();
    let mut discovered_tests = Vec::with_capacity(batch.discovered_tests.len());
    for draft in batch.discovered_tests {
        let (discovered, mut tests) = materialize_discovered_test(root, &adapter, draft)?;
        discovered_tests.push(discovered);
        managed_tests.append(&mut tests);
    }

    let mut source_targets = batch
        .source_targets
        .into_iter()
        .map(|draft| materialize_source_target(root, &adapter, draft))
        .collect::<Result<Vec<_>, _>>()?;

    discovered_tests.sort_by(|left, right| {
        (
            left.location.path.as_str(),
            left.location.locator.as_str(),
            &left.managed,
        )
            .cmp(&(
                right.location.path.as_str(),
                right.location.locator.as_str(),
                &right.managed,
            ))
    });
    managed_tests.sort_by(|left, right| left.id.cmp(&right.id));
    source_targets.sort_by(|left, right| left.target.normalized().cmp(&right.target.normalized()));

    Ok(MaterializedDiscovery {
        adapter,
        discovered_tests,
        managed_tests,
        source_targets,
        diagnostics: batch.diagnostics,
    })
}

fn materialize_discovered_test(
    root: &Path,
    adapter: &AdapterId,
    draft: DiscoveredTestDraft,
) -> Result<(DiscoveredTest, Vec<TestEntity>), ScanError> {
    if draft.adapter != *adapter {
        return Err(malformed_adapter_output(
            "Discovered Test adapter does not match its batch",
        ));
    }
    if draft.location != draft.construct.location {
        return Err(malformed_adapter_output(
            "Discovered Test location does not match its construct fragment",
        ));
    }
    validate_current_fragment(root, adapter, &draft.construct)?;
    for source in &draft.metadata_sources {
        validate_current_fragment(root, adapter, source)?;
    }

    let construct_hash = ContentHash::from_bytes(&draft.construct.bytes);
    let (managed, tests) = match draft.managed {
        ManagedTestDraftLink::Missing => (ManagedTestLink::Missing, Vec::new()),
        ManagedTestDraftLink::One(managed) => {
            require_metadata_provenance(&draft.metadata_sources)?;
            let test = materialize_managed_test(
                adapter,
                &draft.location,
                &draft.construct.bytes,
                managed,
            )?;
            let id = test.id.clone();
            (ManagedTestLink::One(id), vec![test])
        }
        ManagedTestDraftLink::Multiple(managed) => {
            require_metadata_provenance(&draft.metadata_sources)?;
            if managed.len() < 2 {
                return Err(malformed_adapter_output(
                    "ManagedTestDraftLink::Multiple requires at least two drafts",
                ));
            }
            let tests = managed
                .into_iter()
                .map(|managed| {
                    materialize_managed_test(
                        adapter,
                        &draft.location,
                        &draft.construct.bytes,
                        managed,
                    )
                })
                .collect::<Result<Vec<_>, _>>()?;
            let mut ids = tests.iter().map(|test| test.id.clone()).collect::<Vec<_>>();
            ids.sort();
            (ManagedTestLink::Multiple(ids), tests)
        }
    };

    Ok((
        DiscoveredTest {
            adapter: adapter.clone(),
            location: draft.location,
            content_hash: construct_hash,
            managed,
        },
        tests,
    ))
}

fn materialize_managed_test(
    adapter: &AdapterId,
    location: &SourceLocation,
    construct: &[u8],
    managed: ManagedTestDraft,
) -> Result<TestEntity, ScanError> {
    if managed.id.as_str().is_empty()
        || managed.covers.is_empty()
        || managed.intent.trim().is_empty()
    {
        return Err(malformed_adapter_output(
            "managed Test draft is missing required logical metadata",
        ));
    }
    if managed.execution.adapter != *adapter {
        return Err(malformed_adapter_output(
            "managed Test execution adapter does not match discovery adapter",
        ));
    }
    let content_hash = hash_test_subject(&TestSubjectInput {
        adapter,
        id: &managed.id,
        covers: &managed.covers,
        targets: &managed.targets,
        intent: &managed.intent,
        input: managed.input.as_deref(),
        expect: managed.expect.as_deref(),
        kind: managed.kind.as_deref(),
        cases: &managed.cases,
        related: &managed.related,
        location,
        execution: &managed.execution,
        construct,
    });
    let test = TestEntity {
        id: managed.id,
        covers: managed.covers,
        targets: managed.targets,
        intent: managed.intent,
        input: managed.input,
        expect: managed.expect,
        kind: managed.kind,
        cases: managed.cases,
        related: managed.related,
        location: location.clone(),
        content_hash,
        execution: managed.execution,
    };
    test.validate().map_err(malformed_adapter_output)?;
    Ok(test)
}

fn materialize_source_target(
    root: &Path,
    adapter: &AdapterId,
    draft: SourceTargetDraft,
) -> Result<SourceTarget, ScanError> {
    if draft.location != draft.construct.location {
        return Err(malformed_adapter_output(
            "Source Target location does not match its construct fragment",
        ));
    }
    let TargetRef::Locator {
        adapter: target_adapter,
        ..
    } = &draft.target
    else {
        // A permanent SRC ID is how something refers to a Source Target, never
        // the Source Target's own canonical reference.
        return Err(malformed_adapter_output(
            "Source Target canonical target must be a locator, not a permanent SRC ID",
        ));
    };
    if target_adapter != adapter {
        return Err(malformed_adapter_output(
            "Source Target locator adapter does not match its batch",
        ));
    }
    validate_current_fragment(root, adapter, &draft.construct)?;
    let src_id = draft.src_id;
    Ok(SourceTarget {
        content_hash: hash_target_subject(&draft.target, &draft.construct.bytes),
        target: draft.target,
        src_id,
        location: draft.location,
    })
}

fn require_metadata_provenance(sources: &[SourceFragment]) -> Result<(), ScanError> {
    if sources.is_empty() {
        Err(malformed_adapter_output(
            "managed Test draft has no metadata source provenance",
        ))
    } else {
        Ok(())
    }
}

fn validate_current_fragment(
    root: &Path,
    adapter: &AdapterId,
    fragment: &SourceFragment,
) -> Result<(), ScanError> {
    let location = &fragment.location;
    if location.adapter != *adapter || location.locator.is_empty() {
        return Err(malformed_adapter_output(
            "source fragment has an invalid adapter or locator",
        ));
    }
    let relative = Path::new(location.path.as_str());
    if location.path.as_str().is_empty()
        || relative
            .components()
            .any(|component| !matches!(component, std::path::Component::Normal(_)))
    {
        return Err(malformed_adapter_output(
            "source fragment path must be project-relative without traversal",
        ));
    }
    if location.byte_range.start_line == 0
        || location.byte_range.end_line < location.byte_range.start_line
    {
        return Err(malformed_adapter_output(
            "source fragment has an invalid line range",
        ));
    }
    let path = root.join(relative);
    let canonical_root = fs::canonicalize(root).map_err(|error| {
        malformed_adapter_output(format!("cannot resolve project root: {error}"))
    })?;
    let canonical_path = fs::canonicalize(&path).map_err(|error| {
        malformed_adapter_output(format!(
            "cannot resolve current source `{}`: {error}",
            location.path
        ))
    })?;
    if !canonical_path.starts_with(&canonical_root) {
        return Err(malformed_adapter_output(format!(
            "source fragment path `{}` escapes the project root",
            location.path
        )));
    }
    let current = fs::read(&canonical_path).map_err(|error| {
        malformed_adapter_output(format!(
            "cannot read current source `{}`: {error}",
            location.path
        ))
    })?;
    let observed = current
        .get(location.byte_range.start..location.byte_range.end)
        .ok_or_else(|| {
            malformed_adapter_output(format!(
                "source range for `{}` is outside current bytes",
                location.path
            ))
        })?;
    if observed != fragment.bytes {
        return Err(malformed_adapter_output(format!(
            "source fragment bytes for `{}` do not match the current range",
            location.path
        )));
    }
    Ok(())
}

fn malformed_adapter_output(message: impl Into<String>) -> ScanError {
    ScanError::Adapter(AdapterError::MalformedOutput(message.into()))
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
    let projection = rust_cargo_discovery_projection(config);
    let batch = RustCargoDiscovery.discover(root, &projection)?;
    let files = batch
        .discovered_tests
        .iter()
        .map(|draft| draft.location.path.as_str().to_owned())
        .chain(
            batch
                .source_targets
                .iter()
                .map(|draft| draft.location.path.as_str().to_owned()),
        )
        .collect::<BTreeSet<_>>()
        .len();
    let materialized = materialize_discovery_batch(root, batch)?;
    let mut result = ScanResult {
        summary: ScanSummary {
            files,
            tests: materialized.managed_tests.len(),
            sources: materialized.source_targets.len(),
        },
        tests: materialized.managed_tests,
        sources: materialized.source_targets,
        diagnostics: materialized.diagnostics,
    };
    result
        .diagnostics
        .extend(cross_entity_diagnostics(&result.tests, &result.sources, &vo_ids));
    result.diagnostics.extend(record_diagnostics(
        root,
        &entity_ids,
        &result.tests,
        &result.sources,
    ));
    Ok(result)
}

/// Projects the `rust-cargo` config section into the neutral canonical
/// projection the discovery adapter consumes. The adapter never reads
/// `ProjectConfig`; core owns the config → projection mapping.
fn rust_cargo_discovery_projection(config: &ProjectConfig) -> CanonicalProjection {
    let include = config
        .rust_cargo()
        .scan
        .include
        .iter()
        .map(|value| CanonicalProjection::String(value.clone()))
        .collect();
    let mut map = BTreeMap::new();
    map.insert(
        "package".to_owned(),
        CanonicalProjection::String(config.project.name.clone()),
    );
    map.insert("include".to_owned(), CanonicalProjection::List(include));
    CanonicalProjection::Map(map)
}

fn projection_string(projection: &CanonicalProjection, key: &str) -> Option<String> {
    match projection {
        CanonicalProjection::Map(map) => match map.get(key) {
            Some(CanonicalProjection::String(value)) => Some(value.clone()),
            _ => None,
        },
        _ => None,
    }
}

fn projection_strings(projection: &CanonicalProjection, key: &str) -> Vec<String> {
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

fn package_name(root: &Path) -> Option<String> {
    cargo_manifest(root).and_then(|manifest| manifest.package.map(|package| package.name))
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
        known_ids.insert(source.target.normalized());
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
    validate_parent_graph(
        root,
        &layout.req_dir(),
        &req_parents,
        "REQ",
        &mut diagnostics,
    );
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
    validate_parent_graph(root, &layout.vo_dir(), &vo_parents, "VO", &mut diagnostics);

    validate_relations(&layout, &known_ids, &mut diagnostics);
    validate_vo_warnings(&layout, &vos, tests, &mut diagnostics);
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
    let location = record_location(root, &path, id);
    if !is_valid_entity_id(id, "SPEC-") {
        diagnostics.push(
            Diagnostic::error(
                "E-SCAN-010",
                format!("SPEC id `{id}` has an invalid format"),
            )
            .with_location(location.clone()),
        );
    }
    let text = match read_text(&path) {
        Ok(text) => text,
        Err(error) => {
            diagnostics.push(
                Diagnostic::error("E-SCAN-010", format!("SPEC {id} cannot be read: {error}"))
                    .with_location(location.clone()),
            );
            return;
        }
    };
    if let Some(missing) = missing_fields(&text, &["id", "kind", "path", "sha256", "registered_at"])
    {
        diagnostics.push(
            Diagnostic::error(
                "E-SCAN-010",
                format!("SPEC {id} is missing required fields: {missing}"),
            )
            .with_location(location.clone()),
        );
    }
    if !matches!(
        yaml_scalar_value(&text, "kind").as_deref(),
        Some("document" | "api-schema" | "type-spec" | "db-schema" | "other")
    ) {
        diagnostics.push(
            Diagnostic::error("E-SCAN-010", format!("SPEC {id} has an invalid kind"))
                .with_location(location.clone()),
        );
    }
    let record = match read_spec(layout, id) {
        Ok(record) => record,
        Err(error) => {
            diagnostics.push(
                Diagnostic::error(
                    "E-SCAN-010",
                    format!("SPEC {id} has an invalid schema: {error}"),
                )
                .with_location(location.clone()),
            );
            return;
        }
    };
    if record.id.as_str() != id {
        diagnostics.push(
            Diagnostic::error(
                "E-SCAN-010",
                format!("SPEC file name {id} does not match record id {}", record.id),
            )
            .with_location(location.clone()),
        );
    }
    let relative_path = Path::new(&record.path);
    if !is_safe_relative_path(relative_path) {
        diagnostics.push(
            Diagnostic::error(
                "E-SCAN-010",
                format!("SPEC {id} path must be project-relative: {}", record.path),
            )
            .with_location(location.clone()),
        );
        return;
    }
    let source_path = root.join(relative_path);
    let bytes = match fs::read(&source_path) {
        Ok(bytes) => bytes,
        Err(error) => {
            diagnostics.push(
                Diagnostic::error(
                    "E-SCAN-010",
                    format!("SPEC {id} path {} cannot be read: {error}", record.path),
                )
                .with_location(location.clone()),
            );
            return;
        }
    };
    let source = String::from_utf8_lossy(&bytes);
    let actual_hash = hash_specification_source(&source);
    if actual_hash != record.sha256 {
        diagnostics.push(
            Diagnostic::warning(
                "W-SCAN-104",
                format!(
                    "SPEC {id} hash is stale: recorded {}, actual {}",
                    record.sha256, actual_hash
                ),
            )
            .with_location(location),
        );
    }
}

fn validate_req_record(
    layout: &VerifyLayout,
    id: &str,
    diagnostics: &mut Vec<Diagnostic>,
) -> Option<ReqRecord> {
    let path = layout.req_dir().join(format!("{id}.yaml"));
    let location = record_location(&layout.root, &path, id);
    if !is_valid_entity_id(id, "REQ-") {
        diagnostics.push(
            Diagnostic::error("E-SCAN-010", format!("REQ id `{id}` has an invalid format"))
                .with_location(location.clone()),
        );
    }
    let text = match read_text(&path) {
        Ok(text) => text,
        Err(error) => {
            diagnostics.push(
                Diagnostic::error("E-SCAN-010", format!("REQ {id} cannot be read: {error}"))
                    .with_location(location.clone()),
            );
            return None;
        }
    };
    if let Some(missing) = missing_fields(&text, &["id", "summary", "status", "created", "updated"])
    {
        diagnostics.push(
            Diagnostic::error(
                "E-SCAN-010",
                format!("REQ {id} is missing required fields: {missing}"),
            )
            .with_location(location.clone()),
        );
    }
    let record = read_req(layout, id).ok()?;
    if record.id.as_str() != id {
        diagnostics.push(
            Diagnostic::error(
                "E-SCAN-010",
                format!("REQ file name {id} does not match record id {}", record.id),
            )
            .with_location(location.clone()),
        );
    }
    if !matches!(record.status.as_str(), "active" | "withdrawn") {
        diagnostics.push(
            Diagnostic::error(
                "E-SCAN-010",
                format!("REQ {id} has invalid status {}", record.status),
            )
            .with_location(location),
        );
    }
    Some(record)
}

fn validate_vo_record(
    layout: &VerifyLayout,
    id: &str,
    diagnostics: &mut Vec<Diagnostic>,
) -> Option<VoRecord> {
    let path = layout.vo_dir().join(format!("{id}.yaml"));
    let location = record_location(&layout.root, &path, id);
    if !is_valid_entity_id(id, "VO-") {
        diagnostics.push(
            Diagnostic::error("E-SCAN-010", format!("VO id `{id}` has an invalid format"))
                .with_location(location.clone()),
        );
    }
    let text = match read_text(&path) {
        Ok(text) => text,
        Err(error) => {
            diagnostics.push(
                Diagnostic::error("E-SCAN-010", format!("VO {id} cannot be read: {error}"))
                    .with_location(location.clone()),
            );
            return None;
        }
    };
    // `status` is a version 1 compatibility field, never a required one: the
    // effective value is derived from Approvals.
    if let Some(missing) = missing_fields(&text, &["id", "claim", "created", "updated"]) {
        diagnostics.push(
            Diagnostic::error(
                "E-SCAN-010",
                format!("VO {id} is missing required fields: {missing}"),
            )
            .with_location(location.clone()),
        );
    }
    let record = read_vo(layout, id).ok()?;
    if record.id.as_str() != id {
        diagnostics.push(
            Diagnostic::error(
                "E-SCAN-010",
                format!("VO file name {id} does not match record id {}", record.id),
            )
            .with_location(location.clone()),
        );
    }
    if let Some(status) = &record.status {
        diagnostics.push(
            Diagnostic::warning(
                "W-STORE-001",
                format!(
                    "VO {id} carries the non-canonical compatibility field status {status}; \
                     the effective status is derived from Approvals"
                ),
            )
            .with_location(location.clone()),
        );
    }
    if let Some(policy) = &record.coverage_policy {
        if !matches!(
            policy.as_str(),
            "independent-axes" | "full-product" | "explicit"
        ) {
            diagnostics.push(
                Diagnostic::error(
                    "E-SCAN-010",
                    format!("VO {id} has invalid coverage_policy {policy}"),
                )
                .with_location(location.clone()),
            );
        }
    }
    if let Some(message) = invalid_vo_dimensions(&record) {
        diagnostics.push(
            Diagnostic::error("E-SCAN-010", format!("VO {id} {message}")).with_location(location),
        );
    }
    Some(record)
}

fn invalid_vo_dimensions(record: &VoRecord) -> Option<String> {
    let mut names = BTreeSet::new();
    for dimension in &record.dimensions {
        if dimension.name.trim().is_empty() || !names.insert(dimension.name.as_str()) {
            return Some("has an empty or duplicate dimension name".to_owned());
        }
        let mut partitions = BTreeSet::new();
        if dimension.partitions.is_empty()
            || dimension
                .partitions
                .iter()
                .any(|partition| partition.trim().is_empty() || !partitions.insert(partition))
        {
            return Some(format!(
                "dimension {} has empty or duplicate partitions",
                dimension.name
            ));
        }
    }

    match record.coverage_policy.as_deref() {
        None if !record.combinations.is_empty() => {
            return Some("has combinations without a coverage_policy".to_owned());
        }
        Some("independent-axes" | "full-product" | "explicit") if record.dimensions.is_empty() => {
            return Some("has a coverage_policy without dimensions".to_owned());
        }
        Some("explicit") => {
            if record.combinations.is_empty() {
                return Some("explicit coverage requires combinations".to_owned());
            }
            let mut unique = BTreeSet::new();
            for combination in &record.combinations {
                if combination.len() != record.dimensions.len()
                    || !combination
                        .iter()
                        .zip(&record.dimensions)
                        .all(|(partition, dimension)| dimension.partitions.contains(partition))
                {
                    return Some(
                        "has an explicit combination outside the declared dimensions".to_owned(),
                    );
                }
                if !unique.insert(combination) {
                    return Some("has duplicate explicit combinations".to_owned());
                }
            }
        }
        Some("independent-axes" | "full-product") if !record.combinations.is_empty() => {
            return Some("stores combinations for a non-explicit policy".to_owned());
        }
        _ => {}
    }
    None
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

fn record_location(root: &Path, path: &Path, entity: &str) -> SourceLocation {
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

fn validate_parent_graph(
    root: &Path,
    directory: &Path,
    parents: &BTreeMap<String, Option<String>>,
    kind: &str,
    diagnostics: &mut Vec<Diagnostic>,
) {
    for (id, parent) in parents {
        if let Some(parent) = parent {
            if !parents.contains_key(parent) {
                diagnostics.push(
                    Diagnostic::error(
                        "E-SCAN-008",
                        format!("{kind} {id} references missing parent {parent}"),
                    )
                    .with_location(record_location(
                        root,
                        &directory.join(format!("{id}.yaml")),
                        id,
                    )),
                );
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
                    diagnostics.push(
                        Diagnostic::error(
                            "E-SCAN-008",
                            format!("{kind} parent cycle: {}", cycle.join(" -> ")),
                        )
                        .with_location(record_location(
                            root,
                            &directory.join(format!("{current}.yaml")),
                            &current,
                        )),
                    );
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
    let entries = match fs::read_dir(layout.relation_dir()) {
        Ok(entries) => entries,
        Err(_) => return,
    };
    let mut paths = entries
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| path.extension().and_then(|value| value.to_str()) == Some("yaml"))
        .collect::<Vec<_>>();
    paths.sort();
    let mut payload_counts = BTreeMap::<String, usize>::new();
    for path in &paths {
        if let Some(payload) = path
            .file_stem()
            .and_then(|value| value.to_str())
            .and_then(relation_ulid_payload)
        {
            *payload_counts.entry(payload.to_owned()).or_default() += 1;
        }
    }
    for path in paths {
        let file_id = path
            .file_stem()
            .and_then(|value| value.to_str())
            .unwrap_or_default()
            .to_owned();
        let location = record_location(&layout.root, &path, &file_id);
        if let Some(payload) = relation_ulid_payload(&file_id) {
            if payload_counts.get(payload).copied().unwrap_or_default() > 1 {
                diagnostics.push(
                    Diagnostic::error(
                        "E-SCAN-010",
                        format!(
                            "relation {file_id} is not adopted because multiple files use ULID payload {payload}"
                        ),
                    )
                    .with_location(location.clone()),
                );
                continue;
            }
        }
        let text = match read_text(&path) {
            Ok(text) => text,
            Err(error) => {
                diagnostics.push(
                    Diagnostic::error(
                        "E-SCAN-010",
                        format!("relation {file_id} cannot be read: {error}"),
                    )
                    .with_location(location.clone()),
                );
                continue;
            }
        };
        let relation = match RelationRecord::from_yaml(&text, &file_id) {
            Ok(relation) => relation,
            Err(error) => {
                diagnostics.push(
                    Diagnostic::error(
                        "E-SCAN-010",
                        format!("relation {file_id} has an invalid schema: {error}"),
                    )
                    .with_location(location.clone()),
                );
                continue;
            }
        };
        for (field, value) in [("from", relation.from), ("to", relation.to)] {
            if !known_ids.contains(&value) {
                diagnostics.push(
                    Diagnostic::error(
                        "E-SCAN-009",
                        format!("relation {file_id} {field} references missing entity {value}"),
                    )
                    .with_location(location.clone()),
                );
            }
        }
    }
}

fn validate_vo_warnings(
    layout: &VerifyLayout,
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
            diagnostics.push(
                Diagnostic::warning(
                    "W-SCAN-102",
                    format!("VO {id} is isolated and has no covering test"),
                )
                .with_location(record_location(
                    &layout.root,
                    &layout.vo_dir().join(format!("{id}.yaml")),
                    id,
                )),
            );
        }
    }
    for test in tests {
        for vo_id in &test.covers {
            if child_ids.contains(vo_id.as_str()) {
                diagnostics.push(
                    Diagnostic::warning(
                        "W-SCAN-103",
                        format!("test {} covers non-leaf VO {}", test.id, vo_id),
                    )
                    .with_location(test.location.clone()),
                );
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
        let location = record_location(&layout.root, &path, &file_id);
        let text = match read_text(&path) {
            Ok(text) => text,
            Err(error) => {
                diagnostics.push(
                    Diagnostic::error(
                        "E-SCAN-010",
                        format!("approval {file_id} cannot be read: {error}"),
                    )
                    .with_location(location.clone()),
                );
                continue;
            }
        };
        let mut invalid = false;
        if !is_valid_ulid(&file_id) {
            diagnostics.push(
                Diagnostic::error(
                    "E-SCAN-010",
                    format!("approval file name {file_id} is not a valid ULID"),
                )
                .with_location(location.clone()),
            );
            invalid = true;
        }
        if let Some(missing) =
            missing_fields(&text, &["id", "subject", "subject_hash", "approved_at"])
        {
            diagnostics.push(
                Diagnostic::error(
                    "E-SCAN-010",
                    format!("approval {file_id} is missing required fields: {missing}"),
                )
                .with_location(location.clone()),
            );
            invalid = true;
        }
        let approval = match read_approval(&path) {
            Ok(approval) => approval,
            Err(error) => {
                diagnostics.push(
                    Diagnostic::error(
                        "E-SCAN-010",
                        format!("approval {file_id} has an invalid schema: {error}"),
                    )
                    .with_location(location.clone()),
                );
                continue;
            }
        };
        if approval.id != file_id {
            diagnostics.push(
                Diagnostic::error(
                    "E-SCAN-010",
                    format!(
                        "approval file name {file_id} does not match record id {}",
                        approval.id
                    ),
                )
                .with_location(location.clone()),
            );
            invalid = true;
        }
        if invalid {
            continue;
        }
        let subject = approval.subject.as_str();
        if !current_hashes.contains_key(subject) {
            diagnostics.push(
                Diagnostic::error(
                    "E-SCAN-010",
                    format!("approval {file_id} references missing VO {subject}"),
                )
                .with_location(location),
            );
        }
    }
    for (id, vo) in vos {
        let subject = current_approval_subject(layout, vo);
        let derived = derive_vo_status(layout, vo, subject.as_ref());
        for (approval_id, invalidity) in &derived.invalid {
            diagnostics.push(
                Diagnostic::warning(
                    "W-STORE-002",
                    format!("approval {approval_id} does not approve VO {id}: {invalidity}"),
                )
                .with_location(record_location(
                    &layout.root,
                    &layout.approvals_dir().join(format!("{approval_id}.yaml")),
                    approval_id,
                )),
            );
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

fn is_valid_entity_id(id: &str, prefix: &str) -> bool {
    id.starts_with(prefix)
        && id.len() > prefix.len()
        && id.chars().all(|character| {
            character.is_ascii_uppercase() || character.is_ascii_digit() || character == '-'
        })
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

/// Core-owned cross-entity resolution over the materialized discovery result:
/// dangling `covers` (E-SCAN-003), unresolved targets (E-SCAN-004), and
/// permanent SRC ID collisions (E-SCAN-011). The adapter cannot see `.verify/`
/// records or the repository-global source index, so these stay in core.
fn cross_entity_diagnostics(
    tests: &[TestEntity],
    sources: &[SourceTarget],
    vo_ids: &BTreeSet<String>,
) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();
    let mut locators = BTreeMap::<String, usize>::new();
    let mut src_ids = BTreeMap::<String, usize>::new();
    for source in sources {
        *locators.entry(source.target.normalized()).or_default() += 1;
        if let Some(src_id) = &source.src_id {
            *src_ids.entry(src_id.as_str().to_owned()).or_default() += 1;
        }
    }
    for test in tests {
        for vo_id in &test.covers {
            if !vo_ids.contains(vo_id.as_str()) {
                diagnostics.push(
                    Diagnostic::error(
                        "E-SCAN-003",
                        format!("test `{}` references missing VO `{vo_id}`", test.id),
                    )
                    .with_location(test.location.clone()),
                );
            }
        }
    }
    for source in sources {
        let Some(src_id) = &source.src_id else {
            continue;
        };
        if src_ids.get(src_id.as_str()).copied().unwrap_or_default() > 1 {
            diagnostics.push(
                Diagnostic::error(
                    "E-SCAN-011",
                    format!(
                        "permanent SRC ID `{src_id}` is claimed by more than one Source Target"
                    ),
                )
                .with_location(source.location.clone()),
            );
        }
    }
    for test in tests {
        for target in &test.targets {
            let resolved = match target {
                TargetRef::Locator { .. } => locators.get(&target.normalized()).copied() == Some(1),
                TargetRef::SrcId(src_id) => src_ids.get(src_id.as_str()).copied() == Some(1),
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
    diagnostics
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

fn cargo_manifest(root: &Path) -> Option<CargoManifest> {
    let text = fs::read_to_string(root.join("Cargo.toml")).ok()?;
    let manifest = toml::from_str::<CargoManifest>(&text).ok()?;
    manifest.package.as_ref()?;
    Some(manifest)
}

fn source_context(root: &Path, path: &Path, fallback_package: &str) -> SourceContext {
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

fn cargo_target_roots(package_root: &Path, manifest: &CargoManifest) -> Vec<CargoTargetRoot> {
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

fn discovered_target_roots(directory: &Path) -> Vec<(PathBuf, String)> {
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

fn source_slice<'a>(source: &'a str, location: &SourceLocation) -> &'a str {
    source
        .get(location.byte_range.start..location.byte_range.end)
        .unwrap_or("")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use std::time::{SystemTime, UNIX_EPOCH};
    use vtest_adapter_api::{
        AdapterCapability, AdapterDescriptor, AdapterRegistration, AdapterRegistry,
        SourceDiscoveryAdapter,
    };
    use vtest_model::CanonicalProjection;
    use vtest_store::{init_project, new_record_id};

    #[derive(Clone)]
    struct SyntheticDiscoveryAdapter {
        batch: DiscoveryBatch,
    }

    impl SourceDiscoveryAdapter for SyntheticDiscoveryAdapter {
        fn discover(
            &self,
            _root: &Path,
            _config: &CanonicalProjection,
        ) -> Result<DiscoveryBatch, AdapterError> {
            Ok(self.batch.clone())
        }
    }

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
        fs::write(
            root.join("Cargo.toml"),
            "[package]\nname = \"fixture\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
        )
        .unwrap();
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

    fn discovery_materialization_fixture() -> (PathBuf, DiscoveryBatch) {
        let suffix = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = std::env::temp_dir().join(format!("vtest-materialize-{suffix}"));
        fs::create_dir_all(root.join("source")).unwrap();
        fs::create_dir_all(root.join("metadata")).unwrap();

        let source = b"header\nscenario adding two values\nimplementation add\n";
        let test_start = source
            .windows(b"scenario adding two values".len())
            .position(|window| window == b"scenario adding two values")
            .unwrap();
        let test_end = test_start + b"scenario adding two values".len();
        let target_start = source
            .windows(b"implementation add".len())
            .position(|window| window == b"implementation add")
            .unwrap();
        let target_end = target_start + b"implementation add".len();
        fs::write(root.join("source/cases.synth"), source).unwrap();

        let metadata = b"TEST-SYNTH-ADD: VO-SYNTH-ADD";
        fs::write(root.join("metadata/tests.txt"), metadata).unwrap();
        let adapter = AdapterId::new("synthetic");
        let test_location = SourceLocation {
            adapter: adapter.clone(),
            path: ProjectPath::new("source/cases.synth"),
            locator: "scenario[adding two values]".to_owned(),
            byte_range: SourceRange {
                start: test_start,
                end: test_end,
                start_line: 2,
                end_line: 2,
            },
        };
        let target_location = SourceLocation {
            adapter: adapter.clone(),
            path: ProjectPath::new("source/cases.synth"),
            locator: "component[add]".to_owned(),
            byte_range: SourceRange {
                start: target_start,
                end: target_end,
                start_line: 3,
                end_line: 3,
            },
        };
        let metadata_location = SourceLocation {
            adapter: adapter.clone(),
            path: ProjectPath::new("metadata/tests.txt"),
            locator: "tests[TEST-SYNTH-ADD]".to_owned(),
            byte_range: SourceRange {
                start: 0,
                end: metadata.len(),
                start_line: 1,
                end_line: 1,
            },
        };
        let target = TargetRef::Locator {
            adapter: adapter.clone(),
            value: "component[add]".to_owned(),
        };
        let managed = ManagedTestDraft {
            id: TestId::new("TEST-SYNTH-ADD"),
            covers: vec![VoId::new("VO-SYNTH-ADD")],
            targets: vec![target.clone()],
            intent: "adding two values returns their sum".to_owned(),
            input: None,
            expect: None,
            kind: Some("scenario".to_owned()),
            cases: Vec::new(),
            related: Vec::new(),
            execution: ExecutionDescriptor {
                adapter: adapter.clone(),
                project: None,
                suite: None,
                selector: "scenario[adding two values]".to_owned(),
            },
        };
        let batch = DiscoveryBatch {
            adapter: adapter.clone(),
            completeness: DiscoveryCompleteness::Complete,
            discovered_tests: vec![DiscoveredTestDraft {
                adapter: adapter.clone(),
                location: test_location.clone(),
                construct: SourceFragment {
                    location: test_location,
                    bytes: source[test_start..test_end].to_vec(),
                },
                metadata_sources: vec![SourceFragment {
                    location: metadata_location,
                    bytes: metadata.to_vec(),
                }],
                managed: ManagedTestDraftLink::One(managed),
            }],
            source_targets: vec![SourceTargetDraft {
                target,
                src_id: None,
                location: target_location.clone(),
                construct: SourceFragment {
                    location: target_location,
                    bytes: source[target_start..target_end].to_vec(),
                },
            }],
            diagnostics: Vec::new(),
        };
        (root, batch)
    }

    #[test]
    fn core_validates_current_bytes_before_materializing_adapter_drafts() {
        let (root, batch) = discovery_materialization_fixture();
        let expected_construct_hash =
            ContentHash::from_bytes(&batch.discovered_tests[0].construct.bytes);
        let expected_target_hash = hash_target_subject(
            &batch.source_targets[0].target,
            &batch.source_targets[0].construct.bytes,
        );
        let adapter = AdapterId::new("synthetic");
        let mut registration = AdapterRegistration::new(AdapterDescriptor {
            id: adapter.clone(),
            languages: vec!["synthetic".to_owned()],
            capabilities: vec![AdapterCapability::SourceDiscovery],
            config_namespace: "synthetic".to_owned(),
        });
        registration.source_discovery = Some(Arc::new(SyntheticDiscoveryAdapter { batch }));
        let registry = AdapterRegistry::from_registrations([registration]).unwrap();
        let observed = registry
            .source_discovery(&adapter)
            .unwrap()
            .discover(&root, &CanonicalProjection::Null)
            .unwrap();

        let materialized = materialize_discovery_batch(&root, observed).unwrap();
        assert_eq!(materialized.adapter, AdapterId::new("synthetic"));
        assert_eq!(materialized.discovered_tests.len(), 1);
        assert_eq!(materialized.managed_tests.len(), 1);
        assert_eq!(materialized.source_targets.len(), 1);
        assert_eq!(
            materialized.discovered_tests[0].content_hash,
            expected_construct_hash
        );
        assert_eq!(
            materialized.discovered_tests[0].managed,
            ManagedTestLink::One(TestId::new("TEST-SYNTH-ADD"))
        );
        assert_eq!(
            materialized.source_targets[0].content_hash,
            expected_target_hash
        );
        assert!(materialized.managed_tests[0].validate().is_ok());
        assert_ne!(
            materialized.managed_tests[0].content_hash,
            expected_construct_hash
        );
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn core_rejects_stale_fragment_bytes_without_materializing_entities() {
        let (root, batch) = discovery_materialization_fixture();
        fs::write(
            root.join("source/cases.synth"),
            b"header\nscenario changed values!!!\nimplementation add\n",
        )
        .unwrap();
        let error = materialize_discovery_batch(&root, batch).unwrap_err();
        assert!(error.to_string().contains("E-ADAPTER-002"));
        assert!(error.to_string().contains("do not match the current range"));
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn core_rejects_a_permanent_src_id_as_a_canonical_source_target() {
        let (root, mut batch) = discovery_materialization_fixture();
        batch.source_targets[0].target = TargetRef::SrcId(SrcId::new("SRC-SYNTH-ADD"));
        let error = materialize_discovery_batch(&root, batch).unwrap_err();
        assert!(error.to_string().contains("E-ADAPTER-002"));
        assert!(
            error.to_string().contains("must be a locator"),
            "a permanent SRC ID refers to a Source Target and is never its canonical target: {error}"
        );
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn a_permanent_src_id_travels_beside_the_target_without_entering_its_subject() {
        let (anonymous_root, anonymous) = discovery_materialization_fixture();
        let without = materialize_discovery_batch(&anonymous_root, anonymous).unwrap();
        assert_eq!(without.source_targets[0].src_id, None);
        let subject = without.source_targets[0].content_hash.clone();
        fs::remove_dir_all(anonymous_root).unwrap();

        let (root, mut batch) = discovery_materialization_fixture();
        batch.source_targets[0].src_id = Some(SrcId::new("SRC-SYNTH-ADD"));
        let identified = materialize_discovery_batch(&root, batch).unwrap();
        assert_eq!(
            identified.source_targets[0]
                .src_id
                .as_ref()
                .map(SrcId::as_str),
            Some("SRC-SYNTH-ADD"),
            "the permanent identity the adapter declared must reach the Source Target"
        );
        assert_eq!(
            identified.source_targets[0].content_hash, subject,
            "granting a permanent identity does not move the Source Target subject"
        );
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn core_rejects_missing_metadata_provenance_and_incomplete_discovery() {
        let (root, mut batch) = discovery_materialization_fixture();
        batch.discovered_tests[0].metadata_sources.clear();
        let error = materialize_discovery_batch(&root, batch).unwrap_err();
        assert!(error.to_string().contains("E-ADAPTER-002"));
        assert!(error.to_string().contains("provenance"));

        let (incomplete_root, mut incomplete) = discovery_materialization_fixture();
        incomplete.completeness = DiscoveryCompleteness::Incomplete;
        let error = materialize_discovery_batch(&incomplete_root, incomplete).unwrap_err();
        assert!(error.to_string().contains("E-ADAPTER-002"));
        assert!(error.to_string().contains("incomplete discovery"));
        fs::remove_dir_all(root).unwrap();
        fs::remove_dir_all(incomplete_root).unwrap();
    }

    #[test]
    fn core_materializes_missing_and_multiple_links_without_guessing() {
        let (missing_root, mut missing_batch) = discovery_materialization_fixture();
        missing_batch.discovered_tests[0].managed = ManagedTestDraftLink::Missing;
        missing_batch.discovered_tests[0].metadata_sources.clear();
        let missing = materialize_discovery_batch(&missing_root, missing_batch).unwrap();
        assert_eq!(
            missing.discovered_tests[0].managed,
            ManagedTestLink::Missing
        );
        assert!(missing.managed_tests.is_empty());

        let (multiple_root, mut multiple_batch) = discovery_materialization_fixture();
        let ManagedTestDraftLink::One(first) = multiple_batch.discovered_tests[0].managed.clone()
        else {
            unreachable!("fixture has one managed Test draft")
        };
        let mut second = first.clone();
        second.id = TestId::new("TEST-SYNTH-ADD-SECOND");
        multiple_batch.discovered_tests[0].managed =
            ManagedTestDraftLink::Multiple(vec![second, first]);
        let multiple = materialize_discovery_batch(&multiple_root, multiple_batch).unwrap();
        assert_eq!(multiple.managed_tests.len(), 2);
        assert_eq!(
            multiple.discovered_tests[0].managed,
            ManagedTestLink::Multiple(vec![
                TestId::new("TEST-SYNTH-ADD"),
                TestId::new("TEST-SYNTH-ADD-SECOND"),
            ])
        );

        fs::remove_dir_all(missing_root).unwrap();
        fs::remove_dir_all(multiple_root).unwrap();
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
        assert_eq!(result.tests[0].execution.selector, "adds");
        assert_eq!(
            result.tests[0].execution.project.as_deref(),
            Some("fixture")
        );
        assert_eq!(
            result.tests[0].execution.suite.as_ref(),
            Some(&TestSuite {
                kind: "integration".to_owned(),
                name: Some("calc".to_owned())
            })
        );
    }

    #[test]
    fn missing_or_invalid_cargo_metadata_is_fail_closed() {
        let root = fixture();
        fs::write(root.join("Cargo.toml"), "[package\ninvalid = true\n").unwrap();

        let result = scan_project(&root).unwrap();
        assert!(result.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "E-SCAN-004"
                && diagnostic.location.as_ref().is_some_and(|location| {
                    location.path.as_str() == "tests/calc.rs" && location.locator == "adds"
                })
        }));
        assert_eq!(
            result.tests[0]
                .execution
                .suite
                .as_ref()
                .map(|suite| suite.kind.as_str()),
            Some("unknown")
        );

        let root = fixture();
        fs::remove_file(root.join("Cargo.toml")).unwrap();
        let result = scan_project(&root).unwrap();
        assert!(result.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "E-SCAN-004"
                && diagnostic.location.as_ref().is_some_and(|location| {
                    location.path.as_str() == "tests/calc.rs" && location.locator == "adds"
                })
        }));
        assert_eq!(
            result.tests[0]
                .execution
                .suite
                .as_ref()
                .map(|suite| suite.kind.as_str()),
            Some("unknown")
        );
    }

    #[test]
    fn resolves_workspace_packages_targets_and_external_module_filters() {
        let root = fixture();
        fs::create_dir_all(root.join("crates/parser/src/runtime")).unwrap();
        fs::create_dir_all(root.join("crates/parser/src/bin")).unwrap();
        fs::create_dir_all(root.join("crates/parser/tests/suite")).unwrap();
        fs::write(
            root.join("crates/parser/Cargo.toml"),
            "[package]\nname = \"parser-crate\" # an inline TOML comment\nversion = \"0.1.0\"\nedition = \"2021\"\nautobins = false\nautotests = false\n\n[lib]\npath = \"src/runtime/mod.rs\"\n\n[[bin]]\nname = \"parser-check\"\npath = \"src/bin/check.rs\"\n\n[[test]]\nname = \"parser-suite\"\npath = \"tests/suite/main.rs\"\n",
        )
        .unwrap();
        fs::write(
            root.join("crates/parser/src/runtime/mod.rs"),
            "pub mod parser;\n",
        )
        .unwrap();
        fs::write(
            root.join("crates/parser/src/runtime/parser.rs"),
            r#"
pub fn parse() {}

#[cfg(test)]
mod tests {
    /// @vtest.id TEST-PARSER-MODULE
    /// @vtest.covers VO-ADD
    /// @vtest.target crates/parser/src/runtime/parser.rs::parse
    /// @vtest.intent parses from an external module
    #[test]
    fn parses_external_module() { super::parse(); }
}
"#,
        )
        .unwrap();
        fs::write(
            root.join("crates/parser/src/bin/check.rs"),
            r#"
fn main() {}

/// @vtest.id TEST-PARSER-BIN
/// @vtest.covers VO-ADD
/// @vtest.target crates/parser/src/runtime/parser.rs::parse
/// @vtest.intent checks the parser binary
#[test]
fn checks_binary() {}
"#,
        )
        .unwrap();
        fs::write(
            root.join("crates/parser/tests/suite/main.rs"),
            "mod support;\n",
        )
        .unwrap();
        fs::write(
            root.join("crates/parser/tests/suite/support.rs"),
            r#"
pub fn exercise() {}

/// @vtest.id TEST-PARSER-INTEGRATION
/// @vtest.covers VO-ADD
/// @vtest.target crates/parser/src/runtime/parser.rs::parse
/// @vtest.intent parses through an integration target
#[test]
fn parses_integration() { exercise(); }
"#,
        )
        .unwrap();

        let result = scan_project(&root).unwrap();
        let module_test = result
            .tests
            .iter()
            .find(|test| test.id.as_str() == "TEST-PARSER-MODULE")
            .unwrap();
        assert_eq!(
            module_test.execution.project.as_deref(),
            Some("parser-crate")
        );
        assert_eq!(
            module_test
                .execution
                .suite
                .as_ref()
                .map(|suite| suite.kind.as_str()),
            Some("lib")
        );
        assert_eq!(
            module_test.execution.selector,
            "parser::tests::parses_external_module"
        );

        let integration = result
            .tests
            .iter()
            .find(|test| test.id.as_str() == "TEST-PARSER-INTEGRATION")
            .unwrap();
        assert_eq!(
            integration.execution.project.as_deref(),
            Some("parser-crate")
        );
        assert_eq!(
            integration.execution.suite.as_ref(),
            Some(&TestSuite {
                kind: "integration".to_owned(),
                name: Some("parser-suite".to_owned())
            })
        );
        assert_eq!(
            integration.execution.selector,
            "support::parses_integration"
        );

        let binary = result
            .tests
            .iter()
            .find(|test| test.id.as_str() == "TEST-PARSER-BIN")
            .unwrap();
        assert_eq!(binary.execution.project.as_deref(), Some("parser-crate"));
        assert_eq!(
            binary.execution.suite.as_ref(),
            Some(&TestSuite {
                kind: "bin".to_owned(),
                name: Some("parser-check".to_owned())
            })
        );
        assert_eq!(binary.execution.selector, "checks_binary");
    }

    #[test]
    fn ignored_rust_files_are_not_scanned() {
        let root = fixture();
        fs::write(root.join(".gitignore"), "src/ignored.rs\n").unwrap();
        fs::write(root.join(".ignore"), "src/kept.rs\n").unwrap();
        fs::write(root.join("src/ignored.rs"), "this is not rust\n").unwrap();
        fs::write(root.join("src/kept.rs"), "pub fn kept() {}\n").unwrap();

        let result = scan_project(&root).unwrap();
        assert!(!result.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "E-SCAN-001" && diagnostic.message.contains("ignored.rs")
        }));
        assert!(!result
            .sources
            .iter()
            .any(|source| source.location.path.as_str() == "src/ignored.rs"));
        assert!(result
            .sources
            .iter()
            .any(|source| source.location.path.as_str() == "src/kept.rs"));
    }

    #[test]
    fn ambiguous_target_locator_is_not_resolved() {
        let root = fixture();
        fs::write(
            root.join("src/ambiguous.rs"),
            r#"
#[cfg(feature = "left")]
pub fn duplicate() {}
#[cfg(not(feature = "left"))]
pub fn duplicate() {}
"#,
        )
        .unwrap();
        fs::write(
            root.join("tests/ambiguous.rs"),
            r#"
/// @vtest.id TEST-AMBIGUOUS
/// @vtest.covers VO-ADD
/// @vtest.target src/ambiguous.rs::duplicate
/// @vtest.intent rejects an ambiguous source locator
#[test]
fn ambiguous() {}
"#,
        )
        .unwrap();

        let result = scan_project(&root).unwrap();
        assert!(result.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "E-SCAN-004"
                && diagnostic
                    .location
                    .as_ref()
                    .is_some_and(|location| location.locator == "ambiguous")
        }));
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
        assert_eq!(integration.targets.len(), 2);
        assert!(result.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "E-SCAN-005"
                && diagnostic
                    .location
                    .as_ref()
                    .is_some_and(|location| location.locator == "duplicate_target")
        }));
    }

    #[test]
    fn relation_id_aliases_cannot_duplicate_one_ulid_payload() {
        let root = fixture();
        let payload = new_record_id();
        for id in [payload.clone(), format!("REL-{payload}")] {
            fs::write(
                root.join(format!(".verify/rel/{id}.yaml")),
                format!(
                    "id: {id}\ntype: complements\nfrom: VO-ADD\nto: VO-ADD\ncreated: '2026-01-01'\n"
                ),
            )
            .unwrap();
        }

        let result = scan_project(&root).unwrap();
        let duplicates = result
            .diagnostics
            .iter()
            .filter(|diagnostic| {
                diagnostic.code == "E-SCAN-010" && diagnostic.message.contains("ULID payload")
            })
            .collect::<Vec<_>>();
        // Neither spelling is adopted, so both files are diagnosed: a shared
        // payload has no canonical winner.
        assert_eq!(duplicates.len(), 2, "diagnostics: {:?}", result.diagnostics);
        assert!(duplicates
            .iter()
            .all(|diagnostic| diagnostic.location.is_some()));
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

        let relation_id = new_record_id();
        let relation = root.join(format!(".verify/rel/{relation_id}.yaml"));
        fs::write(
            relation,
            format!(
                "id: {relation_id}\ntype: depends-on\nfrom: ENTITY-NOT-FOUND\nto: VO-ADD\ncreated: '2026-01-01'\n"
            ),
        )
        .unwrap();

        let vo_text = fs::read_to_string(root.join(".verify/vo/VO-ADD.yaml")).unwrap();
        let vo_hash = ContentHash::from_text(&vo_text);
        let approval_id = new_record_id();
        fs::write(
            root.join(format!(".verify/approvals/{approval_id}.yaml")),
            format!(
                "id: {approval_id}\nsubject: VO-ADD\nsubject_hash: {vo_hash}\napprover:\n  kind: human\n  id: reviewer\nbasis: []\napproved_at: '2026-01-01'\n"
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
        assert!(
            result
                .diagnostics
                .iter()
                .all(|diagnostic| diagnostic.location.is_some()),
            "every scanner diagnostic must identify its canonical source: {:?}",
            result.diagnostics
        );
    }
}
