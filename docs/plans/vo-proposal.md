# SpecTracer v0.1 VO セット 起草提案（Owner 承認待ち）

外部検証エージェント（AI）による起草提案。**採否は Owner が承認する**（要件§12「発見・提案は外部 AI 可、裁定は外部」、§19 承認、F4）。VO 分解の十分性判定は機械検査でなく外部判断（要件§10.2/§12・基本§10）であり、本書は Owner レビュー用の草案。確定スペック5文書（要求・要件定義 FROZEN / 基本仕様 / 詳細設計本冊 / 別紙A / 別紙C）のみを根拠に起草（別紙B=実装計画は obligation 源にしない）。

規模: 親 VO 19本 / leaf 約80本。

## 方針
- 1 leaf = vtest 挙動についての独立検証可能な1命題。partition はスペックが名指しするもの（per-target verdict / 5状態・4診断ラベル / 境界 topology / DA-001..006 / §11.2 鮮度5条件）でのみ分割。
- エスカレーション領分（分解十分性・意味一致・網羅妥当性 = 要件§10.2/§12）と OOS は VO 化しない。
- derives_from（推奨案）: 各 VO を obligation を最も具体・検証可能に述べる最下流の単一 DOC へ張る（多重も schema 上合法 → 裁定候補①）。上流§はトレース列に併記。
- DOC 命名（仮）: DOC-REQ=要求・要件定義 / DOC-BASIC=基本仕様 / DOC-DETAIL=詳細設計本冊 / DOC-ANNEX-A=別紙A / DOC-ANNEX-C=別紙C。
- parent は集約ノード（covers なし）、leaf のみ covers（双方向完全性で leaf VO は covers≥1）。命名 `VO-<領域>-<小領域>-<連番>`。

## 領域一覧（親 VO 19本）
DOCMODEL（総称 document・宣言鎖）/ CHAIN（検査①chain_integrity）/ ORPHAN（検査②）/ TARGET（検査③target_binding）/ ORACLE（検査④oracle_presence）/ STATE（5状態+診断ラベル2軸）/ EVIDENCE（ハッシュ束縛・鮮度）/ AUTHORITY（判定権威=runner）/ DECISION（判断記録・非ゲート）/ APPROVAL（承認と検証状態独立）/ GATE（フェーズゲート提示のみ）/ SCOPE（scope 2軸）/ AGG（fail-closed 集約）/ ADAPTER（境界・SRC identity・wire 互換）/ EXIT（UNKNOWN 検疫・終了コード）/ ONBOARD（途中導入・pending）/ TRACE（トレーサビリティ・projection）/ STO（Structured Test Operation）/ IFACE（MCP=CLI 同一性）

---

## leaf VO 一覧（領域別: ID / 命題 / derives_from / トレース§ / 想定 covers）

### VO-DOCMODEL（総称 document・宣言鎖）
- **DOCMODEL-01** document は単一総称ノード型(id+path+content_hash+derives_from)で種別専用スキーマを持たない ← DOC-DETAIL / REQ§3.2・BASIC§3.1・DETAIL§3.1 / doc reader 種別非分岐
- **DOCMODEL-02** VO は1件以上の document から derives_from 直結導出、間に他 entity 層なし(旧 requirements/spec_refs 無) ← DOC-DETAIL / DETAIL§3.2
- **DOCMODEL-03** 関係型(derives_from/covers/検証対象/実装traceability)を単一へ潰さず区別 ← DOC-BASIC / REQ§3.4・BASIC§19
- **DOCMODEL-04** リンク説明文(note)は任意、空でも chain_integrity 違反・MISMATCH にしない ← DOC-BASIC / BASIC§3.4・DETAIL§3.1
- **DOCMODEL-05** 文書層の段はリンク、段追加でスキーマ非破壊・検査非増加 ← DOC-REQ / REQ§3.2/§3.3

### VO-CHAIN（chain_integrity）← 主 DOC-ANNEX-C§18.3.1 / DETAIL§11.1.1
- **CHAIN-DOC-01** document derives_from 参照先不在→E-SCAN-012,MISMATCH
- **CHAIN-DOC-02** content_hash 実ファイル不一致→W-SCAN-104,MISMATCH(STALE) ← DETAIL§11.4
- **CHAIN-VO-01** 各VOが解決可能な derives_from(document)1件以上、不在→E-SCAN-012,MISMATCH
- **CHAIN-VO-02** VO parent 不在・循環→E-SCAN-008,MISMATCH
- **CHAIN-TEST-01** 管理宣言(Test ID・covers≥1・intent・adapter必須metadata〔rust-cargo:targets≥1〕)ちょうど1件、欠落→E-SCAN-007/W-SCAN-101,MISMATCH/MISSING
- **CHAIN-TEST-02** 全Testに covers≥1 一律、covers0→MISMATCH/MISSING(role可変制約なし) ← BASIC§12
- **CHAIN-TEST-03** covers 全VO参照解決、不能→E-SCAN-003,MISMATCH(Entity除去せず)
- **CHAIN-TEST-04** Test ID 大局的一意、衝突→E-SCAN-002,MISMATCH
- **CHAIN-BIDIR-01** covers Test 無 leaf VO→MISMATCH/MISSING(双方向完全性)
- **CHAIN-REL-01** Relation from/to 不在→E-SCAN-009、ULID正規化不一致→E-SCAN-010
- **CHAIN-DISC-01** discovery失敗を Test0件正常scanとせず(Incomplete→UNKNOWN)

### VO-ORPHAN（orphan_detection）← 主 DOC-ANNEX-C§18.3.2 / DETAIL§5.6
- **ORPHAN-01** 親無し＆doc.roots非列挙 document→E-SCAN-016,orphan_detection=MISMATCH
- **ORPHAN-02** doc.roots 列挙DOCを根として対象外
- **ORPHAN-03** 文書層のみ対象、実装レイヤー孤児検出せず ← DOC-BASIC / R-2・OOS-005 / 未宣言実装が orphan にならない negative
- **ORPHAN-04** doc.roots が存在しないDOC参照→E-CONFIG-001

### VO-TARGET（target_binding）
- **TARGET-STATIC-01** DA-002 対象未呼出(境界=関数本体+同一file helper1段)呼出無→target別FAIL、他file/クレート→UNKNOWN ← DOC-DETAIL / DETAIL§7.2 / covers=既存 TEST-DOGFOOD-M3-TARGET-RULES(classify_target_call)
- **TARGET-STATIC-02** DA-002 verdict=FAIL は runtime 証明で覆らない ← DETAIL§7.3
- **TARGET-RT-01** per-target count≥1→PASS/0→FAIL(NOT_EXECUTED)/同定不能→UNKNOWN ← ANNEX-C§18.3.5・DETAIL§10.2
- **TARGET-RT-02** DA-002 UNKNOWN の target は runtime target_coverage=PASS(checked:true,count>0)時のみ到達充足 ← DETAIL§7.3
- **TARGET-MULTI-01** 複数target集約=1件FAIL→FAIL/FAILなくUNKNOWN→UNKNOWN/全PASS→PASS ← ANNEX-C§18.3.5
- **TARGET-BOUNDARY-01** subprocess/別スレッド境界越えは DA-002静的UNKNOWN、coverageが帰属できれば target別PASS ← ANNEX-C§18.2/§18.3.5・DETAIL§7.3
- **TARGET-RESULT-01** 有効Evidence result:FAIL→target_binding=FAIL ← DETAIL§11.2・REQ§5.3
- **TARGET-PASS-01** result:PASS＋全target到達充足→PASS、未充足は count0→FAIL(NOT_EXECUTED)/checked:false→NO_EVIDENCE(NOT_CHECKED)/不見当→UNKNOWN ← DETAIL§11.2
- **TARGET-CONTRACT-01** どのtopologyでもtarget非実行の契約のみTestは到達未充足のまま ← ANNEX-C§18.3.3
- **TARGET-CAP-01** coverage capability/tool不在→NO_EVIDENCE(NOT_CHECKED)、解析限界→UNKNOWN ← ANNEX-C§18.3.5
- **TARGET-NODA003-01** runtime coverage は DA-003 を代替せず、DA-003 UNKNOWN/FAIL はそのまま oracle_presence へ寄与 ← DETAIL§7.3

### VO-ORACLE（oracle_presence）← 主 DOC-DETAIL§7.2
- **ORACLE-DA001** 定数アサーション(assert!(true))→FAIL
- **ORACLE-DA003** target呼ぶが結果未検証→FAIL、可変参照/グローバル経由可能性→UNKNOWN
- **ORACLE-DA004** 自己比較 assert_eq!(a,a)→FAIL
- **ORACLE-DA005** 空テスト本体→FAIL
- **ORACLE-DA006** assert相当1つも無→FAIL
- **ORACLE-COMPOSE** DA-001/003/004/005/006合成(全PASS→PASS/1FAIL→FAIL/FAILなくUNKNOWN→UNKNOWN) ← DETAIL§7.1・BASIC§5.4
- **ORACLE-NOPROMOTE** runtime昇格経路なし、証明失敗はUNKNOWNでruntimeでPASSにしない ← DETAIL§7.1
- **ORACLE-CONSERV** 確定違反のみFAIL、クロージャ内・マクロ展開内はUNKNOWN ← DETAIL§7.2
- **ORACLE-SUBPROC-SPLIT** 本体に呼出無 subprocess E2E は DA-003 UNKNOWN残存→oracle=UNKNOWN、一方 target_binding=PASS到達しうる2検査分裂、総合NG ← ANNEX-C§18.3.3・DETAIL§7.3
- **ORACLE-RECALC** 静的解析は正典レコードを持たない再計算派生、監査レコード永続化しない ← DETAIL§7.1
- **ORACLE-FRAGMENT** source fragment 完全性を保証できない場合 UNKNOWN ← ANNEX-C§18.3.3

### VO-STATE（5状態+診断ラベル2軸）
- **STATE-01** 検証状態は5値のみ(PASS/FAIL/MISMATCH/NO_EVIDENCE/UNKNOWN) ← BASIC§4.1・ANNEX-C§18.2
- **STATE-02** 診断ラベル4種(MISSING/NOT_EXECUTED/NOT_CHECKED/STALE)は状態と別軸、state値に用いず併記のみ ← ANNEX-A§12.1・BASIC§4.2
（注: §5.3 事象→状態割当は各検査領域 leaf に内包し重複させない）

### VO-EVIDENCE（ハッシュ束縛・鮮度）
- **EVIDENCE-BIND-01** 証拠は検証対象の内容ハッシュに束縛、ストアはハッシュキー必須 ← REQ§6・BASIC§6
- **EVIDENCE-FRESH-SUBJECT** test_subject hash不一致(非隣接metadata変更含む)→NO_EVIDENCE(STALE) ← DETAIL§11.2
- **EVIDENCE-FRESH-TARGET** target参照集合/construct hash不一致→NO_EVIDENCE(STALE)
- **EVIDENCE-FRESH-REVISION** revision.commit不在/現HEAD不一致→NO_EVIDENCE(STALE)
- **EVIDENCE-FRESH-EXECSTATE** Execution State subject(target外helper/local dep/runner/toolchain/config)不一致→NO_EVIDENCE(STALE)、complete不能→UNKNOWN ← ANNEX-C§18.3.4
- **EVIDENCE-FRESH-ADAPTER** adapter ID不一致→MISMATCH、確認不能→UNKNOWN
- **EVIDENCE-GEN-01** 全target一意解決を生成precondition、1件でも対象無/曖昧→生成せず(部分targets不可)→NO_EVIDENCE(NOT_EXECUTED) ← ANNEX-C§18.3.4・DETAIL§9.4
- **EVIDENCE-GEN-02** build/runner failure・capability欠落・target解決失敗・実行前後 Execution State変化(E-EXEC-004)で生成しない
- **EVIDENCE-NOFALLBACK** 最新EvidenceがSTALE時、古い有効Evidenceへフォールバックしない ← DETAIL§7.3
- **EVIDENCE-HASH-CORE** 内容ハッシュはadapter自己確定せずcoreが言語非依存正規化(hash未計算DTO) ← BASIC§6・DETAIL§1.3

### VO-AUTHORITY（判定権威）
- **AUTHORITY-01** 合否判定権威は adapter runner(rust-cargo=cargo test)、vtest は再判定せず結果を証拠消費 ← REQ§7・BASIC§7
- **AUTHORITY-02** target_binding は runner PASS を前提に「実行を伴ったか」を問う独立照合 ← BASIC§7

### VO-DECISION（判断記録・非ゲート）← 主 DOC-ANNEX-C§18.3.6 / DETAIL§8
- **DECISION-NONGATE** 判断記録の受理は検証状態(5状態)を昇格させない
- **DECISION-SUBMIT** submit は bundle存在(E-AUDIT-001)/subject一致(003)/hash一致(002)/decision受理値(004)を検証
- **DECISION-REASON-OPT** actor/subject/decision必須、理由optional。理由空を根拠に無効/UNKNOWN/NO_EVIDENCE/MISMATCH扱いしない(旧E-AUDIT-005-007撤去) ← REQ§12・BASIC§11.3
- **DECISION-HASHBIND** 受理判断はsubject_hash＋依存closure束縛、変更で失効、document hash不一致で依存判断も無効 ← DETAIL§8.5
- **DECISION-REVERIFY** 変更後は流用せず4検査再実施、結果は5状態いずれにもなる(変更自体はUNKNOWNを生成しない) ← REQ§12
- **DECISION-BUNDLE** bundleはcache/bundles/へ出力(派生・Git管理外)、target一意解決不能なら生成せず候補選択しない ← DETAIL§8.1

### VO-APPROVAL（承認と検証状態独立）← 主 DOC-ANNEX-C§18.3.7 / DETAIL§3.5
- **APPROVAL-INDEP** 承認は独立軸、承認済みで非PASS→PASS昇格せず未承認でPASS→降格しない ← BASIC§4.5/§17
- **APPROVAL-DISTINCT** 判断済み≠承認済み、判断記録と承認記録は別entity可
- **APPROVAL-CLOSURE** VO実効承認=subject_hash一致＋依存closure(再帰parent VO・derives_from document・上位document)完全一致の承認1件以上でapproved、他はdraft
- **APPROVAL-STALE** 対象/依存成果物変更(document再登録含む)で失効
- **APPROVAL-NOCLOSURE** closure/hash欠く互換Approvalはapproved導出せず(W-STORE-002)、作成時解決不能→E-APPROVAL-001でrecord生成せず
- **APPROVAL-STATUS-DERIVED** VO statusは正典fieldでなく承認から導出(writer保存せずreaderはW-STORE-001無視) ← DETAIL§3.2
- **APPROVAL-RECORD** approver(種別human/agent+id)/subject or judgment_ref/approved_state必須、根拠optional

### VO-GATE（フェーズゲート・提示のみ）← 主 DOC-ANNEX-A§12.3
- **GATE-EVAL** verify --gate が(1)検証結果=require.verification充足(2)require.approvals各ロール有効承認存在、を評価し満否と根拠提示、新規cmd/tool増やさず既存verify/reportで露出 ← ANNEX-C§18.3.9
- **GATE-NOTRANSITION** 責務は評価・提示に限る、フェーズ自動遷移せず ← BASIC§20・REQ§26.4
- **GATE-ROLE-RESOLVE** approval_rolesでロール→approver id解決、gates参照ロール未定義→E-CONFIG-001 ← ANNEX-A§12.3

### VO-SCOPE（scope 2軸）← 主 DOC-ANNEX-C§18.3.8
- **SCOPE-2AXIS** 検査軸(4本部分集合)＋エンティティ軸(DOC/VO/Test部分木)で限定可 ← BASIC§4.6
- **SCOPE-NOPROMOTE** scope外・未実施はNO_EVIDENCE(NOT_CHECKED)保持でPASS化せず、要求scopeとscope外未検証を併記
- **SCOPE-NODEGRADE** いかなる設定値も完全検証を4本未満へ縮退させず、--items省略は固定4検査
- **SCOPE-FULLSCOPE-INV** 旧12項目列挙はversion問わずE-CONFIG-001、version1欠落のみ固定4検査へ具体化(in-memory補完なし) ← DETAIL§2.2
- **SCOPE-INTERNAL-DEP** 表示scopeと内部依存評価を分離 ← ANNEX-C§18.3.3

### VO-AGG（集約・完全検証OK）← 主 DOC-BASIC§22
- **AGG-FAILCLOSED** 完全検証OKは4検査全PASS＋証拠が§6満たす場合のみ、1項目でも非PASSでNG ← REQ§26.1
- **AGG-TREE** Test単位結果をVO/Feature/document単位へfail-closed集約(子1つでも非PASSで親非PASS)
- **AGG-PRIORITY** 代表値優先順位 FAIL>MISMATCH>NO_EVIDENCE>UNKNOWN、診断ラベルは順位に用いず併記 ← BASIC§22.2
- **AGG-UNREG-NG** 他検査全PASSでも未登録Test1件でchain_integrityにより総合NG ← ANNEX-C§18.3.8
- **AGG-DRILLDOWN** NG時どのentity・検査・状態・診断ラベルで落ちたか掘下げ可、非PASS根拠を辿れる、text/JSON両出力 ← BASIC§22.3

### VO-ADAPTER（境界・SRC identity・wire互換）← 主 DOC-ANNEX-C§18.3.12
- **ADAPTER-NEUTRAL** 検証契約・ID・ハッシュ・Evidence・状態・集約はlanguage/runner非依存、TestEntityは関数でなくExecutionDescriptorのみ ← BASIC§27
- **ADAPTER-REGISTER** core変更なしで別adapter登録可、registry重複ID/未登録/capability実装不一致/adapter間Test ID重複を拒否
- **ADAPTER-NOPROMOTE** 未登録・能力不足・解析不能でPASS昇格せず ← BASIC§22.3・ANNEX-A§12.1
- **ADAPTER-WIRE** config v1/v2 reader受理・読取りで書換えず、writer/initはv2、Test JSONはexecution常時＋rust-cargoのみ互換field、targets常時list＋単数互換は1件時のみ ← BASIC§2.4
- **ADAPTER-MERGE** adapter結果を決定論的統合、同root共有可・同一adapter内root重複拒否、全merge結果でTest ID global uniqueness検査
- **ADAPTER-SRC-IDENT** Source Targetはcanonical locator＋任意恒久SRC ID併有の単一entity、複数target独立保持(縮約せず)、Test↔SRC双方向 ← BASIC§9.2/§3.3
- **ADAPTER-SRC-DUAL** 恒久SRC ID持つtargetはlocatorでもaddressable、両modeで同一hash・同一identity ← ANNEX-C§18.3.1
- **ADAPTER-SRC-CANON** identityは宣言TargetRef→解決→canonical Locatorの一方向確定、Evidence/判断記録は解決後canonicalを記録、綴り違い複数target同一解決→E-SCAN-005 ← DETAIL§6.1.1
- **ADAPTER-SRC-RESOLVE** 解決は解決済/対象なし/曖昧を区別、曖昧はfail-closed終端(候補を解決結果に記録せず診断表示のみ)、coreの単一経路が所有
- **ADAPTER-SRC-UNIQ** 恒久SRC IDは全adapter統合後repository全体で一意、衝突→E-SCAN-011、衝突しても各targetはcanonical locatorで独立具体化

### VO-EXIT（UNKNOWN検疫・終了コード）
- **EXIT-QUARANTINE** UNKNOWNは正常動作の降参でエラーfallback先に使わず、内部エラー・入力不正は終了コードで別系統 ← REQ§5.4・BASIC§4.4
- **EXIT-CODES** 0=要求scope OK/1=検証NG/2=操作拒否/3=内部エラー、E-ADAPTER/E-CONFIG→2,E-SCAN→1,none→0 ← BASIC§26.1・ANNEX-A§12.1

### VO-ONBOARD（途中導入・既存資産）← 主 DOC-BASIC§18
- **ONBOARD-VISUALIZE** 既存大量コード・Testを対象にでき、未登録Test/欠落宣言/未確定VO/未実施検査を検証済み扱いしない ← REQ§17
- **ONBOARD-INIT** vtest init は.verify/作成し既存コード非改変、一部欠落状態も読取可
- **ONBOARD-PENDING** 判断待ち情報を機械可読構造(pending: subject/kind/check/basis/bundle_ref)としてverify/report JSONへ横断集約 ← DETAIL§11.7・ANNEX-A§12.4

### VO-TRACE（トレーサビリティ・projection）← 主 DOC-DETAIL§11.6
- **TRACE-ANYNODE** 最小単位「上流→関係→下流」を任意ノードから取得、上下流連続追跡・全体構造取得、全チェーン常時表示は求めず ← REQ§3.4・NFR-003
- **TRACE-INDEX** 逆引き(VO→Tests/SRC→Tests/DOC→VOs/DOC→DOCs)を正典から再構築(派生) ← DETAIL§5.3・NFR-004
- **TRACE-PROJECTION** 役割別projection(pm/tester/coder view)を同一構造から粒度変えて取得、役割固定enum化せず ← BASIC§19

### VO-STO（Structured Test Operation）← 主 DOC-ANNEX-A§15
- **STO-DESIRED** Create/Editはdesired state方式、adapter差分計算＋core再スキャン検証、再適用は冪等 ← BASIC§15.1・ANNEX-C§18.3.10
- **STO-1TEST** Edit一回対象は原則1 Test、拡張rangeの単一置換で他Test・通常source非改変(前後二重検査) ← ANNEX-A§15.4
- **STO-INPUT-VALIDATE** 構造化入力を受理時検証(symbol/Test ID/参照VO不在→候補提示)、Form必須値・未知fieldを常に検証
- **STO-FORM-RESOLVE** Form kindはrepository全体で一意＋owner adapter別field宣言、1件解決時のみcreate/form_get許可、重複/未知/曖昧/capability無を拒否しファイル非改変 ← BASIC§15.4
- **STO-HELPER-OOS** helper/fixture/通常source編集手段を提供しない(OOS-003) ← ANNEX-A§15.4

### VO-IFACE（MCP=CLI 同一性）← 主 DOC-ANNEX-C§18.3.11
- **IFACE-PARITY** 全MCP toolが同一入力にCLI JSONと同じdata/diagnostics返す、CLI/MCPは同じregistry composition・JSON envelope・adapter選択エラー ← BASIC§26.2
- **IFACE-JSON-RPC** request/notification/batch/malformed transportをcontract通り処理、不正入力はcode/message/candidates付tool error
- **IFACE-RESCAN** MCP長時間実行中もsource変更を再scan、staleなPASS保持せず(mtimeベース判定) ← ANNEX-A§13.1

---

## 既存 VO の処遇案
`.verify/vo/VO-DOGFOOD-M3-STATIC-AUDIT.yaml`（claim: "Static rules bind the declared target and result flow without promoting ambiguity to PASS"）:
- **再ターゲット先**: covering test `TEST-DOGFOOD-M3-TARGET-RULES`(classify_target_call) は DA-002 の target 解決寄り → **VO-TARGET-STATIC-01 へ改名・再ターゲット**推奨。claim が "result flow" にも触れる点は VO-ORACLE-DA003 に跨る（裁定候補⑦）。
- **schema 移行必須**: 現ファイルは旧モデル(requirements/spec_refs/status)。新 schema(derives_from/parent/claim/dimensions/coverage_policy)へ書換え要。status は W-STORE-001 を誘発する非正典 field。
- code 側 `crates/vtest-audit/src/lib.rs:1907` の `@vtest.covers` も新 VO ID へ追随（=実装 PR、#13 スコープ外）。

## 網羅の自己評価（「十分」とは断定しない — 十分性判定は Owner）
- **意図的に leaf 分割**: DA-001..006(供給先検査が異なる=DETAIL§7.2)、鮮度5条件(§11.2)、per-target verdict と境界topology(ANNEX-C§18.3.3/5)、SRC identity の dual/canon/resolve/uniq(ANNEX-C§18.3.1)。
- **意図的にまとめた**: §5.3 事象→状態割当は各検査 leaf に内包。NFR は各領域 leaf の副次性質として吸収。
- **VO 化しなかった**: 分解十分性・意味一致・網羅妥当性(要件§10.2/§12=検査外)、OOS-001..005、別紙B 実装計画。

## Owner 裁定候補（ACCEPT/REJECT/修正の単位）
1. **derives_from 単一 vs 多重**: 推奨=最下流単一DOC＋トレース列で上流§。多重も schema 合法。
2. **negative/boundary 命題の leaf 化**: ORPHAN-03 / GATE-NOTRANSITION / STO-HELPER-OOS / ORACLE-NOPROMOTE / DECISION-NONGATE 等「〜を行わない」不変条件を独立 leaf にするか parent note へ畳むか。推奨=独立 leaf(重要な fail-closed 不変条件)。
3. **命名段数**: 3段(CHAIN/TARGET/EVIDENCE/ADAPTER)と 2段の混在を統一するか領域裁量か。
4. **§5.3 割当表の扱い**: 各検査 leaf へ内包(推奨・重複回避)か、1 parent VO へ正典参照点として集約か。
5. **SRC target identity の所属**: VO-ADAPTER 配下(推奨)か独立領域 VO-SRC へ切出すか。
6. **static_audit 系の分割方針**: DA-002→target_binding、DA-001/003-006→oracle_presence へ分割(DETAIL§7.2 の供給先に従う)を承認するか。
7. **既存VOの再ターゲット先**: VO-TARGET-STATIC-01 単独 covers か、DA-002＋DA-003 の2 leaf へ分割 covers か。
