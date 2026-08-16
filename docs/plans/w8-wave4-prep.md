# Fix wave 4 — 起動準備（wave 3 merge 後に発射）

作成 2026-08-17。wave 3（wf_5173fa00-262）走行中に main thread で準備した worker 分割・事前裁定・merge 規則。起動は wave-3 merge + 統合 gate 緑の後。

## 事前裁定（main thread 判断・実施済み）

### VO-INTAKE-04: SPEC subject 鮮度の anchor と kind 一般化 — 裁定確定
- **裁定**: state-anchored + 全 kind 一般化。仕様が一意に決めるため自由設計選択ではない。
  - 詳細設計 §11.4 の無効化文は kind 無限定（「当該 SPEC を subjects に含む監査レコードは無効（STALE）」）。
  - 基本仕様 §7.5 が impl-consistency を名指しで STALE 対象に含める。
  - 同段落の Approval 無効化は実装済みで record.sha256 比較（state-anchored）— 同一段落の監査文だけ event-anchored に読み替える余地なし（dossier の読解を維持）。
- **fix 形**: 現行の current-bytes binding（bundle 時点の実ソース hash 束縛 — 登録後 drift 検出用）は**保持**し、その上に「SPEC の登録 sha256 ≠ 現ソース hash の間（W-SCAN-104 窓）、当該 SPEC を subject に含む監査は kind を問わず STALE」を評価時条件として**追加**する。再登録で不一致が解消し、かつ束縛 hash が現ソースに一致すれば有効に戻る（state-anchored の帰結）。
- **必須アサーション2本**（B の acceptance）: ① dossier の5段階 repro（W-SCAN-104 窓内 bundle → STALE）② **回復ケース: SPEC を H1 で再登録 → H1 束縛の監査が valid に戻る**。②が state-anchored / event-anchored（窓内 bundle 永久 STALE）を判別する唯一のテスト — ①だけでは event-anchored 実装でも通る。
- wave-3 worker A の spec-coverage 限定実装（§1.3 L87）と同族 — merge 後、評価述語を共有できるなら共有し、二重実装しない。

### cluster 8 の述語共有 obligation（再掲）
- evaluate_target_execution の entry-set 導出は、verify 側 runtime-rescue が既に使う判定（wave-2 時点の実装挙動）と**述語を共有**する。二つの「canonical Source Target との突合」を並立させない。

## 完遂算術
wave1 = 8 + wave2 = 3 + wave3 = 3 + wave4 = 5（cluster 6/8/9 + VO-EXIST-08 + VO-INTAKE-04）= **19/19**。wave 4 の merge で Owner 指示の 19 root cause 修正が閉じる（完了判定基準）。

## worker 分割（3 worker・sonnet xhigh・worktree 隔離）

### Worker A: fix/w4-structured-ops — cluster 6（VO-REGISTRY-05, VO-STRUCTOP-09, VO-STRUCTOP-10）
- adapter 分離が Structured Test operation 経路に未達: create_test/edit_test が registry を受けず `vtest_adapter_rust::operations_support::*` を静的 import。AdapterRegistry::structured_test / built_in_form_kinds / accepts_compatibility_form が呼び出しゼロ。
- fix 形: registry を create/edit へ通す。Form kind → StructuredTest-capable adapter の一意解決（0件/2件以上は拒否）。宣言 rendering を adapter trait メソッド化し core から adapter 所有構文を排除（`///` vs `/** */` 非対称の根治 — render_edited_test を adapter へ）。
- 主対象: vtest-scan/src/operations.rs・vtest-adapter-api（trait 拡張）・vtest-adapter-rust。B/C と crate 単位でほぼ非干渉。
- ID レンジ: TEST-SCAN-030..039 / TEST-ARUST-030..039

### Worker B: fix/w4-exec-validity — cluster 8（VO-EXEC-09, VO-REPORT-03）+ VO-EXIST-08 + VO-INTAKE-04
- cluster 8: evaluate_target_execution が保存済み集約 scalar のみ返し entry set（record.target_execution.targets）を読まない。fix = entry set を現 canonical Source Target 集合と 1:1 検証（基数・重複・locator 同一性）して値を導出し、target 毎の report child（locator/result/count）を emit。**述語共有 obligation 上記**。
- VO-EXIST-08: Incomplete/解析不能 discovery batch を「Test 0件の完全 scan」として扱わない。DiscoveryBatch.completeness を scan 具現化から verify まで搬送し test_traceability = UNKNOWN（scan は完了・exit 1。E-SCAN-001 は completed-scan 側 — exit 区分が正典）。
- VO-INTAKE-04: 上記裁定の実装。
- 主対象: vtest-verify/src/lib.rs（evaluate_target_execution・test_traceability・evidence freshness）・vtest-scan（completeness 搬送）・vtest-store/vtest-cli（bundle 束縛周辺）。
- EXIST-08 の B 配属は確定（adapter 側の E-SCAN-001 + Incomplete 発出は実装済み — load-bearing な変更は completeness の verify までの搬送で B の領域）。
- ID レンジ: TEST-VERIFY-060..069 / TEST-CLI-140..149 / TEST-SCAN-040..044

### Worker C: fix/w4-annex-a — cluster 9（VO-REPORT-05, VO-AGG-04, VO-AGG-08, VO-EXIST-09）+ VO-AGG-10 basis 構造化
- emit_text が debug printer（tree XOR scope items — entity 存在で items 分岐が死に、project-checks 節欠落・全ノード hardcoded branch mark）。fix = Annex A 詳細 report への引き上げ + node assembly（build_req_node/build_vo_node/test_node）の対応構造化 + VO-AGG-10 の basis 構造化。
- **renderer は node children を種類非依存に描画する（child kind の列挙 hardcode 禁止）** — これにより B の per-target children は C 側無改修で表示され、B↔C merge が意味統合でなくテキスト merge に落ちる。
- 主対象: vtest-cli/src/lib.rs（emit_text/print_text_tree_node）・vtest-verify/src/lib.rs（node assembly）。
- ID レンジ: TEST-VERIFY-070..079 / TEST-CLI-150..159

## B↔C 事前 merge 規則（vtest-verify test_node が共有熱点）
- B は test_node へ per-target children（データ）を追加。C は node 構造/renderer を Annex A 形へ再構成。
- 衝突時は**両意図の和集合**: B の per-target children データを C の Annex A 描画構造の中で保持・表示する。children を落とす解決は禁止。
- merge 順: A → B → C（A は crate 非干渉で先。C が最後に renderer 統合）。

## 起動チェックリスト（wave-3 merge 完了後）
1. base hash を worker prompt に記名（wave-3 merge 後の feature tip。worktree stale-base 教訓）。
2. dossier の行番号は wave-3 merge で変位している — worker には**シンボリックアンカー**（関数名・診断 ID）で指示済みの本書を参照させ、行番号は worker 自身に再解決させる。
3. 既知 flake（static_audit_orders_offsets… → --test-threads=1 確認）・bridge VO 新規発行禁止・repo root への vtest 実行禁止を prompt に必ず含める。
4. 完了済み workflow agent へ SendMessage しない。resume は Workflow({scriptPath, resumeFromRunId})。
