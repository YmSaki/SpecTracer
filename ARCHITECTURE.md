# Architecture

SpecTracer (`vtest`) is a fail-closed verification layer around language and
test-runner ecosystems. The current production adapter is Rust/Cargo, while
the verification contract is intentionally language-independent.

## Crates and dependency direction

The workspace has twelve crates. Lower crates own canonical data and
verification rules; upper crates delegate to them. The table is the current
state at `develop` `046c0d1` (2026-11-12); the rules behind the table are
[`AGENTS.md`](AGENTS.md) §3–§4. The last column lists normal dependencies
only; dev-dependencies, which only tests use, are left out.

| Crate | Layer | Owns | Depends on (`vtest-*`) |
| --- | --- | --- | --- |
| `vtest-model` | model | entities, identifiers, content and subject hashes, diagnostics, verification states, `JsonEnvelope`, `ExitCode` | — |
| `vtest-store` | persistence | the `.verify/` layout, config (v1 / v2), reading and writing the canonical records | `model` |
| `vtest-adapter-api` | adapter contract | capability traits, hash-free DTOs, `AdapterRegistry`, the Test wire shape | `model` |
| `vtest-adapter-rust` | adapter | the `rust-cargo` adapter: Rust discovery with `syn`, Cargo test execution, `cargo llvm-cov` coverage, static oracle analysis | `adapter-api`, `model` |
| `vtest-adapter-lexical` | adapter | framework-neutral, discovery-only adapters for eleven other languages (experimental; see below) | `adapter-api`, `model` |
| `vtest-adapter-synthetic` | adapter | a test-only adapter proving the core assumes nothing about Rust; not registered in the product | `adapter-api`, `model` |
| `vtest-scan` | orchestration | discovery delegation, output validation, deterministic merge by adapter id, record integrity, Structured Test Operations | `adapter-api`, `store`, `model` |
| `vtest-exec` | orchestration | running tests through the adapter runner, target-coverage attribution, Evidence recording | `adapter-api`, `store`, `model` |
| `vtest-verify` | orchestration | rebuilding the declaration chain, the four checks, Evidence freshness, scope, fail-closed aggregation | `scan`, `adapter-api`, `store`, `model` |
| `vtest-ops` | operations | one function per operation, the adapter registry (`registry::builtin_registry`), the shared envelope and diagnostic-code mapping | orchestration, `adapter-api`, `adapter-rust`, `adapter-lexical`, `store`, `model` |
| `vtest-cli` | entrypoint | the `vtest` binary, clap definitions, text rendering, exit codes | `ops`, `mcp` (the binary only, for `vtest mcp`), `scan`, `verify`, `adapter-api`, `store`, `model` |
| `vtest-mcp` | entrypoint | MCP stdio transport, tool schemas, argument validation, calling the same `ops` functions and returning the same envelope | `ops`, `scan`, `exec`, `verify`, `adapter-api`, `store`, `model` |

The layering (#175 D1, D2): both entrypoints call the operations crate and
neither depends on the other's implementation.

```text
vtest-cli / vtest-mcp         entrypoints: arguments → operation, result → text / JSON-RPC
        │
vtest-ops                     one operation = one function; registry composition,
        │                     envelope, exit codes, diagnostic-code mapping
        │
vtest-scan / vtest-exec / vtest-verify     orchestration: delegate to adapters, validate,
        │                                  merge deterministically, judge, aggregate
        │                                  │
vtest-store ── vtest-model                 vtest-adapter-rust / -lexical / -synthetic ── vtest-adapter-api ── vtest-model
```

`crates/vtest-cli/tests/architecture.rs` checks the edges that must not
exist: a core crate on a language adapter or a Rust syntax crate,
`vtest-adapter-api` on anything but `vtest-model`, `vtest-mcp` on `vtest-cli`,
and `vtest-ops` on either entrypoint.

Known deviation from the target, tracked by a child issue of #175:

- `vtest-cli` and `vtest-mcp` also depend on orchestration and persistence
  crates and call `vtest-scan` and `vtest-store` directly in places instead of
  going through `ops` (R3, #188).

The composition root (`vtest_ops::registry::builtin_registry`) is the only
place adapters are instantiated: `rust-cargo` and the eleven lexical adapters
(`c`, `cpp`, `csharp`, `go`, `java`, `javascript`, `kotlin`, `php`, `python`,
`ruby`, `swift`). The repository's own `.verify/config.yaml` enables only
`rust-cargo`.

## Canonical data and freshness

Canonical records live under `.verify/`, one record per file where specified.
Indexes, graphs, and aggregate results are derived by rebuilding from those
records; treat `.verify/cache/` as rebuildable output, and when it disagrees with
the records, rebuild it from them. Relation records are
immutable and changes are represented by removing the old relation and adding a
new one.

Content hashes bind specifications, approvals, audits, and execution Evidence to
the revision they describe. A changed subject invalidates prior proof. Missing,
stale, unavailable, ambiguous, or otherwise non-`PASS` states remain non-passing;
promote one only through new evidence bound to the current subject, because a
silent promotion is a false `PASS`.

## Adapter boundary

The stable contract covers document nodes, Verification Obligations, test
identity and intent, approvals, Evidence, hashes, aggregation, and reporting. Ecosystem-specific discovery, symbol resolution,
test execution, and coverage measurement belong behind explicit adapters.

The Rust adapter uses Cargo and Rust test metadata. The language-adapter
separation was planned in
[`docs/SpecTracer 言語アダプタ分離リファクタリング計画 v0.2.md`](docs/SpecTracer%20言語アダプタ分離リファクタリング計画%20v0.2.md)
(process material, not normative); every adapter, including the eleven
experimental lexical ones, must preserve the same contract and evidence rules.
Each language has a user-facing page under [`docs/adapters/`](docs/adapters/).

`vtest-adapter-lexical` is the shared, framework-neutral discovery engine for
languages other than Rust. A Test construct is the construct that immediately
follows a comment block declaring `@vtest.` test-keys, so discovery does not
depend on the project's test framework. Each language is a `LanguageProfile`
(lexical syntax, file extensions, function and scope recognition) registered as
an adapter that declares only `source_discovery`; runner, coverage, and static
analysis stay undeclared, so verification reports their checks as
`NO_EVIDENCE` rather than inventing a framework-specific default.

## Interface parity

CLI and MCP operations must be non-interactive and return the same JSON result
shape. Keep MCP an adapter that delegates to the core, because a second
verification engine would give a second answer. When a core operation changes,
update both interface paths and their parity tests.

For the normative requirements and detailed design, see the documents under
[`docs/`](docs/), especially the requirements, basic specification, detailed
design, and interface annexes.
