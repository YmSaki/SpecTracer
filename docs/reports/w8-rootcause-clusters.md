# Root-cause clustering 結果（39 CONFIRMED → 10 clusters + 9 singletons = 19 独立 root cause）

クラスタ化 worker（opus/effort-high, read-only）出力: `docs/plans/dogfood-rootcause-clusters.json`。基準は「**単一の底流欠陥が全 member を説明し、一つの整合的な修正で全 member が消える**」。main thread で被覆検証と構造裁定を実施済み。

## 被覆検証（独立再検証）

CONFIRMED 39 の ID 集合とクラスタ+singleton の和集合を機械照合: **完全一致（重複0・欠落0・過剰0）**。10 clusters（member 30）+ singletons 9 = 39。

## クラスタ一覧

| # | cluster | 規模 | 確度 | root cause（一行） |
|---|---|---|---|---|
| 1 | spec-coverage-is-req-vo-linkage | 4 | high | spec_coverage 項目が SPEC 起点の audit 駆動でなく REQ→VO リンク検査として実装。`spec-coverage` audit kind が全 allow-list に不在で正規 PASS 経路が**構築不能** |
| 2 | req-vo-structural-dimension-unimplemented | 5 | medium | spec_refs 構造参照グラフの解決/検証がどこにも無い（E-SCAN-012 emitter 0箇所）。vo_decomposition は any-error の一括 gate で under/over-inclusive 両症状が同一欠落から派生 |
| 3 | audit-subject-set-snapshot-only | 4 | high | audit record 妥当性が「記録済み subject の再ハッシュ」のみで「要求 subject 集合の再導出→集合一致」が無い。bundle 側も `targets.first()`/parent 無走査で不完全集合を束縛 |
| 4 | target-resolution-ambiguity-collapsed | 2 | high | target 解決に arity 規律なし（`.find()` で曖昧一致を自動選択）。E-SCAN-004/011 は check item に届かず absent/ambiguous が NOT_CHECKED に縮退 |
| 5 | vo-coverage-not-req-anchored | 2 | high | vo_coverage が per-VO 実装で `selection.req_ids` 不読（コード裏取り済み）。VO ゼロの REQ から MISSING が導出不能 |
| 6 | structured-test-ops-not-adapter-routed | 3 | medium | adapter 分離が Structured Test 操作経路に未到達。create/edit は registry 不経由で rust adapter を静的 import、`render_edited_test` が core で `///` 構文を hard-code（= `/** */` Edit 全滅の VO-REGISTRY-05） |
| 7 | evidence-validity-adapter-axis-hardwired | 3 | medium | evidence_record_validity の adapter 軸が rust-cargo 固定。adapter-less record は無条件 STALE（compat 経路不在）、全 Test を rust_cargo_execution_state_hash で再導出 |
| 8 | target-execution-entry-set-never-read | 2 | high | `evaluate_target_execution` が集約スカラのみ読み per-target entry を不読 → 1:1 検証も per-target report も不在（下記補正あり） |
| 9 | text-report-not-annex-a | 3 | medium | text renderer が Annex A でなく debug printer（glyph hard-code・prefix 累積・basis 不読・items 分岐 dead） |
| 10 | basis-grounding-unstructured | 2 | medium | ReportItem.basis が free text で producer 側が構造化 grounding（diagnostic code/位置/Evidence ID）を破棄 |

**Singletons 9**: VO-EXIST-08（completeness 信号が構造的に不在）/ VO-INTAKE-04（SPEC subject が current hash 束縛 — 設計選択であり cluster 3 の集合完全性とは別述語）/ VO-STATAUDIT-04（DA-006 に UNKNOWN 分岐なし）/ VO-APPROVE-07（error mapping 欠陥）/ VO-PLAN-04（exclusions 不読・E-AUDIT-007 0箇所）/ VO-EXEC-06（freshness 縮退）/ VO-EXEC-07（adapter fold の優先順逆転 — 永続化済み集約も誤るため cluster 8 と非併合）/ VO-AGG-03（combine_values 迂回）/ VO-SEMAUDIT-05（audited_at 不読）。

## 裁定（main thread）

1. **クラスタ構造を承認**。straddle 判断（VO-PLAN-01 の決定脚、VO-INTAKE-04/VO-EXEC-07 の非併合、mega-cluster 棄却、renderer/producer の分離維持）は全て「一つの整合的修正で全 member が消えるか」基準に照らして妥当。「共通の背景条件」や「同型の欠陥形状」は単一欠陥ではない、という棄却理由も正しい。
2. **補正1件（cluster 8）**: root_cause の「per-target entry list is never consulted」は大域では過剰主張。entry list には既に唯一の消費者がある — evaluate_static_audit の runtime-rescue（verify/lib.rs:988、W6 実装）。`evaluate_target_execution` 自身が不読、が正確。fix 設計時は既存消費者との整合（同じ per-target 照合述語の共有）を取ること。
3. **spot-check 結果**: 新規主張のうち load-bearing な4点（spec-coverage kind 全 allow-list 不在 / E-SCAN-012 emitter 0箇所 / vo_coverage の req_ids 不読 / target_execution の entry 不読）を grep+実読で裏取りし、上記補正1件を除き**全て真**。

## 含意

- **39 CONFIRMED = 実質 19 独立 root cause**（10+9）。修正計画の単位はこの19。**ただし 39 自体が sampling 再懐疑パス未実施**（dd7dd44 の caveat: 汚染 incident により dossier の repro 文脈も要確認）— 再懐疑完了までは「confirmed 候補の root cause 単位」として扱う。clustering は再懐疑を安くした: **クラスタ層化サンプリング**（各クラスタ1件 + singleton 9件優先）で足り、本裁定の spot-check 4点が cluster 1/2/5/8 の裏取りを既に部分的に果たしている。
- 正典の優先関係: `dogfood-rootcause-clusters.json` は worker 原文であり、cluster 8 の「never consulted」過剰主張が原文のまま残る。**cluster 8 については本報告の補正（裁定2）が JSON に優先する** — JSON を機械的に読む後段の fix 計画は本節を継承すること。
- 二大テーマ: (a) **SPEC→REQ→VO intake 軸が end-to-end 未実装**（cluster 1+2 = 9件、corpus の約1/4）。(b) **diagnostic→check-item routing table の不在**（cluster 2/4 と VO-EXIST-08 の共通背景 — 単一欠陥ではないが設計欠落として記録に値する）。
- cluster 6 は adapter 分離の未完了作業そのもの（discovery.rs:638 の mid-flight コメントと整合）— 「新規バグ」でなく「既知の分離作業の残り」に分類可能。
- 修正の決定は P-001 どおり Owner 判断。confidence=medium の4クラスタ（2/6/7/9）は fix 着手前に locus の追加確認を推奨。
