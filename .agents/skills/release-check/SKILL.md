---
name: release-check
description: Assess SpecTracer milestone or release readiness against detailed-design annex B M1-M9 and the full fail-closed verification contract. Use before marking a milestone complete, publishing a plugin or binary, enabling the vtest MCP server, merging a broad integration, or claiming the project is release-ready.
---

# Release Check

## Select the gate

Identify the highest milestone claimed complete. Read every acceptance criterion from M1 through that milestone; later milestones may remain `NOT_CHECKED`, but no criterion inside the claimed range may be skipped.

## Run baseline validation

Run:

```text
python scripts/release_check.py --root <repo-root> --milestone M<n>
```

The script checks expected crate/file presence and runs formatting, workspace tests, clippy, and `vtest doctor` when available. Treat it as a deterministic baseline, not as a substitute for milestone-specific assertions.

## Verify milestone evidence

- M1: scanner entities, filters/targets, E-SCAN-002..010, W-SCAN-101, JSON envelope, exit codes.
- M2: record writes, approval invalidation, VO product expansion, SPEC staleness.
- M3: DA-001..006, W-DA-101, and conservative UNKNOWN behavior.
- M4: per-Test Evidence, stale target hashes, build-failure non-recording.
- M5: all three bundles, E-AUDIT-001..006, reason and hash rejection, staleness.
- M6: all 11 items individually force NG; limited scope retains `NOT_CHECKED`; tree output matches the interface.
- M7: target hit PASS, missed target FAIL, unavailable llvm-cov `NOT_CHECKED`.
- M8: Form validation and candidates, recognized generated tests, one-Test edit boundary, idempotent desired state.
- M9: every MCP tool matches CLI JSON behavior and the full reference flow succeeds.

Use fixture-based automated tests for each item. Cite test names and commands.

## Check distribution

- Validate all project Skills with `quick_validate.py`.
- Validate `plugins/spectracer-development` with `validate_plugin.py`.
- Confirm project and plugin Skill copies are byte-identical.
- Keep project MCP disabled before M9. After M9, confirm `vtest mcp` starts over stdio before enabling it.
- Confirm hooks are reviewed/trusted and their scripts pass syntax tests.

## Decide

Return `READY` only when all criteria in the claimed range pass. Otherwise return `NOT_READY` with each `FAIL`, `NOT_CHECKED`, `NOT_EXECUTED`, `STALE`, or `UNKNOWN` item and the exact next gate. Do not collapse missing evidence into an overall pass.
