# Architecture

SpecTracer (`vtest`) is a fail-closed verification layer around language and
test-runner ecosystems. In v0.1.0-alpha.2 the verification contract is exposed
through a language-neutral adapter API and the built-in adapter is Rust/Cargo.

## Dependency direction

The implementation is layered so that entrypoints do not own canonical data or
verification rules:

```text
vtest-cli / vtest-mcp
        |
vtest-verify / vtest-exec / vtest-audit / vtest-scan
        |                         |
        v                         v
vtest-store <------------- vtest-adapter-rust
        |                         |
        v                         v
vtest-model <------------- vtest-adapter-api
```

- `vtest-model` contains shared entities, identities, hashes, diagnostics, and
  verification states.
- `vtest-store` owns canonical append-only record persistence and reload.
- `vtest-scan` selects registered discovery capabilities, merges deterministic
  results, and applies the structured-operation facade.
- `vtest-audit` selects static-audit capabilities and preserves append-only
  audit storage.
- `vtest-exec` selects runner and coverage capabilities and records execution
  Evidence.
- `vtest-verify` rebuilds the verification graph and performs fail-closed
  aggregation.
- `vtest-cli` is the non-interactive command-line entrypoint.
- `vtest-mcp` exposes the supported MCP stdio surface by delegating to the same
  CLI/core behavior and JSON envelope.

## Canonical data and freshness

Canonical records live under `.verify/`, one record per file where specified.
Indexes, graphs, and aggregate results are derived by rebuilding from those
records; `.verify/cache/` is never a source of truth. Relation records are
immutable and changes are represented by removing the old relation and adding a
new one.

Content hashes bind specifications, approvals, audits, and execution Evidence to
the revision they describe. A changed subject invalidates prior proof. Missing,
stale, unavailable, ambiguous, or otherwise non-`PASS` states remain non-passing
and are never silently promoted.

## Adapter boundary

The stable contract covers specifications, requirements, Verification
Obligations, test identity and intent, approvals, audits, Evidence, hashes,
aggregation, and reporting. Ecosystem-specific discovery, symbol resolution,
test execution, and coverage measurement belong behind explicit adapters.

`vtest-adapter-api` contains the object-safe discovery, static-audit,
structured-test, runner, and coverage capabilities. `vtest-adapter-rust` owns
Rust parsing, Cargo target resolution, static rules, runner commands,
`rustc-demangle`, `cargo-llvm-cov`, and built-in Rust forms. The separation is
tracked in [`docs/SpecTracer 言語アダプタ分離リファクタリング計画 v0.2.md`](docs/SpecTracer%20言語アダプタ分離リファクタリング計画%20v0.2.md).
Future TypeScript/JavaScript, Go, C#, and other adapters must preserve the same
contract and evidence rules without adding language-specific types to the
neutral API.

## Interface parity

CLI and MCP operations must be non-interactive and return the same JSON result
shape. MCP is an adapter, not a second verification engine. Changes to a core
operation must therefore update both interface paths and their parity tests.

For the normative requirements and detailed design, see the documents under
[`docs/`](docs/), especially the requirements, basic specification, detailed
design, and interface annexes.
