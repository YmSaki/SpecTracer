# Contributing to SpecTracer

Thank you for helping make verification claims more explicit, bounded, and
auditable. Contributions should strengthen the verification contract rather than
simply turn more cases into `PASS`.

Please read the [Code of Conduct](CODE_OF_CONDUCT.md) before participating. For
security-sensitive reports, follow [SECURITY.md](SECURITY.md) instead of opening a
public issue.

## Git Flow

Branch roles, branch naming, tag naming, and merge destinations are defined
canonically in [DEVELOPMENT.md](DEVELOPMENT.md). Follow that policy, because a
second workflow definition here would diverge from it; when it does not cover
your case, open an issue.

Update shared branches by merge or fast-forward, because a force-push rewrites
history that others have built on; when a push is rejected, merge the remote
branch and push again. Keep pull requests focused and preserve the
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
that provide evidence, and any specification or design review item. Name the
missing or stale evidence beside every passing test that depends on it, because a
passing test hides an evidence gap that nobody has named.

## Development boundaries

Keep the dependency direction and one-source-of-truth rules in `AGENTS.md`.
Rebuild `.verify/cache/` from canonical records, because a hand edit makes
derived data disagree with its source; when a cache file looks wrong, fix the
canonical record and rebuild. Keep CLI and MCP on the same core implementation
and JSON result shape.

The current v0.1 implementation is Rust/Cargo-specific at its adapter boundary,
but the verification model is language-independent. New language support should
follow the adapter separation plan in
[`docs/SpecTracer 言語アダプタ分離リファクタリング計画 v0.2.md`](docs/SpecTracer%20言語アダプタ分離リファクタリング計画%20v0.2.md).

## Releases

Create release tags as a maintainer; as a contributor, ask a maintainer to cut the
release, because a published tag is an immutable reference to a released commit.
When a published tag looks wrong, report it in an issue and leave it where it is.

The canonical release branch roles, merge destinations, tag naming, and release
workflow are defined in [DEVELOPMENT.md](DEVELOPMENT.md).
