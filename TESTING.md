# Testing guide

SpecTracer tests whether a passing test is trustworthy; a green test command is
not by itself proof that the verification graph is complete. Tests should keep
missing, stale, unavailable, and ambiguous evidence non-passing.

## Required local gate

Run this set before opening or updating a pull request:

```bash
cargo fmt --all -- --check
cargo test --workspace --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo run --quiet -p vtest-cli --locked -- doctor
```

The same gate runs in `.github/workflows/ci.yml` for `main`, `develop`, and their
pull requests.

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

Not every 別紙C §18 node has a dedicated acceptance test yet: nodes covering
product areas this slice's `crates/vtest-cli/src/ops/` does not implement
(static audit, coverage measurement, Structured Test Operation create/edit)
are tracked in `reports/closure-trace.md`'s gap table rather than given a
test that would only assert the absence of a feature.

## Verification behavior to preserve

- Every non-`PASS` state (`FAIL`, `MISMATCH`, `MISSING`, `NOT_CHECKED`,
  `NOT_EXECUTED`, `STALE`, or `UNKNOWN`) contributes to a non-passing result.
- Stale approval, audit, and Evidence hashes cannot satisfy a current verify.
- Limited scopes keep out-of-scope items as `NOT_CHECKED` rather than converting
  them to `PASS`.
- CLI and MCP must preserve the same core JSON envelope and result semantics.
- Structured test edits must stay within one Test and be deterministic and
  idempotent.

When changing static audit, execution, aggregation, record schemas, CLI, or MCP,
also run the applicable fixture flow (`vtest audit static`, bundle/submit,
`vtest run`, and `vtest verify`) and record the result.

## Release verification

The release workflow repeats the full gate before building binaries. It also
checks the semantic tag, `main` ancestry, and Cargo version. After a release,
download an archive from GitHub Releases, verify its `.sha256` file, extract
`vtest`, and run `vtest --help` and a read-only command such as `vtest doctor` in
an isolated project.
