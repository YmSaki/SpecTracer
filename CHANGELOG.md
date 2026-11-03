# Changelog

All notable user-visible changes to SpecTracer are recorded here. The project
follows the spirit of [Keep a Changelog](https://keepachangelog.com/en/1.1.0/)
and uses semantic version tags for releases.

## [Unreleased]

### Added

- GitHub Actions CI for `main`, `develop`, and pull requests.
- Tag-driven GitHub Releases containing `vtest` archives for Linux x86_64,
  macOS x86_64, and Windows x86_64 with SHA-256 checksums.
- OSS contribution, architecture, development, testing, security, and roadmap
  documentation.
- Canonical v0.1 CLI: `init`, `scan`, `doctor`, `run`, `verify` (four checks,
  five states, `--items` / `--doc` / `--vo` / `--test` scopes, `--gate`,
  `--summary`), `approval create|withdraw|show`, and `doc add|list|show`.
- Adapter API with capability declarations and a registry, the Rust/Cargo
  adapter, and a test-only synthetic adapter.
- MCP stdio server with nine tools (`scan`, `run_tests`, `verify`,
  `approval_create`, `approval_withdraw`, `approval_get`, `doc_upsert`,
  `doc_list`, `doc_get`) sharing the CLI's operation layer.

### Changed

- Canonical record schema failures in CLI/MCP operations now report
  `E-SCAN-010` (CLI exit 2); structured test operations preserve config
  and adapter diagnostic codes instead of masking them as internal errors.

- The README, roadmap, and architecture notes now describe the canonical
  four-check model, the commands and MCP tools that actually exist, and the
  crate layout; features that are not provided yet are labelled as such.

### Removed

- The retired eleven-item verification surface (`spec add`, `req add`,
  `vo add`, `audit static`, and the `m1`–`m9` acceptance suites) is no longer
  part of the CLI.

There is no published version tag yet. The first release entry will be moved from
`[Unreleased]` into a dated `vMAJOR.MINOR.PATCH` section when that tag is created.
