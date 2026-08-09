# AI並列開発向けテスト検証システム 実装スケジュール v0.1

## 0. 位置付け

本書は、要件定義・基本仕様・詳細設計本冊・別紙A・別紙Cの正規契約と、非正規の別紙B実装計画に対するcurrent status、evidence、blockerを管理するプロジェクト台帳である。製品仕様や受入基準を追加・変更しない。

仕様の優先順位は次のとおりとする。

1. 要件定義・要件分解 v0.1
2. 基本仕様 v0.1
3. 詳細設計 v0.1 本冊
4. 詳細設計 別紙A・別紙C

別紙Bは非正規の実装計画であり、この優先順位へ正規仕様として加えない。

別紙BのM1〜M9は現在の製品能力の依存順、`SpecTracer 言語アダプタ分離リファクタリング計画 v0.2`のW0〜W8は既存実装を現在contractへ移行するactive work sequenceを管理する。本書は両者とは別に、現在contractに対する検証状態と旧baseline evidenceを区別して記録する。active migrationではW0〜W8を実行順として用い、M1〜M9を競合する第二の移行順序として扱わない。

矛盾を発見した場合は実装で吸収せず、該当箇所、観測される影響、判定不能な点を記録して停止する。

本書による進捗管理は v0.1 製品の機能ではない。要件定義 OOS-004 の「開発プロセス全体の管理」を `vtest` に実装しない。

## 1. 実行前提

- 主担当は Sol オーケストレーターとし、設計判断、共有ファイル、統合、マイルストーン完了判定を所有する。
- Luna サブエージェントは `explorer`、`reviewer`、`tester` の専門担当として使う。
- Luna は通常速度で使う。起動時の `service_tier` は指定せず、環境の既定値を適用する。
- Luna は原則 `fork_turns: "none"` で起動し、依頼文に作業ディレクトリ、対象文書、対象ファイル、完了条件、禁止事項、実行コマンドをすべて含める。
- 最大3サブエージェントを想定するが、同一ファイル、同一レコードスキーマ、同一公開APIを複数エージェントへ同時に割り当てない。
- 新規構築でM1〜M9を実行する場合は別紙Bのcapability依存順を守る。言語adapter分離のactive migrationはrefactor plan W0〜W8の順を守り、各waveが参照するM1〜M9 acceptance groupを現在contractで再検証する。
- 進行量は暦日ではなく、SolとLunaが実際に消費した入出力トークンで管理する。見積りは1名の統合担当と最大3名の独立担当を使える場合の初期予算であり、品質ゲートを省略して予算内へ収めない。

## 2. 常時守る実装制約

- 依存方向は `vtest-cli / vtest-mcp -> vtest-verify / vtest-exec / vtest-audit / vtest-scan -> vtest-store -> vtest-model` とし、言語固有capabilityは `vtest-scan / vtest-audit / vtest-exec -> vtest-adapter-rust -> vtest-adapter-api -> vtest-model` の方向に限定する。
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
| 6 | M6 集約とverify/report | 12項目評価、scope、fail-closed集約 | 180 kTok | 300 kTok | 480 kTok | 12項目の単独非PASS試験を全PASS |
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
- トークンを使い切っても完了とはしない。別紙Bのマイルストーン条件、別紙Cの受入条件、共通ゲートを満たした場合だけ `DONE` とする。

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

1. Cargo workspace と10クレート（core 8 crate＋`vtest-adapter-api`＋`vtest-adapter-rust`）の空の骨格を作る。
2. `tests/fixtures/calc/` の正常系・異常系一覧を別紙B §2と別紙C §18.2からテスト台帳へ転記する。
3. M1〜M9の受入基準を統合テスト名へ対応付ける。未実装テストは無視せず、段階的に有効化できる構造にする。
4. 共通の診断コード、終了コード、JSON envelope、内容ハッシュ型の所有クレートを決める。
5. CIで `cargo fmt`、`cargo test`、`cargo clippy` を実行できる最小構成を作る。

### 完了条件

- 全クレートが依存方向を破らずビルドできる。
- fixture要素と別紙C受入条件の対応表に欠落がない。
- 共有型の所有先が `vtest-model` を中心に一意である。
- 製品機能はまだPASS扱いにせず、未実装状態を明示できる。

## 5. M1 コアモデルとスキャナ

### 実装順

1. Solが `vtest-model` のID型、状態値、診断、hash未計算discovery DTOとdomain entity境界を確定する。
2. `vtest-adapter-api` にneutral discovery / capability contractを実装し、`vtest-adapter-rust`にRust構文解析、annotation解析、Test抽出、symbol解決を実装する。
3. `vtest-store` に `.verify/` の読み込み、設定読み込み、重複・参照整合性検査を実装する。
4. `vtest-scan` にadapter委譲、DTO検証、core hash計算、domain materialization、決定論的統合、検証グラフ構築を実装する。Rust parserを直接所有しない。
5. `vtest-cli` に `init`、`scan`、`doctor` と text/json 出力、終了コードを実装する。
6. `tests/fixtures/calc/` とsynthetic fixtureに別紙B §2と別紙C §18.2の全Test形態を用意し、M1受入テストを実装する。

### Lunaへ並列化できる作業

- `explorer`: Rust workspace、Test target集合、`rust-cargo` wire互換fieldのencode規則を設計箇所と対応付ける。読み取りのみ。
- `tester`: fixtureとE-SCAN-002〜010、W-SCAN-101、JSON構造、終了コードの統合テストを担当する。
- `reviewer`: スキャナがcovers/targetを外部レコードへ重複保存していないか、解析限界をPASSにしていないかを独立確認する。

### ゲート

- 別紙B M1の全完了条件を満たす。
- 正常ケース0、完了scanのE-SCAN-*は1、E-ADAPTER-*等の操作拒否2、内部エラー3の区別を確認する。
- `rust-cargo` wire codecが出力する互換field `filter`、`package`、`test_target`が`execution`と一致し、Cargo起動座標へ損失なく変換できることを確認する。

## 6. M2 レコード管理とVO実体化

### 実装順

1. `vtest-store` に原子的な書き込みと1エンティティ1ファイル規約を実装する。
2. SPEC / REQ / VO / Relation / Approval のスキーマと検証を実装する。
3. `spec`、`req`、`vo` 系CLIを実装する。
4. VO内容ハッシュと上流依存closureを承認レコードへ束縛し、対象VO、parent VO、REQ、SPEC変更による自動失効を実装する。
5. dependency closureを完全・currentに解決できないapproveをE-APPROVAL-001で拒否する。
6. `vo expand` の `full-product` と `dry-run` を実装する。
7. SPEC内容変更の W-SCAN-104 を実装する。

### Lunaへ並列化できる作業

- `tester`: add → approve → edit、SPEC改変、dry-run直積の受入テストを担当する。
- `reviewer`: 追記型レコード、Relation不変性、派生情報の非保存、ハッシュ正規化を確認する。

### ゲート

- 別紙B M2の全完了条件を満たす。
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
3. Git revision、dirty、Test subject hash、全宣言targetの参照と内容hashを実行直前に取得する。
4. EvidenceをULIDファイルへ新規追加し、raw logをcacheへ保存する。
5. `vtest run --fast` とEvidence鮮度判定を実装する。

### Lunaへ並列化できる作業

- `tester`: PASS、FAIL、ignored、結果行欠落、終了コード矛盾、ビルド失敗の実行fixtureを担当する。
- `reviewer`: ビルド失敗時にEvidenceが生成されないこと、未知revisionがvalidity PASSにならないことを確認する。

### ゲート

- 別紙B M4の全完了条件を満たす。
- canonical Test metadata、execution、Test constructまたはtarget変更後は以前のEvidenceが必ずSTALEになる。
- 複数target Evidenceのentry欠落、重複、余剰を有効なPASSとして扱わない。
- E-EXEC-001〜003の分岐とEvidence生成有無を再現する。

## 9. M5 意味監査プロトコル

### 実装順

1. `spec-coverage`を含む4種別のbundle生成と共有envelopeを実装する。
2. submitスキーマとE-AUDIT-001〜007を実装する。
3. reasonsとbasis参照の必須性を検証する。
4. bundle生成時ハッシュとsubmit時ハッシュを照合する。
5. 受理結果を追記型監査レコードとして保存し、有効性を導出する。

### Lunaへ並列化できる作業

- `tester`: 各E-AUDIT診断と対象変更による拒否・STALEを担当する。
- `reviewer`: bundleを正典扱いしていないこと、有効なFAILとPASSの混在時にFAILを採ることを確認する。

### ゲート

- 別紙B M5の全完了条件を満たす。
- 空理由、未知subject、不一致hash、重複・矛盾結果をfail-closedで拒否または集約する。

## 10. M6 集約とverify/report

### 実装順

1. 12チェック項目の評価地点と状態遷移を表駆動で実装する。
2. SPEC → REQ → VO → Testの評価treeと、Test → VO → REQ → SPECのfail-closed集約を実装する。
3. entity scopeとitem scopeを実装し、scope外を `NOT_CHECKED` とする。
4. `vtest verify` の簡易・ツリー・JSON出力を実装する。
5. `vtest report` に根拠レコードとEvidence参照を追加する。
6. 各12項目を1つずつ非PASSにした全数テストを実装する。

### Lunaへ並列化できる作業

- `tester`: 12項目単独非PASS、未登録Test、階層集約、限定scope、出力snapshotを担当する。
- `reviewer`: M6専用の独立fail-closedレビューを行い、誤ったPASS昇格を最優先で探す。
- `explorer`: 基本仕様 §4.2と詳細設計 §11.1の対応を一覧化する。矛盾時は実装判断をせず報告する。

### ゲート

- 別紙B M6の全完了条件を満たす。
- 12項目のどれか1つでも非PASSなら完全検証はNGとなる。
- active REQの存在だけで`spec_coverage`をPASSにせず、Test / adapter errorを`vo_decomposition`へ漏らさない。
- 限定scopeのOKを完全検証OKとして表示しない。
- 基本仕様と詳細設計に判定差が見つかった場合は仕様確認まで停止する。

## 11. M7 Target Execution Verification

### 実装順

1. cargo-llvm-covの利用可否検出を実装する。
2. Test単位でcoverageを採取し、全宣言targetのロケータと実行countを個別に対応付ける。
3. Evidenceの `target_execution.targets` へtarget別結果を記録し、fail-closed集約値を計算する。
4. `vtest run` を既定の計測モード、`--fast` を非計測モードとして整理する。

### Lunaへ並列化できる作業

- `tester`: 対象通過、対象未通過、複数targetの一部未通過・解析不能、複数行関数、ツール未導入のケースを担当する。
- `reviewer`: ツール未導入をPASSにせずW-EXEC-101とNOT_CHECKEDにすることを確認する。

### ゲート

- 別紙B M7の全完了条件を満たす。
- Test単位計測が他Testの実行を混入させない。
- 複数targetは各targetを独立に判定し、全targetがPASSの場合だけTest単位PASSになる。
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

- 別紙B M8の全完了条件を満たす。
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

- 別紙B M9の全完了条件を満たす。
- CLIとMCPの同一入力に対し、transport差を除く同一JSON結果を返す。
- M1〜M9の全回帰試験を通過する。

## 14. マイルストーン共通の完了手順

各マイルストーンの終了時に、オーケストレーターは次の順で処理する。

1. 別紙Bの該当マイルストーン条件および別紙C §18の受入条件とテスト名を1対1で照合する。
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

次の表は現在の正規contractに対するstatusと、旧contractで得たbaseline evidenceを分離して記録する。

| Milestone | Current status | Historical baseline evidence | Current-contract evidence | Blocker |
|---|---|---|---|---|
| S0 | REVALIDATION_REQUIRED | 8-crate workspace、init/layout、旧`tests/ACCEPTANCE.md` | NOT_CHECKED | W0のOwner承認・merge後、10-crate構成と新受入台帳を再検証する |
| M1 | REVALIDATION_REQUIRED | 旧`m1_acceptance` 5/5、旧release-check READY | NOT_CHECKED | hash未計算DTO、core hash ownership、adapter分離、SRC ID一意性を未検証 |
| M2 | REVALIDATION_REQUIRED | 旧`m2_acceptance` 12/12、旧release-check READY | NOT_CHECKED | Approval dependency closure、REL-ID、derived VO statusを未検証 |
| M3 | REVALIDATION_REQUIRED | 旧`m3_acceptance` PASS、旧dogfood static audit PASS | NOT_CHECKED | Test subject hashとadapter-owned static auditで再検証が必要 |
| M4 | REVALIDATION_REQUIRED | 旧`m4_acceptance` 6/6、旧Evidence multi-target baseline | NOT_CHECKED | `test_subject` Evidenceと非隣接metadata freshnessを未検証 |
| M5 | REVALIDATION_REQUIRED | 旧`m5_acceptance` 4/4、3 bundle種別baseline | NOT_CHECKED | `spec-coverage`を含む4種別とE-AUDIT-007を未検証 |
| M6 | REVALIDATION_REQUIRED | 旧11項目`m6_acceptance` 6/6 | NOT_CHECKED | 12項目、SPEC root集約、spec_coverage、独立vo_decompositionを未検証 |
| M7 | REVALIDATION_REQUIRED | 旧`m7_acceptance` 3/3、target count baseline | NOT_CHECKED | adapter-owned coverageとTest subject Evidenceで再検証が必要 |
| M8 | REVALIDATION_REQUIRED | 旧`m8_acceptance` 4/4、編集境界・冪等性baseline | NOT_CHECKED | adapter-owned Form validatorとneutral DTOで再検証が必要 |
| M9 | REVALIDATION_REQUIRED | 旧`m9_acceptance` 9/9、22 tool / CLI parity baseline | NOT_CHECKED | current registry・schema・JSON envelopeで全MCP parityを再検証する |

Statusは `NOT_STARTED`、`IN_PROGRESS`、`BLOCKED`、`REVALIDATION_REQUIRED`、`DONE` のいずれかとする。正規contractまたは受入条件が変化したmilestoneは、旧evidenceがPASSでも`REVALIDATION_REQUIRED`へ戻す。`DONE` は現在contractの受入基準、共通検証、architecture-check、独立レビュー、release-checkがすべて完了した場合にだけ設定する。

`N/R` は、実行環境がマイルストーン別 token 使用量を公開しておらず、推測値を記録しないことを示す。以下の記録はすべてhistorical baselineであり、現在contractのPASSまたはDONEを表さない。

旧M2 baselineはbare / prefixed Relation IDの両方とVO単体hashだけのApprovalを扱った。現在contractは`REL-<ULID>`、derived VO status、Approval dependency closureを要求するため、このevidenceはcurrent M2へ流用しない。

自己適用ではリポジトリ直下の `.verify/` で `vtest doctor` を実行し、exit 0 と canonical 再読込を確認した。M3 では `TEST-DOGFOOD-M3-TARGET-RULES` を VO に登録し、実プロセス `audit static` と scoped `verify --items static_audit` を実行して six-rule PASS を確認した。M4 では同じTestを `run --all --fast` で実行し、Evidenceを再読込して `static_audit`、`test_execution`、`runtime_result`、`evidence_validity` のPASSを確認した（fastの `target_execution` はNOT_CHECKED）。M5では実プロセスbundle/submitを独立fixtureで実行し、typed AuditRecordを再読込して対象改変時のSTALEを確認した。再試行で生成された古い immutable AuditRecord は verifier が STALE として除外することも確認済みである。
旧M6 baselineでは、REQ/VO/Testの各entity scopeとitem scope、全11項目の単独非PASS、leaf VOの未カバー、部分実行Evidence、根拠付きtree、11項目PASS・exit 0を確認した。このevidenceは12項目、SPEC root、`spec_coverage`意味監査、独立した`vo_decomposition`、`test_traceability`を含む現在contractの証拠ではない。
M7では、設定を `llvm-cov` にした独立fixtureで、対象関数を通るTestの `target_execution: PASS`（count >= 1）、通らないがコンパイル時に保持された対象の `FAIL`（count = 0）、および `cargo llvm-cov --version` だけを不成立にしたfallbackの `W-EXEC-101` / `NOT_CHECKED` を実プロセスで確認した。
M8では、Structured Test Operationの実プロセスfixtureで、候補付き `E-OP-001`、dry-runを含む `test create` と再スキャン、`show/list/query`、対象Testだけの `covers` 編集、他Testのcontent hash不変、同一desired stateのbyte-idempotenceを確認した。
M9では、`vtest mcp` のstdio JSON-RPC transport、initialize/tools/list、22 toolすべてのschemaとCLI envelope parity、REQ/VO/Test/Form/Audit/Run/Verify/Reportの§13.3参照経路、候補付き入力エラー、notification無応答、mtime refreshを実プロセスで確認した。MCPの判定は既存CLIを再利用するため、第二の集約ロジックは存在しない。独立Luna reviewerも修正後のupsert、notification、freshness、非PASS保持を再確認した。`rmcp` SDKの文字どおりのwire interoperabilityだけは、最小通常利用の完了条件とは分離したNOT_CHECKEDの実装詳細として残す。

## 16. Luna依頼テンプレート

```text
役割: <explorer | tester | reviewer>
モデル: gpt-5.6-luna
作業ディレクトリ: C:\programing\SpecTracer
対象マイルストーン: <M1〜M9>
対象ファイル: <排他的に所有するファイルまたは読み取り対象>
参照仕様: <文書名と節番号>
作業: <単一の独立した成果>
完了条件: <別紙Bマイルストーン条件、別紙C受入条件、または検証可能な条件>
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
- 別紙B §2のfixtureと別紙C §18.3の全受入条件が `cargo test` で再現できる。
- workspace全体のfmt、test、clippy、doctorが成功する。
- 完全検証の12項目すべてについて、単独非PASSが総合NGになることを確認できる。
- staleな承認、監査、EvidenceがPASSに使われない。
- CLI/MCP同値性とStructured Editの1 Test境界が検証済みである。
- v0.1のスコープ外機能を含まない。
- 最終 `$release-check` がREADYを報告する。
