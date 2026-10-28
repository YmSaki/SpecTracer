---
name: verify-change
description: Verify a SpecTracer change with current canonical traceability, focused and repository checks, and fail-closed evidence, counting historical tests as regression evidence only.
---

# Verify change

Determine whether a change has enough current evidence for review or integration. Verify read-only; when fixes are wanted, ask for them as a separate request.

## Declare scope and axes

Before running checks, state:

- target: exact diff, files, commit range, or worktree change;
- purpose: intended handoff;
- fixed axes: canonical conformance, behavioral evidence, repository quality gate, and change containment, omitting only axes that demonstrably do not apply;
- exclusions: untested platforms, unavailable tools, unrelated baseline gaps, and unevaluated paths.

Keep the axes as declared throughout, because a changed axis moves the standard after the result is seen. When an axis proves inadequate, report the defect separately and leave the new concern unscored until a separately declared review covers it. Reject only for failures inside a declared axis, and report every material failure inside an axis; the number of findings is not a reason to suppress one.

Report a material canonical or safety violation discovered outside the declared axes as an out-of-scope blocker, or re-scope explicitly, because silence or a `READY` would hide it. While it is unresolved, return a decision other than `READY`. Score it as an in-axis finding only after the review contract is explicitly re-scoped.

## Build the contract map

Read the applicable requirements, basic specification, detailed design, interface specification, and Annex C clauses. Use `DEVELOPMENT.md` and `TESTING.md` for executable repository procedures; product semantics come from the canonical clauses.

For each changed behavior record the governing sections, affected domain/wire/compatibility/lifecycle paths, required observable, and command or test supplying that evidence.

Use the current model:

- checks: `chain_integrity`, `orphan_detection`, `target_binding`, `oracle_presence`;
- states: `PASS`, `FAIL`, `MISMATCH`, `NO_EVIDENCE`, `UNKNOWN`;
- diagnostics: `MISSING`, `NOT_EXECUTED`, `NOT_CHECKED`, `STALE`; these are diagnostic labels attached to a state.

Keep state and diagnostic separate. Keep Judgment, Approval, aggregate OK/NG, and gate satisfaction separate from verification state. Judgment or Approval leaves a verification state as it was. With `--gate`, exit `0` or `ok: true` can express gate satisfaction; read the aggregate state to learn whether verification is `PASS`.

## Collect evidence

1. Inspect the current diff and reachable call sites, including changed schemas, readers, writers, adapters, aggregation, CLI, MCP, fixtures, and tests.
2. Run focused tests for mapped Annex C observations. Claim a clause only when a current fixture and an observable assertion cover it.
3. Run the repository gate from current development guidance:

   ```text
   cargo fmt --all -- --check
   cargo test --workspace --locked
   cargo clippy --workspace --all-targets --locked -- -D warnings
   cargo run --quiet -p vtest-cli --locked -- doctor
   ```

   Record unavailable required tools or commands as not executed and remain fail-closed. State a substitute tool explicitly when you use one, because a different tool may check something else.
4. Exercise write-capable CLI or MCP flows in an isolated fixture project, because a flow run against the repository writes records into canonical data. Verify successful observables and failure atomicity where rejection must not mutate state.
5. Exercise compatibility paths with explicit compatibility-shaped input. Confirm normalization reaches the current canonical model and the same identity, completeness, freshness, scope, and aggregation rules.
6. Check determinism with equivalent input ordering or repeated runs where required. Report an analysis limit as a limit (`UNKNOWN`), because a guessed result looks like evidence.
7. Run `git diff --check` and confirm that the diff is contained to the intended change surface.

Acceptance evidence establishes current canonical conformance only when its assertions are explicitly mapped to current canonical clauses and observables. Use historical or superseded acceptance evidence for regression checks, and add current mapped evidence to establish current conformance. A green repository gate or `doctor` is supporting evidence only when its assertions match the mapped current contract.

Name the omissions of a limited verification, and treat work outside the requested scope as `NO_EVIDENCE` with `NOT_CHECKED`. Omitted checks represented in the aggregation tree remain `NO_EVIDENCE` with `NOT_CHECKED`. Entity scope outside the request remains explicitly unverified through the canonical scope representation. Missing current conformance evidence leaves the handoff incomplete even when executed commands pass.

## Report and decide

Provide an evidence matrix with canonical clause, observable, command/test, result, and residual gap. Separate change failures, pre-existing implementation gaps, unavailable evidence, and excluded paths.

End with exactly one handoff decision:

- `READY`: every applicable current obligation changed or relied upon has current evidence and the repository gate passes;
- `NOT_READY`: an applicable obligation is violated, a required command fails, or a compatibility/scope path bypasses an invariant;
- `INSUFFICIENT_EVIDENCE`: no violation is proven, but required current evidence is missing or could not be executed.

Keep this decision separate from verification states. Describe behavior as implemented or verified only where current code and evidence demonstrate it, because an invented status is the false `PASS` this tool exists to stop.
