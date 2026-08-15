# test role 仕様設計案 v3（role × covers × anchor の直交設計）

Owner 方針（2026-08-16）: auxiliary は削除せず宣言で第一級化に賛成・仕様設計は要チェック。v2 に対する Owner レビュー = **大枠 APPROVE + spec PR 前の3修正要求**（① anchor classification の構造化 ② test-local audit の通常適用の明文化 ③ role × topology 直交の明文化）を反映した v3。syntax は概念確定後。

## 形式モデル（確定した最終形）

- **ManagedTest(t)**: 全テストに要求（∀t Managed(t) — test_traceability の責務）
- **Role(t)**: テストの存在理由（purpose）
- **Covers(t,o)**: 規範対応宣言（normative correspondence）
- **AdequateEvidence(t,o)**: 証拠品質（意味監査・DA 規則の領分）
- **Contributes(t,o) = Covers(t,o) ∧ AdequateEvidence(t,o)** — 閉包寄与はこの2項だけで決まる
- **RoleConstraints(t)**: role は covers / anchor 宣言の**合法性だけ**を制約する
- **Role ⇏ Contribution**（role から寄与を導出しない — 本設計の不変条件）

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

| role | covers / anchor 制約 | 閉包寄与（= Covers ∧ AdequateEvidence） | 違反時 |
|---|---|---|---|
| **verification**（省略時の既定） | covers **≥1 必須**（現行維持） | covers 先の VO の適合性証拠 | covers 0 → E-SCAN-007（現行どおり・後方互換） |
| **supporting** | covers **0 必須**（**Owner 確定** — 事故史論拠により禁止で確定） | なし（閉包の外。実行管理は下記「test-local 監査」どおり維持） | covers 併存 → role-covers 制約違反 |
| **regression** | **anchor classification 必須**（下記） | anchor=normative なら verification と同じく寄与（role は由来 metadata） | anchor/covers 不整合 → anchor-covers 制約違反 |
| **characterization** | — | — | **reserved role value; not admitted by this version**（予約語。受理語彙だが本 version では宣言不可。意味論は導入時に定義） |

### regression の anchor classification（v3 で構造化 — free-text escape の廃止）

cover-less regression を「理由を書けば済む」free-text 免罪にしない。**anchor を第一級の分類宣言**にする:

```
role   = regression
anchor = normative | none
```

- **anchor=normative** → covers ≥1 必須（保護対象の挙動は規範義務 — 通常の閉包寄与）
- **anchor=none** → covers 0 必須 + rationale 必須（「この regression は normative obligation を保護していない」という**第一級の宣言**）

これにより escape は免罪文でなく監査対象の分類になり、report にも自然に出せる（例: `Regression tests — normative: N / non-normative: M`）。宣言は内容ハッシュに参加するため silent relabel 不可。anchor=none ∧ 実は規範義務あり、の誤分類は remap/監査時に anchor=none 一覧を舐めるだけで発見可能。
### 診断 ontology（番号は spec PR で振る — 013 等は仮）

failure class を潰さずに並べてから番号を振る（Owner レビュー指摘）:

| class | 例 | 性質 |
|---|---|---|
| **invalid role value** | `@vtest.role xyz` / characterization（予約語） | metadata syntax/schema validation の層 |
| **role-covers 制約違反** | `role supporting` + `covers VO-X` | semantic constraint violation |
| **anchor-covers 制約違反** | `anchor normative` + covers 0 / `anchor none` + covers 1 / anchor 欠落 regression | semantic constraint violation |

同じコード族に載せる場合も**診断意味は潰さない**（message でなく class として区別可能に）。E-SCAN-007 は既定 role の covers 欠落のまま温存（後方互換・fail-closed 既定の維持）。E-SCAN-012 は cluster 2 修正が emitter を実装予定のため番号は流用しない。

- **fail-closed の要**: role 省略 = verification。忘れによる免除は構造的に不可能。

### test-local 監査の通常適用（Auditability ≠ Contribution — 明文化必須）

**Auditability(t) と Contribution(t,o) は別物**。covers を持たないテストも監査の二級市民にならない。仕様に pin する文（趣旨）:

> All managed tests remain subject to test-local integrity, execution, freshness, and applicable static/semantic checks. A test without covers does not contribute those results to any Verification Obligation.

具体的に、supporting / anchor=none regression / （将来の）characterization にも通常適用されるもの:
- intent と実装の一致（意味監査の test-local 面）
- fixture が条件を成立させているか
- oracle が結果を実際に観測しているか（DA-001/003 系の test-local 面）
- targets 宣言があれば当該 target への実到達（DA-002 / target_execution）
- execution evidence の鮮度（test_execution / evidence_validity）

**Contribution(t,o) = 0 なだけ** — 検査値はどの VO にも算入されない。これを書かないと「supporting は audit 不要」という実装読み違えが起きる（cluster 6/7 型の読み違えの予防）。

### role × topology の直交（明文化必須）

**role = なぜ存在するか。topology = どうやって SUT へ到達するか。** 両者は独立の軸であり、規範文として pin する（趣旨）:

> Test role MUST NOT determine execution topology, target-reachability applicability, or boundary semantics.

`verification + subprocess` / `regression + subprocess` / `supporting + in-process` / `supporting + structural` — 全組合せが理論上あり得る。「targets 宣言があれば target_execution 通常適用」の規則はこの原則の系。到達性モデル（DA-002 静的 OR runtime）の適用可否も role からは決して導出しない — 過去事故（test kind と execution topology の混同を却下した経緯）の再発防止。

## 確定済み論点（Owner レビューで解決）

1. **supporting + covers 禁止 — 確定**。論拠: covers の意味の単一性（Covers(t,o) ⇒ CandidateEvidence(t,o) が role 非依存の全称で読める）。optional だと verification core に「covers あり ∧ role ≠ supporting」の余計な条件が入り、事故史（条件分岐の under/over-inclusion 19件）に照らして誤り。
2. **characterization は予約語のみ — 確定**。字句は「reserved role value; not admitted by this version」（受理語彙だが本 version で宣言不可、と読める表現。「予約語なのに error」という不正確な見え方を避ける）。
3. **DetailedDesignContract → SupportingTest 層**: 層としての仕様化は scope 外。ただし B19 は supporting 確定の**前に**軽い上流再導出を1回かける（下記見取り図 — 正規化・ハッシュ・parser 意味論は詳細設計の normative contract が既に存在する可能性がある）。原則は不変: **Test から VO を生やさない**。spec に契約があれば verification へ、無ければ supporting。

## 仕様変更点（規範層別）

### 基本仕様
1. §2 L35（Managed Test Entity 完全性）: covers 要件を role 条件化（上表）。role の導入文。
2. §3.3 metadata 表（L317 近傍）: `role` 行追加。`covers` 行を role 条件で改訂。cover-less regression の根拠宣言を必須 metadata として追記。
3. L327 キー列挙に `@vtest.role`（+ 根拠宣言キー）を追加。
4. 12項目表（L213）: 変更不要（traceability の意味不変が本設計の要）。

### 詳細設計
5. §4.2: admitted key に `role`（反復不可。不正値は error）+ 根拠宣言キー。**注意: 新キーの追加は VO-REGISTRY-15 の scope 判断（③ = §4.2 L513 の適用範囲）と同じ PR で扱うこと** — キーを足す PR とキー検出の scope を確定させる PR が別だと L513 問題を再演する。
6. §4.4: E-SCAN-007 の適用を「既定 role の covers 欠落」と明記。**role-covers / anchor-covers 制約違反の診断を新設**（番号は診断 ontology 確定後に spec PR で付与）。**宣言→entity 具体化のどの段で「構文上完全」を判定するかの条件分岐位置を明記**（現行 §4.4 は role 分岐を持たない。L545-547/L858 の ManagedTestLink::Missing 経路が supporting の covers 0 を Missing と誤判定しないよう、完全性検査の role 条件化を scanner 段で確定させる）。
7. §5.4 診断表: 上記診断 ontology の3 class を行として追加（invalid role value / role-covers / anchor-covers）。
8. 検証項目の適用（supporting / cover-less regression / characterization）:
   - `test_traceability` / `test_execution` / `runtime_result` / `evidence_validity`: **通常適用**（壊れれば赤くなる。実行管理は維持 — 削除しない価値はここ）
   - `target_execution`: targets 宣言があれば通常適用
   - `test_existence` / `semantic_audit` / `static_audit` の VO 対応面: 寄与しない（covers を持つ regression は verification と同じ扱い）
9. Report: covers を持たない test は VO 子ノードでなく project レベル節に role 別で表示（Annex A の project checks 系 — cluster 9 修正と同じ着地点。依存は無いが表示先の定義だけ本仕様で確定）。

### 別紙A
10. TestEntity に `role` field（省略時 verification。field 無し = verification 読み・読取だけで書換えない）。

### 別紙C
11. 受入: (a) supporting・covers 0 → scan OK・traceability PASS / (b) supporting・covers 1 → role-covers 制約違反 / (c) 既定 role・covers 0 → E-SCAN-007 / (d) regression・anchor=normative・covers ≥1 → scan OK・閉包寄与 / (e) regression・anchor=normative・covers 0 → anchor-covers 制約違反 / (f) regression・anchor=none・covers 0・rationale あり → scan OK・寄与なし / (g) regression・anchor=none・covers 1 → anchor-covers 制約違反 / (h) regression・anchor 欠落 → anchor-covers 制約違反 / (i) characterization 宣言 → reserved role value エラー / (j) relabel（role/anchor いずれも）→ hash 変化 → evidence STALE / (k) supporting test の test-local 監査（intent 一致・target 到達・鮮度）が通常適用されること。

## 系の性質

- **悪用耐性**: 付替えても VO 側が fail-closed（唯一の証拠を失った VO は即 uncovered）。免除種別が NOT_VERIFIED の丸め上げにならない。regression の escape は宣言必須で監査可能。
- **ハッシュ束縛**: role・根拠宣言とも source declaration 内 → 内容ハッシュ参加（基本仕様 L290）→ silent relabel 不可。

## auxiliary 23 への適用見取り図（v2）

| 群 | 処置 |
|---|---|
| B 19 | **即 supporting 確定しない** — 軽い上流再導出を先にかける: 「normative design contract が詳細設計に存在するか？ yes → verification + covers（freeze v4 amendment 経由で VO 化）/ no → supporting」。CRLF 正規化・ハッシュ正規化・parser 意味論あたりは contract 実在の可能性が高い |
| C 2 | `role regression` + anchor classification。**TEST-STORE-012**: anchor=normative 候補 = 基本仕様 L116「writer の正規形は version 2・adapter ごとに namespace 化」（writer 側 VO は v3 に不在を確認済み）→ mini 上流再導出が成立すれば covers 獲得。**TEST-CLI-060**: アンカー無しの見込み → anchor=none + rationale か削除 |
| A 2 | 「verification evidence ではない」は確定（上流再導出済み）。**VO 捏造での救済は禁止**。残すなら supporting（実装品質 pin）、削除は保守判断 — Owner 傾きは削除 |

## 手順

1. 本 v2 を Owner レビュー → 概念確定 → syntax 確定
2. spec/* ブランチで4文書改訂 → PR → merge（別紙B は含めない。③ VO-REGISTRY-15 scope 判断を同 PR に同梱）
3. 実装: scanner `role`/根拠キー・E-SCAN-013・entity 完全性の role 条件化・verify 寄与規則・report 表示先
4. covers remap 適用（169 covers 書換 + 19 supporting + C/A の個別処置）→ doctor → 旧63/7 retire

**TEST-STORE-012 の mini 上流再導出は本 spec PR から切り離す** — writer 側 VO の追加は ontology 変更（freeze v4 系）で、role 仕様 PR（契約変更）とは変更系が別。remap 適用前に独立に回す（成立すれば conformance、不成立なら regression + 根拠宣言）。
