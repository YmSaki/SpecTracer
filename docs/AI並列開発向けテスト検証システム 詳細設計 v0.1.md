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

**内容ハッシュ**はSHA-256を使用し、`sha256:<hex>`形式で記録する。テキストfragmentは改行をLFへ統一し、各行の末尾空白を除去する。これ以外の空白は正規化しない。

hash inputはdomain separatorと長さ付きfieldから構成する。各fieldは`field-name`、UTF-8 byte length、byte列の順にencodeし、単純な文字列連結を行わない。mapはkey昇順、集合として扱う`covers`・`targets`・`related`は正規化値の昇順、順序に意味がある`cases`は宣言順とする。null、空文字、空listは異なる値としてencodeする。

- Test subject hash：domain `vtest:test-subject:v1`を用い、adapter ID、Test ID、全canonical metadata、Source Locationのadapter・project-relative path・opaque locator、ExecutionDescriptor、および正規化したTest construct bytesを束縛する。byte range自体は前方の無関係な編集で変化するためhash inputにしない。metadata宣言がmanifest等の非隣接箇所に存在しても、adapterが返す論理metadataを同じsubjectへ含める。
- Source Target hash：domain `vtest:target-subject:v1`を用い、正規化TargetRefとadapterが返すimplementation construct bytesを束縛する。
- Static Audit Config subject hash：domain `vtest:static-audit-config:v1`を用い、adapter ID、static audit capabilityのrule-set ID / version、および現在の静的rule判定へ影響する実効adapter configのcanonical projectionを束縛する。adapterは同じ入力に対するverdictまたは根拠を変えうるrule実装変更ごとにrule-set versionを変更しなければならない。adapterはrule影響fieldだけを型付き・順序正規化済みのhash未計算DTOとして返し、coreがencodingとSHA-256を行う。静的ruleと無関係なrun、coverage、root等の設定はprojectionへ含めない。
- VO / REQ hash：domain `vtest:record-subject:v1`を用い、readerが具体化したcanonical recordをfield規則に従ってencodeする。VOの読取り互換field `status`は正典ではないため含めない。
- SPEC subject hash：domain `vtest:spec-subject:v1`を用い、canonical SPEC recordと参照先Specification sourceの正規化内容を束縛する。SPEC recordの`sha256`と実sourceが不一致ならsubject hashは現在有効な値として成立せず、STALEとする。

adapterはsource location、source rangeと現在のbytes、解析済みlogical metadata、ExecutionDescriptorをhash未計算のdiscovery DTOとして返す。coreはadapter出力と現在のsource bytesの対応を検証し、上記の言語非依存encodingとSHA-256計算を行ってからdomain entityを具体化する。adapterが最終的な`TestEntity.content_hash`または`SourceTarget.content_hash`を返して自己確定してはならない。coreはASTや言語固有構文からrangeを再計算しない。

`rust-cargo` adapterはTest constructとしてmetadata doc commentを除き、実行に影響する属性、signature、bodyを含む関数itemのbytesを返す。doc comment由来metadataはlogical metadataと`metadata_sources`として別に返す。Source Targetには属性とdoc commentを含む関数item全体を返す。format変更を構文上の意味だけから同値とみなさず、上記正規化後のsource bytesが変化した場合は安全側でSTALEにする。

---

## 2. データディレクトリと設定

### 2.1 `.verify/` レイアウト

```text
.verify/
  config.yaml
  spec/       SPEC-<NAME>.yaml
  req/        REQ-<NAME>.yaml
  vo/         VO-<NAME>.yaml
  rel/        REL-<ULID>.yaml
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
  # 完全検証の固定チェック項目（基本仕様 §4.2 の12項目。追加・削除不可）
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

`verify.full_scope`は利用者が完全検証を縮小する設定ではなく、基本仕様 §4.2の固定12項目を列挙するconfig invariantである。version 2では重複・未知項目・欠落・余剰をE-CONFIG-001で拒否する。version 1ではfield欠落を固定12項目として具体化し、重複または未知項目はE-CONFIG-001で拒否する。認識可能な列挙は欠けている現在の項目をin-memoryで補って固定12項目へ正規化し、特に11項目形は`test_traceability`を補う。どちらも読み取りだけでconfigを書き換えない。`--items`による明示的な部分集合だけを限定scopeとして扱い、項目指定を省略したCLI / MCP検証は常に固定12項目を評価する。
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

active REQは1件以上の`spec_refs`を必須とする。`withdrawn` REQは履歴表示のため参照を保持するが、
`spec_coverage`の現在の対応REQ集合には含めない。
`spec_refs.spec`は存在するSPEC recordとcurrent sourceへ決定論的に解決する。`section`は非空のopaque citationであり、coreは任意形式のSpecification本文からsectionの存在を構文的に判定しない。citationの意味的妥当性は§8の監査理由で検証する。

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
created: 2026-08-08
updated: 2026-08-08
```

VOの`status`は承認レコードから導出する表示値であり、canonical writerはVO recordへ保存しない。
readerは読取り互換fieldとして`status`を受理するが、実効判定とVO内容hashでは無視し、
存在自体をW-STORE-001として通知する。互換field値と導出値が異なる場合も導出値だけを使用する。

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

### 3.4 Relation レコード（`.verify/rel/REL-<ULID>.yaml`）

導出できない関係のみを保存する（基本仕様 §2.3）。

```yaml
id: REL-01J8XVZK3Q...
type: depends-on          # depends-on | supersedes | regression-for |
                          # derived-from | same-partition | complements | conflicts-with
from: TEST-PARSER-044     # 任意のエンティティID
to: TEST-PARSER-012
note: ""
created: 2026-08-08T00:00:00Z
```

canonical Relation IDは`REL-`と26文字のULID payloadからなり、writerは`.verify/rel/REL-<ULID>.yaml`と同値の`id`だけを生成する。readerはversion 1互換入力として`.verify/rel/<ULID>.yaml`かつ同値のbare `id`を受理し、`REL-<ULID>`へin-memoryで正規化するが、読み取りだけでファイルを書き換えない。prefixed / bareの混在、ファイル名と`id`のpayload不一致、または同じpayloadの複数recordはE-SCAN-010とし、いずれかを選ばない。
Relation は不変。変更はファイル削除＋新規作成で表す。
`from` / `to` の存在はスキャン時に検査する。

### 3.5 承認レコード（`.verify/approvals/<ULID>.yaml`）

```yaml
id: 01J8XW0A9M...
subject: VO-PARSER-UTF8-003     # 承認対象のエンティティID
subject_hash: "sha256:..."      # 承認時点の対象の内容ハッシュ
dependencies:                   # 承認時点の上流依存closure（完全一致を要求）
  - kind: vo
    id: VO-PARSER-UTF8
    hash: "sha256:..."
  - kind: req
    id: REQ-PARSER-001
    hash: "sha256:..."
  - kind: spec
    id: SPEC-BASIC-001
    hash: "sha256:..."          # §1.3のSPEC subject hash
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
   かつ dependencies が現在の上流依存closureとentity・hashとも完全一致する
   という条件を満たす承認レコードが1件以上存在する」
それ以外は draft（承認失効を含む）
```

上流依存closureは、対象VOの再帰的なparent VO、対象VOとparent VOが参照するREQ、
それらREQの再帰的なparent REQ、および各VO / REQの`spec_refs`が参照するSPECからなる。
対象VO自身は`subject_hash`で束縛するため`dependencies`へ重複して含めない。
entryは`kind`、`id`の順でsortし、欠落・重複・余剰entryを許可しない。

SPEC dependencyは§1.3のSPEC subject hashを使用するため、SPEC recordまたは参照先sourceの変更で
承認が失効する。依存entryを持たない互換Approvalは読取りと履歴表示だけを許可し、
現在の`approved`を導出しない。W-STORE-002を出し、VOは`draft`相当とする。

VOの実効`status`は常にこの式から導出し、canonical VO recordへ保存しない。

### 3.6 監査レコード（`.verify/audits/<ULID>.yaml`）

```yaml
id: 01J8XVZZ...
kind: test-semantic             # spec-coverage | test-semantic | vo-coverage |
                                # impl-consistency | static
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
`kind: static`のsubjectsはTest subject、重複のない全宣言target subject、および選択adapterのStatic Audit Config subjectをそれぞれちょうど1回含む。config subjectを欠く読取り互換recordは履歴表示できるが、現在の`static_audit`へ有効なPASSを供給せず`STALE`とする。

```yaml
# kind: staticのconfig subject
- config:
    adapter: rust-cargo
    capability: static-audit
  hash: "sha256:..."            # §1.3のStatic Audit Config subject hash
```

### 3.7 Evidence レコード（`.verify/evidence/<ULID>.yaml`）

```yaml
id: 01J8XW1B...
test_id: TEST-PARSER-044
adapter: rust-cargo
result: PASS                    # PASS | FAIL
executed_at: 2026-08-08T00:00:00Z
revision: { commit: "abc123...", dirty: false }
hashes:
  test_subject: "sha256:..."
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

writerは`hashes.test_subject`を必須とし、Test construct単体のhashを現在のEvidence freshness keyとして出力しない。

readerは`rust-cargo` Evidenceに限り、互換fieldの`hashes.test_fn`または`hashes.test_construct`とtarget entry内の`target_fn`を読み取れる。互換Test hashを現在の`test_subject`へ正規化できるのは、現在の`rust-cargo` adapterが当該互換hashのsource rangeに全canonical metadataとTest constructが含まれること、現在bytesとの完全一致、および現在のlogical metadataとの一致を証明できる場合だけとする。証明できなければrecordは保持するが`evidence_validity`をPASSにしない。中立fieldと互換fieldが併存する場合は導出される値の同値を必須とし、非`rust-cargo` Evidenceでは互換fieldを解釈しない。
readerは単数互換形の`hashes.target_fn`および`target_execution.result/count`を、現在の`rust-cargo` Testがtargetをちょうど1件宣言し、§3.7の条件でTest subjectを証明でき、target construct hashも照合できる場合だけ1要素listへ正規化して扱う。
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
   adapter ID、Source Location、source range、current bytes、hash未計算のTest draft、
   metadata source、Target draft、診断を検証する
   capability宣言と出力が矛盾するbatchは拒否する

4. core materialization
   Test subjectとSource Targetのhashを§1.3で計算し、TestEntity、SourceTarget、
   DiscoveredTest、ManagedTestLinkを具体化する

5. 決定論的な統合
   adapter ID、project-relative path、opaque locator、Test IDの順に正規化する
   adapter間を含むTest ID・SRC ID衝突と不正な複数対応を検査する

6. .verify/ 読み込み
   vtest-storeが全レコードを読み込み、スキーマ検証する

7. 参照整合性検査
   coversのVO ID、targetsのTarget Reference / SRC ID、Relation、parentを解決する

8. グラフ構築と整合性検査（§5.3、§5.4）
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
    pub content_hash: ContentHash,  // §1.3のTest subject hash（coreが計算）
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

`TargetRef::SrcId`はadapter IDを含まないため、`SrcId`は全adapterを統合したrepositoryで
global uniqueでなければならない。collision時はE-SCAN-011とし、TargetRefを解決しない。

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

`SourceDiscoveryAdapter`は次のhash未計算DTOを返す。

```rust
pub struct SourceFragment {
    pub location: SourceLocation,
    pub bytes: Vec<u8>,
}

pub struct ManagedTestDraft {
    pub id: TestId,
    pub covers: Vec<VoId>,
    pub targets: Vec<TargetRef>,
    pub intent: String,
    pub input: Option<String>,
    pub expect: Option<String>,
    pub kind: Option<String>,
    pub cases: Vec<String>,
    pub related: Vec<TestId>,
    pub execution: ExecutionDescriptor,
}

pub struct DiscoveredTestDraft {
    pub adapter: AdapterId,
    pub location: SourceLocation,
    pub construct: SourceFragment,
    pub metadata_sources: Vec<SourceFragment>,
    pub managed: ManagedTestDraftLink,
}

pub enum ManagedTestDraftLink {
    Missing,
    One(ManagedTestDraft),
    Multiple(Vec<ManagedTestDraft>),
}

pub struct SourceTargetDraft {
    pub target: TargetRef,
    pub location: SourceLocation,
    pub construct: SourceFragment,
}

pub struct DiscoveryBatch {
    pub adapter: AdapterId,
    pub completeness: DiscoveryCompleteness,
    pub discovered_tests: Vec<DiscoveredTestDraft>,
    pub source_targets: Vec<SourceTargetDraft>,
    pub diagnostics: Vec<Diagnostic>,
}

pub enum DiscoveryCompleteness {
    Complete,
    Incomplete,
}
```

adapterは`SourceFragment.bytes`が`location.byte_range`の現在bytesと一致する状態だけを返す。
manifest等にある非隣接metadataも`metadata_sources`へ列挙するが、hash inputはadapter構文のraw表現ではなく
`ManagedTestDraft`のcanonical logical metadataである。coreはrange・bytes対応を検証し、§1.3でhashを計算してから
`TestEntity`、`SourceTarget`および次のobservationを具体化する。
`ManagedTestDraftLink::One` / `Multiple`の各draftは、全logical metadataを導出した1件以上の
`metadata_sources`を持たなければならない。provenance欠落はmalformed adapter outputとしてE-ADAPTER-002で拒否する。

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
```

`SourceDiscoveryAdapter`はadapterがTestとして認識した全Discovered Test draftを返す。`ManagedTestDraftLink::One`は、構文上有効なTest ID、1件以上の`covers`、その他の必須metadataをdraftとして具体化できる場合に設定する。
VO参照の解決とTest IDの大局的一意性はadapterではなくcoreが検査する。したがって、解決不能な`covers`を持つdraftもcore materialization後のmanaged entity集合に保持され、対応するobservationは`ManagedTestLink::One(id)`を持つ。
`ManagedTestDraftLink::Missing`は管理宣言の欠落または必須metadataの欠落、`Multiple`は同一Test constructから複数draftが生じる状態を表す。core materialization後の対応する状態が`ManagedTestLink`となる。

adapter capabilityは `SourceDiscoveryAdapter`、`TestWireCodec`、`StaticAuditAdapter`、
`StructuredTestAdapter`、`TestRunnerAdapter`、`CoverageAdapter` に分割する。
各adapterは一意なID、languages、capabilities、config namespaceを宣言し、registryは
宣言と実装の不一致および重複IDを拒否する。明示操作に必須のcapabilityがない場合は
E-ADAPTER-004で操作を中止する。検証集約では、static audit / coverage欠落は
`NOT_CHECKED`、runner欠落は `NOT_EXECUTED`、解析限界は `UNKNOWN` とする。

Structured Test capabilityを宣言するadapterは、処理可能なbuilt-in Form `kind`集合と、adapter fieldを持たないForm Schemaを判定するcompatibility matcherを宣言する。Form `kind`はbuilt-inと`.verify/forms/`を統合したrepository全体で一意であり、Form Schemaの`adapter` field、registryのowner、Structured Test capabilityが同じadapter IDを示す場合だけ`kind → adapter`を確定する。重複kindまたは対応の不一致はE-ADAPTER-001、未知kindはE-OP-001とし、coreが名前からRust adapterを推測しない。`adapter` fieldを欠く読取り互換Formは、登録済みStructured Test adapterのbuilt-in kind宣言またはcompatibility matcherのうちちょうど1件だけがschemaを受理する場合に限ってin-memoryでownerを補える。0件または複数件なら操作を拒否し、ファイルを書き換えない。matcherはsource bytes、schema field / validator集合等から決定論的に判定し、form kindの文字列だけを理由に汎用fallbackしてはならない。

VO / REQ / SPEC / Relation / Approval / AuditRecord / Evidence も §3 のスキーマに対応する struct を定義する。

### 5.3 検証グラフ

インメモリのグラフを構築する。

```text
ノード：SPEC, REQ, VO, TEST, SRC（ロケータ単位）
エッジ：
  REQ  → REQ    (parent)
  REQ  → SPEC   (spec_refs)
  VO   → VO     (parent)
  VO   → REQ    (requirements)
  VO   → SPEC   (spec_refs)
  TEST → VO     (covers)      ※adapter所有のTest metadata宣言由来
  TEST → SRC    (targets)     ※adapter所有のTest metadata宣言由来、1:N
  外部 Relation (rel/ 由来)
逆引きインデックス：VO → Tests、SRC → Tests、REQ → VOs、SPEC → REQs
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
| E-SCAN-010 | error | レコードのid / ファイル名 / schema不一致、または互換正規化後のlogical record ID重複 |
| E-SCAN-011 | error | 恒久SRC IDが複数adapterまたは複数Source Targetで衝突 |
| E-SCAN-012 | error | REQ / VO のrequirementsまたはspec_refs.specが存在しないentityを参照、またはspec_refs.sectionが空 |
| W-SCAN-101 | warning | adapterが発見したが管理宣言に対応しないTest construct（unregistered test） |
| W-SCAN-102 | warning | どの VO からも参照されず、Test も参照しない孤立 VO |
| W-SCAN-103 | warning | `covers` を持つが対応 VO が leaf でない（中間 VO 直接参照。許容するが警告） |
| W-STORE-001 | warning | VO recordに非正典の読取り互換field `status`が存在（値は無視し承認から導出） |
| W-STORE-002 | warning | Approvalが現在の上流依存closureを欠くか一致せず、承認として無効 |

error は該当エンティティに関わるチェック項目を非PASSにする。
warningは診断severityだけでは検証値を変更しないが、レポートに常に表示する。
W-SCAN-101またはE-SCAN-007が示す`ManagedTestLink::Missing`は、診断とは独立した`test_traceability`評価で`MISSING`になる。
`ManagedTestLink::Multiple`、E-SCAN-002のTest ID衝突、またはE-SCAN-003の解決不能なVO参照は`test_traceability = MISMATCH`とする。E-SCAN-003が発生しても対応するTest Entityと`ManagedTestLink::One`を除去しない。E-SCAN-011があるSRC ID参照は曖昧なため、関係するtarget解決項目を`MISMATCH`とし、いずれかのadapterを選ばない。

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

7. draft生成
   全Discovered Test draft、ManagedTestDraftLink、SourceTargetDraft、Source Location、
   construct / metadata source rangeとbytes、logical metadata、ExecutionDescriptor、診断を
   hash未計算のDiscoveryBatchに格納する
```

---

## 6. Target Reference解決

### 6.1 adapter-neutral解決contract

coreは`TargetRef::Locator.adapter`をregistryで解決し、opaque locatorの解釈を該当する`SourceDiscoveryAdapter`へ委譲する。adapterは正規化されたTarget Reference、Source Location、source range、content bytes、解決status、候補を返す。
coreは返却されたadapter IDとTarget Referenceの一致、source rangeの範囲、current bytesとの一致を検証し、§1.3のSource Target hashを計算するが、opaque locatorの内部構文は解釈しない。解決が0件または複数候補で一意に定まらない場合はE-SCAN-004とし、推測で候補を選択しない。

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

`vtest-audit`は`TestEntity.execution.adapter`をregistryで解決し、Test、全Target Reference、各source range、content hash、および選択adapterの現在configを`StaticAuditAdapter`へ渡す。adapterはrule ID、verdict、根拠span、解析限界と、同じ判定に用いたrule-set identity・rule影響configのhash未計算projectionを返す。coreはadapter ID、projection schemaと決定論的encodingを検証し、§1.3のStatic Audit Config subject hashを計算して監査対象集合へ加える。rule判定へ影響するfieldをprojectionから欠落させない責任はadapter contractと§18の受入試験で強制する。coreは入力hashと返却されたsubjectの一致を検証して上記規則で集約するが、adapter固有のASTやassertion構文を解釈しない。

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
ルールごとの判定結果と根拠（該当スパン）は監査レコード（kind: `static`、auditor.kind: `deterministic`）として保存する。subjectsにはTest、全宣言target、および§1.3のStatic Audit Config subjectの現在hashを含める。`assertion_macros`またはrule-set ID / versionの変更はconfig subject hashを変化させ、既存recordを`STALE`にする。

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
| `spec-coverage` | `--spec SPEC-X` | 対象SPEC record、Specification source全文とSPEC subject hash、対象SPECを参照するactive REQの完全な集合・全record・内容hash、各REQのsection参照、withdrawn REQ一覧、有効な過去監査の要約 |
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

`spec-coverage`と`vo-coverage`の提出では `verdict` に `COMPLETE` / `INCOMPLETE` / `UNKNOWN` を用い、内部で `PASS` / `FAIL` / `UNKNOWN` へ写像する。
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

`spec-coverage`では各reasonにSpecification上の要求事項と対応REQを示すbasisを含める。
`exclusions`には取り込み対象外とした節または記述とSpecification上の根拠を列挙する。
対応REQが0件の場合はbundleを生成できるが、提出の有無にかかわらず`spec_coverage = MISSING`を維持する。

`basis.kind` は `spec` / `vo` / `req` / `test-code` / `target-code` のいずれかとする。

`impl-consistency`の提出は`PASS` / `FAIL` / `UNKNOWN`を用いる。Audit Recordには提出verdictを保持し、検証項目へは`PASS → PASS`、`FAIL → MISMATCH`、`UNKNOWN → UNKNOWN`と写像する。target解決不能の`MISSING`、監査未実施の`NOT_CHECKED`、無効recordだけが存在する`STALE`をこの写像で上書きしない。

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
7. spec-coverage の場合、各reasonがspecとreqの
   basisを持ち、exclusionにspec根拠がある          （E-AUDIT-007）
```

受理された提出は監査レコード（§3.6）として保存される。
`subjects`にはバンドル生成時の全対象を記録する。`spec-coverage`ではSPEC subjectと対象SPECを参照するactive REQの完全な集合、`test-semantic`ではTest subject・VO・全target、`vo-coverage`ではREQ・VO部分木・SPEC subject、`impl-consistency`ではTest / VO・全targetを含める。

### 8.5 有効性と多重監査

監査レコードの有効性は判定時に評価する。

```text
有効 = subjects の対象集合が現在の監査対象集合と完全一致し、
       全ハッシュが現在のsubject hashと一致する
（SPEC は登録された sha256 と実ファイルの一致も要求。
 不一致の場合は W-SCAN-104 を出し、当該レコードは STALE）
```

`kind: static`の現在対象集合はTest subject、全宣言target subject、Static Audit Config subjectからなる。静的rule判定へ影響するconfig projectionまたはrule-setが変化したrecord、あるいはconfig subjectを欠くrecordは`STALE`であり、有効なPASSに数えない。

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
- `hashes`：実行直前のdiscovery結果から、Test subject hashと、全宣言targetの正規化Target Reference・implementation construct hash（§1.3）を宣言順で記録する。欠落・重複を許可しない。
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
| `spec_coverage` | SPEC | sourceがcurrentでactive REQが1件以上存在し、現在のSPEC subjectと対応REQ完全集合に束縛された有効なspec-coverage監査がPASSならPASS |
| `vo_decomposition` | REQ / VO | REQ / VO部分木のparent・requirements・spec_refs・構造Relationだけを§11.1.1で評価 |
| `vo_coverage` | REQ / VO | 有効なvo-coverage監査（§8.5）がPASSかつ対象VOが承認済（§3.5）ならPASS |
| `test_existence` | leaf VO | covers する Test が1件以上あれば PASS、なければ MISSING |
| `static_audit` | TEST | §7.1 の合成 |
| `semantic_audit` | TEST | 有効な test-semantic 監査の合成（§8.5） |
| `impl_consistency` | TEST / VO | 有効なimpl-consistency監査を§8.3でCheckValueへ写像して合成。監査FAILはMISMATCH、対象シンボル不在はMISSING |
| `test_execution` | TEST | 有効なEvidenceが存在すればPASS、Evidenceあり・無効なら§11.2の非PASS、EvidenceなしはNOT_EXECUTED |
| `runtime_result` | TEST | 有効なEvidenceのresult（PASS / FAIL）。無効または不在は§11.2 |
| `target_execution` | TEST | 有効なEvidenceのtarget別結果を§10.2で集約した値。無効または不在は§11.2、checked: falseはNOT_CHECKED |
| `evidence_validity` | TEST | §11.2 の判定 |
| `test_traceability` | repository scan result | 全Discovered Testが構造上完全なManaged Test Entityへ1対1で対応し、Test IDが一意かつ全`covers`参照を解決できればPASS |

`spec_coverage`の判定は次のとおりとする。

- 完全検証または指定scopeに登録SPECが0件なら`MISSING`とし、空集合をPASSにしない。
- SPEC recordまたはsourceが存在しなければ`MISSING`、SPEC recordの登録hashとsourceが不一致なら`STALE`。
- 対象SPECを参照するactive REQが0件なら`MISSING`。REQの`spec_refs.spec`が解決不能、`section`が空、またはschema不正なら`MISMATCH`。opaqueなsection citationの本文内存在をcoreが構文的に判定した結果だけでPASSにしない。
- currentな対象SPECと対応active REQ完全集合に対する有効な`spec-coverage`監査が`COMPLETE`なら`PASS`、`INCOMPLETE`なら`FAIL`、判定不能なら`UNKNOWN`。
- 監査が一度もなければ`NOT_CHECKED`、現在の対象集合に無効な監査だけがあれば`STALE`。
- REQ / VO / TEST scopeは対象SPECの対応active REQ集合を狭めない。特定範囲だけを評価する場合はSPEC scopeを指定し、Specification内部の一部節だけを完全検証済みとして扱わない。
- 複数の非PASS条件が同時に成立する場合はすべてを根拠として保持し、表示代表値は基本仕様 §4.3の優先順位で選ぶ。

`vo_coverage`はactive REQごとに対応VO集合を評価する。対応VOが0件なら`MISSING`、
有効な`vo-coverage`監査がなければ`NOT_CHECKED`または`STALE`、監査が`INCOMPLETE`なら`FAIL`、
判定不能なら`UNKNOWN`とする。監査が`COMPLETE`で、対象VO部分木の承認が§3.5によりすべて有効な場合だけ`PASS`とする。

#### 11.1.1 `vo_decomposition`の診断写像

`vo_decomposition`は独立した構造dimensionであり、§5.4の全errorを一括して取り込まない。
選択したREQ / VO部分木について、次だけを評価する。

- REQ / VOのparent不在は`MISSING`、cycleは`MISMATCH`。
- VOの`requirements`またはREQ / VOの`spec_refs.spec`が解決不能なら`MISSING`、`section`が空、参照型またはschemaが矛盾する場合は`MISMATCH`。opaqueなsection citationの意味的妥当性はこの構造dimensionで判定しない。
- 選択部分木のREQ / VOをendpointに持つ構造Relationのdangling endpointは`MISSING`、矛盾したschemaは`MISMATCH`。
- 対象recordの読込みが解析不能または不完全なら`UNKNOWN`。
- 上記がなく、parent、requirements、spec_refsのSPEC entity参照、構造Relationがすべて解決できる場合だけ`PASS`。

E-SCAN-008、E-SCAN-012、および選択部分木のREQ / VO / 構造RelationをsubjectとするE-SCAN-009 / E-SCAN-010だけがこの項目へ影響する。Test ID、Test metadata、target解決、adapter source parse、Evidenceの各errorは対応するTest・実行・Evidence dimensionへ写像し、`vo_decomposition`を変更しない。

`test_traceability`の判定は次のとおりとする。

- `ManagedTestLink::Missing`なら`MISSING`。これは管理宣言の欠落、必須metadataの欠落、または空の`covers`を含む。
- `ManagedTestLink::Multiple`、Test ID衝突、または`ManagedTestLink::One`が指すentityの`covers`参照を解決できない場合は`MISMATCH`。
- 全Discovered Testが`ManagedTestLink::One`を持ち、各linkがちょうど1件の構造上完全なentityを指し、Test IDが大局的に一意で、全`covers`参照を解決できる場合だけ`PASS`。
- discovery結果が不完全または解析不能なら`UNKNOWN`とし、PASSにしない。
- repository-level項目であるため、REQ / VO / TESTのentity scopeを指定してもDiscovered Test集合を狭めない。必要な場合は`--items`でこの項目自体をscope外にできるが、その値は`NOT_CHECKED`のまま保持する。

### 11.2 Evidence 鮮度判定

```text
対象 Test の Evidence のうち最新のものについて：

1. evidence.hashes.test_subject == 現在のTest subject hash
2. evidence.hashes.targetsの参照集合が現在のTest.targetsと重複なく一致し、各target_constructが現在のimplementation construct hashと一致
3. evidence.revision.commit が非 null
4. evidence.adapter が現在のTest.execution.adapterと一致する。adapter欠落形は§3.7の互換条件で一意に確認できる

1〜4 すべて成立 → evidence_validity = PASS
  （dirty: true でもハッシュ一致なら有効。ハッシュが実体を保証する）
1 または 2 不成立 → STALE
3 不成立          → STALE（現在revisionへの有効性を確認できない実行）
4が明示的不一致  → MISMATCH
4を確認不能      → UNKNOWN
Evidence なし     → NOT_EXECUTED
```

複数条件が非PASSなら根拠をすべて保持し、表示代表値は基本仕様 §4.3の優先順位で選ぶ。`evidence_validity`がPASSの場合だけ`test_execution = PASS`とし、`runtime_result`と`target_execution`を当該Evidenceから評価する。Evidenceが存在するが有効でない場合、この3項目はEvidenceを再利用せず、`evidence_validity`と同じ`MISMATCH` / `STALE` / `UNKNOWN`を保持する。Evidenceがなければ3項目とも`NOT_EXECUTED`とする。有効なEvidenceで`target_execution.checked: false`の場合だけ`target_execution = NOT_CHECKED`とする。

### 11.3 集約アルゴリズム

項目scopeが省略された場合、aggregatorはconfig値から部分集合を組み立てず、基本仕様 §4.2の固定12項目を選択する。`verify.full_scope`はconfig読込み時に§2.2のinvariantとして検証・正規化済みでなければならない。明示的な部分集合だけを限定scopeとし、その結果を完全検証として表示しない。

```text
fn aggregate(scope) -> Report:
  1. scan によりグラフ構築（§5）
  2. test_traceabilityが項目scopeにあればrepository全体のDiscovered Test集合を評価
  3. scope のエンティティ軸で SPEC/REQ/VO/TEST 部分木を選択
  4. spec_coverageが項目scopeにあれば対象SPECごとに対応active REQ完全集合を評価
  5. 各 TEST について、scope のチェック項目軸に含まれる
     TEST 評価項目を評価（含まれない項目は NOT_CHECKED）
  6. 各 leaf VO について VO 評価項目を評価し、
     covers する TEST 群の結果を fail-closed で合成
  7. 中間 VO は子 VO の合成（fail-closed）
  8. REQ は vo_decomposition / vo_coverage と VO 部分木の合成
  9. SPEC は spec_coverage と REQ 部分木の合成
 10. 総合判定：repository-level項目とentity treeのscope内評価がすべてPASS → OK、それ以外 → NG

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
当該SPEC subjectをdependencyに含むApprovalも無効となる。
仕様文書の更新は `vtest spec add --update` による再登録で反映し、依存する監査・承認が失効することを利用者へ提示する。再登録でSPEC subject hashが変化するため、以前のdependency entryを現在の承認へ流用しない。

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
| W-SCAN-104 | warning | SPEC レコードの sha256 と実ファイルの不一致（依存監査はSTALE、依存Approvalは無効） |
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
| E-AUDIT-007 | error | spec-coverage でspec / req basisまたはexclusion根拠を欠く |
| E-APPROVAL-001 | error | Approval対象または上流依存closureを完全・currentに解決できず、recordを生成しない |
| E-CONFIG-001 | error | config version、`verify.full_scope`、config field型または登録adapterが検証する設定値が現在のconfig invariantに違反（未知・重複adapter IDはE-ADAPTER-001） |
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
| 2 | 操作拒否（E-OP-* / E-ADAPTER-* / E-APPROVAL-* / E-CONFIG-*、引数不正、adapter前提・capability・実行失敗、スキーマ違反の提出など。検証結果は生成しない） |
| 3 | 内部エラー（ツール自体の異常） |

終了コードは診断severityだけでなく操作段階で決める。`vtest scan` / `vtest doctor`では、
registry・config・adapter契約の検証またはadapter呼出しがE-ADAPTER-* / E-CONFIG-*で拒否された場合は2、
scanが完了してrepository整合性のE-SCAN-*を報告した場合は1、errorがなければ0とする。
同一実行に複数候補がある場合は内部エラー3、操作拒否2、検証NG1、成功0の順で優先する。

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
