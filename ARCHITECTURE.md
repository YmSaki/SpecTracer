# Architecture

SpecTracer (`vtest`) is a fail-closed verification layer around language and
test-runner ecosystems. The current production adapter is Rust/Cargo, while
the verification contract is intentionally language-independent.

## Crates and dependency direction

The workspace has eleven crates. Lower crates own canonical data and
verification rules; upper crates delegate to them. The table is the current
state at `develop` `fd22549` (2026-11-07); the rules behind the table are
[`AGENTS.md`](AGENTS.md) §3–§4.

| Crate | Layer | Owns | Depends on (`vtest-*`) |
| --- | --- | --- | --- |
| `vtest-model` | model | entities, identifiers, content and subject hashes, diagnostics, verification states, `JsonEnvelope`, `ExitCode` | — |
| `vtest-store` | persistence | the `.verify/` layout, config (v1 / v2), reading and writing the canonical records | `model` |
| `vtest-adapter-api` | adapter contract | capability traits, hash-free DTOs, `AdapterRegistry`, the Test wire shape | `model` |
| `vtest-adapter-rust` | adapter | the `rust-cargo` adapter: Rust discovery with `syn`, Cargo test execution, `cargo llvm-cov` coverage, static oracle analysis | `adapter-api`, `model` |
| `vtest-adapter-lexical` | adapter | framework-neutral, discovery-only adapters for eleven other languages (experimental; see below) | `adapter-api`, `model` |
| `vtest-adapter-synthetic` | adapter | a test-only adapter proving the core assumes nothing about Rust; not registered in the product | `adapter-api`, `model` |
| `vtest-scan` | orchestration | discovery delegation, output validation, deterministic merge by adapter id, record integrity, Structured Test Operations | `adapter-api`, `store`, `model` (and `adapter-rust`; see deviations) |
| `vtest-exec` | orchestration | running tests through the adapter runner, target-coverage attribution, Evidence recording | `adapter-api`, `store`, `model` |
| `vtest-verify` | orchestration | rebuilding the declaration chain, the four checks, Evidence freshness, scope, fail-closed aggregation | `scan`, `adapter-api`, `store`, `model` |
| `vtest-cli` | entrypoint | the `vtest` binary, clap definitions, text rendering, exit codes; today also the `ops` layer and the adapter registry (`adapters::builtin_registry`) | orchestration, all adapters, `store`, `model` |
| `vtest-mcp` | entrypoint | MCP stdio transport, tool schemas, argument validation, calling the same `ops` functions and returning the same envelope | `cli`, orchestration, `adapter-api`, `store`, `model` |

The target layering (#175 D1; D2 approved 2026-11-07) moves the `ops` layer
out of `vtest-cli` into an operations crate that both entrypoints depend on.
Its name is pending (`vtest-ops` is the working name):

```text
vtest-cli / vtest-mcp         entrypoints: arguments → operation, result → text / JSON-RPC
        │
operations crate              one operation = one function; registry composition,
(name pending)                envelope, exit codes, diagnostic-code mapping
        │
vtest-scan / vtest-exec / vtest-verify     orchestration: delegate to adapters, validate,
        │                                  merge deterministically, judge, aggregate
        │                                  │
vtest-store ── vtest-model                 vtest-adapter-rust / -lexical / -synthetic ── vtest-adapter-api ── vtest-model
```

Known deviations from the target, each tracked by a child issue of #175:

- `vtest-mcp` depends on `vtest-cli` for the `ops` layer, so the `vtest`
  binary compiles `vtest-mcp`'s source in through a `#[path]` module instead
  of a package dependency, which would be circular (R2, #187).
- `vtest-mcp` also calls `vtest-scan`, `vtest-store`, `vtest-exec` and
  `vtest-verify` directly in places instead of going through `ops` (R3, #188).
- `vtest-scan` has a normal dependency on `vtest-adapter-rust` and `syn` for
  Structured Test Operations (R6, #191).
- Evidence YAML is written by `vtest-exec`, while the record format belongs to
  `vtest-store` (R9, #194).

The composition root (`vtest_cli::adapters::builtin_registry`) is the only
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
