## Summary

<!-- What changed, and why? Keep the scope narrow. -->

## Branch flow

- [ ] Feature work targets `develop` (or `main` until `develop` exists).
- [ ] Release/hotfix work targets `main` and includes the matching back-merge plan.

## Verification evidence

- [ ] `cargo fmt --all -- --check`
- [ ] `cargo test --workspace --locked`
- [ ] `cargo clippy --workspace --all-targets --locked -- -D warnings`
- [ ] `cargo run --quiet -p vtest-cli --locked -- doctor`
- [ ] Applicable acceptance or fixture tests are updated and passing.

## Review notes

- User-visible changes and release notes are reflected in `CHANGELOG.md`.
- Any `NOT_CHECKED`, `NOT_EXECUTED`, `UNKNOWN`, stale, or blocked item is called out.
- No derived `.verify/cache/` data was hand-edited.
