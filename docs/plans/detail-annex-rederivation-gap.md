# 詳細設計 別紙 A/C/B 再導出ギャップ（凍結要件 + 再導出基本仕様への適合）

独立監査（read-only, Opus 5, 新規文脈）。canon = 凍結「要求・要件定義 v0.1」(WHY) + 再導出「基本仕様 v0.1」(WHAT)。方向は上流→下流のみ。**別紙の再導出は詳細設計本冊の再導出完了に依存**（別紙が本冊の model=検査項目・到達性・意味監査 kind 等を相互参照するため）。

## 判定基準（上流の到達目標）
- エンティティ: 総称 `document`(DOC-) → VO → Test。**SPEC-/REQ- 実体層なし**（基本仕様§2.2/§3.1、要件§3.2）。
- 検査: **4本のみ** chain_integrity / orphan_detection / target_binding / oracle_presence。段=リンクで検査は増えない。
- 状態: **5値** PASS/FAIL/MISMATCH/NO_EVIDENCE/UNKNOWN。MISSING/NOT_EXECUTED/NOT_CHECKED/STALE は**診断ラベル**（状態でない）。
- role/anchor/anchor_rationale は **v0.1 不採用**、covers≥1 一律。
- 意味監査は検査でなく **UNKNOWN エスカレーション → 判断記録**。判断受理は検証状態を昇格させない（基本仕様§11.3）。
- 判断記録(actor/subject/decision) と 承認記録(approver/subject/approved state) は別軸・別 entity。
- コマンド正典: 基本仕様§26.1 `vtest doc add/list/show`、§26.2 MCP `doc_list/doc_get/doc_upsert`。

## 別紙A インターフェース仕様 — 再導出ワークリスト
1. `vtest spec`/`vtest req` コマンド、`spec_*`/`req_*` MCP を**除去**し `vtest doc add/list/show` + `doc_list/doc_get/doc_upsert`（derives_from・根指定引数）へ**新設**（基本仕様§16/§26.1/§26.2）。
2. `vtest verify`/`report`/ツリー出力/§13.2 の**12項目→4検査**全面書換え。spec_coverage/vo_decomposition/test_existence/static_audit/semantic_audit/test_execution/runtime_result/evidence_validity 廃止、target_execution→target_binding、static/semantic→oracle_presence 統合（§5/§22）。
3. 状態語彙を**5状態+診断ラベル2列**へ。ツリーの MISSING/STALE を状態から分離（§4.1/§4.2）。
4. `role_declared/anchor_declared/実効role`(§12.1 L29-34)、§14.4 全体、§15.3 role/anchor キー、`show --role` を**除去**（§12, covers≥1）。
5. `audit bundle/submit` を**判断記録 bundle/submit へ再定義**（kind は意味検査でなく UNKNOWN 判断対象・非昇格明記）。判断記録面(.verify/decisions actor/subject/decision)と承認記録面の分離を新設（§11.3）。
6. vo コマンドの `--req/--spec/--section` を **document 参照へ**（VO derives_from document 直結）。
7. verify の `--req` 除去。
- **MISSING 新設**: doc コマンド群 / 判断記録 IF（判断=承認分離・非昇格）/ 診断ラベル語彙面（NO_EVIDENCE が A/C に不在）/ orphan_detection 根指定コマンド / フェーズゲート評価 IF（基本仕様§20 MUST）。
- **CONFORM 生存**: envelope/exit0-3/registry/wire-compat、init/scan/doctor、vo コマンド殻、test create/edit、audit static(DA命名はHOW)、run/report/mcp 殻、§14.1-14.3 schema/validators（`test_kind` の regression は**意図ラベル**で廃止 role とは別概念・生存）、§15.1-15.3 編集機構。

## 別紙C 受入仕様 — 再導出ワークリスト
1. §18.2 fixture の **SPEC/REQ→document(DOC-)**、role/anchor/characterization fixture 群と E-SCAN-007/013/014/015 を**除去**（§12）。
2. §18.2 L41・§18.3.5 L190 の**状態列挙を5状態へ、MISSING/NOT_CHECKED/NOT_EXECUTED/STALE を診断ラベル欄へ分離**（§4.1/§4.2）。
3. §18.3.4 を**意味監査受入 → 判断記録受入へ再導出**。impl-consistency/spec-coverage/vo-coverage の検査扱い廃止、bundle/submit は「判断記録・非ゲート」。impl_consistency=MISMATCH 写像削除（§11.3）。
4. §18.3.5 の**12項目→4検査**、role別表示除去（§22/§12）。
5. §18.3.2/§18.3.3 の**項目名書換え**（static/semantic→oracle_presence、test_execution/runtime_result/target_execution→target_binding、evidence_validity→§6 ハッシュ束縛制約）。実体の受入条件（確定違反FAIL/限界UNKNOWN/hash鮮度/per-target）は生存。
- **MISSING 新設**: chain_integrity（document derives_from・hash 照合）受入 / orphan_detection（文書層孤児・根除外）受入 / NO_EVIDENCE を生む入力の受入 / 判断記録・承認記録の分離受入（判断非昇格）/ scope 2軸モデル（基本仕様§4.6）受入 / フェーズゲート評価（§20 MUST）受入。
- **CONFORM 強生存**: §18.1 共通、Source Target locator/SRC-ID/DA-002/003 到達性、§18.3.1 identity/hash/Relation/承認closure、§18.3.6 target_binding per-target、§18.3.7 Structured Test Operation、§18.3.9 adapter contract、§18.4 OOS。

## 別紙B 実装計画（プロセス文書・PROCESS_DRIFT 記録）
正規再導出の対象外。A/C 再導出後、その完了条件(=C §18)と項目名(=4検査/5状態)へ M5/M6/§2 fixture/M2 を追随させれば drift 解消。
- §2 fixture の SPEC/REQ 前提、M2 spec/req コマンド、M5 意味監査 bundle 4種別・impl-consistency=MISMATCH、M6 12項目/固定12項目 → 全て旧モデル前提。
- B は完了条件を別紙C §18 に委譲しているため C の drift を推移継承。M3(DA)/M4(Evidence)/M7(target_execution=target_binding) は実体整合・命名のみ旧。

## 全別紙共通
全「基本仕様 §n」相互参照が旧 revision 採番（例: C「§4.2の12項目」だが現§4.2=診断ラベル）。**全上流相互参照の再ポイント必須**。別紙A/C 修正は**本冊再導出に依存**（本冊が §5.2 role_declared・§7.3 到達性・§8 意味監査 kind 等を旧モデルで定義）。
