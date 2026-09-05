<!-- generated from docs/canonical/specification.json by build.py; do not edit -->

# 基本設計

## BD-S001 0. 本書の位置付け

*導出元: P-005, REQ-S049, REQ-S058*

### BD-001

ツール名は `vtest` とする（バイナリ名・ディレクトリ名に使用する）。

*導出元: REQ-309, REQ-310, REQ-311, REQ-312, REQ-313, REQ-314, REQ-315, REQ-316, REQ-317, REQ-318, REQ-319, REQ-320, REQ-321, REQ-322, REQ-323, REQ-324, REQ-325, REQ-326, REQ-327, REQ-328, REQ-329, REQ-330, REQ-331, REQ-332*

*引用: 要件定義 §28*

### BD-002

`vtest` 本体の実装言語はRustとする。

### BD-003

組込 production adapter は `rust-cargo` とする。

### BD-004

インターフェースはCLIと、AI Agent向けMCPサーバとする。

*導出元: REQ-267, REQ-268, REQ-269, REQ-270*

*引用: 要件定義 §22*

### BD-005

MCPを本体とする。

### BD-006

Rust固有処理は組込 `rust-cargo` adapter が所有する。

### BD-007

CLI・MCP・検証coreはadapter registryを介して能力を選択する。

### BD-008

coreの検証契約は言語・test runnerに依存しない。

*導出元: REQ-258, REQ-259, REQ-260, REQ-261, REQ-262, REQ-263, REQ-264, REQ-265, REQ-266*

*引用: 要件定義 §21*

## BD-S002 1. 用語定義

*導出元: REQ-S001, REQ-S003, REQ-S013, REQ-S020, REQ-S025, REQ-S029, REQ-S035, REQ-S036, REQ-S037, REQ-S046*

### BD-009

正典（source of truth）とは、ある事実を決定する唯一の記録である。

## BD-S003 2 全体像

### BD-S004 2.1 正典の三層構造

*導出元: P-003, REQ-S019, REQ-S035, REQ-S046*

### BD-010

本システムは扱う情報を三層に分ける。

### BD-011

宣言層は、adapter所有のTest metadata宣言、および.verify/配下のdocument / VO / Relationレコードからなり、Gitで管理される正典である。

### BD-012

実装層は、テストコード本体と対象ソースコードからなり、Gitで管理される正典である。

### BD-013

事実層は、実行結果・判断記録・承認記録からなる.verify/配下の追記型レコードファイルであり、Gitで管理される。

### BD-014

派生情報（検索インデックス、検証グラフ、集約結果）は上記から毎回再構築する。

### BD-015

派生情報はGit管理しない。

*導出元: P-003*

*引用: 要件定義 P-003 / NFR-004*

### BD-016

source discovery、決定論的解析、Structured Test Operation、test runner起動、coverage計測はadapter capabilityとして提供する。

### BD-S005 2.2 宣言鎖と照合

*導出元: REQ-S004, REQ-S005*

### BD-017

文書層の段数は総称的に扱い、リンクを追加してもスキーマが壊れないことを設計制約とする。

### BD-S006 2.3 導出できる関係は保存しない

*導出元: P-003*

### BD-018

外部レコードとして保存するのは、どちらか一方のエンティティに自然に所属しない関係（VO間の依存、Test間の補完関係など）だけとする。

*導出元: P-003*

*引用: 要件定義 P-003*

## BD-S007 3 エンティティと ID 体系

### BD-S008 3.1 エンティティ種別

*導出元: REQ-S005, REQ-S025, REQ-S035, REQ-S036, REQ-S040, REQ-S046*

### BD-019

documentのIDは `DOC-` とし、正典は `.verify/doc/` に置く。

### BD-020

Verification ObligationのIDは `VO-` とし、正典は `.verify/vo/` に置く。

### BD-021

TestのIDは `TEST-` とし、正典はadapter所有のTest metadata宣言とする。

### BD-022

Source TargetはIDを持たない、または任意で `SRC-` を用い、adapter IDとopaque locatorで識別する。

### BD-023

RelationのIDは `REL-`（ULID）とし、正典は `.verify/rel/` に置く。

### BD-024

判断記録のIDはULIDとし、正典は `.verify/decisions/` に置く。

### BD-025

承認記録のIDはULIDとし、正典は `.verify/approvals/` に置く。

### BD-026

Execution EvidenceのIDはULIDとし、正典は `.verify/evidence/` に置く。

### BD-S009 3.3 Source Target の識別

*導出元: R-3, REQ-S027*

### BD-027

opaque locatorの構文と恒久SRC IDの宣言方法はadapterが定める。

### BD-028

共通契約がpath・module・function等の特定言語構造を必須としてはならない。

*導出元: R-3, REQ-149, REQ-150, REQ-151, REQ-152, REQ-153, REQ-154, REQ-155, REQ-156*

*引用: 要件定義 §9.2、R-3*

### BD-029

Test→SRCの対応はadapter所有のTest metadata宣言から提供する。

### BD-030

SRC→Testの逆引きはスキャン結果から提供する。

## BD-S010 5 検査

### BD-S011 5.2 orphan_detection — 文書層の孤児検出

*導出元: REQ-S010*

### BD-031

根の指定方式は `.verify/` 設定における明示的な根指定として保持する。

### BD-S012 5.5 決定論的に検出可能な不成立構造

*導出元: REQ-S024*

### BD-032

coreはadapter固有のAST・assertion構文・call graphを解釈しない。

### BD-033

coreは正規化されたルール結果を検証・集約する。

### BD-034

code fragmentの具体構文はadapterの言語・runnerに従う。

## BD-S013 6. 証拠

*導出元: REQ-S019*

### BD-035

Evidenceの判定結果を変えうるTestの意味・実行条件・対象実装・実行可能状態が現在状態と一致することを確認できなければそのEvidenceを現在の `PASS` として利用してはならないという要求は、ハッシュ束縛によって設計制約として満たす。

### BD-036

adapterが最終内容ハッシュを自己確定してはならない。

## BD-S014 8 Test の検証成立性

*導出元: REQ-S021, REQ-S025*

### BD-S015 8.3 決定論的に検出可能な不成立構造

### BD-037

各adapterは対応する言語・runnerの構造に対して決定論的に判定できる範囲を提供する。

## BD-S016 9 検証対象と Source Target

*導出元: REQ-S025*

### BD-S017 9.2 Source Target の識別

### BD-038

各adapterはSource Targetを一意に解決でき、同一source stateから決定論的に正規化できるTarget Referenceを提供する。

## BD-S018 14. Parameterized / Table-Driven Test

*導出元: R-3, REQ-S038*

### BD-039

code fragmentの具体構文はadapterの言語・runnerに従う。

*導出元: R-3, REQ-206, REQ-207, REQ-208, REQ-209*

*引用: 要件定義 §15、R-3*

## BD-S019 15 Structured Test Operation

*導出元: P-004, REQ-S039*

### BD-S020 15.4 Form Schema

### BD-040

テスト種別ごとの質問・入力項目テンプレートをForm Schemaとして `.verify/forms/` に定義できる。

## BD-S021 23. スキャンと整合性検査

*導出元: REQ-S036, REQ-S050*

### BD-041

`vtest scan` はregistryに登録された全source discovery adapterへ委譲する。

### BD-042

`vtest scan` は統合したdiscovery結果と `.verify/` からエンティティと関係の全体グラフを再構築する。

### BD-043

`vtest scan` はその過程で `chain_integrity` / `orphan_detection` を構成する整合性検査を行う。

*導出元: REQ-193, REQ-194, REQ-195, REQ-196, REQ-197, REQ-198, REQ-199, REQ-200, REQ-201, REQ-202, REQ-271, REQ-272, REQ-273, REQ-274, REQ-275, REQ-276, REQ-277, REQ-278, REQ-279*

*引用: 要件定義 §13、§23*

## BD-S022 24 データ保存の基本方針

*導出元: REQ-S050, REQ-S058*

### BD-S023 24.1 `.verify/` ディレクトリ

### BD-044

プロジェクトルート直下に `.verify/` を置く。

### BD-045

`.verify/` にテストコード外の正典と事実レコードを保存する。

### BD-046

`.verify/config.yaml` は設定（正典）である。

### BD-047

`.verify/doc/` はdocumentレコード（正典）を格納する。

### BD-048

`.verify/vo/` はVOレコード（正典）を格納する。

### BD-049

`.verify/rel/` は外部Relationレコード（正典・不変）を格納する。

### BD-050

`.verify/forms/` はForm Schema（正典）を格納する。

### BD-051

`.verify/decisions/` は判断記録（事実・追記型）を格納する。

### BD-052

`.verify/approvals/` は承認記録（事実・追記型）を格納する。

### BD-053

`.verify/evidence/` は実行証拠レコード（事実・追記型）を格納する。

### BD-054

`.verify/cache/` は派生情報（Git管理外）を格納する。

### BD-055

ファイル形式はすべてYAMLとする。

### BD-056

`cache/` 以外はGit管理対象とする。

### BD-S024 24.2 並列編集耐性の設計原則

### BD-057

1レコード＝1ファイルとする。

> 多数の AI Agent が並列で Test を追加・変更する前提。

*導出元: REQ-271, REQ-272, REQ-273, REQ-274, REQ-275, REQ-276, REQ-277, REQ-278, REQ-279*

*引用: 要件定義 §23*

### BD-058

全員が編集する中央共有台帳を持たない。

### BD-059

document / VOは1エンティティ1ファイルとする。

### BD-060

document / VOのファイル名をIDとする。

### BD-061

異なるエンティティへの並列変更は異なるファイルへの変更になる。

### BD-062

Relation・判断・承認・Evidenceの各レコードはULIDをファイル名とする新規ファイル追加のみで作成する。

### BD-063

Relation・判断・承認・Evidenceの各レコードの作成は既存ファイルの編集を伴わない。

### BD-064

Relationレコードの変更は「旧削除＋新追加」で表現する。

### BD-065

同一エンティティファイルへの並列変更が衝突した場合の解決はGitのマージに委ねる。

### BD-S025 24.3 派生情報の再構築

### BD-066

検証グラフ、逆引きインデックス、集約結果は `vtest scan` によりいつでも再構築できる。

*引用: 要件定義 NFR-004*

### BD-067

`cache/` が破損・削除されても正典は影響を受けない。

## BD-S026 26. インターフェース概要

*導出元: REQ-S049, REQ-S058*

### BD-068

MCPを本体とする。

### BD-069

CLI・CIは同じ検証の別入口とする。

### BD-S027 26.1 CLI コマンド体系

### BD-070

`vtest init` の責務は `.verify/` の初期化とする。

### BD-071

`vtest scan` の責務はスキャンと整合性検査、派生情報の再構築とする。

### BD-072

`vtest doc add / list / show` の責務はdocumentレコードの管理（derives_from・根指定を含む）とする。

### BD-073

`vtest vo add / edit / list / show / expand / approve` の責務はVOレコードの管理、組合せの実体化とする。

### BD-074

`approve` は `vtest approval create` の別名とする。

### BD-075

`vtest approval create / withdraw / show` の責務は承認レコードの生成・取消・照会とする。

### BD-076

`vtest approval create / withdraw / show` は対象種別（VO・document・判断記録）を引数に取る、承認の唯一の正典面である。

### BD-077

`vtest test create / edit / show / list / query` の責務はStructured Test Operationとする。

### BD-078

`vtest audit static` の責務は決定論的解析（oracle_presenceの不成立検出）の実行とする。

### BD-079

`vtest audit bundle / submit` の責務は判断記録（§11）のbundle生成と結果提出とする。

### BD-080

`vtest run` の責務はテスト実行とEvidence記録とする。

### BD-081

`vtest verify` の責務は検証の実行（scope指定可）とOK / NG判定とする。

### BD-082

`vtest report` の責務は詳細レポート出力（ツリー／JSON）とする。

### BD-083

`vtest doctor` の責務は整合性検査のみの実行とする。

### BD-S028 26.2 MCP ツール体系

### BD-084

MCPサーバは `vtest mcp` として起動する。

### BD-085

MCPサーバはCLIと同一のコア機能を呼び出す。

### BD-086

`scan` の対応機能はスキャンと整合性検査とする。

### BD-087

`doc_list` / `doc_get` / `doc_upsert` の対応機能はdocument管理とする。

### BD-088

`vo_list` / `vo_get` / `vo_upsert` / `vo_expand` / `vo_approve` の対応機能はVO管理とする。

### BD-089

`vo_approve` は `approval_create` の別名とする。

### BD-090

`approval_create` / `approval_withdraw` / `approval_get` の対応機能は承認レコードの生成・取消・照会とする。

### BD-091

`approval_create` / `approval_withdraw` / `approval_get` は対象種別を引数に取る、承認の唯一の正典面である。

### BD-092

`test_query` / `test_get` の対応機能はTest検索・逆引きとする。

### BD-093

`test_create` / `test_edit` の対応機能はStructured Test Operationとする。

### BD-094

`form_get` の対応機能はForm Schemaの取得とする。

### BD-095

`audit_static` の対応機能は決定論的解析とする。

### BD-096

`audit_bundle` / `audit_submit` の対応機能は判断記録プロトコルとする。

### BD-097

`run_tests` の対応機能はテスト実行とする。

### BD-098

`verify` の対応機能は検証実行とする。

### BD-099

`report` の対応機能は詳細レポート取得とする。

### BD-100

CLIとMCPは同じadapter registry composition・JSON envelope・adapter選択エラーを利用する。

## BD-S029 27. 対応範囲と adapter 境界

*導出元: R-2, R-3, REQ-S048*

### BD-101

source discovery、決定論的解析、Structured Test Operation、test runner起動、coverage計測はadapter能力として提供する。

### BD-102

core verifierを変更せずに別adapterを登録できる境界を要求する。

### BD-103

adapter追加によって共通契約・スキーマが壊れないことを設計制約とする。

### BD-104

組込production adapterは `rust-cargo` とする。

## BD-S030 28. 非機能要求への対応方針

*導出元: REQ-S051*

### BD-105

NFR-001並列性への対応は、1レコード1ファイル、ULIDファイル名、不変Relation、中央台帳の不在とする（§24.2）。

*導出元: REQ-280, REQ-281, REQ-282, REQ-283, REQ-284, REQ-285, REQ-286, REQ-287*

*引用: 要件定義 §24*

### BD-106

NFR-002再現性への対応は、Evidenceのリビジョン束縛、決定論的解析の再実行可能性、scanによる全再構築とする（§21）。

### BD-107

NFR-004再構築可能性への対応は、派生情報はcacheのみとし、正典から `vtest scan` で再構築することとする（§24.3）。

## BD-S031 29. スコープ外

*導出元: REQ-S052*

### BD-108

READMEに非関知宣言を一行入れる。

## BD-S032 0. 本書の位置付け

*導出元: P-005, SPEC-S001, SPEC-S060, SPEC-S066, DS-S001, DS-S060*

### BD-109

新設機能は既存コマンド・ツールの引数と出力で露出する。

## BD-S033 1. 実装構成

### BD-S034 1.1 ワークスペース構成

*導出元: SPEC-S063, SPEC-S066, DS-S063*

### BD-110

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

### BD-111

実装は単一バイナリ `vtest` を生成する。

### BD-112

`vtest-adapter-api` は `vtest-model` 以外の言語実装・Cargo実装へ依存しない。

### BD-113

`vtest-scan`、`vtest-audit`、`vtest-exec` はadapterを選択・委譲するorchestrationである。

### BD-114

`vtest-scan`、`vtest-audit`、`vtest-exec` は、それぞれが `syn`、`quote`、`rustc-demangle`、Cargo commandを直接所有しない。

### BD-115

`vtest-store` はForm Schemaの中立parserとcanonical保存だけを提供する。

### BD-116

組込Rust formの内容と配置は `vtest-adapter-rust` が所有する。

### BD-117

依存方向は `cli / mcp → verify / exec / audit / scan → store → model` を維持する。

### BD-118

言語固有能力の依存方向は `scan / audit / exec → adapter-rust → adapter-api → model` とする。

### BD-119

`adapter-rust → store` はForm Schemaとcanonical layoutの利用に限る。

### BD-S035 1.2 主要依存クレート

*導出元: SPEC-S063, DS-S063*

### BD-120

Rust構文解析には `syn` 2.x（features: `full`, `extra-traits`, `visit`）を使用し、`vtest-adapter-rust` が所有するAST解析に用いる。

### BD-121

Rustスパン位置の特定には `proc-macro2`（feature: `span-locations`）を使用し、`vtest-adapter-rust` が所有する編集・ハッシュ対象範囲の特定に用いる。

### BD-122

CLIには `clap` 4.x（derive）を使用する。

### BD-123

シリアライズには `serde`、`serde_json` を使用する。

### BD-124

YAMLレコードファイルの処理には `serde_yaml` を使用する。

### BD-125

レコードIDには `ulid` を使用する。

### BD-126

内容ハッシュ（SHA-256）の計算には `sha2` を使用する。

### BD-127

Rust sourceの走査には `ignore` を使用し、`vtest-adapter-rust` が所有する `.gitignore` 準拠の走査に用いる。

### BD-128

エラー処理には、ライブラリでは `thiserror`、バイナリでは `anyhow` を使用する。

### BD-129

MCPには `rmcp`（公式 Rust MCP SDK）を使用し、stdio transportとする。

### BD-130

日時の扱いには `time` を使用し、RFC 3339形式とする。

### BD-S036 1.3 内容ハッシュの定義

*導出元: SPEC-S002, SPEC-S007, SPEC-S026, SPEC-S027, SPEC-S051, DS-S002, DS-S008, DS-S025, DS-S030, DS-S031, DS-S050*

### BD-131

内容ハッシュはSHA-256を使用する。

### BD-132

coreはadapter出力と現在のsource bytesの対応を検証し、言語非依存encodingとSHA-256計算を行ってからdomain entityを具体化する。

### BD-133

adapterは、最終的な `TestEntity.content_hash` または `SourceTarget.content_hash` を返して自己確定してはならない。

### BD-134

coreはASTや言語固有構文からrangeを再計算しない。

### BD-135

静的解析は正典レコードを持たず、検証のたびに現在のsource / configから再計算する派生情報である（§7・§7.1）。

*導出元: P-003*

*引用: 基本仕様 P-003*

## BD-S037 2. データディレクトリと設定

### BD-S038 2.1 `.verify/` レイアウト

*導出元: SPEC-S008, DS-S009*

### BD-136

基本仕様 §24.1 の layout をそのまま採用する。

*引用: 基本仕様 §24.1*

### BD-137

`.verify/` 直下に `config.yaml` を置く。

### BD-138

`.verify/doc/` は `DOC-<NAME>.yaml` 形式で総称documentレコード（正典）を格納する。

### BD-139

`.verify/vo/` は `VO-<NAME>.yaml` 形式でVOレコード（正典）を格納する。

### BD-140

`.verify/rel/` は `REL-<ULID>.yaml` 形式で外部Relationレコード（正典・不変）を格納する。

### BD-141

`.verify/forms/` は `<kind>.yaml` 形式でForm Schema（正典）を格納する。

### BD-142

`.verify/decisions/` は `<ULID>.yaml` 形式で判断記録（事実・追記型）を格納する。

### BD-143

`.verify/approvals/` は `<ULID>.yaml` 形式で承認記録（事実・追記型）を格納する。

### BD-144

`.verify/evidence/` は `<ULID>.yaml` 形式で実行証拠レコード（事実・追記型）を格納する。

### BD-145

`.verify/cache/` はGit管理外とし、`.verify/.gitignore` に `cache/` を出力する。

### BD-146

`.verify/cache/bundles/` は判断バンドルJSON（派生・再生成可能）を格納する。

### BD-147

`.verify/cache/logs/` はテスト実行の生ログを格納する。

### BD-148

`.verify/cache/cov/` はcoverage生出力を格納する。

### BD-149

文書種別ごとの専用ディレクトリ（旧 `spec/` / `req/`）を設けず、上流文書はすべて `doc/` の総称documentレコード1種で表現する。

*導出元: SPEC-038, SPEC-039, SPEC-040, SPEC-041, SPEC-042, SPEC-043, SPEC-044, SPEC-277, SPEC-278, SPEC-279, SPEC-280*

*引用: 基本仕様 §3.1, 基本仕様 §3.2*

### BD-150

決定論的解析の結果を保存する正典ディレクトリ（旧 `audits/`）を設けない。

### BD-151

静的解析は再計算派生であり `cache/` にのみ置く（§7.1）。

### BD-152

外部判断は `decisions/` の判断記録として保存する。

### BD-153

`vtest init` は上記ディレクトリ、`config.yaml` の雛形、`.verify/.gitignore`、組込 Form Schema を生成する。

*引用: 別紙A §14*

### BD-S039 2.2 `config.yaml`

*導出元: SPEC-S012, SPEC-S019, SPEC-S050, SPEC-S053, DS-S007, DS-S013, DS-S018, DS-S021, DS-S049, DS-S053*

### BD-154

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

### BD-155

adapter固有設定の検証は登録adapterへ委譲する。

### BD-156

`vtest init` はversion 2を生成する。

### BD-157

`scan` と `run` はversion 1 schema互換のwire値とする。

### BD-158

Rust固有のmacro pathや `llvm-cov` 制約は `rust-cargo` adapterに限って適用する。

### BD-S040 2.3 派生情報

*導出元: P-003, SPEC-S058, DS-S059*

### BD-159

検証グラフとindexは実行のたびにインメモリで再構築する。

### BD-160

永続cacheを正典または検証入力として使用しない。

### BD-161

`cache/` は再生成可能な派生物（判断バンドル、静的解析結果、実行ログ、coverage生出力）だけを格納する。

## BD-S041 3. レコードファイルスキーマ

### BD-162

すべてのレコードはYAMLとする。

### BD-S042 3.1 document レコード（`.verify/doc/DOC-*.yaml`）

*導出元: SPEC-S008, SPEC-S009, SPEC-S044, DS-S009, DS-S010, DS-S045*

### BD-163

上流文書はすべて単一の総称ノード型 `document` で表現する。

*導出元: SPEC-038, SPEC-039, SPEC-040, SPEC-041, SPEC-042, SPEC-277, SPEC-278*

*引用: 基本仕様 §3.1*

### BD-164

要件定義・基本仕様・詳細設計・API Schema・Protocol Specification・型/データ仕様・DB schema・その他の機械可読仕様を種別で区別する専用スキーマを持たない。

### BD-165

`derives_from` は上流documentへの唯一のリンク種別である。

*導出元: SPEC-043, SPEC-044, SPEC-279, SPEC-280*

*引用: 基本仕様 §3.2*

### BD-166

文書層の段（要件→仕様→詳細設計…）は `derives_from` リンクで表現し、段を増やしても種別を増やさない。

### BD-167

リンクを追加してもスキーマは壊れない。

### BD-168

仕様文書そのものは `.verify/` へ複製しない。

### BD-S043 3.2 VO レコード（`.verify/vo/VO-*.yaml`）

*導出元: SPEC-S009, SPEC-S030, DS-S010, DS-S034*

### BD-169

VOとdocumentの間に他のエンティティ層を置かない（§3.2）。

*導出元: SPEC-012, SPEC-013, SPEC-014, SPEC-015, SPEC-016, SPEC-017, SPEC-018, SPEC-019, SPEC-020, SPEC-021, SPEC-022, SPEC-023, SPEC-024, SPEC-025, SPEC-026, SPEC-027, SPEC-028, SPEC-029, SPEC-030, SPEC-031*

*引用: 基本仕様 §1*

### BD-170

VOは旧モデルの `requirements`（REQ参照）と `spec_refs`（SPEC + 節参照）は持たず、上流参照は `derives_from:[DOC-]` へ一本化する。

### BD-171

「どの上流条項がどのVOへ対応するか」の対応ペアは、`anchor` 付き `derives_from` エッジとして保持し、§11.6のprojection出力で露出する。

*導出元: SPEC-109, SPEC-110, SPEC-111, SPEC-112, SPEC-113, SPEC-320, SPEC-321, SPEC-322, SPEC-323, SPEC-324, SPEC-325, SPEC-326, SPEC-327, SPEC-328, SPEC-329, SPEC-330, SPEC-331, SPEC-332, SPEC-333*

*引用: 基本仕様 §11.1*

#### BD-S044 3.2.1 dimensions と組合せの実体化

*導出元: SPEC-S030, DS-S034*

### BD-172

`dimensions` を持つ VO は、`vtest vo expand VO-X` により子 VO を実体化できる。

> dimensions:
>   - name: operand-sign
>     partitions: [positive, negative]
>   - name: operator
>     partitions: [add, sub, mul, div]
> coverage_policy: full-product

### BD-173

子VO生成は、生成前に一覧を提示して `--dry-run` で確認できる。

### BD-S045 3.3 Relation レコード（`.verify/rel/REL-<ULID>.yaml`）

*導出元: SPEC-S006, SPEC-S009, DS-S006, DS-S010*

### BD-174

Relationは、どちらか一方のエンティティに自然に所属しない関係（VO間の依存、Test間の補完関係など）だけを保存する。

*導出元: SPEC-037*

*引用: 基本仕様 §2.3*

### BD-175

`derives_from`・`covers`・`targets` はadapter所有の宣言またはdocument / VO recordから導出できるため、外部Relationとして重複保存しない。

### BD-S046 3.5 承認レコード（`.verify/approvals/<ULID>.yaml`）

*導出元: REQ-S046, SPEC-S016, SPEC-S043, SPEC-S066, DS-S017, DS-S044*

### BD-176

承認レコードの構造・値域・実効承認の導出・状態遷移は本節だけで定義し、対象の種別ごとに別の承認規則を置かない。

### BD-177

承認の入力経路は対象種別で分けず、対象種別を引数に取る単一の正典面に一本化する（§13.2）。

*引用: 別紙A §12.2*

### BD-178

エンティティ側に置く承認操作（`vtest vo approve` / `vo_approve`）は正典面への別名であり、独自の意味論を持たない。

### BD-179

誰がどの対象・範囲を承認できるか（approval authority）、承認ロール、必要承認数、権限schemaはプロジェクト側で定義可能とし、その具体は別紙A / プロジェクト設定へ委譲する。

*導出元: SPEC-150, SPEC-151, SPEC-152, SPEC-153, SPEC-154, SPEC-155, SPEC-230, SPEC-231, SPEC-232, SPEC-233, SPEC-234, SPEC-235, SPEC-236, SPEC-237, SPEC-238, SPEC-239, SPEC-240, SPEC-241, SPEC-242, SPEC-243, SPEC-244, SPEC-245, SPEC-246, SPEC-247, SPEC-248, SPEC-249, SPEC-250, SPEC-251, SPEC-252, SPEC-253, SPEC-254, SPEC-255, SPEC-355, SPEC-356, SPEC-357, SPEC-358*

*引用: 基本仕様 §17, 基本仕様 §30*

### BD-180

承認レコードの入力経路は別紙A §12.2・§13.2 に定める。

*引用: 別紙A §12.2, 別紙A §13.2*

### BD-S047 3.6 Evidence レコード（`.verify/evidence/<ULID>.yaml`）

*導出元: SPEC-S022, SPEC-S051, DS-S025, DS-S050*

### BD-181

Evidence内の `target` は実行時snapshotを識別するkeyであり、TEST → SRC edgeの正典ではない。

### BD-182

graphはadapter所有のTest metadata宣言からだけ構築し、Evidenceのtarget listからedgeを生成しない。

*導出元: SPEC-037*

*引用: 基本仕様 §2.3*

## BD-S048 4. Test metadata宣言contract

### BD-S049 4.1 adapter-neutralな正規化

*導出元: REQ-S026, SPEC-S007, SPEC-S027, SPEC-S028, SPEC-S035, DS-S008, DS-S031, DS-S032, DS-S038*

### BD-183

検証対象を実装construct（Source Target）として実現するか、外部から観測可能な契約・境界上の振る舞いとして実現するかは実行形態が定める。

*導出元: REQ-064, REQ-065, REQ-066, REQ-067, REQ-068, REQ-069, REQ-070, REQ-071, REQ-072, SPEC-079, SPEC-080, SPEC-081, SPEC-082, SPEC-083, SPEC-084*

*引用: 基本仕様 §8.3, 基本仕様 §9.1, 要件定義 §4.3*

### BD-184

`targets[]` は検証対象をSource Targetとして実現するためのcapability fieldであり、その要求件数はadapterが定める。

*導出元: SPEC-085, SPEC-086*

*引用: 基本仕様 §9.2*

### BD-185

非sourceの境界形態（外部契約・境界上の振る舞い）の具体的表現・確認方法は特定形態を他形態へ一律要求せず、下位仕様・後続adapter・後続版へ委譲する（本versionでContract-Target類の新schemaは設けない）。

*導出元: REQ-064, REQ-065, REQ-066, REQ-067, REQ-068, REQ-069, REQ-070, REQ-071, REQ-072, SPEC-061, SPEC-062, SPEC-063, SPEC-064, SPEC-065, SPEC-289*

*引用: 要件定義 §4.3, 基本仕様 §5.3*

### BD-186

coreはsource declarationの構文と配置を解釈しない。

### BD-187

coreはadapterが返したTest Entity、Discovered Test observation、Source Location、Target Reference、source range、診断を検証・統合する。

### BD-188

coreはpath、module、symbol種別を分解しない。

### BD-S050 4.4 宣言エラーの扱い

*導出元: SPEC-S018, SPEC-S028, SPEC-S035, DS-S020, DS-S032, DS-S038*

### BD-189

VO参照の解決とTest IDの大局的一意性はadapterではなくcoreが検査する。

### BD-190

VO解決・ID一意性・target解決はcoreが参照整合性検査で判定する（§5）。

## BD-S051 5. Discovery orchestration設計

### BD-S052 5.2 エンティティモデル（vtest-model）

*導出元: SPEC-S007, SPEC-S012, SPEC-S017, DS-S008, DS-S013, DS-S019*

### BD-191

coreは `project`、`suite.kind`、`suite.name`、`selector` の文字列を解釈しない。

### BD-192

`filter`、`package`、`test_target` および `TestTarget` 型を `vtest-model` へ置かない。

### BD-193

`vtest-adapter-api` は言語非依存の `TestWireCodec` capabilityを定義する。

### BD-194

codecはadapter固有のcompatibility propertyをJSON objectとしてencode / decodeできるが、core domain typeへadapter固有fieldを追加しない。

### BD-195

`rust-cargo` codecはversion 1互換の `filter`、`package`、`test_target` を所有する。

### BD-196

VO参照の解決とTest IDの大局的一意性はadapterではなくcoreが検査する。

### BD-197

adapter capabilityは `SourceDiscoveryAdapter`、`TestWireCodec`、`StaticAnalysisAdapter`、`StructuredTestAdapter`、`TestRunnerAdapter`、`CoverageAdapter` に分割する。

### BD-S053 5.3 検証グラフ

*導出元: SPEC-S008, SPEC-S049, DS-S009, DS-S048*

### BD-198

インメモリのグラフを構築する。

### BD-199

上流文書はすべてDOCノードとし、文書間・VO→文書は `derives_from` の一種で表現する（§19）。

*導出元: SPEC-038, SPEC-039, SPEC-040, SPEC-041, SPEC-042, SPEC-043, SPEC-044, SPEC-277, SPEC-278, SPEC-279, SPEC-280*

*引用: 基本仕様 §3.1, 基本仕様 §3.2*

### BD-S054 5.5 `rust-cargo` SourceDiscoveryAdapter

*導出元: SPEC-S063, SPEC-S066, DS-S063*

### BD-200

`vtest-scan` はこれらのRust固有処理を実行しない。

### BD-201

各管理対象Testに1件以上のSource Target（`targets ≥ 1`）を必須とすることはadapter層に属し、core中立の `chain_integrity` 必須リンクではない（§4.1・§11.1.1）。

### BD-S055 5.6 文書層 orphan_detection

*導出元: REQ-S010, SPEC-S019, DS-S021*

### BD-202

根指定は `.verify/` 設定として保持する。

*導出元: SPEC-059, SPEC-060, SPEC-287, SPEC-288*

*引用: 基本仕様 §5.2*

### BD-203

根指定の追加・削除は `vtest doc` コマンドの引数で管理し `doc.roots` へ反映する。

*導出元: SPEC-212, SPEC-213, SPEC-214*

*引用: 基本仕様 §26.1, 別紙A*

## BD-S056 6. Target Reference解決

### BD-S057 6.1 adapter-neutral解決contract

*導出元: SPEC-S010, SPEC-S028, DS-S011, DS-S032*

### BD-204

coreは`TargetRef::Locator.adapter`をregistryで解決する。

### BD-205

coreは、opaque locatorの解釈を該当する`SourceDiscoveryAdapter`へ委譲する。

### BD-206

coreはopaque locatorの内部構文は解釈しない。

### BD-207

SRC ID参照はcoreが統合済みSRC索引で一意性を検査する。

### BD-208

Target Referenceの解決はcoreの単一経路が所有し、静的解析、実行、Evidence writer、検証集約はいずれもTarget Referenceの解決の結果を消費する。

### BD-209

各subsystemが独自にcandidate列を走査して1件を選ぶ経路を持ってはならない。

## BD-S058 7. Static Analysis orchestrationと`rust-cargo`ルール

### BD-210

決定論的解析結果は正典レコードを持たず、検証のたびに現在のsource / configから再計算する派生情報である。

*導出元: P-003*

*引用: 基本仕様 P-003*

### BD-S059 7.1 判定の原則

*導出元: P-003, SPEC-S021, SPEC-S023, DS-S023, DS-S026*

### BD-211

adapter固有のASTやassertion構文をcoreは解釈しない。

### BD-S060 7.2 `rust-cargo` ルール一覧

*導出元: SPEC-S021, SPEC-S025, DS-S023, DS-S024, DS-S029*

### BD-212

静的解析は再計算派生であるため、これらのtarget別verdictと規則単位verdictは検証のたびに現在sourceから計算し、正典レコードへ永続化しない（§7.1）。

## BD-S061 8. 判断記録プロトコル

*導出元: REQ-S035, SPEC-S031, DS-S035*

### BD-S062 8.1 バンドル生成

### BD-213

`vtest audit bundle`は判断対象ごとに、判断に必要な情報をJSONとして`cache/bundles/<ULID>.json`へ出力する。

### BD-214

バンドルは派生情報でありGit管理しない。

### BD-215

提出結果の検証に必要な情報（対象の内容ハッシュ）は判断記録へ複製されるため、バンドル自体の永続化は不要である。

### BD-S063 8.4 提出の検証

### BD-216

`audit submit`は、検証に失敗した場合は§17のエラーコードで拒否する。

### BD-217

受理された提出は判断記録（§3.4）として`.verify/decisions/`へ保存される。

## BD-S064 9. テスト実行設計

*導出元: SPEC-S022, SPEC-S051, DS-S050*

### BD-S065 9.1 実行対象の解決

### BD-218

旧モデルの`--req`（REQ指定）はdocument層の総称化により廃止し、document scopeが必要な場合はVO部分木経由で指定する。

### BD-S066 9.2 `rust-cargo` TestRunnerAdapter

### BD-219

実行対象の解釈とcommand生成は`TestRunnerAdapter`が所有する。

## BD-S067 10. `rust-cargo` Target Binding 動的計測

*導出元: SPEC-S020, SPEC-S051, DS-S022, DS-S050*

### BD-S068 10.1 計測方式

### BD-220

`rust-cargo` CoverageAdapterは`cargo-llvm-cov`を使用する（adapter configの`run.coverage: llvm-cov`）。

### BD-221

Testが起動したsubprocess・spawnしたthreadの実行を宣言targetへ帰属させられるかは`rust-cargo` CoverageAdapterの能力に属する（§10.2・§7.3）。

### BD-222

coverageは独立した`CoverageAdapter` capabilityとして扱う。

### BD-S069 10.2 判定

### BD-223

coverage providerが境界越しの実行を宣言targetへ帰属させられるかはadapterのcoverage capabilityに属し、能力の有無で計測結果を捏造しない。

## BD-S070 11. 鮮度検証と集約

### BD-S071 11.3 集約アルゴリズム

*導出元: SPEC-S052, DS-S052*

### BD-224

Featureを独立のエンティティ種別・レコードファイル・ID体系・宣言fieldとして設けず、`.verify/`にFeature用ディレクトリを置かない（基本仕様 §3.1のエンティティ種別を増やさない）。

*導出元: SPEC-038, SPEC-039, SPEC-040, SPEC-041, SPEC-042, SPEC-277, SPEC-278*

*引用: 基本仕様 §3.1*

### BD-225

機能単位の表示経路（起点指定と内訳の提示）は§11.6のprojectionで露出し、新規コマンド・ツール・出力エンティティを増やさない。

### BD-S072 11.5 フェーズゲート評価

*導出元: REQ-S057, SPEC-S050, DS-S049*

### BD-226

ゲート定義は、`config.yaml`の`gates`（§2.2）に、ゲート名と進行条件（`require.verification`＝要求する検証結果、`require.approvals`＝要求する承認ロール集合）を保持する。

### BD-227

新規CLIコマンド・MCPツールを増やさず、既存の`vtest verify`の`--gate`引数と出力、および`report`のJSONでゲート評価を露出する（引数・出力schemaは別紙A）。

*引用: 別紙A*

### BD-S073 11.6 役割別 projection

*導出元: REQ-S007, SPEC-S049, SPEC-S054, DS-S048, DS-S054*

### BD-228

新規コマンド・ツールを増やさず、既存の`vtest report`のview / projection引数と、`test query`の逆引きで露出する（引数・出力schemaは別紙A）。

*引用: 別紙A*

### BD-229

「どの上流条項が、どの概念（VO）へ対応するか」の対応ペアの取得のために新規コマンド・ツールを設けない。

### BD-S074 11.7 判断待ち情報の構造

*導出元: SPEC-S047, SPEC-S066*

### BD-230

新規コマンド・ツールを増やさず、`vtest verify` / `vtest report`のJSON出力に判断待ちsectionを含めて露出する。

## BD-S075 16. 並列動作と整合性

### BD-S076 16.1 ロック不要の根拠

*導出元: SPEC-S057, DS-S058*

### BD-231

書き込み操作は上記のいずれかに分類され、ファイルロックを必要としない。

*導出元: SPEC-197, SPEC-198, SPEC-199*

*引用: 基本仕様 §24.2*

### BD-232

新規レコード追加（rel / decisions / approvals / evidence）はULIDファイル名の新規作成のみであり、並列生成は衝突しない。

*導出元: SPEC-197, SPEC-198, SPEC-199*

*引用: 基本仕様 §24.2*

### BD-233

エンティティファイル編集（doc / vo）は1エンティティ1ファイルであり、異なるエンティティの並列編集は独立であり、同一エンティティの並列編集はGitのマージ衝突として顕在化する。

*導出元: SPEC-197, SPEC-198, SPEC-199*

*引用: 基本仕様 §24.2*

### BD-234

テストコード編集は通常のソース編集と同じ扱いとする。

*導出元: SPEC-197, SPEC-198, SPEC-199*

*引用: 基本仕様 §24.2*

### BD-235

同時実行された`vtest`プロセス同士の調停は行わない。

### BD-236

「その時点の正典の読み取り」は書込みの原子的公開（基本仕様 §24.2）を前提とする。

*導出元: SPEC-197, SPEC-198, SPEC-199*

*引用: 基本仕様 §24.2*

### BD-237

テストコード編集は通常のソース編集と同じ扱いで本規定の対象外とする。

## BD-S077 12. CLI 詳細仕様

### BD-S078 12.1 共通仕様

*導出元: REQ-S009, SPEC-S012, SPEC-S013, SPEC-S055, SPEC-S078, SPEC-S096, SPEC-S106, DS-S013, DS-S014, DS-S018, DS-S055, DS-S079, DS-S085, DS-S117, DS-S127*

### BD-238

CLIの操作は登録済みadapter registryを通じて実装を選択する。

### BD-239

targetsの必須件数は adapter が定める。

### BD-S079 12.2 `vtest report`

*導出元: SPEC-S008, SPEC-S009, SPEC-S017, SPEC-S021, SPEC-S030, SPEC-S031, SPEC-S034, SPEC-S035, SPEC-S038, SPEC-S039, SPEC-S042, SPEC-S043, SPEC-S049, SPEC-S051, SPEC-S053, SPEC-S055, SPEC-S061, SPEC-S062, SPEC-S066, SPEC-S069, SPEC-S071, SPEC-S072, SPEC-S073, SPEC-S074, SPEC-S075, SPEC-S076, SPEC-S077, SPEC-S079, SPEC-S081, SPEC-S084, SPEC-S085, SPEC-S087, SPEC-S088, SPEC-S090, SPEC-S092, SPEC-S094, SPEC-S096, SPEC-S097, SPEC-S098, SPEC-S099, SPEC-S100, SPEC-S103, DS-S009, DS-S010, DS-S018, DS-S019, DS-S023, DS-S024, DS-S034, DS-S035, DS-S037, DS-S038, DS-S039, DS-S043, DS-S044, DS-S048, DS-S050, DS-S053, DS-S055, DS-S056, DS-S061, DS-S062, DS-S069, DS-S071, DS-S072, DS-S073, DS-S075, DS-S076, DS-S077, DS-S079, DS-S080, DS-S083, DS-S088, DS-S092, DS-S093, DS-S094, DS-S095, DS-S098, DS-S099, DS-S100, DS-S101, DS-S103, DS-S104, DS-S105, DS-S112, DS-S114, DS-S117, DS-S118, DS-S119, DS-S120, DS-S121, DS-S124*

### BD-240

逆引きインデックス（VO → Tests、SRC → Tests、DOC → VOs、DOC → DOCs）を projection の基盤とする。

*引用: 本冊 §5.3*

### BD-241

`report --from DOC-REQ-001 --direction down --format json` が返す対応ペア集合が要求該当箇所と対応概念のペアの構造化出力であり、この用途に新規コマンド・ツールを設けない。

### BD-S080 12.4 判断待ち情報 section（`verify` / `report` JSON）

*導出元: REQ-S043, SPEC-S047, SPEC-S066, SPEC-S100, DS-S121*

### BD-242

新規コマンド・ツールを増やさず、既存出力の section として露出する。

## BD-S081 13. MCP ツール詳細仕様

### BD-S082 13.1 共通仕様

*導出元: SPEC-S062, DS-S062*

### BD-243

transport は stdio である。

### BD-244

`rmcp` で実装する。

### BD-S083 13.3 エージェント向け利用フロー（参考）

*導出元: SPEC-S031, SPEC-S059, SPEC-S087, DS-S035, DS-S098*

### BD-245

各操作は、CLIとMCPで同じadapter registryを解決する。

## BD-S084 14. Form Schema 設計

### BD-S085 14.2 検証器

*導出元: SPEC-S041, SPEC-S078, SPEC-S083, DS-S042, DS-S085, DS-S090, DS-S092*

### BD-246

`adapter` はその Form を処理する Structured Test adapter ID である。

### BD-S086 14.3 組込フォーム

*導出元: SPEC-S041, SPEC-S063, DS-S042, DS-S063, DS-S080, DS-S081*

### BD-247

組込フォームは `rust-cargo` adapter が提供する。

### BD-248

`rust-unit-function`（§14.1）は組込 Form の1つである。

### BD-S087 14.4 テスト種別ごとのフォーム拡張

*導出元: SPEC-S035, SPEC-S041, SPEC-S078, DS-S038, DS-S042, DS-S079, DS-S085*

### BD-249

Form Schema はユーザー定義可能とし、大局的に一意な `kind` と登録済み Structured Test adapter の `adapter` ID を必須とする。

*導出元: SPEC-145, SPEC-146, SPEC-413, SPEC-414*

*引用: 本冊 §4.1・§5.2, 基本仕様 §15.4*

### BD-250

`fields` の追加・変更で API Test・CLI Test 等の質問列を定義できる（要件定義の質問テンプレート構想に対応）。

### BD-251

partition・境界値を必須入力とする種別は、該当フィールドに `required: true` を設定することで表現する。

*導出元: SPEC-145, SPEC-146, SPEC-413, SPEC-414*

*引用: 基本仕様 §15.4*

### BD-252

境界値・partition の必須入力化は組込 Form では設けず、user-defined Form Schema が指定できる。

### BD-253

他 field の回答値によって `required` が変わる cross-field 制約は導入しない。

### BD-254

Form Schema の検証は単一 field の `required` と検証器だけで閉じる。

### BD-255

user-defined Form も `kind` と owner `adapter` ID を宣言する通常の Form Schema であり、kind の大局的一意性と owner 解決の規則（§14.2）は変わらない。

*導出元: SPEC-145, SPEC-146, SPEC-413, SPEC-414*

*引用: 基本仕様 §15.4*

## BD-S088 15. Structured Test Operation adapter contract

### BD-256

Structured Edit の構文解析・再生成・selector 解釈は対応 adapter が所有する。

### BD-257

orchestration は Test ID と adapter ID で対象を一意に選択し、adapter が返す拡張範囲を単一置換として適用する。

### BD-258

production adapter として提供するのは `rust-cargo` だけである。

### BD-259

§15.1〜§15.4 は `rust-cargo` StructuredTestAdapter の構文処理を定める。

## BD-S089 18. 受入契約

### BD-S090 18.1 共通条件

*導出元: SPEC-S053, SPEC-S056, DS-S018, DS-S025, DS-S053, DS-S057*

### BD-260

CLIとMCPは同じcore処理、adapter registry、JSON envelope、診断codeを使用する。

### BD-S091 18.2 共通fixture

*導出元: SPEC-S008, SPEC-S012, SPEC-S013, SPEC-S014, SPEC-S019, SPEC-S035, SPEC-S081, SPEC-S086, DS-S009, DS-S013, DS-S014, DS-S015, DS-S021, DS-S038, DS-S082, DS-S088, DS-S097*

### BD-261

Rustの受入fixtureは、総称document、VO、登録Test、Source Target、承認記録、判断記録、Evidenceを含む小規模projectとする。

### BD-262

fixture は、正しい annotation を持つ Test を表現できる。

### BD-263

fixture は、`assert!(true)` だけの Test を表現できる。

### BD-264

fixture は、宣言 target を呼ばない Test を表現できる。

### BD-265

fixture は、結果を検証しない Test を表現できる。

### BD-266

fixture は、自己比較を行う Test を表現できる。

### BD-267

fixture は、annotation を持たない test function（W-SCAN-101、`chain_integrity = MISMATCH`、診断`MISSING`）を表現できる。

### BD-268

fixture は、`covers` を宣言しない Test（`covers` 0）を表現できる。

### BD-269

fixture は、`rust-cargo` で `targets` を宣言しない Test（E-SCAN-007、`chain_integrity = MISMATCH`、診断`MISSING`）を表現できる。

> `targets ≥ 1`は`rust-cargo` adapterの必須metadata。

*引用: 本冊 §4.4・§5.5*

### BD-270

fixture は、存在しない VO を参照する Test（E-SCAN-003、`chain_integrity = MISMATCH`）を表現できる。

### BD-271

fixture は、Test ID が衝突する Test（E-SCAN-002、`chain_integrity = MISMATCH`）を表現できる。

### BD-272

fixture は、Test construct と非隣接の metadata 宣言だけを変更した状態（Test subject hash が変化する）を表現できる。

### BD-273

fixture は、Test / 宣言 target を変更せず、実行結果を変えうる target 外 helper または local dependency だけを変更した状態（Execution State subject が変化し Evidence が STALE 化）を表現できる。

### BD-274

fixture は、`@vtest.case` を持つ table-driven Test を表現できる。

### BD-275

fixture は、複数 target を宣言し、target ごとに PASS / FAIL / UNKNOWN が異なる integration Test を表現できる。

### BD-276

fixture は、5 状態それぞれを生じる入力（`PASS` / `FAIL` / `MISMATCH` / `NO_EVIDENCE` / `UNKNOWN`）を表現できる。

### BD-277

fixture は、4 診断ラベルそれぞれを生じる入力（`MISSING` / `NOT_EXECUTED` / `NOT_CHECKED` / `STALE`）を表現できる。

### BD-278

fixture は、Test または target の hash 変更によって無効になる判断記録 / Evidence を表現できる。

### BD-279

fixture は、複数 adapter が同じ恒久 SRC ID を宣言する状態（E-SCAN-011）を表現できる。

### BD-280

fixture は、同一の Source Target を、一方の Test が locator で、他方の Test が恒久 SRC ID で宣言する状態を表現できる。

### BD-281

fixture は、同一の Test が同一 Source Target を locator と恒久 SRC ID の両方で宣言する状態（E-SCAN-005）を表現できる。

### BD-282

fixture は、Source Target construct の内側にある `@vtest.src-id` 宣言だけを付与・変更・削除した状態（construct bytes が変化し Source Target hash も変化する）を表現できる。

### BD-283

fixture は、呼出を静的に確認できない到達境界を越えて target を実行する Test（subprocess spawn型・spawn thread型）を表現できる。

### BD-284

fixture は、他ファイル・他クレートへ呼び出すが戻り値を Test 本体内で assert する Test（DA-002 UNKNOWN・DA-003 PASS）を表現できる。

### BD-285

fixture は、文書鎖の状態として `doc.roots` に列挙された根 document を表現できる。

### BD-286

fixture は、文書鎖の状態として `derives_from` が空かつ根に列挙されない孤児 document（E-SCAN-016、`orphan_detection = MISMATCH`）を表現できる。

### BD-287

fixture は、文書鎖の状態として `derives_from` の参照先が存在しない document / VO（E-SCAN-012、`chain_integrity = MISMATCH`）を表現できる。

### BD-288

fixture は、文書鎖の状態として `content_hash` と実ファイルが一致しない document（W-SCAN-104、`chain_integrity = MISMATCH`、診断 `STALE`）を表現できる。

### BD-289

fixture は、文書鎖の状態として document 再登録で失効する判断記録・承認記録を表現できる。

### BD-290

fixture は、判断記録を受理しても対象の検証状態が昇格しない状態（判断受理前後で `UNKNOWN` が `PASS` へ変わらない）を表現できる。

### BD-291

fixture は、上流依存 closure またはハッシュを欠く互換 Approval（W-STORE-002、VO は `draft` 相当）を表現できる。

### BD-292

fixture は、フェーズゲート定義（`config.yaml` の `gates`）を持ち、`vtest verify --gate <name>` が条件充足・不足の両方を提示する状態を表現できる。

### BD-293

adapter境界fixtureは、Rust parser、Cargo、llvm-covを使用しないin-process synthetic adapterを使用できる。

### BD-294

synthetic fixtureは`.rs`以外のsource、関数ではないTest construct、doc commentではないmetadata宣言、Rust item pathではないopaque locatorを使用する。

### BD-S092 18.3 機能別受入条件

#### BD-S093 18.3.1 discovery・record・graph と chain_integrity

*導出元: SPEC-S009, SPEC-S018, SPEC-S028, SPEC-S043, SPEC-S077, SPEC-S083, SPEC-S095, DS-S010, DS-S020, DS-S032, DS-S044, DS-S056, DS-S083, DS-S090, DS-S115*

### BD-295

Source Target identityは「宣言された`TargetRef` → 解決 → canonical Locator」の一方向で確定する。

### BD-296

Target Reference解決はcoreの単一経路が所有する。

### BD-297

discovery、静的解析、実行、Evidence writer、検証集約が独自にcandidate列を走査して1件を選ぶ経路を持たない。

#### BD-S094 18.3.2 orphan_detection（文書層の孤児検出）

*導出元: REQ-S010, SPEC-S019, SPEC-S081, DS-S021, DS-S088*

### BD-298

根指定の追加・削除は `vtest doc` コマンドの引数で管理する。

*導出元: SPEC-212, SPEC-213, SPEC-214*

*引用: 基本仕様 §26.1*

#### BD-S095 18.3.3 決定論的静的解析（oracle_presence・target_binding 静的到達）

*導出元: SPEC-S021, SPEC-S025, SPEC-S085, SPEC-S086, DS-S023, DS-S024, DS-S029, DS-S094, DS-S095, DS-S097*

### BD-299

静的解析は正典レコードを持たない再計算派生であり、検証のたびに現在のsource / configから再計算する。

*導出元: P-003*

*引用: 本冊 §7.1, 基本仕様 P-003*

### BD-300

表示scopeと内部依存評価を分離する。

#### BD-S096 18.3.6 判断記録プロトコル（非ゲート）

*導出元: REQ-S035, SPEC-S031, SPEC-S034, SPEC-S087, DS-S035, DS-S037, DS-S098*

### BD-301

`vtest audit bundle` は判断対象ごとに、判断に必要な情報（対象 VO と claim、Test Intent、テストコード、対象実装、関連テスト、既知 partition、過去の判断、対象の内容ハッシュとリビジョン）を JSON として `cache/bundles/` へ出力する。

### BD-302

バンドルは派生情報であり Git 管理しない。

*導出元: SPEC-116, SPEC-117, SPEC-118, SPEC-119, SPEC-120, SPEC-121, SPEC-122, SPEC-334, SPEC-335, SPEC-336*

*引用: 本冊 §8.1, 基本仕様 §11.3*

### BD-303

deterministic 結果（§18.3.3 の静的解析）と agent / human の判断結果を区別して保存・表示する。

#### BD-S097 18.3.7 承認と判断記録の分離

*導出元: SPEC-S016, SPEC-S034, SPEC-S043, SPEC-S074, SPEC-S075, DS-S017, DS-S037, DS-S044, DS-S075, DS-S076*

### BD-304

承認レコード生成の正典面は対象種別を引数に取る単一の経路である。

### BD-305

`vtest approval create --subject-type <vo|document|judgment> --subject-id <id>` および `approval_create`（`subject: { type, id }`）だけが承認レコードを生成し、`vtest vo approve` / `vo_approve` は同経路への別名として同一のレコード・同一の拒否条件を持つ。

### BD-306

対象種別ごとに別の承認規則・別の承認コマンドを設けない。

*引用: 本冊 §3.5, 別紙A §12.2・§13.2*

### BD-307

`vtest approval withdraw <approval-id>` / `approval_withdraw` は `state: withdrawn` かつ `supersedes: [approval-id]` の `create` と同一のレコードを生成する。

### BD-308

`vtest approval show` / `approval_get` は当該対象の承認レコード一覧と実効承認状態（`draft` / `approved`）を返す。

#### BD-S098 18.3.8 verify・report と scope

*導出元: SPEC-S017, SPEC-S053, SPEC-S054, SPEC-S055, SPEC-S069, SPEC-S094, SPEC-S096, DS-S018, DS-S019, DS-S053, DS-S054, DS-S055, DS-S069, DS-S114, DS-S117*

### BD-309

scope は 2 軸で限定できる。

*導出元: P-002*

*引用: 基本仕様 §4.6, 要件定義 P-002*

#### BD-S099 18.3.12 adapter contract

*導出元: SPEC-S063, SPEC-S078, SPEC-S105, DS-S007, DS-S063, DS-S085, DS-S126*

### BD-310

`rust-cargo` adapterはRust discovery、static audit、Structured Test Operation、runner、coverageを所有する。

### BD-311

`vtest-scan`はadapter discoveryの委譲・出力検証・決定論的統合・core record整合性を所有し、`*.rs`列挙、`syn::parse_file`、`#[test]`抽出、doc comment parseを所有しない。
