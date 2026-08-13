# W4–W5 Checkpoint 自己レビュー報告

Branch: `feature/adapter-separation-alpha2-implementation` / HEAD `a12af37`
到達: W4（Static/Semantic Audit 分離）+ W5（runner/coverage/Execution Evidence 分離）完了。
失敗テスト **23 → 2**（残る2件は W6/Phase 7 owed）。全段 fmt/clippy 0・build all-targets green。

## 契約遵守（§15）
このセッションのコミットが変更した `docs/` は作業計画 (`docs/plans/`) と本報告 (`docs/reports/`) のみ。
**正規仕様書（基本仕様 / 詳細設計 / 別紙A / 別紙C / リファクタリング計画）は git diff で不変を確認**（差分空）。
仕様書は変更していない。

## W4 完了条件（詳細設計/上位計画）vs 実績
| 条件 | Expected | Evidence | Actual | Result |
|---|---|---|---|---|
| M3 acceptance | 全PASS | `m3_static_audit_maps_failures…` | green | ✅ |
| M5 acceptance | 全PASS | `m5_acceptance` 4/4 | green | ✅ |
| helper 変更→STALE | static audit stale | `static_helper_only_change_stales…` | green | ✅ |
| assertion_macros 変更→STALE | stale | `assertion_macro_change_stales…` | green | ✅ |
| run/coverage config を static subject へ混入させない | 非stale | `static_audit_ignores_run_only_config_changes` | green | ✅ |
| incomplete closure→UNKNOWN | PASS禁止 | adapter `complete` flag + core 強制（§5.2 line 781） | 実装済 | ✅ |
| Spec 変更→impl_consistency STALE | STALE | AF-025 `specification_only_change_stales_impl_consistency` | green | ✅ |
| impl-consistency FAIL→MISMATCH | MISMATCH | AF-026 `impl_consistency_fail_maps_to_mismatch` | green | ✅ |
| vtest-audit に syn/quote 直接依存なし | dep 排除 | `orchestration_crates…`（audit部分）+ `cargo tree` | green | ✅ |

## W5 完了条件（上位計画 §542 / 別紙C §18.3.3, §18.3.6）vs 実績
| 条件 | Expected | Evidence | Actual | Result |
|---|---|---|---|---|
| M4 acceptance | 全PASS | m4_run_fast / m4_build_failure / m4_missing_result / m4_ignored / m4_multi_target / m4_target_mutation | green | ✅ |
| M7 acceptance | 全PASS | m7_called / m7_passing_misses / m7_missing_llvm_cov | green | ✅ |
| target-external helper 変更→STALE | STALE | `target_external_helper_change_stales_evidence` | green | ✅ |
| local dependency 変更→STALE | STALE | `local_dependency_change_stales_evidence` | green | ✅ |
| execution mutation→E-EXEC-004・Evidenceなし | E-EXEC-004 | `execution_state_mutation_reports_e_exec_004…` | green | ✅ |
| multi-target 集約 | per-target result | `multi_target_evidence_keeps_target_specific_results`（coverage実行） | green | ✅ |
| HEAD 変更→STALE / revision unknown→STALE / missing state→STALE | STALE | head_change / evidence_without_revision / evidence_without_execution_state | green | ✅ |
| incomplete snapshot→UNKNOWN | UNKNOWN | `incomplete_current_execution_snapshot_is_unknown` | green | ✅ |
| adapter は runner observation と hash未計算 draft のみ返す | Evidence は core 所有 | `RunnerOutcome` + vtest-exec orchestrator | 実装済 | ✅ |
| coverage capability なし→NOT_CHECKED + W-EXEC-101 | 警告+NOT_CHECKED | `m7_missing_llvm_cov…`（core が W-EXEC-101 導出、not-checked→verify で NOT_CHECKED） | green | ✅ |
| not-checked 保存形（§442/§3.7/§10.3） | checked:false, result:null, targets:[] | Owner 決定(B) 準拠、不整合受入テスト整合済 | 実装済 | ✅ |
| vtest-exec に rustc-demangle 直接依存なし | dep 排除 | `orchestration_crates_have_no_direct_rust_analysis_dependencies` | green | ✅ |

## W1 frozen-API touch（3件、全て code-only refinement）
いずれも spec 文書に signature/DTO の pin が無いことを grep 確認済み（詳細設計 §5.2 は DTO sub-draft のみ pin、
別紙A・v0.2 は capability 名のみ）。§15 非該当。
1. `StaticAuditObservation.rules` additive field（`0e4555c`）— M3 の per-rule 契約に必要。
2. `StaticAuditAdapter::audit` に root+config 引数（`4aa3c00`）— discovery 前例に整合、store-free 維持。
3. `TestRunnerAdapter::run` に root+config 引数（`52749cf`）+ 戻り型 `RunnerObservation`→`RunnerOutcome`（`61102c6`）
   — Ignored/MissingResult の非-結果アウトカム伝達 + fail-closed を core 側に保持。

## 設計上の単一実装保証（divergence 防止）
Execution State 再構築は単一実装: walk は `vtest-adapter-rust::build_execution_state`（runner が使用、verify は
`vtest-scan::rust_cargo_execution_state_hash` 経由で使用）、draft→hash は `vtest-adapter-api::hash_execution_state_draft`
（runner path と verify 再導出の両方が使用）。fresh-run→verify→PASS の byte 一致 canary で確認。

## W6 へ deferred（記録）
1. **line-781 verify closure 再導出の adapter 化**: W5 では core が再構築（scan 非依存、Rust 固有ロジックを
   vtest-scan/adapter-rust 経由で使用）。W6 で persist と re-eval を adapter へ統一する際に相対化。
2. **temp dep `vtest-scan → vtest-adapter-rust`**（W3 由来、operations.rs + 今回 build_execution_state 再利用）。
3. **spec-coverage bundle 未実装 / bundle-submit の共通service化**（詳細設計 §980）— W4 完了条件外、W6 CLI/MCP phase。
4. **fixture Cargo.lock 追加**（m1 base）: §456 の manifest 決定性のため commit（生成 lock の非決定性回避）。

## 残（W6/Phase 7、W4-W5 checkpoint 外）
- `m6_complete_fixture_is_ok_for_all_eleven_items`（verify/report 全項目統合）
- `m9_reference_flow_completes_over_mcp_stdio`（MCP reference flow）

## 判定
W4-W5 の全 Acceptance Criteria を自己レビューし、全達成。checkpoint 到達につき Owner 承認を要請し、ここで停止する。
