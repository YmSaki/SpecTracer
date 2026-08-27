# AI並列開発向けテスト検証システム 詳細設計 別紙A インターフェース仕様 v0.1

本冊 §0 の分冊構成に基づき、本別紙は §12〜§15 を収録する。
参照規則・診断コード・終了コードは本冊 §17 に従う。本別紙は基本仕様 §26.1（CLI コマンド一覧）・§26.2（MCP ツール一覧）が確定したコマンド・ツールの引数と入出力 schema を具体化する HOW であり、新規コマンド・ツールを増やさない（本冊 §0）。上流（要件定義＝WHY、基本仕様＝WHAT、詳細設計本冊＝HOW 中核）に無い義務・検査・状態・文書種別・関係型を発明しない。

---

## 12. CLI 詳細仕様

### 12.1 共通仕様

- すべてのコマンドは非対話で完結する。確認プロンプトを出す場合は `--yes` で抑止できる。
- 出力は既定で人間向けテキスト、`--format json` で機械可読 JSON。
- JSON 出力は最上位に `{ "ok": bool, "data": ..., "diagnostics": [...] }` を持つ。`diagnostics` の要素は `{ "code": "E-SCAN-002", "severity": "error", "message": "...", "location": ... }`。
- 終了コードは本冊 §17.2 に従う。
- グローバルオプション：`--project <dir>`（プロジェクトルート。既定はカレントから `.verify/` を上方探索）、`--format <text|json>`、`--quiet`。

**検証状態と診断ラベルの2列表現。** 検証結果を出力するすべてのコマンド（`verify` / `report` / `scan` の集約表示等）は、検証状態と診断ラベルを常に別軸の2列として提示する（本冊 §5.2、基本仕様 §4.1・§4.2）。

- **検証状態は5値**：`PASS` / `FAIL` / `MISMATCH` / `NO_EVIDENCE` / `UNKNOWN`。JSON では各検査ノードの `state` field へ入れる。
- **診断ラベルは状態に付随する原因説明**：`MISSING` / `NOT_EXECUTED` / `NOT_CHECKED` / `STALE`。JSON では各検査ノードの `diagnostic` field（0件以上）へ入れ、`state` の値には決して用いない。`NO_EVIDENCE` は状態であって診断ラベルではない。診断ラベルを集約の代表値選択（本冊 §11.3、基本仕様 §22.2）に用いず、原因説明として併記するだけとする。

CLIの操作は登録済みadapter registryを通じて実装を選択する。
JSON envelope、adapter選択エラー、capability不足の非PASS扱いはMCPと共通であり、
CLIだけがRust固有の既定値へフォールバックしてはならない。

Testを含むJSONは本冊 §5.2の `execution` を必ず返す。`rust-cargo` Testについてだけ、
wire compatibility layerが`filter`、`package`、`test_target`を追加できる。これらは
`TestEntity`のfieldではない。非Rust TestではRust互換fieldを省略し、空値またはdummy値を返さない。

Test JSONは `targets` を常に list として返す（本冊 §5.2）。coreは `targets ≥ 1` を adapter 中立の必須件数にせず型としては空 list を許容し、必須件数は adapter が定める（`rust-cargo` は `targets ≥ 1` を必須とする。本冊 §4.1・§4.4）。targetが1件の場合だけ同値の単数互換field`target`を追加できる。複数target Testでは単数fieldを省略し、先頭targetを代表値として返さない。

Test入力から `execution` を復元できるのは、`rust-cargo` codecに完全で相互整合するRust互換実行座標が
与えられた場合だけである。`execution`と互換fieldが併存する場合は一致を必須とする。

本 version の Test metadata は存在理由分類（旧 `role` / `anchor` / `anchor_rationale`）を持たず、すべての管理対象 Test に `covers ≥ 1` を一律に要求する（本冊 §4.1、基本仕様 §12、要件定義 §4.1）。したがって CLI・MCP の入出力に role / anchor の宣言逐語 field・実効 field・既定値埋めは存在しない。VO への寄与は `covers` 宣言と証拠の十分性判定だけから導出する。

明示操作に必須のadapter capabilityが未提供なら、`ok: false`、E-ADAPTER-004、終了コード2を返す。
create / editではファイルを変更せず、auditでは判断記録を、runではEvidenceを生成しない。
検証・reportで能力不足を観測した場合はW-ADAPTER-101と能力別の非PASS値（static / coverage 欠落は `NO_EVIDENCE`／診断 `NOT_CHECKED`、runner 欠落は `NO_EVIDENCE`／診断 `NOT_EXECUTED`、解析限界は `UNKNOWN`）を返す（本冊 §5.2 末尾、基本仕様 §22.3）。

### 12.2 コマンド仕様

#### `vtest init`

```text
vtest init [--name <project-name>]
```

`.verify/` 一式（本冊 §2.1）を生成する。`config.yaml` は本冊 §2.2のversion 2で、
組込 `rust-cargo` adapter namespaceを含む。生成物には `doc/` / `vo/` / `rel/` / `forms/` / `decisions/` / `approvals/` / `evidence/` / `cache/` と `.verify/.gitignore`、組込 Form Schema（§14）を含む。既存の `.verify/` があればエラー（終了コード 2）。

**非改変不変条件**（基本仕様 §18.1、要件定義 R-5）。`vtest init` は `.verify/` を作成するだけであり、既存コードを変更しない。観測可能な条件として次を満たす。

- 作成するファイル・ディレクトリは `.verify/` とその配下に限る。プロジェクトルート直下の `.gitignore`・ビルド設定（`Cargo.toml` 等）・CI 設定を含め、`.verify/` の外にあるいかなるファイルも新規作成・変更・削除しない。
- 既存ソースコード・既存テストコードのバイト列を変更しない。Test metadata 宣言（`@vtest.` 行）・annotation・doc comment を既存ソースへ挿入しない。管理宣言の付与は `test create` / `test edit`（§15）と利用者自身の編集だけが行い、`init` は行わない。
- 既存の `.verify/` があるときは終了コード 2 で中止し、その実行でファイル・ディレクトリを 1 件も作成・変更・削除しない。既存 `.verify/` の内容を上書き・マージ・移動しない。
- したがって `init` の実行前後で、`.verify/` を除いた作業ツリーの内容は同一である。既存プロジェクトへの後からの導入が既存資産を書き換えないことは、この不変条件で保証する。

#### `vtest scan`

```text
vtest scan
```

スキャンと整合性検査（本冊 §5）を実行し、診断一覧とエンティティ数のサマリを出力する。整合性検査は `chain_integrity`（文書鎖・VO derives_from・Test 管理宣言）と `orphan_detection`（文書層孤児）を構成する（本冊 §5.6、基本仕様 §23）。
registry・config・adapter契約の検証またはadapter呼出しがE-ADAPTER-* / E-CONFIG-*で拒否された場合は終了コード2とし、scan結果を生成しない。scanが完了し、repository整合性のE-SCAN-*診断がある場合は終了コード1、error診断がなければ0とする（本冊 §17.2）。

#### `vtest doctor`

`vtest scan` と同一処理の別名。自動化環境の整合性検査に使用する（本冊 §16.2）。同じTest IDの重複（E-SCAN-002）、covers先VOの欠落（E-SCAN-003）、文書鎖のリンク切れ（E-SCAN-012）、孤児 document（E-SCAN-016）、承認・判断・Evidenceのハッシュ束縛による失効（診断 `STALE`）など、version controlの構文的整合性だけでは判定できない論理的不整合を検出する。

#### `vtest doc add / list / show`

```text
vtest doc add --id DOC-BASIC-001 --path docs/basic-spec.md
              [--title <t>]
              [--derives-from DOC-REQ-001 [--anchor <text>] [--note <text>]]...
              [--root | --no-root] [--update]
vtest doc list [--tree] [--roots]
vtest doc show DOC-BASIC-001
```

`doc` は上流文書を総称 `document` レコード（本冊 §3.1、基本仕様 §3.1・§3.2）として管理する唯一のコマンドである。文書種別（要件定義・基本仕様・詳細設計・API Schema 等）を区別せず、段（要件→仕様→詳細設計…）は `derives_from` リンクで表現し種別を増やさない。旧モデルの `vtest spec` / `vtest req` は廃し、SPEC / REQ 実体層は持たない。

- `add` は `--path` の対象ファイルの sha256 を計算して document subject（本冊 §1.3 document subject hash）へ束縛した DOC レコードを作成する。`--derives-from` は上流 document への導出リンク（0件可＝根候補）で、各リンクに任意の `--note`（導出理由・空可・非 `MISMATCH`。基本仕様 §3.4）を付けられる。
- `--anchor <text>` は直前の `--derives-from` に束縛し、参照先 document 内の該当箇所（節番号・条項番号・見出し等）を記録する（本冊 §3.1）。`--note` と同じ結合規則・同じ任意性であり、省略・空文字列は `chain_integrity` 違反にならない。値は不透明な文字列として保存し、文書内位置への解決・実在確認・書式検証を行わない。`--derives-from` を伴わない `--anchor`、または 1 つの `--derives-from` に対する 2 個目以降の `--anchor` は引数不正として終了コード 2 で拒否し、レコードを書かない。`show` は各 `derives_from` entry の `anchor` を表示する。
- `--root` / `--no-root` は当該 DOC を `orphan_detection` の除外根（`config.yaml` の `doc.roots`。本冊 §2.2・§5.6）へ追加／除外する。根指定の追加・削除はこのフラグで管理し `doc.roots` へ反映する（`doc edit` は設けない。正典編集は `add --update`）。
- `--update` は既存 DOC レコードの sha256 を現ファイルで再計算して更新する。document subject hash が変化するため、当該 document を依存 closure に含む判断記録・承認が失効する旨を出力する（本冊 §3.5・§8.5・§11.4）。`--root` / `--no-root` を併せて根指定も更新できる。
- `list --tree` は `derives_from` の文書鎖を木として表示し、`--roots` は現在の根集合を表示する。`show` は DOC の path・content_hash・derives_from・根指定・鮮度（content_hash と実ファイルの一致）を表示する。
- `derives_from` の参照先 document が存在しなければ文書鎖のリンク切れとして `chain_integrity = MISMATCH`（E-SCAN-012）、`path` の実ファイルが `content_hash` と一致しなくなれば `chain_integrity = MISMATCH`（診断 `STALE`。本冊 §11.4）、根に指定されず親も持たない document は孤児として `orphan_detection = MISMATCH`（E-SCAN-016。本冊 §5.6）とする。

#### `vtest vo add / edit / list / show / expand / approve`

```text
vtest vo add --id VO-X --claim <c>
             --derives-from DOC-X [--anchor <text>] [--note <text>]
             [--derives-from DOC-Y [--anchor <text>] [--note <text>]]...
             [--parent VO-Y]
             [--dimension <name>=<p1>,<p2>...]... [--policy <policy>]
             [--combination <dim>=<part>[,<dim>=<part>]...]...
vtest vo edit VO-X [--claim ...] [--derives-from DOC-X [--anchor <text>] [--note <text>]]...
             [--parent ...] [--dimension ...]... [--policy ...]
             [--combination ...]... [--clear-combinations]
vtest vo list [--tree] [--doc DOC-X] [--status draft|approved]
vtest vo show VO-X          # claim、derives_from、covers している Test、判断記録・承認状態を表示
vtest vo expand VO-X [--dry-run]
vtest vo approve VO-X --approver-kind <human|agent> --approver-id <id>
                 [--model <m>] [--judgment <decision-id>] [--basis <ref>]...
```

VO は 1 件以上の `document` から `derives_from` で直結して導出される（本冊 §3.2、基本仕様 §3.2）。旧モデルの `--req`（REQ 参照）・`--spec` / `--section`（SPEC + 節参照）は廃し、上流参照は `--derives-from DOC-*`（任意の `--note`）へ一本化する。VO の `status`（`draft` / `approved`）は正典 field ではなく承認レコードから導出する表示値である（本冊 §3.2・§3.5。読取り互換 field として保存されていても値は無視し、存在自体は W-STORE-001）。旧 REQ の `active` / `withdrawn` 語彙は REQ 層とともに廃止する。`--doc DOC-X` は当該 document を根とする下流 VO の絞り込みである。

`--anchor <text>` は直前の `--derives-from` に束縛し、参照先 document 内の該当箇所（節番号・条項番号・見出し等）を記録する（本冊 §3.2）。`--note` と同じ結合規則・同じ任意性であり、省略・空文字列は `chain_integrity` 違反にならず、値は不透明な文字列として保存する。`--derives-from` を伴わない `--anchor`、または 1 つの `--derives-from` に対する 2 個目以降の `--anchor` は引数不正として終了コード 2 で拒否し、レコードを書かない。`show` は各 `derives_from` entry の `anchor` を表示する。`anchor` は VO subject hash に入らないため、`anchor` だけを変更した `edit` は承認を失効させない（本冊 §3.2）。

`--combination` は `coverage_policy: explicit` のときに実体化する組合せ（本冊 §3.2.1 の `combinations`）を入力する。1 回の出現が 1 tuple に対応し、`<dim>=<part>` をカンマ区切りで並べて全軸の値を与える（例：`--combination operand-sign=positive,operator=div`）。複数 tuple は `--combination` を繰り返して与える。`edit` の `--combination` は desired state であり、1 回以上与えたときは既存 `combinations` を与えた集合で置換する（追記しない）。`--clear-combinations` は `combinations` を空にする。`--combination` も `--clear-combinations` も与えない `edit` は既存 `combinations` を保持する。
`--combination` の値が本冊 §3.2.1 の受理条件（`explicit` 以外での指定、未宣言 dimension、未列挙 partition、宣言 dimension の欠落・重複、重複 tuple、`explicit` かつ tuple 0 件）に違反する場合は E-SCAN-017、終了コード 2 で拒否し、レコードを書かない（`add` では新規レコードを作成せず、`edit` では既存レコードを変更しない）。`<dim>=<part>` の形をなさない値は引数不正として終了コード 2 で拒否する。

`expand` は本冊 §3.2.1 の実体化（`independent-axes` / `full-product` / `explicit`）。`--dry-run` は生成予定の子 VO 一覧のみ表示する。`explicit` の VO は `combinations` の各 tuple につき 1 件の子 VO を、`dimensions` の宣言順に連結した suffix（`VO-X-<P1>-<P2>`）で生成する。`combinations` が本冊 §3.2.1 の受理条件に違反する VO に対しては E-SCAN-017、終了コード 2 とし、子 VO を 1 件も生成しない（部分生成しない）。
`approve` は現在のVO内容ハッシュと本冊 §3.5の上流依存closure（再帰 parent VO・derives_from document・その上位 document）に束縛された承認レコード（`.verify/approvals/`）を追加する。`--judgment` は参照する判断記録 ID（本冊 §3.5 の `judgment_ref`、任意）、`--basis` は根拠参照（任意）である。承認は検証状態と独立の別軸であり、承認済みを理由に非 `PASS` を `PASS` へ昇格させない（本冊 §3.5、基本仕様 §4.5・§17）。
対象またはいずれかの依存entity / document sourceを完全・currentに解決できない場合はE-APPROVAL-001、終了コード2としてrecordを追加しない。
`edit` は承認済 VO に対して警告を出す（編集自体は許可し、承認はハッシュ不一致で自動失効する）。

#### `vtest test create`

```text
vtest test create --form rust-unit-function
                  --answers answers.yaml [--dry-run]
```

Form Schema（§14）に基づく回答ファイルを受け取り、検証のうえ対応adapterがTest constructとmetadata宣言を生成して挿入する。
`--dry-run` は挿入内容と挿入位置のみを表示する。
回答の検証エラーは E-OP-001 として候補付きで報告する（本冊 §6.3）。

回答ファイル例：

```yaml
form: rust-unit-function
answers:
  target: src/parser.rs::Parser::parse
  covers: [VO-PARSER-UTF8-003]
  behavior: 不正 UTF-8 入力の拒否
  test_kind: error
  input: 不正な continuation byte を含むバイト列
  expect: ParseError::InvalidUtf8
  fn_name: rejects_malformed_utf8
  file: tests/parser_test.rs        # 省略時は target と同居する tests モジュール
```

#### `vtest test edit`

```text
vtest test edit TEST-X --answers desired.yaml [--dry-run]
vtest test edit TEST-X --set covers=VO-A,VO-B [--set intent="..."]...
```

desired state 方式（基本仕様 §15.1）。
`--answers` は完全なあるべき状態、`--set` は指定フィールドのみのあるべき値を宣言する。
編集の実装は §15。Test implementationの書き換えは`--body-file <path>`でadapterへ全文を与える。

#### `vtest test show / list / query`

```text
vtest test show TEST-X        # intent、covers、targets、位置、判断記録・Evidence 状態
vtest test list [--vo VO-X] [--unregistered]
vtest test query --source rust-cargo::src/parser.rs::Parser::parse   # SRC からの逆引き
```

`show` は Test の intent・covers・targets（宣言 target 集合）・Source Location・判断記録（§8）・Evidence（§9）の状態を表示する。role / anchor の表示・`--role` フィルタは持たない（本冊 §4.1）。`query` の逆引きは §11.6 の役割別 projection の基盤（VO → Tests、SRC → Tests。本冊 §5.3）としても用いる。

#### `vtest audit static`

```text
vtest audit static [--test TEST-X | --all]
```

決定論的な静的解析（本冊 §7）を要求時に起動し、rule 別 verdict（`FAIL` / `UNKNOWN` / 違反なし）と根拠 span を **stdout と `cache/`** へ出力する。静的解析は正典レコードを持たない再計算派生であり（本冊 §7.1、基本仕様 P-003）、`audit static` は正典の監査レコードを生成しない。`oracle_presence` へ供給する DA-001 / DA-003 / DA-004 / DA-005 / DA-006 と、`target_binding` の静的到達（DA-002）を評価し、target-scoped な DA-002 / DA-003 は宣言 target ごとの verdict を規則単位 verdict と併せて提示する（本冊 §3.6・§7.2）。判断記録（§8）とは別機構であり、外部判断の記録には転用しない（本冊 §7）。

#### `vtest audit bundle / submit`（判断記録プロトコル）

```text
vtest audit bundle (--test TEST-X | --vo VO-X) [--kind test-semantic | impl-consistency]
                   [--include-failed]
vtest audit submit --file result.json
```

`audit bundle` / `submit` は本冊 §8 の**判断記録**プロトコルであり、意味検査ではない。本システムは宣言されていない義務・網羅漏れ・宣言と実装の意味のずれを自ら発見・裁定せず、機械が決定論で確定できない疑義を `UNKNOWN` として外部（人間または判断可能 Agent）へ引き渡し、その判断を判断記録（`.verify/decisions/`）として追跡する（本冊 §8 冒頭、基本仕様 §11、要件定義 §12）。

- `bundle` は判断対象（`--test` / `--vo`）ごとに、判断に必要な情報一式（対象 VO と claim・Test Intent・テストコード全文・対象実装全文・関連テスト・既知 partition・過去の判断・対象の内容ハッシュとリビジョン。本冊 §8.1）を JSON として `cache/bundles/<ULID>.json` へ出力し、パスと `bundle_id` を返す。バンドルは派生情報でありGit管理しない。
- `--kind` は判断させる**UNKNOWN のエスカレーション質問**のラベルであって検査項目ではない。`test-semantic`＝「テストコードは VO の claim と Test Intent が宣言する振る舞いを実際に検証しているか」（本冊 §8.6）、`impl-consistency`＝「対象実装が宣言と一致するか」（本冊 §8.1。上流 document を要するため §3.5 と同じ上流依存規則で document 完全集合を同梱する）。省略時は対象種別に応じた既定の質問で生成する。VO 網羅の疑義は `--vo` バンドルの既知 partition 情報で運ぶ。旧モデルの `spec-coverage`（SPEC 層依存）は復活させない。
- `submit` は本冊 §8.4 の検証（bundle_id 存在＝E-AUDIT-001、subject 一致＝E-AUDIT-003、記録時ハッシュと現在ハッシュの一致＝E-AUDIT-002、decision が受理値＝E-AUDIT-004）を行い、受理時に判断記録 ID（`.verify/decisions/` の ULID）を出力する。判断は少なくとも actor / subject / decision を含み、理由・根拠（`reason` / `exclusions`）は任意である。**理由が空であることだけを根拠に判断を無効化しない**（本冊 §8.3、基本仕様 §11.3、要件定義 §12）。`decision` の受理値は `accepted` / `rejected` / `deferred` 等（本冊 §8.3）。
- **判断記録の受理は当該対象の検証状態（5状態）を昇格させない**（本冊 §8.3・§3.4、基本仕様 §11.3）。判断記録は検査ゲートではなく、`UNKNOWN` に対する外部判断の追跡である。旧モデルの `verdict → CheckValue` 写像・reasons / basis 必須検査（E-AUDIT-005〜007）は撤去する（本冊 §8.4）。

**判断記録面と承認記録面の分離。** 判断記録（`.verify/decisions/` の actor / subject / decision・理由 optional。本冊 §3.4）と承認記録（`.verify/approvals/` の approver / subject または judgment_ref / approved_state。本冊 §3.5、`vo approve` で生成）は別軸・別 entity である。判断済み ≠ 承認済みであり（本冊 §8.5、基本仕様 §17）、判断は承認なしでも記録でき、正式採用は承認の別段階である。いずれも検証状態を昇格・降格させない。

#### `vtest run`

```text
vtest run (--test TEST-X | --vo VO-X | --all) [--fast]
```

テスト実行と Evidence 記録（本冊 §9、§10）。旧モデルの `--req`（REQ 指定）は document 層の総称化により廃止し、document scope が必要な場合は VO 部分木経由で指定する（本冊 §9.1）。
`--fast` は cargo test のみで、`target_coverage` を `checked: false` として記録する（`target_binding` の動的証拠を採らない。検証時 `NO_EVIDENCE`／診断 `NOT_CHECKED`。本冊 §10.3）。

#### `vtest verify`

```text
vtest verify [--items <check1,check2,...>]
             [--doc DOC-X | --vo VO-X | --test TEST-X]
             [--gate <name>] [--summary]
```

集約（本冊 §11.3）を実行し、`OK` / `NG` を返す。検査は基本仕様 §5 の**固定4検査**（`chain_integrity` / `orphan_detection` / `target_binding` / `oracle_presence`）のみである。旧モデルの12項目（`spec_coverage` / `vo_decomposition` / `vo_coverage` / `test_existence` / `static_audit` / `semantic_audit` / `impl_consistency` / `test_execution` / `runtime_result` / `target_execution` / `evidence_validity` / `test_traceability`）は検査として存在しない（本冊 §11.1 に吸収・撤去の写像）。

- **scope の2軸**（基本仕様 §4.6、本冊 §11.3）：`--items` が検査軸（4検査の部分集合）、`--doc` / `--vo` / `--test` がエンティティ軸（部分木）である。旧モデルの `--spec` / `--req` は廃止し、`--req` は除去する。
- `--items` 省略時は常に固定4検査による完全検証を行う。`config.yaml` の `verify.full_scope` は本冊 §2.2 の invariant として事前に検証・正規化し、項目選択 knob として使用しない。旧12項目の列挙は version を問わず E-CONFIG-001 とし、version 1 の field 欠落だけを固定4検査へ具体化する（in-memory の項目補完は行わない。本冊 §2.2）。
- `--items` に4検査未満の明示的な集合を指定した場合だけ限定 scope とし、scope 外・未実施の検査は `NO_EVIDENCE`（診断 `NOT_CHECKED`）として保持し、`PASS` へ変換しない。限定 scope の結果を完全検証 OK と表示しない。いかなる設定値も完全検証を4本未満へ縮退させない（基本仕様 §4.6・§22.1）。
- scope を限定した場合、出力冒頭に要求 scope と「scope 外は未検証」の旨を必ず表示する。
- `--gate <name>` はフェーズゲート評価（§12.3）。`--summary` は総合 `OK` / `NG` と非 `PASS` 件数のみを出力する。

出力例（テキスト）：

```text
Requested scope: full (4 checks), entity: DOC-BASIC-001 部分木
（entity 軸で限定。scope 外エンティティは未検証）

Structural checks:
├─ chain_integrity                      MISMATCH   [MISSING]        (leaf VO-PARSER-UTF8-004 に covers する Test なし)
└─ orphan_detection                     PASS

└─ DOC-BASIC-001                        NG
   └─ VO-PARSER-UTF8                     NG
      ├─ VO-PARSER-UTF8-003              NG
      │  └─ TEST-PARSER-044              NG
      │     ├─ target_binding            FAIL       [NOT_EXECUTED]  (evidence 01J8XW1B..., 2 targets: 1 PASS / 1 count 0)
      │     └─ oracle_presence           FAIL                       (DA-006 空検証: src/parser.rs へ assert 相当なし)
      └─ VO-PARSER-UTF8-004              MISMATCH   [MISSING]        (no covering test)

Result: NG
```

- 状態列（`PASS` / `FAIL` / `MISMATCH` / `NO_EVIDENCE` / `UNKNOWN`）と診断ラベル列（`[MISSING]` / `[NOT_EXECUTED]` / `[NOT_CHECKED]` / `[STALE]`）を分離して表示する。診断ラベルは代表値の順位（基本仕様 §22.2 の `FAIL > MISMATCH > NO_EVIDENCE > UNKNOWN`）に用いず、原因説明として併記する。
- `target_binding` の非 `PASS` は根拠として用いた Evidence ID と当該 target の `target_coverage` 結果を、`oracle_presence` の非 `PASS` は違反した DA rule と根拠 span を引用する（本冊 §11.3）。判断記録（§8）を引用する場合は decision ID を示す。静的解析は正典レコードを持たないため監査レコード ID は引用しない（本冊 §7.1）。
- `chain_integrity` は repository-level の構造検査であり、`Structural checks` 配下に表示する。発見された各 Test の管理宣言解決と Test ID 大局的一意性をすべて評価し、未登録または不正対応の各 Test について adapter ID・source location・diagnostic code・判定値を `chain_integrity` 配下に列挙する（`MISMATCH`／診断 `MISSING`。本冊 §11.1.1）。covers する Test の無い leaf VO も `chain_integrity = MISMATCH`（診断 `MISSING`）として entity tree 上に示す。JSONでも同じ根拠一覧を返す。
- Evidenceが複数targetの計測結果を持つ場合、text reportはTest単位の集約値に加えて各targetの canonical Locator（本冊 §6.1.1）・result・countを子要素として表示する。JSONは本冊 §3.6のtarget別listを欠落なく返す。

各行のprefixは、その行の祖先に後続兄弟があれば `│  `、なければ空白3文字を階層ごとに連結し、
現在nodeが途中の兄弟なら `├─ `、最後の兄弟なら `└─ ` を付けて構成する。最上位nodeにも
同じ途中・末尾branch規則を適用する。祖先node自身の `├─ ` / `└─ ` を子孫行へ引き継がない。

#### `vtest report`

```text
vtest report [--doc DOC-X | --vo VO-X | --test TEST-X]
             [--items <check1,check2,...>] [--gate <name>]
             [--from <node>] [--view pm|tester|coder] [--depth <n>]
             [--direction up|down|both] [--format json]
```

`verify` と同じ集約を実行し、根拠（判断記録 ID・Evidence ID・DA rule 診断）を含む完全な詳細を出力する。`verify` が判定用、`report` が閲覧・提出用という役割分担とする。

- **役割別 projection**（本冊 §11.6、基本仕様 §19）：`--from <node>` は任意ノード（DOC / VO / TEST / SRC）からの局所トレースの起点、`--direction` は上流／下流／双方、`--depth` は連続追跡の段数、`--view` は役割 preset（`pm`＝上位 document・VO の状態と未確定/NG、`tester`＝VO・Test・検証対象・Evidence・未実施/失敗理由、`coder`＝実装から関連 Test・VO・上流 document へのトレース）である。役割を固定 enum として本冊は仕様化せず、preset・view 体系はここに委譲される（本冊 §11.6、基本仕様 §30 item21）。逆引きインデックス（VO → Tests、SRC → Tests、DOC → VOs、DOC → DOCs）を projection の基盤とする（本冊 §5.3）。
- **上流該当箇所の同伴**（本冊 §11.6・§3.1・§3.2、基本仕様 §11.1）：`--format json` の trace 出力に含まれる `derives_from` エッジ（DOC → DOC、DOC → VO）は、`anchor` と `note` を同伴する。エッジ要素は `{ "from": "DOC-REQ-001", "relation": "derives_from", "anchor": "§12.3", "note": "", "to": "VO-PARSER-UTF8-003" }` の形とし、`anchor` を持たない entry では `anchor` を省略または `null` とし空文字列で埋めない。`report --from DOC-REQ-001 --direction down --format json` は、この形式で「どの上流条項がどの概念（VO）へ対応するか」の対応ペア集合を返す。これが要求該当箇所と対応概念のペアの構造化出力であり、この用途に新規コマンド・ツールを設けない。`anchor` は不透明な文字列として transport し、文書内位置への解決・整合検査を行わない。
- **判断待ち section**（本冊 §11.7、基本仕様 §18.3）：`--format json` の出力へ、未確定・要判断事項を横断的に集約した `pending` section を含める（§12.4）。

#### `vtest mcp`

```text
vtest mcp
```

stdio で MCP サーバを起動する（§13）。

### 12.3 フェーズゲート評価（`verify --gate` / `report --gate`）

プロジェクト側が登録したフェーズ・工程・ゲートの進行条件について、現在の検証状態（5状態）と承認（§3.5）が通過条件を満たすかを**評価・提示できなければならない（MUST）**（本冊 §11.5、基本仕様 §20、要件定義 §26.4）。本システムの責務はゲート条件が現在満たされているかの評価・提示に限り、フェーズのライフサイクル管理・工程の自動遷移は責務外とする（「Release フェーズへ遷移させる」ではなく「Release gate の条件を現在満たしている」を提示する）。

- **ゲート定義**：`config.yaml` の `gates`（本冊 §2.2）に、ゲート名と進行条件（`require.verification`＝要求する検証結果、`require.approvals`＝要求する承認ロール集合）を保持する。
- **承認ロールの解決**（本別紙が新設する最小規則。基本仕様 §17・§30 item22 が別紙／プロジェクト設定へ委譲）：承認レコード（本冊 §3.5）は role field を持たないため、`config.yaml` に承認ロール → approver id 集合の対応を project 定義可能とする。

  ```yaml
  approval_roles:
    reviewer: [reviewer-agent-01, alice]
    owner:    [owner-human-01]
  ```

  ロール `R` の承認が存在するとは、「本冊 §3.5 で有効な（subject_hash・依存 closure が現在一致する）対象の承認レコードのうち、`approver.id` が `approval_roles[R]` に属するものが1件以上存在する」ことをいう。`gates.require.approvals` が参照するロールが `approval_roles` に無い場合は config invariant 違反として E-CONFIG-001 とする。
- **承認 subject の範囲**：ロール充足の判定対象は、当該 `verify` / `report` のエンティティ軸で指定した対象（`--doc` / `--vo` / `--test`。省略時は評価 scope の根エンティティ）に束縛された有効承認とする。scope 内に複数の対象がある場合は各対象について当該ロールの有効承認を要求する（fail-closed）。より細粒度の承認 authority・対象範囲はプロジェクト設定へ委譲する（基本仕様 §17・§30 item22）。
- **評価**：`vtest verify --gate <name>` は、指定ゲートの対象 scope について検証を実行し、(1) 検証結果が `require.verification`（例 `PASS`）を満たすか、(2) `require.approvals` の各ロールについて上記解決規則で有効な承認が存在するか、を評価して満否と根拠（不足している非 `PASS` 検査・未充足の承認ロール）を提示する。`report --gate` は同評価を JSON の `gate` section で返す。検証状態と承認は独立の軸であり、承認未充足は検証状態を降格させない（本冊 §3.5、基本仕様 §4.5）。
- 具体的なフェーズ名・承認ロール・必要承認数・権限 schema はプロジェクト設定（`config.yaml`）へ委譲する（基本仕様 §30 items 22-23）。

### 12.4 判断待ち情報 section（`verify` / `report` JSON）

導入時・検証時に生じる、未確定事項・不整合・未検証事項・機械的に確認済みの事実・外部判断が必要な事項を、機械可読な構造として `verify` / `report` の JSON 出力へ含める（本冊 §11.7、基本仕様 §18.3、要件定義 §17.3）。新規コマンド・ツールを増やさず、既存出力の section として露出する。

```json
"pending": [
  {
    "subject": "TEST-PARSER-044",
    "kind": "unknown",
    "check": { "item": "oracle_presence", "state": "UNKNOWN", "diagnostic": [] },
    "basis": [ { "kind": "da-rule", "ref": "DA-003", "note": "クロージャ内到達のため確定不能" } ],
    "bundle_ref": "cache/bundles/01J8XVYY.json"
  }
]
```

- `subject`：対象エンティティ ID または解決済み canonical Locator。
- `kind`：`unknown`（`UNKNOWN` によるエスカレーション）/ `unregistered`（管理宣言欠落）/ `unresolved`（参照解決不能）/ `undecided`（VO 未確定）/ `pending_approval`（承認待ち）。
- `check`：関係する4検査のいずれかと現在の検証状態・診断ラベル。
- `basis`：機械的に確認済みの事実（宣言鎖・検査結果・対象外とした範囲）への参照。
- `bundle_ref`：外部判断が必要な場合の判断バンドル（§8.1）への参照（任意）。

`UNKNOWN` だけでなく、検証出力全体にわたる未確定・要判断事項を横断的に集約する。

---

## 13. MCP ツール詳細仕様

### 13.1 共通仕様

- transport は stdio。`rmcp` で実装する。
- 各ツールの結果は CLI の `--format json` と同一の JSON 構造とする（検証状態 `state` と診断ラベル `diagnostic` の2軸を含む。§12.1）。
- エラーは MCP のツールエラーとして返し、`{ "code": "E-OP-001", "message": "...", "candidates": [...] }` の構造を含める。入力検証エラーには可能な限り `candidates` を含める（本冊 §6.3）。
- 各ツール呼び出しの冒頭で mtime ベースの再スキャン判定を行う（本冊 §2.3）。

### 13.2 ツール一覧

| ツール | 入力（主要フィールド） | 出力 |
|---|---|---|
| `scan` | なし | 診断一覧、エンティティ数サマリ |
| `doc_list` / `doc_get` | `id`（get のみ）、`tree: bool`、`roots: bool` | document レコード（木・根集合・鮮度） |
| `doc_upsert` | document フィールド一式（`path`、`derives_from[]`（`doc` + 任意 `anchor` + 任意 `note`）、`root: bool`、`update: bool`） | 作成・更新結果（依存判断・承認の失効警告を含む） |
| `vo_list` / `vo_get` | `id`、`doc`、`status` | VO レコード、derives_from（`doc` + 任意 `anchor` + 任意 `note`）、covers 状況、承認状態 |
| `vo_upsert` | VO フィールド一式（`derives_from[]` 必須1件以上（`doc` + 任意 `anchor` + 任意 `note`）、`dimensions[]`、`coverage_policy`、`combinations[]`（`explicit` のとき必須1件以上。各要素は dimension 名 → partition 値の map）） | 作成・更新結果（承認失効の警告含む） |
| `vo_expand` | `id`、`dry_run: bool` | 生成される子 VO 一覧 |
| `vo_approve` | `id`、`approver`、`judgment`（任意）、`basis[]`（任意） | 承認レコード ID |
| `test_query` | `vo` / `source` / `unregistered` のいずれか | Test 一覧 |
| `test_get` | `id` | Test 詳細（intent、covers、targets、位置、判断記録・Evidence 状態） |
| `form_get` | 大局的に一意な`kind` | owner adapterを明示したForm Schema（§14） |
| `test_create` | `form`、`answers`（オブジェクト）、`dry_run` | 生成された Test ID、挿入位置、diff |
| `test_edit` | `id`、`answers` または `set`、`body`、`dry_run` | 更新結果、diff |
| `audit_static` | `test` または `all` | rule 別 verdict（target-scoped な DA-002 / DA-003 は target 別 verdict を含む。本冊 §3.6・§7.2）と根拠 span。正典レコードは生成しない（本冊 §7.1） |
| `audit_bundle` | 対象 ID（`test` / `vo`）、`kind`（`test-semantic` / `impl-consistency`、任意） | bundle_id とバンドル本体（JSON） |
| `audit_submit` | 提出 JSON（本冊 §8.3） | 受理結果、判断記録 ID（`.verify/decisions/`）。受理は検証状態を昇格させない |
| `run_tests` | `test` / `vo` / `all`、`fast: bool` | Test ごとの結果と Evidence ID |
| `verify` | optional `items[]`（4検査の部分集合）、`doc` / `vo` / `test`、`gate`（任意）。items省略は固定4検査 | 総合 OK / NG、集約ツリー、`pending` section、`gate` 評価（指定時） |
| `report` | 同上＋ `from` / `view` / `depth` / `direction`。items省略は固定4検査 | 根拠付き完全レポート、projection、`pending` section |

`doc_upsert` / `vo_upsert` の `derives_from[]` 各要素は `doc`（必須）、`anchor`（任意）、`note`（任意）からなる。`anchor` は参照先 document 内の該当箇所を指す不透明な文字列であり、省略・空文字列を許容し `chain_integrity` 違反にしない（本冊 §3.1・§3.2）。CLI の `--anchor` と同じ値域・同じ扱いとし、文書内位置への解決・実在確認を行わない。

`vo_upsert` の `combinations[]` は本冊 §3.2.1 の `combinations` を desired state として与える。各要素は dimension 名 → partition 値の map（例 `{"operand-sign": "positive", "operator": "div"}`）で、`dimensions` に宣言された全軸をちょうど 1 回ずつ持つ。`coverage_policy` が `explicit` のときは 1 件以上を必須とし、`explicit` 以外のときは省略または空 list でなければならない。本冊 §3.2.1 の受理条件（`explicit` での欠落・空、`explicit` 以外での非空、未宣言 dimension、未列挙 partition、宣言 dimension の欠落・重複、重複 tuple、`dimensions` 空での `explicit`）に違反する入力は、`ok: false` と `{ "code": "E-SCAN-017", ... }` の tool error で拒否し、レコードを作成・更新しない。`vo_upsert` で `combinations` を省略した更新は既存値を保持し、空 list を明示した更新は既存値を空にする。`vo_expand` は不正 `combinations` の VO に対して同じ E-SCAN-017 で拒否し、子 VO を 1 件も生成しない。

`audit_static` は正典の監査レコード ID を返さない（再計算派生。本冊 §7.1）。`audit_submit` の受理結果は判断記録 ID であり、これは検証状態を変えない（本冊 §8.3）。旧モデルの `spec_list` / `spec_get` / `req_list` / `req_get` / `req_upsert` は廃止し、`doc_*` へ統合した。

### 13.3 エージェント向け利用フロー（参考）

各操作は、CLIとMCPで同じadapter registryを解決する。
フォーム、監査、実行の入力に含まれるadapter namespaceはopaque値として扱い、
未登録adapterや未提供capabilityをRust用の既定値へ暗黙変換しない。

```text
Coder AI がテストを追加する典型フロー：

form_get(kind: rust-unit-function)
  → test_create(answers, dry_run: true)   # 検証と diff 確認
  → test_create(answers)                  # 挿入
  → （関数本体を実装：test_edit の body、または直接編集）
  → audit_static(test)                    # 決定論的な不成立検出（再計算派生）
  → audit_bundle(kind: test-semantic, test)
  → （エージェント自身が判定）
  → audit_submit(result)                  # 判断記録に保存（検証状態は昇格しない）
  → run_tests(test)
  → verify(test)                          # 自タスクの完了確認
```

`audit_submit` は `UNKNOWN` に対する外部判断を記録するだけで、`oracle_presence` 等の検証状態を `PASS` へ昇格させない（本冊 §8、基本仕様 §11.3）。完了確認は `verify` の4検査で行う。

---

## 14. Form Schema 設計

### 14.1 スキーマ形式（`.verify/forms/<kind>.yaml`）

次は`rust-cargo` adapterが登録するForm Schemaである。coreは`fn_name`、`.rs`、Rust構文をForm Schemaの共通fieldとして要求しない。

```yaml
kind: rust-unit-function
adapter: rust-cargo
title: Rust 関数単体テスト
fields:
  - name: target
    question: 対象ソースシンボルは？
    type: symbol            # symbol | vo-ref | vo-ref-list | test-ref |
                            # enum | string | ident | path
    required: true
    validate: [symbol-exists]
  - name: covers
    question: どの VO を検証しますか？
    type: vo-ref-list
    required: true
    validate: [vo-exists]
  - name: behavior
    question: どの振る舞いを検証しますか？
    type: string
    required: true
  - name: test_kind
    question: テスト種別は？
    type: enum
    options: [normal, error, boundary, regression]
    required: true
  - name: input
    question: 入力条件は？
    type: string
    required: true
  - name: expect
    question: 期待結果は？
    type: string
    required: true
    validate: [enum-variant-exists]   # best effort（本冊 §6.3）
  - name: fn_name
    question: テスト関数名は？
    type: ident
    required: true
    validate: [unique-fn-name]
  - name: file
    question: 追加先ファイルは？（省略可）
    type: path
    required: false
    validate: [rust-file]
template: |
  /// @vtest.id {test_id}
  /// @vtest.covers {covers}
  /// @vtest.target {target}
  /// @vtest.intent {behavior}
  /// @vtest.input {input}
  /// @vtest.expect {expect}
  /// @vtest.kind unit-{test_kind}
  #[test]
  fn {fn_name}() {
      todo!("implement test body")
  }
```

`test_kind` の `regression` は Test の**意図ラベル**（`@vtest.kind` の値）であり、廃止された存在理由分類（role / anchor）とは別概念である（本冊 §4.1・§4.2）。組込 Form は `role` を宣言せず、`kind` の値に regression を含む Test（`unit-regression` 等）も `kind` から存在理由分類を導出しない。

### 14.2 検証器

| validate | 内容 | 失敗時 |
|---|---|---|
| `symbol-exists` | Target Reference解決を対応adapterへ委譲（本冊 §6.1） | E-OP-001＋候補 |
| `vo-exists` / `test-exists` | エンティティ存在確認 | E-OP-001 |
| `enum-variant-exists` | `rust-cargo` adapterが`Type::Variant`形式の場合のみAST検索。解決不能な自由記述は受理 | E-OP-001＋候補 |
| `unique-fn-name` | `rust-cargo` adapterが挿入先モジュール内で関数名重複を確認 | E-OP-001 |
| `rust-file` | `rust-cargo` adapterが`.rs`ファイルがscan対象内に存在することを確認 | E-OP-001 |

`required` を欠く回答、未知のフィールド名は E-OP-001 とする。
Test ID は `--id` による明示指定がなければ、`TEST-<領域>-<連番>`（領域は covers 先 VO の ID から継承、連番は既存最大＋1）で自動採番し、結果に含めて返す。

`kind`は`[a-z0-9][a-z0-9-]*`のcase-sensitive文字列で、`.verify/forms/<kind>.yaml`のファイル名と一致するrepository-globalなForm ID、`adapter`はそのFormを処理するStructured Test adapter IDである。registryはbuilt-inとuser-defined Formを統合し、同じkindの重複、schemaのadapterとregistry ownerの不一致、未知adapter、Structured Test capability欠落を拒否する。`adapter`を欠く読取り互換Formは、登録済みadapterのbuilt-in kind宣言またはschemaを検査するcompatibility matcherのうちちょうど1件だけが受理する場合に限って解決し、0件または複数件なら拒否する。matcherはschema内容から決定論的に判定し、kind名だけでRust用と推測しない。readerは互換解決だけでFormファイルを書き換えない。

### 14.3 組込フォーム

組込フォームは `rust-cargo` adapterが提供する。コアはフォームのkindを
Rust固有と推測せず、schemaの`adapter`、registryが宣言する大局的に一意なkind ownership、登録済みcapabilityを照合する。未提供の
Structured Test capabilityはE-ADAPTER-004として作成・編集を中止し、ファイルを変更しない。

次の2種を組込Formとして同梱する。

- `rust-unit-function`（§14.1）
- `rust-integration`：単一の`target` fieldに代えて、1件以上のロケータを持つ`targets`を必須入力として受け取る。また`file`はrequired:trueとする — Integration Testの配置先（test suite location）はSource Targetのlocationとは別概念であり、targetsから一意に導出できないためである。将来、Test Suiteまたは同等の配置概念が第一級化され配置先を一意に導出できる規則が導入された場合にのみ、省略可能性を再検討する。§14.1との差分はこの2点であり、他は同一。

`rust-integration`は`targets`の全要素を入力順に個別の`@vtest.target`行として出力する。
空listと重複targetをE-OP-001で拒否する。`target`キーはintegration種別に限り複数行を許容する
（本冊 §4.2の例外）。先頭以外のtargetを`@vtest.related`へ変換しない。

### 14.4 テスト種別ごとのフォーム拡張

Form Schema はユーザー定義可能とし、大局的に一意な`kind`と登録済みStructured Test adapterの`adapter` IDを必須とする（本冊 §4.1・§5.2、基本仕様 §15.4）。`fields` の追加・変更で API Test・CLI Test 等の質問列を定義できる（要件定義の質問テンプレート構想に対応）。

- partition・境界値を必須入力とする種別は、該当フィールドに `required: true` を設定することで表現する（基本仕様 §15.4）。境界値・partition の必須入力化は組込 Form では設けず、user-defined Form Schema が指定できる。
- 他 field の回答値によって `required` が変わる cross-field 制約は導入しない。Form Schema の検証は単一 field の `required` と検証器だけで閉じる。
- すべての管理対象 Test に `covers ≥ 1` を一律要求するため（本冊 §4.1、基本仕様 §12）、user-defined Form も `covers` を `required: true` の `vo-ref-list` として持つ。本 version は role / anchor / anchor_rationale による存在理由分類・固定 Form 群を持たず、`covers` 件数の可変制約も設けない。

これらの Form も `kind` と owner `adapter` ID を宣言する通常の Form Schema であり、kind の大局的一意性と owner 解決の規則（§14.2、基本仕様 §15.4）は変わらない。

---

## 15. Structured Test Operation adapter contract

Structured Editの構文解析・再生成・selector解釈は対応adapterが所有する。
orchestrationはTest IDとadapter IDで対象を一意に選択し、adapterが返す拡張範囲を
単一置換として適用する。production adapterとして提供するのは `rust-cargo` だけである。
§15.1〜§15.4は`rust-cargo` StructuredTestAdapterの構文処理を定める。

### 15.1 `rust-cargo` 対象の特定

Test ID から編集対象を特定する。

```text
TEST-X → スキャン結果 → SourceLocation
  （ファイル、関数アイテムの byte range、
    doc comment 開始位置を含む拡張 range）
```

スキャン結果が古い可能性があるため、編集直前に対象ファイルのみ再パースし、Test ID の位置を再確認する。
再確認で見つからない場合は E-OP-002。

### 15.2 `rust-cargo` 編集・挿入の適用

**Edit（既存 Test の更新）**

1. desired state（answers / set / body）から、あるべきアノテーションブロックと関数シグネチャ・本体を生成する。
2. 現状とあるべき状態の diff を計算する。
3. 変更を、対象テスト関数の拡張 range（doc comment 先頭〜関数末尾）の**単一置換**として適用する。
4. 適用後、対象ファイルを再パースし、次を確認する。
   - 構文的に妥当である
   - 対象 Test のアノテーションが desired state と一致する
   - 他の Test エンティティのソーステキストが変化していない
5. 確認に失敗した場合はファイルを元へ戻し、E-OP-003 を返す。

**Create（新規 Test の生成・挿入）**

挿入後の再パース検証とロールバックは Edit と同一の規則で Create にも適用する。Create 経路にだけ検証を省く分岐を設けない（基本仕様 §15.1）。

1. Form 回答（§14）から、あるべきアノテーションブロックと関数シグネチャ・本体、および挿入位置を決定する。回答自体の検証エラーは E-OP-001（候補付き。本冊 §6.3）。
2. 挿入前の対象ファイルの内容を保持する。対象ファイルが存在しない場合は「不存在」を挿入前の状態として保持する。
3. 生成した Test construct を挿入位置へ**単一挿入**として適用する。
4. 適用後、対象ファイルを再パースし、次を確認する。
   - 構文的に妥当である
   - 挿入した Test construct がちょうど 1 件の Test エンティティとして認識される
   - その Test のアノテーションが Form 回答から導いた desired state と一致し、Test ID が回答どおりである
   - 挿入した Test 以外の Test エンティティのソーステキストが変化していない
   - 挿入した Test 以外のソーステキスト（helper・fixture・通常コード）が変化していない
5. 確認のいずれかに失敗した場合は、適用前の状態へ復元し（挿入によりファイルが新規作成された場合は不存在へ戻す）、E-OP-003 を返す。ロールバック後は、当該操作より前と同じソーステキストが観測できなければならない。部分適用された挿入内容を残さない。
6. `--dry-run` は 1〜3 の生成結果と挿入位置のみを提示し、ファイルを変更しない。

Create / Edit いずれも、E-OP-003 で中止した操作は Test ID の採番・Evidence・判断記録を含む副産物をひとつも残さない。ロールバック後の再スキャンで、当該操作が無かった場合と同一のエンティティ集合・内容ハッシュが得られる。

### 15.3 `rust-cargo` annotation blockの再生成

アノテーションは常にキー順（id, covers, target, intent, input, expect, kind, case, related）で再生成し、`@vtest.` を含まない自由記述の doc comment 行は元の位置関係を保って温存する。これにより、Structured Edit を繰り返しても差分が安定する。この再生成規則は Create が挿入する annotation block にも同一に適用する。同一の desired state からは Create / Edit のいずれの経路でも同一の annotation block を生成し、Create 直後に同じ desired state で Edit しても差分を生じない。

このキー集合は本冊 §4.2 の test-key（`id` / `covers` / `target` / `intent` / `input` / `expect` / `kind` / `case` / `related`）と一致する。本 version は存在理由分類（旧 `role` / `anchor` / `anchor-rationale`）のキーを持たず、再生成でも出力しない。`@vtest.src-id` は Test construct ではなく対象実装側の関数に付与するキーであり（本冊 §4.2 の source-target-key）、Test annotation block の再生成対象に含めない。

### 15.4 `rust-cargo` 1 Test境界の保証

置換範囲が単一のテスト関数の拡張 range に限られることを、適用前（範囲計算）と適用後（他 Test のハッシュ不変確認）の二重で検査する。
`edit TEST-001` は他のTestへ影響しない（要件定義 §16、基本仕様 §15.3）。

helper・fixture・通常ソースコードの編集手段は提供しない（要件定義 OOS-003、基本仕様 §15.3）。
関数本体が helper を必要とする場合、helper の作成は通常のソース編集として利用者（人間・AI）が行う。

---

## 付記（非規範）: トレーサビリティ表

本表は別紙A の各節が実現する上流§（要件定義＝WHY / 基本仕様＝WHAT / 詳細設計本冊＝HOW 中核）と、その導出区分（CONFORM＝旧版から生存し引用・項目名・連番の修復のみ／再導出＝旧構造を凍結モデルへ書き換え／新設＝旧版に無く上流から新規）を記録する。全節が上流へトレースでき、親を持たない節を作らないことを設計制約とする。

| 別紙A の節 | 実現する上流§ | 区分 |
|---|---|---|
| §12.1 共通仕様 | 本冊§17.2・§5.2・§4.1／基本§4.1・§4.2・§22.3／要件§4.1 | 再導出（role/anchor 宣言逐語・実効 field 段落を除去、targets≥1 を adapter 層件数へ、状態5＋診断ラベル4の2列を明示） |
| §12.2 `vtest init` | 本冊§2.1・§2.2／基本§26.1 | CONFORM（生成物に doc/・decisions/ を反映） |
| §12.2 `vtest scan` | 本冊§5・§5.6／基本§23・§26.1 | CONFORM（整合性検査を chain_integrity/orphan_detection へ言換え） |
| §12.2 `vtest doctor` | 本冊§16.2／基本§26.1 | CONFORM（失効を判断記録・承認のハッシュ束縛 STALE へ） |
| §12.2 `vtest doc add/list/show` | 本冊§3.1・§5.6・§11.4／基本§3.1・§3.2・§16・§26.1・§30 item1-2 | 新設（spec/req コマンドを廃し doc へ統合、derives_from・根指定フラグ） |
| §12.2 `vtest vo …` | 本冊§3.2・§3.2.1・§3.5／基本§10・§17・§26.1 | 再導出（--req/--spec/--section→--derives-from、承認 basis→judgment_ref+basis） |
| §12.2 `vtest test create` | 本冊§4.2・§6.3／基本§15・§26.1 | CONFORM（回答例の role 不在） |
| §12.2 `vtest test edit` | 本冊§15／基本§15.1・§26.1 | CONFORM（desired state 参照を基本§15.1 へ） |
| §12.2 `vtest test show/list/query` | 本冊§4.1・§5.3・§11.6／基本§12・§26.1 | 再導出（show の role 除去、covers/targets 表示、逆引きを projection 基盤へ） |
| §12.2 `vtest audit static` | 本冊§7・§7.1・§7.2・§3.6／基本§5.4・§5.5・§26.1 | 再導出（正典監査レコード廃止＝再計算派生、stdout/cache のみ） |
| §12.2 `vtest audit bundle/submit` | 本冊§8・§3.4・§8.1・§8.3・§8.4・§8.6／基本§11・§11.3・§26.1 | 再導出（意味検査→判断記録、kind を UNKNOWN 質問ラベルへ、非昇格明記、判断/承認面分離） |
| §12.2 `vtest run` | 本冊§9・§9.1・§10.3／基本§21・§26.1 | CONFORM（--req 除去、--fast を target_coverage 非計測へ） |
| §12.2 `vtest verify` | 本冊§11.1・§11.3・§2.2・§11.5・§11.7／基本§5・§4.6・§22.1・§26.1 | 再導出（12項目→4検査、--req 除去、--doc/--gate、full_scope invariant、状態5＋診断ラベルのツリー） |
| §12.2 `vtest report` | 本冊§11.3・§11.6・§11.7／基本§19・§22.3・§26.1・§30 item21 | 再導出（4検査、projection view/from、pending section） |
| §12.2 `vtest mcp` | 本冊§13／基本§26.2 | CONFORM |
| §12.3 フェーズゲート評価 | 本冊§11.5・§2.2・§3.5／基本§20・§17・§30 item22-23／要件§26.4 | 新設（MUST、承認ロール解決規則 approval_roles を新設、評価/提示のみ） |
| §12.4 判断待ち情報 section | 本冊§11.7／基本§18.3・§30 item19／要件§17.3 | 新設（verify/report JSON の pending section） |
| §13.1 MCP 共通仕様 | 本冊§13／基本§26.2 | CONFORM（状態/診断2軸を反映） |
| §13.2 ツール一覧 | 本冊§8・§7.1・§11.1／基本§26.2 | 再導出（spec_/req_→doc_、verify/report 固定4検査、audit_static はレコードID返さず、audit_submit は非昇格） |
| §13.3 エージェント向けフロー | 本冊§8／基本§11・§25 | CONFORM（doc/判断記録・非昇格を明示） |
| §14.1 スキーマ形式 | 本冊§4.1・§4.2・§6.3／基本§15.4・§30 item4 | CONFORM（test_kind regression を意図ラベルとして明示、role 不在） |
| §14.2 検証器 | 本冊§6.1・§6.3・§5.2／基本§15.4 | CONFORM（Form owner 解決参照を基本§15.4 へ） |
| §14.3 組込フォーム | 本冊§4.2・§4.3／基本§15.4・§27 | CONFORM |
| §14.4 テスト種別ごとの拡張 | 本冊§4.1・§5.2／基本§15.4・§12 | 再導出（role/anchor 固定 Form 群を全廃し、user-defined Form・partition required・cross-field 禁止・covers≥1 のみ残置） |
| §15.1 対象の特定 | 本冊§15／基本§15.3 | CONFORM |
| §15.2 編集の適用 | 本冊§15／基本§15.1・§15.3 | CONFORM |
| §15.3 annotation 再生成 | 本冊§4.2／基本§15.3 | 再導出（キー順から role/anchor/anchor-rationale を除去、本冊 §4.2 test-key と一致） |
| §15.4 1 Test 境界の保証 | 要件§16・OOS-003／基本§15.3 | CONFORM |
