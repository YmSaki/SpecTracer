# Testing guide

SpecTracer tests whether a passing test is trustworthy; treat a green test command
as one input and confirm the completeness of the verification graph separately.
Tests should keep missing, stale, unavailable, and ambiguous evidence non-passing.

## Required local gate

Run this set before opening or updating a pull request:

```bash
cargo fmt --all -- --check
cargo test --workspace --locked --no-fail-fast
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo run --quiet -p vtest-cli --locked -- doctor
```

The same gate runs in `.github/workflows/ci.yml` for `main`, `develop`, and their
pull requests. Pass `--no-fail-fast` locally, because without it `cargo test`
stops at the first failing test binary and the later binaries never report.
Without `cargo-llvm-cov` nine tests fail (`vtest-adapter-rust --lib` 2,
`evidence_e2e` 5, `execution_state_e2e` 2); they are classified in
[`docs/reports/refactor-baseline-2026-11-07.md`](docs/reports/refactor-baseline-2026-11-07.md)
§2.1. Report them as `NOT_EXECUTED` in the PR and cite the CI run, which has the
tool, as the evidence.

The CI environment installs `llvm-tools-preview` and `cargo-llvm-cov` so that M7
can distinguish measured target execution from the fail-closed unavailable
coverage path. Local M7 acceptance requires the same tools:

```bash
rustup component add llvm-tools-preview
cargo install cargo-llvm-cov --locked
```

## Acceptance tests

Acceptance tests live in `crates/vtest-cli/tests/` and are organized by 別紙C
（詳細設計 v0.1 別紙C 受入仕様）§18 section, one file per subsection —
`acceptance_18_1.rs`, `acceptance_18_3_7.rs`, `acceptance_18_3_8.rs`,
`acceptance_18_3_9.rs`. Each `#[test]` cites the 別紙C node id(s) it covers in
a doc comment. The predecessor `m1_acceptance.rs`…`m9_acceptance.rs` files
(keyed to 別紙B §18.3, a non-normative implementation-plan document, not the
normative 別紙C) have been retired; `autotests` is on, so every file in
`tests/` is discovered automatically. Run a single acceptance file while
developing, for example:

```bash
cargo test -p vtest-cli --test acceptance_18_3_7
```

Run all CLI acceptance tests with:

```bash
cargo test -p vtest-cli --tests --locked
```

Not every 別紙C §18 node has a dedicated acceptance test yet. Record a node
without one as a gap in [`tests/ACCEPTANCE.md`](tests/ACCEPTANCE.md) or
[`docs/reports/adapter-acceptance-matrix.md`](docs/reports/adapter-acceptance-matrix.md)
instead of writing a test that asserts the feature's absence, because such a
test passes while the feature is missing. When the feature is implemented, add
its acceptance test and remove the gap row.

## Verification behavior to preserve

- Every non-`PASS` state (`FAIL`, `MISMATCH`, `NO_EVIDENCE`, or `UNKNOWN`)
  contributes to a non-passing result. Diagnostic labels (`MISSING`,
  `NOT_CHECKED`, `NOT_EXECUTED`, `STALE`) explain the cause and are never
  ranked.
- Stale approval and Evidence hashes leave a current verify non-passing.
- Limited scopes keep out-of-scope checks as `NO_EVIDENCE` (`NOT_CHECKED`)
  rather than converting them to `PASS`.
- CLI and MCP must preserve the same core JSON envelope and result semantics.
- Structured test edits must stay within one Test and be deterministic and
  idempotent.

When changing execution, aggregation, record schemas, CLI, or MCP, also run
the applicable fixture flow (`vtest scan`, `vtest run`, and `vtest verify`)
and record the result. (`vtest audit` and bundle/submit are not provided yet.)
For a refactoring PR that must not change behavior, build the base and head
binaries and compare their outputs with the procedure in
[`docs/reports/refactor-baseline-2026-11-07.md`](docs/reports/refactor-baseline-2026-11-07.md)
§6; write any difference, with its clause ID, into the PR.

## Release verification

The release workflow repeats the full gate before building binaries. It also
checks the semantic tag, `main` ancestry, and Cargo version. After a release,
download an archive from GitHub Releases, verify its `.sha256` file, extract
`vtest`, and run `vtest --help` and a read-only command such as `vtest doctor` in
an isolated project.
