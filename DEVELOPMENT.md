# Development guide

## Prerequisites

- Git
- Rust stable with `cargo`, `rustfmt`, and `clippy`
- `cargo-llvm-cov` and the Rust `llvm-tools-preview` component for M7 coverage
  acceptance
- Python 3 for project verification scripts

Clone the repository and work from its root:

```bash
git clone https://github.com/YmSaki/SpecTracer.git
cd SpecTracer
cargo build --workspace --locked
```

## Git Flow

Use the Git Flow branch model in `AGENTS.md`: `develop` is the integration
branch, `feature/*` branches merge into it, and `release/*` or `hotfix/*`
branches merge into both `main` and `develop`. `main` contains released
production code only. The `develop` branch must be created before normal feature
work begins.

Keep commits focused. Documentation-only changes should not modify generated
`.verify/cache/` data or unrelated source files.

## Common development loop

```bash
cargo fmt --all
cargo test --workspace --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo run --quiet -p vtest-cli --locked -- doctor
```

If coverage tooling is not already installed, run:

```bash
rustup component add llvm-tools-preview
cargo install cargo-llvm-cov --locked
```

For the full project verification gate, use the commands in
[TESTING.md](TESTING.md) and the project skill scripts under `.agents/skills/`.

Run the CLI against an isolated project directory when testing commands that
write `.verify/` records. Do not edit another test's source or hand-edit derived
indexes.

## Building the distributable binary

Build the release binary locally with:

```bash
cargo build --locked --release --package vtest-cli
./target/release/vtest --help
```

The authoritative packaging path is `.github/workflows/release.yml`. A
`vMAJOR.MINOR.PATCH` or `vMAJOR.MINOR.PATCH-alpha.1` tag is accepted only when
its commit is reachable from `main` and matches the `vtest-cli` workspace version.
The workflow marks pre-release tags accordingly and publishes Linux x86_64, macOS
x86_64, and Windows x86_64 archives with checksums.

## Documentation and specification changes

Read the relevant requirements, basic specification, and detailed design before
changing behavior. If they disagree, report the exact sections instead of
silently selecting a new interpretation. Update the corresponding acceptance
evidence in [`tests/ACCEPTANCE.md`](tests/ACCEPTANCE.md) for milestone work.
