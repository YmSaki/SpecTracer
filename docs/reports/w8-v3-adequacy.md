# freeze v3 新規6 VO の adequacy 評価（PARTIAL 5・CONTRADICTED 候補 1）

wf_a35fae93-ba3（6 agents, opus/effort-high, repo read-only, contradiction duty 込み）。全 dossier: `docs/plans/dogfood-v3-adequacy.json`。

## 結果

| VO | final | 要点 |
|---|---|---|
| VO-ADAPTER-13 | PARTIAL | ARUST-001+002 が同一 surface で compose し F2-F5 を evidence。未証: 実行座標単独入力（F1）・両欠落拒否（F6）・無合成（F7）・他 adapter（F8）。実装自体は忠実（lib.rs:33-58 の fail-closed 分岐を確認） |
| VO-ADAPTER-14 | PARTIAL | no-rewrite facet は強い（TEST-CLI-014/015 の byte 比較）。未証: 「exactly one」の cardinality・run 値の正着・v1+`adapters:` 拒否。書換え経路の不在は grep で全数確認済み |
| VO-ADAPTER-15 | PARTIAL | 除外 path は「非 Rust バイトのファイルで E-SCAN-001 不発 = 未読取」の強い推論。未証: directory pattern・否定規則・file-valued include root。**remap 時の claim 精密化推奨**: 「VCS ignore rules」→「.gitignore（repository-local。global/info-exclude は対象外）」が §5.5 と実装（.git_global(false) 等）に整合 |
| VO-REGISTRY-15 | **CONTRADICTED 候補** | 下記 |
| VO-PARALLEL-10 | PARTIAL | duplicate 衝突は scan/CLI で evidence 済み。mismatch facet は store-parser surface のみで**構成不能**（compose しない）。「非採用」は Relation の下流消費者不在で観測不能 |
| VO-EXEC-14 | PARTIAL | **実装は無罪確定**: 鮮度判定経路は compare_evidence_recency の offset 正規化時系比較（verify/lib.rs:1822-1837, 1132-1175）。scan/operations.rs:784 の辞書順 sort は `test show` 表示専用で鮮度判定に不関与（後段検証への免罪注記）。gap: 異 offset 2レコードの統合テスト不在 |

PROVEN 0 は想定どおり — センサー test は gap 検出器であって証明器ではない（正の facet の一部しか張らない）。

## VO-REGISTRY-15 の矛盾候補（検証パス済み）

**主張**: E-SCAN-005/006 の emission（discovery.rs:971-982）が test 関数判定の早期 return（:958-960）の**後ろ**にあり、production 関数の `@vtest.` typo キー（例: `src_id` vs 正規 `src-id`）は無診断で素通り — Source Target は `src_id: None` で登録され、§4.2 L524「打鍵ミスの検出を優先し、警告ではなくエラーとする」の意図する検出が働かない。副次: `__parse_error__` の else-if により unknown+duplicate 併発時に片方のみ報告 / 該当 Test は早期 return で落ちるため「check items 非PASS」の実装 locus 不在。

- main thread でコード裏取り済み（discovery.rs:940-984 実読 — 主張どおり）。
- 8点鎖検証（隔離再現6ケース + 仕様 scope 判定, dossier: `docs/plans/dogfood-vo-registry-15-dossier.md`）: **NEEDS-SPEC-JUDGMENT**。実装の欠落動作は全ケース再現（control: test 関数の typo → E-SCAN-006 / counterexample: production 関数の同一 typo → 無診断・exit 0・doctor OK・後段 gate なし）。ただし仕様が二読可能:
  - **Narrow（test-scoped・現実装と一致）**: §4.2 L513「テスト関数直前の doc comment を対象とする」＋基本仕様 §16「Test metadata 宣言構文」。
  - **Broad（declaration-scoped）**: §5.4 の E-SCAN-005/006 行は「adapter 所有の宣言」とだけ書き（E-SCAN-007 は同表内で Test 固有に書き分け）、§4.2 L526 は `@vtest.src-id` を**非テスト関数に付与**させる — strict narrow だと L526 の打鍵ミスが検出体系の外に置かれる。
  - **Owner 論点は一つ**: L513 の scope 文を維持（現実装適合。src-id typo の未検出を明示的に受容）か、§5.4「adapter 所有の宣言」を採る（L513 の scope 拡大 + discovery.rs:958 の early return より前へ診断移設）か。
- **副次所見の判定**: (a) `__parse_error__` sentinel の else-if による **E-SCAN-005 握り潰しは scope 非依存で CONFIRMED**（test 宣言に unknown+duplicate 併存時、E-SCAN-006 のみ報告 — 単一 sentinel slot が原因。v3 波からの新規確定欠陥 +1）。(b)「非PASS locus 不在」は REFUTED — locus は verify/lib.rs:584-589 の blanket fold として実在。ただしこれは entity 帰属を持たない any-error fold であり、**既存 cluster 2（req-vo-structural-dimension-unimplemented）の over-inclusive 脚の独立再発見** = 収束の傍証。

## 数値正典の更新（160 VO）

- PROVEN 14 / PARTIAL **72**（67+5）/ UNSUPPORTED 31 / CONTRADICTED-CONFIRMED 39 / REFUTED 2（VO-EXEC-10, VO-PLAN-06 — facet 再算定は未実施）/ **NEEDS-SPEC-JUDGMENT 2**（VO-STATAUDIT-01, VO-REGISTRY-15）。確定欠陥は 19 root cause + v3 波の scope 非依存副次 1（E-SCAN-005 握り潰し）。
- 残パイプライン: covers 設計（auxiliary 22 の張力は Owner 判断待ち）→ 適用 → doctor → 旧63/7 retire。
