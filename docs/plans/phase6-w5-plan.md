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

## 移送戦略（W4 パターン踏襲）
1. **runner trait signature 変更**（config 引数追加、synthetic binding test 追随）。first increment。
2. Rust runner ロジック（cargo test 起動・結果 parse・target execution）+ coverage（llvm-cov・demangle）を
   `vtest-adapter-rust` へ移送、`TestRunnerAdapter`/`CoverageAdapter for RustCargo…` 実装、registration 追加。
3. `vtest-exec` を orchestrator 化: registry 経由 adapter 呼出し、Execution State subject 計算、Evidence 生成・
   append-only 保存、fail-closed 判定。core が hash 所有（W4 の subject パターン）。
4. `vtest-exec` から rustc-demangle dep 除去（demangle は adapter へ）。CLI が adapter を渡す（DI seam、
   vtest-exec→adapter-rust dep を作らない）。
5. M4/M7 revalidation（neutral 契約追随）。

## ゲート
各段 name-invariant（現 baseline 19件）+ fmt/clippy 0。
最終: `cargo tree -p vtest-exec` に rustc-demangle なし + M4/M7 green + `orchestration_crates…` green。
