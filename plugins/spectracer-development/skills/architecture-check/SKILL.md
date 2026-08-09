---
name: architecture-check
description: Review SpecTracer changes for conformance to the v0.1 requirements, basic specification, and detailed design. Use for new crates, dependency changes, record schemas, scanner or audit logic, test execution and Evidence, aggregation, CLI/MCP interfaces, or any change that could affect fail-closed behavior or source-of-truth ownership.
---

# Architecture Check

## Trace authority

1. Read the relevant requirement and basic-specification sections.
2. Read the corresponding detailed-design section and annex acceptance criteria.
3. If documents conflict, treat the basic specification as authoritative and report the conflict without choosing a repair.

## Check invariants

Evaluate each applicable invariant:

- `vtest-cli / vtest-mcp -> vtest-verify / vtest-exec / vtest-audit / vtest-scan -> vtest-store -> vtest-model` remains one-directional.
- Declarations, implementation, and facts remain separate canonical layers.
- Covers and target relations are derived from Test annotations, not duplicated in external records.
- Indexes, graphs, aggregate reports, bundles, and raw logs are derived and rebuildable.
- SPEC/REQ/VO use one entity per file; append-only facts use ULID files; Relation is immutable.
- Every approval, audit, and Evidence result is bound to the complete current subject set capable of changing its judgment, not merely the most obvious entity hashes.
- Static Audit freshness includes every helper or source fragment actually consulted; incomplete analysis-input closure cannot produce PASS.
- Evidence freshness includes exact HEAD revision and a complete Execution State subject for runner, toolchain, execution-affecting config, repository inputs, and local dependencies; Test/target hash equality alone is insufficient.
- `impl-consistency` freshness includes the complete current Specification subject closure used by the judgment, including in limited scope.
- Every aggregate is fail-closed; narrowing scope does not rewrite out-of-scope values to PASS.
- Deterministic rules use UNKNOWN at analysis limits.
- Structured edits are desired-state operations and preserve the one-Test boundary.
- CLI and MCP share core logic, JSON envelopes, validation errors, and non-interactive behavior.
- v0.1 does not grow into GUI, repair-policy decisions, cross-specification auditing, or general development-process management.

## Review interfaces

- Verify error codes and exit codes remain consistent with detailed design §17.
- Verify MCP write tools expose only structured, validated operations.
- Verify `cargo-llvm-cov` absence produces `NOT_CHECKED`, not PASS.
- Verify dirty revisions require a matching complete Execution State subject, changed HEAD revisions are stale, and unknown revisions or incomplete snapshots cannot pass Evidence validity.

## Output

List findings first, ordered by risk. For each finding include:

- file and symbol;
- violated invariant and document section;
- how the violation could incorrectly change a check value or observable interface;
- whether the result is certain or `UNKNOWN`.

Do not prescribe which of specification, test, or implementation must change unless the parent explicitly requests design options.
