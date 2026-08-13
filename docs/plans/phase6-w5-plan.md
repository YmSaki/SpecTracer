# Phase 6 / W5 実行計画 — Rust runner / coverage / Execution Evidence 分離

Branch: `feature/adapter-separation-alpha2-implementation`
Entry: W4 完了（`1a38aaa`）。上位計画 W5。前 Phase: `phase5-w4-plan.md`
Baseline: 失敗19件 `<scratchpad>/w4-complete-baseline.txt`（うち W5 関連16, W6 関連3=m6×2/m9）。

## W5 完了条件（上位計画 §542）
- M4、M7 acceptance 全PASS。
- target-only change / target-external helper change / local dependency change / execution mutation /
  multi-target aggregation の各 freshness/集約。
- Rust adapter は runner observation と hash 未計算 Execution State draft **のみ** を返す。
  Evidence 生成・append-only 保存は `vtest-exec` が所有。
- Execution State closure: canonical invocation, runner identity, toolchain identity, exec-affecting config,
  exact HEAD revision, repo input manifest, local dependency inputs, pre/post snapshot consistency。
- Evidence: test_subject, 全 target_construct hash, target-specific results, Execution State subject, adapter identity。
- Fail-closed（§530）: build fail→Evidenceなし / missing result→なし / HEAD mismatch→STALE / revision unknown→STALE /
  missing Execution State→STALE / incomplete snapshot→UNKNOWN / State 変化→E-EXEC-004・Evidenceなし /
  adapter mismatch→non-PASS / coverage capabilityなし→NOT_CHECKED。
- `vtest-exec` から rustc-demangle（Rust analysis）直接依存排除。

## 構造分析（実測）
- `vtest-exec/src/lib.rs` 712行。deps: rustc-demangle, serde, serde_json, thiserror, vtest-model, vtest-store。
  **syn/quote なし**（AST parse 無し）。Rust analysis dep = `rustc-demangle`（coverage symbol demangle）のみ。
- `TestRunnerAdapter::run(&self, root: &Path, test: &TestEntity) -> RunnerObservation`（frozen W1）。
  **config gap**: audit と同様 config を受け取らない（実装ゼロ、synthetic のみ）。W5 first increment で
  `run(&self, root, config: &CanonicalProjection, test)` へ（W4 audit と同一手順・§15 非該当の見込み、要 spec grep 確認）。
- `CoverageAdapter::capability_id(&self) -> &str`（最小）。coverage 実メソッド未定義 → W5 で拡張要。
- `RunnerObservation { result, runner, target_execution, execution_state: ExecutionStateDraft, log }`。
  `ExecutionStateDraft`（詳細設計 §5.2）は既に定義済み。

## advisor carry-in（W4 から）
1. **runner trait config gap**: `run(root, test)` に config 無し・実装ゼロ・signature 未 pin。W5 first increment で
   audit と同一手順（別紙A/v0.2/詳細設計 grep → code-only 確認 → 変更 → synthetic binding test 追随）。
2. **W6-deferral list**（W4 から持ち越し、W6 で対処）:
   - 詳細設計 §5.2 line 781: verify の closure 再導出 adapter 化（static audit の re-eval と runner Evidence 両方）。
   - temp dep `vtest-scan → vtest-adapter-rust`（W3 由来、operations.rs）。
   - vo-coverage は W4 で current source hash 済（債務消化）。
3. **checkpoint report 義務**（W4-W5 checkpoint）:
   - W1-API touch 2件（additive `rules` field / `audit` signature）を code-only refinement として spec-silence 根拠付きで報告。
   - W5 で runner signature も変更するなら同様に報告（3件目）。
   - E-AUDIT-001〜007: 各 code の emitting site + covering test の grep マッピングを report 組立時に作成。
   - spec-coverage bundle 未実装 / service 移設の W6 deferral 判断（phase5-w4-plan.md 記載）を報告。

## move/stay インベントリ（vtest-exec/src/lib.rs 実測）
**adapter へ移送（Rust 固有）**:
- Cargo command 生成: `cargo_command`(232) / `cargo_llvm_cov_command`(274) / `command_string`(319) /
  `llvm_cov_command_string`(329) / `rust_suite_argument`(345)。
- libtest 出力 parse: `parse_result`(215) / `ObservedResult`(209)。
- coverage 属性: `cargo_llvm_cov_available`(381) / `target_execution_from_coverage`(389) /
  `llvm_cov_function_count`(406) / `llvm_name_matches`(447, **rustc_demangle 使用**) /
  `llvm_filenames_match`(464) / `path_suffix_matches`(477)。
- `RustLocator`(34)。TargetExecution builder（`not_checked/measured/unavailable/unknown_target_execution`
  483-527）は neutral 値生成なので core 側判断で使うが、coverage 由来値は adapter。

**vtest-exec 残置（orchestration）**:
- `run_tests`(51) → registry 経由 adapter 呼出しへ改修。Execution State subject 計算、Evidence 生成・
  append-only persist、fail-closed 判定（build fail/missing result/HEAD mismatch/State mutation=E-EXEC-004）。
- `evidence_yaml`(527) / `check_value_name`(586) / `yaml_scalar`(600) / `git_revision`(362)。
- `RunnableTest`(27) / `ExecutionResult`(40) / `ExecutionError`(18)。

adapter は `run(root, config, test) -> RunnerObservation { result, runner, target_execution,
execution_state: ExecutionStateDraft, log }` を返す。core が hash を計算し Evidence subject を束縛。

## 移送戦略（W4 パターン踏襲）— 進捗
1. ✅ **runner trait signature 変更**（`52749cf`、implementor ゼロで安全）。
2. **module 隔離**（`fe299ce`）: Rust runner/coverage を `vtest-exec/src/rust_runner.rs` へ隔離（intra-crate、
   19-invariant 維持）。**次**: これを `vtest-adapter-rust/src/runner.rs` へ移送し `TestRunnerAdapter`/
   `CoverageAdapter for RustCargo…` 実装、`rust_cargo_registration()` に追加。RunnerObservation は
   `{result, runner, target_execution, execution_state: ExecutionStateDraft, log}` を返す。
   **DTO 十分**（audit の `rules` gap に相当する不足なし。ExecutionStateDraft が full state を保持）。
3. `vtest-exec` orchestrator 化: registry 経由 adapter 呼出し、**実 Execution State 構築**（現状 stub
   `complete:false, hash:None` → canonical invocation/toolchain/HEAD/manifest/local deps/pre-post snapshot を
   ExecutionStateDraft で構築、core が hash 計算）、Evidence 生成・append-only、fail-closed（E-EXEC-001/002/003
   既存 + E-EXEC-004 State mutation 追加）。**ここが W5 の主要機能追加**（evidence_* テスト群）。
4. `vtest-exec` から rustc-demangle dep 除去（demangle は adapter へ）。CLI が adapter を渡す（DI seam、
   vtest-exec→adapter-rust dep を作らない、W4 audit と同型）。
5. M4/M7 revalidation。既知の小修正: fast パス target_execution は `result: None`→Null だが m4 は
   `result:"NOT_CHECKED", checked:false` を要求 → NOT_CHECKED 値化。

## 現在地（handoff）
完了コミット: `52749cf`(trait) → `fe299ce`(module 隔離)。失敗19件不変。次の increment は step 2 後半
（rust_runner を adapter へ移送 + adapter 実装）。その後 step 3（実 Execution State + Evidence orchestrator）が
evidence_*/m4/m7 を green 化する主要機能。vtest-exec 現状 304行（orchestrator + evidence_yaml/git_revision）、
rust_runner.rs 421行（移送対象）。

## ゲート
各段 name-invariant（現 baseline 19件）+ fmt/clippy 0。
最終: `cargo tree -p vtest-exec` に rustc-demangle なし + M4/M7 green + `orchestration_crates…` green。
