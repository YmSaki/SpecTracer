# Contributing to SpecTracer

Thank you for helping make verification claims more explicit, bounded, and
auditable. Contributions should strengthen the verification contract rather than
simply turn more cases into `PASS`.

Please read the [Code of Conduct](CODE_OF_CONDUCT.md) before participating. For
security-sensitive reports, follow [SECURITY.md](SECURITY.md) instead of opening a
public issue.

## Git Flow

This repository uses **Git Flow, not GitHub Flow**.

* `main`: released production code only. Do not develop directly on it.
* `develop`: integration branch for the next release.
* `feature/*`: branch from `develop`; merge back into `develop`.
* `release/*`: branch from `develop`; when complete, merge into **both `main` and `develop`**.
* `hotfix/*`: branch from `main`; when complete, merge into **both `main` and `develop`**.
* For normal implementation work, use `feature/*` based on `develop`.
* Do not treat `main` as the default development branch.

The `develop` branch must be created before normal feature work begins. Until it
exists, do not create feature branches from `main`; maintainers should establish
the integration branch first.

Do not force-push shared branches. Keep pull requests focused and preserve the
canonical-record and fail-closed invariants described in [ARCHITECTURE.md](ARCHITECTURE.md).

## Pull requests

Before opening a pull request:

1. Rebase or merge the current target branch as appropriate for your branch.
2. Add or update reproducible tests and fixtures for behavior changes.
3. Update public documentation and `CHANGELOG.md` when the user-visible behavior
   changes.
4. Run the checks in [TESTING.md](TESTING.md).
5. Describe any `NOT_CHECKED`, `NOT_EXECUTED`, `UNKNOWN`, or other remaining
   limitation explicitly.

The pull request body should explain the problem, the chosen scope, the tests
that provide evidence, and any specification or design review item. A passing
test must not be used to hide missing or stale evidence.

## Development boundaries

Keep the dependency direction and one-source-of-truth rules in `AGENTS.md`. Do
not hand-edit `.verify/cache/`; rebuild derived data from canonical records. Keep
CLI and MCP on the same core implementation and JSON result shape.

The current v0.1 implementation is Rust/Cargo-specific at its adapter boundary,
but the verification model is language-independent. New language support should
follow the adapter separation plan in
[`docs/SpecTracer 言語アダプタ分離リファクタリング計画 v0.2.md`](docs/SpecTracer%20言語アダプタ分離リファクタリング計画%20v0.2.md).

## Releases

Only maintainers create release tags. Update the workspace version, add the
user-visible entries to `CHANGELOG.md`, merge the release branch into `main`, and
create a `vMAJOR.MINOR.PATCH` or `vMAJOR.MINOR.PATCH-alpha.1` tag. GitHub Actions
verifies that the tag is on `main`, runs the full gate, builds Linux x86_64, macOS
x86_64, and Windows x86_64 archives, and publishes them with SHA-256 checksum
files. Pre-release tags are marked as GitHub pre-releases. See the
[prebuilt release instructions](README.md#install-a-prebuilt-release).
