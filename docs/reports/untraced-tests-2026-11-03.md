# Untraced tests (W-SCAN-101) — 2026-11-03

Issue #78. Measured with `cargo run --quiet -p vtest-cli --locked -- --format json doctor`.

- **Before:** 19 tests without `@vtest` annotations.
- **After:** 13.

Each of the 6 newly annotated tests covers a VO whose claim is the exact statement of one canonical clause.

## Annotated in this change

| Test | VO | Clause |
|---|---|---|
| `edit_test_set_test_kind_always_generates_the_unit_prefix_regardless_of_target_count` | `VO-SCAN-RUST-INTEGRATION-KIND-LINE-MATCHES-UNIT` | DS-1245 |
| `relation_known_keys_match_the_record_shape` | `VO-STORE-RELATION-UNKNOWN-FIELD-REJECTED` | DS-1676 |
| `register_accepts_a_consistent_declaration_and_resolves_the_typed_accessor` | `VO-ADAPTER-REGISTRY-RESOLVES-REGISTERED` | REQ-261 |
| `evidence_is_built_from_runner_observation` | `VO-EXEC-EVIDENCE-MODEL-RUNNER-NEUTRAL` (new) | DS-331 |
| `vo_ref_list_field_rejects_a_scalar` | `VO-SCAN-FORM-COVERS-VO-REF-LIST` (new) | DS-1250 |
| `locator_from_declared_value_passes_through_an_unparseable_value_verbatim` | `VO-ADAPTER-RUST-DECLARED-LOCATOR-NORMALIZED` | DES-222 |

## Left unannotated, with the reason

| Test | Reason |
|---|---|
| `runner_rejects_unknown_suite_kind`, `runner_rejects_missing_project`, `runner_rejects_missing_suite`, `runner_rejects_missing_suite_name`, `runner_rejects_missing_coverage_output_path` (adapter-rust) | The candidate clauses are DS-042/043 (reject missing or contradictory input) and DES-433 to 435 (how `project` / `suite` are interpreted). The canon does not say whether DS-042/043 apply to runner command generation in §9.2 or only to the wire reader in §2.4. The in-code comments already record this as an open upstream question. |
| `availability_reports_the_adapters_own_unavailable_reason` (adapter-rust coverage) | Asserts only the wording of the error string. DS-760 and BD-220 require W-EXEC-101 but do not fix its text. |
| `show_of_an_unregistered_id_is_a_usage_error` (cli doc) | No clause says that `doc show` of an unknown id is E-OP-001. DS-1015 to 1017 define what `show` outputs, not how it rejects. |
| `vo_ref_field_still_rejects_an_empty_scalar` (scan operations) | No clause defines the shape of a scalar `vo-ref` value (DS-1250 names only `vo-ref-list` for `covers`). |
| `document_derives_from_lists_every_owner_when_a_node_id_is_duplicated` (store doc_registry) | DS-1676 makes a duplicated node id E-SCAN-010. This test pins behavior that runs before that diagnostic and that the canon does not define. |
| `duplicate_top_level_version_key_is_rejected`, `approval_roles_with_a_duplicate_key_is_rejected` (store config) | No clause makes a duplicate YAML key in `config.yaml` a rejection. The closest is `VO-STORE-CONFIG-VERSION-YAML-MODEL-PARSE`, which is a derived claim and not one clause. |
| `audit_round_trip_binds_subjects_and_reads_from_a_ulid_file`, `audit_rejects_malformed_or_untraceable_records` (store records) | These exercise the audit-record shape. The judgment-record model (§3.4, `.verify/decisions/`) has not been implemented yet (#88), so the VOs belong in that work. |

## Effect on `chain_integrity`

W-SCAN-101 is a warning. The 13 remaining tests have no `covers`, so they do not create orphan VOs. However, `chain_integrity` keeps reporting the missing management declarations until either clauses are added for them, or the Owner decides they are permanently outside the chain. That decision is reported for an Owner ruling (`needs-owner`) on #78.

## Update 2026-11-15

Measured on `develop` f10dfbd: 11 tests without `@vtest` annotations (the two audit-record tests above are gone). After this change: 4.

### Annotated in this change

| Test | VO | Clause |
|---|---|---|
| `runner_rejects_unknown_suite_kind`, `runner_rejects_missing_project`, `runner_rejects_missing_suite`, `runner_rejects_missing_suite_name` | `VO-ADAPTER-RUST-RUNNER-INCOMPLETE-COORDINATE-NOT-EXECUTABLE` (new) | DS-043 |
| `runner_rejects_missing_coverage_output_path` | `VO-ADAPTER-RUST-RUNNER-COVERAGE-COMMAND` | DES-455 |
| `availability_reports_the_adapters_own_unavailable_reason` | `VO-ADAPTER-RUST-COVERAGE-AVAILABILITY-PROBE` (new) | DES-452 |
| `show_of_an_unregistered_id_is_a_usage_error` | `VO-DOC-SHOW-UNREGISTERED-ID-EXIT-TWO` (new) | DS-929 |

This reverses the 2026-11-03 reading of three rows above.

- DS-043 stands in the same paragraph as DES-016 (the execution coordinate: adapter, project, suite, selector). A runner that returns no command for an incomplete coordinate is what "do not treat it as executable by guessing" asks for, wherever the coordinate is consumed.
- The availability test observes the outcome of the `cargo llvm-cov --version` probe (DES-452). The error text is how it identifies that outcome.
- `doc show` of an unknown id is an invalid argument, which DS-929 lists under exit code 2.

### Still unannotated

| Test | What the canon lacks |
|---|---|
| `duplicate_top_level_version_key_is_rejected`, `approval_roles_with_a_duplicate_key_is_rejected` (store config) | DS-361 and DS-1662 list the config violations that are E-CONFIG-001. A duplicate YAML key is not in the list. |
| `vo_ref_field_still_rejects_an_empty_scalar` (scan operations) | No clause defines the value shape of a scalar `vo-ref` field. |
| `document_derives_from_lists_every_owner_when_a_node_id_is_duplicated` (store doc_registry) | DS-1677 says a reference to a duplicated node id resolves to none of the candidates. The test pins the document-level tree showing every owner, which no clause defines. |

`chain_integrity` stays MISMATCH until these four are resolved. Each needs a clause in the canon (a `spec/*` change), or the test removed.
