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

Milestone acceptance tests live in `crates/vtest-cli/tests/` and are recorded in
[`tests/ACCEPTANCE.md`](tests/ACCEPTANCE.md). Run a single milestone while
developing, for example:

```bash
cargo test -p vtest-cli --test m9_acceptance
```

Run all CLI acceptance tests with:

```bash
cargo test -p vtest-cli --tests --locked
```

The acceptance ledger is the evidence source for M1 through M9. A milestone is
not complete when a required check is unavailable or not executed; record that
state as `NOT_CHECKED` or `NOT_EXECUTED`.

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
