# W8 完全 dogfood — Pilot 報告と scope 決定依頼

対象: Option A（194 テスト完全自己管理、全12項目 PASS）の実装可能性を Pilot で検証した結果。

## 結論（先に）
- **per-test の全経路は実証済み**（Pilot 1）。unit テストは全12項目のうち execution/static/semantic 系すべてに genuine 到達可能。
- ただし **subprocess 駆動の acceptance テストは static_audit PASS が構造的に不可能**（Pilot 2）。これらを dogfood scope に含めたままでは W8 gate「all 12 items PASS」は**達成不能**。
- よって **scope の Owner 決定が必要**（下記）。

## Pilot 1: unit テスト経路の実証（M3 chain）
`TEST-DOGFOOD-M3-TARGET-RULES` で per-test 全工程を通し、生成直後に **10/12 項目 PASS**（vo_coverage と test_traceability を除く全て）:

| item | 結果 | 手段 |
|---|---|---|
| spec_coverage / vo_decomposition / test_existence | PASS | 構造(chain/scan) |
| static_audit | PASS | `audit static`（**修飾呼び出し必須**, finding A） |
| semantic_audit | PASS | `audit bundle test-semantic`→genuine 判定→`submit`（根拠 basis 付き） |
| impl_consistency | PASS | `audit bundle impl-consistency`→submit |
| test_execution / runtime_result | PASS | `run`（非fast） |
| target_execution | PASS | **measured llvm-cov**（Windows 動作確認, checked:true, classify_target_call を count1 実行） |
| evidence_validity | PASS | HEAD+execution_state hash 束縛（dirty でも tree 不変なら PASS） |
| vo_coverage | 未 | VO に複数テスト mapping 要（finding D） |
| test_traceability | 未 | 全テスト管理要（scale） |

→ semantic 提出は CLI で完結（MCP 不要）。subagent 委譲可能。

## Pilot 2: subprocess acceptance テストの構造的不能
static audit DA-002 は「宣言 target を in-process で直接呼ぶ」ことを要求。`Command::new(vtest)`/MCP stdio で駆動するテストは production シンボルを in-process 呼び出ししない → DA-002 FAIL/UNKNOWN → static_audit PASS 不能 → 全12項目 PASS 到達不能。

### 分類（統合テスト `crates/*/tests/*.rs`）
確定分（`vtest_` 参照ゼロ・完全 subprocess）:
- m8_acceptance.rs: **4 SUBPROCESS**（CLI）
- m9_acceptance.rs: **9 SUBPROCESS**（MCP stdio）

### grand-total（統合テスト tests/, 実テスト計104 ※m3 の M3_CASES 内14件は fixture ソースで除外）
| file | 総数 | IN_PROC | SUBPROC | MIXED | STRUCT |
|---|---|---|---|---|---|
| adapter-api/tests/w1_acceptance_binding.rs | 13 | **5** | 0 | 0 | 8 |
| cli/tests/adapter_acceptance.rs | 41 | 0 | 34 | 1 | 6 |
| cli/tests/m1_acceptance.rs | 5 | 0 | 5 | 0 | 0 |
| cli/tests/m2_acceptance.rs | 12 | 0 | 12 | 0 | 0 |
| cli/tests/m3_acceptance.rs | 1 | 0 | 0 | 1 | 0 |
| cli/tests/m4_acceptance.rs | 6 | 0 | 4 | 2 | 0 |
| cli/tests/m5_acceptance.rs | 4 | 0 | 1 | 3 | 0 |
| cli/tests/m6_acceptance.rs | 6 | 0 | 6 | 0 | 0 |
| cli/tests/m7_acceptance.rs | 3 | 0 | 3 | 0 | 0 |
| cli/tests/m8_acceptance.rs | 4 | 0 | 4 | 0 | 0 |
| cli/tests/m9_acceptance.rs | 9 | 0 | 9 | 0 | 0 |
| **合計** | **104** | **5** | **78** | **7** | **14** |

- **IN_PROCESS = わずか 5**（全て w1、しかも同一ファイルに STRUCTURAL 8 が混在）。
- **SUBPROCESS 78 / MIXED 7 / STRUCTURAL 14 = 99 テストは static_audit PASS 構造的に不能**:
  - SUBPROCESS: `Command::new(CARGO_BIN_EXE_vtest)` / MCP stdio で駆動、production シンボルを in-process 呼び出ししない → DA-002 不成立。
  - MIXED: in-process の `vtest_store::read_*` は assert 補助にすぎず、検証対象の挙動は subprocess 駆動 → 真の target は呼ばれない。
  - STRUCTURAL: crate 構造/compile 契約/依存グラフを assert、呼び出す target 関数が存在しない → DA-002 対象外。
- **全11統合ファイルが「static_audit PASS 不能なテストを1件以上含む」** → dir/file 単位 scan では健全に切り出せない（w1 すら 5 in-proc と 8 struct が同居）。
- ⇒ **健全に scope 可能な dogfood 対象は実質 `crates/*/src` の unit テストのみ。**

## finding 一覧
- **A 修飾呼び出し必須**: `mod tests` の unit テストは target を `super::target(...)` 等で修飾しないと DA-002 Ambiguous→UNKNOWN。M3 テストは移動時に `crate::` 修飾が脱落した回帰（`super::` で修正済）。→ ~89 unit テストは注釈＋**呼び出し修飾（本体編集）**。
- **B subprocess 不能**（＝エスカレーション事由）: 上記。scope 判断が必要。
- **C scope escape**: `config.scan.include` を `crates/*/src` 群に絞れば tests/ を dogfood 対象外にできる（dir 単位; 混在ファイル分離不可）。
- **D VO 複数テスト**: vo_coverage COMPLETE には VO claim を網羅する複数テスト mapping が必要。
- **E manifest churn（実地確認済）**: §456 manifest は `.git/.verify/target` 以外の全ツリーを含む（docs/.claude/.agents/__pycache__ も）。docs を1つ編集しただけで全証跡が STALE 化した。→ 証跡生成は最後に frozen tree で。§456 が AI 作業ディレクトリ（.claude/.agents）まで含める意図か要確認。

## 根本原因分析（Owner 診断への回答）
Owner 問い:「DI してない感じ？／仕様上 subprocess 並列実行を考慮していなかった感じ？」→ **両方 Yes、本質は後者（仕様の設計ギャップ）**。

### (1) テスト側: acceptance テストは black-box subprocess（DI なし）
78 SUBPROCESS + 7 MIXED は全て `Command::new(CARGO_BIN_EXE_vtest)` / MCP stdio で**ビルド済みバイナリを別プロセス起動**して駆動。in-process の testable core への DI をしていない。これは CLI 引数解析・exit code・envelope・MCP protocol 境界という **end-to-end 挙動を検証する意図的選択**（in-process 呼び出しでは到達不能な領域）だが、静的に可視な in-process target 呼び出しを持たない。

### (2) 仕様側: static audit は本質的に white-box / in-process 前提（設計ギャップ）
詳細設計が DA-002/003 の解析境界を明示:
- §953:「関数本体および**同一ファイル内**の呼出先 helper（1段）を探索」「**他ファイル・他クレートの関数呼出**があり間接呼出の可能性を排除できない → **UNKNOWN**」
- §960:「クロージャ内・マクロ展開内は UNKNOWN」

⇒ DA rules は **source 内 white-box 静的解析**。「テストが subprocess を起動して target を別プロセスで実行する」という形態は**仕様の監査モデルに概念として存在しない**。よって subprocess テストは構造的に UNKNOWN/FAIL にしかならない（fail-closed として正しい挙動だが、dogfood の「全 static_audit PASS」とは両立不能）。

### (3) 自己言及的ギャップ
**white-box 検証ツールの自身の acceptance テストが black-box** という自己言及構造。仕様は runtime coverage（target_execution, §249「Test単位でcoverage採取・全宣言target実行count対応付け」）を持ち、これは**原理的には subprocess 実行の target カバレッジを証明できる**が、dogfood gate は static_audit PASS を一律要求するため、プロセス境界を越えられない静的監査で頭打ちになる。

### 上流修正の方向（Owner 決定事項）
- **(甲) 現行仕様の white-box scope を受容 → dogfood を unit テストに限定**（＝下記 案1）。仕様変更不要。acceptance は cargo test で担保。最小リスク。
- **(乙) 仕様拡張: integration 監査意味論を新設** — `@vtest.kind integration-*` は static_audit を not-applicable とし、target カバレッジを **runtime target_execution（subprocess coverage 計測）で証明**する。原理的に正当で black-box カバレッジを保持するが、**canonical 詳細設計の改訂（§15 の対象、Owner 権限）＋ subprocess coverage 捕捉の実装**が要る。alpha2 refactoring の範囲外の新機能。
- **(丙) テスト側 DI 化** — acceptance を in-process core 呼び出しへ書換。end-to-end/CLI-MCP parity の black-box カバレッジを失い、大規模・高リスク。非推奨。

## Scope 決定依頼（Owner）
finding B により、以下いずれかの決定が必要:

- **案1（推奨）— dogfood を unit テストに scope**: `scan.include` を `crates/*/src` に絞り、統合テスト（tests/, subprocess 群）を dogfood 対象外とする。W8 gate「all **unit** tests managed」の文言に整合。統合テストは cargo test で担保継続。→ (A) を「全 unit テスト＝~89」で完遂。
- **案2 — unit + in-process 統合テストのみ管理**: subprocess テストだけ除外。ただし scan は dir 単位のため、混在ファイルは物理分割（テスト移動）が必要になり侵襲大。
- **案3 — 別解**: ご指示ください。

いずれの案でも finding A（呼び出し修飾）と finding E（frozen tree 生成）は前提として実施します。
