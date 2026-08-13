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
- [~] subprocess coverage spike（背景実行中）
- [ ] 仕様変更 PR（develop→spec/*）
- [ ] （merge 後）別紙B 追加 → 必要 W へ戻り実装修正
- [ ] dogfood 再実行 → 残 W8 gate
