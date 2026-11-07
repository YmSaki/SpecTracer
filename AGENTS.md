# AGENTS.md — SpecTracer 開発ルール

このファイルは、このリポジトリで設計、実装、レビューを行うすべての開発者と AI エージェントが従うルールです。
禁止事項の一覧ではなく、アーキテクチャ上の判断基準、責務の分割、ファイルの配置、依存関係の構築方法を、設計と実装でそのまま使える手順として定めます。
用語は専門用語と正典の語彙（`PASS`、`NO_EVIDENCE`、Verification Obligation など）をそのまま使います。言い換えによる意味のずれを避けるためです。
ルールを変えるときは、このファイルを更新する PR を出してから変えます。口頭やチャットだけで変えると、次のセッションが古いルールで作業するためです。

**参照の順序**

- 新機能の設計: 第 1 節から第 7 節を順に読み、第 7 節の手順で配置を決める。
- 新しい言語への対応: 第 9 節を読み、第 7 節の手順で配置を決める。
- 実装: 第 8 節から第 13 節を参照する。
- レビューと定期リファクタリング: 第 15 節の手順で、第 3 節から第 6 節と照合する。

## 1. 前提

- **プロダクト**: SpecTracer（コマンド名 `vtest`）は、仕様 → Verification Obligation（VO）→ Test → Source Target → Evidence のトレースを検証する、fail-closed の検証層です。用途は AI による並列開発で、「テストが通った」という主張が仕様に結び付いた証拠で裏付けられているかを判定します。**証明されていないものを `PASS` にしないことが必須要件**です。
- **仕様の正本**: `docs/canonical/specification.json`（スキーマ `docs/canonical/specification.schema.json`、層の定義 `docs/canonical/LAYERING.md`）。層の権威は `root`（Owner 裁定）→ `request` → `require` → `spec` → `detailed_spec` → `basic_design` → `design` の順に下がる。
    - 下位の層に規範を足すのは、上位の層がその規範を含むときだけにする。上流のない規範は導出を検査できないためです。下位の層が上位にない規範を必要としたら、その欠落を上流へ報告する。層どうしが矛盾したら上位の層に従い、矛盾を上流へ報告する。
    - `docs/canonical/export/*.md` は同じデータの生成物として読み、権威は JSON に置く。生成物は JSON より遅れることがあるためです。
    - `docs/archive/` は変換前の退役した原典です。規範は正典 JSON から引用し、`docs/archive/` にしかない主張は JSON で探すか、未解決として報告する。
    - 実装スケジュールや計画書（`docs/plans/`、`docs/*実装スケジュール*`）は、計画に必要なときの工程資料として使い、権威は上の文書に置く。スケジュールは製品の規範ではなく、それで上書きすると製品契約がずれるためです。
- **正典が沈黙しているとき**: 有効な規範文書どうしが食い違うか、必要な判断を決めていなければ、該当する節を正確に引用して、未解決の契約として報告する。判断は正典から、正典が沈黙していれば上流から得る。過去のファイル、テスト、実装の振る舞い、Git の履歴は「何が行われたか」を示すだけで「何が決まったか」を示さないので、それで欠落を埋めない。迷ったら `.claude/agents/judge.md` の判定役に聞き、判定役が昇格したものは Owner に報告する（`needs-owner`）。
- **条項 ID**: 正典の各規範は `REQ-`、`SPEC-`、`DS-`、`BD-`、`DES-` などの ID を持つ。コード、コメント、Issue、PR では条項 ID で参照する（第 10 節）。ID は改版で番号がずれる節番号と違い、同じ規範を指し続けるためです。
- **リポジトリ運用の正本**: Git Flow、ブランチ名、タグ、共有履歴の扱い、仕様変更の手順は `DEVELOPMENT.md` が唯一の正本です。このファイルでは要点と参照先だけを書き、規則そのものは `DEVELOPMENT.md` を読む（第 14 節）。二か所に規則を書くと食い違うためです。
- **文書の役割分担**:

    | 文書 | 置くもの |
    | --- | --- |
    | `AGENTS.md`（このファイル） | 設計と実装の判断基準、責務、依存、配置、規約、レビューの手順 |
    | `ARCHITECTURE.md` | 利用者と新規参加者向けの構成の説明 |
    | `DEVELOPMENT.md` | Git Flow、ブランチ、タグ、仕様変更の手順、開発ループ |
    | `TESTING.md` | ゲートの実行方法、受入テストの構成 |
    | `CONTRIBUTING.md` | 外部からの貢献の入口 |

    内容が食い違ったら、仕様は正典 JSON、Git 運用は `DEVELOPMENT.md`、設計規則はこのファイルを優先し、食い違った側を同じ PR か別 Issue で直す。
- **ライセンス**: MIT。依存ライブラリは MIT と互換のライセンスのものに限る。

### 1.1 アーキテクチャ上の制約となる製品不変条件

次の不変条件は例外のない製品要件です。以降の構造は、これらを特定のレイヤーや機構で保証する前提で決めています。

| 不変条件 | 内容 | 保証する機構 |
| --- | --- | --- |
| 4 つの検査 | 検証は `chain_integrity`、`orphan_detection`、`target_binding`、`oracle_presence` の 4 つだけで行う。文書の層、証拠の種類、診断、Judgment、Approval の規則を足しても、同じトレースの別の事例として扱い、検査を増やさない | `vtest-verify` の `ALL_CHECKS` と、検査ごとの評価関数 |
| 5 つの状態 | 検証状態は `PASS`、`FAIL`、`MISMATCH`、`NO_EVIDENCE`、`UNKNOWN` の 5 つだけ。`MISSING`、`NOT_EXECUTED`、`NOT_CHECKED`、`STALE` は状態に付く診断ラベルとして扱う。状態、診断ラベル、診断の重大度、操作エラー、集約の OK/NG、gate の充足は別のフィールドと概念に保つ | `vtest-model` の型。`CheckOutcome::state` と `CheckOutcome::labels` |
| fail-closed | 欠落、古い、曖昧、不完全、未対応、解析不能な入力には `PASS` 以外の状態を返す。`UNKNOWN` は決定論的な限界として使い、内部エラーとは分ける。操作の拒否と内部障害は 5 状態の外（エラーコードと終了コード）で報告する | `vtest-verify` の集約。第 6.2 節のエラー分類 |
| 静的解析の `PASS` / `FAIL` | 静的解析など決定論的な解析から `PASS` / `FAIL` を出すのは、該当する正典の規則がその結果を確立するときだけ。反証できないことは証明ではないためです | 各 adapter の capability と `vtest-verify` |
| スコープ外を `PASS` にしない | 要求スコープ外の作業は証拠がないので `PASS` に数えない。scope は 2 軸で、扱いが違う（DS-1104）。**検査軸**（`--items`）で省略した検査は、集約木の中に `NO_EVIDENCE` と `NOT_CHECKED` で残す（DS-1110）。**エンティティ軸**（`--doc` / `--vo` / `--test`）で要求外のエンティティは、集約木に入れず**集約の対象外**とし、正典の scope 表現（`scope.requested`、`scope.unverified_outside_scope`）で未検証と明示する（DES-548）。たとえば Test A だけを検証したとき、Test B を `NO_EVIDENCE` として集約に混ぜない。限定した結果を報告するときは、要求 scope と「scope 外は未検証」を出力の冒頭に示し、完全な検証ではないと書く（DS-1113） | `vtest-verify` の scope（`EntityScope`、`ScopeReport`）と集約木 |
| 汎用 `document` モデル | 上流の規範的成果物は、新しい文書種別も含めて汎用の `document` モデルと `derives_from` で表す。文書種別ごとの正典エンティティやスキーマは検査を増やすためです | `vtest-model::document`、`vtest-store` の document file |
| Judgment と Approval の独立 | Judgment と Approval は独立した領域で、記録しても検証状態を変えない。どちらも証拠ではないためです。Approval の正規 CLI は `vtest approval create` / `withdraw` / `show` で、`vtest vo approve` は同じ意味の別名 | `vtest-store::approval`、操作層の `approval` |
| 検証状態と gate の分離 | `--gate` では終了コード `0` と `ok: true` が gate の充足を表すことがある。実際の集約状態は別に必ず出力する | 操作層の `verify` と envelope |
| 正典と派生の分離 | グラフ、索引、静的解析結果、bundle、レポートは正典 record から再生成する。派生物がおかしく見えたら、古ければ再生成し、生成器が誤っていれば生成器を直し、正典の誤りが確認できたときだけ、その record を所有する操作で正典を直す | `vtest-store`（正典の読み書き）と、各操作での再構築 |
| 鮮度の束縛 | Evidence、Judgment、Approval の有効性は、正典設計が求める現在の subject と依存閉包の全体に束縛する。過去の record は履歴として読めるが、現在の閉包に束縛されたときだけ現在の証拠になる | `vtest-model` の content hash / subject hash |
| 言語固有処理の隔離 | 言語固有の発見、解析、編集、実行、カバレッジは adapter の capability の背後に置く。adapter や capability がないときに、core やワイヤーで言語固有の既定値を補わない。明示的な操作に必要な capability がなければ `E-ADAPTER-004` で何も変えずに拒否し、検証に必要な capability がなければ `NO_EVIDENCE` と `NOT_CHECKED` または `NOT_EXECUTED` を報告する | `vtest-adapter-api` の capability と `AdapterRegistry` |
| 互換 reader の正規化 | 互換 reader とワイヤー adapter のデータは、現在の正典モデルへ正規化され、現在のすべての不変条件を通った後でだけ結果に影響させる | `vtest-store` の reader、`vtest-adapter-api::wire` |
| CLI と MCP の同値 | CLI と MCP は非対話で、同じ入力に同じ JSON 結果（`ok` / `data` / `diagnostics`）を返す（DS-1563） | 共有の操作層（第 2.1 節）。MCP は第 2 の検証エンジンを持たない |
| 主張と証拠 | 振る舞いを「実装済み」「検証済み」「受入済み」「リリース可能」と書くのは、現在の実装の証拠が該当する正典契約を示すときだけ。証拠がなければ、何が示されていないかを書く | 第 14 節の Definition of Done、レビュー |
| 受入証拠の対応付け | 受入証拠の各 assertion は、現在の正典条項と observable に明示的に対応付けてから、現在の正典への適合と呼ぶ。過去や置き換え済みの受入証拠は回帰証拠としてだけ使う | `tests/ACCEPTANCE.md`、`docs/reports/adapter-acceptance-matrix.md` |

### 1.2 トレードオフの優先順位

設計上の選択肢で迷ったら、上位を優先します。

1. **fail-closed**: 未定義の状態、不正な入力、想定外のエラーでは、`PASS` にしない側、操作を拒否して何も変えない側に倒す。
2. **決定論と再現性**: 列挙順、環境、時刻、一時パスなどの非決定要因を制御し、同じ意味の入力からは同じ ID、ハッシュ、判定、出力順序を得る。出力順序を変えるときは、外部契約への影響を先に確認する。
3. **外部契約の維持**: CLI の引数、出力、終了コード、MCP の tool 名、スキーマ、envelope、record の形式と保存先を変えない。変える必要があれば仕様変更として扱う（第 14 節）。
4. **テスタビリティ**: 判定ロジックを副作用（ファイル、git、外部プロセス、時刻）から分離し、隔離した fixture で検証できるようにする（第 6.1 節）。
5. **シンプルさ**: 現時点で不要な汎用化や抽象化はしない（YAGNI）。ただし知識の重複は排除する（DRY、第 11 節）。

## 2. アーキテクチャ

### 2.1 アーキテクチャスタイル

Rust の workspace を、**依存が一方向のレイヤー構成**と、**言語固有処理を capability の背後に置く Ports and Adapters** で構成します。
判定ロジックは副作用を持たない関数に保ち、ファイル、git、外部プロセスの読み書きは操作の入口で順序づけます（Functional Core, Imperative Shell）。
言語ごとの差を core から切り離し、新しい言語を adapter の追加だけで扱えるようにし、検証ロジックを OS やツールチェインなしで単体テストできるようにするためです。

コードは次のレイヤーのいずれかに属します。依存は上のレイヤーから下のレイヤーへだけ向けます。

| レイヤー | 責務 | 配置 |
| --- | --- | --- |
| 入口 | 引数やリクエストを操作の呼び出しに変換し、結果を text、JSON、JSON-RPC に変換する。業務ルールを持たない | `crates/vtest-cli`（clap と text 描画）、`crates/vtest-mcp`（MCP stdio） |
| 操作層（Application layer） | 1 操作 = 1 関数。adapter registry の構成（コンポジションルート）、envelope、終了コード、エラーから診断コードへの写像を所有する | 現在は `crates/vtest-cli/src/ops/` と `crates/vtest-cli/src/adapters.rs`。#175 の D2（承認済み）で新 crate へ切り出す（第 2.4 節） |
| オーケストレーション | adapter への委譲、adapter 出力の検証、決定論的な統合、実行、4 検査の判定と fail-closed の集約 | `crates/vtest-scan`、`crates/vtest-exec`、`crates/vtest-verify` |
| 永続化 | 正典 record（`.verify/`）の append-only な読み書き、レイアウト、設定の読み込み | `crates/vtest-store` |
| adapter 契約 | 言語・runner に依存しない capability の trait、DTO、`AdapterRegistry`、ワイヤー形式 | `crates/vtest-adapter-api` |
| adapter | 言語とツールチェインに固有の発見、解析、実行、カバレッジを capability として実装する | `crates/vtest-adapter-rust`、`crates/vtest-adapter-lexical`、`crates/vtest-adapter-synthetic`（受入テスト専用） |
| モデル | 共有のエンティティ、ID、ハッシュ、診断、検証状態、プロトコルの型 | `crates/vtest-model` |

### 2.2 入口と実行形態

| 入口 | 責務 | 備考 |
| --- | --- | --- |
| `vtest <subcommand>` | 非対話の CLI。`init`、`scan`、`doctor`、`run`、`verify`、`approval`、`doc` などを操作層に委譲する | `--format json` で操作層と同じ JSON envelope を出す |
| `vtest mcp` | MCP サーバーを stdio で起動する。tool 名は正典（別紙A の tool 一覧）の名前をそのまま使う | 各 tool handler は CLI と同じ操作層の関数を呼ぶ |

- 実行ファイルは `vtest` の 1 つだけです。`vtest mcp` を同じバイナリで提供するため、現在は `crates/vtest-cli/src/main.rs` が `#[path]` で `vtest-mcp` のソースを取り込んでいます。これは `vtest-mcp` が `vtest-cli` に依存するサイクルを避けるための暫定措置で、操作層の切り出し（第 2.4 節）で削除します。新しいコードでこの方式を真似しない。
- どの入口も対話入力を求めない。AI エージェントと CI から同じ結果で呼べるようにするためです。

### 2.3 データの流れと副作用

```text
.verify/（正典 record）  ──読み込み──▶  vtest-store  ──▶  操作層
ソースコード            ──adapter──▶   vtest-scan   ──▶  操作層
runner / coverage       ──adapter──▶   vtest-exec   ──▶  vtest-store（Evidence を追記）
                                       vtest-verify ◀──  scan 結果、record、Evidence 文脈
```

- 正典 record は `.verify/` に 1 record 1 ファイル（仕様が定める場合）で置き、追記と不変を基本にする。関係の変更は、古い関係の削除と新しい関係の追加で表す。
- 索引、グラフ、集約結果は毎回 record から再構築する。`.verify/cache/` は再構築可能な出力として扱い、record と食い違ったら record から再構築する。
- 副作用（ファイルの書き込み、`git rev-parse` や `cargo` などの外部プロセス）は操作の入口とその直下で行い、判定と集約の関数には読み取った値を渡す。拒否された操作とエラーでは、設定、Evidence、Approval を一切変えない（失敗時の原子性）。

### 2.4 操作層の切り出し（#175 D2、承認済み）

#175 の D2 は 2026-11-07 に Owner が承認しました。現在 `vtest-cli` にある操作層を新しい crate に移し、`vtest-cli` と `vtest-mcp` の両方がその crate に依存する形にします。

- **crate 名は未決**です。候補は `vtest-ops`（既存の `ops` module 名と呼び出し箇所を保てる）と `vtest-app`（Application layer の直訳）。決まるまで、このファイルでは「操作層 crate」と書く。決まったら、このファイルの表を同じ PR か直後の PR で更新する。
- 移すもの: `vtest-cli/src/ops/**`、`vtest-cli/src/adapters.rs` の `builtin_registry`、envelope、終了コード、エラーから診断コードへの写像。
- 移した後の形: `vtest-cli` は「clap の解析 → 操作層の呼び出し → text / JSON の印字 → 終了コード」だけ、`vtest-mcp` は「JSON-RPC の transport、tool スキーマ、引数の検証 → 操作層の呼び出し → envelope」だけにする。`vtest` バイナリは `vtest-cli` と `vtest-mcp` に依存し、`main.rs` の `#[path]` 取り込みと `#[allow(clippy::all)]` をなくす。
- 切り出しまでの間に操作を追加、変更するときも、最初から「1 操作 = `ops` の 1 関数、CLI と MCP は変換だけ」の形で書く。切り出しが純移動で済むようにするためです。
- 具体的な移動表と順序は #175 の §8〜§10 が正本です。

## 3. 責務の分割

### 3.1 分割の判断基準

crate、module、ファイルの境界は、次の問いに順に答えて決めます。

1. **特定の言語、ツールチェイン、テストフレームワークに依存するか。** 依存する部分は adapter に置き、core からは `vtest-adapter-api` の capability 経由でだけ使う。
2. **副作用（ファイル、git、外部プロセス、時刻）を持つか。** 持つ部分は操作の入口側に置き、判定、集約、変換は入出力が明確な関数にする。
3. **変更の理由は何か（単一責任の原則、SRP）。** 変更の理由が 2 つあれば分割する。例: record の YAML 形式の変更と、Evidence の鮮度判定の変更の両方で同じ関数を修正するなら、分割が誤っている。
4. **外部入力をパースするか。** パースとバリデーションは信頼境界の 1 か所（`vtest-store` の reader、`vtest-adapter-api::wire`、MCP の引数検証、adapter の構文解析）に集約し、内側には検証済みの型だけを渡す（Parse, don't validate）。
5. **正典の record か、派生物か。** 正典の読み書きは `vtest-store` に置き、派生物は毎回再構築する関数として置く。

### 3.2 トップレベルのディレクトリ

```text
crates/          Rust workspace の crate（3.3）
docs/canonical/  仕様の正本（specification.json とスキーマ、層の定義、生成物 export/）
docs/adapters/   言語 adapter の利用者向け説明（1 言語 1 ファイル）
docs/plans/      計画と工程資料（規範ではない）
docs/reports/    調査と監査の報告
docs/archive/    退役した変換前の原典
reports/         過去の調査報告
tests/           リポジトリ共通の fixture と受入対応表（ACCEPTANCE.md）
.verify/         このリポジトリ自身の正典 record（自己検証用）
.agents/skills/  検証、レビュー、リリース判定のプロジェクト Skill
.github/         CI、Issue と PR のテンプレート、spec-guard スクリプト
plugins/         Claude Code 用の開発プラグイン（hook と Skill）
```

| ディレクトリ | 置くもの | 置かないもの |
| --- | --- | --- |
| `crates/` | 製品コードと、その単体テスト、統合テスト | 仕様の規範、工程資料 |
| `docs/canonical/` | 仕様の正本 | 実装の都合で決めた規則（ADR 相当は Issue の `kind/decision` で Owner に判断を求める） |
| `docs/plans/`、`docs/reports/` | 計画、調査結果、レビューの経緯 | 規範。ここにしかない規則は正典に上げるか未解決として扱う |
| `.verify/` | 正典 record | テストの出力。CLI を試すときは隔離したプロジェクトを使う |
| `tests/fixtures/` | 複数のテストが共有する入力 | 特定のテストだけの一時データ（テスト内で生成する） |

### 3.3 crate の責務

crate の追加や責務の変更は、先にこの表を更新する PR で行います（第 7 節）。

| crate | レイヤー | 責務 | 持たない責務（担当） |
| --- | --- | --- | --- |
| `vtest-model` | モデル | エンティティ、ID、content hash と subject hash、診断、検証状態、`JsonEnvelope` と `ExitCode` | I/O、言語固有の解釈 |
| `vtest-store` | 永続化 | `.verify/` のレイアウト、設定（v1 / v2）の読み込み、正典 record（document、VO、Test、Approval、Evidence、Execution State、Form）の読み書きと検証 | 派生の索引（呼び出し側が毎回再構築）、言語固有の設定値の意味づけ（adapter） |
| `vtest-adapter-api` | adapter 契約 | capability の trait（source discovery、runner、coverage、static analysis など）、hash 未計算の DTO、`AdapterRegistry`、Test のワイヤー形式。`vtest-model` 以外に依存しない | 言語の実装（各 adapter）、materialization（`vtest-scan`） |
| `vtest-adapter-rust` | adapter | `rust-cargo` adapter。`syn` による Rust の解析、`Cargo.toml` と module path の解決、doc comment の `@vtest.` 宣言、`cargo test` の実行、`cargo llvm-cov` のカバレッジ、oracle の静的判定 | 検証の判定（`vtest-verify`） |
| `vtest-adapter-lexical` | adapter | Rust 以外の言語のフレームワーク非依存な Test 発見。言語ごとの module（`c`、`cpp`、`csharp`、`go`、`java`、`javascript`、`kotlin`、`php`、`python`、`ruby`、`swift`）と、意味が同じと確認できた部分（`annotation`、`shared`）。`source_discovery` だけを宣言する（実験的、`docs/adapters/lexical.md`） | runner、coverage、static analysis（未宣言のまま `NO_EVIDENCE` にする） |
| `vtest-adapter-synthetic` | adapter | `.rs` 以外の source、関数ではない Test construct などを core の変更なしで扱えることを示す受入テスト専用の adapter（DS-1584） | 本番の言語対応。本番の registry に登録しない |
| `vtest-scan` | オーケストレーション | adapter への委譲、Test ID の大域的一意性（E-SCAN-002）、VO 参照の解決（E-SCAN-003）、Target Reference の解決、adapter ID 順の決定論的統合、record の参照整合性、Structured Test Operation の orchestration | 言語構文の解釈（adapter）、opaque locator の分解 |
| `vtest-exec` | オーケストレーション | runner の起動、target coverage の帰属、Evidence の構築と追記 | 正典 record の形式（`vtest-store`。#175 R9 で Evidence の writer を移す） |
| `vtest-verify` | オーケストレーション | 検証グラフの再構築、4 検査、Evidence の鮮度、スコープ、fail-closed の集約 | ファイルや git の読み取り（`verify_project` の入口に集める）、record の形式 |
| 操作層（現 `vtest-cli/src/ops/` と `adapters.rs`） | 操作層 | 1 操作 = 1 関数、builtin registry の構成、envelope、終了コード、診断コードの写像 | 印字（`vtest-cli`）、JSON-RPC（`vtest-mcp`） |
| `vtest-cli` | 入口 | clap の引数定義、操作層の呼び出し、text 描画、出力、終了コード。`vtest` バイナリ | 業務ルール、エラーの写像（操作層） |
| `vtest-mcp` | 入口 | MCP stdio の transport、tool 一覧と入力スキーマ、引数の検証、操作層の呼び出し | 業務ルール、`vtest-scan` や `vtest-store` の直接呼び出し（#175 R3 で解消する） |

### 3.4 module とファイルの粒度

- 「別の理由で変わるものを別のファイルに」を唯一の基準にする（#175 D9）。行数の削減や「1 関数 1 ファイル」を目的にしない。
- 1,000 行を超える同居テスト（`mod tests`）は `tests.rs`（`#[cfg(test)] mod tests;`）へ移す。移動は純移動の PR で行う。
- ファイルが 1,000 行、関数が 100 行を超えそうになったら、3.1 の基準で分割を見直す。数値は閾値ではなく、複数の変更理由が混ざっていないかを確認する契機として使う。
- 公開 API（`pub`）は最小限にする。crate の外から使われない項目は `pub(crate)` にする。テストのためだけに公開しない。

## 4. 依存関係の構築

### 4.1 依存の向きの決め方

- **依存は上のレイヤーから下のレイヤーへ向ける。** モデルは何にも依存せず、adapter は `vtest-adapter-api` と `vtest-model` にだけ依存する。
- **言語固有の処理が core で必要になったら、`vtest-adapter-api` に capability を定義し、adapter が実装する。** 例: Structured Test Operation の構文の問いは `StructuredTestAdapter` として adapter へ移す（#73、PR #155）。core が `syn` や特定の adapter crate を直接使うことはない。
- **adapter crate に依存してよいのは操作層（コンポジションルート）だけ。** オーケストレーション、永続化、モデルから adapter crate への通常依存は作らない。テストで adapter を使うときは dev-dependency にする。
- **入口どうしは依存しない。** `vtest-cli` と `vtest-mcp` は操作層に依存し、互いには依存しない（#175 D2 の切り出し後。バイナリだけは両方に依存する）。
- **汎用の trait、factory、plugin 機構を、将来の拡張のためだけに作らない。** 必要になった時点で、必要な形だけを作る。

### 4.2 crate 間の依存の許可リスト

表にない依存は作りません。新しい依存が必要になったら、4.1 の基準で向きを確認し、この表を更新する PR を先に出します。

| crate | 依存してよい crate | 現状との差（解消先） |
| --- | --- | --- |
| `vtest-model` | なし | |
| `vtest-store` | `vtest-model` | |
| `vtest-adapter-api` | `vtest-model` | |
| `vtest-adapter-rust`、`vtest-adapter-lexical`、`vtest-adapter-synthetic` | `vtest-adapter-api`、`vtest-model` | |
| `vtest-scan` | `vtest-adapter-api`、`vtest-store`、`vtest-model` | `vtest-adapter-rust` への通常依存が残っている（#155、#175 R6 で dev-dependency にする） |
| `vtest-exec` | `vtest-adapter-api`、`vtest-store`、`vtest-model` | |
| `vtest-verify` | `vtest-scan`、`vtest-adapter-api`、`vtest-store`、`vtest-model` | `vtest-exec` へは依存しない（git HEAD の読み取りは #175 R9 で 1 か所にする） |
| 操作層 crate（名称未決） | オーケストレーション、永続化、adapter 契約、すべての adapter、モデル | 現在は `vtest-cli` の中にある |
| `vtest-cli` | 操作層 crate、`vtest-model`。バイナリは `vtest-mcp` も | 現在は操作層を内蔵し、オーケストレーションと adapter に直接依存する（#175 R2） |
| `vtest-mcp` | 操作層 crate、`vtest-model` | 現在は `vtest-cli`、`vtest-scan`、`vtest-store`、`vtest-exec`、`vtest-verify` に依存する（#175 R2、R3） |

- 依存は各 crate の `Cargo.toml` に宣言する。外部ライブラリは workspace の `Cargo.toml` の `[workspace.dependencies]` で版を固定し、各 crate は `.workspace = true` で参照する。
- 依存の検査は `cargo metadata` を使う architecture test で行う（#175 D1、R14）。導入されるまでは、レビューで `Cargo.toml` の差分をこの表と照合する。
- この表を変えるときは、architecture test も同じ PR で更新する。

外部ライブラリの使用範囲も固定します。言語やツールチェインの型が core に漏れると、別の言語で同じ契約を保てなくなるためです。

| ライブラリ | 使用してよい範囲 |
| --- | --- |
| `syn`、`proc-macro2`、`quote`、`rustc-demangle`、`toml` | `vtest-adapter-rust`（`vtest-scan` の `syn` は #155、#175 R6 で解消する） |
| `ignore` | ファイル走査を行う adapter |
| `clap` | `vtest-cli` |
| `yaml_serde` | `vtest-store`（正典 record の形式の所有者）。`vtest-verify` の既存利用は #175 で store への移管を確認する |
| `sha2` | `vtest-model` |
| `serde`、`serde_json`、`thiserror`、`anyhow` | 制約なし |

### 4.3 コンポジションルートと adapter の登録

adapter はコンポジションルート（現在は `crates/vtest-cli/src/adapters.rs` の `builtin_registry()`、D2 の後は操作層 crate）で生成し、`AdapterRegistry` に登録して操作に渡します。

1. コンポジションルートが、組み込みの adapter を**明示的に列挙して**登録する。自動検出、`build.rs`、汎用 plugin 機構は使わない（#175 D8）。登録箇所が 1 か所で読めれば、BD-007 と BD-102 の要求を満たすためです。
2. `vtest-scan`、`vtest-exec`、`vtest-verify` は adapter を自分で生成しない。registry を引数で受け取る。
3. `AdapterRegistry::register` の `Err`（adapter ID の重複、宣言した capability と実装の不一致）は、黙殺せず呼び出し元へ伝播させ、`E-ADAPTER-001` の usage failure にする。
4. 登録の順序は出力に影響させない。`vtest-scan` が adapter ID の辞書順に正規化する（DES-231）。
5. 言語を追加する PR は、登録行と自分の言語のファイルだけを変える（第 9 節）。登録行の競合は統合担当が解消する。
6. 正典の範囲外の adapter（現在は `rust-cargo` 以外の言語 adapter。REQ-264、SPEC-218、SPEC-361、SPEC-458）を production として扱うには、先に spec PR が必要です。実験的に登録するときは `docs/adapters/` に実験的であることを書く。

### 4.4 境界を越えるデータのスキーマ

境界を越えるデータのスキーマは、それぞれ 1 か所で定義します（Single Source of Truth）。

| 境界 | スキーマの所有者 | 対になる側 |
| --- | --- | --- |
| CLI の JSON 出力と MCP の tool 結果（envelope、DS-941） | `vtest-model::protocol` と操作層 | `vtest-cli` の出力、`vtest-mcp` の envelope |
| MCP の tool 名と入力スキーマ | `vtest-mcp`（名前は正典の tool 一覧から取る） | 操作層の引数 |
| 正典 record（`.verify/**`） | `vtest-store`（読み手と書き手を同じ crate に置く） | `vtest-exec` の Evidence 書き込み（#175 R9 で store に移す） |
| adapter と core の DTO、Test のワイヤー形式 | `vtest-adapter-api`（`wire.rs`） | 各 adapter の `wire_codec` |
| 仕様の文書 | `docs/canonical/specification.schema.json` | `.verify/doc/` の射影、`vtest-store` の document file |

### 4.5 crate 内の依存

```text
src/lib.rs                ← crate の公開 API と、責務の説明（//!）。下位の module を宣言する
src/<責務>.rs              ← 1 つの変更理由に対応する module
src/<責務>/mod.rs          ← 責務の中にさらに変更理由が分かれるとき（例: operations/edit.rs、operations/query.rs）
src/tests.rs              ← 1,000 行を超える同居テスト
tests/<対象>.rs            ← 公開 API 経由の統合テスト
tests/support/mod.rs      ← 統合テストの共通 helper（fixture 生成など）
```

module 間の依存も、判定の module からファイル I/O の module へは向けません。判定は読み取った値を引数で受け取ります。

## 5. 配置の地図

この節は「何を、どこに、どの形で置くか」の正本です。
レビューと定期リファクタリングは、この地図と第 3 節、第 4 節との照合で判断します。

### 5.1 配置表

| 対象 | 配置 | 形式 |
| --- | --- | --- |
| バイナリのエントリポイント | `crates/vtest-cli/src/main.rs` | 「引数のパース → 実行 → 終了コード」だけ |
| CLI の引数定義 | `vtest-cli` の clap 定義（#175 R4 の後は `src/args.rs`） | `#[derive(Parser)]` / `#[derive(Subcommand)]` |
| CLI のコマンド処理 | `vtest-cli` の `run_*`（R4 の後は `src/commands.rs`） | 操作層を呼んで結果を印字するだけ |
| text 描画 | `vtest-cli`（R4 の後は `src/render.rs`） | 結果の型を文字列にする関数 |
| 操作（1 操作 = 1 関数） | 操作層の `<操作名>.rs`（現在は `vtest-cli/src/ops/<操作名>.rs`） | 型付きのデータを返し、印字と終了コードの決定をしない |
| adapter の登録 | コンポジションルート（現在は `vtest-cli/src/adapters.rs`） | `builtin_registry()` に明示列挙 |
| エラーから（診断コード、終了コード）への写像 | そのエラー型を定義する操作層の module に 1 つ | 第 6.2 節。CLI と MCP は同じ関数を使う |
| MCP の transport、スキーマ、tool handler | `vtest-mcp`（#175 R5 の後は `transport.rs`、`schema.rs`、`tools.rs`） | handler は引数の抽出、操作層の呼び出し、envelope だけ |
| capability の trait と DTO | `vtest-adapter-api` | 言語の型を含めない。シグネチャは `vtest-model` の型だけ |
| 言語固有の処理 | その言語の adapter（Rust は `vtest-adapter-rust`、他の言語は `vtest-adapter-lexical/src/<言語>.rs`） | 第 9 節 |
| 複数の言語で意味が同じと確認できた処理 | 言語族の共通 module（`vtest-adapter-lexical/src/shared.rs`、`annotation.rs`） | 第 9.3 節の手順で比較してから置く |
| 正典 record の読み書きと形式 | `vtest-store` の record 種別ごとの module | 読み手と書き手を同じ module に置く |
| 判定（4 検査、鮮度、スコープ、集約）。スコープの 2 軸の扱いは第 1.1 節 | `vtest-verify`（#175 R8 の後は `checks/<検査名>.rs`、`evidence.rs`、`scope.rs`、`tree.rs`） | 入力を受け取って結果を返す、副作用のない関数 |
| 外部プロセスの起動 | 実行は adapter の runner、git の読み取りは 1 か所（#175 R9） | 引数は配列で渡し、文字列結合でコマンドを組み立てない |
| ドメインの型 | `vtest-model` の領域ごとのファイル | `serde` の derive。不変条件は型とコンストラクターで守る |
| 定数 | 利用する module の先頭。複数の crate が使うものは所有する crate の公開定数 | `const`。値の根拠（条項 ID）をコメントに書く |
| ユーティリティ | その契約を所有する層（#175 D7）。例: Locator の文字列化 → `vtest-model`、YAML の quoting → `vtest-store`、envelope → 操作層、fixture 生成 → `tests/support` | 何でも入る `utils` module や crate は作らない |
| 単体テスト | 対象と同じファイルの `#[cfg(test)] mod tests`、大きければ `src/tests.rs` | 第 13 節 |
| CLI と MCP の統合テスト、受入テスト | `crates/vtest-cli/tests/<対象>.rs` | 隔離したプロジェクトを生成して実行する |
| 統合テストの共通 helper | `crates/vtest-cli/tests/support/mod.rs` | 各テストファイルにコピーしない（#175 R12） |
| 言語 adapter の利用者向け説明 | `docs/adapters/<言語>.md` | 宣言の書き方、対応する構文、制限 |
| 調査、監査、レビューの経緯 | `docs/reports/<題名>-YYYY-MM-DD.md` | コードのコメントには書かない |

### 5.2 配置の決定手順

上から順に判定し、最初に該当した場所に置きます。

1. 特定の言語、ツールチェイン、テストフレームワークの知識が必要か → その言語の adapter。core に必要なら `vtest-adapter-api` に capability を足す
2. 外部（ファイル、record、MCP の引数、adapter の出力）からの入力をパースするか → その信頼境界（`vtest-store` の reader、MCP のスキーマ検証、`vtest-adapter-api::wire`、`vtest-scan` の出力検証）
3. 正典 record の形式か → `vtest-store`
4. 検証の判定か → `vtest-verify`
5. 操作の手順（読み込み → registry → scan → 実行 → 結果）か、エラーの写像か → 操作層
6. 入出力の変換（引数、text、JSON-RPC）か → 入口（`vtest-cli`、`vtest-mcp`）
7. それ以外 → 3.3 の責務表で責務の合う crate の、責務の合う module

### 5.3 crate のディレクトリ構成

ファイルは変更理由ごとの module に分けます。空の module は作らず、該当するコードができた時点で作ります。

```text
crates/<crate>/
  Cargo.toml                 依存は [workspace.dependencies] を参照する
  src/
    lib.rs                   //! で責務、境界、条項 ID。公開 API と mod 宣言
    <責務>.rs                 変更理由ごとの module
    <責務>/mod.rs             責務の中で変更理由が分かれるとき
    tests.rs                 大きな同居テスト
  tests/
    <対象>.rs                 統合テスト
    support/mod.rs           統合テストの共通 helper
    fixtures/                そのテストだけが使う固定の入力
```

### 5.4 入口と操作層の構成

| 入口 | 操作層への接続 |
| --- | --- |
| CLI（`vtest-cli`） | clap の引数を操作の引数型に変換し、操作の結果を `--format text` なら text に描画、`json` なら envelope として出力し、終了コードを返す |
| MCP（`vtest-mcp`） | JSON-RPC の `tools/call` の引数をスキーマで検証し、同じ操作を呼び、同じ envelope を tool 結果として返す |

- 操作を追加するときは、CLI と MCP の両方の経路と、その同値性のテストを同じ PR で更新する。片方だけを変えると DS-1563 が手合わせになるためです。正典の tool 一覧に MCP の tool がない操作（`init`、`doctor`）は CLI だけに置く。
- 終了コードは `ExitCode` の 4 値（`0` 成功、`1` 検証 NG、`2` usage / 操作の拒否、`3` 内部エラー）だけを使う。

### 5.5 ファイルと項目の記述順序

- ファイル: `//!` の説明 → `use`（rustfmt の順序） → 定数 → 公開型 → 公開関数 → 非公開の関数 → `#[cfg(test)] mod tests`。
- `impl` ブロック: コンストラクター → 公開メソッド → 非公開メソッド。

## 6. 場面ごとの設計

### 6.1 採用する設計パターン

場面ごとに使うパターンを固定します。ここにない場面で新しいパターンが必要になったら、`kind/decision` の Issue で Owner の判断を受けてからこの表に追加します。

| 場面 | パターン | 実装方法 |
| --- | --- | --- |
| 言語やツールチェインへのアクセス | Ports and Adapters（capability） | `vtest-adapter-api` の trait を adapter が実装し、`AdapterRegistry` から取り出す。capability がなければ第 1.1 節の「言語固有処理の隔離」に従う |
| adapter の組み立て | コンポジションルート | 4.3 のとおり。明示列挙 |
| CLI と MCP からの操作 | 1 操作 = 1 関数（Application layer） | 操作層の関数が型付きの結果を返し、入口は変換だけを行う |
| 結果の返却 | JSON envelope（DS-941） | `ok`、`data`、`diagnostics`。CLI と MCP で同じ形 |
| 判定と副作用 | Functional Core, Imperative Shell | 判定関数は scan 結果、record、Evidence 文脈を引数に取る。ファイル、git、時刻は操作の入口で読む |
| 外部入力 | Parse, don't validate | 信頼境界で検証済みの型に変換し、内側では同じ検証を繰り返さない |
| 正典 record の更新 | append-only と不変 | 追記で表し、関係の変更は削除と追加で表す |
| 鮮度 | content hash による束縛 | 証拠を対象の現在の hash に結び付け、対象が変われば古い証明を無効にする |
| 集約 | fail-closed の集約木 | 子に `PASS` 以外があれば親を `PASS` にしない。代表状態は決定論的に選ぶ。検査軸で省略した検査は木の中に `NO_EVIDENCE` で残し、エンティティ軸で要求外のエンティティは木に入れず scope 表現で未検証と示す（第 1.1 節） |
| エラーの返却 | `Result` とエラー型 | 6.2 のとおり |

採用しないパターン: モードフラグで意味の違う処理を 1 関数にまとめること、将来の拡張のためだけの trait や factory、汎用 plugin 機構、何でも入る `utils`。

### 6.2 エラー処理（#175 D5）

- 失敗しうる処理は `Result` を返す。ライブラリ crate の公開 API では `thiserror` で原因別のエラー型を定義し、panic で失敗を伝えない。
- エラーは次の 4 分類のいずれかにし、原因と診断コードを保ったまま境界で変換する。

    | 分類 | 診断コード | 終了コード | 根拠 |
    | --- | --- | --- | --- |
    | 入力誤り、操作の拒否 | `E-OP-*`、`E-CONFIG-*`、`E-APPROVAL-*`、`E-DA-*` | 2 | DS-322、DS-929、DS-935 |
    | 能力不足、adapter の前提 | `E-ADAPTER-001`〜`004` | 2 | DS-929、DS-1663、DS-1555 |
    | 検証 NG（結果は生成する） | 検証状態と診断ラベル（`E-SCAN-*` などの診断を含む） | 1 | DS-322 |
    | 内部エラー（I/O、直列化、不変条件の違反） | `E-INTERNAL-001` | 3 | DS-1689、DS-1690 |

- 各操作のエラー型から（診断コード、終了コード）への写像関数は、そのエラー型を定義する操作層の module に 1 つだけ置く。CLI と MCP はその関数を使う。
- 新しい診断コードは、正典に定義があるものだけを使う。正典にない状況に出会ったら、既存のコードに無理に当てはめず、Issue で上流に報告する。
- 処理方法が定まらないエラーは、1.2 の fail-closed に従い、`PASS` にしない、または操作を何も変えずに拒否する。内部の失敗を正常値や `PASS` で代用しない。

### 6.3 外部入力と信頼境界

- record、設定、MCP の引数、adapter の出力は、信頼境界でバリデーションし、検証済みの型に変換する。内側の関数は検証済みの型だけを受け取るので、同じバリデーションを入口ごとに重複して書かずに済む。
- 互換 reader と移行用 reader も、正規化した後に現在の不変条件を通す（第 1.1 節）。reader が検証を迂回する経路を作らない。
- 解釈できない構文断片を正常な対象として採用しない。adapter が読めないファイルは `E-SCAN-001` にして何も返さず、範囲を確定できない Test は Missing として診断を付ける。

### 6.4 決定論

- `HashMap` の列挙順、ファイルシステムの走査順、環境変数、時刻、一時パスに出力を依存させない。順序が意味を持つ出力は `BTreeMap` や明示的な並べ替えで決める。
- テストでは時刻、一時パス、Evidence ID を固定するかマスクして比較する。

### 6.5 安全性と秘密情報

- 外部プロセスは `std::process::Command` に引数を配列で渡す。文字列を結合してシェルに渡さない。
- パスはプロジェクトルートからの相対パスとして正規化し、ルートの外へ出るパスを受け付けない。
- ログと出力に、認証情報や環境変数の値を含めない。
- 無用な全件の再読み込みとデータのコピーを増やさない。性能を変えうる PR では、release build で計測した値を PR に書く（#175 §11）。

## 7. 新機能の設計手順

1. **関連する正典の条項を読む。** 要求から受入条件まで、該当する層を上から読み、条項 ID を控える。条項どうしが矛盾したら、条項 ID を添えて上流に報告し、上流が決めてから規則を選ぶ。黙って選ぶと、レビューなしで契約が変わるためです。
2. **機能を関心事ごとに分解する。** 「言語固有の処理」「外部入力のパース」「正典 record」「判定」「操作の手順」「入出力の変換」の 6 つに分ける。
3. **関心事ごとに配置を決める。** 5.2 の手順で判定し、3.3 の責務表の crate に割り当てる。1 つの機能が複数の crate にまたがるのは通常のことで、各部分を責務の合う crate に置く。
4. **副作用を割り当てる。** ファイル、git、外部プロセスを読む部分を操作の入口側に置き、判定は読み取った値を受け取る関数にする。
5. **依存を確認する。** 新しく必要な依存が 4.2 の許可リストにあるか確認する。なければ 4.1 の基準で capability を定義し、依存の向きを合わせる。
6. **既存の crate に収まらない場合だけ、新しい crate を作る。** 条件は次の 3 つをすべて満たすこと。
    - 3.3 の責務表のどの crate にも該当しない
    - 単独でテストできる
    - 4.2 の許可リストに、循環依存を作らずに配置できる

    新しい crate は、2.1、3.3、4.2 の表を更新する PR を先に出してから作る（例: 操作層 crate、#175 D2）。
7. **足りない仕組みは別 Issue に切り出す。** 機能の実装に必要な基盤（capability、registry の機能、テスト helper など）が足りなければ、その不足を Issue にし、基盤の PR を先に出して、その上に機能の PR を積む。機能の PR の中で基盤を作ると、基盤の設計が 1 つの機能の都合で決まり、レビューも分けられないためです。
8. **下のレイヤーから実装する。** モデルの型 → adapter 契約 → adapter → 永続化 → オーケストレーション（判定）と単体テスト → 操作層 → 入口（CLI と MCP）と同値性のテスト。下から実装すると、入口なしで単体テストできる状態を保ったまま進められる。
9. **PR はレイヤー単位で分割してよい。** 移動と論理の変更は別の PR にする（第 14 節）。

例として、「新しい言語 Go の Test を発見する」機能は次のように配置します。

| 関心事 | 内容 | 配置 |
| --- | --- | --- |
| 正典の範囲 | `rust-cargo` 以外の production adapter は v0.1 の範囲外（REQ-264 など）。実験的として扱うか、先に spec PR を出すかを Owner に確認する | `kind/decision` の Issue |
| 言語固有の処理 | Go のコメントと文字列の構文、関数と本文の範囲、名前と scope を読み、`FileItems` を返す | `crates/vtest-adapter-lexical/src/go.rs`（他の言語のファイルを変えない） |
| 意味が同じ処理 | `@vtest.` の文法、括弧の対応付け、コメントと次の construct の結び付け | 既存の `annotation.rs`、`shared.rs` を呼ぶ。足りなければ第 9.3 節の手順で比較してから足す |
| adapter の登録 | `vtest_adapter_lexical::go::adapter()` を登録する | コンポジションルート（`adapters.rs`）の 1 行 |
| 判定 | `source_discovery` だけを宣言するので、`target_binding` と `oracle_presence` は `NO_EVIDENCE` になる | 既存の `vtest-verify`（変更なし） |
| テスト | 言語のファイルの単体テストと、scan → verify の e2e | `src/go.rs` の `mod tests`、`crates/vtest-cli/tests/lexical_go_e2e.rs` |
| 利用者向け説明 | 宣言の書き方、対応する構文、制限、実験的であること | `docs/adapters/go.md` |

## 8. コーディング規約

### 8.1 共通

- 識別子は英語。コメントとドキュメントの言語（日本語と英語の混在の扱い）は **Owner の決定待ち**です（#175 D10）。決まるまで、言語を理由に差し戻さず、既存のファイルを言語の統一のためだけに書き換えない。決まったら、決定を #175 に記録してから、この項目を規則に書き換える。
- 文字コードは UTF-8、改行コードは LF（`.gitattributes` に従う）。
- フォーマットは `rustfmt` に任せる。CI は `cargo fmt --all -- --check` で検査する。
- `clippy` の警告は `-D warnings` で CI を失敗させるので、すべて直す。`#[allow(...)]` で抑制するときは、その行に理由をコメントで書く。`#[allow(clippy::too_many_arguments)]` は、引数を 1 つの型にまとめられないか先に検討する。
- マジックナンバーは名前付き定数にし、値の根拠（条項 ID）をコメントに書く。

### 8.2 Rust

**命名規則**（Rust の標準の規約に従う）

| 対象 | 規則 | 例 |
| --- | --- | --- |
| crate | `vtest-<役割>`（kebab-case）。adapter は `vtest-adapter-<名前>` | `vtest-adapter-rust` |
| module、関数、変数 | snake_case | `scan_project`、`resolve_targets` |
| 型、trait、列挙子 | UpperCamelCase | `AdapterRegistry`、`VerificationState::NoEvidence` |
| 定数 | SCREAMING_SNAKE_CASE | `ALL_CHECKS`、`ADAPTER_ID` |
| adapter ID | kebab-case の文字列 | `rust-cargo` |
| ファイル | snake_case | `execution_state.rs` |

**記述規則**

- production のコードで `unwrap()` と `expect()` を使うのは、不変条件で失敗しないことが示せるときだけにし、理由をコメントに書く。失敗しうるなら `Result` を返す。
- 自分の crate の列挙型を `match` するときは、なるべく全列挙子を書く。列挙子を追加したときに、扱いの漏れをコンパイラが検出するためです。`_` を使うときは、追加された列挙子をその腕で扱ってよい理由をコメントに書く。
- 所有権を持たない引数は `&str`、`&Path`、`&[T]` で受け取る。
- 戻り値を無視してはならない関数には `#[must_use]` を付ける。
- `unsafe` は、OS の API の呼び出しなど安全な代替がない場合だけに使い、直前に `// SAFETY:` で前提を書く。新しく `unsafe` を足すときは、PR の概要に理由を書く。

### 8.3 テストの宣言（`@vtest.`）

- このリポジトリのテストは自己検証の対象なので、テストには `@vtest.id`、`@vtest.covers`、`@vtest.target`、`@vtest.intent` を付ける（#61）。
- `covers` は `.verify/vo/` に存在する VO を指す。新しい VO の claim は、`docs/canonical/specification.json` の条項の文をそのまま使う。
- `target` は解決できる Source Target を指す。解決できなければ `vtest doctor` が `E-SCAN-004` を出す。
- 複数の VO は 1 行に `@vtest.covers A, B` と書く。同じキーを繰り返すと `E-SCAN-005` になる。

## 9. 言語対応の設計

### 9.1 方針

言語ごとの処理は、**ファイル（言語）ごとに分岐させ、各言語の処理を独立させます。**
各言語の処理は「その言語でどんな処理があれば実現できるか」だけを考えて設計し、他の言語を考慮しません。
共通化を先に設計せず、言語ごとに書いた処理を比較して、意味が同じと分かった部分だけを共通化します（第 9.3 節）。
共有の解析器が言語ごとの構文を誤認し、本文を欠落させる不具合が実際に起きたためです（#175 F8）。似た構文を 1 つの関数で扱うと、言語の差が条件分岐の中に隠れ、ある言語の修正が別の言語を壊します。

### 9.2 言語ごとの責務（#175 D3）

| 層 | 責務 | 配置 |
| --- | --- | --- |
| core | adapter への委譲、出力の検証（Test ID の一意性、VO 参照、locator の衝突）、adapter ID 順の決定論的統合、record の整合性。言語の構文を持たない | `vtest-scan` |
| 言語族の共通部分 | ファイルの走査（include / ignore）、各言語が返した `FileItems` の後段処理（`@vtest.` 宣言の文法と診断、ID と Locator の正規化、Draft への変換）。意味が同じものだけを置く | `vtest-adapter-lexical/src/lib.rs`、`annotation.rs`、`shared.rs` |
| 言語ごと | コメントと文字列の構文、construct の範囲（本文、宣言、式）、名前と scope | `vtest-adapter-lexical/src/<言語>.rs`（1 言語 1 ファイル、互いに独立）。Rust は `vtest-adapter-rust` |

- 各言語の module は他の言語の module を呼ばない。共通にしたいものは、9.3 の手順で共通部分に移してから呼ぶ。
- 各言語が `SourceDiscoveryAdapter` を直接実装してもよい。共通部分を使うことは義務ではない。
- 条件フラグで複数の言語の文法を 1 つの共有関数に実装しない。言語ごとに crate を分割すること、自動登録の仕組みを作ることもしない（4.3）。
- 解釈できない入力は正常な断片として返さない（6.3）。本文の末尾だけを変えても construct と subject の hash が変わることをテストで確認する。

### 9.3 言語間の共通化の手順

1. 2 つ以上の言語で、それぞれ独立に処理を書き、テストを通す。
2. 書いた処理を比較し、同じに見える部分を挙げる。
3. 挙げた部分ごとに、第 11 節の判定（「一方を変えたとき、もう一方も必ず同じ変更が必要か」）を行う。
4. 「必ず同じ変更が必要」なものだけを共通部分に移し、移した理由（なぜ言語に依存しないか）を `//!` に書く。例: 括弧の対応付けは、どれが括弧かを各言語の lexer が決めた後は言語に依存しない。コメントと次の construct の結び付けは、SpecTracer の宣言規則であり言語の文法ではない。
5. 偶然同じに見えるだけのもの（例: `//` と `/* */` のコメント構文は複数の言語で同じ綴りだが、それぞれの言語の文法）は、言語ごとのファイルに書いたままにする。

共通化には 2 種類の判断があります。

- **構造的に共通化してよいもの**: SpecTracer 自身の規則（宣言の文法、ID と Locator の形式、診断コード）と、言語の判断が済んだ後の機械的な処理（括弧の対応付け）。
- **共通化できるが、しないほうが分かりやすいもの**: 言語の文法に属するもの。コードの形が同じでも、言語ごとの変更理由で別々に変わるので、言語ごとのファイルに置いたほうが、その言語だけを読んで理解できる。

## 10. コメント（#175 D10）

- **すべての `src/*.rs`** の先頭に `//!` で「責務」「境界（この module がしないこと）」「実現する条項 ID」を書く。読み手が本文より先に役割を知るためです。ないファイルを触ったら追加する。
- **公開 API**（`pub` の型と関数）に `///` で契約（入力、出力、失敗時の診断コード、副作用、不変条件）を書く。呼び出し側が実装を読まずに使えるようにするためです。
- 実装内のコメントには、処理内容（what）ではなく理由（why）を書く。非自明な選択には、理由と根拠（条項 ID、Issue 番号）を 1〜2 文で書く。
    - 良い例: `// adapter が読めないファイルは何も返さない。断片を返すと、本文を欠いた Test が PASS しうる（DS-535）`
    - 悪い例: `// 詳細設計 §4.4 参照`
- **仕様書の節番号（「§4.4」「本冊:565」など）だけで参照しない。** 改版で番号がずれたり節が削除されたりすると、参照先を追跡できなくなるためです。条項 ID を使い、コメント単体で理由が理解できるように書く。
- **レビューの経緯（誰がどのラウンドで何を直したか）はコードの外（PR、Issue、`docs/reports/`）に置く。** 現在の契約と経緯が混ざると、実装と食い違うコメントになるためです。既にあるものは、触った PR か #175 R13 で移す。
- コードを読めば分かる処理の逐語的な説明は書かない。古くなったコメントは、コードと同じ PR で直す。コメントアウトしたコードは残さない（履歴は Git にある）。
- `CHECKME` などの保留マーカーは、Issue 番号を付けて残し、Issue で解消する。

## 11. 共通化の基準（DRY）

- **知識の重複は排除する。** 知識とは、同じ規則やドメイン知識（`@vtest.` 宣言の文法、Locator の文字列化、YAML の quoting、envelope の形など）のこと。同じ意味と同じ変更理由を持つと**確認できた**時点で、その契約を所有する層に抽出する（#175 D7）。
- **偶発的な重複（accidental duplication）は抽出しない。** コードの形が同じでも、変更理由が異なるなら別の知識として扱う。判定基準は「一方を変更したとき、もう一方も必ず同じ変更が必要か」。必要がなければ、それぞれに書いたままにする。偶発的な重複を共通化すると、各呼び出し元の都合で共通関数に引数や分岐が増え、結合度が上がって可読性と変更容易性が下がる。
    - 例: CLI の text 描画と MCP の JSON、`vtest-scan` の record 検証と `vtest-verify` の評価、`vtest-scan` の `locator_key`（タプルの鍵）と Locator の文字列化は、似ていても意味が違うので統合しない。
- **共通化は比較の後に行う。** 1 か所目を書く時点で、将来の共通化を見越した抽象を作らない。2 か所目ができてから比較し、意味が同じものだけを抽出する（言語間の手順は第 9.3 節）。
- 抽出先は 5.1 の「ユーティリティ」行で決める。何でも入る `utils` は作らない。
- 抽出したユーティリティには単体テストを付ける。

## 12. CLI と MCP の設計

- 入口は状態を持たず、業務ルールを持たない。引数を操作の引数に変換し、操作の結果を出力に変換するだけにする。
- CLI と MCP は同じ入力に同じ `ok`、`data`、`diagnostics` を返す。違うのは表現（text、終了コード、JSON-RPC）だけにする。
- MCP の tool 名は正典の tool 一覧（DS-1193 など）の名前をそのまま使い、CLI のサブコマンド名に合わせて作らない。
- MCP の tool 入力スキーマと引数の検証は `vtest-mcp` に置き、検証に通った値だけを操作層に渡す。
- CLI の `--quiet` は出力を止めるだけで、終了コードと副作用を変えない。
- 正典に定義のある操作で未実装のものは、実装したように見せない。tool 一覧に出さず、未実装であることを文書と報告に書く。

## 13. テスト

- 単体テストは対象と同じファイルの `#[cfg(test)] mod tests` に置き、大きくなったら `src/tests.rs` に移す。
- CLI と MCP の統合テスト、受入テストは `crates/vtest-cli/tests/` に置く。受入テストの構成は `TESTING.md` に従う。
- テスト名は「条件と期待結果」が分かる snake_case にする（例: `stale_evidence_keeps_verify_non_passing`）。仕様との対応は `@vtest.covers` と `@vtest.intent` で書く（8.3）。
- CLI を実行するテストは、一時ディレクトリに隔離したプロジェクトを生成して使う。このリポジトリの `.verify/` に書き込むと、テストの出力が正典のデータに混ざるためです。
- fixture の生成などの共通 helper は `tests/support/mod.rs` に 1 つだけ置く。
- 外部の振る舞い（公開 API、実ファイルを使う fixture、CLI と MCP の同値性、古い record の互換性、境界値、失敗経路）に結び付けてテストする。内部の実装の行をなぞるだけのテストは増やさない。
- 未実装の機能の受入項目を、「機能がないこと」を確かめるテストで満たしたことにしない。機能がないまま通り、未実装を実装済みに見せるためです。未実装の受入項目は、受入の対応表（`tests/ACCEPTANCE.md`、`docs/reports/adapter-acceptance-matrix.md`）に gap として記録する。
- 製品が保証する**否定条件の契約**は、テストで押さえる。これは上の項目とは別のものです。例: 未実装の MCP tool が `tools/list` に出ないこと（第 12 節）、capability がない操作を `E-ADAPTER-004` で拒否し、Evidence と Judgment を作らないこと（第 1.1 節）、拒否された操作が何も変えないこと（第 2.3 節）。テストの `@vtest.covers` は、その否定条件を定める条項の VO を指す。
- 純移動の PR はテストを増やさず、既存のテストと出力の比較で振る舞いの維持を示す。境界に触れる PR は契約テスト（CLI と MCP の同値性、architecture test、出力のバイト一致）を追加する（#175 D11）。

## 14. 開発プロセス

1. 着手前に、関連する正典の条項を読む。新機能は第 7 節の手順で設計する。
2. **すべての作業は Issue から始める。** テンプレートは「作業」「不具合」「仕様の変更（spec/*）」「Owner 判断」の 4 種類。
3. **1 つの PR は 1 つの目的に限る。** 機能追加、仕様変更、既知の不具合の修正、依存の更新、リファクタリングを同じ PR に混ぜない。
4. **移動（ファイルの移動、名前の変更）と論理の変更は別の PR にする。** 移動の PR は差分が大きく、論理の差分が埋もれるためです。移動の PR は `git diff -M` で rename として見える形にする。
5. **共有ファイルを変える PR は同時に 1 本だけ開く。** 対象は `vtest-cli/src/lib.rs`、`vtest-mcp/src/lib.rs`、`adapters.rs`、workspace の `Cargo.toml` と `Cargo.lock`、`vtest-verify/src/lib.rs`。並行すると、移動の差分の取り込みが繰り返されるためです（#175 §10）。手順は `DEVELOPMENT.md` の「Pull request scope」。
6. **既存の振る舞いを変えない。** リファクタリングは、外部契約（第 1.2 節の 3）を変えずに行い、変更前後の出力を比較して PR に書く（比較手順は `docs/reports/refactor-baseline-2026-11-07.md` §6）。振る舞いを変える必要がある場合は、別の Issue と PR にする。
7. **足りない基盤と露出した問題は別の Issue にする。** 作業中に仕組みの不足、既存の不具合、仕様の欠陥や矛盾を見つけたら、再現条件、影響、条項 ID を書いた Issue にする（仕様の問題は `needs-owner`）。作業中の PR に混ぜず、コードで回避もしない。回避すると、仕様の欠陥が次の実装者に残るためです。
8. **仕様の変更は `spec/*` ブランチで行う。** spec path（`docs/canonical/**`（`export/` を除く）と `.verify/doc/**`）は `spec/*` でだけ変え、`spec/*` では `crates/` と `tests/` を変えない。spec PR は同時に 1 本だけ。CI の `spec-guard` が検査する。詳細は `DEVELOPMENT.md` の「Specification changes」を読む。
9. Definition of Done:
    - ゲート（第 17 節）が成功している。成功しない項目は、環境の差か不具合かを分けて PR に書く
    - 追加、変更した処理にテストがあり、`@vtest.` の宣言が付いている
    - 新しいコードが第 3 節から第 5 節の責務、依存、配置に準拠している
    - 必要なドキュメント（このファイルの表、`ARCHITECTURE.md`、`docs/adapters/`、`TESTING.md`）が更新されている
    - `NOT_CHECKED`、`NOT_EXECUTED`、`UNKNOWN`、古い証拠などの残る制限が PR に明記されている
10. 静的解析の指摘や既存コードの細かなドリフトは、PR ごとにはブロックしない。第 15 節の定期リファクタリングでまとめて直す。PR では、新しいコードを地図どおりに配置することを守る。

## 15. 定期リファクタリングとコードレビュー

### 15.1 方針

CI はビルド、テスト、フォーマット、clippy、`vtest doctor`、`spec-guard` を検査します。構造のドリフトは日々の PR ではブロックせず、計画 Issue（例: #175）にまとめ、現在の位置から第 5 節の地図が示す位置へ移動して直します。
地図が明確であれば、どのドリフトも移動（Move）、分割（Split）、抽出（Extract）、名前の変更（Rename）、capability の導入の小さなリファクタリングで直せます。そのため、地図の曖昧さを残さないことを最優先にします。

### 15.2 サイクルの手順

1. 計画 Issue を起票し、基準のコミット（SHA）、環境、ゲートの結果、代表的な出力を固定する（#175 R0）。
2. `cargo clippy` と `cargo fmt` の指摘を直す。これを 1 つ目の PR にする。
3. コードベース全体を第 3 節から第 5 節と照合し、ドリフトを計画 Issue の移動表に列挙する（形式は 15.3）。照合には `.agents/skills/architecture-check` を使う。
4. ドリフトを「1 PR に 1 種類のリファクタリング」にまとめて直す（例: 「`vtest-mcp` の transport を分ける」「Evidence の writer を store に移す」）。各 PR で変更前後の出力を比較する。
5. 地図に欠落や矛盾が見つかったら、コードより先にこのファイルを直す PR を出す。
6. すべて解消したら計画 Issue をクローズする。

### 15.3 ドリフトの記述形式

ドリフトは次の 5 項目で記述します。5 項目が揃っていれば、誰が直しても同じ結果になります。

| 項目 | 内容 | 例 |
| --- | --- | --- |
| 現在の位置 | ファイルと関数、または型 | `vtest-exec/src/lib.rs` の `evidence_yaml` |
| あるべき位置と形式 | 第 5 節の表に当てはめた結果 | `vtest-store/src/evidence.rs` の `write_evidence`（読み手の `read_evidence` と同居） |
| 根拠 | このファイルの節番号と条項 ID | 5.1「正典 record の読み書きと形式」行、4.4、BD-115 |
| 修正方法 | Move、Split、Extract、Rename、capability の導入のいずれか | Move し、`vtest-exec` からは呼び出すだけにする |
| 影響範囲 | 変更するファイルと、振る舞いを固定するテスト | Evidence YAML のバイト一致の golden テストを先に追加する |

### 15.4 重大度

| 重大度 | 対象 | 扱い |
| --- | --- | --- |
| 即時修正（`priority/high` 以上） | 製品不変条件（1.1）、依存の向き（4.1、4.2）、fail-closed（6.2、6.3）、外部契約（1.2 の 3）への違反 | その PR で差し戻す。マージ済みなら定期サイクルを待たずに直す |
| 次回サイクルで修正 | 配置、形式、命名、知識の重複、コメント、フォーマット | PR はマージしてよい。ドリフトを計画 Issue に記録する |

### 15.5 レビューの規則

- **レビューの前に、対象、目的、固定する判断軸、除外を書く。** 該当する軸がなければ、レビューの前に軸を定義する。レビュー中は宣言した軸を変えない。軸を変えると、新しい基準の陰に失敗が隠れるためです。軸に不足があれば別に報告し、宣言した軸で続ける。
- 「判断軸を変えること」と「宣言した軸で失敗を見つけること」を区別する。
- 宣言した軸の中の失敗では差し戻す。軸の中の重要な問題はすべて報告する。指摘の数そのものは過剰なレビューではない。
- 宣言した軸の外で正典や安全性の重大な違反を見つけたら、軸外のブロッカーとして報告するか、明示的にスコープを変える。未解決の間は ready 以外の判定を返す。
- gate は、現在の段階が次の段階に責務を正しく引き渡せるかで判断し、成果物の完璧さでは判断しない。必要な引き渡しの証拠を書き、証拠がなければ fail-closed にする。
- 直接の観察と推論を分ける。指摘にはファイルと節、またはコマンドの証拠を付け、未解決の正典の問題は推測せず、未解決と書く。
- 指摘には、このファイルの節番号か条項 ID を根拠として書く。根拠を示せない好みは指摘にしない。このファイルに書かれていないことでは差し戻さず、必要ならこのファイルの改訂を提案する。
- リファクタリングの PR は、既存のテストを変えずに成功することを確認する。テストを変えた場合は、振る舞いが変わらない根拠を PR に書く。
- 検証とリリースの判断には `.agents/skills/`（`verify-change`、`architecture-check`、`release-check`）を使う。
- 削除された helper script を作り直すのは、新しい決定論的な手順が現在の正典契約と現在の実装から完全に導けるときだけにし、足した helper は検証してから使う。検証されていない helper は、レビューされていない結果の出どころになるためです。
- 現在の実装とテストを調べ、その振る舞いより正典を権威とする。振る舞いは過去の判断の記録で、誤っていることがあるためです。作業ツリーに作業範囲外の正当な利用者の変更があれば、そのまま残す。
- チェックは変更した範囲に比例して実行する。開発のゲート（第 17 節、`DEVELOPMENT.md`、`TESTING.md`）はコードの品質を示すもので、正典への適合を示さないので、適合の証拠は別に集める。

## 16. GitHub の運用

- **Issue**: すべての作業は Issue から始める。テンプレートは「作業」「不具合」「仕様の変更（spec/*）」「Owner 判断」。脆弱性は公開の Issue ではなく、`SECURITY.md` の非公開の窓口に送る。
- **ラベル**: 種類（`kind/`）、領域（`area/`）、優先度（`priority/`）、規模（`size/`）の軸で付ける。Owner の判断が必要なものは `needs-owner`、他の作業を待つものは `blocked`。
- **ブランチ**: Git Flow（`main` はリリース、`develop` は統合）。通常の作業は `develop` から `feature/<topic>`、仕様の変更は `spec/<topic>`。名前はトピックで付け、版のような名前を付けない。正本は `DEVELOPMENT.md`。
- **共有履歴**: `main`、`develop`、共有の作業ブランチは fast-forward か merge だけで更新し、force-push しない。他の人が積んだ履歴を書き換えるためです。push が non-fast-forward で拒否されたら、リモートのブランチを merge してから push する。作業ブランチを `develop` に追従させるときも merge する。
- **コミットメッセージ**: Conventional Commits（`feat:`、`fix:`、`refactor:`、`docs:`、`test:`、`ci:`、scope は crate や adapter、例: `fix(adapter-c): ...`）。
- **PR**:
    - `develop` を base にし、`.github/PULL_REQUEST_TEMPLATE.md`（Summary、Branch flow、Verification evidence、Review notes）を埋め、`Closes #<Issue 番号>` で Issue をリンクする。
    - レビューを依頼できる状態になったら、draft にせず通常の PR（Ready for review）にする。draft は作業中のものだけに使う。
    - Issue と PR の本文に「Requested by … · project thread」の帰属ブロック（`claude-projects-attribution` のマーカー行を含む）を書かない。
    - `CHANGELOG.md` の更新は、PR 間の競合を避けるため #90 にまとめる。
    - CI の成功後にマージする（merge commit）。`main` と `develop` へ直接 push しない。
- **GitHub Actions**:
    - `CI`（`ci.yml`）: `spec-guard`（`develop` への PR の spec path 検査）と、fmt、test、clippy、`vtest doctor`
    - `Self-verify`（`self-verify.yml`）: `vtest run --all` と `vtest verify` をこのリポジトリに実行する。毎晩と手動で動き、必須のチェックではない
    - `Release`（`release.yml`）: `main` から到達できるタグでバイナリを公開する

## 17. よく使うコマンド

```sh
# ゲート（PR を出す前と更新する前に実行する）
cargo fmt --all -- --check
cargo test --workspace --locked --no-fail-fast
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo run --quiet -p vtest-cli --locked -- doctor

# 受入テストを 1 ファイルだけ実行する
cargo test -p vtest-cli --test acceptance_18_3_7

# カバレッジの道具（M7 の受入と target_binding に必要）
rustup component add llvm-tools-preview
cargo install cargo-llvm-cov --locked

# 依存の確認
cargo metadata --format-version 1 --no-deps

# release build
cargo build --locked --release --package vtest-cli
```

`--no-fail-fast` を付けないと、最初に失敗したテストバイナリで止まり、後続の結果が記録されない。
`cargo-llvm-cov` がない環境では、カバレッジを前提にするテスト（`evidence_e2e`、`execution_state_e2e` の一部）が失敗する。その場合は環境の差として PR に `NOT_EXECUTED` と書き、CI（`cargo-llvm-cov` あり）の結果を証拠にする。

## 18. 指示と文書の書き方

- 規則は「取る行動 → 理由 → 起こりうる状況と、それぞれの行動」の順で書く。短く書くと「X する。理由は Y。Z が起こりうるので、W の場合は V する」。どの言語でもこの順を守る。
- 禁止は、同じ目的を果たす行動と、その理由と、場合ごとの行動で表す。例: 「退役した文書を規範として引用しない」ではなく、「規範は正典 JSON から引用する。archive は退役しているため。archive にしかない主張は JSON で探すか、未解決として報告する」。
- 書き直した規則は、古い禁止が防いでいた状況で読み手が何をするかを問うて確かめる。それが不明なら書き直す。否定がなくなったことではなく、読み手が行動を知ることで合格とする。
- 行動、理由、条件のどれも持たない文は削る。指示ファイルのすべての文は毎回のセッションで読まれるためです。
- これらの規則は指示に適用する。製品についての事実の記述（機能の説明、変更履歴、正典の条項）は事実の記述のままにする。
