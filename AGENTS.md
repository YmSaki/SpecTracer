# SpecTracer project guidance

## Product boundary

- Build `vtest`, a Rust verification tool that decides whether a passing test is trustworthy from Specification through Execution Evidence.
- Act as a verifier, not as the authority that decides whether a specification, test, or implementation should be changed. Report mismatches and leave the correction choice to the owner.
- Treat all non-`PASS` states (`FAIL`, `MISMATCH`, `MISSING`, `NOT_CHECKED`, `NOT_EXECUTED`, `STALE`, `UNKNOWN`) as non-passing. Never infer `PASS` from missing evidence.
- Keep one source of truth. Rebuild indexes, graphs, and aggregate results from canonical files; do not hand-edit derived `.verify/cache/` data.
- Keep the initial implementation Rust-specific without making the domain model unnecessarily Rust-specific.

## Specification precedence

Read the relevant documents before changing behavior:

1. `docs/*要件定義・要件分解*.md` defines required guarantees and scope.
2. `docs/*基本仕様*.md` defines externally observable behavior and wins over detailed design on conflict.
3. `docs/*詳細設計 v0.1.md` and annexes A/B define implementation details and milestone order.

If documents disagree, report the exact sections. Do not silently choose a repair. The known `spec_coverage` and stored-versus-derived VO status tensions remain review items, not permission to weaken fail-closed behavior.

## Architecture and implementation order

- Preserve the dependency direction `vtest-cli / vtest-mcp -> vtest-verify / vtest-exec / vtest-audit / vtest-scan -> vtest-store -> vtest-model`.
- Implement milestones M1 through M9 in order. Do not declare a milestone complete until every acceptance criterion in detailed-design annex B §18 is reproducible under `cargo test`.
- Prefer one record per file and append-only ULID records. Keep Relation records immutable; represent a change as removing the old record and adding a new one.
- Use SHA-256 content binding exactly as detailed design §1.3 specifies. Changes must invalidate approvals, audits, and Evidence instead of carrying a prior pass forward.
- Prefer Form Schema and desired-state Structured Test Operations. One test edit must not alter another test or ordinary implementation/helper/fixture code.
- Keep CLI and MCP on the same core implementation and JSON result shape. CLI and MCP operations must be non-interactive.

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
