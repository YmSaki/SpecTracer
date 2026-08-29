---
name: architecture-check
description: Review SpecTracer design or implementation changes against the current v0.1 canonical architecture, compatibility invariants, interfaces, and acceptance handoff contract.
---

# Architecture check

Current canonical contracts govern this review. Produce findings and a handoff decision; do not redesign or edit unless separately requested.

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

Select only applicable axes, but do not add, remove, or change them after review begins. If the axes prove insufficient, report an axis defect separately and leave the new concern unscored pending a separately declared review. Do not reject on an excluded axis. Report every material issue within a declared axis; finding count is not a reason to suppress findings.

A material canonical or safety violation discovered outside the declared axes must be reported as an out-of-scope blocker or trigger an explicit re-scope. Do not silently ignore it or return `READY` while it remains unresolved. Do not score it as an in-axis finding unless the review contract is explicitly re-scoped.

## Authority

Read the applicable sections in this order:

1. `docs/AI並列開発向けテスト検証システム 要求・要件定義 v0.1.md`
2. `docs/AI並列開発向けテスト検証システム 基本仕様 v0.1.md`
3. `docs/AI並列開発向けテスト検証システム 詳細設計 v0.1.md`
4. `docs/AI並列開発向けテスト検証システム 詳細設計 別紙A インターフェース仕様 v0.1.md`
5. `docs/AI並列開発向けテスト検証システム 詳細設計 別紙C 受入仕様 v0.1.md`
6. `DEVELOPMENT.md` for process and Git policy only.

An implementation schedule is non-normative. `DEVELOPMENT.md` is not a product-behavior authority. When normative levels disagree, preserve the higher product authority and cite exact sections. Do not choose implementation behavior as authority.

## Fixed model baseline

- Checks are exactly `chain_integrity`, `orphan_detection`, `target_binding`, and `oracle_presence`.
- States are exactly `PASS`, `FAIL`, `MISMATCH`, `NO_EVIDENCE`, and `UNKNOWN`.
- `MISSING`, `NOT_EXECUTED`, `NOT_CHECKED`, and `STALE` are diagnostic labels, not verification states.
- Upstream artifacts are generic `document` nodes connected by `derives_from`.
- Judgment, Approval, verification state, aggregate OK/NG, and gate satisfaction are separate concepts.
- The canonical Approval CLI is `vtest approval create`, `vtest approval withdraw`, and `vtest approval show`; `vtest vo approve` is an alias only.
- Limited scope never treats work outside the requested scope as `PASS`. Omitted checks represented in the aggregation tree remain `NO_EVIDENCE` with `NOT_CHECKED`. Entity scope outside the request remains explicitly unverified through the canonical scope representation.
- Deterministic inability does not justify a guessed `PASS` or `FAIL`.
- Compatibility input is effective only after current normalization, identity, completeness, freshness, and aggregation invariants pass.

## Review method

1. Inventory changed public behavior, persisted data, domain types, dependency edges, and compatibility paths. Trace each to canonical sections.
2. Check ownership and dependency direction. Core types stay language-neutral; language and runner specifics stay behind adapter capabilities. Derived data never becomes a competing source of truth.
3. Trace every result path from raw input through normalization, applicable check, state/diagnostic separation, aggregation, output, and exit behavior.
4. Trace Evidence, Judgment, and Approval through creation, dependency change, invalidation or re-evaluation, compatibility read, and reporting. A readable historical record is not necessarily effective.
5. Confirm every compatibility branch converges on the same canonical validation path. Reject candidate selection, default synthesis, partial closure, or older representations that bypass current invariants.
6. Map changed guarantees to deterministic Annex C observations. A test name or green suite alone proves nothing unless its assertions establish the current contract.
7. Treat implementation and tests as evidence, not authority. Acceptance evidence establishes current canonical conformance only when its assertions are explicitly mapped to current canonical clauses and observables. Historical or superseded acceptance evidence may support regression checks, but cannot by itself establish current conformance.

For each defect, give the smallest input or lifecycle event demonstrating it. Distinguish change-introduced defects from pre-existing canonical gaps without letting an in-axis baseline gap pass the handoff.

## Output

Order findings by handoff risk. Include severity, declared axis, exact canonical and implementation evidence, minimal counterexample or missing proof, next-phase consequence, and whether the issue is change-introduced, baseline debt, or unresolved authority.

End with exactly one review decision:

- `READY`: every selected-axis responsibility can be handed to the next phase;
- `NOT_READY`: a selected-axis responsibility is violated or lacks required evidence;
- `INSUFFICIENT_EVIDENCE`: no violation is proven, but required handoff evidence is unavailable.

This is an architecture-review decision, not a verification state or gate-satisfaction value. List excluded axes and untested paths. Do not claim global consistency.
