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
| M4 | Every registered Test execution writes one Evidence record | `m4_run_fast_records_one_evidence_per_registered_test`; `m4_multi_target_evidence_records_every_declared_target_hash` | PASS |
| M4 | A changed target makes `evidence_validity` STALE | `m4_target_mutation_makes_evidence_stale` | PASS |
| M4 | Build failure emits E-EXEC-001 and writes no Evidence | `m4_build_failure_reports_e_exec_001_without_evidence` | PASS |
| M4 | Ignored tests and missing result lines never publish Evidence | `m4_ignored_test_emits_no_evidence`; `m4_missing_result_line_emits_e_exec_002_without_evidence` | PASS |
| M4 | Evidence recency uses the actual RFC3339 instant | `evidence_recency_uses_the_actual_rfc3339_instant` | PASS |
| M5 | Test-semantic bundle contains every §8.2 field | `m5_bundles_include_schema_fields_for_all_audit_kinds` | PASS |
| M5 | Empty reasons are rejected with E-AUDIT-005 | `m5_empty_reasons_are_rejected_without_an_audit_record` | PASS |
| M5 | Post-bundle Test changes are rejected with E-AUDIT-002 | `m5_changed_test_rejects_submission_with_e_audit_002` | PASS |
| M5 | Accepted audit becomes STALE after its subject changes | `m5_accepted_audit_is_typed_and_becomes_stale_after_target_change` | PASS |
| M6 | Full-scope all-PASS fixture verifies OK with exit 0 | `m6_complete_fixture_is_ok_for_all_eleven_items` | PASS |
| M6 | Each of 11 non-PASS values independently forces NG/exit 1 | `m6_each_check_item_can_be_non_pass_without_aggregate_promotion` | PASS |
| M6 | Limited scope leaves every other item NOT_CHECKED | `m6_limited_scope_keeps_other_items_not_checked_and_text_is_tree_like` | PASS |
| M6 | Verify output tree matches Annex A §12.2 | `m6_entity_scope_selects_test_vo_and_report`; text tree assertion | PASS |
| M7 | A called target records PASS with count ≥ 1 | `m7_called_target_records_measured_pass` | PASS |
| M7 | A passing test that misses its target records FAIL/count 0 | `m7_passing_test_that_misses_target_records_measured_fail` | PASS |
| M7 | Missing cargo-llvm-cov emits W-EXEC-101 and NOT_CHECKED | `m7_missing_llvm_cov_is_warning_and_not_checked` | PASS |
| M8 | Invalid symbols are rejected with candidate-bearing E-OP-001 | `m8_invalid_symbol_is_rejected_with_candidates` | PASS |
| M8 | `test create` output is recognized on rescan | `m8_test_create_is_scanned_and_exposed_by_queries` | PASS |
| M8 | `test edit` changes no other Test bytes or hash | `m8_edit_changes_only_selected_test_and_preserves_other_hash` | PASS |
| M8 | Reapplying desired state is byte-idempotent | `m8_reapplying_same_edit_is_byte_idempotent` | PASS |
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

M4 records one append-only Evidence per observed registered Test. `--fast` emits
`target_execution: NOT_CHECKED`, captures the Test and every declared target
hash (`target_fns`), and preserves legacy single-target records as non-passing
for multi-target Tests. The acceptance flow covers PASS, ignored/no-result
non-execution, build failure (`E-EXEC-001`), target mutation (`STALE`), and
offset-aware RFC3339 recency. Repository dogfooding also generated a current
Evidence record and verified `static_audit`, `test_execution`, `runtime_result`,
and `evidence_validity` as PASS; `target_execution` remains NOT_CHECKED in fast
mode by design.

M5 exercises all three bundle kinds (`test-semantic`, `vo-coverage`, and
`impl-consistency`). Submissions preserve the canonical typed AuditRecord
schema (one id-or-locator subject per entry, exclusions, auditor, timestamp,
and revision), reject empty reasons, reject changed bundle subjects with
`E-AUDIT-002`, and become `STALE` when an accepted target changes.

M6 evaluates all eleven checks fail-closed over both item and REQ/VO/Test
scopes. The complete fixture proves an all-PASS result only after current
static/semantic/implementation audits, an approved VO, valid Evidence, and a
measured target execution are present. The matrix proves each individual
non-PASS state yields exit 1; missing leaf coverage and partially executed
Tests cannot be masked by another PASS. JSON reports now include a deterministic
REQ → VO → Test tree with audit/Evidence basis references, while text reports
render the same tree and explicitly label scope-outside values as
`NOT_CHECKED`.

M7 exercises the default measured execution path in a disposable real-process
fixture. A target reached through the passing Test records `cargo-llvm-cov`,
`checked: true`, and a positive count; a statically retained but uncalled target
records `FAIL` with count 0. A toolchain wrapper that makes only
`cargo llvm-cov --version` unavailable falls back to `cargo-test`, emits
`W-EXEC-101`, and records `NOT_CHECKED` without promoting the result.

M8 exercises the public Structured Test Operation commands against disposable
projects. Invalid source symbols return `E-OP-001` with candidates; `test
create` supports dry-run and produces a Test visible to scan/show/list/query;
editing one Test preserves another Test's content hash; and repeating the same
desired `covers` state produces no byte changes.

M9 now has a transport smoke gate: `m9_acceptance` starts the MCP stdio server,
checks initialization and the complete tool registry, compares a `scan` tool
result byte-for-byte with the CLI envelope, runs the create/query/audit/run
reference path, and verifies candidate-bearing invalid input. The three full
M9 criteria (all-tool parity matrix, complete reference flow, and every
transport error case) remain `NOT_CHECKED` until their dedicated matrix is
implemented.
