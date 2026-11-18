# AI並列開発向けテスト検証システム 詳細設計 別紙B 実装計画 v0.1

本別紙は非正規のprocess文書であり、実装順序、各マイルストーンの実装対象、完了条件を定める。
要件定義、基本仕様、詳細設計本冊、別紙A、別紙Cの正規なシステム契約を追加・変更・上書きしない。
状態語彙・検査・文書モデル・コマンド・診断コードの規範は上流と本冊・別紙A/C が所有し、本別紙はそれを実装計画へ写すに留める。

本別紙のM1〜M9は、現在の正規契約を満たす製品能力を依存順に構築・再検証するための
capability milestoneである。既存実装を現在のarchitectureへ移行する作業順は
`SpecTracer 言語アダプタ分離リファクタリング計画 v0.2`のW0〜W8が管理し、
現在contractに対するstatusとevidenceは`実装スケジュール v0.1`の進捗台帳が管理する。
旧contractでの完了履歴は本別紙の完了条件を満たす現在の`DONE`を意味しない。

---

## 1. 方針

- マイルストーンは依存順に並ぶ。各マイルストーンの完了条件は、別紙C §18の適用可能な受入条件を含む全条件の達成とする。完了条件と受入項目名は別紙C §18の確定した4検査（`chain_integrity` / `orphan_detection` / `target_binding` / `oracle_presence`）・5状態（`PASS` / `FAIL` / `MISMATCH` / `NO_EVIDENCE` / `UNKNOWN`）・4診断ラベル（`MISSING` / `NOT_EXECUTED` / `NOT_CHECKED` / `STALE`）に整合させる。
- 受入条件は `tests/fixtures/` のサンプルプロジェクトに対する統合テストとして実装し、`cargo test` で再現可能にする。
- 本ツール自体のテストにも、可能な範囲で本ツールの思想（意図の明示、fail-closed）を適用する。

## 2. fixtureプロジェクト

`tests/fixtures/calc/` として、要件定義の例に沿った小規模プロジェクトを用意する。
権威的なfixture要件は別紙C §18.2が定める。本節はM1〜M9の依存順を駆動する最小の代表集合を示す。

```text
tests/fixtures/calc/
  Cargo.toml
  src/lib.rs        # 四則演算式の評価器
  tests/calc_test.rs
  .verify/          # 総称 document(DOC-) / VO 登録済みの状態
  docs/spec.md      # document(DOC-) として登録する上流文書
```

`.verify/` には上流文書を総称 `document`（`DOC-`）レコードとして登録し、VO は `derives_from` で
1件以上の document へ直結する。SPEC / REQ 実体層は持たない（基本仕様 §3.1・§3.2、本冊 §3.1）。

テストコードおよび登録状態には次を含める。

- 正しくアノテーションされた正常なテスト
- `assert!(true)` のみのテスト（DA-001）
- 対象を呼ばないテスト（DA-002）
- 結果を検証しないテスト（DA-003）
- 自己比較テスト（DA-004）
- アノテーションのない `#[test]`（W-SCAN-101、`chain_integrity = MISMATCH`、診断 `MISSING`）
- 存在しないVOを参照するテスト（E-SCAN-003、`chain_integrity = MISMATCH`）
- table-drivenテスト（`@vtest.case` 付き）
- 複数targetを宣言し、一方だけを実行するintegration Test
- covers する Test が 1 件も存在しない leaf VO（`chain_integrity = MISMATCH`、診断 `MISSING`）
- Test constructと非隣接のmetadata宣言だけを変更できるsynthetic Test

document 鎖・孤児・鮮度の登録状態として次を含める。

- `doc.roots` に列挙された根 document
- `derives_from` が空かつ根に列挙されない孤児 document（E-SCAN-016、`orphan_detection = MISMATCH`）
- `derives_from` の参照先が存在しない document / VO（E-SCAN-012、`chain_integrity = MISMATCH`）
- `content_hash` と実ファイルが一致しない document（W-SCAN-104、`chain_integrity = MISMATCH`、診断 `STALE`）

検証状態・診断ラベル・ゲートの網羅入力として次を含める。

- 5状態（`PASS` / `FAIL` / `MISMATCH` / `NO_EVIDENCE` / `UNKNOWN`）それぞれを生じる入力
- 4診断ラベル（`MISSING` / `NOT_EXECUTED` / `NOT_CHECKED` / `STALE`）それぞれを生じる入力
- フェーズゲート定義（`config.yaml` の `gates` と `approval_roles`）と、条件充足・不足の両方を表す承認状態

## 3. マイルストーン一覧

### M1 コアモデルとスキャナ

- 実装：`vtest-model`、`vtest-store`（読み込みのみ）、`vtest-adapter-api`、`vtest-adapter-rust`のdiscovery、`vtest-scan`の委譲・統合、`vtest init` / `vtest scan` / `vtest doctor`、`chain_integrity`（文書鎖・VO derives_from・Test 管理宣言）と `orphan_detection`（文書層孤児）の整合性検査、診断出力（text / JSON）
- 完了条件：
  - fixtureのDiscovered Test、構造上完全なManaged Test Entity、ManagedTestLinkを区別して抽出できる。
  - 存在しないVOを参照するTest Entityを`ManagedTestLink::One`のまま保持し、E-SCAN-003と`chain_integrity = MISMATCH`を導出できる。
  - 複数target Testの全TargetRefを宣言順に抽出し、重複を拒否できる。
  - `TestEntity`は中立な`execution`を持ち、Rust互換fieldは`rust-cargo` wire codecだけが入出力する。
  - E-SCAN-002〜012、E-SCAN-016、W-SCAN-101がfixtureの該当箇所で検出される。
  - 未登録Testが存在する場合、`ManagedTestLink::Missing`から`chain_integrity = MISMATCH`（診断 `MISSING`）を導出できる。
  - document の `derives_from` 参照先不在（E-SCAN-012）・`content_hash` 不一致（W-SCAN-104、診断 `STALE`）を `chain_integrity = MISMATCH` として、根に列挙されない孤児 document（E-SCAN-016）を `orphan_detection = MISMATCH` として、文書層で導出できる。`doc.roots` に列挙された根 document は `orphan_detection` の対象外になる。
  - covers する Test が 1 件も存在しない leaf VO を `chain_integrity = MISMATCH`（診断 `MISSING`）として導出できる。
  - Relation writerは`REL-<ULID>`を生成し、readerは整合したbare ULID互換recordをin-memoryで正規化する。同一payloadの重複・混在・不一致はE-SCAN-010になる。
  - `vtest scan --format json`の出力が別紙A §12.1の envelope 構造（検証状態 `state` と診断ラベル `diagnostic` の2軸）に従う。
  - E-ADAPTER-*による操作拒否は終了コード2、完了したscanのE-SCAN-*は1、errorなしは0になる。
  - `vtest init` が `.verify/` 配下だけを生成し、既存ソース・既存 `.gitignore`・ビルド設定を新規作成・変更・削除しない（非改変不変条件、別紙A §12.2、基本仕様 §18.1）。既存 `.verify/` がある場合はファイル・ディレクトリを1件も作成・変更・削除せず終了コード2で中止する。

### M2 レコード管理とVO実体化

- 実装：`vtest-store`（書き込み）、`doc` / `vo` 系コマンド（別紙A §12.2 の `vtest doc add/list/show`・`vtest vo add/edit/list/show/expand`。`--derives-from` の任意 `--anchor`・`--note` を含む）、承認レコード生成の唯一の正典面 `vtest approval create/withdraw/show`（対象種別 `vo` / `document` / `judgment`。`vtest vo approve` はこの経路への別名。別紙A §12.2、本冊 §3.5）と対象hash・上流依存closure束縛、VO の `dimensions` / `coverage_policy` / `combinations`、`vo expand`、`doc.roots`（`orphan_detection` の除外根）の管理
- 完了条件：
  - `doc add` が `--path` の sha256 を document subject（本冊 §1.3）へ束縛した DOC レコードを作成し、`--derives-from` で上流 document への導出リンク（0件可＝根候補）を、任意の `--note` 付きで登録できる。
  - `doc add --root` / `--no-root` が当該 DOC を `doc.roots` へ追加・除外し、根指定の変更が `orphan_detection` の除外に反映される。
  - `doc add --update` が既存 DOC の sha256 を現ファイルで再計算し、document subject hash の変化により当該 document を依存 closureに含む判断記録・承認が失効する旨を出力する。
  - VOのadd → approve → editの順、および依存 document / parent VO の変更で承認が自動失効しdraftへ戻る。
  - dependency closureを完全・currentに解決できないapproveはE-APPROVAL-001となり、recordを生成しない。
  - `vo expand --dry-run`が`full-product`で直積の子VO一覧を出す。
  - document 登録後に実ファイルを書き換えると `content_hash` 不一致でW-SCAN-104（`chain_integrity = MISMATCH`、診断 `STALE`）が出る。
  - `derives_from` entryの `anchor` は欠落・空文字列でも `chain_integrity` 違反にならず、値の変更は `path` 実ファイルを変えないため document の `content_hash` を変化させない（本冊 §3.1・§3.2）。
  - `coverage_policy: explicit` での `combinations` 欠落・空、`explicit` 以外での非空、未宣言 dimension・未列挙 partition の参照、宣言 dimension の欠落・重複、重複 tupleをE-SCAN-017で拒否し、当該VOの`chain_integrity`を`MISMATCH`とする。不正`combinations`のVOに対する`vo expand`は子VOを1件も生成しない（本冊 §3.2.1・§17.1）。
  - `approved_state`（`approved` / `rejected` / `withdrawn`）または `subject` 種別の値域外、および `supersedes` の不整合（参照先不在・対象不一致・自己参照）をE-APPROVAL-002で拒否する（本冊 §3.5）。
  - `withdrawn` / `rejected` 後の再承認は、当該レコードのULIDを`supersedes`に名指しした`approved`レコードでのみ成立し、名指ししない追加は`draft`のままとなる（本冊 §3.5）。
  - `judgment_ref` を対象とする承認（subject種別`judgment`）の参照先が存在しない場合は書込み時にE-APPROVAL-001で拒否する（本冊 §3.5）。

### M3 決定論的静的解析

- 実装：`vtest-adapter-rust`の静的rule DA-001〜DA-006・W-DA-101、`vtest-audit`の委譲・集約、`vtest audit static`（再計算派生であり正典の監査レコードを生成せず stdout / `cache/` へ出力）
- 完了条件：
  - fixtureの各NG Testが対応ruleでFAILになる。
  - 正常Testは全rule違反なしになる。
  - 他ファイルの関数を呼ぶTestがDA-002でFAILではなくUNKNOWNになる。
  - 静的解析は正典レコードを持たない再計算派生であり、検証のたびに現在の source / config から再計算する（本冊 §7.1、基本仕様 P-003）。`assertion_macros`または参照helperを変更しても、STALE 化する永続レコードは存在せず、次回検証で再計算結果へ反映される。
  - adapterが解析入力集合（判定に用いた helper 等の source fragment 完全集合）の完全性を保証できないruleはUNKNOWNとなり、PASSへ集約されない。

### M4 テスト実行とEvidence

- 実装：`vtest-adapter-rust`のrunner起動・結果parse、`vtest-exec`の委譲・Evidence記録、`vtest run --fast`、鮮度判定（本冊 §11.2）
- 完了条件：
  - fixtureの全登録Testが実行され、TestごとにEvidenceが1件記録される。
  - Evidenceが`test_subject`、全宣言targetの`target_construct`内容hash、および完全なExecution State subjectを重複なく記録する。
  - Test constructと非隣接のcanonical metadataだけを変更してもEvidenceがSTALEになる。
  - 対象関数を書き換えた状態の検証でEvidenceが `NO_EVIDENCE`（診断 `STALE`）になり、`target_binding` を `PASS` にしない。
  - Test / 宣言targetを変更せずtarget外helperまたはlocal dependencyだけを変更してもEvidenceがSTALEになる。
  - HEAD revision不一致、Execution State subject欠落・不完全・不一致を現在のPASSへ利用しない。
  - revisionを特定できないEvidenceがSTALEになり、現在のPASSへ利用されない。
  - build failure fixtureでE-EXEC-001が出てEvidenceが記録されない。
  - 実行中にExecution State subjectが変化したfixtureでE-EXEC-004が出てEvidenceが記録されない。

### M5 判断記録プロトコル

- 実装：`vtest audit bundle` / `vtest audit submit`、bundle生成（`--kind` = `test-semantic` / `impl-consistency` / `case-coverage`。本冊 §8.1）、提出検証（E-AUDIT-001〜004、E-AUDIT-008）、判断記録保存（`.verify/decisions/`）
- 完了条件：
  - `test-semantic` bundleに本冊 §8.2の全fieldが含まれる。
  - `--vo` バンドルが対象 VO の claim・既知 partition・過去の判断を同梱し、網羅の疑義を `UNKNOWN` のエスカレーションとして運べる（網羅・意味の疑義は検査でなく判断記録へエスカレーションする）。
  - 理由・根拠（`reason` / `exclusions`）が空でも提出は拒否されない（本冊 §8.3・§8.4）。
  - bundle生成時と異なる対象hashの提出がE-AUDIT-002で拒否される。
  - `supersedes` に自己参照・存在しない・同一 `(subject, judgment_kind)` でないULIDを含む提出がE-AUDIT-008で拒否される（本冊 §8.4）。
  - `judgment_kind` を欠くか値域外の判断記録、および同一 `(subject, judgment_kind)` に判断値の食い違う有効判断記録が併存する場合をそれぞれW-STORE-003・W-STORE-004とし、実効判断へ寄与させない（本冊 §8.5）。
  - `supersedes` の参照先を解決できない、または循環する判断記録をW-STORE-005とし、実効集合へ寄与させない（本冊 §8.4・§8.5）。
  - 受理された判断記録が、対象変更（`subject_hash` 不一致）によって無効になる。
  - `impl-consistency` バンドルが対象VOの上流 document subject完全集合へ束縛され、document だけの変更でも判断記録が無効になる。
  - `impl-consistency` の判断提出（`accepted` / `rejected` / `deferred` 等）が判断記録として保存され、対象の検証状態（5状態）を昇格させない。旧モデルの `verdict → CheckValue` 写像・`impl_consistency = MISMATCH` は設けない。
  - 判断記録の受理は、承認（別紙A §12.2 `vo approve` の承認記録）とは別軸であり、いずれも検証状態を昇格・降格させない（判断済み ≠ 承認済み）。

### M6 集約とverify / report

- 実装：`vtest-verify`（固定4検査評価、fail-closed集約、2軸 scope、フェーズゲート評価、判断待ち情報 section）、`vtest verify` / `vtest report`（役割別 projection）
- 完了条件：
  - 全4検査（`chain_integrity` / `orphan_detection` / `target_binding` / `oracle_presence`）がPASSのfixture状態で`verify`がOK・終了コード0になる。happy-path fixtureでは全宣言 target が静的到達（DA-002 = PASS）で `target_binding = PASS` になるよう設計し、M7（coverage）到達前でも `target_binding` をPASSにできる状態にする。
  - 項目指定省略時はconfigに関係なく固定4検査を評価する。旧12項目の列挙は version を問わずE-CONFIG-001で拒否し、version 1 の `full_scope` 欠落だけを固定4検査へ具体化する（in-memory 補完なし）。version 2 の不完全・重複・未知・余剰 `full_scope` はE-CONFIG-001になる。
  - covers する Test の無い leaf VO で `chain_integrity = MISMATCH`（診断 `MISSING`）になり総合NGになる（旧 `spec_coverage` 検査は設けず、意味的網羅の疑義は `UNKNOWN` エスカレーションで扱う）。
  - Test metadata error は `chain_integrity` の非PASS要因になる（VO 分解を評価する旧 `vo_decomposition` 検査は存在しない）。
  - 4検査のそれぞれを単独で非PASSにするとNG・終了コード1になる。
  - 未登録Testが1件でもあれば、他の3検査がPASSでも`chain_integrity`によりNGになる。
  - `--items chain_integrity,orphan_detection`の限定scopeでOKが出ても、scope外の検査は `NO_EVIDENCE`（診断 `NOT_CHECKED`）のまま表示され、`--format json` 出力の最上位 `scope` field（別紙A §12.1）に要求scopeが示され、テキスト出力冒頭にも要求 scope と「scope 外は未検証」の旨が併記される。エンティティ軸（`--doc` / `--vo` / `--test` の部分木）でも限定できる。
  - 出力treeが別紙A §12.2のbranch規則に従い、状態列と診断ラベル列を分離して表示する。
  - `vtest verify --gate <name>` が、指定ゲートの検証結果（`require.verification`）と承認ロール（`require.approvals`、別紙A §12.3 の `approval_roles` 解決規則）の充足・不足を評価・提示する。承認済みを理由に検証状態を昇格させない。
  - config に定義の無いゲート名を指定した `--gate` 呼び出しをE-CONFIG-002で拒否し、検証・ゲート評価を実行せず結果を生成しない（本冊 §11.5・§17.1）。
  - `vtest report --from / --view / --depth / --direction` が役割別 projection（PM / Tester / Coder preset）を、M1 の逆引きインデックス（VO → Tests、SRC → Tests、DOC → VOs、DOC → DOCs。本冊 §5.3）を基盤に提示する。
  - `vtest verify` / `vtest report` の `--format json` 出力が判断待ち情報 section（`pending`、本冊 §11.7・別紙A §12.4）を含む。

### M7 target_binding 動的計測

- 実装：`rust-cargo` coverage連携、Test単位計測、`vtest run`（既定mode）
- 完了条件：
  - 対象関数を実際に通るTestで当該targetの`target_coverage`がPASS（count ≥ 1）になり、`target_binding`の到達要件を充足する。
  - 対象を呼ばないが`result: PASS`のTestで当該targetがcount 0となり`target_binding = FAIL`（診断 `NOT_EXECUTED`）になる。
  - 複数target Testで一方がPASS、他方がcount 0ならTest単位の`target_binding`集約がFAILになる。
  - 複数target TestでFAILがなく一方がUNKNOWNならTest単位の`target_binding`集約がUNKNOWNになる。
  - coverage toolを利用できない環境でW-EXEC-101が出て、当該Testの`target_coverage`が`checked: false`となり`target_binding`が `NO_EVIDENCE`（診断 `NOT_CHECKED`）になる。

### M8 Structured Test Operation

- 実装：coreのForm Schema読み込み・操作委譲、`vtest-adapter-rust`のStructuredTestAdapter・Form・検証器、`test create` / `test edit` / `test show` / `test list` / `test query`、別紙A §15のadapter contract
- 完了条件：
  - 誤ったsymbolを含む回答が候補付きE-OP-001で拒否される。
  - `test create`で生成されたTestがscanで正しく認識される。
  - `test edit`でcoversを変更しても他のTestのsource textが変化しない。
  - `test create` / `test edit` の適用後検証に失敗した場合（再パース不能、生成された宣言が desired state と不一致、変更が1 Test の範囲を超える）、適用前の状態へロールバックし操作を中止する（E-OP-003、別紙A §15.2・§15.4）。
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
