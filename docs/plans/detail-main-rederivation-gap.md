# 詳細設計 本冊 再導出ギャップ（凍結要件 + 再導出基本仕様への適合）

独立監査（read-only, Opus 5, 新規文脈）。canon = 凍結「要求・要件定義 v0.1」(WHY) + 再導出「基本仕様 v0.1」(WHAT)。詳細設計本冊(§1〜§11・§16・§17・§19)は Issue #11 以前の旧基本仕様に対して書かれ、**全面再導出が必要**。旧モデル4系統（12検証項目/8値フラット CheckValue/SPEC-REQ 型付き文書層/role・anchor 機構）が全体を貫通。全「基本仕様§n」引用が旧連番。kihon-rederivation-gap.md の C-1〜C-8/M-1〜M-4 がほぼ1:1で転写できる。

## 中核 旧モデル残存 × 基本仕様の衝突
- **12検証項目**（§2.2 config `verify.full_scope`, §5.2 `enum CheckItem`, §11.1表）→ 4検査(chain_integrity/orphan_detection/target_binding/oracle_presence)。「§4.2の12項目・追加削除不可」=MAJOR CONFLICT。
- **8値フラット CheckValue**（§5.2 `{Pass,Fail,Mismatch,Missing,NotChecked,NotExecuted,Stale,Unknown}`）→ 5状態+診断ラベル2軸。NO_EVIDENCE が本冊に皆無、NotChecked/NotExecuted/Stale を状態値として直接使用=CONFLICT。
- **SPEC/REQ 型付き文書層**（§2.1 `spec/`/`req/`, §3.1 SPEC, §3.2 REQ, §5.3 グラフ REQ→SPEC/VO→REQ）→ 総称 document(DOC-)+derives_from、VO は document 直結。VO の `requirements`/`spec_refs` を `derives_from:[DOC-]` へ=CONFLICT。
- **role/anchor/anchor_rationale**（§4.1/§4.2 キー・語彙・characterization, §4.4 実効role確定+E-SCAN-013/014/015, §5.2 TestRole/TestAnchor enum, §11.1.2 適用項目集合）→ 削除し covers≥1 一律。E-SCAN-007 の role条件付き covers は基本§12 と直接矛盾=MAJOR CONFLICT。
- **Test Intent**: §5.2 で概ね付随扱い=CONFORM（基本§13）。
- **runtime_result 検査**（§2.2, §11.1）→ 検査除去、ランナーPASSを target_binding 証拠へ（C-3）。
- **独立 evidence_validity 検査**（§1.2 L73, §2.2, §11.1/§11.2）→ 検査解消し STALE 診断ラベル、ハッシュ束縛は設計制約で温存（C-8）。§11.2 鮮度判定機構自体は CONFORM、産出ラベルのみ書換。
- **意味監査ゲート**（§3.6 監査レコード `kind:spec-coverage/test-semantic/vo-coverage/impl-consistency`, §8 全体, §8.3 verdict→CheckValue 写像）→ 検査ゲートから除去（基本§11「このプロトコルは検証状態のゲートではない」）。bundle→hash検証submit→ハッシュ束縛保存は判断記録(actor/subject/decision必須・理由optional)へ転用。§8.4 E-AUDIT-005「reasons が空でなく claim/basis 必須」は要件§12「理由が空であることだけを根拠に無効扱いしない」と矛盾=除去。`kind:static` のみ残し oracle_presence/target_binding へ供給=MAJOR CONFLICT。
- **承認の PASS 前提**（§3.5）→ 承認は検証状態と独立軸（基本§4.5/§17）。§3.5 依存closure 機構は CONFORM だが kind:vo/req/spec を vo/document へ、基本§17 の「参照document の再帰的上位document」を closure に追加。
- **DA-002/003/006 到達性**（§7）→ 機構 CONFORM/分類 CONFLICT。DA-002 静的UNKNOWN→当該target runtime到達で昇格 join は基本§5.3 そのもの(修正不要)。現行4検査では DA-002到達性+§10 coverage=**target_binding**、DA-001/003/004/005/006=**oracle_presence**。「static_audit 単一検査を2検査へ分割」であり到達性モデル自体の書換ではない。§7.1「UNKNOWN→意味監査へ委ねる」は UNKNOWN→判断記録エスカレーションへ。

## MISSING（上流が要求/委譲するのに HOW 不在）
- **M-1** 文書層 orphan_detection 実装+根の指定方式（基本§5.2、根指定は§30 item2 委譲）。§5.4 W-SCAN-102 は「孤立VO」で文書層 orphan ではない。
- **M-2** 判断記録層（基本§11、`.verify/decisions/`）。§8 を転用して新設。
- **M-3** フェーズゲート評価・提示【MUST】（基本§20/要件§26.4）。本冊に皆無。優先度高。
- **M-4** 役割別 projection 取得・提示（基本§19/要件§3.4）。§11.3 report は固定ツリーのみ。
- **M-5** 判断待ち情報の構造 schema と取得IF（基本§18.3/§30 item19）。
- **M-6** derives_from の任意説明文（基本§3.2/§19、空可・非MISMATCH）が総称document側に無い（Relation の note のみ）。

## 節別判定サマリ
- **CONFORM**: §0, §1.1(audit crate責務のみ読替), §2.3, §3.4, §3.7, §4.3, §6全体, §9(§9.1 --req除く), §10, §16.2, §17.2, §19
- **CONFLICT**: §1.2(L73), §1.3, §2.1, §2.2, §3.1, §3.2, §3.3(+§3.3.1), §3.5, §3.6, §4.1, §4.2, §4.4, §5.1(step4), §5.2, §5.3, §5.4, §7全体(分類), §8全体, §11.1, §11.1.1, §11.1.2, §11.2(ラベル), §11.3, §11.4, §16.1, §17.1
- **MISSING**: M-1〜M-6
- **STALE**: 該当なし（逸脱は全て「凍結が検査から排除した判断の検査化」型で CONFLICT 書換に収束）

## 再導出ワークリスト（依存順）
**フェーズ0 全引用修復**: 全「基本仕様§n」を現行連番へ。特に「§4.2の12項目」(検査=§5/状態=§4)、「§4.3の優先順位」(→§22.2)、「§6.2 role制約」(現行§6にrole無し)、「§7.4-7.9意味監査」、「§5.2原子的公開」(→§24.2)。

**フェーズ1 状態・型基盤**: (1)§5.2 CheckValue 8値→5状態+診断ラベル2軸 (2)§5.2 CheckItem 12→4検査 (3)§5.2 TestEntity から role/anchor系・TestRole/TestAnchor 除去 (4)§1.3 ハッシュから role/anchor 束縛除去、VO/REQ/SPEC hash→総称document hash。

**フェーズ2 文書層総称化**: (5)§2.1 spec/+req/→doc/, audits/→decisions/ (6)§3.1/§3.2→総称document レコード1種 (7)§3.3 VO→derives_from:[DOC-] (8)§5.3グラフ・§5.4 E-SCAN-012・§11.1.1 を DOC/derives_from へ (9)§3.5 closure kind→vo/document+上位document再帰、§11.4/§16.1 SPEC鮮度→document鮮度。

**フェーズ3 role/anchor全廃**: (10)§4.1/§4.2 role/anchorキー・語彙・characterization 除去 (11)§4.4 role materialization(E-SCAN-013/014/015)撤去→「id/covers≥1/targets≥1/intent欠落→MISMATCH(MISSING)」 (12)§5.4診断表・§11.1.2適用項目集合を再構成。

**フェーズ4 意味監査→判断記録転用**: (13)§3.6/§8 の4意味監査kind を検査ゲートから除去、§8.3写像全廃 (14)bundle/submit を判断記録へ転用 (15)§8.4 E-AUDIT-005/006/007除去・001〜004存続、§17.1追随 (16)kind:static 分離維持。

**フェーズ5 4検査再マップ**: (17)§7 static_audit を target_binding(DA-002+§10)と oracle_presence(DA-001/003/004/005/006)へ分割、§7.1→UNKNOWNエスカレーション (18)§11.1/§11.3 を4検査・DOC/VO/Test ツリー・5状態・§22.2優先順位へ、§2.2 config/E-CONFIG-001 を4検査invariant へ (19)§11.2 evidence_validity産出→NO_EVIDENCE/STALE、runtime_result→target_binding証拠吸収。

**フェーズ6 新設**: (20)orphan_detection+根指定(M-1) (21)フェーズゲート評価・提示(M-3,MUST) (22)役割別projection(M-4) (23)判断待ち情報schema(M-5)。

## 再マップ表1: 旧12項目→4検査
test_existence→chain_integrity / test_traceability→chain_integrity / static_audit→oracle_presence+target_binding(DA-002) / target_execution→target_binding動的 / runtime_result→除去(ランナーPASSを証拠化) / test_execution→target_binding証拠鮮度 / evidence_validity→除去(STALE診断) / spec_coverage・vo_coverage・vo_decomposition・semantic_audit・impl_consistency→検査から除去(§12エスカレーション/判断記録) / 新設 orphan_detection。

## 再マップ表2: 8値→5状態+診断ラベル
Pass→PASS / Fail→FAIL / Mismatch→MISMATCH / Missing→MISMATCH+診断MISSING / NotChecked→NO_EVIDENCE+診断NOT_CHECKED / **NotExecuted→分裂**(Evidence不在→NO_EVIDENCE/NOT_EXECUTED[基本§21.1]、実行count0→FAIL/NOT_EXECUTED[基本§4.3]) / Stale→NO_EVIDENCE(証拠鮮度失効)またはMISMATCH(文書鎖hash不一致)+診断STALE / Unknown→UNKNOWN。集約優先順位=基本§22.2 FAIL>MISMATCH>NO_EVIDENCE>UNKNOWN(診断ラベルは順位に用いず併記)。

---

## Disposition（2026-08-24）: 検証者 NEEDS_OWNER への処遇 — 下流修正（Owner 裁定不要）

**指摘（detail-main-verifier）**: 詳細設計が検証対象を Source Target（実装 construct）へ core レベルで一律収斂し、`targets ≥ 1` を必須化（§4.1/§4.4/§5.2、E-SCAN-007→chain_integrity MISMATCH）、§1.3 で target hash を construct bytes へ束縛、§7.3 で「target を持たない Test は到達未充足」。上流 WHAT の狭窄では、と NEEDS_OWNER。

**処遇: 下流修正で解消（Owner エスカレーション不要）。** 根拠は canon が一意に決めること:
- 凍結要件 §9.1「検証対象は…実装 construct そのものに限定しない…外部契約・境界上の振る舞いも検証対象にできる」「内部 Source Target の宣言を Test 成立性の必須条件としない」。§9.2 は Source Target 宣言を「できる」（capability）。§9.3 traceability は任意。§4.3「特定の実行形態の確認方法を別形態の Test へ一律要求してはならない」。
- 再導出基本仕様 §9（同旨）、§12 chain_integrity の Test 層必須は「Test ID・covers≥1・必須 metadata」で **targets を含まない**、§5.3「他の実行形態の確認方法は詳細設計・特定形態を他形態へ一律要求しない」。
- 両 canon 層は内部矛盾なく検証対象を一般に保つ。狭窄は詳細設計本冊のみが core レベルで導入した下流の逸脱。検証者の「上流内の緊張（§9.1 vs §5.3）」は誤読（§5.3 は Source-Target 形態の**確認方法**を定めるのみで、全 Test に当該形態を要求しない）。よって要件から一意に決まり Owner 判断は不要（P-005: 下流を上流へ適合）。

**修正方針（relocation + 一般化。capability 構築や新 schema 発明は禁止＝v0.2 scope creep）**:
1. adapter 中立 core: 管理対象 Test は **≥1 検証対象（一般概念）** を要求。検証対象は Source Target として実現し**得る**が、targets-as-source-construct を core chain_integrity 必須にしない。E-SCAN-007 を core から adapter 層へ移設/再キー。
2. rust-cargo adapter 層: 当該 adapter は検証対象を Source Target として実現し ≥1 を要求。→ **既存の rust-cargo の挙動・E コード・fixture は実効的に不変**。
3. §1.3 hash: construct bytes 束縛は **Source-Target 実現形態の束縛**として記述（core の target 定義にしない）。§7.3「target を持たない Test は到達未充足」も **form-scoped** 化（普遍規則にしない）。
4. 非 source 境界形態の表現・確認は **明示的に委譲/据え置き**（凍結 §4.3・基本 §5.3）。Contract-Target 類の schema を新設しない。
5. editorial: §17.1 E-OP-001 の「§6.2」参照を **§6.3** へ修正（候補提示は §6.3）。

修正後、同一検証者（detail-main-verifier）へ delta 再検証を依頼。緑なら本冊を v13 コミット。
