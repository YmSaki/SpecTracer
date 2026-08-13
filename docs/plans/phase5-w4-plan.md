# Phase 5 / W4 実行計画 — Rust Static Audit 移送

Branch: `feature/adapter-separation-alpha2-implementation`
Entry: W1–W3 Checkpoint（W3 完了）。上位計画 W4。前 Phase: `phase4-w3-plan.md`

## W4 完了条件（上位計画）
- M3、M5 acceptance が全PASS。
- analysis limit が UNKNOWN のまま。
- Audit Record が Test・全target・rule-set・rule影響config projection・参照 Static Analysis Source subject 完全集合へ束縛。
  `assertion_macros` または参照 helper だけの変更で STALE。
- adapter が解析入力集合の完全性を保証できない rule は UNKNOWN、PASS へ集約しない。
- static rule と無関係な run/coverage 設定を config subject から除外する根拠と試験。
- `vtest-audit` に `syn`/`quote` の直接依存が残らない。

## 構造分析（実測）
- `vtest-audit/src/lib.rs` 2193行、syn/quote/proc-macro2 39箇所。deps に syn/quote/proc-macro2/vtest-scan/vtest-store。
- `StaticAuditAdapter::audit(&self, test: &TestEntity) -> Result<StaticAuditObservation, AdapterError>`（frozen W1 API）。
  `StaticAuditObservation { verdict: CheckValue, reasons: Vec<String>, config: StaticAuditConfigDraft, analysis: StaticAnalysisClosureDraft }`。
- **重要**: rule_da001-006（368/595/937/1420 等）と AST helper（call_facts/functions_by_name/collect_assertions 等）は
  `target_resolution`/`item`/`syntax`/`assertion_macros` を引数に取り **scan-free / store-free**。
  scan を使うのは `audit_one`(171) の target 解決部（`target_source(scan, target)`）のみ。
- `audit_static`(82) が orchestrator（config load、test 反復、audit_one 呼出し）。audit_one が per-test（parse・target解決・rules・subject構築）。

## 移送戦略
~~frozen trait `audit(test)` は…よって trait は十分~~ ← **この以前の結論は誤り（撤回）**。
`StaticAuditAdapter::audit(&self, test)` は **root も config も受け取らない**。Rust 静的監査は
test/target ソース読取り（root 必須）と assertion_macros（config 必須）が要るため、trait signature が不足。
synthetic adapter は synthetic データを返すだけなので露見しなかった。実装済み前例は discovery の
`discover(root, config: &CanonicalProjection)`（core が vtest-store で config load → adapter は projection を受領）。
adapter は store-free 制約なので self-read は `.verify/config.yaml` parse の重複になり不適。よって trait を
`audit(&self, root: &Path, config: &CanonicalProjection, test: &TestEntity)` へ変更する（discovery 前例に整合）。
W5 の runner も同じ gap を持つ（`run(root, test)` に config なし・実装ゼロ）— W5 で対処する旨を明記。

**§15 判定 = ブロック対象外（code-only）**: trait method signature はどの spec 文書にも pin されない
（v0.2 §3.2 は capability 名の列挙のみ、別紙A は無記載、詳細設計 §5.2 は DTO/挙動を抽象記述）。よって
signature 変更は code-only refinement（DTO field 追加と同格）。spec 文書は変更しない。W4–W5 checkpoint で
両 W1-API touch（additive `rules` field / `audit` signature 変更）を code-only refinement として Owner へ報告。

W3 と同じ多層パターン:
1. **rule + AST helper を module へ隔離**（intra-crate、scan-free なので decouple 済み）。
2. trait signature 変更（root/config 引数追加）— implementor は synthetic binding test 1件のみ、core 未接続。
3. rule module + 解決 + parse を `vtest-adapter-rust` へ移送、`StaticAuditAdapter for RustCargoStaticAudit` を実装
   （filesystem target 解決 = path::item + src-id 逆引き）、`rust_cargo_registration()` に `static_audit` 追加。
4. `vtest-audit::audit_static` は registry 経由で adapter を呼び、observation から subject を計算・persist（core 側）。
   config subject = `hash_static_audit_config_subject(adapter, rule_set, ver, observation.config.effective_config)`
   → run-only 変更は adapter が返す rule 影響 subset に入らないので非stale。verify 側 re-eval も同一 projection へ揃える。
5. `vtest-audit` から syn/quote/proc-macro2 dep 除去。
6. M3/M5 revalidation（neutral 契約追随、W3 の location.path/effective_status パターン）。

## 設計判断（W4-S1 後、advisor 確認済み）
**observation DTO の per-rule 不足**: frozen `StaticAuditObservation { verdict, reasons: Vec<String>, config, analysis }`
は M3 が要求する per-rule 構造（CLI `data.audits[].rules[]` の rule/verdict/reason/location、および
persisted record `reasons[].rule/verdict/claim/basis`、E-AUDIT-005）を運べない。`reasons: Vec<String>` は不足。

- **§15 判定 = ブロック対象外（code-only）**: 詳細設計 §5.2 は sub-draft（`StaticAnalysisClosureDraft`,
  `StaticAuditConfigDraft`）を pin するが、トップレベル `StaticAuditObservation` 構造と `reasons` の型は
  spec 文書に定義されない。逆に record の per-rule 構造（rule/verdict/claim/basis）は §8 / E-AUDIT-005 で必須。
  よって observation への per-rule フィールド additive 追加は spec 追従を可能にする code-only 変更であり、
  §15（仕様不足・矛盾 → STOP → 独立 spec PR）の対象ではない。spec 文書は変更しない。
- **決定**: `StaticAuditObservation` に additive フィールド `rules: Vec<RuleObservationDraft>` を追加。
  `RuleObservationDraft { rule: String, verdict: CheckValue, reason: String, location: SourceLocation }`。
  既存4フィールドは不変（純 additive）。`CheckValue`（Pass/Fail/Unknown を含む、SCREAMING_SNAKE_CASE）を
  per-rule verdict に流用 → 新 enum 不要・crate 跨ぎ移動不要。binding test `w1_acceptance_binding.rs` を追随更新。

**SRC-ID target 解決（step 2/3 のサイズ）**: `@vtest.target SRC-M3-LOCAL-KNOWN` は scan で locator 化されず
`TargetRef::SrcId` として `TestEntity.targets` に残る（discovery.rs:1086）。adapter の filesystem target 解決は
`path::item` parse に加え src-id 逆引き（source tree を走査し `@vtest.src-id` 一致 item を探す）が必須。
discovery.rs の `parse_src_id`（167）/ SourceTargetDraft src_id（947）ロジックを再利用する。

## 改訂 increment（step 2 は adapter 内へ畳み込み）
- **W4-S1**（済 `09f26d8`）: rule logic を `audit_rules` module へ隔離。
- **W4-S2**: `StaticAuditObservation.rules` additive 追加 + binding test 追随。無挙動変化（23 不変）。
- **W4-S3**: `StaticAuditAdapter for RustCargoStaticAudit` を adapter-rust に実装
  （filesystem target 解決 = path::item + src-id 逆引き、parse、rule 実行 → observation.rules 充填）。
  `rust_cargo_registration()` に `static_audit` 追加。未接続なので 23 不変。
- **W4-S4**: `vtest-audit::audit_static` を registry 経由 adapter 呼出しへ。observation から
  StaticAudit record 構築（subjects←analysis.sources、reasons←rules、config←config）。CLI JSON 形状維持。
- **W4-S5**: `vtest-audit` から syn/quote/proc-macro2 dep 除去。
- **W4-S6**: M3/M5 revalidation（location.file→path 等の中立契約追随）。migrated path で green 化。

## S4 実行メモ（advisor 検証済み）
**line-781 deferral（W4–W5 checkpoint で Owner 報告必須）**: 詳細設計 §5.2 line 781 は有効性再評価時に
adapter へ closure 再導出を要求する設計だが、これは verify crate の機構であり W6 phase の担当。W4 では現行の
scan ベース再評価を維持したまま subject の *内容* のみ修正する（観測挙動 = assertion_macros/helper 変更→STALE、
run-only→非stale は満たす）。§15 非該当（矛盾でなく未到達）。**W6 で persist と re-eval を同時に adapter 化する**
（record は再生成可、audit は再実行可能）。無言吸収のみがこの判断を誤りにする。

**wiring**: `audit_static` は adapter を**引数**で受ける（DI seam、`scan_project_with_discovery` と同型）。
CLI が registry 解決の adapter を渡す。`vtest-audit → vtest-adapter-rust` の dep を作らない（W3 で指摘した temp-dep 債務の再来を回避）。
呼出2箇所（lib.rs:746, :1161）更新。vtest-mcp が audit_static へ vtest-cli 以外から到達しないか確認。

**S4 gotcha checklist**:
1. vtest-verify 自身の unit test（write_static_audit ~1753、CONFIG subject 1808、stale test 1856-1894）は旧 whole-config
   hash を計算 → 同一コミットで helper 更新しないと green の verify unit test が RED 化。
2. vtest-audit lib.rs の rule unit test 約15件を adapter-rust へ移送（audit_rules.rs 削除時）。rule 意味論の唯一の網羅。
3. `AuditError` に `AdapterError` を包む variant 追加（read/parse 失敗を E-OP-001 exit 2 に維持）。
4. CheckValue→AuditVerdict: adapter は Pass/Fail/Unknown のみ emit するが、残り5 variant は防御的に Unknown へ（unreachable! 不可）。
5. adapter `RULE_SET_ID` = vtest-scan 定数（core は vtest-scan 値で subject 計算、observation.config は informational — 各所コメント）。
6. `static_audit_binds_test_subjects`（verify:838）を S4 中に一度目視（subject 形状は現行維持のはずだが freshness path 唯一の未読関数）。

**S4 期待ゲート**: 23→22（`static_audit_ignores_run_only_config_changes` green = 認可された revalidation 縮小）。
m3 は S6 まで red 維持。2 つの stale test（`static_helper_only_change_stales…` / `assertion_macro_change_stales…`）は
regression canary — どちらか red 化したら subject mapping を停止診断。m6 fixture は pre-baked CONFIG hash が新計算で
stale 化しうるが既に RED・W6-owed で name-invariant は保たれる（W6 用にメモ）。

## W4 完了記録（2026-08-13）
コミット列: S1 `09f26d8` / S2 `0e4555c` / S3a `4aa3c00` / S3b `ec245dc` / S4 `9bddbea` /
S6(m3) `d462c3a` / semantic(AF-025/026) — 全ゲート green（fmt/clippy 0）。失敗 23→19。

W4 完了条件 vs 実績:
- M3 acceptance 全PASS ✓（`d462c3a` で location 中立形状 revalidation）
- M5 acceptance 全PASS ✓（m5_acceptance 4/4、変更不要）
- helper-only change → STALE ✓（`static_helper_only_change_stales…` green 維持）
- assertion_macros change → STALE ✓（`assertion_macro_change_stales…` green 維持）
- run/coverage config は static config subject へ混入しない ✓（`static_audit_ignores_run_only_config_changes` green化）
- Specification-only change → impl_consistency STALE ✓（AF-025 green化、bundle が upstream SPEC 束縛）
- impl-consistency FAIL → MISMATCH ✓（AF-026 green化）
- `vtest-audit` に syn/quote 直接依存なし ✓（`cargo tree` 直接 deps クリーン、audit_rules 削除）
- incomplete closure → UNKNOWN ✓（**達成**）: adapter は target 宣言ありだが解決不能なら `analysis.complete=false`、
  core（record_from_observation）が `!complete && Pass → Unknown` を強制（詳細設計 §5.2 line 781）。現行 green テストに
  未解決 target は無いので 19-name invariant 維持。

**B: spec-coverage bundle / service 移設ディレクティブの判断（advisor B）**:
Semantic Audit 本文「CLI内のbundle/submitロジックを共通serviceへ移す」「4種類を成立させる（spec-coverage/
test-semantic/vo-coverage/impl-consistency）」を評価。実測: CLI bundle は3種（test-semantic|vo-coverage|
impl-consistency、lib.rs:922）で **spec-coverage bundle 未実装**。詳細設計 §980/§1099 は spec-coverage bundle
（`--spec` selector、SPEC subject + active REQ 集合）を定義。
**判断 = W6 へ deferred（W4 blocker ではない）**: (1) spec-coverage bundle も service 移設も W4 完了条件（§489-499）に
無く、acceptance test も未行使（残19失敗に spec-coverage bundle 無し）。(2) verify item `spec_coverage` は成立・green。
(3) bundle/submit の共通 service 化は W6（CLI/MCP phase、MCP が CLI shell-out を止める工程）に整合。checkpoint で
Owner へ明示報告する（未実装 body ディレクティブとして）。

残り19失敗は全て W5（evidence_*/m4_*/m7_*/multi_target/head_change/local_dependency/incomplete_execution/
execution_state_mutation/target_external_helper/orchestration_crates=vtest-exec dep）または
W6（m6_*/m9/AF-052）owed。W4-owed はゼロ。

**未完債務（W6/後続）**: (1) line-781 verify closure 再導出の adapter 化、(2) vo-coverage spec subject の
registered_snapshot→current source hash（"W4 owes" コメント、テスト未要求のため保留）、(3) `analysis.complete`
の UNKNOWN 配線、(4) W3 由来の temp dep `vtest-scan→vtest-adapter-rust`（operations.rs）。

## ゲート
各段 name-invariant（現 baseline 23件、`<scratchpad>/w3-s1-baseline.txt`）+ fmt/clippy 0。
最終: `cargo tree -p vtest-audit` に syn/quote なし + M3/M5 green。**達成**。
