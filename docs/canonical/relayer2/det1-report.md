# 詳細設計 v0.1 本冊 §0〜§5（DES-001〜DES-700 相当）演繹的再配置 — det1 二次分類パス（2026-09-05）

対象: `docs/canonical/specification.json` の `spec` / `detailed_spec` / `basic_design` / `design` 配列のうち、`source.doc` が「詳細設計 v0.1」本冊（別紙A・別紙Cを除く）かつ `source.lines[0] <= 948` の全700文。これは一次分類パス（`docs/canonical/relayer/det1.json`、旧DES-001〜DES-700採番時点の判定）が既に適用され、現在の `specification.json` へ反映された後の状態を母集団とする。

判定基準: `docs/canonical/LAYERING-reference.md` の「各層が定義できていれば成立、の一覧」（L37–L42）のうち、各文がどの1項目を最も具体的に実現しているかを名指しし、その項目が属する層へ配置した（`docs/canonical/LAYERING.md` §2 冒頭の第一の判定＝一覧項目一致を優先し、決め手が必要な場合のみ同§2の1→1a→2→3の手順を併用）。どの項目にも当たらない文は現在の層に据え置き `list_item: "NONE"` とした。`code_like` フラグは `docs/canonical/relayer/det1.json`（旧DES-*id採番）から、断片データ `docs/canonical/fragments/det1.json` の文書順・文本文突合によって引き継いだ。

作業は5並列サブエージェントに範囲分割（旧DES-001〜159 / 160〜276 / 277〜455 / 456〜632 / 633〜700、一次分類パスの節構成と同一境界）して実施し、本書はその結果を統合・機械検証したもの。

## 1. 層ごとの集計（before → after）

| 層 | before（現在のspecification.json） | after（本パスの判定） | 差分 |
|---|---:|---:|---:|
| 基本仕様(spec) | 40 | 38 | -2 |
| 詳細仕様(detailed_spec) | 177 | 229 | +52 |
| 基本設計(basic_design) | 88 | 95 | +7 |
| 詳細設計(design) | 395 | 338 | -57 |
| **合計** | **700** | **700** | 0 |

- **移動件数（層が変わった文）**: 59 / 700
- **NONE件数（一覧項目に当たらず現在層へ据え置き）**: 26 / 700

## 2. 移動の全件（層が変わった文）

| id | 文（60字まで） | 移動前 → 移動後 | 一覧項目 |
|---|---|---|---|
| `DES-046` | coreはadapter出力と現在のsource bytesの対応を検証し、言語非依存encodingとSHA-256計… | 詳細設計(design) → **基本設計(basic_design)** | 基本設計: 主要なデータの流れ |
| `DES-099` | `judgment_ref` が指す判断記録が存在しない場合は、書込み時にE-APPROVAL-001として拒否する。既… | 詳細設計(design) → **詳細仕様(detailed_spec)** | 詳細仕様: エラーの外部的意味 |
| `DES-100` | 判断記録ULIDを `subject` に持つ承認レコードは、書込み時にE-APPROVAL-002として拒否する。既存… | 詳細設計(design) → **詳細仕様(detailed_spec)** | 詳細仕様: エラーの外部的意味 |
| `DES-101` | VO ID・document IDのいずれにも解決しない `subject`（Test ID、Source Target… | 詳細設計(design) → **詳細仕様(detailed_spec)** | 詳細仕様: エラーの外部的意味 |
| `DES-105` | `approved_state` が値域外の他の値の場合、書込み時にE-APPROVAL-002として拒否する。既存レコ… | 詳細設計(design) → **詳細仕様(detailed_spec)** | 詳細仕様: エラーの外部的意味 |
| `DES-108` | 参照先を解決できない、対象が一致しない、または自己参照する `supersedes` entryを含むレコードは、書込み… | 詳細設計(design) → **詳細仕様(detailed_spec)** | 詳細仕様: エラーの外部的意味 |
| `DES-109` | 既存レコードとして読み取った場合、およびsupersede関係が循環する場合は、当該レコードを実効集合へ寄与させずW-S… | 詳細設計(design) → **詳細仕様(detailed_spec)** | 詳細仕様: エラーの外部的意味 |
| `DES-110` | 承認レコードの `supersedes` はRelationとは独立であり、`type: supersedes` のRe… | 詳細設計(design) → **詳細仕様(detailed_spec)** | 詳細仕様: 仕様上の不変条件 |
| `DES-138` | 依存entryを持たない互換Approvalは読取りと履歴表示だけを許可し、現在の `approved` を導出しない。… | 詳細設計(design) → **詳細仕様(detailed_spec)** | 詳細仕様: エラーの外部的意味 |
| `DES-153` | `target_coverage.checked: false` では `method` と `result` をnul… | 詳細設計(design) → **詳細仕様(detailed_spec)** | 詳細仕様: 正常系・異常系・境界値・状態ごとの振る舞い |
| `DES-163` | Evidence内の `target` は実行時snapshotを識別するkeyであり、TEST → SRC edgeの… | 詳細設計(design) → **基本設計(basic_design)** | 基本設計: データの所有主体 |
| `DES-171` | repository内helperだけの変更もmanifest hashを変化させる。 | 詳細設計(design) → **詳細仕様(detailed_spec)** | 詳細仕様: 仕様上の不変条件 |
| `DES-172` | Evidence readerは `execution_state` を欠く互換recordを履歴表示できるが、現在のE… | 詳細設計(design) → **詳細仕様(detailed_spec)** | 詳細仕様: 正常系・異常系・境界値・状態ごとの振る舞い |
| `DES-173` | schema違反、target entryの欠落・重複・余剰、またはaggregate resultとtarget別結果… | 詳細設計(design) → **詳細仕様(detailed_spec)** | 詳細仕様: エラーの外部的意味 |
| `DES-178` | `targets[]` は検証対象をSource Targetとして実現するためのcapability fieldであり… | 詳細設計(design) → **基本設計(basic_design)** | 基本設計: コンポーネント間の境界 |
| `DES-180` | `rust-cargo` の `@vtest.` 宣言表面は2種であり、表面ごとに認識する行形式が異なる。 | 詳細設計(design) → **詳細仕様(detailed_spec)** | 詳細仕様: 入力値の範囲・形式・制約 |
| `DES-181` | Test constructのdoc comment（`///` または `/** */`）は表面1であり、test-a… | 詳細設計(design) → **詳細仕様(detailed_spec)** | 詳細仕様: 入力値の範囲・形式・制約 |
| `DES-182` | Test constructではない関数itemのdoc comment（対象実装側の関数等）は表面2であり、sourc… | 詳細設計(design) → **詳細仕様(detailed_spec)** | 詳細仕様: 入力値の範囲・形式・制約 |
| `DES-183` | test-annotation-lineの文法は `"@vtest." test-key SP value` である。 | 詳細設計(design) → **詳細仕様(detailed_spec)** | 詳細仕様: 入力値の範囲・形式・制約 |
| `DES-184` | source-target-annotation-lineの文法は `"@vtest." source-target-k… | 詳細設計(design) → **詳細仕様(detailed_spec)** | 詳細仕様: 入力値の範囲・形式・制約 |
| `DES-185` | test-keyの値域は `id` / `covers` / `target` / `intent` / `input`… | 詳細設計(design) → **詳細仕様(detailed_spec)** | 詳細仕様: 入力値の範囲・形式・制約 |
| `DES-186` | source-target-keyの値域は `src-id` である。 | 詳細設計(design) → **詳細仕様(detailed_spec)** | 詳細仕様: 入力値の範囲・形式・制約 |
| `DES-187` | valueは行末までのテキストとし、前後空白は除去する。 | 詳細設計(design) → **詳細仕様(detailed_spec)** | 詳細仕様: 入力値の範囲・形式・制約 |
| `DES-188` | annotation行は1行1キーとする。 | 詳細設計(design) → **詳細仕様(detailed_spec)** | 詳細仕様: 入力値の範囲・形式・制約 |
| `DES-189` | `covers` と `related` の値はカンマ区切りで複数指定できる。 | 詳細設計(design) → **詳細仕様(detailed_spec)** | 詳細仕様: 入力値の範囲・形式・制約 |
| `DES-190` | `case` と `related` はキー自体を複数行書ける。 | 詳細設計(design) → **詳細仕様(detailed_spec)** | 詳細仕様: 入力値の範囲・形式・制約 |
| `DES-191` | `case` と `related` 以外のキーの重複はエラーE-SCAN-005とする。 | 詳細設計(design) → **詳細仕様(detailed_spec)** | 詳細仕様: エラーの外部的意味 |
| `DES-192` | ただし `kind` がintegration系のTestに限り、`target` の複数行を許容する。 | 詳細設計(design) → **詳細仕様(detailed_spec)** | 詳細仕様: 入力値の範囲・形式・制約 |
| `DES-193` | 許容された複数 `target` 内でも同じTargetRefの重複はE-SCAN-005とする。 | 詳細設計(design) → **詳細仕様(detailed_spec)** | 詳細仕様: エラーの外部的意味 |
| `DES-194` | 綴りが異なっても解決後に同一canonical Source Targetへ到達する複数宣言（同じSource Targ… | 詳細設計(design) → **詳細仕様(detailed_spec)** | 詳細仕様: エラーの外部的意味 |
| `DES-195` | 表面1で、`@vtest.` で始まるがtest-keyを持たない行はエラーE-SCAN-006とする（打鍵ミスの検出を… | 詳細設計(design) → **詳細仕様(detailed_spec)** | 詳細仕様: エラーの外部的意味 |
| `DES-196` | 表面1のE-SCAN-006は、未知キーに加え、source-target-key（`src-id`）の誤配置も含む。`… | 詳細設計(design) → **詳細仕様(detailed_spec)** | 詳細仕様: エラーの外部的意味 |
| `DES-197` | 表面2で、`@vtest.` で始まるがsource-target-keyを持たない行（test-keyを含む）は警告W… | 詳細設計(design) → **詳細仕様(detailed_spec)** | 詳細仕様: エラーの外部的意味 |
| `DES-199` | `src-id` は表面2でも反復不可であり、同一関数itemでの重複は採用すべきIDを決定できないためエラーE-SCA… | 詳細設計(design) → **詳細仕様(detailed_spec)** | 詳細仕様: エラーの外部的意味 |
| `DES-200` | `src-id` 重複時はいずれの宣言値も採用せず、当該Source TargetのSRC IDは無しとして扱う（どちら… | 詳細設計(design) → **詳細仕様(detailed_spec)** | 詳細仕様: エラーの外部的意味 |
| `DES-201` | doc comment 内の `@vtest.` を含まない行は自由記述として無視する。 | 詳細設計(design) → **詳細仕様(detailed_spec)** | 詳細仕様: 入力値の範囲・形式・制約 |
| `DES-202` | `@vtest.src-id` はテストではなく対象実装側の関数に付与し、任意の恒久SRC IDを宣言する。 | 詳細設計(design) → **詳細仕様(detailed_spec)** | 詳細仕様: 入力値の範囲・形式・制約 |
| `DES-205` | 表面2での打鍵ミス（`src_id` 等の未知キー）はW-SCAN-105、`src-id` の重複はE-SCAN-00… | 詳細設計(design) → **詳細仕様(detailed_spec)** | 詳細仕様: エラーの外部的意味 |
| `DES-206` | locator文法は `locator = path "::" item-path` である。 | 詳細設計(design) → **詳細仕様(detailed_spec)** | 詳細仕様: 入力値の範囲・形式・制約 |
| `DES-207` | pathはプロジェクトルートからの相対パス（"/" 区切り、".rs" で終わる）である。 | 詳細設計(design) → **詳細仕様(detailed_spec)** | 詳細仕様: 入力値の範囲・形式・制約 |
| `DES-208` | item-pathはRustアイテムパス（"::" 区切り）であり、implブロック内の関数は"型名::関数名"とする。 | 詳細設計(design) → **詳細仕様(detailed_spec)** | 詳細仕様: 入力値の範囲・形式・制約 |
| `DES-211` | `@vtest.target` の値が `SRC-` で始まる場合はSRC ID参照として返す。 | 詳細設計(design) → **詳細仕様(detailed_spec)** | 詳細仕様: 入力値の範囲・形式・制約 |
| `DES-213` | coreは当該Testを管理宣言欠落として `chain_integrity` の `MISMATCH`（診断 `MIS… | 詳細設計(design) → **詳細仕様(detailed_spec)** | 詳細仕様: 正常系・異常系・境界値・状態ごとの振る舞い |
| `DES-215` | coreの `id` / `covers ≥ 1` / `intent`、および `rust-cargo` の `tar… | 詳細設計(design) → **詳細仕様(detailed_spec)** | 詳細仕様: エラーの外部的意味 |
| `DES-272` | collision時はE-SCAN-011とし、TargetRefを解決しない。 | 詳細設計(design) → **詳細仕様(detailed_spec)** | 詳細仕様: エラーの外部的意味 |
| `DES-274` | `filter`、`package`、`test_target` および `TestTarget` 型を `vtest-… | 詳細設計(design) → **基本設計(basic_design)** | 基本設計: コンポーネント間の境界 |
| `DES-275` | `vtest-adapter-api` は言語非依存の `TestWireCodec` capabilityを定義する。 | 詳細設計(design) → **基本設計(basic_design)** | 基本設計: 各コンポーネントの責務 |
| `DES-276` | codecはadapter固有のcompatibility propertyをJSON objectとしてencode … | 詳細設計(design) → **基本設計(basic_design)** | 基本設計: コンポーネント間の境界 |
| `DES-328` | Source Target hashは常にcanonical Locatorとconstruct bytesから計算し、… | 詳細設計(design) → **詳細仕様(detailed_spec)** | 詳細仕様: 仕様上の不変条件 |
| `DES-329` | canonical Locatorは恒久SRC IDの増減で変化しないため、参照方法の違いによってSource Targ… | 詳細設計(design) → **詳細仕様(detailed_spec)** | 詳細仕様: 仕様上の不変条件 |
| `DES-330` | 恒久SRC IDの宣言をconstruct bytesの内側へ置くadapterでは、その宣言の追加・変更・削除がcon… | 詳細設計(design) → **詳細仕様(detailed_spec)** | 詳細仕様: 正常系・異常系・境界値・状態ごとの振る舞い |
| `DES-331` | 恒久SRC IDの宣言をconstruct bytesの内側へ置くadapterでSource Target hashが… | 詳細設計(design) → **詳細仕様(detailed_spec)** | 詳細仕様: 正常系・異常系・境界値・状態ごとの振る舞い |
| `DES-333` | 恒久SRC IDを持つSource Targetも引き続きcanonical locatorでaddressableでな… | 詳細設計(design) → **詳細仕様(detailed_spec)** | 詳細仕様: 仕様上の不変条件 |
| `DES-335` | manifest等にある非隣接metadataも `metadata_sources` へ列挙するが、hash inpu… | 詳細設計(design) → **詳細仕様(detailed_spec)** | 詳細仕様: 仕様上の不変条件 |
| `DES-338` | provenance欠落はmalformed adapter outputとしてE-ADAPTER-002で拒否する。 | 詳細設計(design) → **詳細仕様(detailed_spec)** | 詳細仕様: エラーの外部的意味 |
| `DES-354` | 明示操作に必須のcapabilityがない場合はE-ADAPTER-004で操作を中止する。 | 詳細設計(design) → **詳細仕様(detailed_spec)** | 詳細仕様: エラーの外部的意味 |
| `DES-375` | 変化した場合はE-EXEC-004としてEvidenceを生成しない。 | 詳細設計(design) → **詳細仕様(detailed_spec)** | 詳細仕様: エラーの外部的意味 |
| `SPEC-433` | 新設機能は既存コマンド・ツールの引数と出力で露出する。 | 基本仕様(spec) → **基本設計(basic_design)** | 基本設計: 主要な外部・内部インターフェース |
| `SPEC-448` | 診断ラベルは検証状態と別軸である。 | 基本仕様(spec) → **詳細仕様(detailed_spec)** | 詳細仕様: 仕様上の不変条件 |

## 3. NONE の全件（一覧項目のどれにも当たらない）

| id | 文（60字まで） | 現在層 |
|---|---|---|
| `BD-154` | 基本仕様 §24.1 の layout をそのまま採用する。 | 基本設計(basic_design) |
| `BD-198` | 承認レコードの入力経路は別紙A §12.2・§13.2 に定める。 | 基本設計(basic_design) |
| `DS-223` | 検証対象は一般概念であり、このhashは検証対象をSource Targetとして実現した形態のidentity束縛であ… | 詳細仕様(detailed_spec) |
| `DS-241` | `--gate` に未定義名を指定した場合の扱いは §11.5 で定める。 | 詳細仕様(detailed_spec) |
| `DS-319` | 判断記録の有効性判定と実効判断の決定は §8.5 に従う。 | 詳細仕様(detailed_spec) |
| `DS-391` | したがって `rust-cargo` のTestは従来どおりSource Target宣言を要し、挙動・Eコード・fix… | 詳細仕様(detailed_spec) |
| `SPEC-415` | 本書は「基本仕様 v0.1」を実装可能なレベルまで具体化する。 | 基本仕様(spec) |
| `SPEC-416` | 本書は、基本仕様が定めた外部挙動の保証を変更しない。 | 基本仕様(spec) |
| `SPEC-417` | 本書と基本仕様の間に矛盾がある場合、基本仕様を正とし、本書の該当箇所を不整合として扱う。 | 基本仕様(spec) |
| `SPEC-418` | 本書は HOW（具体構文・アルゴリズム・データ構造・ID 命名・schema）を定める。 | 基本仕様(spec) |
| `SPEC-419` | 本書は、基本仕様（WHAT）に無い義務・検査・状態・文書種別・関係型を発明しない。 | 基本仕様(spec) |
| `SPEC-420` | 規範の伝播は上流から下流である。 | 基本仕様(spec) |
| `SPEC-421` | 矛盾・不足を発見した場合は、本書を書き換えず上流へフィードバックしOwner判断を経る。 | 基本仕様(spec) |
| `SPEC-422` | 本書からの `基本仕様 §n` 参照は、再導出済み基本仕様 v0.1 の連番（§0〜§30）を指す。 | 基本仕様(spec) |
| `SPEC-423` | 本書からの `要件定義 §n` 参照は、凍結要件定義 v0.1 の連番（§1〜§28・P-001〜P-005・R-1〜R… | 基本仕様(spec) |
| `SPEC-424` | 正規の詳細設計は3分冊とする。 | 基本仕様(spec) |
| `SPEC-425` | 節番号は正規文書間を通した連番とする。 | 基本仕様(spec) |
| `SPEC-426` | 別紙Bは非正規のprocess文書として別に扱う。 | 基本仕様(spec) |
| `SPEC-427` | 本冊（コア設計）は正規であり、§1〜§11、§16、§17、§19を収録節とする。 | 基本仕様(spec) |
| `SPEC-428` | 別紙A（CLI・MCPインターフェース仕様）は正規であり、§12〜§15を収録節とする。 | 基本仕様(spec) |
| `SPEC-429` | 別紙B（実装計画）は非正規/process文書であり、正規節番号を持たない。 | 基本仕様(spec) |
| `SPEC-430` | 別紙C（受入仕様）は正規であり、§18を収録節とする。 | 基本仕様(spec) |
| `SPEC-431` | 本冊の新設サブ節（§5.6 文書層孤児検出、§11.5 フェーズゲート、§11.6 役割別 projection、§11… | 基本仕様(spec) |
| `SPEC-432` | 本書は、基本仕様が固定するCLIコマンド一覧・MCPツール一覧を増やさない。 | 基本仕様(spec) |
| `SPEC-434` | 引数・入出力の完全schemaは別紙Aが定める。 | 基本仕様(spec) |
| `SPEC-435` | 本書は意味論とデータschema、および露出点だけを確定する。 | 基本仕様(spec) |

## 4. 出力ファイル

- `docs/canonical/relayer2/det1.json`: 700件全件の判定結果（`layer` / `list_item` / `statement_prefix` / `code_like`）。

