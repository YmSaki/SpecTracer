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
- 複数targetを宣言し、targetごとにPASS / FAIL / UNKNOWNが異なるintegration Test
- PASS、FAIL、MISMATCH、MISSING、NOT_CHECKED、NOT_EXECUTED、STALE、UNKNOWNを生じる入力
- Testまたはtargetのhash変更によって無効になるAudit / Evidence
- Specification sourceまたは上流REQの変更によって無効になるspec-coverage Audit / Approval
- Specificationに要求事項が存在するが対応active REQが欠落する状態
- Test constructと非隣接のmetadata宣言だけを変更した状態
- 複数adapterが同じ恒久SRC IDを宣言する状態

adapter境界fixtureは、Rust parser、Cargo、llvm-covを使用しないin-process synthetic
adapterを使用できる。synthetic adapterは配布対象のproduction language adapterではない。
synthetic fixtureは`.rs`以外のsource、関数ではないTest construct、doc commentではないmetadata宣言、Rust item pathではないopaque locatorを使用する。

### 18.3 機能別受入条件

#### 18.3.1 discovery・record・graph

- source discovery adapterは全Discovered Test draft、ManagedTestDraftLink、SourceTargetDraft、Source Location、source range、current bytes、logical metadata、Test execution descriptorをhash未計算で返す。coreは出力を検証してTest subject / Source Target hashを計算してからManaged Test Entity、ManagedTestLink、Source Targetを具体化する。
- adapter所有のmetadata宣言、ID、target、VO参照、record schema、Relationの違反を対応診断codeで検出する。
- 管理宣言または必須metadataを持たないTestが1件でもあれば、W-SCAN-101またはE-SCAN-007を表示し、`ManagedTestLink::Missing`から`test_traceability = MISSING`を導出する。
- 存在しないVOを`covers`するTestは構造上完全なManaged Test Entityと`ManagedTestLink::One`のまま保持し、E-SCAN-003と`test_traceability = MISMATCH`を導出する。`MISSING`として二重定義しない。
- `ManagedTestLink::Multiple`またはTest ID衝突は`test_traceability = MISMATCH`になる。
- 全Discovered Testが`ManagedTestLink::One`で構造上完全なentityへ1対1で対応し、Test IDが一意、`covers`が1件以上、かつ全VO参照を解決できる場合だけ`test_traceability = PASS`になる。
- W-SCAN-101のwarning severityだけを理由に検証値を変更せず、Discovered Testとmanaged entityの対応事実から判定する。
- adapter discoveryの失敗をTest 0件の正常scanとして扱わない。
- SPEC sourceの内容hash不一致をW-SCAN-104として検出する。
- active REQは1件以上の解決可能なSPEC sectionを参照し、Specification → REQ edgeを構築する。
- Relation IDは`REL-<ULID>`であり、bare ULIDをRelation IDとして受理しない。
- VO writerは`status`を保存せず、実効値をApprovalから導出する。読取り互換field `status`は警告して無視する。
- VOの承認はVO内容hashと現在の上流依存closureへ束縛され、SPEC / REQ / parent VOの内容または集合が不一致の承認を有効として扱わない。
- Approval作成時に対象または上流依存closureを完全・currentに解決できなければE-APPROVAL-001で拒否し、recordを生成しない。
- dependenciesを欠く互換Approvalを現在のapprovedへ昇格しない。
- 恒久SRC IDは全adapter統合後にrepository全体で一意であり、衝突をE-SCAN-011として拒否する。
- `vtest scan` / `doctor`はE-ADAPTER-*による操作拒否をexit 2、完了したscanのE-SCAN-*をexit 1、errorなしをexit 0にする。
- `full-product` VOは宣言partitionの直積を決定論的に実体化する。

#### 18.3.2 deterministic static audit

- DA-001〜DA-006とW-DA-101は本冊 §7の判定条件に従う。
- 確定違反だけをFAILとし、解析限界をUNKNOWNとして保持する。
- 正常Testは違反なしとなり、各違反fixtureは対応ruleで非PASSになる。
- Audit Recordは対象Test、全宣言target、configの現在hashへ束縛される。

#### 18.3.3 execution・Evidence

- 選択した登録Testだけをrunnerのexact selectorで実行する。
- Testごとの結果、revision、hash、adapter ID、runner情報をEvidenceへ記録する。
- build failure、runner failure、必須runner capabilityの欠落ではEvidenceを生成しない。
- Evidence writerはadapter IDを必ず記録する。
- Evidence writerは中立fieldの`hashes.test_subject`と`hashes.targets[].target_construct`を出力する。`test_fn` / `test_construct` / `target_fn`の互換入力は`rust-cargo` Evidenceで全canonical metadataを含むsource rangeと現在値の同一性を証明できる場合だけ受理する。
- Evidence readerはadapter IDを欠くrecordについて、現在のTestが `rust-cargo` で、
  runner kindと内容hashからRust実行を一意に確認できる場合だけ互換Evidenceとして扱う。
- Evidenceは全宣言targetの参照と内容hashを重複なく保持する。
- canonical Test metadata、ExecutionDescriptor、Test construct、宣言target集合、またはいずれかのtarget内容hashがEvidenceと異なる場合はSTALEになる。
- 単数互換形のEvidenceは、現在のTestがtargetをちょうど1件持つ場合だけ有効性を評価できる。複数target Testでは有効なPASSにしない。
- Evidenceのadapter IDがTest execution adapterと異なる場合はMISMATCHになる。

#### 18.3.4 semantic audit protocol

- spec-coverage、test-semantic、vo-coverage、impl-consistencyのbundleは本冊 §8の必須情報を含む。
- spec-coverageは対象SPEC sourceと、それを参照するactive REQの完全集合へ束縛され、要求事項の取り込み、対応REQ、exclusion根拠を含む場合だけ受理される。
- 空のreasons、schema違反、kind不一致、stale bundle hashを拒否する。
- 受理するAudit Recordはsubjectsの内容hashへ束縛される。
- deterministic結果とagent / human結果を区別して保存・表示する。

#### 18.3.5 verify・report

- 完全検証は基本仕様 §4.2の12項目をすべて評価し、各項目の非PASSを総合NGへ反映する。
- 完全検証は全12項目がPASSの場合だけOKとする。
- `spec_coverage`は登録Specificationの要求事項がactive REQへ完全に取り込まれたことを有効なspec-coverage監査で確認した場合だけPASSとする。active REQの存在だけ、またはREQ → VO対応の存在だけでPASSにしない。
- 登録Specificationが0件の完全検証を`spec_coverage = MISSING`とし、空集合をPASSにしない。
- `vo_decomposition`はREQ / VOのparent、requirements、spec_refs、構造Relationだけを評価し、Test metadata、target、adapter parse、Evidenceのerrorによって値を変更しない。
- 完全検証fixtureで12項目のそれぞれを単独で非PASSにすると総合NGになる。
- 管理済みgraph側の11項目がすべてPASSでも、未登録Testが1件あれば`test_traceability`により総合NGになる。
- 要求scope内の`FAIL`、`MISMATCH`、`MISSING`、`NOT_CHECKED`、`NOT_EXECUTED`、`STALE`、`UNKNOWN`のいずれも総合PASSへ昇格しない。
- 限定scopeは要求項目だけを集約し、scope外をNOT_CHECKEDのまま表示する。
- 限定scopeは要求された項目・entityがすべてPASSなら「要求scope内のOK」とし、完全検証OKとは表示しない。
- reportはSPEC → REQ → VO → Testの構造と、各非PASSの根拠をtext / JSONで返す。
- text treeのancestor continuation、middle child、last childを一意なbranch記号で描画する。

#### 18.3.6 Target Execution Verification

- 各宣言targetについて、計測countが1以上ならtarget別PASS、0ならtarget別FAIL、確実に同定または計測できなければtarget別UNKNOWNになる。
- 複数target Testの集約値は、1件でもtarget別FAILがあればFAIL、FAILがなく1件でもUNKNOWNがあればUNKNOWN、1件以上の全宣言targetがPASSの場合だけPASSになる。
- target AがPASSでもtarget BがFAILまたはUNKNOWNなら、Test単位の`target_execution`をPASSにしない。
- `target_execution.checked: true`のEvidenceでtarget別entryが欠落、重複、または宣言target集合と不一致ならPASSにしない。
- coverage capabilityまたは計測toolが利用できない場合はNOT_CHECKEDとなり、PASSにならない。
- coverage解析限界はUNKNOWNとなり、PASSにならない。

#### 18.3.7 Structured Test Operation

- Form Schemaの必須値と未知fieldを常に検証する。symbol、VO / Test参照、identifier、pathは選択したFormが該当fieldとvalidatorを宣言した場合だけ検証し、すべてのadapterへ一律に要求しない。
- create結果はscanで同じTest ID・intent・covers・targetsとして認識される。
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
- `vtest-model::TestEntity`はTestを関数として表現せず、adapter所有のTest constructを論理metadata、Source Location、content hash、ExecutionDescriptorで表現する。
- `TargetRef::Locator`はadapter IDとadapter所有のopaque locatorを保持する。`SourceLocation`はadapter ID、project-relative path、opaque locator、source rangeを保持する。どちらもRust module path、関数名、`.rs`拡張子をcoreの不変条件にしない。
- `vtest-model::TestEntity`は`ExecutionDescriptor`だけを実行座標として持ち、`filter`、`package`、`test_target`、`TestTarget`を含まない。
- `TestEntity.content_hash`はTest constructだけでなくcanonical metadata、locationのadapter・path・opaque locator、ExecutionDescriptorを含むTest subjectへ束縛される。byte range自体は含めず、非隣接metadataだけの意味変更でもhashが変化する。
- `SourceDiscoveryAdapter`はhash未計算DTOを返し、coreがDTO検証・hash計算・domain entity具体化をこの順で行う。
- `rust-cargo` adapterはRust discovery、static audit、Structured Test Operation、runner、coverageを所有する。
- `vtest-scan`はadapter discoveryの委譲・出力検証・決定論的統合・core record整合性を所有し、`*.rs`列挙、`syn::parse_file`、`#[test]`抽出、doc comment parseを所有しない。
- registryはadapter IDの重複、宣言capabilityと実装の不一致、未登録adapterを拒否する。
- 異なるadapterが同じrootを共有でき、同一adapter内のroot重複は拒否される。
- 全adapterのmerge結果でTest IDのglobal uniquenessを検査する。
- config readerはversion 1とversion 2を受理し、読み取りだけでconfigを書き換えない。
- config writerと`vtest init`はversion 2のadapter namespaceを出力する。
- Test JSON writerは`execution`を常に出力し、`rust-cargo` Testについてだけwire codecが互換field `filter` / `package` / `test_target`を追加する。
- Test JSON writerは1件以上の`targets` listを常に出力し、targetが1件の場合だけ同値の単数互換field`target`を追加できる。複数targetを単数fieldへ縮約しない。
- synthetic TestのJSONはRust互換fieldを省略し、空値またはdummy値を出力しない。
- `execution`を欠くTest入力は、`rust-cargo` codecが完全で相互整合するRust互換fieldからだけdescriptorを導出する。
- `execution`とRust互換fieldが矛盾する入力を拒否する。
- 明示操作に必須のcapabilityがなければE-ADAPTER-004となり、変更・Audit・Evidenceを生成しない。
- 検証時のstatic audit / coverage capability欠落はNOT_CHECKED、runner欠落はNOT_EXECUTED、解析限界はUNKNOWNになる。
- Rustとsyntheticの結果をadapter ID、path、Test IDで決定論的に統合する。
- synthetic adapterは`.rs`以外のsource、関数ではないTest construct、doc commentではないmetadata宣言、Rust item pathではないopaque locatorを、`vtest-model`、`vtest-scan`、`vtest-verify`の変更なしで登録・scan・verifyできる。

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
