---
name: architecture-check
description: Review SpecTracer design or implementation changes against the current v0.1 canonical architecture, compatibility invariants, interfaces, and acceptance handoff contract.
---

# Architecture check

Current canonical contracts govern this review. Produce findings and a handoff decision; keep the review read-only, because findings are evidence for the next decision; when a redesign or edit is wanted, ask for it as a separate request.

## Fix the review contract first

Before inspecting the target, state:

- target: files, diff, component, or proposal;
- purpose: the next engineering decision supported by the review;
- fixed axes selected from the list below;
- exclusions and unreviewed paths.

Available axes are:

1. authority and vertical traceability;
2. domain model and source-of-truth ownership;
3. component and adapter boundaries;
4. verification, aggregation, scope, and determinism;
5. Judgment, Approval, Evidence, and freshness lifecycles;
6. CLI, MCP, JSON, exit, and gate semantics;
7. compatibility normalization and invariant preservation;
8. acceptance observability and next-phase handoff evidence.

Select the applicable axes and keep them as declared after review begins, because a changed axis moves the standard after the result is seen. When the axes prove insufficient, report an axis defect separately and leave the new concern unscored pending a separately declared review. Reject only for issues inside a declared axis, and report every material issue within it; finding count is not a reason to suppress findings.

Report a material canonical or safety violation discovered outside the declared axes as an out-of-scope blocker, or re-scope explicitly, because silence or a `READY` would hide it. While it is unresolved, return a decision other than `READY`. Score it as an in-axis finding only after the review contract is explicitly re-scoped.

## Authority

Read the applicable sections in this order:

1. `docs/canonical/specification.json`（正本。層の順に root → request → require → spec → detailed_spec → basic_design → design。旧 md は `docs/archive/` に退役、規範として引用しない）
6. `DEVELOPMENT.md` for process and Git policy only.

Take an implementation schedule as process material and `DEVELOPMENT.md` as process and Git policy. When normative levels disagree, follow the higher product authority and cite exact sections. Take implementation behavior as evidence to check against the authority, because it records choices that may be wrong.

## Fixed model baseline

- Checks are exactly `chain_integrity`, `orphan_detection`, `target_binding`, and `oracle_presence`.
- States are exactly `PASS`, `FAIL`, `MISMATCH`, `NO_EVIDENCE`, and `UNKNOWN`.
- `MISSING`, `NOT_EXECUTED`, `NOT_CHECKED`, and `STALE` are diagnostic labels attached to a state.
- Upstream artifacts are generic `document` nodes connected by `derives_from`.
- Judgment, Approval, verification state, aggregate OK/NG, and gate satisfaction are separate concepts.
- The canonical Approval CLI is `vtest approval create`, `vtest approval withdraw`, and `vtest approval show`; `vtest vo approve` is an alias only.
- Limited scope keeps work outside the requested scope from counting as `PASS`. Omitted checks represented in the aggregation tree remain `NO_EVIDENCE` with `NOT_CHECKED`. Entity scope outside the request remains explicitly unverified through the canonical scope representation, outside the aggregate.
- When deterministic analysis cannot establish a result, report that limit (`UNKNOWN`) instead of a guessed `PASS` or `FAIL`.
- Compatibility input is effective only after current normalization, identity, completeness, freshness, and aggregation invariants pass.

## Review method

1. Inventory changed public behavior, persisted data, domain types, dependency edges, and compatibility paths. Trace each to canonical sections.
2. Check ownership and dependency direction. Core types stay language-neutral; language and runner specifics stay behind adapter capabilities. Keep derived data derived from canonical records, so it cannot compete as a source of truth.
3. Trace every result path from raw input through normalization, applicable check, state/diagnostic separation, aggregation, output, and exit behavior.
4. Trace Evidence, Judgment, and Approval through creation, dependency change, invalidation or re-evaluation, compatibility read, and reporting. Treat a readable historical record as effective only after its binding to the current subject and closure is confirmed.
5. Confirm every compatibility branch converges on the same canonical validation path. Reject candidate selection, default synthesis, partial closure, or older representations that bypass current invariants.
6. Map changed guarantees to deterministic Annex C observations. Take a test or green suite as evidence for a guarantee when its assertions establish the current contract; a test name alone shows nothing.
7. Treat implementation and tests as evidence to check against the authority. Acceptance evidence establishes current canonical conformance only when its assertions are explicitly mapped to current canonical clauses and observables. Use historical or superseded acceptance evidence for regression checks, and add current mapped evidence to establish current conformance.

For each defect, give the smallest input or lifecycle event demonstrating it. Distinguish change-introduced defects from pre-existing canonical gaps and count an in-axis baseline gap against the handoff.

## Output

Order findings by handoff risk. Include severity, declared axis, exact canonical and implementation evidence, minimal counterexample or missing proof, next-phase consequence, and whether the issue is change-introduced, baseline debt, or unresolved authority.

End with exactly one review decision:

- `READY`: every selected-axis responsibility can be handed to the next phase;
- `NOT_READY`: a selected-axis responsibility is violated or lacks required evidence;
- `INSUFFICIENT_EVIDENCE`: no violation is proven, but required handoff evidence is unavailable.

Keep this decision as an architecture-review decision, separate from verification states and gate-satisfaction values. List excluded axes and untested paths, and scope the claim to the reviewed axes, because consistency outside them was not examined.
