# AI並列開発向けテスト検証システム 基本仕様 v0.1

## 0. 本書の位置付け

本書は「AI並列開発向けテスト検証システム 要件定義・要件分解 v0.1」（以下、要件定義）の下流文書である。
要件定義が「何を保証しなければならないか」を定めたのに対し、本書はシステムが外部に対して保証する挙動、データモデル、状態モデル、インターフェースの範囲を確定する。
実装内部の構造、ファイルスキーマの全フィールド、アルゴリズム、コマンドの引数仕様は「詳細設計 v0.1」で定める。
詳細設計は本冊（コア設計 §1〜§11、§16〜§17、§19）、別紙A（CLI・MCPインターフェース仕様 §12〜§15）、別紙B（受入仕様 §18）の3分冊であり、節番号は全冊を通した連番である。本書からの「詳細設計 §n」の参照はこの連番を指す。

要件定義 §34 の委譲事項に対する設計責任の所在を §15 に示す。

本書で確定する前提は次のとおり。

- **ツール名**：`vtest`（バイナリ名・ディレクトリ名に使用する）
- **`vtest` 本体の実装言語**：Rust
- **組込対応対象**：`rust-cargo` adapterが扱うRustの関数単体テストおよび小規模な結合テスト（`#[test]` 属性を持つテスト関数）
- **インターフェース**：CLI と、AI Agent 向け MCP サーバ
- **AI監査の実行方式**：エージェント委譲方式とする。`vtest` 自身は LLM API を呼ばず、監査に必要な情報を構造化して出力し、外部のAIエージェントが判定結果を構造化して提出する（§7.3）

Rust固有処理は組込 `rust-cargo` adapterが所有する。CLI・MCP・検証coreはadapter registryを介して能力を選択し、coreの検証契約は言語・test runnerに依存しない。

---

## 1. 用語定義

- **Specification（仕様）**：実装より上流で定義された、期待される振る舞い・制約・インターフェースを表す成果物。要件定義書、基本仕様書、詳細設計書、API Schema、型・データ仕様などを含む。対象ソースコード自身の doc comment は含まない（要件定義 §24）。
- **Requirement（REQ）**：仕様から抽出された、検証対象となる要求の単位。階層構造を持てる。Feature 相当の中間まとめは REQ 階層の中間ノードとして表現する。
- **Verification Obligation（VO）**：独立して「この条件が成立するか」と検証可能な仕様上の命題（要件定義 §4.1）。階層構造を持てる。粒度は assert 文・テスト関数などのコード構文単位で決めない。
- **Test**：Test ID で識別されるテスト関数。VO の実装単位であり、VO と N:M の対応を持ちうる。
- **Test Intent**：Test が「何を検証するか」を、コードを読まずに判断できる形で表した宣言情報。テストコードのアノテーションとして記述する（§6）。
- **Source Target（SRC）**：テスト対象となる実装コード上の識別可能な地点。プロジェクト相対パスとシンボルの組（ロケータ）で識別する。
- **Execution Evidence**：テスト実行の事実の記録。結果、実行時のリポジトリ状態、対象コードの内容ハッシュを含む。
- **チェック項目**：完全検証を構成する個々の検証観点（§4.2 の11項目）。
- **チェック結果値**：各チェック項目が取る値（PASS / FAIL / MISMATCH / MISSING / NOT_CHECKED / NOT_EXECUTED / STALE / UNKNOWN）。
- **完全検証**：11のチェック項目すべてを対象とする検証。1項目でも PASS 以外があれば NG（fail-closed）。
- **scope**：利用者が検証対象とするチェック項目・エンティティ範囲の選択。scope を狭めても、対象外項目の状態は書き換えない。
- **正典（source of truth）**：ある事実を決定する唯一の記録。正典から導出できる情報は派生情報とし、独立して保存しない（要件定義 P-003）。
- **監査バンドル**：AI意味監査のために `vtest` が整形して出力する、判定に必要な情報一式（§7.3）。
- **Agent Form Engineering**：既知の作業手順・入力項目を持つ操作を、自由編集ではなく構造化された質問・入力・検証で行わせる方式（要件定義 P-004）。

---

## 2. 全体像

### 2.1 正典の三層構造

本システムは、扱う情報を三層に分ける。

```text
1. 宣言（declaration）
   テストコード内アノテーション（@vtest.*）と .verify/ 配下のレコードファイル。
   Git で管理される正典。

2. 実装（implementation）
   テストコード本体と対象ソースコード。
   Git で管理される正典。

3. 事実（evidence）
   監査結果・実行結果・承認の記録。
   .verify/ 配下の追記型レコードファイル。Git で管理される。

派生情報（検索インデックス、検証グラフ、集約結果）は上記から毎回再構築する。
派生情報は Git 管理しない。
```

source discovery、static audit、Structured Test Operation、test runner、coverageはadapter capabilityとして提供する。adapterが返す導出結果はregistryでmergeし、adapter ID、path、Test IDの順に正規化する。registryの重複ID、未登録adapter、またはadapter間のTest ID重複は操作エラーとし、空のscanとして成功扱いしない。

「宣言と実装が一致しているか」「事実が現在の宣言・実装に対して有効か」を照合するのが本システムの仕事である。
どれかを正として他を修正させることはしない（要件定義 P-001）。

### 2.2 検証チェーンと照合

要件定義 §3 の検証モデルをそのまま採用する。

```text
Specification
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

各成果物間の一致・不一致を独立に検証する。
単純な上流優先のチェーンではなく、不一致はどちらが正かを決めずに MISMATCH として提示する。

### 2.3 導出できる関係は保存しない

Test → VO（covers）、Test → SRC（target）の関係は、テストコードのアノテーションから決定論的に導出できる。
これらを外部ファイルへ重複保存しない。

外部レコードとして保存するのは、どちらか一方のエンティティに自然に所属しない関係（VO 間の依存、Test 間の補完関係など）だけとする。
この方針により、テスト編集時に同期しなければならない外部レコードを最小化する。

### 2.4 adapter設定とwire互換

`config.yaml` writerの正規形はversion 2とし、adapterごとにroot、scan、run設定をnamespace化する。readerはversion 1を単一の `rust-cargo` adapter設定としてin-memory変換して読み取るが、読み取りだけで正典を書き換えない。`vtest init`はversion 2を生成する。

adapter IDは設定内で一意でなければならず、同一adapter内のroot重複も拒否する。一方、polyglot repositoryを扱えるよう、異なるadapterが同じrootを走査することは許可する。未知のadapterやadapter固有設定の検証失敗は操作エラーとし、利用可能な言語や能力を推測して補完しない。

TestのJSON writerは、言語・runner非依存の `execution`（adapter、project、suite、opaque selector）と、互換field `filter`、`package`、`test_target` を出力する。Test入力に `execution` がない場合は、完全で相互整合するRust互換fieldからだけ `rust-cargo` の値を導出できる。導出に必要な値が欠落・矛盾する場合は、空値や推測値で実行可能として扱わない。

---

## 3. エンティティとID体系

### 3.1 エンティティ種別

| 種別 | ID接頭辞 | 正典の所在 | 説明 |
|---|---|---|---|
| Specification Source | `SPEC-` | `.verify/spec/` | 仕様文書への参照（パス＋内容ハッシュ） |
| Requirement | `REQ-` | `.verify/req/` | 要求ノード。階層可 |
| Verification Obligation | `VO-` | `.verify/vo/` | 検証命題。階層可 |
| Test | `TEST-` | テストコード内アノテーション | テスト関数 |
| Source Target | （IDなし／任意で `SRC-`） | ロケータで識別 | 対象実装。恒久IDは必須としない |
| Relation | `REL-`（ULID） | `.verify/rel/` | 外部関係レコード。不変 |
| Approval | ULID | `.verify/approvals/` | 承認レコード。追記型 |
| Audit Record | ULID | `.verify/audits/` | 監査結果レコード。追記型 |
| Execution Evidence | ULID | `.verify/evidence/` | 実行証拠レコード。追記型 |

### 3.2 ID規則

SPEC / REQ / VO / TEST の ID は人間可読な形式とし、利用者（人間またはAI）が命名する。

- 文字集合は `[A-Z0-9-]`、接頭辞は種別ごとに固定（`TEST-` 等）。
- 推奨形式は `TEST-<領域>-<連番>`（例：`TEST-PARSER-044`）だが、ツールは形式の推奨に従うことを強制せず、一意性のみを強制する。
- ID の一意性はスキャン時に全数検査し、衝突は整合性エラーとする（§7.1）。
- レコードファイル（REL、承認、監査、Evidence）の ID は ULID とし、ファイル名に用いる。ULID により並列生成時のファイル名衝突を実用上排除する。

### 3.3 Source Target の識別

ソースコードへ恒久IDを埋め込むことは必須としない（要件定義 §8）。
対象は**ロケータ**で識別する。

```text
<プロジェクト相対パス>::<ファイル内アイテムパス>

例：src/parser.rs::Parser::parse
```

ソース側アノテーション（`@vtest.src-id SRC-...`）で恒久IDを付与できる。恒久IDは必須ではなく、指定された場合だけscannerが認識する。

Test → SRC の対応はアノテーションから、SRC → Test の逆引きはスキャン結果から提供する（要件定義 §8）。

---

## 4. 状態モデル

### 4.1 チェック結果値

各チェック項目は次のいずれかの値を取る。

| 値 | 意味 | 完全検証でのOK可否 |
|---|---|---|
| `PASS` | 検証を実施し、成立を確認した | 可 |
| `FAIL` | 検証を実施し、不成立を確認した | 不可 |
| `MISMATCH` | 成果物間の不一致を検出した | 不可 |
| `MISSING` | 必要な成果物が存在しない | 不可 |
| `NOT_CHECKED` | 検証を実施していない | 不可 |
| `NOT_EXECUTED` | テストを実行していない | 不可 |
| `STALE` | 過去の記録はあるが、現在のコード状態に対する有効性を確認できない | 不可 |
| `UNKNOWN` | 判定不能 | 不可 |

`STALE` は、記録された内容ハッシュが検証対象の現在の内容ハッシュと一致せず、証拠を現在の `PASS` として利用できない状態を表す。
要件定義 §17.2 の非OK値集合に対する追加であり、OKへ向かう緩和ではない。

### 4.2 チェック項目

完全検証は次の11項目で構成する（要件定義 §17.2 に対応）。

| # | チェック項目キー | 内容 | 主な判定方式 |
|---|---|---|---|
| 1 | `spec_coverage` | 仕様（REQ）が VO へ対応付けられているか | 決定論（対応の存在）＋AI（網羅性は #3 へ） |
| 2 | `vo_decomposition` | VO 分解が構造的に妥当か（階層・参照の整合） | 決定論 |
| 3 | `vo_coverage` | VO 群が仕様を網羅しているか | AI監査（理由必須）＋承認 |
| 4 | `test_existence` | 各 leaf VO に対応する Test が存在するか | 決定論 |
| 5 | `static_audit` | Test が決定論的監査を通過したか | 決定論 |
| 6 | `semantic_audit` | VO・Test Intent・テストコードが同じ振る舞いを検証しているか | AI監査（理由必須） |
| 7 | `impl_consistency` | 仕様・VO・Test と対象実装が一致しているか | AI監査（理由必須）＋決定論（対象シンボル存在） |
| 8 | `test_execution` | Test が実際に実行されたか | 決定論 |
| 9 | `runtime_result` | 実行結果が PASS だったか | 決定論 |
| 10 | `target_execution` | 宣言された対象コードが実行経路へ入ったか | 決定論（カバレッジ計測） |
| 11 | `evidence_validity` | Evidence が現在の Test・対象実装・リビジョンに対して有効か | 決定論 |

### 4.3 総合判定

- 利用者向け簡易出力は `OK` / `NG` の二値とする（要件定義 §17.1）。
- **完全検証の OK** は、対象範囲内のすべてのチェック項目・すべてのエンティティが `PASS` である場合に限る。1項目でも非 PASS があれば NG（fail-closed。要件定義 P-002）。
- 下位から上位への集約（Test → VO → REQ）も fail-closed とする。子に1つでも非 PASS があれば親は非 PASS（要件定義 §19）。
- 集約時に複数の非 PASS 値が混在する場合、上位に表示する代表値の優先順位は `FAIL > MISMATCH > MISSING > STALE > NOT_EXECUTED > NOT_CHECKED > UNKNOWN` とする。詳細出力では子の個別値をすべて確認できる。

### 4.4 scope

利用者は検証 scope を次の2軸で限定できる（要件定義 P-002）。

- **チェック項目軸**：実施するチェック項目の部分集合を指定する（例：`spec_coverage` と `vo_coverage` のみ）。
- **エンティティ軸**：対象とする REQ / VO / TEST の部分木を指定する。

scope を限定した検証の OK は「要求された scope 内が OK」という意味に限られる。
scope 外の項目は `NOT_CHECKED` のまま保持し、PASS へ変換しない。
出力には要求 scope と、scope 外項目が未検証である旨を必ず併記する。

`target_execution` は完全検証ではデフォルト有効とする（要件定義 §16）。
高速な限定 scope 検証では省略でき、その場合は `NOT_CHECKED` として扱う。

---

## 5. データ保存の基本方針

### 5.1 `.verify/` ディレクトリ

プロジェクトルート直下に `.verify/` を置き、テストコード外の正典と事実レコードを保存する。

```text
.verify/
  config.yaml        設定（正典）
  spec/              SPEC レコード（正典）
  req/               REQ レコード（正典）
  vo/                VO レコード（正典）
  rel/               外部 Relation レコード（正典・不変）
  forms/             Form Schema（正典）
  approvals/         承認レコード（事実・追記型）
  audits/            監査結果レコード（事実・追記型）
  evidence/          実行証拠レコード（事実・追記型）
  cache/             派生情報（Git管理外）
```

ファイル形式はすべて YAML とする。
`cache/` 以外は Git 管理対象とする。

### 5.2 並列編集耐性の設計原則

要件定義 §29 への対応として、次を原則とする。

- **1レコード＝1ファイル**。全員が編集する中央台帳ファイルを持たない。
- SPEC / REQ / VO は 1 エンティティ 1 ファイルとし、ファイル名を ID とする。異なるエンティティへの並列変更は異なるファイルへの変更になる。
- Relation・承認・監査・Evidence の各レコードは ULID をファイル名とする新規ファイル追加のみで作成する。既存ファイルの編集を伴わない。
- Relation レコードは**不変**とする。変更は「旧レコード削除＋新レコード追加」で表現する。
- 同一エンティティファイルへの並列変更が衝突した場合の解決は Git のマージに委ね、マージ後の論理的不整合（ID衝突、dangling reference、承認の失効）はスキャンと整合性検査で検出する（§7.1）。

### 5.3 派生情報の再構築

検証グラフ、逆引きインデックス、集約結果はすべて正典からの導出物であり、`vtest scan` によっていつでも再構築できる（要件定義 NFR-004）。
`cache/` が破損・削除されても正典は影響を受けない。

---

## 6. テストメタデータ（アノテーション）

### 6.1 記述場所と正典性

Test のメタデータは、テスト関数直前の doc comment 内に `@vtest.` 名前空間のアノテーションとして記述する。
これが Test エンティティおよび Test 由来の関係（covers / target）の正典である。

```rust
/// @vtest.id TEST-PARSER-044
/// @vtest.covers VO-PARSER-003
/// @vtest.target src/parser.rs::Parser::parse
/// @vtest.intent 不正な UTF-8 入力を与えた場合、ParseError::InvalidUtf8 を返すことを検証する
/// @vtest.input 不正な continuation byte を含むバイト列
/// @vtest.expect ParseError::InvalidUtf8
/// @vtest.kind unit-error
#[test]
fn rejects_invalid_utf8() {
    // ...
}
```

### 6.2 キー一覧

| キー | 必須 | 意味 |
|---|---|---|
| `@vtest.id` | 必須 | Test ID。全体で一意 |
| `@vtest.covers` | 必須 | 検証する VO ID。複数指定可（N:M 対応。要件定義 §4.4） |
| `@vtest.target` | 必須 | 対象のロケータまたは SRC ID |
| `@vtest.intent` | 必須 | 何を検証するかの一文 |
| `@vtest.input` | 任意 | 入力条件 |
| `@vtest.expect` | 任意 | 期待結果 |
| `@vtest.kind` | 任意 | テスト種別（Form Schema の種別と対応） |
| `@vtest.case` | 任意・複数可 | table-driven test の代表ケース記述 |
| `@vtest.related` | 任意・複数可 | 外部 Relation に昇格しない軽量な関連 Test の参照 |

具体的入力値の記載は許容するが必須としない（要件定義 §9）。
構文の完全な文法は詳細設計 §4 で定める。

### 6.3 直接編集の扱い

アノテーションとテストコードの直接編集は禁止しない（要件定義 §22）。
公式経路として Structured Test Operation（§8）を提供し、直接編集された場合もスキャンと整合性検査が不整合（ID重複、dangling VO 参照、アノテーション欠落）を検出する。
アノテーションが正典であるため、直接編集と外部レコードの「同期漏れ」は covers / target については構造的に発生しない。

---

## 7. 検証機能の外部仕様

### 7.1 スキャンと整合性検査

`vtest scan` は、テストコードと `.verify/` を読み、エンティティと関係の全体グラフを再構築する。
その過程で次の整合性検査を行う。

- Test ID の重複（identity collision）
- `@vtest.covers` が存在しない VO を参照（dangling reference）
- `@vtest.id` を持つがどの VO も参照しない Test（orphan test）※警告
- VO の parent が存在しない、または循環している
- Relation の from / to が存在しないエンティティを参照
- 必須アノテーションキーの欠落
- `@vtest` アノテーションを持たない `#[test]` 関数（unregistered test）※警告

エラーは検証結果に反映され、該当エンティティのチェック項目を非 PASS にする。

### 7.2 決定論的テスト監査

明らかに無意味なテストを、AST 解析による決定論的ルールで検出する（要件定義 §11）。
少なくとも次を検出する。

- 定数のみの assertion（`assert!(true)` 等）
- 宣言された対象シンボルを呼び出していない
- 対象を呼び出しているが、その結果を一切検証していない
- 自己比較（`assert_eq!(x, x)` 等）
- 空のテスト本体

判定は保守的に行う。
決定論的に確定できる違反のみ `FAIL` とし、確定できないものは `UNKNOWN` として意味監査へ送る。
決定論的監査で `FAIL` となった Test は、意味監査へ送る前に拒否できる。

### 7.3 意味監査（エージェント委譲プロトコル）

静的監査で判断できない一致性は、LLM による意味監査で検証する（要件定義 §12）。
`vtest` 自身は LLM を呼ばない。
プロトコルは次の3段階とする。

```text
1. バンドル生成
   vtest が監査対象の情報一式（VO、Test Intent、テストコード、
   対象実装、関連テスト、既知 partition、過去の監査結果）を
   構造化 JSON として出力する。
   バンドルには対象の内容ハッシュとリビジョンを含める。

2. 判定
   外部のAIエージェントがバンドルを読み、判定する。

3. 提出
   エージェントが判定結果（verdict＋構造化された理由）を
   vtest へ提出する。vtest は次を検証して受理・拒否する。
   - バンドルとの対応
   - 対象の内容ハッシュが現在も一致しているか（不一致なら拒否）
   - verdict 値の妥当性
   - 理由の必須構造（空の理由は拒否）
```

受理された結果は監査レコードとして保存され、対象の内容ハッシュに束縛される。
対象（VO・Test・実装）が変更されると、監査レコードは自動的に `STALE` となる。

理由を伴わない判定は受理しない（要件定義 §6）。
決定論的監査結果とAI監査結果は区別して保存・提示する（要件定義 §12）。

監査種別は `test-semantic`（VO ↔ Test Intent ↔ テストコード）、`vo-coverage`（§7.4）、`impl-consistency`（§7.5）の3種とする。

`vtest` はLLM APIを直接呼び出さない。AI判定は上記のbundle生成・提出検証・保存の経路だけから受理する。

### 7.4 VO網羅性監査

VO 分解の網羅性は `vo-coverage` 種別の意味監査として実施する（要件定義 §6）。
判定結果には次の構造化された理由を必須とする。

- 根拠となった仕様箇所（SPEC ID＋節参照）
- 分解観点（どの軸・partition で分解したか）
- 対象外とした条件と、その根拠となる仕様箇所
- 必要に応じて具体例

`COMPLETE` 相当の判定であっても、理由の構造を欠く提出は受理されず、`vo_coverage` は非 PASS のままとなる。

### 7.5 実装一致検証

仕様・VO・Test と対象実装の一致は `impl-consistency` 種別の意味監査として実施する（要件定義 §13）。
決定論的に検証できる部分（対象シンボルの存在、シグネチャの取得）はバンドル生成時に検証し、シンボルが存在しなければ `MISSING` とする。
不一致は `MISMATCH` として提示し、どちらを修正すべきかは決定しない。

### 7.6 Partition と組合せ検証

VO には検証軸（dimension）と partition を定義できる（要件定義 §5.1）。
すべての VO への partition 定義は要求しない。

複数の検証軸を持つ VO には、必要な組合せ coverage の方針を宣言できる。

- `independent-axes`：各軸が独立に検証されればよい
- `full-product`：全直積の検証を要求する
- `explicit`：必要な組合せを明示列挙する

複数 VO を同時に検証する Test は、その存在だけでは各 VO の独立検証の完了とみなさない（要件定義 §5.3）。
宣言された coverage 方針に基づき、独立検証または組合せ空間の充足を判定する。

### 7.7 テスト実行と Execution Evidence

`vtest run` はテストを実際に実行し、結果を Evidence として記録する（要件定義 §14）。
Evidence には少なくとも次を含める。

- Test ID と実行結果（PASS / FAIL）
- 実行したadapter ID
- 実行時のリポジトリリビジョン（Git commit hash）と dirty フラグ
- テスト関数ソースと対象関数ソースの内容ハッシュ
- 実行日時と実行方式
- Target Execution Verification の結果（実施した場合）

### 7.8 Evidence の鮮度検証

検証時、Evidence は次の条件をすべて満たす場合のみ有効とする（要件定義 §15）。

- Evidence 記録時のテスト関数内容ハッシュが現在と一致する
- Evidence 記録時の対象関数内容ハッシュが現在と一致する
- Evidenceのadapter IDが現在のTestのexecution adapterと一致する
- リビジョンが特定できている（dirty 状態での実行は Evidence に明示され、完全検証では内容ハッシュ一致を必須とする）

内容ハッシュまたはrevision条件を満たさない Evidence は `STALE` とし、有効な PASS として扱わない。adapter IDが存在して現在値と不一致の場合は `MISMATCH` とする。
Evidence readerはadapter IDを欠くrecordも受理できる。現在のTestが `rust-cargo` で、互換runner情報と内容ハッシュからRust実行であることを一意に確認できる場合に限り互換Evidenceとして評価し、それ以外は `UNKNOWN` とする。

### 7.9 Target Execution Verification

宣言された対象コードが実際にテスト実行経路へ入ったことを、カバレッジ計測により確認する(要件定義 §16)。

- 完全検証ではデフォルト有効。
- 高速な限定 scope 検証では省略可能。省略時は `NOT_CHECKED`。
- 計測環境（カバレッジツール）が利用できない場合も `NOT_CHECKED` とし、PASS へ変換しない。
- 対象シンボルの実行回数が 0 の場合、`target_execution` は `FAIL` とする。

### 7.10 集約とレポート

検証結果は Test → VO → REQ の階層へ fail-closed で集約する（要件定義 §19）。
出力は次の2形式を提供する。

- **簡易出力**：総合 OK / NG
- **詳細出力**：要件定義 §18 の形式に準じたツリー表示。NG の場合、どのエンティティのどのチェック項目が、どの値で、どの根拠（監査レコード・Evidence への参照）により非 PASS かを掘り下げられる

人間向けテキストと機械可読 JSON の両方を出力できる（要件定義 NFR-007 / NFR-008）。

adapter能力の欠落または失敗を `PASS` へ補完しない。検証時にstatic auditまたはcoverage能力がなければ該当項目は `NOT_CHECKED`、runner能力がなければ実行関連項目は `NOT_EXECUTED`、解析限界は `UNKNOWN` とする。create / edit / audit / runなど明示的に要求された操作に必須の能力がなければ、操作を失敗させてファイル、Audit、Evidenceを生成しない。discoveryまたはrunnerの確定的失敗でも該当操作を失敗させ、Evidenceを生成しない。

---

## 8. Structured Test Operation

### 8.1 提供する操作

Test 操作の公式経路として、次の構造化操作を提供する（要件定義 §20）。

- **Create Test**：Form Schema に基づく構造化入力からテスト雛形とアノテーションを生成し、正しい位置へ挿入する
- **Edit Test**：Test ID を編集ハンドルとして、対象テストのアノテーションおよび関数本体を更新する
- **Query Test**：Test ID・VO・SRC ロケータ等からの検索と逆引き
- **Audit Test**：監査バンドルの生成と監査結果の提出（§7.3）

### 8.2 desired state 方式

Create / Edit の入力は差分操作ではなく**あるべき状態（desired state）**とする。
利用者は「TEST-X はこの状態である」を宣言し、`vtest` が現状との差分を計算してアノテーション更新・検証を行う。

### 8.3 入力検証

構造化入力の各項目は、可能な限り受理時に検証する（要件定義 §20）。

- 対象シンボルの存在（不存在なら候補を提示）
- 参照 VO / Test ID の存在
- Test ID の重複
- 期待値として指定された enum variant 等の存在（解決可能な範囲で）

### 8.4 編集境界

- 公式 Edit 操作の一回の対象は原則 1 Test とする（要件定義 §21）。`edit TEST-001` が他の Test を暗黙に変更することはない。
- 編集はテスト関数の doc comment と関数本体の範囲に限定する。
- Test 外部の通常ソースコード、helper、fixture の編集は責務外とし、操作を提供しない（要件定義 OOS-003）。

### 8.5 Form Schema

テスト種別ごとの質問・入力項目のテンプレートを **Form Schema** として `.verify/forms/` に定義できる。
Rust関数単体Test用と小規模結合Test用の組込schemaを同梱する。
CLI・MCP のいずれからも同一のスキーマを消化できる。

組込formは `rust-cargo` adapterが登録する。Form Schemaのkindとadapter namespaceは分離し、未知のformをcoreがRust用として推測してはならない。

---

## 9. 承認モデル

VO などの検証成果物の確定・承認状態を表現する（要件定義 §25）。

- VO の状態は `draft` / `approved` の2値とする。
- 承認は承認レコード（追記型）として記録し、**承認対象の内容ハッシュに束縛する**。承認後に VO が編集されると、承認レコードは現在の内容と一致しなくなり、VO は自動的に `draft` 相当（承認失効）として扱われる。fail-closed の一貫である。
- 承認主体は人間に限定しない。`human` / `agent` の種別と識別子（エージェント名・モデル名等）を記録する。
- 承認レコードには根拠（参照した監査レコードの ID 等）を記録でき、「誰が・何を・どの根拠で承認したか」を追跡できる。

---

## 10. 利用者別ユースケース

要件定義 §26 の利用者ごとに、想定する主経路を示す。

- **Coder AI**：MCP 経由。担当した VO / Test を scope に指定して検証し、自身の変更が要求された検証を満たしたか確認する。
- **Developer**：CLI。Structured Test Operation によるテスト作成・変更、検証結果の詳細表示。
- **CI**：CLI（非対話）。`vtest verify` を同一 revision で再実行し、終了コードで判定する。Evidence を成果物として保存する。
- **Reviewer AI**：MCP 経由。Coder の提出した Evidence・監査レコードと、自身の再検証結果を照合する。
- **PM / PM Agent**：CLI または MCP。REQ 単位の集約結果から NG 箇所へ掘り下げる。scope を `spec_coverage` / `vo_coverage` に限定した分解確認を行う。

---

## 11. インターフェース概要

### 11.1 CLI コマンド体系

コマンドの完全仕様（引数・出力・終了コード）は詳細設計 §12 で定める。
本書ではコマンド一覧と責務を確定する。

| コマンド | 責務 |
|---|---|
| `vtest init` | `.verify/` の初期化 |
| `vtest scan` | スキャンと整合性検査、派生情報の再構築 |
| `vtest spec add / list / show` | SPEC レコードの管理 |
| `vtest req add / edit / list / show` | REQ レコードの管理 |
| `vtest vo add / edit / list / show / expand / approve` | VO レコードの管理、組合せの実体化、承認 |
| `vtest test create / edit / show / list / query` | Structured Test Operation |
| `vtest audit static` | 決定論的監査の実行 |
| `vtest audit bundle / submit` | 意味監査バンドルの生成と結果提出 |
| `vtest run` | テスト実行と Evidence 記録 |
| `vtest verify` | 検証の実行（scope 指定可）と OK / NG 判定 |
| `vtest report` | 詳細レポート出力（ツリー／JSON） |
| `vtest doctor` | 整合性検査のみの実行 |

終了コードは `0`＝要求 scope が OK、`1`＝NG、`2`＝入力・使用方法エラー、`3`＝内部エラーとする。
CI はこの終了コードのみで判定できる。

### 11.2 MCP ツール体系

MCP サーバは `vtest mcp` として起動し、CLI と同一のコア機能を呼び出す。
ツールの完全な入出力スキーマは詳細設計 §13 で定める。

| MCP ツール | 対応機能 |
|---|---|
| `scan` | スキャンと整合性検査 |
| `spec_list` / `spec_get` | SPEC 参照 |
| `vo_list` / `vo_get` / `vo_upsert` / `vo_expand` / `vo_approve` | VO 管理 |
| `req_list` / `req_get` / `req_upsert` | REQ 管理 |
| `test_query` / `test_get` | Test 検索・逆引き |
| `test_create` / `test_edit` | Structured Test Operation |
| `form_get` | Form Schema の取得 |
| `audit_static` | 決定論的監査 |
| `audit_bundle` / `audit_submit` | 意味監査プロトコル |
| `run_tests` | テスト実行 |
| `verify` | 検証実行 |
| `report` | 詳細レポート取得 |

すべてのツールは非対話で完結する（要件定義 NFR-007）。

CLIとMCPは同じadapter registry composition、JSON envelope、adapter選択エラーを利用する。MCPがCLIと異なるadapterを暗黙に選択してはならない。

---

## 12. 既存プロジェクト対応

既存のSpecification、Source、Testを持つプロジェクトに対し、次を保証する（要件定義 §23）。

- `vtest init`は `.verify/` を作成し、既存コードを変更しない。
- `vtest scan`は発見した未登録Testを未登録として報告する。
- SPEC / REQ / VO、Test annotation、Audit、Evidenceの一部が欠ける状態も読み取り可能とする。
- `vtest verify`は正典または検証事実の欠落を対応する非PASS値として表示する。
- 部分的な登録・監査・実行状態を総合 `OK` として扱わない。

---

## 13. 非機能要求への対応方針

| NFR | 対応 |
|---|---|
| NFR-001 並列性 | 1レコード1ファイル、ULID ファイル名、不変 Relation、中央台帳の不在（§5.2） |
| NFR-002 再現性 | Evidence のリビジョン束縛（§7.7）、決定論的監査の再実行可能性、scan による全再構築 |
| NFR-003 追跡可能性 | REQ → VO → TEST → SRC → Evidence の双方向グラフ（§7.1、§7.10） |
| NFR-004 再構築可能性 | 派生情報は cache のみ、正典から `vtest scan` で再構築（§5.3） |
| NFR-005 Fail Closed | 状態モデルと集約規則（§4）、承認・監査の内容ハッシュ束縛（§7.3、§9） |
| NFR-006 説明可能性 | 理由必須の監査プロトコル（§7.3、§7.4）、詳細レポート（§7.10） |
| NFR-007 自動化適性 | 非対話 CLI・MCP、JSON 出力、終了コード（§11） |
| NFR-008 人間可読性 | ツリー形式の詳細出力、ID の人間可読性（§3.2、§7.10） |

---

## 14. 要件対応表

要件定義の各章が本書のどこで具体化されたかを示す。

| 要件定義 | 本書 |
|---|---|
| §1 目的 / §35 製品定義 | §0、§2 |
| P-001 検証機であり修正方針決定機ではない | §2.1、§7.5 |
| P-002 Fail Closed | §4.3、§4.4 |
| P-003 単一の正典 | §2.1、§2.3、§5.3 |
| P-004 Agent Form Engineering | §8 |
| §3 検証モデル | §2.2 |
| §4 Verification Obligation | §1、§3.1、§7.6 |
| §5 検証空間と網羅性 | §7.6 |
| §6 VO網羅性のAI検証 | §7.4 |
| §7 Test Registry | §3、§6、§7.1 |
| §8 Source Target | §3.3 |
| §9 Test Intent | §6.2 |
| §10 Parameterized Test | §6.2（`@vtest.case`）、§7.6 |
| §11 Test静的監査 | §7.2 |
| §12 Test意味監査 | §7.3 |
| §13 対象実装との一致検証 | §7.5 |
| §14 Test Execution | §7.7 |
| §15 Evidenceの鮮度 | §7.8 |
| §16 Target Execution Verification | §7.9 |
| §17 OK / NG | §4 |
| §18 詳細出力 | §7.10 |
| §19 上位への集約 | §4.3、§7.10 |
| §20 Structured Test Operation | §8 |
| §21 Test編集の境界 | §8.4 |
| §22 直接編集 | §6.3 |
| §23 既存プロジェクト対応 | §12 |
| §24 仕様入力 | §1、§3.1 |
| §25 承認 | §9 |
| §26 利用者 | §10 |
| §27 対応範囲 | §0 |
| §28 インターフェース | §11 |
| §29 並列AI開発への要求 | §5.2、§7.1 |
| §30 非機能要求 | §13 |
| §31 スコープ外 | §8.4（OOS-003）ほか、本書はスコープ外事項の機能を定義しない |

---

## 15. 委譲事項対応表

要件定義 §34 の18項目に対する設計契約または責任の所在を示す。

| # | 項目 | 設計契約・責任箇所 |
|---|---|---|
| 1 | Specification の入力フォーマット | 本書 §3.1：SPEC レコードによる参照方式。文書自体の形式は問わない |
| 2 | VO 保存形式 | 本書 §5.1：1 VO 1 YAML ファイル。スキーマは詳細設計 §3 |
| 3 | Test metadata の annotation syntax | 本書 §6：`@vtest.*`。文法は詳細設計 §4 |
| 4 | relation の保存形式 | 本書 §2.3、§5.2：導出可能関係は保存しない。外部関係は 1 ファイル 1 レコード |
| 5 | Test ID 命名規則 | 本書 §3.2 |
| 6 | Source symbol の識別方式 | 本書 §3.3：ロケータ方式 |
| 7 | AST/LSP 等の解析技術 | 詳細設計 §5：`rust-cargo` adapterはsynによるAST解析を行う。LSPは提供範囲外 |
| 8 | LLM provider / model | 本書 §7.3：エージェント委譲によりツールはプロバイダ非依存 |
| 9 | AI監査の prompt / skill / agent 構成 | ツールの責務外。バンドルと提出のプロトコルのみ規定（§7.3）。参考プロンプトを詳細設計 §8 に添付 |
| 10 | coverage reason の schema | 本書 §7.4。スキーマは詳細設計 §8 |
| 11 | evidence の revision 識別方式 | 本書 §7.7、§7.8：Git commit hash＋dirty フラグ＋内容ハッシュ |
| 12 | CLI command 体系 | 本書 §11.1。詳細は詳細設計 §12 |
| 13 | MCP tool 体系 | 本書 §11.2。詳細は詳細設計 §13 |
| 14 | キャッシュ / index のデータ形式 | 詳細設計 §2：実行ごとにインメモリ再構築し、永続cacheを正典として使用しない |
| 15 | 並列編集時の物理的保存方式 | 本書 §5.2 |
| 16 | 境界値・partition を必須入力にする Test 種別 | 組込Formでは必須とする種別を設けない。user-defined Form Schemaは `required` で指定できる（詳細設計 §14） |
| 17 | AI監査の多重度 | 運用ポリシーとしツールは制限しない。監査レコードは複数保持でき、最新の有効レコードを判定に用いる（詳細設計 §8） |
| 18 | VO承認 workflow の状態遷移 | 本書 §9：`draft` / `approved` ＋内容ハッシュ束縛による自動失効 |

---

## 16. 詳細設計へ委譲する事項

- ファイルスキーマの全フィールド定義
- アノテーションの文法とパースエラーの扱い
- 決定論的監査ルールの個別仕様
- 監査バンドル・提出結果の JSON スキーマ
- テスト実行方式（cargo test の起動形態、結果パース）とカバレッジ計測方式
- CLI 引数・出力・MCP 入出力スキーマ
- クレート構成と依存方向
