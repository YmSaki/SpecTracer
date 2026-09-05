# 詳細設計 本冊 §0〜§5 層仕分け結果（DES-001〜DES-700）

対象: `docs/canonical/specification.json` の `design` 配列のうち、`source.doc` が詳細設計 本冊（別紙A/別紙Cではない）かつ `source.lines[0] <= 948` の全statement。DES-001〜DES-700、§0〜§5、本冊見出し26節。
判定基準: `docs/canonical/LAYERING.md` §2（判定1→1a→2→3）を、節ごとに既定の層を判断（多数決ではなく内容を読んでの判断。ただし本冊のように既に「design」IDで格納されている配列でも、六層モデルの問いに照らして spec/detailed_spec/basic_design/design へ再仕分けした）したうえで文単位に適用した。作業は5並列サブエージェントに範囲分割（DES-001〜159 / 160〜276 / 277〜455 / 456〜632 / 633〜700）して実施し、本書はその結果を統合・機械検証したもの。

## (a) 節ごとの既定の層

| 節 | 既定の層 | 文数 | 例外数 | 理由 |
|---|---|---:|---:|---|
| 0. 本書の位置付け | spec | 9 | 0 | 本書と上位仕様の関係・自己の性格づけを述べる約束で、実装の作り方によらず真であり続ける宣言的記述 |
| 分冊構成 | spec | 12 | 0 | 分冊・節番号・CLI/MCP一覧非拡大という文書運用上の約束で、システム構造の決定ではない |
| 1.1 ワークスペース構成 | basic_design | 10 | 0 | crate分割・依存方向・adapter/core分界という構造決定 |
| 1.2 主要依存クレート | basic_design | 14 | 3 | 採用ライブラリ・ツールの技術選定 |
| 1.3 内容ハッシュの定義 | design | 56 | 8 | 各hash domainの束縛field・符号化規則という型契約 |
| 2.1 `.verify/` レイアウト | basic_design | 18 | 1 | ディレクトリ・ファイル形式というpersistence構成決定 |
| 2.2 `config.yaml` | detailed_spec | 36 | 9 | fieldごとの必須/任意・許容値・拒否条件・エラーコードという精密な受入条件 |
| 2.3 派生情報 | basic_design | 4 | 1 | 派生情報の再構築・cache方針という永続化アーキテクチャ決定 |
| 3. レコードファイルスキーマ | detailed_spec | 3 | 1 | 未知フィールド扱い・id/ファイル名一致など精密な入力制約が主体 |
| 3.1 document レコード（`.verify/doc/DOC-*.yaml`） | detailed_spec | 25 | 12 | derives_from/anchor/noteの任意性・空値非MISMATCH等、精密な境界規定が過半 |
| 3.2 VO レコード（`.verify/vo/VO-*.yaml`） | detailed_spec | 23 | 8 | anchor/note/dimensions等の任意性・カーディナリティ制約が主体 |
| 3.2.1 dimensions と組合せの実体化 | detailed_spec | 24 | 6 | E-SCAN-017条件からMISMATCH等、条件から状態への精密な割当が大多数 |
| 3.3 Relation レコード（`.verify/rel/REL-<ULID>.yaml`） | design | 12 | 7 | writer/readerの互換契約・不変性宣言・型宣言が主軸 |
| 3.4 判断記録レコード（`.verify/decisions/<ULID>.yaml`） | detailed_spec | 30 | 12 | 必須/任意フィールド・条件から診断コード割当が主体 |
| 3.5 承認レコード（`.verify/approvals/<ULID>.yaml`） | design | 75 | 14 | フィールド型・値域・エラーコード・実効承認導出・状態遷移の規定が大半 |
| 3.6 Evidence レコード（`.verify/evidence/<ULID>.yaml`） | design | 43 | 7 | フィールド型・互換field正規化条件・manifest範囲・エラーコードなど記録schemaと機構規定が中心 |
| 4.1 adapter-neutralな正規化 | basic_design | 15 | 9 | 節全体の主眼はadapter/coreの構造的分界（責務・データフロー・裁量委譲）の記述 |
| 4.2 `rust-cargo` annotation文法 | design | 27 | 1 | rust-cargo adapter専有の文法・語彙・エラーコード対応という「このadapterの契約」の定義 |
| 4.3 `rust-cargo` locator構文 | design | 6 | 0 | locator文法・型マッピングのadapter専有契約 |
| 4.4 宣言エラーの扱い | design | 13 | 4 | 必須metadata欠如時のエラーコード・状態導出というper-component契約が中心 |
| 5.1 処理フロー | design | 11 | 1 | 各段が何を検証・生成するかという段の入出力契約(データ流れ)が主体 |
| 5.2 エンティティモデル（vtest-model） | design | 166 | 15 | 圧倒的多数が型・variant宣言と不変条件 |
| 5.3 検証グラフ | design | 12 | 3 | ノード/エッジ種別を列挙する内部型宣言の集まり |
| 5.4 整合性診断 | detailed_spec | 34 | 1 | E/Wコードごとのseverityと外部的意味、検証状態への写像を精密に定義する診断体系 |
| 5.5 `rust-cargo` SourceDiscoveryAdapter | design | 14 | 5 | discovery各段の入出力契約・DiscoveryBatchのDTOフィールド宣言が中心 |
| 5.6 文書層 orphan_detection | spec | 8 | 5 | 定義文・スコープ文が既存spec文言の反復で最も先例と一致度が高い(basic_design/detailed_specとほぼ三分) |

## (b) 集計

| 層 | 件数 | 割合 |
|---|---:|---:|
| spec | 42 | 6.0% |
| detailed_spec | 167 | 23.9% |
| basic_design | 94 | 13.4% |
| design | 397 | 56.7% |
| **合計** | **700** | 100.0% |

`confidence: low` は **38件**（DES-003, DES-046, DES-062, DES-067, DES-090, DES-096, DES-102, DES-121, DES-141, DES-152, DES-204, DES-215, DES-217, DES-240, DES-266, DES-272, DES-290, DES-307, DES-333, DES-346, DES-350, DES-380, DES-401, DES-403, DES-405, DES-433, DES-445, DES-463, DES-510, DES-517, DES-518, DES-620, DES-626, DES-628, DES-643, DES-684, DES-688, DES-694）。全体700文に対し規則§2で定めた上限目安(~40)を下回るため、規則の再定義は不要と判断した。

`code_like: true` は **7件**（DES-043, DES-159, DES-440, DES-457, DES-461, DES-685, DES-686）。詳細は(d)。

## (c) 例外一覧（節の既定層と異なる文）

既定層と異なる層に置いた文は133件。ID順。

| ID | 文（先頭60字） | 層 | 理由 | 確信度 |
|---|---|---|---|---|
| DES-043 | Git 操作（HEAD の取得、dirty 判定）は `git` CLI の呼び出しで行う（`git rev-parse… | design | `git rev-parse HEAD`・`git status --porcelain`という具体的コマンド文字列を規範として固定しており、別の取得手段（ライブラリ呼び出し等）を… | high |
| DES-044 | `git` が利用できない場合、リビジョンは特定できず、当該 Evidence はハッシュ束縛（revision 一致）… | detailed_spec | git不使用時にEvidenceがtarget_bindingの有効なPASS証拠にならないという精密な条件別結果の規定（fail-closed）。 | high |
| DES-045 | `git` が利用できない場合の失効は独立検査ではなく診断ラベル `STALE` として説明する（§11.2）。 | detailed_spec | git不使用時の失効を独立検査でなく診断ラベルSTALEとして扱うという精密な割当規定。 | high |
| DES-046 | 内容ハッシュはSHA-256を使用する。 | basic_design | 内容ハッシュのアルゴリズムをSHA-256とする技術選定。§1.2のsha2採用と同種の決定であり、型宣言（design）との境界が曖昧。 | low |
| DES-062 | canonical metadataの `targets` が宣言された `TargetRef` の正規化値を束縛し、解… | detailed_spec | targetの参照方法変更がTest subject hashで捕捉されるという、外部から観測可能な精密な帰結の説明。 | low |
| DES-067 | 検証対象は一般概念であり、このhashは検証対象をSource Targetとして実現した形態のidentity束縛であ… | spec | 検証対象は一般概念でありhashはSource Target実現形態への束縛に過ぎないという、基本仕様§9.1の概念範囲を確認するだけの記述。 | low |
| DES-078 | document recordの `content_hash` と実sourceが不一致ならsubject hashは現… | detailed_spec | content_hashと実sourceの不一致時にchain_integrityの非PASS（MISMATCH・STALE）とするという精密な条件別状態割当。 | high |
| DES-091 | adapterは、最終的な `TestEntity.content_hash` または `SourceTarget.co… | basic_design | adapterが最終content_hashを自己確定してはならないという、adapter/core責務分界の明示。 | high |
| DES-092 | coreはASTや言語固有構文からrangeを再計算しない。 | basic_design | coreがASTからrangeを再計算しないという、adapter固有情報を解釈しないadapter/core責務分界の明示。 | high |
| DES-096 | 静的解析は正典レコードを持たず、検証のたびに現在のsource / configから再計算する派生情報である（§7・§7… | basic_design | 静的解析が正典レコードを持たず毎回再計算する派生情報であるという永続化方式の決定（§2.1のcache配置決定と同種）。 | low |
| DES-101 | format変更を構文上の意味だけから同値とみなさず、正規化後のsource bytesが変化した場合は安全側でSTAL… | detailed_spec | 正規化後source bytesが変化した場合に安全側でSTALEとするという精密な条件別診断割当。 | high |
| DES-102 | 基本仕様 §24.1 の layout をそのまま採用する。 | spec | 基本仕様§24.1のlayoutをそのまま採用するという追認で、新たな構造決定を行わない委譲文。 | low |
| DES-120 | `config.yaml` writerの正規形はversion 2とし、adapterごとにroot・scan・run… | basic_design | config.yamlをversion 2・adapterごとnamespace化するという外部設定ファイル構成の決定。 | high |
| DES-121 | `config.yaml` readerはversion 1を単一の `rust-cargo` adapter設定として… | basic_design | version 1のin-memory変換読み込みという互換readerの動作決定。 | low |
| DES-127 | 統合したTest IDは全adapterでglobal uniquenessを検査する。 | spec | 統合Test IDのglobal uniquenessを検査するという主要な機能の約束。 | high |
| DES-128 | adapter固有設定の検証は登録adapterへ委譲する。 | basic_design | adapter固有設定検証を登録adapterへ委譲するという責務分界の明示。 | high |
| DES-130 | `vtest init` はversion 2を生成する。 | basic_design | `vtest init`がversion 2を生成するという名指しコマンドの責務定義。 | high |
| DES-137 | `gates` はフェーズゲートの進行条件定義を保持する（§11.5）。 | design | `gates`が保持するデータ（フェーズゲート進行条件定義）という型・field宣言。 | high |
| DES-152 | `doc.roots` は orphan_detection の除外根をDOC IDの集合として保持する（§5.6）。 | design | `doc.roots`が保持するデータ（除外根のDOC ID集合）という型・field宣言。 | low |
| DES-153 | `scan` と `run` はversion 1 schema互換のwire値とする。 | basic_design | `scan`/`run`をversion 1互換のwire値とするという外部フォーマット互換の決定。 | high |
| DES-154 | Rust固有のmacro pathや `llvm-cov` 制約は `rust-cargo` adapterに限って適用… | basic_design | Rust固有制約をrust-cargo adapterに限定適用するという構成上の境界決定。 | high |
| DES-159 | MCPサーバは長時間動作するため、ツール呼び出しごとに対象ファイルのmtimeを確認し、変化があれば再スキャンする。 | design | 「ツール呼び出しごとにmtimeを確認し、変化があれば再スキャンする」という具体的な検知アルゴリズム・手順を固定しており、別の鮮度検知方式を採ると文が偽になる。 | high |
| DES-160 | すべてのレコードはYAMLとする。 | basic_design | レコード保存形式をYAMLとする技術選定（構造決定）。 | high |
| DES-163 | 上流文書はすべて単一の総称ノード型 `document` で表現する。 | basic_design | 上流文書を単一総称ノード型で表現するという、文書種別ごとのスキーマを設けないデータモデル構造の決定。 | high |
| DES-164 | 要件定義・基本仕様・詳細設計・API Schema・Protocol Specification・型/データ仕様・DB … | basic_design | 文書種別ごとの専用スキーマを持たないというデータモデル構造の決定（163と同一クラスタ）。 | high |
| DES-165 | document レコードの `path` fieldはプロジェクト相対パスである。 | design | pathフィールドの型・意味の宣言。 | high |
| DES-166 | document レコードの `content_hash` fieldは登録時の内容ハッシュである（§1.3）。 | design | content_hashフィールドの型・意味の宣言。 | high |
| DES-171 | `derives_from` は上流documentへの唯一のリンク種別である。 | basic_design | derives_fromを上流への唯一のリンク種別とする、関係型を増やさないモデル構造の決定。 | high |
| DES-172 | 文書層の段（要件→仕様→詳細設計…）は `derives_from` リンクで表現し、段を増やしても種別を増やさない。 | basic_design | 文書層の段が増えてもリンク種別を増やさないという拡張性に関するモデル構造の決定。 | high |
| DES-173 | リンクを追加してもスキーマは壊れない。 | basic_design | リンク追加でスキーマが壊れないという、スキーマ設計上の構造的性質の決定。 | high |
| DES-175 | `note` は付加・保存できる構造とする。 | spec | noteを付加・保存できるという構造上の能力の約束（実装によらず真）。 | high |
| DES-180 | `anchor` は `derives_from` entryのfieldであり、Test metadataには存在しな… | design | anchorフィールドがderives_from entryにのみ存在しTest metadataには無いという、フィールド所在の型宣言。 | high |
| DES-181 | `anchor` はcanonical document recordの一部であり、document subject h… | design | anchorがdocument subject hashの入力に含まれるという、ハッシュ計算契約の宣言。 | high |
| DES-186 | 仕様文書そのものは `.verify/` へ複製しない。 | basic_design | 仕様文書そのものを.verify/へ複製しないという永続化方式の決定。 | high |
| DES-187 | 本システムは文書内容の意味的良否を検証しない。 | spec | 文書内容の意味的良否を検証しないという、基本仕様OOS-001を再掲する範囲の約束。 | high |
| DES-188 | VO レコードの `parent` fieldはVO IDまたはnull（階層化）である。 | design | parentフィールドの型（VO IDまたはnull）の宣言。 | high |
| DES-193 | VO レコードの `coverage_policy` fieldの値域は `independent-axes` / `f… | design | coverage_policyフィールドの値域（型）の宣言。 | high |
| DES-194 | VO レコードの `combinations` fieldは `coverage_policy: explicit` の… | design | combinationsフィールドの意味（explicit時に実体化する組合せ）の宣言。 | high |
| DES-197 | VOとdocumentの間に他のエンティティ層を置かない（§3.2）。 | basic_design | VOとdocumentの間に他エンティティ層を置かないという、エンティティ階層構造の決定。 | high |
| DES-198 | VOは旧モデルの `requirements`（REQ参照）と `spec_refs`（SPEC + 節参照）は持たず、… | basic_design | 旧モデルのrequirements/spec_refsを廃しderives_fromへ一本化するという、参照スキーマ統合の構造決定。 | high |
| DES-204 | 「どの上流条項がどのVOへ対応するか」の対応ペアは、`anchor` 付き `derives_from` エッジとして保… | basic_design | 上流条項↔VO対応をanchor付きderives_fromエッジとして保持しprojection出力で露出するという、データの流れ・出力インターフェースに関する決定。 | low |
| DES-205 | `anchor` と `note` はVO subject hashの入力に含まれない（VO subject hashは… | design | anchor・noteがVO subject hashの入力に含まれないというハッシュ計算契約の宣言。 | high |
| DES-208 | VOの `status` は承認レコードから導出する表示値であり、canonical writerはVO recordへ… | design | VOのstatusが承認レコードからの導出表示値でありwriterが保存しないという、writerコンポーネントの契約宣言。 | high |
| DES-211 | `dimensions` を持つ VO は、`vtest vo expand VO-X` により子 VO を実体化できる… | basic_design | `vtest vo expand`という名指しコマンドが子VOを実体化するという、入口コマンドの責務定義。 | high |
| DES-216 | 子VO生成は、生成前に一覧を提示して `--dry-run` で確認できる。 | basic_design | `--dry-run`による事前提示確認という、CLIインターフェース機能の決定。 | high |
| DES-220 | 組合せ空間の定義が仕様に対して十分かは本システムの検査ではなく、`UNKNOWN` としてエスカレーションの領分である（… | spec | 組合せ空間の十分性は本システムの検査でなくUNKNOWNのエスカレーション領分であるという、基本仕様を再掲するスコープの約束。 | high |
| DES-221 | `combinations` は組合せtupleのlistである。 | design | combinationsフィールドの型（組合せtupleのlist）の宣言。 | high |
| DES-232 | `combinations` はcanonical VO recordの一部であり、VO subject hashに束縛… | design | combinationsがVO subject hashに束縛されるというハッシュ計算契約の宣言。 | high |
| DES-234 | `combinations` の値が仕様に対して十分な組合せ集合かは本システムの検査ではなく、エスカレーションの領分であ… | spec | combinations値の十分性は検査でなくエスカレーション領分であるという、基本仕様を再掲するスコープの約束（220と同一クラスタ）。 | high |
| DES-235 | Relationは、どちらか一方のエンティティに自然に所属しない関係（VO間の依存、Test間の補完関係など）だけを保存… | basic_design | Relationがどちらにも自然に所属しない関係だけを保存するという、専用エンティティを設ける範囲のモデル構造決定。 | high |
| DES-236 | `derives_from`・`covers`・`targets` はadapter所有の宣言またはdocument /… | basic_design | adapter宣言・document/VO recordから導出できる関係を外部Relationとして重複保存しないという、データ導出方式の構造決定。 | high |
| DES-238 | Relationレコードの `from` fieldは任意のエンティティIDである。 | detailed_spec | fromフィールドの任意性という入力制約。 | high |
| DES-239 | Relationレコードの `note` fieldは任意の説明文である。 | detailed_spec | noteフィールドの任意性という入力制約。 | high |
| DES-240 | canonical Relation IDは `REL-` と26文字のULID payloadからなる。 | detailed_spec | canonical Relation IDの精密な書式（REL-+26文字ULID）の規定。 | low |
| DES-243 | prefixed / bareの混在、ファイル名と `id` のpayload不一致、または同じpayloadの複数re… | detailed_spec | prefixed/bare混在・payload不一致・重複recordをE-SCAN-010とし選択しないという精密な拒否規定。 | high |
| DES-246 | `from` / `to` の存在はスキャン時に検査し、不在はE-SCAN-009、`chain_integrity` … | detailed_spec | from/to不在をE-SCAN-009・MISMATCHとするという精密な条件→状態の割当。 | high |
| DES-247 | 判断記録は、`UNKNOWN` に対して外部（人間または判断可能Agent）が下した判断の記録である。 | spec | 判断記録がUNKNOWNに対する外部判断の記録であるという、基本仕様・要件定義を再掲する概念的定義の約束。 | high |
| DES-250 | 判断記録は依存closureのハッシュに束縛される。 | design | 判断記録が依存closureのハッシュに束縛されるという、エンティティの不変条件宣言。 | high |
| DES-251 | 判断記録の `subject` fieldは判断対象のエンティティIDまたは解決済みcanonical Locatorで… | design | subjectフィールドの型（エンティティIDまたはLocator）の宣言。 | high |
| DES-254 | 判断記録の `subject_hash` fieldは判断時点の対象の内容ハッシュである。 | design | subject_hashフィールドの型・意味の宣言。 | high |
| DES-256 | 判断記録の `dependencies` entryの `hash` fieldはdocument subject ha… | design | dependencies entryのhashフィールドの型・意味の宣言。 | high |
| DES-258 | 判断記録の `actor` の `kind` fieldの値域は `human` / `agent` である。 | design | actor.kindフィールドの値域（型）の宣言。 | high |
| DES-266 | 判断記録の有効性判定と実効判断の決定は §8.5 に従う。 | design | 有効性判定・実効判断決定が§8.5に従うという同一設計文書内の参照。 | low |
| DES-267 | `judgment_kind` は判断対象を一意に区切る第二のkeyである。 | design | judgment_kindが判断対象を区切る第二keyであるという、複合キー構造の宣言。 | high |
| DES-270 | `supersedes` は、この判断記録が明示に置き換える旧判断記録のULIDを名指しするlistである。 | design | supersedesフィールドの型（旧判断記録ULIDのlist）の宣言。 | high |
| DES-272 | `supersedes` はRelationとは独立であり、`type: supersedes` のRelationレコ… | design | supersedesがRelationとは独立でtype:supersedesのRelationを実効判断決定に用いないという、コンポーネント間の独立性の宣言。 | low |
| DES-275 | 判断記録は検査ゲートではなく、`UNKNOWN` に対する外部判断の追跡である。 | spec | 判断記録は検査ゲートでなくUNKNOWNへの外部判断追跡であるという、基本仕様相当の概念的定義の約束。 | high |
| DES-276 | 判断済みと承認済みは区別する（判断済み ≠ 承認済み）（§3.5）。 | spec | 判断済みと承認済みを区別するという、基本仕様SPEC-400相当の概念的約束。 | high |
| DES-287 | 承認は検証状態と独立の別軸である。 | spec | 承認と検証状態は別軸という主要な約束の短い言い換え（基本仕様§4.5・§17、要件定義§5.5を直接引用）。別実装でも真であり続ける。 | high |
| DES-288 | 承認済みを理由に非 `PASS` を `PASS` へ昇格させない。 | detailed_spec | 承認済みでも非PASSをPASSへ昇格させない、fieldに縛られない精密な外部的約束（基本仕様相当のSPEC-495と同文）。 | high |
| DES-289 | 未承認を理由に `PASS` を降格させない。 | detailed_spec | 未承認でもPASSを降格させない、精密な外部的約束（SPEC-496と同文）。 | high |
| DES-292 | 承認は特定のエンティティ型に従属しない独立の領域である。 | spec | 承認は特定entity型に従属しない独立領域という一般的な軸の約束（287と同型、fieldに紐付かない）。 | high |
| DES-293 | 承認レコードの構造・値域・実効承認の導出・状態遷移は本節だけで定義し、対象の種別ごとに別の承認規則を置かない。 | basic_design | 承認規則を本節だけに一本化し種別ごとの規則を設けないという、コンポーネント境界の決定。 | high |
| DES-294 | 承認の入力経路は対象種別で分けず、対象種別を引数に取る単一の正典面に一本化する（§13.2）。 | basic_design | 承認入力経路を対象種別で分けず単一正典面に一本化するという、入口構造の決定。 | high |
| DES-295 | エンティティ側に置く承認操作（`vtest vo approve` / `vo_approve`）は正典面への別名であり… | basic_design | エンティティ側の承認操作は正典面への別名に過ぎないという入口構造の決定（vtest vo approveのalias原則）。 | high |
| DES-310 | `approved_state` の値 `approved` は、この内容で進めることを認めたこと（承認）を意味する。 | detailed_spec | approved_state=approvedの外部的意味を定める、エラーの外部的意味に類する精密な定義。 | high |
| DES-311 | `approved_state` の値 `rejected` は、この内容で進めることを認めないこと（却下）を意味する。 | detailed_spec | approved_state=rejectedの外部的意味を定める精密な定義。 | high |
| DES-312 | `approved_state` の値 `withdrawn` は、先に与えた承認を取り消したこと（承認取消）を意味する… | detailed_spec | approved_state=withdrawnの外部的意味を定める精密な定義。 | high |
| DES-348 | 承認記録は「誰が（approver）」「何を（subjectまたはjudgment reference）」「どの承認状態… | detailed_spec | 承認記録の必須項目（誰が/何を/どの状態か）と根拠任意という精密な約束（基本仕様相当のSPEC-485/486と同文）。 | high |
| DES-349 | 承認主体は種別（`human` / `agent`）と識別子を記録する。 | detailed_spec | 承認主体の種別・識別子記録という精密な約束（SPEC-492と同文）。 | high |
| DES-350 | 誰がどの対象・範囲を承認できるか（approval authority）、承認ロール、必要承認数、権限schemaはプロ… | basic_design | 承認権限・ロール・必要承認数の定義を別紙A/プロジェクト設定へ委譲するという構成上の境界決定。委譲宣言はspec的とも読める。 | low |
| DES-351 | 承認レコードの入力経路は別紙A §12.2・§13.2 に定める。 | basic_design | 承認レコードの入力経路を別紙A§12.2・§13.2に委ねるという入口構造の参照。 | high |
| DES-360 | `result` はテストランナー（判定権威）が報告した合否をそのまま記録する（§7）。 | detailed_spec | resultはランナー報告をそのまま記録するという、field名に縛られない精密な外部的約束。 | high |
| DES-361 | 本システムは合否を再判定せず、`result` を `target_binding` の証拠として消費する。 | detailed_spec | 合否を再判定せずtarget_bindingの証拠として消費するという、基本仕様§7を直接引く精密な約束。 | high |
| DES-362 | 有効なEvidenceの `result: FAIL` は `target_binding = FAIL`（テストランナ… | detailed_spec | 有効EvidenceのFAILがtarget_binding=FAILへ至るという、条件→結果の精密な対応。 | high |
| DES-363 | `target_coverage` は `target_binding` の動的計測（宣言対象の実行が生じたか）の結果で… | spec | target_coverageは独立の検査項目ではないという、検査を増やさない主要な約束の短い言い換え。 | high |
| DES-380 | Evidence内の `target` は実行時snapshotを識別するkeyであり、TEST → SRC edgeの… | basic_design | Evidence内targetは実行時snapshotのkeyでTEST→SRC edgeの正典でないという、正典と派生データの分離原則。fieldの意味説明とも読める。 | low |
| DES-381 | graphはadapter所有のTest metadata宣言からだけ構築し、Evidenceのtarget listか… | basic_design | graphはadapter所有のTest metadata宣言からのみ構築しEvidenceからedgeを生成しないという、正典と派生データを分離するデータフロー構造の決定（基本仕… | high |
| DES-394 | 確認不能は `UNKNOWN`、明示adapterの不一致は `MISMATCH` とし、いずれも `PASS` へ昇格… | detailed_spec | 確認不能→UNKNOWN、明示不一致→MISMATCHでいずれもPASS非昇格という、field名に縛られない条件→結果の精密な対応。 | high |
| DES-395 | `SourceDiscoveryAdapter` は、adapter所有のsource declarationを `id… | design | SourceDiscoveryAdapterの正規化出力fieldを列挙する型・インターフェース宣言。 | high |
| DES-396 | 本versionでは、Testの存在理由分類（旧 `role` / `anchor` / `anchor_rationa… | design | 存在理由分類（旧role/anchor）を論理fieldに持たないという、schemaからの除去決定。 | high |
| DES-397 | すべての管理対象Testに `covers ≥ 1` を一律に要求する。 | detailed_spec | 全管理対象Testにcovers≥1を要求するという、基本仕様§12・要件定義§4.1を直接引く境界値付きの精密な約束。 | high |
| DES-398 | VOへの寄与は `covers` 宣言と証拠の十分性判定だけから導出する。 | detailed_spec | VO寄与はcovers宣言と証拠十分性判定だけから導出するという、field名に縛られない排他的な精密な約束。 | high |
| DES-399 | 検証対象は一般概念であり、adapter中立coreは各管理対象Testに1件以上の検証対象を要求する。 | detailed_spec | 各Testに1件以上の検証対象を要求するという、基本仕様§9.1・要件定義§9.1を直接引く境界値付きの約束。 | high |
| DES-400 | 検証対象は「そのTestが検証成立性を証明しようとする対象＝宣言された『何の時にどうなる』の主語」であって、実装cons… | detailed_spec | 検証対象は実装constructに限定しないという、語彙の意味を精密化する定義的記述。 | high |
| DES-402 | coreの `chain_integrity` は「検証対象をSource Targetとして実現し `targets … | detailed_spec | chain_integrityがtargets≥1をadapter中立の必須リンクとしないという、検査範囲を限定する精密な約束（検査を増やさない原則の具体化）。 | high |
| DES-404 | v0.1の唯一のadapter `rust-cargo` は検証対象をSource Targetとして実現し `targ… | detailed_spec | 唯一のadapter rust-cargoがtargets≥1を必須とするという、境界値付きの精密な外部的事実。 | high |
| DES-408 | locatorは `TargetRef::Locator { adapter, value }` とし、`value` … | design | locatorの型TargetRef::Locator{adapter, value}を定義する型宣言（LAYERING.mdのSourceLocation例と同型）。 | high |
| DES-433 | scannerは `@vtest.src-id` の指定値を認識するが、付与を必須としない。 | detailed_spec | scannerがsrc-id指定値を認識するが付与を必須としないという、基本仕様§9.2を引く精密な任意性の約束。field認識契約とも読める。 | low |
| DES-446 | `rust-cargo` は検証対象をSource Targetとして実現する形態であり、追加必須metadataとして… | detailed_spec | rust-cargoがtargets≥1を追加必須metadataとして要求するという境界値付きの精密な約束（404と同旨）。 | high |
| DES-449 | `covers` 件数の可変制約（旧role/anchor由来）は設けず、すべての管理対象Testに `covers ≥… | detailed_spec | covers件数の可変制約を設けず一律covers≥1を要求するという、基本仕様§12を引く精密な約束（397と同旨）。 | high |
| DES-453 | VO参照の解決とTest IDの大局的一意性はadapterではなくcoreが検査する。 | basic_design | VO参照解決とTest ID大局的一意性はcoreが検査するという、adapterとcoreの分界そのもの。 | high |
| DES-455 | VO解決・ID一意性・target解決はcoreが参照整合性検査で判定する（§5）。 | basic_design | VO解決・ID一意性・target解決はcoreが参照整合性検査で判定するという、adapterとcoreの分界。 | high |
| DES-466 | adapterが解析不能または不完全なbatchを返した場合、coreは対応する検証を `UNKNOWN` とし、Tes… | detailed_spec | 解析不能・不完全なbatch時に検証をUNKNOWNとし0件discoveryと扱わないという条件別の検証状態割当 | high |
| DES-493 | 検証状態は `Pass` / `Fail` / `Mismatch` / `NoEvidence` / `Unknown… | spec | 検証状態が5つのみという外部から見た約束を基本仕様/要件定義から引用して再掲。LAYERING.md PM仮置き「検証状態は5つのみ→spec」と同型 | high |
| DES-499 | 診断ラベルは検証状態と別軸である。 | spec | 診断ラベルと検証状態が別軸という外部から見た約束を基本仕様/要件定義から引用して再掲 | high |
| DES-504 | 検査は `ChainIntegrity` / `OrphanDetection` / `TargetBinding` /… | spec | 検査が4本のみという外部から見た約束を基本仕様/要件定義から引用して再掲。5状態と同型のPM仮置き対象 | high |
| DES-510 | 集約の代表値選択に診断ラベルを用いない（§11.3）。 | detailed_spec | 集約の代表値選択が診断ラベルに依存しないという検証可能な受入条件・不変条件 | low |
| DES-512 | `TargetRef::SrcId` はadapter IDを含まないため、`SrcId` は全adapterを統合した… | detailed_spec | SRC IDのrepository全体一意性という精密な制約。base-reportのSPEC-152/348と同内容 | high |
| DES-515 | coreは `project`、`suite.kind`、`suite.name`、`selector` の文字列を解釈… | basic_design | coreがadapter固有文字列を解釈しないというadapter/core分界の明示。base-reportのSPEC-283と同型 | high |
| DES-517 | `vtest-adapter-api` は言語非依存の `TestWireCodec` capabilityを定義する。 | basic_design | capabilityをどのcrateが定義するかというコンポーネント配置の決定 | low |
| DES-518 | codecはadapter固有のcompatibility propertyをJSON objectとしてencode … | basic_design | codecとcore型の間のadapter/core分界の明示 | low |
| DES-519 | `rust-cargo` codecはversion 1互換の `filter`、`package`、`test_tar… | basic_design | rust-cargo codecへの互換field所有権割当。base-reportのSPEC-168と同型のadapter/core分界 | high |
| DES-591 | VO参照の解決とTest IDの大局的一意性はadapterではなくcoreが検査する。 | basic_design | VO参照解決とTest ID大局的一意性をcoreが検査しadapterは検査しないというadapter/core分界の明示。base-reportのSPEC-283/284クラス… | high |
| DES-596 | adapter capabilityは `SourceDiscoveryAdapter`、`TestWireCodec`… | basic_design | adapter APIを6種のcapability traitへどう分割するかというシステム構造の決定 | high |
| DES-600 | 検証集約では、static解析 / coverage欠落は `NO_EVIDENCE`（診断 `NOT_CHECKED`… | detailed_spec | static解析/coverage欠落という特定条件から検証状態NO_EVIDENCEへの精密な割当。base-reportのSPEC-151型と同型 | high |
| DES-601 | 検証集約では、runner欠落は `NO_EVIDENCE`（診断 `NOT_EXECUTED`）とする。 | detailed_spec | runner欠落という特定条件から検証状態NO_EVIDENCEへの精密な割当 | high |
| DES-602 | 検証集約では、解析限界は `UNKNOWN` とする。 | detailed_spec | 解析限界という特定条件から検証状態UNKNOWNへの精密な割当 | high |
| DES-626 | Form `kind` はbuilt-inと `.verify/forms/` を統合したrepository全体で一意… | detailed_spec | Form kindのrepository全体一意性という識別子の精密な制約。SPEC-106型と同型だが直接引用がなく境界は曖昧 | low |
| DES-633 | インメモリのグラフを構築する。 | basic_design | 検証グラフをインメモリ構築し正典化しないという永続化方式の決定（派生データを独立正典にしない）。 | high |
| DES-643 | 上流文書はすべてDOCノードとし、文書間・VO→文書は `derives_from` の一種で表現する（§19）。 | basic_design | 上流文書をすべて単一DOCノード型で表現し文書型固有ノードを設けないという構造決定。約束(spec)か構造決定かの境界は曖昧。 | low |
| DES-644 | 関係型（`derives_from` / `covers` / `targets` / 外部Relation）は横断トレ… | spec | 関係型を単一へ潰さず意味論的増殖もさせないという約束（基本仕様側の同内容の言明と同種）。 | high |
| DES-670 | warningはレポートに常に表示する。 | spec | warningを常にレポート表示するという主要な振る舞いの約束（条件分岐を伴わない一般的機能記述）。 | high |
| DES-680 | `vtest-scan` はこれらのRust固有処理を実行しない。 | basic_design | vtest-scanがRust固有処理を実行しないというadapter/core責務分界の明示。 | high |
| DES-682 | 各管理対象Testに1件以上のSource Target（`targets ≥ 1`）を必須とすることはadapter層… | basic_design | targets≥1要求はadapter層所属でcore中立必須リンクでないという層帰属・構造分界の明示。 | high |
| DES-683 | 欠落はE-SCAN-007として報告する（§4.4・§5.4）。 | detailed_spec | targets欠落という条件からE-SCAN-007への精密な写像。 | high |
| DES-684 | したがって `rust-cargo` のTestは従来どおりSource Target宣言を要し、挙動・Eコード・fix… | basic_design | 本改訂でrust-cargoの挙動・Eコード・fixtureが実効的に不変という継続性の注記で、六層のいずれにも収まりにくい。 | low |
| DES-691 | `rust-cargo` discoveryの第6段（Source Target抽出）で非Test constructの… | detailed_spec | 非Test construct宣言の条件からW-SCAN-105/E-SCAN-005への精密な写像。 | high |
| DES-694 | `config.yaml` の `doc.roots` に列挙されたDOC IDを根として扱い、`orphan_dete… | basic_design | config.yamlのdoc.rootsという具体的な設定ファイル・fieldで根を指定する仕組みの決定。basic_designかdetailed_spec（除外条件そのもの）… | low |
| DES-695 | 根指定は `.verify/` 設定として保持する。 | basic_design | 根指定を.verify/設定として保持するという永続化方式の決定。 | high |
| DES-696 | 根指定の追加・削除は `vtest doc` コマンドの引数で管理し `doc.roots` へ反映する。 | basic_design | `vtest doc`コマンドが根指定の追加・削除を管理するという名指しコマンドの責務定義。 | high |
| DES-697 | `derives_from` が空、かつ他のどのdocumentからも `derives_from` で参照されないdo… | detailed_spec | 孤児と判定する条件（derives_from空・非参照・非root）を漏れなく列挙しE-SCAN-016へ写像する精密な検出規則。 | high |
| DES-700 | 根に指定されたdocumentが存在しないDOC IDを参照する場合は、config invariant違反としてE-C… | detailed_spec | 存在しない根DOC ID参照という条件からE-CONFIG-001への精密な写像。 | high |

## (d) code_like 一覧（全件）

この表はOwner向けの判断材料（詳細設計をどう扱うか）であるため完全収録する。

| ID | 文（先頭80字） | 理由 |
|---|---|---|
| DES-043 | Git 操作（HEAD の取得、dirty 判定）は `git` CLI の呼び出しで行う（`git rev-parse HEAD`、`git status -… | `git rev-parse HEAD`・`git status --porcelain`という具体的コマンド文字列を固定しており、別の取得手段（ライブラリ呼び出し等）を採ると文が偽になる |
| DES-159 | MCPサーバは長時間動作するため、ツール呼び出しごとに対象ファイルのmtimeを確認し、変化があれば再スキャンする。 | 「mtime確認→変化時再スキャン」という具体的な検知アルゴリズム・手順を固定しており、別の鮮度検知方式（イベント監視等）を採ると文が偽になる |
| DES-440 | `path` は `.rs` で終わる最初の `::` で item-path と分離する。 | pathとitem-pathの区切り位置を「.rsで終わる最初の::」という具体的な解析アルゴリズムで固定しており、別の分離規則（例:最後の::）を選べば嘘になる手順記述 |
| DES-457 | 処理フロー第2段は、discovery委譲であり、登録順ではなくadapter ID順にSourceDiscoveryAdapterを呼び出し、各adapter… | 「登録順ではなくadapter ID順」という呼び出し順序を明示的に固定。別実装（登録順呼び出し）なら文が偽になる |
| DES-461 | 処理フロー第5段は、決定論的な統合であり、adapter ID、project-relative path、opaque locator、Test IDの順に正… | 「adapter ID、path、locator、Test IDの順に正規化する」という多フィールドの正規化順序を明示的に固定 |
| DES-685 | `rust-cargo` discoveryの第1段はファイル探索であり、adapter configのinclude配下の `*.rs` をignoreクレー… | 段の入出力宣言自体はdesignだが、「`ignore`クレートで列挙する」という言語固有ライブラリの指定が、別実装（例：walkdir+自前.gitignore解析）なら文を偽にする |
| DES-686 | `rust-cargo` discoveryの第2段は構文解析であり、ファイルごとに `syn::parse_file` する。解析エラーのファイルはE-SCA… | 「`syn::parse_file`する」という具体API呼び出しの明記が、別実装（別parserクレート）なら文を偽にする |

## (e) design に残るものについての直接観察

六層への再仕分け後も `design` に残った文は397件（全700件の56.7%）で、うち `code_like: true` は7件。`design` の内訳は、§1.3内容ハッシュの定義（48/56）、§3.3 Relationレコード（5/12）、§3.5承認レコード（61/75）、§3.6 Evidenceレコード（36/43）、§4.2 rust-cargo annotation文法（26/27）、§4.3 rust-cargo locator構文（6/6）、§4.4宣言エラーの扱い（9/13）、§5.1処理フロー（10/11）、§5.2エンティティモデル（151/166）、§5.3検証グラフ（9/12）、§5.5 rust-cargo SourceDiscoveryAdapter（9/14）に集中しており、いずれも型・フィールド・値域・状態遷移・adapter専有構文の宣言が主体で、`spec`・`detailed_spec`・`basic_design`に判定された文は各節で少数の例外にとどまる。一方 `detailed_spec`（167件）は§2.2 config.yaml、§3.1〜3.4の各レコードスキーマ、§5.4整合性診断に集中し、`basic_design`（94件）は§1.1〜1.2・§2.1・§2.3・§4.1のadapter/core分界・永続化構成に集中する。`spec`（42件）は§0・分冊構成、および各節に散在する「基本仕様の概念・スコープをそのまま再掲するだけの文」で構成される。

