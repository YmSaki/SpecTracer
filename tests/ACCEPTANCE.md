# Acceptance Test Ledger

This ledger is the executable-work map for the implemented milestones.  A criterion remains
`NOT_CHECKED` until a reproducible test and its command output exist; missing
coverage is never treated as PASS.

| Milestone | Criterion | Planned test / fixture | Status |
|---|---|---|---|
| S0 | Eight crates build with one-way dependencies | `cargo test --workspace` | IN_PROGRESS |
| S0 | Canonical `.verify/` layout is created | `vtest-cli` init unit test | IN_PROGRESS |
| S0 | Shared envelope, diagnostics, exit codes, and hash type | `vtest-model` unit tests | IN_PROGRESS |
| S0 | Fixture forms are recorded | `tests/fixtures/calc` | IN_PROGRESS |
| M1 | Annotated and unregistered tests are extracted | `vtest-scan` tests | IN_PROGRESS |
| M1 | E-SCAN-002..010 and W-SCAN-101 fixture matrix | `tests/fixtures/calc` | NOT_CHECKED |
| M1 | JSON output and exit code 0/1 | CLI integration coverage | NOT_CHECKED |
| M1 | `filter`, `package`, and `test_target` values | scanner fixture assertions | NOT_CHECKED |
| M5 | Three audit bundle kinds include subject hashes | `vtest audit bundle` on `tests/fixtures/calc` | IN_PROGRESS |
| M5 | Empty reasons and missing basis are rejected with E-AUDIT-005 | `vtest audit submit` | IN_PROGRESS |
| M5 | Changed subject rejects submission with E-AUDIT-002 | bundle/submit fixture flow | IN_PROGRESS |
| M5 | Accepted audit is PASS while hashes match and STALE after change | scoped `vtest verify` | IN_PROGRESS |
| M6 | Semantic audit aggregation is fail-closed and hash-aware | `vtest verify --items semantic_audit` | IN_PROGRESS |
| M7 | Called target records PASS with count >= 1 | `TEST-CALC-ADD` plus `vtest-exec` coverage tests | IN_PROGRESS |
| M7 | Passing test that skips its target records FAIL with count 0 | `TEST-CALC-NO-CALL` plus `vtest-exec` coverage tests | IN_PROGRESS |
| M7 | Missing cargo-llvm-cov records W-EXEC-101 and NOT_CHECKED | isolated `CARGO_HOME` fixture run plus `vtest-exec` test | IN_PROGRESS |
| M8 | Invalid symbols and enum variants return candidate-bearing E-OP-001 | `structured_create_generates_a_scannable_test` | IN_PROGRESS |
| M8 | Create output is rescanned; integration targets are retained | `structured_create_generates_a_scannable_test` | IN_PROGRESS |
| M8 | Edit changes only one Test and is idempotent | source hash assertions in `structured_create_generates_a_scannable_test` | IN_PROGRESS |
| M8 | Multi-target Evidence cannot pass with one target hash | `multi_target_evidence_cannot_pass_with_a_single_target_hash` | IN_PROGRESS |
| M8 | Form conditional-branch validation | detailed design does not define a condition schema | NOT_CHECKED |
| M2–M4, M9 | Remaining milestone criteria | Annex B §18.3 | NOT_STARTED |
