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

## ★実装 Phase 1 完了（2026-08-15）— 到達性モデル実装 + E2E 検証済み
コミット列（feature/adapter-separation-alpha2-implementation）:
- `a20ccec` W4 step2: adapter per-target loop + RULE_SET_VERSION "1"→"2"
- `0e8a7c9` W4 step3: core fold 所有 + store per-target round-trip + classifier / `2ef1356` M3 acceptance per-target E2E
- `e48f38c` refactor: fold+classifier を vtest-store へ移設（verify から到達可能に）
- `1f2f806` W6: verify 評価時 join（per-target 実効 DA-002 × runtime target_execution、DA-003 非救済、malformed 除外、§11.3 scope）
gate: 全コミットで fmt + clippy -D warnings + workspace test green（206 tests）。join は unit で positive 救済（UNKNOWN→PASS, valid Evidence + count>0）/ count0 非救済 / DA-003 非救済 / DA-002 FAIL 支配（recency 無視）/ malformed→UNKNOWN を全網羅。

**実 dogfood テストで E2E 検証済み**（`TEST-DOGFOOD-M3-TARGET-RULES`, `classify_target_call` を修飾呼出＋結果 assert）:
- `vtest audit static --test ...` → verdict PASS、DA-002/DA-003 とも per-target canonical Locator 付き PASS（静的到達証明、runtime 救済不要）。
- `vtest verify --items static_audit --test ...` → **static_audit = PASS**。旧 v1 record 8件は version bump で STALE、v2 record が有効。§11.3 で scope 外項目は NOT_CHECKED。

### 後回し（dogfood クリティカルパス外）
- **Evidence identity canonical 化**: M3 dogfood は Locator 宣言（`declared.normalized()==canonical`）なので runtime 救済は canonical 化なしで機能。SrcId 宣言テストの救済のみ dead（fail-closed で誤りではない）。exec writer(:171) + evidence_record_validity(:1794) の両側を同一コミットで find_target_source 経由 canonical に揃える改修は SrcId テストを dogfood に含める場合のみ必要。§6.1.1 準拠の latent 修正として保留。
- **Phase 2 subprocess coverage（W5）**: spike で 0% 実証。かつ §7.3 で subprocess は DA-003 UNKNOWN のまま → static_audit UNKNOWN → all-12-PASS 不到達（設計どおり）。∴ subprocess coverage 帰属は all-12 目標に寄与せず、dogfood scope から subprocess を除外すれば不要。

### 残：完全 dogfood（W8 の本体、要 scope 判断）
- all-194-all-12-PASS は subprocess/structural に対し**仕様上到達不能**（PR#5 設計の帰結、pivot で既知）。達成可能 scope = in-process/in-body-assert テスト（DA-002 静的PASS or cross-file+runtime救済、DA-003 PASS）。subprocess/structural は明示除外（文書化）。
- 達成可能 subset の実行: 対象選定 → 一括注釈（target 修飾込み, finding A）→ 真正 semantic/impl/vo-coverage bundle+submit（機械的 PASS 量産禁止）→ VO approve → measured run → full verify。大規模・多時間。
- SPEC-DOGFOOD-M3.yaml sha256 再登録（W-SCAN-104 既出＝詳細設計 doc 変更由来）。

## ★★top-down ontology 再構築（Owner 指示 + GPT-5.6-sol 批評反映, 2026-08-15）
**前提の転換**: 旧63 VO は test 実装から逆生成（層の逆転）→ scaffolding として保持するが正統性なし。正しい導出鎖を一本で作る:
```
Normative Specification → Requirement → Design decision/mechanism → VO → 既存 Test/Evidence 突合
```
- **正典性（doc 自身の明文, 基本仕様§0/詳細設計§0）**: 要件定義=最上流（何を保証するか）。基本仕様=外部保証（**詳細設計と矛盾時は基本仕様が正**）。詳細設計=本冊§1-11,16,17,19 + 別紙A§12-15 + 別紙C§18 が正規（通し節番号）。別紙B=非正規 process 文書。
- **REQ 層（登録済 26件, main thread 直読で導出）**: P-001..004 / NFR-001..008（doc 内 ID 流用）+ 章32 の10機能分解 + 章32 外の要求章4件（23/25/27/29）。ID 形式が ontology を決めないよう、無 ID 章も意味で REQ 化済み。粒度は doc 章単位＝粗い。個別 normative statement は VO 層で拾う。
- **VO 層（wf_ac560087-476 実行中）**: 15 fan-out（REQ 毎）が spec docs だけを読み（test コード参照禁止）、反証可能な設計レベル claim + verbatim 引用 + 複数 doc refs を返す。main thread が dedup（fail-closed 等の横断は VO 1件に複数 refs）・品質ゲート（節タイトル言い換え reject）・**REQ→mechanism→VO の明示的な鎖として合成**して登録。
- **63 VO ゼロベース検証**: 上流導出の VO 数が20でも120でもそれが正しい。旧 VO との一致を仮定しない。
- **inventory の使い所は後段**: 192管理テストの分類・突合データは「VO ↔ Test 突合」フェーズで生きる（例: resolver 非対称は『discovery/audit が同一 target に異なる resolution を返してはならない』という VO の違反 evidence として表現される — これが SpecTracer の本来の向き）。
- 旧 bottom-up 63 VO/7 REQ は 191 注釈が参照中のため削除しない（dangling covers 防止）。remapping フェーズで置換。

### ★★現在地（2026-08-15）: A-gap 上流再導出 + freeze v3 完了（154→160 VO）→ 次=新6 VO の adequacy 評価
- **A-gap 8候補の上流再導出 完了**（wf_a5c2b410-844, docs/reports/w8-agap-freeze-v3.md + docs/plans/dogfood-agap-rederivation.json）: YES 6 / NO 2 / AMBIGUOUS 0。**freeze v3 = v2 の154無変更 + 6 amendment**（VO-ADAPTER-13 wire-decode / VO-ADAPTER-14 config-v1 / VO-ADAPTER-15 discovery-scope / VO-REGISTRY-15 宣言キー error / VO-PARALLEL-10 Relation-ID 衝突 / VO-EXEC-14 最新Evidence時系順）→ docs/plans/dogfood-ontology-topdown-v3.json（**数値正典: 160 VO**）。全6件の規範引用を main thread が一次資料 grep で裏取り済み（全一致・脱落なし）。棄却2件（TEST-STORE-020/021 原子的公開）は spec 沈黙 — **書込み原子性の追補要否は Owner 判断の spec gap** として surface、テストは B-supporting のまま。
- **新6 VO の adequacy 評価 完了**（wf_a35fae93-ba3, docs/reports/w8-v3-adequacy.md + docs/plans/dogfood-v3-adequacy.json）: PARTIAL 5 / CONTRADICTED 候補 1（VO-REGISTRY-15）/ PROVEN 0。VO-EXEC-14 は実装無罪確定（鮮度判定は時系比較・辞書順 sort は表示専用）。VO-REGISTRY-15 は8点鎖検証（dossier: docs/plans/dogfood-vo-registry-15-dossier.md）で **NEEDS-SPEC-JUDGMENT**: 実装欠落は6ケース再現済みだが §4.2 L513（test-scoped）vs §5.4「adapter所有の宣言」（declaration-scoped）の二読 → Owner 論点は「L513 維持（src-id typo 未検出を受容）か §5.4 採用（scope 拡大+診断を early return 前へ移設）か」。副次: **E-SCAN-005 握り潰し（else-if 単一 sentinel slot）は scope 非依存で CONFIRMED**（新規確定欠陥 +1）。数値正典: 160 VO = PROVEN 14 / PARTIAL 72 / UNSUPPORTED 31 / CONFIRMED 39 / REFUTED 2 / NEEDS-SPEC-JUDGMENT 2。
- **covers 設計 完了・適用前で停止**（docs/reports/w8-covers-proposal.md + docs/plans/dogfood-covers-proposal.json）: rule =「covers = correspondence = PROVES|PARTIAL」。**freeze v4 後の現在値: 184 tests / 400 mappings / auxiliary 8**。
- **role 仕様設計 v3 承認（条件付き）+ B19+C2 再導出 + freeze v4 完了**（docs/plans/spec-draft-vtest-role.md, docs/reports/w8-b19-freeze-v4.md, docs/plans/dogfood-ontology-topdown-v4.json, docs/plans/dogfood-b19-rederivation.json）: Owner の3修正（anchor classification 構造化 / Auditability≠Contribution 明文化 / role×topology 直交）反映済み。B19+C2 の上流再導出 = contract YES 16 / NO 3 / AMBIGUOUS 2 → **採択11 VO（提案15→統合12→引用検証で1棄却）で 171 VO**。棄却 = VO-PARALLEL-11（仕様は「実用上排除」— 絶対形 claim は over-claim、fail-closed）。VO-REGISTRY-21 は cluster 4 既知欠陥の直接アンカー（誕生時矛盾候補）。cluster 4 fix_shape の「別コード化」は仕様変更扱いと判明。**auxiliary 残 8** = supporting 5 + anchor-none 1 + A 2（削除傾き）。
- **v4 adequacy 完了**（wf_cdd2a414-e24, docs/reports/w8-v4-adequacy.md + docs/plans/dogfood-v4-adequacy.json）: PARTIAL 8 / UNSUPPORTED 1（VO-ADAPTER-17 — ontology 維持・§5.1 L562 の規範文は grep 検証済み。TEST-AAPI-002 の mapping は verdict NO で除去→supporting）/ CONTRADICTED 候補 2 = VO-REGISTRY-21（cluster 4 結線を具体確認 + adapter 側 suffix 一致の §907 違反という副次新候補）と VO-STRUCTOP-15（file required 非対称 vs 別紙A「他は同一」— 検証 agent 走行中）。ADAPTER-16 の to_yaml に latent v1 branch（到達 caller 無し・免罪、unreachable pin 推奨）。E-SCAN-004 は3意味に overload — remap 前に分別要。
- **現在値: 183 tests / 399 mappings / auxiliary 9**。per-VO 全体: PROVEN 14 / PARTIAL 80 / UNSUPPORTED 32 / CONFIRMED 39 / REFUTED 2 / NSJ 2 / 候補2。
- **STRUCTOP-15 検証確定: CONFIRMED**（宣言成果物レベル。dossier: docs/plans/dogfood-vo-structop-15-dossier.json）。判別原則=「正当差分は全て宣言済み、file だけが沈黙の逸脱」。候補の強制箇所（operations.rs:920-927）は反証 — required フラグは create 経路で不活性（検証器順で遮蔽）、拒否は operations_support.rs:157-160 の別機構。VO claim の「targets のみ」も偽と判明し claim 補正済み（kind 接頭辞が第2の宣言済み差分）。**Owner 二択**: §14.3 に file 必須を宣言 or フラグ反転+targets 対応導出追加（反転だけでは不成立を実証済み）。
- **次**: remap 適用（role 仕様 PR merge 後）→ doctor → 旧63/7 retire。数値正典: **171 VO**、検証済み CONFIRMED は **40**（39 + STRUCTOP-15。ほか scope 非依存副次 = E-SCAN-005 握り潰し、REGISTRY-21 候補は cluster 4 fix に随伴）。

### ★★修正フェーズ（2026-08-16 開始・Owner 許可: role 仕様 PR + 19 root cause 修正）
- **role 仕様 PR #6 提出済み**（spec/test-role-declaration → develop、2 commits、5文書 — 要件定義 §7.1 含む。最重点レビュー = 最上位文書変更。E-SCAN-013/014/015 新設。merge 判断 = Owner）。
- **Fix wave 1 完了・merge 済み**（86b7add/596e0d1/81b9ff7）: 8欠陥 fixed（DA-006 UNKNOWN 分岐 / fold FAIL 優先 / E-SCAN-005 多重報告 / freshness 伝播 / combine_values 経由 / audit recency / approve_vo E-APPROVAL-001 / vo-coverage exclusions E-AUDIT-006）。各 worker が revert 反証で fix の実効性を確認、各 worktree gate 緑。**E-AUDIT-007 でなく 006 が正**（spec 裏取り済み — dossier 側が不正確。007 = spec-coverage 用で wave 3 実装範囲）。
- **bridge VO 5件**（VO-ARUST-012, VO-CLI-019/020/021/022 — 旧 registry へ worker が追加した新 test 用の一時橋渡し。claim は実装参照形で freeze 品質ではない）: **remap 時に新 test の covers を新 ontology の真対応へ書換え、bridge VO は旧63+5 として retire**。
- **side-findings（新規記録）**: ① 詳細設計 §7.2 DA-006 行「退避例: なし」が UNKNOWN 分岐実装と desync → spec-sync 要（Owner）② 隣接欠陥2件 = evaluate_static_audit の per-test malformed→Unknown（~L789、§3.6 は STALE/NOT_CHECKED）/ evaluate_vo_coverage の severity-max fold が audited_at 不読（VO-SEMAUDIT-05 と同型、approval-gating 分岐で複雑）→ wave 2 に追加 ③ flaky test（static_audit_orders_offsets… — 温度: fixture の ns suffix 衝突、Windows 並列時）→ 小修正候補。
- **ワーカー方針改訂適用**: 実装 = sonnet xhigh + obligation discovery/closure 足場（wave 1 で初適用・成功。落穂: rename hunk 未コミット1件は main 側で回収 e496c47）。worktree isolation の stale base 問題と対策は memory 記録済み。
- **Fix wave 2 完了・merge 済み**（merge 3本 → acf1aa1、統合 gate exit 0）: **cluster 4**（arity 規律 — classify_target_resolution 新設・find_target_source は 0/2+ で None・bundle 生成は全 target 未解決で拒否・impl_consistency へ absent→MISSING/ambiguous→MISMATCH の diagnostic 経路・adapter 側 suffix fallback 撤去 = §907 単一解決経路。E-SCAN-004 は仕様どおり単一コード維持）**cluster 3**（required_subject_keys + required_spec_closure の単一導出を bundle 生成と validity の両方が使用・validity = 集合一致 + per-subject currency・--vo の SPEC subject ゼロ解消・spec subject は SPEC record + source の両束縛）**cluster 7**（evidence_record_validity の registry dispatch・adapter-less は compat 経路→UNKNOWN・per-adapter execution state）+ 隣接2件（malformed→§3.6 の STALE 系・vo_coverage の recency fold）。全 fix に revert 反証。merge 時の裁定: cross-branch テスト ID 衝突2件を renumber（TEST-CLI-106/107）・衝突9件は「実質変更 vs 整形」を判別して解決。**副産物: 基本仕様 §7.5 L444 の multi-target bundle content 不完全（cluster 4 単独では残る）は cluster 3 の resolve_all_targets が merge で解消**。残 wart: adapter find_function の同一 exact path 重複時 first-match（core gate が上流で遮断・記録のみ）。
- **root cause 進捗: 19中 11 相当が修正済み**（wave1 = 7 singleton + E-SCAN-005 / wave2 = cluster 3・4・7 + REGISTRY-21 随伴 + 隣接2）。残 = cluster 1・2・5（wave 3）・cluster 6・8・9 + VO-EXIST-08・VO-INTAKE-04（wave 4。INTAKE-04 は設計選択の裁定込み）。
### ★★★compact 手前の現在地（2026-08-17）— 作業目的と in-flight

**大目的**: W8 dogfood が確定させた 19 root cause の修正完遂 + role 仕様（merge 済み）の実装 → SPEC sha256 再登録 → covers remap 適用 → 旧63+bridge5 VO/7 REQ retire。Owner 指示 =「role 仕様 PR（済）と 19 root cause 修正を先に、残り（remap 等）はその後」。

- **branch**: feature/adapter-separation-alpha2-implementation @ 65c4125（wave1+2 merge 済み・gate 緑）。
- **in-flight: Fix wave 3 走行中**（wf_5173fa00-262、3 worker・sonnet xhigh・worktree 隔離、base 65c4125）: fix/w3-spec-coverage（cluster 1 = spec-coverage kind end-to-end。縮退値 matrix 6状態を spec 引用付きで返させる）/ fix/w3-structural（cluster 2 = E-SCAN-012 emitter + vo_decomposition の subject-routed 化）/ fix/w3-vo-coverage（cluster 5 = REQ-anchored 化。wave2 の recency fold 保存 obligation 付き）。
- **完了通知が来たら**: ①branch 先端 = 申告 final_commit を突合 + worktree 残差確認（wave1 の教訓。既知残差: worker1 の scratch_extract*.txt / worker3 の .tmp-repro/ — final commit tree に混入していないこと）①' **各ブランチ diff の新規 TEST-\* ID を予約レンジと突合**（2回噛まれた故障モード — cluster1: TEST-CLI-110..119/TEST-VERIFY-040..049, cluster2: TEST-SCAN-020..029/TEST-CLI-120..129, cluster5: TEST-VERIFY-050..059/TEST-CLI-130..139）②順次 merge — A↔C の build_req_node 衝突は事前規則「A は REQ copy から spec_coverage 除去・C は REQ node へ vo_coverage 追加 → 両意図の和集合」③統合 gate ④**新構造 pass が repo 自身の旧 registry（63+bridge5）を初審査して赤が出る見込み — fail-closed の正しい可視化。repo-root scan を1回観測して plan doc に記録。弱める修正は禁止**（retire で消える debt）⑤worktree/branch 掃除 → wave 4 起動（cluster 6 = structured ops adapter routing〔/** */ 根治〕・cluster 8 = target_execution entry set〔runtime-rescue と述語共有〕・cluster 9 = Annex A renderer + VO-AGG-10 basis 構造化・VO-EXIST-08・VO-INTAKE-04〔設計選択の裁定込み — §1.3 L87 の kind 一般化と同件〕）。
- **wave 4 後**: role 実装（scanner 3キー・E-SCAN-013/014/015・declared/effective・適用項目集合・covers-0 監査・固定 Form）→ develop→feature merge（7コミット差）→ SPEC sha256 再登録 → remap 適用（docs/plans/dogfood-covers-proposal.json、183/399/9）→ doctor → retire。
- **workflow 運用**: worker prompt に base-check（現 tip hash 記名）+ テスト ID 事前レンジ + bridge VO 禁止 + 既知 flake（static_audit_orders_offsets…は --test-threads=1 で確認）を必ず含める。完了済み workflow agent へ SendMessage しない（transcript 再開でレースする）。resume は Workflow({scriptPath, resumeFromRunId:'wf_5173fa00-262'})。
- **permission**: .claude/settings.local.json に cargo/git/gh/python 等の allow + force-push deny を設定済み（2026-08-17）。

- **role 仕様 PR #6 MERGE 済み（2026-08-17）**: レビュー4輪（blocker 計7件 — draft 搬送 / declared-effective 分離 / cover-less semantic_audit / 適用項目集合 / §4.3 集約式 / §7.1 冒頭 / 量化 domain）を経て develop へ。**判断キュー①（auxiliary の扱い）は仕様決着** — 検証閉包 thesis の operational definition も読み替え済み（thesis doc 改訂 2026-08-17）。remap への残依存 = **role 実装**（scanner 3キー・E-SCAN-013/014/015・declared/effective entity・適用項目集合・covers-0 監査・固定 Form 2種）→ waves 完了後に実施 → SPEC sha256 再登録 → remap 適用。develop→feature の merge は7コミット差で軽微（waves 完了後にまとめて）。
- **Owner 判断キュー（自動パイプラインはここまで）**: ① auxiliary 23 の扱い（covers 適用のブロッカー）② VO-STATAUDIT-01 仕様二読 ③ VO-REGISTRY-15 仕様 scope 二読（§4.2 L513 vs §5.4）④ 19 root cause + E-SCAN-005 握り潰しの修正着手可否/優先順位（P-001）⑤ 書込み原子性の spec gap ⑥ black-box topology ⑦ Evidence Set 第一級。
- **検証パス完了**（c493432, dossiers=docs/plans/dogfood-contradiction-verification.json）: 42候補 → **CONFIRMED 39 / REFUTED 2（VO-EXEC-10, VO-PLAN-06）/ NEEDS-SPEC-JUDGMENT 1（VO-STATAUDIT-01）/ NOT-REPRO 0**。8点鎖+隔離再現付き。報告書（w8-contradiction-verification.md）に「39/42 確認率自体への懐疑注記＝サンプル再懐疑 or Owner dossier レビュー推奨」明記。
- **`/** */` 件（VO-REGISTRY-05）の root cause 特定済み**: discovery は syn attribute 経由（///と/** */が同じ #[doc] に脱糖＝形式非依存でタダで両対応, discovery.rs:93）、edit renderer は raw text で `starts_with("///")` のみ（operations.rs:523）。**AST vs raw text の表現レベル差から創発した意図なき非対称** + operations.rs は adapter 分離以前の M4 期コードで create 側だけ移設済み・edit renderer が移設漏れ。自然な修正形= StructuredTestAdapter に edit-rendering 契約を追加し renderer を adapter へ（着手は Owner 判断/P-001）。
- **caveat（検証 dossier）**: 検証 agent の少なくとも1つが隔離指示に反し repo root を汚染（.verify/config.yaml を m1 fixture 設定で上書き・VO-KNOWN/src/tests を root に作成）。**全復元済み**（git checkout + 削除、doctor 0 errors・W-SCAN-102=156 invariant 維持、.verify に audit/evidence 混入なし）。ただし該当 agent の repro は汚染文脈で走った可能性 → 推奨済みの「サンプル再懐疑パス」で repro 文脈の妥当性も併せて確認すること。
- **root-cause clustering 完了・裁定済み**（docs/plans/dogfood-rootcause-clusters.json + docs/reports/w8-rootcause-clusters.md）: **39 CONFIRMED = 10 clusters(30) + 9 singletons = 19 独立 root cause**。exactly-one 被覆を機械検証（完全一致）、load-bearing 主張4点を code spot-check（1点補正: cluster 8 の「entry list never consulted」は evaluate_target_execution スコープでのみ真、verify/lib.rs:988 の runtime-rescue が既存消費者 — 報告書の裁定が JSON 原文に優先）。二大テーマ: SPEC→REQ→VO intake 軸 end-to-end 未実装（9件）/ diagnostic→check-item routing table 不在（背景条件）。PLAN 数値不整合も決着: 候補8・CONFIRMED 7（過去報告2件の散文を訂正済み）。
- **sampling 再懐疑 完了**（wf_e9839da5-30f, docs/reports/w8-sampling-reskeptic.md + docs/plans/dogfood-sampling-reskeptic.json）: 19/19 UPHELD・OVERTURNED 0・**TAINTED 0 → dd7dd44 の汚染 caveat 退役**（構造的根拠: find_project_root は cwd 祖先walkのみで temp repro から repo root config に到達不能 + 全件 clean 環境で再現一致）。amendment 1件: VO-EXIST-09 を cluster 9 に再配属（cluster 10 の fix は必要でも十分でもない）、cluster 10 → singleton 降格。**確定構成: 9 clusters(29) + 10 singletons = 19**。39 CONFIRMED は検証済み確定。
- **残 Owner 判断**: VO-STATAUDIT-01（仕様二読）/ 修正着手可否と優先順位（P-001）/ auxiliary-covers 張力 / black-box topology / Evidence Set 第一級。**残パイプライン**: A-gap candidate 8 の上流再導出 → 必要なら freeze v3 → adequacy 再計算 → covers 設計 → 適用 → 旧63/7 retire。
- **検証閉包 thesis 正典化済み**（docs/reports/verification-closure-thesis.md, cce3c40 + memory）。auxiliary test（22件）と covers≥1 の張力は未決仕様判断。
- **残パイプライン**: clustering 裁定 → VO-STATAUDIT-01 の仕様判断（Owner）→ A-gap candidate 8 の上流再導出（Test はセンサーであって正典でない）→ 必要なら freeze v3 → adequacy 再計算 → covers 設計（auxiliary に偽 covers 禁止）→ 適用 → 旧63/7 retire（比較資料保存）。
- 数値正典: 206 scan可視 = 192 managed + 14 unmanaged(structural)。154 VO = PROVEN 14 / PARTIAL 67 / UNSUPPORTED 31 / CONFIRMED 39+α（4値は dogfood-evidence-adequacy.json の final_per_vo、検証後の再計算はまだ）。
- ワーカーモデル方針: 通常=明示 opus（+effort で調整）、Fable は最重要単発のみ（週間残量注意）。

### 現在地: adequacy フェーズ（shadow mapping 完了後, Owner 3点補正反映済み b944b60）
- shadow mapping 完了（7ba3f76, docs/plans/dogfood-shadow-mapping.json）: Test側 15 proves/147 partial/30 none、VO側 13 proven/88 partial/23 touched/30 uncovered。506 candidate mappings 中 PARTIAL 349、negative facet 欠落 153 / positive 欠落 154（単位=candidate mapping）。
- **covers 意味論確定**（基本仕様§6.2/§7.4）: covers=対応宣言（test_existence/traceability 駆動）、十分性= vo-coverage 監査。Evidence Set ⊨ VO の判定席は監査理由（第一級 record 無し=finding）。remap 安全。
- **adequacy workflow 起動**: (P1) 88 partial VO → required facets 正規化 → test 横断候補集約 → facet completeness → **composability gate**（欄埋め≠証明。同一契約への evidence set として構成可能か）→ SUFFICIENT/PARTIAL/CONTRADICTED/NONE。CONTRADICTED=実装が claim に反する実測（w8-dogfood-findings.md の resolver 非対称等を各 area で照合）、INSUFFICIENT と区別。(P2) proves_no_vo 30 test を A ontology-gap/B supporting/C regression/D impl-detail/E redundant/F obsolete に分類（B/C は存在してよい。Test exists ≠ must prove VO）。
- その後: Sufficient Evidence Set 確定 → uncovered/insufficient/contradicted 確定 → 新 covers 適用 → doctor → 旧63 retire（比較資料は保存: bottom-up が見落とした/束ねた/test-cluster に引かれた obligations 分類 = test-derived traceability の危険の自己実証）。

### Owner 確定パイプライン（destructive remap は最後）
```
118 VO 登録 → [ONTOLOGY FREEZE GATE]（今ここ） → 192 Test shadow mapping（covers 不変・非破壊）
→ evidence adequacy 分析（relevance ≠ proof を分離） → gap/contradiction/orphan 検出
→ 新 covers 確定 → 旧63 VO/7 REQ retire（bottom-up が何を見落としたか自体が成果）
```
- **母集団照合済み**: W-SCAN-102×120 = 新118（全 leaf・test 未 mapping＝正当）+ 旧2（VO-AAPI-006=唯一 member の注釈除去 / VO-CLI-007=member が structural skip 群で孤立）。invariant `W-SCAN-102(new)=118=registered_new_vo` 成立。
- **freeze gate（wf_081a6a94-9fe 実行中）**: 15 area fan-out の敵対的レビュー。reject 基準 A-I: A=REQ言い換え / B=機構説明 / C=判定不能語 / D=compound（独立命題の and 結合→分割） / E=実装現在形の仕様昇格（normative 根拠を doc 照合） / F=Test 存在前提 / G=implementation-specific（設計が normative に要求する場合は可） / H=**VO vs evidence-admissibility rule 境界**（SpecTracer は verifier ゆえ大半は正当な verifier-behavior VO。純粋に dogfood 側の判定規則のみ flag） / I=**merge 21件の truth-condition 同一性**（文言類似でなく同一状況で成立するか。異なる input domain/failure mode なら de-merge）。
- **shadow mapping の形式**（gate 通過後）: Test→candidate VO→{relevance, evidence(positive/negative/ambiguity case), verdict PROVES/PARTIAL/NO} を別 inventory として作成。双方向分類: VO→{sufficient/partial/no evidence}, Test→{proves/partially supports/proves no current VO}。resolver 非対称は「VO は uncovered かつ現実装に反証 evidence あり」と表現される。

### freeze gate 結果（wf_081a6a94-9fe, 118件全数判定・findings は dogfood-ontology-gate-findings.json）
KEEP 57 / REVISE 19 / SPLIT 41 / DROP 1。criteria: D(compound)44, E(実装現在形/refs不実)10, C(判定不能語)7, I(merge truth-condition)7, B5, A4, F1。実質誤りの検出例: VO-EXEC-10「never a current PASS」は §11.2 条件5（compat 一意確認で PASS 可）に反する / VO-REPORT-05 exit-code は操作コマンド成功=0 を落とす / VO-STRUCTOP-01 candidates は §14.2 で validator 毎（全 failure class でない）。
**orchestrator 裁定（X1-X6）**: X1=REPORT-03≡AGG-03 split（AGG 側へ統合・REQ-REPORTING 付与）。X2=capability-gap 写像4重複（ADAPTER-04/AGG-07/INTEG-06/REPORT-02b）→ ADAPTER-04 改訂版に一本化（op+verify の対は spec が一息で規定する完全分割＝non-compound）。X3=REPORT-02a→AGG-02 統合（P-002/NFR-005 維持）。X4=PLAN-01 part2 + PLAN-10(DROP)→INTAKE-05 split へ fold。X5=STRUCTOP-04 は absorbed 原文（PARALLEL-DEV#1）が edit-scoped 確認済→KEEP。X6=REGISTRY-02 は原文維持+§23 companion VO 復元。他は gate 提案どおり。組立=opus worker（機械転記・全118 origin 被覆検証込み）→ v2 JSON → 再登録。

## ★dogfood 実行フェーズ（Owner 指示 2026-08-15: 問題1=全テスト管理を ultracode で埋める）
Owner 指示: 「問題1だったもの（未注釈205件）を全部埋めてみて。サブエージェント/ultracode で」。仕様変更なし。black-box(問題2)は parked だが、注釈自体は可能（target を実シンボルにすれば test_traceability は通る。static_audit UNKNOWN は問題2として観測）。
段階設計（VO 一貫性のため main thread が ontology 所有・fan-out は読取り/適用のみ）:
1. **inventory workflow（read-only, 実行中 wf_82f1b92f-93a）**: 24ファイルを fan-out、各テストの {test_fn, target(locator or ""), intent, topology(in-process/subprocess/structural)} を分類 JSON で返す。black-box/white-box 定量も兼ねる。
2. **ontology 生成（main thread）**: inventory から SPEC/REQ/VO を設計。record 作成手段=CLI `vtest spec add --id --path --title` / `vtest req add --id --summary --spec --sections` / `vtest vo add --id --claim --req --spec --sections`。既存=SPEC-DOGFOOD-M3(詳細設計)/REQ-DOGFOOD-M3/VO-DOGFOOD-M3。方針: SPEC=spec docs、REQ=crate/subsystem 単位、VO=test-cluster or per-test。dangling 回避のため SPEC→REQ→VO→TEST を全段繋ぐ。
3. **annotation workflow（fan-out per file, source 編集）**: 各テスト fn の上に doc-comment `/// @vtest.id <ID>` `/// @vtest.covers <VO>` `/// @vtest.target <locator>` `/// @vtest.intent <text>` を追記（別ファイルなので worktree 不要・並列安全）。map は main thread 供給、sub-agent は適用のみ（発明禁止）。
4. **verify**: test_traceability を確認。black-box の static_audit UNKNOWN 等の問題を観測・記録。
注意: 注釈は construct hash を変えるので、注釈を全コミット後に audit/evidence 再生成（finding E, frozen tree）。
自己チェック: 断定は advisor/sonnet に当ててから出す（[[check-overclaims-with-second-model]]）。

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
