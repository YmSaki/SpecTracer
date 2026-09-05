<!-- generated from docs/canonical/specification.json by build.py; do not edit -->

# 詳細設計

## DES-S001 2 全体像

### DES-S002 2.4 adapter 設定と wire 互換

*導出元: REQ-S048, REQ-S058*

### DES-001

core domainの `TestEntity` は、言語・runner非依存の `execution`（adapter・project・suite・opaque selector）だけを実行座標として持つ。

### DES-002

`filter` / `package` / `test_target` は `TestEntity` のfieldではない。

## DES-S003 6. 証拠

*導出元: REQ-S019*

### DES-003

adapterはsource range・source bytes・解析した論理metadata・実行座標をhash未計算のdiscovery DTOとして返す。

### DES-004

coreが言語非依存の正規化規則でsubject hashを計算してからTest Entityを具体化する。

## DES-S004 1. 実装構成

### DES-S005 1.2 主要依存クレート

*導出元: SPEC-S070, DS-S051, BD-S033*

### DES-005

Git 操作（HEAD の取得、dirty 判定）は `git` CLI の呼び出しで行う（`git rev-parse HEAD`、`git status --porcelain`）。

### DES-S006 1.3 内容ハッシュの定義

*導出元: SPEC-S002, SPEC-S008, SPEC-S025, SPEC-S031, SPEC-S032, SPEC-S056, DS-S003, DS-S016, DS-S021, DS-S022, DS-S038, BD-S005, BD-S012, BD-S016, BD-S023*

### DES-006

内容ハッシュは `sha256:<hex>` 形式で記録する。

### DES-007

subject固有規則でbyte-exactを要求しないテキストfragmentは、改行をLFへ統一する。

### DES-008

subject固有規則でbyte-exactを要求しないテキストfragmentは、各行の末尾空白を除去する。

### DES-009

改行の統一と各行末尾空白の除去以外の空白は正規化しない。

### DES-010

hash inputはdomain separatorと長さ付きfieldから構成する。

### DES-011

hash inputの各fieldは `field-name`、UTF-8 byte length、byte列の順にencodeする。

### DES-012

hash inputのencodeは単純な文字列連結を行わない。

### DES-013

hash inputにおいて、mapはkey昇順でencodeする。

### DES-014

hash inputにおいて、集合として扱う `covers`・`targets`・`related` は正規化値の昇順でencodeする。

### DES-015

hash inputにおいて、順序に意味がある `cases` は宣言順でencodeする。

### DES-016

hash inputにおいて、null、空文字、空listは異なる値としてencodeする。

### DES-017

Test subject hashはdomain `vtest:test-subject:v1` を用い、adapter ID、Test ID、全canonical metadata、Source Locationのadapter・project-relative path・opaque locator、ExecutionDescriptor、および正規化したTest construct bytesを束縛する。

### DES-018

Test subject hashは、byte range自体を前方の無関係な編集で変化するためhash inputにしない。

### DES-019

metadata宣言がmanifest等の非隣接箇所に存在しても、Test subject hashはadapterが返す論理metadataを同じsubjectへ含める。

### DES-020

canonical metadataの `targets` は宣言された `TargetRef` の正規化値を束縛し、解決後のcanonical Locatorへ置換しない。

### DES-021

canonical metadataは `id` / `covers` / `targets` / `intent` / `input` / `expect` / `kind` / `cases` / `related` からなる。

### DES-022

canonical metadataは、`role` / `anchor` 等の存在理由分類 fieldを本versionでは持たない（§4.1）。

### DES-023

Test subject hashは、宣言の不在と空値の明示を異なる値としてencodeする。

### DES-024

Source Target hashはdomain `vtest:target-subject:v1` を用い、canonical Target Referenceとadapterが返すimplementation construct bytesを束縛する。

### DES-025

construct bytesへの束縛は当該実現形態（`rust-cargo` 等のSource Target形態）に対する規則であり、Source Targetを宣言しない検証対象形態へは適用しない。

### DES-026

canonical Target Referenceは常に `TargetRef::Locator`（adapter IDとadapter所有のopaque locator）であり、`TargetRef::SrcId` をcanonical Target Referenceにしない。

### DES-027

`TargetRef::SrcId` はSource Targetを参照する側の表現であって、Source Target自身の識別ではない。

### DES-028

恒久SRC IDはhash inputの独立fieldとして束縛せず、canonical Target Reference経由でもhash inputへ入らない。

### DES-029

恒久SRC IDの宣言・変更・削除はcanonical Target Referenceを変えない。

### DES-030

恒久SRC IDの宣言をSource Targetのconstruct bytesの内側へ置くadapter（`rust-cargo` の `@vtest.src-id` doc comment等）では、その宣言の追加・変更・削除がconstruct bytesを変化させ、construct bytes経由でSource Target hashが変化しうる（§5.5）。

### DES-031

恒久SRC IDの宣言をSource Targetのconstruct bytesの内側へ置くadapterで、その宣言の追加・変更・削除がconstruct bytesを変化させ、construct bytes経由でSource Target hashが変化しうることは正しい挙動であり、恒久SRC IDが独立したhash fieldであることを意味しない。

### DES-032

Source Target hashはSource Target自身のcanonical Locatorから一度だけ計算し、当該Source Targetを参照するTest側の `TargetRef` 綴りからは計算しない。

### DES-033

Evidence、検証は解決後のcanonical Source Targetのcanonical Locatorとhashへ束縛し、addressing modeごとに別subjectを作らない（§6.1）。

### DES-034

document subject hashはdomain `vtest:document-subject:v1` を用い、canonical document recordと参照先source（`path` の実ファイル）の正規化内容を束縛する。

### DES-035

document subject hashは、要件定義・基本仕様・詳細設計・API Schema等を種別で区別せず、すべて同一の総称document subjectとして計算する（§3.1）。

*導出元: SPEC-107, SPEC-108, SPEC-109, SPEC-110, SPEC-111, SPEC-112, SPEC-439, SPEC-440*

*引用: 基本仕様 §3.2*

### DES-036

VO subject hashはdomain `vtest:record-subject:v1` を用い、readerが具体化したcanonical VO recordをfield規則に従ってencodeする。

### DES-037

VO subject hashは、VOの読取り互換field `status` を正典ではないため含めない。

### DES-038

VO subject hashは `derives_from`（参照先document ID集合）と `parent` を束縛する。

### DES-039

VO subject hashは、`covers` の増減をTest側subjectで捕捉するため含めない。

### DES-040

Execution State subject hashはdomain `vtest:execution-state:v1` を用い、adapter ID、snapshot schema ID / version、HEAD revision、runner kindとcanonical invocation projection、toolchain identity、実行結果へ影響するadapter configのcanonical projection、および実行可能状態を変えうるrepository / local dependency入力の完全なmanifestを束縛する。

### DES-041

manifest entryはstable root identity、root-relative path、input kind、byte-exact file bytesからなる。

### DES-042

manifest entry集合は正規化identity順にencodeする。

### DES-043

stable root identityはmachine上の絶対pathを用いず、workspace内の論理rootまたはdependency identityから決定論的に導出する。

### DES-044

adapterはhash未計算のmanifestと完全性を返し、coreが各entryとsubject全体を検証・hash化する。

### DES-045

adapterはsource location、source rangeと現在のbytes、解析済みlogical metadata、ExecutionDescriptorをhash未計算のdiscovery DTOとして返す。

### DES-046

coreはadapter出力と現在のsource bytesの対応を検証し、言語非依存encodingとSHA-256計算を行ってからdomain entityを具体化する。

### DES-047

Test Runner adapterも、実行状態へ用いたconfig / manifestをhash未計算DTOとして返す。

### DES-048

coreは現在bytesとの対応、重複、集合完全性、schema versionを検証してsubject hashを計算する。

### DES-049

adapterは、完全性を保証できないDTOから `PASS` 用subjectを具体化してはならない。

### DES-050

静的解析結果は内容ハッシュに束縛された永続subjectを持たず、hash体系に静的解析専用のsubjectを設けない。

### DES-051

`rust-cargo` adapterはTest constructとして、metadata doc commentを除き、実行に影響する属性、signature、bodyを含む関数itemのbytesを返す。

### DES-052

doc comment由来metadataはlogical metadataと `metadata_sources` として別に返す。

### DES-053

`rust-cargo` adapterはSource Targetには属性とdoc commentを含む関数item全体を返す。

## DES-S007 2. データディレクトリと設定

### DES-S008 2.2 `config.yaml`

*導出元: SPEC-S007, SPEC-S013, SPEC-S018, SPEC-S021, SPEC-S055, SPEC-S059, DS-S002, DS-S006, DS-S009, DS-S012, DS-S041, BD-S004, BD-S010*

### DES-054

`config.yaml` readerはversion 1を単一の `rust-cargo` adapter設定としてin-memory変換して読み取るが、読み取りだけで正典を書き換えない。

*導出元: SPEC-091*

*引用: 基本仕様 §2.4*

### DES-055

`gates` はフェーズゲートの進行条件定義を保持する（§11.5）。

### DES-056

`doc.roots` は orphan_detection の除外根をDOC IDの集合として保持する（§5.6）。

### DES-S009 2.3 派生情報

*導出元: P-003, SPEC-S065, DS-S047, BD-S029*

### DES-057

MCPサーバは長時間動作するため、ツール呼び出しごとに対象ファイルのmtimeを確認し、変化があれば再スキャンする。

## DES-S010 3. レコードファイルスキーマ

### DES-S011 3.1 document レコード（`.verify/doc/DOC-*.yaml`）

*導出元: SPEC-S009, SPEC-S010, SPEC-S049, DS-S004, DS-S035, BD-S006, BD-S007, BD-S021*

### DES-058

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

### DES-059

document レコードの `content_hash` fieldは登録時の内容ハッシュである（§1.3）。

### DES-060

`anchor` は `derives_from` entryのfieldであり、Test metadataには存在しない（§4.1）。

*導出元: SPEC-236, SPEC-237, SPEC-238, SPEC-239, SPEC-543, SPEC-544, SPEC-545, SPEC-546, SPEC-547, SPEC-548, SPEC-549, SPEC-550, SPEC-551, SPEC-552, SPEC-553, SPEC-554, SPEC-555, SPEC-556, SPEC-557, SPEC-558, SPEC-559, SPEC-560, SPEC-561, SPEC-562, SPEC-563, SPEC-564, SPEC-565, SPEC-566, SPEC-567, SPEC-568, SPEC-569, SPEC-570*

*引用: 基本仕様 §12*

### DES-061

`anchor` はcanonical document recordの一部であり、document subject hashの入力に含まれる（§1.3）。

### DES-S012 3.2 VO レコード（`.verify/vo/VO-*.yaml`）

*導出元: SPEC-S010, SPEC-S035, DS-S004, DS-S025, BD-S007*

### DES-062

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

### DES-063

VO レコードの `coverage_policy` fieldの値域は `independent-axes` / `full-product` / `explicit` / `null` である。

### DES-064

VO レコードの `combinations` fieldは `coverage_policy: explicit` のとき実体化する組合せである（§3.2.1）。

### DES-065

`anchor` と `note` はVO subject hashの入力に含まれない（VO subject hashは `derives_from` の参照先document ID集合を束縛する）（§1.3）。

### DES-066

VOの `status` は承認レコードから導出する表示値であり、canonical writerはVO recordへ保存しない。

#### DES-S013 3.2.1 dimensions と組合せの実体化

*導出元: SPEC-S035, DS-S025*

### DES-067

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

*導出元: SPEC-201, SPEC-202, SPEC-203, SPEC-204, SPEC-205, SPEC-206, SPEC-207, SPEC-208, SPEC-209, SPEC-210, SPEC-211, SPEC-212, SPEC-213, SPEC-214, SPEC-491*

*引用: 基本仕様 §10*

### DES-068

`combinations` はcanonical VO recordの一部であり、VO subject hashに束縛される（§1.3）。

### DES-S014 3.3 Relation レコード（`.verify/rel/REL-<ULID>.yaml`）

*導出元: SPEC-S006, SPEC-S010, DS-S004, BD-S007*

### DES-069

Relationレコードの `type` fieldの値域は `depends-on` / `supersedes` / `regression-for` / `derived-from` / `same-partition` / `complements` / `conflicts-with` である。

> id: REL-01J8XVZK3Q...
> type: depends-on          # depends-on | supersedes | regression-for |
>                           # derived-from | same-partition | complements | conflicts-with
> from: TEST-PARSER-044     # 任意のエンティティID
> to: TEST-PARSER-012
> note: ""                  # 任意の説明文
> created: 2026-08-08T00:00:00Z

### DES-070

writerは `.verify/rel/REL-<ULID>.yaml` と同値の `id` だけを生成する。

### DES-071

readerはversion 1互換入力として `.verify/rel/<ULID>.yaml` かつ同値のbare `id` を受理し、`REL-<ULID>` へin-memoryで正規化するが、読み取りだけでファイルを書き換えない。

### DES-072

Relationは不変である。

### DES-073

Relationの変更はファイル削除＋新規作成で表す。

### DES-S015 3.4 判断記録レコード（`.verify/decisions/<ULID>.yaml`）

*導出元: REQ-S035, SPEC-S039, DS-S027*

### DES-074

判断記録は依存closureのハッシュに束縛される。

### DES-075

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

### DES-076

判断記録の `subject_hash` fieldは判断時点の対象の内容ハッシュである。

### DES-077

判断記録の `dependencies` entryの `hash` fieldはdocument subject hashである（§1.3）。

### DES-078

判断記録の `actor` の `kind` fieldの値域は `human` / `agent` である。

### DES-079

`judgment_kind` は判断対象を一意に区切る第二のkeyである。

### DES-080

`supersedes` は、この判断記録が明示に置き換える旧判断記録のULIDを名指しするlistである。

### DES-S016 3.5 承認レコード（`.verify/approvals/<ULID>.yaml`）

*導出元: REQ-S046, SPEC-S017, SPEC-S048, SPEC-S073, DS-S008, DS-S034*

### DES-081

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

### DES-082

承認レコードの `judgment_ref` fieldは参照する判断記録IDであり、任意（judgment reference）とする。

### DES-083

承認レコードの `supersedes` fieldは明示に置き換える旧承認レコードのULID listであり、既定は空listとする。

### DES-084

承認レコードの `subject_hash` fieldは承認時点の対象の内容ハッシュである。

### DES-085

承認レコードの `dependencies` fieldは承認時点の上流依存closureであり、完全一致を要求する。

### DES-086

承認レコードの `dependencies` entryの `hash` fieldはdocument subject hashである（§1.3）。

### DES-087

承認レコードの `approver` の `kind` fieldの値域は `human` / `agent` である。

### DES-088

承認レコードの `approver` の `model` fieldはagentの場合任意とする。

### DES-089

承認レコードの `approved_state` fieldはどの承認状態かを表し、必須とする。値域は `approved` / `rejected` / `withdrawn` である。

### DES-090

承認レコードの `basis` fieldは根拠であり、任意とする。

### DES-091

承認記録は判断記録と同一entityであることを要求しない（別entityでありうる）（§3.4）。

### DES-092

承認は対象自身（`subject`）または参照する判断（`judgment_ref`）に承認済み状態を与える。

### DES-093

承認対象は `vo`・`document`・`judgment` の3種であり、レコード上の表現は種別ごとに定まる。

### DES-094

承認対象種別 `vo` は、レコード上 `subject` にVO ID（`VO-*`）で表現し、上流依存closureは対象VOの再帰的なparent VO、対象VOと各parent VOが `derives_from` で参照するdocument、および各documentの再帰的な上位document（`derives_from` 先）である。

### DES-095

承認対象種別 `document` は、レコード上 `subject` にdocument ID（`DOC-*`）で表現し、上流依存closureは対象documentの再帰的な上位document（`derives_from` 先）である。

### DES-096

承認対象種別 `document` は、総称documentとして登録した文書で表現し、専用のエンティティ型を設けない（§3.1）。

*導出元: SPEC-092, SPEC-093, SPEC-094, SPEC-095, SPEC-096, SPEC-097, SPEC-098, SPEC-099, SPEC-100, SPEC-101, SPEC-102, SPEC-103, SPEC-104, SPEC-105, SPEC-106, SPEC-437, SPEC-438*

*引用: 基本仕様 §3.1*

### DES-097

承認対象種別 `judgment` は、レコード上 `judgment_ref` に判断記録ULIDを置き、`subject` には当該判断記録の `subject`（VO IDまたはdocument ID）を置き、上流依存closureは `subject` の種別に応じた同表のclosureとする。

### DES-098

判断記録ULIDは `subject` に置かない。判断記録の承認は `judgment_ref` によってのみ表す。

### DES-099

`judgment_ref` が指す判断記録が存在しない場合は、書込み時にE-APPROVAL-001として拒否する。既存レコードとして読み取った場合は当該レコードからVO / documentの実効承認も判断記録の実効承認も導出しない（W-STORE-006）。

### DES-100

判断記録ULIDを `subject` に持つ承認レコードは、書込み時にE-APPROVAL-002として拒否する。既存レコードとして読み取った場合は履歴表示だけを許可していかなる実効承認も導出せず、W-STORE-006を出す。

### DES-101

VO ID・document IDのいずれにも解決しない `subject`（Test ID、Source Target locator、Relation ID等）も、判断記録ULIDを `subject` に持つ承認レコードと同じ扱いとする。

### DES-102

いずれの種別でも対象自身は `subject_hash` で束縛するため `dependencies` へ重複して含めない。

### DES-103

`dependencies` のentryは `kind`（`vo` | `document`）、`id` の順でsortし、欠落・重複・余剰entryを許可しない。

### DES-104

`approved_state` は `approved` / `rejected` / `withdrawn` の3値だけを受理する。

### DES-105

`approved_state` が値域外の他の値の場合、書込み時にE-APPROVAL-002として拒否する。既存レコードとして読み取った場合は履歴表示だけを許可していかなる実効承認も導出せず、W-STORE-006を出す。

### DES-106

承認レコードの `supersedes` は、この承認レコードが明示に置き換える旧承認レコードのULIDを名指しするlistである。

### DES-107

`supersedes` に列挙する各ULIDは、同一 `subject`（`judgment_ref` を持つ承認では同一 `judgment_ref`）の既存承認レコードを指さなければならない。

### DES-108

参照先を解決できない、対象が一致しない、または自己参照する `supersedes` entryを含むレコードは、書込み時にE-APPROVAL-002として拒否する。

### DES-109

既存レコードとして読み取った場合、およびsupersede関係が循環する場合は、当該レコードを実効集合へ寄与させずW-STORE-005を出す。

### DES-110

承認レコードの `supersedes` はRelationとは独立であり、`type: supersedes` のRelationレコードは実効承認の決定に用いない（§3.3）。

### DES-111

実効承認は対象X（VO ID、document ID、または `judgment_ref` が指す判断記録）に対して所定の順に評価して導出する。

### DES-112

`approved_state` を参照せずに承認済みを導出してはならない。

### DES-113

承認レコード a が A(X) に属するのは、`a.approved_state` が値域内であること、a の対象指定が X と一致すること（X が VO / document のとき `a.subject == X`、X が判断記録のとき `a.judgment_ref == X` の ULID）、`a.subject_hash` が `a.subject` の現在の内容ハッシュと一致すること、`a.dependencies` が `a.subject` の現在の上流依存closureと entity・hash とも完全一致すること、`a.dependencies` の各 document が登録 content_hash と実ファイルの一致を満たすこと（§11.4）、および X が判断記録のとき当該判断記録が §8.5 の有効判断でありかつ §8.5 の実効集合 E に属すること、をすべて満たす場合だけとする。

### DES-114

承認レコードaがA(X)に属するには、`a.approved_state` が値域内でなければならない。

### DES-115

承認レコードaがA(X)に属するには、aの対象指定がXと一致しなければならない（XがVO / documentのとき `a.subject == X`、Xが判断記録のとき `a.judgment_ref == X` のULID）。

### DES-116

承認レコードaがA(X)に属するには、`a.subject_hash` が `a.subject` の現在の内容ハッシュと一致しなければならない。

### DES-117

承認レコードaがA(X)に属するには、`a.dependencies` が `a.subject` の現在の上流依存closureとentity・hashとも完全一致しなければならない。

### DES-118

承認レコードaがA(X)に属するには、`a.dependencies` の各documentが登録content_hashと実ファイルの一致を満たさなければならない（§11.4）。

### DES-119

Xが判断記録のとき、承認レコードaがA(X)に属するには、さらに当該判断記録が §8.5 の有効判断であり、かつ §8.5 の実効集合Eに属さなければならない。

### DES-120

実効集合 A’(X) は、A(X) から、A(X)内の他レコードの `supersedes` に名指しされたものを除いた集合である。

### DES-121

A’(X) が空の場合、実効承認状態は `draft` とする。

### DES-122

A’(X) に `approved_state` が `rejected` または `withdrawn` のレコードが1件以上ある場合、実効承認状態は `draft` とする（fail-closed。機械はどちらかを選ばない）。

### DES-123

A’(X) の全レコードが `approved_state == approved` の場合、実効承認状態は `approved` とする。

### DES-124

実効集合 A’(X) からの除外は `supersedes` による明示の名指しだけで起きる。

### DES-125

`supersedes` 関係にない複数の有効承認レコードはすべてA’(X)に属する。

### DES-126

同一対象に `approved` と `rejected` の有効承認レコードが `supersedes` 関係なく併存する場合、機械はどちらも採らずfail-closedに `draft` とする。

### DES-127

`approved` を取り消すには `approved_state: withdrawn` の承認レコードを追加する。

### DES-128

`approved` を否認するには `approved_state: rejected` の承認レコードを追加する。

### DES-129

取消・却下の後に再承認するには、当該 `withdrawn` / `rejected` レコードのULIDを `supersedes` に名指しした `approved_state: approved` のレコードを追加する。

### DES-130

旧レコードを名指ししない `approved` の追加では `draft` のままとする。

### DES-131

対象Xの実効承認状態は `draft` と `approved` の2値であり、遷移は所定の入力だけで起きる。

### DES-132

`draft` から `approved` への遷移は、`approved_state: approved` の有効承認レコードが加わり、実効集合に `rejected` / `withdrawn` が1件も残らなくなることで起きる。

### DES-133

`approved` から `draft` への遷移は、`approved_state` が `rejected` または `withdrawn` の有効承認レコードが加わることで起きる。

### DES-134

`approved` から `draft` への遷移は、実効集合の `approved` レコードがすべて他レコードの `supersedes` に名指しされることで起きる。

### DES-135

`approved` から `draft` への遷移は、`subject` の内容ハッシュが変化することで起きる。

### DES-136

`approved` から `draft` への遷移は、上流依存closureのentity構成またはいずれかのhashが変化する（document再登録・参照先source変更を含む）ことで起きる（§11.4）。

### DES-137

Xが判断記録のとき、`approved` から `draft` への遷移は、当該判断記録が §8.5 の有効判断または実効集合Eから外れることで起きる。

### DES-138

依存entryを持たない互換Approvalは読取りと履歴表示だけを許可し、現在の `approved` を導出しない。W-STORE-002を出し、対象は `draft` 相当とする。

### DES-S017 3.6 Evidence レコード（`.verify/evidence/<ULID>.yaml`）

*導出元: SPEC-S025, SPEC-S026, SPEC-S056, DS-S016, DS-S038, BD-S012, BD-S013, BD-S023*

### DES-139

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

### DES-140

Evidenceレコードの `execution_state` の `hash` fieldは、`complete: false` の場合だけnullとする。

### DES-141

Evidenceレコードの `target_coverage` fieldは `target_binding` の動的計測結果である（旧 `target_execution` fieldを改名）。

### DES-142

Evidenceレコードの `target_coverage.checked` fieldは計測を実施したかを表す。

### DES-143

Evidenceレコードの `target_coverage.result` fieldはtarget別結果の集約であり、値域は `PASS` / `FAIL` / `UNKNOWN` である。

### DES-144

Evidenceレコードの `target_coverage` の `targets` 配下の各entryの `result` fieldの値域は `PASS` / `FAIL` / `UNKNOWN` である。

### DES-145

Evidenceレコードの `target_coverage` の `targets` 配下の各entryの `count` fieldは、`result` が `UNKNOWN` の場合nullとする。

### DES-146

Evidenceレコードの `log_ref` fieldはGit管理外の生ログを指す。

### DES-147

旧モデルの `target_execution` 検査項目は撤去し、その計測事実だけをEvidenceの `target_coverage` fieldとして保持して `target_binding` の証拠源へ吸収する（§10・§11.2）。

### DES-148

`hashes.targets` はTestの宣言順で常に記録し、各 `target` は§6.1で解決したcanonical Source Targetのcanonical Locatorの正規化文字列表現とする。

### DES-149

参照側Testが宣言した `TargetRef` の綴り（SRC ID参照を含む）をEvidence上のtarget identityとして記録しない（§6.1.1）。

### DES-150

`hashes.targets` のlistはTestの宣言target集合を解決したcanonical Source Target集合と重複なく1対1に対応する。

### DES-151

Evidence生成のprecondition（§9.4）により全宣言targetは一意に解決済みであるため、`hashes.targets` の集合は宣言target集合と同数になる。

### DES-152

`target_coverage.checked: true` では `target_coverage.targets` も同じ順序・同じcanonical Locator集合で1対1に対応する。

### DES-153

`target_coverage.checked: false` では `method` と `result` をnull、`targets` を空listとし、`target_binding` の動的計測を `NO_EVIDENCE`（診断 `NOT_CHECKED`）として扱う（§11.2）。

### DES-154

writerは `hashes.test_subject` を必須とする。

### DES-155

writerはTest construct単体のhashを現在のEvidence freshness keyとして出力しない。

### DES-156

readerは `rust-cargo` Evidenceに限り、互換fieldの `hashes.test_fn` または `hashes.test_construct` とtarget entry内の `target_fn` を読み取れる。

### DES-157

互換Test hashを現在の `test_subject` へ正規化できるのは、現在の `rust-cargo` adapterが当該互換hashのsource rangeに全canonical metadataとTest constructが含まれること、現在bytesとの完全一致、および現在のlogical metadataとの一致を証明できる場合だけとする。

### DES-158

証明できなければrecordは保持するが、`target_binding` の証拠として有効な `PASS` にしない。

### DES-159

中立fieldと互換fieldが併存する場合は導出される値の同値を必須とし、非 `rust-cargo` Evidenceでは互換fieldを解釈しない。

### DES-160

readerは単数互換形の `hashes.target_fn` および `target_coverage.result/count` を、現在の `rust-cargo` Testがtargetをちょうど1件宣言し、Test subjectを証明でき、target construct hashも照合できる場合だけ1要素listへ正規化して扱う。

### DES-161

複数target Testに単数互換形を適用しない。

### DES-162

writerは常にlist形を出力する。

### DES-163

Evidence内の `target` は実行時snapshotを識別するkeyであり、TEST → SRC edgeの正典ではない。

### DES-164

`execution_state` は§1.3のExecution State subjectである。

### DES-165

writerは実行直前にadapterからsnapshot DTOを取得し、core検証後のschema ID、完全性、subject hashを記録する。

### DES-166

`complete: true` は、選択Testのビルドと実行可能状態を変えうるrepository / local dependency入力、runner、toolchain、実行影響configをadapterが漏れなく列挙した場合だけ許可する。

### DES-167

snapshot生成不能または不完全の場合も実行事実の履歴を記録できるが、`complete: false`、`hash: null` として現在の有効な `PASS` 証拠へ使用しない。

### DES-168

`rust-cargo-execution-state-v1` のmanifestは、選択Testを含むCargo workspace / package root、全local path dependency root、各root内の通常file、Cargo manifest / lockfile、`.cargo` config、build script、Rust source / test / fixture / compile-time resource、toolchain指定を含む。

### DES-169

`.git/`、`.verify/` のcanonical record / cache、Cargo target directory等の生成物は実行入力から除外する。

### DES-170

除外領域をbuild script、macro、`include_*`、path dependencyその他の経路で読み込む可能性を排除できない場合、snapshotを完全と報告しない。

### DES-171

repository内helperだけの変更もmanifest hashを変化させる。

### DES-172

Evidence readerは `execution_state` を欠く互換recordを履歴表示できるが、現在のEvidence freshnessを証明できないため `NO_EVIDENCE`（診断 `STALE`）とする。

### DES-173

schema違反、target entryの欠落・重複・余剰、またはaggregate resultとtarget別結果の矛盾はE-SCAN-010として扱い、そのEvidenceを有効な結果に使用しない。

### DES-174

Evidence writerは `adapter` を必須で記録し、保存前にTestの `ExecutionDescriptor.adapter` およびrunner kindとの整合を検証する。

### DES-175

Evidence readerは `adapter` の欠落を許容するが、現在のTestが `rust-cargo` で、互換runner kindと内容ハッシュからRust実行であることを一意に確認できる場合だけ互換Evidenceとして扱う。

## DES-S018 4. Test metadata宣言contract

### DES-S019 4.1 adapter-neutralな正規化

*導出元: REQ-S026, SPEC-S008, SPEC-S032, SPEC-S033, SPEC-S040, DS-S003, DS-S022, DS-S023, DS-S028, BD-S005, BD-S017*

### DES-176

`SourceDiscoveryAdapter` は、adapter所有のsource declarationを `id`、`covers[]`、`targets[]`、`intent`、`input?`、`expect?`、`kind?`、`cases[]`、`related[]` の論理fieldへ正規化する。

> id, covers[], targets[], intent, input?, expect?, kind?, cases[], related[]

### DES-177

本versionでは、Testの存在理由分類（旧 `role` / `anchor` / `anchor_rationale`）を論理fieldに持たない。

### DES-178

`targets[]` は検証対象をSource Targetとして実現するためのcapability fieldであり、その要求件数はadapterが定める。

*導出元: SPEC-190, SPEC-191, SPEC-192, SPEC-193, SPEC-194*

*引用: 基本仕様 §9.2*

### DES-179

locatorは `TargetRef::Locator { adapter, value }` とし、`value` はadapter所有のopaque文字列である。

### DES-S020 4.2 `rust-cargo` annotation文法

*導出元: SPEC-S040, SPEC-S073, DS-S028*

### DES-180

`rust-cargo` の `@vtest.` 宣言表面は2種であり、表面ごとに認識する行形式が異なる。

### DES-181

Test constructのdoc comment（`///` または `/** */`）は表面1であり、test-annotation-lineを認識する。

### DES-182

Test constructではない関数itemのdoc comment（対象実装側の関数等）は表面2であり、source-target-annotation-lineを認識する。

### DES-183

test-annotation-lineの文法は `"@vtest." test-key SP value` である。

### DES-184

source-target-annotation-lineの文法は `"@vtest." source-target-key SP value` である。

### DES-185

test-keyの値域は `id` / `covers` / `target` / `intent` / `input` / `expect` / `kind` / `case` / `related` である。

### DES-186

source-target-keyの値域は `src-id` である。

### DES-187

valueは行末までのテキストとし、前後空白は除去する。

### DES-188

annotation行は1行1キーとする。

### DES-189

`covers` と `related` の値はカンマ区切りで複数指定できる。

### DES-190

`case` と `related` はキー自体を複数行書ける。

### DES-191

`case` と `related` 以外のキーの重複はエラーE-SCAN-005とする。

### DES-192

ただし `kind` がintegration系のTestに限り、`target` の複数行を許容する。

*引用: 別紙A §14.3*

### DES-193

許容された複数 `target` 内でも同じTargetRefの重複はE-SCAN-005とする。

### DES-194

綴りが異なっても解決後に同一canonical Source Targetへ到達する複数宣言（同じSource Targetへのlocator参照とSRC ID参照の併記等）も、coreが解決時にE-SCAN-005とする（§6.1.1）。

### DES-195

表面1で、`@vtest.` で始まるがtest-keyを持たない行はエラーE-SCAN-006とする（打鍵ミスの検出を優先し、警告ではなくエラーとする）。

### DES-196

表面1のE-SCAN-006は、未知キーに加え、source-target-key（`src-id`）の誤配置も含む。`src-id` は対象実装側の関数に付与すべきキーであり、Test metadataへの取り込み先を持たない。

### DES-197

表面2で、`@vtest.` で始まるがsource-target-keyを持たない行（test-keyを含む）は警告W-SCAN-105とする（§5.4）。

### DES-198

打鍵ミス検出の目的は両表面に及ぶが、表面2の宣言はTest metadataを破損させず採用値の曖昧さも生まないため、errorではなくwarningとする。

### DES-199

`src-id` は表面2でも反復不可であり、同一関数itemでの重複は採用すべきIDを決定できないためエラーE-SCAN-005とする。

### DES-200

`src-id` 重複時はいずれの宣言値も採用せず、当該Source TargetのSRC IDは無しとして扱う（どちらかを推測で選ばない）。

### DES-201

doc comment 内の `@vtest.` を含まない行は自由記述として無視する。

### DES-202

`@vtest.src-id` はテストではなく対象実装側の関数に付与し、任意の恒久SRC IDを宣言する。

### DES-203

`rust-cargo` のSource Target constructは属性とdoc commentを含む関数item全体であり、`@vtest.src-id` の宣言行はconstruct bytesの内側にある（§1.3）。

### DES-204

したがって `@vtest.src-id` の付与・変更・削除はSource Target hashを変化させる。

### DES-205

表面2での打鍵ミス（`src_id` 等の未知キー）はW-SCAN-105、`src-id` の重複はE-SCAN-005で検出し、無音で無視しない（§4.2・§5.4）。

### DES-S021 4.3 `rust-cargo` locator構文

*導出元: SPEC-S033, SPEC-S073, DS-S023, BD-S017*

### DES-206

locator文法は `locator = path "::" item-path` である。

> 例：src/parser.rs::Parser::parse
>     src/lib.rs::validate_input

### DES-207

pathはプロジェクトルートからの相対パス（"/" 区切り、".rs" で終わる）である。

### DES-208

item-pathはRustアイテムパス（"::" 区切り）であり、implブロック内の関数は"型名::関数名"とする。

### DES-209

`path` は `.rs` で終わる最初の `::` で item-path と分離する。

### DES-210

`rust-cargo` adapterはlocator値を `TargetRef::Locator { adapter: "rust-cargo", value: locator }` へ正規化する。

### DES-211

`@vtest.target` の値が `SRC-` で始まる場合はSRC ID参照として返す。

### DES-S022 4.4 宣言エラーの扱い

*導出元: SPEC-S020, SPEC-S033, SPEC-S040, DS-S011, DS-S023, DS-S028, BD-S017*

### DES-212

adapter固有のsource declarationを構文解析できない場合、adapterは該当Test constructをDiscovered Testとして返し、対応を `ManagedTestLink::Missing` として診断を付与する。

### DES-213

coreは当該Testを管理宣言欠落として `chain_integrity` の `MISMATCH`（診断 `MISSING`）とし、対応VOを推測で寄与関係へ関連付けない。

### DES-214

source declarationを構文上完全なTest Entityへ正規化できる条件は、adapter中立coreが要求する必須metadata（構文上有効なTest ID・1件以上の `covers`・`intent`）に加え、当該adapterが必須とする追加metadataをTest Entityとして具体化できることをいう。

### DES-215

coreの `id` / `covers ≥ 1` / `intent`、および `rust-cargo` の `targets ≥ 1` という必須metadataを欠く場合はE-SCAN-007とし、`ManagedTestLink::Missing`（`chain_integrity` の `MISMATCH`、診断 `MISSING`）とする。

### DES-216

E-SCAN-007はadapterが報告する構文・必須metadata診断であり、`targets ≥ 1` は `rust-cargo` の必須metadataとしてこの経路で検出される（core中立の必須リンクへは加えない）（§11.1.1）。

### DES-217

構文上完全なTest Entityへ正規化できるが、`covers` のVO IDをcore storeで解決できない場合、そのentityと `ManagedTestLink::One(id)` を保持する。

### DES-218

E-SCAN-003と `chain_integrity = MISMATCH` はcoreの参照整合性検査で生成する。

### DES-219

E-SCAN-003が発生しても対応するTest Entityと `ManagedTestLink::One` を除去しない。

### DES-220

adapterはTest構文の違反（重複不可キーの重複、未知キー、必須キーの欠落）をE-SCAN-005 / E-SCAN-006 / E-SCAN-007で報告する（§5.4）。

## DES-S023 5. Discovery orchestration設計

### DES-S024 5.1 処理フロー

*導出元: SPEC-S004, SPEC-S062, DS-S044, BD-S003, BD-S025*

### DES-221

処理フロー第1段は、registryとconfigの検証であり、adapter ID、capability宣言、config namespace、rootを検証する。

### DES-222

処理フロー第2段は、discovery委譲であり、登録順ではなくadapter ID順にSourceDiscoveryAdapterを呼び出し、各adapterはDiscoveryBatchを返す。

### DES-223

処理フロー第3段は、adapter出力の検証であり、adapter ID、Source Location、source range、current bytes、hash未計算のTest draft、metadata source、Target draft、診断を検証する。

### DES-224

処理フロー第3段では、capability宣言と出力が矛盾するbatchは拒否する。

### DES-225

処理フロー第4段は、core materializationであり、Test subjectとSource Targetのhashを§1.3で計算し、TestEntity、SourceTarget、DiscoveredTest、ManagedTestLinkを具体化する（存在理由分類の実効値確定は行わない。covers ≥ 1 を一律要求する）。

### DES-226

処理フロー第5段は、決定論的な統合であり、adapter ID、project-relative path、opaque locator、Test IDの順に正規化する。

### DES-227

処理フロー第5段では、adapter間を含むTest ID・SRC ID衝突と不正な複数対応を検査する。

### DES-228

処理フロー第6段は、`.verify/` 読み込みであり、vtest-storeが全レコード（document / VO / Relation / 判断 / 承認 / Evidence）を読み込み、スキーマ検証する。

### DES-229

処理フロー第7段は、参照整合性検査であり、coversのVO ID、targetsのTarget Reference / SRC ID、Relation、VO parent、VO derives_from（document）、document derives_from を解決する。

### DES-230

処理フロー第8段は、グラフ構築と整合性検査である（§5.3・§5.4・§5.6）。

### DES-S025 5.2 エンティティモデル（vtest-model）

*導出元: SPEC-S008, SPEC-S013, SPEC-S019, DS-S003, DS-S006, DS-S010, BD-S005, BD-S009*

### DES-231

`TestEntity` の `id` fieldは `TestId` 型である。

### DES-232

`TestEntity` の `covers` fieldは `Vec<VoId>` 型であり、1件以上を持つ（`covers ≥ 1` 一律）（§4.4）。

### DES-233

`TestEntity` の `targets` fieldは `Vec<TargetRef>` 型であり、各要素はadapter付きopaque locatorまたはSrcIdである。

### DES-234

`TestEntity.targets` の件数はadapterが定める（`rust-cargo` は `targets ≥ 1` を必須とする）（§4.1・§4.4）。

### DES-235

coreは `targets ≥ 1` を中立必須にせず、`TestEntity.targets` の型としては空を許容する。

### DES-236

`TestEntity` の `intent` fieldは `String` 型である。

### DES-237

`TestEntity` の `input` fieldは `Option<String>` 型である。

### DES-238

`TestEntity` の `expect` fieldは `Option<String>` 型である。

### DES-239

`TestEntity` の `kind` fieldは `Option<String>` 型である。

### DES-240

`TestEntity` の `cases` fieldは `Vec<String>` 型である。

### DES-241

`TestEntity` の `related` fieldは `Vec<TestId>` 型である。

### DES-242

`TestEntity` の `location` fieldは `SourceLocation` 型である。

### DES-243

`TestEntity` の `content_hash` fieldは `ContentHash` 型であり、§1.3のTest subject hash（coreが計算）である。

### DES-244

`TestEntity` の `execution` fieldは `ExecutionDescriptor` 型である。

### DES-245

`TargetRef` は `Locator { adapter: AdapterId, value: String }` variantを持つ。

### DES-246

`TargetRef` は `SrcId(SrcId)` variantを持つ。

### DES-247

`SourceLocation` の `adapter` fieldは `AdapterId` 型である。

### DES-248

`SourceLocation` の `path` fieldは `ProjectPath` 型である。

### DES-249

`SourceLocation` の `locator` fieldは `String` 型であり、adapter所有のopaque construct locatorである。

### DES-250

`SourceLocation` の `byte_range` fieldは `SourceRange` 型である。

### DES-251

`ExecutionDescriptor` の `adapter` fieldは `AdapterId` 型である。

### DES-252

`ExecutionDescriptor` の `project` fieldは `Option<String>` 型である。

### DES-253

`ExecutionDescriptor` の `suite` fieldは `Option<TestSuite>` 型である。

### DES-254

`ExecutionDescriptor` の `selector` fieldは `String` 型である。

### DES-255

`TestSuite` の `kind` fieldは `String` 型である。

### DES-256

`TestSuite` の `name` fieldは `Option<String>` 型である。

### DES-257

`CheckValue` は `Pass` variantを持つ。

### DES-258

`CheckValue` は `Fail` variantを持つ。

### DES-259

`CheckValue` は `Mismatch` variantを持つ。

### DES-260

`CheckValue` は `NoEvidence` variantを持つ。

### DES-261

`CheckValue` は `Unknown` variantを持つ。

### DES-262

`DiagnosticLabel` は `Missing` variantを持つ。

### DES-263

`DiagnosticLabel` は `NotExecuted` variantを持つ。

### DES-264

`DiagnosticLabel` は `NotChecked` variantを持つ。

### DES-265

`DiagnosticLabel` は `Stale` variantを持つ。

### DES-266

`CheckItem` は `ChainIntegrity` variantを持つ。

### DES-267

`CheckItem` は `OrphanDetection` variantを持つ。

### DES-268

`CheckItem` は `TargetBinding` variantを持つ。

### DES-269

`CheckItem` は `OraclePresence` variantを持つ。

### DES-270

`CheckValue` は状態のみを表し、原因説明は `DiagnosticLabel` として併記する。

### DES-271

`Missing` / `NotChecked` / `NotExecuted` / `Stale` を検証状態のvariantとして持たせない（旧8値モデルの排除）。

### DES-272

collision時はE-SCAN-011とし、TargetRefを解決しない。

### DES-273

`TestEntity.execution` はadapter、project、suite、opaque selectorからなる中立な実行座標である。

### DES-274

`filter`、`package`、`test_target` および `TestTarget` 型を `vtest-model` へ置かない。

### DES-275

`vtest-adapter-api` は言語非依存の `TestWireCodec` capabilityを定義する。

### DES-276

codecはadapter固有のcompatibility propertyをJSON objectとしてencode / decodeできるが、core domain typeへadapter固有fieldを追加しない。

### DES-277

JSON writerは `execution` を常に出力し、`rust-cargo` TestだけにRust互換fieldを追加する。

### DES-278

非Rust TestではRust互換fieldを省略する。

### DES-279

JSON readerは `execution` を優先し、互換field併存時はdescriptorとの一致を検証する。

### DES-280

`execution` が欠ける場合、完全で相互整合するRust互換fieldからだけ `rust-cargo` descriptorを導出する。

### DES-281

不完全・矛盾時は入力を拒否し、空selectorまたはdummy値を生成しない。

### DES-282

Test JSON writerは `TestEntity.targets` を1件以上のlistとして常に出力する。

### DES-283

targetが1件の場合だけ同値の単数互換field `target` を追加できる。

### DES-284

readerは `target` だけの入力を1要素listへ正規化し、`targets` との併存時は完全一致を検証する。

### DES-285

複数targetから代表値を選んで `target` を生成しない。

### DES-286

`SourceDiscoveryAdapter` は、`SourceFragment`・`ManagedTestDraft`・`DiscoveredTestDraft`・`ManagedTestDraftLink`・`SourceTargetDraft`・`DiscoveryBatch`・`DiscoveryCompleteness` をhash未計算のDTOとして返す。

### DES-287

`SourceFragment` の `location` fieldは `SourceLocation` 型である。

### DES-288

`SourceFragment` の `bytes` fieldは `Vec<u8>` 型である。

### DES-289

`ManagedTestDraft` の `id` fieldは `TestId` 型である。

### DES-290

`ManagedTestDraft` の `covers` fieldは `Vec<VoId>` 型である。

### DES-291

`ManagedTestDraft` の `targets` fieldは `Vec<TargetRef>` 型である。

### DES-292

`ManagedTestDraft` の `intent` fieldは `String` 型である。

### DES-293

`ManagedTestDraft` の `input` fieldは `Option<String>` 型である。

### DES-294

`ManagedTestDraft` の `expect` fieldは `Option<String>` 型である。

### DES-295

`ManagedTestDraft` の `kind` fieldは `Option<String>` 型である。

### DES-296

`ManagedTestDraft` の `cases` fieldは `Vec<String>` 型である。

### DES-297

`ManagedTestDraft` の `related` fieldは `Vec<TestId>` 型である。

### DES-298

`ManagedTestDraft` の `execution` fieldは `ExecutionDescriptor` 型である。

### DES-299

`DiscoveredTestDraft` の `adapter` fieldは `AdapterId` 型である。

### DES-300

`DiscoveredTestDraft` の `location` fieldは `SourceLocation` 型である。

### DES-301

`DiscoveredTestDraft` の `construct` fieldは `SourceFragment` 型である。

### DES-302

`DiscoveredTestDraft` の `metadata_sources` fieldは `Vec<SourceFragment>` 型である。

### DES-303

`DiscoveredTestDraft` の `managed` fieldは `ManagedTestDraftLink` 型である。

### DES-304

`ManagedTestDraftLink` は `Missing` variantを持つ。

### DES-305

`ManagedTestDraftLink` は `One(ManagedTestDraft)` variantを持つ。

### DES-306

`ManagedTestDraftLink` は `Multiple(Vec<ManagedTestDraft>)` variantを持つ。

### DES-307

`SourceTargetDraft` の `target` fieldは `TargetRef` 型である。

### DES-308

`SourceTargetDraft` の `src_id` fieldは `Option<SrcId>` 型である。

### DES-309

`SourceTargetDraft` の `location` fieldは `SourceLocation` 型である。

### DES-310

`SourceTargetDraft` の `construct` fieldは `SourceFragment` 型である。

### DES-311

`DiscoveryBatch` の `adapter` fieldは `AdapterId` 型である。

### DES-312

`DiscoveryBatch` の `completeness` fieldは `DiscoveryCompleteness` 型である。

### DES-313

`DiscoveryBatch` の `discovered_tests` fieldは `Vec<DiscoveredTestDraft>` 型である。

### DES-314

`DiscoveryBatch` の `source_targets` fieldは `Vec<SourceTargetDraft>` 型である。

### DES-315

`DiscoveryBatch` の `diagnostics` fieldは `Vec<Diagnostic>` 型である。

### DES-316

`DiscoveryCompleteness` は `Complete` variantを持つ。

### DES-317

`DiscoveryCompleteness` は `Incomplete` variantを持つ。

### DES-318

Source Targetはcanonical locator（`TargetRef::Locator`）と任意の恒久SRC IDを併有する単一のdomain entityである。

### DES-319

`TargetRef::Locator` と `TargetRef::SrcId` はいずれも同一Source Targetへのaddressing modeであり、別個のentityを指さない。

### DES-320

恒久SRC IDはlocatorの代替ではなく、同じSource Targetへ与えられるoptional permanent identityである。

### DES-321

adapterは `@vtest.src-id` 等で宣言された恒久SRC IDを `SourceTargetDraft.src_id` として返す。

### DES-322

同一constructをlocator版とSrcId版の2件のdraftへ複製してはならない。

### DES-323

`SourceTargetDraft.target` は必ず `TargetRef::Locator` でなければならない（§1.3）。

### DES-324

`TargetRef::SrcId` はSource Targetへの参照表現であり、`SourceTargetDraft` のcanonical targetとして返してはならない。

### DES-325

adapterが `target` に `TargetRef::SrcId` を返した場合はmalformed adapter outputとして拒否する。

### DES-326

恒久SRC IDは `src_id` だけで搬送し、`target` の綴りを変えない。

### DES-327

coreは `src_id` を統合済みSRC索引へ登録し、locator参照とSRC ID参照のどちらから解決しても同一のcanonical Source Targetへ到達させる。

### DES-328

Source Target hashは常にcanonical Locatorとconstruct bytesから計算し、恒久SRC IDを独立したhash fieldとして含めない。

### DES-329

canonical Locatorは恒久SRC IDの増減で変化しないため、参照方法の違いによってSource Targetの件数、content / subject hash、Evidence上のtarget identityが分裂しない。

### DES-330

恒久SRC IDの宣言をconstruct bytesの内側へ置くadapterでは、その宣言の追加・変更・削除がconstruct bytesを変え、Source Target hashを変化させうる（§1.3）。

### DES-331

恒久SRC IDの宣言をconstruct bytesの内側へ置くadapterでSource Target hashが変化することは、sourceが実際に変化したことの帰結であり、参照方法による分裂ではない。

### DES-332

coreは統合済みSRC索引から、その恒久SRC IDを宣言した `SourceTargetDraft.target`（= canonical Locator）へ解決する。

### DES-333

恒久SRC IDを持つSource Targetも引き続きcanonical locatorでaddressableでなければならない。

### DES-334

adapterは `SourceFragment.bytes` が `location.byte_range` の現在bytesと一致する状態だけを返す。

### DES-335

manifest等にある非隣接metadataも `metadata_sources` へ列挙するが、hash inputはadapter構文のraw表現ではなく `ManagedTestDraft` のcanonical logical metadataである。

### DES-336

coreはrange・bytes対応を検証し、§1.3でhashを計算してから `TestEntity`、`SourceTarget` および `DiscoveredTest` を具体化する。

### DES-337

`ManagedTestDraftLink::One` / `Multiple` の各draftは、全logical metadataを導出した1件以上の `metadata_sources` を持たなければならない。

### DES-338

provenance欠落はmalformed adapter outputとしてE-ADAPTER-002で拒否する。

### DES-339

`DiscoveredTest` の `adapter` fieldは `AdapterId` 型である。

### DES-340

`DiscoveredTest` の `location` fieldは `SourceLocation` 型である。

### DES-341

`DiscoveredTest` の `content_hash` fieldは `ContentHash` 型である。

### DES-342

`DiscoveredTest` の `managed` fieldは `ManagedTestLink` 型である。

### DES-343

`ManagedTestLink` は `Missing` variantを持つ。

### DES-344

`ManagedTestLink` は `One(TestId)` variantを持つ。

### DES-345

`ManagedTestLink` は `Multiple(Vec<TestId>)` variantを持つ。

### DES-346

`SourceDiscoveryAdapter` はadapterがTestとして認識した全Discovered Test draftを返す。

### DES-347

`ManagedTestDraftLink::One` は、構文上有効なTest IDと必須metadata（core中立の `covers ≥ 1` / `intent`、および当該adapterが必須とする追加metadata。`rust-cargo` では `targets ≥ 1`）をdraftとして具体化できる場合に設定する（§4.1・§4.4）。

### DES-348

解決不能な `covers` を持つdraftもcore materialization後のmanaged entity集合に保持され、対応するobservationは `ManagedTestLink::One(id)` を持つ。

### DES-349

`ManagedTestDraftLink::Missing` は管理宣言の欠落または必須metadataの欠落を表す。

### DES-350

`ManagedTestDraftLink` の `Multiple` は同一Test constructから複数draftが生じる状態を表す。

### DES-351

core materialization後の対応する状態が `ManagedTestLink` となる。

### DES-352

各adapterは一意なID、languages、capabilities、config namespaceを宣言する。

### DES-353

registryは宣言と実装の不一致および重複IDを拒否する。

### DES-354

明示操作に必須のcapabilityがない場合はE-ADAPTER-004で操作を中止する。

### DES-355

`TestRunnerAdapter` は、coreがfreshness subjectを所有できるよう `ExecutionInputDraft`・`ExecutionStateDraft` をhash未計算のDTOとして返す。

### DES-356

`CanonicalProjection` は型tag、null、list順序、map key順序を保持する言語非依存値とする。

### DES-357

`ExecutionInputDraft` の `root_identity` fieldは `String` 型である。

### DES-358

`ExecutionInputDraft` の `root_relative_path` fieldは `String` 型である。

### DES-359

`ExecutionInputDraft` の `kind` fieldは `String` 型である。

### DES-360

`ExecutionInputDraft` の `bytes` fieldは `Vec<u8>` 型である。

### DES-361

`ExecutionStateDraft` の `schema_id` fieldは `String` 型である。

### DES-362

`ExecutionStateDraft` の `schema_version` fieldは `String` 型である。

### DES-363

`ExecutionStateDraft` の `complete` fieldは `bool` 型である。

### DES-364

`ExecutionStateDraft` の `head_revision` fieldは `Option<String>` 型である。

### DES-365

`ExecutionStateDraft` の `runner_kind` fieldは `String` 型である。

### DES-366

`ExecutionStateDraft` の `invocation` fieldは `CanonicalProjection` 型である。

### DES-367

`ExecutionStateDraft` の `toolchain_identity` fieldは `String` 型である。

### DES-368

`ExecutionStateDraft` の `effective_config` fieldは `CanonicalProjection` 型である。

### DES-369

`ExecutionStateDraft` の `inputs` fieldは `Vec<ExecutionInputDraft>` 型である。

### DES-370

`StaticAnalysisAdapter` は正典レコードを持たない再計算派生であり、判定は現在のsource / target / configから都度計算する（§7.1）。

### DES-371

coreはfreshness subjectを静的解析用に永続化せず、検証のたびに現在入力で再導出する。

### DES-372

Test Runnerはcommand起動前に `ExecutionStateDraft` を構築し、実際に使用するinvocation / toolchain / configと一致するDTOだけを実行結果へ添付する。

### DES-373

`invocation` はselector、working root、runner option等をmachine非依存に正規化し、絶対pathを含む表示用commandとは分離する。

### DES-374

coreは実行前後でExecution State subject全体が変化していないことを確認してからEvidenceを記録する。

### DES-375

変化した場合はE-EXEC-004としてEvidenceを生成しない。

### DES-376

有効性再評価では同じschemaを持つ現在DTOを再構築し、保存hashと比較する。

### DES-377

Structured Test capabilityを宣言するadapterは、処理可能なbuilt-in Form `kind` 集合と、adapter fieldを持たないForm Schemaを判定するcompatibility matcherを宣言する。

### DES-378

Form Schemaの `adapter` field、registryのowner、Structured Test capabilityが同じadapter IDを示す場合だけ `kind → adapter` を確定する。

### DES-379

`adapter` fieldを欠く読取り互換Formは、登録済みStructured Test adapterのbuilt-in kind宣言またはcompatibility matcherのうちちょうど1件だけがschemaを受理する場合に限ってin-memoryでownerを補える。

### DES-380

登録済みStructured Test adapterのbuilt-in kind宣言またはcompatibility matcherのうちschemaを受理するものが0件または複数件なら操作を拒否し、ファイルを書き換えない。

### DES-381

matcherはsource bytes、schema field / validator集合等から決定論的に判定し、form kindの文字列だけを理由に汎用fallbackしてはならない。

### DES-382

document / VO / Relation / 判断記録 / 承認記録 / Evidence も §3 のスキーマに対応するstructを定義する。

### DES-S026 5.3 検証グラフ

*導出元: SPEC-S009, SPEC-S054, DS-S037, BD-S006*

### DES-383

検証グラフのノードは `DOC`、`VO`、`TEST`、`SRC`（ロケータ単位）である。

### DES-384

検証グラフのエッジ `DOC → DOC` は `derives_from` であり、documentレコード由来である。

### DES-385

検証グラフのエッジ `VO → DOC` は `derives_from` であり、VOレコード由来、1:N（1件以上）である。

### DES-386

検証グラフのエッジ `VO → VO` は `parent` である。

### DES-387

検証グラフのエッジ `TEST → VO` は `covers` であり、adapter所有のTest metadata宣言由来である。

### DES-388

検証グラフのエッジ `TEST → SRC` は `targets` であり、検証対象をSource Targetとして実現する形態、1:N（`rust-cargo` では `targets ≥ 1`）である（§4.1）。

### DES-389

検証グラフは、`rel/` 由来の外部Relationをエッジとして持つ。

### DES-390

検証グラフは、VO → Tests、SRC → Tests、DOC → VOs、DOC → DOCs（下流）の逆引きインデックスを持つ。

### DES-391

旧モデルのSPEC / REQノードとREQ→SPEC / VO→REQエッジは持たない。

### DES-S027 5.5 `rust-cargo` SourceDiscoveryAdapter

*導出元: SPEC-S070, SPEC-S073, DS-S051, BD-S033*

### DES-392

`rust-cargo` adapterは§5.1の `DiscoveryBatch` を構築する。

### DES-393

当該adapterは検証対象をSource Targetとして実現する形態であり、各管理対象Testに1件以上のSource Target（`targets ≥ 1`）を必須とする。

### DES-394

`rust-cargo` discoveryの第1段はファイル探索であり、adapter configのinclude配下の `*.rs` をignoreクレートで列挙する（`.gitignore` 準拠、`target/` は除外）。

### DES-395

`rust-cargo` discoveryの第2段は構文解析であり、ファイルごとに `syn::parse_file` する。解析エラーのファイルはE-SCAN-001を返し、batchをIncompleteとする。

### DES-396

`rust-cargo` discoveryの第3段はモジュールパス構築であり、crateルート（`src/lib.rs` / `src/main.rs` / `tests/*.rs`）からmod宣言を辿り、各itemの完全モジュールパスを構築する。

### DES-397

`rust-cargo` discoveryの第5段はmetadata宣言抽出であり、doc属性（`#[doc = "..."]`）を§4.2の文法でparseする（id / covers / target / intent / input / expect / kind / case / related）。

### DES-398

`rust-cargo` discoveryの第6段はSource Target抽出であり、すべてのfn / impl fnをSRC候補として索引化し、§4.3のlocator解決・逆引き・`@vtest.src-id` 認識（非Test constructの宣言に限る）に使用する（§4.2）。

### DES-399

`rust-cargo` discoveryの第7段はdraft生成であり、全Discovered Test draft、ManagedTestDraftLink、SourceTargetDraft、Source Location、construct / metadata source rangeとbytes、logical metadata、宣言された恒久SRC ID、ExecutionDescriptor、診断をhash未計算のDiscoveryBatchに格納する。

## DES-S028 6. Target Reference解決

### DES-S029 6.1 adapter-neutral解決contract

*導出元: SPEC-S011, SPEC-S033, DS-S023, BD-S008, BD-S017*

### DES-400

adapterは正規化されたTarget Reference、Source Location、source range、content bytes、解決status、候補を返す。

### DES-401

coreは、返却されたadapter IDとTarget Referenceの一致、source rangeの範囲、current bytesとの一致を検証する。

### DES-402

coreは§1.3のSource Target hashを計算する。

### DES-403

SRC ID参照は対応するadapterのSource Locationとsource rangeを使用する。

### DES-S030 6.2 `rust-cargo` locator解決

*導出元: SPEC-S033, DS-S023, BD-S017*

### DES-404

`rust-cargo`のlocator`path::item-path`の解決は、§5.5で構築したSRC索引への完全一致検索とする。

### DES-405

`rust-cargo`のlocator解決は、pathが索引に存在するかを確認する。

> 1. path が索引に存在するか / 2. path 内で item-path が一致する fn / impl fn が存在するか / 3. 一意に決まらない場合（同名 fn が cfg 分岐で複数等）はすべて候補として返し、解決失敗（E-SCAN-004）とする

### DES-406

`rust-cargo`のlocator解決は、path内でitem-pathが一致するfn / impl fnが存在するかを確認する。

### DES-407

`rust-cargo`のlocator解決で一意に決まらない場合（同名fnがcfg分岐で複数等）は、すべて候補として返し、解決失敗（E-SCAN-004）とする。

### DES-S031 6.3 候補提示

*導出元: SPEC-S043, DS-S029, BD-S019*

### DES-408

`rust-cargo` adapterは、item-pathの末尾セグメント一致（別パスの同名関数）の順で候補を構築する。

> 1. item-path の末尾セグメント一致（別パスの同名関数） / 2. 編集距離 2 以内の近似名 / 出力例： ✗ symbol not found: src/parser.rs::Parser::prase / candidates: src/parser.rs::Parser::parse / src/parser.rs::Parser::parse_inner

### DES-409

`rust-cargo` adapterは、編集距離2以内の近似名の順で候補を構築する。

### DES-410

`rust-cargo` adapterのenum variant検証（`expect`の値が`ParseError::InvalidUtf8`形式の場合）は、スキャン済みASTからenum定義を検索する。

## DES-S032 7. Static Analysis orchestrationと`rust-cargo`ルール

### DES-S033 7.1 判定の原則

*導出元: P-003, SPEC-S023, SPEC-S027, DS-S014, DS-S017, BD-S014*

### DES-411

`vtest-audit`は`TestEntity.execution.adapter`をregistryで解決する。

### DES-412

`vtest-audit`は、Test、全Target Reference、各source range、content hash、および選択adapterの現在configを`StaticAnalysisAdapter`へ渡す。

### DES-413

adapterはrule ID、verdict、根拠span、解析限界を返す。

### DES-414

target-scopedなDA-002 / DA-003については、宣言targetごとのverdictと根拠spanを（規則単位のverdictへ畳み込む前の形で）返す。

### DES-415

target-scopedなDA-002 / DA-003の集合を全宣言targetと過不足なく1対1に対応させる。

### DES-416

coreはadapter ID、source location・現在bytesとの対応、決定論的encodingを検証し、§7.2の規則で集約する。

### DES-S034 7.3 target 到達の静的証明と runtime 証明の関係（target_binding）

*導出元: SPEC-S022, SPEC-S026, SPEC-S032, DS-S013, DS-S022, BD-S013*

### DES-417

`target_binding`項目値は検証時に算出する。

## DES-S035 8. 判断記録プロトコル

*導出元: REQ-S035, SPEC-S036, DS-S026*

### DES-S036 8.1 バンドル生成

### DES-418

バンドルには基本仕様§11.3が定める判断対象の情報一式を含める。

*導出元: SPEC-226, SPEC-227, SPEC-228, SPEC-229, SPEC-230, SPEC-231, SPEC-232, SPEC-233, SPEC-234, SPEC-235, SPEC-506, SPEC-507, SPEC-508, SPEC-509*

*引用: 基本仕様 §11.3*

### DES-419

バンドルは対象VO（`--vo` / `--test`から導出したcovers先VOレコードとclaim）を含める。

### DES-420

バンドルはTest Intent（`--test`の場合の対象Testのintent・input・expect）を含める。

### DES-421

バンドルはテストコード（Test construct source全文とmetadata宣言）を含める。

### DES-422

バンドルはTestのcases集合（対象Testが`@vtest.case`で宣言したcaseの正規化文字列を宣言順に並べたlist。§4.1の論理field`cases[]`）を含める。

> 宣言が無いTestでは空listを明示し、項目自体を省略しない。

### DES-423

バンドルは対象実装（全宣言targetのimplementation construct source全文）を含める。

### DES-424

バンドルは関連テスト（related / 同一VOをcoversする他Testのidとintent）を含める。

### DES-425

バンドルは既知partition（対象VOのdimensions・coverage_policy・representative_cases）を含める。

### DES-426

バンドルは過去の判断（同一`(subject, judgment_kind)`への有効・無効な過去判断記録の要約）を含める。

### DES-427

バンドルは対象の内容ハッシュとリビジョン（Test subject / target subject / VO subjectの現在hash、revision）を含める。

### DES-428

バンドルは判断型（`judgment_kind`）をちょうど1件持ち、その値を`judgment_kind`として出力する。

### DES-429

判断型は判断対象を一意に区切るkeyであり（§3.4）、判断記録へ複製される。

### DES-430

`impl-consistency`型の判断（対象実装が宣言と一致するかの意味判定）のように上流documentを要する対象では、対象VOから§3.5と同じ上流依存規則で導出するdocument subject完全集合とsource全文を加える。

### DES-431

`case-coverage`型のバンドルでは、covers先の全leaf / 中間VOの`dimensions`・`coverage_policy`・`representative_cases`と、Testのcases集合を必須項目として含める。

### DES-S037 8.2 バンドル JSON スキーマ（例）

### DES-432

バンドルJSONスキーマは`bundle_id`・`generated_at`・`revision`・`subject`・`judgment_kind`・`test`・`vos`・`targets`・`related_tests`・`static_analysis`・`prior_decisions`のfieldからなる。

> 例: { "bundle_id": "01J8XVYY...", "generated_at": "2026-08-08T00:00:00Z", "revision": { "commit": "abc123...", "dirty": false }, "subject": "TEST-PARSER-044", "judgment_kind": "test-semantic", "test": {...}, "vos": [...], "targets": [...], "related_tests": [...], "static_analysis": {...}, "prior_decisions": [...] }

### DES-S038 8.3 提出スキーマ

### DES-433

`vtest audit submit --file result.json`で提出する。

### DES-434

提出スキーマは`bundle_id`・`subject`・`judgment_kind`・`supersedes`・`decision`・`reason`・`exclusions`・`actor`のfieldからなる。

> 例: { "bundle_id": "01J8XVYY...", "subject": "TEST-PARSER-044", "judgment_kind": "test-semantic", "supersedes": [], "decision": "accepted", "reason": [...], "exclusions": [...], "actor": { "kind": "agent", "id": "judge-agent-01", "model": "claude-fable-5" } }

### DES-S039 8.4 提出の検証

### DES-435

旧モデルのreasons / claim / basis必須検査（E-AUDIT-005）、decomposition-viewpoint検査（E-AUDIT-006）、spec / req basis検査（E-AUDIT-007）は撤去する。

## DES-S040 9. テスト実行設計

*導出元: SPEC-S026, SPEC-S056, DS-S038, BD-S013, BD-S023*

### DES-S041 9.2 `rust-cargo` TestRunnerAdapter

### DES-436

`rust-cargo` adapterは`TestEntity.execution`の`project`をcargo package名として解釈する。

> `rust-cargo` adapterはTestEntity.executionを次のCargo実行座標として解釈する。

### DES-437

`rust-cargo` adapterは`TestEntity.execution`の`suite.kind`を`lib` / `bin` / `integration`として解釈する。

### DES-438

`rust-cargo` adapterは`TestEntity.execution`の`suite.name`をbin名またはintegration test target名として解釈し、`lib`では省略する。

### DES-439

`rust-cargo` adapterは`TestEntity.execution`の`selector`をtest targetのrootからのmodule path＋function名（例：`parser::tests::rejects_invalid_utf8`）として解釈する。

### DES-440

adapter内部ではこれらからCargo launch coordinateを構築する。

### DES-441

`TestEntity`へCargo固有fieldを戻してはならない。

### DES-442

実行は（project, suite）で分けたbatchとし、libtestの`--exact` flagと複数selectorを用いる。

> cargo test -p <project> --lib -- --exact <selector1> <selector2> ...（IntegrationTest の場合は --lib の代わりに --test <name>）

### DES-443

`--exact`は後続の全フィルタへ適用されるフラグであり、各フィルタは完全一致で解釈される。

### DES-S042 9.3 `rust-cargo` 結果のパース

### DES-444

stdoutのパースはstable toolchainの標準出力形式のみに依存する。

### DES-445

`running N tests`という出力は実行対象数の確認を意味する。

### DES-446

`test <selector> ... ok`という出力はPASSを意味する。

### DES-447

`test <selector> ... FAILED`という出力はFAILを意味する。

### DES-448

`test <selector> ... ignored`という出力は実行されずを意味する。

### DES-449

stdout / stderrの全文は`cache/logs/<ULID>.log`へ保存し、Evidenceの`log_ref`から参照する。

### DES-S043 9.4 Evidence の記録

### DES-450

`revision`は実行直前に`git rev-parse HEAD`と`git status --porcelain`で取得する。

### DES-451

`hashes`は、実行直前のdiscovery結果から、Test subject hashと、全宣言targetを§6.1で解決したcanonical Locator・implementation construct hash（§1.3）を宣言順で記録する。

### DES-452

`execution_state`は、実行直前にrunner adapterが返すsnapshot schema、runner / toolchain / 実行影響config、およびrepository / local dependency入力manifestをcoreが検証し、§1.3のExecution State subject hashとして記録する。

## DES-S044 10. `rust-cargo` Target Binding 動的計測

*導出元: SPEC-S022, SPEC-S056, DS-S013, DS-S038, BD-S023*

### DES-S045 10.1 計測方式

### DES-453

起動時に`cargo llvm-cov --version`で利用可否を確認し、利用不能なら計測しない。

### DES-454

カバレッジをTest単位で対象関数へ帰属させるため、計測時はTestを1件ずつ実行する。

### DES-455

subprocess内の実行を計測するには起動される実行体もinstrument対象とし、子プロセスのprofileをmergeする必要がある。

### DES-456

計測コマンドは`cargo llvm-cov test -p <project> --lib --json --output-path cache/cov/<ULID>.json -- --exact <selector>`である。

### DES-S046 10.2 判定

### DES-457

出力JSON（llvm-cov export形式）の`data[].functions[]`から、Testが宣言する各対象関数を検索する。

### DES-458

ジェネリック関数は複数インスタンスが現れるため、同じtargetに対応するcountを合算する。

### DES-459

各targetのcanonical Locator（§6.1.1）・result・countとTest単位集約結果をEvidenceの`target_coverage`へ記録する。

### DES-460

coverage providerは当該境界越しの実行を宣言targetへ帰属させなければならない（例：起動される実行体も計測対象としてinstrumentし、子プロセスのprofileをmergeする）。

## DES-S047 11. 鮮度検証と集約

### DES-S048 11.3 集約アルゴリズム

*導出元: SPEC-S058, DS-S040*

### DES-461

`verify.full_scope`はconfig読込み時に§2.2のinvariantとして検証・正規化済みでなければならない。

### DES-462

aggregateは、scanによりグラフ構築する（§5）。

### DES-463

基本仕様 §22.2がTest単位の結果の集約先として挙げる「Feature単位」は、親VO（`parent`により1件以上の子VOを持つVO。§3.2）を単位として実現する。

*導出元: SPEC-324, SPEC-325, SPEC-326*

*引用: 基本仕様 §22.2*

### DES-S049 11.6 役割別 projection

*導出元: REQ-S007, SPEC-S054, SPEC-S060, DS-S037, DS-S042*

### DES-464

親VOを起点とする下流方向のprojectionが、§11.3の機能単位の集約（Feature単位＝親VO）を提示する経路である。

### DES-465

Feature名・Feature IDの別fieldを出力に設けず、束ねの識別子は親VOのIDとする。

### DES-466

逆引きインデックス（VO → Tests、SRC → Tests、DOC → VOs、DOC → DOCs）をprojectionの基盤とする（§5.3）。

### DES-467

projectionが出力する`derives_from`エッジ（DOC → DOC、DOC → VO）には、当該entryの`anchor`（§3.1・§3.2）を常に同伴させる。

### DES-S050 11.7 判断待ち情報の構造

*導出元: SPEC-S052, SPEC-S073*

### DES-468

`subject`は対象エンティティIDまたは解決済みcanonical Locatorとする。

### DES-469

`kind`は`unknown`（UNKNOWNによるエスカレーション）/ `unregistered`（管理宣言欠落）/ `unresolved`（参照解決不能）/ `undecided`（VO未確定）/ `pending_approval`（承認待ち）のいずれかとする。

### DES-470

`check`は関係する検査（4検査のいずれか）と現在の検証状態・診断ラベルとする。

### DES-471

`judgment_kind`は外部判断が必要な場合の判断型（§8.1の値域）とする。

### DES-472

不要な項目では`judgment_kind`を`null`とする。

### DES-473

`basis`は機械的に確認済みの事実（宣言鎖・検査結果・対象外とした範囲）への参照とする。

### DES-474

`bundle_ref`は外部判断が必要な場合の判断バンドル（§8.1）への参照（任意）とする。

## DES-S051 16. 並列動作と整合性

### DES-S052 16.1 ロック不要の根拠

*導出元: SPEC-S064, DS-S046, BD-S028*

### DES-475

同時実行された`vtest`プロセス同士の調停は行わない。

## DES-S053 19. 実装選択と提供範囲

*導出元: R-2, R-3, SPEC-S070, DS-S051, BD-S033*

### DES-476

demangle実装（`rustc-demangle`）の適用範囲は、基本仕様と本詳細設計の観測可能な契約を変更しない範囲で実装が選択できる。

> 次の事項は、基本仕様と本詳細設計の観測可能な契約を変更しない範囲で実装が選択できる。

### DES-477

`#[tokio::test]`等、属性末尾`test`以外のカスタムテスト属性への対応範囲は、基本仕様と本詳細設計の観測可能な契約を変更しない範囲で実装が選択できる。

### DES-478

cargo workspace外の単一クレートプロジェクトでのパス解決の細部は、基本仕様と本詳細設計の観測可能な契約を変更しない範囲で実装が選択できる。

### DES-479

レポートのツリー描画の細部（文字種、折返し）は、基本仕様と本詳細設計の観測可能な契約を変更しない範囲で実装が選択できる。

## DES-S054 12. CLI 詳細仕様

### DES-S055 12.1 共通仕様

*導出元: REQ-S009, SPEC-S013, SPEC-S014, SPEC-S018, SPEC-S061, SPEC-S085, SPEC-S106, SPEC-S114, DS-S006, DS-S009, DS-S043, DS-S066, DS-S071, DS-S102, DS-S112, BD-S051, BD-S054, BD-S072*

### DES-480

`filter`、`package`、`test_target` は `TestEntity` の field ではない。

### DES-481

coreは `targets ≥ 1` を adapter 中立の必須件数にせず、型としては空 list を許容する。

## DES-S056 13. MCP ツール詳細仕様

### DES-S057 13.1 共通仕様

*導出元: SPEC-S069, DS-S050, BD-S032*

### DES-482

各ツール呼び出しの冒頭で mtime ベースの再スキャン判定を行う。

*引用: 本冊 §2.3*

## DES-S058 15. Structured Test Operation adapter contract

### DES-S059 15.1 `rust-cargo` 対象の特定

*導出元: SPEC-S045, DS-S032*

### DES-483

Test ID から編集対象を特定する。

> ```text
> TEST-X → スキャン結果 → SourceLocation
>   （ファイル、関数アイテムの byte range、
>     doc comment 開始位置を含む拡張 range）
> ```

### DES-484

スキャン結果が古い可能性があるため、編集直前に対象ファイルのみ再パースし、Test ID の位置を再確認する。

### DES-S060 15.2 `rust-cargo` 編集・挿入の適用

*導出元: SPEC-S044, SPEC-S045, DS-S030, DS-S032*

### DES-485

desired state（answers / set / body）から、あるべきアノテーションブロックと関数シグネチャ・本体を生成する。

### DES-486

現状とあるべき状態の diff を計算する。

### DES-487

変更を、対象テスト関数の拡張 range（doc comment 先頭〜関数末尾）の単一置換として適用する。

### DES-488

Form 回答（§14）から、あるべきアノテーションブロックと関数シグネチャ・本体、および挿入位置を決定する。

### DES-489

挿入前の対象ファイルの内容を保持する。

### DES-490

対象ファイルが存在しない場合は「不存在」を挿入前の状態として保持する。

### DES-491

生成した Test construct を挿入位置へ単一挿入として適用する。

### DES-S061 15.4 `rust-cargo` 1 Test境界の保証

*導出元: REQ-S039, SPEC-S045, DS-S032*

### DES-492

置換範囲が単一のテスト関数の拡張 range に限られることを、適用前（範囲計算）と適用後（他 Test のハッシュ不変確認）の二重で検査する。

## DES-S062 18. 受入契約

### DES-S063 18.1 共通条件

*導出元: SPEC-S018, SPEC-S025, SPEC-S059, SPEC-S063, DS-S009, DS-S016, DS-S041, DS-S045, BD-S012, BD-S026*

### DES-493

受入条件は決定論的なfixtureと統合テストで再現できる。

### DES-494

Rust workspaceの受入テストは`cargo test --workspace`で実行できる。

### DES-495

canonical record、承認記録、判断記録、Evidence、内容hashの不変条件をfixtureの都合で緩和しない。

### DES-S064 18.3 機能別受入条件

#### DES-S065 18.3.1 discovery・record・graph と chain_integrity

*導出元: SPEC-S010, SPEC-S020, SPEC-S033, SPEC-S048, SPEC-S062, SPEC-S084, SPEC-S090, SPEC-S105, DS-S004, DS-S011, DS-S023, DS-S034, DS-S044, DS-S069, DS-S076, DS-S100, BD-S007, BD-S017, BD-S025, BD-S053, BD-S059*

### DES-496

source discovery adapterは全Discovered Test draft、ManagedTestDraftLink、SourceTargetDraft、Source Location、source range、current bytes、logical metadata、宣言された恒久SRC ID、Test execution descriptorをhash未計算で返す。

### DES-497

coreは出力を検証してTest subject / Source Target hashを計算してからManaged Test Entity、ManagedTestLink、Source Targetを具体化する。

### DES-498

Source Targetはcanonical locatorと任意の恒久SRC IDを併有する単一のentityである。

### DES-499

adapterは同一constructをlocator版とSrcId版の2 draftへ複製せず、恒久SRC IDを`SourceTargetDraft.src_id`として返す。

### DES-500

`SourceTargetDraft.target`は必ず`TargetRef::Locator`である。

### DES-501

SRC ID参照はcoreの統合済みSRC索引から、その恒久SRC IDを宣言したSource Targetのcanonical locatorへ解決する。

### DES-502

Relation writerは`REL-<ULID>`だけを生成する。

### DES-503

readerはファイル名とrecord IDが同じbare ULIDのversion 1互換Relationを読み取り、in-memoryで`REL-<ULID>`へ正規化するが、ファイルを書き換えない。

### DES-504

VO writerは`status`を保存せず、実効値をApprovalから導出する。

#### DES-S066 18.3.3 決定論的静的解析（oracle_presence・target_binding 静的到達）

*導出元: SPEC-S023, SPEC-S024, SPEC-S030, SPEC-S092, DS-S014, DS-S015, DS-S020, DS-S080, DS-S081, DS-S083, BD-S011, BD-S015, BD-S061, BD-S062*

### DES-505

正常Testは違反なしとなり、各違反fixtureは対応ruleで非PASSになる。

#### DES-S067 18.3.4 execution・Evidence（target_binding の証拠）

*導出元: SPEC-S025, SPEC-S056, SPEC-S057, SPEC-S083, SPEC-S099, DS-S016, DS-S038, DS-S039, DS-S064, DS-S089, DS-S101, BD-S012, BD-S023, BD-S024, BD-S049, BD-S066*

### DES-506

Evidence writerは中立fieldの`hashes.test_subject`と`hashes.targets[].target_construct`を出力する。

#### DES-S068 18.3.6 判断記録プロトコル（非ゲート）

*導出元: REQ-S035, SPEC-S036, SPEC-S039, SPEC-S093, DS-S026, DS-S027, DS-S084, BD-S063*

### DES-507

`vtest audit submit` の判断は少なくとも actor / subject / decision / judgment_kind を含み、理由・根拠（`reason` / `exclusions`）と `supersedes` は任意（optional）とする。

### DES-508

値域は `test-semantic` / `impl-consistency` / `case-coverage` であり、`subject` の値域は前 2 者が Test ID、`case-coverage` が Test ID または VO ID である。

#### DES-S069 18.3.7 承認と判断記録の分離

*導出元: SPEC-S017, SPEC-S039, SPEC-S048, SPEC-S081, SPEC-S082, DS-S008, DS-S027, DS-S034, DS-S062, DS-S063, BD-S048*

### DES-509

`approved_state` の値域は `approved` / `rejected` / `withdrawn` の 3 値である。

### DES-510

実効承認の導出は `approved_state` を参照する。

### DES-511

承認対象の値域は VO ID と document ID である。

### DES-512

承認主体は種別（`human` / `agent`）と識別子を記録する。

#### DES-S070 18.3.8 verify・report と scope

*導出元: SPEC-S018, SPEC-S019, SPEC-S059, SPEC-S060, SPEC-S061, SPEC-S076, SPEC-S104, SPEC-S106, DS-S009, DS-S010, DS-S041, DS-S042, DS-S043, DS-S056, DS-S099, DS-S102, BD-S009, BD-S041, BD-S072*

### DES-513

`verify` / `report` の JSON（CLI・MCP）は最上位に `scope` を返し、`scope.requested.items`（`--items` 省略時は固定4検査を4件すべて列挙）、`scope.requested.entities`（エンティティ軸無指定は空 list）、`scope.unverified_outside_scope`（検査軸4件未満またはエンティティ軸指定ありで `true`、完全検証で `false`）を持つ。

### DES-514

text treeのancestor continuation、middle child、last childを一意なbranch記号で描画する。

### DES-515

評価経路へそのような seam を導入する変更を行う場合は、正反対の判定を返す stub を注入しても 4 検査の結果が変化しないことを受入で確認する。

#### DES-S071 18.3.9 フェーズゲート評価

*導出元: REQ-S057, SPEC-S013, SPEC-S017, SPEC-S055, SPEC-S076, SPEC-S107, SPEC-S113, SPEC-S114, DS-S006, DS-S008, DS-S056, DS-S104, DS-S111, DS-S112, BD-S041, BD-S073*

### DES-516

ゲート定義は `config.yaml` の `gates` に、ゲート名と進行条件（`require.verification` ＝要求する検証結果、`require.approvals` ＝要求する承認ロール集合）として保持する。

### DES-517

`--gate` を指定した `verify` / `report` の JSON は `data.gate` に `name`・`verification.{required, actual, satisfied}`・`approvals[].{role, satisfied, missing_subjects}`・`satisfied` を返す。

#### DES-S072 18.3.10 Structured Test Operation

*導出元: SPEC-S043, SPEC-S085, DS-S029, DS-S071, BD-S019, BD-S054*

### DES-518

Form `kind`は`[a-z0-9][a-z0-9-]*`のcase-sensitive文字列で、built-inとuser-defined schemaを通してrepository全体で一意であり、schemaはowner `adapter` IDを別fieldで宣言する。

#### DES-S073 18.3.12 adapter contract

*導出元: SPEC-S007, SPEC-S070, SPEC-S085, SPEC-S113, DS-S002, DS-S051, DS-S071, DS-S111, BD-S004, BD-S033, BD-S054*

### DES-519

`vtest-adapter-api`は言語・runner非依存であり、Cargo、Rust parser、llvm-cov固有型を公開しない。

### DES-520

`vtest-model::TestEntity`はTestを関数として表現せず、adapter所有のTest constructを論理metadata、Source Location、content hash、ExecutionDescriptorで表現する。

### DES-521

`TargetRef::Locator`はadapter IDとadapter所有のopaque locatorを保持する。

### DES-522

`SourceLocation`はadapter ID、project-relative path、opaque locator、source rangeを保持する。

### DES-523

`TargetRef::Locator`と`SourceLocation`のどちらもRust module path、関数名、`.rs`拡張子をcoreの不変条件にしない。

### DES-524

`vtest-model::TestEntity`は`ExecutionDescriptor`だけを実行座標として持ち、`filter`、`package`、`test_target`、`TestTarget`を含まない。

### DES-525

`SourceDiscoveryAdapter`はhash未計算DTOを返し、coreがDTO検証・hash計算・domain entity具体化をこの順で行う。
