<!-- generated from docs/canonical/specification.json by build.py; do not edit -->

# 基本仕様

## SPEC-S001 0. 本書の位置付け

*導出元: P-005, REQ-S049, REQ-S058*

### SPEC-001

本書は「AI並列開発向けテスト検証システム 要求・要件定義 v0.1」（以下、要件定義。FROZEN/v0.1 baseline）の下流文書である。

### SPEC-002

要件定義はWHY（何を保証しなければならないか）を定める。

### SPEC-003

本書はWHAT（システムが外部に対して保証する挙動・データモデル・状態モデル・インターフェースの範囲）を確定する。

### SPEC-004

具体構文・アルゴリズム・スキーマの全フィールド・コマンド引数などのHOWは「詳細設計 v0.1」で定める。

### SPEC-005

本書は具体構文・アルゴリズム・スキーマの全フィールド・コマンド引数などのHOWを発明しない。

### SPEC-006

要件定義に無い義務・検査・状態・文書種別を本書で新設しない。

> 規範の伝播は上流→下流。

*導出元: P-005*

*引用: 要件定義 P-005*

### SPEC-007

矛盾・不足を発見した場合、本書を書き換えない。

### SPEC-008

矛盾・不足を発見した場合、上流へフィードバックしOwner判断を経る。

### SPEC-009

本書からの `要件定義 §n` 参照は、FROZEN 要件定義の連番（§1〜§28、および原則 P-001〜P-005、要求 R-1〜R-5）を指す。

*導出元: R-1, R-5, P-001, P-005*

*引用: P-001, P-005, R-1, R-5*

### SPEC-010

`rust-cargo` はRustの関数単体テストおよび小規模な結合テスト（`#[test]` 属性を持つテスト関数）を対象とする。

### SPEC-011

`rust-cargo` 以外の production language adapter はv0.1の提供範囲に含めない。

*導出元: REQ-258, REQ-259, REQ-260, REQ-261, REQ-262, REQ-263, REQ-264, REQ-265, REQ-266*

*引用: 要件定義 §21*

### SPEC-012

`vtest` 自身はLLM APIを呼ばない。

### SPEC-013

`vtest` 自身は宣言と実装の意味的な良し悪しを裁定しない。

### SPEC-014

機械が決定論で確定できない疑義は `UNKNOWN` として外部の判断者へ引き渡す（§11）。

*導出元: REQ-180, REQ-181, REQ-182, REQ-183, REQ-184, REQ-185, REQ-186, REQ-187, REQ-188, REQ-189, REQ-190, REQ-191, REQ-192*

*引用: 要件定義 §12*

### SPEC-015

coreの検証契約は言語・test runnerに依存しない。

*導出元: REQ-258, REQ-259, REQ-260, REQ-261, REQ-262, REQ-263, REQ-264, REQ-265, REQ-266*

*引用: 要件定義 §21*

## SPEC-S002 1. 用語定義

*導出元: REQ-S001, REQ-S003, REQ-S013, REQ-S020, REQ-S025, REQ-S029, REQ-S035, REQ-S036, REQ-S037, REQ-S046*

### SPEC-016

documentとは、ソースコードより上流に位置する成果物を表す単一の総称ノードである。

### SPEC-017

documentは要件定義書・基本仕様書・詳細設計書・API Schema・Protocol Specification・型/データ仕様・DB schema・その他の機械可読仕様を含む。

### SPEC-018

documentは `id + path + content_hash + 上流参照（derives_from）` を持つ。

### SPEC-019

文書種別ごとの専用スキーマは設けない。

*導出元: REQ-024, REQ-025, REQ-026, REQ-027, REQ-028, REQ-029, REQ-030, REQ-031, REQ-032, REQ-033*

*引用: 要件定義 §3.2*

### SPEC-020

対象ソースコード自身のdoc commentは、その対象実装の唯一の仕様根拠としては用いない。

*導出元: REQ-233, REQ-234, REQ-235*

*引用: 要件定義 §18*

### SPEC-021

derives_fromとは、document間の唯一のリンク種別である。

### SPEC-022

derives_fromは上流documentから下流documentへの導出を表す。

### SPEC-023

各derives_fromリンクは任意（optional）の説明文・導出理由を保持できる（§3.2）。

*導出元: REQ-036, REQ-037, REQ-038, REQ-039, REQ-040, REQ-041, REQ-042, REQ-043, REQ-044, REQ-045, REQ-046, REQ-047, REQ-048, REQ-049*

*引用: 要件定義 §3.4*

### SPEC-024

Verification Obligation（VO）とは、独立して「この条件が成立するか」と検証可能な仕様上の命題である。

*導出元: REQ-162, REQ-163*

*引用: 要件定義 §10.1*

### SPEC-025

VOは1件以上のdocumentからderives_fromで導出される。

### SPEC-026

VOとdocumentの間に他のエンティティ層を置かない。

### SPEC-027

VOは階層構造を持てる。

### SPEC-028

VOの粒度をassert文・test function・テストファイルなどのコード構文で決めない。

### SPEC-029

Testとは、登録adapterが実行可能な検証単位として識別し、Test IDで管理するtest constructである。

### SPEC-030

TestはVOの検証実装単位であり、VOとN:Mの対応を持ちうる。

### SPEC-031

Testは `covers` 宣言でVOを参照する。

### SPEC-032

Test Intentとは、Testが「何を検証するか」を実装コードを読まずに判断できる形で表した付随情報である。

### SPEC-033

Test Intentは宣言鎖のノードではない。

*導出元: REQ-203, REQ-204, REQ-205*

*引用: 要件定義 §14*

### SPEC-034

検証対象とは、その Test が検証成立性（§8）を証明しようとする対象、すなわち宣言された「何の時にどうなる」の主語である。

### SPEC-035

検証対象は実装constructに限定せず、外部から観測可能な契約・境界上の振る舞いも含む。

*導出元: REQ-144, REQ-145, REQ-146, REQ-147, REQ-148*

*引用: 要件定義 §9.1*

### SPEC-036

Source Target（SRC）とは、実装コード上の識別可能なimplementation constructである。

### SPEC-037

Source Targetは、adapter IDとadapter所有のopaque locatorからなるTarget Reference、または任意の恒久SRC IDで識別する。

*導出元: REQ-149, REQ-150, REQ-151, REQ-152, REQ-153, REQ-154, REQ-155, REQ-156*

*引用: 要件定義 §9.2*

### SPEC-038

Execution Evidenceとは、テスト実行の事実の記録である。

### SPEC-039

Execution Evidenceは結果・実行時リポジトリ状態・解決後のcanonical Source Target参照・各内容ハッシュ・実行計測結果を含む。

### SPEC-040

Execution Evidenceは検証対象の内容ハッシュに束縛される（§6）。

*導出元: REQ-115, REQ-116, REQ-117, REQ-118, REQ-119, REQ-120*

*引用: 要件定義 §6*

### SPEC-041

判断記録（judgment record）とは、`UNKNOWN` に対して外部（人間または判断可能Agent）が下した判断の記録である。

### SPEC-042

判断記録はactor / subject / decisionを必須項目とする。

### SPEC-043

判断記録の理由・根拠は任意とする。

### SPEC-044

判断記録は依存closureのハッシュに束縛される（§11）。

*導出元: REQ-180, REQ-181, REQ-182, REQ-183, REQ-184, REQ-185, REQ-186, REQ-187, REQ-188, REQ-189, REQ-190, REQ-191, REQ-192*

*引用: 要件定義 §12*

### SPEC-045

承認記録（approval record）とは、判断または方針を「この内容で進める」と正式に認めた記録である。

### SPEC-046

承認記録はapprover / subject（またはjudgment reference）/ approved stateを必須とする。

### SPEC-047

承認記録は上流依存closureのハッシュに束縛される。

### SPEC-048

承認記録は判断記録とは別軸・別entityでありうる（§17）。

*導出元: REQ-236, REQ-237, REQ-238, REQ-239, REQ-240, REQ-241, REQ-242, REQ-243, REQ-244, REQ-245, REQ-246, REQ-247, REQ-248, REQ-249, REQ-250, REQ-251, REQ-252*

*引用: 要件定義 §19*

### SPEC-049

検証状態は `PASS` / `FAIL` / `MISMATCH` / `NO_EVIDENCE` / `UNKNOWN` の5つとする（§4.1）。

*導出元: REQ-085, REQ-086, REQ-087, REQ-088, REQ-089, REQ-090, REQ-091*

*引用: 要件定義 §5.1*

### SPEC-050

検証状態は検証結果のみを表す。

### SPEC-051

検証状態に承認状態を混入させない。

*導出元: REQ-109, REQ-110, REQ-111, REQ-112, REQ-113, REQ-114*

*引用: 要件定義 §5.5*

### SPEC-052

診断ラベルとは、検証状態に付随して原因を説明するラベル（`MISSING` / `NOT_EXECUTED` / `NOT_CHECKED` / `STALE` 等）である。

### SPEC-053

診断ラベルは検証状態ではない。

### SPEC-054

診断ラベルの語彙は詳細設計で定める。

*導出元: REQ-092, REQ-093, REQ-309, REQ-310, REQ-311, REQ-312, REQ-313, REQ-314, REQ-315, REQ-316, REQ-317, REQ-318, REQ-319, REQ-320, REQ-321, REQ-322, REQ-323, REQ-324, REQ-325, REQ-326, REQ-327, REQ-328, REQ-329, REQ-330, REQ-331, REQ-332*

*引用: 要件定義 §5.2、§28*

### SPEC-055

検査は `chain_integrity` / `orphan_detection` / `target_binding` / `oracle_presence` の4本のみとする（§5）。

*導出元: REQ-034, REQ-035, REQ-050, REQ-051, REQ-052, REQ-053, REQ-054, REQ-055, REQ-056, REQ-057, REQ-058, REQ-059, REQ-060, REQ-061, REQ-062, REQ-063, REQ-064, REQ-065, REQ-066, REQ-067, REQ-068, REQ-069, REQ-070, REQ-071, REQ-072, REQ-073, REQ-074, REQ-075, REQ-076, REQ-077, REQ-078, REQ-079, REQ-080, REQ-081, REQ-082, REQ-083, REQ-084*

*引用: 要件定義 §3.3、§4*

### SPEC-056

完全検証とは、宣言鎖全体に対する検査（`chain_integrity` / `orphan_detection`）と、scope内の各「宣言＋コード＋証拠」の組に対する検査（`target_binding` / `oracle_presence`）をすべて対象とする検証である。

### SPEC-057

完全検証は一項目でも非 `PASS` があればNGとする（fail-closed）。

*導出元: REQ-295, REQ-296*

*引用: 要件定義 §26.1*

### SPEC-058

scopeとは、利用者が限定する検査・エンティティの範囲である。

### SPEC-059

scopeを狭めても対象外項目を `PASS` へ書き換えない。

*導出元: P-002*

*引用: 要件定義 §2/P-002*

### SPEC-060

正典（source of truth）とは、ある事実を決定する唯一の記録である。

### SPEC-061

正典から導出できる情報は派生情報とし独立保存しない。

*導出元: P-003*

*引用: 要件定義 P-003*

### SPEC-062

Agent Form Engineeringとは、既知の作業手順・入力項目を持つ操作を、自由編集ではなく構造化された質問・入力・検証で行わせる方式である。

*導出元: P-004*

*引用: 要件定義 P-004*

## SPEC-S003 2. 全体像

### SPEC-S004 2.1 正典の三層構造

*導出元: P-003, REQ-S019, REQ-S035, REQ-S046*

### SPEC-063

本システムは扱う情報を三層に分ける。

### SPEC-064

実装層は、テストコード本体と対象ソースコードからなり、Gitで管理される正典である。

### SPEC-065

派生情報（検索インデックス、検証グラフ、集約結果）は上記から毎回再構築する。

### SPEC-066

派生情報はGit管理しない。

*導出元: P-003*

*引用: 要件定義 P-003 / NFR-004*

### SPEC-067

adapterが返す導出結果はregistryでmergeし、adapter ID・path・Test IDの順に正規化する。

### SPEC-068

registryの重複ID、未登録adapter、adapter間のTest ID重複は操作エラーとする。

### SPEC-069

registryの重複ID、未登録adapter、adapter間のTest ID重複は空のscanとして成功扱いしない。

*導出元: REQ-258, REQ-259, REQ-260, REQ-261, REQ-262, REQ-263, REQ-264, REQ-265, REQ-266*

*引用: 要件定義 §21*

### SPEC-070

本システムの仕事は「宣言と実装が一致しているか」「事実が現在の宣言・実装に対して有効か」を照合することに限る。

### SPEC-071

どれかを正として他を修正させることはしない。

*導出元: P-001*

*引用: 要件定義 P-001*

### SPEC-S005 2.2 宣言鎖と照合

*導出元: REQ-S004, REQ-S005*

### SPEC-072

要件定義 §3.2 の宣言鎖をそのまま採用する。

*導出元: REQ-024, REQ-025, REQ-026, REQ-027, REQ-028, REQ-029, REQ-030, REQ-031, REQ-032, REQ-033*

*引用: 要件定義 §3.2*

### SPEC-073

上流文書はすべて単一の総称ノード型 `document` で表現する。

### SPEC-074

文書間リンクは `derives_from` の一種のみとする。

### SPEC-075

宣言鎖は document（上流文書）から derives_from で document（下流文書）へ、document から derives_from で Verification Obligation（VO）へ、VO から covers（Test 宣言）で Test へと連なる。

> document（上流文書）
>       | derives_from
>       v
> document（下流文書）
>       | derives_from
>       v
> Verification Obligation (VO)
>       | covers（Test 宣言）
>       v
> Test

### SPEC-076

文書層の段数は総称的に扱い、リンクを追加してもスキーマが壊れないことを設計制約とする。

### SPEC-077

段はリンクであって検査ではない。

*導出元: REQ-024, REQ-025, REQ-026, REQ-027, REQ-028, REQ-029, REQ-030, REQ-031, REQ-032, REQ-033, REQ-034, REQ-035*

*引用: 要件定義 §3.2/§3.3*

### SPEC-078

VOは1件以上の `document` からderives_fromで導出される。

### SPEC-079

VOとdocumentの間に他のエンティティ層を置かない。

### SPEC-080

本システムは文書内容の意味的な良し悪しに関知しない。

### SPEC-081

文書種別ごとの専用スキーマ・文書間リンク意味論の増殖・文書内容の良否検証を行わない。

*導出元: REQ-024, REQ-025, REQ-026, REQ-027, REQ-028, REQ-029, REQ-030, REQ-031, REQ-032, REQ-033, REQ-288, REQ-289, REQ-290, REQ-291, REQ-292, REQ-293, REQ-294*

*引用: 要件定義 §3.2、§25 OOS-001*

### SPEC-082

不一致はどちらが正かを決めない。

### SPEC-083

不一致は状態（§4）として提示する。

*導出元: P-001*

*引用: 要件定義 P-001*

### SPEC-S006 2.3 導出できる関係は保存しない

*導出元: P-003*

### SPEC-084

Test→VO（`covers`）、Test→SRC（`targets`）の関係はadapter所有のTest metadata宣言から決定論的に導出できる。

### SPEC-085

Test→VO、Test→SRCの関係を外部ファイルへ重複保存しない。

### SPEC-086

graphと現在のtarget集合は常にadapter所有のTest metadata宣言から再構築する。

### SPEC-087

graphと現在のtarget集合はEvidenceのtarget参照から関係を生成・修復しない。

### SPEC-088

Evidenceに含むtarget参照は、target別の実行事実と内容ハッシュを束縛する実行時snapshot keyである。

### SPEC-089

Evidenceに含むtarget参照はTest→SRC関係の正典ではない。

### SPEC-090

外部レコードとして保存するのは、どちらか一方のエンティティに自然に所属しない関係（VO間の依存、Test間の補完関係など）だけとする。

*導出元: P-003*

*引用: 要件定義 P-003*

### SPEC-S007 2.4 adapter 設定と wire 互換

*導出元: REQ-S048, REQ-S058*

### SPEC-091

readerは読み取りだけで正典を書き換えない。

## SPEC-S008 3. エンティティと ID 体系

### SPEC-S009 3.1 エンティティ種別

*導出元: REQ-S005, REQ-S025, REQ-S035, REQ-S036, REQ-S040, REQ-S046*

### SPEC-092

documentは総称の上流文書ノード（path＋content_hash＋derives_from）である。

### SPEC-093

documentは種別専用スキーマを持たない。

### SPEC-094

Verification Obligationは検証命題である。

### SPEC-095

Verification Obligationは階層を持てる。

### SPEC-096

Testはadapterが識別する実行可能なtest constructである。

### SPEC-097

Source Targetは対象implementation constructである。

### SPEC-098

Source Targetの恒久IDは必須としない。

### SPEC-099

Relationは外部関係レコードであり、不変とする。

### SPEC-100

derives_fromの説明文もRelationに保持できる。

### SPEC-101

判断記録は `UNKNOWN` への外部判断であり、追記型とする。

### SPEC-102

承認記録は判断・方針の正式採用であり、追記型とする。

### SPEC-103

Execution Evidenceは実行証拠レコードであり、追記型とする。

### SPEC-104

documentは単一の総称ノードであり、要件定義・基本仕様・詳細設計・API Schema等を種別で区別する専用スキーマを持たない。

### SPEC-105

文書層の段（要件→仕様→詳細設計…）はderives_fromリンクとして表現する。

### SPEC-106

段を増やしても種別を増やさない。

*導出元: REQ-024, REQ-025, REQ-026, REQ-027, REQ-028, REQ-029, REQ-030, REQ-031, REQ-032, REQ-033*

*引用: 要件定義 §3.2*

### SPEC-S010 3.2 ID 規則と関係リンク

*導出元: REQ-S007, REQ-S027, REQ-S058*

### SPEC-107

ツールはID形式を強制せず一意性のみを強制する。

### SPEC-108

IDの一意性はスキャン時に全数検査する。

### SPEC-109

関係リンクは説明文・導出理由を任意（optional）で保持できる。

### SPEC-110

derives_from・covers・検証対象・実装traceabilityなど性質の異なる関係型は潰さず区別する。

### SPEC-111

関係型そのものの意味論的増殖は求めない。

### SPEC-112

ULID payloadにより並列生成時のファイル名衝突を実用上排除する。

### SPEC-113

関係リンクの任意説明文・役割別projectionの保存形式・presetは詳細設計へ委譲する。

*導出元: REQ-309, REQ-310, REQ-311, REQ-312, REQ-313, REQ-314, REQ-315, REQ-316, REQ-317, REQ-318, REQ-319, REQ-320, REQ-321, REQ-322, REQ-323, REQ-324, REQ-325, REQ-326, REQ-327, REQ-328, REQ-329, REQ-330, REQ-331, REQ-332*

*引用: 要件定義 §28*

### SPEC-S011 3.3 Source Target の識別

*導出元: R-3, REQ-S027*

### SPEC-114

ソースコードへ恒久IDを埋め込むことは必須としない。

*導出元: REQ-149, REQ-150, REQ-151, REQ-152, REQ-153, REQ-154, REQ-155, REQ-156*

*引用: 要件定義 §9.2*

### SPEC-115

対象はTarget Referenceで識別する。

### SPEC-116

Target Referenceは、adapter IDとadapter所有のopaque locatorの組、または任意のSRC ID参照である。

> <adapter-id>::<opaque-locator>
> 例：rust-cargo::src/parser.rs::Parser::parse

### SPEC-117

共通契約がpath・module・function等の特定言語構造を必須としてはならない。

*導出元: R-3, REQ-149, REQ-150, REQ-151, REQ-152, REQ-153, REQ-154, REQ-155, REQ-156*

*引用: 要件定義 §9.2、R-3*

### SPEC-118

1つのTestは1件以上のSource Targetを持ち、各target参照を個別に保持する。

### SPEC-119

Source Targetは代表1件へ縮約しない。

## SPEC-S012 4. 検証状態と診断ラベル

### SPEC-S013 4.1 状態は 5 つ

*導出元: REQ-S014*

### SPEC-120

検証状態は `PASS` / `FAIL` / `MISMATCH` / `NO_EVIDENCE` / `UNKNOWN` の5つのみとする。

*導出元: REQ-085, REQ-086, REQ-087, REQ-088, REQ-089, REQ-090, REQ-091*

*引用: 要件定義 §5.1*

### SPEC-121

状態の存在資格は「受け取った者の行動が変わるか」である。

### SPEC-122

意味の違いは資格にならない。

### SPEC-123

`PASS` を受け取った者はマージできる。

### SPEC-124

`FAIL` を受け取った者は実装（テスト実装を含む）を直す。

### SPEC-125

`MISMATCH` を受け取った者はコードを触る前に宣言側（上流）を直す。

### SPEC-126

`NO_EVIDENCE` を受け取った者は証拠を作る（機械的に解決可能）。

### SPEC-127

`UNKNOWN` は決定論の限界であり、受け取った者は意味判定できる者へエスカレーションする。

### SPEC-S014 4.2 診断ラベル

*導出元: REQ-S015, REQ-S058*

### SPEC-128

`MISSING` / `NOT_EXECUTED` / `NOT_CHECKED` / `STALE` 等は、状態に付随して原因を説明する診断ラベルである。

### SPEC-129

診断ラベルは検証状態ではない。

### SPEC-130

診断ラベルの語彙は詳細設計で定める。

*導出元: REQ-092, REQ-093, REQ-309, REQ-310, REQ-311, REQ-312, REQ-313, REQ-314, REQ-315, REQ-316, REQ-317, REQ-318, REQ-319, REQ-320, REQ-321, REQ-322, REQ-323, REQ-324, REQ-325, REQ-326, REQ-327, REQ-328, REQ-329, REQ-330, REQ-331, REQ-332*

*引用: 要件定義 §5.2、§28*

### SPEC-131

本書は状態と診断ラベルを常に別軸として扱い、混同しない。

### SPEC-S015 4.3 状態の割当

*導出元: REQ-S016*

### SPEC-132

要件定義 §5.3 の割当をそのまま採用する。

*導出元: REQ-094, REQ-095, REQ-096, REQ-097, REQ-098, REQ-099, REQ-100, REQ-101, REQ-102, REQ-103, REQ-104*

*引用: 要件定義 §5.3*

### SPEC-S016 4.4 UNKNOWN の検疫

*導出元: REQ-S017*

### SPEC-133

`UNKNOWN` はエラーではなく正常動作としての降参である。

### SPEC-134

内部エラー・入力不正は検証状態と別系統（終了コード。§27）で表現する。

### SPEC-135

`UNKNOWN` をエラー処理のフォールバック先として使う実装は仕様違反とする。

*導出元: REQ-105, REQ-106, REQ-107, REQ-108*

*引用: 要件定義 §5.4*

### SPEC-S017 4.5 検証状態と承認の分離

*導出元: REQ-S018*

### SPEC-136

検証状態（§4.1の5状態）は検証結果のみを表す。

### SPEC-137

検証状態は承認状態を混入させない。

### SPEC-138

承認（§17）は独立した別軸である。

*導出元: REQ-109, REQ-110, REQ-111, REQ-112, REQ-113, REQ-114*

*引用: 要件定義 §5.5*

### SPEC-139

フェーズ進行に承認を要するかは、検証状態と承認の組合せとして §21 のゲート条件で扱う。

### SPEC-S018 4.6 scope

*導出元: P-002, REQ-S016*

### SPEC-140

検査軸は実施する検査（4本の部分集合）を指定する。

*導出元: P-002*

*引用: 要件定義 P-002*

### SPEC-141

エンティティ軸は対象とするdocument / VO / Testの部分木を指定する。

## SPEC-S019 5. 検査

### SPEC-142

検証は `chain_integrity` / `orphan_detection` / `target_binding` / `oracle_presence` の4検査のみで行う。

### SPEC-143

鎖に段（リンク）が増えても検査は増えない。

*導出元: REQ-034, REQ-035, REQ-050, REQ-051, REQ-052, REQ-053, REQ-054, REQ-055, REQ-056, REQ-057, REQ-058, REQ-059, REQ-060, REQ-061, REQ-062, REQ-063, REQ-064, REQ-065, REQ-066, REQ-067, REQ-068, REQ-069, REQ-070, REQ-071, REQ-072, REQ-073, REQ-074, REQ-075, REQ-076, REQ-077, REQ-078, REQ-079, REQ-080, REQ-081, REQ-082, REQ-083, REQ-084*

*引用: 要件定義 §3.3、§4*

### SPEC-144

各検査は一つの問いを持つ。

### SPEC-145

各検査は複数の証拠源で答えてよい。

### SPEC-146

凍結要件が検査から明示的に排除した判断（仕様網羅・VO網羅・VO分解妥当性・意味一致・実装一致）は、本書でも検査に含めない。

### SPEC-147

網羅・意味の疑義はエスカレーション（§11）の領分である。

*導出元: REQ-164, REQ-165, REQ-166, REQ-174, REQ-175, REQ-176, REQ-177, REQ-178, REQ-179, REQ-180, REQ-181, REQ-182, REQ-183, REQ-184, REQ-185, REQ-186, REQ-187, REQ-188, REQ-189, REQ-190, REQ-191, REQ-192*

*引用: 要件定義 §10.2、§11、§12*

### SPEC-S020 5.1 chain_integrity — 宣言鎖の完全性

*導出元: REQ-S009, REQ-S036*

### SPEC-148

chain_integrityの問いは、宣言鎖のすべてのリンクが存在し、ハッシュ照合が成立するかである。

### SPEC-149

どのリンクで切れたかは診断ラベルで示す。

### SPEC-150

すべてのTestを管理対象とすることと、当該Testを仕様適合の証拠として算入すること（§8）は別個の条件とする。

### SPEC-S021 5.2 orphan_detection — 文書層の孤児検出

*導出元: REQ-S010*

### SPEC-151

orphan_detectionの問いは、親を持たない `document` ノードが存在するかである。

### SPEC-152

根の指定の具体構文は詳細設計へ委譲する。

*導出元: REQ-309, REQ-310, REQ-311, REQ-312, REQ-313, REQ-314, REQ-315, REQ-316, REQ-317, REQ-318, REQ-319, REQ-320, REQ-321, REQ-322, REQ-323, REQ-324, REQ-325, REQ-326, REQ-327, REQ-328, REQ-329, REQ-330, REQ-331, REQ-332*

*引用: 要件定義 §28*

### SPEC-S022 5.3 target_binding — 宣言対象の振る舞いの実現

*導出元: REQ-S011, REQ-S020, REQ-S054*

### SPEC-153

target_bindingの問いは、そのTestが検証対象とする振る舞いが実際に生じ、その振る舞いを反映した観測が得られたかである。

### SPEC-154

テストランナーの `PASS`/`FAIL` は判定権威（§7）の証拠として消費する。

### SPEC-155

target_binding検査は、その証拠が検証対象の実行を伴ったかを問う。

### SPEC-156

target_bindingは一つの問いに対し静的解析と動的計測の2つの証拠源を持つ。

### SPEC-157

他の実行形態における確認方法は、当該形態に適した方法として詳細設計で定める。

### SPEC-158

特定形態の確認方法を別形態のTestへ一律要求しない。

*導出元: REQ-064, REQ-065, REQ-066, REQ-067, REQ-068, REQ-069, REQ-070, REQ-071, REQ-072, REQ-125, REQ-126, REQ-127, REQ-128, REQ-129, REQ-130, REQ-131, REQ-132, REQ-133, REQ-134, REQ-135, REQ-136, REQ-137, REQ-138, REQ-139, REQ-140, REQ-141, REQ-142, REQ-143*

*引用: 要件定義 §4.3、§8 条項 3*

### SPEC-S023 5.4 oracle_presence — 照合装置の存在

*導出元: REQ-S012, REQ-S021*

### SPEC-159

oracle_presenceの問いは、宣言された「何の時にどうなる」の不成立を、Testの非成功として反映する装置が存在するかである。

### SPEC-160

静的解析の役割は不成立の証明である。

### SPEC-161

静的解析は成立の証明装置ではない。

### SPEC-162

照合内容が宣言の期待と意味的に一致するかはoracle_presence検査の主張に含めない。

### SPEC-163

意味の疑義は検査ではなくエスカレーション（§11）の領分である。

### SPEC-164

実行形態別の判定規則を設けない。

### SPEC-S024 5.5 決定論的に検出可能な不成立構造

*導出元: REQ-S024*

### SPEC-165

`rust-cargo` adapterのStatic Audit capabilityは、§8.3の不成立構造を決定論的に検出する。

### SPEC-166

判定は保守的に行う。

### SPEC-167

共通契約がRust構文を要求しない。

*導出元: R-3*

*引用: 要件定義 R-3、§8.3*

## SPEC-S025 6. 証拠

*導出元: REQ-S019*

### SPEC-168

証拠は検証対象の内容ハッシュに束縛される。

*導出元: REQ-115, REQ-116, REQ-117, REQ-118, REQ-119, REQ-120*

*引用: 要件定義 §6*

### SPEC-169

鮮度の独立検査は設けない。

## SPEC-S026 7. 判定権威

*導出元: REQ-S020, REQ-S054*

### SPEC-170

テスト合否の判定権威は、当該adapterのテストランナーにある。

*導出元: REQ-121, REQ-122, REQ-123, REQ-124*

*引用: 要件定義 §7*

### SPEC-171

本システムは合否を判定しない。

### SPEC-172

本システムはテストランナーの結果を証拠として消費する。

### SPEC-173

実行の起動は本システムから行ってよい。

### SPEC-174

実行の起動を本システムから行っても、判定はしない。

### SPEC-175

`vtest` は照合（宣言・実装・証拠の一致検査＝§5の4検査）を行う。

### SPEC-176

`vtest` はテストの合否そのものを再判定しない。

### SPEC-177

`target_binding`（§5.3）はランナーの `PASS` を前提に、その `PASS` が検証対象の実行を伴ったかを問う独立の照合である。

## SPEC-S027 8. Test の検証成立性

*導出元: REQ-S021, REQ-S025*

### SPEC-S028 8.1 成立と算入の独立

### SPEC-178

Testとして成立しているかの検査（§8）と、仕様適合性の証拠として算入するかの判定は独立である。

*導出元: REQ-125, REQ-126, REQ-127, REQ-128, REQ-129, REQ-130*

*引用: 要件定義 §8.1*

### SPEC-179

全Testを管理対象とすること（`chain_integrity`）と証拠算入（成立性）は別系統とする。

### SPEC-S029 8.2 成立性の必要条件

### SPEC-180

成立条件の確認方法は検証対象・実行形態・観測方法に応じて異なってよい（証明方法への非依存）。

### SPEC-181

特定形態固有の確認方法を別形態へ一律要求しない。

### SPEC-182

`oracle_presence` の信頼基盤の具体的範囲（標準assert構文・framework failure semantics・設定による列挙）と委譲確認の方法は詳細設計へ委譲する。

*導出元: REQ-309, REQ-310, REQ-311, REQ-312, REQ-313, REQ-314, REQ-315, REQ-316, REQ-317, REQ-318, REQ-319, REQ-320, REQ-321, REQ-322, REQ-323, REQ-324, REQ-325, REQ-326, REQ-327, REQ-328, REQ-329, REQ-330, REQ-331, REQ-332*

*引用: 要件定義 §28*

### SPEC-S030 8.3 決定論的に検出可能な不成立構造

### SPEC-183

以下は §8.2 の成立条件を満たさないことを、宣言の中身に依らず決定論的に検出できる例である。

> 例示の前提宣言：`VO-EX-001: parse_u32 は不正な 10 進文字列に対して Err を返す` / `Test 宣言: covers = [VO-EX-001], 検証対象 = parse_u32`。
>
> 失敗し得ない（成否判定が定数）：
>
> ```rust
> #[test]
> fn rejects_invalid_decimal() {
>     parse_u32("12a");
>     assert!(true);
> }
> ```
>
> 検証対象の振る舞いを生じさせるが、その観測を成否判定に利用していない：
>
> ```rust
> #[test]
> fn rejects_invalid_decimal() {
>     parse_u32("12a");
>     let x = 1 + 1;
>     assert_eq!(x, 2);
> }
> ```
>
> 観測同士の自己比較で、成否が検証対象の振る舞いに依存しない：
>
> ```rust
> #[test]
> fn rejects_invalid_decimal() {
>     let r = parse_u32("12a");
>     assert_eq!(r, r);
> }
> ```

### SPEC-184

code fragmentはRustによる例示である。

### SPEC-185

共通契約がRust構文を要求しない。

*導出元: R-3*

*引用: R-3*

### SPEC-186

`static_audit` に相当する判定は §5.4/§5.5 の `oracle_presence` として現れる。

### SPEC-187

`static_audit` に相当する判定は独立した検査項目を新設しない。

## SPEC-S031 9. 検証対象と Source Target

*導出元: REQ-S025*

### SPEC-S032 9.1 検証対象

### SPEC-188

検証対象は、そのTestが検証成立性（§8）を証明しようとする対象、すなわち宣言された「何の時にどうなる」の主語である。

### SPEC-189

検証対象は実装constructに限定しない。

### SPEC-190

外部から観測可能な契約・境界上の振る舞いも検証対象にできる。

*導出元: REQ-144, REQ-145, REQ-146, REQ-147, REQ-148*

*引用: 要件定義 §9.1*

### SPEC-S033 9.2 Source Target の識別

### SPEC-191

1つのTestは1件以上のSource Targetを宣言できる。

### SPEC-192

複数targetを宣言した場合も代表1件へ縮約しない。

### SPEC-193

ソースコードへの恒久ID埋め込みは必須としない。

### SPEC-194

具体的構文・namespace・symbol種別は詳細設計へ委譲する。

### SPEC-195

共通契約が特定言語構造を必須としない。

*導出元: R-3*

*引用: R-3*

### SPEC-S034 9.3 実装 traceability

### SPEC-196

検証対象とは別に、Testまたは検証対象から関連するSource Targetへのtraceabilityを保持できる。

### SPEC-197

Testまたは検証対象から関連するSource Targetへのtraceabilityは任意であり、影響分析・逆引きに利用できる。

### SPEC-198

traceabilityは関連付けであって実装対応の証明ではない。

### SPEC-199

検証対象と実装traceabilityは別の関係として扱う。

### SPEC-200

Source Targetとの関係を持つTestについて、TestからSourceを検索できる。

### SPEC-201

Source Targetとの関係を持つTestについて、Sourceから関連Testを逆引きできる。

## SPEC-S035 10. Verification Obligation

*導出元: REQ-S029, REQ-S034*

### SPEC-202

VOは独立して「この条件が成立するか」と検証可能な仕様上の命題とする。

### SPEC-203

VOの粒度をassert文・test function・テストファイルなどのコード構文で決めない。

*導出元: REQ-162, REQ-163*

*引用: 要件定義 §10.1*

### SPEC-204

仕様は、テストで十分な網羅性を確認できる単位までVOへ分解できる。

### SPEC-205

本システムは分解を表現・保持するデータモデルを提供する。

### SPEC-206

分解が十分かの判定は本システムの検査ではなくエスカレーション（§11）の領分である。

*導出元: REQ-164, REQ-165, REQ-166*

*引用: 要件定義 §10.2*

### SPEC-207

VOは階層構造を持てる。

### SPEC-208

初回登録時の階層化を必須としない。

### SPEC-209

flatなVO群と階層化VO群の双方を扱う。

### SPEC-210

flatなVOを再帰分解・階層化する操作を提供する。

*導出元: REQ-167, REQ-168, REQ-169*

*引用: 要件定義 §10.3*

### SPEC-211

TestはVOの検証実装単位でありVOそのものではない。

*導出元: REQ-170, REQ-171, REQ-172, REQ-173*

*引用: 要件定義 §10.4*

### SPEC-212

VOには検証軸（dimension）と同値/境界値partitionを定義できる。

### SPEC-213

検証軸とpartitionの定義はすべてのVOへは要求しない。

### SPEC-214

何をもって十分とするかの判定は本システムの検査ではない（→§11）。

### SPEC-215

各観点の独立検証、または必要と定義された組合せ空間の検証のいずれかを表現・確認できる。

*導出元: REQ-174, REQ-175, REQ-176, REQ-177, REQ-178, REQ-179*

*引用: 要件定義 §11*

### SPEC-216

partition・組合せcoverage方針の具体的保存形式・語彙は詳細設計へ委譲する。

*導出元: REQ-309, REQ-310, REQ-311, REQ-312, REQ-313, REQ-314, REQ-315, REQ-316, REQ-317, REQ-318, REQ-319, REQ-320, REQ-321, REQ-322, REQ-323, REQ-324, REQ-325, REQ-326, REQ-327, REQ-328, REQ-329, REQ-330, REQ-331, REQ-332*

*引用: 要件定義 §28*

## SPEC-S036 11. 発見・意味判定のエスカレーションと判断記録

*導出元: REQ-S035, REQ-S046*

### SPEC-217

本システムは、宣言されていない義務・網羅漏れ・宣言と実装の意味のずれを、自ら発見・裁定しない。

### SPEC-218

発見者・裁定者は外部（人間またはAgent）である。

*導出元: REQ-180, REQ-181, REQ-182, REQ-183, REQ-184, REQ-185, REQ-186, REQ-187, REQ-188, REQ-189, REQ-190, REQ-191, REQ-192*

*引用: 要件定義 §12*

### SPEC-219

本システムの責務は、データ形態の提供（§11.1）、エスカレーション（§11.2）、判断の記録と再検証（§11.3）の3つに限る。

### SPEC-S037 11.1 データ形態の提供

### SPEC-220

外部の発見者が判断できる構造化出力（要求該当箇所と対応概念のペア、宣言鎖と検査結果、対象外とした範囲）を提供する。

### SPEC-221

`vtest` 自身はLLM APIを呼ばない。

### SPEC-222

`vtest` は意味判定・候補生成を検証成立条件にしない。

*導出元: REQ-224, REQ-225, REQ-226, REQ-227*

*引用: 要件定義 §17.2*

### SPEC-223

外部AI/Agentによる補助・提案は許容する。

### SPEC-224

外部AI/Agentの能力を成立条件にしない。

### SPEC-S038 11.2 エスカレーション

### SPEC-225

機械が決定論で確定できない疑義は `UNKNOWN` として、意味判定できる者へ引き渡す。

### SPEC-226

`UNKNOWN` は正常動作としての降参である。

### SPEC-227

`UNKNOWN` はエラー処理のフォールバック先に使わない（§4.4）。

### SPEC-S039 11.3 判断の記録と再検証

### SPEC-228

`UNKNOWN` に対して外部（人間または判断可能Agent）が判断できる。

### SPEC-229

判断記録は追跡可能とする。

### SPEC-230

判断記録の理由・根拠・evidence noteは保存できる構造とする。

### SPEC-231

外部の人間/Agentが判断し、判断結果（decision＋任意の理由）を提出する。

### SPEC-232

判断記録の生成・保存の構造化プロトコルは検証状態のゲートではない。

### SPEC-233

判断済みと承認済みは区別する（判断済み ≠ 承認済み）。

### SPEC-234

判断は承認なしでも記録できる。

### SPEC-235

正式採用は§17の別段階である。

### SPEC-236

判断記録と承認記録は同一entityであることを要求しない（別entityでありうる）。

### SPEC-237

エスカレーション出力・判断記録の具体的schema、判断待ち情報（§18.3）の構造schemaと取得インターフェース、判断の多重度は詳細設計へ委譲する。

*導出元: REQ-309, REQ-310, REQ-311, REQ-312, REQ-313, REQ-314, REQ-315, REQ-316, REQ-317, REQ-318, REQ-319, REQ-320, REQ-321, REQ-322, REQ-323, REQ-324, REQ-325, REQ-326, REQ-327, REQ-328, REQ-329, REQ-330, REQ-331, REQ-332*

*引用: 要件定義 §28*

## SPEC-S040 12. Test Registry

*導出元: REQ-S009, REQ-S036*

### SPEC-238

各Testは安定したTest IDによって識別可能とする。

### SPEC-239

Test IDをハンドルとして、Test Intent・`covers`（VO参照）・検証対象・Source Target・Location・判断記録・Execution Evidenceを検索可能とする。

*導出元: REQ-193, REQ-194, REQ-195, REQ-196, REQ-197, REQ-198, REQ-199, REQ-200, REQ-201, REQ-202*

*引用: 要件定義 §13*

### SPEC-240

診断severityと検証状態を混同しない。

### SPEC-241

Testの存在理由による分類（role / anchor / anchor_rationale等）と、それに基づく `covers` 件数の可変制約はv0.1では設けない。

## SPEC-S041 13. Test Intent

*導出元: REQ-S037*

### SPEC-242

Testには、その実装コードだけを読まなくても、何を検証するか・どのVOに対応するか・何を入力条件とするか・何を期待するかを判断できる情報を関連付けられる。

### SPEC-243

Test IntentはTest Entityの付随情報であり、宣言鎖のノードではない。

*導出元: REQ-203, REQ-204, REQ-205*

*引用: 要件定義 §14*

### SPEC-244

具体的入力値をTest IntentまたはVOへ含めることを許容する。

### SPEC-245

具体的入力値をTest IntentまたはVOへ含めることは必須としない。

## SPEC-S042 14. Parameterized / Table-Driven Test

*導出元: R-3, REQ-S038*

### SPEC-246

table-drivenの論理形式を正式に許容する。

### SPEC-247

adapterが識別したtable-driven test construct全体を一つのTestとして登録できる。

### SPEC-248

内部の各caseを独立Test IDへ分解することを必須としない。

### SPEC-249

cases集合がVOに必要な入力空間を十分に代表・網羅しているかは§11の発見・判定の対象とする。

## SPEC-S043 15. Structured Test Operation

*導出元: P-004, REQ-S039*

### SPEC-S044 15.1 desired state 方式

### SPEC-250

Create / Editの入力は差分操作ではなくあるべき状態（desired state）とする。

### SPEC-251

利用者は「TEST-Xはこの状態である」を宣言する。

### SPEC-S045 15.3 編集境界

### SPEC-252

Test外部の通常ソースコード・helper・fixtureの編集は責務外とする。

### SPEC-253

Test外部の通常ソースコード・helper・fixtureの編集操作は提供しない。

*導出元: REQ-210, REQ-211, REQ-212, REQ-213, REQ-214, REQ-215, REQ-216, REQ-217, REQ-288, REQ-289, REQ-290, REQ-291, REQ-292, REQ-293, REQ-294*

*引用: 要件定義 §16、§25 OOS-003*

### SPEC-254

通常のwrite/editツールや人間による直接ソース編集は完全禁止しない。

### SPEC-255

公式経路の提供により誤編集・更新忘れ・複数Test同時変更の事故を低減する。

### SPEC-256

直接編集による不整合も検証（§5.1）で検出可能とする。

### SPEC-257

source declarationが正典であるため、`covers` / `targets` の「同期漏れ」は構造的に発生しない。

*導出元: REQ-210, REQ-211, REQ-212, REQ-213, REQ-214, REQ-215, REQ-216, REQ-217*

*引用: 要件定義 §16*

### SPEC-S046 15.4 Form Schema

### SPEC-258

Rust関数単体Test用と小規模結合Test用の組込schemaを同梱する。

### SPEC-259

CLI・MCPのいずれからも同一schemaを消化できる。

### SPEC-260

境界値・partitionの必須入力化は組込Formでは設けない。

### SPEC-261

境界値・partitionの必須入力化はuser-defined Form Schemaが指定できる。

*導出元: REQ-174, REQ-175, REQ-176, REQ-177, REQ-178, REQ-179, REQ-309, REQ-310, REQ-311, REQ-312, REQ-313, REQ-314, REQ-315, REQ-316, REQ-317, REQ-318, REQ-319, REQ-320, REQ-321, REQ-322, REQ-323, REQ-324, REQ-325, REQ-326, REQ-327, REQ-328, REQ-329, REQ-330, REQ-331, REQ-332*

*引用: 要件定義 §28、§11*

## SPEC-S047 16. 仕様入力（文書層）

*導出元: REQ-S045*

### SPEC-262

仕様ソースとして、ソースコードより上流に位置する成果物（要件定義・基本仕様・詳細設計・API Schema・Protocol Specification・型/データ仕様・DB schema・その他の機械可読仕様）を利用可能とする。

### SPEC-263

取り込まれた上流成果物は §2.2 の `document` ノードとして登録される。

### SPEC-264

取り込まれた上流成果物はcontent_hashとderives_fromを持つ。

*導出元: REQ-233, REQ-234, REQ-235*

*引用: 要件定義 §18*

### SPEC-265

対象ソースコード内のdoc commentを、その対象実装自身の正当性を証明する唯一の仕様根拠として使用しない。

*導出元: REQ-233, REQ-234, REQ-235*

*引用: 要件定義 §18*

### SPEC-266

文書の具体的入力フォーマットと登録方式、根の指定方式は詳細設計へ委譲する。

*導出元: REQ-309, REQ-310, REQ-311, REQ-312, REQ-313, REQ-314, REQ-315, REQ-316, REQ-317, REQ-318, REQ-319, REQ-320, REQ-321, REQ-322, REQ-323, REQ-324, REQ-325, REQ-326, REQ-327, REQ-328, REQ-329, REQ-330, REQ-331, REQ-332*

*引用: 要件定義 §28*

## SPEC-S048 17. 承認

*導出元: REQ-S018, REQ-S046*

### SPEC-267

承認とは、判断（§11の判断記録を含む）または方針を「この内容で進める」と正式に認め確定状態にすることである。

### SPEC-268

判断済みと承認済みは区別する（判断済み ≠ 承認済み）。

### SPEC-269

未承認の判断は承認済みより弱い。

*導出元: REQ-236, REQ-237, REQ-238, REQ-239, REQ-240, REQ-241, REQ-242, REQ-243, REQ-244, REQ-245, REQ-246, REQ-247, REQ-248, REQ-249, REQ-250, REQ-251, REQ-252*

*引用: 要件定義 §19*

### SPEC-270

VO等の検証成果物について確定・承認状態を表現可能とする。

### SPEC-271

承認は対象または参照する判断（judgment reference）に承認済み状態を与える。

### SPEC-272

§11の `UNKNOWN` 判断も承認対象になり得る。

### SPEC-273

判断できることと正式承認は別段階である。

### SPEC-274

承認記録は§11の判断記録と同一entityであることを要求しない。

### SPEC-275

承認主体を人間に限定しない。

### SPEC-276

Agentも承認権限を持ち得る（Human / Verification Agent / Reviewer Agent / PM Agent等）。

### SPEC-277

全Agentが承認権限を持つことは要求しない。

### SPEC-278

一般作業Agentが承認権限を持つべきとも要求しない。

### SPEC-279

誰がどの対象・範囲を承認できるか（approval authority）はプロジェクト側で定義可能とする。

### SPEC-280

承認は検証状態と独立の別軸である（§4.5）。

### SPEC-281

具体的な承認ロール・必要承認数・権限schema・承認workflowの状態遷移は詳細設計へ委譲する。

*導出元: REQ-309, REQ-310, REQ-311, REQ-312, REQ-313, REQ-314, REQ-315, REQ-316, REQ-317, REQ-318, REQ-319, REQ-320, REQ-321, REQ-322, REQ-323, REQ-324, REQ-325, REQ-326, REQ-327, REQ-328, REQ-329, REQ-330, REQ-331, REQ-332*

*引用: 要件定義 §28*

## SPEC-S049 18. 途中導入と既存プロジェクト対応

*導出元: R-5, REQ-S040*

### SPEC-282

本システムはプロジェクト開始時からの導入を前提としない。

### SPEC-283

開発途中または既存プロジェクトへ後から導入できる。

*導出元: R-5*

*引用: 要件定義 R-5、§17*

### SPEC-S050 18.1 既存資産の可視化

### SPEC-284

既に大量のソースコードとTestが存在するプロジェクトを検証対象として扱える。

### SPEC-285

既存の文書・Source・Testを読み取り、VOの存在状況・既存TestとVOの対応・Testの不足・検証成立性・宣言との不一致を可視化する。

### SPEC-286

VOが確定していない範囲を含むプロジェクトも読み取れる。

### SPEC-287

document / VO、Test metadata宣言、判断記録、Evidenceの一部が欠ける状態も読み取り可能とする。

### SPEC-S051 18.2 導入時の責務境界

### SPEC-288

決定論的に処理可能な作業について人間の反復手入力を必須としない。

### SPEC-289

要求・要件・仕様・VO等の意味上の定義や対応関係を決定する責任はプロジェクト側（開発者・設計者・PM等）にある。

### SPEC-290

本システムが意味判断・候補生成を行うことを必須要件としない。

### SPEC-291

外部AI/Agentによる補助・提案は許容する。

### SPEC-292

外部AI/Agentの能力を検証成立条件にしない。

*導出元: REQ-224, REQ-225, REQ-226, REQ-227*

*引用: 要件定義 §17.2*

### SPEC-S052 18.3 判断待ち情報の構造化

### SPEC-293

導入時・検証時に生じる、未確定事項・不整合・未検証事項・機械的に確認済みの事実・外部判断が必要な事項を、機械可読な構造として保持・取得可能とする。

### SPEC-294

表示形式（表・GUI等）は要件でなく詳細設計へ委譲する。

*導出元: REQ-228, REQ-229, REQ-309, REQ-310, REQ-311, REQ-312, REQ-313, REQ-314, REQ-315, REQ-316, REQ-317, REQ-318, REQ-319, REQ-320, REQ-321, REQ-322, REQ-323, REQ-324, REQ-325, REQ-326, REQ-327, REQ-328, REQ-329, REQ-330, REQ-331, REQ-332*

*引用: 要件定義 §17.3、§28*

### SPEC-S053 18.4 導入難度の規模非依存

### SPEC-295

プロジェクト規模が大きいこと自体とは別の理由で導入難度が構造的に増大する設計を避ける。

### SPEC-296

プロジェクト規模が大きいこと自体とは別の理由で導入難度が構造的に増大する設計を避けることは、強い不変条件ではなく設計原則とする。

### SPEC-297

物量増加に伴う処理量・作業量の増加は許容する。

*導出元: REQ-230, REQ-231, REQ-232*

*引用: 要件定義 §17.4*

## SPEC-S054 19. トレーサビリティと役割別 projection

*導出元: REQ-S007*

### SPEC-298

関係型を単一へ潰さず、横断してトレース可能にする。

*導出元: REQ-036, REQ-037, REQ-038, REQ-039, REQ-040, REQ-041, REQ-042, REQ-043, REQ-044, REQ-045, REQ-046, REQ-047, REQ-048, REQ-049*

*引用: 要件定義 §3.4、NFR-003*

### SPEC-299

文書間はderives_from、VO→Testはcovers、Test↔実装は検証対象／実装traceabilityのように性質の異なる関係型を区別する。

### SPEC-300

関係型そのものの意味論的増殖は求めない。

### SPEC-301

説明文を付加・保存できるデータ構造とする。

### SPEC-302

最小の意味単位「上流ノード → 関係 → 下流ノード」を任意のノードから取得できる。

### SPEC-303

必要に応じて上流／下流へ連続して辿れる。

### SPEC-304

プロジェクト全体のトレーサビリティ構造も取得できる。

### SPEC-305

常に全チェーンを表示することは求めない。

### SPEC-306

同一のトレーサビリティ構造から、利用者の役割または利用目的に応じて参照対象・関係・集約粒度を変えたprojectionを取得・提示できる。

> 例：PM は上位の document・VO の状態と未確定/NG、Tester は VO・Test・検証対象・Evidence・未実施/失敗理由、Coder は実装から関連 Test・VO・上流文書へのトレース。

### SPEC-307

役割を固定enumやモード名として仕様化することは本書では行わない。

### SPEC-308

preset・UI・モード体系は詳細設計へ委譲する。

*導出元: REQ-309, REQ-310, REQ-311, REQ-312, REQ-313, REQ-314, REQ-315, REQ-316, REQ-317, REQ-318, REQ-319, REQ-320, REQ-321, REQ-322, REQ-323, REQ-324, REQ-325, REQ-326, REQ-327, REQ-328, REQ-329, REQ-330, REQ-331, REQ-332*

*引用: 要件定義 §28*

## SPEC-S055 20. フェーズゲートと進行条件

*導出元: REQ-S057*

### SPEC-309

プロジェクト側が登録したフェーズ・工程・ゲートの進行条件について、現在の検証状態（§4）と承認（§17）が通過条件を満たすかを評価・提示できなければならない（MUST）。

*導出元: REQ-302, REQ-303, REQ-304, REQ-305, REQ-306, REQ-307, REQ-308*

*引用: 要件定義 §26.4*

### SPEC-310

検証状態と承認は独立の軸である（§4.5）。

### SPEC-311

ゲートは両者の組合せを進行条件にできる。

### SPEC-312

ゲート条件の定義を受理できる。

> 通常開発中   : verification = PASS で進行可、approval 不要
> Release gate : verification = PASS + Reviewer approval
> Delivery gate: verification = PASS + Owner / PM approval

### SPEC-313

本システムの責務はゲート条件が現在満たされているかの評価・提示に限る。

### SPEC-314

フェーズのライフサイクル管理・工程の自動遷移は責務外とする。

*導出元: REQ-288, REQ-289, REQ-290, REQ-291, REQ-292, REQ-293, REQ-294, REQ-302, REQ-303, REQ-304, REQ-305, REQ-306, REQ-307, REQ-308*

*引用: 要件定義 §26.4、§25 OOS-004*

### SPEC-315

「Releaseフェーズへ遷移させる」のではなく「Release gateの条件を現在満たしている」を提示する。

### SPEC-316

具体的なフェーズ名・承認ロール・必要承認数・権限schema・進行条件定義は詳細設計へ委譲する。

*導出元: REQ-309, REQ-310, REQ-311, REQ-312, REQ-313, REQ-314, REQ-315, REQ-316, REQ-317, REQ-318, REQ-319, REQ-320, REQ-321, REQ-322, REQ-323, REQ-324, REQ-325, REQ-326, REQ-327, REQ-328, REQ-329, REQ-330, REQ-331, REQ-332*

*引用: 要件定義 §28*

## SPEC-S056 21. テスト実行と Execution Evidence

*導出元: REQ-S019, REQ-S054*

### SPEC-S057 21.1 Evidence の鮮度（ハッシュ束縛による設計制約）

### SPEC-317

鮮度は独立検査ではなく§6のハッシュ束縛により満たす。

## SPEC-S058 22. 完全検証・集約・報告

*導出元: REQ-S053*

### SPEC-S059 22.1 完全検証 OK

### SPEC-318

利用者向け簡易出力は `OK` / `NG` の二値とする。

### SPEC-319

完全検証の検査集合はこの4検査に固定する。

### SPEC-320

検査の部分集合を指定した実行は限定scopeである。

### SPEC-S060 22.2 集約

### SPEC-321

Test単位の結果をVO・Feature・document単位へ集約可能とする。

### SPEC-322

集約はfail-closedを基本とする。

### SPEC-323

詳細出力では子の個別値をすべて確認できる。

### SPEC-S061 22.3 報告

### SPEC-324

NGの場合、どのエンティティの・どの検査が・どの状態で・どの診断ラベルとともに落ちたかを掘り下げ可能とする。

*導出元: REQ-297, REQ-298*

*引用: 要件定義 §26.2、NFR-006*

### SPEC-325

簡易出力は総合OK / NGとする。

### SPEC-326

詳細出力は、任意ノードからの局所／経路／全体トレース（§19）に沿ったツリー表示とする。

### SPEC-327

詳細出力では非 `PASS` の根拠（判断記録・Evidenceへの参照）を辿れる。

### SPEC-328

`covers` を持つTestはVOの子として表示する。

### SPEC-329

管理下にある事実と、いずれのVOへも寄与しない事実の双方を出力から確認できる状態にする。

### SPEC-330

人間向けテキストと機械可読JSONの両方を出力できる。

*引用: 要件定義 NFR-007 / NFR-008*

## SPEC-S062 23. スキャンと整合性検査

*導出元: REQ-S036, REQ-S050*

### SPEC-331

診断severityと検証状態を混同しない。

### SPEC-332

content_hash照合は決定論的に解決する。

### SPEC-333

参照位置の意味的妥当性・取り込み完全性は検査対象としない。

### SPEC-334

参照位置の意味的妥当性・取り込み完全性は必要ならエスカレーション（§11）で扱う。

## SPEC-S063 24. データ保存の基本方針

*導出元: REQ-S050, REQ-S058*

### SPEC-S064 24.2 並列編集耐性の設計原則

### SPEC-335

Relationレコードは不変とする。

### SPEC-336

マージ後の論理的不整合（ID衝突、dangling reference、承認の失効）はスキャンと整合性検査で検出する（§23）。

### SPEC-337

並列編集耐性では部分書込みの検出・修復は行わない。

### SPEC-338

Test ID衝突・dangling referenceの検出、派生indexの再構築、Testと関連情報の同期を人間/Agentの記憶だけに依存させないことは§23と§24.3で担保する。

### SPEC-339

具体的な物理保存方式は詳細設計へ委譲する。

*導出元: REQ-309, REQ-310, REQ-311, REQ-312, REQ-313, REQ-314, REQ-315, REQ-316, REQ-317, REQ-318, REQ-319, REQ-320, REQ-321, REQ-322, REQ-323, REQ-324, REQ-325, REQ-326, REQ-327, REQ-328, REQ-329, REQ-330, REQ-331, REQ-332*

*引用: 要件定義 §28*

### SPEC-S065 24.3 派生情報の再構築

### SPEC-340

検証グラフ、逆引きインデックス、集約結果はすべて正典からの導出物である。

### SPEC-341

検証グラフ、逆引きインデックス、集約結果は `vtest scan` によりいつでも再構築できる。

*引用: 要件定義 NFR-004*

### SPEC-342

キャッシュ / indexの具体的データ形式は詳細設計へ委譲する。

*導出元: REQ-309, REQ-310, REQ-311, REQ-312, REQ-313, REQ-314, REQ-315, REQ-316, REQ-317, REQ-318, REQ-319, REQ-320, REQ-321, REQ-322, REQ-323, REQ-324, REQ-325, REQ-326, REQ-327, REQ-328, REQ-329, REQ-330, REQ-331, REQ-332*

*引用: 要件定義 §28*

## SPEC-S066 25. 利用者別ユースケース

*導出元: REQ-S047*

### SPEC-343

具体的role taxonomy・presetの固定は行わない。

### SPEC-344

役割別の参照観点は§19のprojectionとして提供する。

### SPEC-345

Coder AIはMCP経由で、担当したVO / Testをscopeに指定して検証する。

> 要件定義 §20 の利用者ごとに想定する主経路を示す。

### SPEC-346

Coder AIは自身の変更が要求された検証を満たしたか確認する。

### SPEC-347

DeveloperはCLIで、Structured Test Operationによるテスト作成・変更を行う。

### SPEC-348

Developerは検証結果の詳細表示を行う。

### SPEC-349

CIはCLI（非対話）で `vtest verify` を同一revisionで再実行し、終了コードで判定する。

### SPEC-350

CIはEvidenceを成果物として保存する。

### SPEC-351

Reviewer AIはMCP経由で、Coderが提出したEvidence・判断記録と、自身の再検証結果を照合する。

### SPEC-352

PM / PM AgentはCLIまたはMCPで、documentまたはVO単位の集約結果からNG箇所へ掘り下げる。

## SPEC-S067 26. インターフェース概要

*導出元: REQ-S049, REQ-S058*

### SPEC-353

GUIは必須要件としない。

*導出元: REQ-267, REQ-268, REQ-269, REQ-270*

*引用: 要件定義 §22*

### SPEC-S068 26.1 CLI コマンド体系

### SPEC-354

コマンドの完全仕様（引数・出力・終了コード）は詳細設計で定める。

### SPEC-355

本書ではコマンド一覧と責務を確定する。

### SPEC-356

ゲート充足は検証状態とは別軸の評価である。

### SPEC-357

出力では検証状態とゲート満否を別に提示する。

### SPEC-358

終了コード体系の詳細は詳細設計へ委譲する。

*導出元: REQ-105, REQ-106, REQ-107, REQ-108, REQ-309, REQ-310, REQ-311, REQ-312, REQ-313, REQ-314, REQ-315, REQ-316, REQ-317, REQ-318, REQ-319, REQ-320, REQ-321, REQ-322, REQ-323, REQ-324, REQ-325, REQ-326, REQ-327, REQ-328, REQ-329, REQ-330, REQ-331, REQ-332*

*引用: 要件定義 §5.4、§28*

### SPEC-S069 26.2 MCP ツール体系

### SPEC-359

MCPサーバはCLIと同一のコア機能を呼び出す。

### SPEC-360

ツールの完全な入出力スキーマは詳細設計で定める。

### SPEC-361

すべてのツールは非対話で完結する。

*引用: 要件定義 NFR-007*

### SPEC-362

CLIとMCPは同じadapter registry composition・JSON envelope・adapter選択エラーを利用する。

### SPEC-363

CLI command体系・MCP tool体系の詳細は詳細設計へ委譲する。

*導出元: REQ-309, REQ-310, REQ-311, REQ-312, REQ-313, REQ-314, REQ-315, REQ-316, REQ-317, REQ-318, REQ-319, REQ-320, REQ-321, REQ-322, REQ-323, REQ-324, REQ-325, REQ-326, REQ-327, REQ-328, REQ-329, REQ-330, REQ-331, REQ-332*

*引用: 要件定義 §28*

## SPEC-S070 27. 対応範囲と adapter 境界

*導出元: R-2, R-3, REQ-S048*

### SPEC-364

検証契約・ID・ハッシュ・Evidence・状態・集約の概念モデルは、言語およびtest runnerに依存しない。

*導出元: R-3, REQ-258, REQ-259, REQ-260, REQ-261, REQ-262, REQ-263, REQ-264, REQ-265, REQ-266*

*引用: 要件定義 §21、R-3*

### SPEC-365

共通契約は特定言語の構文・構造を必須としない。

### SPEC-366

`rust-cargo` はRust・Rust function unit test・小規模なintegration testを対象とする。

### SPEC-367

`rust-cargo` 以外のproduction language adapterはv0.1の提供範囲に含めない。

*導出元: R-2*

*引用: 要件定義 R-2*

### SPEC-368

能力不足で確認できない項目は§8条項4に従い扱う。

## SPEC-S071 28. 非機能要求への対応方針

*導出元: REQ-S051*

### SPEC-369

NFR-001並列性への対応は、1レコード1ファイル、ULIDファイル名、不変Relation、中央台帳の不在とする（§24.2）。

*導出元: REQ-280, REQ-281, REQ-282, REQ-283, REQ-284, REQ-285, REQ-286, REQ-287*

*引用: 要件定義 §24*

### SPEC-370

NFR-002再現性への対応は、Evidenceのリビジョン束縛、決定論的解析の再実行可能性、scanによる全再構築とする（§21）。

### SPEC-371

NFR-003追跡可能性への対応は、document→VO→Test→SRC→Evidenceの双方向グラフ、任意ノードからの局所／経路／全体取得とする（§19、§23）。

### SPEC-372

NFR-004再構築可能性への対応は、派生情報はcacheのみとし、正典から `vtest scan` で再構築することとする（§24.3）。

### SPEC-373

NFR-005 Fail Closedへの対応は、状態モデルと集約規則（§4、§22）、承認・判断の内容ハッシュ束縛（§11、§17）とする。

### SPEC-374

NFR-006説明可能性への対応は、状態・診断ラベルの分離（§4）、根拠を辿れる詳細レポート（§22.3）とする。

### SPEC-375

NFR-007自動化適性への対応は、非対話CLI・MCP、JSON出力、終了コードとする（§26）。

### SPEC-376

NFR-008人間可読性への対応は、ツリー形式の詳細出力、IDの人間可読性とする（§3.2、§22.3）。

## SPEC-S072 29. スコープ外

*導出元: REQ-S052*

### SPEC-377

要件定義 §25 のスコープ外事項に対応する機能を本書では定義しない。

*導出元: REQ-288, REQ-289, REQ-290, REQ-291, REQ-292, REQ-293, REQ-294*

*引用: 要件定義 §25*

### SPEC-378

文書層は§2.2の通りリンクとハッシュのみを扱う（OOS-001仕様書同士の品質監査）。

### SPEC-379

文書内容の意味的良否を検証しない（OOS-001仕様書同士の品質監査）。

### SPEC-380

不一致はどれを正とするか決めず状態として提示する（OOS-002修正方針決定。§4）。

*導出元: P-001*

*引用: P-001*

### SPEC-381

Test Edit対象外の一般編集を管理しない（OOS-003通常ソースコード編集管理。§15.3）。

### SPEC-382

フェーズのライフサイクル管理・工程遷移は責務外とする（OOS-004開発プロセス全体の管理。§20）。

### SPEC-383

本システムはVerification Infrastructureとして機能する。

### SPEC-384

v0.1は宣言された義務の裏付けのみ検証する（OOS-005宣言されていない実装）。

### SPEC-385

v0.1は宣言されていない実装の存在を関知しない（OOS-005宣言されていない実装）。

*導出元: R-2*

*引用: R-2*

### SPEC-386

実装レイヤーの孤児検出・シンボル列挙の定義・上流文書の意味構造はv0.2のスコープとする。

## SPEC-S073 30. 詳細設計へ委譲する事項

*導出元: REQ-S058*

### SPEC-387

以下は本書の要求・要件を基に詳細設計で決定する（要件定義 §28 の23項目に対応）。

*導出元: REQ-309, REQ-310, REQ-311, REQ-312, REQ-313, REQ-314, REQ-315, REQ-316, REQ-317, REQ-318, REQ-319, REQ-320, REQ-321, REQ-322, REQ-323, REQ-324, REQ-325, REQ-326, REQ-327, REQ-328, REQ-329, REQ-330, REQ-331, REQ-332*

*引用: 要件定義 §28*

### SPEC-388

HOWは本書で発明しない。

### SPEC-389

詳細設計は、文書の具体的な入力フォーマットと登録方式を決定する（§16）。

### SPEC-390

詳細設計は、文書層の根の指定方式（orphan_detectionの除外指定。§5.2）を決定する。

### SPEC-391

詳細設計は、VO保存形式を決定する（§10、§24.1）。

### SPEC-392

詳細設計は、Test metadataの具体的annotation syntax（`rust-cargo` の `@vtest.*` 文法を含む。§22）を決定する。

### SPEC-393

詳細設計は、relationの保存形式を決定する（§3.2、§24.2）。

### SPEC-394

詳細設計は、Test ID命名規則を決定する（§3.2）。

### SPEC-395

詳細設計は、Target Reference / SRC IDの具体的識別方式を決定する（§9.2）。

### SPEC-396

詳細設計は、AST / LSP等の具体的解析技術（不成立証明・存在確認・静的到達の実装。§5.5、§8.3）を決定する。

### SPEC-397

詳細設計は、`oracle_presence` の信頼基盤の具体的範囲と委譲確認の方法を決定する（§8.2）。

### SPEC-398

詳細設計は、`target_binding` の動的計測方式を決定する（§5.3）。

### SPEC-399

詳細設計は、診断ラベルの語彙を決定する（§4.2）。

### SPEC-400

詳細設計は、終了コード体系（検証状態と内部エラーの分離。§26.1）を決定する。

### SPEC-401

詳細設計は、エスカレーション出力・判断記録・承認記録の具体的schemaを決定する（§11、§17）。

### SPEC-402

詳細設計は、CLI command体系を決定する（§26.1）。

### SPEC-403

詳細設計は、MCP tool体系を決定する（§26.2）。

### SPEC-404

詳細設計は、キャッシュ / indexの具体的データ形式を決定する（§24.3）。

### SPEC-405

詳細設計は、並列編集時の物理的保存方式を決定する（§24.2）。

### SPEC-406

詳細設計は、承認workflowの具体的状態遷移を決定する（§17）。

### SPEC-407

詳細設計は、判断待ち情報（§18.3）の具体的な構造schemaと取得インターフェースを決定する。

### SPEC-408

詳細設計は、関係リンクの任意説明（§19）の保存形式を決定する。

### SPEC-409

詳細設計は、役割別projection / view（§19）のpreset・UI・モード体系を決定する。

### SPEC-410

詳細設計は、approval authority（§17）の承認ロール・必要承認数・権限schemaを決定する。

### SPEC-411

詳細設計は、フェーズ・ゲート（§20）の具体的なフェーズ名と進行条件定義を決定する。

### SPEC-412

本書の要求・要件を基に詳細設計で決定するHOWを本書で確定しない。

## SPEC-S074 0. 本書の位置付け

*導出元: P-005*

### SPEC-413

本書は「基本仕様 v0.1」を実装可能なレベルまで具体化する。

### SPEC-414

本書は、基本仕様が定めた外部挙動の保証を変更しない。

### SPEC-415

本書と基本仕様の間に矛盾がある場合、基本仕様を正とし、本書の該当箇所を不整合として扱う。

### SPEC-416

本書は HOW（具体構文・アルゴリズム・データ構造・ID 命名・schema）を定める。

### SPEC-417

本書は、基本仕様（WHAT）に無い義務・検査・状態・文書種別・関係型を発明しない。

### SPEC-418

規範の伝播は上流から下流である。

*導出元: P-005*

*引用: 要件定義 P-005*

### SPEC-419

矛盾・不足を発見した場合は、本書を書き換えず上流へフィードバックしOwner判断を経る。

### SPEC-420

本書からの `基本仕様 §n` 参照は、再導出済み基本仕様 v0.1 の連番（§0〜§30）を指す。

### SPEC-421

本書からの `要件定義 §n` 参照は、凍結要件定義 v0.1 の連番（§1〜§28・P-001〜P-005・R-1〜R-5）を指す。

*導出元: R-1, R-5, P-001, P-005*

*引用: P-001, P-005, R-1, R-5*

### SPEC-422

正規の詳細設計は3分冊とする。

### SPEC-423

節番号は正規文書間を通した連番とする。

### SPEC-424

別紙Bは非正規のprocess文書として別に扱う。

### SPEC-425

本冊（コア設計）は正規であり、§1〜§11、§16、§17、§19を収録節とする。

> | 文書 | 位置付け | 収録節 |
> |---|---|---|
> | 本冊（コア設計） | 正規 | §1〜§11、§16、§17、§19 |
> | 別紙A（CLI・MCPインターフェース仕様） | 正規 | §12〜§15 |
> | 別紙B（実装計画） | 非正規 / process | 正規節番号を持たない |
> | 別紙C（受入仕様） | 正規 | §18 |

### SPEC-426

別紙A（CLI・MCPインターフェース仕様）は正規であり、§12〜§15を収録節とする。

### SPEC-427

別紙B（実装計画）は非正規/process文書であり、正規節番号を持たない。

### SPEC-428

別紙C（受入仕様）は正規であり、§18を収録節とする。

### SPEC-429

本冊の新設サブ節（§5.6 文書層孤児検出、§11.5 フェーズゲート、§11.6 役割別 projection、§11.7 判断待ち情報）は本冊の収録節範囲内に置き、別紙A / C の節番号を侵さない。

### SPEC-430

本書は、基本仕様が固定するCLIコマンド一覧・MCPツール一覧を増やさない。

*引用: 基本仕様 §26.1, 基本仕様 §26.2*

### SPEC-431

新設機能は既存コマンド・ツールの引数と出力で露出する。

### SPEC-432

引数・入出力の完全schemaは別紙Aが定める。

### SPEC-433

本書は意味論とデータschema、および露出点だけを確定する。

## SPEC-S075 1. 実装構成

### SPEC-S076 1.3 内容ハッシュの定義

### SPEC-434

検証対象は一般概念であり、このhashは検証対象をSource Targetとして実現した形態のidentity束縛であって、coreが「検証対象とは何か」をSource Targetに限定して定義するものではない（§1.3・§4.1）。

*引用: 基本仕様 §9.1*

## SPEC-S077 2. データディレクトリと設定

### SPEC-S078 2.1 `.verify/` レイアウト

### SPEC-435

基本仕様 §24.1 の layout をそのまま採用する。

*引用: 基本仕様 §24.1*

### SPEC-S079 2.2 `config.yaml`

### SPEC-436

統合したTest IDは全adapterでglobal uniquenessを検査する。

## SPEC-S080 3. レコードファイルスキーマ

### SPEC-S081 3.1 document レコード（`.verify/doc/DOC-*.yaml`）

### SPEC-437

`note` は付加・保存できる構造とする。

### SPEC-438

本システムは文書内容の意味的良否を検証しない。

*引用: 基本仕様 §29 OOS-001*

### SPEC-S082 3.2 VO レコード（`.verify/vo/VO-*.yaml`）

#### SPEC-S083 3.2.1 dimensions と組合せの実体化

### SPEC-439

組合せ空間の定義が仕様に対して十分かは本システムの検査ではなく、`UNKNOWN` としてエスカレーションの領分である（§8）。

*引用: 基本仕様 §11, 基本仕様 §10*

### SPEC-440

`combinations` の値が仕様に対して十分な組合せ集合かは本システムの検査ではなく、エスカレーションの領分である。

*引用: 基本仕様 §10, 基本仕様 §11*

### SPEC-S084 3.4 判断記録レコード（`.verify/decisions/<ULID>.yaml`）

*導出元: REQ-S035*

### SPEC-441

判断記録は、`UNKNOWN` に対して外部（人間または判断可能Agent）が下した判断の記録である。

*導出元: REQ-180, REQ-181, REQ-182, REQ-183, REQ-184, REQ-185, REQ-186, REQ-187, REQ-188, REQ-189, REQ-190, REQ-191, REQ-192*

*引用: 基本仕様 §11.3, 要件定義 §12*

### SPEC-442

判断記録は検査ゲートではなく、`UNKNOWN` に対する外部判断の追跡である。

### SPEC-443

判断済みと承認済みは区別する（判断済み ≠ 承認済み）（§3.5）。

### SPEC-S085 3.5 承認レコード（`.verify/approvals/<ULID>.yaml`）

*導出元: REQ-S046*

### SPEC-444

承認は検証状態と独立の別軸である。

*導出元: REQ-109, REQ-110, REQ-111, REQ-112, REQ-113, REQ-114*

*引用: 基本仕様 §4.5, 基本仕様 §17, 要件定義 §5.5*

### SPEC-445

承認は特定のエンティティ型に従属しない独立の領域である。

### SPEC-S086 3.6 Evidence レコード（`.verify/evidence/<ULID>.yaml`）

### SPEC-446

`target_coverage` は `target_binding` の動的計測（宣言対象の実行が生じたか）の結果であり、独立の検査項目ではない。

## SPEC-S087 5. Discovery orchestration設計

### SPEC-S088 5.2 エンティティモデル（vtest-model）

### SPEC-447

検証状態は `Pass` / `Fail` / `Mismatch` / `NoEvidence` / `Unknown` の5つのみである。

*導出元: REQ-085, REQ-086, REQ-087, REQ-088, REQ-089, REQ-090, REQ-091*

*引用: 基本仕様 §4.1, 要件定義 §5.1*

### SPEC-448

診断ラベルは検証状態と別軸である。

*導出元: REQ-092, REQ-093*

*引用: 基本仕様 §4.2, 要件定義 §5.2*

### SPEC-449

検査は `ChainIntegrity` / `OrphanDetection` / `TargetBinding` / `OraclePresence` の4本のみである。

*導出元: REQ-034, REQ-035, REQ-050, REQ-051, REQ-052, REQ-053, REQ-054, REQ-055, REQ-056, REQ-057, REQ-058, REQ-059, REQ-060, REQ-061, REQ-062, REQ-063, REQ-064, REQ-065, REQ-066, REQ-067, REQ-068, REQ-069, REQ-070, REQ-071, REQ-072, REQ-073, REQ-074, REQ-075, REQ-076, REQ-077, REQ-078, REQ-079, REQ-080, REQ-081, REQ-082, REQ-083, REQ-084*

*引用: 基本仕様 §5, 要件定義 §3.3, 要件定義 §4*

### SPEC-S089 5.3 検証グラフ

### SPEC-450

関係型（`derives_from` / `covers` / `targets` / 外部Relation）は横断トレース可能とするが、単一へ潰さず、また意味論的に増殖もさせない。

### SPEC-S090 5.4 整合性診断

### SPEC-451

warningはレポートに常に表示する。

### SPEC-S091 5.6 文書層 orphan_detection

*導出元: REQ-S010*

### SPEC-452

`orphan_detection` は文書層の孤児検出であり、親（上流document）を持たない `document` ノードが存在するかを問う。

*導出元: REQ-059, REQ-060, REQ-061, REQ-062, REQ-063*

*引用: 基本仕様 §5.2, 要件定義 §4.2*

### SPEC-453

`orphan_detection` の対象は文書層のみである。実装レイヤーの孤児検出（宣言されていない実装の検出）は行わない。

*導出元: R-2, REQ-292*

*引用: 要件定義 R-2, 基本仕様 §29 OOS-005*

### SPEC-454

旧モデルのW-SCAN-102（孤立VO）はVO層の警告であり、文書層 `orphan_detection` とは別物として存置する。

## SPEC-S092 6. Target Reference解決

### SPEC-S093 6.1 adapter-neutral解決contract

### SPEC-455

解決結果は「解決済み」「対象なし」「曖昧」の3状態を区別する。

## SPEC-S094 7. Static Analysis orchestrationと`rust-cargo`ルール

### SPEC-456

決定論的解析結果は正典レコードを持たず、検証のたびに現在のsource / configから再計算する派生情報である。

*導出元: P-003*

*引用: 基本仕様 P-003*

### SPEC-457

`vtest audit static`は判断記録（§8）とは別機構であり、外部判断の記録には転用しない。

### SPEC-S095 7.2 `rust-cargo` ルール一覧

### SPEC-458

静的解析は再計算派生であるため、これらのtarget別verdictと規則単位verdictは検証のたびに現在sourceから計算し、正典レコードへ永続化しない（§7.1）。

### SPEC-S096 7.3 target 到達の静的証明と runtime 証明の関係（target_binding）

### SPEC-459

`target_binding`項目値は検証時に算出する。

### SPEC-460

本節の到達要件は検証対象をSource Targetとして実現する形態に限定する（`rust-cargo`）。

> 基本仕様 §5.3「実装 construct（Source Target）を検証対象とする実行形態では…」

*引用: 基本仕様 §5.3*

### SPEC-461

検証対象をSource Targetとして宣言しない他の実行形態（外部契約・境界上の振る舞い）の確認方法は、特定形態を他形態へ一律要求せず下位仕様・後続版へ委譲する。

*導出元: REQ-064, REQ-065, REQ-066, REQ-067, REQ-068, REQ-069, REQ-070, REQ-071, REQ-072*

*引用: 要件定義 §4.3, 基本仕様 §5.3・§8.3*

### SPEC-462

本節の target 実行到達規則を普遍規則として適用しない。

### SPEC-463

本節はprocess boundaryによってDA-002到達が恒久UNKNOWNになる問題だけを解消するものであり、boundary testを完全にoracle_presence PASS可能にするものではない。

## SPEC-S097 8. 判断記録プロトコル

*導出元: REQ-S035*

### SPEC-464

本システムは、宣言されていない義務・網羅漏れ・宣言と実装の意味のずれを、自ら発見・裁定しない。

*導出元: REQ-180, REQ-181, REQ-182, REQ-183, REQ-184, REQ-185, REQ-186, REQ-187, REQ-188, REQ-189, REQ-190, REQ-191, REQ-192*

*引用: 基本仕様 §11, 要件定義 §12*

### SPEC-465

機械が決定論で確定できない疑義は`UNKNOWN`として外部（人間または判断可能Agent）へ引き渡し、その判断を判断記録（§3.4）として追跡する。

### SPEC-466

判断記録プロトコルは検証状態のゲートではない。

### SPEC-467

判断記録の受理は当該対象の検証状態を昇格させない。

*導出元: REQ-180, REQ-181, REQ-182, REQ-183, REQ-184, REQ-185, REQ-186, REQ-187, REQ-188, REQ-189, REQ-190, REQ-191, REQ-192*

*引用: 基本仕様 §11.3, 要件定義 §12*

### SPEC-S098 8.1 バンドル生成

### SPEC-468

本書が定義する判断型の値は`test-semantic` / `impl-consistency` / `case-coverage`の3種であり、これ以外の値でバンドルを生成しない。

### SPEC-469

`test-semantic`は、subjectの値域がTest IDであり、外部へ引き渡す問いは「テストコードは、covers先VOのclaimとTest Intentが宣言する振る舞いを実際に検証しているか」である。

### SPEC-470

`impl-consistency`は、subjectの値域がTest IDであり、外部へ引き渡す問いは「対象実装は宣言と一致しているか」である。

### SPEC-471

`case-coverage`は、subjectの値域をTest IDまたはVO IDとする。

### SPEC-472

`case-coverage`は、subjectがTest IDのとき、外部へ引き渡す問いは「当該Testが宣言したcases集合は、covers先VOの要求入力空間を十分に代表・網羅しているか」である。

*引用: 基本仕様 §14、§11*

### SPEC-473

`case-coverage`は、subjectがVO IDのとき、外部へ引き渡す問いは「当該VOをcoversするTest群のcases集合は、当該VOの要求入力空間を十分に代表・網羅しているか」である。

*引用: 基本仕様 §14、§11*

### SPEC-474

`case-coverage`は§11の判断対象であって§5の4検査ではない。

### SPEC-475

`case-coverage`の未判断・判断結果はいずれも4検査の値へ写像せず、§11.3の集約へ寄与しない。

### SPEC-476

外部判断が必要な事実は§11.7の判断待ち情報として提示する。

### SPEC-S099 8.3 提出スキーマ

### SPEC-477

判断記録は検証状態を変更しない（§8 冒頭）。

### SPEC-478

理由が空であることだけを根拠に判断を無効化しない。

*引用: 基本仕様 §11.3*

### SPEC-S100 8.4 提出の検証

### SPEC-479

旧モデルのreasons / claim / basis必須検査（E-AUDIT-005）、decomposition-viewpoint検査（E-AUDIT-006）、spec / req basis検査（E-AUDIT-007）は要件定義§12「理由が空であることだけを根拠に無効扱いしない」と矛盾するため、判断記録層では課さない。

*導出元: REQ-180, REQ-181, REQ-182, REQ-183, REQ-184, REQ-185, REQ-186, REQ-187, REQ-188, REQ-189, REQ-190, REQ-191, REQ-192*

*引用: 要件定義 §12*

### SPEC-S101 8.5 有効性と再判断

### SPEC-480

判断記録の有効性は判定時に評価する。

### SPEC-481

対象は`(subject, judgment_kind)`の組であり、組ごとに独立に評価する。

### SPEC-482

未確定である事実は§11.7の判断待ち情報として提示する。

### SPEC-483

同一対象に有効な判断記録が複数あってよい（再判断・多重判断）。

### SPEC-484

回数はツールとして制限しない（運用ポリシー）。

### SPEC-485

仕様・VO・Test等が変更された場合、過去の判断を現在状態へそのまま流用してはならず、現在状態に対して通常の検証（§5の4検査）を再実施する。

*導出元: REQ-180, REQ-181, REQ-182, REQ-183, REQ-184, REQ-185, REQ-186, REQ-187, REQ-188, REQ-189, REQ-190, REQ-191, REQ-192*

*引用: 基本仕様 §11.3, 要件定義 §12*

### SPEC-486

現在状態に対して通常の検証（§5の4検査）を再実施した結果は`PASS` / `FAIL` / `MISMATCH` / `NO_EVIDENCE` / `UNKNOWN`のいずれにもなり得る。

### SPEC-487

変更そのものが`UNKNOWN`を生成するのではない。

### SPEC-488

判断済みと承認済みは区別する（判断済み≠承認済み）。

### SPEC-489

判断は承認なしでも記録でき、正式採用は§3.5の承認の別段階である。

### SPEC-S102 8.6 参考プロンプト

### SPEC-490

判断エージェントのプロンプト・スキル構成はツールの責務外だが、参考として骨子を示す。

> あなたは検証対象の意味判定者である。添付のバンドルについて、以下だけを判定せよ。修正方針の提案はしない。判定事項：テストコードは、VOのclaimとTest Intentが宣言する振る舞いを実際に検証しているか。判定はaccepted / rejected / deferredのいずれかとし、判定ごとにclaim（何を確認したか）とbasis（根拠にしたバンドル内の情報への参照）を任意で列挙してよい。

### SPEC-491

判断の受理は検証状態を昇格させない。

### SPEC-492

判断は`UNKNOWN`に対する外部判断の追跡であり、検査ゲートではない（§8 冒頭、基本仕様 §11.3）。

*引用: 基本仕様 §11.3*

## SPEC-S103 10. `rust-cargo` Target Binding 動的計測

### SPEC-S104 10.3 実行モードの整理

### SPEC-493

`vtest run`は2モードを持つ。

## SPEC-S105 11. 鮮度検証と集約

### SPEC-S106 11.1 検査の評価地点

### SPEC-494

`target_binding`は評価地点をTESTとし、§7.3の合成による。

### SPEC-495

`target_binding`の未充足は§11.2の写像に従う。

### SPEC-496

`oracle_presence`は評価地点をTESTとし、§7.1の合成（DA-001 / DA-003 / DA-004 / DA-005 / DA-006）による。

### SPEC-497

本システムは意味判定・候補生成を外部の判定器へ委ねるseam（実行時に差し替え可能な意味判定・意味生成の呼出し点）を4検査の評価経路に持たない。

### SPEC-498

外部AI／Agentは判断記録（§8）の著者として`.verify/decisions/`へ記録を残す経路でのみ関与し、その記録は入力集合の一部としてファイル経由で読まれる。

### SPEC-499

完全検証の検査集合はこの4検査に固定し、設定で追加・削除できない（§2.2、基本仕様 §22.1）。

*引用: 基本仕様 §22.1*

### SPEC-500

旧モデルの12項目（`spec_coverage` / `vo_decomposition` / `vo_coverage` / `test_existence` / `static_audit` / `semantic_audit` / `impl_consistency` / `test_execution` / `runtime_result` / `target_execution` / `evidence_validity` / `test_traceability`）は検査として存在しない。

### SPEC-501

`test_existence` / `test_traceability`は`chain_integrity`へ統合した。

### SPEC-502

`static_audit`は`oracle_presence`（DA-001/003/004/005/006）と`target_binding`の静的到達（DA-002）へ分割した。

### SPEC-503

`test_execution` / `target_execution` / `runtime_result`は`target_binding`の証拠（Evidenceの存在・鮮度、`result`、`target_coverage`）へ吸収した。

### SPEC-504

`evidence_validity`は独立検査を廃し、鮮度喪失を診断ラベル`STALE`として§11.2で説明した（基本仕様 §6）。

*引用: 基本仕様 §6*

### SPEC-505

`spec_coverage` / `vo_coverage` / `vo_decomposition` / `semantic_audit` / `impl_consistency`は検査から除去し、網羅・意味の疑義は`UNKNOWN`として判断記録エスカレーションとした（§8、基本仕様 §11、要件定義 §12）。

*導出元: REQ-180, REQ-181, REQ-182, REQ-183, REQ-184, REQ-185, REQ-186, REQ-187, REQ-188, REQ-189, REQ-190, REQ-191, REQ-192*

*引用: 基本仕様 §11, 要件定義 §12*

#### SPEC-S107 11.1.1 `chain_integrity` の評価

### SPEC-506

`chain_integrity`は宣言鎖のすべてのリンクが存在し、ハッシュ照合が成立するかを問う。

*引用: 基本仕様 §5.1*

### SPEC-507

すべてのTestを管理対象とすることと、当該Testを証拠として算入すること（§7 / §10のtarget_binding / oracle_presence）は別個の条件とする。

*引用: 基本仕様 §8.1*

### SPEC-S108 11.3 集約アルゴリズム

### SPEC-508

利用者向け簡易出力は`OK` / `NG`の二値とする。

*引用: 基本仕様 §22.1*

### SPEC-509

詳細出力は任意ノードからの局所／経路／全体トレース（§11.6）に沿ったツリー表示とし、非`PASS`の根拠（判断記録・Evidenceへの参照）を辿れる。

### SPEC-510

人間向けテキストと機械可読JSONの両方を出力できる。

*引用: 基本仕様 §22.3*

### SPEC-511

covers宣言を経由しない「機能名による束ね」（ファイルパス・モジュール名・命名規約からの推定束ね）を設けない。

### SPEC-S109 11.5 フェーズゲート評価

*導出元: REQ-S057*

### SPEC-512

プロジェクト側が登録したフェーズ・工程・ゲートの進行条件について、現在の検証状態（§4.1の5状態）と承認（§3.5）が通過条件を満たすかを評価・提示できなければならない（MUST）。

*導出元: REQ-302, REQ-303, REQ-304, REQ-305, REQ-306, REQ-307, REQ-308*

*引用: 基本仕様 §20, 要件定義 §26.4*

### SPEC-513

検証状態と承認は独立の軸であり、ゲートは両者の組合せを進行条件にできる。

### SPEC-514

5状態に順序・優劣・包含関係を設けない。

### SPEC-515

本システムの責務はゲート条件が現在満たされているかの評価・提示に限る。

### SPEC-516

フェーズのライフサイクル管理・工程の自動遷移は責務外とする（§29 OOS-004）。

*導出元: REQ-302, REQ-303, REQ-304, REQ-305, REQ-306, REQ-307, REQ-308*

*引用: 基本仕様 §20, 要件定義 §26.4, OOS-004*

### SPEC-517

「Releaseフェーズへ遷移させる」のではなく「Release gateの条件を現在満たしている」を提示する。

### SPEC-518

具体的なフェーズ名・承認ロール・必要承認数・権限schemaはプロジェクト設定と別紙Aへ委譲する（基本仕様 §30）。

*引用: 別紙A, 基本仕様 §30*

### SPEC-S110 11.6 役割別 projection

*導出元: REQ-S007*

### SPEC-519

同一のトレーサビリティ構造から、利用者の役割または利用目的に応じて参照対象・関係・集約粒度を変えたprojectionを取得・提示できる。

*導出元: REQ-036, REQ-037, REQ-038, REQ-039, REQ-040, REQ-041, REQ-042, REQ-043, REQ-044, REQ-045, REQ-046, REQ-047, REQ-048, REQ-049*

*引用: 基本仕様 §19, 要件定義 §3.4*

### SPEC-520

最小の意味単位「上流ノード → 関係 → 下流ノード」を任意のノード（DOC / VO / TEST / SRC）から取得でき、必要に応じて上流／下流へ連続して辿れ、プロジェクト全体のトレーサビリティ構造も取得できる。

> 任意ノードからの取得。

### SPEC-521

常に全チェーンを表示することは求めない。

### SPEC-522

役割または利用目的に応じた参照観点をpresetとして提供する（例：PMは上位のdocument・VOの状態と未確定/NG、Testerは VO・Test・検証対象・Evidence・未実施/失敗理由、Coderは実装から関連Test・VO・上流documentへのトレース）。

### SPEC-523

役割を固定enumやモード名として本冊で仕様化せず、preset・UI・モード体系は別紙Aへ委譲する（基本仕様 §30）。

*引用: 別紙A, 基本仕様 §30*

### SPEC-524

projectionが出力する`derives_from`エッジに当該entryの`anchor`を常に同伴させることにより「どの上流条項が、どの概念（VO）へ対応するか」の対応ペアが構造化出力として取得でき、外部の発見者が未宣言の義務・網羅漏れを裁定する材料になる（基本仕様 §11.1）。

*引用: 基本仕様 §11.1*

### SPEC-S111 11.7 判断待ち情報の構造

### SPEC-525

導入時・検証時に生じる、未確定事項・不整合・未検証事項・機械的に確認済みの事実・外部判断が必要な事項を、機械可読な構造として保持・取得可能とする。

*導出元: REQ-228, REQ-229*

*引用: 基本仕様 §18.3, 要件定義 §17.3*

### SPEC-526

UNKNOWNだけでなく、検証出力全体にわたる未確定・要判断事項を横断的に集約する（表示形式は別紙A、基本仕様 §30 item 19）。

*引用: 別紙A, 基本仕様 §30 item 19, 基本仕様 §30*

## SPEC-S112 16. 並列動作と整合性

### SPEC-527

本冊の§12〜§15は別紙Aで定義する。

### SPEC-S113 16.1 ロック不要の根拠

### SPEC-528

すべての判定は「その時点の正典の読み取り」に基づき、正典が変われば次回のscan / verifyが差分を反映する。

## SPEC-S114 17. 診断・終了コード体系

### SPEC-S115 17.1 診断コード

### SPEC-529

診断コードは§5.4のスキャン診断に加えて定義する。

### SPEC-530

旧モデルの意味監査提出検査（E-AUDIT-005 / E-AUDIT-006 / E-AUDIT-007）は判断記録層への転用（§8.4）に伴い撤去する。

### SPEC-S116 17.2 終了コード

### SPEC-531

要求scopeの総合OK / NGはJSONとtextの集約出力から読み取れる（別紙A §12.1・§12.3）。

*引用: 別紙A §12.1・§12.3, 別紙A §12.1, 別紙A §12.3*

### SPEC-532

終了コードは診断severityだけでなく操作段階で決める。

### SPEC-533

検証状態と内部エラーは終了コードで分離する。

*引用: 基本仕様 §4.4、§26.1*

## SPEC-S117 19. 実装選択と提供範囲

*導出元: R-2, R-3*

### SPEC-534

demangle実装（`rustc-demangle`）の適用範囲は、基本仕様と本詳細設計の観測可能な契約を変更しない範囲で実装が選択できる。

> 次の事項は、基本仕様と本詳細設計の観測可能な契約を変更しない範囲で実装が選択できる。

### SPEC-535

`#[tokio::test]`等、属性末尾`test`以外のカスタムテスト属性への対応範囲は、基本仕様と本詳細設計の観測可能な契約を変更しない範囲で実装が選択できる。

### SPEC-536

cargo workspace外の単一クレートプロジェクトでのパス解決の細部は、基本仕様と本詳細設計の観測可能な契約を変更しない範囲で実装が選択できる。

### SPEC-537

レポートのツリー描画の細部（文字種、折返し）は、基本仕様と本詳細設計の観測可能な契約を変更しない範囲で実装が選択できる。

### SPEC-538

LSP / rust-analyzer連携によるシンボル解決は提供範囲外とする。

> 次の事項は提供範囲外とする。

### SPEC-539

永続インデックス（`cache/`の活用）は提供範囲外とする。

### SPEC-540

Relationのtombstone方式は提供範囲外とする。

### SPEC-541

`rust-cargo`以外のproduction language adapter（synthetic adapterは受入fixture専用）は提供範囲外とする。

### SPEC-542

LLM API直接呼び出しによる判断は提供範囲外とする。

### SPEC-543

rename追跡とSRC恒久IDの自動昇格支援は提供範囲外とする。

### SPEC-544

cargo-nextest対応は提供範囲外とする。

## SPEC-S118 0

### SPEC-545

参照規則・診断コード・終了コードは本冊 §17 に従う。

*引用: 本冊 §17*

### SPEC-546

本別紙は基本仕様 §26.1（CLI コマンド一覧）・§26.2（MCP ツール一覧）が確定したコマンド・ツールの引数と入出力 schema を具体化する HOW である。

*引用: 本冊 §0, 基本仕様 §26.1*

### SPEC-547

本別紙は新規コマンド・ツールを増やさない。

*引用: 本冊 §0*

### SPEC-548

本別紙は、上流（要件定義＝WHY、基本仕様＝WHAT、詳細設計本冊＝HOW 中核）に無い義務・検査・状態・文書種別・関係型を発明しない。

## SPEC-S119 12. CLI 詳細仕様

### SPEC-S120 12.1 共通仕様

*導出元: REQ-S009*

### SPEC-549

終了コードは本冊 §17.2 に従う。

*引用: 本冊 §17.2*

### SPEC-550

検証状態は5値（`PASS` / `FAIL` / `MISMATCH` / `NO_EVIDENCE` / `UNKNOWN`）である。

### SPEC-551

診断ラベルは状態に付随する原因説明であり、`MISSING` / `NOT_EXECUTED` / `NOT_CHECKED` / `STALE` である。

### SPEC-552

`NO_EVIDENCE` は状態であって診断ラベルではない。

### SPEC-553

診断ラベルを集約の代表値選択に用いず、原因説明として併記するだけとする。

*引用: 本冊 §11.3, 基本仕様 §22.2*

### SPEC-554

本 version の Test metadata は存在理由分類（旧 `role` / `anchor` / `anchor_rationale`）を持たない。

*導出元: REQ-052, REQ-053, REQ-054, REQ-055, REQ-056, REQ-057, REQ-058*

*引用: 本冊 §4.1, 基本仕様 §12, 要件定義 §4.1*

### SPEC-555

本 version はすべての管理対象 Test に `covers ≥ 1` を一律に要求する。

*導出元: REQ-052, REQ-053, REQ-054, REQ-055, REQ-056, REQ-057, REQ-058*

*引用: 本冊 §4.1, 基本仕様 §12, 要件定義 §4.1*

### SPEC-556

VO への寄与は `covers` 宣言と証拠の十分性判定だけから導出する。

### SPEC-S121 12.2 `vtest doc add / list / show`

### SPEC-557

`doc` は上流文書を総称 `document` レコードとして管理する唯一のコマンドである。

> ```text
> vtest doc add --id DOC-BASIC-001 --path docs/basic-spec.md
>               [--title <t>]
>               [--derives-from DOC-REQ-001 [--anchor <text>] [--note <text>]]...
>               [--root | --no-root] [--update]
> vtest doc list [--tree] [--roots]
> vtest doc show DOC-BASIC-001
> ```

*引用: 本冊 §3.1, 基本仕様 §3.1・§3.2*

### SPEC-558

`doc` は文書種別（要件定義・基本仕様・詳細設計・API Schema 等）を区別しない。

### SPEC-559

段（要件→仕様→詳細設計…）は `derives_from` リンクで表現し、種別を増やさない。

### SPEC-560

旧モデルの `vtest spec` / `vtest req` は廃し、SPEC / REQ 実体層は持たない。

### SPEC-561

document の承認・却下・取消は `vtest approval` で行い、`doc` 側に承認操作を置かない。

### SPEC-562

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

### SPEC-563

エンティティ側の `vo approve` / `vo_approve` はこの経路への別名にすぎず、追加・相異する規則を持たない。

### SPEC-564

`audit bundle` / `submit` は本冊 §8 の判断記録プロトコルであり、意味検査ではない。

> ```text
> vtest audit bundle (--test TEST-X | --vo VO-X)
>                    [--kind test-semantic | impl-consistency | case-coverage]
>                    [--include-failed]
> vtest audit submit --file result.json
> ```

*引用: 本冊 §8*

### SPEC-565

本システムは宣言されていない義務・網羅漏れ・宣言と実装の意味のずれを自ら発見・裁定しない。

### SPEC-566

本システムは機械が決定論で確定できない疑義を `UNKNOWN` として外部（人間または判断可能 Agent）へ引き渡し、その判断を判断記録（`.verify/decisions/`）として追跡する。

*導出元: REQ-180, REQ-181, REQ-182, REQ-183, REQ-184, REQ-185, REQ-186, REQ-187, REQ-188, REQ-189, REQ-190, REQ-191, REQ-192*

*引用: 本冊 §8 冒頭, 基本仕様 §11, 要件定義 §12*

### SPEC-567

旧モデルの `spec-coverage`（SPEC 層依存）は復活させない。

### SPEC-568

判断記録は検査ゲートではなく、`UNKNOWN` に対する外部判断の追跡である。

### SPEC-569

判断記録（`.verify/decisions/` の actor / subject / decision / judgment_kind・理由 optional）と承認記録（`.verify/approvals/` の approver / subject または judgment_ref / approved_state、`vtest approval create` で生成）は別軸・別 entity である。

*引用: 本冊 §3.4, 本冊 §3.5*

### SPEC-570

判断済み ≠ 承認済みである。

*引用: 本冊 §8.5, 基本仕様 §17*

### SPEC-571

判断は承認なしでも記録でき、正式採用は承認の別段階である。

### SPEC-572

検査は基本仕様 §5 の固定4検査（`chain_integrity` / `orphan_detection` / `target_binding` / `oracle_presence`）のみである。

*引用: 基本仕様 §5*

### SPEC-573

旧モデルの12項目（`spec_coverage` / `vo_decomposition` / `vo_coverage` / `test_existence` / `static_audit` / `semantic_audit` / `impl_consistency` / `test_execution` / `runtime_result` / `target_execution` / `evidence_validity` / `test_traceability`）は検査として存在しない。

*引用: 本冊 §11.1*

### SPEC-574

`verify` が判定用、`report` が閲覧・提出用という役割分担とする。

### SPEC-S122 12.3 フェーズゲート評価（`verify --gate` / `report --gate`）

*導出元: REQ-S057*

### SPEC-575

プロジェクト側が登録したフェーズ・工程・ゲートの進行条件について、現在の検証状態（5状態）と承認（§3.5）が通過条件を満たすかを評価・提示できなければならない（MUST）。

*導出元: REQ-302, REQ-303, REQ-304, REQ-305, REQ-306, REQ-307, REQ-308*

*引用: 本冊 §11.5, 基本仕様 §20, 要件定義 §26.4*

### SPEC-576

本システムの責務はゲート条件が現在満たされているかの評価・提示に限り、フェーズのライフサイクル管理・工程の自動遷移は責務外とする（「Release フェーズへ遷移させる」ではなく「Release gate の条件を現在満たしている」を提示する）。

## SPEC-S123 13. MCP ツール詳細仕様

### SPEC-S124 13.3 エージェント向け利用フロー（参考）

### SPEC-577

完了確認は `verify` の4検査で行う。

## SPEC-S125 14. Form Schema 設計

### SPEC-S126 14.1 スキーマ形式（`.verify/forms/<kind>.yaml`）

### SPEC-578

`test_kind` の `regression` は Test の意図ラベル（`@vtest.kind` の値）であり、廃止された存在理由分類（role / anchor）とは別概念である。

*引用: 本冊 §4.1・§4.2*

## SPEC-S127 18. 受入契約

### SPEC-S128 18.1 共通条件

### SPEC-579

検証結果はfail-closedである。

### SPEC-S129 18.2 共通fixture

### SPEC-580

状態は5つのみとする。

*引用: 基本仕様 §4.1*

### SPEC-581

診断ラベルは検証状態と別軸の原因説明である。

*引用: 基本仕様 §4.2*

### SPEC-582

診断ラベルは状態値ではない。

*引用: 基本仕様 §4.2*

### SPEC-583

synthetic adapterは配布対象のproduction language adapterではない。

### SPEC-S130 18.3 機能別受入条件

#### SPEC-S131 18.3.1 discovery・record・graph と chain_integrity

### SPEC-584

adapter所有のmetadata宣言、ID、target、VO参照、record schema、Relationの違反を対応診断codeで検出する。

### SPEC-585

診断ラベルを二重定義しない。

### SPEC-586

document 種別を区別せず、要件定義・基本仕様・詳細設計・API Schema 等をすべて総称 document として同一に扱う。

*引用: 本冊 §3.1*

#### SPEC-S132 18.3.2 orphan_detection（文書層の孤児検出）

*導出元: REQ-S010*

### SPEC-587

`orphan_detection` は文書層のみを対象とし、親（上流 document）を持たない `document` ノードの有無を問う。

*導出元: REQ-059, REQ-060, REQ-061, REQ-062, REQ-063*

*引用: 本冊 §5.6, 基本仕様 §5.2, 要件定義 §4.2*

### SPEC-588

実装レイヤーの孤児検出（宣言されていない実装の検出）は行わない。

*導出元: R-2, REQ-292*

*引用: 要件定義 R-2, 基本仕様 §29 OOS-005*

### SPEC-589

旧モデルの W-SCAN-102（孤立 VO）は VO 層の警告であり、文書層 `orphan_detection` とは別物として存置する。

#### SPEC-S133 18.3.3 決定論的静的解析（oracle_presence・target_binding 静的到達）

### SPEC-590

DA-001〜DA-006とW-DA-101は本冊§7の判定条件に従う。

*引用: 本冊 §7*

### SPEC-591

確定違反だけをFAILとし、解析限界をUNKNOWNとして保持する。

### SPEC-592

信頼を宣言する専用の注釈・設定項目・レコードを新設せず、covers / 宣言targetのグラフだけで上記の各値が決まる。

#### SPEC-S134 18.3.4 execution・Evidence（target_binding の証拠）

### SPEC-593

旧モデルの`test_execution` / `runtime_result` / `target_execution`の3独立項目は撤去し、`target_binding`単一検査の証拠（Evidenceの存在・鮮度、`result`、`target_coverage`）へ吸収する。

*引用: 本冊 §11.1*

### SPEC-594

鮮度喪失の独立検査（旧`evidence_validity`）は設けず、鮮度は基本仕様§6のハッシュ束縛により満たし、喪失を診断ラベル`STALE`として説明する。

*引用: 基本仕様 §6*

#### SPEC-S135 18.3.5 target_binding 動的計測（per-target）

### SPEC-595

`target_coverage` は `target_binding` の動的計測結果であり独立の検査項目ではない。

### SPEC-596

旧モデルの`target_execution`検査項目は撤去し、計測事実だけをEvidenceの`target_coverage` fieldとして保持して`target_binding`の証拠源へ吸収する。

*引用: 本冊 §3.6・§10*

#### SPEC-S136 18.3.6 判断記録プロトコル（非ゲート）

*導出元: REQ-S035*

### SPEC-597

旧モデルの reasons / claim / basis 必須検査（E-AUDIT-005）、decomposition-viewpoint 検査（E-AUDIT-006）、spec / req basis 検査（E-AUDIT-007）は撤去し、判断記録層で課さない。

### SPEC-598

判断記録プロトコルは検証状態のゲートではなく、`UNKNOWN` に対する外部判断の追跡である。

*引用: 本冊 §8, 基本仕様 §11.3*

### SPEC-599

旧モデルの `verdict → CheckValue` 写像（`impl_consistency = MISMATCH` を含む検証状態への変換経路）は撤去する。

### SPEC-600

旧モデルの意味監査 bundle 種別（spec-coverage / test-semantic / vo-coverage / impl-consistency）を検査として扱わず、網羅・意味の疑義は `UNKNOWN` として本プロトコルへエスカレーションする。

*導出元: REQ-180, REQ-181, REQ-182, REQ-183, REQ-184, REQ-185, REQ-186, REQ-187, REQ-188, REQ-189, REQ-190, REQ-191, REQ-192*

*引用: 本冊 §7.1・§8, 基本仕様 §5・§11, 要件定義 §12*

### SPEC-601

`spec_coverage` / `vo_decomposition` / `vo_coverage` / `impl_consistency` は検証項目として存在しない。

### SPEC-602

`case-coverage` は §11 の判断対象であって基本仕様 §5 の 4 検査ではない。

*引用: 基本仕様 §5*

### SPEC-603

§5 の 4 検査を再実施した結果は `PASS` / `FAIL` / `MISMATCH` / `NO_EVIDENCE` / `UNKNOWN` のいずれにもなり得る。

### SPEC-604

変更そのものが `UNKNOWN` を生成するのではない。

#### SPEC-S137 18.3.7 承認と判断記録の分離

### SPEC-605

判断済みと承認済みを区別する（判断済み ≠ 承認済み）。

### SPEC-606

判断記録と承認記録は同一 entity であることを要求せず、別 entity でありうる。

*引用: 本冊 §3.4・§3.5, 基本仕様 §11.3・§17*

### SPEC-607

判断は承認なしでも記録でき、正式採用は承認の別段階である。

### SPEC-608

承認は検証状態と独立の別軸である。

### SPEC-609

方針は総称 document として登録した文書で表現し、専用のエンティティ型を設けない。

### SPEC-610

承認権限（approval authority）・承認ロール・必要承認数・権限 schema はプロジェクト設定と別紙A へ委譲する。

*引用: 基本仕様 §17・§30*

### SPEC-611

承認 workflow の状態遷移と `approved_state` の値域は本冊 §3.5 に定める。

*引用: 本冊 §3.5*

#### SPEC-S138 18.3.8 verify・report と scope

### SPEC-612

機能単位の集約は親 VO（子 VO を持つ VO）を単位とし、Feature を別エンティティ・別レコード・別 ID として設けない。

### SPEC-613

旧モデルの SPEC → REQ → VO → Test 構造は総称 document 化により DOC → VO → Test へ再導出する。

### SPEC-614

「どの上流条項がどの VO へ対応するか」の対応ペアの取得に新規 CLI コマンド・MCP ツールを用いない（既存の `report` projection と `test query` 逆引きだけで取得できる）。

#### SPEC-S139 18.3.9 フェーズゲート評価

*導出元: REQ-S057*

### SPEC-615

プロジェクト側が登録したフェーズ・工程・ゲートの進行条件について、現在の検証状態（§4.1 の 5 状態）と承認（§18.3.7）が通過条件を満たすかを評価・提示できなければならない（MUST）。

*導出元: REQ-302, REQ-303, REQ-304, REQ-305, REQ-306, REQ-307, REQ-308*

*引用: 本冊 §11.5, 基本仕様 §20, 要件定義 §26.4*

### SPEC-616

検証状態と承認は独立の軸であり、ゲートは両者の組合せを進行条件にできる。

### SPEC-617

責務はゲート条件が現在満たされているかの評価・提示に限る。

### SPEC-618

フェーズのライフサイクル管理・工程の自動遷移は責務外とする。

*導出元: REQ-302, REQ-303, REQ-304, REQ-305, REQ-306, REQ-307, REQ-308*

*引用: 基本仕様 §20・§29 OOS-004, 要件定義 §26.4*

### SPEC-619

新規 CLI コマンド・MCP ツールを増やさず、既存の `vtest verify` の `--gate` 引数と出力、および `report` の JSON でゲート評価を露出する。

### SPEC-620

具体的なフェーズ名・承認ロール・必要承認数はプロジェクト設定と別紙A へ委譲する。

*引用: 基本仕様 §30*

### SPEC-S140 18.4 提供範囲外

*導出元: R-2, R-3*

### SPEC-621

GUI は提供範囲外である。

### SPEC-622

仕様書同士の矛盾判定は提供範囲外である。

### SPEC-623

仕様・Test・実装のどれを変更すべきかという修正方針の決定は提供範囲外である。

### SPEC-624

helper、fixture、通常sourceの編集管理は提供範囲外である。

### SPEC-625

開発process管理は提供範囲外である。

### SPEC-626

`rust-cargo`以外のproduction language adapterは提供範囲外である。

### SPEC-627

third-party plugin ABIは提供範囲外である。

### SPEC-628

LSP統合は提供範囲外である。

### SPEC-629

runner / coverage providerの自動選択または推測fallbackは提供範囲外である。
