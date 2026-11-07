//! Hash-uncomputed execution inputs crossing the adapter boundary (DES-352–366).

/// Language-neutral, type-preserving projection; maps use sorted keys and lists
/// preserve order. No machine path or display-command parsing is needed by core.
pub type CanonicalProjection = serde_json::Value;

/// One repository or local-dependency input the adapter enumerated for the
/// Execution State manifest, before hashing (DES-101: the adapter lists,
/// core hashes).
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExecutionInputDraft {
    pub root_identity: String,
    pub root_relative_path: String,
    pub kind: String,
    pub bytes: Vec<u8>,
}

/// The adapter-produced, not-yet-hashed Execution State: schema identity,
/// completeness, HEAD revision, runner kind and invocation projection,
/// toolchain identity, effective config projection and the input manifest
/// (DES-097). Core derives `ExecutionState` from it.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExecutionStateDraft {
    pub schema_id: String,
    pub schema_version: String,
    pub complete: bool,
    pub head_revision: Option<String>,
    pub runner_kind: String,
    pub invocation: CanonicalProjection,
    pub toolchain_identity: String,
    pub effective_config: CanonicalProjection,
    pub inputs: Vec<ExecutionInputDraft>,
}

impl ExecutionStateDraft {
    /// `"<schema_id>-v<schema_version>"`, the value stored as
    /// `ExecutionState.schema`.
    pub fn schema(&self) -> String {
        format!("{}-v{}", self.schema_id, self.schema_version)
    }
}
