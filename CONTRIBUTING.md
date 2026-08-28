# Contributing to SpecTracer

Thank you for helping make verification claims more explicit, bounded, and
auditable. Contributions should strengthen the verification contract rather than
simply turn more cases into `PASS`.

Please read the [Code of Conduct](CODE_OF_CONDUCT.md) before participating. For
security-sensitive reports, follow [SECURITY.md](SECURITY.md) instead of opening a
public issue.

## Git Flow

Branch roles, branch naming, tag naming, and merge destinations are defined
canonically in [DEVELOPMENT.md](DEVELOPMENT.md). Contributors must follow that
policy rather than defining or inferring a separate Git workflow here.

Do not force-push shared branches. Keep pull requests focused and preserve the
canonical-record and fail-closed invariants described in
[ARCHITECTURE.md](ARCHITECTURE.md).

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

Only maintainers create release tags. Contributors must not create, move, or
replace published release tags.

The canonical release branch roles, merge destinations, tag naming, and release
workflow are defined in [DEVELOPMENT.md](DEVELOPMENT.md).
