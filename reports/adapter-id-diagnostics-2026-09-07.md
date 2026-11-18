# adapter ID の診断コード — DS-352 と DS-1652 の矛盾（Issue #24）逐語・判定・適用（2026-09-07）

## 0. 枠

**対象**: `docs/canonical/specification.json` のうち、`config.yaml` の `adapters` と adapter registry の欠陥に診断コードを割り当てる条文。

**穴（Issue #24）**: 同じ detailed_spec 層で、未知 adapter ID のコードが 2 つに分裂している。

> `DS-352`（§2.2）: adapter IDの重複、同一adapter内のroot重複、未知adapter、無効なadapter設定はusage error（E-CONFIG-001）とする。
>
> `DS-1652`（§17.1 診断コード表、括弧内）: …（未知・重複adapter IDはE-ADAPTER-001）。

**軸（調査中に変更しない）**:

1. 逐語。要約・報告書・テスト名からは断定しない。
2. 層の権威。上位層（root / request / require / spec）の明文があればそれに従い、無ければ導出であることを明記する。
3. 目的テスト（Issue #11 F1）。規則の可否は「その規則で偽のリンク・偽の PASS が通るか、鎖の切れ目が隠れるか」。
4. 行動テスト（`ROOT-033` F7）。区別の存在資格は「受け取った者（人間／エージェント）の行動が変わるか」。
5. 明文と導出を混ぜない。

**除外**: 実装コードとテスト（PR #21 の作業）。`docs/canonical/export/*.md`（生成物、JSON との乖離は既知）。E-ADAPTER-002 / 003 / 004、W-ADAPTER-101 / 102。

**基準 HEAD**: `d68b84f`。

---

## 1. 逐語

### 1.1 上位層（root / request / require / spec）— 診断コードの割り当て規範は無い

診断コードの**割り当て**（どの事実にどのコードを付けるか）を定める条文は上位 4 層に 1 件も無い。上位層が持つのは委譲と、コードが乗る枠だけである。

| id | 層 | 逐語 |
|---|---|---|
| `REQ-107` | require | 内部エラー・入力不正は検証状態と別系統（終了コード）で表現し、その体系は下位仕様で定める。 |
| `REQ-320` | require | 終了コード体系（検証状態と内部エラーの分離）は、下位仕様へ委譲する設計事項である。 |
| `REQ-261` | require | core verifier を変更せずに別 adapter を登録できる境界を要求する。 |
| `REQ-263` | require | 組込 production adapter は `rust-cargo` とし、Rust・Rust function unit test・小規模な integration test を対象とする。 |
| `REQ-264` | require | `rust-cargo` 以外の production language adapter は v0.1 の提供範囲に含めない。 |
| `REQ-265` | require | adapter が未登録、能力不足、または解析不能の場合、検証結果を推測で `PASS` へ昇格してはならない。 |
| `SPEC-011` | spec | `rust-cargo` 以外の production language adapter はv0.1の提供範囲に含めない。 |
| `SPEC-243` | spec | 詳細設計は、終了コード体系（検証状態と内部エラーの分離。§26.1）を決定する。 |
| `SPEC-354` | spec | 診断コードは§5.4のスキャン診断に加えて定義する。 |
| `SPEC-356` | spec | 終了コードは診断severityだけでなく操作段階で決める。 |
| `SPEC-419` | spec | adapter所有のmetadata宣言、ID、target、VO参照、record schema、Relationの違反を対応診断codeで検出する。 |

`ROOT-033`（F7、Owner 決定）の逐語のうち本件に効く 1 行:

> 存在資格の基準: **受け取った者(人間/エージェント)の行動が変わるか**。意味の違いは資格にならない。

**したがって上位層は沈黙である。** 判定は目的（F1）と `ROOT-033` の基準からの導出になる。

### 1.2 詳細仕様（detailed_spec）— 矛盾する 2 件

| id | 節 | 逐語（適用前） |
|---|---|---|
| `DS-352` | 本冊 §2.2 `config.yaml` | adapter IDの重複、同一adapter内のroot重複、未知adapter、無効なadapter設定はusage error（E-CONFIG-001）とする。 |
| `DS-1652` | 本冊 §17.1 診断コード表 | `E-CONFIG-001`はerrorであり、config version、`verify.full_scope`（固定4検査）、`gates`（名前重複、`require` / `require.verification`欠落、`require.verification`が5状態語彙外、`require.approvals`の不正・未解決ロール）、config field型または登録adapterが検証する設定値が現在のconfig invariantに違反することである（未知・重複adapter IDはE-ADAPTER-001）。 |
| `DS-921` | 本冊 §17.1 診断コード表 | `E-ADAPTER-001`はerrorであり、adapterが未登録、重複、またはregistryの宣言と実装が不一致であることである。 |

### 1.3 詳細仕様 — 「重複」が 2 つの別事実であることを示す条文

| id | 節 | 逐語 |
|---|---|---|
| `DS-036` | 本冊 §2.4 adapter 設定と wire 互換 | adapter IDは**設定内で**一意でなければならない。 |
| `DS-037` | 同上 | 同一adapter内のroot重複も拒否する。 |
| `DS-038` | 同上 | 異なるadapterが同じrootを走査することは許可する。 |
| `DS-039` | 同上 | 未知のadapterやadapter固有設定の検証失敗は操作エラーとする。 |
| `DS-040` | 同上 | 未知のadapterやadapter固有設定の検証失敗時、利用可能な言語や能力を推測補完しない。 |
| `DS-1631` | 基本仕様 §2.1 由来 | **registryの**重複ID、未登録adapterは操作エラーとする。 |
| `DS-1632` | 同上 | registryの重複ID、未登録adapterは空のscanとして成功扱いしない。 |

`DS-036` の「設定内で」と `DS-1631` の「registryの」は、**同じ「重複」という語で別の事実を指している**。前者は 1 つの `config.yaml` に同じ adapter ID が 2 回現れること、後者は registry に同じ ID の adapter が 2 件登録されること。

### 1.4 詳細仕様 — 同型の割り当て先例（判定の決め手）

| id | 逐語 |
|---|---|
| `DS-533` | Form `kind` はbuilt-inと `.verify/forms/` を統合したrepository全体で一意である。 |
| `DS-534` | **重複kindまたは対応の不一致はE-ADAPTER-001、未知kindはE-OP-001とし**、coreが名前からRust adapterを推測しない。 |
| `DS-918` | `E-OP-001`はerrorであり、Structured Operationの入力検証失敗（候補提示を伴う。§6.3）である。 |

### 1.5 詳細仕様 / 詳細設計 — 両コードが同じ severity・同じ終了コードであること

| id | 層 | 逐語 |
|---|---|---|
| `DS-929` | detailed_spec | 終了コード`2`は、操作拒否（E-OP-* / E-ADAPTER-* / E-APPROVAL-* / E-CONFIG-*、引数不正、adapter前提・capability・実行失敗、スキーマ違反の提出など。検証結果は生成しない）であることを意味する。 |
| `DS-935` | detailed_spec | `vtest scan` / `vtest doctor`では、registry・config・adapter契約の検証またはadapter呼出しがE-ADAPTER-* / E-CONFIG-*で拒否された場合は2とする。 |
| `DS-998` | detailed_spec | `vtest scan` は registry・config・adapter契約の検証または adapter 呼出しが E-ADAPTER-* / E-CONFIG-* で拒否された場合は終了コード2とし、scan結果を生成しない。 |

### 1.6 基本設計 / 詳細設計 — registry と config は別の検証対象

| id | 層 | 逐語 |
|---|---|---|
| `BD-155` | basic_design | adapter固有設定の検証は登録adapterへ委譲する。 |
| `BD-007` | basic_design | CLI・MCP・検証coreはadapter registryを介して能力を選択する。 |
| `BD-116` | basic_design | 組込Rust formの内容と配置は `vtest-adapter-rust` が所有する。 |
| `DES-230` | design | 処理フロー第1段は、**registryとconfigの検証**であり、adapter ID、capability宣言、config namespace、rootを検証する。 |
| `DES-351` | design | **registryは**宣言と実装の不一致および重複IDを拒否する。 |
| `DES-489` | design | registry は built-in と user-defined Form を統合し、同じ kind の重複、schema の adapter と registry owner の不一致、未知 adapter、Structured Test capability 欠落を拒否する。 |
| `DS-1230` | detailed_spec | フォーム、監査、実行の入力に含まれる adapter namespace は opaque 値として扱い、未登録 adapter や未提供 capability を Rust 用の既定値へ暗黙変換しない。 |
| `BD-204` | basic_design | coreは`TargetRef::Locator.adapter`をregistryで解決する。 |

---

## 2. 分類 — 5 つの事実、原因の所在、受け取った者の行動

Issue #24 が 1 つの問い（「未知 adapter ID はどちらか」）に見えるのは、**5 つの別事実が 2 つの語（「重複」「未知／未登録」）に畳まれているため**である。分けると次になる。

| # | 事実 | 欠陥のある成果物 | v0.1 で受け取った者が取れる行動 | 同じ入力が別原因でもありうるか |
|---|---|---|---|---|
| A | `config.yaml` 内で同じ adapter ID が 2 回現れる（`DS-036`） | `config.yaml` | `config.yaml` から重複行を消す | ない。registry を見ずに判定できる |
| B | 同一 adapter 内で root が重複する（`DS-037`） | `config.yaml` | `config.yaml` の root を直す | ない |
| C | `config.yaml` の `adapters` が指す ID を registry で解決できない（`DS-352` の「未知adapter」） | `config.yaml`（参照）／registry（被参照） | **v0.1 では `config.yaml` を直す以外にない**（§3.2） | ありうる（typo か、build に当該 adapter が無いか） |
| D | 登録 adapter が検証する設定値が不正（`DS-352` の「無効なadapter設定」、検証は `BD-155` で adapter へ委譲） | `config.yaml` | `config.yaml` の当該設定値を直す | ない |
| E | registry に同じ ID の adapter が 2 件登録される／registry の宣言と実装が不一致／`config.yaml` 以外の参照元が指す adapter が未登録（`DS-1631`・`DES-351`・`DS-1230`・`BD-204`） | registry・adapter 実装・レコード・操作入力 | adapter の登録・実装・当該レコードを直す。`config.yaml` を直しても解消しない | ない |

**A・B・D は `config.yaml` の中だけで判定でき、registry を参照しない。** ここを E-ADAPTER-001 にすると、コードが指す成果物（adapter）と直すべき成果物（config）がずれる。`DS-1652` の括弧書きはこのうち A を E-ADAPTER-001 側へ送っていた。

**C だけが両方の成果物にまたがる。** これが Issue #24 の実体である。

---

## 3. 判定

### 3.0 まず、目的テスト（F1）は判別しない — 明示しておく

`DS-929` により E-CONFIG-* と E-ADAPTER-* はどちらも severity `error`・終了コード `2`・検証結果を生成しない。`DS-1632` はどちらの場合も「空の scan として成功扱いしない」を要求する。**どちらのコードを割り当てても偽の PASS は通らず、鎖の切れ目も隠れない。** したがって F1 の嘘テストはこの 2 択を判別しない。判別するのは `ROOT-033` F7 の行動基準である。

### 3.1 A・B・D は E-CONFIG-001。**同層の明文で決着する（導出ではない）**

`DS-352` が §2.2 で、`DS-036` / `DS-037` / `DS-039` が §2.4 で、いずれも `config.yaml` の条文としてこれらを usage error / 操作エラーと定めている。`DS-1652` の括弧書き「重複adapter ID」が A を奪っていたのは、`DS-036` の「**設定内で**」というスコープ語を落として読んだ結果である。registry の重複（`DS-1631`・`DES-351`）とは別の事実なので、両立する。

### 3.2 C（`config.yaml` が指す未知 adapter）も E-CONFIG-001。**これは導出である**

上位層に割り当て規範が無い（§1.1）ため、以下は導出として書く。3 本の独立な線が同じ答えに収束する。

**(1) 同層に同型の先例がある — `DS-534`。** Form `kind` の registry について、詳細仕様層は既にこの形の問いに答えている。

> 重複kindまたは対応の不一致はE-ADAPTER-001、**未知kindはE-OP-001**とし、coreが名前からRust adapterを推測しない。

構造は本件と同一である。「registry 自身が壊れている（重複・宣言と実装の不一致）」は E-ADAPTER-001、「**入力が registry に無いものを指している**」は入力側のコード（`DS-918` により E-OP-001 は「Structured Operation の入力検証失敗。候補提示を伴う」）。**コードは検証中の成果物を名指すのであって、解決先の registry を名指すのではない。** `config.yaml` にとっての入力側コードは E-CONFIG-001 である。

**(2) `ROOT-033` F7 の行動基準。** v0.1 の registry の中身は build 時に確定する（`REQ-263` 組込 production adapter は `rust-cargo`、`REQ-264` / `SPEC-011` それ以外は提供範囲外、`BD-116` 組込 form は `vtest-adapter-rust` が所有）。**v0.1 の利用者が adapter を追加登録する経路は正本に無い。** したがって C を受け取った利用者に取れる行動は `config.yaml` を直すことだけである。E-ADAPTER-001 は「adapter を登録・修正せよ」という、v0.1 に存在しない行動を名指す。「意味の違いは資格にならない」（`ROOT-033`）。

**(3) 観測可能なものだけでコードを決める。** C は原因が 2 通り（config の typo／build に当該 adapter が無い）ありうるが、システムが観測できるのは「`config.yaml` の adapter ID を registry で解決できなかった」の 1 つだけである。観測できない原因でコードを分けることはできない。観測が起きた場所は config 読込み時の config invariant 検査（`DS-361`・`DES-230` 第 1 段）であり、その成果物は `config.yaml` である。

### 3.3 E は E-ADAPTER-001 のまま

registry 自身の重複 ID と宣言・実装の不一致（`DS-1631`・`DES-351`）は E-ADAPTER-001 のままである。「未登録」も、`config.yaml` 以外の経路で判明するものは本コードに残る。`REQ-265`（adapter が未登録でも推測で `PASS` へ昇格しない）と `DS-1631`・`DS-1632` がこの経路を要求している。

**ただし、参照元を列挙してはならない（本書の初版の誤り。§5.2 と自己矛盾していた）。** 正本は「未登録 adapter が registry 参照で判明する」場面に一律のコードを与えていない。実測:

| 参照元 | 条文 | 正本が与えているコード |
|---|---|---|
| `ExecutionDescriptor.adapter` | `DS-745` | **E-ADAPTER-003**（adapter 不一致）。解決失敗のコードは無い |
| `TargetRef::Locator.adapter` | `BD-204` | 無い（「registryで解決する」だけ） |
| Form schema の `adapter` field | `DES-489` | 無い（§5.2 の穴） |
| フォーム・監査・実行の操作入力 | `DS-1230` | 無い（既定値への暗黙変換の禁止だけ） |

**参照元ごとのコードは当該参照元の条文が定めるべきで、`DS-1663` は決めていない。** `DS-1663` の適用範囲は「registry 自身の欠陥」と「`config.yaml` を経由しない未登録の判明」に限る。

### 3.4 開示 — これは誤記の修正ではなく、上書きである

`DS-1652` の括弧書きには**筋の通った読みがあった**: 「adapter ID の解決は registry の関心事であり、registry に無い ID を指したのなら adapter 側のコードである」。この読みは §3.2 の 3 本によって退けられるが、退けているのは導出であって、上位層の明文ではない。**タイポの修正として扱ってはいない。** Owner が (2) の前提（v0.1 で利用者は adapter を登録できない）を将来変える場合、C の割り当ては再検討の対象になる。

### 3.5 Owner 裁定は要らないと判断した理由

`AGENTS.md` の規律は、同層の矛盾を上位層で決め、上位が沈黙なら目的から演繹し、**両案が判別せず上位も沈黙なら Owner へ 2 択で出す**ことを求める。本件は §3.2 の 3 本が判別するため、2 択にならない。とくに (1) は「私が持ち込んだ原理」ではなく**同じ層が既に下している同型の割り当て**であり、これを無視して C だけ逆向きに割り当てると、`DS-534` と `DS-1663` が同じ問いに別々の答えを持つことになる。

---

## 4. 適用

コミット 2 本（第 1 コミット `604f2ee` と、その description の誤りを撤回する第 2 コミット。§4.2）。対象は `docs/canonical/specification.json`、`docs/canonical/relations/retired-ids.json`、本書。

| 処置 | id | 変更後の statement | 理由 |
|---|---|---|---|
| **新 id** | `DS-1652` → `DS-1662` | `E-CONFIG-001`はerrorであり、config version、`verify.full_scope`（固定4検査）、`gates`（名前重複、`require` / `require.verification`欠落、`require.verification`が5状態語彙外、`require.approvals`の不正・未解決ロール）、`adapters`（設定内のadapter ID重複、同一adapter内のroot重複、未知adapter）、config field型または登録adapterが検証する設定値が現在のconfig invariantに違反することである（registryのadapter ID重複およびregistryの宣言と実装の不一致はE-ADAPTER-001）。 | 括弧書きが `DS-352` / `DS-036` / `DS-037` と矛盾していた。`adapters` の 3 件は同層からの転記。除外を registry 自体の欠陥に限った |
| **新 id** | `DS-921` → `DS-1663` | `E-ADAPTER-001`はerrorであり、adapterが未登録、registryのadapter IDが重複、またはregistryの宣言と実装が不一致であることである（`config.yaml` の `adapters` におけるadapter IDの重複・未知adapterはE-CONFIG-001）。 | 「重複」を registry に限定。「未登録」から `config.yaml` の未知 adapter を除いた。前身の括弧書きと同じ形で carve-out を書いた |
| **REWORD（id 維持、第 2 コミット）** | `DS-1663` | statement は不変 | 初版の description が参照元（レコード・操作入力・Form schema）を列挙して E-ADAPTER-001 を主張しており、§5.2 の「Form schema の未知 adapter にコードが無い」と自己矛盾していた。列挙を撤回し「参照元ごとのコードは当該参照元の条文が定める」へ改めた。`DS-1599` により subject hash は動かない |
| **REWORD（id 維持）** | `DS-352` | statement は 1 文字も変えていない | 判定により `DS-352` は正しい。変更は `description` のみで、「重複」「未知adapter」「無効なadapter設定」の指す先を `DS-036` / `DS-1663` / `BD-155` の語で確定した。`DS-1599` により description のみの変更は subject hash を動かさない |

`derives_from`（2 件とも `[]`）・`cites`（無し）・`source` は前身の値をそのまま保持した。**辺を足しても引いてもいない。ノードの追加・削除も無い。**

新 id は正本のノード id と `relations/retired-ids.json` の `old_id` / `new_id` の**両方**の最大値から採った（`DS-` 1661 → 1662・1663）。

### 4.1 帰結（開示）

`DS-1601` / `DS-1610` により、規範内容（`id` と `statement`）を変えた `DS-1662` / `DS-1663` は、前身 `DS-1652` / `DS-921` を上流依存 closure に含む承認・判断記録を失効させる。`DS-352` は `DS-1599` により失効しない。

### 4.2 第 2 コミット — 本書初版と `DS-1663` description の誤りの撤回

**第 1 コミット `604f2ee` は、`DS-1663` の description と本書 §3.3 で、E-ADAPTER-001 が掛かる参照元を「レコード・操作入力・Form schema」と列挙していた。これは正本に根拠が無く、しかも本書 §5.2（Form schema の `adapter` field が指す未知 adapter にコードは無い）と同一コミット内で矛盾していた。** 撤回の実測根拠は §3.3 の表のとおり。

第 2 コミットの変更は 2 箇所で、どちらも規範内容ではない。

- `DS-1663` の description のみ（statement 不変、id 維持、退役台帳への追記なし）。`DS-1599` により subject hash は動かない。
- 本書 §3.3 / §4 / §4.2 / §6。

**判定そのもの（config 側 = E-CONFIG-001、registry 側 = E-ADAPTER-001）は変わっていない。** 変わったのは、E-ADAPTER-001 が `config.yaml` 以外のどの参照元に掛かるかを本書が言えるかどうかで、**言えないというのが正しい。**

---

## 5. 触っていないもの（開示）

### 5.1 本判定と整合しており、変更が不要だったノード

閉包を可視にするため列挙する。いずれも適用後の 2 条文と矛盾しない。

`DS-036`（設定内一意）／`DS-037`（同一 adapter 内 root 重複）／`DS-038`（異 adapter の root 共有は許可）／`DS-039`・`DS-040`（未知 adapter は操作エラー、推測補完しない）／`DS-353`（polyglot の root 共有）／`DS-354`・`DS-375`（未知 namespace を Rust 設定と解釈しない）／`DS-361`（config 読込み時の検査と終了コード 2）／`DS-534`（Form `kind` の割り当て）／`DS-918`（E-OP-001）／`DS-929`（終了コード 2）／`DS-935`・`DS-998`（scan / doctor の終了コード）／`DS-1230`（操作入力の adapter namespace）／`DS-1631`・`DS-1632`（registry の重複 ID・未登録 adapter）／`BD-155`（adapter 固有設定の検証委譲）／`BD-204`（Locator の adapter 解決）／`DES-230`（第 1 段で registry と config を検証）／`DES-351`（registry が重複 ID と宣言・実装の不一致を拒否）／`DES-489`（Form registry の拒否条件）。

### 5.2 軸の外に残る材料違反 1 件（`AGENTS.md` の out-of-scope 開示）

**`DES-489` は「未知 adapter」を registry の拒否条件として列挙するが、コードを与えていない。** 逐語:

> registry は built-in と user-defined Form を統合し、同じ kind の重複、schema の adapter と registry owner の不一致、未知 adapter、Structured Test capability 欠落を拒否する。

同節の詳細仕様 `DS-534` は Form `kind` について 4 件のうち 3 件にコードを与えている（重複 kind・対応の不一致 → E-ADAPTER-001、未知 kind → E-OP-001）が、**`.verify/forms/<kind>.yaml` の `adapter` field が未登録 adapter を指した場合のコードは正本に無い。** 本書の軸（`config.yaml` の adapter ID）の外なので触っていないが、§3.2 (1) の割り当てを Form schema に当てれば「Form schema という入力が指す未知 adapter」であり、入力側のコードになるはずである。**別の穴として立てるべきで、本適用では埋めていない。**

### 5.3 その他

1. **`docs/canonical/export/detailed_spec.md` に `DS-921` の見出しが残る。** 生成物であり、JSON との乖離は既知（`AGENTS.md`「`docs/canonical/export/*.md` は生成された読み物であり、それ自体は権威を持たない」）。触っていない。
2. **`relations/retired-ids.json` の既存 2 件の `reason` 本文が `DS-1652` を参照している**（`canonical holes 2026-09-07 (H-8)` の 2 件）。過去の記録なので書き換えていない。台帳を辿れば `DS-1652` → `DS-1662` に解決できる。
3. **実装側への申し送り。** Issue #24 の報告によれば `crates/vtest-store/src/lib.rs` の `validate_v2_config` はまだどちらのコードも付与していない（本書は当該ブランチのコードを直接検証していない。`feature/v01-canonical-store` の同ファイルに、本矛盾を未解決として記録したコメントがあることだけを確認した）。コードを付与する段（PR6 想定）で本判定を適用する。
4. **`DS-1652` / `DS-921` を `derives_from` で参照するノードは無い**（実測: 正本内で両 id を参照する辺は 0）。したがって id の変更で dangling は生じない。

---

## 6. 機械検査

第 2 コミットの変更は `DS-1663` の description と本書だけなので、数値は第 1 コミットと同一である（下表の「適用後」は両コミットに共通）。

| 検査 | `d68b84f`（適用前） | 適用後 |
|---|---|---|
| ① schema 適合（`jsonschema` 4.26.0、Draft 2020-12、`specification.schema.json`） | OK（エラー 0） | **OK（エラー 0）** |
| ノード総数 | 3,849 | **3,849（不変。追加 0・削除 0）** |
| ② id 一意 | 重複 0 | **重複 0** |
| ③ 退役台帳の `old_id` が正本に生存 | 0 件（台帳 145 件） | **0 件（台帳 147 件）** |
| ④ `derives_from` の参照先が存在しない辺 | 0 | **0** |
| ④ `derives_from` の参照先が同層または下位層 | 0 | **0** |
| ⑤ `root` 以外で実効的上流（自ノード ∪ 祖先の辺）が空のノード | 270 | **270（不変）** |

delta はすべて 0。`DS-1662` / `DS-1663` は前身の辺（どちらも空）と親節 `DS-S127`（`derives_from: ["SPEC-S013", "SPEC-S066"]`）をそのまま引き継ぐため、⑤ は動かない。`DS-352` は statement・`derives_from`・`source` とも不変で、`description` を足しただけである。
