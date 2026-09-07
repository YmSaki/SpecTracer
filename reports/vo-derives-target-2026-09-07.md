# VO の `derives_from` entry は何を指すか — 逐語・判定・適用（2026-09-07）

## 0. 枠

**対象**: `docs/canonical/specification.json` のうち、VO レコードの `derives_from` entry の参照先（値域と粒度）を定める条文。

**目的**: 実装 PR #21 の修正者が止まった論点、すなわち文書モデルの再整合（`ROOT-047` / `ROOT-048`、`DOC-` の値域の死亡、`BD-330` / `DES-585` により文書が識別子 field を持たないこと）のあとで、VO の `derives_from` entry の `doc` field が何を保持するのかを、正本の中で一意に読めるようにすること。

**軸（調査中に変更しない）**:

1. 逐語。要約・報告書・テスト名からは断定しない。
2. 層の権威。上位の明文があればそれに従い、無ければ導出であることを明記する。
3. 嘘テスト。規則の可否は「その規則で偽のリンク・偽の PASS が通るか、鎖の切れ目が隠れるか」で決める。
4. 明文と導出を混ぜない。

**除外**: `DS-1022` / `DS-1026` ほか Issue #14 で先送りされた CLI / MCP 面（`reports/canonical-holes-2026-09-07.md` §10.6 の据え置き集合）。`docs/canonical/export/*.md`。実装コードとテスト。

**基準 HEAD**: `fc35e58`。

---

## 1. 逐語

### 1.1 上流（require / spec）

| id | 層 | 逐語 |
|---|---|---|
| `REQ-026` | require | ノード（文・節）間のリンクは `derives_from` の一種のみとする。 |
| `REQ-028` | require | VO は 1 件以上の `document` から derives_from で導出される。 |
| `REQ-054` | require | VO 層では、各 VO が 1 件以上の `document` への解決可能な derives_from を持つことを要求する。 |
| `REQ-334` | require | 文書層では、各ノードの `derives_from` 参照先が存在することを要求する。 |
| `SPEC-013` | spec | `derives_from` は上流ノードへの参照である。 |
| `SPEC-015` | spec | VOは1件以上のdocumentからderives_fromで導出される。 |
| `SPEC-035` | spec | VOは1件以上の `document` からderives_fromで導出される。 |
| `SPEC-348` | spec | projection が出力する `derives_from` エッジは到達先の上流ノード id（節または文）を伴い、「どの上流条項が、どの概念（VO）へ対応するか」の対応ペアが構造化出力として取得でき、外部の発見者が未宣言の義務・網羅漏れを裁定する材料になる（基本仕様 §11.1）。 |

**`SPEC-013` / `SPEC-348` / `REQ-026` の現行逐語の出所（重要）**。3 件はいずれも 2026-09-06 の Issue #14 再整合コミット `c8c6ec1` が **id を維持したまま書き換えた**ものであり、退役した md の逐語ではない。再整合前の逐語は次のとおり（`git show c8c6ec1^` で実測）。

| id | `c8c6ec1` 以前 |
|---|---|
| `REQ-026` | 文書間のリンクは `derives_from` の一種のみとする。 |
| `SPEC-013` | derives_fromは上流documentから下流documentへの導出を表す。 |
| `SPEC-348` | projectionが出力する`derives_from`エッジに当該entryの`anchor`を常に同伴させることにより「どの上流条項が、どの概念（VO）へ対応するか」の対応ペアが構造化出力として取得でき、外部の発見者が未宣言の義務・網羅漏れを裁定する材料になる（基本仕様 §11.1）。 |

3 件とも `relations/retired-ids.json` に退役記録が無い（`SPEC-348` / `SPEC-013` / `REQ-026` は `new_id` として台帳に現れない）。したがって「基本仕様 v0.1 の原文にそう書いてあった」とは言えない。**言えるのは、現行の正本において spec 層と require 層の明文がそう定めている、ということだけである。** 本書はその意味でのみ「明文」と呼ぶ。

### 1.2 基本設計（basic_design）

| id | 逐語 |
|---|---|
| `BD-165` | `derives_from` は上流ノードへの唯一のリンク種別である。 |
| `BD-171` | 「どの上流条項がどのVOへ対応するか」の対応ペアは、`anchor` 付き `derives_from` エッジとして保持し、§11.6のprojection出力で露出する。 |
| `BD-313` | 検証グラフでは、ノード間・VO → ノードの関係を `derives_from` の一種で表現する（§19）。 |
| `BD-318` | ノードの ID は層ごとの接頭辞（`ROOT-` / `R-` / `P-` / `REQ-` / `SPEC-` / `DS-` / `BD-` / `DES-`、節は `-S` を伴う）とし、正典は `.verify/doc/` に置く。 |
| `BD-330` | 上流文書のファイル名（拡張子を除く）は当該文書を指す名前として利用者（人間または AI）が命名するものとし、機械生成の識別子（ULID）としない。 |

`BD-318` は `BD-019`「document の ID は `DOC-` とし」の後継である（台帳。理由: `DOC-` は `$defs/id` に無く、`ROOT-047` のもとで死んだ値域）。**「document の ID」がそのまま「ノードの ID」へ改まっている**ことが、`document` が総称ノード型の名であることの正本内の裏づけになる。もとの定義は `ROOT-037` F11:

> 上流文書は全て **単一の総称ノード型** `document` (id + path + content_hash + 上流参照)

### 1.3 詳細仕様（detailed_spec）

| id | 逐語 |
|---|---|
| `DS-1594` | ノードの `derives_from` は上流ノード id の並びであり、参照先の該当箇所を指す `anchor` field を entry ごとに持たない。 |
| `DS-1595` | ノードの `derives_from` は、導出理由を記す `note` field を entry ごとに持たない。 |
| `DS-1638` | VOの `derives_from` entry は、任意の `anchor`（参照先ノード内の該当箇所を指す不透明な文字列。節番号・条項番号・見出し等）と任意の `note` を持つ。 |
| `DS-391` | VO レコードの `derives_from` fieldは1件以上のdocumentへの直結を表す。 |
| `DS-396` | VOは1件以上の `document` から `derives_from` で導出される。 |
| `DS-397` | `derives_from` の参照先documentが存在しなければ、`chain_integrity` の `MISMATCH`（dangling reference。E-SCAN-003相当はE-SCAN-012）とする（§5.4）。 |
| `DS-401` | 同一 `doc` を `anchor` 違いで複数entryとして持つことを許容し、重複としない。 |
| `DS-402`（適用前） | `anchor` と `note` はVO subject hashの入力に含まれない（VO subject hashは `derives_from` の参照先document ID集合を束縛する）（§1.3）。 |
| `DS-404` | 参照先document集合そのものの変更はVOの承認・判断記録を従来どおり失効させる。 |
| `DS-303` | VOの `derives_from`（document参照）が存在しないdocumentを参照する場合は `MISMATCH` とする。 |
| `DS-1643` | エッジ要素は `{ "from": "<上流ノード id>", "relation": "derives_from", "anchor": "<任意>", "note": "<任意>", "to": "VO-PARSER-UTF8-003" }` の形とする。 |
| `DS-1659` | 節ノードの子の並び順は当該節の規範内容に含まれ、子の集合を変えない並べ替えでも当該節の subject hash が変化して、当該ノードを上流依存 closure に含む判断記録・承認は失効する。 |
| `DS-1487` | VO を対象とする承認の上流依存closureは、対象 VO の再帰的 parent VO、対象 VO と parent VO が `derives_from` で参照するノード、および各ノードの実効的な上流（自分の辺 ∪ 先祖の辺）の再帰的閉包からなる。 |

`DS-1022` / `DS-1026`（据え置き、CLI 面）:

> `DS-1022`: 旧モデルの `--req`（REQ 参照）・`--spec` / `--section`（SPEC + 節参照）は廃し、上流参照は `--derives-from DOC-*`（任意の `--note`）へ一本化する。
> `DS-1026`: `--doc DOC-X` は当該 document を根とする下流 VO の絞り込みである。

### 1.4 詳細設計（design）

| id | 逐語（適用前） |
|---|---|
| `DES-568` | ノードは、文ノードが `id` / `statement` / `derives_from` / `source` を、節ノードが `id` / `title` / `source` を、根ノードが `id` / `statement` / `source` を必須 field として持つ。 |
| `DES-570` | 各 `derives_from` リンクは説明文・導出理由を保持せず、上流ノードの id だけを持つ（§3.2）。 |
| `DES-583` | ノードの `derives_from` は entry field を持たず、`anchor` はノードの `derives_from` にも Test metadata にも存在しない（§4.1）。VO レコードの `derives_from` entry の `anchor` は §3.2 に従う。 |
| `DES-584` | projectionが出力する `derives_from` エッジのうち、ノード → ノードのエッジは `anchor` を同伴しない。ノード → VO のエッジは当該 entry の `anchor`（§3.2）を同伴させる。 |
| `DES-585` | 上流文書のファイルは、当該文書を識別する field を持たない。 |
| `DES-587` | 節ノードの subject hash は、子を `items`（文ノードの列）と `sections`（下位節ノードの列）の 2 つの名前付き順序列として束縛し、各列を当該配列の宣言順で encode する。空の列も空 list として明示し、省略しない。 |
| `DES-380` | 検証グラフのノード間エッジは `derives_from` であり、仕様ノード由来である。 |
| `DES-381` | 検証グラフのエッジ `VO → DOC` は `derives_from` であり、VOレコード由来、1:N（1件以上）である。 |
| `DES-095` | VO subject hashは `derives_from`（参照先document ID集合）と `parent` を束縛する。 |
| `DES-117` | statement: VO レコードの `parent` fieldはVO IDまたはnull（階層化）である。 |

`DES-117` の description（YAML 例）の適用前の全文:

```yaml
id: VO-PARSER-UTF8-003
parent: VO-PARSER-UTF8          # VO ID または null（階層化）
derives_from:                   # 1件以上の document への直結（基本仕様 §3.2）
  - doc: DOC-BASIC-001
    anchor: "§8.2条項2"         # 任意の上流該当箇所（節番号等・空可・非 MISMATCH）
    note: ""                    # 任意（空可・非 MISMATCH）
claim: 不正な continuation byte を含む入力を与えた場合、ParseError::InvalidUtf8 を返す
dimensions: []                  # 検証軸（任意。§3.2.1）
coverage_policy: null           # independent-axes | full-product | explicit | null
combinations: []                # coverage_policy: explicit のとき実体化する組合せ（§3.2.1）
representative_cases: []        # 代表入力値（任意）
created: 2026-08-08
updated: 2026-08-08
```

---

## 2. 判定

### 2.1 参照先は上流ノード id である。**明文で決着する（導出ではない）。**

`SPEC-013`（spec）が「`derives_from` は上流ノードへの参照である」と無条件に定めている。VO の上流参照は `derives_from` 一本である（`SPEC-035` / `BD-329`）から、この文は VO の entry にも掛かる。

さらに `SPEC-348`（spec）が、**まさに VO への対応ペアを作るエッジについて**「到達先の上流ノード id（節または文）を伴い」と書いている。値域（上流ノード id）と粒度（節ノードも可）の両方が spec 層にある。

したがって **Owner 裁定は要らない。** 下位層も一致するが、これは根拠ではなく確認である: `BD-313`「ノード間・VO → ノードの関係」、`DS-1638`「参照先ノード内の該当箇所」、`DS-1643`（エッジ要素の `from` は `<上流ノード id>`）。

### 2.2 `document` という語は正本内で総称ノード型の名であり、文書ファイルではない

`REQ-028` / `SPEC-015` / `SPEC-035` / `DS-391` / `DS-396` などが使う `document` は、`ROOT-037` F11 が定義した**単一の総称ノード型**の名である。`BD-318` が `BD-019`「document の ID は `DOC-` とし」を「ノードの ID は層ごとの接頭辞…」へ改めた事実が、この語とノードが同じ対象であることを正本内で示している。

**ただし `ROOT-048` 以降、「文書」は保存の単位（1 文書 = 1 JSON ファイル）でもある。** 同じ語が型の名とファイルの単位の両方を指しているため、`doc: <値>` を「文書ファイルの名前」と読む余地が生まれていた。`BD-330` / `DES-585` により文書ファイルには識別子 field が無く、識別はファイル名なので、この誤読は実際に成立しうる。本適用はその読みを塞ぐ。

### 2.3 節ノードを指してよい。明文（`SPEC-348`「節または文」）であり、嘘テストでも緩まない

節ノードを指した VO の承認・判断記録は、`DS-1487`（VO の上流依存 closure に参照先ノードが入る）と `DS-1659` / `DES-587`（節の subject hash は子を順序つきで束縛する）により、**当該節の子の追加・削除・並べ替えでも失効する**。文ノードを指す場合より失効の範囲は広い。fail-closed の向きであり、嘘が通る余地は増えない。

対して**文書ファイルを指す読みだけが緩む**。ファイル単位の識別に束縛すると、節や文の差し替えが承認の外に落ち、鎖の切れ目が隠れる。`DS-303` / `DS-397` の dangling 検査もファイルの存在しか見なくなり、実在しない条項への参照が検査を通る。**この非対称が判定を決める。**

### 2.4 field 名 `doc` は維持する

`doc` は `DS-401` / `DS-1641` / `DS-1218` / `DS-1195` / `DS-1199` / `DS-1200` が同じ綴りで参照している wire 形であり、名前を変えることは規範変更にあたる。本適用では変えない。**名前が文書ファイルを指すかのように読める点は、`DS-1660` の description に明記した。**

---

## 3. 適用

コミット 1 本。対象は `docs/canonical/specification.json` と `docs/canonical/relations/retired-ids.json`、および本書。

| 処置 | id | 変更後の statement | 理由 |
|---|---|---|---|
| **新規** | `DS-1660`（detailed_spec、`DS-S072` の `DS-391` の直後） | VO レコードの `derives_from` entry の `doc` field の値は上流ノード id（節ノードまたは文ノード）であり、上流文書のファイル名ではない。 | `SPEC-013` と `SPEC-348` の転記。`derives_from: ["SPEC-013","SPEC-348"]` |
| **新 id** | `DES-570` → `DES-588` | ノードの各 `derives_from` リンクは説明文・導出理由を保持せず、上流ノードの id だけを持つ（§3.2）。VO レコードの `derives_from` entry の `note` は §3.2 に従う。 | 無条件の否定文が上位層の `DS-1638` と `BD-171` に反していた。`DS-1595` のスコープ語を転記。`anchor` 側の同型処置 `DES-583` と対 |
| **新 id** | `DES-381` → `DES-589` | 検証グラフのエッジ `VO → ノード` は `derives_from` であり、VOレコード由来、1:N（1件以上）である。 | `DOC` は `BD-318`（H-1）で死んだ値域。上位層の `BD-313` と同層の `DES-380` に字面を揃えた |
| **新 id** | `DS-402` → `DS-1661` | `anchor` と `note` はVO subject hashの入力に含まれない（VO subject hashは `derives_from` の参照先ノード id 集合を束縛する）（§1.3）。 | 「document の ID」は `DES-585` / `BD-318` のもとで指す先を失った値域。hash の束縛対象を確定 |
| **新 id** | `DES-095` → `DES-590` | VO subject hashは `derives_from`（参照先ノード id 集合）と `parent` を束縛する。 | `DS-1661` と同旨の design 層条文。片方だけ直すと層をまたいで矛盾する |
| **REWORD（id 維持）** | `DES-117` | statement は 1 文字も変えていない | 変更は description の YAML 例のみ。`DS-1599` により description のみの変更は subject hash を動かさない |

`derives_from` / `cites` / `source` は 5 件とも前身の値をそのまま保持した。辺を足しても引いてもいない。

### 3.1 `DES-117` の YAML 例（第 1 コミット `ac4974f` 時点）

> **上書きの訂正（第 2 コミット、§6）**: 本節が採ったプレースホルダ `<上流ノード id>` は依頼側の上書き指示により**採用されなかった**。
> 現行の値は `SPEC-999`（値域適合の例示 id）である。**理由と適用は §6 を読むこと。**
> 本節は第 1 コミット時点の記録として書き換えずに残す。

```yaml
derives_from:                   # 1件以上の上流ノードへの直結（基本仕様 §3.2）
  - doc: <上流ノード id>        # 節ノードまたは文ノードの id（§3.2）
    anchor: "§8.2条項2"         # 任意の上流該当箇所（節番号等・空可・非 MISMATCH）
    note: ""                    # 任意（空可・非 MISMATCH）
```

`anchor` / `note` の行と、それ以外の全行は逐語のまま。

**依頼との逸脱（1 件）。** 依頼は「実在する上流ノード id が望ましい」だったが、**プレースホルダ `<上流ノード id>` を採った。** 理由は 2 つ。

1. **`DS-1643` が同じ架空 VO の trace エッジを `{ "from": "<上流ノード id>", … "to": "VO-PARSER-UTF8-003" }` と書いている。** 同一の架空 VO についての例が正本内に 2 つあり、片方だけ実在 id にすると 2 例が食い違う。
2. `VO-PARSER-UTF8-003` は UTF-8 パーサを対象とする架空の VO であり、vtest 自身の条項から導出されていない。実在する `SPEC-*` / `REQ-*` を書くと、正本が言っていない導出関係を例として主張することになる（`reports/canonical-holes-2026-09-07.md` §7.4 が「具体的なノード id は選ばない」と判定した理由と同じ）。

代わりに、値域は同じ行のコメント（「節ノードまたは文ノードの id」）で示した。**上書きの判断は依頼側に残す。**

---

## 4. 触っていないもの（開示）

### 4.1 据え置き（Issue #14 の先送り。本決着と字面で矛盾する）

`reports/canonical-holes-2026-09-07.md` §10.6 の据え置き集合のうち、本決着と字面で矛盾するのは次の 5 件。**いずれも触っていない。**

| id | 層 | 逐語 | 矛盾の内容 |
|---|---|---|---|
| `DS-1022` | detailed_spec | 上流参照は `--derives-from DOC-*`（任意の `--note`）へ一本化する。 | `DOC-*` は死んだ値域。`DS-1660` の下では `--derives-from <上流ノード id>` |
| `DS-1026` | detailed_spec | `--doc DOC-X` は当該 document を根とする下流 VO の絞り込みである。 | 同上。フラグ名 `--doc` と値域の両方 |
| `DS-1145` | detailed_spec | `report --from DOC-REQ-001 --direction down --format json` は… | `DOC-REQ-001` は死んだ値域 |
| `DS-1524` | detailed_spec | `report --from DOC-X --direction down --format json` は… | 同上 |
| `SPEC-378` | spec | （description に `DOC-` を含む）`doc` は上流文書を総称 `document` レコードとして管理する唯一のコマンドである。 | description の例が死んだ値域 |

据え置き集合に**入っていない**が同種の残存が 1 件ある。**`BD-241`（basic_design、statement に `DOC-REQ-001`）。** `report --from DOC-REQ-001 …` を含む CLI 面の条文であり、内容は `DS-1145` と同一だが §10.6 の列挙に無い。本適用の軸（VO の entry の参照先）の外なので触っていないが、**据え置きが解けるときに `DS-1145` と一緒に処置されないと取り残される。**

### 4.2 `document` を総称ノード型の名として使い続けている条文（正しいまま残す）

`REQ-028` / `REQ-054` / `SPEC-015` / `SPEC-035` / `DS-391` / `DS-396` / `DS-397` / `DS-404` / `DS-303` / `DS-388` / `DS-546` / `DS-801` / `DS-1021` / `DS-1284` / `DES-238` ほか。§2.2 のとおり `document` = ノードなので偽ではない。**書き換えなかったのは、語そのものの掃引が本決着より広く、require / spec 層に及ぶためである。**

**これは本書が閉じない穴として残る**: `ROOT-048` が「文書」をファイルの単位にした結果、`document` が型の名とファイルの単位の 2 つを指している。**別の穴として立てるべきで、本適用では埋めていない。**

同節の `DS-404`「参照先document集合そのものの変更は…失効させる」を `DS-1661` と一緒に直さなかったのは、`DS-404` が「ID」の語を含まず、hash の束縛対象を定義する条文でもないため（定義は `DS-1661` / `DES-590` が持つ）。**同じ節に `ノード id 集合` と `document集合` が並ぶ状態は残る。**

### 4.3 その他

1. **`DES-128` / `DES-134` の description に死んだ `DOC-` が残る。** 判断記録・承認記録の `subject` field についての条文であり、VO の `derives_from` ではない。本軸の外。
2. **節タイトル「3.1 document レコード（`.verify/doc/DOC-*.yaml`）」が 4 層に残る**（`SPEC-S071` / `DS-S071` / `BD-S042` / `DES-S036`）。`reports/canonical-holes-2026-09-07.md` §10.7-3 の既知の開示。触っていない。
3. **`DS-1660` を `DS-S072` に挿入したことで、当該節の子の並びが変わる。** `DS-1659` / `DES-587` により `DS-S072` の subject hash は変化し、この節を上流依存 closure に含む承認・判断記録は失効する。ノードの追加である以上避けられない。
4. **`docs/canonical/export/*.md` は触っていない。** 生成物であり、JSON との乖離は既知。
5. **実装側への申し送り**: `DES-117` の YAML 例を逐語で再現している Rust テストがあれば、本変更で不一致になる。PR #21 の修正者の作業であり、本書の範囲ではない。

---

## 5. 機械検査

| 検査 | `fc35e58`（適用前） | 適用後 |
|---|---|---|
| ① schema 適合（`jsonschema` 4.26.0、Draft 2020-12、`specification.schema.json`） | OK（エラー 0） | **OK（エラー 0）** |
| ノード総数 | 3,848 | **3,849**（新規 +1、削除 0） |
| ② id 一意 | 重複 0 | **重複 0** |
| ③ 退役台帳の `old_id` が正本に生存 | 0 件（台帳 141 件） | **0 件（台帳 145 件）** |
| ④ `derives_from` の参照先が存在しない辺 | 0 | **0** |
| ④ `derives_from` の参照先が同層または下位層 | 0 | **0** |
| ⑤ `root` 以外で実効的上流（自ノード ∪ 祖先の辺）が空のノード | 270 | **270（不変）** |

⑤ の delta は 0。`DS-1660` は `SPEC-013` / `SPEC-348` への辺を持ち、`DES-588` / `DES-589` / `DS-1661` / `DES-590` はいずれも前身の辺と親節の辺をそのまま引き継ぐため、新たに孤児になったノードは無い。

新 id は正本のノード id と退役台帳の `old_id` / `new_id` の**両方**の最大値から採った（`DS-` 1659 → 1660・1661、`DES-` 587 → 588・589・590）。`reports/canonical-holes-2026-09-07.md` §10.10.2 が記録した「採番案の id が別ノードとして実在していた」欠陥の再発を避けるため。


---

## 6. 上書き適用（第 2 コミット、2026-09-07）— 例示 id `SPEC-999`

依頼側の上書き指示（逐語）:

> `DES-117` の YAML 例は正本が持つ唯一の VO 逐語例で、実装（PR #21）はこれを**そのまま**読めることを逐語テストにしています。`<上流ノード id>` は `$defs/id` の値域に合わない文字列なので、reader が VO の `doc` を値域検査すると（`DS-1660` で確定した以上そうなる）正本の例自体が拒否される。例は wire として妥当でなければならない。

**採った値: `SPEC-999`。** 選定の根拠は 3 つ。

1. `specification.schema.json` の `$defs/id` の `SPEC-[0-9]{3,}` に適合し、`BD-318` の層接頭辞規則にも適合する。
2. 正本のノード id にも `relations/retired-ids.json` の `old_id` / `new_id` にも存在しない（実測: `specification.json` 内の文字列出現 0 件）。3 桁の上限側から採ったので、将来採番される実在 id と衝突しない。
3. 接頭辞 `SPEC-` は、退役した値 `DOC-BASIC-001`（`BASIC` = 基本仕様）と、同じ例に逐語で残る `anchor: "§8.2条項2"` が指す層に一致する。`BD-318` の下で基本仕様の層のノードは `SPEC-` を持つ。

### 6.1 変更 2 件（どちらも id 維持）

| id | 変更 | 種別 |
|---|---|---|
| `DES-117` | description のみ。`doc:` の値を `SPEC-999` にし、例示 id である旨の 1 行を末尾に足した | REWORD（`DS-1599` により description のみの変更は subject hash を動かさない） |
| `DS-1643` | statement 中の `"from"` の値を同じ `SPEC-999` へ。description に例示 id である旨 | REWORD（**依頼側の指示による**。§6.2 の開示を参照） |

適用後の逐語:

```yaml
derives_from:                   # 1件以上の上流ノードへの直結（基本仕様 §3.2）
  - doc: SPEC-999               # 節ノードまたは文ノードの id（§3.2）
    anchor: "§8.2条項2"         # 任意の上流該当箇所（節番号等・空可・非 MISMATCH）
    note: ""                    # 任意（空可・非 MISMATCH）
```

> `DS-1643`: エッジ要素は `{ "from": "SPEC-999", "relation": "derives_from", "anchor": "<任意>", "note": "<任意>", "to": "VO-PARSER-UTF8-003" }` の形とする。

`anchor` / `note` の `<任意>` は値域を持たない自由文字列の placeholder であり、`$defs/id` の検査に掛からないため、そのまま残した。

### 6.2 開示 — 運用規約からの逸脱 1 件

**`DS-1643` は statement を変更しながら id を維持し、退役台帳に追記していない。** `reports/canonical-holes-2026-09-07.md` §10.3 が確立した運用（statement を変えたら新 id + 台帳、description だけなら REWORD）から外れる。依頼側が「意味不変なので REWORD（id 維持、退役台帳追記なし）」と明示的に指示したため従った。

**帰結を明示する。** `DS-1601` / `DS-1610`（規範内容 = `id` と `statement`）により、**`DS-1643` の subject hash は変化し、このノードを上流依存 closure に含む承認・判断記録は失効する。** id が変わらないため、**その失効は退役台帳からは追えない。** 「意味不変」は規範の内容についての判断であって、subject hash の計算には効かない。

---

## 7. 軸の外の未処置（id と逐語。PR #38 の既知事項へ転記できる形）

本書の軸は「VO の `derives_from` entry の参照先」であり、以下 3 件はその外にある。**直していない。** `AGENTS.md` の規律に従い、out-of-scope の材料違反として列挙する。

### 7.1 承認・判断記録が要求する「document の識別子」が正本に無い

| id | 層 | 逐語（`statement`） |
|---|---|---|
| `DES-134` | design | 承認レコードの `subject` fieldは承認対象のエンティティID（VO IDまたはdocument ID）である。 |
| `DES-128` | design | 判断記録の `subject` fieldは判断対象のエンティティIDまたは解決済みcanonical Locatorである。 |

`DES-134` の `description`（該当部分の逐語）:

```yaml
subject: VO-PARSER-UTF8-003     # 承認対象のエンティティID（VO ID または document ID）
dependencies:                   # 承認時点の上流依存closure（完全一致を要求）
  - kind: vo
    id: VO-PARSER-UTF8
    hash: "sha256:..."
  - kind: document
    id: DOC-BASIC-001
    hash: "sha256:..."          # §1.3 document subject hash
```

`DES-128` の `description` にも同一形の `- kind: document` / `id: DOC-BASIC-001` がある。

**なぜ穴か。** `DES-585`「上流文書のファイルは、当該文書を識別する field を持たない」と `BD-330`（ファイル名が当該文書を指す名前）のもとで、**「document ID」は指す先を持たない。** `DS-1660` は VO の `derives_from` entry についてのみ値域を確定しており、承認・判断記録の `subject` と `dependencies` には及ばない。

**帰結。** `kind: document` の依存 closure entry の `id` に何を書くかが決まっていない。ノード id を書くなら `kind` 名が対象を誤って呼んでおり、ファイル名を書くなら `$defs/id` とは別の値域が要る。**承認の失効判定そのものが依存する箇所なので、決まらないまま実装すると承認が誤って生き残る経路になる。**

### 7.2 据え置き集合から漏れている同種の残存 4 件

`reports/canonical-holes-2026-09-07.md` §10.6 の据え置き一覧に**入っていない**が、本決着と字面で矛盾する。

| id | 層 | 逐語（`statement`） | 矛盾 |
|---|---|---|---|
| `BD-241` | basic_design | `report --from DOC-REQ-001 --direction down --format json` が返す対応ペア集合が要求該当箇所と対応概念のペアの構造化出力であり、この用途に新規コマンド・ツールを設けない。 | `DOC-REQ-001` は `BD-318` で死んだ値域。内容は据え置きの `DS-1145` と同一 |
| `DS-1218` | detailed_spec | `doc_upsert` / `vo_upsert` の `derives_from[]` 各要素は `doc`（必須）、`anchor`（任意）、`note`（任意）からなる。 | `doc_upsert`（ノード側）にも `anchor` / `note` を持たせており、`DS-1594` / `DS-1595` に反する |
| `DS-1219` | detailed_spec | `anchor` は参照先 document 内の該当箇所を指す不透明な文字列であり、省略・空文字列を許容し `chain_integrity` 違反にしない。 | 同上。`DS-1638` は VO の entry についてのみ `anchor` を認める |
| `DS-1220` | detailed_spec | `anchor` は CLI の `--anchor` と同じ値域・同じ扱いとし、文書内位置への解決・実在確認を行わない。 | 同上 |

据え置きが解けるとき、`DS-1145` / `DS-1022` / `DS-1026` / `DS-1524` / `SPEC-378`（§4.1）と**この 4 件を同じ単位で処置しないと取り残される。**

### 7.3 `DES-588` の「§3.2 に従う」が指す節

> `DES-588`（design）: ノードの各 `derives_from` リンクは説明文・導出理由を保持せず、上流ノードの id だけを持つ（§3.2）。VO レコードの `derives_from` entry の `note` は §3.2 に従う。

`DES-588` の `source.doc` は `docs/AI並列開発向けテスト検証システム 基本仕様 v0.1.md`、`source.heading` は「1. 用語定義」である。文書名を伴わない節参照は同一文書内参照として扱う規則（`CONVERSION.md` §3）に従うと、この「§3.2」は**基本仕様 §3.2** に解決する。

一方、同型の条文 `DES-583`（「VO レコードの `derives_from` entry の `anchor` は §3.2 に従う」）の `source.doc` は `詳細設計 v0.1.md` であり、その「§3.2」は VO レコードの節（`DS-1638` の置き場所）を指す。

**同じ字面「§3.2」が 2 つの条文で別の節を指している。** 層の権威により実際に支配するのは `DS-1638` なので嘘は通らないが、`DES-588` の参照先が `note` について何も言わない節である可能性がある。**reword 候補: 「§3.2 に従う」→「`DS-1638` に従う」。** 本書では直していない（依頼が 1 コミット・当該 2 ノード限定のため）。

---

## 8. 機械検査（第 2 コミット適用後）

| 検査 | `ac4974f` | 第 2 コミット適用後 |
|---|---|---|
| ① schema 適合（`jsonschema` 4.26.0、Draft 2020-12） | OK（エラー 0） | **OK（エラー 0）** |
| ノード総数 | 3,849 | **3,849（不変）** |
| ② id 一意 | 重複 0 | **重複 0** |
| ③ 退役台帳の `old_id` が正本に生存 | 0 件（台帳 145 件） | **0 件（台帳 145 件、追記なし）** |
| ④ `derives_from` の参照先が存在しない辺 | 0 | **0** |
| ④ `derives_from` の参照先が同層または下位層 | 0 | **0** |
| ⑤ `root` 以外で実効的上流が空のノード | 270 | **270（不変）** |

delta はすべて 0。ノードの追加・削除・辺の増減は無く、変更は `DES-117` の description と `DS-1643` の statement / description のみ。
