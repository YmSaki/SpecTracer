---
name: verify-change
description: Validate a SpecTracer implementation or test change with fail-closed, traceable checks. Use after modifying Rust code, tests, fixtures, `.verify` schemas or records, CLI/MCP behavior, static or semantic audits, execution Evidence, aggregation, or report output; also use before handing a change to review.
---

# Verify Change

## Establish scope

1. Identify the active milestone in non-normative annex B and the corresponding normative acceptance criteria in annex C §18.
2. Trace changed behavior to the requirement, basic-specification guarantee, and detailed-design section.
3. Inspect the diff and list affected crates, commands, record types, check items, and fixtures.
4. Treat an unavailable check as `NOT_CHECKED` or `NOT_EXECUTED`; never infer `PASS`.

## Review the change

- Preserve the crate dependency direction in `AGENTS.md`.
- Confirm canonical facts are stored once and derived data remains rebuildable.
- Confirm approval, audit, and Evidence hashes become stale after any member or value in their complete judgment-input subject sets changes.
- For static audit changes, mutate only a consulted helper/source fragment and confirm the old Audit becomes stale; incomplete analysis-input closure must remain UNKNOWN.
- For execution changes, mutate only a target-external helper or local dependency and confirm the old Evidence becomes stale through its Execution State subject; changed HEAD, missing snapshot, incomplete snapshot, and unknown revision must not pass.
- For `impl-consistency`, mutate only a referenced Specification and confirm the old Audit becomes stale even under `--items impl_consistency`.
- For a Structured Test Operation, prove the edit changes exactly one Test extended range and leaves other Test hashes unchanged.
- For deterministic analysis, return `UNKNOWN` instead of `FAIL` when the rule cannot prove a violation.
- For semantic results, require a non-empty claim and at least one concrete basis reference.
- Confirm CLI and MCP use the same core behavior and JSON envelope.

## Run deterministic gates

Run `python scripts/verify_change.py --root <repo-root>` from this skill directory. Review the JSON rather than relying only on its exit code.

The script runs available baseline checks:

```text
cargo fmt --all -- --check
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo run --quiet -p vtest-cli -- doctor   # once vtest-cli exists
```

Then run the feature-specific path when available:

```text
vtest audit static --test TEST-X
vtest audit bundle --kind test-semantic --test TEST-X
vtest audit submit --file result.json
vtest run --test TEST-X
vtest verify --test TEST-X
```

Use full target-execution measurement for a full verification. A fast run leaves `target_execution` as `NOT_CHECKED`.

## Report

Return:

- requirement and design references;
- commands and exit codes;
- individual PASS/non-PASS states;
- stale or unavailable evidence;
- residual risk and the next required gate.

Do not claim “fully verified” unless all 12 full-scope items are `PASS`.
