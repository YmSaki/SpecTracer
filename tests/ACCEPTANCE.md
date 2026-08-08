# Acceptance Test Ledger

This ledger maps Annex B §18.3 to reproducible tests. A criterion remains
`NOT_CHECKED` until its milestone-specific fixture flow passes; unit coverage
alone is not promoted to acceptance evidence.

| Milestone | Criterion | Reproducible evidence | Status |
|---|---|---|---|
| S0 | Eight-crate workspace and one-way dependency baseline | `cargo test --workspace`; architecture review | PASS |
| S0 | Canonical `.verify/` layout and built-in forms | `init_creates_project_and_second_init_is_usage_error` | PASS |
| S0 | Shared hashes, diagnostics, JSON envelope, and exit-code types | `vtest-model` unit tests; M1 CLI acceptance | PASS |
| S0 | Annex B calc fixture and M1 case variants are tracked | `tests/fixtures/calc/` | PASS |
| M1 | Extract every fixture Test with exact `filter`, `package`, and `test_target` | `m1_calc_fixture_extracts_tests_and_scan_matches_doctor` | PASS |
| M1 | Detect E-SCAN-002..010 and W-SCAN-101 at their source files | `m1_error_diagnostic_matrix_is_reported_by_the_cli`; `m1_warning_only_scan_exits_zero` | PASS |
| M1 | Emit the §12.1 JSON envelope and location-bearing scan diagnostics | all tests in `crates/vtest-cli/tests/m1_acceptance.rs` | PASS |
| M1 | Distinguish success, verification, usage, and internal exits (0/1/2/3) | M1 clean/warning, error matrix, repeated-init, and invalid-config cases | PASS* |
| M2 | VO add → approve → edit invalidates approval and derives draft | `m2_vo_edit_invalidates_approval_and_returns_to_effective_draft` | PASS |
| M2 | `vo expand --dry-run` returns the `full-product` Cartesian children | `m2_full_product_expand_dry_run_lists_cartesian_children_without_writes` | PASS |
| M2 | Editing a registered SPEC document emits W-SCAN-104 | `m2_mutating_registered_spec_document_reports_w_scan_104` | PASS |
| M3 | Each intentional NG test maps to DA-001..006 / W-DA-101 | `m3_static_audit_maps_failures_preserves_unknown_and_warns_for_ignored_tests` | PASS |
| M3 | The normal test has no deterministic violation | normal and configured-macro cases in `m3_acceptance` | PASS |
| M3 | A cross-file call is UNKNOWN, never a certain DA-002 FAIL | cross-file, helper-boundary, homonym, import-alias, and local-shadow regressions | PASS |
| M4 | Every registered Test execution writes one Evidence record | add `vtest run --fast` fixture flow | NOT_CHECKED |
| M4 | A changed target makes `evidence_validity` STALE | add before/after content-hash flow | NOT_CHECKED |
| M4 | Build failure emits E-EXEC-001 and writes no Evidence | add broken-build fixture | NOT_CHECKED |
| M5 | Test-semantic bundle contains every §8.2 field | add bundle schema assertion | NOT_CHECKED |
| M5 | Empty reasons are rejected with E-AUDIT-005 | add submit rejection fixture | NOT_CHECKED |
| M5 | Post-bundle Test changes are rejected with E-AUDIT-002 | add stale-bundle submit flow | NOT_CHECKED |
| M5 | Accepted audit becomes STALE after its subject changes | add audit/modify/verify flow | NOT_CHECKED |
| M6 | Full-scope all-PASS fixture verifies OK with exit 0 | add complete canonical fixture | NOT_CHECKED |
| M6 | Each of 11 non-PASS values independently forces NG/exit 1 | add table-driven aggregation matrix | NOT_CHECKED |
| M6 | Limited scope leaves every other item NOT_CHECKED | add scoped verify/report assertion | NOT_CHECKED |
| M6 | Verify output tree matches Annex A §12.2 | add JSON and text tree snapshots | NOT_CHECKED |
| M7 | A called target records PASS with count ≥ 1 | promote llvm-cov parser tests into an execution fixture | NOT_CHECKED |
| M7 | A passing test that misses its target records FAIL/count 0 | add measured no-call fixture | NOT_CHECKED |
| M7 | Missing cargo-llvm-cov emits W-EXEC-101 and NOT_CHECKED | add isolated-toolchain CLI flow | NOT_CHECKED |
| M8 | Invalid symbols are rejected with candidate-bearing E-OP-001 | add CLI process acceptance | NOT_CHECKED |
| M8 | `test create` output is recognized on rescan | promote `structured_create_generates_a_scannable_test` | NOT_CHECKED |
| M8 | `test edit` changes no other Test bytes or hash | promote the existing two-Test boundary assertion | NOT_CHECKED |
| M8 | Reapplying desired state is byte-idempotent | promote the existing idempotence assertion | NOT_CHECKED |
| M9 | Every MCP tool matches the CLI JSON shape | implement MCP parity matrix | NOT_CHECKED |
| M9 | Annex A §13.3 completes over MCP stdio | implement reference-flow integration test | NOT_CHECKED |
| M9 | Invalid input returns code/message/candidates | implement MCP error-contract test | NOT_CHECKED |

`PASS*` follows the authoritative observable contract: basic specification §11
maps exit 1 to NG, detailed design §5.4 says warnings do not change verification,
and Annex A §12.2 requires exit 1 for **error** diagnostics. Annex B §18.3 uses
the broader phrase “diagnostic present → 1”; that wording conflict remains a
specification review item. The regression test deliberately fixes warning-only
scan at exit 0 and error-bearing scan at exit 1.

M2 also fixes append-only publication, strict Approval/Relation validation,
explicit combinations, tree listing, and `vo show` coverage/approval output in
`m2_acceptance` and `vtest-store` tests. Two specification review items remain:

- Basic specification §3.1 writes Relation as `REL-` (ULID), while its §3.2 and
  detailed design §§2.1/3.4 use a bare ULID filename. The reader validates and
  accepts either spelling, while scan rejects a bare/prefixed pair sharing one
  payload; this preserves one logical identity per ULID.
- Basic specification §9 and detailed design §3.5 derive approval only from the
  current VO hash, while detailed design §§3.1/11.4 also say a dependent SPEC
  change invalidates approval. M2 follows the higher-precedence VO-hash formula
  and reports SPEC drift with W-SCAN-104; dependency-bound approval needs a
  specification/schema decision before implementation.

M3 persists one typed, append-only static AuditRecord per Test. A current record
must contain exactly one Test ID, all DA-001..006 results, the raw config hash,
the Test-code locator/hash, and the exact declared target locator/hash. Missing,
malformed, ambiguous, or stale inputs cannot produce PASS. The real-process M3
matrix also covers `--all`, deterministic diagnostics, configured macros,
source IDs, one-hop helpers, and canonical record round-tripping. Repository
dogfooding registered `TEST-DOGFOOD-M3-TARGET-RULES`; its six rules and scoped
`static_audit` verification pass, while five earlier immutable trial records
are reported as stale and ignored.
