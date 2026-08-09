# Roadmap

This roadmap describes the public direction of SpecTracer. The requirements and
basic specification under [`docs/`](docs/) remain authoritative when a roadmap
item and an implementation detail differ.

## Current status

- M1 through M9 are complete and recorded in `tests/ACCEPTANCE.md`.
- The M9 MCP surface is accepted for the minimum ordinary stdio use case and
  delegates to the existing CLI/core JSON behavior.
- GitHub Actions now runs the main/develop CI gate and publishes tagged binary
  releases. The first live release still requires a maintainer-created version
  tag.
- The verification contract remains language- and test-runner-agnostic by design;
  v0.1.0-alpha.2 ships the `vtest-adapter-api` contract and the built-in
  `rust-cargo` discovery, audit, Structured Test Operation, runner, and coverage
  adapter. Synthetic mixed-adapter acceptance proves the boundary without
  claiming production support for another language.

## Next: production language adapters

The completed alpha.2 separation program is documented in
[`docs/SpecTracer 言語アダプタ分離リファクタリング計画 v0.2.md`](docs/SpecTracer%20言語アダプタ分離リファクタリング計画%20v0.2.md):

1. choose one ecosystem and one runner;
2. specify its selectors, source/target identity, freshness, and coverage binding;
3. implement only the capabilities the adapter can prove; and
4. add reproducible acceptance coverage before enabling it in a release.

TypeScript/JavaScript, Go, C#, and other language adapters should be added only
after their discovery, execution, freshness, and target-measurement behavior has
evidence-based acceptance coverage.

## Later opportunities

- additional tested release targets and package-manager integrations;
- native SDK interoperability for MCP if it becomes a supported requirement;
- ecosystem-specific adapters with explicit capability and `NOT_CHECKED` states;
- improved release provenance and reproducible build attestations.

These are not permission to weaken hash binding, source-of-truth ownership, or
fail-closed behavior. GUI features, automatic specification repair, and general
source-edit management remain outside the current product boundary.
