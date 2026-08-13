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

## runner adapter 境界設計（advisor 確定）
- `run(root, config, test) -> Result<RunnerOutcome, AdapterError>`。`RunnerOutcome`（`61102c6` で追加済）:
  `Completed(Box<RunnerObservation>)` / `Ignored{runner,log}` / `MissingResult{runner,log}`。全 variant が log 保持
  （core が Evidence/log 永続を所有）。
- **fail-closed は全て core 側**（§530、詳細設計 §1182-1194 が裏付け）: `Ignored`→Evidence なし、
  `MissingResult`→exit_code で E-EXEC-001（build fail, code≠0）/ E-EXEC-002（no result line）を判別、
  `Completed` の `result==Pass != (exit_code==0)`→E-EXEC-003。libtest 出力 parse は core へ越境させない。
- **fast は trait param にしない**: core が exec-affecting config projection に畳む（fast なら coverage=off）。
  `rust_cargo_static_audit_projection` と同じ core-owns-config パターン。
- **coverage は今は CoverageAdapter に分離しない**: llvm-cov は同一 cargo プロセス内、target_execution は
  RunnerObservation に載せる（現状通り）。CoverageAdapter は m7/`coverage なし→NOT_CHECKED` が要求する時に再訪。
- **W-EXEC-101（coverage unavailable warning）**: observation に diagnostics channel なし。core が導出
  （coverage 要求(!fast) かつ target_execution が not-checked → W-EXEC-101）。
- ExecutionStateDraft は S3b では **minimal**（complete:false）。vtest-exec が persisted subject を byte 一致で描画
  （`schema: rust-cargo-execution-state-v1, complete: false, hash: null`）。実 Execution State は step 3（別コミット）。

## increment 分割（advisor 確定）
- ✅ S3a（`61102c6`）: RunnerOutcome enum + run 戻り型変更（implementor ゼロ、trivial）。第3の W1-API touch。
- **S3b（次）**: adapter impl を `vtest-adapter-rust/src/runner.rs` に（W4-S3b 同様の temp 重複、rust_runner を copy）。
  target locator は audit の `resolve_target` 同様に test.targets から自己解決（Locator::parse / src-id walk）。
  rustc-demangle を adapter-rust deps に追加、`test_runner` を registration に。未接続なので 19 不変。
- **S3c**: `run_tests(…, adapter)` DI param 化、CLI が registry 解決 runner を渡す（vtest-exec→adapter-rust dep 作らない）、
  vtest-exec の rust_runner 削除、rustc-demangle dep 除去。期待ゲート **19→18**
  （`orchestration_crates_have_no_direct_rust_analysis_dependencies` green化）。
- **step 3（W5 主要機能）**: 実 Execution State 構築（canonical invocation/toolchain/HEAD/manifest/local deps/
  pre-post snapshot）+ Evidence subject 束縛（core が hash）+ E-EXEC-004（State mutation）。evidence_*/m4/m7 を green 化。
- **step 4（M4/M7 revalidation）**: fast/unavailable の target_execution を `result: Some(NotChecked)` 値化。
  **landmine**: 現 green unit test `unavailable_coverage_is_not_checked_and_never_passes`（`result==None` を assert、
  rust_runner の tests に移動済）を同コミットで更新。両 renderer 確認（CLI JSON serde = m4 が見る / `evidence_yaml`）。

## checkpoint report 用 W1-API touch（3件、全て code-only・spec-silence 根拠）
1. `StaticAuditObservation.rules` additive field（`0e4555c`）。
2. `StaticAuditAdapter::audit` に root+config 引数（`4aa3c00`）。
3. `TestRunnerAdapter::run` に root+config 引数（`52749cf`）＋戻り型 RunnerObservation→RunnerOutcome（`61102c6`）。

## 現在地（handoff）
**runner adapter 分離 完了**（S3a/b/c）:
- `52749cf`(runner trait config) → `fe299ce`(rust_runner 隔離) → `61102c6`(RunnerOutcome) → `947e805`(S3b: adapter 実装) →
  `79bd695`(S3c: vtest-exec rewire、rustc-demangle 除去、CLI DI seam)。**失敗 19→18**（`orchestration_crates…` green化）。
- vtest-exec は中立 Evidence orchestrator（run_tests が adapter 呼出し→RunnerOutcome 分岐→Evidence/diagnostic）。
  fail-closed（E-EXEC-001/002/003）は全て core 側。vtest-exec deps: rustc-demangle なし、adapter-rust 依存なし（adapter-api のみ）。

**残: step 3（W5 主要機能、evidence_*/m4/m7 の約15件を green 化）**
- 実 Execution State 構築: 現状 adapter は minimal ExecutionStateDraft（complete:false）、vtest-exec は stub subject
  （schema/complete:false/hash:null）を byte 一致描画。→ canonical invocation/toolchain identity/HEAD revision/
  repo input manifest/local dependency inputs/pre-post snapshot consistency を ExecutionStateDraft で構築し、
  core が Execution State subject hash 計算。
- Evidence subject 束縛（test_subject/全 target_construct hash/Execution State subject/adapter identity）を完全化。
- fail-closed 追加: HEAD mismatch→STALE、revision unknown→STALE、missing Execution State→STALE、
  incomplete snapshot→UNKNOWN、run 中の State 変化→E-EXEC-004・Evidence なし。
- 対象テスト: evidence_contains_neutral_subjects_and_complete_execution_state /
  evidence_without_execution_state_is_compatibility_stale / evidence_without_revision_commit_is_stale /
  execution_state_mutation_reports_e_exec_004_without_evidence / head_change_without_test_or_target_change_stales_evidence /
  incomplete_current_execution_snapshot_is_unknown / local_dependency_change_stales_evidence /
  target_external_helper_change_stales_evidence / multi_target_evidence_keeps_target_specific_results /
  m4_multi_target/run_fast/target_mutation。

**残: step 4（M4/M7 revalidation）**
- fast/unavailable の target_execution を `result: Some(NotChecked)` 値化（m4 が result:"NOT_CHECKED", checked:false を要求）。
  **landmine**: unit test `unavailable_coverage_is_not_checked_and_never_passes`（`result==None` assert、runner.rs の tests に
  移動済）を同コミットで更新。両 renderer（CLI JSON serde / evidence_yaml）確認。m7_* 3件。

現在 18失敗 = W5 step3/4 対象約15 + W6 対象3（m6×2/m9、AF-052）。baseline `<scratchpad>/w5-s3c-baseline.txt`(18)。以後は 18 基準で diff。

## step 3 の実測契約（`evidence_contains_neutral_subjects_and_complete_execution_state` 他）
Evidence JSON の `execution_state` が要求する形状（現状 stub と乖離）:
```
evidence["execution_state"]["subject"]         : string（Execution State subject hash）
evidence["execution_state"]["complete"]        : true（stub は false）
evidence["execution_state"]["revision"]["commit"] : string
evidence["execution_state"]["repository_inputs"]  : array（repo input manifest = target + 局所依存ソース群）
```
- `target_external_helper_change_stales_evidence`: target が呼ぶ helper ファイル（src/helper.rs 等、target 構文外）の
  変更で Evidence STALE。→ repository_inputs に **target-external な局所依存ファイル**を含める必要。verify の
  evidence_validity 再評価が現在の inputs hash と保存 subject を比較。
- `execution_state_mutation_reports_e_exec_004_without_evidence`: run 中に input ファイルが変化 → E-EXEC-004、Evidence なし
  （pre/post snapshot consistency）。
- `head_change_without_test_or_target_change_stales_evidence` / `evidence_without_revision_commit_is_stale`:
  HEAD revision を Execution State subject に束縛。
- `incomplete_current_execution_snapshot_is_unknown`: 現在スナップショット不完全 → UNKNOWN。

**設計方針（step 3、要 advisor 確認）**: adapter が完全 ExecutionStateDraft を構築
（invocation=canonical cargo 起動座標 projection、toolchain_identity=rustc/cargo version、head_revision、
inputs=ExecutionInputDraft[]{root_identity, root_relative_path, kind, bytes} で target 構文 + target-external 局所依存 +
実行時 input ファイル、complete=true）、core が `hash_*` で Execution State subject 計算 + Evidence subject 束縛、
CLI JSON が execution_state.{subject,complete,revision,repository_inputs} 露出、verify が inputs 再導出で freshness、
pre/post snapshot 差分で E-EXEC-004。局所依存の抽出範囲（target が use する同一 crate 内モジュール等）が要設計。
これは adapter/core(vtest-exec)/CLI/verify 跨ぎの大型機能で W5 の主要残作業。

## step 3 設計（advisor 確定 + 詳細設計 §3 実測、**重要な訂正含む**）
**persisted Evidence の execution_state は §3(406-409) で pin**: `{schema, complete, hash}` のみ（＝現行 model
`ExecutionStateSubject` と一致、**manifest は永続化しない、hash のみ**）。read_evidence の round-trip 変更は**不要**。
一方 acceptance test の `execution_state.{subject, revision, repository_inputs}` は **CLI run レスポンスの view**
（subject=hash、revision=record.revision、repository_inputs=実行時 manifest のパス列）。→ run_tests の `ExecutionResult` が
manifest を response 用に露出する必要（persist はしない）。

**manifest 定義（§456、file-tree ベース・call-graph でない）**: 選択 Test を含む Cargo workspace/package root +
全 local path dependency root 配下の通常 file（Cargo manifest/lockfile、`.cargo` config、build script、
Rust source/test/fixture/compile-time resource、toolchain 指定）。**除外必須: `.git/`、`.verify/`、Cargo target dir**
（除外しないと毎 run が E-EXEC-004 を自己誘発）。collect_rs_files 不可（全 file 種別が要る）。repository 内 helper 変更でも
manifest hash 変化。

**E-EXEC-004（新 API 不要、§783）**: command 起動**前**の draft が pre-snapshot（full bytes 保持）。core が run() 後に
各 ExecutionInputDraft の `root.join(root_relative_path)` を再読込し bytes 比較 + HEAD 再確認 → 差分あれば E-EXEC-004・
Evidence なし。**adapter は cargo 起動前に bytes capture 必須**（現 S3b は run 後構築なので要修正）。

**verify 再導出（step3c）**: adapter を verify に入れない（W4 同様）。verify が manifest（tree+除外）を再走査し
subject hash 再構築 → record.execution_state.hash と比較。HEAD は record.revision vs 現 git。読取不能/欠落 input→UNKNOWN。
**neutrality 債務**: 完全な adapter ベース DTO 再導出（toolchain identity 等、テストは rustc version 不変）は
line-781 deferral に合流（W6）。W5 の verify は manifest+HEAD 再導出に留める。invocation/toolchain/config は run 時と
verify 時で不変前提（テストが変えない）なので hash 比較で helper/HEAD 変化のみ効く。

## step 3/4 increment（advisor 確定、18 baseline）
- **step3a**: adapter 完全 draft（tree manifest を **cargo 起動前**に、invocation projection は絶対path除外、
  toolchain=`rustc --version`、head_revision）+ core が `hash_execution_state_subject` で subject 計算 +
  record（schema/complete:true/hash）+ evidence_yaml + CLI JSON view（subject/revision/repository_inputs、
  manifest は ExecutionResult 経由）→ `evidence_contains_neutral_subjects_and_complete_execution_state` green（18→17）。
- **step3b**: core post-check（bytes 再読込 + HEAD）→ E-EXEC-004 → `execution_state_mutation…` green。
- **step3c**: verify manifest/HEAD 再導出 → helper/local-dep/head-change/revision-missing/without-execution-state/
  incomplete-snapshot green（≈17→11）。
- **step4**: NOT_CHECKED 値化 + landmine（`unavailable_coverage_is_not_checked_and_never_passes` を同コミット更新）+
  multi-target attribution（`TargetExecution.targets` は現状常に空、per-target `TargetExecutionObservation` が要る）→ m4/m7/multi_target。

**canary（step3）**: 3つの fail-closed m4 + **m1 fixture のクリーン run が E-EXEC-004 を出さないこと**（除外リスト検証、
w4dbg 式 repro で手動確認してからスイート実行）。

## ゲート
各段 name-invariant（現 baseline 19件）+ fmt/clippy 0。
最終: `cargo tree -p vtest-exec` に rustc-demangle なし + M4/M7 green + `orchestration_crates…` green。
