<!-- generated from docs/canonical/specification.json by build.py; do not edit -->

# 詳細設計

## DA-001 0. 本書の位置付け

### DES-001

本書は「基本仕様 v0.1」を実装可能なレベルまで具体化する。

### DES-002

本書は、基本仕様が定めた外部挙動の保証を変更しない。

### DES-003

本書と基本仕様の間に矛盾がある場合、基本仕様を正とし、本書の該当箇所を不整合として扱う。

### DES-004

本書は HOW（具体構文・アルゴリズム・データ構造・ID 命名・schema）を定める。

### DES-005

本書は、基本仕様（WHAT）に無い義務・検査・状態・文書種別・関係型を発明しない。

### DES-006

規範の伝播は上流から下流である。

*導出元: P-005*

*引用: 要件定義 P-005*

### DES-007

矛盾・不足を発見した場合は、本書を書き換えず上流へフィードバックしOwner判断を経る。

### DES-008

本書からの `基本仕様 §n` 参照は、再導出済み基本仕様 v0.1 の連番（§0〜§30）を指す。

### DES-009

本書からの `要件定義 §n` 参照は、凍結要件定義 v0.1 の連番（§1〜§28・P-001〜P-005・R-1〜R-5）を指す。

*導出元: R-1, R-5, P-001, P-005*

*引用: P-001, P-005, R-1, R-5*

### DES-010

正規の詳細設計は3分冊とする。

### DES-011

節番号は正規文書間を通した連番とする。

### DES-012

別紙Bは非正規のprocess文書として別に扱う。

### DES-013

本冊（コア設計）は正規であり、§1〜§11、§16、§17、§19を収録節とする。

> | 文書 | 位置付け | 収録節 |
> |---|---|---|
> | 本冊（コア設計） | 正規 | §1〜§11、§16、§17、§19 |
> | 別紙A（CLI・MCPインターフェース仕様） | 正規 | §12〜§15 |
> | 別紙B（実装計画） | 非正規 / process | 正規節番号を持たない |
> | 別紙C（受入仕様） | 正規 | §18 |

### DES-014

別紙A（CLI・MCPインターフェース仕様）は正規であり、§12〜§15を収録節とする。

### DES-015

別紙B（実装計画）は非正規/process文書であり、正規節番号を持たない。

### DES-016

別紙C（受入仕様）は正規であり、§18を収録節とする。

### DES-017

本冊の新設サブ節（§5.6 文書層孤児検出、§11.5 フェーズゲート、§11.6 役割別 projection、§11.7 判断待ち情報）は本冊の収録節範囲内に置き、別紙A / C の節番号を侵さない。

### DES-018

本書は、基本仕様が固定するCLIコマンド一覧・MCPツール一覧を増やさない。

*導出元: SPEC-665, SPEC-666, SPEC-667, SPEC-668, SPEC-669, SPEC-670, SPEC-671, SPEC-672, SPEC-673, SPEC-674, SPEC-675, SPEC-676, SPEC-677, SPEC-678, SPEC-679, SPEC-680, SPEC-681, SPEC-682, SPEC-683, SPEC-684, SPEC-685, SPEC-686, SPEC-687, SPEC-688, SPEC-689, SPEC-690, SPEC-691, SPEC-692, SPEC-693, SPEC-694, SPEC-695, SPEC-696, SPEC-697, SPEC-698, SPEC-699, SPEC-700, SPEC-701, SPEC-702, SPEC-703, SPEC-704, SPEC-705, SPEC-706, SPEC-707, SPEC-708, SPEC-709, SPEC-710*

*引用: 基本仕様 §26.1, 基本仕様 §26.2*

### DES-019

新設機能は既存コマンド・ツールの引数と出力で露出する。

### DES-020

引数・入出力の完全schemaは別紙Aが定める。

### DES-021

本書は意味論とデータschema、および露出点だけを確定する。

## DA-002 1. 実装構成

### DES-022

実装は Cargo workspace とする。

> vtest/
>   Cargo.toml            # workspace
>   crates/
>     vtest-model/        # エンティティ・状態モデル・ID型（依存最小）
>     vtest-store/        # .verify/ レコードファイルの読み書きとスキーマ検証
>     vtest-adapter-api/  # 言語・runner非依存のcapability契約とregistry
>     vtest-adapter-rust/ # rust-cargo discovery/static-analysis/operations/runner/coverage
>     vtest-scan/         # discovery委譲、結果統合、record整合性
>     vtest-audit/        # 静的解析委譲、判断記録bundle生成、提出結果検証
>     vtest-exec/         # runner委譲、revision取得、Evidence記録
>     vtest-verify/       # 整合性検査、鮮度検証、集約、レポート生成、ゲート評価
>     vtest-cli/          # バイナリ vtest（clap によるCLI、mcp サブコマンド含む）
>     vtest-mcp/          # MCPサーバ実装（vtest-cli から起動）
>   tests/
>     fixtures/           # 検証用サンプルプロジェクト（§18 受入基準で使用）

### DES-023

実装は単一バイナリ `vtest` を生成する。

### DES-024

`vtest-adapter-api` は `vtest-model` 以外の言語実装・Cargo実装へ依存しない。

### DES-025

`vtest-scan`、`vtest-audit`、`vtest-exec` はadapterを選択・委譲するorchestrationである。

### DES-026

`vtest-scan`、`vtest-audit`、`vtest-exec` は、それぞれが `syn`、`quote`、`rustc-demangle`、Cargo commandを直接所有しない。

### DES-027

`vtest-store` はForm Schemaの中立parserとcanonical保存だけを提供する。

### DES-028

組込Rust formの内容と配置は `vtest-adapter-rust` が所有する。

### DES-029

依存方向は `cli / mcp → verify / exec / audit / scan → store → model` を維持する。

### DES-030

言語固有能力の依存方向は `scan / audit / exec → adapter-rust → adapter-api → model` とする。

### DES-031

`adapter-rust → store` はForm Schemaとcanonical layoutの利用に限る。

### DES-032

Rust構文解析には `syn` 2.x（features: `full`, `extra-traits`, `visit`）を使用し、`vtest-adapter-rust` が所有するAST解析に用いる。

### DES-033

Rustスパン位置の特定には `proc-macro2`（feature: `span-locations`）を使用し、`vtest-adapter-rust` が所有する編集・ハッシュ対象範囲の特定に用いる。

### DES-034

CLIには `clap` 4.x（derive）を使用する。

### DES-035

シリアライズには `serde`、`serde_json` を使用する。

### DES-036

YAMLレコードファイルの処理には `serde_yaml` を使用する。

### DES-037

レコードIDには `ulid` を使用する。

### DES-038

内容ハッシュ（SHA-256）の計算には `sha2` を使用する。

### DES-039

Rust sourceの走査には `ignore` を使用し、`vtest-adapter-rust` が所有する `.gitignore` 準拠の走査に用いる。

### DES-040

エラー処理には、ライブラリでは `thiserror`、バイナリでは `anyhow` を使用する。

### DES-041

MCPには `rmcp`（公式 Rust MCP SDK）を使用し、stdio transportとする。

### DES-042

日時の扱いには `time` を使用し、RFC 3339形式とする。

### DES-043

Git 操作（HEAD の取得、dirty 判定）は `git` CLI の呼び出しで行う（`git rev-parse HEAD`、`git status --porcelain`）。

### DES-044

`git` が利用できない場合、リビジョンは特定できず、当該 Evidence はハッシュ束縛（revision 一致）を満たさないため `target_binding` の証拠として有効な `PASS` にならない（fail-closed）（§6）。

### DES-045

`git` が利用できない場合の失効は独立検査ではなく診断ラベル `STALE` として説明する（§11.2）。

### DES-046

内容ハッシュはSHA-256を使用する。

### DES-047

内容ハッシュは `sha256:<hex>` 形式で記録する。

### DES-048

subject固有規則でbyte-exactを要求しないテキストfragmentは、改行をLFへ統一する。

### DES-049

subject固有規則でbyte-exactを要求しないテキストfragmentは、各行の末尾空白を除去する。

### DES-050

改行の統一と各行末尾空白の除去以外の空白は正規化しない。

### DES-051

hash inputはdomain separatorと長さ付きfieldから構成する。

### DES-052

hash inputの各fieldは `field-name`、UTF-8 byte length、byte列の順にencodeする。

### DES-053

hash inputのencodeは単純な文字列連結を行わない。

### DES-054

hash inputにおいて、mapはkey昇順でencodeする。

### DES-055

hash inputにおいて、集合として扱う `covers`・`targets`・`related` は正規化値の昇順でencodeする。

### DES-056

hash inputにおいて、順序に意味がある `cases` は宣言順でencodeする。

### DES-057

hash inputにおいて、null、空文字、空listは異なる値としてencodeする。

### DES-058

Test subject hashはdomain `vtest:test-subject:v1` を用い、adapter ID、Test ID、全canonical metadata、Source Locationのadapter・project-relative path・opaque locator、ExecutionDescriptor、および正規化したTest construct bytesを束縛する。

### DES-059

Test subject hashは、byte range自体を前方の無関係な編集で変化するためhash inputにしない。

### DES-060

metadata宣言がmanifest等の非隣接箇所に存在しても、Test subject hashはadapterが返す論理metadataを同じsubjectへ含める。

### DES-061

canonical metadataの `targets` は宣言された `TargetRef` の正規化値を束縛し、解決後のcanonical Locatorへ置換しない。

### DES-062

canonical metadataの `targets` が宣言された `TargetRef` の正規化値を束縛し、解決後のcanonical Locatorへ置換しないことにより、Testの参照方法の変更（同一Source Targetへのlocator参照からSRC ID参照への書き換え等）はTest subject hashで捕捉される（§6.1.1）。

### DES-063

canonical metadataは `id` / `covers` / `targets` / `intent` / `input` / `expect` / `kind` / `cases` / `related` からなる。

### DES-064

canonical metadataは、`role` / `anchor` 等の存在理由分類 fieldを本versionでは持たない（§4.1）。

### DES-065

Test subject hashは、宣言の不在と空値の明示を異なる値としてencodeする。

### DES-066

Source Target hashはdomain `vtest:target-subject:v1` を用い、canonical Target Referenceとadapterが返すimplementation construct bytesを束縛する。

### DES-067

検証対象は一般概念であり、このhashは検証対象をSource Targetとして実現した形態のidentity束縛であって、coreが「検証対象とは何か」をSource Targetに限定して定義するものではない（§1.3・§4.1）。

*導出元: SPEC-332, SPEC-333, SPEC-334, SPEC-335, SPEC-336, SPEC-337, SPEC-338, SPEC-339*

*引用: 基本仕様 §9.1*

### DES-068

construct bytesへの束縛は当該実現形態（`rust-cargo` 等のSource Target形態）に対する規則であり、Source Targetを宣言しない検証対象形態へは適用しない。

### DES-069

canonical Target Referenceは常に `TargetRef::Locator`（adapter IDとadapter所有のopaque locator）であり、`TargetRef::SrcId` をcanonical Target Referenceにしない。

### DES-070

`TargetRef::SrcId` はSource Targetを参照する側の表現であって、Source Target自身の識別ではない。

### DES-071

恒久SRC IDはhash inputの独立fieldとして束縛せず、canonical Target Reference経由でもhash inputへ入らない。

### DES-072

恒久SRC IDの宣言・変更・削除はcanonical Target Referenceを変えない。

### DES-073

恒久SRC IDの宣言をSource Targetのconstruct bytesの内側へ置くadapter（`rust-cargo` の `@vtest.src-id` doc comment等）では、その宣言の追加・変更・削除がconstruct bytesを変化させ、construct bytes経由でSource Target hashが変化しうる（§5.5）。

### DES-074

恒久SRC IDの宣言をSource Targetのconstruct bytesの内側へ置くadapterで、その宣言の追加・変更・削除がconstruct bytesを変化させ、construct bytes経由でSource Target hashが変化しうることは正しい挙動であり、恒久SRC IDが独立したhash fieldであることを意味しない。

### DES-075

Source Target hashはSource Target自身のcanonical Locatorから一度だけ計算し、当該Source Targetを参照するTest側の `TargetRef` 綴りからは計算しない。

### DES-076

Evidence、検証は解決後のcanonical Source Targetのcanonical Locatorとhashへ束縛し、addressing modeごとに別subjectを作らない（§6.1）。

### DES-077

document subject hashはdomain `vtest:document-subject:v1` を用い、canonical document recordと参照先source（`path` の実ファイル）の正規化内容を束縛する。

### DES-078

document recordの `content_hash` と実sourceが不一致ならsubject hashは現在有効な値として成立せず、`chain_integrity` の非 `PASS`（`MISMATCH`、診断 `STALE`）とする（§11.4）。

### DES-079

document subject hashは、要件定義・基本仕様・詳細設計・API Schema等を種別で区別せず、すべて同一の総称document subjectとして計算する（§3.1）。

*導出元: SPEC-144, SPEC-145, SPEC-146, SPEC-147, SPEC-148, SPEC-149, SPEC-150, SPEC-151, SPEC-152, SPEC-153, SPEC-154, SPEC-155, SPEC-156, SPEC-157, SPEC-158, SPEC-159, SPEC-160, SPEC-161, SPEC-162, SPEC-163, SPEC-164*

*引用: 基本仕様 §3.2*

### DES-080

VO subject hashはdomain `vtest:record-subject:v1` を用い、readerが具体化したcanonical VO recordをfield規則に従ってencodeする。

### DES-081

VO subject hashは、VOの読取り互換field `status` を正典ではないため含めない。

### DES-082

VO subject hashは `derives_from`（参照先document ID集合）と `parent` を束縛する。

### DES-083

VO subject hashは、`covers` の増減をTest側subjectで捕捉するため含めない。

### DES-084

Execution State subject hashはdomain `vtest:execution-state:v1` を用い、adapter ID、snapshot schema ID / version、HEAD revision、runner kindとcanonical invocation projection、toolchain identity、実行結果へ影響するadapter configのcanonical projection、および実行可能状態を変えうるrepository / local dependency入力の完全なmanifestを束縛する。

### DES-085

manifest entryはstable root identity、root-relative path、input kind、byte-exact file bytesからなる。

### DES-086

manifest entry集合は正規化identity順にencodeする。

### DES-087

stable root identityはmachine上の絶対pathを用いず、workspace内の論理rootまたはdependency identityから決定論的に導出する。

### DES-088

adapterはhash未計算のmanifestと完全性を返し、coreが各entryとsubject全体を検証・hash化する。

### DES-089

adapterはsource location、source rangeと現在のbytes、解析済みlogical metadata、ExecutionDescriptorをhash未計算のdiscovery DTOとして返す。

### DES-090

coreはadapter出力と現在のsource bytesの対応を検証し、言語非依存encodingとSHA-256計算を行ってからdomain entityを具体化する。

### DES-091

adapterは、最終的な `TestEntity.content_hash` または `SourceTarget.content_hash` を返して自己確定してはならない。

### DES-092

coreはASTや言語固有構文からrangeを再計算しない。

### DES-093

Test Runner adapterも、実行状態へ用いたconfig / manifestをhash未計算DTOとして返す。

### DES-094

coreは現在bytesとの対応、重複、集合完全性、schema versionを検証してsubject hashを計算する。

### DES-095

adapterは、完全性を保証できないDTOから `PASS` 用subjectを具体化してはならない。

### DES-096

静的解析は正典レコードを持たず、検証のたびに現在のsource / configから再計算する派生情報である（§7・§7.1）。

*導出元: P-003*

*引用: 基本仕様 P-003*

### DES-097

静的解析結果は内容ハッシュに束縛された永続subjectを持たず、hash体系に静的解析専用のsubjectを設けない。

### DES-098

`rust-cargo` adapterはTest constructとして、metadata doc commentを除き、実行に影響する属性、signature、bodyを含む関数itemのbytesを返す。

### DES-099

doc comment由来metadataはlogical metadataと `metadata_sources` として別に返す。

### DES-100

`rust-cargo` adapterはSource Targetには属性とdoc commentを含む関数item全体を返す。

### DES-101

format変更を構文上の意味だけから同値とみなさず、正規化後のsource bytesが変化した場合は安全側でSTALEにする。

## DA-003 2. データディレクトリと設定

### DES-102

基本仕様 §24.1 の layout をそのまま採用する。

*導出元: SPEC-618, SPEC-619, SPEC-620, SPEC-621, SPEC-622, SPEC-623, SPEC-624, SPEC-625, SPEC-626, SPEC-627, SPEC-628, SPEC-629, SPEC-630*

*引用: 基本仕様 §24.1*

### DES-103

`.verify/` 直下に `config.yaml` を置く。

### DES-104

`.verify/doc/` は `DOC-<NAME>.yaml` 形式で総称documentレコード（正典）を格納する。

### DES-105

`.verify/vo/` は `VO-<NAME>.yaml` 形式でVOレコード（正典）を格納する。

### DES-106

`.verify/rel/` は `REL-<ULID>.yaml` 形式で外部Relationレコード（正典・不変）を格納する。

### DES-107

`.verify/forms/` は `<kind>.yaml` 形式でForm Schema（正典）を格納する。

### DES-108

`.verify/decisions/` は `<ULID>.yaml` 形式で判断記録（事実・追記型）を格納する。

### DES-109

`.verify/approvals/` は `<ULID>.yaml` 形式で承認記録（事実・追記型）を格納する。

### DES-110

`.verify/evidence/` は `<ULID>.yaml` 形式で実行証拠レコード（事実・追記型）を格納する。

### DES-111

`.verify/cache/` はGit管理外とし、`.verify/.gitignore` に `cache/` を出力する。

### DES-112

`.verify/cache/bundles/` は判断バンドルJSON（派生・再生成可能）を格納する。

### DES-113

`.verify/cache/logs/` はテスト実行の生ログを格納する。

### DES-114

`.verify/cache/cov/` はcoverage生出力を格納する。

### DES-115

文書種別ごとの専用ディレクトリ（旧 `spec/` / `req/`）を設けず、上流文書はすべて `doc/` の総称documentレコード1種で表現する。

*導出元: SPEC-121, SPEC-122, SPEC-123, SPEC-124, SPEC-125, SPEC-126, SPEC-127, SPEC-128, SPEC-129, SPEC-130, SPEC-131, SPEC-132, SPEC-133, SPEC-134, SPEC-135, SPEC-136, SPEC-137, SPEC-138, SPEC-139, SPEC-140, SPEC-141, SPEC-142, SPEC-143, SPEC-144, SPEC-145, SPEC-146, SPEC-147, SPEC-148, SPEC-149, SPEC-150, SPEC-151, SPEC-152, SPEC-153, SPEC-154, SPEC-155, SPEC-156, SPEC-157, SPEC-158, SPEC-159, SPEC-160, SPEC-161, SPEC-162, SPEC-163, SPEC-164*

*引用: 基本仕様 §3.1, 基本仕様 §3.2*

### DES-116

決定論的解析の結果を保存する正典ディレクトリ（旧 `audits/`）を設けない。

### DES-117

静的解析は再計算派生であり `cache/` にのみ置く（§7.1）。

### DES-118

外部判断は `decisions/` の判断記録として保存する。

### DES-119

`vtest init` は上記ディレクトリ、`config.yaml` の雛形、`.verify/.gitignore`、組込 Form Schema を生成する。

*引用: 別紙A §14*

### DES-120

`config.yaml` writerの正規形はversion 2とし、adapterごとにroot・scan・run設定をnamespace化する。

> version: 2
> project:
>   name: example
> adapters:
>   - id: rust-cargo
>     roots: ["."]
>     scan:
>       include: [src, tests, crates]   # テストコード走査パス。省略時はワークスペース全体
>       assertion_macros: []            # 追加で assert 相当として扱うマクロ名
>     run:
>       coverage: llvm-cov              # target_binding 動的計測方式: llvm-cov | off
> doc:
>   roots: [DOC-REQ-ROOT]               # orphan_detection の除外根（§5.6、基本仕様 §5.2）
> verify:
>   full_scope: [chain_integrity, orphan_detection, target_binding, oracle_presence]
> gates:                                # フェーズゲート定義（§11.5、基本仕様 §20）
>   - name: development
>     require: { verification: PASS }
>   - name: release
>     require: { verification: PASS, approvals: [reviewer] }
>   - name: delivery
>     require: { verification: PASS, approvals: [owner] }

### DES-121

`config.yaml` readerはversion 1を単一の `rust-cargo` adapter設定としてin-memory変換して読み取るが、読み取りだけで正典を書き換えない。

*導出元: SPEC-101, SPEC-102, SPEC-103, SPEC-104, SPEC-105, SPEC-106, SPEC-107, SPEC-108, SPEC-109, SPEC-110, SPEC-111, SPEC-112, SPEC-113, SPEC-114, SPEC-115, SPEC-116, SPEC-117, SPEC-118, SPEC-119, SPEC-120*

*引用: 基本仕様 §2.4*

### DES-122

`config.yaml` の各adapterの `scan` 設定の `include` はテストコード走査パスであり、省略時はワークスペース全体を対象とする。

### DES-123

`config.yaml` の各adapterの `scan` 設定の `assertion_macros` は、追加でassert相当として扱うマクロ名を指定する。

### DES-124

`config.yaml` の各adapterの `run` 設定の `coverage` は `target_binding` の動的計測方式を指定し、値は `llvm-cov` または `off` である。

### DES-125

adapter IDの重複、同一adapter内のroot重複、未知adapter、無効なadapter設定はusage error（E-CONFIG-001）とする。

### DES-126

異なるadapterが同じrootを共有することはpolyglot repositoryのために許可する。

### DES-127

統合したTest IDは全adapterでglobal uniquenessを検査する。

### DES-128

adapter固有設定の検証は登録adapterへ委譲する。

### DES-129

coreは未知のnamespaceや値をRust設定として解釈しない。

### DES-130

`vtest init` はversion 2を生成する。

### DES-131

`verify.full_scope` は利用者が完全検証を縮小する設定ではなく、基本仕様 §5 の固定4検査（`chain_integrity` / `orphan_detection` / `target_binding` / `oracle_presence`）を列挙するconfig invariantである。

*導出元: SPEC-217, SPEC-218, SPEC-219, SPEC-220, SPEC-221, SPEC-222, SPEC-223, SPEC-224, SPEC-225, SPEC-226, SPEC-227, SPEC-228, SPEC-229, SPEC-230, SPEC-231, SPEC-232, SPEC-233, SPEC-234, SPEC-235, SPEC-236, SPEC-237, SPEC-238, SPEC-239, SPEC-240, SPEC-241, SPEC-242, SPEC-243, SPEC-244, SPEC-245, SPEC-246, SPEC-247, SPEC-248, SPEC-249, SPEC-250, SPEC-251, SPEC-252, SPEC-253, SPEC-254, SPEC-255, SPEC-256, SPEC-257, SPEC-258, SPEC-259, SPEC-260, SPEC-261, SPEC-262, SPEC-263, SPEC-264, SPEC-265, SPEC-266, SPEC-267, SPEC-268, SPEC-269, SPEC-270, SPEC-271, SPEC-272, SPEC-273, SPEC-274, SPEC-275, SPEC-276, SPEC-277, SPEC-278, SPEC-279, SPEC-280, SPEC-281, SPEC-282, SPEC-283, SPEC-284, SPEC-285, SPEC-286*

*引用: 基本仕様 §5*

### DES-132

version 2では、`verify.full_scope` の重複・未知項目・欠落・余剰をE-CONFIG-001で拒否する。

### DES-133

version 1では、`verify.full_scope` のfield欠落を固定4検査として具体化し、重複または未知項目はE-CONFIG-001で拒否する。

### DES-134

旧12項目の列挙（`spec_coverage` / `test_existence` 等）は現行invariantに違反するため、versionを問わずE-CONFIG-001とし、in-memory補完で受理しない。

### DES-135

`--items` による明示的な部分集合だけを限定scopeとして扱い、項目指定を省略したCLI / MCP検証は常に固定4検査を評価する。

### DES-136

いかなる設定値も完全検証の検査を4本未満へ縮退させない（§22.1）。

*導出元: SPEC-210, SPEC-211, SPEC-212, SPEC-213, SPEC-214, SPEC-215, SPEC-216*

*引用: 基本仕様 §4.6*

### DES-137

`gates` はフェーズゲートの進行条件定義を保持する（§11.5）。

### DES-138

config読込み時に次を検査し、いずれか違反があればE-CONFIG-001（終了コード2）として設定を受理せず、検証結果を生成しない。

### DES-139

`gates` field自体の欠落と空listは「ゲート定義なし」として受理する。

### DES-140

`--gate` を指定しない実行は、`gates` field自体の欠落と空listの影響を受けない。

### DES-141

`--gate` に未定義名を指定した場合の扱いは §11.5 で定める。

### DES-142

`gates[].name` は非空文字列であり、大文字小文字を区別した完全一致で重複してはならない。重複した場合はE-CONFIG-001（終了コード2）とする。

### DES-143

`--gate <name>` の解決は `gates[].name` と同じ大文字小文字を区別した完全一致で行う（§11.5）。

### DES-144

`gates[].require` は必須とする。欠落はE-CONFIG-001（終了コード2）とする。

### DES-145

`gates[].require` の `verification` は必須とする。欠落はE-CONFIG-001（終了コード2）とする。

### DES-146

`require.verification` の値は、基本仕様 §4.1 の5状態語彙（`PASS` / `FAIL` / `MISMATCH` / `NO_EVIDENCE` / `UNKNOWN`）のいずれかと大文字小文字を区別して完全一致しなければならない。

*導出元: SPEC-174, SPEC-175, SPEC-176, SPEC-177, SPEC-178, SPEC-179, SPEC-180, SPEC-181, SPEC-182, SPEC-183, SPEC-184, SPEC-185, SPEC-186*

*引用: 基本仕様 §4.1*

### DES-147

診断ラベル（`MISSING` / `NOT_EXECUTED` / `NOT_CHECKED` / `STALE`）、`OK` / `NG`、旧12項目名、5状態の小文字表記・別綴り、list・objectなどの非文字列値を `require.verification` に指定した場合はすべてE-CONFIG-001（終了コード2）とする。

### DES-148

5状態のうち `PASS` 以外を要求する `require.verification` の定義自体は受理し、充足判定の意味は §11.5 で定める。

### DES-149

`require.approvals` は省略可能とし、省略は「要求する承認ロールなし（空集合）」として受理する。

### DES-150

`require.approvals` を指定する場合は文字列ロール名のlistとし、空文字列・重複ロール名はE-CONFIG-001（終了コード2）とする。

### DES-151

`require.approvals` のロール名が `approval_roles` に解決できない場合もE-CONFIG-001（終了コード2）とする。

*引用: 別紙A §12.3*

### DES-152

`doc.roots` は orphan_detection の除外根をDOC IDの集合として保持する（§5.6）。

### DES-153

`scan` と `run` はversion 1 schema互換のwire値とする。

### DES-154

Rust固有のmacro pathや `llvm-cov` 制約は `rust-cargo` adapterに限って適用する。

### DES-155

非Rust namespaceの値をcoreがRust設定として推測・書換えしてはならない。

### DES-156

検証グラフとindexは実行のたびにインメモリで再構築する。

### DES-157

永続cacheを正典または検証入力として使用しない。

### DES-158

`cache/` は再生成可能な派生物（判断バンドル、静的解析結果、実行ログ、coverage生出力）だけを格納する。

### DES-159

MCPサーバは長時間動作するため、ツール呼び出しごとに対象ファイルのmtimeを確認し、変化があれば再スキャンする。

## DA-004 3. レコードファイルスキーマ

### DES-160

すべてのレコードはYAMLとする。

### DES-161

レコードの未知フィールドはエラーではなく警告とする。

### DES-162

`id` とファイル名（拡張子除く）は一致しなければならない。

### DES-163

上流文書はすべて単一の総称ノード型 `document` で表現する。

*導出元: SPEC-121, SPEC-122, SPEC-123, SPEC-124, SPEC-125, SPEC-126, SPEC-127, SPEC-128, SPEC-129, SPEC-130, SPEC-131, SPEC-132, SPEC-133, SPEC-134, SPEC-135, SPEC-136, SPEC-137, SPEC-138, SPEC-139, SPEC-140, SPEC-141, SPEC-142, SPEC-143*

*引用: 基本仕様 §3.1*

### DES-164

要件定義・基本仕様・詳細設計・API Schema・Protocol Specification・型/データ仕様・DB schema・その他の機械可読仕様を種別で区別する専用スキーマを持たない。

### DES-165

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

### DES-166

document レコードの `content_hash` fieldは登録時の内容ハッシュである（§1.3）。

### DES-167

document レコードの `title` fieldは任意の表示名である。

### DES-168

document レコードの `derives_from` fieldは上流documentへの導出リンクであり、0件も許容する（0件は根候補を意味する）。

### DES-169

document レコードの `derives_from` entryの `anchor` fieldは任意の上流該当箇所（節番号等）であり、空も許容し、`chain_integrity` の `MISMATCH` としない。

### DES-170

document レコードの `derives_from` entryの `note` fieldは任意の導出理由であり、空も許容し、`chain_integrity` の `MISMATCH` としない。

*引用: 基本仕様 §3.4*

### DES-171

`derives_from` は上流documentへの唯一のリンク種別である。

*導出元: SPEC-144, SPEC-145, SPEC-146, SPEC-147, SPEC-148, SPEC-149, SPEC-150, SPEC-151, SPEC-152, SPEC-153, SPEC-154, SPEC-155, SPEC-156, SPEC-157, SPEC-158, SPEC-159, SPEC-160, SPEC-161, SPEC-162, SPEC-163, SPEC-164*

*引用: 基本仕様 §3.2*

### DES-172

文書層の段（要件→仕様→詳細設計…）は `derives_from` リンクで表現し、段を増やしても種別を増やさない。

### DES-173

リンクを追加してもスキーマは壊れない。

### DES-174

各 `derives_from` entryの `note`（導出理由・説明文）は任意であり、空でも `chain_integrity` 違反・`MISMATCH` としてはならない（§19）。

*引用: 基本仕様 §3.4*

### DES-175

`note` は付加・保存できる構造とする。

### DES-176

各 `derives_from` entryの `anchor`（参照先document内の該当箇所を指す文字列。節番号・条項番号・見出し等）は任意であり、欠落・空文字列を `chain_integrity` 違反・`MISMATCH` としてはならない（§19）。

*引用: 基本仕様 §3.4*

### DES-177

`anchor` の値は不透明な文字列として保存・出力するだけであり、本システムは `anchor` を `path` の実ファイル内位置へ解決せず、実在・一意性・書式を検証しない。

### DES-178

`anchor` の内容不一致を検出する検査・診断コードは存在しない。

### DES-179

同一 `doc` を指す複数entryを `anchor` 違いで持つことを許容し、重複としない。

### DES-180

`anchor` は `derives_from` entryのfieldであり、Test metadataには存在しない（§4.1）。

*導出元: SPEC-409, SPEC-410, SPEC-411, SPEC-412, SPEC-413, SPEC-414, SPEC-415, SPEC-416, SPEC-417, SPEC-418, SPEC-419, SPEC-420, SPEC-421, SPEC-422, SPEC-423, SPEC-424, SPEC-425*

*引用: 基本仕様 §12*

### DES-181

`anchor` はcanonical document recordの一部であり、document subject hashの入力に含まれる（§1.3）。

### DES-182

`anchor` だけの変更は `path` の実ファイルを変えないため `content_hash` を変化させないが、document subject hashを変化させるため、当該documentを上流依存closureに含む判断記録・承認は失効する（§3.5・§8.5）。

### DES-183

`derives_from` の参照先documentが存在しない場合は、文書鎖のリンク切れとして `chain_integrity` の `MISMATCH` とする。

### DES-184

`path` の実ファイルが `content_hash` と一致しなくなった場合は `chain_integrity` の `MISMATCH`（診断 `STALE`）とする（§11.4）。

### DES-185

`derives_from` が空のdocumentは根候補であり、`config.yaml` の `doc.roots` に列挙されない場合は孤児として `orphan_detection` の `MISMATCH` とする（§5.6）。

### DES-186

仕様文書そのものは `.verify/` へ複製しない。

### DES-187

本システムは文書内容の意味的良否を検証しない。

*導出元: SPEC-729, SPEC-730, SPEC-731, SPEC-732, SPEC-733, SPEC-734, SPEC-735, SPEC-736, SPEC-737, SPEC-738, SPEC-739*

*引用: 基本仕様 §29 OOS-001*

### DES-188

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

### DES-189

VO レコードの `derives_from` fieldは1件以上のdocumentへの直結を表す。

*導出元: SPEC-144, SPEC-145, SPEC-146, SPEC-147, SPEC-148, SPEC-149, SPEC-150, SPEC-151, SPEC-152, SPEC-153, SPEC-154, SPEC-155, SPEC-156, SPEC-157, SPEC-158, SPEC-159, SPEC-160, SPEC-161, SPEC-162, SPEC-163, SPEC-164*

*引用: 基本仕様 §3.2*

### DES-190

VO レコードの `derives_from` entryの `anchor` fieldは任意の上流該当箇所（節番号等）であり、空も許容し、`chain_integrity` 違反・`MISMATCH` としない。

### DES-191

VO レコードの `derives_from` entryの `note` fieldは任意であり、空も許容し、`chain_integrity` 違反・`MISMATCH` としない。

### DES-192

VO レコードの `dimensions` fieldは検証軸であり、任意である（§3.2.1）。

### DES-193

VO レコードの `coverage_policy` fieldの値域は `independent-axes` / `full-product` / `explicit` / `null` である。

### DES-194

VO レコードの `combinations` fieldは `coverage_policy: explicit` のとき実体化する組合せである（§3.2.1）。

### DES-195

VO レコードの `representative_cases` fieldは代表入力値であり、任意である。

### DES-196

VOは1件以上の `document` から `derives_from` で導出される。

### DES-197

VOとdocumentの間に他のエンティティ層を置かない（§3.2）。

*導出元: SPEC-023, SPEC-024, SPEC-025, SPEC-026, SPEC-027, SPEC-028, SPEC-029, SPEC-030, SPEC-031, SPEC-032, SPEC-033, SPEC-034, SPEC-035, SPEC-036, SPEC-037, SPEC-038, SPEC-039, SPEC-040, SPEC-041, SPEC-042, SPEC-043, SPEC-044, SPEC-045, SPEC-046, SPEC-047, SPEC-048, SPEC-049, SPEC-050, SPEC-051, SPEC-052, SPEC-053, SPEC-054, SPEC-055, SPEC-056, SPEC-057, SPEC-058, SPEC-059, SPEC-060, SPEC-061, SPEC-062, SPEC-063, SPEC-064, SPEC-065, SPEC-066, SPEC-067, SPEC-068, SPEC-069*

*引用: 基本仕様 §1*

### DES-198

VOは旧モデルの `requirements`（REQ参照）と `spec_refs`（SPEC + 節参照）は持たず、上流参照は `derives_from:[DOC-]` へ一本化する。

### DES-199

`derives_from` の参照先documentが存在しなければ、`chain_integrity` の `MISMATCH`（dangling reference。E-SCAN-003相当はE-SCAN-012）とする（§5.4）。

### DES-200

VOの `derives_from` entryもdocumentレコードと同じく任意の `anchor`（参照先document内の該当箇所を指す不透明な文字列。節番号・条項番号・見出し等）と任意の `note` を持つ。

### DES-201

VOの `derives_from` entryの `anchor` / `note` の欠落・空文字列は `chain_integrity` 違反・`MISMATCH` としない（§19）。

*引用: 基本仕様 §3.4*

### DES-202

本システムはVOの `anchor` を文書内位置へ解決せず、実在・一意性・書式を検証せず、内容不一致を検出する検査を持たない。

### DES-203

同一 `doc` を `anchor` 違いで複数entryとして持つことを許容し、重複としない。

### DES-204

「どの上流条項がどのVOへ対応するか」の対応ペアは、`anchor` 付き `derives_from` エッジとして保持し、§11.6のprojection出力で露出する。

*導出元: SPEC-379, SPEC-380, SPEC-381, SPEC-382, SPEC-383*

*引用: 基本仕様 §11.1*

### DES-205

`anchor` と `note` はVO subject hashの入力に含まれない（VO subject hashは `derives_from` の参照先document ID集合を束縛する）（§1.3）。

### DES-206

`anchor` だけの変更でVOの承認・判断記録は失効しない。

### DES-207

参照先document集合そのものの変更はVOの承認・判断記録を従来どおり失効させる。

### DES-208

VOの `status` は承認レコードから導出する表示値であり、canonical writerはVO recordへ保存しない。

### DES-209

readerは読取り互換fieldとして `status` を受理するが、実効判定とVO subject hashでは無視し、存在自体をW-STORE-001として通知する。

### DES-210

互換field値と導出値が異なる場合も導出値だけを使用する。

### DES-211

`dimensions` を持つ VO は、`vtest vo expand VO-X` により子 VO を実体化できる。

> dimensions:
>   - name: operand-sign
>     partitions: [positive, negative]
>   - name: operator
>     partitions: [add, sub, mul, div]
> coverage_policy: full-product

### DES-212

`independent-axes` はpartitionごとに1子VOを生成する（上例では2+4=6件）。

### DES-213

`full-product` は直積ごとに1子VOを生成する（上例では8件）。

### DES-214

`explicit` は `combinations` フィールドに列挙された組合せのみを生成する。

### DES-215

生成される子VOのIDは `VO-X-<PARTITION>`（直積は `VO-X-<P1>-<P2>`）を既定とする。

### DES-216

子VO生成は、生成前に一覧を提示して `--dry-run` で確認できる。

### DES-217

子VO IDのsuffixはpartition値を大文字化した文字列とし、複数軸のsuffixは `dimensions` の宣言順に連結する。

### DES-218

子VO ID生成は `combinations` entry内の記述順・map key順には依存しない。

### DES-219

実体化後は通常のVOとして扱われるため、`chain_integrity` のleaf VO → Test検査は「leaf VOにcoversするTestが存在するか」だけを見ればよい。

### DES-220

組合せ空間の定義が仕様に対して十分かは本システムの検査ではなく、`UNKNOWN` としてエスカレーションの領分である（§8）。

*導出元: SPEC-358, SPEC-359, SPEC-360, SPEC-361, SPEC-362, SPEC-363, SPEC-364, SPEC-365, SPEC-366, SPEC-367, SPEC-368, SPEC-369, SPEC-370, SPEC-371, SPEC-372, SPEC-373, SPEC-374, SPEC-375, SPEC-376, SPEC-377, SPEC-378, SPEC-379, SPEC-380, SPEC-381, SPEC-382, SPEC-383, SPEC-384, SPEC-385, SPEC-386, SPEC-387, SPEC-388, SPEC-389, SPEC-390, SPEC-391, SPEC-392, SPEC-393, SPEC-394, SPEC-395, SPEC-396, SPEC-397, SPEC-398, SPEC-399, SPEC-400, SPEC-401, SPEC-402, SPEC-403, SPEC-404, SPEC-405, SPEC-406, SPEC-407, SPEC-408*

*引用: 基本仕様 §11, 基本仕様 §10*

### DES-221

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

*導出元: SPEC-358, SPEC-359, SPEC-360, SPEC-361, SPEC-362, SPEC-363, SPEC-364, SPEC-365, SPEC-366, SPEC-367, SPEC-368, SPEC-369, SPEC-370, SPEC-371, SPEC-372, SPEC-373, SPEC-374, SPEC-375*

*引用: 基本仕様 §10*

### DES-222

`combinations` の各entryはdimension名→partition値のmapとし、`dimensions` に宣言された全軸をちょうど1回ずつ持つ。

### DES-223

上例の `explicit` 実体化は `VO-X-POSITIVE-DIV` と `VO-X-NEGATIVE-DIV` の2件を生成する。

### DES-224

`vo expand` は子VOを1件も生成せず、部分生成もしない。

### DES-225

`coverage_policy: explicit` かつ `combinations` が欠落、`null`、または空listである場合、当該VOレコードは `combinations` 不正とし、`E-SCAN-017` を報告して当該VOの `chain_integrity` を `MISMATCH` とする（§17.1）。

### DES-226

`coverage_policy: explicit` かつ `dimensions` が空である場合、当該VOレコードは `combinations` 不正とし、`E-SCAN-017` を報告して当該VOの `chain_integrity` を `MISMATCH` とする（§17.1）。

### DES-227

`combinations` が空listでないのに `coverage_policy` が `explicit` 以外（`independent-axes` / `full-product` / `null`）である場合、当該VOレコードは `combinations` 不正とし、`E-SCAN-017` を報告して当該VOの `chain_integrity` を `MISMATCH` とする（§17.1）。

### DES-228

entryが `dimensions` に宣言されていないdimension名を含む場合、当該VOレコードは `combinations` 不正とし、`E-SCAN-017` を報告して当該VOの `chain_integrity` を `MISMATCH` とする（§17.1）。

### DES-229

entryのpartition値が当該dimensionの `partitions` に列挙されていない場合、当該VOレコードは `combinations` 不正とし、`E-SCAN-017` を報告して当該VOの `chain_integrity` を `MISMATCH` とする（§17.1）。

### DES-230

entryが宣言済みdimensionのいずれかを欠く、または同じdimension名を2回以上持つ場合、当該VOレコードは `combinations` 不正とし、`E-SCAN-017` を報告して当該VOの `chain_integrity` を `MISMATCH` とする（§17.1）。

### DES-231

同一の（dimension名→partition値）対応を持つentryが2件以上ある（重複tuple）場合、当該VOレコードは `combinations` 不正とし、`E-SCAN-017` を報告して当該VOの `chain_integrity` を `MISMATCH` とする（§17.1）。

### DES-232

`combinations` はcanonical VO recordの一部であり、VO subject hashに束縛される（§1.3）。

### DES-233

`combinations` の変更は当該VOの承認を失効させる（§3.5）。

### DES-234

`combinations` の値が仕様に対して十分な組合せ集合かは本システムの検査ではなく、エスカレーションの領分である。

*導出元: SPEC-358, SPEC-359, SPEC-360, SPEC-361, SPEC-362, SPEC-363, SPEC-364, SPEC-365, SPEC-366, SPEC-367, SPEC-368, SPEC-369, SPEC-370, SPEC-371, SPEC-372, SPEC-373, SPEC-374, SPEC-375, SPEC-376, SPEC-377, SPEC-378, SPEC-379, SPEC-380, SPEC-381, SPEC-382, SPEC-383, SPEC-384, SPEC-385, SPEC-386, SPEC-387, SPEC-388, SPEC-389, SPEC-390, SPEC-391, SPEC-392, SPEC-393, SPEC-394, SPEC-395, SPEC-396, SPEC-397, SPEC-398, SPEC-399, SPEC-400, SPEC-401, SPEC-402, SPEC-403, SPEC-404, SPEC-405, SPEC-406, SPEC-407, SPEC-408*

*引用: 基本仕様 §10, 基本仕様 §11*

### DES-235

Relationは、どちらか一方のエンティティに自然に所属しない関係（VO間の依存、Test間の補完関係など）だけを保存する。

*導出元: SPEC-094, SPEC-095, SPEC-096, SPEC-097, SPEC-098, SPEC-099, SPEC-100*

*引用: 基本仕様 §2.3*

### DES-236

`derives_from`・`covers`・`targets` はadapter所有の宣言またはdocument / VO recordから導出できるため、外部Relationとして重複保存しない。

### DES-237

Relationレコードの `type` fieldの値域は `depends-on` / `supersedes` / `regression-for` / `derived-from` / `same-partition` / `complements` / `conflicts-with` である。

> id: REL-01J8XVZK3Q...
> type: depends-on          # depends-on | supersedes | regression-for |
>                           # derived-from | same-partition | complements | conflicts-with
> from: TEST-PARSER-044     # 任意のエンティティID
> to: TEST-PARSER-012
> note: ""                  # 任意の説明文
> created: 2026-08-08T00:00:00Z

### DES-238

Relationレコードの `from` fieldは任意のエンティティIDである。

### DES-239

Relationレコードの `note` fieldは任意の説明文である。

### DES-240

canonical Relation IDは `REL-` と26文字のULID payloadからなる。

### DES-241

writerは `.verify/rel/REL-<ULID>.yaml` と同値の `id` だけを生成する。

### DES-242

readerはversion 1互換入力として `.verify/rel/<ULID>.yaml` かつ同値のbare `id` を受理し、`REL-<ULID>` へin-memoryで正規化するが、読み取りだけでファイルを書き換えない。

### DES-243

prefixed / bareの混在、ファイル名と `id` のpayload不一致、または同じpayloadの複数recordはE-SCAN-010とし、いずれかを選ばない。

### DES-244

Relationは不変である。

### DES-245

Relationの変更はファイル削除＋新規作成で表す。

### DES-246

`from` / `to` の存在はスキャン時に検査し、不在はE-SCAN-009、`chain_integrity` の `MISMATCH` とする。

### DES-247

判断記録は、`UNKNOWN` に対して外部（人間または判断可能Agent）が下した判断の記録である。

*導出元: REQ-180, REQ-181, REQ-182, REQ-183, REQ-184, REQ-185, REQ-186, REQ-187, REQ-188, REQ-189, REQ-190, REQ-191, REQ-192, SPEC-387, SPEC-388, SPEC-389, SPEC-390, SPEC-391, SPEC-392, SPEC-393, SPEC-394, SPEC-395, SPEC-396, SPEC-397, SPEC-398, SPEC-399, SPEC-400, SPEC-401, SPEC-402, SPEC-403, SPEC-404, SPEC-405, SPEC-406, SPEC-407, SPEC-408*

*引用: 基本仕様 §11.3, 要件定義 §12*

### DES-248

判断記録は actor / subject / decision / judgment_kind を必須項目とする。

### DES-249

判断記録の理由・根拠は任意とする。

### DES-250

判断記録は依存closureのハッシュに束縛される。

### DES-251

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

### DES-252

判断記録の `judgment_kind` fieldは判断型であり、必須とする（§8.1）。

### DES-253

判断記録の `supersedes` fieldは明示に置き換える旧判断記録のULID listであり、既定は空listとする（§8.5）。

### DES-254

判断記録の `subject_hash` fieldは判断時点の対象の内容ハッシュである。

### DES-255

判断記録の `dependencies` fieldは判断時点の上流依存closureであり、完全一致を要求する。

### DES-256

判断記録の `dependencies` entryの `hash` fieldはdocument subject hashである（§1.3）。

### DES-257

判断記録の `actor` fieldは誰が判断したかを表し、必須とする。

### DES-258

判断記録の `actor` の `kind` fieldの値域は `human` / `agent` である。

### DES-259

判断記録の `actor` の `model` fieldはagentの場合任意とする。

### DES-260

判断記録の `decision` fieldはどう判断したかを表し、必須とする。値の妥当性は §8.4 で定める。

### DES-261

判断記録の `reason` fieldは理由・根拠・evidence noteであり、任意とし、空でも無効化しない。

### DES-262

判断記録の `exclusions` fieldは対象外とした範囲であり、任意とする。

### DES-263

理由が空であることだけを根拠に、その判断を無効・`UNKNOWN`・`NO_EVIDENCE`・`MISMATCH` 等として扱ってはならない。

*導出元: REQ-180, REQ-181, REQ-182, REQ-183, REQ-184, REQ-185, REQ-186, REQ-187, REQ-188, REQ-189, REQ-190, REQ-191, REQ-192, SPEC-387, SPEC-388, SPEC-389, SPEC-390, SPEC-391, SPEC-392, SPEC-393, SPEC-394, SPEC-395, SPEC-396, SPEC-397, SPEC-398, SPEC-399, SPEC-400, SPEC-401, SPEC-402, SPEC-403, SPEC-404, SPEC-405, SPEC-406, SPEC-407, SPEC-408*

*引用: 基本仕様 §11.3, 要件定義 §12*

### DES-264

`reason` / `exclusions` はoptionalである。

### DES-265

同一対象への判断記録は複数存在してよい（再判断・多重判断）。

### DES-266

判断記録の有効性判定と実効判断の決定は §8.5 に従う。

### DES-267

`judgment_kind` は判断対象を一意に区切る第二のkeyである。

### DES-268

実効判断は `subject` 単独ではなく `(subject, judgment_kind)` の組ごとに独立に決まり、判断型の異なる判断記録どうしは競合しない（§8.5）。

### DES-269

`judgment_kind` を欠くか §8.1 の値域外の判断記録は、履歴として保持するがいずれの `(subject, judgment_kind)` の実効判断へも寄与させず、W-STORE-003を出す。

### DES-270

`supersedes` は、この判断記録が明示に置き換える旧判断記録のULIDを名指しするlistである。

### DES-271

`supersedes` に列挙する各ULIDは、同一 `subject` かつ同一 `judgment_kind` の既存判断記録を指さなければならない。提出時の検証は §8.4、読取り時の扱いは §8.5 に従う。

### DES-272

`supersedes` はRelationとは独立であり、`type: supersedes` のRelationレコードは実効判断の決定に用いない（§3.3）。

### DES-273

`subject` の `target` 参照は §6.1 で解決したcanonical Source Targetのcanonical Locatorとし、解決できないtargetを任意の候補で埋めない（§6.1.1）。

### DES-274

判断記録の受理は当該対象の検証状態（§4.1の5状態）を昇格させない（§8.3）。

*導出元: SPEC-387, SPEC-388, SPEC-389, SPEC-390, SPEC-391, SPEC-392, SPEC-393, SPEC-394, SPEC-395, SPEC-396, SPEC-397, SPEC-398, SPEC-399, SPEC-400, SPEC-401, SPEC-402, SPEC-403, SPEC-404, SPEC-405, SPEC-406, SPEC-407, SPEC-408*

*引用: 基本仕様 §11.3*

### DES-275

判断記録は検査ゲートではなく、`UNKNOWN` に対する外部判断の追跡である。

### DES-276

判断済みと承認済みは区別する（判断済み ≠ 承認済み）（§3.5）。

### DES-277

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

### DES-278

承認レコードの `judgment_ref` fieldは参照する判断記録IDであり、任意（judgment reference）とする。

### DES-279

承認レコードの `supersedes` fieldは明示に置き換える旧承認レコードのULID listであり、既定は空listとする。

### DES-280

承認レコードの `subject_hash` fieldは承認時点の対象の内容ハッシュである。

### DES-281

承認レコードの `dependencies` fieldは承認時点の上流依存closureであり、完全一致を要求する。

### DES-282

承認レコードの `dependencies` entryの `hash` fieldはdocument subject hashである（§1.3）。

### DES-283

承認レコードの `approver` の `kind` fieldの値域は `human` / `agent` である。

### DES-284

承認レコードの `approver` の `model` fieldはagentの場合任意とする。

### DES-285

承認レコードの `approved_state` fieldはどの承認状態かを表し、必須とする。値域は `approved` / `rejected` / `withdrawn` である。

### DES-286

承認レコードの `basis` fieldは根拠であり、任意とする。

### DES-287

承認は検証状態と独立の別軸である。

*導出元: REQ-109, REQ-110, REQ-111, REQ-112, REQ-113, REQ-114, SPEC-203, SPEC-204, SPEC-205, SPEC-206, SPEC-207, SPEC-208, SPEC-209, SPEC-471, SPEC-472, SPEC-473, SPEC-474, SPEC-475, SPEC-476, SPEC-477, SPEC-478, SPEC-479, SPEC-480, SPEC-481, SPEC-482, SPEC-483, SPEC-484, SPEC-485, SPEC-486, SPEC-487, SPEC-488, SPEC-489, SPEC-490, SPEC-491, SPEC-492, SPEC-493, SPEC-494, SPEC-495, SPEC-496, SPEC-497*

*引用: 基本仕様 §4.5, 基本仕様 §17, 要件定義 §5.5*

### DES-288

承認済みを理由に非 `PASS` を `PASS` へ昇格させない。

### DES-289

未承認を理由に `PASS` を降格させない。

### DES-290

承認記録は判断記録と同一entityであることを要求しない（別entityでありうる）（§3.4）。

### DES-291

承認は対象自身（`subject`）または参照する判断（`judgment_ref`）に承認済み状態を与える。

### DES-292

承認は特定のエンティティ型に従属しない独立の領域である。

### DES-293

承認レコードの構造・値域・実効承認の導出・状態遷移は本節だけで定義し、対象の種別ごとに別の承認規則を置かない。

### DES-294

承認の入力経路は対象種別で分けず、対象種別を引数に取る単一の正典面に一本化する（§13.2）。

*引用: 別紙A §12.2*

### DES-295

エンティティ側に置く承認操作（`vtest vo approve` / `vo_approve`）は正典面への別名であり、独自の意味論を持たない。

### DES-296

承認対象は `vo`・`document`・`judgment` の3種であり、レコード上の表現は種別ごとに定まる。

### DES-297

承認対象種別 `vo` は、レコード上 `subject` にVO ID（`VO-*`）で表現し、上流依存closureは対象VOの再帰的なparent VO、対象VOと各parent VOが `derives_from` で参照するdocument、および各documentの再帰的な上位document（`derives_from` 先）である。

### DES-298

承認対象種別 `document` は、レコード上 `subject` にdocument ID（`DOC-*`）で表現し、上流依存closureは対象documentの再帰的な上位document（`derives_from` 先）である。

### DES-299

承認対象種別 `document` は、総称documentとして登録した文書で表現し、専用のエンティティ型を設けない（§3.1）。

*導出元: SPEC-121, SPEC-122, SPEC-123, SPEC-124, SPEC-125, SPEC-126, SPEC-127, SPEC-128, SPEC-129, SPEC-130, SPEC-131, SPEC-132, SPEC-133, SPEC-134, SPEC-135, SPEC-136, SPEC-137, SPEC-138, SPEC-139, SPEC-140, SPEC-141, SPEC-142, SPEC-143*

*引用: 基本仕様 §3.1*

### DES-300

承認対象種別 `judgment` は、レコード上 `judgment_ref` に判断記録ULIDを置き、`subject` には当該判断記録の `subject`（VO IDまたはdocument ID）を置き、上流依存closureは `subject` の種別に応じた同表のclosureとする。

### DES-301

判断記録ULIDは `subject` に置かない。判断記録の承認は `judgment_ref` によってのみ表す。

### DES-302

`judgment_ref` が指す判断記録が存在しない場合は、書込み時にE-APPROVAL-001として拒否する。既存レコードとして読み取った場合は当該レコードからVO / documentの実効承認も判断記録の実効承認も導出しない（W-STORE-006）。

### DES-303

判断記録ULIDを `subject` に持つ承認レコードは、書込み時にE-APPROVAL-002として拒否する。既存レコードとして読み取った場合は履歴表示だけを許可していかなる実効承認も導出せず、W-STORE-006を出す。

### DES-304

VO ID・document IDのいずれにも解決しない `subject`（Test ID、Source Target locator、Relation ID等）も、判断記録ULIDを `subject` に持つ承認レコードと同じ扱いとする。

### DES-305

いずれの種別でも対象自身は `subject_hash` で束縛するため `dependencies` へ重複して含めない。

### DES-306

`dependencies` のentryは `kind`（`vo` | `document`）、`id` の順でsortし、欠落・重複・余剰entryを許可しない。

### DES-307

document dependencyはdocument subject hashを使用するため、document recordまたは参照先sourceの変更で承認が失効する（§1.3）。

### DES-308

`approved_state` は `approved` / `rejected` / `withdrawn` の3値だけを受理する。

### DES-309

`approved_state` が値域外の他の値の場合、書込み時にE-APPROVAL-002として拒否する。既存レコードとして読み取った場合は履歴表示だけを許可していかなる実効承認も導出せず、W-STORE-006を出す。

### DES-310

`approved_state` の値 `approved` は、この内容で進めることを認めたこと（承認）を意味する。

### DES-311

`approved_state` の値 `rejected` は、この内容で進めることを認めないこと（却下）を意味する。

### DES-312

`approved_state` の値 `withdrawn` は、先に与えた承認を取り消したこと（承認取消）を意味する。

### DES-313

承認レコードの `supersedes` は、この承認レコードが明示に置き換える旧承認レコードのULIDを名指しするlistである。

### DES-314

`supersedes` に列挙する各ULIDは、同一 `subject`（`judgment_ref` を持つ承認では同一 `judgment_ref`）の既存承認レコードを指さなければならない。

### DES-315

参照先を解決できない、対象が一致しない、または自己参照する `supersedes` entryを含むレコードは、書込み時にE-APPROVAL-002として拒否する。

### DES-316

既存レコードとして読み取った場合、およびsupersede関係が循環する場合は、当該レコードを実効集合へ寄与させずW-STORE-005を出す。

### DES-317

承認レコードの `supersedes` はRelationとは独立であり、`type: supersedes` のRelationレコードは実効承認の決定に用いない（§3.3）。

### DES-318

実効承認は対象X（VO ID、document ID、または `judgment_ref` が指す判断記録）に対して所定の順に評価して導出する。

### DES-319

`approved_state` を参照せずに承認済みを導出してはならない。

### DES-320

承認レコード a が A(X) に属するのは、`a.approved_state` が値域内であること、a の対象指定が X と一致すること（X が VO / document のとき `a.subject == X`、X が判断記録のとき `a.judgment_ref == X` の ULID）、`a.subject_hash` が `a.subject` の現在の内容ハッシュと一致すること、`a.dependencies` が `a.subject` の現在の上流依存closureと entity・hash とも完全一致すること、`a.dependencies` の各 document が登録 content_hash と実ファイルの一致を満たすこと（§11.4）、および X が判断記録のとき当該判断記録が §8.5 の有効判断でありかつ §8.5 の実効集合 E に属すること、をすべて満たす場合だけとする。

### DES-321

承認レコードaがA(X)に属するには、`a.approved_state` が値域内でなければならない。

### DES-322

承認レコードaがA(X)に属するには、aの対象指定がXと一致しなければならない（XがVO / documentのとき `a.subject == X`、Xが判断記録のとき `a.judgment_ref == X` のULID）。

### DES-323

承認レコードaがA(X)に属するには、`a.subject_hash` が `a.subject` の現在の内容ハッシュと一致しなければならない。

### DES-324

承認レコードaがA(X)に属するには、`a.dependencies` が `a.subject` の現在の上流依存closureとentity・hashとも完全一致しなければならない。

### DES-325

承認レコードaがA(X)に属するには、`a.dependencies` の各documentが登録content_hashと実ファイルの一致を満たさなければならない（§11.4）。

### DES-326

Xが判断記録のとき、承認レコードaがA(X)に属するには、さらに当該判断記録が §8.5 の有効判断であり、かつ §8.5 の実効集合Eに属さなければならない。

### DES-327

実効集合 A’(X) は、A(X) から、A(X)内の他レコードの `supersedes` に名指しされたものを除いた集合である。

### DES-328

A’(X) が空の場合、実効承認状態は `draft` とする。

### DES-329

A’(X) に `approved_state` が `rejected` または `withdrawn` のレコードが1件以上ある場合、実効承認状態は `draft` とする（fail-closed。機械はどちらかを選ばない）。

### DES-330

A’(X) の全レコードが `approved_state == approved` の場合、実効承認状態は `approved` とする。

### DES-331

実効集合 A’(X) からの除外は `supersedes` による明示の名指しだけで起きる。

### DES-332

`supersedes` 関係にない複数の有効承認レコードはすべてA’(X)に属する。

### DES-333

`approved_at` / ULIDの順序、レコードの新旧、件数の多寡のいずれも、採用する承認レコードを選ぶ規則に用いてはならない。

### DES-334

同一対象に `approved` と `rejected` の有効承認レコードが `supersedes` 関係なく併存する場合、機械はどちらも採らずfail-closedに `draft` とする。

### DES-335

`approved` を取り消すには `approved_state: withdrawn` の承認レコードを追加する。

### DES-336

`approved` を否認するには `approved_state: rejected` の承認レコードを追加する。

### DES-337

取消・却下の後に再承認するには、当該 `withdrawn` / `rejected` レコードのULIDを `supersedes` に名指しした `approved_state: approved` のレコードを追加する。

### DES-338

旧レコードを名指ししない `approved` の追加では `draft` のままとする。

### DES-339

対象Xの実効承認状態は `draft` と `approved` の2値であり、遷移は所定の入力だけで起きる。

### DES-340

`draft` から `approved` への遷移は、`approved_state: approved` の有効承認レコードが加わり、実効集合に `rejected` / `withdrawn` が1件も残らなくなることで起きる。

### DES-341

`approved` から `draft` への遷移は、`approved_state` が `rejected` または `withdrawn` の有効承認レコードが加わることで起きる。

### DES-342

`approved` から `draft` への遷移は、実効集合の `approved` レコードがすべて他レコードの `supersedes` に名指しされることで起きる。

### DES-343

`approved` から `draft` への遷移は、`subject` の内容ハッシュが変化することで起きる。

### DES-344

`approved` から `draft` への遷移は、上流依存closureのentity構成またはいずれかのhashが変化する（document再登録・参照先source変更を含む）ことで起きる（§11.4）。

### DES-345

Xが判断記録のとき、`approved` から `draft` への遷移は、当該判断記録が §8.5 の有効判断または実効集合Eから外れることで起きる。

### DES-346

検証状態（§4.1の5状態）の変化、判断記録の追加そのもの、`basis` の内容は、実効承認状態を変えない。

### DES-347

依存entryを持たない互換Approvalは読取りと履歴表示だけを許可し、現在の `approved` を導出しない。W-STORE-002を出し、対象は `draft` 相当とする。

### DES-348

承認記録は「誰が（approver）」「何を（subjectまたはjudgment reference）」「どの承認状態か（approved_state）」を必須項目として追跡可能とし、根拠は任意に記録できる。

### DES-349

承認主体は種別（`human` / `agent`）と識別子を記録する。

### DES-350

誰がどの対象・範囲を承認できるか（approval authority）、承認ロール、必要承認数、権限schemaはプロジェクト側で定義可能とし、その具体は別紙A / プロジェクト設定へ委譲する。

*導出元: SPEC-471, SPEC-472, SPEC-473, SPEC-474, SPEC-475, SPEC-476, SPEC-477, SPEC-478, SPEC-479, SPEC-480, SPEC-481, SPEC-482, SPEC-483, SPEC-484, SPEC-485, SPEC-486, SPEC-487, SPEC-488, SPEC-489, SPEC-490, SPEC-491, SPEC-492, SPEC-493, SPEC-494, SPEC-495, SPEC-496, SPEC-497, SPEC-740, SPEC-741, SPEC-742, SPEC-743, SPEC-744, SPEC-745, SPEC-746, SPEC-747, SPEC-748, SPEC-749, SPEC-750, SPEC-751, SPEC-752, SPEC-753, SPEC-754, SPEC-755, SPEC-756, SPEC-757, SPEC-758, SPEC-759, SPEC-760, SPEC-761, SPEC-762, SPEC-763, SPEC-764, SPEC-765*

*引用: 基本仕様 §17, 基本仕様 §30*

### DES-351

承認レコードの入力経路は別紙A §12.2・§13.2 に定める。

*引用: 別紙A §12.2, 別紙A §13.2*

### DES-352

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

### DES-353

Evidenceレコードの `execution_state` の `hash` fieldは、`complete: false` の場合だけnullとする。

### DES-354

Evidenceレコードの `target_coverage` fieldは `target_binding` の動的計測結果である（旧 `target_execution` fieldを改名）。

### DES-355

Evidenceレコードの `target_coverage.checked` fieldは計測を実施したかを表す。

### DES-356

Evidenceレコードの `target_coverage.result` fieldはtarget別結果の集約であり、値域は `PASS` / `FAIL` / `UNKNOWN` である。

### DES-357

Evidenceレコードの `target_coverage` の `targets` 配下の各entryの `result` fieldの値域は `PASS` / `FAIL` / `UNKNOWN` である。

### DES-358

Evidenceレコードの `target_coverage` の `targets` 配下の各entryの `count` fieldは、`result` が `UNKNOWN` の場合nullとする。

### DES-359

Evidenceレコードの `log_ref` fieldはGit管理外の生ログを指す。

### DES-360

`result` はテストランナー（判定権威）が報告した合否をそのまま記録する（§7）。

### DES-361

本システムは合否を再判定せず、`result` を `target_binding` の証拠として消費する。

*導出元: SPEC-300, SPEC-301, SPEC-302, SPEC-303, SPEC-304, SPEC-305, SPEC-306, SPEC-307, SPEC-308*

*引用: 基本仕様 §7*

### DES-362

有効なEvidenceの `result: FAIL` は `target_binding = FAIL`（テストランナーが失敗を報告）へ至る（§11.2）。

*導出元: REQ-094, REQ-095, REQ-096, REQ-097, REQ-098, REQ-099, REQ-100, REQ-101, REQ-102, REQ-103, REQ-104*

*引用: 要件定義 §5.3*

### DES-363

`target_coverage` は `target_binding` の動的計測（宣言対象の実行が生じたか）の結果であり、独立の検査項目ではない。

### DES-364

旧モデルの `target_execution` 検査項目は撤去し、その計測事実だけをEvidenceの `target_coverage` fieldとして保持して `target_binding` の証拠源へ吸収する（§10・§11.2）。

### DES-365

`hashes.targets` はTestの宣言順で常に記録し、各 `target` は§6.1で解決したcanonical Source Targetのcanonical Locatorの正規化文字列表現とする。

### DES-366

参照側Testが宣言した `TargetRef` の綴り（SRC ID参照を含む）をEvidence上のtarget identityとして記録しない（§6.1.1）。

### DES-367

`hashes.targets` のlistはTestの宣言target集合を解決したcanonical Source Target集合と重複なく1対1に対応する。

### DES-368

Evidence生成のprecondition（§9.4）により全宣言targetは一意に解決済みであるため、`hashes.targets` の集合は宣言target集合と同数になる。

### DES-369

`target_coverage.checked: true` では `target_coverage.targets` も同じ順序・同じcanonical Locator集合で1対1に対応する。

### DES-370

`target_coverage.checked: false` では `method` と `result` をnull、`targets` を空listとし、`target_binding` の動的計測を `NO_EVIDENCE`（診断 `NOT_CHECKED`）として扱う（§11.2）。

### DES-371

writerは `hashes.test_subject` を必須とする。

### DES-372

writerはTest construct単体のhashを現在のEvidence freshness keyとして出力しない。

### DES-373

readerは `rust-cargo` Evidenceに限り、互換fieldの `hashes.test_fn` または `hashes.test_construct` とtarget entry内の `target_fn` を読み取れる。

### DES-374

互換Test hashを現在の `test_subject` へ正規化できるのは、現在の `rust-cargo` adapterが当該互換hashのsource rangeに全canonical metadataとTest constructが含まれること、現在bytesとの完全一致、および現在のlogical metadataとの一致を証明できる場合だけとする。

### DES-375

証明できなければrecordは保持するが、`target_binding` の証拠として有効な `PASS` にしない。

### DES-376

中立fieldと互換fieldが併存する場合は導出される値の同値を必須とし、非 `rust-cargo` Evidenceでは互換fieldを解釈しない。

### DES-377

readerは単数互換形の `hashes.target_fn` および `target_coverage.result/count` を、現在の `rust-cargo` Testがtargetをちょうど1件宣言し、Test subjectを証明でき、target construct hashも照合できる場合だけ1要素listへ正規化して扱う。

### DES-378

複数target Testに単数互換形を適用しない。

### DES-379

writerは常にlist形を出力する。

### DES-380

Evidence内の `target` は実行時snapshotを識別するkeyであり、TEST → SRC edgeの正典ではない。

### DES-381

graphはadapter所有のTest metadata宣言からだけ構築し、Evidenceのtarget listからedgeを生成しない。

*導出元: SPEC-094, SPEC-095, SPEC-096, SPEC-097, SPEC-098, SPEC-099, SPEC-100*

*引用: 基本仕様 §2.3*

### DES-382

`execution_state` は§1.3のExecution State subjectである。

### DES-383

writerは実行直前にadapterからsnapshot DTOを取得し、core検証後のschema ID、完全性、subject hashを記録する。

### DES-384

`complete: true` は、選択Testのビルドと実行可能状態を変えうるrepository / local dependency入力、runner、toolchain、実行影響configをadapterが漏れなく列挙した場合だけ許可する。

### DES-385

snapshot生成不能または不完全の場合も実行事実の履歴を記録できるが、`complete: false`、`hash: null` として現在の有効な `PASS` 証拠へ使用しない。

### DES-386

`rust-cargo-execution-state-v1` のmanifestは、選択Testを含むCargo workspace / package root、全local path dependency root、各root内の通常file、Cargo manifest / lockfile、`.cargo` config、build script、Rust source / test / fixture / compile-time resource、toolchain指定を含む。

### DES-387

`.git/`、`.verify/` のcanonical record / cache、Cargo target directory等の生成物は実行入力から除外する。

### DES-388

除外領域をbuild script、macro、`include_*`、path dependencyその他の経路で読み込む可能性を排除できない場合、snapshotを完全と報告しない。

### DES-389

repository内helperだけの変更もmanifest hashを変化させる。

### DES-390

Evidence readerは `execution_state` を欠く互換recordを履歴表示できるが、現在のEvidence freshnessを証明できないため `NO_EVIDENCE`（診断 `STALE`）とする。

### DES-391

schema違反、target entryの欠落・重複・余剰、またはaggregate resultとtarget別結果の矛盾はE-SCAN-010として扱い、そのEvidenceを有効な結果に使用しない。

### DES-392

Evidence writerは `adapter` を必須で記録し、保存前にTestの `ExecutionDescriptor.adapter` およびrunner kindとの整合を検証する。

### DES-393

Evidence readerは `adapter` の欠落を許容するが、現在のTestが `rust-cargo` で、互換runner kindと内容ハッシュからRust実行であることを一意に確認できる場合だけ互換Evidenceとして扱う。

### DES-394

確認不能は `UNKNOWN`、明示adapterの不一致は `MISMATCH` とし、いずれも `PASS` へ昇格しない。

## DA-005 4. Test metadata宣言contract

### DES-395

`SourceDiscoveryAdapter` は、adapter所有のsource declarationを `id`、`covers[]`、`targets[]`、`intent`、`input?`、`expect?`、`kind?`、`cases[]`、`related[]` の論理fieldへ正規化する。

> id, covers[], targets[], intent, input?, expect?, kind?, cases[], related[]

### DES-396

本versionでは、Testの存在理由分類（旧 `role` / `anchor` / `anchor_rationale`）を論理fieldに持たない。

### DES-397

すべての管理対象Testに `covers ≥ 1` を一律に要求する。

*導出元: REQ-052, REQ-053, REQ-054, REQ-055, REQ-056, REQ-057, REQ-058, SPEC-409, SPEC-410, SPEC-411, SPEC-412, SPEC-413, SPEC-414, SPEC-415, SPEC-416, SPEC-417, SPEC-418, SPEC-419, SPEC-420, SPEC-421, SPEC-422, SPEC-423, SPEC-424, SPEC-425*

*引用: 基本仕様 §12, 要件定義 §4.1*

### DES-398

VOへの寄与は `covers` 宣言と証拠の十分性判定だけから導出する。

### DES-399

検証対象は一般概念であり、adapter中立coreは各管理対象Testに1件以上の検証対象を要求する。

*導出元: REQ-144, REQ-145, REQ-146, REQ-147, REQ-148, SPEC-332, SPEC-333, SPEC-334, SPEC-335, SPEC-336, SPEC-337, SPEC-338, SPEC-339*

*引用: 基本仕様 §9.1, 要件定義 §9.1*

### DES-400

検証対象は「そのTestが検証成立性を証明しようとする対象＝宣言された『何の時にどうなる』の主語」であって、実装constructに限定しない。

### DES-401

検証対象を実装construct（Source Target）として実現するか、外部から観測可能な契約・境界上の振る舞いとして実現するかは実行形態が定める。

*導出元: REQ-064, REQ-065, REQ-066, REQ-067, REQ-068, REQ-069, REQ-070, REQ-071, REQ-072, SPEC-325, SPEC-326, SPEC-327, SPEC-328, SPEC-329, SPEC-330, SPEC-331, SPEC-332, SPEC-333, SPEC-334, SPEC-335, SPEC-336, SPEC-337, SPEC-338, SPEC-339*

*引用: 基本仕様 §8.3, 基本仕様 §9.1, 要件定義 §4.3*

### DES-402

coreの `chain_integrity` は「検証対象をSource Targetとして実現し `targets ≥ 1` を宣言すること」をadapter中立の必須リンクとしない（coreのTest層必須はTest ID・`covers ≥ 1`・その他の必須metadata）（§11.1.1）。

*導出元: SPEC-224, SPEC-225, SPEC-226, SPEC-227, SPEC-228, SPEC-229, SPEC-230, SPEC-231, SPEC-232, SPEC-233, SPEC-234, SPEC-409, SPEC-410, SPEC-411, SPEC-412, SPEC-413, SPEC-414, SPEC-415, SPEC-416, SPEC-417, SPEC-418, SPEC-419, SPEC-420, SPEC-421, SPEC-422, SPEC-423, SPEC-424, SPEC-425*

*引用: 基本仕様 §5.1, 基本仕様 §12*

### DES-403

`targets[]` は検証対象をSource Targetとして実現するためのcapability fieldであり、その要求件数はadapterが定める。

*導出元: SPEC-340, SPEC-341, SPEC-342, SPEC-343, SPEC-344, SPEC-345, SPEC-346, SPEC-347, SPEC-348, SPEC-349*

*引用: 基本仕様 §9.2*

### DES-404

v0.1の唯一のadapter `rust-cargo` は検証対象をSource Targetとして実現し `targets ≥ 1` を必須とする（§4.2・§4.4・§5.5）。

### DES-405

非sourceの境界形態（外部契約・境界上の振る舞い）の具体的表現・確認方法は特定形態を他形態へ一律要求せず、下位仕様・後続adapter・後続版へ委譲する（本versionでContract-Target類の新schemaは設けない）。

*導出元: REQ-064, REQ-065, REQ-066, REQ-067, REQ-068, REQ-069, REQ-070, REQ-071, REQ-072, SPEC-242, SPEC-243, SPEC-244, SPEC-245, SPEC-246, SPEC-247, SPEC-248, SPEC-249, SPEC-250, SPEC-251, SPEC-252, SPEC-253, SPEC-254, SPEC-255, SPEC-256, SPEC-257, SPEC-258, SPEC-259, SPEC-260, SPEC-261, SPEC-262*

*引用: 要件定義 §4.3, 基本仕様 §5.3*

### DES-406

coreはsource declarationの構文と配置を解釈しない。

### DES-407

coreはadapterが返したTest Entity、Discovered Test observation、Source Location、Target Reference、source range、診断を検証・統合する。

### DES-408

locatorは `TargetRef::Locator { adapter, value }` とし、`value` はadapter所有のopaque文字列である。

### DES-409

coreはpath、module、symbol種別を分解しない。

### DES-410

`rust-cargo` の `@vtest.` 宣言表面は2種であり、表面ごとに認識する行形式が異なる。

### DES-411

Test constructのdoc comment（`///` または `/** */`）は表面1であり、test-annotation-lineを認識する。

### DES-412

Test constructではない関数itemのdoc comment（対象実装側の関数等）は表面2であり、source-target-annotation-lineを認識する。

### DES-413

test-annotation-lineの文法は `"@vtest." test-key SP value` である。

### DES-414

source-target-annotation-lineの文法は `"@vtest." source-target-key SP value` である。

### DES-415

test-keyの値域は `id` / `covers` / `target` / `intent` / `input` / `expect` / `kind` / `case` / `related` である。

### DES-416

source-target-keyの値域は `src-id` である。

### DES-417

valueは行末までのテキストとし、前後空白は除去する。

### DES-418

annotation行は1行1キーとする。

### DES-419

`covers` と `related` の値はカンマ区切りで複数指定できる。

### DES-420

`case` と `related` はキー自体を複数行書ける。

### DES-421

`case` と `related` 以外のキーの重複はエラーE-SCAN-005とする。

### DES-422

ただし `kind` がintegration系のTestに限り、`target` の複数行を許容する。

*引用: 別紙A §14.3*

### DES-423

許容された複数 `target` 内でも同じTargetRefの重複はE-SCAN-005とする。

### DES-424

綴りが異なっても解決後に同一canonical Source Targetへ到達する複数宣言（同じSource Targetへのlocator参照とSRC ID参照の併記等）も、coreが解決時にE-SCAN-005とする（§6.1.1）。

### DES-425

表面1で、`@vtest.` で始まるがtest-keyを持たない行はエラーE-SCAN-006とする（打鍵ミスの検出を優先し、警告ではなくエラーとする）。

### DES-426

表面1のE-SCAN-006は、未知キーに加え、source-target-key（`src-id`）の誤配置も含む。`src-id` は対象実装側の関数に付与すべきキーであり、Test metadataへの取り込み先を持たない。

### DES-427

表面2で、`@vtest.` で始まるがsource-target-keyを持たない行（test-keyを含む）は警告W-SCAN-105とする（§5.4）。

### DES-428

打鍵ミス検出の目的は両表面に及ぶが、表面2の宣言はTest metadataを破損させず採用値の曖昧さも生まないため、errorではなくwarningとする。

### DES-429

`src-id` は表面2でも反復不可であり、同一関数itemでの重複は採用すべきIDを決定できないためエラーE-SCAN-005とする。

### DES-430

`src-id` 重複時はいずれの宣言値も採用せず、当該Source TargetのSRC IDは無しとして扱う（どちらかを推測で選ばない）。

### DES-431

doc comment 内の `@vtest.` を含まない行は自由記述として無視する。

### DES-432

`@vtest.src-id` はテストではなく対象実装側の関数に付与し、任意の恒久SRC IDを宣言する。

### DES-433

scannerは `@vtest.src-id` の指定値を認識するが、付与を必須としない。

*導出元: SPEC-340, SPEC-341, SPEC-342, SPEC-343, SPEC-344, SPEC-345, SPEC-346, SPEC-347, SPEC-348, SPEC-349*

*引用: 基本仕様 §9.2*

### DES-434

`rust-cargo` のSource Target constructは属性とdoc commentを含む関数item全体であり、`@vtest.src-id` の宣言行はconstruct bytesの内側にある（§1.3）。

### DES-435

したがって `@vtest.src-id` の付与・変更・削除はSource Target hashを変化させる。

### DES-436

表面2での打鍵ミス（`src_id` 等の未知キー）はW-SCAN-105、`src-id` の重複はE-SCAN-005で検出し、無音で無視しない（§4.2・§5.4）。

### DES-437

locator文法は `locator = path "::" item-path` である。

> 例：src/parser.rs::Parser::parse
>     src/lib.rs::validate_input

### DES-438

pathはプロジェクトルートからの相対パス（"/" 区切り、".rs" で終わる）である。

### DES-439

item-pathはRustアイテムパス（"::" 区切り）であり、implブロック内の関数は"型名::関数名"とする。

### DES-440

`path` は `.rs` で終わる最初の `::` で item-path と分離する。

### DES-441

`rust-cargo` adapterはlocator値を `TargetRef::Locator { adapter: "rust-cargo", value: locator }` へ正規化する。

### DES-442

`@vtest.target` の値が `SRC-` で始まる場合はSRC ID参照として返す。

### DES-443

adapter固有のsource declarationを構文解析できない場合、adapterは該当Test constructをDiscovered Testとして返し、対応を `ManagedTestLink::Missing` として診断を付与する。

### DES-444

coreは当該Testを管理宣言欠落として `chain_integrity` の `MISMATCH`（診断 `MISSING`）とし、対応VOを推測で寄与関係へ関連付けない。

### DES-445

source declarationを構文上完全なTest Entityへ正規化できる条件は、adapter中立coreが要求する必須metadata（構文上有効なTest ID・1件以上の `covers`・`intent`）に加え、当該adapterが必須とする追加metadataをTest Entityとして具体化できることをいう。

### DES-446

`rust-cargo` は検証対象をSource Targetとして実現する形態であり、追加必須metadataとして `targets ≥ 1` を要求する（§4.1・§4.2・§5.5）。

*導出元: SPEC-340, SPEC-341, SPEC-342, SPEC-343, SPEC-344, SPEC-345, SPEC-346, SPEC-347, SPEC-348, SPEC-349*

*引用: 基本仕様 §9.2*

### DES-447

coreの `id` / `covers ≥ 1` / `intent`、および `rust-cargo` の `targets ≥ 1` という必須metadataを欠く場合はE-SCAN-007とし、`ManagedTestLink::Missing`（`chain_integrity` の `MISMATCH`、診断 `MISSING`）とする。

### DES-448

E-SCAN-007はadapterが報告する構文・必須metadata診断であり、`targets ≥ 1` は `rust-cargo` の必須metadataとしてこの経路で検出される（core中立の必須リンクへは加えない）（§11.1.1）。

### DES-449

`covers` 件数の可変制約（旧role/anchor由来）は設けず、すべての管理対象Testに `covers ≥ 1` を一律要求する。

*導出元: SPEC-409, SPEC-410, SPEC-411, SPEC-412, SPEC-413, SPEC-414, SPEC-415, SPEC-416, SPEC-417, SPEC-418, SPEC-419, SPEC-420, SPEC-421, SPEC-422, SPEC-423, SPEC-424, SPEC-425*

*引用: 基本仕様 §12*

### DES-450

構文上完全なTest Entityへ正規化できるが、`covers` のVO IDをcore storeで解決できない場合、そのentityと `ManagedTestLink::One(id)` を保持する。

### DES-451

E-SCAN-003と `chain_integrity = MISMATCH` はcoreの参照整合性検査で生成する。

### DES-452

E-SCAN-003が発生しても対応するTest Entityと `ManagedTestLink::One` を除去しない。

### DES-453

VO参照の解決とTest IDの大局的一意性はadapterではなくcoreが検査する。

### DES-454

adapterはTest構文の違反（重複不可キーの重複、未知キー、必須キーの欠落）をE-SCAN-005 / E-SCAN-006 / E-SCAN-007で報告する（§5.4）。

### DES-455

VO解決・ID一意性・target解決はcoreが参照整合性検査で判定する（§5）。

## DA-006 5. Discovery orchestration設計

### DES-456

処理フロー第1段は、registryとconfigの検証であり、adapter ID、capability宣言、config namespace、rootを検証する。

### DES-457

処理フロー第2段は、discovery委譲であり、登録順ではなくadapter ID順にSourceDiscoveryAdapterを呼び出し、各adapterはDiscoveryBatchを返す。

### DES-458

処理フロー第3段は、adapter出力の検証であり、adapter ID、Source Location、source range、current bytes、hash未計算のTest draft、metadata source、Target draft、診断を検証する。

### DES-459

処理フロー第3段では、capability宣言と出力が矛盾するbatchは拒否する。

### DES-460

処理フロー第4段は、core materializationであり、Test subjectとSource Targetのhashを§1.3で計算し、TestEntity、SourceTarget、DiscoveredTest、ManagedTestLinkを具体化する（存在理由分類の実効値確定は行わない。covers ≥ 1 を一律要求する）。

### DES-461

処理フロー第5段は、決定論的な統合であり、adapter ID、project-relative path、opaque locator、Test IDの順に正規化する。

### DES-462

処理フロー第5段では、adapter間を含むTest ID・SRC ID衝突と不正な複数対応を検査する。

### DES-463

処理フロー第6段は、`.verify/` 読み込みであり、vtest-storeが全レコード（document / VO / Relation / 判断 / 承認 / Evidence）を読み込み、スキーマ検証する。

### DES-464

処理フロー第7段は、参照整合性検査であり、coversのVO ID、targetsのTarget Reference / SRC ID、Relation、VO parent、VO derives_from（document）、document derives_from を解決する。

### DES-465

処理フロー第8段は、グラフ構築と整合性検査である（§5.3・§5.4・§5.6）。

### DES-466

adapterが解析不能または不完全なbatchを返した場合、coreは対応する検証を `UNKNOWN` とし、Test 0件の完全なdiscoveryとして扱わない。

### DES-467

`TestEntity` の `id` fieldは `TestId` 型である。

### DES-468

`TestEntity` の `covers` fieldは `Vec<VoId>` 型であり、1件以上を持つ（`covers ≥ 1` 一律）（§4.4）。

### DES-469

`TestEntity` の `targets` fieldは `Vec<TargetRef>` 型であり、各要素はadapter付きopaque locatorまたはSrcIdである。

### DES-470

`TestEntity.targets` の件数はadapterが定める（`rust-cargo` は `targets ≥ 1` を必須とする）（§4.1・§4.4）。

### DES-471

coreは `targets ≥ 1` を中立必須にせず、`TestEntity.targets` の型としては空を許容する。

### DES-472

`TestEntity` の `intent` fieldは `String` 型である。

### DES-473

`TestEntity` の `input` fieldは `Option<String>` 型である。

### DES-474

`TestEntity` の `expect` fieldは `Option<String>` 型である。

### DES-475

`TestEntity` の `kind` fieldは `Option<String>` 型である。

### DES-476

`TestEntity` の `cases` fieldは `Vec<String>` 型である。

### DES-477

`TestEntity` の `related` fieldは `Vec<TestId>` 型である。

### DES-478

`TestEntity` の `location` fieldは `SourceLocation` 型である。

### DES-479

`TestEntity` の `content_hash` fieldは `ContentHash` 型であり、§1.3のTest subject hash（coreが計算）である。

### DES-480

`TestEntity` の `execution` fieldは `ExecutionDescriptor` 型である。

### DES-481

`TargetRef` は `Locator { adapter: AdapterId, value: String }` variantを持つ。

### DES-482

`TargetRef` は `SrcId(SrcId)` variantを持つ。

### DES-483

`SourceLocation` の `adapter` fieldは `AdapterId` 型である。

### DES-484

`SourceLocation` の `path` fieldは `ProjectPath` 型である。

### DES-485

`SourceLocation` の `locator` fieldは `String` 型であり、adapter所有のopaque construct locatorである。

### DES-486

`SourceLocation` の `byte_range` fieldは `SourceRange` 型である。

### DES-487

`ExecutionDescriptor` の `adapter` fieldは `AdapterId` 型である。

### DES-488

`ExecutionDescriptor` の `project` fieldは `Option<String>` 型である。

### DES-489

`ExecutionDescriptor` の `suite` fieldは `Option<TestSuite>` 型である。

### DES-490

`ExecutionDescriptor` の `selector` fieldは `String` 型である。

### DES-491

`TestSuite` の `kind` fieldは `String` 型である。

### DES-492

`TestSuite` の `name` fieldは `Option<String>` 型である。

### DES-493

検証状態は `Pass` / `Fail` / `Mismatch` / `NoEvidence` / `Unknown` の5つのみである。

*導出元: REQ-085, REQ-086, REQ-087, REQ-088, REQ-089, REQ-090, REQ-091, SPEC-174, SPEC-175, SPEC-176, SPEC-177, SPEC-178, SPEC-179, SPEC-180, SPEC-181, SPEC-182, SPEC-183, SPEC-184, SPEC-185, SPEC-186*

*引用: 基本仕様 §4.1, 要件定義 §5.1*

### DES-494

`CheckValue` は `Pass` variantを持つ。

### DES-495

`CheckValue` は `Fail` variantを持つ。

### DES-496

`CheckValue` は `Mismatch` variantを持つ。

### DES-497

`CheckValue` は `NoEvidence` variantを持つ。

### DES-498

`CheckValue` は `Unknown` variantを持つ。

### DES-499

診断ラベルは検証状態と別軸である。

*導出元: REQ-092, REQ-093, SPEC-187, SPEC-188, SPEC-189, SPEC-190*

*引用: 基本仕様 §4.2, 要件定義 §5.2*

### DES-500

`DiagnosticLabel` は `Missing` variantを持つ。

### DES-501

`DiagnosticLabel` は `NotExecuted` variantを持つ。

### DES-502

`DiagnosticLabel` は `NotChecked` variantを持つ。

### DES-503

`DiagnosticLabel` は `Stale` variantを持つ。

### DES-504

検査は `ChainIntegrity` / `OrphanDetection` / `TargetBinding` / `OraclePresence` の4本のみである。

*導出元: REQ-034, REQ-035, REQ-050, REQ-051, REQ-052, REQ-053, REQ-054, REQ-055, REQ-056, REQ-057, REQ-058, REQ-059, REQ-060, REQ-061, REQ-062, REQ-063, REQ-064, REQ-065, REQ-066, REQ-067, REQ-068, REQ-069, REQ-070, REQ-071, REQ-072, REQ-073, REQ-074, REQ-075, REQ-076, REQ-077, REQ-078, REQ-079, REQ-080, REQ-081, REQ-082, REQ-083, REQ-084, SPEC-217, SPEC-218, SPEC-219, SPEC-220, SPEC-221, SPEC-222, SPEC-223, SPEC-224, SPEC-225, SPEC-226, SPEC-227, SPEC-228, SPEC-229, SPEC-230, SPEC-231, SPEC-232, SPEC-233, SPEC-234, SPEC-235, SPEC-236, SPEC-237, SPEC-238, SPEC-239, SPEC-240, SPEC-241, SPEC-242, SPEC-243, SPEC-244, SPEC-245, SPEC-246, SPEC-247, SPEC-248, SPEC-249, SPEC-250, SPEC-251, SPEC-252, SPEC-253, SPEC-254, SPEC-255, SPEC-256, SPEC-257, SPEC-258, SPEC-259, SPEC-260, SPEC-261, SPEC-262, SPEC-263, SPEC-264, SPEC-265, SPEC-266, SPEC-267, SPEC-268, SPEC-269, SPEC-270, SPEC-271, SPEC-272, SPEC-273, SPEC-274, SPEC-275, SPEC-276, SPEC-277, SPEC-278, SPEC-279, SPEC-280, SPEC-281, SPEC-282, SPEC-283, SPEC-284, SPEC-285, SPEC-286*

*引用: 基本仕様 §5, 要件定義 §3.3, 要件定義 §4*

### DES-505

`CheckItem` は `ChainIntegrity` variantを持つ。

### DES-506

`CheckItem` は `OrphanDetection` variantを持つ。

### DES-507

`CheckItem` は `TargetBinding` variantを持つ。

### DES-508

`CheckItem` は `OraclePresence` variantを持つ。

### DES-509

`CheckValue` は状態のみを表し、原因説明は `DiagnosticLabel` として併記する。

### DES-510

集約の代表値選択に診断ラベルを用いない（§11.3）。

*導出元: SPEC-577, SPEC-578, SPEC-579, SPEC-580, SPEC-581, SPEC-582*

*引用: 基本仕様 §22.2*

### DES-511

`Missing` / `NotChecked` / `NotExecuted` / `Stale` を検証状態のvariantとして持たせない（旧8値モデルの排除）。

### DES-512

`TargetRef::SrcId` はadapter IDを含まないため、`SrcId` は全adapterを統合したrepositoryでglobal uniqueでなければならない。

### DES-513

collision時はE-SCAN-011とし、TargetRefを解決しない。

### DES-514

`TestEntity.execution` はadapter、project、suite、opaque selectorからなる中立な実行座標である。

### DES-515

coreは `project`、`suite.kind`、`suite.name`、`selector` の文字列を解釈しない。

### DES-516

`filter`、`package`、`test_target` および `TestTarget` 型を `vtest-model` へ置かない。

### DES-517

`vtest-adapter-api` は言語非依存の `TestWireCodec` capabilityを定義する。

### DES-518

codecはadapter固有のcompatibility propertyをJSON objectとしてencode / decodeできるが、core domain typeへadapter固有fieldを追加しない。

### DES-519

`rust-cargo` codecはversion 1互換の `filter`、`package`、`test_target` を所有する。

### DES-520

JSON writerは `execution` を常に出力し、`rust-cargo` TestだけにRust互換fieldを追加する。

### DES-521

非Rust TestではRust互換fieldを省略する。

### DES-522

JSON readerは `execution` を優先し、互換field併存時はdescriptorとの一致を検証する。

### DES-523

`execution` が欠ける場合、完全で相互整合するRust互換fieldからだけ `rust-cargo` descriptorを導出する。

### DES-524

不完全・矛盾時は入力を拒否し、空selectorまたはdummy値を生成しない。

### DES-525

Test JSON writerは `TestEntity.targets` を1件以上のlistとして常に出力する。

### DES-526

targetが1件の場合だけ同値の単数互換field `target` を追加できる。

### DES-527

readerは `target` だけの入力を1要素listへ正規化し、`targets` との併存時は完全一致を検証する。

### DES-528

複数targetから代表値を選んで `target` を生成しない。

### DES-529

`SourceDiscoveryAdapter` は、`SourceFragment`・`ManagedTestDraft`・`DiscoveredTestDraft`・`ManagedTestDraftLink`・`SourceTargetDraft`・`DiscoveryBatch`・`DiscoveryCompleteness` をhash未計算のDTOとして返す。

### DES-530

`SourceFragment` の `location` fieldは `SourceLocation` 型である。

### DES-531

`SourceFragment` の `bytes` fieldは `Vec<u8>` 型である。

### DES-532

`ManagedTestDraft` の `id` fieldは `TestId` 型である。

### DES-533

`ManagedTestDraft` の `covers` fieldは `Vec<VoId>` 型である。

### DES-534

`ManagedTestDraft` の `targets` fieldは `Vec<TargetRef>` 型である。

### DES-535

`ManagedTestDraft` の `intent` fieldは `String` 型である。

### DES-536

`ManagedTestDraft` の `input` fieldは `Option<String>` 型である。

### DES-537

`ManagedTestDraft` の `expect` fieldは `Option<String>` 型である。

### DES-538

`ManagedTestDraft` の `kind` fieldは `Option<String>` 型である。

### DES-539

`ManagedTestDraft` の `cases` fieldは `Vec<String>` 型である。

### DES-540

`ManagedTestDraft` の `related` fieldは `Vec<TestId>` 型である。

### DES-541

`ManagedTestDraft` の `execution` fieldは `ExecutionDescriptor` 型である。

### DES-542

`DiscoveredTestDraft` の `adapter` fieldは `AdapterId` 型である。

### DES-543

`DiscoveredTestDraft` の `location` fieldは `SourceLocation` 型である。

### DES-544

`DiscoveredTestDraft` の `construct` fieldは `SourceFragment` 型である。

### DES-545

`DiscoveredTestDraft` の `metadata_sources` fieldは `Vec<SourceFragment>` 型である。

### DES-546

`DiscoveredTestDraft` の `managed` fieldは `ManagedTestDraftLink` 型である。

### DES-547

`ManagedTestDraftLink` は `Missing` variantを持つ。

### DES-548

`ManagedTestDraftLink` は `One(ManagedTestDraft)` variantを持つ。

### DES-549

`ManagedTestDraftLink` は `Multiple(Vec<ManagedTestDraft>)` variantを持つ。

### DES-550

`SourceTargetDraft` の `target` fieldは `TargetRef` 型である。

### DES-551

`SourceTargetDraft` の `src_id` fieldは `Option<SrcId>` 型である。

### DES-552

`SourceTargetDraft` の `location` fieldは `SourceLocation` 型である。

### DES-553

`SourceTargetDraft` の `construct` fieldは `SourceFragment` 型である。

### DES-554

`DiscoveryBatch` の `adapter` fieldは `AdapterId` 型である。

### DES-555

`DiscoveryBatch` の `completeness` fieldは `DiscoveryCompleteness` 型である。

### DES-556

`DiscoveryBatch` の `discovered_tests` fieldは `Vec<DiscoveredTestDraft>` 型である。

### DES-557

`DiscoveryBatch` の `source_targets` fieldは `Vec<SourceTargetDraft>` 型である。

### DES-558

`DiscoveryBatch` の `diagnostics` fieldは `Vec<Diagnostic>` 型である。

### DES-559

`DiscoveryCompleteness` は `Complete` variantを持つ。

### DES-560

`DiscoveryCompleteness` は `Incomplete` variantを持つ。

### DES-561

Source Targetはcanonical locator（`TargetRef::Locator`）と任意の恒久SRC IDを併有する単一のdomain entityである。

### DES-562

`TargetRef::Locator` と `TargetRef::SrcId` はいずれも同一Source Targetへのaddressing modeであり、別個のentityを指さない。

### DES-563

恒久SRC IDはlocatorの代替ではなく、同じSource Targetへ与えられるoptional permanent identityである。

### DES-564

adapterは `@vtest.src-id` 等で宣言された恒久SRC IDを `SourceTargetDraft.src_id` として返す。

### DES-565

同一constructをlocator版とSrcId版の2件のdraftへ複製してはならない。

### DES-566

`SourceTargetDraft.target` は必ず `TargetRef::Locator` でなければならない（§1.3）。

### DES-567

`TargetRef::SrcId` はSource Targetへの参照表現であり、`SourceTargetDraft` のcanonical targetとして返してはならない。

### DES-568

adapterが `target` に `TargetRef::SrcId` を返した場合はmalformed adapter outputとして拒否する。

### DES-569

恒久SRC IDは `src_id` だけで搬送し、`target` の綴りを変えない。

### DES-570

coreは `src_id` を統合済みSRC索引へ登録し、locator参照とSRC ID参照のどちらから解決しても同一のcanonical Source Targetへ到達させる。

### DES-571

Source Target hashは常にcanonical Locatorとconstruct bytesから計算し、恒久SRC IDを独立したhash fieldとして含めない。

### DES-572

canonical Locatorは恒久SRC IDの増減で変化しないため、参照方法の違いによってSource Targetの件数、content / subject hash、Evidence上のtarget identityが分裂しない。

### DES-573

恒久SRC IDの宣言をconstruct bytesの内側へ置くadapterでは、その宣言の追加・変更・削除がconstruct bytesを変え、Source Target hashを変化させうる（§1.3）。

### DES-574

恒久SRC IDの宣言をconstruct bytesの内側へ置くadapterでSource Target hashが変化することは、sourceが実際に変化したことの帰結であり、参照方法による分裂ではない。

### DES-575

coreは統合済みSRC索引から、その恒久SRC IDを宣言した `SourceTargetDraft.target`（= canonical Locator）へ解決する。

### DES-576

恒久SRC IDを持つSource Targetも引き続きcanonical locatorでaddressableでなければならない。

### DES-577

adapterは `SourceFragment.bytes` が `location.byte_range` の現在bytesと一致する状態だけを返す。

### DES-578

manifest等にある非隣接metadataも `metadata_sources` へ列挙するが、hash inputはadapter構文のraw表現ではなく `ManagedTestDraft` のcanonical logical metadataである。

### DES-579

coreはrange・bytes対応を検証し、§1.3でhashを計算してから `TestEntity`、`SourceTarget` および `DiscoveredTest` を具体化する。

### DES-580

`ManagedTestDraftLink::One` / `Multiple` の各draftは、全logical metadataを導出した1件以上の `metadata_sources` を持たなければならない。

### DES-581

provenance欠落はmalformed adapter outputとしてE-ADAPTER-002で拒否する。

### DES-582

`DiscoveredTest` の `adapter` fieldは `AdapterId` 型である。

### DES-583

`DiscoveredTest` の `location` fieldは `SourceLocation` 型である。

### DES-584

`DiscoveredTest` の `content_hash` fieldは `ContentHash` 型である。

### DES-585

`DiscoveredTest` の `managed` fieldは `ManagedTestLink` 型である。

### DES-586

`ManagedTestLink` は `Missing` variantを持つ。

### DES-587

`ManagedTestLink` は `One(TestId)` variantを持つ。

### DES-588

`ManagedTestLink` は `Multiple(Vec<TestId>)` variantを持つ。

### DES-589

`SourceDiscoveryAdapter` はadapterがTestとして認識した全Discovered Test draftを返す。

### DES-590

`ManagedTestDraftLink::One` は、構文上有効なTest IDと必須metadata（core中立の `covers ≥ 1` / `intent`、および当該adapterが必須とする追加metadata。`rust-cargo` では `targets ≥ 1`）をdraftとして具体化できる場合に設定する（§4.1・§4.4）。

### DES-591

VO参照の解決とTest IDの大局的一意性はadapterではなくcoreが検査する。

### DES-592

解決不能な `covers` を持つdraftもcore materialization後のmanaged entity集合に保持され、対応するobservationは `ManagedTestLink::One(id)` を持つ。

### DES-593

`ManagedTestDraftLink::Missing` は管理宣言の欠落または必須metadataの欠落を表す。

### DES-594

`ManagedTestDraftLink` の `Multiple` は同一Test constructから複数draftが生じる状態を表す。

### DES-595

core materialization後の対応する状態が `ManagedTestLink` となる。

### DES-596

adapter capabilityは `SourceDiscoveryAdapter`、`TestWireCodec`、`StaticAnalysisAdapter`、`StructuredTestAdapter`、`TestRunnerAdapter`、`CoverageAdapter` に分割する。

### DES-597

各adapterは一意なID、languages、capabilities、config namespaceを宣言する。

### DES-598

registryは宣言と実装の不一致および重複IDを拒否する。

### DES-599

明示操作に必須のcapabilityがない場合はE-ADAPTER-004で操作を中止する。

### DES-600

検証集約では、static解析 / coverage欠落は `NO_EVIDENCE`（診断 `NOT_CHECKED`）とする。

### DES-601

検証集約では、runner欠落は `NO_EVIDENCE`（診断 `NOT_EXECUTED`）とする。

### DES-602

検証集約では、解析限界は `UNKNOWN` とする。

### DES-603

`TestRunnerAdapter` は、coreがfreshness subjectを所有できるよう `ExecutionInputDraft`・`ExecutionStateDraft` をhash未計算のDTOとして返す。

### DES-604

`CanonicalProjection` は型tag、null、list順序、map key順序を保持する言語非依存値とする。

### DES-605

`ExecutionInputDraft` の `root_identity` fieldは `String` 型である。

### DES-606

`ExecutionInputDraft` の `root_relative_path` fieldは `String` 型である。

### DES-607

`ExecutionInputDraft` の `kind` fieldは `String` 型である。

### DES-608

`ExecutionInputDraft` の `bytes` fieldは `Vec<u8>` 型である。

### DES-609

`ExecutionStateDraft` の `schema_id` fieldは `String` 型である。

### DES-610

`ExecutionStateDraft` の `schema_version` fieldは `String` 型である。

### DES-611

`ExecutionStateDraft` の `complete` fieldは `bool` 型である。

### DES-612

`ExecutionStateDraft` の `head_revision` fieldは `Option<String>` 型である。

### DES-613

`ExecutionStateDraft` の `runner_kind` fieldは `String` 型である。

### DES-614

`ExecutionStateDraft` の `invocation` fieldは `CanonicalProjection` 型である。

### DES-615

`ExecutionStateDraft` の `toolchain_identity` fieldは `String` 型である。

### DES-616

`ExecutionStateDraft` の `effective_config` fieldは `CanonicalProjection` 型である。

### DES-617

`ExecutionStateDraft` の `inputs` fieldは `Vec<ExecutionInputDraft>` 型である。

### DES-618

`StaticAnalysisAdapter` は正典レコードを持たない再計算派生であり、判定は現在のsource / target / configから都度計算する（§7.1）。

### DES-619

coreはfreshness subjectを静的解析用に永続化せず、検証のたびに現在入力で再導出する。

### DES-620

Test Runnerはcommand起動前に `ExecutionStateDraft` を構築し、実際に使用するinvocation / toolchain / configと一致するDTOだけを実行結果へ添付する。

### DES-621

`invocation` はselector、working root、runner option等をmachine非依存に正規化し、絶対pathを含む表示用commandとは分離する。

### DES-622

coreは実行前後でExecution State subject全体が変化していないことを確認してからEvidenceを記録する。

### DES-623

変化した場合はE-EXEC-004としてEvidenceを生成しない。

### DES-624

有効性再評価では同じschemaを持つ現在DTOを再構築し、保存hashと比較する。

### DES-625

Structured Test capabilityを宣言するadapterは、処理可能なbuilt-in Form `kind` 集合と、adapter fieldを持たないForm Schemaを判定するcompatibility matcherを宣言する。

### DES-626

Form `kind` はbuilt-inと `.verify/forms/` を統合したrepository全体で一意である。

### DES-627

Form Schemaの `adapter` field、registryのowner、Structured Test capabilityが同じadapter IDを示す場合だけ `kind → adapter` を確定する。

### DES-628

重複kindまたは対応の不一致はE-ADAPTER-001、未知kindはE-OP-001とし、coreが名前からRust adapterを推測しない。

### DES-629

`adapter` fieldを欠く読取り互換Formは、登録済みStructured Test adapterのbuilt-in kind宣言またはcompatibility matcherのうちちょうど1件だけがschemaを受理する場合に限ってin-memoryでownerを補える。

### DES-630

登録済みStructured Test adapterのbuilt-in kind宣言またはcompatibility matcherのうちschemaを受理するものが0件または複数件なら操作を拒否し、ファイルを書き換えない。

### DES-631

matcherはsource bytes、schema field / validator集合等から決定論的に判定し、form kindの文字列だけを理由に汎用fallbackしてはならない。

### DES-632

document / VO / Relation / 判断記録 / 承認記録 / Evidence も §3 のスキーマに対応するstructを定義する。

### DES-633

インメモリのグラフを構築する。

### DES-634

検証グラフのノードは `DOC`、`VO`、`TEST`、`SRC`（ロケータ単位）である。

### DES-635

検証グラフのエッジ `DOC → DOC` は `derives_from` であり、documentレコード由来である。

### DES-636

検証グラフのエッジ `VO → DOC` は `derives_from` であり、VOレコード由来、1:N（1件以上）である。

### DES-637

検証グラフのエッジ `VO → VO` は `parent` である。

### DES-638

検証グラフのエッジ `TEST → VO` は `covers` であり、adapter所有のTest metadata宣言由来である。

### DES-639

検証グラフのエッジ `TEST → SRC` は `targets` であり、検証対象をSource Targetとして実現する形態、1:N（`rust-cargo` では `targets ≥ 1`）である（§4.1）。

### DES-640

検証グラフは、`rel/` 由来の外部Relationをエッジとして持つ。

### DES-641

検証グラフは、VO → Tests、SRC → Tests、DOC → VOs、DOC → DOCs（下流）の逆引きインデックスを持つ。

### DES-642

旧モデルのSPEC / REQノードとREQ→SPEC / VO→REQエッジは持たない。

### DES-643

上流文書はすべてDOCノードとし、文書間・VO→文書は `derives_from` の一種で表現する（§19）。

*導出元: SPEC-121, SPEC-122, SPEC-123, SPEC-124, SPEC-125, SPEC-126, SPEC-127, SPEC-128, SPEC-129, SPEC-130, SPEC-131, SPEC-132, SPEC-133, SPEC-134, SPEC-135, SPEC-136, SPEC-137, SPEC-138, SPEC-139, SPEC-140, SPEC-141, SPEC-142, SPEC-143, SPEC-144, SPEC-145, SPEC-146, SPEC-147, SPEC-148, SPEC-149, SPEC-150, SPEC-151, SPEC-152, SPEC-153, SPEC-154, SPEC-155, SPEC-156, SPEC-157, SPEC-158, SPEC-159, SPEC-160, SPEC-161, SPEC-162, SPEC-163, SPEC-164*

*引用: 基本仕様 §3.1, 基本仕様 §3.2*

### DES-644

関係型（`derives_from` / `covers` / `targets` / 外部Relation）は横断トレース可能とするが、単一へ潰さず、また意味論的に増殖もさせない。

### DES-645

E-SCAN-001はerrorであり、adapterのsource構文解析失敗（DiscoveryBatchは `Incomplete`）を意味する。

### DES-646

E-SCAN-002はerrorであり、Test ID重複（identity collision）を意味する。

### DES-647

E-SCAN-003はerrorであり、`covers` の参照先VOが存在しない（dangling reference）ことを意味する。

### DES-648

E-SCAN-004はerrorであり、`target` のロケータ／SRC IDを解決できないことを意味する。

### DES-649

E-SCAN-005はerrorであり、adapter所有の宣言で重複不可fieldが重複、または綴りの異なる複数の `target` 宣言が同一canonical Source Targetへ解決することを意味する。

### DES-650

E-SCAN-006はerrorであり、Test constructのadapter所有の宣言に未知fieldが存在することを意味する（非Test construct表面はW-SCAN-105）。

### DES-651

E-SCAN-007はerrorであり、必須metadata（core中立: id / covers ≥ 1 / intent、および当該adapterが必須とする追加metadata。`rust-cargo` では targets ≥ 1）の欠落を意味する。

### DES-652

E-SCAN-008はerrorであり、VOのparent不在または循環を意味する。

### DES-653

E-SCAN-009はerrorであり、Relationのfrom / toが不在であることを意味する。

### DES-654

E-SCAN-010はerrorであり、レコードのid / ファイル名 / schema不一致、または互換正規化後のlogical record ID重複を意味する。

### DES-655

E-SCAN-011はerrorであり、恒久SRC IDが複数adapterまたは複数Source Targetで衝突することを意味する。

### DES-656

E-SCAN-012はerrorであり、VOの `derives_from` が存在しないdocumentを参照、またはdocumentの `derives_from` が存在しないdocumentを参照する（文書鎖のリンク切れ）ことを意味する。

### DES-657

E-SCAN-016はerrorであり、根に指定されない孤児document（親documentを持たず `doc.roots` にも列挙されない）を意味する（§5.6）。

### DES-658

W-SCAN-101はwarningであり、adapterが発見したが管理宣言に対応しないTest construct（unregistered test）を意味する。

### DES-659

W-SCAN-102はwarningであり、どのVOからも参照されず、Testも参照しない孤立VOを意味する。

### DES-660

W-SCAN-103はwarningであり、`covers` を持つが対応VOがleafでない（中間VO直接参照。許容するが警告）ことを意味する。

### DES-661

W-SCAN-105はwarningであり、Test constructとして解析されない関数itemのdoc comment内の `@vtest.` 行に認識されないキーが存在することを意味する（打鍵ミス検出。`src-id` の重複はE-SCAN-005）（§4.2）。

### DES-662

W-STORE-001はwarningであり、VO recordに非正典の読取り互換field `status` が存在することを意味する（値は無視し承認から導出）。

### DES-663

W-STORE-002はwarningであり、Approvalが現在の上流依存closureを欠くか一致せず、承認として無効であることを意味する。

### DES-664

W-STORE-003はwarningであり、判断記録が `judgment_kind` を欠くか値域外で、いずれの実効判断へも寄与しないことを意味する（§8.5）。

### DES-665

W-STORE-004はwarningであり、同一 `(subject, judgment_kind)` に判断値の食い違う有効判断記録が併存し、実効判断が未確定であることを意味する（§8.5）。

### DES-666

W-STORE-005はwarningであり、判断記録または承認レコードの `supersedes` の参照先を解決できない、対象が一致しない、またはsupersede関係が循環し、当該recordが実効集合へ寄与しないことを意味する（§8.5・§3.5）。

### DES-667

W-STORE-006はwarningであり、承認レコードの `approved_state` または `subject` の種別が値域外、あるいは `judgment_ref` の参照先が存在せず、実効承認を導出しないことを意味する（§3.5）。

### DES-668

errorは該当エンティティに関わる検査を非 `PASS` にする。

### DES-669

warningは診断severityだけでは検証値を変更しない。

### DES-670

warningはレポートに常に表示する。

### DES-671

管理宣言の欠落・E-SCAN-007（必須metadata欠落）が示す `ManagedTestLink::Missing` は `chain_integrity = MISMATCH`（診断 `MISSING`）に写像する。

### DES-672

`ManagedTestLink::Multiple`、E-SCAN-002（Test ID衝突）、E-SCAN-003（解決不能なVO参照）は `chain_integrity = MISMATCH` に写像する。

### DES-673

E-SCAN-003が発生しても対応するTest Entityと `ManagedTestLink::One` を除去しない。

### DES-674

E-SCAN-008（VO parent不在・循環）、E-SCAN-009（Relation dangling）、E-SCAN-012（文書鎖・VO derives_fromのリンク切れ）は `chain_integrity = MISMATCH` に写像する。

### DES-675

E-SCAN-016（孤児document）は `orphan_detection = MISMATCH` に写像する（§5.6）。

### DES-676

E-SCAN-011があるSRC ID参照は曖昧なため、関係するtarget解決を `MISMATCH` とし、いずれのSource Targetも選択しない。

### DES-677

候補の1件を解決結果としてEvidence・検証へ永続化しない（§6.1）。

### DES-678

衝突する恒久SRC IDを宣言した各Source Target自体は、canonical locatorで独立に具体化されたまま保持する。

### DES-679

`rust-cargo` adapterは§5.1の `DiscoveryBatch` を構築する。

### DES-680

`vtest-scan` はこれらのRust固有処理を実行しない。

### DES-681

当該adapterは検証対象をSource Targetとして実現する形態であり、各管理対象Testに1件以上のSource Target（`targets ≥ 1`）を必須とする。

### DES-682

各管理対象Testに1件以上のSource Target（`targets ≥ 1`）を必須とすることはadapter層に属し、core中立の `chain_integrity` 必須リンクではない（§4.1・§11.1.1）。

### DES-683

欠落はE-SCAN-007として報告する（§4.4・§5.4）。

### DES-684

したがって `rust-cargo` のTestは従来どおりSource Target宣言を要し、挙動・Eコード・fixtureは本改訂で実効的に変わらない。

### DES-685

`rust-cargo` discoveryの第1段はファイル探索であり、adapter configのinclude配下の `*.rs` をignoreクレートで列挙する（`.gitignore` 準拠、`target/` は除外）。

### DES-686

`rust-cargo` discoveryの第2段は構文解析であり、ファイルごとに `syn::parse_file` する。解析エラーのファイルはE-SCAN-001を返し、batchをIncompleteとする。

### DES-687

`rust-cargo` discoveryの第3段はモジュールパス構築であり、crateルート（`src/lib.rs` / `src/main.rs` / `tests/*.rs`）からmod宣言を辿り、各itemの完全モジュールパスを構築する。

### DES-688

`rust-cargo` discoveryの第4段はTest construct抽出であり、属性pathの末尾segmentが"test"である関数（`#[test]`、`#[tokio::test]` 等）を抽出する。

### DES-689

`rust-cargo` discoveryの第5段はmetadata宣言抽出であり、doc属性（`#[doc = "..."]`）を§4.2の文法でparseする（id / covers / target / intent / input / expect / kind / case / related）。

### DES-690

`rust-cargo` discoveryの第6段はSource Target抽出であり、すべてのfn / impl fnをSRC候補として索引化し、§4.3のlocator解決・逆引き・`@vtest.src-id` 認識（非Test constructの宣言に限る）に使用する（§4.2）。

### DES-691

`rust-cargo` discoveryの第6段（Source Target抽出）で非Test constructのdoc comment内の `@vtest.` 行を検査し、認識されないキーからW-SCAN-105を、`src-id` の重複からE-SCAN-005を生成する（§4.2）。

### DES-692

`rust-cargo` discoveryの第7段はdraft生成であり、全Discovered Test draft、ManagedTestDraftLink、SourceTargetDraft、Source Location、construct / metadata source rangeとbytes、logical metadata、宣言された恒久SRC ID、ExecutionDescriptor、診断をhash未計算のDiscoveryBatchに格納する。

### DES-693

`orphan_detection` は文書層の孤児検出であり、親（上流document）を持たない `document` ノードが存在するかを問う。

*導出元: REQ-059, REQ-060, REQ-061, REQ-062, REQ-063, SPEC-235, SPEC-236, SPEC-237, SPEC-238, SPEC-239, SPEC-240, SPEC-241*

*引用: 基本仕様 §5.2, 要件定義 §4.2*

### DES-694

`config.yaml` の `doc.roots` に列挙されたDOC IDを根として扱い、`orphan_detection` の対象外とする（§2.2）。

### DES-695

根指定は `.verify/` 設定として保持する。

*導出元: SPEC-235, SPEC-236, SPEC-237, SPEC-238, SPEC-239, SPEC-240, SPEC-241*

*引用: 基本仕様 §5.2*

### DES-696

根指定の追加・削除は `vtest doc` コマンドの引数で管理し `doc.roots` へ反映する。

*導出元: SPEC-665, SPEC-666, SPEC-667, SPEC-668, SPEC-669, SPEC-670, SPEC-671, SPEC-672, SPEC-673, SPEC-674, SPEC-675, SPEC-676, SPEC-677, SPEC-678, SPEC-679, SPEC-680, SPEC-681, SPEC-682, SPEC-683, SPEC-684, SPEC-685, SPEC-686, SPEC-687, SPEC-688, SPEC-689*

*引用: 基本仕様 §26.1, 別紙A*

### DES-697

`derives_from` が空、かつ他のどのdocumentからも `derives_from` で参照されないdocumentのうち、`doc.roots` に列挙されないものを孤児とし、E-SCAN-016（`orphan_detection = MISMATCH`）とする。

### DES-698

`orphan_detection` の対象は文書層のみである。実装レイヤーの孤児検出（宣言されていない実装の検出）は行わない。

*導出元: R-2, REQ-292, SPEC-729, SPEC-730, SPEC-731, SPEC-732, SPEC-733, SPEC-734, SPEC-735, SPEC-736, SPEC-737, SPEC-738, SPEC-739*

*引用: 要件定義 R-2, 基本仕様 §29 OOS-005*

### DES-699

旧モデルのW-SCAN-102（孤立VO）はVO層の警告であり、文書層 `orphan_detection` とは別物として存置する。

### DES-700

根に指定されたdocumentが存在しないDOC IDを参照する場合は、config invariant違反としてE-CONFIG-001とする。

## DA-007 6. Target Reference解決

### DES-701

coreは`TargetRef::Locator.adapter`をregistryで解決する。

### DES-702

coreは、opaque locatorの解釈を該当する`SourceDiscoveryAdapter`へ委譲する。

### DES-703

adapterは正規化されたTarget Reference、Source Location、source range、content bytes、解決status、候補を返す。

### DES-704

coreは、返却されたadapter IDとTarget Referenceの一致、source rangeの範囲、current bytesとの一致を検証する。

### DES-705

coreは§1.3のSource Target hashを計算する。

### DES-706

coreはopaque locatorの内部構文は解釈しない。

### DES-707

解決が0件または複数候補で一意に定まらない場合はE-SCAN-004とする。

### DES-708

解決が0件または複数候補で一意に定まらない場合、推測で候補を選択しない。

### DES-709

SRC ID参照はcoreが統合済みSRC索引で一意性を検査する。

### DES-710

SRC ID参照は対応するadapterのSource Locationとsource rangeを使用する。

### DES-711

SRC ID参照は当該恒久SRC IDを宣言したSource Targetのcanonical locatorへ解決する。

### DES-712

SRC ID参照は、同じSource Targetへのlocator参照と同一のcanonical Source Target・同一のSource Target hashへ到達する。

### DES-713

解決結果をlocator版とSrcId版の別entityへ分岐させない。

### DES-714

恒久SRC IDが複数adapterまたは複数Source Targetで衝突する場合はE-SCAN-011とする。

### DES-715

恒久SRC IDが複数adapterまたは複数Source Targetで衝突する場合、いずれのSource Targetも選択しない。

### DES-716

解決結果は「解決済み」「対象なし」「曖昧」の3状態を区別する。

### DES-717

曖昧はfail-closedな終端状態とする。

### DES-718

曖昧な解決から代表候補を選ばない。

### DES-719

曖昧な解決について、解決済みのcanonical Source Targetを要求する後段（静的解析、Evidence、`target_coverage`、鮮度判定）へ候補を1件も引き渡さない。

### DES-720

候補は§6.3の診断表示にだけ用い、表示できることを選択の根拠にしない。

### DES-721

曖昧な解決について候補を後段へ1件も引き渡さないという禁止はTarget Referenceの解決に関するものであり、Source Targetの具体化を止めるものではない。

### DES-722

各Source Targetは自身のcanonical locatorで独立に具体化され、恒久SRC IDが衝突していても`SourceTargetDraft`ごとに1件のSource Targetとして成立する。

### DES-723

衝突が壊すのは当該恒久SRC IDによる参照の一意性だけである。

### DES-724

Target Referenceの解決はcoreの単一経路が所有し、静的解析、実行、Evidence writer、検証集約はいずれもTarget Referenceの解決の結果を消費する。

### DES-725

各subsystemが独自にcandidate列を走査して1件を選ぶ経路を持ってはならない。

### DES-726

E-SCAN-004またはE-SCAN-011で解決できなかったtargetを、後段が任意の候補で埋めて記録・永続化することを禁ずる。

### DES-727

Source Target identityは「宣言されたTargetRef（Locator / SrcId）→ resolve（§6.1）→ canonical Locator」の一方向でだけ確定する。

> TestEntity.targets = 宣言されたTargetRef（Locator / SrcId） / ↓ resolve（§6.1） / Canonical Source Target = canonical Locator / ↓ / Evidence / target_coverage / 検証 = canonical Locatorをidentityとして使用

### DES-728

Evidence（§3.6、§9.4）、`target_coverage`（§10.2）、および鮮度判定（§11.2）は、解決後のcanonical Locatorをtarget identityとして記録・比較する。

### DES-729

参照側Testが宣言した`TargetRef`の綴り（SRC ID参照を含む）をこれらのidentityとして保存してはならない。

### DES-730

Testがどう宣言したか（同じSource Targetに対するLocator参照からSRC ID参照への書き換え等）の変更は、`targets`をcanonical metadataとして束縛する§1.3のTest subject hashが捕捉する。

### DES-731

Evidence側で宣言表現を保持する必要はなく、保持すれば同一Source Targetが参照方法ごとに別identityへ分裂する。

### DES-732

Testの宣言target集合は解決後のcanonical Source Target単位で一意でなければならない。

### DES-733

綴りの異なる複数の宣言が同一のcanonical Source Targetへ解決する場合は重複targetとしてE-SCAN-005とする。

### DES-734

`rust-cargo`のlocator`path::item-path`の解決は、§5.5で構築したSRC索引への完全一致検索とする。

### DES-735

`rust-cargo`のlocator解決は、pathが索引に存在するかを確認する。

> 1. path が索引に存在するか / 2. path 内で item-path が一致する fn / impl fn が存在するか / 3. 一意に決まらない場合（同名 fn が cfg 分岐で複数等）はすべて候補として返し、解決失敗（E-SCAN-004）とする

### DES-736

`rust-cargo`のlocator解決は、path内でitem-pathが一致するfn / impl fnが存在するかを確認する。

### DES-737

`rust-cargo`のlocator解決で一意に決まらない場合（同名fnがcfg分岐で複数等）は、すべて候補として返し、解決失敗（E-SCAN-004）とする。

### DES-738

Structured Operationの入力検証（別紙A §14、§15）で解決に失敗した場合、coreはadapterが返した候補を共通envelopeで表示する。

*引用: 別紙A §14, 別紙A §15*

### DES-739

`rust-cargo` adapterは、item-pathの末尾セグメント一致（別パスの同名関数）の順で候補を構築する。

> 1. item-path の末尾セグメント一致（別パスの同名関数） / 2. 編集距離 2 以内の近似名 / 出力例： ✗ symbol not found: src/parser.rs::Parser::prase / candidates: src/parser.rs::Parser::parse / src/parser.rs::Parser::parse_inner

### DES-740

`rust-cargo` adapterは、編集距離2以内の近似名の順で候補を構築する。

### DES-741

`rust-cargo` adapterのenum variant検証（`expect`の値が`ParseError::InvalidUtf8`形式の場合）は、スキャン済みASTからenum定義を検索する。

### DES-742

`rust-cargo` adapterのenum variant検証は、解決できる場合のみ検証する。

### DES-743

`rust-cargo` adapterのenum variant検証は、解決できない自由記述はそのまま受理する（best effort。拒否はしない）。

## DA-008 7. Static Analysis orchestrationと`rust-cargo`ルール

### DES-744

静的解析は`oracle_presence`（照合装置の存在）へ証拠を供給する。

### DES-745

静的解析は`target_binding`の静的到達証明（DA-002）へ証拠を供給する。

### DES-746

決定論的解析結果は正典レコードを持たず、検証のたびに現在のsource / configから再計算する派生情報である。

*導出元: P-003*

*引用: 基本仕様 P-003*

### DES-747

`vtest audit static`は要求時に解析を起動し、結果をstdoutと`cache/`へ出力する。

*導出元: SPEC-665, SPEC-666, SPEC-667, SPEC-668, SPEC-669, SPEC-670, SPEC-671, SPEC-672, SPEC-673, SPEC-674, SPEC-675, SPEC-676, SPEC-677, SPEC-678, SPEC-679, SPEC-680, SPEC-681, SPEC-682, SPEC-683, SPEC-684, SPEC-685, SPEC-686, SPEC-687, SPEC-688, SPEC-689*

*引用: 基本仕様 §26.1*

### DES-748

`vtest audit static`は判断記録（§8）とは別機構であり、外部判断の記録には転用しない。

### DES-749

各ルールは`FAIL` / `UNKNOWN` / `PASS(違反なし)`のいずれかを返す。

### DES-750

決定論的に確定できる違反のみFAILとする。

### DES-751

解析の限界で確定できない場合はFAILではなくUNKNOWNとする。

### DES-752

`UNKNOWN`は意味判定できる者への判断記録エスカレーション（§8）の領分である。

*導出元: SPEC-376, SPEC-377, SPEC-378, SPEC-379, SPEC-380, SPEC-381, SPEC-382, SPEC-383, SPEC-384, SPEC-385, SPEC-386, SPEC-387, SPEC-388, SPEC-389, SPEC-390, SPEC-391, SPEC-392, SPEC-393, SPEC-394, SPEC-395, SPEC-396, SPEC-397, SPEC-398, SPEC-399, SPEC-400, SPEC-401, SPEC-402, SPEC-403, SPEC-404, SPEC-405, SPEC-406, SPEC-407, SPEC-408*

*引用: 基本仕様 §11*

### DES-753

ただしDA-002のtarget到達UNKNOWNは§7.3のruntime到達証明で解決し、判断記録へは委ねない。

### DES-754

`oracle_presence`はDA-001 / DA-003 / DA-004 / DA-005 / DA-006の合成とする。

### DES-755

`oracle_presence`は、全ルールが違反なしなら`PASS`とする。

*導出元: REQ-073, REQ-074, REQ-075, REQ-076, REQ-077, REQ-078, REQ-079, REQ-080, REQ-081, REQ-082, REQ-083, REQ-084, SPEC-263, SPEC-264, SPEC-265, SPEC-266, SPEC-267, SPEC-268, SPEC-269, SPEC-270, SPEC-271, SPEC-272, SPEC-273, SPEC-274*

*引用: 基本仕様 §5.4, 要件定義 §4.4*

### DES-756

`oracle_presence`は、1つでも`FAIL`があれば`FAIL`とする。

*導出元: REQ-073, REQ-074, REQ-075, REQ-076, REQ-077, REQ-078, REQ-079, REQ-080, REQ-081, REQ-082, REQ-083, REQ-084, SPEC-263, SPEC-264, SPEC-265, SPEC-266, SPEC-267, SPEC-268, SPEC-269, SPEC-270, SPEC-271, SPEC-272, SPEC-273, SPEC-274*

*引用: 基本仕様 §5.4, 要件定義 §4.4*

### DES-757

`oracle_presence`は、`FAIL`がなく`UNKNOWN`があれば`UNKNOWN`とする。

*導出元: REQ-073, REQ-074, REQ-075, REQ-076, REQ-077, REQ-078, REQ-079, REQ-080, REQ-081, REQ-082, REQ-083, REQ-084, SPEC-263, SPEC-264, SPEC-265, SPEC-266, SPEC-267, SPEC-268, SPEC-269, SPEC-270, SPEC-271, SPEC-272, SPEC-273, SPEC-274*

*引用: 基本仕様 §5.4, 要件定義 §4.4*

### DES-758

`oracle_presence`に動的な昇格経路は無い。

### DES-759

静的解析は不成立の証明であり、証明の失敗は`UNKNOWN`であって、runtime証拠で`PASS`へ昇格しない。

### DES-760

`target_binding`の静的到達証明はDA-002が担う（§7.3）。

### DES-761

DA-002のtarget別verdictの`UNKNOWN`は「静的解析の到達判定境界の外にあり、静的には到達を証明できない」ことだけを表し、到達しないことを意味しない。

### DES-762

DA-002のtarget別verdictのUNKNOWNは§7.3に従い当該targetのruntime計測（§10）が実行を証明した場合に限り充足される。

> 要件定義 §4.3の2証拠源モデルは`target_binding`に固有。

*導出元: REQ-064, REQ-065, REQ-066, REQ-067, REQ-068, REQ-069, REQ-070, REQ-071, REQ-072*

*引用: 要件定義 §4.3*

### DES-763

`vtest-audit`は`TestEntity.execution.adapter`をregistryで解決する。

### DES-764

`vtest-audit`は、Test、全Target Reference、各source range、content hash、および選択adapterの現在configを`StaticAnalysisAdapter`へ渡す。

### DES-765

adapterはrule ID、verdict、根拠span、解析限界を返す。

### DES-766

target-scopedなDA-002 / DA-003については、宣言targetごとのverdictと根拠spanを（規則単位のverdictへ畳み込む前の形で）返す。

### DES-767

target-scopedなDA-002 / DA-003の集合を全宣言targetと過不足なく1対1に対応させる。

### DES-768

coreはadapter ID、source location・現在bytesとの対応、決定論的encodingを検証し、§7.2の規則で集約する。

### DES-769

adapter固有のASTやassertion構文をcoreは解釈しない。

### DES-770

Static Analysis capabilityがない場合は`NO_EVIDENCE`（診断`NOT_CHECKED`）とする。

### DES-771

adapterが不完全、解析限界、または解析入力集合の不完全性を報告した場合は`UNKNOWN`とし、違反なしと推測しない。

### DES-772

`rust-cargo`のassert相当の構文はDA-001〜DA-006で共通に用いる。

> assert!/assert_eq!/assert_ne!/panic! を含む標準マクロおよびconfigのassertion_macros列挙マクロ、#[should_panic]属性、.unwrap()/.expect(..)/?演算子（Result/Optionの成立検証として扱う）、Test関数がResultを返しErrを返しうる構造、の4分類の総称。

### DES-773

`rust-cargo`のassert相当の構文は`assert!` / `assert_eq!` / `assert_ne!` / `panic!`を含む標準マクロ、および`rust-cargo` configの`assertion_macros`に列挙されたマクロを含む。

### DES-774

`rust-cargo`のassert相当の構文は`#[should_panic]`属性を含む。

### DES-775

`rust-cargo`のassert相当の構文は`.unwrap()` / `.expect(..)` / `?`演算子（Result / Optionの成立検証として扱う）を含む。

### DES-776

`rust-cargo`のassert相当の構文はTest関数が`Result`を返し`Err`を返しうる構造を含む。

### DES-777

DA-001（定数アサーション）はoracle_presenceへ供給する検査であり、引数がすべてリテラル・定数式のassertを内容とし、関数内のassert相当がすべて定数アサーションであることをFAIL条件とし、定数性を確定できない式をUNKNOWNへ退避する例とする。

### DES-778

DA-002（対象未呼出）はtarget_bindingへ供給する検査であり、宣言されたtargetシンボルを呼んでいないことを内容とし、関数本体および同一ファイル内の呼出先helper（1段）を探索して呼出が存在しない、かつ他ファイルへの呼出も存在しないことをFAIL条件とし、他ファイル・他クレートの関数呼出があり間接呼出の可能性を排除できない場合をUNKNOWNへ退避する例とする。

### DES-779

DA-003（結果未検証）はoracle_presenceへ供給する検査であり、targetを呼ぶがその結果をassert相当で一切検証しないこと（照合の委譲先がある場合は§7.2.1で終端を確認する）を内容とし、target呼出結果（戻り値、および結果から派生した束縛）がassert相当に到達しない、かつ`#[should_panic]`がないことをFAIL条件とし、結果が可変参照・グローバル状態経由で検証される可能性がある場合をUNKNOWNへ退避する例とする。

### DES-780

DA-004（自己比較）はoracle_presenceへ供給する検査であり、`assert_eq!(a, b)`でaとbがトークン列として同一であることを内容とし、該当assertが存在することをFAIL条件とし、UNKNOWNへ退避する例は無い（構文的に確定）。

### DES-781

DA-005（空テスト）はoracle_presenceへ供給する検査であり、関数本体に文が存在しないことを内容とし、該当することをFAIL条件とし、UNKNOWNへ退避する例は無い。

### DES-782

DA-006（検証構文なし）はoracle_presenceへ供給する検査であり、関数内にassert相当が1つも存在しないことを内容とし、関数内にassert相当が1つも存在せず、かつ§7.2.1の照合の委譲先も同定できないことをFAIL条件とし、委譲先を同定できるが終端を確認できない場合（§7.2.1）をUNKNOWNへ退避する例とする。

### DES-783

W-DA-101（ignored）は`#[ignore]`属性を内容とし、FAILにしない警告のみであり、実行されなければ`target_binding`が診断NOT_EXECUTEDになる。

### DES-784

DA-002 / DA-003のデータフロー解析は関数内のローカル束縛の追跡（let束縛、メソッドチェーン、フィールドアクセス）までとする。

### DES-785

DA-002 / DA-003のデータフロー解析はクロージャ内・マクロ展開内はUNKNOWNとする。

### DES-786

複数target TestではDA-002 / DA-003を各targetへ個別適用する。

### DES-787

target別結果に1件でもFAILがあればrule結果をFAILとする。

### DES-788

FAILがなく1件でもUNKNOWNがあればrule結果をUNKNOWNとする。

### DES-789

全targetが違反なしの場合だけrule結果をPASSとする。

### DES-790

静的解析は再計算派生であるため、これらのtarget別verdictと規則単位verdictは検証のたびに現在sourceから計算し、正典レコードへ永続化しない（§7.1）。

### DES-791

宣言targetへの呼出がTest本体に静的に現れない場合（subprocessを起動して別プロセスでtargetを実行する等、target呼出がsource内に存在しない）、DA-003の当該target別verdictをUNKNOWNとする。

### DES-792

呼出結果を観測できないことを「違反なし（空虚PASS）」とも「結果未到達（空虚FAIL）」とも判定しない。

### DES-793

宣言targetへの呼出がTest本体に静的に現れない場合、DA-002も同targetでUNKNOWNであり、DA-002が§7.3のruntime証明で救済されてもDA-003はUNKNOWNのまま`oracle_presence`へ寄与するため、呼出が本体に現れないTest（典型的なsubprocess E2E）はoracle_presence = PASSに到達しない。

### DES-794

target呼出はTest本体に現れるがDA-002がUNKNOWNになる場合（他ファイル・他クレートへの直接呼出で間接呼出の可能性を排除できない等）、その呼出結果がTest本体内でassert相当へ到達すればDA-003 = PASSになりうる。

### DES-795

target呼出はTest本体に現れるがDA-002がUNKNOWNになる場合のtargetは、DA-002をruntimeで救済すれば`target_binding` = PASSに到達しうる（runtime救済で実益が出る型）。

### DES-796

クロージャ・マクロ展開の内側での到達は§7.2の一般則どおりDA-002 / DA-003ともUNKNOWNとする。

### DES-797

ルールごとの判定結果と根拠（該当スパン）は`vtest audit static`の出力および`cache/`の派生結果として提示する。

### DES-798

Testの成否判定が、assert相当の構文でなく通常の関数へ委譲されている場合、その委譲先が検証閉包の中で終端しない限り、照合装置検査の成立側を確定しない。

### DES-799

assert相当の引数、または引数へ到達する束縛（§7.2のデータフロー解析の範囲内）に現れる関数呼出は当該Testの照合の委譲先とする。

> 呼出先はadapterが列挙するassert相当の構文に含まれないものに限る。

### DES-800

関数内にassert相当が1つも存在しないTest本体に現れる関数呼出は当該Testの照合の委譲先とする。

> 呼出先はadapterが列挙するassert相当の構文に含まれないものに限る。

### DES-801

委譲先`H`は、`H`を宣言targetとするTestが1件以上存在し、それらすべての`oracle_presence`が`PASS`であるとき、かつそのときに限り終端する。

### DES-802

終端の判定は covers / 宣言target のグラフの参照だけで行う。

### DES-803

終端の判定について、信頼を宣言する専用の記録・注釈・設定項目は設けない。

### DES-804

委譲先が無い（assert相当の構文だけで照合が完結する）場合、DA-003 / DA-006は従来どおり各ルールのFAIL条件で評価する。

> 判定。

### DES-805

委譲先がすべて終端する場合、DA-003 / DA-006は違反なしとする。

### DES-806

委譲先を同定できるが、それを宣言targetとするTestが0件の場合、DA-003 / DA-006は`UNKNOWN`とする。

### DES-807

委譲先を宣言targetとするTestが存在するが、その`oracle_presence`が`PASS`でない（`FAIL` / `UNKNOWN` / `NO_EVIDENCE`のいずれか）場合、DA-003 / DA-006は`UNKNOWN`とする。

### DES-808

終端の探索が循環する（`H`を宣言targetとするTestの照合が`H`自身へ、または相互に委譲される）場合、DA-003 / DA-006は`UNKNOWN`とする。

### DES-809

委譲先が他ファイル・他クレート・マクロ展開内にあり呼出先を同定できない場合、DA-003 / DA-006は`UNKNOWN`とする。

### DES-810

終端を確認できない委譲先を、違反なし（成立側）としても`FAIL`としても扱わない。

> 前者は未検証の照合装置を成立と読み替えることになり、後者は解析の限界を確定した違反と読み替えることになる（§7.1）。

### DES-811

終端の判定は同一のscan結果から算出する（§11.1の決定性）。

### DES-812

他Testの`oracle_presence`を参照するため、循環は上表のとおり`UNKNOWN`で閉じ、評価順序によって結果が変わる経路を作らない。

### DES-813

委譲先が終端したことは当該Testの`oracle_presence`を昇格させる根拠にはならず、上表の枝が示すのはDA-003 / DA-006の値だけである。

### DES-814

`oracle_presence`全体は§7.1の合成規則で決まる。

### DES-815

`target_binding`は「そのTestが検証対象とする振る舞いが実際に生じ、その振る舞いを反映した観測が得られたか」を問う。

*導出元: REQ-064, REQ-065, REQ-066, REQ-067, REQ-068, REQ-069, REQ-070, REQ-071, REQ-072, SPEC-242, SPEC-243, SPEC-244, SPEC-245, SPEC-246, SPEC-247, SPEC-248, SPEC-249, SPEC-250, SPEC-251, SPEC-252, SPEC-253, SPEC-254, SPEC-255, SPEC-256, SPEC-257, SPEC-258, SPEC-259, SPEC-260, SPEC-261, SPEC-262*

*引用: 基本仕様 §5.3, 要件定義 §4.3*

### DES-816

`target_binding`は静的解析（DA-002）と動的計測（§10 coverage）の2証拠源を持ち、静的に確定できなければ`UNKNOWN`とし動的証拠で昇格できる。

### DES-817

DA-002は§7.2の解析境界（関数本体および同一ファイル内helper1段。クロージャ内・マクロ展開内・他ファイル・他クレートへの呼出は§7.1 / §7.2に従いUNKNOWN）で行う静的なtarget到達証明である。

### DES-818

Testがtargetを静的解析の追えない実行境界を越えて到達させる形態はいずれもDA-002のUNKNOWNとして現れる。

### DES-819

Testがtargetを静的解析の追えない実行境界を越えて到達させる形態は、Testのkind（unit / integration）とは独立に、execution topologyによって決まる。

### DES-820

静的解析の追えない実行境界は、他ファイル・他クレートへの呼出を介した間接到達を含む。

### DES-821

静的解析の追えない実行境界は、クロージャ・マクロ展開内での到達を含む。

### DES-822

静的解析の追えない実行境界は、生成した別スレッド（in-process, thread boundary）での到達を含む。

### DES-823

静的解析の追えない実行境界は、別プロセス（subprocessを起動し、そのプロセス内でtargetを実行するprocess boundary）での到達を含む。

### DES-824

到達要件は、targetごとに、静的証明：当該targetのDA-002 verdict = PASS（§7.2の解析境界内で呼出を確認）のいずれかで充足される。

### DES-825

到達要件は、targetごとに、runtime証明：§11.2が選択した最新Evidenceが§6のハッシュ束縛（鮮度）を満たすとき、そのEvidenceの§10.2target別`target_coverage` result = PASS（`checked: true`かつ実行count > 0）のいずれかで充足される。

### DES-826

DA-002 verdictがUNKNOWN（静的に証明できない）であるtargetは、runtime証明が成立するときに限り到達要件を満たす。

### DES-827

複数target Testではtargetごとに到達要件を適用する。

### DES-828

Testの`target_binding`到達は全宣言targetの到達要件が充足された場合にのみ成立する。

### DES-829

static側は§7.2のDA-002 verdictをtargetごとに用いる。

### DES-830

DA-002 verdict = FAIL（解析境界内で到達を静的に否定）はruntime証明で覆さない。

### DES-831

runtime側は§11.2が選択した最新Evidenceだけを用いる。

### DES-832

最新Evidenceが鮮度を満たさなければruntime証明は成立せず、古いEvidenceへフォールバックしない。

> これにより同一検証内で計測が§11.2でSTALEの一方target_bindingが別EvidenceでPASSになる履歴不一致を防ぐ。

### DES-833

`target_binding`項目値は検証時に算出する。

### DES-834

static側は§7.2のDA-002を再計算し、runtime側は§11.2選択Evidenceを用いて、targetごとに実効到達状態を定める。

### DES-835

静的到達とは、DA-002 verdict = PASSである状態をいう。

### DES-836

runtime到達とは、DA-002 verdict = UNKNOWNかつruntime証明成立である状態をいう。

### DES-837

未充足とは、DA-002 verdict = FAIL、またはUNKNOWNでruntime証明が成立しない状態をいう。

### DES-838

`target_binding`は、Evidenceの`result: FAIL`（テストランナーが失敗を報告）なら`FAIL`とする。

### DES-839

`target_binding`は、そうでなく全宣言targetの到達が静的到達またはruntime到達で充足されれば`PASS`とする。

### DES-840

到達未充足のtargetがあれば§11.2の写像に従い非`PASS`（動的計測count 0は`FAIL`/診断NOT_EXECUTED、計測不能・未計測は`NO_EVIDENCE`、解析限界は`UNKNOWN`）とする。

### DES-841

`target_binding`の到達要件が静的到達またはruntime到達の充足によってのみ`PASS`となる関係はfail-closedを保つ。

### DES-842

runtime証明は当該targetの`target_coverage` = PASSのときだけ成立する。

### DES-843

`target_coverage`がFAIL（count 0）・UNKNOWN（関数不見当）・NOT_CHECKED（coverage利用不能、未計測、`--fast`）のときは到達要件を満たさず、当該targetは未充足となり、`target_binding`を非`PASS`にする。

### DES-844

本節の到達要件は検証対象をSource Targetとして実現する形態に限定する（`rust-cargo`）。

> 基本仕様 §5.3「実装 construct（Source Target）を検証対象とする実行形態では…」

*導出元: SPEC-242, SPEC-243, SPEC-244, SPEC-245, SPEC-246, SPEC-247, SPEC-248, SPEC-249, SPEC-250, SPEC-251, SPEC-252, SPEC-253, SPEC-254, SPEC-255, SPEC-256, SPEC-257, SPEC-258, SPEC-259, SPEC-260, SPEC-261, SPEC-262*

*引用: 基本仕様 §5.3*

### DES-845

検証対象をSource Targetとして実現する形態（`rust-cargo`）で、宣言targetをどのtopologyでも実行しないTest（構造・契約のみをassertするTest）は静的にもruntimeにも到達を確立できず、到達要件は未充足のままとなる。

### DES-846

検証対象をSource Targetとして宣言しない他の実行形態（外部契約・境界上の振る舞い）の確認方法は、特定形態を他形態へ一律要求せず下位仕様・後続版へ委譲する。

*導出元: REQ-064, REQ-065, REQ-066, REQ-067, REQ-068, REQ-069, REQ-070, REQ-071, REQ-072, SPEC-242, SPEC-243, SPEC-244, SPEC-245, SPEC-246, SPEC-247, SPEC-248, SPEC-249, SPEC-250, SPEC-251, SPEC-252, SPEC-253, SPEC-254, SPEC-255, SPEC-256, SPEC-257, SPEC-258, SPEC-259, SPEC-260, SPEC-261, SPEC-262, SPEC-325, SPEC-326, SPEC-327, SPEC-328, SPEC-329, SPEC-330, SPEC-331*

*引用: 要件定義 §4.3, 基本仕様 §5.3・§8.3*

### DES-847

本節の target 実行到達規則を普遍規則として適用しない。

### DES-848

v0.1の唯一のadapter`rust-cargo`では検証対象をSource Targetとして宣言しないTestはE-SCAN-007（`targets ≥ 1`欠落）として`target_binding`評価の手前で`chain_integrity`の`MISMATCH`になる。

### DES-849

targetを持たないTestは本節の合成へ到達しない。

### DES-850

DA-003はこのto-runtime joinに含めない。

### DES-851

DA-003は`oracle_presence`（照合装置の存在）へ寄与するstatic data-flow判定であり（§7.2）、targetの「結果検証」を問う。

### DES-852

runtime coverageはtargetの「実行」を証明するが「結果検証」を証明しないため、coverageはDA-003を代替せず、DA-003は§7.2の意味論のまま維持する。

### DES-853

典型的なsubprocess E2E（targetの戻り値 → 子プロセスのstdout / exit code → 親プロセスのassert）では、このdata-flowはstatic analyzerから追えないためDA-003はUNKNOWNのまま残りやすい。

### DES-854

本節はprocess boundaryによってDA-002到達が恒久UNKNOWNになる問題だけを解消するものであり、boundary testを完全にoracle_presence PASS可能にするものではない。

## DA-009 8. 判断記録プロトコル

### DES-855

本システムは、宣言されていない義務・網羅漏れ・宣言と実装の意味のずれを、自ら発見・裁定しない。

*導出元: REQ-180, REQ-181, REQ-182, REQ-183, REQ-184, REQ-185, REQ-186, REQ-187, REQ-188, REQ-189, REQ-190, REQ-191, REQ-192, SPEC-376, SPEC-377, SPEC-378, SPEC-379, SPEC-380, SPEC-381, SPEC-382, SPEC-383, SPEC-384, SPEC-385, SPEC-386, SPEC-387, SPEC-388, SPEC-389, SPEC-390, SPEC-391, SPEC-392, SPEC-393, SPEC-394, SPEC-395, SPEC-396, SPEC-397, SPEC-398, SPEC-399, SPEC-400, SPEC-401, SPEC-402, SPEC-403, SPEC-404, SPEC-405, SPEC-406, SPEC-407, SPEC-408*

*引用: 基本仕様 §11, 要件定義 §12*

### DES-856

機械が決定論で確定できない疑義は`UNKNOWN`として外部（人間または判断可能Agent）へ引き渡し、その判断を判断記録（§3.4）として追跡する。

### DES-857

判断記録プロトコルは検証状態のゲートではない。

### DES-858

判断記録の受理は当該対象の検証状態を昇格させない。

*導出元: REQ-180, REQ-181, REQ-182, REQ-183, REQ-184, REQ-185, REQ-186, REQ-187, REQ-188, REQ-189, REQ-190, REQ-191, REQ-192, SPEC-387, SPEC-388, SPEC-389, SPEC-390, SPEC-391, SPEC-392, SPEC-393, SPEC-394, SPEC-395, SPEC-396, SPEC-397, SPEC-398, SPEC-399, SPEC-400, SPEC-401, SPEC-402, SPEC-403, SPEC-404, SPEC-405, SPEC-406, SPEC-407, SPEC-408*

*引用: 基本仕様 §11.3, 要件定義 §12*

### DES-859

`vtest audit bundle`は判断対象ごとに、判断に必要な情報をJSONとして`cache/bundles/<ULID>.json`へ出力する。

### DES-860

バンドルは派生情報でありGit管理しない。

### DES-861

提出結果の検証に必要な情報（対象の内容ハッシュ）は判断記録へ複製されるため、バンドル自体の永続化は不要である。

### DES-862

バンドルには基本仕様§11.3が定める判断対象の情報一式を含める。

*導出元: SPEC-387, SPEC-388, SPEC-389, SPEC-390, SPEC-391, SPEC-392, SPEC-393, SPEC-394, SPEC-395, SPEC-396, SPEC-397, SPEC-398, SPEC-399, SPEC-400, SPEC-401, SPEC-402, SPEC-403, SPEC-404, SPEC-405, SPEC-406, SPEC-407, SPEC-408*

*引用: 基本仕様 §11.3*

### DES-863

バンドルは対象VO（`--vo` / `--test`から導出したcovers先VOレコードとclaim）を含める。

### DES-864

バンドルはTest Intent（`--test`の場合の対象Testのintent・input・expect）を含める。

### DES-865

バンドルはテストコード（Test construct source全文とmetadata宣言）を含める。

### DES-866

バンドルはTestのcases集合（対象Testが`@vtest.case`で宣言したcaseの正規化文字列を宣言順に並べたlist。§4.1の論理field`cases[]`）を含める。

> 宣言が無いTestでは空listを明示し、項目自体を省略しない。

### DES-867

バンドルは対象実装（全宣言targetのimplementation construct source全文）を含める。

### DES-868

バンドルは関連テスト（related / 同一VOをcoversする他Testのidとintent）を含める。

### DES-869

バンドルは既知partition（対象VOのdimensions・coverage_policy・representative_cases）を含める。

### DES-870

バンドルは過去の判断（同一`(subject, judgment_kind)`への有効・無効な過去判断記録の要約）を含める。

### DES-871

バンドルは対象の内容ハッシュとリビジョン（Test subject / target subject / VO subjectの現在hash、revision）を含める。

### DES-872

バンドルは判断型（`judgment_kind`）をちょうど1件持ち、その値を`judgment_kind`として出力する。

### DES-873

判断型は判断対象を一意に区切るkeyであり（§3.4）、判断記録へ複製される。

### DES-874

本書が定義する判断型の値は`test-semantic` / `impl-consistency` / `case-coverage`の3種であり、これ以外の値でバンドルを生成しない。

### DES-875

`test-semantic`は、subjectの値域がTest IDであり、外部へ引き渡す問いは「テストコードは、covers先VOのclaimとTest Intentが宣言する振る舞いを実際に検証しているか」である。

### DES-876

`impl-consistency`は、subjectの値域がTest IDであり、外部へ引き渡す問いは「対象実装は宣言と一致しているか」である。

### DES-877

`case-coverage`は、subjectの値域をTest IDまたはVO IDとする。

### DES-878

`case-coverage`は、subjectがTest IDのとき、外部へ引き渡す問いは「当該Testが宣言したcases集合は、covers先VOの要求入力空間を十分に代表・網羅しているか」である。

*導出元: SPEC-376, SPEC-377, SPEC-378, SPEC-379, SPEC-380, SPEC-381, SPEC-382, SPEC-383, SPEC-384, SPEC-385, SPEC-386, SPEC-387, SPEC-388, SPEC-389, SPEC-390, SPEC-391, SPEC-392, SPEC-393, SPEC-394, SPEC-395, SPEC-396, SPEC-397, SPEC-398, SPEC-399, SPEC-400, SPEC-401, SPEC-402, SPEC-403, SPEC-404, SPEC-405, SPEC-406, SPEC-407, SPEC-408, SPEC-430, SPEC-431, SPEC-432, SPEC-433, SPEC-434*

*引用: 基本仕様 §14、§11*

### DES-879

`case-coverage`は、subjectがVO IDのとき、外部へ引き渡す問いは「当該VOをcoversするTest群のcases集合は、当該VOの要求入力空間を十分に代表・網羅しているか」である。

*導出元: SPEC-376, SPEC-377, SPEC-378, SPEC-379, SPEC-380, SPEC-381, SPEC-382, SPEC-383, SPEC-384, SPEC-385, SPEC-386, SPEC-387, SPEC-388, SPEC-389, SPEC-390, SPEC-391, SPEC-392, SPEC-393, SPEC-394, SPEC-395, SPEC-396, SPEC-397, SPEC-398, SPEC-399, SPEC-400, SPEC-401, SPEC-402, SPEC-403, SPEC-404, SPEC-405, SPEC-406, SPEC-407, SPEC-408, SPEC-430, SPEC-431, SPEC-432, SPEC-433, SPEC-434*

*引用: 基本仕様 §14、§11*

### DES-880

`judgment_kind`と`subject`の種別の組合せがこの表にない要求ではバンドルを生成しない（別紙A §12.2のusage error、終了コード2）。

*引用: 別紙A §12.2*

### DES-881

`case-coverage`は§11の判断対象であって§5の4検査ではない。

### DES-882

`case-coverage`の未判断・判断結果はいずれも4検査の値へ写像せず、§11.3の集約へ寄与しない。

### DES-883

外部判断が必要な事実は§11.7の判断待ち情報として提示する。

### DES-884

`impl-consistency`型の判断（対象実装が宣言と一致するかの意味判定）のように上流documentを要する対象では、対象VOから§3.5と同じ上流依存規則で導出するdocument subject完全集合とsource全文を加える。

### DES-885

`case-coverage`型のバンドルでは、covers先の全leaf / 中間VOの`dimensions`・`coverage_policy`・`representative_cases`と、Testのcases集合を必須項目として含める。

### DES-886

宣言targetのいずれか、または上流documentのいずれかを解決できない場合はバンドルを生成せず、候補のいずれも選択しない（§6.1）。

### DES-887

解決失敗の種別は対象不在（E-SCAN-004、document不在）を`MISMATCH`（診断`MISSING`）として当該対象の検証結果へ保持する。

### DES-888

解決失敗の種別は恒久SRC ID衝突による曖昧（E-SCAN-011）を`MISMATCH`として当該対象の検証結果へ保持する。

### DES-889

バンドルJSONスキーマは`bundle_id`・`generated_at`・`revision`・`subject`・`judgment_kind`・`test`・`vos`・`targets`・`related_tests`・`static_analysis`・`prior_decisions`のfieldからなる。

> 例: { "bundle_id": "01J8XVYY...", "generated_at": "2026-08-08T00:00:00Z", "revision": { "commit": "abc123...", "dirty": false }, "subject": "TEST-PARSER-044", "judgment_kind": "test-semantic", "test": {...}, "vos": [...], "targets": [...], "related_tests": [...], "static_analysis": {...}, "prior_decisions": [...] }

### DES-890

`vtest audit submit --file result.json`で提出する。

### DES-891

判断は少なくともactor / subject / decisionを含む。

*導出元: REQ-180, REQ-181, REQ-182, REQ-183, REQ-184, REQ-185, REQ-186, REQ-187, REQ-188, REQ-189, REQ-190, REQ-191, REQ-192, SPEC-387, SPEC-388, SPEC-389, SPEC-390, SPEC-391, SPEC-392, SPEC-393, SPEC-394, SPEC-395, SPEC-396, SPEC-397, SPEC-398, SPEC-399, SPEC-400, SPEC-401, SPEC-402, SPEC-403, SPEC-404, SPEC-405, SPEC-406, SPEC-407, SPEC-408*

*引用: 基本仕様 §11.3, 要件定義 §12*

### DES-892

理由・根拠は任意（optional）とする。

*導出元: REQ-180, REQ-181, REQ-182, REQ-183, REQ-184, REQ-185, REQ-186, REQ-187, REQ-188, REQ-189, REQ-190, REQ-191, REQ-192, SPEC-387, SPEC-388, SPEC-389, SPEC-390, SPEC-391, SPEC-392, SPEC-393, SPEC-394, SPEC-395, SPEC-396, SPEC-397, SPEC-398, SPEC-399, SPEC-400, SPEC-401, SPEC-402, SPEC-403, SPEC-404, SPEC-405, SPEC-406, SPEC-407, SPEC-408*

*引用: 基本仕様 §11.3, 要件定義 §12*

### DES-893

提出スキーマは`bundle_id`・`subject`・`judgment_kind`・`supersedes`・`decision`・`reason`・`exclusions`・`actor`のfieldからなる。

> 例: { "bundle_id": "01J8XVYY...", "subject": "TEST-PARSER-044", "judgment_kind": "test-semantic", "supersedes": [], "decision": "accepted", "reason": [...], "exclusions": [...], "actor": { "kind": "agent", "id": "judge-agent-01", "model": "claude-fable-5" } }

### DES-894

`decision`の値集合はツールが受理する判断値（`accepted` / `rejected` / `deferred`等）とし、その妥当性を§8.4で検証する。

### DES-895

旧モデルの`verdict → CheckValue`写像（`PASS`/`FAIL`/`COMPLETE`/`INCOMPLETE`を検証状態へ変換する経路）は撤去する。

### DES-896

判断記録は検証状態を変更しない（§8 冒頭）。

### DES-897

`judgment_kind`は必須であり、`bundle_id`が指すバンドルの`judgment_kind`と一致しなければならない（§8.4）。

### DES-898

`supersedes`は任意であり、省略時は空listとして記録する。

### DES-899

`supersedes`に列挙する各ULIDは、同一`subject`かつ同一`judgment_kind`の既存判断記録を指さなければならない（§8.4）。

### DES-900

`reason` / `exclusions`は任意である。

### DES-901

`basis.kind`は`document` / `vo` / `test-code` / `target-code`のいずれかとする。

### DES-902

理由が空であることだけを根拠に判断を無効化しない。

*導出元: SPEC-387, SPEC-388, SPEC-389, SPEC-390, SPEC-391, SPEC-392, SPEC-393, SPEC-394, SPEC-395, SPEC-396, SPEC-397, SPEC-398, SPEC-399, SPEC-400, SPEC-401, SPEC-402, SPEC-403, SPEC-404, SPEC-405, SPEC-406, SPEC-407, SPEC-408*

*引用: 基本仕様 §11.3*

### DES-903

`audit submit`は、検証に失敗した場合は§17のエラーコードで拒否する。

### DES-904

`audit submit`は、bundle_idのバンドルがcacheに存在することを検証する（E-AUDIT-001）。

### DES-905

`audit submit`は、subjectがバンドルと一致することを検証する（E-AUDIT-003）。

### DES-906

`audit submit`は、judgment_kindがバンドルと一致し、§8.1の値域内であることを検証する（E-AUDIT-003）。

### DES-907

`audit submit`は、バンドル記録時の各対象の内容ハッシュが、現在のハッシュと一致することを検証する（対象が変更されていれば判断は無効。E-AUDIT-002）。

### DES-908

`audit submit`は、decisionが受理する判断値であることを検証する（E-AUDIT-004）。

### DES-909

`audit submit`は、supersedesの各ULIDが、同一subjectかつ同一judgment_kindの既存判断記録を指し、自己参照でないことを検証する（E-AUDIT-008）。

### DES-910

受理された提出は判断記録（§3.4）として`.verify/decisions/`へ保存される。

### DES-911

`subjects`に相当する対象集合はバンドル生成時の全対象の内容ハッシュを`subject_hash`と`dependencies`として記録し、依存closureのハッシュに束縛する。

### DES-912

理由（`reason` / `exclusions`）の有無を提出の受理条件にしない。

### DES-913

旧モデルのreasons / claim / basis必須検査（E-AUDIT-005）、decomposition-viewpoint検査（E-AUDIT-006）、spec / req basis検査（E-AUDIT-007）は撤去する。

### DES-914

旧モデルのreasons / claim / basis必須検査（E-AUDIT-005）、decomposition-viewpoint検査（E-AUDIT-006）、spec / req basis検査（E-AUDIT-007）は要件定義§12「理由が空であることだけを根拠に無効扱いしない」と矛盾するため、判断記録層では課さない。

*導出元: REQ-180, REQ-181, REQ-182, REQ-183, REQ-184, REQ-185, REQ-186, REQ-187, REQ-188, REQ-189, REQ-190, REQ-191, REQ-192*

*引用: 要件定義 §12*

### DES-915

判断記録の有効性は判定時に評価する。

### DES-916

判断記録が有効であるとは、judgment_kindが§8.1の値域内であり、subjectが一致し、subject_hashが現在の内容ハッシュと一致し、dependenciesが現在の上流依存closureとentity・hashとも完全一致することをいう。

> document は登録 content_hash と実ファイルの一致も要求。不一致の場合は当該 document を STALE とし、依存する判断記録も無効。

### DES-917

対象は`(subject, judgment_kind)`の組であり、組ごとに独立に評価する。

### DES-918

判断値が食い違う有効判断記録が併存する場合、機械はどれも選ばない。

### DES-919

V(subject, judgment_kind)は、「有効」を満たす判断記録の集合とする。

### DES-920

実効集合Eは、Vから、V内の他レコードのsupersedesに名指しされたものを除いた集合とする。

> supersede できるのは有効判断記録だけである。無効な判断記録の supersedes は何も除かない。

### DES-921

実効判断は、Eが空のとき未確定（UNKNOWN）とする。

### DES-922

実効判断は、Eの全レコードのdecisionが同一値であるとき、そのdecisionを実効判断値とする。

### DES-923

実効判断は、Eに2種以上のdecision値がある（競合）とき未確定（UNKNOWN）とし、W-STORE-004を出す。

### DES-924

実効判断が「未確定（`UNKNOWN`）」であることは、当該対象が§11のエスカレーション状態にとどまることを意味する。

### DES-925

実効判断が未確定（`UNKNOWN`）であり当該対象が§11のエスカレーション状態にとどまることは、§4.1の検証状態を変更せず、`UNKNOWN`に§4.2の診断ラベルを付与しない（§8 冒頭）。

### DES-926

未確定である事実は§11.7の判断待ち情報として提示する。

### DES-927

競合は、新しい判断記録が旧判断記録を`supersedes`で明示に名指しして置き換えたときにだけ解消する。

### DES-928

判断記録の新旧（`decided_at` / ULID順）、`decision`値の優先順位（`rejected`優先等）、記録件数の多寡のいずれも解消規則に用いてはならない。

### DES-929

`supersedes`が循環する（レコード群が互いを名指ししてEが空になる）場合は未確定（`UNKNOWN`）とし、W-STORE-005を出す。

### DES-930

いずれかのレコードを推測で残さない。

### DES-931

`judgment_kind`を欠くか値域外の判断記録は、いずれの`(subject, judgment_kind)`のVにも属さず、実効判断へ寄与しない。

### DES-932

`judgment_kind`を欠くか値域外の判断記録は、履歴表示だけを許可し、W-STORE-003を出す。

### DES-933

判断記録を対象とする承認（§3.5の`judgment_ref`）は、当該判断記録が有効かつEに属する場合にだけ実効承認を導出する。

### DES-934

Eから外れた判断記録への承認は`draft`相当とする。

### DES-935

同一対象に有効な判断記録が複数あってよい（再判断・多重判断）。

### DES-936

回数はツールとして制限しない（運用ポリシー）。

### DES-937

仕様・VO・Test等が変更された場合、過去の判断を現在状態へそのまま流用してはならず、現在状態に対して通常の検証（§5の4検査）を再実施する。

*導出元: REQ-180, REQ-181, REQ-182, REQ-183, REQ-184, REQ-185, REQ-186, REQ-187, REQ-188, REQ-189, REQ-190, REQ-191, REQ-192, SPEC-387, SPEC-388, SPEC-389, SPEC-390, SPEC-391, SPEC-392, SPEC-393, SPEC-394, SPEC-395, SPEC-396, SPEC-397, SPEC-398, SPEC-399, SPEC-400, SPEC-401, SPEC-402, SPEC-403, SPEC-404, SPEC-405, SPEC-406, SPEC-407, SPEC-408*

*引用: 基本仕様 §11.3, 要件定義 §12*

### DES-938

現在状態に対して通常の検証（§5の4検査）を再実施した結果は`PASS` / `FAIL` / `MISMATCH` / `NO_EVIDENCE` / `UNKNOWN`のいずれにもなり得る。

### DES-939

変更そのものが`UNKNOWN`を生成するのではない。

### DES-940

判断済みと承認済みは区別する（判断済み≠承認済み）。

### DES-941

判断は承認なしでも記録でき、正式採用は§3.5の承認の別段階である。

### DES-942

判断エージェントのプロンプト・スキル構成はツールの責務外だが、参考として骨子を示す。

> あなたは検証対象の意味判定者である。添付のバンドルについて、以下だけを判定せよ。修正方針の提案はしない。判定事項：テストコードは、VOのclaimとTest Intentが宣言する振る舞いを実際に検証しているか。判定はaccepted / rejected / deferredのいずれかとし、判定ごとにclaim（何を確認したか）とbasis（根拠にしたバンドル内の情報への参照）を任意で列挙してよい。

### DES-943

判断の受理は検証状態を昇格させない。

### DES-944

判断は`UNKNOWN`に対する外部判断の追跡であり、検査ゲートではない（§8 冒頭、基本仕様 §11.3）。

*導出元: SPEC-387, SPEC-388, SPEC-389, SPEC-390, SPEC-391, SPEC-392, SPEC-393, SPEC-394, SPEC-395, SPEC-396, SPEC-397, SPEC-398, SPEC-399, SPEC-400, SPEC-401, SPEC-402, SPEC-403, SPEC-404, SPEC-405, SPEC-406, SPEC-407, SPEC-408*

*引用: 基本仕様 §11.3*

## DA-010 9. テスト実行設計

### DES-945

`vtest run`は`--test` / `--vo` / `--all`で対象を受け取り、検証グラフからTest集合へ展開する（VO指定は部分木のcoversを辿る）。

### DES-946

旧モデルの`--req`（REQ指定）はdocument層の総称化により廃止し、document scopeが必要な場合はVO部分木経由で指定する。

### DES-947

`rust-cargo` adapterは`TestEntity.execution`の`project`をcargo package名として解釈する。

> `rust-cargo` adapterはTestEntity.executionを次のCargo実行座標として解釈する。

### DES-948

`rust-cargo` adapterは`TestEntity.execution`の`suite.kind`を`lib` / `bin` / `integration`として解釈する。

### DES-949

`rust-cargo` adapterは`TestEntity.execution`の`suite.name`をbin名またはintegration test target名として解釈し、`lib`では省略する。

### DES-950

`rust-cargo` adapterは`TestEntity.execution`の`selector`をtest targetのrootからのmodule path＋function名（例：`parser::tests::rejects_invalid_utf8`）として解釈する。

### DES-951

adapter内部ではこれらからCargo launch coordinateを構築する。

### DES-952

`TestEntity`へCargo固有fieldを戻してはならない。

### DES-953

実行は（project, suite）で分けたbatchとし、libtestの`--exact` flagと複数selectorを用いる。

> cargo test -p <project> --lib -- --exact <selector1> <selector2> ...（IntegrationTest の場合は --lib の代わりに --test <name>）

### DES-954

実行対象の解釈とcommand生成は`TestRunnerAdapter`が所有する。

### DES-955

orchestrationは`ExecutionDescriptor.adapter`をregistryで解決し、adapter不一致（E-ADAPTER-003）を拒否する。

### DES-956

明示的なrunでrunner未提供ならE-ADAPTER-004としてEvidenceを生成せず、検証集約の`target_binding`は`NO_EVIDENCE`（診断`NOT_EXECUTED`）とする。

### DES-957

`--exact`は後続の全フィルタへ適用されるフラグであり、各フィルタは完全一致で解釈される。

### DES-958

stdoutのパースはstable toolchainの標準出力形式のみに依存する。

### DES-959

`running N tests`という出力は実行対象数の確認を意味する。

### DES-960

`test <selector> ... ok`という出力はPASSを意味する。

### DES-961

`test <selector> ... FAILED`という出力はFAILを意味する。

### DES-962

`test <selector> ... ignored`という出力は実行されずを意味する。

### DES-963

`test <selector> ... ignored`が実行されずの場合、Evidenceは記録しない。

### DES-964

`test <selector> ... ignored`が実行されずの場合、target_bindingは診断NOT_EXECUTEDとする。

### DES-965

要求した各フィルタについて結果行が得られなかった場合、そのTestの実行は失敗（E-EXEC-002）とし、Evidenceを記録しない。

### DES-966

プロセス終了コードと結果行の集計が矛盾する場合もE-EXEC-003とする。

### DES-967

stdout / stderrの全文は`cache/logs/<ULID>.log`へ保存し、Evidenceの`log_ref`から参照する。

### DES-968

Testごとに§3.6のレコードを1件生成する。

### DES-969

`revision`は実行直前に`git rev-parse HEAD`と`git status --porcelain`で取得する。

### DES-970

`revision`の取得失敗時は`commit: null`とし、このEvidenceは鮮度（§11.2）のrevision一致を満たさず`target_binding`の有効な`PASS`にならない。

### DES-971

`hashes`は、実行直前のdiscovery結果から、Test subject hashと、全宣言targetを§6.1で解決したcanonical Locator・implementation construct hash（§1.3）を宣言順で記録する。

### DES-972

`hashes`は欠落・重複を許可しない。

### DES-973

`hashes`は宣言された`TargetRef`の綴りではなく解決後のcanonical Locatorを記録する（§6.1.1）。

### DES-974

全宣言targetがcanonical Source Targetへ一意に解決できることをEvidence生成の前提とする。

### DES-975

1件でも「対象なし」または「曖昧」（E-SCAN-004 / E-SCAN-011）ならEvidenceを生成しない。

### DES-976

部分的な`hashes.targets`を持つEvidenceを生成して後段で弾く方式は採らない。

### DES-977

全宣言targetのうち1件でも「対象なし」または「曖昧」（E-SCAN-004 / E-SCAN-011）の場合、`target_binding`は`NO_EVIDENCE`（診断`NOT_EXECUTED`）のままとし、target解決の診断で非`PASS`を示す。

### DES-978

`execution_state`は、実行直前にrunner adapterが返すsnapshot schema、runner / toolchain / 実行影響config、およびrepository / local dependency入力manifestをcoreが検証し、§1.3のExecution State subject hashとして記録する。

### DES-979

完全性を保証できない場合は`complete: false`とし、後続の鮮度を`PASS`にしない。

### DES-980

ビルド失敗（コンパイルエラー）の場合、対象Test群のEvidenceは記録せずE-EXEC-001を報告する。

### DES-981

`target_binding`は`NO_EVIDENCE`（診断`NOT_EXECUTED`）のままとなる。

## DA-011 10. `rust-cargo` Target Binding 動的計測

### DES-982

`rust-cargo` CoverageAdapterは`cargo-llvm-cov`を使用する（adapter configの`run.coverage: llvm-cov`）。

### DES-983

起動時に`cargo llvm-cov --version`で利用可否を確認し、利用不能なら計測しない。

### DES-984

利用不能な場合、Evidenceの`target_coverage`を`checked: false`（検証時`NO_EVIDENCE`、診断`NOT_CHECKED`）とし診断W-EXEC-101を出す（`PASS`へ変換しない）。

*導出元: SPEC-242, SPEC-243, SPEC-244, SPEC-245, SPEC-246, SPEC-247, SPEC-248, SPEC-249, SPEC-250, SPEC-251, SPEC-252, SPEC-253, SPEC-254, SPEC-255, SPEC-256, SPEC-257, SPEC-258, SPEC-259, SPEC-260, SPEC-261, SPEC-262*

*引用: 基本仕様 §5.3*

### DES-985

カバレッジをTest単位で対象関数へ帰属させるため、計測時はTestを1件ずつ実行する。

### DES-986

Testが起動したsubprocess・spawnしたthreadの実行を宣言targetへ帰属させられるかは`rust-cargo` CoverageAdapterの能力に属する（§10.2・§7.3）。

### DES-987

subprocess内の実行を計測するには起動される実行体もinstrument対象とし、子プロセスのprofileをmergeする必要がある。

### DES-988

起動される実行体をinstrument対象とし子プロセスのprofileをmergeすることを提供できない構成では境界越しtargetをUNKNOWNとする。

### DES-989

計測不能なら`target_coverage.checked: false`（`NO_EVIDENCE`/`NOT_CHECKED`）とし、能力の有無で計測結果を捏造しない。

### DES-990

Testが起動したsubprocess・spawnしたthreadの実行を宣言targetへ帰属させられる能力の実装可否は§7.3のruntime到達証明がsubprocess E2Eに及ぶかを左右するが、欠如時もfail-closedを保つ（DA-002はUNKNOWNのまま）。

### DES-991

計測コマンドは`cargo llvm-cov test -p <project> --lib --json --output-path cache/cov/<ULID>.json -- --exact <selector>`である。

### DES-992

coverageは独立した`CoverageAdapter` capabilityとして扱う。

### DES-993

提供されない場合は`target_coverage.checked: false`（`NO_EVIDENCE`/`NOT_CHECKED`）とする。

### DES-994

解析限界は`UNKNOWN`とし、測定済み`PASS`を推測しない。

### DES-995

出力JSON（llvm-cov export形式）の`data[].functions[]`から、Testが宣言する各対象関数を検索する。

### DES-996

一致条件は、demangle済み関数名の末尾がlocatorのitem-pathと一致し、かつfilenamesのいずれかの末尾がlocatorのpathと一致することである。

### DES-997

ジェネリック関数は複数インスタンスが現れるため、同じtargetに対応するcountを合算する。

### DES-998

target別判定は、count > 0なら`PASS`とする。

### DES-999

target別判定は、count == 0なら`FAIL`（診断NOT_EXECUTED）とする。

*導出元: SPEC-191, SPEC-192, SPEC-193, SPEC-194, SPEC-195, SPEC-196, SPEC-197, SPEC-198, SPEC-199*

*引用: 基本仕様 §4.3*

### DES-1000

target別判定は、関数が見つからなければ`UNKNOWN`（インライン化・cfg除外等の可能性）とする。

### DES-1001

Test単位集約は、FAILが1件以上あれば`FAIL`とする。

### DES-1002

Test単位集約は、FAILなし、UNKNOWNが1件以上あれば`UNKNOWN`とする。

### DES-1003

Test単位集約は、1件以上の全宣言targetがPASSなら`PASS`とする。

### DES-1004

各targetのcanonical Locator（§6.1.1）・result・countとTest単位集約結果をEvidenceの`target_coverage`へ記録する。

### DES-1005

target別entryの欠落、重複、余分なentry、または解決後のcanonical Source Target集合との不一致を`PASS`として保存しない。

### DES-1006

Evidenceの`target_coverage`へ記録する計測結果は§7.3のtarget_binding runtime証明の証拠源であり、独立の検査項目ではない。

### DES-1007

Testが別プロセス（起動したsubprocess内）・別スレッド等の実行境界越しにtargetを到達させる場合も、判定は実行countに基づく。

### DES-1008

coverage providerは当該境界越しの実行を宣言targetへ帰属させなければならない（例：起動される実行体も計測対象としてinstrumentし、子プロセスのprofileをmergeする）。

### DES-1009

providerが境界越しの実行を帰属できない場合はそのtargetを`UNKNOWN`（関数不見当扱い）とする。

### DES-1010

計測自体が不能なら`target_coverage.checked: false`とする。

### DES-1011

providerが境界越しの実行を帰属できない場合、または計測自体が不能な場合は、いずれも§7.3のruntime到達証明を成立させず、静的到達のUNKNOWNを`PASS`へ変換しない。

### DES-1012

coverage providerが境界越しの実行を宣言targetへ帰属させられるかはadapterのcoverage capabilityに属し、能力の有無で計測結果を捏造しない。

### DES-1013

`vtest run`は2モードを持つ。

### DES-1014

`--fast`モードはcargo testのみとし、`target_coverage.checked: false`で記録し、検証時は`NO_EVIDENCE`（診断`NOT_CHECKED`）とする。

### DES-1015

既定モード（完全検証向け）はcargo-llvm-covによるTest単位実行とする。

> 実行時間と引き換えに `target_binding` の動的証拠を得る。

## DA-012 11. 鮮度検証と集約

### DES-1016

`chain_integrity`は評価地点をrepository scan result / DOC / VO / TESTとし、文書鎖（document derives_from・content_hash）、VOのderives_from（document 1件以上）、Testの管理宣言（Test ID・covers ≥ 1・その他の必須metadata〔intent、および当該adapterが必須とする追加metadata。rust-cargoではtargets ≥ 1〕）・covers参照解決・Test ID大局的一意性がすべて成立すれば`PASS`とする（§11.1.1）。

> 4検査（基本仕様§5）は、次の地点で評価する。表: | 検査 | 評価地点 | 評価方法 |

*導出元: SPEC-217, SPEC-218, SPEC-219, SPEC-220, SPEC-221, SPEC-222, SPEC-223, SPEC-224, SPEC-225, SPEC-226, SPEC-227, SPEC-228, SPEC-229, SPEC-230, SPEC-231, SPEC-232, SPEC-233, SPEC-234, SPEC-235, SPEC-236, SPEC-237, SPEC-238, SPEC-239, SPEC-240, SPEC-241, SPEC-242, SPEC-243, SPEC-244, SPEC-245, SPEC-246, SPEC-247, SPEC-248, SPEC-249, SPEC-250, SPEC-251, SPEC-252, SPEC-253, SPEC-254, SPEC-255, SPEC-256, SPEC-257, SPEC-258, SPEC-259, SPEC-260, SPEC-261, SPEC-262, SPEC-263, SPEC-264, SPEC-265, SPEC-266, SPEC-267, SPEC-268, SPEC-269, SPEC-270, SPEC-271, SPEC-272, SPEC-273, SPEC-274, SPEC-275, SPEC-276, SPEC-277, SPEC-278, SPEC-279, SPEC-280, SPEC-281, SPEC-282, SPEC-283, SPEC-284, SPEC-285, SPEC-286*

*引用: 基本仕様 §5*

### DES-1017

`orphan_detection`は評価地点をDOCとし、親を持たず`doc.roots`にも列挙されないdocumentが無ければ`PASS`、あれば`MISMATCH`とする（§5.6）。

### DES-1018

`target_binding`は評価地点をTESTとし、§7.3の合成による。

### DES-1019

`target_binding`は、Evidence result FAILは`FAIL`、全宣言targetの到達が静的到達またはruntime到達で充足されれば`PASS`とする。

### DES-1020

`target_binding`の未充足は§11.2の写像に従う。

### DES-1021

`oracle_presence`は評価地点をTESTとし、§7.1の合成（DA-001 / DA-003 / DA-004 / DA-005 / DA-006）による。

### DES-1022

`oracle_presence`は、全PASSで`PASS`、1つでもFAILで`FAIL`、FAILなくUNKNOWNで`UNKNOWN`とする。

### DES-1023

4検査の評価入力は、当該revisionのrepositoryを走査したscan結果（adapterが返すdiscovery出力と、そこからcoreが具体化したエンティティ・内容ハッシュ）、`.verify/`配下の正典ファイル集合（`config.yaml`、documentレコード、VOレコード、Relationレコード、判断記録〔`.verify/decisions/`〕、承認レコード〔`.verify/approvals/`〕、Evidenceレコード〔`.verify/evidence/`〕）、Evidence鮮度判定（§11.2）が現在のsnapshotとして再構築するExecution State subjectの入力（toolchain identity、実行結果へ影響するadapter configのcanonical projection、repository / local dependencyの入力manifest。§1.3）、および当該実行の要求scope指定（検査軸・エンティティ軸・`--gate`）に限る。

*導出元: REQ-224, REQ-225, REQ-226, REQ-227, SPEC-379, SPEC-380, SPEC-381, SPEC-382, SPEC-383*

*引用: 基本仕様 §11.1, 要件定義 §17.2*

### DES-1024

4検査の評価入力は、当該revisionのrepositoryを走査したscan結果（adapterが返すdiscovery出力と、そこからcoreが具体化したエンティティ・内容ハッシュ）を含む。

### DES-1025

4検査の評価入力は、`.verify/`配下の正典ファイル集合（`config.yaml`、documentレコード、VOレコード、Relationレコード、判断記録〔`.verify/decisions/`〕、承認レコード〔`.verify/approvals/`〕、Evidenceレコード〔`.verify/evidence/`〕）を含む。

### DES-1026

4検査の評価入力は、Evidence鮮度判定（§11.2）が現在のsnapshotとして再構築するExecution State subjectの入力（toolchain identity、実行結果へ影響するadapter configのcanonical projection、repository / local dependencyの入力manifest。§1.3）を含む。

### DES-1027

4検査の評価入力は、当該実行の要求scope指定（検査軸・エンティティ軸・`--gate`）を含む。

### DES-1028

4検査の評価入力の集合が同一であれば、4検査の検証状態（5状態）・診断ラベル・診断コード集合・集約結果・`pending` sectionの内容・終了コードは同一でなければならない。

> 次を、それ自体として検査入力にしてはならない。

### DES-1029

実行時の現在時刻・経過時間・乱数・プロセスIDを、それ自体として検査入力にしてはならない。

### DES-1030

ロケール・タイムゾーン・環境変数・呼出し元の作業ディレクトリ（`--project`で解決したプロジェクトルート自体は入力に含む）を、それ自体として検査入力にしてはならない。

### DES-1031

ネットワーク応答、およびLLM APIを含む外部サービスの応答を、それ自体として検査入力にしてはならない。

### DES-1032

環境の変化が結果へ影響しうるのは、Execution State subjectの入力（toolchain identity・adapter config・入力manifest）を変える範囲に限る。

### DES-1033

Execution State subjectの入力（toolchain identity・adapter config・入力manifest）を変える範囲で環境の変化が結果へ影響する場合の影響はEvidenceの鮮度喪失（`NO_EVIDENCE`、診断`STALE`。§11.2）として現れ、環境そのものを判定条件として読むわけではない。

### DES-1034

ネットワーク応答と外部サービス応答はExecution State subjectの入力に含まれないため、例外なく検査入力にならない。

### DES-1035

本システムは意味判定・候補生成を外部の判定器へ委ねるseam（実行時に差し替え可能な意味判定・意味生成の呼出し点）を4検査の評価経路に持たない。

### DES-1036

外部AI／Agentは判断記録（§8）の著者として`.verify/decisions/`へ記録を残す経路でのみ関与し、その記録は入力集合の一部としてファイル経由で読まれる。

### DES-1037

判断記録の受理は検証状態を昇格させない（§8.3）。

### DES-1038

将来そのようなseamを評価経路へ設ける場合は、任意の判定を返す実装（正反対の判定を返す実装を含む）を差し替えても4検査の結果が変化しないことを満たさなければならない。

### DES-1039

完全検証の検査集合はこの4検査に固定し、設定で追加・削除できない（§2.2、基本仕様 §22.1）。

*導出元: SPEC-570, SPEC-571, SPEC-572, SPEC-573, SPEC-574, SPEC-575, SPEC-576*

*引用: 基本仕様 §22.1*

### DES-1040

旧モデルの12項目（`spec_coverage` / `vo_decomposition` / `vo_coverage` / `test_existence` / `static_audit` / `semantic_audit` / `impl_consistency` / `test_execution` / `runtime_result` / `target_execution` / `evidence_validity` / `test_traceability`）は検査として存在しない。

### DES-1041

`test_existence` / `test_traceability`は`chain_integrity`へ統合した。

### DES-1042

`static_audit`は`oracle_presence`（DA-001/003/004/005/006）と`target_binding`の静的到達（DA-002）へ分割した。

### DES-1043

`test_execution` / `target_execution` / `runtime_result`は`target_binding`の証拠（Evidenceの存在・鮮度、`result`、`target_coverage`）へ吸収した。

### DES-1044

`evidence_validity`は独立検査を廃し、鮮度喪失を診断ラベル`STALE`として§11.2で説明した（基本仕様 §6）。

*導出元: SPEC-287, SPEC-288, SPEC-289, SPEC-290, SPEC-291, SPEC-292, SPEC-293, SPEC-294, SPEC-295, SPEC-296, SPEC-297, SPEC-298, SPEC-299*

*引用: 基本仕様 §6*

### DES-1045

`spec_coverage` / `vo_coverage` / `vo_decomposition` / `semantic_audit` / `impl_consistency`は検査から除去し、網羅・意味の疑義は`UNKNOWN`として判断記録エスカレーションとした（§8、基本仕様 §11、要件定義 §12）。

*導出元: REQ-180, REQ-181, REQ-182, REQ-183, REQ-184, REQ-185, REQ-186, REQ-187, REQ-188, REQ-189, REQ-190, REQ-191, REQ-192, SPEC-376, SPEC-377, SPEC-378, SPEC-379, SPEC-380, SPEC-381, SPEC-382, SPEC-383, SPEC-384, SPEC-385, SPEC-386, SPEC-387, SPEC-388, SPEC-389, SPEC-390, SPEC-391, SPEC-392, SPEC-393, SPEC-394, SPEC-395, SPEC-396, SPEC-397, SPEC-398, SPEC-399, SPEC-400, SPEC-401, SPEC-402, SPEC-403, SPEC-404, SPEC-405, SPEC-406, SPEC-407, SPEC-408*

*引用: 基本仕様 §11, 要件定義 §12*

### DES-1046

`chain_integrity`は宣言鎖のすべてのリンクが存在し、ハッシュ照合が成立するかを問う。

*導出元: SPEC-224, SPEC-225, SPEC-226, SPEC-227, SPEC-228, SPEC-229, SPEC-230, SPEC-231, SPEC-232, SPEC-233, SPEC-234*

*引用: 基本仕様 §5.1*

### DES-1047

いずれか違反があれば`MISMATCH`（切れた箇所を診断ラベルで示す）とする。

### DES-1048

文書層は、各`document`の`derives_from`参照先が存在すること（E-SCAN-012）を評価する。

### DES-1049

文書層は、`content_hash`が現物と一致すること（不一致は診断`STALE`。§11.4）を評価する。

### DES-1050

VO層は、各VOが1件以上の`document`への解決可能な`derives_from`を持つこと（不在・解決不能はE-SCAN-012）を評価する。

### DES-1051

VO層は、VO parentの不在・循環をE-SCAN-008とする。

### DES-1052

VO層は、`combinations`が§3.2.1の受理条件を満たすこと（違反はE-SCAN-017）を評価する。

### DES-1053

Test層は、発見された各Testに対応する管理宣言（構文上有効なTest ID・1件以上の`covers`・`intent`その他の必須metadata。`targets ≥ 1`はadapter中立coreの必須リンクに含めず、当該adapterが必須とする追加metadataとして扱う〔`rust-cargo`では1件以上の`targets`〕。§4.1・基本仕様 §5.1・§9.1）がちょうど1件存在すること（欠落はE-SCAN-007、診断`MISSING`）を評価する。

*導出元: SPEC-224, SPEC-225, SPEC-226, SPEC-227, SPEC-228, SPEC-229, SPEC-230, SPEC-231, SPEC-232, SPEC-233, SPEC-234, SPEC-332, SPEC-333, SPEC-334, SPEC-335, SPEC-336, SPEC-337, SPEC-338, SPEC-339*

*引用: 基本仕様 §5.1・§9.1, 基本仕様 §5.1, 基本仕様 §9.1*

### DES-1054

Test層は、`covers`の全VO参照を解決できること（E-SCAN-003）を評価する。

### DES-1055

Test層は、Test IDが発見結果全体で一意であること（衝突はE-SCAN-002）を評価する。

### DES-1056

双方向完全性は、leaf VO → Test（検証実装の存在。coversするTestが1件以上）と、発見されたTest → 宣言（管理宣言の解決）の両方向が成立して初めて成立する。

### DES-1057

coversするTestの無いleaf VOは`MISMATCH`（診断`MISSING`）とする。

### DES-1058

Relationのfrom / to不在はE-SCAN-009とする。

### DES-1059

恒久SRC IDのadapter越え衝突はE-SCAN-011とする。

### DES-1060

すべてのTestを管理対象とすることと、当該Testを証拠として算入すること（§7 / §10のtarget_binding / oracle_presence）は別個の条件とする。

*導出元: SPEC-309, SPEC-310, SPEC-311, SPEC-312*

*引用: 基本仕様 §8.1*

### DES-1061

旧モデルの`role`に基づく`covers`可変制約・適用項目集合は設けず、すべての管理対象Testに`covers ≥ 1`を一律要求する。

*導出元: SPEC-409, SPEC-410, SPEC-411, SPEC-412, SPEC-413, SPEC-414, SPEC-415, SPEC-416, SPEC-417, SPEC-418, SPEC-419, SPEC-420, SPEC-421, SPEC-422, SPEC-423, SPEC-424, SPEC-425*

*引用: 基本仕様 §12*

### DES-1062

`covers`を持たない（0件の）Testは管理宣言不整合として`chain_integrity = MISMATCH`であり、特別扱いの分岐を設けない。

### DES-1063

対象Testの Evidenceのうち最新のものについて、evidence.hashes.test_subject == 現在のTest subject hashを検査する。

### DES-1064

対象TestのEvidenceのうち最新のものについて、evidence.hashes.targetsの参照集合が、現在のTest.targetsを§6.1で解決したcanonical Locator集合と重複なく一致し、各target_constructが現在のimplementation construct hashと一致することを検査する。

### DES-1065

対象TestのEvidenceのうち最新のものについて、evidence.revision.commitが非nullかつ現在のHEAD revisionと一致することを検査する。

### DES-1066

対象TestのEvidenceのうち最新のものについて、evidence.execution_state.complete == trueかつ、同じschemaで現在再構築したExecution State subjectがcompleteで、hashが一致することを検査する。

### DES-1067

対象TestのEvidenceのうち最新のものについて、evidence.adapterが現在のTest.execution.adapterと一致することを検査する。

> adapter欠落形は§3.6の互換条件で一意に確認できる。

### DES-1068

evidence.hashes.test_subject == 現在のTest subject hashであること、evidence.hashes.targetsの参照集合が現在のTest.targetsを§6.1で解決したcanonical Locator集合と重複なく一致し各target_constructが現在のimplementation construct hashと一致すること、evidence.revision.commitが非nullかつ現在のHEAD revisionと一致すること、evidence.execution_state.complete == trueかつ同じschemaで現在再構築したExecution State subjectがcompleteでhashが一致すること、およびevidence.adapterが現在のTest.execution.adapterと一致することのすべてが成立する場合、当該Evidenceは現在の証拠として有効とする（dirty: trueでもExecution State subject一致なら有効。実行入力manifestが実体を保証する）。

### DES-1069

evidence.hashes.test_subjectが現在のTest subject hashと一致しない場合、または、evidence.hashes.targetsの参照集合が現在のTest.targetsを§6.1で解決したcanonical Locator集合と重複なく一致しない場合（各target_constructが現在のimplementation construct hashと一致しない場合を含む）、`NO_EVIDENCE`（診断STALE）とする。

### DES-1070

evidence.revision.commitが非nullでない、または現在のHEAD revisionと一致しない場合、`NO_EVIDENCE`（診断STALE。現在revisionに対する実行ではない）とする。

### DES-1071

evidence.execution_stateのrecordが欠落している場合、またはhashが一致しない場合、`NO_EVIDENCE`（診断STALE）とする。

### DES-1072

evidence.execution_state.completeがtrueでない、または現在再構築したExecution State subjectを完全に構築不能の場合、`UNKNOWN`とする。

### DES-1073

Evidenceのadapterが現在のTestのexecution.adapterと明示的に不一致の場合、`MISMATCH`とする。

### DES-1074

Evidenceのadapterが現在のTestのexecution.adapterと一致するかを確認不能の場合、`UNKNOWN`とする。

### DES-1075

Evidenceなしの場合、`NO_EVIDENCE`（診断NOT_EXECUTED）とする。

### DES-1076

Evidenceは全宣言targetが一意に解決できる場合だけ生成される（§9.4）。

### DES-1077

現在の宣言targetのうち1件でもcanonical Source Targetへ一意に解決できなくなった場合、記録済み参照集合は現在のcanonical集合と一致しないため、evidence.hashes.targetsの参照集合が現在のTest.targetsを§6.1で解決したcanonical Locator集合と重複なく一致するという条件は成立せず、`target_binding`を有効な`PASS`にしない。

### DES-1078

対象が存在せずE-SCAN-004となるtargetは`MISMATCH`（診断`MISSING`）として保持する（§5.4）。

### DES-1079

複数候補により曖昧でE-SCAN-011となるtargetは`MISMATCH`として保持する（§5.4）。

### DES-1080

有効なEvidenceが得られたとき、`result: FAIL`（テストランナーが失敗を報告）なら`target_binding`は`FAIL`とする。

*導出元: REQ-094, REQ-095, REQ-096, REQ-097, REQ-098, REQ-099, REQ-100, REQ-101, REQ-102, REQ-103, REQ-104*

*引用: 要件定義 §5.3*

### DES-1081

有効なEvidenceが得られたとき、`result: PASS`かつ全宣言targetの到達要件が§7.3で充足（静的到達またはruntime到達）されれば`target_binding`は`PASS`とする。

### DES-1082

有効なEvidenceが得られたとき、`result: PASS`だが到達未充足のtargetがある場合、当該targetの`target_coverage`に従い、count 0は`FAIL`（診断NOT_EXECUTED）、計測不能・未計測（`checked: false`）は`NO_EVIDENCE`（診断NOT_CHECKED）、関数不見当は`UNKNOWN`とする。

### DES-1083

Evidenceが存在するが有効でない場合、`target_binding`はEvidenceを再利用せず、上表の`MISMATCH` / `NO_EVIDENCE`（STALE）/ `UNKNOWN`を保持する。

### DES-1084

Evidenceが無ければ`NO_EVIDENCE`（診断`NOT_EXECUTED`）とする。

### DES-1085

複数条件が非`PASS`なら根拠をすべて保持し、表示代表値は基本仕様 §22.2の優先順位で選ぶ（診断ラベルは順位に用いず併記する）。

*導出元: SPEC-577, SPEC-578, SPEC-579, SPEC-580, SPEC-581, SPEC-582*

*引用: 基本仕様 §22.2*

### DES-1086

項目scopeが省略された場合、aggregatorはconfig値から部分集合を組み立てず、基本仕様 §5の固定4検査を選択する。

*導出元: SPEC-217, SPEC-218, SPEC-219, SPEC-220, SPEC-221, SPEC-222, SPEC-223, SPEC-224, SPEC-225, SPEC-226, SPEC-227, SPEC-228, SPEC-229, SPEC-230, SPEC-231, SPEC-232, SPEC-233, SPEC-234, SPEC-235, SPEC-236, SPEC-237, SPEC-238, SPEC-239, SPEC-240, SPEC-241, SPEC-242, SPEC-243, SPEC-244, SPEC-245, SPEC-246, SPEC-247, SPEC-248, SPEC-249, SPEC-250, SPEC-251, SPEC-252, SPEC-253, SPEC-254, SPEC-255, SPEC-256, SPEC-257, SPEC-258, SPEC-259, SPEC-260, SPEC-261, SPEC-262, SPEC-263, SPEC-264, SPEC-265, SPEC-266, SPEC-267, SPEC-268, SPEC-269, SPEC-270, SPEC-271, SPEC-272, SPEC-273, SPEC-274, SPEC-275, SPEC-276, SPEC-277, SPEC-278, SPEC-279, SPEC-280, SPEC-281, SPEC-282, SPEC-283, SPEC-284, SPEC-285, SPEC-286*

*引用: 基本仕様 §5*

### DES-1087

`verify.full_scope`はconfig読込み時に§2.2のinvariantとして検証・正規化済みでなければならない。

### DES-1088

明示的な部分集合だけを限定scopeとし、その結果を完全検証として表示しない。

### DES-1089

aggregateは、scanによりグラフ構築する（§5）。

### DES-1090

aggregateは、chain_integrity / orphan_detectionをrepository / DOC / VO / TEST構造に対して評価する。

### DES-1091

aggregateは、scopeのエンティティ軸でDOC/VO/TEST部分木を選択する。

### DES-1092

aggregateは、各TESTについて、scopeの検査軸に含まれるtarget_binding / oracle_presenceを評価する（含まれない検査はNO_EVIDENCE、診断NOT_CHECKED）。

### DES-1093

aggregateは、各leaf VOについてcoversするTEST群の結果をfail-closedで合成する。

### DES-1094

aggregateは、子VOを持つVO（親VO）について、子VOの値と、当該親VOを直接coversするTESTの値を合わせてfail-closedで合成する（直接coversするTESTが無ければ子VOの値だけを合成する）。

### DES-1095

aggregateは、DOCについて下流VO部分木の合成（fail-closed）を行う。

### DES-1096

総合判定は、構造検査（chain_integrity / orphan_detection）とentity treeのscope内評価がすべてPASSならOK、それ以外ならNGとする。

### DES-1097

fail-closed合成は、子にFAIL/MISMATCH/NO_EVIDENCE/UNKNOWNが1つでもあれば親を非PASSとする。

### DES-1098

fail-closed合成の代表値は基本仕様 §22.2の優先順位FAIL > MISMATCH > NO_EVIDENCE > UNKNOWNで選ぶ。

*導出元: SPEC-577, SPEC-578, SPEC-579, SPEC-580, SPEC-581, SPEC-582*

*引用: 基本仕様 §22.2*

### DES-1099

診断ラベル（MISSING / NOT_EXECUTED / NOT_CHECKED / STALE）は順位に用いず併記する。

### DES-1100

利用者向け簡易出力は`OK` / `NG`の二値とする。

*導出元: SPEC-570, SPEC-571, SPEC-572, SPEC-573, SPEC-574, SPEC-575, SPEC-576*

*引用: 基本仕様 §22.1*

### DES-1101

詳細出力は任意ノードからの局所／経路／全体トレース（§11.6）に沿ったツリー表示とし、非`PASS`の根拠（判断記録・Evidenceへの参照）を辿れる。

### DES-1102

人間向けテキストと機械可読JSONの両方を出力できる。

*導出元: SPEC-583, SPEC-584, SPEC-585, SPEC-586, SPEC-587, SPEC-588, SPEC-589, SPEC-590, SPEC-591, SPEC-592, SPEC-593, SPEC-594, SPEC-595*

*引用: 基本仕様 §22.3*

### DES-1103

検査の表示scopeと、検査導出に必要な内部依存の評価は分離する。

### DES-1104

§7.3により`target_binding`は当該TestのEvidence鮮度（§11.2）とtarget別`target_coverage`へ依存する。

### DES-1105

`target_binding`が項目scopeに含まれる場合、aggregatorは§7.3のruntime到達証明の判定に必要な範囲でこれらを内部依存として評価する。

### DES-1106

runtime証明に依存する`target_binding`の値は、根拠として用いたEvidence IDと当該targetの`target_coverage`結果をreportで引用し、原因を辿れる状態にする。

### DES-1107

`covers`を持つTestはcovers先それぞれのVOの合成に独立に参加する。

### DES-1108

「1つのTestが複数VOを検証していること」自体は許容し、各leaf VOの充足と組合せは§3.2.1の実体化されたleaf VO単位で判定する。

*導出元: SPEC-358, SPEC-359, SPEC-360, SPEC-361, SPEC-362, SPEC-363, SPEC-364, SPEC-365, SPEC-366, SPEC-367, SPEC-368, SPEC-369, SPEC-370, SPEC-371, SPEC-372, SPEC-373, SPEC-374, SPEC-375, SPEC-577, SPEC-578, SPEC-579, SPEC-580, SPEC-581, SPEC-582*

*引用: 基本仕様 §10、§22.2*

### DES-1109

基本仕様 §22.2がTest単位の結果の集約先として挙げる「Feature単位」は、親VO（`parent`により1件以上の子VOを持つVO。§3.2）を単位として実現する。

*導出元: SPEC-577, SPEC-578, SPEC-579, SPEC-580, SPEC-581, SPEC-582*

*引用: 基本仕様 §22.2*

### DES-1110

Featureを独立のエンティティ種別・レコードファイル・ID体系・宣言fieldとして設けず、`.verify/`にFeature用ディレクトリを置かない（基本仕様 §3.1のエンティティ種別を増やさない）。

*導出元: SPEC-121, SPEC-122, SPEC-123, SPEC-124, SPEC-125, SPEC-126, SPEC-127, SPEC-128, SPEC-129, SPEC-130, SPEC-131, SPEC-132, SPEC-133, SPEC-134, SPEC-135, SPEC-136, SPEC-137, SPEC-138, SPEC-139, SPEC-140, SPEC-141, SPEC-142, SPEC-143*

*引用: 基本仕様 §3.1*

### DES-1111

親VOの値は、子VOの値と当該親VOを直接coversするTESTの値を合わせたfail-closed合成そのものであり、機能単位の表示のために別の合成規則・緩和規則を設けない。

### DES-1112

子に1つでも非`PASS`があれば親VOは非`PASS`であり、代表値の優先順位も基本仕様 §22.2と同一とする。

*導出元: SPEC-577, SPEC-578, SPEC-579, SPEC-580, SPEC-581, SPEC-582*

*引用: 基本仕様 §22.2*

### DES-1113

Testの結果が親VOへ寄与する経路は、(a) coversするleaf VO経由の伝播と、(b) 当該親VOを直接coversするTestの直接参加の2つに限る。

### DES-1114

covers宣言を経由しない「機能名による束ね」（ファイルパス・モジュール名・命名規約からの推定束ね）を設けない。

### DES-1115

親VOを持たないleaf VOは、それ自体が最上位の束ね単位となる。

### DES-1116

DOC単位の集約は下流VO部分木の合成であり、機能単位の集約はその中間段に位置する。

### DES-1117

機能単位の表示経路（起点指定と内訳の提示）は§11.6のprojectionで露出し、新規コマンド・ツール・出力エンティティを増やさない。

### DES-1118

スキャン時にdocumentレコードの`content_hash`と実ファイル（`path`）を比較し、不一致ならW-SCAN-104を出す。

### DES-1119

当該documentを`derives_from`で参照するVO / 上位documentの鎖は、content_hash不一致として`chain_integrity = MISMATCH`（診断`STALE`）となる（§11.1.1）。

### DES-1120

当該document subjectをdependencyに含む判断記録（§8.5）・承認記録（§3.5）も無効となる。

### DES-1121

仕様文書の更新は`vtest doc add --update`による再登録で反映し、依存する判断・承認が失効することを利用者へ提示する。

### DES-1122

再登録でdocument subject hashが変化するため、以前のdependency entryを現在の承認・判断へ流用しない。

### DES-1123

プロジェクト側が登録したフェーズ・工程・ゲートの進行条件について、現在の検証状態（§4.1の5状態）と承認（§3.5）が通過条件を満たすかを評価・提示できなければならない（MUST）。

*導出元: REQ-302, REQ-303, REQ-304, REQ-305, REQ-306, REQ-307, REQ-308, SPEC-535, SPEC-536, SPEC-537, SPEC-538, SPEC-539, SPEC-540, SPEC-541, SPEC-542*

*引用: 基本仕様 §20, 要件定義 §26.4*

### DES-1124

検証状態と承認は独立の軸であり、ゲートは両者の組合せを進行条件にできる。

### DES-1125

ゲート定義は、`config.yaml`の`gates`（§2.2）に、ゲート名と進行条件（`require.verification`＝要求する検証結果、`require.approvals`＝要求する承認ロール集合）を保持する。

### DES-1126

`vtest verify --gate <name>`は、指定ゲートの対象scopeについて検証を実行し、(1) 検証結果が`require.verification`を満たすか、(2) `require.approvals`の各ロールについて対象の実効承認状態（§3.5）が`approved`であるか、を評価して満否と根拠（不足している非`PASS`検査・未充足の承認ロール）を提示する。

### DES-1127

`--gate <name>`は`gates[].name`との大文字小文字を区別した完全一致で解決する。

### DES-1128

`--gate <name>`の解決は前方一致・部分一致・近似一致・既定ゲートへの代替は行わない。

### DES-1129

一致するゲート定義が無い場合（`gates`が空、または未定義名の指定）はusage errorとしてE-CONFIG-002（終了コード2）で拒否し、スキャン・検証・ゲート評価のいずれも実行せず、検証結果・部分結果を生成しない。

### DES-1130

診断には指定名と定義済みゲート名の一覧を含める。

### DES-1131

検証条件の充足判定は、`require.verification`の値と、要求scopeの集約代表値との完全一致でのみ充足する。

### DES-1132

集約代表値は、要求scope内で評価した全値（構造検査`chain_integrity` / `orphan_detection`と、エンティティ軸の部分木で評価した各Test / VO / DOCの検査値）を§11.3のfail-closed規則で合成した1値とする。

### DES-1133

全値が`PASS`なら代表値は`PASS`（総合OKと同値）、非`PASS`が混在する場合は基本仕様 §22.2の優先順位`FAIL > MISMATCH > NO_EVIDENCE > UNKNOWN`で選ぶ。

*導出元: SPEC-577, SPEC-578, SPEC-579, SPEC-580, SPEC-581, SPEC-582*

*引用: 基本仕様 §22.2*

### DES-1134

診断ラベルは充足判定に用いない。

### DES-1135

5状態に順序・優劣・包含関係を設けない。

### DES-1136

「要求値以上」「要求値より良い」といった比較解釈を採らず、`require.verification: UNKNOWN`は代表値が`UNKNOWN`のときだけ充足し、代表値が`PASS`でも充足しない。

### DES-1137

同様に`require.verification: PASS`は代表値が`PASS`のときだけ充足する。

### DES-1138

`--items`で検査軸を限定した実行では、scope外の検査が`NO_EVIDENCE`（診断`NOT_CHECKED`）として代表値の合成に参加する（§11.3、基本仕様 §4.6）。

*導出元: SPEC-210, SPEC-211, SPEC-212, SPEC-213, SPEC-214, SPEC-215, SPEC-216*

*引用: 基本仕様 §4.6*

### DES-1139

したがって限定scopeでの`require.verification: PASS`は充足せず、限定scopeの結果でゲートを充足させることはできない。

### DES-1140

承認条件は検証条件と独立に評価し、`require.approvals`が空集合（省略）なら承認条件は充足とする。

### DES-1141

承認未充足は検証状態を降格させず、検証の非`PASS`は承認の充足有無を変えない（基本仕様 §4.5）。

*導出元: SPEC-203, SPEC-204, SPEC-205, SPEC-206, SPEC-207, SPEC-208, SPEC-209*

*引用: 基本仕様 §4.5*

### DES-1142

ゲート全体の充足は検証条件と承認条件の両方が充足した場合に限る。

### DES-1143

本システムの責務はゲート条件が現在満たされているかの評価・提示に限る。

### DES-1144

フェーズのライフサイクル管理・工程の自動遷移は責務外とする（§29 OOS-004）。

*導出元: REQ-302, REQ-303, REQ-304, REQ-305, REQ-306, REQ-307, REQ-308, SPEC-535, SPEC-536, SPEC-537, SPEC-538, SPEC-539, SPEC-540, SPEC-541, SPEC-542*

*引用: 基本仕様 §20, 要件定義 §26.4, OOS-004*

### DES-1145

「Releaseフェーズへ遷移させる」のではなく「Release gateの条件を現在満たしている」を提示する。

### DES-1146

新規CLIコマンド・MCPツールを増やさず、既存の`vtest verify`の`--gate`引数と出力、および`report`のJSONでゲート評価を露出する（引数・出力schemaは別紙A）。

*引用: 別紙A*

### DES-1147

具体的なフェーズ名・承認ロール・必要承認数・権限schemaはプロジェクト設定と別紙Aへ委譲する（基本仕様 §30）。

*導出元: SPEC-740, SPEC-741, SPEC-742, SPEC-743, SPEC-744, SPEC-745, SPEC-746, SPEC-747, SPEC-748, SPEC-749, SPEC-750, SPEC-751, SPEC-752, SPEC-753, SPEC-754, SPEC-755, SPEC-756, SPEC-757, SPEC-758, SPEC-759, SPEC-760, SPEC-761, SPEC-762, SPEC-763, SPEC-764, SPEC-765*

*引用: 別紙A, 基本仕様 §30*

### DES-1148

同一のトレーサビリティ構造から、利用者の役割または利用目的に応じて参照対象・関係・集約粒度を変えたprojectionを取得・提示できる。

*導出元: REQ-036, REQ-037, REQ-038, REQ-039, REQ-040, REQ-041, REQ-042, REQ-043, REQ-044, REQ-045, REQ-046, REQ-047, REQ-048, REQ-049, SPEC-520, SPEC-521, SPEC-522, SPEC-523, SPEC-524, SPEC-525, SPEC-526, SPEC-527, SPEC-528, SPEC-529, SPEC-530, SPEC-531, SPEC-532, SPEC-533, SPEC-534*

*引用: 基本仕様 §19, 要件定義 §3.4*

### DES-1149

最小の意味単位「上流ノード → 関係 → 下流ノード」を任意のノード（DOC / VO / TEST / SRC）から取得でき、必要に応じて上流／下流へ連続して辿れ、プロジェクト全体のトレーサビリティ構造も取得できる。

> 任意ノードからの取得。

### DES-1150

常に全チェーンを表示することは求めない。

### DES-1151

役割または利用目的に応じた参照観点をpresetとして提供する（例：PMは上位のdocument・VOの状態と未確定/NG、Testerは VO・Test・検証対象・Evidence・未実施/失敗理由、Coderは実装から関連Test・VO・上流documentへのトレース）。

### DES-1152

役割を固定enumやモード名として本冊で仕様化せず、preset・UI・モード体系は別紙Aへ委譲する（基本仕様 §30）。

*導出元: SPEC-740, SPEC-741, SPEC-742, SPEC-743, SPEC-744, SPEC-745, SPEC-746, SPEC-747, SPEC-748, SPEC-749, SPEC-750, SPEC-751, SPEC-752, SPEC-753, SPEC-754, SPEC-755, SPEC-756, SPEC-757, SPEC-758, SPEC-759, SPEC-760, SPEC-761, SPEC-762, SPEC-763, SPEC-764, SPEC-765*

*引用: 別紙A, 基本仕様 §30*

### DES-1153

親VOを起点とする下流方向のprojectionが、§11.3の機能単位の集約（Feature単位＝親VO）を提示する経路である。

### DES-1154

当該親VOの代表値と、その配下の子VOごと・Testごとの内訳を同じ出力から辿れる。

### DES-1155

Feature名・Feature IDの別fieldを出力に設けず、束ねの識別子は親VOのIDとする。

### DES-1156

新規コマンド・ツールを増やさず、既存の`vtest report`のview / projection引数と、`test query`の逆引きで露出する（引数・出力schemaは別紙A）。

*引用: 別紙A*

### DES-1157

逆引きインデックス（VO → Tests、SRC → Tests、DOC → VOs、DOC → DOCs）をprojectionの基盤とする（§5.3）。

### DES-1158

projectionが出力する`derives_from`エッジ（DOC → DOC、DOC → VO）には、当該entryの`anchor`（§3.1・§3.2）を常に同伴させる。

### DES-1159

`anchor`を持たないentryでは当該fieldを省略または`null`とし、空文字列で埋めない。

### DES-1160

projectionが出力する`derives_from`エッジに当該entryの`anchor`を常に同伴させることにより「どの上流条項が、どの概念（VO）へ対応するか」の対応ペアが構造化出力として取得でき、外部の発見者が未宣言の義務・網羅漏れを裁定する材料になる（基本仕様 §11.1）。

*導出元: SPEC-379, SPEC-380, SPEC-381, SPEC-382, SPEC-383*

*引用: 基本仕様 §11.1*

### DES-1161

`anchor`の値は不透明な文字列として transport するだけで、projectionは文書内位置への解決・整合検査を行わない。

### DES-1162

「どの上流条項が、どの概念（VO）へ対応するか」の対応ペアの取得のために新規コマンド・ツールを設けない。

### DES-1163

導入時・検証時に生じる、未確定事項・不整合・未検証事項・機械的に確認済みの事実・外部判断が必要な事項を、機械可読な構造として保持・取得可能とする。

*導出元: REQ-228, REQ-229, SPEC-515, SPEC-516*

*引用: 基本仕様 §18.3, 要件定義 §17.3*

### DES-1164

判断待ち情報は構造化record（report JSON内のsection）として提示する。

### DES-1165

`subject`は対象エンティティIDまたは解決済みcanonical Locatorとする。

### DES-1166

`kind`は`unknown`（UNKNOWNによるエスカレーション）/ `unregistered`（管理宣言欠落）/ `unresolved`（参照解決不能）/ `undecided`（VO未確定）/ `pending_approval`（承認待ち）のいずれかとする。

### DES-1167

`check`は関係する検査（4検査のいずれか）と現在の検証状態・診断ラベルとする。

### DES-1168

4検査のいずれにも由来しない項目（判断型に由来する項目・判断競合）では`check`を`null`とする。

### DES-1169

`check`が`null`の項目は§11.3の集約へ寄与せず、いかなる検査の値も変更しない。

### DES-1170

`judgment_kind`は外部判断が必要な場合の判断型（§8.1の値域）とする。

### DES-1171

不要な項目では`judgment_kind`を`null`とする。

### DES-1172

`basis`は機械的に確認済みの事実（宣言鎖・検査結果・対象外とした範囲）への参照とする。

### DES-1173

`bundle_ref`は外部判断が必要な場合の判断バンドル（§8.1）への参照（任意）とする。

### DES-1174

`judgment_kind: case-coverage`の項目（`kind: unknown`、`check: null`、`subject`＝対象Test ID）は、条件をすべて満たす管理対象Testごとにちょうど1件生成する。

### DES-1175

判断型に由来する項目の生成条件の1つは、`covers`が1件以上あることである。

### DES-1176

判断型に由来する項目の生成条件の1つは、当該Testの`cases`が1件以上ある、または解決済みのcovers先VO（レコードが存在するVO。E-SCAN-003のdangling参照を除く）のいずれかが`dimensions`を1件以上持つことである。

### DES-1177

判断型に由来する項目の生成条件の1つは、`(当該Test, case-coverage)`の実効判断（§8.5）が`accepted`でないことである。

### DES-1178

実効判断が未確定・`rejected`・`deferred`のいずれの場合も項目を生成し、参照した判断記録IDを`basis`に載せる。

### DES-1179

判断型に由来する項目の生成条件は`case-coverage`型の項目にだけ適用する。

### DES-1180

検査に由来する`kind: unknown`の項目（DA規則の解析限界等）の生成・消滅は当該検査の値だけで決まり、判断記録の有無で変わらない。

### DES-1181

§8.5の実効判断が競合により未確定となった`(subject, judgment_kind)`は、`kind: unknown`、`check: null`、当該`judgment_kind`、および競合した全判断記録IDを`basis`（`kind: decision`）に持つ項目として提示する。

### DES-1182

新規コマンド・ツールを増やさず、`vtest verify` / `vtest report`のJSON出力に判断待ちsectionを含めて露出する。

### DES-1183

UNKNOWNだけでなく、検証出力全体にわたる未確定・要判断事項を横断的に集約する（表示形式は別紙A、基本仕様 §30 item 19）。

*導出元: SPEC-740, SPEC-741, SPEC-742, SPEC-743, SPEC-744, SPEC-745, SPEC-746, SPEC-747, SPEC-748, SPEC-749, SPEC-750, SPEC-751, SPEC-752, SPEC-753, SPEC-754, SPEC-755, SPEC-756, SPEC-757, SPEC-758, SPEC-759, SPEC-760, SPEC-761, SPEC-762, SPEC-763, SPEC-764, SPEC-765*

*引用: 別紙A, 基本仕様 §30 item 19, 基本仕様 §30*

## DA-013 16. 並列動作と整合性

### DES-1184

本冊の§12〜§15は別紙Aで定義する。

### DES-1185

書き込み操作は上記のいずれかに分類され、ファイルロックを必要としない。

*導出元: SPEC-631, SPEC-632, SPEC-633, SPEC-634, SPEC-635, SPEC-636, SPEC-637, SPEC-638, SPEC-639, SPEC-640, SPEC-641, SPEC-642, SPEC-643, SPEC-644, SPEC-645, SPEC-646, SPEC-647*

*引用: 基本仕様 §24.2*

### DES-1186

新規レコード追加（rel / decisions / approvals / evidence）はULIDファイル名の新規作成のみであり、並列生成は衝突しない。

*導出元: SPEC-631, SPEC-632, SPEC-633, SPEC-634, SPEC-635, SPEC-636, SPEC-637, SPEC-638, SPEC-639, SPEC-640, SPEC-641, SPEC-642, SPEC-643, SPEC-644, SPEC-645, SPEC-646, SPEC-647*

*引用: 基本仕様 §24.2*

### DES-1187

エンティティファイル編集（doc / vo）は1エンティティ1ファイルであり、異なるエンティティの並列編集は独立であり、同一エンティティの並列編集はGitのマージ衝突として顕在化する。

*導出元: SPEC-631, SPEC-632, SPEC-633, SPEC-634, SPEC-635, SPEC-636, SPEC-637, SPEC-638, SPEC-639, SPEC-640, SPEC-641, SPEC-642, SPEC-643, SPEC-644, SPEC-645, SPEC-646, SPEC-647*

*引用: 基本仕様 §24.2*

### DES-1188

テストコード編集は通常のソース編集と同じ扱いとする。

*導出元: SPEC-631, SPEC-632, SPEC-633, SPEC-634, SPEC-635, SPEC-636, SPEC-637, SPEC-638, SPEC-639, SPEC-640, SPEC-641, SPEC-642, SPEC-643, SPEC-644, SPEC-645, SPEC-646, SPEC-647*

*引用: 基本仕様 §24.2*

### DES-1189

同時実行された`vtest`プロセス同士の調停は行わない。

### DES-1190

すべての判定は「その時点の正典の読み取り」に基づき、正典が変われば次回のscan / verifyが差分を反映する。

### DES-1191

「その時点の正典の読み取り」は書込みの原子的公開（基本仕様 §24.2）を前提とする。

*導出元: SPEC-631, SPEC-632, SPEC-633, SPEC-634, SPEC-635, SPEC-636, SPEC-637, SPEC-638, SPEC-639, SPEC-640, SPEC-641, SPEC-642, SPEC-643, SPEC-644, SPEC-645, SPEC-646, SPEC-647*

*引用: 基本仕様 §24.2*

### DES-1192

原子的公開の対象は`.verify/`配下のrecord・エンティティファイル（新規レコード追加とエンティティファイル編集）であり、完全な内容が単一の操作で可視になる方式（同一ファイルシステム内へのtemp書込み＋rename等）で公開し、書きかけ状態・一時ファイル残渣を正典ディレクトリの読み手に観測させてはならない。

### DES-1193

テストコード編集は通常のソース編集と同じ扱いで本規定の対象外とする。

### DES-1194

解析不能な中間状態はadapter discoveryのE-SCAN-001 / Incompleteとしてfail-closedに検出される（§5.1）。

### DES-1195

`vtest doctor`は、同じTest IDの重複、covers先VOの欠落、承認済VOの内容不一致など、version controlの構文的整合性だけでは判定できない論理的不整合を検出する。

### DES-1196

ID衝突はE-SCAN-002として検出する。

### DES-1197

dangling referenceはE-SCAN-003 / E-SCAN-009 / E-SCAN-012として検出する。

### DES-1198

孤児documentはE-SCAN-016として検出する。

### DES-1199

承認の失効は§3.5のハッシュ束縛により自動的にdraftへ遷移する。

### DES-1200

判断記録・Evidenceの失効は§8.5 / §11.2のハッシュ束縛により自動的に無効（診断STALE）へ遷移する。

## DA-014 17. 診断・終了コード体系

### DES-1201

診断コードは§5.4のスキャン診断に加えて定義する。

### DES-1202

`E-SCAN-017`はerrorであり、VOの`combinations`が不正（`coverage_policy: explicit`で欠落・空、`explicit`以外で非空、未宣言dimension・未列挙partitionの参照、宣言dimensionの欠落・重複、重複tuple。§3.2.1）である。当該VOの`chain_integrity`を`MISMATCH`とし、`vo expand`は子VOを生成しない。

> 表: | コード | 種別 | 内容 |

### DES-1203

`W-SCAN-104`はwarningであり、documentレコードのcontent_hashと実ファイルの不一致である（依存判断・依存Approvalは無効、鎖はchain_integrity STALE）。

### DES-1204

`E-EXEC-001`はerrorであり、テストビルド失敗である。

### DES-1205

`E-EXEC-002`はerrorであり、要求したテストの結果行が得られないことである。

### DES-1206

`E-EXEC-003`はerrorであり、終了コードと結果行集計の矛盾である。

### DES-1207

`E-EXEC-004`はerrorであり、実行中にExecution State subjectが変化することである。

### DES-1208

`W-EXEC-101`はwarningであり、カバレッジツール利用不能である（target_coverageはchecked: false、検証時NO_EVIDENCE/NOT_CHECKED）。

### DES-1209

`E-AUDIT-001`はerrorであり、提出されたbundle_idが存在しないことである。

### DES-1210

`E-AUDIT-002`はerrorであり、バンドル記録時のハッシュと現在のハッシュの不一致（対象が変更済）である。

### DES-1211

`E-AUDIT-003`はerrorであり、subjectまたはjudgment_kindの不一致・値域外・スキーマ違反である。

### DES-1212

`E-AUDIT-004`はerrorであり、decisionが受理する判断値でないことである。

### DES-1213

`E-AUDIT-008`はerrorであり、supersedesの参照先が存在しない、subjectまたはjudgment_kindが一致しない、または自己参照であることである（§8.4）。

### DES-1214

`E-APPROVAL-001`はerrorであり、Approval対象、`judgment_ref`の参照先、または上流依存closureを完全・currentに解決できず、recordを生成しないことである。

### DES-1215

`E-APPROVAL-002`はerrorであり、`approved_state`が値域外、`subject`の種別が値域外（判断記録ULID・Test ID等）、または`supersedes`の参照先が存在しない・対象が一致しない・自己参照であることである（§3.5。recordを生成しない）。

### DES-1216

`E-CONFIG-001`はerrorであり、config version、`verify.full_scope`（固定4検査）、`doc.roots`、`gates`（名前重複、`require` / `require.verification`欠落、`require.verification`が5状態語彙外、`require.approvals`の不正・未解決ロール）、config field型または登録adapterが検証する設定値が現在のconfig invariantに違反することである（未知・重複adapter IDはE-ADAPTER-001）。

### DES-1217

`E-CONFIG-002`はerrorであり、呼出しがconfigに定義の無いゲート名を参照することである（`--gate` / MCPの`gate`入力。config内容自体はinvariantを満たす。検証・ゲート評価を実行せず結果を生成しない。§11.5）。

### DES-1218

`E-OP-001`はerrorであり、Structured Operationの入力検証失敗（候補提示を伴う。§6.3）である。

### DES-1219

`E-OP-002`はerrorであり、Edit対象Testの特定失敗である。

### DES-1220

`E-OP-003`はerrorであり、Create / Editの適用後検証に失敗（再パース不能、生成された宣言がdesired stateと不一致、変更が1 Testの範囲を超える）することである。適用前の状態へロールバックし操作を中止する（別紙A §15.2・§15.4）。

*引用: 別紙A §15.2・§15.4, 別紙A §15.2, 別紙A §15.4*

### DES-1221

`E-ADAPTER-001`はerrorであり、adapterが未登録、重複、またはregistryの宣言と実装が不一致であることである。

### DES-1222

`E-ADAPTER-002`はerrorであり、adapterのdiscoveryまたはrunnerが確定的に失敗（Evidenceなし）することである。

### DES-1223

`E-ADAPTER-003`はerrorであり、Testのexecution descriptorと選択adapterが不一致であることである。

### DES-1224

`E-ADAPTER-004`はerrorであり、明示操作に必須のadapter capabilityが未提供（変更・判断・Evidenceなし）であることである。

### DES-1225

`W-ADAPTER-101`はwarningであり、検証対象のadapter capabilityが未提供であることである（能力に応じNO_EVIDENCE/NOT_CHECKEDまたはNOT_EXECUTED）。

### DES-1226

`W-ADAPTER-102`はwarningであり、adapterが解析限界を報告することである（該当検査はUNKNOWN）。

### DES-1227

旧モデルの意味監査提出検査（E-AUDIT-005 / E-AUDIT-006 / E-AUDIT-007）は判断記録層への転用（§8.4）に伴い撤去する。

### DES-1228

終了コード`0`は、要求scopeの検証結果がOK（操作コマンドでは成功）であることを意味する。

> 表: | コード | 意味 |

### DES-1229

終了コード`1`は、検証結果がNGであることを意味する。

### DES-1230

終了コード`2`は、操作拒否（E-OP-* / E-ADAPTER-* / E-APPROVAL-* / E-CONFIG-*、引数不正、adapter前提・capability・実行失敗、スキーマ違反の提出など。検証結果は生成しない）であることを意味する。

### DES-1231

終了コード`3`は、内部エラー（ツール自体の異常）であることを意味する。

### DES-1232

`--gate <name>`を指定した`vtest verify` / `vtest report`では、0と1をゲート充足で決める。

### DES-1233

ゲート全体が充足（§11.5の検証条件と承認条件の両方が充足）なら0、いずれかが不充足なら1とする。

### DES-1234

`require.verification`に`PASS`以外を定義したゲートでは、集約代表値が要求値と一致して充足した実行が0になり、この場合に総合がNGであることは0を妨げない。

### DES-1235

要求scopeの総合OK / NGはJSONとtextの集約出力から読み取れる（別紙A §12.1・§12.3）。

*引用: 別紙A §12.1・§12.3, 別紙A §12.1, 別紙A §12.3*

### DES-1236

ゲート名が未定義の場合はE-CONFIG-002で2とし、0 / 1を返さない。

### DES-1237

終了コードは診断severityだけでなく操作段階で決める。

### DES-1238

`vtest scan` / `vtest doctor`では、registry・config・adapter契約の検証またはadapter呼出しがE-ADAPTER-* / E-CONFIG-*で拒否された場合は2とする。

### DES-1239

`vtest scan` / `vtest doctor`では、scanが完了してrepository整合性のE-SCAN-*を報告した場合は1とする。

### DES-1240

`vtest scan` / `vtest doctor`では、errorがなければ0とする。

### DES-1241

同一実行に複数候補がある場合は内部エラー3、操作拒否2、検証NG1、成功0の順で優先する。

### DES-1242

検証状態と内部エラーは終了コードで分離する。

*導出元: SPEC-200, SPEC-201, SPEC-202, SPEC-665, SPEC-666, SPEC-667, SPEC-668, SPEC-669, SPEC-670, SPEC-671, SPEC-672, SPEC-673, SPEC-674, SPEC-675, SPEC-676, SPEC-677, SPEC-678, SPEC-679, SPEC-680, SPEC-681, SPEC-682, SPEC-683, SPEC-684, SPEC-685, SPEC-686, SPEC-687, SPEC-688, SPEC-689*

*引用: 基本仕様 §4.4、§26.1*

## DA-015 19. 実装選択と提供範囲

### DES-1243

demangle実装（`rustc-demangle`）の適用範囲は、基本仕様と本詳細設計の観測可能な契約を変更しない範囲で実装が選択できる。

> 次の事項は、基本仕様と本詳細設計の観測可能な契約を変更しない範囲で実装が選択できる。

### DES-1244

`#[tokio::test]`等、属性末尾`test`以外のカスタムテスト属性への対応範囲は、基本仕様と本詳細設計の観測可能な契約を変更しない範囲で実装が選択できる。

### DES-1245

cargo workspace外の単一クレートプロジェクトでのパス解決の細部は、基本仕様と本詳細設計の観測可能な契約を変更しない範囲で実装が選択できる。

### DES-1246

レポートのツリー描画の細部（文字種、折返し）は、基本仕様と本詳細設計の観測可能な契約を変更しない範囲で実装が選択できる。

### DES-1247

LSP / rust-analyzer連携によるシンボル解決は提供範囲外とする。

> 次の事項は提供範囲外とする。

### DES-1248

永続インデックス（`cache/`の活用）は提供範囲外とする。

### DES-1249

Relationのtombstone方式は提供範囲外とする。

### DES-1250

`rust-cargo`以外のproduction language adapter（synthetic adapterは受入fixture専用）は提供範囲外とする。

### DES-1251

LLM API直接呼び出しによる判断は提供範囲外とする。

### DES-1252

rename追跡とSRC恒久IDの自動昇格支援は提供範囲外とする。

### DES-1253

cargo-nextest対応は提供範囲外とする。

## DA-016 12. CLI 詳細仕様

### DES-1254

参照規則・診断コード・終了コードは本冊 §17 に従う。

*引用: 本冊 §17*

### DES-1255

本別紙は基本仕様 §26.1（CLI コマンド一覧）・§26.2（MCP ツール一覧）が確定したコマンド・ツールの引数と入出力 schema を具体化する HOW である。

*導出元: SPEC-665, SPEC-666, SPEC-667, SPEC-668, SPEC-669, SPEC-670, SPEC-671, SPEC-672, SPEC-673, SPEC-674, SPEC-675, SPEC-676, SPEC-677, SPEC-678, SPEC-679, SPEC-680, SPEC-681, SPEC-682, SPEC-683, SPEC-684, SPEC-685, SPEC-686, SPEC-687, SPEC-688, SPEC-689*

*引用: 本冊 §0, 基本仕様 §26.1*

### DES-1256

本別紙は新規コマンド・ツールを増やさない。

*引用: 本冊 §0*

### DES-1257

本別紙は、上流（要件定義＝WHY、基本仕様＝WHAT、詳細設計本冊＝HOW 中核）に無い義務・検査・状態・文書種別・関係型を発明しない。

### DES-1258

すべてのコマンドは非対話で完結する。

### DES-1259

確認プロンプトを出す場合は `--yes` で抑止できる。

### DES-1260

出力は既定で人間向けテキスト、`--format json` で機械可読 JSON。

### DES-1261

JSON 出力は最上位に `{ "ok": bool, "data": ..., "diagnostics": [...] }` を持つ。

### DES-1262

`diagnostics` の要素は `{ "code": "E-SCAN-002", "severity": "error", "message": "...", "location": ... }` である。

### DES-1263

検証結果を返す `verify` / `report`（CLI の `--format json` と同名 MCP ツール）は、これに加えて最上位に `scope` を持つ（下記要求 scope の最上位表現）。

### DES-1264

終了コードは本冊 §17.2 に従う。

*引用: 本冊 §17.2*

### DES-1265

グローバルオプションは `--project <dir>`（プロジェクトルート。既定はカレントから `.verify/` を上方探索）を持つ。

### DES-1266

グローバルオプションは `--format <text|json>` を持つ。

### DES-1267

グローバルオプションは `--quiet` を持つ。

### DES-1268

限定 scope の検証結果を完全検証と取り違えないため、`verify` / `report` の JSON は要求 scope と「scope 外は未検証」の旨を最上位 field `scope` として返す。

> ```json
> {
>   "ok": true,
>   "scope": {
>     "requested": {
>       "items": ["chain_integrity", "orphan_detection", "target_binding", "oracle_presence"],
>       "entities": [ { "kind": "doc", "id": "DOC-BASIC-001" } ]
>     },
>     "unverified_outside_scope": true
>   },
>   "data": {},
>   "diagnostics": []
> }
> ```

*導出元: SPEC-210, SPEC-211, SPEC-212, SPEC-213, SPEC-214, SPEC-215, SPEC-216*

*引用: 本冊 §11.3, 基本仕様 §4.6*

### DES-1269

text 出力の冒頭表示（§12.2）と同じ内容を機械可読に表したものである。

### DES-1270

`scope.requested.items` は、この実行で評価した検査軸を本冊 §11.1 の検査名で列挙する。

*引用: 本冊 §11.1*

### DES-1271

`--items`（MCP は `items[]`）省略時は固定4検査を 4 件すべて列挙し、空 list にしない。

### DES-1272

列挙順は上記例の固定順（`chain_integrity` / `orphan_detection` / `target_binding` / `oracle_presence`）とする。

### DES-1273

`scope.requested.entities` は、エンティティ軸で指定した対象を `{ "kind": "doc" | "vo" | "test", "id": ... }` の list として返す。

### DES-1274

`--doc` / `--vo` / `--test` をいずれも指定しない実行では `scope.requested.entities` は空 list とし、暗黙の根エンティティで埋めない。

### DES-1275

`scope.unverified_outside_scope` は、`requested.items` が 4 件未満、または `requested.entities` が空でない場合に `true`、それ以外（固定4検査 × エンティティ軸無指定）は `false` とする。

### DES-1276

`true` は「要求 scope 外は未検証であり、`PASS` ではない」ことを表す。

### DES-1277

scope 外・未実施の検査は集約ツリー内で `NO_EVIDENCE`（診断 `NOT_CHECKED`）として保持する。

*引用: 本冊 §11.3*

### DES-1278

`verify` / `report` は `unverified_outside_scope: false` の完全検証でも `scope` を省略しない。

### DES-1279

`scope` を持たない出力は限定 scope と区別できないため、完全検証の根拠として扱わない。

### DES-1280

`init` / `scan` / `doc *` / `vo *` / `test *` / `audit *` / `run` など検証結果を返さないコマンドは `scope` を持たない。

### DES-1281

検証結果を出力するすべてのコマンド（`verify` / `report` / `scan` の集約表示等）は、検証状態と診断ラベルを常に別軸の2列として提示する。

*導出元: SPEC-174, SPEC-175, SPEC-176, SPEC-177, SPEC-178, SPEC-179, SPEC-180, SPEC-181, SPEC-182, SPEC-183, SPEC-184, SPEC-185, SPEC-186, SPEC-187, SPEC-188, SPEC-189, SPEC-190*

*引用: 本冊 §5.2, 基本仕様 §4.1・§4.2*

### DES-1282

検証状態は5値（`PASS` / `FAIL` / `MISMATCH` / `NO_EVIDENCE` / `UNKNOWN`）である。

### DES-1283

JSON では検証状態を各検査ノードの `state` field へ入れる。

### DES-1284

診断ラベルは状態に付随する原因説明であり、`MISSING` / `NOT_EXECUTED` / `NOT_CHECKED` / `STALE` である。

### DES-1285

JSON では診断ラベルを各検査ノードの `diagnostic` field（0件以上）へ入れ、`state` の値には決して用いない。

### DES-1286

`NO_EVIDENCE` は状態であって診断ラベルではない。

### DES-1287

診断ラベルを集約の代表値選択に用いず、原因説明として併記するだけとする。

*導出元: SPEC-577, SPEC-578, SPEC-579, SPEC-580, SPEC-581, SPEC-582*

*引用: 本冊 §11.3, 基本仕様 §22.2*

### DES-1288

CLIの操作は登録済みadapter registryを通じて実装を選択する。

### DES-1289

JSON envelope、adapter選択エラー、capability不足の非PASS扱いはMCPと共通である。

### DES-1290

CLIだけがRust固有の既定値へフォールバックしてはならない。

### DES-1291

Testを含むJSONは本冊 §5.2の `execution` を必ず返す。

*引用: 本冊 §5.2*

### DES-1292

`rust-cargo` Testについてだけ、wire compatibility layerが `filter`、`package`、`test_target` を追加できる。

### DES-1293

`filter`、`package`、`test_target` は `TestEntity` の field ではない。

### DES-1294

非Rust TestではRust互換fieldを省略し、空値またはdummy値を返さない。

### DES-1295

Test JSONは `targets` を常に list として返す。

*引用: 本冊 §5.2*

### DES-1296

coreは `targets ≥ 1` を adapter 中立の必須件数にせず、型としては空 list を許容する。

### DES-1297

targetsの必須件数は adapter が定める。

### DES-1298

`rust-cargo` は `targets ≥ 1` を必須とする。

*引用: 本冊 §4.1・§4.4*

### DES-1299

targetが1件の場合だけ同値の単数互換field `target` を追加できる。

### DES-1300

複数target Testでは単数fieldを省略し、先頭targetを代表値として返さない。

### DES-1301

Test入力から `execution` を復元できるのは、`rust-cargo` codecに完全で相互整合するRust互換実行座標が与えられた場合だけである。

### DES-1302

`execution`と互換fieldが併存する場合は一致を必須とする。

### DES-1303

本 version の Test metadata は存在理由分類（旧 `role` / `anchor` / `anchor_rationale`）を持たない。

*導出元: REQ-052, REQ-053, REQ-054, REQ-055, REQ-056, REQ-057, REQ-058, SPEC-409, SPEC-410, SPEC-411, SPEC-412, SPEC-413, SPEC-414, SPEC-415, SPEC-416, SPEC-417, SPEC-418, SPEC-419, SPEC-420, SPEC-421, SPEC-422, SPEC-423, SPEC-424, SPEC-425*

*引用: 本冊 §4.1, 基本仕様 §12, 要件定義 §4.1*

### DES-1304

本 version はすべての管理対象 Test に `covers ≥ 1` を一律に要求する。

*導出元: REQ-052, REQ-053, REQ-054, REQ-055, REQ-056, REQ-057, REQ-058, SPEC-409, SPEC-410, SPEC-411, SPEC-412, SPEC-413, SPEC-414, SPEC-415, SPEC-416, SPEC-417, SPEC-418, SPEC-419, SPEC-420, SPEC-421, SPEC-422, SPEC-423, SPEC-424, SPEC-425*

*引用: 本冊 §4.1, 基本仕様 §12, 要件定義 §4.1*

### DES-1305

したがって CLI・MCP の入出力に role / anchor の宣言逐語 field・実効 field・既定値埋めは存在しない。

### DES-1306

VO への寄与は `covers` 宣言と証拠の十分性判定だけから導出する。

### DES-1307

明示操作に必須のadapter capabilityが未提供なら、`ok: false`、E-ADAPTER-004、終了コード2を返す。

### DES-1308

明示操作に必須のadapter capabilityが未提供なら、create / editではファイルを変更しない。

### DES-1309

明示操作に必須のadapter capabilityが未提供なら、auditでは判断記録を生成しない。

### DES-1310

明示操作に必須のadapter capabilityが未提供なら、runではEvidenceを生成しない。

### DES-1311

検証・reportで能力不足を観測した場合はW-ADAPTER-101と能力別の非PASS値（static / coverage 欠落は `NO_EVIDENCE`／診断 `NOT_CHECKED`、runner 欠落は `NO_EVIDENCE`／診断 `NOT_EXECUTED`、解析限界は `UNKNOWN`）を返す。

*導出元: SPEC-583, SPEC-584, SPEC-585, SPEC-586, SPEC-587, SPEC-588, SPEC-589, SPEC-590, SPEC-591, SPEC-592, SPEC-593, SPEC-594, SPEC-595*

*引用: 本冊 §5.2 末尾, 基本仕様 §22.3*

### DES-1312

`vtest init` は `.verify/` 一式を生成する。

> ```text
> vtest init [--name <project-name>]
> ```

*引用: 本冊 §2.1*

### DES-1313

`config.yaml` は本冊 §2.2 の version 2 で、組込 `rust-cargo` adapter namespace を含む。

*引用: 本冊 §2.2*

### DES-1314

`vtest init` の生成物には `doc/` / `vo/` / `rel/` / `forms/` / `decisions/` / `approvals/` / `evidence/` / `cache/` と `.verify/.gitignore`、組込 Form Schema（§14）を含む。

### DES-1315

既存の `.verify/` があれば `vtest init` はエラー（終了コード 2）とする。

### DES-1316

`vtest init` は `.verify/` を作成するだけであり、既存コードを変更しない。

*導出元: R-5, SPEC-500, SPEC-501, SPEC-502, SPEC-503, SPEC-504, SPEC-505, SPEC-506, SPEC-507, SPEC-508, SPEC-509*

*引用: 基本仕様 §18.1, 要件定義 R-5*

### DES-1317

`vtest init` が作成するファイル・ディレクトリは `.verify/` とその配下に限る。

### DES-1318

`vtest init` は、プロジェクトルート直下の `.gitignore`・ビルド設定（`Cargo.toml` 等）・CI 設定を含め、`.verify/` の外にあるいかなるファイルも新規作成・変更・削除しない。

### DES-1319

`vtest init` は既存ソースコード・既存テストコードのバイト列を変更しない。

### DES-1320

`vtest init` は Test metadata 宣言（`@vtest.` 行）・annotation・doc comment を既存ソースへ挿入しない。

### DES-1321

管理宣言の付与は `test create` / `test edit`（§15）と利用者自身の編集だけが行い、`vtest init` は行わない。

### DES-1322

`vtest init` は既存の `.verify/` があるときは終了コード 2 で中止し、その実行でファイル・ディレクトリを 1 件も作成・変更・削除しない。

### DES-1323

`vtest init` は既存 `.verify/` の内容を上書き・マージ・移動しない。

### DES-1324

したがって `vtest init` の実行前後で、`.verify/` を除いた作業ツリーの内容は同一である。

### DES-1325

既存プロジェクトへの後からの導入が既存資産を書き換えないことは、この不変条件で保証する。

### DES-1326

`vtest scan` はスキャンと整合性検査を実行し、診断一覧とエンティティ数のサマリを出力する。

> ```text
> vtest scan
> ```

*引用: 本冊 §5*

### DES-1327

整合性検査は `chain_integrity`（文書鎖・VO derives_from・Test 管理宣言）と `orphan_detection`（文書層孤児）を構成する。

*導出元: SPEC-596, SPEC-597, SPEC-598, SPEC-599, SPEC-600, SPEC-601, SPEC-602, SPEC-603, SPEC-604, SPEC-605, SPEC-606, SPEC-607, SPEC-608, SPEC-609, SPEC-610, SPEC-611, SPEC-612, SPEC-613, SPEC-614, SPEC-615, SPEC-616, SPEC-617*

*引用: 本冊 §5.6, 基本仕様 §23*

### DES-1328

`vtest scan` は registry・config・adapter契約の検証または adapter 呼出しが E-ADAPTER-* / E-CONFIG-* で拒否された場合は終了コード2とし、scan結果を生成しない。

### DES-1329

`vtest scan` は scan が完了し repository 整合性の E-SCAN-* 診断がある場合は終了コード1、error 診断がなければ0とする。

*引用: 本冊 §17.2*

### DES-1330

`vtest doctor` は `vtest scan` と同一処理の別名であり、自動化環境の整合性検査に使用する。

*引用: 本冊 §16.2*

### DES-1331

`vtest doctor` は、同じTest IDの重複（E-SCAN-002）、covers先VOの欠落（E-SCAN-003）、文書鎖のリンク切れ（E-SCAN-012）、孤児 document（E-SCAN-016）、承認・判断・Evidenceのハッシュ束縛による失効（診断 `STALE`）など、version control の構文的整合性だけでは判定できない論理的不整合を検出する。

### DES-1332

`doc` は上流文書を総称 `document` レコードとして管理する唯一のコマンドである。

> ```text
> vtest doc add --id DOC-BASIC-001 --path docs/basic-spec.md
>               [--title <t>]
>               [--derives-from DOC-REQ-001 [--anchor <text>] [--note <text>]]...
>               [--root | --no-root] [--update]
> vtest doc list [--tree] [--roots]
> vtest doc show DOC-BASIC-001
> ```

*導出元: SPEC-121, SPEC-122, SPEC-123, SPEC-124, SPEC-125, SPEC-126, SPEC-127, SPEC-128, SPEC-129, SPEC-130, SPEC-131, SPEC-132, SPEC-133, SPEC-134, SPEC-135, SPEC-136, SPEC-137, SPEC-138, SPEC-139, SPEC-140, SPEC-141, SPEC-142, SPEC-143, SPEC-144, SPEC-145, SPEC-146, SPEC-147, SPEC-148, SPEC-149, SPEC-150, SPEC-151, SPEC-152, SPEC-153, SPEC-154, SPEC-155, SPEC-156, SPEC-157, SPEC-158, SPEC-159, SPEC-160, SPEC-161, SPEC-162, SPEC-163, SPEC-164*

*引用: 本冊 §3.1, 基本仕様 §3.1・§3.2*

### DES-1333

`doc` は文書種別（要件定義・基本仕様・詳細設計・API Schema 等）を区別しない。

### DES-1334

段（要件→仕様→詳細設計…）は `derives_from` リンクで表現し、種別を増やさない。

### DES-1335

旧モデルの `vtest spec` / `vtest req` は廃し、SPEC / REQ 実体層は持たない。

### DES-1336

`doc add` は `--path` の対象ファイルの sha256 を計算して document subject へ束縛した DOC レコードを作成する。

*引用: 本冊 §1.3*

### DES-1337

`--derives-from` は上流 document への導出リンク（0件可＝根候補）である。

### DES-1338

各 `--derives-from` リンクに任意の `--note`（導出理由・空可・非 `MISMATCH`）を付けられる。

*引用: 基本仕様 §3.4*

### DES-1339

`--anchor <text>` は直前の `--derives-from` に束縛し、参照先 document 内の該当箇所（節番号・条項番号・見出し等）を記録する。

*引用: 本冊 §3.1*

### DES-1340

`--anchor` は `--note` と同じ結合規則・同じ任意性であり、省略・空文字列は `chain_integrity` 違反にならない。

### DES-1341

`--anchor` の値は不透明な文字列として保存し、文書内位置への解決・実在確認・書式検証を行わない。

### DES-1342

`--derives-from` を伴わない `--anchor`、または 1 つの `--derives-from` に対する 2 個目以降の `--anchor` は引数不正として終了コード 2 で拒否し、レコードを書かない。

### DES-1343

`doc show` は各 `derives_from` entry の `anchor` を表示する。

### DES-1344

`--root` / `--no-root` は当該 DOC を `orphan_detection` の除外根（`config.yaml` の `doc.roots`）へ追加／除外する。

*引用: 本冊 §2.2・§5.6*

### DES-1345

根指定の追加・削除はこのフラグで管理し `doc.roots` へ反映する。

### DES-1346

`doc edit` は設けない。

### DES-1347

正典編集は `add --update` で行う。

### DES-1348

`--update` は既存 DOC レコードの sha256 を現ファイルで再計算して更新する。

### DES-1349

`--update` は document subject hash が変化するため、当該 document を依存 closure に含む判断記録・承認が失効する旨を出力する。

*引用: 本冊 §3.5・§8.5・§11.4*

### DES-1350

`--update` は `--root` / `--no-root` を併せて根指定も更新できる。

### DES-1351

`doc list --tree` は `derives_from` の文書鎖を木として表示する。

### DES-1352

`doc list --roots` は現在の根集合を表示する。

### DES-1353

`doc show` は DOC の path・content_hash・derives_from・根指定・鮮度（content_hash と実ファイルの一致）・実効承認状態を表示する。

*引用: 本冊 §3.5*

### DES-1354

document の承認・却下・取消は `vtest approval` で行い、`doc` 側に承認操作を置かない。

### DES-1355

`derives_from` の参照先 document が存在しなければ文書鎖のリンク切れとして `chain_integrity = MISMATCH`（E-SCAN-012）とする。

### DES-1356

`path` の実ファイルが `content_hash` と一致しなくなれば `chain_integrity = MISMATCH`（診断 `STALE`）とする。

*引用: 本冊 §11.4*

### DES-1357

根に指定されず親も持たない document は孤児として `orphan_detection = MISMATCH`（E-SCAN-016）とする。

*引用: 本冊 §5.6*

### DES-1358

VO は 1 件以上の `document` から `derives_from` で直結して導出される。

> ```text
> vtest vo add --id VO-X --claim <c>
>              --derives-from DOC-X [--anchor <text>] [--note <text>]
>              [--derives-from DOC-Y [--anchor <text>] [--note <text>]]...
>              [--parent VO-Y]
>              [--dimension <name>=<p1>,<p2>...]... [--policy <policy>]
>              [--combination <dim>=<part>[,<dim>=<part>]...]...
> vtest vo edit VO-X [--claim ...] [--derives-from DOC-X [--anchor <text>] [--note <text>]]...
>              [--parent ...] [--dimension ...]... [--policy ...]
>              [--combination ...]... [--clear-combinations]
> vtest vo list [--tree] [--doc DOC-X] [--status draft|approved]
> vtest vo show VO-X          # claim、derives_from、covers している Test、判断記録・承認状態を表示
> vtest vo expand VO-X [--dry-run]
> vtest vo approve VO-X --approver-kind <human|agent> --approver-id <id>
>                  --state <approved|rejected|withdrawn>
>                  [--model <m>] [--basis <ref>]... [--supersedes <approval-id>]...
> ```

*導出元: SPEC-144, SPEC-145, SPEC-146, SPEC-147, SPEC-148, SPEC-149, SPEC-150, SPEC-151, SPEC-152, SPEC-153, SPEC-154, SPEC-155, SPEC-156, SPEC-157, SPEC-158, SPEC-159, SPEC-160, SPEC-161, SPEC-162, SPEC-163, SPEC-164*

*引用: 本冊 §3.2, 基本仕様 §3.2*

### DES-1359

旧モデルの `--req`（REQ 参照）・`--spec` / `--section`（SPEC + 節参照）は廃し、上流参照は `--derives-from DOC-*`（任意の `--note`）へ一本化する。

### DES-1360

VO の `status`（`draft` / `approved`）は正典 field ではなく承認レコードから導出する表示値である。

*引用: 本冊 §3.2・§3.5*

### DES-1361

`status` が読取り互換 field として保存されていても値は無視し、存在自体は W-STORE-001 とする。

### DES-1362

旧 REQ の `active` / `withdrawn` 語彙は REQ 層とともに廃止する。

### DES-1363

`--doc DOC-X` は当該 document を根とする下流 VO の絞り込みである。

### DES-1364

`vo add` / `vo edit` の `--anchor <text>` は直前の `--derives-from` に束縛し、参照先 document 内の該当箇所（節番号・条項番号・見出し等）を記録する。

*引用: 本冊 §3.2*

### DES-1365

`vo add` / `vo edit` の `--anchor` は `--note` と同じ結合規則・同じ任意性であり、省略・空文字列は `chain_integrity` 違反にならず、値は不透明な文字列として保存する。

### DES-1366

`--derives-from` を伴わない `--anchor`、または 1 つの `--derives-from` に対する 2 個目以降の `--anchor` は引数不正として終了コード 2 で拒否し、レコードを書かない。

### DES-1367

`vo show` は各 `derives_from` entry の `anchor` を表示する。

### DES-1368

`anchor` は VO subject hash に入らないため、`anchor` だけを変更した `edit` は承認を失効させない。

*引用: 本冊 §3.2*

### DES-1369

`--combination` は `coverage_policy: explicit` のときに実体化する組合せ（`combinations`）を入力する。

*引用: 本冊 §3.2.1*

### DES-1370

`--combination` の1回の出現が1 tupleに対応し、`<dim>=<part>` をカンマ区切りで並べて全軸の値を与える（例：`--combination operand-sign=positive,operator=div`）。

### DES-1371

複数 tuple は `--combination` を繰り返して与える。

### DES-1372

`vo edit` の `--combination` は desired state であり、1 回以上与えたときは既存 `combinations` を与えた集合で置換する（追記しない）。

### DES-1373

`--clear-combinations` は `combinations` を空にする。

### DES-1374

`--combination` も `--clear-combinations` も与えない `vo edit` は既存 `combinations` を保持する。

### DES-1375

`--combination` の値が本冊 §3.2.1 の受理条件（`explicit` 以外での指定、未宣言 dimension、未列挙 partition、宣言 dimension の欠落・重複、重複 tuple、`explicit` かつ tuple 0 件）に違反する場合は E-SCAN-017、終了コード 2 で拒否し、レコードを書かない。

*引用: 本冊 §3.2.1*

### DES-1376

`vo add` はこの違反時に新規レコードを作成せず、`vo edit` は既存レコードを変更しない。

### DES-1377

`<dim>=<part>` の形をなさない値は引数不正として終了コード 2 で拒否する。

### DES-1378

`vo expand` は本冊 §3.2.1 の実体化（`independent-axes` / `full-product` / `explicit`）である。

*引用: 本冊 §3.2.1*

### DES-1379

`--dry-run` は生成予定の子 VO 一覧のみ表示する。

### DES-1380

`explicit` の VO は `combinations` の各 tuple につき 1 件の子 VO を、`dimensions` の宣言順に連結した suffix（`VO-X-<P1>-<P2>`）で生成する。

### DES-1381

`combinations` が本冊 §3.2.1 の受理条件に違反する VO に対しては E-SCAN-017、終了コード 2 とし、子 VO を 1 件も生成しない（部分生成しない）。

*引用: 本冊 §3.2.1*

### DES-1382

`vo approve VO-X <承認引数>` は `vtest approval create --subject-type vo --subject-id VO-X <承認引数>` の別名であり、引数・拒否条件・生成されるレコードは同一である。

### DES-1383

承認の意味論を重複して定義せず、正典は次項の `vtest approval` と本冊 §3.5 とする。

*引用: 本冊 §3.5*

### DES-1384

`vo list --status` および `vo show` が表示する承認状態は、本冊 §3.5 の実効承認導出（`approved_state` を参照し、実効集合に `rejected` / `withdrawn` が1件でも残れば `draft`）の結果であり、承認レコードの件数・新旧からは導出しない。

*引用: 本冊 §3.5*

### DES-1385

`vo edit` は実効承認が `approved` の VO に対して警告を出す（編集自体は許可し、承認はハッシュ不一致で自動失効する）。

### DES-1386

承認は特定のエンティティ型に従属しない独立の領域であり、対象種別を引数に取るこの経路が承認レコード生成の唯一の正典面である。

> ```text
> vtest approval create --subject-type <vo|document|judgment> --subject-id <id>
>                       --state <approved|rejected|withdrawn>
>                       --approver-kind <human|agent> --approver-id <id>
>                       [--model <m>] [--basis <ref>]... [--supersedes <approval-id>]...
> vtest approval withdraw <approval-id>
>                       --approver-kind <human|agent> --approver-id <id>
>                       [--model <m>] [--basis <ref>]...
> vtest approval show --subject-type <vo|document|judgment> --subject-id <id>
> ```

*引用: 本冊 §3.5*

### DES-1387

エンティティ側の `vo approve` / `vo_approve` はこの経路への別名にすぎず、追加・相異する規則を持たない。

### DES-1388

`--subject-type` と `--subject-id` は本冊 §3.5 の承認対象の値域に対応する。

*引用: 本冊 §3.5*

### DES-1389

`--subject-type vo` は `subject` に VO ID を書き込む。

### DES-1390

`--subject-type document` は `subject` に document ID を書き込む。

### DES-1391

`--subject-type judgment` は `--subject-id` に判断記録 ULID を取り、`judgment_ref` へ書き込んだうえで `subject` に当該判断記録の `subject` を、`subject_hash` / `dependencies` にその対象の現在値を記録する。

### DES-1392

方針は総称 document として登録した文書で表現するため、方針の承認・却下・取消は `--subject-type document` で記録する。

*引用: 本冊 §3.1・§3.5*

### DES-1393

`--state` は必須で、本冊 §3.5 の `approved_state`（`approved` / `rejected` / `withdrawn`）を与える。

*引用: 本冊 §3.5*

### DES-1394

`--basis` は根拠参照（任意）である。

### DES-1395

`--supersedes` は明示に置き換える旧承認レコード ID（0件以上）である。

### DES-1396

`withdraw <approval-id>` は `create --subject-type <当該レコードの対象種別> --subject-id <当該レコードの対象> --state withdrawn --supersedes <approval-id>` と同一のレコードを生成する短縮形であり、追加の意味論を持たない。

### DES-1397

`show` は当該対象の承認レコード一覧（`approved_state`・`supersedes`・有効性）と、本冊 §3.5 の実効承認状態（`draft` / `approved`）を返す。

*引用: 本冊 §3.5*

### DES-1398

対象、`--subject-type judgment` の参照先判断記録、またはいずれかの依存 entity / document source を完全・current に解決できない場合は E-APPROVAL-001、終了コード 2 として record を追加しない。

### DES-1399

`--state` が値域外、`--subject-type` と `--subject-id` の種別が一致しない、`--supersedes` の参照先が存在しない・対象が一致しない・自己参照のいずれかであれば E-APPROVAL-002、終了コード 2 として record を追加しない。

### DES-1400

実効承認は明示の `supersedes` 関係だけで決まる。

### DES-1401

`supersedes` 関係にない複数の有効承認レコードはすべて実効集合に属し、`approved_at` / ULID の順序でどれかを「現在の承認」に選ぶことはしない。

### DES-1402

実効集合に `rejected` / `withdrawn` が 1 件でも残れば `draft` とする。

### DES-1403

取消・却下の後に再承認するには、当該レコード ID を `--supersedes` で名指しした `--state approved` を追加する。

*引用: 本冊 §3.5*

### DES-1404

承認は検証状態と独立の別軸であり、承認済みを理由に非 `PASS` を `PASS` へ昇格させない。

*導出元: SPEC-203, SPEC-204, SPEC-205, SPEC-206, SPEC-207, SPEC-208, SPEC-209, SPEC-471, SPEC-472, SPEC-473, SPEC-474, SPEC-475, SPEC-476, SPEC-477, SPEC-478, SPEC-479, SPEC-480, SPEC-481, SPEC-482, SPEC-483, SPEC-484, SPEC-485, SPEC-486, SPEC-487, SPEC-488, SPEC-489, SPEC-490, SPEC-491, SPEC-492, SPEC-493, SPEC-494, SPEC-495, SPEC-496, SPEC-497*

*引用: 本冊 §3.5, 基本仕様 §4.5・§17*

### DES-1405

`vtest test create` は Form Schema（§14）に基づく回答ファイルを受け取り、検証のうえ対応 adapter が Test construct と metadata 宣言を生成して挿入する。

> ```text
> vtest test create --form rust-unit-function
>                   --answers answers.yaml [--dry-run]
> ```
>
> 回答ファイル例：
>
> ```yaml
> form: rust-unit-function
> answers:
>   target: src/parser.rs::Parser::parse
>   covers: [VO-PARSER-UTF8-003]
>   behavior: 不正 UTF-8 入力の拒否
>   test_kind: error
>   input: 不正な continuation byte を含むバイト列
>   expect: ParseError::InvalidUtf8
>   fn_name: rejects_malformed_utf8
>   file: tests/parser_test.rs        # 省略時は target と同居する tests モジュール
> ```

### DES-1406

`--dry-run` は挿入内容と挿入位置のみを表示する。

### DES-1407

回答の検証エラーは E-OP-001 として候補付きで報告する。

*引用: 本冊 §6.3*

### DES-1408

`vtest test edit` は desired state 方式である。

> ```text
> vtest test edit TEST-X --answers desired.yaml [--dry-run]
> vtest test edit TEST-X --set covers=VO-A,VO-B [--set intent="..."]...
> ```

*導出元: SPEC-440, SPEC-441, SPEC-442, SPEC-443*

*引用: 基本仕様 §15.1*

### DES-1409

`--answers` は完全なあるべき状態を宣言する。

### DES-1410

`--set` は指定フィールドのみのあるべき値を宣言する。

### DES-1411

Test implementation の書き換えは `--body-file <path>` で adapter へ全文を与える。

> 編集の実装は §15 に定める。

### DES-1412

`test show` は Test の intent・covers・targets（宣言 target 集合）・Source Location・判断記録（§8）・Evidence（§9）の状態を表示する。

> ```text
> vtest test show TEST-X        # intent、covers、targets、位置、判断記録・Evidence 状態
> vtest test list [--vo VO-X] [--unregistered]
> vtest test query --source rust-cargo::src/parser.rs::Parser::parse   # SRC からの逆引き
> ```

### DES-1413

`test show` は role / anchor の表示・`--role` フィルタを持たない。

*引用: 本冊 §4.1*

### DES-1414

`test query` の逆引きは §11.6 の役割別 projection の基盤（VO → Tests、SRC → Tests）としても用いる。

*引用: 本冊 §5.3*

### DES-1415

`vtest audit static` は決定論的な静的解析を要求時に起動し、rule 別 verdict（`FAIL` / `UNKNOWN` / 違反なし）と根拠 span を stdout と `cache/` へ出力する。

> ```text
> vtest audit static [--test TEST-X | --all]
> ```

*引用: 本冊 §7*

### DES-1416

静的解析は正典レコードを持たない再計算派生であり、`audit static` は正典の監査レコードを生成しない。

*導出元: P-003*

*引用: 本冊 §7.1, 基本仕様 P-003*

### DES-1417

`audit static` は `oracle_presence` へ供給する DA-001 / DA-003 / DA-004 / DA-005 / DA-006 と、`target_binding` の静的到達（DA-002）を評価する。

### DES-1418

target-scoped な DA-002 / DA-003 は宣言 target ごとの verdict を規則単位 verdict と併せて提示する。

*引用: 本冊 §3.6・§7.2*

### DES-1419

`audit static` は判断記録（§8）とは別機構であり、外部判断の記録には転用しない。

*引用: 本冊 §7*

### DES-1420

`audit bundle` / `submit` は本冊 §8 の判断記録プロトコルであり、意味検査ではない。

> ```text
> vtest audit bundle (--test TEST-X | --vo VO-X)
>                    [--kind test-semantic | impl-consistency | case-coverage]
>                    [--include-failed]
> vtest audit submit --file result.json
> ```

*引用: 本冊 §8*

### DES-1421

本システムは宣言されていない義務・網羅漏れ・宣言と実装の意味のずれを自ら発見・裁定しない。

### DES-1422

本システムは機械が決定論で確定できない疑義を `UNKNOWN` として外部（人間または判断可能 Agent）へ引き渡し、その判断を判断記録（`.verify/decisions/`）として追跡する。

*導出元: REQ-180, REQ-181, REQ-182, REQ-183, REQ-184, REQ-185, REQ-186, REQ-187, REQ-188, REQ-189, REQ-190, REQ-191, REQ-192, SPEC-376, SPEC-377, SPEC-378, SPEC-379, SPEC-380, SPEC-381, SPEC-382, SPEC-383, SPEC-384, SPEC-385, SPEC-386, SPEC-387, SPEC-388, SPEC-389, SPEC-390, SPEC-391, SPEC-392, SPEC-393, SPEC-394, SPEC-395, SPEC-396, SPEC-397, SPEC-398, SPEC-399, SPEC-400, SPEC-401, SPEC-402, SPEC-403, SPEC-404, SPEC-405, SPEC-406, SPEC-407, SPEC-408*

*引用: 本冊 §8 冒頭, 基本仕様 §11, 要件定義 §12*

### DES-1423

`audit bundle` は判断対象（`--test` / `--vo`）ごとに、判断に必要な情報一式（対象 VO と claim・Test Intent・テストコード全文・Test が宣言した cases 集合・対象実装全文・関連テスト・既知 partition・過去の判断・対象の内容ハッシュとリビジョン）を JSON として `cache/bundles/<ULID>.json` へ出力し、パスと `bundle_id` を返す。

*引用: 本冊 §8.1*

### DES-1424

`cases` は `@vtest.case` 宣言の正規化文字列を宣言順に並べた list であり、宣言が無い Test でも空 list を明示して項目を省略しない。

### DES-1425

バンドルは派生情報であり Git 管理しない。

### DES-1426

`--kind` は判断させる UNKNOWN のエスカレーション質問のラベル（本冊 §8.1 の判断型）であって検査項目ではない。

*引用: 本冊 §8.1*

### DES-1427

`--kind` の値と `subject` 値域は本冊 §8.1 の表に従う。

*引用: 本冊 §8.1*

### DES-1428

`test-semantic` は「テストコードは VO の claim と Test Intent が宣言する振る舞いを実際に検証しているか」を意味し、`--test` のみに使う。

*引用: 本冊 §8.6*

### DES-1429

`impl-consistency` は「対象実装が宣言と一致するか」を意味し、`--test` のみに使い、上流 document を要するため §3.5 と同じ上流依存規則で document 完全集合を同梱する。

### DES-1430

`case-coverage` は「cases 集合が VO の要求入力空間を十分に代表・網羅しているか」を意味し、`--test` / `--vo` の双方に使う。

*導出元: SPEC-430, SPEC-431, SPEC-432, SPEC-433, SPEC-434*

*引用: 本冊 §8.1, 基本仕様 §14*

### DES-1431

`--test` で `--kind` を省略した場合は `test-semantic` とする。

### DES-1432

`--vo` では `--kind case-coverage` を必須とし、`--kind` 省略および `--vo` と `test-semantic` / `impl-consistency` の組合せは usage error（終了コード 2）としてバンドルを生成しない。

### DES-1433

旧モデルの `spec-coverage`（SPEC 層依存）は復活させない。

### DES-1434

バンドルは選ばれた判断型を `judgment_kind` として出力し、`audit submit` はこれを判断記録へ複製する。

### DES-1435

`case-coverage` は §11 の判断対象であって基本仕様 §5 の 4 検査ではない。

*導出元: SPEC-217, SPEC-218, SPEC-219, SPEC-220, SPEC-221, SPEC-222, SPEC-223, SPEC-224, SPEC-225, SPEC-226, SPEC-227, SPEC-228, SPEC-229, SPEC-230, SPEC-231, SPEC-232, SPEC-233, SPEC-234, SPEC-235, SPEC-236, SPEC-237, SPEC-238, SPEC-239, SPEC-240, SPEC-241, SPEC-242, SPEC-243, SPEC-244, SPEC-245, SPEC-246, SPEC-247, SPEC-248, SPEC-249, SPEC-250, SPEC-251, SPEC-252, SPEC-253, SPEC-254, SPEC-255, SPEC-256, SPEC-257, SPEC-258, SPEC-259, SPEC-260, SPEC-261, SPEC-262, SPEC-263, SPEC-264, SPEC-265, SPEC-266, SPEC-267, SPEC-268, SPEC-269, SPEC-270, SPEC-271, SPEC-272, SPEC-273, SPEC-274, SPEC-275, SPEC-276, SPEC-277, SPEC-278, SPEC-279, SPEC-280, SPEC-281, SPEC-282, SPEC-283, SPEC-284, SPEC-285, SPEC-286*

*引用: 基本仕様 §5*

### DES-1436

`case-coverage` の未判断・判断結果はいずれの検査の値にも写像せず集約へ寄与しない。

*引用: 本冊 §8.1・§11.3*

### DES-1437

`audit submit` は本冊 §8.4 の検証（bundle_id 存在＝E-AUDIT-001、subject 一致＝E-AUDIT-003、judgment_kind 一致・値域＝E-AUDIT-003、記録時ハッシュと現在ハッシュの一致＝E-AUDIT-002、decision が受理値＝E-AUDIT-004、supersedes の参照先が同一 subject・同一 judgment_kind の既存判断記録で自己参照でない＝E-AUDIT-008）を行い、受理時に判断記録 ID（`.verify/decisions/` の ULID）を出力する。

*引用: 本冊 §8.4*

### DES-1438

判断は少なくとも actor / subject / decision / judgment_kind を含み、理由・根拠（`reason` / `exclusions`）と `supersedes` は任意である。

### DES-1439

理由が空であることだけを根拠に判断を無効化しない。

*導出元: REQ-180, REQ-181, REQ-182, REQ-183, REQ-184, REQ-185, REQ-186, REQ-187, REQ-188, REQ-189, REQ-190, REQ-191, REQ-192, SPEC-387, SPEC-388, SPEC-389, SPEC-390, SPEC-391, SPEC-392, SPEC-393, SPEC-394, SPEC-395, SPEC-396, SPEC-397, SPEC-398, SPEC-399, SPEC-400, SPEC-401, SPEC-402, SPEC-403, SPEC-404, SPEC-405, SPEC-406, SPEC-407, SPEC-408*

*引用: 本冊 §8.3, 基本仕様 §11.3, 要件定義 §12*

### DES-1440

`decision` の受理値は `accepted` / `rejected` / `deferred` 等である。

*引用: 本冊 §8.3*

### DES-1441

競合の解消は `supersedes` だけによる。

### DES-1442

同一 `(subject, judgment_kind)` に判断値の食い違う有効判断記録が併存する場合、実効判断は未確定（`UNKNOWN`）とし、W-STORE-004 を出す。

### DES-1443

機械は新旧・decision 値・件数のいずれによっても採用記録を選ばない。

### DES-1444

新しい判断記録が旧記録の ULID を `supersedes` で名指しした場合にだけ解消する。

*引用: 本冊 §8.5*

### DES-1445

未確定の事実は `verify` / `report` の判断待ち section（§12.4）へ載せる。

### DES-1446

判断記録の受理は当該対象の検証状態（5状態）を昇格させない。

*導出元: SPEC-387, SPEC-388, SPEC-389, SPEC-390, SPEC-391, SPEC-392, SPEC-393, SPEC-394, SPEC-395, SPEC-396, SPEC-397, SPEC-398, SPEC-399, SPEC-400, SPEC-401, SPEC-402, SPEC-403, SPEC-404, SPEC-405, SPEC-406, SPEC-407, SPEC-408*

*引用: 本冊 §8.3・§3.4, 基本仕様 §11.3*

### DES-1447

判断記録は検査ゲートではなく、`UNKNOWN` に対する外部判断の追跡である。

### DES-1448

旧モデルの `verdict → CheckValue` 写像・reasons / basis 必須検査（E-AUDIT-005〜007）は撤去する。

*引用: 本冊 §8.4*

### DES-1449

判断記録（`.verify/decisions/` の actor / subject / decision / judgment_kind・理由 optional）と承認記録（`.verify/approvals/` の approver / subject または judgment_ref / approved_state、`vtest approval create` で生成）は別軸・別 entity である。

*引用: 本冊 §3.4, 本冊 §3.5*

### DES-1450

判断済み ≠ 承認済みである。

*導出元: SPEC-471, SPEC-472, SPEC-473, SPEC-474, SPEC-475, SPEC-476, SPEC-477, SPEC-478, SPEC-479, SPEC-480, SPEC-481, SPEC-482, SPEC-483, SPEC-484, SPEC-485, SPEC-486, SPEC-487, SPEC-488, SPEC-489, SPEC-490, SPEC-491, SPEC-492, SPEC-493, SPEC-494, SPEC-495, SPEC-496, SPEC-497*

*引用: 本冊 §8.5, 基本仕様 §17*

### DES-1451

判断は承認なしでも記録でき、正式採用は承認の別段階である。

### DES-1452

判断記録・承認記録のいずれも検証状態を昇格・降格させない。

### DES-1453

`vtest run` はテスト実行と Evidence 記録を行う。

> ```text
> vtest run (--test TEST-X | --vo VO-X | --all) [--fast]
> ```

*引用: 本冊 §9、§10*

### DES-1454

旧モデルの `--req`（REQ 指定）は document 層の総称化により廃止し、document scope が必要な場合は VO 部分木経由で指定する。

*引用: 本冊 §9.1*

### DES-1455

`--fast` は cargo test のみで、`target_coverage` を `checked: false` として記録する。

### DES-1456

`--fast` は `target_binding` の動的証拠を採らず、検証時 `NO_EVIDENCE`／診断 `NOT_CHECKED` とする。

*引用: 本冊 §10.3*

### DES-1457

`vtest verify` は集約を実行し、`OK` / `NG` を返す。

> ```text
> vtest verify [--items <check1,check2,...>]
>              [--doc DOC-X | --vo VO-X | --test TEST-X]
>              [--gate <name>] [--summary]
> ```

*引用: 本冊 §11.3*

### DES-1458

検査は基本仕様 §5 の固定4検査（`chain_integrity` / `orphan_detection` / `target_binding` / `oracle_presence`）のみである。

*導出元: SPEC-217, SPEC-218, SPEC-219, SPEC-220, SPEC-221, SPEC-222, SPEC-223, SPEC-224, SPEC-225, SPEC-226, SPEC-227, SPEC-228, SPEC-229, SPEC-230, SPEC-231, SPEC-232, SPEC-233, SPEC-234, SPEC-235, SPEC-236, SPEC-237, SPEC-238, SPEC-239, SPEC-240, SPEC-241, SPEC-242, SPEC-243, SPEC-244, SPEC-245, SPEC-246, SPEC-247, SPEC-248, SPEC-249, SPEC-250, SPEC-251, SPEC-252, SPEC-253, SPEC-254, SPEC-255, SPEC-256, SPEC-257, SPEC-258, SPEC-259, SPEC-260, SPEC-261, SPEC-262, SPEC-263, SPEC-264, SPEC-265, SPEC-266, SPEC-267, SPEC-268, SPEC-269, SPEC-270, SPEC-271, SPEC-272, SPEC-273, SPEC-274, SPEC-275, SPEC-276, SPEC-277, SPEC-278, SPEC-279, SPEC-280, SPEC-281, SPEC-282, SPEC-283, SPEC-284, SPEC-285, SPEC-286*

*引用: 基本仕様 §5*

### DES-1459

旧モデルの12項目（`spec_coverage` / `vo_decomposition` / `vo_coverage` / `test_existence` / `static_audit` / `semantic_audit` / `impl_consistency` / `test_execution` / `runtime_result` / `target_execution` / `evidence_validity` / `test_traceability`）は検査として存在しない。

*引用: 本冊 §11.1*

### DES-1460

scope は2軸であり、`--items` が検査軸（4検査の部分集合）、`--doc` / `--vo` / `--test` がエンティティ軸（部分木）である。

*導出元: SPEC-210, SPEC-211, SPEC-212, SPEC-213, SPEC-214, SPEC-215, SPEC-216*

*引用: 基本仕様 §4.6, 本冊 §11.3*

### DES-1461

旧モデルの `--spec` / `--req` は廃止し、`--req` は除去する。

### DES-1462

`--items` 省略時は常に固定4検査による完全検証を行う。

### DES-1463

`config.yaml` の `verify.full_scope` は本冊 §2.2 の invariant として事前に検証・正規化し、項目選択 knob として使用しない。

*引用: 本冊 §2.2*

### DES-1464

旧12項目の列挙は version を問わず E-CONFIG-001 とし、version 1 の field 欠落だけを固定4検査へ具体化する。

### DES-1465

`verify.full_scope` の in-memory の項目補完は行わない。

*引用: 本冊 §2.2*

### DES-1466

`--items` に4検査未満の明示的な集合を指定した場合だけ限定 scope とし、scope 外・未実施の検査は `NO_EVIDENCE`（診断 `NOT_CHECKED`）として保持し、`PASS` へ変換しない。

### DES-1467

限定 scope の結果を完全検証 OK と表示しない。

### DES-1468

いかなる設定値も完全検証を4本未満へ縮退させない。

*導出元: SPEC-210, SPEC-211, SPEC-212, SPEC-213, SPEC-214, SPEC-215, SPEC-216, SPEC-570, SPEC-571, SPEC-572, SPEC-573, SPEC-574, SPEC-575, SPEC-576*

*引用: 基本仕様 §4.6・§22.1*

### DES-1469

scope を限定した場合、出力冒頭に要求 scope と「scope 外は未検証」の旨を必ず表示する。

### DES-1470

`--format json` では同じ内容を最上位 field `scope`（§12.1）として返し、完全検証の場合も省略しない。

### DES-1471

`--gate <name>` はフェーズゲート評価（§12.3）である。

### DES-1472

config の `gates` に同名の定義が無ければ E-CONFIG-002・終了コード 2 で拒否し、検証を実行しない。

*引用: 本冊 §11.5・§17.1*

### DES-1473

`--summary` は総合 `OK` / `NG` と非 `PASS` 件数のみを出力する。

### DES-1474

`vtest verify` は状態列（`PASS` / `FAIL` / `MISMATCH` / `NO_EVIDENCE` / `UNKNOWN`）と診断ラベル列（`[MISSING]` / `[NOT_EXECUTED]` / `[NOT_CHECKED]` / `[STALE]`）を分離して表示する。

> 出力例（テキスト）：
>
> ```text
> Requested scope: full (4 checks), entity: DOC-BASIC-001 部分木
> （entity 軸で限定。scope 外エンティティは未検証）
>
> Structural checks:
> ├─ chain_integrity                      MISMATCH   [MISSING]        (leaf VO-PARSER-UTF8-004 に covers する Test なし)
> └─ orphan_detection                     PASS
>
> └─ DOC-BASIC-001                        NG
>    └─ VO-PARSER-UTF8                     NG
>       ├─ VO-PARSER-UTF8-003              NG
>       │  └─ TEST-PARSER-044              NG
>       │     ├─ target_binding            FAIL       [NOT_EXECUTED]  (evidence 01J8XW1B..., 2 targets: 1 PASS / 1 count 0)
>       │     └─ oracle_presence           FAIL                       (DA-006 空検証: src/parser.rs へ assert 相当なし)
>       └─ VO-PARSER-UTF8-004              MISMATCH   [MISSING]        (no covering test)
>
> Result: NG
> ```

### DES-1475

診断ラベルは代表値の順位（基本仕様 §22.2 の `FAIL > MISMATCH > NO_EVIDENCE > UNKNOWN`）に用いず、原因説明として併記する。

*導出元: SPEC-577, SPEC-578, SPEC-579, SPEC-580, SPEC-581, SPEC-582*

*引用: 基本仕様 §22.2*

### DES-1476

`target_binding` の非 `PASS` は根拠として用いた Evidence ID と当該 target の `target_coverage` 結果を引用する。

*引用: 本冊 §11.3*

### DES-1477

`oracle_presence` の非 `PASS` は違反した DA rule と根拠 span を引用する。

*引用: 本冊 §11.3*

### DES-1478

判断記録（§8）を引用する場合は decision ID を示す。

### DES-1479

静的解析は正典レコードを持たないため監査レコード ID は引用しない。

*引用: 本冊 §7.1*

### DES-1480

`chain_integrity` は repository-level の構造検査であり、`Structural checks` 配下に表示する。

### DES-1481

`chain_integrity` は発見された各 Test の管理宣言解決と Test ID 大局的一意性をすべて評価し、未登録または不正対応の各 Test について adapter ID・source location・diagnostic code・判定値を `chain_integrity` 配下に列挙する（`MISMATCH`／診断 `MISSING`）。

*引用: 本冊 §11.1.1*

### DES-1482

covers する Test の無い leaf VO も `chain_integrity = MISMATCH`（診断 `MISSING`）として entity tree 上に示す。

### DES-1483

JSON でも同じ根拠一覧を返す。

### DES-1484

Evidence が複数 target の計測結果を持つ場合、text report は Test 単位の集約値に加えて各 target の canonical Locator・result・count を子要素として表示する。

*引用: 本冊 §6.1.1*

### DES-1485

JSON は target 別 list を欠落なく返す。

*引用: 本冊 §3.6*

### DES-1486

各行の prefix は、その行の祖先に後続兄弟があれば `│  `、なければ空白3文字を階層ごとに連結し、現在 node が途中の兄弟なら `├─ `、最後の兄弟なら `└─ ` を付けて構成する。

### DES-1487

最上位 node にも同じ途中・末尾 branch 規則を適用する。

### DES-1488

祖先 node 自身の `├─ ` / `└─ ` を子孫行へ引き継がない。

### DES-1489

`vtest report` は `verify` と同じ集約を実行し、根拠（判断記録 ID・Evidence ID・DA rule 診断）を含む完全な詳細を出力する。

> ```text
> vtest report [--doc DOC-X | --vo VO-X | --test TEST-X]
>              [--items <check1,check2,...>] [--gate <name>]
>              [--from <node>] [--view pm|tester|coder] [--depth <n>]
>              [--direction up|down|both] [--format json]
> ```

### DES-1490

`verify` が判定用、`report` が閲覧・提出用という役割分担とする。

### DES-1491

`--from <node>` は任意ノード（DOC / VO / TEST / SRC）からの局所トレースの起点である。

*導出元: SPEC-520, SPEC-521, SPEC-522, SPEC-523, SPEC-524, SPEC-525, SPEC-526, SPEC-527, SPEC-528, SPEC-529, SPEC-530, SPEC-531, SPEC-532, SPEC-533, SPEC-534*

*引用: 本冊 §11.6, 基本仕様 §19*

### DES-1492

`--direction` は上流／下流／双方である。

### DES-1493

`--depth` は連続追跡の段数である。

### DES-1494

`--view` は役割 preset（`pm`＝上位 document・VO の状態と未確定/NG、`tester`＝VO・Test・検証対象・Evidence・未実施/失敗理由、`coder`＝実装から関連 Test・VO・上流 document へのトレース）である。

### DES-1495

役割を固定 enum として本冊は仕様化せず、preset・view 体系はここに委譲される。

*導出元: SPEC-740, SPEC-741, SPEC-742, SPEC-743, SPEC-744, SPEC-745, SPEC-746, SPEC-747, SPEC-748, SPEC-749, SPEC-750, SPEC-751, SPEC-752, SPEC-753, SPEC-754, SPEC-755, SPEC-756, SPEC-757, SPEC-758, SPEC-759, SPEC-760, SPEC-761, SPEC-762, SPEC-763, SPEC-764, SPEC-765*

*引用: 本冊 §11.6, 基本仕様 §30 item21*

### DES-1496

逆引きインデックス（VO → Tests、SRC → Tests、DOC → VOs、DOC → DOCs）を projection の基盤とする。

*引用: 本冊 §5.3*

### DES-1497

機能単位の集約は親 VO（子 VO を持つ VO）を単位とする。

*導出元: SPEC-577, SPEC-578, SPEC-579, SPEC-580, SPEC-581, SPEC-582*

*引用: 本冊 §11.3・§11.6, 基本仕様 §22.2*

### DES-1498

`--vo <親VO>` または `--from <親VO> --direction down` は、当該親 VO の代表値（fail-closed 合成）と、その配下の子 VO ごと・Test ごとの内訳を同じツリーに返す。

### DES-1499

Feature を別エンティティとして出力せず、Feature 名・Feature ID の field を設けない。

### DES-1500

束ねの識別子は親 VO の ID とする。

### DES-1501

`--format json` の trace 出力に含まれる `derives_from` エッジ（DOC → DOC、DOC → VO）は、`anchor` と `note` を同伴する。

*導出元: SPEC-379, SPEC-380, SPEC-381, SPEC-382, SPEC-383*

*引用: 本冊 §11.6・§3.1・§3.2, 基本仕様 §11.1*

### DES-1502

エッジ要素は `{ "from": "DOC-REQ-001", "relation": "derives_from", "anchor": "§12.3", "note": "", "to": "VO-PARSER-UTF8-003" }` の形とする。

### DES-1503

`anchor` を持たない entry では `anchor` を省略または `null` とし空文字列で埋めない。

### DES-1504

`report --from DOC-REQ-001 --direction down --format json` は、この形式で「どの上流条項がどの概念（VO）へ対応するか」の対応ペア集合を返す。

### DES-1505

`report --from DOC-REQ-001 --direction down --format json` が返す対応ペア集合が要求該当箇所と対応概念のペアの構造化出力であり、この用途に新規コマンド・ツールを設けない。

### DES-1506

`anchor` は不透明な文字列として transport し、文書内位置への解決・整合検査を行わない。

### DES-1507

`--format json` の出力へ、未確定・要判断事項を横断的に集約した `pending` section を含める（§12.4）。

*導出元: SPEC-515, SPEC-516*

*引用: 本冊 §11.7, 基本仕様 §18.3*

### DES-1508

`--gate <name>` は `verify` と同じ解決規則に従い、未定義名は E-CONFIG-002・終了コード 2 で拒否する。

*引用: 本冊 §11.5*

### DES-1509

`vtest mcp` は stdio で MCP サーバを起動する（§13）。

> ```text
> vtest mcp
> ```

### DES-1510

プロジェクト側が登録したフェーズ・工程・ゲートの進行条件について、現在の検証状態（5状態）と承認（§3.5）が通過条件を満たすかを評価・提示できなければならない（MUST）。

*導出元: REQ-302, REQ-303, REQ-304, REQ-305, REQ-306, REQ-307, REQ-308, SPEC-535, SPEC-536, SPEC-537, SPEC-538, SPEC-539, SPEC-540, SPEC-541, SPEC-542*

*引用: 本冊 §11.5, 基本仕様 §20, 要件定義 §26.4*

### DES-1511

本システムの責務はゲート条件が現在満たされているかの評価・提示に限り、フェーズのライフサイクル管理・工程の自動遷移は責務外とする（「Release フェーズへ遷移させる」ではなく「Release gate の条件を現在満たしている」を提示する）。

### DES-1512

`config.yaml` の `gates` に、ゲート名と進行条件（`require.verification`＝要求する検証結果、`require.approvals`＝要求する承認ロール集合）を保持する。

*引用: 本冊 §2.2*

### DES-1513

`require.verification` は 5 状態語彙（`PASS` / `FAIL` / `MISMATCH` / `NO_EVIDENCE` / `UNKNOWN`）のいずれかとの完全一致でなければならず、違反は config 受理時に E-CONFIG-001（終了コード 2）とする。

### DES-1514

`require.approvals` の省略は空集合として受理する。

*引用: 本冊 §2.2*

### DES-1515

`--gate <name>`（MCP は `gate` 入力）は `gates[].name` との大文字小文字を区別した完全一致でだけ解決する。

### DES-1516

未定義名・`gates` が空の状態での指定は E-CONFIG-002、`ok: false`、終了コード 2 とし、検証もゲート評価も実行せず、`data` に部分結果を返さない。

### DES-1517

診断 message には指定名と定義済みゲート名の一覧を含め、MCP では §13.1 の `candidates` に定義済みゲート名を入れる。

### DES-1518

検証条件は `require.verification` と要求 scope の集約代表値との完全一致でのみ充足する。

*引用: 本冊 §11.5*

### DES-1519

5 状態に順序を設けず、「要求値以上」の解釈を採らない。

### DES-1520

したがって `require.verification: PASS` は代表値 `PASS` のときだけ、`require.verification: UNKNOWN` は代表値 `UNKNOWN` のときだけ充足する。

### DES-1521

`--items` で検査軸を限定した実行では scope 外検査が `NO_EVIDENCE`（診断 `NOT_CHECKED`）として代表値に参加するため、`require.verification: PASS` のゲートは限定 scope では充足しない。

### DES-1522

承認ロールの解決は本別紙が新設する最小規則である。

*導出元: SPEC-471, SPEC-472, SPEC-473, SPEC-474, SPEC-475, SPEC-476, SPEC-477, SPEC-478, SPEC-479, SPEC-480, SPEC-481, SPEC-482, SPEC-483, SPEC-484, SPEC-485, SPEC-486, SPEC-487, SPEC-488, SPEC-489, SPEC-490, SPEC-491, SPEC-492, SPEC-493, SPEC-494, SPEC-495, SPEC-496, SPEC-497, SPEC-740, SPEC-741, SPEC-742, SPEC-743, SPEC-744, SPEC-745, SPEC-746, SPEC-747, SPEC-748, SPEC-749, SPEC-750, SPEC-751, SPEC-752, SPEC-753, SPEC-754, SPEC-755, SPEC-756, SPEC-757, SPEC-758, SPEC-759, SPEC-760, SPEC-761, SPEC-762, SPEC-763, SPEC-764, SPEC-765*

*引用: 基本仕様 §17・§30 item22*

### DES-1523

承認レコードは role field を持たないため、`config.yaml` に承認ロール → approver id 集合の対応を project 定義可能とする。

> ```yaml
> approval_roles:
>   reviewer: [reviewer-agent-01, alice]
>   owner:    [owner-human-01]
> ```

*引用: 本冊 §3.5*

### DES-1524

ロール `R` の承認が存在するとは、「本冊 §3.5 で有効な（subject_hash・依存 closure が現在一致する）対象の承認レコードのうち、`approver.id` が `approval_roles[R]` に属するものが1件以上存在する」ことをいう。

*引用: 本冊 §3.5*

### DES-1525

`gates.require.approvals` が参照するロールが `approval_roles` に無い場合は config invariant 違反として E-CONFIG-001 とする。

### DES-1526

ロール充足の判定対象は、当該 `verify` / `report` のエンティティ軸で指定した対象（`--doc` / `--vo` / `--test`。省略時は評価 scope の根エンティティ）に束縛された有効承認とする。

### DES-1527

scope 内に複数の対象がある場合は各対象について当該ロールの有効承認を要求する（fail-closed）。

### DES-1528

より細粒度の承認 authority・対象範囲はプロジェクト設定へ委譲する。

*導出元: SPEC-471, SPEC-472, SPEC-473, SPEC-474, SPEC-475, SPEC-476, SPEC-477, SPEC-478, SPEC-479, SPEC-480, SPEC-481, SPEC-482, SPEC-483, SPEC-484, SPEC-485, SPEC-486, SPEC-487, SPEC-488, SPEC-489, SPEC-490, SPEC-491, SPEC-492, SPEC-493, SPEC-494, SPEC-495, SPEC-496, SPEC-497, SPEC-740, SPEC-741, SPEC-742, SPEC-743, SPEC-744, SPEC-745, SPEC-746, SPEC-747, SPEC-748, SPEC-749, SPEC-750, SPEC-751, SPEC-752, SPEC-753, SPEC-754, SPEC-755, SPEC-756, SPEC-757, SPEC-758, SPEC-759, SPEC-760, SPEC-761, SPEC-762, SPEC-763, SPEC-764, SPEC-765*

*引用: 基本仕様 §17・§30 item22*

### DES-1529

`vtest verify --gate <name>` は、指定ゲートの対象 scope について検証を実行し、(1) 検証結果が `require.verification`（例 `PASS`）を満たすか、(2) `require.approvals` の各ロールについて上記解決規則で有効な承認が存在するか、を評価して満否と根拠（不足している非 `PASS` 検査・未充足の承認ロール）を提示する。

### DES-1530

`report --gate` は同評価を JSON の `gate` section で返す。

### DES-1531

検証状態と承認は独立の軸であり、承認未充足は検証状態を降格させない。

*導出元: SPEC-203, SPEC-204, SPEC-205, SPEC-206, SPEC-207, SPEC-208, SPEC-209*

*引用: 本冊 §3.5, 基本仕様 §4.5*

### DES-1532

`--gate` を指定した `verify` / `report` の JSON は `data.gate` を返す。

> ```json
> "gate": {
>   "name": "release",
>   "verification": { "required": "PASS", "actual": "MISMATCH", "satisfied": false },
>   "approvals": [
>     { "role": "reviewer", "satisfied": false, "missing_subjects": ["VO-PARSER-UTF8-004"] }
>   ],
>   "satisfied": false
> }
> ```

### DES-1533

`verification.required` は `require.verification` の値、`verification.actual` は要求 scope の集約代表値（5 状態のいずれか）、`verification.satisfied` は両者の完全一致である。

### DES-1534

`approvals[]` は `require.approvals` の各ロールについて充足有無と未充足の対象を返し、`require.approvals` が空集合なら空 list とする。

### DES-1535

`gate.satisfied` は `verification.satisfied` と全 `approvals[].satisfied` の論理積とする。

### DES-1536

text 出力では同じ 3 項目（要求値・現在の代表値・満否）と未充足ロール・不足している非 `PASS` 検査を提示する。

### DES-1537

`--gate` を指定した実行では最上位 `ok` と終了コードをゲート充足で決める（充足 → `ok: true`・0、不充足 → `ok: false`・1、未定義ゲート名 → `ok: false`・2）。

### DES-1538

要求 scope の総合 OK / NG は集約ツリーと `gate.verification.actual` から読み取る。

*引用: 本冊 §17.2*

### DES-1539

ゲート充足は検証状態とは別軸の評価であり、検証状態を書き換えない。

### DES-1540

JSON では検証状態（集約ツリーと `gate.verification.actual`）と `gate.satisfied` を別 field として常に併記し、text 出力でも検証状態の行とゲート満否の行を分けて表示する。

### DES-1541

`--gate` 指定時の `ok: true`・終了コード 0 を検証状態 `PASS` と読ませる表示（例：検証状態の行を省略する、`PASS` の語をゲート満否に流用する）はしない。

### DES-1542

具体的なフェーズ名・承認ロール・必要承認数・権限 schema はプロジェクト設定（`config.yaml`）へ委譲する。

*導出元: SPEC-740, SPEC-741, SPEC-742, SPEC-743, SPEC-744, SPEC-745, SPEC-746, SPEC-747, SPEC-748, SPEC-749, SPEC-750, SPEC-751, SPEC-752, SPEC-753, SPEC-754, SPEC-755, SPEC-756, SPEC-757, SPEC-758, SPEC-759, SPEC-760, SPEC-761, SPEC-762, SPEC-763, SPEC-764, SPEC-765*

*引用: 基本仕様 §30 items 22-23*

### DES-1543

導入時・検証時に生じる、未確定事項・不整合・未検証事項・機械的に確認済みの事実・外部判断が必要な事項を、機械可読な構造として `verify` / `report` の JSON 出力へ含める。

*導出元: REQ-228, REQ-229, SPEC-515, SPEC-516*

*引用: 本冊 §11.7, 基本仕様 §18.3, 要件定義 §17.3*

### DES-1544

新規コマンド・ツールを増やさず、既存出力の section として露出する。

### DES-1545

`subject` は対象エンティティ ID または解決済み canonical Locator である。

> ```json
> "pending": [
>   {
>     "subject": "TEST-PARSER-044",
>     "kind": "unknown",
>     "check": { "item": "oracle_presence", "state": "UNKNOWN", "diagnostic": [] },
>     "judgment_kind": null,
>     "basis": [ { "kind": "da-rule", "ref": "DA-003", "note": "クロージャ内到達のため確定不能" } ],
>     "bundle_ref": "cache/bundles/01J8XVYY.json"
>   },
>   {
>     "subject": "TEST-PARSER-044",
>     "kind": "unknown",
>     "check": null,
>     "judgment_kind": "case-coverage",
>     "basis": [ { "kind": "decision", "ref": "01J8XVZZ...", "note": "実効判断 deferred" } ],
>     "bundle_ref": null
>   }
> ]
> ```

### DES-1546

`kind` は `unknown`（`UNKNOWN` によるエスカレーション）/ `unregistered`（管理宣言欠落）/ `unresolved`（参照解決不能）/ `undecided`（VO 未確定）/ `pending_approval`（承認待ち）のいずれかである。

### DES-1547

`check` は関係する4検査のいずれかと現在の検証状態・診断ラベルである。

### DES-1548

4 検査のいずれにも由来しない項目（判断型に由来する項目・判断競合）では `check` を `null` とする。

### DES-1549

`check` が `null` の項目は集約へ寄与せず、いかなる検査の値も変更しない。

*引用: 本冊 §11.3*

### DES-1550

`judgment_kind` は外部判断が必要な場合の判断型（`test-semantic` / `impl-consistency` / `case-coverage`）である。

*引用: 本冊 §8.1*

### DES-1551

不要な項目では `judgment_kind` を `null` とする。

### DES-1552

`basis` は機械的に確認済みの事実（宣言鎖・検査結果・対象外とした範囲）への参照である。

### DES-1553

判断競合の項目では `basis` に競合した全判断記録 ID を `kind: decision` として列挙する。

### DES-1554

`bundle_ref` は外部判断が必要な場合の判断バンドル（§8.1）への参照であり、任意である。

### DES-1555

`UNKNOWN` だけでなく、検証出力全体にわたる未確定・要判断事項を横断的に集約する。

> `judgment_kind: case-coverage` の項目の生成条件、および判断競合の項目の生成条件は本冊 §11.7 に定める。

## DA-017 13. MCP ツール詳細仕様

### DES-1556

transport は stdio である。

### DES-1557

`rmcp` で実装する。

### DES-1558

各ツールの結果は CLI の `--format json` と同一の JSON 構造とする（検証状態 `state` と診断ラベル `diagnostic` の2軸を含む。§12.1）。

### DES-1559

エラーは MCP のツールエラーとして返し、`{ "code": "E-OP-001", "message": "...", "candidates": [...] }` の構造を含める。

### DES-1560

入力検証エラーには可能な限り `candidates` を含める。

*引用: 本冊 §6.3*

### DES-1561

各ツール呼び出しの冒頭で mtime ベースの再スキャン判定を行う。

*引用: 本冊 §2.3*

### DES-1562

`scan` は入力を取らず、診断一覧、エンティティ数サマリを出力する。

### DES-1563

`doc_list` / `doc_get` は `id`（get のみ）、`tree: bool`、`roots: bool` を入力とし、document レコード（木・根集合・鮮度）を出力する。

### DES-1564

`doc_upsert` は document フィールド一式（`path`、`derives_from[]`（`doc` + 任意 `anchor` + 任意 `note`）、`root: bool`、`update: bool`）を入力とし、作成・更新結果（依存判断・承認の失効警告を含む）を出力する。

### DES-1565

`approval_create` は `subject: { type: vo | document | judgment, id }`、`approver`、`state`（`approved` / `rejected` / `withdrawn`）、`basis[]`（任意）、`supersedes[]`（任意）を入力とし、承認レコード ID を出力する。

### DES-1566

`approval_create` は承認レコード生成の唯一の正典面である。

*引用: 本冊 §3.5*

### DES-1567

`approval_withdraw` は `approval_id`、`approver`、`basis[]`（任意）を入力とし、承認レコード ID を出力する。

### DES-1568

`approval_withdraw` は `approval_create` の `state: withdrawn` ＋ `supersedes: [approval_id]` と同一である。

### DES-1569

`approval_get` は `subject: { type, id }` を入力とし、承認レコード一覧（`approved_state` / `supersedes` / 有効性）と実効承認状態（`draft` / `approved`）を出力する。

### DES-1570

`vo_list` / `vo_get` は `id`、`doc`、`status` を入力とし、VO レコード、derives_from（`doc` + 任意 `anchor` + 任意 `note`）、covers 状況、承認状態を出力する。

### DES-1571

`vo_upsert` は VO フィールド一式（`derives_from[]` 必須1件以上（`doc` + 任意 `anchor` + 任意 `note`）、`dimensions[]`、`coverage_policy`、`combinations[]`（`explicit` のとき必須1件以上。各要素は dimension 名 → partition 値の map））を入力とし、作成・更新結果（承認失効の警告含む）を出力する。

### DES-1572

`vo_expand` は `id`、`dry_run: bool` を入力とし、生成される子 VO 一覧を出力する。

### DES-1573

`vo_approve` は `id`、`approver`、`state`（必須）、`basis[]`（任意）、`supersedes[]`（任意）を入力とし、承認レコード ID を出力する。

### DES-1574

`vo_approve` は `approval_create` に `subject: { type: vo, id }` を与えた場合の別名であり、独自の意味論を持たない。

### DES-1575

`test_query` は `vo` / `source` / `unregistered` のいずれかを入力とし、Test 一覧を出力する。

### DES-1576

`test_get` は `id` を入力とし、Test 詳細（intent、covers、targets、位置、判断記録・Evidence 状態）を出力する。

### DES-1577

`form_get` は大局的に一意な `kind` を入力とし、owner adapter を明示した Form Schema（§14）を出力する。

### DES-1578

`test_create` は `form`、`answers`（オブジェクト）、`dry_run` を入力とし、生成された Test ID、挿入位置、diff を出力する。

### DES-1579

`test_edit` は `id`、`answers` または `set`、`body`、`dry_run` を入力とし、更新結果、diff を出力する。

### DES-1580

`audit_static` は `test` または `all` を入力とし、rule 別 verdict（target-scoped な DA-002 / DA-003 は target 別 verdict を含む）と根拠 span を出力する。

*引用: 本冊 §3.6・§7.2*

### DES-1581

`audit_static` は正典レコードを生成しない。

*引用: 本冊 §7.1*

### DES-1582

`audit_bundle` は対象 ID（`test` / `vo`）、`kind`（`test-semantic` / `impl-consistency` / `case-coverage`。`test` では省略時 `test-semantic`、`vo` では `case-coverage` を必須）を入力とし、bundle_id と `judgment_kind` を含むバンドル本体（JSON）を出力する。

### DES-1583

`audit_submit` は提出 JSON（`judgment_kind` 必須、`supersedes[]` 任意）を入力とし、受理結果、判断記録 ID（`.verify/decisions/`）を出力する。

*引用: 本冊 §8.3*

### DES-1584

`audit_submit` の受理は検証状態を昇格させない。

### DES-1585

`run_tests` は `test` / `vo` / `all`、`fast: bool` を入力とし、Test ごとの結果と Evidence ID を出力する。

### DES-1586

`verify` は optional `items[]`（4検査の部分集合）、`doc` / `vo` / `test`、`gate`（任意）を入力とし（`items` 省略は固定4検査）、最上位 `scope`（§12.1）、総合 OK / NG、集約ツリー、`pending` section、`data.gate` 評価（指定時）を出力する。

### DES-1587

`report` は `verify` と同上の入力に `from` / `view` / `depth` / `direction` を加えて受け取り（`items` 省略は固定4検査）、最上位 `scope`（§12.1）、根拠付き完全レポート、projection（親 VO 起点の機能単位の束ねを含む）、`pending` section を出力する。

### DES-1588

`verify` / `report` の `gate` 入力は CLI の `--gate` と同じ解決規則に従い、config に定義の無いゲート名は E-CONFIG-002 の tool error（`candidates` に定義済みゲート名）とし、検証結果・部分結果を返さない。

*引用: 本冊 §11.5・§17.1*

### DES-1589

`gate` を指定した呼び出しの `ok` はゲート充足を表す（§12.3）。

### DES-1590

`doc_upsert` / `vo_upsert` の `derives_from[]` 各要素は `doc`（必須）、`anchor`（任意）、`note`（任意）からなる。

### DES-1591

`anchor` は参照先 document 内の該当箇所を指す不透明な文字列であり、省略・空文字列を許容し `chain_integrity` 違反にしない。

*引用: 本冊 §3.1・§3.2*

### DES-1592

`anchor` は CLI の `--anchor` と同じ値域・同じ扱いとし、文書内位置への解決・実在確認を行わない。

### DES-1593

`vo_upsert` の `combinations[]` は `combinations` を desired state として与える。

*引用: 本冊 §3.2.1*

### DES-1594

`combinations[]` の各要素は dimension 名 → partition 値の map（例 `{"operand-sign": "positive", "operator": "div"}`）で、`dimensions` に宣言された全軸をちょうど 1 回ずつ持つ。

### DES-1595

`coverage_policy` が `explicit` のときは `combinations[]` は 1 件以上を必須とし、`explicit` 以外のときは省略または空 list でなければならない。

### DES-1596

本冊 §3.2.1 の受理条件（`explicit` での欠落・空、`explicit` 以外での非空、未宣言 dimension、未列挙 partition、宣言 dimension の欠落・重複、重複 tuple、`dimensions` 空での `explicit`）に違反する入力は、`ok: false` と `{ "code": "E-SCAN-017", ... }` の tool error で拒否し、レコードを作成・更新しない。

*引用: 本冊 §3.2.1*

### DES-1597

`vo_upsert` で `combinations` を省略した更新は既存値を保持し、空 list を明示した更新は既存値を空にする。

### DES-1598

`vo_expand` は不正 `combinations` の VO に対して同じ E-SCAN-017 で拒否し、子 VO を 1 件も生成しない。

### DES-1599

`audit_static` は正典の監査レコード ID を返さない（再計算派生）。

*引用: 本冊 §7.1*

### DES-1600

`audit_submit` の受理結果は判断記録 ID であり、これは検証状態を変えない。

*引用: 本冊 §8.3*

### DES-1601

旧モデルの `spec_list` / `spec_get` / `req_list` / `req_get` / `req_upsert` は廃止し、`doc_*` へ統合した。

### DES-1602

各操作は、CLIとMCPで同じadapter registryを解決する。

### DES-1603

フォーム、監査、実行の入力に含まれる adapter namespace は opaque 値として扱い、未登録 adapter や未提供 capability を Rust 用の既定値へ暗黙変換しない。

### DES-1604

`audit_submit` は `UNKNOWN` に対する外部判断を記録するだけで、`oracle_presence` 等の検証状態を `PASS` へ昇格させない。

> ```text
> Coder AI がテストを追加する典型フロー：
>
> form_get(kind: rust-unit-function)
>   → test_create(answers, dry_run: true)   # 検証と diff 確認
>   → test_create(answers)                  # 挿入
>   → （関数本体を実装：test_edit の body、または直接編集）
>   → audit_static(test)                    # 決定論的な不成立検出（再計算派生）
>   → audit_bundle(kind: test-semantic, test)
>   → （エージェント自身が判定）
>   → audit_submit(result)                  # 判断記録に保存（検証状態は昇格しない）
>   → run_tests(test)
>   → verify(test)                          # 自タスクの完了確認
> ```

*導出元: SPEC-387, SPEC-388, SPEC-389, SPEC-390, SPEC-391, SPEC-392, SPEC-393, SPEC-394, SPEC-395, SPEC-396, SPEC-397, SPEC-398, SPEC-399, SPEC-400, SPEC-401, SPEC-402, SPEC-403, SPEC-404, SPEC-405, SPEC-406, SPEC-407, SPEC-408*

*引用: 本冊 §8, 基本仕様 §11.3*

### DES-1605

完了確認は `verify` の4検査で行う。

## DA-018 14. Form Schema 設計

### DES-1606

次は `rust-cargo` adapter が登録する Form Schema である。

> ```yaml
> kind: rust-unit-function
> adapter: rust-cargo
> title: Rust 関数単体テスト
> fields:
>   - name: target
>     question: 対象ソースシンボルは？
>     type: symbol            # symbol | vo-ref | vo-ref-list | test-ref |
>                             # enum | string | ident | path
>     required: true
>     validate: [symbol-exists]
>   - name: covers
>     question: どの VO を検証しますか？
>     type: vo-ref-list
>     required: true
>     validate: [vo-exists]
>   - name: behavior
>     question: どの振る舞いを検証しますか？
>     type: string
>     required: true
>   - name: test_kind
>     question: テスト種別は？
>     type: enum
>     options: [normal, error, boundary, regression]
>     required: true
>   - name: input
>     question: 入力条件は？
>     type: string
>     required: true
>   - name: expect
>     question: 期待結果は？
>     type: string
>     required: true
>     validate: [enum-variant-exists]   # best effort（本冊 §6.3）
>   - name: fn_name
>     question: テスト関数名は？
>     type: ident
>     required: true
>     validate: [unique-fn-name]
>   - name: file
>     question: 追加先ファイルは？（省略可）
>     type: path
>     required: false
>     validate: [rust-file]
> template: |
>   /// @vtest.id {test_id}
>   /// @vtest.covers {covers}
>   /// @vtest.target {target}
>   /// @vtest.intent {behavior}
>   /// @vtest.input {input}
>   /// @vtest.expect {expect}
>   /// @vtest.kind unit-{test_kind}
>   #[test]
>   fn {fn_name}() {
>       todo!("implement test body")
>   }
> ```

### DES-1607

core は `fn_name`、`.rs`、Rust 構文を Form Schema の共通 field として要求しない。

### DES-1608

`test_kind` の `regression` は Test の意図ラベル（`@vtest.kind` の値）であり、廃止された存在理由分類（role / anchor）とは別概念である。

*引用: 本冊 §4.1・§4.2*

### DES-1609

組込 Form は `role` を宣言しない。

### DES-1610

`kind` の値に regression を含む Test（`unit-regression` 等）も `kind` から存在理由分類を導出しない。

### DES-1611

`symbol-exists` 検証器は Target Reference 解決を対応 adapter へ委譲し、失敗時は E-OP-001＋候補を返す。

*引用: 本冊 §6.1*

### DES-1612

`vo-exists` / `test-exists` 検証器はエンティティ存在確認を行い、失敗時は E-OP-001 を返す。

### DES-1613

`enum-variant-exists` 検証器は `rust-cargo` adapter が `Type::Variant` 形式の場合のみ AST 検索し、解決不能な自由記述は受理し、失敗時は E-OP-001＋候補を返す。

### DES-1614

`unique-fn-name` 検証器は `rust-cargo` adapter が挿入先モジュール内で関数名重複を確認し、失敗時は E-OP-001 を返す。

### DES-1615

`rust-file` 検証器は `rust-cargo` adapter が `.rs` ファイルが scan 対象内に存在することを確認し、失敗時は E-OP-001 を返す。

### DES-1616

`required` を欠く回答、未知のフィールド名は E-OP-001 とする。

### DES-1617

Test ID は `--id` による明示指定がなければ、`TEST-<領域>-<連番>`（領域は covers 先 VO の ID から継承、連番は既存最大＋1）で自動採番し、結果に含めて返す。

### DES-1618

`kind` は `[a-z0-9][a-z0-9-]*` の case-sensitive 文字列で、`.verify/forms/<kind>.yaml` のファイル名と一致する repository-global な Form ID である。

### DES-1619

`adapter` はその Form を処理する Structured Test adapter ID である。

### DES-1620

registry は built-in と user-defined Form を統合し、同じ kind の重複、schema の adapter と registry owner の不一致、未知 adapter、Structured Test capability 欠落を拒否する。

### DES-1621

`adapter` を欠く読取り互換 Form は、登録済み adapter の built-in kind 宣言または schema を検査する compatibility matcher のうちちょうど1件だけが受理する場合に限って解決し、0件または複数件なら拒否する。

### DES-1622

matcher は schema 内容から決定論的に判定し、kind 名だけで Rust 用と推測しない。

### DES-1623

reader は互換解決だけで Form ファイルを書き換えない。

### DES-1624

組込フォームは `rust-cargo` adapter が提供する。

### DES-1625

コアはフォームの kind を Rust 固有と推測せず、schema の `adapter`、registry が宣言する大局的に一意な kind ownership、登録済み capability を照合する。

### DES-1626

未提供の Structured Test capability は E-ADAPTER-004 として作成・編集を中止し、ファイルを変更しない。

### DES-1627

`rust-unit-function`（§14.1）は組込 Form の1つである。

### DES-1628

`rust-integration` は組込 Form であり、単一の `target` field に代えて、1件以上のロケータを持つ `targets` を必須入力として受け取る。

### DES-1629

`rust-integration` は `file` を `required:true` とする。

### DES-1630

Integration Test の配置先（test suite location）は Source Target の location とは別概念であり、targets から一意に導出できないためである。

### DES-1631

将来、Test Suite または同等の配置概念が第一級化され配置先を一意に導出できる規則が導入された場合にのみ、省略可能性を再検討する。

### DES-1632

`rust-integration` の §14.1 との差分はこの2点であり、他は同一。

### DES-1633

`rust-integration` は `targets` の全要素を入力順に個別の `@vtest.target` 行として出力する。

### DES-1634

`rust-integration` は空 list と重複 target を E-OP-001 で拒否する。

### DES-1635

`target` キーは integration 種別に限り複数行を許容する。

*引用: 本冊 §4.2の例外*

### DES-1636

先頭以外の target を `@vtest.related` へ変換しない。

### DES-1637

Form Schema はユーザー定義可能とし、大局的に一意な `kind` と登録済み Structured Test adapter の `adapter` ID を必須とする。

*導出元: SPEC-455, SPEC-456, SPEC-457, SPEC-458, SPEC-459, SPEC-460, SPEC-461, SPEC-462, SPEC-463, SPEC-464, SPEC-465*

*引用: 本冊 §4.1・§5.2, 基本仕様 §15.4*

### DES-1638

`fields` の追加・変更で API Test・CLI Test 等の質問列を定義できる（要件定義の質問テンプレート構想に対応）。

### DES-1639

partition・境界値を必須入力とする種別は、該当フィールドに `required: true` を設定することで表現する。

*導出元: SPEC-455, SPEC-456, SPEC-457, SPEC-458, SPEC-459, SPEC-460, SPEC-461, SPEC-462, SPEC-463, SPEC-464, SPEC-465*

*引用: 基本仕様 §15.4*

### DES-1640

境界値・partition の必須入力化は組込 Form では設けず、user-defined Form Schema が指定できる。

### DES-1641

他 field の回答値によって `required` が変わる cross-field 制約は導入しない。

### DES-1642

Form Schema の検証は単一 field の `required` と検証器だけで閉じる。

### DES-1643

すべての管理対象 Test に `covers ≥ 1` を一律要求するため、user-defined Form も `covers` を `required: true` の `vo-ref-list` として持つ。

*導出元: SPEC-409, SPEC-410, SPEC-411, SPEC-412, SPEC-413, SPEC-414, SPEC-415, SPEC-416, SPEC-417, SPEC-418, SPEC-419, SPEC-420, SPEC-421, SPEC-422, SPEC-423, SPEC-424, SPEC-425*

*引用: 本冊 §4.1, 基本仕様 §12*

### DES-1644

本 version は role / anchor / anchor_rationale による存在理由分類・固定 Form 群を持たない。

### DES-1645

本 version は `covers` 件数の可変制約も設けない。

### DES-1646

user-defined Form も `kind` と owner `adapter` ID を宣言する通常の Form Schema であり、kind の大局的一意性と owner 解決の規則（§14.2）は変わらない。

*導出元: SPEC-455, SPEC-456, SPEC-457, SPEC-458, SPEC-459, SPEC-460, SPEC-461, SPEC-462, SPEC-463, SPEC-464, SPEC-465*

*引用: 基本仕様 §15.4*

## DA-019 15. Structured Test Operation adapter contract

### DES-1647

Structured Edit の構文解析・再生成・selector 解釈は対応 adapter が所有する。

### DES-1648

orchestration は Test ID と adapter ID で対象を一意に選択し、adapter が返す拡張範囲を単一置換として適用する。

### DES-1649

production adapter として提供するのは `rust-cargo` だけである。

### DES-1650

§15.1〜§15.4 は `rust-cargo` StructuredTestAdapter の構文処理を定める。

### DES-1651

Test ID から編集対象を特定する。

> ```text
> TEST-X → スキャン結果 → SourceLocation
>   （ファイル、関数アイテムの byte range、
>     doc comment 開始位置を含む拡張 range）
> ```

### DES-1652

スキャン結果が古い可能性があるため、編集直前に対象ファイルのみ再パースし、Test ID の位置を再確認する。

### DES-1653

再確認で見つからない場合は E-OP-002 とする。

### DES-1654

desired state（answers / set / body）から、あるべきアノテーションブロックと関数シグネチャ・本体を生成する。

### DES-1655

現状とあるべき状態の diff を計算する。

### DES-1656

変更を、対象テスト関数の拡張 range（doc comment 先頭〜関数末尾）の単一置換として適用する。

### DES-1657

適用後の対象ファイルの再パースは、構文的に妥当であることを確認する。

### DES-1658

適用後の対象ファイルの再パースは、対象 Test のアノテーションが desired state と一致することを確認する。

### DES-1659

適用後の対象ファイルの再パースは、他の Test エンティティのソーステキストが変化していないことを確認する。

### DES-1660

確認に失敗した場合はファイルを元へ戻し、E-OP-003 を返す。

### DES-1661

挿入後の再パース検証とロールバックは Edit と同一の規則で Create にも適用する。

### DES-1662

Create 経路にだけ検証を省く分岐を設けない。

*導出元: SPEC-440, SPEC-441, SPEC-442, SPEC-443*

*引用: 基本仕様 §15.1*

### DES-1663

Form 回答（§14）から、あるべきアノテーションブロックと関数シグネチャ・本体、および挿入位置を決定する。

### DES-1664

回答自体の検証エラーは E-OP-001（候補付き）とする。

*引用: 本冊 §6.3*

### DES-1665

挿入前の対象ファイルの内容を保持する。

### DES-1666

対象ファイルが存在しない場合は「不存在」を挿入前の状態として保持する。

### DES-1667

生成した Test construct を挿入位置へ単一挿入として適用する。

### DES-1668

適用後の対象ファイルの再パースは、構文的に妥当であることを確認する。

### DES-1669

適用後の対象ファイルの再パースは、挿入した Test construct がちょうど 1 件の Test エンティティとして認識されることを確認する。

### DES-1670

適用後の対象ファイルの再パースは、その Test のアノテーションが Form 回答から導いた desired state と一致し、Test ID が回答どおりであることを確認する。

### DES-1671

適用後の対象ファイルの再パースは、挿入した Test 以外の Test エンティティのソーステキストが変化していないことを確認する。

### DES-1672

適用後の対象ファイルの再パースは、挿入した Test 以外のソーステキスト（helper・fixture・通常コード）が変化していないことを確認する。

### DES-1673

確認のいずれかに失敗した場合は、適用前の状態へ復元し（挿入によりファイルが新規作成された場合は不存在へ戻す）、E-OP-003 を返す。

### DES-1674

ロールバック後は、当該操作より前と同じソーステキストが観測できなければならない。

### DES-1675

部分適用された挿入内容を残さない。

### DES-1676

`--dry-run` は、Form 回答（§14）から決定したあるべきアノテーションブロックと関数シグネチャ・本体、および挿入位置の結果のみを提示し、ファイルを変更しない。

### DES-1677

Create / Edit いずれも、E-OP-003 で中止した操作は Test ID の採番・Evidence・判断記録を含む副産物をひとつも残さない。

### DES-1678

ロールバック後の再スキャンで、当該操作が無かった場合と同一のエンティティ集合・内容ハッシュが得られる。

### DES-1679

アノテーションは常にキー順（id, covers, target, intent, input, expect, kind, case, related）で再生成する。

### DES-1680

`@vtest.` を含まない自由記述の doc comment 行は元の位置関係を保って温存する。

### DES-1681

アノテーションを常にキー順（id, covers, target, intent, input, expect, kind, case, related）で再生成し、`@vtest.` を含まない自由記述の doc comment 行を元の位置関係を保って温存することにより、Structured Edit を繰り返しても差分が安定する。

### DES-1682

アノテーションを常にキー順（id, covers, target, intent, input, expect, kind, case, related）で再生成する規則は、Create が挿入する annotation block にも同一に適用する。

### DES-1683

同一の desired state からは Create / Edit のいずれの経路でも同一の annotation block を生成し、Create 直後に同じ desired state で Edit しても差分を生じない。

### DES-1684

アノテーションの再生成キー順（id, covers, target, intent, input, expect, kind, case, related）は本冊 §4.2 の test-key（`id` / `covers` / `target` / `intent` / `input` / `expect` / `kind` / `case` / `related`）と一致する。

*引用: 本冊 §4.2*

### DES-1685

本 version は存在理由分類（旧 `role` / `anchor` / `anchor-rationale`）のキーを持たず、再生成でも出力しない。

### DES-1686

`@vtest.src-id` は Test construct ではなく対象実装側の関数に付与するキーである（本冊 §4.2 の source-target-key）。

*引用: 本冊 §4.2*

### DES-1687

`@vtest.src-id` は Test annotation block の再生成対象に含めない。

### DES-1688

置換範囲が単一のテスト関数の拡張 range に限られることを、適用前（範囲計算）と適用後（他 Test のハッシュ不変確認）の二重で検査する。

### DES-1689

`edit TEST-001` は他のTestへ影響しない。

*導出元: REQ-210, REQ-211, REQ-212, REQ-213, REQ-214, REQ-215, REQ-216, REQ-217, SPEC-446, SPEC-447, SPEC-448, SPEC-449, SPEC-450, SPEC-451, SPEC-452, SPEC-453, SPEC-454*

*引用: 要件定義 §16, 基本仕様 §15.3*

### DES-1690

helper・fixture・通常ソースコードの編集手段は提供しない。

*導出元: SPEC-446, SPEC-447, SPEC-448, SPEC-449, SPEC-450, SPEC-451, SPEC-452, SPEC-453, SPEC-454*

*引用: 要件定義 OOS-003, 基本仕様 §15.3*

### DES-1691

関数本体が helper を必要とする場合、helper の作成は通常のソース編集として利用者（人間・AI）が行う。

## DA-020 受入契約

### DES-1692

受入条件は決定論的なfixtureと統合テストで再現できる。

### DES-1693

Rust workspaceの受入テストは`cargo test --workspace`で実行できる。

### DES-1694

検証結果はfail-closedである。

### DES-1695

要求scopeに1件でも非PASSがあれば総合結果はNGになる。

### DES-1696

scopeを限定してもscope外の値をPASSへ変更しない。

### DES-1697

CLIとMCPは同じcore処理、adapter registry、JSON envelope、診断codeを使用する。

### DES-1698

canonical record、承認記録、判断記録、Evidence、内容hashの不変条件をfixtureの都合で緩和しない。

### DES-1699

Rustの受入fixtureは、総称document、VO、登録Test、Source Target、承認記録、判断記録、Evidenceを含む小規模projectとする。

### DES-1700

fixture は、正しい annotation を持つ Test を表現できる。

### DES-1701

fixture は、`assert!(true)` だけの Test を表現できる。

### DES-1702

fixture は、宣言 target を呼ばない Test を表現できる。

### DES-1703

fixture は、結果を検証しない Test を表現できる。

### DES-1704

fixture は、自己比較を行う Test を表現できる。

### DES-1705

fixture は、annotation を持たない test function（W-SCAN-101、`chain_integrity = MISMATCH`、診断`MISSING`）を表現できる。

### DES-1706

fixture は、`covers` を宣言しない Test（`covers` 0）を表現できる。

### DES-1707

すべての管理対象 Test に `covers ≥ 1` を一律要求するため、`covers` を宣言しない Test は E-SCAN-007 と `chain_integrity = MISMATCH`（診断`MISSING`）になる。

*導出元: SPEC-409, SPEC-410, SPEC-411, SPEC-412, SPEC-413, SPEC-414, SPEC-415, SPEC-416, SPEC-417, SPEC-418, SPEC-419, SPEC-420, SPEC-421, SPEC-422, SPEC-423, SPEC-424, SPEC-425*

*引用: 本冊 §11.1.1, 基本仕様 §12*

### DES-1708

fixture は、`rust-cargo` で `targets` を宣言しない Test（E-SCAN-007、`chain_integrity = MISMATCH`、診断`MISSING`）を表現できる。

> `targets ≥ 1`は`rust-cargo` adapterの必須metadata。

*引用: 本冊 §4.4・§5.5*

### DES-1709

fixture は、存在しない VO を参照する Test（E-SCAN-003、`chain_integrity = MISMATCH`）を表現できる。

### DES-1710

fixture は、Test ID が衝突する Test（E-SCAN-002、`chain_integrity = MISMATCH`）を表現できる。

### DES-1711

fixture は、Test construct と非隣接の metadata 宣言だけを変更した状態（Test subject hash が変化する）を表現できる。

### DES-1712

fixture は、Test / 宣言 target を変更せず、実行結果を変えうる target 外 helper または local dependency だけを変更した状態（Execution State subject が変化し Evidence が STALE 化）を表現できる。

### DES-1713

fixture は、`@vtest.case` を持つ table-driven Test を表現できる。

### DES-1714

fixture は、複数 target を宣言し、target ごとに PASS / FAIL / UNKNOWN が異なる integration Test を表現できる。

### DES-1715

fixture は、5 状態それぞれを生じる入力（`PASS` / `FAIL` / `MISMATCH` / `NO_EVIDENCE` / `UNKNOWN`）を表現できる。

### DES-1716

状態は5つのみとする。

*導出元: SPEC-174, SPEC-175, SPEC-176, SPEC-177, SPEC-178, SPEC-179, SPEC-180, SPEC-181, SPEC-182, SPEC-183, SPEC-184, SPEC-185, SPEC-186*

*引用: 基本仕様 §4.1*

### DES-1717

fixture は、4 診断ラベルそれぞれを生じる入力（`MISSING` / `NOT_EXECUTED` / `NOT_CHECKED` / `STALE`）を表現できる。

### DES-1718

診断ラベルは検証状態と別軸の原因説明である。

*導出元: SPEC-187, SPEC-188, SPEC-189, SPEC-190*

*引用: 基本仕様 §4.2*

### DES-1719

診断ラベルは状態値ではない。

*導出元: SPEC-187, SPEC-188, SPEC-189, SPEC-190*

*引用: 基本仕様 §4.2*

### DES-1720

fixture は、Test または target の hash 変更によって無効になる判断記録 / Evidence を表現できる。

### DES-1721

fixture は、複数 adapter が同じ恒久 SRC ID を宣言する状態（E-SCAN-011）を表現できる。

### DES-1722

fixture は、同一の Source Target を、一方の Test が locator で、他方の Test が恒久 SRC ID で宣言する状態を表現できる。

### DES-1723

fixture は、同一の Test が同一 Source Target を locator と恒久 SRC ID の両方で宣言する状態（E-SCAN-005）を表現できる。

### DES-1724

fixture は、Source Target construct の内側にある `@vtest.src-id` 宣言だけを付与・変更・削除した状態（construct bytes が変化し Source Target hash も変化する）を表現できる。

### DES-1725

fixture は、呼出を静的に確認できない到達境界を越えて target を実行する Test（subprocess spawn型・spawn thread型）を表現できる。

### DES-1726

DA-002 / DA-003がtarget別UNKNOWNになる。

### DES-1727

runtimeの`target_coverage`のみでDA-002到達が充足される。

### DES-1728

fixture は、他ファイル・他クレートへ呼び出すが戻り値を Test 本体内で assert する Test（DA-002 UNKNOWN・DA-003 PASS）を表現できる。

### DES-1729

fixture は、文書鎖の状態として `doc.roots` に列挙された根 document を表現できる。

### DES-1730

fixture は、文書鎖の状態として `derives_from` が空かつ根に列挙されない孤児 document（E-SCAN-016、`orphan_detection = MISMATCH`）を表現できる。

### DES-1731

fixture は、文書鎖の状態として `derives_from` の参照先が存在しない document / VO（E-SCAN-012、`chain_integrity = MISMATCH`）を表現できる。

### DES-1732

fixture は、文書鎖の状態として `content_hash` と実ファイルが一致しない document（W-SCAN-104、`chain_integrity = MISMATCH`、診断 `STALE`）を表現できる。

### DES-1733

fixture は、文書鎖の状態として document 再登録で失効する判断記録・承認記録を表現できる。

### DES-1734

fixture は、判断記録を受理しても対象の検証状態が昇格しない状態（判断受理前後で `UNKNOWN` が `PASS` へ変わらない）を表現できる。

### DES-1735

fixture は、上流依存 closure またはハッシュを欠く互換 Approval（W-STORE-002、VO は `draft` 相当）を表現できる。

### DES-1736

fixture は、フェーズゲート定義（`config.yaml` の `gates`）を持ち、`vtest verify --gate <name>` が条件充足・不足の両方を提示する状態を表現できる。

### DES-1737

adapter境界fixtureは、Rust parser、Cargo、llvm-covを使用しないin-process synthetic adapterを使用できる。

### DES-1738

synthetic adapterは配布対象のproduction language adapterではない。

### DES-1739

synthetic fixtureは`.rs`以外のsource、関数ではないTest construct、doc commentではないmetadata宣言、Rust item pathではないopaque locatorを使用する。

### DES-1740

source discovery adapterは全Discovered Test draft、ManagedTestDraftLink、SourceTargetDraft、Source Location、source range、current bytes、logical metadata、宣言された恒久SRC ID、Test execution descriptorをhash未計算で返す。

### DES-1741

coreは出力を検証してTest subject / Source Target hashを計算してからManaged Test Entity、ManagedTestLink、Source Targetを具体化する。

### DES-1742

Source Targetはcanonical locatorと任意の恒久SRC IDを併有する単一のentityである。

### DES-1743

adapterは同一constructをlocator版とSrcId版の2 draftへ複製せず、恒久SRC IDを`SourceTargetDraft.src_id`として返す。

### DES-1744

恒久SRC IDを持つSource Targetはcanonical locatorでもaddressableであり、locator参照とSRC ID参照は同一のcanonical Source Targetへ解決する。

### DES-1745

両addressing modeで同一のSource Target hashに到達し、Source Targetの件数、content / subject hash、Evidenceおよび判断記録上のtarget identityが参照方法によって分裂しない。

### DES-1746

Source Target identityは「宣言された`TargetRef` → 解決 → canonical Locator」の一方向で確定する。

### DES-1747

Evidence、判断記録、`target_binding` の証拠、鮮度判定は解決後のcanonical Locatorをidentityとして記録・比較し、参照側Testが宣言した綴り（SRC ID参照を含む）を保存しない。

### DES-1748

同一のSource Targetをlocator参照するTestとSRC ID参照するTestは、Evidence上で同一のtarget identityを持つ。

### DES-1749

Testがどう宣言したかの変更（同一Source Targetに対するlocator参照からSRC ID参照への書き換え等）はTest subject hashの変化として捕捉され、Evidence側のtarget identityを変化させない。

### DES-1750

綴りの異なる複数の`target`宣言が同一のcanonical Source Targetへ解決する場合はE-SCAN-005とする。

### DES-1751

`SourceTargetDraft.target`は必ず`TargetRef::Locator`である。

### DES-1752

`TargetRef::SrcId`をcanonical targetとして返したadapter出力はmalformed adapter outputとして拒否する。

### DES-1753

恒久SRC IDの宣言・変更・削除でcanonical locatorは変化しない。

### DES-1754

Source Target hashは常にcanonical locatorとconstruct bytesから計算し、参照側Testの`TargetRef`綴りからは計算しない。

### DES-1755

恒久SRC IDを独立したhash fieldとしてSource Target hashのinputに含めない。

### DES-1756

恒久SRC IDの宣言をSource Target constructの内側へ置くadapter（`rust-cargo`の`@vtest.src-id` doc comment等）では、その宣言の付与・変更・削除がconstruct bytesを変えるため、Source Target hashも変化する。

### DES-1757

Source Target hashも変化することはsourceが実際に変化したことの帰結として正しい挙動であり、恒久SRC IDが独立したhash fieldであることを意味しない。

### DES-1758

SRC ID参照はcoreの統合済みSRC索引から、その恒久SRC IDを宣言したSource Targetのcanonical locatorへ解決する。

### DES-1759

Target Reference解決は解決済み / 対象なし / 曖昧を区別し、曖昧はfail-closedな終端状態とする。

### DES-1760

E-SCAN-004またはE-SCAN-011で曖昧・未解決となったtargetについて、判断記録subject、Evidence、`target_binding` の証拠のいずれも候補の1件を解決結果として記録しない。

### DES-1761

候補は診断表示にだけ用いる。

### DES-1762

判断記録subject、Evidence、`target_binding` の証拠のいずれも候補の1件を解決結果として記録しないという禁止は解決に関するものであり、Source Targetの具体化を止めない。

### DES-1763

恒久SRC IDが衝突していても、各Source Targetは自身のcanonical locatorで独立したentityとして具体化され、Source Targetの件数と各content / subject hashは衝突の有無で変化しない。

### DES-1764

衝突が壊すのは当該恒久SRC IDによる参照の一意性だけである。

### DES-1765

Target Reference解決はcoreの単一経路が所有する。

### DES-1766

discovery、静的解析、実行、Evidence writer、検証集約が独自にcandidate列を走査して1件を選ぶ経路を持たない。

### DES-1767

adapter所有のmetadata宣言、ID、target、VO参照、record schema、Relationの違反を対応診断codeで検出する。

### DES-1768

Test 層は、管理宣言または必須metadata（core 中立の Test ID・`covers ≥ 1`・`intent`、および当該 adapter が必須とする追加 metadata〔`rust-cargo` では `targets ≥ 1`〕）を持たないTestが1件でもあれば、W-SCAN-101またはE-SCAN-007を表示し、`ManagedTestLink::Missing`から`chain_integrity = MISMATCH`（診断 `MISSING`）を導出する。

*導出元: SPEC-224, SPEC-225, SPEC-226, SPEC-227, SPEC-228, SPEC-229, SPEC-230, SPEC-231, SPEC-232, SPEC-233, SPEC-234*

*引用: 本冊 §11.1.1, 基本仕様 §5.1*

### DES-1769

存在しないVOを`covers`するTestは構造上完全なManaged Test Entityと`ManagedTestLink::One`のまま保持し、E-SCAN-003と`chain_integrity = MISMATCH`を導出する。

### DES-1770

診断ラベルを二重定義しない。

### DES-1771

`ManagedTestLink::Multiple`またはTest ID衝突（E-SCAN-002）は`chain_integrity = MISMATCH`になる。

### DES-1772

`covers` を持たない（0 件の）Testは管理宣言不整合として`chain_integrity = MISMATCH`（診断 `MISSING`）になる。

### DES-1773

役割による`covers`可変制約・特別扱いの分岐を設けず、すべての管理対象 Test に`covers ≥ 1`を一律要求する。

*導出元: SPEC-409, SPEC-410, SPEC-411, SPEC-412, SPEC-413, SPEC-414, SPEC-415, SPEC-416, SPEC-417, SPEC-418, SPEC-419, SPEC-420, SPEC-421, SPEC-422, SPEC-423, SPEC-424, SPEC-425*

*引用: 基本仕様 §12*

### DES-1774

既定を緩和して0件を受理しない。

### DES-1775

全Discovered Testが`ManagedTestLink::One`で構造上完全なentityへ1対1で対応し、Test IDが一意、各entityが`covers ≥ 1`を満たし、かつ全VO参照を解決できる場合だけTest層の`chain_integrity`が成立する。

### DES-1776

各 VO は 1 件以上の `document` への解決可能な `derives_from` を持つ。

### DES-1777

参照先 document が存在しない、または解決不能な場合は E-SCAN-012、`chain_integrity = MISMATCH`。

### DES-1778

VO parent の不在・循環は E-SCAN-008、`chain_integrity = MISMATCH`。

### DES-1779

各`document`の`derives_from`参照先が存在することを要求する（不在はE-SCAN-012、`chain_integrity = MISMATCH`）。

### DES-1780

各`document`の`content_hash`が実ファイル（`path`）と一致することを要求する（不一致はW-SCAN-104、`chain_integrity = MISMATCH`、診断`STALE`）。

### DES-1781

document 種別を区別せず、要件定義・基本仕様・詳細設計・API Schema 等をすべて総称 document として同一に扱う。

*引用: 本冊 §3.1*

### DES-1782

`covers` する Test が 1 件以上存在しない leaf VO は `chain_integrity = MISMATCH`（診断 `MISSING`）。

### DES-1783

発見された Test → 管理宣言の解決と、leaf VO → Test の両方向が成立して初めて `chain_integrity` が成立する。

### DES-1784

W-SCAN-101のwarning severityだけを理由に検証値を変更せず、Discovered Testとmanaged entityの対応事実から判定する。

### DES-1785

adapter discoveryの失敗をTest 0件の正常scanとして扱わない。

### DES-1786

解析不能・不完全なbatchは対応する検証を`UNKNOWN`とする。

### DES-1787

Relation writerは`REL-<ULID>`だけを生成する。

### DES-1788

readerはファイル名とrecord IDが同じbare ULIDのversion 1互換Relationを読み取り、in-memoryで`REL-<ULID>`へ正規化するが、ファイルを書き換えない。

### DES-1789

同じpayloadのbare / prefixed重複、混在形、ファイル名とIDの不一致はE-SCAN-010になる。

### DES-1790

Relation の from / to 不在は E-SCAN-009、`chain_integrity = MISMATCH`。

### DES-1791

VO writerは`status`を保存せず、実効値をApprovalから導出する。

### DES-1792

読取り互換field `status`は警告（W-STORE-001）して無視する。

### DES-1793

VOの承認はVO内容hashと現在の上流依存closureへ束縛され、`document` / parent VO の内容または集合が不一致の承認を有効として扱わない。

### DES-1794

Approval作成時に対象または上流依存closureを完全・currentに解決できなければE-APPROVAL-001で拒否し、recordを生成しない。

### DES-1795

上流依存closureまたはハッシュを欠く互換Approvalを現在のapprovedへ昇格しない（W-STORE-002、VOは`draft`相当）。

### DES-1796

恒久SRC IDは全adapter統合後にrepository全体で一意である。

### DES-1797

恒久SRC IDの衝突をE-SCAN-011として拒否する。

### DES-1798

`vtest scan` / `doctor`はE-ADAPTER-* / E-CONFIG-*による操作拒否をexit 2にする。

### DES-1799

`vtest scan` / `doctor`は完了したscanのE-SCAN-*をexit 1にする。

### DES-1800

`vtest scan` / `doctor`はerrorなしをexit 0にする。

### DES-1801

`full-product` VOは宣言partitionの直積を決定論的に実体化する。

### DES-1802

`coverage_policy: explicit`と妥当な`combinations`を持つVOは、列挙されたtupleごとにちょうど1件の子VOを生成する。

*引用: 本冊 §3.2.1・§17.1*

### DES-1803

子VO IDのsuffixは`dimensions`の宣言順で連結される。

### DES-1804

同じtuple集合を記述順・map key順を変えて与えても、生成される子VO集合とIDは同一になる。

### DES-1805

`explicit`かつ`combinations`欠落を持つVOレコードはE-SCAN-017と`chain_integrity = MISMATCH`になり、`vo expand`は子VOを1件も生成しない（部分生成しない）。

### DES-1806

`explicit`かつ`combinations`空listを持つVOレコードはE-SCAN-017と`chain_integrity = MISMATCH`になり、`vo expand`は子VOを1件も生成しない（部分生成しない）。

### DES-1807

`explicit`かつ`dimensions`空を持つVOレコードはE-SCAN-017と`chain_integrity = MISMATCH`になり、`vo expand`は子VOを1件も生成しない（部分生成しない）。

### DES-1808

`independent-axes` / `full-product` / `null`かつ`combinations`非空を持つVOレコードはE-SCAN-017と`chain_integrity = MISMATCH`になり、`vo expand`は子VOを1件も生成しない（部分生成しない）。

### DES-1809

未宣言dimension名を含むtupleを持つVOレコードはE-SCAN-017と`chain_integrity = MISMATCH`になり、`vo expand`は子VOを1件も生成しない（部分生成しない）。

### DES-1810

当該dimensionの`partitions`に無いpartition値を含むtupleを持つVOレコードはE-SCAN-017と`chain_integrity = MISMATCH`になり、`vo expand`は子VOを1件も生成しない（部分生成しない）。

### DES-1811

宣言済みdimensionを欠くtupleを持つVOレコードはE-SCAN-017と`chain_integrity = MISMATCH`になり、`vo expand`は子VOを1件も生成しない（部分生成しない）。

### DES-1812

同一dimension名を2回持つtupleを持つVOレコードはE-SCAN-017と`chain_integrity = MISMATCH`になり、`vo expand`は子VOを1件も生成しない（部分生成しない）。

### DES-1813

重複tupleを持つVOレコードはE-SCAN-017と`chain_integrity = MISMATCH`になり、`vo expand`は子VOを1件も生成しない（部分生成しない）。

### DES-1814

`vo add` / `vo edit` / MCP `vo_upsert` は上記の各入力を受理時に E-SCAN-017・終了コード 2 で拒否し、レコードを作成・更新しない（拒否後に scan したエンティティ集合は操作前と同一）。

### DES-1815

`vo edit --combination` は desired state として既存 `combinations` を置換し、追記しない。

### DES-1816

`--clear-combinations` は空にする。

### DES-1817

`--combination`と`--clear-combinations`のどちらも与えない `edit` は既存 `combinations` を保持する。

### DES-1818

`combinations` だけを変更した `edit` は VO subject hash を変化させ、当該 VO の承認を失効させる。

### DES-1819

document / VO の `derives_from` entry に `anchor` を持つ状態と持たない状態の双方を読み取り、いずれも `chain_integrity` に影響しない（`anchor` の欠落・空文字列で `MISMATCH` にならない）。

*引用: 本冊 §3.1・§3.2*

### DES-1820

`anchor` の値を文書内位置へ解決せず、実在しない節番号を書いても診断を出さない。

### DES-1821

同一 `doc` を指す複数 `derives_from` entry を `anchor` 違いで保持でき、重複として拒否しない。

### DES-1822

`anchor`だけを変更したdocumentは`content_hash`（`path`の実ファイルのハッシュ）が不変のままdocument subject hashが変化する。

### DES-1823

`anchor`だけを変更したdocumentは、当該documentを上流依存closureに含む承認・判断記録を失効させる。

### DES-1824

`anchor` だけを変更した VO は VO subject hash が変化せず、当該 VO の承認が失効しない。

### DES-1825

CLI で `--derives-from` を伴わない `--anchor`、または 1 つの `--derives-from` に 2 個目の `--anchor` を与えた場合は終了コード 2 で拒否し、レコードを書かない。

### DES-1826

既存ソース・既存テストを含む fixture project で `vtest init` を実行した前後で、`.verify/` を除いた作業ツリーの全ファイルのバイト列が同一である。

*導出元: SPEC-500, SPEC-501, SPEC-502, SPEC-503, SPEC-504, SPEC-505, SPEC-506, SPEC-507, SPEC-508, SPEC-509*

*引用: 別紙A §12.2, 基本仕様 §18.1*

### DES-1827

`.verify/` 外のファイルの新規作成・変更・削除が 1 件も観測されない。

### DES-1828

`init` は既存ソースへ Test metadata 宣言（`@vtest.` 行）・annotation・doc comment を挿入しない。

### DES-1829

既存 `.verify/` があるプロジェクトでの `init` は終了コード 2 で中止し、その実行でファイル・ディレクトリを 1 件も作成・変更・削除しない（既存 `.verify/` の内容も不変）。

### DES-1830

`orphan_detection` は文書層のみを対象とし、親（上流 document）を持たない `document` ノードの有無を問う。

*導出元: REQ-059, REQ-060, REQ-061, REQ-062, REQ-063, SPEC-235, SPEC-236, SPEC-237, SPEC-238, SPEC-239, SPEC-240, SPEC-241*

*引用: 本冊 §5.6, 基本仕様 §5.2, 要件定義 §4.2*

### DES-1831

実装レイヤーの孤児検出（宣言されていない実装の検出）は行わない。

*導出元: R-2, REQ-292, SPEC-729, SPEC-730, SPEC-731, SPEC-732, SPEC-733, SPEC-734, SPEC-735, SPEC-736, SPEC-737, SPEC-738, SPEC-739*

*引用: 要件定義 R-2, 基本仕様 §29 OOS-005*

### DES-1832

根の除外は、`config.yaml` の `doc.roots` に列挙された DOC ID を根として扱い、`orphan_detection` の対象外とする。

### DES-1833

根指定の追加・削除は `vtest doc` コマンドの引数で管理する。

*導出元: SPEC-665, SPEC-666, SPEC-667, SPEC-668, SPEC-669, SPEC-670, SPEC-671, SPEC-672, SPEC-673, SPEC-674, SPEC-675, SPEC-676, SPEC-677, SPEC-678, SPEC-679, SPEC-680, SPEC-681, SPEC-682, SPEC-683, SPEC-684, SPEC-685, SPEC-686, SPEC-687, SPEC-688, SPEC-689*

*引用: 基本仕様 §26.1*

### DES-1834

孤児判定は、`derives_from` が空、かつ他のどの document からも `derives_from` で参照されず、`doc.roots` にも列挙されない document を孤児とし、E-SCAN-016、`orphan_detection = MISMATCH` になる。

### DES-1835

`doc.roots` が存在しない DOC ID を参照する場合は config invariant 違反として E-CONFIG-001 とする。

### DES-1836

旧モデルの W-SCAN-102（孤立 VO）は VO 層の警告であり、文書層 `orphan_detection` とは別物として存置する。

### DES-1837

DA-001〜DA-006とW-DA-101は本冊§7の判定条件に従う。

*引用: 本冊 §7*

### DES-1838

静的解析は正典レコードを持たない再計算派生であり、検証のたびに現在のsource / configから再計算する。

*導出元: P-003*

*引用: 本冊 §7.1, 基本仕様 P-003*

### DES-1839

確定違反だけをFAILとし、解析限界をUNKNOWNとして保持する。

### DES-1840

正常Testは違反なしとなり、各違反fixtureは対応ruleで非PASSになる。

### DES-1841

`oracle_presence` は DA-001 / DA-003 / DA-004 / DA-005 / DA-006 の合成とし、全ルール違反なしで `PASS` になる。

*導出元: SPEC-263, SPEC-264, SPEC-265, SPEC-266, SPEC-267, SPEC-268, SPEC-269, SPEC-270, SPEC-271, SPEC-272, SPEC-273, SPEC-274*

*引用: 本冊 §7.1, 基本仕様 §5.4*

### DES-1842

1つでも `FAIL` があれば `oracle_presence` は `FAIL` になる。

### DES-1843

`FAIL` がなく `UNKNOWN` があれば `oracle_presence` は `UNKNOWN` になる。

### DES-1844

`oracle_presence` に動的な昇格経路は無く、runtime 証拠で `PASS` へ昇格しない。

### DES-1845

Test の成否判定が assert 相当の構文でなく通常の関数へ委譲されている場合において、委譲先を宣言targetとするTestが存在し、その`oracle_presence`がすべて`PASS`であるTestは、DA-003 / DA-006が違反なしとなる。

*引用: 本冊 §7.2.1*

### DES-1846

委譲先のassert相当が委譲先側にしか無いことだけを理由に`FAIL`としない。

### DES-1847

委譲先を宣言targetとするTestが0件のTestは、DA-003 / DA-006が`UNKNOWN`となる。

### DES-1848

常に真を返す照合ヘルパを呼ぶだけのTestが`oracle_presence` = `PASS`にならない。

### DES-1849

委譲先を宣言targetとするTestは存在するが、その`oracle_presence`が`PASS`でないTestは、DA-003 / DA-006が`UNKNOWN`となる。

### DES-1850

委譲先の終端が循環する（相互に照合を委譲し合う）2 Testは、いずれもDA-003 / DA-006が`UNKNOWN`となり、評価順序を変えても同じ値になる。

### DES-1851

委譲先が他ファイル・他クレート・マクロ展開内で同定できないTestは、DA-003 / DA-006が`UNKNOWN`となる。

### DES-1852

信頼を宣言する専用の注釈・設定項目・レコードを新設せず、covers / 宣言targetのグラフだけで上記の各値が決まる。

### DES-1853

DA-002の target別verdictが`UNKNOWN`のとき、当該targetのruntime計測（§18.3.5）が実行を証明した場合に限り到達要件が充足される。

*引用: 本冊 §7.3*

### DES-1854

DA-002の target別verdictが`UNKNOWN`のとき、当該targetのruntime計測（§18.3.5）が実行を証明した場合に限り到達要件が充足されるというruntime救済は`target_binding`に固有であり、`oracle_presence`には及ばない。

### DES-1855

static audit adapterが判定へ使用したsource fragment集合の完全性を保証できない場合、当該判定はUNKNOWNとなりPASSにならない。

### DES-1856

別プロセス・別スレッド・クロージャ・他ファイル等、静的解析の到達境界を越えてtargetを実行するTestは、当該targetのtarget別DA-002 verdictがUNKNOWNになる。

*引用: 本冊 §7.3*

### DES-1857

当該targetのruntime`target_coverage`がPASS（checked: true・count > 0）ならDA-002到達要件は充足され、検証時にそのtarget別DA-002はUNKNOWN扱いにならない。

### DES-1858

呼出自体を静的に確認できないtarget（subprocess spawn等）は、DA-002だけでなくDA-003のtarget別verdictもUNKNOWNになる（空虚PASS / FAILとしない）。

*引用: 本冊 §7.3*

### DES-1859

呼出自体を静的に確認できないtargetについて、DA-003はruntimeで救済されない。

### DES-1860

したがってexit code / stdoutだけをassertするsubprocess E2Eは、当該targetのDA-002がruntimeで充足されて`target_binding = PASS`に到達しうる一方で、DA-003がUNKNOWNのまま残り`oracle_presence = UNKNOWN`となる。

### DES-1861

DA-002とDA-003の2検査が別々の値をとる場合が新モデルの識別fixtureであり、総合判定はNGになる。

### DES-1862

他ファイル・他クレートへ呼び出すが戻り値をTest本体内でassertするTestは、DA-002 UNKNOWN・DA-003 PASSとなり、runtime`target_coverage`がPASSでかつ他ルールも違反なしなら`target_binding`は到達充足、`oracle_presence = PASS`になる（runtime救済で実益が出るのはこの型）。

### DES-1863

複数targetを宣言するTestで、target Aは静的（DA-002 = PASS）、target Bはruntime（Bのtarget別`target_coverage` = PASS）でDA-002到達を充足する場合、BもTest本体内で結果をassertしDA-003 = PASSなら`oracle_presence = PASS`かつBの`target_binding`到達も充足する。

### DES-1864

Bが呼出不可視（subprocess）でDA-003 UNKNOWNなら`oracle_presence = UNKNOWN`となる。

### DES-1865

到達判定はtarget別に行い、AとBのstatic verdictを取り違えない。

### DES-1866

DA-002 verdict = FAIL（解析境界内で到達を静的に否定）は runtime 証明で覆らない。

### DES-1867

runtime証明に依存する`target_binding`の値は、§18.3.4の鮮度判定が選択した最新Evidenceが鮮度を満たすときだけ用い、無効な最新Evidenceから古い有効Evidenceへフォールバックしない。

*引用: 本冊 §11.2*

### DES-1868

無効な最新Evidenceから古い有効Evidenceへフォールバックしないことにより同一検証内で計測がSTALEの一方`target_binding`が別Evidenceで PASSになる履歴不一致を生じない。

### DES-1869

表示scopeと内部依存評価を分離する。

### DES-1870

`vtest verify --items oracle_presence` / `--items target_binding` のような限定scopeでも、aggregatorは本冊§7.3のruntime到達判定に必要なEvidence鮮度・target別`target_coverage`を内部依存として評価するが、scope外の項目自体のreport valueは`NO_EVIDENCE`（診断`NOT_CHECKED`）のまま保持する。

*引用: 本冊 §7.3*

### DES-1871

同じ到達UNKNOWNのTestでも、当該targetの`target_coverage`がFAIL・UNKNOWN・NOT_CHECKED（coverage利用不能・未計測・`--fast`）なら到達要件は未充足で、当該targetのDA-002 UNKNOWNは`target_binding`の非PASS要因として残る。

### DES-1872

runtime coverageはDA-003を代替しない。

### DES-1873

結果検証はDA-003の静的判定（結果がassert相当へ到達）のまま評価し、到達がruntimeで充足されてもDA-003 UNKNOWN / FAILはそのまま`oracle_presence`へ寄与する。

### DES-1874

宣言targetをどのtopologyでも実行しない構造・契約のみのTestは、静的にもruntimeにも到達を確立できず`target_binding`の到達要件は未充足のままになる。

### DES-1875

選択した登録Testだけをrunnerのexact selectorで実行する。

### DES-1876

Testごとの結果、revision、hash、adapter ID、runner情報、およびExecution State subjectをEvidenceへ記録する。

### DES-1877

build failure、runner failure、必須runner capabilityの欠落、および宣言targetの解決失敗ではEvidenceを生成しない。

### DES-1878

実行前後でExecution State subjectが変化した場合はE-EXEC-004となり、Evidenceを生成しない。

### DES-1879

Evidence writerはadapter IDを必ず記録する。

### DES-1880

Evidence writerは中立fieldの`hashes.test_subject`と`hashes.targets[].target_construct`を出力する。

### DES-1881

`test_fn` / `test_construct` / `target_fn`の互換入力は`rust-cargo` Evidenceで全canonical metadataを含むsource rangeと現在値の同一性を証明できる場合だけ受理する。

### DES-1882

Evidence readerはadapter IDを欠くrecordについて、現在のTestが `rust-cargo` で、runner kindと内容hashからRust実行を一意に確認できる場合だけ互換Evidenceとして扱う。

### DES-1883

Evidenceは全宣言targetを解決したcanonical Locatorと内容hashを重複なく保持し、参照側Testが宣言した`TargetRef`の綴り（SRC ID参照を含む）をtarget identityとして保存しない。

### DES-1884

同一Source Targetをlocator参照するTestとSRC ID参照するTestのEvidenceは、同じtarget identityと同じtarget内容hashを持つ。

### DES-1885

全宣言targetがcanonical Source Targetへ一意に解決できることをEvidence生成のpreconditionとする。

### DES-1886

1件でも対象なしまたは曖昧ならEvidenceを生成しない。

### DES-1887

部分的な`hashes.targets`を持つEvidenceを生成しない。

### DES-1888

Evidenceを生成しない場合`target_binding`は`NO_EVIDENCE`（診断`NOT_EXECUTED`）のままとなる。

### DES-1889

Evidence記録後に宣言targetのいずれかが一意に解決できなくなった場合、記録済み参照集合が現在のcanonical集合と一致しないため`NO_EVIDENCE`（診断`STALE`）になり、`target_binding`をPASSにしない。

### DES-1890

解決できなくなったtargetは、対象が存在しない場合（E-SCAN-004）は`MISMATCH`（診断`MISSING`）、複数候補により曖昧な場合（E-SCAN-011）は`MISMATCH`として保持する。

### DES-1891

両者を一括して同一の状態値にしない。

### DES-1892

canonical Test metadata、ExecutionDescriptor、Test construct、宣言target集合、いずれかのtarget内容hash、HEAD revision、またはExecution State subjectがEvidenceと異なる場合はSTALE（`NO_EVIDENCE`、診断`STALE`）になる。

### DES-1893

`revision.commit`を特定できないEvidence、および現在のHEAD revisionと一致しないEvidenceは`NO_EVIDENCE`（診断`STALE`）になり、FAILまたは有効なPASSとして扱わない。

### DES-1894

Execution State subjectはrunner / toolchain / 実行影響configと、実行可能状態を変えうるrepository / local dependency入力の完全なmanifestを束縛する。

### DES-1895

Testと宣言targetを変更せずtarget外helperだけを変更しても既存Evidenceは`NO_EVIDENCE`（診断`STALE`）になる。

### DES-1896

EvidenceがExecution State subjectを欠く互換recordなら`NO_EVIDENCE`（診断`STALE`）になり、PASSにならない。

### DES-1897

recordのsnapshotまたは現在snapshotの完全性を証明できなければ`UNKNOWN`となり、PASSにならない。

### DES-1898

Evidenceが無効（STALE / MISMATCH / UNKNOWN）なら`target_binding`へ同じ非PASSを伝播し、無効Evidenceのresultまたはcoverageを再利用しない。

### DES-1899

Evidenceなしでは`target_binding`は`NO_EVIDENCE`（診断`NOT_EXECUTED`）になる。

### DES-1900

旧モデルの`test_execution` / `runtime_result` / `target_execution`の3独立項目は撤去し、`target_binding`単一検査の証拠（Evidenceの存在・鮮度、`result`、`target_coverage`）へ吸収する。

*引用: 本冊 §11.1*

### DES-1901

鮮度喪失の独立検査（旧`evidence_validity`）は設けず、鮮度は基本仕様§6のハッシュ束縛により満たし、喪失を診断ラベル`STALE`として説明する。

*導出元: SPEC-287, SPEC-288, SPEC-289, SPEC-290, SPEC-291, SPEC-292, SPEC-293, SPEC-294, SPEC-295, SPEC-296, SPEC-297, SPEC-298, SPEC-299*

*引用: 基本仕様 §6*

### DES-1902

単数互換形のEvidenceは、現在のTestがtargetをちょうど1件持つ場合だけ有効性を評価できる。

### DES-1903

複数target Testでは有効なPASSにしない。

### DES-1904

Evidenceのadapter IDがTest execution adapterと異なる場合はMISMATCHになる。

### DES-1905

有効なEvidenceについて、`result: FAIL`（テストランナーが失敗を報告）なら`target_binding`は`FAIL`になる。

*導出元: REQ-094, REQ-095, REQ-096, REQ-097, REQ-098, REQ-099, REQ-100, REQ-101, REQ-102, REQ-103, REQ-104*

*引用: 本冊 §11.2, 要件定義 §5.3*

### DES-1906

有効なEvidenceについて、`result: PASS`かつ全宣言targetの到達要件が§18.3.3 / §18.3.5で充足されれば`target_binding`は`PASS`になる。

### DES-1907

有効なEvidenceについて、`result: PASS`だが到達未充足targetがあれば、当該targetの`target_coverage`のcount 0は`target_binding`を`FAIL`（診断`NOT_EXECUTED`）にする。

### DES-1908

有効なEvidenceについて、`result: PASS`だが到達未充足targetがあれば、当該targetの`target_coverage`が計測不能・未計測（`checked: false`）は`target_binding`を`NO_EVIDENCE`（診断`NOT_CHECKED`）にする。

### DES-1909

有効なEvidenceについて、`result: PASS`だが到達未充足targetがあれば、当該targetの関数不見当は`target_binding`を`UNKNOWN`にする。

### DES-1910

各宣言targetについて、計測countが1以上ならtarget別PASSになる。

### DES-1911

各宣言targetについて、計測countが0ならtarget別FAILになる。

### DES-1912

各宣言targetについて、確実に同定または計測できなければtarget別UNKNOWNになる。

### DES-1913

複数target Testの集約値は、1件でもtarget別FAILがあればFAILになる。

### DES-1914

複数target Testの集約値は、FAILがなく1件でもUNKNOWNがあればUNKNOWNになる。

### DES-1915

複数target Testの集約値は、1件以上の全宣言targetがPASSの場合だけPASSになる。

### DES-1916

target AがPASSでもtarget BがFAILまたはUNKNOWNなら、Test単位の`target_binding`をPASSにしない。

### DES-1917

`target_coverage.checked: true`のEvidenceでtarget別entryが欠落、重複、または解決後のcanonical Source Target集合と不一致ならPASSにしない。

### DES-1918

target別entryは解決後のcanonical Locatorをidentityとし、宣言側の綴りを用いない。

*引用: 本冊 §6.1.1*

### DES-1919

coverage capabilityまたは計測toolが利用できない場合は`NO_EVIDENCE`（診断`NOT_CHECKED`）となり、PASSにならない。

### DES-1920

coverage解析限界は`UNKNOWN`となり、PASSにならない。

### DES-1921

Testが別プロセス（起動したsubprocess）・別スレッドでtargetを実行する場合、coverage計測が当該境界越しの実行を宣言targetへ帰属できればtarget別PASS（count > 0）になる。

### DES-1922

target別PASS（count > 0）という結果は本冊§7.3のruntime到達証明としても機能する。

*引用: 本冊 §7.3*

### DES-1923

providerが境界越しの実行を帰属できなければtarget別UNKNOWNとなり、PASSにならない。

### DES-1924

計測不能ならTestの`target_coverage`を`checked: false`（`NO_EVIDENCE`、診断`NOT_CHECKED`）とし、PASSにならない。

### DES-1925

`target_coverage` は `target_binding` の動的計測結果であり独立の検査項目ではない。

### DES-1926

旧モデルの`target_execution`検査項目は撤去し、計測事実だけをEvidenceの`target_coverage` fieldとして保持して`target_binding`の証拠源へ吸収する。

*引用: 本冊 §3.6・§10*

### DES-1927

`vtest audit bundle` は判断対象ごとに、判断に必要な情報（対象 VO と claim、Test Intent、テストコード、対象実装、関連テスト、既知 partition、過去の判断、対象の内容ハッシュとリビジョン）を JSON として `cache/bundles/` へ出力する。

### DES-1928

バンドルは派生情報であり Git 管理しない。

*導出元: SPEC-387, SPEC-388, SPEC-389, SPEC-390, SPEC-391, SPEC-392, SPEC-393, SPEC-394, SPEC-395, SPEC-396, SPEC-397, SPEC-398, SPEC-399, SPEC-400, SPEC-401, SPEC-402, SPEC-403, SPEC-404, SPEC-405, SPEC-406, SPEC-407, SPEC-408*

*引用: 本冊 §8.1, 基本仕様 §11.3*

### DES-1929

`vtest audit submit` の判断は少なくとも actor / subject / decision / judgment_kind を含み、理由・根拠（`reason` / `exclusions`）と `supersedes` は任意（optional）とする。

### DES-1930

submit は、bundle_id のバンドルが存在する（E-AUDIT-001）ことを順に検証し、失敗は§17のエラーコードで拒否する。

### DES-1931

submit は、subject がバンドルと一致する（E-AUDIT-003）ことを順に検証し、失敗は§17のエラーコードで拒否する。

### DES-1932

submit は、judgment_kind がバンドルと一致し値域内である（E-AUDIT-003）ことを順に検証し、失敗は§17のエラーコードで拒否する。

### DES-1933

submit は、バンドル記録時の各対象の内容ハッシュが現在と一致する（E-AUDIT-002）ことを順に検証し、失敗は§17のエラーコードで拒否する。

### DES-1934

submit は、decision が受理する判断値である（E-AUDIT-004）ことを順に検証し、失敗は§17のエラーコードで拒否する。

### DES-1935

submit は、supersedes の各 ULID が同一 subject かつ同一 judgment_kind の既存判断記録を指し自己参照でない（E-AUDIT-008）ことを順に検証し、失敗は§17のエラーコードで拒否する。

### DES-1936

理由が空であることだけを根拠に判断を無効・`UNKNOWN`・`NO_EVIDENCE`・`MISMATCH` 等として扱わない。

*導出元: REQ-180, REQ-181, REQ-182, REQ-183, REQ-184, REQ-185, REQ-186, REQ-187, REQ-188, REQ-189, REQ-190, REQ-191, REQ-192, SPEC-387, SPEC-388, SPEC-389, SPEC-390, SPEC-391, SPEC-392, SPEC-393, SPEC-394, SPEC-395, SPEC-396, SPEC-397, SPEC-398, SPEC-399, SPEC-400, SPEC-401, SPEC-402, SPEC-403, SPEC-404, SPEC-405, SPEC-406, SPEC-407, SPEC-408*

*引用: 基本仕様 §11.3, 要件定義 §12*

### DES-1937

旧モデルの reasons / claim / basis 必須検査（E-AUDIT-005）、decomposition-viewpoint 検査（E-AUDIT-006）、spec / req basis 検査（E-AUDIT-007）は撤去し、判断記録層で課さない。

### DES-1938

受理された提出は判断記録として `.verify/decisions/` へ保存され、バンドル生成時の全対象の内容ハッシュを `subject_hash` と `dependencies` として記録し、依存 closure のハッシュへ束縛する。

### DES-1939

判断記録の受理は当該対象の検証状態（§4.1 の 5 状態）を昇格させない。

### DES-1940

判断記録プロトコルは検証状態のゲートではなく、`UNKNOWN` に対する外部判断の追跡である。

*導出元: SPEC-387, SPEC-388, SPEC-389, SPEC-390, SPEC-391, SPEC-392, SPEC-393, SPEC-394, SPEC-395, SPEC-396, SPEC-397, SPEC-398, SPEC-399, SPEC-400, SPEC-401, SPEC-402, SPEC-403, SPEC-404, SPEC-405, SPEC-406, SPEC-407, SPEC-408*

*引用: 本冊 §8, 基本仕様 §11.3*

### DES-1941

旧モデルの `verdict → CheckValue` 写像（`impl_consistency = MISMATCH` を含む検証状態への変換経路）は撤去する。

### DES-1942

旧モデルの意味監査 bundle 種別（spec-coverage / test-semantic / vo-coverage / impl-consistency）を検査として扱わず、網羅・意味の疑義は `UNKNOWN` として本プロトコルへエスカレーションする。

*導出元: REQ-180, REQ-181, REQ-182, REQ-183, REQ-184, REQ-185, REQ-186, REQ-187, REQ-188, REQ-189, REQ-190, REQ-191, REQ-192, SPEC-217, SPEC-218, SPEC-219, SPEC-220, SPEC-221, SPEC-222, SPEC-223, SPEC-224, SPEC-225, SPEC-226, SPEC-227, SPEC-228, SPEC-229, SPEC-230, SPEC-231, SPEC-232, SPEC-233, SPEC-234, SPEC-235, SPEC-236, SPEC-237, SPEC-238, SPEC-239, SPEC-240, SPEC-241, SPEC-242, SPEC-243, SPEC-244, SPEC-245, SPEC-246, SPEC-247, SPEC-248, SPEC-249, SPEC-250, SPEC-251, SPEC-252, SPEC-253, SPEC-254, SPEC-255, SPEC-256, SPEC-257, SPEC-258, SPEC-259, SPEC-260, SPEC-261, SPEC-262, SPEC-263, SPEC-264, SPEC-265, SPEC-266, SPEC-267, SPEC-268, SPEC-269, SPEC-270, SPEC-271, SPEC-272, SPEC-273, SPEC-274, SPEC-275, SPEC-276, SPEC-277, SPEC-278, SPEC-279, SPEC-280, SPEC-281, SPEC-282, SPEC-283, SPEC-284, SPEC-285, SPEC-286, SPEC-376, SPEC-377, SPEC-378, SPEC-379, SPEC-380, SPEC-381, SPEC-382, SPEC-383, SPEC-384, SPEC-385, SPEC-386, SPEC-387, SPEC-388, SPEC-389, SPEC-390, SPEC-391, SPEC-392, SPEC-393, SPEC-394, SPEC-395, SPEC-396, SPEC-397, SPEC-398, SPEC-399, SPEC-400, SPEC-401, SPEC-402, SPEC-403, SPEC-404, SPEC-405, SPEC-406, SPEC-407, SPEC-408*

*引用: 本冊 §7.1・§8, 基本仕様 §5・§11, 要件定義 §12*

### DES-1943

`spec_coverage` / `vo_decomposition` / `vo_coverage` / `impl_consistency` は検証項目として存在しない。

### DES-1944

deterministic 結果（§18.3.3 の静的解析）と agent / human の判断結果を区別して保存・表示する。

### DES-1945

判断記録の有効性は判定時に評価し、subject が一致し `subject_hash` が現在の内容ハッシュと一致し、`dependencies` が現在の上流依存closureとentity・hashとも完全一致する場合だけ有効とする。

### DES-1946

document は登録 content_hash と実ファイルの一致も要求し、不一致の document を STALE とし、依存する判断記録も無効とする。

*引用: 本冊 §8.5・§11.4*

### DES-1947

同一対象に有効な判断記録が複数あってよい（再判断・多重判断）。

### DES-1948

判断バンドルは Test が宣言した cases 集合を規範項目として含む。

### DES-1949

`@vtest.case` 宣言の正規化文字列を宣言順に並べた list として出力する。

*導出元: SPEC-430, SPEC-431, SPEC-432, SPEC-433, SPEC-434*

*引用: 本冊 §8.1・§8.2, 基本仕様 §14*

### DES-1950

`@vtest.case` を持たない Test でも空 list を明示して項目を省略しない。

### DES-1951

バンドルと判断記録は判断型 `judgment_kind` をちょうど 1 件持つ。

### DES-1952

値域は `test-semantic` / `impl-consistency` / `case-coverage` であり、`subject` の値域は前 2 者が Test ID、`case-coverage` が Test ID または VO ID である。

### DES-1953

表にない組合せの要求ではバンドルを生成せず usage error（終了コード 2）とする。

*引用: 本冊 §8.1, 別紙A §12.2*

### DES-1954

`case-coverage` は §11 の判断対象であって基本仕様 §5 の 4 検査ではない。

*導出元: SPEC-217, SPEC-218, SPEC-219, SPEC-220, SPEC-221, SPEC-222, SPEC-223, SPEC-224, SPEC-225, SPEC-226, SPEC-227, SPEC-228, SPEC-229, SPEC-230, SPEC-231, SPEC-232, SPEC-233, SPEC-234, SPEC-235, SPEC-236, SPEC-237, SPEC-238, SPEC-239, SPEC-240, SPEC-241, SPEC-242, SPEC-243, SPEC-244, SPEC-245, SPEC-246, SPEC-247, SPEC-248, SPEC-249, SPEC-250, SPEC-251, SPEC-252, SPEC-253, SPEC-254, SPEC-255, SPEC-256, SPEC-257, SPEC-258, SPEC-259, SPEC-260, SPEC-261, SPEC-262, SPEC-263, SPEC-264, SPEC-265, SPEC-266, SPEC-267, SPEC-268, SPEC-269, SPEC-270, SPEC-271, SPEC-272, SPEC-273, SPEC-274, SPEC-275, SPEC-276, SPEC-277, SPEC-278, SPEC-279, SPEC-280, SPEC-281, SPEC-282, SPEC-283, SPEC-284, SPEC-285, SPEC-286*

*引用: 基本仕様 §5*

### DES-1955

`case-coverage`の未判断・判断結果はいずれの検査の値へも写像せず、集約へ寄与しない。

*引用: 本冊 §11.3*

### DES-1956

外部判断が必要な事実は判断待ち section（`check: null`、`judgment_kind: case-coverage`）としてだけ提示する。

*引用: 本冊 §8.1・§11.7*

### DES-1957

`case-coverage` の判断待ち項目は決定論的に生成する。

### DES-1958

`covers ≥ 1` かつ（`cases ≥ 1` または解決済みの covers 先 VO（レコードが存在する VO。E-SCAN-003 の dangling 参照を除く）のいずれかが `dimensions ≥ 1`）を満たす管理対象 Test ごとにちょうど 1 件生成し、`(当該 Test, case-coverage)` の実効判断が `accepted` の場合にだけ消滅する。

### DES-1959

実効判断が未確定・`rejected`・`deferred` のいずれでも項目は生成され、参照した判断記録 ID を `basis` に載せる。

### DES-1960

実効判断が `accepted` の場合にだけ消滅するという規則は `case-coverage` 型の項目にだけ適用し、検査に由来する `kind: unknown` の項目の生成・消滅は判断記録の有無で変わらない。

*引用: 本冊 §11.7*

### DES-1961

実効判断は `(subject, judgment_kind)` の組ごとに決まる。

### DES-1962

有効判断記録集合から、他の有効判断記録の `supersedes` に名指しされたものを除いた実効集合 E について、E が空なら未確定（`UNKNOWN`）とする。

### DES-1963

実効集合 E の decision 値が全て同一ならその値とする。

### DES-1964

実効集合 E に 2 種以上の decision 値があれば未確定（`UNKNOWN`）かつ W-STORE-004 とする。

*引用: 本冊 §8.5*

### DES-1965

競合は `supersedes` による明示の置き換えでだけ解消する。

### DES-1966

判断記録の新旧（`decided_at` / ULID 順）、`decision` 値の優先順位、記録件数の多寡のいずれも採用規則に用いない。

### DES-1967

競合中の対象について機械がいずれかの判断記録を採用した結果を出力しない。

### DES-1968

提出時、`supersedes` の各 ULID が同一 `subject` かつ同一 `judgment_kind` の既存判断記録を指し自己参照でないことを検証し、違反を E-AUDIT-008 で拒否する。

*引用: 本冊 §8.4*

### DES-1969

`judgment_kind` がバンドルと不一致または値域外の提出は E-AUDIT-003 で拒否する。

### DES-1970

レコード群が互いを名指しして実効集合 E が空になる場合は未確定（`UNKNOWN`）とし W-STORE-005 を出す。

*引用: 本冊 §8.5*

### DES-1971

いずれかのレコードを推測で残さない。

### DES-1972

`judgment_kind` を欠くか値域外の判断記録は履歴表示だけを許可し、いずれの実効判断へも寄与させず W-STORE-003 を出す。

*引用: 本冊 §3.4・§8.5*

### DES-1973

実効判断が未確定であることは検証状態（§4.1 の 5 状態）を変更せず、`UNKNOWN` に §4.2 の診断ラベルを付与しない。

### DES-1974

未確定の事実は判断待ち section としてだけ提示する。

*引用: 本冊 §8.5・§11.7*

### DES-1975

仕様・VO・Test 等が変更された場合、過去の判断を現在状態へそのまま流用せず、現在状態に対して §5 の 4 検査を再実施する。

*導出元: REQ-180, REQ-181, REQ-182, REQ-183, REQ-184, REQ-185, REQ-186, REQ-187, REQ-188, REQ-189, REQ-190, REQ-191, REQ-192, SPEC-387, SPEC-388, SPEC-389, SPEC-390, SPEC-391, SPEC-392, SPEC-393, SPEC-394, SPEC-395, SPEC-396, SPEC-397, SPEC-398, SPEC-399, SPEC-400, SPEC-401, SPEC-402, SPEC-403, SPEC-404, SPEC-405, SPEC-406, SPEC-407, SPEC-408*

*引用: 基本仕様 §11.3, 要件定義 §12*

### DES-1976

§5 の 4 検査を再実施した結果は `PASS` / `FAIL` / `MISMATCH` / `NO_EVIDENCE` / `UNKNOWN` のいずれにもなり得る。

### DES-1977

変更そのものが `UNKNOWN` を生成するのではない。

### DES-1978

判断対象の target を一意に解決できない場合はバンドルを生成せず、候補のいずれも選択しない。

*引用: 本冊 §8.1*

### DES-1979

対象が存在しない場合（E-SCAN-004）は `MISMATCH`（診断 `MISSING`）、複数候補により曖昧な場合（E-SCAN-011）は `MISMATCH` とし、両者を一括して同一の状態値にしない。

### DES-1980

判断済みと承認済みを区別する（判断済み ≠ 承認済み）。

### DES-1981

判断記録と承認記録は同一 entity であることを要求せず、別 entity でありうる。

*導出元: SPEC-387, SPEC-388, SPEC-389, SPEC-390, SPEC-391, SPEC-392, SPEC-393, SPEC-394, SPEC-395, SPEC-396, SPEC-397, SPEC-398, SPEC-399, SPEC-400, SPEC-401, SPEC-402, SPEC-403, SPEC-404, SPEC-405, SPEC-406, SPEC-407, SPEC-408, SPEC-471, SPEC-472, SPEC-473, SPEC-474, SPEC-475, SPEC-476, SPEC-477, SPEC-478, SPEC-479, SPEC-480, SPEC-481, SPEC-482, SPEC-483, SPEC-484, SPEC-485, SPEC-486, SPEC-487, SPEC-488, SPEC-489, SPEC-490, SPEC-491, SPEC-492, SPEC-493, SPEC-494, SPEC-495, SPEC-496, SPEC-497*

*引用: 本冊 §3.4・§3.5, 基本仕様 §11.3・§17*

### DES-1982

判断は承認なしでも記録でき、正式採用は承認の別段階である。

### DES-1983

承認は検証状態と独立の別軸である。

### DES-1984

承認済みを理由に非`PASS`（`FAIL` / `MISMATCH` / `NO_EVIDENCE` / `UNKNOWN`）を`PASS`へ昇格させず、未承認を理由に`PASS`を降格させない。

*導出元: SPEC-203, SPEC-204, SPEC-205, SPEC-206, SPEC-207, SPEC-208, SPEC-209, SPEC-471, SPEC-472, SPEC-473, SPEC-474, SPEC-475, SPEC-476, SPEC-477, SPEC-478, SPEC-479, SPEC-480, SPEC-481, SPEC-482, SPEC-483, SPEC-484, SPEC-485, SPEC-486, SPEC-487, SPEC-488, SPEC-489, SPEC-490, SPEC-491, SPEC-492, SPEC-493, SPEC-494, SPEC-495, SPEC-496, SPEC-497*

*引用: 基本仕様 §4.5・§17*

### DES-1985

判断受理も承認も、いずれも検証状態を昇格させない。

### DES-1986

`approved_state` の値域は `approved` / `rejected` / `withdrawn` の 3 値である。

### DES-1987

値域外の値、および値域外の `subject` 種別（判断記録 ULID・Test ID 等）は書込み時に E-APPROVAL-002 で拒否し record を生成しない。

### DES-1988

既存レコードとして読み取った場合は履歴表示だけを許可していかなる実効承認も導出せず W-STORE-006 を出す。

*引用: 本冊 §3.5*

### DES-1989

実効承認の導出は `approved_state` を参照する。

### DES-1990

有効承認レコード集合から、他の有効承認レコードの `supersedes` に名指しされたものを除いた実効集合について、集合が空なら `draft`、`rejected` または `withdrawn` が 1 件以上残るなら `draft`、全件が `approved` なら `approved` とする。

### DES-1991

有効の条件は `approved_state` が値域内であること、対象指定が一致すること、`subject_hash` が現在の内容ハッシュと一致すること、`dependencies` が現在の上流依存closureと entity・hash とも完全一致することである。

*引用: 本冊 §3.5*

### DES-1992

承認取消・却下は実効承認を `draft` へ落とす。

### DES-1993

`approved` の承認レコードが存在しても、後から `withdrawn` または `rejected` の有効承認レコードを追加すると実効承認は `draft` になる。

### DES-1994

機械は `approved` と `rejected` / `withdrawn` のどちらかを新旧・件数で選ばない。

### DES-1995

取消・却下後の再承認は `supersedes` による。

### DES-1996

当該 `withdrawn` / `rejected` レコードの ULID を `supersedes` に名指しした `approved` レコードを追加した場合にだけ `approved` へ戻る。

### DES-1997

名指ししない `approved` の追加では `draft` のままとする。

### DES-1998

`supersedes` の参照先が存在しない・対象が一致しない・自己参照は E-APPROVAL-002、循環は W-STORE-005 とする。

*引用: 本冊 §3.5*

### DES-1999

承認対象の値域は VO ID と document ID である。

### DES-2000

判断記録の承認は `judgment_ref` によってのみ表し、判断記録 ULID を `subject` に置かない。

### DES-2001

`judgment_ref` の参照先が存在しない場合は書込み時に E-APPROVAL-001、読取り時は当該レコードから VO / document の実効承認も判断記録の実効承認も導出せず W-STORE-006 とする。

*引用: 本冊 §3.5*

### DES-2002

判断記録を対象とする実効承認は、当該判断記録が §8.5 の有効判断でありかつ実効集合 E に属する場合にだけ導出する。

### DES-2003

supersede された判断記録・競合により未確定となった判断記録への承認は `draft` 相当とする。

*引用: 本冊 §3.5・§8.5*

### DES-2004

承認レコード生成の正典面は対象種別を引数に取る単一の経路である。

### DES-2005

`vtest approval create --subject-type <vo|document|judgment> --subject-id <id>` および `approval_create`（`subject: { type, id }`）だけが承認レコードを生成し、`vtest vo approve` / `vo_approve` は同経路への別名として同一のレコード・同一の拒否条件を持つ。

### DES-2006

対象種別ごとに別の承認規則・別の承認コマンドを設けない。

*引用: 本冊 §3.5, 別紙A §12.2・§13.2*

### DES-2007

`vtest approval withdraw <approval-id>` / `approval_withdraw` は `state: withdrawn` かつ `supersedes: [approval-id]` の `create` と同一のレコードを生成する。

### DES-2008

`vtest approval show` / `approval_get` は当該対象の承認レコード一覧と実効承認状態（`draft` / `approved`）を返す。

### DES-2009

方針は総称 document として登録した文書で表現し、専用のエンティティ型を設けない。

### DES-2010

document を対象とする承認の上流依存closureは当該 document の再帰的な上位 document（`derives_from` 先）からなり、`--subject-type document` で記録する。

### DES-2011

document 再登録（`--update`）で document subject hash が変化すると当該承認は失効する。

*引用: 本冊 §3.1・§3.5・§11.4*

### DES-2012

判断記録を対象とする承認は `--subject-type judgment` で記録し、`judgment_ref` へ判断記録 ULID を、`subject` へ当該判断記録の `subject` を書き込む。

*引用: 本冊 §3.5*

### DES-2013

判断記録 ULID を `subject` に置くレコードは生成しない。

### DES-2014

実効承認は明示の `supersedes` 関係だけで決まる。

### DES-2015

`supersedes` 関係にない複数の有効承認レコードはすべて実効集合に属し、`approved_at` / ULID の順序・レコードの新旧・件数の多寡のいずれも採用規則に用いない。

### DES-2016

`approved` と `rejected` が `supersedes` 関係なく併存する対象について、機械がどちらかに確定した結果を出力せず fail-closed に `draft` とする。

*引用: 本冊 §3.5*

### DES-2017

VO を対象とする承認の上流依存closureは、対象 VO の再帰的 parent VO、対象 VO と parent VO が `derives_from` で参照する document、および各 document の再帰的な上位 document からなる。

### DES-2018

document dependency は §1.3 の document subject hash を使用するため、document record または参照先 source の変更で承認が失効する。

*引用: 本冊 §3.5・§11.4*

### DES-2019

実効承認状態の遷移は `draft` と `approved` の 2 値の間でだけ起き、検証状態（§4.1 の 5 状態）の変化・判断記録の追加そのもの・`basis` の内容によっては遷移しない。

*引用: 本冊 §3.5*

### DES-2020

上流依存closureまたはハッシュを欠く互換 Approval は読取りと履歴表示だけを許可し、現在の `approved` を導出しない（W-STORE-002、VO は `draft` 相当）。

### DES-2021

承認主体は種別（`human` / `agent`）と識別子を記録する。

### DES-2022

承認権限（approval authority）・承認ロール・必要承認数・権限 schema はプロジェクト設定と別紙A へ委譲する。

*導出元: SPEC-471, SPEC-472, SPEC-473, SPEC-474, SPEC-475, SPEC-476, SPEC-477, SPEC-478, SPEC-479, SPEC-480, SPEC-481, SPEC-482, SPEC-483, SPEC-484, SPEC-485, SPEC-486, SPEC-487, SPEC-488, SPEC-489, SPEC-490, SPEC-491, SPEC-492, SPEC-493, SPEC-494, SPEC-495, SPEC-496, SPEC-497, SPEC-740, SPEC-741, SPEC-742, SPEC-743, SPEC-744, SPEC-745, SPEC-746, SPEC-747, SPEC-748, SPEC-749, SPEC-750, SPEC-751, SPEC-752, SPEC-753, SPEC-754, SPEC-755, SPEC-756, SPEC-757, SPEC-758, SPEC-759, SPEC-760, SPEC-761, SPEC-762, SPEC-763, SPEC-764, SPEC-765*

*引用: 基本仕様 §17・§30*

### DES-2023

承認 workflow の状態遷移と `approved_state` の値域は本冊 §3.5 に定める。

*引用: 本冊 §3.5*

### DES-2024

完全検証は基本仕様 §5 の 4 検査（`chain_integrity` / `orphan_detection` / `target_binding` / `oracle_presence`）をすべて評価し、各検査の非PASSを総合NGへ反映する。

*導出元: SPEC-217, SPEC-218, SPEC-219, SPEC-220, SPEC-221, SPEC-222, SPEC-223, SPEC-224, SPEC-225, SPEC-226, SPEC-227, SPEC-228, SPEC-229, SPEC-230, SPEC-231, SPEC-232, SPEC-233, SPEC-234, SPEC-235, SPEC-236, SPEC-237, SPEC-238, SPEC-239, SPEC-240, SPEC-241, SPEC-242, SPEC-243, SPEC-244, SPEC-245, SPEC-246, SPEC-247, SPEC-248, SPEC-249, SPEC-250, SPEC-251, SPEC-252, SPEC-253, SPEC-254, SPEC-255, SPEC-256, SPEC-257, SPEC-258, SPEC-259, SPEC-260, SPEC-261, SPEC-262, SPEC-263, SPEC-264, SPEC-265, SPEC-266, SPEC-267, SPEC-268, SPEC-269, SPEC-270, SPEC-271, SPEC-272, SPEC-273, SPEC-274, SPEC-275, SPEC-276, SPEC-277, SPEC-278, SPEC-279, SPEC-280, SPEC-281, SPEC-282, SPEC-283, SPEC-284, SPEC-285, SPEC-286*

*引用: 基本仕様 §5*

### DES-2025

完全検証は、各検査の評価地点（DOC / VO / TEST / repository）で評価した全値がPASSの場合だけOKとする。

### DES-2026

`--items`を省略したCLI / MCP検証は常に固定4検査を評価する。

### DES-2027

version 1 configの`full_scope`欠落は固定4検査へ具体化し、version 1 / version 2 いずれでも旧12項目の列挙（`spec_coverage` / `test_existence` 等）は E-CONFIG-001 で拒否し、in-memory 補完で受理しない。

*引用: 本冊 §2.2*

### DES-2028

version 1 の重複・未知項目、version 2 の欠落・重複・未知・余剰項目も E-CONFIG-001 とし、検証結果を生成しない。

### DES-2029

4検査未満を明示した`--items`だけを限定scopeとして扱い、「完全検証」と表示しない。

### DES-2030

scope は 2 軸で限定できる。

*導出元: P-002, SPEC-210, SPEC-211, SPEC-212, SPEC-213, SPEC-214, SPEC-215, SPEC-216*

*引用: 基本仕様 §4.6, 要件定義 P-002*

### DES-2031

検査軸（4 本の部分集合）とエンティティ軸（対象とする document / VO / Test の部分木）を指定でき、限定scopeのOKは「要求scope内のOK」に限られる。

### DES-2032

いかなる設定値も完全検証の検査を 4 本未満へ縮退させない。

### DES-2033

限定scopeは要求項目だけを集約し、scope外・未実施の項目を `NO_EVIDENCE`（診断 `NOT_CHECKED`）として保持・併記する。

### DES-2034

出力には要求 scope と scope 外項目が未検証である旨を必ず併記する。

### DES-2035

`verify` / `report` の JSON（CLI・MCP）は最上位に `scope` を返し、`scope.requested.items`（`--items` 省略時は固定4検査を4件すべて列挙）、`scope.requested.entities`（エンティティ軸無指定は空 list）、`scope.unverified_outside_scope`（検査軸4件未満またはエンティティ軸指定ありで `true`、完全検証で `false`）を持つ。

### DES-2036

完全検証でも `scope` を省略しない。

### DES-2037

検証結果を返さないコマンド（`init` / `scan` / `doc *` / `vo *` / `test *` / `audit *` / `run`）の JSON は `scope` を持たない。

### DES-2038

限定 scope の JSON 出力だけから、要求 scope と「scope 外は未検証」の旨を判定できる（`scope.unverified_outside_scope` が `true` で、scope 外検査ノードが `NO_EVIDENCE`／診断 `NOT_CHECKED`）。

### DES-2039

機能単位の集約は親 VO（子 VO を持つ VO）を単位とし、Feature を別エンティティ・別レコード・別 ID として設けない。

### DES-2040

親 VO の値は子 VO の値と当該親 VO を直接 covers する Test の値の fail-closed 合成であり、いずれかに非 `PASS` が 1 件でもあれば親 VO は非 `PASS` になる。

### DES-2041

`--vo <親VO>` および `--from <親VO> --direction down` が親 VO の代表値と配下の子 VO・Test の内訳を同一出力で返し、出力に Feature 名・Feature ID の field を含めない。

### DES-2042

要求scope内の `FAIL`・`MISMATCH`・`NO_EVIDENCE`・`UNKNOWN` のいずれも総合PASSへ昇格しない。

### DES-2043

NO_EVIDENCE を生む入力（証拠が存在しない／証拠のハッシュが現在の対象と不一致／scope 限定により検査を実施しなかった項目）を受入で表現する。

### DES-2044

NO_EVIDENCE を生む入力は `NO_EVIDENCE`（診断は順に `NOT_EXECUTED` / `STALE` / `NOT_CHECKED`）となり `PASS` へ変換されない。

*導出元: SPEC-191, SPEC-192, SPEC-193, SPEC-194, SPEC-195, SPEC-196, SPEC-197, SPEC-198, SPEC-199, SPEC-210, SPEC-211, SPEC-212, SPEC-213, SPEC-214, SPEC-215, SPEC-216*

*引用: 基本仕様 §4.3・§4.6*

### DES-2045

完全検証fixtureで4検査のそれぞれを単独で非PASSにすると総合NGになる。

### DES-2046

管理済みgraph側の他検査がすべてPASSでも、未登録Testが1件あれば`chain_integrity`により総合NGになる。

### DES-2047

集約は fail-closed とし、子に 1 つでも非 `PASS` があれば親は非 `PASS`。

### DES-2048

代表値の優先順位は `FAIL > MISMATCH > NO_EVIDENCE > UNKNOWN` とし、診断ラベル（`MISSING` / `NOT_EXECUTED` / `NOT_CHECKED` / `STALE`）は代表値の順位に用いず原因説明として併記する。

*導出元: SPEC-577, SPEC-578, SPEC-579, SPEC-580, SPEC-581, SPEC-582*

*引用: 基本仕様 §22.2, 本冊 §11.3*

### DES-2049

report は DOC → VO → Test の構造と、各非PASSの根拠（判断記録・Evidence への参照）を text / JSON で返す。

### DES-2050

旧モデルの SPEC → REQ → VO → Test 構造は総称 document 化により DOC → VO → Test へ再導出する。

### DES-2051

`covers` を持つ Test は covers 先 VO の子ノードとして表示する。

### DES-2052

管理下にある事実と、いずれの VO へも寄与しない事実の双方を出力から確認できる。

*導出元: SPEC-583, SPEC-584, SPEC-585, SPEC-586, SPEC-587, SPEC-588, SPEC-589, SPEC-590, SPEC-591, SPEC-592, SPEC-593, SPEC-594, SPEC-595*

*引用: 基本仕様 §22.3*

### DES-2053

`covers` を持たない Test は §18.3.1 の `chain_integrity = MISMATCH` として扱い、役割別表示を設けない。

### DES-2054

text treeのancestor continuation、middle child、last childを一意なbranch記号で描画する。

### DES-2055

同一 revision・同一 `.verify/` ファイル集合（`config.yaml`・document / VO / Relation レコード・判断記録・承認・Evidence）・同一 scope 指定に対して `verify` を繰り返し実行すると、4 検査の検証状態・診断ラベル・診断コード集合・集約結果・`pending` section・終了コードが毎回一致する。

*導出元: SPEC-379, SPEC-380, SPEC-381, SPEC-382, SPEC-383*

*引用: 本冊 §11.1, 基本仕様 §11.1*

### DES-2056

実行時刻・ロケール・タイムゾーン・呼出し元の作業ディレクトリを変えても、また Execution State subject の入力に影響しない環境変数を変えても、上記の出力が変化しない。

*引用: 本冊 §1.3*

### DES-2057

ネットワークを遮断した環境でも同一の出力を返す。

### DES-2058

toolchain identity・adapter config・入力 manifest を変える環境変更（`RUSTUP_TOOLCHAIN` の切替等）の影響は Evidence の鮮度喪失（`NO_EVIDENCE`、診断 `STALE`。本冊 §11.2）としてのみ現れ、環境そのものを判定条件として読む経路を持たない。

*引用: 本冊 §11.2*

### DES-2059

`vtest` は 4 検査の評価中に LLM API を含む外部サービスへ要求を出さない。

### DES-2060

外部 AI／Agent の関与は `.verify/decisions/` の判断記録ファイル経由に限られ、判断記録の受理は検証状態を昇格させない。

### DES-2061

4 検査の評価経路に、実行時に差し替え可能な意味判定 seam を持たない。

### DES-2062

評価経路へそのような seam を導入する変更を行う場合は、正反対の判定を返す stub を注入しても 4 検査の結果が変化しないことを受入で確認する。

### DES-2063

`report --from DOC-X --direction down --format json` は、`derives_from` エッジごとに `from` / `relation` / `to` と当該 entry の `anchor`・`note` を返し、「どの上流条項がどの VO へ対応するか」の対応ペア集合として読める。

*導出元: SPEC-379, SPEC-380, SPEC-381, SPEC-382, SPEC-383*

*引用: 本冊 §11.6・§3.1・§3.2, 基本仕様 §11.1*

### DES-2064

`anchor` を持たない entry では `anchor` を省略または `null` とし、空文字列で埋めない。

### DES-2065

「どの上流条項がどの VO へ対応するか」の対応ペアの取得に新規 CLI コマンド・MCP ツールを用いない（既存の `report` projection と `test query` 逆引きだけで取得できる）。

### DES-2066

プロジェクト側が登録したフェーズ・工程・ゲートの進行条件について、現在の検証状態（§4.1 の 5 状態）と承認（§18.3.7）が通過条件を満たすかを評価・提示できなければならない（MUST）。

*導出元: REQ-302, REQ-303, REQ-304, REQ-305, REQ-306, REQ-307, REQ-308, SPEC-535, SPEC-536, SPEC-537, SPEC-538, SPEC-539, SPEC-540, SPEC-541, SPEC-542*

*引用: 本冊 §11.5, 基本仕様 §20, 要件定義 §26.4*

### DES-2067

ゲート定義は `config.yaml` の `gates` に、ゲート名と進行条件（`require.verification` ＝要求する検証結果、`require.approvals` ＝要求する承認ロール集合）として保持する。

### DES-2068

`vtest verify --gate <name>` は、指定ゲートの対象 scope について検証を実行し、(1) 検証結果が `require.verification` を満たすか、(2) `require.approvals` の各ロールについて対象の有効な承認が存在するか、を評価して満否と根拠（不足している非 `PASS` 検査・未充足の承認ロール）を提示する。

### DES-2069

条件充足・不足の両方を fixture で確認する。

### DES-2070

検証状態と承認は独立の軸であり、ゲートは両者の組合せを進行条件にできる。

### DES-2071

承認済みを理由に検証状態を昇格させない。

### DES-2072

`require.verification` の値域を config 受理時に検査する。

### DES-2073

5 状態語彙（`PASS` / `FAIL` / `MISMATCH` / `NO_EVIDENCE` / `UNKNOWN`）との完全一致は受理する。

### DES-2074

診断ラベル（`MISSING` / `NOT_EXECUTED` / `NOT_CHECKED` / `STALE`）・`OK` / `NG`・小文字表記・旧12項目名・非文字列値は E-CONFIG-001・終了コード 2 で拒否して検証結果を生成しない。

### DES-2075

`require` および `require.verification` の欠落、`gates[].name` の重複も E-CONFIG-001 とする。

*引用: 本冊 §2.2*

### DES-2076

`require.approvals` の省略と `gates` field 自体の欠落・空 list は受理する。

### DES-2077

ゲートの検証条件は `require.verification` と要求 scope の集約代表値の完全一致でのみ充足する。

### DES-2078

`require.verification` に `PASS` 以外（例 `UNKNOWN`）を定義したゲートは、代表値が同じ値のときだけ充足し、代表値が `PASS` のときは充足しない。

### DES-2079

逆に `require.verification: PASS` のゲートは代表値が非 `PASS` のとき充足しない。

### DES-2080

順序・包含解釈による充足を認めない fixture を持つ。

### DES-2081

集約代表値は構造検査（`chain_integrity` / `orphan_detection`）を含む要求 scope 内の全評価値の fail-closed 合成であり、エンティティ軸の部分木が全 `PASS` でも構造検査が非 `PASS` なら代表値は非 `PASS` になる。

### DES-2082

`--items` で検査軸を限定した実行では scope 外検査が `NO_EVIDENCE`（診断 `NOT_CHECKED`）として代表値に参加するため、`require.verification: PASS` のゲートは限定 scope で充足しない。

### DES-2083

`--gate` を指定した `verify` / `report` の JSON は `data.gate` に `name`・`verification.{required, actual, satisfied}`・`approvals[].{role, satisfied, missing_subjects}`・`satisfied` を返す。

### DES-2084

`require.approvals` が空集合なら `approvals` は空 list、`gate.satisfied` は `verification.satisfied` と全 `approvals[].satisfied` の論理積になる。

### DES-2085

`--gate` 指定時の最上位 `ok` と終了コードはゲート充足で決まる（充足 → `ok: true`・0、不充足 → `ok: false`・1）。

### DES-2086

`require.verification` に `PASS` 以外を定義したゲートが充足した実行は、総合が NG でも終了コード 0 になる。

### DES-2087

config の `gates` に定義の無いゲート名を `verify --gate` / `report --gate` / MCP の `gate` 入力へ指定すると、E-CONFIG-002・`ok: false`・終了コード 2 で拒否し、検証もゲート評価も実行せず部分結果を返さない。

### DES-2088

診断には指定名と定義済みゲート名の一覧を含み、MCP tool error は `candidates` に定義済みゲート名を持つ。

### DES-2089

`gates` が空・未定義の状態での指定も同じ扱いとする。

### DES-2090

ゲート名の解決は大文字小文字を区別した完全一致だけで行い、前方一致・部分一致・近似一致・既定ゲートへの代替で受理しない。

### DES-2091

責務はゲート条件が現在満たされているかの評価・提示に限る。

### DES-2092

フェーズのライフサイクル管理・工程の自動遷移は責務外とする。

*導出元: REQ-302, REQ-303, REQ-304, REQ-305, REQ-306, REQ-307, REQ-308, SPEC-535, SPEC-536, SPEC-537, SPEC-538, SPEC-539, SPEC-540, SPEC-541, SPEC-542, SPEC-729, SPEC-730, SPEC-731, SPEC-732, SPEC-733, SPEC-734, SPEC-735, SPEC-736, SPEC-737, SPEC-738, SPEC-739*

*引用: 基本仕様 §20・§29 OOS-004, 要件定義 §26.4*

### DES-2093

新規 CLI コマンド・MCP ツールを増やさず、既存の `vtest verify` の `--gate` 引数と出力、および `report` の JSON でゲート評価を露出する。

### DES-2094

具体的なフェーズ名・承認ロール・必要承認数はプロジェクト設定と別紙A へ委譲する。

*導出元: SPEC-740, SPEC-741, SPEC-742, SPEC-743, SPEC-744, SPEC-745, SPEC-746, SPEC-747, SPEC-748, SPEC-749, SPEC-750, SPEC-751, SPEC-752, SPEC-753, SPEC-754, SPEC-755, SPEC-756, SPEC-757, SPEC-758, SPEC-759, SPEC-760, SPEC-761, SPEC-762, SPEC-763, SPEC-764, SPEC-765*

*引用: 基本仕様 §30*

### DES-2095

Form `kind`は`[a-z0-9][a-z0-9-]*`のcase-sensitive文字列で、built-inとuser-defined schemaを通してrepository全体で一意であり、schemaはowner `adapter` IDを別fieldで宣言する。

### DES-2096

registryのkind owner、schemaのadapter、Structured Test capabilityが一意に一致する場合だけcreate / form_getを許可する。

### DES-2097

同じkindを複数adapterが宣言する、schemaとregistry ownerが不一致、adapterが未知、またはcapabilityがない場合は操作を拒否し、ファイルを変更しない。

### DES-2098

`adapter`を欠く読取り互換Formは、登録済みStructured Test adapterのbuilt-in kind宣言またはschema compatibility matcherのうちちょうど1件だけがschemaを受理する場合に限って解決し、曖昧またはowner不在なら拒否する。

### DES-2099

matcherはschema内容から決定論的に判定し、coreは未知kindを`rust-cargo`へfallbackしない。

### DES-2100

Form Schemaの必須値と未知fieldを常に検証する。

### DES-2101

symbol、VO / Test参照、identifier、pathは選択したFormが該当fieldとvalidatorを宣言した場合だけ検証し、すべてのadapterへ一律に要求しない。

### DES-2102

create結果はscanで同じTest ID・intent・covers・targetsとして認識される。

### DES-2103

editは1 Testの拡張rangeだけを単一置換し、他Testと通常sourceを変更しない。

### DES-2104

同じdesired stateの再適用は冪等になる。

### DES-2105

Structured Test capabilityがないadapterへのcreate / editはE-ADAPTER-004となり、ファイルを変更しない。

### DES-2106

create は挿入後に対象ファイルを再パースし、構文妥当性・挿入分がちょうど 1 Test として認識されること・その Test ID と annotation が desired state と一致すること・他の Test と通常 source が不変であることを確認する。

*導出元: SPEC-440, SPEC-441, SPEC-442, SPEC-443*

*引用: 別紙A §15.2, 基本仕様 §15.1*

### DES-2107

edit と同じ確認項目を create でも実施し、create 経路にだけ検証を省く分岐を設けない。

### DES-2108

挿入後の再パースが構文エラーになる fixture、挿入結果の annotation が desired state と一致しない fixture、挿入が他の Test 範囲へ及ぶ fixture のそれぞれで、create は E-OP-003・終了コード 2 になり、対象ファイルが挿入前のバイト列へ復元される。

### DES-2109

挿入によりファイルが新規作成されていた場合は不存在へ戻る。

### DES-2110

ロールバック後に scan すると、当該 create 操作が無かった場合と同一のエンティティ集合・内容ハッシュが得られる。

### DES-2111

部分適用された挿入内容・採番された Test ID・Evidence・判断記録がいずれも残らない。

### DES-2112

`create --dry-run` は挿入内容と挿入位置を提示し、ファイルを変更しない。

### DES-2113

同一 desired state からの create と、その直後の同一 desired state による edit は差分を生じない（annotation block の再生成規則が create / edit で同一。別紙A §15.3）。

*引用: 別紙A §15.3*

### DES-2114

別紙A（§12〜§15）が定める全 MCP tool が同じ入力に対するCLI JSONと同じdata / diagnosticsを返す。

### DES-2115

不正入力はcode / message / candidatesを持つtool errorになる。

### DES-2116

request、notification、batch、malformed transportの各入力をJSON-RPC contractどおりに処理する。

### DES-2117

MCP serverの長時間実行中もsource変更を再scanし、staleなPASSを保持しない。

### DES-2118

`vtest-adapter-api`は言語・runner非依存であり、Cargo、Rust parser、llvm-cov固有型を公開しない。

### DES-2119

`vtest-model::TestEntity`はTestを関数として表現せず、adapter所有のTest constructを論理metadata、Source Location、content hash、ExecutionDescriptorで表現する。

### DES-2120

`TargetRef::Locator`はadapter IDとadapter所有のopaque locatorを保持する。

### DES-2121

`SourceLocation`はadapter ID、project-relative path、opaque locator、source rangeを保持する。

### DES-2122

`TargetRef::Locator`と`SourceLocation`のどちらもRust module path、関数名、`.rs`拡張子をcoreの不変条件にしない。

### DES-2123

`vtest-model::TestEntity`は`ExecutionDescriptor`だけを実行座標として持ち、`filter`、`package`、`test_target`、`TestTarget`を含まない。

### DES-2124

`TestEntity.content_hash`はTest constructだけでなくcanonical metadata、locationのadapter・path・opaque locator、ExecutionDescriptorを含むTest subjectへ束縛される。

### DES-2125

byte range自体は含めず、非隣接metadataだけの意味変更でもhashが変化する。

### DES-2126

`SourceDiscoveryAdapter`はhash未計算DTOを返し、coreがDTO検証・hash計算・domain entity具体化をこの順で行う。

### DES-2127

`rust-cargo` adapterはRust discovery、static audit、Structured Test Operation、runner、coverageを所有する。

### DES-2128

`vtest-scan`はadapter discoveryの委譲・出力検証・決定論的統合・core record整合性を所有し、`*.rs`列挙、`syn::parse_file`、`#[test]`抽出、doc comment parseを所有しない。

### DES-2129

registryはadapter IDの重複、宣言capabilityと実装の不一致、未登録adapterを拒否する。

### DES-2130

異なるadapterが同じrootを共有でき、同一adapter内のroot重複は拒否される。

### DES-2131

全adapterのmerge結果でTest IDのglobal uniquenessを検査する。

### DES-2132

config readerはversion 1とversion 2を受理し、読み取りだけでconfigを書き換えない。

### DES-2133

config writerと`vtest init`はversion 2のadapter namespaceを出力する。

### DES-2134

Test JSON writerは`execution`を常に出力し、`rust-cargo` Testについてだけwire codecが互換field `filter` / `package` / `test_target`を追加する。

### DES-2135

Test JSON writerは1件以上の`targets` listを常に出力し、targetが1件の場合だけ同値の単数互換field`target`を追加できる。

### DES-2136

複数targetを単数fieldへ縮約しない。

### DES-2137

synthetic TestのJSONはRust互換fieldを省略し、空値またはdummy値を出力しない。

### DES-2138

`execution`を欠くTest入力は、`rust-cargo` codecが完全で相互整合するRust互換fieldからだけdescriptorを導出する。

### DES-2139

`execution`とRust互換fieldが矛盾する入力を拒否する。

### DES-2140

明示操作に必須のcapabilityがなければE-ADAPTER-004となり、変更・判断記録・Evidenceを生成しない。

### DES-2141

検証時のstatic audit / coverage capability欠落は`NO_EVIDENCE`（診断`NOT_CHECKED`）になる。

### DES-2142

検証時のrunner欠落は`NO_EVIDENCE`（診断`NOT_EXECUTED`）になる。

### DES-2143

検証時の解析限界は`UNKNOWN`になる。

### DES-2144

Rustとsyntheticの結果をadapter ID、path、Test IDで決定論的に統合する。

### DES-2145

synthetic adapterは`.rs`以外のsource、関数ではないTest construct、doc commentではないmetadata宣言、Rust item pathではないopaque locatorを、`vtest-model`、`vtest-scan`、`vtest-verify`の変更なしで登録・scan・verifyできる。

### DES-2146

GUI は提供範囲外である。

### DES-2147

仕様書同士の矛盾判定は提供範囲外である。

### DES-2148

仕様・Test・実装のどれを変更すべきかという修正方針の決定は提供範囲外である。

### DES-2149

helper、fixture、通常sourceの編集管理は提供範囲外である。

### DES-2150

開発process管理は提供範囲外である。

### DES-2151

`rust-cargo`以外のproduction language adapterは提供範囲外である。

### DES-2152

third-party plugin ABIは提供範囲外である。

### DES-2153

LSP統合は提供範囲外である。

### DES-2154

runner / coverage providerの自動選択または推測fallbackは提供範囲外である。
