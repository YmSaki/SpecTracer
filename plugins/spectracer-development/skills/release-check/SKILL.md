---
name: release-check
description: Assess SpecTracer release readiness against current canonical conformance, engineering gates, release identity, packaging, and artifact evidence, failing closed when evidence is missing.
---

# Release check

Assess readiness without modifying files, creating or moving tags, pushing branches, publishing artifacts, or changing release state. Those actions require a separate explicit request.

## Declare the release review contract

Before inspection, state:

- target: candidate revision, branch, version, tag if any, and intended artifact;
- purpose: the next release step this decision supports;
- fixed axes: current canonical conformance, engineering gate, release identity and Git eligibility, packaging/workflow evidence, and artifact smoke evidence;
- exclusions: platforms or publication steps not observable here.

Do not add, remove, or change axes during review. Report an inadequate axis separately. Do not reject for an excluded concern, and report every material failure inside an axis.

A material canonical or safety violation discovered outside the declared axes must be reported as an out-of-scope blocker or trigger an explicit re-scope. Do not silently ignore it or return `READY` while it remains unresolved. Do not score it as an in-axis finding unless the review contract is explicitly re-scoped.

## Authorities

Use the current v0.1 requirements, basic specification, detailed design, interface specification, and Annex C for product conformance. Use `DEVELOPMENT.md` as the separate, sole authority for Git flow, release tags, and packaging process. Use `TESTING.md` and workflow files as executable evidence, not authority over the canonical design.

An implementation schedule is non-normative. Acceptance evidence establishes current canonical conformance only when its assertions are explicitly mapped to current canonical clauses and observables. Historical or superseded acceptance evidence may support regression checks, but cannot by itself establish current conformance.

## Readiness axes

### Current canonical conformance

Require current implementation plus current Annex C evidence for:

- exactly four checks: `chain_integrity`, `orphan_detection`, `target_binding`, `oracle_presence`;
- exactly five states: `PASS`, `FAIL`, `MISMATCH`, `NO_EVIDENCE`, `UNKNOWN`;
- diagnostic labels `MISSING`, `NOT_EXECUTED`, `NOT_CHECKED`, and `STALE` are not verification states and remain outside the state type and aggregation ordering;
- generic `document` records and `derives_from` links;
- independent Judgment and Approval domains, neither changing verification state;
- canonical Approval CLI commands `vtest approval create`, `vtest approval withdraw`, and `vtest approval show`, with `vtest vo approve` only an alias;
- limited scope never treating work outside the requested scope as `PASS`; omitted checks represented in the aggregation tree remain `NO_EVIDENCE` with `NOT_CHECKED`, while entity scope outside the request remains explicitly unverified through the canonical scope representation;
- gate satisfaction separate from verification state, including `--gate` exit and JSON semantics;
- compatibility readers unable to bypass current identity, completeness, freshness, scope, or aggregation invariants;
- no guessed `PASS` or `FAIL` when deterministic analysis cannot establish one.

A green suite, build, `doctor`, or historical ledger entry cannot close this axis unless its assertions are mapped to the current canonical contract. Required behavior without current evidence prevents `READY`.

### Engineering gate

Run:

```text
cargo fmt --all -- --check
cargo test --workspace --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo run --quiet -p vtest-cli --locked -- doctor
```

Also run focused Annex C tests required by the conformance map. Unavailable tooling and skipped required tests are insufficient evidence, not success. Limited execution is not complete verification.

### Release identity and Git eligibility

Using read-only Git inspection, verify the candidate follows `DEVELOPMENT.md`: branch role, target version, release-tag syntax, tag immutability expectations, and required reachability from `main`. Confirm workspace/package version and intended tag agree. Do not create or move a tag.

### Packaging and artifact evidence

Inspect the release workflow named by `DEVELOPMENT.md` and verify that the candidate preserves its eligibility and packaging contract. For supplied or local artifacts, verify checksums, archive contents, executable help, and a read-only smoke command in an isolated project. Do not claim unexecuted target platforms passed.

## Interpret gates correctly

When using `--gate`, read aggregate verification and gate fields independently. Exit `0` or `ok: true` may mean the configured gate is satisfied, including a gate requiring a non-`PASS` value; it is not standalone proof of verification `PASS` or release readiness.

Keep product states, diagnostics, command exit status, Approval, and this Skill's release decision separate.

## Decision

Report evidence per axis, exact failing commands or missing observables, and excluded platforms. End with exactly one decision:

- `READY`: every axis has current evidence and the candidate can move to the next release step under `DEVELOPMENT.md`;
- `NOT_READY`: a canonical obligation, required gate, Git eligibility rule, or packaging contract is violated;
- `INSUFFICIENT_EVIDENCE`: no violation is established, but required current conformance, platform, or artifact evidence is unavailable.

The standard is correct responsibility transfer, not perfection. Never infer `READY` from historical or superseded acceptance evidence or a green repository gate alone.
