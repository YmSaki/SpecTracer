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

## ★全テスト注釈を実施した結果（ultracode, 2026-08-15）— 決定的な実測

Owner 指示「問題1だったものを全部埋めてみて」を ultracode で実行。inventory(24ファイル fan-out) → ontology 生成(SPEC 流用/REQ 7/VO 63, target クラスタ) → annotation(24ファイル fan-out で `@vtest` 適用) → build/scan/verify。

### 母集団の照合（全数・決定論的に確定）
scan 可視テスト総数 **206** = 管理済み **192**（新規注釈 191 + 既存 M3 1）+ 未管理 **14**（annotation skip 12 + 誤 target 除去 1 + inventory 見逃し 1 `relation_aliases...`）。当初「205 未管理」と inventory「204」の差は見逃し1件。build + `cargo test` 全 206 PASS（注釈は doc-comment のみで挙動不変）。

### 管理化した192件の static_audit 分布（audit --all 実測）
| topology | 件数 | static_audit |
|---|---|---|
| in-process | 108 | UNKNOWN 107 / PASS 1（既存 M3, 修飾済） |
| subprocess | 84 | UNKNOWN 81 / FAIL 3 |

**注釈だけでは PASS にならない（管理化 = UNKNOWN 到達）。** 通ったのは修飾済み M3 の1件のみ。

### in-process UNKNOWN 107件の原因分布（全数・機械突合）
tool 自身の解析結果（`test list` の byte_range）で各テスト本体を切り出し、宣言 target・実呼出トークン・診断を 107件全数で突合した（トークン regex による判定＝完全な AST 突合ではない点に注意）:

| 件数 | バケツ | 内容 |
|---|---|---|
| **79** | DA-002 ambiguous / 本体に **bare call** あり / target は実在する自由関数 | 修飾なし呼出を静的に target と同定できない |
| **23** | **resolver failure / 全て impl-method target** | 下記「resolver 非対称」参照 |
| **5** | DA-002 ambiguous / 本体に target の直接呼出トークンなし | helper 経由等・個別未分類 |

さらに **DA-001（定数性）= UNKNOWN が 107/107 で普遍**（subprocess でも 76/84）。DA-002/003 と独立に PASS を阻む第3のブロッカー。原因仮説: rule_da001 の runtime 性証明が target 照合に依存するため（static_audit.rs:78-137, `classify(expr, target_resolution, ...)`）、target 同定失敗が定数性未解決に連鎖する — **仮説であり未確認**。

### 確定 finding: 2つの resolver の非対称（コードで確認済みの tool 欠陥候補）
23件の resolver failure は全て `Type::method` 形式の impl-method target。
- discovery は `Item::Impl` を走査し method を Source Target として index する（discovery.rs:814, 869）→ scan は受理（**E-SCAN-004 = 0件**）。
- static audit の `resolve_target → find_function` は `Item::Fn`/`Item::Mod` のみ走査（static_audit.rs:1361-1390）→ 同じ target が「could not be resolved」。
∴ **scan が受理する target を audit が解決できない**。§907（target 解決は core 単一経路、subsystem 毎の独自解決の禁止）が防ごうとした非対称そのもの。

### 確定 finding: DA-006 FAIL 3件は全件 helper 委譲 assert（3/3 個別確認）
TEST-CLI-022/023/060 の本体には inline assert 相当（assert!/unwrap/expect/?）が無く、`assert_adapter_usage_error` / `assert_ok` / `assert_tree_child` という **assert helper 関数**に検証を委譲している。DA-006 は関数呼出を assert 相当と認識しないため「検証構文なし=FAIL」。

### 総括（証拠に見合う強さで）
現行 static-audit を suite 全体へ適用すると大多数（188/192）が UNKNOWN となり、少なくとも次の不整合が実在する: **(1) bare-call の symbol 同定（79件, 修正先が test 側修飾か scanner 側 name resolution かは未決）**、**(2) impl-method target の resolver 非対称（23件, コード確認済み）**、**(3) DA-001 定数性の普遍 UNKNOWN（107件, 原因は仮説段階）**、**(4) helper 委譲 assert の DA-006 誤 FAIL（3件, 全数確認）**、**(5) symbol target を持たない structural 検証の未表現（14件, 現行 symbol-target-only モデルでは表現不能）**、**(6) black-box 契約テストの監査モデル未分化（84件, 問題2・未決）**。
各バケツの修正先（テスト側か scanner/model 側か）は**いずれも未決**であり、「テストを直すべき」とはこのデータからは言えない。

## 再現コマンド

```
vtest audit static --test TEST-DOGFOOD-M3-TARGET-RULES        # → per-target PASS
vtest run --test TEST-DOGFOOD-M3-TARGET-RULES                 # measured
vtest verify --test TEST-DOGFOOD-M3-TARGET-RULES              # → 8/12 PASS, test_traceability MISSING
vtest verify --items test_traceability                        # → 205 未注釈 / 24 ファイル
```
