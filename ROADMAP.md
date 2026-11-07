# Roadmap

This roadmap describes the public direction of SpecTracer. The requirements and
basic specification under [`docs/`](docs/) remain authoritative when a roadmap
item and an implementation detail differ.

## Current status

- The canonical v0.1 specification (`docs/canonical/specification.json`)
  replaced the earlier eleven-item surface with four checks and five states.
  The M1–M9 rows in `tests/ACCEPTANCE.md` record the retired surface and are
  not evidence for the current model.
- Implemented CLI commands: `init`, `scan`, `doctor`, `mcp`, `run`, `verify`
  (including `--gate` with approval-role evaluation), `approval`, and `doc`. Not provided yet: `vo`,
  `audit`, `report`, and judgment records.
- The MCP stdio server starts with `vtest mcp` and advertises nine tools that
  share the CLI's operation layer; the remaining tools are open work.
- The adapter API, registry, Rust adapter, and synthetic adapter exist; the
  v0.2 separation is in progress. An experimental, discovery-only Java adapter
  is built on the framework-neutral `vtest-adapter-lexical` engine
  ([docs/adapters/](docs/adapters/lexical.md)).
- GitHub Actions runs the main/develop CI gate and publishes tagged binary
  releases. A self-verify workflow runs `vtest run --all` and `vtest verify`
  on this repository nightly and on demand. The first live release still requires a maintainer-created version
  tag.
- Progress is tracked with GitHub
  [milestones](https://github.com/YmSaki/SpecTracer/milestones) and
  [issues](https://github.com/YmSaki/SpecTracer/issues), not in this file.

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
