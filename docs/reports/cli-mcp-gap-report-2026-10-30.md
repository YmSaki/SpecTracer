# CLI / MCP 契約・実装差分（2026-10-30）

## 調査の基準と読み方

- 対象: Issue [#84](https://github.com/YmSaki/SpecTracer/issues/84)。develop `74878c3fc2c25e04461d6b54e61b06e4fb9c265c` の正本、CLI parser/dispatch、MCP tools/list/dispatch、共有操作層、verify出力。
- 目的: 未実装の入口と、入口はあるが不足する契約を、着手可能なIssueに対応付ける。
- 正本: [`docs/canonical/specification.json`](../canonical/specification.json)。ROOT → request → require → spec → detailed_spec → basic_design → design の順で解釈する。旧文書名・節番号は正本のsource情報としてのみ使い、archiveや生成exportを規範にしない。
- 固定確認軸: 公開操作の網羅、入口から共有処理までの到達性、gate/scope/pendingの入力・評価・出力、修正Issueへの対応、変更範囲と検証記録。
- 除外: 全操作の全引数・全失敗経路の適合性保証、各adapter解析の正しさ、未実装操作の設計・実装、他ワーカーの未マージ変更。

表の「有」はソース上の入口と処理経路が存在する意味であり、全契約の実装完了や受入完了を意味しない。「無」は公開入口がない。「部分」は具体的な未達を確認したもの。各行の #84 はこの棚卸しの証跡であり、実装済み操作を改めて実装するIssueではない。

## 結果

- 基本設計 BD-070〜084 のCLI操作を末端サブコマンドで数えると27件。11件に入口があり、16件は入口がない。11件のうちverifyはgate/scope/pendingの契約に未達がある。
- DS-1193〜1215 のMCP操作は23件。広告・dispatchがあるものは9件、ないものは14件。9件のうちverifyに同じ未達がある。
- MCPの9件はライブラリ内の公開経路である。実行ファイルからの `vtest mcp` 起動は未実装（#83）なので、9件を通常のCLIから利用可能という意味で数えない。
- `--scope` という引数は正本のverify契約にない。検査軸は `--items`、エンティティ軸は `--doc` / `--vo` / `--test`。引数の不在と、出力scopeの不備を分ける。
- 新規追跡: #107 承認ロール評価、#108 scopeの位置・形式、#109 form_get、#110 未定義gateの候補。pendingは既存#89へ対応付ける。

## 実装証拠の索引

リンク先はこの報告書と同じtreeのファイルで読む。将来更新するときは上記基準SHAとのdiffも確認する。

| 記号 | 観測した経路・ファイル |
|---|---|
| C | [`vtest-cli/src/lib.rs`](../../crates/vtest-cli/src/lib.rs) の `Command`、`DocCommand`、`ApprovalCommand`、`run` |
| I | [`ops/init.rs`](../../crates/vtest-cli/src/ops/init.rs) の `execute` |
| S | [`ops/scan.rs`](../../crates/vtest-cli/src/ops/scan.rs)、[`ops/doctor.rs`](../../crates/vtest-cli/src/ops/doctor.rs) の `execute` |
| D | [`ops/doc.rs`](../../crates/vtest-cli/src/ops/doc.rs) の `add/list/show`、[`doc_acceptance.rs`](../../crates/vtest-cli/tests/doc_acceptance.rs) |
| A | [`ops/approval.rs`](../../crates/vtest-cli/src/ops/approval.rs) の `create/withdraw/show`、[`approval_acceptance.rs`](../../crates/vtest-cli/tests/approval_acceptance.rs) |
| R | [`ops/run.rs`](../../crates/vtest-cli/src/ops/run.rs) の `resolve_runnables/run`、[`run_acceptance.rs`](../../crates/vtest-cli/tests/run_acceptance.rs) |
| V | [`ops/verify.rs`](../../crates/vtest-cli/src/ops/verify.rs) の `execute/resolve_gate/evaluate_gate/VerifyData`、[`vtest-verify/src/lib.rs`](../../crates/vtest-verify/src/lib.rs) の `verify_project/ScopeReport/RequestedScope` |
| M | [`vtest-mcp/src/lib.rs`](../../crates/vtest-mcp/src/lib.rs) の `TOOL_NAMES/tools_list_result/tool_input_schema/dispatch_tool/verify_tool/success_envelope/tool_result` |
| E | [`vtest-model/src/protocol.rs`](../../crates/vtest-model/src/protocol.rs) の `JsonEnvelope`、Cの `run_verify/render_verify_text` |
| T | [`verify_acceptance.rs`](../../crates/vtest-cli/tests/verify_acceptance.rs)、[`acceptance_18_3_9.rs`](../../crates/vtest-cli/tests/acceptance_18_3_9.rs)、Mの `mcp_*_tool_matches_*` tests |

Cに存在しないコマンドはClapの入口がないため、下位に類似処理が存在しても公開CLIを実装済みとはしない。Mに存在しないツールは `tools_call_result` の登録確認で拒否され、公開dispatchへ到達しない。

## CLI 全操作

| 操作（`vtest`以下） | 正本条項 | 実装・証拠 | 対応Issue |
|---|---|---|---|
| `init` | SPEC-374、BD-070、BD-153/156 | 有: C→I | #84 |
| `scan` | SPEC-375、BD-071 | 有: C→S | #84 |
| `doctor` | SPEC-376/377、BD-083 | 有: C→S。scanと同じ診断源を使用。全表示の同一性は今回未監査 | #84 |
| `doc add`（`--update`を含む） | BD-072、DES-595、DS-1003/1013 | 有: C→D | #84 |
| `doc list` | BD-072、DS-1015/1016 | 有: C→D、tree/roots引数あり | #84 |
| `doc show` | BD-072、DS-1017/1682 | 有: C→D | #84 |
| `vo add` | BD-073、DS-1021〜1040 | 無: CにVO入口なし | #86 |
| `vo edit` | BD-073、DS-1027〜1040/1048 | 無: C | #86 |
| `vo list` | BD-073、DS-1026/1047 | 無: C | #86 |
| `vo show` | BD-073、DS-1030/1047 | 無: C | #86 |
| `vo expand` | BD-073、DS-1041〜1044 | 無: C | #86 |
| `vo approve` | BD-074/305、DS-1045/1046 | 無: C。approval createへの別名が未提供 | #86 |
| `approval create` | BD-075/305、DS-1049〜1064 | 有: C→A | #84 |
| `approval withdraw` | BD-075/307 | 有: C→A | #84 |
| `approval show` | BD-075/308、DS-1057 | 有: C→A | #84 |
| `test create` | BD-077、SPEC-385、DS-1065/1066 | 無: CにTest入口なし | #85（#73待ち） |
| `test edit` | BD-077、SPEC-386、DS-1067〜1069 | 無: C | #85（#73待ち） |
| `test show` | BD-077、DS-1070/1071 | 無: C | #70（#67/#68待ち） |
| `test list` | BD-077 | 無: C | #70 |
| `test query` | BD-077、DS-1072 | 無: C | #70 |
| `audit static` | BD-078、SPEC-387、DS-1073〜1076 | 無: C。verify内部に解析があっても要求時の公開操作とは別 | #87 |
| `audit bundle` | BD-079/213/301、DS-1077〜1088 | 無: C | #88 |
| `audit submit` | BD-079、DES-429、DS-1089〜1100 | 無: C | #88 |
| `run` | BD-080、SPEC-397、DS-1101〜1103 | 有: C→R、test/vo/all/fast引数あり | #84 |
| `verify` | BD-081、SPEC-398、DS-1104〜1132 | 部分: C→V→E。次節参照 | #84、#107、#108、#110、#89 |
| `report` | BD-082、SPEC-401、DS-1133〜1148 | 無: C | #89 |
| `mcp` | BD-084、SPEC-403 | 無: C。Mのserve関数はある | #83 |

旧 `spec` / `req` はDS-1002による廃止なので、復活対象の不足に数えない。CLIにMCP専用の `form_get` と対になる新コマンドを推測して追加しない。

## MCP 全ツール

「有」はMの広告、入力schema、dispatch先を照合したもの。外部stdio起動は共通して#83の未達がある。

| ツール | 正本条項 | 実装・証拠 | 対応Issue |
|---|---|---|---|
| `scan` | DS-1193 | 有: M→S | #84、#83 |
| `doc_list` | DS-1194 | 有: M→D | #84、#83 |
| `doc_get` | DS-1194 | 有: M→D | #84、#83 |
| `doc_upsert` | DS-1195、DS-1218〜1220 | 有: M→D | #84、#83 |
| `approval_create` | DS-1196 | 有: M→A | #84、#83 |
| `approval_withdraw` | DS-1197 | 有: M→A | #84、#83 |
| `approval_get` | DS-1198 | 有: M→A | #84、#83 |
| `vo_list` | DS-1199 | 無: M | #86 |
| `vo_get` | DS-1199 | 無: M | #86 |
| `vo_upsert` | DS-1200 | 無: M | #86 |
| `vo_expand` | DS-1201 | 無: M | #86 |
| `vo_approve` | DS-1202、BD-305 | 無: M。approval_createと同じ意味の別名が未提供 | #86 |
| `test_query` | DS-1203 | 無: M | #70 |
| `test_get` | DS-1204 | 無: M | #70 |
| `form_get` | DS-1205 | 無: M。kind→owner adapter→Form Schema取得の公開経路なし | #109 |
| `test_create` | DS-1206 | 無: M | #85 |
| `test_edit` | DS-1207 | 無: M | #85 |
| `audit_static` | DS-1208/1209 | 無: M | #87 |
| `audit_bundle` | DS-1210 | 無: M | #88 |
| `audit_submit` | DS-1211/1212 | 無: M | #88 |
| `run_tests` | DS-1213 | 有: M→R | #84、#83 |
| `verify` | DS-1214/1216/1217 | 部分: M→V→M。次節参照 | #84、#107、#108、#110、#89 |
| `report` | DS-1215/1216/1217 | 無: M | #89 |

## gate / scope / pending の処理と残差

CLIの `run_verify` とMCPの `verify_tool` は同じ `ops::verify::execute` へ渡す。そこで引数を解釈し、config・gate名を解決し、scan結果を `verify_project` へ渡す。集約値からgate評価を作り、各入口がJSON/textへ整形する。この共有によって両入口が同じ結果を返しても、共有実装が正本に合うことまでは保証されない。

| 観点 | 正本 | 観測した処理・不足 | 対応Issue |
|---|---|---|---|
| 検査scope入力 | DS-1104〜1112 | `--items` / MCP `items[]` → `parse_items`。省略は固定4検査、未知名を拒否。V/T | #84 |
| エンティティscope入力 | DS-1104、DS-952/953 | `--doc/--vo/--test` → `entity_scope`。複数指定を拒否し、verify_projectへ伝達。V/T | #84 |
| 限定scopeの表示 | DS-1113 | Cのtext rendererが要求scopeと範囲外未検証を表示。Vは省略検査を保持する。全境界値は今回未実行 | #84 |
| JSON scopeの位置 | DS-943/947/957/1114/1214 | `VerifyData.scope` が `JsonEnvelope.data` 内へ入り、CLI/MCPとも最上位scopeを欠く。E/M | #108 |
| JSON entitiesの形 | DS-952/953 | `RequestedScope.entities: Vec<String>`。必要な `{kind, id}` 配列にならず種別を落とす。V | #108 |
| gate名・検証条件 | DS-1115/1116、DS-1152/1155〜1158 | `resolve_gate`は完全一致、`evaluate_gate`は要求状態との完全一致。空の承認要求なら満足可能。V/T | #84 |
| 未定義gateの候補 | DS-1154/1216 | E-CONFIG-002は返すがmessageに定義済み一覧なし、MCPにもcandidatesなし。V→Mのエラー経路 | #110 |
| gate承認条件 | DS-1160〜1166、DS-1171/1172 | 各ロールを `satisfied: false` 固定、`missing_subjects: []` 固定。承認ストアを参照せず、非空の要求は必ず不充足。V | #107 |
| gate結果と検証状態 | DS-1174〜1178 | exit/okはgate満否、data.state/resultは集約から作る。textにも状態とgateを別表示する。承認評価は上行の不足あり。C/V/M/T | #84、#107 |
| 判断待ちpending | BD-230、DS-1097/1147/1180〜1189/1214/1215 | `VerifyData`にpendingなし、CLI/MCPの整形にも追加なし。report自体も入口なし。V/E/M | #89（verifyとreportの両方を含めて対応） |

具体的には、currentな承認が揃ったVOを指定しても、要求ロールがあるgateはfalseになる（#107）。また限定verifyを受け取るクライアントが規定の最上位scopeを読むと欠落し、data内を読んでもentitiesの種別がない（#108）。これらは入口の存在確認だけでは分からない。

pendingはUNKNOWN、未登録、未解決、未確定、承認待ちと判断競合を横断的に提示する契約。#89の「pendingセクション」をreportだけで終えず、verifyのJSONと同名MCPにも実装する必要がある。判断由来の項目の `check: null` は集約へ寄与させない（DS-1183/1184）。

## 要件に追加された出力機能（上記件数の外）

| 対象 | 根拠・現状 | 対応Issue |
|---|---|---|
| テスト仕様書出力 `vtest export` / MCP `export` | REQ-339〜342は存在するが、当該入口の詳細契約は#66/#67で導出する段階。C/Mに入口なし。未確定の引数を既成契約として数えない | #103、#66、#67、#68、#69、#71 |
| 正本のMarkdown生成 `vtest doc export` | 仕様化作業#91と実装#92で追跡。Test宣言の仕様書出力とは別用途 | #91、#92 |

## 検証記録・引継ぎ

今回のソース照合は上記基準SHAに固定した。全27 CLI操作と23 MCPツールに条項・存在判定・Issueを付け、未知の入口を推測で追加していない。追加の4件はM4へ登録済み。

実行環境では `cargo` がPATHに無く、`/root/.cargo/bin` も存在しなかったため、Rustの共通ゲートとCLI/MCPの動的観測は現時点でNOT_EXECUTED。既存テストの名前・内容は処理確認の補助であり、今回通した証拠とはしない。別checkoutに残る既存バイナリはビルド元を証明できないため、本SHAの実行証拠に使用していない。

検証結果を更新するときは実際に実行したコマンドと結果をPRへ追記する。既存のgate/scope/pendingの不足はこの文書追加が生んだ退行ではなく、後続実装Issueで追跡する。文書追加の差分はdocs/reports内に限定する。

並列作業#25は診断分類、#35はLocator、#74はExecution Stateの境界を変更する。取り込み後に本表を更新する場合、旧SHAの観測と新SHAの結果を混ぜず、基準と該当行を一緒に更新する。
