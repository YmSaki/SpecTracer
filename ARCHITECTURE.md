# Architecture

SpecTracer (`vtest`) is a fail-closed verification layer around language and
test-runner ecosystems. The current production adapter is Rust/Cargo, while
the verification contract is intentionally language-independent.

## Dependency direction

The implementation is layered so that canonical data and verification rules live in
the lower crates and entrypoints delegate to them:

```text
vtest-mcp
    |
vtest-cli (CLI + shared `ops` layer)
    |
vtest-verify / vtest-exec / vtest-scan
    |
vtest-adapter-rust / vtest-adapter-lexical / vtest-adapter-synthetic
    |
vtest-adapter-api
    |
vtest-store
    |
vtest-model
```

- `vtest-model` contains shared entities, identities, hashes, diagnostics, and
  verification states (four checks, five states, diagnostic labels).
- `vtest-store` owns canonical append-only record persistence and reload.
- `vtest-adapter-api` defines the adapter traits, capability declarations, and
  the registry. The core resolves adapters only through the registry.
- `vtest-adapter-rust` implements Rust discovery, Locators, Cargo test
  execution, and coverage. `vtest-adapter-synthetic` is a test-only adapter
  that proves the core does not assume Rust. `vtest-adapter-lexical` is the
  experimental, discovery-only engine for other languages (see below).
- `vtest-scan` orchestrates discovery and implements structured test
  operations (which still parse Rust directly; tracked as open work).
- `vtest-exec` runs tests through the adapter runner and records Evidence.
- `vtest-verify` rebuilds the declaration chain and performs fail-closed
  aggregation.
- `vtest-cli` is the non-interactive command-line entrypoint and owns the
  shared `ops` layer that both interfaces call.
- `vtest-mcp` exposes the MCP stdio tools by calling the same `ops`
  functions and returning the same JSON envelope. `vtest mcp` starts it: the
  `vtest` binary compiles `vtest-mcp`'s source in through a `#[path]` module,
  because `vtest-mcp` already depends on `vtest-cli` and a package dependency
  would be circular.

The CLI composition root (`vtest_cli::adapters::builtin_registry`) is the only
place adapters are instantiated.

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
separation is tracked in
[`docs/SpecTracer 言語アダプタ分離リファクタリング計画 v0.2.md`](docs/SpecTracer%20言語アダプタ分離リファクタリング計画%20v0.2.md);
future TypeScript/JavaScript, Go, C#, and other adapters must preserve the same
contract and evidence rules.

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
