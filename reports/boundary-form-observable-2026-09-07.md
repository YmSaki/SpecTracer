# 実行形態の観測子（DS-1621 / DS-1622） — 逐語・候補・嘘テスト・判定

対象: `docs/canonical/specification.json`（HEAD 807166a、ブランチ `spec/upstream-traceability-audit`）
作成: 2026-09-07

---

## 0. 結論（先に書く）

1. **観測子は候補 (a)「Test 宣言に実行形態を明示する」で決まる。** 目的（Issue #11 F1）からの演繹であり、Owner 裁定を要しない。候補 (b)(c)(d) は §3 の嘘テストで落ちる。判定の分かれ目は「嘘の `PASS` が出るか」ではない（どの候補でも出ない）。**「切れ目が隠れるか」**である。(b) では「target を書き忘れた」と「意図して境界形態にした」が区別できず、受け取った者の取るべき行動が決まらない。

2. **ただし (a) は本作業の編集枠に入らない。** `@vtest.` の test-key 値域は閉じた列挙であり（`DS-489`、未知キーは `E-SCAN-006`）、キーを1つ足すと **7 ノード**（`DS-489` と design 層 6 件）の statement が偽になる。指示は「既存ノードの statement は変えない」であり、退役＋新 id は本作業の枠を超える。**§6 に後継 statement と退役台帳の行を起草した。適用は再スコープの判断を要する。**

3. **適用したのは別の穴を塞ぐ 1 ノードである。** `DS-687`（「targetを持たないTestは本節の合成へ到達しない」）は **`target_binding` の値を定めていなかった**。`DS-680` は「全宣言targetの到達が充足されれば `PASS`」であり、宣言 target が 0 件なら空虚に真になる。ここが唯一の**嘘の `PASS` 経路**であり、観測子の選択とは独立に閉じられる。`DS-1664` を追加した（§5）。

4. **帰結（Owner が先に知るべきこと）**: 確認方法が `SPEC-301` により後続版へ委譲されている以上、**target を持たない Test を1件でも含む repository は v0.1 で完全検証 OK に到達しない**（`REQ-295` / `DS-844` / `DS-1491`）。これは委譲を正直に描画した結果であって、`PASS` を発明する理由にはしない。

---

## 1. 逐語

### 1.1 穴の所在

| id | 層 | statement（逐語） |
|---|---|---|
| `DS-1621` | detailed_spec | E-SCAN-007はerrorであり、必須metadata（core中立: id / covers ≥ 1 / intent、および当該adapterが必須とする追加metadata。\`rust-cargo\` では検証対象をSource Targetとして実現する実行形態について targets ≥ 1）の欠落を意味する。 |
| `DS-1622` | detailed_spec | v0.1の唯一のadapter\`rust-cargo\`では、検証対象をSource Targetとして実現する実行形態のTestが検証対象をSource Targetとして宣言しない場合にE-SCAN-007（\`targets ≥ 1\`欠落）として\`target_binding\`評価の手前で\`chain_integrity\`の\`MISMATCH\`になり、外部契約・境界上の振る舞いを検証する実行形態のTestはこれに当たらない。 |

両ノードは「検証対象をSource Targetとして実現する実行形態」で条件づけているが、**Test 宣言側の何を見てその条件を評価するかを定めるノードが正本に無い。**

### 1.2 上流（委譲の明文）

| id | 層 | statement（逐語） |
|---|---|---|
| `REQ-070` | require | 他の実行形態における確認方法は、§8 条項 3 に従い、当該形態に適した方法として下位仕様で定める。 |
| `REQ-071` | require | 特定の実行形態の確認方法を、別の実行形態の Test へ一律に要求してはならない。 |
| `REQ-132` | require | 特定の実行形態に固有の確認方法を、別の実行形態へ一律に要求してはならない。 |
| `REQ-147` | require | 実装 construct（Source Target）を直接検証する実行形態では、Source Target 宣言をそのまま検証対象の宣言として扱い、同一対象の二重宣言を要求しない。 |
| `REQ-148` | require | 外部契約・境界上の振る舞いを検証する実行形態では、その契約または振る舞いを検証対象とし、内部 Source Target の宣言を Test 成立性の必須条件としない。 |
| `REQ-064` | require | \`target_binding\` の問いは、その Test が検証対象とする振る舞いが実際に生じ、その振る舞いを反映した観測が得られたか、である。 |
| `REQ-135` | require | 確認不能であることだけを根拠として違反を推定してはならず、成立確認済みとして扱ってもならない。 |
| `REQ-295` | require | 完全検証における OK は、宣言鎖全体に対する検査（chain_integrity / orphan_detection）と、scope に含まれる各「宣言 + コード + 証拠」の組に対する検査（target_binding / oracle_presence）がすべて \`PASS\` であり、テストランナーの結果を含む証拠が §6 を満たす場合に限る。 |
| `SPEC-065` | spec | 他の実行形態における確認方法は、当該形態に適した方法として**詳細設計で定める**。 |
| `SPEC-084` | spec | 外部から観測可能な契約・境界上の振る舞いも検証対象にできる。 |
| `SPEC-300` | spec | 本節の到達要件は検証対象をSource Targetとして実現する形態に限定する（\`rust-cargo\`）。 |
| `SPEC-301` | spec | 検証対象をSource Targetとして宣言しない他の実行形態（外部契約・境界上の振る舞い）の確認方法は、特定形態を他形態へ一律要求せず**下位仕様・後続版へ委譲する**。 |
| `BD-183` | basic_design | 検証対象を実装constructとして実現するか、外部から観測可能な契約・境界上の振る舞いとして実現するかは実行形態が定める。 |
| `BD-185` | basic_design | 非sourceの境界形態…の具体的表現・確認方法は特定形態を他形態へ一律要求せず、下位仕様・後続adapter・後続版へ委譲する（本versionでContract-Target類の新schemaは設けない）。 |
| `DES-580` | design | 当該adapterは、検証対象をSource Targetとして実現する実行形態のTestに1件以上のSource Target（\`targets ≥ 1\`）を必須とし、外部契約・境界上の振る舞いを検証する実行形態のTestには内部Source Targetの宣言をTest成立性の必須条件としない。 |

### 1.3 実行形態を示す宣言が無いことの確認（事実）

| id | 層 | statement（逐語） |
|---|---|---|
| `DS-489` | detailed_spec | test-keyの値域は \`id\` / \`covers\` / \`target\` / \`intent\` / \`input\` / \`expect\` / \`kind\` / \`case\` / \`related\` である。 |
| `DS-499` | detailed_spec | 表面1で、\`@vtest.\` で始まるがtest-keyを持たない行はエラーE-SCAN-006とする（打鍵ミスの検出を優先し、警告ではなくエラーとする）。 |
| `DS-661` | detailed_spec | Testがtargetを静的解析の追えない実行境界を越えて到達させる形態は、Testのkind（unit / integration）とは独立に、execution topologyによって決まる。 |
| `DS-1234` | detailed_spec | \`test_kind\` の \`regression\` は Test の意図ラベル（\`@vtest.kind\` の値）であり、廃止された存在理由分類（role / anchor）とは別概念である。 |
| `DS-1236` | detailed_spec | \`kind\` の値に regression を含む Test（\`unit-regression\` 等）も \`kind\` から存在理由分類を導出しない。 |

値域は9キーで閉じており、実行形態を示すキーは無い。`kind` からの導出は `DS-1236` が禁じ、`DS-661` が実行形態と `kind` の独立を述べる。

### 1.4 target_binding 側の空白

| id | 層 | statement（逐語） |
|---|---|---|
| `DS-680` | detailed_spec | \`target_binding\` は、そうでなく全宣言targetの到達が静的到達またはruntime到達で充足されれば \`PASS\` とする。 |
| `DS-687` | detailed_spec | targetを持たないTestは本節の合成へ到達しない。 |
| `SPEC-319` | spec | \`target_binding\` は評価地点をTESTとし、§7.3の合成による。 |
| `DS-840` | detailed_spec | aggregateは、各TESTについて、scopeの検査軸に含まれるtarget_binding / oracle_presenceを評価する（含まれない検査はNO_EVIDENCE、診断NOT_CHECKED）。 |
| `DS-1103` | detailed_spec | \`--fast\` は \`target_binding\` の動的証拠を採らず、検証時 \`NO_EVIDENCE\`／診断 \`NOT_CHECKED\` とする。 |
| `DS-1403` | detailed_spec | 有効なEvidenceについて…当該targetの関数不見当は \`target_binding\` を \`UNKNOWN\` にする。 |

`SPEC-319` は `target_binding` を §7.3 の合成と定め、`DS-687` は target を持たない Test をその合成から外す。**外した先の値を定めるノードが無かった。** `DS-680` の「全宣言target」は 0 件で空虚に真になるため、実装が `DS-687` を読み落とせば `PASS` に落ちる。

---

## 2. 候補

| 候補 | 内容 | 正本の語彙との関係 |
|---|---|---|
| (a) | Test 宣言に実行形態を明示する field / 値を設ける | 概念語（実行形態 / Source Target / 外部契約）は正本にある。英語綴りは新語（§4.3 で導出と明記） |
| (b) | `targets` の有無そのもので判別する | 新語なし |
| (c) | `covers` 先の VO の属性で判別する | VO レコードに該当 field が無い（`DES-117` / `DES-118` / `DES-119`、`DS-391`〜`DS-395`） |
| (d) | `kind` から導出する | `DS-1236` が明文で禁止 |

---

## 3. 嘘テスト

**当てる問い**: 利用者がその宣言を悪用して `E-SCAN-007` を逃れたとき、検証状態がどうなるか。境界形態の Test の `target_binding` が `PASS` になる経路があってはならない。

前提として、以下は `DS-1664`（§5、本作業で適用）により全候補共通で成立する: **target を持たない Test の `target_binding` は `NO_EVIDENCE`（診断 `NOT_CHECKED`）であり、`PASS` にならない。** したがって「嘘の `PASS`」はどの候補でも出ない。分かれるのは次の段である。

### (d) `kind` から導出

`DS-1236`「`kind` から存在理由分類を導出しない」、`DS-661`「実行形態は kind とは独立」に正面から反する。下位層が上位に無い規範を発明する形にもなる。**落ちる。**

### (c) `covers` 先の VO の属性

VO レコードの field は `id` / `parent` / `derives_from` / `coverage_policy` / `combinations` / `dimensions` / `representative_cases` / `status` であり（`DES-117`〜`DES-119`、`DS-391`〜`DS-395`）、実行形態を持つ field は無い。新設すれば VO 側が Test の形態を決めることになり、`BD-183`「実行形態が定める」と主語が食い違う。同一 VO を単体テストと境界テストの両方で検証する構成も表現できなくなる。**落ちる。**

### (b) `targets` の有無そのもの

**まず、この候補には正当な根拠がある。** `REQ-147` は「Source Target 宣言をそのまま検証対象の宣言として扱い」と述べる。source 形態では Source Target 宣言が検証対象の宣言そのものである。したがって「Source Target を宣言していない Test は source 形態ではない」は上流からの演繹として成り立つ。

**嘘テスト**: 内部 construct を検証すべき Test（本体で `parse()` を呼び assert する）から `@vtest.target` を落とす。

- `chain_integrity`: `id` / `covers` / `intent` が揃っていれば **PASS**。`E-SCAN-007` の `targets ≥ 1` 節は前件が空集合になるため発火しない。
- `target_binding`: **`NO_EVIDENCE`（診断 `NOT_CHECKED`）**。`PASS` にならない。
- 総合: **NG**（`DS-844` / `DS-1491`）。

**嘘の `PASS` は通らない。しかし切れ目が隠れる。** この Test の行は、意図して境界形態にした Test の行と**一字一句同じ**になる。受け取った者は「target を書き忘れているから直せ」と「v0.1 では確認方法が未定義だから受け入れるしかない」を区別できない。AI の完了宣言を受け取る瞬間（F1）に、40 件の `NOT_CHECKED` が並んだとき、行動が決まらない。

さらに (b) を採ると `E-SCAN-007` の `targets ≥ 1` 節は空文になるため、`DS-1621` の当該節・`DS-1622`・`DS-1620`・`DS-1624`・`DS-1625`・`DS-1633`・`DS-1634`・`DS-1635`・`BD-315`・`BD-316`・`DES-580`・`DES-581`・`DES-582` の 13 件が空集合を述べるノードになる。**落ちる**（嘘は通らないが、切れ目が隠れる）。

### (a) Test 宣言に実行形態を明示

**嘘テスト**: 同じ Test に、境界形態である旨を明示して宣言する。

- `chain_integrity`: `E-SCAN-007` は発火しない（宣言どおり境界形態なので `targets ≥ 1` を課さない）。**PASS**。
- `target_binding`: **`NO_EVIDENCE`（診断 `NOT_CHECKED`）**。`PASS` にならない。
- 総合: **NG**。

**逃れても検証状態は良くならない。** 明示しなければ `E-SCAN-007` → `chain_integrity` の `MISMATCH`（診断 `MISSING`）、明示すれば `target_binding` の `NO_EVIDENCE`（診断 `NOT_CHECKED`）。どちらも非 `PASS` で総合 NG である。**利得はラベルが変わることだけで、`PASS` へ近づく経路は無い。**

一方、書き忘れと意図の区別は残る。書き忘れは `E-SCAN-007`（error）として現れ、意図は宣言として記録される。**受け取った者の行動が変わる**（`ROOT` F7 の存在資格の基準）。宣言の真偽を裁定するのは vtest ではなく外部である（F4）。vtest は宣言を記録し、状態を見せ、`PASS` へ昇格させない。

**残る。**

### 判定

(b)(c)(d) が落ち、(a) が残る。**両候補で嘘が通らず目的から区別できない場合**にあたらないため、Owner 裁定は要らない。**上流（`REQ-070` / `SPEC-065` / `SPEC-301`）が明示的に委譲した HOW として、detailed_spec で決まる。**

---

## 4. (a) の設計（起草。未適用）

### 4.1 上位層に禁止が無いことの確認

`root` / `request` / `require` / `spec` を「新設しない|設けない|新schema|新スキーマ|scope拡大|スコープ拡大|増やさない|発明しない|追加しない」で走査した（13 件ヒット）。宣言面の新設を禁じるノードは無い。

- `SPEC-006`「要件定義に無い**義務・検査・状態・文書種別**を本書で新設しない」、`SPEC-260`「基本仕様（WHAT）に無い**義務・検査・状態・文書種別・関係型**を発明しない」。実行形態の宣言は任意（省略時は既定値、§4.4）であり、新しい義務ではない。検査・状態・診断ラベル・関係型のいずれも増えない。
- `SPEC-005` / `SPEC-231`「本書は具体構文…の HOW を発明しない」。**構文は下流の領分であるという明文**であり、(a) を支える。
- `R-2` / `ROOT-035`（F9）「v0.1 は順方向のみで出荷、スコープ拡大は所有者の既知の悪癖」。(a) は逆方向トレースも新検査も追加しない。既に v0.1 にある規則（`DS-1621` / `DS-1622` の形態条件）を評価可能にするだけである。
- `REQ-084`「実行形態別の判定規則を設けない」は **`oracle_presence`（§4.4、`REQ-S012`）に係る文**である。(a) は `oracle_presence` に触れない。`target_binding` の形態別扱いは `REQ-070` / `REQ-071` が明示的に想定している。
- `BD-185`「本versionでContract-Target類の新schemaは設けない」は **`basic_design` 層**であり、層順（root > request > require > spec > **detailed_spec** > basic_design > design）では detailed_spec の**下**にある。**下位層は上位層を拘束しない。** 加えて形態マーカは「検証対象が何であるか」を表現しないので Contract-Target 類の schema ではない。上位層に同旨の禁止は無い（上記走査）。

### 4.2 `targets ≥ 1` の上流根拠（`DS-1621` は発明ではない）

`REQ-064`（`target_binding` は**宣言された**検証対象の振る舞いが生じたかを問う）＋ `REQ-147`（source 形態では Source Target 宣言が検証対象の宣言そのもの）から、**source 形態で target を 0 件しか宣言しない Test は検証対象を宣言していない**ことになり、`REQ-064` の問い自体が立たない。`targets ≥ 1` はこの帰結の adapter 側での表現である。形態マーカが足すのは「`REQ-148` を偽のターゲット無しで守る手段」だけであり、検査・状態・診断ラベルは1つも増えない。

### 4.3 語彙（導出。明文ではない）

- キー: `execution-form`。正本の概念語**実行形態**（`REQ-068` / `REQ-070` / `REQ-071` / `REQ-147` / `REQ-148`）の英語綴り。**新語であり導出である。**
- 値: `source-target` / `external-contract`。正本の **Source Target**（`REQ-147`）と**外部契約**（`REQ-148`「外部契約・境界上の振る舞い」）の綴り。**新語であり導出である。**
- **`boundary` を値に使わない理由**: `DS-1232` の組込 Form が `test_kind: [normal, error, boundary, regression]` を持ち、この `boundary` は**境界値**分割を指す。同じ語を実行形態に使うと、`kind` から実行形態を導出したくなる誘引を作り、`DS-1236` の禁止と衝突する。
- キー順の挿入位置: `covers` の直後、`target` の直前。`target` が必須かどうかを支配するキーなので `target` より前に置く。**位置の選択は HOW であり、上流に根拠は無い。**

### 4.4 既定値（導出）

**宣言が無い場合は `source-target` とする。** 根拠は `REQ-135`「確認不能であることだけを根拠として…成立確認済みとして扱ってもならない」。宣言の不在は境界形態であることの証拠ではないので、免除は積極的な宣言によってのみ得られる。副次的に、`DS-1232` / `DS-1241` の組込 Form（いずれも `@vtest.target` を出力する source 形態）と `BD-316` の fixture（形態宣言も target も無い Test → `E-SCAN-007`）は**一字も変えずに現在の意味を保つ**。

### 4.5 `external-contract` かつ `targets ≥ 1` の場合（明示しておく）

形態宣言が支配するのは `E-SCAN-007` の `targets ≥ 1` 要求だけである。target を宣言していれば §7.3 の合成は書かれているとおり適用され、`target_binding` は宣言 target を通常どおり評価する（`DS-687` が合成から外すのは target を**持たない** Test だけ）。`REQ-148` は内部 Source Target の宣言を**禁じて**いない。「必須条件としない」だけである。

### 4.6 subject hash に入れるか（入れない）

形態値は `targets = 0` のときにしか効かず、`targets = 0` ではどの形態でも `target_binding` は `PASS` にならない。`targets` が有るときの形態値は無効である（§4.5）。したがって**形態値が `PASS` や承認の有効性を決める経路は無い**ので、canonical metadata（`DES-080`、hash 入力）へ加える必要は無い。target の追加・削除自体は `targets` が canonical metadata に入っているため既に hash に反映される。

---

## 5. 適用したもの — `DS-1664`

**§7.3（`DS-S098`）に 1 ノードを追加した。既存ノードの statement は1件も変えていない。**

```
id: DS-1664
statement: targetを持たないTestの`target_binding`は`NO_EVIDENCE`（診断`NOT_CHECKED`）とする。
derives_from: REQ-064, REQ-070, REQ-071, REQ-135, REQ-148, SPEC-065, SPEC-301
配置: DS-687 の直後
```

description（要旨）:

- `SPEC-065` が確認方法を詳細設計へ委ね、`SPEC-301` が下位仕様・後続版へ委譲しているため、v0.1 は当該形態の到達確認方法を持たない。
- 診断ラベルが `NOT_CHECKED` である理由: 検査を実施していない状態（`DS-840` の scope 外検査、`DS-1103` の `--fast`）と同型だからである。解析を実施して限界に達した `UNKNOWN`（`DS-1403`）とは別軸である。**新しい状態も新しい診断ラベルも増えない。**
- `DS-687` は合成から外すことだけを定め値を定めていなかった。値が定まらないことを `PASS` の側へ倒さないため、`REQ-135` に従って非 `PASS` の側へ確定する。
- `REQ-295` の完全検証 OK は4検査すべての `PASS` を要するため、target を持たない Test を含む repository は v0.1 で完全検証 OK に到達しない。
- `REQ-071` により、source 形態の確認方法を当該 Test へ一律に要求はしない。

**このノードは観測子の選択と独立に正しい。** (a) を採れば境界形態と宣言された Test の値であり、(b) を採れば target を持たない Test の値であり、現状（`E-SCAN-007` が無条件に発火する実装）では到達しないため無害である。**塞いだのは §1.4 の空白、すなわち唯一の嘘の `PASS` 経路である。**

### 機械検査

| 項目 | 適用前 | 適用後 |
|---|---|---|
| ノード総数 | 3849 | 3850 |
| detailed_spec | 1736 | 1737 |
| schema エラー | 0 | 0 |
| id 重複 | 0 | 0 |
| dangling `derives_from` | 0 | 0 |
| 上位層以外への `derives_from` | 0 | 0 |
| 退役 id の生存 | 0 | 0 |
| root 外の実効的上流が空のノード | 270 | 270 |

`git diff --numstat` は `docs/canonical/specification.json` が `22 0`。削除行ゼロ。

---

## 6. 適用していないもの — 観測子（再スコープを要する）

### 6.1 なぜ枠外か

キーを1つ足すと、キー集合を**網羅的に列挙している** statement が偽になる。1件でも更新を漏らすと正本が自己矛盾する（`DS-489` が9キーと言い、別ノードが10キー目を足し、`DS-499` がその10キー目を `E-SCAN-006` にする）。**現状の穴より悪い。** よって「足すが列挙は直さない」は採れない。

`derives_from` で以下7件を指す生きたノードは**無い**（全 3850 ノードを走査して確認）。したがって退役しても dangling は 0、退役 id の生存も 0 のまま保てる。後継に前身と同じ `derives_from`（いずれも空）を与えれば 270 も不変である。

### 6.2 退役＋後継の起草（挿入は1語のみ。他は一字も変えていない）

| 退役 id | 層 | 後継 statement（起草） |
|---|---|---|
| `DS-489` | detailed_spec | test-keyの値域は \`id\` / \`covers\` / \`execution-form\` / \`target\` / \`intent\` / \`input\` / \`expect\` / \`kind\` / \`case\` / \`related\` である。 |
| `DES-215` | design | \`SourceDiscoveryAdapter\` は、adapter所有のsource declarationを \`id\`、\`covers[]\`、\`execution_form\`、\`targets[]\`、\`intent\`、\`input?\`、\`expect?\`、\`kind?\`、\`cases[]\`、\`related[]\` の論理fieldへ正規化する。 |
| `DES-394` | design | \`rust-cargo\` discoveryの第5段はmetadata宣言抽出であり、doc属性（\`#[doc = "..."]\`）を§4.2の文法でparseする（id / covers / execution-form / target / intent / input / expect / kind / case / related）。 |
| `DES-521` | design | アノテーションは常にキー順（id, covers, execution-form, target, intent, input, expect, kind, case, related）で再生成する。 |
| `DES-523` | design | アノテーションを常にキー順（id, covers, execution-form, target, intent, input, expect, kind, case, related）で再生成し、\`@vtest.\` を含まない自由記述の doc comment 行を元の位置関係を保って温存することにより、Structured Edit を繰り返しても差分が安定する。 |
| `DES-524` | design | アノテーションを常にキー順（id, covers, execution-form, target, intent, input, expect, kind, case, related）で再生成する規則は、Create が挿入する annotation block にも同一に適用する。 |
| `DES-526` | design | アノテーションの再生成キー順（id, covers, execution-form, target, intent, input, expect, kind, case, related）は本冊 §4.2 の test-key（\`id\` / \`covers\` / \`execution-form\` / \`target\` / \`intent\` / \`input\` / \`expect\` / \`kind\` / \`case\` / \`related\`）と一致する。 |

`DES-215` を含める理由: Structured Edit の再生成は論理 field から annotation block を組み立て直し、`@vtest.` を含む行のうちキー順に無いものは温存されない。`DES-522`「`@vtest.` を含まない自由記述の doc comment 行は元の位置関係を保って温存する」が温存するのは `@vtest.` を**含まない**行だけだからである。論理 field に載せないと、Structured Edit を1回通すたびに形態宣言が黙って消える（データ喪失）。**adapter 内部だけで消費して論理 field に載せない案は、この1点で落ちる。**

`DES-080`（canonical metadata＝hash 入力）は**含めない**。理由は §4.6。ただし「形態値の変更が既存 Evidence を失効させない」ことを保守側に倒したいなら `DES-080` も退役対象に加える判断はありうる（その場合 8 件）。**これは選択であり、判断を残す。**

### 6.3 新規ノードの起草

| 新 id | 層 | 節 | statement（起草） |
|---|---|---|---|
| （新規） | detailed_spec | `DS-S080`（§4.2） | test-key \`execution-form\` の値域は \`source-target\` / \`external-contract\` であり、宣言が無いTestは \`source-target\` とする。 |
| （新規） | detailed_spec | `DS-S086`（§5.4）または `DS-S098`（§7.3） | \`rust-cargo\` は \`execution-form\` が \`source-target\` であるTestについて \`targets ≥ 1\` を要求し、\`external-contract\` であるTestには要求しない。 |

`derives_from` はいずれも上位層のみ（`REQ-070` / `REQ-071` / `REQ-147` / `REQ-148` / `SPEC-084` / `SPEC-301` など）。design 層に wire 形（annotation 行としての綴り）を別ノードで足す必要は無い。`DES-394` の後継が parse 対象として、`DES-521` / `DES-523` / `DES-524` / `DES-526` の後継が出力キー順として、すでに wire 形を定めるためである。

2件目は `DS-1621` / `DS-1622` の**言い換えではない**。両ノードが既に課している `targets ≥ 1` を、何を見て課すかで**条件づける**フックである。したがってここで新しい義務は生じない。`rust-cargo` が source 形態の Test に `targets ≥ 1` を課すこと自体は `DS-1621` / `DS-1633` / `DS-1634` / `DS-1635` / `BD-315` / `DES-580` / `DES-582` が既に定めており、`SPEC-006` / `SPEC-260`（要件定義・基本仕様に無い義務を新設しない）に対しては、**新設ではなく既存義務の評価可能化**として立つ。`execution-form` の宣言そのものは任意である（§4.4）。

### 6.4 退役台帳の行（起草）

`reason` は主題ごとに書き分ける（総称文言を流用しない）。

```json
{"old_id": "DS-489",  "new_id": "<新>", "reason": "execution-form observable 2026-09-07 (REQ-070 delegation): test-key value domain extended"}
{"old_id": "DES-215", "new_id": "<新>", "reason": "execution-form observable 2026-09-07: logical field added so Structured Edit round-trips the declaration"}
{"old_id": "DES-394", "new_id": "<新>", "reason": "execution-form observable 2026-09-07: stage-5 parse key list extended"}
{"old_id": "DES-521", "new_id": "<新>", "reason": "execution-form observable 2026-09-07: annotation regeneration key order extended"}
{"old_id": "DES-523", "new_id": "<新>", "reason": "execution-form observable 2026-09-07: annotation regeneration key order extended (diff stability clause)"}
{"old_id": "DES-524", "new_id": "<新>", "reason": "execution-form observable 2026-09-07: annotation regeneration key order extended (Create path)"}
{"old_id": "DES-526", "new_id": "<新>", "reason": "execution-form observable 2026-09-07: regeneration key order / test-key correspondence extended"}
```

判定基準は先例（`reports/audit13-node-disposition-2026-09-06.md` L525）の「その文が真になる入力集合が変わるか」。7件すべて変わるので新 id である。

### 6.5 実装への波及（事実の報告。規範の根拠ではない）

`crates/vtest-scan/src/lib.rs:1178` が `@vtest.target` 欠落で**無条件に** `E-SCAN-007` を出す（`reports/audit13-node-disposition-2026-09-06.md` L502 の記録）。したがって現行実装は `DS-1621` / `DS-1622` の形態条件をまだ満たしていない。`DS-1664` も現行実装では到達しない（`E-SCAN-007` が先に出るため）。**これは事実であって、規範側の判断材料ではない。**

---

## 7. 開示（本書の限界と、残る未決）

1. **`SPEC-065` と `SPEC-301` は同一層で食い違う。** `SPEC-065`「他の実行形態における確認方法は、当該形態に適した方法として**詳細設計で定める**」に対し、`SPEC-301`「…確認方法は…**下位仕様・後続版へ委譲する**」。前者は v0.1 の詳細設計に定める義務を課し、後者は後続版への先送りを許す。両方 `spec` 層なので層順では解けない。**未決事項として開示する。本書は解かない。** `DS-1664` は「確認方法は実施しないことである」という最も弱い読みでのみ `SPEC-065` を満たす。

2. **観測子は適用していない。** §6 は起草であって適用ではない。適用には「既存ノードの statement は変えない」という本作業の枠の再スコープを要する。

3. **`DS-489` を除く7件の後継 statement は、キー1語の挿入以外を変えていない**が、**採否と最終文言は Owner の判断による**。

4. **`execution-form` / `source-target` / `external-contract` は新語である。** 概念は正本にあるが、この綴りは本書の導出である。

5. **キー順の挿入位置（`covers` の直後）は上流に根拠が無い HOW の選択である。**

6. **`export/*.md` は再生成していない。** 変換足場は Owner の指示で削除済み（`CONVERSION.md` L3）であり、本リポジトリに再生成手段が無い。`AGENTS.md` の宣言どおり export は JSON に遅れる。

7. **逆方向の監査（上流にあるのに下流に無い＝欠落）はしていない。** 走査は §1 の逐語確認と、「実行形態」「target_binding」「@vtest.」「Form」「related」を含むノードの全件列挙による。言い換えで書かれた同旨ノードは検出できていない。

8. **`DS-1664` の `source` は転記元ではない。** 新規ノードなので「元 md のどこから写したか」が成立しない。対になる `DS-687` の位置を指すだけである（2026-09-06 の文書モデル再整合と同じ開示）。
