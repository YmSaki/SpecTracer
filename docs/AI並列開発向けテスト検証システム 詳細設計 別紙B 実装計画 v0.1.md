# AI並列開発向けテスト検証システム 詳細設計 別紙B 実装計画 v0.1

本別紙は非正規のprocess文書であり、実装順序、各マイルストーンの実装対象、完了条件を定める。
要件定義、基本仕様、詳細設計本冊、別紙A、別紙Cの正規なシステム契約を追加・変更・上書きしない。

本別紙のM1〜M9は、現在の正規契約を満たす製品能力を依存順に構築・再検証するための
capability milestoneである。既存実装を現在のarchitectureへ移行する作業順は
`SpecTracer 言語アダプタ分離リファクタリング計画 v0.2`のW0〜W8が管理し、
現在contractに対するstatusとevidenceは`実装スケジュール v0.1`の進捗台帳が管理する。
旧contractでの完了履歴は本別紙の完了条件を満たす現在の`DONE`を意味しない。

---

## 1. 方針

- マイルストーンは依存順に並ぶ。各マイルストーンの完了条件は、別紙C §18の適用可能な受入条件を含む全条件の達成とする。
- 受入条件は `tests/fixtures/` のサンプルプロジェクトに対する統合テストとして実装し、`cargo test` で再現可能にする。
- 本ツール自体のテストにも、可能な範囲で本ツールの思想（意図の明示、fail-closed）を適用する。

## 2. fixtureプロジェクト

`tests/fixtures/calc/` として、要件定義の例に沿った小規模プロジェクトを用意する。

```text
tests/fixtures/calc/
  Cargo.toml
  src/lib.rs        # 四則演算式の評価器
  tests/calc_test.rs
  .verify/          # SPEC / REQ / VO 登録済みの状態
  docs/spec.md      # SPEC として登録する仕様文書
```

テストコードには次を含める。

- 正しくアノテーションされた正常なテスト
- `assert!(true)` のみのテスト（DA-001）
- 対象を呼ばないテスト（DA-002）
- 結果を検証しないテスト（DA-003）
- 自己比較テスト（DA-004）
- アノテーションのない `#[test]`（W-SCAN-101、`test_traceability = MISSING`）
- 存在しないVOを参照するテスト（E-SCAN-003、`test_traceability = MISMATCH`）
- table-drivenテスト（`@vtest.case` 付き）
- 複数targetを宣言し、一方だけを実行するintegration Test
- Specification内の要求事項に対応active REQがない状態
- Test constructと非隣接のmetadata宣言だけを変更できるsynthetic Test

## 3. マイルストーン一覧

### M1 コアモデルとスキャナ

- 実装：`vtest-model`、`vtest-store`（読み込みのみ）、`vtest-adapter-api`、`vtest-adapter-rust`のdiscovery、`vtest-scan`の委譲・統合、`vtest init` / `vtest scan` / `vtest doctor`、診断出力（text / JSON）
- 完了条件：
  - fixtureのDiscovered Test、構造上完全なManaged Test Entity、ManagedTestLinkを区別して抽出できる。
  - 存在しないVOを参照するTest Entityを`ManagedTestLink::One`のまま保持し、E-SCAN-003と`MISMATCH`を導出できる。
  - 複数target Testの全TargetRefを宣言順に抽出し、重複を拒否できる。
  - `TestEntity`は中立な`execution`を持ち、Rust互換fieldは`rust-cargo` wire codecだけが入出力する。
  - E-SCAN-002〜010、W-SCAN-101がfixtureの該当箇所で検出される。
  - 未登録Testが存在する場合、`ManagedTestLink::Missing`から`test_traceability = MISSING`を導出できる。
  - Relation writerは`REL-<ULID>`を生成し、readerは整合したbare ULID互換recordをin-memoryで正規化する。同一payloadの重複・混在・不一致はE-SCAN-010になる。
  - `vtest scan --format json`の出力が別紙A §12.1の構造に従う。
  - E-ADAPTER-*による操作拒否は終了コード2、完了したscanのE-SCAN-*は1、errorなしは0になる。

### M2 レコード管理とVO実体化

- 実装：`vtest-store`（書き込み）、`spec` / `req` / `vo` 系コマンド、承認レコードと対象hash・上流依存closure束縛、`vo expand`
- 完了条件：
  - VOのadd → approve → editの順、および依存SPEC / REQ / parent VOの変更で承認が自動失効しdraftへ戻る。
  - dependency closureを完全・currentに解決できないapproveはE-APPROVAL-001となり、recordを生成しない。
  - `vo expand --dry-run`が`full-product`で直積の子VO一覧を出す。
  - SPEC登録後に文書を書き換えるとW-SCAN-104が出る。

### M3 決定論的監査

- 実装：`vtest-adapter-rust`の静的rule DA-001〜DA-006・W-DA-101、`vtest-audit`の委譲・集約、`vtest audit static`、監査レコード保存
- 完了条件：
  - fixtureの各NG Testが対応ruleでFAILになる。
  - 正常Testは全rule違反なしになる。
  - 他ファイルの関数を呼ぶTestがDA-002でFAILではなくUNKNOWNになる。
  - static Audit RecordがTest、全target、rule-setとrule影響config projectionへ束縛され、`assertion_macros`変更でSTALEになる。

### M4 テスト実行とEvidence

- 実装：`vtest-adapter-rust`のrunner起動・結果parse、`vtest-exec`の委譲・Evidence記録、`vtest run --fast`、鮮度判定（本冊 §11.2）
- 完了条件：
  - fixtureの全登録Testが実行され、TestごとにEvidenceが1件記録される。
  - Evidenceが`test_subject`と全宣言targetの`target_construct`内容hashを重複なく記録する。
  - Test constructと非隣接のcanonical metadataだけを変更してもEvidenceがSTALEになる。
  - 対象関数を書き換えた状態の検証で`evidence_validity`がSTALEになる。
  - revisionを特定できないEvidenceがSTALEになり、現在のPASSへ利用されない。
  - build failure fixtureでE-EXEC-001が出てEvidenceが記録されない。

### M5 意味監査プロトコル

- 実装：`audit bundle` / `audit submit`、bundle生成（4種別）、提出検証（E-AUDIT-001〜007）
- 完了条件：
  - test-semantic bundleに本冊 §8.2の全fieldが含まれる。
  - spec-coverage bundleが対象SPECと対応active REQ完全集合を束縛し、取り込み漏れをFAILにできる。
  - reasonsが空の提出がE-AUDIT-005で拒否される。
  - bundle生成時と異なる対象hashの提出がE-AUDIT-002で拒否される。
  - 受理された監査が、対象変更によってSTALEになる。
  - impl-consistencyの提出FAILが検証項目`impl_consistency = MISMATCH`へ一意に写像される。

### M6 集約とverify / report

- 実装：`vtest-verify`（12チェック項目評価、fail-closed集約、scope）、`vtest verify` / `vtest report`
- 完了条件：
  - 全12項目PASSのfixture状態で`verify`がOK・終了コード0になる。
  - 項目指定省略時はconfigに関係なく固定12項目を評価し、version 1の11項目`full_scope`から`test_traceability`を迂回できない。version 2の不完全な`full_scope`はE-CONFIG-001になる。
  - Specificationの要求事項に対応active REQがない、監査がない、または監査がINCOMPLETEの各状態で`spec_coverage`が非PASSになる。
  - Test metadata errorだけでは`vo_decomposition`が非PASSにならない。
  - 12項目のそれぞれを単独で非PASSにするとNG・終了コード1になる。
  - 未登録Testが1件でもあれば、他の11項目がPASSでも`test_traceability`によりNGになる。
  - `--items spec_coverage,vo_coverage`の限定scopeでOKが出ても、scope外項目はNOT_CHECKEDのまま表示される。
  - 出力treeが別紙A §12.2のbranch規則に従う。

### M7 Target Execution Verification

- 実装：`rust-cargo` coverage連携、Test単位計測、`vtest run`（既定mode）
- 完了条件：
  - 対象関数を実際に通るTestで`target_execution`がPASS（count ≥ 1）になる。
  - 対象を呼ばないがPASSするTestでFAILになる。
  - 複数target Testで一方がPASS、他方がFAILならTest単位集約がFAILになる。
  - 複数target TestでFAILがなく一方がUNKNOWNならTest単位集約がUNKNOWNになる。
  - coverage toolを利用できない環境でW-EXEC-101が出てNOT_CHECKEDになる。

### M8 Structured Test Operation

- 実装：coreのForm Schema読み込み・操作委譲、`vtest-adapter-rust`のStructuredTestAdapter・Form・検証器、`test create` / `test edit` / `test show` / `test list` / `test query`、別紙A §15のadapter contract
- 完了条件：
  - 誤ったsymbolを含む回答が候補付きE-OP-001で拒否される。
  - `test create`で生成されたTestがscanで正しく認識される。
  - `test edit`でcoversを変更しても他のTestのsource textが変化しない。
  - annotation再生成が冪等になる。
  - Form kindがrepository-globalに一意で、schema adapter・registry owner・Structured Test capabilityの一致からownerを解決する。重複・曖昧・未知ownerはfallbackせず拒否する。

### M9 MCPサーバ

- 実装：`vtest-mcp`（別紙A §13の全tool）、`vtest mcp`
- 完了条件：
  - 全toolがCLIと同一のJSON構造を返す。
  - 別紙A §13.3の利用flowがMCP経由で完了する。
  - 不正入力に対しcode / message / candidatesを持つerror objectが返る。

## 4. マイルストーン外

- GUI（要件定義 §28）
- 仕様書同士の矛盾検出（OOS-001）
- 修正方針の提案・自動修正（OOS-002）
- helper / fixture / 通常sourceの編集管理（OOS-003）
- 開発process管理（OOS-004）
- 本冊 §19の提供範囲外事項
