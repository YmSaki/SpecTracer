# test role 仕様設計案 v2（免除フラグではなく role × covers の直交設計）

Owner 方針（2026-08-16）: auxiliary は削除せず宣言で第一級化に賛成・仕様設計は要チェック。GPT-5.6-sol の整理（role taxonomy・closure 寄与規則の分離・regression の escape 穴の指摘）を取り込んだ v2。**採否・字句は Owner レビュー（spec/* PR）で確定**。syntax は概念確定後。

## 中心原理

**「すべてのテストを管理する。しかし、すべてのテストを specification conformance evidence として数えない。」**

仕様の実文がこの分離を既に支持している:
- `test_traceability`（詳細設計 L1344）の責務 = **∀t Managed(t)**（1:1 対応・ID 一意・宣言済み covers の解決）
- **∀t ∃o Covers(t,o)** は traceability でなく **Managed Test Entity の完全性定義**（基本仕様 L35「1件以上の covers」）に居る

→ 変更は entity 完全性定義の role 条件化。**traceability の判定式は不変だが、判定式が参照する完全性定義（L35）が role 条件化されるため、判定結果は変わる**（supporting が PASS 可能になる — それが狙い）。「traceability に触れずに traceability の挙動が変わる」のはこの参照構造による。dogfood はこの2命題が実証的に別物であることを示した（auxiliary 23 の存在）。

## 設計: role（目的の分類）× covers（対応宣言）× 閉包寄与（導出）

3層を分離する:
1. **role** = テストの目的分類（人間と report のための第一級 taxonomy）
2. **covers** = 対応宣言（correspondence。従来どおり）
3. **閉包への寄与** = role から直接導出**しない**。寄与は covers の有無（+ 意味監査の十分性判定）から導出する。role は covers に**制約**を課すだけ

**避けるべき設計（GPT 指摘・当初案 v1 の欠陥）**: 「role=X なら covers 不要」だけの免除フラグ。これだと `role=regression, covers=[]` が量産可能になり、規範に紐づけられる regression まで閉包の外へ逃げられる。

## role の値と covers 制約

| role | covers 制約 | 閉包寄与 | 違反時 |
|---|---|---|---|
| **verification**（省略時の既定） | **≥1 必須**（現行維持） | covers 先の VO の適合性証拠 | covers 0 → E-SCAN-007（現行どおり・後方互換） |
| **supporting** | **0 必須**（推奨案。下記 open-1） | なし（閉包の外。実行管理のみ） | covers 併存 → E-SCAN-013 |
| **regression** | **≥1、または明示の no-anchor 根拠宣言**（保護対象の挙動が規範由来なら covers 必須。規範義務が無いと主張するならその理由を宣言に書く） | covers があれば verification と同じく寄与（role は由来 metadata） | covers 0 ∧ 根拠宣言なし → E-SCAN-013 |
| **characterization** | 0（現状固定の記録。非適合性証拠） | なし | covers 併存 → E-SCAN-013 |

- **regression の escape hatch は「うるさく」する**: 「保護対象が規範的か」は機械判定不能なので、cover-less regression には明示の根拠宣言（例: `@vtest.no-anchor <理由>` — syntax 未定）を必須にする。逃がすことは可能だが、**silent には逃がせない**（宣言 = 内容ハッシュに参加 = 監査可能・grep 可能・relabel は STALE 誘発）。
- E-SCAN-013 = 「role-covers 制約違反」の族コード（message で細別）。**コード番号は spec PR 時点の空き番号を採る（013 は仮）** — E-SCAN-012 は cluster 2 修正が emitter を実装予定であり、VO-REGISTRY-15 の scope 判断（判断キュー③）の帰結次第で spec 側が別の新コードを要する可能性があるため。E-SCAN-007 は既定 role の covers 欠落のまま温存（後方互換・fail-closed 既定の維持）。
- **fail-closed の要**: role 省略 = verification。忘れによる免除は構造的に不可能。

## open 論点（Owner 判断）

1. **supporting + covers の扱い**: 推奨 = 禁止（E-SCAN-013）。最強の論拠は **covers の意味の単一性**: 禁止なら「covers がある = 閉包に寄与する」が例外なしの全称になり、verify の寄与規則が role を読まずに済む。optional 許容（GPT 原案）だと寄与判定が「covers あり ∧ role ≠ supporting」の2変数条件になり、cluster 2 型（条件分岐の under/over-inclusion）の温床になる。実例: TEST-STORE-013 は supporting 的な見た目だが v3 sweep で VO-ADAPTER-14 の facet を獲得 → 一語で verification に変えるのが正しい処置。
2. **characterization を初期版に入れるか**: 推奨 = **予約語のみ確保**（宣言されたら「未実装 role」の error。意味論は導入時に定義）。現 corpus に該当 0件の role に受入ケース・診断分岐・report 表示を生やすと死んだ仕様面になる — text-report-not-annex-a（debug printer 化）で観測済みの失敗様式。
3. **DetailedDesignContract → SupportingTest 層**（GPT 提案）: helper が実は設計契約であるケース（正規化・ハッシュ規則等）の個別 VO 化余地は残る（排他でない）。層としての仕様化は今回 scope 外を推奨。

## 仕様変更点（規範層別）

### 基本仕様
1. §2 L35（Managed Test Entity 完全性）: covers 要件を role 条件化（上表）。role の導入文。
2. §3.3 metadata 表（L317 近傍）: `role` 行追加。`covers` 行を role 条件で改訂。cover-less regression の根拠宣言を必須 metadata として追記。
3. L327 キー列挙に `@vtest.role`（+ 根拠宣言キー）を追加。
4. 12項目表（L213）: 変更不要（traceability の意味不変が本設計の要）。

### 詳細設計
5. §4.2: admitted key に `role`（反復不可。不正値は error）+ 根拠宣言キー。**注意: 新キーの追加は VO-REGISTRY-15 の scope 判断（③ = §4.2 L513 の適用範囲）と同じ PR で扱うこと** — キーを足す PR とキー検出の scope を確定させる PR が別だと L513 問題を再演する。
6. §4.4: E-SCAN-007 の適用を「既定 role の covers 欠落」と明記。**E-SCAN-013**（role-covers 制約違反、番号は仮）を新設。**宣言→entity 具体化のどの段で「構文上完全」を判定するかの条件分岐位置を明記**（現行 §4.4 は role 分岐を持たない。L545-547/L858 の ManagedTestLink::Missing 経路が supporting の covers 0 を Missing と誤判定しないよう、完全性検査の role 条件化を scanner 段で確定させる）。
7. §5.4 診断表: E-SCAN-013 行追加。
8. 検証項目の適用（supporting / cover-less regression / characterization）:
   - `test_traceability` / `test_execution` / `runtime_result` / `evidence_validity`: **通常適用**（壊れれば赤くなる。実行管理は維持 — 削除しない価値はここ）
   - `target_execution`: targets 宣言があれば通常適用
   - `test_existence` / `semantic_audit` / `static_audit` の VO 対応面: 寄与しない（covers を持つ regression は verification と同じ扱い）
9. Report: covers を持たない test は VO 子ノードでなく project レベル節に role 別で表示（Annex A の project checks 系 — cluster 9 修正と同じ着地点。依存は無いが表示先の定義だけ本仕様で確定）。

### 別紙A
10. TestEntity に `role` field（省略時 verification。field 無し = verification 読み・読取だけで書換えない）。

### 別紙C
11. 受入: (a) supporting・covers 0 → scan OK・traceability PASS / (b) supporting・covers 1 → E-SCAN-013 / (c) 既定 role・covers 0 → E-SCAN-007 / (d) regression・covers 0・根拠なし → E-SCAN-013 / (e) regression・covers 0・根拠あり → scan OK / (f) relabel → hash 変化 → evidence STALE。

## 系の性質

- **悪用耐性**: 付替えても VO 側が fail-closed（唯一の証拠を失った VO は即 uncovered）。免除種別が NOT_VERIFIED の丸め上げにならない。regression の escape は宣言必須で監査可能。
- **ハッシュ束縛**: role・根拠宣言とも source declaration 内 → 内容ハッシュ参加（基本仕様 L290）→ silent relabel 不可。

## auxiliary 23 への適用見取り図（v2）

| 群 | 処置 |
|---|---|
| B 19 | `role supporting`（covers 0） |
| C 2 | `role regression`。**TEST-STORE-012**: 規範アンカー候補 = 基本仕様 L116「writer の正規形は version 2・adapter ごとに namespace 化」（writer 側 VO は v3 に不在を確認済み）→ mini 上流再導出が成立すれば covers 獲得。**TEST-CLI-060**: アンカー無しの見込み → 根拠宣言付き cover-less regression か削除 |
| A 2 | 「verification evidence ではない」は確定（上流再導出済み）。**VO 捏造での救済は禁止**。残すなら supporting（実装品質 pin）、削除は保守判断 — Owner 傾きは削除 |

## 手順

1. 本 v2 を Owner レビュー → 概念確定 → syntax 確定
2. spec/* ブランチで4文書改訂 → PR → merge（別紙B は含めない。③ VO-REGISTRY-15 scope 判断を同 PR に同梱）
3. 実装: scanner `role`/根拠キー・E-SCAN-013・entity 完全性の role 条件化・verify 寄与規則・report 表示先
4. covers remap 適用（169 covers 書換 + 19 supporting + C/A の個別処置）→ doctor → 旧63/7 retire

**TEST-STORE-012 の mini 上流再導出は本 spec PR から切り離す** — writer 側 VO の追加は ontology 変更（freeze v4 系）で、role 仕様 PR（契約変更）とは変更系が別。remap 適用前に独立に回す（成立すれば conformance、不成立なら regression + 根拠宣言）。
