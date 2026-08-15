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
  - static Audit RecordがTest、全target、rule-set、rule影響config projection、および判定時に参照したhelper等のsource fragment完全集合へ束縛され、`assertion_macros`または参照helperだけの変更でSTALEになる。
  - adapterが解析入力集合の完全性を保証できないruleはUNKNOWNとなり、PASSへ集約されない。

### M4 テスト実行とEvidence

- 実装：`vtest-adapter-rust`のrunner起動・結果parse、`vtest-exec`の委譲・Evidence記録、`vtest run --fast`、鮮度判定（本冊 §11.2）
- 完了条件：
  - fixtureの全登録Testが実行され、TestごとにEvidenceが1件記録される。
  - Evidenceが`test_subject`、全宣言targetの`target_construct`内容hash、および完全なExecution State subjectを重複なく記録する。
  - Test constructと非隣接のcanonical metadataだけを変更してもEvidenceがSTALEになる。
  - 対象関数を書き換えた状態の検証で`evidence_validity`がSTALEになる。
  - Test / 宣言targetを変更せずtarget外helperまたはlocal dependencyだけを変更してもEvidenceがSTALEになる。
  - HEAD revision不一致、Execution State subject欠落・不完全・不一致を現在のPASSへ利用しない。
  - revisionを特定できないEvidenceがSTALEになり、現在のPASSへ利用されない。
  - build failure fixtureでE-EXEC-001が出てEvidenceが記録されない。
  - 実行中にExecution State subjectが変化したfixtureでE-EXEC-004が出てEvidenceが記録されない。

### M5 意味監査プロトコル

- 実装：`audit bundle` / `audit submit`、bundle生成（4種別）、提出検証（E-AUDIT-001〜007）
- 完了条件：
  - test-semantic bundleに本冊 §8.2の全fieldが含まれる。
  - spec-coverage bundleが対象SPECと対応active REQ完全集合を束縛し、取り込み漏れをFAILにできる。
  - reasonsが空の提出がE-AUDIT-005で拒否される。
  - bundle生成時と異なる対象hashの提出がE-AUDIT-002で拒否される。
  - 受理された監査が、対象変更によってSTALEになる。
  - impl-consistency監査が対象VOの上流SPEC subject完全集合へ束縛され、Specificationだけの変更でもSTALEになる。
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

## 5. 到達 static/runtime 補完（PR #5）実装項目

PR #5（詳細設計 §3.6・§7.1〜§7.3・§10.1〜§10.2・§11.2〜§11.3、別紙A・別紙C）を満たすため、既存 M3 / M6 / M7 を該当 wave まで戻って再修正する。

### 5.1 M3 / W4 — per-target verdict と DA-003 pin（情報モデル）
- `vtest-adapter-rust` static_audit: DA-002 / DA-003 を宣言 target ごとの verdict（PASS/UNKNOWN/FAIL + basis）で返す（fold 前）。
- DA-003: 宣言 target への呼出が Test 本体に現れない場合（subprocess spawn 等）は UNKNOWN に pin（空虚 PASS/FAIL 禁止）。
- `vtest-adapter-api` / `vtest-model`: RuleObservationDraft が per-target verdict list を持つ。
- `vtest-audit`(core): per-target verdict を保存、規則単位 verdict は純静的 fold の派生値。
- `vtest-store`: 監査 record read/write に per-target verdict。malformed（欠落/重複/余剰/宣言target不一致/純静的fold不整合）= E-SCAN-010、有効 record から除外。
- static rule-set version を bump（既存 record が STALE 化）。
- 完了条件（別紙C §18.3.2）: per-target verdict の1:1・純静的fold整合・malformed=E-SCAN-010・旧 record STALE。

### 5.2 M7 / W5 — subprocess coverage 帰属（capability）
- `vtest-adapter-rust` coverage: 起動した subprocess / spawn thread の実行を宣言 target へ帰属（起動実行体も instrument 対象化＋子プロセス profile merge、LLVM_PROFILE_FILE 継承）。
- 帰属不能なら当該 target UNKNOWN、計測不能なら `target_execution = NOT_CHECKED`（能力欠如で捏造しない）。
- 完了条件（別紙C §18.3.6）: subprocess/thread 実行の target 別 PASS、帰属不能時 UNKNOWN / NOT_CHECKED。
- **★リスク**: spike で現状 0%。実現性を最初に検証。不能なら capability 無しとして fail-closed のまま出荷し別途課題化（DA-002 は UNKNOWN のまま）。

### 5.3 M6 / W6 — 評価時 join と scope（統合）
- `vtest-verify`: static_audit 項目を検証時に算出（target ごとに 静的到達 / runtime到達 / 未充足、全 target 充足かつ DA-001/003/004/005/006 全 PASS で PASS）。
- 選択規則再利用: per-target DA-002 は §3.6/§8.5 を per-target 適用、runtime は §11.2 選択 Evidence（`evidence_validity = PASS` 時）。§11.2 と独立に fallback しない。
- §11.3: 表示 scope と内部依存評価の分離（`--items static_audit` で Evidence 鮮度/target_execution を内部依存評価、scope 外 report value は NOT_CHECKED）。
- report: runtime 依存 static_audit は Evidence 根拠を引用。
- 完了条件（別紙C §18.3.2）: 評価時 join、A=static/B=runtime の multi-target、subprocess は PASS 不到達、限定 scope 依存評価、履歴不一致を生じない。

### 5.4 順序
- **Phase 1**: 5.1（M3/W4）→ 5.3（M6/W6）= per-target verdict + 評価時 join。in-process / cross-crate で検証可能、subprocess coverage 非依存。
- **Phase 2**: 5.2（M7/W5）= subprocess coverage。独立・高リスク、実現性検証を先行。
- 各 Phase で `cargo test --workspace` / clippy / fmt / `vtest doctor` の gate を通す。
- 完了後: SPEC-DOGFOOD-M3 sha256 再登録 → dogfood 再実行。
