# Specification Verification Closure — SpecTracer の中心命題（Owner + GPT-5.6-sol 整理, 2026-08-15）

> Tests passing is not the goal. The goal is to establish that every in-scope
> specification obligation has valid evidence, and that the evidence correctly
> detects conformance or non-conformance.
>
> Know not only that your tests pass. Know whether your specification is actually verified.

## 概念

対象範囲の仕様上の義務集合 O = {o₁..oₙ} について、∀o∈O, ∃e: Verify(e,o) が成立し、かつ各 e の鎖

```
義務 → テスト存在 → 本当にその義務を検証 → fixture が条件を成立 → 対象実装へ実到達
     → oracle が適合/非適合を識別 → 結果が現在状態に対し観測される
```

が閉じている状態 = **検証閉包（Verification Closure）**。SpecTracer が測るのは test coverage ではなく specification verification closure。

## 12項目モデル = 閉包の鎖の operational 定義

| 鎖のリンク | 検査項目 |
|---|---|
| O の列挙が完全 | spec_coverage / vo_decomposition / vo_coverage |
| ∃e | test_existence / test_traceability |
| e が o を検証 | static_audit（DA規則）/ semantic_audit |
| fixture が条件成立 | semantic_audit（intent/input/expect） |
| 実装へ実到達 | target_execution / DA-002（静的 or runtime 証明） |
| oracle が識別 | DA-001/003/004/006 / runtime_result |
| 現在状態で観測 | test_execution / evidence_validity |

「12項目 all PASS」= 閉包成立。

## Red の意味（2×2）

|  | 実装適合 | 実装非適合 |
|---|---|---|
| テスト PASS | 正常 | **見逃し（最悪）** |
| テスト FAIL | 誤検出 | **正常（検証系の成功）** |

仕様違反へ FAIL したテストは仕事を完璧に果たした。DA 規則・意味監査・target_execution は「見逃し」を各リンクで潰す装置。誤検出側は P-001（MISMATCH 提示・fix 決定はしない）。

## 3軸の分離

- **C_code**（コード実行率）: 分母が実装。100%が達成可能だが無意味（空虚 assert で膨らむ）。<100% 許容はこの metric への合理的反応だった。
- **C_spec**（義務カバレッジ）: 分母が仕様義務。分母の構築（spec→REQ→VO）が前提で、歴史的に誰も維持機構を持たなかった。分母が存在すれば「90%」は「名指しできる10%の義務が未検証」になる。
- **Q_evidence**（証拠品質）: リンクが検証として成立しているか。トレーサビリティ行列（REQ→TEST のリンク存在）では足りない — リンクの検証成立性まで見る。

SpecTracer は後ろ2つ、特に Q_evidence まで踏み込む。

## Coverage threshold ではなく risk acceptance

NOT_VERIFIED は最後まで NOT_VERIFIED（fail-closed）。90% で出荷は可能だが、それは「閾値超え=OK」ではなく「**名指しされた未検証義務のリスクを受容する意思決定**」であり、責任の所在が残る。ツールは丸め上げをしない。release 判断は別レイヤー。

## Auxiliary test の分離（未決の設計事項）

仕様に直接紐付かないテスト（supporting / regression / characterization 等）は存在してよい。問題は**それを適合性の証拠として数えること**。dogfood 実測: 206 all green のうち正直に VO を claim できないテスト 22件（B-supporting 19 + C-regression 3）。現行契約は managed test に covers ≥1 を必須とするため、(a) 偽 covers で汚染 or (b) 未管理で test_traceability MISSING の二択になる張力が実在。auxiliary の第一級表現（covers 免除種別等）は**仕様判断待ち**（black-box topology 問題と同棚）。

## 実証（dogfood 2026-08-15）

同一 suite の二つの姿: C_code 視点 = 206 テスト all PASS。閉包視点 = PROVEN 14 / PARTIAL 67 / UNSUPPORTED 31 / 反証候補 42（検証中、既に CONFIRMED 1: VO-REGISTRY-05 = core が adapter 所有構文を解釈し `/** */` 宣言で Edit Test が全滅）。「テストが通っている」と「仕様が検証されている」の乖離の定量例。
