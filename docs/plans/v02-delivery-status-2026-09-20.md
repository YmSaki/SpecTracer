# v0.2 delivery status — 2026-09-20

This is a working status note for the adapter-separation program. The canonical
requirements and specifications take precedence over this note.

## Branches and baseline

- `main` is the v0.1.0-alpha.1 release line. It is not the development base.
- `develop` is the integration base at `198d645` (merged runner and coverage
  adapter capability slices).
- `feature/adapter-registry` at `530b219` is the latest feature PR (#56) against
  `develop`. Its last GitHub Actions run passed formatting, workspace tests,
  and clippy, but failed `vtest doctor`.
- `feature/nested-run-test-isolation` at `fda4847` is a separate, unverified
  follow-up on top of `develop`; it is newer by five seconds, but is not the
  latest feature implementation.
- Nine remote feature/spec heads already reachable from `develop` were removed
  on 2026-09-20. Their commits remain in `develop` history. The two unmerged
  feature heads remain available.

## Scope

The public roadmap marks v0.1 M1–M9 complete. The active implementation goal is
v0.2 adapter separation, W0–W8 in the adapter plan. PR #56 is one slice of
that program, not proof that all W0–W8 acceptance criteria are complete.

The current branch has a neutral `ExecutionDescriptor`, v2 config parsing,
Rust runner and coverage adapter capabilities, a registry, a static-analysis
capability, and a synthetic adapter fixture. Some planned boundaries remain
unfinished: `vtest-scan` still directly uses `syn` for Structured Test
Operations, and the synthetic acceptance matrix in plan §9 is not yet complete.

Two open issues affect the final contract: #35 documents canonical Rust
Locator collisions between distinct constructs, and #33 requests an explicit
runner-to-core DTO for Execution State hash input. Their behavior and normative
implications must be tested against the current branch before selecting a fix.
Issue #32's latest discussion says its original multi-target question is no
longer actionable; it should not drive speculative framework work.

## Work sequence

1. Reproduce and repair PR #56's `doctor` failure. Run focused checks and the
   workspace gate. Keep the repair on `feature/adapter-registry`.
2. Validate the nested-run test isolation change independently. Integrate it
   only if the tests reproduce the flaky condition and the change is compatible
   with the feature branch.
3. Finish the remaining W3/W4/W7 capability boundaries and acceptance cases in
   separate, scoped feature branches based on `develop`, respecting the
   dependency order and exclusive file ownership in plan §8.
4. Run `verify-change`, `architecture-check`, and the W8 `release-check` after
   the architecture and acceptance work. Do not mark v0.2 complete while any
   required criterion remains non-PASS.
5. Merge accepted feature work into `develop`. Prepare a release branch only
   after the full gate; merge that release to both `main` and `develop`.

No canonical Audit, Evidence, Approval, or `.verify/cache/` data should be
rewritten to make a gate pass. Specification tensions disclosed in PR #56,
including the simultaneous missing-runner/missing-coverage priority, require
an owner ruling before a behavioral contract is declared final.
