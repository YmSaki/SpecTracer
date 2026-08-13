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

## ゲート
各段 name-invariant（現 baseline 23件、`<scratchpad>/w3-s1-baseline.txt`）+ fmt/clippy 0。
最終: `cargo tree -p vtest-audit` に syn/quote なし + M3/M5 green。
