# Test の実行形態を分類しない — Owner 裁定の適用（2026-09-07）

対象: `docs/canonical/specification.json`（正本）と `docs/canonical/relations/retired-ids.json`（退役台帳）
基点コミット: `71b08b4`
根拠: Owner 裁定 2026-09-07、Issue #14 [issuecomment-5565960636](https://github.com/YmSaki/SpecTracer/issues/14#issuecomment-5565960636)

---

## 0. 裁定と、正本への転記

Owner の逐語:

> テストがこれを検証しますよ。で済む問題じゃないの？
> ブラックボックステストもホワイトボックステストもテストをするという抽象面では同一であり、動作に対してどのように動いたかを見ればいいし、それが網羅されているものを全部通過したら合格と見做せるわけで。

`ROOT-049` として root 層に転記した。statement は上の 2 文そのもの。`derives_from` は root なので持たない。`source` は上記コメント URL。

`description` に PM の適用方針を 4 項目で書いた。要旨:

1. Test の実行形態（黒箱 / 白箱、内部 construct 検証 / 境界の振る舞い検証）は分類しない。形態別の規範・形態別の必須条件・形態を宣言するキーを設けない。
2. Test が宣言するのは「何を検証するか」（`covers` → VO）。`target` は義務ではなく「どこが動いたか」の観測を束縛する任意の宣言。target を持たない Test の `target_binding` は `NO_EVIDENCE`（診断 `NOT_CHECKED`、`DS-1664`）。宣言した target は既存の到達性規範で検査する。
3. 網羅した検証範囲を全部通過したら合格。範囲外・未観測は `PASS` にしない。
4. `E-SCAN-007` は退役させない（§1 参照）。

---

## 1. 依頼との相違 — `E-SCAN-007` はコードごと廃止しなかった

**依頼は「E-SCAN-007（target 未宣言）は廃止」だった。これは適用していない。** 一次資料が、このコードは target 専用ではないと示している。

| ノード | 逐語 |
|---|---|
| `DS-1621`（適用前） | 「E-SCAN-007はerrorであり、必須metadata（core中立: id / covers ≥ 1 / intent、および当該adapterが必須とする追加metadata。`rust-cargo` では検証対象をSource Targetとして実現する実行形態について targets ≥ 1）の欠落を意味する。」 |
| `DS-1256` | 「すべての管理対象 Test に `covers ≥ 1` を一律要求するため、`covers` を宣言しない Test は E-SCAN-007 と `chain_integrity = MISMATCH`（診断`MISSING`）になる。」 |
| `DES-229` | 「adapterはTest構文の違反（重複不可キーの重複、未知キー、必須キーの欠落）をE-SCAN-005 / E-SCAN-006 / E-SCAN-007で報告する（§5.4）。」 |

コードを退役させると、`covers ≥ 1` 欠落の報告経路と、adapter が構文違反を報告する経路が同時に消える。**どちらも Owner 裁定が触れていない規範であり、裁定に無い規範を消すことになる。**

適用したのは、必須 metadata の列挙から **`targets ≥ 1` の条項だけ**を外すこと。コード、`ManagedTestLink::Missing` への写像、`chain_integrity = MISMATCH`（診断 `MISSING`）は無改変。この判断は `ROOT-049` の `description` (4) と、team-lead へのメッセージで開示済み。

依頼との相違はもう 1 点ある。依頼 (c) は `REQ-150` / `SPEC-085` を「1 件以上の Source Target を宣言する義務」として挙げていたが、**両ノードは適用前から「宣言できる」という許可形であり義務ではない**。したがって無改変とした（§3 の「そのまま」欄）。実際に義務を述べていたのは `DS-063` / `DS-1575` / `DES-287` / `DES-384` の側で、そちらを下限 0 に書き換えた。

---

## 2. 掃引

`statement` と `description` の両方を、全 7 層・全 3850 ノード（適用前）に対して走査した。語: 実行形態 / 外部契約 / 境界上の振る舞い / 境界検証 / Source Target として実現 / E-SCAN-007 / 1 件以上の Source Target / 1件以上 / 黒箱 / 白箱 / ブラックボックス / ホワイトボックス。2 巡目で追加: 境界形態 / 非source / 内部 Source / target を宣言しない / integration / Contract-Target / 形態。

該当語を含むノードは 92 件。うち**処置が要るのは 44 件**だった。残り 48 件は「1件以上」が VO の `derives_from`・`covers`・承認レコード件数など無関係の文脈で出たものである（`SPEC-015` / `SPEC-035` / `DS-106` / `DS-107` / `DS-209` / `DS-213` / `DS-391` / `DS-396` / `DS-801` / `DS-807` / `DS-889` / `DS-890` / `DS-1161` / `DS-1200` / `DES-117` / `DES-167` / `DES-224` / `DES-241` / `DES-336` / `DES-463` / `DES-589` ほか）。

**分類の基準**（適用前に固定し、途中で変えていない）: 語の出現ではなく、**そのノードの条件節・適用範囲が「Source Target 形態か境界形態か」の二分に依存しているか**。依存していれば処置対象。「境界上の振る舞いも検証対象にできる」のように Test を分類せず対象の範囲だけを述べるものは対象外。

---

## 3. 全該当ノードと処置

### 3.1 退役（後継なし・`new_id: null`）— 21 件

| id | 層 | 理由 |
|---|---|---|
| `REQ-070` | require | 他の実行形態における確認方法の下位仕様への委譲。委譲先の「他の形態」が存在しない |
| `REQ-071` | require | 特定形態の確認方法を別形態へ一律要求しない、という形態間規範。前提が消滅 |
| `REQ-132` | require | `REQ-071` と同旨 |
| `REQ-147` | require | 内部 construct を直接検証する実行形態についての規範。`REQ-336` が無条件になったため不要 |
| `SPEC-065` | spec | 他の実行形態における確認方法の詳細設計への委譲 |
| `SPEC-301` | spec | 実行形態別の確認方法の委譲。無条件部分は `SPEC-465` が引き継ぐ |
| `DS-128` / `DS-167` | detailed_spec | `REQ-071` / `REQ-132` の詳細設計面の写し |
| `DS-176` / `DS-177` | detailed_spec | 内部 construct を直接検証する形態の規範（二重宣言を要求しない、を含む） |
| `DS-178` | detailed_spec | 境界形態を検証対象とする規範 |
| `DS-1622` | detailed_spec | 実行形態別に `E-SCAN-007` の適用可否を分ける規範 |
| `DS-1633` / `DS-1634` / `DS-1635` | detailed_spec | `rust-cargo` が当該形態について `targets ≥ 1` を必須とする規範 |
| `BD-183` | basic_design | 「Source Target として実現するか境界の振る舞いとして実現するかは実行形態が定める」— 分類そのもの |
| `BD-185` | basic_design | 非 source の境界形態の表現・確認方法の委譲 |
| `BD-315` | basic_design | `targets ≥ 1` の必須が adapter 層に属する、という規範 |
| `BD-316` | basic_design | 当該形態でありながら `targets` を宣言しない Test を fixture が表現できる、という規範 |
| `DES-580` | design | 形態別に `targets ≥ 1` を課す / 課さない |
| `DES-581` | design | `targets ≥ 1` を `E-SCAN-007` 経路で検出する形態別規範 |

### 3.2 新 id（規範変更）— 20 件

| 新 id | 前身 | 変更 |
|---|---|---|
| `REQ-336` | `REQ-148` | 「target の宣言は Test 成立性の必須条件ではない。」条件節を落として無条件化 |
| `REQ-337` | `REQ-068` | 到達の確認方法の条件を「実行形態」から「target を宣言した Test」へ |
| `SPEC-464` | `SPEC-300` | 到達要件の限定を「target を宣言した Test」へ |
| `SPEC-465` | （新規） | `REQ-336` の基本仕様面の写し |
| `DS-1665` | `DS-1620` | 必須 metadata の列挙から `targets ≥ 1` を除去 |
| `DS-1666` | `DS-1621` | 同上（`E-SCAN-007` の定義文） |
| `DS-1667` | `DS-1624` | 同上（Test 層の評価） |
| `DS-1668` | `DS-1625` | 同上（表示と `MISMATCH` 導出） |
| `DS-1669` | `DS-1644` | 同上（`chain_integrity` の `PASS` 条件） |
| `DS-1670` | `DS-120` | 確認方法の条件を「target を宣言した Test」へ |
| `DS-1671` | `DS-179` | 「target の宣言は Test 成立性の必須条件ではない」を無条件化 |
| `DS-1672` | `DS-685` | 到達未確立の帰結の条件を「target を宣言した Test」へ |
| `DS-1673` | `DS-063` | Test が持つ Source Target の下限を 1 → 0 |
| `DS-1674` | `DS-1575` | writer が常に出力する `targets` list の下限を 1 → 0 |
| `DES-591` | `DES-582` | `targets` 件数は adapter が定める。`rust-cargo` の `targets ≥ 1` の括弧を削除 |
| `DES-592` | `DES-384` | エッジ `TEST → SRC` を 1:N → 0:N、実行形態の限定を削除 |
| `DES-593` | `DES-287` | writer 出力の `targets` list 下限を 1 → 0 |
| `DES-594` | `DES-084` | construct bytes 束縛の条件を「target を宣言したか」へ |

`REWORD` と新 id の判別に使った基準: **前身と後継を読んだ実装者が、ある入力に対して違う振る舞いを選びうるか。** 選びうるなら新 id。`REQ-068` → `REQ-337` は選びうる（境界形態とされていた Test が target を宣言していた場合、前身では確認方法の対象外、後継では対象）。

### 3.3 REWORD（id 維持）— 3 件

| id | 変更 |
|---|---|
| `REQ-083` | 「（内部 construct 検証か境界の振る舞い検証か）」の括弧書きを削除。判定規則は元から形態非依存で、括弧は分類の例示だった |
| `DS-139` | `REQ-083` と同文。同じ括弧を削除 |
| `DS-1664` | `description` のみ。退役した `SPEC-065` / `SPEC-301` / `REQ-071` を根拠にしていた記述を「宣言が無いので束縛すべき対象が無い」へ書き直した。**statement（`NO_EVIDENCE` / `NOT_CHECKED`）は不変。** `derives_from` を `REQ-064` / `REQ-135` / `REQ-336` / `ROOT-049` へ付け替え |

### 3.4 そのまま（③）— 主なもの

| id | なぜ変えないか |
|---|---|
| `REQ-051` / `REQ-084` / `REQ-131` / `DS-102` / `DS-140` / `DS-166` | 「答えは実行形態に依らず同一」「実行形態別の判定規則を設けない」。裁定と同方向であり、分類を前提していない |
| `REQ-144` / `DS-175` | 「1 件以上の検証対象を宣言**できなければならない**」。可能性の要件であって宣言の義務ではない |
| `REQ-146` / `SPEC-022` / `SPEC-084` | 境界上の振る舞いも検証対象に**できる**。Test を分類しない。`derives_from` から退役 id を落としただけ |
| `REQ-150` / `SPEC-085` | 「1 件以上の Source Target を宣言**できる**」。許可（§1 参照） |
| `DS-560` / `DS-1256` / `DES-229` | `E-SCAN-007` の一般経路。§1 |
| `DS-482` / `DES-244` / `DES-481` | core は `targets ≥ 1` を中立必須にせず空 list を許容する。裁定と同方向 |
| `DS-769` / `DS-770` / `DS-771` / `DS-1409` | §4 の嘘テスト 1 の要。下記 |
| `DS-687` / `DS-343` / `DS-660` / `DS-661` | 分類を前提していない |

**`DS-771`「Test単位集約は、1件以上の全宣言targetがPASSなら`PASS`とする。」と `DS-1409` から「1件以上」を外していない。** target 0 件のとき「全宣言 target が `PASS`」は空虚に真になり、`PASS` へ倒れる。この限定語が空虚な真を塞いでいる。

### 3.5 開示 — 変えなかったが齟齬が残るノード

`DS-480`「検証対象は一般概念であり、adapter中立coreは各管理対象Testに1件以上の検証対象を要求する。」

上流の `REQ-144` は「宣言**できなければならない**」（可能性）だが、`DS-480` は「**要求する**」（義務）と読める。**この不一致は本裁定より前から存在し、本裁定が触れていない。** `検証対象` は `Source Target` より広い概念（`SPEC-082` / `SPEC-083`）なので、`covers ≥ 1` の言い換えとも読める。本適用では `derives_from` から退役 id を落としただけで statement は変えていない。上流へ差し戻す価値のある齟齬として記録する。

---

## 4. 嘘テスト — 3 つの経路が `PASS` に到達しないこと

残った条文の逐語だけで示す。

### 4.1 target を持たない Test

1. `DS-1664`「targetを持たないTestの`target_binding`は`NO_EVIDENCE`（診断`NOT_CHECKED`）とする。」
2. `DS-1511`「集約は fail-closed とし、子に 1 つでも非 `PASS` があれば親は非 `PASS`。」`NO_EVIDENCE` は `PASS` ではない。
3. `REQ-295`「完全検証における OK は、宣言鎖全体に対する検査（chain_integrity / orphan_detection）と、scope に含まれる各「宣言 + コード + 証拠」の組に対する検査（target_binding / oracle_presence）が**すべて `PASS`** であり…場合に限る。」
4. 補強: `DS-687`「targetを持たないTestは本節の合成へ到達しない。」`DS-771`「1件以上の全宣言targetがPASSなら`PASS`とする。」— 0 件では前件が成立しないため、空虚な真で `PASS` にならない。

→ **到達しない。** `chain_integrity` は通る（`DS-1667`：必須は Test ID・`covers ≥ 1`・`intent`）。`target_binding` が `NO_EVIDENCE` で止まる。

### 4.2 到達しない target を宣言した Test

1. `DS-1672`「targetを宣言したTestが宣言targetをどのtopologyでも実行しない場合…静的にもruntimeにも到達を確立できず、到達要件は未充足のままとなる。」
2. `REQ-069`「複数 target を宣言した Test では各 target の実行を個別に計測し、1 件でも実行回数が 0 なら `FAIL`（診断: NOT_EXECUTED）、1 件でも解析不能でかつ `FAIL` がなければ `UNKNOWN`、**全 target の実行を確認できた場合だけ `PASS`** とする。」
3. `DS-1511` の fail-closed 合成 → 親も非 `PASS`。
4. `REQ-135`「確認不能であることだけを根拠として…成立確認済みとして扱ってもならない。」

→ **到達しない。** `FAIL` または `UNKNOWN`。

### 4.3 網羅範囲外の Test / 検査

1. `REQ-100`「scope 限定により検査を実施しなかった項目は、完全検証の集約時に状態 `NO_EVIDENCE`、診断ラベル NOT_CHECKED となる。」
2. `DS-956`「scope 外・未実施の検査は集約ツリー内で `NO_EVIDENCE`（診断 `NOT_CHECKED`）として保持する。」`DS-1499` / `DS-083` / `DS-840` も同旨。
3. `DS-1511` の fail-closed 合成、`DS-1536`「エンティティ軸の部分木が全 `PASS` でも構造検査が非 `PASS` なら代表値は非 `PASS` になる。」
4. `DS-288`「代表値の優先順位は `FAIL > MISMATCH > NO_EVIDENCE > UNKNOWN`。」非 `PASS` が代表値になる。

→ **到達しない。**

**示せなかった経路は無い。**

### 4.4 帰結の開示

上の 4.1 から、**target を宣言しない Test を 1 件でも含む repository は、v0.1 では完全検証 OK に到達しない**（`DS-1664` の `description` が同じことを書いている）。裁定の適用方針 (3)「範囲外・未観測は `PASS` にしない」の直接の帰結であり、意図どおりである。

ただし帰結として記録しておく: 旧仕様の dogfood で観測された「境界だけを検証する Test に偽のターゲットを 84 件作らせた」圧力は、**これで解消していない**。`E-SCAN-007` / `chain_integrity = MISMATCH` は出なくなるので Test は成立するが、`target_binding` が `NO_EVIDENCE` に留まるため完全検証 OK は取れない。偽ターゲットを書けば `PASS` に届く、という誘因は残る。**これは提案ではなく事実の報告である。**

---

## 5. 据え置き（触っていない）

Issue #14 が CLI / MCP 面を先送りしているため、以下は本適用の対象外とした。いずれも別紙A（`§12`〜`§15`）に属する Form の入出力規定である。

| id | 内容 |
|---|---|
| `DS-1241` | `rust-integration` は組込 Form であり、単一の `target` field に代えて、1件以上のロケータを持つ `targets` を必須入力として受け取る |
| `DS-1242` | `rust-integration` は `file` を `required:true` とする |
| `DS-1245` | `rust-integration` の §14.1 との差分はこの 2 点であり、他は同一 |
| `DS-1246` | `rust-integration` は `targets` の全要素を入力順に個別の `@vtest.target` 行として出力する |
| `DS-1247` | `rust-integration` は空 list と重複 target を E-OP-001 で拒否する |
| `BD-275` | fixture は、複数 target を宣言し、target ごとに PASS / FAIL / UNKNOWN が異なる integration Test を表現できる |
| `DES-434` / `DES-435` | `rust-cargo` が `suite.kind` を `lib` / `bin` / `integration` として解釈する |

**`DS-1241` と `DS-1247` は裁定と緊張がある。** Form の入力として `targets` を 1 件以上必須にし、空 list を `E-OP-001` で拒否することは、target を書かない Test をこの Form では作れないことを意味する。ただしこれは **Form の入力規定**であって Test 成立性の条件ではなく、`@vtest.target` を書かない Test は別の Form で作れる。Issue #14 が CLI 面を先送りしている以上、ここを本適用で動かすと裁定に無い判断を下流で埋めることになるため、据え置いた。**CLI / MCP 面を開くときに再検討が要る点として記録する。**

`DES-434` / `DES-435` / `BD-275` の `integration` は Cargo の suite 種別であって Test の実行形態の分類ではないため、裁定の対象外である。

---

## 6. 機械検査

各コミットで実測した。検査項目は先行報告（`reports/section-hash-order-2026-09-07.md` §10.8 ほか）と同一。

| 検査 | 適用前（`71b08b4`） | 適用後 |
|---|---|---|
| `specification.schema.json` に適合 | OK | **OK** |
| ノード総数 | 3,850 | 3,831 |
| id の重複 | 0 | **0** |
| dangling な `derives_from` | 0 | **0** |
| 上位層のみを参照（下位・同層への辺） | 0 件の違反 | **0 件の違反** |
| 退役 `old_id` の生存 | 0 | **0** |
| 退役 `old_id` の再利用 | 0 | **0** |
| `root` 外で実効的上流（自ノード ∪ 祖先の辺）が空のノード | 270 | **269** |

**⑧ の 270 → 269 について。** 集合を突き合わせた結果、**離脱 1 件（`SPEC-300`）、新規流入 0 件**である。`SPEC-300` は `derives_from` が空で祖先の辺も無い孤児だった。後継 `SPEC-464` は `ROOT-049` への辺を持つため孤児集合を抜けた。**依頼は「270 で不変」を条件としていたが、269 が正しい。** 孤児を 1 件減らしたのは本適用の副次的な改善であり、無理に 270 へ戻していない（戻すには後継から Owner 裁定への辺を外すことになり、根拠が消える）。

各コミットは単体で dangling 0 である。退役 id を指していた辺は、その id を消すコミットと同じコミットの中で付け替えた。

---

## 7. コミット

| hash | 内容 |
|---|---|
| `d971029` | `ROOT-049`（Owner 裁定の転記） |
| `72bcb32` | require + spec 層。退役 id への辺 34 ノード分の付け替えを同梱 |
| `02b3406` | detailed_spec 層 |
| `7e5f64a` | basic_design + design 層 |
| （本コミット） | 本報告書と `reports/boundary-form-observable-2026-09-07.md` §9 |

`docs/canonical/export/*.md` は再生成していない。先行の正本改訂コミット `e04b6b9` も再生成しておらず、`AGENTS.md` が「生成された読み物であり権威を持たない（JSON に遅れることがある）」と定めているためである。
