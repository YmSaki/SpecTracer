# W8 dogfood 実施findings（仕様変更なし・問題surface目的）

Owner 指示: 仕様変更は現段階で行わず dogfooding し、「どのような問題が出るか」を実データで観測する。
以下はすべて `feature/adapter-separation-alpha2-implementation`（到達性モデル実装後）で実測。

## 0. 前提: 到達性モデル実装は動作する（spec-fix は成功）

実 dogfood テスト `TEST-DOGFOOD-M3-TARGET-RULES`（in-process, `super::classify_target_call` を修飾呼出＋結果 assert）:

| 項目 | 値 | 備考 |
|---|---|---|
| static_audit | **PASS** | DA-002/DA-003 とも per-target で canonical Locator 付き PASS（静的到達証明）。旧 v1 record は version bump で自動 STALE。 |
| test_execution / runtime_result / target_execution / evidence_validity | **PASS** | measured llvm-cov run 後。target_execution は classify_target_call の実行を count>0 で帰属。 |
| spec_coverage / vo_decomposition / test_existence | PASS | 構造。 |
| vo_coverage / semantic_audit / impl_consistency | NOT_CHECKED | 真正監査（bundle→submit）未実施。実施すれば PASS 可能＝**11/12 到達可能**。 |
| test_traceability | **MISSING** | ← 唯一の構造的な壁（下記 問題1）。 |

→ **in-process テストは 8/12 PASS（監査追加で 11/12）。12/12 を阻むのは test_traceability のみ。**

## 問題1: test_traceability は project 全体 item で、incremental dogfood 不能

- test_traceability は `scan.diagnostics` の W-SCAN-101（未注釈テスト）を集約する **project 全体**の判定。単一テストを完全管理しても、他に未注釈テストが1件でもあれば MISSING。
- 実測: **未注釈テスト 205件 / 24ファイル**（adapter_acceptance 41, scan/lib 17, verify/lib 17, store/records 14, w1 13, m2 12, static_audit 11, ...）。
- scope 絞り（finding C, `scan.include` を dir/file 限定）でも、**scan 対象単位の中に未注釈テストが混在**すれば同じ壁。全テストが管理済みのファイル/dir を作らない限り PASS しない。
- **帰結**: 「1本ずつ緑化して積み上げる」incremental dogfood が構造的に不能。test_traceability は all-or-nothing。
- **副次観測（動く的）**: 本セッションで W6 実装の unit テストを追加した結果、未注釈テストが 194→205 に増えた。テストを増やすほど dogfood 対象が増える。all-194（今や all-205）は移動する目標。

## 問題2: subprocess/black-box テストに white-box 規則を当てている（Owner 仮説の裏付け）

- DA-002（対象呼出）/ DA-003（結果検証）は **white-box 規則**: 「テスト本体が宣言 target（内部 Rust シンボル）を呼び、その戻り値を assert したか」を静的に見る。
- しかし acceptance テストの多く（`Command::new(vtest)` / MCP stdio 駆動、10ファイル）は **black-box 契約テスト**: 別プロセスの vtest を起動し、exit code / stdout / 生成ファイルという**境界可観測な出力**を assert する。target 関数は子プロセス内で動き、その戻り値はテスト本体に存在しない。
- ∴ subprocess テストが宣言できる「内部 Rust シンボル target」は、テストが本体で呼ぶものではない → DA-002/DA-003 とも UNKNOWN（別紙C §18.3.2 L103）。DA-002 は runtime で救済可能だが DA-003 は据置ゆえ UNKNOWN のまま → **static_audit = UNKNOWN → all-12-PASS 不到達**。
- **これは DA-003 の厳しさの問題ではなく、分解のミスマッチ**（Owner 仮説どおり）: black-box 契約テストの「検証対象」は内部シンボルではなく**契約（CLI/MCP インターフェース）**であり、その検証は境界 assert（= runtime_result の PASS）が担う。white-box target（DA-002/003）を強制するのが category error。
- 現モデルには「契約を対象とする black-box テスト」の第一級表現が無い（target は必ず Rust シンボル）。integration という test kind はあるが、kind と topology（in-process / subprocess）は別概念であり、kind だけでは black-box/white-box を区別しない（pivot で「integration→N/A」を却下した理由と整合）。

## 問題3: 混在ファイルは分解不能

- `crates/vtest-scan/src/lib.rs`(17) や `crates/vtest-verify/src/lib.rs`(17) は in-process unit テストの塊だが、同一クレートの `tests/*_acceptance.rs` は subprocess。**同一ファイル内**でも in-process 単体テストと契約テストが混じるケースがあり（例: adapter が in-process、CLI が subprocess）、ファイル単位の scope 分離では white-box/black-box を切り分けられない。

## 示唆（判断は Owner。ここでは仕様変更しない）

実データは Owner 仮説「分解が間違っている / black-box と white-box を混ぜている」を支持する。到達性モデル（DA-002 runtime 救済）は in-process/cross-file-in-body-assert には正しく効くが、**black-box 契約テストは監査モデルの対象種別として未分化**である。考えられる方向（いずれも要仕様判断・今は未実施）:

1. **達成可能 subset のみ dogfood**: in-process/in-body-assert テストを管理し all-12-PASS。black-box 契約テストは監査対象外として明示除外。ただし test_traceability の all-or-nothing により、除外テストを scan から外す（scope 設計）か、test_traceability の意味を「管理対象と宣言したテスト集合の網羅」に再定義する必要。
2. **black-box 契約テストの第一級化**: 契約（CLI コマンド / MCP method）を target とする test topology を導入し、その検証を境界 assert（runtime_result）で行う。DA-002/DA-003（white-box）は N/A とし、契約網羅（呼出項目一覧の一致等）を別 rule で見る。← Owner の「呼び出せるか・項目一覧が一致するか」に対応。
3. 現状維持で subprocess は恒久的に 11/12 未満（UNKNOWN）を受容。

## 再現コマンド

```
vtest audit static --test TEST-DOGFOOD-M3-TARGET-RULES        # → per-target PASS
vtest run --test TEST-DOGFOOD-M3-TARGET-RULES                 # measured
vtest verify --test TEST-DOGFOOD-M3-TARGET-RULES              # → 8/12 PASS, test_traceability MISSING
vtest verify --items test_traceability                        # → 205 未注釈 / 24 ファイル
```
