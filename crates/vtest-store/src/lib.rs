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

    /// A record does not conform to the canonical schema it declares —
    /// DS-1645: "E-SCAN-010はerrorであり、レコードのid / ファイル名 /
    /// schema不一致（宣言されていない余剰 field を含む）...を意味する". This
    /// is the fail-closed replacement for the retired `DS-376` "warn and
    /// continue" behavior (see `docs/canonical/relations/retired-ids.json`);
    /// `code` carries the diagnostic code so a caller (PR6) can map it to an
    /// exit code without re-deriving it from a string.
    #[error("{code} at {location}: {detail}")]
    SchemaMismatch {
        code: &'static str,
        location: String,
        detail: String,
    },
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

    /// Canonical upstream-document record directory. BD-318: "...正典は
    /// `.verify/doc/` に置く"; BD-323: "`.verify/doc/` は文書ごとに1つの
    /// JSON ファイルとして上流文書（正典）を格納する". Replaces the
    /// predecessor `spec/`/`req/` split; those accessors stay for the readers
    /// that still use them until PR8 removes the predecessor model.
    pub fn doc_dir(&self) -> PathBuf {
        self.verify_dir().join("doc")
    }

    pub fn vo_dir(&self) -> PathBuf {
        self.verify_dir().join("vo")
    }

    /// Judgment-record directory. BD-024: "判断記録のIDはULIDとし、正典は
    /// `.verify/decisions/` に置く"; BD-051: "`.verify/decisions/` は判断記録
    /// （事実・追記型）を格納する". New in the canonical v0.1 layout; there is
    /// no predecessor equivalent.
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

/// Canonical v0.1 project configuration. The writer's normal form is
/// version 2; an explicit `version: 1` is read as a single implicit
/// `rust-cargo` adapter and converted in-memory to this shape without
/// rewriting the file (DES-014/DES-109: "readerはversion 1を単一の
/// `rust-cargo` adapter設定としてin-memory変換して読み取るが、読み取りだけで
/// 正典を書き換えない"). `version` itself is required (DS-1572: "config
/// readerはversion 1とversion 2を受理し、読み取りだけでconfigを書き換えない" —
/// stated only for a *declared* 1 or 2, silent on an absent key; this reader
/// treats that silence as fail-closed rather than as license to guess,
/// matching DS-1652's listing of `config version` itself among the
/// `E-CONFIG-001` conditions), and every key must belong to the schema its
/// declared version actually has — see `ProjectConfig::from_yaml`.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProjectConfig {
    pub version: u32,
    pub project: ProjectSection,
    pub adapters: Vec<AdapterConfig>,
    pub verify: VerifySection,

    /// DS-362: "`gates` field自体の欠落と空listは「ゲート定義なし」として
    /// 受理する" — absence and `gates: []` are equivalent.
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
/// sections): BD-155 delegates adapter-payload validation to the registered
/// adapter itself ("adapter固有設定の検証は登録adapterへ委譲する"), and
/// BD-157 treats `scan`/`run` as version-1-schema-compatible wire values
/// ("`scan` と `run` はversion 1 schema互換のwire値とする"). PR2 has no
/// adapter registry yet, so this struct's fixed Rust-cargo-shaped fields are
/// an existing constraint, not this invariant's concern; a registry PR
/// replaces this direct-deserialize with delegated validation instead of
/// tightening it here.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ScanSection {
    /// DS-349: "`config.yaml` の各adapterの `scan` 設定の `include` はテスト
    /// コード走査パスであり、省略時はワークスペース全体を対象とする". `None`
    /// carries that omission faithfully (this store crate does not itself
    /// resolve "whole workspace" into a path list — that is scan's own
    /// concern once it consumes this config); `Some(paths)` is an explicit,
    /// non-default set of scan paths. `default_for` writes `Some(vec!["src",
    /// "tests", "crates"])` because that is the concrete value BD-154's own
    /// literal `config.yaml` example gives for this key, not because that
    /// list is DS-349's stated default (DS-349's default is "the whole
    /// workspace", not these three directories). What an *explicit*
    /// `include: []` should mean is not stated by any canonical node found
    /// so far — disclosed, not decided here.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub include: Option<Vec<String>>,
    pub assertion_macros: Vec<String>,
}

/// See `ScanSection`'s doc comment: same adapter-delegated-validation reason
/// for not denying unknown fields here.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct RunSection {
    pub coverage: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct VerifySection {
    pub full_scope: Vec<String>,
}

/// One phase-gate definition. BD-226: "ゲート定義は、`config.yaml`の
/// `gates`に、ゲート名と進行条件（`require.verification`＝要求する検証結果、
/// `require.approvals`＝要求する承認ロール集合）を保持する".
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

    /// DS-372: "`require.approvals` は省略可能とし、省略は「要求する承認
    /// ロールなし（空集合）」として受理する".
    #[serde(default)]
    pub approvals: Vec<String>,
}

/// The fixed four checks `verify.full_scope` must enumerate exactly — no
/// more, no fewer, no duplicates, no unrecognized names. SPEC-053: "検証は
/// `chain_integrity` / `orphan_detection` / `target_binding` /
/// `oracle_presence` の4検査のみで行う"; DS-355/DS-356/DS-1652 make this a
/// `config.yaml` invariant (`E-CONFIG-001` on violation).
const FIXED_FULL_SCOPE: [&str; 4] = [
    "chain_integrity",
    "orphan_detection",
    "target_binding",
    "oracle_presence",
];

/// The five verification states `gates[].require.verification` may name.
/// SPEC-371: "検証状態は5値（`PASS` / `FAIL` / `MISMATCH` / `NO_EVIDENCE` /
/// `UNKNOWN`）である"; DS-1529/DS-1652 require an exact, case-sensitive
/// match at config-read time (`E-CONFIG-001` otherwise).
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
                    include: Some(vec![
                        "src".to_owned(),
                        "tests".to_owned(),
                        "crates".to_owned(),
                    ]),
                    assertion_macros: Vec::new(),
                },
                run: RunSection {
                    coverage: "llvm-cov".to_owned(),
                },
            }],
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
    /// guarantee it), matching DS-1573: "config writerと`vtest init`は
    /// version 2のadapter namespaceを出力する".
    pub fn to_yaml(&self) -> String {
        yaml_serde::to_string(self).expect("ProjectConfig always serializes to valid YAML")
    }

    /// Read a project configuration. `version` is read through the YAML
    /// model itself, not a hand-rolled text scan: a config that is not a
    /// YAML mapping, or that has no `version` key, or whose `version` is not
    /// an integer, fails closed rather than being guessed at. DES-014
    /// ("readerはversion 1を単一の `rust-cargo` adapter設定として
    /// in-memory変換して読み取る") and DS-1572 ("config readerはversion 1と
    /// version 2を受理し、読み取りだけでconfigを書き換えない") both only ever
    /// speak of a *declared* version 1 or 2 — neither states nor implies a
    /// third "absent" case. This reader's rejection of an absent `version`
    /// key is therefore this crate's own decision under that silence, not a
    /// stated canonical rule: disclosed here as existing, undocumented
    /// behavior kept because the alternative (defaulting a missing version
    /// to some number) is exactly the kind of silent-promotion DS-1652
    /// forbids for every *other* E-CONFIG-001 condition it lists, `config
    /// version` among them.
    /// `version: 2` is parsed as written; `version: 1` is parsed under the
    /// version 1 shape and converted in-memory to this (version 2) shape —
    /// DES-109: "`config.yaml` readerはversion 1を単一の `rust-cargo`
    /// adapter設定としてin-memory変換して読み取るが、読み取りだけで正典を
    /// 書き換えない" (a read never rewrites the canonical file). Any other
    /// version — malformed, or a number this reader does not recognize — is
    /// rejected: DS-1652 lists `config version` itself among the
    /// `E-CONFIG-001` conditions, so guessing at an unknown schema version
    /// would be exactly the silent-promotion this system's fail-closed
    /// design forbids. Every key must also belong to the schema its
    /// declared version actually has (`#[serde(deny_unknown_fields)]` on
    /// `ProjectConfig`/`V1Config` and their sub-sections), e.g. a `version:
    /// 1` config carrying a v2-only `gates:` key is rejected. DS-1652's own
    /// enumeration does state this for some fields specifically —
    /// `verify.full_scope`'s duplicate/unknown/missing/surplus items
    /// (DS-356/DS-1495) and an unresolved `gates[].require.approvals` role
    /// — and its own parenthetical carves unknown/duplicate adapter ids out
    /// to `E-ADAPTER-001` instead ("未知・重複adapter IDはE-ADAPTER-001").
    /// (DS-352, elsewhere in the same detailed_spec layer, assigns exactly
    /// that case — "adapter IDの重複...未知adapter...はusage error
    /// （E-CONFIG-001）" — to `E-CONFIG-001` instead, contradicting DS-1652's
    /// own carve-out; not resolved here, since this crate has no adapter
    /// registry yet for either code to apply to — see `ScanSection`'s doc
    /// comment on that gap.) Beyond these named fields, DS-1652 does not
    /// name a stray top-level or nested key in general (a bare `config
    /// field型` mismatch is the closest listed condition, and a surplus key
    /// is not a type mismatch) as an `E-CONFIG-001` condition. Rejecting
    /// every unrecognized key unconditionally is therefore this crate's own
    /// decision under that silence, not a stated canonical rule for the
    /// general case — kept
    /// fail-closed for the same reason the absent-`version` case above is:
    /// silently accepting a surplus key a writer expected to constrain
    /// something (e.g. a misspelled restriction field) is exactly the
    /// silent-promotion DS-1645/DS-1652 forbid elsewhere.
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
    /// `project`/`adapters`/`verify` section fails closed through the
    /// derive's standard "missing field" behavior (none of the three carry
    /// `#[serde(default)]`); `gates`/`approval_roles` default to empty,
    /// matching DS-362's "`gates` field自体の欠落と空listは「ゲート定義な
    /// し」として受理する".
    fn from_yaml_v2(value: yaml_serde::Value) -> Result<Self, StoreError> {
        let config: Self = yaml_serde::from_value(value)
            .map_err(|error| StoreError::InvalidConfig(format!("invalid v2 config: {error}")))?;
        validate_v2_config(&config)?;
        Ok(config)
    }

    /// Reads a version 1 configuration and converts it in-memory to the
    /// version 2 shape: a single implicit `rust-cargo` adapter, no gates, no
    /// approval roles. (A prior version of this comment also said "no doc
    /// roots"; 0861f12 removed the `doc`/`DocSection`/`doc.roots` key this
    /// referred to from `ProjectConfig` entirely — DS-1646 makes root-layer
    /// membership itself the orphan-detection root, so there is no
    /// config-level document-root list left for a v1 config to lack.)
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

        // DS-349: an omitted `scan.include` means "the whole workspace",
        // not this store's own guess at a directory list — carried through
        // as `None` (see `ScanSection::include`'s doc comment) rather than
        // backfilled with `["src", "tests", "crates"]` as a prior version
        // of this reader did (that value is BD-154's example, not DS-349's
        // stated default, and narrowing the scan surface by inventing it
        // would hide tests outside those three directories from downstream
        // orphan/coverage checks without the writer ever having said so).
        let include = v1.scan.as_ref().and_then(|scan| scan.include.clone());

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

        // DS-357/DS-1494/DS-1108: "version 1では、`verify.full_scope` の
        // field欠落を固定4検査として具体化し、重複または未知項目は
        // E-CONFIG-001で拒否する"; DS-1109: "in-memory の項目補完は行わない".
        // A *present* full_scope goes straight to validate_full_scope with
        // no dedup step first — a prior version of this reader deduped
        // before validating, which silently hid adjacent duplicates from
        // the very check meant to reject them.
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
/// vocabulary, and unresolved `require.approvals` roles (DS-1162, DS-1652).
/// This config carries no document-root configuration to cross-reference:
/// DS-1646 makes root-layer membership itself the orphan-detection root
/// ("根の指定は `root` 層への所属であり…設定による除外指定は持たない"), so
/// there is no `doc.roots`-shaped entry for this parser (or any other
/// component) to resolve against a registered document set.
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
        // DS-373: "`require.approvals` を指定する場合は文字列ロール名の
        // listとし、空文字列・重複ロール名はE-CONFIG-001（終了コード2）と
        // する" — both conditions are scoped to *this gate's own*
        // `approvals` list, distinct from whether a name resolves in
        // `approval_roles` at all (checked below, DS-1162).
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
/// Generic over the directory: used for both the canonical (`vo/`) and the
/// predecessor (`spec/`, `req/`) record layouts. Full schema validation is a
/// separate concern; this read-side helper never writes derived cache
/// files. Not used for `doc/`: BD-319/BD-320 make the upstream document
/// model's file format JSON, not YAML — see `read_document_names`.
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

/// Returns the file-stem names of every `.json` upstream document file in
/// `directory` (`.verify/doc/`), sorted. BD-330/DES-585: this name is the
/// document's own identity (author-chosen, never a machine-generated
/// identifier) — there is no separate `id` field inside the file to check
/// it against, unlike `read_record_ids`'s YAML records.
pub fn read_document_names(directory: &Path) -> Result<Vec<String>, StoreError> {
    let entries = fs::read_dir(directory).map_err(|source| StoreError::Io {
        path: directory.to_owned(),
        source,
    })?;
    let mut names = Vec::new();
    for entry in entries {
        let entry = entry.map_err(|source| StoreError::Io {
            path: directory.to_owned(),
            source,
        })?;
        let path = entry.path();
        if path.extension().and_then(|v| v.to_str()) != Some("json") {
            continue;
        }
        if let Some(stem) = path.file_stem().and_then(|v| v.to_str()) {
            names.push(stem.to_owned());
        }
    }
    names.sort();
    Ok(names)
}

/// Names/IDs of every registered upstream document and VO record (BD-323's
/// `doc/`+BD-139's `vo/` layout — the predecessor `spec/`+`req/` split
/// collapsed into the single generic document model PR1/PR20 introduced,
/// so this returns two slots, not the predecessor reader's three).
/// `vtest-scan`, this function's only caller, still expects the retired
/// three-slot `[spec, req, vo]` shape and does not compile against this
/// branch's canonical `ProjectConfig` regardless (28 pre-existing errors,
/// unrelated to this change); updating that caller to the shape below is
/// PR3's job, when scan itself moves onto the canonical model.
pub fn read_entity_ids(root: &Path) -> Result<[Vec<String>; 2], StoreError> {
    let layout = VerifyLayout::new(root);
    Ok([
        read_document_names(&layout.doc_dir())?,
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

        // BD-323 replaces spec/+req/ with doc/ ("`.verify/doc/` は文書ごとに
        // 1つの JSON ファイルとして上流文書（正典）を格納する"); DS-985's own
        // `vtest init` generation list ("`doc/` / `vo/` / `rel/` / `forms/` /
        // `decisions/` / `approvals/` / `evidence/` / `cache/` と
        // `.verify/.gitignore`...") has no `audits/` entry, backing its
        // absence here too.
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
        canonical::write_document_file(
            &layout,
            "DOC-A",
            &vtest_model::DocumentFile {
                schema_version: "0.1".to_owned(),
                root: vec![vtest_model::RootNode {
                    id: vtest_model::DocumentId::new("ROOT-001"),
                    statement: "A frozen ruling.".to_owned(),
                    description: None,
                    source: vtest_model::NodeSource {
                        doc: "docs/a.md".to_owned(),
                        heading: "1".to_owned(),
                        lines: [1, 1],
                    },
                }],
                request: vec![],
                require: vec![],
                spec: vec![],
                detailed_spec: vec![],
                basic_design: vec![],
                design: vec![],
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
        assert!(parsed.adapters[0]
            .scan
            .include
            .as_ref()
            .is_some_and(|include| include.contains(&"src".to_owned())));
        assert_eq!(parsed.adapters[0].run.coverage, "llvm-cov");
    }

    /// BD-154's own literal `config.yaml` example (its `description`
    /// field), verbatim including its inline comments — no `doc:` block:
    /// BD-154 goes straight from `adapters:` to `verify:`, corroborating
    /// removal of `DocSection`/`doc.roots` (DS-1646 already makes `root`
    /// layer membership itself the orphan-detection root, with no config
    /// exclusion mechanism).
    ///
    /// BD-154's own text ends without an `approval_roles:` section even
    /// though its `gates` reference the `reviewer`/`owner` roles — DS-1162/
    /// DS-1652 make an unresolved `gates.require.approvals` role a fail-
    /// closed E-CONFIG-001 condition, so this literal example, fed exactly
    /// as BD-154 states it, is rejected by this reader. This is disclosed
    /// as a spec-internal gap (BD-154's own quoted `lines` range, 131-155,
    /// is a sub-range of the full "### 2.2 config.yaml" section, 131-173 —
    /// the source markdown very likely continued past line 155 with an
    /// `approval_roles:` block that BD-154's own text does not capture),
    /// not something this PR resolves by inventing role data BD-154 itself
    /// does not state.
    #[test]
    fn bd_154_example_config_fails_closed_on_its_own_unresolved_approval_roles() {
        let yaml = concat!(
            "version: 2\n",
            "project:\n",
            "  name: example\n",
            "adapters:\n",
            "  - id: rust-cargo\n",
            "    roots: [\".\"]\n",
            "    scan:\n",
            "      include: [src, tests, crates]   # テストコード走査パス。省略時はワークスペース全体\n",
            "      assertion_macros: []            # 追加で assert 相当として扱うマクロ名\n",
            "    run:\n",
            "      coverage: llvm-cov              # target_binding 動的計測方式: llvm-cov | off\n",
            "verify:\n",
            "  full_scope: [chain_integrity, orphan_detection, target_binding, oracle_presence]\n",
            "gates:                                # フェーズゲート定義（§11.5、基本仕様 §20）\n",
            "  - name: development\n",
            "    require: { verification: PASS }\n",
            "  - name: release\n",
            "    require: { verification: PASS, approvals: [reviewer] }\n",
            "  - name: delivery\n",
            "    require: { verification: PASS, approvals: [owner] }\n",
        );

        let error = ProjectConfig::from_yaml(yaml, "fallback")
            .expect_err("BD-154's own example, taken verbatim, does not itself define the approval roles its gates reference");
        assert!(error.to_string().contains("approval role"));
    }

    /// The structural (non-role) shape of BD-154's example does parse: this
    /// isolates that from the unresolved-role gap the test above discloses,
    /// by supplying the `approval_roles:` DS-1160 itself shows as the
    /// mapping's shape (role name → list of approver ids) — not part of
    /// BD-154's own quoted text, and not asserted to be BD-154's own text.
    #[test]
    fn bd_154_example_config_parses_once_its_disclosed_gap_is_filled() {
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
            Some(vec![
                "src".to_owned(),
                "tests".to_owned(),
                "crates".to_owned()
            ])
        );
        assert!(config.adapters[0].scan.assertion_macros.is_empty());
        assert_eq!(config.adapters[0].run.coverage, "llvm-cov");
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
        assert_eq!(
            parsed.adapters[0].scan.include,
            Some(vec!["examples".to_owned()])
        );
        assert!(parsed.gates.is_empty());
    }

    /// DS-349: "省略時はワークスペース全体を対象とする" — an omitted v2
    /// `scan.include` must parse as `None`, not be backfilled with any
    /// concrete directory list this store crate invents on its own.
    #[test]
    fn v2_config_with_omitted_scan_include_parses_to_none() {
        let yaml = "version: 2\nproject:\n  name: x\nadapters:\n  - id: rust-cargo\n    roots: [\".\"]\n    scan:\n      assertion_macros: []\n    run:\n      coverage: llvm-cov\nverify:\n  full_scope: [chain_integrity, orphan_detection, target_binding, oracle_presence]\n";
        let parsed = ProjectConfig::from_yaml(yaml, "fallback").unwrap();
        assert_eq!(parsed.adapters[0].scan.include, None);
    }

    /// Same DS-349 omission, on the version 1 compatibility path — a prior
    /// version of this reader backfilled an omitted v1 `scan.include` with
    /// `["src", "tests", "crates"]`, a value under no canonical node and
    /// narrower than DS-349's stated "whole workspace" default.
    #[test]
    fn v1_config_with_omitted_scan_include_parses_to_none() {
        let parsed =
            ProjectConfig::from_yaml("version: 1\nproject:\n  name: x\n", "fallback").unwrap();
        assert_eq!(parsed.adapters[0].scan.include, None);
    }

    /// DS-1572 ("config readerはversion 1とversion 2を受理し") speaks only
    /// of a *declared* version 1 or 2 — it neither states nor implies a
    /// third "no declared version" case, and no writer in this codebase
    /// (nor its predecessor) has ever emitted a config without a `version`
    /// key. This test locks in this reader's own fail-closed choice under
    /// that canonical silence (see `ProjectConfig::from_yaml`'s doc
    /// comment): guessing "1" for an absent version would be exactly the
    /// kind of silent-promotion DS-1652 forbids for every *stated*
    /// E-CONFIG-001 condition.
    #[test]
    fn unversioned_config_is_rejected() {
        let error = ProjectConfig::from_yaml(
            "scan:\n  include: [\"examples,with-comma\", tests]\n  assertion_macros: []\n",
            "fallback",
        )
        .expect_err("a config with no `version` key must fail closed");
        assert!(error.to_string().contains("version"));
    }

    /// DS-1652 lists `config version` among the `E-CONFIG-001` conditions:
    /// an unrecognized version must fail closed, not be guessed at as
    /// whichever schema is "closest".
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
        let yaml = "version: 2\nproject:\n  name: x\nadapters: []\nverify:\n  full_scope: [chain_integrity, orphan_detection, target_binding, oracle_presence]\napproval_roles:\n  reviewer: [a]\n  reviewer: [b]\n";
        ProjectConfig::from_yaml(yaml, "fallback")
            .expect_err("a duplicate key inside approval_roles must fail closed");
    }

    #[test]
    fn v2_config_missing_a_required_section_is_rejected() {
        let full = ProjectConfig::default_for("calc").to_yaml();
        let lines: Vec<&str> = full.lines().collect();
        for section in ["project", "adapters", "verify"] {
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
        let yaml = "version: 2\nproject:\n  name: x\nadapters: []\nverify:\n  full_scope: [chain_integrity, orphan_detection, target_binding, oracle_presence]\n";
        let parsed = ProjectConfig::from_yaml(yaml, "fallback").unwrap();
        assert!(
            parsed.adapters.is_empty(),
            "an explicitly empty adapters: [] must not be silently backfilled with the default adapter"
        );
    }

    /// DS-358/DS-1494: the old 12-item full_scope enumeration ("旧12項目の
    /// 列挙…") violates the current fixed-4-checks invariant regardless of
    /// config version.
    #[test]
    fn predecessor_twelve_item_full_scope_is_rejected() {
        let error = ProjectConfig::from_yaml(
            "version: 1\nverify:\n  full_scope:\n    - spec_coverage\n    - vo_decomposition\n",
            "fallback",
        )
        .expect_err("the predecessor full_scope vocabulary must fail closed");
        assert!(error.to_string().contains("full_scope"));
    }

    /// DS-357: "version 1では、`verify.full_scope` のfield欠落を固定4検査
    /// として具体化し、重複または未知項目はE-CONFIG-001で拒否する";
    /// DS-1109: "in-memory の項目補完は行わない". A prior version of this
    /// reader ran `Vec::dedup()` (adjacent-only) on a present v1
    /// `full_scope` before validating it, which silently erased exactly
    /// this shape of duplicate instead of rejecting it.
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

    /// Rejecting a stray key outside a declared version's own schema is
    /// this crate's own fail-closed choice, not a DS-1652 condition stated
    /// for the general case (see `ProjectConfig::from_yaml`'s doc comment)
    /// — implemented as `#[serde(deny_unknown_fields)]`, not a
    /// version-conditioned branch: a `version: 1` config carrying the
    /// v2-only `gates:` key is simply an invalid version-1 config,
    /// independent of any compatibility concern.
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
        let yaml = "version: 2\nproject:\n  name: x\nadapters: []\nverify:\n  full_scope: [chain_integrity, orphan_detection, target_binding, oracle_presence]\ngate: []\n";
        let error = ProjectConfig::from_yaml(yaml, "fallback")
            .expect_err("an unrecognized top-level config key must fail closed");
        assert!(error.to_string().contains("gate"));
    }

    #[test]
    fn v2_config_with_an_unknown_nested_project_key_is_rejected() {
        let yaml = "version: 2\nproject: {name: x, foo: y}\nadapters: []\nverify:\n  full_scope: [chain_integrity, orphan_detection, target_binding, oracle_presence]\n";
        let error = ProjectConfig::from_yaml(yaml, "fallback")
            .expect_err("an unknown key nested inside `project` must fail closed");
        assert!(error.to_string().contains("foo"));
    }

    #[test]
    fn v2_config_with_a_misspelled_nested_gate_requirement_key_is_rejected() {
        let yaml = "version: 2\nproject:\n  name: x\nadapters: []\nverify:\n  full_scope: [chain_integrity, orphan_detection, target_binding, oracle_presence]\ngates:\n  - name: release\n    require:\n      verification: PASS\n      approval: []\n";
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

    /// DS-373: "`require.approvals` を指定する場合は文字列ロール名の
    /// listとし、空文字列・重複ロール名はE-CONFIG-001とする".
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
