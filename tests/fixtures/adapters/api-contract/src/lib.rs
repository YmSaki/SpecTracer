#![allow(dead_code)]

use vtest_adapter_api::{
    DiscoveredTestDraft, DiscoveryBatch, ExecutionInputDraft, ExecutionStateDraft,
    ManagedTestDraft, ManagedTestDraftLink, SourceFragment, SourceTargetDraft,
    StaticAnalysisClosureDraft, StaticAuditConfigDraft,
};
use vtest_model::{ExecutionDescriptor, SourceLocation, TargetRef, TestEntity};

fn source_fragment_fields(value: SourceFragment) {
    let SourceFragment { location, bytes } = value;
    let _: SourceLocation = location;
    let _: Vec<u8> = bytes;
}

fn managed_test_draft_fields(value: ManagedTestDraft) {
    let ManagedTestDraft {
        id: _,
        covers: _,
        targets,
        intent: _,
        input: _,
        expect: _,
        kind: _,
        cases: _,
        related: _,
        execution,
    } = value;
    let _: Vec<TargetRef> = targets;
    let _: ExecutionDescriptor = execution;
}

fn discovered_test_draft_fields(value: DiscoveredTestDraft) {
    let DiscoveredTestDraft {
        adapter: _,
        location,
        construct,
        metadata_sources,
        managed,
    } = value;
    let _: SourceLocation = location;
    let _: SourceFragment = construct;
    let _: Vec<SourceFragment> = metadata_sources;
    let _: ManagedTestDraftLink = managed;
}

fn source_target_draft_fields(value: SourceTargetDraft) {
    let SourceTargetDraft {
        target,
        location,
        construct,
    } = value;
    let _: TargetRef = target;
    let _: SourceLocation = location;
    let _: SourceFragment = construct;
}

fn discovery_batch_fields(value: DiscoveryBatch) {
    let DiscoveryBatch {
        adapter: _,
        completeness: _,
        discovered_tests,
        source_targets,
        diagnostics: _,
    } = value;
    let _: Vec<DiscoveredTestDraft> = discovered_tests;
    let _: Vec<SourceTargetDraft> = source_targets;
}

fn static_subject_draft_fields(
    closure: StaticAnalysisClosureDraft,
    config: StaticAuditConfigDraft,
) {
    let StaticAnalysisClosureDraft { complete, sources } = closure;
    let _: bool = complete;
    let _: Vec<SourceFragment> = sources;
    let StaticAuditConfigDraft {
        rule_set_id: _,
        rule_set_version: _,
        effective_config: _,
    } = config;
}

fn execution_state_draft_fields(value: ExecutionStateDraft) {
    let ExecutionStateDraft {
        schema_id: _,
        schema_version: _,
        complete,
        head_revision: _,
        runner_kind: _,
        invocation: _,
        toolchain_identity: _,
        effective_config: _,
        inputs,
    } = value;
    let _: bool = complete;
    let _: Vec<ExecutionInputDraft> = inputs;
}

fn neutral_test_entity_fields(value: TestEntity) {
    let TestEntity {
        id: _,
        covers: _,
        targets,
        intent: _,
        input: _,
        expect: _,
        kind: _,
        cases: _,
        related: _,
        location,
        content_hash: _,
        execution,
    } = value;
    let _: Vec<TargetRef> = targets;
    let _: SourceLocation = location;
    let _: ExecutionDescriptor = execution;
}
