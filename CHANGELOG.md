# Changelog

All notable user-visible changes to SpecTracer are recorded here. The project
follows the spirit of [Keep a Changelog](https://keepachangelog.com/en/1.1.0/)
and uses semantic version tags for releases.

## [Unreleased]

## [0.1.0-rc.1] - 2026-11-21

A release candidate. The command set, the four checks, and the five states are
the same as in `0.1.0-beta.1`. This release changes what the Rust static
analysis (`oracle_presence`, and the static side of `target_binding`) reports,
makes `vtest verify` much faster when Evidence is present, and turns an
unexpected panic into `E-INTERNAL-001`.

What each language gets in this release:

- **Rust (`rust-cargo`)**: officially supported. `vtest scan`, `vtest run`, and
  `vtest verify` work end to end, and `vtest verify` can return `PASS`.
- **Go and JavaScript/TypeScript**: early-access preview. The canonical scope
  now names `go` and `javascript` as v0.1 production adapters (#278), but in
  this release they still implement discovery only. `vtest scan` reads the
  `@vtest.` declarations, `vtest run` is rejected with `E-ADAPTER-004`, and
  `target_binding` and `oracle_presence` stay `NO_EVIDENCE`, so `vtest verify`
  ends with `Result: NG`.
- **The other nine adapters (C, C++, C#, Java, Kotlin, PHP, Python, Ruby,
  Swift)**: early-access preview, unchanged from `0.1.0-beta.1`. Declaration
  tracing only.

Residual shapes in which the Rust static analysis can still report a false
`PASS` are disclosed under "Known limitations". They are accepted for a
release candidate on the condition that they are listed there.

### Added

- Rust `oracle_presence` and `target_binding` read the static side of target
  reachability. `target_binding` now composes reach per target from the static
  analysis (DA-002) and the runtime coverage: a static `PASS` meets the reach
  requirement where coverage cannot decide, a measured count of 0 is still
  `FAIL` with `NOT_EXECUTED`, and a static `FAIL` is not overturned by runtime
  coverage (#334, #338). `audit static` reads the same verdict.
- `E-EXEC-004` now names what changed: the added, removed, and modified input
  paths (at most ten per kind, then `(+N omitted)`) and the names of other
  elements that changed. File contents, environment values, and absolute paths
  are not shown (#276).
- Release archives now include `aarch64-apple-darwin` (Apple Silicon), in
  addition to Linux x86_64, macOS x86_64, and Windows x86_64 (#272).
- `README.md` states that Evidence is bound to the execution environment
  (a different `CARGO*` / `RUST*` variable between `vtest run` and
  `vtest verify` makes the result `STALE`, `CARGO_TARGET_DIR` included), and
  that `doc list --tree` shows the chain of documents, not the nodes inside
  them (#344).

### Changed

- `vtest verify` collects the Test-independent Rust workspace inputs once per
  verification instead of once per Test. On this repository (933 Tests, 930
  Evidence records, release build, Windows 11), `verify --format json` went
  from 975.6 s to 62.8 s; the output is byte-identical (#345). `vtest run` is
  unchanged.
- An unexpected panic in a CLI command or an MCP tool call now ends as
  `E-INTERNAL-001` (CLI exit code 3; an MCP tool error, and the server keeps
  serving) instead of exit code 101 (#275). The panic payload is not copied
  into the result.
- Rust `oracle_presence` fails closed on many more shapes. Each change below
  moves a result from `PASS` toward `UNKNOWN` or `FAIL`; none moves toward
  `PASS` except where stated.
  - A Test that passes the target's result to a helper that does the
    comparison is `UNKNOWN`, not `FAIL` (#280). Only constructors
    (`Some` / `Ok` / `Err`), `Box::new`, and `String::from` count as value
    conversions; passing a result to any other call is a delegation and is
    `UNKNOWN` unless the helper's termination is confirmed (#310).
  - A target called as a method, used as a value (`f(2)` after `let f = double`,
    `map(Calc::double)`), or called inside the tokens of a non-assert macro is
    `UNKNOWN`, not "not called" (#284, #290, #292).
  - A `let` binding reaches an assert only when its exact identifier appears in
    a checked argument, and only when the target's value flows into the
    initializer and into the position the assert checks (#287, #296, #298).
    A `let mut` binding changed through a place is `UNKNOWN` (#302).
  - The assert macro list is closed (`assert!`, `assert_eq!`, `assert_ne!`,
    `panic!`, `unreachable!`, `todo!`, `unimplemented!`, plus config
    `assertion_macros`); a Test checked only by `debug_assert*` is `UNKNOWN`
    (#312).
  - A check that a literal makes always true (`X || true`) is not an oracle
    (#314). `matches!` with an irrefutable pattern is a constant assertion
    (`FAIL`), and one whose pattern only the type can decide is `UNKNOWN`
    (#326, #341).
  - Failure divergences (`if r != 4 { panic!() }`, `match`, let-else) are
    judged by the positions that decide them (#323). An assert at a position
    that never runs does not count as reach, and gives `UNKNOWN` (#330).
  - Branch values are joined per position, so a branch that is derived in one
    arm only is `UNKNOWN` (#328).
  - Moves toward `PASS`, where the spec now allows it: derivation is tracked
    per position through projection assignments (#318), and through local
    closures and pattern bindings (#327).
- Documentation: `README.md`, `ROADMAP.md`, and the adapter notes were brought
  in line with `0.1.0-beta.1` (supported languages, 13 CLI subcommands, 24 MCP
  tools, the 60-second tour with its `cargo-llvm-cov` prerequisite and
  `cargo generate-lockfile`) (#273); the production adapter scope is stated as
  `rust-cargo`, `go`, and `javascript` in `docs/adapters/lexical.md` (#320); and
  `README.md` states that `E-EXEC-004` is reported per Test (#347).

### Fixed

- An unexpected panic exits with `E-INTERNAL-001` and exit code 3, not 101
  (#230, #275).
- `E-EXEC-004` no longer leaves the cause unnamed (#234, #276).
- Documentation that said Java was the only non-Rust adapter, that `vtest vo`
  was missing, and that MCP had 9 tools is corrected (#212, #273). The Ruby
  adapter note no longer lists a `coverage/` exclusion the adapter does not
  have, and the `run.coverage` value in the lexical adapter example is `off`
  (#273).

### Known limitations

- Residual false-`PASS` shapes of the Rust static analysis, accepted for this
  release candidate (#322). In both, a divergence is read as checked although
  the Test passes for any target result:
  - complementary conditions, such as `if c {} else if !c {} else { panic!() }`
    or `if c { return } if !c { return } panic!()`;
  - an unrelated non-constant condition next to a derived exit, such as
    `if double(2) == 99 { return } if flag() { panic!() }`.
- Other open Rust analysis gaps that can report `oracle_presence` `PASS`
  although the target's result is not checked: a tuple or struct position that
  does not hold the target result but is read (`let t = (double(2), 1);
  assert_eq!(t.1, 1)`, #315); the whole non-literal base of a projection counted
  as flowing (#297); a change to a non-`let mut` binding through a `&mut T`
  return or interior mutability (#301); and a target matched by name only, so a
  same-named function or method can be taken for it (#283). The analysis also
  keeps some shapes disclosed in the PR for #330 (for example a guard decided
  only by a bound name, or a call named `exit()`).
- Rust: config `assertion_macros` entries never match in production, because
  the store rejects `!` and the matcher expects `name!`. A macro you list there
  is not counted as an assert-equivalent, so a Test that uses only that macro
  stays below `PASS` (#311). Config version 2 also does not validate
  `adapters[].scan.assertion_macros` (#211).
- Rust: a correct Test can be `UNKNOWN` where the analysis is narrower than the
  spec, for example when the same name is bound by several `let`s (#285).
- Go and JavaScript/TypeScript are an early-access preview, as are the other
  nine adapters. In this release none of them can reach `PASS` or
  `Result: OK`, and `vtest run` is rejected with `E-ADAPTER-004` (#278 brings
  `go` and `javascript` into the v0.1 production scope; their runners are not
  implemented yet).
- This repository's own `vtest verify` ends `NO_EVIDENCE` with `Result: NG`,
  with 0 `FAIL` and 0 `MISMATCH`. Every non-`PASS` cites the clause that makes
  it non-`PASS`: `target_binding` is `PASS` 894, `UNKNOWN` 34, and
  `NO_EVIDENCE` 13; `oracle_presence` is `PASS` 870 and `UNKNOWN` 71.
- Rust: `E-EXEC-004` is reported per Test. In a project with no `Cargo.lock`,
  the first `vtest run` writes no Evidence for the Test during which Cargo
  creates `Cargo.lock`; Tests that run after it get Evidence. The project is
  not broken: run `vtest run` again, or create the file first with
  `cargo generate-lockfile`. Whether the unit should be the Test or the whole
  run is open (#266).
- Rust: `vtest run` needs `cargo-llvm-cov` and the `llvm-tools-preview`
  rustup component for `target_binding` Evidence.
- Rust: a `STALE` result from a hash mismatch does not name the input that
  changed. #276 names the changed inputs for `E-EXEC-004` only.
- Rust: when a Test's `@vtest.target` names a type, not a function or method,
  that Test's `target_binding` check is `UNKNOWN`. Name the function or method
  the Test calls (#243).
- `vtest doc show` reports a broken document file as "not registered" (#255).
- A debug build of `vtest scan` / `doctor` / `verify` is about four times
  slower than before on this repository; use a release build (#202).

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
