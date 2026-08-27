# AI並列開発向けテスト検証システム 詳細設計 別紙C 受入仕様 v0.1

本冊 §0 の分冊構成に基づき、本別紙は §18 を収録する。

本別紙からの `基本仕様 §n` 参照は再導出済み基本仕様 v0.1 の連番（§0〜§30）を、`要件定義 §n` 参照は凍結要件定義 v0.1 の連番を、`本冊 §n` 参照は再導出済み詳細設計本冊 v0.1 の連番を指す。別紙A への参照は分冊構成が固定する収録節範囲（§12〜§15）に留め、内部項番を引かない。

---

## 18. 受入契約

### 18.1 共通条件

- 受入条件は決定論的なfixtureと統合テストで再現できる。
- Rust workspaceの受入テストは `cargo test --workspace` で実行できる。
- 検証結果はfail-closedであり、要求scopeに1件でも非PASSがあれば総合結果はNGになる。
- scopeを限定してもscope外の値をPASSへ変更しない。
- CLIとMCPは同じcore処理、adapter registry、JSON envelope、診断codeを使用する。
- canonical record、承認記録、判断記録、Evidence、内容hashの不変条件をfixtureの都合で緩和しない。

### 18.2 共通fixture

Rustの受入fixtureは、総称 document、VO、登録Test、Source Target、承認記録、判断記録、
Evidenceを含む小規模projectとする。fixtureは少なくとも次を表現できる。

- 正しいannotationを持つTest
- `assert!(true)`だけのTest
- 宣言targetを呼ばないTest
- 結果を検証しないTest
- 自己比較を行うTest
- annotationを持たないtest function（W-SCAN-101、`chain_integrity = MISMATCH`、診断 `MISSING`）
- `covers`を宣言しないTest（`covers` 0）。すべての管理対象 Test に `covers ≥ 1` を一律要求するため、E-SCAN-007 と `chain_integrity = MISMATCH`（診断 `MISSING`）になる（本冊 §11.1.1、基本仕様 §12）
- `rust-cargo` で `targets` を宣言しないTest（E-SCAN-007、`chain_integrity = MISMATCH`、診断 `MISSING`。`targets ≥ 1` は `rust-cargo` adapter の必須 metadata。本冊 §4.4・§5.5）
- 存在しないVOを参照するTest（E-SCAN-003、`chain_integrity = MISMATCH`）
- Test IDが衝突するTest（E-SCAN-002、`chain_integrity = MISMATCH`）
- Test constructと非隣接のmetadata宣言だけを変更した状態（Test subject hashが変化する）
- Test / 宣言targetを変更せず、実行結果を変えうるtarget外helperまたはlocal dependencyだけを変更した状態（Execution State subjectが変化しEvidenceがSTALE化）
- `@vtest.case`を持つtable-driven Test
- 複数targetを宣言し、targetごとにPASS / FAIL / UNKNOWNが異なるintegration Test
- **5 状態それぞれを生じる入力**（`PASS` / `FAIL` / `MISMATCH` / `NO_EVIDENCE` / `UNKNOWN`）。状態は 5 つのみとする（基本仕様 §4.1）
- **4 診断ラベルそれぞれを生じる入力**（`MISSING` / `NOT_EXECUTED` / `NOT_CHECKED` / `STALE`）。診断ラベルは検証状態と別軸の原因説明であり、状態値ではない（基本仕様 §4.2）
- Testまたはtargetのhash変更によって無効になる判断記録 / Evidence
- 複数adapterが同じ恒久SRC IDを宣言する状態（E-SCAN-011）
- 同一のSource Targetを、一方のTestがlocatorで、他方のTestが恒久SRC IDで宣言する状態
- 同一のTestが同一Source Targetをlocatorと恒久SRC IDの両方で宣言する状態（E-SCAN-005）
- Source Target constructの内側にある`@vtest.src-id`宣言だけを付与・変更・削除した状態（construct bytesが変化しSource Target hashも変化する）
- 呼出を静的に確認できない到達境界を越えてtargetを実行するTest（subprocess spawn型・spawn thread型）。DA-002 / DA-003がtarget別UNKNOWNになり、runtimeの`target_coverage`のみでDA-002到達が充足される
- 他ファイル・他クレートへ呼び出すが戻り値をTest本体内でassertするTest（DA-002 UNKNOWN・DA-003 PASS）
- 文書鎖の各状態：`doc.roots` に列挙された根 document、`derives_from` が空かつ根に列挙されない孤児 document（E-SCAN-016、`orphan_detection = MISMATCH`）、`derives_from` の参照先が存在しない document / VO（E-SCAN-012、`chain_integrity = MISMATCH`）、`content_hash` と実ファイルが一致しない document（W-SCAN-104、`chain_integrity = MISMATCH`、診断 `STALE`）、document 再登録で失効する判断記録・承認記録
- 判断記録を受理しても対象の検証状態が昇格しない状態（判断受理前後で `UNKNOWN` が `PASS` へ変わらない）
- 上流依存closureまたはハッシュを欠く互換Approval（W-STORE-002、VOは `draft` 相当）
- フェーズゲート定義（`config.yaml` の `gates`）を持ち、`vtest verify --gate <name>` が条件充足・不足の両方を提示する状態

adapter境界fixtureは、Rust parser、Cargo、llvm-covを使用しないin-process synthetic
adapterを使用できる。synthetic adapterは配布対象のproduction language adapterではない。
synthetic fixtureは`.rs`以外のsource、関数ではないTest construct、doc commentではないmetadata宣言、Rust item pathではないopaque locatorを使用する。

### 18.3 機能別受入条件

#### 18.3.1 discovery・record・graph と chain_integrity

- source discovery adapterは全Discovered Test draft、ManagedTestDraftLink、SourceTargetDraft、Source Location、source range、current bytes、logical metadata、宣言された恒久SRC ID、Test execution descriptorをhash未計算で返す。coreは出力を検証してTest subject / Source Target hashを計算してからManaged Test Entity、ManagedTestLink、Source Targetを具体化する。
- Source Targetはcanonical locatorと任意の恒久SRC IDを併有する単一のentityである。adapterは同一constructをlocator版とSrcId版の2 draftへ複製せず、恒久SRC IDを`SourceTargetDraft.src_id`として返す。
- 恒久SRC IDを持つSource Targetはcanonical locatorでもaddressableであり、locator参照とSRC ID参照は同一のcanonical Source Targetへ解決する。両addressing modeで同一のSource Target hashに到達し、Source Targetの件数、content / subject hash、Evidenceおよび判断記録上のtarget identityが参照方法によって分裂しない。
- Source Target identityは「宣言された`TargetRef` → 解決 → canonical Locator」の一方向で確定する。Evidence、判断記録、`target_binding` の証拠、鮮度判定は解決後のcanonical Locatorをidentityとして記録・比較し、参照側Testが宣言した綴り（SRC ID参照を含む）を保存しない。同一のSource Targetをlocator参照するTestとSRC ID参照するTestは、Evidence上で同一のtarget identityを持つ。
- Testがどう宣言したかの変更（同一Source Targetに対するlocator参照からSRC ID参照への書き換え等）はTest subject hashの変化として捕捉され、Evidence側のtarget identityを変化させない。
- 綴りの異なる複数の`target`宣言が同一のcanonical Source Targetへ解決する場合はE-SCAN-005とする。
- `SourceTargetDraft.target`は必ず`TargetRef::Locator`である。`TargetRef::SrcId`をcanonical targetとして返したadapter出力はmalformed adapter outputとして拒否する。恒久SRC IDの宣言・変更・削除でcanonical locatorは変化しない。
- Source Target hashは常にcanonical locatorとconstruct bytesから計算し、参照側Testの`TargetRef`綴りからは計算しない。恒久SRC IDを独立したhash fieldとしてSource Target hashのinputに含めない。
- 恒久SRC IDの宣言をSource Target constructの内側へ置くadapter（`rust-cargo`の`@vtest.src-id` doc comment等）では、その宣言の付与・変更・削除がconstruct bytesを変えるため、Source Target hashも変化する。これはsourceが実際に変化したことの帰結として正しい挙動であり、恒久SRC IDが独立したhash fieldであることを意味しない。
- SRC ID参照はcoreの統合済みSRC索引から、その恒久SRC IDを宣言したSource Targetのcanonical locatorへ解決する。
- Target Reference解決は解決済み / 対象なし / 曖昧を区別し、曖昧はfail-closedな終端状態とする。E-SCAN-004またはE-SCAN-011で曖昧・未解決となったtargetについて、判断記録subject、Evidence、`target_binding` の証拠のいずれも候補の1件を解決結果として記録しない。候補は診断表示にだけ用いる。
- この禁止は解決に関するものであり、Source Targetの具体化を止めない。恒久SRC IDが衝突していても、各Source Targetは自身のcanonical locatorで独立したentityとして具体化され、Source Targetの件数と各content / subject hashは衝突の有無で変化しない。衝突が壊すのは当該恒久SRC IDによる参照の一意性だけである。
- この解決はcoreの単一経路が所有する。discovery、静的解析、実行、Evidence writer、検証集約が独自にcandidate列を走査して1件を選ぶ経路を持たない。
- adapter所有のmetadata宣言、ID、target、VO参照、record schema、Relationの違反を対応診断codeで検出する。

**chain_integrity（宣言鎖の完全性）**（本冊 §11.1.1、基本仕様 §5.1）

- **Test 層**：管理宣言または必須metadata（core 中立の Test ID・`covers ≥ 1`・`intent`、および当該 adapter が必須とする追加 metadata〔`rust-cargo` では `targets ≥ 1`〕）を持たないTestが1件でもあれば、W-SCAN-101またはE-SCAN-007を表示し、`ManagedTestLink::Missing`から`chain_integrity = MISMATCH`（診断 `MISSING`）を導出する。
- 存在しないVOを`covers`するTestは構造上完全なManaged Test Entityと`ManagedTestLink::One`のまま保持し、E-SCAN-003と`chain_integrity = MISMATCH`を導出する。診断ラベルを二重定義しない。
- `ManagedTestLink::Multiple`またはTest ID衝突（E-SCAN-002）は`chain_integrity = MISMATCH`になる。
- `covers` を持たない（0 件の）Testは管理宣言不整合として`chain_integrity = MISMATCH`（診断 `MISSING`）になる。役割による`covers`可変制約・特別扱いの分岐を設けず、すべての管理対象 Test に`covers ≥ 1`を一律要求する（基本仕様 §12）。既定を緩和して 0 件を受理しない。
- 全Discovered Testが`ManagedTestLink::One`で構造上完全なentityへ1対1で対応し、Test IDが一意、各entityが`covers ≥ 1`を満たし、かつ全VO参照を解決できる場合だけTest層の`chain_integrity`が成立する。
- **VO 層**：各 VO は 1 件以上の `document` への解決可能な `derives_from` を持つ。参照先 document が存在しない、または解決不能な場合は E-SCAN-012、`chain_integrity = MISMATCH`。VO parent の不在・循環は E-SCAN-008、`chain_integrity = MISMATCH`。
- **文書層**：各 `document` の `derives_from` 参照先が存在すること（不在は E-SCAN-012、`chain_integrity = MISMATCH`）、`content_hash` が実ファイル（`path`）と一致すること（不一致は W-SCAN-104、`chain_integrity = MISMATCH`、診断 `STALE`）を要求する。document 種別を区別せず、要件定義・基本仕様・詳細設計・API Schema 等をすべて総称 document として同一に扱う（本冊 §3.1）。
- **双方向完全性**：`covers` する Test が 1 件以上存在しない leaf VO は `chain_integrity = MISMATCH`（診断 `MISSING`）。発見された Test → 管理宣言の解決と、leaf VO → Test の両方向が成立して初めて `chain_integrity` が成立する。
- W-SCAN-101のwarning severityだけを理由に検証値を変更せず、Discovered Testとmanaged entityの対応事実から判定する。
- adapter discoveryの失敗をTest 0件の正常scanとして扱わない。解析不能・不完全なbatchは対応する検証を `UNKNOWN` とする。
- Relation writerは`REL-<ULID>`だけを生成する。readerはファイル名とrecord IDが同じbare ULIDのversion 1互換Relationを読み取り、in-memoryで`REL-<ULID>`へ正規化するが、ファイルを書き換えない。同じpayloadのbare / prefixed重複、混在形、ファイル名とIDの不一致はE-SCAN-010になる。Relation の from / to 不在は E-SCAN-009、`chain_integrity = MISMATCH`。
- VO writerは`status`を保存せず、実効値をApprovalから導出する。読取り互換field `status`は警告（W-STORE-001）して無視する。
- VOの承認はVO内容hashと現在の上流依存closureへ束縛され、`document` / parent VO の内容または集合が不一致の承認を有効として扱わない。
- Approval作成時に対象または上流依存closureを完全・currentに解決できなければE-APPROVAL-001で拒否し、recordを生成しない。
- 上流依存closureまたはハッシュを欠く互換Approvalを現在のapprovedへ昇格しない（W-STORE-002、VOは `draft` 相当）。
- 恒久SRC IDは全adapter統合後にrepository全体で一意であり、衝突をE-SCAN-011として拒否する。
- `vtest scan` / `doctor`はE-ADAPTER-* / E-CONFIG-*による操作拒否をexit 2、完了したscanのE-SCAN-*をexit 1、errorなしをexit 0にする。
- `full-product` VOは宣言partitionの直積を決定論的に実体化する。

**VO の `combinations`（`coverage_policy: explicit`）**（本冊 §3.2.1・§17.1）

- `coverage_policy: explicit` と妥当な `combinations` を持つ VO は、列挙された tuple ごとにちょうど 1 件の子 VO を生成し、子 VO ID の suffix は `dimensions` の宣言順で連結される。同じ tuple 集合を記述順・map key 順を変えて与えても、生成される子 VO 集合と ID は同一になる。
- 次の各入力を持つ VO レコードは E-SCAN-017 と `chain_integrity = MISMATCH` になり、`vo expand` は子 VO を 1 件も生成しない（部分生成しない）：`explicit` かつ `combinations` 欠落、`explicit` かつ `combinations` 空 list、`explicit` かつ `dimensions` 空、`independent-axes` / `full-product` / `null` かつ `combinations` 非空、未宣言 dimension 名を含む tuple、当該 dimension の `partitions` に無い partition 値を含む tuple、宣言済み dimension を欠く tuple、同一 dimension 名を 2 回持つ tuple、重複 tuple。
- `vo add` / `vo edit` / MCP `vo_upsert` は上記の各入力を受理時に E-SCAN-017・終了コード 2 で拒否し、レコードを作成・更新しない（拒否後に scan したエンティティ集合は操作前と同一）。
- `vo edit --combination` は desired state として既存 `combinations` を置換し、追記しない。`--clear-combinations` は空にする。どちらも与えない `edit` は既存 `combinations` を保持する。
- `combinations` だけを変更した `edit` は VO subject hash を変化させ、当該 VO の承認を失効させる。

**`derives_from` の `anchor`**（本冊 §3.1・§3.2）

- document / VO の `derives_from` entry に `anchor` を持つ状態と持たない状態の双方を読み取り、いずれも `chain_integrity` に影響しない（`anchor` の欠落・空文字列で `MISMATCH` にならない）。`anchor` の値を文書内位置へ解決せず、実在しない節番号を書いても診断を出さない。
- 同一 `doc` を指す複数 `derives_from` entry を `anchor` 違いで保持でき、重複として拒否しない。
- `anchor` だけを変更した document は `content_hash`（`path` の実ファイルのハッシュ）が不変のまま document subject hash が変化し、当該 document を上流依存 closure に含む承認・判断記録が失効する。
- `anchor` だけを変更した VO は VO subject hash が変化せず、当該 VO の承認が失効しない。
- CLI で `--derives-from` を伴わない `--anchor`、または 1 つの `--derives-from` に 2 個目の `--anchor` を与えた場合は終了コード 2 で拒否し、レコードを書かない。

**`vtest init` の非改変不変条件**（別紙A §12.2、基本仕様 §18.1）

- 既存ソース・既存テストを含む fixture project で `vtest init` を実行した前後で、`.verify/` を除いた作業ツリーの全ファイルのバイト列が同一である。`.verify/` 外のファイルの新規作成・変更・削除が 1 件も観測されない。
- `init` は既存ソースへ Test metadata 宣言（`@vtest.` 行）・annotation・doc comment を挿入しない。
- 既存 `.verify/` があるプロジェクトでの `init` は終了コード 2 で中止し、その実行でファイル・ディレクトリを 1 件も作成・変更・削除しない（既存 `.verify/` の内容も不変）。

#### 18.3.2 orphan_detection（文書層の孤児検出）

- `orphan_detection` は文書層のみを対象とし、親（上流 document）を持たない `document` ノードの有無を問う（本冊 §5.6、基本仕様 §5.2、要件定義 §4.2）。実装レイヤーの孤児検出（宣言されていない実装の検出）は行わない（要件定義 R-2、基本仕様 §29 OOS-005）。
- **根の除外**：`config.yaml` の `doc.roots` に列挙された DOC ID を根として扱い、`orphan_detection` の対象外とする。根指定の追加・削除は `vtest doc` コマンドの引数で管理する（基本仕様 §26.1）。
- **孤児判定**：`derives_from` が空、かつ他のどの document からも `derives_from` で参照されず、`doc.roots` にも列挙されない document を孤児とし、E-SCAN-016、`orphan_detection = MISMATCH` になる。
- `doc.roots` が存在しない DOC ID を参照する場合は config invariant 違反として E-CONFIG-001 とする。
- 旧モデルの W-SCAN-102（孤立 VO）は VO 層の警告であり、文書層 `orphan_detection` とは別物として存置する。

#### 18.3.3 決定論的静的解析（oracle_presence・target_binding 静的到達）

- DA-001〜DA-006とW-DA-101は本冊 §7の判定条件に従う。静的解析は正典レコードを持たない再計算派生であり、検証のたびに現在の source / config から再計算する（本冊 §7.1、基本仕様 P-003）。
- 確定違反だけをFAILとし、解析限界をUNKNOWNとして保持する。
- 正常Testは違反なしとなり、各違反fixtureは対応ruleで非PASSになる。
- **oracle_presence への合成**：`oracle_presence` は DA-001 / DA-003 / DA-004 / DA-005 / DA-006 の合成とし、全ルール違反なしで `PASS`、1つでも `FAIL` があれば `FAIL`、`FAIL` がなく `UNKNOWN` があれば `UNKNOWN` になる（本冊 §7.1、基本仕様 §5.4）。`oracle_presence` に動的な昇格経路は無く、runtime 証拠で `PASS` へ昇格しない。
- **target_binding 静的到達（DA-002）**：DA-002 の target 別 verdict が `UNKNOWN` のとき、当該 target の runtime 計測（§18.3.5）が実行を証明した場合に限り到達要件が充足される（本冊 §7.3）。この runtime 救済は `target_binding` に固有であり、`oracle_presence` には及ばない。
- static audit adapterが判定へ使用したsource fragment集合の完全性を保証できない場合、当該判定はUNKNOWNとなりPASSにならない。
- 別プロセス・別スレッド・クロージャ・他ファイル等、静的解析の到達境界を越えてtargetを実行するTestは、当該targetのtarget別DA-002 verdictがUNKNOWNになる（本冊 §7.3）。当該targetのruntime `target_coverage` がPASS（checked: true・count > 0）ならDA-002到達要件は充足され、検証時にその target別DA-002はUNKNOWN扱いにならない。
- **呼出自体を静的に確認できないtarget（subprocess spawn等）は、DA-002だけでなくDA-003のtarget別verdictもUNKNOWNになり（空虚PASS / FAILとしない）、DA-003はruntimeで救済されない。** したがってexit code / stdoutだけをassertするsubprocess E2Eは、当該targetのDA-002がruntimeで充足されて`target_binding = PASS`に到達しうる一方で、DA-003がUNKNOWNのまま残り`oracle_presence = UNKNOWN`となる。この2検査が別々の値をとる場合が新モデルの識別fixtureであり、総合判定はNGになる（本冊 §7.3）。
- 他ファイル・他クレートへ呼び出すが戻り値をTest本体内でassertするTestは、DA-002 UNKNOWN・DA-003 PASSとなり、runtime `target_coverage` がPASSでかつ他ルールも違反なしなら`target_binding`は到達充足、`oracle_presence = PASS`になる（runtime救済で実益が出るのはこの型）。
- 複数targetを宣言するTestで、target Aは静的（DA-002 = PASS）、target Bはruntime（Bのtarget別`target_coverage` = PASS）でDA-002到達を充足する場合、BもTest本体内で結果をassertしDA-003 = PASSなら`oracle_presence = PASS`かつBの`target_binding`到達も充足する。Bが呼出不可視（subprocess）でDA-003 UNKNOWNなら`oracle_presence = UNKNOWN`となる。到達判定はtarget別に行い、AとBのstatic verdictを取り違えない。
- DA-002 verdict = FAIL（解析境界内で到達を静的に否定）は runtime 証明で覆らない。
- runtime証明に依存する`target_binding`の値は、§18.3.4 の鮮度判定（本冊 §11.2）が選択した最新Evidenceが鮮度を満たすときだけ用い、無効な最新Evidenceから古い有効Evidenceへフォールバックしない。これにより同一検証内で計測がSTALEの一方 `target_binding` が別 Evidence でPASSになる履歴不一致を生じない。
- 表示scopeと内部依存評価を分離する。`vtest verify --items oracle_presence` / `--items target_binding` のような限定scopeでも、aggregatorは本冊 §7.3のruntime到達判定に必要なEvidence鮮度・target別`target_coverage`を内部依存として評価するが、scope外の項目自体のreport valueは `NO_EVIDENCE`（診断 `NOT_CHECKED`）のまま保持する。
- 同じ到達UNKNOWNのTestでも、当該targetの`target_coverage`がFAIL・UNKNOWN・NOT_CHECKED（coverage利用不能・未計測・`--fast`）なら到達要件は未充足で、当該targetのDA-002 UNKNOWNは`target_binding`の非PASS要因として残る。
- runtime coverageはDA-003を代替しない。結果検証はDA-003の静的判定（結果がassert相当へ到達）のまま評価し、到達がruntimeで充足されてもDA-003 UNKNOWN / FAILはそのまま`oracle_presence`へ寄与する。
- 宣言targetをどのtopologyでも実行しない構造・契約のみのTestは、静的にもruntimeにも到達を確立できず`target_binding`の到達要件は未充足のままになる。

#### 18.3.4 execution・Evidence（target_binding の証拠）

- 選択した登録Testだけをrunnerのexact selectorで実行する。
- Testごとの結果、revision、hash、adapter ID、runner情報、およびExecution State subjectをEvidenceへ記録する。
- build failure、runner failure、必須runner capabilityの欠落、および宣言targetの解決失敗ではEvidenceを生成しない。
- 実行前後でExecution State subjectが変化した場合はE-EXEC-004となり、Evidenceを生成しない。
- Evidence writerはadapter IDを必ず記録する。
- Evidence writerは中立fieldの`hashes.test_subject`と`hashes.targets[].target_construct`を出力する。`test_fn` / `test_construct` / `target_fn`の互換入力は`rust-cargo` Evidenceで全canonical metadataを含むsource rangeと現在値の同一性を証明できる場合だけ受理する。
- Evidence readerはadapter IDを欠くrecordについて、現在のTestが `rust-cargo` で、runner kindと内容hashからRust実行を一意に確認できる場合だけ互換Evidenceとして扱う。
- Evidenceは全宣言targetを解決したcanonical Locatorと内容hashを重複なく保持し、参照側Testが宣言した`TargetRef`の綴り（SRC ID参照を含む）をtarget identityとして保存しない。同一Source Targetをlocator参照するTestとSRC ID参照するTestのEvidenceは、同じtarget identityと同じtarget内容hashを持つ。
- 全宣言targetがcanonical Source Targetへ一意に解決できることをEvidence生成のpreconditionとする。1件でも対象なしまたは曖昧なら**Evidenceを生成しない**。部分的な`hashes.targets`を持つEvidenceを生成しない。この場合`target_binding`は `NO_EVIDENCE`（診断 `NOT_EXECUTED`）のままとなる。
- Evidence記録後に宣言targetのいずれかが一意に解決できなくなった場合、記録済み参照集合が現在のcanonical集合と一致しないため `NO_EVIDENCE`（診断 `STALE`）になり、`target_binding`をPASSにしない。
- 解決できなくなったtargetは、対象が存在しない場合（E-SCAN-004）は`MISMATCH`（診断 `MISSING`）、複数候補により曖昧な場合（E-SCAN-011）は`MISMATCH`として保持する。両者を一括して同一の状態値にしない。
- canonical Test metadata、ExecutionDescriptor、Test construct、宣言target集合、いずれかのtarget内容hash、HEAD revision、またはExecution State subjectがEvidenceと異なる場合はSTALE（`NO_EVIDENCE`、診断 `STALE`）になる。
- `revision.commit`を特定できないEvidence、および現在のHEAD revisionと一致しないEvidenceは `NO_EVIDENCE`（診断 `STALE`）になり、FAILまたは有効なPASSとして扱わない。
- Execution State subjectはrunner / toolchain / 実行影響configと、実行可能状態を変えうるrepository / local dependency入力の完全なmanifestを束縛する。Testと宣言targetを変更せずtarget外helperだけを変更しても既存Evidenceは `NO_EVIDENCE`（診断 `STALE`）になる。
- EvidenceがExecution State subjectを欠く互換recordなら `NO_EVIDENCE`（診断 `STALE`）、recordのsnapshotまたは現在snapshotの完全性を証明できなければ `UNKNOWN` となり、いずれもPASSにならない。
- Evidenceが無効（STALE / MISMATCH / UNKNOWN）なら`target_binding`へ同じ非PASSを伝播し、無効Evidenceのresultまたはcoverageを再利用しない。Evidenceなしでは`target_binding`は `NO_EVIDENCE`（診断 `NOT_EXECUTED`）になる。旧モデルの `test_execution` / `runtime_result` / `target_execution` の3独立項目は撤去し、`target_binding` 単一検査の証拠（Evidence の存在・鮮度、`result`、`target_coverage`）へ吸収する（本冊 §11.1）。鮮度喪失の独立検査（旧 `evidence_validity`）は設けず、鮮度は基本仕様 §6 のハッシュ束縛により満たし、喪失を診断ラベル `STALE` として説明する。
- 単数互換形のEvidenceは、現在のTestがtargetをちょうど1件持つ場合だけ有効性を評価できる。複数target Testでは有効なPASSにしない。
- Evidenceのadapter IDがTest execution adapterと異なる場合はMISMATCHになる。
- 有効なEvidenceについて、`target_binding` は次で定まる（本冊 §11.2）。`result: FAIL`（テストランナーが失敗を報告）なら `FAIL`（要件定義 §5.3）。`result: PASS` かつ全宣言 target の到達要件が §18.3.3 / §18.3.5 で充足されれば `PASS`。`result: PASS` だが到達未充足 target があれば、当該 target の `target_coverage` に従い count 0 は `FAIL`（診断 `NOT_EXECUTED`）、計測不能・未計測（`checked: false`）は `NO_EVIDENCE`（診断 `NOT_CHECKED`）、関数不見当は `UNKNOWN`。

#### 18.3.5 target_binding 動的計測（per-target）

- 各宣言targetについて、計測countが1以上ならtarget別PASS、0ならtarget別FAIL、確実に同定または計測できなければtarget別UNKNOWNになる。
- 複数target Testの集約値は、1件でもtarget別FAILがあればFAIL、FAILがなく1件でもUNKNOWNがあればUNKNOWN、1件以上の全宣言targetがPASSの場合だけPASSになる。
- target AがPASSでもtarget BがFAILまたはUNKNOWNなら、Test単位の`target_binding`をPASSにしない。
- `target_coverage.checked: true`のEvidenceでtarget別entryが欠落、重複、または解決後のcanonical Source Target集合と不一致ならPASSにしない。
- target別entryは解決後のcanonical Locatorをidentityとし、宣言側の綴りを用いない（本冊 §6.1.1）。
- coverage capabilityまたは計測toolが利用できない場合は `NO_EVIDENCE`（診断 `NOT_CHECKED`）となり、PASSにならない。
- coverage解析限界は `UNKNOWN` となり、PASSにならない。
- Testが別プロセス（起動したsubprocess）・別スレッドでtargetを実行する場合、coverage計測が当該境界越しの実行を宣言targetへ帰属できればtarget別PASS（count > 0）になり、その結果は本冊 §7.3のruntime到達証明としても機能する。providerが境界越しの実行を帰属できなければtarget別UNKNOWN、計測不能ならTestの`target_coverage`を`checked: false`（`NO_EVIDENCE`、診断 `NOT_CHECKED`）とし、いずれもPASSにならない。
- `target_coverage` は `target_binding` の動的計測結果であり独立の検査項目ではない。旧モデルの `target_execution` 検査項目は撤去し、計測事実だけを Evidence の `target_coverage` field として保持して `target_binding` の証拠源へ吸収する（本冊 §3.6・§10）。

#### 18.3.6 判断記録プロトコル（非ゲート）

- `vtest audit bundle` は判断対象ごとに、判断に必要な情報（対象 VO と claim、Test Intent、テストコード、対象実装、関連テスト、既知 partition、過去の判断、対象の内容ハッシュとリビジョン）を JSON として `cache/bundles/` へ出力する。バンドルは派生情報であり Git 管理しない（本冊 §8.1、基本仕様 §11.3）。
- `vtest audit submit` の判断は少なくとも actor / subject / decision / judgment_kind を含み、理由・根拠（`reason` / `exclusions`）と `supersedes` は任意（optional）とする。
- submit は次を順に検証し、失敗は §17 のエラーコードで拒否する：bundle_id のバンドルが存在する（E-AUDIT-001）、subject がバンドルと一致する（E-AUDIT-003）、judgment_kind がバンドルと一致し値域内である（E-AUDIT-003）、バンドル記録時の各対象の内容ハッシュが現在と一致する（E-AUDIT-002）、decision が受理する判断値である（E-AUDIT-004）、supersedes の各 ULID が同一 subject かつ同一 judgment_kind の既存判断記録を指し自己参照でない（E-AUDIT-008）。
- **理由が空であることだけを根拠に判断を無効・`UNKNOWN`・`NO_EVIDENCE`・`MISMATCH` 等として扱わない**（基本仕様 §11.3、要件定義 §12）。旧モデルの reasons / claim / basis 必須検査（E-AUDIT-005）、decomposition-viewpoint 検査（E-AUDIT-006）、spec / req basis 検査（E-AUDIT-007）は撤去し、判断記録層で課さない。
- 受理された提出は判断記録として `.verify/decisions/` へ保存され、バンドル生成時の全対象の内容ハッシュを `subject_hash` と `dependencies` として記録し、依存 closure のハッシュへ束縛する。
- **判断記録の受理は当該対象の検証状態（§4.1 の 5 状態）を昇格させない。** このプロトコルは検証状態のゲートではなく、`UNKNOWN` に対する外部判断の追跡である（本冊 §8、基本仕様 §11.3）。旧モデルの `verdict → CheckValue` 写像（`impl_consistency = MISMATCH` を含む検証状態への変換経路）は撤去する。
- 旧モデルの意味監査 bundle 種別（spec-coverage / test-semantic / vo-coverage / impl-consistency）を検査として扱わず、網羅・意味の疑義は `UNKNOWN` として本プロトコルへエスカレーションする（本冊 §7.1・§8、基本仕様 §5・§11、要件定義 §12）。`spec_coverage` / `vo_decomposition` / `vo_coverage` / `impl_consistency` は検証項目として存在しない。
- deterministic 結果（§18.3.3 の静的解析）と agent / human の判断結果を区別して保存・表示する。
- 判断記録の有効性は判定時に評価し、subject が一致し `subject_hash` が現在の内容ハッシュと一致し、`dependencies` が現在の上流依存closureとentity・hashとも完全一致する場合だけ有効とする。document は登録 content_hash と実ファイルの一致も要求し、不一致の document を STALE とし、依存する判断記録も無効とする（本冊 §8.5・§11.4）。
- 同一対象に有効な判断記録が複数あってよい（再判断・多重判断）。
- **判断バンドルは Test が宣言した cases 集合を規範項目として含む。** `@vtest.case` 宣言の正規化文字列を宣言順に並べた list として出力し、`@vtest.case` を持たない Test でも空 list を明示して項目を省略しない（本冊 §8.1・§8.2、基本仕様 §14）。
- **バンドルと判断記録は判断型 `judgment_kind` をちょうど 1 件持つ。** 値域は `test-semantic` / `impl-consistency` / `case-coverage` であり、`subject` の値域は前 2 者が Test ID、`case-coverage` が Test ID または VO ID である。表にない組合せの要求ではバンドルを生成せず usage error（終了コード 2）とする（本冊 §8.1、別紙A §12.2）。
- **`case-coverage` は §11 の判断対象であって基本仕様 §5 の 4 検査ではない。** その未判断・判断結果はいずれの検査の値へも写像せず、集約（本冊 §11.3）へ寄与しない。外部判断が必要な事実は判断待ち section（`check: null`、`judgment_kind: case-coverage`）としてだけ提示する（本冊 §8.1・§11.7）。
- **`case-coverage` の判断待ち項目は決定論的に生成する。** `covers ≥ 1` かつ（`cases ≥ 1` または解決済みの covers 先 VO（レコードが存在する VO。E-SCAN-003 の dangling 参照を除く）のいずれかが `dimensions ≥ 1`）を満たす管理対象 Test ごとにちょうど 1 件生成し、`(当該 Test, case-coverage)` の実効判断が `accepted` の場合にだけ消滅する。実効判断が未確定・`rejected`・`deferred` のいずれでも項目は生成され、参照した判断記録 ID を `basis` に載せる。この消滅規則は `case-coverage` 型の項目にだけ適用し、検査に由来する `kind: unknown` の項目の生成・消滅は判断記録の有無で変わらない（本冊 §11.7）。
- **実効判断は `(subject, judgment_kind)` の組ごとに決まる。** 有効判断記録集合から、他の有効判断記録の `supersedes` に名指しされたものを除いた実効集合 E について、E が空なら未確定（`UNKNOWN`）、E の decision 値が全て同一ならその値、E に 2 種以上の decision 値があれば未確定（`UNKNOWN`）かつ W-STORE-004 とする（本冊 §8.5）。
- **競合は `supersedes` による明示の置き換えでだけ解消する。** 判断記録の新旧（`decided_at` / ULID 順）、`decision` 値の優先順位、記録件数の多寡のいずれも採用規則に用いない。競合中の対象について機械がいずれかの判断記録を採用した結果を出力しない。
- **`supersedes` の検証**：提出時、`supersedes` の各 ULID が同一 `subject` かつ同一 `judgment_kind` の既存判断記録を指し自己参照でないことを検証し、違反を E-AUDIT-008 で拒否する。`judgment_kind` がバンドルと不一致または値域外の提出は E-AUDIT-003 で拒否する（本冊 §8.4）。
- **`supersedes` の循環**：レコード群が互いを名指しして実効集合 E が空になる場合は未確定（`UNKNOWN`）とし W-STORE-005 を出す。いずれかのレコードを推測で残さない（本冊 §8.5）。
- **`judgment_kind` を欠くか値域外の判断記録**は履歴表示だけを許可し、いずれの実効判断へも寄与させず W-STORE-003 を出す（本冊 §3.4・§8.5）。
- 実効判断が未確定であることは検証状態（§4.1 の 5 状態）を変更せず、`UNKNOWN` に §4.2 の診断ラベルを付与しない。未確定の事実は判断待ち section としてだけ提示する（本冊 §8.5・§11.7）。
- 仕様・VO・Test 等が変更された場合、過去の判断を現在状態へそのまま流用せず、現在状態に対して §5 の 4 検査を再実施する。その結果は `PASS` / `FAIL` / `MISMATCH` / `NO_EVIDENCE` / `UNKNOWN` のいずれにもなり得る。変更そのものが `UNKNOWN` を生成するのではない（基本仕様 §11.3、要件定義 §12）。
- 判断対象の target を一意に解決できない場合はバンドルを生成せず、候補のいずれも選択しない。対象が存在しない場合（E-SCAN-004）は `MISMATCH`（診断 `MISSING`）、複数候補により曖昧な場合（E-SCAN-011）は `MISMATCH` とし、両者を一括して同一の状態値にしない（本冊 §8.1）。

#### 18.3.7 承認と判断記録の分離

- **判断済みと承認済みを区別する**（判断済み ≠ 承認済み）。判断記録と承認記録は同一 entity であることを要求せず、別 entity でありうる（本冊 §3.4・§3.5、基本仕様 §11.3・§17）。判断は承認なしでも記録でき、正式採用は承認の別段階である。
- **承認は検証状態と独立の別軸である。** 承認済みを理由に非 `PASS`（`FAIL` / `MISMATCH` / `NO_EVIDENCE` / `UNKNOWN`）を `PASS` へ昇格させず、未承認を理由に `PASS` を降格させない（基本仕様 §4.5・§17）。判断受理も承認も、いずれも検証状態を昇格させない。
- **`approved_state` の値域は `approved` / `rejected` / `withdrawn` の 3 値である。** 値域外の値、および値域外の `subject` 種別（判断記録 ULID・Test ID 等）は書込み時に E-APPROVAL-002 で拒否し record を生成しない。既存レコードとして読み取った場合は履歴表示だけを許可していかなる実効承認も導出せず W-STORE-006 を出す（本冊 §3.5）。
- **実効承認の導出は `approved_state` を参照する。** 有効承認レコード集合から、他の有効承認レコードの `supersedes` に名指しされたものを除いた実効集合について、集合が空なら `draft`、`rejected` または `withdrawn` が 1 件以上残るなら `draft`、全件が `approved` なら `approved` とする。有効の条件は `approved_state` が値域内であること、対象指定が一致すること、`subject_hash` が現在の内容ハッシュと一致すること、`dependencies` が現在の上流依存closureと entity・hash とも完全一致することである（本冊 §3.5）。
- **承認取消・却下は実効承認を `draft` へ落とす。** `approved` の承認レコードが存在しても、後から `withdrawn` または `rejected` の有効承認レコードを追加すると実効承認は `draft` になる。機械は `approved` と `rejected` / `withdrawn` のどちらかを新旧・件数で選ばない。
- **取消・却下後の再承認は `supersedes` による。** 当該 `withdrawn` / `rejected` レコードの ULID を `supersedes` に名指しした `approved` レコードを追加した場合にだけ `approved` へ戻る。名指ししない `approved` の追加では `draft` のままとする。`supersedes` の参照先が存在しない・対象が一致しない・自己参照は E-APPROVAL-002、循環は W-STORE-005 とする（本冊 §3.5）。
- **承認対象の値域は VO ID と document ID である。** 判断記録の承認は `judgment_ref` によってのみ表し、判断記録 ULID を `subject` に置かない。`judgment_ref` の参照先が存在しない場合は書込み時に E-APPROVAL-001、読取り時は当該レコードから VO / document の実効承認も判断記録の実効承認も導出せず W-STORE-006 とする（本冊 §3.5）。
- **判断記録を対象とする実効承認は、当該判断記録が §8.5 の有効判断でありかつ実効集合 E に属する場合にだけ導出する。** supersede された判断記録・競合により未確定となった判断記録への承認は `draft` 相当とする（本冊 §3.5・§8.5）。
- **方針は総称 document として登録した文書で表現し、専用のエンティティ型を設けない。** document を対象とする承認の上流依存closureは当該 document の再帰的な上位 document（`derives_from` 先）からなり、`vtest doc approve` / `doc_approve` で記録する。document 再登録（`--update`）で document subject hash が変化すると当該承認は失効する（本冊 §3.1・§3.5・§11.4、別紙A §12.2・§13.2）。
- VO を対象とする承認の上流依存closureは、対象 VO の再帰的 parent VO、対象 VO と parent VO が `derives_from` で参照する document、および各 document の再帰的な上位 document からなる。document dependency は §1.3 の document subject hash を使用するため、document record または参照先 source の変更で承認が失効する（本冊 §3.5・§11.4）。
- 実効承認状態の遷移は `draft` と `approved` の 2 値の間でだけ起き、検証状態（§4.1 の 5 状態）の変化・判断記録の追加そのもの・`basis` の内容によっては遷移しない（本冊 §3.5）。
- 上流依存closureまたはハッシュを欠く互換 Approval は読取りと履歴表示だけを許可し、現在の `approved` を導出しない（W-STORE-002、VO は `draft` 相当）。
- 承認主体は種別（`human` / `agent`）と識別子を記録する。承認権限（approval authority）・承認ロール・必要承認数・権限 schema はプロジェクト設定と別紙A へ委譲する（基本仕様 §17・§30）。承認 workflow の状態遷移と `approved_state` の値域は本冊 §3.5 に定める。

#### 18.3.8 verify・report と scope

- 完全検証は基本仕様 §5 の 4 検査（`chain_integrity` / `orphan_detection` / `target_binding` / `oracle_presence`）をすべて評価し、各検査の非PASSを総合NGへ反映する。
- 完全検証は、各検査の評価地点（DOC / VO / TEST / repository）で評価した全値がPASSの場合だけOKとする。
- `--items`を省略したCLI / MCP検証は常に固定4検査を評価する。version 1 configの`full_scope`欠落は固定4検査へ具体化し、version 1 / version 2 いずれでも旧12項目の列挙（`spec_coverage` / `test_existence` 等）は E-CONFIG-001 で拒否し、in-memory 補完で受理しない。version 1 の重複・未知項目、version 2 の欠落・重複・未知・余剰項目も E-CONFIG-001 とし、検証結果を生成しない（本冊 §2.2）。
- 4検査未満を明示した`--items`だけを限定scopeとして扱い、「完全検証」と表示しない。
- **scope は 2 軸で限定できる**（基本仕様 §4.6、要件定義 P-002）。検査軸（4 本の部分集合）とエンティティ軸（対象とする document / VO / Test の部分木）を指定でき、限定scopeのOKは「要求scope内のOK」に限られる。いかなる設定値も完全検証の検査を 4 本未満へ縮退させない。
- 限定scopeは要求項目だけを集約し、scope外・未実施の項目を `NO_EVIDENCE`（診断 `NOT_CHECKED`）として保持・併記する。出力には要求 scope と scope 外項目が未検証である旨を必ず併記する。
- `verify` / `report` の JSON（CLI・MCP）は最上位に `scope` を返し、`scope.requested.items`（`--items` 省略時は固定4検査を4件すべて列挙）、`scope.requested.entities`（エンティティ軸無指定は空 list）、`scope.unverified_outside_scope`（検査軸4件未満またはエンティティ軸指定ありで `true`、完全検証で `false`）を持つ。完全検証でも `scope` を省略しない。
- 検証結果を返さないコマンド（`init` / `scan` / `doc *` / `vo *` / `test *` / `audit *` / `run`）の JSON は `scope` を持たない。
- 限定 scope の JSON 出力だけから、要求 scope と「scope 外は未検証」の旨を判定できる（`scope.unverified_outside_scope` が `true` で、scope 外検査ノードが `NO_EVIDENCE`／診断 `NOT_CHECKED`）。
- 機能単位の集約は親 VO（子 VO を持つ VO）を単位とし、Feature を別エンティティ・別レコード・別 ID として設けない。親 VO の値は子 VO の値と当該親 VO を直接 covers する Test の値の fail-closed 合成であり、いずれかに非 `PASS` が 1 件でもあれば親 VO は非 `PASS` になる。`--vo <親VO>` および `--from <親VO> --direction down` が親 VO の代表値と配下の子 VO・Test の内訳を同一出力で返し、出力に Feature 名・Feature ID の field を含めない。
- 要求scope内の `FAIL`・`MISMATCH`・`NO_EVIDENCE`・`UNKNOWN` のいずれも総合PASSへ昇格しない。
- **NO_EVIDENCE を生む入力**（証拠が存在しない／証拠のハッシュが現在の対象と不一致／scope 限定により検査を実施しなかった項目）を受入で表現する。これらは `NO_EVIDENCE`（診断は順に `NOT_EXECUTED` / `STALE` / `NOT_CHECKED`）となり `PASS` へ変換されない（基本仕様 §4.3・§4.6）。
- 完全検証fixtureで4検査のそれぞれを単独で非PASSにすると総合NGになる。
- 管理済みgraph側の他検査がすべてPASSでも、未登録Testが1件あれば`chain_integrity`により総合NGになる。
- 集約は fail-closed とし、子に 1 つでも非 `PASS` があれば親は非 `PASS`。代表値の優先順位は `FAIL > MISMATCH > NO_EVIDENCE > UNKNOWN` とし、診断ラベル（`MISSING` / `NOT_EXECUTED` / `NOT_CHECKED` / `STALE`）は代表値の順位に用いず原因説明として併記する（基本仕様 §22.2、本冊 §11.3）。
- report は DOC → VO → Test の構造と、各非PASSの根拠（判断記録・Evidence への参照）を text / JSON で返す。旧モデルの SPEC → REQ → VO → Test 構造は総称 document 化により DOC → VO → Test へ再導出する。
- `covers` を持つ Test は covers 先 VO の子ノードとして表示する。管理下にある事実と、いずれの VO へも寄与しない事実の双方を出力から確認できる（基本仕様 §22.3）。`covers` を持たない Test は §18.3.1 の `chain_integrity = MISMATCH` として扱い、役割別表示を設けない。
- text treeのancestor continuation、middle child、last childを一意なbranch記号で描画する。

**判定の決定性**（本冊 §11.1、基本仕様 §11.1）

- 同一 revision・同一 `.verify/` ファイル集合（`config.yaml`・document / VO / Relation レコード・判断記録・承認・Evidence）・同一 scope 指定に対して `verify` を繰り返し実行すると、4 検査の検証状態・診断ラベル・診断コード集合・集約結果・`pending` section・終了コードが毎回一致する。
- 実行時刻・ロケール・タイムゾーン・呼出し元の作業ディレクトリを変えても、また Execution State subject（本冊 §1.3）の入力に影響しない環境変数を変えても、上記の出力が変化しない。ネットワークを遮断した環境でも同一の出力を返す。
- toolchain identity・adapter config・入力 manifest を変える環境変更（`RUSTUP_TOOLCHAIN` の切替等）の影響は Evidence の鮮度喪失（`NO_EVIDENCE`、診断 `STALE`。本冊 §11.2）としてのみ現れ、環境そのものを判定条件として読む経路を持たない。
- `vtest` は 4 検査の評価中に LLM API を含む外部サービスへ要求を出さない。外部 AI／Agent の関与は `.verify/decisions/` の判断記録ファイル経由に限られ、判断記録の受理は検証状態を昇格させない（§18.3.6）。
- 4 検査の評価経路に、実行時に差し替え可能な意味判定 seam を持たない。評価経路へそのような seam を導入する変更を行う場合は、正反対の判定を返す stub を注入しても 4 検査の結果が変化しないことを受入で確認する。

**上流該当箇所の同伴**（本冊 §11.6・§3.1・§3.2、基本仕様 §11.1）

- `report --from DOC-X --direction down --format json` は、`derives_from` エッジごとに `from` / `relation` / `to` と当該 entry の `anchor`・`note` を返し、「どの上流条項がどの VO へ対応するか」の対応ペア集合として読める。`anchor` を持たない entry では `anchor` を省略または `null` とし、空文字列で埋めない。
- この対応ペアの取得に新規 CLI コマンド・MCP ツールを用いない（既存の `report` projection と `test query` 逆引きだけで取得できる）。

#### 18.3.9 フェーズゲート評価

- プロジェクト側が登録したフェーズ・工程・ゲートの進行条件について、現在の検証状態（§4.1 の 5 状態）と承認（§18.3.7）が通過条件を満たすかを**評価・提示できなければならない（MUST）**（本冊 §11.5、基本仕様 §20、要件定義 §26.4）。
- ゲート定義は `config.yaml` の `gates` に、ゲート名と進行条件（`require.verification` ＝要求する検証結果、`require.approvals` ＝要求する承認ロール集合）として保持する。
- `vtest verify --gate <name>` は、指定ゲートの対象 scope について検証を実行し、(1) 検証結果が `require.verification` を満たすか、(2) `require.approvals` の各ロールについて対象の有効な承認が存在するか、を評価して満否と根拠（不足している非 `PASS` 検査・未充足の承認ロール）を提示する。条件充足・不足の両方を fixture で確認する。
- 検証状態と承認は独立の軸であり、ゲートは両者の組合せを進行条件にできる。承認済みを理由に検証状態を昇格させない。
- `require.verification` の値域を config 受理時に検査する。5 状態語彙（`PASS` / `FAIL` / `MISMATCH` / `NO_EVIDENCE` / `UNKNOWN`）との完全一致は受理し、診断ラベル（`MISSING` / `NOT_EXECUTED` / `NOT_CHECKED` / `STALE`）・`OK` / `NG`・小文字表記・旧12項目名・非文字列値は E-CONFIG-001・終了コード 2 で拒否して検証結果を生成しない。`require` および `require.verification` の欠落、`gates[].name` の重複も E-CONFIG-001 とし、`require.approvals` の省略と `gates` field 自体の欠落・空 list は受理する（本冊 §2.2）。
- ゲートの検証条件は `require.verification` と要求 scope の集約代表値の完全一致でのみ充足する。`require.verification` に `PASS` 以外（例 `UNKNOWN`）を定義したゲートは、代表値が同じ値のときだけ充足し、代表値が `PASS` のときは充足しない。逆に `require.verification: PASS` のゲートは代表値が非 `PASS` のとき充足しない。順序・包含解釈による充足を認めない fixture を持つ。
- 集約代表値は構造検査（`chain_integrity` / `orphan_detection`）を含む要求 scope 内の全評価値の fail-closed 合成であり、エンティティ軸の部分木が全 `PASS` でも構造検査が非 `PASS` なら代表値は非 `PASS` になる。
- `--items` で検査軸を限定した実行では scope 外検査が `NO_EVIDENCE`（診断 `NOT_CHECKED`）として代表値に参加するため、`require.verification: PASS` のゲートは限定 scope で充足しない。
- `--gate` を指定した `verify` / `report` の JSON は `data.gate` に `name`・`verification.{required, actual, satisfied}`・`approvals[].{role, satisfied, missing_subjects}`・`satisfied` を返す。`require.approvals` が空集合なら `approvals` は空 list、`gate.satisfied` は `verification.satisfied` と全 `approvals[].satisfied` の論理積になる。
- `--gate` 指定時の最上位 `ok` と終了コードはゲート充足で決まる（充足 → `ok: true`・0、不充足 → `ok: false`・1）。`require.verification` に `PASS` 以外を定義したゲートが充足した実行は、総合が NG でも終了コード 0 になる。
- config の `gates` に定義の無いゲート名を `verify --gate` / `report --gate` / MCP の `gate` 入力へ指定すると、E-CONFIG-002・`ok: false`・終了コード 2 で拒否し、検証もゲート評価も実行せず部分結果を返さない。診断には指定名と定義済みゲート名の一覧を含み、MCP tool error は `candidates` に定義済みゲート名を持つ。`gates` が空・未定義の状態での指定も同じ扱いとする。
- ゲート名の解決は大文字小文字を区別した完全一致だけで行い、前方一致・部分一致・近似一致・既定ゲートへの代替で受理しない。
- **責務はゲート条件が現在満たされているかの評価・提示に限る。** フェーズのライフサイクル管理・工程の自動遷移は責務外とする（基本仕様 §20・§29 OOS-004、要件定義 §26.4）。
- 新規 CLI コマンド・MCP ツールを増やさず、既存の `vtest verify` の `--gate` 引数と出力、および `report` の JSON でゲート評価を露出する。具体的なフェーズ名・承認ロール・必要承認数はプロジェクト設定と別紙A へ委譲する（基本仕様 §30）。

#### 18.3.10 Structured Test Operation

- Form `kind`は`[a-z0-9][a-z0-9-]*`のcase-sensitive文字列で、built-inとuser-defined schemaを通してrepository全体で一意であり、schemaはowner `adapter` IDを別fieldで宣言する。registryのkind owner、schemaのadapter、Structured Test capabilityが一意に一致する場合だけcreate / form_getを許可する。
- 同じkindを複数adapterが宣言する、schemaとregistry ownerが不一致、adapterが未知、またはcapabilityがない場合は操作を拒否し、ファイルを変更しない。
- `adapter`を欠く読取り互換Formは、登録済みStructured Test adapterのbuilt-in kind宣言またはschema compatibility matcherのうちちょうど1件だけがschemaを受理する場合に限って解決し、曖昧またはowner不在なら拒否する。matcherはschema内容から決定論的に判定し、coreは未知kindを`rust-cargo`へfallbackしない。
- Form Schemaの必須値と未知fieldを常に検証する。symbol、VO / Test参照、identifier、pathは選択したFormが該当fieldとvalidatorを宣言した場合だけ検証し、すべてのadapterへ一律に要求しない。
- create結果はscanで同じTest ID・intent・covers・targetsとして認識される。
- editは1 Testの拡張rangeだけを単一置換し、他Testと通常sourceを変更しない。
- 同じdesired stateの再適用は冪等になる。
- Structured Test capabilityがないadapterへのcreate / editはE-ADAPTER-004となり、ファイルを変更しない。

**Create の挿入後検証とロールバック**（別紙A §15.2、基本仕様 §15.1）

- create は挿入後に対象ファイルを再パースし、構文妥当性・挿入分がちょうど 1 Test として認識されること・その Test ID と annotation が desired state と一致すること・他の Test と通常 source が不変であることを確認する。edit と同じ確認項目を create でも実施し、create 経路にだけ検証を省く分岐を設けない。
- 挿入後の再パースが構文エラーになる fixture、挿入結果の annotation が desired state と一致しない fixture、挿入が他の Test 範囲へ及ぶ fixture のそれぞれで、create は E-OP-003・終了コード 2 になり、対象ファイルが挿入前のバイト列へ復元される。挿入によりファイルが新規作成されていた場合は不存在へ戻る。
- ロールバック後に scan すると、当該 create 操作が無かった場合と同一のエンティティ集合・内容ハッシュが得られる。部分適用された挿入内容・採番された Test ID・Evidence・判断記録がいずれも残らない。
- `create --dry-run` は挿入内容と挿入位置を提示し、ファイルを変更しない。
- 同一 desired state からの create と、その直後の同一 desired state による edit は差分を生じない（annotation block の再生成規則が create / edit で同一。別紙A §15.3）。

#### 18.3.11 MCP interface

- 別紙A（§12〜§15）が定める全 MCP tool が同じ入力に対するCLI JSONと同じdata / diagnosticsを返す。
- 不正入力はcode / message / candidatesを持つtool errorになる。
- request、notification、batch、malformed transportの各入力をJSON-RPC contractどおりに処理する。
- MCP serverの長時間実行中もsource変更を再scanし、staleなPASSを保持しない。

#### 18.3.12 adapter contract

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
- 明示操作に必須のcapabilityがなければE-ADAPTER-004となり、変更・判断記録・Evidenceを生成しない。
- 検証時のstatic audit / coverage capability欠落は `NO_EVIDENCE`（診断 `NOT_CHECKED`）、runner欠落は `NO_EVIDENCE`（診断 `NOT_EXECUTED`）、解析限界は `UNKNOWN` になる。
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

---

## 付記（非規範）: トレーサビリティ表

本表は別紙C の各節が実現する上流§と、その導出区分（CONFORM＝旧版から生存し引用・項目名の修復のみ／再導出＝旧構造を凍結モデルへ書き換え／新設＝旧版に無く上流から新規）を記録する。全節が上流（要件定義 / 基本仕様 / 本冊）へトレースでき、親を持たない節を作らないことを設計制約とする。

| 別紙C の節 | 実現する上流§ | 区分 |
|---|---|---|
| §18.1 共通条件 | 基本§22.1・§4.6・§6・§24 | CONFORM（Approval/Audit→承認記録/判断記録へ用語修復） |
| §18.2 共通fixture | 基本§3.1・§4.1・§4.2・§4.3・§5.2・§12・本冊§4.4・§5.6・§7.3 | 再導出（SPEC/REQ→document、role/anchor/characterization fixture 除去、8状態列挙→5状態＋4診断ラベル別記、静的監査 record fixture 除去、文書鎖・判断非昇格・gates fixture 追加） |
| §18.3.1 discovery・record・graph と chain_integrity | 基本§5.1・§9.2・§3.2・§17・§23・本冊§5・§6.1・§11.1.1 | 再導出＋新設（test_traceability→chain_integrity、role materialization/E-SCAN-013-015 除去、active REQ 束縛除去、covers≥1 一律、document derives_from・content_hash 照合の chain_integrity 受入を新設） |
| §18.3.2 orphan_detection | 基本§5.2・要件§4.2・本冊§5.6 | 新設（文書層孤児・根除外・E-SCAN-016） |
| §18.3.3 決定論的静的解析（oracle_presence・target_binding 静的到達） | 基本§5.4・§5.5・§8.3・本冊§7.1・§7.2・§7.3 | 再導出（static/semantic→oracle_presence、DA-002→target_binding 静的到達、Static Audit Record 永続・STALE・実効監査選択を撤去、subprocess E2E の2検査分裂を明示） |
| §18.3.4 execution・Evidence（target_binding の証拠） | 基本§6・§21・§21.1・本冊§3.6・§9・§11.2 | 再導出（test_execution/runtime_result/target_execution→target_binding 単一検査、evidence_validity→§6 ハッシュ束縛、target_coverage 改名、8→5状態写像） |
| §18.3.5 target_binding 動的計測（per-target） | 基本§5.3・§21・本冊§7.3・§10.2 | 再導出（target_execution 検査→target_binding 証拠、count0→FAIL/NOT_EXECUTED、per-target 生存） |
| §18.3.6 判断記録プロトコル（非ゲート） | 基本§11・§11.3・要件§12・本冊§8 | 再導出（意味監査 bundle 4種別・検査扱い廃止、impl_consistency=MISMATCH 写像削除、E-AUDIT-005-007 撤去、bundle/submit を非昇格の判断記録へ、理由 optional） |
| §18.3.7 承認と判断記録の分離 | 基本§4.5・§11.3・§17・本冊§3.4・§3.5 | 新設（判断≠承認・別 entity、承認は独立軸で非昇格、依存 closure hash 束縛、W-STORE-002） |
| §18.3.8 verify・report と scope | 基本§4.6・§5・§22.1・§22.2・§22.3・本冊§2.2・§11.1・§11.3 | 再導出（12項目→4検査、旧項目列挙→E-CONFIG-001 拒否、role別表示除去、SPEC/REQ→DOC ツリー、scope 2軸・NO_EVIDENCE を生む入力・§22.2 優先順位を明示） |
| §18.3.9 フェーズゲート評価 | 基本§20・§4.1・§4.5・要件§26.4・本冊§11.5・§2.2・§17.1・§17.2 | 新設（MUST・評価/提示のみ、自動遷移は責務外、既存 verify --gate 露出） |
| §18.3.10 Structured Test Operation | 基本§15・本冊§5.2 | CONFORM（項目名整合のみ） |
| §18.3.11 MCP interface | 基本§26.2・本冊§16 | CONFORM（別紙A 参照を収録節範囲へ修復） |
| §18.3.12 adapter contract | 基本§27・§2.4・本冊§5.2・§17.1 | CONFORM（NOT_CHECKED/NOT_EXECUTED を NO_EVIDENCE 診断へ、判断記録へ用語修復） |
| §18.4 提供範囲外 | 基本§29・要件R-2・R-3 | CONFORM |
