//! Language-neutral validation and hashing of adapter Execution State DTOs.
use std::collections::BTreeSet;
use vtest_model::{
    encode_nested_fields, AdapterId, ExecutionState, ExecutionStateDraft, FieldValue,
    SubjectDomain, SubjectHashInput,
};

/// DES-097–101: core owns the hash and rejects malformed/duplicate identities.
pub fn materialize_execution_state(
    adapter: &AdapterId,
    draft: &ExecutionStateDraft,
) -> ExecutionState {
    let schema = draft.schema();
    let incomplete = || ExecutionState {
        schema: schema.clone(),
        complete: false,
        hash: None,
    };
    if !draft.complete
        || draft.head_revision.as_deref().is_none_or(str::is_empty)
        || draft.schema_id.is_empty()
        || draft.schema_version.is_empty()
        || draft.runner_kind.is_empty()
        || draft.toolchain_identity.is_empty()
    {
        return incomplete();
    }
    let mut identities = BTreeSet::new();
    let mut entries: Vec<_> = draft.inputs.iter().collect();
    for entry in &entries {
        if !logical_path(&entry.root_identity)
            || !logical_path(&entry.root_relative_path)
            || entry.kind.is_empty()
            || !identities.insert((&entry.root_identity, &entry.root_relative_path))
        {
            return incomplete();
        }
    }
    entries.sort_by_key(|e| (&e.root_identity, &e.root_relative_path));
    let manifest = FieldValue::Ordered(
        entries
            .iter()
            .map(|e| {
                encode_nested_fields([
                    ("root", FieldValue::exact_bytes(e.root_identity.as_bytes())),
                    (
                        "path",
                        FieldValue::exact_bytes(e.root_relative_path.as_bytes()),
                    ),
                    ("kind", FieldValue::exact_bytes(e.kind.as_bytes())),
                    ("bytes", FieldValue::exact_bytes(e.bytes.as_slice())),
                ])
            })
            .collect(),
    );
    let hash = SubjectHashInput::new(SubjectDomain::ExecutionState)
        .field("adapter", FieldValue::text_fragment(adapter.as_str()))
        .field("schema_id", FieldValue::text_fragment(&draft.schema_id))
        .field(
            "schema_version",
            FieldValue::text_fragment(&draft.schema_version),
        )
        .field(
            "head_revision",
            FieldValue::text_fragment(draft.head_revision.as_deref().unwrap()),
        )
        .field("runner_kind", FieldValue::text_fragment(&draft.runner_kind))
        .field(
            "invocation",
            FieldValue::exact_bytes(draft.invocation.to_string().into_bytes()),
        )
        .field(
            "toolchain",
            FieldValue::text_fragment(&draft.toolchain_identity),
        )
        .field(
            "adapter_config",
            FieldValue::exact_bytes(draft.effective_config.to_string().into_bytes()),
        )
        .field("manifest", manifest)
        .finish();
    ExecutionState {
        schema,
        complete: true,
        hash: Some(hash),
    }
}

fn logical_path(path: &str) -> bool {
    !path.is_empty()
        && !path.contains('\\')
        && !path.contains(':')
        && path.split('/').all(|part| !matches!(part, "" | "." | ".."))
}

#[cfg(test)]
mod tests {
    use super::*;
    use vtest_model::ExecutionInputDraft;

    fn draft() -> ExecutionStateDraft {
        ExecutionStateDraft {
            schema_id: "synthetic-state".into(),
            schema_version: "1".into(),
            complete: true,
            head_revision: Some("revision".into()),
            runner_kind: "synthetic".into(),
            invocation: serde_json::json!({"args": ["a", "b"]}),
            toolchain_identity: "toolchain".into(),
            effective_config: serde_json::Value::Null,
            inputs: vec![ExecutionInputDraft {
                root_identity: "workspace".into(),
                root_relative_path: "src/input".into(),
                kind: "file".into(),
                bytes: vec![0, 255, 13, 10],
            }],
        }
    }

    /// DES-097–101: adapter input bytes, every subject field and projections
    /// are bound by core, including type and list order.
    /// @vtest.id TEST-EXEC-STATE-DTO-BINDING
    /// @vtest.covers VO-EXEC-STATE-DTO-VALIDATION
    /// @vtest.target crates/vtest-store/src/execution_state.rs::materialize_execution_state
    /// @vtest.intent all execution inputs are bound and manifest order is canonical
    #[test]
    fn all_execution_inputs_are_bound_and_manifest_order_is_canonical() {
        let adapter = AdapterId::new("synthetic");
        let original = draft();
        let baseline = materialize_execution_state(&adapter, &original);
        assert!(baseline.complete);
        let mut variants = Vec::new();
        let mut d = original.clone();
        d.schema_version = "2".into();
        variants.push(d);
        let mut d = original.clone();
        d.head_revision = Some("other".into());
        variants.push(d);
        let mut d = original.clone();
        d.runner_kind = "other".into();
        variants.push(d);
        let mut d = original.clone();
        d.toolchain_identity = "other".into();
        variants.push(d);
        let mut d = original.clone();
        d.invocation = serde_json::json!({"args": ["b", "a"]});
        variants.push(d);
        let mut d = original.clone();
        d.effective_config = serde_json::json!("null");
        variants.push(d);
        let mut d = original.clone();
        d.inputs[0].bytes[1] = 254;
        variants.push(d);
        for variant in variants {
            assert_ne!(
                baseline.hash,
                materialize_execution_state(&adapter, &variant).hash
            );
        }
        assert_ne!(
            baseline.hash,
            materialize_execution_state(&AdapterId::new("other"), &original).hash
        );
        let mut two = original.clone();
        let mut extra = two.inputs[0].clone();
        extra.root_relative_path = "src/second".into();
        two.inputs.push(extra);
        let first = materialize_execution_state(&adapter, &two);
        two.inputs.reverse();
        assert_eq!(first, materialize_execution_state(&adapter, &two));
    }

    /// DES-101: malformed or ambiguous adapter identities cannot acquire a
    /// complete core subject even when the adapter claims completeness.
    /// @vtest.id TEST-EXEC-STATE-DTO-VALIDATION
    /// @vtest.covers VO-EXEC-STATE-DTO-VALIDATION
    /// @vtest.target crates/vtest-store/src/execution_state.rs::materialize_execution_state
    /// @vtest.intent incomplete and invalid manifests never produce a hash
    #[test]
    fn incomplete_and_invalid_manifests_never_produce_a_hash() {
        let adapter = AdapterId::new("synthetic");
        for path in [
            "/absolute",
            "../outside",
            "src/../outside",
            "C:/input",
            "src\\input",
            "",
        ] {
            let mut d = draft();
            d.inputs[0].root_relative_path = path.into();
            let state = materialize_execution_state(&adapter, &d);
            assert!(!state.complete && state.hash.is_none(), "{path}");
        }
        let mut d = draft();
        d.inputs.push(d.inputs[0].clone());
        assert!(!materialize_execution_state(&adapter, &d).complete);
        let mut d = draft();
        d.complete = false;
        assert!(materialize_execution_state(&adapter, &d).hash.is_none());
    }
}
