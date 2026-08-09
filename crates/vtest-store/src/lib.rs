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

pub mod forms;
pub mod records;
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

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ProjectConfig {
    pub version: u32,
    pub project: ProjectSection,
    pub scan: ScanSection,
    pub verify: VerifySection,
    pub run: RunSection,
    /// Version 2 adapter namespaces.  The legacy `scan` and `run` fields are
    /// retained as the effective first-adapter view for v0.1 callers.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub adapters: Vec<ConfiguredAdapter>,
}

impl PartialEq for ProjectConfig {
    fn eq(&self, other: &Self) -> bool {
        self.version == other.version
            && self.project == other.project
            && self.scan == other.scan
            && self.verify == other.verify
            && self.run == other.run
            // v1 has no adapter section on the wire; its effective
            // rust-cargo mapping is derived in-memory by the loader.
            && (self.version < 2 || self.adapters == other.adapters)
    }
}

impl Eq for ProjectConfig {}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ProjectSection {
    pub name: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ScanSection {
    pub include: Vec<String>,
    pub assertion_macros: Vec<String>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct VerifySection {
    pub full_scope: Vec<String>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct RunSection {
    pub coverage: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ConfiguredAdapter {
    pub id: String,
    pub roots: Vec<String>,
    pub scan: ScanSection,
    pub run: RunSection,
}

impl ProjectConfig {
    pub fn default_for(name: impl Into<String>) -> Self {
        Self::default_v1_for(name)
    }

    /// Generate the version 2 configuration used by `vtest init`.
    pub fn default_v2_for(name: impl Into<String>) -> Self {
        let scan = ScanSection {
            include: vec!["src".to_owned(), "tests".to_owned(), "crates".to_owned()],
            assertion_macros: Vec::new(),
        };
        let run = RunSection {
            coverage: "llvm-cov".to_owned(),
        };
        Self {
            version: 2,
            project: ProjectSection { name: name.into() },
            scan: scan.clone(),
            verify: VerifySection {
                full_scope: vec![
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
                ]
                .into_iter()
                .map(str::to_owned)
                .collect(),
            },
            run: run.clone(),
            adapters: vec![ConfiguredAdapter {
                id: "rust-cargo".to_owned(),
                roots: vec![".".to_owned()],
                scan,
                run,
            }],
        }
    }

    fn default_v1_for(name: impl Into<String>) -> Self {
        let scan = ScanSection {
            include: vec!["src".to_owned(), "tests".to_owned(), "crates".to_owned()],
            assertion_macros: Vec::new(),
        };
        let run = RunSection {
            coverage: "llvm-cov".to_owned(),
        };
        Self {
            version: 1,
            project: ProjectSection { name: name.into() },
            scan: scan.clone(),
            verify: VerifySection {
                full_scope: [
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
                ]
                .into_iter()
                .map(str::to_owned)
                .collect(),
            },
            run: run.clone(),
            adapters: vec![ConfiguredAdapter {
                id: "rust-cargo".to_owned(),
                roots: vec![".".to_owned()],
                scan,
                run,
            }],
        }
    }

    /// Serialize the project configuration in a deterministic YAML subset.
    pub fn to_yaml(&self) -> String {
        if self.version >= 2 {
            return self.to_yaml_v2();
        }
        let mut out = format!(
            "version: {}\nproject:\n  name: {}\nscan:\n  include:\n",
            self.version,
            yaml_scalar(&self.project.name)
        );
        for include in &self.scan.include {
            out.push_str(&format!("    - {}\n", yaml_scalar(include)));
        }
        if self.scan.assertion_macros.is_empty() {
            out.push_str("  assertion_macros: []\n");
        } else {
            out.push_str("  assertion_macros:\n");
            for macro_name in &self.scan.assertion_macros {
                out.push_str(&format!("    - {}\n", yaml_scalar(macro_name)));
            }
        }
        out.push_str("verify:\n  full_scope:\n");
        for item in &self.verify.full_scope {
            out.push_str(&format!("    - {}\n", yaml_scalar(item)));
        }
        out.push_str(&format!(
            "run:\n  coverage: {}\n",
            yaml_scalar(&self.run.coverage)
        ));
        out
    }

    fn to_yaml_v2(&self) -> String {
        let mut out = format!(
            "version: 2\nproject:\n  name: {}\nadapters:\n",
            yaml_scalar(&self.project.name)
        );
        for adapter in &self.adapters {
            out.push_str(&format!(
                "  - id: {}\n    roots:\n",
                yaml_scalar(&adapter.id)
            ));
            for root in &adapter.roots {
                out.push_str(&format!("      - {}\n", yaml_scalar(root)));
            }
            out.push_str("    scan:\n      include:\n");
            for include in &adapter.scan.include {
                out.push_str(&format!("        - {}\n", yaml_scalar(include)));
            }
            if adapter.scan.assertion_macros.is_empty() {
                out.push_str("      assertion_macros: []\n");
            } else {
                out.push_str("      assertion_macros:\n");
                for macro_name in &adapter.scan.assertion_macros {
                    out.push_str(&format!("        - {}\n", yaml_scalar(macro_name)));
                }
            }
            out.push_str(&format!(
                "    run:\n      coverage: {}\n",
                yaml_scalar(&adapter.run.coverage)
            ));
        }
        out.push_str("verify:\n  full_scope:\n");
        for item in &self.verify.full_scope {
            out.push_str(&format!("    - {}\n", yaml_scalar(item)));
        }
        out
    }

    /// Read the documented configuration subset. Missing fields use the
    /// documented defaults; malformed values in supported fields fail closed
    /// rather than silently changing scan behaviour. Adapter-specific values
    /// are retained for the selected adapter to validate.
    pub fn from_yaml(text: &str, project_name: impl Into<String>) -> Result<Self, StoreError> {
        let fallback_name = project_name.into();
        let mut config = Self::default_v2_for(fallback_name);
        let mut saw_version = false;
        let mut section = String::new();
        let mut list_field = None;
        let mut saw_include = false;
        let mut saw_assertion_macros = false;
        let mut saw_full_scope = false;
        let mut current_adapter: Option<usize> = None;
        let mut adapter_section = String::new();
        let mut saw_adapter_roots = false;
        let mut saw_adapter_include = false;
        let mut saw_adapter_assertions = false;
        for raw in text.lines() {
            let line = raw.trim_end();
            if line.trim().is_empty() || line.trim_start().starts_with('#') {
                continue;
            }
            let indent = line.chars().take_while(|ch| *ch == ' ').count();
            let trimmed = line.trim();
            if indent == 0 && trimmed.ends_with(':') {
                section = trimmed.trim_end_matches(':').to_owned();
                if section == "adapters" {
                    config.adapters.clear();
                }
                current_adapter = None;
                adapter_section.clear();
                list_field = None;
                continue;
            }
            if section == "adapters" && indent == 2 && trimmed.starts_with("- ") {
                let pair = trimmed.trim_start_matches("- ").trim();
                let (key, value) = pair
                    .split_once(':')
                    .ok_or_else(|| StoreError::InvalidConfig(line.to_owned()))?;
                if key.trim() != "id" {
                    return Err(StoreError::InvalidConfig(line.to_owned()));
                }
                let id = unquote_yaml_scalar(value.trim())?;
                config.adapters.push(ConfiguredAdapter {
                    id,
                    roots: vec![".".to_owned()],
                    scan: config.scan.clone(),
                    run: config.run.clone(),
                });
                current_adapter = Some(config.adapters.len() - 1);
                adapter_section.clear();
                list_field = None;
                saw_adapter_roots = false;
                saw_adapter_include = false;
                saw_adapter_assertions = false;
                continue;
            }
            if trimmed.starts_with('-') {
                let value = unquote_yaml_scalar(trimmed.trim_start_matches('-').trim())?;
                if let Some(index) = current_adapter {
                    match list_field {
                        Some(ConfigList::AdapterRoots) => {
                            if !saw_adapter_roots {
                                config.adapters[index].roots.clear();
                                saw_adapter_roots = true;
                            }
                            config.adapters[index].roots.push(value);
                        }
                        Some(ConfigList::AdapterInclude) => {
                            if !saw_adapter_include {
                                config.adapters[index].scan.include.clear();
                                saw_adapter_include = true;
                            }
                            config.adapters[index].scan.include.push(value);
                        }
                        Some(ConfigList::AdapterAssertions) => {
                            if !saw_adapter_assertions {
                                config.adapters[index].scan.assertion_macros.clear();
                                saw_adapter_assertions = true;
                            }
                            config.adapters[index].scan.assertion_macros.push(value);
                        }
                        _ => {}
                    }
                } else {
                    match list_field {
                        Some(ConfigList::ScanInclude) => {
                            if !saw_include {
                                config.scan.include.clear();
                                saw_include = true;
                            }
                            config.scan.include.push(value);
                        }
                        Some(ConfigList::AssertionMacros) => {
                            if !saw_assertion_macros {
                                config.scan.assertion_macros.clear();
                                saw_assertion_macros = true;
                            }
                            config.scan.assertion_macros.push(value);
                        }
                        Some(ConfigList::VerifyFullScope) => {
                            if !saw_full_scope {
                                config.verify.full_scope.clear();
                                saw_full_scope = true;
                            }
                            config.verify.full_scope.push(value);
                        }
                        _ => {}
                    }
                }
                continue;
            }
            let Some((key, value)) = trimmed.split_once(':') else {
                return Err(StoreError::InvalidConfig(line.to_owned()));
            };
            let key = key.trim();
            let value = value.trim();
            list_field = None;
            if let Some(index) = current_adapter {
                match (indent, adapter_section.as_str(), key) {
                    (4, _, "roots") => {
                        parse_list_value(
                            value,
                            &mut config.adapters[index].roots,
                            &mut saw_adapter_roots,
                        )?;
                        if value.is_empty() {
                            list_field = Some(ConfigList::AdapterRoots);
                        }
                    }
                    (4, _, "scan") => adapter_section = "scan".to_owned(),
                    (4, _, "run") => adapter_section = "run".to_owned(),
                    (6, "scan", "include") => {
                        parse_list_value(
                            value,
                            &mut config.adapters[index].scan.include,
                            &mut saw_adapter_include,
                        )?;
                        if value.is_empty() {
                            list_field = Some(ConfigList::AdapterInclude);
                        }
                    }
                    (6, "scan", "assertion_macros") => {
                        parse_list_value(
                            value,
                            &mut config.adapters[index].scan.assertion_macros,
                            &mut saw_adapter_assertions,
                        )?;
                        if value.is_empty() {
                            list_field = Some(ConfigList::AdapterAssertions);
                        }
                    }
                    (6, "run", "coverage") => {
                        config.adapters[index].run.coverage = unquote_yaml_scalar(value)?
                    }
                    _ => {
                        return Err(StoreError::InvalidConfig(format!(
                            "unsupported adapter configuration: {line}"
                        )))
                    }
                }
                continue;
            }
            match (section.as_str(), key) {
                ("project", "name") => config.project.name = unquote_yaml_scalar(value)?,
                ("run", "coverage") => config.run.coverage = unquote_yaml_scalar(value)?,
                ("scan", "include") => {
                    parse_list_value(value, &mut config.scan.include, &mut saw_include)?;
                    if value.is_empty() {
                        list_field = Some(ConfigList::ScanInclude);
                    }
                }
                ("scan", "assertion_macros") => {
                    parse_list_value(
                        value,
                        &mut config.scan.assertion_macros,
                        &mut saw_assertion_macros,
                    )?;
                    if value.is_empty() {
                        list_field = Some(ConfigList::AssertionMacros);
                    }
                }
                ("verify", "full_scope") => {
                    parse_list_value(value, &mut config.verify.full_scope, &mut saw_full_scope)?;
                    if value.is_empty() {
                        list_field = Some(ConfigList::VerifyFullScope);
                    }
                }
                ("", "version") => {
                    config.version = value
                        .parse()
                        .map_err(|_| StoreError::InvalidConfig(line.to_owned()))?;
                    saw_version = true;
                }
                ("adapters", _) => {
                    return Err(StoreError::InvalidConfig(format!(
                        "adapter entry must start with `- id`: {line}"
                    )))
                }
                _ => {}
            }
        }
        if !saw_version {
            config.version = 1;
        }
        if config.version > 2 || config.version == 0 {
            return Err(StoreError::InvalidConfig(format!(
                "unsupported config version {}",
                config.version
            )));
        }
        if config.version == 1 {
            config.adapters = vec![ConfiguredAdapter {
                id: "rust-cargo".to_owned(),
                roots: vec![".".to_owned()],
                scan: config.scan.clone(),
                run: config.run.clone(),
            }];
        } else {
            if config.adapters.is_empty() {
                return Err(StoreError::InvalidConfig(
                    "version 2 requires at least one adapter".to_owned(),
                ));
            }
            config.scan = config.adapters[0].scan.clone();
            config.run = config.adapters[0].run.clone();
        }
        config.scan.include.dedup();
        config.scan.assertion_macros.dedup();
        config.verify.full_scope.dedup();
        let mut ids = std::collections::BTreeSet::new();
        for adapter in &mut config.adapters {
            if !ids.insert(adapter.id.clone()) {
                return Err(StoreError::InvalidConfig(format!(
                    "duplicate adapter id `{}`",
                    adapter.id
                )));
            }
            let root_count = adapter.roots.len();
            adapter.roots.dedup();
            if adapter.roots.len() != root_count {
                return Err(StoreError::InvalidConfig(format!(
                    "duplicate root in adapter `{}`",
                    adapter.id
                )));
            }
            if adapter.roots.is_empty() {
                return Err(StoreError::InvalidConfig(format!(
                    "adapter `{}` must declare a root",
                    adapter.id
                )));
            }
            adapter.scan.include.dedup();
            adapter.scan.assertion_macros.dedup();
            if adapter.id == "rust-cargo" {
                for macro_name in &adapter.scan.assertion_macros {
                    if !is_rust_macro_path(macro_name) {
                        return Err(StoreError::InvalidConfig(format!(
                            "scan.assertion_macros contains invalid Rust macro path `{macro_name}`"
                        )));
                    }
                }
                if !matches!(adapter.run.coverage.as_str(), "llvm-cov" | "off") {
                    return Err(StoreError::InvalidConfig(format!(
                        "adapter `{}` run.coverage must be `llvm-cov` or `off`, got `{}`",
                        adapter.id, adapter.run.coverage
                    )));
                }
            }
        }
        if config.version == 1
            || config
                .adapters
                .first()
                .is_some_and(|adapter| adapter.id == "rust-cargo")
        {
            for macro_name in &config.scan.assertion_macros {
                if !is_rust_macro_path(macro_name) {
                    return Err(StoreError::InvalidConfig(format!(
                        "scan.assertion_macros contains invalid Rust macro path `{macro_name}`"
                    )));
                }
            }
        }
        Ok(config)
    }
}

#[derive(Clone, Copy)]
enum ConfigList {
    ScanInclude,
    AssertionMacros,
    VerifyFullScope,
    AdapterRoots,
    AdapterInclude,
    AdapterAssertions,
}

fn parse_list_value(
    value: &str,
    target: &mut Vec<String>,
    saw_list: &mut bool,
) -> Result<(), StoreError> {
    if !*saw_list {
        target.clear();
        *saw_list = true;
    }
    if value.is_empty() || value == "[]" {
        return Ok(());
    }
    if !value.starts_with('[') || !value.ends_with(']') {
        return Err(StoreError::InvalidConfig(format!(
            "list value must be a block list or bracketed list, got `{value}`"
        )));
    }
    target.extend(parse_inline_list(value)?);
    Ok(())
}

fn parse_inline_list(value: &str) -> Result<Vec<String>, StoreError> {
    let Some(body) = value
        .strip_prefix('[')
        .and_then(|body| body.strip_suffix(']'))
    else {
        return Err(StoreError::InvalidConfig(format!(
            "invalid bracketed YAML list `{value}`"
        )));
    };
    if body.trim().is_empty() {
        return Ok(Vec::new());
    }

    let mut values = Vec::new();
    let mut current = String::new();
    let mut quote = None;
    let mut chars = body.chars().peekable();
    while let Some(ch) = chars.next() {
        match quote {
            Some(delimiter) => {
                current.push(ch);
                if ch == delimiter {
                    if delimiter == '\'' && chars.peek() == Some(&'\'') {
                        current.push(chars.next().expect("peeked quote must exist"));
                    } else {
                        quote = None;
                    }
                }
            }
            None => match ch {
                '\'' | '"' => {
                    quote = Some(ch);
                    current.push(ch);
                }
                ',' => {
                    values.push(unquote_yaml_scalar(current.trim())?);
                    current.clear();
                }
                _ => current.push(ch),
            },
        }
    }
    if quote.is_some() {
        return Err(StoreError::InvalidConfig(format!(
            "unterminated quoted YAML list value `{value}`"
        )));
    }
    values.push(unquote_yaml_scalar(current.trim())?);
    Ok(values)
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

fn unquote_yaml_scalar(value: &str) -> Result<String, StoreError> {
    if value.is_empty() {
        Err(StoreError::InvalidConfig("empty YAML scalar".to_owned()))
    } else if value.starts_with('\'') || value.ends_with('\'') {
        if value.starts_with('\'') && value.ends_with('\'') && value.len() >= 2 {
            Ok(value[1..value.len() - 1].replace("''", "'"))
        } else {
            Err(StoreError::InvalidConfig(format!(
                "unterminated single-quoted YAML scalar `{value}`"
            )))
        }
    } else if value.starts_with('"') || value.ends_with('"') {
        if value.starts_with('"') && value.ends_with('"') && value.len() >= 2 {
            Ok(value[1..value.len() - 1].to_owned())
        } else {
            Err(StoreError::InvalidConfig(format!(
                "unterminated double-quoted YAML scalar `{value}`"
            )))
        }
    } else {
        Ok(value.to_owned())
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
    let config = ProjectConfig::default_v2_for(name);
    write_new_file(&layout.config(), config.to_yaml().as_bytes())?;
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

    #[test]
    fn generated_config_round_trips_m1_fields() {
        let expected = ProjectConfig::default_for("calc");
        let parsed = ProjectConfig::from_yaml(&expected.to_yaml(), "fallback").unwrap();
        assert_eq!(parsed.project.name, "calc");
        assert!(parsed.scan.include.contains(&"src".to_owned()));
        assert_eq!(parsed.run.coverage, "llvm-cov");
    }

    #[test]
    fn explicit_include_list_does_not_retain_defaults() {
        let parsed = ProjectConfig::from_yaml(
            "version: 1\nproject:\n  name: x\nscan:\n  include:\n    - examples\n",
            "fallback",
        )
        .unwrap();
        assert_eq!(parsed.scan.include, vec!["examples"]);
    }

    #[test]
    fn assertion_macro_block_list_round_trips_without_becoming_scan_includes() {
        let mut expected = ProjectConfig::default_for("calc");
        expected.scan.include = vec!["tests".to_owned()];
        expected.scan.assertion_macros = vec![
            "assert_valid".to_owned(),
            "crate::checks::assert_result".to_owned(),
        ];

        let yaml = expected.to_yaml();
        assert!(yaml.contains("  assertion_macros:\n    - assert_valid\n"));
        let parsed = ProjectConfig::from_yaml(&yaml, "fallback").unwrap();

        assert_eq!(parsed, expected);
        assert_eq!(parsed.scan.include, vec!["tests"]);
    }

    #[test]
    fn scan_lists_accept_documented_bracketed_and_empty_forms() {
        let parsed = ProjectConfig::from_yaml(
            "scan:\n  include: [\"examples,with-comma\", tests]\n  assertion_macros: []\n",
            "fallback",
        )
        .unwrap();

        assert_eq!(parsed.scan.include, vec!["examples,with-comma", "tests"]);
        assert!(parsed.scan.assertion_macros.is_empty());
    }

    #[test]
    fn invalid_assertion_macro_path_is_rejected() {
        let error = ProjectConfig::from_yaml(
            "scan:\n  assertion_macros:\n    - assert-valid\n",
            "fallback",
        )
        .expect_err("macro names must be Rust identifiers or Rust paths");
        assert!(error.to_string().contains("assertion_macros"));
    }

    #[test]
    fn unsupported_coverage_mode_is_rejected() {
        let error = ProjectConfig::from_yaml("run:\n  coverage: guessed\n", "fallback")
            .expect_err("unknown coverage mode must fail closed");
        assert!(error.to_string().contains("run.coverage"));
    }

    #[test]
    fn version_two_config_round_trips_adapter_namespaces() {
        let config = ProjectConfig::default_v2_for("calc");
        let yaml = config.to_yaml();
        assert!(yaml.contains("version: 2\n"));
        assert!(yaml.contains("adapters:\n  - id: rust-cargo\n"));
        let parsed = ProjectConfig::from_yaml(&yaml, "fallback").unwrap();
        assert_eq!(parsed, config);
        assert_eq!(parsed.adapters[0].roots, vec!["."]);
    }

    #[test]
    fn version_two_duplicate_adapter_id_or_root_fails_closed() {
        let duplicate_id = "version: 2\nadapters:\n  - id: rust-cargo\n    roots: [.]\n    scan:\n      include: [src]\n      assertion_macros: []\n    run:\n      coverage: off\n  - id: rust-cargo\n    roots: [tests]\n    scan:\n      include: [tests]\n      assertion_macros: []\n    run:\n      coverage: off\n";
        assert!(ProjectConfig::from_yaml(duplicate_id, "fallback").is_err());

        let duplicate_root = "version: 2\nadapters:\n  - id: rust-cargo\n    roots:\n      - .\n      - .\n    scan:\n      include: [src]\n      assertion_macros: []\n    run:\n      coverage: off\n";
        assert!(ProjectConfig::from_yaml(duplicate_root, "fallback").is_err());
    }

    #[test]
    fn version_two_non_rust_adapter_keeps_opaque_runner_settings() {
        let yaml = "version: 2\nproject:\n  name: mixed\nadapters:\n  - id: synthetic\n    roots: [. ]\n    scan:\n      include: [fixtures]\n      assertion_macros: [not-a-rust-macro] \n    run:\n      coverage: synthetic-cov\n";
        let parsed = ProjectConfig::from_yaml(yaml, "fallback").unwrap();
        assert_eq!(parsed.adapters[0].id, "synthetic");
        assert_eq!(parsed.adapters[0].run.coverage, "synthetic-cov");
        assert_eq!(
            parsed.adapters[0].scan.assertion_macros,
            ["not-a-rust-macro"]
        );
    }
}
