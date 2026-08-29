---
name: verify-change
description: Verify a SpecTracer change with current canonical traceability, focused and repository checks, and fail-closed evidence without treating historical tests as conformance.
---

# Verify change

Determine whether a change has enough current evidence for review or integration. Verification is read-only unless fixes are separately requested.

## Declare scope and axes

Before running checks, state:

- target: exact diff, files, commit range, or worktree change;
- purpose: intended handoff;
- fixed axes: canonical conformance, behavioral evidence, repository quality gate, and change containment, omitting only axes that demonstrably do not apply;
- exclusions: untested platforms, unavailable tools, unrelated baseline gaps, and unevaluated paths.

Do not add, remove, or change axes after starting. Report an inadequate axis separately. Do not reject on an excluded concern, and report all material failures within an axis.

A material canonical or safety violation discovered outside the declared axes must be reported as an out-of-scope blocker or trigger an explicit re-scope. Do not silently ignore it or return `READY` while it remains unresolved. Do not score it as an in-axis finding unless the review contract is explicitly re-scoped.

## Build the contract map

Read the applicable requirements, basic specification, detailed design, interface specification, and Annex C clauses. Use `DEVELOPMENT.md` and `TESTING.md` for executable repository procedures, not to redefine product semantics.

For each changed behavior record the governing sections, affected domain/wire/compatibility/lifecycle paths, required observable, and command or test supplying that evidence.

Use only the current model:

- checks: `chain_integrity`, `orphan_detection`, `target_binding`, `oracle_presence`;
- states: `PASS`, `FAIL`, `MISMATCH`, `NO_EVIDENCE`, `UNKNOWN`;
- diagnostics: `MISSING`, `NOT_EXECUTED`, `NOT_CHECKED`, `STALE`; these are diagnostic labels, not verification states.

Keep state and diagnostic separate. Keep Judgment, Approval, aggregate OK/NG, and gate satisfaction separate from verification state. Judgment or Approval never changes a verification state. With `--gate`, exit `0` or `ok: true` can express gate satisfaction and is not proof that aggregate verification is `PASS`.

## Collect evidence

1. Inspect the current diff and reachable call sites, including changed schemas, readers, writers, adapters, aggregation, CLI, MCP, fixtures, and tests.
2. Run focused tests for mapped Annex C observations. Claim no clause that lacks a current fixture and observable assertion.
3. Run the repository gate from current development guidance:

   ```text
   cargo fmt --all -- --check
   cargo test --workspace --locked
   cargo clippy --workspace --all-targets --locked -- -D warnings
   cargo run --quiet -p vtest-cli --locked -- doctor
   ```

   Record unavailable required tools or commands as not executed and remain fail-closed. Do not silently substitute tools.
4. Exercise write-capable CLI or MCP flows only in an isolated fixture project. Verify successful observables and failure atomicity where rejection must not mutate state.
5. Exercise compatibility paths with explicit compatibility-shaped input. Confirm normalization reaches the current canonical model and the same identity, completeness, freshness, scope, and aggregation rules.
6. Check determinism with equivalent input ordering or repeated runs where required. Do not turn analysis limits into guessed results.
7. Run `git diff --check` and confirm that the diff is contained to the intended change surface.

Acceptance evidence establishes current canonical conformance only when its assertions are explicitly mapped to current canonical clauses and observables. Historical or superseded acceptance evidence may support regression checks, but cannot by itself establish current conformance. A green repository gate or `doctor` is supporting evidence only when its assertions match the mapped current contract.

Limited verification must name omissions and never treat work outside the requested scope as `PASS`. Omitted checks represented in the aggregation tree remain `NO_EVIDENCE` with `NOT_CHECKED`. Entity scope outside the request remains explicitly unverified through the canonical scope representation. Missing current conformance evidence prevents a complete handoff even when executed commands pass.

## Report and decide

Provide an evidence matrix with canonical clause, observable, command/test, result, and residual gap. Separate change failures, pre-existing implementation gaps, unavailable evidence, and excluded paths.

End with exactly one handoff decision:

- `READY`: every applicable current obligation changed or relied upon has current evidence and the repository gate passes;
- `NOT_READY`: an applicable obligation is violated, a required command fails, or a compatibility/scope path bypasses an invariant;
- `INSUFFICIENT_EVIDENCE`: no violation is proven, but required current evidence is missing or could not be executed.

This decision is not a verification state. Never invent implemented or verified status for behavior not demonstrated by current code and evidence.
