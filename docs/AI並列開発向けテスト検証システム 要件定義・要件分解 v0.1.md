# AI並列開発向けテスト検証システム 要件定義・要件分解 v0.1

## 1. 目的

本システムは、大規模なソフトウェアを複数のAIエージェントおよび人間が並列開発する環境において、テストが存在してPASSしているという事実だけでは信用できない問題を解決するための検証システムである。

本システムは、以下を相互に照合する。

1. 仕様上、何を検証しなければならないか
2. その仕様上の要求がRequirementへ取り込まれ、適切なVerification Obligationへ分解されているか
3. 各Verification Obligationに対して必要なテストが存在するか
4. テストコードが宣言された検証内容を実際に検証しているか
5. 対象実装が仕様・検証内容と一致しているか
6. テストが実際に現在の実装に対して実行されたか
7. 実行結果がPASSしたか

これらを満たした場合のみ、完全検証においてその検証対象をOKとする。

---

# 2. システムの基本原則

## P-001 検証機であり修正方針決定機ではない

本システムは、仕様・Verification Obligation・Test・実装などが食い違っている場合、その不一致を検出する。

本システム自身は、

- 仕様が正しい
- 実装が正しい
- テストが正しい
- どれを修正するべきである

と決定してはならない。

例えば、

```text
Specification       : divide by zero -> Error
Verification        : divide by zero -> Error
Test                : expects 0
Implementation      : returns 0
```

の場合、

```text
Specification <-> Verification     MATCH
Verification  <-> Test             MISMATCH
Test          <-> Implementation   MATCH
```

という検証結果を提示する。

修正対象の選択は、本システムの外側にある開発者・レビュー担当・PM・AI Agentなどが行う。

---

## P-002 Fail Closed

完全検証において、未検証は合格として扱わない。

以下はすべて完全検証ではOKにならない。

```text
NOT_CHECKED
UNKNOWN
NOT_EXECUTED
MISSING
MISMATCH
FAIL
STALE
```

利用者が特定の検証観点だけを確認したい場合は、検証scopeを限定できる。

例えばPMがVerification Obligationの分解だけを確認する場合、

```text
Requested scope:
Specification -> VO

Result:
OK
```

となることを許可する。

この場合も、コード監査や実行が未検証であるという状態そのものをPASSへ変更してはならない。

---

## P-003 単一の正典を重複して持たない

ある事実を別の情報から決定論的に導出できる場合、その事実を複数箇所へ独立して記録し、手動同期を要求する構造を避ける。

検索インデックス、グラフ、キャッシュなどの派生情報は、正となる情報から再構築可能でなければならない。

---

## P-004 AIの自由度を必要以上に与えない

既知の作業手順と既知の入力項目を持つ操作については、AIへ自然言語で自由に編集させる方式より、構造化された質問・入力・検証によって操作させる方式を優先する。

この考え方を本プロジェクトでは仮に

**Agent Form Engineering**

と呼称する。

---

# 3. 検証モデル

本システムでは以下を独立した検証対象として扱う。

```text
Specification
      |
      v
Requirement
      |
      v
Verification Obligation
      |
      v
Test Intent
      |
      v
Test Implementation
      |
      v
Target Implementation
      |
      v
Execution Evidence
```

これは単純な正典チェーンではない。

各成果物間について、独立して一致・不一致を検証する。

登録されたSpecificationの要求事項がRequirementへ取り込まれている完全性と、
RequirementがVerification Obligationへ分解されている完全性は別の検証対象とする。
Requirementは1件以上のSpecification箇所を参照しなければならない。

`spec_coverage`は、検証scopeに登録Specificationが1件以上存在し、各Specificationに参照するactive Requirementが存在し、
根拠付きの意味監査によって要求事項の取り込みが完全であると確認された場合だけ`PASS`とする。
SpecificationまたはRequirementが存在しない場合は`MISSING`、監査未実施は`NOT_CHECKED`、現在の対象hash・集合と一致しない監査だけが存在する場合は
`STALE`、取り込み漏れを確認した場合は`FAIL`、判定不能は`UNKNOWN`とし、既存Requirementの存在だけで
`PASS`へ昇格してはならない。

---

# 4. Verification Obligation

## 4.1 定義

Verification Obligation（VO）は、

> 独立して「この条件が成立するか」と検証可能な仕様上の命題

とする。

VOの粒度を、

- assert文
- test function
- テストファイル

などのコード構文によって決定してはならない。

---

## 4.2 VO分解

仕様は、テストによって十分な網羅性を確認できる単位までVOへ分解できなければならない。

例：

```text
「UTF-8を解析できる」
    |
    +-- 正常UTF-8
    |    +-- ASCII
    |    +-- 2 byte
    |    +-- 3 byte
    |    +-- 4 byte
    |
    +-- 不正UTF-8
         +-- truncated
         +-- invalid continuation
         +-- ...
```

「不正UTF-8でエラーになる」という単一項目を登録しただけでは、必要な不正入力partitionが欠落している可能性があるため、網羅されたとはみなさない。

---

## 4.3 VO階層

VOは階層構造を持つことができる。

```text
VO-CALC
├─ addition
├─ subtraction
├─ multiplication
├─ division
├─ signed operands
└─ invalid operations
   └─ division by zero
```

初回登録時に階層化されていることを必須とはしない。

flatなVO群と階層化されたVO群の両方を扱い、flatなVOを再帰分解または階層化する操作を提供する。

---

## 4.4 VOとTestの対応

VOとTestは1:1に限定しない。

以下を許容する。

```text
1 VO -> 1 Test
1 VO -> N Tests
N VOs -> 1 Test
N VOs -> M Tests
```

TestはVOの検証実装単位であり、VOそのものではない。Testを構成する言語構文やtest runner上の実行単位はadapterが識別する。

---

# 5. 検証空間と網羅性

## 5.1 Partition

仕様から必要な場合、

```text
positive
negative
zero
boundary
invalid
```

などの同値partition・境界値partitionを表現できることが望ましい。

すべてのVOへ明示的なpartition定義を要求しない。

---

## 5.2 組合せ検証

複数の検証軸が存在する場合、その組合せを扱えること。

例：

```text
Sign:
  positive
  negative

Operator:
  addition
  subtraction
  multiplication
  division
```

必要なcoverageが全直積であれば、

```text
positive × addition
positive × subtraction
positive × multiplication
positive × division
negative × addition
...
```

を確認する。

仕様上、独立検証で十分である場合は、

```text
{positive, negative}
{add, sub, mul, div}
```

という軸単位の検証も許容する。

何をもって十分とするかは仕様・検証理由によって決定される。

---

## 5.3 複数VOを検証するTest

例えば、

```text
-3 * 4
```

というTestは、

```text
multiplication
negative operand
```

という複数の条件を同時に確認できる。

このTestが存在することだけを理由として、

```text
multiplication covered
negative operands covered
```

と独立に証明したことにはしない。

以下のいずれかが必要となる。

1. 各観点について独立した検証が存在する
2. 必要と定義された組合せ空間がすべて検証されている

---

# 6. Specification・VO網羅性のAI検証

SpecificationからRequirementへの取り込み完全性、およびRequirementからVOへの分解・網羅性について
LLMを利用可能とする。両者は別の監査結果として保存し、一方の判定を他方へ流用しない。

AIは、

```text
COMPLETE
```

という結論だけを出してはならない。

判定には理由を伴わせる。

例：

```text
Coverage: COMPLETE

Reason:
- 四則演算であるため加算・減算・乗算・除算へ分解した
- signed operandについて正負を確認している
- 複数operatorを含む式を確認している
- zero divisionを特殊異常系として含めている
- parenthesesは仕様書 §4.2 により対象外
```

判定理由から、

- 根拠となった仕様
- 分解観点
- 対象外とした条件
- 必要に応じて具体例

を追跡可能であること。

理由を伴わないAI判定を、正式な網羅性検証済み状態として扱ってはならない。

Specification coverageの判定理由には、対象Specification、取り込んだRequirement、
取り込み対象外とした節または記述、およびその根拠を含める。監査対象には対象Specificationの
現在hashと、それを参照するactive Requirementの完全な集合および各内容hashを束縛する。
SpecificationまたはそのRequirement集合が変化した監査を現在の`PASS`として扱ってはならない。

---

# 7. Test Registry

各Testは安定したTest IDによって識別可能であること。

Test IDをハンドルとして、

```text
Test ID
├─ Test Intent
├─ Verification Obligations
├─ Source Targets
├─ Test Construct
├─ Location
├─ Audit Results
└─ Execution Evidence
```

を検索可能とする。

## 7.1 Test traceability

登録adapterがTestとして発見した実行可能なtest constructは、すべて検証目的を持つ管理対象でなければならない。

発見されたTest集合を `D`、構造上完全なmanaged Test Entity集合を `M` とする。
構造上完全とは、source declarationから構文上有効なTest ID、1件以上の`covers`、その他の必須metadataをTest Entityとして具体化できることをいう。Discovered Testとentityの対応数は構造完全性に含めず、独立した整合性条件とする。
`M`はVO参照の解決とTest IDの大局的一意性を検査する前の集合とし、解決不能な`covers`を持つentityや、他のentityとTest IDが衝突するentityも含む。

完全検証では次を要求する。

```text
∀ d ∈ D:
  dに対応するmanaged Test Entityがちょうど1件存在する
  and managed Test Entity.coversは1件以上である
  and coversの全VO参照を解決できる
  and Test IDが発見結果全体で一意である
```

adapter所有の管理宣言を持たないTest、必須metadataが欠落したTest、または空の`covers`によって対応するmanaged Test Entityが存在しない状態は`MISSING`とする。
構造上完全なentityが持つVO参照を解決できない状態、同一Test constructから複数entityが生じる状態、またはTest ID衝突は`MISMATCH`とする。
discoveryが不完全または解析不能な状態は`UNKNOWN`とする。いずれも完全検証のPASSとして扱ってはならない。

`test_existence` はleaf VOからTestへの方向、`test_traceability` は発見されたTestからVOへの方向を検証する。
両方がPASSの場合だけ、VOとTestの双方向完全性が成立する。

---

# 8. Source Target

テスト対象となる実装コード上のimplementation constructを識別可能でなければならない。

1つのTestは1件以上のSource Targetを宣言できる。複数targetを宣言した場合も、各targetを独立に識別し、代表1件へ縮約してはならない。

ソースコード自体への恒久ID埋め込みは必須としない。
各adapterは、Source Targetを一意に解決でき、同一のsource stateから決定論的に正規化できるTarget Referenceを提供する。
Target Referenceの具体的な構文、namespace、symbol種別は下位仕様へ委譲し、共通契約がpath、module、function等の特定言語構造を必須としてはならない。

恒久SRC IDを使用する場合、そのIDはadapter境界を越えてrepository全体で一意でなければならない。
同一SRC IDを複数adapterまたは複数Source Targetが宣言した状態を曖昧な参照として受理してはならない。

TestからSourceを検索でき、Sourceから関連Testを逆引きできること。

---

# 9. Test Intent

Testには、その実装コードだけを読まなくても、

- 何を検証するか
- どのVOに対応するか
- 何を入力条件とするか
- 何を期待するか

を判断できる情報を関連付けられること。

具体的入力値をTest IntentまたはVOへ含めることを許容するが、必須とはしない。

---

# 10. Parameterized / Table-Driven Test

以下のような論理形式を正式に許容する。code fragmentはRustによる例示であり、共通契約がRust構文を要求するものではない。

```rust
for (input, expected) in cases {
    assert_eq!(target(input), expected);
}
```

この場合、adapterが識別したtable-driven test construct全体を一つのTestとして登録できる。

Test内部の各caseを独立Test IDへ分解することを必須とはしない。

必要な場合、

> cases集合がVOに必要な入力空間を十分に代表または網羅しているか

を監査対象とする。

---

# 11. Test静的監査

明らかに意味のないテストを、可能な限り決定論的な解析によって検出する。

以下の論理的な違反はNGである。code fragmentはRustによる例示であり、各adapterは対応する言語・runnerの構造に対して決定論的に判定できる範囲を提供する。

```rust
assert!(true);
```

宣言されたSource Targetを実行していない。

```rust
let x = 1 + 1;
assert_eq!(x, 2);
```

対象を呼んだだけで結果を検証していない。

```rust
target();
assert!(true);
```

自己比較を行っている。

```rust
assert_eq!(result, result);
```

その他、同様に検証結果が対象実装の正しさに依存しない構造。

決定論的解析で明確にNGとできるTestは、LLM監査へ送る前に拒否可能であること。

---

# 12. Test意味監査

静的監査だけでは判断できないTestについて、LLMによる意味監査を実施可能とする。

主として、

```text
VO
↕
Test Intent
↕
Actual Test Code
```

が本当に同じ振る舞いを検証しているか確認する。

決定論的監査結果とAI監査結果は区別して保持・提示する。

---

# 13. 対象実装との一致検証

仕様・VO・Testと対象実装が一致しているか確認できること。

不一致を検出した場合、

```text
MISMATCH
```

として提示する。

どちらを修正すべきかは決定しない。

---

# 14. Test Execution

Testを実際に実行し、その結果をVerification Evidenceとして利用できること。

実行結果は少なくとも、

```text
PASS
FAIL
NOT_EXECUTED
UNKNOWN
```

を識別可能とする。

---

# 15. Execution Evidenceの鮮度

PASS結果がどのコード状態に対して得られたものか追跡可能であること。

現在の対象実装hashと一致しない状態に束縛されたPASSを、有効なPASSとして利用してはならない。

現在のTest subject・宣言target集合・各対象実装とEvidenceの対応関係を確認できない場合、完全検証では有効なPASS Evidenceとはみなさない。

Test subjectはTest constructだけでなく、Test Entityを具体化するcanonical metadataの論理値、
実行座標およびidentityを含む。metadataの配置がTest constructと隣接しないadapterであっても、
`covers`、`targets`、Test Intentその他のcanonical metadata変更がTest subject hashを必ず変化させなければならない。
意味が同一な宣言表現の正規化はadapterが行えるが、同値性を確定できない変更は安全側でhashを変化させる。

Evidenceは、Testが宣言する全targetについてtarget参照と対象内容hashを保持し、現在の宣言target集合および各対象内容hashと照合可能でなければならない。

---

# 16. Target Execution Verification

詳細な完全検証では、

> 宣言された対象コードが実際にTest実行経路へ入ったこと

を確認可能であること。

TestがPASSしても、対象実装を実際には通っていない場合、そのTestを完全検証済みOKとしない。

複数targetを宣言したTestでは、各targetの実行を個別に計測する。1件でも実行回数が0なら`FAIL`、1件でも解析不能でかつ`FAIL`がなければ`UNKNOWN`とし、全targetの実行を確認できた場合だけ`PASS`とする。

この検証は、完全検証モードではデフォルトで有効とする。

高速な限定scopeの検証では省略可能とする。

省略された場合は、

```text
NOT_CHECKED
```

として扱い、PASSへ変換しない。

---

# 17. OK / NG

## 17.1 簡易出力

利用者向けに、

```text
OK
NG
```

という簡易結果を提供する。

## 17.2 完全検証OK

完全検証におけるOKは、少なくとも以下が成立した状態とする。

```text
Specification coverage        PASS
VO decomposition              PASS
VO coverage                   PASS
Test existence                PASS
Test traceability             PASS
Test static audit             PASS
Test semantic audit           PASS
Implementation consistency    PASS
Target execution              PASS
Test execution                PASS
Runtime result                PASS
Evidence validity             PASS
```

一項目でも、

```text
FAIL
MISMATCH
MISSING
NOT_CHECKED
UNKNOWN
NOT_EXECUTED
STALE
```

であれば完全検証はNG。

---

# 18. 詳細出力

NGの場合、単にNGと表示するだけではなく原因を確認可能とする。

例：

```text
REQ-PARSER-001       NG
├─ VO completeness          PASS
├─ VO-PARSER-001            OK
├─ VO-PARSER-002            NG
│  ├─ Test exists           PASS
│  ├─ Static audit          PASS
│  ├─ Semantic audit        FAIL
│  └─ Execution             PASS
└─ VO-PARSER-003            MISSING
```

---

# 19. 上位への集約

Test単位の結果を上位検証対象へ集約可能とする。

```text
TEST-001       OK
TEST-002       OK
TEST-003       NG
     |
     v
VO-012         NG
     |
     v
Feature        NG
     |
     v
Requirement    NG
     |
     v
Specification  NG
```

集約はfail-closedを基本とする。

PMなどは上位SpecificationまたはRequirement単位からNG箇所まで掘り下げられること。

---

# 20. Structured Test Operation

Test操作の公式経路として、Test IDまたはadapterが識別可能なTest constructを対象とした構造化操作を提供する。

少なくとも、

```text
Create Test
Edit Test
Query Test
Audit Test
```

を対象とする。

Test変更時には、

```text
対象
何を検証するか
VO
入力条件
期待結果
```

などの既知項目を構造化入力できること。

入力内容は可能な限りその場で検証する。

例：

```text
Target symbol not found
Test ID not found
Referenced VO not found
Expected enum variant not found
```

---

# 21. Test編集の境界

公式Test Edit操作では、一回の操作対象を原則として一つのTestとする。

```text
edit TEST-001
```

という操作が、暗黙にTEST-002・TEST-003まで変更することを許可しない。

Test外部の通常ソースコード、helper、fixture等の編集はTest Editの責務としない。

それらは通常のソースコード変更として扱う。

---

# 22. 直接編集

通常のwrite/editツールや人間による直接ソース編集そのものを完全禁止することを要件とはしない。

公式経路としてStructured Test Operationを提供し、それを利用することで、

- 誤ったTestの編集
- 関係情報の更新忘れ
- 同時に複数Testを変更する事故

を低減する。

直接編集による不整合も検証で検出可能であることが望ましい。

---

# 23. 既存プロジェクト対応

本システムは、既に大量のソースコードとTestが存在するプロジェクトを検証対象として扱えなければならない。

既存のSpecification、Source、Testを読み取り、次の状態をそれぞれ可視化できること。

- VOが十分に存在するか
- 既存TestがどのVOを検証するか
- Testに不足がないか
- Testが意味のある検証を行っているか
- 実装との不一致がないか

VOが確定していない範囲を含むプロジェクトも読み取れること。
未登録Test、欠落する正典、未確定のVO、未実施の監査または実行を検証済みとして扱ってはならない。

---

# 24. 仕様入力

仕様ソースとして、ソースコードより上流に位置する成果物を利用可能とする。

対象候補：

- 要件定義
- 基本仕様
- 詳細設計
- API Schema
- Protocol Specification
- interface definition
- 型・データ仕様
- DB schema
- その他の機械可読仕様

対象ソースコード内のdoc commentを、その対象実装自身の正当性を証明する唯一の仕様根拠として使用することは想定しない。

---

# 25. 承認

Verification Obligation等の検証成果物について、確定・承認状態を表現可能とする。

承認は対象自身の内容だけでなく、承認判断が依存するSpecification、Requirement、上位VOの
現在の依存closureへ束縛する。対象またはいずれかの依存成果物が変更された承認を、現在の
承認済み状態として利用してはならない。依存closureまたはhashを欠く承認を推測で有効化してはならない。

承認主体を人間に限定しない。

```text
Human
Verification Agent
Reviewer Agent
PM Agent
```

などが承認主体となり得る。

可能な範囲で、

```text
誰が
何を
どの根拠で
承認したか
```

を追跡可能とする。

---

# 26. 利用者

以下の利用者を正式に想定する。

## Coder AI

自身の変更が要求された検証を満たしたか確認する。

## Developer

Testの作成・変更・検索・検証を行う。

## CI

同一条件で検証を再実行し、再現性を確認する。

## Reviewer AI

Coderの提出したEvidenceと自身の再検証結果を照合する。

## PM / PM Agent

Requirement・Feature単位から、

```text
必要なVOが存在するか
必要なTestへ分解されているか
現在どこがNGか
```

を確認する。

---

# 27. 対応範囲

組込production adapterは `rust-cargo` とし、次を対象とする。

- Rust
- Rust function unit test
- 小規模なintegration test

検証契約・ID・ハッシュ・Evidence・集約の概念モデルは言語およびtest runnerに依存しない。adapterはsource discovery、static audit、Structured Test Operation、test runner、coverageの能力を個別に提供する。

core verifierを変更せずに別adapterを登録できる境界を要求する。`rust-cargo`以外のproduction language adapterは提供範囲に含めない。adapterが未登録、能力不足、または解析不能の場合、検証結果を推測でPASSへ昇格してはならない。

---

# 28. インターフェース

主要インターフェースとして、

```text
CLI
+
AI Agent向けMCP interface
```

を提供する。

GUIは必須要件としない。

---

# 29. 並列AI開発への要求

多数のAI Agentが並列でTestを追加・変更することを前提とする。

以下を要求する。

- 不必要に単一共有台帳へ書き込ませない
- 一つのTest操作が他Testへ波及しにくい
- Test ID衝突を検出できる
- dangling referenceを検出できる
- semantic conflictを検出可能にする
- 派生indexを再構築可能にする
- Testと関連情報の同期を人間・Agentの記憶だけに依存させない

具体的な保存形式や競合回避方式は基本設計で決定する。

---

# 30. 非機能要求

## NFR-001 並列性

複数AI Agentによる並列変更に耐えられること。

## NFR-002 再現性

CI等により同一revision・同一検証条件で結果を再現可能であること。

## NFR-003 追跡可能性

RequirementからTest、TestからSource、SourceからTest、TestからExecution Evidenceまで追跡可能であること。

## NFR-004 再構築可能性

検索index等の派生データを正典から再構築可能であること。

## NFR-005 Fail Closed

検証されていない事実を推測によってPASSとしないこと。

## NFR-006 説明可能性

特にAIによる非自明な検証について、結論だけでなく根拠を提示可能であること。

## NFR-007 自動化適性

AI Agent、CI、CLI等から非対話的に利用可能であること。

## NFR-008 人間可読性

人間がRequirement・VO・Testの不足、不一致、NG理由を一覧で理解可能であること。

---

# 31. スコープ外

以下は本システムの責務としない。

## OOS-001 仕様書同士の品質監査

基本仕様と詳細設計など、複数仕様成果物同士の文章上・意味上の矛盾を発見すること。

## OOS-002 修正方針決定

仕様・Test・実装が不一致の場合に、どれを正としてどれを変更するべきか判断すること。

## OOS-003 通常ソースコード編集管理

Test Edit対象外の通常実装コード、helper、fixtureなどの一般的な編集そのものを管理すること。

## OOS-004 開発プロセス全体の管理

PM、実装、レビュー等の開発工程自体を本システム内部で管理すること。

本システムはそれらの工程が利用するVerification Infrastructureとして機能する。

---

# 32. 要件分解

最上位機能を以下へ分解する。

```text
AI Testing Verification System
│
├─ 1. Specification Intake
│
├─ 2. Verification Planning
│   ├─ Specification -> VO
│   ├─ VO decomposition
│   ├─ hierarchy
│   ├─ partitions
│   ├─ combinations
│   └─ coverage reasoning
│
├─ 3. Test Registry
│   ├─ Test ID
│   ├─ Test Intent
│   ├─ VO mapping
│   └─ Source mapping
│
├─ 4. Structured Test Operations
│   ├─ Create
│   ├─ Edit
│   ├─ Query
│   └─ Audit
│
├─ 5. Deterministic Test Audit
│   ├─ target invocation
│   ├─ assertion validation
│   ├─ trivial assertion detection
│   └─ structural inconsistency
│
├─ 6. Semantic Test Audit
│   ├─ Specification <-> VO
│   ├─ VO <-> Test Intent
│   ├─ Test Intent <-> Test Code
│   └─ Specification/Test <-> Implementation
│
├─ 7. Test Execution
│   ├─ execution
│   ├─ runtime result
│   ├─ target-path verification
│   └─ evidence/revision binding
│
├─ 8. Verification Aggregation
│   ├─ Test
│   ├─ VO
│   ├─ Feature
│   └─ Requirement
│
├─ 9. Reporting
│   ├─ OK / NG
│   ├─ detailed result
│   └─ evidence / reasoning
│
└─ 10. Integration
    ├─ Developer CLI
    ├─ AI MCP
    └─ CI
```

---

# 33. 完全検証の概念フロー

```text
Specification
      |
      v
Generate / Load VO
      |
      v
Audit VO completeness
      |
      v
Locate mapped Tests
      |
      v
Deterministic Test Audit
      |
      v
Semantic Test Audit
      |
      v
Compare with Implementation
      |
      v
Execute Tests
      |
      v
Verify Target Execution
      |
      v
Validate Execution Evidence
      |
      v
Aggregate Results
      |
      +-- all required checks PASS -> OK
      |
      +-- anything else -----------> NG
```

---

# 34. 下位仕様へ委譲する設計事項

以下の具体化は、本文書の要件を満たす基本仕様または詳細設計の責務とする。

1. Specificationの具体的な入力フォーマット
2. VO保存形式
3. Test metadataの具体的なannotation syntax
4. relationの保存形式
5. Test ID命名規則
6. Source symbolの具体的識別方式
7. AST/LSP等の具体的解析技術
8. LLM provider / model
9. AI監査のprompt・skill・agent構成
10. coverage reasonの具体的schema
11. execution evidenceのrevision識別方式
12. CLI command体系
13. MCP tool体系
14. キャッシュ/indexの具体的データ形式
15. 並列編集時の物理的保存方式
16. 境界値・partitionを必須入力にするTest種別
17. AI監査を何重に行うか
18. VO承認workflowの具体的状態遷移

これらは本要件定義を基に次工程で決定する。

---

# 35. 一文での製品定義

本システムは、

> **「テストが通った」を検証するツールではなく、「そのテストを通ったという事実を信用してよいか」を仕様から実行結果まで追跡して検証するツール**

である。
