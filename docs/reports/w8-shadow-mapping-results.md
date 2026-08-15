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

- PARTIAL 判定候補のうち **negative facet 欠落 153 / positive facet 欠落 154**。本 ontology の claim は大半が fail-closed（negative 側が load-bearing）だが、既存 suite は片面しか叩いていない — 「happy-path は fail-closed を証明しない」がスイート全体で実証された。
- uncovered 30 の内訳例: VO-PLAN 系6件（vo-coverage/spec-coverage 監査の受理・拒否規則）、VO-STRUCTOP 系5件、**VO-STATAUDIT-08/09**（runtime 到達証明 join の no-fallback / DA-003 非代替 — 今実装したばかりで専用 test 未着手の verify unit はあるが annotate 済み集合外）、VO-P001-01（pair 独立検証）、VO-REGISTRY-05。
- 「テストが通っている」→「何が証明され、何が証明されていないか」への移行が数値で完了: **suite は 192 test で 154 命題中 13 しか証明していない**。

## 位置づけ（Owner パイプライン上）

```
[完了] freeze gate → 154 VO 確定
[完了] shadow mapping（本報告）
[次]   evidence adequacy 深掘り: PARTIAL 88 VO の facet 合算で「複数 test の結合で sufficient になる VO」を判定
       / gap・矛盾・orphan の分類確定 → 新 covers 設計 → 旧 ontology retire
```

covers は依然不変。proves_no_vo=30 の test 群は「不要な test」ではなく、(a) ontology の欠け（structural/plumbing 命題の未定義）か (b) 真に設計命題を証明しない test かを retire 前に分類する。
