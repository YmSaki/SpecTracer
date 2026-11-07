# adapter 受入マトリクス（計画 v0.2 §9.1）の現行条項への対応表

作成: 2026-11-04（#76）

計画 v0.2 §9.1 の表は工程資料であり、規範ではない（AGENTS.md「実装計画は process material」）。
この表は、計画の各行を現行の正本の条項に対応付け、現行の期待値と、それを断言するテストを示す。
計画の期待値が現行の正本と食い違う行は「計画値は旧い」に理由を書いた。

テスト ID の所在は `grep -rn "@vtest.id <ID>" crates` で引ける。
本 PR で追加したテストには ★ を付けた。

## 凡例

- **済**: 現行の期待値を断言するテストがある。
- **範囲外**: 計画の行が前提とする仕組みが現行の正本にない。根拠の条項を添えた。
- **実装差異**: 現行の条項はあるが、実装がまだ従っていない。テストは足さず、報告する（#76「範囲外: 実装の変更」）。

## 対応表

| # | 計画の Case | 現行の条項 | 現行の期待値 | テスト／判定 | 計画値は旧い |
|---|---|---|---|---|---|
| 1 | v1 config + Rust fixture | DES-109、DS-357、SPEC-399 | v1 は単一の `rust-cargo` adapter として in-memory で読み、config を書き換えない。verify は固定4検査 | 済: ★TEST-MATRIX-V1-CONFIG-FIXED-FOUR-NO-REWRITE、TEST-STORE-V1-UPCONVERTS-TO-RUST-CARGO-ADAPTER | 「固定12項目」は旧い。固定4検査（SPEC-399） |
| 2 | v1 config without full_scope | DS-357、DS-1108、DS-1494 | 欠落だけを固定4検査へ具体化し、file を書き換えない | 済: ★TEST-MATRIX-V1-CONFIG-FIXED-FOUR-NO-REWRITE | 「固定12項目」は旧い |
| 3 | v1 11-item full_scope | DS-358、DS-1108、DS-1109 | 旧12項目の列挙は version を問わず E-CONFIG-001。補完して受理しない。検証結果なし | 済: ★TEST-MATRIX-FULL-SCOPE-REJECTED-NO-RESULT（v1・v2 の 11 項目）、TEST-STORE-LEGACY-TWELVE-ITEM-FULL-SCOPE | 「test_traceability を補って12項目」は旧い。補完しない（DS-1109） |
| 4 | v2 config + rust-cargo | DES-109、DS-355 | 同じ Rust fixture で v1 と同値の scan / verify。full_scope は固定4検査 | 済: ★TEST-MATRIX-V1-V2-SAME-RUST-RESULT | 「固定12項目」は旧い |
| 5 | v2 incomplete / duplicate full_scope | DS-1662、DS-1109、DS-935 | E-CONFIG-001、終了コード 2、検証結果なし | 済: ★TEST-MATRIX-FULL-SCOPE-REJECTED-NO-RESULT、TEST-VERIFY-SUBSET-FULL-SCOPE-REJECTED | なし |
| 6 | unknown adapter | DS-352、DS-1663、DS-1292 | config が指す未知 adapter は E-CONFIG-001（registry 未登録は E-ADAPTER-001）。record を書かない | 済: TEST-SCAN-UNKNOWN-ADAPTER-ID-REJECTED、TEST-EXEC-RUN-TESTS-SKIPS-UNREGISTERED-AND-CAPABILITY-MISSING-ADAPTERS | config 側の未知 adapter は E-ADAPTER-001 ではなく E-CONFIG-001（DS-1663 の括弧書き） |
| 7 | duplicate adapter ID | DS-352、DS-1569、DES-351 | config 内の重複は E-CONFIG-001、registry の重複は E-ADAPTER-001。scan しない | 済: TEST-STORE-DUPLICATE-ADAPTER-ID、TEST-ADAPTER-API-REGISTRY-REJECTS-DUPLICATE-ID | なし |
| 8 | adapter discovery failure | DS-922、DS-1292 | E-ADAPTER-002。空の scan を正常扱いしない | 済: ★TEST-MATRIX-DISCOVERY-FAILURE-EXIT-TWO、TEST-SCAN-DISCOVERY-ERROR-CARRIES-E-ADAPTER-002 | なし |
| 9 | adapter without audit | DS-1581、DS-983、SPEC-327 | static analysis capability 欠落は `oracle_presence` = NO_EVIDENCE（NOT_CHECKED） | 済: TEST-CLI-SYNTHETIC-ADAPTER-SCAN-AND-VERIFY-WITHOUT-CORE-CHANGES。W-ADAPTER-101 は**実装差異**（下記） | 「static_audit」は独立検査ではない（DS-174、SPEC-327） |
| 10 | adapter without runner | DS-1582 | `target_binding` = NO_EVIDENCE（NOT_EXECUTED）、Evidence なし | 済: TEST-VERIFY-TARGET-BINDING-NO-RUNNER-CAPABILITY-NOT-EXECUTED、TEST-CLI-SYNTHETIC-ADAPTER-SCAN-AND-VERIFY-WITHOUT-CORE-CHANGES | なし |
| 11 | adapter without coverage | DS-1581、DS-983、DS-925 | NO_EVIDENCE（NOT_CHECKED）と W-ADAPTER-101 | 状態は済: TEST-VERIFY-TARGET-BINDING-NO-COVERAGE-CAPABILITY-NOT-CHECKED。W-ADAPTER-101 は**実装差異**（下記） | なし |
| 12 | Rust + synthetic | DES-563 | 決定論的 merge | 済: TEST-CLI-MIXED-RUST-CARGO-SYNTHETIC-SCAN-MERGE | なし |
| 13 | duplicate Test ID across adapters | DS-536 | E-SCAN-002 | 済: TEST-CLI-MIXED-DUPLICATE-TEST-ID-REJECTED | なし |
| 14 | duplicate SRC ID across adapters | DS-545、DS-564 | E-SCAN-011。target 解決を PASS にしない | 済: TEST-CLI-MIXED-DUPLICATE-SRC-ID-REJECTED、TEST-SCAN-COLLIDING-PERMANENT-SRC-ID-REJECTED | なし |
| 15 | stale synthetic Evidence | DS-819、DS-833、DS-1582 | Evidence の検証は adapter に依存しない。subject 不一致は NO_EVIDENCE（STALE）。runner のない adapter は Evidence を作れず NOT_EXECUTED | 済: TEST-E2E-STALE-SUBJECT-HASH、TEST-VERIFY-EVIDENCE-TARGET-SET-MISMATCH-STALE、TEST-VERIFY-TARGET-BINDING-NO-RUNNER-CAPABILITY-NOT-EXECUTED | 同梱の synthetic adapter は runner を持たない（DS-1584 の範囲）。STALE の判定は core 側で adapter 共通 |
| 16 | non-adjacent metadata changed | DES-077、DES-078、DES-080、DS-819 | metadata の変更は Test subject hash を変え、Evidence は NO_EVIDENCE（STALE） | 済: TEST-MODEL-TEST-SUBJECT-HASH-EACH-METADATA-FIELD-CHANGES-HASH、TEST-SCAN-PROJECT-HASH-CHANGES-ON-COVERS-SOURCE-EDIT、TEST-E2E-STALE-SUBJECT-HASH | 「Audit STALE」は旧い。静的解析は監査レコードを持たない（DS-1073） |
| 17 | assertion_macros changed | DS-1073、BD-135、BD-299 | 静的解析は毎回現在の config から再計算する派生。鮮度を持つ record がない | **範囲外**: 監査レコードの鮮度という前提が成り立たない | 「static Audit Record の config subject mismatch」は旧い |
| 18 | static helper changed | DS-1073、DS-1123、BD-299 | 同上 | **範囲外** | 「static Audit STALE」は旧い |
| 19 | static-rule-irrelevant config changed | DS-1073、BD-135 | 同上 | **範囲外** | 「static Audit Record の config subject 一致」は旧い |
| 20 | revision commit missing | DS-820、DS-339 | `target_binding` = NO_EVIDENCE（STALE） | 済: ★TEST-VERIFY-EVIDENCE-REVISION-ABSENT-OR-DIFFERENT-STALE | 「evidence_validity」は独立検査ではない（SPEC-325）。`target_binding` の STALE |
| 21 | current HEAD differs from Evidence | DS-820、DS-270 | 同上 | 済: ★TEST-VERIFY-EVIDENCE-REVISION-ABSENT-OR-DIFFERENT-STALE、TEST-EXEC-STATE-HEAD-REVISION-CHANGE-CHANGES-HASH | 同上 |
| 22 | target外 helper / local dependency changed | DES-058、DES-097、DS-821 | Execution State subject が変わり、NO_EVIDENCE（STALE） | 済: TEST-EXEC-STATE-MANIFEST-CHANGE-CHANGES-HASH、TEST-VERIFY-EXECUTION-STATE-HASH-MISMATCH-STALE | 「evidence_validity」は旧い（同上） |
| 23 | incomplete current execution snapshot | DS-822、DES-209 | UNKNOWN | 済: TEST-VERIFY-INCOMPLETE-EXECUTION-STATE-UNKNOWN、TEST-VERIFY-CURRENT-RECONSTRUCTION-FAILURE-UNKNOWN | 「evidence_validity」は旧い（同上） |
| 24 | Evidence without Execution State subject | DS-821 | 読み取れて、NO_EVIDENCE（STALE） | 済: ★TEST-VERIFY-EVIDENCE-EXECUTION-STATE-ABSENT-STALE | 「evidence_validity」は旧い（同上） |
| 25 | execution state changed during run | DS-1375、DS-532、DS-907 | E-EXEC-004、Evidence なし | 済: TEST-EXEC-STATE-PREPOST-STABILITY | なし |
| 26 | impl-consistency FAIL | SPEC-433、SPEC-330、DS-1082、DS-1185 | `impl_consistency` は検査項目ではない。実装整合は判断型 `impl-consistency` の判断記録になる | **範囲外**: 判断記録は #88（PR #149）の対象 | 「CheckValue MISMATCH」は旧い |
| 27 | Specification changed after impl-consistency Audit | DS-901、DS-404、DS-1613 | 判断記録は依存 closure のハッシュ束縛で自動的に失効する | **範囲外**: 判断記録は #88（PR #149）の対象 | 「impl-consistency Audit STALE」は旧い。Audit ではなく判断記録 |
| 28 | bare Relation compatibility input | DES-125、DES-542 | bare ULID を受理し、`REL-<ULID>` へ in-memory で正規化し、file を書き換えない | **実装差異**（下記） | なし |
| 29 | duplicate bare / prefixed Relation payload | DS-428、DS-1294 | E-SCAN-010。いずれも採用しない | 済: TEST-SCAN-RELATION-ID-ALIASES-CANNOT-DUPLICATE-ULID | なし |
| 30 | duplicate Form kind across adapters | DS-534、DES-489 | E-ADAPTER-001。Rust adapter を名前から推測しない | **実装差異**（下記） | なし |
| 31 | SPEC requirement without active REQ | SPEC-433、DS-1655、DS-1650 | `spec_coverage` は検査項目ではない。上流のない文ノードは `orphan_detection` = MISMATCH（E-SCAN-016） | 現行の相当: TEST-SCAN-REPORTS-ORPHAN-NODE-NO-EFFECTIVE-UPSTREAM。計画の行そのものは**範囲外** | 「spec_coverage non-PASS」は旧い |
| 32 | SPEC / REQ dependency changed after VO approval | DS-1467 | 承認は依存 hash 不一致で無効になり、VO は draft | 済: TEST-STORE-APPROVAL-STALE-DEPENDENCY-HASH-DRAFT、TEST-STORE-APPROVAL-STALE-SUBJECT-HASH-DRAFT | なし |
| 33 | scan rejected by adapter | DS-935、DS-929 | E-ADAPTER-*、終了コード 2、scan 結果なし | 済: ★TEST-MATRIX-DISCOVERY-FAILURE-EXIT-TWO | なし |
| 34 | limited scope | DS-1110 | scope 外は NO_EVIDENCE（NOT_CHECKED）。PASS にしない | 済: TEST-VERIFY-OUT-OF-ITEM-SCOPE-NOT-CHECKED、TEST-ACCEPTANCE-18-3-8-OMITTED-ITEMS | なし |
| 35 | CLI / MCP same input | DS-1563 | 同じ data / diagnostics | 済: TEST-MCP-SCAN-MATCHES-CLI、TEST-MCP-VERIFY-MATCHES-CLI ほか TEST-MCP-*-MATCHES-CLI | なし |

## 実装差異（報告）

テストを足すと落ちる行である。実装は変更していない。

1. **W-ADAPTER-101 が出ない**（#9、#11）。DS-983 と DS-925 は、capability の欠落を観測したときに warning W-ADAPTER-101 を返すとしている。
   `crates/` のどこにもこの code を出す処理がない（`grep -rn "W-ADAPTER-101" crates` は 0 件）。
   非 PASS の状態と診断ラベル（NOT_CHECKED / NOT_EXECUTED）は条項どおりである。
2. **bare Relation を `REL-<ULID>` へ正規化しない**（#28）。`RelationRecord::from_yaml` は bare ULID を受理し、file は書き換えない。
   ただし `id` は bare のまま保持し、DES-125 / DES-542 の「`REL-<ULID>` へ in-memory で正規化する」を満たさない。
   scan は Relation を検証するだけで結果に載せないので、今は外から観測できない。
3. **Form registry が adapter ごとにない**（#30）。built-in Form（`rust-unit-function` / `rust-integration`）は `vtest-store::forms::load_form_schema` が名前で直接返す。
   adapter をまたぐ kind の重複を E-ADAPTER-001 で拒否する registry（DS-534、DES-489）がない。
   計画 §9.2「Rust 固有 built-in form は Rust adapter だけが登録する」も未達である。
   Structured Test 構文の adapter 移管（#73、PR #155）の後続になる。
