# SpecTracer 言語アダプタ分離リファクタリング計画 v0.2

## 0. 文書の位置付け

本書は、SpecTracer の検証契約を言語・テストランナー非依存に保ちながら、
現行 v0.1 に直接埋め込まれている Rust / Cargo 固有処理をアダプタへ分離する
ための実装計画である。

本書は要件定義、基本仕様、詳細設計を上書きしない。実装開始前に、本書で
列挙する仕様変更を正規の仕様文書へ反映し、仕様の優先順位を次のまま保つ。

1. 要件定義・要件分解
2. 基本仕様
3. 詳細設計本冊
4. 詳細設計別紙A・別紙B

### 0.1 開始条件

このリファクタリングは v0.1 の M9 完了後に開始する。M9 の全 MCP ツールの
CLI parity、別紙A §13.3 の完全な参照フロー、transport error matrix は完了し、
受入台帳は `PASS` になった。独立Luna reviewerもupsert、notification、freshness、
非PASS保持を再確認して `PASS` を報告した。アダプタ分離自体は、W0の仕様同期と
本計画の依存順序を満たしてから開始する。

M9 完了前に本計画を着手する場合は、マイルストーン順序を変更する仕様判断と
受入台帳の再ベースラインが必要である。実装エージェントはこの判断を代行しない。

### 0.2 ベースライン

- README 更新コミット：`85ff47e` (`docs: update status and language support`)
- M9 完了コミット：`ee406eb` (`feat: complete M9 MCP usability`) + `575f36f` (`fix: close M9 MCP review gaps`)
- M1〜M8：`DONE`
- M9：`DONE`
- M9 独立レビュー：`PASS`（修正コミット `46fbb4d` を再監査）
- 現行 workspace：8 crates
- 現行回帰：110 tests
- M9 acceptance：9 tests（全PASS）
- 共通ゲート：fmt / workspace test / clippy `-D warnings` / `vtest doctor`

## 1. 目的

### 1.1 達成すること

- 11項目の検証状態、REQ / VO / Test の検証グラフ、Approval、Audit、
  Evidence、内容ハッシュ、fail-closed 集約を言語非依存の契約として維持する。
- source discovery、symbol resolution、static audit、Structured Test Operation、
  test execution、coverage attribution をアダプタ能力として分離する。
- Rust / Cargo の既存機能を built-in `rust-cargo` adapter として移植する。
- adapter が存在しない、能力を持たない、または判定不能な場合に PASS を生成しない。
- v0.1 の canonical records と既存 Evidence を破棄・書換えせず読み取れるようにする。
- CLI と MCP が同じ adapter registry とコア処理を利用する。
- 将来の TypeScript、Go、C# adapter を、コア verifier の変更なしで追加できる
  crate・型・設定境界を確立する。

### 1.2 この計画で実装しないこと

- TypeScript、Go、C# の production adapter 本体
- 動的ライブラリや外部プロセスによる third-party plugin ABI
- LSP サーバとの統合
- 複数言語に共通する自動修正方針
- unsupported ecosystem に対する推測 PASS
- canonical Audit / Evidence / Approval レコードの履歴書換え

最初の非Rust adapterは、境界試験専用の in-process synthetic adapter とする。
production language adapter は本計画完了後の独立マイルストーンで追加する。

## 2. 現状と解消対象

### 2.1 `vtest-model` の Cargo 固有実行座標

`TestEntity` は現在、次を直接保持する。

- `filter: String`
- `package: String`
- `test_target: TestTarget`
- `TestTarget::Lib / Bin / IntegrationTest`

`TestEntity` は scan から導出される情報であり canonical record ではないが、
scan JSON、CLI / MCP、scan / operations / exec / verify の公開境界へ露出している。

### 2.2 `vtest-store` の Rust 固有設定

- `scan.assertion_macros` は Rust macro path を前提とする。
- `run.coverage` は `llvm-cov | off` のみを受理する。
- `init` が `rust-unit-function` と `rust-integration` を常に配置する。

### 2.3 orchestration crate と Rust 実装の混在

- `vtest-scan` が `.rs` discovery、`syn` parse、Cargo target 解決、annotation、
  symbol resolution、Structured Test Operation を同時に所有する。
- `vtest-audit` が `syn` AST と DA-001〜006 の Rust 判定を直接所有する。
- `vtest-exec` が cargo command、cargo output parser、cargo-llvm-cov、
  rustc demangle を直接所有する。
- `vtest-cli` が built-in Rust form と上記実装を直接結合する。

### 2.4 仕様上の tension

- 要件定義 §27 は概念モデルを Rust 固有に限定しない。
- 基本仕様 §0 と詳細設計 v0.1 は初期対象を Rust に限定する。
- 詳細設計 §5.2 のコア `TestEntity` は Cargo 固有フィールドを持つ。
- 詳細設計 §19 は非Rust対応を将来課題とする。

この tension は、v0.1 の実装範囲としては矛盾ではない。しかし v0.2 で
言語追加を開始する前に、adapter contract と互換方針を仕様へ昇格させる必要がある。

## 3. 目標アーキテクチャ

### 3.1 crate 構成

次の2 crateを追加する。

```text
crates/
  vtest-adapter-api/    # 言語非依存 adapter 契約、registry、能力・error型
  vtest-adapter-rust/   # built-in rust-cargo adapter
```

依存方向を次に拡張する。

```text
vtest-cli / vtest-mcp
        |
        v
vtest-verify / vtest-exec / vtest-audit / vtest-scan
        +---------------------> vtest-store ------> vtest-model
        |
        v
vtest-adapter-rust -----------> vtest-store
        |
        v
vtest-adapter-api ------------> vtest-model
```

補足：

- `vtest-adapter-api` は `vtest-model` のみに依存することを優先する。
- form schema 等が必要なため `vtest-adapter-rust -> vtest-store` は許容する。
- `vtest-adapter-rust` は scan / audit / exec / cli / mcp / verify に依存しない。
- core crate から `syn`、`rustc-demangle`、Cargo manifest parser への依存を除き、
  `vtest-adapter-rust` だけが所有する。
- CLI と MCP は adapter を個別実装せず、同じ registry composition を使う。

### 3.2 capability 分割

1個の巨大な `LanguageAdapter` trait に全能力を必須化しない。次を独立した
object-safe capability とする。

```text
SourceDiscoveryAdapter
StaticAuditAdapter
StructuredTestAdapter
TestRunnerAdapter
CoverageAdapter
```

各 adapter は `AdapterDescriptor` で次を宣言する。

```text
id                例: rust-cargo
languages         例: [rust]
capabilities      discovery / static-audit / structured-test / runner / coverage
config_namespace  例: rust
```

registry は adapter ID の重複を拒否し、ID順の決定論的な列挙を保証する。

### 3.3 orchestration の責務

| crate | adapter分離後も所有する責務 |
|---|---|
| `vtest-model` | IDs、Locator、hash、CheckValue、Evidence、言語非依存DTO |
| `vtest-store` | canonical record、config読書き、append-only、schema validation |
| `vtest-scan` | adapter選択、結果merge、record integrity、graph、diagnostic集約 |
| `vtest-audit` | adapter audit呼出し、config hash、AuditRecord追記、結果合成 |
| `vtest-exec` | adapter runner呼出し、Git revision、raw log、Evidence追記 |
| `vtest-verify` | 現行11項目、鮮度、scope、fail-closed集約 |
| `vtest-cli` | 非対話CLI、registry composition、JSON envelope |
| `vtest-mcp` | CLIと同一core呼出し、MCP transport、tool schema |

### 3.4 Rust adapter の責務

`vtest-adapter-rust` は次を所有する。

- `.rs` discovery と ignore 処理
- `syn` parse
- `@vtest.*` doc annotation 抽出
- Rust / Cargo symbol・package・target・selector解決
- DA-001〜006、W-DA-101 の Rust AST 実装
- Rust unit / integration test の create / edit / query support
- cargo test command と結果 parser
- cargo-llvm-cov command、JSON parser、rustc demangle、target count
- `rust-unit-function` / `rust-integration` built-in forms の提供

## 4. 言語非依存モデル

### 4.1 新しい導出型

`TestTarget`、`filter`、`package` を直接コア判断へ使わず、次の型を導入する。

```rust
pub struct AdapterId(String);

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
```

意味：

- `project`：Cargo package、npm workspace、Go package、`.csproj` 等の実行単位
- `suite.kind`：`lib`、`bin`、`integration`、`vitest`、`go-package`、`xunit` 等
- `suite.name`：必要な場合の target / project / config 名
- `selector`：adapterが1 Testを一意に実行するためのopaque selector

core は各文字列を解釈しない。解釈とcommand生成は該当 adapterだけが行う。

### 4.2 `TestEntity` の段階移行

第1段階は additive migration とする。

```text
TestEntity.execution: ExecutionDescriptor を追加
filter/package/test_target は legacy compatibility field として残す
Rust scanner は execution と legacy fields の両方を同じ情報から生成
core consumer は execution のみ参照
```

全consumer移行後、legacy fieldsをdeprecatedにする。削除はv0.2内で行わず、
versioned JSON contractを定義した後の別マイルストーンとする。

これによりM1のscan JSONとMCP parityを不必要に破壊しない。

### 4.3 Source model

`Locator.path`と`Locator.item_path`は文字列として既に他言語を表現できるため維持する。
`SourceLocation.function`と`SourceFunction`の名称はRust寄りだが、v0.2ではwire互換を
優先して変更しない。仕様では「function」値をadapterが返すopaque symbol display name
として再定義する。名称変更はversioned schemaの別判断とする。

### 4.4 Evidence互換性

- 既存Evidenceは書き換えない。
- `RunnerInfo.kind`、`command`、`exit_code`は既にrunner非依存なので維持する。
- `TargetExecution.method`もstringのためcoverage provider名を保持できる。
- adapter IDをEvidenceへ追加する場合はoptional fieldとし、旧recordの欠落を
  PASS条件に追加しない。旧recordの有効性は既存hash / revision規則で判定する。
- 新recordではadapter IDとrunner kindの整合を保存前に検証する。
- unknown adapterのEvidenceを推測で実行済み・coverage PASSへ昇格しない。

## 5. 設定と互換性

### 5.1 config version 2

v2 config の目標形を次とする。

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
  full_scope: [...]
```

adapter固有設定は該当adapter namespace内で検証する。core config parserは未知の
adapter設定を黙って受理せず、登録adapterへ検証を委譲する。

### 5.2 v1互換

- v1 configを読み込むと、in-memoryで単一`rust-cargo` adapter設定へ変換する。
- 読取りだけでconfig.yamlを書き換えない。
- `vtest init`はv2を生成する。
- 明示的な`vtest config migrate --dry-run`なしにcanonical configを更新しない。
- v1とv2から得られるRust scan / audit / run / verify結果が同値であるfixtureを置く。
- 未知adapter、重複ID、重複root、無効capability設定はusage errorとして拒否する。

## 6. fail-closed契約

新しい診断コードは仕様 §17へ追加してから実装する。

| code | 条件 | 結果 |
|---|---|---|
| `E-ADAPTER-001` | 設定adapterが未登録・重複 | 操作失敗、PASSなし |
| `E-ADAPTER-002` | discovery / runnerの確定的失敗 | 該当操作失敗、Evidenceなし |
| `E-ADAPTER-003` | Testとexecution descriptorのadapter不一致 | 該当Test非PASS |
| `W-ADAPTER-101` | requested capabilityをadapterが提供しない | 該当項目`NOT_CHECKED` |
| `W-ADAPTER-102` | adapterが解析限界を報告 | 該当項目`UNKNOWN` |

規則：

- discovery不能時に「Testが0件」として正常終了しない。
- runner未提供時にEvidenceを生成しない。
- coverage未提供時は`target_execution = NOT_CHECKED`でありPASSではない。
- static audit未提供時は`static_audit = NOT_CHECKED`または`UNKNOWN`でありPASSではない。
- adapterが返したTest IDは全adapterをmergeした後にglobal uniquenessを検査する。
- adapter結果の順序はadapter ID、path、Test IDで決定論的に正規化する。
- adapter panicやmalformed outputを内部エラーまたはadapter errorとして閉じる。

## 7. 実装ウェーブ

各ウェーブ完了時に狭いテスト、workspace全体、architecture-checkを実行する。
先行ウェーブがPASSするまで次ウェーブの公開APIを変更しない。

### W0 仕様と受入契約の確定

所有：主担当のみ。Lunaはread-only explorer / reviewerとして使用する。

変更対象：

- 要件定義 §27
- 基本仕様 §0、§2、§7.2〜7.10、§8、§11、§16
- 詳細設計 §1、§2.2、§5、§7、§9、§10、§17、§19
- 別紙A §12〜15
- 別紙B §18
- `AGENTS.md`
- `tests/ACCEPTANCE.md`

完了条件：

- adapter contract、v1/v2 config、JSON互換、Evidence互換が仕様化される。
- Rust v0.1の観測可能挙動を維持する項目と、versioned変更項目が区別される。
- 新しいadapter受入基準にテスト名の予約がある。
- M9がDONEである。

### W1 adapter APIとneutral model

成果：`vtest-adapter-api`、neutral execution descriptor、registry unit tests。

完了条件：

- duplicate adapter IDを拒否する。
- capability lookupが決定論的である。
- API crateに`cargo`、`syn`、`rustc-demangle`固有型が露出しない。
- `TestEntity.execution`がadditiveにserializeされる。
- legacy fieldsとのRust変換round-tripが一致する。

### W2 config v2とRust adapter skeleton

成果：v1互換loader、v2 writer、`vtest-adapter-rust` module skeleton、built-in registry。

完了条件：

- v1 configを無変更で読める。
- v2 initとround-tripが決定論的である。
- unknown / duplicate adapterがfail-closedになる。
- Rust adapter descriptorとcapability宣言が取得できる。

### W3 Rust discovery / operations移植

成果：Rust source discovery、Cargo execution descriptor、symbol resolution、
Structured Test Operationを`vtest-adapter-rust`へ移す。

完了条件：

- M1、M2、M8 acceptanceが既存fixtureで全PASS。
- scan JSONのlegacy fieldsが維持され、新しいexecution descriptorも正しい。
- editが1 Test境界を維持する。
- `vtest-scan`に`syn`の直接利用が残らない。

### W4 Rust static audit移植

成果：DA-001〜006、W-DA-101をRust adapterへ移し、`vtest-audit`をorchestrator化する。

完了条件：

- M3、M5 acceptanceが全PASS。
- analysis limitがUNKNOWNのままである。
- audit subject hash、config hash、append-only保存が変わらない。
- `vtest-audit`に`syn` / `quote`の直接依存が残らない。

### W5 Rust runner / coverage移植

成果：cargo test、result parser、cargo-llvm-cov、demangleをRust adapterへ移す。

完了条件：

- M4、M7 acceptanceが全PASS。
- build failureでEvidenceを記録しない。
- target変更でEvidenceがSTALEになる。
- llvm-cov不在はNOT_CHECKEDのままである。
- `vtest-exec`にCargo / llvm-cov / rustc-demangle固有処理が残らない。

### W6 CLI / MCP / verify統合

成果：単一registry composition、adapter-aware JSON、CLI/MCP parity。

完了条件：

- M6、M9 acceptanceが全PASS。
- 全MCP toolがCLIと同じregistryとenvelopeを使う。
- adapter選択失敗のcode / message / candidatesがCLI/MCPで一致する。
- reportはadapter metadataを根拠として表示できるが、scope外をPASSにしない。

### W7 synthetic adapterによる境界証明

成果：テスト専用`synthetic` adapterとmixed-adapter fixture。

synthetic adapterはRust parser、Cargo、llvm-covを使わず、fixture内の宣言ファイルから
Test / Sourceを返し、固定のrunner observationを生成する。

完了条件：

- Rust以外のadapter実装が`vtest-model`変更なしで登録できる。
- RustとsyntheticのTestを1回のscanでmergeできる。
- duplicate Test IDをE-SCAN系またはE-ADAPTER系errorで拒否する。
- synthetic runnerのEvidenceがhash変更後にSTALEになる。
- coverage capabilityなしでtarget_executionがNOT_CHECKEDになる。
- 全11項目で非PASSがaggregate NGになる。

### W8 cleanupとリリースゲート

成果：禁止依存の除去、文書同期、配布plugin更新、最終review。

完了条件：

- 全既存M1〜M9とadapter acceptanceがPASS。
- project / plugin Skillがbyte-identicalでvalid。
- MCPはCLI parity完了後のみenabled。
- legacy fieldsはdeprecatedだが読取り・JSON互換を維持する。
- architecture-check、verify-change、release-check、独立reviewerがPASS。

## 8. Luna max 作業パッケージ

### 8.0 実行モデル

- production実装を担当するP2〜P11は`gpt-5.6-luna`、reasoning effort `max`を使う。
- 各agentは`fork_turns: "none"`で起動し、依頼文だけで作業境界を完結させる。
- explorer / reviewer / testerはread-onlyまたはテスト専任の既存profileを利用できるが、
  production実装をそれらの低いeffort設定へ黙って置換しない。
- 現在のrepositoryにはLuna max実装worker profileがないため、実装開始前に主担当が
  orchestration環境の対応可否を確認し、必要なら主担当所有でprofileを追加する。
- Luna maxを指定できない場合は別モデルへ自動代替せず、`NOT_EXECUTED`として
  ownerへ返す。

### 8.1 共通ルール

すべての依頼文に次を含める。

- あなたはコードベース内で単独ではない。他agentの変更をrevertしない。
- 指定した所有ファイル以外を編集しない。
- 共有API変更が必要なら編集せず主担当へ返す。
- canonical Audit / Evidence / Approvalを更新・削除しない。
- `.verify/cache/`を正典として編集しない。
- PASS以外をPASSへ昇格しない。
- 仕様矛盾を実装で解消しない。
- 返却時に変更ファイル、実行コマンド、FAIL / NOT_CHECKED / UNKNOWNを列挙する。

各waveの同時実行は最大3 Lunaとし、主担当が共有ファイルと統合を所有する。

### 8.2 所有マトリクス

| Package | Luna role | 排他的所有 | 依存 | 完了条件 |
|---|---|---|---|---|
| P0 | explorer | read-only: docs、全crate | なし | coupling mapと仕様tensionを返す |
| P1 | reviewer | read-only: W0仕様差分 | P0 | fail-closed / compatibility review |
| P2 | worker | `crates/vtest-adapter-api/**` | W0 | registry / capability tests PASS |
| P3 | worker | `crates/vtest-model/src/lib.rs` | P2 API freeze | additive neutral types、model tests PASS |
| P4 | worker | `crates/vtest-store/**` | P2/P3 | v1/v2 config、store tests PASS |
| P5 | worker | `crates/vtest-adapter-rust/src/discovery.rs`, `operations.rs` | P2〜P4 | Rust scan/ops adapter tests PASS |
| P6 | worker | `crates/vtest-scan/**` | P5 API freeze | orchestration化、M1/M2/M8 PASS |
| P7 | worker | `crates/vtest-adapter-rust/src/audit.rs`, `crates/vtest-audit/**` | P5 | M3/M5 PASS |
| P8 | worker | `crates/vtest-adapter-rust/src/execution.rs`, `crates/vtest-exec/**` | P5 | M4/M7 PASS |
| P9 | worker | `crates/vtest-verify/**` | P3/P6/P8 | M6、freshness、aggregation PASS |
| P10 | worker | `crates/vtest-cli/src/**` | P4〜P9 | CLI registry integration PASS |
| P11 | worker | `crates/vtest-mcp/**` | P10 public API freeze | 全MCP parity PASS |
| P12 | tester | `tests/fixtures/adapters/**`, new `adapter_acceptance.rs` | P2 API freeze | synthetic/mixed fixtures PASS |
| P13 | reviewer | read-only: 全差分 | P2〜P12 | independent fail-closed review |

### 8.3 主担当だけが編集する共有ファイル

- workspace `Cargo.toml` / `Cargo.lock`
- `crates/vtest-adapter-rust/src/lib.rs`
- 要件定義・基本仕様・詳細設計・別紙
- `AGENTS.md`
- `tests/ACCEPTANCE.md`
- `.codex/` と plugin distribution

主担当はmodule skeletonとpublic interfaceを先に確定し、その後にLunaへ排他的な
module fileを割り当てる。複数Lunaへ同じcrate rootや同じacceptance fileを渡さない。

### 8.4 推奨ウェーブ配置

```text
Wave A: P0 + P1（read-only、仕様確定支援）
Wave B: P2 -> P3 / P4 / P12
Wave C: P5 -> P6 / P7 / P8
Wave D: P9 / P10 -> P11
Wave E: P12完成 -> P13 -> 主担当release gate
```

矢印の前後は逐次である。同じwave内の`/`は排他的所有が成立する場合のみ並列化する。

## 9. 必須受入テスト

### 9.1 compatibility matrix

| Case | Expected |
|---|---|
| v1 config + Rust fixture | v0.1と同じscan/audit/run/verify |
| v2 config + rust-cargo | v1と意味的に同値 |
| unknown adapter | E-ADAPTER-001、record writeなし |
| duplicate adapter ID | usage error、scanなし |
| adapter discovery failure | error、空scanを正常扱いしない |
| adapter without audit | static_audit非PASS |
| adapter without runner | Evidenceなし、NOT_EXECUTED |
| adapter without coverage | W-ADAPTER-101、NOT_CHECKED |
| Rust + synthetic | 決定論的merge |
| duplicate Test ID across adapters | error |
| stale synthetic Evidence | STALE |
| limited scope | scope外NOT_CHECKED |
| CLI / MCP same input | 同じJSON envelope |

### 9.2 architecture tests

- `cargo metadata`から禁止依存を検査する。
- `vtest-adapter-api`にRust固有crateがない。
- `vtest-scan`、`vtest-audit`、`vtest-exec`に`syn`、`quote`、
  `rustc-demangle`の直接依存がない。
- Rust固有built-in formはRust adapterだけが登録する。
- adapterを0件にしたscanがfail-closedになる。
- adapter順やfilesystem列挙順に出力が依存しない。

### 9.3 共通コマンド

```powershell
cargo fmt --all -- --check
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo run --quiet -p vtest-cli -- doctor
```

変更種別に応じて、各M1〜M9 acceptance、`vtest audit static`、bundle / submit、
`vtest run`、`vtest verify`、MCP stdio parityを追加実行する。

## 10. 完了判定

次のすべてを満たしたときだけadapter分離を完了とする。

- Rust v0.1の受入基準がすべて再現できる。
- M9がDONEでありCLI/MCP parityが維持される。
- `vtest-model`のコア判断がCargo fieldに依存しない。
- scan / audit / exec orchestrationがRust parser / runnerを直接所有しない。
- synthetic adapterがコアcrate変更なしに登録・scan・runできる。
- missing capabilityがPASSへ昇格しない。
- 既存canonical recordsとEvidenceを読める。
- config v1を黙って書き換えない。
- Structured Editの1 Test境界が維持される。
- project / plugin distributionが同期する。
- `$verify-change`、`$architecture-check`、`$release-check`がPASSする。
- 独立Luna reviewerの全確定指摘が解消または明示的review itemになる。

## 11. 後続言語マイルストーン

adapter分離完了後、各言語を別マイルストーンとして順番に追加する。

### L1 TypeScript / JavaScript

最初から全runnerを扱わず、1 runnerを選ぶ。候補はVitestであり、Node built-in test、
Jest等は別adapterまたは別capability profileとする。source mapとtranspile後coverageの
hash bindingを仕様化してから実装する。

### L2 Go

`go test`、package selector、`TestXxx` discovery、subtestのTest ID表現、coverprofileの
target attributionを仕様化する。

### L3 C#

`.csproj` / solution、xUnit / NUnit / MSTestのうち最初のframework、fully qualified
test selector、Coverlet等のcoverage providerを仕様化する。

各言語で discovery、static audit、execution、coverage、Structured Test Operationの
提供範囲は独立に宣言する。全capabilityがないadapterを「完全対応」と表示しない。
