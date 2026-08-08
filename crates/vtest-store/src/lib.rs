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

impl ProjectConfig {
    pub fn default_for(name: impl Into<String>) -> Self {
        Self {
            version: 1,
            project: ProjectSection { name: name.into() },
            scan: ScanSection {
                include: vec!["src".to_owned(), "tests".to_owned(), "crates".to_owned()],
                assertion_macros: Vec::new(),
            },
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
            run: RunSection {
                coverage: "llvm-cov".to_owned(),
            },
        }
    }

    /// Serialize the stable subset needed by `vtest init` without depending
    /// on a YAML parser.  The full YAML parser is introduced with M2 record
    /// management; this output is valid YAML and intentionally deterministic.
    pub fn to_yaml(&self) -> String {
        let mut out = format!(
            "version: {}\nproject:\n  name: {}\nscan:\n  include:\n",
            self.version,
            yaml_scalar(&self.project.name)
        );
        for include in &self.scan.include {
            out.push_str(&format!("    - {}\n", yaml_scalar(include)));
        }
        out.push_str("  assertion_macros: []\nverify:\n  full_scope:\n");
        for item in &self.verify.full_scope {
            out.push_str(&format!("    - {}\n", yaml_scalar(item)));
        }
        out.push_str(&format!(
            "run:\n  coverage: {}\n",
            yaml_scalar(&self.run.coverage)
        ));
        out
    }

    /// Read only the fields needed by M1. Unknown or absent fields fall back
    /// to the documented defaults; malformed scalar values are rejected.
    pub fn from_yaml(text: &str, project_name: impl Into<String>) -> Result<Self, StoreError> {
        let mut config = Self::default_for(project_name);
        let mut section = "";
        let mut saw_include = false;
        let mut saw_full_scope = false;
        for raw in text.lines() {
            let line = raw.trim_end();
            if line.trim().is_empty() || line.trim_start().starts_with('#') {
                continue;
            }
            if !line.starts_with(' ') && line.ends_with(':') {
                section = line.trim_end_matches(':');
                continue;
            }
            let trimmed = line.trim();
            if trimmed.starts_with('-') {
                let value = trimmed.trim_start_matches('-').trim();
                let value = unquote_yaml_scalar(value)?;
                match section {
                    "scan" => {
                        if !saw_include {
                            config.scan.include.clear();
                            saw_include = true;
                        }
                        config.scan.include.push(value);
                    }
                    "verify" => {
                        if !saw_full_scope {
                            config.verify.full_scope.clear();
                            saw_full_scope = true;
                        }
                        config.verify.full_scope.push(value);
                    }
                    _ => {}
                }
                continue;
            }
            let Some((key, value)) = trimmed.split_once(':') else {
                return Err(StoreError::InvalidConfig(line.to_owned()));
            };
            let value = value.trim();
            match (section, key.trim()) {
                ("project", "name") => config.project.name = unquote_yaml_scalar(value)?,
                ("run", "coverage") => config.run.coverage = unquote_yaml_scalar(value)?,
                ("scan", "include") | ("verify", "full_scope") => {}
                ("scan", "assertion_macros") if value == "[]" => {}
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
        config.verify.full_scope.dedup();
        if !matches!(config.run.coverage.as_str(), "llvm-cov" | "off") {
            return Err(StoreError::InvalidConfig(format!(
                "run.coverage must be `llvm-cov` or `off`, got `{}`",
                config.run.coverage
            )));
        }
        Ok(config)
    }
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
    if value.starts_with('\'') && value.ends_with('\'') && value.len() >= 2 {
        Ok(value[1..value.len() - 1].replace("''", "'"))
    } else if value.starts_with('"') && value.ends_with('"') && value.len() >= 2 {
        Ok(value[1..value.len() - 1].to_owned())
    } else if value.is_empty() {
        Err(StoreError::InvalidConfig("empty YAML scalar".to_owned()))
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
    fn unsupported_coverage_mode_is_rejected() {
        let error = ProjectConfig::from_yaml("run:\n  coverage: guessed\n", "fallback")
            .expect_err("unknown coverage mode must fail closed");
        assert!(error.to_string().contains("run.coverage"));
    }
}
