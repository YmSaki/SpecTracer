//! Canonical `.verify/` layout and read-side project configuration.
//!
//! Derived indexes are deliberately absent from this crate: callers rebuild
//! them from the canonical records on every operation.

use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    fs, io,
    path::{Path, PathBuf},
};
use thiserror::Error;
use vtest_model::DocumentId;

pub mod canonical;
pub mod forms;
pub mod records;
pub use canonical::*;
pub use forms::*;
pub use records::*;

#[derive(Debug, Error)]
pub enum StoreError {
    #[error("I/O error at {path}: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: io::Error,
    },
    #[error(".verify directory already exists at {0}")]
    AlreadyInitialized(PathBuf),
    #[error("project root does not contain .verify: {0}")]
    NotInitialized(PathBuf),
    #[error("invalid project configuration: {0}")]
    InvalidConfig(String),
    #[error("invalid form schema: {0}")]
    InvalidForm(String),
    #[error("invalid form answers: {0}")]
    InvalidAnswers(String),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct VerifyLayout {
    pub root: PathBuf,
}

impl VerifyLayout {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    pub fn verify_dir(&self) -> PathBuf {
        self.root.join(".verify")
    }

    pub fn config(&self) -> PathBuf {
        self.verify_dir().join("config.yaml")
    }

    pub fn spec_dir(&self) -> PathBuf {
        self.verify_dir().join("spec")
    }

    pub fn req_dir(&self) -> PathBuf {
        self.verify_dir().join("req")
    }

    /// Canonical `document` record directory (詳細設計 v0.1 §2.1). Replaces the
    /// predecessor `spec/`/`req/` split; those accessors stay for the readers
    /// that still use them until PR8 removes the predecessor model.
    pub fn doc_dir(&self) -> PathBuf {
        self.verify_dir().join("doc")
    }

    pub fn vo_dir(&self) -> PathBuf {
        self.verify_dir().join("vo")
    }

    /// Judgment-record directory (詳細設計 v0.1 §2.1, §3.4). New in the
    /// canonical v0.1 layout; there is no predecessor equivalent.
    pub fn decisions_dir(&self) -> PathBuf {
        self.verify_dir().join("decisions")
    }

    pub fn relation_dir(&self) -> PathBuf {
        self.verify_dir().join("rel")
    }

    pub fn forms_dir(&self) -> PathBuf {
        self.verify_dir().join("forms")
    }

    pub fn approvals_dir(&self) -> PathBuf {
        self.verify_dir().join("approvals")
    }

    pub fn audits_dir(&self) -> PathBuf {
        self.verify_dir().join("audits")
    }

    pub fn evidence_dir(&self) -> PathBuf {
        self.verify_dir().join("evidence")
    }

    pub fn cache_dir(&self) -> PathBuf {
        self.verify_dir().join("cache")
    }

    pub fn source_record_dirs(&self) -> [PathBuf; 3] {
        [self.spec_dir(), self.req_dir(), self.vo_dir()]
    }
}

/// Canonical v0.1 project configuration (詳細設計 v0.1 §2.2). The writer's
/// normal form is version 2; an explicit `version: 1` is read as a single
/// implicit `rust-cargo` adapter and converted in-memory to this shape
/// without rewriting the file (§2.4). `version` itself is required (別紙C
/// §18.3.12: the reader accepts exactly versions 1 and 2 — never a config
/// with no declared version), and every key must belong to the schema its
/// declared version actually has — see `ProjectConfig::from_yaml`.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProjectConfig {
    pub version: u32,
    pub project: ProjectSection,
    pub adapters: Vec<AdapterConfig>,
    pub doc: DocSection,
    pub verify: VerifySection,

    /// 詳細設計 v0.1 §2.2: "`gates` field自体の欠落と空 list は「ゲート定義
    /// なし」として受理する" — absence and `gates: []` are equivalent.
    #[serde(default)]
    pub gates: Vec<GateConfig>,

    #[serde(default)]
    pub approval_roles: BTreeMap<String, Vec<String>>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProjectSection {
    pub name: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AdapterConfig {
    pub id: String,
    pub roots: Vec<String>,
    pub scan: ScanSection,
    pub run: RunSection,
}

/// Deliberately no `#[serde(deny_unknown_fields)]` here (unlike its sibling
/// sections): 詳細設計 v0.1 §2.2 delegates adapter-payload validation to the
/// registered adapter itself — "adapter固有設定の検証は登録adapterへ委譲し、
/// coreは未知のnamespaceや値をRust設定として解釈しない". PR2 has no adapter
/// registry yet, so this struct's fixed Rust-cargo-shaped fields are an
/// existing constraint, not this invariant's concern; a registry PR replaces
/// this direct-deserialize with delegated validation instead of tightening it
/// here.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ScanSection {
    pub include: Vec<String>,
    pub assertion_macros: Vec<String>,
}

/// See `ScanSection`'s doc comment: same adapter-delegated-validation reason
/// for not denying unknown fields here.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct RunSection {
    pub coverage: String,
}

/// Orphan-detection roots for the document layer (詳細設計 v0.1 §2.2, §5.6).
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DocSection {
    pub roots: Vec<DocumentId>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct VerifySection {
    pub full_scope: Vec<String>,
}

/// One phase-gate definition (詳細設計 v0.1 §2.2, §11.5).
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GateConfig {
    pub name: String,
    pub require: GateRequirement,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GateRequirement {
    pub verification: String,

    /// 詳細設計 v0.1 §2.2: "`require.approvals` は省略可能とし、省略は「要求
    /// する承認ロールなし（空集合）」として受理する".
    #[serde(default)]
    pub approvals: Vec<String>,
}

/// The fixed four checks (基本仕様 §5) `verify.full_scope` must enumerate
/// exactly — no more, no fewer, no duplicates, no unrecognized names
/// (詳細設計 v0.1 §2.2).
const FIXED_FULL_SCOPE: [&str; 4] = [
    "chain_integrity",
    "orphan_detection",
    "target_binding",
    "oracle_presence",
];

/// The five verification states `gates[].require.verification` may name
/// (基本仕様 §4.1, 詳細設計 v0.1 §2.2). Case-sensitive exact match.
const VERIFICATION_STATES: [&str; 5] = ["PASS", "FAIL", "MISMATCH", "NO_EVIDENCE", "UNKNOWN"];

impl ProjectConfig {
    pub fn default_for(name: impl Into<String>) -> Self {
        Self {
            version: 2,
            project: ProjectSection { name: name.into() },
            adapters: vec![AdapterConfig {
                id: "rust-cargo".to_owned(),
                roots: vec![".".to_owned()],
                scan: ScanSection {
                    include: vec!["src".to_owned(), "tests".to_owned(), "crates".to_owned()],
                    assertion_macros: Vec::new(),
                },
                run: RunSection {
                    coverage: "llvm-cov".to_owned(),
                },
            }],
            doc: DocSection { roots: Vec::new() },
            verify: VerifySection {
                full_scope: FIXED_FULL_SCOPE
                    .iter()
                    .map(|item| (*item).to_owned())
                    .collect(),
            },
            gates: Vec::new(),
            approval_roles: BTreeMap::new(),
        }
    }

    /// Serializes the project configuration in its canonical version 2 shape
    /// via `yaml_serde`, using `ProjectConfig`'s own `Serialize` derive.
    /// Every in-memory `ProjectConfig` this crate constructs already carries
    /// `version: 2` (`default_for`, `from_yaml_v1`, and `from_yaml_v2` all
    /// guarantee it), matching 詳細設計 v0.1 §2.2's "writer の正規形は
    /// version 2".
    pub fn to_yaml(&self) -> String {
        yaml_serde::to_string(self).expect("ProjectConfig always serializes to valid YAML")
    }

    /// Read a project configuration. `version` is read through the YAML
    /// model itself, not a hand-rolled text scan: a config that is not a
    /// YAML mapping, or that has no `version` key, or whose `version` is not
    /// an integer, fails closed rather than being guessed at (基本仕様 §2.4:
    /// "reader は version 1 を...読み取る" and 別紙C §18.3.12: "config reader
    /// はversion 1とversion 2を受理し" both only ever speak of a *declared*
    /// version 1 or 2 — neither states or implies a third "absent" case).
    /// `version: 2` is parsed as written; `version: 1` is parsed under the
    /// version 1 shape and converted in-memory to this (version 2) shape
    /// (§2.4: a read never rewrites the canonical file). Any other version
    /// — malformed, or a number this reader does not recognize — is
    /// rejected: 詳細設計 v0.1 §17.1 lists `config version` itself among the
    /// E-CONFIG-001 conditions, so guessing at an unknown schema version
    /// would be exactly the silent-promotion this system's fail-closed
    /// design forbids. Every key must also belong to the schema its
    /// declared version actually has (`#[serde(deny_unknown_fields)]` on
    /// `ProjectConfig`/`V1Config` and their sub-sections) — the same
    /// E-CONFIG-001 condition covers a declared version whose body does not
    /// match it, e.g. a `version: 1` config carrying a v2-only `gates:` key.
    pub fn from_yaml(text: &str, project_name: impl Into<String>) -> Result<Self, StoreError> {
        let value: yaml_serde::Value = yaml_serde::from_str(text)
            .map_err(|error| StoreError::InvalidConfig(format!("invalid config: {error}")))?;
        let mapping = value.as_mapping().ok_or_else(|| {
            StoreError::InvalidConfig("config.yaml must be a YAML mapping".to_owned())
        })?;
        let version =
            match mapping.get("version") {
                None => return Err(StoreError::InvalidConfig(
                    "config is missing `version`; 1 (compatibility) or 2 (canonical) is required"
                        .to_owned(),
                )),
                Some(version_value) => version_value.as_u64().ok_or_else(|| {
                    StoreError::InvalidConfig(
                        "config version must be a non-negative integer".to_owned(),
                    )
                })?,
            };
        match version {
            1 => Self::from_yaml_v1(value, project_name),
            2 => Self::from_yaml_v2(value),
            other => Err(StoreError::InvalidConfig(format!(
                "unsupported config version {other}; only 1 (compatibility) and 2 (canonical) are recognized"
            ))),
        }
    }

    /// Parses a canonical version 2 configuration directly via `yaml_serde`,
    /// using `ProjectConfig`'s own `Deserialize` derive. A missing
    /// `project`/`adapters`/`doc`/`verify` section fails closed through the
    /// derive's standard "missing field" behavior (none of the four carry
    /// `#[serde(default)]`); `gates`/`approval_roles` default to empty,
    /// matching 詳細設計 v0.1 §2.2's "`gates` field自体の欠落と空 list は
    /// 「ゲート定義なし」として受理する".
    fn from_yaml_v2(value: yaml_serde::Value) -> Result<Self, StoreError> {
        let config: Self = yaml_serde::from_value(value)
            .map_err(|error| StoreError::InvalidConfig(format!("invalid v2 config: {error}")))?;
        validate_v2_config(&config)?;
        Ok(config)
    }

    /// Reads a version 1 configuration and converts it in-memory to the
    /// version 2 shape: a single implicit `rust-cargo` adapter, no doc
    /// roots, no gates, no approval roles.
    fn from_yaml_v1(
        value: yaml_serde::Value,
        project_name: impl Into<String>,
    ) -> Result<Self, StoreError> {
        let v1: V1Config = yaml_serde::from_value(value)
            .map_err(|error| StoreError::InvalidConfig(format!("invalid config: {error}")))?;

        let name = v1
            .project
            .and_then(|section| section.name)
            .unwrap_or_else(|| project_name.into());

        let include = v1
            .scan
            .as_ref()
            .and_then(|scan| scan.include.clone())
            .unwrap_or_else(|| vec!["src".to_owned(), "tests".to_owned(), "crates".to_owned()]);

        let mut assertion_macros = v1
            .scan
            .and_then(|scan| scan.assertion_macros)
            .unwrap_or_default();
        assertion_macros.dedup();
        for macro_name in &assertion_macros {
            if !is_rust_macro_path(macro_name) {
                return Err(StoreError::InvalidConfig(format!(
                    "scan.assertion_macros contains invalid Rust macro path `{macro_name}`"
                )));
            }
        }

        let coverage = v1
            .run
            .and_then(|run| run.coverage)
            .unwrap_or_else(|| "llvm-cov".to_owned());
        if !matches!(coverage.as_str(), "llvm-cov" | "off") {
            return Err(StoreError::InvalidConfig(format!(
                "run.coverage must be `llvm-cov` or `off`, got `{coverage}`"
            )));
        }

        // 詳細設計 v0.1 §2.2: "version 1 では field 欠落を固定4検査として
        // 具体化し、重複または未知項目は E-CONFIG-001 で拒否する...
        // in-memory 補完で受理しない". A *present* full_scope goes straight
        // to validate_full_scope with no dedup step first — a prior version
        // of this reader deduped before validating, which silently hid
        // adjacent duplicates from the very check meant to reject them.
        let full_scope = match v1.verify.and_then(|verify| verify.full_scope) {
            Some(list) => {
                validate_full_scope(&list)?;
                list
            }
            None => FIXED_FULL_SCOPE
                .iter()
                .map(|item| (*item).to_owned())
                .collect(),
        };

        Ok(Self {
            version: 2,
            project: ProjectSection { name },
            adapters: vec![AdapterConfig {
                id: "rust-cargo".to_owned(),
                roots: vec![".".to_owned()],
                scan: ScanSection {
                    include,
                    assertion_macros,
                },
                run: RunSection { coverage },
            }],
            doc: DocSection { roots: Vec::new() },
            verify: VerifySection { full_scope },
            gates: Vec::new(),
            approval_roles: BTreeMap::new(),
        })
    }
}

/// Intermediate shape for parsing a version 1 (predecessor) `config.yaml` via
/// `yaml_serde`: every field is `Option` so `from_yaml_v1` can tell "key
/// absent" (apply the documented default) apart from "key present" (use it,
/// after validation) — a distinction a plain default value would erase.
/// Carries `version` even though `from_yaml_v1`'s body never reads it
/// (`ProjectConfig::from_yaml` already dispatched on it): without a field to
/// receive it, `deny_unknown_fields` would reject every valid `version: 1`
/// config for the very key that got it routed here.
#[derive(Deserialize, Default)]
#[serde(default, deny_unknown_fields)]
struct V1Config {
    version: Option<u64>,
    project: Option<V1Project>,
    scan: Option<V1Scan>,
    verify: Option<V1Verify>,
    run: Option<V1Run>,
}

#[derive(Deserialize, Default)]
#[serde(default, deny_unknown_fields)]
struct V1Project {
    name: Option<String>,
}

#[derive(Deserialize, Default)]
#[serde(default, deny_unknown_fields)]
struct V1Scan {
    include: Option<Vec<String>>,
    assertion_macros: Option<Vec<String>>,
}

#[derive(Deserialize, Default)]
#[serde(default, deny_unknown_fields)]
struct V1Verify {
    full_scope: Option<Vec<String>>,
}

#[derive(Deserialize, Default)]
#[serde(default, deny_unknown_fields)]
struct V1Run {
    coverage: Option<String>,
}

fn validate_full_scope(full_scope: &[String]) -> Result<(), StoreError> {
    let mut seen = std::collections::BTreeSet::new();
    for item in full_scope {
        if !FIXED_FULL_SCOPE.contains(&item.as_str()) {
            return Err(StoreError::InvalidConfig(format!(
                "verify.full_scope contains an unrecognized check `{item}`"
            )));
        }
        if !seen.insert(item.as_str()) {
            return Err(StoreError::InvalidConfig(format!(
                "verify.full_scope duplicates `{item}`"
            )));
        }
    }
    if full_scope.len() != FIXED_FULL_SCOPE.len() {
        return Err(StoreError::InvalidConfig(
            "verify.full_scope must enumerate exactly the four fixed checks".to_owned(),
        ));
    }
    Ok(())
}

/// Structural (not cross-referential) validation of a version 2 config.
/// Checks that only need the config text itself: adapter id/root duplicates,
/// `verify.full_scope`, gate name duplicates, `require.verification`
/// vocabulary, and unresolved `require.approvals` roles. Whether a `doc.roots`
/// entry names a document that actually exists needs the registered document
/// set, which this parser does not have; that check belongs to whichever
/// component evaluates `orphan_detection` (out of PR2's scope).
fn validate_v2_config(config: &ProjectConfig) -> Result<(), StoreError> {
    let mut seen_adapter_ids = std::collections::BTreeSet::new();
    for adapter in &config.adapters {
        if adapter.id.trim().is_empty() {
            return Err(StoreError::InvalidConfig(
                "adapter id must not be empty".to_owned(),
            ));
        }
        if !seen_adapter_ids.insert(adapter.id.as_str()) {
            return Err(StoreError::InvalidConfig(format!(
                "duplicate adapter id `{}`",
                adapter.id
            )));
        }
        let mut seen_roots = std::collections::BTreeSet::new();
        for root in &adapter.roots {
            if !seen_roots.insert(root.as_str()) {
                return Err(StoreError::InvalidConfig(format!(
                    "adapter `{}` duplicates root `{root}`",
                    adapter.id
                )));
            }
        }
    }

    validate_full_scope(&config.verify.full_scope)?;

    let mut seen_gate_names = std::collections::BTreeSet::new();
    for gate in &config.gates {
        if gate.name.trim().is_empty() {
            return Err(StoreError::InvalidConfig(
                "gate name must not be empty".to_owned(),
            ));
        }
        if !seen_gate_names.insert(gate.name.as_str()) {
            return Err(StoreError::InvalidConfig(format!(
                "duplicate gate name `{}`",
                gate.name
            )));
        }
        if !VERIFICATION_STATES.contains(&gate.require.verification.as_str()) {
            return Err(StoreError::InvalidConfig(format!(
                "gate `{}` requires an unrecognized verification state `{}`",
                gate.name, gate.require.verification
            )));
        }
        // 詳細設計 v0.1 §2.2: "指定する場合は文字列ロール名の list とし、
        // 空文字列・重複ロール名は E-CONFIG-001 とする" — both conditions
        // are scoped to *this gate's own* `approvals` list, distinct from
        // whether a name resolves in `approval_roles` at all (checked below).
        let mut seen_gate_approval_roles = std::collections::BTreeSet::new();
        for role in &gate.require.approvals {
            if role.trim().is_empty() {
                return Err(StoreError::InvalidConfig(format!(
                    "gate `{}` has an empty approval role name",
                    gate.name
                )));
            }
            if !seen_gate_approval_roles.insert(role.as_str()) {
                return Err(StoreError::InvalidConfig(format!(
                    "gate `{}` duplicates approval role `{role}`",
                    gate.name
                )));
            }
            if !config.approval_roles.contains_key(role) {
                return Err(StoreError::InvalidConfig(format!(
                    "gate `{}` requires approval role `{role}`, which approval_roles does not define",
                    gate.name
                )));
            }
        }
    }

    Ok(())
}

fn is_rust_macro_path(value: &str) -> bool {
    !value.is_empty() && value.split("::").all(is_rust_identifier)
}

fn is_rust_identifier(segment: &str) -> bool {
    let identifier = segment.strip_prefix("r#").unwrap_or(segment);
    let mut chars = identifier.chars();
    matches!(chars.next(), Some(ch) if ch == '_' || ch.is_ascii_alphabetic())
        && chars.all(|ch| ch == '_' || ch.is_ascii_alphanumeric())
}

pub fn init_project(root: &Path, name: &str) -> Result<VerifyLayout, StoreError> {
    let layout = VerifyLayout::new(root);
    if layout.verify_dir().exists() {
        return Err(StoreError::AlreadyInitialized(layout.verify_dir()));
    }
    for directory in [
        layout.doc_dir(),
        layout.vo_dir(),
        layout.relation_dir(),
        layout.forms_dir(),
        layout.decisions_dir(),
        layout.approvals_dir(),
        layout.evidence_dir(),
        layout.cache_dir().join("bundles"),
        layout.cache_dir().join("logs"),
        layout.cache_dir().join("cov"),
    ] {
        fs::create_dir_all(&directory).map_err(|source| StoreError::Io {
            path: directory,
            source,
        })?;
    }
    let config = ProjectConfig::default_for(name);
    write_new_file(&layout.config(), config.to_yaml().as_bytes())?;
    write_new_file(
        &layout.forms_dir().join("rust-unit-function.yaml"),
        RUST_UNIT_FUNCTION_FORM.as_bytes(),
    )?;
    write_new_file(
        &layout.forms_dir().join("rust-integration.yaml"),
        RUST_INTEGRATION_FORM.as_bytes(),
    )?;
    for directory in [
        layout.doc_dir(),
        layout.vo_dir(),
        layout.relation_dir(),
        layout.decisions_dir(),
        layout.approvals_dir(),
        layout.evidence_dir(),
    ] {
        write_new_file(&directory.join(".gitkeep"), b"")?;
    }
    write_new_file(&layout.verify_dir().join(".gitignore"), b"cache/\n")?;
    Ok(layout)
}

fn write_new_file(path: &Path, bytes: &[u8]) -> Result<(), StoreError> {
    fs::write(path, bytes).map_err(|source| StoreError::Io {
        path: path.to_owned(),
        source,
    })
}

pub fn find_project_root(start: &Path) -> Result<PathBuf, StoreError> {
    let start = if start.is_file() {
        start.parent().unwrap_or(start)
    } else {
        start
    };
    for candidate in start.ancestors() {
        if candidate.join(".verify").is_dir() {
            return Ok(candidate.to_owned());
        }
    }
    Err(StoreError::NotInitialized(start.to_owned()))
}

pub fn load_config(root: &Path) -> Result<ProjectConfig, StoreError> {
    let layout = VerifyLayout::new(root);
    let text = fs::read_to_string(layout.config()).map_err(|source| StoreError::Io {
        path: layout.config(),
        source,
    })?;
    ProjectConfig::from_yaml(
        &text,
        root.file_name()
            .and_then(|v| v.to_str())
            .unwrap_or("project"),
    )
}

/// Returns the file-stem IDs of every `.yaml` record in `directory`, sorted.
/// Generic over the directory: used for both the canonical (`doc/`, `vo/`)
/// and the predecessor (`spec/`, `req/`) record layouts. Full schema
/// validation is a separate concern; this read-side helper never writes
/// derived cache files.
pub fn read_record_ids(directory: &Path) -> Result<Vec<String>, StoreError> {
    let entries = fs::read_dir(directory).map_err(|source| StoreError::Io {
        path: directory.to_owned(),
        source,
    })?;
    let mut ids = Vec::new();
    for entry in entries {
        let entry = entry.map_err(|source| StoreError::Io {
            path: directory.to_owned(),
            source,
        })?;
        let path = entry.path();
        if path.extension().and_then(|v| v.to_str()) != Some("yaml") {
            continue;
        }
        if let Some(stem) = path.file_stem().and_then(|v| v.to_str()) {
            ids.push(stem.to_owned());
        }
    }
    ids.sort();
    Ok(ids)
}

/// IDs of every registered `document` and `VO` record (詳細設計 v0.1 §2.1's
/// `doc/`+`vo/` layout — the predecessor `spec/`+`req/` split collapsed into
/// the single generic `document` type PR1 introduced, so this returns two
/// slots, not the predecessor reader's three). `vtest-scan`, this function's
/// only caller, still expects the retired three-slot `[spec, req, vo]` shape
/// and does not compile against this branch's canonical `ProjectConfig`
/// regardless (18 pre-existing errors, unrelated to this change); updating
/// that caller to the shape below is PR3's job, when scan itself moves onto
/// the canonical model.
pub fn read_entity_ids(root: &Path) -> Result<[Vec<String>; 2], StoreError> {
    let layout = VerifyLayout::new(root);
    Ok([
        read_record_ids(&layout.doc_dir())?,
        read_record_ids(&layout.vo_dir())?,
    ])
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temporary_directory(name: &str) -> PathBuf {
        let root = std::env::temp_dir().join(format!("vtest-store-lib-{name}-{}", new_record_id()));
        fs::create_dir_all(&root).unwrap();
        root
    }

    #[test]
    fn init_project_creates_the_canonical_v01_layout() {
        let root = temporary_directory("init");
        let layout = init_project(&root, "example").unwrap();

        for directory in [
            layout.doc_dir(),
            layout.vo_dir(),
            layout.relation_dir(),
            layout.forms_dir(),
            layout.decisions_dir(),
            layout.approvals_dir(),
            layout.evidence_dir(),
            layout.cache_dir().join("bundles"),
            layout.cache_dir().join("logs"),
            layout.cache_dir().join("cov"),
        ] {
            assert!(directory.is_dir(), "expected {directory:?} to exist");
        }

        // 詳細設計 v0.1 §2.1 replaces spec/+req/ with doc/, and drops the
        // canonical audits/ directory entirely.
        for removed in [layout.spec_dir(), layout.req_dir(), layout.audits_dir()] {
            assert!(
                !removed.exists(),
                "expected {removed:?} not to be created by init_project"
            );
        }
    }

    #[test]
    fn read_entity_ids_succeeds_against_a_freshly_initialized_canonical_project() {
        let root = temporary_directory("read-entity-ids");
        init_project(&root, "example").unwrap();
        assert_eq!(
            read_entity_ids(&root).unwrap(),
            [Vec::<String>::new(), Vec::new()]
        );
    }

    #[test]
    fn read_entity_ids_reflects_registered_documents_and_vos() {
        let root = temporary_directory("read-entity-ids-populated");
        let layout = init_project(&root, "example").unwrap();
        write_document(
            &layout,
            &vtest_model::DocumentRecord {
                id: vtest_model::DocumentId::new("DOC-A"),
                path: "docs/a.md".to_owned(),
                content_hash: vtest_model::ContentHash::from_text("a"),
                title: None,
                derives_from: vec![],
                registered_at: "2026-08-08T00:00:00Z".to_owned(),
            },
        )
        .unwrap();
        write_vo_record(
            &layout,
            &vtest_model::VoRecord {
                id: vtest_model::VoId::new("VO-A"),
                parent: None,
                derives_from: vec![vtest_model::DerivesFrom {
                    doc: vtest_model::DocumentId::new("DOC-A"),
                    anchor: None,
                    note: None,
                }],
                claim: "claim".to_owned(),
                dimensions: vec![],
                coverage_policy: None,
                combinations: vec![],
                representative_cases: vec![],
                created: "2026-08-08".to_owned(),
                updated: "2026-08-08".to_owned(),
            },
        )
        .unwrap();

        assert_eq!(
            read_entity_ids(&root).unwrap(),
            [vec!["DOC-A".to_owned()], vec!["VO-A".to_owned()]]
        );
    }

    #[test]
    fn default_config_round_trips_through_canonical_v2_yaml() {
        let expected = ProjectConfig::default_for("calc");
        let parsed = ProjectConfig::from_yaml(&expected.to_yaml(), "fallback").unwrap();
        assert_eq!(parsed, expected);
        assert_eq!(parsed.version, 2);
        assert_eq!(parsed.adapters[0].id, "rust-cargo");
        assert!(parsed.adapters[0].scan.include.contains(&"src".to_owned()));
        assert_eq!(parsed.adapters[0].run.coverage, "llvm-cov");
    }

    /// 詳細設計 v0.1 §2.2's own literal `config.yaml` example, verified
    /// field-for-field rather than derived from the writer.
    #[test]
    fn documented_v2_example_parses_as_specified() {
        let yaml = concat!(
            "version: 2\n",
            "project:\n",
            "  name: example\n",
            "adapters:\n",
            "  - id: rust-cargo\n",
            "    roots: [\".\"]\n",
            "    scan:\n",
            "      include: [src, tests, crates]\n",
            "      assertion_macros: []\n",
            "    run:\n",
            "      coverage: llvm-cov\n",
            "doc:\n",
            "  roots: [DOC-REQ-ROOT]\n",
            "verify:\n",
            "  full_scope: [chain_integrity, orphan_detection, target_binding, oracle_presence]\n",
            "gates:\n",
            "  - name: development\n",
            "    require: { verification: PASS }\n",
            "  - name: release\n",
            "    require: { verification: PASS, approvals: [reviewer] }\n",
            "  - name: delivery\n",
            "    require: { verification: PASS, approvals: [owner] }\n",
            "approval_roles:\n",
            "  reviewer: [reviewer-agent-01]\n",
            "  owner: [owner-human-01]\n",
        );

        let config = ProjectConfig::from_yaml(yaml, "fallback").unwrap();

        assert_eq!(config.project.name, "example");
        assert_eq!(config.adapters.len(), 1);
        assert_eq!(config.adapters[0].id, "rust-cargo");
        assert_eq!(config.adapters[0].roots, vec!["."]);
        assert_eq!(
            config.adapters[0].scan.include,
            vec!["src", "tests", "crates"]
        );
        assert!(config.adapters[0].scan.assertion_macros.is_empty());
        assert_eq!(config.adapters[0].run.coverage, "llvm-cov");
        assert_eq!(
            config.doc.roots,
            vec![vtest_model::DocumentId::new("DOC-REQ-ROOT")]
        );
        assert_eq!(
            config.verify.full_scope,
            vec![
                "chain_integrity",
                "orphan_detection",
                "target_binding",
                "oracle_presence"
            ]
        );
        assert_eq!(config.gates.len(), 3);
        assert_eq!(config.gates[0].name, "development");
        assert_eq!(config.gates[0].require.verification, "PASS");
        assert!(config.gates[0].require.approvals.is_empty());
        assert_eq!(config.gates[1].require.approvals, vec!["reviewer"]);
        assert_eq!(config.gates[2].require.approvals, vec!["owner"]);
        assert_eq!(
            config.approval_roles.get("reviewer"),
            Some(&vec!["reviewer-agent-01".to_owned()])
        );

        // The example round-trips through the writer too.
        let rewritten = ProjectConfig::from_yaml(&config.to_yaml(), "fallback").unwrap();
        assert_eq!(rewritten, config);
    }

    #[test]
    fn version_1_config_upconverts_to_a_single_rust_cargo_adapter() {
        let parsed = ProjectConfig::from_yaml(
            "version: 1\nproject:\n  name: x\nscan:\n  include:\n    - examples\n",
            "fallback",
        )
        .unwrap();
        assert_eq!(parsed.version, 2);
        assert_eq!(parsed.adapters.len(), 1);
        assert_eq!(parsed.adapters[0].id, "rust-cargo");
        assert_eq!(parsed.adapters[0].roots, vec!["."]);
        assert_eq!(parsed.adapters[0].scan.include, vec!["examples"]);
        assert!(parsed.doc.roots.is_empty());
        assert!(parsed.gates.is_empty());
    }

    /// 別紙C §18.3.12: the config reader accepts exactly version 1 and
    /// version 2 — it never states or implies a third "no declared version"
    /// case, and no writer in this codebase (nor its predecessor) has ever
    /// emitted a config without a `version` key. Guessing "1" for an absent
    /// version would be exactly the kind of silent-promotion this system's
    /// fail-closed design forbids elsewhere.
    #[test]
    fn unversioned_config_is_rejected() {
        let error = ProjectConfig::from_yaml(
            "scan:\n  include: [\"examples,with-comma\", tests]\n  assertion_macros: []\n",
            "fallback",
        )
        .expect_err("a config with no `version` key must fail closed");
        assert!(error.to_string().contains("version"));
    }

    /// 詳細設計 v0.1 §17.1 lists `config version` among the E-CONFIG-001
    /// conditions: an unrecognized version must fail closed, not be guessed
    /// at as whichever schema is "closest".
    #[test]
    fn unrecognized_config_version_is_rejected() {
        for text in [
            "version: 3\nproject:\n  name: x\n",
            "version: 0\nproject:\n  name: x\n",
            "version: not-a-number\n",
        ] {
            let error = ProjectConfig::from_yaml(text, "fallback")
                .expect_err("an unrecognized config version must fail closed");
            assert!(error.to_string().contains("version"));
        }
    }

    /// `version` is read through the real YAML model, not a hand-rolled
    /// text scan — this must therefore judge these three shapes purely on
    /// YAML type, not on incidental text layout the old line-scanner was
    /// sensitive to.
    #[test]
    fn non_integer_config_version_is_rejected() {
        for text in [
            "version: \"2\"\nproject:\n  name: x\n",
            "version: 2.0\nproject:\n  name: x\n",
            "version: [2]\nproject:\n  name: x\n",
        ] {
            let error = ProjectConfig::from_yaml(text, "fallback")
                .expect_err("a non-integer config version must fail closed");
            assert!(error.to_string().contains("version"));
        }
    }

    /// A trailing inline comment on the `version:` line is ordinary YAML,
    /// not a malformed version — the old line-scanning `detect_config_
    /// version` misread `2  # canonical` as the unparseable literal
    /// `2  # canonical` and rejected it; reading through the YAML model
    /// does not have that failure mode.
    #[test]
    fn config_version_with_a_trailing_comment_is_accepted() {
        let yaml = ProjectConfig::default_for("calc").to_yaml().replacen(
            "version: 2",
            "version: 2  # canonical",
            1,
        );
        let parsed = ProjectConfig::from_yaml(&yaml, "fallback").unwrap();
        assert_eq!(parsed.version, 2);
    }

    /// A prior version of this reader detected `version` by scanning text
    /// line-by-line and stopping at the first match, so a duplicate
    /// top-level `version:` key went unnoticed. Reading through the YAML
    /// model instead means `yaml_serde` itself rejects the duplicate key
    /// during parsing, before `ProjectConfig::from_yaml` ever inspects a
    /// version value.
    #[test]
    fn duplicate_top_level_version_key_is_rejected() {
        ProjectConfig::from_yaml("version: 1\nversion: 2\n", "fallback")
            .expect_err("a duplicate top-level `version` key must fail closed");
    }

    /// The same `yaml_serde::Value`-first parse that lets `from_yaml` inspect
    /// `version` before dispatching also rejects a duplicate key *anywhere*
    /// in the document, not only at the top level — round 2's PR description
    /// stated this was "genuinely unenforced" for `approval_roles`, verified
    /// against round 2's code, which parsed straight into `ProjectConfig`/
    /// `V1Config` via `yaml_serde::from_str` (a plain `BTreeMap`'s own
    /// `Deserialize` silently keeps the last of two duplicate keys — no
    /// rejection). Once every config goes through a `Value` first, its
    /// `Mapping` visitor's own duplicate-key check runs on every nested
    /// mapping, `approval_roles` included, before `from_value` ever builds
    /// the `BTreeMap`. This re-verifies that specifically, rather than
    /// leaving the round 2 claim uncorrected.
    #[test]
    fn approval_roles_with_a_duplicate_key_is_rejected() {
        let yaml = "version: 2\nproject:\n  name: x\nadapters: []\ndoc:\n  roots: []\nverify:\n  full_scope: [chain_integrity, orphan_detection, target_binding, oracle_presence]\napproval_roles:\n  reviewer: [a]\n  reviewer: [b]\n";
        ProjectConfig::from_yaml(yaml, "fallback")
            .expect_err("a duplicate key inside approval_roles must fail closed");
    }

    #[test]
    fn v2_config_missing_a_required_section_is_rejected() {
        let full = ProjectConfig::default_for("calc").to_yaml();
        let lines: Vec<&str> = full.lines().collect();
        for section in ["project", "adapters", "doc", "verify"] {
            // Drops the section's header line *and* its indented body, so
            // the result is valid YAML that genuinely lacks the section
            // (not a header-only removal, which can fold an orphaned body
            // line into a sibling scalar instead of representing "absent").
            let mut without_section = Vec::new();
            let mut skipping = false;
            for line in &lines {
                let is_header = !line.starts_with(' ')
                    && (*line == format!("{section}:")
                        || line.starts_with(&format!("{section}: ")));
                if is_header {
                    skipping = true;
                    continue;
                }
                if skipping {
                    // yaml_serde writes a top-level key's block sequence at
                    // the *same* indentation as the key itself (`adapters:`
                    // then `- id: ...` with no extra indent), so a
                    // continuation line is either indented or a bare `-`.
                    if line.starts_with(' ') || line.starts_with('-') {
                        continue;
                    }
                    skipping = false;
                }
                without_section.push(*line);
            }
            let without_section = without_section.join("\n");
            let error = ProjectConfig::from_yaml(&without_section, "fallback").expect_err(
                &format!("a v2 config missing the `{section}` section must fail closed"),
            );
            assert!(error.to_string().contains(section));
        }
    }

    #[test]
    fn v2_config_with_explicitly_empty_adapters_parses_to_no_adapters() {
        let yaml = "version: 2\nproject:\n  name: x\nadapters: []\ndoc:\n  roots: []\nverify:\n  full_scope: [chain_integrity, orphan_detection, target_binding, oracle_presence]\n";
        let parsed = ProjectConfig::from_yaml(yaml, "fallback").unwrap();
        assert!(
            parsed.adapters.is_empty(),
            "an explicitly empty adapters: [] must not be silently backfilled with the default adapter"
        );
    }

    /// 詳細設計 v0.1 §2.2: the old 12-item full_scope enumeration violates
    /// the current fixed-4-checks invariant regardless of config version.
    #[test]
    fn predecessor_twelve_item_full_scope_is_rejected() {
        let error = ProjectConfig::from_yaml(
            "version: 1\nverify:\n  full_scope:\n    - spec_coverage\n    - vo_decomposition\n",
            "fallback",
        )
        .expect_err("the predecessor full_scope vocabulary must fail closed");
        assert!(error.to_string().contains("full_scope"));
    }

    /// 詳細設計 v0.1 §2.2: "version 1 では...重複または未知項目は
    /// E-CONFIG-001 で拒否する...in-memory 補完で受理しない". A prior
    /// version of this reader ran `Vec::dedup()` (adjacent-only) on a
    /// present v1 `full_scope` before validating it, which silently erased
    /// exactly this shape of duplicate instead of rejecting it.
    #[test]
    fn v1_full_scope_with_an_adjacent_duplicate_is_rejected() {
        let error = ProjectConfig::from_yaml(
            "version: 1\nverify:\n  full_scope:\n    - chain_integrity\n    - chain_integrity\n    - orphan_detection\n    - target_binding\n    - oracle_presence\n",
            "fallback",
        )
        .expect_err("an adjacent duplicate in v1 full_scope must fail closed, not be silently deduped");
        assert!(error.to_string().contains("full_scope"));
    }

    #[test]
    fn invalid_assertion_macro_path_is_rejected() {
        let error = ProjectConfig::from_yaml(
            "version: 1\nscan:\n  assertion_macros:\n    - assert-valid\n",
            "fallback",
        )
        .expect_err("macro names must be Rust identifiers or Rust paths");
        assert!(error.to_string().contains("assertion_macros"));
    }

    #[test]
    fn unsupported_coverage_mode_is_rejected() {
        let error = ProjectConfig::from_yaml("version: 1\nrun:\n  coverage: guessed\n", "fallback")
            .expect_err("unknown coverage mode must fail closed");
        assert!(error.to_string().contains("run.coverage"));
    }

    /// 詳細設計 v0.1 §17.1's E-CONFIG-001 covers a declared `version` whose
    /// body does not match it — the fix here is a rewritten implementation
    /// (`#[serde(deny_unknown_fields)]`), not a version-conditioned branch:
    /// a `version: 1` config carrying the v2-only `gates:` key is simply an
    /// invalid version-1 config, independent of any compatibility concern.
    #[test]
    fn v1_config_with_a_v2_only_key_is_rejected() {
        let yaml =
            "version: 1\ngates:\n  - name: release\n    require:\n      verification: PASS\n";
        let error = ProjectConfig::from_yaml(yaml, "fallback")
            .expect_err("a stray v2-shaped `gates` key must fail closed under version 1");
        assert!(error.to_string().contains("gates"));
    }

    #[test]
    fn v2_config_with_a_v1_only_top_level_key_is_rejected() {
        let mut yaml = ProjectConfig::default_for("calc").to_yaml();
        yaml.push_str("scan:\n  include: [src]\n");
        let error = ProjectConfig::from_yaml(&yaml, "fallback")
            .expect_err("a stray v1-shaped top-level `scan` key must fail closed under version 2");
        assert!(error.to_string().contains("scan"));
    }

    #[test]
    fn v2_config_with_a_misspelled_top_level_key_is_rejected() {
        let yaml = "version: 2\nproject:\n  name: x\nadapters: []\ndoc:\n  roots: []\nverify:\n  full_scope: [chain_integrity, orphan_detection, target_binding, oracle_presence]\ngate: []\n";
        let error = ProjectConfig::from_yaml(yaml, "fallback")
            .expect_err("an unrecognized top-level config key must fail closed");
        assert!(error.to_string().contains("gate"));
    }

    #[test]
    fn v2_config_with_an_unknown_nested_project_key_is_rejected() {
        let yaml = "version: 2\nproject: {name: x, foo: y}\nadapters: []\ndoc:\n  roots: []\nverify:\n  full_scope: [chain_integrity, orphan_detection, target_binding, oracle_presence]\n";
        let error = ProjectConfig::from_yaml(yaml, "fallback")
            .expect_err("an unknown key nested inside `project` must fail closed");
        assert!(error.to_string().contains("foo"));
    }

    #[test]
    fn v2_config_with_a_misspelled_nested_gate_requirement_key_is_rejected() {
        let yaml = "version: 2\nproject:\n  name: x\nadapters: []\ndoc:\n  roots: []\nverify:\n  full_scope: [chain_integrity, orphan_detection, target_binding, oracle_presence]\ngates:\n  - name: release\n    require:\n      verification: PASS\n      approval: []\n";
        let error = ProjectConfig::from_yaml(yaml, "fallback")
            .expect_err("an unknown key nested inside `gates[].require` must fail closed");
        assert!(error.to_string().contains("approval"));
    }

    #[test]
    fn v1_config_with_an_unknown_nested_scan_key_is_rejected() {
        let yaml = "version: 1\nscan:\n  include: [src]\n  foo: 1\n";
        let error = ProjectConfig::from_yaml(yaml, "fallback")
            .expect_err("an unknown key nested inside v1 `scan` must fail closed");
        assert!(error.to_string().contains("foo"));
    }

    #[test]
    fn duplicate_adapter_id_is_rejected() {
        let mut config = ProjectConfig::default_for("calc");
        config.adapters.push(config.adapters[0].clone());
        let error = ProjectConfig::from_yaml(&config.to_yaml(), "fallback")
            .expect_err("duplicate adapter ids must fail closed");
        assert!(error.to_string().contains("adapter id"));
    }

    #[test]
    fn gate_with_unresolved_approval_role_is_rejected() {
        let mut config = ProjectConfig::default_for("calc");
        config.gates.push(GateConfig {
            name: "release".to_owned(),
            require: GateRequirement {
                verification: "PASS".to_owned(),
                approvals: vec!["reviewer".to_owned()],
            },
        });
        let error = ProjectConfig::from_yaml(&config.to_yaml(), "fallback")
            .expect_err("a gate referencing an undefined approval role must fail closed");
        assert!(error.to_string().contains("approval role"));
    }

    /// 詳細設計 v0.1 §2.2: "指定する場合は文字列ロール名の list とし、
    /// 空文字列・重複ロール名は E-CONFIG-001 とする".
    #[test]
    fn gate_with_duplicate_approval_role_is_rejected() {
        let mut config = ProjectConfig::default_for("calc");
        config
            .approval_roles
            .insert("reviewer".to_owned(), vec!["reviewer-agent-01".to_owned()]);
        config.gates.push(GateConfig {
            name: "release".to_owned(),
            require: GateRequirement {
                verification: "PASS".to_owned(),
                approvals: vec!["reviewer".to_owned(), "reviewer".to_owned()],
            },
        });
        let error = ProjectConfig::from_yaml(&config.to_yaml(), "fallback")
            .expect_err("a gate listing the same approval role twice must fail closed");
        assert!(error.to_string().contains("duplicates approval role"));
    }

    #[test]
    fn gate_with_empty_approval_role_name_is_rejected() {
        let mut config = ProjectConfig::default_for("calc");
        config.gates.push(GateConfig {
            name: "release".to_owned(),
            require: GateRequirement {
                verification: "PASS".to_owned(),
                approvals: vec![String::new()],
            },
        });
        let error = ProjectConfig::from_yaml(&config.to_yaml(), "fallback")
            .expect_err("a gate with an empty approval role name must fail closed");
        assert!(error.to_string().contains("empty approval role name"));
    }

    #[test]
    fn gate_with_unrecognized_verification_state_is_rejected() {
        let mut config = ProjectConfig::default_for("calc");
        config.gates.push(GateConfig {
            name: "release".to_owned(),
            require: GateRequirement {
                verification: "OK".to_owned(),
                approvals: vec![],
            },
        });
        let error = ProjectConfig::from_yaml(&config.to_yaml(), "fallback")
            .expect_err("require.verification must be one of the five documented states");
        assert!(error.to_string().contains("verification state"));
    }
}
