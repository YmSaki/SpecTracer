# Contradiction verification 結果（42候補・per-candidate 8点鎖 + 隔離再現）

wf_e7cc4f6e-00d（42 per-candidate agents, opus/effort-high, 懐疑 default・隔離 temp プロジェクトで実再現）。全 dossier: `docs/plans/dogfood-contradiction-verification.json`。

## 4値集計

| outcome | 件数 | |
|---|---|---|
| **CONFIRMED** | **39** | 8点鎖 + system-level negation（reachable・先行 gate 不 reject・normative behavior 破れ）を通過 |
| REFUTED | 2 | VO-EXEC-10, VO-PLAN-06（候補の論理/コード読解が誤り） |
| NEEDS-SPEC-JUDGMENT | 1 | VO-STATAUDIT-01（仕様の二読が両立 → Owner 判断） |
| NOT-REPRODUCIBLE | 0 | |

## 代表 CONFIRMED（dossier 抜粋）

- **VO-REGISTRY-05**: core（vtest-scan の `render_edited_test`）が adapter 所有の宣言構文を解釈・生成。§4.2 が正規に認める `/** */` 宣言では **Edit Test が100%失敗**（dry-run は壊れた rendering を ok:true で返す。rollback は containment であって rejecting gate ではない）。control/counterexample の両再現付き。
- **VO-INTAKE-08**: `vo_decomposition` が spec_refs を検査せず、`SPEC-GHOST` 参照でも PASS（fail-closed 穴）。
- **VO-INTAKE-04**: audit の SPEC subject が registered sha256 でなく現在 hash に束縛 — approval 側と非対称（resolver 非対称と同型の subsystem 分裂パターン3例目）。

## 読み方の注意（fail-closed な自己申告）

- **39/42 という確認率は高く、検証 agent 側の confirm 傾向の可能性を排除しきれない**。dossier には再現ログ・コード引用が揃っており個別の質は高いが、CONFIRMED 全39を確定欠陥として扱う前に、**サンプル抽出の再懐疑パス（または Owner による dossier 直接レビュー）**を推奨。NOT-REPRODUCIBLE が0件なのは、候補が元々コード引用ベースで生成されており到達性の弱い候補が少なかったためと考えられるが、これも上記パスで裏取りするのが安全。
- 分布の偏り: PLAN 7/8・SEMAUDIT 7/7・EXIST 4/4 が CONFIRMED（候補数は adequacy JSON の prefix 別集計で機械確認済み） — **監査受理/検証規則まわり（submission validation・coverage 判定・spec_refs 解決）に系統的な未実装/穴が集中**している示唆。個別39件は少数の root cause に畳める可能性が高く、修正計画は root-cause clustering から始めるべき。

## 位置づけ

これらは「テスト不足」ではなく **implementation が設計命題に反している（可能性が高い）反証 evidence** — 検証閉包の観点で dogfood の一級成果。修正方針の決定は P-001 どおりツール外（Owner）。

## 残パイプライン

1. （推奨）CONFIRMED 39 の root-cause clustering + サンプル再懐疑
2. NEEDS-SPEC-JUDGMENT 1件（VO-STATAUDIT-01）の Owner 判断
3. A-gap candidate 8 の上流再導出 → 必要なら freeze v3
4. adequacy 再計算 → covers 設計（auxiliary 22 件に偽 covers を書かせない）→ 適用 → 旧63 retire
