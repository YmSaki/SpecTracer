<!-- generated from docs/canonical/specification.json by build.py; do not edit -->

# 詳細設計

## DES-S001 1. 用語定義

*導出元: REQ-S001, REQ-S003, REQ-S013, REQ-S020, REQ-S025, REQ-S029, REQ-S035, REQ-S036, REQ-S037, REQ-S046*

### DES-001

documentとは、ソースコードより上流に位置する成果物を表す単一の総称ノードである。

### DES-002

documentは `id + path + content_hash + 上流参照（derives_from）` を持つ。

### DES-003

各derives_fromリンクは任意（optional）の説明文・導出理由を保持できる（§3.2）。

*導出元: REQ-036, REQ-037, REQ-038, REQ-039, REQ-040, REQ-041, REQ-042, REQ-043, REQ-044, REQ-045, REQ-046, REQ-047, REQ-048, REQ-049*

*引用: 要件定義 §3.4*

### DES-004

Source Targetは、adapter IDとadapter所有のopaque locatorからなるTarget Reference、または任意の恒久SRC IDで識別する。

*導出元: REQ-149, REQ-150, REQ-151, REQ-152, REQ-153, REQ-154, REQ-155, REQ-156*

*引用: 要件定義 §9.2*

### DES-005

Execution Evidenceは結果・実行時リポジトリ状態・解決後のcanonical Source Target参照・各内容ハッシュ・実行計測結果を含む。

### DES-006

判断記録はactor / subject / decisionを必須項目とする。

### DES-007

判断記録の理由・根拠は任意とする。

### DES-008

承認記録はapprover / subject（またはjudgment reference）/ approved stateを必須とする。

## DES-S002 2 全体像

### DES-S003 2.1 正典の三層構造

*導出元: P-003, REQ-S019, REQ-S035, REQ-S046*

### DES-009

adapterが返す導出結果はregistryでmergeし、adapter ID・path・Test IDの順に正規化する。

### DES-S004 2.2 宣言鎖と照合

*導出元: REQ-S004, REQ-S005*

### DES-010

上流文書はすべて単一の総称ノード型 `document` で表現する。

### DES-S005 2.3 導出できる関係は保存しない

*導出元: P-003*

### DES-011

Evidenceに含むtarget参照は、target別の実行事実と内容ハッシュを束縛する実行時snapshot keyである。

### DES-S006 2.4 adapter 設定と wire 互換

*導出元: REQ-S048, REQ-S058*

### DES-012

`config.yaml` writerの正規形はversion 2とする。

### DES-013

`config.yaml` writerはadapterごとにroot・scan・run設定をnamespace化する。

### DES-014

readerはversion 1を単一の `rust-cargo` adapter設定としてin-memory変換して読み取る。

### DES-015

`vtest init` はversion 2を生成する。

### DES-016

core domainの `TestEntity` は、言語・runner非依存の `execution`（adapter・project・suite・opaque selector）だけを実行座標として持つ。

### DES-017

`filter` / `package` / `test_target` は `TestEntity` のfieldではない。

### DES-018

Test JSONのwire compatibility layerは `execution` を常に出力する。

### DES-019

Test JSONのwire compatibility layerは `rust-cargo` Testについてだけversion 1互換fieldを追加出力できる。

### DES-020

非Rust Testでは version 1互換fieldを省略する。

### DES-021

`targets` listを常に出力する。

### DES-022

単数互換field `target` はtarget 1件のときだけ追加出力する。

## DES-S007 3 エンティティと ID 体系

### DES-S008 3.1 エンティティ種別

*導出元: REQ-S005, REQ-S025, REQ-S035, REQ-S036, REQ-S040, REQ-S046*

### DES-023

documentは総称の上流文書ノード（path＋content_hash＋derives_from）である。

### DES-024

Relationは外部関係レコードであり、不変とする。

### DES-025

derives_fromの説明文もRelationに保持できる。

### DES-026

判断記録は `UNKNOWN` への外部判断であり、追記型とする。

### DES-027

承認記録は判断・方針の正式採用であり、追記型とする。

### DES-028

Execution Evidenceは実行証拠レコードであり、追記型とする。

### DES-S009 3.2 ID 規則と関係リンク

*導出元: REQ-S007, REQ-S027, REQ-S058*

### DES-029

関係リンクは説明文・導出理由を任意（optional）で保持できる。

### DES-030

Relation writerは `REL-<ULID>` を正規IDとしてファイル名に用いる。

### DES-031

readerはversion 1互換入力としてbare ULIDを `REL-<ULID>` へin-memory正規化する。

### DES-032

判断・承認・EvidenceのIDはbare ULIDとする。

### DES-033

ULID payloadにより並列生成時のファイル名衝突を実用上排除する。

### DES-S010 3.3 Source Target の識別

*導出元: R-3, REQ-S027*

### DES-034

Target Referenceは、adapter IDとadapter所有のopaque locatorの組、または任意のSRC ID参照である。

> <adapter-id>::<opaque-locator>
> 例：rust-cargo::src/parser.rs::Parser::parse

## DES-S011 5 検査

### DES-S012 5.5 決定論的に検出可能な不成立構造

*導出元: REQ-S024*

### DES-035

`rust-cargo` adapterのStatic Audit capabilityは、§8.3の不成立構造を決定論的に検出する。

## DES-S013 6. 証拠

*導出元: REQ-S019*

### DES-036

Testの内容ハッシュは Test construct だけでなく Test subject 全体（少なくとも adapter ID・Test ID・全論理 field・Source Location・実行座標・Test construct）へ束縛する。

### DES-037

adapterはsource range・source bytes・解析した論理metadata・実行座標をhash未計算のdiscovery DTOとして返す。

### DES-038

coreが言語非依存の正規化規則でsubject hashを計算してからTest Entityを具体化する。

## DES-S014 7. 判定権威

*導出元: REQ-S020, REQ-S054*

### DES-039

`rust-cargo` adapterにおける判定権威は `cargo test` である。

## DES-S015 11 発見・意味判定のエスカレーションと判断記録

*導出元: REQ-S035, REQ-S046*

### DES-S016 11.3 判断の記録と再検証

### DES-040

判断記録は少なくとも「誰が（actor）」「何を（subject）」「どう判断したか（decision）」を必須項目とする。

### DES-041

`vtest` が判断対象の情報一式（VO、Test Intent、テストコード、対象実装、関連テスト、既知partition、過去の判断、対象の内容ハッシュとリビジョン）を構造化出力する（bundle生成）。

### DES-042

判断記録と承認記録は同一entityであることを要求しない（別entityでありうる）。

## DES-S017 15 Structured Test Operation

*導出元: P-004, REQ-S039*

### DES-S018 15.1 desired state 方式

### DES-043

adapterが現状との差分を計算してTest constructとmetadata宣言を更新する。

### DES-044

coreが結果を再スキャンして検証する。

### DES-S019 15.2 入力検証

### DES-045

解決不能な場合はadapterが候補を提示する。

### DES-S020 15.4 Form Schema

### DES-046

`rust-cargo` adapterが組込schemaを登録する。

### DES-047

schemaはそれを処理するadapter IDを別fieldで宣言する。

### DES-048

registryは `kind` からちょうど1件のStructured Test adapterへ解決できる場合だけ操作を許可する。

### DES-049

registryは重複・未知adapter・未対応capability・曖昧な対応を拒否する。

### DES-050

未知のformをcoreがRust用として推測してはならない。

## DES-S021 16. 仕様入力（文書層）

*導出元: REQ-S045*

### DES-051

取り込まれた上流成果物はcontent_hashとderives_fromを持つ。

*導出元: REQ-233, REQ-234, REQ-235*

*引用: 要件定義 §18*

## DES-S022 17. 承認

*導出元: REQ-S018, REQ-S046*

### DES-052

承認記録は「誰が（approver）」「何を（subject または judgment reference）」「どの承認状態か（approved state）」を必須項目として追跡可能とする。

### DES-053

承認主体は種別（`human` / `agent`）と識別子（エージェント名・モデル名等）を記録する。

## DES-S023 21. テスト実行と Execution Evidence

*導出元: REQ-S019, REQ-S054*

### DES-054

Evidenceには少なくともTest IDと実行結果（ランナーが報告した `PASS` / `FAIL`）を含める。

### DES-055

Evidenceには実行したadapter IDを含める。

### DES-056

Evidenceには実行時のリポジトリリビジョン（Git commit hash）とdirtyフラグを含める。

### DES-057

Evidenceには現在のTest subject全体の内容ハッシュ、および全宣言targetを解決したcanonical Target Referenceとimplementation constructの内容ハッシュを含める。

### DES-058

Evidenceには実行時HEAD revision、実行adapter・runner・toolchain・実行影響config、現在の実行可能状態を変えうるrepository / local dependency入力の完全なsnapshotを束縛したExecution State subjectを含める。

### DES-059

Evidenceには実行日時と実行方式を含める。

### DES-060

Evidenceには `target_binding`（§5.3）のtarget別結果とfail-closed集約結果（実施した場合）を含める。

### DES-S024 21.1 Evidence の鮮度（ハッシュ束縛による設計制約）

### DES-061

Evidence readerはadapter IDを欠く互換recordも履歴として読み取れる。

### DES-062

Evidence readerは、現在のTestが `rust-cargo` で互換runner情報と内容ハッシュからRust実行と一意に確認できる場合に限り評価する。

## DES-S025 24 データ保存の基本方針

*導出元: REQ-S050, REQ-S058*

### DES-S026 24.2 並列編集耐性の設計原則

### DES-063

record / エンティティファイルの書込みは原子的に公開する。

## DES-S027 26 インターフェース概要

*導出元: REQ-S049, REQ-S058*

### DES-S028 26.2 MCP ツール体系

### DES-064

MCPがCLIと異なるadapterを暗黙選択してはならない。

## DES-S029 1. 実装構成

### DES-S030 1.2 主要依存クレート

*導出元: SPEC-S063, DS-S063, BD-S029*

### DES-065

Git 操作（HEAD の取得、dirty 判定）は `git` CLI の呼び出しで行う（`git rev-parse HEAD`、`git status --porcelain`）。

### DES-S031 1.3 内容ハッシュの定義

*導出元: SPEC-S002, SPEC-S007, SPEC-S026, SPEC-S027, SPEC-S051, DS-S002, DS-S008, DS-S025, DS-S030, DS-S031, DS-S050, BD-S002, BD-S007, BD-S013, BD-S016*

### DES-066

内容ハッシュは `sha256:<hex>` 形式で記録する。

### DES-067

subject固有規則でbyte-exactを要求しないテキストfragmentは、改行をLFへ統一する。

### DES-068

subject固有規則でbyte-exactを要求しないテキストfragmentは、各行の末尾空白を除去する。

### DES-069

改行の統一と各行末尾空白の除去以外の空白は正規化しない。

### DES-070

hash inputはdomain separatorと長さ付きfieldから構成する。

### DES-071

hash inputの各fieldは `field-name`、UTF-8 byte length、byte列の順にencodeする。

### DES-072

hash inputのencodeは単純な文字列連結を行わない。

### DES-073

hash inputにおいて、mapはkey昇順でencodeする。

### DES-074

hash inputにおいて、集合として扱う `covers`・`targets`・`related` は正規化値の昇順でencodeする。

### DES-075

hash inputにおいて、順序に意味がある `cases` は宣言順でencodeする。

### DES-076

hash inputにおいて、null、空文字、空listは異なる値としてencodeする。

### DES-077

Test subject hashはdomain `vtest:test-subject:v1` を用い、adapter ID、Test ID、全canonical metadata、Source Locationのadapter・project-relative path・opaque locator、ExecutionDescriptor、および正規化したTest construct bytesを束縛する。

### DES-078

Test subject hashは、byte range自体を前方の無関係な編集で変化するためhash inputにしない。

### DES-079

metadata宣言がmanifest等の非隣接箇所に存在しても、Test subject hashはadapterが返す論理metadataを同じsubjectへ含める。

### DES-080

canonical metadataは `id` / `covers` / `targets` / `intent` / `input` / `expect` / `kind` / `cases` / `related` からなる。

### DES-081

canonical metadataは、`role` / `anchor` 等の存在理由分類 fieldを本versionでは持たない（§4.1）。

### DES-082

Test subject hashは、宣言の不在と空値の明示を異なる値としてencodeする。

### DES-083

Source Target hashはdomain `vtest:target-subject:v1` を用い、canonical Target Referenceとadapterが返すimplementation construct bytesを束縛する。

### DES-084

construct bytesへの束縛は当該実現形態（`rust-cargo` 等のSource Target形態）に対する規則であり、Source Targetを宣言しない検証対象形態へは適用しない。

### DES-085

canonical Target Referenceは常に `TargetRef::Locator`（adapter IDとadapter所有のopaque locator）であり、`TargetRef::SrcId` をcanonical Target Referenceにしない。

### DES-086

`TargetRef::SrcId` はSource Targetを参照する側の表現であって、Source Target自身の識別ではない。

### DES-087

恒久SRC IDの宣言・変更・削除はcanonical Target Referenceを変えない。

### DES-088

恒久SRC IDの宣言をSource Targetのconstruct bytesの内側へ置くadapter（`rust-cargo` の `@vtest.src-id` doc comment等）では、その宣言の追加・変更・削除がconstruct bytesを変化させ、construct bytes経由でSource Target hashが変化しうる（§5.5）。

### DES-089

恒久SRC IDの宣言をSource Targetのconstruct bytesの内側へ置くadapterで、その宣言の追加・変更・削除がconstruct bytesを変化させ、construct bytes経由でSource Target hashが変化しうることは正しい挙動であり、恒久SRC IDが独立したhash fieldであることを意味しない。

### DES-090

Source Target hashはSource Target自身のcanonical Locatorから一度だけ計算し、当該Source Targetを参照するTest側の `TargetRef` 綴りからは計算しない。

### DES-091

document subject hashはdomain `vtest:document-subject:v1` を用い、canonical document recordと参照先source（`path` の実ファイル）の正規化内容を束縛する。

### DES-092

document subject hashは、要件定義・基本仕様・詳細設計・API Schema等を種別で区別せず、すべて同一の総称document subjectとして計算する（§3.1）。

*導出元: SPEC-043, SPEC-044, SPEC-279, SPEC-280*

*引用: 基本仕様 §3.2*

### DES-093

VO subject hashはdomain `vtest:record-subject:v1` を用い、readerが具体化したcanonical VO recordをfield規則に従ってencodeする。

### DES-094

VO subject hashは、VOの読取り互換field `status` を正典ではないため含めない。

### DES-095

VO subject hashは `derives_from`（参照先document ID集合）と `parent` を束縛する。

### DES-096

VO subject hashは、`covers` の増減をTest側subjectで捕捉するため含めない。

### DES-097

Execution State subject hashはdomain `vtest:execution-state:v1` を用い、adapter ID、snapshot schema ID / version、HEAD revision、runner kindとcanonical invocation projection、toolchain identity、実行結果へ影響するadapter configのcanonical projection、および実行可能状態を変えうるrepository / local dependency入力の完全なmanifestを束縛する。

### DES-098

manifest entryはstable root identity、root-relative path、input kind、byte-exact file bytesからなる。

### DES-099

manifest entry集合は正規化identity順にencodeする。

### DES-100

stable root identityはmachine上の絶対pathを用いず、workspace内の論理rootまたはdependency identityから決定論的に導出する。

### DES-101

adapterはhash未計算のmanifestと完全性を返し、coreが各entryとsubject全体を検証・hash化する。

### DES-102

adapterはsource location、source rangeと現在のbytes、解析済みlogical metadata、ExecutionDescriptorをhash未計算のdiscovery DTOとして返す。

### DES-103

Test Runner adapterも、実行状態へ用いたconfig / manifestをhash未計算DTOとして返す。

### DES-104

coreは現在bytesとの対応、重複、集合完全性、schema versionを検証してsubject hashを計算する。

### DES-105

adapterは、完全性を保証できないDTOから `PASS` 用subjectを具体化してはならない。

### DES-106

`rust-cargo` adapterはTest constructとして、metadata doc commentを除き、実行に影響する属性、signature、bodyを含む関数itemのbytesを返す。

### DES-107

doc comment由来metadataはlogical metadataと `metadata_sources` として別に返す。

### DES-108

`rust-cargo` adapterはSource Targetには属性とdoc commentを含む関数item全体を返す。

## DES-S032 2. データディレクトリと設定

### DES-S033 2.2 `config.yaml`

*導出元: SPEC-S012, SPEC-S019, SPEC-S050, SPEC-S053, DS-S007, DS-S013, DS-S018, DS-S021, DS-S049, DS-S053, BD-S011*

### DES-109

`config.yaml` readerはversion 1を単一の `rust-cargo` adapter設定としてin-memory変換して読み取るが、読み取りだけで正典を書き換えない。

*引用: 基本仕様 §2.4*

### DES-110

`gates` はフェーズゲートの進行条件定義を保持する（§11.5）。

### DES-111

`doc.roots` は orphan_detection の除外根をDOC IDの集合として保持する（§5.6）。

### DES-S034 2.3 派生情報

*導出元: P-003, SPEC-S058, DS-S059, BD-S025*

### DES-112

MCPサーバは長時間動作するため、ツール呼び出しごとに対象ファイルのmtimeを確認し、変化があれば再スキャンする。

## DES-S035 3. レコードファイルスキーマ

### DES-S036 3.1 document レコード（`.verify/doc/DOC-*.yaml`）

*導出元: SPEC-S008, SPEC-S009, SPEC-S044, DS-S009, DS-S010, DS-S045, BD-S008*

### DES-113

document レコードの `path` fieldはプロジェクト相対パスである。

> id: DOC-BASIC-001
> path: docs/basic-spec.md        # プロジェクト相対パス
> content_hash: "sha256:..."      # 登録時の内容ハッシュ（§1.3 document subject）
> title: 基本仕様書               # 任意の表示名
> derives_from:                   # 上流 document への導出リンク（0件可＝根候補）
>   - doc: DOC-REQ-001
>     anchor: "§12.3"             # 任意の上流該当箇所（節番号等・空可・非 MISMATCH）
>     note: ""                    # 任意の導出理由（空可・非 MISMATCH。基本仕様 §3.4）
> registered_at: 2026-08-08T00:00:00Z

### DES-114

document レコードの `content_hash` fieldは登録時の内容ハッシュである（§1.3）。

### DES-115

`anchor` は `derives_from` entryのfieldであり、Test metadataには存在しない（§4.1）。

*導出元: SPEC-123, SPEC-124, SPEC-369, SPEC-370, SPEC-371, SPEC-372, SPEC-373, SPEC-374, SPEC-375, SPEC-376, SPEC-377, SPEC-378, SPEC-379, SPEC-380, SPEC-381, SPEC-382, SPEC-383, SPEC-384, SPEC-385, SPEC-386, SPEC-387, SPEC-388, SPEC-389, SPEC-390, SPEC-391, SPEC-392, SPEC-393, SPEC-394, SPEC-395, SPEC-396, SPEC-397, SPEC-398, SPEC-399, SPEC-400, SPEC-401, SPEC-402, SPEC-403, SPEC-404, SPEC-405, SPEC-406, SPEC-407*

*引用: 基本仕様 §12*

### DES-116

`anchor` はcanonical document recordの一部であり、document subject hashの入力に含まれる（§1.3）。

### DES-S037 3.2 VO レコード（`.verify/vo/VO-*.yaml`）

*導出元: SPEC-S009, SPEC-S030, DS-S010, DS-S034*

### DES-117

VO レコードの `parent` fieldはVO IDまたはnull（階層化）である。

> id: VO-PARSER-UTF8-003
> parent: VO-PARSER-UTF8          # VO ID または null（階層化）
> derives_from:                   # 1件以上の document への直結（基本仕様 §3.2）
>   - doc: DOC-BASIC-001
>     anchor: "§8.2条項2"         # 任意の上流該当箇所（節番号等・空可・非 MISMATCH）
>     note: ""                    # 任意（空可・非 MISMATCH）
> claim: 不正な continuation byte を含む入力を与えた場合、ParseError::InvalidUtf8 を返す
> dimensions: []                  # 検証軸（任意。§3.2.1）
> coverage_policy: null           # independent-axes | full-product | explicit | null
> combinations: []                # coverage_policy: explicit のとき実体化する組合せ（§3.2.1）
> representative_cases: []        # 代表入力値（任意）
> created: 2026-08-08
> updated: 2026-08-08

### DES-118

VO レコードの `coverage_policy` fieldの値域は `independent-axes` / `full-product` / `explicit` / `null` である。

### DES-119

VO レコードの `combinations` fieldは `coverage_policy: explicit` のとき実体化する組合せである（§3.2.1）。

### DES-120

VOの `status` は承認レコードから導出する表示値であり、canonical writerはVO recordへ保存しない。

#### DES-S038 3.2.1 dimensions と組合せの実体化

*導出元: SPEC-S030, DS-S034*

### DES-121

`combinations` は組合せtupleのlistである。

> dimensions:
>   - name: operand-sign
>     partitions: [positive, negative]
>   - name: operator
>     partitions: [add, sub, mul, div]
> coverage_policy: explicit
> combinations:
>   - { operand-sign: positive, operator: div }
>   - { operand-sign: negative, operator: div }

*導出元: SPEC-092, SPEC-093, SPEC-094, SPEC-095, SPEC-096, SPEC-097, SPEC-098, SPEC-099, SPEC-100, SPEC-101, SPEC-102, SPEC-103, SPEC-104, SPEC-105, SPEC-318*

*引用: 基本仕様 §10*

### DES-122

`combinations` はcanonical VO recordの一部であり、VO subject hashに束縛される（§1.3）。

### DES-S039 3.3 Relation レコード（`.verify/rel/REL-<ULID>.yaml`）

*導出元: SPEC-S006, SPEC-S009, DS-S006, DS-S010, BD-S006*

### DES-123

Relationレコードの `type` fieldの値域は `depends-on` / `supersedes` / `regression-for` / `derived-from` / `same-partition` / `complements` / `conflicts-with` である。

> id: REL-01J8XVZK3Q...
> type: depends-on          # depends-on | supersedes | regression-for |
>                           # derived-from | same-partition | complements | conflicts-with
> from: TEST-PARSER-044     # 任意のエンティティID
> to: TEST-PARSER-012
> note: ""                  # 任意の説明文
> created: 2026-08-08T00:00:00Z

### DES-124

writerは `.verify/rel/REL-<ULID>.yaml` と同値の `id` だけを生成する。

### DES-125

readerはversion 1互換入力として `.verify/rel/<ULID>.yaml` かつ同値のbare `id` を受理し、`REL-<ULID>` へin-memoryで正規化するが、読み取りだけでファイルを書き換えない。

### DES-126

Relationは不変である。

### DES-127

Relationの変更はファイル削除＋新規作成で表す。

### DES-S040 3.4 判断記録レコード（`.verify/decisions/<ULID>.yaml`）

*導出元: REQ-S035, SPEC-S034, DS-S037*

### DES-128

判断記録の `subject` fieldは判断対象のエンティティIDまたは解決済みcanonical Locatorである。

> id: 01J8XVZZ...
> subject: TEST-PARSER-044        # 判断対象のエンティティID または解決済み canonical Locator
> judgment_kind: test-semantic    # 判断型（必須。値域は §8.1）
> supersedes: []                  # 明示に置き換える旧判断記録の ULID list（既定は空 list。§8.5）
> subject_hash: "sha256:..."      # 判断時点の対象の内容ハッシュ
> dependencies:                   # 判断時点の上流依存closure（完全一致を要求）
>   - kind: vo
>     id: VO-PARSER-UTF8-003
>     hash: "sha256:..."
>   - kind: document
>     id: DOC-BASIC-001
>     hash: "sha256:..."          # §1.3 document subject hash
> actor:                          # 誰が（必須）
>   kind: agent                   # human | agent
>   id: judge-agent-01
>   model: claude-fable-5         # agent の場合任意
> decision: accepted              # どう判断したか（必須。値の妥当性は §8.4）
> reason:                         # 理由・根拠・evidence note（任意。空でも無効化しない）
>   - claim: テストは不正UTF-8入力に対する InvalidUtf8 の返却を検証している
>     basis:
>       - kind: test-code
>         ref: "rust-cargo::tests/parser_test.rs::rejects_invalid_utf8"
> exclusions: []                  # 対象外とした範囲（任意）
> decided_at: 2026-08-08T00:00:00Z
> revision: { commit: "abc123...", dirty: false }

### DES-129

判断記録の `subject_hash` fieldは判断時点の対象の内容ハッシュである。

### DES-130

判断記録の `dependencies` entryの `hash` fieldはdocument subject hashである（§1.3）。

### DES-131

判断記録の `actor` の `kind` fieldの値域は `human` / `agent` である。

### DES-132

`judgment_kind` は判断対象を一意に区切る第二のkeyである。

### DES-133

`supersedes` は、この判断記録が明示に置き換える旧判断記録のULIDを名指しするlistである。

### DES-S041 3.5 承認レコード（`.verify/approvals/<ULID>.yaml`）

*導出元: REQ-S046, SPEC-S016, SPEC-S043, SPEC-S066, DS-S017, DS-S044*

### DES-134

承認レコードの `subject` fieldは承認対象のエンティティID（VO IDまたはdocument ID）である。

> id: 01J8XW0A9M...
> subject: VO-PARSER-UTF8-003     # 承認対象のエンティティID（VO ID または document ID）
> judgment_ref: 01J8XVZZ...       # 参照する判断記録ID（任意。judgment reference）
> supersedes: []                  # 明示に置き換える旧承認レコードの ULID list（既定は空 list）
> subject_hash: "sha256:..."      # 承認時点の対象の内容ハッシュ
> dependencies:                   # 承認時点の上流依存closure（完全一致を要求）
>   - kind: vo
>     id: VO-PARSER-UTF8
>     hash: "sha256:..."
>   - kind: document
>     id: DOC-BASIC-001
>     hash: "sha256:..."          # §1.3 document subject hash
> approver:
>   kind: agent                   # human | agent
>   id: reviewer-agent-01
>   model: claude-fable-5         # agent の場合任意
> approved_state: approved        # どの承認状態か（必須。approved | rejected | withdrawn）
> basis: []                       # 根拠（任意）
> approved_at: 2026-08-08T00:00:00Z

### DES-135

承認レコードの `judgment_ref` fieldは参照する判断記録IDであり、任意（judgment reference）とする。

### DES-136

承認レコードの `supersedes` fieldは明示に置き換える旧承認レコードのULID listであり、既定は空listとする。

### DES-137

承認レコードの `subject_hash` fieldは承認時点の対象の内容ハッシュである。

### DES-138

承認レコードの `dependencies` fieldは承認時点の上流依存closureであり、完全一致を要求する。

### DES-139

承認レコードの `dependencies` entryの `hash` fieldはdocument subject hashである（§1.3）。

### DES-140

承認レコードの `approver` の `kind` fieldの値域は `human` / `agent` である。

### DES-141

承認レコードの `approver` の `model` fieldはagentの場合任意とする。

### DES-142

承認レコードの `approved_state` fieldはどの承認状態かを表し、必須とする。値域は `approved` / `rejected` / `withdrawn` である。

### DES-143

承認レコードの `basis` fieldは根拠であり、任意とする。

### DES-144

承認記録は判断記録と同一entityであることを要求しない（別entityでありうる）（§3.4）。

### DES-145

承認は対象自身（`subject`）または参照する判断（`judgment_ref`）に承認済み状態を与える。

### DES-146

承認対象は `vo`・`document`・`judgment` の3種であり、レコード上の表現は種別ごとに定まる。

### DES-147

承認対象種別 `vo` は、レコード上 `subject` にVO ID（`VO-*`）で表現し、上流依存closureは対象VOの再帰的なparent VO、対象VOと各parent VOが `derives_from` で参照するdocument、および各documentの再帰的な上位document（`derives_from` 先）である。

### DES-148

承認対象種別 `document` は、レコード上 `subject` にdocument ID（`DOC-*`）で表現し、上流依存closureは対象documentの再帰的な上位document（`derives_from` 先）である。

### DES-149

承認対象種別 `document` は、総称documentとして登録した文書で表現し、専用のエンティティ型を設けない（§3.1）。

*導出元: SPEC-038, SPEC-039, SPEC-040, SPEC-041, SPEC-042, SPEC-277, SPEC-278*

*引用: 基本仕様 §3.1*

### DES-150

承認対象種別 `judgment` は、レコード上 `judgment_ref` に判断記録ULIDを置き、`subject` には当該判断記録の `subject`（VO IDまたはdocument ID）を置き、上流依存closureは `subject` の種別に応じた同表のclosureとする。

### DES-151

判断記録ULIDは `subject` に置かない。判断記録の承認は `judgment_ref` によってのみ表す。

### DES-152

`dependencies` のentryは `kind`（`vo` | `document`）、`id` の順でsortし、欠落・重複・余剰entryを許可しない。

### DES-153

`approved_state` は `approved` / `rejected` / `withdrawn` の3値だけを受理する。

### DES-154

承認レコードの `supersedes` は、この承認レコードが明示に置き換える旧承認レコードのULIDを名指しするlistである。

### DES-155

`supersedes` に列挙する各ULIDは、同一 `subject`（`judgment_ref` を持つ承認では同一 `judgment_ref`）の既存承認レコードを指さなければならない。

### DES-156

実効承認は対象X（VO ID、document ID、または `judgment_ref` が指す判断記録）に対して所定の順に評価して導出する。

### DES-157

`approved_state` を参照せずに承認済みを導出してはならない。

### DES-158

承認レコード a が A(X) に属するのは、`a.approved_state` が値域内であること、a の対象指定が X と一致すること（X が VO / document のとき `a.subject == X`、X が判断記録のとき `a.judgment_ref == X` の ULID）、`a.subject_hash` が `a.subject` の現在の内容ハッシュと一致すること、`a.dependencies` が `a.subject` の現在の上流依存closureと entity・hash とも完全一致すること、`a.dependencies` の各 document が登録 content_hash と実ファイルの一致を満たすこと（§11.4）、および X が判断記録のとき当該判断記録が §8.5 の有効判断でありかつ §8.5 の実効集合 E に属すること、をすべて満たす場合だけとする。

### DES-159

承認レコードaがA(X)に属するには、`a.approved_state` が値域内でなければならない。

### DES-160

承認レコードaがA(X)に属するには、aの対象指定がXと一致しなければならない（XがVO / documentのとき `a.subject == X`、Xが判断記録のとき `a.judgment_ref == X` のULID）。

### DES-161

承認レコードaがA(X)に属するには、`a.subject_hash` が `a.subject` の現在の内容ハッシュと一致しなければならない。

### DES-162

承認レコードaがA(X)に属するには、`a.dependencies` が `a.subject` の現在の上流依存closureとentity・hashとも完全一致しなければならない。

### DES-163

承認レコードaがA(X)に属するには、`a.dependencies` の各documentが登録content_hashと実ファイルの一致を満たさなければならない（§11.4）。

### DES-164

Xが判断記録のとき、承認レコードaがA(X)に属するには、さらに当該判断記録が §8.5 の有効判断であり、かつ §8.5 の実効集合Eに属さなければならない。

### DES-165

実効集合 A’(X) は、A(X) から、A(X)内の他レコードの `supersedes` に名指しされたものを除いた集合である。

### DES-166

A’(X) が空の場合、実効承認状態は `draft` とする。

### DES-167

A’(X) に `approved_state` が `rejected` または `withdrawn` のレコードが1件以上ある場合、実効承認状態は `draft` とする（fail-closed。機械はどちらかを選ばない）。

### DES-168

A’(X) の全レコードが `approved_state == approved` の場合、実効承認状態は `approved` とする。

### DES-169

実効集合 A’(X) からの除外は `supersedes` による明示の名指しだけで起きる。

### DES-170

`supersedes` 関係にない複数の有効承認レコードはすべてA’(X)に属する。

### DES-171

同一対象に `approved` と `rejected` の有効承認レコードが `supersedes` 関係なく併存する場合、機械はどちらも採らずfail-closedに `draft` とする。

### DES-172

`approved` を取り消すには `approved_state: withdrawn` の承認レコードを追加する。

### DES-173

`approved` を否認するには `approved_state: rejected` の承認レコードを追加する。

### DES-175

旧レコードを名指ししない `approved` の追加では `draft` のままとする。

### DES-176

対象Xの実効承認状態は `draft` と `approved` の2値であり、遷移は所定の入力だけで起きる。

### DES-177

`draft` から `approved` への遷移は、`approved_state: approved` の有効承認レコードが加わり、実効集合に `rejected` / `withdrawn` が1件も残らなくなることで起きる。

### DES-178

`approved` から `draft` への遷移は、`approved_state` が `rejected` または `withdrawn` の有効承認レコードが加わることで起きる。

### DES-179

`approved` から `draft` への遷移は、実効集合の `approved` レコードがすべて他レコードの `supersedes` に名指しされることで起きる。

### DES-180

`approved` から `draft` への遷移は、`subject` の内容ハッシュが変化することで起きる。

### DES-181

`approved` から `draft` への遷移は、上流依存closureのentity構成またはいずれかのhashが変化する（document再登録・参照先source変更を含む）ことで起きる（§11.4）。

### DES-182

Xが判断記録のとき、`approved` から `draft` への遷移は、当該判断記録が §8.5 の有効判断または実効集合Eから外れることで起きる。

### DES-S042 3.6 Evidence レコード（`.verify/evidence/<ULID>.yaml`）

*導出元: SPEC-S022, SPEC-S051, DS-S025, DS-S050, BD-S013*

### DES-183

Evidenceレコードの `result` fieldはランナーが報告した `PASS` | `FAIL` である（判定権威§7）。

> id: 01J8XW1B...
> test_id: TEST-PARSER-044
> adapter: rust-cargo
> result: PASS                    # ランナーが報告した PASS | FAIL（判定権威 §7）
> executed_at: 2026-08-08T00:00:00Z
> revision: { commit: "abc123...", dirty: false }
> execution_state:
>   schema: rust-cargo-execution-state-v1
>   complete: true
>   hash: "sha256:..."             # complete: falseの場合だけnull
> hashes:
>   test_subject: "sha256:..."
>   targets:
>     - target: "rust-cargo::src/parser.rs::Parser::parse"
>       target_construct: "sha256:..."
>     - target: "rust-cargo::src/lexer.rs::Lexer::next"
>       target_construct: "sha256:..."
> runner:
>   kind: cargo-test
>   command: "cargo test -p parser --lib -- --exact parser::tests::rejects_invalid_utf8"
>   exit_code: 0
> target_coverage:                # target_binding の動的計測結果（旧 target_execution field を改名）
>   checked: true                 # 計測を実施したか
>   method: llvm-cov
>   result: FAIL                  # target別結果の集約: PASS | FAIL | UNKNOWN
>   targets:
>     - target: "rust-cargo::src/parser.rs::Parser::parse"
>       result: PASS              # PASS | FAIL | UNKNOWN
>       count: 3                  # UNKNOWNではnull
>     - target: "rust-cargo::src/lexer.rs::Lexer::next"
>       result: FAIL
>       count: 0
> log_ref: "cache/logs/01J8XW1B.log"   # Git管理外の生ログ

### DES-184

Evidenceレコードの `execution_state` の `hash` fieldは、`complete: false` の場合だけnullとする。

### DES-185

Evidenceレコードの `target_coverage` fieldは `target_binding` の動的計測結果である（旧 `target_execution` fieldを改名）。

### DES-186

Evidenceレコードの `target_coverage.checked` fieldは計測を実施したかを表す。

### DES-187

Evidenceレコードの `target_coverage.result` fieldはtarget別結果の集約であり、値域は `PASS` / `FAIL` / `UNKNOWN` である。

### DES-188

Evidenceレコードの `target_coverage` の `targets` 配下の各entryの `result` fieldの値域は `PASS` / `FAIL` / `UNKNOWN` である。

### DES-189

Evidenceレコードの `target_coverage` の `targets` 配下の各entryの `count` fieldは、`result` が `UNKNOWN` の場合nullとする。

### DES-190

Evidenceレコードの `log_ref` fieldはGit管理外の生ログを指す。

### DES-191

旧モデルの `target_execution` 検査項目は撤去し、その計測事実だけをEvidenceの `target_coverage` fieldとして保持して `target_binding` の証拠源へ吸収する（§10・§11.2）。

### DES-192

`hashes.targets` はTestの宣言順で常に記録し、各 `target` は§6.1で解決したcanonical Source Targetのcanonical Locatorの正規化文字列表現とする。

### DES-193

参照側Testが宣言した `TargetRef` の綴り（SRC ID参照を含む）をEvidence上のtarget identityとして記録しない（§6.1.1）。

### DES-194

`hashes.targets` のlistはTestの宣言target集合を解決したcanonical Source Target集合と重複なく1対1に対応する。

### DES-195

Evidence生成のprecondition（§9.4）により全宣言targetは一意に解決済みであるため、`hashes.targets` の集合は宣言target集合と同数になる。

### DES-196

`target_coverage.checked: true` では `target_coverage.targets` も同じ順序・同じcanonical Locator集合で1対1に対応する。

### DES-197

writerは `hashes.test_subject` を必須とする。

### DES-198

writerはTest construct単体のhashを現在のEvidence freshness keyとして出力しない。

### DES-199

readerは `rust-cargo` Evidenceに限り、互換fieldの `hashes.test_fn` または `hashes.test_construct` とtarget entry内の `target_fn` を読み取れる。

### DES-200

互換Test hashを現在の `test_subject` へ正規化できるのは、現在の `rust-cargo` adapterが当該互換hashのsource rangeに全canonical metadataとTest constructが含まれること、現在bytesとの完全一致、および現在のlogical metadataとの一致を証明できる場合だけとする。

### DES-201

証明できなければrecordは保持するが、`target_binding` の証拠として有効な `PASS` にしない。

### DES-202

中立fieldと互換fieldが併存する場合は導出される値の同値を必須とし、非 `rust-cargo` Evidenceでは互換fieldを解釈しない。

### DES-203

readerは単数互換形の `hashes.target_fn` および `target_coverage.result/count` を、現在の `rust-cargo` Testがtargetをちょうど1件宣言し、Test subjectを証明でき、target construct hashも照合できる場合だけ1要素listへ正規化して扱う。

### DES-204

複数target Testに単数互換形を適用しない。

### DES-205

writerは常にlist形を出力する。

### DES-206

`execution_state` は§1.3のExecution State subjectである。

### DES-207

writerは実行直前にadapterからsnapshot DTOを取得し、core検証後のschema ID、完全性、subject hashを記録する。

### DES-208

`complete: true` は、選択Testのビルドと実行可能状態を変えうるrepository / local dependency入力、runner、toolchain、実行影響configをadapterが漏れなく列挙した場合だけ許可する。

### DES-209

snapshot生成不能または不完全の場合も実行事実の履歴を記録できるが、`complete: false`、`hash: null` として現在の有効な `PASS` 証拠へ使用しない。

### DES-210

`rust-cargo-execution-state-v1` のmanifestは、選択Testを含むCargo workspace / package root、全local path dependency root、各root内の通常file、Cargo manifest / lockfile、`.cargo` config、build script、Rust source / test / fixture / compile-time resource、toolchain指定を含む。

### DES-211

`.git/`、`.verify/` のcanonical record / cache、Cargo target directory等の生成物は実行入力から除外する。

### DES-212

除外領域をbuild script、macro、`include_*`、path dependencyその他の経路で読み込む可能性を排除できない場合、snapshotを完全と報告しない。

### DES-213

Evidence writerは `adapter` を必須で記録し、保存前にTestの `ExecutionDescriptor.adapter` およびrunner kindとの整合を検証する。

### DES-214

Evidence readerは `adapter` の欠落を許容するが、現在のTestが `rust-cargo` で、互換runner kindと内容ハッシュからRust実行であることを一意に確認できる場合だけ互換Evidenceとして扱う。

## DES-S043 4. Test metadata宣言contract

### DES-S044 4.1 adapter-neutralな正規化

*導出元: REQ-S026, SPEC-S007, SPEC-S027, SPEC-S028, SPEC-S035, DS-S008, DS-S031, DS-S032, DS-S038, BD-S007, BD-S017*

### DES-215

`SourceDiscoveryAdapter` は、adapter所有のsource declarationを `id`、`covers[]`、`targets[]`、`intent`、`input?`、`expect?`、`kind?`、`cases[]`、`related[]` の論理fieldへ正規化する。

> id, covers[], targets[], intent, input?, expect?, kind?, cases[], related[]

### DES-216

本versionでは、Testの存在理由分類（旧 `role` / `anchor` / `anchor_rationale`）を論理fieldに持たない。

### DES-217

locatorは `TargetRef::Locator { adapter, value }` とし、`value` はadapter所有のopaque文字列である。

### DES-S045 4.2 `rust-cargo` annotation文法

*導出元: SPEC-S035, SPEC-S066, DS-S038*

### DES-218

打鍵ミス検出の目的は両表面に及ぶが、表面2の宣言はTest metadataを破損させず採用値の曖昧さも生まないため、errorではなくwarningとする。

### DES-219

`rust-cargo` のSource Target constructは属性とdoc commentを含む関数item全体であり、`@vtest.src-id` の宣言行はconstruct bytesの内側にある（§1.3）。

### DES-220

したがって `@vtest.src-id` の付与・変更・削除はSource Target hashを変化させる。

### DES-S046 4.3 `rust-cargo` locator構文

*導出元: SPEC-S028, SPEC-S066, DS-S032, BD-S017*

### DES-221

`path` は `.rs` で終わる最初の `::` で item-path と分離する。

### DES-222

`rust-cargo` adapterはlocator値を `TargetRef::Locator { adapter: "rust-cargo", value: locator }` へ正規化する。

### DES-S047 4.4 宣言エラーの扱い

*導出元: SPEC-S018, SPEC-S028, SPEC-S035, DS-S020, DS-S032, DS-S038, BD-S017*

### DES-223

adapter固有のsource declarationを構文解析できない場合、adapterは該当Test constructをDiscovered Testとして返し、対応を `ManagedTestLink::Missing` として診断を付与する。

### DES-224

source declarationを構文上完全なTest Entityへ正規化できる条件は、adapter中立coreが要求する必須metadata（構文上有効なTest ID・1件以上の `covers`・`intent`）に加え、当該adapterが必須とする追加metadataをTest Entityとして具体化できることをいう。

### DES-225

E-SCAN-007はadapterが報告する構文・必須metadata診断であり、`targets ≥ 1` は `rust-cargo` の必須metadataとしてこの経路で検出される（core中立の必須リンクへは加えない）（§11.1.1）。

### DES-226

構文上完全なTest Entityへ正規化できるが、`covers` のVO IDをcore storeで解決できない場合、そのentityと `ManagedTestLink::One(id)` を保持する。

### DES-227

E-SCAN-003と `chain_integrity = MISMATCH` はcoreの参照整合性検査で生成する。

### DES-228

E-SCAN-003が発生しても対応するTest Entityと `ManagedTestLink::One` を除去しない。

### DES-229

adapterはTest構文の違反（重複不可キーの重複、未知キー、必須キーの欠落）をE-SCAN-005 / E-SCAN-006 / E-SCAN-007で報告する（§5.4）。

## DES-S048 5. Discovery orchestration設計

### DES-S049 5.1 処理フロー

*導出元: SPEC-S004, DS-S004, DS-S056, BD-S004, BD-S021*

### DES-230

処理フロー第1段は、registryとconfigの検証であり、adapter ID、capability宣言、config namespace、rootを検証する。

### DES-231

処理フロー第2段は、discovery委譲であり、登録順ではなくadapter ID順にSourceDiscoveryAdapterを呼び出し、各adapterはDiscoveryBatchを返す。

### DES-232

処理フロー第3段は、adapter出力の検証であり、adapter ID、Source Location、source range、current bytes、hash未計算のTest draft、metadata source、Target draft、診断を検証する。

### DES-233

処理フロー第3段では、capability宣言と出力が矛盾するbatchは拒否する。

### DES-234

処理フロー第4段は、core materializationであり、Test subjectとSource Targetのhashを§1.3で計算し、TestEntity、SourceTarget、DiscoveredTest、ManagedTestLinkを具体化する（存在理由分類の実効値確定は行わない。covers ≥ 1 を一律要求する）。

### DES-235

処理フロー第5段は、決定論的な統合であり、adapter ID、project-relative path、opaque locator、Test IDの順に正規化する。

### DES-236

処理フロー第5段では、adapter間を含むTest ID・SRC ID衝突と不正な複数対応を検査する。

### DES-237

処理フロー第6段は、`.verify/` 読み込みであり、vtest-storeが全レコード（document / VO / Relation / 判断 / 承認 / Evidence）を読み込み、スキーマ検証する。

### DES-238

処理フロー第7段は、参照整合性検査であり、coversのVO ID、targetsのTarget Reference / SRC ID、Relation、VO parent、VO derives_from（document）、document derives_from を解決する。

### DES-239

処理フロー第8段は、グラフ構築と整合性検査である（§5.3・§5.4・§5.6）。

### DES-S050 5.2 エンティティモデル（vtest-model）

*導出元: SPEC-S007, SPEC-S012, SPEC-S017, DS-S008, DS-S013, DS-S019, BD-S007, BD-S010*

### DES-240

`TestEntity` の `id` fieldは `TestId` 型である。

### DES-241

`TestEntity` の `covers` fieldは `Vec<VoId>` 型であり、1件以上を持つ（`covers ≥ 1` 一律）（§4.4）。

### DES-242

`TestEntity` の `targets` fieldは `Vec<TargetRef>` 型であり、各要素はadapter付きopaque locatorまたはSrcIdである。

### DES-243

`TestEntity.targets` の件数はadapterが定める（`rust-cargo` は `targets ≥ 1` を必須とする）（§4.1・§4.4）。

### DES-244

coreは `targets ≥ 1` を中立必須にせず、`TestEntity.targets` の型としては空を許容する。

### DES-245

`TestEntity` の `intent` fieldは `String` 型である。

### DES-246

`TestEntity` の `input` fieldは `Option<String>` 型である。

### DES-247

`TestEntity` の `expect` fieldは `Option<String>` 型である。

### DES-248

`TestEntity` の `kind` fieldは `Option<String>` 型である。

### DES-249

`TestEntity` の `cases` fieldは `Vec<String>` 型である。

### DES-250

`TestEntity` の `related` fieldは `Vec<TestId>` 型である。

### DES-251

`TestEntity` の `location` fieldは `SourceLocation` 型である。

### DES-252

`TestEntity` の `content_hash` fieldは `ContentHash` 型であり、§1.3のTest subject hash（coreが計算）である。

### DES-253

`TestEntity` の `execution` fieldは `ExecutionDescriptor` 型である。

### DES-254

`TargetRef` は `Locator { adapter: AdapterId, value: String }` variantを持つ。

### DES-255

`TargetRef` は `SrcId(SrcId)` variantを持つ。

### DES-256

`SourceLocation` の `adapter` fieldは `AdapterId` 型である。

### DES-257

`SourceLocation` の `path` fieldは `ProjectPath` 型である。

### DES-258

`SourceLocation` の `locator` fieldは `String` 型であり、adapter所有のopaque construct locatorである。

### DES-259

`SourceLocation` の `byte_range` fieldは `SourceRange` 型である。

### DES-260

`ExecutionDescriptor` の `adapter` fieldは `AdapterId` 型である。

### DES-261

`ExecutionDescriptor` の `project` fieldは `Option<String>` 型である。

### DES-262

`ExecutionDescriptor` の `suite` fieldは `Option<TestSuite>` 型である。

### DES-263

`ExecutionDescriptor` の `selector` fieldは `String` 型である。

### DES-264

`TestSuite` の `kind` fieldは `String` 型である。

### DES-265

`TestSuite` の `name` fieldは `Option<String>` 型である。

### DES-266

`CheckValue` は `Pass` variantを持つ。

### DES-267

`CheckValue` は `Fail` variantを持つ。

### DES-268

`CheckValue` は `Mismatch` variantを持つ。

### DES-269

`CheckValue` は `NoEvidence` variantを持つ。

### DES-270

`CheckValue` は `Unknown` variantを持つ。

### DES-271

`DiagnosticLabel` は `Missing` variantを持つ。

### DES-272

`DiagnosticLabel` は `NotExecuted` variantを持つ。

### DES-273

`DiagnosticLabel` は `NotChecked` variantを持つ。

### DES-274

`DiagnosticLabel` は `Stale` variantを持つ。

### DES-275

`CheckItem` は `ChainIntegrity` variantを持つ。

### DES-276

`CheckItem` は `OrphanDetection` variantを持つ。

### DES-277

`CheckItem` は `TargetBinding` variantを持つ。

### DES-278

`CheckItem` は `OraclePresence` variantを持つ。

### DES-279

`CheckValue` は状態のみを表し、原因説明は `DiagnosticLabel` として併記する。

### DES-280

`Missing` / `NotChecked` / `NotExecuted` / `Stale` を検証状態のvariantとして持たせない（旧8値モデルの排除）。

### DES-281

`TestEntity.execution` はadapter、project、suite、opaque selectorからなる中立な実行座標である。

### DES-282

JSON writerは `execution` を常に出力し、`rust-cargo` TestだけにRust互換fieldを追加する。

### DES-283

非Rust TestではRust互換fieldを省略する。

### DES-284

JSON readerは `execution` を優先し、互換field併存時はdescriptorとの一致を検証する。

### DES-285

`execution` が欠ける場合、完全で相互整合するRust互換fieldからだけ `rust-cargo` descriptorを導出する。

### DES-286

不完全・矛盾時は入力を拒否し、空selectorまたはdummy値を生成しない。

### DES-287

Test JSON writerは `TestEntity.targets` を1件以上のlistとして常に出力する。

### DES-288

targetが1件の場合だけ同値の単数互換field `target` を追加できる。

### DES-289

readerは `target` だけの入力を1要素listへ正規化し、`targets` との併存時は完全一致を検証する。

### DES-290

複数targetから代表値を選んで `target` を生成しない。

### DES-291

`SourceDiscoveryAdapter` は、`SourceFragment`・`ManagedTestDraft`・`DiscoveredTestDraft`・`ManagedTestDraftLink`・`SourceTargetDraft`・`DiscoveryBatch`・`DiscoveryCompleteness` をhash未計算のDTOとして返す。

### DES-292

`SourceFragment` の `location` fieldは `SourceLocation` 型である。

### DES-293

`SourceFragment` の `bytes` fieldは `Vec<u8>` 型である。

### DES-294

`ManagedTestDraft` の `id` fieldは `TestId` 型である。

### DES-295

`ManagedTestDraft` の `covers` fieldは `Vec<VoId>` 型である。

### DES-296

`ManagedTestDraft` の `targets` fieldは `Vec<TargetRef>` 型である。

### DES-297

`ManagedTestDraft` の `intent` fieldは `String` 型である。

### DES-298

`ManagedTestDraft` の `input` fieldは `Option<String>` 型である。

### DES-299

`ManagedTestDraft` の `expect` fieldは `Option<String>` 型である。

### DES-300

`ManagedTestDraft` の `kind` fieldは `Option<String>` 型である。

### DES-301

`ManagedTestDraft` の `cases` fieldは `Vec<String>` 型である。

### DES-302

`ManagedTestDraft` の `related` fieldは `Vec<TestId>` 型である。

### DES-303

`ManagedTestDraft` の `execution` fieldは `ExecutionDescriptor` 型である。

### DES-304

`DiscoveredTestDraft` の `adapter` fieldは `AdapterId` 型である。

### DES-305

`DiscoveredTestDraft` の `location` fieldは `SourceLocation` 型である。

### DES-306

`DiscoveredTestDraft` の `construct` fieldは `SourceFragment` 型である。

### DES-307

`DiscoveredTestDraft` の `metadata_sources` fieldは `Vec<SourceFragment>` 型である。

### DES-308

`DiscoveredTestDraft` の `managed` fieldは `ManagedTestDraftLink` 型である。

### DES-309

`ManagedTestDraftLink` は `Missing` variantを持つ。

### DES-310

`ManagedTestDraftLink` は `One(ManagedTestDraft)` variantを持つ。

### DES-311

`ManagedTestDraftLink` は `Multiple(Vec<ManagedTestDraft>)` variantを持つ。

### DES-312

`SourceTargetDraft` の `target` fieldは `TargetRef` 型である。

### DES-313

`SourceTargetDraft` の `src_id` fieldは `Option<SrcId>` 型である。

### DES-314

`SourceTargetDraft` の `location` fieldは `SourceLocation` 型である。

### DES-315

`SourceTargetDraft` の `construct` fieldは `SourceFragment` 型である。

### DES-316

`DiscoveryBatch` の `adapter` fieldは `AdapterId` 型である。

### DES-317

`DiscoveryBatch` の `completeness` fieldは `DiscoveryCompleteness` 型である。

### DES-318

`DiscoveryBatch` の `discovered_tests` fieldは `Vec<DiscoveredTestDraft>` 型である。

### DES-319

`DiscoveryBatch` の `source_targets` fieldは `Vec<SourceTargetDraft>` 型である。

### DES-320

`DiscoveryBatch` の `diagnostics` fieldは `Vec<Diagnostic>` 型である。

### DES-321

`DiscoveryCompleteness` は `Complete` variantを持つ。

### DES-322

`DiscoveryCompleteness` は `Incomplete` variantを持つ。

### DES-323

Source Targetはcanonical locator（`TargetRef::Locator`）と任意の恒久SRC IDを併有する単一のdomain entityである。

### DES-324

`TargetRef::Locator` と `TargetRef::SrcId` はいずれも同一Source Targetへのaddressing modeであり、別個のentityを指さない。

### DES-325

恒久SRC IDはlocatorの代替ではなく、同じSource Targetへ与えられるoptional permanent identityである。

### DES-326

adapterは `@vtest.src-id` 等で宣言された恒久SRC IDを `SourceTargetDraft.src_id` として返す。

### DES-327

同一constructをlocator版とSrcId版の2件のdraftへ複製してはならない。

### DES-328

`SourceTargetDraft.target` は必ず `TargetRef::Locator` でなければならない（§1.3）。

### DES-329

`TargetRef::SrcId` はSource Targetへの参照表現であり、`SourceTargetDraft` のcanonical targetとして返してはならない。

### DES-330

adapterが `target` に `TargetRef::SrcId` を返した場合はmalformed adapter outputとして拒否する。

### DES-331

恒久SRC IDは `src_id` だけで搬送し、`target` の綴りを変えない。

### DES-332

coreは `src_id` を統合済みSRC索引へ登録し、locator参照とSRC ID参照のどちらから解決しても同一のcanonical Source Targetへ到達させる。

### DES-333

coreは統合済みSRC索引から、その恒久SRC IDを宣言した `SourceTargetDraft.target`（= canonical Locator）へ解決する。

### DES-334

adapterは `SourceFragment.bytes` が `location.byte_range` の現在bytesと一致する状態だけを返す。

### DES-335

coreはrange・bytes対応を検証し、§1.3でhashを計算してから `TestEntity`、`SourceTarget` および `DiscoveredTest` を具体化する。

### DES-336

`ManagedTestDraftLink::One` / `Multiple` の各draftは、全logical metadataを導出した1件以上の `metadata_sources` を持たなければならない。

### DES-337

`DiscoveredTest` の `adapter` fieldは `AdapterId` 型である。

### DES-338

`DiscoveredTest` の `location` fieldは `SourceLocation` 型である。

### DES-339

`DiscoveredTest` の `content_hash` fieldは `ContentHash` 型である。

### DES-340

`DiscoveredTest` の `managed` fieldは `ManagedTestLink` 型である。

### DES-341

`ManagedTestLink` は `Missing` variantを持つ。

### DES-342

`ManagedTestLink` は `One(TestId)` variantを持つ。

### DES-343

`ManagedTestLink` は `Multiple(Vec<TestId>)` variantを持つ。

### DES-344

`SourceDiscoveryAdapter` はadapterがTestとして認識した全Discovered Test draftを返す。

### DES-345

`ManagedTestDraftLink::One` は、構文上有効なTest IDと必須metadata（core中立の `covers ≥ 1` / `intent`、および当該adapterが必須とする追加metadata。`rust-cargo` では `targets ≥ 1`）をdraftとして具体化できる場合に設定する（§4.1・§4.4）。

### DES-346

解決不能な `covers` を持つdraftもcore materialization後のmanaged entity集合に保持され、対応するobservationは `ManagedTestLink::One(id)` を持つ。

### DES-347

`ManagedTestDraftLink::Missing` は管理宣言の欠落または必須metadataの欠落を表す。

### DES-348

`ManagedTestDraftLink` の `Multiple` は同一Test constructから複数draftが生じる状態を表す。

### DES-349

core materialization後の対応する状態が `ManagedTestLink` となる。

### DES-350

各adapterは一意なID、languages、capabilities、config namespaceを宣言する。

### DES-351

registryは宣言と実装の不一致および重複IDを拒否する。

### DES-352

`TestRunnerAdapter` は、coreがfreshness subjectを所有できるよう `ExecutionInputDraft`・`ExecutionStateDraft` をhash未計算のDTOとして返す。

### DES-353

`CanonicalProjection` は型tag、null、list順序、map key順序を保持する言語非依存値とする。

### DES-354

`ExecutionInputDraft` の `root_identity` fieldは `String` 型である。

### DES-355

`ExecutionInputDraft` の `root_relative_path` fieldは `String` 型である。

### DES-356

`ExecutionInputDraft` の `kind` fieldは `String` 型である。

### DES-357

`ExecutionInputDraft` の `bytes` fieldは `Vec<u8>` 型である。

### DES-358

`ExecutionStateDraft` の `schema_id` fieldは `String` 型である。

### DES-359

`ExecutionStateDraft` の `schema_version` fieldは `String` 型である。

### DES-360

`ExecutionStateDraft` の `complete` fieldは `bool` 型である。

### DES-361

`ExecutionStateDraft` の `head_revision` fieldは `Option<String>` 型である。

### DES-362

`ExecutionStateDraft` の `runner_kind` fieldは `String` 型である。

### DES-363

`ExecutionStateDraft` の `invocation` fieldは `CanonicalProjection` 型である。

### DES-364

`ExecutionStateDraft` の `toolchain_identity` fieldは `String` 型である。

### DES-365

`ExecutionStateDraft` の `effective_config` fieldは `CanonicalProjection` 型である。

### DES-366

`ExecutionStateDraft` の `inputs` fieldは `Vec<ExecutionInputDraft>` 型である。

### DES-367

`StaticAnalysisAdapter` は正典レコードを持たない再計算派生であり、判定は現在のsource / target / configから都度計算する（§7.1）。

### DES-368

coreはfreshness subjectを静的解析用に永続化せず、検証のたびに現在入力で再導出する。

### DES-369

Test Runnerはcommand起動前に `ExecutionStateDraft` を構築し、実際に使用するinvocation / toolchain / configと一致するDTOだけを実行結果へ添付する。

### DES-370

`invocation` はselector、working root、runner option等をmachine非依存に正規化し、絶対pathを含む表示用commandとは分離する。

### DES-371

coreは実行前後でExecution State subject全体が変化していないことを確認してからEvidenceを記録する。

### DES-372

有効性再評価では同じschemaを持つ現在DTOを再構築し、保存hashと比較する。

### DES-373

Structured Test capabilityを宣言するadapterは、処理可能なbuilt-in Form `kind` 集合と、adapter fieldを持たないForm Schemaを判定するcompatibility matcherを宣言する。

### DES-374

Form Schemaの `adapter` field、registryのowner、Structured Test capabilityが同じadapter IDを示す場合だけ `kind → adapter` を確定する。

### DES-375

`adapter` fieldを欠く読取り互換Formは、登録済みStructured Test adapterのbuilt-in kind宣言またはcompatibility matcherのうちちょうど1件だけがschemaを受理する場合に限ってin-memoryでownerを補える。

### DES-376

登録済みStructured Test adapterのbuilt-in kind宣言またはcompatibility matcherのうちschemaを受理するものが0件または複数件なら操作を拒否し、ファイルを書き換えない。

### DES-377

matcherはsource bytes、schema field / validator集合等から決定論的に判定し、form kindの文字列だけを理由に汎用fallbackしてはならない。

### DES-378

document / VO / Relation / 判断記録 / 承認記録 / Evidence も §3 のスキーマに対応するstructを定義する。

### DES-S051 5.3 検証グラフ

*導出元: SPEC-S008, SPEC-S049, DS-S009, DS-S048, BD-S008*

### DES-379

検証グラフのノードは `DOC`、`VO`、`TEST`、`SRC`（ロケータ単位）である。

### DES-380

検証グラフのエッジ `DOC → DOC` は `derives_from` であり、documentレコード由来である。

### DES-381

検証グラフのエッジ `VO → DOC` は `derives_from` であり、VOレコード由来、1:N（1件以上）である。

### DES-382

検証グラフのエッジ `VO → VO` は `parent` である。

### DES-383

検証グラフのエッジ `TEST → VO` は `covers` であり、adapter所有のTest metadata宣言由来である。

### DES-384

検証グラフのエッジ `TEST → SRC` は `targets` であり、検証対象をSource Targetとして実現する形態、1:N（`rust-cargo` では `targets ≥ 1`）である（§4.1）。

### DES-385

検証グラフは、`rel/` 由来の外部Relationをエッジとして持つ。

### DES-386

検証グラフは、VO → Tests、SRC → Tests、DOC → VOs、DOC → DOCs（下流）の逆引きインデックスを持つ。

### DES-387

旧モデルのSPEC / REQノードとREQ→SPEC / VO→REQエッジは持たない。

### DES-S052 5.4 整合性診断

*導出元: SPEC-S017, SPEC-S028, DS-S019, DS-S032, DS-S056, BD-S010, BD-S017, BD-S021*

### DES-388

E-SCAN-003が発生しても対応するTest Entityと `ManagedTestLink::One` を除去しない。

### DES-S053 5.5 `rust-cargo` SourceDiscoveryAdapter

*導出元: SPEC-S063, SPEC-S066, DS-S063, BD-S029*

### DES-389

`rust-cargo` adapterは§5.1の `DiscoveryBatch` を構築する。

### DES-390

当該adapterは検証対象をSource Targetとして実現する形態であり、各管理対象Testに1件以上のSource Target（`targets ≥ 1`）を必須とする。

### DES-391

`rust-cargo` discoveryの第1段はファイル探索であり、adapter configのinclude配下の `*.rs` をignoreクレートで列挙する（`.gitignore` 準拠、`target/` は除外）。

### DES-392

`rust-cargo` discoveryの第2段は構文解析であり、ファイルごとに `syn::parse_file` する。解析エラーのファイルはE-SCAN-001を返し、batchをIncompleteとする。

### DES-393

`rust-cargo` discoveryの第3段はモジュールパス構築であり、crateルート（`src/lib.rs` / `src/main.rs` / `tests/*.rs`）からmod宣言を辿り、各itemの完全モジュールパスを構築する。

### DES-394

`rust-cargo` discoveryの第5段はmetadata宣言抽出であり、doc属性（`#[doc = "..."]`）を§4.2の文法でparseする（id / covers / target / intent / input / expect / kind / case / related）。

### DES-395

`rust-cargo` discoveryの第6段はSource Target抽出であり、すべてのfn / impl fnをSRC候補として索引化し、§4.3のlocator解決・逆引き・`@vtest.src-id` 認識（非Test constructの宣言に限る）に使用する（§4.2）。

### DES-396

`rust-cargo` discoveryの第7段はdraft生成であり、全Discovered Test draft、ManagedTestDraftLink、SourceTargetDraft、Source Location、construct / metadata source rangeとbytes、logical metadata、宣言された恒久SRC ID、ExecutionDescriptor、診断をhash未計算のDiscoveryBatchに格納する。

## DES-S054 6. Target Reference解決

### DES-S055 6.1 adapter-neutral解決contract

*導出元: SPEC-S010, SPEC-S028, DS-S011, DS-S032, BD-S009, BD-S017*

### DES-397

adapterは正規化されたTarget Reference、Source Location、source range、content bytes、解決status、候補を返す。

### DES-398

coreは、返却されたadapter IDとTarget Referenceの一致、source rangeの範囲、current bytesとの一致を検証する。

### DES-399

coreは§1.3のSource Target hashを計算する。

### DES-400

SRC ID参照は対応するadapterのSource Locationとsource rangeを使用する。

### DES-S056 6.2 `rust-cargo` locator解決

*導出元: SPEC-S028, DS-S032, BD-S017*

### DES-401

`rust-cargo`のlocator`path::item-path`の解決は、§5.5で構築したSRC索引への完全一致検索とする。

### DES-402

`rust-cargo`のlocator解決は、pathが索引に存在するかを確認する。

> 1. path が索引に存在するか / 2. path 内で item-path が一致する fn / impl fn が存在するか / 3. 一意に決まらない場合（同名 fn が cfg 分岐で複数等）はすべて候補として返し、解決失敗（E-SCAN-004）とする

### DES-403

`rust-cargo`のlocator解決は、path内でitem-pathが一致するfn / impl fnが存在するかを確認する。

### DES-S057 6.3 候補提示

*導出元: SPEC-S038, DS-S039, BD-S019*

### DES-404

`rust-cargo` adapterは、item-pathの末尾セグメント一致（別パスの同名関数）の順で候補を構築する。

> 1. item-path の末尾セグメント一致（別パスの同名関数） / 2. 編集距離 2 以内の近似名 / 出力例： ✗ symbol not found: src/parser.rs::Parser::prase / candidates: src/parser.rs::Parser::parse / src/parser.rs::Parser::parse_inner

### DES-405

`rust-cargo` adapterは、編集距離2以内の近似名の順で候補を構築する。

### DES-406

`rust-cargo` adapterのenum variant検証（`expect`の値が`ParseError::InvalidUtf8`形式の場合）は、スキャン済みASTからenum定義を検索する。

## DES-S058 7. Static Analysis orchestrationと`rust-cargo`ルール

### DES-S059 7.1 判定の原則

*導出元: P-003, SPEC-S021, SPEC-S023, DS-S023, DS-S026, BD-S014*

### DES-407

`vtest-audit`は`TestEntity.execution.adapter`をregistryで解決する。

### DES-408

`vtest-audit`は、Test、全Target Reference、各source range、content hash、および選択adapterの現在configを`StaticAnalysisAdapter`へ渡す。

### DES-409

adapterはrule ID、verdict、根拠span、解析限界を返す。

### DES-410

target-scopedなDA-002 / DA-003については、宣言targetごとのverdictと根拠spanを（規則単位のverdictへ畳み込む前の形で）返す。

### DES-411

target-scopedなDA-002 / DA-003の集合を全宣言targetと過不足なく1対1に対応させる。

### DES-412

coreはadapter ID、source location・現在bytesとの対応、決定論的encodingを検証し、§7.2の規則で集約する。

### DES-S060 7.3 target 到達の静的証明と runtime 証明の関係（target_binding）

*導出元: SPEC-S020, SPEC-S022, SPEC-S027, DS-S022, DS-S031*

### DES-413

`target_binding`項目値は検証時に算出する。

## DES-S061 8. 判断記録プロトコル

*導出元: REQ-S035, SPEC-S031, DS-S035*

### DES-S062 8.1 バンドル生成

### DES-414

バンドルには基本仕様§11.3が定める判断対象の情報一式を含める。

*導出元: SPEC-116, SPEC-117, SPEC-118, SPEC-119, SPEC-120, SPEC-121, SPEC-122, SPEC-333, SPEC-334, SPEC-335*

*引用: 基本仕様 §11.3*

### DES-415

バンドルは対象VO（`--vo` / `--test`から導出したcovers先VOレコードとclaim）を含める。

### DES-416

バンドルはTest Intent（`--test`の場合の対象Testのintent・input・expect）を含める。

### DES-417

バンドルはテストコード（Test construct source全文とmetadata宣言）を含める。

### DES-418

バンドルはTestのcases集合（対象Testが`@vtest.case`で宣言したcaseの正規化文字列を宣言順に並べたlist。§4.1の論理field`cases[]`）を含める。

> 宣言が無いTestでは空listを明示し、項目自体を省略しない。

### DES-419

バンドルは対象実装（全宣言targetのimplementation construct source全文）を含める。

### DES-420

バンドルは関連テスト（related / 同一VOをcoversする他Testのidとintent）を含める。

### DES-421

バンドルは既知partition（対象VOのdimensions・coverage_policy・representative_cases）を含める。

### DES-422

バンドルは過去の判断（同一`(subject, judgment_kind)`への有効・無効な過去判断記録の要約）を含める。

### DES-423

バンドルは対象の内容ハッシュとリビジョン（Test subject / target subject / VO subjectの現在hash、revision）を含める。

### DES-424

バンドルは判断型（`judgment_kind`）をちょうど1件持ち、その値を`judgment_kind`として出力する。

### DES-425

判断型は判断対象を一意に区切るkeyであり（§3.4）、判断記録へ複製される。

### DES-426

`impl-consistency`型の判断（対象実装が宣言と一致するかの意味判定）のように上流documentを要する対象では、対象VOから§3.5と同じ上流依存規則で導出するdocument subject完全集合とsource全文を加える。

### DES-427

`case-coverage`型のバンドルでは、covers先の全leaf / 中間VOの`dimensions`・`coverage_policy`・`representative_cases`と、Testのcases集合を必須項目として含める。

### DES-S063 8.2 バンドル JSON スキーマ（例）

### DES-428

バンドルJSONスキーマは`bundle_id`・`generated_at`・`revision`・`subject`・`judgment_kind`・`test`・`vos`・`targets`・`related_tests`・`static_analysis`・`prior_decisions`のfieldからなる。

> 例: { "bundle_id": "01J8XVYY...", "generated_at": "2026-08-08T00:00:00Z", "revision": { "commit": "abc123...", "dirty": false }, "subject": "TEST-PARSER-044", "judgment_kind": "test-semantic", "test": {...}, "vos": [...], "targets": [...], "related_tests": [...], "static_analysis": {...}, "prior_decisions": [...] }

### DES-S064 8.3 提出スキーマ

### DES-429

`vtest audit submit --file result.json`で提出する。

### DES-430

提出スキーマは`bundle_id`・`subject`・`judgment_kind`・`supersedes`・`decision`・`reason`・`exclusions`・`actor`のfieldからなる。

> 例: { "bundle_id": "01J8XVYY...", "subject": "TEST-PARSER-044", "judgment_kind": "test-semantic", "supersedes": [], "decision": "accepted", "reason": [...], "exclusions": [...], "actor": { "kind": "agent", "id": "judge-agent-01", "model": "claude-fable-5" } }

### DES-431

旧モデルの`verdict → CheckValue`写像（`PASS`/`FAIL`/`COMPLETE`/`INCOMPLETE`を検証状態へ変換する経路）は撤去する。

### DES-S065 8.4 提出の検証

### DES-432

旧モデルのreasons / claim / basis必須検査（E-AUDIT-005）、decomposition-viewpoint検査（E-AUDIT-006）、spec / req basis検査（E-AUDIT-007）は撤去する。

## DES-S066 9. テスト実行設計

*導出元: SPEC-S022, SPEC-S051, DS-S050*

### DES-S067 9.2 `rust-cargo` TestRunnerAdapter

### DES-433

`rust-cargo` adapterは`TestEntity.execution`の`project`をcargo package名として解釈する。

> `rust-cargo` adapterはTestEntity.executionを次のCargo実行座標として解釈する。

### DES-434

`rust-cargo` adapterは`TestEntity.execution`の`suite.kind`を`lib` / `bin` / `integration`として解釈する。

### DES-435

`rust-cargo` adapterは`TestEntity.execution`の`suite.name`をbin名またはintegration test target名として解釈し、`lib`では省略する。

### DES-436

`rust-cargo` adapterは`TestEntity.execution`の`selector`をtest targetのrootからのmodule path＋function名（例：`parser::tests::rejects_invalid_utf8`）として解釈する。

### DES-437

adapter内部ではこれらからCargo launch coordinateを構築する。

### DES-438

`TestEntity`へCargo固有fieldを戻してはならない。

### DES-439

実行は（project, suite）で分けたbatchとし、libtestの`--exact` flagと複数selectorを用いる。

> cargo test -p <project> --lib -- --exact <selector1> <selector2> ...（IntegrationTest の場合は --lib の代わりに --test <name>）

### DES-440

`--exact`は後続の全フィルタへ適用されるフラグであり、各フィルタは完全一致で解釈される。

### DES-S068 9.3 `rust-cargo` 結果のパース

### DES-441

stdoutのパースはstable toolchainの標準出力形式のみに依存する。

### DES-442

`running N tests`という出力は実行対象数の確認を意味する。

### DES-443

`test <selector> ... ok`という出力はPASSを意味する。

### DES-444

`test <selector> ... FAILED`という出力はFAILを意味する。

### DES-445

`test <selector> ... ignored`という出力は実行されずを意味する。

### DES-446

stdout / stderrの全文は`cache/logs/<ULID>.log`へ保存し、Evidenceの`log_ref`から参照する。

### DES-S069 9.4 Evidence の記録

### DES-447

`revision`は実行直前に`git rev-parse HEAD`と`git status --porcelain`で取得する。

### DES-448

`hashes`は、実行直前のdiscovery結果から、Test subject hashと、全宣言targetを§6.1で解決したcanonical Locator・implementation construct hash（§1.3）を宣言順で記録する。

### DES-449

`hashes`は宣言された`TargetRef`の綴りではなく解決後のcanonical Locatorを記録する（§6.1.1）。

### DES-450

部分的な`hashes.targets`を持つEvidenceを生成して後段で弾く方式は採らない。

### DES-451

`execution_state`は、実行直前にrunner adapterが返すsnapshot schema、runner / toolchain / 実行影響config、およびrepository / local dependency入力manifestをcoreが検証し、§1.3のExecution State subject hashとして記録する。

## DES-S070 10. `rust-cargo` Target Binding 動的計測

*導出元: SPEC-S020, SPEC-S051, DS-S022, DS-S050*

### DES-S071 10.1 計測方式

### DES-452

起動時に`cargo llvm-cov --version`で利用可否を確認し、利用不能なら計測しない。

### DES-453

カバレッジをTest単位で対象関数へ帰属させるため、計測時はTestを1件ずつ実行する。

### DES-454

subprocess内の実行を計測するには起動される実行体もinstrument対象とし、子プロセスのprofileをmergeする必要がある。

### DES-455

計測コマンドは`cargo llvm-cov test -p <project> --lib --json --output-path cache/cov/<ULID>.json -- --exact <selector>`である。

### DES-S072 10.2 判定

### DES-456

出力JSON（llvm-cov export形式）の`data[].functions[]`から、Testが宣言する各対象関数を検索する。

### DES-457

一致条件は、demangle済み関数名の末尾がlocatorのitem-pathと一致し、かつfilenamesのいずれかの末尾がlocatorのpathと一致することである。

### DES-458

ジェネリック関数は複数インスタンスが現れるため、同じtargetに対応するcountを合算する。

### DES-459

各targetのcanonical Locator（§6.1.1）・result・countとTest単位集約結果をEvidenceの`target_coverage`へ記録する。

### DES-460

coverage providerは当該境界越しの実行を宣言targetへ帰属させなければならない（例：起動される実行体も計測対象としてinstrumentし、子プロセスのprofileをmergeする）。

## DES-S073 11. 鮮度検証と集約

### DES-S074 11.3 集約アルゴリズム

*導出元: SPEC-S052, DS-S052*

### DES-461

`verify.full_scope`はconfig読込み時に§2.2のinvariantとして検証・正規化済みでなければならない。

### DES-462

aggregateは、scanによりグラフ構築する（§5）。

### DES-463

基本仕様 §22.2がTest単位の結果の集約先として挙げる「Feature単位」は、親VO（`parent`により1件以上の子VOを持つVO。§3.2）を単位として実現する。

*導出元: SPEC-190, SPEC-191*

*引用: 基本仕様 §22.2*

### DES-S075 11.6 役割別 projection

*導出元: REQ-S007, SPEC-S049, SPEC-S054, DS-S048, DS-S054*

### DES-464

親VOを起点とする下流方向のprojectionが、§11.3の機能単位の集約（Feature単位＝親VO）を提示する経路である。

### DES-465

Feature名・Feature IDの別fieldを出力に設けず、束ねの識別子は親VOのIDとする。

### DES-466

逆引きインデックス（VO → Tests、SRC → Tests、DOC → VOs、DOC → DOCs）をprojectionの基盤とする（§5.3）。

### DES-467

projectionが出力する`derives_from`エッジ（DOC → DOC、DOC → VO）には、当該entryの`anchor`（§3.1・§3.2）を常に同伴させる。

### DES-S076 11.7 判断待ち情報の構造

*導出元: SPEC-S047, SPEC-S066*

### DES-468

`subject`は対象エンティティIDまたは解決済みcanonical Locatorとする。

### DES-469

`kind`は`unknown`（UNKNOWNによるエスカレーション）/ `unregistered`（管理宣言欠落）/ `unresolved`（参照解決不能）/ `undecided`（VO未確定）/ `pending_approval`（承認待ち）のいずれかとする。

### DES-470

`check`は関係する検査（4検査のいずれか）と現在の検証状態・診断ラベルとする。

### DES-471

`judgment_kind`は外部判断が必要な場合の判断型（§8.1の値域）とする。

### DES-472

`basis`は機械的に確認済みの事実（宣言鎖・検査結果・対象外とした範囲）への参照とする。

### DES-473

`bundle_ref`は外部判断が必要な場合の判断バンドル（§8.1）への参照（任意）とする。

## DES-S077 16. 並列動作と整合性

### DES-S078 16.1 ロック不要の根拠

*導出元: SPEC-S057, DS-S058, BD-S024*

### DES-474

原子的公開の対象は`.verify/`配下のrecord・エンティティファイル（新規レコード追加とエンティティファイル編集）であり、完全な内容が単一の操作で可視になる方式（同一ファイルシステム内へのtemp書込み＋rename等）で公開し、書きかけ状態・一時ファイル残渣を正典ディレクトリの読み手に観測させてはならない。

## DES-S079 17. 診断・終了コード体系

### DES-S080 17.1 診断コード

*導出元: SPEC-S013, SPEC-S066, DS-S014*

### DES-475

旧モデルの意味監査提出検査（E-AUDIT-005 / E-AUDIT-006 / E-AUDIT-007）は判断記録層への転用（§8.4）に伴い撤去する。

## DES-S081 19. 実装選択と提供範囲

*導出元: R-2, R-3, SPEC-S063, DS-S063, BD-S029*

### DES-476

demangle実装（`rustc-demangle`）の適用範囲は、基本仕様と本詳細設計の観測可能な契約を変更しない範囲で実装が選択できる。

> 次の事項は、基本仕様と本詳細設計の観測可能な契約を変更しない範囲で実装が選択できる。

### DES-477

`#[tokio::test]`等、属性末尾`test`以外のカスタムテスト属性への対応範囲は、基本仕様と本詳細設計の観測可能な契約を変更しない範囲で実装が選択できる。

### DES-478

cargo workspace外の単一クレートプロジェクトでのパス解決の細部は、基本仕様と本詳細設計の観測可能な契約を変更しない範囲で実装が選択できる。

### DES-479

レポートのツリー描画の細部（文字種、折返し）は、基本仕様と本詳細設計の観測可能な契約を変更しない範囲で実装が選択できる。

## DES-S082 12. CLI 詳細仕様

### DES-S083 12.1 共通仕様

*導出元: REQ-S009, SPEC-S012, SPEC-S013, SPEC-S055, SPEC-S078, SPEC-S096, SPEC-S106, DS-S013, DS-S014, DS-S018, DS-S055, DS-S079, DS-S085, DS-S118, DS-S128, BD-S049, BD-S052, BD-S071*

### DES-480

`filter`、`package`、`test_target` は `TestEntity` の field ではない。

### DES-481

coreは `targets ≥ 1` を adapter 中立の必須件数にせず、型としては空 list を許容する。

### DES-S084 12.2 `vtest doc add / list / show`

*導出元: SPEC-S008, SPEC-S009, SPEC-S017, SPEC-S021, SPEC-S030, SPEC-S031, SPEC-S034, SPEC-S035, SPEC-S038, SPEC-S039, SPEC-S042, SPEC-S043, SPEC-S049, SPEC-S051, SPEC-S053, SPEC-S055, SPEC-S061, SPEC-S062, SPEC-S066, SPEC-S069, SPEC-S071, SPEC-S072, SPEC-S073, SPEC-S074, SPEC-S075, SPEC-S076, SPEC-S077, SPEC-S079, SPEC-S081, SPEC-S084, SPEC-S085, SPEC-S087, SPEC-S088, SPEC-S090, SPEC-S092, SPEC-S094, SPEC-S096, SPEC-S097, SPEC-S098, SPEC-S099, SPEC-S100, SPEC-S103, DS-S009, DS-S010, DS-S018, DS-S019, DS-S023, DS-S024, DS-S034, DS-S035, DS-S037, DS-S038, DS-S039, DS-S043, DS-S044, DS-S048, DS-S050, DS-S053, DS-S055, DS-S056, DS-S061, DS-S062, DS-S069, DS-S071, DS-S072, DS-S073, DS-S075, DS-S076, DS-S077, DS-S079, DS-S080, DS-S083, DS-S088, DS-S093, DS-S094, DS-S095, DS-S096, DS-S099, DS-S100, DS-S101, DS-S102, DS-S104, DS-S105, DS-S106, DS-S113, DS-S115, DS-S118, DS-S119, DS-S120, DS-S121, DS-S122, DS-S125, BD-S008, BD-S010, BD-S012, BD-S019, BD-S021, BD-S027, BD-S028, BD-S038, BD-S039, BD-S042, BD-S043, BD-S044, BD-S046, BD-S047, BD-S049, BD-S051, BD-S053, BD-S055, BD-S058, BD-S059, BD-S060, BD-S061, BD-S062, BD-S063, BD-S064, BD-S065, BD-S071, BD-S072, BD-S073, BD-S074*

### DES-482

`doc add` は `--path` の対象ファイルの sha256 を計算して document subject へ束縛した DOC レコードを作成する。

*引用: 本冊 §1.3*

## DES-S085 13. MCP ツール詳細仕様

### DES-S086 13.1 共通仕様

*導出元: SPEC-S062, DS-S062, BD-S028*

### DES-483

各ツール呼び出しの冒頭で mtime ベースの再スキャン判定を行う。

*引用: 本冊 §2.3*

## DES-S087 14. Form Schema 設計

### DES-S088 14.2 検証器

*導出元: SPEC-S041, SPEC-S078, SPEC-S083, DS-S042, DS-S085, DS-S090, DS-S093, BD-S020, BD-S052, BD-S057*

### DES-484

`symbol-exists` 検証器は Target Reference 解決を対応 adapter へ委譲し、失敗時は E-OP-001＋候補を返す。

*引用: 本冊 §6.1*

### DES-485

`vo-exists` / `test-exists` 検証器はエンティティ存在確認を行い、失敗時は E-OP-001 を返す。

### DES-486

`enum-variant-exists` 検証器は `rust-cargo` adapter が `Type::Variant` 形式の場合のみ AST 検索し、解決不能な自由記述は受理し、失敗時は E-OP-001＋候補を返す。

### DES-487

`unique-fn-name` 検証器は `rust-cargo` adapter が挿入先モジュール内で関数名重複を確認し、失敗時は E-OP-001 を返す。

### DES-488

`rust-file` 検証器は `rust-cargo` adapter が `.rs` ファイルが scan 対象内に存在することを確認し、失敗時は E-OP-001 を返す。

### DES-489

registry は built-in と user-defined Form を統合し、同じ kind の重複、schema の adapter と registry owner の不一致、未知 adapter、Structured Test capability 欠落を拒否する。

### DES-490

`adapter` を欠く読取り互換 Form は、登録済み adapter の built-in kind 宣言または schema を検査する compatibility matcher のうちちょうど1件だけが受理する場合に限って解決し、0件または複数件なら拒否する。

### DES-491

matcher は schema 内容から決定論的に判定し、kind 名だけで Rust 用と推測しない。

### DES-492

reader は互換解決だけで Form ファイルを書き換えない。

### DES-S089 14.3 組込フォーム

*導出元: SPEC-S041, SPEC-S063, DS-S042, DS-S063, DS-S080, DS-S081, BD-S020, BD-S029*

### DES-493

コアはフォームの kind を Rust 固有と推測せず、schema の `adapter`、registry が宣言する大局的に一意な kind ownership、登録済み capability を照合する。

## DES-S090 15. Structured Test Operation adapter contract

### DES-S091 15.1 `rust-cargo` 対象の特定

*導出元: SPEC-S040, DS-S041*

### DES-494

Test ID から編集対象を特定する。

> ```text
> TEST-X → スキャン結果 → SourceLocation
>   （ファイル、関数アイテムの byte range、
>     doc comment 開始位置を含む拡張 range）
> ```

### DES-495

スキャン結果が古い可能性があるため、編集直前に対象ファイルのみ再パースし、Test ID の位置を再確認する。

### DES-S092 15.2 `rust-cargo` 編集・挿入の適用

*導出元: SPEC-S039, SPEC-S040, DS-S041*

### DES-496

desired state（answers / set / body）から、あるべきアノテーションブロックと関数シグネチャ・本体を生成する。

### DES-497

現状とあるべき状態の diff を計算する。

### DES-498

変更を、対象テスト関数の拡張 range（doc comment 先頭〜関数末尾）の単一置換として適用する。

### DES-499

適用後の対象ファイルの再パースは、構文的に妥当であることを確認する。

### DES-500

適用後の対象ファイルの再パースは、対象 Test のアノテーションが desired state と一致することを確認する。

### DES-501

適用後の対象ファイルの再パースは、他の Test エンティティのソーステキストが変化していないことを確認する。

### DES-502

確認に失敗した場合はファイルを元へ戻し、E-OP-003 を返す。

### DES-503

挿入後の再パース検証とロールバックは Edit と同一の規則で Create にも適用する。

### DES-504

Create 経路にだけ検証を省く分岐を設けない。

*導出元: SPEC-138, SPEC-139*

*引用: 基本仕様 §15.1*

### DES-505

Form 回答（§14）から、あるべきアノテーションブロックと関数シグネチャ・本体、および挿入位置を決定する。

### DES-506

回答自体の検証エラーは E-OP-001（候補付き）とする。

*引用: 本冊 §6.3*

### DES-507

挿入前の対象ファイルの内容を保持する。

### DES-508

対象ファイルが存在しない場合は「不存在」を挿入前の状態として保持する。

### DES-509

生成した Test construct を挿入位置へ単一挿入として適用する。

### DES-510

適用後の対象ファイルの再パースは、構文的に妥当であることを確認する。

### DES-511

適用後の対象ファイルの再パースは、挿入した Test construct がちょうど 1 件の Test エンティティとして認識されることを確認する。

### DES-512

適用後の対象ファイルの再パースは、その Test のアノテーションが Form 回答から導いた desired state と一致し、Test ID が回答どおりであることを確認する。

### DES-513

適用後の対象ファイルの再パースは、挿入した Test 以外の Test エンティティのソーステキストが変化していないことを確認する。

### DES-515

確認のいずれかに失敗した場合は、適用前の状態へ復元し（挿入によりファイルが新規作成された場合は不存在へ戻す）、E-OP-003 を返す。

### DES-516

ロールバック後は、当該操作より前と同じソーステキストが観測できなければならない。

### DES-517

部分適用された挿入内容を残さない。

### DES-518

`--dry-run` は、Form 回答（§14）から決定したあるべきアノテーションブロックと関数シグネチャ・本体、および挿入位置の結果のみを提示し、ファイルを変更しない。

### DES-519

Create / Edit いずれも、E-OP-003 で中止した操作は Test ID の採番・Evidence・判断記録を含む副産物をひとつも残さない。

### DES-520

ロールバック後の再スキャンで、当該操作が無かった場合と同一のエンティティ集合・内容ハッシュが得られる。

### DES-S093 15.3 `rust-cargo` annotation blockの再生成

*導出元: SPEC-S040, DS-S041, DS-S080*

### DES-521

アノテーションは常にキー順（id, covers, target, intent, input, expect, kind, case, related）で再生成する。

### DES-522

`@vtest.` を含まない自由記述の doc comment 行は元の位置関係を保って温存する。

### DES-523

アノテーションを常にキー順（id, covers, target, intent, input, expect, kind, case, related）で再生成し、`@vtest.` を含まない自由記述の doc comment 行を元の位置関係を保って温存することにより、Structured Edit を繰り返しても差分が安定する。

### DES-524

アノテーションを常にキー順（id, covers, target, intent, input, expect, kind, case, related）で再生成する規則は、Create が挿入する annotation block にも同一に適用する。

### DES-525

同一の desired state からは Create / Edit のいずれの経路でも同一の annotation block を生成し、Create 直後に同じ desired state で Edit しても差分を生じない。

### DES-526

アノテーションの再生成キー順（id, covers, target, intent, input, expect, kind, case, related）は本冊 §4.2 の test-key（`id` / `covers` / `target` / `intent` / `input` / `expect` / `kind` / `case` / `related`）と一致する。

*引用: 本冊 §4.2*

### DES-527

本 version は存在理由分類（旧 `role` / `anchor` / `anchor-rationale`）のキーを持たず、再生成でも出力しない。

### DES-528

`@vtest.src-id` は Test construct ではなく対象実装側の関数に付与するキーである（本冊 §4.2 の source-target-key）。

*引用: 本冊 §4.2*

### DES-529

`@vtest.src-id` は Test annotation block の再生成対象に含めない。

### DES-S094 15.4 `rust-cargo` 1 Test境界の保証

*導出元: REQ-S039, SPEC-S040, DS-S041*

### DES-530

置換範囲が単一のテスト関数の拡張 range に限られることを、適用前（範囲計算）と適用後（他 Test のハッシュ不変確認）の二重で検査する。

### DES-531

`edit TEST-001` は他のTestへ影響しない。

*導出元: REQ-210, REQ-211, REQ-212, REQ-213, REQ-214, REQ-215, REQ-216, REQ-217, SPEC-140, SPEC-141, SPEC-142, SPEC-143, SPEC-144*

*引用: 要件定義 §16, 基本仕様 §15.3*

## DES-S095 18. 受入契約

### DES-S096 18.1 共通条件

*導出元: SPEC-S053, SPEC-S056, DS-S018, DS-S025, DS-S053, DS-S057, BD-S013, BD-S022*

### DES-532

受入条件は決定論的なfixtureと統合テストで再現できる。

### DES-533

Rust workspaceの受入テストは`cargo test --workspace`で実行できる。

### DES-534

canonical record、承認記録、判断記録、Evidence、内容hashの不変条件をfixtureの都合で緩和しない。

### DES-S097 18.3 機能別受入条件

#### DES-S098 18.3.1 discovery・record・graph と chain_integrity

*導出元: SPEC-S009, SPEC-S018, SPEC-S028, SPEC-S043, SPEC-S077, SPEC-S083, SPEC-S095, DS-S010, DS-S020, DS-S032, DS-S044, DS-S056, DS-S083, DS-S090, DS-S116, BD-S017, BD-S021, BD-S051, BD-S057*

### DES-535

source discovery adapterは全Discovered Test draft、ManagedTestDraftLink、SourceTargetDraft、Source Location、source range、current bytes、logical metadata、宣言された恒久SRC ID、Test execution descriptorをhash未計算で返す。

### DES-536

coreは出力を検証してTest subject / Source Target hashを計算してからManaged Test Entity、ManagedTestLink、Source Targetを具体化する。

### DES-537

Source Targetはcanonical locatorと任意の恒久SRC IDを併有する単一のentityである。

### DES-538

adapterは同一constructをlocator版とSrcId版の2 draftへ複製せず、恒久SRC IDを`SourceTargetDraft.src_id`として返す。

### DES-539

`SourceTargetDraft.target`は必ず`TargetRef::Locator`である。

### DES-540

SRC ID参照はcoreの統合済みSRC索引から、その恒久SRC IDを宣言したSource Targetのcanonical locatorへ解決する。

### DES-541

Relation writerは`REL-<ULID>`だけを生成する。

### DES-542

readerはファイル名とrecord IDが同じbare ULIDのversion 1互換Relationを読み取り、in-memoryで`REL-<ULID>`へ正規化するが、ファイルを書き換えない。

### DES-543

VO writerは`status`を保存せず、実効値をApprovalから導出する。

#### DES-S099 18.3.4 execution・Evidence（target_binding の証拠）

*導出元: SPEC-S051, SPEC-S076, DS-S025, DS-S050, DS-S051, DS-S077, DS-S105, DS-S117, BD-S013, BD-S047, BD-S064*

### DES-544

Evidence writerは中立fieldの`hashes.test_subject`と`hashes.targets[].target_construct`を出力する。

#### DES-S100 18.3.5 target_binding 動的計測（per-target）

*導出元: SPEC-S020, SPEC-S051, SPEC-S086, DS-S022, DS-S050, DS-S098, DS-S112, BD-S069*

### DES-545

旧モデルの`target_execution`検査項目は撤去し、計測事実だけをEvidenceの`target_coverage` fieldとして保持して`target_binding`の証拠源へ吸収する。

*引用: 本冊 §3.6・§10*

#### DES-S101 18.3.7 承認と判断記録の分離

*導出元: SPEC-S016, SPEC-S034, SPEC-S043, SPEC-S074, SPEC-S075, DS-S017, DS-S037, DS-S044, DS-S075, DS-S076, BD-S046*

### DES-546

実効承認の導出は `approved_state` を参照する。

### DES-547

承認主体は種別（`human` / `agent`）と識別子を記録する。

#### DES-S102 18.3.8 verify・report と scope

*導出元: SPEC-S017, SPEC-S053, SPEC-S054, SPEC-S055, SPEC-S069, SPEC-S094, SPEC-S096, DS-S018, DS-S019, DS-S053, DS-S054, DS-S055, DS-S069, DS-S115, DS-S118, BD-S010, BD-S039, BD-S071*

### DES-548

`verify` / `report` の JSON（CLI・MCP）は最上位に `scope` を返し、`scope.requested.items`（`--items` 省略時は固定4検査を4件すべて列挙）、`scope.requested.entities`（エンティティ軸無指定は空 list）、`scope.unverified_outside_scope`（検査軸4件未満またはエンティティ軸指定ありで `true`、完全検証で `false`）を持つ。

### DES-549

text treeのancestor continuation、middle child、last childを一意なbranch記号で描画する。

### DES-550

評価経路へそのような seam を導入する変更を行う場合は、正反対の判定を返す stub を注入しても 4 検査の結果が変化しないことを受入で確認する。

#### DES-S103 18.3.9 フェーズゲート評価

*導出元: REQ-S057, SPEC-S012, SPEC-S016, SPEC-S050, SPEC-S069, SPEC-S098, SPEC-S105, SPEC-S106, DS-S013, DS-S017, DS-S049, DS-S069, DS-S120, DS-S127, DS-S128, BD-S039, BD-S072*

### DES-551

ゲート定義は `config.yaml` の `gates` に、ゲート名と進行条件（`require.verification` ＝要求する検証結果、`require.approvals` ＝要求する承認ロール集合）として保持する。

### DES-552

条件充足・不足の両方を fixture で確認する。

### DES-553

順序・包含解釈による充足を認めない fixture を持つ。

### DES-554

`--gate` を指定した `verify` / `report` の JSON は `data.gate` に `name`・`verification.{required, actual, satisfied}`・`approvals[].{role, satisfied, missing_subjects}`・`satisfied` を返す。

#### DES-S104 18.3.10 Structured Test Operation / Create の挿入後検証とロールバック

*導出元: SPEC-S038, SPEC-S078, DS-S039, DS-S085, BD-S019, BD-S052*

### DES-555

create は挿入後に対象ファイルを再パースし、構文妥当性・挿入分がちょうど 1 Test として認識されること・その Test ID と annotation が desired state と一致すること・他の Test と通常 source が不変であることを確認する。

*導出元: SPEC-138, SPEC-139*

*引用: 別紙A §15.2, 基本仕様 §15.1*

#### DES-S105 18.3.12 adapter contract

*導出元: SPEC-S063, SPEC-S078, SPEC-S105, DS-S007, DS-S063, DS-S085, DS-S127, BD-S029, BD-S052*

### DES-556

`vtest-adapter-api`は言語・runner非依存であり、Cargo、Rust parser、llvm-cov固有型を公開しない。

### DES-557

`vtest-model::TestEntity`はTestを関数として表現せず、adapter所有のTest constructを論理metadata、Source Location、content hash、ExecutionDescriptorで表現する。

### DES-558

`TargetRef::Locator`はadapter IDとadapter所有のopaque locatorを保持する。

### DES-559

`SourceLocation`はadapter ID、project-relative path、opaque locator、source rangeを保持する。

### DES-560

`TargetRef::Locator`と`SourceLocation`のどちらもRust module path、関数名、`.rs`拡張子をcoreの不変条件にしない。

### DES-561

`vtest-model::TestEntity`は`ExecutionDescriptor`だけを実行座標として持ち、`filter`、`package`、`test_target`、`TestTarget`を含まない。

### DES-562

`SourceDiscoveryAdapter`はhash未計算DTOを返し、coreがDTO検証・hash計算・domain entity具体化をこの順で行う。

### DES-563

Rustとsyntheticの結果をadapter ID、path、Test IDで決定論的に統合する。
