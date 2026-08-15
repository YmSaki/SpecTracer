//! Canonical `.verify/` layout and read-side project configuration.
//!
//! Derived indexes are deliberately absent from this crate: callers rebuild
//! them from the canonical records on every operation.

use serde::{Deserialize, Serialize};
use std::{
    fs, io,
    path::{Path, PathBuf},
};
use thiserror::Error;

pub mod approval;
pub mod forms;
pub mod records;
pub use approval::*;
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
    #[error("invalid adapter configuration: {0}")]
    InvalidAdapter(String),
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

    pub fn vo_dir(&self) -> PathBuf {
        self.verify_dir().join("vo")
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

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ProjectConfig {
    pub version: u32,
    pub project: ProjectSection,
    pub adapters: Vec<AdapterSection>,
    pub verify: VerifySection,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ProjectSection {
    pub name: String,
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct ScanSection {
    #[serde(default)]
    pub include: Vec<String>,
    #[serde(default)]
    pub assertion_macros: Vec<String>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct AdapterSection {
    pub id: String,
    #[serde(default)]
    pub roots: Vec<String>,
    #[serde(default)]
    pub scan: ScanSection,
    #[serde(default = "default_run")]
    pub run: RunSection,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct VerifySection {
    pub full_scope: Vec<String>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct RunSection {
    #[serde(default = "default_coverage")]
    pub coverage: String,
}

impl Default for RunSection {
    fn default() -> Self {
        default_run()
    }
}

impl ProjectConfig {
    pub fn default_for(name: impl Into<String>) -> Self {
        Self {
            version: 2,
            project: ProjectSection { name: name.into() },
            adapters: vec![AdapterSection {
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
            verify: VerifySection {
                full_scope: fixed_full_scope(),
            },
        }
    }

    pub fn adapter(&self, id: &str) -> Option<&AdapterSection> {
        self.adapters.iter().find(|adapter| adapter.id == id)
    }

    pub fn rust_cargo(&self) -> &AdapterSection {
        self.adapter("rust-cargo")
            .expect("validated v0.1 config always contains rust-cargo")
    }

    pub fn rust_cargo_mut(&mut self) -> &mut AdapterSection {
        self.adapters
            .iter_mut()
            .find(|adapter| adapter.id == "rust-cargo")
            .expect("validated v0.1 config always contains rust-cargo")
    }

    /// Serialize the project configuration in a deterministic YAML subset.
    pub fn to_yaml(&self) -> String {
        let mut out = format!(
            "version: {}\nproject:\n  name: {}\n",
            self.version,
            yaml_scalar(&self.project.name)
        );
        if self.version == 1 {
            let adapter = self.rust_cargo();
            out.push_str("scan:\n  include:\n");
            write_yaml_list(&mut out, 4, &adapter.scan.include);
            write_named_yaml_list(
                &mut out,
                2,
                "assertion_macros",
                &adapter.scan.assertion_macros,
            );
            out.push_str("verify:\n  full_scope:\n");
            write_yaml_list(&mut out, 4, &self.verify.full_scope);
            out.push_str(&format!(
                "run:\n  coverage: {}\n",
                yaml_scalar(&adapter.run.coverage)
            ));
            return out;
        }
        out.push_str("adapters:\n");
        for adapter in &self.adapters {
            out.push_str(&format!("  - id: {}\n", yaml_scalar(&adapter.id)));
            out.push_str("    roots:\n");
            write_yaml_list(&mut out, 6, &adapter.roots);
            out.push_str("    scan:\n      include:\n");
            write_yaml_list(&mut out, 8, &adapter.scan.include);
            write_named_yaml_list(
                &mut out,
                6,
                "assertion_macros",
                &adapter.scan.assertion_macros,
            );
            out.push_str(&format!(
                "    run:\n      coverage: {}\n",
                yaml_scalar(&adapter.run.coverage)
            ));
        }
        out.push_str("verify:\n  full_scope:\n");
        write_yaml_list(&mut out, 4, &self.verify.full_scope);
        out
    }

    /// Read the documented configuration subset. Unknown fields fall back to
    /// the documented defaults; malformed values in supported fields fail
    /// closed rather than silently changing scan behaviour.
    pub fn from_yaml(text: &str, project_name: impl Into<String>) -> Result<Self, StoreError> {
        let wire: WireProjectConfig = serde_yaml::from_str(text)
            .map_err(|error| StoreError::InvalidConfig(error.to_string()))?;
        let version = wire.version.unwrap_or(1);
        if !matches!(version, 1 | 2) {
            return Err(StoreError::InvalidConfig(format!(
                "unsupported config version {version}"
            )));
        }
        let project = wire.project.unwrap_or(ProjectSection {
            name: project_name.into(),
        });
        let verify = normalize_full_scope(version, wire.verify)?;
        let adapters = if version == 1 {
            if wire.adapters.is_some() {
                return Err(StoreError::InvalidConfig(
                    "version 1 config must not contain adapters".to_owned(),
                ));
            }
            vec![AdapterSection {
                id: "rust-cargo".to_owned(),
                roots: vec![".".to_owned()],
                scan: wire.scan.unwrap_or_else(default_scan),
                run: wire.run.unwrap_or_else(default_run),
            }]
        } else {
            if wire.scan.is_some() || wire.run.is_some() {
                return Err(StoreError::InvalidConfig(
                    "version 2 config requires scan/run under an adapter namespace".to_owned(),
                ));
            }
            wire.adapters.ok_or_else(|| {
                StoreError::InvalidAdapter("version 2 config is missing adapters".to_owned())
            })?
        };
        let config = Self {
            version,
            project,
            adapters,
            verify,
        };
        validate_adapters(&config.adapters)?;
        Ok(config)
    }
}

#[derive(Deserialize)]
struct WireProjectConfig {
    version: Option<u32>,
    project: Option<ProjectSection>,
    scan: Option<ScanSection>,
    verify: Option<VerifySection>,
    run: Option<RunSection>,
    adapters: Option<Vec<AdapterSection>>,
}

const FIXED_FULL_SCOPE: [&str; 12] = [
    "spec_coverage",
    "vo_decomposition",
    "vo_coverage",
    "test_existence",
    "static_audit",
    "semantic_audit",
    "impl_consistency",
    "test_execution",
    "runtime_result",
    "target_execution",
    "evidence_validity",
    "test_traceability",
];

fn fixed_full_scope() -> Vec<String> {
    FIXED_FULL_SCOPE
        .iter()
        .map(|value| (*value).to_owned())
        .collect()
}

fn normalize_full_scope(
    version: u32,
    verify: Option<VerifySection>,
) -> Result<VerifySection, StoreError> {
    let supplied = verify.map(|verify| verify.full_scope);
    if version == 1 && supplied.as_ref().is_none_or(Vec::is_empty) {
        return Ok(VerifySection {
            full_scope: fixed_full_scope(),
        });
    }
    let scope = supplied.unwrap_or_default();
    let unique = scope.iter().collect::<std::collections::BTreeSet<_>>();
    let legacy = &FIXED_FULL_SCOPE[..11];
    if version == 1 && scope.iter().map(String::as_str).eq(legacy.iter().copied()) {
        return Ok(VerifySection {
            full_scope: fixed_full_scope(),
        });
    }
    if scope.len() != FIXED_FULL_SCOPE.len()
        || unique.len() != scope.len()
        || !scope
            .iter()
            .map(String::as_str)
            .eq(FIXED_FULL_SCOPE.iter().copied())
    {
        return Err(StoreError::InvalidConfig(
            "verify.full_scope must contain the exact fixed 12 items once and in canonical order"
                .to_owned(),
        ));
    }
    Ok(VerifySection { full_scope: scope })
}

fn default_scan() -> ScanSection {
    ScanSection {
        include: vec!["src".to_owned(), "tests".to_owned(), "crates".to_owned()],
        assertion_macros: Vec::new(),
    }
}

fn default_run() -> RunSection {
    RunSection {
        coverage: default_coverage(),
    }
}

fn default_coverage() -> String {
    "llvm-cov".to_owned()
}

fn validate_adapters(adapters: &[AdapterSection]) -> Result<(), StoreError> {
    if adapters.is_empty() {
        return Err(StoreError::InvalidAdapter(
            "at least one adapter is required".to_owned(),
        ));
    }
    let mut ids = std::collections::BTreeSet::new();
    for adapter in adapters {
        if adapter.id != "rust-cargo" {
            return Err(StoreError::InvalidAdapter(format!(
                "unknown adapter `{}`",
                adapter.id
            )));
        }
        if !ids.insert(adapter.id.as_str()) {
            return Err(StoreError::InvalidAdapter(format!(
                "duplicate adapter ID `{}`",
                adapter.id
            )));
        }
        if adapter.roots.is_empty() {
            return Err(StoreError::InvalidAdapter(format!(
                "adapter `{}` has no roots",
                adapter.id
            )));
        }
        let mut roots = std::collections::BTreeSet::new();
        for root in &adapter.roots {
            let normalized = normalize_project_relative(root)?;
            if !roots.insert(normalized) {
                return Err(StoreError::InvalidAdapter(format!(
                    "adapter `{}` contains a duplicate root `{root}`",
                    adapter.id
                )));
            }
        }
        for include in &adapter.scan.include {
            normalize_project_relative(include)?;
        }
        for macro_name in &adapter.scan.assertion_macros {
            if !is_rust_macro_path(macro_name) {
                return Err(StoreError::InvalidConfig(format!(
                    "scan.assertion_macros contains invalid Rust macro path `{macro_name}`"
                )));
            }
        }
        if !matches!(adapter.run.coverage.as_str(), "llvm-cov" | "off") {
            return Err(StoreError::InvalidConfig(format!(
                "run.coverage must be `llvm-cov` or `off`, got `{}`",
                adapter.run.coverage
            )));
        }
    }
    Ok(())
}

fn normalize_project_relative(value: &str) -> Result<String, StoreError> {
    let normalized = value.replace('\\', "/").trim_end_matches('/').to_owned();
    if normalized == "." {
        return Ok(normalized);
    }
    let path = Path::new(&normalized);
    if normalized.is_empty()
        || path.is_absolute()
        || path
            .components()
            .any(|component| !matches!(component, std::path::Component::Normal(_)))
    {
        return Err(StoreError::InvalidAdapter(format!(
            "path `{value}` must be project-relative without traversal"
        )));
    }
    Ok(normalized)
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

fn yaml_scalar(value: &str) -> String {
    if value
        .chars()
        .all(|ch| ch.is_ascii_alphanumeric() || "._-/".contains(ch))
    {
        value.to_owned()
    } else {
        format!("'{}'", value.replace('\'', "''"))
    }
}

fn write_yaml_list(output: &mut String, indent: usize, values: &[String]) {
    for value in values {
        output.push_str(&format!("{}- {}\n", " ".repeat(indent), yaml_scalar(value)));
    }
}

fn write_named_yaml_list(output: &mut String, indent: usize, name: &str, values: &[String]) {
    let prefix = " ".repeat(indent);
    if values.is_empty() {
        output.push_str(&format!("{prefix}{name}: []\n"));
    } else {
        output.push_str(&format!("{prefix}{name}:\n"));
        write_yaml_list(output, indent + 2, values);
    }
}

pub fn init_project(root: &Path, name: &str) -> Result<VerifyLayout, StoreError> {
    let layout = VerifyLayout::new(root);
    if layout.verify_dir().exists() {
        return Err(StoreError::AlreadyInitialized(layout.verify_dir()));
    }
    for directory in [
        layout.spec_dir(),
        layout.req_dir(),
        layout.vo_dir(),
        layout.relation_dir(),
        layout.forms_dir(),
        layout.approvals_dir(),
        layout.audits_dir(),
        layout.evidence_dir(),
        layout.cache_dir().join("bundles"),
        layout.cache_dir().join("logs"),
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
        layout.spec_dir(),
        layout.req_dir(),
        layout.vo_dir(),
        layout.relation_dir(),
        layout.approvals_dir(),
        layout.audits_dir(),
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

/// Return IDs from canonical entity records.  Full schema validation is an
/// M2 concern; even this read-side helper never writes derived cache files.
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

pub fn read_entity_ids(root: &Path) -> Result<[Vec<String>; 3], StoreError> {
    let layout = VerifyLayout::new(root);
    Ok([
        read_record_ids(&layout.spec_dir())?,
        read_record_ids(&layout.req_dir())?,
        read_record_ids(&layout.vo_dir())?,
    ])
}

#[cfg(test)]
mod tests {
    use super::*;

    /// @vtest.id TEST-STORE-010
    /// @vtest.covers VO-STORE-005
    /// @vtest.target crates/vtest-store/src/lib.rs::ProjectConfig::from_yaml
    /// @vtest.intent Default config serialized then parsed preserves name, includes, coverage
    #[test]
    fn generated_config_round_trips_m1_fields() {
        let expected = ProjectConfig::default_for("calc");
        let parsed = ProjectConfig::from_yaml(&expected.to_yaml(), "fallback").unwrap();
        assert_eq!(parsed.project.name, "calc");
        assert!(parsed.rust_cargo().scan.include.contains(&"src".to_owned()));
        assert_eq!(parsed.rust_cargo().run.coverage, "llvm-cov");
    }

    /// @vtest.id TEST-STORE-011
    /// @vtest.covers VO-STORE-005
    /// @vtest.target crates/vtest-store/src/lib.rs::ProjectConfig::from_yaml
    /// @vtest.intent Explicit scan.include replaces defaults rather than merging
    #[test]
    fn explicit_include_list_does_not_retain_defaults() {
        let parsed = ProjectConfig::from_yaml(
            "version: 1\nproject:\n  name: x\nscan:\n  include:\n    - examples\n",
            "fallback",
        )
        .unwrap();
        assert_eq!(parsed.rust_cargo().scan.include, vec!["examples"]);
    }

    /// @vtest.id TEST-STORE-012
    /// @vtest.covers VO-STORE-006
    /// @vtest.target crates/vtest-store/src/lib.rs::ProjectConfig::to_yaml
    /// @vtest.intent assertion_macros round-trip and stay separate from scan.include
    #[test]
    fn assertion_macro_block_list_round_trips_without_becoming_scan_includes() {
        let mut expected = ProjectConfig::default_for("calc");
        expected.rust_cargo_mut().scan.include = vec!["tests".to_owned()];
        expected.rust_cargo_mut().scan.assertion_macros = vec![
            "assert_valid".to_owned(),
            "crate::checks::assert_result".to_owned(),
        ];

        let yaml = expected.to_yaml();
        assert!(yaml.contains("assertion_macros:\n        - assert_valid\n"));
        let parsed = ProjectConfig::from_yaml(&yaml, "fallback").unwrap();

        assert_eq!(parsed, expected);
        assert_eq!(parsed.rust_cargo().scan.include, vec!["tests"]);
    }

    /// @vtest.id TEST-STORE-013
    /// @vtest.covers VO-STORE-005
    /// @vtest.target crates/vtest-store/src/lib.rs::ProjectConfig::from_yaml
    /// @vtest.intent Bracketed and empty scan list forms parse as documented
    #[test]
    fn scan_lists_accept_documented_bracketed_and_empty_forms() {
        let parsed = ProjectConfig::from_yaml(
            "scan:\n  include: [\"examples,with-comma\", tests]\n  assertion_macros: []\n",
            "fallback",
        )
        .unwrap();

        assert_eq!(
            parsed.rust_cargo().scan.include,
            vec!["examples,with-comma", "tests"]
        );
        assert!(parsed.rust_cargo().scan.assertion_macros.is_empty());
    }

    /// @vtest.id TEST-STORE-014
    /// @vtest.covers VO-STORE-005
    /// @vtest.target crates/vtest-store/src/lib.rs::ProjectConfig::from_yaml
    /// @vtest.intent Non-identifier assertion macro path fails closed
    #[test]
    fn invalid_assertion_macro_path_is_rejected() {
        let error = ProjectConfig::from_yaml(
            "scan:\n  assertion_macros:\n    - assert-valid\n",
            "fallback",
        )
        .expect_err("macro names must be Rust identifiers or Rust paths");
        assert!(error.to_string().contains("assertion_macros"));
    }

    /// @vtest.id TEST-STORE-015
    /// @vtest.covers VO-STORE-005
    /// @vtest.target crates/vtest-store/src/lib.rs::ProjectConfig::from_yaml
    /// @vtest.intent Unknown run.coverage mode is rejected
    #[test]
    fn unsupported_coverage_mode_is_rejected() {
        let error = ProjectConfig::from_yaml("run:\n  coverage: guessed\n", "fallback")
            .expect_err("unknown coverage mode must fail closed");
        assert!(error.to_string().contains("run.coverage"));
    }

    const CANONICAL_TWELVE: &str = "spec_coverage, vo_decomposition, vo_coverage, test_existence, \
static_audit, semantic_audit, impl_consistency, test_execution, runtime_result, target_execution, \
evidence_validity, test_traceability";

    fn one_adapter(roots: &str) -> String {
        format!(
            "  - id: rust-cargo\n    roots: {roots}\n    scan:\n      include: [src]\n      \
             assertion_macros: []\n    run:\n      coverage: llvm-cov\n"
        )
    }

    fn version_two(adapters: &str, full_scope: &str) -> Result<ProjectConfig, StoreError> {
        ProjectConfig::from_yaml(
            &format!(
                "version: 2\nproject:\n  name: scope\nadapters:\n{adapters}verify:\n  \
                 full_scope: [{full_scope}]\n"
            ),
            "fallback",
        )
    }

    /// @vtest.id TEST-STORE-016
    /// @vtest.covers VO-STORE-005
    /// @vtest.target crates/vtest-store/src/lib.rs::ProjectConfig::from_yaml
    /// @vtest.intent v2 full_scope rejects unknown items and a thirteenth entry
    #[test]
    fn version_two_full_scope_rejects_unknown_and_extra_items() {
        let adapters = one_adapter("[\".\"]");
        assert!(version_two(&adapters, CANONICAL_TWELVE).is_ok());

        let unknown = CANONICAL_TWELVE.replace("runtime_result", "guessed_item");
        let error = version_two(&adapters, &unknown)
            .expect_err("an unknown scope item is not one of the fixed twelve");
        assert!(error.to_string().contains("full_scope"), "{error}");

        let extra = format!("{CANONICAL_TWELVE}, spec_coverage");
        let error =
            version_two(&adapters, &extra).expect_err("a thirteenth entry is not the fixed twelve");
        assert!(error.to_string().contains("full_scope"), "{error}");
    }

    /// @vtest.id TEST-STORE-017
    /// @vtest.covers VO-STORE-005
    /// @vtest.target crates/vtest-store/src/lib.rs::ProjectConfig::from_yaml
    /// @vtest.intent Adapter roots must be present, unique, and project-relative
    #[test]
    fn adapter_roots_must_be_present_unique_and_project_relative() {
        let error = version_two(&one_adapter("[\"crates\", \"crates/\"]"), CANONICAL_TWELVE)
            .expect_err("one root cannot be declared twice under different spellings");
        assert!(error.to_string().contains("duplicate root"), "{error}");

        let error = version_two(&one_adapter("[\"../outside\"]"), CANONICAL_TWELVE)
            .expect_err("a root must stay inside the project");
        assert!(error.to_string().contains("project-relative"), "{error}");

        let error = version_two(&one_adapter("[]"), CANONICAL_TWELVE)
            .expect_err("an adapter with no root cannot be scanned");
        assert!(error.to_string().contains("no roots"), "{error}");
    }
}
