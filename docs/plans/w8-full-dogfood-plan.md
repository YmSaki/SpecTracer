# W8 完全 dogfood 実装計画 (Option A)

Owner 決定: **(A) 完全自己管理** — 194 テスト全てを @vtest 管理し、全12項目 PASS の完全 dogfood を構築。
branch: `feature/adapter-separation-alpha2-implementation`

## W8 Final dogfood gate（binding な完了条件 / 作業計画 ~L706-727）
```
all unit tests managed
test_traceability = PASS
Specification → REQ → VO → Test complete
Approval current / Static Audit current / Semantic Audit current
Evidence bound to current HEAD / complete Execution State
full target measurement
all 12 items PASS
text / JSON report consistent
CLI / MCP parity
no STALE / MISSING / UNKNOWN path can become PASS
```
→ W8=PASS, Implementation=DONE, READY_FOR_PR=YES

## 集約セマンティクス（一次情報 crates/vtest-verify/src/lib.rs）
- `verify_project_scoped` L138-142: report.result = requested_scope 全項目を `fold(Pass, combine_values)`。
- `combine_values` L1640: PASS は単位元、非PASS が支配（rank: Fail8>Mismatch7>Missing6>Stale5>NotExecuted4>NotChecked3>Unknown2>Pass1）。
- **⇒ report=PASS には scope 内全項目 PASS 必須。** default scope = `config.verify.full_scope`（11項目）+ test_traceability。
- **target_execution も full_scope。checked:false→NOT_CHECKED→PASS阻む。∴ 194証跡は全て measured llvm-cov 実行（--fast 不可）。**

## 12項目の性質
| item | 種別 | 生成手段 |
|---|---|---|
| spec_coverage | 構造 | SPEC-REQ chain |
| vo_decomposition | 構造 | REQ-VO chain |
| vo_coverage | 意味(LLM) | `audit bundle --kind vo-coverage` → submit |
| test_existence | 構造 | scan |
| test_traceability | 構造 | 全テスト @vtest 管理（W-SCAN-101=0） |
| static_audit | 決定論 | `audit static` |
| semantic_audit | 意味(LLM) | `audit bundle --kind test-semantic` → submit |
| impl_consistency | 意味(LLM) | `audit bundle --kind impl-consistency` → submit |
| test_execution | 証跡 | `run`（非fast） |
| runtime_result | 証跡 | `run` 結果 PASS |
| target_execution | 証跡 | `run` measured coverage |
| evidence_validity | 証跡 | HEAD束縛+鮮度 |

## CLI flow（subagent 委譲可能 — MCP 不要）
- `vtest audit static --test <id>` / `--all`
- `vtest audit bundle --kind <kind> --test/--vo <id>` → bundle 出力（監査文脈）
- 監査者(LLM)が bundle をレビュー → verdict file 作成 → `vtest audit submit --file <f>`
- `vtest vo approve <VO>` （Approval current）
- `vtest run --test/--vo/--all`（非fast=llvm-cov measured coverage; config.run.coverage=llvm-cov）
- `vtest verify --items <csv> --test/--vo/--req <id>`

## 倫理原則（advisor）
意味監査は**必ず読んで根拠(basis line-refs)付きで判定**。機械的 PASS 量産は「空虚な PASS」= ツールが排除する対象そのもの。genuine FAIL は VO/マッピング側で修正、verdict 反転で誤魔化さない。main thread が sample を spot-check。

## 194 未管理テスト分布
src(unit) 約89: scan/lib(17) static_audit(11) verify/lib(11) store/records(9) model(8) store/lib(8) cli/lib(6) approval(6) runner(5) adapter-api(3) forms(3) adapter-rust/lib(2)
tests(integration) 約105: w1_acceptance_binding(13) adapter_acceptance(多) m1-m9_acceptance ほか

## 実行順序（advisor 承認）
1. **[済] W8 gate 原文確認** — (A) と一致。
2. **Pilot 1**: 既存 M3 chain を test_traceability 以外の全 full_scope 項目 PASS 化。static 再生成 + genuine semantic/impl/vo-coverage bundle+submit + vo approve + measured coverage run。→ **per-test コスト測定**。（test_traceability は 194 未注釈ゆえ MISSING のまま＝想定内、Pilot 失敗ではない）
3. **Pilot 2**: subprocess acceptance テスト **1件**を rust-integration form で管理 → static_audit PASS 可能か判定。**構造的に不可能なら Owner エスカレーション（唯一の停止トリガ）**。
4. **Ontology**（main thread, docs/plans doc）: 残る canonical docs を追加 SPEC 登録（additive .verify、§15 不変）。REQ クラスタ（purpose 宣言由来）。VO ~30-60件（194テストが自然に対応）。不変条件: dangling VO 無/empty covers 無/TEST ID repo-global 一意。
5. **一括注釈**（subagent 委譲, per-file, main-thread 供給の test→(ID,VO,target) マップ。subagent は適用のみ、発明しない）→ **freeze: 注釈を全コミットしてから audit/evidence 生成**（doc-comment 編集は construct hash を変える）。
6. **生成**: scripted audit-static + measured run（background, 数時間想定）; semantic bundle は subagent+rubric。→ full verify → 残 W8 gate（verify-change, architecture-check, release-check --milestone M9, independent reviewer, final dogfood 宣言）。

## commit 規律
records / annotation / generated-audit-evidence を**別コミット**（test-vs-impl 分離と同じ）。Pilot が実装バグを露出したら独立コミットで修正。store は append-only（verify は executed_at 最新を採用）— stale は消さず追加。

## Pilot 1 結果（実証済み）
per-test 全経路を M3 chain で検証。**12項目中 9項目 PASS** 到達:
- 構造: spec_coverage / vo_decomposition / test_existence = PASS
- static_audit = PASS ← **修飾呼び出しが必須**（下記 finding A）
- semantic_audit = PASS（genuine test-semantic bundle→submit, verdict PASS, 根拠 basis 付き）
- 証跡: test_execution / runtime_result / target_execution / evidence_validity = PASS
  - **measured llvm-cov 動作確認**（Windows, ~14s cached）: target_execution checked:true, method:llvm-cov, classify_target_call を count 1 実行
  - evidence_validity は dirty tree でも PASS（HEAD commit + execution_state hash 束縛; 木が変わらなければ有効）
- 未到達: vo_coverage / impl_consistency（NOT_CHECKED, 同一 submit flow で到達可）, test_traceability（MISSING, 194 未注釈ゆえ想定内）

### 重大 finding（scope に影響）
- **finding A — 修飾呼び出し必須**: `mod tests` 内 unit テストが親モジュールの target を **bare 呼び出し**すると DA-002 は別モジュール判定で Ambiguous → UNKNOWN。`super::target(...)` 等の修飾で Proven=PASS。M3 テスト自身が移動時に `crate::` 修飾を落として回帰していた（`super::` で修正済・未コミット）。→ **~89 unit テストは注釈だけでなく target 呼び出しの修飾（本体編集）が要る**。
- **finding B — subprocess acceptance テストは static_audit PASS 不可能（構造的）**: `Command::new(vtest)` / MCP stdio で駆動するテストは target Rust シンボルを in-process で呼ばない → DA-002 FAIL/UNKNOWN。全12項目 PASS に到達不能。（分類数は subagent 集計待ち）
- **finding C — scope escape**: `config.scan.include` を `crates/*/src` 群に絞れば tests/ 統合テストを除外可能（dir 単位; 混在ファイルは分離不可）。
- **finding D — VO は複数テスト必須**: vo_coverage COMPLETE には VO claim を網羅する複数テストの mapping が要る。単一 narrow テストは INCOMPLETE。→ ontology で VO 粒度を「mapped テスト群で claim を網羅」する設計に。
- **finding E — manifest churn**: execution_state manifest が `.claude/agent-memory`・`.agents`・`docs`・`__pycache__` 等の全ツリーを含む。任意のファイル変更で全証跡 STALE 化。→ dogfood は point-in-time snapshot; 生成→コミット→検証を frozen tree で atomically 実施する必要。

## ★PIVOT（2026-08-14）— 仕様修正フェーズ突入。従来の unit-scope 案は SUPERSEDED
Pilot で「(A) 全194・全12 PASS は構造的に不能」と判明（統合104件中99件が subprocess/mixed/structural）。Owner 診断確定: **根本原因は仕様が process-boundary / black-box Test topology を監査モデルに入れていなかったこと**。Owner 決定:
- subprocess acceptance は**意図した E2E 検証**。削除も DI 化もしない。
- 乙「integration-* → static_audit N/A」は**却下**（DA-001/004/005/006 は subprocess にも適用可。test kind と execution topology は別概念）。
- **正しい解**: 正規仕様に「DA-002/003 の source-level target reachability 検査の**適用可能性**」と「**runtime target_execution による到達証明**」の関係を定義。**静的に証明できない target 到達（他ファイル/クレート・クロージャ・マクロ・thread・process の全境界を一般化）を runtime evidence が担保**するモデル。multithread も同一クラス（§960 クロージャ=UNKNOWN）として一般化で解決。
- fail-closed 保持: 到達証明 = static-PASS **または**(static-UNKNOWN **かつ** runtime-coverage-PASS)。FAIL は rescue 不可、coverage NOT_CHECKED/不在は未達。

### 手順（Owner 指定）
1. **仕様変更 PR を `develop`（Dev ブランチ）から `spec/*` ブランチで生成** → Owner がマージ。
2. マージ後、**別紙B 実装計画に追加** → 必要な W まで戻って実装修正。
（別紙B はこの PR に含めない＝post-merge。W-rework は投機的に先行しない。）

### 仕様ドラフトの必須論点（advisor）
- **gate 衝突**: 「no UNKNOWN→PASS」invariant を literal に保つ。2案: (a) DA-002/003 に UNKNOWN と別の「runtime-deferred」outcome を新設し §935 写像拡張 / (b) rule は UNKNOWN のまま、static_audit **item 導出**（§935+§1262表）を条件付き化（reachability = DA-002/003 PASS or 未証明∧当該 test の target_execution=PASS、basis に「runtime evidence で確立」明記）。PR で preservation 論証を明示。§961 の per-target 合成を貫通。
- **DA-003 ≠ DA-002**: coverage は「実行」を証明するが「結果検証」ではない。境界 test の DA-003 の関心（結果が assert に到達）は test 自身の boundary-observable 出力 assert（=runtime_result）が担う。002/003 を同一視しない。
- **STRUCTURAL 14件は本モデルで未解決**（実行 target が無いので coverage で rescue 不可）。PR で structural-test の扱いを定義するか、明示的に open Owner item として scope 外にする（W8 gate で不意打ちにしない）。w1 は 5 IN_PROC と 8 STRUCT 同居に注意。
- **empirical spike（実行中）**: m8 subprocess test を measured-run し、coverage が vtest シンボルへ実行帰属するか確認。失敗すれば runtime 担保に機構が無い → PR に必要実装 or coverage capability gating を明記。
- **整合性 sweep**: 基本仕様 §4.2/§7.9, 別紙C §93/§118, 詳細設計 §935/§953/§960-962/§1262-1267 の fail-closed 文を全 doc 横断で更新（1 doc 変更で他 doc に矛盾文が残ると contract 違反）。
- **follow-up**: 詳細設計変更で SPEC-DOGFOOD-M3.yaml sha256 が stale → 再登録要（既に1回実施済）。

## 進捗
- [x] doctor exit 0（記録同期コミット 0fae5ef）
- [x] Pilot 1 実証（9/12 PASS, per-test 全経路, finding A-E）＋ M3 `super::` 修飾修正（未コミット）
- [x] Pilot 2 分類確定（IN_PROC 5 / MIXED 7 / SUBPROC 78 / STRUCT 14）→ Owner 報告済
- [x] Owner 決定: 仕様修正フェーズ（上記 PIVOT）
- [x] subprocess coverage spike: cargo-llvm-cov は subprocess の実行を vtest シンボルへ**帰属しない（0.00%）** → §10.2/§7.9 に coverage capability 要件として明記
- [x] 仕様変更 **PR #5**（spec/target-reachability-proof → develop）作成。https://github.com/YmSaki/SpecTracer/pull/5
      - 詳細設計 §7.1/§7.3(新)/§10.2/§11.1、基本仕様 §7.2/§7.9、別紙C §18.3.2/§18.3.6
      - モデル: DA-002 到達 = 静的 OR runtime target_execution。DA-003 据置。fail-closed literal 保持。
      - open item: subprocess coverage 帰属は実装課題（spike 実証）／STRUCTURAL は本モデル未解決
- [x] Owner REQUEST CHANGES（1 blocker）: モデルは正しいが複数 target の join 評価に必要な **target 別 static verdict が canonical record に無い**（DA-002/003 は fold 済み test 単位のみ）。「情報モデルを先に仕様化」。
      → 追加2コミット: (1fbe73d) 情報モデル — static record の DA-002/003 に per-target verdict list（§3.6/§7.1 adapter契約/§7.2 fold・version bump/別紙A wire/別紙C）。(9078c87) join 再配線 — §7.3 は single-record から per-target DA-002 verdict を読む。DA-002 のみ join、DA-003 は記録のみ。PR #5 に push・再レビュー依頼済。
- [x] **PR #5 マージ完了**（Owner）。develop=2aa428e。feature へ merge 済（新 spec 取込、競合なし）。
- [x] 別紙B に §5 実装項目追記（M3/W4・M7/W5・M6/W6 マッピング、Phase 1/2）。
- [ ] **← 実装 Phase 1（現在ここ）**: 5.1 per-target verdict + DA-003 pin（M3/W4）→ 5.3 評価時 join + §11.3 scope（M6/W6）。in-process/cross-crate で検証、subprocess coverage 非依存。
  - [x] W4 step2 adapter per-target loop + version bump（a20ccec）
  - [x] W4 step3 core fold所有 + store per-target round-trip + classifier（0e8a7c9 impl / 2ef1356 test）。find_target_source を scan へ公開。malformed classifier `static_record_target_defect` は W6 verify が呼ぶ。
  - [ ] **← W6 評価時 join（現在ここ）**: verify が static_audit を検証時算出（保存 fold は使わない）。per-target 実効 DA-002（§8.5）× runtime target_execution（§11.2）。classifier で malformed 除外。§11.3 scope。+ 横断 finding（Evidence identity canonical 化）。
- [ ] 実装 Phase 2: 5.2 subprocess coverage 帰属（M7/W5）。高リスク、実現性検証先行。
- [ ] SPEC-DOGFOOD-M3.yaml sha256 再登録 → dogfood 再実行 → 残 W8 gate

### 実装対象クレート（PR #5）
- W4: adapter-rust/static_audit.rs（per-target verdict, DA-003 pin）, adapter-api/model（RuleObservationDraft）, vtest-audit（純静的 fold）, store/records.rs（record + E-SCAN-010）
- W6: vtest-verify（評価時 join, §11.2/§3.6/§8.5 再利用, §11.3 scope, basis 引用）
- W5: adapter-rust/runner.rs coverage（subprocess 帰属）

### Phase 1 実装順序（advisor 確定, DTO-first 依存チェーン）
1. **adapter-api**: `RuleObservationDraft` に `targets: Vec<RuleTargetVerdictDraft>{target,verdict,reason,location}` 追加。空 vec=非target-scoped。w1 acceptance の DTO surface test（`adapter_api_crate_exposes_every_neutral_draft_type`）更新。
2. **adapter-rust**: 全 target ループ + per-target verdict + DA-003 pin + rule-set version bump（1コミット）。
3. **vtest-audit + vtest-store**: 純静的 fold（**core 所有**、adapter は fold しない）, record write/read, malformed→E-SCAN-010（1コミット）。
4. **vtest-verify**: 評価時 join（1コミット）。
- **test-fix は impl と別コミット**（§442 と同様、spec 節を引用する test-only コミット）。
### 実装 design 決定（advisor）
- fold は adapter でなく core（§7.1: adapter は pre-fold を返す）。
- 解決不能 target（E-SCAN-004）は per-target UNKNOWN entry に「黙って」しない。§8.1/§6.1（bundle 拒否・MISSING/MISMATCH 写像）と整合。
- DA-003 pin 条件＝「本体に当該 target 呼出が現れない」を analyzer facts（当該 target の call 分類が無い）から導出。**DA-002-Ambiguous-but-visible（M3 の bare 呼出）は pin しない**。
- DA-001 の target_resolution: single-resolution のままか target 集合要かを**明示決定**（first() を惰性継承しない）。
- §8.5 per-target 選択（全有効 record 走査・FAIL 支配・非FAILは最新）: **矛盾 R1/R2 手書き test を実装前に書く**（決定論監査では作れない唯一の検出手段）。
### トラップ（advisor）
- rule-set version bump で M3 dogfood audit は STALE 化＝想定内、追わない（dogfood 再実行は Phase1+2 後）。
- 既存 acceptance（audit JSON shape, m3 `read_audit`, adapter DTO pin）は spec 追従で壊れる＝**test-only コミットで spec 節引用**して更新。
- store read/write plumbing と fixture 手書き（旧形式/矛盾/malformed record）は DTO 確定＋exemplar 後に subagent 委譲。DA-rule ループ/fold/verify join は main thread。

### step 2 詳細設計（adapter-rust static_audit.rs, 実装中）
- [x] step1 DTO（commit b4cb137）: RuleTargetVerdictDraft + RuleObservationDraft.targets。
- [x] DA-003 pin（commit e333969）: rule_da003 の `!called → PASS`（空虚）を `!called → UNKNOWN` に修正（Fable Blocker 1）。co-located unit test 更新済。**まだ per-target ではない・version bump もまだ**。
- [x] **per-target loop + version bump（commit a20ccec）**。adapter が全 test.targets を解決・各 target で DA-002/003 実行・FAIL支配 fold（DA-001 は PASS支配・単一 verdict）・per-target list を DTO へ・RULE_SET_VERSION "1"→"2"（両所）。19 unit pass, workspace green。
  - RuleResult は不変（4フィールド）。per-target は audit() が `BTreeMap<String, Vec<RuleTargetResult{target,verdict,reason,location}>>`（rule名→per-target）で管理。RuleTargetResult struct を追加。
  - audit(): `test.targets.first()` を廃し全 test.targets ループ。まず `resolved: Vec<(TargetRef, ResolvedSource)>` を collect（resolve_target で None は除外＝解決不能は core 委任）。各 (tref, rsrc) で `TargetResolution::new(&rsrc.item_path, locator, rsrc.path==test_path)` を作り、DA-002/DA-003 を実行→per-target verdict。
  - fold: DA-002/003 は FAIL支配（§962）。DA-001 は per-target 実行し PASS支配（any runtime→PASS）＝単一 verdict のまま（per-target list 無し）。
  - lifetime 注意: TargetResolution<'a> は item_path を借用→ResolvedSource を先に collect して存続させる。
  - DTO map（現 b4cb137 で targets:Vec::new()）: rule名で per-target map を引いて RuleTargetVerdictDraft へ写す。
  - RULE_SET_VERSION "1"→"2"（static_audit.rs:1459 と vtest-scan/lib.rs:457 STATIC_AUDIT_RULE_SET_VERSION 両方）。← per-target verdict が実際に出る時点で bump（version2=full shape）。
  - 既存 acceptance（m3 read_audit, adapter DTO pin, m5 等）は shape 変化で壊れる→**別 test-only コミット**で spec 節引用更新。
  - Python brace-match は match 式ネストで誤挿入するので使わない（手 Edit か慎重に）。
- RULE_SET_VERSION "1"→"2"（adapter-rust static_audit.rs:1459 **と** vtest-scan/src/lib.rs:457 STATIC_AUDIT_RULE_SET_VERSION、両方同時）。
- RuleResult に `targets: Vec<RuleTargetResult{target:TargetRef,verdict:AuditVerdict,reason,location}>` 追加（非target-scoped は空）。
- audit(): `test.targets.first()` を廃し **全 test.targets をループ**。各 target を resolve_target→TargetResolution。
  - DA-002/DA-003: target ごとに rule 実行→per-target verdict 収集。rule-level verdict＝**FAIL支配 fold**（§962: 1件FAIL→FAIL, FAILなくUNKNOWN有→UNKNOWN, 全PASS→PASS）。
  - **DA-003 pin**: 当該 target への呼出が Test 本体に現れない（analyzer facts で当該 target が Absent かつ same-file helper でも到達せず ＝ `called==false` かつ helper_boundary なし）→ verdict UNKNOWN（空虚 FAIL にしない）。DA-002-Ambiguous(可視 bare)は pin 対象外。
  - DA-001: 単一 verdict（per-target list 無し, §3.6 対象外）。ただし multi-target を正しく見るため各 target で classify し **PASS支配 fold**（any target で runtime→PASS、なければ UNKNOWN>FAIL）。
  - DA-004/005/006/W-DA-101: target 非依存、単一のまま。
  - 解決不能 target（resolve_target=None）: per-target UNKNOWN entry に黙ってしない。§8.1/§6.1 整合（core が MISSING/MISMATCH 側で扱う）→ adapter は解決済み target のみ per-target 出力、未解決は observation に含めず core 判定に委ねる（要 core 側確認）。
- 変換（1765 map）: DA-002/003 の RuleResult.targets を RuleTargetVerdictDraft へ写す。fold は adapter が DTO 用に埋めるが **core が純静的 fold を再導出・所有**（step3）。
- fmt/clippy/test gate。既存 audit shape test は test-only コミットで更新。

### step 3 確定設計（core fold 所有 + store per-target + malformed, advisor 承認）
一次情報で確定した事実:
- **canonical Locator 解決**: `find_target_source(scan, &TargetRef)`（現 vtest-cli:1584）が既存 resolver。Locator は完全一致、SrcId は `source.src_id` 一致で SourceTarget を引く。`SourceFunction = SourceTarget`（model:603）で `.target` は常に canonical Locator（materialize が Locator 強制, scan:265）。canonical = `source.target.normalized()`。→ **vtest-scan へ `pub fn` 移動**して audit/cli 共用（§907 単一解決経路）。
- **`TestEntity.targets` は宣言形を保持**（materialize_test:232 は書換えず, SrcId は SrcId のまま）。∴ per-target の `target` identity は宣言 ref を find_target_source で canonical へ解決して得る（`declared.normalized()` を使わない — SrcId で §6.1.1 違反になる）。
- **fold ownership**: core が per-target list から FAIL支配 fold を再導出し保存。adapter 自身の rule-level verdict と core fold が不一致なら **malformed adapter output として拒否**（黙って上書きしない, 前例 scan §728）。observation 全体 verdict も再集約（§7.1 L962）+ incomplete-clamp。
- **解決不能 target**: adapter は解決分のみ per-target 出力 + `complete=false`。core は **complete 時のみ per-target list を record へ添付**。incomplete 時は DA-002/003 を per-target 無し・rule verdict UNKNOWN で保存 → §7.3 L1019 で「per-target 無し record は無効」→ STALE → 非PASS（fail-closed）。部分 1:1 な malformed record を書かない（§3.6 L422, Evidence L1263 と対称）。
- **canonical dedup**: 二重宣言（Locator+SrcId が同一 Source Target, §922）は canonical で1件に潰れる。write 時に canonical で dedup（宣言順）、per-target set は「解決 canonical 集合」と 1:1（Evidence L465 と対称）。
- **malformed 検査（E-SCAN-010）**: 単一分類関数 `classify_static_record`（Valid/Malformed/Stale）を **core（vtest-audit）に実装**し W6 verify(§8.5) が呼ぶ。条件: target集合≠宣言canonical集合（欠落/重複/余剰）| per-target fold≠rule verdict。malformed は有効集合から除外し per-target FAIL も抽出しない（L422）。

実行分割: **main thread** = find_target_source 移動 / store struct+field 凍結 / write 側（canonical解決・fold・cross-check・re-aggregate）/ classifier / exemplar YAML + round-trip golden。**subagent 委譲** = 別紙C §18.2 malformed fixtures 手書き + 追加 acceptance 更新（struct 凍結・exemplar 後）。壊れる acceptance は spec 節引用の test-only 別コミット。

### W6 評価時 join 実装計画（advisor 承認, 別紙C §18.3.2 が受入マトリクス）
一次情報: runtime 側 `evidence: BTreeMap<test_id, EvidenceRecord>` は `read_evidence_records`（verify:1608）が test ごと最新1件のみ保持＝§11.2 選択が**データ構造で強制**（古い有効 Evidence へフォールバック不能）。`TargetExecutionObservation{target:String canonical, result:CheckValue, count:Option<u64>}`（model:791）。static 側 valid record の DA-002/003 per-target を §8.5 で実効選択。
- `evaluate_static_audit(layout,scan)` → `(root,layout,evidence,scan)` に変更。dispatcher(647) は既に4値保持。
- **手順（各 test）**: (1) declared_canonical = `test.targets` を find_target_source で canonical 集合へ。**解決不能 target が1件でもあれば先頭で非PASS+診断**（classifier を空集合で回さない）。(2) valid record 選択: **subject-currency 通過 → その後 classifier**（`static_record_target_defect`）。stale は classifier より先に落とす（stale≠malformed, 値もメッセージも別）。classifier-malformed は既存 parse-malformed バケツへ合流＝UNKNOWN 強制。(3) 実効選択: **FAIL 支配は全 valid record 横断**、非FAIL は**最新 valid record 1件**の当該 target verdict（古い PASS で UNKNOWN を上書きしない, L1009）。valid は全て classifier 通過＝target 集合同一 + store validate が全6 rule 強制 → lookup total。(4) DA-002 per-target 到達: effective PASS→到達 / effective UNKNOWN→runtime 証明成立時のみ到達 / effective FAIL→未達（救済不可）。runtime 証明 = **evidence_validity==PASS ∧ checked ∧ 当該 canonical の per-target result==PASS ∧ count>0**（literal）。(5) DA-003 per-target・DA-001/004/005/006 は §8.5 実効、DA-003 UNKNOWN は runtime 救済せず非PASS 寄与。(6) static_audit = 全宣言 target 到達充足 ∧ 全ルール PASS のとき PASS、FAIL 支配、それ以外 UNKNOWN。runtime 救済 basis に Evidence record ID を引用。
- **invariant**: L963「充足済み到達は算出時点で UNKNOWN を生じない」をコード comment に引用（PR#5 争点）。§11.3: 限定 scope（--items static_audit）でも Evidence 鮮度/target_execution を内部依存評価、scope 外 item の report value は NOT_CHECKED 保持（L109）。
- no-per-target→STALE は三層（version bump が CONFIG hash 変更で v1 自動 STALE / incomplete-closure v2 は target subject 欠落で binds_test_subjects が落とす / 明示 per-target 欠落チェックは手書き・破損用の薄い第三層）。
- **テスト先行**: R1(DA-002 FAIL)/R2(UNKNOWN) 矛盾 record を**手書き fixture**で（決定論監査で生成不能, L107）。matrix=別紙C §18.3.2 L101-112（UNKNOWN+cov PASS→PASS / per-target FAIL 救済不可 / NOT_CHECKED·STALE·Evidence 不在 救済不可 / DA-003 UNKNOWN 据置 / malformed 除外+UNKNOWN / legacy→STALE / 多 target A静的 B runtime / 構造のみ未達）。既存 `write_static_audit` helper(verify:1806) は legacy 形状 → `static_audit_uses_only_current_per_test_records_and_fail_wins` 等が**設計どおり壊れる** → helper に per-target 変種追加・期待値更新は §7.3 L1019/L990 引用の test-only コミット。
- **コミット**: join impl + co-located unit tests / fixture・期待値更新 test-only / Evidence canonical 化は独立。

### ★W6 前提として持ち越す横断 finding（Evidence identity 整合）
Evidence writer（vtest-exec:177）は `test.entity.targets[i].normalized()`（宣言形）で target_execution/hashes.targets の identity を書く。SrcId 宣言（M3 `@vtest.target SRC-*`, TEST-DUAL-SRC 等が実使用）では SrcId 文字列になり、W4 の静的 record（canonical Locator）と §7.3 join で不一致になる。spec §6.1.1/§921 は**両者 canonical 必須**。→ **W6 で join を配線する際、Evidence writer も find_target_source 経由で canonical へ揃える**（Evidence YAML が SrcId target で変わる＝該当 acceptance を spec 節引用で更新）。W4 単体テストには影響しない（join は W6）。

### 注記（Owner 判断待ち事項）
- origin/develop(036a166) は local develop(4357562, #3 マージ済) より3コミット遅延。PR #5 は origin/develop 基準の単一コミットに rebase 済み。develop の同期は Owner 管理。
- 旧中間 remote ブランチ `spec/runtime-target-reachability`（rebase 前・#3 混在）が origin に残存（force-push 権限拒否のため）。不要なら削除可。
- **重要な帰結**: DA-003 据置ゆえ、本 spec でも subprocess test は DA-003 UNKNOWN のまま → static_audit UNKNOWN → all-12-PASS 不到達。dogfood の最終 scope（unit のみ / subprocess を含めるか）は merge 後の別判断。
