# B19+C2 上流再導出と freeze v4（160 → 171 VO・auxiliary 23 → 8）

wf_72449ef9-48b（21 agents, opus/effort-high, repo read-only）。全 dossier: `docs/plans/dogfood-b19-rederivation.json`。ontology: `docs/plans/dogfood-ontology-topdown-v4.json`（v3 の160は不変、+11 amendment。提案15→統合で12→**棄却1で11**）。

## 結果（21候補）

| 判定 | 件数 | 内訳 |
|---|---|---|
| contract **YES** | 16 | 15 が新 VO 提案・1（TEST-ARUST-005）は**既存 VO-EXEC-07 で被覆**（重複回避が正しく機能した例） |
| contract NO | 3 | TEST-STORE-008/009（parser 便宜・規範文なし）、TEST-DOGFOOD-M3-TARGET-RULES（既存 VO 群で被覆済みの断片 pin） |
| AMBIGUOUS | 2 | TEST-CLI-005 → supporting（候補 claim は gate E で自己棄却）、TEST-CLI-060 → **regression-anchor-none**（予想どおり） |

**YES 率 76% への懐疑と決着**: confirm 傾向を疑い、freeze v3 と同じ規律で**採択候補の operative 引用を全件、一次資料の grep + 該当節の実読で裏取り**（§1.3 全文・基本仕様 §3.3・詳細設計 §6.2/§6.3・E-SCAN-004 診断表を実読）。**11件で一致、1件は不一致で棄却**（下記5）。特に §1.3（内容ハッシュの定義）は「null、空文字、空listは異なる値としてencodeする」「byte range自体は…hash inputにしない」等、**当初 B 分類が見落としていた明示的規範契約が密集**しており、過小評価の原因は「helper だから」という表層分類が §1.3 の契約密度を読んでいなかったこと。

## main-thread 裁定

1. **真理条件同一性による統合 2組**: config writer v2 系の3提案（TEST-CLI-025/STORE-010/STORE-012 — 同一 claim の言い換え）→ **VO-ADAPTER-16** に統合。llvm-cov 帰属系の2提案（TEST-ARUST-004/006 — 006 は 004 の部分集合）→ **VO-EXEC-15** に統合。
2. **§1.3 系4提案は別真理条件と裁定** → 4 VO（正規化 / byte-range 除外 / null-empty 単射 / domain 分離）。
3. **ID 再割当**（agent 間衝突 5組を意味→ID の順で解消）: VO-ADAPTER-16/17, VO-EXEC-15, VO-REGISTRY-16〜21, VO-STRUCTOP-15/16, VO-PARALLEL-11。全て family 空き確認済み。
4. **留保付き採択 2件**: VO-EXEC-15 は mechanism-shape の残余リスクを agent が自己申告（§10.2 が規範的に課すため gate G 例外で採択 — Owner が「機構的すぎる」と判断する余地は gate 値に記録）。VO-STRUCTOP-15 は列挙過多の claim を節番号を含まない自己完結形へ**字句圧縮**（真理条件不変。VO-REGISTRY-19 の claim 内節番号も同様に除去 — claim に節番号を埋めない規約の維持）。
5. **棄却 1件（VO-PARALLEL-11 / TEST-STORE-018）**: 仕様の実文は「ULID payload により並列生成時のファイル名衝突を**実用上**排除する」（基本仕様 L154）— 提案 claim の絶対形「never address the same file name」は**仕様強度を超過**し、仕様強度まで弱めると決定不能（gate C 違反）。fail-closed で不採択、TEST-STORE-018 は supporting へ。
6. **VO-REGISTRY-21 は誕生時点で矛盾候補**: 「exactly-one 解決・0/2+ は E-SCAN-004」（§6.2 L927-933 + 診断表 L841 で実文確認）は、cluster 4（target-resolution-ambiguity-collapsed・CONFIRMED 済み）の `.find()` 自動選択がまさに破っている規則。既知欠陥の**直接の仕様アンカー**が立った — 修正計画で cluster 4 と紐づく。**併せて判明**: 仕様はゼロ一致と曖昧を単一コード E-SCAN-004 で括っており、cluster 4 の fix_shape にある「absent/ambiguous を別コードに」は**現仕様を超える設計提案（採るなら仕様変更）**。確定欠陥（曖昧時の自動選択 = §6.2 step 3 違反）自体はどちらでも不変。

## roles 確定（v3 設計モデル適用後の auxiliary = 8）

| test | role |
|---|---|
| TEST-CLI-005, TEST-STORE-008, TEST-STORE-009, TEST-DOGFOOD-M3-TARGET-RULES, TEST-STORE-018 | supporting |
| TEST-CLI-060 | regression-anchor-none（+ rationale） |
| TEST-STORE-020/021 | Owner 傾き=削除。残すなら supporting |
| （TEST-STORE-012） | **regression-anchor-normative + covers VO-ADAPTER-16** — anchor 論点は決着 |

## 数値正典の更新

- **171 VO**（v4）。covers 提案: **184 tests / 400 mappings / auxiliary 8**（23→8。ブリッジ: 15 tests が v4 で covers 獲得 + STORE-018 は棄却により supporting 残留。分布は再計算済み）。
- v4 由来の15 mapping は**現時点で shadow-mapping verdict を持たない**（sensor 由来）— v4 adequacy 評価で PROVES|PARTIAL rule に載せる（proposal JSON に注記済み）。
- 新11 VO の adequacy（facet 評価・contradiction duty）は次段（VO-REGISTRY-21 は既知 CONFIRMED と即結線見込み）。
- remap 適用のブロッカーは不変（auxiliary の Owner spec 判断 = role 仕様 PR）だが、**適用対象が激減**: 免除が必要なのは実質 supporting 5 + anchor-none 1 の6件（+A 2件の削除判断）。
