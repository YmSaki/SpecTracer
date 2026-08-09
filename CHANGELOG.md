# Changelog

All notable user-visible changes to SpecTracer are recorded here. The project
follows the spirit of [Keep a Changelog](https://keepachangelog.com/en/1.1.0/)
and uses semantic version tags for releases.

## [Unreleased]

## [0.1.0-alpha.2]

### Added

- Language-neutral `vtest-adapter-api` capability contracts and deterministic
  adapter registry.
- Built-in `rust-cargo` adapter crate containing Rust discovery, structured
  operations, static audit, runner, coverage, and built-in forms.
- Version 2 adapter-namespaced configuration with version 1 read compatibility.
- Synthetic mixed-adapter acceptance coverage for deterministic merge,
  duplicate Test IDs, missing capabilities, adapter-bound Evidence, and stale
  hash invalidation.

### Changed

- Scan, audit, execution, CLI, and MCP paths now compose the same adapter-aware
  core while retaining the v0.1 JSON and legacy Test fields.
- Rust-specific parser, Cargo, demangling, coverage, and form logic is isolated
  from the neutral model and orchestration facades.

## [0.1.0-alpha.1]

### Added

- GitHub Actions CI for `main`, `develop`, and pull requests.
- Tag-driven GitHub Releases containing `vtest` archives for Linux x86_64,
  macOS x86_64, and Windows x86_64 with SHA-256 checksums.
- M9 MCP stdio operations using the same CLI/core JSON behavior while preserving
  fail-closed states.

The alpha entries are pre-release checkpoints. Production language adapters and
stable package-manager distribution remain future work.
