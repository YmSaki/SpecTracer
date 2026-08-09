# AI並列開発向けテスト検証システム 詳細設計 v0.1（本冊）

## 0. 本書の位置付け

本書は「基本仕様 v0.1」を実装可能なレベルまで具体化する。
基本仕様が定めた外部挙動の保証を変更しない。
本書と基本仕様の間に矛盾がある場合、基本仕様を正とし、本書の該当箇所を不整合として扱う。

### 分冊構成

正規の詳細設計は3分冊とし、節番号は正規文書間を通した連番とする。別紙Bは非正規のprocess文書として別に扱う。

| 文書 | 位置付け | 収録節 |
|---|---|---|
| 本冊（コア設計） | 正規 | §1〜§11、§16、§17、§19 |
| 別紙A（CLI・MCPインターフェース仕様） | 正規 | §12〜§15 |
| 別紙B（実装計画） | 非正規 / process | 正規節番号を持たない |
| 別紙C（受入仕様） | 正規 | §18 |

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
    vtest-adapter-api/  # 言語・runner非依存のcapability契約とregistry
    vtest-adapter-rust/ # rust-cargo discovery/audit/operations/runner/coverage
    vtest-scan/         # discovery委譲、結果統合、record整合性
    vtest-audit/        # audit委譲、監査bundle生成、提出結果検証
    vtest-exec/         # runner委譲、revision取得、Evidence記録
    vtest-verify/       # 整合性検査、鮮度検証、集約、レポート生成
    vtest-cli/          # バイナリ vtest（clap によるCLI、mcp サブコマンド含む）
    vtest-mcp/          # MCPサーバ実装（vtest-cli から起動）
  tests/
    fixtures/           # 検証用サンプルプロジェクト（§18 受入基準で使用）
```

`vtest-adapter-api` は `vtest-model` 以外の言語実装・Cargo実装へ依存しない。
`vtest-scan`、`vtest-audit`、`vtest-exec` はadapterを選択・委譲するorchestrationであり、
それぞれが `syn`、`quote`、`rustc-demangle`、Cargo commandを直接所有しない。
`vtest-store`はForm Schemaの中立parserとcanonical保存だけを提供し、組込Rust formの
内容と配置は `vtest-adapter-rust` が所有する。

依存方向は `cli / mcp → verify / exec / audit / scan → store → model` を維持し、
言語固有能力は `scan / audit / exec → adapter-rust → adapter-api → model` とする。
`adapter-rust → store` はForm Schemaとcanonical layoutの利用に限る。

### 1.2 主要依存クレート

| 用途 | クレート | 備考 |
|---|---|---|
| Rust構文解析 | `syn` 2.x（features: `full`, `extra-traits`, `visit`） | `vtest-adapter-rust`が所有するAST解析 |
| Rustスパン位置 | `proc-macro2`（feature: `span-locations`） | `vtest-adapter-rust`が所有する編集・ハッシュ対象範囲の特定 |
| CLI | `clap` 4.x（derive） | |
| シリアライズ | `serde`, `serde_json` | |
| YAML | `serde_yaml` | レコードファイル |
| ID | `ulid` | レコードID |
| ハッシュ | `sha2` | 内容ハッシュ（SHA-256） |
| Rust source走査 | `ignore` | `vtest-adapter-rust`が所有する`.gitignore`準拠の走査 |
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
  - Test：discovery adapterが単一Test constructに対して返すsource range全体のtestデータ。metadata宣言がsource rangeに隣接する場合はその宣言もrangeに含める
  - Source Target：discovery adapterがTarget Referenceに対して返すimplementation construct全体のsource range
  - VO / REQ / SPEC レコード：YAML ファイル全体
- 空白の正規化はこれ以上行わない。インデント変更はハッシュ不一致となる（安全側）。

adapterはsource rangeとそのバイト列を返し、coreは上記の正規化とSHA-256計算だけを行う。coreがASTや言語固有の構文からrangeを再計算しない。`rust-cargo` adapterはTestにdoc comment・属性を含む関数item全体、Source Targetに属性とdoc commentを含む関数item全体を返す。

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
  # 完全検証のチェック項目（基本仕様 §4.2 の12項目。通常は変更しない）
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
    - test_traceability
run:
  # target_execution 計測方式: llvm-cov | off
  coverage: llvm-cov
```

config writerはversion 2を正規形とする。readerはversion 1を
in-memoryで単一 `rust-cargo` adapterへ変換し、読み取りだけでcanonical configを書き換えない。

```yaml
version: 2
project:
  name: example
adapters:
  - id: rust-cargo
    roots: ["."]
    scan:
      include: [src, tests, crates]
      assertion_macros: []
    run:
      coverage: llvm-cov
verify:
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
    - test_traceability
```

adapter IDの重複、同一adapter内のroot重複、未知adapter、無効なadapter設定はusage errorとする。
異なるadapterが同じrootを共有することはpolyglot repositoryのために許可し、統合したTest IDは
全adapterでglobal uniquenessを検査する。adapter固有設定の検証は登録adapterへ委譲し、
coreは未知のnamespaceや値をRust設定として解釈しない。
`vtest init`はversion 2を生成する。
`scan` と `run` はversion 1 schema互換のwire値とする。Rust固有のmacro pathや
`llvm-cov`制約は `rust-cargo` adapterに限って適用する。非Rust namespaceの値を
coreがRust設定として推測・書換えしてはならない。

### 2.3 派生情報

検証グラフとindexは実行のたびにインメモリで再構築する。
永続cacheを正典または検証入力として使用しない。`cache/`は再生成可能な派生物だけを格納する。
MCP サーバは長時間動作するため、ツール呼び出しごとに対象ファイルの mtime を確認し、変化があれば再スキャンする。

---

## 3. レコードファイルスキーマ

すべてのレコードは YAML とし、未知フィールドはエラーではなく警告とする。
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
  - target: "rust-cargo::src/parser.rs::Parser::parse"
    hash: "sha256:..."
verdict: PASS                   # PASS | FAIL | UNKNOWN
reasons:                        # §8.3 の構造。static では規則違反の一覧
  - claim: テストは不正UTF-8入力に対する InvalidUtf8 の返却を検証している
    basis:
      - kind: test-code
        ref: "rust-cargo::tests/parser_test.rs::rejects_invalid_utf8"
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
adapter: rust-cargo
result: PASS                    # PASS | FAIL
executed_at: 2026-08-08T00:00:00Z
revision: { commit: "abc123...", dirty: false }
hashes:
  test_construct: "sha256:..."
  targets:
    - target: "rust-cargo::src/parser.rs::Parser::parse"
      target_construct: "sha256:..."
    - target: "rust-cargo::src/lexer.rs::Lexer::next"
      target_construct: "sha256:..."
runner:
  kind: cargo-test
  command: "cargo test -p parser --lib -- --exact parser::tests::rejects_invalid_utf8"
  exit_code: 0
target_execution:
  checked: true                 # 計測を実施したか
  method: llvm-cov
  result: FAIL                  # target別結果の集約: PASS | FAIL | UNKNOWN
  targets:
    - target: "rust-cargo::src/parser.rs::Parser::parse"
      result: PASS              # PASS | FAIL | UNKNOWN
      count: 3                  # UNKNOWNではnull
    - target: "rust-cargo::src/lexer.rs::Lexer::next"
      result: FAIL
      count: 0
log_ref: "cache/logs/01J8XW1B.log"   # Git管理外の生ログ
```

`hashes.targets`はTestの宣言順で常に記録し、各`target`は正規化したTargetRef文字列表現とする。
このlistはTestが宣言するtarget集合と重複なく1対1に対応する。`target_execution.checked: true`では
`target_execution.targets`も同じ順序・target集合で1対1に対応する。
`target_execution.checked: false`では`method`と`result`をnull、`targets`を空listとし、検証値を
`NOT_CHECKED`とする。

readerは`rust-cargo` Evidenceに限り、互換fieldの`hashes.test_fn`とtarget entry内の`target_fn`を`test_construct`と`target_construct`へ正規化できる。両fieldが併存する場合は同値を必須とし、非`rust-cargo` Evidenceでは互換fieldを解釈しない。
readerは単数互換形の`hashes.target_fn`および`target_execution.result/count`を、現在の`rust-cargo` Testがtargetをちょうど1件宣言し、Test construct hashとtarget construct hashを照合できる場合だけ1要素listへ正規化して扱う。
複数target Testに単数互換形を適用せず、writerは常にlist形を出力する。

Evidence内の`target`は実行時snapshotを識別するkeyであり、TEST → SRC edgeの正典ではない。
graphはadapter所有のTest metadata宣言からだけ構築し、Evidenceのtarget listからedgeを生成しない。

schema違反、target entryの欠落・重複・余剰、またはaggregate resultとtarget別結果の矛盾は
E-SCAN-010として扱い、そのEvidenceを有効な結果に使用しない。

Evidence writerは `adapter` を必須で記録し、保存前にTestの
`ExecutionDescriptor.adapter`およびrunner kindとの整合を検証する。Evidence readerは
`adapter` の欠落を許容するが、現在のTestが `rust-cargo` で、互換runner kindと内容ハッシュから
Rust実行であることを一意に確認できる場合だけ互換Evidenceとして扱う。確認不能は
`UNKNOWN`、明示adapterの不一致は `MISMATCH` とし、いずれもPASSへ昇格しない。

---

## 4. Test metadata宣言contract

### 4.1 adapter-neutralな正規化

`SourceDiscoveryAdapter`は、adapter所有のsource declarationを次の論理fieldへ正規化する。

```text
id, covers[], targets[], intent, input?, expect?, kind?, cases[], related[]
```

coreはsource declarationの構文と配置を解釈せず、adapterが返したTest Entity、Discovered Test observation、Source Location、Target Reference、source range、診断を検証・統合する。
locatorは`TargetRef::Locator { adapter, value }`とし、`value`はadapter所有のopaque文字列である。coreがpath、module、symbol種別を分解しない。

### 4.2 `rust-cargo` annotation文法

テスト関数直前の doc comment（`///` または `/** */`）内の行を対象とする。

```text
annotation-line = "@vtest." key SP value
key             = "id" | "covers" | "target" | "intent" | "input"
                | "expect" | "kind" | "case" | "related" | "src-id"
value           = 行末までのテキスト（前後空白は除去）
```

- 1行1キー。`covers` と `related` の値はカンマ区切りで複数指定できる。
- `case` と `related` はキー自体を複数行書ける。他のキーの重複はエラー E-SCAN-005。ただし `kind` が integration 系の Test に限り、`target` の複数行を許容する（別紙A §14.3）。許容された複数`target`内でも同じTargetRefの重複はE-SCAN-005とする。
- `@vtest.` で始まるが未知のキーを持つ行はエラー E-SCAN-006（打鍵ミスの検出を優先し、警告ではなくエラーとする）。
- doc comment 内の `@vtest.` を含まない行は自由記述として無視する。
- `@vtest.src-id` はテストではなく対象実装側の関数に付与し、任意の恒久SRC IDを宣言する。scannerは指定値を認識するが、付与を必須としない（基本仕様 §3.3）。

### 4.3 `rust-cargo` locator構文

```text
locator   = path "::" item-path
path      = プロジェクトルートからの相対パス（"/" 区切り、".rs" で終わる）
item-path = Rust アイテムパス（"::" 区切り）
            impl ブロック内の関数は "型名::関数名"

例：src/parser.rs::Parser::parse
    src/lib.rs::validate_input
```

`path` は `.rs` で終わる最初の `::` で item-path と分離する。
`rust-cargo` adapterはこの値を`TargetRef::Locator { adapter: "rust-cargo", value: locator }`へ正規化する。`@vtest.target`の値が`SRC-`で始まる場合はSRC ID参照として返す。

### 4.4 宣言エラーの扱い

adapter固有のsource declarationを構文解析できない場合、adapterは該当Test constructをDiscovered Testとして返し、対応を`ManagedTestLink::Missing`として診断を付与する。coreは`test_traceability = MISSING`とし、対応VOを推測で`test_existence`へ関連付けない。

source declarationを構文上完全なTest Entityへ正規化できるが、`covers`のVO IDをcore storeで解決できない場合、そのentityと`ManagedTestLink::One(id)`を保持する。E-SCAN-003と`test_traceability = MISMATCH`はcoreの参照整合性検査で生成する。

`rust-cargo` annotationの構文違反は§5.4のE-SCAN-005、E-SCAN-006、E-SCAN-007で報告する。

---

## 5. Discovery orchestration設計

### 5.1 処理フロー

```text
1. registryとconfigの検証
   adapter ID、capability宣言、config namespace、rootを検証する

2. discovery委譲
   登録順ではなくadapter ID順にSourceDiscoveryAdapterを呼び出す
   各adapterはDiscoveryBatchを返す

3. adapter出力の検証
   adapter ID、Source Location、source range、content bytes、Test Entity、
   Discovered TestとManagedTestLinkの対応、Target Reference、診断を検証する
   capability宣言と出力が矛盾するbatchは拒否する

4. 決定論的な統合
   adapter ID、project-relative path、opaque locator、Test IDの順に正規化する
   adapter間を含むTest ID衝突と不正な複数対応を検査する

5. .verify/ 読み込み
   vtest-storeが全レコードを読み込み、スキーマ検証する

6. 参照整合性検査
   coversのVO ID、targetsのTarget Reference / SRC ID、Relation、parentを解決する

7. グラフ構築と整合性検査（§5.3、§5.4）
```

adapterが解析不能または不完全なbatchを返した場合、coreは対応する検証を`UNKNOWN`とし、Test 0件の完全なdiscoveryとして扱わない。

### 5.2 エンティティモデル（vtest-model）

```rust
pub struct TestEntity {
    pub id: TestId,
    pub covers: Vec<VoId>,
    pub targets: Vec<TargetRef>,    // 各要素はadapter付きopaque locatorまたはSrcId、1件以上
    pub intent: String,
    pub input: Option<String>,
    pub expect: Option<String>,
    pub kind: Option<String>,
    pub cases: Vec<String>,
    pub related: Vec<TestId>,
    pub location: SourceLocation,
    pub content_hash: ContentHash,  // §1.3
    pub execution: ExecutionDescriptor,
}

pub enum TargetRef {
    Locator { adapter: AdapterId, value: String },
    SrcId(SrcId),
}

pub struct SourceLocation {
    pub adapter: AdapterId,
    pub path: ProjectPath,
    pub locator: String,            // adapter所有のopaque construct locator
    pub byte_range: SourceRange,
}

pub struct ExecutionDescriptor {
    pub adapter: AdapterId,
    pub project: Option<String>,
    pub suite: Option<TestSuite>,
    pub selector: String,
}

pub struct TestSuite {
    pub kind: String,
    pub name: Option<String>,
}

pub enum CheckValue {
    Pass, Fail, Mismatch, Missing,
    NotChecked, NotExecuted, Stale, Unknown,
}

pub enum CheckItem {
    SpecCoverage, VoDecomposition, VoCoverage, TestExistence,
    StaticAudit, SemanticAudit, ImplConsistency,
    TestExecution, RuntimeResult, TargetExecution, EvidenceValidity,
    TestTraceability,
}
```

`TestEntity.execution` はadapter、project、suite、opaque selectorからなる中立な実行座標である。
coreは `project`、`suite.kind`、`suite.name`、`selector` の文字列を解釈しない。
`filter`、`package`、`test_target`および`TestTarget`型を`vtest-model`へ置かない。

`vtest-adapter-api`は言語非依存の`TestWireCodec` capabilityを定義する。codecはadapter固有の
compatibility propertyをJSON objectとしてencode / decodeできるが、core domain typeへadapter固有fieldを
追加しない。`rust-cargo` codecはversion 1互換の`filter`、`package`、`test_target`を所有する。

JSON writerは`execution`を常に出力し、`rust-cargo` TestだけにRust互換fieldを追加する。
非Rust TestではRust互換fieldを省略する。JSON readerは`execution`を優先し、互換field併存時は
descriptorとの一致を検証する。`execution`が欠ける場合、完全で相互整合するRust互換fieldからだけ
`rust-cargo` descriptorを導出する。不完全・矛盾時は入力を拒否し、空selectorまたはdummy値を生成しない。

Test JSON writerは`TestEntity.targets`を1件以上のlistとして常に出力する。targetが1件の場合だけ
同値の単数互換field`target`を追加できる。readerは`target`だけの入力を1要素listへ正規化し、
`targets`との併存時は完全一致を検証する。複数targetから代表値を選んで`target`を生成しない。

scan時の導出結果には、Managed Test Entityとは別に次のin-memory observationを保持する。

```rust
pub struct DiscoveredTest {
    pub adapter: AdapterId,
    pub location: SourceLocation,
    pub content_hash: ContentHash,
    pub managed: ManagedTestLink,
}

pub enum ManagedTestLink {
    Missing,
    One(TestId),
    Multiple(Vec<TestId>),
}

pub struct DiscoveryBatch {
    pub adapter: AdapterId,
    pub completeness: DiscoveryCompleteness,
    pub discovered_tests: Vec<DiscoveredTest>,
    pub managed_tests: Vec<TestEntity>,
    pub source_targets: Vec<SourceTarget>,
    pub diagnostics: Vec<Diagnostic>,
}

pub enum DiscoveryCompleteness {
    Complete,
    Incomplete,
}
```

`SourceDiscoveryAdapter`はadapterがTestとして認識した全Discovered Testを返す。`ManagedTestLink::One`は、構文上有効なTest ID、1件以上の`covers`、その他の必須metadataを持つTest Entityへ対応する場合に設定する。
VO参照の解決とTest IDの大局的一意性はadapterではなくcoreが検査する。したがって、解決不能な`covers`を持つTest Entityも`managed_tests`に含まれ、対応するobservationは`ManagedTestLink::One(id)`を持つ。
`ManagedTestLink::Missing`は管理宣言の欠落または必須metadataの欠落、`Multiple`は同一Test constructから複数entityが生じる状態を表す。

adapter capabilityは `SourceDiscoveryAdapter`、`TestWireCodec`、`StaticAuditAdapter`、
`StructuredTestAdapter`、`TestRunnerAdapter`、`CoverageAdapter` に分割する。
各adapterは一意なID、languages、capabilities、config namespaceを宣言し、registryは
宣言と実装の不一致および重複IDを拒否する。明示操作に必須のcapabilityがない場合は
E-ADAPTER-004で操作を中止する。検証集約では、static audit / coverage欠落は
`NOT_CHECKED`、runner欠落は `NOT_EXECUTED`、解析限界は `UNKNOWN` とする。

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
  TEST → VO     (covers)      ※adapter所有のTest metadata宣言由来
  TEST → SRC    (targets)     ※adapter所有のTest metadata宣言由来、1:N
  外部 Relation (rel/ 由来)
逆引きインデックス：VO → Tests、SRC → Tests、REQ → VOs
```

### 5.4 整合性診断

| コード | 種別 | 内容 |
|---|---|---|
| E-SCAN-001 | error | adapterのsource構文解析失敗（DiscoveryBatchは`Incomplete`） |
| E-SCAN-002 | error | Test ID 重複（identity collision） |
| E-SCAN-003 | error | `covers` の参照先 VO が存在しない（dangling reference） |
| E-SCAN-004 | error | `target` のロケータ／SRC ID を解決できない |
| E-SCAN-005 | error | adapter所有の宣言で重複不可fieldが重複 |
| E-SCAN-006 | error | adapter所有の宣言に未知fieldが存在 |
| E-SCAN-007 | error | 必須metadata（id / covers / targets / intent）の欠落 |
| E-SCAN-008 | error | VO / REQ の parent 不在または循環 |
| E-SCAN-009 | error | Relation の from / to が不在 |
| E-SCAN-010 | error | レコードの id とファイル名の不一致、スキーマ違反 |
| W-SCAN-101 | warning | adapterが発見したが管理宣言に対応しないTest construct（unregistered test） |
| W-SCAN-102 | warning | どの VO からも参照されず、Test も参照しない孤立 VO |
| W-SCAN-103 | warning | `covers` を持つが対応 VO が leaf でない（中間 VO 直接参照。許容するが警告） |
| W-STORE-001 | warning | VO の `status` フィールドと承認導出結果の不一致 |

error は該当エンティティに関わるチェック項目を非PASSにする。
warningは診断severityだけでは検証値を変更しないが、レポートに常に表示する。
W-SCAN-101またはE-SCAN-007が示す`ManagedTestLink::Missing`は、診断とは独立した`test_traceability`評価で`MISSING`になる。
`ManagedTestLink::Multiple`、E-SCAN-002のTest ID衝突、またはE-SCAN-003の解決不能なVO参照は`test_traceability = MISMATCH`とする。E-SCAN-003が発生しても対応するTest Entityと`ManagedTestLink::One`を除去しない。

### 5.5 `rust-cargo` SourceDiscoveryAdapter

`rust-cargo` adapterは次の処理で§5.1の`DiscoveryBatch`を構築する。`vtest-scan`はこれらのRust固有処理を実行しない。

```text
1. ファイル探索
   adapter configのinclude配下の*.rsをignoreクレートで列挙
   （.gitignore準拠、target/は除外）

2. 構文解析
   ファイルごとにsyn::parse_file
   解析エラーのファイルはE-SCAN-001を返し、batchをIncompleteとする

3. モジュールパス構築
   crateルート（src/lib.rs / src/main.rs / tests/*.rs）からmod宣言を辿り、
   各itemの完全モジュールパスを構築する

4. Test construct抽出
   属性pathの末尾segmentが"test"である関数（#[test]、#[tokio::test]等）を抽出する

5. metadata宣言抽出
   doc属性（#[doc = "..."]）を§4.2の文法でparseする

6. Source Target抽出
   すべてのfn / impl fnをSRC候補として索引化し、
   §4.3のlocator解決・逆引き・@vtest.src-id認識に使用する

7. 正規化
   全Discovered Test、構造上完全なTest Entity、Source Target、Source Location、
   source range、ExecutionDescriptor、診断をDiscoveryBatchに格納する
```

---

## 6. Target Reference解決

### 6.1 adapter-neutral解決contract

coreは`TargetRef::Locator.adapter`をregistryで解決し、opaque locatorの解釈を該当する`SourceDiscoveryAdapter`へ委譲する。adapterは正規化されたTarget Reference、Source Location、source range、content bytes、解決status、候補を返す。
coreは返却されたadapter IDとTarget Referenceの一致、source rangeの範囲、content hashを検証するが、opaque locatorの内部構文は解釈しない。解決が0件または複数候補で一意に定まらない場合はE-SCAN-004とし、推測で候補を選択しない。

SRC ID参照はcoreが統合済みSRC索引で一意性を検査し、対応するadapterのSource Locationとsource rangeを使用する。

### 6.2 `rust-cargo` locator解決

`rust-cargo`のlocator `path::item-path`の解決は、§5.5で構築したSRC索引への完全一致検索とする。

```text
1. path が索引に存在するか
2. path 内で item-path が一致する fn / impl fn が存在するか
3. 一意に決まらない場合（同名 fn が cfg 分岐で複数等）は
   すべて候補として返し、解決失敗（E-SCAN-004）とする
```

### 6.3 候補提示

Structured Operationの入力検証（§14、§15）で解決に失敗した場合、coreはadapterが返した候補を共通envelopeで表示する。`rust-cargo` adapterは次の順で候補を構築する。

```text
1. item-path の末尾セグメント一致（別パスの同名関数）
2. 編集距離 2 以内の近似名
出力例：
  ✗ symbol not found: src/parser.rs::Parser::prase
  candidates:
    src/parser.rs::Parser::parse
    src/parser.rs::Parser::parse_inner
```

`rust-cargo` adapterのenum variant検証（`expect`の値が`ParseError::InvalidUtf8`形式の場合）は、スキャン済みASTからenum定義を検索する。
解決できる場合のみ検証し、解決できない自由記述はそのまま受理する（best effort。拒否はしない）。

---

## 7. Static Audit orchestrationと`rust-cargo`ルール

### 7.1 判定の原則

各ルールは `FAIL` / `UNKNOWN` / `PASS(違反なし)` のいずれかを返す。
**決定論的に確定できる違反のみ FAIL とする。**
解析の限界で確定できない場合は FAIL ではなく UNKNOWN とし、意味監査へ委ねる。
Test の `static_audit` チェック項目は、全ルールが違反なしなら PASS、1つでも FAIL があれば FAIL、FAIL がなく UNKNOWN があれば UNKNOWN とする。

`vtest-audit`は`TestEntity.execution.adapter`をregistryで解決し、Test、全Target Reference、各source range、content hashを`StaticAuditAdapter`へ渡す。adapterはrule ID、verdict、根拠span、解析限界を返す。coreは入力hashと返却されたsubjectの一致を検証し、上記規則で集約するが、adapter固有のASTやassertion構文を解釈しない。

Static Audit capabilityがない場合は`NOT_CHECKED`、adapterが不完全または解析限界を報告した場合は`UNKNOWN`とし、違反なしと推測しない。

### 7.2 `rust-cargo` ルール一覧

`rust-cargo`の**assert相当の構文**は次のとおり定義し、DA-001〜DA-006で共通に用いる。

- `assert!` / `assert_eq!` / `assert_ne!` / `panic!`を含む標準マクロ、および`rust-cargo` configの`assertion_macros`に列挙されたマクロ
- `#[should_panic]`属性
- `.unwrap()` / `.expect(..)` / `?`演算子（Result / Optionの成立検証として扱う）
- Test関数が`Result`を返し`Err`を返しうる構造

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
複数target TestではDA-002 / DA-003を各targetへ個別適用する。target別結果に1件でもFAILがあればrule結果をFAIL、FAILがなく1件でもUNKNOWNがあればUNKNOWN、全targetが違反なしの場合だけPASSとする。
ルールごとの判定結果と根拠（該当スパン）は監査レコード（kind: `static`、auditor.kind: `deterministic`）として保存する。subjectsにはTestと全宣言targetの現在hashを含める。

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
| `test-semantic` | `--test TEST-X` | Test（metadata宣言・Test construct source全文）、covers先VOレコード、全targetのimplementation construct source全文、関連Testのidとintent、同一VOをcoversする他Testの一覧、決定論的監査の結果、有効な過去監査の要約 |
| `vo-coverage` | `--vo VO-X` または `--req REQ-X` | 対象 VO 部分木の全レコード、対応 REQ レコード、spec_refs（SPEC の path・sha256・節参照。文書本文は含めず、監査エージェントがリポジトリ内で読む）、各 leaf VO の covers 状況 |
| `impl-consistency` | `--test TEST-X` または `--vo VO-X` | 対象VOレコード、spec_refs、全targetのimplementation construct source全文とadapterが提供する構造情報、関連Testのintent |

`impl-consistency` のバンドル生成時、宣言targetのいずれかを解決できない場合はバンドルを生成せず、`impl_consistency` を `MISSING` として記録する（基本仕様 §7.5）。

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
    "metadata": { "input": "...", "expect": "...", "kind": "unit-error", "cases": [] },
    "location": {
      "adapter": "rust-cargo",
      "path": "tests/parser_test.rs",
      "locator": "rejects_invalid_utf8",
      "byte_range": { "start": 120, "end": 340 }
    },
    "source": "/// @vtest.id ...\n#[test]\nfn rejects_invalid_utf8() { ... }",
    "content_hash": "sha256:..."
  },
  "vos": [
    { "id": "VO-PARSER-UTF8-003", "claim": "...", "dimensions": [],
      "spec_refs": [{ "spec": "SPEC-BASIC-001", "section": "4.2" }],
      "content_hash": "sha256:..." }
  ],
  "targets": [
    {
      "target": "rust-cargo::src/parser.rs::Parser::parse",
      "source": "pub fn parse(...) { ... }",
      "content_hash": "sha256:..."
    }
  ],
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
        { "kind": "test-code", "ref": "rust-cargo::tests/parser_test.rs::rejects_invalid_utf8" },
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
`subjects`にはバンドル生成時の全対象（Test・VO・全targetのimplementation construct、vo-coverageではSPECのsha256）を記録する。

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

### 9.2 `rust-cargo` TestRunnerAdapter

`rust-cargo` adapterは`TestEntity.execution`を次のCargo実行座標として解釈する。

- `project`：cargo package名
- `suite.kind`：`lib` / `bin` / `integration`
- `suite.name`：bin名またはintegration test target名。`lib`では省略する。
- `selector`：test targetのrootからのmodule path＋function名（例：`parser::tests::rejects_invalid_utf8`）

adapter内部ではこれらからCargo launch coordinateを構築する。`TestEntity`へCargo固有fieldを戻してはならない。
実行は（project, suite）で分けたbatchとし、libtestの`--exact` flagと複数selectorを用いる。

```text
cargo test -p <project> --lib -- --exact <selector1> <selector2> ...
（IntegrationTest の場合は --lib の代わりに --test <name>）
```

実行対象の解釈とcommand生成は `TestRunnerAdapter` が所有する。orchestrationは
`ExecutionDescriptor.adapter`をregistryで解決し、adapter不一致（E-ADAPTER-003）を
拒否する。明示的なrunでrunner未提供ならE-ADAPTER-004としてEvidenceを生成せず、
検証集約の実行関連項目は `NOT_EXECUTED` とする。

`--exact` は後続の全フィルタへ適用されるフラグであり、各フィルタは完全一致で解釈される。

### 9.3 `rust-cargo` 結果のパース

stdout を次の規則でパースする（stable toolchain の標準出力形式のみに依存する）。

```text
running N tests            → 実行対象数の確認
test <selector> ... ok       → PASS
test <selector> ... FAILED   → FAIL
test <selector> ... ignored  → NOT_EXECUTED（Evidence は記録しない）
```

要求した各フィルタについて結果行が得られなかった場合、その Test の実行は失敗（E-EXEC-002）とし、Evidence を記録しない。
プロセス終了コードと結果行の集計が矛盾する場合も E-EXEC-003 とする。
stdout / stderr の全文は `cache/logs/<ULID>.log` へ保存し、Evidence の `log_ref` から参照する。

### 9.4 Evidence の記録

Test ごとに §3.7 のレコードを1件生成する。

- `revision`：実行直前に `git rev-parse HEAD` と `git status --porcelain` で取得。取得失敗時は `commit: null` とし、この Evidence は `evidence_validity` で PASS にならない。
- `hashes`：実行直前のdiscovery結果から、Test construct hashと、全宣言targetの正規化Target Reference・implementation construct hash（§1.3）を宣言順で記録する。欠落・重複を許可しない。
- ビルド失敗（コンパイルエラー）の場合、対象 Test 群の Evidence は記録せず E-EXEC-001 を報告する。`test_execution` は `NOT_EXECUTED` のままとなる。

---

## 10. `rust-cargo` Target Execution Verification

### 10.1 計測方式

`rust-cargo` CoverageAdapterは`cargo-llvm-cov`を使用する（adapter configの`run.coverage: llvm-cov`）。
起動時に `cargo llvm-cov --version` で利用可否を確認し、利用不能なら計測せず、`target_execution` を `NOT_CHECKED` とし診断 W-EXEC-101 を出す（PASS へ変換しない。基本仕様 §7.9）。

カバレッジを Test 単位で対象関数へ帰属させるため、計測時は Test を1件ずつ実行する。

```text
cargo llvm-cov test -p <project> --lib --json
  --output-path cache/cov/<ULID>.json
  -- --exact <selector>
```

coverageは独立した `CoverageAdapter` capabilityとして扱う。提供されない場合は
`target_execution = NOT_CHECKED`、解析限界は `UNKNOWN` とし、測定済みPASSを推測しない。

### 10.2 判定

出力 JSON（llvm-cov export 形式）の `data[].functions[]` から、Testが宣言する各対象関数を検索する。

```text
一致条件：
  demangle 済み関数名の末尾が locator の item-path と一致し、
  かつ filenames のいずれかの末尾が locator の path と一致する

ジェネリック関数は複数インスタンスが現れるため、同じtargetに対応するcountを合算する。

target別判定：
  count > 0          → PASS
  count == 0         → FAIL
  関数が見つからない → UNKNOWN（インライン化・cfg 除外等の可能性）

Test単位集約：
  FAILが1件以上                    → FAIL
  FAILなし、UNKNOWNが1件以上        → UNKNOWN
  1件以上の全宣言targetがPASS       → PASS
```

各targetの参照・result・countとTest単位集約結果をEvidenceの`target_execution`へ記録する。
target別entryの欠落、重複、余分なentry、または宣言targetとの不一致をPASSとして保存しない。

### 10.3 実行モードの整理

`vtest run` は2モードを持つ。

- `--fast`：cargo test のみ。`target_execution.checked: false` で記録し、検証時は `NOT_CHECKED`。
- 既定（完全検証向け）：cargo-llvm-cov による Test 単位実行。実行時間と引き換えに `target_execution` を判定する。

---

## 11. 鮮度検証と集約

### 11.1 チェック項目の評価地点

12のチェック項目（基本仕様 §4.2）は、次の地点で評価する。

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
| `target_execution` | TEST | 有効なEvidenceのtarget別結果を§10.2で集約した値（checked: falseはNOT_CHECKED） |
| `evidence_validity` | TEST | §11.2 の判定 |
| `test_traceability` | repository scan result | 全Discovered Testが構造上完全なManaged Test Entityへ1対1で対応し、Test IDが一意かつ全`covers`参照を解決できればPASS |

`test_traceability`の判定は次のとおりとする。

- `ManagedTestLink::Missing`なら`MISSING`。これは管理宣言の欠落、必須metadataの欠落、または空の`covers`を含む。
- `ManagedTestLink::Multiple`、Test ID衝突、または`ManagedTestLink::One`が指すentityの`covers`参照を解決できない場合は`MISMATCH`。
- 全Discovered Testが`ManagedTestLink::One`を持ち、各linkがちょうど1件の構造上完全なentityを指し、Test IDが大局的に一意で、全`covers`参照を解決できる場合だけ`PASS`。
- discovery結果が不完全または解析不能なら`UNKNOWN`とし、PASSにしない。
- repository-level項目であるため、REQ / VO / TESTのentity scopeを指定してもDiscovered Test集合を狭めない。必要な場合は`--items`でこの項目自体をscope外にできるが、その値は`NOT_CHECKED`のまま保持する。

### 11.2 Evidence 鮮度判定

```text
対象 Test の Evidence のうち最新のものについて：

1. evidence.hashes.test_construct == 現在のTest construct hash
2. evidence.hashes.targetsの参照集合が現在のTest.targetsと重複なく一致し、各target_constructが現在のimplementation construct hashと一致
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
  2. test_traceabilityが項目scopeにあればrepository全体のDiscovered Test集合を評価
  3. scope のエンティティ軸で REQ/VO/TEST 部分木を選択
  4. 各 TEST について、scope のチェック項目軸に含まれる
     TEST 評価項目を評価（含まれない項目は NOT_CHECKED）
  5. 各 leaf VO について VO 評価項目を評価し、
     covers する TEST 群の結果を fail-closed で合成
  6. 中間 VO は子 VO の合成（fail-closed）
  7. REQ は spec_coverage と VO 部分木の合成
  8. 総合判定：repository-level項目とentity treeのscope内評価がすべてPASS → OK、それ以外 → NG

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

### 16.2 意味的衝突検出

`vtest doctor`は、同じTest IDの重複、covers先VOの欠落、承認済VOの内容不一致など、
version controlの構文的整合性だけでは判定できない論理的不整合を次の規則で検出する。

- ID 衝突 → E-SCAN-002
- dangling reference → E-SCAN-003 / E-SCAN-009
- 承認の失効 → §3.5 のハッシュ束縛により自動的に draft へ
- 監査・Evidence の失効 → §8.5 / §11.2 のハッシュ束縛により自動的に STALE へ

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
| E-ADAPTER-001 | error | adapterが未登録、重複、またはregistryの宣言と実装が不一致 |
| E-ADAPTER-002 | error | adapterのdiscoveryまたはrunnerが確定的に失敗（Evidenceなし） |
| E-ADAPTER-003 | error | Testのexecution descriptorと選択adapterが不一致 |
| E-ADAPTER-004 | error | 明示操作に必須のadapter capabilityが未提供（変更・Audit・Evidenceなし） |
| W-ADAPTER-101 | warning | 検証対象のadapter capabilityが未提供（能力に応じNOT_CHECKEDまたはNOT_EXECUTED） |
| W-ADAPTER-102 | warning | adapterが解析限界を報告（該当項目はUNKNOWN） |

### 17.2 終了コード

| コード | 意味 |
|---|---|
| 0 | 要求 scope の検証結果が OK（操作コマンドでは成功） |
| 1 | 検証結果が NG |
| 2 | 入力・使用方法エラー（診断コード E-OP-* / E-ADAPTER-*、引数不正、スキーマ違反の提出など） |
| 3 | 内部エラー（ツール自体の異常） |

---

## 19. 実装選択と提供範囲

次の事項は、基本仕様と本詳細設計の観測可能な契約を変更しない範囲で実装が選択できる。

- demangle 実装（`rustc-demangle`）の適用範囲
- `#[tokio::test]` 等、属性末尾 `test` 以外のカスタムテスト属性への対応範囲
- cargo workspace 外の単一クレートプロジェクトでのパス解決の細部
- レポートのツリー描画の細部（文字種、折返し）

次の事項は提供範囲外とする。

- LSP / rust-analyzer 連携によるシンボル解決
- 永続インデックス（`cache/` の活用）
- Relation の tombstone 方式
- `rust-cargo`以外のproduction language adapter（synthetic adapterは受入fixture専用）
- LLM API直接呼び出しによる監査
- rename 追跡と SRC 恒久 ID の自動昇格支援
- cargo-nextest 対応
