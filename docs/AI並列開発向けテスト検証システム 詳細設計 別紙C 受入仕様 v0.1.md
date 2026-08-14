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
- Test / targetを変更せず、static auditが参照した同一file helperだけを変更した状態
- Test / 宣言targetを変更せず、実行結果を変えうるtarget外helperまたはlocal dependencyだけを変更した状態
- VO / Test / targetを変更せず、impl-consistencyが参照するSpecification sourceだけを変更した状態
- 複数adapterが同じ恒久SRC IDを宣言する状態
- 同一のSource Targetを、一方のTestがlocatorで、他方のTestが恒久SRC IDで宣言する状態
- 同一のTestが同一Source Targetをlocatorと恒久SRC IDの両方で宣言する状態（E-SCAN-005）
- Source Target constructの内側にある`@vtest.src-id`宣言だけを付与・変更・削除した状態

adapter境界fixtureは、Rust parser、Cargo、llvm-covを使用しないin-process synthetic
adapterを使用できる。synthetic adapterは配布対象のproduction language adapterではない。
synthetic fixtureは`.rs`以外のsource、関数ではないTest construct、doc commentではないmetadata宣言、Rust item pathではないopaque locatorを使用する。

### 18.3 機能別受入条件

#### 18.3.1 discovery・record・graph

- source discovery adapterは全Discovered Test draft、ManagedTestDraftLink、SourceTargetDraft、Source Location、source range、current bytes、logical metadata、宣言された恒久SRC ID、Test execution descriptorをhash未計算で返す。coreは出力を検証してTest subject / Source Target hashを計算してからManaged Test Entity、ManagedTestLink、Source Targetを具体化する。
- Source Targetはcanonical locatorと任意の恒久SRC IDを併有する単一のentityである。adapterは同一constructをlocator版とSrcId版の2 draftへ複製せず、恒久SRC IDを`SourceTargetDraft.src_id`として返す。
- 恒久SRC IDを持つSource Targetはcanonical locatorでもaddressableであり、locator参照とSRC ID参照は同一のcanonical Source Targetへ解決する。両addressing modeで同一のSource Target hashに到達し、Source Targetの件数、content / subject hash、EvidenceおよびAudit上のtarget identityが参照方法によって分裂しない。
- Source Target identityは「宣言された`TargetRef` → 解決 → canonical Locator」の一方向で確定する。Evidence、監査レコード、`target_execution`、鮮度判定は解決後のcanonical Locatorをidentityとして記録・比較し、参照側Testが宣言した綴り（SRC ID参照を含む）を保存しない。同一のSource Targetをlocator参照するTestとSRC ID参照するTestは、Evidence / Audit上で同一のtarget identityを持つ。
- Testがどう宣言したかの変更（同一Source Targetに対するlocator参照からSRC ID参照への書き換え等）はTest subject hashの変化として捕捉され、Evidence / Audit側のtarget identityを変化させない。
- 綴りの異なる複数の`target`宣言が同一のcanonical Source Targetへ解決する場合はE-SCAN-005とする。
- `SourceTargetDraft.target`は必ず`TargetRef::Locator`である。`TargetRef::SrcId`をcanonical targetとして返したadapter出力はmalformed adapter outputとして拒否する。恒久SRC IDの宣言・変更・削除でcanonical locatorは変化しない。
- Source Target hashは常にcanonical locatorとconstruct bytesから計算し、参照側Testの`TargetRef`綴りからは計算しない。恒久SRC IDを独立したhash fieldとしてSource Target hashのinputに含めない。
- 恒久SRC IDの宣言をSource Target constructの内側へ置くadapter（`rust-cargo`の`@vtest.src-id` doc comment等）では、その宣言の付与・変更・削除がconstruct bytesを変えるため、Source Target hashも変化する。これはsourceが実際に変化したことの帰結として正しい挙動であり、恒久SRC IDが独立したhash fieldであることを意味しない。
- SRC ID参照はcoreの統合済みSRC索引から、その恒久SRC IDを宣言したSource Targetのcanonical locatorへ解決する。
- Target Reference解決は解決済み / 対象なし / 曖昧を区別し、曖昧はfail-closedな終端状態とする。E-SCAN-004またはE-SCAN-011で曖昧・未解決となったtargetについて、監査subject、Evidence、`target_execution`のいずれも候補の1件を解決結果として記録しない。候補は診断表示にだけ用いる。
- この禁止は解決に関するものであり、Source Targetの具体化を止めない。恒久SRC IDが衝突していても、各Source Targetは自身のcanonical locatorで独立したentityとして具体化され、Source Targetの件数と各content / subject hashは衝突の有無で変化しない。衝突が壊すのは当該恒久SRC IDによる参照の一意性だけである。
- この解決はcoreの単一経路が所有する。discovery、静的監査、実行、Evidence writer、検証集約が独自にcandidate列を走査して1件を選ぶ経路を持たない。
- adapter所有のmetadata宣言、ID、target、VO参照、record schema、Relationの違反を対応診断codeで検出する。
- 管理宣言または必須metadataを持たないTestが1件でもあれば、W-SCAN-101またはE-SCAN-007を表示し、`ManagedTestLink::Missing`から`test_traceability = MISSING`を導出する。
- 存在しないVOを`covers`するTestは構造上完全なManaged Test Entityと`ManagedTestLink::One`のまま保持し、E-SCAN-003と`test_traceability = MISMATCH`を導出する。`MISSING`として二重定義しない。
- `ManagedTestLink::Multiple`またはTest ID衝突は`test_traceability = MISMATCH`になる。
- 全Discovered Testが`ManagedTestLink::One`で構造上完全なentityへ1対1で対応し、Test IDが一意、`covers`が1件以上、かつ全VO参照を解決できる場合だけ`test_traceability = PASS`になる。
- W-SCAN-101のwarning severityだけを理由に検証値を変更せず、Discovered Testとmanaged entityの対応事実から判定する。
- adapter discoveryの失敗をTest 0件の正常scanとして扱わない。
- SPEC sourceの内容hash不一致をW-SCAN-104として検出する。
- active REQは1件以上の`spec_refs`を持ち、各`spec`はcurrentなSPEC record / sourceへ解決し、各`section`は非空のopaque citationとして保持され、Specification → REQ edgeを構築する。coreは任意形式の本文からsection存在を推測せず、citationの意味的妥当性を監査理由で確認する。
- Relation writerは`REL-<ULID>`だけを生成する。readerはファイル名とrecord IDが同じbare ULIDのversion 1互換Relationを読み取り、in-memoryで`REL-<ULID>`へ正規化するが、ファイルを書き換えない。同じpayloadのbare / prefixed重複、混在形、ファイル名とIDの不一致はE-SCAN-010になる。
- VO writerは`status`を保存せず、実効値をApprovalから導出する。読取り互換field `status`は警告して無視する。
- VOの承認はVO内容hashと現在の上流依存closureへ束縛され、SPEC / REQ / parent VOの内容または集合が不一致の承認を有効として扱わない。
- Approval作成時に対象または上流依存closureを完全・currentに解決できなければE-APPROVAL-001で拒否し、recordを生成しない。
- dependenciesを欠く互換Approvalを現在のapprovedへ昇格しない。
- 恒久SRC IDは全adapter統合後にrepository全体で一意であり、衝突をE-SCAN-011として拒否する。
- `vtest scan` / `doctor`はE-ADAPTER-* / E-CONFIG-*による操作拒否をexit 2、完了したscanのE-SCAN-*をexit 1、errorなしをexit 0にする。
- `full-product` VOは宣言partitionの直積を決定論的に実体化する。

#### 18.3.2 deterministic static audit

- DA-001〜DA-006とW-DA-101は本冊 §7の判定条件に従う。
- 確定違反だけをFAILとし、解析限界をUNKNOWNとして保持する。
- 正常Testは違反なしとなり、各違反fixtureは対応ruleで非PASSになる。
- Audit Recordは対象Test、全宣言target、adapter ID・static rule-set・rule影響config projectionからなるStatic Audit Config subject、および判定時に実際に参照したhelper等のStatic Analysis Source subject完全集合の現在hashへ束縛される。
- `rust-cargo`の`assertion_macros`だけを変更すると既存static Audit RecordはSTALEになり、再監査なしに`static_audit = PASS`へ利用されない。
- DA-002 / DA-003が参照した同一file helperだけを変更すると既存static Audit RecordはSTALEになり、再監査なしに`static_audit = PASS`へ利用されない。
- static audit adapterが判定へ使用したsource fragment集合の完全性を保証できない場合、当該判定はUNKNOWNとなりPASSにならない。
- target-scopedなDA-002 / DA-003は宣言targetごとのverdictを監査レコードへ正典として保存し（本冊 §3.6）、その集合は全宣言targetと過不足なく1対1に対応する。規則単位verdictはこのtarget別verdictのfoldで導出する。target entryの欠落・重複・余剰、宣言target集合との不一致、またはtarget別verdictと規則単位verdictのfold不整合はmalformed recordとし、現在の`static_audit`へ有効なPASSを供給しない。
- target別verdictを持たない読取り互換static Audit Recordはrule-set version相違によりSTALEとなり、現在のPASSへ昇格しない。
- 別プロセス・別スレッド・クロージャ・他ファイル等、静的解析の到達境界を越えてtargetを実行するTestはDA-002がUNKNOWNになる（本冊 §7.3）。当該targetのruntime target_executionがPASS（checked: true・count > 0）なら到達要件は充足され、DA-002はstatic_audit集約へUNKNOWNを寄与しない。他ルールが違反なしならstatic_auditはPASSになる。
- 同じ到達UNKNOWNのTestでも、当該targetのtarget_executionがFAIL・UNKNOWN・NOT_CHECKED（coverage利用不能・未計測・`--fast`）なら到達要件は未充足で、DA-002 UNKNOWNはstatic_auditのUNKNOWNとして残る。DA-002 FAIL（境界内で到達を静的否定）はruntime証明で覆らない。
- runtime coverageはDA-003を代替しない。結果検証はDA-003の静的判定（結果がassert相当へ到達）のまま評価し、到達がruntimeで充足されてもDA-003 UNKNOWN / FAILはそのままstatic_auditへ寄与する。
- 宣言targetをどのtopologyでも実行しない構造・契約のみのTestは、静的にもruntimeにも到達を確立できず到達要件は未充足のままになる。
- 同じ入力に対するverdictまたは根拠を変えるstatic rule実装変更はrule-set versionを変更し、既存recordをSTALEにする。
- static ruleへ影響しないrun / coverage設定だけの変更ではStatic Audit Config subject hashを変えない。
- config subjectを欠く読取り互換static Audit Recordを現在のPASSへ昇格しない。

#### 18.3.3 execution・Evidence

- 選択した登録Testだけをrunnerのexact selectorで実行する。
- Testごとの結果、revision、hash、adapter ID、runner情報、およびExecution State subjectをEvidenceへ記録する。
- build failure、runner failure、必須runner capabilityの欠落、および宣言targetの解決失敗ではEvidenceを生成しない。
- 実行前後でExecution State subjectが変化した場合はE-EXEC-004となり、Evidenceを生成しない。
- Evidence writerはadapter IDを必ず記録する。
- Evidence writerは中立fieldの`hashes.test_subject`と`hashes.targets[].target_construct`を出力する。`test_fn` / `test_construct` / `target_fn`の互換入力は`rust-cargo` Evidenceで全canonical metadataを含むsource rangeと現在値の同一性を証明できる場合だけ受理する。
- Evidence readerはadapter IDを欠くrecordについて、現在のTestが `rust-cargo` で、
  runner kindと内容hashからRust実行を一意に確認できる場合だけ互換Evidenceとして扱う。
- Evidenceは全宣言targetを解決したcanonical Locatorと内容hashを重複なく保持し、参照側Testが宣言した`TargetRef`の綴り（SRC ID参照を含む）をtarget identityとして保存しない。同一Source Targetをlocator参照するTestとSRC ID参照するTestのEvidenceは、同じtarget identityと同じtarget内容hashを持つ。
- 監査レコードの`subjects`の`target` entryも解決後のcanonical Locatorとし、Evidence側のtarget identityと一致する。
- 全宣言targetがcanonical Source Targetへ一意に解決できることをEvidence生成のpreconditionとする。1件でも対象なしまたは曖昧なら**Evidenceを生成しない**。部分的な`hashes.targets`を持つEvidenceを生成しない。この場合`test_execution`はNOT_EXECUTEDのままとなる。
- Evidence記録後に宣言targetのいずれかが一意に解決できなくなった場合、記録済み参照集合が現在のcanonical集合と一致しないためSTALEになり、`target_execution`もPASSにしない。
- 解決できなくなったtargetは、対象が存在しない場合（E-SCAN-004）は`MISSING`、複数候補により曖昧な場合（E-SCAN-011）は`MISMATCH`として保持する。両者を一括して同一の状態値にしない。
- canonical Test metadata、ExecutionDescriptor、Test construct、宣言target集合、いずれかのtarget内容hash、HEAD revision、またはExecution State subjectがEvidenceと異なる場合はSTALEになる。
- `revision.commit`を特定できないEvidence、および現在のHEAD revisionと一致しないEvidenceはSTALEになり、FAILまたは有効なPASSとして扱わない。
- Execution State subjectはrunner / toolchain / 実行影響configと、実行可能状態を変えうるrepository / local dependency入力の完全なmanifestを束縛する。Testと宣言targetを変更せずtarget外helperだけを変更しても既存EvidenceはSTALEになる。
- EvidenceがExecution State subjectを欠く互換recordならSTALE、recordのsnapshotまたは現在snapshotの完全性を証明できなければUNKNOWNとなり、いずれもPASSにならない。
- EvidenceがSTALE / MISMATCH / UNKNOWNなら`test_execution`、`runtime_result`、`target_execution`へ同じ非PASSを伝播し、無効Evidenceのresultまたはcoverageを再利用しない。Evidenceなしでは3項目ともNOT_EXECUTEDになる。
- 単数互換形のEvidenceは、現在のTestがtargetをちょうど1件持つ場合だけ有効性を評価できる。複数target Testでは有効なPASSにしない。
- Evidenceのadapter IDがTest execution adapterと異なる場合はMISMATCHになる。

#### 18.3.4 semantic audit protocol

- spec-coverage、test-semantic、vo-coverage、impl-consistencyのbundleは本冊 §8の必須情報を含む。
- spec-coverageは対象SPEC sourceと、それを参照するactive REQの完全集合へ束縛され、要求事項の取り込み、対応REQ、exclusion根拠を含む場合だけ受理される。
- 空のreasons、schema違反、kind不一致、stale bundle hashを拒否する。
- 受理するAudit Recordはsubjectsの内容hashへ束縛される。
- deterministic結果とagent / human結果を区別して保存・表示する。
- impl-consistency bundleとAudit Recordは、対象VOと上流VO / REQの`spec_refs`から導出したSPEC subject完全集合へ束縛される。Specification record、参照先source、または集合だけを変更しても既存recordはSTALEになり、限定scopeの`impl_consistency = PASS`へ利用されない。
- impl-consistency提出verdictのFAILはAudit Recordに保持され、検証項目`impl_consistency`ではMISMATCHへ写像される。監査未実施はNOT_CHECKED、無効recordだけがある場合はSTALE、判定不能はUNKNOWNのままとする。
- targetを一意に解決できない場合はimpl-consistency bundleを生成せず、候補のいずれも選択しない。対象が存在しない場合（E-SCAN-004）は`impl_consistency = MISSING`、複数候補により曖昧な場合（E-SCAN-011）は`MISMATCH`とし、両者を一括して同一の状態値にしない。複数の解決失敗が異なる種別で併存する場合の代表値は基本仕様 §4.3の優先順位に従う。

#### 18.3.5 verify・report

- 完全検証は基本仕様 §4.2の12項目をすべて評価し、各項目の非PASSを総合NGへ反映する。
- 完全検証は全12項目がPASSの場合だけOKとする。
- `--items`を省略したCLI / MCP検証は常に固定12項目を評価する。version 1 configの`full_scope`欠落は固定12項目、11項目形は`test_traceability`を補った固定12項目へin-memoryで正規化し、configを書き換えない。version 1の重複・未知項目、およびversion 2の欠落・重複・未知・余剰項目はE-CONFIG-001とし、検証結果を生成しない。
- 12項目未満を明示した`--items`だけを限定scopeとして扱い、「完全検証」と表示しない。
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
- `target_execution.checked: true`のEvidenceでtarget別entryが欠落、重複、または解決後のcanonical Source Target集合と不一致ならPASSにしない。
- target別entryは解決後のcanonical Locatorをidentityとし、宣言側の綴りを用いない。
- coverage capabilityまたは計測toolが利用できない場合はNOT_CHECKEDとなり、PASSにならない。
- coverage解析限界はUNKNOWNとなり、PASSにならない。
- Testが別プロセス（起動したsubprocess）・別スレッドでtargetを実行する場合、coverage計測が当該境界越しの実行を宣言targetへ帰属できればtarget別PASS（count > 0）になり、その結果は本冊 §7.3のruntime到達証明としても機能する。provider が境界越しの実行を帰属できなければtarget別UNKNOWN、計測不能ならTestの`target_execution = NOT_CHECKED`となり、いずれもPASSにならない。

#### 18.3.7 Structured Test Operation

- Form `kind`は`[a-z0-9][a-z0-9-]*`のcase-sensitive文字列で、built-inとuser-defined schemaを通してrepository全体で一意であり、schemaはowner `adapter` IDを別fieldで宣言する。registryのkind owner、schemaのadapter、Structured Test capabilityが一意に一致する場合だけcreate / form_getを許可する。
- 同じkindを複数adapterが宣言する、schemaとregistry ownerが不一致、adapterが未知、またはcapabilityがない場合は操作を拒否し、ファイルを変更しない。
- `adapter`を欠く読取り互換Formは、登録済みStructured Test adapterのbuilt-in kind宣言またはschema compatibility matcherのうちちょうど1件だけがschemaを受理する場合に限って解決し、曖昧またはowner不在なら拒否する。matcherはschema内容から決定論的に判定し、coreは未知kindを`rust-cargo`へfallbackしない。
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
