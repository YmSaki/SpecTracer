# covers remap 提案（非破壊 — 適用は auxiliary の Owner 判断でブロック）

導出物: `docs/plans/dogfood-covers-proposal.json`（main thread の機械導出。agent 判断なし — 入力は凍結済みの shadow mapping + v3 adequacy）。

## 設計 rule

**covers = correspondence 宣言 = shadow mapping で verdict PROVES または PARTIAL が付いた (test, VO) 組**（ontology v3 = 160 VO 基準）。

- 仕様の covers 意味論に整合: covers は対応宣言であって証明主張ではない。**十分性の判定は vo-coverage 意味監査の仕事**のまま — facet が部分的でも対応は真であり、宣言してよい。
- verdict NO（relevance のみ・facet なし）は correspondence ではない → 書かない。
- **auxiliary には covers を一切書かない**（偽 covers 禁止の Owner 制約を fail-closed に満たす）。

## 数値

| 項目 | 値 |
|---|---|
| managed tests | 192 |
| covers を持つ test | **169** |
| auxiliary（covers ゼロ） | **23** = B-supporting 19 + C-regression 2 + A-棄却（spec 沈黙）2 |
| 総 mapping 数 | **385**（旧候補 506 から NO verdict を落とした数） |
| 分布（covers/test） | 1:61, 2:45, 3:32, 4:19, 5:11, 7:1 |
| v3 amendment 由来の新規 mapping | 18 tests → 6 新 VO（sensor 6 + 周辺 12） |

## 適用（destructive remap）を停止する理由

適用すると 192 test の `@vtest.covers` 注釈が書き換わり、**auxiliary 23 test は covers を失って test_traceability MISSING で赤くなる**。これは隠蔽より正しい（fail-closed）が、その赤の最終的な扱いは Owner が明示的に保留した spec 判断そのもの:

- 選択肢 a: covers 免除種別（auxiliary の第一級表現）を仕様に追加 → MISSING にならない
- 選択肢 b: auxiliary を未管理化 → 管理外に出す（W-SCAN-101 側へ）
- 選択肢 c: MISSING を named risk として受容（検証閉包 thesis の risk acceptance レイヤー）

どれを採るかで remap の適用形が変わるため、**適用は Owner 判断後**。判断が出れば、proposal JSON → 各 test ファイルの注釈書換 → doctor → 旧 63 VO / 7 REQ retire（比較資料として保存）まで一気に実行可能。

## 副産物

- 旧 covers はすべて旧 63 VO 体系への参照であり、適用時に全 192 test で置換となる（増分適用の余地なし — これは旧体系が test 起点で生成されていたことの帰結で、「test 由来 traceability の危険」の比較資料そのもの）。
