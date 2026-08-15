# クラスタ層化 sampling 再懐疑パス結果（19/19 UPHELD・汚染 caveat 退役）

wf_e9839da5-30f（19 agents, opus/effort-high, 反証デフォルト姿勢・隔離 mktemp 再現・汚染チェック必須・repo read-only）。全 dossier: `docs/plans/dogfood-sampling-reskeptic.json`（local path は redact 済み）。

## 方法

39 CONFIRMED から**各クラスタ代表1件 + singleton 全9件 = 19件**を層化抽出。各 agent への指示: 8点鎖の最弱リンクを攻撃（到達性・先行 gate・normative 期待の再読・現行コードとの一致）、隔離 temp プロジェクトでの独立再現、汚染文脈依存の判定。

## 結果

| verdict | 件数 |
|---|---|
| **UPHELD** | **19** |
| OVERTURNED / TAINTED / UNCLEAR | 0 / 0 / 0 |

- 全19件で反証攻撃が失敗し、独立再現が dossier と一致（多くが byte-identical）。
- confirm-bias への自己注記: 全件 UPHELD という一様性自体は疑わしく見えるが、各 dossier には**半命中の反証記録**が残る（例: VO-PLAN-02 の expected は MISSING でなく NOT_CHECKED が正 — 詳細設計 L1352。ただし非 PASS である点は不変で 8点鎖は破れない）、引用行ドリフトの指摘、そして後述の**帰属1件の覆し** — rubber-stamp の痕跡ではない。

## 汚染 caveat（dd7dd44）の退役

2つの独立根拠で **TAINTED 0**:

1. **構造的**: project root 解決は `find_project_root`（store/lib.rs:534-546）による **cwd 祖先walk のみ**。元検証の repro は AppData temp 配下で走っており、repo root の汚染 config（calc-m1-base）には構造的に到達不能。global/env fallback も無い。
2. **経験的**: 全19件を復元後の clean repo + 新規 build で最初から再現し、結果一致。
3. **全数走査**: 42 dossier 全件の repro 記述を repo-root 実行痕跡（`--project <REPO>` / `cd <REPO>` / `cwd=<REPO>`）で機械走査 → **0件**、かつ全42件が隔離実行を明記。記録された repro 手順に repo-root 実行は存在しない。

→ 「dossier repro が汚染文脈で走った可能性」の caveat は**退役**。残余リスクは未抽出20件だが、いずれも UPHELD 代表と機構を共有するクラスタ member であり、loci は clustering 裁定の spot-check で部分裏取り済み。

## 裁定: VO-EXIST-09 の再配属（クラスタ構成の amendment）

再懐疑 agent が cluster 帰属1件を覆した: VO-EXIST-09（text report に test_traceability の diagnostic 詳細が出ない）の支配的原因は **renderer/scope 側**（Project checks 節不在・tree-XOR-items・test_traceability が全 tree node に不在）= cluster 9 の機構であり、cluster 10（producer 側 basis 平坦化）の fix は**この欠陥に必要でも十分でもない**（renderer が envelope.diagnostics を join すれば basis に触れず仕様を満たせる）。

main thread 裁定: 元 worker は「cluster 9 の fix でも VO-EXIST-09 は解消しない」と主張しており（notes 項6）、これは十分性についての**事実対立** — 決着は frozen ontology の claim 文で行った。VO-EXIST-09 の claim は「Non-PASS test_traceability lists each … test with adapter ID, source location, diagnostic code and verdict, identically in text and JSON reports」— **提示の義務であって basis の構造化の義務ではない**（channel 非依存）。よって diagnostics-join でも仕様を満たせる = 再懐疑 agent の十分性論拠が立つ。**採用**。

- **amendment 1**: VO-EXIST-09 を cluster 9（text-report-not-annex-a）へ再配属。producer 平坦化（verify/lib.rs:664,673）は JSON basis 半分への正当な二次寄与として注記。
- **amendment 2（cluster 9 の fix_shape 補正）**: 原文の「per-node basis lines」だけでは basis が平坦なまま新 member を解消しない。fix_shape に「**表示する grounding は構造化ソース（envelope.diagnostics の join または構造化 basis）から取得する**」を追加 — これで cluster 9 は「一つの整合的修正で全 member が消える」基準を維持する。
- cluster 10 は VO-AGG-10 単独となり **singleton に降格**（VO-AGG-10 の claim は Evidence ID/per-target 結果の提示で、producer が破棄している点が支配的 — 独立 root cause として存続）。
- **新構成: 9 clusters（29件）+ 10 singletons = 19 独立 root cause（総数不変）**。`dogfood-rootcause-clusters.json` は worker 原文のまま — 本 amendment が JSON に優先する（w8-rootcause-clusters.md の優先規則と同じ）。

## 副次所見

- VO-INTAKE-08 の agent が独立に確認: **SPEC record を削除しても spec_coverage が PASS のまま** — 詳細設計 §11.1 の「SPEC record/source 不在→MISSING」に反し、cluster 1 の「正規経路が構築不能」を別角度から裏付ける（cluster 1 の member 扱いの追加候補ではなく、既存 member の機構の追認）。
- VO-PLAN-02 dossier への軽微訂正2点（verdict 不変）: expected は NOT_CHECKED が正（上述）、TEST-CLI-040 の vacuity は m1 base fixture に REQ record が0件であることが原因。

## ステータス更新

**19 root cause（9 clusters + 10 singletons）は「confirmed 候補の単位」から「修正計画単位」へ昇格**。ただし直接再検証したのは 19/39 — 残り20件は「同クラスタ代表の UPHELD + 機構共有 + loci spot-check」による**間接確認**であり、その区別はここに残す。修正着手の可否・優先順位は P-001 どおり Owner 判断。
