# Shadow mapping 結果（frozen 154-VO ontology × 192 managed tests, covers 不変）

wf_223a5aba-029（24 fan-out, opus/effort-high, read-only）。relevance ≠ proof を分離し、候補 VO ごとに positive / negative / ambiguity facet を判定。inventory 全量: `docs/plans/dogfood-shadow-mapping.json`。

## 双方向ヘッドライン

**Test 側（192件）**
| 分類 | 件数 | 意味 |
|---|---|---|
| proves ≥1 VO | **15** | 単独で何らかの設計命題を証明 |
| partially supports のみ | **147** | 関連するが単独では証明しない |
| proves no current VO | **30** | 現 ontology のどの命題にも PARTIAL 未満 |

**VO 側（154件）**
| 分類 | 件数 | 意味 |
|---|---|---|
| proven（≥1 test が PROVES） | **13** | 証明済み設計命題 |
| partial evidence | **88** | 断片的証拠のみ |
| touched / no evidence | **23** | 関連 test はあるが必要 facet ゼロ |
| **uncovered** | **30** | 関連 test が1件も無い |

## 支配的な gap パターン

母集団: candidate mapping（Test × VO の候補対）は全 **506**、うち verdict=PARTIAL は **349**。
- **153 candidate mappings lack negative-facet evidence / 154 candidate mappings lack positive-facet evidence**（いずれも分母は PARTIAL 349 対）。本 ontology の claim は大半が fail-closed（negative 側が load-bearing）だが、既存 suite は片面しか叩いていない — 「happy-path は fail-closed を証明しない」がスイート全体で観測された。
- uncovered 30 の内訳例: VO-PLAN 系6件（vo-coverage/spec-coverage 監査の受理・拒否規則）、VO-STRUCTOP 系5件、**VO-STATAUDIT-08/09**（runtime 到達証明 join の no-fallback / DA-003 非代替 — 今実装したばかりで専用 test 未着手の verify unit はあるが annotate 済み集合外）、VO-P001-01（pair 独立検証）、VO-REGISTRY-05。
- **単独 test ベースでは 154 VO 中 13 VO が proven**（proven の現定義 = ≥1 test が単独 PROVES）。複数 test の evidence set が composable と判定されれば次工程で増え得る — この数字は下限であって suite の証明力の総量ではない。

## covers の意味論（destructive remap 前の仕様確認 — 完了）

基本仕様の定義: `covers` は「検証する VO ID」の**対応宣言**（N:M、要件定義 §4.4）で、決定論項目 test_existence（leaf VO に対応 Test が存在するか）と test_traceability を駆動する。**十分性の主張ではない** — 「対応VOが存在しても、それだけではPASSにしない」（基本仕様 §7.4）、十分性は `vo-coverage` 意味監査（構造化理由必須＋承認）が VO 部分木×対応 Test 集合に対して判定する。∴ remap で `T1..Tn covers VO-X` と書いても「各 test が単独で証明」とは主張されず、**Evidence Set ⊨ VO の判定席は vo-coverage 監査**に既にある。
**dogfood finding**: ただし Evidence Set は第一級 record ではなく、facet ごとの test 割当（VO→facet→test）は vo-coverage 監査の理由テキストにしか残らない。証明構成の機械可読な保存形式が無いことは、SpecTracer 自身のデータモデルへの示唆。

## 位置づけ（Owner パイプライン上）

```
[完了] freeze gate → 154 VO 確定
[完了] shadow mapping（本報告）
[次]   evidence adequacy: PARTIAL 88 VO → required facets 正規化 → test 横断で候補集約
       → facet completeness → **evidence composability 判定**（欄が埋まる ≠ 証明。同一設計契約に
         対する evidence set として構成可能かを明示 gate）→ VO 毎に
         SUFFICIENT / PARTIAL / CONTRADICTED / NONE
       → proves_no_vo 30 test の分類（A ontology gap / B supporting / C regression /
         D implementation-detail / E redundant / F obsolete — B/C は存在してよい）
       → contradiction detection（evidence 不足=INSUFFICIENT と、実装が claim に反する
         実測=CONTRADICTED を区別）→ covers 意味論確認済み → 新 mapping 適用 → 旧 ontology retire
```

covers は依然不変。「Test exists ≠ Test must prove a VO」— 重要なのは VO の証明を主張する Test が本当に証明していること。旧63 VO は retire 後も比較資料として保存（bottom-up が見落とした/誤って束ねた/test cluster に引かれた obligations の分類 = test-derived traceability の危険性の自己実証）。
