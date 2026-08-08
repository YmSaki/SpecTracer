# AI並列開発向けテスト検証システム 詳細設計 v0.1（本冊）

## 0. 本書の位置付け

本書は「基本仕様 v0.1」を実装可能なレベルまで具体化する。
基本仕様が定めた外部挙動の保証を変更しない。
本書と基本仕様の間に矛盾がある場合、基本仕様を正とし、本書の該当箇所を不整合として扱う。

読者は実装エージェント（Codex）を想定する。
各節は実装単位に対応し、別紙B §18 のマイルストーンに沿って実装する。

### 分冊構成

詳細設計は3分冊とし、節番号は全冊を通した連番とする（基本仕様からの節参照を維持するため）。

| 分冊 | 収録節 |
|---|---|
| 本冊（コア設計） | §1〜§11、§16、§17、§19 |
| 別紙A（CLI・MCPインターフェース仕様） | §12〜§15 |
| 別紙B（実装計画） | §18 |

---

## 1. 実装構成

### 1.1 ワークスペース構成

Cargo workspace とし、単一バイナリ `vtest` を生成する。

```text
vtest/
  Cargo.toml            # workspace
  crates/
    vtest-model/        # エンティティ・状態モデル・ID型（依存最小）
    vtest-store/        # .verify/ レコードファイルの読み書きとスキーマ検証
    vtest-scan/         # Rustソース走査、アノテーション抽出、シンボル解決
    vtest-audit/        # 決定論的監査、監査バンドル生成、提出結果検証
    vtest-exec/         # テスト実行、Evidence記録、カバレッジ計測
    vtest-verify/       # 整合性検査、鮮度検証、集約、レポート生成
    vtest-cli/          # バイナリ vtest（clap によるCLI、mcp サブコマンド含む）
    vtest-mcp/          # MCPサーバ実装（vtest-cli から起動）
  tests/
    fixtures/           # 検証用サンプルプロジェクト（§18 受入基準で使用）
```

依存方向は `cli / mcp → verify / exec / audit / scan → store → model` の一方向とする。

### 1.2 主要依存クレート

| 用途 | クレート | 備考 |
|---|---|---|
| Rust構文解析 | `syn` 2.x（features: `full`, `extra-traits`, `visit`） | AST解析の中核 |
| スパン位置 | `proc-macro2`（feature: `span-locations`） | 編集・ハッシュ対象範囲の特定 |
| CLI | `clap` 4.x（derive） | |
| シリアライズ | `serde`, `serde_json` | |
| YAML | `serde_yaml`（メンテ状況により `serde_yml` 等の後継を選択してよい） | レコードファイル |
| ID | `ulid` | レコードID |
| ハッシュ | `sha2` | 内容ハッシュ（SHA-256） |
| ファイル走査 | `ignore` | `.gitignore` 準拠の走査 |
| エラー | `thiserror`（ライブラリ）, `anyhow`（バイナリ） | |
| MCP | `rmcp`（公式 Rust MCP SDK） | stdio transport |
| 日時 | `time` | RFC 3339 |

Git 操作（HEAD の取得、dirty 判定）は `git` CLI の呼び出しで行う（`git rev-parse HEAD`、`git status --porcelain`）。
`git` が利用できない場合、リビジョンは `UNKNOWN` とし、Evidence の `evidence_validity` は PASS にならない（fail-closed）。

### 1.3 内容ハッシュの定義

**内容ハッシュ**は次の正規化を施したテキストの SHA-256 とし、`sha256:<hex>` 形式で記録する。

- 改行を LF へ統一する。
- 各行の末尾空白を除去する。
- 対象範囲は次のとおり。
  - テスト関数：doc comment・属性を含む関数アイテム全体のソーステキスト
  - 対象関数：属性を含む関数アイテム全体のソーステキスト（doc comment を含む）
  - VO / REQ / SPEC レコード：YAML ファイル全体
- 空白の正規化はこれ以上行わない。インデント変更はハッシュ不一致となる（安全側）。

---

## 2. データディレクトリと設定

### 2.1 `.verify/` レイアウト

```text
.verify/
  config.yaml
  spec/       SPEC-<NAME>.yaml
  req/        REQ-<NAME>.yaml
  vo/         VO-<NAME>.yaml
  rel/        <ULID>.yaml
  forms/      <kind>.yaml
  approvals/  <ULID>.yaml
  audits/     <ULID>.yaml
  evidence/   <ULID>.yaml
  cache/      （Git管理外。.verify/.gitignore に `cache/` を出力する）
    bundles/  監査バンドル JSON
    logs/     テスト実行の生ログ
```

`vtest init` は上記ディレクトリ、`config.yaml` の雛形、`.verify/.gitignore`、組込 Form Schema（§14）を生成する。

### 2.2 `config.yaml`

```yaml
version: 1
project:
  name: my-project
scan:
  # テストコードを走査するパス。省略時はワークスペース全体
  include: ["src", "tests", "crates"]
  # 追加で assert 相当として扱うマクロ名
  assertion_macros: []
verify:
  # 完全検証のチェック項目（基本仕様 §4.2 の11項目。通常は変更しない）
  full_scope:
    - spec_coverage
    - vo_decomposition
    - vo_coverage
    - test_existence
    - static_audit
    - semantic_audit
    - impl_consistency
    - test_execution
    - runtime_result
    - target_execution
    - evidence_validity
run:
  # target_execution 計測方式: llvm-cov | off
  coverage: llvm-cov
```

### 2.3 派生情報

初期リリースでは、検証グラフとインデックスは実行のたびにインメモリで再構築する。
永続キャッシュは実装しない（将来の最適化点として `cache/` を確保する）。
MCP サーバは長時間動作するため、ツール呼び出しごとに対象ファイルの mtime を確認し、変化があれば再スキャンする。

---

## 3. レコードファイルスキーマ

すべてのレコードは YAML とし、未知フィールドはエラーではなく警告とする（前方互換のため）。
`id` とファイル名（拡張子除く）は一致しなければならない。

### 3.1 SPEC レコード（`.verify/spec/SPEC-*.yaml`）

```yaml
id: SPEC-BASIC-001
kind: document            # document | api-schema | type-spec | db-schema | other
path: docs/basic-spec.md  # プロジェクト相対パス
sha256: "sha256:..."      # 登録時の内容ハッシュ
title: 基本仕様書
note: ""                  # 任意
registered_at: 2026-08-08T00:00:00Z
```

`path` の実ファイルが変更され `sha256` と一致しなくなった場合、その SPEC を根拠とする承認・監査レコードの鮮度検証で検出する（§11）。
仕様文書そのものは `.verify/` へ複製しない。

### 3.2 REQ レコード（`.verify/req/REQ-*.yaml`）

```yaml
id: REQ-PARSER-001
parent: null              # REQ ID（階層化。Feature相当は中間ノード）
spec_refs:
  - spec: SPEC-BASIC-001
    section: "4.2"        # 節参照（自由形式の文字列）
summary: 不正なUTF-8入力をエラーとして拒否する
status: active            # active | withdrawn
created: 2026-08-08
updated: 2026-08-08
```

### 3.3 VO レコード（`.verify/vo/VO-*.yaml`）

```yaml
id: VO-PARSER-UTF8-003
parent: VO-PARSER-UTF8    # VO ID または null
requirements: [REQ-PARSER-001]
spec_refs:
  - spec: SPEC-BASIC-001
    section: "4.2"
claim: 不正な continuation byte を含む入力を与えた場合、ParseError::InvalidUtf8 を返す
dimensions: []            # 検証軸（任意。下記 3.3.1）
coverage_policy: null     # independent-axes | full-product | explicit | null
representative_cases: []  # 代表入力値（任意）
status: draft             # draft | approved（approved は承認レコードの有効性で決まる。§3.5）
created: 2026-08-08
updated: 2026-08-08
```

#### 3.3.1 dimensions と組合せの実体化

```yaml
dimensions:
  - name: operand-sign
    partitions: [positive, negative]
  - name: operator
    partitions: [add, sub, mul, div]
coverage_policy: full-product
```

`dimensions` を持つ VO は、`vtest vo expand VO-X` により子 VO を**実体化**できる。

- `independent-axes`：partition ごとに 1 子 VO（上例では 2 + 4 = 6 件）
- `full-product`：直積ごとに 1 子 VO（上例では 8 件）
- `explicit`：`combinations:` フィールドに列挙された組合せのみ

生成される子 VO の ID は `VO-X-<PARTITION>`（直積は `VO-X-<P1>-<P2>`）を既定とし、生成前に一覧を提示して確認できる（`--dry-run`）。
実体化後は通常の VO として扱われるため、`test_existence` などの決定論的検査は「leaf VO に検証があるか」だけを見ればよい。
組合せ空間の定義が仕様に対して十分かは `vo-coverage` 意味監査の判定対象である（基本仕様 §7.6）。

### 3.4 Relation レコード（`.verify/rel/<ULID>.yaml`）

導出できない関係のみを保存する（基本仕様 §2.3）。

```yaml
id: 01J8XVZK3Q...
type: depends-on          # depends-on | supersedes | regression-for |
                          # derived-from | same-partition | complements | conflicts-with
from: TEST-PARSER-044     # 任意のエンティティID
to: TEST-PARSER-012
note: ""
created: 2026-08-08T00:00:00Z
```

Relation は不変。変更はファイル削除＋新規作成で表す。
`from` / `to` の存在はスキャン時に検査する。

### 3.5 承認レコード（`.verify/approvals/<ULID>.yaml`）

```yaml
id: 01J8XW0A9M...
subject: VO-PARSER-UTF8-003     # 承認対象のエンティティID
subject_hash: "sha256:..."      # 承認時点の対象の内容ハッシュ
approver:
  kind: agent                   # human | agent
  id: reviewer-agent-01
  model: claude-fable-5         # agent の場合任意
basis:                          # 根拠（任意）
  - kind: audit
    ref: 01J8XVZZ...            # 監査レコードID
approved_at: 2026-08-08T00:00:00Z
```

VO の実効状態は次で決まる。

```text
approved =
  「subject が一致し、subject_hash が現在の内容ハッシュと一致する
   承認レコードが1件以上存在する」
それ以外は draft（承認失効を含む）
```

VO レコードの `status` フィールドは表示用であり、実効判定は常に承認レコードから導出する（正典の重複を避けるため、`status` と導出結果が食い違う場合は導出結果を採り、警告 W-STORE-001 を出す）。

### 3.6 監査レコード（`.verify/audits/<ULID>.yaml`）

```yaml
id: 01J8XVZZ...
kind: test-semantic             # test-semantic | vo-coverage | impl-consistency | static
bundle_id: 01J8XVYY...          # static の場合 null
subjects:                       # 監査対象と当時の内容ハッシュ
  - id: TEST-PARSER-044
    hash: "sha256:..."
  - id: VO-PARSER-UTF8-003
    hash: "sha256:..."
  - locator: "src/parser.rs::Parser::parse"
    hash: "sha256:..."
verdict: PASS                   # PASS | FAIL | UNKNOWN
reasons:                        # §8.3 の構造。static では規則違反の一覧
  - claim: テストは不正UTF-8入力に対する InvalidUtf8 の返却を検証している
    basis:
      - kind: test-code
        ref: "tests/parser_test.rs::rejects_invalid_utf8"
exclusions: []
auditor:
  kind: agent                   # deterministic | agent | human
  id: auditor-agent-01
  model: claude-fable-5
confidence: high                # high | medium | low（deterministic では省略）
audited_at: 2026-08-08T00:00:00Z
revision: { commit: "abc123...", dirty: false }
```

同一対象への監査レコードは複数存在してよい（再監査・多重監査）。
判定に用いるのは「subjects の全ハッシュが現在と一致する（＝有効な）レコードのうち最新のもの」とする。
有効なレコードに FAIL と PASS が混在する場合は FAIL を採る（fail-closed）。

### 3.7 Evidence レコード（`.verify/evidence/<ULID>.yaml`）

```yaml
id: 01J8XW1B...
test_id: TEST-PARSER-044
result: PASS                    # PASS | FAIL
executed_at: 2026-08-08T00:00:00Z
revision: { commit: "abc123...", dirty: false }
hashes:
  test_fn: "sha256:..."
  target_fn: "sha256:..."
runner:
  kind: cargo-test
  command: "cargo test -p parser --lib -- --exact parser::tests::rejects_invalid_utf8"
  exit_code: 0
target_execution:
  checked: true                 # 計測を実施したか
  method: llvm-cov
  result: PASS                  # PASS | FAIL | UNKNOWN
  count: 3                      # 対象関数の実行回数
log_ref: "cache/logs/01J8XW1B.log"   # Git管理外の生ログ
```

---

## 4. アノテーション構文

### 4.1 文法

テスト関数直前の doc comment（`///` または `/** */`）内の行を対象とする。

```text
annotation-line = "@vtest." key SP value
key             = "id" | "covers" | "target" | "intent" | "input"
                | "expect" | "kind" | "case" | "related" | "src-id"
value           = 行末までのテキスト（前後空白は除去）
```

- 1行1キー。`covers` と `related` の値はカンマ区切りで複数指定できる。
- `case` と `related` はキー自体を複数行書ける。他のキーの重複はエラー E-SCAN-005。ただし `kind` が integration 系の Test に限り、`target` の複数行を許容する（別紙A §14.3）。
- `@vtest.` で始まるが未知のキーを持つ行はエラー E-SCAN-006（打鍵ミスの検出を優先し、警告ではなくエラーとする）。
- doc comment 内の `@vtest.` を含まない行は自由記述として無視する。
- `@vtest.src-id` はテストではなく対象実装側の関数に付与し、恒久 SRC ID を宣言する（基本仕様 §3.3 の昇格。初期リリースではスキャンによる認識のみ実装し、必須機能としない）。

### 4.2 ロケータ構文

```text
locator   = path "::" item-path
path      = プロジェクトルートからの相対パス（"/" 区切り、".rs" で終わる）
item-path = Rust アイテムパス（"::" 区切り）
            impl ブロック内の関数は "型名::関数名"

例：src/parser.rs::Parser::parse
    src/lib.rs::validate_input
```

`path` は `.rs` で終わる最初の `::` で item-path と分離する。
`@vtest.target` の値が `SRC-` で始まる場合は SRC ID 参照として解決する。

### 4.3 パースエラーの扱い

アノテーションのパースエラーはスキャン診断（§5.4）として報告し、当該 Test はエンティティとして登録しない。
未登録の `#[test]` 関数と同様に unregistered として扱われ、fail-closed により関連 VO の `test_existence` は PASS にならない。

---

## 5. スキャナ設計

### 5.1 処理フロー

```text
1. ファイル探索
   config.scan.include 配下の *.rs を ignore クレートで列挙
   （.gitignore 準拠、target/ は除外）

2. 構文解析
   ファイルごとに syn::parse_file
   解析エラーのファイルは診断 E-SCAN-001 を出し、当該ファイルをスキップ

3. モジュールパス構築
   crate ルート（src/lib.rs / src/main.rs / tests/*.rs）から
   mod 宣言を辿り、各アイテムの完全モジュールパスを構築
   （mod foo; → foo.rs または foo/mod.rs）

4. テスト関数抽出
   属性パスの末尾セグメントが "test" である関数
   （#[test], #[tokio::test] 等）を抽出

5. アノテーション抽出
   doc 属性（#[doc = "..."]）を §4 の文法でパース

6. 対象関数抽出
   すべての fn / impl fn を SRC 候補として索引化
   （ロケータ解決・逆引き・@vtest.src-id 認識に使用）

7. .verify/ 読み込み
   vtest-store が全レコードを読み込み、スキーマ検証

8. グラフ構築と整合性検査（§5.3、§5.4）
```

### 5.2 エンティティモデル（vtest-model）

```rust
pub struct TestEntity {
    pub id: TestId,
    pub covers: Vec<VoId>,
    pub target: TargetRef,          // Locator(Locator) | SrcId(SrcId)
    pub intent: String,
    pub input: Option<String>,
    pub expect: Option<String>,
    pub kind: Option<String>,
    pub cases: Vec<String>,
    pub related: Vec<TestId>,
    pub location: SourceLocation,   // ファイル、モジュールパス、関数名、byte range
    pub content_hash: ContentHash,  // §1.3
    pub filter: String,             // cargo test 用フィルタ（§9.2）
    pub package: String,            // cargo package 名
    pub test_target: TestTarget,    // Lib | Bin(String) | IntegrationTest(String)
}

pub enum CheckValue {
    Pass, Fail, Mismatch, Missing,
    NotChecked, NotExecuted, Stale, Unknown,
}

pub enum CheckItem {
    SpecCoverage, VoDecomposition, VoCoverage, TestExistence,
    StaticAudit, SemanticAudit, ImplConsistency,
    TestExecution, RuntimeResult, TargetExecution, EvidenceValidity,
}
```

VO / REQ / SPEC / Relation / Approval / AuditRecord / Evidence も §3 のスキーマに対応する struct を定義する。

### 5.3 検証グラフ

インメモリのグラフを構築する。

```text
ノード：SPEC, REQ, VO, TEST, SRC（ロケータ単位）
エッジ：
  REQ  → REQ    (parent)
  VO   → VO     (parent)
  VO   → REQ    (requirements)
  VO   → SPEC   (spec_refs)
  TEST → VO     (covers)      ※アノテーション由来
  TEST → SRC    (target)      ※アノテーション由来
  外部 Relation (rel/ 由来)
逆引きインデックス：VO → Tests、SRC → Tests、REQ → VOs
```

### 5.4 整合性診断

| コード | 種別 | 内容 |
|---|---|---|
| E-SCAN-001 | error | ファイルの構文解析失敗 |
| E-SCAN-002 | error | Test ID 重複（identity collision） |
| E-SCAN-003 | error | `covers` の参照先 VO が存在しない（dangling reference） |
| E-SCAN-004 | error | `target` のロケータ／SRC ID を解決できない |
| E-SCAN-005 | error | 重複不可キーの重複 |
| E-SCAN-006 | error | 未知の `@vtest.` キー |
| E-SCAN-007 | error | 必須キー（id / covers / target / intent）の欠落 |
| E-SCAN-008 | error | VO / REQ の parent 不在または循環 |
| E-SCAN-009 | error | Relation の from / to が不在 |
| E-SCAN-010 | error | レコードの id とファイル名の不一致、スキーマ違反 |
| W-SCAN-101 | warning | `@vtest` アノテーションのない `#[test]` 関数（unregistered test） |
| W-SCAN-102 | warning | どの VO からも参照されず、Test も参照しない孤立 VO |
| W-SCAN-103 | warning | `covers` を持つが対応 VO が leaf でない（中間 VO 直接参照。許容するが警告） |
| W-STORE-001 | warning | VO の `status` フィールドと承認導出結果の不一致 |

error は該当エンティティに関わるチェック項目を非 PASS にする。
warning は検証結果を直接変えないが、レポートに常に表示する。

---

## 6. シンボル解決

### 6.1 解決アルゴリズム

ロケータ `path::item-path` の解決は、スキャン済みの SRC 索引（§5.1 手順6）への完全一致検索とする。

```text
1. path が索引に存在するか
2. path 内で item-path が一致する fn / impl fn が存在するか
3. 一意に決まらない場合（同名 fn が cfg 分岐で複数等）は
   すべて候補として返し、解決失敗（E-SCAN-004）とする
```

### 6.2 候補提示

Structured Operation の入力検証（§14、§15）で解決に失敗した場合、次の順で候補を返す。

```text
1. item-path の末尾セグメント一致（別パスの同名関数）
2. 編集距離 2 以内の近似名
出力例：
  ✗ symbol not found: src/parser.rs::Parser::prase
  candidates:
    src/parser.rs::Parser::parse
    src/parser.rs::Parser::parse_inner
```

enum variant の検証（`expect` の値が `ParseError::InvalidUtf8` 形式の場合）は、スキャン済み AST から enum 定義を検索する。
解決できる場合のみ検証し、解決できない自由記述はそのまま受理する（best effort。拒否はしない）。

---

## 7. 決定論的監査ルール

### 7.1 判定の原則

各ルールは `FAIL` / `UNKNOWN` / `PASS(違反なし)` のいずれかを返す。
**決定論的に確定できる違反のみ FAIL とする。**
解析の限界で確定できない場合は FAIL ではなく UNKNOWN とし、意味監査へ委ねる。
Test の `static_audit` チェック項目は、全ルールが違反なしなら PASS、1つでも FAIL があれば FAIL、FAIL がなく UNKNOWN があれば UNKNOWN とする。

**assert 相当の構文**は次のとおり定義し、全ルールで共通に用いる。

- `assert!` / `assert_eq!` / `assert_ne!` / `panic!` を含む標準マクロ、および config の `assertion_macros` に列挙されたマクロ
- `#[should_panic]` 属性
- `.unwrap()` / `.expect(..)` / `?` 演算子（Result / Option の成立検証として扱う）
- テスト関数が `Result` を返し `Err` を返しうる構造

### 7.2 ルール一覧

| ルール | 内容 | FAIL 条件 | UNKNOWN へ退避する例 |
|---|---|---|---|
| DA-001 定数アサーション | 引数がすべてリテラル・定数式の assert | 関数内の assert 相当がすべて定数アサーション | 定数性を確定できない式 |
| DA-002 対象未呼出 | 宣言された target シンボルを呼んでいない | 関数本体および同一ファイル内の呼出先 helper（1段）を探索して呼出が存在しない、かつ他ファイルへの呼出も存在しない | 他ファイル・他クレートの関数呼出があり、間接呼出の可能性を排除できない |
| DA-003 結果未検証 | target を呼ぶが、その結果を assert 相当で一切検証しない | target 呼出結果（戻り値、および結果から派生した束縛）が assert 相当に到達しない、かつ `#[should_panic]` がない | 結果が可変参照・グローバル状態経由で検証される可能性がある場合 |
| DA-004 自己比較 | `assert_eq!(a, b)` で a と b がトークン列として同一 | 該当 assert が存在する | なし（構文的に確定） |
| DA-005 空テスト | 関数本体に文が存在しない | 該当 | なし |
| DA-006 検証構文なし | 関数内に assert 相当が1つも存在しない | 該当 | なし |
| W-DA-101 ignored | `#[ignore]` 属性 | （FAILにしない。警告のみ。実行されなければ `test_execution` が NOT_EXECUTED になる） | |

DA-002 / DA-003 のデータフロー解析は関数内のローカル束縛の追跡（let 束縛、メソッドチェーン、フィールドアクセス）までとし、クロージャ内・マクロ展開内は UNKNOWN とする。
ルールごとの判定結果と根拠（該当スパン）は監査レコード（kind: `static`、auditor.kind: `deterministic`）として保存する。

DA-001〜DA-006 で FAIL した Test は、意味監査バンドルの生成対象から除外できる（`vtest audit bundle` は既定でスキップし、`--include-failed` で強制生成できる）。

---

## 8. 意味監査プロトコル

### 8.1 バンドル生成

`vtest audit bundle` は監査種別ごとに、判定に必要な情報を JSON として `cache/bundles/<ULID>.json` へ出力する。
バンドルは派生情報であり Git 管理しない。
提出結果の検証に必要な情報（対象の内容ハッシュ）は監査レコードへ複製されるため、バンドル自体の永続化は不要である。

各種別のバンドル内容は次のとおり。

| kind | 対象指定 | 含める情報 |
|---|---|---|
| `test-semantic` | `--test TEST-X` | Test（アノテーション・ソース全文）、covers 先 VO レコード、target の対象関数ソース全文、関連 Test の id と intent、同一 VO を covers する他 Test の一覧、決定論的監査の結果、有効な過去監査の要約 |
| `vo-coverage` | `--vo VO-X` または `--req REQ-X` | 対象 VO 部分木の全レコード、対応 REQ レコード、spec_refs（SPEC の path・sha256・節参照。文書本文は含めず、監査エージェントがリポジトリ内で読む）、各 leaf VO の covers 状況 |
| `impl-consistency` | `--test TEST-X` または `--vo VO-X` | 対象 VO レコード、spec_refs、対象関数ソース全文とシグネチャ、関連 Test の intent |

`impl-consistency` のバンドル生成時、対象シンボルが解決できない場合はバンドルを生成せず、`impl_consistency` を `MISSING` として記録する（基本仕様 §7.5）。

### 8.2 バンドル JSON スキーマ（test-semantic の例）

```json
{
  "bundle_id": "01J8XVYY...",
  "kind": "test-semantic",
  "generated_at": "2026-08-08T00:00:00Z",
  "revision": { "commit": "abc123...", "dirty": false },
  "test": {
    "id": "TEST-PARSER-044",
    "intent": "不正な UTF-8 入力を与えた場合、ParseError::InvalidUtf8 を返すことを検証する",
    "annotations": { "input": "...", "expect": "...", "kind": "unit-error", "cases": [] },
    "location": { "file": "tests/parser_test.rs", "function": "rejects_invalid_utf8" },
    "source": "/// @vtest.id ...\n#[test]\nfn rejects_invalid_utf8() { ... }",
    "content_hash": "sha256:..."
  },
  "vos": [
    { "id": "VO-PARSER-UTF8-003", "claim": "...", "dimensions": [],
      "spec_refs": [{ "spec": "SPEC-BASIC-001", "section": "4.2" }],
      "content_hash": "sha256:..." }
  ],
  "target": {
    "locator": "src/parser.rs::Parser::parse",
    "source": "pub fn parse(...) { ... }",
    "content_hash": "sha256:..."
  },
  "related_tests": [ { "id": "TEST-PARSER-003", "intent": "..." } ],
  "sibling_tests": [ { "id": "TEST-PARSER-045", "intent": "..." } ],
  "static_audit": { "verdict": "PASS", "rules": [] },
  "prior_audits": [ { "id": "01J...", "verdict": "FAIL", "audited_at": "...", "valid": false } ]
}
```

### 8.3 提出スキーマ

`vtest audit submit --file result.json` で提出する。

```json
{
  "bundle_id": "01J8XVYY...",
  "kind": "test-semantic",
  "verdict": "PASS",
  "reasons": [
    {
      "claim": "テストは不正 continuation byte 入力に対し InvalidUtf8 の返却を検証している",
      "basis": [
        { "kind": "test-code", "ref": "tests/parser_test.rs::rejects_invalid_utf8" },
        { "kind": "vo", "ref": "VO-PARSER-UTF8-003" }
      ]
    }
  ],
  "exclusions": [
    { "item": "overlong encoding の検証", "basis": "SPEC-BASIC-001#4.2 により対象外" }
  ],
  "auditor": { "kind": "agent", "id": "auditor-agent-01", "model": "claude-fable-5" },
  "confidence": "high"
}
```

`vo-coverage` の提出では `verdict` に `COMPLETE` / `INCOMPLETE` / `UNKNOWN` を用い、内部で `PASS` / `FAIL` / `UNKNOWN` へ写像する。
さらに `reasons` に次の構造を必須とする（基本仕様 §7.4）。

```json
{
  "reasons": [
    {
      "claim": "四則演算のため add / sub / mul / div の4軸へ分解した",
      "kind": "decomposition-viewpoint",
      "basis": [ { "kind": "spec", "ref": "SPEC-BASIC-001#3.1" } ]
    }
  ],
  "exclusions": [
    { "item": "括弧による優先順位", "basis": "SPEC-BASIC-001#4.2 により対象外" }
  ]
}
```

`basis.kind` は `spec` / `vo` / `req` / `test-code` / `target-code` のいずれかとする。

### 8.4 提出の検証

`audit submit` は次を順に検証し、失敗した場合は §17 のエラーコードで拒否する。

```text
1. bundle_id のバンドルが cache に存在する      （E-AUDIT-001）
2. kind がバンドルと一致する                    （E-AUDIT-003）
3. バンドル記録時の各対象の内容ハッシュが、
   現在のハッシュと一致する（対象が変更されて
   いれば監査は無効）                            （E-AUDIT-002）
4. verdict が種別ごとの許容値である             （E-AUDIT-004）
5. reasons が空でなく、各要素に claim と
   basis（1件以上）がある                        （E-AUDIT-005）
6. vo-coverage の場合、decomposition-viewpoint
   を1件以上含み、spec 参照の basis を持つ       （E-AUDIT-006）
```

受理された提出は監査レコード（§3.6）として保存される。
`subjects` にはバンドル生成時の全対象（Test・VO・対象関数、vo-coverage では SPEC の sha256）を記録する。

### 8.5 有効性と多重監査

監査レコードの有効性は判定時に評価する。

```text
有効 = subjects の全ハッシュが現在の内容ハッシュと一致する
（SPEC は登録された sha256 と実ファイルの一致も要求。
 不一致の場合は W-SCAN-104 を出し、当該レコードは STALE）
```

同一対象に有効なレコードが複数ある場合、FAIL が1件でもあれば FAIL、なければ最新の verdict を採る。
有効なレコードが1件もなければ `NOT_CHECKED`（一度も監査されていない）または `STALE`（無効化されたレコードのみ存在）とする。
再監査・多重監査の回数はツールとして制限しない（運用ポリシー）。

### 8.6 参考プロンプト

監査エージェントのプロンプト・スキル構成はツールの責務外だが、参考として test-semantic 用の骨子を示す。

```text
あなたはテスト意味監査者である。添付のバンドルについて、
以下だけを判定せよ。修正方針の提案はしない。

判定事項：
テストコードは、VO の claim と Test Intent が宣言する
振る舞いを実際に検証しているか。

判定は PASS / FAIL / UNKNOWN のいずれかとし、
判定ごとに claim（何を確認したか）と basis（根拠にした
バンドル内の情報への参照）を列挙せよ。
入力ケース集合が VO の要求する入力空間を代表しているかも
確認し、不足があれば FAIL の理由として挙げよ。
```

---

## 9. テスト実行設計

### 9.1 実行対象の解決

`vtest run` は `--test` / `--vo` / `--req` / `--all` で対象を受け取り、検証グラフから Test 集合へ展開する（VO / REQ 指定は部分木の covers を辿る）。

### 9.2 cargo test の起動

スキャナは各 Test について次を記録している（§5.2）。

- `package`：cargo package 名（ファイルパスから該当 `Cargo.toml` を特定）
- `test_target`：`Lib` / `Bin(name)` / `IntegrationTest(name)`（`tests/foo.rs` → `IntegrationTest("foo")`）
- `filter`：テストターゲットのルートからのモジュールパス＋関数名（例：`parser::tests::rejects_invalid_utf8`）

実行は（package, test_target）で分けたバッチとし、libtest の `--exact` フラグと複数フィルタを用いる。

```text
cargo test -p <package> --lib -- --exact <filter1> <filter2> ...
（IntegrationTest の場合は --lib の代わりに --test <name>）
```

`--exact` は後続の全フィルタへ適用されるフラグであり、各フィルタは完全一致で解釈される。

### 9.3 結果のパース

stdout を次の規則でパースする（stable toolchain の標準出力形式のみに依存する）。

```text
running N tests            → 実行対象数の確認
test <filter> ... ok       → PASS
test <filter> ... FAILED   → FAIL
test <filter> ... ignored  → NOT_EXECUTED（Evidence は記録しない）
```

要求した各フィルタについて結果行が得られなかった場合、その Test の実行は失敗（E-EXEC-002）とし、Evidence を記録しない。
プロセス終了コードと結果行の集計が矛盾する場合も E-EXEC-003 とする。
stdout / stderr の全文は `cache/logs/<ULID>.log` へ保存し、Evidence の `log_ref` から参照する。

### 9.4 Evidence の記録

Test ごとに §3.7 のレコードを1件生成する。

- `revision`：実行直前に `git rev-parse HEAD` と `git status --porcelain` で取得。取得失敗時は `commit: null` とし、この Evidence は `evidence_validity` で PASS にならない。
- `hashes`：実行直前のスキャン結果から、テスト関数と対象関数の内容ハッシュ（§1.3）を記録。
- ビルド失敗（コンパイルエラー）の場合、対象 Test 群の Evidence は記録せず E-EXEC-001 を報告する。`test_execution` は `NOT_EXECUTED` のままとなる。

---

## 10. Target Execution Verification

### 10.1 計測方式

`cargo-llvm-cov` を使用する（`config.run.coverage: llvm-cov`）。
起動時に `cargo llvm-cov --version` で利用可否を確認し、利用不能なら計測せず、`target_execution` を `NOT_CHECKED` とし診断 W-EXEC-101 を出す（PASS へ変換しない。基本仕様 §7.9）。

カバレッジを Test 単位で対象関数へ帰属させるため、計測時は Test を1件ずつ実行する。

```text
cargo llvm-cov test -p <package> --lib --json
  --output-path cache/cov/<ULID>.json
  -- --exact <filter>
```

### 10.2 判定

出力 JSON（llvm-cov export 形式）の `data[].functions[]` から対象関数を検索する。

```text
一致条件：
  demangle 済み関数名の末尾が locator の item-path と一致し、
  かつ filenames のいずれかの末尾が locator の path と一致する

ジェネリック関数は複数インスタンスが現れるため、count を合算する。

判定：
  count > 0        → PASS
  count == 0       → FAIL
  関数が見つからない → UNKNOWN（インライン化・cfg 除外等の可能性）
```

結果は Evidence の `target_execution` フィールドへ記録する。

### 10.3 実行モードの整理

`vtest run` は2モードを持つ。

- `--fast`：cargo test のみ。`target_execution.checked: false` で記録し、検証時は `NOT_CHECKED`。
- 既定（完全検証向け）：cargo-llvm-cov による Test 単位実行。実行時間と引き換えに `target_execution` を判定する。

---

## 11. 鮮度検証と集約

### 11.1 チェック項目の評価地点

11のチェック項目（基本仕様 §4.2）は、次の地点で評価する。

| チェック項目 | 評価地点 | 評価方法 |
|---|---|---|
| `spec_coverage` | REQ | REQ に対応する VO が1件以上存在すれば PASS、なければ MISSING |
| `vo_decomposition` | VO | 部分木に §5.4 の error 診断がなければ PASS |
| `vo_coverage` | VO | 有効な vo-coverage 監査（§8.5）が PASS かつ VO が承認済（§3.5）なら PASS |
| `test_existence` | leaf VO | covers する Test が1件以上あれば PASS、なければ MISSING |
| `static_audit` | TEST | §7.1 の合成 |
| `semantic_audit` | TEST | 有効な test-semantic 監査の合成（§8.5） |
| `impl_consistency` | TEST / VO | 有効な impl-consistency 監査の合成。対象シンボル不在は MISSING |
| `test_execution` | TEST | 有効な Evidence が存在すれば PASS、なければ NOT_EXECUTED |
| `runtime_result` | TEST | 有効な Evidence の result（PASS / FAIL） |
| `target_execution` | TEST | 有効な Evidence の target_execution（checked: false は NOT_CHECKED） |
| `evidence_validity` | TEST | §11.2 の判定 |

### 11.2 Evidence 鮮度判定

```text
対象 Test の Evidence のうち最新のものについて：

1. evidence.hashes.test_fn == 現在のテスト関数ハッシュ
2. evidence.hashes.target_fn == 現在の対象関数ハッシュ
3. evidence.revision.commit が非 null

1〜3 すべて成立 → evidence_validity = PASS
  （dirty: true でもハッシュ一致なら有効。ハッシュが実体を保証する）
1 または 2 不成立 → STALE
3 不成立          → FAIL（リビジョン不明の実行）
Evidence なし     → NOT_EXECUTED
```

`evidence_validity` が PASS でない場合、`runtime_result` と `target_execution` は当該 Evidence から値を採らず、それぞれ `STALE`（Evidence があるが無効）または `NOT_EXECUTED`（Evidence なし）とする。

### 11.3 集約アルゴリズム

```text
fn aggregate(scope) -> Report:
  1. scan によりグラフ構築（§5）
  2. scope のエンティティ軸で REQ/VO/TEST 部分木を選択
  3. 各 TEST について、scope のチェック項目軸に含まれる
     TEST 評価項目を評価（含まれない項目は NOT_CHECKED）
  4. 各 leaf VO について VO 評価項目を評価し、
     covers する TEST 群の結果を fail-closed で合成
  5. 中間 VO は子 VO の合成（fail-closed）
  6. REQ は spec_coverage と VO 部分木の合成
  7. 総合判定：scope 内の全評価が PASS → OK、それ以外 → NG

fail-closed 合成：
  子に FAIL/MISMATCH/MISSING/STALE/NOT_EXECUTED/
  NOT_CHECKED/UNKNOWN が1つでもあれば親は非 PASS。
  代表値は基本仕様 §4.3 の優先順位で選ぶ。
```

複数 VO を covers する Test の結果は、covers 先それぞれの VO の合成に独立に参加する。
「1つの Test が複数 VO を検証していること」自体は許容し、各 leaf VO の `test_existence` と組合せの充足は §3.3.1 の実体化された leaf VO 単位で判定する（基本仕様 §7.6）。

### 11.4 SPEC 鮮度

スキャン時に SPEC レコードの `sha256` と実ファイルを比較し、不一致なら W-SCAN-104 を出す。
当該 SPEC を subjects に含む監査レコードは無効（STALE）となる。
仕様文書の更新は `vtest spec add --update` による再登録で反映し、依存する監査・承認が失効することを利用者へ提示する。

---

## 16. 並列動作と整合性

（§12〜§15 は別紙A参照）

### 16.1 ロック不要の根拠

書き込み操作は次のいずれかに分類され、ファイルロックを必要としない。

- **新規レコード追加**（rel / approvals / audits / evidence）：ULID ファイル名の新規作成のみ。並列生成は衝突しない。
- **エンティティファイル編集**（spec / req / vo）：1 エンティティ 1 ファイル。異なるエンティティの並列編集は独立。同一エンティティの並列編集は Git のマージ衝突として顕在化する。
- **テストコード編集**：通常のソース編集と同じ扱い。

同時実行された `vtest` プロセス同士の調停は行わない。
すべての判定は「その時点の正典の読み取り」に基づき、正典が変われば次回の scan / verify が差分を反映する。

### 16.2 マージ後の意味的衝突検出

Git マージが成功しても、論理的な不整合（別ブランチで同じ Test ID を採番、covers 先 VO の削除、承認済 VO の変更）が残りうる。
これらはすべて次で検出される。

- ID 衝突 → E-SCAN-002
- dangling reference → E-SCAN-003 / E-SCAN-009
- 承認の失効 → §3.5 のハッシュ束縛により自動的に draft へ
- 監査・Evidence の失効 → §8.5 / §11.2 のハッシュ束縛により自動的に STALE へ

CI はマージ後に `vtest doctor` を実行し、終了コードで意味的衝突を検知することを推奨する。

---

## 17. 診断・終了コード体系

### 17.1 診断コード

§5.4 のスキャン診断に加え、次を定義する。

| コード | 種別 | 内容 |
|---|---|---|
| W-SCAN-104 | warning | SPEC レコードの sha256 と実ファイルの不一致（依存する監査・承認は STALE） |
| E-EXEC-001 | error | テストビルド失敗 |
| E-EXEC-002 | error | 要求したテストの結果行が得られない |
| E-EXEC-003 | error | 終了コードと結果行集計の矛盾 |
| W-EXEC-101 | warning | カバレッジツール利用不能（target_execution は NOT_CHECKED） |
| E-AUDIT-001 | error | 提出された bundle_id が存在しない |
| E-AUDIT-002 | error | バンドル記録時のハッシュと現在のハッシュの不一致（対象が変更済） |
| E-AUDIT-003 | error | kind の不一致・スキーマ違反 |
| E-AUDIT-004 | error | verdict が許容値でない |
| E-AUDIT-005 | error | reasons が空、または claim / basis を欠く |
| E-AUDIT-006 | error | vo-coverage で decomposition-viewpoint / spec 参照を欠く |
| E-OP-001 | error | Structured Operation の入力検証失敗（候補提示を伴う。§6.2） |
| E-OP-002 | error | Edit 対象 Test の特定失敗 |
| E-OP-003 | error | 編集結果が1 Test の範囲を超える（§15.4。操作は中止される） |

### 17.2 終了コード

| コード | 意味 |
|---|---|
| 0 | 要求 scope の検証結果が OK（操作コマンドでは成功） |
| 1 | 検証結果が NG |
| 2 | 入力・使用方法エラー（診断コード E-OP-*、引数不正、スキーマ違反の提出など） |
| 3 | 内部エラー（ツール自体の異常） |

---

## 19. 本書で未決とし実装時判断へ委譲する事項

- `serde_yaml` 後継クレートの選定
- demangle 実装（`rustc-demangle`）の適用範囲
- `#[tokio::test]` 等、属性末尾 `test` 以外のカスタムテスト属性への対応範囲
- cargo workspace 外の単一クレートプロジェクトでのパス解決の細部
- レポートのツリー描画の細部（文字種、折返し）

次の事項は初期リリースの対象外とし、将来課題とする。

- LSP / rust-analyzer 連携によるシンボル解決
- 永続インデックス（`cache/` の活用）
- Relation の tombstone 方式
- Rust 以外の言語対応（概念モデルは言語非依存に保つ）
- API 直接呼び出しによる監査の内部実行（基本仕様 §7.3 の拡張点）
- rename 追跡と SRC 恒久 ID の自動昇格支援
- cargo-nextest 対応
