# SpecTracer project guidance

## Product boundary

- Build `vtest`, a language- and test-runner-neutral verification tool that decides whether a passing test is trustworthy from Specification through Execution Evidence. The provided production adapter is `rust-cargo`.
- Act as a verifier, not as the authority that decides whether a specification, test, or implementation should be changed. Report mismatches and leave the correction choice to the owner.
- Treat all non-`PASS` states (`FAIL`, `MISMATCH`, `MISSING`, `NOT_CHECKED`, `NOT_EXECUTED`, `STALE`, `UNKNOWN`) as non-passing. Never infer `PASS` from missing evidence.
- Keep one source of truth. Rebuild indexes, graphs, and aggregate results from canonical files; do not hand-edit derived `.verify/cache/` data.
- Keep language- and test-runner-specific behavior behind adapters; do not make the core domain model language-specific.

## Specification precedence

Read the relevant documents before changing behavior:

1. `docs/*要件定義・要件分解*.md` defines required guarantees and scope.
2. `docs/*基本仕様*.md` defines externally observable behavior and wins over detailed design on conflict.
3. `docs/*詳細設計 v0.1.md` and normative annexes A/C define implementation details, interfaces, and acceptance criteria. Annex B is a non-normative implementation plan.

If documents disagree, report the exact sections. Do not silently choose a repair. The known `spec_coverage` and stored-versus-derived VO status tensions remain review items, not permission to weaken fail-closed behavior.

## Canonical specification writing

- Requirements, the basic specification, the detailed design, and its normative annexes describe only the currently normative system contract: required states, observable behavior, interfaces, constraints, and acceptance criteria.
- Do not put implementation chronology, migration history, before-and-after comparisons, release-introduction history, refactoring procedure, or development phases in canonical specifications.
- Record implementation history in `CHANGELOG.md`, migration and refactoring procedure in the applicable plan, and prospective work in `ROADMAP.md`.
- Write an established architectural responsibility directly. For example, specify what an adapter owns, not when or how that responsibility was moved into the adapter.

## Upstream correction and approval

- A downstream phase may discover a defect in an upstream artifact, but it must not self-approve or silently rewrite that artifact. Stop the downstream work, return the evidence and proposed change to the owner, and resume only after the owner has approved and the upstream artifact has been finalized.
- During a specification-change phase, do not modify implementation code or test code. Finalize the specification change as an independent commit and pull request, and do not begin downstream implementation or test changes until the owner has approved and merged it.
- Even after a specification has been finalized, if implementation, testing, or verification exposes a new specification defect, contradiction, or omission, do not repair the specification in place. Stop the work, report the evidence to the owner, and restart from the appropriate upstream phase.
- Treat the workflow as iterative rather than one-way. When implementation or testing exposes a problem, classify it against the already-fixed upstream artifacts: fix an implementation defect downstream, check a suspected test defect against the specification, and return a suspected specification defect to the owner.
- The owner decides whether a normative specification changes and is responsible for sending or merging that decision. Agents report mismatches and options to the owner; they do not act as the specification authority.
- Do not combine a specification change, its implementation, its tests, and its acceptance-contract update into one self-validating commit or pull request. After the upstream change is merged, derive acceptance criteria and tests from it, confirm expected failures where useful, implement the downstream change, diagnose every failure against the fixed artifacts, and run full verification before proposing the implementation merge.

## Architecture and implementation order

- Preserve the dependency direction `vtest-cli / vtest-mcp -> vtest-verify / vtest-exec / vtest-audit / vtest-scan -> vtest-store -> vtest-model`, with `vtest-scan / vtest-audit / vtest-exec -> vtest-adapter-rust -> vtest-adapter-api -> vtest-model` for language-specific capabilities. `vtest-adapter-rust -> vtest-store` is limited to neutral Form Schema and canonical-layout types.
- Do not declare an implementation or release complete until every applicable acceptance criterion in detailed-design annex C §18 is reproducible under `cargo test`.
- Prefer one record per file and append-only ULID records. Keep Relation records immutable; represent a change as removing the old record and adding a new one.
- Use SHA-256 content binding exactly as detailed design §1.3 specifies. Changes must invalidate approvals, audits, and Evidence instead of carrying a prior pass forward.
- Prefer Form Schema and desired-state Structured Test Operations. One test edit must not alter another test or ordinary implementation/helper/fixture code.
- Keep CLI and MCP on the same core implementation and JSON result shape. CLI and MCP operations must be non-interactive.

## Adapter separation (v0.1.0-alpha.2)

- Follow `docs/SpecTracer 言語アダプタ分離リファクタリング計画 v0.2.md` W0-W8 in order; do not add production TypeScript, Go, C#, plugin ABI, LSP, or automatic repair policy in this release.
- Keep `vtest-adapter-api` language and runner neutral. Rust parser, Cargo command construction, Rust AST audit, demangling, and llvm-cov handling belong only to `vtest-adapter-rust`.
- Treat missing static-audit or coverage capabilities as `NOT_CHECKED`, a missing runner capability as `NOT_EXECUTED`, and analysis limits as `UNKNOWN`; never promote any of them to `PASS`. Reject unknown or duplicate adapter IDs and duplicate Test IDs across adapters.
- Read config versions 1 and 2 without rewriting them; `vtest init` writes version 2 adapter namespaces. `TestEntity` contains only `execution`; the `rust-cargo` wire codec owns version 1 compatibility fields and omits them for non-Rust Tests.
- Full verification has 12 items. `test_traceability` is repository-level and is `PASS` only when every Discovered Test maps to exactly one Managed Test Entity with at least one resolvable VO; W-SCAN-101 remains a warning diagnostic but its underlying unregistered Test makes `test_traceability` non-passing.
- CLI and MCP must compose the same adapter registry and retain the same JSON envelope and fail-closed diagnostics.

## Agent and skill use

- Use `$verify-change` after behavior, test, record-schema, CLI, MCP, audit, execution, or aggregation changes.
- Use `$architecture-check` for crate boundaries, canonical-data ownership, fail-closed aggregation, record immutability, and CLI/MCP parity.
- Use `$release-check` before completing a milestone, release, or broad integration change.
- Delegate only independent, bounded work. Use the luna `explorer` for read-only tracing, `reviewer` for an independent fail-closed review, and `tester` for tests and reproducible checks.
- Do not have multiple agents edit the same file or entity concurrently. The primary agent owns integration and waits for all delegated findings before concluding.

## Required validation

Run the narrowest relevant checks during development, then the full available set before handoff:

```powershell
cargo fmt --all -- --check
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
```

When `vtest-cli` exists, also run:

```powershell
cargo run --quiet -p vtest-cli -- doctor
```

For test changes, additionally run the applicable `vtest audit static`, audit-bundle/submit, `vtest run`, and `vtest verify` flow. State `NOT_CHECKED` or `NOT_EXECUTED` when a check is unavailable; never describe the overall result as fully verified.

## Code review rules

- Prioritize incorrect PASS promotion, stale hash acceptance, source-of-truth duplication, cross-test edits, nondeterministic output, CLI/MCP schema drift, and missing fixture coverage.
- Deterministic analysis may return `FAIL` only when the violation is certain; use `UNKNOWN` when analysis cannot prove it.
- Require reasons and concrete basis references for semantic audit results. Reject empty reasons and stale bundle hashes.
- Do not request GUI work, automatic repair policy, specification-to-specification auditing, general source-edit management, or other items explicitly outside v0.1 scope.

## Git Flow

This repository uses **Git Flow, not GitHub Flow**.

* `main`: released production code only. Do not develop directly on it.
* `develop`: integration branch for the next release.
* `feature/*`: branch from `develop`; merge back into `develop`.
* `release/*`: branch from `develop`; when complete, merge into **both `main` and `develop`**.
* `hotfix/*`: branch from `main`; when complete, merge into **both `main` and `develop`**.
* For normal implementation work, use `feature/*` based on `develop`.
* Do not treat `main` as the default development branch.
