# Evidence adequacy 結果（composability gate + contradiction duty + proves_no_vo 分類）

wf_99f90f2a-be8（15 area + 1 分類 agent, opus/effort-high, read-only）。全量: `docs/plans/dogfood-evidence-adequacy.json`。

## VO 最終4値（154件・現時点）

| 分類 | 件数 | 意味 |
|---|---|---|
| PROVEN | **14** | 単独 PROVES 13 + evidence set が composable と判定された 2（VO-REGISTRY-09, VO-AGG-01）− 再判定降格 1 |
| PARTIAL | **67** | 必要 facet の一部のみ evidenced |
| UNSUPPORTED | **31** | 必要 facet ゼロ（uncovered 含む） |
| **CONTRADICTED（候補）** | **42** | agent がコード引用付きで「実装が claim に反する」と報告。**全件が未検証の候補であり、確定欠陥ではない** — main-thread / 高信頼レビューの検証パスを通すまで昇格させない |

composability gate は実際に効いた: facet が揃っても構成不能（別機構・別設定を叩いている）と判定された組が複数（例: VO-INTAKE-04 は「approval 側は registered sha256 / audit 側は current source hash」という**二重機構の非対称**そのものが原因で non-composable かつ CONTRADICTED 候補）。

## CONTRADICTED 候補の代表例（コード引用付き・未検証）

- **VO-INTAKE-08**: `vo_decomposition` は spec_refs を一切検査しない（verify/lib.rs:584-598）。scan も REQ/VO の spec_refs 解決を検査せず（scan/lib.rs:714-840 に検査なし）、`SPEC-GHOST` を引く REQ/VO でも **PASS** になる — §11.1.1 の「解決不能→MISSING」に反する fail-closed 穴の疑い。bundle も unresolvable 参照を黙って drop（cli/lib.rs:1328,1480 の `if let Ok`）。
- **VO-INTAKE-04**: audit の SPEC subject は registered sha256 でなく**現在の source hash**に束縛（cli/lib.rs:1327-1345 ほか）。register 後に source を編集 → bundle/submit すると、W-SCAN-104 が生きたまま audit が VALID になる経路が到達可能。approval 側（store/approval.rs:138-151）は registered sha256 を守っており、**resolver 非対称と同型の subsystem 間非対称**。
- 集中領域: VO-PLAN 系 8、VO-SEMAUDIT 系 7、VO-EXEC 系 5 — 監査受理・検証規則まわりに候補が密集（真の未実装/穴か、agent の過剰判定かは検証パスで判別）。

## proves_no_vo 30 test の分類

| カテゴリ | 件数 |
|---|---|
| **A ontology-gap candidate（upstream re-derivation candidate）** | **8** |
| B supporting（helper/parser 検証・正当な存在） | 19 |
| C regression（過去 bug の再発防止・正当な存在） | 3 |
| D/E/F | 0 |

A の例: 「compat wire fields は decode 時に neutral execution と一致必須（基本仕様 §2.4）」「v1 flat config の compat 読み（詳細設計）」。**現時点で確定しているのは「現154 VO に対応先が無い」まで** — Test は gap の**センサー**であって ontology の正典ではない。各件は上流（Normative Spec → design mechanism）から再導出し、spec に要求が無ければ VO に追加しない（8件全部が追補されるとは限らない）。

## 次段（remap はまだ）

1. **CONTRADICTED 42 の検証パス**（4値: **CONFIRMED / REFUTED / NOT-REPRODUCIBLE / NEEDS-SPEC-JUDGMENT**）。REFUTED=候補の論理自体が誤り、NOT-REPRODUCIBLE=local observation は正しいが reachable path/前提の欠如で system-level 反例が成立しない — 両者の区別が contradiction detector の品質評価を可能にする。NEEDS-SPEC-JUDGMENT で spec が silent なら **SPEC GAP** として surface（実装バグではない）。CONFIRMED の成立条件は8点鎖を必須とする: (1)VO claim (2)normative source (3)design mechanism (4)反例を起こす concrete condition/input (5)implementation path (6)expected (7)actual (8)**actual が claim を論理的に否定する理由**（local omission ≠ system-level contradiction。他 gate が必ず reject するなら非成立。reachable + 他 gate 不 reject + normative behavior 破れ、まで通す）。
2. A-gap candidate 8 件の**上流再導出**（spec に要求あり→mechanism 確認→omission 確定 / 無し→追加しない）。genuine omission が出れば **freeze v2 → controlled amendment → freeze v3**（154 を最終数として守らない。Test 起点の疑義を上流から正当に再導出した amendment は freeze の失敗ではない）→ adequacy 再計算。
3. その後に新 covers 設計 → 適用 → doctor → 旧63 retire（比較資料保存）。
