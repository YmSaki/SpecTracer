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

This repository uses **Git Flow, not GitHub Flow**. This section is the
canonical source for branch roles, branch naming, and tag naming.

### Long-lived branches

- `main`: released production code only. Do not develop directly on it.
- `develop`: integration branch for the next release. Normal work starts from
  `develop` and returns to `develop`.

### Working branches

- `feature/<topic>`: implementation, tooling, documentation, or other normal
  development work. Branch from `develop`; merge into `develop`.
- `spec/<topic>`: normative specification or design work. Branch from
  `develop`; merge into `develop`.
- `release/<semver>`: release stabilization. Branch from `develop`; when
  complete, merge into both `main` and `develop`.
- `hotfix/<topic>`: urgent correction to released production code. Branch from
  `main`; when complete, merge into both `main` and `develop`.
- `experiment/<topic>`: optional disposable investigation branch. It is not a
  release or baseline reference. Promote useful results through an appropriate
  `feature/*` or `spec/*` change rather than treating the experiment branch as
  permanent history.

Use `feature/*`, not the abbreviated `feat/*`.

Do not use numbered or version-like working branch names such as `v2`, `v13`,
or similar names that can be confused with releases or baselines.

Delete working branches after their merged or otherwise preserved work no
longer requires a movable branch reference.

### Tags

Tags represent immutable historical points, not active development lines.

- `vMAJOR.MINOR.PATCH` and prerelease forms such as
  `vMAJOR.MINOR.PATCH-alpha.1`: released versions. Release tags must refer to
  commits reachable from `main`.
- `baseline/<name>-v<version>`: owner-approved normative baselines, for example
  `baseline/requirements-v0.1` and `baseline/design-v0.1`.
- `archive/<topic>-YYYY-MM-DD`: preserved historical or abandoned work whose
  branch can then be deleted.

Release, baseline, and archive identities belong in tags rather than permanent
version-number working branches.

Do not move or force-update an existing release, baseline, or archive tag.
If a historical point was tagged incorrectly, stop and resolve it explicitly
rather than silently repointing the tag.

### Shared-history rules

Do not force-push `main`, `develop`, shared working branches, or published tags.
Keep commits focused. Documentation-only changes should not modify generated
`.verify/cache/` data or unrelated source files.

### Specification changes

The canonical specification is changed only on `spec/<topic>` branches.

- **Spec paths:** `docs/canonical/**` (except `docs/canonical/export/`, which is
  derived and carries no authority) and `.verify/doc/**` (the document records
  the verifier reads). VO records, tests, and code are not spec paths.
- **Only `spec/*` branches may change spec paths.** Every other branch leaves
  them untouched.
- **A `spec/*` branch changes no code and no tests** (`crates/`, `tests/`).
  Implementation and tests that depend on a spec change start after that spec
  PR is merged.
- **Target `develop`, never `main`.** `main` receives specification changes only
  through a release. A hotfix does not change the specification; if one needs
  it, make the change through `spec/*` and ship it in the next release.
- **Short-lived: one topic per branch, one spec PR open at a time.** IDs are
  allocated at the end of each layer, so two open spec PRs allocate the same
  numbers. If a second one is unavoidable, merge `develop` into it after the
  first is merged and rerun the gate (`vtest doctor` reports duplicate ids as
  `E-SCAN-010`).
- **Other branches pick the change up by merging `develop`** (merge, not
  rebase). They never touch spec paths, so the merge cannot conflict there.
- **If implementation exposes a defect, gap, or contradiction in the
  specification,** stop. Open an issue citing the clause IDs, fix it through a
  `spec/*` PR, then merge `develop` into the working branch and continue. Do
  not resolve it in code.
- **Keep derived data in step:** a spec PR updates the `.verify/doc/`
  projection together with `docs/canonical/specification.json`. Regenerating
  `docs/canonical/export/` may happen in any PR.
- **CI enforces the path rules** for pull requests into `develop`
  (`.github/scripts/spec-guard.sh`, job `spec-guard`).

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

Specification changes follow the branch and path rules in
[Git Flow → Specification changes](#specification-changes).

Read the relevant requirements, basic specification, and detailed design before
changing behavior. If they disagree, report the exact sections instead of
silently selecting a new interpretation. Update the corresponding acceptance
evidence in [`tests/ACCEPTANCE.md`](tests/ACCEPTANCE.md) for milestone work.
