# document モデルの再整合 — Issue #14 に対する正規仕様の突合（提案）

作成 2026-09-06 / 対象ブランチ `spec/upstream-traceability-audit` / 対象 `docs/canonical/specification.json`

**本書はすべて提案である。決定ではない。** `specification.json`・`docs/`・実装のいずれも変更していない。

---

## 0. 凡例と、固定した審査の枠

### 0.1 審査対象・目的・軸・除外（審査中に変更しない）

- **対象**: `specification.json` のうち、vtest の `document` モデルを定義または前提とする文。
  事前フィルタ 350 文（広めの正規表現、偽陽性を含む）に、取りこぼし掃引で見つけた 25 件を足した。
- **目的**: 上位層から順に、Issue #14 の決定がどの文を**偽にする**か、どの文がそのまま真か、語だけ直せばよいのはどれかを決める。
  最初に偽になる文が現れる**最上位の層**を特定する。書き換えはそこから始めて下流へ伝播させるべきもので、下位層だけを繕ってはならない。
- **軸 A（#14 との整合）**: 各候補に KEEP / REWORD / FALSIFIED / UNRESOLVED / NOT-DOC-MODEL のいずれかを付け、理由を 1 行で書き、#14 の本文を引く。
- **軸 B（置換案の上流根拠）**: 置換案は既決材料（#14 の Owner コメント、`specification.schema.json` のノード形、`LAYERING.md` §1.1、`CONVERSION.md`）の**転記**に限る。
  転記できない置換は UNRESOLVED とし、何が足りないかを書く。
- **除外**: 保存形式（単一 `specification.json` か 1 ノード 1 ファイルか）は決めない。
  `reports/upstream-traceability-audit-2026-09-04.md` §5 の 13 件の指摘には触れない。

### 0.2 判定値

| 判定 | 意味 |
|---|---|
| KEEP | #14 のもとでもそのまま真。`document` を「識別子と内容ハッシュを持つ参照可能なエンティティ」としてしか使っていない文がここに入る。 |
| REWORD | 規範の内容は生きるが、語（端点・列挙・粒度）を直す必要がある。置換案は既決材料の転記。 |
| FALSIFIED（F11 改定） | Issue #11 の凍結事項 F11（ROOT-037）の逐語に依存する文。**層ごとの型付け**が入ったことで偽になる。前提 P（下記 0.3）を前提とするが、Owner の直接の支持材料がある（PM が製品側 F11 との衝突を示し、Owner が台帳を解除した）。 |
| FALSIFIED（前提 P 依存） | **ノードの形**（`path` / `content_hash` / `anchor` entry が無い）が変わったことで偽になる文。前提 P に依存し、その根拠は `CONVERSION.md` の PM 記述だけである。 |
| UNRESOLVED | 置換に必要な決定が既決材料に無い。何が足りないかを書く。 |
| NOT-DOC-MODEL | 事前フィルタの偽陽性。Source Target / SRC ID / Evidence / VO レコード / adapter など、document モデルではない。 |

### 0.3 前提 P — 本書で最も重要な開示

**前提 P**: 「`specification.json` は vtest 自身の `.verify/doc/` の文書モデルそのものである。」

**この一文は Issue #14 に Owner の言葉として存在しない。** `gh issue view 14 --json title,body,comments` で本文とコメント 10 件すべてを読んで確認した。
出典は `docs/canonical/CONVERSION.md` 冒頭の PM 記述「Issue #14 の決定（JSON を正本、md は一方向エクスポート、`.verify/doc/` の文書モデルそのもの）」だけである。

むしろ #14 の**本文**は逆向きの限定を置いている（Owner はこの本文に対して 2026-09-05「導入するでいいんじゃないの？何か問題あった？」と述べた）:

> 現時点では implementation / verification / evidence / approval 等は対象に含めず、要求から設計までを対象とする。

したがって #14 には 2 通りの読みが残っている。

- **読み A（前提 P を置く）**: #14 は vtest の製品側 `document` エンティティを置き換える。本書の FALSIFIED（前提 P 依存）がすべて生きる。
- **読み B（前提 P を置かない）**: #14 は本リポジトリ自身の上流文書を data 化する決定であり、製品側 `.verify/doc/` の規範は動かない。
  この読みを最後まで通すと、`specification.json` は 1 ファイルであり、vtest はそれを総称 `document` 1 件として `path` + ファイルの `content_hash` で登録し、VO は `anchor: "REQ-025"` でその中を指す。
  **この場合 F11 は無傷で、`REQ-025` も `SPEC-421` も `DES-002` も真のままであり、書き換えは 1 件も要らない。最初に偽になる層も存在しない。**
  つまり本書の判定は FALSIFIED（F11 改定）17 件も FALSIFIED（前提 P 依存）57 件も、まとめて前提 P にぶら下がっている。

**前提 P を支持する Owner の材料は 2 つある。** いずれも前提 P そのものの逐語ではない。

1. **2026-09-04（ROOT-040 提案）**: PM が示した衝突相手は F11、すなわち**製品側**の文書モデルである。Owner はそれを見たうえで「凍結台帳はどうでもいいです。そんなん今解答されました。」と答えた。少なくとも F11 の範囲では、裁定は製品側に届いている。
2. **2026-09-06（ROOT-046 提案）**: 「vtest がやるべきことを python で先に作る分には構わないんですが、これを正としないために消してください。」
   消された足場がやっていたのは、文への id 付番、節の木の構築、`cites` → 上流 statement の辺の解決である。**それが「vtest がやるべきこと」なら、vtest のモデルは文ノード単位である。**
   `CONVERSION.md` も同じ理解で「同等の機能…は vtest 側で実装する」と書いている。読み B はこの発言と整合しない。

**したがって上流へ返す第一の質問は二択ではなく確認である**（§6 Q1）。前提 P に Owner の逐語は無いが、2 つの材料が支持し、読み B は 09-06 の発言と衝突する。

**読み A / 読み B の選択は Owner の裁定事項であり、本書は決めない。** 依頼は読み A を前提として与えられたので、作業は読み A で進めたうえで、
前提 P に依存することを判定値として明示した。

判定値を 2 つに割ったのは、**証拠の強さと、変わったものの種類が違う**からである。
FALSIFIED（F11 改定）は「上流文書が層ごとに型付けされた」ことによる偽で、Owner の直接の裁定に近い。
FALSIFIED（前提 P 依存）は「ノードの形から `path` と `content_hash` が消えた」ことによる偽で、根拠は schema の形と PM 記述である。

### 0.4 `derives_from` と `derived_from` の綴り

既決材料（#14 本文・schema・`LAYERING.md`）はすべて `derived_from` と綴る。現行仕様の文はすべて `derives_from` と綴る。
**この綴りの差だけを理由に約 150 文を REWORD にはしない。** 帰結として一度だけ記録し（§4.4）、各行の判定には持ち込まない。
VO レコードの field 名にも及ぶため、#14 の対象外（VO）に波及する点も §4.4 に書いた。

---

## 1. root 層へ足すべきノード（提案）

現在 `root` は 38 ノードすべてが 2026-08 の Issue #11 由来で、#14 の 2026-09-04/05/06 の裁定が 1 つも入っていない。
`request` 層 5 件（R-1〜R-5）も全文を読んだが、#14 に由来する文は無い。

既存 ROOT-027〜038 の `source` は `{"doc": "github:YmSaki/SpecTracer/issues/11", "heading": "F1", "lines": [1, 1]}` の形なので、これに揃える。
id は ROOT-039 から。`relations/retired-ids.json` に ROOT- の退役は無いので衝突しない。

```json
[
  {
    "id": "ROOT-039",
    "statement": "要求や要件定義、基本仕様や設計なんてものを全部 JSON データ化してグラフで表現しやすくして、JSON を正本にして特定のフォーマットで md にエクスポートするってのを導入しちゃうのがよさそうだ。今はやらないとか言ってる暇じゃねぇ。",
    "description": "Owner 裁定 2026-09-04（Issue #14 コメント 1）。JSON を正本、md は一方向エクスポート。",
    "source": {
      "doc": "github:YmSaki/SpecTracer/issues/14",
      "heading": "C1",
      "lines": [
        1,
        1
      ]
    }
  },
  {
    "id": "ROOT-040",
    "statement": "凍結台帳はどうでもいいです。そんなん今解答されました。",
    "description": "Owner 裁定 2026-09-04（Issue #14 コメント 1）。凍結事項 F11（ROOT-037）との衝突を示されたうえでの回答。「（上流差戻フェーズなので）」は書記の補足。",
    "source": {
      "doc": "github:YmSaki/SpecTracer/issues/14",
      "heading": "C1",
      "lines": [
        1,
        1
      ]
    }
  },
  {
    "id": "ROOT-041",
    "statement": "あくまで derived_from は参照なので。",
    "description": "Owner 裁定 2026-09-05（Issue #14 コメント 2）。`derived_from` は導出の証明ではなく参照。ペアごとの承認は行わない。",
    "source": {
      "doc": "github:YmSaki/SpecTracer/issues/14",
      "heading": "C2",
      "lines": [
        1,
        1
      ]
    }
  },
  {
    "id": "ROOT-042",
    "statement": "辺は層をまたぐ参照だけ。要求 → 要件定義 → 基本仕様 → 詳細設計の間の参照を辺にし、同じ層の中の節参照（本冊内の「§7.3」、別紙A → 本冊）は辺にしない。",
    "description": "Owner 裁定 2026-09-05（Issue #14 コメント 4）。直前のコメント 3「関係をグラフに置き換えると言っているだけなので。」（＝参照はすべて辺）を撤回してこれで確定。",
    "source": {
      "doc": "github:YmSaki/SpecTracer/issues/14",
      "heading": "C4",
      "lines": [
        1,
        1
      ]
    }
  },
  {
    "id": "ROOT-043",
    "statement": "導入するでいいんじゃないの？何か問題あった？",
    "description": "Owner 裁定 2026-09-05（Issue #14 コメント 5）。「この JSON を仕様書の正本として受け入れるか」への回答。",
    "source": {
      "doc": "github:YmSaki/SpecTracer/issues/14",
      "heading": "C5",
      "lines": [
        1,
        1
      ]
    }
  },
  {
    "id": "ROOT-044",
    "statement": "基本仕様 = 何を実現するかを確定する\n基本設計 = それをどんな構造で実現するかを決める\n詳細設計 = 各部品をどう実装するかまで落とす\n詳細設計は自然言語で記述したコードになるのは避けたい。",
    "description": "Owner 定義 2026-09-05（Issue #14 コメント 6）。同コメントの「ついでにこれの分割もしてしまおう。」で層の分割を指示。`LAYERING.md` 冒頭に逐語保存。",
    "source": {
      "doc": "github:YmSaki/SpecTracer/issues/14",
      "heading": "C6",
      "lines": [
        1,
        1
      ]
    }
  },
  {
    "id": "ROOT-045",
    "statement": "採用してもいいし、これを既定の規定にしてもいい。\n基本仕様は複数の詳細仕様を内包できる。\n基本設計は[仕様]から導出され、複数の詳細設計に分割される。\n詳細設計は基本仕様から導出される。",
    "description": "Owner 裁定 2026-09-05（Issue #14 コメント 8）。6 段（要求 / 要件 / 基本仕様 / 詳細仕様 / 基本設計 / 詳細設計）を既定の規定として採用。",
    "source": {
      "doc": "github:YmSaki/SpecTracer/issues/14",
      "heading": "C8",
      "lines": [
        1,
        1
      ]
    }
  },
  {
    "id": "ROOT-046",
    "statement": "残す必要あるんですか？ vtest がやるべきことを python で先に作る分には構わないんですが、これを正としないために消してください。",
    "description": "Owner 指示 2026-09-06（Issue #14 コメント 10）。変換の Python 足場を削除。同等機能は vtest 側で実装する。",
    "source": {
      "doc": "github:YmSaki/SpecTracer/issues/14",
      "heading": "C10",
      "lines": [
        1,
        1
      ]
    }
  }
]
```

`heading` は既存 ROOT-027〜038 が F 番号（`F1`）を使うのに合わせ、#14 にはコメント番号（`C1`〜`C10`）を当てた。**この命名は提案であり、既決材料に根拠は無い。**

### 1.1 ROOT-037（F11）の扱い — UNRESOLVED

ROOT-037 は Owner 自身の裁定なので**書き換えない**。#14 のコメント 1 で改定されたという事実を、どう観測可能にするかが決まっていない。
`specification.schema.json` の `rootItem` に supersede を表す field は無く、`root` 層は `derived_from` を持たない唯一の層と定義されている。
`specification.json` を実際に調べたところ、ROOT-037 を `derived_from` に持つのは節ノード 2 件（`REQ-S005`、`REQ-S045`）だけで、文ノードからの辺は 1 本も無い。
**したがって `REQ-025` などの文を書き換えても、ROOT-037 への辺は 2 本とも残る。改定を示す観測可能な変化は現在の schema では 1 つも作れない。**

**UNRESOLVED**: 改定された根ノードをどう記録するか（新 root ノードを足すだけか、supersede 関係を schema に足すか、節の辺を張り替えるか）が #14 にも schema にも無い。

---

## 2. 層ごとの突合表

上位層から並べる。FALSIFIED / REWORD / UNRESOLVED は全件、KEEP と NOT-DOC-MODEL は id の列挙だけ。

### 2.1 要件定義（`require`）

候補 13 件。FALSIFIED（F11 改定） 1 / FALSIFIED（前提 P 依存） 4 / REWORD 2 / KEEP 6

| id | 現在の文 | 判定 | 理由 | 置換案 / 不足事項 |
|---|---|---|---|---|
| `REQ-025` | 上流文書はすべて単一の総称ノード型 `document`（id + path + content_hash + 上流参照）で表現する。 | FALSIFIED（F11 改定） | F11（ROOT-037）の最小形の逐語。#14 2026-09-04 Owner「凍結台帳はどうでもいいです。そんなん今解答されました。」で F11 は改定済み。schema は 7 層のトップレベル配列を required にし、ノード field に `path` / `content_hash` を持たない。 | 上流文書は層ごとのトップレベル配列（`root` / `request` / `require` / `spec` / `detailed_spec` / `basic_design` / `design`）に分け、各層は「文書 > 節 > 小節 > 文」の木のノードとして表現する。（出典: specification.schema.json の `required`、LAYERING.md §1・§1.1） |
| `REQ-030` | 文書層の段数は総称的に設計し、リンクを追加してもスキーマが壊れないことを設計制約とする。 | FALSIFIED（前提 P 依存） | 候補一覧の外（追加検出）。schema は `required` に 7 層を列挙し `additionalProperties: false` を置くため、層を 1 つ足すと schema が変わる。「リンクを追加してもスキーマが壊れない」は成立しない。 | UNRESOLVED: 層を増やすときに schema をどう扱うか（`required` の 7 層固定を維持するか）が #14 にも schema にも無い。 |
| `REQ-053` | 文書層では、各 `document` の derives_from 参照先が存在し、content_hash が現物と一致することを要求する。 | FALSIFIED（前提 P 依存） | 「content_hash が現物と一致する」は、1 document = 1 ファイルを前提とする。JSON 正本ではノードに対応する「現物ファイル」が無い。derives_from 参照先の存在の要求は生きる。 | UNRESOLVED: ノードの内容ハッシュが何を束縛するか（statement のみか、description を含むか、節は子の Merkle か）が #14・schema・LAYERING.md のいずれにも無い。 |
| `REQ-098` | 文書鎖のリンク切れ / content_hash 不一致 / 孤児文書のいずれかが生じる場合、状態は `MISMATCH`（診断ラベルは STALE 等）となる。 | FALSIFIED（前提 P 依存） | 3 つの選言のうち「content_hash 不一致」が REQ-053 と同じ理由で成立しない。リンク切れと孤児は生きる。 | UNRESOLVED: REQ-053 と同じ（ノードの内容ハッシュの束縛対象が未決）。 |
| `REQ-234` | 取り込まれた上流成果物は §3.2 の `document` ノードとして登録され、content_hash と derives_from を持つ。 | FALSIFIED（前提 P 依存） | 「content_hash と derives_from を持つ」の content_hash が REQ-053 と同じ理由で成立しない。 | UNRESOLVED: REQ-053 と同じ。 |
| `REQ-026` | 文書間のリンクは `derives_from` の一種のみとする。 | REWORD | リンクが 1 種のみという内容は生きる（LAYERING.md §1「6段になっても辺の種類は増えない」）。ただし辺の端点が「文書」から「文・節ノード」へ変わる。 | ノード（文・節）間のリンクは `derived_from` の一種のみとする。（出典: LAYERING.md §1.1「辺（trace / realization）: `derived_from` 一種のまま。文にも節にも付く」） |
| `REQ-059` | `orphan_detection` の問いは、親を持たない `document` ノードが存在するか、である。 | REWORD | 孤児の判定基準が「自分の辺だけ」だと、節に覆われた文と解決できない引用（#14 2026-09-05「解決できない引用（21件）は空のまま残す」）がすべて孤児になる。 | `orphan_detection` の問いは、実効的な上流（自分の辺 ∪ 先祖の辺）を持たないノードが存在するか、である。（出典: LAYERING.md §1.1「文の実効的な上流 = 自分の辺 ∪ 先祖（節・文書）の辺。保存せず計算する」） |

**KEEP（6 件）**: `REQ-018`、`REQ-024`、`REQ-028`、`REQ-037`、`REQ-054`、`REQ-283`

### 2.2 基本仕様（`spec`）

候補 31 件。FALSIFIED（F11 改定） 4 / FALSIFIED（前提 P 依存） 1 / REWORD 7 / UNRESOLVED 4 / KEEP 15

| id | 現在の文 | 判定 | 理由 | 置換案 / 不足事項 |
|---|---|---|---|---|
| `SPEC-378` | `doc` は上流文書を総称 `document` レコードとして管理する唯一のコマンドである。 | FALSIFIED（F11 改定） | 「総称 `document` レコード」は F11 の改定で成立しない（SPEC-421 と同じ）。`doc` コマンドが存続するかは別問題で未決。 | UNRESOLVED: CLI 面（`vtest doc`）が新モデルで存続するか、存続するなら何を引数に取るかが #14 に無い。#14 本文は「MCP の CRUD API…については、このデータモデルの方向性が決まった後に別途検討する」と明示的に先送りしている。 |
| `SPEC-421` | document 種別を区別せず、要件定義・基本仕様・詳細設計・API Schema 等をすべて総称 document として同一に扱う。 | FALSIFIED（F11 改定） | 「document 種別を区別せず…総称 document として同一に扱う」は F11 の逐語。#14 は 7 層のトップレベル配列に分け、id 接頭辞も層ごとに分ける。 | 上流文書は層ごとのトップレベル配列に分け、層ごとに id 接頭辞（`ROOT-` / `R-` / `REQ-`・`P-` / `SPEC-` / `DS-` / `BD-` / `DES-`）を持つ。（出典: specification.schema.json `$defs/id` の pattern、LAYERING.md §1 の表） |
| `SPEC-441` | 方針は総称 document として登録した文書で表現し、専用のエンティティ型を設けない。 | FALSIFIED（F11 改定） | 「総称 document として登録した文書で表現し、専用のエンティティ型を設けない」は F11 の逐語。加えて「方針」は 7 層のどれにも当たらない。 | UNRESOLVED: SPEC-012 と同じ（層に当たらない文書の置き場が未決）。 |
| `SPEC-445` | 旧モデルの SPEC → REQ → VO → Test 構造は総称 document 化により DOC → VO → Test へ再導出する。 | FALSIFIED（F11 改定） | 「総称 document 化により DOC → VO → Test へ再導出する」は F11 の逐語に依存。#14 は総称化ではなく層ごとの分割。 | 旧モデルの SPEC → REQ → VO → Test 構造は、7 層の木 → VO → Test へ再導出する。（出典: specification.schema.json の `required`、LAYERING.md §1） |
| `SPEC-S071` | [SECTION] 3.1 document レコード（`.verify/doc/DOC-*.yaml`） | FALSIFIED（前提 P 依存） | 候補一覧の外（追加検出）。節の title が `.verify/doc/DOC-*.yaml` を名指しする。schema の id pattern に `DOC-` は無い。 | UNRESOLVED: 保存形式は本レビューの除外事項。REQ-272「本システムは、不必要に単一共有台帳へ書き込ませない。」および REQ-325「並列編集時の物理的保存方式は、下位仕様へ委譲する設計事項である。」を添えて上流へ返す。 |
| `SPEC-013` | derives_fromは上流documentから下流documentへの導出を表す。 | REWORD | #14 本文は `derived_from` を「この成果物の存在・内容を、指定された上流成果物から直接導出できる」と定義したが、2026-09-05 Owner「あくまで derived_from は参照なので。」で参照へ後退した。「導出を表す」は現在の裁定と合わない。 | `derived_from` は上流ノードへの参照である。（Owner 逐語は「あくまで derived_from は参照なので。」の部分だけ。「導出の証明ではない」は書記の読みであり逐語ではないので、置換文に入れるかは Owner 確認事項） |
| `SPEC-042` | 文書層の段（要件→仕様→詳細設計…）はderives_fromリンクとして表現する。 | REWORD | 段の列挙（要件→仕様→詳細設計）が 6 段の採用で変わった。リンクで表現するという内容は生きる。 | 層の段（要求 → 要件 → 基本仕様 → 詳細仕様 → 基本設計 → 詳細設計）は `derived_from` リンクとして表現する。（出典: #14 2026-09-05 Owner「採用してもいいし、これを既定の規定にしてもいい。」、LAYERING.md §1） |
| `SPEC-059` | orphan_detectionの問いは、親を持たない `document` ノードが存在するかである。 | REWORD | REQ-059 と同じ。 | REQ-059 の置換案と同じ表現へ揃える。 |
| `SPEC-291` | `orphan_detection` は文書層の孤児検出であり、親（上流document）を持たない `document` ノードが存在するかを問う。 | REWORD | REQ-059 と同じ。 | REQ-059 の置換案と同じ表現へ揃える。 |
| `SPEC-348` | projectionが出力する`derives_from`エッジに当該entryの`anchor`を常に同伴させることにより「どの上流条項が、どの概念（VO）へ対応するか」の対応ペアが構造化出力として取得でき、外部の発見者が未宣言の義務・網羅漏れを裁定する材料になる（基本仕様 §11.1）。 | REWORD | `anchor` の同伴という手段は schema に無い（`derived_from` は id の並びで entry object を持たない）が、目的（「どの上流条項がどの概念へ対応するか」の対応ペア）は辺の到達先が節・文ノードの id であることで構造的に満たされる。ROOT-030（F4）の要求は保たれる。 | projection が出力する `derived_from` エッジは到達先の上流ノード id（節または文）を伴い、「どの上流条項が、どの概念（VO）へ対応するか」の対応ペアとして読める。（出典: specification.schema.json `derived_from` は `#/$defs/id` の配列、LAYERING.md §1.1） |
| `SPEC-377` | `vtest doctor` は、同じTest IDの重複（E-SCAN-002）、covers先VOの欠落（E-SCAN-003）、文書鎖のリンク切れ（E-SCAN-012）、孤児 document（E-SCAN-016）、承認・判断・Evidenceのハッシュ束縛による失効（診断 `STALE`）など、version control の構文的整合性だけでは判定できない論理的不整合を検出する。 | REWORD | 列挙のうち「孤児 document（E-SCAN-016）」が REQ-059 と同じ理由で判定基準を変える。他の項目は生きる。 | 孤児の項を REQ-059 の置換案（実効上流を持たないノード）へ揃える。 |
| `SPEC-422` | `orphan_detection` は文書層のみを対象とし、親（上流 document）を持たない `document` ノードの有無を問う。 | REWORD | REQ-059 と同じ。 | REQ-059 の置換案と同じ表現へ揃える。 |
| `SPEC-012` | documentは要件定義書・基本仕様書・詳細設計書・API Schema・Protocol Specification・型/データ仕様・DB schema・その他の機械可読仕様を含む。 | UNRESOLVED | document の値域として API Schema / Protocol Specification / DB schema を挙げるが、#14 の 7 層（root/request/require/spec/detailed_spec/basic_design/design）にこれらの置き場が無い。 | UNRESOLVED: 層に当たらない上流成果物（API Schema、DB schema 等）を canonical model のどこに置くかが #14 に無い。 |
| `SPEC-190` | Test単位の結果をVO・Feature・document単位へ集約可能とする。 | UNRESOLVED | 「document 単位へ集約」の「単位」が、木のどの高さ（文書 / 節 / 小節 / 文）を指すか決まらない。 | UNRESOLVED: 木になったあとの集約粒度（節単位か文単位か）が #14・LAYERING.md に無い。LAYERING.md §1.1 は辿り方（実効上流）だけを決めており、集約は決めていない。 |
| `SPEC-336` | 仕様文書の更新は`vtest doc add --update`による再登録で反映し、依存する判断・承認が失効することを利用者へ提示する。 | UNRESOLVED | 候補一覧の外（追加検出）。`vtest doc add --update` による再登録が前提。 | UNRESOLVED: SPEC-378 と同じ（CLI 面が未決）。 |
| `SPEC-380` | 正典編集は `add --update` で行う。 | UNRESOLVED | `add --update` は「1 document = 1 ファイルを path で登録し直す」経路。JSON 正本ではノード編集になる。 | UNRESOLVED: SPEC-378 と同じ（CLI 面が未決）。 |

**KEEP（15 件）**: `SPEC-015`、`SPEC-034`、`SPEC-035`、`SPEC-148`、`SPEC-163`、`SPEC-173`、`SPEC-180`、`SPEC-210`、`SPEC-220`、`SPEC-289`、`SPEC-346`、`SPEC-352`、`SPEC-381`、`SPEC-382`、`SPEC-408`

### 2.3 詳細仕様（`detailed_spec`）

候補 185 件。FALSIFIED（F11 改定） 3 / FALSIFIED（前提 P 依存） 35 / REWORD 25 / UNRESOLVED 26 / KEEP 46 / NOT-DOC-MODEL 50

| id | 現在の文 | 判定 | 理由 | 置換案 / 不足事項 |
|---|---|---|---|---|
| `DS-044` | documentは種別専用スキーマを持たない。 | FALSIFIED（F11 改定） | 「documentは種別専用スキーマを持たない」は F11 の却下リストの逐語。#14 は層ごとのトップレベル配列を持つ。 | SPEC-421 の置換案と同じ。 |
| `DS-046` | documentは単一の総称ノードであり、要件定義・基本仕様・詳細設計・API Schema等を種別で区別する専用スキーマを持たない。 | FALSIFIED（F11 改定） | DS-044 と同じ（種別で区別する専用スキーマを持たない）。 | SPEC-421 の置換案と同じ。 |
| `DS-1053` | 方針は総称 document として登録した文書で表現するため、方針の承認・却下・取消は `--subject-type document` で記録する。 | FALSIFIED（F11 改定） | 「方針は総称 document として登録した文書で表現するため」。SPEC-441 と同じ。 | UNRESOLVED: SPEC-012 と同じ。 |
| `DS-S071` | [SECTION] 3.1 document レコード（`.verify/doc/DOC-*.yaml`） | FALSIFIED（前提 P 依存） | 候補一覧の外（追加検出）。節 title が `.verify/doc/DOC-*.yaml`。 | UNRESOLVED: DS-784 と同じ。 |
| `DS-081` | 文書鎖のリンク切れ、content_hash不一致、または孤児文書の場合、状態は `MISMATCH`、診断ラベルはSTALE等とする。 | FALSIFIED（前提 P 依存） | REQ-098 と同じ（content_hash 不一致の選言）。 | UNRESOLVED: REQ-053 と同じ。 |
| `DS-105` | 文書層では、各documentのcontent_hashが現物と一致する。 | FALSIFIED（前提 P 依存） | 「各documentのcontent_hashが現物と一致する」— ノードに対応する現物ファイルが無い。 | UNRESOLVED: REQ-053 と同じ。 |
| `DS-313` | content_hash照合は決定論的に解決する。 | FALSIFIED（前提 P 依存） | content_hash 照合そのものが対象を失う。 | UNRESOLVED: REQ-053 と同じ。 |
| `DS-346` | document recordの `content_hash` と実sourceが不一致ならsubject hashは現在有効な値として成立せず、`chain_integrity` の非 `PASS`（`MISMATCH`、診断 `STALE`）とする（§11.4）。 | FALSIFIED（前提 P 依存） | 「document recordの content_hash と実sourceが不一致なら」— 実 source が無い。 | UNRESOLVED: REQ-053 と同じ。 |
| `DS-378` | document レコードの `title` fieldは任意の表示名である。 | FALSIFIED（前提 P 依存） | 「document レコードの `title` field は任意の表示名」。schema では `title` は section の必須 field で、文ノード（derivedItem）は `title` を持たない。 | 節ノードは必須 field `title` を持ち、文ノードは `title` を持たない。（出典: specification.schema.json `$defs/section` の `required: [id, title, source]`、`$defs/derivedItem`） |
| `DS-379` | document レコードの `derives_from` fieldは上流documentへの導出リンクであり、0件も許容する（0件は根候補を意味する）。 | FALSIFIED（前提 P 依存） | 0 件許容は残るが「0件は根候補を意味する」が偽になる。木では 0 件は「節の辺に覆われている」か「解決できない引用 21 件のどれか」を意味し、根は `root` 層である（schema「root … derived_from を持たない唯一の層」）。schema では derivedItem の `derived_from` は必須（空配列可）、section の `derived_from` は任意。 | 文ノードの `derived_from` は必須 field で 0 件を許容し、節ノードの `derived_from` は任意 field である。（出典: specification.schema.json `$defs/derivedItem` の `required`、`$defs/section` の `properties`） |
| `DS-380` | document レコードの `derives_from` entryの `anchor` fieldは任意の上流該当箇所（節番号等）であり、空も許容し、`chain_integrity` の `MISMATCH` としない。 | FALSIFIED（前提 P 依存） | `derived_from` entry の `anchor` field。schema の `derived_from` は id の並びで entry object を持たない。到達先のノード id 自体が該当箇所を指す。 | SPEC-348 の置換案と同じ（entry field を廃し、到達先ノード id で表す）。 |
| `DS-381` | document レコードの `derives_from` entryの `note` fieldは任意の導出理由であり、空も許容し、`chain_integrity` の `MISMATCH` としない。 | FALSIFIED（前提 P 依存） | DS-380 と同じ（`note` field）。 | SPEC-348 の置換案と同じ。 |
| `DS-382` | 各 `derives_from` entryの `note`（導出理由・説明文）は任意であり、空でも `chain_integrity` 違反・`MISMATCH` としてはならない（§19）。 | FALSIFIED（前提 P 依存） | DS-380 と同じ（`note`）。 | SPEC-348 の置換案と同じ。 |
| `DS-383` | 各 `derives_from` entryの `anchor`（参照先document内の該当箇所を指す文字列。節番号・条項番号・見出し等）は任意であり、欠落・空文字列を `chain_integrity` 違反・`MISMATCH` としてはならない（§19）。 | FALSIFIED（前提 P 依存） | DS-380 と同じ（`anchor`）。 | SPEC-348 の置換案と同じ。 |
| `DS-387` | `anchor` だけの変更は `path` の実ファイルを変えないため `content_hash` を変化させないが、document subject hashを変化させるため、当該documentを上流依存closureに含む判断記録・承認は失効する（§3.5・§8.5）。 | FALSIFIED（前提 P 依存） | 「`anchor` だけの変更は `path` の実ファイルを変えない」— `anchor` field も `path` も無い。 | UNRESOLVED: REQ-053 と同じ（ノードの subject hash の入力が未決）。 |
| `DS-389` | `path` の実ファイルが `content_hash` と一致しなくなった場合は `chain_integrity` の `MISMATCH`（診断 `STALE`）とする（§11.4）。 | FALSIFIED（前提 P 依存） | 「`path` の実ファイルが `content_hash` と一致しなくなった場合」— `path` が無い。 | UNRESOLVED: REQ-053 と同じ。 |
| `DS-800` | 文書層は、`content_hash`が現物と一致すること（不一致は診断`STALE`。§11.4）を評価する。 | FALSIFIED（前提 P 依存） | 「`content_hash`が現物と一致すること（不一致は診断`STALE`）を評価する」。 | UNRESOLVED: REQ-053 と同じ。 |
| `DS-860` | スキャン時にdocumentレコードの`content_hash`と実ファイル（`path`）を比較し、不一致ならW-SCAN-104を出す。 | FALSIFIED（前提 P 依存） | 「documentレコードの`content_hash`と実ファイル（`path`）を比較」— `path` が無い。 | UNRESOLVED: REQ-053 と同じ。 |
| `DS-861` | 当該documentを`derives_from`で参照するVO / 上位documentの鎖は、content_hash不一致として`chain_integrity = MISMATCH`（診断`STALE`）となる（§11.1.1）。 | FALSIFIED（前提 P 依存） | content_hash 不一致を鎖へ伝播させる規則。前提となる比較が成立しない。 | UNRESOLVED: REQ-053 と同じ。 |
| `DS-862` | 当該document subjectをdependencyに含む判断記録（§8.5）・承認記録（§3.5）も無効となる。 | FALSIFIED（前提 P 依存） | DS-861 と同じ（判断・承認の無効化）。 | UNRESOLVED: REQ-053 と同じ。 |
| `DS-863` | 再登録でdocument subject hashが変化するため、以前のdependency entryを現在の承認・判断へ流用しない。 | FALSIFIED（前提 P 依存） | 「再登録で document subject hash が変化する」— 再登録（`path` からの取り込み）が無い。 | UNRESOLVED: REQ-053 と同じ。 |
| `DS-903` | `W-SCAN-104`はwarningであり、documentレコードのcontent_hashと実ファイルの不一致である（依存判断・依存Approvalは無効、鎖はchain_integrity STALE）。 | FALSIFIED（前提 P 依存） | W-SCAN-104（content_hash と実ファイルの不一致）の定義そのもの。 | UNRESOLVED: REQ-053 と同じ。 |
| `DS-1003` | `--derives-from` は上流 document への導出リンク（0件可＝根候補）である。 | FALSIFIED（前提 P 依存） | `--derives-from` は「上流 document への導出リンク」。到達先が文書ではなくノードになる。CLI の存続は未決。 | UNRESOLVED: SPEC-378 と同じ。 |
| `DS-1005` | `--anchor <text>` は直前の `--derives-from` に束縛し、参照先 document 内の該当箇所（節番号・条項番号・見出し等）を記録する。 | FALSIFIED（前提 P 依存） | `--anchor` を直前の `--derives-from` に束縛する CLI。entry field が無い。 | SPEC-348 の置換案。加えて UNRESOLVED: SPEC-378（CLI 面）。 |
| `DS-1009` | `doc show` は各 `derives_from` entry の `anchor` を表示する。 | FALSIFIED（前提 P 依存） | `doc show` が entry の `anchor` を表示する。 | SPEC-348 の置換案。加えて UNRESOLVED: SPEC-378。 |
| `DS-1012` | `--update` は既存 DOC レコードの sha256 を現ファイルで再計算して更新する。 | FALSIFIED（前提 P 依存） | 候補一覧の外（追加検出）。「既存 DOC レコードの sha256 を現ファイルで再計算」。現ファイルが無い。 | UNRESOLVED: REQ-053 および SPEC-378 と同じ。 |
| `DS-1013` | `--update` は document subject hash が変化するため、当該 document を依存 closure に含む判断記録・承認が失効する旨を出力する。 | FALSIFIED（前提 P 依存） | 「`--update` は document subject hash が変化するため…失効する旨を出力する」。 | UNRESOLVED: REQ-053 および SPEC-378 と同じ。 |
| `DS-1017` | `doc show` は DOC の path・content_hash・derives_from・根指定・鮮度（content_hash と実ファイルの一致）・実効承認状態を表示する。 | FALSIFIED（前提 P 依存） | `doc show` が「path・content_hash・…・鮮度（content_hash と実ファイルの一致）」を表示する。 | UNRESOLVED: REQ-053 および SPEC-378 と同じ。 |
| `DS-1019` | `path` の実ファイルが `content_hash` と一致しなくなれば `chain_integrity = MISMATCH`（診断 `STALE`）とする。 | FALSIFIED（前提 P 依存） | DS-389 と同じ（CLI 文脈）。 | UNRESOLVED: REQ-053 と同じ。 |
| `DS-1026` | `--doc DOC-X` は当該 document を根とする下流 VO の絞り込みである。 | FALSIFIED（前提 P 依存） | `--doc DOC-X`。`DOC-` 接頭辞が schema の id pattern に無い。 | UNRESOLVED: SPEC-378 と同じ。 |
| `DS-1195` | `doc_upsert` は document フィールド一式（`path`、`derives_from[]`（`doc` + 任意 `anchor` + 任意 `note`）、`root: bool`、`update: bool`）を入力とし、作成・更新結果（依存判断・承認の失効警告を含む）を出力する。 | FALSIFIED（前提 P 依存） | `doc_upsert` の入力 field（`path`、`derives_from[]` に `anchor`/`note`、`root`、`update`）がすべて新しいノード形と一致しない。 | UNRESOLVED: DS-1194 と同じ。 |
| `DS-1288` | 各`document`の`content_hash`が実ファイル（`path`）と一致することを要求する（不一致はW-SCAN-104、`chain_integrity = MISMATCH`、診断`STALE`）。 | FALSIFIED（前提 P 依存） | 「各`document`の`content_hash`が実ファイル（`path`）と一致することを要求する」。 | UNRESOLVED: REQ-053 と同じ。 |
| `DS-1323` | document / VO の `derives_from` entry に `anchor` を持つ状態と持たない状態の双方を読み取り、いずれも `chain_integrity` に影響しない（`anchor` の欠落・空文字列で `MISMATCH` にならない）。 | FALSIFIED（前提 P 依存） | 「document / VO の `derives_from` entry に `anchor` を持つ状態と持たない状態の双方を読み取り」。document 側は entry object が無い。VO 側は未決。 | SPEC-348 の置換案（document 側）。VO 側は UNRESOLVED: DS-392 と同じ。 |
| `DS-1325` | 同一 `doc` を指す複数 `derives_from` entry を `anchor` 違いで保持でき、重複として拒否しない。 | FALSIFIED（前提 P 依存） | 「同一 `doc` を指す複数 `derives_from` entry を `anchor` 違いで保持できる」。schema の `derived_from` は `uniqueItems: true` の id 配列で、同一 id の重複を持てない。 | SPEC-348 の置換案と同じ。 |
| `DS-1326` | `anchor`だけを変更したdocumentは`content_hash`（`path`の実ファイルのハッシュ）が不変のままdocument subject hashが変化する。 | FALSIFIED（前提 P 依存） | 「`anchor`だけを変更したdocumentは`content_hash`が不変のまま document subject hash が変化する」。両方の field が無い。 | UNRESOLVED: REQ-053 と同じ。 |
| `DS-1327` | `anchor`だけを変更したdocumentは、当該documentを上流依存closureに含む承認・判断記録を失効させる。 | FALSIFIED（前提 P 依存） | DS-1326 と同じ。 | UNRESOLVED: REQ-053 と同じ。 |
| `DS-1430` | document は登録 content_hash と実ファイルの一致も要求し、不一致の document を STALE とし、依存する判断記録も無効とする。 | FALSIFIED（前提 P 依存） | 「登録 content_hash と実ファイルの一致も要求し、不一致の document を STALE とする」。 | UNRESOLVED: REQ-053 と同じ。 |
| `DS-1481` | document 再登録（`--update`）で document subject hash が変化すると当該承認は失効する。 | FALSIFIED（前提 P 依存） | 「document 再登録（`--update`）で document subject hash が変化する」。 | UNRESOLVED: REQ-053 および SPEC-378 と同じ。 |
| `DS-006` | derives_fromとは、document間の唯一のリンク種別である。 | REWORD | REQ-026 と同じ（端点が文書からノードへ変わる）。 | REQ-026 の置換案と同じ。 |
| `DS-025` | 文書間リンクは `derives_from` の一種のみとする。 | REWORD | REQ-026 と同じ。 | REQ-026 の置換案と同じ。 |
| `DS-304` | documentのderives_fromが存在しないdocumentを参照する場合（文書鎖のリンク切れ）は `MISMATCH` とする。 | REWORD | 「documentのderives_fromが存在しないdocumentを参照する場合」— 端点がノードへ変わる。 | ノードの `derived_from` が存在しないノードを参照する場合（鎖のリンク切れ）は `MISMATCH` とする。 |
| `DS-305` | 根に指定されない孤児document（`orphan_detection`）は `MISMATCH` とする。 | REWORD | 「根に指定されない孤児document」の判定基準が REQ-059 で変わる。MISMATCH への写像は生きる。 | REQ-059 の置換案へ揃える。 |
| `DS-314` | content_hash照合は任意形式の文書本文から参照位置の存在を構文的に推測しない。 | REWORD | 「任意形式の文書本文から参照位置の存在を構文的に推測しない」は md 本文を解析する前提。JSON 正本では参照位置がノード id なので推測が構造的に不要になる。 | 参照位置は上流ノードの id であり、文書本文の解析によって推測しない。（出典: specification.schema.json `derived_from` は `#/$defs/id` の配列） |
| `DS-390` | `derives_from` が空のdocumentは根候補であり、`config.yaml` の `doc.roots` に列挙されない場合は孤児として `orphan_detection` の `MISMATCH` とする（§5.6）。 | REWORD | 孤児判定を「`derives_from` が空」で行うと REQ-059 と同じ問題が出る。`doc.roots` による除外は保存形式・config に依存し未決。 | REQ-059 の置換案（実効上流）へ揃える。根の指定については UNRESOLVED: schema の `root` 層が「derived_from を持たない唯一の層」であることが `config.yaml` の `doc.roots` を置き換えるのかが #14 に無い。 |
| `DS-457` | document dependencyはdocument subject hashを使用するため、document recordまたは参照先sourceの変更で承認が失効する（§1.3）。 | REWORD | 「document record または参照先 source の変更で承認が失効する」— 参照先 source が無い。承認の失効という規則自体は生きる。 | UNRESOLVED: REQ-053 と同じ（ノードの subject hash が何を束縛するか）。 |
| `DS-547` | E-SCAN-016はerrorであり、根に指定されない孤児document（親documentを持たず `doc.roots` にも列挙されない）を意味する（§5.6）。 | REWORD | E-SCAN-016 の定義が孤児判定に依存する。 | REQ-059 の置換案へ揃える。 |
| `DS-563` | E-SCAN-016（孤児document）は `orphan_detection = MISMATCH` に写像する（§5.6）。 | REWORD | DS-547 と同じ。 | REQ-059 の置換案へ揃える。 |
| `DS-572` | `derives_from` が空、かつ他のどのdocumentからも `derives_from` で参照されないdocumentのうち、`doc.roots` に列挙されないものを孤児とし、E-SCAN-016（`orphan_detection = MISMATCH`）とする。 | REWORD | DS-390 と同じ。 | DS-390 と同じ。 |
| `DS-781` | `orphan_detection`は評価地点をDOCとし、親を持たず`doc.roots`にも列挙されないdocumentが無ければ`PASS`、あれば`MISMATCH`とする（§5.6）。 | REWORD | `orphan_detection` の PASS 条件が孤児判定に依存する。 | REQ-059 の置換案へ揃える。 |
| `DS-899` | 孤児documentはE-SCAN-016として検出する。 | REWORD | DS-547 と同じ。 | REQ-059 の置換案へ揃える。 |
| `DS-1001` | 段（要件→仕様→詳細設計…）は `derives_from` リンクで表現し、種別を増やさない。 | REWORD | SPEC-042 と同じ（段の列挙が 6 段へ）。 | SPEC-042 の置換案と同じ。 |
| `DS-1015` | `doc list --tree` は `derives_from` の文書鎖を木として表示する。 | REWORD | `doc list --tree` が `derives_from` の文書鎖を木として表示する。木は `derived_from` ではなく層内の節構造になり、`derived_from` は層をまたぐ辺になる。 | UNRESOLVED: SPEC-378（CLI 面）。木の意味の変更については LAYERING.md §1.1「木 = 各層の『文書 > 節 > 小節 > 文』」を根拠に、木の表示と辺の表示を分ける。 |
| `DS-1018` | `derives_from` の参照先 document が存在しなければ文書鎖のリンク切れとして `chain_integrity = MISMATCH`（E-SCAN-012）とする。 | REWORD | 参照先 document 不在をリンク切れとする規則。到達先がノードになるだけで規則は生きる。 | 参照先ノードが存在しなければリンク切れとして `chain_integrity = MISMATCH`（E-SCAN-012）とする。 |
| `DS-1020` | 根に指定されず親も持たない document は孤児として `orphan_detection = MISMATCH`（E-SCAN-016）とする。 | REWORD | DS-390 と同じ（CLI 文脈）。 | DS-390 と同じ。加えて UNRESOLVED: SPEC-378（CLI 面）。 |
| `DS-1051` | `--subject-type document` は `subject` に document ID を書き込む。 | REWORD | `--subject-type document` が subject に document ID を書き込む。承認は #14 の対象外だが、書き込む id の形（`DOC-` ではなくノード id）が変わる。 | UNRESOLVED: SPEC-378（CLI 面）。id の形は schema の `$defs/id` pattern に従う。 |
| `DS-1142` | `--format json` の trace 出力に含まれる `derives_from` エッジ（DOC → DOC、DOC → VO）は、`anchor` と `note` を同伴する。 | REWORD | trace 出力の `derived_from` エッジ（DOC → DOC、DOC → VO）に `anchor`・`note` を同伴させる。ノード → ノードは到達先 id で置き換わるが、VO 側は #14 の対象外。 | ノード → ノードは SPEC-348 の置換案。DOC → VO は UNRESOLVED: DS-392 と同じ。 |
| `DS-1143` | エッジ要素は `{ "from": "DOC-REQ-001", "relation": "derives_from", "anchor": "§12.3", "note": "", "to": "VO-PARSER-UTF8-003" }` の形とする。 | REWORD | エッジ要素の例に `anchor`・`note` と `DOC-REQ-001` を含む。schema の id pattern に `DOC-` は無い。 | エッジ要素は `{ "from": "<下流ノード id>", "relation": "derived_from", "to": "<上流ノード id>" }` の形とする。（出典: specification.schema.json `$defs/id` の pattern、`derived_from` の型） |
| `DS-1145` | `report --from DOC-REQ-001 --direction down --format json` は、この形式で「どの上流条項がどの概念（VO）へ対応するか」の対応ペア集合を返す。 | REWORD | 候補一覧の外（追加検出）。`DOC-REQ-001` を例に使う。 | DS-1143 の置換案と同じ。加えて UNRESOLVED: SPEC-378。 |
| `DS-1335` | 孤児判定は、`derives_from` が空、かつ他のどの document からも `derives_from` で参照されず、`doc.roots` にも列挙されない document を孤児とし、E-SCAN-016、`orphan_detection = MISMATCH` になる。 | REWORD | DS-390 と同じ。 | DS-390 と同じ。 |
| `DS-1480` | document を対象とする承認の上流依存closureは当該 document の再帰的な上位 document（`derives_from` 先）からなり、`--subject-type document` で記録する。 | REWORD | 「対象 document の再帰的な上位 document（`derives_from` 先）」— 木になったので先祖の辺も上流に入る。 | 上流依存closureは、対象ノードの実効的な上流（自分の辺 ∪ 先祖の辺）を再帰的に閉じたものとする。（出典: LAYERING.md §1.1） |
| `DS-1487` | VO を対象とする承認の上流依存closureは、対象 VO の再帰的 parent VO、対象 VO と parent VO が `derives_from` で参照する document、および各 document の再帰的な上位 document からなる。 | REWORD | DS-1480 と同じ（VO 対象の closure に含まれる document の閉包）。 | DS-1480 の置換案と同じ。 |
| `DS-1488` | document dependency は §1.3 の document subject hash を使用するため、document record または参照先 source の変更で承認が失効する。 | REWORD | DS-457 と同じ。 | UNRESOLVED: REQ-053 と同じ。 |
| `DS-1524` | `report --from DOC-X --direction down --format json` は、`derives_from` エッジごとに `from` / `relation` / `to` と当該 entry の `anchor`・`note` を返し、「どの上流条項がどの VO へ対応するか」の対応ペア集合として読める。 | REWORD | DS-1142 と同じ。 | SPEC-348 の置換案と同じ。 |
| `DS-095` | エンティティ軸は対象とするdocument / VO / Testの部分木を指定する。 | UNRESOLVED | 「エンティティ軸は対象とする document / VO / Test の部分木を指定する」。木が 2 つある（層内の節の木と `derived_from` の辺のグラフ）ため、どちらの部分木かが決まらない。DS-1015 と同じ曖昧さ。 | UNRESOLVED: `LAYERING.md` §1.1 は「木 = 各層の『文書 > 節 > 小節 > 文』」「辺 = `derived_from` 一種」と 2 つを分けたが、scope 指定がどちらを指すかは #14 にも `LAYERING.md` にも無い。 |
| `DS-392` | VO レコードの `derives_from` entryの `anchor` fieldは任意の上流該当箇所（節番号等）であり、空も許容し、`chain_integrity` 違反・`MISMATCH` としない。 | UNRESOLVED | VO レコードの `derives_from` entry の `anchor`。#14 は仕様 JSON を決めたのであって `.verify/vo/` のレコード形は決めていない（#14 本文「現時点では implementation / verification / evidence / approval 等は対象に含めず」）。 | UNRESOLVED: VO レコードが上流ノードをどう参照するか（entry object を保つか id 参照へ揃えるか）が #14 に無い。 |
| `DS-393` | VO レコードの `derives_from` entryの `note` fieldは任意であり、空も許容し、`chain_integrity` 違反・`MISMATCH` としない。 | UNRESOLVED | DS-392 と同じ（`note`）。 | UNRESOLVED: DS-392 と同じ。 |
| `DS-398` | VOの `derives_from` entryもdocumentレコードと同じく任意の `anchor`（参照先document内の該当箇所を指す不透明な文字列。節番号・条項番号・見出し等）と任意の `note` を持つ。 | UNRESOLVED | 「VOの `derives_from` entry も document レコードと同じく任意の `anchor` と `note` を持つ」— document 側の根拠が消えるため前提が崩れる。 | UNRESOLVED: DS-392 と同じ。 |
| `DS-399` | VOの `derives_from` entryの `anchor` / `note` の欠落・空文字列は `chain_integrity` 違反・`MISMATCH` としない（§19）。 | UNRESOLVED | DS-392 と同じ。 | UNRESOLVED: DS-392 と同じ。 |
| `DS-402` | `anchor` と `note` はVO subject hashの入力に含まれない（VO subject hashは `derives_from` の参照先document ID集合を束縛する）（§1.3）。 | UNRESOLVED | 「`anchor` と `note` は VO subject hash の入力に含まれない」。 | UNRESOLVED: DS-392 と同じ。 |
| `DS-571` | `config.yaml` の `doc.roots` に列挙されたDOC IDを根として扱い、`orphan_detection` の対象外とする（§2.2）。 | UNRESOLVED | 候補一覧の外（追加検出）。`config.yaml` の `doc.roots` に列挙した DOC ID を根とする。 | UNRESOLVED: DS-390 の根指定と同じ（`root` 層が `doc.roots` を置き換えるかが未決）。 |
| `DS-573` | 根に指定されたdocumentが存在しないDOC IDを参照する場合は、config invariant違反としてE-CONFIG-001とする。 | UNRESOLVED | DS-571 と同じ。 | UNRESOLVED: DS-571 と同じ。 |
| `DS-784` | 4検査の評価入力は、当該revisionのrepositoryを走査したscan結果（adapterが返すdiscovery出力と、そこからcoreが具体化したエンティティ・内容ハッシュ）、`.verify/`配下の正典ファイル集合（`config.yaml`、documentレコード、VOレコード、Relationレコード、判断記録〔`.verify/decisions/`〕、承認レコード〔`.verify/approvals/… | UNRESOLVED | 4 検査の評価入力に「`.verify/`配下の正典ファイル集合（…documentレコード…）」を含む。保存形式は本レビューの除外事項。 | UNRESOLVED: 保存形式（単一 `specification.json` か 1 ノード 1 ファイルか）。REQ-272「本システムは、不必要に単一共有台帳へ書き込ませない。」REQ-325「並列編集時の物理的保存方式は、下位仕様へ委譲する設計事項である。」 |
| `DS-786` | 4検査の評価入力は、`.verify/`配下の正典ファイル集合（`config.yaml`、documentレコード、VOレコード、Relationレコード、判断記録〔`.verify/decisions/`〕、承認レコード〔`.verify/approvals/`〕、Evidenceレコード〔`.verify/evidence/`〕）を含む。 | UNRESOLVED | DS-784 と同じ。 | UNRESOLVED: DS-784 と同じ。 |
| `DS-916` | `E-CONFIG-001`はerrorであり、config version、`verify.full_scope`（固定4検査）、`doc.roots`、`gates`（名前重複、`require` / `require.verification`欠落、`require.verification`が5状態語彙外、`require.approvals`の不正・未解決ロール）、config field型または登録adapterが検証… | UNRESOLVED | 候補一覧の外（追加検出）。E-CONFIG-001 の値域に `doc.roots` を含む。 | UNRESOLVED: DS-571 と同じ。 |
| `DS-1010` | `--root` / `--no-root` は当該 DOC を `orphan_detection` の除外根（`config.yaml` の `doc.roots`）へ追加／除外する。 | UNRESOLVED | 候補一覧の外（追加検出）。`--root` / `--no-root` が `doc.roots` を操作する。 | UNRESOLVED: DS-571 および SPEC-378 と同じ。 |
| `DS-1011` | 根指定の追加・削除はこのフラグで管理し `doc.roots` へ反映する。 | UNRESOLVED | DS-1010 と同じ。 | UNRESOLVED: DS-1010 と同じ。 |
| `DS-1022` | 旧モデルの `--req`（REQ 参照）・`--spec` / `--section`（SPEC + 節参照）は廃し、上流参照は `--derives-from DOC-*`（任意の `--note`）へ一本化する。 | UNRESOLVED | 候補一覧の外（追加検出）。`--derives-from DOC-*`（任意の `--note`）へ一本化。 | UNRESOLVED: SPEC-378 と同じ。加えて `DOC-` 接頭辞は schema の id pattern に無い。 |
| `DS-1027` | `vo add` / `vo edit` の `--anchor <text>` は直前の `--derives-from` に束縛し、参照先 document 内の該当箇所（節番号・条項番号・見出し等）を記録する。 | UNRESOLVED | `vo add` / `vo edit` の `--anchor`。 | UNRESOLVED: DS-392 と同じ。加えて SPEC-378（CLI 面）。 |
| `DS-1030` | `vo show` は各 `derives_from` entry の `anchor` を表示する。 | UNRESOLVED | `vo show` の `anchor` 表示。 | UNRESOLVED: DS-392 と同じ。 |
| `DS-1101` | 旧モデルの `--req`（REQ 指定）は document 層の総称化により廃止し、document scope が必要な場合は VO 部分木経由で指定する。 | UNRESOLVED | 「旧モデルの `--req` は document 層の総称化により廃止し」— 廃止の理由（総称化）が F11 改定で消える。廃止という結論の当否は別。 | UNRESOLVED: SPEC-378 と同じ。廃止の根拠を層分割（LAYERING.md §1）へ置き換えるかは #14 に無い。 |
| `DS-1194` | `doc_list` / `doc_get` は `id`（get のみ）、`tree: bool`、`roots: bool` を入力とし、document レコード（木・根集合・鮮度）を出力する。 | UNRESOLVED | `doc_list` / `doc_get` の出力（木・根集合・鮮度）。鮮度は REQ-053、根集合は DS-571 の未決に依存し、MCP CRUD は #14 が明示的に先送り。 | UNRESOLVED: #14 本文「MCP の CRUD API…については、このデータモデルの方向性が決まった後に別途検討する」。 |
| `DS-1199` | `vo_list` / `vo_get` は `id`、`doc`、`status` を入力とし、VO レコード、derives_from（`doc` + 任意 `anchor` + 任意 `note`）、covers 状況、承認状態を出力する。 | UNRESOLVED | `vo_list` / `vo_get` の出力に `anchor` / `note` を含む。 | UNRESOLVED: DS-392 と同じ。#14 本文は MCP の CRUD API を明示的に先送りしている。 |
| `DS-1200` | `vo_upsert` は VO フィールド一式（`derives_from[]` 必須1件以上（`doc` + 任意 `anchor` + 任意 `note`）、`dimensions[]`、`coverage_policy`、`combinations[]`（`explicit` のとき必須1件以上。各要素は dimension 名 → partition 値の map））を入力とし、作成・更新結果（承認失効の警告含む）を出力する。 | UNRESOLVED | `vo_upsert` の `derives_from[]` が `doc` + `anchor` + `note`。 | UNRESOLVED: DS-392 と同じ。 |
| `DS-1218` | `doc_upsert` / `vo_upsert` の `derives_from[]` 各要素は `doc`（必須）、`anchor`（任意）、`note`（任意）からなる。 | UNRESOLVED | `doc_upsert` / `vo_upsert` の `derives_from[]` 要素の形。document 側は FALSIFIED、VO 側は未決。 | document 側は SPEC-348 の置換案。VO 側は UNRESOLVED: DS-392 と同じ。 |
| `DS-1219` | `anchor` は参照先 document 内の該当箇所を指す不透明な文字列であり、省略・空文字列を許容し `chain_integrity` 違反にしない。 | UNRESOLVED | DS-1218 と同じ。 | UNRESOLVED: DS-1218 と同じ。 |
| `DS-1334` | 根の除外は、`config.yaml` の `doc.roots` に列挙された DOC ID を根として扱い、`orphan_detection` の対象外とする。 | UNRESOLVED | DS-571 と同じ。 | UNRESOLVED: DS-571 と同じ。 |
| `DS-1336` | `doc.roots` が存在しない DOC ID を参照する場合は config invariant 違反として E-CONFIG-001 とする。 | UNRESOLVED | DS-571 と同じ（存在しない DOC ID 参照は E-CONFIG-001）。 | UNRESOLVED: DS-571 と同じ。 |
| `DS-1497` | 検査軸（4 本の部分集合）とエンティティ軸（対象とする document / VO / Test の部分木）を指定でき、限定scopeのOKは「要求scope内のOK」に限られる。 | UNRESOLVED | DS-095 と同じ。 | UNRESOLVED: DS-095 と同じ。 |
| `DS-1517` | 同一 revision・同一 `.verify/` ファイル集合（`config.yaml`・document / VO / Relation レコード・判断記録・承認・Evidence）・同一 scope 指定に対して `verify` を繰り返し実行すると、4 検査の検証状態・診断ラベル・診断コード集合・集約結果・`pending` section・終了コードが毎回一致する。 | UNRESOLVED | 決定論の条件に「同一 `.verify/` ファイル集合（…document / VO / Relation レコード…）」を置く。 | UNRESOLVED: DS-784 と同じ。 |

**KEEP（46 件）**: `DS-007`、`DS-021`、`DS-027`、`DS-035`、`DS-058`、`DS-104`、`DS-106`、`DS-236`、`DS-252`、`DS-303`、`DS-321`、`DS-388`、`DS-391`、`DS-396`、`DS-397`、`DS-404`、`DS-453`、`DS-455`、`DS-546`、`DS-562`、`DS-702`、`DS-703`、`DS-713`、`DS-724`、`DS-780`、`DS-787`、`DS-799`、`DS-801`、`DS-898`、`DS-997`、`DS-1021`、`DS-1046`、`DS-1058`、`DS-1073`、`DS-1082`、`DS-1123`、`DS-1136`、`DS-1196`、`DS-1209`、`DS-1227`、`DS-1284`、`DS-1285`、`DS-1287`、`DS-1297`、`DS-1475`、`DS-1477`

**NOT-DOC-MODEL（50 件）**: `DS-034`、`DS-226`、`DS-267`、`DS-341`、`DS-342`、`DS-344`、`DS-345`、`DS-427`、`DS-449`、`DS-498`、`DS-521`、`DS-522`、`DS-525`、`DS-526`、`DS-539`、`DS-552`、`DS-566`、`DS-576`、`DS-577`、`DS-583`、`DS-586`、`DS-589`、`DS-590`、`DS-592`、`DS-594`、`DS-595`、`DS-754`、`DS-772`、`DS-814`、`DS-818`、`DS-819`、`DS-827`、`DS-1023`、`DS-1128`、`DS-1180`、`DS-1259`、`DS-1261`、`DS-1264`、`DS-1265`、`DS-1266`、`DS-1267`、`DS-1275`、`DS-1377`、`DS-1379`、`DS-1381`、`DS-1385`、`DS-1388`、`DS-1411`、`DS-1412`、`DS-1567`

### 2.4 基本設計（`basic_design`）

候補 64 件。FALSIFIED（F11 改定） 3 / FALSIFIED（前提 P 依存） 5 / REWORD 4 / UNRESOLVED 12 / KEEP 21 / NOT-DOC-MODEL 19

| id | 現在の文 | 判定 | 理由 | 置換案 / 不足事項 |
|---|---|---|---|---|
| `BD-149` | 文書種別ごとの専用ディレクトリ（旧 `spec/` / `req/`）を設けず、上流文書はすべて `doc/` の総称documentレコード1種で表現する。 | FALSIFIED（F11 改定） | 「文書種別ごとの専用ディレクトリを設けず、上流文書はすべて `doc/` の総称documentレコード1種で表現する」。F11 の逐語＋保存形式。 | SPEC-421 の置換案。保存形式は UNRESOLVED: DS-784 と同じ。 |
| `BD-163` | 上流文書はすべて単一の総称ノード型 `document` で表現する。 | FALSIFIED（F11 改定） | 「上流文書はすべて単一の総称ノード型 `document` で表現する」は F11 の逐語。 | SPEC-421 の置換案と同じ。 |
| `BD-199` | 上流文書はすべてDOCノードとし、文書間・VO→文書は `derives_from` の一種で表現する（§19）。 | FALSIFIED（F11 改定） | 「上流文書はすべてDOCノードとし」。F11 の逐語。加えて `DOC-` 接頭辞が schema の id pattern に無い。 | SPEC-421 の置換案と同じ。 |
| `BD-017` | 文書層の段数は総称的に扱い、リンクを追加してもスキーマが壊れないことを設計制約とする。 | FALSIFIED（前提 P 依存） | 候補一覧の外（追加検出）。REQ-030 と同じ。 | UNRESOLVED: REQ-030 と同じ。 |
| `BD-019` | documentのIDは `DOC-` とし、正典は `.verify/doc/` に置く。 | FALSIFIED（前提 P 依存） | 「documentのIDは `DOC-` とし、正典は `.verify/doc/` に置く」。schema の id pattern に `DOC-` は無く、層ごとの接頭辞になる。 | id 接頭辞は SPEC-421 の置換案。保存先は UNRESOLVED: DS-784 と同じ。 |
| `BD-S042` | [SECTION] 3.1 document レコード（`.verify/doc/DOC-*.yaml`） | FALSIFIED（前提 P 依存） | 候補一覧の外（追加検出）。節 title が `.verify/doc/DOC-*.yaml`。 | UNRESOLVED: DS-784 と同じ。 |
| `BD-138` | `.verify/doc/` は `DOC-<NAME>.yaml` 形式で総称documentレコード（正典）を格納する。 | FALSIFIED（前提 P 依存） | 「`.verify/doc/` は `DOC-<NAME>.yaml` 形式で総称documentレコード（正典）を格納する」。 | UNRESOLVED: DS-784 と同じ。 |
| `BD-288` | fixture は、文書鎖の状態として `content_hash` と実ファイルが一致しない document（W-SCAN-104、`chain_integrity = MISMATCH`、診断 `STALE`）を表現できる。 | FALSIFIED（前提 P 依存） | fixture が「`content_hash` と実ファイルが一致しない document（W-SCAN-104）」を表現できる。前提の検査が成立しない。 | UNRESOLVED: REQ-053 と同じ。 |
| `BD-165` | `derives_from` は上流documentへの唯一のリンク種別である。 | REWORD | REQ-026 と同じ。 | REQ-026 の置換案と同じ。 |
| `BD-166` | 文書層の段（要件→仕様→詳細設計…）は `derives_from` リンクで表現し、段を増やしても種別を増やさない。 | REWORD | SPEC-042 と同じ。 | SPEC-042 の置換案と同じ。 |
| `BD-171` | 「どの上流条項がどのVOへ対応するか」の対応ペアは、`anchor` 付き `derives_from` エッジとして保持し、§11.6のprojection出力で露出する。 | REWORD | SPEC-348 と同じ（`anchor` 付きエッジで対応ペアを保持）。対応ペアの到達先が VO なので、VO 側の参照形は #14 の対象外。 | ノード → ノードは SPEC-348 の置換案。VO への対応ペアは UNRESOLVED: DS-392 と同じ。 |
| `BD-286` | fixture は、文書鎖の状態として `derives_from` が空かつ根に列挙されない孤児 document（E-SCAN-016、`orphan_detection = MISMATCH`）を表現できる。 | REWORD | fixture の孤児 document。REQ-059 で判定基準が変わる。 | REQ-059 の置換案へ揃える。 |
| `BD-011` | 宣言層は、adapter所有のTest metadata宣言、および.verify/配下のdocument / VO / Relationレコードからなり、Gitで管理される正典である。 | UNRESOLVED | 宣言層に「.verify/配下のdocument / VO / Relationレコード」を数える。保存形式は除外事項。 | UNRESOLVED: DS-784 と同じ。 |
| `BD-045` | `.verify/` にテストコード外の正典と事実レコードを保存する。 | UNRESOLVED | 「`.verify/` にテストコード外の正典と事実レコードを保存する」。 | UNRESOLVED: DS-784 と同じ。 |
| `BD-047` | `.verify/doc/` はdocumentレコード（正典）を格納する。 | UNRESOLVED | 「`.verify/doc/` はdocumentレコード（正典）を格納する」。 | UNRESOLVED: DS-784 と同じ。 |
| `BD-059` | document / VOは1エンティティ1ファイルとする。 | UNRESOLVED | 「document / VOは1エンティティ1ファイルとする」。単一 `specification.json` は 1 ファイルに全ノードを載せる。 | UNRESOLVED: DS-784 と同じ。単一 `specification.json` それ自体が共有台帳であり、REQ-272 との緊張は Owner の裁定事項。 |
| `BD-060` | document / VOのファイル名をIDとする。 | UNRESOLVED | 「document / VOのファイル名をIDとする」。 | UNRESOLVED: BD-059 と同じ。 |
| `BD-072` | `vtest doc add / list / show` の責務はdocumentレコードの管理（derives_from・根指定を含む）とする。 | UNRESOLVED | `vtest doc add / list / show` の責務。 | UNRESOLVED: SPEC-378 と同じ。 |
| `BD-087` | `doc_list` / `doc_get` / `doc_upsert` の対応機能はdocument管理とする。 | UNRESOLVED | `doc_list` / `doc_get` / `doc_upsert` の対応機能。 | UNRESOLVED: DS-1194 と同じ。 |
| `BD-203` | 根指定の追加・削除は `vtest doc` コマンドの引数で管理し `doc.roots` へ反映する。 | UNRESOLVED | 候補一覧の外（追加検出）。根指定を `vtest doc` の引数で管理し `doc.roots` へ反映する。 | UNRESOLVED: DS-571 および SPEC-378 と同じ。 |
| `BD-218` | 旧モデルの`--req`（REQ指定）はdocument層の総称化により廃止し、document scopeが必要な場合はVO部分木経由で指定する。 | UNRESOLVED | 「旧モデルの`--req`はdocument層の総称化により廃止し」。 | UNRESOLVED: DS-1101 と同じ。 |
| `BD-236` | 「その時点の正典の読み取り」は書込みの原子的公開（基本仕様 §24.2）を前提とする。 | UNRESOLVED | 「その時点の正典の読み取り」が書込みの原子的公開を前提とする。 | UNRESOLVED: DS-784 と同じ。 |
| `BD-285` | fixture は、文書鎖の状態として `doc.roots` に列挙された根 document を表現できる。 | UNRESOLVED | fixture が `doc.roots` に列挙された根 document を表現できる。 | UNRESOLVED: DS-571 と同じ。 |
| `BD-289` | fixture は、文書鎖の状態として document 再登録で失効する判断記録・承認記録を表現できる。 | UNRESOLVED | fixture が「document 再登録で失効する判断記録・承認記録」を表現できる。再登録経路が未決。 | UNRESOLVED: REQ-053 および SPEC-378 と同じ。 |

**KEEP（21 件）**: `BD-009`、`BD-067`、`BD-076`、`BD-091`、`BD-107`、`BD-135`、`BD-150`、`BD-160`、`BD-169`、`BD-170`、`BD-175`、`BD-177`、`BD-178`、`BD-210`、`BD-212`、`BD-241`、`BD-261`、`BD-287`、`BD-299`、`BD-304`、`BD-305`

**NOT-DOC-MODEL（19 件）**: `BD-012`、`BD-020`、`BD-021`、`BD-023`、`BD-024`、`BD-025`、`BD-026`、`BD-046`、`BD-048`、`BD-049`、`BD-050`、`BD-115`、`BD-119`、`BD-133`、`BD-139`、`BD-140`、`BD-141`、`BD-181`、`BD-295`

### 2.5 詳細設計（`design`）

候補 80 件。FALSIFIED（F11 改定） 6 / FALSIFIED（前提 P 依存） 12 / REWORD 5 / UNRESOLVED 5 / KEEP 15 / NOT-DOC-MODEL 37

| id | 現在の文 | 判定 | 理由 | 置換案 / 不足事項 |
|---|---|---|---|---|
| `DES-001` | documentとは、ソースコードより上流に位置する成果物を表す単一の総称ノードである。 | FALSIFIED（F11 改定） | 「documentとは、…単一の総称ノードである」。F11 の逐語。 | SPEC-421 の置換案と同じ。 |
| `DES-002` | documentは `id + path + content_hash + 上流参照（derives_from）` を持つ。 | FALSIFIED（F11 改定） | 「documentは `id + path + content_hash + 上流参照（derives_from）` を持つ」。F11 の最小形の逐語で、schema のノード形と一致しない。 | 文ノードは `id` / `statement` / `derived_from` / `source` を必須とし `description` を任意とする。節ノードは `id` / `title` / `source` を必須とし `description` / `derived_from` / `sections` / `items` を任意とする。根ノードは `id` / `statement` / `source` を必須とし `derived_from` を持たない。（出典: specification.schema.json `$defs/derivedItem` / `$defs/section` / `$defs/rootItem`。ただし `source` は schema 自身が「移行期間限定」と宣言している） |
| `DES-010` | 上流文書はすべて単一の総称ノード型 `document` で表現する。 | FALSIFIED（F11 改定） | DES-001 と同じ。 | SPEC-421 の置換案と同じ。 |
| `DES-023` | documentは総称の上流文書ノード（path＋content_hash＋derives_from）である。 | FALSIFIED（F11 改定） | 「documentは総称の上流文書ノード（path＋content_hash＋derives_from）である」。 | DES-002 の置換案と同じ。 |
| `DES-092` | document subject hashは、要件定義・基本仕様・詳細設計・API Schema等を種別で区別せず、すべて同一の総称document subjectとして計算する（§3.1）。 | FALSIFIED（F11 改定） | 「document subject hashは、要件定義・基本仕様・詳細設計・API Schema等を種別で区別せず、すべて同一の総称document subjectとして計算する」。F11 の逐語。 | UNRESOLVED: SPEC-421（層で分ける）と REQ-053（ハッシュの束縛対象）の両方。 |
| `DES-149` | 承認対象種別 `document` は、総称documentとして登録した文書で表現し、専用のエンティティ型を設けない（§3.1）。 | FALSIFIED（F11 改定） | 「総称documentとして登録した文書で表現し、専用のエンティティ型を設けない」。SPEC-441 と同じ。 | UNRESOLVED: SPEC-012 と同じ。 |
| `DES-003` | 各derives_fromリンクは任意（optional）の説明文・導出理由を保持できる（§3.2）。 | FALSIFIED（前提 P 依存） | 「各derives_fromリンクは任意の説明文・導出理由を保持できる」。schema の `derived_from` は id の並びで、リンクに付随する値を持てない。 | SPEC-348 の置換案と同じ。 |
| `DES-025` | derives_fromの説明文もRelationに保持できる。 | FALSIFIED（前提 P 依存） | 「derives_fromの説明文もRelationに保持できる」。DES-003 と同じ前提に立つ。 | SPEC-348 の置換案と同じ。 |
| `DES-S036` | [SECTION] 3.1 document レコード（`.verify/doc/DOC-*.yaml`） | FALSIFIED（前提 P 依存） | 候補一覧の外（追加検出）。節 title が `.verify/doc/DOC-*.yaml`。 | UNRESOLVED: DS-784 と同じ。 |
| `DES-051` | 取り込まれた上流成果物はcontent_hashとderives_fromを持つ。 | FALSIFIED（前提 P 依存） | 「取り込まれた上流成果物はcontent_hashとderives_fromを持つ」。 | UNRESOLVED: REQ-053 と同じ。 |
| `DES-091` | document subject hashはdomain `vtest:document-subject:v1` を用い、canonical document recordと参照先source（`path` の実ファイル）の正規化内容を束縛する。 | FALSIFIED（前提 P 依存） | 「document subject hashは…canonical document recordと参照先source（`path` の実ファイル）の正規化内容を束縛する」。参照先 source が無い。domain 分離（`vtest:document-subject:v1`）自体は生きる。 | UNRESOLVED: ノードの subject hash が何を束縛するか（statement のみか、`description` を含むか、節ノードは子の Merkle か、`source` は移行期間限定なので入れられない）が #14・schema・LAYERING.md のいずれにも無い。 |
| `DES-113` | document レコードの `path` fieldはプロジェクト相対パスである。 | FALSIFIED（前提 P 依存） | 「document レコードの `path` fieldはプロジェクト相対パスである」。schema のノードに `path` は無い。`source.doc` は schema 自身が「移行期間限定」と宣言しているので置換に使えない。 | UNRESOLVED: DES-091 と同じ。 |
| `DES-114` | document レコードの `content_hash` fieldは登録時の内容ハッシュである（§1.3）。 | FALSIFIED（前提 P 依存） | 「document レコードの `content_hash` fieldは登録時の内容ハッシュである」。 | UNRESOLVED: DES-091 と同じ。 |
| `DES-115` | `anchor` は `derives_from` entryのfieldであり、Test metadataには存在しない（§4.1）。 | FALSIFIED（前提 P 依存） | 「`anchor` は `derives_from` entryのfieldであり」。entry object が無い。 | SPEC-348 の置換案と同じ。 |
| `DES-116` | `anchor` はcanonical document recordの一部であり、document subject hashの入力に含まれる（§1.3）。 | FALSIFIED（前提 P 依存） | 「`anchor` はcanonical document recordの一部であり、document subject hashの入力に含まれる」。 | SPEC-348 の置換案。ハッシュ入力は UNRESOLVED: DES-091 と同じ。 |
| `DES-158` | 承認レコード a が A(X) に属するのは、`a.approved_state` が値域内であること、a の対象指定が X と一致すること（X が VO / document のとき `a.subject == X`、X が判断記録のとき `a.judgment_ref == X` の ULID）、`a.subject_hash` が `a.subject` の現在の内容ハッシュと一致すること、`a.dependencies` … | FALSIFIED（前提 P 依存） | A(X) の条件に「`a.dependencies` の各 document が登録 content_hash と実ファイルの一致を満たすこと」を含む。 | UNRESOLVED: REQ-053 と同じ。 |
| `DES-163` | 承認レコードaがA(X)に属するには、`a.dependencies` の各documentが登録content_hashと実ファイルの一致を満たさなければならない（§11.4）。 | FALSIFIED（前提 P 依存） | DES-158 の該当条項を単独で述べたもの。 | UNRESOLVED: REQ-053 と同じ。 |
| `DES-482` | `doc add` は `--path` の対象ファイルの sha256 を計算して document subject へ束縛した DOC レコードを作成する。 | FALSIFIED（前提 P 依存） | 「`doc add` は `--path` の対象ファイルの sha256 を計算して document subject へ束縛した DOC レコードを作成する」。 | UNRESOLVED: REQ-053 および SPEC-378 と同じ。 |
| `DES-147` | 承認対象種別 `vo` は、レコード上 `subject` にVO ID（`VO-*`）で表現し、上流依存closureは対象VOの再帰的なparent VO、対象VOと各parent VOが `derives_from` で参照するdocument、および各documentの再帰的な上位document（`derives_from` 先）である。 | REWORD | VO 対象の closure に含まれる「各documentの再帰的な上位document」。木では先祖の辺も上流に入る。 | DS-1480 の置換案と同じ。 |
| `DES-148` | 承認対象種別 `document` は、レコード上 `subject` にdocument ID（`DOC-*`）で表現し、上流依存closureは対象documentの再帰的な上位document（`derives_from` 先）である。 | REWORD | 「上流依存closureは対象documentの再帰的な上位document（`derives_from` 先）である」。 | DS-1480 の置換案と同じ。 |
| `DES-378` | document / VO / Relation / 判断記録 / 承認記録 / Evidence も §3 のスキーマに対応するstructを定義する。 | REWORD | 「document / VO / … も §3 のスキーマに対応するstructを定義する」。document 側の struct 形が変わる。 | DES-002 の置換案に対応する struct を定義する。 |
| `DES-380` | 検証グラフのエッジ `DOC → DOC` は `derives_from` であり、documentレコード由来である。 | REWORD | 「検証グラフのエッジ `DOC → DOC` は `derives_from` であり、documentレコード由来である」。端点がノードになる。 | 検証グラフのエッジ「上流ノード ← 下流ノード」は `derived_from` であり、仕様ノード由来である。（出典: specification.schema.json `derived_from`） |
| `DES-467` | projectionが出力する`derives_from`エッジ（DOC → DOC、DOC → VO）には、当該entryの`anchor`（§3.1・§3.2）を常に同伴させる。 | REWORD | SPEC-348 と同じ（projection の `anchor` 同伴）。ただし本文は「DOC → DOC、DOC → VO」と両方を挙げており、VO 側は #14 の対象外。 | ノード → ノードのエッジは SPEC-348 の置換案。DOC → VO のエッジは UNRESOLVED: DS-392 と同じ。 |
| `DES-S084` | [SECTION] 12.2 `vtest doc add / list / show` | UNRESOLVED | 候補一覧の外（追加検出）。節 title が `12.2 `vtest doc add / list / show``。 | UNRESOLVED: SPEC-378 と同じ。 |
| `DES-111` | `doc.roots` は orphan_detection の除外根をDOC IDの集合として保持する（§5.6）。 | UNRESOLVED | 候補一覧の外（追加検出）。「`doc.roots` は orphan_detection の除外根をDOC IDの集合として保持する」。 | UNRESOLVED: DS-571 と同じ。 |
| `DES-181` | `approved` から `draft` への遷移は、上流依存closureのentity構成またはいずれかのhashが変化する（document再登録・参照先source変更を含む）ことで起きる（§11.4）。 | UNRESOLVED | 「`approved` から `draft` への遷移は…（document再登録・参照先source変更を含む）」。再登録経路と参照先 source が無い。 | UNRESOLVED: REQ-053 および SPEC-378 と同じ。 |
| `DES-237` | 処理フロー第6段は、`.verify/` 読み込みであり、vtest-storeが全レコード（document / VO / Relation / 判断 / 承認 / Evidence）を読み込み、スキーマ検証する。 | UNRESOLVED | 「vtest-storeが全レコード（document / VO / …）を読み込み、スキーマ検証する」。 | UNRESOLVED: DS-784 と同じ。 |
| `DES-474` | 原子的公開の対象は`.verify/`配下のrecord・エンティティファイル（新規レコード追加とエンティティファイル編集）であり、完全な内容が単一の操作で可視になる方式（同一ファイルシステム内へのtemp書込み＋rename等）で公開し、書きかけ状態・一時ファイル残渣を正典ディレクトリの読み手に観測させてはならない。 | UNRESOLVED | 原子的公開の対象が「`.verify/`配下のrecord・エンティティファイル」。 | UNRESOLVED: DS-784 と同じ。 |

**KEEP（15 件）**: `DES-066`、`DES-095`、`DES-128`、`DES-130`、`DES-134`、`DES-139`、`DES-146`、`DES-150`、`DES-152`、`DES-156`、`DES-160`、`DES-238`、`DES-381`、`DES-426`、`DES-534`

**NOT-DOC-MODEL（37 件）**: `DES-005`、`DES-057`、`DES-077`、`DES-080`、`DES-081`、`DES-083`、`DES-085`、`DES-087`、`DES-090`、`DES-093`、`DES-094`、`DES-097`、`DES-109`、`DES-117`、`DES-120`、`DES-122`、`DES-192`、`DES-194`、`DES-196`、`DES-200`、`DES-211`、`DES-252`、`DES-323`、`DES-329`、`DES-332`、`DES-333`、`DES-339`、`DES-353`、`DES-363`、`DES-365`、`DES-367`、`DES-448`、`DES-449`、`DES-459`、`DES-468`、`DES-537`、`DES-540`

---

## 3. 最初に偽になる層（所見）

**前提 P（§0.3）が成立するなら、書き換えは要件定義（`require`）から始めなければならない。** 監査した 5 層のうち最上位である。
**前提 P が成立しないなら、最初に偽になる層は存在しない**（§0.3 の読み B）。以下は前提 P を置いた場合の所見である。

### 3.1 層ごとの型付けによる偽 — 要件定義 `REQ-025`

```
REQ-025  上流文書はすべて単一の総称ノード型 `document`（id + path + content_hash + 上流参照）で表現する。
```

これは ROOT-037（Issue #11 F11）の「最小形の内容」の逐語である。#14 コメント 1 で Owner が「凍結台帳はどうでもいいです。そんなん今解答されました。」と述べ、F11 は改定された。
`specification.schema.json` は 7 層のトップレベル配列を `required` に置き、`$defs/id` の pattern を層ごとの接頭辞に分けている。総称 1 種ではない。

REQ-025 と同根で、層ごとの型付けによって偽になる文は全層で 17 件ある。Owner の 2026-09-04 の裁定が F11 を名指しで解除したので、本書の判定のうち最も証拠が強い群である。

| 層 | id |
|---|---|
| require | `REQ-025` |
| spec | `SPEC-378`、`SPEC-421`、`SPEC-441`、`SPEC-445` |
| detailed_spec | `DS-044`、`DS-046`、`DS-1053` |
| basic_design | `BD-149`、`BD-163`、`BD-199` |
| design | `DES-001`、`DES-002`、`DES-010`、`DES-023`、`DES-092`、`DES-149` |

### 3.2 ノードの形が変わったことによる偽 — 要件定義 `REQ-030` / `REQ-053` / `REQ-098` / `REQ-234`

```
REQ-030  文書層の段数は総称的に設計し、リンクを追加してもスキーマが壊れないことを設計制約とする。
REQ-053  文書層では、各 `document` の derives_from 参照先が存在し、content_hash が現物と一致することを要求する。
REQ-098  文書鎖のリンク切れ / content_hash 不一致 / 孤児文書のいずれかが生じる場合、状態は `MISMATCH`（診断ラベルは STALE 等）となる。
REQ-234  取り込まれた上流成果物は §3.2 の `document` ノードとして登録され、content_hash と derives_from を持つ。
```

`REQ-030` は schema が `required` に 7 層を列挙し `additionalProperties: false` を置いているため、層を 1 つ足すと schema が変わる。
残る 3 件は「1 document = 1 ファイル」を前提とする内容ハッシュの照合に依存する。JSON 正本ではノードに対応する現物ファイルが無い。

### 3.3 root 層

`ROOT-037` は Owner 自身の裁定なので偽にはならない。**改定された**（superseded）。§1.1 のとおり、その記録方法は UNRESOLVED。

### 3.4 語だけの直しで済む最上位 — `REQ-026` / `REQ-059`

`REQ-026`（リンクは 1 種のみ）と `REQ-059`（孤児の問い）は規範の内容が生きる。ただし `REQ-059` の直しは見た目より重い。§4.7 に書いた。

### 3.5 下位層だけを繕ってはならない箇所

`SPEC-421` / `BD-163` / `DES-001` / `DES-010` は、いずれも `REQ-025` の具体化として書かれている。`REQ-025` を直さずにこれらだけ直すと、要件定義が総称モデルを命じたまま下位が層分割を実装する形になる。
同様に `DS-105` / `DS-800` / `DS-1288` / `BD-288` / `DES-091` / `DES-114` の内容ハッシュ照合は、すべて `REQ-053` の具体化である。

---

## 4. 帰結（決定ではなく、書き換えが引き起こすこと）

### 4.1 subject hash の束縛対象が「ファイル」から「ノード」へ移ることの意味

詳細設計 §1.3 は 5 つの domain 分離ハッシュを定める。うち document に関わるのは `vtest:document-subject:v1`（`DES-091`）で、現在は「canonical document record と参照先 source（`path` の実ファイル）の正規化内容」を束縛する。

束縛対象が文ノードになると、次が決まっていない。**いずれも既決材料に無いので、本書は案を出さない。**

1. **何を入力に取るか。** `statement` だけか、`description` を含むか。`description` は schema で任意 field であり、規範ではなく背景・理由・例（`CONVERSION.md` §2）。含めれば、例を直しただけで承認が失効する。含めなければ、規範でない説明が正本の中でハッシュに守られない領域になる。
2. **節ノードのハッシュは何か。** 節は文を持たないこともある（schema `$defs/section` の説明が明記）。子の Merkle にするのか、`title` と自分の `derived_from` だけにするのか。
   子の Merkle にすると、末端の文を 1 つ直すだけで文書ノードまでのすべての祖先ハッシュが変わり、`LAYERING.md` §1.1 の「文の実効的な上流 = 自分の辺 ∪ 先祖の辺」と組み合わさって、無関係な承認が広く失効する。
3. **`source` は入力にできない。** schema は `source` を「移行期間限定」と宣言している。`source.doc` は元 md のパスであり、`path` の代わりに使うと移行期間限定の field が恒久の hash 入力になる。`cites` も同じ理由で使えない。
4. **F5 との関係。** ROOT-031（F5）「証拠は検証対象の内容ハッシュに束縛される」は生きる。失効の仕組みそのものは残り、束縛する対象だけが変わる。

**この 4 点は Owner へ返すべき最大の未決事項である。** 決まらないうちは `DES-091`・`DES-114`・`REQ-053` の置換文が書けない。

### 4.2 `export/*.md` は JSON を編集した瞬間に古くなる

`export/` の md 6 本は `specification.json` からの生成物である。生成に使った `build.py` は 2026-09-06 に Owner の指示で削除された（`CONVERSION.md` 冒頭、Issue #14 コメント 10）。

したがって **JSON を 1 文でも編集すると、`export/*.md` を再生成する手段が現時点で存在しない。** `CONVERSION.md` は「同等の機能（id の付番、参照の辺、節の木、被覆・限定語の検査、md エクスポート）は vtest 側で実装する」と書いているが、それは未実装である。

帰結は 2 つ。

- 本書の書き換えを適用する前に、md エクスポートを vtest 側で実装しておく必要がある。さもなければ `export/*.md` は正本と食い違ったまま残る。
- 足場の最終版は git 履歴 `a2fa4aa` にある。ただし Owner は「これを正としないために消してください」と指示しているので、**復活させて使うことは指示に反する。** 実装の参照としてのみ読める。

### 4.3 書き換えが作る退役 id

`CONVERSION.md` §7 は、id が退役するのは**層を移した文**の場合だと書いている（「`layer` を変えた文は新しい層で新 id を得て、旧 id は `relations/retired-ids.json` に退役として記録され再利用されない」）。

本書の書き換えは層を移さない。文を**消す**か、文の内容を**置き換える**。この 2 つの id の扱いが `CONVERSION.md` に無い。

- **消す場合**: §7 は「文を消しても番号は詰めない」と書くが、消した id を退役台帳へ入れるかは書いていない。
- **REWORD の場合**: 同じ id を保つのか、新 id を振って旧 id を退役させるのかが書いていない。判断記録・承認は id ではなく subject hash に束縛されるので、id を保っても失効は起きる。

**UNRESOLVED**: 削除と内容置換のときの id の扱い。仮に「FALSIFIED は削除して退役、REWORD は id 据置」とすると、退役する id は前提 P の裁定で **0 件 / 17 件 / 74 件** のいずれかになる（読み B なら 0、F11 改定だけ通すなら 17、前提 P も通すなら 74）。

### 4.4 `derives_from` / `derived_from` の綴り

既決材料はすべて `derived_from`、現行仕様はすべて `derives_from`。両者は同じものを指す。

- 綴りを揃えると、`document` 側だけでなく **VO レコードの field 名にも及ぶ**（`DS-391`、`DS-396`、`DES-095`、`BD-170` ほか）。VO は #14 の対象外である（#14 本文「現時点では implementation / verification / evidence / approval 等は対象に含めず」）。
- 綴りを揃えないと、正本 JSON と製品モデルで同じ関係が 2 つの名前を持つ。

**UNRESOLVED**: どちらの綴りを正とするか、および VO レコードへ波及させるか。#14 に記述が無い。

### 4.5 「参照」への後退が `derived_from` の意味を変えた

#14 の**本文**は `derived_from` を「この成果物の存在・内容を、指定された上流成果物から直接導出できる」と定義していた。
2026-09-05 の Owner 裁定「あくまで derived_from は参照なので。」はこれを参照へ後退させ、「ペアごとの承認は行わない」（書記の読み）ことになった。

現行仕様の `SPEC-013`「derives_fromは上流documentから下流documentへの導出を表す」はこの後退の前の意味を書いている。§2.2 で REWORD にした。
`ROOT-030`（F4）「発見を可能にするデータ形態…までが vtest の責務。発見者は外部」とは整合する。参照であって導出の証明ではないなら、対応の良し悪しを裁定するのは外部である。

### 4.6 保存形式は決めていないが、緊張だけは記録する

本レビューの除外事項なので決めない。ただし次の 2 文は現行仕様の中にあり、単一 `specification.json` と正面から当たる。

```
REQ-272  本システムは、不必要に単一共有台帳へ書き込ませない。
BD-058   全員が編集する中央共有台帳を持たない。
```

一方、逃げ道も現行仕様の中にある。

```
REQ-325  並列編集時の物理的保存方式は、下位仕様へ委譲する設計事項である。
```

`REQ-272` は「不必要に」で限定されているので、必要性が示されれば単一ファイルも排除されない。`REQ-325` は物理的保存方式を下位へ委譲している。
**したがって、単一 `specification.json` を採るかどうかは要件定義の書き換えを必要とせず、下位仕様の決定として処理できる可能性がある。これは所見であって決定ではない。**

保存形式の裁定に依存する文は 22 件: `SPEC-S071`、`DS-784`、`DS-786`、`DS-1517`、`DS-S071`、`BD-011`、`BD-019`、`BD-045`、`BD-047`、`BD-057`、`BD-058`、`BD-059`、`BD-060`、`BD-061`、`BD-105`、`BD-138`、`BD-149`、`BD-233`、`BD-236`、`BD-S042`、`DES-237`、`DES-474`、`DES-S036`。

### 4.7 孤児判定を直さないと、正本の大半が孤児になる

`LAYERING.md` §1.1 は「文の実効的な上流 = 自分の辺 ∪ 先祖（節・文書）の辺。保存せず計算する」と書き、
「文書末尾のトレーサビリティ表（節 → 上流節、136 行）は節の辺」「表を文単位に展開しない」と決めている。

つまり **大多数の文ノードは自分の辺を持たず、節の辺に覆われている。** 現行の孤児判定（`REQ-059` ほか）は「親を持たない `document` ノード」を問うので、そのまま適用すると正本の大半が孤児になる。
加えて #14 コメント 2 は「解決できない引用（21件）は空のまま残す」と決めており、これらは実効上流も持たない。

`orphan_detection` に関わる文（`REQ-059`、`SPEC-059`、`SPEC-291`、`SPEC-422`、`DS-305`、`DS-390`、`DS-547`、`DS-563`、`DS-572`、`DS-781`、`DS-899`、`DS-1020`、`DS-1335`、`BD-286`）はすべて REWORD が必要で、
**この 1 点だけは前提 P の裁定にかかわらず、正本 JSON をこのまま vtest に読ませたときに直ちに誤検出を出す。**

### 4.8 「根」が 2 つある

現行モデルの根は `config.yaml` の `doc.roots` に列挙した DOC ID（`DS-571`、`DES-111`、`DS-1010`）。
schema の根は `root` 層（「`derived_from` を持たない唯一の層」）。

**UNRESOLVED**: `root` 層が `doc.roots` を置き換えるのか、両方が要るのかが #14 にも schema にも無い。関係する文は 10 件: `DS-571`、`DS-573`、`DS-916`、`DS-1010`、`DS-1011`、`DS-1334`、`DS-1336`、`DES-111`、`BD-203`、`BD-285`。

---

## 5. 開示 — 確認していないこと

1. **前提 P の当否を確かめていない。** §0.3 のとおり、#14 に Owner の言葉として存在しないことは確認し、支持材料 2 件を挙げた。どちらの読みが正しいかは Owner にしか決められない。**本書の FALSIFIED 判定 74 件（F11 改定 17 + 前提 P 依存 57）はすべてこの裁定にぶら下がっている。**
2. **`specification.json` の全 3,854 ノードを読んでいない。** 事前フィルタの 350 件と、取りこぼし掃引の 25 件だけを見た。掃引に使った語は `DOC-` / `doc.roots` / `registered_at` / `--path` / `sha256` / `W-SCAN-104` / `E-SCAN-016` / `E-SCAN-012` / `上流成果物` / `doc_upsert` / `doc add` / `.verify/doc` / `文書鎖` / `総称` / `導出関係` / `正本` / `エクスポート`。これ以外の語で書かれた document モデル依存文は見つけていない。
3. **`description` field を読んでいない。** 判定は各ノードの `statement` だけに当てた。`description` に規範が混ざっている可能性は排除していない（`CONVERSION.md` §2 は混ぜないと定めているが、検査はしていない）。
4. **節ノード（`*-S###`）を体系的には見ていない。** 掃引で引っかかった 5 件（`SPEC-S071`、`DS-S071`、`BD-S042`、`DES-S036`、`DES-S084`）だけを扱った。節の `title` と `description` は全件見ていない。
5. **実装（`crates/`）を一切見ていない。** 本書は仕様の突合であり、実装との差分は対象外。
6. **`reports/upstream-traceability-audit-2026-09-04.md` §5 の 13 件には触れていない**（除外事項）。それらの指摘と本書の書き換えが衝突するかは確認していない。
7. **保存形式を決めていない**（除外事項）。§4.6 に緊張の記録だけを置いた。
8. **`export/*.md` の内容と `specification.json` の一致を検証していない。** §4.2 は「編集すれば古くなる」という帰結であり、現時点で一致しているかは確かめていない。
9. **置換案を `specification.json` へ適用していない。** ファイルは一切変更していない。置換案が schema を通るかの機械検証もしていない。
10. **VO / Evidence / 承認レコードの形は #14 の対象外として扱った。** #14 本文の限定に従ったが、Owner がこの限定を維持しているかは 2026-09-05 の「導入するでいいんじゃないの？」以外に確認材料が無い。

---

## 6. 上流へ返す質問（優先順）

いずれも本書では決められない。**答えが変わると下流の書き換え範囲が変わる順**に並べた。

1. **前提 P を確認する。** `specification.json` は vtest 自身の `.verify/doc/` 文書モデルそのものか。#14 に Owner の逐語は無いが、2026-09-04 の F11 解除と 2026-09-06 の「vtest がやるべきこと」の 2 件が支持し、置かない読みは 09-06 の発言と衝突する。（§0.3。FALSIFIED 判定 74 件がこの確認にぶら下がる）
2. **ノードの subject hash は何を束縛するか。** `statement` のみか、`description` を含むか。節ノードは子の Merkle か。（§4.1。`REQ-053`・`DES-091`・`DES-114` の置換文が書けない）
3. **改定された根ノード（ROOT-037）をどう記録するか。**（§1.1）
4. **`root` 層は `config.yaml` の `doc.roots` を置き換えるか。**（§4.8）
5. **`derives_from` / `derived_from` のどちらを正とするか。VO レコードへ波及させるか。**（§4.4）
6. **7 層に当たらない上流成果物（API Schema、DB schema、方針）をどこへ置くか。**（`SPEC-012`、`SPEC-441`、`DES-149`）
7. **CLI（`vtest doc add / list / show`）と MCP CRUD は新モデルで存続するか。** #14 本文は明示的に先送りしている。（§2.2 以降の UNRESOLVED 多数）
8. **削除・内容置換のときの id の扱い。**（§4.3）

---

# 適用結果（2026-09-06）

**本節より上は提案である。本節は適用の記録である。** Owner が 2026-09-06 に前提 P（§0.3）を確認したため、読み A で `docs/canonical/specification.json` と `docs/canonical/relations/retired-ids.json` を編集した。コミットはしていない。

適用の枠（PM 指示、審査中に変更していない）:

- 書き換えは最上位層から下流へ順に当てた（require → spec → detailed_spec → basic_design → design）。
- 置換文は既決材料（#14 の Owner 裁定、`specification.schema.json` のノード形、`LAYERING.md` §1.1、`CONVERSION.md`）の転記に限った。転記できないものは**据置**とし、本節 §A.5 に一覧で出す。
- 規範が変わった文は**新 id** を得て、旧 id を `relations/retired-ids.json` へ退役させた。規範が変わらない語の直し（REWORD）は **id 据置**とした。
- `reports/upstream-traceability-audit-2026-09-04.md` §5 の 13 箇所に対応するノードには触れていない（Owner が別途審査中）。
- **孤児判定の REWORD は 14 件中 12 件である。** 残り 2 件（`DS-572`、`DS-1335`）は監査主題 C（孤児判定に第二条件を AND で追加）として Owner 審査中のため未適用であり、**`detailed_spec` の中で孤児の定義が一時的に不揃いである。**
- 本節は 3 回に分けて適用した。**第1次**（§A.2・§A.3）は層分割・ノード形・孤児判定、**第2次**（§A.2.1）は `W-SCAN-104` の再束縛と `ROOT-047` の出典付け替え、**第3次**（§A.2.2）は導出 3 の訂正。
- **第2次の導出 3 は誤っていた。** 「VO レコードの `derives_from` に記録された束縛ハッシュ」という記録場所を発明していた。第3次で訂正し、第2次が作った新 id のうち 11 件を同じ日のうちに再度退役させた。経緯は §A.4 の導出 3 に残す。

## A.1 追加した root ノード（9 件）

| id | 内容 | 出典 |
|---|---|---|
| `ROOT-039` | JSON を正本にして md へ一方向エクスポートする | Issue #14 コメント 1（2026-09-04） |
| `ROOT-040` | 「凍結台帳はどうでもいいです。そんなん今解答されました。」 | Issue #14 コメント 1（2026-09-04） |
| `ROOT-041` | 「あくまで derived_from は参照なので。」 | Issue #14 コメント 2（2026-09-05） |
| `ROOT-042` | 辺は層をまたぐ参照だけ | Issue #14 コメント 4（2026-09-05） |
| `ROOT-043` | 「導入するでいいんじゃないの？何か問題あった？」 | Issue #14 コメント 5（2026-09-05） |
| `ROOT-044` | 基本仕様 / 基本設計 / 詳細設計の定義 | Issue #14 コメント 6（2026-09-05） |
| `ROOT-045` | 6 段を既定の規定として採用 | Issue #14 コメント 8（2026-09-05） |
| `ROOT-046` | Python 足場は正としないために消す | Issue #14 コメント 10（2026-09-06） |
| `ROOT-047` | **前提 P**「`specification.json` は vtest 自身の `.verify/doc/` の文書モデルそのものである。」 | Issue #14 コメント 11（2026-09-06） |

`heading` は既存 `ROOT-027`〜`038` が F 番号を使うのに合わせ、#14 にはコメント番号（`C1`〜`C10`）を当てた。**この命名は #14 に根拠が無い。**

**`ROOT-047` の出典**: 第2次適用で Owner 裁定コメント <https://github.com/YmSaki/SpecTracer/issues/14#issuecomment-5556643752> へ付け替えた（`heading` は `C11`）。第1次適用では Issue 上に該当コメントを確認できず `PM セッション 2026-09-06` と書いていたが、**現在は Issue の Owner 裁定が出典であり、逐語転記の残作業は無い。**

`ROOT-037`（F11）は Owner 自身の裁定なので書き換えていない。改定の事実を観測可能にする方法は §1.1 のとおり **UNRESOLVED** のままで、`REQ-S005` / `REQ-S045` から `ROOT-037` への節の辺 2 本もそのまま残した。

## A.2 新 id を得た書き換え（20 件）と、退役した id（26 件）

| 旧 id | 新 id | 新しい文 |
|---|---|---|
| `REQ-025` | `REQ-333` | 上流文書は層ごとのトップレベル配列（`root` / `request` / `require` / `spec` / `detailed_spec` / `basic_design` / `design`）に分け、各層は「文書 > 節 > 小節 > 文」の木のノードとして表現する。 |
| `REQ-053` | `REQ-334` | 文書層では、各ノードの `derives_from` 参照先が存在することを要求する。 |
| `SPEC-421` | `SPEC-462` | 上流文書は層ごとのトップレベル配列に分け、層ごとに id 接頭辞（`ROOT-` / `R-` / `REQ-`・`P-` / `SPEC-` / `DS-` / `BD-` / `DES-`）を持つ。 |
| `SPEC-445` | `SPEC-463` | 旧モデルの SPEC → REQ → VO → Test 構造は、7 層の木 → VO → Test へ再導出する。 |
| `DS-044` | `DS-1590` | 上流文書のノードは層ごとのトップレベル配列（…）に分かれる。 |
| `DS-046` | `DS-1591` | 上流文書のノードは層ごとに id 接頭辞（…）を持つ。 |
| `DS-378` | `DS-1592` | 節ノードは必須 field `title` を持ち、文ノードは `title` を持たない。 |
| `DS-379` | `DS-1593` | 文ノードの `derives_from` は必須 field で 0 件を許容し、節ノードの `derives_from` は任意 field である。 |
| `DS-380`、`DS-383` | `DS-1594` | ノードの `derives_from` は上流ノード id の並びであり、参照先の該当箇所を指す `anchor` field を entry ごとに持たない。 |
| `DS-381`、`DS-382` | `DS-1595` | ノードの `derives_from` は、導出理由を記す `note` field を entry ごとに持たない。 |
| `DS-1325` | `DS-1596` | ノードの `derives_from` は同一の上流ノード id を重複して保持しない。 |
| `BD-163` | `BD-312` | 上流文書は層ごとのトップレベル配列に分けたノードで表現する。 |
| `BD-199` | `BD-313` | 検証グラフでは、ノード間・VO → ノードの関係を `derives_from` の一種で表現する（§19）。 |
| `DES-001` | `DES-567` | ノードとは、ソースコードより上流に位置する成果物を、層ごとのトップレベル配列に分けて表す「文書 > 節 > 小節 > 文」の木の要素である。 |
| `DES-002`、`DES-023` | `DES-568` | ノードは、文ノードが `id` / `statement` / `derives_from` / `source` を、節ノードが `id` / `title` / `source` を、根ノードが `id` / `statement` / `source` を必須 field として持つ。 |
| `DES-010` | `DES-569` | 上流文書は層ごとのトップレベル配列に分けたノードで表現する。 |
| `DES-003`、`DES-025` | `DES-570` | 各 `derives_from` リンクは説明文・導出理由を保持せず、上流ノードの id だけを持つ（§3.2）。 |
| `DES-115` | `DES-571` | `anchor` は `derives_from` の entry field としても Test metadata としても存在しない（§4.1）。 |
| `DES-091`、`DES-116` | `DES-572` | ノードの subject hash は domain `vtest:document-subject:v1` を用い、文ノードでは当該ノードの `id` と `statement` を、節ノードでは子ノードの subject hash を束縛し、`description` を束縛しない。 |
| `DES-114` | `DES-573` | ノードは登録時の内容ハッシュを field として保持せず、subject hash を §1.3 に従って計算する。 |
| `DES-113` | **なし**（`new_id: null`） | 削除。`path` field は新しいノード形に存在せず、既決材料に後継の field が無い（`source` は schema 自身が移行期間限定と宣言）。 |

`DES-113` の削除は、同ノードが `description` に持っていた **document レコードの YAML 例**（`path` / `content_hash` / `derives_from` の `anchor`・`note` を含む旧レコード形）も一緒に落とす。旧レコード形の例は全体が falsified なので、置換案を書かずに落とした。

**新ノードの `source` と `cites` は旧ノードから継承した。** 転記元の位置を残すためだけの継承であり、指す行の本文は退役した旧規範である（例: `REQ-333` の `source.lines` は要件定義 L118 を指すが、その行にあるのは旧 `REQ-025` の文）。**置換文は元 md のどこにも存在しない。** schema は `source` を「元 md のどこから写したか」と定義しているので、書き換え後のノードでは「写した」が成立しない。`source` は移行期間限定の field であり、辺を辿って元 md を読む者が旧規範を正と誤読しないよう、ここに開示する。

退役 id を `derives_from` で指していた生きたノードは、新 id へ張り替えた（15 本）。
`SPEC-033`、`DS-004`、`DS-026`、`DS-029`、`DS-047` → `REQ-333`。
`SPEC-288`、`DS-017`、`DS-101`、`DS-102`、`DS-220`、`DS-478`、`DS-975`、`DS-976` → `REQ-334`。
`DS-1147` → `SPEC-462`・`SPEC-463`。

`relations/retired-ids.json` の追加は 2 回合わせて 49 件で、既存項目と同じ key（`old_id` / `new_id`）に `reason` を加えた形にした。

### A.2.1 第2次適用: `W-SCAN-104` の再束縛（新 id 23 件）

導出 3（§A.4）により、§A.5.4 で据置としていた 27 件のうち 23 件を書き換えた。`path` の実ファイル・登録 content_hash・再登録・参照先 source といった、成立しなくなった機構を名指しする文がこの 23 件である。

| 旧 id | 新 id | 新しい文 |
|---|---|---|
| `REQ-234` | `REQ-335` | 取り込まれた上流成果物は §3.2 のノードとして登録され、規範内容から計算される subject hash と `derives_from` を持つ。 |
| `DS-105` | `DS-1597` | 文書層では、VO レコードの `derives_from` に記録された束縛ハッシュが当該 document ノードの現在の subject hash と一致する。 |
| `DS-346` | `DS-1598` | 束縛ハッシュと現在の subject hash が不一致なら、その束縛は現在有効な値として成立せず、`chain_integrity` の非 `PASS`（`MISMATCH`、診断 `STALE`）とする（§11.4）。 |
| `DS-387` | `DS-1599` | `description` だけの変更は subject hash を変化させないため、当該ノードを上流依存closureに含む判断記録・承認は失効しない（§3.5・§8.5）。 |
| `DS-389` | `DS-1600` | 束縛ハッシュが現在の subject hash と一致しなくなった場合は `chain_integrity` の `MISMATCH`（診断 `STALE`）とする（§11.4）。 |
| `DS-457` | `DS-1601` | document dependency は document subject hash を使用するため、document ノードの規範内容（`id` と `statement`）の変更で承認が失効する（§1.3）。 |
| `DS-800` | `DS-1602` | 文書層は、束縛ハッシュが現在の subject hash と一致すること（不一致は診断 `STALE`）を評価する。 |
| `DS-860` | `DS-1603` | スキャン時に束縛ハッシュと現在の subject hash を比較し、不一致なら `W-SCAN-104` を出す。 |
| `DS-861` | `DS-1604` | 当該 document ノードを `derives_from` で参照する VO の鎖は、束縛ハッシュの不一致として `chain_integrity = MISMATCH`（診断 `STALE`）となる（§11.1.1）。 |
| `DS-863` | `DS-1605` | document ノードの規範内容が変われば subject hash が変化するため、以前の dependency entry を現在の承認・判断へ流用しない。 |
| `DS-903` | `DS-1606` | `W-SCAN-104` は warning であり、束縛ハッシュと現在の subject hash の不一致である（依存判断・依存 Approval は無効、鎖は chain_integrity STALE）。 |
| `DS-1019` | `DS-1607` | 束縛ハッシュが現在の subject hash と一致しなくなれば `chain_integrity = MISMATCH`（診断 `STALE`）とする。 |
| `DS-1288` | `DS-1608` | 各 VO レコードの束縛ハッシュが現在の subject hash と一致することを要求する（不一致は `W-SCAN-104`）。 |
| `DS-1326` | `DS-1609` | `description` だけを変更した document ノードは subject hash が変化しない。 |
| `DS-1327` | `DS-1610` | 規範内容を変更した document ノードは、当該ノードを上流依存closureに含む承認・判断記録を失効させる。 |
| `DS-1430` | `DS-1611` | document ノードは束縛ハッシュと現在の subject hash の一致も要求し、不一致のノードを STALE とし、依存する判断記録も無効とする。 |
| `DS-1488` | `DS-1612` | document dependency は §1.3 の document subject hash を使用するため、規範内容の変更で承認が失効する。 |
| `BD-288` | `BD-314` | fixture は、束縛ハッシュが現在と一致しない document ノード（`W-SCAN-104`、`chain_integrity = MISMATCH`、診断 `STALE`）を表現できる。 |
| `DES-051` | `DES-574` | 取り込まれた上流成果物は、規範内容から計算される subject hash と `derives_from` を持つ。 |
| `DES-092` | `DES-575` | document subject hash は、層を区別せず、すべてのノードについて §1.3 の同一の規則で計算する（§3.1）。 |
| `DES-158` | `DES-576` | A(X) の条件のうち「各 document が登録 content_hash と実ファイルの一致を満たすこと」を「各 document ノードについて記録された束縛ハッシュが現在の subject hash と一致すること」へ置き換え、他の条件はそのまま。 |
| `DES-163` | `DES-577` | 承認レコード a が A(X) に属するには、`a.dependencies` の各 document ノードについて記録された束縛ハッシュが現在の subject hash と一致しなければならない（§11.4）。 |
| `DES-181` | `DES-578` | `approved` から `draft` への遷移は、上流依存closureの entity 構成またはいずれかの hash が変化する（document ノードの規範内容の変更を含む）ことで起きる（§11.4）。 |

`REQ-234` を `derives_from` で指していた `DS-005`・`DS-230`・`DES-574` は `REQ-335` へ張り替えた。

**第2次適用で REWORD にした 4 件（id 据置）**: `REQ-098`・`DS-081`（状態の割当。列挙の「content_hash 不一致」を「VO が束縛した document ノードのハッシュの不一致」へ）、`DS-313`（照合の決定論。「束縛ハッシュの照合」へ）、`DS-236`（VO の依存 closure。第1次適用で `DS-1480`・`DS-1487`・`DES-147`・`DES-148` を「実効的な上流」へ揃えたのに旧語のまま残っていたので揃えた。**候補一覧の外であり、自分が作った不揃いの後始末である。**）。

**変更不要**: `DS-862`「当該 document subject を dependency に含む判断記録・承認記録も無効となる」は `path` にも登録値にも触れておらず、導出 3 のもとでそのまま真なので触れていない。


### A.2.2 第3次適用: 導出 3 の訂正（新 id 5 件 / 退役 6 件）

§A.4 の導出 3 訂正により、第2次が「VO レコードの束縛ハッシュ」と書いた文をすべて直した。

**記録場所を直した（新 id 5 件。第2次の新 id を同じ日のうちに再度退役させている）**

| 旧 id（第2次） | 新 id | 新しい文 |
|---|---|---|
| `DS-1598`（←`DS-346`） | `DS-1613` | 依存 closure（判断記録・承認の `dependencies`、Evidence の subject hash）が束縛した document ノードのハッシュと現在の subject hash が不一致なら、その束縛は現在有効な値として成立せず、`chain_integrity` の非 `PASS`（`MISMATCH`、診断 `STALE`）とする（§11.4）。 |
| `DS-1600`（←`DS-389`） | `DS-1614` | 依存 closure が束縛した document ノードのハッシュが現在の subject hash と一致しなくなった場合は `chain_integrity` の `MISMATCH`（診断 `STALE`）とする（§11.4）。 |
| `DS-1604`（←`DS-861`） | `DS-1615` | 依存 closure が束縛した document ノードのハッシュが現在の subject hash と一致しない場合、当該 document ノードを参照する鎖は `chain_integrity = MISMATCH`（診断 `STALE`）となる。 |
| `DS-1607`（←`DS-1019`） | `DS-1616` | 依存 closure が束縛した document ノードのハッシュが現在の subject hash と一致しなくなれば `chain_integrity = MISMATCH`（診断 `STALE`）とする。 |
| `DS-1611`（←`DS-1430`） | `DS-1617` | 判断記録の `dependencies` が束縛した document ノードのハッシュと現在の subject hash の一致も要求し、不一致の document ノードを STALE とし、依存する判断記録も無効とする。 |

**後継なしで退役（`new_id: null`、6 件）**

| 退役した id | 理由 | その節で残る文 |
|---|---|---|
| `DS-1603`（←`DS-860`） | スキャン時に読む正典レコードは束縛ハッシュを持たず、この比較は入力を持たない。 | — |
| `DS-1606`（←`DS-903`） | `W-SCAN-104` は入力を失った診断コードであり、意味を与える既決材料が無い。 | — |
| `DS-1597`（←`DS-105`） | 文書層の scan はハッシュを評価しない。 | `DS-104`「文書層では、各 document の derives_from 参照先が存在する。」 |
| `DS-1602`（←`DS-800`） | 同上。 | `DS-799`「文書層は、各 `document` の `derives_from` 参照先が存在すること（E-SCAN-012）を評価する。」 |
| `DS-1608`（←`DS-1288`） | 同上、かつ `W-SCAN-104` の退役。 | `DS-1287`「各 `document` の `derives_from` 参照先が存在することを要求する（不在は E-SCAN-012、`chain_integrity = MISMATCH`）。」 |
| `BD-314`（←`BD-288`） | fixture が表現する対象（`W-SCAN-104` の状態）が退役したため。 | — |

**PM 指示との差**: PM は文書層の評価文（`DS-1602`・`DS-1608` 系）を「ハッシュの節を落とす（規範変更 → 新 id）」としたが、**この 3 文は全体がハッシュの節であり、落とすと何も残らない。** 残る規範（参照先の存在）は同じ節の `DS-104` / `DS-799` / `DS-1287` が既に述べているので、新ノードを作ると `CONVERSION.md` §2「1 statement = 1 規範文」に反する重複になる。**そのため新 id ではなく後継なしの退役にした。** 上の表の右列がその根拠である。

**id 据置で直した 3 件**: `REQ-098`・`DS-081`（状態の割当。選言を「依存 closure が束縛した document ノードのハッシュの不一致」へ）、`DS-313`（照合の決定論）。

**変更不要（訂正後の導出 3 のもとでそのまま真）**: `DS-1599`、`DS-1601`、`DS-1605`、`DS-1610`、`DS-1612`、`DES-576`、`DES-577`、`DES-578`、`DS-862`、`DS-236`。うち `DES-576` と `DES-577` は「承認レコードの `dependencies` に記録された束縛ハッシュ」と書いており、**これが訂正後に正しい記録場所である。**

**残った所見**: `DS-111`「違反時の状態は §4.3 に従う（…リンク切れ・hash不一致は `MISMATCH`）」は候補一覧の外にあり触れていない。指す先の `DS-081` を直したので読みは通るが、`hash不一致` がどの層のハッシュを指すかはこの文だけでは決まらない。
## A.3 id を据え置いた書き換え（REWORD、31 件）

**孤児判定（13 件）** — §4.7 が挙げた 14 件から後述の `DS-572`・`DS-1335` を除いた 12 件に、同じ判定基準を列挙に含む `SPEC-377` を加えた。`LAYERING.md` §1.1「文の実効的な上流 = 自分の辺 ∪ 先祖（節・文書）の辺」に従い、問いを「実効的な上流（自分の辺 ∪ 先祖の辺）を持たないノードが存在するか」へ揃えた。
`REQ-059`、`SPEC-059`、`SPEC-291`、`SPEC-377`、`SPEC-422`、`DS-305`、`DS-390`、`DS-547`、`DS-563`、`DS-781`、`DS-899`、`DS-1020`、`BD-286`。

**端点（文書 → ノード、8 件）**: `REQ-026`、`DS-006`、`DS-025`、`DS-304`、`DS-1018`、`BD-165`、`DES-378`、`DES-380`。

**段の列挙（6 段、3 件）**: `SPEC-042`、`DS-1001`、`BD-166`。

**その他（7 件）**: `SPEC-013`（導出 → 参照）、`SPEC-348`（`anchor` 同伴 → 到達先ノード id）、`DS-314`（参照位置は上流ノードの id）、`DS-1480`、`DS-1487`、`DES-147`、`DES-148`（上流依存 closure に先祖の辺を含める）。

**第4次（`DS-111`、1 件）**: 「違反時の状態は §4.3 に従う（…リンク切れ・hash不一致は `MISMATCH`）」の括弧内は §4.3（`DS-081`）の要約なので、`DS-081` と同じ語「依存 closure が束縛した document ノードのハッシュの不一致」へ揃えた（id 据置）。同型に「hash不一致」「content_hash 不一致」を括弧内要約として運ぶ文が他に無いかを全ノード grep で確かめ、**`DS-111` の 1 件だけ**だった（`SPEC-377` は「承認・判断・Evidence のハッシュ束縛による失効」と既に正しい語で書いており、`DS-214`「違反時の状態は §4.3 に従う。」は要約を持たない）。

**開示（規範の変化）**: 孤児判定の 12 件は判定基準そのものが「親を持たない」から「実効的な上流を持たない」へ変わる。厳密には規範の変化であり、A.2 の規則に従えば新 id を得るべきものである。**PM の指示が REWORD（id 据置）と定めたのでそれに従った。** この緊張は開示事項として残す。

## A.4 適用した導出（既決材料からの演繹であって、Owner の逐語ではない）

1. **辺の名は `derives_from`。** 正典の語彙は基本仕様が持ち、現行仕様の全文は `derives_from` と綴る。置換文もすべて `derives_from` に揃えた。**JSON の field 名（`derived_from`）はこのパスでは改名していない。** 結果として `DES-568`・`DS-1593` のように **JSON の field 名そのものを述べる文**が、schema の実際の key（`derived_from`）と字義的に食い違う。**schema と JSON key の `derives_from` への改名は、後続の機械的変更として残っている。**（§4.4 の UNRESOLVED は、この意味で「綴りを揃える」側へ倒れた）
2. **ノードの subject hash は規範内容だけを束縛する。** 入力は `statement` の本文と当該ノードの `id`。`description` は非規範の背景（`CONVERSION.md` §2）なので入力に含めない。節ノードのハッシュは子ノードのハッシュから導く。詳細設計 §1.3 の domain 分離（`vtest:document-subject:v1`）は維持する。`DES-572` / `DES-573` / `REQ-334` はこの基礎の上に書いた。**`REQ-053` から「content_hash が現物と一致する」節を落としたのもこの帰結である。** ノードの内容ハッシュは当該ノード自身から計算されるので、突き合わせる「現物」が別に存在しない。**この削除は導出 2 の帰結であって Owner の逐語ではない。** 残した「`derives_from` 参照先が存在する」義務は無傷。
3. **`W-SCAN-104` は入力を持たないので退役する（導出 3、第2次で誤り、第3次で訂正）。**

   **第2次で書いた版（誤り）**: 「`W-SCAN-104` は VO レコードの `derives_from` に記録された束縛ハッシュと、当該 document ノードの現在の subject hash の不一致を意味する。」

   **何が発明だったか**: **VO レコード（§3.2）にハッシュ field は無い。** Owner 裁定（`ROOT-041`）で `derives_from` は参照のみであり、ハッシュを載せる場所ではない。「VO レコードに記録された束縛ハッシュ」という記録場所は既決材料のどこにも無く、書き手が作ったものである。

   **訂正後の演繹**: 束縛ハッシュが実際に記録されるのは、既存仕様が置いている 2 か所だけである。**判断記録・承認レコードの `dependencies`**（`DES-577` が述べる形）と、**Evidence の subject hash 閉包**（§1.3・§11.4）。
   スキャン時に読むのは document / VO / Test の正典レコードであり、そのどれも document ノードの束縛ハッシュを持たない。したがって「スキャン時に束縛ハッシュと現在の subject hash を比較する」検査は**入力を持たない**。`W-SCAN-104` は対象を失うので退役させた。
   不一致の検出は**検証時**（§11.4）に、判断記録・承認の `dependencies` と Evidence の subject hash が現在の閉包と一致するかで行う。これは既存の文が既に述べており（`DS-1599`・`DS-1601`・`DS-1605`・`DS-1610`・`DS-1612`・`DES-576`・`DES-577`・`DES-578`・`DS-862`）、変更不要である。
   文書層の scan が `chain_integrity` として評価するのは `derives_from` 参照先の存在だけになる（孤児は `orphan_detection` という別の検査であり、`chain_integrity` には含めない）。

   コード名・状態・診断のうち存続するもの（`chain_integrity = MISMATCH`、診断 `STALE`、依存判断・Approval の無効化）は据置とした。`REQ-098` の選言は「依存 closure が束縛した document ノードのハッシュの不一致」と読み替えて生存し、`REQ-334` との緊張（要求の無い罰）は解消したままである。
4. **`REQ-333` / `REQ-334` に根への辺を足した。** `REQ-333` → `ROOT-039`・`ROOT-040`・`ROOT-045`、`REQ-334` → `ROOT-039`・`ROOT-040`。**提案文書はこの辺を明示していない**（§1 は「`request` 層 5 件に #14 由来の文は無い」とだけ述べる）。層分割と JSON 正本化がこれらの裁定に由来することからの導出である。

## A.5 据置（適用しなかったもの）

### A.5.1 CLI / MCP は #14 が先送りしているので触れていない

#14 本文「MCP の CRUD API…については、このデータモデルの方向性が決まった後に別途検討する」に従い、`vtest doc add / list / show` と MCP の doc CRUD に関わる文は**すべて据置**。
`SPEC-378`、`SPEC-336`、`SPEC-380`、`DS-1003`、`DS-1005`、`DS-1009`、`DS-1012`、`DS-1013`、`DS-1015`、`DS-1017`、`DS-1022`、`DS-1026`、`DS-1051`、`DS-1101`、`DS-1145`、`DS-1194`、`DS-1195`、`DS-1481`、`DS-1524`、`BD-072`、`BD-087`、`BD-203`、`BD-218`、`DES-S084`、`DES-482`。

### A.5.2 保存形式（単一ファイル / 1 ノード 1 ファイル）は UNRESOLVED のまま

本レビューの除外事項であり、PM も未決と定めた。関係する basic_design / design の配置文はすべて据置とし、要件定義の逐語を並べて記録する。

> **`REQ-272`（要件定義 §23 並列 AI 開発への要求、L493）**: 「本システムは、不必要に単一共有台帳へ書き込ませない。」
>
> **`REQ-325`（要件定義 §28 下位仕様へ委譲する設計事項、L600）**: 「並列編集時の物理的保存方式は、下位仕様へ委譲する設計事項である。」

| 据置した id | 現在の文（要旨） | 並べて読むべき要件定義の逐語 |
|---|---|---|
| `SPEC-S071`、`DS-S071`、`BD-S042`、`DES-S036` | 節 title「3.1 document レコード（`.verify/doc/DOC-*.yaml`）」 | 「本システムは、不必要に単一共有台帳へ書き込ませない。」／「並列編集時の物理的保存方式は、下位仕様へ委譲する設計事項である。」 |
| `BD-011` | 宣言層は…`.verify/` 配下の document / VO / Relation レコードからなる | 同上 |
| `BD-019` | document の ID は `DOC-` とし、正典は `.verify/doc/` に置く | 同上 |
| `BD-045` | `.verify/` にテストコード外の正典と事実レコードを保存する | 同上 |
| `BD-047` | `.verify/doc/` は document レコード（正典）を格納する | 同上 |
| `BD-059` | document / VO は 1 エンティティ 1 ファイルとする | 同上 |
| `BD-060` | document / VO のファイル名を ID とする | 同上 |
| `BD-138` | `.verify/doc/` は `DOC-<NAME>.yaml` 形式で総称 document レコードを格納する | 同上 |
| `BD-149` | 文書種別ごとの専用ディレクトリを設けず、上流文書はすべて `doc/` の総称 document レコード 1 種で表現する | 同上 |
| `BD-236` | 「その時点の正典の読み取り」は書込みの原子的公開を前提とする | 同上 |
| `DES-237` | 処理フロー第 6 段は `.verify/` 読み込みであり、vtest-store が全レコードを読み込む | 同上 |
| `DES-474` | 原子的公開の対象は `.verify/` 配下の record・エンティティファイルである | 同上 |
| `DS-784`、`DS-786`、`DS-1517` | 4 検査の評価入力／決定論の条件に `.verify/` 配下の正典ファイル集合を置く | 同上 |

`BD-058`「全員が編集する中央共有台帳を持たない。」も同じ緊張の中にあるが、本書の候補一覧に入っておらず判定していない。

**単一 `specification.json` それ自体が共有台帳である。** `REQ-272` の「不必要に」と `REQ-325` の委譲がこれを許すかは Owner の裁定事項であり、本適用では決めていない。

### A.5.3 7 層に当たらない上流成果物は据置

`SPEC-012`（document の値域に API Schema / Protocol Specification / DB schema を含む）と、同根の `SPEC-441`、`DS-1053`、`DES-149`（「方針」）は変更していない。**7 層のどこにも置き場が無いことは未決のまま残る。**

### A.5.4 内容ハッシュの照合（`W-SCAN-104`）は導出 3 で解けた

第1次適用の時点では、導出 2 が「ノードの subject hash が**何を束縛するか**」しか決めておらず、「**スキャン時に何と突き合わせるか**」が未決だったため 27 件を据置にしていた。
第2次適用で導出 3（§A.4）が突き合わせ先を確定させたので、そのうち **23 件を書き換え（§A.2.1）、3 件を REWORD、1 件を変更不要**とした。**この節に残る据置は無い。**

**ただし第2次の導出 3 は突き合わせ先を誤っていた。** 第3次で訂正し、`W-SCAN-104` は入力を持たないものとして退役、文書層の scan のハッシュ要求も退役させた（§A.2.2・§A.4 の導出 3）。**据置が残らないことは変わらない。**

`REQ-334` が「content_hash が現物と一致する」要求を落とした一方で `REQ-098` が「content_hash 不一致 → `MISMATCH`」を罰として残していた緊張（第1次適用が作った、要求の無い罰）は、`REQ-098` の REWORD で解消した。

**第2次が残した未決は、第3次の訂正で消えた。** 「束縛ハッシュを VO レコードのどの field に記録するか」は、そもそも VO レコードに記録しないので問う必要が無い。記録場所は判断記録・承認の `dependencies`（`DES-577`）と Evidence の subject hash 閉包（§1.3・§11.4）であり、どちらも既存仕様が既に定めている。

### A.5.5 VO レコードの参照形が未決なので据置

VO は #14 の対象外（#14 本文「現時点では implementation / verification / evidence / approval 等は対象に含めず」）。document 側と VO 側が同じ文に同居するものは、doc 側だけ直すと文が割れるので据置。
`DS-1142`、`DS-1143`、`DS-1323`、`DS-1524`、`BD-171`、`DES-467`、および §2.3 の VO 系 UNRESOLVED（`DS-392`、`DS-393`、`DS-398`、`DS-399`、`DS-402`、`DS-1027`、`DS-1030`、`DS-1199`、`DS-1200`、`DS-1218`、`DS-1219`）。

### A.5.6 その他の据置

- `REQ-030`、`BD-017`（「リンクを追加してもスキーマが壊れない」）: 層を増やすときの schema の扱いが未決（§3.2）。
- `DS-571`、`DS-573`、`DS-916`、`DS-1010`、`DS-1011`、`DS-1334`、`DS-1336`、`DES-111`、`BD-285`（`config.yaml` の `doc.roots`）: 「根が 2 つある」問題（§4.8）が未決。
- `DS-095`、`DS-1497`、`SPEC-190`（scope・集約の粒度）、`BD-289`（fixture の再登録）: §2 の判定どおり未決。
- **`DS-572`、`DS-1335`**: 孤児判定の 14 件のうちこの 2 件は `reports/upstream-traceability-audit-2026-09-04.md` §5 の主題 C（孤児判定に第二条件を AND で追加）に対応するノードであり、Owner が別途審査中のため触れていない。**結果として孤児判定の言い換えは層内で不揃いである**（`DS-563`・`DS-899` は直り、同じ §5.6 を扱う `DS-572` は「他のどの document からも `derives_from` で参照されない」の第二条件を残したまま）。

据置の総数は 34 件（F11 改定 5 / 前提 P 依存 19 / REWORD 10）。第1次適用の時点では 60 件で、第2次適用の導出 3 が 26 件を解いた。§2 で UNRESOLVED と判定した 47 件のうち 46 件はそのままで、`DES-181` だけ導出 3 で書けたため `DES-578` になった。

## A.6 検査結果

第1次・第2次・第3次のそれぞれの編集後に機械検査した（`docs/canonical/specification.json`、`docs/canonical/relations/retired-ids.json`）。以下は第3次適用後の結果。

```
total nodes: 3851
1)  id unique:                    OK
1b) id pattern (schema $defs/id): OK
2)  retired ids alive:            OK (0)
3)  derived_from targets exist:   OK
4)  live node -> retired id:      OK (0)
5)  sections with empty items:    OK (0)
5b) retirement chains resolve:    OK (55 live / 13 deleted)
6)  jsonschema validate:          OK
RESULT: PASS
```

`jsonschema` はインストール済みで、`specification.schema.json` に対する検証を実際に走らせて通っている。

検査 5b は第3次で足した。同じ日のうちに 2 回退役した id（`DS-346` → `DS-1598` → `DS-1613` など 11 本）ができたので、**退役の連鎖をたどって生きた id か明示的な削除（`new_id: null`）に必ず行き着くこと**を確かめる。68 本の退役 id のうち 55 本が生きた後継へ、13 本が削除へ解決し、宙に浮いたものと循環は 0 だった。

既存の退役項目のうち `DS-1585`・`DS-1586`・`DES-565` の連鎖は 2026-09-05〜06 の二重付番の後始末（コミット `b0625c6`）で作られたものであり、本適用が作ったものではない。本適用が追加した退役項目は 3 回合わせて 60 件。

JSON の書式（indent 2、key 順、`ensure_ascii=False`、CRLF、末尾改行）は保存した。`git diff --numstat`:

```
300    0  docs/canonical/relations/retired-ids.json
254  301  docs/canonical/specification.json
276    0  reports/doc-model-realignment-2026-09-06.md
```

## A.7 `export/*.md` は正本から乖離した

§4.2 のとおり、`export/` の md 6 本は `specification.json` からの生成物であり、生成に使った `build.py` は 2026-09-06 に Owner の指示で削除されている（`ROOT-046`）。**本適用で JSON を編集したので、`export/*.md` は正本と食い違ったまま残る。再生成する手段は現時点で存在しない。**

これは設計どおりの帰結である（`CONVERSION.md`「同等の機能…は vtest 側で実装する」）。md エクスポートが vtest 側に実装されるまで、`export/*.md` は**古い版として読むしかない**。足場を git 履歴 `a2fa4aa` から復活させて使うことは Owner の指示に反する。
