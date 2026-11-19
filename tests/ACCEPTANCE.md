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

M9 is covered by a transport and parity gate: `m9_acceptance` starts the MCP
stdio server, checks initialization and the complete 22-tool registry with
focused schemas, compares applicable tool envelopes and values with the CLI
(normalizing only generated record IDs/timestamps), runs the complete
specification/requirement/VO/approval/form/test/audit/run/verify/report flow,
and exercises malformed JSON-RPC, unsupported methods, invalid arguments,
candidate-bearing symbols, and rejected audit submissions. Rejected inputs
are checked for `isError` envelopes and absence of unintended audit writes.

## Issue #74: Execution State adapter boundary

The following focused observations cover the Execution State collection/hash
boundary. They do not certify unrelated historical milestone claims above.

| Contract | Observation |
|---|---|
| DES-097–101, DES-352–366 | `vtest-store::execution_state` validates neutral DTOs, binds byte-exact inputs and projections, sorts manifest identities, and rejects duplicate or non-relative identities. |
| DES-208–212 | `execution_state_adapter` covers readable manifests, local dependency bytes, unresolved/external inputs, build scripts, and include escape risks. Unsupported external roots and symbolic inputs remain incomplete. |
| DES-370 | `canonical_invocation_is_relocation_independent_and_binds_selector_and_mode` compares snapshots from two physical roots and changes selector/runner mode. |
| DS-1375, DES-099, DES-371 | `execution_input_change_rejects_evidence_and_stable_inputs_are_recorded` exercises eight synthetic-runner cases: complete/incomplete snapshots, unchanged/reversed manifest order, and stable/changed bytes. Order alone preserves Evidence; changed bytes yield `E-EXEC-004` and zero Evidence writes even when hashes are unavailable. Incomplete snapshots remain incomplete with no hash. |
| DS-816–822, DES-372 | `evidence_e2e` exercises current Rust adapter snapshots through writer and verifier; verifier unit tests distinguish incomplete reconstruction from a stale hash. |

The corrected hash encoding binds schema ID/version and canonical invocation /
config projections separately. Previously recorded hashes may become STALE and
need a fresh execution; the Evidence wire shape and snapshot schema name remain
unchanged. Single-workspace-root collection remains a supported subset: external
local path dependencies report incomplete snapshots rather than a complete hash.

## Issue #71: test specification output and `vtest test show / list / query`

Each clause of DS-S164 (§18.3.13) and DS-S167 (§18.3.14) is mapped to the test
that observes it. The clauses that name the shipped `vtest` run the binary; the
MCP clauses go through the `vtest mcp` stdio transport; the clauses that name
the acceptance-only registry call the operation in-process with `rust-cargo`
and `synthetic` registered (`tests/support/spec_acceptance.rs`), because
`synthetic` is not in the shipped registry. `vtest run` is called with `--fast`,
so no observation depends on `cargo-llvm-cov`.

| Clause | Test | Status |
|---|---|---|
| DS-1780 | `a_mixed_project_exports_the_tests_of_both_adapters_in_one_form` (`acceptance_18_3_13.rs`) | PASS |
| DS-1781 | `a_mixed_project_exports_the_tests_of_both_adapters_in_one_form` (`acceptance_18_3_13.rs`) | PASS |
| DS-1782 | `constructs_without_a_complete_declaration_are_exported_as_unmanaged` (`acceptance_18_3_13.rs`) | PASS |
| DS-1783 | `constructs_without_a_complete_declaration_are_exported_as_unmanaged` (`acceptance_18_3_13.rs`) | PASS |
| DS-1784 | `constructs_without_a_complete_declaration_are_exported_as_unmanaged` (`acceptance_18_3_13.rs`) | PASS |
| DS-1785 | `unresolved_references_are_exported_and_not_dropped` (`acceptance_18_3_13.rs`) | PASS |
| DS-1786 | `unresolved_references_are_exported_and_not_dropped` (`acceptance_18_3_13.rs`) | PASS |
| DS-1787 | `unresolved_references_are_exported_and_not_dropped` (`acceptance_18_3_13.rs`) | PASS |
| DS-1788 | `unresolved_references_are_exported_and_not_dropped` (`acceptance_18_3_13.rs`) | PASS |
| DS-1789 | `two_tests_with_one_id_are_both_exported_with_numbered_keys` (`acceptance_18_3_13.rs`) | PASS |
| DS-1790 | `the_export_does_not_depend_on_the_run_or_on_enumeration_order` (`acceptance_18_3_13.rs`) | PASS |
| DS-1791 | `the_export_does_not_depend_on_the_run_or_on_enumeration_order` (`acceptance_18_3_13.rs`) | PASS |
| DS-1792 | `values_that_look_like_markdown_do_not_change_the_document_structure` (`acceptance_18_3_13.rs`) | PASS |
| DS-1793 | `the_mcp_export_tool_returns_what_the_cli_prints` (`acceptance_18_3_13.rs`) | PASS |
| DS-1794 | `evidence_approvals_and_judgments_do_not_change_the_export` (`acceptance_18_3_13.rs`) | PASS |
| DS-1795 | `evidence_approvals_and_judgments_do_not_change_the_export` (`acceptance_18_3_13.rs`) | PASS |
| DS-1796 | `export_leaves_the_project_files_as_they_were` (`acceptance_18_3_13.rs`) | PASS |
| DS-1797 | `a_project_without_document_records_is_exported` (`acceptance_18_3_13.rs`) | PASS |
| DS-1798 | `a_rejected_config_and_quiet_keep_the_exit_code_of_export` (`acceptance_18_3_13.rs`) | PASS |
| DS-1799 | `a_rejected_config_and_quiet_keep_the_exit_code_of_export` (`acceptance_18_3_13.rs`) | PASS |
| DS-1800 | `export_rejects_formats_and_arguments_outside_its_contract` (`acceptance_18_3_13.rs`) | PASS |
| DS-1801 | `the_mcp_export_tool_returns_what_the_cli_prints` (`acceptance_18_3_13.rs`) | PASS |
| DS-1802 | `export_rejects_formats_and_arguments_outside_its_contract` (`acceptance_18_3_13.rs`) | PASS |
| DS-1803 | `export_rejects_formats_and_arguments_outside_its_contract` (`acceptance_18_3_13.rs`) | PASS |
| DS-1804 | `export_rejects_formats_and_arguments_outside_its_contract` (`acceptance_18_3_13.rs`) | PASS |
| DS-1805 | `export_rejects_formats_and_arguments_outside_its_contract` (`acceptance_18_3_13.rs`) | PASS |
| DS-1806 | `the_mcp_export_tool_returns_what_the_cli_prints` (`acceptance_18_3_13.rs`) | PASS |
| DS-1807 | `the_mcp_export_tool_returns_what_the_cli_prints` (`acceptance_18_3_13.rs`) | PASS |
| DS-1808 | `markdown_values_are_encoded_without_losing_a_character` (`acceptance_18_3_13.rs`) | PASS |
| DS-1809 | `markdown_values_are_encoded_without_losing_a_character` (`acceptance_18_3_13.rs`) | PASS |
| DS-1810 | `markdown_values_are_encoded_without_losing_a_character` (`acceptance_18_3_13.rs`) | PASS |
| DS-1811 | `the_shipped_export_prints_the_data_and_document_of_the_operation` (`acceptance_18_3_13.rs`) | PASS |
| DS-1837 | `list_agrees_with_the_test_specification` (`acceptance_18_3_14.rs`) | PASS |
| DS-1838 | `list_vo_returns_the_declaring_tests_only` (`acceptance_18_3_14.rs`) | PASS |
| DS-1839 | `list_agrees_with_the_test_specification` (`acceptance_18_3_14.rs`) | PASS |
| DS-1840 | `list_vo_returns_the_declaring_tests_only` (`acceptance_18_3_14.rs`) | PASS |
| DS-1841 | `query_resolves_the_source_before_comparing` (`acceptance_18_3_14.rs`) | PASS |
| DS-1842 | `query_resolves_the_source_before_comparing` (`acceptance_18_3_14.rs`) | PASS |
| DS-1843 | `show_returns_what_verify_returns_for_the_test` (`acceptance_18_3_14.rs`) | PASS |
| DS-1844 | `show_returns_what_verify_returns_for_the_test` (`acceptance_18_3_14.rs`) | PASS |
| DS-1845 | `the_mcp_tools_return_what_the_cli_prints` (`acceptance_18_3_14.rs`) | PASS |
| DS-1846 | `the_mcp_tools_return_what_the_cli_prints` (`acceptance_18_3_14.rs`) | PASS |
| DS-1847 | `the_mcp_tools_return_what_the_cli_prints` (`acceptance_18_3_14.rs`) | PASS |
| DS-1848 | `the_query_commands_and_tools_leave_the_project_files_as_they_were` (`acceptance_18_3_14.rs`) | PASS |
| DS-1861 | `list_and_query_handle_both_adapters_of_a_mixed_project` (`acceptance_18_3_14.rs`) | PASS |
| DS-1862 | `list_and_query_handle_both_adapters_of_a_mixed_project` (`acceptance_18_3_14.rs`) | PASS |
| DS-1863 | `list_and_query_handle_both_adapters_of_a_mixed_project` (`acceptance_18_3_14.rs`) | PASS |
| DS-1864 | `list_and_query_handle_both_adapters_of_a_mixed_project` (`acceptance_18_3_14.rs`) | PASS |
| DS-1865 | `the_query_commands_reject_arguments_outside_their_forms` (`acceptance_18_3_14.rs`) | PASS |
| DS-1866 | `the_query_commands_reject_arguments_outside_their_forms` (`acceptance_18_3_14.rs`) | PASS |
| DS-1867 | `list_and_query_keep_their_result_next_to_a_scan_error` (`acceptance_18_3_14.rs`) | PASS |
| DS-1868 | `text_lists_one_test_per_line_and_quiet_prints_nothing` (`acceptance_18_3_14.rs`) | PASS |
| DS-1869 | `text_lists_one_test_per_line_and_quiet_prints_nothing` (`acceptance_18_3_14.rs`) | PASS |
| DS-1870 | `show_reports_the_state_of_the_judgment_records` (`acceptance_18_3_14.rs`) | PASS |
| DS-1871 | `show_reports_the_state_of_the_judgment_records` (`acceptance_18_3_14.rs`) | PASS |
| DS-1872 | `show_reports_the_state_of_the_judgment_records` (`acceptance_18_3_14.rs`) | PASS |
| DS-1873 | `show_reports_the_latest_evidence_and_prints_every_field_as_text` (`acceptance_18_3_14.rs`) | PASS |
| DS-1874 | `show_reports_the_latest_evidence_and_prints_every_field_as_text` (`acceptance_18_3_14.rs`) | PASS |
| DS-1875 | `the_mcp_tools_return_what_the_cli_prints` (`acceptance_18_3_14.rs`) | PASS |
| DS-1876 | `the_mcp_tools_return_what_the_cli_prints` (`acceptance_18_3_14.rs`) | PASS |
| DS-1877 | `show_reports_the_latest_evidence_and_prints_every_field_as_text` (`acceptance_18_3_14.rs`) | PASS |

## Issue #294: DA-001 constancy of a failure divergence

`rust-cargo` collects each failure divergence (DS-1951) with the positions that
decide whether it runs (DS-1947, DS-1983). DA-003 counts a target result that
flows to one of them (`TEST-ORACLE-DIVERGENCE-DECIDING-POSITION-REACH`,
`TEST-ORACLE-FAILURE-DIVERGENCE-CONSTRUCTS-COUNT`), except after an irrefutable
arm (DS-1952, `TEST-ORACLE-DIVERGENCE-AFTER-IRREFUTABLE-ARM-NOT-REACHED`), under
a constant `false` (DS-1953, `TEST-ORACLE-DIVERGENCE-UNDER-CONSTANT-FALSE-NOT-REACHED`),
where syntax does not fix the reach (DS-1963,
`TEST-ORACLE-DIVERGENCE-REACH-UNDECIDABLE-IS-UNKNOWN`), and where an earlier exit
leaves it undecided (DS-1983, `TEST-ORACLE-DIVERGENCE-AFTER-EXIT-DECIDED-BY-EXIT`).
DA-001 judges `panic!`, `unreachable!`, `todo!`, `unimplemented!` and
`assert!(<fixed false>)` by those positions (DS-1950,
`TEST-ORACLE-DA-001-DIVERGENCE-CONSTANCY-BY-DECIDER`,
`TEST-ORACLE-DA-001-UNCONDITIONAL-DIVERGENCE-IS-CONSTANT`).

| Clause | Gap | Status |
|---|---|---|
| DS-1950 | A divergence that is not reached while one of its deciding positions is not constant (`while false { if r != 4 { panic!() } }`, `for _ in 0..0 { .. }`, after an unconditional `return`) is constant; DA-001 reports `UNKNOWN`. | NOT_CHECKED |
| DS-1950 | Under `#[should_panic]`, a divergence with a non-constant deciding position is not constant by DS-1950, yet one on every path (`if double(2) != 4 { panic!() } else { panic!() }`) passes whatever the target returns; DA-001 reports `UNKNOWN` for every such divergence, also where some path avoids them. | NOT_CHECKED |
| DS-1950 | An unconditional `return Err(..)` (DS-1951) is not counted by DA-001, which still reads only assert macros, so it is not reported as a constant assertion. | NOT_CHECKED |
| DS-1947 | A guard that reads a name its arm binds (`x if x == 4`) is not traced to the scrutinee, so DA-003 reports `UNKNOWN`. | NOT_CHECKED |
| DS-1947 | A pattern identifier starting with an uppercase letter, a struct / tuple-struct / fixed-length slice pattern before the arm, a literal-only guard or condition whose value is not evaluated, and a `for` over anything but a range of two integer literals or an array literal leave the reach undecided, so DA-003 and DA-001 report `UNKNOWN`. | NOT_CHECKED |
| DS-1947 | A pattern that checks a literal element of the scrutinee (`(_, 1)` over `(x, 1)`), or elements of more than one scrutinee element together (`(4, 2)` over `(double(2), k)`), leaves its arm and the later ones undecided, so DA-003 and DA-001 report `UNKNOWN`. | NOT_CHECKED |
| DS-1947 | A divergence on the right side of `&&` / `||`, or in one expression beside an exit, is undecided, so DA-003 and DA-001 report `UNKNOWN`. | NOT_CHECKED |
| DS-1947 | The variant check of a sub-position of a binding that holds the target's result only in part is not read, so DA-003 reports `UNKNOWN`. | NOT_CHECKED |
| DS-1983 | Any method call in a loop body is read as changing what the loop condition reads, so the reach is undecided and DA-003 reports `UNKNOWN`. | NOT_CHECKED |
| DS-1983 | A `?` is not read as a deciding position, and a divergence after it is undecided, so DA-003 and DA-001 report `UNKNOWN`. | NOT_CHECKED |
| DS-1947 | Remaining false PASS: complementary conditions (`if c {} else if !c {} else { panic!() }`) are not compared, so DA-003 counts the last `else` as reached. | NOT_CHECKED |
| DS-1963 | The pattern of an `if let` and of a `let ... else` (with the patterns refuted on the path to it) is also read by DS-1963 (2), which names `match` arms only: one that may match every value at a position leaves the `else` undecided even where the value is not exhausted (`let Some(Kind::A(x)) = f() else { .. }`), so DA-003 and DA-001 report `UNKNOWN`; three Tests of this repository (`TEST-ADAPTER-LEXICAL-ANNOTATION-GRAMMAR-DIAGNOSTICS`, `TEST-ADAPTER-LEXICAL-ANNOTATION-SOURCE-SURFACE`, `TEST-VERIFY-UNDEFINED-GATE-LISTS-DEFINED`) stay `UNKNOWN` by it. | NOT_CHECKED |
| DS-1963 | The patterns refuted on the path to an `if let`, a `let ... else` or a `match` at any depth (inside the `else` of an `if let` or a `let ... else`, in a statement, a block or parentheses there, inside an arm after earlier arms without a guard, or after an `if let` whose `then` block or a `match` whose unguarded arms leave on every path) are read with its own: where they may together match every value, or where one of them checks a scrutinee not shown to be the same expression (after parentheses, references and a block's tail are removed), the `else` or the later arms are undecided, and so is a branch whose pattern one refuted pattern over the same scrutinee covers (read pairwise: `Some(5)` after `Some(_)`), so DA-003 and DA-001 report `UNKNOWN`. | NOT_CHECKED |
| DS-1963 | Remaining false PASS: refuted patterns and a later check of one value through another name (`let p = o;` then `if let None = p`) or through a method (`o.is_some()` beside `if let None = o`) are not read as exhausting it, so DA-003 counts the `else` as reached. | NOT_CHECKED |
| DS-1963 | Remaining false PASS: a pattern whose values earlier refuted patterns cover only together (`Some(4) \| Some(5)` after arms `Some(4)` and `Some(5)`) or through a range (`Some(4)` after `Some(0..=9)`), or that repeats values matched before it (inside the `then` of `if let Some(_)`, or in a `match` arm that checks again the value the arm already checked), is not read as covered, so DA-003 counts its branch as reached though it never runs. | NOT_CHECKED |
| DS-1947 | Remaining false PASS: an arm or an exit at a non-constant position that is not derived (`let k = 1;`) and always matches or always runs is not evaluated, so DA-003 counts the divergence after it as reached. | NOT_CHECKED |
| DS-1983 | Remaining false PASS: an exit decided by the target's result beside an unrelated non-constant condition (`if double(2) == 99 { return } if flag() { panic!() }`) makes DA-003 count the divergence as reached, though a wrong result may skip it. | NOT_CHECKED |
