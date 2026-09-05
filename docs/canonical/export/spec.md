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

## SPEC-S002 1. 用語定義

*導出元: REQ-S001, REQ-S003, REQ-S013, REQ-S020, REQ-S025, REQ-S029, REQ-S035, REQ-S036, REQ-S037, REQ-S046*

### SPEC-012

documentは要件定義書・基本仕様書・詳細設計書・API Schema・Protocol Specification・型/データ仕様・DB schema・その他の機械可読仕様を含む。

### SPEC-013

derives_fromは上流documentから下流documentへの導出を表す。

### SPEC-014

Verification Obligation（VO）とは、独立して「この条件が成立するか」と検証可能な仕様上の命題である。

*導出元: REQ-162, REQ-163*

*引用: 要件定義 §10.1*

### SPEC-015

VOは1件以上のdocumentからderives_fromで導出される。

### SPEC-016

VOは階層構造を持てる。

### SPEC-017

Testとは、登録adapterが実行可能な検証単位として識別し、Test IDで管理するtest constructである。

### SPEC-018

TestはVOの検証実装単位であり、VOとN:Mの対応を持ちうる。

### SPEC-019

Testは `covers` 宣言でVOを参照する。

### SPEC-020

Test Intentとは、Testが「何を検証するか」を実装コードを読まずに判断できる形で表した付随情報である。

### SPEC-021

検証対象とは、その Test が検証成立性（§8）を証明しようとする対象、すなわち宣言された「何の時にどうなる」の主語である。

### SPEC-022

検証対象は実装constructに限定せず、外部から観測可能な契約・境界上の振る舞いも含む。

*導出元: REQ-144, REQ-145, REQ-146, REQ-147, REQ-148*

*引用: 要件定義 §9.1*

### SPEC-023

Source Target（SRC）とは、実装コード上の識別可能なimplementation constructである。

### SPEC-024

Execution Evidenceとは、テスト実行の事実の記録である。

### SPEC-025

判断記録（judgment record）とは、`UNKNOWN` に対して外部（人間または判断可能Agent）が下した判断の記録である。

### SPEC-026

承認記録（approval record）とは、判断または方針を「この内容で進める」と正式に認めた記録である。

### SPEC-027

承認記録は判断記録とは別軸・別entityでありうる（§17）。

*導出元: REQ-236, REQ-237, REQ-238, REQ-239, REQ-240, REQ-241, REQ-242, REQ-243, REQ-244, REQ-245, REQ-246, REQ-247, REQ-248, REQ-249, REQ-250, REQ-251, REQ-252*

*引用: 要件定義 §19*

### SPEC-028

診断ラベルとは、検証状態に付随して原因を説明するラベル（`MISSING` / `NOT_EXECUTED` / `NOT_CHECKED` / `STALE` 等）である。

### SPEC-029

診断ラベルの語彙は詳細設計で定める。

*導出元: REQ-092, REQ-093, REQ-309, REQ-310, REQ-311, REQ-312, REQ-313, REQ-314, REQ-315, REQ-316, REQ-317, REQ-318, REQ-319, REQ-320, REQ-321, REQ-322, REQ-323, REQ-324, REQ-325, REQ-326, REQ-327, REQ-328, REQ-329, REQ-330, REQ-331, REQ-332*

*引用: 要件定義 §5.2、§28*

### SPEC-030

scopeとは、利用者が限定する検査・エンティティの範囲である。

### SPEC-031

Agent Form Engineeringとは、既知の作業手順・入力項目を持つ操作を、自由編集ではなく構造化された質問・入力・検証で行わせる方式である。

*導出元: P-004*

*引用: 要件定義 P-004*

## SPEC-S003 2 全体像

### SPEC-S004 2.1 正典の三層構造

*導出元: P-003, REQ-S019, REQ-S035, REQ-S046*

### SPEC-032

本システムの仕事は「宣言と実装が一致しているか」「事実が現在の宣言・実装に対して有効か」を照合することに限る。

### SPEC-S005 2.2 宣言鎖と照合

*導出元: REQ-S004, REQ-S005*

### SPEC-033

要件定義 §3.2 の宣言鎖をそのまま採用する。

*導出元: REQ-024, REQ-025, REQ-026, REQ-027, REQ-028, REQ-029, REQ-030, REQ-031, REQ-032, REQ-033*

*引用: 要件定義 §3.2*

### SPEC-034

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

### SPEC-035

VOは1件以上の `document` からderives_fromで導出される。

### SPEC-036

不一致は状態（§4）として提示する。

*導出元: P-001*

*引用: 要件定義 P-001*

### SPEC-S006 2.3 導出できる関係は保存しない

*導出元: P-003*

### SPEC-037

Test→VO（`covers`）、Test→SRC（`targets`）の関係はadapter所有のTest metadata宣言から決定論的に導出できる。

## SPEC-S007 3 エンティティと ID 体系

### SPEC-S008 3.1 エンティティ種別

*導出元: REQ-S005, REQ-S025, REQ-S035, REQ-S036, REQ-S040, REQ-S046*

### SPEC-038

Verification Obligationは検証命題である。

### SPEC-039

Verification Obligationは階層を持てる。

### SPEC-040

Testはadapterが識別する実行可能なtest constructである。

### SPEC-041

Source Targetは対象implementation constructである。

### SPEC-042

文書層の段（要件→仕様→詳細設計…）はderives_fromリンクとして表現する。

### SPEC-S009 3.2 ID 規則と関係リンク

*導出元: REQ-S007, REQ-S027, REQ-S058*

### SPEC-043

DOC / VO / TESTのIDは利用者（人間またはAI）が命名する。

### SPEC-044

関係リンクの任意説明文・役割別projectionの保存形式・presetは詳細設計へ委譲する。

*導出元: REQ-309, REQ-310, REQ-311, REQ-312, REQ-313, REQ-314, REQ-315, REQ-316, REQ-317, REQ-318, REQ-319, REQ-320, REQ-321, REQ-322, REQ-323, REQ-324, REQ-325, REQ-326, REQ-327, REQ-328, REQ-329, REQ-330, REQ-331, REQ-332*

*引用: 要件定義 §28*

### SPEC-S010 3.3 Source Target の識別

*導出元: R-3, REQ-S027*

### SPEC-045

対象はTarget Referenceで識別する。

## SPEC-S011 4 検証状態と診断ラベル

### SPEC-S012 4.1 状態は 5 つ

*導出元: REQ-S014*

### SPEC-046

状態の存在資格は「受け取った者の行動が変わるか」である。

### SPEC-S013 4.2 診断ラベル

*導出元: REQ-S015, REQ-S058*

### SPEC-047

`MISSING` / `NOT_EXECUTED` / `NOT_CHECKED` / `STALE` 等は、状態に付随して原因を説明する診断ラベルである。

### SPEC-048

診断ラベルの語彙は詳細設計で定める。

*導出元: REQ-092, REQ-093, REQ-309, REQ-310, REQ-311, REQ-312, REQ-313, REQ-314, REQ-315, REQ-316, REQ-317, REQ-318, REQ-319, REQ-320, REQ-321, REQ-322, REQ-323, REQ-324, REQ-325, REQ-326, REQ-327, REQ-328, REQ-329, REQ-330, REQ-331, REQ-332*

*引用: 要件定義 §5.2、§28*

### SPEC-S014 4.3 状態の割当

*導出元: REQ-S016*

### SPEC-049

要件定義 §5.3 の割当をそのまま採用する。

*導出元: REQ-094, REQ-095, REQ-096, REQ-097, REQ-098, REQ-099, REQ-100, REQ-101, REQ-102, REQ-103, REQ-104*

*引用: 要件定義 §5.3*

### SPEC-S015 4.4 UNKNOWN の検疫

*導出元: REQ-S017*

### SPEC-050

`UNKNOWN` はエラーではなく正常動作としての降参である。

### SPEC-S016 4.5 検証状態と承認の分離

*導出元: REQ-S018*

### SPEC-051

承認（§17）は独立した別軸である。

*導出元: REQ-109, REQ-110, REQ-111, REQ-112, REQ-113, REQ-114*

*引用: 要件定義 §5.5*

### SPEC-052

フェーズ進行に承認を要するかは、検証状態と承認の組合せとして §21 のゲート条件で扱う。

## SPEC-S017 5. 検査

### SPEC-053

検証は `chain_integrity` / `orphan_detection` / `target_binding` / `oracle_presence` の4検査のみで行う。

### SPEC-054

各検査は一つの問いを持つ。

### SPEC-055

各検査は複数の証拠源で答えてよい。

### SPEC-056

網羅・意味の疑義はエスカレーション（§11）の領分である。

*導出元: REQ-164, REQ-165, REQ-166, REQ-174, REQ-175, REQ-176, REQ-177, REQ-178, REQ-179, REQ-180, REQ-181, REQ-182, REQ-183, REQ-184, REQ-185, REQ-186, REQ-187, REQ-188, REQ-189, REQ-190, REQ-191, REQ-192*

*引用: 要件定義 §10.2、§11、§12*

### SPEC-S018 5.1 chain_integrity — 宣言鎖の完全性

*導出元: REQ-S009, REQ-S036*

### SPEC-057

chain_integrityの問いは、宣言鎖のすべてのリンクが存在し、ハッシュ照合が成立するかである。

### SPEC-058

どのリンクで切れたかは診断ラベルで示す。

### SPEC-S019 5.2 orphan_detection — 文書層の孤児検出

*導出元: REQ-S010*

### SPEC-059

orphan_detectionの問いは、親を持たない `document` ノードが存在するかである。

### SPEC-060

根の指定の具体構文は詳細設計へ委譲する。

*導出元: REQ-309, REQ-310, REQ-311, REQ-312, REQ-313, REQ-314, REQ-315, REQ-316, REQ-317, REQ-318, REQ-319, REQ-320, REQ-321, REQ-322, REQ-323, REQ-324, REQ-325, REQ-326, REQ-327, REQ-328, REQ-329, REQ-330, REQ-331, REQ-332*

*引用: 要件定義 §28*

### SPEC-S020 5.3 target_binding — 宣言対象の振る舞いの実現

*導出元: REQ-S011, REQ-S020, REQ-S054*

### SPEC-061

target_bindingの問いは、そのTestが検証対象とする振る舞いが実際に生じ、その振る舞いを反映した観測が得られたかである。

### SPEC-062

テストランナーの `PASS`/`FAIL` は判定権威（§7）の証拠として消費する。

### SPEC-063

target_binding検査は、その証拠が検証対象の実行を伴ったかを問う。

### SPEC-064

target_bindingは一つの問いに対し静的解析と動的計測の2つの証拠源を持つ。

### SPEC-065

他の実行形態における確認方法は、当該形態に適した方法として詳細設計で定める。

### SPEC-S021 5.4 oracle_presence — 照合装置の存在

*導出元: REQ-S012, REQ-S021*

### SPEC-066

oracle_presenceの問いは、宣言された「何の時にどうなる」の不成立を、Testの非成功として反映する装置が存在するかである。

### SPEC-067

静的解析の役割は不成立の証明である。

### SPEC-068

静的解析は成立の証明装置ではない。

### SPEC-069

意味の疑義は検査ではなくエスカレーション（§11）の領分である。

## SPEC-S022 7. 判定権威

*導出元: REQ-S020, REQ-S054*

### SPEC-070

テスト合否の判定権威は、当該adapterのテストランナーにある。

*導出元: REQ-121, REQ-122, REQ-123, REQ-124*

*引用: 要件定義 §7*

### SPEC-071

本システムは合否を判定しない。

### SPEC-072

本システムはテストランナーの結果を証拠として消費する。

### SPEC-073

実行の起動は本システムから行ってよい。

### SPEC-074

実行の起動を本システムから行っても、判定はしない。

### SPEC-075

`vtest` は照合（宣言・実装・証拠の一致検査＝§5の4検査）を行う。

### SPEC-076

`vtest` はテストの合否そのものを再判定しない。

### SPEC-077

`target_binding`（§5.3）はランナーの `PASS` を前提に、その `PASS` が検証対象の実行を伴ったかを問う独立の照合である。

## SPEC-S023 8 Test の検証成立性

*導出元: REQ-S021, REQ-S025*

### SPEC-S024 8.2 成立性の必要条件

### SPEC-078

`oracle_presence` の信頼基盤の具体的範囲（標準assert構文・framework failure semantics・設定による列挙）と委譲確認の方法は詳細設計へ委譲する。

*導出元: REQ-309, REQ-310, REQ-311, REQ-312, REQ-313, REQ-314, REQ-315, REQ-316, REQ-317, REQ-318, REQ-319, REQ-320, REQ-321, REQ-322, REQ-323, REQ-324, REQ-325, REQ-326, REQ-327, REQ-328, REQ-329, REQ-330, REQ-331, REQ-332*

*引用: 要件定義 §28*

### SPEC-S025 8.3 決定論的に検出可能な不成立構造

### SPEC-079

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

### SPEC-080

code fragmentはRustによる例示である。

### SPEC-081

`static_audit` に相当する判定は §5.4/§5.5 の `oracle_presence` として現れる。

## SPEC-S026 9 検証対象と Source Target

*導出元: REQ-S025*

### SPEC-S027 9.1 検証対象

### SPEC-082

検証対象は、そのTestが検証成立性（§8）を証明しようとする対象、すなわち宣言された「何の時にどうなる」の主語である。

### SPEC-083

検証対象は実装constructに限定しない。

### SPEC-084

外部から観測可能な契約・境界上の振る舞いも検証対象にできる。

*導出元: REQ-144, REQ-145, REQ-146, REQ-147, REQ-148*

*引用: 要件定義 §9.1*

### SPEC-S028 9.2 Source Target の識別

### SPEC-085

1つのTestは1件以上のSource Targetを宣言できる。

### SPEC-086

具体的構文・namespace・symbol種別は詳細設計へ委譲する。

### SPEC-S029 9.3 実装 traceability

### SPEC-087

検証対象とは別に、Testまたは検証対象から関連するSource Targetへのtraceabilityを保持できる。

### SPEC-088

Testまたは検証対象から関連するSource Targetへのtraceabilityは任意であり、影響分析・逆引きに利用できる。

### SPEC-089

traceabilityは関連付けであって実装対応の証明ではない。

### SPEC-090

Source Targetとの関係を持つTestについて、TestからSourceを検索できる。

### SPEC-091

Source Targetとの関係を持つTestについて、Sourceから関連Testを逆引きできる。

## SPEC-S030 10. Verification Obligation

*導出元: REQ-S029, REQ-S034*

### SPEC-092

VOは独立して「この条件が成立するか」と検証可能な仕様上の命題とする。

### SPEC-093

VOの粒度をassert文・test function・テストファイルなどのコード構文で決めない。

*導出元: REQ-162, REQ-163*

*引用: 要件定義 §10.1*

### SPEC-094

仕様は、テストで十分な網羅性を確認できる単位までVOへ分解できる。

### SPEC-095

本システムは分解を表現・保持するデータモデルを提供する。

### SPEC-096

分解が十分かの判定は本システムの検査ではなくエスカレーション（§11）の領分である。

*導出元: REQ-164, REQ-165, REQ-166*

*引用: 要件定義 §10.2*

### SPEC-097

VOは階層構造を持てる。

### SPEC-098

flatなVO群と階層化VO群の双方を扱う。

### SPEC-099

flatなVOを再帰分解・階層化する操作を提供する。

*導出元: REQ-167, REQ-168, REQ-169*

*引用: 要件定義 §10.3*

### SPEC-100

TestはVOの検証実装単位でありVOそのものではない。

*導出元: REQ-170, REQ-171, REQ-172, REQ-173*

*引用: 要件定義 §10.4*

### SPEC-101

VOには検証軸（dimension）と同値/境界値partitionを定義できる。

### SPEC-102

検証軸とpartitionの定義はすべてのVOへは要求しない。

### SPEC-103

何をもって十分とするかの判定は本システムの検査ではない（→§11）。

### SPEC-104

各観点の独立検証、または必要と定義された組合せ空間の検証のいずれかを表現・確認できる。

*導出元: REQ-174, REQ-175, REQ-176, REQ-177, REQ-178, REQ-179*

*引用: 要件定義 §11*

### SPEC-105

partition・組合せcoverage方針の具体的保存形式・語彙は詳細設計へ委譲する。

*導出元: REQ-309, REQ-310, REQ-311, REQ-312, REQ-313, REQ-314, REQ-315, REQ-316, REQ-317, REQ-318, REQ-319, REQ-320, REQ-321, REQ-322, REQ-323, REQ-324, REQ-325, REQ-326, REQ-327, REQ-328, REQ-329, REQ-330, REQ-331, REQ-332*

*引用: 要件定義 §28*

## SPEC-S031 11. 発見・意味判定のエスカレーションと判断記録

*導出元: REQ-S035, REQ-S046*

### SPEC-106

本システムは、宣言されていない義務・網羅漏れ・宣言と実装の意味のずれを、自ら発見・裁定しない。

### SPEC-107

発見者・裁定者は外部（人間またはAgent）である。

*導出元: REQ-180, REQ-181, REQ-182, REQ-183, REQ-184, REQ-185, REQ-186, REQ-187, REQ-188, REQ-189, REQ-190, REQ-191, REQ-192*

*引用: 要件定義 §12*

### SPEC-108

本システムの責務は、データ形態の提供（§11.1）、エスカレーション（§11.2）、判断の記録と再検証（§11.3）の3つに限る。

### SPEC-S032 11.1 データ形態の提供

### SPEC-109

外部の発見者が判断できる構造化出力（要求該当箇所と対応概念のペア、宣言鎖と検査結果、対象外とした範囲）を提供する。

### SPEC-110

`vtest` 自身はLLM APIを呼ばない。

### SPEC-111

`vtest` は意味判定・候補生成を検証成立条件にしない。

*導出元: REQ-224, REQ-225, REQ-226, REQ-227*

*引用: 要件定義 §17.2*

### SPEC-112

外部AI/Agentによる補助・提案は許容する。

### SPEC-113

外部AI/Agentの能力を成立条件にしない。

### SPEC-S033 11.2 エスカレーション

### SPEC-114

機械が決定論で確定できない疑義は `UNKNOWN` として、意味判定できる者へ引き渡す。

### SPEC-115

`UNKNOWN` は正常動作としての降参である。

### SPEC-S034 11.3 判断の記録と再検証

### SPEC-116

`UNKNOWN` に対して外部（人間または判断可能Agent）が判断できる。

### SPEC-117

判断記録は追跡可能とする。

### SPEC-118

判断記録の理由・根拠・evidence noteは保存できる構造とする。

### SPEC-119

外部の人間/Agentが判断し、判断結果（decision＋任意の理由）を提出する。

### SPEC-120

判断は承認なしでも記録できる。

### SPEC-121

正式採用は§17の別段階である。

### SPEC-122

エスカレーション出力・判断記録の具体的schema、判断待ち情報（§18.3）の構造schemaと取得インターフェース、判断の多重度は詳細設計へ委譲する。

*導出元: REQ-309, REQ-310, REQ-311, REQ-312, REQ-313, REQ-314, REQ-315, REQ-316, REQ-317, REQ-318, REQ-319, REQ-320, REQ-321, REQ-322, REQ-323, REQ-324, REQ-325, REQ-326, REQ-327, REQ-328, REQ-329, REQ-330, REQ-331, REQ-332*

*引用: 要件定義 §28*

## SPEC-S035 12. Test Registry

*導出元: REQ-S009, REQ-S036*

### SPEC-123

各Testは安定したTest IDによって識別可能とする。

### SPEC-124

Test IDをハンドルとして、Test Intent・`covers`（VO参照）・検証対象・Source Target・Location・判断記録・Execution Evidenceを検索可能とする。

*導出元: REQ-193, REQ-194, REQ-195, REQ-196, REQ-197, REQ-198, REQ-199, REQ-200, REQ-201, REQ-202*

*引用: 要件定義 §13*

## SPEC-S036 13. Test Intent

*導出元: REQ-S037*

### SPEC-125

Testには、その実装コードだけを読まなくても、何を検証するか・どのVOに対応するか・何を入力条件とするか・何を期待するかを判断できる情報を関連付けられる。

### SPEC-126

Test IntentはTest Entityの付随情報であり、宣言鎖のノードではない。

*導出元: REQ-203, REQ-204, REQ-205*

*引用: 要件定義 §14*

### SPEC-127

具体的入力値をTest IntentまたはVOへ含めることを許容する。

### SPEC-128

具体的入力値をTest IntentまたはVOへ含めることは必須としない。

## SPEC-S037 14. Parameterized / Table-Driven Test

*導出元: R-3, REQ-S038*

### SPEC-129

table-drivenの論理形式を正式に許容する。

### SPEC-130

adapterが識別したtable-driven test construct全体を一つのTestとして登録できる。

### SPEC-131

内部の各caseを独立Test IDへ分解することを必須としない。

### SPEC-132

cases集合がVOに必要な入力空間を十分に代表・網羅しているかは§11の発見・判定の対象とする。

## SPEC-S038 15. Structured Test Operation

*導出元: P-004, REQ-S039*

### SPEC-133

Test操作の公式経路として、Test IDまたはadapterが識別可能なTest constructを対象とした構造化操作を提供する。

*導出元: P-004, REQ-210, REQ-211, REQ-212, REQ-213, REQ-214, REQ-215, REQ-216, REQ-217*

*引用: 要件定義 §16、P-004*

### SPEC-134

Create Testは、Form Schemaに基づく構造化入力をadapterへ渡し、Test constructと対応するmetadata宣言を生成する。

### SPEC-135

Edit Testは、Test IDを編集ハンドルとして、adapterが識別する対象Testのmetadata宣言およびTest constructを更新する。

### SPEC-136

Query Testは、Test ID・VO・Target Reference等からの検索と逆引きを行う。

### SPEC-137

Audit（判断）Testは、§11の判断記録bundle生成と判断結果の提出を行う。

### SPEC-S039 15.1 desired state 方式

### SPEC-138

Create / Editの入力は差分操作ではなくあるべき状態（desired state）とする。

### SPEC-139

利用者は「TEST-Xはこの状態である」を宣言する。

### SPEC-S040 15.3 編集境界

### SPEC-140

Test外部の通常ソースコード・helper・fixtureの編集は責務外とする。

### SPEC-141

Test外部の通常ソースコード・helper・fixtureの編集操作は提供しない。

*導出元: REQ-210, REQ-211, REQ-212, REQ-213, REQ-214, REQ-215, REQ-216, REQ-217, REQ-288, REQ-289, REQ-290, REQ-291, REQ-292, REQ-293, REQ-294*

*引用: 要件定義 §16、§25 OOS-003*

### SPEC-142

通常のwrite/editツールや人間による直接ソース編集は完全禁止しない。

### SPEC-143

公式経路の提供により誤編集・更新忘れ・複数Test同時変更の事故を低減する。

### SPEC-144

直接編集による不整合も検証（§5.1）で検出可能とする。

### SPEC-S041 15.4 Form Schema

### SPEC-145

Rust関数単体Test用と小規模結合Test用の組込schemaを同梱する。

### SPEC-146

CLI・MCPのいずれからも同一schemaを消化できる。

## SPEC-S042 16. 仕様入力（文書層）

*導出元: REQ-S045*

### SPEC-147

仕様ソースとして、ソースコードより上流に位置する成果物（要件定義・基本仕様・詳細設計・API Schema・Protocol Specification・型/データ仕様・DB schema・その他の機械可読仕様）を利用可能とする。

### SPEC-148

取り込まれた上流成果物は §2.2 の `document` ノードとして登録される。

### SPEC-149

文書の具体的入力フォーマットと登録方式、根の指定方式は詳細設計へ委譲する。

*導出元: REQ-309, REQ-310, REQ-311, REQ-312, REQ-313, REQ-314, REQ-315, REQ-316, REQ-317, REQ-318, REQ-319, REQ-320, REQ-321, REQ-322, REQ-323, REQ-324, REQ-325, REQ-326, REQ-327, REQ-328, REQ-329, REQ-330, REQ-331, REQ-332*

*引用: 要件定義 §28*

## SPEC-S043 17. 承認

*導出元: REQ-S018, REQ-S046*

### SPEC-150

承認とは、判断（§11の判断記録を含む）または方針を「この内容で進める」と正式に認め確定状態にすることである。

### SPEC-151

VO等の検証成果物について確定・承認状態を表現可能とする。

### SPEC-152

承認は対象または参照する判断（judgment reference）に承認済み状態を与える。

### SPEC-153

Agentも承認権限を持ち得る（Human / Verification Agent / Reviewer Agent / PM Agent等）。

### SPEC-154

誰がどの対象・範囲を承認できるか（approval authority）はプロジェクト側で定義可能とする。

### SPEC-155

具体的な承認ロール・必要承認数・権限schema・承認workflowの状態遷移は詳細設計へ委譲する。

*導出元: REQ-309, REQ-310, REQ-311, REQ-312, REQ-313, REQ-314, REQ-315, REQ-316, REQ-317, REQ-318, REQ-319, REQ-320, REQ-321, REQ-322, REQ-323, REQ-324, REQ-325, REQ-326, REQ-327, REQ-328, REQ-329, REQ-330, REQ-331, REQ-332*

*引用: 要件定義 §28*

## SPEC-S044 18. 途中導入と既存プロジェクト対応

*導出元: R-5, REQ-S040*

### SPEC-156

本システムはプロジェクト開始時からの導入を前提としない。

### SPEC-157

開発途中または既存プロジェクトへ後から導入できる。

*導出元: R-5*

*引用: 要件定義 R-5、§17*

### SPEC-S045 18.1 既存資産の可視化

### SPEC-158

既に大量のソースコードとTestが存在するプロジェクトを検証対象として扱える。

### SPEC-159

既存の文書・Source・Testを読み取り、VOの存在状況・既存TestとVOの対応・Testの不足・検証成立性・宣言との不一致を可視化する。

### SPEC-160

VOが確定していない範囲を含むプロジェクトも読み取れる。

### SPEC-161

`vtest init` は `.verify/` を作成する。

### SPEC-162

`vtest scan` は発見した未登録Testを未登録として報告する。

### SPEC-163

document / VO、Test metadata宣言、判断記録、Evidenceの一部が欠ける状態も読み取り可能とする。

### SPEC-S046 18.2 導入時の責務境界

### SPEC-164

決定論的に処理可能な作業について人間の反復手入力を必須としない。

### SPEC-165

要求・要件・仕様・VO等の意味上の定義や対応関係を決定する責任はプロジェクト側（開発者・設計者・PM等）にある。

### SPEC-166

外部AI/Agentによる補助・提案は許容する。

### SPEC-S047 18.3 判断待ち情報の構造化

### SPEC-167

導入時・検証時に生じる、未確定事項・不整合・未検証事項・機械的に確認済みの事実・外部判断が必要な事項を、機械可読な構造として保持・取得可能とする。

### SPEC-168

表示形式（表・GUI等）は要件でなく詳細設計へ委譲する。

*導出元: REQ-228, REQ-229, REQ-309, REQ-310, REQ-311, REQ-312, REQ-313, REQ-314, REQ-315, REQ-316, REQ-317, REQ-318, REQ-319, REQ-320, REQ-321, REQ-322, REQ-323, REQ-324, REQ-325, REQ-326, REQ-327, REQ-328, REQ-329, REQ-330, REQ-331, REQ-332*

*引用: 要件定義 §17.3、§28*

### SPEC-S048 18.4 導入難度の規模非依存

### SPEC-169

プロジェクト規模が大きいこと自体とは別の理由で導入難度が構造的に増大する設計を避ける。

### SPEC-170

プロジェクト規模が大きいこと自体とは別の理由で導入難度が構造的に増大する設計を避けることは、強い不変条件ではなく設計原則とする。

### SPEC-171

物量増加に伴う処理量・作業量の増加は許容する。

*導出元: REQ-230, REQ-231, REQ-232*

*引用: 要件定義 §17.4*

## SPEC-S049 19. トレーサビリティと役割別 projection

*導出元: REQ-S007*

### SPEC-172

関係型を単一へ潰さず、横断してトレース可能にする。

*導出元: REQ-036, REQ-037, REQ-038, REQ-039, REQ-040, REQ-041, REQ-042, REQ-043, REQ-044, REQ-045, REQ-046, REQ-047, REQ-048, REQ-049*

*引用: 要件定義 §3.4、NFR-003*

### SPEC-173

文書間はderives_from、VO→Testはcovers、Test↔実装は検証対象／実装traceabilityのように性質の異なる関係型を区別する。

### SPEC-174

関係型そのものの意味論的増殖は求めない。

### SPEC-175

説明文を付加・保存できるデータ構造とする。

### SPEC-176

最小の意味単位「上流ノード → 関係 → 下流ノード」を任意のノードから取得できる。

### SPEC-177

必要に応じて上流／下流へ連続して辿れる。

### SPEC-178

プロジェクト全体のトレーサビリティ構造も取得できる。

### SPEC-179

常に全チェーンを表示することは求めない。

### SPEC-180

同一のトレーサビリティ構造から、利用者の役割または利用目的に応じて参照対象・関係・集約粒度を変えたprojectionを取得・提示できる。

> 例：PM は上位の document・VO の状態と未確定/NG、Tester は VO・Test・検証対象・Evidence・未実施/失敗理由、Coder は実装から関連 Test・VO・上流文書へのトレース。

### SPEC-181

役割を固定enumやモード名として仕様化することは本書では行わない。

### SPEC-182

preset・UI・モード体系は詳細設計へ委譲する。

*導出元: REQ-309, REQ-310, REQ-311, REQ-312, REQ-313, REQ-314, REQ-315, REQ-316, REQ-317, REQ-318, REQ-319, REQ-320, REQ-321, REQ-322, REQ-323, REQ-324, REQ-325, REQ-326, REQ-327, REQ-328, REQ-329, REQ-330, REQ-331, REQ-332*

*引用: 要件定義 §28*

## SPEC-S050 20. フェーズゲートと進行条件

*導出元: REQ-S057*

### SPEC-183

プロジェクト側が登録したフェーズ・工程・ゲートの進行条件について、現在の検証状態（§4）と承認（§17）が通過条件を満たすかを評価・提示できなければならない（MUST）。

*導出元: REQ-302, REQ-303, REQ-304, REQ-305, REQ-306, REQ-307, REQ-308*

*引用: 要件定義 §26.4*

### SPEC-184

ゲートは両者の組合せを進行条件にできる。

### SPEC-185

ゲート条件の定義を受理できる。

> 通常開発中   : verification = PASS で進行可、approval 不要
> Release gate : verification = PASS + Reviewer approval
> Delivery gate: verification = PASS + Owner / PM approval

### SPEC-186

具体的なフェーズ名・承認ロール・必要承認数・権限schema・進行条件定義は詳細設計へ委譲する。

*導出元: REQ-309, REQ-310, REQ-311, REQ-312, REQ-313, REQ-314, REQ-315, REQ-316, REQ-317, REQ-318, REQ-319, REQ-320, REQ-321, REQ-322, REQ-323, REQ-324, REQ-325, REQ-326, REQ-327, REQ-328, REQ-329, REQ-330, REQ-331, REQ-332*

*引用: 要件定義 §28*

## SPEC-S051 21. テスト実行と Execution Evidence

*導出元: REQ-S019, REQ-S054*

### SPEC-187

`vtest run` はテストを実際に実行する。

### SPEC-188

`vtest run` は判定権威（§7）であるランナーの結果をEvidenceとして記録する。

*導出元: REQ-115, REQ-116, REQ-117, REQ-118, REQ-119, REQ-120, REQ-295, REQ-296*

*引用: 要件定義 §6、§26.1*

## SPEC-S052 22 完全検証・集約・報告

*導出元: REQ-S053*

### SPEC-S053 22.1 完全検証 OK

### SPEC-189

検査の部分集合を指定した実行は限定scopeである。

### SPEC-S054 22.2 集約

### SPEC-190

Test単位の結果をVO・Feature・document単位へ集約可能とする。

### SPEC-191

詳細出力では子の個別値をすべて確認できる。

### SPEC-S055 22.3 報告

### SPEC-192

NGの場合、どのエンティティの・どの検査が・どの状態で・どの診断ラベルとともに落ちたかを掘り下げ可能とする。

*導出元: REQ-297, REQ-298*

*引用: 要件定義 §26.2、NFR-006*

### SPEC-193

詳細出力は、任意ノードからの局所／経路／全体トレース（§19）に沿ったツリー表示とする。

### SPEC-194

詳細出力では非 `PASS` の根拠（判断記録・Evidenceへの参照）を辿れる。

### SPEC-195

管理下にある事実と、いずれのVOへも寄与しない事実の双方を出力から確認できる状態にする。

### SPEC-196

人間向けテキストと機械可読JSONの両方を出力できる。

*引用: 要件定義 NFR-007 / NFR-008*

## SPEC-S056 24 データ保存の基本方針

*導出元: REQ-S050, REQ-S058*

### SPEC-S057 24.2 並列編集耐性の設計原則

### SPEC-197

マージ後の論理的不整合（ID衝突、dangling reference、承認の失効）はスキャンと整合性検査で検出する（§23）。

### SPEC-198

Test ID衝突・dangling referenceの検出、派生indexの再構築、Testと関連情報の同期を人間/Agentの記憶だけに依存させないことは§23と§24.3で担保する。

### SPEC-199

具体的な物理保存方式は詳細設計へ委譲する。

*導出元: REQ-309, REQ-310, REQ-311, REQ-312, REQ-313, REQ-314, REQ-315, REQ-316, REQ-317, REQ-318, REQ-319, REQ-320, REQ-321, REQ-322, REQ-323, REQ-324, REQ-325, REQ-326, REQ-327, REQ-328, REQ-329, REQ-330, REQ-331, REQ-332*

*引用: 要件定義 §28*

### SPEC-S058 24.3 派生情報の再構築

### SPEC-200

キャッシュ / indexの具体的データ形式は詳細設計へ委譲する。

*導出元: REQ-309, REQ-310, REQ-311, REQ-312, REQ-313, REQ-314, REQ-315, REQ-316, REQ-317, REQ-318, REQ-319, REQ-320, REQ-321, REQ-322, REQ-323, REQ-324, REQ-325, REQ-326, REQ-327, REQ-328, REQ-329, REQ-330, REQ-331, REQ-332*

*引用: 要件定義 §28*

## SPEC-S059 25. 利用者別ユースケース

*導出元: REQ-S047*

### SPEC-201

具体的role taxonomy・presetの固定は行わない。

### SPEC-202

役割別の参照観点は§19のprojectionとして提供する。

### SPEC-203

Coder AIはMCP経由で、担当したVO / Testをscopeに指定して検証する。

> 要件定義 §20 の利用者ごとに想定する主経路を示す。

### SPEC-204

Coder AIは自身の変更が要求された検証を満たしたか確認する。

### SPEC-205

DeveloperはCLIで、Structured Test Operationによるテスト作成・変更を行う。

### SPEC-206

Developerは検証結果の詳細表示を行う。

### SPEC-207

CIはCLI（非対話）で `vtest verify` を同一revisionで再実行し、終了コードで判定する。

### SPEC-208

CIはEvidenceを成果物として保存する。

### SPEC-209

Reviewer AIはMCP経由で、Coderが提出したEvidence・判断記録と、自身の再検証結果を照合する。

### SPEC-210

PM / PM AgentはCLIまたはMCPで、documentまたはVO単位の集約結果からNG箇所へ掘り下げる。

## SPEC-S060 26. インターフェース概要

*導出元: REQ-S049, REQ-S058*

### SPEC-211

GUIは必須要件としない。

*導出元: REQ-267, REQ-268, REQ-269, REQ-270*

*引用: 要件定義 §22*

### SPEC-S061 26.1 CLI コマンド体系

### SPEC-212

コマンドの完全仕様（引数・出力・終了コード）は詳細設計で定める。

### SPEC-213

本書ではコマンド一覧と責務を確定する。

### SPEC-214

終了コード体系の詳細は詳細設計へ委譲する。

*導出元: REQ-105, REQ-106, REQ-107, REQ-108, REQ-309, REQ-310, REQ-311, REQ-312, REQ-313, REQ-314, REQ-315, REQ-316, REQ-317, REQ-318, REQ-319, REQ-320, REQ-321, REQ-322, REQ-323, REQ-324, REQ-325, REQ-326, REQ-327, REQ-328, REQ-329, REQ-330, REQ-331, REQ-332*

*引用: 要件定義 §5.4、§28*

### SPEC-S062 26.2 MCP ツール体系

### SPEC-215

ツールの完全な入出力スキーマは詳細設計で定める。

### SPEC-216

CLI command体系・MCP tool体系の詳細は詳細設計へ委譲する。

*導出元: REQ-309, REQ-310, REQ-311, REQ-312, REQ-313, REQ-314, REQ-315, REQ-316, REQ-317, REQ-318, REQ-319, REQ-320, REQ-321, REQ-322, REQ-323, REQ-324, REQ-325, REQ-326, REQ-327, REQ-328, REQ-329, REQ-330, REQ-331, REQ-332*

*引用: 要件定義 §28*

## SPEC-S063 27. 対応範囲と adapter 境界

*導出元: R-2, R-3, REQ-S048*

### SPEC-217

`rust-cargo` はRust・Rust function unit test・小規模なintegration testを対象とする。

### SPEC-218

`rust-cargo` 以外のproduction language adapterはv0.1の提供範囲に含めない。

*導出元: R-2*

*引用: 要件定義 R-2*

### SPEC-219

能力不足で確認できない項目は§8条項4に従い扱う。

## SPEC-S064 28. 非機能要求への対応方針

*導出元: REQ-S051*

### SPEC-220

NFR-003追跡可能性への対応は、document→VO→Test→SRC→Evidenceの双方向グラフ、任意ノードからの局所／経路／全体取得とする（§19、§23）。

### SPEC-221

NFR-005 Fail Closedへの対応は、状態モデルと集約規則（§4、§22）、承認・判断の内容ハッシュ束縛（§11、§17）とする。

### SPEC-222

NFR-006説明可能性への対応は、状態・診断ラベルの分離（§4）、根拠を辿れる詳細レポート（§22.3）とする。

### SPEC-223

NFR-007自動化適性への対応は、非対話CLI・MCP、JSON出力、終了コードとする（§26）。

### SPEC-224

NFR-008人間可読性への対応は、ツリー形式の詳細出力、IDの人間可読性とする（§3.2、§22.3）。

## SPEC-S065 29. スコープ外

*導出元: REQ-S052*

### SPEC-225

要件定義 §25 のスコープ外事項に対応する機能を本書では定義しない。

*導出元: REQ-288, REQ-289, REQ-290, REQ-291, REQ-292, REQ-293, REQ-294*

*引用: 要件定義 §25*

### SPEC-226

本システムはVerification Infrastructureとして機能する。

### SPEC-227

v0.1は宣言された義務の裏付けのみ検証する（OOS-005宣言されていない実装）。

### SPEC-228

v0.1は宣言されていない実装の存在を関知しない（OOS-005宣言されていない実装）。

*導出元: R-2*

*引用: R-2*

### SPEC-229

実装レイヤーの孤児検出・シンボル列挙の定義・上流文書の意味構造はv0.2のスコープとする。

## SPEC-S066 30. 詳細設計へ委譲する事項

*導出元: REQ-S058*

### SPEC-230

以下は本書の要求・要件を基に詳細設計で決定する（要件定義 §28 の23項目に対応）。

*導出元: REQ-309, REQ-310, REQ-311, REQ-312, REQ-313, REQ-314, REQ-315, REQ-316, REQ-317, REQ-318, REQ-319, REQ-320, REQ-321, REQ-322, REQ-323, REQ-324, REQ-325, REQ-326, REQ-327, REQ-328, REQ-329, REQ-330, REQ-331, REQ-332*

*引用: 要件定義 §28*

### SPEC-231

HOWは本書で発明しない。

### SPEC-232

詳細設計は、文書の具体的な入力フォーマットと登録方式を決定する（§16）。

### SPEC-233

詳細設計は、文書層の根の指定方式（orphan_detectionの除外指定。§5.2）を決定する。

### SPEC-234

詳細設計は、VO保存形式を決定する（§10、§24.1）。

### SPEC-235

詳細設計は、Test metadataの具体的annotation syntax（`rust-cargo` の `@vtest.*` 文法を含む。§22）を決定する。

### SPEC-236

詳細設計は、relationの保存形式を決定する（§3.2、§24.2）。

### SPEC-237

詳細設計は、Test ID命名規則を決定する（§3.2）。

### SPEC-238

詳細設計は、Target Reference / SRC IDの具体的識別方式を決定する（§9.2）。

### SPEC-239

詳細設計は、AST / LSP等の具体的解析技術（不成立証明・存在確認・静的到達の実装。§5.5、§8.3）を決定する。

### SPEC-240

詳細設計は、`oracle_presence` の信頼基盤の具体的範囲と委譲確認の方法を決定する（§8.2）。

### SPEC-241

詳細設計は、`target_binding` の動的計測方式を決定する（§5.3）。

### SPEC-242

詳細設計は、診断ラベルの語彙を決定する（§4.2）。

### SPEC-243

詳細設計は、終了コード体系（検証状態と内部エラーの分離。§26.1）を決定する。

### SPEC-244

詳細設計は、エスカレーション出力・判断記録・承認記録の具体的schemaを決定する（§11、§17）。

### SPEC-245

詳細設計は、CLI command体系を決定する（§26.1）。

### SPEC-246

詳細設計は、MCP tool体系を決定する（§26.2）。

### SPEC-247

詳細設計は、キャッシュ / indexの具体的データ形式を決定する（§24.3）。

### SPEC-248

詳細設計は、並列編集時の物理的保存方式を決定する（§24.2）。

### SPEC-249

詳細設計は、承認workflowの具体的状態遷移を決定する（§17）。

### SPEC-250

詳細設計は、判断待ち情報（§18.3）の具体的な構造schemaと取得インターフェースを決定する。

### SPEC-251

詳細設計は、関係リンクの任意説明（§19）の保存形式を決定する。

### SPEC-252

詳細設計は、役割別projection / view（§19）のpreset・UI・モード体系を決定する。

### SPEC-253

詳細設計は、approval authority（§17）の承認ロール・必要承認数・権限schemaを決定する。

### SPEC-254

詳細設計は、フェーズ・ゲート（§20）の具体的なフェーズ名と進行条件定義を決定する。

### SPEC-255

本書の要求・要件を基に詳細設計で決定するHOWを本書で確定しない。

## SPEC-S067 0. 本書の位置付け

*導出元: P-005*

### SPEC-256

本書は「基本仕様 v0.1」を実装可能なレベルまで具体化する。

### SPEC-257

本書は、基本仕様が定めた外部挙動の保証を変更しない。

### SPEC-258

本書と基本仕様の間に矛盾がある場合、基本仕様を正とし、本書の該当箇所を不整合として扱う。

### SPEC-259

本書は HOW（具体構文・アルゴリズム・データ構造・ID 命名・schema）を定める。

### SPEC-260

本書は、基本仕様（WHAT）に無い義務・検査・状態・文書種別・関係型を発明しない。

### SPEC-261

規範の伝播は上流から下流である。

*導出元: P-005*

*引用: 要件定義 P-005*

### SPEC-262

矛盾・不足を発見した場合は、本書を書き換えず上流へフィードバックしOwner判断を経る。

### SPEC-263

本書からの `基本仕様 §n` 参照は、再導出済み基本仕様 v0.1 の連番（§0〜§30）を指す。

### SPEC-264

本書からの `要件定義 §n` 参照は、凍結要件定義 v0.1 の連番（§1〜§28・P-001〜P-005・R-1〜R-5）を指す。

*導出元: R-1, R-5, P-001, P-005*

*引用: P-001, P-005, R-1, R-5*

### SPEC-265

正規の詳細設計は3分冊とする。

### SPEC-266

節番号は正規文書間を通した連番とする。

### SPEC-267

別紙Bは非正規のprocess文書として別に扱う。

### SPEC-268

本冊（コア設計）は正規であり、§1〜§11、§16、§17、§19を収録節とする。

> | 文書 | 位置付け | 収録節 |
> |---|---|---|
> | 本冊（コア設計） | 正規 | §1〜§11、§16、§17、§19 |
> | 別紙A（CLI・MCPインターフェース仕様） | 正規 | §12〜§15 |
> | 別紙B（実装計画） | 非正規 / process | 正規節番号を持たない |
> | 別紙C（受入仕様） | 正規 | §18 |

### SPEC-269

別紙A（CLI・MCPインターフェース仕様）は正規であり、§12〜§15を収録節とする。

### SPEC-270

別紙B（実装計画）は非正規/process文書であり、正規節番号を持たない。

### SPEC-271

別紙C（受入仕様）は正規であり、§18を収録節とする。

### SPEC-272

本冊の新設サブ節（§5.6 文書層孤児検出、§11.5 フェーズゲート、§11.6 役割別 projection、§11.7 判断待ち情報）は本冊の収録節範囲内に置き、別紙A / C の節番号を侵さない。

### SPEC-273

本書は、基本仕様が固定するCLIコマンド一覧・MCPツール一覧を増やさない。

*引用: 基本仕様 §26.1, 基本仕様 §26.2*

### SPEC-274

引数・入出力の完全schemaは別紙Aが定める。

### SPEC-275

本書は意味論とデータschema、および露出点だけを確定する。

## SPEC-S068 2. データディレクトリと設定

### SPEC-S069 2.2 `config.yaml`

### SPEC-276

統合したTest IDは全adapterでglobal uniquenessを検査する。

## SPEC-S070 3. レコードファイルスキーマ

### SPEC-S071 3.1 document レコード（`.verify/doc/DOC-*.yaml`）

### SPEC-277

`note` は付加・保存できる構造とする。

### SPEC-278

本システムは文書内容の意味的良否を検証しない。

*引用: 基本仕様 §29 OOS-001*

### SPEC-S072 3.2 VO レコード（`.verify/vo/VO-*.yaml`）

#### SPEC-S073 3.2.1 dimensions と組合せの実体化

### SPEC-279

組合せ空間の定義が仕様に対して十分かは本システムの検査ではなく、`UNKNOWN` としてエスカレーションの領分である（§8）。

*引用: 基本仕様 §11, 基本仕様 §10*

### SPEC-280

`combinations` の値が仕様に対して十分な組合せ集合かは本システムの検査ではなく、エスカレーションの領分である。

*引用: 基本仕様 §10, 基本仕様 §11*

### SPEC-S074 3.4 判断記録レコード（`.verify/decisions/<ULID>.yaml`）

*導出元: REQ-S035*

### SPEC-281

判断記録は、`UNKNOWN` に対して外部（人間または判断可能Agent）が下した判断の記録である。

*導出元: REQ-180, REQ-181, REQ-182, REQ-183, REQ-184, REQ-185, REQ-186, REQ-187, REQ-188, REQ-189, REQ-190, REQ-191, REQ-192*

*引用: 基本仕様 §11.3, 要件定義 §12*

### SPEC-282

判断記録は検査ゲートではなく、`UNKNOWN` に対する外部判断の追跡である。

### SPEC-283

判断済みと承認済みは区別する（判断済み ≠ 承認済み）（§3.5）。

### SPEC-S075 3.5 承認レコード（`.verify/approvals/<ULID>.yaml`）

*導出元: REQ-S046*

### SPEC-284

承認は検証状態と独立の別軸である。

*導出元: REQ-109, REQ-110, REQ-111, REQ-112, REQ-113, REQ-114*

*引用: 基本仕様 §4.5, 基本仕様 §17, 要件定義 §5.5*

### SPEC-285

承認は特定のエンティティ型に従属しない独立の領域である。

### SPEC-S076 3.6 Evidence レコード（`.verify/evidence/<ULID>.yaml`）

### SPEC-286

`target_coverage` は `target_binding` の動的計測（宣言対象の実行が生じたか）の結果であり、独立の検査項目ではない。

## SPEC-S077 5. Discovery orchestration設計

### SPEC-S078 5.2 エンティティモデル（vtest-model）

### SPEC-287

検証状態は `Pass` / `Fail` / `Mismatch` / `NoEvidence` / `Unknown` の5つのみである。

*導出元: REQ-085, REQ-086, REQ-087, REQ-088, REQ-089, REQ-090, REQ-091*

*引用: 基本仕様 §4.1, 要件定義 §5.1*

### SPEC-288

検査は `ChainIntegrity` / `OrphanDetection` / `TargetBinding` / `OraclePresence` の4本のみである。

*導出元: REQ-034, REQ-035, REQ-050, REQ-051, REQ-052, REQ-053, REQ-054, REQ-055, REQ-056, REQ-057, REQ-058, REQ-059, REQ-060, REQ-061, REQ-062, REQ-063, REQ-064, REQ-065, REQ-066, REQ-067, REQ-068, REQ-069, REQ-070, REQ-071, REQ-072, REQ-073, REQ-074, REQ-075, REQ-076, REQ-077, REQ-078, REQ-079, REQ-080, REQ-081, REQ-082, REQ-083, REQ-084*

*引用: 基本仕様 §5, 要件定義 §3.3, 要件定義 §4*

### SPEC-S079 5.3 検証グラフ

### SPEC-289

関係型（`derives_from` / `covers` / `targets` / 外部Relation）は横断トレース可能とするが、単一へ潰さず、また意味論的に増殖もさせない。

### SPEC-S080 5.4 整合性診断

### SPEC-290

warningはレポートに常に表示する。

### SPEC-S081 5.6 文書層 orphan_detection

*導出元: REQ-S010*

### SPEC-291

`orphan_detection` は文書層の孤児検出であり、親（上流document）を持たない `document` ノードが存在するかを問う。

*導出元: REQ-059, REQ-060, REQ-061, REQ-062, REQ-063*

*引用: 基本仕様 §5.2, 要件定義 §4.2*

### SPEC-292

`orphan_detection` の対象は文書層のみである。実装レイヤーの孤児検出（宣言されていない実装の検出）は行わない。

*導出元: R-2, REQ-292*

*引用: 要件定義 R-2, 基本仕様 §29 OOS-005*

### SPEC-293

旧モデルのW-SCAN-102（孤立VO）はVO層の警告であり、文書層 `orphan_detection` とは別物として存置する。

## SPEC-S082 6. Target Reference解決

### SPEC-S083 6.1 adapter-neutral解決contract

### SPEC-294

解決結果は「解決済み」「対象なし」「曖昧」の3状態を区別する。

## SPEC-S084 7. Static Analysis orchestrationと`rust-cargo`ルール

### SPEC-295

静的解析は`oracle_presence`（照合装置の存在）へ証拠を供給する。

### SPEC-296

静的解析は`target_binding`の静的到達証明（DA-002）へ証拠を供給する。

### SPEC-297

`vtest audit static`は要求時に解析を起動し、結果をstdoutと`cache/`へ出力する。

*引用: 基本仕様 §26.1*

### SPEC-298

`vtest audit static`は判断記録（§8）とは別機構であり、外部判断の記録には転用しない。

### SPEC-S085 7.2 `rust-cargo` ルール一覧

### SPEC-299

ルールごとの判定結果と根拠（該当スパン）は`vtest audit static`の出力および`cache/`の派生結果として提示する。

### SPEC-S086 7.3 target 到達の静的証明と runtime 証明の関係（target_binding）

### SPEC-300

本節の到達要件は検証対象をSource Targetとして実現する形態に限定する（`rust-cargo`）。

> 基本仕様 §5.3「実装 construct（Source Target）を検証対象とする実行形態では…」

*引用: 基本仕様 §5.3*

### SPEC-301

検証対象をSource Targetとして宣言しない他の実行形態（外部契約・境界上の振る舞い）の確認方法は、特定形態を他形態へ一律要求せず下位仕様・後続版へ委譲する。

*導出元: REQ-064, REQ-065, REQ-066, REQ-067, REQ-068, REQ-069, REQ-070, REQ-071, REQ-072*

*引用: 要件定義 §4.3, 基本仕様 §5.3・§8.3*

### SPEC-302

本節の target 実行到達規則を普遍規則として適用しない。

### SPEC-303

本節はprocess boundaryによってDA-002到達が恒久UNKNOWNになる問題だけを解消するものであり、boundary testを完全にoracle_presence PASS可能にするものではない。

## SPEC-S087 8. 判断記録プロトコル

*導出元: REQ-S035*

### SPEC-304

本システムは、宣言されていない義務・網羅漏れ・宣言と実装の意味のずれを、自ら発見・裁定しない。

*導出元: REQ-180, REQ-181, REQ-182, REQ-183, REQ-184, REQ-185, REQ-186, REQ-187, REQ-188, REQ-189, REQ-190, REQ-191, REQ-192*

*引用: 基本仕様 §11, 要件定義 §12*

### SPEC-305

判断記録プロトコルは検証状態のゲートではない。

### SPEC-S088 8.1 バンドル生成

### SPEC-306

`case-coverage`は§11の判断対象であって§5の4検査ではない。

### SPEC-307

外部判断が必要な事実は§11.7の判断待ち情報として提示する。

### SPEC-S089 8.5 有効性と再判断

### SPEC-308

判断記録の有効性は判定時に評価する。

### SPEC-309

対象は`(subject, judgment_kind)`の組であり、組ごとに独立に評価する。

### SPEC-310

未確定である事実は§11.7の判断待ち情報として提示する。

### SPEC-311

同一対象に有効な判断記録が複数あってよい（再判断・多重判断）。

### SPEC-312

回数はツールとして制限しない（運用ポリシー）。

### SPEC-313

現在状態に対して通常の検証（§5の4検査）を再実施した結果は`PASS` / `FAIL` / `MISMATCH` / `NO_EVIDENCE` / `UNKNOWN`のいずれにもなり得る。

### SPEC-314

変更そのものが`UNKNOWN`を生成するのではない。

### SPEC-315

判断済みと承認済みは区別する（判断済み≠承認済み）。

### SPEC-316

判断は承認なしでも記録でき、正式採用は§3.5の承認の別段階である。

### SPEC-S090 8.6 参考プロンプト

### SPEC-317

判断エージェントのプロンプト・スキル構成はツールの責務外だが、参考として骨子を示す。

> あなたは検証対象の意味判定者である。添付のバンドルについて、以下だけを判定せよ。修正方針の提案はしない。判定事項：テストコードは、VOのclaimとTest Intentが宣言する振る舞いを実際に検証しているか。判定はaccepted / rejected / deferredのいずれかとし、判定ごとにclaim（何を確認したか）とbasis（根拠にしたバンドル内の情報への参照）を任意で列挙してよい。

### SPEC-318

判断は`UNKNOWN`に対する外部判断の追跡であり、検査ゲートではない（§8 冒頭、基本仕様 §11.3）。

*引用: 基本仕様 §11.3*

## SPEC-S091 10. `rust-cargo` Target Binding 動的計測

### SPEC-S092 10.3 実行モードの整理

### SPEC-319

`vtest run`は2モードを持つ。

## SPEC-S093 11. 鮮度検証と集約

### SPEC-S094 11.1 検査の評価地点

### SPEC-320

`target_binding`は評価地点をTESTとし、§7.3の合成による。

### SPEC-321

`target_binding`の未充足は§11.2の写像に従う。

### SPEC-322

`oracle_presence`は評価地点をTESTとし、§7.1の合成（DA-001 / DA-003 / DA-004 / DA-005 / DA-006）による。

### SPEC-323

本システムは意味判定・候補生成を外部の判定器へ委ねるseam（実行時に差し替え可能な意味判定・意味生成の呼出し点）を4検査の評価経路に持たない。

### SPEC-324

外部AI／Agentは判断記録（§8）の著者として`.verify/decisions/`へ記録を残す経路でのみ関与し、その記録は入力集合の一部としてファイル経由で読まれる。

### SPEC-325

完全検証の検査集合はこの4検査に固定し、設定で追加・削除できない（§2.2、基本仕様 §22.1）。

*引用: 基本仕様 §22.1*

### SPEC-326

旧モデルの12項目（`spec_coverage` / `vo_decomposition` / `vo_coverage` / `test_existence` / `static_audit` / `semantic_audit` / `impl_consistency` / `test_execution` / `runtime_result` / `target_execution` / `evidence_validity` / `test_traceability`）は検査として存在しない。

### SPEC-327

`test_existence` / `test_traceability`は`chain_integrity`へ統合した。

### SPEC-328

`static_audit`は`oracle_presence`（DA-001/003/004/005/006）と`target_binding`の静的到達（DA-002）へ分割した。

### SPEC-329

`test_execution` / `target_execution` / `runtime_result`は`target_binding`の証拠（Evidenceの存在・鮮度、`result`、`target_coverage`）へ吸収した。

### SPEC-330

`evidence_validity`は独立検査を廃し、鮮度喪失を診断ラベル`STALE`として§11.2で説明した（基本仕様 §6）。

*引用: 基本仕様 §6*

### SPEC-331

`spec_coverage` / `vo_coverage` / `vo_decomposition` / `semantic_audit` / `impl_consistency`は検査から除去し、網羅・意味の疑義は`UNKNOWN`として判断記録エスカレーションとした（§8、基本仕様 §11、要件定義 §12）。

*導出元: REQ-180, REQ-181, REQ-182, REQ-183, REQ-184, REQ-185, REQ-186, REQ-187, REQ-188, REQ-189, REQ-190, REQ-191, REQ-192*

*引用: 基本仕様 §11, 要件定義 §12*

#### SPEC-S095 11.1.1 `chain_integrity` の評価

### SPEC-332

`chain_integrity`は宣言鎖のすべてのリンクが存在し、ハッシュ照合が成立するかを問う。

*引用: 基本仕様 §5.1*

### SPEC-333

すべてのTestを管理対象とすることと、当該Testを証拠として算入すること（§7 / §10のtarget_binding / oracle_presence）は別個の条件とする。

*引用: 基本仕様 §8.1*

### SPEC-S096 11.3 集約アルゴリズム

### SPEC-334

利用者向け簡易出力は`OK` / `NG`の二値とする。

*引用: 基本仕様 §22.1*

### SPEC-335

詳細出力は任意ノードからの局所／経路／全体トレース（§11.6）に沿ったツリー表示とし、非`PASS`の根拠（判断記録・Evidenceへの参照）を辿れる。

### SPEC-336

人間向けテキストと機械可読JSONの両方を出力できる。

*引用: 基本仕様 §22.3*

### SPEC-S097 11.4 document 鮮度

### SPEC-337

仕様文書の更新は`vtest doc add --update`による再登録で反映し、依存する判断・承認が失効することを利用者へ提示する。

### SPEC-S098 11.5 フェーズゲート評価

*導出元: REQ-S057*

### SPEC-338

プロジェクト側が登録したフェーズ・工程・ゲートの進行条件について、現在の検証状態（§4.1の5状態）と承認（§3.5）が通過条件を満たすかを評価・提示できなければならない（MUST）。

*導出元: REQ-302, REQ-303, REQ-304, REQ-305, REQ-306, REQ-307, REQ-308*

*引用: 基本仕様 §20, 要件定義 §26.4*

### SPEC-339

検証状態と承認は独立の軸であり、ゲートは両者の組合せを進行条件にできる。

### SPEC-340

本システムの責務はゲート条件が現在満たされているかの評価・提示に限る。

### SPEC-341

フェーズのライフサイクル管理・工程の自動遷移は責務外とする（§29 OOS-004）。

*導出元: REQ-302, REQ-303, REQ-304, REQ-305, REQ-306, REQ-307, REQ-308*

*引用: 基本仕様 §20, 要件定義 §26.4, OOS-004*

### SPEC-342

「Releaseフェーズへ遷移させる」のではなく「Release gateの条件を現在満たしている」を提示する。

### SPEC-343

具体的なフェーズ名・承認ロール・必要承認数・権限schemaはプロジェクト設定と別紙Aへ委譲する（基本仕様 §30）。

*引用: 別紙A, 基本仕様 §30*

### SPEC-S099 11.6 役割別 projection

*導出元: REQ-S007*

### SPEC-344

同一のトレーサビリティ構造から、利用者の役割または利用目的に応じて参照対象・関係・集約粒度を変えたprojectionを取得・提示できる。

*導出元: REQ-036, REQ-037, REQ-038, REQ-039, REQ-040, REQ-041, REQ-042, REQ-043, REQ-044, REQ-045, REQ-046, REQ-047, REQ-048, REQ-049*

*引用: 基本仕様 §19, 要件定義 §3.4*

### SPEC-345

最小の意味単位「上流ノード → 関係 → 下流ノード」を任意のノード（DOC / VO / TEST / SRC）から取得でき、必要に応じて上流／下流へ連続して辿れ、プロジェクト全体のトレーサビリティ構造も取得できる。

> 任意ノードからの取得。

### SPEC-346

常に全チェーンを表示することは求めない。

### SPEC-347

役割または利用目的に応じた参照観点をpresetとして提供する（例：PMは上位のdocument・VOの状態と未確定/NG、Testerは VO・Test・検証対象・Evidence・未実施/失敗理由、Coderは実装から関連Test・VO・上流documentへのトレース）。

### SPEC-348

役割を固定enumやモード名として本冊で仕様化せず、preset・UI・モード体系は別紙Aへ委譲する（基本仕様 §30）。

*引用: 別紙A, 基本仕様 §30*

### SPEC-349

projectionが出力する`derives_from`エッジに当該entryの`anchor`を常に同伴させることにより「どの上流条項が、どの概念（VO）へ対応するか」の対応ペアが構造化出力として取得でき、外部の発見者が未宣言の義務・網羅漏れを裁定する材料になる（基本仕様 §11.1）。

*引用: 基本仕様 §11.1*

### SPEC-S100 11.7 判断待ち情報の構造

### SPEC-350

導入時・検証時に生じる、未確定事項・不整合・未検証事項・機械的に確認済みの事実・外部判断が必要な事項を、機械可読な構造として保持・取得可能とする。

*導出元: REQ-228, REQ-229*

*引用: 基本仕様 §18.3, 要件定義 §17.3*

### SPEC-351

UNKNOWNだけでなく、検証出力全体にわたる未確定・要判断事項を横断的に集約する（表示形式は別紙A、基本仕様 §30 item 19）。

*引用: 別紙A, 基本仕様 §30 item 19, 基本仕様 §30*

## SPEC-S101 16. 並列動作と整合性

### SPEC-352

本冊の§12〜§15は別紙Aで定義する。

### SPEC-S102 16.1 ロック不要の根拠

### SPEC-353

すべての判定は「その時点の正典の読み取り」に基づき、正典が変われば次回のscan / verifyが差分を反映する。

### SPEC-S103 16.2 意味的衝突検出

### SPEC-354

`vtest doctor`は、同じTest IDの重複、covers先VOの欠落、承認済VOの内容不一致など、version controlの構文的整合性だけでは判定できない論理的不整合を検出する。

## SPEC-S104 17. 診断・終了コード体系

### SPEC-S105 17.1 診断コード

### SPEC-355

診断コードは§5.4のスキャン診断に加えて定義する。

### SPEC-S106 17.2 終了コード

### SPEC-356

要求scopeの総合OK / NGはJSONとtextの集約出力から読み取れる（別紙A §12.1・§12.3）。

*引用: 別紙A §12.1・§12.3, 別紙A §12.1, 別紙A §12.3*

### SPEC-357

終了コードは診断severityだけでなく操作段階で決める。

### SPEC-358

検証状態と内部エラーは終了コードで分離する。

*引用: 基本仕様 §4.4、§26.1*

## SPEC-S107 19. 実装選択と提供範囲

*導出元: R-2, R-3*

### SPEC-359

LSP / rust-analyzer連携によるシンボル解決は提供範囲外とする。

> 次の事項は提供範囲外とする。

### SPEC-360

永続インデックス（`cache/`の活用）は提供範囲外とする。

### SPEC-361

Relationのtombstone方式は提供範囲外とする。

### SPEC-362

`rust-cargo`以外のproduction language adapter（synthetic adapterは受入fixture専用）は提供範囲外とする。

### SPEC-363

LLM API直接呼び出しによる判断は提供範囲外とする。

### SPEC-364

rename追跡とSRC恒久IDの自動昇格支援は提供範囲外とする。

### SPEC-365

cargo-nextest対応は提供範囲外とする。

## SPEC-S108 0

### SPEC-366

参照規則・診断コード・終了コードは本冊 §17 に従う。

*引用: 本冊 §17*

### SPEC-367

本別紙は基本仕様 §26.1（CLI コマンド一覧）・§26.2（MCP ツール一覧）が確定したコマンド・ツールの引数と入出力 schema を具体化する HOW である。

*引用: 本冊 §0, 基本仕様 §26.1*

### SPEC-368

本別紙は新規コマンド・ツールを増やさない。

*引用: 本冊 §0*

### SPEC-369

本別紙は、上流（要件定義＝WHY、基本仕様＝WHAT、詳細設計本冊＝HOW 中核）に無い義務・検査・状態・文書種別・関係型を発明しない。

## SPEC-S109 12. CLI 詳細仕様

### SPEC-S110 12.1 共通仕様

*導出元: REQ-S009*

### SPEC-370

すべてのコマンドは非対話で完結する。

### SPEC-371

終了コードは本冊 §17.2 に従う。

*引用: 本冊 §17.2*

### SPEC-372

検証状態は5値（`PASS` / `FAIL` / `MISMATCH` / `NO_EVIDENCE` / `UNKNOWN`）である。

### SPEC-373

診断ラベルは状態に付随する原因説明であり、`MISSING` / `NOT_EXECUTED` / `NOT_CHECKED` / `STALE` である。

### SPEC-374

`NO_EVIDENCE` は状態であって診断ラベルではない。

### SPEC-S111 12.2 `vtest init`

### SPEC-375

`vtest init` は `.verify/` 一式を生成する。

> ```text
> vtest init [--name <project-name>]
> ```

*引用: 本冊 §2.1*

### SPEC-376

`vtest scan` はスキャンと整合性検査を実行し、診断一覧とエンティティ数のサマリを出力する。

> ```text
> vtest scan
> ```

*引用: 本冊 §5*

### SPEC-377

`vtest doctor` は `vtest scan` と同一処理の別名であり、自動化環境の整合性検査に使用する。

*引用: 本冊 §16.2*

### SPEC-378

`vtest doctor` は、同じTest IDの重複（E-SCAN-002）、covers先VOの欠落（E-SCAN-003）、文書鎖のリンク切れ（E-SCAN-012）、孤児 document（E-SCAN-016）、承認・判断・Evidenceのハッシュ束縛による失効（診断 `STALE`）など、version control の構文的整合性だけでは判定できない論理的不整合を検出する。

### SPEC-379

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

### SPEC-380

`doc edit` は設けない。

### SPEC-381

正典編集は `add --update` で行う。

### SPEC-382

document の承認・却下・取消は `vtest approval` で行い、`doc` 側に承認操作を置かない。

### SPEC-383

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

### SPEC-384

エンティティ側の `vo approve` / `vo_approve` はこの経路への別名にすぎず、追加・相異する規則を持たない。

### SPEC-385

`withdraw <approval-id>` は `create --subject-type <当該レコードの対象種別> --subject-id <当該レコードの対象> --state withdrawn --supersedes <approval-id>` と同一のレコードを生成する短縮形であり、追加の意味論を持たない。

### SPEC-386

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

### SPEC-387

`vtest test edit` は desired state 方式である。

> ```text
> vtest test edit TEST-X --answers desired.yaml [--dry-run]
> vtest test edit TEST-X --set covers=VO-A,VO-B [--set intent="..."]...
> ```

*引用: 基本仕様 §15.1*

### SPEC-388

`vtest audit static` は決定論的な静的解析を要求時に起動し、rule 別 verdict（`FAIL` / `UNKNOWN` / 違反なし）と根拠 span を stdout と `cache/` へ出力する。

> ```text
> vtest audit static [--test TEST-X | --all]
> ```

*引用: 本冊 §7*

### SPEC-389

`audit bundle` / `submit` は本冊 §8 の判断記録プロトコルであり、意味検査ではない。

> ```text
> vtest audit bundle (--test TEST-X | --vo VO-X)
>                    [--kind test-semantic | impl-consistency | case-coverage]
>                    [--include-failed]
> vtest audit submit --file result.json
> ```

*引用: 本冊 §8*

### SPEC-390

本システムは宣言されていない義務・網羅漏れ・宣言と実装の意味のずれを自ら発見・裁定しない。

### SPEC-391

本システムは機械が決定論で確定できない疑義を `UNKNOWN` として外部（人間または判断可能 Agent）へ引き渡し、その判断を判断記録（`.verify/decisions/`）として追跡する。

*導出元: REQ-180, REQ-181, REQ-182, REQ-183, REQ-184, REQ-185, REQ-186, REQ-187, REQ-188, REQ-189, REQ-190, REQ-191, REQ-192*

*引用: 本冊 §8 冒頭, 基本仕様 §11, 要件定義 §12*

### SPEC-392

`audit bundle` は判断対象（`--test` / `--vo`）ごとに、判断に必要な情報一式（対象 VO と claim・Test Intent・テストコード全文・Test が宣言した cases 集合・対象実装全文・関連テスト・既知 partition・過去の判断・対象の内容ハッシュとリビジョン）を JSON として `cache/bundles/<ULID>.json` へ出力し、パスと `bundle_id` を返す。

*引用: 本冊 §8.1*

### SPEC-393

旧モデルの `spec-coverage`（SPEC 層依存）は復活させない。

### SPEC-394

判断記録は検査ゲートではなく、`UNKNOWN` に対する外部判断の追跡である。

### SPEC-395

判断記録（`.verify/decisions/` の actor / subject / decision / judgment_kind・理由 optional）と承認記録（`.verify/approvals/` の approver / subject または judgment_ref / approved_state、`vtest approval create` で生成）は別軸・別 entity である。

*引用: 本冊 §3.4, 本冊 §3.5*

### SPEC-396

判断済み ≠ 承認済みである。

*引用: 本冊 §8.5, 基本仕様 §17*

### SPEC-397

判断は承認なしでも記録でき、正式採用は承認の別段階である。

### SPEC-398

`vtest run` はテスト実行と Evidence 記録を行う。

> ```text
> vtest run (--test TEST-X | --vo VO-X | --all) [--fast]
> ```

*引用: 本冊 §9、§10*

### SPEC-399

`vtest verify` は集約を実行し、`OK` / `NG` を返す。

> ```text
> vtest verify [--items <check1,check2,...>]
>              [--doc DOC-X | --vo VO-X | --test TEST-X]
>              [--gate <name>] [--summary]
> ```

*引用: 本冊 §11.3*

### SPEC-400

検査は基本仕様 §5 の固定4検査（`chain_integrity` / `orphan_detection` / `target_binding` / `oracle_presence`）のみである。

*引用: 基本仕様 §5*

### SPEC-401

旧モデルの12項目（`spec_coverage` / `vo_decomposition` / `vo_coverage` / `test_existence` / `static_audit` / `semantic_audit` / `impl_consistency` / `test_execution` / `runtime_result` / `target_execution` / `evidence_validity` / `test_traceability`）は検査として存在しない。

*引用: 本冊 §11.1*

### SPEC-402

`vtest report` は `verify` と同じ集約を実行し、根拠（判断記録 ID・Evidence ID・DA rule 診断）を含む完全な詳細を出力する。

> ```text
> vtest report [--doc DOC-X | --vo VO-X | --test TEST-X]
>              [--items <check1,check2,...>] [--gate <name>]
>              [--from <node>] [--view pm|tester|coder] [--depth <n>]
>              [--direction up|down|both] [--format json]
> ```

### SPEC-403

`verify` が判定用、`report` が閲覧・提出用という役割分担とする。

### SPEC-404

`vtest mcp` は stdio で MCP サーバを起動する（§13）。

> ```text
> vtest mcp
> ```

### SPEC-S112 12.3 フェーズゲート評価（`verify --gate` / `report --gate`）

*導出元: REQ-S057*

### SPEC-405

プロジェクト側が登録したフェーズ・工程・ゲートの進行条件について、現在の検証状態（5状態）と承認（§3.5）が通過条件を満たすかを評価・提示できなければならない（MUST）。

*導出元: REQ-302, REQ-303, REQ-304, REQ-305, REQ-306, REQ-307, REQ-308*

*引用: 本冊 §11.5, 基本仕様 §20, 要件定義 §26.4*

### SPEC-406

本システムの責務はゲート条件が現在満たされているかの評価・提示に限り、フェーズのライフサイクル管理・工程の自動遷移は責務外とする（「Release フェーズへ遷移させる」ではなく「Release gate の条件を現在満たしている」を提示する）。

### SPEC-S113 12.4 判断待ち情報 section（`verify` / `report` JSON）

*導出元: REQ-S043*

### SPEC-407

導入時・検証時に生じる、未確定事項・不整合・未検証事項・機械的に確認済みの事実・外部判断が必要な事項を、機械可読な構造として `verify` / `report` の JSON 出力へ含める。

*導出元: REQ-228, REQ-229*

*引用: 本冊 §11.7, 基本仕様 §18.3, 要件定義 §17.3*

### SPEC-408

`UNKNOWN` だけでなく、検証出力全体にわたる未確定・要判断事項を横断的に集約する。

> `judgment_kind: case-coverage` の項目の生成条件、および判断競合の項目の生成条件は本冊 §11.7 に定める。

## SPEC-S114 13. MCP ツール詳細仕様

### SPEC-S115 13.2 ツール一覧

### SPEC-409

`approval_create` は承認レコード生成の唯一の正典面である。

*引用: 本冊 §3.5*

### SPEC-410

`approval_withdraw` は `approval_create` の `state: withdrawn` ＋ `supersedes: [approval_id]` と同一である。

### SPEC-411

`vo_approve` は `approval_create` に `subject: { type: vo, id }` を与えた場合の別名であり、独自の意味論を持たない。

### SPEC-S116 13.3 エージェント向け利用フロー（参考）

### SPEC-412

完了確認は `verify` の4検査で行う。

## SPEC-S117 15. Structured Test Operation adapter contract

### SPEC-S118 15.4 `rust-cargo` 1 Test境界の保証

*導出元: REQ-S039*

### SPEC-413

helper・fixture・通常ソースコードの編集手段は提供しない。

*引用: 要件定義 OOS-003, 基本仕様 §15.3*

### SPEC-414

関数本体が helper を必要とする場合、helper の作成は通常のソース編集として利用者（人間・AI）が行う。

## SPEC-S119 18. 受入契約

### SPEC-S120 18.1 共通条件

### SPEC-415

検証結果はfail-closedである。

### SPEC-S121 18.2 共通fixture

### SPEC-416

状態は5つのみとする。

*引用: 基本仕様 §4.1*

### SPEC-417

診断ラベルは検証状態と別軸の原因説明である。

*引用: 基本仕様 §4.2*

### SPEC-418

診断ラベルは状態値ではない。

*引用: 基本仕様 §4.2*

### SPEC-419

synthetic adapterは配布対象のproduction language adapterではない。

### SPEC-S122 18.3 機能別受入条件

#### SPEC-S123 18.3.1 discovery・record・graph と chain_integrity

### SPEC-420

adapter所有のmetadata宣言、ID、target、VO参照、record schema、Relationの違反を対応診断codeで検出する。

### SPEC-421

診断ラベルを二重定義しない。

### SPEC-422

document 種別を区別せず、要件定義・基本仕様・詳細設計・API Schema 等をすべて総称 document として同一に扱う。

*引用: 本冊 §3.1*

#### SPEC-S124 18.3.2 orphan_detection（文書層の孤児検出）

*導出元: REQ-S010*

### SPEC-423

`orphan_detection` は文書層のみを対象とし、親（上流 document）を持たない `document` ノードの有無を問う。

*導出元: REQ-059, REQ-060, REQ-061, REQ-062, REQ-063*

*引用: 本冊 §5.6, 基本仕様 §5.2, 要件定義 §4.2*

### SPEC-424

実装レイヤーの孤児検出（宣言されていない実装の検出）は行わない。

*導出元: R-2, REQ-292*

*引用: 要件定義 R-2, 基本仕様 §29 OOS-005*

#### SPEC-S125 18.3.3 決定論的静的解析（oracle_presence・target_binding 静的到達）

### SPEC-425

確定違反だけをFAILとし、解析限界をUNKNOWNとして保持する。

### SPEC-426

信頼を宣言する専用の注釈・設定項目・レコードを新設せず、covers / 宣言targetのグラフだけで上記の各値が決まる。

#### SPEC-S126 18.3.4 execution・Evidence（target_binding の証拠）

### SPEC-427

旧モデルの`test_execution` / `runtime_result` / `target_execution`の3独立項目は撤去し、`target_binding`単一検査の証拠（Evidenceの存在・鮮度、`result`、`target_coverage`）へ吸収する。

*引用: 本冊 §11.1*

### SPEC-428

鮮度喪失の独立検査（旧`evidence_validity`）は設けず、鮮度は基本仕様§6のハッシュ束縛により満たし、喪失を診断ラベル`STALE`として説明する。

*引用: 基本仕様 §6*

#### SPEC-S127 18.3.5 target_binding 動的計測（per-target）

### SPEC-429

`target_coverage` は `target_binding` の動的計測結果であり独立の検査項目ではない。

#### SPEC-S128 18.3.6 判断記録プロトコル（非ゲート）

*導出元: REQ-S035*

### SPEC-430

旧モデルの reasons / claim / basis 必須検査（E-AUDIT-005）、decomposition-viewpoint 検査（E-AUDIT-006）、spec / req basis 検査（E-AUDIT-007）は撤去し、判断記録層で課さない。

### SPEC-431

判断記録プロトコルは検証状態のゲートではなく、`UNKNOWN` に対する外部判断の追跡である。

*引用: 本冊 §8, 基本仕様 §11.3*

### SPEC-432

旧モデルの `verdict → CheckValue` 写像（`impl_consistency = MISMATCH` を含む検証状態への変換経路）は撤去する。

### SPEC-433

旧モデルの意味監査 bundle 種別（spec-coverage / test-semantic / vo-coverage / impl-consistency）を検査として扱わず、網羅・意味の疑義は `UNKNOWN` として本プロトコルへエスカレーションする。

*導出元: REQ-180, REQ-181, REQ-182, REQ-183, REQ-184, REQ-185, REQ-186, REQ-187, REQ-188, REQ-189, REQ-190, REQ-191, REQ-192*

*引用: 本冊 §7.1・§8, 基本仕様 §5・§11, 要件定義 §12*

### SPEC-434

`spec_coverage` / `vo_decomposition` / `vo_coverage` / `impl_consistency` は検証項目として存在しない。

### SPEC-435

`case-coverage` は §11 の判断対象であって基本仕様 §5 の 4 検査ではない。

*引用: 基本仕様 §5*

### SPEC-436

§5 の 4 検査を再実施した結果は `PASS` / `FAIL` / `MISMATCH` / `NO_EVIDENCE` / `UNKNOWN` のいずれにもなり得る。

### SPEC-437

変更そのものが `UNKNOWN` を生成するのではない。

#### SPEC-S129 18.3.7 承認と判断記録の分離

### SPEC-438

判断済みと承認済みを区別する（判断済み ≠ 承認済み）。

### SPEC-439

判断記録と承認記録は同一 entity であることを要求せず、別 entity でありうる。

*引用: 本冊 §3.4・§3.5, 基本仕様 §11.3・§17*

### SPEC-440

判断は承認なしでも記録でき、正式採用は承認の別段階である。

### SPEC-441

承認は検証状態と独立の別軸である。

### SPEC-442

方針は総称 document として登録した文書で表現し、専用のエンティティ型を設けない。

### SPEC-443

承認権限（approval authority）・承認ロール・必要承認数・権限 schema はプロジェクト設定と別紙A へ委譲する。

*引用: 基本仕様 §17・§30*

### SPEC-444

承認 workflow の状態遷移と `approved_state` の値域は本冊 §3.5 に定める。

*引用: 本冊 §3.5*

#### SPEC-S130 18.3.8 verify・report と scope

### SPEC-445

機能単位の集約は親 VO（子 VO を持つ VO）を単位とし、Feature を別エンティティ・別レコード・別 ID として設けない。

### SPEC-446

旧モデルの SPEC → REQ → VO → Test 構造は総称 document 化により DOC → VO → Test へ再導出する。

### SPEC-447

「どの上流条項がどの VO へ対応するか」の対応ペアの取得に新規 CLI コマンド・MCP ツールを用いない（既存の `report` projection と `test query` 逆引きだけで取得できる）。

#### SPEC-S131 18.3.9 フェーズゲート評価

*導出元: REQ-S057*

### SPEC-448

プロジェクト側が登録したフェーズ・工程・ゲートの進行条件について、現在の検証状態（§4.1 の 5 状態）と承認（§18.3.7）が通過条件を満たすかを評価・提示できなければならない（MUST）。

*導出元: REQ-302, REQ-303, REQ-304, REQ-305, REQ-306, REQ-307, REQ-308*

*引用: 本冊 §11.5, 基本仕様 §20, 要件定義 §26.4*

### SPEC-449

検証状態と承認は独立の軸であり、ゲートは両者の組合せを進行条件にできる。

### SPEC-450

責務はゲート条件が現在満たされているかの評価・提示に限る。

### SPEC-451

フェーズのライフサイクル管理・工程の自動遷移は責務外とする。

*導出元: REQ-302, REQ-303, REQ-304, REQ-305, REQ-306, REQ-307, REQ-308*

*引用: 基本仕様 §20・§29 OOS-004, 要件定義 §26.4*

### SPEC-452

新規 CLI コマンド・MCP ツールを増やさず、既存の `vtest verify` の `--gate` 引数と出力、および `report` の JSON でゲート評価を露出する。

### SPEC-453

具体的なフェーズ名・承認ロール・必要承認数はプロジェクト設定と別紙A へ委譲する。

*引用: 基本仕様 §30*

### SPEC-S132 18.4 提供範囲外

*導出元: R-2, R-3*

### SPEC-454

GUI は提供範囲外である。

### SPEC-455

仕様書同士の矛盾判定は提供範囲外である。

### SPEC-456

仕様・Test・実装のどれを変更すべきかという修正方針の決定は提供範囲外である。

### SPEC-457

helper、fixture、通常sourceの編集管理は提供範囲外である。

### SPEC-458

開発process管理は提供範囲外である。

### SPEC-459

`rust-cargo`以外のproduction language adapterは提供範囲外である。

### SPEC-460

third-party plugin ABIは提供範囲外である。

### SPEC-461

LSP統合は提供範囲外である。

### SPEC-462

runner / coverage providerの自動選択または推測fallbackは提供範囲外である。
