//! Pre/post-run Execution State diff formatting for `E-EXEC-004` diagnostics.
//!
//! Boundary: this module only summarizes differences between two already
//! captured `(ExecutionStateDraft, ExecutionState)` pairs. It does not decide
//! whether `E-EXEC-004` applies, does not read files or environment variables,
//! and never emits absolute paths, file contents, or environment values.
//!
//! Clauses: DS-532, DS-907, DS-1375, DES-097, DES-098, DES-099, DES-371.

use std::{
    collections::{BTreeMap, BTreeSet},
    path::Path,
};

use vtest_model::{ExecutionInputDraft, ExecutionState, ExecutionStateDraft};

/// Upper bound on paths shown per change kind (`added`, `removed`, `modified`)
/// so a bulk file change stays readable in both text and JSON diagnostics.
const MAX_REPORTED_INPUT_PATHS_PER_KIND: usize = 10;

const REDACTED_PATH: &str = "<non-relative-path>";
const REDACTED_ROOT: &str = "<non-relative-root>";

type InputKey<'a> = (&'a str, &'a str);
type InputEntry<'a> = (&'a str, &'a [u8]);
type GroupedInputs<'a> = BTreeMap<InputKey<'a>, Vec<InputEntry<'a>>>;

/// Formats a deterministic, secret-free summary explaining why the pre-run and
/// post-run Execution State snapshots differed.
pub(crate) fn summarize_execution_state_change(
    root: &Path,
    before_draft: Option<&ExecutionStateDraft>,
    after_draft: Option<&ExecutionStateDraft>,
    before_state: Option<&ExecutionState>,
    after_state: Option<&ExecutionState>,
) -> String {
    match (before_draft, after_draft) {
        (Some(_), None) => {
            "snapshot availability changed (before: available, after: unavailable)".to_owned()
        }
        (None, Some(_)) => {
            "snapshot availability changed (before: unavailable, after: available)".to_owned()
        }
        (None, None) => "execution state changed".to_owned(),
        (Some(before), Some(after)) => {
            summarize_draft_diff(root, before, after, before_state, after_state)
        }
    }
}

fn summarize_draft_diff(
    root: &Path,
    before: &ExecutionStateDraft,
    after: &ExecutionStateDraft,
    before_state: Option<&ExecutionState>,
    after_state: Option<&ExecutionState>,
) -> String {
    let mut sections = Vec::new();
    let (added, removed, modified) = diff_inputs(root, &before.inputs, &after.inputs);

    if let Some(section) = format_path_list("added", &added, MAX_REPORTED_INPUT_PATHS_PER_KIND) {
        sections.push(section);
    }
    if let Some(section) = format_path_list("removed", &removed, MAX_REPORTED_INPUT_PATHS_PER_KIND)
    {
        sections.push(section);
    }
    if let Some(section) =
        format_path_list("modified", &modified, MAX_REPORTED_INPUT_PATHS_PER_KIND)
    {
        sections.push(section);
    }

    let changed_elements = diff_non_input_elements(before, after, before_state, after_state);
    if !changed_elements.is_empty() {
        sections.push(format!("changed: {}", changed_elements.join(", ")));
    }

    if sections.is_empty() {
        // Fallback when only the materialized hash differs while all inspected
        // fields matched (should be unreachable for deterministic hashing).
        sections.push("changed: state_hash".to_owned());
    }

    sections.join("; ")
}

fn diff_inputs(
    root: &Path,
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
        match (before_map.get(&key), after_map.get(&key)) {
            (None, Some(_)) => {
                added.push(format_input_path(root, root_identity, root_relative_path))
            }
            (Some(_), None) => {
                removed.push(format_input_path(root, root_identity, root_relative_path))
            }
            (Some(lhs), Some(rhs)) if lhs != rhs => {
                modified.push(format_input_path(root, root_identity, root_relative_path))
            }
            (Some(_), Some(_)) | (None, None) => {}
        }
    }

    // Re-sort after sanitization so redacted entries also stay deterministic.
    added.sort();
    removed.sort();
    modified.sort();

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

fn format_path_list(label: &str, paths: &[String], limit: usize) -> Option<String> {
    if paths.is_empty() {
        return None;
    }
    let shown = paths.len().min(limit);
    let omitted = paths.len() - shown;
    let joined = paths[..shown].join(", ");
    if omitted > 0 {
        Some(format!("{label}: {joined} (+{omitted} omitted)"))
    } else {
        Some(format!("{label}: {joined}"))
    }
}

fn format_input_path(root: &Path, root_identity: &str, root_relative_path: &str) -> String {
    let rel = sanitize_relative_path(root, root_relative_path);
    if matches!(root_identity, "workspace-root" | "workspace") {
        rel
    } else {
        let root_label = sanitize_root_identity(root_identity);
        format!("{root_label}:{rel}")
    }
}

fn sanitize_relative_path(root: &Path, raw: &str) -> String {
    let stripped_os = Path::new(raw)
        .strip_prefix(root)
        .ok()
        .map(|path| path.to_string_lossy().replace('\\', "/"));

    let candidate = match stripped_os {
        Some(rel) => rel,
        None => {
            let root_norm = root.to_string_lossy().replace('\\', "/");
            let root_trimmed = root_norm.trim_end_matches('/');
            let raw_norm = raw.replace('\\', "/");
            if !root_trimmed.is_empty() {
                let prefix = format!("{root_trimmed}/");
                if let Some(rel) = raw_norm.strip_prefix(&prefix) {
                    rel.to_owned()
                } else {
                    raw_norm
                }
            } else {
                raw_norm
            }
        }
    };

    if is_safe_relative_path(&candidate) {
        candidate
    } else {
        REDACTED_PATH.to_owned()
    }
}

fn is_safe_relative_path(path: &str) -> bool {
    !path.is_empty()
        && !path.starts_with('/')
        && !path.starts_with('\\')
        && !path.contains(':')
        && path.split('/').all(|part| !matches!(part, "" | "." | ".."))
}

fn sanitize_root_identity(root_identity: &str) -> &str {
    if !root_identity.is_empty()
        && !root_identity.contains(['/', '\\', ':'])
        && !matches!(root_identity, "." | "..")
    {
        root_identity
    } else {
        REDACTED_ROOT
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
            root_identity: "workspace-root".into(),
            root_relative_path: path.into(),
            kind: "file".into(),
            bytes: bytes.to_vec(),
        }
    }

    /// @vtest.id TEST-EXEC-STATE-DIFF-AVAILABILITY-AND-SCHEMA
    /// @vtest.covers VO-EXEC-STATE-PREPOST-STABILITY
    /// @vtest.target crates/vtest-exec/src/state_diff.rs::summarize_execution_state_change
    /// @vtest.intent summarize_execution_state_change reports snapshot availability mismatches, schema, runner_kind, completeness, and non-workspace root identities deterministically
    #[test]
    fn reports_availability_schema_runner_completeness_and_non_workspace_roots() {
        let root = Path::new("/tmp/project");
        let base = draft_with_inputs(vec![input("Cargo.toml", b"a")]);

        let lost = summarize_execution_state_change(root, Some(&base), None, None, None);
        assert!(
            lost.contains("before: available, after: unavailable"),
            "{lost}"
        );

        let gained = summarize_execution_state_change(root, None, Some(&base), None, None);
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
        ]);
        modified.schema_version = "2".into();
        modified.runner_kind = "cargo-llvm-cov".into();
        modified.complete = false;

        let summary =
            summarize_execution_state_change(root, Some(&base), Some(&modified), None, None);
        assert!(summary.contains("added: dep-a:src/lib.rs"), "{summary}");
        assert!(
            summary.contains("changed: schema, runner_kind, completeness"),
            "{summary}"
        );
    }
}
