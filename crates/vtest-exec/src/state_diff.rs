//! Pre/post-run Execution State diff formatting for `E-EXEC-004` diagnostics.
//!
//! Boundary: this module only summarizes differences between two already
//! captured `(ExecutionStateDraft, ExecutionState)` pairs. It does not decide
//! whether `E-EXEC-004` applies (the caller invokes it only when the pairs
//! differ), does not read files or environment variables, and does not
//! interpret adapter-specific root identity values: every input is shown as
//! `<root_identity>:<root_relative_path>`. A root identity or path that is not
//! a logical relative path (the rule `vtest-store` applies when materializing
//! the state) violates the adapter contract and is replaced by a placeholder,
//! so absolute paths, file contents, and environment values are never emitted.
//!
//! Clauses: DS-532, DS-907, DS-1375, DES-097, DES-098, DES-099, DES-371.

use std::collections::{BTreeMap, BTreeSet};

use vtest_model::{ExecutionInputDraft, ExecutionState, ExecutionStateDraft};
use vtest_store::is_logical_path;

/// Upper bound on paths shown per change kind (`added`, `removed`, `modified`)
/// so a bulk file change stays readable in both text and JSON diagnostics.
/// The canonical specification does not define the content of this message
/// (DS-942); the value is an implementation choice to keep the display finite.
const MAX_REPORTED_INPUT_PATHS_PER_KIND: usize = 10;

const REDACTED_PATH: &str = "<non-relative-path>";

type InputKey<'a> = (&'a str, &'a str);
type InputEntry<'a> = (&'a str, &'a [u8]);
type GroupedInputs<'a> = BTreeMap<InputKey<'a>, Vec<InputEntry<'a>>>;

/// Formats a deterministic, secret-free summary explaining why the pre-run and
/// post-run Execution State snapshots differed.
///
/// The caller must call this only when the pairs differ; the states are
/// derived from the drafts, so both drafts being `None` cannot occur then.
pub(crate) fn summarize_execution_state_change(
    before_draft: Option<&ExecutionStateDraft>,
    after_draft: Option<&ExecutionStateDraft>,
    before_state: Option<&ExecutionState>,
    after_state: Option<&ExecutionState>,
) -> String {
    match (before_draft, after_draft) {
        (Some(before), Some(after)) => {
            summarize_draft_diff(before, after, before_state, after_state)
        }
        (Some(_), None) => {
            "snapshot availability changed (before: available, after: unavailable)".to_owned()
        }
        (None, _) => {
            "snapshot availability changed (before: unavailable, after: available)".to_owned()
        }
    }
}

fn summarize_draft_diff(
    before: &ExecutionStateDraft,
    after: &ExecutionStateDraft,
    before_state: Option<&ExecutionState>,
    after_state: Option<&ExecutionState>,
) -> String {
    let mut sections = Vec::new();
    let (added, removed, modified) = diff_inputs(&before.inputs, &after.inputs);

    for (label, paths) in [
        ("added", &added),
        ("removed", &removed),
        ("modified", &modified),
    ] {
        if let Some(section) = format_path_list(label, paths) {
            sections.push(section);
        }
    }

    let changed_elements = diff_non_input_elements(before, after, before_state, after_state);
    if !changed_elements.is_empty() {
        sections.push(format!("changed: {}", changed_elements.join(", ")));
    }

    sections.join("; ")
}

/// Returns `(added, removed, modified)` displayed inputs, each in the
/// lexicographic order of their `(root_identity, root_relative_path)` keys.
fn diff_inputs(
    before_inputs: &[ExecutionInputDraft],
    after_inputs: &[ExecutionInputDraft],
) -> (Vec<String>, Vec<String>, Vec<String>) {
    let before_map = group_inputs(before_inputs);
    let after_map = group_inputs(after_inputs);

    let keys: BTreeSet<InputKey<'_>> = before_map.keys().chain(after_map.keys()).copied().collect();

    let mut added = Vec::new();
    let mut removed = Vec::new();
    let mut modified = Vec::new();

    for key @ (root_identity, root_relative_path) in keys {
        let shown = format_input(root_identity, root_relative_path);
        match (before_map.get(&key), after_map.get(&key)) {
            (None, Some(_)) => added.push(shown),
            (Some(_), None) => removed.push(shown),
            (Some(lhs), Some(rhs)) if lhs != rhs => modified.push(shown),
            _ => {}
        }
    }

    (added, removed, modified)
}

fn group_inputs(inputs: &[ExecutionInputDraft]) -> GroupedInputs<'_> {
    let mut grouped: GroupedInputs<'_> = BTreeMap::new();
    for input in inputs {
        grouped
            .entry((
                input.root_identity.as_str(),
                input.root_relative_path.as_str(),
            ))
            .or_default()
            .push((input.kind.as_str(), input.bytes.as_slice()));
    }
    for entries in grouped.values_mut() {
        entries.sort_unstable();
    }
    grouped
}

fn diff_non_input_elements(
    before: &ExecutionStateDraft,
    after: &ExecutionStateDraft,
    before_state: Option<&ExecutionState>,
    after_state: Option<&ExecutionState>,
) -> Vec<&'static str> {
    let mut changed = Vec::new();
    if before.schema_id != after.schema_id
        || before.schema_version != after.schema_version
        || before_state.map(|state| &state.schema) != after_state.map(|state| &state.schema)
    {
        changed.push("schema");
    }
    if before.head_revision != after.head_revision {
        changed.push("head_revision");
    }
    if before.runner_kind != after.runner_kind {
        changed.push("runner_kind");
    }
    if before.invocation != after.invocation {
        changed.push("invocation");
    }
    if before.toolchain_identity != after.toolchain_identity {
        changed.push("toolchain");
    }
    // `effective_config` holds environment variable values and out-of-root
    // config contents; report only that `config` changed, never its values.
    if before.effective_config != after.effective_config {
        changed.push("config");
    }
    if before.complete != after.complete
        || before_state.map(|state| state.complete) != after_state.map(|state| state.complete)
    {
        changed.push("completeness");
    }
    changed
}

fn format_path_list(label: &str, paths: &[String]) -> Option<String> {
    if paths.is_empty() {
        return None;
    }
    let shown = paths.len().min(MAX_REPORTED_INPUT_PATHS_PER_KIND);
    let omitted = paths.len() - shown;
    let joined = paths[..shown].join(", ");
    if omitted > 0 {
        Some(format!("{label}: {joined} (+{omitted} omitted)"))
    } else {
        Some(format!("{label}: {joined}"))
    }
}

fn format_input(root_identity: &str, root_relative_path: &str) -> String {
    format!(
        "{}:{}",
        displayable(root_identity),
        displayable(root_relative_path)
    )
}

/// Adapters must return logical relative values; anything else (an absolute
/// or out-of-root path) is a contract violation and is never shown (AGENTS.md
/// 6.3).
fn displayable(value: &str) -> &str {
    if is_logical_path(value) {
        value
    } else {
        REDACTED_PATH
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn draft_with_inputs(inputs: Vec<ExecutionInputDraft>) -> ExecutionStateDraft {
        ExecutionStateDraft {
            schema_id: "rust-cargo-execution-state".into(),
            schema_version: "1".into(),
            complete: true,
            head_revision: Some("abc".into()),
            runner_kind: "cargo-test".into(),
            invocation: serde_json::json!(["cargo", "test"]),
            toolchain_identity: "rustc 1.0".into(),
            effective_config: serde_json::Value::Null,
            inputs,
        }
    }

    fn input(path: &str, bytes: &[u8]) -> ExecutionInputDraft {
        ExecutionInputDraft {
            root_identity: "root-a".into(),
            root_relative_path: path.into(),
            kind: "file".into(),
            bytes: bytes.to_vec(),
        }
    }

    /// @vtest.id TEST-EXEC-STATE-DIFF-AVAILABILITY-AND-SCHEMA
    /// @vtest.covers VO-EXEC-STATE-PREPOST-STABILITY
    /// @vtest.target crates/vtest-exec/src/state_diff.rs::summarize_execution_state_change
    /// @vtest.intent summarize_execution_state_change reports snapshot availability mismatches, schema, runner_kind, completeness, and every root identity uniformly and deterministically
    #[test]
    fn reports_availability_schema_runner_completeness_and_root_identities() {
        let base = draft_with_inputs(vec![input("Cargo.toml", b"a")]);

        let lost = summarize_execution_state_change(Some(&base), None, None, None);
        assert!(
            lost.contains("before: available, after: unavailable"),
            "{lost}"
        );

        let gained = summarize_execution_state_change(None, Some(&base), None, None);
        assert!(
            gained.contains("before: unavailable, after: available"),
            "{gained}"
        );

        let mut modified = draft_with_inputs(vec![
            input("Cargo.toml", b"a"),
            ExecutionInputDraft {
                root_identity: "dep-a".into(),
                root_relative_path: "src/lib.rs".into(),
                kind: "file".into(),
                bytes: b"x".to_vec(),
            },
            input("Cargo.lock", b"y"),
        ]);
        modified.schema_version = "2".into();
        modified.runner_kind = "cargo-llvm-cov".into();
        modified.complete = false;

        let summary = summarize_execution_state_change(Some(&base), Some(&modified), None, None);
        assert!(
            summary.contains("added: dep-a:src/lib.rs, root-a:Cargo.lock"),
            "{summary}"
        );
        assert!(
            summary.contains("changed: schema, runner_kind, completeness"),
            "{summary}"
        );
    }

    /// @vtest.id TEST-EXEC-STATE-DIFF-NON-LOGICAL-REDACTION
    /// @vtest.covers VO-EXEC-STATE-PREPOST-STABILITY
    /// @vtest.target crates/vtest-exec/src/state_diff.rs::summarize_execution_state_change
    /// @vtest.intent summarize_execution_state_change replaces absolute, drive-qualified, backslash, and parent-relative root identities or paths with a placeholder instead of showing them
    #[test]
    fn non_logical_identities_and_paths_are_redacted() {
        let base = draft_with_inputs(vec![]);
        let mut after = draft_with_inputs(vec![]);
        for path in [
            "/abs/secret.key",
            "C:/abs/secret.key",
            "a\\b.txt",
            "../up.txt",
        ] {
            after.inputs.push(input(path, b"x"));
        }
        after.inputs.push(ExecutionInputDraft {
            root_identity: "/abs/root".into(),
            root_relative_path: "ok.txt".into(),
            kind: "file".into(),
            bytes: b"x".to_vec(),
        });

        let summary = summarize_execution_state_change(Some(&base), Some(&after), None, None);
        for leaked in ["abs", "secret.key", "a\\b.txt", "up.txt"] {
            assert!(!summary.contains(leaked), "leaked {leaked}: {summary}");
        }
        assert!(summary.contains("<non-relative-path>:ok.txt"), "{summary}");
        assert!(summary.contains("root-a:<non-relative-path>"), "{summary}");
    }
}
