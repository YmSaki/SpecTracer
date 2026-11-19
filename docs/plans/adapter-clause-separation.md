# 言語を足しても既存条項を書き換えずに済む構造にする（#335）

状態: **ドラフト**。Owner の確認と、独立したレビューを待っている。この文書は工程資料であり、規範ではない。規範は `docs/canonical/specification.json` にある。

## 1. 何を直したか

言語（adapter）を 1 つ足すときに、正本の既存条項を書き換えずに済むようにした。規範の内容は変えていない。変えたのは、条項の置き場所だけである。

Go と JavaScript を足したとき（#278 の最初の 5 コミット）は、追記が 65 件、既存条項の書き換えが 17 件、削除が 0 件だった。書き換え 17 件を読むと、原因は 2 つだった。

| 原因 | 件数 | 次の言語でも起きるか |
|---|---|---|
| 同じ adapter の一覧を、各層で繰り返している | 9 | 起きる |
| 言語共通の規則の中に、言語ごとの構文を括弧で書き込んでいる | 6 | 起きる |
| Rust だけを前提にした文を一般の文に直した | 2 | 起きない |

コードは、言語ごとの処理を adapter の trait の裏に分け、`vtest-ops/src/registry.rs` の 1 か所で切り替えている。正本の文書だけが、同じ分離をしていなかった。

## 2. 変えないと決めたもの

作業の前に、次を変えないと決めた。§6 の機械検査が、これを確かめる。

- 条項の id。消さない、付け替えない、再利用しない。
- 既存条項の `derives_from`。
- `crates/` と `tests/`。この PR では実装とテストに触れない。
- 規範の内容。書き換え前の条項にあったコード字句（バッククォートで囲んだ語）は、書き換え後の条項か、移し先の条項にすべて残す。
- ゲートの結果（fmt、test、clippy、`vtest doctor`）。

## 3. 構造の案を、書き換えの件数で比べた

「言語を 1 つ足す」という変更を想定し、そのとき既存条項を何件書き換えるかで比べた。

| 案 | 内容 | 言語を足すときの書き換え | この PR で動かす量 |
|---|---|---|---|
| A（採用） | 一覧を 1 か所にし、括弧書きを adapter ごとの条項へ移す | 0 件 | 書き換え 21、追記 16、移動 0、削除 0 |
| B | adapter ごとに文書（JSON）を分ける（ROOT-048 が認める形） | 0 件 | A に加えて、adapter を主語にする条項およそ 100 件の移動と、schema・投影・読み手の変更 |
| C | 一覧だけを直す | 6 件以上残る | 書き換え 9 |
| D | 何もしない | 15 件（#278 の実績） | 0 |

A と B は、言語を足すときの書き換えがどちらも 0 件になる。B は移動が大きく、得られる結果が A と同じなので採らなかった。A の代わりに、adapter を主語にする条項は、共通の条項と同じ節に並んだままになる（節の中で言語ごとにまとまっていない）。読む順序の良さより、壊す量の少なさを取った。

ほかの変更（既存の adapter に runner を足す、1 つの言語の静的解析の規則を直す、言語共通の規則を直す）では、どの案でも書き換えの件数は変わらない。

## 4. 入れた変更

### 4.1 adapter の一覧を 1 か所にした

adapter の一覧は、要件定義 §21（`REQ-S048`）の adapter ごとの条項（`REQ-343`、`REQ-344`、`REQ-345`）だけが持つ。`REQ-263` は、その形の条項が対象を定める adapter を「組込 production adapter」と定義する。ほかの層は、この語を参照する。

言語を足すときは、`REQ-S048` に「`<adapter ID>` は…を対象とする」の条項を 1 件足す。

### 4.2 共通の規則から、言語ごとの記述を出した

共通の規則は抽象語（期待する失敗を宣言する構文、2 つの値を照合する assert 相当、Test construct の本体、演算の結果が真偽値になる演算子、ループ、言語固有の依存）だけで書き、「adapter ごとの条項が定める」と書く。言語ごとの中身は、adapter を主語にした条項へ移した。

### 4.3 書き換えた条項（21 件）

| 条項 | 変更前（変わった部分） | 変更後 | 移し先 |
|---|---|---|---|
| `REQ-263` |  `rust-cargo`、`go`、`javascript` | 、本節の「`<adapter ID>` は…を対象とする」の形の条項が対象を定める adapter | — |
| `REQ-264` | `rust-cargo`、`go`、`javascript` | 組込 production adapter | — |
| `SPEC-011` | `rust-cargo`・`go`・`javascript`  | 組込 production adapter（要件定義 §21） | — |
| `SPEC-218` | `rust-cargo`・`go`・`javascript`  | 組込 production adapter（要件定義 §21） | — |
| `SPEC-361` | `rust-cargo`・`go`・`javascript` | 組込 production adapter（要件定義 §21） | — |
| `SPEC-458` | `rust-cargo`・`go`・`javascript` | 組込 production adapter（要件定義 §21） | — |
| `DS-350` | （`rust-cargo` ではマクロ名、`go`・`javascript` では呼出し先の名前）を指定す | を指定し、名前の種類はadapterごとの条項が定め | DS-1988、DS-1989、DS-1990 |
| `DS-1881` | `rust-cargo`・`go`・`javascript`  | 値域を定めたadapter | — |
| `DS-623` | `rust-cargo`では`#[should_panic]` | adapterごとの条項が定める | DS-1991 |
| `DS-624` | `rust-cargo`では`assert_eq!(a, b)`に限 | adapterごとの条項が定め | DS-1992 |
| `DS-625` | `rust-cargo`・`go`では関数本体 | adapterごとの条項が定める | DS-1993、DS-1994 |
| `DS-1936` | `rust-cargo`ではDS-1960、`go`ではDS-1986、`javascript`ではDS-1987の値の変換・比較 | adapterごとの条項が値の変換・比較とする呼出 | — |
| `DS-1948` | 「言語と演算子（`rust-cargo`の`\|\|`・`&&`・`\|`・`&`、`go`の`\|\|`・`&&`）」と、末尾の「`javascript`の`\|\|`・`&&`・`\|`・`&`は、演算の結果が真偽値にならないため対象としない。」 | 「演算子（adapterごとの条項が定める）」。末尾の文は削った | DS-1995、DS-1996、DS-1997 |
| `DS-1949` | 「言語と演算子（DS-1948と同じ）」と、末尾の「`javascript`の`\|\|`・`&&`・`\|`・`&`は対象としない。」 | 「演算子（DS-1948と同じ）」。末尾の文は削った | —（DS-1997 が DS-1949 も対象にする） |
| `DS-1950` | 「（`rust-cargo`ではDS-1951）」「（`rust-cargo`ではDS-1952）」 | どちらも「（adapterごとの条項が定める）」 | —（既存の DS-1951、DS-1952） |
| `DS-1983` | 「（`rust-cargo`ではDS-1951、`go`ではDS-1973、`javascript`ではDS-1974）」「（DS-1947。`go`ではDS-1887、`javascript`ではDS-1897）」「DS-1951・DS-1973・DS-1974が失敗に数えない抜け方」「ループ（`rust-cargo`の`while`・`while let`・`for ... in`、`go`の`for`文、`javascript`の`while`・`do ... while`・`for`・`for ... of`・`for ... in`）」 | 「（adapterごとの条項が定める）」「（DS-1947と、adapterごとの条項が定める式）」「adapterごとの条項が失敗としての発散に数えない抜け方」「ループ（adapterごとの条項が定める）」 | DS-1998、DS-1999、DS-2000（ほかは既存の条項） |
| `DS-641` | `rust-cargo`ではDS-1960、`go`ではDS-1986、`javascript`ではDS-1987 | adapterごとの条項が定める | — |
| `BD-003` |  `rust-cargo`、`go`、`javascript` | 、要件定義 §21 が定める adapter | — |
| `BD-104` |  `rust-cargo`、`go`、`javascript` | 、要件定義 §21 が定める adapter | — |
| `BD-114` |  `syn`、`quote`、`rustc-demangle`、Cargo command、`go` command、Node.js・vitestの起動 | 言語固有の依存（adapterごとの条項が定める） | BD-359、BD-360、BD-361 |
| `BD-258` |  `rust-cargo`、`go`、`javascript`  | 、組込 production adapter（要件定義 §21） | — |

### 4.4 足した条項（16 件）

どれも、既存条項の括弧書きを移したものである。`derives_from` と `source` は移す元の条項と同じにした。

| 条項 | 文 | 移す元 |
|---|---|---|
| `DS-1988` | `rust-cargo` の `scan.assertion_macros` に指定する名前は、マクロ名である。 | `DS-350` |
| `DS-1989` | `go` の `scan.assertion_macros` に指定する名前は、呼出し先の名前である。 | `DS-350` |
| `DS-1990` | `javascript` の `scan.assertion_macros` に指定する名前は、呼出し先の名前である。 | `DS-350` |
| `DS-1991` | `rust-cargo`で、DS-623の期待する失敗を宣言する構文は`#[should_panic]`である。 | `DS-623` |
| `DS-1992` | `rust-cargo`で2つの値を照合するassert相当（DA-004）は、`assert_eq!(a, b)`に限る。 | `DS-624` |
| `DS-1993` | `rust-cargo`で、DA-005が見るTest constructの本体は関数本体である。 | `DS-625` |
| `DS-1994` | `go`で、DA-005が見るTest constructの本体は関数本体である。 | `DS-625` |
| `DS-1995` | `rust-cargo`で、DS-1948・DS-1949の演算の結果が真偽値になる演算子は`\|\|`・`&&`・`\|`・`&`である。 | `DS-1948` |
| `DS-1996` | `go`で、DS-1948・DS-1949の演算の結果が真偽値になる演算子は`\|\|`・`&&`である。 | `DS-1948` |
| `DS-1997` | `javascript`の`\|\|`・`&&`・`\|`・`&`は、演算の結果が真偽値にならないため、DS-1948・DS-1949の対象としない。 | `DS-1948` |
| `DS-1998` | `rust-cargo`で、DS-1983のループは`while`・`while let`・`for ... in`である。 | `DS-1983` |
| `DS-1999` | `go`で、DS-1983のループは`for`文である。 | `DS-1983` |
| `DS-2000` | `javascript`で、DS-1983のループは`while`・`do ... while`・`for`・`for ... of`・`for ... in`である。 | `DS-1983` |
| `BD-359` | `rust-cargo` の言語固有の依存は、`syn`、`quote`、`rustc-demangle`、Cargo command である。 | `BD-114` |
| `BD-360` | `go` の言語固有の依存は、`go` command である。 | `BD-114` |
| `BD-361` | `javascript` の言語固有の依存は、Node.js・vitestの起動である。 | `BD-114` |

## 5. 用語 × adapter の表（未定義のマス）

共通の規則が「adapter ごとの条項が定める」とする事項について、各 adapter に条項があるかを表にした。空きのマスが未定義である。

**この PR は空きを埋めていない。** 埋めると規範の内容が変わるので、Owner の裁定を受けてから別の PR にする。

条項の有無は、文を機械的に検索して拾い、読んで確かめた。「なし」は「見つからなかった」であり、無いことの証明ではない。

| 事項 | `rust-cargo` | `go` | `javascript` |
|---|---|---|---|
| 対象（要件定義 §21） | REQ-343 | REQ-344 | REQ-345 |
| `assertion_macros` の名前の種類 | DS-1988 | DS-1989 | DS-1990 |
| `run.coverage` の値域 | DS-1878 | DS-1879 | DS-1934（`off` だけ） |
| assert 相当の構文 | DS-616〜DS-620 | DS-1883〜DS-1886 | DS-1890〜DS-1894 |
| 期待する失敗を宣言する構文 | DS-1991 | **なし** | **なし** |
| 2 つの値を照合する assert 相当 | DS-1992 | DS-1889 | DS-1898 |
| Test construct の本体 | DS-1993 | DS-1994 | DS-1899 |
| 値の変換として数える呼出 | DS-1960、DS-1961 | DS-1986 | DS-1987 |
| 演算の結果が真偽値になる演算子 | DS-1995 | DS-1996 | DS-1997（対象なし） |
| 失敗としての発散に数える構文 | DS-1951 | DS-1973 | DS-1974 |
| 決める位置に加える式 | **なし**（DS-1947 の本文が Rust の構文で書いてある） | DS-1887 | DS-1897 |
| 反駁不能なパターン | DS-1952 | **なし** | **なし** |
| ループ | DS-1998 | DS-1999 | DS-2000 |
| 数える位置 | DS-1962 | DS-1970 | DS-1971 |
| 必ず追跡する範囲（下限） | DS-1937 以降 | DS-1968 | DS-1969 |
| 「構文から確定できる」の範囲 | DS-1967 | DS-1979 | DS-1980 |
| 言語固有の依存 | BD-359 | BD-360 | BD-361 |
| annotation の文法と locator | 詳細仕様 §4.2、§5.5、§6.2 | **なし**（BD-357 が所有を述べるだけ） | **なし**（BD-358 が所有を述べるだけ） |
| runner | 詳細仕様 §9.3 ほか | 詳細仕様 §9.5 | **なし**（SPEC-489 が vitest 以外を範囲外とするだけ） |
| coverage の計測 | DS-1925 | DS-1926 | **なし**（DS-1934 により `off` だけ） |
| Execution State の manifest | DES-210 | DES-623 以降 | **なし** |
| 実行されなかった Test の報告 | DS-747 の括弧書き | DES-649 | **なし** |
| Structured Test Operation と Form | DS-1232 ほか | 範囲外（SPEC-490） | 範囲外（SPEC-490） |

### Owner の裁定を求める点

1. **期待する失敗を宣言する構文が `go` と `javascript` に無い。** いまの文は「その構文がないこと」を FAIL 条件にするので、条項が無い adapter では「無い」と読むしかない。`go` で panic を期待する Test が FAIL になる件（#307）は、このマスの空きである。
2. **反駁不能なパターンが `go` と `javascript` に無い。** `javascript` の分割代入や `go` の `switch` の `default` をどう扱うかが決まっていない。
3. **条項が無いときの既定が無い。** 「adapter の条項が定めを置かない事項は、その adapter に当該の構文が無いものとして共通の条項を適用する」という既定を 1 件置けば、上の 1 と 2 を含めて未定義が無くなる。いまの文の唯一の読み方をそのまま書いた形になるが、規範の文が 1 件増えるので、この PR には入れていない。
4. **`javascript` の runner、coverage、Execution State、annotation の文法が無い。** R-6 が v0.1 に求める範囲であり、実装の前に仕様が要る（#278 が残した前提）。
5. **DS-1947 は言語共通の条項だが、本文が Rust の構文（`if let` / `let ... else`、腕、ガード）で書いてある。** `go` と `javascript` は自分の条項（DS-1887、DS-1897）で読み替えている。共通の文を抽象語に直すかどうか。

## 6. 機械検査

合格条件を、変更前（`develop`）と変更後の正本を比べて確かめた。

| 検査 | 結果 |
|---|---|
| 消えた id | 0 件 |
| 文が変わった条項 | 21 件（§4.3 のとおり） |
| 足した条項 | 16 件（§4.4 のとおり） |
| 文以外だけが変わった既存条項 | 0 件 |
| 既存条項の `derives_from` の変化 | 0 件 |
| 書き換えた条項のうち、adapter を名指しするもの | 0 件 |
| 書き換え前のコード字句で、移し先に無いもの | 0 件 |
| 新しく現れたコード字句 | 2 件（`<adapter ID>`、`scan.assertion_macros`。どちらも条項を指すための語） |
| `.verify/doc/` の投影と正本の不一致 | 0 件 |
| adapter を 2 つ以上名指しする条項 | 18 件 → 2 件 |

残る 2 件（`DS-1984`、`SPEC-490`）は、2 つの adapter を主語にした条項である。新しい adapter は自分の条項を足せばよく、この 2 件を書き換える必要はない。

`.github/scripts/spec-adapter-coupling.py` は、最後の行の検査を単独で実行する。adapter の一覧は `REQ-S048` から読むので、adapter を足しても script を直す必要はない。変更前の正本では 18 件を報告して終了コード 1、変更後は 0 になる。CI にはまだ組み込んでいない（§8）。

## 7. この PR で直していないもの

- **例として adapter を 1 つだけ挙げる条項。** `DS-216`、`DS-747`、`DS-1269`、`DES-088`、`SPEC-235`。「`rust-cargo` では…等」の形で、言語を足しても書き換えは要らない。
- **`rust-cargo` の旧形式との互換を定める条項。** `DS-967`、`DES-199`〜`DES-203` など。言語を足しても変わらない。
- **adapter を名指ししないが、特定の言語の構文で書いてある共通の条項。** `DS-627`（`#[ignore]`）、`DS-1947`（§5 の 5）、`DS-1983` の括弧書き（`goto`、`range`、`for ... of`）。抽象語に直すと読み方が変わりうるので、裁定を受けてから扱う。
- **`REQ-345` と `SPEC-489` が vitest を名指しする。** 既存の adapter に runner を足すと、この 2 件は書き換えになる。
- **VO の claim。** 書き換えた条項から導出する VO は 10 件ある。うち 4 件（`VO-ORACLE-DA-001-LITERAL-FIXED-LOGIC-IS-CONSTANT`、`VO-ORACLE-DA-003-LITERAL-ABSORBED-OPERAND-NOT-FLOWING`、`VO-ORACLE-DIVERGENCE-AFTER-EXIT`、`VO-ORACLE-DIVERGENCE-CONSTANCY-BY-DECIDING-POSITION`）の claim は、書き換え前の条項の文と同じである。VO は承認の対象なので、この PR では変えていない。
- **`docs/canonical/export/`。** この PR より前から正本より遅れている（`require.md` の `REQ-263` は #278 より前の文のまま）。生成に使う道具がリポジトリに無いので、再生成していない。
- **コード。** core の製品コードで adapter を名指しするのは 2 か所（`vtest-verify/src/lib.rs` の `adapter == "rust-cargo"`、`vtest-store/src/lib.rs` の `rust-cargo` と `llvm-cov` の直書き）。前者は `DES-203` が定める互換の読み方。後者は `DS-351`（値域は adapter ごと）とずれており、別の Issue にする。

## 8. このあと

1. この PR のレビューと merge。
2. §5 の裁定。裁定ごとに `spec/*` の PR を 1 本ずつ出す。
3. `spec-adapter-coupling.py` を CI の `spec-guard` に組み込む（`.github/workflows/ci.yml` を変えるので別の PR）。
4. 言語を足す PR の合格条件に「既存条項の書き換え 0 件」を加える。
