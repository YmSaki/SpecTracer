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
- 集中領域: VO-PLAN 系 7、VO-SEMAUDIT 系 7、VO-EXEC 系 5 — 監査受理・検証規則まわりに候補が密集（真の未実装/穴か、agent の過剰判定かは検証パスで判別）。

## proves_no_vo 30 test の分類

| カテゴリ | 件数 |
|---|---|
| **A ontology-gap（導出漏れの obligation を検証していた）** | **8** |
| B supporting（helper/parser 検証・正当な存在） | 19 |
| C regression（過去 bug の再発防止・正当な存在） | 3 |
| D/E/F | 0 |

A の例: 「compat wire fields は decode 時に neutral execution と一致必須（基本仕様 §2.4）」「v1 flat config の compat 読み（詳細設計）」— **top-down 導出が config-compat / wire-compat 領域の obligation を落としていた**ことを既存 test が逆照射。ontology への追補候補 8 件。

## 次段（remap はまだ）

1. **CONTRADICTED 42 の検証パス**: 候補ごとに引用コードを main thread（紛糾時のみ Fable 単発）で追試し、CONFIRMED / REFUTED / NEEDS-SPEC-JUDGMENT に確定。confirmed は SpecTracer 本来の出力「仕様・設計上成立すべき命題に対する反証 evidence」として一級の成果。
2. A-gap 8 件の ontology 追補（freeze の追記手続き＝gate を単発で通す）。
3. その後に新 covers 設計 → 適用 → doctor → 旧63 retire（比較資料保存）。
