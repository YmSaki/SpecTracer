# Acceptance Test Ledger

This ledger maps normative Annex C §18.3 to reproducible tests. A criterion remains
`NOT_CHECKED` until its milestone-specific fixture flow passes; unit coverage
alone is not promoted to acceptance evidence.

## Adapter separation acceptance freeze

The adapter-separation scenarios are frozen against baseline
`575ea724ad2ec6977d3a26bcb6a18e7192a4eb6d`. Status is evidence-specific:

- `FROZEN_RED`: a criterion-specific executable assertion fails on the baseline.
- `REGRESSION_LOCKED`: an existing assertion already produces the required value.
- `W1_BOUND_RED`: a W1 API-level assertion now executes and passes for the frozen
  input/observable shape, while the owning later-wave product assertion remains RED.

Production code is unchanged by this freeze.

| ID | Specification clause | Acceptance criterion / observable | Test / fixture | Baseline actual | Freeze status |
|---|---|---|---|---|---|
| AF-001 | Annex C §18.3.9 | All neutral hash-free draft types exist with the normative field sets in a Rust-neutral API crate | `adapter_api_crate_exposes_every_neutral_draft_type`; `adapter_api_compile_contract_type_checks`; `api-contract/` | missing package | FROZEN_RED |
| AF-002 | Annex C §§18.2, 18.3.9 | Non-`.rs`, non-function construct and exact current byte range | `adapter_boundary_fixture_is_non_rust_and_non_adjacent`; `adapters/synthetic` | valid `65..176` range | REGRESSION_LOCKED |
| AF-003 | Annex C §18.3.9 | Core model has opaque locations, execution descriptor, fixed 12; no Rust coordinates | `core_model_is_neutral_and_has_the_fixed_check_set`; compile-contract field destructuring | neutral types absent | FROZEN_RED |
| AF-004 | Annex C §§18.2, 18.3.9 | Non-adjacent metadata changes Test subject and freshness | model subject-hash tests; `tests.json` → `tests.changed.json` mutation | W1 core hash detects logical metadata mutation; freshness integration remains RED | W1_BOUND_RED |
| AF-005 | Annex C §18.3.1 | Unregistered Test → traceability `MISSING` | `unregistered_test_is_traceability_missing` | unknown item / exit 2 | FROZEN_RED |
| AF-006 | Annex C §18.3.1 | Empty covers → traceability `MISSING` | `empty_covers_is_traceability_missing` | unknown item / exit 2 | FROZEN_RED |
| AF-007 | Annex C §18.3.1 | Dangling VO remains managed-one → `MISMATCH` | `dangling_vo_is_traceability_mismatch` | unknown item / exit 2 | FROZEN_RED |
| AF-008 | Annex C §18.3.1 | Duplicate Test ID → `MISMATCH` | `duplicate_test_id_is_traceability_mismatch` | unknown item / exit 2 | FROZEN_RED |
| AF-009 | Annex C §18.3.1 | Cross-adapter Test/SRC collision is diagnosed and unresolved | `collision_fixture_proves_repository_global_ids_are_not_namespaced`; product collision test | W1 binding proves global duplicate input; W3 product diagnostic remains RED | W1_BOUND_RED |
| AF-010 | Annex C §§18.3.5, 18.3.9 | v1 11-item scope becomes fixed 12 without rewrite | `default_verify_evaluates_the_fixed_twelve_items` | 11 items | FROZEN_RED |
| AF-011 | Annex C §18.3.5 | v1 missing full_scope becomes fixed 12 without rewrite | `version_one_without_full_scope_uses_fixed_twelve_without_rewrite` | 11 items | FROZEN_RED |
| AF-012 | Annex C §§18.3.5, 18.3.9 | v2 complete accepted; incomplete/duplicate scope → `E-CONFIG-001` | `version_two_config_is_accepted_and_incomplete_scope_is_rejected`; `version_two_duplicate_full_scope_is_e_config_001` | invalid scopes accepted | FROZEN_RED |
| AF-013 | Annex C §18.3.9 | unknown, duplicate, or zero adapters reject scan with exit 2 and no result | three adapter-registry tests | scan succeeds | FROZEN_RED |
| AF-014 | Annex C §18.3.3 | Evidence without Execution State → `STALE` | `evidence_without_execution_state_is_compatibility_stale` | remains `PASS` | FROZEN_RED |
| AF-015 | Annex C §18.3.3 | incomplete current execution snapshot → `UNKNOWN` | `incomplete_current_execution_snapshot_is_unknown` | not `UNKNOWN` | FROZEN_RED |
| AF-016 | Annex C §18.3.2 | consulted helper-only change stales static Audit | `static_helper_only_change_stales_the_audit_record` | `STALE` | REGRESSION_LOCKED |
| AF-017 | Annex C §18.3.2 | assertion macro change stales static Audit | `assertion_macro_change_stales_the_static_audit_record` | `STALE` | REGRESSION_LOCKED |
| AF-018 | Annex C §18.3.2 | run-only config does not stale static Audit | `static_audit_ignores_run_only_config_changes` | incorrectly `STALE` | FROZEN_RED |
| AF-019 | Annex C §18.3.3 | neutral Evidence binds adapter, Test, every target, and complete Execution State | `evidence_contains_neutral_subjects_and_complete_execution_state` | fields absent | FROZEN_RED |
| AF-020 | Annex C §18.3.3 | target-external helper change stales Evidence | `target_external_helper_change_stales_evidence` | remains `PASS` | FROZEN_RED |
| AF-021 | Annex C §18.3.3 | local dependency change stales Evidence | `local_dependency_change_stales_evidence` | remains `PASS` | FROZEN_RED |
| AF-022 | Annex C §18.3.3 | pre/post state mutation → `E-EXEC-004`, no Evidence | `execution_state_mutation_reports_e_exec_004_without_evidence` | Evidence written | FROZEN_RED |
| AF-023 | Annex C §18.3.3 | missing revision commit → `STALE` | `evidence_without_revision_commit_is_stale` | maps to `FAIL` | FROZEN_RED |
| AF-024 | Annex C §18.3.3 | HEAD mismatch stales unchanged Test/targets | `head_change_without_test_or_target_change_stales_evidence` | remains `PASS` | FROZEN_RED |
| AF-025 | Annex C §18.3.4 | Specification-only change stales impl-consistency | `specification_only_change_stales_impl_consistency` | remains `PASS` | FROZEN_RED |
| AF-026 | Annex C §18.3.4 | impl-consistency FAIL maps to `MISMATCH` | `impl_consistency_fail_maps_to_mismatch` | not `MISMATCH` | FROZEN_RED |
| AF-027 | Annex C §18.3.1 | Specification dependency change invalidates Approval | `specification_dependency_change_invalidates_vo_approval` | remains approved | FROZEN_RED |
| AF-028 | Annex C §§18.3.3, 18.3.6 | multi-target Evidence has every neutral target/result | `multi_target_evidence_keeps_target_specific_results` | neutral entries absent | FROZEN_RED |
| AF-029 | Annex C §18.3.6 | target aggregation is `FAIL > UNKNOWN > PASS` | `target_observation_fixture_has_no_representative_target_escape_hatch`; product aggregation test | W1 observation shape binds all three targets; W6 aggregation remains RED | W1_BOUND_RED |
| AF-030 | Annex C §18.3.5 | limited scope leaves outside items `NOT_CHECKED` | existing M6 test | PASS | REGRESSION_LOCKED |
| AF-031 | Annex C §18.3.5 | deterministic text/JSON tree | existing M6/M9 tests | PASS | REGRESSION_LOCKED |
| AF-032 | Annex C §18.3.7 | explicit Form owner; duplicate/ambiguous owner rejected without Rust fallback | init owner test; `form_owner_fixtures_forbid_ambiguous_or_rust_fallback_resolution` | W1 matcher/owner observable bound; W2/W3 product paths remain RED | W1_BOUND_RED |
| AF-033 | Annex C §§18.1, 18.3.8 | CLI/MCP share fixed-12 envelope | `cli_and_mcp_default_verify_share_the_fixed_contract` | parity only at 11 | FROZEN_RED |
| AF-034 | Annex C §§18.3.2, 18.3.3, 18.3.9 | missing audit/coverage/runner → `NOT_CHECKED`/`NOT_EXECUTED`; limits → `UNKNOWN` | `capability_absence_and_analysis_limits_bind_to_non_pass_states`; synthetic manifests | W1 missing-capability mapping is executable; W4–W6 aggregation remains RED | W1_BOUND_RED |
| AF-035 | Annex C §18.3.9; plan §9.2 | forbidden direct Rust dependencies are absent by `cargo metadata` | `orchestration_crates_have_no_direct_rust_analysis_dependencies` | direct deps remain | FROZEN_RED |
| AF-036 | Annex C §18.3.5 | Specification requirement without active REQ is non-PASS | `specification_requirement_without_active_req_is_non_pass` | `MISSING` | REGRESSION_LOCKED |
| AF-037 | Annex C §18.3.9 | Rust + synthetic merge and adapter/filesystem ordering are deterministic | registry unit tests; `frozen_ordering_variants_have_one_canonical_observable` | W1 registry/order binding is deterministic; W7 mixed merge remains RED | W1_BOUND_RED |
| AF-038 | Annex C §18.3.1; plan §9.1 | bare Relation normalizes in memory without rewrite; bare/prefixed duplicate is rejected | `relation_aliases_bind_to_one_in_memory_identity_without_rewrite`; duplicate product test | W1 binding and duplicate rejection execute; W2 canonical reader integration remains RED | W1_BOUND_RED |
| AF-039 | Annex C §§18.3.1, 18.3.9 | incomplete adapter discovery is an error, never a complete empty scan | `incomplete_discovery_is_representable_and_never_complete_empty_success`; product discovery test | W1 completeness state is executable; W3 orchestration remains RED | W1_BOUND_RED |
| AF-040 | Annex C §18.3.9; plan §9.1 | v1 and v2 Rust scan/audit/run/verify observations are semantically equal | `v2_rust_fixture_binds_to_the_same_neutral_adapter_id_as_v1_compatibility`; v1/v2 product flow | W1 adapter identity is bound; W2 codec/orchestration equivalence remains RED | W1_BOUND_RED |

| AF-041 | 詳細設計 §3.5; Annex C §18.3.1 | The canonical VO record stores no approval-derived `status`; the effective value comes from Approvals | `canonical_vo_record_never_stores_the_derived_status`; `calc/m1/base` | writer emits `status: 'draft'` | FROZEN_RED |
| AF-042 | 詳細設計 §3.5, §7 (W-STORE-002); Annex C §18.3.1 | An Approval lacking the upstream dependency closure never derives `approved` and is reported | `approval_without_a_dependency_closure_is_reported_and_never_approves`; `calc/m1/base` | derives `approved`; no `W-STORE-002` exists | FROZEN_RED |

Baseline command:

```text
cargo test -p vtest-cli --test adapter_acceptance
```

### AF-041 / AF-042 freeze record

These two criteria were added after the initial freeze: the contract stated them
but the ledger did not carry them. The classification below was measured by
running each test at `075313a`, whose production tree is byte-identical to
baseline `575ea72` (the freeze commits touch tests and fixtures only).

| Field | AF-041 | AF-042 |
|---|---|---|
| Specification clause | 詳細設計 §3.5「VOの実効`status`は…canonical VO recordへ保存しない」; Annex C §18.3.1 | 詳細設計 §3.5「依存entryを持たない互換Approvalは…現在の`approved`を導出しない。W-STORE-002を出し、VOは`draft`相当とする」 |
| Criterion | `vo edit` rewrites the record without a `status` key | A compatibility Approval with a current `subject_hash` and no `dependencies` is inert |
| Observable | `.verify/vo/VO-KNOWN.yaml` has no `status:` line; `vo show` reports `effective_status = draft` | `vo show` reports `effective_status = draft`; `scan` diagnostics contain `W-STORE-002` |
| Minimal counterexample / fixture | `calc/m1/base`, then `vo edit VO-KNOWN --claim …` | `calc/m1/base` plus a hand-written Approval whose `subject_hash` is the current VO content hash and whose `dependencies` key is absent |
| Expected result | no `status:` key; `draft` | `draft`; `W-STORE-002` present |
| Baseline actual (`075313a`) | record contains `status: 'draft'` | `effective_status = approved`; `W-STORE-002` absent from the codebase |
| Baseline classification | FROZEN_RED | FROZEN_RED |
| Owning wave | W2 | W2 |

All formerly `API_BINDING_REQUIRED` rows now have executable W1 binding
assertions. `W1_BOUND_RED` is not product PASS evidence: the owning wave must
still make its criterion-specific product assertion green. The suite must not be
described as implementation PASS until every row is executable and green in
its owning wave.

## Pre-adapter regression ledger

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
| M9 | Every MCP tool matches the CLI JSON shape | `m9_all_advertised_tools_match_cli_envelopes` | PASS |
| M9 | Annex A §13.3 completes over MCP stdio | `m9_reference_flow_completes_over_mcp_stdio` | PASS |
| M9 | Invalid input returns code/message/candidates | `m9_protocol_and_error_matrix_is_fail_closed_without_writes` | PASS |

`PASS*` follows the authoritative observable contract: basic specification §11
maps exit 1 to NG, detailed design §5.4 says warnings do not change verification,
and Annex A §12.2 requires exit 1 for **error** diagnostics. Annex B §18.3 uses
the broader phrase “diagnostic present → 1”; that wording conflict remains a
specification review item. The regression test deliberately fixes warning-only
scan at exit 0 and error-bearing scan at exit 1.

M2 also fixes append-only publication, strict Approval/Relation validation,
explicit combinations, tree listing, and `vo show` coverage/approval output in
`m2_acceptance` and `vtest-store` tests. The current contract requires canonical
Relation writers to emit `REL-<ULID>` while normalizing consistent bare v1 input
in memory without rewriting it. Approval effectiveness is bound to both the VO
hash and the complete current upstream dependency closure; the adapter freeze
rows supersede historical VO-hash-only regression evidence.

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

M9 is covered by a transport and parity gate: `m9_acceptance` starts the MCP
stdio server, checks initialization and the complete 22-tool registry with
focused schemas, compares applicable tool envelopes and values with the CLI
(normalizing only generated record IDs/timestamps), runs the complete
specification/requirement/VO/approval/form/test/audit/run/verify/report flow,
and exercises malformed JSON-RPC, unsupported methods, invalid arguments,
candidate-bearing symbols, and rejected audit submissions. Rejected inputs
are checked for `isError` envelopes and absence of unintended audit writes.
