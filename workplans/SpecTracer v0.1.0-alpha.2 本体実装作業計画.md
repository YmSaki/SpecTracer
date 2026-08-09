# SpecTracer v0.1.0-alpha.2 本体実装作業計画

最終更新: 2026-08-10  
状態: `READY`（計画確定、実装未開始）  
仕様基準: `origin/develop` の merge commit `575ea72`  
対象: 言語アダプタ分離後の正規仕様へ本体実装を適合させる作業

## 文書の位置づけ

この文書は、マージ済みの正規仕様を実装へ反映するための非正規な作業計画・再開台帳である。要件定義、基本仕様、詳細設計、正規の別紙を上書きせず、新しいシステム契約を定義しない。

このファイルはコンテクスト圧縮や作業セッションの中断後にも、同じ基準と順序で作業を再開するためにローカルへ保存する。コミットは必須ではない。

実装中に正規仕様の欠陥、矛盾、不足が見つかった場合は、この文書や正規仕様を実装都合で修正して作業を続けてはならない。実装を停止してOwnerへ報告し、仕様変更を独立した上流工程へ戻す。

## 再開時に最初に読む正典

次の順序で確認する。

1. `AGENTS.md`
2. `docs/AI並列開発向けテスト検証システム 要件定義・要件分解 v0.1.md`
3. `docs/AI並列開発向けテスト検証システム 基本仕様 v0.1.md`
4. `docs/AI並列開発向けテスト検証システム 詳細設計 v0.1.md`
5. `docs/AI並列開発向けテスト検証システム 詳細設計 別紙A インターフェース仕様 v0.1.md`
6. `docs/AI並列開発向けテスト検証システム 詳細設計 別紙B 実装計画 v0.1.md`
7. `docs/AI並列開発向けテスト検証システム 詳細設計 別紙C 受入仕様 v0.1.md`
8. `docs/SpecTracer 言語アダプタ分離リファクタリング計画 v0.2.md`
9. この作業計画

正典とこの計画が矛盾する場合は正典を優先し、計画の修正だけで解決できない契約上の問題はOwnerへ報告する。

## 現状判定

仕様の基準revisionは、`develop`へのmerge commitである`575ea72`である。

現在の未コミットWIPは以下の状態である。

- `cargo check --workspace`: `PASS`
- `cargo test --workspace`: `PASS`
- adapter crateの骨格とRust固有処理の物理的な移動は進んでいる
- ただし現在仕様に対しては、W1〜W8のいずれも完了とは判定できない

主な差分は次のとおりである。

- adapter APIが完成済み`TestEntity`、`EvidenceRecord`を返しており、hash未計算DTO／core hash ownershipを満たしていない
- `TestEntity`に`filter`、`package`、`test_target`、`TestTarget`が残っている
- `Locator`と`SourceLocation`が`.rs`、function、Rust item path前提である
- `CheckItem`が11項目で、`test_traceability`がない
- config v2の`full_scope`が11項目である
- Static Auditにanalysis-source closureがない
- EvidenceにExecution State subjectがない
- `impl-consistency`にSPEC subject closureがない
- Semantic Auditのbundle/submitがCLI内に残り、4種類目の`spec-coverage`が不足している
- `vtest-audit`がRust adapterを直接選び、`vtest-exec`ではadapterがEvidenceを完成させている
- synthetic acceptanceもRust固有model fieldを使用している
- 現在の`release-check`が「11項目」「3 bundle」のままで、確定仕様と不一致である

したがって、現在WIPは捨てないが、完成度の基準にはせず、再利用可能なRust実装資産として扱う。

## 全体原則

- Git Flowに従い、`develop`から`feature/*`を作成する。
- 現在のdirty worktreeを破壊、reset、暗黙stashしない。
- 受入成果物をproduction実装より先に固定する。
- W0〜W8とM1〜M9の順序・依存関係を守る。ただし、発見した問題の根が上流成果物にある場合はPhase番号を修正範囲の上限にしない。
- 下流工程は上流成果物の欠陥を発見できるが、上流成果物を自己承認して変更しない。
- adapterは観測・言語固有処理を担当し、coreは正規化、hash、record生成、集約、fail-closed判定を担当する。
- 欠落、未対応、不完全、不明、staleを`PASS`へ昇格しない。
- CLIとMCPは同じcore service、adapter registry、JSON契約を使用する。
- production TypeScript、Go、C# adapter、plugin ABI、LSP、自動修復policyはalpha.2の対象外とする。

## Phase状態台帳

| Phase | 対応範囲 | 状態 | 完了の意味 |
| --- | --- | --- | --- |
| 0 | 作業環境・受入基準 | `NOT_STARTED` | cleanな実装laneと現在契約対応の検査基盤がある |
| 1 | 受入成果物 | `NOT_STARTED` | 別紙Cから導出した受入試験が実装前に固定される |
| 2 | W1 neutral model/API | `NOT_STARTED` | 共有型とadapter APIがfreezeされる |
| 3 | W2 store/config/compat | `NOT_STARTED` | v1/v2互換と固定12項目が成立する |
| 4 | W3 Rust discovery/operations | `NOT_STARTED` | Rust固有処理がneutral境界の内側で機能する |
| 5 | W4 static/semantic audit | `NOT_STARTED` | 判定dependency closureを束縛できる |
| 6 | W5 execution/evidence | `NOT_STARTED` | current execution stateへEvidenceを束縛できる |
| 7 | W6 verify/CLI/MCP | `NOT_STARTED` | 12項目とCLI/MCP共通契約が成立する |
| 8 | W7 synthetic adapter | `NOT_STARTED` | 非Rust constructで言語中立性を証明できる |
| 9 | W8 cleanup/dogfood/release | `NOT_STARTED` | 宣言済み全ゲートに根拠付きで合格する |

状態は、実際の完了ゲートを再現できた場合だけ更新する。旧契約でのテスト成功を現在契約の完了根拠にしない。

## 実装作業計画

### Phase 0 — 作業環境と受入基準の固定

開始地点を`origin/develop`の`575ea72`に固定する。

現在のdirty worktreeは一切reset/stashせず、そのまま保存する。実装時には別worktreeを作り、Git Flowに従って次のbranchを`develop`から作る。

```text
feature/adapter-separation-alpha2-implementation
```

この段階で実施すること:

- 現在WIPをファイル単位で分類する
  - そのまま再利用可能
  - API変更後に移植可能
  - 現在仕様と矛盾するため再実装
  - `.codex/config.toml`等の実装対象外／要確認
- refactor plan上のW0を`MERGED`として扱う
- `release-check`の11項目／3 bundle記述を12項目／4 bundleへ修正する
- release script、Skill、進捗台帳が現在契約を検査できる状態にする
- Luna max laneの利用可否を確認する。利用できなければ別モデルへ黙って切り替えず、実装開始を`NOT_EXECUTED`として報告する

完了ゲート:

- 基準revisionとfeature branchが明確である
- 既存dirty WIPが保全されている
- WIP再利用分類が記録されている
- 受入検査基盤の契約が12項目／4 bundleへ同期している

### Phase 1 — 受入成果物をproduction実装より先に確定

`docs/AI並列開発向けテスト検証システム 詳細設計 別紙C 受入仕様 v0.1.md`から、fixtureと受入テストを再導出する。

production codeを変更する前に、少なくとも次をテストとして固定する。

- hash未計算Discovery DTOとcore hash ownership
- 非Rust Test construct／opaque locator／非隣接metadata
- 未登録Test、空covers、dangling VO、Test ID/SRC ID衝突
- config v1/v2、固定12項目、互換reader
- Static Audit helper-only変更
- Runtime Evidence target外helper/local dependency変更
- Execution State欠落・不完全・不一致、E-EXEC-004
- `impl-consistency`のSpecification-only変更
- multi-target Evidence／coverage集約
- limited scopeとtree renderer
- Form kind ownership
- CLI/MCP parity

旧実装で期待どおり非PASSになることを記録する。受入成果物とproduction実装は同じコミットに入れない。

完了ゲート:

- 各受入テストが別紙Cの条項へ追跡できる
- 期待値が現実装の挙動ではなく正規仕様から導出されている
- false-PASS最小反例がfixtureとして固定されている
- production実装の変更前に受入テストだけを独立コミットできる状態である

### Phase 2 — W1: neutral modelとadapter API

最初に共有型と公開APIを確定する。ここがfreezeするまで後続crateへ進まない。

`vtest-model`で実装する項目:

- adapter-scoped opaque `TargetRef::Locator`
- adapter、path、opaque locator、byte rangeを持つ`SourceLocation`
- Rust fieldを持たない`TestEntity`
- 1件以上の`targets: Vec<TargetRef>`
- `ExecutionDescriptor`だけを実行座標として保持
- `CheckItem::TestTraceability`を含む固定12項目
- domain-separated、長さ付きfieldのSHA-256実装
- Test／Target／SPEC／record／Static Config／Static Analysis Source／Execution State subject
- multi-target Evidence schema
- Execution State subject
- 型付きAudit subject

`vtest-adapter-api`で実装する項目:

- `SourceFragment`
- `ManagedTestDraft`
- `DiscoveredTestDraft`
- `ManagedTestDraftLink`
- `SourceTargetDraft`
- completeness付き`DiscoveryBatch`
- `TestWireCodec`
- `StaticAuditConfigDraft`
- `StaticAnalysisClosureDraft`
- `ExecutionStateDraft`
- Evidenceを含まないrunner observation
- capability registryと明示操作／検証時の欠落semantics

完了ゲート:

```powershell
cargo test -p vtest-model
cargo test -p vtest-adapter-api
```

加えて、両crateにCargo、`syn`、Rust function、`.rs`前提がないことを検査する。

### Phase 3 — W2: store、config v2、互換reader

実装対象は主に`vtest-store`とRust wire codecである。

- v1 configを無変更で読み取る
- v1の11項目をin-memoryで固定12項目へ補完
- v2は12項目の欠落・重複・未知・余剰を`E-CONFIG-001`
- `vtest init`はv2を出力
- adapter namespaceとrootsを検証
- 同一adapter内のroot重複を拒否
- Relation writerは`REL-<ULID>`
- bare ULID互換readerとpayload重複検出
- VO statusをApprovalから導出
- Approvalを完全な上流dependency closureへ束縛
- Rust互換JSON fieldを`rust-cargo` wire codecだけへ隔離

完了ゲート:

- M2
- config／compatibility matrixの全項目
- legacy inputから現在の不変条件を迂回できないこと

### Phase 4 — W3: Rust discoveryとStructured Test Operation

現在WIPの`discovery.rs`と`operations.rs`は、Rust AST処理資産として再利用する。ただし返却型とownershipは作り直す。

- Rust adapterはhash未計算draftとcurrent bytesだけを返す
- core scanがDTOを検証し、hash計算後にdomain entityを具体化
- 全Discovered TestとManaged Testを分離
- unregistered Testを`ManagedTestLink::Missing`
- dangling coversを`ManagedTestLink::One`＋`MISMATCH`
- 全adapter統合後にTest ID／SRC IDを大局検査
- configのadapter rootsごとにscan
- `vtest-scan`からRust operationsのre-exportを除去
- Form ownerをregistry／schema／capabilityから一意に解決
- create/editの1 Test境界と冪等性を維持
- Rust互換JSONはwire codec経由のみ

完了ゲート:

- M1
- M2
- M8
- discovery／Form関連の別紙C受入条件

### Phase 5 — W4: Static AuditとSemantic Audit

Static Auditでは、Rust adapterは判定結果と根拠だけでなく、判定に使用した全入力を返す。

- rule-set ID/version
- rule影響config projection
- Test／全target
- helper等のStatic Analysis Source closure
- closure completeness

coreがsubject hashを計算し、Audit Recordを保存する。adapter自身は完成済みAudit Recordを書かない。

CLI内にあるsemantic bundle/submit処理を`vtest-audit`の共通serviceへ移す。

- `spec-coverage`
- `test-semantic`
- `vo-coverage`
- `impl-consistency`

`impl-consistency`はVOの上流SPEC subject完全集合を必須にする。

完了ゲート:

- M3
- M5
- helper-only変更でStatic Auditが`STALE`
- closure不完全で`UNKNOWN`
- Specification-only変更で`impl_consistency = STALE`
- E-AUDIT-001〜007

### Phase 6 — W5: runner、coverage、Execution Evidence

Rust adapterはrunner observationとhash未計算Execution State draftだけを返す。Evidenceの生成・append-only保存は`vtest-exec`が所有する。

- canonical invocation
- toolchain identity
- 実行影響config
- HEAD revision
- workspace/local dependencyのbyte-exact manifest
- pre/post Execution State同一性
- E-EXEC-004
- multi-target hash／coverage result
- legacy Evidence compatibility
- missing subject=`STALE`
- incomplete snapshot=`UNKNOWN`
- HEAD不一致=`STALE`
- adapter不一致=`MISMATCH`

coverageはrunnerと分離したcapabilityとして扱い、不在なら`NOT_CHECKED`とする。

完了ゲート:

- M4
- M7
- target外helper変更
- local dependency変更
- build failure／missing result／state mutation時のEvidence非生成

### Phase 7 — W6: 12項目verify、report、CLI/MCP統合

`vtest-verify`を現在契約へ合わせる。

- 固定12項目
- repository-level `test_traceability`
- Specification rootからの`spec_coverage`
- Test診断を混ぜない`vo_decomposition`
- `impl_consistency FAIL -> MISMATCH`
- invalid Evidenceの実行3項目への非PASS伝播
- limited scope外は`NOT_CHECKED`
- 正しいtree branch renderer
- 全subject集合のexact-set freshness判定

CLI内のcore処理を対応crateへ降ろし、CLIとMCPは同じserviceとregistry compositionを呼ぶ。

- MCPに`spec-coverage`を含む4 bundle
- 22 toolのCLI envelope parity
- adapter error code/message/candidates一致
- `vtest-mcp`から`vtest-adapter-rust`への不要な直接依存を除去
- 長時間MCP sessionでも再scanし、stale PASSを保持しない

完了ゲート:

- M6
- M9
- 12項目単独非PASSの全数試験
- limited scope
- text/JSON tree golden test
- CLI/MCP parity

### Phase 8 — W7: synthetic adapterによる言語非依存性の証明

synthetic adapterをテストコードだけに実装する。

- `.rs`以外のsource
- 関数ではないTest construct
- doc commentではないmetadata
- opaque locator
- fixed runner observation
- coverage capabilityなし

このadapterを追加するために`vtest-model`、`vtest-scan`、`vtest-verify`を変更してはならない。

完了ゲート:

- Rust＋syntheticの決定論的merge
- duplicate Test/SRC ID拒否
- non-adjacent metadata freshness
- synthetic Evidence freshness
- coverageなし=`NOT_CHECKED`
- Rust互換fieldを一切出力しない

### Phase 9 — W8: cleanup、dogfood、release gate

最終段階でのみ非正規文書、配布物、進捗台帳を同期する。

- 禁止依存を`cargo metadata`で検査
- `release-check`とscriptを現在契約へ同期
- project/plugin Skillをbyte-identicalにする
- README／ARCHITECTURE／DEVELOPMENT等のWIPを現在実装に照合して採否判断
- M1〜M9を現在契約で再実行
- `verify-change`
- `architecture-check`
- `release-check --milestone M9`
- 独立reviewer

最終dogfoodingを開始する前に、チャット上で次を合格条件として宣言する。

- 全unit testが管理対象で`test_traceability = PASS`
- Specification → REQ → VO → Testが完全
- Approval／Static Audit／Semantic Auditがcurrent
- current HEADに対するExecution State付きEvidence
- full target measurement
- 12項目すべて`PASS`
- text/JSON reportが一致
- CLI/MCP parity
- stale／missing／unknown入力から`PASS`へ昇格する経路がない

宣言後にのみdogfoodを実行し、項目ごとの結果と根拠recordを報告する。

## コミット／PR方針

production実装PRは`develop`向けの通常PRとし、次の責務でコミットを分ける。

1. acceptance fixtures/tests
2. neutral model/hash
3. adapter API/registry
4. config/store/compatibility
5. Rust discovery/Structured Operations
6. static/semantic audit
7. runner/coverage/Evidence
8. verify/CLI/MCP
9. synthetic boundary
10. cleanup/docs/distribution

仕様書は変更しない。実装中に仕様の不足・矛盾を発見した場合は、その場で仕様を直さず作業を停止し、Ownerへ報告する。

## 作業再開プロトコル

新しいセッションまたはコンテクスト圧縮後は、次の順序で再開する。

1. この文書と「再開時に最初に読む正典」を読む。
2. `git status --short --branch`、`git log --oneline --decorate -n 10`、`git worktree list`を取得する。
3. 基準revision、作業branch、dirty WIPの保全状態を照合する。
4. Phase状態台帳と実際のコミット・テスト根拠を照合する。
5. 最初の未完了Phaseから再開する。後続Phaseの見かけ上の実装を完了根拠にしない。
6. production変更前に対象Phaseの受入条件をチャットで宣言する。
7. Phase完了時は、実行した検査、結果、根拠ファイル／record、未実行項目を記録してから台帳を更新する。

## 停止条件

次のいずれかが発生した場合は実装を停止し、Ownerへ報告する。

- 正規仕様間の矛盾または実装不能な契約を発見した
- 仕様変更なしでは受入条件を一意に実装できない
- 既存canonical recordを破壊・書換えないと互換性を維持できない
- fail-closedを維持すると公開契約と衝突する
- Luna task lane等、Ownerが指定した実行条件を満たせない
- dirty WIPの保全とcleanな実装laneの両立ができない

この計画に従い、現在WIPを保全しながら、W1の共有型から順に現在仕様へ実装を合わせる。
