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

**内容ハッシュ**はSHA-256を使用し、`sha256:<hex>`形式で記録する。subject固有規則でbyte-exactを要求しないテキストfragmentは改行をLFへ統一し、各行の末尾空白を除去する。これ以外の空白は正規化しない。

hash inputはdomain separatorと長さ付きfieldから構成する。各fieldは`field-name`、UTF-8 byte length、byte列の順にencodeし、単純な文字列連結を行わない。mapはkey昇順、集合として扱う`covers`・`targets`・`related`は正規化値の昇順、順序に意味がある`cases`は宣言順とする。null、空文字、空listは異なる値としてencodeする。

- Test subject hash：domain `vtest:test-subject:v1`を用い、adapter ID、Test ID、全canonical metadata、Source Locationのadapter・project-relative path・opaque locator、ExecutionDescriptor、および正規化したTest construct bytesを束縛する。byte range自体は前方の無関係な編集で変化するためhash inputにしない。metadata宣言がmanifest等の非隣接箇所に存在しても、adapterが返す論理metadataを同じsubjectへ含める。canonical metadataの`targets`は**宣言された**`TargetRef`の正規化値を束縛し、解決後のcanonical Locatorへ置換しない。これによりTestの参照方法の変更（同一Source Targetへのlocator参照からSRC ID参照への書き換え等）はTest subject hashで捕捉される（§6.1.1）。canonical metadataの`role` / `anchor` / `anchor_rationale`も同じ前例に従い、**宣言された逐語値**（§5.2の`role_declared` / `anchor_declared` / `anchor_rationale`）を束縛し、core materializationが確定した実効値を束縛しない。宣言の不在と既定値の明示宣言は異なる値としてencodeするため、実効値が同一でもTest subject hashは一致しない。
- Source Target hash：domain `vtest:target-subject:v1`を用い、**canonical Target Reference**とadapterが返すimplementation construct bytesを束縛する。canonical Target Referenceは**常に`TargetRef::Locator`**（adapter IDとadapter所有のopaque locator）であり、`TargetRef::SrcId`をcanonical Target Referenceにしない。`TargetRef::SrcId`はSource Targetを参照する側の表現であって、Source Target自身の識別ではない。恒久SRC IDはhash inputの独立fieldとして束縛せず、canonical Target Reference経由でもhash inputへ入らない。恒久SRC IDの宣言・変更・削除はcanonical Target Referenceを変えない。ただし恒久SRC IDの宣言をSource Targetのconstruct bytesの内側へ置くadapter（`rust-cargo`の`@vtest.src-id` doc comment等。§5.5）では、その宣言の追加・変更・削除がconstruct bytesを変化させ、construct bytes経由でSource Target hashが変化しうる。これは正しい挙動であり、恒久SRC IDが独立したhash fieldであることを意味しない。hashはSource Target自身のcanonical Locatorから一度だけ計算し、当該Source Targetを参照するTest側の`TargetRef`綴りからは計算しない。Evidence、Audit、検証は解決後のcanonical Source Targetのcanonical Locatorとhashへ束縛し、addressing modeごとに別subjectを作らない（§6.1）。
- Static Audit Config subject hash：domain `vtest:static-audit-config:v1`を用い、adapter ID、static audit capabilityのrule-set ID / version、および現在の静的rule判定へ影響する実効adapter configのcanonical projectionを束縛する。adapterは同じ入力に対するverdictまたは根拠を変えうるrule実装変更ごとにrule-set versionを変更しなければならない。adapterはrule影響fieldだけを型付き・順序正規化済みのhash未計算DTOとして返し、coreがencodingとSHA-256を行う。静的ruleと無関係なrun、coverage、root等の設定はprojectionへ含めない。
- Static Analysis Source subject hash：domain `vtest:static-analysis-source:v1`を用い、adapter ID、project-relative path、opaque locator、およびadapterが静的rule判定時に実際に参照したbyte-exact source fragmentを束縛する。byte range自体はhash inputにしない。Test subjectまたはSource Target subjectが同じ解析入力を完全に束縛する場合は重複subjectを作らない。
- Execution State subject hash：domain `vtest:execution-state:v1`を用い、adapter ID、snapshot schema ID / version、HEAD revision、runner kindとcanonical invocation projection、toolchain identity、実行結果へ影響するadapter configのcanonical projection、および実行可能状態を変えうるrepository / local dependency入力の完全なmanifestを束縛する。manifest entryはstable root identity、root-relative path、input kind、byte-exact file bytesからなり、entry集合は正規化identity順にencodeする。stable root identityはmachine上の絶対pathを用いず、workspace内の論理rootまたはdependency identityから決定論的に導出する。adapterはhash未計算のmanifestと完全性を返し、coreが各entryとsubject全体を検証・hash化する。
- VO / REQ hash：domain `vtest:record-subject:v1`を用い、readerが具体化したcanonical recordをfield規則に従ってencodeする。VOの読取り互換field `status`は正典ではないため含めない。
- SPEC subject hash：domain `vtest:spec-subject:v1`を用い、canonical SPEC recordと参照先Specification sourceの正規化内容を束縛する。SPEC recordの`sha256`と実sourceが不一致ならsubject hashは現在有効な値として成立せず、STALEとする。

adapterはsource location、source rangeと現在のbytes、解析済みlogical metadata、ExecutionDescriptorをhash未計算のdiscovery DTOとして返す。coreはadapter出力と現在のsource bytesの対応を検証し、上記の言語非依存encodingとSHA-256計算を行ってからdomain entityを具体化する。adapterが最終的な`TestEntity.content_hash`または`SourceTarget.content_hash`を返して自己確定してはならない。coreはASTや言語固有構文からrangeを再計算しない。

Static Audit adapterとTest Runner adapterも、判定または実行状態へ用いたsource / config / manifestをhash未計算DTOとして返す。coreは現在bytesとの対応、重複、集合完全性、schema versionを検証してsubject hashを計算する。adapterが完全性を保証できないDTOから`PASS`用subjectを具体化してはならない。

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
  - analysis_source:
      adapter: rust-cargo
      path: tests/parser_test.rs
      locator: helper_for_invalid_utf8
    hash: "sha256:..."
  - id: SPEC-BASIC-001
    hash: "sha256:..."
verdict: PASS                   # PASS | FAIL | UNKNOWN
reasons:                        # semantic は §8.3 の claim/basis 構造。static は §7.2 の規則別判定（後述の static reasons schema）
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

`subjects`の`target` entryは§6.1で解決したcanonical Source Targetの**canonical Locator**とする。参照側Testが宣言した`TargetRef`の綴り（SRC ID参照を含む）を監査対象のidentityとして記録しない（§6.1.1）。解決できないtargetを持つTestでは、任意の候補を選んで`target` subjectを生成しない。

同一対象への監査レコードは複数存在してよい（再監査・多重監査）。
判定に用いるのは「subjects の全ハッシュが現在と一致する（＝有効な）レコードのうち最新のもの」とする。
有効なレコードに FAIL と PASS が混在する場合は FAIL を採る（fail-closed）。
`kind: static`のsubjectsはTest subject、重複のない全宣言target subject、選択adapterのStatic Audit Config subject、およびTest / target subjectだけでは束縛されない全Static Analysis Source subjectをそれぞれちょうど1回含む。analysis source集合はadapterが実際に判定へ使用したsource fragmentの完全集合と一致しなければならない。config subjectまたは必要なanalysis source subjectを欠く読取り互換recordは履歴表示できるが、現在の`static_audit`へ有効なPASSを供給せず`STALE`とする。

```yaml
# kind: staticのconfig subject
- config:
    adapter: rust-cargo
    capability: static-audit
  hash: "sha256:..."            # §1.3のStatic Audit Config subject hash
```

`kind: static`の`reasons`は規則ごとの判定を保持する。target-scopedな**DA-002 / DA-003**は`targets`に**target別verdict**を持つ。これは`target_execution.targets`（§10.2）と対称な正典であり、規則単位の`verdict`はこのtarget別verdictを§7.2のfoldで導出した派生値である。

```yaml
# kind: staticのreasons（規則別。DA-002 / DA-003はtarget別verdictを持つ）
reasons:
  - rule: DA-002                # 対象未呼出（target-scoped）
    verdict: UNKNOWN            # §7.2のfoldで導出する派生値
    targets:                    # 全宣言targetと過不足なく1対1。canonical Locatorで識別
      - target: "rust-cargo::src/parser.rs::Parser::parse"
        verdict: PASS           # target別: PASS | UNKNOWN | FAIL
        basis:
          - kind: test-code
            ref: "rust-cargo::tests/parser_test.rs::rejects_invalid_utf8:12"
      - target: "rust-cargo::src/parser.rs::Parser::finish"
        verdict: UNKNOWN        # 静的に到達を証明できない（境界越し等。§7.3）
        basis:
          - kind: test-code
            ref: "rust-cargo::tests/parser_test.rs::rejects_invalid_utf8:20"
  - rule: DA-001                # target-scopedでない規則はtargetsを持たず規則単位verdictのみ
    verdict: PASS
    basis:
      - kind: test-code
        ref: "rust-cargo::tests/parser_test.rs::rejects_invalid_utf8:18"
```

target別verdictの`target`は§6.1で解決したcanonical Locatorとし、その集合はTestが宣言する全targetと過不足なく1対1に対応する。target entryの欠落・重複・余剰、宣言target集合との不一致、またはtarget別verdictと規則単位verdictの純静的fold結果の矛盾はmalformed recordとする。malformed recordは§3.7のEvidence（E-SCAN-010）と対称に**E-SCAN-010**とし、有効な結果に使用せず有効record集合から除外する（malformed recordの内容は信頼せず、その中のper-target FAILも抽出しない）。malformedでない有効なstatic Audit Recordが1件も残らなければ、`static_audit`はSTALE（無効化recordのみ存在）またはNOT_CHECKED（一度も監査なし）とし、PASSにしない。target-scopedでない規則（DA-001 / DA-004 / DA-005 / DA-006）は`targets`を持たない。

### 3.7 Evidence レコード（`.verify/evidence/<ULID>.yaml`）

```yaml
id: 01J8XW1B...
test_id: TEST-PARSER-044
adapter: rust-cargo
result: PASS                    # PASS | FAIL
executed_at: 2026-08-08T00:00:00Z
revision: { commit: "abc123...", dirty: false }
execution_state:
  schema: rust-cargo-execution-state-v1
  complete: true
  hash: "sha256:..."             # complete: falseの場合だけnull
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

`hashes.targets`はTestの宣言順で常に記録し、各`target`は§6.1で解決したcanonical Source Targetの
**canonical Locator**の正規化文字列表現とする。参照側Testが宣言した`TargetRef`の綴り（SRC ID参照を含む）を
Evidence上のtarget identityとして記録しない（§6.1.1）。
このlistはTestの宣言target集合を解決したcanonical Source Target集合と重複なく1対1に対応する。
Evidence生成のprecondition（§9.4）により全宣言targetは一意に解決済みであるため、この集合は宣言target集合と同数になる。
`target_execution.checked: true`では`target_execution.targets`も同じ順序・同じcanonical Locator集合で
1対1に対応する。
`target_execution.checked: false`では`method`と`result`をnull、`targets`を空listとし、検証値を
`NOT_CHECKED`とする。

writerは`hashes.test_subject`を必須とし、Test construct単体のhashを現在のEvidence freshness keyとして出力しない。

readerは`rust-cargo` Evidenceに限り、互換fieldの`hashes.test_fn`または`hashes.test_construct`とtarget entry内の`target_fn`を読み取れる。互換Test hashを現在の`test_subject`へ正規化できるのは、現在の`rust-cargo` adapterが当該互換hashのsource rangeに全canonical metadataとTest constructが含まれること、現在bytesとの完全一致、および現在のlogical metadataとの一致を証明できる場合だけとする。証明できなければrecordは保持するが`evidence_validity`をPASSにしない。中立fieldと互換fieldが併存する場合は導出される値の同値を必須とし、非`rust-cargo` Evidenceでは互換fieldを解釈しない。
readerは単数互換形の`hashes.target_fn`および`target_execution.result/count`を、現在の`rust-cargo` Testがtargetをちょうど1件宣言し、§3.7の条件でTest subjectを証明でき、target construct hashも照合できる場合だけ1要素listへ正規化して扱う。
複数target Testに単数互換形を適用せず、writerは常にlist形を出力する。

Evidence内の`target`は実行時snapshotを識別するkeyであり、TEST → SRC edgeの正典ではない。
graphはadapter所有のTest metadata宣言からだけ構築し、Evidenceのtarget listからedgeを生成しない。

`execution_state`は§1.3のExecution State subjectである。writerは実行直前にadapterからsnapshot DTOを取得し、core検証後のschema ID、完全性、subject hashを記録する。`complete: true`は、選択Testのビルドと実行可能状態を変えうるrepository / local dependency入力、runner、toolchain、実行影響configをadapterが漏れなく列挙した場合だけ許可する。snapshot生成不能または不完全の場合も実行事実の履歴を記録できるが、`complete: false`、`hash: null`として現在の`evidence_validity = PASS`へ使用しない。

`rust-cargo-execution-state-v1`のmanifestは、選択Testを含むCargo workspace / package root、全local path dependency root、各root内の通常file、Cargo manifest / lockfile、`.cargo` config、build script、Rust source / test / fixture / compile-time resource、toolchain指定を含む。`.git/`、`.verify/`のcanonical record / cache、Cargo target directory等の生成物は実行入力から除外する。除外領域をbuild script、macro、`include_*`、path dependencyその他の経路で読み込む可能性を排除できない場合、snapshotを完全と報告しない。repository内helperだけの変更もmanifest hashを変化させる。

Evidence readerは`execution_state`を欠く互換recordを履歴表示できるが、現在のEvidence freshnessを証明できないため`STALE`とする。

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
id, covers[], targets[], intent, role?, anchor?, anchor_rationale?,
input?, expect?, kind?, cases[], related[]
```

`role`、`anchor`、`anchor_rationale`はadapter非依存の論理fieldであり、adapterはsource declarationの値を逐語のまま搬送する。adapterは受理語彙との照合、既定値の補完、基本仕様 §6.2の制約評価のいずれも行わない（§4.4）。受理語彙にない値と予約値`characterization`も、除去・置換・正規化せずそのままcoreへ渡す。これらの値がcoreへ届くことがE-SCAN-013の成立要件であり、adapterが不正値を落とすと当該診断を生成できない。搬送先のfieldは§5.2の`ManagedTestDraft.role_declared` / `anchor_declared` / `anchor_rationale`である。

coreはsource declarationの構文と配置を解釈せず、adapterが返したTest Entity、Discovered Test observation、Source Location、Target Reference、source range、診断を検証・統合する。
locatorは`TargetRef::Locator { adapter, value }`とし、`value`はadapter所有のopaque文字列である。coreがpath、module、symbol種別を分解しない。

### 4.2 `rust-cargo` annotation文法

テスト関数直前の doc comment（`///` または `/** */`）内の行を対象とする。

```text
annotation-line = "@vtest." key SP value
key             = "id" | "covers" | "target" | "intent" | "input"
                | "expect" | "kind" | "case" | "related" | "src-id"
                | "role" | "anchor" | "anchor-rationale"
value           = 行末までのテキスト（前後空白は除去）
role-value      = "verification" | "supporting" | "regression"
                | "characterization"
anchor-value    = "normative" | "none"
```

- 1行1キー。`covers` と `related` の値はカンマ区切りで複数指定できる。
- `case` と `related` はキー自体を複数行書ける。他のキーの重複はエラー E-SCAN-005。ただし `kind` が integration 系の Test に限り、`target` の複数行を許容する（別紙A §14.3）。許容された複数`target`内でも同じTargetRefの重複はE-SCAN-005とする。綴りが異なっても解決後に同一canonical Source Targetへ到達する複数宣言（同じSource Targetへのlocator参照とSRC ID参照の併記等）も、coreが解決時にE-SCAN-005とする（§6.1.1）。
- `@vtest.` で始まるが未知のキーを持つ行はエラー E-SCAN-006（打鍵ミスの検出を優先し、警告ではなくエラーとする）。本節の文法とこの規則は、Test constructとして解析される宣言に適用する。
- Test constructとして解析されない関数item（対象実装側の関数等）のdoc comment内でも、`@vtest.` で始まる行は自由記述として無視せず検査する。当該表面で認識されるキーは `src-id` のみであり、それ以外のキーは警告 W-SCAN-105 とする（§5.4）。打鍵ミス検出の目的は表面を問わず及ぶが、認識されないキーはTest metadataを破損させず採用値の曖昧さも生まないため、errorではなくwarningとする。
- `src-id` はこの表面でも反復不可であり、同一関数itemでの重複は採用すべきIDを決定できないためエラー E-SCAN-005 とする。このときいずれの宣言値も採用せず、当該Source TargetのSRC IDは無しとして扱う（どちらかを推測で選ばない）。
- doc comment 内の `@vtest.` を含まない行は自由記述として無視する。
- `@vtest.role`、`@vtest.anchor`、`@vtest.anchor-rationale` はいずれも反復不可であり、重複はE-SCAN-005とする。`@vtest.role` は `role_declared`、`@vtest.anchor` は `anchor_declared`、`@vtest.anchor-rationale` は `anchor_rationale` へ、値を逐語のまま対応付ける（§4.1）。
- `role-value` と `anchor-value` は受理語彙の定義であって、adapterが行う受理判定ではない。adapterはこの語彙に一致しない値も逐語で搬送し、値がいずれの語彙にも属さない場合、および `anchor-rationale` の値が空白のみの場合は、coreがcore materializationでエラー E-SCAN-013 を生成する（§4.4）。`characterization` は `role-value` に含まれる予約されたrole値であり、本versionでは宣言できない。宣言された場合も未知キーのE-SCAN-006ではなくE-SCAN-013で報告する。キーの綴り自体が未知である場合だけがadapter側のE-SCAN-006であり、既知キーの値が語彙に違反する場合はcore側のE-SCAN-013である。
- これら3キーはTest宣言の論理metadataであり、Test constructに対応する宣言として解析された場合にだけ当該Test Entityのfieldへ採り込む。coreはこれらの値をSource Targetのidentity、恒久SRC ID、target解決のいずれにも用いない。
- `@vtest.src-id` はテストではなく対象実装側の関数に付与し、任意の恒久SRC IDを宣言する。scannerは指定値を認識するが、付与を必須としない（基本仕様 §3.3）。`rust-cargo`のSource Target constructは属性とdoc commentを含む関数item全体であり（§1.3）、この宣言行はconstruct bytesの内側にある。したがって`@vtest.src-id`の付与・変更・削除はSource Target hashを変化させる。この表面での打鍵ミス（`src_id` 等の未知キー）は W-SCAN-105、`src-id` の重複は E-SCAN-005 で検出し、無音で無視しない（§4.2・§5.4）。

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

構文上完全なTest Entityの判定は、§5.1 step 4のcore materializationで実効`role`を確定してから行う。coreはdraftの`role_declared`を次のとおり実効`role`へ写す。

- 宣言なし（`role_declared`が`None`）→ `Some(Verification)`
- 受理語彙（§4.2の`role-value`）のうち本versionで宣言できる値 → 当該値の`Some`
- 受理語彙にない値、または予約値`characterization` → `None`とし、E-SCAN-013を生成する

`anchor_declared`も同じ規則で実効`anchor`へ写し、受理語彙（`anchor-value`）にない値は`None`とE-SCAN-013にする。実効`anchor`が`Some`になるのは宣言値が受理語彙に属する場合であり、その宣言を実効`role`が許すかどうかは`covers` / `anchor`制約として別に評価する。

**不正な値または予約値を宣言したTestを`verification`として確定してはならない。** source declarationはTest Entityの正典であり（基本仕様 §6.1）、sourceが宣言していない目的をcoreが補うことは、宣言の意味を捏造して不正宣言を有効な検証Testへ昇格させる。実効`role`が`None`のentityは除去せずに保持し、`test_traceability = MISMATCH`とする。当該entityに対して評価する項目は§11.1.2の適用項目集合に従う。

実効`role`が`Some`の場合だけ、確定した`role`に対して基本仕様 §6.2の`covers` / `anchor`制約を評価する。adapterは`role`を解釈せず、この評価地点より前で`covers` 0を不完全と判定しない。

E-SCAN-007の`covers`欠落は`role`が`verification`の場合にだけ成立する。`id` / `targets` / `intent`の欠落は`role`によらずE-SCAN-007とする。`role`が`supporting`または`regression`のTestの`covers`件数はE-SCAN-007では評価せず、次の区分に従う。

- **E-SCAN-014**：確定した`role`がその宣言自体を許さない場合に成立する。`supporting`で`covers`が1件以上、または`regression`以外の`role`で`anchor`もしくは`anchor_rationale`を宣言した場合が該当する。
- **E-SCAN-015**：`role`が`regression`のentity内部で`anchor`分類が整合しない場合に成立する。`anchor`宣言の欠落、`anchor normative`で`covers` 0、`anchor none`で`covers`が1件以上、`anchor none`で`anchor_rationale`欠落、`anchor normative`で`anchor_rationale`宣言が該当する。

両者の成立条件は排他であり、1件の違反へ2つのcodeを与えない。

E-SCAN-013 / E-SCAN-014 / E-SCAN-015が成立するTestは、entityと`ManagedTestLink::One(id)`を保持し、構造上完全でないentityとして`test_traceability = MISMATCH`とする。E-SCAN-003と同じく、違反を理由にentityを除去しない。`ManagedTestLink::Missing`となるのは管理宣言の欠落とE-SCAN-007の場合に限る。同一TestでE-SCAN-007とE-SCAN-013 / E-SCAN-014 / E-SCAN-015が同時に成立する場合、link状態はE-SCAN-007の`Missing`を優先し、他の診断は併記するだけでlink状態と`test_traceability`の値を変えない。

実効`role`が`None`の場合は`covers` / `anchor`制約を評価せず、E-SCAN-013とMISMATCHだけを生成する。実効`anchor`が`None`の場合も`anchor`分類を確定できないものとし、当該TestのE-SCAN-015を評価しない。

`rust-cargo` annotationの構文違反（重複不可キーの重複、未知キー、必須キーの欠落）は§5.4のE-SCAN-005、E-SCAN-006、E-SCAN-007で報告する。宣言値の語彙違反であるE-SCAN-013、および`role`確定後にだけ判定できるE-SCAN-014 / E-SCAN-015は、adapter構文の違反ではなくcore materializationの診断として同じcode体系で報告する。

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
    pub role_declared: Option<String>,      // 宣言の逐語。内容hashはこの値を束縛する（§1.3）
    pub anchor_declared: Option<String>,    // 同上
    pub anchor_rationale: Option<String>,   // 同上
    pub role: Option<TestRole>,             // 実効値。宣言なしはSome(Verification)、
                                            // 受理語彙外・予約値はNone（§4.4）
    pub anchor: Option<TestAnchor>,         // 実効値。anchor_declaredが受理語彙の値ならSome。
                                            // その宣言をroleが許すかは§6.2の制約で別に判定する
    pub input: Option<String>,
    pub expect: Option<String>,
    pub kind: Option<String>,
    pub cases: Vec<String>,
    pub related: Vec<TestId>,
    pub location: SourceLocation,
    pub content_hash: ContentHash,  // §1.3のTest subject hash（coreが計算）
    pub execution: ExecutionDescriptor,
}

pub enum TestRole {
    Verification,
    Supporting,
    Regression,
    // characterizationは予約値であり、本versionではvariantを持たない（§4.2）
}

pub enum TestAnchor {
    Normative,
    None,                           // 規範義務を保護しない宣言（anchor_rationale必須）
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
    pub role_declared: Option<String>,      // 宣言の逐語。受理語彙外・予約値も搬送する（§4.1）
    pub anchor_declared: Option<String>,    // 同上
    pub anchor_rationale: Option<String>,   // 同上
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
    pub src_id: Option<SrcId>,
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

Source Targetはcanonical locator（`TargetRef::Locator`）と任意の恒久SRC IDを併有する**単一のdomain entity**である。
`TargetRef::Locator`と`TargetRef::SrcId`はいずれも同一Source Targetへのaddressing modeであり、別個のentityを指さない。
恒久SRC IDはlocatorの代替ではなく、同じSource Targetへ与えられるoptional permanent identityである。

- adapterは`@vtest.src-id`等で宣言された恒久SRC IDを`SourceTargetDraft.src_id`として返す。
  同一constructをlocator版とSrcId版の2件のdraftへ複製してはならない。
- `SourceTargetDraft.target`は**必ず`TargetRef::Locator`**でなければならない（§1.3 canonical Target Reference）。
  `TargetRef::SrcId`はSource Targetへの参照表現であり、`SourceTargetDraft`のcanonical targetとして返してはならない。
  adapterが`target`に`TargetRef::SrcId`を返した場合は malformed adapter output として拒否する。
  恒久SRC IDは`src_id`だけで搬送し、`target`の綴りを変えない。
- coreは`src_id`を統合済みSRC索引へ登録し、locator参照とSRC ID参照のどちらから解決しても
  同一のcanonical Source Targetへ到達させる。
- Source Target hashは常に canonical Locator と construct bytes から計算し、恒久SRC IDを独立したhash fieldとして
  含めない。canonical Locatorは恒久SRC IDの増減で変化しないため、**参照方法**の違いによってSource Targetの件数、
  content / subject hash、EvidenceおよびAudit上のtarget identityが分裂しない。
  一方、恒久SRC IDの宣言をconstruct bytesの内側へ置くadapterでは、その宣言の追加・変更・削除が
  construct bytesを変え、Source Target hashを変化させうる（§1.3）。これはsourceが実際に変化したことの帰結であり、
  参照方法による分裂ではない。
- coreは統合済みSRC索引から、その恒久SRC IDを宣言した`SourceTargetDraft.target`（= canonical Locator）へ解決する。
- 恒久SRC IDを持つSource Targetも引き続きcanonical locatorでaddressableでなければならない。

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

`SourceDiscoveryAdapter`はadapterがTestとして認識した全Discovered Test draftを返す。`ManagedTestDraftLink::One`は、構文上有効なTest IDと、`covers`件数以外の必須metadataをdraftとして具体化できる場合に設定する。`covers`の要求件数は`role`に依存するため、adapterはこれを判定せず、coreが§4.4のcore materializationで評価して`ManagedTestLink`を確定する。`role_declared` / `anchor_declared`の値が受理語彙に違反していてもdraft化を止めず、逐語値を載せた`One`として返す。adapterが不正宣言のdraftを`Missing`へ落とすと、coreがE-SCAN-013と実効`role`の不確定を区別できない。
VO参照の解決とTest IDの大局的一意性はadapterではなくcoreが検査する。したがって、解決不能な`covers`を持つdraftもcore materialization後のmanaged entity集合に保持され、対応するobservationは`ManagedTestLink::One(id)`を持つ。
`ManagedTestDraftLink::Missing`は管理宣言の欠落または必須metadataの欠落、`Multiple`は同一Test constructから複数draftが生じる状態を表す。core materialization後の対応する状態が`ManagedTestLink`となる。

adapter capabilityは `SourceDiscoveryAdapter`、`TestWireCodec`、`StaticAuditAdapter`、
`StructuredTestAdapter`、`TestRunnerAdapter`、`CoverageAdapter` に分割する。
各adapterは一意なID、languages、capabilities、config namespaceを宣言し、registryは
宣言と実装の不一致および重複IDを拒否する。明示操作に必須のcapabilityがない場合は
E-ADAPTER-004で操作を中止する。検証集約では、static audit / coverage欠落は
`NOT_CHECKED`、runner欠落は `NOT_EXECUTED`、解析限界は `UNKNOWN` とする。

`StaticAuditAdapter`と`TestRunnerAdapter`は、coreがfreshness subjectを所有できるよう次のhash未計算DTOを返す。`CanonicalProjection`は型tag、null、list順序、map key順序を保持する言語非依存値とする。

```rust
pub struct StaticAnalysisClosureDraft {
    pub complete: bool,
    pub sources: Vec<SourceFragment>,
}

pub struct StaticAuditConfigDraft {
    pub rule_set_id: String,
    pub rule_set_version: String,
    pub effective_config: CanonicalProjection,
}

pub struct ExecutionInputDraft {
    pub root_identity: String,
    pub root_relative_path: String,
    pub kind: String,
    pub bytes: Vec<u8>,
}

pub struct ExecutionStateDraft {
    pub schema_id: String,
    pub schema_version: String,
    pub complete: bool,
    pub head_revision: Option<String>,
    pub runner_kind: String,
    pub invocation: CanonicalProjection,
    pub toolchain_identity: String,
    pub effective_config: CanonicalProjection,
    pub inputs: Vec<ExecutionInputDraft>,
}
```

Static Auditの実行結果は`StaticAuditConfigDraft`と`StaticAnalysisClosureDraft`を必ず伴う。`complete: false`ではrule結果に違反がなくても`UNKNOWN`とする。有効性再評価時、coreは保存recordのsource listを正典として再利用せず、現在のTest・target・configからadapterへclosure再導出だけを要求し、返された現在集合とrecord集合の完全一致および各hash一致を検証する。

Test Runnerはcommand起動前に`ExecutionStateDraft`を構築し、実際に使用するinvocation / toolchain / configと一致するDTOだけを実行結果へ添付する。`invocation`はselector、working root、runner option等をmachine非依存に正規化し、絶対pathを含む表示用commandとは分離する。coreは実行前後でExecution State subject全体が変化していないことを確認してからEvidenceを記録する。変化した場合はE-EXEC-004としてEvidenceを生成しない。有効性再評価では同じschemaを持つ現在DTOを再構築し、保存hashと比較する。

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
| E-SCAN-005 | error | adapter所有の宣言で重複不可fieldが重複、または綴りの異なる複数の`target`宣言が同一canonical Source Targetへ解決 |
| E-SCAN-006 | error | Test constructのadapter所有の宣言に未知fieldが存在（非Test construct表面はW-SCAN-105） |
| E-SCAN-007 | error | 必須metadata（id / targets / intent、および`role`が`verification`の場合の covers）の欠落 |
| E-SCAN-008 | error | VO / REQ の parent 不在または循環 |
| E-SCAN-009 | error | Relation の from / to が不在 |
| E-SCAN-010 | error | レコードのid / ファイル名 / schema不一致、または互換正規化後のlogical record ID重複 |
| E-SCAN-011 | error | 恒久SRC IDが複数adapterまたは複数Source Targetで衝突 |
| E-SCAN-012 | error | REQ / VO のrequirementsまたはspec_refs.specが存在しないentityを参照、またはspec_refs.sectionが空 |
| E-SCAN-013 | error | `role` / `anchor` / `anchor_rationale` の値が受理語彙または値制約に違反（予約値`characterization`の宣言を含む） |
| E-SCAN-014 | error | 確定した`role`が許さない宣言の併存（`supporting`と1件以上の`covers`、`regression`以外の`role`と`anchor` / `anchor_rationale`） |
| E-SCAN-015 | error | `regression`の`anchor`分類が`covers` / `anchor_rationale`と不整合、または`anchor`宣言の欠落 |
| W-SCAN-101 | warning | adapterが発見したが管理宣言に対応しないTest construct（unregistered test） |
| W-SCAN-102 | warning | どの VO からも参照されず、Test も参照しない孤立 VO |
| W-SCAN-103 | warning | `covers` を持つが対応 VO が leaf でない（中間 VO 直接参照。許容するが警告） |
| W-SCAN-105 | warning | Test constructとして解析されない関数itemのdoc comment内の`@vtest.`行に認識されないキーが存在（§4.2。打鍵ミス検出。`src-id`の重複はE-SCAN-005） |
| W-STORE-001 | warning | VO recordに非正典の読取り互換field `status`が存在（値は無視し承認から導出） |
| W-STORE-002 | warning | Approvalが現在の上流依存closureを欠くか一致せず、承認として無効 |

error は該当エンティティに関わるチェック項目を非PASSにする。
warningは診断severityだけでは検証値を変更しないが、レポートに常に表示する。
W-SCAN-101またはE-SCAN-007が示す`ManagedTestLink::Missing`は、診断とは独立した`test_traceability`評価で`MISSING`になる。
`ManagedTestLink::Multiple`、E-SCAN-002のTest ID衝突、E-SCAN-003の解決不能なVO参照、またはE-SCAN-013 / E-SCAN-014 / E-SCAN-015の`role` / `anchor`制約違反は`test_traceability = MISMATCH`とする。E-SCAN-003が発生しても対応するTest Entityと`ManagedTestLink::One`を除去しない。E-SCAN-011があるSRC ID参照は曖昧なため、関係するtarget解決項目を`MISMATCH`とし、いずれのSource Targetも選択しない。いずれのadapterも選ばず、候補の1件を解決結果としてEvidence、監査subject、`target_execution`へ永続化しない（§6.1）。衝突する恒久SRC IDを宣言した各Source Target自体は、canonical locatorで独立に具体化されたまま保持する。

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
   §4.3のlocator解決・逆引き・@vtest.src-id認識に使用する。
   このpassで非Test constructのdoc comment内の`@vtest.`行を検査し、
   認識されないキーからW-SCAN-105を、`src-id`の重複からE-SCAN-005を生成する（§4.2）

7. draft生成
   全Discovered Test draft、ManagedTestDraftLink、SourceTargetDraft、Source Location、
   construct / metadata source rangeとbytes、logical metadata、宣言された恒久SRC ID、
   ExecutionDescriptor、診断をhash未計算のDiscoveryBatchに格納する
```

---

## 6. Target Reference解決

### 6.1 adapter-neutral解決contract

coreは`TargetRef::Locator.adapter`をregistryで解決し、opaque locatorの解釈を該当する`SourceDiscoveryAdapter`へ委譲する。adapterは正規化されたTarget Reference、Source Location、source range、content bytes、解決status、候補を返す。
coreは返却されたadapter IDとTarget Referenceの一致、source rangeの範囲、current bytesとの一致を検証し、§1.3のSource Target hashを計算するが、opaque locatorの内部構文は解釈しない。解決が0件または複数候補で一意に定まらない場合はE-SCAN-004とし、推測で候補を選択しない。

SRC ID参照はcoreが統合済みSRC索引で一意性を検査し、対応するadapterのSource Locationとsource rangeを使用する。SRC ID参照は当該恒久SRC IDを宣言したSource Targetのcanonical locatorへ解決し、同じSource Targetへのlocator参照と**同一のcanonical Source Target・同一のSource Target hash**へ到達する。解決結果をlocator版とSrcId版の別entityへ分岐させない。恒久SRC IDが複数adapterまたは複数Source Targetで衝突する場合はE-SCAN-011とし、いずれのSource Targetも選択しない。

解決結果は「解決済み」「対象なし」「曖昧」の3状態を区別し、曖昧はfail-closedな終端状態とする。曖昧な解決から代表候補を選ばず、解決済みのcanonical Source Targetを要求する後段（静的監査subject、Evidence、`target_execution`、鮮度判定）へ候補を1件も引き渡さない。候補は§6.3の診断表示にだけ用い、表示できることを選択の根拠にしない。
この禁止はTarget Referenceの**解決**に関するものであり、Source Targetの**具体化**を止めるものではない。各Source Targetは自身のcanonical locatorで独立に具体化され、恒久SRC IDが衝突していても`SourceTargetDraft`ごとに1件のSource Targetとして成立する。衝突が壊すのは当該恒久SRC IDによる**参照**の一意性だけである。
この解決はcoreの単一経路が所有し、静的監査、実行、Evidence writer、検証集約はいずれもその結果を消費する。各subsystemが独自にcandidate列を走査して1件を選ぶ経路を持ってはならない。E-SCAN-004またはE-SCAN-011で解決できなかったtargetを、後段が任意の候補で埋めて記録・永続化することを禁ずる。

#### 6.1.1 target identityの一方向確定

Source Target identityは次の一方向でだけ確定する。

```text
TestEntity.targets  = 宣言されたTargetRef（Locator / SrcId）
        ↓ resolve（§6.1）
Canonical Source Target = canonical Locator
        ↓
Evidence / Audit / target_execution / 検証 = canonical Locatorをidentityとして使用
```

Evidence（§3.7、§9.4）、監査レコード（§3.6）、`target_execution`（§10.2）、および鮮度判定（§11.2）は、解決後のcanonical Locatorをtarget identityとして記録・比較する。参照側Testが宣言した`TargetRef`の綴り（SRC ID参照を含む）をこれらのidentityとして保存してはならない。
Testがどう宣言したか（同じSource Targetに対するLocator参照からSRC ID参照への書き換え等）の変更は、`targets`をcanonical metadataとして束縛する§1.3のTest subject hashが捕捉する。したがってEvidence / Audit側で宣言表現を保持する必要はなく、保持すれば同一Source Targetが参照方法ごとに別identityへ分裂する。
Testの宣言target集合は解決後のcanonical Source Target単位で一意でなければならない。綴りの異なる複数の宣言が同一のcanonical Source Targetへ解決する場合は重複targetとしてE-SCAN-005とする。

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
解析の限界で確定できない場合は FAIL ではなく UNKNOWN とし、意味監査へ委ねる（ただし DA-002 の target 到達 UNKNOWN は §7.3 の runtime 到達証明で解決し、意味監査 bundle へは委ねない）。
Test の `static_audit` チェック項目は、全ルールが違反なしなら PASS、1つでも FAIL があれば FAIL、FAIL がなく UNKNOWN があれば UNKNOWN とする。
DA-002（target 到達）は静的な到達証明であり、その target 別 verdict（§3.6）の UNKNOWN は「静的解析の到達判定境界の外にあり、静的には到達を証明できない」ことだけを表し、到達しないことを意味しない。§7.3 に従い当該 target の runtime target_execution が到達を証明した場合、その target の到達要件は充足済みとなり、DA-002 の当該 target 別 verdict は**検証時の static_audit 項目算出（§7.3）で UNKNOWN 扱いにならない**。record が保存する純静的 fold（§7.2）は Evidence を参照せず、当該 target 別 verdict の UNKNOWN をそのまま反映する（record 表示・malformed 整合検査用）。DA-003 を含む他ルールの UNKNOWN、runtime 証明の無い target 別 DA-002 UNKNOWN、および target 別 DA-002 FAIL は検証時算出でも従来どおり非 PASS を生む。この規則は「UNKNOWN の項目値を PASS へ昇格させない」不変条件を保つ（充足済み到達は算出時点で UNKNOWN を生じない）。

`vtest-audit`は`TestEntity.execution.adapter`をregistryで解決し、Test、全Target Reference、各source range、content hash、および選択adapterの現在configを`StaticAuditAdapter`へ渡す。adapterはrule ID、verdict、根拠span、解析限界と、同じ判定に用いたrule-set identity・rule影響configのhash未計算projection、および判定時に実際に参照した全source fragmentのhash未計算DTOを返す。target-scopedなDA-002 / DA-003については、宣言targetごとのverdictと根拠spanを（規則単位のverdictへ畳み込む前の形で）返し、その集合を全宣言targetと過不足なく1対1に対応させる。coreはadapter ID、projection schema、source location・現在bytesとの対応、重複、決定論的encodingを検証し、§1.3のStatic Audit Config subject hashとStatic Analysis Source subject hashを監査対象集合へ加える。Test / target subjectが同じfragmentを完全に束縛する場合だけanalysis source subjectとの重複を除く。rule判定へ影響するfieldまたは参照sourceを欠落させない責任はadapter contractと§18の受入試験で強制する。coreは入力hashと返却されたsubjectの一致を検証して上記規則で集約するが、adapter固有のASTやassertion構文を解釈しない。

Static Audit capabilityがない場合は`NOT_CHECKED`、adapterが不完全、解析限界、または解析入力集合の不完全性を報告した場合は`UNKNOWN`とし、違反なしと推測しない。adapterがsource fragmentを参照したにもかかわらず対応DTOを返さないことをcoreが検出した場合も、そのrule verdictをPASSへ使用しない。

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
| DA-006 検証構文なし | 関数内に assert 相当が1つも存在しない | 探索視界（関数本体および同一ファイル内の呼出先 helper 1段）に assert 相当が1つも存在せず、かつ視界外への呼出も存在しない | 視界内に assert 相当が無いが視界外への呼出が存在する場合（helper からのさらに先＝2段目以降の呼出、他ファイル・他クレート呼出、クロージャ内・マクロ展開内）— assert 相当の存在を排除できない |
| W-DA-101 ignored | `#[ignore]` 属性 | （FAILにしない。警告のみ。実行されなければ `test_execution` が NOT_EXECUTED になる） | |

DA-002 / DA-003 のデータフロー解析は関数内のローカル束縛の追跡（let 束縛、メソッドチェーン、フィールドアクセス）までとし、クロージャ内・マクロ展開内は UNKNOWN とする。
DA-006 の探索境界も同一とする（関数本体および同一ファイル内 helper 1段）。assert 相当を同一ファイルの helper へ委譲する Test は「helper が検証を行うこと」の確認と同値であり、視界内の helper に assert 相当が見つかれば違反なしとする。FAIL は「視界内に assert 相当が無く、かつ視界外への呼出も無い」ことを確認できた場合に限る。1段目の helper がさらに先へ呼出す場合（2段目以降）、その先は視界外であり「1段」の探索保証を超えて FAIL を主張しない — §7.1 に従い FAIL でなく UNKNOWN として意味監査へ送る。この判定変更は rule-set version を更新し、既存の static Audit Record は STALE となり再監査で現在の判定へ更新される（§3.6）。
複数target TestではDA-002 / DA-003を各targetへ個別適用する。target別結果に1件でもFAILがあればrule結果をFAIL、FAILがなく1件でもUNKNOWNがあればUNKNOWN、全targetが違反なしの場合だけPASSとする。
このtarget別verdictは監査レコードに正典として保存し（§3.6）、規則単位のverdictは上記foldで導出する派生値とする。この規則単位verdictは**Evidenceを参照しない純静的fold**であり、§3.6のmalformed整合検査（target別verdictと規則単位verdictの一致）はこの純静的foldに対して行う。§7.3のtarget別到達判定はこの保存済みtarget別DA-002 verdictを参照する。target別verdictの記録は監査根拠の構造を変える変更であり、これを採用するrule実装変更はrule-set versionを更新する。versionはStatic Audit Config subjectに含まれるため、target別verdictを持たない既存recordはSTALEとなり、再監査で現在の形へ更新される。

**呼出そのものが Test 本体に現れない場合の DA-003。** 宣言 target への呼出が Test 本体に静的に現れない場合（subprocess を起動して別プロセスで target を実行する等、target 呼出が source 内に存在しない）、**DA-003 の当該target別verdictを UNKNOWN** とする。呼出結果を観測できないことを「違反なし（空虚PASS）」とも「結果未到達（空虚FAIL）」とも判定しない。この場合 DA-002 も同 target で UNKNOWN であり、DA-002 が §7.3 の runtime 証明で救済されても DA-003 は UNKNOWN のまま static_audit へ寄与するため、**呼出が本体に現れない Test（典型的な subprocess E2E）は static_audit = PASS に到達しない**。
一方、target 呼出は Test 本体に現れるが DA-002 が UNKNOWN になる場合（他ファイル・他クレートへの直接呼出で間接呼出の可能性を排除できない等）、その呼出結果が Test 本体内で assert 相当へ到達すれば DA-003 = PASS になりうる。この target は DA-002 を runtime で救済すれば static_audit = PASS に到達しうる（runtime 救済で実益が出る型）。クロージャ・マクロ展開の内側での到達は §7.2 の一般則どおり DA-002 / DA-003 とも UNKNOWN とする。
ルールごとの判定結果と根拠（該当スパン）は監査レコード（kind: `static`、auditor.kind: `deterministic`）として保存する。subjectsにはTest、全宣言target、§1.3のStatic Audit Config subject、および判定時に参照したhelper等のStatic Analysis Source subject完全集合の現在hashを含める。DA-002 / DA-003 / DA-006で同一file helperを探索した場合、そのhelper fragmentはTest / target subjectと重複しない限り必須subjectである。`assertion_macros`、rule-set ID / version、参照helperの内容または参照集合の変更は既存recordを`STALE`にする。

DA-001〜DA-006 で FAIL した Test は、意味監査バンドルの生成対象から除外できる（`vtest audit bundle` は既定でスキップし、`--include-failed` で強制生成できる）。

### 7.3 target 到達の静的証明と runtime 証明の関係

DA-002 は §7.2 の解析境界（関数本体および同一ファイル内 helper 1段。クロージャ内・マクロ展開内・他ファイル・他クレートへの呼出は §7.1 / §7.2 に従い UNKNOWN）で行う**静的な target 到達証明**である。Test が target を静的解析の追えない**実行境界**を越えて到達させる形態はいずれも DA-002 の UNKNOWN として現れる。これには次が含まれ、Test の kind（unit / integration）とは独立に、**execution topology** によって決まる。

- 他ファイル・他クレートへの呼出を介した間接到達
- クロージャ・マクロ展開内での到達
- 生成した別スレッド（in-process, thread boundary）での到達
- 別プロセス（subprocess を起動し、そのプロセス内で target を実行する process boundary）での到達

**到達要件は、target ごとに、次のいずれかで充足される。**

1. **静的証明**: 当該 target の**実効 target 別 DA-002 verdict = PASS**。実効 verdict は §3.6・§8.5 の有効性・多重監査規則を target ごとに適用して定める。すなわち有効な static Audit Record のうち、当該 target の DA-002 verdict に FAIL が1件でもあれば FAIL、なければ最新の有効 record の当該 target verdict を採る。
2. **runtime 証明**: §11.2 が選択した最新 Evidence の `evidence_validity` が PASS のとき、その Evidence の §10.2 target 別 target_execution result = PASS（`checked: true` かつ実行 count > 0）。

実効 target 別 DA-002 verdict が UNKNOWN（静的に証明できない）である target は、runtime 証明が成立するときに限り到達要件を満たす。複数 target Test では target ごとに適用し、Test の static_audit 到達は**全宣言 target の到達要件が充足された場合にのみ**成立する。

いずれの判定も既存の選択規則を再利用し、独自の探索をしない。

- static 側は §3.6・§8.5 の実効監査選択を target ごとに用いる。したがって有効 record のいずれかで当該 target が FAIL なら record selection の段階から FAIL が支配し、runtime で覆らない（「最新の有効 record 1件」を独自に選んで FAIL を回避しない）。
- runtime 側は §11.2 が選択した最新 Evidence だけを用いる。最新 Evidence が無効（`evidence_validity ≠ PASS`）なら runtime 証明は成立せず、§11.2 と独立に古い有効 Evidence へフォールバックしない。これにより同一検証内で `target_execution` が §11.2 で STALE の一方 static_audit が別 Evidence で PASS になる履歴不一致を防ぐ。

DA-002 の target 別 verdict を持たない読取り互換 record（rule-set version 相違で STALE となる旧 record 等）は有効 record にならない。有効な static Audit Record が1件も無ければ、static_audit は runtime 証明の有無に関わらず STALE（無効化された record のみ存在）または NOT_CHECKED（一度も監査なし）であり、到達要件の充足は有効 record の代替にならない。再監査で per-target verdict を持つ有効 record が生成された後に §7.3 の到達判定が適用される。

**static_audit 項目値は検証時に算出し、record へ保存しない。** record が保存するのは per-target verdict と、それらの純静的 fold（§7.2）だけである。検証時は、有効な static Audit Record（§3.6・§8.5）と §11.2 選択 Evidence から、target ごとに実効到達状態を定める。

- **静的到達**: 実効 target 別 DA-002 verdict = PASS。
- **runtime 到達**: 実効 target 別 DA-002 verdict = UNKNOWN かつ runtime 証明成立。
- **未充足**: 実効 target 別 DA-002 verdict = FAIL、または UNKNOWN で runtime 証明が成立しない。

static_audit 項目は、全宣言 target の DA-002 到達が静的到達または runtime 到達で充足され、**かつ** DA-003・DA-001・DA-004・DA-005・DA-006 がいずれも PASS のときだけ PASS になる。DA-003 を含むいずれかの rule が非 PASS なら §7.1 の集約に従い非 PASS。rule verdict の record 出所は、target-scoped rule（DA-002・DA-003）は §3.6・§8.5 を per-target 適用、非 target-scoped rule（DA-001・DA-004・DA-005・DA-006）は §8.5 の record 単位実効選択（FAIL 支配、なければ最新）とする。純静的 fold（§7.2）は record 表示と malformed 整合検査に用い、この項目値の算出には用いない。

この関係は fail-closed を保つ。

- runtime 証明は当該 target の target_execution = PASS のときだけ成立する。target_execution が FAIL（count 0）・UNKNOWN（関数不見当）・NOT_CHECKED（coverage 利用不能、未計測、`--fast`）のときは到達要件を満たさず、当該 target は上記の**未充足**となり、検証時の static_audit 項目算出で static_audit を非 PASS にする。
- target 別 DA-002 verdict = FAIL（解析境界内で到達を静的に否定）は runtime 証明で覆さない。
- 実行 target を持たない Test（構造・契約のみを assert し、宣言 target をどの topology でも実行しない Test）は静的にも runtime にも到達を確立できず、到達要件は未充足のままとなる。

**DA-003 はこの関係に含めず、本節の非目標である。** runtime coverage は target の「実行」を証明するが「結果検証」を証明しない。DA-003 も target-scoped であり target 別 verdict を §3.6 に記録するが、この to-runtime join には DA-002 の target 別 verdict だけが入る。したがって coverage は DA-003 を代替せず、DA-003 は §7.2 の意味論のまま（target 呼出結果が assert 相当へ到達することの静的 data-flow 判定）を維持する。典型的な subprocess E2E（target の戻り値 → 子プロセスの stdout / exit code → 親プロセスの assert）では、この data-flow は static analyzer から追えないため DA-003 は UNKNOWN のまま残りやすい。**本節は process boundary によって DA-002 到達が恒久 UNKNOWN になる問題だけを解消するものであり、boundary test を完全に static_audit PASS 可能にするものではない。** boundary-observable output への assert は検証項目 `runtime_result` に反映されるが、これは DA-003 を formal に充足する意味ではない。

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
| `test-semantic` | `--test TEST-X` | Test（metadata宣言・Test construct source全文）、全targetのimplementation construct source全文、関連Testのidとintent、決定論的監査の結果、有効な過去監査の要約。`covers`が1件以上の場合はcovers先VOレコードと同一VOをcoversする他Testの一覧を加える（下記） |
| `vo-coverage` | `--vo VO-X` または `--req REQ-X` | 対象 VO 部分木の全レコード、対応 REQ レコード、spec_refs（SPEC の path・sha256・節参照。文書本文は含めず、監査エージェントがリポジトリ内で読む）、各 leaf VO の covers 状況 |
| `impl-consistency` | `--test TEST-X` または `--vo VO-X` | 対象VOレコード、対象VOと上流VO / REQの`spec_refs`から導出したSPEC subject完全集合とSpecification source全文、全targetのimplementation construct source全文とadapterが提供する構造情報、関連Testのintent |

`test-semantic`の監査次元は`role`ではなく`covers`の有無で定める。`role`は宣言可能性だけを制約し、判定対象を決めない（§11.1.2）。

- `covers`が1件以上：VO claim × Test Intent × Test codeの三者が同じ振る舞いを指しているかを判定する。
- `covers`が0件：Test Intent × Test codeの二者が一致しているかを判定する。`intent` / `input` / `expect`が宣言する検証を、Test constructのfixture・oracle・assertionが実際に行っているかが判定対象である。

`covers`が0件のTestのbundleは`vos`を空listとし、subjects集合からVO recordを除外する。Test subjectと全宣言targetのsubjectは維持する。`sibling_tests`は意味論上の空集合とする。sibling提示の目的は同一VOを検証する他証拠との整合検査であり、VOが無ければ整合の対象そのものが定義されない。`covers`を持たないTest同士の重複・類似の検出は検証閉包の外にあり、本監査の目的ではない。したがって比較対象が空であることを、他の`covers` 0 Testを選んで埋め合わせない。

`covers`が0件のTestに対する提出は、intent-code整合の根拠となるbasis（`test-code`、必要に応じて`target-code`）を要求し、VO claimに対する`vo` basisを要求しない（§8.4）。有効性判定（§8.5）はこの縮小したsubjects集合に対して行い、VO recordを含まないことを理由にrecordをSTALEとしない。

`impl-consistency` のバンドル生成時、宣言targetのいずれか、または対象VOから§3.5と同じ上流依存規則で導出するSPEC subjectのいずれかを解決できない場合はバンドルを生成せず、候補のいずれも選択しない（§6.1）。記録する`impl_consistency`は解決失敗の種別で分ける。対象が存在しない場合（E-SCAN-004、SPEC subject不在）は`MISSING`、恒久SRC IDの衝突により複数候補で曖昧な場合（E-SCAN-011）は`MISMATCH`とする（基本仕様 §7.5、§5.4）。複数の解決失敗が異なる種別で併存する場合の代表値は基本仕様 §4.3の優先順位に従い、個別の理由をすべて保持する。SPEC recordと参照先sourceの現在性を確認できない場合は`STALE`とし、SPECを省略したbundleを生成しない。`--test`ではTestがcoversする全VO、`--vo`では選択VO部分木を起点とし、上流SPEC集合を狭めない。

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

上記は`covers`を1件以上持つTestの例である。`covers`が0件のTestでは`vos`と`sibling_tests`をいずれも空listとして出力し、fieldごと省略しない。判定対象がTest Intent × Test codeへ縮退していることを、bundleの構造から読み取れる状態にする（§8.1）。

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
`covers`が0件のTestに対する`test-semantic`提出では、各reasonが`test-code`のbasisを1件以上持つことを要求し、`vo` basisを要求しない。判定対象がTest Intentとtest codeの一致だからであり、参照できるVO claimが存在しないことをbasis欠落の理由にしない。

`impl-consistency`の提出は`PASS` / `FAIL` / `UNKNOWN`を用いる。Audit Recordには提出verdictを保持し、検証項目へは`PASS → PASS`、`FAIL → MISMATCH`、`UNKNOWN → UNKNOWN`と写像する。target解決不能（対象なしの`MISSING`、曖昧の`MISMATCH`。§8.1）、監査未実施の`NOT_CHECKED`、無効recordだけが存在する`STALE`をこの写像で上書きしない。

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
8. test-semantic で対象Testの covers が0件の場合、
   各reasonが test-code の basis を持つ            （E-AUDIT-005）
```

受理された提出は監査レコード（§3.6）として保存される。
`subjects`にはバンドル生成時の全対象を記録する。`spec-coverage`ではSPEC subjectと対象SPECを参照するactive REQの完全な集合、`test-semantic`ではTest subject・全target、および`covers`が1件以上の場合はcovers先VO、`vo-coverage`ではREQ・VO部分木・SPEC subject、`impl-consistency`ではTest / VO・全target、および対象VOの上流依存から導出したSPEC subjectの完全な集合を含める。

### 8.5 有効性と多重監査

監査レコードの有効性は判定時に評価する。

```text
有効 = subjects の対象集合が現在の監査対象集合と完全一致し、
       全ハッシュが現在のsubject hashと一致する
（SPEC は登録された sha256 と実ファイルの一致も要求。
 不一致の場合は W-SCAN-104 を出し、当該レコードは STALE）
```

`kind: static`の現在対象集合はTest subject、全宣言target subject、Static Audit Config subject、および判定に参照したStatic Analysis Source subjectの完全な集合からなる。静的rule判定へ影響するconfig projection、rule-set、参照sourceの内容または集合が変化したrecord、あるいは必要subjectを欠くrecordは`STALE`であり、有効なPASSに数えない。解析入力の完全性を証明できなければ再監査結果も`UNKNOWN`とする。

`kind: test-semantic`の現在対象集合はTest subjectと全宣言target subject、および対象Testが`covers`を1件以上持つ場合のcovers先VOからなる。`covers`が0件のTestで生成したrecordを、VO subjectを含まないことだけを理由に`STALE`としない。`covers`の増減はcanonical metadataの変化としてTest subject hashを変えるため（§1.3）、監査次元が変わったrecordが有効なまま残ることはない。

`kind: impl-consistency`の現在対象集合はTest / VO、全target、および§8.1で導出したSPEC subjectの完全な集合からなる。SPEC record、参照先Specification source、またはSPEC集合だけが変化した場合もrecordを`STALE`とし、限定scopeの`impl_consistency = PASS`へ利用しない。SPEC subjectを欠く読取り互換recordも`STALE`とする。

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

`covers`が0件のTestではバンドルの`vos`が空listであり、判定事項はTest Intentが宣言する振る舞いをテストコードが実際に検証しているかに限られる（§8.1）。VOの入力空間に関する判定事項は成立しない。

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
- `hashes`：実行直前のdiscovery結果から、Test subject hashと、全宣言targetを§6.1で解決したcanonical Locator・implementation construct hash（§1.3）を宣言順で記録する。欠落・重複を許可しない。宣言された`TargetRef`の綴りではなく解決後のcanonical Locatorを記録する（§6.1.1）。
- **Evidence生成のprecondition**：全宣言targetがcanonical Source Targetへ一意に解決できることをEvidence生成の前提とする。1件でも「対象なし」または「曖昧」（E-SCAN-004 / E-SCAN-011）なら**Evidenceを生成しない**。部分的な`hashes.targets`を持つEvidenceを生成して後段で弾く方式は採らない。この場合`test_execution`は`NOT_EXECUTED`のままとし、target解決の診断で非PASSを示す。
- `execution_state`：実行直前にrunner adapterが返すsnapshot schema、runner / toolchain / 実行影響config、およびrepository / local dependency入力manifestをcoreが検証し、§1.3のExecution State subject hashとして記録する。完全性を保証できない場合は`complete: false`とし、後続のvalidityをPASSにしない。
- ビルド失敗（コンパイルエラー）の場合、対象 Test 群の Evidence は記録せず E-EXEC-001 を報告する。`test_execution` は `NOT_EXECUTED` のままとなる。

---

## 10. `rust-cargo` Target Execution Verification

### 10.1 計測方式

`rust-cargo` CoverageAdapterは`cargo-llvm-cov`を使用する（adapter configの`run.coverage: llvm-cov`）。
起動時に `cargo llvm-cov --version` で利用可否を確認し、利用不能なら計測せず、`target_execution` を `NOT_CHECKED` とし診断 W-EXEC-101 を出す（PASS へ変換しない。基本仕様 §7.9）。

カバレッジを Test 単位で対象関数へ帰属させるため、計測時は Test を1件ずつ実行する。

Test が起動した subprocess・spawn した thread の実行を宣言 target へ帰属させられるかは `rust-cargo` CoverageAdapter の能力に属する（§10.2・§7.3）。subprocess 内の実行を計測するには起動される実行体も instrument 対象とし、子プロセスの profile を merge する必要がある。これを提供できない構成では境界越し target を UNKNOWN、計測不能なら `target_execution = NOT_CHECKED` とし、能力の有無で計測結果を捏造しない。この能力の実装可否が §7.3 の runtime 到達証明が subprocess E2E に及ぶかを左右するが、欠如時も fail-closed を保つ（DA-002 は UNKNOWN のまま）。

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

各targetのcanonical Locator（§6.1.1）・result・countとTest単位集約結果をEvidenceの`target_execution`へ記録する。
target別entryの欠落、重複、余分なentry、または解決後のcanonical Source Target集合との不一致をPASSとして保存しない。

Test が別プロセス（起動した subprocess 内）・別スレッド等の実行境界越しに target を到達させる場合も、判定は上記の実行 count に基づく。coverage provider は当該境界越しの実行を宣言 target へ帰属させなければならない（例：起動される実行体も計測対象としてinstrumentし、子プロセスの profile を merge する）。provider が境界越しの実行を帰属できない場合はその target を `UNKNOWN`（関数不見当扱い）とし、計測自体が不能なら Test の `target_execution = NOT_CHECKED` とする。いずれも §7.3 の runtime 到達証明を成立させず、静的到達の UNKNOWN を PASS へ変換しない。この帰属可否は adapter の coverage capability に属し、能力の有無で計測結果を捏造しない。

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
| `static_audit` | TEST | §7.1 の合成（DA-002 到達は §7.3 に従い静的または当該 target の runtime target_execution で充足。DA-003 は静的判定のまま） |
| `semantic_audit` | TEST | 有効な test-semantic 監査の合成（§8.5） |
| `impl_consistency` | TEST / VO | 有効なimpl-consistency監査を§8.3でCheckValueへ写像して合成。監査FAILはMISMATCH、対象シンボル不在はMISSING、曖昧な解決（E-SCAN-011）はMISMATCH。entity単位の適用可否は§11.1.2の適用項目集合による |
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

- `ManagedTestLink::Missing`なら`MISSING`。これは管理宣言の欠落、必須metadataの欠落、および`role`が`verification`のTestの空の`covers`を含む。
- `ManagedTestLink::Multiple`、Test ID衝突、`ManagedTestLink::One`が指すentityの`covers`参照を解決できない場合、または当該entityが`role`の課す`covers` / `anchor`制約に違反して構造上完全でない場合（§4.4）は`MISMATCH`。
- 全Discovered Testが`ManagedTestLink::One`を持ち、各linkがちょうど1件の構造上完全なentityを指し、Test IDが大局的に一意で、全`covers`参照を解決できる場合だけ`PASS`。
- discovery結果が不完全または解析不能なら`UNKNOWN`とし、PASSにしない。
- repository-level項目であるため、REQ / VO / TESTのentity scopeを指定してもDiscovered Test集合を狭めない。必要な場合は`--items`でこの項目自体をscope外にできるが、その値は`NOT_CHECKED`のまま保持する。

#### 11.1.2 適用項目集合と`role`

`role`はTestが存在する理由の分類であり、チェック項目の適用可否を`role`値から決めない。どの項目を評価するかは、entityごとの**適用項目集合（applicable check-item set）**として定める。

**適用項目集合**とは、ある具体化されたTest Entity — 構造上完全なManaged Test Entityと、§4.4で除去せずに保持する構造上完全でないentityの双方 — について値を生成して評価するチェック項目の集合をいう。構造上完全でないentityはManaged Test Entityではないが（基本仕様 §2、要件定義 §7.1）、具体化されている限り適用項目集合を持つ。repository-level項目である`test_traceability`は、個々のentityへはinstantiateせずrepositoryのscan result全体に対して一度だけ評価する（§11.1.1）。したがっていかなるentityの適用項目集合にも含まれない。構造上完全でないentityは`test_traceability`の**値の原因**（MISMATCH）にはなるが、その項目を当該entityへinstantiateすることとは別である。集合に含まれない項目は当該entityへ**instantiateしない**。値を生成せず、§11.3 step 10のTest単位のfail-closed算入にも参加しない。reportでは「非適用」として理由（例：`covers`なし）とともに表示し、`NOT_CHECKED`と明確に区別する。`NOT_CHECKED`は「適用対象でありながら未検証」を意味する値である（基本仕様 §4.1）。適用対象でない項目へこの値を与えると、未検証の事実と適用外の事実が同じ表示に潰れ、`NOT_CHECKED`が保護している意味が失われる。

完全検証の項目集合はproject全体で基本仕様 §4.2の12項目のまま変わらない。適用項目集合が制御するのはentity単位のinstantiateだけであり、非適用の存在は完全検証を12項目未満の限定scopeにしない。

適用項目集合は次で定める。

- 既定では、Test単位で評価する全項目を適用項目集合に含める。
- `covers`が0件のTestでは`impl_consistency`をinstantiateしない。`impl_consistency`は仕様・VO・Testと対象実装の一致判定であり（基本仕様 §7.5）、対象VOが無ければ判定根拠となる上流SPEC subjectの完全集合を導出できず、判定の入力自体が定義されない。
- 実効`role`が`None`のentity（§4.4）では、適用項目集合を`test_execution`、`runtime_result`、`evidence_validity`、`target_execution`へ縮退させる。この集合は、Testの目的を前提とせずに評価できるtest-localな実行系だけからなる。目的が確定していない段階で、その目的を前提とする監査次元（`static_audit`、`semantic_audit`、`impl_consistency`）をinstantiateしない。目的の不確定そのものは`test_traceability = MISMATCH`として報告済みであり、確定しない目的に対する監査結果を重ねて生成しない。

その上で次を不変条件とする。

- **Auditability と Contribution の分離**：管理下の全Testに、適用項目集合内の完全性・実行・鮮度・静的・意味の各検査を通常どおり適用する。`covers`を持たないTestは、これらの結果をいずれのVerification Obligationへも寄与させないだけである。
- `test_execution` / `runtime_result` / `evidence_validity`は全ての具体化されたTest Entityの適用項目集合に含まれ、`role`値によらず評価する。`test_traceability`はrepository-level項目であり、entityの適用項目集合の対象外である（上記）。
- `targets`は`role`によらず1件以上必須であり（基本仕様 §6.2）、`target_execution`は宣言targetに対して`role`によらず通常どおり適用する。
- 適用項目集合に含まれる限り、`static_audit`と`semantic_audit`のTest単位の判定は`role`値によらず評価する。`semantic_audit`の判定次元は`covers`の有無で定まる（§8.1）。`test_existence`、および`static_audit` / `semantic_audit`のVO対応面へは`covers`を持つTestだけが寄与する。`role`が`regression`で`anchor`が`normative`のTestは、`covers`先VOに対して`verification`のTestと同一に寄与する。
- **`role`とtopologyの直交**：`role`は実行topology、target到達性モデル（§7.3）の適用可否、および境界の意味論を決定してはならない。`role`の各値と、in-process / subprocess / 構造的な各topologyの組合せに制限を設けない。
- **`kind`と`role`の直交**：`kind`は実行上・Form上の分類であり、`role`を導出してはならない。`kind`にregressionを含むTest（`unit-regression`等。別紙A §14.1）と`role regression`は独立であり、一方から他方を推測しない。組込Formは`role`を宣言せず`verification`のTestだけを生成するため（別紙A §14.4）、`kind`にregressionを含む組込Form生成Testの実効`role`は`verification`である。

VOの検証への寄与は`covers`宣言と証拠の十分性判定だけから導出し、`role`から導出しない。

### 11.2 Evidence 鮮度判定

```text
対象 Test の Evidence のうち最新のものについて：

1. evidence.hashes.test_subject == 現在のTest subject hash
2. evidence.hashes.targetsの参照集合が、現在のTest.targetsを§6.1で解決したcanonical Locator集合と重複なく一致し、
   各target_constructが現在のimplementation construct hashと一致
3. evidence.revision.commit が非 null かつ現在のHEAD revisionと一致する
4. evidence.execution_state.complete == true かつ、同じschemaで現在再構築したExecution State subjectがcompleteで、hashが一致する
5. evidence.adapter が現在のTest.execution.adapterと一致する。adapter欠落形は§3.7の互換条件で一意に確認できる

1〜5 すべて成立 → evidence_validity = PASS
  （dirty: true でもExecution State subject一致なら有効。実行入力manifestが実体を保証する）
1 または 2 不成立 → STALE
3 不成立          → STALE（現在revisionに対する実行ではない）
4のrecord欠落またはhash不一致             → STALE
4のrecordがcompleteでない、または現在snapshotを完全に構築不能 → UNKNOWN
5が明示的不一致  → MISMATCH
5を確認不能      → UNKNOWN
Evidence なし     → NOT_EXECUTED
```

Evidenceは全宣言targetが一意に解決できる場合だけ生成される（§9.4）。現在の宣言targetのうち1件でもcanonical Source Targetへ一意に解決できなくなった場合、記録済み参照集合は現在のcanonical集合と一致しないため条件2は成立せず、`evidence_validity`をPASSにしない。
対象が存在せずE-SCAN-004となるtargetは`MISSING`、複数候補により曖昧でE-SCAN-011となるtargetは`MISMATCH`として保持する（§5.4）。いずれの場合も`target_execution`をPASSにしない。

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

チェック項目の**表示scope**と、項目導出に必要な**内部依存の評価**は分離する。§7.3 により `static_audit` は当該 Test の Evidence 鮮度（§11.2）と target 別 target_execution へ横依存する。`static_audit` が項目scopeに含まれる場合、aggregator は §7.3 の runtime 到達証明の判定に必要な範囲でこれらを**内部依存として評価する**（`target_execution` / `evidence_validity` が項目scopeに無くても評価する）。ただしこれは導出のための内部評価であり、scope 外の `target_execution` / `evidence_validity` 自体の report value は step 5 のとおり `NOT_CHECKED` のまま保持する。内部依存評価は scope 外項目の表示値を変えない。runtime 証明に依存する `static_audit` の値は、根拠として用いた Evidence ID と当該 target の target_execution 結果を report で引用する。限定 scope で内部依存の STALE / 非充足が `static_audit` の非 PASS を生む場合も、その内部依存状態を根拠として提示し、scope 外項目の表示値（NOT_CHECKED）だけでは原因を辿れない状態にしない。

`covers`を持たないTestのscope内評価値も総合判定に参加する。いずれのleaf VOの合成にも参加しないため、step 10では当該Testの**適用項目集合内**の非PASSをTest単位で直接fail-closedに算入する。適用項目集合外の項目は値を持たないため算入対象にならず、非適用を`NOT_CHECKED`として算入することもしない（§11.1.2）。管理下のTestが壊れた事実を、寄与先VOが無いことを理由に総合判定から落とさない。

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

この「その時点の正典の読み取り」は書込みの**原子的公開**（基本仕様 §5.2）を前提とする。原子的公開の対象は`.verify/`配下のrecord・エンティティファイル（新規レコード追加とエンティティファイル編集）であり、完全な内容が単一の操作で可視になる方式（同一ファイルシステム内へのtemp書込み＋rename等）で公開し、書きかけ状態・一時ファイル残渣を正典ディレクトリの読み手に観測させてはならない。テストコード編集は通常のソース編集と同じ扱いで本規定の対象外とし、解析不能な中間状態は adapter discovery の E-SCAN-001 / Incomplete としてfail-closedに検出される（§5.1）。

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
| E-EXEC-004 | error | 実行中にExecution State subjectが変化 |
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
