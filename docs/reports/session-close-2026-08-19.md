# セッション終了記録（2026-08-19）— Issue #11 凍結台帳を正典として再設計へ移行

Owner 決定: **Issue #11（v0.1 spec 凍結台帳）を正とし、Owner 自身が設計し直す。** 本記録は再設計時に参照するための最終状態の台帳。

## 正典の所在

- **最上位**: https://github.com/YmSaki/SpecTracer/issues/11 — 検査4本（chain_integrity / orphan_detection〔文書層のみ〕/ target_binding / oracle_presence）・状態5つ（PASS/FAIL/MISMATCH/NO_EVIDENCE/UNKNOWN）・総称 document ノード + derives_from。矛盾する実装・文書変更は MISMATCH。
- 旧規範文書群（要件定義/基本仕様/詳細設計/別紙A/C、PR #6/#7/#9 反映済み）は**再設計の素材**であり正典ではない。

## ブランチ / PR の最終状態

| 対象 | 状態 |
|---|---|
| develop | PR #6（role仕様）/ #7（typo検出・file required:true・原子性）/ #9（§11/§16/§17 検証成立性）merge 済み |
| **PR #10**（§8 検証対象/traceability 分離） | **open のまま**。A′ の精神（検証対象の一般化・traceability=関連付け・相互推定禁止）は台帳 F10/F12 と整合するが、処遇は再設計側の判断 |
| feature/adapter-separation-alpha2-implementation | 19/19 root cause 修正 merge 済み・workspace gate 緑。docs/plans, docs/reports の全記録はここ |

## 実装の既知未処理（再設計と独立に実在する欠陥・債務）

1. **records.rs の filter_map silent drop**（vtest-store）: target 欠落 entry を parse 時に黙って落とし、wave-4 の entry-set 厳格化を迂回可能。修正案B（落とさず不一致値として保持→述語が set ごと無効化）まで合意済み・未着手。
2. **TEST-CLI-098 重複**（src/lib.rs と m1_acceptance.rs、E-SCAN-002）: base 既在。covers 参照があるため改番は remap 相当の作業時。
3. repo-root scan は W-SCAN-102×131 等で NG — 旧 registry（63+bridge5 VO）への fail-closed 可視化として意図的に放置中。
4. spec-audit-subject v1→v2 bump 済み（既存 "spec" subject 監査は再 bundle まで STALE。Owner 事後承認済み）。
5. 仕様先行・実装未追随分: W-SCAN-105 emitter / src-id 誤配置・重複診断 / rust-integration file 導出撤去確認。→ ただし台帳の下ではこれら仕様自体が再導出対象。

## 再設計の素材として生き残る資産（測定データ — 仕様ではない）

- docs/plans/dogfood-*.json: 171 VO ontology (v4) / covers 提案 183/399/9 / 39 CONFIRMED 検証 dossier / root-cause clusters / shadow mapping / adequacy
- docs/reports/: w8 系報告書群・verification-closure-thesis（旧 thesis — 台帳 F 群が上書き）・**spec-freeze-blind-review.md**（盲検5観点。台帳の診断「同型検査の増殖」を独立に裏付けた findings を含む — A系の文書間矛盾・B系の要件穴・F系の meta-leak〔特に「version 1」互換の開発史 leak〕は旧文書を素材にする際の注意書きとして有効）
- docs/plans/owner-decision-queue.md ほか裁定記録: **歴史文書**。個々の裁定（A′、検証コンポーネント、trust boundary、両側 epistemic 等）は台帳と整合する限りで再設計の参考

## プロセス教訓（Issue #11 末尾、以後の作業規範）

要件は抽出でなく決定 / AI は尋問官と書記であって著者ではない / PR リジェクト連発 = 要件発見が最下流で起きている兆候 / 未定義の直感を AI に渡さない / 増殖を見たら同型操作の N 回インスタンス化を疑う。
