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

## 観測1（問題ではない）: test_traceability は project 全体を要求＝設計どおり

**Owner 判定: これは問題ではなく、意図した挙動**。test_traceability は「全テストが追跡下にあるか」を検証する項目なので、未注釈テストが1件でもあれば MISSING になるのが正しい。単に dogfood が未完なだけ。

- 実測: 現時点で**未注釈テスト 205件 / 24ファイル**（adapter_acceptance 41, scan/lib 17, verify/lib 17, store/records 14, w1 13, m2 12, static_audit 11, ...）。dogfood 完了 = これらを全て @vtest 管理下に置くこと。
- 帰結（作業計画上の含意, defect ではない）: dogfood は「全テスト管理」が前提のため、達成可能 subset だけ緑化しても test_traceability は PASS しない。完了には未注釈 205件すべての管理が要る。テストを増やせば対象も増える（本セッションで 194→205）。これは目標が動くのではなく、dogfood の定義（全テスト）どおり。

## 問題2: subprocess/black-box テストに white-box 規則を当てている（Owner 仮説の裏付け）

- DA-002（対象呼出）/ DA-003（結果検証）は **white-box 規則**: 「テスト本体が宣言 target（内部 Rust シンボル）を呼び、その戻り値を assert したか」を静的に見る。
- しかし acceptance テストの多く（`Command::new(vtest)` / MCP stdio 駆動、10ファイル）は **black-box 契約テスト**: 別プロセスの vtest を起動し、exit code / stdout / 生成ファイルという**境界可観測な出力**を assert する。target 関数は子プロセス内で動き、その戻り値はテスト本体に存在しない。
- ∴ subprocess テストが宣言できる「内部 Rust シンボル target」は、テストが本体で呼ぶものではない → DA-002/DA-003 とも UNKNOWN（別紙C §18.3.2 L103）。DA-002 は runtime で救済可能だが DA-003 は据置ゆえ UNKNOWN のまま → **static_audit = UNKNOWN → all-12-PASS 不到達**。
- **これは DA-003 の厳しさの問題ではなく、分解のミスマッチ**（Owner 仮説どおり）: black-box 契約テストの「検証対象」は内部シンボルではなく**契約（CLI/MCP インターフェース）**であり、その検証は境界 assert（= runtime_result の PASS）が担う。white-box target（DA-002/003）を強制するのが category error。
- 現モデルには「契約を対象とする black-box テスト」の第一級表現が無い（target は必ず Rust シンボル）。integration という test kind はあるが、kind と topology（in-process / subprocess）は別概念であり、kind だけでは black-box/white-box を区別しない（pivot で「integration→N/A」を却下した理由と整合）。

## 問題3: 混在ファイルは分解不能

- `crates/vtest-scan/src/lib.rs`(17) や `crates/vtest-verify/src/lib.rs`(17) は in-process unit テストの塊だが、同一クレートの `tests/*_acceptance.rs` は subprocess。**同一ファイル内**でも in-process 単体テストと契約テストが混じるケースがあり（例: adapter が in-process、CLI が subprocess）、ファイル単位の scope 分離では white-box/black-box を切り分けられない。

## 唯一の未決事項（問題2）: black-box テストの監査モデル

Owner 判定: **完全未定**。仕様事項か運用ルールかも含めて未定。ここでは仕様変更も設計決定もしない。以下は判断のための材料のみ。

- 到達性モデル（DA-002 runtime 救済）は in-process / cross-file-in-body-assert には正しく効く。
- **black-box 契約テストは監査モデルの対象種別として未分化**（target は必ず Rust シンボル＝white-box 前提）。Owner の「呼び出せるか・呼び出し項目一覧が一致するか」は契約検証であり、現 white-box 規則（DA-002/003）では表現できない。
- 「仕様 or 運用」の切り分け材料: 現モデルで black-box テストを扱う場合、(a) target を Rust シンボルにする限り DA-002/003 は UNKNOWN で止まる（仕様の対象種別の話）、(b) 契約網羅（コマンド/method 一覧の一致）や境界 assert の十分性は、テスト設計の規律に寄る面もある（運用ルールの話）。両者が混じるため未定というのは妥当。

（達成可能 subset のみを管理して緑化する運用は現仕様のまま可能だが、test_traceability は全テスト管理を要するため、black-box テストの扱いが決まるまで dogfood 全体は完了しない。）

## 再現コマンド

```
vtest audit static --test TEST-DOGFOOD-M3-TARGET-RULES        # → per-target PASS
vtest run --test TEST-DOGFOOD-M3-TARGET-RULES                 # measured
vtest verify --test TEST-DOGFOOD-M3-TARGET-RULES              # → 8/12 PASS, test_traceability MISSING
vtest verify --items test_traceability                        # → 205 未注釈 / 24 ファイル
```
