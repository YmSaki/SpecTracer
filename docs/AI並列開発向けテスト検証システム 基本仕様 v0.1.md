# AI並列開発向けテスト検証システム 基本仕様 v0.1

## 0. 本書の位置付け

本書は「AI並列開発向けテスト検証システム 要求・要件定義 v0.1」（以下、要件定義。FROZEN/v0.1 baseline）の下流文書である。要件定義が **WHY（何を保証しなければならないか）** を定めたのに対し、本書は **WHAT（システムが外部に対して保証する挙動・データモデル・状態モデル・インターフェースの範囲）** を確定する。

具体構文・アルゴリズム・スキーマの全フィールド・コマンド引数などの **HOW** は「詳細設計 v0.1」で定める。本書はそれらを発明しない。要件定義に無い義務・検査・状態・文書種別を本書で新設しない（規範の伝播は上流→下流。要件定義 P-005）。矛盾・不足を発見した場合は本書を書き換えず、上流へフィードバックし Owner 判断を経る。

本書からの `要件定義 §n` 参照は、FROZEN 要件定義の連番（§1〜§28、および原則 P-001〜P-005、要求 R-1〜R-5）を指す。

本書で確定する実現前提（要件定義 §28 委譲事項に対する本書の決定）は次のとおり。

- **ツール名**：`vtest`（バイナリ名・ディレクトリ名に使用する）。
- **`vtest` 本体の実装言語**：Rust。
- **組込 production adapter**：`rust-cargo`。Rust の関数単体テストおよび小規模な結合テスト（`#[test]` 属性を持つテスト関数）を対象とする。`rust-cargo` 以外の production language adapter は v0.1 の提供範囲に含めない（要件定義 §21）。
- **インターフェース**：CLI と、AI Agent 向け MCP サーバ（要件定義 §22。MCP を本体とする）。
- **意味判定の非搭載**：`vtest` 自身は LLM API を呼ばず、宣言と実装の意味的な良し悪しを裁定しない。機械が決定論で確定できない疑義は `UNKNOWN` として外部の判断者へ引き渡す（§11、要件定義 §12）。

Rust 固有処理は組込 `rust-cargo` adapter が所有する。CLI・MCP・検証 core は adapter registry を介して能力を選択し、core の検証契約は言語・test runner に依存しない（要件定義 §21）。

---

## 1. 用語定義

- **document（文書）**：ソースコードより上流に位置する成果物を表す**単一の総称ノード**。要件定義書・基本仕様書・詳細設計書・API Schema・Protocol Specification・型/データ仕様・DB schema・その他の機械可読仕様を含む。`id + path + content_hash + 上流参照（derives_from）` を持つ。文書種別ごとの専用スキーマは設けない（要件定義 §3.2）。対象ソースコード自身の doc comment は、その対象実装の唯一の仕様根拠としては用いない（要件定義 §18）。
- **derives_from**：document 間の唯一のリンク種別。上流 document から下流 document への導出を表す。各リンクは任意（optional）の説明文・導出理由を保持できる（§3.2、要件定義 §3.4）。
- **Verification Obligation（VO）**：独立して「この条件が成立するか」と検証可能な仕様上の命題（要件定義 §10.1）。1 件以上の document から derives_from で導出される。VO と document の間に他のエンティティ層を置かない。階層構造を持てる。粒度を assert 文・test function・テストファイルなどのコード構文で決めない。
- **Test**：登録 adapter が実行可能な検証単位として識別し、Test ID で管理する test construct。VO の検証実装単位であり、VO と N:M の対応を持ちうる。`covers` 宣言で VO を参照する。
- **Test Intent**：Test が「何を検証するか」を実装コードを読まずに判断できる形で表した付随情報。**宣言鎖のノードではない**（要件定義 §14）。
- **検証対象**：その Test が検証成立性（§8）を証明しようとする対象＝宣言された「何の時にどうなる」の主語。実装 construct に限定せず、外部から観測可能な契約・境界上の振る舞いも含む（要件定義 §9.1）。
- **Source Target（SRC）**：実装コード上の識別可能な implementation construct。adapter ID と adapter 所有の opaque locator からなる Target Reference、または任意の恒久 SRC ID で識別する（要件定義 §9.2）。
- **Execution Evidence**：テスト実行の事実の記録。結果・実行時リポジトリ状態・解決後の canonical Source Target 参照・各内容ハッシュ・実行計測結果を含む。検証対象の内容ハッシュに束縛される（§6、要件定義 §6）。
- **判断記録（judgment record）**：`UNKNOWN` に対して外部（人間または判断可能 Agent）が下した判断の記録。actor / subject / decision を必須項目とし、理由・根拠は任意。依存 closure のハッシュに束縛される（§11、要件定義 §12）。
- **承認記録（approval record）**：判断または方針を「この内容で進める」と正式に認めた記録。approver / subject（または judgment reference）/ approved state を必須とし、上流依存 closure のハッシュに束縛される。判断記録とは別軸・別 entity でありうる（§17、要件定義 §19）。
- **検証状態**：`PASS` / `FAIL` / `MISMATCH` / `NO_EVIDENCE` / `UNKNOWN` の 5 つ（§4.1、要件定義 §5.1）。検証結果のみを表し、承認状態を混入させない（要件定義 §5.5）。
- **診断ラベル**：検証状態に付随して原因を説明するラベル（`MISSING` / `NOT_EXECUTED` / `NOT_CHECKED` / `STALE` 等）。検証状態ではない。語彙は詳細設計で定める（要件定義 §5.2、§28）。
- **検査**：`chain_integrity` / `orphan_detection` / `target_binding` / `oracle_presence` の **4 本のみ**（§5、要件定義 §3.3、§4）。
- **完全検証**：宣言鎖全体に対する検査（`chain_integrity` / `orphan_detection`）と、scope 内の各「宣言 + コード + 証拠」の組に対する検査（`target_binding` / `oracle_presence`）をすべて対象とする検証。一項目でも非 `PASS` があれば NG（fail-closed。要件定義 §26.1）。
- **scope**：利用者が限定する検査・エンティティの範囲。狭めても対象外項目を `PASS` へ書き換えない（要件定義 §2/P-002）。
- **正典（source of truth）**：ある事実を決定する唯一の記録。正典から導出できる情報は派生情報とし独立保存しない（要件定義 P-003）。
- **Agent Form Engineering**：既知の作業手順・入力項目を持つ操作を、自由編集ではなく構造化された質問・入力・検証で行わせる方式（要件定義 P-004）。

---

## 2. 全体像

### 2.1 正典の三層構造

本システムは扱う情報を三層に分ける。

```text
1. 宣言（declaration）
   adapter 所有の Test metadata 宣言、および .verify/ 配下の
   document / VO / Relation レコード。Git で管理される正典。

2. 実装（implementation）
   テストコード本体と対象ソースコード。Git で管理される正典。

3. 事実（evidence / decision）
   実行結果・判断記録・承認記録。
   .verify/ 配下の追記型レコードファイル。Git で管理される。

派生情報（検索インデックス、検証グラフ、集約結果）は上記から毎回再構築する。
派生情報は Git 管理しない（要件定義 P-003 / NFR-004）。
```

source discovery、決定論的解析、Structured Test Operation、test runner 起動、coverage 計測は adapter capability として提供する。adapter が返す導出結果は registry で merge し、adapter ID・path・Test ID の順に正規化する。registry の重複 ID、未登録 adapter、adapter 間の Test ID 重複は操作エラーとし、空の scan として成功扱いしない（要件定義 §21）。

本システムの仕事は「宣言と実装が一致しているか」「事実が現在の宣言・実装に対して有効か」を照合することに限る。どれかを正として他を修正させることはしない（要件定義 P-001）。

### 2.2 宣言鎖と照合

要件定義 §3.2 の宣言鎖をそのまま採用する。上流文書はすべて単一の総称ノード型 `document` で表現し、文書間リンクは `derives_from` の一種のみとする。

```text
document（上流文書）
      | derives_from
      v
document（下流文書）
      | derives_from
      v
Verification Obligation (VO)
      | covers（Test 宣言）
      v
Test
```

- 文書層の段数は総称的に扱い、**リンクを追加してもスキーマが壊れない**ことを設計制約とする。段はリンクであって検査ではない（要件定義 §3.2/§3.3）。
- VO は 1 件以上の `document` から derives_from で導出される。VO と document の間に他のエンティティ層を置かない。
- 本システムは文書内容の意味的な良し悪しに関知しない。文書種別ごとの専用スキーマ・文書間リンク意味論の増殖・文書内容の良否検証を行わない（要件定義 §3.2、§25 OOS-001）。
- 不一致はどちらが正かを決めず、状態（§4）として提示する（要件定義 P-001）。

### 2.3 導出できる関係は保存しない

Test → VO（`covers`）、Test → SRC（`targets`）の関係は adapter 所有の Test metadata 宣言から決定論的に導出できる。これらを外部ファイルへ重複保存しない。graph と現在の target 集合は常に adapter 所有の Test metadata 宣言から再構築し、Evidence の target 参照から関係を生成・修復しない。Evidence に含む target 参照は target 別の実行事実と内容ハッシュを束縛する実行時 snapshot key であり、Test → SRC 関係の正典ではない。

外部レコードとして保存するのは、どちらか一方のエンティティに自然に所属しない関係（VO 間の依存、Test 間の補完関係など）だけとする（要件定義 P-003）。

### 2.4 adapter 設定と wire 互換

`config.yaml` writer の正規形は version 2 とし、adapter ごとに root・scan・run 設定を namespace 化する。reader は version 1 を単一の `rust-cargo` adapter 設定として in-memory 変換して読み取るが、読み取りだけで正典を書き換えない。`vtest init` は version 2 を生成する。

adapter ID は設定内で一意でなければならず、同一 adapter 内の root 重複も拒否する。polyglot repository を扱えるよう、異なる adapter が同じ root を走査することは許可する。未知の adapter や adapter 固有設定の検証失敗は操作エラーとし、利用可能な言語や能力を推測補完しない（要件定義 §21）。

core domain の `TestEntity` は、言語・runner 非依存の `execution`（adapter・project・suite・opaque selector）だけを実行座標として持つ。`filter` / `package` / `test_target` は `TestEntity` の field ではない。Test JSON の wire compatibility layer は `execution` を常に出力し、`rust-cargo` Test についてだけ version 1 互換 field を追加出力できる。非 Rust Test ではこれらを省略し、空値・dummy 値・Rust 既定値を生成しない。`targets` list を常に出力し、単数互換 field `target` は target 1 件のときだけ追加出力する。欠落・矛盾時は入力を拒否し、推測で実行可能として扱わない。

---

## 3. エンティティと ID 体系

### 3.1 エンティティ種別

| 種別 | ID | 正典の所在 | 説明 |
|---|---|---|---|
| document | `DOC-` | `.verify/doc/` | 総称の上流文書ノード（path＋content_hash＋derives_from）。種別専用スキーマを持たない |
| Verification Obligation | `VO-` | `.verify/vo/` | 検証命題。階層可 |
| Test | `TEST-` | adapter 所有の Test metadata 宣言 | adapter が識別する実行可能な test construct |
| Source Target | （ID なし／任意で `SRC-`） | adapter ID と opaque locator で識別 | 対象 implementation construct。恒久 ID は必須としない |
| Relation | `REL-`（ULID） | `.verify/rel/` | 外部関係レコード。不変。derives_from の説明文もここに保持できる |
| 判断記録 | ULID | `.verify/decisions/` | `UNKNOWN` への外部判断。追記型 |
| 承認記録 | ULID | `.verify/approvals/` | 判断・方針の正式採用。追記型 |
| Execution Evidence | ULID | `.verify/evidence/` | 実行証拠レコード。追記型 |

document は単一の総称ノードであり、要件定義・基本仕様・詳細設計・API Schema 等を種別で区別する専用スキーマを持たない。文書層の段（要件→仕様→詳細設計…）は derives_from リンクとして表現し、段を増やしても種別を増やさない（要件定義 §3.2）。

### 3.2 ID 規則と関係リンク

DOC / VO / TEST の ID は人間可読な形式とし、利用者（人間または AI）が命名する。

- 文字集合は `[A-Z0-9-]`、接頭辞は種別ごとに固定（`TEST-` 等）。推奨形式は `TEST-<領域>-<連番>`（例：`TEST-PARSER-044`）だが、ツールは形式を強制せず一意性のみを強制する。
- ID の一意性はスキャン時に全数検査し、衝突は `chain_integrity` の非 `PASS`（`MISMATCH`）とする（§5.1、§23）。
- 任意の恒久 SRC ID は adapter namespace を持たないため repository 全体で一意とする。衝突は曖昧参照として受理せず、どの Source Target を指すか推測しない（要件定義 §9.2）。
- **関係リンクは説明文・導出理由を任意（optional）で保持できる。** derives_from・covers・検証対象・実装 traceability など性質の異なる関係型は潰さず区別する。存在するリンクに付す説明文は空でもよく、空であることを理由に `chain_integrity` 違反・`MISMATCH` としてはならない（要件定義 §3.4）。関係型そのものの意味論的増殖は求めない。
- Relation writer は `REL-<ULID>` を正規 ID としてファイル名に用いる。reader は version 1 互換入力として bare ULID を `REL-<ULID>` へ in-memory 正規化する。判断・承認・Evidence の ID は bare ULID とする。ULID payload により並列生成時のファイル名衝突を実用上排除する。
- 関係リンクの任意説明文・役割別 projection の保存形式・preset は詳細設計へ委譲する（要件定義 §28）。

### 3.3 Source Target の識別

ソースコードへ恒久 ID を埋め込むことは必須としない（要件定義 §9.2）。対象は **Target Reference** で識別する。Target Reference は adapter ID と adapter 所有の opaque locator の組、または任意の SRC ID 参照である。

```text
<adapter-id>::<opaque-locator>
例：rust-cargo::src/parser.rs::Parser::parse
```

opaque locator の構文と恒久 SRC ID の宣言方法は adapter が定める。共通契約が path・module・function 等の特定言語構造を必須としてはならない（要件定義 §9.2、R-3）。1 つの Test は 1 件以上の Source Target を持ち、各 target 参照を個別に保持する。代表 1 件へ縮約しない。Test → SRC の対応は adapter 所有の Test metadata 宣言から、SRC → Test の逆引きはスキャン結果から提供する。

---

## 4. 検証状態と診断ラベル

### 4.1 状態は 5 つ

検証状態は次の 5 つのみとする（要件定義 §5.1）。状態の存在資格は「**受け取った者の行動が変わるか**」であり、意味の違いは資格にならない。

| 状態 | 受け取った者の行動 | 完全検証での OK 可否 |
|---|---|---|
| `PASS` | マージ可 | 可 |
| `FAIL` | 実装（テスト実装を含む）を直す | 不可 |
| `MISMATCH` | コードを触る前に宣言側（上流）を直す | 不可 |
| `NO_EVIDENCE` | 証拠を作る（機械的に解決可能） | 不可 |
| `UNKNOWN` | 決定論の限界。意味判定できる者へエスカレーションする | 不可 |

### 4.2 診断ラベル

`MISSING` / `NOT_EXECUTED` / `NOT_CHECKED` / `STALE` 等は、状態に付随して原因を説明する**診断ラベル**であり、検証状態ではない。診断ラベルの語彙は詳細設計で定める（要件定義 §5.2、§28）。本書は状態と診断ラベルを常に別軸として扱い、混同しない。

### 4.3 状態の割当

要件定義 §5.3 の割当をそのまま採用する。

| 事象 | 状態 | 診断ラベル |
|---|---|---|
| 発見された Test に管理宣言が無い | `MISMATCH` | MISSING |
| `covers` の VO 参照を解決できない／同一 construct から複数 entity／Test ID 衝突 | `MISMATCH` | — |
| 文書鎖のリンク切れ／content_hash 不一致／孤児文書 | `MISMATCH` | STALE 等 |
| 証拠が存在しない／証拠のハッシュが現在の対象と不一致 | `NO_EVIDENCE` | STALE 等 |
| scope 限定により検査を実施しなかった項目（完全検証の集約時） | `NO_EVIDENCE` | NOT_CHECKED |
| discovery が不完全／解析不能 | `UNKNOWN` | — |
| テストランナーが失敗を報告 | `FAIL` | — |
| 宣言された検証対象の実行が 0 回 | `FAIL` | NOT_EXECUTED |

### 4.4 UNKNOWN の検疫

`UNKNOWN` はエラーではなく**正常動作としての降参**である。内部エラー・入力不正は検証状態と別系統（終了コード。§27）で表現する。`UNKNOWN` をエラー処理のフォールバック先として使う実装は仕様違反とする（要件定義 §5.4）。

### 4.5 検証状態と承認の分離

検証状態（§4.1 の 5 状態）は検証結果のみを表し、承認状態を混入させない。承認（§17）は独立した別軸である（要件定義 §5.5）。

- 技術的に `PASS` であっても未承認である状態を許容する。
- 未承認であることだけを理由に `PASS` を `UNKNOWN` 等へ変更してはならない。
- 承認済みであることを理由に `FAIL` / `MISMATCH` / `NO_EVIDENCE` / `UNKNOWN` を `PASS` へ変更してはならない。

フェーズ進行に承認を要するかは、検証状態と承認の組合せとして §21 のゲート条件で扱う。

### 4.6 scope

利用者は検証 scope を次の 2 軸で限定できる（要件定義 P-002）。

- **検査軸**：実施する検査（4 本の部分集合）を指定する。
- **エンティティ軸**：対象とする document / VO / Test の部分木を指定する。

scope を限定した検証の OK は「要求された scope 内が OK」の意味に限られる。scope 外・未実施の項目は `NO_EVIDENCE`（診断 NOT_CHECKED）として保持し、`PASS` へ変換しない。出力には要求 scope と、scope 外項目が未検証である旨を必ず併記する。いかなる設定値も完全検証の検査を 4 本未満へ縮退させない。

---

## 5. 検査

検証は次の 4 検査のみで行う。鎖に段（リンク）が増えても検査は増えない（要件定義 §3.3、§4）。各検査は一つの問いを持ち、複数の証拠源で答えてよい。答えは検証方法・実行形態に依らず同一でなければならない（要件定義 §4 冒頭、§8 条項 3）。

凍結要件が検査から明示的に排除した判断（仕様網羅・VO 網羅・VO 分解妥当性・意味一致・実装一致）は、本書でも検査に含めない。網羅・意味の疑義はエスカレーション（§11）の領分である（要件定義 §10.2、§11、§12）。

### 5.1 chain_integrity — 宣言鎖の完全性

**問い：宣言鎖のすべてのリンクが存在し、ハッシュ照合が成立するか。**

- 文書層：各 `document` の derives_from 参照先が存在し、content_hash が現物と一致すること。
- VO 層：各 VO が 1 件以上の `document` への解決可能な derives_from を持つこと。
- Test 層：発見された各 Test に対応する管理宣言（構文上有効な Test ID・1 件以上の `covers`・その他の必須 metadata）がちょうど 1 件存在し、`covers` の全 VO 参照を解決でき、Test ID が発見結果全体で一意であること。
- leaf VO → Test（検証実装の存在）と、発見された Test → 宣言（管理宣言の解決）の両方向が成立して初めて双方向完全性が成立する。
- どのリンクで切れたかは診断ラベルで示す。違反時の状態は §4.3 に従う（管理宣言欠落は `MISMATCH`/MISSING、参照解決不能・ID 衝突は `MISMATCH`、リンク切れ・hash 不一致は `MISMATCH`）。

すべての Test を管理対象とすることと、当該 Test を仕様適合の証拠として算入すること（§8）は別個の条件とする。

### 5.2 orphan_detection — 文書層の孤児検出

**問い：親を持たない `document` ノードが存在するか。**

- 根として指定された文書は対象外とする。根の指定方式は、`.verify/` 設定における明示的な根指定として保持する（具体構文は詳細設計へ委譲。要件定義 §28）。
- 対象は文書層のみとする。実装レイヤーの孤児検出（宣言されていない実装の検出）は行わない（要件定義 R-2、§25 OOS-005）。
- 根に指定されない孤児文書は `MISMATCH` とする（§4.3）。

### 5.3 target_binding — 宣言対象の振る舞いの実現

**問い：その Test が検証対象とする振る舞いが実際に生じ、その振る舞いを反映した観測が得られたか。**

- Test がテストランナー上で `PASS` しても、検証対象とする振る舞いを実際には生じさせていない場合、完全検証済み OK としない。テストランナーの `PASS`/`FAIL` は判定権威（§7）の証拠として消費し、本検査はその証拠が検証対象の実行を伴ったかを問う。
- 一つの問いに対し静的解析と動的計測の 2 つの証拠源を持つ。静的に確定できなければ `UNKNOWN` とし、動的証拠で昇格できる。
- 実装 construct（Source Target）を検証対象とする実行形態では、宣言された対象コードが実際に Test 実行経路へ入ったことを確認方法とする。複数 target を宣言した Test では各 target の実行を個別に計測し、**1 件でも実行回数が 0 なら `FAIL`（診断 NOT_EXECUTED）、1 件でも解析不能でかつ `FAIL` が無ければ `UNKNOWN`、全 target の実行を確認できた場合だけ `PASS`** とする。
- 別プロセス（起動した subprocess）・別スレッド・クロージャ・他ファイル等、静的解析の到達境界を越えて target を実行する Test では静的に到達を証明できず `UNKNOWN` となる。この到達 `UNKNOWN` は、当該 target の動的計測が実行を証明した場合に限り到達要件を満たす。subprocess であること自体を欠陥としない。
- 他の実行形態における確認方法は、当該形態に適した方法として詳細設計で定める。特定形態の確認方法を別形態の Test へ一律要求しない（要件定義 §4.3、§8 条項 3）。
- 完全検証ではデフォルト有効とし、高速な限定 scope では省略可能とする。省略・計測環境不在の場合は `NO_EVIDENCE`（診断 NOT_CHECKED）とし、`PASS` へ変換しない。

### 5.4 oracle_presence — 照合装置の存在

**問い：宣言された「何の時にどうなる」の不成立を、Test の非成功として反映する装置が存在するか。**

出力は次で定まる（要件定義 §4.4）。

1. 不成立が構造から証明できる（どんな宣言の下でも不成立を検出できない＝失敗し得ない、または失敗が検証対象の振る舞いに依存しない、ことが構造から証明できる）場合、`FAIL`。
2. 照合装置の存在が決定論的に確認できる場合、この検査は成立側とする。
3. どちらも決定論的に言えない（解析不能等）場合、`UNKNOWN`。

- 静的解析の役割は**不成立の証明**である。成立条件から明確に外れる Test を決定論的に検出し、外部監査へ送る前に拒否する（§8）。静的解析は成立の証明装置ではなく、証明の失敗は `UNKNOWN` の事由ではない。
- 照合内容が宣言の期待と意味的に一致するかは本検査の主張に含めない。意味の疑義は検査ではなくエスカレーション（§11）の領分である。
- 答えは assert の所在・実行形態（内部 construct 検証か境界の振る舞い検証か）に依らず同一でなければならない。実行形態別の判定規則を設けない。

### 5.5 決定論的に検出可能な不成立構造

`rust-cargo` adapter の Static Audit capability は、§8.3 の不成立構造を決定論的に検出する。少なくとも次を対象とする。

- 成否判定が定数である（`assert!(true)` 等、失敗し得ない）。
- 検証対象の振る舞いを生じさせるだけで、その観測を成否判定に利用していない。
- 観測同士の自己比較（`assert_eq!(x, x)` 等）で、成否が検証対象の振る舞いに依存しない。
- 空のテスト本体。

判定は保守的に行い、決定論的に確定できる違反のみ `FAIL`、確定できないものは `UNKNOWN` とする。core は adapter 固有の AST・assertion 構文・call graph を解釈せず、正規化されたルール結果を検証・集約する。code fragment の具体構文は adapter の言語・runner に従い、共通契約が Rust 構文を要求しない（要件定義 R-3、§8.3）。

---

## 6. 証拠

証拠は検証対象の**内容ハッシュに束縛**される（要件定義 §6）。

- 証拠ストアはハッシュキーを必須とする。現在のソースのハッシュと一致しない証拠は、検証時に「存在しないもの」として扱い、`NO_EVIDENCE`（診断 STALE）とする。
- Evidence の判定結果を変えうる Test の意味・実行条件・対象実装・実行可能状態が現在状態と一致することを確認できなければ、その Evidence を現在の `PASS` として利用してはならない。この要求は**ハッシュ束縛によって設計制約として満たす**。鮮度の独立検査は設けず、鮮度喪失は診断ラベル `STALE` として説明する。
- Test の内容ハッシュは Test construct だけでなく Test subject 全体（少なくとも adapter ID・Test ID・全論理 field・Source Location・実行座標・Test construct）へ束縛する。`covers` / `targets` / `intent` / 実行座標その他の意味変更は内容ハッシュを必ず変化させる。
- adapter は source range・source bytes・解析した論理 metadata・実行座標を hash 未計算の discovery DTO として返す。core が言語非依存の正規化規則で subject hash を計算してから Test Entity を具体化する。adapter が最終内容ハッシュを自己確定してはならない。

Evidence 記録・鮮度照合の具体手順は §22 に、実行機構は §22 に示す。

---

## 7. 判定権威

テスト合否の判定権威は、当該 adapter のテストランナーにある（要件定義 §7）。**本システムは合否を判定せず、テストランナーの結果を証拠として消費する。** 実行の起動は本システムから行ってよいが、判定はしない。`rust-cargo` adapter における判定権威は `cargo test` である。

`vtest` は照合（宣言・実装・証拠の一致検査＝§5 の 4 検査）を行うのであって、テストの合否そのものを再判定しない。`target_binding`（§5.3）はランナーの `PASS` を前提に、その `PASS` が検証対象の実行を伴ったかを問う独立の照合である。

---

## 8. Test の検証成立性

### 8.1 成立と算入の独立

管理対象となる Test は、その宣言された目的に対して、検証対象の振る舞いを反映した観測に基づく有効な成否判定を持たなければならない。仕様適合性の証拠として算入する Test は、その検証成立性が確認済みでなければならない。**Test として成立しているかの検査（§8）と、仕様適合性の証拠として算入するかの判定は独立である**（要件定義 §8.1）。全 Test を管理対象とすること（`chain_integrity`）と証拠算入（成立性）は別系統とする。

### 8.2 成立性の必要条件

1. **検証成立性**：Test は、検証対象の振る舞いを反映した結果・状態・観測に基づいて適合と不適合を識別し、不適合が Test の非成功として反映されるものでなければならない。
2. **依存要素の信頼性**：Test の成否判定が他の構成要素の判定能力に依存する場合、その依存要素の正当性が確認されるか、検証基盤として明示的に信頼されていなければ、当該 Test の検証成立性を確認済みとして扱ってはならない。判定能力を担う依存要素は、正当性確認対象または明示的な信頼基盤として識別可能であり、成立性確認はそのいずれかで終端しなければならない。
3. **証明方法への非依存**：成立条件の確認方法は検証対象・実行形態・観測方法に応じて異なってよい。特定形態固有の確認方法を別形態へ一律要求しない。成立性の問いへの答えは確認方法に依らず同一でなければならない。
4. **未確認と違反の区別**：成立条件を確認できないことと、成立条件に違反していることを区別する。確認不能であることだけを根拠に違反を推定してはならず、成立確認済みとして扱ってもならない。

`oracle_presence` の信頼基盤の具体的範囲（標準 assert 構文・framework failure semantics・設定による列挙）と委譲確認の方法は詳細設計へ委譲する（要件定義 §28）。

### 8.3 決定論的に検出可能な不成立構造

以下は §8.2 の成立条件を満たさないことを、**宣言の中身に依らず**決定論的に検出できる例である。いずれも「どんな宣言の下でも不成立を検出できない」ことが構造から証明できる（要件定義 §8.3）。code fragment は Rust による例示であり、共通契約が Rust 構文を要求しない（R-3）。

例示の前提宣言：`VO-EX-001: parse_u32 は不正な 10 進文字列に対して Err を返す` / `Test 宣言: covers = [VO-EX-001], 検証対象 = parse_u32`。

失敗し得ない（成否判定が定数）：

```rust
#[test]
fn rejects_invalid_decimal() {
    parse_u32("12a");
    assert!(true);
}
```

検証対象の振る舞いを生じさせるが、その観測を成否判定に利用していない：

```rust
#[test]
fn rejects_invalid_decimal() {
    parse_u32("12a");
    let x = 1 + 1;
    assert_eq!(x, 2);
}
```

観測同士の自己比較で、成否が検証対象の振る舞いに依存しない：

```rust
#[test]
fn rejects_invalid_decimal() {
    let r = parse_u32("12a");
    assert_eq!(r, r);
}
```

各 adapter は対応する言語・runner の構造に対して決定論的に判定できる範囲を提供する。`static_audit` に相当する判定は §5.4/§5.5 の `oracle_presence` として現れ、独立した検査項目を新設しない。

---

## 9. 検証対象と Source Target

### 9.1 検証対象

すべての管理対象 Test は 1 件以上の検証対象を宣言できなければならない。検証対象は、その Test が検証成立性（§8）を証明しようとする対象＝宣言された「何の時にどうなる」の主語であり、実装 construct に限定しない。外部から観測可能な契約・境界上の振る舞いも検証対象にできる（要件定義 §9.1）。

実装 construct（Source Target）を直接検証する実行形態では Source Target 宣言をそのまま検証対象の宣言として扱い、同一対象の二重宣言を要求しない。外部契約・境界上の振る舞いを検証する実行形態では、その契約・振る舞いを検証対象とし、内部 Source Target の宣言を Test 成立性の必須条件としない。

### 9.2 Source Target の識別

実装コード上の implementation construct を Source Target として識別可能でなければならない（要件定義 §9.2）。

- 1 つの Test は 1 件以上の Source Target を宣言できる。複数 target を宣言した場合も各 target を独立に識別し、代表 1 件へ縮約しない。
- ソースコードへの恒久 ID 埋め込みは必須としない。各 adapter は Source Target を一意に解決でき、同一 source state から決定論的に正規化できる Target Reference を提供する。具体的構文・namespace・symbol 種別は詳細設計へ委譲し、共通契約が特定言語構造を必須としない（R-3）。
- 恒久 SRC ID を使用する場合、adapter 境界を越えて repository 全体で一意でなければならない。同一 SRC ID の複数宣言を曖昧参照として受理しない。

### 9.3 実装 traceability

検証対象とは別に、Test または検証対象から関連する Source Target への traceability を保持できる。この traceability は**任意**であり、影響分析・逆引きに利用できるが、その存在自体を Test 成立性の条件としてはならない。traceability は関連付けであって実装対応の証明ではない。**検証対象と実装 traceability は別の関係として扱い、一方から他方を推定してはならない**（要件定義 §9.3）。Source Target との関係を持つ Test について、Test から Source を検索でき、Source から関連 Test を逆引きできる。

---

## 10. Verification Obligation

- **定義**：VO は独立して「この条件が成立するか」と検証可能な仕様上の命題とする。粒度を assert 文・test function・テストファイルなどのコード構文で決めない（要件定義 §10.1）。
- **分解**：仕様は、テストで十分な網羅性を確認できる単位まで VO へ分解**できる**。本システムは分解を表現・保持するデータモデルを提供する。分解が十分かの判定は本システムの検査ではなくエスカレーション（§11）の領分である（要件定義 §10.2）。
- **階層**：VO は階層構造を持てる。初回登録時の階層化を必須とせず、flat な VO 群と階層化 VO 群の双方を扱い、flat な VO を再帰分解・階層化する操作を提供する（要件定義 §10.3）。
- **VO と Test の対応**：1:1 に限定せず `1:1` / `1:N` / `N:1` / `N:M` を許容する。Test は VO の検証実装単位であり VO そのものではない（要件定義 §10.4）。
- **検証空間の表現**：VO には検証軸（dimension）と同値/境界値 partition を定義**できる**（すべての VO へは要求しない）。複数軸を持つ VO には組合せ coverage の方針を宣言できる（各軸独立／全直積／明示列挙）。何をもって十分とするかの判定は本システムの検査ではない（→ §11）。複数観点を同時確認する Test の存在だけを理由に各観点を独立に証明したことにはしない。各観点の独立検証、または必要と定義された組合せ空間の検証のいずれかを表現・確認できる（要件定義 §11）。

partition・組合せ coverage 方針の具体的保存形式・語彙は詳細設計へ委譲する（要件定義 §28）。

---

## 11. 発見・意味判定のエスカレーションと判断記録

本システムは、宣言されていない義務・網羅漏れ・宣言と実装の意味のずれを、自ら発見・裁定しない。発見者・裁定者は外部（人間または Agent）である（要件定義 §12）。本システムの責務は次の 3 つに限る。

### 11.1 データ形態の提供

外部の発見者が判断できる構造化出力（要求該当箇所と対応概念のペア、宣言鎖と検査結果、対象外とした範囲）を提供する。`vtest` 自身は LLM API を呼ばず、意味判定・候補生成を検証成立条件にしない（要件定義 §17.2）。外部 AI/Agent による補助・提案は許容するが、その能力を成立条件にしない。

### 11.2 エスカレーション

機械が決定論で確定できない疑義は `UNKNOWN` として、意味判定できる者へ引き渡す。`UNKNOWN` は正常動作としての降参であり、エラー処理のフォールバック先に使わない（§4.4）。

### 11.3 判断の記録と再検証

`UNKNOWN` に対して外部（人間または判断可能 Agent）が判断できる。判断はその時点の対象成果物・前提状態に対して、依存 closure（ハッシュ）とともに**判断記録**へ保存する。

- 判断記録は少なくとも「誰が（actor）」「何を（subject）」「どう判断したか（decision）」を必須項目とし、追跡可能とする。理由・根拠・evidence note（根拠となった宣言、対象外とした範囲、具体例等）は任意（optional）とし、保存できる構造とする。**理由が空であることだけを根拠に、その判断を無効・`UNKNOWN`・`NO_EVIDENCE`・`MISMATCH` 等として扱ってはならない**（要件定義 §12）。
- 判断記録の生成・保存は、次の構造化プロトコルで行う。`vtest` が判断対象の情報一式（VO、Test Intent、テストコード、対象実装、関連テスト、既知 partition、過去の判断、対象の内容ハッシュとリビジョン）を構造化出力する（bundle 生成）。外部の人間/Agent が判断し、判断結果（decision＋任意の理由）を提出する。`vtest` は bundle との対応・対象内容ハッシュの現在一致・decision 値の妥当性を検証して受理・拒否し、受理結果を依存 closure のハッシュに束縛して保存する。**このプロトコルは検証状態のゲートではない**。判断記録の受理は当該対象の検証状態を昇格させない（§4.5）。
- **判断済みと承認済みは区別する**（判断済み ≠ 承認済み）。判断は承認なしでも記録でき、正式採用は §17 の別段階である。判断記録と承認記録は同一 entity であることを要求しない（別 entity でありうる）。
- 仕様・VO・Test 等が変更された場合、過去の判断を現在状態へそのまま流用してはならず、現在状態に対して通常の検証を再実施する。その結果は `PASS` / `FAIL` / `MISMATCH` / `NO_EVIDENCE` / `UNKNOWN` のいずれにもなり得る。変更そのものが `UNKNOWN` を生成するのではない。

エスカレーション出力・判断記録の具体的 schema、判断待ち情報（§18.3）の構造 schema と取得インターフェース、判断の多重度は詳細設計へ委譲する（要件定義 §28）。

---

## 12. Test Registry

各 Test は安定した Test ID によって識別可能とする。Test ID をハンドルとして、Test Intent・`covers`（VO 参照）・検証対象・Source Target・Location・判断記録・Execution Evidence を検索可能とする（要件定義 §13）。

登録 adapter が Test として発見した実行可能な test construct はすべて管理対象とする。発見された Test 集合を `D`、構造上完全な managed Test Entity 集合を `M` とする。構造上完全とは、source declaration から構文上有効な Test ID・1 件以上の `covers`・その他の必須 metadata を Test Entity として具体化できることをいう。Discovered Test と entity の対応数は構造完全性に含めず、独立した整合性条件とする。`M` は VO 参照の解決と Test ID の大局的一意性を検査する前の集合とし、解決不能な `covers` を持つ entity や Test ID が衝突する entity も含む。

完全検証では次を要求する（`chain_integrity`。§5.1）。

```text
∀ d ∈ D:
  d に対応する managed Test Entity がちょうど 1 件存在する
  and managed Test Entity.covers は 1 件以上である
  and covers の全 VO 参照を解決できる
  and Test ID が発見結果全体で一意である
```

違反時の状態は §4.3 に従い、いずれも完全検証の `PASS` として扱わない。発見されたが管理宣言を持たない construct（`rust-cargo` では `@vtest` annotation を持たない `#[test]` 等）は診断 severity としては warning のままとするが、構造上完全な managed Test Entity へ対応しない事実は `chain_integrity` の非 `PASS`（`MISMATCH`/MISSING）として完全検証へ反映する。診断 severity と検証状態を混同しない。

**Test の存在理由による分類（role / anchor / anchor_rationale 等）と、それに基づく `covers` 件数の可変制約は v0.1 では設けない。** すべての管理対象 Test に `covers ≥ 1` を一律に要求する（要件定義 §4.1、§13）。VO への寄与は `covers` 宣言と証拠の十分性判定だけから導出する。

---

## 13. Test Intent

Test には、その実装コードだけを読まなくても、何を検証するか・どの VO に対応するか・何を入力条件とするか・何を期待するかを判断できる情報を関連付けられること。**Test Intent は Test Entity の付随情報であり、宣言鎖のノードではない**（要件定義 §14）。具体的入力値を Test Intent または VO へ含めることを許容するが必須としない。

---

## 14. Parameterized / Table-Driven Test

table-driven の論理形式を正式に許容する。adapter が識別した table-driven test construct 全体を一つの Test として登録でき、内部の各 case を独立 Test ID へ分解することを必須としない。必要な場合、cases 集合が VO に必要な入力空間を十分に代表・網羅しているかは §11 の発見・判定の対象とする。code fragment の具体構文は adapter の言語・runner に従う（要件定義 §15、R-3）。

---

## 15. Structured Test Operation

Test 操作の公式経路として、Test ID または adapter が識別可能な Test construct を対象とした構造化操作を提供する（要件定義 §16、P-004）。

- **Create Test**：Form Schema に基づく構造化入力を adapter へ渡し、Test construct と対応する metadata 宣言を生成する。
- **Edit Test**：Test ID を編集ハンドルとして、adapter が識別する対象 Test の metadata 宣言および Test construct を更新する。
- **Query Test**：Test ID・VO・Target Reference 等からの検索と逆引き。
- **Audit（判断）Test**：§11 の判断記録 bundle 生成と判断結果の提出。

### 15.1 desired state 方式

Create / Edit の入力は差分操作ではなく**あるべき状態（desired state）**とする。利用者は「TEST-X はこの状態である」を宣言し、adapter が現状との差分を計算して Test construct と metadata 宣言を更新し、core が結果を再スキャンして検証する。

### 15.2 入力検証

構造化入力の各項目は可能な限り受理時に検証する（対象 symbol 不在、Test ID 不在、参照 VO 不在等）。解決不能な場合は adapter が候補を提示する。

### 15.3 編集境界

- 公式 Edit 操作の一回の対象は原則 1 Test とする。暗黙に他 Test を変更しない。
- 編集は adapter が特定した単一の metadata 宣言範囲と Test construct 範囲に限定する。
- Test 外部の通常ソースコード・helper・fixture の編集は責務外とし操作を提供しない（要件定義 §16、§25 OOS-003）。
- 通常の write/edit ツールや人間による直接ソース編集は完全禁止しない。公式経路の提供により誤編集・更新忘れ・複数 Test 同時変更の事故を低減し、直接編集による不整合も検証（§5.1）で検出可能とする。source declaration が正典であるため、`covers` / `targets` の「同期漏れ」は構造的に発生しない（要件定義 §16）。

### 15.4 Form Schema

テスト種別ごとの質問・入力項目テンプレートを **Form Schema** として `.verify/forms/` に定義できる。Rust 関数単体 Test 用と小規模結合 Test 用の組込 schema を同梱し、`rust-cargo` adapter が登録する。CLI・MCP のいずれからも同一 schema を消化できる。Form Schema の `kind` は repository 内で大局的に一意な Form ID とし、schema はそれを処理する adapter ID を別 field で宣言する。registry は `kind` からちょうど 1 件の Structured Test adapter へ解決できる場合だけ操作を許可し、重複・未知 adapter・未対応 capability・曖昧な対応を拒否する。未知の form を core が Rust 用として推測してはならない。境界値・partition の必須入力化は組込 Form では設けず、user-defined Form Schema が指定できる（要件定義 §28、§11）。

---

## 16. 仕様入力（文書層）

仕様ソースとして、ソースコードより上流に位置する成果物（要件定義・基本仕様・詳細設計・API Schema・Protocol Specification・型/データ仕様・DB schema・その他の機械可読仕様）を利用可能とする。取り込まれた上流成果物は §2.2 の `document` ノードとして登録され、content_hash と derives_from を持つ（要件定義 §18）。

対象ソースコード内の doc comment を、その対象実装自身の正当性を証明する唯一の仕様根拠として使用しない（要件定義 §18）。文書の具体的入力フォーマットと登録方式、根の指定方式は詳細設計へ委譲する（要件定義 §28）。

---

## 17. 承認

承認とは、判断（§11 の判断記録を含む）または方針を「この内容で進める」と正式に認め確定状態にすることである。**判断済みと承認済みは区別し（判断済み ≠ 承認済み）、未承認の判断は承認済みより弱い**（要件定義 §19）。VO 等の検証成果物について確定・承認状態を表現可能とする。

- 承認は対象または参照する判断（judgment reference）に承認済み状態を与える。§11 の `UNKNOWN` 判断も承認対象になり得るが、判断できることと正式承認は別段階である。
- 承認は対象自身の内容だけでなく、承認判断が依存する上流文書・上位 VO の現在の依存 closure へ束縛する。VO の依存 closure は、再帰的な上位 VO・参照する document（およびその上位 document）からなる。対象またはいずれかの依存成果物が変更された承認を、現在の承認済み状態として利用してはならない。変更後は現在状態に対して検証を再実施し、その結果（§4.1 の 5 状態のいずれか）に従う。依存 closure またはハッシュを欠く承認を推測で有効化してはならない（承認レコードは読み取り互換のため保持できるが、現在の承認済みを導出してはならない）。
- 承認記録は「誰が（approver）」「何を（subject または judgment reference）」「どの承認状態か（approved state）」を必須項目として追跡可能とし、根拠は任意（optional）に記録できる。承認記録は §11 の判断記録と同一 entity であることを要求しない。
- 承認主体を人間に限定しない。Agent も承認権限を持ち得る（Human / Verification Agent / Reviewer Agent / PM Agent 等）。ただし全 Agent が承認権限を持つことは要求せず、一般作業 Agent が承認権限を持つべきとも要求しない。承認主体は種別（`human` / `agent`）と識別子（エージェント名・モデル名等）を記録する。
- 誰がどの対象・範囲を承認できるか（approval authority）はプロジェクト側で定義可能とする。

**承認は検証状態と独立の別軸である**（§4.5）。承認済みを理由に非 `PASS` を `PASS` へ昇格させず、未承認を理由に `PASS` を降格させない。具体的な承認ロール・必要承認数・権限 schema・承認 workflow の状態遷移は詳細設計へ委譲する（要件定義 §28）。

---

## 18. 途中導入と既存プロジェクト対応

本システムはプロジェクト開始時からの導入を前提としない。開発途中または既存プロジェクトへ後から導入できる（要件定義 R-5、§17）。

### 18.1 既存資産の可視化

既に大量のソースコードと Test が存在するプロジェクトを検証対象として扱える。既存の文書・Source・Test を読み取り、VO の存在状況・既存 Test と VO の対応・Test の不足・検証成立性・宣言との不一致を可視化する。VO が確定していない範囲を含むプロジェクトも読み取れる。未登録 Test・欠落する宣言・未確定の VO・未実施の検査または実行を検証済みとして扱わない（状態は §4.3）。

- `vtest init` は `.verify/` を作成し、既存コードを変更しない。
- `vtest scan` は発見した未登録 Test を未登録として報告する。
- document / VO、Test metadata 宣言、判断記録、Evidence の一部が欠ける状態も読み取り可能とする。
- `vtest verify` は正典または検証事実の欠落を対応する非 `PASS` 値として表示し、部分的な登録・判断・実行状態を総合 `OK` として扱わない。

### 18.2 導入時の責務境界

決定論的に処理可能な作業について人間の反復手入力を必須としない。一方、要求・要件・仕様・VO 等の意味上の定義や対応関係を決定する責任はプロジェクト側（開発者・設計者・PM 等）にある。本システムが意味判断・候補生成を行うことを必須要件としない。外部 AI/Agent による補助・提案は許容するが、その能力を検証成立条件にしない（要件定義 §17.2）。

### 18.3 判断待ち情報の構造化

導入時・検証時に生じる、未確定事項・不整合・未検証事項・機械的に確認済みの事実・外部判断が必要な事項を、機械可読な構造として保持・取得可能とする。表示形式（表・GUI 等）は要件でなく詳細設計へ委譲する（要件定義 §17.3、§28）。

### 18.4 導入難度の規模非依存

プロジェクト規模が大きいこと自体とは別の理由で導入難度が構造的に増大する設計を避ける。これは強い不変条件ではなく設計原則とし、物量増加に伴う処理量・作業量の増加は許容する（要件定義 §17.4）。

---

## 19. トレーサビリティと役割別 projection

関係型を単一へ潰さず、横断してトレース可能にする（要件定義 §3.4、NFR-003）。

- 文書間は derives_from、VO → Test は covers、Test ↔ 実装は検証対象／実装 traceability のように性質の異なる関係型を区別する。関係型そのものの意味論的増殖は求めない。
- 契約上必須と定義したリンク（`parent --relation--> child`）は必須とし、任意（optional）と定義した関係（例：§9.3 実装 traceability）は欠落してよい。存在するリンクに付す説明文・導出理由は任意とし、空でも `chain_integrity` 違反・`MISMATCH` としない。ただし説明文を付加・保存できるデータ構造とする。
- 最小の意味単位「上流ノード → 関係 → 下流ノード」を**任意のノードから取得**でき、必要に応じて上流／下流へ連続して辿れ、プロジェクト全体のトレーサビリティ構造も取得できる。常に全チェーンを表示することは求めない。
- **役割ごとの参照観点**：同一のトレーサビリティ構造から、利用者の役割または利用目的に応じて参照対象・関係・集約粒度を変えた **projection** を取得・提示できる（例：PM は上位の document・VO の状態と未確定/NG、Tester は VO・Test・検証対象・Evidence・未実施/失敗理由、Coder は実装から関連 Test・VO・上流文書へのトレース）。役割を固定 enum やモード名として仕様化することは本書では行わず、preset・UI・モード体系は詳細設計へ委譲する（要件定義 §28）。

---

## 20. フェーズゲートと進行条件

プロジェクト側が登録したフェーズ・工程・ゲートの進行条件について、現在の検証状態（§4）と承認（§17）が通過条件を満たすかを**評価・提示できなければならない**（MUST。要件定義 §26.4）。検証状態と承認は独立の軸であり（§4.5）、ゲートは両者の組合せを進行条件にできる。ゲート条件の定義を受理できること。

```text
通常開発中   : verification = PASS で進行可、approval 不要
Release gate : verification = PASS + Reviewer approval
Delivery gate: verification = PASS + Owner / PM approval
```

**本システムの責務はゲート条件が現在満たされているかの評価・提示に限る。** フェーズのライフサイクル管理・工程の自動遷移は責務外とする（要件定義 §26.4、§25 OOS-004）。「Release フェーズへ遷移させる」のではなく「Release gate の条件を現在満たしている」を提示する。具体的なフェーズ名・承認ロール・必要承認数・権限 schema・進行条件定義は詳細設計へ委譲する（要件定義 §28）。

---

## 21. テスト実行と Execution Evidence

`vtest run` はテストを実際に実行し、判定権威（§7）であるランナーの結果を Evidence として記録する（要件定義 §6、§26.1）。Evidence には少なくとも次を含める。

- Test ID と実行結果（ランナーが報告した `PASS` / `FAIL`）。
- 実行した adapter ID。
- 実行時のリポジトリリビジョン（Git commit hash）と dirty フラグ。
- 現在の Test subject 全体の内容ハッシュ、および全宣言 target を解決した canonical Target Reference と implementation construct の内容ハッシュ。
- 実行時 HEAD revision、実行 adapter・runner・toolchain・実行影響 config、現在の実行可能状態を変えうる repository / local dependency 入力の完全な snapshot を束縛した Execution State subject。
- 実行日時と実行方式。
- `target_binding`（§5.3）の target 別結果と fail-closed 集約結果（実施した場合）。

### 21.1 Evidence の鮮度（ハッシュ束縛による設計制約）

検証時、Evidence は次をすべて満たす場合のみ有効とする（要件定義 §6）。鮮度は独立検査ではなく §6 のハッシュ束縛により満たす。

- Evidence 記録時の Test subject 内容ハッシュが現在と一致する。
- Evidence の target 参照集合が、現在の Test の宣言 target を解決した canonical Source Target 集合と重複なく一致する。
- Evidence 記録時の各 target 内容ハッシュが、現在解決される各 implementation construct の内容ハッシュと一致する。
- Evidence の adapter ID が現在の Test の execution adapter と一致する。
- Evidence 記録時の HEAD revision が特定され、現在の HEAD revision と一致する。
- Execution State subject が完全であり、現在再構築した Execution State subject と一致する（dirty 状態の source、target 外 helper、build script、local dependency、runner / toolchain / 実行影響 config の変更を含む）。

内容ハッシュ・Execution State subject・revision 条件を満たさない Evidence は `NO_EVIDENCE`（診断 STALE）とし、有効な `PASS` として扱わない。adapter が実行入力集合の完全性を証明できない場合は `UNKNOWN` とし、部分的 snapshot から現在実装への `PASS` を推測しない。Evidence が存在しても鮮度が満たされないなら、その Evidence から実行関連の判定を `PASS`/`FAIL` として再利用せず、同じ鮮度・対応関係の非 `PASS` 値を保持する。Evidence が存在しない場合は実行関連を `NO_EVIDENCE`（診断 NOT_EXECUTED）とする。Evidence reader は adapter ID を欠く互換 record も履歴として読み取れるが、現在の Test が `rust-cargo` で互換 runner 情報と内容ハッシュから Rust 実行と一意に確認できる場合に限り評価し、それ以外は `UNKNOWN` とする。

---

## 22. 完全検証・集約・報告

### 22.1 完全検証 OK

完全検証における OK は、宣言鎖全体に対する検査（`chain_integrity` / `orphan_detection`）と、scope に含まれる各「宣言 + コード + 証拠」の組に対する検査（`target_binding` / `oracle_presence`）がすべて `PASS` であり、テストランナーの結果を含む証拠が §6 を満たす場合に限る。一項目でも `FAIL` / `MISMATCH` / `NO_EVIDENCE` / `UNKNOWN` であれば NG（fail-closed。要件定義 §26.1、P-002）。

利用者向け簡易出力は `OK` / `NG` の二値とする。完全検証の検査集合はこの 4 検査に固定し、設定によって追加・削除できない。検査の部分集合を指定した実行は限定 scope であり、完全検証として表示・集約しない（§4.6）。

### 22.2 集約

Test 単位の結果を VO・Feature・document 単位へ集約可能とする。集約は fail-closed を基本とし、子に 1 つでも非 `PASS` があれば親は非 `PASS`（要件定義 §26.3）。集約時に複数の非 `PASS` 値が混在する場合、上位に表示する代表値の優先順位は `FAIL > MISMATCH > NO_EVIDENCE > UNKNOWN` とする（診断ラベルは代表値の順位に用いず、原因説明として併記する）。詳細出力では子の個別値をすべて確認できる。

### 22.3 報告

NG の場合、どのエンティティの・どの検査が・どの状態で・どの診断ラベルとともに落ちたかを掘り下げ可能とする（要件定義 §26.2、NFR-006）。

- **簡易出力**：総合 OK / NG。
- **詳細出力**：任意ノードからの局所／経路／全体トレース（§19）に沿ったツリー表示。非 `PASS` の根拠（判断記録・Evidence への参照）を辿れる。

`covers` を持つ Test は VO の子として表示する。管理下にある事実と、いずれの VO へも寄与しない事実の双方を出力から確認できる状態にする。人間向けテキストと機械可読 JSON の両方を出力できる（要件定義 NFR-007 / NFR-008）。

adapter 能力の欠落・失敗を `PASS` へ補完しない。static 解析または coverage 能力がなければ該当項目は `NO_EVIDENCE`（診断 NOT_CHECKED）、runner 能力がなければ実行関連は `NO_EVIDENCE`（診断 NOT_EXECUTED）、解析限界は `UNKNOWN` とする。create / edit / audit / run 等の明示的操作に必須の能力がなければ操作を失敗させ、ファイル・判断記録・Evidence を生成しない（要件定義 §21）。

---

## 23. スキャンと整合性検査

`vtest scan` は registry に登録された全 source discovery adapter へ委譲し、統合した discovery 結果と `.verify/` からエンティティと関係の全体グラフを再構築する。その過程で `chain_integrity` / `orphan_detection` を構成する整合性検査を行う（要件定義 §13、§23）。

- Test ID の重複（identity collision）→ `MISMATCH`。
- `covers` が存在しない VO を参照（dangling reference）→ `MISMATCH`。
- Test ID を宣言するが `covers` をどの VO も参照しない Test（orphan test）→ `MISMATCH`。すべての管理対象 Test に `covers ≥ 1` を一律要求する（§12）。
- VO の parent が存在しない、または循環している → `MISMATCH`。
- VO の `derives_from`（document 参照）が存在しない document を参照 → `MISMATCH`。
- document の derives_from が存在しない document を参照（文書鎖のリンク切れ）→ `MISMATCH`。
- 根に指定されない孤児 document（`orphan_detection`）→ `MISMATCH`。
- Relation の from / to が存在しないエンティティを参照 → `MISMATCH`。
- 恒久 SRC ID が adapter 境界を越えて重複 → `MISMATCH`。
- 必須 Test metadata の欠落 → `MISMATCH`。
- adapter が Test として発見したが管理宣言を持たない construct（unregistered test）→ 診断 severity は warning。ただし managed Test Entity へ対応しない事実は `chain_integrity`（`MISMATCH`/MISSING）へ反映する（§12）。

エラーは検証結果に反映され、該当エンティティの検査を非 `PASS` にする。診断 severity と検証状態を混同しない。content_hash 照合は決定論的に解決し、任意形式の文書本文から参照位置の存在を構文的に推測しない。参照位置の意味的妥当性・取り込み完全性は検査対象とせず、必要ならエスカレーション（§11）で扱う。

---

## 24. データ保存の基本方針

### 24.1 `.verify/` ディレクトリ

プロジェクトルート直下に `.verify/` を置き、テストコード外の正典と事実レコードを保存する。

```text
.verify/
  config.yaml        設定（正典）
  doc/               document レコード（正典）
  vo/                VO レコード（正典）
  rel/               外部 Relation レコード（正典・不変）
  forms/             Form Schema（正典）
  decisions/         判断記録（事実・追記型）
  approvals/         承認記録（事実・追記型）
  evidence/          実行証拠レコード（事実・追記型）
  cache/             派生情報（Git 管理外）
```

ファイル形式はすべて YAML とする。`cache/` 以外は Git 管理対象とする。

### 24.2 並列編集耐性の設計原則

要件定義 §23 への対応として次を原則とする（多数の AI Agent が並列で Test を追加・変更する前提）。

- **1 レコード＝1 ファイル**。全員が編集する中央共有台帳を持たない。
- document / VO は 1 エンティティ 1 ファイルとし、ファイル名を ID とする。異なるエンティティへの並列変更は異なるファイルへの変更になる。
- Relation・判断・承認・Evidence の各レコードは ULID をファイル名とする新規ファイル追加のみで作成し、既存ファイルの編集を伴わない。Relation レコードは**不変**とし、変更は「旧削除＋新追加」で表現する。
- 同一エンティティファイルへの並列変更が衝突した場合の解決は Git のマージに委ね、マージ後の論理的不整合（ID 衝突、dangling reference、承認の失効）はスキャンと整合性検査で検出する（§23）。
- record / エンティティファイルの書込みは**原子的に公開**し、読み手に書きかけの部分状態を観測させない。並列編集耐性は「公開されたファイルは常に完全である」ことを前提とし、部分書込みの検出・修復は行わない。

Test ID 衝突・dangling reference の検出、派生 index の再構築、Test と関連情報の同期を人間/Agent の記憶だけに依存させないことは §23 と §24.3 で担保する。具体的な物理保存方式は詳細設計へ委譲する（要件定義 §28）。

### 24.3 派生情報の再構築

検証グラフ、逆引きインデックス、集約結果はすべて正典からの導出物であり、`vtest scan` によりいつでも再構築できる（要件定義 NFR-004）。`cache/` が破損・削除されても正典は影響を受けない。キャッシュ / index の具体的データ形式は詳細設計へ委譲する（要件定義 §28）。

---

## 25. 利用者別ユースケース

要件定義 §20 の利用者ごとに想定する主経路を示す。具体的 role taxonomy・preset の固定は行わず、役割別の参照観点は §19 の projection として提供する。

- **Coder AI**：MCP 経由。担当した VO / Test を scope に指定して検証し、自身の変更が要求された検証を満たしたか確認する。
- **Developer**：CLI。Structured Test Operation によるテスト作成・変更、検証結果の詳細表示。
- **CI**：CLI（非対話）。`vtest verify` を同一 revision で再実行し、終了コードで判定する。Evidence を成果物として保存する。
- **Reviewer AI**：MCP 経由。Coder が提出した Evidence・判断記録と、自身の再検証結果を照合する。
- **PM / PM Agent**：CLI または MCP。document または VO 単位の集約結果から NG 箇所へ掘り下げる。

---

## 26. インターフェース概要

MCP を本体とし、CLI・CI は同じ検証の別入口とする。GUI は必須要件としない（要件定義 §22）。

### 26.1 CLI コマンド体系

コマンドの完全仕様（引数・出力・終了コード）は詳細設計で定める。本書ではコマンド一覧と責務を確定する。

| コマンド | 責務 |
|---|---|
| `vtest init` | `.verify/` の初期化 |
| `vtest scan` | スキャンと整合性検査、派生情報の再構築 |
| `vtest doc add / list / show` | document レコードの管理（derives_from・根指定を含む） |
| `vtest vo add / edit / list / show / expand / approve` | VO レコードの管理、組合せの実体化、承認 |
| `vtest test create / edit / show / list / query` | Structured Test Operation |
| `vtest audit static` | 決定論的解析（oracle_presence の不成立検出）の実行 |
| `vtest audit bundle / submit` | 判断記録（§11）の bundle 生成と結果提出 |
| `vtest run` | テスト実行と Evidence 記録 |
| `vtest verify` | 検証の実行（scope 指定可）と OK / NG 判定 |
| `vtest report` | 詳細レポート出力（ツリー／JSON） |
| `vtest doctor` | 整合性検査のみの実行 |

終了コードは `0`＝要求 scope が OK、`1`＝検証 NG、`2`＝入力・adapter 前提・capability 等による操作拒否、`3`＝内部エラーとする。検証状態と内部エラーは終了コードで分離する（§4.4）。CI はこの終了コードのみで判定できる。終了コード体系の詳細は詳細設計へ委譲する（要件定義 §5.4、§28）。

### 26.2 MCP ツール体系

MCP サーバは `vtest mcp` として起動し、CLI と同一のコア機能を呼び出す。ツールの完全な入出力スキーマは詳細設計で定める。

| MCP ツール | 対応機能 |
|---|---|
| `scan` | スキャンと整合性検査 |
| `doc_list` / `doc_get` / `doc_upsert` | document 管理 |
| `vo_list` / `vo_get` / `vo_upsert` / `vo_expand` / `vo_approve` | VO 管理 |
| `test_query` / `test_get` | Test 検索・逆引き |
| `test_create` / `test_edit` | Structured Test Operation |
| `form_get` | Form Schema の取得 |
| `audit_static` | 決定論的解析 |
| `audit_bundle` / `audit_submit` | 判断記録プロトコル |
| `run_tests` | テスト実行 |
| `verify` | 検証実行 |
| `report` | 詳細レポート取得 |

すべてのツールは非対話で完結する（要件定義 NFR-007）。CLI と MCP は同じ adapter registry composition・JSON envelope・adapter 選択エラーを利用し、MCP が CLI と異なる adapter を暗黙選択してはならない。CLI command 体系・MCP tool 体系の詳細は詳細設計へ委譲する（要件定義 §28）。

---

## 27. 対応範囲と adapter 境界

- 検証契約・ID・ハッシュ・Evidence・状態・集約の概念モデルは、言語および test runner に依存しない（要件定義 §21、R-3）。
- source discovery、決定論的解析、Structured Test Operation、test runner 起動、coverage 計測は adapter 能力として提供し、共通契約は特定言語の構文・構造を必須としない。
- core verifier を変更せずに別 adapter を登録できる境界を要求する。adapter 追加によって共通契約・スキーマが壊れないことを設計制約とする。
- 組込 production adapter は `rust-cargo` とし、Rust・Rust function unit test・小規模な integration test を対象とする。`rust-cargo` 以外の production language adapter は v0.1 の提供範囲に含めない（要件定義 R-2）。
- adapter が未登録・能力不足・解析不能の場合、検証結果を推測で `PASS` へ昇格してはならない。能力不足で確認できない項目は §8 条項 4 に従い扱う。

---

## 28. 非機能要求への対応方針

| NFR（要件定義 §24） | 対応 |
|---|---|
| NFR-001 並列性 | 1 レコード 1 ファイル、ULID ファイル名、不変 Relation、中央台帳の不在（§24.2） |
| NFR-002 再現性 | Evidence のリビジョン束縛（§21）、決定論的解析の再実行可能性、scan による全再構築 |
| NFR-003 追跡可能性 | document → VO → Test → SRC → Evidence の双方向グラフ、任意ノードからの局所／経路／全体取得（§19、§23） |
| NFR-004 再構築可能性 | 派生情報は cache のみ、正典から `vtest scan` で再構築（§24.3） |
| NFR-005 Fail Closed | 状態モデルと集約規則（§4、§22）、承認・判断の内容ハッシュ束縛（§11、§17） |
| NFR-006 説明可能性 | 状態・診断ラベルの分離（§4）、根拠を辿れる詳細レポート（§22.3） |
| NFR-007 自動化適性 | 非対話 CLI・MCP、JSON 出力、終了コード（§26） |
| NFR-008 人間可読性 | ツリー形式の詳細出力、ID の人間可読性（§3.2、§22.3） |

---

## 29. スコープ外

要件定義 §25 のスコープ外事項に対応する機能を本書では定義しない。

- **OOS-001 仕様書同士の品質監査**：文書層は §2.2 の通りリンクとハッシュのみを扱い、文書内容の意味的良否を検証しない。
- **OOS-002 修正方針決定**：不一致はどれを正とするか決めず状態として提示する（§4、P-001）。
- **OOS-003 通常ソースコード編集管理**：Test Edit 対象外の一般編集を管理しない（§15.3）。
- **OOS-004 開発プロセス全体の管理**：フェーズのライフサイクル管理・工程遷移は責務外（§20）。本システムは Verification Infrastructure として機能する。
- **OOS-005 宣言されていない実装**：v0.1 は宣言された義務の裏付けのみ検証し、宣言されていない実装の存在を関知しない（R-2）。実装レイヤーの孤児検出・シンボル列挙の定義・上流文書の意味構造は v0.2 のスコープとする。README に非関知宣言を一行入れる。

---

## 30. 詳細設計へ委譲する事項

以下は本書の要求・要件を基に詳細設計で決定する（要件定義 §28 の 23 項目に対応。HOW は本書で発明しない）。

1. 文書の具体的な入力フォーマットと登録方式（§16）。
2. 文書層の根の指定方式（orphan_detection の除外指定。§5.2）。
3. VO 保存形式（§10、§24.1）。
4. Test metadata の具体的 annotation syntax（`rust-cargo` の `@vtest.*` 文法を含む。§22）。
5. relation の保存形式（§3.2、§24.2）。
6. Test ID 命名規則（§3.2）。
7. Target Reference / SRC ID の具体的識別方式（§9.2）。
8. AST / LSP 等の具体的解析技術（不成立証明・存在確認・静的到達の実装。§5.5、§8.3）。
9. `oracle_presence` の信頼基盤の具体的範囲と委譲確認の方法（§8.2）。
10. `target_binding` の動的計測方式（§5.3）。
11. 診断ラベルの語彙（§4.2）。
12. 終了コード体系（検証状態と内部エラーの分離。§26.1）。
13. エスカレーション出力・判断記録・承認記録の具体的 schema（§11、§17）。
14. CLI command 体系（§26.1）。
15. MCP tool 体系（§26.2）。
16. キャッシュ / index の具体的データ形式（§24.3）。
17. 並列編集時の物理的保存方式（§24.2）。
18. 承認 workflow の具体的状態遷移（§17）。
19. 判断待ち情報（§18.3）の具体的な構造 schema と取得インターフェース。
20. 関係リンクの任意説明（§19）の保存形式。
21. 役割別 projection / view（§19）の preset・UI・モード体系。
22. approval authority（§17）の承認ロール・必要承認数・権限 schema。
23. フェーズ・ゲート（§20）の具体的なフェーズ名と進行条件定義。

これらの HOW を本書で確定しない。

---

## 付記（非規範）: トレーサビリティ表

本表は本書の各節が実現する凍結要件と、その導出区分（CONFORM＝旧版から生存し引用修復のみ／再導出＝旧構造を凍結モデルへ書き換え／新設＝旧版に無く凍結要件から新規）を記録する。全節が凍結要件へトレースできること、要件に親を持たない節を作らないことを設計制約とする。

| 本書の節 | 実現する凍結要件 | 区分 |
|---|---|---|
| §0 本書の位置付け | §0 前文・P-005・§22・§28 | 再導出（分冊構造・委譲参照の凍結連番化、AI 監査前提の除去） |
| §1 用語定義 | §1・§3・§5・§7・§9・§10・§12・§13・§14・§19 | 再導出（SPEC/REQ 型・role 語彙・12 項目/8 値の除去、総称 document・判断記録の導入） |
| §2.1 正典の三層構造 | P-003・§6・§12・§19・NFR-004 | CONFORM（事実層に判断記録を追加） |
| §2.2 宣言鎖と照合 | §3.1・§3.2 | 再導出（Specification→Requirement→…→Execution 鎖を document→document→VO→Test へ） |
| §2.3 導出できる関係は保存しない | P-003 | CONFORM |
| §2.4 adapter 設定と wire 互換 | §21・§28 | CONFORM |
| §3.1 エンティティ種別 | §3.2・§9・§12・§17・§19・§13 | 再導出（SPEC-/REQ- 廃止、DOC-・判断記録の新設） |
| §3.2 ID 規則と関係リンク | §3.4・§9.2・§28 | 再導出（関係リンクの任意説明を追加） |
| §3.3 Source Target の識別 | §9.2・R-3 | CONFORM |
| §4.1 状態は 5 つ | §5.1 | 再導出（8 値 → 5 状態、NO_EVIDENCE 新規） |
| §4.2 診断ラベル | §5.2・§28 | 新設（状態と別軸の診断ラベルを分離） |
| §4.3 状態の割当 | §5.3 | 再導出（凍結割当表の採用） |
| §4.4 UNKNOWN の検疫 | §5.4 | CONFORM |
| §4.5 検証状態と承認の分離 | §5.5 | 新設（承認前提を検証状態から分離） |
| §4.6 scope | P-002・§5.3 | 再導出（scope 外を NOT_CHECKED から NO_EVIDENCE/NOT_CHECKED へ） |
| §5.1 chain_integrity | §4.1・§13 | 再導出（複数検査項目の統合） |
| §5.2 orphan_detection | §4.2 | 新設（文書層孤児検出＋根指定） |
| §5.3 target_binding | §4.3・§7・§26.1 | 再導出（runtime_result を証拠側へ吸収、per-target 規則） |
| §5.4 oracle_presence | §4.4・§8 | 再導出（semantic_audit を排し不成立証明の 3 値へ） |
| §5.5 決定論的に検出可能な不成立構造 | §8.3 | CONFORM |
| §6 証拠 | §6 | 再導出（evidence_validity 検査を廃しハッシュ束縛の設計制約へ） |
| §7 判定権威 | §7・§26.1 | CONFORM（明示宣言を追記） |
| §8 Test の検証成立性 | §8・§9 | CONFORM（引用修復） |
| §9 検証対象と Source Target | §9 | 再導出（検証対象の一般化、traceability 分離） |
| §10 Verification Obligation | §10・§11 | 再導出（vo_decomposition 検査を排し表現能力のみへ） |
| §11 発見・意味判定のエスカレーションと判断記録 | §12・§19 | 新設＋再導出（bundle/submit を検証ゲートから切離し判断記録へ転用） |
| §12 Test Registry | §13・§4.1 | 再導出（role/anchor 機構を廃し covers ≥ 1 一律へ） |
| §13 Test Intent | §14 | 再導出（宣言鎖ノードから付随情報へ） |
| §14 Parameterized / Table-Driven Test | §15・R-3 | CONFORM |
| §15 Structured Test Operation | §16・P-004 | CONFORM（意味監査 → 判断記録へ用語修復） |
| §16 仕様入力（文書層） | §18 | CONFORM（総称 document へ用語修復） |
| §17 承認 | §19・§5.5 | 再導出（承認前提の分離、judgment reference・approval authority を明示） |
| §18 途中導入と既存プロジェクト対応 | §17・R-5 | CONFORM |
| §19 トレーサビリティと役割別 projection | §3.4・NFR-003 | 新設（任意説明・任意ノード取得・役割別 projection） |
| §20 フェーズゲートと進行条件 | §26.4 | 新設（評価・提示のみ、自動遷移は責務外） |
| §21 テスト実行と Execution Evidence | §6・§26.1 | CONFORM（鮮度をハッシュ束縛制約として再叙述） |
| §22 完全検証・集約・報告 | §26 | 再導出（4 検査 × 5 状態 × fail-closed、集約優先順位の診断ラベル混在解消） |
| §23 スキャンと整合性検査 | §13・§23 | 再導出（role/anchor 検査を除去、文書鎖・孤児検査を追加） |
| §24 データ保存の基本方針 | §23・§28 | CONFORM（.verify レイアウトを DOC-・判断記録へ更新） |
| §25 利用者別ユースケース | §20 | CONFORM（固定 role taxonomy を projection へ委譲） |
| §26 インターフェース概要 | §22・§28 | CONFORM（SPEC/REQ ツールを doc へ、意味監査を判断記録へ） |
| §27 対応範囲と adapter 境界 | §21・R-2・R-3 | CONFORM |
| §28 非機能要求への対応方針 | §24 | CONFORM（引用修復） |
| §29 スコープ外 | §25 | CONFORM（引用修復） |
| §30 詳細設計へ委譲する事項 | §28 | 再導出（18 項目 → 23 項目へ） |
