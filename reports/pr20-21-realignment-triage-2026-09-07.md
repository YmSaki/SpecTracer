# PR #20 / #21 の正本再整合 — 仕分け

作成 2026-09-07 / 正本 `docs/canonical/specification.json`（ブランチ `spec/upstream-traceability-audit`、コミット `e04b6b9`）
対象 PR #20 `feature/v01-canonical-model`（base `develop`）／ PR #21 `feature/v01-canonical-store`（base `feature/v01-canonical-model`）

**本書は仕分けである。実装も `docs/` の変更も行っていない。ブランチも切り替えていない。**

---

## 0. 凡例と、固定した仕分けの枠

### 0.1 対象・目的・軸・除外

- **対象**: #20 が `crates/vtest-model` へ入れた型・関数・テスト群と、#21 が `crates/vtest-store` へ入れた
  モジュール・関数・テスト群。#21 の差分は `git diff feature/v01-canonical-model...feature/v01-canonical-store`
  で取り、#21 が触っていない predecessor コード（`records.rs` の `SpecRecord` / `ReqRecord` /
  `AuditRecord` 等）は §3 の末尾で一括して言及するだけにし、仕分け表には入れない。
- **目的**: 正本の Document モデル再整合（Issue #14）後に、各単位が「そのまま使えるか／直すか／消すか／足りないか」を決める。
- **軸**: 各行に KEEP（流用可）/ REWRITE（書き換え）/ DROP（捨てる）のいずれかを付け、根拠として正本ノード id を挙げる。
  正本に該当する明文が無い単位は **NO-NORM** と書く（推測で規範を埋めない）。不足は §4 に別立てする。
- **除外**: PR の品質・テスト網羅・コーディングスタイルは見ない。`.verify/doc/` の保存形式（1 ファイルか層別配列か）は決めない（§6）。

### 0.2 判定値

| 判定 | 意味 |
|---|---|
| KEEP | 正本ノードと一致する。そのまま次の PR へ持ち越せる。 |
| REWRITE | 概念は正本にあるが、形（field・variant・名前・計算規則）が違う。 |
| DROP | 正本に対応する概念が無い、または正本が明示的に否定している。 |
| NO-NORM | 正本に明文が無い。実装裁量として残すか、上流へ問うかは本書では決めない。 |

### 0.3 計数の単位

**モジュール／型／関数／テスト群**を 1 単位とする。1 つの型の中の複数 field は、判定が分かれる場合だけ行を分ける。
この単位は途中で変えていない。

---

## 1. 正本の Document モデル（逐語）

### 1.1 ノードの形

> **DES-568**: ノードは、文ノードが `id` / `statement` / `derives_from` / `source` を、節ノードが `id` / `title` / `source` を、根ノードが `id` / `statement` / `source` を必須 field として持つ。

> **DS-1592**: 節ノードは必須 field `title` を持ち、文ノードは `title` を持たない。

> **DS-1593**: 文ノードの `derives_from` は必須 field で 0 件を許容し、節ノードの `derives_from` は任意 field である。

> **DES-573**: ノードは登録時の内容ハッシュを field として保持せず、subject hash を §1.3 に従って計算する。

> **BD-312**: 上流文書は層ごとのトップレベル配列に分けたノードで表現する。

> **BD-164**: 要件定義・基本仕様・詳細設計・API Schema・Protocol Specification・型/データ仕様・DB schema・その他の機械可読仕様を種別で区別する専用スキーマを持たない。

> **ROOT-047**: `specification.json` は vtest 自身の `.verify/doc/` の文書モデルそのものである。

### 1.2 `derives_from`

> **DS-1594**: ノードの `derives_from` は上流ノード id の並びであり、参照先の該当箇所を指す `anchor` field を entry ごとに持たない。

> **DS-1595**: ノードの `derives_from` は、導出理由を記す `note` field を entry ごとに持たない。

> **DS-1596**: ノードの `derives_from` は同一の上流ノード id を重複して保持しない。

> **BD-165**: `derives_from` は上流ノードへの唯一のリンク種別である。

> **ROOT-041**: あくまで derived_from は参照なので。

VO 側は別である。

> **DS-391**: VO レコードの `derives_from` field は 1 件以上の document への直結を表す。

> **DS-398**: VO の `derives_from` entry も document レコードと同じく任意の `anchor`（参照先 document 内の該当箇所を指す不透明な文字列。節番号・条項番号・見出し等）と任意の `note` を持つ。

> **DS-402**: `anchor` と `note` は VO subject hash の入力に含まれない（VO subject hash は `derives_from` の参照先 document ID 集合を束縛する）。

**したがって「document ノードの `derives_from`」と「VO の `derives_from`」は別の形であり、同じ型で表してはならない。**
（この二分に対する正本内の反証候補は §6 の穴 H-3 に置いた。）

### 1.3 ノードのハッシュ

> **DES-572**: ノードの subject hash は domain `vtest:document-subject:v1` を用い、文ノードでは当該ノードの `id` と `statement` を、節ノードでは子ノードの subject hash を束縛し、`description` を束縛しない。

> **DES-575**: document subject hash は、層を区別せず、すべてのノードについて §1.3 の同一の規則で計算する。

> **DS-1599**: `description` だけの変更は document ノードの subject hash を変化させないため、当該ノードを上流依存 closure に含む判断記録・承認は失効しない。

§1.3 の共通規則:

> **DES-066**: 内容ハッシュは `sha256:<hex>` 形式で記録する。
> **DES-067 / DES-068 / DES-069**: byte-exact を要求しないテキスト fragment は改行を LF へ統一し、各行の末尾空白を除去する。それ以外の空白は正規化しない。
> **DES-070**: hash input は domain separator と長さ付き field から構成する。
> **DES-071**: hash input の各 field は `field-name`、UTF-8 byte length、byte 列の順に encode する。
> **DES-072**: hash input の encode は単純な文字列連結を行わない。
> **DES-073 / DES-074 / DES-075 / DES-076**: map は key 昇順、集合は正規化値の昇順、`cases` は宣言順、null / 空文字 / 空 list は異なる値として encode する。

domain は 5 つ: `vtest:test-subject:v1`（DES-077）、`vtest:target-subject:v1`（DES-083）、
`vtest:document-subject:v1`（DES-572）、`vtest:record-subject:v1`（DES-093）、`vtest:execution-state:v1`（DES-097）。

### 1.4 配置と検査

> **BD-019**: document の ID は `DOC-` とし、正典は `.verify/doc/` に置く。
> **BD-138**: `.verify/doc/` は `DOC-<NAME>.yaml` 形式で総称 document レコード（正典）を格納する。
> **BD-149**: 文書種別ごとの専用ディレクトリ（旧 `spec/` / `req/`）を設けず、上流文書はすべて `doc/` の総称 document レコード 1 種で表現する。
> **BD-150**: 決定論的解析の結果を保存する正典ディレクトリ（旧 `audits/`）を設けない。

> **DS-1287**: 各 `document` の `derives_from` 参照先が存在することを要求する（不在は E-SCAN-012、`chain_integrity = MISMATCH`）。
> **DS-390**: `root` 層のノードを除き、実効的な上流（自分の辺 ∪ 先祖の辺）を持たないノードは根候補であり、`config.yaml` の `doc.roots` に列挙されない場合は孤児として `orphan_detection` の `MISMATCH` とする。
> **DS-1614**: 依存 closure が束縛した document ノードのハッシュが現在の subject hash と一致しなくなった場合は `chain_integrity` の `MISMATCH`（診断 `STALE`）とする。

### 1.5 レコード読取（未知 field・config version）

> **DS-376**: レコードの未知フィールドはエラーではなく警告とする。
> **DS-405**: reader は読取り互換 field として `status` を受理するが、実効判定と VO subject hash では無視し、存在自体を W-STORE-001 として通知する。
> **DS-1572**: config reader は version 1 と version 2 を受理し、読み取りだけで config を書き換えない。
> **DS-1573 / BD-154 / BD-156 / DES-012 / DES-015**: config writer と `vtest init` は version 2 の adapter namespace を出力する。
> **DES-014 / DES-109**: reader は version 1 を単一の `rust-cargo` adapter 設定として in-memory 変換して読み取るが、読み取りだけで正典を書き換えない。
> **DS-357 / DS-1494**: version 1 では `verify.full_scope` の field 欠落を固定 4 検査として具体化し、重複または未知項目は E-CONFIG-001 で拒否する。
> **DS-358 / DS-1108**: 旧 12 項目の列挙（`spec_coverage` / `test_existence` 等）は現行 invariant に違反するため、version を問わず E-CONFIG-001 とし、in-memory 補完で受理しない。
> **DS-916**: `E-CONFIG-001` は error であり、config version、`verify.full_scope`（固定 4 検査）、`doc.roots`、`gates`（名前重複、`require` / `require.verification` 欠落、`require.verification` が 5 状態語彙外、`require.approvals` の不正・未解決ロール）、config field 型または登録 adapter が検証する設定値が現在の config invariant に違反することである。

診断コードのうち **W-STORE-001〜W-STORE-006 は正本にある**（DS-552〜DS-557）。**W-STORE-007 は正本に存在しない**
（`specification.json` 全文で 0 件）。

---

## 2. PR #20 仕分け（`crates/vtest-model`）

| # | 単位 | 判定 | 根拠ノード | 内容 |
|---|---|---|---|---|
| 1 | `lib.rs` モジュール分割（11 モジュール） | NO-NORM | — | 正本はモジュール構成を定めない。DES-378「ノード / VO / Relation / 判断記録 / 承認記録 / Evidence も §3 のスキーマに対応する struct を定義する」は型の存在だけを要求する。 |
| 2 | `id.rs` `define_string_id_type!` マクロ | KEEP | — | ID の型付け手段。正本は型名だけを定める。 |
| 3 | `id.rs` `SpecId` | DROP | BD-149, BD-170 | 旧 `spec/` の ID。BD-170「VO は旧モデルの `requirements`（REQ 参照）と `spec_refs`（SPEC + 節参照）は持たず、上流参照は `derives_from:[DOC-]` へ一本化する」。 |
| 4 | `id.rs` `ReqId` | DROP | BD-149, BD-170 | 同上。 |
| 5 | `id.rs` `VoId` | KEEP | BD-020, DES-241 | |
| 6 | `id.rs` `TestId` | KEEP | BD-021, DES-240 | |
| 7 | `id.rs` `SrcId` | KEEP | BD-022, DES-255, DES-313 | |
| 8 | `id.rs` `DocumentId` | KEEP | BD-019, DES-568 | 型としては流用可。値域（`DOC-` 前置か、層別ノード id `REQ-025` / `SPEC-421` か）は §6 穴 H-1。 |
| 9 | `hash.rs` `ContentHash` 型・`sha256:<hex>` 表現・`FromStr` | KEEP | DES-066 | |
| 10 | `hash.rs` `normalize_hashed_text` | KEEP | DES-067, DES-068, DES-069 | CRLF/CR → LF、行末の空白・タブ除去、他の空白は不変。正本と一致。 |
| 11 | `hash.rs` `ContentHash::from_text` | REWRITE | DES-070, DES-071 | 正規化テキストの素の SHA-256。DES-070 の domain separator も DES-071 の長さ付き field encode（`field-name` + UTF-8 byte length + bytes）も無い。単一 field の裸 digest であり、どの subject にも束縛されない。 |
| 12 | `hash.rs` `ContentHash::from_bytes` | REWRITE | DES-077, DES-083, DES-098 | `String::from_utf8_lossy` を通してから正規化する。Test construct bytes・implementation construct bytes・manifest entry の byte-exact 束縛を壊す。 |
| 13 | `hash.rs` テスト群（1 本） | KEEP | DES-067〜069 | 正規化規則そのものを検証している。ただし `ContentHash::from_text` を経由して比較しているため、#11 で `from_text` が消えるなら `normalize_hashed_text` を直接検証する形へ書き換えれば内容は生きる。 |
| 14 | `document.rs` `DerivesFrom { doc, anchor, note }` — **VO 用として** | KEEP | DS-391, DS-392, DS-393, DS-398, DS-399, DES-117 | |
| 15 | `document.rs` `DerivesFrom` — **document ノード用として** | REWRITE | DS-1594, DS-1595, DS-1596, DES-568 | ノードの `derives_from` は上流ノード id の並び。`anchor` / `note` を持たず、重複も持たない。VO 用と同じ型で表せない。 |
| 16 | `document.rs` `DocumentRecord.id` | KEEP | DES-568 | |
| 17 | `document.rs` `DocumentRecord.path` | DROP | DES-568, DES-573, BD-168 | 必須 field の列挙に無い。BD-168「仕様文書そのものは `.verify/` へ複製しない」下でも、ノードは文そのものを持つため path を必要としない。 |
| 18 | `document.rs` `DocumentRecord.content_hash` | DROP | DES-573 | 「ノードは登録時の内容ハッシュを field として保持せず、subject hash を §1.3 に従って計算する」。 |
| 19 | `document.rs` `DocumentRecord.title` | REWRITE | DS-1592, DES-568 | 現在は全ノードで `Option<String>`。正本では節ノードの必須 field であり、文ノードは持たない。ノード種別の型分けが要る。 |
| 20 | `document.rs` `DocumentRecord.derives_from` | REWRITE | DS-1593, DS-1594 | 型は #15 に従う。加えて文ノードでは必須（0 件可）、節ノードでは任意という差がある。 |
| 21 | `document.rs` `DocumentRecord.registered_at` | DROP | DES-568 | `registered_at` は `specification.json` 全文で 0 件。DES-568 の必須 field 列挙にも無い。**正本に明文が無い field** なので、正本形へ移す根拠が存在しないという理由で DROP に数えた。 |
| 22 | `document.rs` テスト群（2 本） | DROP | DES-568, DES-573 | `document_record_serializes_correctly` は `path` / `content_hash` / `registered_at` を含む wire 形を固定する。`derives_from_serializes_correctly` は entry の `anchor` / `note` を固定する。どちらもノードモデルでは偽になる。 |
| 23 | `vo.rs` `Dimension` | KEEP | DES-121, BD-172 | |
| 24 | `vo.rs` `CoveragePolicy`（`independent-axes` / `full-product` / `explicit`） | KEEP | DES-118 | 正本は `null` も値域に含めるが、実装は `Option<CoveragePolicy>` で表しており一致。 |
| 25 | `vo.rs` `VoRecord`（`status` を持たないこと含む） | KEEP | DES-117, DES-119, DES-120, DES-122 | `derives_from` の要素型は #14 に従う。DES-120「VO の `status` は承認レコードから導出する表示値であり、canonical writer は VO record へ保存しない」と一致。 |
| 26 | `vo.rs` テスト群（5 本） | KEEP | DES-117, DES-121 | `vo_record_combinations_are_dimension_keyed_maps` は DES-121 の逐語例と一致。 |
| 27 | `verification.rs` `VerificationState`（5 値） | REWRITE | DES-266〜DES-270 | 値は正本と一致。**型名が違う**。正本は同じ 5 variant を `CheckValue` に置く。 |
| 28 | `verification.rs` `DiagnosticLabel`（4 値） | KEEP | DES-271〜DES-274 | |
| 29 | `verification.rs` `VerificationCheck`（4 検査） | REWRITE | DES-275〜DES-278 | variant は正本と一致。**型名が違う**。正本は `CheckItem` に置く。 |
| 30 | `verification.rs` `VerificationResult` | NO-NORM | — | DES-S050 に対応する struct が無い。DES-279「`CheckValue` は状態のみを表し、原因説明は `DiagnosticLabel` として併記する」を満たす形ではあるが、型そのものの明文は無い。 |
| 31 | `verification.rs` テスト群（4 本） | REWRITE | DES-266〜278 | wire 値（`"PASS"` / `"chain_integrity"` 等）の検証は生きる。型名の追随が要る。 |
| 32 | `diagnostic.rs` `DiagnosticSeverity` / `Diagnostic` | KEEP | DES-320, DS-540, DS-552, DS-584 | `severity` の 2 値は DS-540「E-SCAN-006 は error であり」・DS-552「W-STORE-001 は warning であり」に対応。`candidates` は DS-584「候補は §6.3 の診断表示にだけ用い、表示できることを選択の根拠にしない」に対応。 |
| 33 | `test.rs` `TestTarget` enum | DROP | BD-192, DES-561 | BD-192「`filter`、`package`、`test_target` および `TestTarget` 型を `vtest-model` へ置かない」。 |
| 34 | `test.rs` `TestEntity.target` + `additional_targets` | REWRITE | DES-242, DES-287, DES-288, DES-290 | 正本は `targets: Vec<TargetRef>` 単一 field。単数 `target` は writer が 1 件のときだけ追加できる**互換 field**であって、canonical field ではない。「先頭 + 残り」に割る形は DES-290「複数 target から代表値を選んで `target` を生成しない」に反する。 |
| 35 | `test.rs` `TestEntity.filter` / `.package` / `.test_target` | DROP | DES-017, DES-480, DES-561, BD-192, DS-967 | DES-561「`vtest-model::TestEntity` は `ExecutionDescriptor` だけを実行座標として持ち、`filter`、`package`、`test_target`、`TestTarget` を含まない」。DS-967 によりこれらは wire compatibility layer が `rust-cargo` Test にだけ足す。 |
| 36 | `test.rs` `TestEntity` のその他 field（`id` / `covers` / `intent` / `input` / `expect` / `kind` / `cases` / `related` / `location` / `content_hash`） | KEEP | DES-240, DES-241, DES-245〜DES-252 | 型は正本と一致（`location` の中身は #38 で別途 REWRITE）。 |
| 37 | `test.rs` `TestRecord` | REWRITE | DES-294〜DES-303 | 正本に `TestRecord` という型は無い。同じ役割の型は `ManagedTestDraft` で、`execution: ExecutionDescriptor` を必ず持つ（DES-303）。 |
| 38 | `test.rs` テスト群（3 本） | REWRITE | DES-294〜303 | `test_record_carries_the_normalized_logical_fields` は field 集合を固定しており、`execution` の追加で偽になる。 |
| 39 | `source.rs` `Locator { path, item_path }` と `Locator::parse` | REWRITE | DES-254, DES-085, DES-258 | 正本は `TargetRef::Locator { adapter: AdapterId, value: String }`。`parse` は `::` 分割と `.rs` 拡張子判定を core で行っており、core が opaque locator を分解している。DES-258「`SourceLocation` の `locator` field は `String` 型であり、adapter 所有の opaque construct locator である」。 |
| 40 | `source.rs` `TargetRef` | REWRITE | DES-254, DES-255 | variant 名（`Locator` / `SrcId`）は一致するが、`Locator` の中身が #39 のとおり違う。 |
| 41 | `source.rs` `SourceLocation { file, function, start_line, end_line, start_byte, end_byte }` | REWRITE | DES-256, DES-257, DES-258, DES-259 | 正本は `adapter: AdapterId` / `path: ProjectPath` / `locator: String` / `byte_range: SourceRange`。行番号 field は正本に無く、`function` は Rust 固有の構造を core に持ち込む。 |
| 42 | `source.rs` `SourceFunction { locator, src_id, location, content_hash }` | REWRITE | DES-291, DES-312〜DES-315, DES-101, BD-133 | 対応する正本の型は `SourceTargetDraft { target, src_id, location, construct: SourceFragment }`。draft は **hash 未計算 DTO**（DES-291）であり、BD-133「adapter は、最終的な `TestEntity.content_hash` または `SourceTarget.content_hash` を返して自己確定してはならない」。`content_hash` を持つ形は契約違反。 |
| 43 | `source.rs` テスト群（1 本 `locator_splits_at_first_separator`） | DROP | DES-085, DES-254 | core が locator を分解する挙動そのものを固定するテスト。 |
| 44 | `evidence.rs` `CheckValue`（8 値、`#[deprecated]`） | DROP | DES-280 | 「`Missing` / `NotChecked` / `NotExecuted` / `Stale` を検証状態の variant として持たせない（旧 8 値モデルの排除）」。`#[deprecated]` 付きでも `TargetExecution.result` が現に使用している。 |
| 45 | `evidence.rs` `Revision { commit, dirty }` | KEEP | DES-183 の description（§3.6 の逐語例）`revision: { commit: "abc123...", dirty: false }` | description は DES-572 のとおり subject hash に束縛されない補助記述なので、根拠としては例示にとどまる。 |
| 46 | `evidence.rs` `EvidenceHashes { test_fn, target_fn, target_fns }` | REWRITE | DES-192〜DES-199 | 正本は `hashes: { test_subject, targets: [ { target, target_construct } ] }`。DES-197「writer は `hashes.test_subject` を必須とする」、DES-198「writer は Test construct 単体の hash を現在の Evidence freshness key として出力しない」。`test_fn` / `target_fn` は DES-199 が `rust-cargo` に限って**読取り互換 field**として認めるものであり、canonical writer の field ではない。 |
| 47 | `evidence.rs` `RunnerInfo { kind, command, exit_code }` | KEEP | DES-183 の description（§3.6 の逐語例）`runner: { kind, command, exit_code }` | #45 と同じく例示による根拠。 |
| 48 | `evidence.rs` `TargetExecution { checked, method, result, count }` | REWRITE | DES-185, DES-186, DES-187, DES-188, DES-189, DES-196 | 正本の field 名は `target_coverage`（DES-185「旧 `target_execution` field を改名」）。`result` の値域は `PASS` / `FAIL` / `UNKNOWN`（DES-187）で、`CheckValue` 8 値ではない。per-target の `targets[]`（`target` / `result` / `count`）が要る。 |
| 49 | `evidence.rs` `TestResult`（`PASS` / `FAIL`） | KEEP | DES-183 | 「Evidence レコードの `result` field はランナーが報告した `PASS` \| `FAIL` である（判定権威 §7）」。 |
| 50 | `evidence.rs` `EvidenceRecord` | REWRITE | DES-183〜DES-199 | `adapter` と `execution_state { schema, complete, hash }` が無い。`hashes` / `target_execution` は #46 / #48。 |
| 51 | `protocol.rs` `CheckItem`（旧 12 項目、`#[deprecated]`） | DROP | DS-358, DS-1108, DES-275〜DES-278 | 「旧 12 項目の列挙（`spec_coverage` / `test_existence` 等）は現行 invariant に違反するため、version を問わず E-CONFIG-001 とし、in-memory 補完で受理しない」。正本の `CheckItem` は 4 検査だけを持つ。 |
| 52 | `protocol.rs` `JsonEnvelope { ok, data, diagnostics }` | KEEP | DS-941, DS-1174, DS-1178 | DS-941「JSON 出力は最上位に `{ "ok": bool, "data": ..., "diagnostics": [...] }` を持つ」と逐語一致。コード中の CHECKME「`ok` は独立に意味を持つのか、診断から導出すべきか」への答えも正本にある。DS-1174「`--gate` を指定した実行では最上位 `ok` と終了コードをゲート充足で決める」・DS-1178 により、`ok` は診断からの導出値**ではない**。なお DS-947 は `verify` / `report` の JSON に最上位 field `scope` を要求するが、これは CLI 層の責務であり envelope 型の不足ではない。 |
| 53 | `protocol.rs` `ExitCode`（0/1/2/3） | NO-NORM | ROOT-033, SPEC-356, SPEC-357 | ROOT-033 が「内部エラー・入力不正は exit code 2/3 の領分」と述べるのみ。SPEC-S106（§17.2）に数値表は無く、SPEC-356「終了コードは診断 severity だけでなく操作段階で決める」・SPEC-357「検証状態と内部エラーは終了コードで分離する」だけがある。実装の 4 値はこれと矛盾しないが明文の裏付けは無い。 |
| 54 | `protocol.rs` `ScanSummary { files, tests, sources }` | NO-NORM | — | 正本に該当なし。 |
| 55 | `protocol.rs` テスト群（1 本 `envelope_has_required_top_level_fields`） | KEEP | DS-941 | 最上位 3 field の存在を検証しており、DS-941 の逐語と一致する。 |

### #20 集計

| 判定 | 件数 |
|---|---|
| KEEP（流用可） | 22 |
| REWRITE（書き換え） | 18 |
| DROP（捨てる） | 11 |
| NO-NORM（正本に明文が無い） | 4 |
| **合計** | **55** |

NO-NORM の 4 件は #1（モジュール分割）、#30（`VerificationResult`）、#53（`ExitCode` の数値表）、#54（`ScanSummary`）。

不足（正本にあるが #20 に無い）は §4.1 に 20 件。

---

## 3. PR #21 仕分け（`crates/vtest-store`）

#21 の差分は 6 ファイル・+1984 / −360 行。`canonical.rs`（886 行）が新規、`lib.rs` が config 周りの全面書き換え、
`records.rs` は Relation reader/writer の追加と共有ヘルパーの `pub(crate)` 化。

| # | 単位 | 判定 | 根拠ノード | 内容 |
|---|---|---|---|---|
| 1 | `lib.rs` `VerifyLayout::doc_dir()` | KEEP | BD-047, BD-138, BD-149 | |
| 2 | `lib.rs` `VerifyLayout::decisions_dir()` | KEEP | BD-051, BD-142 | |
| 3 | `lib.rs` `VerifyLayout::spec_dir()` / `req_dir()`（既存・未削除） | DROP | BD-149 | 「旧 `spec/` / `req/`」の専用ディレクトリを設けない。#21 は残置し、PR8 で撤去するとコメントで開示している。 |
| 4 | `lib.rs` `VerifyLayout::audits_dir()`（既存・未削除） | DROP | BD-150 | 「決定論的解析の結果を保存する正典ディレクトリ（旧 `audits/`）を設けない」。 |
| 5 | `lib.rs` `VerifyLayout` のその他ディレクトリ（`vo` / `rel` / `forms` / `approvals` / `evidence` / `cache`） | KEEP | BD-048〜BD-054, BD-139〜BD-148 | |
| 6 | `lib.rs` `ProjectConfig`（version 2 正規形、`deny_unknown_fields`、`approval_roles` を含む） | KEEP | BD-154, DES-012, DS-916, DS-1160, DS-1161 | `approval_roles` は BD-154 の逐語例に無いが DS-1160「承認レコードは role field を持たないため、`config.yaml` に承認ロール → approver id 集合の対応を project 定義可能とする」が定めており、その description が `approval_roles:` の YAML 例を持つ。 |
| 7 | `lib.rs` `AdapterConfig` / `ScanSection` / `RunSection` | KEEP | BD-154, DS-354 | `ScanSection` / `RunSection` に `deny_unknown_fields` を付けない判断は DS-354「core は未知の namespace や値を Rust 設定として解釈しない」・adapter 委譲の逐語に沿う。adapter registry 不在は #21 が開示済み。 |
| 8 | `lib.rs` `DocSection { roots: Vec<DocumentId> }` | KEEP | DS-390, BD-154 | 要素型の値域は §6 穴 H-1 に従属。 |
| 9 | `lib.rs` `GateConfig` / `GateRequirement`（`approvals` 省略可） | KEEP | BD-154, DS-916 | |
| 10 | `lib.rs` `FIXED_FULL_SCOPE`（4 検査） | KEEP | DS-356, DS-357, DS-1494, DS-916 | |
| 11 | `lib.rs` `VERIFICATION_STATES`（5 状態） | KEEP | DS-916 | 「`require.verification` が 5 状態語彙外」を E-CONFIG-001 とする。 |
| 12 | `lib.rs` `ProjectConfig::from_yaml_v2` | KEEP | DS-1572, DS-916 | |
| 13 | `lib.rs` `ProjectConfig::from_yaml_v1` + `V1Config` 系 | KEEP | DES-014, DES-109, DS-1572, DS-357 | version 1 を単一 `rust-cargo` adapter へ in-memory 変換し、ファイルを書き換えない。 |
| 14 | `lib.rs` `validate_full_scope`（旧 12 項目拒否を含む） | KEEP | DS-358, DS-1108, DS-1494, DS-1495 | |
| 15 | `lib.rs` `validate_v2_config`（adapter ID 重複・承認ロール未解決・重複） | KEEP | DS-916, DS-352, DS-374, DS-1162 | DS-374「`require.approvals` のロール名が `approval_roles` に解決できない場合も E-CONFIG-001（終了コード 2）とする」。実装は `E-CONFIG-001` をエラー文言に載せている（`lib.rs` に 8 箇所）。 |
| 16 | `lib.rs` version 欠落・非整数・未知 version の拒否 | KEEP | DS-916, DS-1572 | 「config version」自体が E-CONFIG-001 条件。 |
| 17 | `lib.rs` `init_project`（生成ディレクトリ・`.gitignore`・組込 Form） | KEEP | BD-137〜BD-148, BD-153 | 生成集合は `doc` / `vo` / `rel` / `forms` / `decisions` / `approvals` / `evidence` / `cache/{bundles,logs,cov}`。`spec` / `req` / `audits` を作らない。正本と一致。 |
| 18 | `lib.rs` `read_entity_ids` → `[Vec<String>; 2]` | KEEP | BD-149 | 旧 3 系（spec/req/vo）から doc/vo の 2 系へ。 |
| 19 | `lib.rs` config テスト群（約 25 本） | KEEP | DS-356, DS-357, DS-358, DS-916, DS-1494, DS-1495 | 逐語例のパース、version 1 昇格、旧 12 項目拒否、gate 語彙検証まで正本の条件を機械的に踏む。 |
| 20 | `canonical.rs` モジュール（`yaml_serde` 経由・`records.rs` と分離） | KEEP | BD-055 | 「ファイル形式はすべて YAML とする」。 |
| 21 | `canonical.rs` `DOCUMENT_KEYS`（`id` / `path` / `content_hash` / `title` / `derives_from` / `registered_at`） | REWRITE | DES-568, DES-573, DS-1592 | ノードの必須 field は `id` / `statement` / `derives_from` / `source`（文）、`id` / `title` / `source`（節）。`path` / `content_hash` / `registered_at` は正本に無い。 |
| 22 | `canonical.rs` `DERIVES_FROM_KEYS`（`doc` / `anchor` / `note`）— VO 用 | KEEP | DS-398, DS-399 | |
| 23 | `canonical.rs` `DERIVES_FROM_KEYS` — document ノード用 | DROP | DS-1594, DS-1595 | ノードの `derives_from` は id の並びであり entry mapping ではない。既知キー表そのものが不要になる。 |
| 24 | `canonical.rs` `VO_KEYS`（`status` を既知として含む） | KEEP | DES-117〜DES-122, DS-405 | |
| 25 | `canonical.rs` `DIMENSION_KEYS` | KEEP | DES-121 | |
| 26 | `canonical.rs` `unknown_key_diagnostics`（未知 field を警告） | REWRITE | DS-376, DS-552〜DS-557 | 「未知フィールドはエラーではなく警告」という規範は正本にある。**しかしコード `W-STORE-007` は正本に存在しない**（`specification.json` に 0 件。正本の W-STORE は 001〜006 のみ）。実装が診断コードを新設している。 |
| 27 | `canonical.rs` `derives_from_unknown_key_diagnostics` | REWRITE | DS-1594, DS-1595 | #23 に従属。VO 側だけ残る。 |
| 28 | `canonical.rs` `dimension_unknown_key_diagnostics` | REWRITE | DES-121 | コードの扱いは #26 と同じ。 |
| 29 | `canonical.rs` `document_to_yaml` / `document_from_yaml` | REWRITE | DES-568, DES-572, DES-573 | ノード形（`statement` / `source`、節/文/根の区別）へ全面的に作り直す。 |
| 30 | `canonical.rs` `read_document` / `write_document` | REWRITE | DES-568, BD-138 | 単一ノード 1 ファイル前提。保存形式は §6 穴 H-2。 |
| 31 | `canonical.rs` document の id とファイル名の一致検査 | NO-NORM | BD-138 | BD-138 は `DOC-<NAME>.yaml` 形式とだけ述べ、id と NAME の一致を明文化していない。 |
| 32 | `canonical.rs` `vo_record_to_yaml` / `vo_record_from_yaml` | KEEP | DES-117〜DES-122 | |
| 33 | `canonical.rs` `read_vo_record` / `write_vo_record` | KEEP | BD-048, BD-139 | |
| 34 | `canonical.rs` `require_at_least_one_derives_from`（read/write 両側） | KEEP | DS-391, DS-396, DES-S037 | 件数だけを見て、参照先の存在は scan 層（E-SCAN-012）へ残す切り分けが DS-1287 / DS-397 と一致。 |
| 35 | `canonical.rs` `status` を W-STORE-001 として通知 | KEEP | DS-405, DS-552 | 逐語一致。 |
| 36 | `canonical.rs` document テスト群（10 本） | DROP | DES-568, DES-573 | `sample_document` から逐語例パース、既知キー整合テストまで、すべて `path` / `content_hash` / `registered_at` を持つ形を固定する。 |
| 37 | `canonical.rs` VO テスト群（9 本） | KEEP | DES-117, DES-121, DS-405 | 逐語例・combinations 例・`status` 互換の 3 点を押さえている。 |
| 38 | `records.rs` `RelationRecord::from_yaml` の診断返却化 | REWRITE | DS-376 | 未知 field 警告という規範は正しい。コードは #26 と同じ問題。 |
| 39 | `records.rs` `read_relation` / `write_relation`（`REL-<ULID>` 正規化） | KEEP | BD-140, DES-030, DES-031, DES-125 | 「reader は version 1 互換入力として bare ULID を `REL-<ULID>` へ in-memory 正規化する」。 |
| 40 | `records.rs` 共有ヘルパーの `pub(crate)` 化（`scalar` / `list` / `yaml_list` 等） | NO-NORM | — | 実装裁量。 |
| 41 | `records.rs` `read_evidence` の ULID / ファイル名 invariant テスト | KEEP | BD-144 | |
| 42 | `records.rs` Relation テスト群（3 本） | KEEP | BD-140, DES-030 | W-STORE-007 を直接 assert する 1 本は #26 に従属。 |

**#21 が触っていない predecessor コード**: `records.rs` の `SpecRecord` / `ReqRecord` / 旧 `VoRecord` / `AuditRecord` /
`ApprovalRecord` と、それらの `read_spec` / `read_req` / `read_vo` / `read_audit` / `read_approval`。
BD-149・BD-150 により旧系はすべて撤去対象だが、#21 は手を付けておらず PR8 の担当である。仕分け表には入れていない。

### #21 集計

| 判定 | 件数 |
|---|---|
| KEEP（流用可） | 29 |
| REWRITE（書き換え） | 7 |
| DROP（捨てる） | 4 |
| NO-NORM（正本に明文が無い） | 2 |
| **合計** | **42** |

不足（正本にあるが #21 に無い）は §4.2 に 9 件。

---

## 4. 不足一覧

### 4.1 #20（`vtest-model`）に無い正本の型

| # | 型 | 根拠ノード |
|---|---|---|
| 1 | `AdapterId` | DES-254, DES-256, DES-260, DES-304, DES-316, DES-337 |
| 2 | `ProjectPath` | DES-257 |
| 3 | `SourceRange` | DES-259 |
| 4 | `ExecutionDescriptor { adapter, project, suite, selector }` | DES-260〜DES-263, DES-253, DES-281 |
| 5 | `TestSuite { kind, name }` | DES-264, DES-265 |
| 6 | `SourceFragment { location, bytes }` | DES-292, DES-293 |
| 7 | `ManagedTestDraft`（`execution` を含む 10 field） | DES-294〜DES-303 |
| 8 | `DiscoveredTestDraft { adapter, location, construct, metadata_sources, managed }` | DES-304〜DES-308 |
| 9 | `ManagedTestDraftLink { Missing, One, Multiple }` | DES-309〜DES-311 |
| 10 | `SourceTargetDraft { target, src_id, location, construct }` | DES-312〜DES-315 |
| 11 | `DiscoveryBatch { adapter, completeness, discovered_tests, source_targets, diagnostics }` | DES-316〜DES-320 |
| 12 | `DiscoveryCompleteness { Complete, Incomplete }` | DES-321, DES-322 |
| 13 | `DiscoveredTest { adapter, location, content_hash, managed }` | DES-337〜DES-340 |
| 14 | `ManagedTestLink { Missing, One(TestId), Multiple }` | DES-341〜DES-343 |
| 15 | `CanonicalProjection` | DES-353 |
| 16 | `ExecutionInputDraft { root_identity, root_relative_path, kind, bytes }` | DES-354〜DES-357 |
| 17 | `ExecutionStateDraft`（9 field） | DES-358〜DES-366 |
| 18 | 5 domain の subject hash 構築器（`vtest:test-subject:v1` / `target-subject` / `document-subject` / `record-subject` / `execution-state`） | DES-070〜DES-077, DES-083, DES-093, DES-097, DES-572 |
| 19 | 長さ付き field encoder（`field-name` + UTF-8 byte length + bytes、map/集合/list の順序規則） | DES-071, DES-073, DES-074, DES-075, DES-076 |
| 20 | ノード型（文 / 節 / 根の 3 種）と `source` field | DES-568, DES-378 |

### 4.2 #21（`vtest-store`）に無い正本のレコード

| # | 単位 | 根拠ノード |
|---|---|---|
| 1 | 判断記録レコード reader/writer（`.verify/decisions/<ULID>.yaml`） | BD-051, BD-142, DES-S040 |
| 2 | 承認レコードの正本形 reader/writer（現在は predecessor `ApprovalRecord`） | BD-052, BD-143, DES-S041 |
| 3 | Evidence レコードの正本形 reader/writer（`execution_state` / `hashes.test_subject` / `target_coverage`） | BD-053, BD-144, DES-183〜DES-199 |
| 4 | W-STORE-002〜W-STORE-006 の発行 | DS-553〜DS-557 |
| 5 | ノードの subject hash 計算（保存 field ではなく計算） | DES-572, DES-573, DES-575 |
| 6 | `STALE` 判定（依存 closure の束縛ハッシュと現在の subject hash の突合） | DS-1614, DS-1601, DS-1610 |
| 7 | 層別ノード配列の読み書き（`root` / `request` / `require` / `spec` / `detailed_spec` / `basic_design` / `design`） | BD-312, ROOT-047 |
| 8 | `.verify/forms/` の Form Schema reader（`init_project` は書くが読み取り側が無い） | BD-050, BD-141 |
| 9 | 旧 `spec/` / `req/` / `audits/` の撤去 | BD-149, BD-150 |

---

## 5. 作り直しの提案作業順（**提案であり決定ではない**）

正本の層順（基本設計 → 詳細設計）に沿い、下の段が上の段の型を要求する順に並べた。
各段は独立して緑にできる粒度を意図している。

1. **ハッシュ契約を先に確定する。** `hash.rs` に domain separator + 長さ付き field encoder を入れ、
   `ContentHash::from_text` / `from_bytes` の裸の用法を塞ぐ（§4.1 #18・#19）。
   これを後回しにすると、以降の全 struct が誤ったハッシュ入力で束縛される。
2. **ノード型を導入する。** `DocumentRecord` を文 / 節 / 根の 3 種へ置き換え、`derives_from` を id の並びにし、
   `source` を足す（§2 #15〜#22、§4.1 #20）。VO 用 `DerivesFrom` はここで**別型として分離**する。
3. **core 中立の座標型を入れる。** `AdapterId` / `ProjectPath` / `SourceRange` / `SourceLocation` /
   `TargetRef::Locator{adapter,value}` / `ExecutionDescriptor` / `TestSuite`（§4.1 #1〜#5）。
   `Locator::parse` と `TestTarget` はここで消える（§2 #33、#39）。
4. **`TestEntity` を正本形へ揃える。** `targets: Vec<TargetRef>` 単一 field、`execution` の追加、
   `filter` / `package` / `test_target` の除去（§2 #34〜#37）。
5. **draft DTO 群を入れる。** `SourceFragment` から `DiscoveryBatch` まで（§4.1 #6〜#14、#16、#17）。
   `SourceFunction` はここで `SourceTargetDraft` に置き換わる（§2 #42）。
6. **Evidence を正本形へ。** `execution_state` / `hashes.test_subject` / `target_coverage`（§2 #46、#48、#50）。
   旧 8 値 `CheckValue` と旧 12 項目 `CheckItem` をここで削除する（§2 #44、#51）。
7. **store のノード保存を作り直す。** `canonical.rs` の document 側を #2 のノード型に合わせ、
   既知キー表とテスト 10 本を差し替える（§3 #21、#23、#29、#30、#36）。
   VO 側・config 側・Relation 側は触らない。
8. **判断記録 / 承認 / Evidence の正本 store と W-STORE-002〜006 を足す**（§4.2 #1〜#4）。
9. **旧系撤去**（§4.2 #9、§3 #3、#4）。既存の PR8 の担当範囲。

**型名の扱いについて。** §2 #27 / #29 の `VerificationState` / `VerificationCheck` は、値は正本と一致し名前だけが違う。
正本は `CheckValue` / `CheckItem` を使う（DES-266〜DES-278）。改名するか、正本側の型名を見直すよう上流へ返すかは、
本書では決めない。**ただし、どちらも選ばずに現状を残すと「正本に無い型名で正本の値を持つ」状態が固定される。**

---

## 6. 未解決の正本の穴

いずれも本書では埋めない。上流への差し戻し候補として列挙する。

### H-1. ノード id の値域が二重になっている

BD-019「document の ID は `DOC-` とし」・BD-138「`DOC-<NAME>.yaml` 形式で」に対し、
ROOT-047「`specification.json` は vtest 自身の `.verify/doc/` の文書モデルそのものである」と
BD-312「上流文書は層ごとのトップレベル配列に分けたノードで表現する」が同時に立っている。
現在の `specification.json` のノード id は `REQ-025` / `SPEC-421` / `DES-568` であって `DOC-` を持たない。
`config.yaml` の `doc.roots`（BD-154 の例は `[DOC-REQ-ROOT]`）がどちらの語彙を取るかも未決。

### H-2. `.verify/doc/` の保存形式が未決

BD-138 は「1 ノード 1 ファイル」を含意する `DOC-<NAME>.yaml` を定め、
ROOT-047 + BD-312 は「層ごとのトップレベル配列を持つ 1 つの JSON」を含意する。
`reports/doc-model-realignment-2026-09-06.md` §0.1 は保存形式を明示的に審査対象外としており、決定は残っている。
#21 の `read_document` / `write_document` は前者を実装している。

### H-3. `anchor` の有無が design 層の中で矛盾している

- **DES-571**（`3.1 document レコード` 節に置かれている）: 「`anchor` は `derives_from` の entry field としても Test metadata としても存在しない（§4.1）」— 無条件。
- **DS-398**: 「VO の `derives_from` entry も document レコードと同じく任意の `anchor` … と任意の `note` を持つ」。
- **DES-117** の逐語例: VO レコードに `anchor: "§8.2条項2"` / `note: ""` が現れる。
- **BD-171**（basic_design、design より上位）: 「対応ペアは、`anchor` 付き `derives_from` エッジとして保持し、§11.6 の projection 出力で露出する」。
- **DS-1142 / DS-1143**: trace 出力の `derives_from` エッジは `anchor` と `note` を同伴し、`{ "from": "DOC-REQ-001", "relation": "derives_from", "anchor": "§12.3", "note": "", "to": "VO-PARSER-UTF8-003" }` の形とする。

DES-571 の `source` は詳細設計 §3.1 L206、`cites` は「基本仕様 §12」、`derives_from` は SPEC-123 / SPEC-124 と
SPEC-369〜407（§12 Test Registry）を指す。退役 md の同じ行（`docs/archive/…詳細設計 v0.1.md` L206、規範ではなく読解の補助）は
逆向きに書かれていた。

> この `anchor` は `derives_from` entry の field であり、Test metadata には存在しない（Test の存在理由分類 `role` / `anchor` / `anchor_rationale` は持たない。§4.1、基本仕様 §12）。

つまり DES-571 は、この文の前半を #14 に合わせて反転させた結果である。**節の置き場所（§3.1 document レコード）から読めば
「ノードの `derives_from` には無い」で DS-1594 と一致し、VO（DS-398・DES-117・BD-171）とも衝突しない。**
しかし DS-1594 が「ノードの」と明示的に限定しているのに対し、DES-571 の文は無条件である。
本書は §1.2 のとおり「ノードは持たない / VO は持つ」で仕分けたが、**この読みは節の位置から導いたものであって、
DES-571 の逐語には現れていない。** 限定語を足すかどうかは上流の決定事項である。

### H-4. document の `path` / `content_hash` が下流に残っている

上位:
- **DES-573**: ノードは登録時の内容ハッシュを field として保持しない。
- **DES-572**: ノードの subject hash は `id` と `statement` を束縛する。

下位（未伝播）:
- **DS-1017**: 「`doc show` は DOC の path・content_hash・derives_from・根指定・鮮度（content_hash と実ファイルの一致）・実効承認状態を表示する。」
- **DS-1623**: 「文書鎖（document derives_from・content_hash）」。
- **DS-724** の description: 「document は登録 content_hash と実ファイルの一致も要求。不一致の場合は当該 document を STALE とし」。
- **DS-1195**: 「`doc_upsert` は document フィールド一式（`path`、`derives_from[]`（`doc` + 任意 `anchor` + 任意 `note`）、`root: bool`、`update: bool`）を入力とし」。
- **DS-384**: 「本システムは `anchor` を `path` の実ファイル内位置へ解決せず」。
- **SPEC-378** の description: `vtest doc add --id DOC-BASIC-001 --path docs/basic-spec.md [--title <t>] [--derives-from DOC-REQ-001 [--anchor <text>] [--note <text>]]`。

**これらは #20 の `DocumentRecord.path` / `content_hash` を流用する根拠にしてはならない。** 上位が明示的に否定している。
CLI / MCP 層（PR #26 以降）が同じ穴に当たるため、上流での伝播が要る。

### H-5. 未知 field の診断コードが正本に無い

DS-376「レコードの未知フィールドはエラーではなく警告とする」は規範として存在するが、コードが割り当てられていない。
正本の W-STORE は 001〜006 のみ（DS-552〜DS-557）。#21 は `W-STORE-007` を実装側で新設した。
コードを正本に足すのか、既存コードへ寄せるのか、コードなしの警告にするのかは上流の決定事項である。

### H-6. `CheckValue` / `CheckItem` という型名が正本にあるまま

DES-266〜DES-270 は 5 状態を `CheckValue` に、DES-275〜DES-278 は 4 検査を `CheckItem` に置く。
一方で ROOT-033（F7）は「降格（診断ラベル化）」と述べ、旧 8 値モデルの排除（DES-280）と旧 12 項目の排除（DS-358）を要求している。
**値の集合は移行済みだが、型名は旧モデルの語をそのまま使っている。**
正本の型名を変えるか、実装を正本の型名へ揃えるかは決まっていない。
