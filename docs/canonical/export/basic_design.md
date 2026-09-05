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

## BD-S002 2. 全体像

### BD-S003 2.1 正典の三層構造

*導出元: P-003, REQ-S019, REQ-S035, REQ-S046*

### BD-008

宣言層は、adapter所有のTest metadata宣言、および.verify/配下のdocument / VO / Relationレコードからなり、Gitで管理される正典である。

### BD-009

事実層は、実行結果・判断記録・承認記録からなる.verify/配下の追記型レコードファイルであり、Gitで管理される。

### BD-010

source discovery、決定論的解析、Structured Test Operation、test runner起動、coverage計測はadapter capabilityとして提供する。

### BD-S004 2.4 adapter 設定と wire 互換

*導出元: REQ-S048, REQ-S058*

### BD-011

`config.yaml` writerの正規形はversion 2とする。

### BD-012

`config.yaml` writerはadapterごとにroot・scan・run設定をnamespace化する。

### BD-013

readerはversion 1を単一の `rust-cargo` adapter設定としてin-memory変換して読み取る。

### BD-014

`vtest init` はversion 2を生成する。

### BD-015

Test JSONのwire compatibility layerは `execution` を常に出力する。

### BD-016

Test JSONのwire compatibility layerは `rust-cargo` Testについてだけversion 1互換fieldを追加出力できる。

### BD-017

非Rust Testでは version 1互換fieldを省略する。

### BD-018

`targets` listを常に出力する。

### BD-019

単数互換field `target` はtarget 1件のときだけ追加出力する。

## BD-S005 3. エンティティと ID 体系

### BD-S006 3.1 エンティティ種別

*導出元: REQ-S005, REQ-S025, REQ-S035, REQ-S036, REQ-S040, REQ-S046*

### BD-020

documentのIDは `DOC-` とし、正典は `.verify/doc/` に置く。

### BD-021

Verification ObligationのIDは `VO-` とし、正典は `.verify/vo/` に置く。

### BD-022

TestのIDは `TEST-` とし、正典はadapter所有のTest metadata宣言とする。

### BD-023

Source TargetはIDを持たない、または任意で `SRC-` を用い、adapter IDとopaque locatorで識別する。

### BD-024

RelationのIDは `REL-`（ULID）とし、正典は `.verify/rel/` に置く。

### BD-025

判断記録のIDはULIDとし、正典は `.verify/decisions/` に置く。

### BD-026

承認記録のIDはULIDとし、正典は `.verify/approvals/` に置く。

### BD-027

Execution EvidenceのIDはULIDとし、正典は `.verify/evidence/` に置く。

### BD-S007 3.2 ID 規則と関係リンク

*導出元: REQ-S007, REQ-S027, REQ-S058*

### BD-028

DOC / VO / TESTのIDは人間可読な形式とする。

### BD-029

DOC / VO / TESTのIDは利用者（人間またはAI）が命名する。

### BD-030

IDの文字集合は `[A-Z0-9-]` とする。

### BD-031

IDの接頭辞は種別ごとに固定する（`TEST-` 等）。

### BD-032

IDの推奨形式は `TEST-<領域>-<連番>`（例：`TEST-PARSER-044`）とする。

### BD-033

Relation writerは `REL-<ULID>` を正規IDとしてファイル名に用いる。

### BD-034

readerはversion 1互換入力としてbare ULIDを `REL-<ULID>` へin-memory正規化する。

### BD-035

判断・承認・EvidenceのIDはbare ULIDとする。

### BD-S008 3.3 Source Target の識別

*導出元: R-3, REQ-S027*

### BD-036

opaque locatorの構文と恒久SRC IDの宣言方法はadapterが定める。

### BD-037

Test→SRCの対応はadapter所有のTest metadata宣言から提供する。

### BD-038

SRC→Testの逆引きはスキャン結果から提供する。

## BD-S009 5. 検査

### BD-S010 5.2 orphan_detection — 文書層の孤児検出

*導出元: REQ-S010*

### BD-039

根の指定方式は `.verify/` 設定における明示的な根指定として保持する。

### BD-S011 5.5 決定論的に検出可能な不成立構造

*導出元: REQ-S024*

### BD-040

coreはadapter固有のAST・assertion構文・call graphを解釈しない。

### BD-041

coreは正規化されたルール結果を検証・集約する。

### BD-042

code fragmentの具体構文はadapterの言語・runnerに従う。

## BD-S012 6. 証拠

*導出元: REQ-S019*

### BD-043

adapterが最終内容ハッシュを自己確定してはならない。

## BD-S013 7. 判定権威

*導出元: REQ-S020, REQ-S054*

### BD-044

`rust-cargo` adapterにおける判定権威は `cargo test` である。

## BD-S014 8. Test の検証成立性

*導出元: REQ-S021, REQ-S025*

### BD-S015 8.3 決定論的に検出可能な不成立構造

### BD-045

各adapterは対応する言語・runnerの構造に対して決定論的に判定できる範囲を提供する。

## BD-S016 9. 検証対象と Source Target

*導出元: REQ-S025*

### BD-S017 9.2 Source Target の識別

### BD-046

各adapterはSource Targetを一意に解決でき、同一source stateから決定論的に正規化できるTarget Referenceを提供する。

## BD-S018 14. Parameterized / Table-Driven Test

*導出元: R-3, REQ-S038*

### BD-047

code fragmentの具体構文はadapterの言語・runnerに従う。

*導出元: R-3, REQ-206, REQ-207, REQ-208, REQ-209*

*引用: 要件定義 §15、R-3*

## BD-S019 15. Structured Test Operation

*導出元: P-004, REQ-S039*

### BD-048

Test操作の公式経路として、Test IDまたはadapterが識別可能なTest constructを対象とした構造化操作を提供する。

*導出元: P-004, REQ-210, REQ-211, REQ-212, REQ-213, REQ-214, REQ-215, REQ-216, REQ-217*

*引用: 要件定義 §16、P-004*

### BD-049

Create Testは、Form Schemaに基づく構造化入力をadapterへ渡し、Test constructと対応するmetadata宣言を生成する。

### BD-050

Edit Testは、Test IDを編集ハンドルとして、adapterが識別する対象Testのmetadata宣言およびTest constructを更新する。

### BD-051

Query Testは、Test ID・VO・Target Reference等からの検索と逆引きを行う。

### BD-052

Audit（判断）Testは、§11の判断記録bundle生成と判断結果の提出を行う。

### BD-S020 15.4 Form Schema

### BD-053

テスト種別ごとの質問・入力項目テンプレートをForm Schemaとして `.verify/forms/` に定義できる。

### BD-054

`rust-cargo` adapterが組込schemaを登録する。

### BD-055

Form Schemaの `kind` はrepository内で大局的に一意なForm IDとする。

### BD-056

schemaはそれを処理するadapter IDを別fieldで宣言する。

## BD-S021 18. 途中導入と既存プロジェクト対応

*導出元: R-5, REQ-S040*

### BD-S022 18.1 既存資産の可視化

### BD-057

`vtest init` は `.verify/` を作成する。

### BD-058

`vtest init` は既存コードを変更しない。

### BD-059

`vtest scan` は発見した未登録Testを未登録として報告する。

### BD-060

`vtest verify` は正典または検証事実の欠落を対応する非 `PASS` 値として表示する。

### BD-061

`vtest verify` は部分的な登録・判断・実行状態を総合 `OK` として扱わない。

## BD-S023 21. テスト実行と Execution Evidence

*導出元: REQ-S019, REQ-S054*

### BD-062

`vtest run` はテストを実際に実行する。

### BD-063

`vtest run` は判定権威（§7）であるランナーの結果をEvidenceとして記録する。

*導出元: REQ-115, REQ-116, REQ-117, REQ-118, REQ-119, REQ-120, REQ-295, REQ-296*

*引用: 要件定義 §6、§26.1*

### BD-S024 21.1 Evidence の鮮度（ハッシュ束縛による設計制約）

### BD-064

Evidence readerはadapter IDを欠く互換recordも履歴として読み取れる。

### BD-065

Evidence readerは、現在のTestが `rust-cargo` で互換runner情報と内容ハッシュからRust実行と一意に確認できる場合に限り評価する。

### BD-066

Evidence readerは、Rust実行と一意に確認できない場合は `UNKNOWN` とする。

## BD-S025 23. スキャンと整合性検査

*導出元: REQ-S036, REQ-S050*

### BD-067

`vtest scan` はregistryに登録された全source discovery adapterへ委譲する。

### BD-068

`vtest scan` は統合したdiscovery結果と `.verify/` からエンティティと関係の全体グラフを再構築する。

### BD-069

`vtest scan` はその過程で `chain_integrity` / `orphan_detection` を構成する整合性検査を行う。

*導出元: REQ-193, REQ-194, REQ-195, REQ-196, REQ-197, REQ-198, REQ-199, REQ-200, REQ-201, REQ-202, REQ-271, REQ-272, REQ-273, REQ-274, REQ-275, REQ-276, REQ-277, REQ-278, REQ-279*

*引用: 要件定義 §13、§23*

## BD-S026 24. データ保存の基本方針

*導出元: REQ-S050, REQ-S058*

### BD-S027 24.1 `.verify/` ディレクトリ

### BD-070

プロジェクトルート直下に `.verify/` を置く。

### BD-071

`.verify/` にテストコード外の正典と事実レコードを保存する。

### BD-072

`.verify/config.yaml` は設定（正典）である。

### BD-073

`.verify/doc/` はdocumentレコード（正典）を格納する。

### BD-074

`.verify/vo/` はVOレコード（正典）を格納する。

### BD-075

`.verify/rel/` は外部Relationレコード（正典・不変）を格納する。

### BD-076

`.verify/forms/` はForm Schema（正典）を格納する。

### BD-077

`.verify/decisions/` は判断記録（事実・追記型）を格納する。

### BD-078

`.verify/approvals/` は承認記録（事実・追記型）を格納する。

### BD-079

`.verify/evidence/` は実行証拠レコード（事実・追記型）を格納する。

### BD-080

`.verify/cache/` は派生情報（Git管理外）を格納する。

### BD-081

ファイル形式はすべてYAMLとする。

### BD-082

`cache/` 以外はGit管理対象とする。

### BD-S028 24.2 並列編集耐性の設計原則

### BD-083

1レコード＝1ファイルとする。

> 多数の AI Agent が並列で Test を追加・変更する前提。

*導出元: REQ-271, REQ-272, REQ-273, REQ-274, REQ-275, REQ-276, REQ-277, REQ-278, REQ-279*

*引用: 要件定義 §23*

### BD-084

全員が編集する中央共有台帳を持たない。

### BD-085

document / VOは1エンティティ1ファイルとする。

### BD-086

document / VOのファイル名をIDとする。

### BD-087

異なるエンティティへの並列変更は異なるファイルへの変更になる。

### BD-088

Relation・判断・承認・Evidenceの各レコードはULIDをファイル名とする新規ファイル追加のみで作成する。

### BD-089

Relation・判断・承認・Evidenceの各レコードの作成は既存ファイルの編集を伴わない。

### BD-090

Relationレコードの変更は「旧削除＋新追加」で表現する。

### BD-091

同一エンティティファイルへの並列変更が衝突した場合の解決はGitのマージに委ねる。

## BD-S029 26. インターフェース概要

*導出元: REQ-S049, REQ-S058*

### BD-092

MCPを本体とする。

### BD-093

CLI・CIは同じ検証の別入口とする。

### BD-S030 26.1 CLI コマンド体系

### BD-094

`vtest init` の責務は `.verify/` の初期化とする。

### BD-095

`vtest scan` の責務はスキャンと整合性検査、派生情報の再構築とする。

### BD-096

`vtest doc add / list / show` の責務はdocumentレコードの管理（derives_from・根指定を含む）とする。

### BD-097

`vtest vo add / edit / list / show / expand / approve` の責務はVOレコードの管理、組合せの実体化とする。

### BD-098

`approve` は `vtest approval create` の別名とする。

### BD-099

`vtest approval create / withdraw / show` の責務は承認レコードの生成・取消・照会とする。

### BD-100

`vtest approval create / withdraw / show` は対象種別（VO・document・判断記録）を引数に取る、承認の唯一の正典面である。

### BD-101

`vtest test create / edit / show / list / query` の責務はStructured Test Operationとする。

### BD-102

`vtest audit static` の責務は決定論的解析（oracle_presenceの不成立検出）の実行とする。

### BD-103

`vtest audit bundle / submit` の責務は判断記録（§11）のbundle生成と結果提出とする。

### BD-104

`vtest run` の責務はテスト実行とEvidence記録とする。

### BD-105

`vtest verify` の責務は検証の実行（scope指定可）とOK / NG判定とする。

### BD-106

`vtest report` の責務は詳細レポート出力（ツリー／JSON）とする。

### BD-107

`vtest doctor` の責務は整合性検査のみの実行とする。

### BD-108

終了コードは `0`＝要求scopeがOK、`1`＝検証NG、`2`＝入力・adapter前提・capability等による操作拒否、`3`＝内部エラーとする。

### BD-109

フェーズゲートを指定した実行（§20）では、`0` / `1` は当該ゲートの充足・不充足を表す。

### BD-110

ゲート指定時の `0` を検証状態 `PASS` と読ませない。

### BD-111

検証状態と内部エラーは終了コードで分離する（§4.4）。

### BD-112

CIはこの終了コードのみで判定できる。

### BD-S031 26.2 MCP ツール体系

### BD-113

MCPサーバは `vtest mcp` として起動する。

### BD-114

`scan` の対応機能はスキャンと整合性検査とする。

### BD-115

`doc_list` / `doc_get` / `doc_upsert` の対応機能はdocument管理とする。

### BD-116

`vo_list` / `vo_get` / `vo_upsert` / `vo_expand` / `vo_approve` の対応機能はVO管理とする。

### BD-117

`vo_approve` は `approval_create` の別名とする。

### BD-118

`approval_create` / `approval_withdraw` / `approval_get` の対応機能は承認レコードの生成・取消・照会とする。

### BD-119

`approval_create` / `approval_withdraw` / `approval_get` は対象種別を引数に取る、承認の唯一の正典面である。

### BD-120

`test_query` / `test_get` の対応機能はTest検索・逆引きとする。

### BD-121

`test_create` / `test_edit` の対応機能はStructured Test Operationとする。

### BD-122

`form_get` の対応機能はForm Schemaの取得とする。

### BD-123

`audit_static` の対応機能は決定論的解析とする。

### BD-124

`audit_bundle` / `audit_submit` の対応機能は判断記録プロトコルとする。

### BD-125

`run_tests` の対応機能はテスト実行とする。

### BD-126

`verify` の対応機能は検証実行とする。

### BD-127

`report` の対応機能は詳細レポート取得とする。

## BD-S032 27. 対応範囲と adapter 境界

*導出元: R-2, R-3, REQ-S048*

### BD-128

source discovery、決定論的解析、Structured Test Operation、test runner起動、coverage計測はadapter能力として提供する。

### BD-129

core verifierを変更せずに別adapterを登録できる境界を要求する。

### BD-130

adapter追加によって共通契約・スキーマが壊れないことを設計制約とする。

### BD-131

組込production adapterは `rust-cargo` とする。

## BD-S033 29. スコープ外

*導出元: REQ-S052*

### BD-132

READMEに非関知宣言を一行入れる。

## BD-S034 1. 実装構成

### BD-S035 1.1 ワークスペース構成

*導出元: SPEC-S070, SPEC-S073, DS-S051*

### BD-133

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

### BD-134

実装は単一バイナリ `vtest` を生成する。

### BD-135

`vtest-adapter-api` は `vtest-model` 以外の言語実装・Cargo実装へ依存しない。

### BD-136

`vtest-scan`、`vtest-audit`、`vtest-exec` はadapterを選択・委譲するorchestrationである。

### BD-137

`vtest-scan`、`vtest-audit`、`vtest-exec` は、それぞれが `syn`、`quote`、`rustc-demangle`、Cargo commandを直接所有しない。

### BD-138

`vtest-store` はForm Schemaの中立parserとcanonical保存だけを提供する。

### BD-139

組込Rust formの内容と配置は `vtest-adapter-rust` が所有する。

### BD-140

依存方向は `cli / mcp → verify / exec / audit / scan → store → model` を維持する。

### BD-141

言語固有能力の依存方向は `scan / audit / exec → adapter-rust → adapter-api → model` とする。

### BD-142

`adapter-rust → store` はForm Schemaとcanonical layoutの利用に限る。

### BD-S036 1.2 主要依存クレート

*導出元: SPEC-S070, DS-S051*

### BD-143

Rust構文解析には `syn` 2.x（features: `full`, `extra-traits`, `visit`）を使用し、`vtest-adapter-rust` が所有するAST解析に用いる。

### BD-144

Rustスパン位置の特定には `proc-macro2`（feature: `span-locations`）を使用し、`vtest-adapter-rust` が所有する編集・ハッシュ対象範囲の特定に用いる。

### BD-145

CLIには `clap` 4.x（derive）を使用する。

### BD-146

シリアライズには `serde`、`serde_json` を使用する。

### BD-147

YAMLレコードファイルの処理には `serde_yaml` を使用する。

### BD-148

レコードIDには `ulid` を使用する。

### BD-149

内容ハッシュ（SHA-256）の計算には `sha2` を使用する。

### BD-150

Rust sourceの走査には `ignore` を使用し、`vtest-adapter-rust` が所有する `.gitignore` 準拠の走査に用いる。

### BD-151

エラー処理には、ライブラリでは `thiserror`、バイナリでは `anyhow` を使用する。

### BD-152

MCPには `rmcp`（公式 Rust MCP SDK）を使用し、stdio transportとする。

### BD-153

日時の扱いには `time` を使用し、RFC 3339形式とする。

### BD-S037 1.3 内容ハッシュの定義

*導出元: SPEC-S002, SPEC-S008, SPEC-S025, SPEC-S031, SPEC-S032, SPEC-S056, DS-S003, DS-S016, DS-S021, DS-S022, DS-S038*

### BD-154

内容ハッシュはSHA-256を使用する。

### BD-155

adapterは、最終的な `TestEntity.content_hash` または `SourceTarget.content_hash` を返して自己確定してはならない。

### BD-156

coreはASTや言語固有構文からrangeを再計算しない。

### BD-157

静的解析は正典レコードを持たず、検証のたびに現在のsource / configから再計算する派生情報である（§7・§7.1）。

*導出元: P-003*

*引用: 基本仕様 P-003*

## BD-S038 2. データディレクトリと設定

### BD-S039 2.1 `.verify/` レイアウト

*導出元: SPEC-S009*

### BD-158

`.verify/` 直下に `config.yaml` を置く。

### BD-159

`.verify/doc/` は `DOC-<NAME>.yaml` 形式で総称documentレコード（正典）を格納する。

### BD-160

`.verify/vo/` は `VO-<NAME>.yaml` 形式でVOレコード（正典）を格納する。

### BD-161

`.verify/rel/` は `REL-<ULID>.yaml` 形式で外部Relationレコード（正典・不変）を格納する。

### BD-162

`.verify/forms/` は `<kind>.yaml` 形式でForm Schema（正典）を格納する。

### BD-163

`.verify/decisions/` は `<ULID>.yaml` 形式で判断記録（事実・追記型）を格納する。

### BD-164

`.verify/approvals/` は `<ULID>.yaml` 形式で承認記録（事実・追記型）を格納する。

### BD-165

`.verify/evidence/` は `<ULID>.yaml` 形式で実行証拠レコード（事実・追記型）を格納する。

### BD-166

`.verify/cache/` はGit管理外とし、`.verify/.gitignore` に `cache/` を出力する。

### BD-167

`.verify/cache/bundles/` は判断バンドルJSON（派生・再生成可能）を格納する。

### BD-168

`.verify/cache/logs/` はテスト実行の生ログを格納する。

### BD-169

`.verify/cache/cov/` はcoverage生出力を格納する。

### BD-170

文書種別ごとの専用ディレクトリ（旧 `spec/` / `req/`）を設けず、上流文書はすべて `doc/` の総称documentレコード1種で表現する。

*導出元: SPEC-092, SPEC-093, SPEC-094, SPEC-095, SPEC-096, SPEC-097, SPEC-098, SPEC-099, SPEC-100, SPEC-101, SPEC-102, SPEC-103, SPEC-104, SPEC-105, SPEC-106, SPEC-107, SPEC-108, SPEC-109, SPEC-110, SPEC-111, SPEC-112, SPEC-113, SPEC-437, SPEC-438, SPEC-439, SPEC-440*

*引用: 基本仕様 §3.1, 基本仕様 §3.2*

### BD-171

決定論的解析の結果を保存する正典ディレクトリ（旧 `audits/`）を設けない。

### BD-172

静的解析は再計算派生であり `cache/` にのみ置く（§7.1）。

### BD-173

外部判断は `decisions/` の判断記録として保存する。

### BD-174

`vtest init` は上記ディレクトリ、`config.yaml` の雛形、`.verify/.gitignore`、組込 Form Schema を生成する。

*引用: 別紙A §14*

### BD-S040 2.2 `config.yaml`

*導出元: SPEC-S007, SPEC-S013, SPEC-S018, SPEC-S021, SPEC-S055, SPEC-S059, DS-S002, DS-S006, DS-S009, DS-S012, DS-S041*

### BD-175

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

### BD-176

`config.yaml` readerはversion 1を単一の `rust-cargo` adapter設定としてin-memory変換して読み取るが、読み取りだけで正典を書き換えない。

*導出元: SPEC-091*

*引用: 基本仕様 §2.4*

### BD-177

adapter固有設定の検証は登録adapterへ委譲する。

### BD-178

`vtest init` はversion 2を生成する。

### BD-179

`scan` と `run` はversion 1 schema互換のwire値とする。

### BD-180

Rust固有のmacro pathや `llvm-cov` 制約は `rust-cargo` adapterに限って適用する。

### BD-S041 2.3 派生情報

*導出元: P-003, SPEC-S065, DS-S047*

### BD-181

検証グラフとindexは実行のたびにインメモリで再構築する。

### BD-182

永続cacheを正典または検証入力として使用しない。

### BD-183

`cache/` は再生成可能な派生物（判断バンドル、静的解析結果、実行ログ、coverage生出力）だけを格納する。

## BD-S042 3. レコードファイルスキーマ

### BD-184

すべてのレコードはYAMLとする。

### BD-S043 3.1 document レコード（`.verify/doc/DOC-*.yaml`）

*導出元: SPEC-S009, SPEC-S010, SPEC-S049, DS-S004, DS-S035*

### BD-185

上流文書はすべて単一の総称ノード型 `document` で表現する。

*導出元: SPEC-092, SPEC-093, SPEC-094, SPEC-095, SPEC-096, SPEC-097, SPEC-098, SPEC-099, SPEC-100, SPEC-101, SPEC-102, SPEC-103, SPEC-104, SPEC-105, SPEC-106, SPEC-437, SPEC-438*

*引用: 基本仕様 §3.1*

### BD-186

要件定義・基本仕様・詳細設計・API Schema・Protocol Specification・型/データ仕様・DB schema・その他の機械可読仕様を種別で区別する専用スキーマを持たない。

### BD-187

`derives_from` は上流documentへの唯一のリンク種別である。

*導出元: SPEC-107, SPEC-108, SPEC-109, SPEC-110, SPEC-111, SPEC-112, SPEC-113, SPEC-439, SPEC-440*

*引用: 基本仕様 §3.2*

### BD-188

文書層の段（要件→仕様→詳細設計…）は `derives_from` リンクで表現し、段を増やしても種別を増やさない。

### BD-189

リンクを追加してもスキーマは壊れない。

### BD-190

仕様文書そのものは `.verify/` へ複製しない。

### BD-S044 3.2 VO レコード（`.verify/vo/VO-*.yaml`）

*導出元: SPEC-S010, SPEC-S035, DS-S004, DS-S025*

### BD-191

VOとdocumentの間に他のエンティティ層を置かない（§3.2）。

*導出元: SPEC-016, SPEC-017, SPEC-018, SPEC-019, SPEC-020, SPEC-021, SPEC-022, SPEC-023, SPEC-024, SPEC-025, SPEC-026, SPEC-027, SPEC-028, SPEC-029, SPEC-030, SPEC-031, SPEC-032, SPEC-033, SPEC-034, SPEC-035, SPEC-036, SPEC-037, SPEC-038, SPEC-039, SPEC-040, SPEC-041, SPEC-042, SPEC-043, SPEC-044, SPEC-045, SPEC-046, SPEC-047, SPEC-048, SPEC-049, SPEC-050, SPEC-051, SPEC-052, SPEC-053, SPEC-054, SPEC-055, SPEC-056, SPEC-057, SPEC-058, SPEC-059, SPEC-060, SPEC-061, SPEC-062, SPEC-434*

*引用: 基本仕様 §1*

### BD-192

VOは旧モデルの `requirements`（REQ参照）と `spec_refs`（SPEC + 節参照）は持たず、上流参照は `derives_from:[DOC-]` へ一本化する。

### BD-193

「どの上流条項がどのVOへ対応するか」の対応ペアは、`anchor` 付き `derives_from` エッジとして保持し、§11.6のprojection出力で露出する。

*導出元: SPEC-220, SPEC-221, SPEC-222, SPEC-223, SPEC-224, SPEC-494, SPEC-495, SPEC-496, SPEC-497, SPEC-498, SPEC-499, SPEC-500, SPEC-501, SPEC-502, SPEC-503, SPEC-504, SPEC-505, SPEC-506, SPEC-507*

*引用: 基本仕様 §11.1*

#### BD-S045 3.2.1 dimensions と組合せの実体化

*導出元: SPEC-S035, DS-S025*

### BD-194

`dimensions` を持つ VO は、`vtest vo expand VO-X` により子 VO を実体化できる。

> dimensions:
>   - name: operand-sign
>     partitions: [positive, negative]
>   - name: operator
>     partitions: [add, sub, mul, div]
> coverage_policy: full-product

### BD-195

子VO生成は、生成前に一覧を提示して `--dry-run` で確認できる。

### BD-S046 3.3 Relation レコード（`.verify/rel/REL-<ULID>.yaml`）

*導出元: SPEC-S006, SPEC-S010, DS-S004*

### BD-196

Relationは、どちらか一方のエンティティに自然に所属しない関係（VO間の依存、Test間の補完関係など）だけを保存する。

*導出元: SPEC-084, SPEC-085, SPEC-086, SPEC-087, SPEC-088, SPEC-089, SPEC-090*

*引用: 基本仕様 §2.3*

### BD-197

`derives_from`・`covers`・`targets` はadapter所有の宣言またはdocument / VO recordから導出できるため、外部Relationとして重複保存しない。

### BD-S047 3.5 承認レコード（`.verify/approvals/<ULID>.yaml`）

*導出元: REQ-S046, SPEC-S017, SPEC-S048, SPEC-S073, DS-S008, DS-S034*

### BD-198

承認レコードの構造・値域・実効承認の導出・状態遷移は本節だけで定義し、対象の種別ごとに別の承認規則を置かない。

### BD-199

承認の入力経路は対象種別で分けず、対象種別を引数に取る単一の正典面に一本化する（§13.2）。

*引用: 別紙A §12.2*

### BD-200

エンティティ側に置く承認操作（`vtest vo approve` / `vo_approve`）は正典面への別名であり、独自の意味論を持たない。

### BD-201

誰がどの対象・範囲を承認できるか（approval authority）、承認ロール、必要承認数、権限schemaはプロジェクト側で定義可能とし、その具体は別紙A / プロジェクト設定へ委譲する。

*導出元: SPEC-267, SPEC-268, SPEC-269, SPEC-270, SPEC-271, SPEC-272, SPEC-273, SPEC-274, SPEC-275, SPEC-276, SPEC-277, SPEC-278, SPEC-279, SPEC-280, SPEC-281, SPEC-387, SPEC-388, SPEC-389, SPEC-390, SPEC-391, SPEC-392, SPEC-393, SPEC-394, SPEC-395, SPEC-396, SPEC-397, SPEC-398, SPEC-399, SPEC-400, SPEC-401, SPEC-402, SPEC-403, SPEC-404, SPEC-405, SPEC-406, SPEC-407, SPEC-408, SPEC-409, SPEC-410, SPEC-411, SPEC-412, SPEC-529, SPEC-530, SPEC-531, SPEC-532, SPEC-533*

*引用: 基本仕様 §17, 基本仕様 §30*

### BD-202

承認レコードの入力経路は別紙A §12.2・§13.2 に定める。

*引用: 別紙A §12.2, 別紙A §13.2*

### BD-S048 3.6 Evidence レコード（`.verify/evidence/<ULID>.yaml`）

*導出元: SPEC-S025, SPEC-S026, SPEC-S056, DS-S016, DS-S038*

### BD-203

Evidence内の `target` は実行時snapshotを識別するkeyであり、TEST → SRC edgeの正典ではない。

### BD-204

graphはadapter所有のTest metadata宣言からだけ構築し、Evidenceのtarget listからedgeを生成しない。

*導出元: SPEC-084, SPEC-085, SPEC-086, SPEC-087, SPEC-088, SPEC-089, SPEC-090*

*引用: 基本仕様 §2.3*

## BD-S049 4. Test metadata宣言contract

### BD-S050 4.1 adapter-neutralな正規化

*導出元: REQ-S026, SPEC-S008, SPEC-S032, SPEC-S033, SPEC-S040, DS-S003, DS-S022, DS-S023, DS-S028*

### BD-205

検証対象を実装construct（Source Target）として実現するか、外部から観測可能な契約・境界上の振る舞いとして実現するかは実行形態が定める。

*導出元: REQ-064, REQ-065, REQ-066, REQ-067, REQ-068, REQ-069, REQ-070, REQ-071, REQ-072, SPEC-183, SPEC-184, SPEC-185, SPEC-186, SPEC-187, SPEC-188, SPEC-189, SPEC-190, SPEC-477, SPEC-478*

*引用: 基本仕様 §8.3, 基本仕様 §9.1, 要件定義 §4.3*

### BD-206

`targets[]` は検証対象をSource Targetとして実現するためのcapability fieldであり、その要求件数はadapterが定める。

*導出元: SPEC-191, SPEC-192, SPEC-193, SPEC-194, SPEC-195*

*引用: 基本仕様 §9.2*

### BD-207

非sourceの境界形態（外部契約・境界上の振る舞い）の具体的表現・確認方法は特定形態を他形態へ一律要求せず、下位仕様・後続adapter・後続版へ委譲する（本versionでContract-Target類の新schemaは設けない）。

*導出元: REQ-064, REQ-065, REQ-066, REQ-067, REQ-068, REQ-069, REQ-070, REQ-071, REQ-072, SPEC-153, SPEC-154, SPEC-155, SPEC-156, SPEC-157, SPEC-158, SPEC-450*

*引用: 要件定義 §4.3, 基本仕様 §5.3*

### BD-208

coreはsource declarationの構文と配置を解釈しない。

### BD-209

coreはadapterが返したTest Entity、Discovered Test observation、Source Location、Target Reference、source range、診断を検証・統合する。

### BD-210

coreはpath、module、symbol種別を分解しない。

### BD-S051 4.4 宣言エラーの扱い

*導出元: SPEC-S020, SPEC-S033, SPEC-S040, DS-S011, DS-S023, DS-S028*

### BD-211

VO参照の解決とTest IDの大局的一意性はadapterではなくcoreが検査する。

### BD-212

VO解決・ID一意性・target解決はcoreが参照整合性検査で判定する（§5）。

## BD-S052 5. Discovery orchestration設計

### BD-S053 5.2 エンティティモデル（vtest-model）

*導出元: SPEC-S008, SPEC-S013, SPEC-S019, DS-S003, DS-S006, DS-S010*

### BD-213

coreは `project`、`suite.kind`、`suite.name`、`selector` の文字列を解釈しない。

### BD-214

`vtest-adapter-api` は言語非依存の `TestWireCodec` capabilityを定義する。

### BD-215

codecはadapter固有のcompatibility propertyをJSON objectとしてencode / decodeできるが、core domain typeへadapter固有fieldを追加しない。

### BD-216

`rust-cargo` codecはversion 1互換の `filter`、`package`、`test_target` を所有する。

### BD-217

VO参照の解決とTest IDの大局的一意性はadapterではなくcoreが検査する。

### BD-218

adapter capabilityは `SourceDiscoveryAdapter`、`TestWireCodec`、`StaticAnalysisAdapter`、`StructuredTestAdapter`、`TestRunnerAdapter`、`CoverageAdapter` に分割する。

### BD-S054 5.3 検証グラフ

*導出元: SPEC-S009, SPEC-S054, DS-S037*

### BD-219

インメモリのグラフを構築する。

### BD-220

上流文書はすべてDOCノードとし、文書間・VO→文書は `derives_from` の一種で表現する（§19）。

*導出元: SPEC-092, SPEC-093, SPEC-094, SPEC-095, SPEC-096, SPEC-097, SPEC-098, SPEC-099, SPEC-100, SPEC-101, SPEC-102, SPEC-103, SPEC-104, SPEC-105, SPEC-106, SPEC-107, SPEC-108, SPEC-109, SPEC-110, SPEC-111, SPEC-112, SPEC-113, SPEC-437, SPEC-438, SPEC-439, SPEC-440*

*引用: 基本仕様 §3.1, 基本仕様 §3.2*

### BD-S055 5.5 `rust-cargo` SourceDiscoveryAdapter

*導出元: SPEC-S070, SPEC-S073, DS-S051*

### BD-221

`vtest-scan` はこれらのRust固有処理を実行しない。

### BD-222

各管理対象Testに1件以上のSource Target（`targets ≥ 1`）を必須とすることはadapter層に属し、core中立の `chain_integrity` 必須リンクではない（§4.1・§11.1.1）。

### BD-223

したがって `rust-cargo` のTestは従来どおりSource Target宣言を要し、挙動・Eコード・fixtureは本改訂で実効的に変わらない。

### BD-S056 5.6 文書層 orphan_detection

*導出元: REQ-S010, SPEC-S021, DS-S012*

### BD-224

`config.yaml` の `doc.roots` に列挙されたDOC IDを根として扱い、`orphan_detection` の対象外とする（§2.2）。

### BD-225

根指定は `.verify/` 設定として保持する。

*導出元: SPEC-151, SPEC-152, SPEC-447, SPEC-448, SPEC-449*

*引用: 基本仕様 §5.2*

### BD-226

根指定の追加・削除は `vtest doc` コマンドの引数で管理し `doc.roots` へ反映する。

*導出元: SPEC-354, SPEC-355, SPEC-356, SPEC-357, SPEC-358*

*引用: 基本仕様 §26.1, 別紙A*

## BD-S057 6. Target Reference解決

### BD-S058 6.1 adapter-neutral解決contract

*導出元: SPEC-S011, SPEC-S033, DS-S023*

### BD-227

coreは`TargetRef::Locator.adapter`をregistryで解決する。

### BD-228

coreは、opaque locatorの解釈を該当する`SourceDiscoveryAdapter`へ委譲する。

### BD-229

coreはopaque locatorの内部構文は解釈しない。

### BD-230

SRC ID参照はcoreが統合済みSRC索引で一意性を検査する。

### BD-231

Target Referenceの解決はcoreの単一経路が所有し、静的解析、実行、Evidence writer、検証集約はいずれもTarget Referenceの解決の結果を消費する。

### BD-232

各subsystemが独自にcandidate列を走査して1件を選ぶ経路を持ってはならない。

## BD-S059 7. Static Analysis orchestrationと`rust-cargo`ルール

### BD-233

`vtest audit static`は要求時に解析を起動し、結果をstdoutと`cache/`へ出力する。

*導出元: SPEC-354, SPEC-355, SPEC-356, SPEC-357, SPEC-358*

*引用: 基本仕様 §26.1*

### BD-S060 7.1 判定の原則

*導出元: P-003, SPEC-S023, SPEC-S027, DS-S014, DS-S017*

### BD-234

adapter固有のASTやassertion構文をcoreは解釈しない。

### BD-S061 7.2 `rust-cargo` ルール一覧

*導出元: SPEC-S023, SPEC-S024, SPEC-S030, DS-S014, DS-S015, DS-S020*

### BD-235

ルールごとの判定結果と根拠（該当スパン）は`vtest audit static`の出力および`cache/`の派生結果として提示する。

## BD-S062 8. 判断記録プロトコル

*導出元: REQ-S035, SPEC-S036, DS-S026*

### BD-S063 8.1 バンドル生成

### BD-236

`vtest audit bundle`は判断対象ごとに、判断に必要な情報をJSONとして`cache/bundles/<ULID>.json`へ出力する。

### BD-237

バンドルは派生情報でありGit管理しない。

### BD-238

提出結果の検証に必要な情報（対象の内容ハッシュ）は判断記録へ複製されるため、バンドル自体の永続化は不要である。

### BD-S064 8.4 提出の検証

### BD-239

`audit submit`は、検証に失敗した場合は§17のエラーコードで拒否する。

### BD-240

受理された提出は判断記録（§3.4）として`.verify/decisions/`へ保存される。

## BD-S065 9. テスト実行設計

*導出元: SPEC-S026, SPEC-S056, DS-S038*

### BD-S066 9.1 実行対象の解決

### BD-241

旧モデルの`--req`（REQ指定）はdocument層の総称化により廃止し、document scopeが必要な場合はVO部分木経由で指定する。

### BD-S067 9.2 `rust-cargo` TestRunnerAdapter

### BD-242

実行対象の解釈とcommand生成は`TestRunnerAdapter`が所有する。

## BD-S068 10. `rust-cargo` Target Binding 動的計測

*導出元: SPEC-S022, SPEC-S056, DS-S013, DS-S038*

### BD-S069 10.1 計測方式

### BD-243

`rust-cargo` CoverageAdapterは`cargo-llvm-cov`を使用する（adapter configの`run.coverage: llvm-cov`）。

### BD-244

Testが起動したsubprocess・spawnしたthreadの実行を宣言targetへ帰属させられるかは`rust-cargo` CoverageAdapterの能力に属する（§10.2・§7.3）。

### BD-245

coverageは独立した`CoverageAdapter` capabilityとして扱う。

### BD-S070 10.2 判定

### BD-246

coverage providerが境界越しの実行を宣言targetへ帰属させられるかはadapterのcoverage capabilityに属し、能力の有無で計測結果を捏造しない。

## BD-S071 11. 鮮度検証と集約

### BD-S072 11.3 集約アルゴリズム

*導出元: SPEC-S058, DS-S040*

### BD-247

Featureを独立のエンティティ種別・レコードファイル・ID体系・宣言fieldとして設けず、`.verify/`にFeature用ディレクトリを置かない（基本仕様 §3.1のエンティティ種別を増やさない）。

*導出元: SPEC-092, SPEC-093, SPEC-094, SPEC-095, SPEC-096, SPEC-097, SPEC-098, SPEC-099, SPEC-100, SPEC-101, SPEC-102, SPEC-103, SPEC-104, SPEC-105, SPEC-106, SPEC-437, SPEC-438*

*引用: 基本仕様 §3.1*

### BD-248

機能単位の表示経路（起点指定と内訳の提示）は§11.6のprojectionで露出し、新規コマンド・ツール・出力エンティティを増やさない。

### BD-S073 11.5 フェーズゲート評価

*導出元: REQ-S057, SPEC-S055*

### BD-249

ゲート定義は、`config.yaml`の`gates`（§2.2）に、ゲート名と進行条件（`require.verification`＝要求する検証結果、`require.approvals`＝要求する承認ロール集合）を保持する。

### BD-250

新規CLIコマンド・MCPツールを増やさず、既存の`vtest verify`の`--gate`引数と出力、および`report`のJSONでゲート評価を露出する（引数・出力schemaは別紙A）。

*引用: 別紙A*

### BD-S074 11.6 役割別 projection

*導出元: REQ-S007, SPEC-S054, SPEC-S060, DS-S037, DS-S042*

### BD-251

新規コマンド・ツールを増やさず、既存の`vtest report`のview / projection引数と、`test query`の逆引きで露出する（引数・出力schemaは別紙A）。

*引用: 別紙A*

### BD-252

「どの上流条項が、どの概念（VO）へ対応するか」の対応ペアの取得のために新規コマンド・ツールを設けない。

### BD-S075 11.7 判断待ち情報の構造

*導出元: SPEC-S052, SPEC-S073*

### BD-253

新規コマンド・ツールを増やさず、`vtest verify` / `vtest report`のJSON出力に判断待ちsectionを含めて露出する。

## BD-S076 16. 並列動作と整合性

### BD-S077 16.1 ロック不要の根拠

*導出元: SPEC-S064, DS-S046*

### BD-254

書き込み操作は上記のいずれかに分類され、ファイルロックを必要としない。

*導出元: SPEC-335, SPEC-336, SPEC-337, SPEC-338, SPEC-339*

*引用: 基本仕様 §24.2*

### BD-255

新規レコード追加（rel / decisions / approvals / evidence）はULIDファイル名の新規作成のみであり、並列生成は衝突しない。

*導出元: SPEC-335, SPEC-336, SPEC-337, SPEC-338, SPEC-339*

*引用: 基本仕様 §24.2*

### BD-256

エンティティファイル編集（doc / vo）は1エンティティ1ファイルであり、異なるエンティティの並列編集は独立であり、同一エンティティの並列編集はGitのマージ衝突として顕在化する。

*導出元: SPEC-335, SPEC-336, SPEC-337, SPEC-338, SPEC-339*

*引用: 基本仕様 §24.2*

### BD-257

テストコード編集は通常のソース編集と同じ扱いとする。

*導出元: SPEC-335, SPEC-336, SPEC-337, SPEC-338, SPEC-339*

*引用: 基本仕様 §24.2*

### BD-258

同時実行された`vtest`プロセス同士の調停は行わない。

### BD-259

「その時点の正典の読み取り」は書込みの原子的公開（基本仕様 §24.2）を前提とする。

*導出元: SPEC-335, SPEC-336, SPEC-337, SPEC-338, SPEC-339*

*引用: 基本仕様 §24.2*

### BD-260

テストコード編集は通常のソース編集と同じ扱いで本規定の対象外とする。

### BD-S078 16.2 意味的衝突検出

*導出元: SPEC-S062, SPEC-S064, DS-S044, DS-S046*

### BD-261

`vtest doctor`は、同じTest IDの重複、covers先VOの欠落、承認済VOの内容不一致など、version controlの構文的整合性だけでは判定できない論理的不整合を検出する。

## BD-S079 12. CLI 詳細仕様

### BD-S080 12.1 共通仕様

*導出元: REQ-S009, SPEC-S013, SPEC-S014, SPEC-S018, SPEC-S061, SPEC-S088, SPEC-S108, SPEC-S116, DS-S006, DS-S009, DS-S043, DS-S066, DS-S071, DS-S102, DS-S112*

### BD-262

CLIの操作は登録済みadapter registryを通じて実装を選択する。

### BD-263

targetsの必須件数は adapter が定める。

## BD-S081 13. MCP ツール詳細仕様

### BD-S082 13.1 共通仕様

*導出元: SPEC-S069, DS-S050*

### BD-264

transport は stdio である。

### BD-265

`rmcp` で実装する。

### BD-S083 13.3 エージェント向け利用フロー（参考）

*導出元: SPEC-S036, SPEC-S066, SPEC-S097, DS-S026, DS-S084*

### BD-266

各操作は、CLIとMCPで同じadapter registryを解決する。

## BD-S084 14. Form Schema 設計

### BD-S085 14.2 検証器

*導出元: SPEC-S046, SPEC-S088, SPEC-S093, DS-S033, DS-S071, DS-S076, DS-S078*

### BD-267

`kind` は `[a-z0-9][a-z0-9-]*` の case-sensitive 文字列で、`.verify/forms/<kind>.yaml` のファイル名と一致する repository-global な Form ID である。

### BD-268

`adapter` はその Form を処理する Structured Test adapter ID である。

### BD-S086 14.3 組込フォーム

*導出元: SPEC-S046, SPEC-S070, DS-S033, DS-S051, DS-S067*

### BD-269

組込フォームは `rust-cargo` adapter が提供する。

### BD-S087 14.4 テスト種別ごとのフォーム拡張

*導出元: SPEC-S040, SPEC-S046, SPEC-S088, DS-S028, DS-S033, DS-S066, DS-S071*

### BD-270

Form Schema はユーザー定義可能とし、大局的に一意な `kind` と登録済み Structured Test adapter の `adapter` ID を必須とする。

*導出元: SPEC-258, SPEC-259, SPEC-260, SPEC-261*

*引用: 本冊 §4.1・§5.2, 基本仕様 §15.4*

### BD-271

user-defined Form も `kind` と owner `adapter` ID を宣言する通常の Form Schema であり、kind の大局的一意性と owner 解決の規則（§14.2）は変わらない。

*導出元: SPEC-258, SPEC-259, SPEC-260, SPEC-261*

*引用: 基本仕様 §15.4*

## BD-S088 15. Structured Test Operation adapter contract

### BD-272

Structured Edit の構文解析・再生成・selector 解釈は対応 adapter が所有する。

### BD-273

orchestration は Test ID と adapter ID で対象を一意に選択し、adapter が返す拡張範囲を単一置換として適用する。

### BD-274

production adapter として提供するのは `rust-cargo` だけである。

### BD-275

§15.1〜§15.4 は `rust-cargo` StructuredTestAdapter の構文処理を定める。

## BD-S089 18. 受入契約

### BD-S090 18.1 共通条件

*導出元: SPEC-S018, SPEC-S025, SPEC-S059, SPEC-S063, DS-S009, DS-S016, DS-S041, DS-S045*

### BD-276

CLIとMCPは同じcore処理、adapter registry、JSON envelope、診断codeを使用する。

### BD-S091 18.2 共通fixture

*導出元: SPEC-S009, SPEC-S013, SPEC-S014, SPEC-S015, SPEC-S021, SPEC-S040, SPEC-S091, SPEC-S096, DS-S006, DS-S007, DS-S012, DS-S028, DS-S068, DS-S074, DS-S083*

### BD-277

Rustの受入fixtureは、総称document、VO、登録Test、Source Target、承認記録、判断記録、Evidenceを含む小規模projectとする。

### BD-278

fixture は、正しい annotation を持つ Test を表現できる。

### BD-279

fixture は、`assert!(true)` だけの Test を表現できる。

### BD-280

fixture は、宣言 target を呼ばない Test を表現できる。

### BD-281

fixture は、結果を検証しない Test を表現できる。

### BD-282

fixture は、自己比較を行う Test を表現できる。

### BD-283

fixture は、annotation を持たない test function（W-SCAN-101、`chain_integrity = MISMATCH`、診断`MISSING`）を表現できる。

### BD-284

fixture は、`covers` を宣言しない Test（`covers` 0）を表現できる。

### BD-285

fixture は、`rust-cargo` で `targets` を宣言しない Test（E-SCAN-007、`chain_integrity = MISMATCH`、診断`MISSING`）を表現できる。

> `targets ≥ 1`は`rust-cargo` adapterの必須metadata。

*引用: 本冊 §4.4・§5.5*

### BD-286

fixture は、存在しない VO を参照する Test（E-SCAN-003、`chain_integrity = MISMATCH`）を表現できる。

### BD-287

fixture は、Test ID が衝突する Test（E-SCAN-002、`chain_integrity = MISMATCH`）を表現できる。

### BD-288

fixture は、Test construct と非隣接の metadata 宣言だけを変更した状態（Test subject hash が変化する）を表現できる。

### BD-289

fixture は、Test / 宣言 target を変更せず、実行結果を変えうる target 外 helper または local dependency だけを変更した状態（Execution State subject が変化し Evidence が STALE 化）を表現できる。

### BD-290

fixture は、`@vtest.case` を持つ table-driven Test を表現できる。

### BD-291

fixture は、複数 target を宣言し、target ごとに PASS / FAIL / UNKNOWN が異なる integration Test を表現できる。

### BD-292

fixture は、5 状態それぞれを生じる入力（`PASS` / `FAIL` / `MISMATCH` / `NO_EVIDENCE` / `UNKNOWN`）を表現できる。

### BD-293

fixture は、4 診断ラベルそれぞれを生じる入力（`MISSING` / `NOT_EXECUTED` / `NOT_CHECKED` / `STALE`）を表現できる。

### BD-294

fixture は、Test または target の hash 変更によって無効になる判断記録 / Evidence を表現できる。

### BD-295

fixture は、複数 adapter が同じ恒久 SRC ID を宣言する状態（E-SCAN-011）を表現できる。

### BD-296

fixture は、同一の Source Target を、一方の Test が locator で、他方の Test が恒久 SRC ID で宣言する状態を表現できる。

### BD-297

fixture は、同一の Test が同一 Source Target を locator と恒久 SRC ID の両方で宣言する状態（E-SCAN-005）を表現できる。

### BD-298

fixture は、Source Target construct の内側にある `@vtest.src-id` 宣言だけを付与・変更・削除した状態（construct bytes が変化し Source Target hash も変化する）を表現できる。

### BD-299

fixture は、呼出を静的に確認できない到達境界を越えて target を実行する Test（subprocess spawn型・spawn thread型）を表現できる。

### BD-300

fixture は、他ファイル・他クレートへ呼び出すが戻り値を Test 本体内で assert する Test（DA-002 UNKNOWN・DA-003 PASS）を表現できる。

### BD-301

fixture は、文書鎖の状態として `doc.roots` に列挙された根 document を表現できる。

### BD-302

fixture は、文書鎖の状態として `derives_from` が空かつ根に列挙されない孤児 document（E-SCAN-016、`orphan_detection = MISMATCH`）を表現できる。

### BD-303

fixture は、文書鎖の状態として `derives_from` の参照先が存在しない document / VO（E-SCAN-012、`chain_integrity = MISMATCH`）を表現できる。

### BD-304

fixture は、文書鎖の状態として `content_hash` と実ファイルが一致しない document（W-SCAN-104、`chain_integrity = MISMATCH`、診断 `STALE`）を表現できる。

### BD-305

fixture は、文書鎖の状態として document 再登録で失効する判断記録・承認記録を表現できる。

### BD-306

fixture は、判断記録を受理しても対象の検証状態が昇格しない状態（判断受理前後で `UNKNOWN` が `PASS` へ変わらない）を表現できる。

### BD-307

fixture は、上流依存 closure またはハッシュを欠く互換 Approval（W-STORE-002、VO は `draft` 相当）を表現できる。

### BD-308

fixture は、フェーズゲート定義（`config.yaml` の `gates`）を持ち、`vtest verify --gate <name>` が条件充足・不足の両方を提示する状態を表現できる。

### BD-309

adapter境界fixtureは、Rust parser、Cargo、llvm-covを使用しないin-process synthetic adapterを使用できる。

### BD-310

synthetic fixtureは`.rs`以外のsource、関数ではないTest construct、doc commentではないmetadata宣言、Rust item pathではないopaque locatorを使用する。

### BD-S092 18.3 機能別受入条件

#### BD-S093 18.3.1 discovery・record・graph と chain_integrity

*導出元: SPEC-S010, SPEC-S020, SPEC-S033, SPEC-S048, SPEC-S062, SPEC-S087, SPEC-S093, SPEC-S107, DS-S004, DS-S011, DS-S023, DS-S034, DS-S044, DS-S069, DS-S076, DS-S100*

### BD-311

Source Target identityは「宣言された`TargetRef` → 解決 → canonical Locator」の一方向で確定する。

### BD-312

Target Reference解決はcoreの単一経路が所有する。

### BD-313

discovery、静的解析、実行、Evidence writer、検証集約が独自にcandidate列を走査して1件を選ぶ経路を持たない。

#### BD-S094 18.3.2 orphan_detection（文書層の孤児検出）

*導出元: REQ-S010, SPEC-S021, SPEC-S091, DS-S012, DS-S074*

### BD-314

根指定の追加・削除は `vtest doc` コマンドの引数で管理する。

*導出元: SPEC-354, SPEC-355, SPEC-356, SPEC-357, SPEC-358*

*引用: 基本仕様 §26.1*

#### BD-S095 18.3.3 決定論的静的解析（oracle_presence・target_binding 静的到達）

*導出元: SPEC-S023, SPEC-S024, SPEC-S030, SPEC-S095, SPEC-S096, DS-S014, DS-S015, DS-S020, DS-S080, DS-S081, DS-S083*

### BD-315

静的解析は正典レコードを持たない再計算派生であり、検証のたびに現在のsource / configから再計算する。

*導出元: P-003*

*引用: 本冊 §7.1, 基本仕様 P-003*

### BD-316

表示scopeと内部依存評価を分離する。

#### BD-S096 18.3.6 判断記録プロトコル（非ゲート）

*導出元: REQ-S035, SPEC-S036, SPEC-S039, SPEC-S097, DS-S026, DS-S027, DS-S084*

### BD-317

`vtest audit bundle` は判断対象ごとに、判断に必要な情報（対象 VO と claim、Test Intent、テストコード、対象実装、関連テスト、既知 partition、過去の判断、対象の内容ハッシュとリビジョン）を JSON として `cache/bundles/` へ出力する。

### BD-318

バンドルは派生情報であり Git 管理しない。

*導出元: SPEC-228, SPEC-229, SPEC-230, SPEC-231, SPEC-232, SPEC-233, SPEC-234, SPEC-235, SPEC-236, SPEC-237, SPEC-508, SPEC-509, SPEC-510, SPEC-511*

*引用: 本冊 §8.1, 基本仕様 §11.3*

### BD-319

deterministic 結果（§18.3.3 の静的解析）と agent / human の判断結果を区別して保存・表示する。

#### BD-S097 18.3.7 承認と判断記録の分離

*導出元: SPEC-S017, SPEC-S039, SPEC-S048, SPEC-S084, SPEC-S085, DS-S008, DS-S027, DS-S034, DS-S062, DS-S063*

### BD-320

承認レコード生成の正典面は対象種別を引数に取る単一の経路である。

### BD-321

`vtest approval create --subject-type <vo|document|judgment> --subject-id <id>` および `approval_create`（`subject: { type, id }`）だけが承認レコードを生成し、`vtest vo approve` / `vo_approve` は同経路への別名として同一のレコード・同一の拒否条件を持つ。

### BD-322

対象種別ごとに別の承認規則・別の承認コマンドを設けない。

*引用: 本冊 §3.5, 別紙A §12.2・§13.2*

### BD-323

`vtest approval withdraw <approval-id>` / `approval_withdraw` は `state: withdrawn` かつ `supersedes: [approval-id]` の `create` と同一のレコードを生成する。

### BD-324

`vtest approval show` / `approval_get` は当該対象の承認レコード一覧と実効承認状態（`draft` / `approved`）を返す。

#### BD-S098 18.3.8 verify・report と scope

*導出元: SPEC-S018, SPEC-S019, SPEC-S059, SPEC-S060, SPEC-S061, SPEC-S079, SPEC-S106, SPEC-S108, DS-S009, DS-S010, DS-S041, DS-S042, DS-S043, DS-S056, DS-S099, DS-S102*

### BD-325

scope は 2 軸で限定できる。

*導出元: P-002, SPEC-140, SPEC-141*

*引用: 基本仕様 §4.6, 要件定義 P-002*

### BD-326

NO_EVIDENCE を生む入力（証拠が存在しない／証拠のハッシュが現在の対象と不一致／scope 限定により検査を実施しなかった項目）を受入で表現する。

#### BD-S099 18.3.9 フェーズゲート評価

*導出元: REQ-S057, SPEC-S013, SPEC-S017, SPEC-S055, SPEC-S079, SPEC-S109, SPEC-S115, SPEC-S116, DS-S006, DS-S008, DS-S056, DS-S104, DS-S111, DS-S112*

### BD-327

条件充足・不足の両方を fixture で確認する。

### BD-328

順序・包含解釈による充足を認めない fixture を持つ。

#### BD-S100 18.3.12 adapter contract

*導出元: SPEC-S007, SPEC-S070, SPEC-S088, SPEC-S115, DS-S002, DS-S051, DS-S071, DS-S111*

### BD-329

`rust-cargo` adapterはRust discovery、static audit、Structured Test Operation、runner、coverageを所有する。

### BD-330

`vtest-scan`はadapter discoveryの委譲・出力検証・決定論的統合・core record整合性を所有し、`*.rs`列挙、`syn::parse_file`、`#[test]`抽出、doc comment parseを所有しない。
