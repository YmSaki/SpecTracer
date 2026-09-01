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
/// normal form is version 2; version 1 (or an absent/unrecognized version)
/// is read as a single implicit `rust-cargo` adapter and converted in-memory
/// to this shape without rewriting the file (§2.4).
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ProjectConfig {
    pub version: u32,
    pub project: ProjectSection,
    pub adapters: Vec<AdapterConfig>,
    pub doc: DocSection,
    pub verify: VerifySection,
    pub gates: Vec<GateConfig>,
    pub approval_roles: BTreeMap<String, Vec<String>>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ProjectSection {
    pub name: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct AdapterConfig {
    pub id: String,
    pub roots: Vec<String>,
    pub scan: ScanSection,
    pub run: RunSection,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ScanSection {
    pub include: Vec<String>,
    pub assertion_macros: Vec<String>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct RunSection {
    pub coverage: String,
}

/// Orphan-detection roots for the document layer (詳細設計 v0.1 §2.2, §5.6).
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct DocSection {
    pub roots: Vec<DocumentId>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct VerifySection {
    pub full_scope: Vec<String>,
}

/// One phase-gate definition (詳細設計 v0.1 §2.2, §11.5).
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct GateConfig {
    pub name: String,
    pub require: GateRequirement,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct GateRequirement {
    pub verification: String,
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

    /// Serialize the project configuration in its canonical version 2 shape.
    /// Always writes `version: 2`: 詳細設計 v0.1 §2.2 states the writer's
    /// normal form is version 2 and that `vtest init` generates it.
    pub fn to_yaml(&self) -> String {
        let mut out = format!(
            "version: 2\nproject:\n  name: {}\nadapters:\n",
            yaml_scalar(&self.project.name)
        );
        for adapter in &self.adapters {
            out.push_str(&format!(
                "  - id: {}\n    roots: {}\n    scan:\n      include: {}\n      assertion_macros: {}\n    run:\n      coverage: {}\n",
                yaml_scalar(&adapter.id),
                bracket_list(adapter.roots.iter().map(String::as_str)),
                bracket_list(adapter.scan.include.iter().map(String::as_str)),
                bracket_list(adapter.scan.assertion_macros.iter().map(String::as_str)),
                yaml_scalar(&adapter.run.coverage),
            ));
        }
        out.push_str(&format!(
            "doc:\n  roots: {}\n",
            bracket_list(self.doc.roots.iter().map(DocumentId::as_str))
        ));
        out.push_str(&format!(
            "verify:\n  full_scope: {}\n",
            bracket_list(self.verify.full_scope.iter().map(String::as_str))
        ));
        if self.gates.is_empty() {
            out.push_str("gates: []\n");
        } else {
            out.push_str("gates:\n");
            for gate in &self.gates {
                out.push_str(&format!(
                    "  - name: {}\n    require: {{ verification: {}",
                    yaml_scalar(&gate.name),
                    yaml_scalar(&gate.require.verification),
                ));
                if !gate.require.approvals.is_empty() {
                    out.push_str(&format!(
                        ", approvals: {}",
                        bracket_list(gate.require.approvals.iter().map(String::as_str))
                    ));
                }
                out.push_str(" }\n");
            }
        }
        if self.approval_roles.is_empty() {
            out.push_str("approval_roles: {}\n");
        } else {
            out.push_str("approval_roles:\n");
            for (role, members) in &self.approval_roles {
                out.push_str(&format!(
                    "  {}: {}\n",
                    yaml_scalar(role),
                    bracket_list(members.iter().map(String::as_str)),
                ));
            }
        }
        out
    }

    /// Read a project configuration. Version 2 is parsed as written; version
    /// 1 (or an absent/unrecognized version) is parsed under the version 1
    /// shape and converted in-memory to this (version 2) shape (§2.4: a read
    /// never rewrites the canonical file).
    pub fn from_yaml(text: &str, project_name: impl Into<String>) -> Result<Self, StoreError> {
        if detect_config_version(text) >= 2 {
            Self::from_yaml_v2(text, project_name)
        } else {
            Self::from_yaml_v1(text, project_name)
        }
    }

    fn from_yaml_v2(text: &str, project_name: impl Into<String>) -> Result<Self, StoreError> {
        let lines: Vec<&str> = text.lines().collect();
        let mut config = Self::default_for(project_name);
        config.version = 2;

        if let Some(block) = find_top_level_block(&lines, "project") {
            if let Some(name) = scalar(&block.join("\n"), "name") {
                config.project.name = name;
            }
        }

        if let Some(block) = find_top_level_block(&lines, "adapters") {
            let mut adapters = Vec::new();
            for item in list_items(&block) {
                let item_text = item.join("\n");
                let id = scalar(&item_text, "id")
                    .ok_or_else(|| StoreError::InvalidConfig("adapter is missing id".to_owned()))?;
                adapters.push(AdapterConfig {
                    id,
                    roots: list(&item_text, "roots"),
                    scan: ScanSection {
                        include: list(&item_text, "include"),
                        assertion_macros: list(&item_text, "assertion_macros"),
                    },
                    run: RunSection {
                        coverage: scalar(&item_text, "coverage").unwrap_or_default(),
                    },
                });
            }
            config.adapters = adapters;
        }

        if let Some(block) = find_top_level_block(&lines, "doc") {
            config.doc.roots = list(&block.join("\n"), "roots")
                .into_iter()
                .map(DocumentId::new)
                .collect();
        }

        if let Some(block) = find_top_level_block(&lines, "verify") {
            config.verify.full_scope = list(&block.join("\n"), "full_scope");
        }

        if let Some(block) = find_top_level_block(&lines, "gates") {
            let mut gates = Vec::new();
            for item in list_items(&block) {
                let item_text = item.join("\n");
                let name = scalar(&item_text, "name")
                    .ok_or_else(|| StoreError::InvalidConfig("gate is missing name".to_owned()))?;
                let require = scalar(&item_text, "require").ok_or_else(|| {
                    StoreError::InvalidConfig(format!("gate `{name}` is missing require"))
                })?;
                gates.push(GateConfig {
                    name,
                    require: parse_gate_requirement(&require)?,
                });
            }
            config.gates = gates;
        }

        if let Some(block) = find_top_level_block(&lines, "approval_roles") {
            let mut approval_roles = BTreeMap::new();
            for raw in &block {
                let trimmed = raw.trim();
                if trimmed.is_empty() {
                    continue;
                }
                let Some((role, members)) = trimmed.split_once(':') else {
                    return Err(StoreError::InvalidConfig(format!(
                        "invalid approval_roles entry `{trimmed}`"
                    )));
                };
                approval_roles.insert(
                    unquote_yaml_scalar(role.trim())?,
                    parse_inline_list(members.trim())?,
                );
            }
            config.approval_roles = approval_roles;
        }

        validate_v2_config(&config)?;
        Ok(config)
    }

    /// Reads a version 1 (or unversioned) configuration and converts it
    /// in-memory to the version 2 shape: a single implicit `rust-cargo`
    /// adapter, no doc roots, no gates, no approval roles.
    fn from_yaml_v1(text: &str, project_name: impl Into<String>) -> Result<Self, StoreError> {
        let mut name = project_name.into();
        let mut include = vec!["src".to_owned(), "tests".to_owned(), "crates".to_owned()];
        let mut assertion_macros = Vec::new();
        let mut full_scope: Vec<String> = FIXED_FULL_SCOPE
            .iter()
            .map(|item| (*item).to_owned())
            .collect();
        let mut coverage = "llvm-cov".to_owned();

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
                            include.clear();
                            saw_include = true;
                        }
                        include.push(value);
                    }
                    Some(ConfigList::AssertionMacros) => {
                        if !saw_assertion_macros {
                            assertion_macros.clear();
                            saw_assertion_macros = true;
                        }
                        assertion_macros.push(value);
                    }
                    Some(ConfigList::VerifyFullScope) => {
                        if !saw_full_scope {
                            full_scope.clear();
                            saw_full_scope = true;
                        }
                        full_scope.push(value);
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
                ("project", "name") => name = unquote_yaml_scalar(value)?,
                ("run", "coverage") => coverage = unquote_yaml_scalar(value)?,
                ("scan", "include") => {
                    parse_list_value(value, &mut include, &mut saw_include)?;
                    if value.is_empty() {
                        list_field = Some(ConfigList::ScanInclude);
                    }
                }
                ("scan", "assertion_macros") => {
                    parse_list_value(value, &mut assertion_macros, &mut saw_assertion_macros)?;
                    if value.is_empty() {
                        list_field = Some(ConfigList::AssertionMacros);
                    }
                }
                ("verify", "full_scope") => {
                    parse_list_value(value, &mut full_scope, &mut saw_full_scope)?;
                    if value.is_empty() {
                        list_field = Some(ConfigList::VerifyFullScope);
                    }
                }
                ("", "version") => {}
                _ => {}
            }
        }
        include.dedup();
        assertion_macros.dedup();
        if saw_full_scope {
            full_scope.dedup();
        }
        for macro_name in &assertion_macros {
            if !is_rust_macro_path(macro_name) {
                return Err(StoreError::InvalidConfig(format!(
                    "scan.assertion_macros contains invalid Rust macro path `{macro_name}`"
                )));
            }
        }
        if !matches!(coverage.as_str(), "llvm-cov" | "off") {
            return Err(StoreError::InvalidConfig(format!(
                "run.coverage must be `llvm-cov` or `off`, got `{coverage}`"
            )));
        }
        if saw_full_scope {
            validate_full_scope(&full_scope)?;
        }

        let config = Self {
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
        };
        Ok(config)
    }
}

fn detect_config_version(text: &str) -> u32 {
    for raw in text.lines() {
        let line = raw.trim_end();
        if line.starts_with(' ') {
            continue;
        }
        if let Some((key, value)) = line.split_once(':') {
            if key.trim() == "version" {
                return value.trim().parse().unwrap_or(1);
            }
        }
    }
    1
}

/// Returns the indented lines immediately following an unindented `key:`
/// line, or `None` if that top-level key is absent.
fn find_top_level_block<'a>(lines: &[&'a str], key: &str) -> Option<Vec<&'a str>> {
    let start = lines
        .iter()
        .position(|line| !line.starts_with(' ') && line.trim() == format!("{key}:"))?;
    Some(
        lines[start + 1..]
            .iter()
            .take_while(|line| line.starts_with(' ') || line.trim().is_empty())
            .copied()
            .collect(),
    )
}

/// Splits a block into its `  - ` list items, each dedented by 2 spaces so
/// the item's own lines (including any further-nested `key:` sub-blocks)
/// can be searched flatly with `scalar`/`list` — every field name within one
/// adapter or gate item is unique regardless of nesting depth.
fn list_items(block: &[&str]) -> Vec<Vec<String>> {
    let mut items: Vec<Vec<String>> = Vec::new();
    for line in block {
        if let Some(rest) = line.strip_prefix("  - ") {
            items.push(vec![rest.to_owned()]);
        } else if let Some(rest) = line.strip_prefix("    ") {
            if let Some(current) = items.last_mut() {
                current.push(rest.to_owned());
            }
        }
    }
    items
}

/// Parses a `require: { verification: X }` or
/// `require: { verification: X, approvals: [a, b] }` inline flow mapping
/// (詳細設計 v0.1 §2.2's own literal example uses this style).
fn parse_gate_requirement(value: &str) -> Result<GateRequirement, StoreError> {
    let inner = value
        .trim()
        .strip_prefix('{')
        .and_then(|rest| rest.trim_end().strip_suffix('}'))
        .ok_or_else(|| {
            StoreError::InvalidConfig(format!(
                "gate require must be a flow mapping, got `{value}`"
            ))
        })?;

    let mut parts = Vec::new();
    let mut depth = 0i32;
    let mut current = String::new();
    for ch in inner.chars() {
        match ch {
            '[' => {
                depth += 1;
                current.push(ch);
            }
            ']' => {
                depth -= 1;
                current.push(ch);
            }
            ',' if depth == 0 => {
                parts.push(std::mem::take(&mut current));
            }
            _ => current.push(ch),
        }
    }
    if !current.trim().is_empty() {
        parts.push(current);
    }

    let mut verification = None;
    let mut approvals = Vec::new();
    for part in parts {
        let Some((key, val)) = part.split_once(':') else {
            return Err(StoreError::InvalidConfig(format!(
                "invalid gate require entry `{part}`"
            )));
        };
        match key.trim() {
            "verification" => verification = Some(unquote_yaml_scalar(val.trim())?),
            "approvals" => approvals = parse_inline_list(val.trim())?,
            other => {
                return Err(StoreError::InvalidConfig(format!(
                    "unknown gate require field `{other}`"
                )))
            }
        }
    }
    let verification = verification.ok_or_else(|| {
        StoreError::InvalidConfig("gate require is missing verification".to_owned())
    })?;
    Ok(GateRequirement {
        verification,
        approvals,
    })
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
        for role in &gate.require.approvals {
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

/// Formats a list as an inline flow sequence (`[a, b]`), matching the style
/// used throughout 詳細設計 v0.1 §2.2's own `config.yaml` example.
fn bracket_list<'a>(values: impl Iterator<Item = &'a str>) -> String {
    let values = values.map(yaml_scalar).collect::<Vec<_>>();
    format!("[{}]", values.join(", "))
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

    #[test]
    fn unversioned_config_is_read_as_version_1() {
        let parsed = ProjectConfig::from_yaml(
            "scan:\n  include: [\"examples,with-comma\", tests]\n  assertion_macros: []\n",
            "fallback",
        )
        .unwrap();

        assert_eq!(
            parsed.adapters[0].scan.include,
            vec!["examples,with-comma", "tests"]
        );
        assert!(parsed.adapters[0].scan.assertion_macros.is_empty());
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
