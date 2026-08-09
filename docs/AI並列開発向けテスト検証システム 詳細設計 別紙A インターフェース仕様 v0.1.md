# AI並列開発向けテスト検証システム 詳細設計 別紙A インターフェース仕様 v0.1

本冊 §0 の分冊構成に基づき、本別紙は §12〜§15 を収録する。
参照規則・診断コード・終了コードは本冊 §17 に従う。

---

## 12. CLI 詳細仕様

### 12.1 共通仕様

- すべてのコマンドは非対話で完結する。確認プロンプトを出す場合は `--yes` で抑止できる。
- 出力は既定で人間向けテキスト、`--format json` で機械可読 JSON。
- JSON 出力は最上位に `{ "ok": bool, "data": ..., "diagnostics": [...] }` を持つ。`diagnostics` の要素は `{ "code": "E-SCAN-002", "severity": "error", "message": "...", "location": ... }`。
- 終了コードは本冊 §17.2 に従う。
- グローバルオプション：`--project <dir>`（プロジェクトルート。既定はカレントから `.verify/` を上方探索）、`--format <text|json>`、`--quiet`。

CLIの操作は登録済みadapter registryを通じて実装を選択する。
JSON envelope、adapter選択エラー、capability不足の非PASS扱いはMCPと共通であり、
CLIだけがRust固有の既定値へフォールバックしてはならない。

Testを含むJSONは本冊 §5.2の `execution` と互換field `filter`、`package`、
`test_target` を返す。Test入力から `execution` を復元できるのは、
完全で相互整合するRust互換実行座標がある場合だけである。

明示操作に必須のadapter capabilityが未提供なら、`ok: false`、E-ADAPTER-004、終了コード2を返す。
create / editではファイルを変更せず、auditではAudit Recordを、runではEvidenceを生成しない。
検証・reportで能力不足を観測した場合はW-ADAPTER-101と能力別の非PASS値を返す。

### 12.2 コマンド仕様

#### `vtest init`

```text
vtest init [--name <project-name>]
```

`.verify/` 一式（本冊 §2.1）を生成する。`config.yaml` は本冊 §2.2のversion 2で、
組込 `rust-cargo` adapter namespaceを含む。既存の `.verify/` があればエラー（終了コード 2）。

#### `vtest scan`

```text
vtest scan
```

スキャンと整合性検査（本冊 §5）を実行し、診断一覧とエンティティ数のサマリを出力する。
error 診断があれば終了コード 1。

#### `vtest doctor`

`vtest scan` と同一処理の別名。自動化環境の整合性検査に使用する（本冊 §16.2）。

#### `vtest spec add / list / show`

```text
vtest spec add --id SPEC-BASIC-001 --path docs/basic-spec.md
               --kind document [--title <t>] [--update]
vtest spec list
vtest spec show SPEC-BASIC-001
```

`add` は対象ファイルの sha256 を計算して SPEC レコードを作成する。
`--update` は既存レコードの sha256 を現ファイルで更新する（依存する監査・承認が失効する旨を出力する。本冊 §11.4）。

#### `vtest req add / edit / list / show`

```text
vtest req add --id REQ-PARSER-001 --summary <s>
              [--parent REQ-X] [--spec SPEC-X --section <sec>]...
vtest req edit REQ-PARSER-001 [--summary <s>] [--parent ...] ...
vtest req list [--tree]
vtest req show REQ-PARSER-001
```

#### `vtest vo add / edit / list / show / expand / approve`

```text
vtest vo add --id VO-X --claim <c> --req REQ-X
             [--parent VO-Y] [--spec SPEC-X --section <sec>]...
             [--dimension <name>=<p1>,<p2>...]... [--policy <policy>]
vtest vo edit VO-X [--claim ...] ...
vtest vo list [--tree] [--req REQ-X] [--status draft|approved]
vtest vo show VO-X          # claim、covers している Test、監査・承認状態を表示
vtest vo expand VO-X [--dry-run]
vtest vo approve VO-X --approver-kind <human|agent> --approver-id <id>
                 [--model <m>] [--basis <audit-record-id>]...
```

`expand` は本冊 §3.3.1 の実体化。`--dry-run` は生成予定の子 VO 一覧のみ表示する。
`approve` は現在の VO 内容ハッシュに束縛された承認レコードを追加する。
`edit` は承認済 VO に対して警告を出す（編集自体は許可し、承認はハッシュ不一致で自動失効する）。

#### `vtest test create`

```text
vtest test create --form rust-unit-function
                  --answers answers.yaml [--dry-run]
```

Form Schema（§14）に基づく回答ファイルを受け取り、検証のうえテスト雛形＋アノテーションを生成して挿入する。
`--dry-run` は挿入内容と挿入位置のみを表示する。
回答の検証エラーは E-OP-001 として候補付きで報告する（本冊 §6.2）。

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

desired state 方式（基本仕様 §8.2）。
`--answers` は完全なあるべき状態、`--set` は指定フィールドのみのあるべき値を宣言する。
編集の実装は §15。関数本体の書き換えは `--body-file <path>` で本体全文を与える。

#### `vtest test show / list / query`

```text
vtest test show TEST-X        # intent、covers、target、位置、監査・Evidence 状態
vtest test list [--vo VO-X] [--unregistered]
vtest test query --source src/parser.rs::Parser::parse   # SRC からの逆引き
```

#### `vtest audit static`

```text
vtest audit static [--test TEST-X | --all]
```

決定論的監査（本冊 §7）を実行し、監査レコード（kind: static）を保存する。

#### `vtest audit bundle / submit`

```text
vtest audit bundle --kind test-semantic --test TEST-X [--include-failed]
vtest audit bundle --kind vo-coverage  (--vo VO-X | --req REQ-X)
vtest audit bundle --kind impl-consistency (--test TEST-X | --vo VO-X)
vtest audit submit --file result.json
```

`bundle` はバンドル JSON のパスと bundle_id を出力する。
`submit` は本冊 §8.4 の検証を行い、受理時に監査レコード ID を出力する。

#### `vtest run`

```text
vtest run (--test TEST-X | --vo VO-X | --req REQ-X | --all) [--fast]
```

テスト実行と Evidence 記録（本冊 §9、§10）。
`--fast` は cargo test のみ（target_execution 非計測）。

#### `vtest verify`

```text
vtest verify [--items <item1,item2,...>]
             [--req REQ-X | --vo VO-X | --test TEST-X]
             [--summary]
```

集約（本冊 §11.3）を実行し、OK / NG を返す。
`--items` 省略時は config の `full_scope`（完全検証）。
`--summary` は総合 OK / NG と非 PASS 件数のみを出力する。
scope を限定した場合、出力冒頭に要求 scope と「scope 外は未検証」の旨を必ず表示する（基本仕様 §4.4）。

出力例（テキスト）：

```text
Requested scope: full (11 items), REQ-PARSER-001

└─ REQ-PARSER-001                       NG
   ├─ spec_coverage                     PASS
   ├─ VO-PARSER-UTF8                    NG
   │  ├─ vo_coverage                    PASS  (audit 01J8XV..., approved)
   │  ├─ VO-PARSER-UTF8-003             NG
   │  │  ├─ test_existence              PASS
   │  │  ├─ TEST-PARSER-044             NG
   │  │  │  ├─ static_audit             PASS
   │  │  │  ├─ semantic_audit           FAIL  (audit 01J8XW...)
   │  │  │  ├─ test_execution           PASS
   │  │  │  ├─ runtime_result           PASS
   │  │  │  ├─ target_execution         PASS  (count=3)
   │  │  │  └─ evidence_validity        PASS
   │  │  └─ ...
   │  └─ VO-PARSER-UTF8-004             MISSING (no covering test)
   └─ ...

Result: NG
```

各行のprefixは、その行の祖先に後続兄弟があれば `│  `、なければ空白3文字を階層ごとに連結し、
現在nodeが途中の兄弟なら `├─ `、最後の兄弟なら `└─ ` を付けて構成する。最上位nodeにも
同じ途中・末尾branch規則を適用する。祖先node自身の `├─ ` / `└─ ` を子孫行へ引き継がない。

#### `vtest report`

```text
vtest report [--req REQ-X | ...] [--format json]
```

`verify` と同じ集約を実行し、根拠（監査レコード ID・Evidence ID・診断）を含む完全な詳細を出力する。
`verify` が判定用、`report` が閲覧・提出用という役割分担とする。

#### `vtest mcp`

```text
vtest mcp
```

stdio で MCP サーバを起動する（§13）。

---

## 13. MCP ツール詳細仕様

### 13.1 共通仕様

- transport は stdio。`rmcp` で実装する。
- 各ツールの結果は CLI の `--format json` と同一の JSON 構造とする。
- エラーは MCP のツールエラーとして返し、`{ "code": "E-OP-001", "message": "...", "candidates": [...] }` の構造を含める。入力検証エラーには可能な限り `candidates` を含める（本冊 §6.2）。
- 各ツール呼び出しの冒頭で mtime ベースの再スキャン判定を行う（本冊 §2.3）。

### 13.2 ツール一覧

| ツール | 入力（主要フィールド） | 出力 |
|---|---|---|
| `scan` | なし | 診断一覧、エンティティ数サマリ |
| `spec_list` / `spec_get` | `id`（get のみ） | SPEC レコード |
| `req_list` / `req_get` | `id`、`tree: bool` | REQ レコード（木） |
| `req_upsert` | REQ フィールド一式 | 作成・更新結果 |
| `vo_list` / `vo_get` | `id`、`req`、`status` | VO レコード、covers 状況、承認状態 |
| `vo_upsert` | VO フィールド一式 | 作成・更新結果（承認失効の警告含む） |
| `vo_expand` | `id`、`dry_run: bool` | 生成される子 VO 一覧 |
| `vo_approve` | `id`、`approver`、`basis[]` | 承認レコード ID |
| `test_query` | `vo` / `source` / `unregistered` のいずれか | Test 一覧 |
| `test_get` | `id` | Test 詳細（intent、位置、監査・Evidence 状態） |
| `form_get` | `kind` | Form Schema（§14） |
| `test_create` | `form`、`answers`（オブジェクト）、`dry_run` | 生成された Test ID、挿入位置、diff |
| `test_edit` | `id`、`answers` または `set`、`body`、`dry_run` | 更新結果、diff |
| `audit_static` | `test` または `all` | ルール別結果、監査レコード ID |
| `audit_bundle` | `kind`、対象 ID | bundle_id とバンドル本体（JSON） |
| `audit_submit` | 提出 JSON（本冊 §8.3） | 受理結果、監査レコード ID |
| `run_tests` | `test` / `vo` / `req` / `all`、`fast: bool` | Test ごとの結果と Evidence ID |
| `verify` | `items[]`、`req` / `vo` / `test` | 総合 OK / NG、集約ツリー |
| `report` | 同上 | 根拠付き完全レポート |

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
  → audit_static(test)
  → audit_bundle(kind: test-semantic, test)
  → （エージェント自身が判定）
  → audit_submit(result)
  → run_tests(test)
  → verify(test)                          # 自タスクの完了確認
```

---

## 14. Form Schema 設計

### 14.1 スキーマ形式（`.verify/forms/<kind>.yaml`）

```yaml
kind: rust-unit-function
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
    validate: [enum-variant-exists]   # best effort（本冊 §6.2）
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

### 14.2 検証器

| validate | 内容 | 失敗時 |
|---|---|---|
| `symbol-exists` | ロケータ解決（本冊 §6.1） | E-OP-001＋候補 |
| `vo-exists` / `test-exists` | エンティティ存在確認 | E-OP-001 |
| `enum-variant-exists` | `Type::Variant` 形式の場合のみ AST 検索。解決不能な自由記述は受理 | E-OP-001＋候補 |
| `unique-fn-name` | 挿入先モジュール内での関数名重複確認 | E-OP-001 |
| `rust-file` | `.rs` ファイルがスキャン対象内に存在 | E-OP-001 |

`required` を欠く回答、未知のフィールド名は E-OP-001 とする。
Test ID は `--id` による明示指定がなければ、`TEST-<領域>-<連番>`（領域は covers 先 VO の ID から継承、連番は既存最大＋1）で自動採番し、結果に含めて返す。

### 14.3 組込フォーム

組込フォームは `rust-cargo` adapterが提供する。コアはフォームのkindを
Rust固有と推測せず、adapter namespaceと登録済みschemaを照合する。未提供の
Structured Test capabilityはE-ADAPTER-004として作成・編集を中止し、ファイルを変更しない。

次の2種を組込Formとして同梱する。

- `rust-unit-function`（§14.1）
- `rust-integration`：`target` を必須とせず（結合テストでは単一シンボルに限定できない場合がある）、代わりに `targets`（複数ロケータ）を受け取る。他は同一

`@vtest.target` が必須アノテーションであるため、`rust-integration` では `targets` の先頭を `@vtest.target` に、残りを `@vtest.related` ではなく追加の `@vtest.target` 行として出力する（`target` キーは integration 種別に限り複数行を許容する。本冊 §4.1 の例外として実装する）。

### 14.4 テスト種別ごとのフォーム拡張

Form Schema はユーザー定義可能とし、`fields` の追加・変更でAPI Test・CLI Test等の質問列を定義できる（要件定義の質問テンプレート構想に対応）。
partition・境界値を必須入力とする種別は、該当フィールドに `required: true` を設定することで表現する（基本仕様 §15 の項目16）。

---

## 15. Structured Edit の実装

Structured Editの構文解析・再生成・selector解釈は対応adapterが所有する。
orchestrationはTest IDとadapter IDで対象を一意に選択し、adapterが返す拡張範囲を
単一置換として適用する。production adapterとして提供するのは `rust-cargo` だけである。

### 15.1 対象の特定

Test ID から編集対象を特定する。

```text
TEST-X → スキャン結果 → SourceLocation
  （ファイル、関数アイテムの byte range、
    doc comment 開始位置を含む拡張 range）
```

スキャン結果が古い可能性があるため、編集直前に対象ファイルのみ再パースし、Test ID の位置を再確認する。
再確認で見つからない場合は E-OP-002。

### 15.2 編集の適用

1. desired state（answers / set / body）から、あるべきアノテーションブロックと関数シグネチャ・本体を生成する。
2. 現状とあるべき状態の diff を計算する。
3. 変更を、対象テスト関数の拡張 range（doc comment 先頭〜関数末尾）の**単一置換**として適用する。
4. 適用後、対象ファイルを再パースし、次を確認する。
   - 構文的に妥当である
   - 対象 Test のアノテーションが desired state と一致する
   - 他の Test エンティティのソーステキストが変化していない
5. 確認に失敗した場合はファイルを元へ戻し、E-OP-003 を返す。

### 15.3 アノテーションブロックの再生成

アノテーションは常にキー順（id, covers, target, intent, input, expect, kind, case, related）で再生成し、`@vtest.` を含まない自由記述の doc comment 行は元の位置関係を保って温存する。
これにより、Structured Edit を繰り返しても差分が安定する。

### 15.4 1 Test 境界の保証

置換範囲が単一のテスト関数の拡張 range に限られることを、適用前（範囲計算）と適用後（他 Test のハッシュ不変確認）の二重で検査する。
`edit TEST-001` は他のTestへ影響しない（要件定義 §21）。

helper・fixture・通常ソースコードの編集手段は提供しない（要件定義 OOS-003）。
関数本体が helper を必要とする場合、helper の作成は通常のソース編集として利用者（人間・AI）が行う。
