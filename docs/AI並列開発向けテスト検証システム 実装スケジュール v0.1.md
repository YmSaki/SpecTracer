# AI並列開発向けテスト検証システム 実装スケジュール v0.1

## 0. 位置付け

本書は、要件定義・基本仕様・詳細設計本冊・別紙A・別紙Bを、実装オーケストレーターが実行可能な作業順序へ変換したプロジェクト計画である。製品仕様や受入基準を追加・変更しない。

仕様の優先順位は次のとおりとする。

1. 要件定義・要件分解 v0.1
2. 基本仕様 v0.1
3. 詳細設計 v0.1 本冊
4. 詳細設計 別紙A・別紙B

矛盾を発見した場合は実装で吸収せず、該当箇所、観測される影響、判定不能な点を記録して停止する。既知の `spec_coverage` および VO 状態の保存値と導出値に関する論点も、fail-closed を弱める根拠にしない。

本書による進捗管理は v0.1 製品の機能ではない。要件定義 OOS-004 の「開発プロセス全体の管理」を `vtest` に実装しない。

## 1. 実行前提

- 主担当は Sol オーケストレーターとし、設計判断、共有ファイル、統合、マイルストーン完了判定を所有する。
- Luna サブエージェントは `explorer`、`reviewer`、`tester` の専門担当として使う。
- Luna は通常速度で使う。起動時の `service_tier` は指定せず、環境の既定値を適用する。
- Luna は原則 `fork_turns: "none"` で起動し、依頼文に作業ディレクトリ、対象文書、対象ファイル、完了条件、禁止事項、実行コマンドをすべて含める。
- 最大3サブエージェントを想定するが、同一ファイル、同一レコードスキーマ、同一公開APIを複数エージェントへ同時に割り当てない。
- 各マイルストーンは別紙B §18の順序を守り、先行マイルストーンの受入基準が再現できるまで次へ進まない。
- 進行量は暦日ではなく、SolとLunaが実際に消費した入出力トークンで管理する。見積りは1名の統合担当と最大3名の独立担当を使える場合の初期予算であり、品質ゲートを省略して予算内へ収めない。

## 2. 常時守る実装制約

- 依存方向は `vtest-cli / vtest-mcp -> vtest-verify / vtest-exec / vtest-audit / vtest-scan -> vtest-store -> vtest-model` の一方向とする。
- 宣言、実装、実行事実の三層を分離し、導出可能な情報を正典として重複保存しない。
- SPEC / REQ / VO は1エンティティ1ファイルとする。Relation、Approval、Audit、Evidence は ULID による1レコード1ファイルとし、追記型または不変として扱う。
- 承認、監査、Evidence は対象の SHA-256 内容ハッシュへ束縛する。対象変更後に以前の PASS を引き継がない。
- すべての集約は fail-closed とし、PASS 以外を PASS に昇格させない。限定 scope の外側は `NOT_CHECKED` のまま保持する。
- 決定論的解析が違反を証明できない場合は `UNKNOWN` とし、推測で FAIL または PASS にしない。
- Structured Test Operation は desired state とし、1回の編集を1 Testの範囲に限定する。
- CLI と MCP は同じコア処理、JSON構造、検証エラーを共有し、非対話で動作させる。

## 3. トークン予算による全体スケジュール

単位は `kTok`（1,000 tokens）とし、入力と出力の合計を記録する。SolとLunaは単価が異なるため混ぜずに計上し、Luna欄は `explorer`、`tester`、`reviewer` の合計とする。金額換算が必要な場合は、実行時点のモデル単価を使って次式で求める。

```text
推定利用額 = Sol使用tokens × Sol単価 + Luna使用tokens × Luna単価
```

通常速度を前提とし、`fast` の追加倍率は含めない。モデル価格、キャッシュ割引、課金対象の定義は変わり得るため、実績値はCodexが表示する利用量を正典とする。

| 順序 | マイルストーン | 主成果 | Sol予算 | Luna予算 | 合計raw予算 | 完了ゲート |
|---:|---|---|---:|---:|---:|---|
| S0 | 実装開始準備 | workspace骨格、fixture計画、受入テスト台帳 | 60 kTok | 80 kTok | 140 kTok | 設計参照と作業境界が確定 |
| 1 | M1 コアモデルとスキャナ | model/store(read)/scan、init/scan/doctor | 180 kTok | 300 kTok | 480 kTok | M1受入テスト全PASS |
| 2 | M2 レコード管理とVO実体化 | store(write)、spec/req/vo、承認・expand | 120 kTok | 220 kTok | 340 kTok | M2受入テスト全PASS |
| 3 | M3 決定論的監査 | DA-001〜006、W-DA-101、静的監査 | 100 kTok | 180 kTok | 280 kTok | M3受入テスト全PASS |
| 4 | M4 テスト実行とEvidence | cargo test実行、Evidence、鮮度 | 140 kTok | 240 kTok | 380 kTok | M4受入テスト全PASS |
| 5 | M5 意味監査プロトコル | bundle/submit、提出検証 | 120 kTok | 200 kTok | 320 kTok | M5受入テスト全PASS |
| 6 | M6 集約とverify/report | 11項目評価、scope、fail-closed集約 | 180 kTok | 300 kTok | 480 kTok | 11項目の単独非PASS試験を全PASS |
| 7 | M7 Target Execution Verification | cargo-llvm-cov連携、既定run | 120 kTok | 200 kTok | 320 kTok | M7受入テスト全PASS |
| 8 | M8 Structured Test Operation | Form Schema、create/edit/show/list/query | 200 kTok | 360 kTok | 560 kTok | 境界保証・冪等性試験を全PASS |
| 9 | M9 MCPサーバ | 全MCPツール、CLI/MCP同値性 | 140 kTok | 240 kTok | 380 kTok | MCP利用フローと全体回帰を全PASS |
|  | **合計** |  | **1,360 kTok** | **2,320 kTok** | **3,680 kTok** | M1〜M9 DONE |

これは実装開始時のソフト予算である。ハード上限は各マイルストーンの125%（全体4,600 kTok）とし、次の規則で制御する。

- 50%消費時：受入テスト、実装、レビューの残件を照合し、不要な探索を打ち切る。
- 80%消費時：Solが現状を短く再要約し、未完了の受入基準だけにトークンを集中する。
- 100%消費時：自動的に次へ進まず、超過理由と残作業を記録して再見積りする。
- 125%消費時：`BLOCKED` とし、仕様矛盾、設計不足、実装難度、手戻りのどれが原因かを分離して報告する。
- 予算が余っても、次マイルストーンの公開APIや正典スキーマを先行実装しない。余剰は全体予備費へ戻す。
- トークンを使い切っても完了とはしない。別紙Bの受入基準と共通ゲートを満たした場合だけ `DONE` とする。

M1、M6、M8は後続への影響が大きい。これらの予算不足は仕様やテストを削って吸収せず、先に全体予備費または明示的な追加予算を割り当てる。

### 3.1 消費量の記録規則

- 各エージェントの終了時に、モデル、役割、対象マイルストーン、入出力tokens、成果、再作業の要否を記録する。
- Lunaの再依頼は新規消費として加算する。失敗した探索や却下した実装も実績から除外しない。
- 会話圧縮やキャッシュの有無にかかわらず、利用画面または実行環境が報告する課金対象tokensを優先する。取得できない場合だけ入出力tokensの表示値を使う。
- Solはマイルストーン開始時と50%、80%、100%到達時に台帳を更新する。
- 実消費が見積りを25%以上外れた場合、未着手マイルストーンの予算を一括で書き換えず、差異の原因が共通化できるかを確認してから改訂版を作る。
- 金額を比較するときはraw tokensを単純合算せず、SolとLunaそれぞれの実行時単価を掛ける。

## 4. S0 実装開始準備

### 作業

1. Cargo workspace と8クレートの空の骨格を作る。
2. `tests/fixtures/calc/` の正常系・異常系一覧を別紙B §18.2からテスト台帳へ転記する。
3. M1〜M9の受入基準を統合テスト名へ対応付ける。未実装テストは無視せず、段階的に有効化できる構造にする。
4. 共通の診断コード、終了コード、JSON envelope、内容ハッシュ型の所有クレートを決める。
5. CIで `cargo fmt`、`cargo test`、`cargo clippy` を実行できる最小構成を作る。

### 完了条件

- 全クレートが依存方向を破らずビルドできる。
- fixture要素と別紙B受入基準の対応表に欠落がない。
- 共有型の所有先が `vtest-model` を中心に一意である。
- 製品機能はまだPASS扱いにせず、未実装状態を明示できる。

## 5. M1 コアモデルとスキャナ

### 実装順

1. Solが `vtest-model` のID型、状態値、診断、エンティティ境界を確定する。
2. `vtest-store` に `.verify/` の読み込み、設定読み込み、重複・参照整合性検査を実装する。
3. `vtest-scan` にRust構文解析、アノテーション解析、Test抽出、シンボル解決、検証グラフ構築を実装する。
4. `vtest-cli` に `init`、`scan`、`doctor` と text/json 出力、終了コードを実装する。
5. `tests/fixtures/calc/` に別紙B §18.2の全テスト形態を用意し、M1受入テストを実装する。

### Lunaへ並列化できる作業

- `explorer`: Rust workspace、Testターゲット、filter/package/test_target抽出規則を設計箇所と対応付ける。読み取りのみ。
- `tester`: fixtureとE-SCAN-002〜010、W-SCAN-101、JSON構造、終了コードの統合テストを担当する。
- `reviewer`: スキャナがcovers/targetを外部レコードへ重複保存していないか、解析限界をPASSにしていないかを独立確認する。

### ゲート

- 別紙B M1の全受入基準を `cargo test` で再現する。
- 正常ケース0、診断あり1、使用方法エラー2、内部エラー3の区別を確認する。
- `filter`、`package`、`test_target` が実際の `cargo test` 起動に利用できる精度であることを確認する。

## 6. M2 レコード管理とVO実体化

### 実装順

1. `vtest-store` に原子的な書き込みと1エンティティ1ファイル規約を実装する。
2. SPEC / REQ / VO / Relation / Approval のスキーマと検証を実装する。
3. `spec`、`req`、`vo` 系CLIを実装する。
4. VO内容ハッシュと承認レコードの照合による自動失効を実装する。
5. `vo expand` の `full-product` と `dry-run` を実装する。
6. SPEC内容変更の W-SCAN-104 を実装する。

### Lunaへ並列化できる作業

- `tester`: add → approve → edit、SPEC改変、dry-run直積の受入テストを担当する。
- `reviewer`: 追記型レコード、Relation不変性、派生情報の非保存、ハッシュ正規化を確認する。

### ゲート

- 別紙B M2の全受入基準を満たす。
- 同一内容から同じ正規化ハッシュが得られ、改行・末尾空白規則が詳細設計 §1.3と一致する。
- 既存の追記型レコードを更新するコードパスが存在しない。

## 7. M3 決定論的監査

### 実装順

1. `vtest-audit` にルール共通インターフェースと根拠位置を実装する。
2. DA-001〜DA-006、W-DA-101を1ルールずつ追加する。
3. 解析限界を `UNKNOWN` にする境界ケースを先にテストする。
4. `vtest audit static` と監査レコード保存を実装する。

### Lunaへ並列化できる作業

- `tester`: ルールごとに違反、正常、解析限界の3系統を作成する。
- `reviewer`: 他ファイル呼び出しがDA-002でFAILにならずUNKNOWNになることを重点確認する。

### ゲート

- fixtureの各NGテストが対応ルールでFAILとなる。
- 正常テストに誤検知がない。
- 証明不能なケースを推測でFAILまたはPASSにしない。

## 8. M4 テスト実行とEvidence

### 実装順

1. Testから `cargo test` の起動単位を解決する。
2. stable toolchain出力の結果パーサと矛盾検出を実装する。
3. Git revision、dirty、Test/target内容ハッシュを実行直前に取得する。
4. EvidenceをULIDファイルへ新規追加し、raw logをcacheへ保存する。
5. `vtest run --fast` とEvidence鮮度判定を実装する。

### Lunaへ並列化できる作業

- `tester`: PASS、FAIL、ignored、結果行欠落、終了コード矛盾、ビルド失敗の実行fixtureを担当する。
- `reviewer`: ビルド失敗時にEvidenceが生成されないこと、未知revisionがvalidity PASSにならないことを確認する。

### ゲート

- 別紙B M4の全受入基準を満たす。
- Testまたはtarget変更後は以前のEvidenceが必ずSTALEになる。
- E-EXEC-001〜003の分岐とEvidence生成有無を再現する。

## 9. M5 意味監査プロトコル

### 実装順

1. 3種別のbundle生成と共有envelopeを実装する。
2. submitスキーマとE-AUDIT-001〜006を実装する。
3. reasonsとbasis参照の必須性を検証する。
4. bundle生成時ハッシュとsubmit時ハッシュを照合する。
5. 受理結果を追記型監査レコードとして保存し、有効性を導出する。

### Lunaへ並列化できる作業

- `tester`: 各E-AUDIT診断と対象変更による拒否・STALEを担当する。
- `reviewer`: bundleを正典扱いしていないこと、有効なFAILとPASSの混在時にFAILを採ることを確認する。

### ゲート

- 別紙B M5の全受入基準を満たす。
- 空理由、未知subject、不一致hash、重複・矛盾結果をfail-closedで拒否または集約する。

## 10. M6 集約とverify/report

### 実装順

1. 11チェック項目の評価地点と状態遷移を表駆動で実装する。
2. Test → VO → REQのfail-closed集約を実装する。
3. entity scopeとitem scopeを実装し、scope外を `NOT_CHECKED` とする。
4. `vtest verify` の簡易・ツリー・JSON出力を実装する。
5. `vtest report` に根拠レコードとEvidence参照を追加する。
6. 各11項目を1つずつ非PASSにした全数テストを実装する。

### Lunaへ並列化できる作業

- `tester`: 11項目単独非PASS、階層集約、限定scope、出力snapshotを担当する。
- `reviewer`: M6専用の独立fail-closedレビューを行い、誤ったPASS昇格を最優先で探す。
- `explorer`: 基本仕様 §4.2と詳細設計 §11.1の対応、既知の論点を一覧化する。矛盾時は実装判断をせず報告する。

### ゲート

- 別紙B M6の全受入基準を満たす。
- 11項目のどれか1つでも非PASSなら完全検証はNGとなる。
- 限定scopeのOKを完全検証OKとして表示しない。
- 基本仕様と詳細設計に判定差が見つかった場合は仕様確認まで停止する。

## 11. M7 Target Execution Verification

### 実装順

1. cargo-llvm-covの利用可否検出を実装する。
2. Test単位でcoverageを採取し、ロケータと実行countを対応付ける。
3. Evidenceの `target_execution` へ計測結果を記録する。
4. `vtest run` を既定の計測モード、`--fast` を非計測モードとして整理する。

### Lunaへ並列化できる作業

- `tester`: 対象通過、対象未通過、複数行関数、ツール未導入のケースを担当する。
- `reviewer`: ツール未導入をPASSにせずW-EXEC-101とNOT_CHECKEDにすることを確認する。

### ゲート

- 別紙B M7の全受入基準を満たす。
- Test単位計測が他Testの実行を混入させない。
- 計測不能時は完全検証がOKにならない。

## 12. M8 Structured Test Operation

### 実装順

1. Form Schemaの読み込み、型、必須、候補、条件分岐検証を実装する。
2. `test show/list/query` を先に実装し、現在状態の取得形式を固定する。
3. desired stateからアノテーションブロックを決定的に再生成する。
4. `test create` を実装する。
5. `test edit` に拡張range、適用前境界検査、適用後の他Testハッシュ不変検査を実装する。
6. 候補付きE-OP-001、冪等性、1 Test境界の受入テストを実装する。

### Lunaへ並列化できる作業

- `tester`: Form Schema検証、create再scan、edit冪等性、他Test不変のハッシュ試験を担当する。
- `reviewer`: 通常ソース、helper、fixtureを編集できる経路がないことと、部分的失敗時の原状維持を確認する。
- `explorer`: syn spanと拡張rangeの境界ケースを収集する。読み取りと試験提案のみ。

### ゲート

- 別紙B M8の全受入基準を満たす。
- 同じdesired stateの再適用でdiffが出ない。
- 対象外Testまたは通常実装のハッシュが変化した操作は失敗し、部分更新を残さない。

## 13. M9 MCPサーバ

### 実装順

1. `vtest-mcp` をCLIと同じapplication/core APIへ接続する。
2. 別紙A §13.2の全ツールを実装する。
3. 共通JSON構造、入力検証、code/message/candidatesエラーをCLIと共有する。
4. 長時間稼働時のmtime検出と再スキャンを実装する。
5. 別紙A §13.3の利用フローをstdio MCP統合テストにする。
6. プロジェクトのvtest MCP設定を有効化するのは、全受入試験完了後に限る。

### Lunaへ並列化できる作業

- `tester`: 全ツールのCLI/MCP同値性、エラー形、連続呼び出し時の再scanを担当する。
- `reviewer`: MCP書き込みツールがStructured Operationだけを公開し、独自判定や独自スキーマを持たないことを確認する。

### ゲート

- 別紙B M9の全受入基準を満たす。
- CLIとMCPの同一入力に対し、transport差を除く同一JSON結果を返す。
- M1〜M9の全回帰試験を通過する。

## 14. マイルストーン共通の完了手順

各マイルストーンの終了時に、オーケストレーターは次の順で処理する。

1. 別紙B §18の該当受入基準とテスト名を1対1で照合する。
2. `$verify-change` で変更の種類に応じた狭い検証と全体検証を行う。
3. `$architecture-check` で依存方向、正典、ハッシュ束縛、fail-closed、編集境界、CLI/MCP共有性を確認する。
4. Luna `reviewer` に独立レビューを依頼し、Solが指摘の確実性と仕様根拠を確認する。
5. `$release-check` で該当マイルストーンの受入基準が再現可能か確認する。
6. 次の共通コマンドを実行する。

```powershell
cargo fmt --all -- --check
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo run --quiet -p vtest-cli -- doctor
```

7. 結果を進捗台帳へ記録する。利用不能な検証は `NOT_CHECKED`、未実行は `NOT_EXECUTED` とし、マイルストーンを完了扱いにしない。

## 15. オーケストレーター用進捗台帳

次の表を実装中に更新する。初期状態はすべて未着手とする。

| Milestone | Status | Sol used / budget | Luna used / budget | Acceptance evidence | Architecture review | Independent review | Blocker |
|---|---|---:|---:|---|---|---|---|
| S0 | DONE | N/R / 60 kTok | N/R / 80 kTok | workspace test、init/layout、`tests/ACCEPTANCE.md` | PASS | PASS（M1 review で再確認） | — |
| M1 | DONE | N/R / 180 kTok | N/R / 300 kTok | `m1_acceptance` 5/5、`release_check --milestone M1` READY | PASS | PASS（独立 read-only review 2系統） | — |
| M2 | NOT_STARTED | 0 / 120 kTok | 0 / 220 kTok | — | — | — | — |
| M3 | NOT_STARTED | 0 / 100 kTok | 0 / 180 kTok | — | — | — | — |
| M4 | NOT_STARTED | 0 / 140 kTok | 0 / 240 kTok | — | — | — | — |
| M5 | NOT_STARTED | 0 / 120 kTok | 0 / 200 kTok | — | — | — | — |
| M6 | NOT_STARTED | 0 / 180 kTok | 0 / 300 kTok | — | — | — | — |
| M7 | NOT_STARTED | 0 / 120 kTok | 0 / 200 kTok | — | — | — | — |
| M8 | NOT_STARTED | 0 / 200 kTok | 0 / 360 kTok | — | — | — | — |
| M9 | NOT_STARTED | 0 / 140 kTok | 0 / 240 kTok | — | — | — | — |

Statusは `NOT_STARTED`、`IN_PROGRESS`、`BLOCKED`、`DONE` のいずれかとする。`DONE` は受入基準、共通検証、architecture-check、独立レビュー、release-checkがすべて完了した場合にだけ設定する。

`N/R` は、実行環境がマイルストーン別 token 使用量を公開しておらず、推測値を記録しないことを示す。M1 の独立レビューは Luna reviewer の登録モデルが利用不能だったため、同一コードを編集しない独立 agent 2系統で代替した。終了コードについては、別紙B §18.3 の「診断あり」と、基本仕様 §11・詳細設計 §5.4・別紙A §12.2 の「warning は結果を変えず、error 診断で終了コード1」が字義上衝突する。実装と受入テストは後者に従い、`tests/ACCEPTANCE.md` に review item として記録した。

## 16. Luna依頼テンプレート

```text
役割: <explorer | tester | reviewer>
モデル: gpt-5.6-luna
作業ディレクトリ: C:\programing\SpecTracer
対象マイルストーン: <M1〜M9>
対象ファイル: <排他的に所有するファイルまたは読み取り対象>
参照仕様: <文書名と節番号>
作業: <単一の独立した成果>
完了条件: <別紙B受入基準または検証可能な条件>
禁止事項:
- 対象外ファイルを編集しない
- PASS以外をPASSへ昇格しない
- 派生cacheを正典として編集しない
- 仕様矛盾を独断で解消しない
検証コマンド: <対象に絞ったcargo test等>
返却内容: 変更ファイル、実行結果、未確認事項、仕様上の懸念
```

オーケストレーターは `fork_turns: "none"` で起動するため、会話履歴を前提にした「前と同じ」「適宜」「残り全部」のような依頼をしない。共有型や公開APIの変更が必要になった場合、Lunaは編集を止めてSolへ返し、Solが統合順序を決める。

## 17. 停止条件

次のいずれかが発生した場合、先へ進まず `BLOCKED` として根拠を記録する。

- 上位仕様と詳細設計で観測可能な挙動が矛盾する。
- 受入基準を再現するテストが作れない、または結果が非決定的である。
- 依存方向を逆転しないと実装できない。
- 承認、監査、Evidenceのハッシュ束縛を維持できない。
- 非PASSをPASSへ昇格しないと完全検証OKにならない。
- Structured Editで1 Test境界または原子的更新を保証できない。
- CLIとMCPでコア処理またはJSON契約が分岐する。
- GUI、自動修正方針、仕様書間監査、通常ソース編集管理、開発プロセス管理を製品へ追加する必要が生じる。

## 18. 最終完了条件

プロジェクト実装完了は次をすべて満たした状態とする。

- M1〜M9がすべて `DONE` である。
- 別紙B §18.2のfixtureと§18.3の全受入基準が `cargo test` で再現できる。
- workspace全体のfmt、test、clippy、doctorが成功する。
- 完全検証の11項目すべてについて、単独非PASSが総合NGになることを確認できる。
- staleな承認、監査、EvidenceがPASSに使われない。
- CLI/MCP同値性とStructured Editの1 Test境界が検証済みである。
- v0.1のスコープ外機能を含まない。
- 最終 `$release-check` がREADYを報告する。
