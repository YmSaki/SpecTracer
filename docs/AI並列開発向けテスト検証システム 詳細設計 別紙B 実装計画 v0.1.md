# AI並列開発向けテスト検証システム 詳細設計 別紙B 実装計画 v0.1

本冊 §0 の分冊構成に基づき、本別紙は §18 を収録する。

---

## 18. 実装マイルストーン

### 18.1 方針

- マイルストーンは依存順に並ぶ。各マイルストーンの完了条件は受入基準の全達成とする。
- 受入基準は `tests/fixtures/` のサンプルプロジェクトに対する統合テストとして実装し、`cargo test` で再現可能にする。
- 本ツール自体のテストにも、可能な範囲で本ツールの思想（意図の明示、fail-closed）を適用するが、自己適用（vtest で vtest を検証する）はマイルストーンに含めない。

### 18.2 fixture プロジェクト

`tests/fixtures/calc/` として、要件定義の例に沿った小規模プロジェクトを用意する。

```text
tests/fixtures/calc/
  Cargo.toml
  src/lib.rs        # 四則演算式の評価器（意図的な仕様乖離を1箇所含む版を
                    # フィーチャフラグまたは別 fixture で用意）
  tests/calc_test.rs
  .verify/          # SPEC / REQ / VO 登録済みの状態
  docs/spec.md      # SPEC として登録する仕様文書
```

テストコードには次を意図的に含める。

- 正しくアノテーションされた正常なテスト
- `assert!(true)` のみのテスト（DA-001）
- 対象を呼ばないテスト（DA-002）
- 結果を検証しないテスト（DA-003）
- 自己比較テスト（DA-004）
- アノテーションのない `#[test]`（W-SCAN-101）
- 存在しない VO を参照するテスト（E-SCAN-003）
- table-driven テスト（`@vtest.case` 付き）

### 18.3 マイルストーン一覧

#### M1 コアモデルとスキャナ

- 実装：`vtest-model`、`vtest-store`（読み込みのみ）、`vtest-scan`、`vtest init` / `vtest scan` / `vtest doctor`、診断出力（text / json）
- 受入基準：
  - fixture のスキャンで全 Test エンティティが抽出され、`filter` / `package` / `test_target` が正しい
  - E-SCAN-002〜010、W-SCAN-101 が fixture の該当箇所で検出される
  - `vtest scan --format json` の出力が §12.1 の構造に従う
  - 診断ありで終了コード 1、正常で 0

#### M2 レコード管理と VO 実体化

- 実装：`vtest-store`（書き込み）、`spec` / `req` / `vo` 系コマンド、承認レコードとハッシュ束縛、`vo expand`
- 受入基準：
  - VO の add → approve → edit の順で操作すると、承認が自動失効し draft へ戻る
  - `vo expand --dry-run` が `full-product` で直積の子 VO 一覧を出す
  - SPEC 登録後に文書を書き換えると W-SCAN-104 が出る

#### M3 決定論的監査

- 実装：`vtest-audit` の静的ルール DA-001〜DA-006、W-DA-101、`vtest audit static`、監査レコード保存
- 受入基準：
  - fixture の各 NG テストが対応ルールで FAIL になる
  - 正常テストは全ルール違反なし
  - 他ファイルの関数を呼ぶテストが DA-002 で FAIL ではなく UNKNOWN になる（保守性の確認）

#### M4 テスト実行と Evidence

- 実装：`vtest-exec`（cargo test 起動、結果パース、Evidence 記録）、`vtest run --fast`、鮮度判定（本冊 §11.2）
- 受入基準：
  - fixture の全登録テストが実行され、Test ごとに Evidence が1件記録される
  - 対象関数を書き換えた後の検証で `evidence_validity` が STALE になる
  - ビルド失敗 fixture で E-EXEC-001 が出て Evidence が記録されない

#### M5 意味監査プロトコル

- 実装：`audit bundle` / `audit submit`、バンドル生成（3種別）、提出検証（E-AUDIT-001〜006）
- 受入基準：
  - test-semantic バンドルに §8.2 の全フィールドが含まれる
  - reasons が空の提出が E-AUDIT-005 で拒否される
  - バンドル生成後に対象テストを書き換えた提出が E-AUDIT-002 で拒否される
  - 受理された監査が、対象の再変更後の検証で STALE として扱われる

#### M6 集約と verify / report

- 実装：`vtest-verify`（チェック項目評価、fail-closed 集約、scope）、`vtest verify` / `vtest report`
- 受入基準：
  - 全項目 PASS の fixture 状態で `verify` が OK・終了コード 0
  - 任意の1項目を非 PASS にすると NG・終了コード 1（fail-closed の全数確認：11項目それぞれについて1ケース）
  - `--items spec_coverage,vo_coverage` の限定 scope で OK が出ても、`report` 上で scope 外項目が NOT_CHECKED のまま表示される
  - 出力ツリーが §12.2（`vtest verify`）の形式に従う

#### M7 Target Execution Verification

- 実装：cargo-llvm-cov 連携、Test 単位計測、`vtest run`（既定モード）
- 受入基準：
  - 対象関数を実際に通るテストで `target_execution` が PASS（count ≥ 1）
  - 対象を呼ばないが PASS するテスト（mock 相当）で FAIL になる
  - cargo-llvm-cov 未導入環境で W-EXEC-101 が出て NOT_CHECKED になる

#### M8 Structured Test Operation

- 実装：Form Schema 読み込みと検証器、`test create` / `test edit` / `test show` / `test list` / `test query`、§15 の編集機構
- 受入基準：
  - 誤った symbol を含む回答が候補付き E-OP-001 で拒否される
  - `test create` で生成されたテストがスキャンで正しく認識される
  - `test edit` で covers を変更しても他の Test のソーステキストが変化しない（ハッシュ比較で確認）
  - アノテーションの再生成が冪等（同じ desired state の再適用で diff が出ない）

#### M9 MCP サーバ

- 実装：`vtest-mcp`（§13 の全ツール）、`vtest mcp`
- 受入基準：
  - 全ツールが CLI と同一の JSON 構造を返す
  - §13.3 の利用フローが MCP 経由で最後まで通る
  - 不正入力に対しエラーオブジェクト（code / message / candidates）が返る

### 18.4 マイルストーン外（明示的に実装しないもの）

- GUI（要件定義 §28）
- 仕様書同士の矛盾検出（OOS-001）
- 修正方針の提案・自動修正（OOS-002）
- helper / fixture / 通常ソースの編集管理（OOS-003）
- 開発プロセス管理（OOS-004）
- 本冊 §19 に列挙した将来課題
