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

### Changed

- The README now documents prebuilt binary installation and the language-neutral
  verification boundary.
- M9 MCP stdio operations use the same CLI/core JSON behavior while preserving
  fail-closed states.

There is no published version tag yet. The first release entry will be moved from
`[Unreleased]` into a dated `vMAJOR.MINOR.PATCH` section when that tag is created.
