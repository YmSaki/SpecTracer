---
name: release-check
description: Assess SpecTracer release readiness against current canonical conformance, engineering gates, release identity, packaging, and artifact evidence, failing closed when evidence is missing.
---

# Release check

Assess readiness read-only: inspect files, tags, branches, and artifacts as they are, because an assessment that changes release state stops being an assessment. When modifying files, creating or moving tags, pushing branches, publishing artifacts, or changing release state is wanted, ask for it as a separate explicit request.

## Declare the release review contract

Before inspection, state:

- target: candidate revision, branch, version, tag if any, and intended artifact;
- purpose: the next release step this decision supports;
- fixed axes: current canonical conformance, engineering gate, release identity and Git eligibility, packaging/workflow evidence, and artifact smoke evidence;
- exclusions: platforms or publication steps not observable here.

Keep the axes as declared throughout, because a changed axis moves the standard after the result is seen. When an axis proves inadequate, report the defect separately and leave the new concern unscored until a separately declared review covers it. Reject only for failures inside a declared axis, and report every material failure inside an axis; the number of findings is not a reason to suppress one.

Report a material canonical or safety violation discovered outside the declared axes as an out-of-scope blocker, or re-scope explicitly, because silence or a `READY` would hide it. While it is unresolved, return a decision other than `READY`. Score it as an in-axis finding only after the review contract is explicitly re-scoped.

## Authorities

Use the current v0.1 requirements, basic specification, detailed design, interface specification, and Annex C for product conformance. Use `DEVELOPMENT.md` as the separate, sole authority for Git flow, release tags, and packaging process. Use `TESTING.md` and workflow files as executable evidence checked against the canonical design.

Take an implementation schedule as process material. Acceptance evidence establishes current canonical conformance only when its assertions are explicitly mapped to current canonical clauses and observables. Use historical or superseded acceptance evidence for regression checks, and add current mapped evidence to establish current conformance.

## Readiness axes

### Current canonical conformance

Require current implementation plus current Annex C evidence for:

- exactly four checks: `chain_integrity`, `orphan_detection`, `target_binding`, `oracle_presence`;
- exactly five states: `PASS`, `FAIL`, `MISMATCH`, `NO_EVIDENCE`, `UNKNOWN`;
- diagnostic labels `MISSING`, `NOT_EXECUTED`, `NOT_CHECKED`, and `STALE` attached to a state and kept outside the state type and aggregation ordering;
- generic `document` records and `derives_from` links;
- independent Judgment and Approval domains, neither changing verification state;
- canonical Approval CLI commands `vtest approval create`, `vtest approval withdraw`, and `vtest approval show`, with `vtest vo approve` only an alias;
- limited scope keeping work outside the requested scope from counting as `PASS`: omitted checks represented in the aggregation tree remain `NO_EVIDENCE` with `NOT_CHECKED`, while entity scope outside the request remains explicitly unverified through the canonical scope representation, outside the aggregate;
- gate satisfaction separate from verification state, including `--gate` exit and JSON semantics;
- compatibility readers passing through current identity, completeness, freshness, scope, and aggregation invariants;
- `UNKNOWN` when deterministic analysis cannot establish `PASS` or `FAIL`.

Close this axis with evidence whose assertions are mapped to the current canonical contract; a green suite, build, `doctor`, or historical ledger entry counts only when mapped. Required behavior without current evidence keeps the decision below `READY`.

### Engineering gate

Run:

```text
cargo fmt --all -- --check
cargo test --workspace --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo run --quiet -p vtest-cli --locked -- doctor
```

Also run focused Annex C tests required by the conformance map. Record unavailable tooling and skipped required tests as insufficient evidence, and report limited execution as limited.

### Release identity and Git eligibility

Using read-only Git inspection, verify the candidate follows `DEVELOPMENT.md`: branch role, target version, release-tag syntax, tag immutability expectations, and required reachability from `main`. Confirm workspace/package version and intended tag agree. Leave tags as they are.

### Packaging and artifact evidence

Inspect the release workflow named by `DEVELOPMENT.md` and verify that the candidate preserves its eligibility and packaging contract. For supplied or local artifacts, verify checksums, archive contents, executable help, and a read-only smoke command in an isolated project. Report unexecuted target platforms as not executed.

## Interpret gates correctly

When using `--gate`, read aggregate verification and gate fields independently. Exit `0` or `ok: true` may mean the configured gate is satisfied, including a gate requiring a non-`PASS` value; read the aggregate state and the other axes to establish verification `PASS` and release readiness, because a satisfied gate alone does not establish them.

Keep product states, diagnostics, command exit status, Approval, and this Skill's release decision separate.

## Decision

Report evidence per axis, exact failing commands or missing observables, and excluded platforms. End with exactly one decision:

- `READY`: every axis has current evidence and the candidate can move to the next release step under `DEVELOPMENT.md`;
- `NOT_READY`: a canonical obligation, required gate, Git eligibility rule, or packaging contract is violated;
- `INSUFFICIENT_EVIDENCE`: no violation is established, but required current conformance, platform, or artifact evidence is unavailable.

Apply the standard of correct responsibility transfer; perfection is a different question. Infer `READY` from current evidence on every axis; historical or superseded acceptance evidence and a green repository gate alone leave the decision below `READY`.
