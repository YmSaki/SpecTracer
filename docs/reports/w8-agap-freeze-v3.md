# A-gap 上流再導出と freeze v3（154 → 160 VO・controlled amendment）

wf_a5c2b410-844（8 agents, opus/effort-high, repo read-only）。全 dossier: `docs/plans/dogfood-agap-rederivation.json`。ontology: `docs/plans/dogfood-ontology-topdown-v3.json`（v2 の154は不変、+6 amendment）。

## 原則の再確認

Test は gap の**センサー**であって ontology の正典ではない。8候補すべてを「疑いの引用はポインタとして扱い、規範文書を上から読み直す」方式で再導出した（規範順位: 要件定義 > 基本仕様 > 詳細設計本冊+別紙A+別紙C。別紙B 非正規）。**spec に要求が無ければ追加しない** — 実際に 2/8 が棄却された。

## 判定（8候補）

| candidate | 判定 | 新 VO | 根拠の要点 |
|---|---|---|---|
| TEST-ARUST-002 | **YES** | **VO-ADAPTER-13** | wire 入力 decode の fail-closed 受理条件。基本仕様 §2.4 L126 が must-strength（「拒否し、推測で実行可能として扱わない」）、4層で反復。VO-ADAPTER-12（出力側）の入力側補完 |
| TEST-STORE-011 | **YES** | **VO-ADAPTER-14** | v1 config の in-memory 変換・正典不書換（「書き換えない」禁止形が3層）。※test 本来の主張（replace-over-merge）は規範文なし → regression pin のまま |
| TEST-SCAN-010 | **YES** | **VO-ADAPTER-15** | VCS ignore 準拠の discovery scope（本冊 §5.5）。留保: 出典が擬似コードブロック（同ブロックの E-SCAN-001 節は既に規範扱い＝一貫性で受容）。gate 値に注記 |
| TEST-SCAN-013 | **YES** | **VO-REGISTRY-15** | 不許容宣言キー＝error 重大度（E-SCAN-005/006）。留保: 2コードだが「field contract が許容しないキー」という**単一述語の2インスタンス**で §5.4 自身の分類と一致 → 1 VO と裁定 |
| TEST-SCAN-016 | **YES** | **VO-PARALLEL-10** | Relation ID の compat 正規化衝突 → E-SCAN-010・正典勝者なし（詳細設計 §3.4「いずれかを選ばない」）。VO-PARALLEL-03/04（TEST/SRC ID）の Relation 版類例 |
| TEST-VERIFY-001 | **YES** | **VO-EXEC-14** | 「最新 Evidence」= RFC 3339 offset 正規化した時系順・辞書順でない。導出2段（executed_at が鍵）を隠さず開示した上で claim 化 |
| TEST-STORE-020 | **NO** | — | 原子的公開の規範文なし。atomicity への言及は非正規の実装スケジュールのみ。規範側は reader fail-closed（E-SCAN-001/010）で torn 状態を検出する設計 |
| TEST-STORE-021 | **NO** | — | 同上（entity 置換側）。VO-PARALLEL-09 の前提を claim へ昇格させる案は agent 自身が gate で棄却（implementation-promoted） |

## main-thread 裁定内容

1. **ID 衝突の解消**: 3 agent が独立に「VO-ADAPTER-13」を提案（decode / config / scope の別 claim）。family 最大値を確認の上 ADAPTER-13/14/15 に再割当（REGISTRY-15・PARALLEL-10・EXEC-14 は空き確認済み）。ID は意味の後 — 凍結原則どおり。
2. **REQ アンカー実在確認**: 提案された5 REQ すべて既存（新 REQ 不要）。
3. **相互重複チェック**: 6 claim は主題が互いに素（Test JSON decode / config.yaml read / discovery scope / 宣言キー / Relation ID / Evidence 選択順序）。truth-condition 同一性なし。
4. **棄却2件の扱い**: TEST-STORE-020/021 は B-supporting（regression pin）に留まる。**spec gap としての surface**: 「書込み原子性は規範上不在で、reader-side fail-closed が torn 状態を検出する」— これを仕様が意図した設計と読むか、書込み側義務を追補すべきかは **Owner 判断事項**（追補しない場合、現状の VO-PARALLEL-09 は前提を暗黙に負ったまま）。
5. **freeze v3 は controlled amendment**: v2 の154は無変更、追加6件は origin `A-GAP:<test>` で来歴を明示。154を最終数として守らない、という凍結時の合意どおり。

## 帰結と残作業

- **数値正典の更新: 154 → 160 VO**。新6件の adequacy は未評価（センサー test 1件ずつが候補 evidence — facet 評価が必要）→ 次段で6件の adequacy 判定を実施し、per-VO 4値と candidate mapping に反映。
- その後: covers 設計（auxiliary 22 に偽 covers を書かせない・auxiliary-covers 張力は Owner 判断待ちのまま）→ 適用 → doctor → 旧63/7 retire。
