# AI並列開発向けテスト検証システム 詳細設計 v0.1（本冊）

## 0. 本書の位置付け

本書は「基本仕様 v0.1」を実装可能なレベルまで具体化する。
基本仕様が定めた外部挙動の保証を変更しない。
本書と基本仕様の間に矛盾がある場合、基本仕様を正とし、本書の該当箇所を不整合として扱う。

本書は HOW（具体構文・アルゴリズム・データ構造・ID 命名・schema）を定める。基本仕様（WHAT）に無い義務・検査・状態・文書種別・関係型を発明しない。規範の伝播は上流→下流（要件定義 P-005）であり、矛盾・不足を発見した場合は本書を書き換えず上流へフィードバックし Owner 判断を経る。

本書からの `基本仕様 §n` 参照は再導出済み基本仕様 v0.1 の連番（§0〜§30）を、`要件定義 §n` 参照は凍結要件定義 v0.1 の連番（§1〜§28・P-001〜P-005・R-1〜R-5）を指す。

### 分冊構成

正規の詳細設計は3分冊とし、節番号は正規文書間を通した連番とする。別紙Bは非正規のprocess文書として別に扱う。

| 文書 | 位置付け | 収録節 |
|---|---|---|
| 本冊（コア設計） | 正規 | §1〜§11、§16、§17、§19 |
| 別紙A（CLI・MCPインターフェース仕様） | 正規 | §12〜§15 |
| 別紙B（実装計画） | 非正規 / process | 正規節番号を持たない |
| 別紙C（受入仕様） | 正規 | §18 |

本冊の新設サブ節（§5.6 文書層孤児検出、§11.5 フェーズゲート、§11.6 役割別 projection、§11.7 判断待ち情報）は本冊の収録節範囲内に置き、別紙A / C の節番号を侵さない。基本仕様が固定する CLI コマンド一覧（基本仕様 §26.1）・MCP ツール一覧（基本仕様 §26.2）を本書で増やさず、新設機能は既存コマンド・ツールの引数と出力で露出する。引数・入出力の完全 schema は別紙A が定める（本書は意味論とデータ schema、および露出点だけを確定する）。

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
    vtest-adapter-rust/ # rust-cargo discovery/static-analysis/operations/runner/coverage
    vtest-scan/         # discovery委譲、結果統合、record整合性
    vtest-audit/        # 静的解析委譲、判断記録bundle生成、提出結果検証
    vtest-exec/         # runner委譲、revision取得、Evidence記録
    vtest-verify/       # 整合性検査、鮮度検証、集約、レポート生成、ゲート評価
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
`git` が利用できない場合、リビジョンは特定できず、当該 Evidence は §6 のハッシュ束縛（revision 一致）を満たさないため `target_binding` の証拠として有効な `PASS` にならない（fail-closed）。この失効は独立検査ではなく診断ラベル `STALE` として説明する（§11.2）。

### 1.3 内容ハッシュの定義

**内容ハッシュ**はSHA-256を使用し、`sha256:<hex>`形式で記録する。subject固有規則でbyte-exactを要求しないテキストfragmentは改行をLFへ統一し、各行の末尾空白を除去する。これ以外の空白は正規化しない。

hash inputはdomain separatorと長さ付きfieldから構成する。各fieldは`field-name`、UTF-8 byte length、byte列の順にencodeし、単純な文字列連結を行わない。mapはkey昇順、集合として扱う`covers`・`targets`・`related`は正規化値の昇順、順序に意味がある`cases`は宣言順とする。null、空文字、空listは異なる値としてencodeする。

- **Test subject hash**：domain `vtest:test-subject:v1`を用い、adapter ID、Test ID、全canonical metadata、Source Locationのadapter・project-relative path・opaque locator、ExecutionDescriptor、および正規化したTest construct bytesを束縛する。byte range自体は前方の無関係な編集で変化するためhash inputにしない。metadata宣言がmanifest等の非隣接箇所に存在しても、adapterが返す論理metadataを同じsubjectへ含める。canonical metadataの`targets`は**宣言された**`TargetRef`の正規化値を束縛し、解決後のcanonical Locatorへ置換しない。これによりTestの参照方法の変更（同一Source Targetへのlocator参照からSRC ID参照への書き換え等）はTest subject hashで捕捉される（§6.1.1）。canonical metadataは `id` / `covers` / `targets` / `intent` / `input` / `expect` / `kind` / `cases` / `related` からなる（`role` / `anchor` 等の存在理由分類 field は本 version では持たない。§4.1）。宣言の不在と空値の明示は異なる値としてencodeする。
- **Source Target hash**：domain `vtest:target-subject:v1`を用い、**canonical Target Reference**とadapterが返すimplementation construct bytesを束縛する。この hash は検証対象を **Source Target として実現した形態**の identity 束縛であって、core が「検証対象とは何か」を Source Target に限定して定義するものではない（検証対象は一般概念。基本仕様 §9.1、§1.3 冒頭方針は §4.1）。construct bytes への束縛は当該実現形態（`rust-cargo` 等の Source Target 形態）に対する規則であり、Source Target を宣言しない検証対象形態へは適用しない。canonical Target Referenceは**常に`TargetRef::Locator`**（adapter IDとadapter所有のopaque locator）であり、`TargetRef::SrcId`をcanonical Target Referenceにしない。`TargetRef::SrcId`はSource Targetを参照する側の表現であって、Source Target自身の識別ではない。恒久SRC IDはhash inputの独立fieldとして束縛せず、canonical Target Reference経由でもhash inputへ入らない。恒久SRC IDの宣言・変更・削除はcanonical Target Referenceを変えない。ただし恒久SRC IDの宣言をSource Targetのconstruct bytesの内側へ置くadapter（`rust-cargo`の`@vtest.src-id` doc comment等。§5.5）では、その宣言の追加・変更・削除がconstruct bytesを変化させ、construct bytes経由でSource Target hashが変化しうる。これは正しい挙動であり、恒久SRC IDが独立したhash fieldであることを意味しない。hashはSource Target自身のcanonical Locatorから一度だけ計算し、当該Source Targetを参照するTest側の`TargetRef`綴りからは計算しない。Evidence、検証は解決後のcanonical Source Targetのcanonical Locatorとhashへ束縛し、addressing modeごとに別subjectを作らない（§6.1）。
- **document subject hash**：domain `vtest:document-subject:v1`を用い、canonical document recordと参照先 source（`path` の実ファイル）の正規化内容を束縛する。document recordの`content_hash`と実sourceが不一致ならsubject hashは現在有効な値として成立せず、`chain_integrity` の非 `PASS`（`MISMATCH`、診断 `STALE`）とする（§11.4）。要件定義・基本仕様・詳細設計・API Schema 等を種別で区別せず、すべて同一の総称 document subject として計算する（§3.1、基本仕様 §3.2）。
- **VO subject hash**：domain `vtest:record-subject:v1`を用い、readerが具体化したcanonical VO recordをfield規則に従ってencodeする。VOの読取り互換field `status`は正典ではないため含めない。`derives_from`（参照先 document ID 集合）と `parent` を束縛し、`covers`の増減はTest側subjectで捕捉するためVO subjectには含めない。
- **Execution State subject hash**：domain `vtest:execution-state:v1`を用い、adapter ID、snapshot schema ID / version、HEAD revision、runner kindとcanonical invocation projection、toolchain identity、実行結果へ影響するadapter configのcanonical projection、および実行可能状態を変えうるrepository / local dependency入力の完全なmanifestを束縛する。manifest entryはstable root identity、root-relative path、input kind、byte-exact file bytesからなり、entry集合は正規化identity順にencodeする。stable root identityはmachine上の絶対pathを用いず、workspace内の論理rootまたはdependency identityから決定論的に導出する。adapterはhash未計算のmanifestと完全性を返し、coreが各entryとsubject全体を検証・hash化する。

adapterはsource location、source rangeと現在のbytes、解析済みlogical metadata、ExecutionDescriptorをhash未計算のdiscovery DTOとして返す。coreはadapter出力と現在のsource bytesの対応を検証し、上記の言語非依存encodingとSHA-256計算を行ってからdomain entityを具体化する。adapterが最終的な`TestEntity.content_hash`または`SourceTarget.content_hash`を返して自己確定してはならない。coreはASTや言語固有構文からrangeを再計算しない。

Test Runner adapterも、実行状態へ用いたconfig / manifestをhash未計算DTOとして返す。coreは現在bytesとの対応、重複、集合完全性、schema versionを検証してsubject hashを計算する。adapterが完全性を保証できないDTOから`PASS`用subjectを具体化してはならない。

静的解析（§7）は正典レコードを持たず、検証のたびに現在の source / config から再計算する派生情報である（§7.1、基本仕様 P-003）。したがって静的解析結果は内容ハッシュに束縛された永続 subject を持たず、上記の hash 体系に静的解析専用の subject を設けない。

`rust-cargo` adapterはTest constructとしてmetadata doc commentを除き、実行に影響する属性、signature、bodyを含む関数itemのbytesを返す。doc comment由来metadataはlogical metadataと`metadata_sources`として別に返す。Source Targetには属性とdoc commentを含む関数item全体を返す。format変更を構文上の意味だけから同値とみなさず、上記正規化後のsource bytesが変化した場合は安全側でSTALEにする。

---

## 2. データディレクトリと設定

### 2.1 `.verify/` レイアウト

基本仕様 §24.1 の layout をそのまま採用する。

```text
.verify/
  config.yaml
  doc/        DOC-<NAME>.yaml   総称 document レコード（正典）
  vo/         VO-<NAME>.yaml    VO レコード（正典）
  rel/        REL-<ULID>.yaml   外部 Relation レコード（正典・不変）
  forms/      <kind>.yaml       Form Schema（正典）
  decisions/  <ULID>.yaml       判断記録（事実・追記型）
  approvals/  <ULID>.yaml       承認記録（事実・追記型）
  evidence/   <ULID>.yaml       実行証拠レコード（事実・追記型）
  cache/      （Git管理外。.verify/.gitignore に `cache/` を出力する）
    bundles/  判断バンドル JSON（派生・再生成可能）
    logs/     テスト実行の生ログ
    cov/      coverage 生出力
```

文書種別ごとの専用ディレクトリ（旧 `spec/` / `req/`）を設けず、上流文書はすべて `doc/` の総称 document レコード1種で表現する（基本仕様 §3.1、§3.2）。決定論的解析の結果を保存する正典ディレクトリ（旧 `audits/`）を設けない。静的解析は再計算派生であり `cache/` にのみ置く（§7.1）。外部判断は `decisions/` の判断記録として保存する。

`vtest init` は上記ディレクトリ、`config.yaml` の雛形、`.verify/.gitignore`、組込 Form Schema（別紙A §14）を生成する。

### 2.2 `config.yaml`

`config.yaml` writer の正規形は version 2 とし、adapter ごとに root・scan・run 設定を namespace 化する。reader は version 1 を単一の `rust-cargo` adapter 設定として in-memory 変換して読み取るが、読み取りだけで正典を書き換えない（基本仕様 §2.4）。

```yaml
version: 2
project:
  name: example
adapters:
  - id: rust-cargo
    roots: ["."]
    scan:
      include: [src, tests, crates]   # テストコード走査パス。省略時はワークスペース全体
      assertion_macros: []            # 追加で assert 相当として扱うマクロ名
    run:
      coverage: llvm-cov              # target_binding 動的計測方式: llvm-cov | off
doc:
  roots: [DOC-REQ-ROOT]               # orphan_detection の除外根（§5.6、基本仕様 §5.2）
verify:
  full_scope: [chain_integrity, orphan_detection, target_binding, oracle_presence]
gates:                                # フェーズゲート定義（§11.5、基本仕様 §20）
  - name: development
    require: { verification: PASS }
  - name: release
    require: { verification: PASS, approvals: [reviewer] }
  - name: delivery
    require: { verification: PASS, approvals: [owner] }
```

adapter IDの重複、同一adapter内のroot重複、未知adapter、無効なadapter設定はusage error（E-CONFIG-001）とする。
異なるadapterが同じrootを共有することはpolyglot repositoryのために許可し、統合したTest IDは
全adapterでglobal uniquenessを検査する。adapter固有設定の検証は登録adapterへ委譲し、
coreは未知のnamespaceや値をRust設定として解釈しない。`vtest init`はversion 2を生成する。

`verify.full_scope`は利用者が完全検証を縮小する設定ではなく、基本仕様 §5 の**固定4検査**（`chain_integrity` / `orphan_detection` / `target_binding` / `oracle_presence`）を列挙するconfig invariantである。version 2 では重複・未知項目・欠落・余剰を E-CONFIG-001 で拒否する。version 1 では field 欠落を固定4検査として具体化し、重複または未知項目は E-CONFIG-001 で拒否する。旧12項目の列挙（`spec_coverage` / `test_existence` 等）は現行 invariant に違反するため version を問わず E-CONFIG-001 とし、in-memory 補完で受理しない。`--items` による明示的な部分集合だけを限定scopeとして扱い、項目指定を省略したCLI / MCP検証は常に固定4検査を評価する。いかなる設定値も完全検証の検査を4本未満へ縮退させない（基本仕様 §4.6、§22.1）。

`gates` はフェーズゲートの進行条件定義を保持する（§11.5）。config 読込み時に次を検査し、いずれか違反があれば E-CONFIG-001（終了コード 2）として設定を受理せず、検証結果を生成しない。

- `gates` field 自体の欠落と空 list は「ゲート定義なし」として受理する（`--gate` を指定しない実行は影響を受けない。未定義名の指定は §11.5）。
- `gates[].name` は非空文字列であり、大文字小文字を区別した完全一致で重複してはならない。`--gate <name>` の解決は同じ完全一致で行う（§11.5）。
- `gates[].require` は必須とし、その `verification` も必須とする。いずれの欠落も E-CONFIG-001 とする。
- `require.verification` の値は、基本仕様 §4.1 の 5 状態語彙（`PASS` / `FAIL` / `MISMATCH` / `NO_EVIDENCE` / `UNKNOWN`）のいずれかと大文字小文字を区別して完全一致しなければならない。診断ラベル（`MISSING` / `NOT_EXECUTED` / `NOT_CHECKED` / `STALE`）、`OK` / `NG`、旧12項目名、5状態の小文字表記・別綴り、list・object などの非文字列値はすべて E-CONFIG-001 とする。5 状態のうち `PASS` 以外を要求する定義自体は受理し、充足判定の意味は §11.5 で定める。
- `require.approvals` は省略可能とし、省略は「要求する承認ロールなし（空集合）」として受理する。指定する場合は文字列ロール名の list とし、空文字列・重複ロール名は E-CONFIG-001 とする。ロール名が `approval_roles` に解決できない場合も E-CONFIG-001 とする（別紙A §12.3）。

`doc.roots` は orphan_detection の除外根を DOC ID の集合として保持する（§5.6）。`scan` と `run` は version 1 schema 互換の wire 値とし、Rust固有の macro path や `llvm-cov` 制約は `rust-cargo` adapter に限って適用する。非Rust namespaceの値を core がRust設定として推測・書換えしてはならない。

### 2.3 派生情報

検証グラフとindexは実行のたびにインメモリで再構築する。
永続cacheを正典または検証入力として使用しない。`cache/`は再生成可能な派生物（判断バンドル、静的解析結果、実行ログ、coverage 生出力）だけを格納する。
MCP サーバは長時間動作するため、ツール呼び出しごとに対象ファイルの mtime を確認し、変化があれば再スキャンする。

---

## 3. レコードファイルスキーマ

すべてのレコードは YAML とし、未知フィールドはエラーではなく警告とする。
`id` とファイル名（拡張子除く）は一致しなければならない。

### 3.1 document レコード（`.verify/doc/DOC-*.yaml`）

上流文書はすべて単一の総称ノード型 `document` で表現する（基本仕様 §3.1）。要件定義・基本仕様・詳細設計・API Schema・Protocol Specification・型/データ仕様・DB schema・その他の機械可読仕様を種別で区別する専用スキーマを持たない。

```yaml
id: DOC-BASIC-001
path: docs/basic-spec.md        # プロジェクト相対パス
content_hash: "sha256:..."      # 登録時の内容ハッシュ（§1.3 document subject）
title: 基本仕様書               # 任意の表示名
derives_from:                   # 上流 document への導出リンク（0件可＝根候補）
  - doc: DOC-REQ-001
    note: ""                    # 任意の導出理由（空可・非 MISMATCH。基本仕様 §3.4）
registered_at: 2026-08-08T00:00:00Z
```

- `derives_from` は上流 document への唯一のリンク種別である（基本仕様 §3.2）。文書層の段（要件→仕様→詳細設計…）は `derives_from` リンクで表現し、段を増やしても種別を増やさない。リンクを追加してもスキーマは壊れない。
- 各 `derives_from` entry の `note`（導出理由・説明文）は任意（optional）であり、空でも `chain_integrity` 違反・`MISMATCH` としてはならない（基本仕様 §3.4、§19）。ただし付加・保存できる構造とする。
- `derives_from` の参照先 document が存在しない場合は文書鎖のリンク切れとして `chain_integrity` の `MISMATCH`、`path` の実ファイルが `content_hash` と一致しなくなった場合は `MISMATCH`（診断 `STALE`）とする（§11.4）。
- `derives_from` が空の document は根候補であり、`config.yaml` の `doc.roots` に列挙されない場合は孤児として `orphan_detection` の `MISMATCH` とする（§5.6）。
- 仕様文書そのものは `.verify/` へ複製しない。本システムは文書内容の意味的良否を検証しない（基本仕様 §29 OOS-001）。

### 3.2 VO レコード（`.verify/vo/VO-*.yaml`）

```yaml
id: VO-PARSER-UTF8-003
parent: VO-PARSER-UTF8          # VO ID または null（階層化）
derives_from:                   # 1件以上の document への直結（基本仕様 §3.2）
  - doc: DOC-BASIC-001
    note: ""                    # 任意（空可・非 MISMATCH）
claim: 不正な continuation byte を含む入力を与えた場合、ParseError::InvalidUtf8 を返す
dimensions: []                  # 検証軸（任意。§3.2.1）
coverage_policy: null           # independent-axes | full-product | explicit | null
representative_cases: []        # 代表入力値（任意）
created: 2026-08-08
updated: 2026-08-08
```

VO は 1 件以上の `document` から `derives_from` で導出される。VO と document の間に他のエンティティ層を置かない（基本仕様 §1、§3.2）。旧モデルの `requirements`（REQ 参照）と `spec_refs`（SPEC + 節参照）は持たず、上流参照は `derives_from:[DOC-]` へ一本化する。`derives_from` の参照先 document が存在しなければ `chain_integrity` の `MISMATCH`（dangling reference、E-SCAN-003 相当は §5.4 の E-SCAN-012）。

VOの`status`は承認レコードから導出する表示値であり、canonical writerはVO recordへ保存しない。
readerは読取り互換fieldとして`status`を受理するが、実効判定とVO subject hashでは無視し、
存在自体をW-STORE-001として通知する。互換field値と導出値が異なる場合も導出値だけを使用する。

#### 3.2.1 dimensions と組合せの実体化

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
実体化後は通常の VO として扱われるため、`chain_integrity` の leaf VO → Test 検査は「leaf VO に covers する Test が存在するか」だけを見ればよい。
組合せ空間の定義が仕様に対して十分かは本システムの検査ではなく、`UNKNOWN` としてエスカレーション（§8、基本仕様 §11）の領分である（基本仕様 §10）。

### 3.3 Relation レコード（`.verify/rel/REL-<ULID>.yaml`）

どちらか一方のエンティティに自然に所属しない関係（VO 間の依存、Test 間の補完関係など）だけを保存する（基本仕様 §2.3）。`derives_from`・`covers`・`targets` は adapter 所有の宣言または document / VO record から導出できるため、外部 Relation として重複保存しない。

```yaml
id: REL-01J8XVZK3Q...
type: depends-on          # depends-on | supersedes | regression-for |
                          # derived-from | same-partition | complements | conflicts-with
from: TEST-PARSER-044     # 任意のエンティティID
to: TEST-PARSER-012
note: ""                  # 任意の説明文
created: 2026-08-08T00:00:00Z
```

canonical Relation IDは`REL-`と26文字のULID payloadからなり、writerは`.verify/rel/REL-<ULID>.yaml`と同値の`id`だけを生成する。readerはversion 1互換入力として`.verify/rel/<ULID>.yaml`かつ同値のbare `id`を受理し、`REL-<ULID>`へin-memoryで正規化するが、読み取りだけでファイルを書き換えない。prefixed / bareの混在、ファイル名と`id`のpayload不一致、または同じpayloadの複数recordはE-SCAN-010とし、いずれかを選ばない。
Relation は不変。変更はファイル削除＋新規作成で表す。
`from` / `to` の存在はスキャン時に検査する（不在は E-SCAN-009、`chain_integrity` の `MISMATCH`）。

### 3.4 判断記録レコード（`.verify/decisions/<ULID>.yaml`）

判断記録は、`UNKNOWN` に対して外部（人間または判断可能 Agent）が下した判断の記録である（基本仕様 §11.3、要件定義 §12）。actor / subject / decision を必須項目とし、理由・根拠は任意。依存 closure のハッシュに束縛される。

```yaml
id: 01J8XVZZ...
subject: TEST-PARSER-044        # 判断対象のエンティティID または解決済み canonical Locator
subject_hash: "sha256:..."      # 判断時点の対象の内容ハッシュ
dependencies:                   # 判断時点の上流依存closure（完全一致を要求）
  - kind: vo
    id: VO-PARSER-UTF8-003
    hash: "sha256:..."
  - kind: document
    id: DOC-BASIC-001
    hash: "sha256:..."          # §1.3 document subject hash
actor:                          # 誰が（必須）
  kind: agent                   # human | agent
  id: judge-agent-01
  model: claude-fable-5         # agent の場合任意
decision: accepted              # どう判断したか（必須。値の妥当性は §8.4）
reason:                         # 理由・根拠・evidence note（任意。空でも無効化しない）
  - claim: テストは不正UTF-8入力に対する InvalidUtf8 の返却を検証している
    basis:
      - kind: test-code
        ref: "rust-cargo::tests/parser_test.rs::rejects_invalid_utf8"
exclusions: []                  # 対象外とした範囲（任意）
decided_at: 2026-08-08T00:00:00Z
revision: { commit: "abc123...", dirty: false }
```

- **理由が空であることだけを根拠に、その判断を無効・`UNKNOWN`・`NO_EVIDENCE`・`MISMATCH` 等として扱ってはならない**（基本仕様 §11.3、要件定義 §12）。`reason` / `exclusions` は optional である。
- 同一対象への判断記録は複数存在してよい（再判断・多重判断）。有効性判定と選択は §8.5 に従う。
- `subject` の `target` 参照は §6.1 で解決した canonical Source Target の canonical Locator とし、解決できない target を任意の候補で埋めない（§6.1.1）。
- **判断記録の受理は当該対象の検証状態（§4.1 の 5 状態）を昇格させない**（§8.3、基本仕様 §11.3）。判断記録は検査ゲートではなく、`UNKNOWN` に対する外部判断の追跡である。判断済みと承認済みは区別する（判断済み ≠ 承認済み。§3.5）。

### 3.5 承認レコード（`.verify/approvals/<ULID>.yaml`）

```yaml
id: 01J8XW0A9M...
subject: VO-PARSER-UTF8-003     # 承認対象のエンティティID
judgment_ref: 01J8XVZZ...       # 参照する判断記録ID（任意。judgment reference）
subject_hash: "sha256:..."      # 承認時点の対象の内容ハッシュ
dependencies:                   # 承認時点の上流依存closure（完全一致を要求）
  - kind: vo
    id: VO-PARSER-UTF8
    hash: "sha256:..."
  - kind: document
    id: DOC-BASIC-001
    hash: "sha256:..."          # §1.3 document subject hash
approver:
  kind: agent                   # human | agent
  id: reviewer-agent-01
  model: claude-fable-5         # agent の場合任意
approved_state: approved        # どの承認状態か（必須）
basis: []                       # 根拠（任意）
approved_at: 2026-08-08T00:00:00Z
```

承認は検証状態と**独立の別軸**である（基本仕様 §4.5、§17、要件定義 §5.5）。承認済みを理由に非 `PASS` を `PASS` へ昇格させず、未承認を理由に `PASS` を降格させない。承認記録は §3.4 の判断記録と同一 entity であることを要求しない（別 entity でありうる）。承認は対象自身または参照する判断（`judgment_ref`）に承認済み状態を与える。

VO の実効承認状態は次で決まる。

```text
approved =
  「subject が一致し、subject_hash が現在の内容ハッシュと一致する
   かつ dependencies が現在の上流依存closureとentity・hashとも完全一致する
   という条件を満たす承認レコードが1件以上存在する」
それ以外は draft（承認失効を含む）
```

上流依存closureは、対象VOの再帰的なparent VO、対象VOとparent VOが`derives_from`で参照する document、および各 document の再帰的な上位 document（`derives_from` 先）からなる（基本仕様 §17）。
対象VO自身は`subject_hash`で束縛するため`dependencies`へ重複して含めない。
entryは`kind`（`vo` | `document`）、`id`の順でsortし、欠落・重複・余剰entryを許可しない。

document dependencyは§1.3のdocument subject hashを使用するため、document recordまたは参照先sourceの変更で
承認が失効する。依存entryを持たない互換Approvalは読取りと履歴表示だけを許可し、
現在の`approved`を導出しない。W-STORE-002を出し、VOは`draft`相当とする。

承認記録は「誰が（approver）」「何を（subject または judgment reference）」「どの承認状態か（approved_state）」を必須項目として追跡可能とし、根拠は任意に記録できる。承認主体は種別（`human` / `agent`）と識別子を記録する。誰がどの対象・範囲を承認できるか（approval authority）、承認ロール・必要承認数・権限 schema・承認 workflow の状態遷移はプロジェクト側で定義可能とし、その具体は別紙A / プロジェクト設定へ委譲する（基本仕様 §17、§30）。

### 3.6 Evidence レコード（`.verify/evidence/<ULID>.yaml`）

```yaml
id: 01J8XW1B...
test_id: TEST-PARSER-044
adapter: rust-cargo
result: PASS                    # ランナーが報告した PASS | FAIL（判定権威 §7）
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
target_coverage:                # target_binding の動的計測結果（旧 target_execution field を改名）
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

`result` はテストランナー（判定権威。§7）が報告した合否をそのまま記録する。本システムは合否を再判定せず、この `result` を `target_binding` の証拠として消費する（基本仕様 §7）。有効な Evidence の `result: FAIL` は `target_binding = FAIL`（テストランナーが失敗を報告。要件定義 §5.3）へ至る（§11.2）。

`target_coverage` は `target_binding` の動的計測（宣言対象の実行が生じたか）の結果であり、独立の検査項目ではない。旧モデルの `target_execution` 検査項目は撤去し、その計測事実だけを Evidence の `target_coverage` field として保持して `target_binding` の証拠源へ吸収する（§10、§11.2）。

`hashes.targets`はTestの宣言順で常に記録し、各`target`は§6.1で解決したcanonical Source Targetの
**canonical Locator**の正規化文字列表現とする。参照側Testが宣言した`TargetRef`の綴り（SRC ID参照を含む）を
Evidence上のtarget identityとして記録しない（§6.1.1）。
このlistはTestの宣言target集合を解決したcanonical Source Target集合と重複なく1対1に対応する。
Evidence生成のprecondition（§9.4）により全宣言targetは一意に解決済みであるため、この集合は宣言target集合と同数になる。
`target_coverage.checked: true`では`target_coverage.targets`も同じ順序・同じcanonical Locator集合で1対1に対応する。
`target_coverage.checked: false`では`method`と`result`をnull、`targets`を空listとし、`target_binding`の動的計測を`NO_EVIDENCE`（診断 `NOT_CHECKED`）として扱う（§11.2）。

writerは`hashes.test_subject`を必須とし、Test construct単体のhashを現在のEvidence freshness keyとして出力しない。

readerは`rust-cargo` Evidenceに限り、互換fieldの`hashes.test_fn`または`hashes.test_construct`とtarget entry内の`target_fn`を読み取れる。互換Test hashを現在の`test_subject`へ正規化できるのは、現在の`rust-cargo` adapterが当該互換hashのsource rangeに全canonical metadataとTest constructが含まれること、現在bytesとの完全一致、および現在のlogical metadataとの一致を証明できる場合だけとする。証明できなければrecordは保持するが`target_binding`の証拠として有効な`PASS`にしない。中立fieldと互換fieldが併存する場合は導出される値の同値を必須とし、非`rust-cargo` Evidenceでは互換fieldを解釈しない。
readerは単数互換形の`hashes.target_fn`および`target_coverage.result/count`を、現在の`rust-cargo` Testがtargetをちょうど1件宣言し、上記条件でTest subjectを証明でき、target construct hashも照合できる場合だけ1要素listへ正規化して扱う。
複数target Testに単数互換形を適用せず、writerは常にlist形を出力する。

Evidence内の`target`は実行時snapshotを識別するkeyであり、TEST → SRC edgeの正典ではない。
graphはadapter所有のTest metadata宣言からだけ構築し、Evidenceのtarget listからedgeを生成しない（基本仕様 §2.3）。

`execution_state`は§1.3のExecution State subjectである。writerは実行直前にadapterからsnapshot DTOを取得し、core検証後のschema ID、完全性、subject hashを記録する。`complete: true`は、選択Testのビルドと実行可能状態を変えうるrepository / local dependency入力、runner、toolchain、実行影響configをadapterが漏れなく列挙した場合だけ許可する。snapshot生成不能または不完全の場合も実行事実の履歴を記録できるが、`complete: false`、`hash: null`として現在の有効な`PASS`証拠へ使用しない。

`rust-cargo-execution-state-v1`のmanifestは、選択Testを含むCargo workspace / package root、全local path dependency root、各root内の通常file、Cargo manifest / lockfile、`.cargo` config、build script、Rust source / test / fixture / compile-time resource、toolchain指定を含む。`.git/`、`.verify/`のcanonical record / cache、Cargo target directory等の生成物は実行入力から除外する。除外領域をbuild script、macro、`include_*`、path dependencyその他の経路で読み込む可能性を排除できない場合、snapshotを完全と報告しない。repository内helperだけの変更もmanifest hashを変化させる。

Evidence readerは`execution_state`を欠く互換recordを履歴表示できるが、現在のEvidence freshnessを証明できないため`NO_EVIDENCE`（診断 `STALE`）とする。

schema違反、target entryの欠落・重複・余剰、またはaggregate resultとtarget別結果の矛盾は
E-SCAN-010として扱い、そのEvidenceを有効な結果に使用しない。

Evidence writerは `adapter` を必須で記録し、保存前にTestの
`ExecutionDescriptor.adapter`およびrunner kindとの整合を検証する。Evidence readerは
`adapter` の欠落を許容するが、現在のTestが `rust-cargo` で、互換runner kindと内容ハッシュから
Rust実行であることを一意に確認できる場合だけ互換Evidenceとして扱う。確認不能は
`UNKNOWN`、明示adapterの不一致は `MISMATCH` とし、いずれも `PASS` へ昇格しない。

---

## 4. Test metadata宣言contract

### 4.1 adapter-neutralな正規化

`SourceDiscoveryAdapter`は、adapter所有のsource declarationを次の論理fieldへ正規化する。

```text
id, covers[], targets[], intent, input?, expect?, kind?, cases[], related[]
```

本 version では Test の存在理由分類（旧 `role` / `anchor` / `anchor_rationale`）を論理 field に持たない。すべての管理対象 Test に `covers ≥ 1` を一律に要求し（基本仕様 §12、要件定義 §4.1）、VO への寄与は `covers` 宣言と証拠の十分性判定だけから導出する。

**検証対象は一般概念であり、adapter 中立 core は各管理対象 Test に 1 件以上の検証対象を要求する**（基本仕様 §9.1、要件定義 §9.1）。検証対象は「その Test が検証成立性を証明しようとする対象＝宣言された『何の時にどうなる』の主語」であって、実装 construct に限定しない。検証対象を実装 construct（Source Target）として実現するか、外部から観測可能な契約・境界上の振る舞いとして実現するかは**実行形態が定める**（基本仕様 §8.3・§9.1、要件定義 §4.3）。したがって core の `chain_integrity` は「検証対象を Source Target として実現し `targets ≥ 1` を宣言すること」を adapter 中立の必須リンクとしない（core の Test 層必須は Test ID・`covers ≥ 1`・その他の必須 metadata。§11.1.1、基本仕様 §5.1・§12）。`targets[]` は検証対象を Source Target として実現するための capability field であり（基本仕様 §9.2）、その要求件数は adapter が定める。v0.1 の唯一の adapter `rust-cargo` は検証対象を Source Target として実現し `targets ≥ 1` を必須とする（§4.2・§4.4・§5.5）。非 source の境界形態（外部契約・境界上の振る舞い）の具体的表現・確認方法は特定形態を他形態へ一律要求せず（要件定義 §4.3、基本仕様 §5.3）、下位仕様・後続 adapter・後続版へ委譲する（本 version で Contract-Target 類の新 schema は設けない）。

coreはsource declarationの構文と配置を解釈せず、adapterが返したTest Entity、Discovered Test observation、Source Location、Target Reference、source range、診断を検証・統合する。
locatorは`TargetRef::Locator { adapter, value }`とし、`value`はadapter所有のopaque文字列である。coreがpath、module、symbol種別を分解しない。

### 4.2 `rust-cargo` annotation文法

`rust-cargo`の`@vtest.`宣言表面は次の2種であり、本節は両表面の文法を定義する。表面ごとに認識する行形式が異なる。

1. **Test constructのdoc comment**（`///` または `/** */`）: test-annotation-line を認識する。
2. **Test constructではない関数itemのdoc comment**（対象実装側の関数等）: source-target-annotation-line を認識する。

```text
test-annotation-line          = "@vtest." test-key SP value
source-target-annotation-line = "@vtest." source-target-key SP value
test-key          = "id" | "covers" | "target" | "intent" | "input"
                  | "expect" | "kind" | "case" | "related"
source-target-key = "src-id"
value             = 行末までのテキスト（前後空白は除去）
```

- 1行1キー。`covers` と `related` の値はカンマ区切りで複数指定できる。
- `case` と `related` はキー自体を複数行書ける。他のキーの重複はエラー E-SCAN-005。ただし `kind` が integration 系の Test に限り、`target` の複数行を許容する（別紙A §14.3）。許容された複数`target`内でも同じTargetRefの重複はE-SCAN-005とする。綴りが異なっても解決後に同一canonical Source Targetへ到達する複数宣言（同じSource Targetへのlocator参照とSRC ID参照の併記等）も、coreが解決時にE-SCAN-005とする（§6.1.1）。
- 表面1で、`@vtest.` で始まるが test-key を持たない行はエラー E-SCAN-006（打鍵ミスの検出を優先し、警告ではなくエラーとする）。未知キーに加え、source-target-key（`src-id`）の誤配置も含む — `src-id` は対象実装側の関数に付与すべきキーであり、Test metadataへの取り込み先を持たない。
- 表面2で、`@vtest.` で始まるが source-target-key を持たない行（test-keyを含む）は警告 W-SCAN-105 とする（§5.4）。打鍵ミス検出の目的は両表面に及ぶが、表面2の宣言はTest metadataを破損させず採用値の曖昧さも生まないため、errorではなくwarningとする。
- `src-id` は表面2でも反復不可であり、同一関数itemでの重複は採用すべきIDを決定できないためエラー E-SCAN-005 とする。このときいずれの宣言値も採用せず、当該Source TargetのSRC IDは無しとして扱う（どちらかを推測で選ばない）。
- doc comment 内の `@vtest.` を含まない行は自由記述として無視する。
- `@vtest.src-id` はテストではなく対象実装側の関数に付与し、任意の恒久SRC IDを宣言する。scannerは指定値を認識するが、付与を必須としない（基本仕様 §9.2）。`rust-cargo`のSource Target constructは属性とdoc commentを含む関数item全体であり（§1.3）、この宣言行はconstruct bytesの内側にある。したがって`@vtest.src-id`の付与・変更・削除はSource Target hashを変化させる。この表面での打鍵ミス（`src_id` 等の未知キー）は W-SCAN-105、`src-id` の重複は E-SCAN-005 で検出し、無音で無視しない（§4.2・§5.4）。

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

adapter固有のsource declarationを構文解析できない場合、adapterは該当Test constructをDiscovered Testとして返し、対応を`ManagedTestLink::Missing`として診断を付与する。coreは当該 Test を管理宣言欠落として `chain_integrity` の `MISMATCH`（診断 `MISSING`）とし、対応VOを推測で寄与関係へ関連付けない。

source declarationを構文上完全なTest Entityへ正規化できる条件は、**adapter 中立 core が要求する必須 metadata**（構文上有効な Test ID・**1 件以上の `covers`**・`intent`）に加え、**当該 adapter が必須とする追加 metadata** を Test Entity として具体化できることをいう。`rust-cargo` は検証対象を Source Target として実現する形態であり、追加必須 metadata として `targets ≥ 1` を要求する（§4.1・§4.2・§5.5、基本仕様 §9.2）。これらの必須 metadata（core の `id` / `covers ≥ 1` / `intent`、および `rust-cargo` の `targets ≥ 1`）を欠く場合は E-SCAN-007 とし、`ManagedTestLink::Missing`（`chain_integrity` の `MISMATCH`、診断 `MISSING`）とする。E-SCAN-007 は adapter が報告する構文・必須 metadata 診断であり、`targets ≥ 1` は `rust-cargo` の必須 metadata としてこの経路で検出される（core 中立の必須リンクへは加えない。§11.1.1）。`covers` 件数の可変制約（旧 role/anchor 由来）は設けず、すべての管理対象 Test に `covers ≥ 1` を一律要求する（基本仕様 §12）。

構文上完全なTest Entityへ正規化できるが、`covers`のVO IDをcore storeで解決できない場合、そのentityと`ManagedTestLink::One(id)`を保持する。E-SCAN-003と`chain_integrity = MISMATCH`はcoreの参照整合性検査で生成する。E-SCAN-003が発生しても対応するTest Entityと`ManagedTestLink::One`を除去しない。

VO参照の解決とTest IDの大局的一意性はadapterではなくcoreが検査する。adapterはTest構文の違反（重複不可キーの重複、未知キー、必須キーの欠落）を §5.4 の E-SCAN-005 / E-SCAN-006 / E-SCAN-007 で報告し、VO 解決・ID 一意性・target 解決は core が §5 の参照整合性検査で判定する。

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
   （存在理由分類の実効値確定は行わない。covers ≥ 1 を一律要求する）

5. 決定論的な統合
   adapter ID、project-relative path、opaque locator、Test IDの順に正規化する
   adapter間を含むTest ID・SRC ID衝突と不正な複数対応を検査する

6. .verify/ 読み込み
   vtest-storeが全レコード（document / VO / Relation / 判断 / 承認 / Evidence）を
   読み込み、スキーマ検証する

7. 参照整合性検査
   coversのVO ID、targetsのTarget Reference / SRC ID、Relation、
   VO parent、VO derives_from（document）、document derives_from を解決する

8. グラフ構築と整合性検査（§5.3、§5.4、§5.6）
```

adapterが解析不能または不完全なbatchを返した場合、coreは対応する検証を`UNKNOWN`とし、Test 0件の完全なdiscoveryとして扱わない。

### 5.2 エンティティモデル（vtest-model）

```rust
pub struct TestEntity {
    pub id: TestId,
    pub covers: Vec<VoId>,          // 1件以上（covers ≥ 1 一律。§4.4）
    pub targets: Vec<TargetRef>,    // 各要素はadapter付きopaque locatorまたはSrcId。件数はadapterが定める（rust-cargoはtargets≥1を必須。§4.1・§4.4）。coreはtargets≥1を中立必須にせず型としては空を許容する
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

// 検証状態は5つのみ（基本仕様 §4.1、要件定義 §5.1）
pub enum CheckValue {
    Pass,
    Fail,
    Mismatch,
    NoEvidence,
    Unknown,
}

// 診断ラベルは検証状態と別軸（基本仕様 §4.2、要件定義 §5.2）
pub enum DiagnosticLabel {
    Missing,
    NotExecuted,
    NotChecked,
    Stale,
}

// 検査は4本のみ（基本仕様 §5、要件定義 §3.3・§4）
pub enum CheckItem {
    ChainIntegrity,
    OrphanDetection,
    TargetBinding,
    OraclePresence,
}
```

`CheckValue` は状態のみを表し、原因説明は `DiagnosticLabel` として併記する。集約の代表値選択に診断ラベルを用いない（§11.3、基本仕様 §22.2）。`Missing` / `NotChecked` / `NotExecuted` / `Stale` を検証状態の variant として持たせない（旧 8 値モデルの排除）。

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
  content / subject hash、Evidence上のtarget identityが分裂しない。
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

`SourceDiscoveryAdapter`はadapterがTestとして認識した全Discovered Test draftを返す。`ManagedTestDraftLink::One`は、構文上有効なTest IDと必須metadata（core 中立の `covers ≥ 1` / `intent`、および当該 adapter が必須とする追加 metadata。`rust-cargo` では `targets ≥ 1`。§4.1・§4.4）をdraftとして具体化できる場合に設定する。
VO参照の解決とTest IDの大局的一意性はadapterではなくcoreが検査する。したがって、解決不能な`covers`を持つdraftもcore materialization後のmanaged entity集合に保持され、対応するobservationは`ManagedTestLink::One(id)`を持つ。
`ManagedTestDraftLink::Missing`は管理宣言の欠落または必須metadataの欠落、`Multiple`は同一Test constructから複数draftが生じる状態を表す。core materialization後の対応する状態が`ManagedTestLink`となる。

adapter capabilityは `SourceDiscoveryAdapter`、`TestWireCodec`、`StaticAnalysisAdapter`、
`StructuredTestAdapter`、`TestRunnerAdapter`、`CoverageAdapter` に分割する。
各adapterは一意なID、languages、capabilities、config namespaceを宣言し、registryは
宣言と実装の不一致および重複IDを拒否する。明示操作に必須のcapabilityがない場合は
E-ADAPTER-004で操作を中止する。検証集約では、static解析 / coverage欠落は
`NO_EVIDENCE`（診断 `NOT_CHECKED`）、runner欠落は `NO_EVIDENCE`（診断 `NOT_EXECUTED`）、解析限界は `UNKNOWN` とする。

`TestRunnerAdapter`は、coreがfreshness subjectを所有できるよう次のhash未計算DTOを返す。`CanonicalProjection`は型tag、null、list順序、map key順序を保持する言語非依存値とする。

```rust
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

`StaticAnalysisAdapter` は正典レコードを持たない再計算派生であり（§7.1）、判定は現在の source / target / config から都度計算する。coreは freshness subject を静的解析用に永続化せず、検証のたびに現在入力で再導出する。

Test Runnerはcommand起動前に`ExecutionStateDraft`を構築し、実際に使用するinvocation / toolchain / configと一致するDTOだけを実行結果へ添付する。`invocation`はselector、working root、runner option等をmachine非依存に正規化し、絶対pathを含む表示用commandとは分離する。coreは実行前後でExecution State subject全体が変化していないことを確認してからEvidenceを記録する。変化した場合はE-EXEC-004としてEvidenceを生成しない。有効性再評価では同じschemaを持つ現在DTOを再構築し、保存hashと比較する。

Structured Test capabilityを宣言するadapterは、処理可能なbuilt-in Form `kind`集合と、adapter fieldを持たないForm Schemaを判定するcompatibility matcherを宣言する。Form `kind`はbuilt-inと`.verify/forms/`を統合したrepository全体で一意であり、Form Schemaの`adapter` field、registryのowner、Structured Test capabilityが同じadapter IDを示す場合だけ`kind → adapter`を確定する。重複kindまたは対応の不一致はE-ADAPTER-001、未知kindはE-OP-001とし、coreが名前からRust adapterを推測しない。`adapter` fieldを欠く読取り互換Formは、登録済みStructured Test adapterのbuilt-in kind宣言またはcompatibility matcherのうちちょうど1件だけがschemaを受理する場合に限ってin-memoryでownerを補える。0件または複数件なら操作を拒否し、ファイルを書き換えない。matcherはsource bytes、schema field / validator集合等から決定論的に判定し、form kindの文字列だけを理由に汎用fallbackしてはならない。

document / VO / Relation / 判断記録 / 承認記録 / Evidence も §3 のスキーマに対応する struct を定義する。

### 5.3 検証グラフ

インメモリのグラフを構築する。

```text
ノード：DOC, VO, TEST, SRC（ロケータ単位）
エッジ：
  DOC  → DOC    (derives_from)          ※document レコード由来
  VO   → DOC    (derives_from)          ※VO レコード由来、1:N（1件以上）
  VO   → VO     (parent)
  TEST → VO     (covers)                ※adapter所有のTest metadata宣言由来
  TEST → SRC    (targets)               ※検証対象を Source Target として実現する形態、1:N（rust-cargo では targets ≥ 1。§4.1）
  外部 Relation (rel/ 由来)
逆引きインデックス：VO → Tests、SRC → Tests、DOC → VOs、DOC → DOCs（下流）
```

旧モデルの SPEC / REQ ノードと REQ→SPEC / VO→REQ エッジは持たない。上流文書はすべて DOC ノードとし、文書間・VO→文書は `derives_from` の一種で表現する（基本仕様 §3.1、§3.2、§19）。関係型（`derives_from` / `covers` / `targets` / 外部 Relation）は横断トレース可能とするが、単一へ潰さず、また意味論的に増殖もさせない。

### 5.4 整合性診断

| コード | 種別 | 内容 |
|---|---|---|
| E-SCAN-001 | error | adapterのsource構文解析失敗（DiscoveryBatchは`Incomplete`） |
| E-SCAN-002 | error | Test ID 重複（identity collision） |
| E-SCAN-003 | error | `covers` の参照先 VO が存在しない（dangling reference） |
| E-SCAN-004 | error | `target` のロケータ／SRC ID を解決できない |
| E-SCAN-005 | error | adapter所有の宣言で重複不可fieldが重複、または綴りの異なる複数の`target`宣言が同一canonical Source Targetへ解決 |
| E-SCAN-006 | error | Test constructのadapter所有の宣言に未知fieldが存在（非Test construct表面はW-SCAN-105） |
| E-SCAN-007 | error | 必須metadata（core 中立: id / covers ≥ 1 / intent、および当該 adapter が必須とする追加 metadata。`rust-cargo` では targets ≥ 1）の欠落 |
| E-SCAN-008 | error | VO の parent 不在または循環 |
| E-SCAN-009 | error | Relation の from / to が不在 |
| E-SCAN-010 | error | レコードのid / ファイル名 / schema不一致、または互換正規化後のlogical record ID重複 |
| E-SCAN-011 | error | 恒久SRC IDが複数adapterまたは複数Source Targetで衝突 |
| E-SCAN-012 | error | VO の `derives_from` が存在しない document を参照、または document の `derives_from` が存在しない document を参照（文書鎖のリンク切れ） |
| E-SCAN-016 | error | 根に指定されない孤児 document（親 document を持たず `doc.roots` にも列挙されない。§5.6） |
| W-SCAN-101 | warning | adapterが発見したが管理宣言に対応しないTest construct（unregistered test） |
| W-SCAN-102 | warning | どの VO からも参照されず、Test も参照しない孤立 VO |
| W-SCAN-103 | warning | `covers` を持つが対応 VO が leaf でない（中間 VO 直接参照。許容するが警告） |
| W-SCAN-105 | warning | Test constructとして解析されない関数itemのdoc comment内の`@vtest.`行に認識されないキーが存在（§4.2。打鍵ミス検出。`src-id`の重複はE-SCAN-005） |
| W-STORE-001 | warning | VO recordに非正典の読取り互換field `status`が存在（値は無視し承認から導出） |
| W-STORE-002 | warning | Approvalが現在の上流依存closureを欠くか一致せず、承認として無効 |

error は該当エンティティに関わる検査を非 `PASS` にする。
warningは診断severityだけでは検証値を変更しないが、レポートに常に表示する。

各 error の検証状態への写像は次のとおり（診断ラベルは併記）。

- 管理宣言の欠落・E-SCAN-007（必須metadata欠落）が示す`ManagedTestLink::Missing` → `chain_integrity = MISMATCH`（診断 `MISSING`）。
- `ManagedTestLink::Multiple`、E-SCAN-002（Test ID衝突）、E-SCAN-003（解決不能なVO参照） → `chain_integrity = MISMATCH`。E-SCAN-003が発生しても対応するTest Entityと`ManagedTestLink::One`を除去しない。
- E-SCAN-008（VO parent 不在・循環）、E-SCAN-009（Relation dangling）、E-SCAN-012（文書鎖・VO derives_from のリンク切れ） → `chain_integrity = MISMATCH`。
- E-SCAN-016（孤児 document） → `orphan_detection = MISMATCH`（§5.6）。
- E-SCAN-011があるSRC ID参照は曖昧なため、関係するtarget解決を`MISMATCH`とし、いずれのSource Targetも選択しない。候補の1件を解決結果としてEvidence・検証へ永続化しない（§6.1）。衝突する恒久SRC IDを宣言した各Source Target自体は、canonical locatorで独立に具体化されたまま保持する。

### 5.5 `rust-cargo` SourceDiscoveryAdapter

`rust-cargo` adapterは次の処理で§5.1の`DiscoveryBatch`を構築する。`vtest-scan`はこれらのRust固有処理を実行しない。**当該 adapter は検証対象を Source Target として実現する形態であり、各管理対象 Test に 1 件以上の Source Target（`targets ≥ 1`）を必須とする。この必須要件は adapter 層に属し（core 中立の `chain_integrity` 必須リンクではない。§4.1・§11.1.1）、欠落は E-SCAN-007 として報告する（§4.4・§5.4）。したがって `rust-cargo` の Test は従来どおり Source Target 宣言を要し、挙動・E コード・fixture は本改訂で実効的に変わらない。**

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
   doc属性（#[doc = "..."]）を§4.2の文法でparseする（id / covers / target / intent /
   input / expect / kind / case / related）

6. Source Target抽出
   すべてのfn / impl fnをSRC候補として索引化し、
   §4.3のlocator解決・逆引き・@vtest.src-id認識（非Test constructの宣言に限る。§4.2）に使用する。
   このpassで非Test constructのdoc comment内の`@vtest.`行を検査し、
   認識されないキーからW-SCAN-105を、`src-id`の重複からE-SCAN-005を生成する（§4.2）

7. draft生成
   全Discovered Test draft、ManagedTestDraftLink、SourceTargetDraft、Source Location、
   construct / metadata source rangeとbytes、logical metadata、宣言された恒久SRC ID、
   ExecutionDescriptor、診断をhash未計算のDiscoveryBatchに格納する
```

### 5.6 文書層 orphan_detection

`orphan_detection` は文書層の孤児検出であり、親（上流 document）を持たない `document` ノードが存在するかを問う（基本仕様 §5.2、要件定義 §4.2）。

- **根の指定**：`config.yaml` の `doc.roots`（§2.2）に列挙された DOC ID を根として扱い、`orphan_detection` の対象外とする。根指定は `.verify/` 設定として保持する（基本仕様 §5.2）。根指定の追加・削除は `vtest doc` コマンドの引数で管理し `doc.roots` へ反映する（基本仕様 §26.1 の `vtest doc` 責務「derives_from・根指定を含む」。引数 schema は別紙A）。
- **判定**：`derives_from` が空、かつ他のどの document からも `derives_from` で参照されない document のうち、`doc.roots` に列挙されないものを孤児とし、E-SCAN-016（`orphan_detection = MISMATCH`）とする。
- **対象は文書層のみ**。実装レイヤーの孤児検出（宣言されていない実装の検出）は行わない（要件定義 R-2、基本仕様 §29 OOS-005）。旧モデルの W-SCAN-102（孤立 VO）は VO 層の警告であり、文書層 `orphan_detection` とは別物として存置する。
- 根に指定された document が存在しない DOC ID を参照する場合は、config invariant 違反として E-CONFIG-001 とする。

---

## 6. Target Reference解決

### 6.1 adapter-neutral解決contract

coreは`TargetRef::Locator.adapter`をregistryで解決し、opaque locatorの解釈を該当する`SourceDiscoveryAdapter`へ委譲する。adapterは正規化されたTarget Reference、Source Location、source range、content bytes、解決status、候補を返す。
coreは返却されたadapter IDとTarget Referenceの一致、source rangeの範囲、current bytesとの一致を検証し、§1.3のSource Target hashを計算するが、opaque locatorの内部構文は解釈しない。解決が0件または複数候補で一意に定まらない場合はE-SCAN-004とし、推測で候補を選択しない。

SRC ID参照はcoreが統合済みSRC索引で一意性を検査し、対応するadapterのSource Locationとsource rangeを使用する。SRC ID参照は当該恒久SRC IDを宣言したSource Targetのcanonical locatorへ解決し、同じSource Targetへのlocator参照と**同一のcanonical Source Target・同一のSource Target hash**へ到達する。解決結果をlocator版とSrcId版の別entityへ分岐させない。恒久SRC IDが複数adapterまたは複数Source Targetで衝突する場合はE-SCAN-011とし、いずれのSource Targetも選択しない。

解決結果は「解決済み」「対象なし」「曖昧」の3状態を区別し、曖昧はfail-closedな終端状態とする。曖昧な解決から代表候補を選ばず、解決済みのcanonical Source Targetを要求する後段（静的解析、Evidence、`target_coverage`、鮮度判定）へ候補を1件も引き渡さない。候補は§6.3の診断表示にだけ用い、表示できることを選択の根拠にしない。
この禁止はTarget Referenceの**解決**に関するものであり、Source Targetの**具体化**を止めるものではない。各Source Targetは自身のcanonical locatorで独立に具体化され、恒久SRC IDが衝突していても`SourceTargetDraft`ごとに1件のSource Targetとして成立する。衝突が壊すのは当該恒久SRC IDによる**参照**の一意性だけである。
この解決はcoreの単一経路が所有し、静的解析、実行、Evidence writer、検証集約はいずれもその結果を消費する。各subsystemが独自にcandidate列を走査して1件を選ぶ経路を持ってはならない。E-SCAN-004またはE-SCAN-011で解決できなかったtargetを、後段が任意の候補で埋めて記録・永続化することを禁ずる。

#### 6.1.1 target identityの一方向確定

Source Target identityは次の一方向でだけ確定する。

```text
TestEntity.targets  = 宣言されたTargetRef（Locator / SrcId）
        ↓ resolve（§6.1）
Canonical Source Target = canonical Locator
        ↓
Evidence / target_coverage / 検証 = canonical Locatorをidentityとして使用
```

Evidence（§3.6、§9.4）、`target_coverage`（§10.2）、および鮮度判定（§11.2）は、解決後のcanonical Locatorをtarget identityとして記録・比較する。参照側Testが宣言した`TargetRef`の綴り（SRC ID参照を含む）をこれらのidentityとして保存してはならない。
Testがどう宣言したか（同じSource Targetに対するLocator参照からSRC ID参照への書き換え等）の変更は、`targets`をcanonical metadataとして束縛する§1.3のTest subject hashが捕捉する。したがってEvidence側で宣言表現を保持する必要はなく、保持すれば同一Source Targetが参照方法ごとに別identityへ分裂する。
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

Structured Operationの入力検証（別紙A §14、§15）で解決に失敗した場合、coreはadapterが返した候補を共通envelopeで表示する。`rust-cargo` adapterは次の順で候補を構築する。

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

## 7. Static Analysis orchestrationと`rust-cargo`ルール

静的解析は `oracle_presence`（照合装置の存在）と、`target_binding` の静的到達証明（DA-002）へ証拠を供給する。決定論的解析結果は正典レコードを持たず、検証のたびに現在の source / config から再計算する派生情報である（基本仕様 P-003）。`vtest audit static` は要求時に解析を起動し、結果を stdout と `cache/` へ出力する（基本仕様 §26.1）。判断記録（§8）とは別機構であり、外部判断の記録には転用しない。

### 7.1 判定の原則

各ルールは `FAIL` / `UNKNOWN` / `PASS(違反なし)` のいずれかを返す。
**決定論的に確定できる違反のみ FAIL とする。**
解析の限界で確定できない場合は FAIL ではなく UNKNOWN とする。`UNKNOWN` は意味判定できる者への判断記録エスカレーション（§8、基本仕様 §11）の領分である（ただし DA-002 の target 到達 UNKNOWN は §7.3 の runtime 到達証明で解決し、判断記録へは委ねない）。

`oracle_presence` は DA-001 / DA-003 / DA-004 / DA-005 / DA-006 の合成とする。全ルールが違反なしなら `PASS`、1つでも `FAIL` があれば `FAIL`、`FAIL` がなく `UNKNOWN` があれば `UNKNOWN` とする（基本仕様 §5.4、要件定義 §4.4）。`oracle_presence` に動的な昇格経路は無い。静的解析は不成立の証明であり、証明の失敗は `UNKNOWN` であって、runtime 証拠で `PASS` へ昇格しない。

`target_binding` の静的到達証明は DA-002 が担う（§7.3）。DA-002 の target 別 verdict の `UNKNOWN` は「静的解析の到達判定境界の外にあり、静的には到達を証明できない」ことだけを表し、到達しないことを意味しない。この到達 `UNKNOWN` は §7.3 に従い当該 target の runtime 計測（§10）が実行を証明した場合に限り充足される（要件定義 §4.3 の2証拠源モデルは `target_binding` に固有）。

`vtest-audit`は`TestEntity.execution.adapter`をregistryで解決し、Test、全Target Reference、各source range、content hash、および選択adapterの現在configを`StaticAnalysisAdapter`へ渡す。adapterはrule ID、verdict、根拠span、解析限界を返す。target-scopedなDA-002 / DA-003については、宣言targetごとのverdictと根拠spanを（規則単位のverdictへ畳み込む前の形で）返し、その集合を全宣言targetと過不足なく1対1に対応させる。coreはadapter ID、source location・現在bytesとの対応、決定論的encodingを検証し、上記規則で集約する。adapter固有のASTやassertion構文をcoreは解釈しない。

Static Analysis capabilityがない場合は`NO_EVIDENCE`（診断 `NOT_CHECKED`）、adapterが不完全、解析限界、または解析入力集合の不完全性を報告した場合は`UNKNOWN`とし、違反なしと推測しない。

### 7.2 `rust-cargo` ルール一覧

`rust-cargo`の**assert相当の構文**は次のとおり定義し、DA-001〜DA-006で共通に用いる。

- `assert!` / `assert_eq!` / `assert_ne!` / `panic!`を含む標準マクロ、および`rust-cargo` configの`assertion_macros`に列挙されたマクロ
- `#[should_panic]`属性
- `.unwrap()` / `.expect(..)` / `?`演算子（Result / Optionの成立検証として扱う）
- Test関数が`Result`を返し`Err`を返しうる構造

| ルール | 供給先検査 | 内容 | FAIL 条件 | UNKNOWN へ退避する例 |
|---|---|---|---|---|
| DA-001 定数アサーション | oracle_presence | 引数がすべてリテラル・定数式の assert | 関数内の assert 相当がすべて定数アサーション | 定数性を確定できない式 |
| DA-002 対象未呼出 | target_binding | 宣言された target シンボルを呼んでいない | 関数本体および同一ファイル内の呼出先 helper（1段）を探索して呼出が存在しない、かつ他ファイルへの呼出も存在しない | 他ファイル・他クレートの関数呼出があり、間接呼出の可能性を排除できない |
| DA-003 結果未検証 | oracle_presence | target を呼ぶが、その結果を assert 相当で一切検証しない | target 呼出結果（戻り値、および結果から派生した束縛）が assert 相当に到達しない、かつ `#[should_panic]` がない | 結果が可変参照・グローバル状態経由で検証される可能性がある場合 |
| DA-004 自己比較 | oracle_presence | `assert_eq!(a, b)` で a と b がトークン列として同一 | 該当 assert が存在する | なし（構文的に確定） |
| DA-005 空テスト | oracle_presence | 関数本体に文が存在しない | 該当 | なし |
| DA-006 検証構文なし | oracle_presence | 関数内に assert 相当が1つも存在しない | 該当 | なし |
| W-DA-101 ignored | — | `#[ignore]` 属性 | （FAILにしない。警告のみ。実行されなければ `target_binding` が診断 NOT_EXECUTED になる） | |

DA-002 / DA-003 のデータフロー解析は関数内のローカル束縛の追跡（let 束縛、メソッドチェーン、フィールドアクセス）までとし、クロージャ内・マクロ展開内は UNKNOWN とする。
複数target TestではDA-002 / DA-003を各targetへ個別適用する。target別結果に1件でもFAILがあればrule結果をFAIL、FAILがなく1件でもUNKNOWNがあればUNKNOWN、全targetが違反なしの場合だけPASSとする。
静的解析は再計算派生であるため、これらの target 別 verdict と規則単位 verdict は検証のたびに現在 source から計算し、正典レコードへ永続化しない（§7.1）。

**呼出そのものが Test 本体に現れない場合の DA-003。** 宣言 target への呼出が Test 本体に静的に現れない場合（subprocess を起動して別プロセスで target を実行する等、target 呼出が source 内に存在しない）、**DA-003 の当該target別verdictを UNKNOWN** とする。呼出結果を観測できないことを「違反なし（空虚PASS）」とも「結果未到達（空虚FAIL）」とも判定しない。この場合 DA-002 も同 target で UNKNOWN であり、DA-002 が §7.3 の runtime 証明で救済されても DA-003 は UNKNOWN のまま `oracle_presence` へ寄与するため、**呼出が本体に現れない Test（典型的な subprocess E2E）は oracle_presence = PASS に到達しない**。
一方、target 呼出は Test 本体に現れるが DA-002 が UNKNOWN になる場合（他ファイル・他クレートへの直接呼出で間接呼出の可能性を排除できない等）、その呼出結果が Test 本体内で assert 相当へ到達すれば DA-003 = PASS になりうる。この target は DA-002 を runtime で救済すれば `target_binding` = PASS に到達しうる（runtime 救済で実益が出る型）。クロージャ・マクロ展開の内側での到達は §7.2 の一般則どおり DA-002 / DA-003 とも UNKNOWN とする。
ルールごとの判定結果と根拠（該当スパン）は `vtest audit static` の出力および `cache/` の派生結果として提示する。

### 7.3 target 到達の静的証明と runtime 証明の関係（target_binding）

`target_binding` は「その Test が検証対象とする振る舞いが実際に生じ、その振る舞いを反映した観測が得られたか」を問う（基本仕様 §5.3、要件定義 §4.3）。この検査は静的解析（DA-002）と動的計測（§10 coverage）の2証拠源を持ち、静的に確定できなければ `UNKNOWN` とし動的証拠で昇格できる。

DA-002 は §7.2 の解析境界（関数本体および同一ファイル内 helper 1段。クロージャ内・マクロ展開内・他ファイル・他クレートへの呼出は §7.1 / §7.2 に従い UNKNOWN）で行う**静的な target 到達証明**である。Test が target を静的解析の追えない**実行境界**を越えて到達させる形態はいずれも DA-002 の UNKNOWN として現れる。これには次が含まれ、Test の kind（unit / integration）とは独立に、**execution topology** によって決まる。

- 他ファイル・他クレートへの呼出を介した間接到達
- クロージャ・マクロ展開内での到達
- 生成した別スレッド（in-process, thread boundary）での到達
- 別プロセス（subprocess を起動し、そのプロセス内で target を実行する process boundary）での到達

**到達要件は、target ごとに、次のいずれかで充足される。**

1. **静的証明**: 当該 target の DA-002 verdict = PASS（§7.2 の解析境界内で呼出を確認）。
2. **runtime 証明**: §11.2 が選択した最新 Evidence が §6 のハッシュ束縛（鮮度）を満たすとき、その Evidence の §10.2 target 別 `target_coverage` result = PASS（`checked: true` かつ実行 count > 0）。

DA-002 verdict が UNKNOWN（静的に証明できない）である target は、runtime 証明が成立するときに限り到達要件を満たす。複数 target Test では target ごとに適用し、Test の `target_binding` 到達は**全宣言 target の到達要件が充足された場合にのみ**成立する。

- static 側は §7.2 の DA-002 verdict を target ごとに用いる。DA-002 verdict = FAIL（解析境界内で到達を静的に否定）は runtime 証明で覆さない。
- runtime 側は §11.2 が選択した最新 Evidence だけを用いる。最新 Evidence が鮮度を満たさなければ runtime 証明は成立せず、古い Evidence へフォールバックしない。これにより同一検証内で計測が §11.2 で STALE の一方 `target_binding` が別 Evidence で PASS になる履歴不一致を防ぐ。

`target_binding` 項目値は検証時に算出する。static 側は §7.2 の DA-002 を再計算し、runtime 側は §11.2 選択 Evidence を用いて、target ごとに実効到達状態を定める。

- **静的到達**: DA-002 verdict = PASS。
- **runtime 到達**: DA-002 verdict = UNKNOWN かつ runtime 証明成立。
- **未充足**: DA-002 verdict = FAIL、または UNKNOWN で runtime 証明が成立しない。

`target_binding` は、Evidence の `result: FAIL`（テストランナーが失敗を報告）なら `FAIL`、そうでなく全宣言 target の到達が静的到達または runtime 到達で充足されれば `PASS` とする。到達未充足の target があれば §11.2 の写像に従い非 `PASS`（動的計測 count 0 は `FAIL`/診断 NOT_EXECUTED、計測不能・未計測は `NO_EVIDENCE`、解析限界は `UNKNOWN`）。

この関係は fail-closed を保つ。

- runtime 証明は当該 target の `target_coverage` = PASS のときだけ成立する。`target_coverage` が FAIL（count 0）・UNKNOWN（関数不見当）・NOT_CHECKED（coverage 利用不能、未計測、`--fast`）のときは到達要件を満たさず、当該 target は上記の**未充足**となり、`target_binding` を非 `PASS` にする。
- **本節の到達要件は検証対象を Source Target として実現する形態に限定する**（`rust-cargo`。基本仕様 §5.3「実装 construct（Source Target）を検証対象とする実行形態では…」）。この形態で、宣言 target をどの topology でも実行しない Test（構造・契約のみを assert する Test）は静的にも runtime にも到達を確立できず、到達要件は未充足のままとなる。検証対象を Source Target として宣言しない他の実行形態（外部契約・境界上の振る舞い）の確認方法は、特定形態を他形態へ一律要求せず下位仕様・後続版へ委譲するため（要件定義 §4.3、基本仕様 §5.3・§8.3）、本節の target 実行到達規則を普遍規則として適用しない。なお v0.1 の唯一の adapter `rust-cargo` では検証対象を Source Target として宣言しない Test は E-SCAN-007（`targets ≥ 1` 欠落）として `target_binding` 評価の手前で `chain_integrity` の `MISMATCH` になり、target を持たない Test が本節の合成へ到達しない。

**DA-003 はこの to-runtime join に含めない。** DA-003 は `oracle_presence`（照合装置の存在）へ寄与する static data-flow 判定であり（§7.2）、target の「結果検証」を問う。runtime coverage は target の「実行」を証明するが「結果検証」を証明しないため、coverage は DA-003 を代替せず、DA-003 は §7.2 の意味論のまま維持する。典型的な subprocess E2E（target の戻り値 → 子プロセスの stdout / exit code → 親プロセスの assert）では、この data-flow は static analyzer から追えないため DA-003 は UNKNOWN のまま残りやすい。**本節は process boundary によって DA-002 到達が恒久 UNKNOWN になる問題だけを解消するものであり、boundary test を完全に oracle_presence PASS 可能にするものではない。**

---

## 8. 判断記録プロトコル

本システムは、宣言されていない義務・網羅漏れ・宣言と実装の意味のずれを、自ら発見・裁定しない（基本仕様 §11、要件定義 §12）。機械が決定論で確定できない疑義は `UNKNOWN` として外部（人間または判断可能 Agent）へ引き渡し、その判断を判断記録（§3.4）として追跡する。**このプロトコルは検証状態のゲートではない。判断記録の受理は当該対象の検証状態を昇格させない**（基本仕様 §11.3、要件定義 §12）。

### 8.1 バンドル生成

`vtest audit bundle` は判断対象ごとに、判断に必要な情報を JSON として `cache/bundles/<ULID>.json` へ出力する。
バンドルは派生情報であり Git 管理しない。
提出結果の検証に必要な情報（対象の内容ハッシュ）は判断記録へ複製されるため、バンドル自体の永続化は不要である。

バンドルには基本仕様 §11.3 が定める判断対象の情報一式を含める。

- 対象 VO（`--vo` / `--test` から導出した covers 先 VO レコードと claim）
- Test Intent（`--test` の場合の対象 Test の intent・input・expect）
- テストコード（Test construct source 全文と metadata 宣言）
- 対象実装（全宣言 target の implementation construct source 全文）
- 関連テスト（related / 同一 VO を covers する他 Test の id と intent）
- 既知 partition（対象 VO の dimensions・coverage_policy・representative_cases）
- 過去の判断（同一対象への有効・無効な過去判断記録の要約）
- 対象の内容ハッシュとリビジョン（Test subject / target subject / VO subject の現在 hash、revision）

`impl-consistency` 型の判断（対象実装が宣言と一致するかの意味判定）のように上流 document を要する対象では、対象 VO から §3.5 と同じ上流依存規則で導出する document subject 完全集合と source 全文を加える。宣言 target のいずれか、または上流 document のいずれかを解決できない場合はバンドルを生成せず、候補のいずれも選択しない（§6.1）。解決失敗の種別は対象不在（E-SCAN-004、document 不在）を `MISMATCH`（診断 `MISSING`）、恒久 SRC ID 衝突による曖昧（E-SCAN-011）を `MISMATCH` として当該対象の検証結果へ保持する。

### 8.2 バンドル JSON スキーマ（例）

```json
{
  "bundle_id": "01J8XVYY...",
  "generated_at": "2026-08-08T00:00:00Z",
  "revision": { "commit": "abc123...", "dirty": false },
  "subject": "TEST-PARSER-044",
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
      "derives_from": ["DOC-BASIC-001"], "content_hash": "sha256:..." }
  ],
  "targets": [
    { "target": "rust-cargo::src/parser.rs::Parser::parse",
      "source": "pub fn parse(...) { ... }", "content_hash": "sha256:..." }
  ],
  "related_tests": [ { "id": "TEST-PARSER-003", "intent": "..." } ],
  "static_analysis": { "oracle_presence": "PASS", "rules": [] },
  "prior_decisions": [ { "id": "01J...", "decision": "accepted", "decided_at": "...", "valid": false } ]
}
```

### 8.3 提出スキーマ

`vtest audit submit --file result.json` で提出する。判断は少なくとも actor / subject / decision を含み、理由・根拠は任意（optional）とする（基本仕様 §11.3、要件定義 §12）。

```json
{
  "bundle_id": "01J8XVYY...",
  "subject": "TEST-PARSER-044",
  "decision": "accepted",
  "reason": [
    {
      "claim": "テストは不正 continuation byte 入力に対し InvalidUtf8 の返却を検証している",
      "basis": [
        { "kind": "test-code", "ref": "rust-cargo::tests/parser_test.rs::rejects_invalid_utf8" },
        { "kind": "vo", "ref": "VO-PARSER-UTF8-003" }
      ]
    }
  ],
  "exclusions": [
    { "item": "overlong encoding の検証", "basis": "DOC-BASIC-001 により対象外" }
  ],
  "actor": { "kind": "agent", "id": "judge-agent-01", "model": "claude-fable-5" }
}
```

- `decision` の値集合はツールが受理する判断値（`accepted` / `rejected` / `deferred` 等）とし、その妥当性を §8.4 で検証する。旧モデルの `verdict → CheckValue` 写像（`PASS`/`FAIL`/`COMPLETE`/`INCOMPLETE` を検証状態へ変換する経路）は撤去する。判断記録は検証状態を変更しない（§8 冒頭）。
- `reason` / `exclusions` は任意である。`basis.kind` は `document` / `vo` / `test-code` / `target-code` のいずれかとする。理由が空であることだけを根拠に判断を無効化しない（基本仕様 §11.3）。

### 8.4 提出の検証

`audit submit` は次を順に検証し、失敗した場合は §17 のエラーコードで拒否する。

```text
1. bundle_id のバンドルが cache に存在する      （E-AUDIT-001）
2. subject がバンドルと一致する                 （E-AUDIT-003）
3. バンドル記録時の各対象の内容ハッシュが、
   現在のハッシュと一致する（対象が変更されて
   いれば判断は無効）                            （E-AUDIT-002）
4. decision が受理する判断値である              （E-AUDIT-004）
```

受理された提出は判断記録（§3.4）として `.verify/decisions/` へ保存される。`subjects` に相当する対象集合はバンドル生成時の全対象の内容ハッシュを `subject_hash` と `dependencies` として記録し、依存 closure のハッシュに束縛する。**理由（`reason` / `exclusions`）の有無を提出の受理条件にしない**。旧モデルの reasons / claim / basis 必須検査（E-AUDIT-005）、decomposition-viewpoint 検査（E-AUDIT-006）、spec / req basis 検査（E-AUDIT-007）は撤去する。これらは要件定義 §12「理由が空であることだけを根拠に無効扱いしない」と矛盾するため、判断記録層では課さない。

### 8.5 有効性と再判断

判断記録の有効性は判定時に評価する。

```text
有効 = subject が一致し、subject_hash が現在の内容ハッシュと一致し、
       dependencies が現在の上流依存closureとentity・hashとも完全一致する
       （document は登録 content_hash と実ファイルの一致も要求。
        不一致の場合は当該 document を STALE とし、依存する判断記録も無効）
```

- 同一対象に有効な判断記録が複数あってよい（再判断・多重判断）。回数はツールとして制限しない（運用ポリシー）。
- 仕様・VO・Test 等が変更された場合、過去の判断を現在状態へそのまま流用してはならず、現在状態に対して通常の検証（§5 の 4 検査）を再実施する。その結果は `PASS` / `FAIL` / `MISMATCH` / `NO_EVIDENCE` / `UNKNOWN` のいずれにもなり得る。変更そのものが `UNKNOWN` を生成するのではない（基本仕様 §11.3、要件定義 §12）。
- 判断済みと承認済みは区別する（判断済み ≠ 承認済み）。判断は承認なしでも記録でき、正式採用は §3.5 の承認の別段階である。

### 8.6 参考プロンプト

判断エージェントのプロンプト・スキル構成はツールの責務外だが、参考として骨子を示す。

```text
あなたは検証対象の意味判定者である。添付のバンドルについて、
以下だけを判定せよ。修正方針の提案はしない。

判定事項：
テストコードは、VO の claim と Test Intent が宣言する
振る舞いを実際に検証しているか。

判定は accepted / rejected / deferred のいずれかとし、
判定ごとに claim（何を確認したか）と basis（根拠にした
バンドル内の情報への参照）を任意で列挙してよい。
```

判断の受理は検証状態を昇格させない。判断は `UNKNOWN` に対する外部判断の追跡であり、検査ゲートではない（§8 冒頭、基本仕様 §11.3）。

---

## 9. テスト実行設計

### 9.1 実行対象の解決

`vtest run` は `--test` / `--vo` / `--all` で対象を受け取り、検証グラフから Test 集合へ展開する（VO 指定は部分木の covers を辿る）。旧モデルの `--req`（REQ 指定）は document 層の総称化により廃止し、document scope が必要な場合は VO 部分木経由で指定する。

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
検証集約の `target_binding` は `NO_EVIDENCE`（診断 `NOT_EXECUTED`）とする。

`--exact` は後続の全フィルタへ適用されるフラグであり、各フィルタは完全一致で解釈される。

### 9.3 `rust-cargo` 結果のパース

stdout を次の規則でパースする（stable toolchain の標準出力形式のみに依存する）。

```text
running N tests            → 実行対象数の確認
test <selector> ... ok       → PASS
test <selector> ... FAILED   → FAIL
test <selector> ... ignored  → 実行されず（Evidence は記録しない。target_binding は診断 NOT_EXECUTED）
```

要求した各フィルタについて結果行が得られなかった場合、その Test の実行は失敗（E-EXEC-002）とし、Evidence を記録しない。
プロセス終了コードと結果行の集計が矛盾する場合も E-EXEC-003 とする。
stdout / stderr の全文は `cache/logs/<ULID>.log` へ保存し、Evidence の `log_ref` から参照する。

### 9.4 Evidence の記録

Test ごとに §3.6 のレコードを1件生成する。

- `revision`：実行直前に `git rev-parse HEAD` と `git status --porcelain` で取得。取得失敗時は `commit: null` とし、この Evidence は鮮度（§11.2）の revision 一致を満たさず `target_binding` の有効な `PASS` にならない。
- `hashes`：実行直前のdiscovery結果から、Test subject hashと、全宣言targetを§6.1で解決したcanonical Locator・implementation construct hash（§1.3）を宣言順で記録する。欠落・重複を許可しない。宣言された`TargetRef`の綴りではなく解決後のcanonical Locatorを記録する（§6.1.1）。
- **Evidence生成のprecondition**：全宣言targetがcanonical Source Targetへ一意に解決できることをEvidence生成の前提とする。1件でも「対象なし」または「曖昧」（E-SCAN-004 / E-SCAN-011）なら**Evidenceを生成しない**。部分的な`hashes.targets`を持つEvidenceを生成して後段で弾く方式は採らない。この場合 `target_binding` は `NO_EVIDENCE`（診断 `NOT_EXECUTED`）のままとし、target解決の診断で非 `PASS` を示す。
- `execution_state`：実行直前にrunner adapterが返すsnapshot schema、runner / toolchain / 実行影響config、およびrepository / local dependency入力manifestをcoreが検証し、§1.3のExecution State subject hashとして記録する。完全性を保証できない場合は`complete: false`とし、後続の鮮度を `PASS` にしない。
- ビルド失敗（コンパイルエラー）の場合、対象 Test 群の Evidence は記録せず E-EXEC-001 を報告する。`target_binding` は `NO_EVIDENCE`（診断 `NOT_EXECUTED`）のままとなる。

---

## 10. `rust-cargo` Target Binding 動的計測

### 10.1 計測方式

`rust-cargo` CoverageAdapterは`cargo-llvm-cov`を使用する（adapter configの`run.coverage: llvm-cov`）。
起動時に `cargo llvm-cov --version` で利用可否を確認し、利用不能なら計測せず、Evidence の `target_coverage` を `checked: false`（検証時 `NO_EVIDENCE`、診断 `NOT_CHECKED`）とし診断 W-EXEC-101 を出す（`PASS` へ変換しない。基本仕様 §5.3）。

カバレッジを Test 単位で対象関数へ帰属させるため、計測時は Test を1件ずつ実行する。

Test が起動した subprocess・spawn した thread の実行を宣言 target へ帰属させられるかは `rust-cargo` CoverageAdapter の能力に属する（§10.2・§7.3）。subprocess 内の実行を計測するには起動される実行体も instrument 対象とし、子プロセスの profile を merge する必要がある。これを提供できない構成では境界越し target を UNKNOWN、計測不能なら `target_coverage.checked: false`（`NO_EVIDENCE`/`NOT_CHECKED`）とし、能力の有無で計測結果を捏造しない。この能力の実装可否が §7.3 の runtime 到達証明が subprocess E2E に及ぶかを左右するが、欠如時も fail-closed を保つ（DA-002 は UNKNOWN のまま）。

```text
cargo llvm-cov test -p <project> --lib --json
  --output-path cache/cov/<ULID>.json
  -- --exact <selector>
```

coverageは独立した `CoverageAdapter` capabilityとして扱う。提供されない場合は
`target_coverage.checked: false`（`NO_EVIDENCE`/`NOT_CHECKED`）、解析限界は `UNKNOWN` とし、測定済み `PASS` を推測しない。

### 10.2 判定

出力 JSON（llvm-cov export 形式）の `data[].functions[]` から、Testが宣言する各対象関数を検索する。

```text
一致条件：
  demangle 済み関数名の末尾が locator の item-path と一致し、
  かつ filenames のいずれかの末尾が locator の path と一致する

ジェネリック関数は複数インスタンスが現れるため、同じtargetに対応するcountを合算する。

target別判定：
  count > 0          → PASS
  count == 0         → FAIL（診断 NOT_EXECUTED。基本仕様 §4.3）
  関数が見つからない → UNKNOWN（インライン化・cfg 除外等の可能性）

Test単位集約：
  FAILが1件以上                    → FAIL
  FAILなし、UNKNOWNが1件以上        → UNKNOWN
  1件以上の全宣言targetがPASS       → PASS
```

各targetのcanonical Locator（§6.1.1）・result・countとTest単位集約結果をEvidenceの`target_coverage`へ記録する。
target別entryの欠落、重複、余分なentry、または解決後のcanonical Source Target集合との不一致を `PASS` として保存しない。この計測結果は §7.3 の target_binding runtime 証明の証拠源であり、独立の検査項目ではない。

Test が別プロセス（起動した subprocess 内）・別スレッド等の実行境界越しに target を到達させる場合も、判定は上記の実行 count に基づく。coverage provider は当該境界越しの実行を宣言 target へ帰属させなければならない（例：起動される実行体も計測対象としてinstrumentし、子プロセスの profile を merge する）。provider が境界越しの実行を帰属できない場合はその target を `UNKNOWN`（関数不見当扱い）とし、計測自体が不能なら `target_coverage.checked: false` とする。いずれも §7.3 の runtime 到達証明を成立させず、静的到達の UNKNOWN を `PASS` へ変換しない。この帰属可否は adapter の coverage capability に属し、能力の有無で計測結果を捏造しない。

### 10.3 実行モードの整理

`vtest run` は2モードを持つ。

- `--fast`：cargo test のみ。`target_coverage.checked: false` で記録し、検証時は `NO_EVIDENCE`（診断 `NOT_CHECKED`）。
- 既定（完全検証向け）：cargo-llvm-cov による Test 単位実行。実行時間と引き換えに `target_binding` の動的証拠を得る。

---

## 11. 鮮度検証と集約

### 11.1 検査の評価地点

4 検査（基本仕様 §5）は、次の地点で評価する。

| 検査 | 評価地点 | 評価方法 |
|---|---|---|
| `chain_integrity` | repository scan result / DOC / VO / TEST | 文書鎖（document derives_from・content_hash）、VO の derives_from（document 1件以上）、Test の管理宣言（Test ID・covers ≥ 1・その他の必須 metadata〔intent、および当該 adapter が必須とする追加 metadata。rust-cargo では targets ≥ 1〕）・covers 参照解決・Test ID 大局的一意性がすべて成立すれば `PASS`（§11.1.1） |
| `orphan_detection` | DOC | 親を持たず `doc.roots` にも列挙されない document が無ければ `PASS`、あれば `MISMATCH`（§5.6） |
| `target_binding` | TEST | §7.3 の合成。Evidence result FAIL は `FAIL`、全宣言 target の到達が静的到達または runtime 到達で充足されれば `PASS`。未充足は §11.2 の写像に従う |
| `oracle_presence` | TEST | §7.1 の合成（DA-001 / DA-003 / DA-004 / DA-005 / DA-006）。全 PASS で `PASS`、1つでも FAIL で `FAIL`、FAIL なく UNKNOWN で `UNKNOWN` |

完全検証の検査集合はこの4検査に固定し、設定で追加・削除できない（§2.2、基本仕様 §22.1）。旧モデルの12項目（`spec_coverage` / `vo_decomposition` / `vo_coverage` / `test_existence` / `static_audit` / `semantic_audit` / `impl_consistency` / `test_execution` / `runtime_result` / `target_execution` / `evidence_validity` / `test_traceability`）は検査として存在せず、次のとおり吸収・撤去した。

- `test_existence` / `test_traceability` → `chain_integrity` へ統合。
- `static_audit` → `oracle_presence`（DA-001/003/004/005/006）と `target_binding` の静的到達（DA-002）へ分割。
- `test_execution` / `target_execution` / `runtime_result` → `target_binding` の証拠（Evidence の存在・鮮度、`result`、`target_coverage`）へ吸収。
- `evidence_validity` → 独立検査を廃し、鮮度喪失を診断ラベル `STALE` として §11.2 で説明（基本仕様 §6）。
- `spec_coverage` / `vo_coverage` / `vo_decomposition` / `semantic_audit` / `impl_consistency` → 検査から除去し、網羅・意味の疑義は `UNKNOWN` として判断記録エスカレーション（§8、基本仕様 §11、要件定義 §12）。

#### 11.1.1 `chain_integrity` の評価

`chain_integrity` は宣言鎖のすべてのリンクが存在し、ハッシュ照合が成立するかを問う（基本仕様 §5.1）。次を評価し、いずれか違反があれば `MISMATCH`（切れた箇所を診断ラベルで示す）。

- **文書層**：各 `document` の `derives_from` 参照先が存在し（E-SCAN-012）、`content_hash` が現物と一致すること（不一致は診断 `STALE`。§11.4）。
- **VO 層**：各 VO が 1 件以上の `document` への解決可能な `derives_from` を持つこと（不在・解決不能は E-SCAN-012）。VO parent の不在・循環は E-SCAN-008。
- **Test 層**：発見された各 Test に対応する管理宣言（構文上有効な Test ID・1 件以上の `covers`・`intent` その他の必須 metadata。**`targets ≥ 1` は adapter 中立 core の必須リンクに含めず**、当該 adapter が必須とする追加 metadata として扱う〔`rust-cargo` では 1 件以上の `targets`〕。§4.1・基本仕様 §5.1・§9.1）がちょうど 1 件存在し（欠落は E-SCAN-007、診断 `MISSING`）、`covers` の全 VO 参照を解決でき（E-SCAN-003）、Test ID が発見結果全体で一意であること（衝突は E-SCAN-002）。
- **双方向完全性**：leaf VO → Test（検証実装の存在。covers する Test が 1 件以上）と、発見された Test → 宣言（管理宣言の解決）の両方向が成立して初めて成立する。covers する Test の無い leaf VO は `MISMATCH`（診断 `MISSING`）。
- Relation の from / to 不在は E-SCAN-009。恒久 SRC ID の adapter 越え衝突は E-SCAN-011。

すべての Test を管理対象とすることと、当該 Test を証拠として算入すること（§7 / §10 の target_binding / oracle_presence）は別個の条件とする（基本仕様 §8.1）。旧モデルの `role` に基づく `covers` 可変制約・適用項目集合は設けず、すべての管理対象 Test に `covers ≥ 1` を一律要求する（基本仕様 §12）。`covers` を持たない（0 件の）Test は管理宣言不整合として `chain_integrity = MISMATCH` であり、特別扱いの分岐を設けない。

### 11.2 Evidence 鮮度判定（target_binding の証拠有効性）

```text
対象 Test の Evidence のうち最新のものについて：

1. evidence.hashes.test_subject == 現在のTest subject hash
2. evidence.hashes.targetsの参照集合が、現在のTest.targetsを§6.1で解決したcanonical Locator集合と重複なく一致し、
   各target_constructが現在のimplementation construct hashと一致
3. evidence.revision.commit が非 null かつ現在のHEAD revisionと一致する
4. evidence.execution_state.complete == true かつ、同じschemaで現在再構築したExecution State subjectがcompleteで、hashが一致する
5. evidence.adapter が現在のTest.execution.adapterと一致する。adapter欠落形は§3.6の互換条件で一意に確認できる

1〜5 すべて成立 → 当該 Evidence は現在の証拠として有効
  （dirty: true でもExecution State subject一致なら有効。実行入力manifestが実体を保証する）
1 または 2 不成立 → NO_EVIDENCE（診断 STALE）
3 不成立          → NO_EVIDENCE（診断 STALE。現在revisionに対する実行ではない）
4のrecord欠落またはhash不一致             → NO_EVIDENCE（診断 STALE）
4のrecordがcompleteでない、または現在snapshotを完全に構築不能 → UNKNOWN
5が明示的不一致  → MISMATCH
5を確認不能      → UNKNOWN
Evidence なし     → NO_EVIDENCE（診断 NOT_EXECUTED）
```

Evidenceは全宣言targetが一意に解決できる場合だけ生成される（§9.4）。現在の宣言targetのうち1件でもcanonical Source Targetへ一意に解決できなくなった場合、記録済み参照集合は現在のcanonical集合と一致しないため条件2は成立せず、`target_binding` を有効な `PASS` にしない。
対象が存在せずE-SCAN-004となるtargetは`MISMATCH`（診断 `MISSING`）、複数候補により曖昧でE-SCAN-011となるtargetは`MISMATCH`として保持する（§5.4）。

有効な Evidence が得られたとき、`target_binding` は次で定まる。

- `result: FAIL`（テストランナーが失敗を報告） → `FAIL`（要件定義 §5.3）。
- `result: PASS` かつ全宣言 target の到達要件が §7.3 で充足（静的到達または runtime 到達） → `PASS`。
- `result: PASS` だが到達未充足の target がある → 当該 target の `target_coverage` に従い、count 0 は `FAIL`（診断 NOT_EXECUTED）、計測不能・未計測（`checked: false`）は `NO_EVIDENCE`（診断 NOT_CHECKED）、関数不見当は `UNKNOWN`。

Evidence が存在するが有効でない場合、`target_binding` は Evidence を再利用せず、上表の `MISMATCH` / `NO_EVIDENCE`（STALE）/ `UNKNOWN` を保持する。Evidence が無ければ `NO_EVIDENCE`（診断 `NOT_EXECUTED`）とする。複数条件が非 `PASS` なら根拠をすべて保持し、表示代表値は基本仕様 §22.2 の優先順位で選ぶ（診断ラベルは順位に用いず併記）。

### 11.3 集約アルゴリズム

項目scopeが省略された場合、aggregatorはconfig値から部分集合を組み立てず、基本仕様 §5 の固定4検査を選択する。`verify.full_scope`はconfig読込み時に§2.2のinvariantとして検証・正規化済みでなければならない。明示的な部分集合だけを限定scopeとし、その結果を完全検証として表示しない。

```text
fn aggregate(scope) -> Report:
  1. scan によりグラフ構築（§5）
  2. chain_integrity / orphan_detection を repository / DOC / VO / TEST 構造に対して評価
  3. scope のエンティティ軸で DOC/VO/TEST 部分木を選択
  4. 各 TEST について、scope の検査軸に含まれる target_binding / oracle_presence を評価
     （含まれない検査は NO_EVIDENCE、診断 NOT_CHECKED）
  5. 各 leaf VO について covers する TEST 群の結果を fail-closed で合成
  6. 子 VO を持つ VO（親 VO）は、子 VO の値と、当該親 VO を直接 covers する TEST の値を
     合わせて fail-closed で合成（直接 covers する TEST が無ければ子 VO の値だけを合成）
  7. DOC は下流 VO 部分木の合成（fail-closed）
  8. 総合判定：構造検査（chain_integrity / orphan_detection）と
     entity tree の scope 内評価がすべて PASS → OK、それ以外 → NG

fail-closed 合成：
  子に FAIL/MISMATCH/NO_EVIDENCE/UNKNOWN が1つでもあれば親は非 PASS。
  代表値は基本仕様 §22.2 の優先順位 FAIL > MISMATCH > NO_EVIDENCE > UNKNOWN で選ぶ。
  診断ラベル（MISSING / NOT_EXECUTED / NOT_CHECKED / STALE）は順位に用いず併記する。
```

利用者向け簡易出力は `OK` / `NG` の二値とする（基本仕様 §22.1）。詳細出力は任意ノードからの局所／経路／全体トレース（§11.6）に沿ったツリー表示とし、非 `PASS` の根拠（判断記録・Evidence への参照）を辿れる。人間向けテキストと機械可読 JSON の両方を出力できる（基本仕様 §22.3）。

検査の**表示scope**と、検査導出に必要な**内部依存の評価**は分離する。§7.3 により `target_binding` は当該 Test の Evidence 鮮度（§11.2）と target 別 `target_coverage` へ依存する。`target_binding` が項目scopeに含まれる場合、aggregator は §7.3 の runtime 到達証明の判定に必要な範囲でこれらを内部依存として評価する。runtime 証明に依存する `target_binding` の値は、根拠として用いた Evidence ID と当該 target の `target_coverage` 結果を report で引用し、原因を辿れる状態にする。

`covers` を持つ Test は covers 先それぞれの VO の合成に独立に参加する。「1つの Test が複数 VO を検証していること」自体は許容し、各 leaf VO の充足と組合せは §3.2.1 の実体化された leaf VO 単位で判定する（基本仕様 §10、§22.2）。

**機能単位の集約（基本仕様 §22.2 の Feature 単位）。** 基本仕様 §22.2 が Test 単位の結果の集約先として挙げる「Feature 単位」は、**親 VO**（`parent` により 1 件以上の子 VO を持つ VO。§3.2）を単位として実現する。Feature を独立のエンティティ種別・レコードファイル・ID 体系・宣言 field として設けず、`.verify/` に Feature 用ディレクトリを置かない（基本仕様 §3.1 のエンティティ種別を増やさない）。

- 親 VO の値は上記 step 6 の fail-closed 合成そのものであり、機能単位の表示のために別の合成規則・緩和規則を設けない。子に 1 つでも非 `PASS` があれば親 VO は非 `PASS` であり、代表値の優先順位も基本仕様 §22.2 と同一とする。
- Test の結果が親 VO へ寄与する経路は、(a) covers する leaf VO 経由の伝播と、(b) 当該親 VO を直接 covers する Test の直接参加の 2 つに限る。covers 宣言を経由しない「機能名による束ね」（ファイルパス・モジュール名・命名規約からの推定束ね）を設けない。
- 親 VO を持たない leaf VO は、それ自体が最上位の束ね単位となる。DOC 単位の集約（step 7）は VO 部分木の合成であり、機能単位の集約はその中間段に位置する。
- 機能単位の表示経路（起点指定と内訳の提示）は §11.6 の projection で露出し、新規コマンド・ツール・出力エンティティを増やさない。

### 11.4 document 鮮度

スキャン時に document レコードの `content_hash` と実ファイル（`path`）を比較し、不一致なら W-SCAN-104 を出す。
当該 document を `derives_from` で参照する VO / 上位 document の鎖は、content_hash 不一致として `chain_integrity = MISMATCH`（診断 `STALE`）となる（§11.1.1）。
当該 document subject を dependency に含む判断記録（§8.5）・承認記録（§3.5）も無効となる。
仕様文書の更新は `vtest doc add --update` による再登録で反映し、依存する判断・承認が失効することを利用者へ提示する。再登録で document subject hash が変化するため、以前の dependency entry を現在の承認・判断へ流用しない。

### 11.5 フェーズゲート評価

プロジェクト側が登録したフェーズ・工程・ゲートの進行条件について、現在の検証状態（§4.1 の 5 状態）と承認（§3.5）が通過条件を満たすかを**評価・提示できなければならない（MUST）**（基本仕様 §20、要件定義 §26.4）。検証状態と承認は独立の軸であり、ゲートは両者の組合せを進行条件にできる。

- **ゲート定義**：`config.yaml` の `gates`（§2.2）に、ゲート名と進行条件（`require.verification`＝要求する検証結果、`require.approvals`＝要求する承認ロール集合）を保持する。
- **評価**：`vtest verify --gate <name>` は、指定ゲートの対象 scope について検証を実行し、(1) 検証結果が `require.verification` を満たすか、(2) `require.approvals` の各ロールについて対象の有効な承認（§3.5）が存在するか、を評価して満否と根拠（不足している非 `PASS` 検査・未充足の承認ロール）を提示する。
- **ゲート名の解決**：`--gate <name>` は `gates[].name` との大文字小文字を区別した完全一致で解決する。前方一致・部分一致・近似一致・既定ゲートへの代替は行わない。一致するゲート定義が無い場合（`gates` が空、または未定義名の指定）は usage error として **E-CONFIG-002**（終了コード 2）で拒否し、スキャン・検証・ゲート評価のいずれも実行せず、検証結果・部分結果を生成しない。診断には指定名と定義済みゲート名の一覧を含める。
- **検証条件の充足判定**：ゲートの検証条件は、`require.verification` の値と、要求 scope の**集約代表値**との**完全一致**でのみ充足する。
  - 集約代表値は、要求 scope 内で評価した全値（構造検査 `chain_integrity` / `orphan_detection` と、エンティティ軸の部分木で評価した各 Test / VO / DOC の検査値）を §11.3 の fail-closed 規則で合成した 1 値とする。全値が `PASS` なら代表値は `PASS`（総合 OK と同値）、非 `PASS` が混在する場合は基本仕様 §22.2 の優先順位 `FAIL > MISMATCH > NO_EVIDENCE > UNKNOWN` で選ぶ。診断ラベルは充足判定に用いない。
  - 5 状態に順序・優劣・包含関係を設けない。「要求値以上」「要求値より良い」といった比較解釈を採らず、`require.verification: UNKNOWN` は代表値が `UNKNOWN` のときだけ充足し、代表値が `PASS` でも充足しない。同様に `require.verification: PASS` は代表値が `PASS` のときだけ充足する。
  - `--items` で検査軸を限定した実行では、scope 外の検査が `NO_EVIDENCE`（診断 `NOT_CHECKED`）として代表値の合成に参加する（§11.3、基本仕様 §4.6）。したがって限定 scope での `require.verification: PASS` は充足せず、限定 scope の結果でゲートを充足させることはできない。
  - 承認条件は検証条件と独立に評価し、`require.approvals` が空集合（省略）なら承認条件は充足とする。承認未充足は検証状態を降格させず、検証の非 `PASS` は承認の充足有無を変えない（基本仕様 §4.5）。ゲート全体の充足は検証条件と承認条件の両方が充足した場合に限る。
- **責務境界**：本システムの責務はゲート条件が現在満たされているかの**評価・提示に限る**。フェーズのライフサイクル管理・工程の自動遷移は責務外とする（基本仕様 §20、§29 OOS-004、要件定義 §26.4）。「Release フェーズへ遷移させる」のではなく「Release gate の条件を現在満たしている」を提示する。
- **露出点**：新規 CLI コマンド・MCP ツールを増やさず、既存の `vtest verify` の `--gate` 引数と出力、および `report` の JSON でゲート評価を露出する（引数・出力 schema は別紙A）。具体的なフェーズ名・承認ロール・必要承認数・権限 schema はプロジェクト設定と別紙A へ委譲する（基本仕様 §30）。

### 11.6 役割別 projection

同一のトレーサビリティ構造から、利用者の役割または利用目的に応じて参照対象・関係・集約粒度を変えた projection を取得・提示できる（基本仕様 §19、要件定義 §3.4）。

- **任意ノードからの取得**：最小の意味単位「上流ノード → 関係 → 下流ノード」を任意のノード（DOC / VO / TEST / SRC）から取得でき、必要に応じて上流／下流へ連続して辿れ、プロジェクト全体のトレーサビリティ構造も取得できる。常に全チェーンを表示することは求めない。
- **projection**：役割または利用目的に応じた参照観点を preset として提供する（例：PM は上位の document・VO の状態と未確定/NG、Tester は VO・Test・検証対象・Evidence・未実施/失敗理由、Coder は実装から関連 Test・VO・上流 document へのトレース）。役割を固定 enum やモード名として本冊で仕様化せず、preset・UI・モード体系は別紙A へ委譲する（基本仕様 §30）。
- **機能単位の束ね表示**：親 VO を起点とする下流方向の projection が、§11.3 の機能単位の集約（Feature 単位＝親 VO）を提示する経路である。当該親 VO の代表値と、その配下の子 VO ごと・Test ごとの内訳を同じ出力から辿れる。Feature 名・Feature ID の別 field を出力に設けず、束ねの識別子は親 VO の ID とする。
- **露出点**：新規コマンド・ツールを増やさず、既存の `vtest report` の view / projection 引数と、`test query` の逆引きで露出する（引数・出力 schema は別紙A）。逆引きインデックス（VO → Tests、SRC → Tests、DOC → VOs、DOC → DOCs）を projection の基盤とする（§5.3）。

### 11.7 判断待ち情報の構造

導入時・検証時に生じる、未確定事項・不整合・未検証事項・機械的に確認済みの事実・外部判断が必要な事項を、機械可読な構造として保持・取得可能とする（基本仕様 §18.3、要件定義 §17.3）。

- **構造**：判断待ち情報は次の項目を持つ構造化 record（report JSON 内の section）として提示する。
  - `subject`：対象エンティティ ID または解決済み canonical Locator。
  - `kind`：`unknown`（UNKNOWN によるエスカレーション）/ `unregistered`（管理宣言欠落）/ `unresolved`（参照解決不能）/ `undecided`（VO 未確定）/ `pending_approval`（承認待ち）。
  - `check`：関係する検査（4 検査のいずれか）と現在の検証状態・診断ラベル。
  - `basis`：機械的に確認済みの事実（宣言鎖・検査結果・対象外とした範囲）への参照。
  - `bundle_ref`：外部判断が必要な場合の判断バンドル（§8.1）への参照（任意）。
- **露出点**：新規コマンド・ツールを増やさず、`vtest verify` / `vtest report` の JSON 出力に判断待ち section を含めて露出する。UNKNOWN だけでなく、検証出力全体にわたる未確定・要判断事項を横断的に集約する（表示形式は別紙A、基本仕様 §30 item 19）。

---

## 16. 並列動作と整合性

（§12〜§15 は別紙A参照）

### 16.1 ロック不要の根拠

書き込み操作は次のいずれかに分類され、ファイルロックを必要としない（基本仕様 §24.2）。

- **新規レコード追加**（rel / decisions / approvals / evidence）：ULID ファイル名の新規作成のみ。並列生成は衝突しない。
- **エンティティファイル編集**（doc / vo）：1 エンティティ 1 ファイル。異なるエンティティの並列編集は独立。同一エンティティの並列編集は Git のマージ衝突として顕在化する。
- **テストコード編集**：通常のソース編集と同じ扱い。

同時実行された `vtest` プロセス同士の調停は行わない。
すべての判定は「その時点の正典の読み取り」に基づき、正典が変われば次回の scan / verify が差分を反映する。

この「その時点の正典の読み取り」は書込みの**原子的公開**（基本仕様 §24.2）を前提とする。原子的公開の対象は`.verify/`配下のrecord・エンティティファイル（新規レコード追加とエンティティファイル編集）であり、完全な内容が単一の操作で可視になる方式（同一ファイルシステム内へのtemp書込み＋rename等）で公開し、書きかけ状態・一時ファイル残渣を正典ディレクトリの読み手に観測させてはならない。テストコード編集は通常のソース編集と同じ扱いで本規定の対象外とし、解析不能な中間状態は adapter discovery の E-SCAN-001 / Incomplete としてfail-closedに検出される（§5.1）。

### 16.2 意味的衝突検出

`vtest doctor`は、同じTest IDの重複、covers先VOの欠落、承認済VOの内容不一致など、
version controlの構文的整合性だけでは判定できない論理的不整合を次の規則で検出する。

- ID 衝突 → E-SCAN-002
- dangling reference → E-SCAN-003 / E-SCAN-009 / E-SCAN-012
- 孤児 document → E-SCAN-016
- 承認の失効 → §3.5 のハッシュ束縛により自動的に draft へ
- 判断記録・Evidence の失効 → §8.5 / §11.2 のハッシュ束縛により自動的に無効（診断 STALE）へ

---

## 17. 診断・終了コード体系

### 17.1 診断コード

§5.4 のスキャン診断に加え、次を定義する。

| コード | 種別 | 内容 |
|---|---|---|
| W-SCAN-104 | warning | document レコードの content_hash と実ファイルの不一致（依存判断・依存Approvalは無効、鎖は chain_integrity STALE） |
| E-EXEC-001 | error | テストビルド失敗 |
| E-EXEC-002 | error | 要求したテストの結果行が得られない |
| E-EXEC-003 | error | 終了コードと結果行集計の矛盾 |
| E-EXEC-004 | error | 実行中にExecution State subjectが変化 |
| W-EXEC-101 | warning | カバレッジツール利用不能（target_coverage は checked: false、検証時 NO_EVIDENCE/NOT_CHECKED） |
| E-AUDIT-001 | error | 提出された bundle_id が存在しない |
| E-AUDIT-002 | error | バンドル記録時のハッシュと現在のハッシュの不一致（対象が変更済） |
| E-AUDIT-003 | error | subject の不一致・スキーマ違反 |
| E-AUDIT-004 | error | decision が受理する判断値でない |
| E-APPROVAL-001 | error | Approval対象または上流依存closureを完全・currentに解決できず、recordを生成しない |
| E-CONFIG-001 | error | config version、`verify.full_scope`（固定4検査）、`doc.roots`、`gates`（名前重複、`require` / `require.verification` 欠落、`require.verification` が5状態語彙外、`require.approvals` の不正・未解決ロール）、config field型または登録adapterが検証する設定値が現在のconfig invariantに違反（未知・重複adapter IDはE-ADAPTER-001） |
| E-CONFIG-002 | error | 呼出しが config に定義の無いゲート名を参照（`--gate` / MCP の `gate` 入力。config 内容自体は invariant を満たす。検証・ゲート評価を実行せず結果を生成しない。§11.5） |
| E-OP-001 | error | Structured Operation の入力検証失敗（候補提示を伴う。§6.3） |
| E-OP-002 | error | Edit 対象 Test の特定失敗 |
| E-OP-003 | error | 編集結果が1 Test の範囲を超える（別紙A §15.4。操作は中止される） |
| E-ADAPTER-001 | error | adapterが未登録、重複、またはregistryの宣言と実装が不一致 |
| E-ADAPTER-002 | error | adapterのdiscoveryまたはrunnerが確定的に失敗（Evidenceなし） |
| E-ADAPTER-003 | error | Testのexecution descriptorと選択adapterが不一致 |
| E-ADAPTER-004 | error | 明示操作に必須のadapter capabilityが未提供（変更・判断・Evidenceなし） |
| W-ADAPTER-101 | warning | 検証対象のadapter capabilityが未提供（能力に応じ NO_EVIDENCE/NOT_CHECKED または NOT_EXECUTED） |
| W-ADAPTER-102 | warning | adapterが解析限界を報告（該当検査はUNKNOWN） |

旧モデルの意味監査提出検査（E-AUDIT-005 / E-AUDIT-006 / E-AUDIT-007）は判断記録層への転用（§8.4）に伴い撤去する。

### 17.2 終了コード

| コード | 意味 |
|---|---|
| 0 | 要求 scope の検証結果が OK（操作コマンドでは成功） |
| 1 | 検証結果が NG |
| 2 | 操作拒否（E-OP-* / E-ADAPTER-* / E-APPROVAL-* / E-CONFIG-*、引数不正、adapter前提・capability・実行失敗、スキーマ違反の提出など。検証結果は生成しない） |
| 3 | 内部エラー（ツール自体の異常） |

`--gate <name>` を指定した `vtest verify` / `vtest report` では、0 と 1 をゲート充足で決める。ゲート全体が充足（§11.5 の検証条件と承認条件の両方が充足）なら 0、いずれかが不充足なら 1 とする。`require.verification` に `PASS` 以外を定義したゲートでは、集約代表値が要求値と一致して充足した実行が 0 になり、この場合に総合が NG であることは 0 を妨げない。要求 scope の総合 OK / NG は JSON と text の集約出力から読み取れる（別紙A §12.1・§12.3）。ゲート名が未定義の場合は E-CONFIG-002 で 2 とし、0 / 1 を返さない。

終了コードは診断severityだけでなく操作段階で決める。`vtest scan` / `vtest doctor`では、
registry・config・adapter契約の検証またはadapter呼出しがE-ADAPTER-* / E-CONFIG-*で拒否された場合は2、
scanが完了してrepository整合性のE-SCAN-*を報告した場合は1、errorがなければ0とする。
同一実行に複数候補がある場合は内部エラー3、操作拒否2、検証NG1、成功0の順で優先する。検証状態と内部エラーは終了コードで分離する（基本仕様 §4.4、§26.1）。

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
- LLM API直接呼び出しによる判断
- rename 追跡と SRC 恒久 ID の自動昇格支援
- cargo-nextest 対応

---

## 付記（非規範）: トレーサビリティ表

本表は本冊の各節が実現する上流§と、その導出区分（CONFORM＝旧版から生存し引用・項目名の修復のみ／再導出＝旧構造を凍結モデルへ書き換え／新設＝旧版に無く上流から新規）を記録する。全節が上流（要件定義 or 基本仕様）へトレースでき、親を持たない節を作らないことを設計制約とする。

| 本冊の節 | 実現する上流§ | 区分 |
|---|---|---|
| §0 本書の位置付け | 基本§0・P-005・§26・§30 | CONFORM（分冊構造・委譲参照の連番修復、新設サブ節の収録範囲明示） |
| §1.1 ワークスペース構成 | 基本§27・§30 | CONFORM（audit crate 責務を判断記録＋静的解析へ読替） |
| §1.2 主要依存クレート | 基本§27 | CONFORM（evidence_validity 参照を鮮度 STALE へ修復） |
| §1.3 内容ハッシュの定義 | 基本§1・§6・§3・§9・§9.1・§21 | 再導出（role/anchor 束縛除去、SPEC/REQ hash→総称 document hash、静的 subject 廃止、Source Target hash を検証対象の Source Target 実現形態束縛として form-scope） |
| §2.1 `.verify/` レイアウト | 基本§24.1・§3.1 | 再導出（spec/+req/→doc/、audits/ 廃止、decisions/ 追加） |
| §2.2 `config.yaml` | 基本§2.4・§4.1・§4.6・§5.2・§20・§22.1 | 再導出（full_scope 12→4検査 invariant、doc.roots・gates 追加） |
| §2.3 派生情報 | 基本§24.3・P-003 | CONFORM |
| §3.1 document レコード | 基本§3.1・§3.2・§3.4・§18 | 再導出（SPEC/REQ→総称 document、derives_from＋任意 note〔M-6〕） |
| §3.2 VO レコード | 基本§3.2・§10 | 再導出（requirements/spec_refs→derives_from:[DOC-]、section 廃止） |
| §3.2.1 dimensions | 基本§10 | CONFORM（vo-coverage 監査参照を UNKNOWN エスカレーションへ） |
| §3.3 Relation レコード | 基本§2.3・§3.2 | CONFORM |
| §3.4 判断記録レコード | 基本§11.3・要件§12 | 新設（M-2。actor/subject/decision 必須・理由 optional） |
| §3.5 承認レコード | 基本§17・§4.5・要件§19 | 再導出（承認前提の分離、closure kind vo/document＋上位 document 再帰） |
| §3.6 Evidence レコード | 基本§21・§6・§7 | 再導出（target_execution field→target_coverage 改名、runtime_result 吸収） |
| §4.1 adapter-neutral 正規化 | 基本§3・§9.1・§9.2・§12・要件§9.1 | 再導出（role/anchor/characterization 除去、covers≥1 一律、検証対象を core で一般化＝Source Target 実現は adapter 層／rust-cargo が targets≥1） |
| §4.2 rust-cargo annotation 文法 | 基本§30 item4・§12 | 再導出（role/anchor キー・語彙撤去、src-id 存続） |
| §4.3 rust-cargo locator 構文 | 基本§9.2・§30 item7 | CONFORM |
| §4.4 宣言エラーの扱い | 基本§12・§5.1・§9.2 | 再導出（role materialization/E-SCAN-013-015 撤去、core 必須 id/covers≥1/intent 欠落→MISMATCH。targets≥1 は rust-cargo adapter の必須 metadata として E-SCAN-007 で検出＝再キー） |
| §5.1 処理フロー | 基本§2.1・§23 | 再導出（step4 の role 確定除去） |
| §5.2 エンティティモデル | 基本§4.1・§5・§3 | 再導出（CheckValue 8→5＋診断ラベル、CheckItem 12→4、TestRole/TestAnchor 除去） |
| §5.3 検証グラフ | 基本§3.1・§19 | 再導出（SPEC/REQ ノード除去、DOC/derives_from へ） |
| §5.4 整合性診断 | 基本§23・§5・§9.2 | 再導出（E-SCAN-013-015 除去、E-SCAN-012 を document 参照へ、E-SCAN-016 追加、E-SCAN-007 の targets≥1 を rust-cargo adapter 必須 metadata へ再キー） |
| §5.5 rust-cargo SourceDiscoveryAdapter | 基本§27・§30 | 再導出（role/anchor 抽出除去） |
| §5.6 文書層 orphan_detection | 基本§5.2・要件§4.2 | 新設（M-1。根指定＋孤児検出） |
| §6.1 adapter-neutral 解決 | 基本§9.2・§3.3 | CONFORM |
| §6.1.1 target identity の一方向確定 | 基本§9.2・§6 | CONFORM |
| §6.2 rust-cargo locator 解決 | 基本§9.2 | CONFORM |
| §6.3 候補提示 | 基本§15 | CONFORM |
| §7.1 静的解析判定の原則 | 基本§5.4・§8・P-003 | 再導出（意味監査委譲→判断記録エスカレーション、UNKNOWN 昇格経路を target_binding 限定） |
| §7.2 rust-cargo ルール一覧 | 基本§5.4・§5.5・§8.3 | 再導出（DA-001/003-006→oracle_presence、DA-002→target_binding、静的レコード永続化廃止） |
| §7.3 target 到達の静的/runtime 証明 | 基本§5.3・§7・§9.1 | 再導出（static_audit→target_binding、DA-003 を join 外へ、DA-002 再マップ、target 実行到達規則を Source Target 実現形態へ form-scope） |
| §8 判断記録プロトコル | 基本§11・要件§12 | 再導出（bundle/submit を検証ゲートから切離し判断記録へ転用、verdict 写像廃止、M-2/M-5 基盤） |
| §9 テスト実行設計 | 基本§21・§7 | CONFORM（--req 除去、NOT_EXECUTED 診断へ修復） |
| §10 rust-cargo Target Binding 動的計測 | 基本§5.3・§21 | 再導出（target_execution 検査→target_binding 証拠、count0→FAIL/NOT_EXECUTED） |
| §11.1 検査の評価地点 | 基本§5・§22 | 再導出（12項目表→4検査、旧項目の吸収・撤去明示） |
| §11.1.1 chain_integrity の評価 | 基本§5.1・§9.1・§12 | 再導出（test_existence/test_traceability 統合、covers≥1 一律、Test 層 core 必須から targets≥1 を除外＝Source Target 実現は adapter 層） |
| §11.2 Evidence 鮮度判定 | 基本§6・§21.1・§5.3 | 再導出（evidence_validity/runtime_result→target_binding 証拠、8→5状態写像） |
| §11.3 集約アルゴリズム | 基本§22 | 再導出（4検査×5状態×fail-closed、§22.2 優先順位、診断ラベル併記） |
| §11.4 document 鮮度 | 基本§6・§11.4 | 再導出（SPEC 鮮度→document 鮮度、chain_integrity STALE） |
| §11.5 フェーズゲート評価 | 基本§20・要件§26.4 | 新設（M-3。MUST・評価/提示のみ、自動遷移は責務外） |
| §11.6 役割別 projection | 基本§19・§22.2・要件§3.4 | 新設（M-4。任意ノード取得・役割別 projection） |
| §11.7 判断待ち情報の構造 | 基本§18.3・§30 item19 | 新設（M-5。機械可読な判断待ち構造） |
| §16.1 ロック不要の根拠 | 基本§24.2 | CONFORM（decisions/ を追加、SPEC→document） |
| §16.2 意味的衝突検出 | 基本§23・§24.2 | 再導出（監査失効→判断記録失効、孤児 document 追加） |
| §17.1 診断コード | 基本§4.2・§30 item11-12 | 再導出（E-AUDIT-005-007 撤去、E-AUDIT-004→decision 値、SPEC→document） |
| §17.2 終了コード | 基本§4.4・§26.1・§30 item12 | CONFORM |
| §19 実装選択と提供範囲 | 基本§27・R-2・R-3 | CONFORM（監査→判断へ用語修復） |
