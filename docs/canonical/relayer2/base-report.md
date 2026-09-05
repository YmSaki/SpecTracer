# 基本仕様 v0.1 由来 765 文の演繹的再配置（relayer2）

対象: docs/canonical/specification.json の 4 層（spec/detailed_spec/basic_design/design）中、source.doc が docs/AI並列開発向けテスト検証システム 基本仕様 v0.1.md である全 765 文。

手順: docs/canonical/LAYERING-reference.md L37-L42 の層別「定義できていれば成立」一覧のどの項目に当たるかを1文ずつ決め、その項目の層へ機械的に置いた（判断ではなく一覧項目が決める）。判定手順は docs/canonical/LAYERING.md セクション2、キー対応はタスク指示（不変条件→詳細仕様、field構成→詳細設計、ID・値の形式→詳細仕様、コマンドの機能→基本仕様、部品ごとの責務→詳細設計、全体分担→基本設計）に従った。どの一覧項目にも当たらない文は現在の層のまま NONE とした。

作業は6分割（各128件前後）してサブエージェントに委譲し、本エージェントが統合・完全性検証（765件の欠落・重複なし、layer値の妥当性）を行った。

## 1. 層ごとの件数（Before to After）

| 層 | Before | After | 差分 |
|---|---:|---:|---:|
| 基本仕様(spec) | 414 | 255 | -159 |
| 詳細仕様(detailed_spec) | 219 | 337 | +118 |
| 基本設計(basic_design) | 128 | 108 | -20 |
| 詳細設計(design) | 4 | 65 | +61 |
| 合計 | 765 | 765 | 0 |

移動した文の数: 223 / 765

NONE（どの一覧項目にも当たらない）の数: 64 / 765

## 2. 移動した文の一覧（全223件）

| id | 文（60字まで） | 現在層 to 提案層 | 一覧項目 |
|---|---|---|---|
| SPEC-012 | `vtest` 自身はLLM APIを呼ばない。 | 基本仕様(spec) to 詳細仕様(detailed_spec) | 詳細仕様: 仕様上の不変条件 |
| SPEC-013 | `vtest` 自身は宣言と実装の意味的な良し悪しを裁定しない。 | 基本仕様(spec) to 詳細仕様(detailed_spec) | 詳細仕様: 仕様上の不変条件 |
| SPEC-014 | 機械が決定論で確定できない疑義は `UNKNOWN` として外部の判断者へ引き渡す（§11）。 | 基本仕様(spec) to 詳細仕様(detailed_spec) | 詳細仕様: 正常系・異常系・境界値・状態ごとの振る舞い |
| SPEC-015 | coreの検証契約は言語・test runnerに依存しない。 | 基本仕様(spec) to 基本設計(basic_design) | 基本設計: 主要な依存関係 |
| SPEC-016 | documentとは、ソースコードより上流に位置する成果物を表す単一の総称ノードである。 | 基本仕様(spec) to 詳細設計(design) | 詳細設計: 入出力データの構造 |
| SPEC-018 | documentは `id + path + content_hash + 上流参照（derives_from）` を持 | 基本仕様(spec) to 詳細設計(design) | 詳細設計: 入出力データの構造 |
| SPEC-019 | 文書種別ごとの専用スキーマは設けない。 | 基本仕様(spec) to 詳細仕様(detailed_spec) | 詳細仕様: 仕様上の不変条件 |
| SPEC-020 | 対象ソースコード自身のdoc commentは、その対象実装の唯一の仕様根拠としては用いない。 | 基本仕様(spec) to 詳細仕様(detailed_spec) | 詳細仕様: 仕様上の不変条件 |
| SPEC-021 | derives_fromとは、document間の唯一のリンク種別である。 | 基本仕様(spec) to 詳細仕様(detailed_spec) | 詳細仕様: 仕様上の不変条件 |
| SPEC-023 | 各derives_fromリンクは任意（optional）の説明文・導出理由を保持できる（§3.2）。 | 基本仕様(spec) to 詳細設計(design) | 詳細設計: 入出力データの構造 |
| SPEC-026 | VOとdocumentの間に他のエンティティ層を置かない。 | 基本仕様(spec) to 詳細仕様(detailed_spec) | 詳細仕様: 仕様上の不変条件 |
| SPEC-028 | VOの粒度をassert文・test function・テストファイルなどのコード構文で決めない。 | 基本仕様(spec) to 詳細仕様(detailed_spec) | 詳細仕様: 仕様上の不変条件 |
| SPEC-033 | Test Intentは宣言鎖のノードではない。 | 基本仕様(spec) to 詳細仕様(detailed_spec) | 詳細仕様: 仕様上の不変条件 |
| SPEC-037 | Source Targetは、adapter IDとadapter所有のopaque locatorからなるTarget | 基本仕様(spec) to 詳細設計(design) | 詳細設計: 入出力データの構造 |
| SPEC-039 | Execution Evidenceは結果・実行時リポジトリ状態・解決後のcanonical Source Target | 基本仕様(spec) to 詳細設計(design) | 詳細設計: 入出力データの構造 |
| SPEC-040 | Execution Evidenceは検証対象の内容ハッシュに束縛される（§6）。 | 基本仕様(spec) to 詳細仕様(detailed_spec) | 詳細仕様: 仕様上の不変条件 |
| SPEC-042 | 判断記録はactor / subject / decisionを必須項目とする。 | 基本仕様(spec) to 詳細設計(design) | 詳細設計: 入出力データの構造 |
| SPEC-043 | 判断記録の理由・根拠は任意とする。 | 基本仕様(spec) to 詳細設計(design) | 詳細設計: 入出力データの構造 |
| SPEC-044 | 判断記録は依存closureのハッシュに束縛される（§11）。 | 基本仕様(spec) to 詳細仕様(detailed_spec) | 詳細仕様: 仕様上の不変条件 |
| SPEC-046 | 承認記録はapprover / subject（またはjudgment reference）/ approved sta | 基本仕様(spec) to 詳細設計(design) | 詳細設計: 入出力データの構造 |
| SPEC-047 | 承認記録は上流依存closureのハッシュに束縛される。 | 基本仕様(spec) to 詳細仕様(detailed_spec) | 詳細仕様: 仕様上の不変条件 |
| SPEC-049 | 検証状態は `PASS` / `FAIL` / `MISMATCH` / `NO_EVIDENCE` / `UNKNOW | 基本仕様(spec) to 詳細仕様(detailed_spec) | 詳細仕様: すべての出力結果 |
| SPEC-050 | 検証状態は検証結果のみを表す。 | 基本仕様(spec) to 詳細仕様(detailed_spec) | 詳細仕様: 仕様上の不変条件 |
| SPEC-051 | 検証状態に承認状態を混入させない。 | 基本仕様(spec) to 詳細仕様(detailed_spec) | 詳細仕様: 仕様上の不変条件 |
| SPEC-053 | 診断ラベルは検証状態ではない。 | 基本仕様(spec) to 詳細仕様(detailed_spec) | 詳細仕様: 仕様上の不変条件 |
| SPEC-055 | 検査は `chain_integrity` / `orphan_detection` / `target_binding | 基本仕様(spec) to 詳細仕様(detailed_spec) | 詳細仕様: 仕様上の不変条件 |
| SPEC-056 | 完全検証とは、宣言鎖全体に対する検査（`chain_integrity` / `orphan_detection`）と、 | 基本仕様(spec) to 詳細仕様(detailed_spec) | 詳細仕様: 受入条件・検証可能な期待結果 |
| SPEC-057 | 完全検証は一項目でも非 `PASS` があればNGとする（fail-closed）。 | 基本仕様(spec) to 詳細仕様(detailed_spec) | 詳細仕様: 仕様上の不変条件 |
| SPEC-059 | scopeを狭めても対象外項目を `PASS` へ書き換えない。 | 基本仕様(spec) to 詳細仕様(detailed_spec) | 詳細仕様: 仕様上の不変条件 |
| SPEC-060 | 正典（source of truth）とは、ある事実を決定する唯一の記録である。 | 基本仕様(spec) to 基本設計(basic_design) | 基本設計: データの所有主体 |
| SPEC-061 | 正典から導出できる情報は派生情報とし独立保存しない。 | 基本仕様(spec) to 詳細仕様(detailed_spec) | 詳細仕様: 仕様上の不変条件 |
| SPEC-063 | 本システムは扱う情報を三層に分ける。 | 基本仕様(spec) to 基本設計(basic_design) | 基本設計: システム全体の構成 |
| SPEC-064 | 実装層は、テストコード本体と対象ソースコードからなり、Gitで管理される正典である。 | 基本仕様(spec) to 基本設計(basic_design) | 基本設計: 主要な永続化方式 |
| SPEC-065 | 派生情報（検索インデックス、検証グラフ、集約結果）は上記から毎回再構築する。 | 基本仕様(spec) to 基本設計(basic_design) | 基本設計: 主要なデータの流れ |
| SPEC-066 | 派生情報はGit管理しない。 | 基本仕様(spec) to 基本設計(basic_design) | 基本設計: 主要な永続化方式 |
| SPEC-067 | adapterが返す導出結果はregistryでmergeし、adapter ID・path・Test IDの順に正規化 | 基本仕様(spec) to 詳細設計(design) | 詳細設計: 構成要素ごとの責務 |
| SPEC-068 | registryの重複ID、未登録adapter、adapter間のTest ID重複は操作エラーとする。 | 基本仕様(spec) to 詳細仕様(detailed_spec) | 詳細仕様: 例外条件 |
| SPEC-069 | registryの重複ID、未登録adapter、adapter間のTest ID重複は空のscanとして成功扱いしない | 基本仕様(spec) to 詳細仕様(detailed_spec) | 詳細仕様: エラーの外部的意味 |
| SPEC-071 | どれかを正として他を修正させることはしない。 | 基本仕様(spec) to 詳細仕様(detailed_spec) | 詳細仕様: 仕様上の不変条件 |
| SPEC-073 | 上流文書はすべて単一の総称ノード型 `document` で表現する。 | 基本仕様(spec) to 詳細設計(design) | 詳細設計: 入出力データの構造 |
| SPEC-074 | 文書間リンクは `derives_from` の一種のみとする。 | 基本仕様(spec) to 詳細仕様(detailed_spec) | 詳細仕様: 仕様上の不変条件 |
| SPEC-076 | 文書層の段数は総称的に扱い、リンクを追加してもスキーマが壊れないことを設計制約とする。 | 基本仕様(spec) to 基本設計(basic_design) | 基本設計: システム全体の構成 |
| SPEC-077 | 段はリンクであって検査ではない。 | 基本仕様(spec) to 詳細仕様(detailed_spec) | 詳細仕様: 仕様上の不変条件 |
| SPEC-079 | VOとdocumentの間に他のエンティティ層を置かない。 | 基本仕様(spec) to 詳細仕様(detailed_spec) | 詳細仕様: 仕様上の不変条件 |
| SPEC-080 | 本システムは文書内容の意味的な良し悪しに関知しない。 | 基本仕様(spec) to 詳細仕様(detailed_spec) | 詳細仕様: 仕様上の不変条件 |
| SPEC-081 | 文書種別ごとの専用スキーマ・文書間リンク意味論の増殖・文書内容の良否検証を行わない。 | 基本仕様(spec) to 詳細仕様(detailed_spec) | 詳細仕様: 仕様上の不変条件 |
| SPEC-082 | 不一致はどちらが正かを決めない。 | 基本仕様(spec) to 詳細仕様(detailed_spec) | 詳細仕様: 仕様上の不変条件 |
| SPEC-085 | Test→VO、Test→SRCの関係を外部ファイルへ重複保存しない。 | 基本仕様(spec) to 詳細仕様(detailed_spec) | 詳細仕様: 仕様上の不変条件 |
| SPEC-086 | graphと現在のtarget集合は常にadapter所有のTest metadata宣言から再構築する。 | 基本仕様(spec) to 詳細仕様(detailed_spec) | 詳細仕様: 仕様上の不変条件 |
| SPEC-087 | graphと現在のtarget集合はEvidenceのtarget参照から関係を生成・修復しない。 | 基本仕様(spec) to 詳細仕様(detailed_spec) | 詳細仕様: 仕様上の不変条件 |
| SPEC-088 | Evidenceに含むtarget参照は、target別の実行事実と内容ハッシュを束縛する実行時snapshot key | 基本仕様(spec) to 詳細設計(design) | 詳細設計: 入出力データの構造 |
| SPEC-089 | Evidenceに含むtarget参照はTest→SRC関係の正典ではない。 | 基本仕様(spec) to 詳細仕様(detailed_spec) | 詳細仕様: 仕様上の不変条件 |
| SPEC-090 | 外部レコードとして保存するのは、どちらか一方のエンティティに自然に所属しない関係（VO間の依存、Test間の補完関係など | 基本仕様(spec) to 基本設計(basic_design) | 基本設計: データの所有主体 |
| SPEC-091 | readerは読み取りだけで正典を書き換えない。 | 基本仕様(spec) to 詳細仕様(detailed_spec) | 詳細仕様: 仕様上の不変条件 |
| SPEC-092 | documentは総称の上流文書ノード（path＋content_hash＋derives_from）である。 | 基本仕様(spec) to 詳細設計(design) | 詳細設計: 入出力データの構造 |
| SPEC-093 | documentは種別専用スキーマを持たない。 | 基本仕様(spec) to 詳細仕様(detailed_spec) | 詳細仕様: 仕様上の不変条件 |
| SPEC-098 | Source Targetの恒久IDは必須としない。 | 基本仕様(spec) to 詳細仕様(detailed_spec) | 詳細仕様: 仕様上の不変条件 |
| SPEC-099 | Relationは外部関係レコードであり、不変とする。 | 基本仕様(spec) to 詳細設計(design) | 詳細設計: 永続化ルール |
| SPEC-100 | derives_fromの説明文もRelationに保持できる。 | 基本仕様(spec) to 詳細設計(design) | 詳細設計: 入出力データの構造 |
| SPEC-101 | 判断記録は `UNKNOWN` への外部判断であり、追記型とする。 | 基本仕様(spec) to 詳細設計(design) | 詳細設計: 永続化ルール |
| SPEC-102 | 承認記録は判断・方針の正式採用であり、追記型とする。 | 基本仕様(spec) to 詳細設計(design) | 詳細設計: 永続化ルール |
| SPEC-103 | Execution Evidenceは実行証拠レコードであり、追記型とする。 | 基本仕様(spec) to 詳細設計(design) | 詳細設計: 永続化ルール |
| SPEC-104 | documentは単一の総称ノードであり、要件定義・基本仕様・詳細設計・API Schema等を種別で区別する専用スキー | 基本仕様(spec) to 詳細仕様(detailed_spec) | 詳細仕様: 仕様上の不変条件 |
| SPEC-106 | 段を増やしても種別を増やさない。 | 基本仕様(spec) to 詳細仕様(detailed_spec) | 詳細仕様: 仕様上の不変条件 |
| SPEC-107 | ツールはID形式を強制せず一意性のみを強制する。 | 基本仕様(spec) to 詳細仕様(detailed_spec) | 詳細仕様: 入力値の範囲・形式・制約 |
| SPEC-108 | IDの一意性はスキャン時に全数検査する。 | 基本仕様(spec) to 詳細仕様(detailed_spec) | 詳細仕様: 入力値の範囲・形式・制約 |
| SPEC-109 | 関係リンクは説明文・導出理由を任意（optional）で保持できる。 | 基本仕様(spec) to 詳細設計(design) | 詳細設計: 入出力データの構造 |
| SPEC-110 | derives_from・covers・検証対象・実装traceabilityなど性質の異なる関係型は潰さず区別する。 | 基本仕様(spec) to 詳細仕様(detailed_spec) | 詳細仕様: 仕様上の不変条件 |
| SPEC-111 | 関係型そのものの意味論的増殖は求めない。 | 基本仕様(spec) to 詳細仕様(detailed_spec) | 詳細仕様: 仕様上の不変条件 |
| SPEC-113 | ソースコードへ恒久IDを埋め込むことは必須としない。 | 基本仕様(spec) to 詳細仕様(detailed_spec) | 詳細仕様: 仕様上の不変条件 |
| SPEC-115 | Target Referenceは、adapter IDとadapter所有のopaque locatorの組、または任 | 基本仕様(spec) to 詳細設計(design) | 詳細設計: 入出力データの構造 |
| SPEC-116 | 共通契約がpath・module・function等の特定言語構造を必須としてはならない。 | 基本仕様(spec) to 基本設計(basic_design) | 基本設計: 主要な外部・内部インターフェース |
| SPEC-117 | 1つのTestは1件以上のSource Targetを持ち、各target参照を個別に保持する。 | 基本仕様(spec) to 詳細仕様(detailed_spec) | 詳細仕様: 仕様上の不変条件 |
| SPEC-118 | Source Targetは代表1件へ縮約しない。 | 基本仕様(spec) to 詳細仕様(detailed_spec) | 詳細仕様: 仕様上の不変条件 |
| SPEC-119 | 検証状態は `PASS` / `FAIL` / `MISMATCH` / `NO_EVIDENCE` / `UNKNOW | 基本仕様(spec) to 詳細仕様(detailed_spec) | 詳細仕様: すべての出力結果 |
| SPEC-121 | 意味の違いは資格にならない。 | 基本仕様(spec) to 詳細仕様(detailed_spec) | 詳細仕様: 仕様上の不変条件 |
| SPEC-122 | `PASS` を受け取った者はマージできる。 | 基本仕様(spec) to 詳細仕様(detailed_spec) | 詳細仕様: 正常系・異常系・境界値・状態ごとの振る舞い |
| SPEC-123 | `FAIL` を受け取った者は実装（テスト実装を含む）を直す。 | 基本仕様(spec) to 詳細仕様(detailed_spec) | 詳細仕様: 正常系・異常系・境界値・状態ごとの振る舞い |
| SPEC-124 | `MISMATCH` を受け取った者はコードを触る前に宣言側（上流）を直す。 | 基本仕様(spec) to 詳細仕様(detailed_spec) | 詳細仕様: 正常系・異常系・境界値・状態ごとの振る舞い |
| SPEC-125 | `NO_EVIDENCE` を受け取った者は証拠を作る（機械的に解決可能）。 | 基本仕様(spec) to 詳細仕様(detailed_spec) | 詳細仕様: 正常系・異常系・境界値・状態ごとの振る舞い |
| SPEC-126 | `UNKNOWN` は決定論の限界であり、受け取った者は意味判定できる者へエスカレーションする。 | 基本仕様(spec) to 詳細仕様(detailed_spec) | 詳細仕様: 正常系・異常系・境界値・状態ごとの振る舞い |
| SPEC-128 | 診断ラベルは検証状態ではない。 | 基本仕様(spec) to 詳細仕様(detailed_spec) | 詳細仕様: 仕様上の不変条件 |
| SPEC-130 | 本書は状態と診断ラベルを常に別軸として扱い、混同しない。 | 基本仕様(spec) to 詳細仕様(detailed_spec) | 詳細仕様: 仕様上の不変条件 |
| SPEC-133 | 内部エラー・入力不正は検証状態と別系統（終了コード。§27）で表現する。 | 基本仕様(spec) to 詳細仕様(detailed_spec) | 詳細仕様: エラーの外部的意味 |
| SPEC-134 | `UNKNOWN` をエラー処理のフォールバック先として使う実装は仕様違反とする。 | 基本仕様(spec) to 詳細仕様(detailed_spec) | 詳細仕様: 仕様上の不変条件 |
| SPEC-135 | 検証状態（§4.1の5状態）は検証結果のみを表す。 | 基本仕様(spec) to 詳細仕様(detailed_spec) | 詳細仕様: 仕様上の不変条件 |
| SPEC-136 | 検証状態は承認状態を混入させない。 | 基本仕様(spec) to 詳細仕様(detailed_spec) | 詳細仕様: 仕様上の不変条件 |
| SPEC-139 | 検査軸は実施する検査（4本の部分集合）を指定する。 | 基本仕様(spec) to 詳細仕様(detailed_spec) | 詳細仕様: 入力値の範囲・形式・制約 |
| SPEC-140 | エンティティ軸は対象とするdocument / VO / Testの部分木を指定する。 | 基本仕様(spec) to 詳細仕様(detailed_spec) | 詳細仕様: 入力値の範囲・形式・制約 |
| SPEC-149 | すべてのTestを管理対象とすることと、当該Testを仕様適合の証拠として算入すること（§8）は別個の条件とする。 | 基本仕様(spec) to 詳細仕様(detailed_spec) | 詳細仕様: 仕様上の不変条件 |
| SPEC-157 | 特定形態の確認方法を別形態のTestへ一律要求しない。 | 基本仕様(spec) to 詳細仕様(detailed_spec) | 詳細仕様: 仕様上の不変条件 |
| SPEC-161 | 照合内容が宣言の期待と意味的に一致するかはoracle_presence検査の主張に含めない。 | 基本仕様(spec) to 詳細仕様(detailed_spec) | 詳細仕様: 仕様上の不変条件 |
| SPEC-163 | 実行形態別の判定規則を設けない。 | 基本仕様(spec) to 詳細仕様(detailed_spec) | 詳細仕様: 仕様上の不変条件 |
| SPEC-164 | `rust-cargo` adapterのStatic Audit capabilityは、§8.3の不成立構造を決定論 | 基本仕様(spec) to 詳細設計(design) | 詳細設計: 構成要素ごとの責務 |
| SPEC-165 | 判定は保守的に行う。 | 基本仕様(spec) to 詳細仕様(detailed_spec) | 詳細仕様: 仕様上の不変条件 |
| SPEC-166 | 共通契約がRust構文を要求しない。 | 基本仕様(spec) to 詳細仕様(detailed_spec) | 詳細仕様: 仕様上の不変条件 |
| SPEC-142 | 鎖に段（リンク）が増えても検査は増えない。 | 基本仕様(spec) to 詳細仕様(detailed_spec) | 詳細仕様: 仕様上の不変条件 |
| SPEC-145 | 凍結要件が検査から明示的に排除した判断（仕様網羅・VO網羅・VO分解妥当性・意味一致・実装一致）は、本書でも検査に含めな | 基本仕様(spec) to 詳細仕様(detailed_spec) | 詳細仕様: 仕様上の不変条件 |
| SPEC-167 | 証拠は検証対象の内容ハッシュに束縛される。 | 基本仕様(spec) to 詳細仕様(detailed_spec) | 詳細仕様: 仕様上の不変条件 |
| SPEC-168 | 鮮度の独立検査は設けない。 | 基本仕様(spec) to 詳細仕様(detailed_spec) | 詳細仕様: 仕様上の不変条件 |
| SPEC-177 | Testとして成立しているかの検査（§8）と、仕様適合性の証拠として算入するかの判定は独立である。 | 基本仕様(spec) to 詳細仕様(detailed_spec) | 詳細仕様: 仕様上の不変条件 |
| SPEC-178 | 全Testを管理対象とすること（`chain_integrity`）と証拠算入（成立性）は別系統とする。 | 基本仕様(spec) to 詳細仕様(detailed_spec) | 詳細仕様: 仕様上の不変条件 |
| SPEC-179 | 成立条件の確認方法は検証対象・実行形態・観測方法に応じて異なってよい（証明方法への非依存）。 | 基本仕様(spec) to 詳細仕様(detailed_spec) | 詳細仕様: 正常系・異常系・境界値・状態ごとの振る舞い |
| SPEC-180 | 特定形態固有の確認方法を別形態へ一律要求しない。 | 基本仕様(spec) to 詳細仕様(detailed_spec) | 詳細仕様: 仕様上の不変条件 |
| SPEC-184 | 共通契約がRust構文を要求しない。 | 基本仕様(spec) to 詳細仕様(detailed_spec) | 詳細仕様: 仕様上の不変条件 |
| SPEC-186 | `static_audit` に相当する判定は独立した検査項目を新設しない。 | 基本仕様(spec) to 詳細仕様(detailed_spec) | 詳細仕様: 仕様上の不変条件 |
| SPEC-191 | 複数targetを宣言した場合も代表1件へ縮約しない。 | 基本仕様(spec) to 詳細仕様(detailed_spec) | 詳細仕様: 仕様上の不変条件 |
| SPEC-192 | ソースコードへの恒久ID埋め込みは必須としない。 | 基本仕様(spec) to 詳細仕様(detailed_spec) | 詳細仕様: 仕様上の不変条件 |
| SPEC-194 | 共通契約が特定言語構造を必須としない。 | 基本仕様(spec) to 詳細仕様(detailed_spec) | 詳細仕様: 仕様上の不変条件 |
| SPEC-198 | 検証対象と実装traceabilityは別の関係として扱う。 | 基本仕様(spec) to 詳細仕様(detailed_spec) | 詳細仕様: 仕様上の不変条件 |
| SPEC-225 | `UNKNOWN` はエラー処理のフォールバック先に使わない（§4.4）。 | 基本仕様(spec) to 詳細仕様(detailed_spec) | 詳細仕様: 仕様上の不変条件 |
| SPEC-230 | 判断記録の生成・保存の構造化プロトコルは検証状態のゲートではない。 | 基本仕様(spec) to 詳細仕様(detailed_spec) | 詳細仕様: 仕様上の不変条件 |
| SPEC-231 | 判断済みと承認済みは区別する（判断済み ≠ 承認済み）。 | 基本仕様(spec) to 詳細仕様(detailed_spec) | 詳細仕様: 仕様上の不変条件 |
| SPEC-234 | 判断記録と承認記録は同一entityであることを要求しない（別entityでありうる）。 | 基本仕様(spec) to 詳細設計(design) | 詳細設計: 実装者に委ねてよい範囲 |
| SPEC-238 | 診断severityと検証状態を混同しない。 | 基本仕様(spec) to 詳細仕様(detailed_spec) | 詳細仕様: 仕様上の不変条件 |
| SPEC-239 | Testの存在理由による分類（role / anchor / anchor_rationale等）と、それに基づく `c | 基本仕様(spec) to 詳細仕様(detailed_spec) | 詳細仕様: 仕様上の不変条件 |
| SPEC-260 | source declarationが正典であるため、`covers` / `targets` の「同期漏れ」は構造的に | 基本仕様(spec) to 詳細仕様(detailed_spec) | 詳細仕様: 仕様上の不変条件 |
| SPEC-263 | 境界値・partitionの必須入力化は組込Formでは設けない。 | 基本仕様(spec) to 詳細仕様(detailed_spec) | 詳細仕様: 仕様上の不変条件 |
| SPEC-264 | 境界値・partitionの必須入力化はuser-defined Form Schemaが指定できる。 | 基本仕様(spec) to 詳細仕様(detailed_spec) | 詳細仕様: 入力値の範囲・形式・制約 |
| SPEC-267 | 取り込まれた上流成果物はcontent_hashとderives_fromを持つ。 | 基本仕様(spec) to 詳細設計(design) | 詳細設計: 入出力データの構造 |
| SPEC-268 | 対象ソースコード内のdoc commentを、その対象実装自身の正当性を証明する唯一の仕様根拠として使用しない。 | 基本仕様(spec) to 詳細仕様(detailed_spec) | 詳細仕様: 仕様上の不変条件 |
| SPEC-271 | 判断済みと承認済みは区別する（判断済み ≠ 承認済み）。 | 基本仕様(spec) to 詳細仕様(detailed_spec) | 詳細仕様: 仕様上の不変条件 |
| SPEC-272 | 未承認の判断は承認済みより弱い。 | 基本仕様(spec) to 詳細仕様(detailed_spec) | 詳細仕様: 仕様上の不変条件 |
| SPEC-275 | §11の `UNKNOWN` 判断も承認対象になり得る。 | 基本仕様(spec) to 詳細仕様(detailed_spec) | 詳細仕様: 正常系・異常系・境界値・状態ごとの振る舞い |
| SPEC-276 | 判断できることと正式承認は別段階である。 | 基本仕様(spec) to 詳細仕様(detailed_spec) | 詳細仕様: 仕様上の不変条件 |
| SPEC-277 | 承認記録は§11の判断記録と同一entityであることを要求しない。 | 基本仕様(spec) to 詳細仕様(detailed_spec) | 詳細仕様: 仕様上の不変条件 |
| SPEC-278 | 承認主体を人間に限定しない。 | 基本仕様(spec) to 詳細仕様(detailed_spec) | 詳細仕様: 仕様上の不変条件 |
| SPEC-280 | 全Agentが承認権限を持つことは要求しない。 | 基本仕様(spec) to 詳細仕様(detailed_spec) | 詳細仕様: 仕様上の不変条件 |
| SPEC-281 | 一般作業Agentが承認権限を持つべきとも要求しない。 | 基本仕様(spec) to 詳細仕様(detailed_spec) | 詳細仕様: 仕様上の不変条件 |
| SPEC-283 | 承認は検証状態と独立の別軸である（§4.5）。 | 基本仕様(spec) to 詳細仕様(detailed_spec) | 詳細仕様: 仕様上の不変条件 |
| SPEC-293 | 本システムが意味判断・候補生成を行うことを必須要件としない。 | 基本仕様(spec) to 詳細仕様(detailed_spec) | 詳細仕様: 仕様上の不変条件 |
| SPEC-295 | 外部AI/Agentの能力を検証成立条件にしない。 | 基本仕様(spec) to 詳細仕様(detailed_spec) | 詳細仕様: 仕様上の不変条件 |
| SPEC-313 | 検証状態と承認は独立の軸である（§4.5）。 | 基本仕様(spec) to 詳細仕様(detailed_spec) | 詳細仕様: 仕様上の不変条件 |
| SPEC-316 | 本システムの責務はゲート条件が現在満たされているかの評価・提示に限る。 | 基本仕様(spec) to 詳細仕様(detailed_spec) | 詳細仕様: 仕様上の不変条件 |
| SPEC-317 | フェーズのライフサイクル管理・工程の自動遷移は責務外とする。 | 基本仕様(spec) to 詳細仕様(detailed_spec) | 詳細仕様: 仕様上の不変条件 |
| SPEC-318 | 「Releaseフェーズへ遷移させる」のではなく「Release gateの条件を現在満たしている」を提示する。 | 基本仕様(spec) to 詳細仕様(detailed_spec) | 詳細仕様: 受入条件・検証可能な期待結果 |
| SPEC-320 | 鮮度は独立検査ではなく§6のハッシュ束縛により満たす。 | 基本仕様(spec) to 詳細仕様(detailed_spec) | 詳細仕様: 仕様上の不変条件 |
| SPEC-321 | 利用者向け簡易出力は `OK` / `NG` の二値とする。 | 基本仕様(spec) to 詳細仕様(detailed_spec) | 詳細仕様: すべての出力結果 |
| SPEC-322 | 完全検証の検査集合はこの4検査に固定する。 | 基本仕様(spec) to 詳細仕様(detailed_spec) | 詳細仕様: 仕様上の不変条件 |
| SPEC-325 | 集約はfail-closedを基本とする。 | 基本仕様(spec) to 詳細仕様(detailed_spec) | 詳細仕様: 仕様上の不変条件 |
| SPEC-328 | 簡易出力は総合OK / NGとする。 | 基本仕様(spec) to 詳細仕様(detailed_spec) | 詳細仕様: すべての出力結果 |
| SPEC-331 | `covers` を持つTestはVOの子として表示する。 | 基本仕様(spec) to 詳細仕様(detailed_spec) | 詳細仕様: 正常系・異常系・境界値・状態ごとの振る舞い |
| SPEC-334 | 診断severityと検証状態を混同しない。 | 基本仕様(spec) to 詳細仕様(detailed_spec) | 詳細仕様: 仕様上の不変条件 |
| SPEC-335 | content_hash照合は決定論的に解決する。 | 基本仕様(spec) to 詳細仕様(detailed_spec) | 詳細仕様: 仕様上の不変条件 |
| SPEC-336 | 参照位置の意味的妥当性・取り込み完全性は検査対象としない。 | 基本仕様(spec) to 詳細仕様(detailed_spec) | 詳細仕様: 仕様上の不変条件 |
| SPEC-337 | 参照位置の意味的妥当性・取り込み完全性は必要ならエスカレーション（§11）で扱う。 | 基本仕様(spec) to 詳細仕様(detailed_spec) | 詳細仕様: 例外条件 |
| SPEC-338 | Relationレコードは不変とする。 | 基本仕様(spec) to 詳細仕様(detailed_spec) | 詳細仕様: 仕様上の不変条件 |
| SPEC-340 | 並列編集耐性では部分書込みの検出・修復は行わない。 | 基本仕様(spec) to 詳細仕様(detailed_spec) | 詳細仕様: 仕様上の不変条件 |
| SPEC-343 | 検証グラフ、逆引きインデックス、集約結果はすべて正典からの導出物である。 | 基本仕様(spec) to 詳細仕様(detailed_spec) | 詳細仕様: 仕様上の不変条件 |
| SPEC-358 | ゲート充足は検証状態とは別軸の評価である。 | 基本仕様(spec) to 詳細仕様(detailed_spec) | 詳細仕様: 仕様上の不変条件 |
| SPEC-359 | 出力では検証状態とゲート満否を別に提示する。 | 基本仕様(spec) to 詳細仕様(detailed_spec) | 詳細仕様: 受入条件・検証可能な期待結果 |
| SPEC-361 | MCPサーバはCLIと同一のコア機能を呼び出す。 | 基本仕様(spec) to 基本設計(basic_design) | 基本設計: 主要な外部・内部インターフェース |
| SPEC-363 | すべてのツールは非対話で完結する。 | 基本仕様(spec) to 詳細仕様(detailed_spec) | 詳細仕様: 仕様上の不変条件 |
| SPEC-364 | CLIとMCPは同じadapter registry composition・JSON envelope・adapter | 基本仕様(spec) to 基本設計(basic_design) | 基本設計: 主要な外部・内部インターフェース |
| SPEC-366 | 検証契約・ID・ハッシュ・Evidence・状態・集約の概念モデルは、言語およびtest runnerに依存しない。 | 基本仕様(spec) to 詳細仕様(detailed_spec) | 詳細仕様: 仕様上の不変条件 |
| SPEC-367 | 共通契約は特定言語の構文・構造を必須としない。 | 基本仕様(spec) to 詳細仕様(detailed_spec) | 詳細仕様: 仕様上の不変条件 |
| SPEC-371 | NFR-001並列性への対応は、1レコード1ファイル、ULIDファイル名、不変Relation、中央台帳の不在とする（§ | 基本仕様(spec) to 基本設計(basic_design) | 基本設計: 主要な永続化方式 |
| SPEC-372 | NFR-002再現性への対応は、Evidenceのリビジョン束縛、決定論的解析の再実行可能性、scanによる全再構築とす | 基本仕様(spec) to 基本設計(basic_design) | 基本設計: 主要なデータの流れ |
| SPEC-374 | NFR-004再構築可能性への対応は、派生情報はcacheのみとし、正典から `vtest scan` で再構築すること | 基本仕様(spec) to 基本設計(basic_design) | 基本設計: 主要なデータの流れ |
| SPEC-380 | 文書層は§2.2の通りリンクとハッシュのみを扱う（OOS-001仕様書同士の品質監査）。 | 基本仕様(spec) to 詳細仕様(detailed_spec) | 詳細仕様: 仕様上の不変条件 |
| SPEC-381 | 文書内容の意味的良否を検証しない（OOS-001仕様書同士の品質監査）。 | 基本仕様(spec) to 詳細仕様(detailed_spec) | 詳細仕様: 仕様上の不変条件 |
| SPEC-382 | 不一致はどれを正とするか決めず状態として提示する（OOS-002修正方針決定。§4）。 | 基本仕様(spec) to 詳細仕様(detailed_spec) | 詳細仕様: 仕様上の不変条件 |
| SPEC-383 | Test Edit対象外の一般編集を管理しない（OOS-003通常ソースコード編集管理。§15.3）。 | 基本仕様(spec) to 詳細仕様(detailed_spec) | 詳細仕様: 仕様上の不変条件 |
| SPEC-384 | フェーズのライフサイクル管理・工程遷移は責務外とする（OOS-004開発プロセス全体の管理。§20）。 | 基本仕様(spec) to 詳細仕様(detailed_spec) | 詳細仕様: 仕様上の不変条件 |
| DS-015 | ULID payloadにより並列生成時のファイル名衝突を実用上排除する。 | 詳細仕様(detailed_spec) to 詳細設計(design) | 詳細設計: 入出力データの構造 |
| DS-081 | Evidenceの判定結果を変えうるTestの意味・実行条件・対象実装・実行可能状態が現在状態と一致することを確認できな | 詳細仕様(detailed_spec) to 基本設計(basic_design) | 基本設計: 整合性境界 |
| DS-083 | Testの内容ハッシュは Test construct だけでなく Test subject 全体（少なくとも adap | 詳細仕様(detailed_spec) to 詳細設計(design) | 詳細設計: 入出力データの構造 |
| DS-113 | 判断記録は少なくとも「誰が（actor）」「何を（subject）」「どう判断したか（decision）」を必須項目とす | 詳細仕様(detailed_spec) to 詳細設計(design) | 詳細設計: 入出力データの構造 |
| DS-116 | `vtest` が判断対象の情報一式（VO、Test Intent、テストコード、対象実装、関連テスト、既知partit | 詳細仕様(detailed_spec) to 詳細設計(design) | 詳細設計: 入出力データの構造 |
| DS-137 | adapterが現状との差分を計算してTest constructとmetadata宣言を更新する。 | 詳細仕様(detailed_spec) to 詳細設計(design) | 詳細設計: 構成要素ごとの責務 |
| DS-138 | coreが結果を再スキャンして検証する。 | 詳細仕様(detailed_spec) to 詳細設計(design) | 詳細設計: 構成要素ごとの責務 |
| DS-140 | 解決不能な場合はadapterが候補を提示する。 | 詳細仕様(detailed_spec) to 詳細設計(design) | 詳細設計: 構成要素ごとの責務 |
| DS-144 | registryは `kind` からちょうど1件のStructured Test adapterへ解決できる場合だけ操 | 詳細仕様(detailed_spec) to 詳細設計(design) | 詳細設計: 構成要素ごとの責務 |
| DS-145 | registryは重複・未知adapter・未対応capability・曖昧な対応を拒否する。 | 詳細仕様(detailed_spec) to 詳細設計(design) | 詳細設計: 構成要素ごとの責務 |
| DS-146 | 未知のformをcoreがRust用として推測してはならない。 | 詳細仕様(detailed_spec) to 詳細設計(design) | 詳細設計: 構成要素ごとの責務 |
| DS-154 | 承認記録は「誰が（approver）」「何を（subject または judgment reference）」「どの承認 | 詳細仕様(detailed_spec) to 詳細設計(design) | 詳細設計: 入出力データの構造 |
| DS-156 | 承認主体は種別（`human` / `agent`）と識別子（エージェント名・モデル名等）を記録する。 | 詳細仕様(detailed_spec) to 詳細設計(design) | 詳細設計: 入出力データの構造 |
| DS-164 | Evidenceには少なくともTest IDと実行結果（ランナーが報告した `PASS` / `FAIL`）を含める。 | 詳細仕様(detailed_spec) to 詳細設計(design) | 詳細設計: 入出力データの構造 |
| DS-165 | Evidenceには実行したadapter IDを含める。 | 詳細仕様(detailed_spec) to 詳細設計(design) | 詳細設計: 入出力データの構造 |
| DS-166 | Evidenceには実行時のリポジトリリビジョン（Git commit hash）とdirtyフラグを含める。 | 詳細仕様(detailed_spec) to 詳細設計(design) | 詳細設計: 入出力データの構造 |
| DS-167 | Evidenceには現在のTest subject全体の内容ハッシュ、および全宣言targetを解決したcanonica | 詳細仕様(detailed_spec) to 詳細設計(design) | 詳細設計: 入出力データの構造 |
| DS-168 | Evidenceには実行時HEAD revision、実行adapter・runner・toolchain・実行影響co | 詳細仕様(detailed_spec) to 詳細設計(design) | 詳細設計: 入出力データの構造 |
| DS-169 | Evidenceには実行日時と実行方式を含める。 | 詳細仕様(detailed_spec) to 詳細設計(design) | 詳細設計: 入出力データの構造 |
| DS-170 | Evidenceには `target_binding`（§5.3）のtarget別結果とfail-closed集約結果（ | 詳細仕様(detailed_spec) to 詳細設計(design) | 詳細設計: 入出力データの構造 |
| DS-213 | record / エンティティファイルの書込みは原子的に公開する。 | 詳細仕様(detailed_spec) to 詳細設計(design) | 詳細設計: 永続化ルール |
| DS-216 | `cache/` が破損・削除されても正典は影響を受けない。 | 詳細仕様(detailed_spec) to 基本設計(basic_design) | 基本設計: 障害境界 |
| DS-218 | MCPがCLIと異なるadapterを暗黙選択してはならない。 | 詳細仕様(detailed_spec) to 詳細設計(design) | 詳細設計: 構成要素ごとの責務 |
| BD-011 | `config.yaml` writerの正規形はversion 2とする。 | 基本設計(basic_design) to 詳細設計(design) | 詳細設計: 永続化ルール |
| BD-012 | `config.yaml` writerはadapterごとにroot・scan・run設定をnamespace化する。 | 基本設計(basic_design) to 詳細設計(design) | 詳細設計: 入出力データの構造 |
| BD-013 | readerはversion 1を単一の `rust-cargo` adapter設定としてin-memory変換して読 | 基本設計(basic_design) to 詳細設計(design) | 詳細設計: 入出力データの構造 |
| BD-014 | `vtest init` はversion 2を生成する。 | 基本設計(basic_design) to 詳細設計(design) | 詳細設計: 永続化ルール |
| BD-015 | Test JSONのwire compatibility layerは `execution` を常に出力する。 | 基本設計(basic_design) to 詳細設計(design) | 詳細設計: 入出力データの構造 |
| BD-016 | Test JSONのwire compatibility layerは `rust-cargo` Testについてだけv | 基本設計(basic_design) to 詳細設計(design) | 詳細設計: 入出力データの構造 |
| BD-017 | 非Rust Testでは version 1互換fieldを省略する。 | 基本設計(basic_design) to 詳細設計(design) | 詳細設計: 入出力データの構造 |
| BD-018 | `targets` listを常に出力する。 | 基本設計(basic_design) to 詳細設計(design) | 詳細設計: 入出力データの構造 |
| BD-019 | 単数互換field `target` はtarget 1件のときだけ追加出力する。 | 基本設計(basic_design) to 詳細設計(design) | 詳細設計: 入出力データの構造 |
| BD-028 | DOC / VO / TESTのIDは人間可読な形式とする。 | 基本設計(basic_design) to 詳細仕様(detailed_spec) | 詳細仕様: 入力値の範囲・形式・制約 |
| BD-029 | DOC / VO / TESTのIDは利用者（人間またはAI）が命名する。 | 基本設計(basic_design) to 基本仕様(spec) | 基本仕様: 外部から与えられる入力 |
| BD-030 | IDの文字集合は `[A-Z0-9-]` とする。 | 基本設計(basic_design) to 詳細仕様(detailed_spec) | 詳細仕様: 入力値の範囲・形式・制約 |
| BD-031 | IDの接頭辞は種別ごとに固定する（`TEST-` 等）。 | 基本設計(basic_design) to 詳細仕様(detailed_spec) | 詳細仕様: 入力値の範囲・形式・制約 |
| BD-032 | IDの推奨形式は `TEST-<領域>-<連番>`（例：`TEST-PARSER-044`）とする。 | 基本設計(basic_design) to 詳細仕様(detailed_spec) | 詳細仕様: 入力値の範囲・形式・制約 |
| BD-033 | Relation writerは `REL-<ULID>` を正規IDとしてファイル名に用いる。 | 基本設計(basic_design) to 詳細設計(design) | 詳細設計: 永続化ルール |
| BD-034 | readerはversion 1互換入力としてbare ULIDを `REL-<ULID>` へin-memory正規化 | 基本設計(basic_design) to 詳細設計(design) | 詳細設計: 入出力データの構造 |
| BD-035 | 判断・承認・EvidenceのIDはbare ULIDとする。 | 基本設計(basic_design) to 詳細設計(design) | 詳細設計: 入出力データの構造 |
| BD-044 | `rust-cargo` adapterにおける判定権威は `cargo test` である。 | 基本設計(basic_design) to 詳細設計(design) | 詳細設計: 構成要素ごとの責務 |
| BD-049 | `rust-cargo` adapterが組込schemaを登録する。 | 基本設計(basic_design) to 詳細設計(design) | 詳細設計: 構成要素ごとの責務 |
| BD-050 | Form Schemaの `kind` はrepository内で大局的に一意なForm IDとする。 | 基本設計(basic_design) to 詳細仕様(detailed_spec) | 詳細仕様: 仕様上の不変条件 |
| BD-051 | schemaはそれを処理するadapter IDを別fieldで宣言する。 | 基本設計(basic_design) to 詳細設計(design) | 詳細設計: 入出力データの構造 |
| BD-052 | `vtest init` は `.verify/` を作成する。 | 基本設計(basic_design) to 基本仕様(spec) | 基本仕様: 主要な振る舞い |
| BD-053 | `vtest init` は既存コードを変更しない。 | 基本設計(basic_design) to 詳細仕様(detailed_spec) | 詳細仕様: 仕様上の不変条件 |
| BD-054 | `vtest scan` は発見した未登録Testを未登録として報告する。 | 基本設計(basic_design) to 基本仕様(spec) | 基本仕様: 主要な振る舞い |
| BD-055 | `vtest verify` は正典または検証事実の欠落を対応する非 `PASS` 値として表示する。 | 基本設計(basic_design) to 詳細仕様(detailed_spec) | 詳細仕様: 正常系・異常系・境界値・状態ごとの振る舞い |
| BD-056 | `vtest verify` は部分的な登録・判断・実行状態を総合 `OK` として扱わない。 | 基本設計(basic_design) to 詳細仕様(detailed_spec) | 詳細仕様: 仕様上の不変条件 |
| BD-059 | Evidence readerはadapter IDを欠く互換recordも履歴として読み取れる。 | 基本設計(basic_design) to 詳細設計(design) | 詳細設計: 他コンポーネントとの契約 |
| BD-060 | Evidence readerは、現在のTestが `rust-cargo` で互換runner情報と内容ハッシュからR | 基本設計(basic_design) to 詳細設計(design) | 詳細設計: 事前条件・事後条件・不変条件 |
| BD-061 | Evidence readerは、Rust実行と一意に確認できない場合は `UNKNOWN` とする。 | 基本設計(basic_design) to 詳細設計(design) | 詳細設計: 事前条件・事後条件・不変条件 |
| BD-057 | `vtest run` はテストを実際に実行する。 | 基本設計(basic_design) to 基本仕様(spec) | 基本仕様: 提供する機能 |
| BD-058 | `vtest run` は判定権威（§7）であるランナーの結果をEvidenceとして記録する。 | 基本設計(basic_design) to 基本仕様(spec) | 基本仕様: 主要な振る舞い |
| BD-104 | 終了コードは `0`＝要求scopeがOK、`1`＝検証NG、`2`＝入力・adapter前提・capability等に | 基本設計(basic_design) to 詳細仕様(detailed_spec) | 詳細仕様: すべての出力結果 |
| BD-105 | フェーズゲートを指定した実行（§20）では、`0` / `1` は当該ゲートの充足・不充足を表す。 | 基本設計(basic_design) to 詳細仕様(detailed_spec) | 詳細仕様: 正常系・異常系・境界値・状態ごとの振る舞い |
| BD-106 | ゲート指定時の `0` を検証状態 `PASS` と読ませない。 | 基本設計(basic_design) to 詳細仕様(detailed_spec) | 詳細仕様: 仕様上の不変条件 |
| BD-107 | 検証状態と内部エラーは終了コードで分離する（§4.4）。 | 基本設計(basic_design) to 詳細仕様(detailed_spec) | 詳細仕様: 仕様上の不変条件 |
| BD-108 | CIはこの終了コードのみで判定できる。 | 基本設計(basic_design) to 詳細仕様(detailed_spec) | 詳細仕様: 受入条件・検証可能な期待結果 |

## 3. NONE の一覧（全64件、現在層のまま）

| id | 現在層 | 文（60字まで） |
|---|---|---|
| SPEC-001 | 基本仕様(spec) | 本書は「AI並列開発向けテスト検証システム 要求・要件定義 v0.1」（以下、要件定義。FROZEN/v0.1 base |
| SPEC-002 | 基本仕様(spec) | 要件定義はWHY（何を保証しなければならないか）を定める。 |
| SPEC-003 | 基本仕様(spec) | 本書はWHAT（システムが外部に対して保証する挙動・データモデル・状態モデル・インターフェースの範囲）を確定する。 |
| SPEC-004 | 基本仕様(spec) | 具体構文・アルゴリズム・スキーマの全フィールド・コマンド引数などのHOWは「詳細設計 v0.1」で定める。 |
| SPEC-005 | 基本仕様(spec) | 本書は具体構文・アルゴリズム・スキーマの全フィールド・コマンド引数などのHOWを発明しない。 |
| SPEC-006 | 基本仕様(spec) | 要件定義に無い義務・検査・状態・文書種別を本書で新設しない。 |
| SPEC-007 | 基本仕様(spec) | 矛盾・不足を発見した場合、本書を書き換えない。 |
| SPEC-008 | 基本仕様(spec) | 矛盾・不足を発見した場合、上流へフィードバックしOwner判断を経る。 |
| SPEC-009 | 基本仕様(spec) | 本書からの `要件定義 §n` 参照は、FROZEN 要件定義の連番（§1〜§28、および原則 P-001〜P-005、 |
| SPEC-054 | 基本仕様(spec) | 診断ラベルの語彙は詳細設計で定める。 |
| SPEC-072 | 基本仕様(spec) | 要件定義 §3.2 の宣言鎖をそのまま採用する。 |
| SPEC-112 | 基本仕様(spec) | 関係リンクの任意説明文・役割別projectionの保存形式・presetは詳細設計へ委譲する。 |
| SPEC-129 | 基本仕様(spec) | 診断ラベルの語彙は詳細設計で定める。 |
| SPEC-131 | 基本仕様(spec) | 要件定義 §5.3 の割当をそのまま採用する。 |
| SPEC-151 | 基本仕様(spec) | 根の指定の具体構文は詳細設計へ委譲する。 |
| SPEC-156 | 基本仕様(spec) | 他の実行形態における確認方法は、当該形態に適した方法として詳細設計で定める。 |
| SPEC-181 | 基本仕様(spec) | `oracle_presence` の信頼基盤の具体的範囲（標準assert構文・framework failure s |
| SPEC-183 | 基本仕様(spec) | code fragmentはRustによる例示である。 |
| SPEC-193 | 基本仕様(spec) | 具体的構文・namespace・symbol種別は詳細設計へ委譲する。 |
| SPEC-214 | 基本仕様(spec) | partition・組合せcoverage方針の具体的保存形式・語彙は詳細設計へ委譲する。 |
| SPEC-235 | 基本仕様(spec) | エスカレーション出力・判断記録の具体的schema、判断待ち情報（§18.3）の構造schemaと取得インターフェース、 |
| SPEC-269 | 基本仕様(spec) | 文書の具体的入力フォーマットと登録方式、根の指定方式は詳細設計へ委譲する。 |
| SPEC-284 | 基本仕様(spec) | 具体的な承認ロール・必要承認数・権限schema・承認workflowの状態遷移は詳細設計へ委譲する。 |
| SPEC-297 | 基本仕様(spec) | 表示形式（表・GUI等）は要件でなく詳細設計へ委譲する。 |
| SPEC-310 | 基本仕様(spec) | 役割を固定enumやモード名として仕様化することは本書では行わない。 |
| SPEC-311 | 基本仕様(spec) | preset・UI・モード体系は詳細設計へ委譲する。 |
| SPEC-319 | 基本仕様(spec) | 具体的なフェーズ名・承認ロール・必要承認数・権限schema・進行条件定義は詳細設計へ委譲する。 |
| SPEC-342 | 基本仕様(spec) | 具体的な物理保存方式は詳細設計へ委譲する。 |
| SPEC-344 | 基本仕様(spec) | キャッシュ / indexの具体的データ形式は詳細設計へ委譲する。 |
| SPEC-356 | 基本仕様(spec) | コマンドの完全仕様（引数・出力・終了コード）は詳細設計で定める。 |
| SPEC-357 | 基本仕様(spec) | 本書ではコマンド一覧と責務を確定する。 |
| SPEC-360 | 基本仕様(spec) | 終了コード体系の詳細は詳細設計へ委譲する。 |
| SPEC-362 | 基本仕様(spec) | ツールの完全な入出力スキーマは詳細設計で定める。 |
| SPEC-365 | 基本仕様(spec) | CLI command体系・MCP tool体系の詳細は詳細設計へ委譲する。 |
| SPEC-379 | 基本仕様(spec) | 要件定義 §25 のスコープ外事項に対応する機能を本書では定義しない。 |
| SPEC-389 | 基本仕様(spec) | 以下は本書の要求・要件を基に詳細設計で決定する（要件定義 §28 の23項目に対応）。 |
| SPEC-390 | 基本仕様(spec) | HOWは本書で発明しない。 |
| SPEC-391 | 基本仕様(spec) | 詳細設計は、文書の具体的な入力フォーマットと登録方式を決定する（§16）。 |
| SPEC-392 | 基本仕様(spec) | 詳細設計は、文書層の根の指定方式（orphan_detectionの除外指定。§5.2）を決定する。 |
| SPEC-393 | 基本仕様(spec) | 詳細設計は、VO保存形式を決定する（§10、§24.1）。 |
| SPEC-394 | 基本仕様(spec) | 詳細設計は、Test metadataの具体的annotation syntax（`rust-cargo` の `@vt |
| SPEC-395 | 基本仕様(spec) | 詳細設計は、relationの保存形式を決定する（§3.2、§24.2）。 |
| SPEC-396 | 基本仕様(spec) | 詳細設計は、Test ID命名規則を決定する（§3.2）。 |
| SPEC-397 | 基本仕様(spec) | 詳細設計は、Target Reference / SRC IDの具体的識別方式を決定する（§9.2）。 |
| SPEC-398 | 基本仕様(spec) | 詳細設計は、AST / LSP等の具体的解析技術（不成立証明・存在確認・静的到達の実装。§5.5、§8.3）を決定する。 |
| SPEC-399 | 基本仕様(spec) | 詳細設計は、`oracle_presence` の信頼基盤の具体的範囲と委譲確認の方法を決定する（§8.2）。 |
| SPEC-400 | 基本仕様(spec) | 詳細設計は、`target_binding` の動的計測方式を決定する（§5.3）。 |
| SPEC-401 | 基本仕様(spec) | 詳細設計は、診断ラベルの語彙を決定する（§4.2）。 |
| SPEC-402 | 基本仕様(spec) | 詳細設計は、終了コード体系（検証状態と内部エラーの分離。§26.1）を決定する。 |
| SPEC-403 | 基本仕様(spec) | 詳細設計は、エスカレーション出力・判断記録・承認記録の具体的schemaを決定する（§11、§17）。 |
| SPEC-404 | 基本仕様(spec) | 詳細設計は、CLI command体系を決定する（§26.1）。 |
| SPEC-405 | 基本仕様(spec) | 詳細設計は、MCP tool体系を決定する（§26.2）。 |
| SPEC-406 | 基本仕様(spec) | 詳細設計は、キャッシュ / indexの具体的データ形式を決定する（§24.3）。 |
| SPEC-407 | 基本仕様(spec) | 詳細設計は、並列編集時の物理的保存方式を決定する（§24.2）。 |
| SPEC-408 | 基本仕様(spec) | 詳細設計は、承認workflowの具体的状態遷移を決定する（§17）。 |
| SPEC-409 | 基本仕様(spec) | 詳細設計は、判断待ち情報（§18.3）の具体的な構造schemaと取得インターフェースを決定する。 |
| SPEC-410 | 基本仕様(spec) | 詳細設計は、関係リンクの任意説明（§19）の保存形式を決定する。 |
| SPEC-411 | 基本仕様(spec) | 詳細設計は、役割別projection / view（§19）のpreset・UI・モード体系を決定する。 |
| SPEC-412 | 基本仕様(spec) | 詳細設計は、approval authority（§17）の承認ロール・必要承認数・権限schemaを決定する。 |
| SPEC-413 | 基本仕様(spec) | 詳細設計は、フェーズ・ゲート（§20）の具体的なフェーズ名と進行条件定義を決定する。 |
| SPEC-414 | 基本仕様(spec) | 本書の要求・要件を基に詳細設計で決定するHOWを本書で確定しない。 |
| DS-125 | 詳細仕様(detailed_spec) | 発見されたTest集合を `D`、構造上完全なmanaged Test Entity集合を `M` とする。 |
| DS-131 | 詳細仕様(detailed_spec) | 違反時の状態は §4.3 に従う。 |
| BD-128 | 基本設計(basic_design) | READMEに非関知宣言を一行入れる。 |
