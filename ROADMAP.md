# Roadmap

This roadmap describes the public direction of SpecTracer. The requirements and
basic specification under [`docs/`](docs/) remain authoritative when a roadmap
item and an implementation detail differ.

## Current status

- The canonical v0.1 specification (`docs/canonical/specification.json`)
  replaced the earlier eleven-item surface with four checks and five states.
  The M1–M9 rows in `tests/ACCEPTANCE.md` record the retired surface and are
  not evidence for the current model.
- Implemented CLI commands: `init`, `scan`, `doctor`, `export`, `mcp`, `run`,
  `verify` (including `--gate` with approval-role evaluation), `report`,
  `approval`, `doc`, `test`, `audit` (`static`, `bundle`, `submit`), and `vo`.
  Judgment records (`.verify/decisions/` via `vtest audit bundle` / `submit`,
  read by `vtest test show` and `vtest report`) are implemented; `vtest approval
  --subject-type judgment` (`E-APPROVAL-001`) and `vtest audit bundle
  --include-failed` are not implemented yet.
- The MCP stdio server starts with `vtest mcp` and advertises 24 tools that
  share the CLI's operation layer (`vtest-ops`): `scan`, `run_tests`, `verify`,
  `report`, `export`, `approval_create`, `approval_withdraw`, `approval_get`,
  `doc_upsert`, `doc_list`, `doc_get`, `vo_list`, `vo_get`, `vo_upsert`,
  `vo_expand`, `vo_approve`, `test_create`, `test_edit`, `test_get`,
  `test_query`, `form_get`, `audit_static`, `audit_bundle`, and `audit_submit`.
  (`init` and `doctor` stay CLI-only by specification.)
- The adapter API, `vtest-ops` registry, Rust adapter (`rust-cargo`), and
  test-only synthetic adapter exist. Only `rust-cargo` supports the full
  `scan` → `run` → `verify` path to `PASS` (`vtest run` needs
  `cargo-llvm-cov` and `llvm-tools-preview` unless `--fast` is given).
- The Go adapter (`go`, `vtest-adapter-go`) is in the canonical v0.1 production
  scope and is still a preview. Discovery (by Test shape, delegated to the
  lexical `go` module), the `go test` runner with its Execution State, and
  coverage (`go-cover`) are done; see [docs/adapters/go.md](docs/adapters/go.md).
  The static audit (an in-process Go reader; DA-001, DA-003, DA-004, DA-005 and
  DA-006) gives `oracle_presence` a `PASS` for a Test that checks its target's
  result, so a Go Test can be `PASS` as a whole. The static reach of a target
  (DA-002) is not provided.
- Ten experimental, discovery-only adapters are built on the framework-neutral
  `vtest-adapter-lexical` engine (`c`, `cpp`, `csharp`, `java`, `javascript`
  for JS/TS, `kotlin`, `php`, `python`, `ruby`, and `swift`; see
  [docs/adapters/](docs/adapters/lexical.md)); `vtest run` rejects them with
  `E-ADAPTER-004`, and a completed `vtest verify` ends with `Result: NG`.
  `vtest init` writes configuration for `rust-cargo` only.
- GitHub Actions runs the main/develop CI gate and publishes tagged binary
  releases. `v0.1.0-beta.1` is published as a pre-release on the
  [GitHub Releases list](https://github.com/YmSaki/SpecTracer/releases)
  (pre-releases do not appear at `/releases/latest`). A self-verify workflow runs
  `vtest run --all` and `vtest verify` on this repository nightly and on demand;
  this repository does not yet pass its own `vtest verify`.
- Progress is tracked with GitHub
  [milestones](https://github.com/YmSaki/SpecTracer/milestones) and
  [issues](https://github.com/YmSaki/SpecTracer/issues), not in this file.

## Next: closing v0.1.0-beta.1 gaps and language execution

The core adapter separation from
[`docs/SpecTracer 言語アダプタ分離リファクタリング計画 v0.2.md`](docs/SpecTracer%20言語アダプタ分離リファクタリング計画%20v0.2.md)
(language-neutral domain, store, verify, and evidence ports, `rust-cargo`
isolation, `vtest-ops` CLI/MCP parity, and
[`docs/reports/adapter-acceptance-matrix.md`](docs/reports/adapter-acceptance-matrix.md))
is in place. Remaining work:

1. close the remaining adapter-boundary gaps (`W-ADAPTER-101` on missing
   capabilities, per-adapter Form conflict detection, and
   `adapters[].scan.assertion_macros` validation) and `v0.1.0-beta.1` known
   limitations (including `vtest approval --subject-type judgment` and
   `E-INTERNAL-001` panic handling);
2. bring this repository's own self-verification (`vtest run --all` +
   `vtest verify`) toward passing; and
3. provide the static reach of a target (DA-002) for Go (Go discovery, the `go
   test` runner, Go coverage and the Go static audit are done), and the
   specification of the JavaScript Test construct for the `javascript` adapter,
   which is still discovery only; and
4. add test-runner, coverage, freshness, and static-analysis capabilities for
   other languages only after canonical specification updates and reproducible
   acceptance coverage (promoting any of the ten experimental discovery-only
   adapters beyond discovery requires that evidence).

## Later opportunities

- additional tested release targets and package-manager integrations;
- native SDK interoperability for MCP if it becomes a supported requirement;
- ecosystem-specific adapters with explicit capability and `NOT_CHECKED` states;
- improved release provenance and reproducible build attestations.

These are not permission to weaken hash binding, source-of-truth ownership, or
fail-closed behavior. GUI features, automatic specification repair, and general
source-edit management remain outside the current product boundary.
