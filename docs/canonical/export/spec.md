<!-- generated from docs/canonical/specification.json by build.py; do not edit -->

# 基本仕様

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

ツール名は `vtest` とする（バイナリ名・ディレクトリ名に使用する）。

*導出元: REQ-309, REQ-310, REQ-311, REQ-312, REQ-313, REQ-314, REQ-315, REQ-316, REQ-317, REQ-318, REQ-319, REQ-320, REQ-321, REQ-322, REQ-323, REQ-324, REQ-325, REQ-326, REQ-327, REQ-328, REQ-329, REQ-330, REQ-331, REQ-332*

*引用: 要件定義 §28*

### SPEC-011

`vtest` 本体の実装言語はRustとする。

### SPEC-012

組込 production adapter は `rust-cargo` とする。

### SPEC-013

`rust-cargo` はRustの関数単体テストおよび小規模な結合テスト（`#[test]` 属性を持つテスト関数）を対象とする。

### SPEC-014

`rust-cargo` 以外の production language adapter はv0.1の提供範囲に含めない。

*導出元: REQ-258, REQ-259, REQ-260, REQ-261, REQ-262, REQ-263, REQ-264, REQ-265, REQ-266*

*引用: 要件定義 §21*

### SPEC-015

インターフェースはCLIと、AI Agent向けMCPサーバとする。

*導出元: REQ-267, REQ-268, REQ-269, REQ-270*

*引用: 要件定義 §22*

### SPEC-016

MCPを本体とする。

### SPEC-017

`vtest` 自身はLLM APIを呼ばない。

### SPEC-018

`vtest` 自身は宣言と実装の意味的な良し悪しを裁定しない。

### SPEC-019

機械が決定論で確定できない疑義は `UNKNOWN` として外部の判断者へ引き渡す（§11）。

*導出元: REQ-180, REQ-181, REQ-182, REQ-183, REQ-184, REQ-185, REQ-186, REQ-187, REQ-188, REQ-189, REQ-190, REQ-191, REQ-192*

*引用: 要件定義 §12*

### SPEC-020

Rust固有処理は組込 `rust-cargo` adapter が所有する。

### SPEC-021

CLI・MCP・検証coreはadapter registryを介して能力を選択する。

### SPEC-022

coreの検証契約は言語・test runnerに依存しない。

*導出元: REQ-258, REQ-259, REQ-260, REQ-261, REQ-262, REQ-263, REQ-264, REQ-265, REQ-266*

*引用: 要件定義 §21*

### SPEC-023

documentとは、ソースコードより上流に位置する成果物を表す単一の総称ノードである。

### SPEC-024

documentは要件定義書・基本仕様書・詳細設計書・API Schema・Protocol Specification・型/データ仕様・DB schema・その他の機械可読仕様を含む。

### SPEC-025

documentは `id + path + content_hash + 上流参照（derives_from）` を持つ。

### SPEC-026

文書種別ごとの専用スキーマは設けない。

*導出元: REQ-024, REQ-025, REQ-026, REQ-027, REQ-028, REQ-029, REQ-030, REQ-031, REQ-032, REQ-033*

*引用: 要件定義 §3.2*

### SPEC-027

対象ソースコード自身のdoc commentは、その対象実装の唯一の仕様根拠としては用いない。

*導出元: REQ-233, REQ-234, REQ-235*

*引用: 要件定義 §18*

### SPEC-028

derives_fromとは、document間の唯一のリンク種別である。

### SPEC-029

derives_fromは上流documentから下流documentへの導出を表す。

### SPEC-030

各derives_fromリンクは任意（optional）の説明文・導出理由を保持できる（§3.2）。

*導出元: REQ-036, REQ-037, REQ-038, REQ-039, REQ-040, REQ-041, REQ-042, REQ-043, REQ-044, REQ-045, REQ-046, REQ-047, REQ-048, REQ-049*

*引用: 要件定義 §3.4*

### SPEC-031

Verification Obligation（VO）とは、独立して「この条件が成立するか」と検証可能な仕様上の命題である。

*導出元: REQ-162, REQ-163*

*引用: 要件定義 §10.1*

### SPEC-032

VOは1件以上のdocumentからderives_fromで導出される。

### SPEC-033

VOとdocumentの間に他のエンティティ層を置かない。

### SPEC-034

VOは階層構造を持てる。

### SPEC-035

VOの粒度をassert文・test function・テストファイルなどのコード構文で決めない。

### SPEC-036

Testとは、登録adapterが実行可能な検証単位として識別し、Test IDで管理するtest constructである。

### SPEC-037

TestはVOの検証実装単位であり、VOとN:Mの対応を持ちうる。

### SPEC-038

Testは `covers` 宣言でVOを参照する。

### SPEC-039

Test Intentとは、Testが「何を検証するか」を実装コードを読まずに判断できる形で表した付随情報である。

### SPEC-040

Test Intentは宣言鎖のノードではない。

*導出元: REQ-203, REQ-204, REQ-205*

*引用: 要件定義 §14*

### SPEC-041

検証対象とは、その Test が検証成立性（§8）を証明しようとする対象、すなわち宣言された「何の時にどうなる」の主語である。

### SPEC-042

検証対象は実装constructに限定せず、外部から観測可能な契約・境界上の振る舞いも含む。

*導出元: REQ-144, REQ-145, REQ-146, REQ-147, REQ-148*

*引用: 要件定義 §9.1*

### SPEC-043

Source Target（SRC）とは、実装コード上の識別可能なimplementation constructである。

### SPEC-044

Source Targetは、adapter IDとadapter所有のopaque locatorからなるTarget Reference、または任意の恒久SRC IDで識別する。

*導出元: REQ-149, REQ-150, REQ-151, REQ-152, REQ-153, REQ-154, REQ-155, REQ-156*

*引用: 要件定義 §9.2*

### SPEC-045

Execution Evidenceとは、テスト実行の事実の記録である。

### SPEC-046

Execution Evidenceは結果・実行時リポジトリ状態・解決後のcanonical Source Target参照・各内容ハッシュ・実行計測結果を含む。

### SPEC-047

Execution Evidenceは検証対象の内容ハッシュに束縛される（§6）。

*導出元: REQ-115, REQ-116, REQ-117, REQ-118, REQ-119, REQ-120*

*引用: 要件定義 §6*

### SPEC-048

判断記録（judgment record）とは、`UNKNOWN` に対して外部（人間または判断可能Agent）が下した判断の記録である。

### SPEC-049

判断記録はactor / subject / decisionを必須項目とする。

### SPEC-050

判断記録の理由・根拠は任意とする。

### SPEC-051

判断記録は依存closureのハッシュに束縛される（§11）。

*導出元: REQ-180, REQ-181, REQ-182, REQ-183, REQ-184, REQ-185, REQ-186, REQ-187, REQ-188, REQ-189, REQ-190, REQ-191, REQ-192*

*引用: 要件定義 §12*

### SPEC-052

承認記録（approval record）とは、判断または方針を「この内容で進める」と正式に認めた記録である。

### SPEC-053

承認記録はapprover / subject（またはjudgment reference）/ approved stateを必須とする。

### SPEC-054

承認記録は上流依存closureのハッシュに束縛される。

### SPEC-055

承認記録は判断記録とは別軸・別entityでありうる（§17）。

*導出元: REQ-236, REQ-237, REQ-238, REQ-239, REQ-240, REQ-241, REQ-242, REQ-243, REQ-244, REQ-245, REQ-246, REQ-247, REQ-248, REQ-249, REQ-250, REQ-251, REQ-252*

*引用: 要件定義 §19*

### SPEC-056

検証状態は `PASS` / `FAIL` / `MISMATCH` / `NO_EVIDENCE` / `UNKNOWN` の5つとする（§4.1）。

*導出元: REQ-085, REQ-086, REQ-087, REQ-088, REQ-089, REQ-090, REQ-091*

*引用: 要件定義 §5.1*

### SPEC-057

検証状態は検証結果のみを表す。

### SPEC-058

検証状態に承認状態を混入させない。

*導出元: REQ-109, REQ-110, REQ-111, REQ-112, REQ-113, REQ-114*

*引用: 要件定義 §5.5*

### SPEC-059

診断ラベルとは、検証状態に付随して原因を説明するラベル（`MISSING` / `NOT_EXECUTED` / `NOT_CHECKED` / `STALE` 等）である。

### SPEC-060

診断ラベルは検証状態ではない。

### SPEC-061

診断ラベルの語彙は詳細設計で定める。

*導出元: REQ-092, REQ-093, REQ-309, REQ-310, REQ-311, REQ-312, REQ-313, REQ-314, REQ-315, REQ-316, REQ-317, REQ-318, REQ-319, REQ-320, REQ-321, REQ-322, REQ-323, REQ-324, REQ-325, REQ-326, REQ-327, REQ-328, REQ-329, REQ-330, REQ-331, REQ-332*

*引用: 要件定義 §5.2、§28*

### SPEC-062

検査は `chain_integrity` / `orphan_detection` / `target_binding` / `oracle_presence` の4本のみとする（§5）。

*導出元: REQ-034, REQ-035, REQ-050, REQ-051, REQ-052, REQ-053, REQ-054, REQ-055, REQ-056, REQ-057, REQ-058, REQ-059, REQ-060, REQ-061, REQ-062, REQ-063, REQ-064, REQ-065, REQ-066, REQ-067, REQ-068, REQ-069, REQ-070, REQ-071, REQ-072, REQ-073, REQ-074, REQ-075, REQ-076, REQ-077, REQ-078, REQ-079, REQ-080, REQ-081, REQ-082, REQ-083, REQ-084*

*引用: 要件定義 §3.3、§4*

### SPEC-063

完全検証とは、宣言鎖全体に対する検査（`chain_integrity` / `orphan_detection`）と、scope内の各「宣言＋コード＋証拠」の組に対する検査（`target_binding` / `oracle_presence`）をすべて対象とする検証である。

### SPEC-064

完全検証は一項目でも非 `PASS` があればNGとする（fail-closed）。

*導出元: REQ-295, REQ-296*

*引用: 要件定義 §26.1*

### SPEC-065

scopeとは、利用者が限定する検査・エンティティの範囲である。

### SPEC-066

scopeを狭めても対象外項目を `PASS` へ書き換えない。

*導出元: P-002*

*引用: 要件定義 §2/P-002*

### SPEC-067

正典（source of truth）とは、ある事実を決定する唯一の記録である。

### SPEC-068

正典から導出できる情報は派生情報とし独立保存しない。

*導出元: P-003*

*引用: 要件定義 P-003*

### SPEC-069

Agent Form Engineeringとは、既知の作業手順・入力項目を持つ操作を、自由編集ではなく構造化された質問・入力・検証で行わせる方式である。

*導出元: P-004*

*引用: 要件定義 P-004*

### SPEC-070

本システムは扱う情報を三層に分ける。

### SPEC-071

宣言層は、adapter所有のTest metadata宣言、および.verify/配下のdocument / VO / Relationレコードからなり、Gitで管理される正典である。

### SPEC-072

実装層は、テストコード本体と対象ソースコードからなり、Gitで管理される正典である。

### SPEC-073

事実層は、実行結果・判断記録・承認記録からなる.verify/配下の追記型レコードファイルであり、Gitで管理される。

### SPEC-074

派生情報（検索インデックス、検証グラフ、集約結果）は上記から毎回再構築する。

### SPEC-075

派生情報はGit管理しない。

*導出元: P-003*

*引用: 要件定義 P-003 / NFR-004*

### SPEC-076

source discovery、決定論的解析、Structured Test Operation、test runner起動、coverage計測はadapter capabilityとして提供する。

### SPEC-077

adapterが返す導出結果はregistryでmergeし、adapter ID・path・Test IDの順に正規化する。

### SPEC-078

registryの重複ID、未登録adapter、adapter間のTest ID重複は操作エラーとする。

### SPEC-079

registryの重複ID、未登録adapter、adapter間のTest ID重複は空のscanとして成功扱いしない。

*導出元: REQ-258, REQ-259, REQ-260, REQ-261, REQ-262, REQ-263, REQ-264, REQ-265, REQ-266*

*引用: 要件定義 §21*

### SPEC-080

本システムの仕事は「宣言と実装が一致しているか」「事実が現在の宣言・実装に対して有効か」を照合することに限る。

### SPEC-081

どれかを正として他を修正させることはしない。

*導出元: P-001*

*引用: 要件定義 P-001*

### SPEC-082

要件定義 §3.2 の宣言鎖をそのまま採用する。

*導出元: REQ-024, REQ-025, REQ-026, REQ-027, REQ-028, REQ-029, REQ-030, REQ-031, REQ-032, REQ-033*

*引用: 要件定義 §3.2*

### SPEC-083

上流文書はすべて単一の総称ノード型 `document` で表現する。

### SPEC-084

文書間リンクは `derives_from` の一種のみとする。

### SPEC-085

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

### SPEC-086

文書層の段数は総称的に扱い、リンクを追加してもスキーマが壊れないことを設計制約とする。

### SPEC-087

段はリンクであって検査ではない。

*導出元: REQ-024, REQ-025, REQ-026, REQ-027, REQ-028, REQ-029, REQ-030, REQ-031, REQ-032, REQ-033, REQ-034, REQ-035*

*引用: 要件定義 §3.2/§3.3*

### SPEC-088

VOは1件以上の `document` からderives_fromで導出される。

### SPEC-089

VOとdocumentの間に他のエンティティ層を置かない。

### SPEC-090

本システムは文書内容の意味的な良し悪しに関知しない。

### SPEC-091

文書種別ごとの専用スキーマ・文書間リンク意味論の増殖・文書内容の良否検証を行わない。

*導出元: REQ-024, REQ-025, REQ-026, REQ-027, REQ-028, REQ-029, REQ-030, REQ-031, REQ-032, REQ-033, REQ-288, REQ-289, REQ-290, REQ-291, REQ-292, REQ-293, REQ-294*

*引用: 要件定義 §3.2、§25 OOS-001*

### SPEC-092

不一致はどちらが正かを決めない。

### SPEC-093

不一致は状態（§4）として提示する。

*導出元: P-001*

*引用: 要件定義 P-001*

### SPEC-094

Test→VO（`covers`）、Test→SRC（`targets`）の関係はadapter所有のTest metadata宣言から決定論的に導出できる。

### SPEC-095

Test→VO、Test→SRCの関係を外部ファイルへ重複保存しない。

### SPEC-096

graphと現在のtarget集合は常にadapter所有のTest metadata宣言から再構築する。

### SPEC-097

graphと現在のtarget集合はEvidenceのtarget参照から関係を生成・修復しない。

### SPEC-098

Evidenceに含むtarget参照は、target別の実行事実と内容ハッシュを束縛する実行時snapshot keyである。

### SPEC-099

Evidenceに含むtarget参照はTest→SRC関係の正典ではない。

### SPEC-100

外部レコードとして保存するのは、どちらか一方のエンティティに自然に所属しない関係（VO間の依存、Test間の補完関係など）だけとする。

*導出元: P-003*

*引用: 要件定義 P-003*

### SPEC-101

`config.yaml` writerの正規形はversion 2とする。

### SPEC-102

`config.yaml` writerはadapterごとにroot・scan・run設定をnamespace化する。

### SPEC-103

readerはversion 1を単一の `rust-cargo` adapter設定としてin-memory変換して読み取る。

### SPEC-104

readerは読み取りだけで正典を書き換えない。

### SPEC-105

`vtest init` はversion 2を生成する。

### SPEC-106

adapter IDは設定内で一意でなければならない。

### SPEC-107

同一adapter内のroot重複も拒否する。

### SPEC-108

異なるadapterが同じrootを走査することは許可する。

> polyglot repository を扱えるようにするための許可。

### SPEC-109

未知のadapterやadapter固有設定の検証失敗は操作エラーとする。

### SPEC-110

未知のadapterやadapter固有設定の検証失敗時、利用可能な言語や能力を推測補完しない。

*導出元: REQ-258, REQ-259, REQ-260, REQ-261, REQ-262, REQ-263, REQ-264, REQ-265, REQ-266*

*引用: 要件定義 §21*

### SPEC-111

core domainの `TestEntity` は、言語・runner非依存の `execution`（adapter・project・suite・opaque selector）だけを実行座標として持つ。

### SPEC-112

`filter` / `package` / `test_target` は `TestEntity` のfieldではない。

### SPEC-113

Test JSONのwire compatibility layerは `execution` を常に出力する。

### SPEC-114

Test JSONのwire compatibility layerは `rust-cargo` Testについてだけversion 1互換fieldを追加出力できる。

### SPEC-115

非Rust Testでは version 1互換fieldを省略する。

### SPEC-116

非Rust Testでは空値・dummy値・Rust既定値を生成しない。

### SPEC-117

`targets` listを常に出力する。

### SPEC-118

単数互換field `target` はtarget 1件のときだけ追加出力する。

### SPEC-119

欠落・矛盾時は入力を拒否する。

### SPEC-120

欠落・矛盾時は推測で実行可能として扱わない。

### SPEC-121

documentのIDは `DOC-` とし、正典は `.verify/doc/` に置く。

### SPEC-122

documentは総称の上流文書ノード（path＋content_hash＋derives_from）である。

### SPEC-123

documentは種別専用スキーマを持たない。

### SPEC-124

Verification ObligationのIDは `VO-` とし、正典は `.verify/vo/` に置く。

### SPEC-125

Verification Obligationは検証命題である。

### SPEC-126

Verification Obligationは階層を持てる。

### SPEC-127

TestのIDは `TEST-` とし、正典はadapter所有のTest metadata宣言とする。

### SPEC-128

Testはadapterが識別する実行可能なtest constructである。

### SPEC-129

Source TargetはIDを持たない、または任意で `SRC-` を用い、adapter IDとopaque locatorで識別する。

### SPEC-130

Source Targetは対象implementation constructである。

### SPEC-131

Source Targetの恒久IDは必須としない。

### SPEC-132

RelationのIDは `REL-`（ULID）とし、正典は `.verify/rel/` に置く。

### SPEC-133

Relationは外部関係レコードであり、不変とする。

### SPEC-134

derives_fromの説明文もRelationに保持できる。

### SPEC-135

判断記録のIDはULIDとし、正典は `.verify/decisions/` に置く。

### SPEC-136

判断記録は `UNKNOWN` への外部判断であり、追記型とする。

### SPEC-137

承認記録のIDはULIDとし、正典は `.verify/approvals/` に置く。

### SPEC-138

承認記録は判断・方針の正式採用であり、追記型とする。

### SPEC-139

Execution EvidenceのIDはULIDとし、正典は `.verify/evidence/` に置く。

### SPEC-140

Execution Evidenceは実行証拠レコードであり、追記型とする。

### SPEC-141

documentは単一の総称ノードであり、要件定義・基本仕様・詳細設計・API Schema等を種別で区別する専用スキーマを持たない。

### SPEC-142

文書層の段（要件→仕様→詳細設計…）はderives_fromリンクとして表現する。

### SPEC-143

段を増やしても種別を増やさない。

*導出元: REQ-024, REQ-025, REQ-026, REQ-027, REQ-028, REQ-029, REQ-030, REQ-031, REQ-032, REQ-033*

*引用: 要件定義 §3.2*

### SPEC-144

DOC / VO / TESTのIDは人間可読な形式とする。

### SPEC-145

DOC / VO / TESTのIDは利用者（人間またはAI）が命名する。

### SPEC-146

IDの文字集合は `[A-Z0-9-]` とする。

### SPEC-147

IDの接頭辞は種別ごとに固定する（`TEST-` 等）。

### SPEC-148

IDの推奨形式は `TEST-<領域>-<連番>`（例：`TEST-PARSER-044`）とする。

### SPEC-149

ツールはID形式を強制せず一意性のみを強制する。

### SPEC-150

IDの一意性はスキャン時に全数検査する。

### SPEC-151

ID衝突は `chain_integrity` の非 `PASS`（`MISMATCH`）とする（§5.1、§23）。

### SPEC-152

任意の恒久SRC IDはadapter namespaceを持たないためrepository全体で一意とする。

### SPEC-153

恒久SRC IDの衝突は曖昧参照として受理しない。

### SPEC-154

恒久SRC IDの衝突時、どのSource Targetを指すか推測しない。

*導出元: REQ-149, REQ-150, REQ-151, REQ-152, REQ-153, REQ-154, REQ-155, REQ-156*

*引用: 要件定義 §9.2*

### SPEC-155

関係リンクは説明文・導出理由を任意（optional）で保持できる。

### SPEC-156

derives_from・covers・検証対象・実装traceabilityなど性質の異なる関係型は潰さず区別する。

### SPEC-157

存在するリンクに付す説明文は空でもよい。

### SPEC-158

説明文が空であることを理由に `chain_integrity` 違反・`MISMATCH` としてはならない。

*導出元: REQ-036, REQ-037, REQ-038, REQ-039, REQ-040, REQ-041, REQ-042, REQ-043, REQ-044, REQ-045, REQ-046, REQ-047, REQ-048, REQ-049*

*引用: 要件定義 §3.4*

### SPEC-159

関係型そのものの意味論的増殖は求めない。

### SPEC-160

Relation writerは `REL-<ULID>` を正規IDとしてファイル名に用いる。

### SPEC-161

readerはversion 1互換入力としてbare ULIDを `REL-<ULID>` へin-memory正規化する。

### SPEC-162

判断・承認・EvidenceのIDはbare ULIDとする。

### SPEC-163

ULID payloadにより並列生成時のファイル名衝突を実用上排除する。

### SPEC-164

関係リンクの任意説明文・役割別projectionの保存形式・presetは詳細設計へ委譲する。

*導出元: REQ-309, REQ-310, REQ-311, REQ-312, REQ-313, REQ-314, REQ-315, REQ-316, REQ-317, REQ-318, REQ-319, REQ-320, REQ-321, REQ-322, REQ-323, REQ-324, REQ-325, REQ-326, REQ-327, REQ-328, REQ-329, REQ-330, REQ-331, REQ-332*

*引用: 要件定義 §28*

### SPEC-165

ソースコードへ恒久IDを埋め込むことは必須としない。

*導出元: REQ-149, REQ-150, REQ-151, REQ-152, REQ-153, REQ-154, REQ-155, REQ-156*

*引用: 要件定義 §9.2*

### SPEC-166

対象はTarget Referenceで識別する。

### SPEC-167

Target Referenceは、adapter IDとadapter所有のopaque locatorの組、または任意のSRC ID参照である。

> <adapter-id>::<opaque-locator>
> 例：rust-cargo::src/parser.rs::Parser::parse

### SPEC-168

opaque locatorの構文と恒久SRC IDの宣言方法はadapterが定める。

### SPEC-169

共通契約がpath・module・function等の特定言語構造を必須としてはならない。

*導出元: R-3, REQ-149, REQ-150, REQ-151, REQ-152, REQ-153, REQ-154, REQ-155, REQ-156*

*引用: 要件定義 §9.2、R-3*

### SPEC-170

1つのTestは1件以上のSource Targetを持ち、各target参照を個別に保持する。

### SPEC-171

Source Targetは代表1件へ縮約しない。

### SPEC-172

Test→SRCの対応はadapter所有のTest metadata宣言から提供する。

### SPEC-173

SRC→Testの逆引きはスキャン結果から提供する。

### SPEC-174

検証状態は `PASS` / `FAIL` / `MISMATCH` / `NO_EVIDENCE` / `UNKNOWN` の5つのみとする。

*導出元: REQ-085, REQ-086, REQ-087, REQ-088, REQ-089, REQ-090, REQ-091*

*引用: 要件定義 §5.1*

### SPEC-175

状態の存在資格は「受け取った者の行動が変わるか」である。

### SPEC-176

意味の違いは資格にならない。

### SPEC-177

`PASS` を受け取った者はマージできる。

### SPEC-178

完全検証において `PASS` はOKとする。

### SPEC-179

`FAIL` を受け取った者は実装（テスト実装を含む）を直す。

### SPEC-180

完全検証において `FAIL` はOKとしない。

### SPEC-181

`MISMATCH` を受け取った者はコードを触る前に宣言側（上流）を直す。

### SPEC-182

完全検証において `MISMATCH` はOKとしない。

### SPEC-183

`NO_EVIDENCE` を受け取った者は証拠を作る（機械的に解決可能）。

### SPEC-184

完全検証において `NO_EVIDENCE` はOKとしない。

### SPEC-185

`UNKNOWN` は決定論の限界であり、受け取った者は意味判定できる者へエスカレーションする。

### SPEC-186

完全検証において `UNKNOWN` はOKとしない。

### SPEC-187

`MISSING` / `NOT_EXECUTED` / `NOT_CHECKED` / `STALE` 等は、状態に付随して原因を説明する診断ラベルである。

### SPEC-188

診断ラベルは検証状態ではない。

### SPEC-189

診断ラベルの語彙は詳細設計で定める。

*導出元: REQ-092, REQ-093, REQ-309, REQ-310, REQ-311, REQ-312, REQ-313, REQ-314, REQ-315, REQ-316, REQ-317, REQ-318, REQ-319, REQ-320, REQ-321, REQ-322, REQ-323, REQ-324, REQ-325, REQ-326, REQ-327, REQ-328, REQ-329, REQ-330, REQ-331, REQ-332*

*引用: 要件定義 §5.2、§28*

### SPEC-190

本書は状態と診断ラベルを常に別軸として扱い、混同しない。

### SPEC-191

要件定義 §5.3 の割当をそのまま採用する。

*導出元: REQ-094, REQ-095, REQ-096, REQ-097, REQ-098, REQ-099, REQ-100, REQ-101, REQ-102, REQ-103, REQ-104*

*引用: 要件定義 §5.3*

### SPEC-192

発見されたTestに管理宣言が無い場合、状態は `MISMATCH`、診断ラベルはMISSINGとする。

### SPEC-193

`covers` のVO参照を解決できない、同一constructから複数entity、またはTest ID衝突の場合、状態は `MISMATCH` とする。

### SPEC-194

文書鎖のリンク切れ、content_hash不一致、または孤児文書の場合、状態は `MISMATCH`、診断ラベルはSTALE等とする。

### SPEC-195

証拠が存在しない、または証拠のハッシュが現在の対象と不一致の場合、状態は `NO_EVIDENCE`、診断ラベルはSTALE等とする。

### SPEC-196

scope限定により検査を実施しなかった項目（完全検証の集約時）は、状態を `NO_EVIDENCE`、診断ラベルをNOT_CHECKEDとする。

### SPEC-197

discoveryが不完全、または解析不能の場合、状態は `UNKNOWN` とする。

### SPEC-198

テストランナーが失敗を報告した場合、状態は `FAIL` とする。

### SPEC-199

宣言された検証対象の実行が0回の場合、状態は `FAIL`、診断ラベルはNOT_EXECUTEDとする。

### SPEC-200

`UNKNOWN` はエラーではなく正常動作としての降参である。

### SPEC-201

内部エラー・入力不正は検証状態と別系統（終了コード。§27）で表現する。

### SPEC-202

`UNKNOWN` をエラー処理のフォールバック先として使う実装は仕様違反とする。

*導出元: REQ-105, REQ-106, REQ-107, REQ-108*

*引用: 要件定義 §5.4*

### SPEC-203

検証状態（§4.1の5状態）は検証結果のみを表す。

### SPEC-204

検証状態は承認状態を混入させない。

### SPEC-205

承認（§17）は独立した別軸である。

*導出元: REQ-109, REQ-110, REQ-111, REQ-112, REQ-113, REQ-114*

*引用: 要件定義 §5.5*

### SPEC-206

技術的に `PASS` であっても未承認である状態を許容する。

### SPEC-207

未承認であることだけを理由に `PASS` を `UNKNOWN` 等へ変更してはならない。

### SPEC-208

承認済みであることを理由に `FAIL` / `MISMATCH` / `NO_EVIDENCE` / `UNKNOWN` を `PASS` へ変更してはならない。

### SPEC-209

フェーズ進行に承認を要するかは、検証状態と承認の組合せとして §21 のゲート条件で扱う。

### SPEC-210

検査軸は実施する検査（4本の部分集合）を指定する。

*導出元: P-002*

*引用: 要件定義 P-002*

### SPEC-211

エンティティ軸は対象とするdocument / VO / Testの部分木を指定する。

### SPEC-212

scopeを限定した検証のOKは「要求されたscope内がOK」の意味に限られる。

### SPEC-213

scope外・未実施の項目は `NO_EVIDENCE`（診断NOT_CHECKED）として保持する。

### SPEC-214

scope外・未実施の項目は `PASS` へ変換しない。

### SPEC-215

出力には要求scopeと、scope外項目が未検証である旨を必ず併記する。

### SPEC-216

いかなる設定値も完全検証の検査を4本未満へ縮退させない。

### SPEC-217

検証は `chain_integrity` / `orphan_detection` / `target_binding` / `oracle_presence` の4検査のみで行う。

### SPEC-218

鎖に段（リンク）が増えても検査は増えない。

*導出元: REQ-034, REQ-035, REQ-050, REQ-051, REQ-052, REQ-053, REQ-054, REQ-055, REQ-056, REQ-057, REQ-058, REQ-059, REQ-060, REQ-061, REQ-062, REQ-063, REQ-064, REQ-065, REQ-066, REQ-067, REQ-068, REQ-069, REQ-070, REQ-071, REQ-072, REQ-073, REQ-074, REQ-075, REQ-076, REQ-077, REQ-078, REQ-079, REQ-080, REQ-081, REQ-082, REQ-083, REQ-084*

*引用: 要件定義 §3.3、§4*

### SPEC-219

各検査は一つの問いを持つ。

### SPEC-220

各検査は複数の証拠源で答えてよい。

### SPEC-221

答えは検証方法・実行形態に依らず同一でなければならない。

*導出元: REQ-050, REQ-051, REQ-052, REQ-053, REQ-054, REQ-055, REQ-056, REQ-057, REQ-058, REQ-059, REQ-060, REQ-061, REQ-062, REQ-063, REQ-064, REQ-065, REQ-066, REQ-067, REQ-068, REQ-069, REQ-070, REQ-071, REQ-072, REQ-073, REQ-074, REQ-075, REQ-076, REQ-077, REQ-078, REQ-079, REQ-080, REQ-081, REQ-082, REQ-083, REQ-084*

*引用: 要件定義 §4 冒頭、§8 条項 3*

### SPEC-222

凍結要件が検査から明示的に排除した判断（仕様網羅・VO網羅・VO分解妥当性・意味一致・実装一致）は、本書でも検査に含めない。

### SPEC-223

網羅・意味の疑義はエスカレーション（§11）の領分である。

*導出元: REQ-164, REQ-165, REQ-166, REQ-174, REQ-175, REQ-176, REQ-177, REQ-178, REQ-179, REQ-180, REQ-181, REQ-182, REQ-183, REQ-184, REQ-185, REQ-186, REQ-187, REQ-188, REQ-189, REQ-190, REQ-191, REQ-192*

*引用: 要件定義 §10.2、§11、§12*

### SPEC-224

chain_integrityの問いは、宣言鎖のすべてのリンクが存在し、ハッシュ照合が成立するかである。

### SPEC-225

文書層では、各documentのderives_from参照先が存在する。

### SPEC-226

文書層では、各documentのcontent_hashが現物と一致する。

### SPEC-227

VO層では、各VOが1件以上の `document` への解決可能なderives_fromを持つ。

### SPEC-228

Test層では、発見された各Testに対応する管理宣言（構文上有効なTest ID・1件以上の `covers`・その他の必須metadata）がちょうど1件存在する。

### SPEC-229

Test層では、`covers` の全VO参照を解決できる。

### SPEC-230

Test層では、Test IDが発見結果全体で一意である。

### SPEC-231

leaf VO→Test（検証実装の存在）と、発見されたTest→宣言（管理宣言の解決）の両方向が成立して初めて双方向完全性が成立する。

### SPEC-232

どのリンクで切れたかは診断ラベルで示す。

### SPEC-233

違反時の状態は§4.3に従う（管理宣言欠落は `MISMATCH`/MISSING、参照解決不能・ID衝突は `MISMATCH`、リンク切れ・hash不一致は `MISMATCH`）。

### SPEC-234

すべてのTestを管理対象とすることと、当該Testを仕様適合の証拠として算入すること（§8）は別個の条件とする。

### SPEC-235

orphan_detectionの問いは、親を持たない `document` ノードが存在するかである。

### SPEC-236

根として指定された文書は対象外とする。

### SPEC-237

根の指定方式は `.verify/` 設定における明示的な根指定として保持する。

### SPEC-238

根の指定の具体構文は詳細設計へ委譲する。

*導出元: REQ-309, REQ-310, REQ-311, REQ-312, REQ-313, REQ-314, REQ-315, REQ-316, REQ-317, REQ-318, REQ-319, REQ-320, REQ-321, REQ-322, REQ-323, REQ-324, REQ-325, REQ-326, REQ-327, REQ-328, REQ-329, REQ-330, REQ-331, REQ-332*

*引用: 要件定義 §28*

### SPEC-239

対象は文書層のみとする。

### SPEC-240

実装レイヤーの孤児検出（宣言されていない実装の検出）は行わない。

*導出元: R-2, REQ-292*

*引用: 要件定義 R-2、§25 OOS-005*

### SPEC-241

根に指定されない孤児文書は `MISMATCH` とする（§4.3）。

### SPEC-242

target_bindingの問いは、そのTestが検証対象とする振る舞いが実際に生じ、その振る舞いを反映した観測が得られたかである。

### SPEC-243

Testがテストランナー上で `PASS` しても、検証対象とする振る舞いを実際には生じさせていない場合、完全検証済みOKとしない。

### SPEC-244

テストランナーの `PASS`/`FAIL` は判定権威（§7）の証拠として消費する。

### SPEC-245

target_binding検査は、その証拠が検証対象の実行を伴ったかを問う。

### SPEC-246

target_bindingは一つの問いに対し静的解析と動的計測の2つの証拠源を持つ。

### SPEC-247

静的に確定できなければ `UNKNOWN` とする。

### SPEC-248

静的に確定できない場合、動的証拠で昇格できる。

### SPEC-249

実装construct（Source Target）を検証対象とする実行形態では、宣言された対象コードが実際にTest実行経路へ入ったことを確認方法とする。

### SPEC-250

複数targetを宣言したTestでは各targetの実行を個別に計測する。

### SPEC-251

複数targetを宣言したTestでは、1件でも実行回数が0なら `FAIL`（診断NOT_EXECUTED）とする。

### SPEC-252

複数targetを宣言したTestでは、1件でも解析不能でかつ `FAIL` が無ければ `UNKNOWN` とする。

### SPEC-253

複数targetを宣言したTestでは、全targetの実行を確認できた場合だけ `PASS` とする。

### SPEC-254

別プロセス（起動したsubprocess）・別スレッド・クロージャ・他ファイル等、静的解析の到達境界を越えてtargetを実行するTestでは静的に到達を証明できず `UNKNOWN` となる。

### SPEC-255

静的解析の到達境界を越えてtargetを実行するTestの到達 `UNKNOWN` は、当該targetの動的計測が実行を証明した場合に限り到達要件を満たす。

### SPEC-256

subprocessであること自体を欠陥としない。

### SPEC-257

他の実行形態における確認方法は、当該形態に適した方法として詳細設計で定める。

### SPEC-258

特定形態の確認方法を別形態のTestへ一律要求しない。

*導出元: REQ-064, REQ-065, REQ-066, REQ-067, REQ-068, REQ-069, REQ-070, REQ-071, REQ-072, REQ-125, REQ-126, REQ-127, REQ-128, REQ-129, REQ-130, REQ-131, REQ-132, REQ-133, REQ-134, REQ-135, REQ-136, REQ-137, REQ-138, REQ-139, REQ-140, REQ-141, REQ-142, REQ-143*

*引用: 要件定義 §4.3、§8 条項 3*

### SPEC-259

target_bindingは完全検証ではデフォルト有効とする。

### SPEC-260

target_bindingは高速な限定scopeでは省略可能とする。

### SPEC-261

省略・計測環境不在の場合は `NO_EVIDENCE`（診断NOT_CHECKED）とする。

### SPEC-262

省略・計測環境不在の場合は `PASS` へ変換しない。

### SPEC-263

oracle_presenceの問いは、宣言された「何の時にどうなる」の不成立を、Testの非成功として反映する装置が存在するかである。

### SPEC-264

不成立が構造から証明できる（どんな宣言の下でも不成立を検出できない＝失敗し得ない、または失敗が検証対象の振る舞いに依存しないことが構造から証明できる）場合、oracle_presenceの出力は `FAIL` とする。

*導出元: REQ-073, REQ-074, REQ-075, REQ-076, REQ-077, REQ-078, REQ-079, REQ-080, REQ-081, REQ-082, REQ-083, REQ-084*

*引用: 要件定義 §4.4*

### SPEC-265

照合装置の存在が決定論的に確認できる場合、oracle_presence検査は成立側とする。

### SPEC-266

不成立が構造から証明できることと照合装置の存在が決定論的に確認できることのどちらも決定論的に言えない（解析不能等）場合、oracle_presenceの出力は `UNKNOWN` とする。

### SPEC-267

静的解析の役割は不成立の証明である。

### SPEC-268

静的解析は成立条件から明確に外れるTestを決定論的に検出し、外部監査へ送る前に拒否する（§8）。

### SPEC-269

静的解析は成立の証明装置ではない。

### SPEC-270

証明の失敗は `UNKNOWN` の事由ではない。

### SPEC-271

照合内容が宣言の期待と意味的に一致するかはoracle_presence検査の主張に含めない。

### SPEC-272

意味の疑義は検査ではなくエスカレーション（§11）の領分である。

### SPEC-273

答えはassertの所在・実行形態（内部construct検証か境界の振る舞い検証か）に依らず同一でなければならない。

### SPEC-274

実行形態別の判定規則を設けない。

### SPEC-275

`rust-cargo` adapterのStatic Audit capabilityは、§8.3の不成立構造を決定論的に検出する。

### SPEC-276

Static Auditは、成否判定が定数である（`assert!(true)` 等、失敗し得ない）Testを対象とする。

### SPEC-277

Static Auditは、検証対象の振る舞いを生じさせるだけで、その観測を成否判定に利用していないTestを対象とする。

### SPEC-278

Static Auditは、観測同士の自己比較（`assert_eq!(x, x)` 等）で成否が検証対象の振る舞いに依存しないTestを対象とする。

### SPEC-279

Static Auditは、空のテスト本体を持つTestを対象とする。

### SPEC-280

判定は保守的に行う。

### SPEC-281

決定論的に確定できる違反のみ `FAIL` とする。

### SPEC-282

確定できないものは `UNKNOWN` とする。

### SPEC-283

coreはadapter固有のAST・assertion構文・call graphを解釈しない。

### SPEC-284

coreは正規化されたルール結果を検証・集約する。

### SPEC-285

code fragmentの具体構文はadapterの言語・runnerに従う。

### SPEC-286

共通契約がRust構文を要求しない。

*導出元: R-3*

*引用: 要件定義 R-3、§8.3*

### SPEC-287

証拠は検証対象の内容ハッシュに束縛される。

*導出元: REQ-115, REQ-116, REQ-117, REQ-118, REQ-119, REQ-120*

*引用: 要件定義 §6*

### SPEC-288

証拠ストアはハッシュキーを必須とする。

### SPEC-289

現在のソースのハッシュと一致しない証拠は、検証時に「存在しないもの」として扱う。

### SPEC-290

現在のソースのハッシュと一致しない証拠は `NO_EVIDENCE`（診断STALE）とする。

### SPEC-291

Evidenceの判定結果を変えうるTestの意味・実行条件・対象実装・実行可能状態が現在状態と一致することを確認できなければ、そのEvidenceを現在の `PASS` として利用してはならない。

### SPEC-292

Evidenceの判定結果を変えうるTestの意味・実行条件・対象実装・実行可能状態が現在状態と一致することを確認できなければそのEvidenceを現在の `PASS` として利用してはならないという要求は、ハッシュ束縛によって設計制約として満たす。

### SPEC-293

鮮度の独立検査は設けない。

### SPEC-294

鮮度喪失は診断ラベル `STALE` として説明する。

### SPEC-295

Testの内容ハッシュは Test construct だけでなく Test subject 全体（少なくとも adapter ID・Test ID・全論理 field・Source Location・実行座標・Test construct）へ束縛する。

### SPEC-296

`covers` / `targets` / `intent` / 実行座標その他の意味変更は内容ハッシュを必ず変化させる。

### SPEC-297

adapterはsource range・source bytes・解析した論理metadata・実行座標をhash未計算のdiscovery DTOとして返す。

### SPEC-298

coreが言語非依存の正規化規則でsubject hashを計算してからTest Entityを具体化する。

### SPEC-299

adapterが最終内容ハッシュを自己確定してはならない。

### SPEC-300

テスト合否の判定権威は、当該adapterのテストランナーにある。

*導出元: REQ-121, REQ-122, REQ-123, REQ-124*

*引用: 要件定義 §7*

### SPEC-301

本システムは合否を判定しない。

### SPEC-302

本システムはテストランナーの結果を証拠として消費する。

### SPEC-303

実行の起動は本システムから行ってよい。

### SPEC-304

実行の起動を本システムから行っても、判定はしない。

### SPEC-305

`rust-cargo` adapterにおける判定権威は `cargo test` である。

### SPEC-306

`vtest` は照合（宣言・実装・証拠の一致検査＝§5の4検査）を行う。

### SPEC-307

`vtest` はテストの合否そのものを再判定しない。

### SPEC-308

`target_binding`（§5.3）はランナーの `PASS` を前提に、その `PASS` が検証対象の実行を伴ったかを問う独立の照合である。

### SPEC-309

管理対象となるTestは、その宣言された目的に対して、検証対象の振る舞いを反映した観測に基づく有効な成否判定を持たなければならない。

### SPEC-310

仕様適合性の証拠として算入するTestは、その検証成立性が確認済みでなければならない。

### SPEC-311

Testとして成立しているかの検査（§8）と、仕様適合性の証拠として算入するかの判定は独立である。

*導出元: REQ-125, REQ-126, REQ-127, REQ-128, REQ-129, REQ-130*

*引用: 要件定義 §8.1*

### SPEC-312

全Testを管理対象とすること（`chain_integrity`）と証拠算入（成立性）は別系統とする。

### SPEC-313

Testは検証対象の振る舞いを反映した結果・状態・観測に基づいて適合と不適合を識別しなければならない（検証成立性）。

### SPEC-314

不適合がTestの非成功として反映されるものでなければならない（検証成立性）。

### SPEC-315

Testの成否判定が他の構成要素の判定能力に依存する場合、その依存要素の正当性が確認されるか検証基盤として明示的に信頼されていなければ、当該Testの検証成立性を確認済みとして扱ってはならない（依存要素の信頼性）。

### SPEC-316

判定能力を担う依存要素は、正当性確認対象または明示的な信頼基盤として識別可能でなければならない。

### SPEC-317

成立性確認は正当性確認対象または明示的な信頼基盤のいずれかで終端しなければならない。

### SPEC-318

成立条件の確認方法は検証対象・実行形態・観測方法に応じて異なってよい（証明方法への非依存）。

### SPEC-319

特定形態固有の確認方法を別形態へ一律要求しない。

### SPEC-320

成立性の問いへの答えは確認方法に依らず同一でなければならない。

### SPEC-321

成立条件を確認できないことと、成立条件に違反していることを区別する（未確認と違反の区別）。

### SPEC-322

確認不能であることだけを根拠に違反を推定してはならない。

### SPEC-323

確認不能であることだけを根拠に成立確認済みとして扱ってもならない。

### SPEC-324

`oracle_presence` の信頼基盤の具体的範囲（標準assert構文・framework failure semantics・設定による列挙）と委譲確認の方法は詳細設計へ委譲する。

*導出元: REQ-309, REQ-310, REQ-311, REQ-312, REQ-313, REQ-314, REQ-315, REQ-316, REQ-317, REQ-318, REQ-319, REQ-320, REQ-321, REQ-322, REQ-323, REQ-324, REQ-325, REQ-326, REQ-327, REQ-328, REQ-329, REQ-330, REQ-331, REQ-332*

*引用: 要件定義 §28*

### SPEC-325

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

### SPEC-326

§8.2の成立条件を満たさないことを、宣言の中身に依らず決定論的に検出できる例はいずれも「どんな宣言の下でも不成立を検出できない」ことが構造から証明できる。

*導出元: REQ-138, REQ-139, REQ-140, REQ-141, REQ-142, REQ-143*

*引用: 要件定義 §8.3*

### SPEC-327

code fragmentはRustによる例示である。

### SPEC-328

共通契約がRust構文を要求しない。

*導出元: R-3*

*引用: R-3*

### SPEC-329

各adapterは対応する言語・runnerの構造に対して決定論的に判定できる範囲を提供する。

### SPEC-330

`static_audit` に相当する判定は §5.4/§5.5 の `oracle_presence` として現れる。

### SPEC-331

`static_audit` に相当する判定は独立した検査項目を新設しない。

### SPEC-332

すべての管理対象Testは1件以上の検証対象を宣言できなければならない。

### SPEC-333

検証対象は、そのTestが検証成立性（§8）を証明しようとする対象、すなわち宣言された「何の時にどうなる」の主語である。

### SPEC-334

検証対象は実装constructに限定しない。

### SPEC-335

外部から観測可能な契約・境界上の振る舞いも検証対象にできる。

*導出元: REQ-144, REQ-145, REQ-146, REQ-147, REQ-148*

*引用: 要件定義 §9.1*

### SPEC-336

実装construct（Source Target）を直接検証する実行形態ではSource Target宣言をそのまま検証対象の宣言として扱う。

### SPEC-337

実装construct（Source Target）を直接検証する実行形態では、同一対象の二重宣言を要求しない。

### SPEC-338

外部契約・境界上の振る舞いを検証する実行形態では、その契約・振る舞いを検証対象とする。

### SPEC-339

外部契約・境界上の振る舞いを検証する実行形態では、内部Source Targetの宣言をTest成立性の必須条件としない。

### SPEC-340

実装コード上のimplementation constructをSource Targetとして識別可能でなければならない。

*導出元: REQ-149, REQ-150, REQ-151, REQ-152, REQ-153, REQ-154, REQ-155, REQ-156*

*引用: 要件定義 §9.2*

### SPEC-341

1つのTestは1件以上のSource Targetを宣言できる。

### SPEC-342

複数targetを宣言した場合も各targetを独立に識別する。

### SPEC-343

複数targetを宣言した場合も代表1件へ縮約しない。

### SPEC-344

ソースコードへの恒久ID埋め込みは必須としない。

### SPEC-345

各adapterはSource Targetを一意に解決でき、同一source stateから決定論的に正規化できるTarget Referenceを提供する。

### SPEC-346

具体的構文・namespace・symbol種別は詳細設計へ委譲する。

### SPEC-347

共通契約が特定言語構造を必須としない。

*導出元: R-3*

*引用: R-3*

### SPEC-348

恒久SRC IDを使用する場合、adapter境界を越えてrepository全体で一意でなければならない。

### SPEC-349

同一SRC IDの複数宣言を曖昧参照として受理しない。

### SPEC-350

検証対象とは別に、Testまたは検証対象から関連するSource Targetへのtraceabilityを保持できる。

### SPEC-351

Testまたは検証対象から関連するSource Targetへのtraceabilityは任意であり、影響分析・逆引きに利用できる。

### SPEC-352

traceabilityの存在自体をTest成立性の条件としてはならない。

### SPEC-353

traceabilityは関連付けであって実装対応の証明ではない。

### SPEC-354

検証対象と実装traceabilityは別の関係として扱う。

### SPEC-355

検証対象と実装traceabilityは、一方から他方を推定してはならない。

*導出元: REQ-157, REQ-158, REQ-159, REQ-160, REQ-161*

*引用: 要件定義 §9.3*

### SPEC-356

Source Targetとの関係を持つTestについて、TestからSourceを検索できる。

### SPEC-357

Source Targetとの関係を持つTestについて、Sourceから関連Testを逆引きできる。

### SPEC-358

VOは独立して「この条件が成立するか」と検証可能な仕様上の命題とする。

### SPEC-359

VOの粒度をassert文・test function・テストファイルなどのコード構文で決めない。

*導出元: REQ-162, REQ-163*

*引用: 要件定義 §10.1*

### SPEC-360

仕様は、テストで十分な網羅性を確認できる単位までVOへ分解できる。

### SPEC-361

本システムは分解を表現・保持するデータモデルを提供する。

### SPEC-362

分解が十分かの判定は本システムの検査ではなくエスカレーション（§11）の領分である。

*導出元: REQ-164, REQ-165, REQ-166*

*引用: 要件定義 §10.2*

### SPEC-363

VOは階層構造を持てる。

### SPEC-364

初回登録時の階層化を必須としない。

### SPEC-365

flatなVO群と階層化VO群の双方を扱う。

### SPEC-366

flatなVOを再帰分解・階層化する操作を提供する。

*導出元: REQ-167, REQ-168, REQ-169*

*引用: 要件定義 §10.3*

### SPEC-367

VOとTestの対応は1:1に限定せず `1:1` / `1:N` / `N:1` / `N:M` を許容する。

### SPEC-368

TestはVOの検証実装単位でありVOそのものではない。

*導出元: REQ-170, REQ-171, REQ-172, REQ-173*

*引用: 要件定義 §10.4*

### SPEC-369

VOには検証軸（dimension）と同値/境界値partitionを定義できる。

### SPEC-370

検証軸とpartitionの定義はすべてのVOへは要求しない。

### SPEC-371

複数軸を持つVOには組合せcoverageの方針を宣言できる（各軸独立／全直積／明示列挙）。

### SPEC-372

何をもって十分とするかの判定は本システムの検査ではない（→§11）。

### SPEC-373

複数観点を同時確認するTestの存在だけを理由に各観点を独立に証明したことにはしない。

### SPEC-374

各観点の独立検証、または必要と定義された組合せ空間の検証のいずれかを表現・確認できる。

*導出元: REQ-174, REQ-175, REQ-176, REQ-177, REQ-178, REQ-179*

*引用: 要件定義 §11*

### SPEC-375

partition・組合せcoverage方針の具体的保存形式・語彙は詳細設計へ委譲する。

*導出元: REQ-309, REQ-310, REQ-311, REQ-312, REQ-313, REQ-314, REQ-315, REQ-316, REQ-317, REQ-318, REQ-319, REQ-320, REQ-321, REQ-322, REQ-323, REQ-324, REQ-325, REQ-326, REQ-327, REQ-328, REQ-329, REQ-330, REQ-331, REQ-332*

*引用: 要件定義 §28*

### SPEC-376

本システムは、宣言されていない義務・網羅漏れ・宣言と実装の意味のずれを、自ら発見・裁定しない。

### SPEC-377

発見者・裁定者は外部（人間またはAgent）である。

*導出元: REQ-180, REQ-181, REQ-182, REQ-183, REQ-184, REQ-185, REQ-186, REQ-187, REQ-188, REQ-189, REQ-190, REQ-191, REQ-192*

*引用: 要件定義 §12*

### SPEC-378

本システムの責務は、データ形態の提供（§11.1）、エスカレーション（§11.2）、判断の記録と再検証（§11.3）の3つに限る。

### SPEC-379

外部の発見者が判断できる構造化出力（要求該当箇所と対応概念のペア、宣言鎖と検査結果、対象外とした範囲）を提供する。

### SPEC-380

`vtest` 自身はLLM APIを呼ばない。

### SPEC-381

`vtest` は意味判定・候補生成を検証成立条件にしない。

*導出元: REQ-224, REQ-225, REQ-226, REQ-227*

*引用: 要件定義 §17.2*

### SPEC-382

外部AI/Agentによる補助・提案は許容する。

### SPEC-383

外部AI/Agentの能力を成立条件にしない。

### SPEC-384

機械が決定論で確定できない疑義は `UNKNOWN` として、意味判定できる者へ引き渡す。

### SPEC-385

`UNKNOWN` は正常動作としての降参である。

### SPEC-386

`UNKNOWN` はエラー処理のフォールバック先に使わない（§4.4）。

### SPEC-387

`UNKNOWN` に対して外部（人間または判断可能Agent）が判断できる。

### SPEC-388

判断はその時点の対象成果物・前提状態に対して、依存closure（ハッシュ）とともに判断記録へ保存する。

### SPEC-389

判断記録は少なくとも「誰が（actor）」「何を（subject）」「どう判断したか（decision）」を必須項目とする。

### SPEC-390

判断記録は追跡可能とする。

### SPEC-391

判断記録の理由・根拠・evidence note（根拠となった宣言、対象外とした範囲、具体例等）は任意（optional）とする。

### SPEC-392

判断記録の理由・根拠・evidence noteは保存できる構造とする。

### SPEC-393

理由が空であることだけを根拠に、その判断を無効・`UNKNOWN`・`NO_EVIDENCE`・`MISMATCH` 等として扱ってはならない。

*導出元: REQ-180, REQ-181, REQ-182, REQ-183, REQ-184, REQ-185, REQ-186, REQ-187, REQ-188, REQ-189, REQ-190, REQ-191, REQ-192*

*引用: 要件定義 §12*

### SPEC-394

`vtest` が判断対象の情報一式（VO、Test Intent、テストコード、対象実装、関連テスト、既知partition、過去の判断、対象の内容ハッシュとリビジョン）を構造化出力する（bundle生成）。

### SPEC-395

外部の人間/Agentが判断し、判断結果（decision＋任意の理由）を提出する。

### SPEC-396

`vtest` はbundleとの対応・対象内容ハッシュの現在一致・decision値の妥当性を検証して受理・拒否する。

### SPEC-397

`vtest` は受理結果を依存closureのハッシュに束縛して保存する。

### SPEC-398

判断記録の生成・保存の構造化プロトコルは検証状態のゲートではない。

### SPEC-399

判断記録の受理は当該対象の検証状態を昇格させない（§4.5）。

### SPEC-400

判断済みと承認済みは区別する（判断済み ≠ 承認済み）。

### SPEC-401

判断は承認なしでも記録できる。

### SPEC-402

正式採用は§17の別段階である。

### SPEC-403

判断記録と承認記録は同一entityであることを要求しない（別entityでありうる）。

### SPEC-404

仕様・VO・Test等が変更された場合、過去の判断を現在状態へそのまま流用してはならない。

### SPEC-405

仕様・VO・Test等が変更された場合、現在状態に対して通常の検証を再実施する。

### SPEC-406

仕様・VO・Test等が変更された場合、現在状態に対して通常の検証を再実施した結果は `PASS` / `FAIL` / `MISMATCH` / `NO_EVIDENCE` / `UNKNOWN` のいずれにもなり得る。

### SPEC-407

変更そのものが `UNKNOWN` を生成するのではない。

### SPEC-408

エスカレーション出力・判断記録の具体的schema、判断待ち情報（§18.3）の構造schemaと取得インターフェース、判断の多重度は詳細設計へ委譲する。

*導出元: REQ-309, REQ-310, REQ-311, REQ-312, REQ-313, REQ-314, REQ-315, REQ-316, REQ-317, REQ-318, REQ-319, REQ-320, REQ-321, REQ-322, REQ-323, REQ-324, REQ-325, REQ-326, REQ-327, REQ-328, REQ-329, REQ-330, REQ-331, REQ-332*

*引用: 要件定義 §28*

### SPEC-409

各Testは安定したTest IDによって識別可能とする。

### SPEC-410

Test IDをハンドルとして、Test Intent・`covers`（VO参照）・検証対象・Source Target・Location・判断記録・Execution Evidenceを検索可能とする。

*導出元: REQ-193, REQ-194, REQ-195, REQ-196, REQ-197, REQ-198, REQ-199, REQ-200, REQ-201, REQ-202*

*引用: 要件定義 §13*

### SPEC-411

登録adapterがTestとして発見した実行可能なtest constructはすべて管理対象とする。

### SPEC-412

発見されたTest集合を `D`、構造上完全なmanaged Test Entity集合を `M` とする。

### SPEC-413

構造上完全とは、source declarationから構文上有効なTest ID・1件以上の `covers`・その他の必須metadataをTest Entityとして具体化できることをいう。

### SPEC-414

Discovered Testとentityの対応数は構造完全性に含めず、独立した整合性条件とする。

### SPEC-415

`M` はVO参照の解決とTest IDの大局的一意性を検査する前の集合とする。

### SPEC-416

`M` は解決不能な `covers` を持つentityやTest IDが衝突するentityも含む。

### SPEC-417

完全検証では、発見された各Test dについて、dに対応するmanaged Test Entityがちょうど1件存在し、managed Test Entity.coversが1件以上であり、coversの全VO参照を解決でき、Test IDが発見結果全体で一意であることを要求する（`chain_integrity`。§5.1）。

> ∀ d ∈ D:
>   d に対応する managed Test Entity がちょうど 1 件存在する
>   and managed Test Entity.covers は 1 件以上である
>   and covers の全 VO 参照を解決できる
>   and Test ID が発見結果全体で一意である

### SPEC-418

違反時の状態は §4.3 に従う。

### SPEC-419

違反時、いずれも完全検証の `PASS` として扱わない。

### SPEC-420

発見されたが管理宣言を持たないconstruct（`rust-cargo` では `@vtest` annotation を持たない `#[test]` 等）は診断severityとしてはwarningのままとする。

### SPEC-421

構造上完全なmanaged Test Entityへ対応しない事実は `chain_integrity` の非 `PASS`（`MISMATCH`/MISSING）として完全検証へ反映する。

### SPEC-422

診断severityと検証状態を混同しない。

### SPEC-423

Testの存在理由による分類（role / anchor / anchor_rationale等）と、それに基づく `covers` 件数の可変制約はv0.1では設けない。

### SPEC-424

すべての管理対象Testに `covers ≥ 1` を一律に要求する。

*導出元: REQ-052, REQ-053, REQ-054, REQ-055, REQ-056, REQ-057, REQ-058, REQ-193, REQ-194, REQ-195, REQ-196, REQ-197, REQ-198, REQ-199, REQ-200, REQ-201, REQ-202*

*引用: 要件定義 §4.1、§13*

### SPEC-425

VOへの寄与は `covers` 宣言と証拠の十分性判定だけから導出する。

### SPEC-426

Testには、その実装コードだけを読まなくても、何を検証するか・どのVOに対応するか・何を入力条件とするか・何を期待するかを判断できる情報を関連付けられる。

### SPEC-427

Test IntentはTest Entityの付随情報であり、宣言鎖のノードではない。

*導出元: REQ-203, REQ-204, REQ-205*

*引用: 要件定義 §14*

### SPEC-428

具体的入力値をTest IntentまたはVOへ含めることを許容する。

### SPEC-429

具体的入力値をTest IntentまたはVOへ含めることは必須としない。

### SPEC-430

table-drivenの論理形式を正式に許容する。

### SPEC-431

adapterが識別したtable-driven test construct全体を一つのTestとして登録できる。

### SPEC-432

内部の各caseを独立Test IDへ分解することを必須としない。

### SPEC-433

cases集合がVOに必要な入力空間を十分に代表・網羅しているかは§11の発見・判定の対象とする。

### SPEC-434

code fragmentの具体構文はadapterの言語・runnerに従う。

*導出元: R-3, REQ-206, REQ-207, REQ-208, REQ-209*

*引用: 要件定義 §15、R-3*

### SPEC-435

Test操作の公式経路として、Test IDまたはadapterが識別可能なTest constructを対象とした構造化操作を提供する。

*導出元: P-004, REQ-210, REQ-211, REQ-212, REQ-213, REQ-214, REQ-215, REQ-216, REQ-217*

*引用: 要件定義 §16、P-004*

### SPEC-436

Create Testは、Form Schemaに基づく構造化入力をadapterへ渡し、Test constructと対応するmetadata宣言を生成する。

### SPEC-437

Edit Testは、Test IDを編集ハンドルとして、adapterが識別する対象Testのmetadata宣言およびTest constructを更新する。

### SPEC-438

Query Testは、Test ID・VO・Target Reference等からの検索と逆引きを行う。

### SPEC-439

Audit（判断）Testは、§11の判断記録bundle生成と判断結果の提出を行う。

### SPEC-440

Create / Editの入力は差分操作ではなくあるべき状態（desired state）とする。

### SPEC-441

利用者は「TEST-Xはこの状態である」を宣言する。

### SPEC-442

adapterが現状との差分を計算してTest constructとmetadata宣言を更新する。

### SPEC-443

coreが結果を再スキャンして検証する。

### SPEC-444

構造化入力の各項目は可能な限り受理時に検証する（対象symbol不在、Test ID不在、参照VO不在等）。

### SPEC-445

解決不能な場合はadapterが候補を提示する。

### SPEC-446

公式Edit操作の一回の対象は原則1Testとする。

### SPEC-447

公式Edit操作は暗黙に他Testを変更しない。

### SPEC-448

編集はadapterが特定した単一のmetadata宣言範囲とTest construct範囲に限定する。

### SPEC-449

Test外部の通常ソースコード・helper・fixtureの編集は責務外とする。

### SPEC-450

Test外部の通常ソースコード・helper・fixtureの編集操作は提供しない。

*導出元: REQ-210, REQ-211, REQ-212, REQ-213, REQ-214, REQ-215, REQ-216, REQ-217, REQ-288, REQ-289, REQ-290, REQ-291, REQ-292, REQ-293, REQ-294*

*引用: 要件定義 §16、§25 OOS-003*

### SPEC-451

通常のwrite/editツールや人間による直接ソース編集は完全禁止しない。

### SPEC-452

公式経路の提供により誤編集・更新忘れ・複数Test同時変更の事故を低減する。

### SPEC-453

直接編集による不整合も検証（§5.1）で検出可能とする。

### SPEC-454

source declarationが正典であるため、`covers` / `targets` の「同期漏れ」は構造的に発生しない。

*導出元: REQ-210, REQ-211, REQ-212, REQ-213, REQ-214, REQ-215, REQ-216, REQ-217*

*引用: 要件定義 §16*

### SPEC-455

テスト種別ごとの質問・入力項目テンプレートをForm Schemaとして `.verify/forms/` に定義できる。

### SPEC-456

Rust関数単体Test用と小規模結合Test用の組込schemaを同梱する。

### SPEC-457

`rust-cargo` adapterが組込schemaを登録する。

### SPEC-458

CLI・MCPのいずれからも同一schemaを消化できる。

### SPEC-459

Form Schemaの `kind` はrepository内で大局的に一意なForm IDとする。

### SPEC-460

schemaはそれを処理するadapter IDを別fieldで宣言する。

### SPEC-461

registryは `kind` からちょうど1件のStructured Test adapterへ解決できる場合だけ操作を許可する。

### SPEC-462

registryは重複・未知adapter・未対応capability・曖昧な対応を拒否する。

### SPEC-463

未知のformをcoreがRust用として推測してはならない。

### SPEC-464

境界値・partitionの必須入力化は組込Formでは設けない。

### SPEC-465

境界値・partitionの必須入力化はuser-defined Form Schemaが指定できる。

*導出元: REQ-174, REQ-175, REQ-176, REQ-177, REQ-178, REQ-179, REQ-309, REQ-310, REQ-311, REQ-312, REQ-313, REQ-314, REQ-315, REQ-316, REQ-317, REQ-318, REQ-319, REQ-320, REQ-321, REQ-322, REQ-323, REQ-324, REQ-325, REQ-326, REQ-327, REQ-328, REQ-329, REQ-330, REQ-331, REQ-332*

*引用: 要件定義 §28、§11*

### SPEC-466

仕様ソースとして、ソースコードより上流に位置する成果物（要件定義・基本仕様・詳細設計・API Schema・Protocol Specification・型/データ仕様・DB schema・その他の機械可読仕様）を利用可能とする。

### SPEC-467

取り込まれた上流成果物は §2.2 の `document` ノードとして登録される。

### SPEC-468

取り込まれた上流成果物はcontent_hashとderives_fromを持つ。

*導出元: REQ-233, REQ-234, REQ-235*

*引用: 要件定義 §18*

### SPEC-469

対象ソースコード内のdoc commentを、その対象実装自身の正当性を証明する唯一の仕様根拠として使用しない。

*導出元: REQ-233, REQ-234, REQ-235*

*引用: 要件定義 §18*

### SPEC-470

文書の具体的入力フォーマットと登録方式、根の指定方式は詳細設計へ委譲する。

*導出元: REQ-309, REQ-310, REQ-311, REQ-312, REQ-313, REQ-314, REQ-315, REQ-316, REQ-317, REQ-318, REQ-319, REQ-320, REQ-321, REQ-322, REQ-323, REQ-324, REQ-325, REQ-326, REQ-327, REQ-328, REQ-329, REQ-330, REQ-331, REQ-332*

*引用: 要件定義 §28*

### SPEC-471

承認とは、判断（§11の判断記録を含む）または方針を「この内容で進める」と正式に認め確定状態にすることである。

### SPEC-472

判断済みと承認済みは区別する（判断済み ≠ 承認済み）。

### SPEC-473

未承認の判断は承認済みより弱い。

*導出元: REQ-236, REQ-237, REQ-238, REQ-239, REQ-240, REQ-241, REQ-242, REQ-243, REQ-244, REQ-245, REQ-246, REQ-247, REQ-248, REQ-249, REQ-250, REQ-251, REQ-252*

*引用: 要件定義 §19*

### SPEC-474

VO等の検証成果物について確定・承認状態を表現可能とする。

### SPEC-475

承認は対象または参照する判断（judgment reference）に承認済み状態を与える。

### SPEC-476

§11の `UNKNOWN` 判断も承認対象になり得る。

### SPEC-477

判断できることと正式承認は別段階である。

### SPEC-478

承認は対象自身の内容だけでなく、承認判断が依存する上流文書・上位VOの現在の依存closureへ束縛する。

### SPEC-479

VOの依存closureは、再帰的な上位VO・参照するdocument（およびその上位document）からなる。

### SPEC-480

対象またはいずれかの依存成果物が変更された承認を、現在の承認済み状態として利用してはならない。

### SPEC-481

変更後は現在状態に対して検証を再実施する。

### SPEC-482

変更後の検証結果は§4.1の5状態のいずれかに従う。

### SPEC-483

依存closureまたはハッシュを欠く承認を推測で有効化してはならない。

### SPEC-484

承認レコードは読み取り互換のため保持できるが、現在の承認済みを導出してはならない。

### SPEC-485

承認記録は「誰が（approver）」「何を（subject または judgment reference）」「どの承認状態か（approved state）」を必須項目として追跡可能とする。

### SPEC-486

承認記録の根拠は任意（optional）に記録できる。

### SPEC-487

承認記録は§11の判断記録と同一entityであることを要求しない。

### SPEC-488

承認主体を人間に限定しない。

### SPEC-489

Agentも承認権限を持ち得る（Human / Verification Agent / Reviewer Agent / PM Agent等）。

### SPEC-490

全Agentが承認権限を持つことは要求しない。

### SPEC-491

一般作業Agentが承認権限を持つべきとも要求しない。

### SPEC-492

承認主体は種別（`human` / `agent`）と識別子（エージェント名・モデル名等）を記録する。

### SPEC-493

誰がどの対象・範囲を承認できるか（approval authority）はプロジェクト側で定義可能とする。

### SPEC-494

承認は検証状態と独立の別軸である（§4.5）。

### SPEC-495

承認済みを理由に非 `PASS` を `PASS` へ昇格させない。

### SPEC-496

未承認を理由に `PASS` を降格させない。

### SPEC-497

具体的な承認ロール・必要承認数・権限schema・承認workflowの状態遷移は詳細設計へ委譲する。

*導出元: REQ-309, REQ-310, REQ-311, REQ-312, REQ-313, REQ-314, REQ-315, REQ-316, REQ-317, REQ-318, REQ-319, REQ-320, REQ-321, REQ-322, REQ-323, REQ-324, REQ-325, REQ-326, REQ-327, REQ-328, REQ-329, REQ-330, REQ-331, REQ-332*

*引用: 要件定義 §28*

### SPEC-498

本システムはプロジェクト開始時からの導入を前提としない。

### SPEC-499

開発途中または既存プロジェクトへ後から導入できる。

*導出元: R-5*

*引用: 要件定義 R-5、§17*

### SPEC-500

既に大量のソースコードとTestが存在するプロジェクトを検証対象として扱える。

### SPEC-501

既存の文書・Source・Testを読み取り、VOの存在状況・既存TestとVOの対応・Testの不足・検証成立性・宣言との不一致を可視化する。

### SPEC-502

VOが確定していない範囲を含むプロジェクトも読み取れる。

### SPEC-503

未登録Test・欠落する宣言・未確定のVO・未実施の検査または実行を検証済みとして扱わない（状態は§4.3）。

### SPEC-504

`vtest init` は `.verify/` を作成する。

### SPEC-505

`vtest init` は既存コードを変更しない。

### SPEC-506

`vtest scan` は発見した未登録Testを未登録として報告する。

### SPEC-507

document / VO、Test metadata宣言、判断記録、Evidenceの一部が欠ける状態も読み取り可能とする。

### SPEC-508

`vtest verify` は正典または検証事実の欠落を対応する非 `PASS` 値として表示する。

### SPEC-509

`vtest verify` は部分的な登録・判断・実行状態を総合 `OK` として扱わない。

### SPEC-510

決定論的に処理可能な作業について人間の反復手入力を必須としない。

### SPEC-511

要求・要件・仕様・VO等の意味上の定義や対応関係を決定する責任はプロジェクト側（開発者・設計者・PM等）にある。

### SPEC-512

本システムが意味判断・候補生成を行うことを必須要件としない。

### SPEC-513

外部AI/Agentによる補助・提案は許容する。

### SPEC-514

外部AI/Agentの能力を検証成立条件にしない。

*導出元: REQ-224, REQ-225, REQ-226, REQ-227*

*引用: 要件定義 §17.2*

### SPEC-515

導入時・検証時に生じる、未確定事項・不整合・未検証事項・機械的に確認済みの事実・外部判断が必要な事項を、機械可読な構造として保持・取得可能とする。

### SPEC-516

表示形式（表・GUI等）は要件でなく詳細設計へ委譲する。

*導出元: REQ-228, REQ-229, REQ-309, REQ-310, REQ-311, REQ-312, REQ-313, REQ-314, REQ-315, REQ-316, REQ-317, REQ-318, REQ-319, REQ-320, REQ-321, REQ-322, REQ-323, REQ-324, REQ-325, REQ-326, REQ-327, REQ-328, REQ-329, REQ-330, REQ-331, REQ-332*

*引用: 要件定義 §17.3、§28*

### SPEC-517

プロジェクト規模が大きいこと自体とは別の理由で導入難度が構造的に増大する設計を避ける。

### SPEC-518

プロジェクト規模が大きいこと自体とは別の理由で導入難度が構造的に増大する設計を避けることは、強い不変条件ではなく設計原則とする。

### SPEC-519

物量増加に伴う処理量・作業量の増加は許容する。

*導出元: REQ-230, REQ-231, REQ-232*

*引用: 要件定義 §17.4*

### SPEC-520

関係型を単一へ潰さず、横断してトレース可能にする。

*導出元: REQ-036, REQ-037, REQ-038, REQ-039, REQ-040, REQ-041, REQ-042, REQ-043, REQ-044, REQ-045, REQ-046, REQ-047, REQ-048, REQ-049*

*引用: 要件定義 §3.4、NFR-003*

### SPEC-521

文書間はderives_from、VO→Testはcovers、Test↔実装は検証対象／実装traceabilityのように性質の異なる関係型を区別する。

### SPEC-522

関係型そのものの意味論的増殖は求めない。

### SPEC-523

契約上必須と定義したリンク（`parent --relation--> child`）は必須とする。

### SPEC-524

任意（optional）と定義した関係（例：§9.3実装traceability）は欠落してよい。

### SPEC-525

存在するリンクに付す説明文・導出理由は任意とする。

### SPEC-526

存在するリンクに付す説明文・導出理由は空でも `chain_integrity` 違反・`MISMATCH` としない。

### SPEC-527

説明文を付加・保存できるデータ構造とする。

### SPEC-528

最小の意味単位「上流ノード → 関係 → 下流ノード」を任意のノードから取得できる。

### SPEC-529

必要に応じて上流／下流へ連続して辿れる。

### SPEC-530

プロジェクト全体のトレーサビリティ構造も取得できる。

### SPEC-531

常に全チェーンを表示することは求めない。

### SPEC-532

同一のトレーサビリティ構造から、利用者の役割または利用目的に応じて参照対象・関係・集約粒度を変えたprojectionを取得・提示できる。

> 例：PM は上位の document・VO の状態と未確定/NG、Tester は VO・Test・検証対象・Evidence・未実施/失敗理由、Coder は実装から関連 Test・VO・上流文書へのトレース。

### SPEC-533

役割を固定enumやモード名として仕様化することは本書では行わない。

### SPEC-534

preset・UI・モード体系は詳細設計へ委譲する。

*導出元: REQ-309, REQ-310, REQ-311, REQ-312, REQ-313, REQ-314, REQ-315, REQ-316, REQ-317, REQ-318, REQ-319, REQ-320, REQ-321, REQ-322, REQ-323, REQ-324, REQ-325, REQ-326, REQ-327, REQ-328, REQ-329, REQ-330, REQ-331, REQ-332*

*引用: 要件定義 §28*

### SPEC-535

プロジェクト側が登録したフェーズ・工程・ゲートの進行条件について、現在の検証状態（§4）と承認（§17）が通過条件を満たすかを評価・提示できなければならない（MUST）。

*導出元: REQ-302, REQ-303, REQ-304, REQ-305, REQ-306, REQ-307, REQ-308*

*引用: 要件定義 §26.4*

### SPEC-536

検証状態と承認は独立の軸である（§4.5）。

### SPEC-537

ゲートは両者の組合せを進行条件にできる。

### SPEC-538

ゲート条件の定義を受理できる。

> 通常開発中   : verification = PASS で進行可、approval 不要
> Release gate : verification = PASS + Reviewer approval
> Delivery gate: verification = PASS + Owner / PM approval

### SPEC-539

本システムの責務はゲート条件が現在満たされているかの評価・提示に限る。

### SPEC-540

フェーズのライフサイクル管理・工程の自動遷移は責務外とする。

*導出元: REQ-288, REQ-289, REQ-290, REQ-291, REQ-292, REQ-293, REQ-294, REQ-302, REQ-303, REQ-304, REQ-305, REQ-306, REQ-307, REQ-308*

*引用: 要件定義 §26.4、§25 OOS-004*

### SPEC-541

「Releaseフェーズへ遷移させる」のではなく「Release gateの条件を現在満たしている」を提示する。

### SPEC-542

具体的なフェーズ名・承認ロール・必要承認数・権限schema・進行条件定義は詳細設計へ委譲する。

*導出元: REQ-309, REQ-310, REQ-311, REQ-312, REQ-313, REQ-314, REQ-315, REQ-316, REQ-317, REQ-318, REQ-319, REQ-320, REQ-321, REQ-322, REQ-323, REQ-324, REQ-325, REQ-326, REQ-327, REQ-328, REQ-329, REQ-330, REQ-331, REQ-332*

*引用: 要件定義 §28*

### SPEC-543

`vtest run` はテストを実際に実行する。

### SPEC-544

`vtest run` は判定権威（§7）であるランナーの結果をEvidenceとして記録する。

*導出元: REQ-115, REQ-116, REQ-117, REQ-118, REQ-119, REQ-120, REQ-295, REQ-296*

*引用: 要件定義 §6、§26.1*

### SPEC-545

Evidenceには少なくともTest IDと実行結果（ランナーが報告した `PASS` / `FAIL`）を含める。

### SPEC-546

Evidenceには実行したadapter IDを含める。

### SPEC-547

Evidenceには実行時のリポジトリリビジョン（Git commit hash）とdirtyフラグを含める。

### SPEC-548

Evidenceには現在のTest subject全体の内容ハッシュ、および全宣言targetを解決したcanonical Target Referenceとimplementation constructの内容ハッシュを含める。

### SPEC-549

Evidenceには実行時HEAD revision、実行adapter・runner・toolchain・実行影響config、現在の実行可能状態を変えうるrepository / local dependency入力の完全なsnapshotを束縛したExecution State subjectを含める。

### SPEC-550

Evidenceには実行日時と実行方式を含める。

### SPEC-551

Evidenceには `target_binding`（§5.3）のtarget別結果とfail-closed集約結果（実施した場合）を含める。

### SPEC-552

鮮度は独立検査ではなく§6のハッシュ束縛により満たす。

### SPEC-553

Evidenceは、記録時のTest subject内容ハッシュの一致、target参照集合の一致、各target内容ハッシュの一致、adapter IDの一致、HEAD revisionの一致、およびExecution State subjectの一致をすべて満たす場合のみ有効とする。

*導出元: REQ-115, REQ-116, REQ-117, REQ-118, REQ-119, REQ-120*

*引用: 要件定義 §6*

### SPEC-554

Evidenceの記録時のTest subject内容ハッシュが現在と一致する。

### SPEC-555

Evidenceのtarget参照集合が、現在のTestの宣言targetを解決したcanonical Source Target集合と重複なく一致する。

### SPEC-556

Evidenceの記録時の各target内容ハッシュが、現在解決される各implementation constructの内容ハッシュと一致する。

### SPEC-557

Evidenceのadapter IDが現在のTestのexecution adapterと一致する。

### SPEC-558

Evidenceの記録時のHEAD revisionが特定され、現在のHEAD revisionと一致する。

### SPEC-559

Execution State subjectが完全であり、現在再構築したExecution State subjectと一致する（dirty状態のsource、target外helper、build script、local dependency、runner / toolchain / 実行影響configの変更を含む）。

### SPEC-560

内容ハッシュ・Execution State subject・revision条件を満たさないEvidenceは `NO_EVIDENCE`（診断STALE）とする。

### SPEC-561

内容ハッシュ・Execution State subject・revision条件を満たさないEvidenceは有効な `PASS` として扱わない。

### SPEC-562

adapterが実行入力集合の完全性を証明できない場合は `UNKNOWN` とする。

### SPEC-563

部分的snapshotから現在実装への `PASS` を推測しない。

### SPEC-564

Evidenceが存在しても鮮度が満たされないなら、そのEvidenceから実行関連の判定を `PASS`/`FAIL` として再利用しない。

### SPEC-565

Evidenceが存在しても鮮度が満たされないなら、同じ鮮度・対応関係の非 `PASS` 値を保持する。

### SPEC-566

Evidenceが存在しない場合は実行関連を `NO_EVIDENCE`（診断NOT_EXECUTED）とする。

### SPEC-567

Evidence readerはadapter IDを欠く互換recordも履歴として読み取れる。

### SPEC-568

Evidence readerは、現在のTestが `rust-cargo` で互換runner情報と内容ハッシュからRust実行と一意に確認できる場合に限り評価する。

### SPEC-569

Evidence readerは、Rust実行と一意に確認できない場合は `UNKNOWN` とする。

### SPEC-570

完全検証におけるOKは、宣言鎖全体に対する検査（`chain_integrity` / `orphan_detection`）と、scopeに含まれる各「宣言＋コード＋証拠」の組に対する検査（`target_binding` / `oracle_presence`）がすべて `PASS` であり、テストランナーの結果を含む証拠が§6を満たす場合に限る。

### SPEC-571

一項目でも `FAIL` / `MISMATCH` / `NO_EVIDENCE` / `UNKNOWN` であればNGとする（fail-closed）。

*導出元: P-002, REQ-295, REQ-296*

*引用: 要件定義 §26.1、P-002*

### SPEC-572

利用者向け簡易出力は `OK` / `NG` の二値とする。

### SPEC-573

完全検証の検査集合はこの4検査に固定する。

### SPEC-574

完全検証の検査集合は設定によって追加・削除できない。

### SPEC-575

検査の部分集合を指定した実行は限定scopeである。

### SPEC-576

検査の部分集合を指定した実行は完全検証として表示・集約しない（§4.6）。

### SPEC-577

Test単位の結果をVO・Feature・document単位へ集約可能とする。

### SPEC-578

集約はfail-closedを基本とする。

### SPEC-579

子に1つでも非 `PASS` があれば親は非 `PASS` とする。

*導出元: REQ-299, REQ-300, REQ-301*

*引用: 要件定義 §26.3*

### SPEC-580

集約時に複数の非 `PASS` 値が混在する場合、上位に表示する代表値の優先順位は `FAIL > MISMATCH > NO_EVIDENCE > UNKNOWN` とする。

### SPEC-581

診断ラベルは代表値の順位に用いず、原因説明として併記する。

### SPEC-582

詳細出力では子の個別値をすべて確認できる。

### SPEC-583

NGの場合、どのエンティティの・どの検査が・どの状態で・どの診断ラベルとともに落ちたかを掘り下げ可能とする。

*導出元: REQ-297, REQ-298*

*引用: 要件定義 §26.2、NFR-006*

### SPEC-584

簡易出力は総合OK / NGとする。

### SPEC-585

詳細出力は、任意ノードからの局所／経路／全体トレース（§19）に沿ったツリー表示とする。

### SPEC-586

詳細出力では非 `PASS` の根拠（判断記録・Evidenceへの参照）を辿れる。

### SPEC-587

`covers` を持つTestはVOの子として表示する。

### SPEC-588

管理下にある事実と、いずれのVOへも寄与しない事実の双方を出力から確認できる状態にする。

### SPEC-589

人間向けテキストと機械可読JSONの両方を出力できる。

*引用: 要件定義 NFR-007 / NFR-008*

### SPEC-590

adapter能力の欠落・失敗を `PASS` へ補完しない。

### SPEC-591

static解析またはcoverage能力がなければ該当項目は `NO_EVIDENCE`（診断NOT_CHECKED）とする。

### SPEC-592

runner能力がなければ実行関連は `NO_EVIDENCE`（診断NOT_EXECUTED）とする。

### SPEC-593

解析限界は `UNKNOWN` とする。

### SPEC-594

create / edit / audit / run等の明示的操作に必須の能力がなければ操作を失敗させる。

### SPEC-595

create / edit / audit / run等の明示的操作に必須の能力がなければファイル・判断記録・Evidenceを生成しない。

*導出元: REQ-258, REQ-259, REQ-260, REQ-261, REQ-262, REQ-263, REQ-264, REQ-265, REQ-266*

*引用: 要件定義 §21*

### SPEC-596

`vtest scan` はregistryに登録された全source discovery adapterへ委譲する。

### SPEC-597

`vtest scan` は統合したdiscovery結果と `.verify/` からエンティティと関係の全体グラフを再構築する。

### SPEC-598

`vtest scan` はその過程で `chain_integrity` / `orphan_detection` を構成する整合性検査を行う。

*導出元: REQ-193, REQ-194, REQ-195, REQ-196, REQ-197, REQ-198, REQ-199, REQ-200, REQ-201, REQ-202, REQ-271, REQ-272, REQ-273, REQ-274, REQ-275, REQ-276, REQ-277, REQ-278, REQ-279*

*引用: 要件定義 §13、§23*

### SPEC-599

Test IDの重複（identity collision）は `MISMATCH` とする。

### SPEC-600

`covers` が存在しないVOを参照する場合（dangling reference）は `MISMATCH` とする。

### SPEC-601

Test IDを宣言するが `covers` をどのVOも参照しないTest（orphan test）は `MISMATCH` とする。

### SPEC-602

すべての管理対象Testに `covers ≥ 1` を一律要求する（§12）。

### SPEC-603

VOのparentが存在しない、または循環している場合は `MISMATCH` とする。

### SPEC-604

VOの `derives_from`（document参照）が存在しないdocumentを参照する場合は `MISMATCH` とする。

### SPEC-605

documentのderives_fromが存在しないdocumentを参照する場合（文書鎖のリンク切れ）は `MISMATCH` とする。

### SPEC-606

根に指定されない孤児document（`orphan_detection`）は `MISMATCH` とする。

### SPEC-607

Relationのfrom / toが存在しないエンティティを参照する場合は `MISMATCH` とする。

### SPEC-608

恒久SRC IDがadapter境界を越えて重複する場合は `MISMATCH` とする。

### SPEC-609

必須Test metadataの欠落は `MISMATCH` とする。

### SPEC-610

adapterがTestとして発見したが管理宣言を持たないconstruct（unregistered test）は診断severityをwarningとする。

### SPEC-611

managed Test Entityへ対応しない事実は `chain_integrity`（`MISMATCH`/MISSING）へ反映する（§12）。

### SPEC-612

エラーは検証結果に反映され、該当エンティティの検査を非 `PASS` にする。

### SPEC-613

診断severityと検証状態を混同しない。

### SPEC-614

content_hash照合は決定論的に解決する。

### SPEC-615

content_hash照合は任意形式の文書本文から参照位置の存在を構文的に推測しない。

### SPEC-616

参照位置の意味的妥当性・取り込み完全性は検査対象としない。

### SPEC-617

参照位置の意味的妥当性・取り込み完全性は必要ならエスカレーション（§11）で扱う。

### SPEC-618

プロジェクトルート直下に `.verify/` を置く。

### SPEC-619

`.verify/` にテストコード外の正典と事実レコードを保存する。

### SPEC-620

`.verify/config.yaml` は設定（正典）である。

### SPEC-621

`.verify/doc/` はdocumentレコード（正典）を格納する。

### SPEC-622

`.verify/vo/` はVOレコード（正典）を格納する。

### SPEC-623

`.verify/rel/` は外部Relationレコード（正典・不変）を格納する。

### SPEC-624

`.verify/forms/` はForm Schema（正典）を格納する。

### SPEC-625

`.verify/decisions/` は判断記録（事実・追記型）を格納する。

### SPEC-626

`.verify/approvals/` は承認記録（事実・追記型）を格納する。

### SPEC-627

`.verify/evidence/` は実行証拠レコード（事実・追記型）を格納する。

### SPEC-628

`.verify/cache/` は派生情報（Git管理外）を格納する。

### SPEC-629

ファイル形式はすべてYAMLとする。

### SPEC-630

`cache/` 以外はGit管理対象とする。

### SPEC-631

1レコード＝1ファイルとする。

> 多数の AI Agent が並列で Test を追加・変更する前提。

*導出元: REQ-271, REQ-272, REQ-273, REQ-274, REQ-275, REQ-276, REQ-277, REQ-278, REQ-279*

*引用: 要件定義 §23*

### SPEC-632

全員が編集する中央共有台帳を持たない。

### SPEC-633

document / VOは1エンティティ1ファイルとする。

### SPEC-634

document / VOのファイル名をIDとする。

### SPEC-635

異なるエンティティへの並列変更は異なるファイルへの変更になる。

### SPEC-636

Relation・判断・承認・Evidenceの各レコードはULIDをファイル名とする新規ファイル追加のみで作成する。

### SPEC-637

Relation・判断・承認・Evidenceの各レコードの作成は既存ファイルの編集を伴わない。

### SPEC-638

Relationレコードは不変とする。

### SPEC-639

Relationレコードの変更は「旧削除＋新追加」で表現する。

### SPEC-640

同一エンティティファイルへの並列変更が衝突した場合の解決はGitのマージに委ねる。

### SPEC-641

マージ後の論理的不整合（ID衝突、dangling reference、承認の失効）はスキャンと整合性検査で検出する（§23）。

### SPEC-642

record / エンティティファイルの書込みは原子的に公開する。

### SPEC-643

record / エンティティファイルの書込みは読み手に書きかけの部分状態を観測させない。

### SPEC-644

並列編集耐性は「公開されたファイルは常に完全である」ことを前提とする。

### SPEC-645

並列編集耐性では部分書込みの検出・修復は行わない。

### SPEC-646

Test ID衝突・dangling referenceの検出、派生indexの再構築、Testと関連情報の同期を人間/Agentの記憶だけに依存させないことは§23と§24.3で担保する。

### SPEC-647

具体的な物理保存方式は詳細設計へ委譲する。

*導出元: REQ-309, REQ-310, REQ-311, REQ-312, REQ-313, REQ-314, REQ-315, REQ-316, REQ-317, REQ-318, REQ-319, REQ-320, REQ-321, REQ-322, REQ-323, REQ-324, REQ-325, REQ-326, REQ-327, REQ-328, REQ-329, REQ-330, REQ-331, REQ-332*

*引用: 要件定義 §28*

### SPEC-648

検証グラフ、逆引きインデックス、集約結果はすべて正典からの導出物である。

### SPEC-649

検証グラフ、逆引きインデックス、集約結果は `vtest scan` によりいつでも再構築できる。

*引用: 要件定義 NFR-004*

### SPEC-650

`cache/` が破損・削除されても正典は影響を受けない。

### SPEC-651

キャッシュ / indexの具体的データ形式は詳細設計へ委譲する。

*導出元: REQ-309, REQ-310, REQ-311, REQ-312, REQ-313, REQ-314, REQ-315, REQ-316, REQ-317, REQ-318, REQ-319, REQ-320, REQ-321, REQ-322, REQ-323, REQ-324, REQ-325, REQ-326, REQ-327, REQ-328, REQ-329, REQ-330, REQ-331, REQ-332*

*引用: 要件定義 §28*

### SPEC-652

具体的role taxonomy・presetの固定は行わない。

### SPEC-653

役割別の参照観点は§19のprojectionとして提供する。

### SPEC-654

Coder AIはMCP経由で、担当したVO / Testをscopeに指定して検証する。

> 要件定義 §20 の利用者ごとに想定する主経路を示す。

### SPEC-655

Coder AIは自身の変更が要求された検証を満たしたか確認する。

### SPEC-656

DeveloperはCLIで、Structured Test Operationによるテスト作成・変更を行う。

### SPEC-657

Developerは検証結果の詳細表示を行う。

### SPEC-658

CIはCLI（非対話）で `vtest verify` を同一revisionで再実行し、終了コードで判定する。

### SPEC-659

CIはEvidenceを成果物として保存する。

### SPEC-660

Reviewer AIはMCP経由で、Coderが提出したEvidence・判断記録と、自身の再検証結果を照合する。

### SPEC-661

PM / PM AgentはCLIまたはMCPで、documentまたはVO単位の集約結果からNG箇所へ掘り下げる。

### SPEC-662

MCPを本体とする。

### SPEC-663

CLI・CIは同じ検証の別入口とする。

### SPEC-664

GUIは必須要件としない。

*導出元: REQ-267, REQ-268, REQ-269, REQ-270*

*引用: 要件定義 §22*

### SPEC-665

コマンドの完全仕様（引数・出力・終了コード）は詳細設計で定める。

### SPEC-666

本書ではコマンド一覧と責務を確定する。

### SPEC-667

`vtest init` の責務は `.verify/` の初期化とする。

### SPEC-668

`vtest scan` の責務はスキャンと整合性検査、派生情報の再構築とする。

### SPEC-669

`vtest doc add / list / show` の責務はdocumentレコードの管理（derives_from・根指定を含む）とする。

### SPEC-670

`vtest vo add / edit / list / show / expand / approve` の責務はVOレコードの管理、組合せの実体化とする。

### SPEC-671

`approve` は `vtest approval create` の別名とする。

### SPEC-672

`vtest approval create / withdraw / show` の責務は承認レコードの生成・取消・照会とする。

### SPEC-673

`vtest approval create / withdraw / show` は対象種別（VO・document・判断記録）を引数に取る、承認の唯一の正典面である。

### SPEC-674

`vtest test create / edit / show / list / query` の責務はStructured Test Operationとする。

### SPEC-675

`vtest audit static` の責務は決定論的解析（oracle_presenceの不成立検出）の実行とする。

### SPEC-676

`vtest audit bundle / submit` の責務は判断記録（§11）のbundle生成と結果提出とする。

### SPEC-677

`vtest run` の責務はテスト実行とEvidence記録とする。

### SPEC-678

`vtest verify` の責務は検証の実行（scope指定可）とOK / NG判定とする。

### SPEC-679

`vtest report` の責務は詳細レポート出力（ツリー／JSON）とする。

### SPEC-680

`vtest doctor` の責務は整合性検査のみの実行とする。

### SPEC-681

終了コードは `0`＝要求scopeがOK、`1`＝検証NG、`2`＝入力・adapter前提・capability等による操作拒否、`3`＝内部エラーとする。

### SPEC-682

フェーズゲートを指定した実行（§20）では、`0` / `1` は当該ゲートの充足・不充足を表す。

### SPEC-683

ゲート充足は検証状態とは別軸の評価である。

### SPEC-684

ゲート充足は検証状態を書き換えない。

### SPEC-685

出力では検証状態とゲート満否を別に提示する。

### SPEC-686

ゲート指定時の `0` を検証状態 `PASS` と読ませない。

### SPEC-687

検証状態と内部エラーは終了コードで分離する（§4.4）。

### SPEC-688

CIはこの終了コードのみで判定できる。

### SPEC-689

終了コード体系の詳細は詳細設計へ委譲する。

*導出元: REQ-105, REQ-106, REQ-107, REQ-108, REQ-309, REQ-310, REQ-311, REQ-312, REQ-313, REQ-314, REQ-315, REQ-316, REQ-317, REQ-318, REQ-319, REQ-320, REQ-321, REQ-322, REQ-323, REQ-324, REQ-325, REQ-326, REQ-327, REQ-328, REQ-329, REQ-330, REQ-331, REQ-332*

*引用: 要件定義 §5.4、§28*

### SPEC-690

MCPサーバは `vtest mcp` として起動する。

### SPEC-691

MCPサーバはCLIと同一のコア機能を呼び出す。

### SPEC-692

ツールの完全な入出力スキーマは詳細設計で定める。

### SPEC-693

`scan` の対応機能はスキャンと整合性検査とする。

### SPEC-694

`doc_list` / `doc_get` / `doc_upsert` の対応機能はdocument管理とする。

### SPEC-695

`vo_list` / `vo_get` / `vo_upsert` / `vo_expand` / `vo_approve` の対応機能はVO管理とする。

### SPEC-696

`vo_approve` は `approval_create` の別名とする。

### SPEC-697

`approval_create` / `approval_withdraw` / `approval_get` の対応機能は承認レコードの生成・取消・照会とする。

### SPEC-698

`approval_create` / `approval_withdraw` / `approval_get` は対象種別を引数に取る、承認の唯一の正典面である。

### SPEC-699

`test_query` / `test_get` の対応機能はTest検索・逆引きとする。

### SPEC-700

`test_create` / `test_edit` の対応機能はStructured Test Operationとする。

### SPEC-701

`form_get` の対応機能はForm Schemaの取得とする。

### SPEC-702

`audit_static` の対応機能は決定論的解析とする。

### SPEC-703

`audit_bundle` / `audit_submit` の対応機能は判断記録プロトコルとする。

### SPEC-704

`run_tests` の対応機能はテスト実行とする。

### SPEC-705

`verify` の対応機能は検証実行とする。

### SPEC-706

`report` の対応機能は詳細レポート取得とする。

### SPEC-707

すべてのツールは非対話で完結する。

*引用: 要件定義 NFR-007*

### SPEC-708

CLIとMCPは同じadapter registry composition・JSON envelope・adapter選択エラーを利用する。

### SPEC-709

MCPがCLIと異なるadapterを暗黙選択してはならない。

### SPEC-710

CLI command体系・MCP tool体系の詳細は詳細設計へ委譲する。

*導出元: REQ-309, REQ-310, REQ-311, REQ-312, REQ-313, REQ-314, REQ-315, REQ-316, REQ-317, REQ-318, REQ-319, REQ-320, REQ-321, REQ-322, REQ-323, REQ-324, REQ-325, REQ-326, REQ-327, REQ-328, REQ-329, REQ-330, REQ-331, REQ-332*

*引用: 要件定義 §28*

### SPEC-711

検証契約・ID・ハッシュ・Evidence・状態・集約の概念モデルは、言語およびtest runnerに依存しない。

*導出元: R-3, REQ-258, REQ-259, REQ-260, REQ-261, REQ-262, REQ-263, REQ-264, REQ-265, REQ-266*

*引用: 要件定義 §21、R-3*

### SPEC-712

source discovery、決定論的解析、Structured Test Operation、test runner起動、coverage計測はadapter能力として提供する。

### SPEC-713

共通契約は特定言語の構文・構造を必須としない。

### SPEC-714

core verifierを変更せずに別adapterを登録できる境界を要求する。

### SPEC-715

adapter追加によって共通契約・スキーマが壊れないことを設計制約とする。

### SPEC-716

組込production adapterは `rust-cargo` とする。

### SPEC-717

`rust-cargo` はRust・Rust function unit test・小規模なintegration testを対象とする。

### SPEC-718

`rust-cargo` 以外のproduction language adapterはv0.1の提供範囲に含めない。

*導出元: R-2*

*引用: 要件定義 R-2*

### SPEC-719

adapterが未登録・能力不足・解析不能の場合、検証結果を推測で `PASS` へ昇格してはならない。

### SPEC-720

能力不足で確認できない項目は§8条項4に従い扱う。

### SPEC-721

NFR-001並列性への対応は、1レコード1ファイル、ULIDファイル名、不変Relation、中央台帳の不在とする（§24.2）。

*導出元: REQ-280, REQ-281, REQ-282, REQ-283, REQ-284, REQ-285, REQ-286, REQ-287*

*引用: 要件定義 §24*

### SPEC-722

NFR-002再現性への対応は、Evidenceのリビジョン束縛、決定論的解析の再実行可能性、scanによる全再構築とする（§21）。

### SPEC-723

NFR-003追跡可能性への対応は、document→VO→Test→SRC→Evidenceの双方向グラフ、任意ノードからの局所／経路／全体取得とする（§19、§23）。

### SPEC-724

NFR-004再構築可能性への対応は、派生情報はcacheのみとし、正典から `vtest scan` で再構築することとする（§24.3）。

### SPEC-725

NFR-005 Fail Closedへの対応は、状態モデルと集約規則（§4、§22）、承認・判断の内容ハッシュ束縛（§11、§17）とする。

### SPEC-726

NFR-006説明可能性への対応は、状態・診断ラベルの分離（§4）、根拠を辿れる詳細レポート（§22.3）とする。

### SPEC-727

NFR-007自動化適性への対応は、非対話CLI・MCP、JSON出力、終了コードとする（§26）。

### SPEC-728

NFR-008人間可読性への対応は、ツリー形式の詳細出力、IDの人間可読性とする（§3.2、§22.3）。

### SPEC-729

要件定義 §25 のスコープ外事項に対応する機能を本書では定義しない。

*導出元: REQ-288, REQ-289, REQ-290, REQ-291, REQ-292, REQ-293, REQ-294*

*引用: 要件定義 §25*

### SPEC-730

文書層は§2.2の通りリンクとハッシュのみを扱う（OOS-001仕様書同士の品質監査）。

### SPEC-731

文書内容の意味的良否を検証しない（OOS-001仕様書同士の品質監査）。

### SPEC-732

不一致はどれを正とするか決めず状態として提示する（OOS-002修正方針決定。§4）。

*導出元: P-001*

*引用: P-001*

### SPEC-733

Test Edit対象外の一般編集を管理しない（OOS-003通常ソースコード編集管理。§15.3）。

### SPEC-734

フェーズのライフサイクル管理・工程遷移は責務外とする（OOS-004開発プロセス全体の管理。§20）。

### SPEC-735

本システムはVerification Infrastructureとして機能する。

### SPEC-736

v0.1は宣言された義務の裏付けのみ検証する（OOS-005宣言されていない実装）。

### SPEC-737

v0.1は宣言されていない実装の存在を関知しない（OOS-005宣言されていない実装）。

*導出元: R-2*

*引用: R-2*

### SPEC-738

実装レイヤーの孤児検出・シンボル列挙の定義・上流文書の意味構造はv0.2のスコープとする。

### SPEC-739

READMEに非関知宣言を一行入れる。

### SPEC-740

以下は本書の要求・要件を基に詳細設計で決定する（要件定義 §28 の23項目に対応）。

*導出元: REQ-309, REQ-310, REQ-311, REQ-312, REQ-313, REQ-314, REQ-315, REQ-316, REQ-317, REQ-318, REQ-319, REQ-320, REQ-321, REQ-322, REQ-323, REQ-324, REQ-325, REQ-326, REQ-327, REQ-328, REQ-329, REQ-330, REQ-331, REQ-332*

*引用: 要件定義 §28*

### SPEC-741

HOWは本書で発明しない。

### SPEC-742

詳細設計は、文書の具体的な入力フォーマットと登録方式を決定する（§16）。

### SPEC-743

詳細設計は、文書層の根の指定方式（orphan_detectionの除外指定。§5.2）を決定する。

### SPEC-744

詳細設計は、VO保存形式を決定する（§10、§24.1）。

### SPEC-745

詳細設計は、Test metadataの具体的annotation syntax（`rust-cargo` の `@vtest.*` 文法を含む。§22）を決定する。

### SPEC-746

詳細設計は、relationの保存形式を決定する（§3.2、§24.2）。

### SPEC-747

詳細設計は、Test ID命名規則を決定する（§3.2）。

### SPEC-748

詳細設計は、Target Reference / SRC IDの具体的識別方式を決定する（§9.2）。

### SPEC-749

詳細設計は、AST / LSP等の具体的解析技術（不成立証明・存在確認・静的到達の実装。§5.5、§8.3）を決定する。

### SPEC-750

詳細設計は、`oracle_presence` の信頼基盤の具体的範囲と委譲確認の方法を決定する（§8.2）。

### SPEC-751

詳細設計は、`target_binding` の動的計測方式を決定する（§5.3）。

### SPEC-752

詳細設計は、診断ラベルの語彙を決定する（§4.2）。

### SPEC-753

詳細設計は、終了コード体系（検証状態と内部エラーの分離。§26.1）を決定する。

### SPEC-754

詳細設計は、エスカレーション出力・判断記録・承認記録の具体的schemaを決定する（§11、§17）。

### SPEC-755

詳細設計は、CLI command体系を決定する（§26.1）。

### SPEC-756

詳細設計は、MCP tool体系を決定する（§26.2）。

### SPEC-757

詳細設計は、キャッシュ / indexの具体的データ形式を決定する（§24.3）。

### SPEC-758

詳細設計は、並列編集時の物理的保存方式を決定する（§24.2）。

### SPEC-759

詳細設計は、承認workflowの具体的状態遷移を決定する（§17）。

### SPEC-760

詳細設計は、判断待ち情報（§18.3）の具体的な構造schemaと取得インターフェースを決定する。

### SPEC-761

詳細設計は、関係リンクの任意説明（§19）の保存形式を決定する。

### SPEC-762

詳細設計は、役割別projection / view（§19）のpreset・UI・モード体系を決定する。

### SPEC-763

詳細設計は、approval authority（§17）の承認ロール・必要承認数・権限schemaを決定する。

### SPEC-764

詳細設計は、フェーズ・ゲート（§20）の具体的なフェーズ名と進行条件定義を決定する。

### SPEC-765

本書の要求・要件を基に詳細設計で決定するHOWを本書で確定しない。
