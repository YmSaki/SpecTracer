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
  the v0.1 discovery and execution adapter is Rust/Cargo.

## Next: v0.2 adapter separation

The next implementation program is
[`docs/SpecTracer 言語アダプタ分離リファクタリング計画 v0.2.md`](docs/SpecTracer%20言語アダプタ分離リファクタリング計画%20v0.2.md):

1. synchronize the normative specifications and adapter contracts;
2. extract language-neutral domain, store, verification, and evidence ports;
3. isolate the current Rust scanner, runner, and coverage behavior as an adapter;
4. preserve CLI/MCP parity and fail-closed aggregation through the migration; and
5. add reproducible acceptance coverage before enabling another ecosystem.

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
