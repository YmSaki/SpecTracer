# Changelog

All notable user-visible changes to SpecTracer are recorded here. The project
follows the spirit of [Keep a Changelog](https://keepachangelog.com/en/1.1.0/)
and uses semantic version tags for releases.

## [Unreleased]

## [0.1.0-beta.1] - 2026-11-19

A pre-release. It replaces the verification commands of `v0.1.0-alpha.1` with
the canonical v0.1 model:

- four checks: `chain_integrity`, `orphan_detection`, `target_binding`, and
  `oracle_presence`;
- five states for a check and for the aggregate: `PASS`, `FAIL`, `MISMATCH`,
  `NO_EVIDENCE`, and `UNKNOWN`. `vtest verify` also prints the aggregate as
  `Result: OK` (exit code 0) or `Result: NG` (exit code 1).

[`README.md`](README.md) explains the terms used below (Verification
Obligation or VO, Test, Source Target, Evidence, Approval, gate).

What each language gets in this release:

- **Rust (`rust-cargo`)**: the full path. `vtest scan`, `vtest run`, and
  `vtest verify` work end to end, and `vtest verify` can return `PASS`.
- **Eleven experimental adapters (C, C++, C#, Go, Java,
  JavaScript/TypeScript, Kotlin, PHP, Python, Ruby, Swift)**: declaration
  tracing only. `vtest scan` reads the
  `@vtest.` declarations, and `chain_integrity` and `orphan_detection` are
  evaluated. `vtest run` is rejected with `E-ADAPTER-004`. A `vtest verify`
  that completes always ends with `Result: NG` and exit code 1, also when `--items` limits it
  to the two evaluated checks, because `target_binding` and `oracle_presence`
  stay `NO_EVIDENCE`. To see whether the declaration chain is complete, read
  the `chain_integrity` and `orphan_detection` lines of the `vtest verify`
  output: both show `PASS` when it is.

The release has known defects, listed under "Known limitations" below. This
repository does not yet pass its own `vtest verify`.

### Added

- Canonical v0.1 CLI: `init`, `scan`, `doctor`, `run`, `verify` (four checks,
  five states, `--items` / `--doc` / `--vo` / `--test` scopes, `--gate`,
  `--summary`), `report`, `export`, `approval create|withdraw|show`,
  `doc add|list|show`, `vo add|edit|list|show|expand|approve`,
  `test create|edit|list|query|show`, and `audit static|bundle|submit`.
- MCP stdio server, started with `vtest mcp`, with 24 tools that share the
  CLI's operation layer: `scan`, `run_tests`, `verify`, `report`, `export`,
  `approval_create`, `approval_withdraw`, `approval_get`, `doc_upsert`,
  `doc_list`, `doc_get`, `vo_list`, `vo_get`, `vo_upsert`, `vo_expand`,
  `vo_approve`, `test_create`, `test_edit`, `test_get`, `test_query`,
  `form_get`, `audit_static`, `audit_bundle`, and `audit_submit`.
- Adapter API with capability declarations and a registry, the Rust/Cargo
  adapter, and a test-only synthetic adapter.
- Experimental framework-neutral discovery adapters (`vtest-adapter-lexical`)
  for C, C++, C#, Go, Java, JavaScript/TypeScript, Kotlin, PHP, Python, Ruby,
  and Swift. There are eleven adapters, with the ids `c`, `cpp`,
  `csharp`, `go`, `java`, `javascript` (JavaScript and TypeScript), `kotlin`,
  `php`, `python`, `ruby`, and `swift`. `vtest init` writes a configuration
  for `rust-cargo` only, so a project in another language is not scanned until
  you edit `.verify/config.yaml`: set `adapters[].id` to the language's id,
  list your source directories in `adapters[].scan.include`, and set
  `adapters[].run.coverage` to `none`. See
  [`docs/adapters/`](docs/adapters/lexical.md).
- `verify --gate` evaluates the gate's approval roles against the scoped
  subjects.
- The `TestWireCodec` capability and the `rust-cargo` wire codec.
- GitHub Actions CI for `main`, `develop`, and pull requests.
- Tag-driven GitHub Releases containing `vtest` archives for Linux x86_64,
  macOS x86_64, and Windows x86_64 with SHA-256 checksums.
- Informational self-verify workflow that runs `vtest run --all` and
  `vtest verify` on this repository nightly and on demand.
- OSS contribution, architecture, development, testing, security, and roadmap
  documentation.

### Changed

- Canonical record schema failures in CLI/MCP operations now report
  `E-SCAN-010` (CLI exit 2); structured test operations preserve config
  and adapter diagnostic codes instead of masking them as internal errors.
- Internal errors that the tool catches report `E-INTERNAL-001`; a panic does
  not yet (see "Known limitations").
- Verification fails closed on more inputs: an invalid latest Evidence record
  is never replaced by an older one, when a Rust Test declares several
  targets, the coverage of each one is measured and judged, and an Approval whose records are broken or
  inconsistent is not treated as approved.
- The JSON result of `verify` (`vtest verify --format json` and the MCP
  `verify` tool) carries the requested scope as a top-level `scope` field.
- MCP `run_tests` takes exactly one of `test`, `vo`, or `all`; a call that
  gives more than one is rejected with `E-OP-001`.

### Fixed

- `vtest verify` no longer panics (exit code 101) on a Rust test function in
  which non-ASCII text, such as a Japanese string literal, is followed on the
  same line by an assert macro or by a call to the target (#264).

### Removed

- The verification commands of `v0.1.0-alpha.1` are no longer part of the
  CLI: `spec add`, `req add`, and the earlier `audit static`. The `audit static`
  listed under "Added" is a new command with the same name. The `m1`–`m9`
  acceptance suites are removed as well.
- The legacy `.verify/audits/` records and `AuditRecord`.

### Known limitations

- Languages other than Rust cannot reach `PASS` or `Result: OK`, as described
  at the top of this entry.
- Rust: the first `vtest run` in a project that has no `Cargo.lock` writes no
  Evidence and reports `E-EXEC-004`. The project is not broken: the tests ran,
  but Cargo created `Cargo.lock` during the run, so the inputs changed and the
  result was discarded. Run `vtest run` again, or create the file first with
  `cargo generate-lockfile` (#266).
- Rust: `vtest run` needs `cargo-llvm-cov` and the `llvm-tools-preview`
  rustup component for `target_binding` Evidence.
- An unexpected panic exits with code 101 instead of `E-INTERNAL-001` and
  exit code 3 (#230).
- A `STALE` or `E-EXEC-004` diagnostic does not name the input that changed
  (#234).
- Rust: when a Test's `@vtest.target` names a type, not a function or method,
  that Test's `target_binding` check is `UNKNOWN`. Name the function or method
  the Test calls (#243).
- `adapters[].scan.assertion_macros` in config version 2 is not validated
  (#211).
- `vtest doc show` reports a broken document file as "not registered" (#255).
- A debug build of `vtest scan` / `doctor` / `verify` is about four times
  slower than before on this repository; use a release build (#202).
- `README.md` and `ROADMAP.md` are behind this entry: they still describe Java
  as the only non-Rust language and `vtest vo` as missing (#212).
- Prebuilt archives are produced for Linux x86_64, macOS x86_64, and Windows
  x86_64. None of the three published archives was run before the release;
  the only binary run by hand was one built locally on Windows from the same
  source.

## [0.1.0-alpha.1] - 2026-08-09

The first pre-release tag. It carried the verification commands that
0.1.0-beta.1 removes.
