# AI並列開発向けテスト検証システム 詳細設計 別紙C 受入仕様 v0.1

本冊 §0 の分冊構成に基づき、本別紙は §18 を収録する。

---

## 18. 受入契約

### 18.1 共通条件

- 受入条件は決定論的なfixtureと統合テストで再現できる。
- Rust workspaceの受入テストは `cargo test --workspace` で実行できる。
- 検証結果はfail-closedであり、要求scopeに1件でも非PASSがあれば総合結果はNGになる。
- scopeを限定してもscope外の値をPASSへ変更しない。
- CLIとMCPは同じcore処理、adapter registry、JSON envelope、診断codeを使用する。
- canonical record、Approval、Audit、Evidence、内容hashの不変条件をfixtureの都合で緩和しない。

### 18.2 共通fixture

Rustの受入fixtureは、SPEC / REQ / VO、登録Test、Source Target、Approval、Audit、
Evidenceを含む小規模projectとする。fixtureは少なくとも次を表現できる。

- 正しいannotationを持つTest
- `assert!(true)`だけのTest
- 宣言targetを呼ばないTest
- 結果を検証しないTest
- 自己比較を行うTest
- annotationを持たないtest function（W-SCAN-101、`test_traceability = MISSING`）
- 空のcoversを持つTest（`test_traceability = MISSING`）
- 存在しないVOを参照するTest（E-SCAN-003、`test_traceability = MISMATCH`）
- Test IDが衝突するTest（E-SCAN-002、`test_traceability = MISMATCH`）
- `@vtest.case`を持つtable-driven Test
- PASS、FAIL、MISMATCH、MISSING、NOT_CHECKED、NOT_EXECUTED、STALE、UNKNOWNを生じる入力
- Testまたはtargetのhash変更によって無効になるAudit / Evidence

adapter境界fixtureは、Rust parser、Cargo、llvm-covを使用しないin-process synthetic
adapterを使用できる。synthetic adapterは配布対象のproduction language adapterではない。

### 18.3 機能別受入条件

#### 18.3.1 discovery・record・graph

- source discoveryは全Discovered Testと、有効なManaged Test Entity、Source Target、位置、内容hash、Test execution descriptorを区別して抽出する。
- annotation、ID、target、VO参照、record schema、Relationの違反を対応診断codeで検出する。
- annotationを持たないTestが1件でもあれば、W-SCAN-101を表示し、同じ事実を`test_traceability = MISSING`としてfull verificationへ反映する。
- 全Discovered Testがmanaged entityへちょうど1件対応し、coversが1件以上かつ全VO参照を解決できる場合だけ`test_traceability = PASS`になる。
- W-SCAN-101のwarning severityだけを理由に検証値を変更せず、Discovered Testとmanaged entityの対応事実から判定する。
- adapter discoveryの失敗をTest 0件の正常scanとして扱わない。
- SPEC sourceの内容hash不一致をW-SCAN-104として検出する。
- VOの承認はVO内容hashに束縛され、不一致の承認を有効として扱わない。
- `full-product` VOは宣言partitionの直積を決定論的に実体化する。

#### 18.3.2 deterministic static audit

- DA-001〜DA-006とW-DA-101は本冊 §7の判定条件に従う。
- 確定違反だけをFAILとし、解析限界をUNKNOWNとして保持する。
- 正常Testは違反なしとなり、各違反fixtureは対応ruleで非PASSになる。
- Audit Recordは対象Test、target、configの現在hashへ束縛される。

#### 18.3.3 execution・Evidence

- 選択した登録Testだけをrunnerのexact selectorで実行する。
- Testごとの結果、revision、hash、adapter ID、runner情報をEvidenceへ記録する。
- build failure、runner failure、必須runner capabilityの欠落ではEvidenceを生成しない。
- Evidence writerはadapter IDを必ず記録する。
- Evidence readerはadapter IDを欠くrecordについて、現在のTestが `rust-cargo` で、
  runner kindと内容hashからRust実行を一意に確認できる場合だけ互換Evidenceとして扱う。
- Testまたはtargetの内容hashがEvidenceと異なる場合はSTALEになる。
- Evidenceのadapter IDがTest execution adapterと異なる場合はMISMATCHになる。

#### 18.3.4 semantic audit protocol

- test-semantic、vo-coverage、impl-consistencyのbundleは本冊 §8の必須情報を含む。
- 空のreasons、schema違反、kind不一致、stale bundle hashを拒否する。
- 受理するAudit Recordはsubjectsの内容hashへ束縛される。
- deterministic結果とagent / human結果を区別して保存・表示する。

#### 18.3.5 verify・report

- 基本仕様 §4.2の12項目をすべて評価し、各項目の非PASSを総合NGへ反映する。
- 全12項目がPASSの場合だけ要求scopeをOKとする。
- 12項目のそれぞれについて、その項目だけを非PASSにしたfixtureが総合NGになる。
- 管理済みgraph側の11項目がすべてPASSでも、未登録Testが1件あれば`test_traceability`により総合NGになる。
- `FAIL`、`MISMATCH`、`MISSING`、`NOT_CHECKED`、`NOT_EXECUTED`、`STALE`、`UNKNOWN`のいずれも総合PASSへ昇格しない。
- 限定scopeは要求項目だけを集約し、scope外をNOT_CHECKEDのまま表示する。
- reportはREQ → VO → Testの構造と、各非PASSの根拠をtext / JSONで返す。
- text treeのancestor continuation、middle child、last childを一意なbranch記号で描画する。

#### 18.3.6 Target Execution Verification

- 宣言targetを実行したTestは計測countが1以上でPASSになる。
- PASSしたTestでも宣言targetの計測countが0ならFAILになる。
- coverage capabilityまたは計測toolが利用できない場合はNOT_CHECKEDとなり、PASSにならない。
- coverage解析限界はUNKNOWNとなり、PASSにならない。

#### 18.3.7 Structured Test Operation

- Form Schemaの必須値、未知field、symbol、VO / Test参照、identifier、pathを検証する。
- create結果はscanで同じTest ID・intent・covers・targetとして認識される。
- editは1 Testの拡張rangeだけを単一置換し、他Testと通常sourceを変更しない。
- 同じdesired stateの再適用は冪等になる。
- Structured Test capabilityがないadapterへのcreate / editはE-ADAPTER-004となり、ファイルを変更しない。

#### 18.3.8 MCP interface

- 別紙A §13の全toolが同じ入力に対するCLI JSONと同じdata / diagnosticsを返す。
- 不正入力はcode / message / candidatesを持つtool errorになる。
- request、notification、batch、malformed transportの各入力をJSON-RPC contractどおりに処理する。
- MCP serverの長時間実行中もsource変更を再scanし、staleなPASSを保持しない。

#### 18.3.9 adapter contract

- `vtest-adapter-api`は言語・runner非依存であり、Cargo、Rust parser、llvm-cov固有型を公開しない。
- `vtest-model::TestEntity`は`ExecutionDescriptor`だけを実行座標として持ち、`filter`、`package`、`test_target`、`TestTarget`を含まない。
- `rust-cargo` adapterはRust discovery、static audit、Structured Test Operation、runner、coverageを所有する。
- registryはadapter IDの重複、宣言capabilityと実装の不一致、未登録adapterを拒否する。
- 異なるadapterが同じrootを共有でき、同一adapter内のroot重複は拒否される。
- 全adapterのmerge結果でTest IDのglobal uniquenessを検査する。
- config readerはversion 1とversion 2を受理し、読み取りだけでconfigを書き換えない。
- config writerと`vtest init`はversion 2のadapter namespaceを出力する。
- Test JSON writerは`execution`を常に出力し、`rust-cargo` Testについてだけwire codecが互換field `filter` / `package` / `test_target`を追加する。
- synthetic TestのJSONはRust互換fieldを省略し、空値またはdummy値を出力しない。
- `execution`を欠くTest入力は、`rust-cargo` codecが完全で相互整合するRust互換fieldからだけdescriptorを導出する。
- `execution`とRust互換fieldが矛盾する入力を拒否する。
- 明示操作に必須のcapabilityがなければE-ADAPTER-004となり、変更・Audit・Evidenceを生成しない。
- 検証時のstatic audit / coverage capability欠落はNOT_CHECKED、runner欠落はNOT_EXECUTED、解析限界はUNKNOWNになる。
- Rustとsyntheticの結果をadapter ID、path、Test IDで決定論的に統合する。

### 18.4 提供範囲外

- GUI
- 仕様書同士の矛盾判定
- 仕様・Test・実装のどれを変更すべきかという修正方針の決定
- helper、fixture、通常sourceの編集管理
- 開発process管理
- `rust-cargo`以外のproduction language adapter
- third-party plugin ABI
- LSP統合
- runner / coverage providerの自動選択または推測fallback
