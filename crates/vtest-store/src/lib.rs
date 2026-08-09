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

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ProjectConfig {
    pub version: u32,
    pub project: ProjectSection,
    #[serde(default)]
    pub adapters: Vec<AdapterConfigEntry>,
    pub scan: ScanSection,
    pub verify: VerifySection,
    pub run: RunSection,
}

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
pub struct AdapterConfigEntry {
    pub id: String,
    pub roots: Vec<String>,
    pub scan: ScanSection,
    pub run: RunSection,
}

pub const FIXED_FULL_SCOPE: [&str; 12] = [
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

impl ProjectConfig {
    pub fn default_for(name: impl Into<String>) -> Self {
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
            adapters: vec![AdapterConfigEntry {
                id: "rust-cargo".to_owned(),
                roots: vec![".".to_owned()],
                scan: scan.clone(),
                run: run.clone(),
            }],
            scan,
            verify: VerifySection {
                full_scope: FIXED_FULL_SCOPE
                    .iter()
                    .map(|item| (*item).to_owned())
                    .collect(),
            },
            run,
        }
    }

    /// Serialize the project configuration in a deterministic YAML subset.
    pub fn to_yaml(&self) -> String {
        let mut out = format!(
            "version: {}\nproject:\n  name: {}\nadapters:\n",
            self.version,
            yaml_scalar(&self.project.name)
        );
        if self.version >= 2 {
            let mut adapters = self.adapters.clone();
            if let Some(first) = adapters.first_mut() {
                // `scan`/`run` remain a v1 compatibility view for callers
                // that have not yet moved to adapter-scoped settings.
                first.scan = self.scan.clone();
                first.run = self.run.clone();
            }
            for adapter in &adapters {
                out.push_str(&format!("  - id: {}\n", yaml_scalar(&adapter.id)));
                out.push_str("    roots:\n");
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
        } else {
            out.push_str("  - id: rust-cargo\n    roots: [.]\n");
            out.push_str("scan:\n  include:\n");
            for include in &self.scan.include {
                out.push_str(&format!("    - {}\n", yaml_scalar(include)));
            }
            out.push_str("  assertion_macros: []\n");
        }
        out.push_str("verify:\n  full_scope:\n");
        for item in &self.verify.full_scope {
            out.push_str(&format!("    - {}\n", yaml_scalar(item)));
        }
        if self.version < 2 {
            out.push_str(&format!(
                "run:\n  coverage: {}\n",
                yaml_scalar(&self.run.coverage)
            ));
        }
        out
    }

    /// Read the documented configuration subset. Unknown fields fall back to
    /// the documented defaults; malformed values in supported fields fail
    /// closed rather than silently changing scan behaviour.
    pub fn from_yaml(text: &str, project_name: impl Into<String>) -> Result<Self, StoreError> {
        let mut config = Self::default_for(project_name);
        config.version = top_level_scalar(text, "version")
            .map(|value| value.parse::<u32>())
            .transpose()
            .map_err(|_| StoreError::InvalidConfig("version must be an integer".to_owned()))?
            .unwrap_or(1);
        if config.version == 1 {
            config.adapters.clear();
        } else if config.version == 2 {
            config.adapters = parse_v2_adapters(text)?;
            let Some(first) = config.adapters.first() else {
                return Err(StoreError::InvalidConfig(
                    "version 2 requires at least one adapter".to_owned(),
                ));
            };
            config.scan = first.scan.clone();
            config.run = first.run.clone();
        }
        let mut section = "";
        let mut list_field = None;
        let mut saw_include = false;
        let mut saw_assertion_macros = false;
        let mut saw_full_scope = false;
        for raw in text.lines() {
            let line = raw.trim_end();
            if line.trim().is_empty() || line.trim_start().starts_with('#') {
                continue;
            }
            if !line.starts_with(' ') && line.ends_with(':') {
                section = line.trim_end_matches(':');
                list_field = None;
                continue;
            }
            let trimmed = line.trim();
            if trimmed.starts_with('-') {
                let value = trimmed.trim_start_matches('-').trim();
                let value = unquote_yaml_scalar(value)?;
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
                    None => {}
                }
                continue;
            }
            let Some((key, value)) = trimmed.split_once(':') else {
                return Err(StoreError::InvalidConfig(line.to_owned()));
            };
            let value = value.trim();
            list_field = None;
            match (section, key.trim()) {
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
                }
                _ => {}
            }
        }
        // `default_for` already has defaults.  Avoid duplicating them when
        // parsing the generated file itself.
        config.scan.include.dedup();
        config.scan.assertion_macros.dedup();
        normalize_full_scope(config.version, &mut config.verify.full_scope)?;
        for macro_name in &config.scan.assertion_macros {
            if !is_rust_macro_path(macro_name) {
                return Err(StoreError::InvalidConfig(format!(
                    "scan.assertion_macros contains invalid Rust macro path `{macro_name}`"
                )));
            }
        }
        if !matches!(config.run.coverage.as_str(), "llvm-cov" | "off") {
            return Err(StoreError::InvalidConfig(format!(
                "run.coverage must be `llvm-cov` or `off`, got `{}`",
                config.run.coverage
            )));
        }
        Ok(config)
    }
}

fn top_level_scalar(text: &str, key: &str) -> Option<String> {
    text.lines().find_map(|line| {
        if line.starts_with(' ') {
            return None;
        }
        let (name, value) = line.split_once(':')?;
        (name.trim() == key).then(|| unquote_yaml_scalar(value.trim()).ok())?
    })
}

fn parse_v2_adapters(text: &str) -> Result<Vec<AdapterConfigEntry>, StoreError> {
    let mut adapters = Vec::new();
    let mut current: Option<AdapterConfigEntry> = None;
    let mut list_field: Option<&str> = None;
    let mut inline_list;
    let mut section = "";
    for raw in text.lines() {
        let line = raw.trim_end();
        if line.starts_with("  - id:") {
            if let Some(adapter) = current.take() {
                adapters.push(adapter);
            }
            let id = unquote_yaml_scalar(line.trim_start_matches("  - id:").trim())?;
            current = Some(AdapterConfigEntry {
                id,
                roots: Vec::new(),
                scan: ScanSection {
                    include: Vec::new(),
                    assertion_macros: Vec::new(),
                },
                run: RunSection {
                    coverage: "llvm-cov".to_owned(),
                },
            });
            section = "adapter";
            list_field = None;
            continue;
        }
        let Some(adapter) = current.as_mut() else {
            continue;
        };
        let trimmed = line.trim();
        if trimmed == "scan:" {
            section = "scan";
            list_field = None;
            continue;
        }
        if trimmed == "run:" {
            section = "run";
            list_field = None;
            continue;
        }
        if trimmed.starts_with('-') {
            let value = unquote_yaml_scalar(trimmed.trim_start_matches('-').trim())?;
            match (section, list_field) {
                ("adapter", Some("roots")) => adapter.roots.push(value),
                ("scan", Some("include")) => adapter.scan.include.push(value),
                ("scan", Some("assertion_macros")) => adapter.scan.assertion_macros.push(value),
                _ => {}
            }
            continue;
        }
        let Some((key, raw_value)) = trimmed.split_once(':') else {
            continue;
        };
        let value = raw_value.trim();
        match (section, key.trim()) {
            ("adapter", "roots") => {
                list_field = Some("roots");
                inline_list = false;
                parse_list_value(value, &mut adapter.roots, &mut inline_list)?;
            }
            ("scan", "include") => {
                list_field = Some("include");
                inline_list = false;
                parse_list_value(value, &mut adapter.scan.include, &mut inline_list)?;
            }
            ("scan", "assertion_macros") => {
                list_field = Some("assertion_macros");
                inline_list = false;
                parse_list_value(value, &mut adapter.scan.assertion_macros, &mut inline_list)?;
            }
            ("run", "coverage") => {
                adapter.run.coverage = unquote_yaml_scalar(value)?;
                list_field = None;
            }
            _ => {}
        }
    }
    if let Some(adapter) = current {
        adapters.push(adapter);
    }
    for adapter in &mut adapters {
        if adapter.roots.is_empty() {
            adapter.roots.push(".".to_owned());
        }
        if adapter.scan.include.is_empty() {
            adapter.scan.include = vec!["src".to_owned(), "tests".to_owned(), "crates".to_owned()];
        }
        adapter.roots.sort();
        adapter.scan.include.sort();
        adapter.scan.assertion_macros.sort();
        if adapter.roots.windows(2).any(|pair| pair[0] == pair[1]) {
            return Err(StoreError::InvalidConfig(format!(
                "adapter `{}` has duplicate roots",
                adapter.id
            )));
        }
    }
    let mut ids = adapters
        .iter()
        .map(|adapter| adapter.id.as_str())
        .collect::<Vec<_>>();
    ids.sort_unstable();
    if ids.windows(2).any(|pair| pair[0] == pair[1]) {
        return Err(StoreError::InvalidConfig(
            "adapter IDs must be unique".to_owned(),
        ));
    }
    Ok(adapters)
}

fn normalize_full_scope(version: u32, scope: &mut Vec<String>) -> Result<(), StoreError> {
    let known = FIXED_FULL_SCOPE
        .iter()
        .copied()
        .collect::<std::collections::BTreeSet<_>>();
    let mut seen = std::collections::BTreeSet::new();
    for item in scope.iter() {
        if !known.contains(item.as_str()) {
            return Err(StoreError::InvalidConfig(format!(
                "verify.full_scope contains unknown item `{item}`"
            )));
        }
        if !seen.insert(item.as_str()) {
            return Err(StoreError::InvalidConfig(format!(
                "verify.full_scope contains duplicate item `{item}`"
            )));
        }
    }
    if version >= 2 {
        if scope.len() != FIXED_FULL_SCOPE.len()
            || FIXED_FULL_SCOPE.iter().any(|item| !seen.contains(item))
        {
            return Err(StoreError::InvalidConfig(
                "version 2 verify.full_scope must contain exactly the fixed 12 items".to_owned(),
            ));
        }
    } else {
        let existing = scope
            .iter()
            .cloned()
            .collect::<std::collections::BTreeSet<_>>();
        for item in FIXED_FULL_SCOPE {
            if !existing.contains(item) {
                scope.push(item.to_owned());
            }
        }
    }
    Ok(())
}

#[derive(Clone, Copy)]
enum ConfigList {
    ScanInclude,
    AssertionMacros,
    VerifyFullScope,
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
        expected.adapters[0].scan = expected.scan.clone();

        let yaml = expected.to_yaml();
        assert!(yaml.contains("      assertion_macros:\n        - assert_valid\n"));
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
}
