<!-- generated from docs/canonical/specification.json by build.py; do not edit -->

# 詳細仕様

## DS-S001 0. 本書の位置付け

*導出元: P-005, REQ-S049, REQ-S058*

### DS-001

`vtest` 自身はLLM APIを呼ばない。

### DS-002

`vtest` 自身は宣言と実装の意味的な良し悪しを裁定しない。

### DS-003

機械が決定論で確定できない疑義は `UNKNOWN` として外部の判断者へ引き渡す（§11）。

*導出元: REQ-180, REQ-181, REQ-182, REQ-183, REQ-184, REQ-185, REQ-186, REQ-187, REQ-188, REQ-189, REQ-190, REQ-191, REQ-192*

*引用: 要件定義 §12*

## DS-S002 1. 用語定義

*導出元: REQ-S001, REQ-S003, REQ-S013, REQ-S020, REQ-S025, REQ-S029, REQ-S035, REQ-S036, REQ-S037, REQ-S046*

### DS-004

文書種別ごとの専用スキーマは設けない。

*導出元: REQ-024, REQ-025, REQ-026, REQ-027, REQ-028, REQ-029, REQ-030, REQ-031, REQ-032, REQ-033*

*引用: 要件定義 §3.2*

### DS-005

対象ソースコード自身のdoc commentは、その対象実装の唯一の仕様根拠としては用いない。

*導出元: REQ-233, REQ-234, REQ-235*

*引用: 要件定義 §18*

### DS-006

derives_fromとは、document間の唯一のリンク種別である。

### DS-007

VOとdocumentの間に他のエンティティ層を置かない。

### DS-008

VOの粒度をassert文・test function・テストファイルなどのコード構文で決めない。

### DS-009

Test Intentは宣言鎖のノードではない。

*導出元: REQ-203, REQ-204, REQ-205*

*引用: 要件定義 §14*

### DS-010

Execution Evidenceは検証対象の内容ハッシュに束縛される（§6）。

*導出元: REQ-115, REQ-116, REQ-117, REQ-118, REQ-119, REQ-120*

*引用: 要件定義 §6*

### DS-011

判断記録は依存closureのハッシュに束縛される（§11）。

*導出元: REQ-180, REQ-181, REQ-182, REQ-183, REQ-184, REQ-185, REQ-186, REQ-187, REQ-188, REQ-189, REQ-190, REQ-191, REQ-192*

*引用: 要件定義 §12*

### DS-012

承認記録は上流依存closureのハッシュに束縛される。

### DS-013

検証状態は `PASS` / `FAIL` / `MISMATCH` / `NO_EVIDENCE` / `UNKNOWN` の5つとする（§4.1）。

*導出元: REQ-085, REQ-086, REQ-087, REQ-088, REQ-089, REQ-090, REQ-091*

*引用: 要件定義 §5.1*

### DS-014

検証状態は検証結果のみを表す。

### DS-015

検証状態に承認状態を混入させない。

*導出元: REQ-109, REQ-110, REQ-111, REQ-112, REQ-113, REQ-114*

*引用: 要件定義 §5.5*

### DS-016

診断ラベルは検証状態ではない。

### DS-017

検査は `chain_integrity` / `orphan_detection` / `target_binding` / `oracle_presence` の4本のみとする（§5）。

*導出元: REQ-034, REQ-035, REQ-050, REQ-051, REQ-052, REQ-053, REQ-054, REQ-055, REQ-056, REQ-057, REQ-058, REQ-059, REQ-060, REQ-061, REQ-062, REQ-063, REQ-064, REQ-065, REQ-066, REQ-067, REQ-068, REQ-069, REQ-070, REQ-071, REQ-072, REQ-073, REQ-074, REQ-075, REQ-076, REQ-077, REQ-078, REQ-079, REQ-080, REQ-081, REQ-082, REQ-083, REQ-084*

*引用: 要件定義 §3.3、§4*

### DS-018

完全検証とは、宣言鎖全体に対する検査（`chain_integrity` / `orphan_detection`）と、scope内の各「宣言＋コード＋証拠」の組に対する検査（`target_binding` / `oracle_presence`）をすべて対象とする検証である。

### DS-019

完全検証は一項目でも非 `PASS` があればNGとする（fail-closed）。

*導出元: REQ-295, REQ-296*

*引用: 要件定義 §26.1*

### DS-020

scopeを狭めても対象外項目を `PASS` へ書き換えない。

*導出元: P-002*

*引用: 要件定義 §2/P-002*

### DS-021

正典から導出できる情報は派生情報とし独立保存しない。

*導出元: P-003*

*引用: 要件定義 P-003*

## DS-S003 2 全体像

### DS-S004 2.1 正典の三層構造

*導出元: P-003, REQ-S019, REQ-S035, REQ-S046*

### DS-022

registryの重複ID、未登録adapter、adapter間のTest ID重複は操作エラーとする。

### DS-023

registryの重複ID、未登録adapter、adapter間のTest ID重複は空のscanとして成功扱いしない。

*導出元: REQ-258, REQ-259, REQ-260, REQ-261, REQ-262, REQ-263, REQ-264, REQ-265, REQ-266*

*引用: 要件定義 §21*

### DS-024

どれかを正として他を修正させることはしない。

*導出元: P-001*

*引用: 要件定義 P-001*

### DS-S005 2.2 宣言鎖と照合

*導出元: REQ-S004, REQ-S005*

### DS-025

文書間リンクは `derives_from` の一種のみとする。

### DS-026

段はリンクであって検査ではない。

*導出元: REQ-024, REQ-025, REQ-026, REQ-027, REQ-028, REQ-029, REQ-030, REQ-031, REQ-032, REQ-033, REQ-034, REQ-035*

*引用: 要件定義 §3.2/§3.3*

### DS-027

VOとdocumentの間に他のエンティティ層を置かない。

### DS-028

本システムは文書内容の意味的な良し悪しに関知しない。

### DS-029

文書種別ごとの専用スキーマ・文書間リンク意味論の増殖・文書内容の良否検証を行わない。

*導出元: REQ-024, REQ-025, REQ-026, REQ-027, REQ-028, REQ-029, REQ-030, REQ-031, REQ-032, REQ-033, REQ-288, REQ-289, REQ-290, REQ-291, REQ-292, REQ-293, REQ-294*

*引用: 要件定義 §3.2、§25 OOS-001*

### DS-030

不一致はどちらが正かを決めない。

### DS-S006 2.3 導出できる関係は保存しない

*導出元: P-003*

### DS-031

Test→VO、Test→SRCの関係を外部ファイルへ重複保存しない。

### DS-032

graphと現在のtarget集合は常にadapter所有のTest metadata宣言から再構築する。

### DS-033

graphと現在のtarget集合はEvidenceのtarget参照から関係を生成・修復しない。

### DS-034

Evidenceに含むtarget参照はTest→SRC関係の正典ではない。

### DS-S007 2.4 adapter 設定と wire 互換

*導出元: REQ-S048, REQ-S058*

### DS-035

readerは読み取りだけで正典を書き換えない。

### DS-036

adapter IDは設定内で一意でなければならない。

### DS-037

同一adapter内のroot重複も拒否する。

### DS-038

異なるadapterが同じrootを走査することは許可する。

> polyglot repository を扱えるようにするための許可。

### DS-039

未知のadapterやadapter固有設定の検証失敗は操作エラーとする。

### DS-040

未知のadapterやadapter固有設定の検証失敗時、利用可能な言語や能力を推測補完しない。

*導出元: REQ-258, REQ-259, REQ-260, REQ-261, REQ-262, REQ-263, REQ-264, REQ-265, REQ-266*

*引用: 要件定義 §21*

### DS-041

非Rust Testでは空値・dummy値・Rust既定値を生成しない。

### DS-042

欠落・矛盾時は入力を拒否する。

### DS-043

欠落・矛盾時は推測で実行可能として扱わない。

## DS-S008 3 エンティティと ID 体系

### DS-S009 3.1 エンティティ種別

*導出元: REQ-S005, REQ-S025, REQ-S035, REQ-S036, REQ-S040, REQ-S046*

### DS-044

documentは種別専用スキーマを持たない。

### DS-045

Source Targetの恒久IDは必須としない。

### DS-046

documentは単一の総称ノードであり、要件定義・基本仕様・詳細設計・API Schema等を種別で区別する専用スキーマを持たない。

### DS-047

段を増やしても種別を増やさない。

*導出元: REQ-024, REQ-025, REQ-026, REQ-027, REQ-028, REQ-029, REQ-030, REQ-031, REQ-032, REQ-033*

*引用: 要件定義 §3.2*

### DS-S010 3.2 ID 規則と関係リンク

*導出元: REQ-S007, REQ-S027, REQ-S058*

### DS-048

DOC / VO / TESTのIDは人間可読な形式とする。

### DS-049

IDの文字集合は `[A-Z0-9-]` とする。

### DS-050

IDの接頭辞は種別ごとに固定する（`TEST-` 等）。

### DS-051

IDの推奨形式は `TEST-<領域>-<連番>`（例：`TEST-PARSER-044`）とする。

### DS-052

ツールはID形式を強制せず一意性のみを強制する。

### DS-053

IDの一意性はスキャン時に全数検査する。

### DS-054

ID衝突は `chain_integrity` の非 `PASS`（`MISMATCH`）とする（§5.1、§23）。

### DS-055

任意の恒久SRC IDはadapter namespaceを持たないためrepository全体で一意とする。

### DS-056

恒久SRC IDの衝突は曖昧参照として受理しない。

### DS-057

恒久SRC IDの衝突時、どのSource Targetを指すか推測しない。

*導出元: REQ-149, REQ-150, REQ-151, REQ-152, REQ-153, REQ-154, REQ-155, REQ-156*

*引用: 要件定義 §9.2*

### DS-058

derives_from・covers・検証対象・実装traceabilityなど性質の異なる関係型は潰さず区別する。

### DS-059

存在するリンクに付す説明文は空でもよい。

### DS-060

説明文が空であることを理由に `chain_integrity` 違反・`MISMATCH` としてはならない。

*導出元: REQ-036, REQ-037, REQ-038, REQ-039, REQ-040, REQ-041, REQ-042, REQ-043, REQ-044, REQ-045, REQ-046, REQ-047, REQ-048, REQ-049*

*引用: 要件定義 §3.4*

### DS-061

関係型そのものの意味論的増殖は求めない。

### DS-S011 3.3 Source Target の識別

*導出元: R-3, REQ-S027*

### DS-062

ソースコードへ恒久IDを埋め込むことは必須としない。

*導出元: REQ-149, REQ-150, REQ-151, REQ-152, REQ-153, REQ-154, REQ-155, REQ-156*

*引用: 要件定義 §9.2*

### DS-063

1つのTestは1件以上のSource Targetを持ち、各target参照を個別に保持する。

### DS-064

Source Targetは代表1件へ縮約しない。

## DS-S012 4 検証状態と診断ラベル

### DS-S013 4.1 状態は 5 つ

*導出元: REQ-S014*

### DS-065

検証状態は `PASS` / `FAIL` / `MISMATCH` / `NO_EVIDENCE` / `UNKNOWN` の5つのみとする。

*導出元: REQ-085, REQ-086, REQ-087, REQ-088, REQ-089, REQ-090, REQ-091*

*引用: 要件定義 §5.1*

### DS-066

意味の違いは資格にならない。

### DS-067

`PASS` を受け取った者はマージできる。

### DS-068

完全検証において `PASS` はOKとする。

### DS-069

`FAIL` を受け取った者は実装（テスト実装を含む）を直す。

### DS-070

完全検証において `FAIL` はOKとしない。

### DS-071

`MISMATCH` を受け取った者はコードを触る前に宣言側（上流）を直す。

### DS-072

完全検証において `MISMATCH` はOKとしない。

### DS-073

`NO_EVIDENCE` を受け取った者は証拠を作る（機械的に解決可能）。

### DS-074

完全検証において `NO_EVIDENCE` はOKとしない。

### DS-075

`UNKNOWN` は決定論の限界であり、受け取った者は意味判定できる者へエスカレーションする。

### DS-076

完全検証において `UNKNOWN` はOKとしない。

### DS-S014 4.2 診断ラベル

*導出元: REQ-S015, REQ-S058*

### DS-077

診断ラベルは検証状態ではない。

### DS-078

本書は状態と診断ラベルを常に別軸として扱い、混同しない。

### DS-S015 4.3 状態の割当

*導出元: REQ-S016*

### DS-079

発見されたTestに管理宣言が無い場合、状態は `MISMATCH`、診断ラベルはMISSINGとする。

### DS-080

`covers` のVO参照を解決できない、同一constructから複数entity、またはTest ID衝突の場合、状態は `MISMATCH` とする。

### DS-081

文書鎖のリンク切れ、content_hash不一致、または孤児文書の場合、状態は `MISMATCH`、診断ラベルはSTALE等とする。

### DS-082

証拠が存在しない、または証拠のハッシュが現在の対象と不一致の場合、状態は `NO_EVIDENCE`、診断ラベルはSTALE等とする。

### DS-083

scope限定により検査を実施しなかった項目（完全検証の集約時）は、状態を `NO_EVIDENCE`、診断ラベルをNOT_CHECKEDとする。

### DS-084

discoveryが不完全、または解析不能の場合、状態は `UNKNOWN` とする。

### DS-085

テストランナーが失敗を報告した場合、状態は `FAIL` とする。

### DS-086

宣言された検証対象の実行が0回の場合、状態は `FAIL`、診断ラベルはNOT_EXECUTEDとする。

### DS-S016 4.4 UNKNOWN の検疫

*導出元: REQ-S017*

### DS-087

内部エラー・入力不正は検証状態と別系統（終了コード。§27）で表現する。

### DS-088

`UNKNOWN` をエラー処理のフォールバック先として使う実装は仕様違反とする。

*導出元: REQ-105, REQ-106, REQ-107, REQ-108*

*引用: 要件定義 §5.4*

### DS-S017 4.5 検証状態と承認の分離

*導出元: REQ-S018*

### DS-089

検証状態（§4.1の5状態）は検証結果のみを表す。

### DS-090

検証状態は承認状態を混入させない。

### DS-091

技術的に `PASS` であっても未承認である状態を許容する。

### DS-092

未承認であることだけを理由に `PASS` を `UNKNOWN` 等へ変更してはならない。

### DS-093

承認済みであることを理由に `FAIL` / `MISMATCH` / `NO_EVIDENCE` / `UNKNOWN` を `PASS` へ変更してはならない。

### DS-S018 4.6 scope

*導出元: P-002, REQ-S016*

### DS-094

検査軸は実施する検査（4本の部分集合）を指定する。

*導出元: P-002*

*引用: 要件定義 P-002*

### DS-095

エンティティ軸は対象とするdocument / VO / Testの部分木を指定する。

### DS-096

scopeを限定した検証のOKは「要求されたscope内がOK」の意味に限られる。

### DS-097

scope外・未実施の項目は `NO_EVIDENCE`（診断NOT_CHECKED）として保持する。

### DS-098

scope外・未実施の項目は `PASS` へ変換しない。

### DS-099

出力には要求scopeと、scope外項目が未検証である旨を必ず併記する。

### DS-100

いかなる設定値も完全検証の検査を4本未満へ縮退させない。

## DS-S019 5. 検査

### DS-101

鎖に段（リンク）が増えても検査は増えない。

*導出元: REQ-034, REQ-035, REQ-050, REQ-051, REQ-052, REQ-053, REQ-054, REQ-055, REQ-056, REQ-057, REQ-058, REQ-059, REQ-060, REQ-061, REQ-062, REQ-063, REQ-064, REQ-065, REQ-066, REQ-067, REQ-068, REQ-069, REQ-070, REQ-071, REQ-072, REQ-073, REQ-074, REQ-075, REQ-076, REQ-077, REQ-078, REQ-079, REQ-080, REQ-081, REQ-082, REQ-083, REQ-084*

*引用: 要件定義 §3.3、§4*

### DS-102

答えは検証方法・実行形態に依らず同一でなければならない。

*導出元: REQ-050, REQ-051, REQ-052, REQ-053, REQ-054, REQ-055, REQ-056, REQ-057, REQ-058, REQ-059, REQ-060, REQ-061, REQ-062, REQ-063, REQ-064, REQ-065, REQ-066, REQ-067, REQ-068, REQ-069, REQ-070, REQ-071, REQ-072, REQ-073, REQ-074, REQ-075, REQ-076, REQ-077, REQ-078, REQ-079, REQ-080, REQ-081, REQ-082, REQ-083, REQ-084*

*引用: 要件定義 §4 冒頭、§8 条項 3*

### DS-103

凍結要件が検査から明示的に排除した判断（仕様網羅・VO網羅・VO分解妥当性・意味一致・実装一致）は、本書でも検査に含めない。

### DS-S020 5.1 chain_integrity — 宣言鎖の完全性

*導出元: REQ-S009, REQ-S036*

### DS-104

文書層では、各documentのderives_from参照先が存在する。

### DS-105

文書層では、各documentのcontent_hashが現物と一致する。

### DS-106

VO層では、各VOが1件以上の `document` への解決可能なderives_fromを持つ。

### DS-107

Test層では、発見された各Testに対応する管理宣言（構文上有効なTest ID・1件以上の `covers`・その他の必須metadata）がちょうど1件存在する。

### DS-108

Test層では、`covers` の全VO参照を解決できる。

### DS-109

Test層では、Test IDが発見結果全体で一意である。

### DS-110

leaf VO→Test（検証実装の存在）と、発見されたTest→宣言（管理宣言の解決）の両方向が成立して初めて双方向完全性が成立する。

### DS-111

違反時の状態は§4.3に従う（管理宣言欠落は `MISMATCH`/MISSING、参照解決不能・ID衝突は `MISMATCH`、リンク切れ・hash不一致は `MISMATCH`）。

### DS-112

すべてのTestを管理対象とすることと、当該Testを仕様適合の証拠として算入すること（§8）は別個の条件とする。

### DS-S021 5.2 orphan_detection — 文書層の孤児検出

*導出元: REQ-S010*

### DS-113

根として指定された文書は対象外とする。

### DS-114

対象は文書層のみとする。

### DS-115

実装レイヤーの孤児検出（宣言されていない実装の検出）は行わない。

*導出元: R-2, REQ-292*

*引用: 要件定義 R-2、§25 OOS-005*

### DS-116

根に指定されない孤児文書は `MISMATCH` とする（§4.3）。

### DS-S022 5.3 target_binding — 宣言対象の振る舞いの実現

*導出元: REQ-S011, REQ-S020, REQ-S054*

### DS-117

Testがテストランナー上で `PASS` しても、検証対象とする振る舞いを実際には生じさせていない場合、完全検証済みOKとしない。

### DS-118

静的に確定できなければ `UNKNOWN` とする。

### DS-119

静的に確定できない場合、動的証拠で昇格できる。

### DS-120

実装construct（Source Target）を検証対象とする実行形態では、宣言された対象コードが実際にTest実行経路へ入ったことを確認方法とする。

### DS-121

複数targetを宣言したTestでは各targetの実行を個別に計測する。

### DS-122

複数targetを宣言したTestでは、1件でも実行回数が0なら `FAIL`（診断NOT_EXECUTED）とする。

### DS-123

複数targetを宣言したTestでは、1件でも解析不能でかつ `FAIL` が無ければ `UNKNOWN` とする。

### DS-124

複数targetを宣言したTestでは、全targetの実行を確認できた場合だけ `PASS` とする。

### DS-125

別プロセス（起動したsubprocess）・別スレッド・クロージャ・他ファイル等、静的解析の到達境界を越えてtargetを実行するTestでは静的に到達を証明できず `UNKNOWN` となる。

### DS-126

静的解析の到達境界を越えてtargetを実行するTestの到達 `UNKNOWN` は、当該targetの動的計測が実行を証明した場合に限り到達要件を満たす。

### DS-127

subprocessであること自体を欠陥としない。

### DS-128

特定形態の確認方法を別形態のTestへ一律要求しない。

*導出元: REQ-064, REQ-065, REQ-066, REQ-067, REQ-068, REQ-069, REQ-070, REQ-071, REQ-072, REQ-125, REQ-126, REQ-127, REQ-128, REQ-129, REQ-130, REQ-131, REQ-132, REQ-133, REQ-134, REQ-135, REQ-136, REQ-137, REQ-138, REQ-139, REQ-140, REQ-141, REQ-142, REQ-143*

*引用: 要件定義 §4.3、§8 条項 3*

### DS-129

target_bindingは完全検証ではデフォルト有効とする。

### DS-130

target_bindingは高速な限定scopeでは省略可能とする。

### DS-131

省略・計測環境不在の場合は `NO_EVIDENCE`（診断NOT_CHECKED）とする。

### DS-132

省略・計測環境不在の場合は `PASS` へ変換しない。

### DS-S023 5.4 oracle_presence — 照合装置の存在

*導出元: REQ-S012, REQ-S021*

### DS-133

不成立が構造から証明できる（どんな宣言の下でも不成立を検出できない＝失敗し得ない、または失敗が検証対象の振る舞いに依存しないことが構造から証明できる）場合、oracle_presenceの出力は `FAIL` とする。

*導出元: REQ-073, REQ-074, REQ-075, REQ-076, REQ-077, REQ-078, REQ-079, REQ-080, REQ-081, REQ-082, REQ-083, REQ-084*

*引用: 要件定義 §4.4*

### DS-134

照合装置の存在が決定論的に確認できる場合、oracle_presence検査は成立側とする。

### DS-135

不成立が構造から証明できることと照合装置の存在が決定論的に確認できることのどちらも決定論的に言えない（解析不能等）場合、oracle_presenceの出力は `UNKNOWN` とする。

### DS-136

静的解析は成立条件から明確に外れるTestを決定論的に検出し、外部監査へ送る前に拒否する（§8）。

### DS-137

証明の失敗は `UNKNOWN` の事由ではない。

### DS-138

照合内容が宣言の期待と意味的に一致するかはoracle_presence検査の主張に含めない。

### DS-139

答えはassertの所在・実行形態（内部construct検証か境界の振る舞い検証か）に依らず同一でなければならない。

### DS-140

実行形態別の判定規則を設けない。

### DS-S024 5.5 決定論的に検出可能な不成立構造

*導出元: REQ-S024*

### DS-141

Static Auditは、成否判定が定数である（`assert!(true)` 等、失敗し得ない）Testを対象とする。

### DS-142

Static Auditは、検証対象の振る舞いを生じさせるだけで、その観測を成否判定に利用していないTestを対象とする。

### DS-143

Static Auditは、観測同士の自己比較（`assert_eq!(x, x)` 等）で成否が検証対象の振る舞いに依存しないTestを対象とする。

### DS-144

Static Auditは、空のテスト本体を持つTestを対象とする。

### DS-145

判定は保守的に行う。

### DS-146

決定論的に確定できる違反のみ `FAIL` とする。

### DS-147

確定できないものは `UNKNOWN` とする。

### DS-148

共通契約がRust構文を要求しない。

*導出元: R-3*

*引用: 要件定義 R-3、§8.3*

## DS-S025 6. 証拠

*導出元: REQ-S019*

### DS-149

証拠は検証対象の内容ハッシュに束縛される。

*導出元: REQ-115, REQ-116, REQ-117, REQ-118, REQ-119, REQ-120*

*引用: 要件定義 §6*

### DS-150

証拠ストアはハッシュキーを必須とする。

### DS-151

現在のソースのハッシュと一致しない証拠は、検証時に「存在しないもの」として扱う。

### DS-152

現在のソースのハッシュと一致しない証拠は `NO_EVIDENCE`（診断STALE）とする。

### DS-153

Evidenceの判定結果を変えうるTestの意味・実行条件・対象実装・実行可能状態が現在状態と一致することを確認できなければ、そのEvidenceを現在の `PASS` として利用してはならない。

### DS-154

鮮度の独立検査は設けない。

### DS-155

鮮度喪失は診断ラベル `STALE` として説明する。

### DS-156

`covers` / `targets` / `intent` / 実行座標その他の意味変更は内容ハッシュを必ず変化させる。

## DS-S026 8 Test の検証成立性

*導出元: REQ-S021, REQ-S025*

### DS-S027 8.1 成立と算入の独立

### DS-157

管理対象となるTestは、その宣言された目的に対して、検証対象の振る舞いを反映した観測に基づく有効な成否判定を持たなければならない。

### DS-158

仕様適合性の証拠として算入するTestは、その検証成立性が確認済みでなければならない。

### DS-159

Testとして成立しているかの検査（§8）と、仕様適合性の証拠として算入するかの判定は独立である。

*導出元: REQ-125, REQ-126, REQ-127, REQ-128, REQ-129, REQ-130*

*引用: 要件定義 §8.1*

### DS-160

全Testを管理対象とすること（`chain_integrity`）と証拠算入（成立性）は別系統とする。

### DS-S028 8.2 成立性の必要条件

### DS-161

Testは検証対象の振る舞いを反映した結果・状態・観測に基づいて適合と不適合を識別しなければならない（検証成立性）。

### DS-162

不適合がTestの非成功として反映されるものでなければならない（検証成立性）。

### DS-163

Testの成否判定が他の構成要素の判定能力に依存する場合、その依存要素の正当性が確認されるか検証基盤として明示的に信頼されていなければ、当該Testの検証成立性を確認済みとして扱ってはならない（依存要素の信頼性）。

### DS-164

判定能力を担う依存要素は、正当性確認対象または明示的な信頼基盤として識別可能でなければならない。

### DS-165

成立性確認は正当性確認対象または明示的な信頼基盤のいずれかで終端しなければならない。

### DS-166

成立条件の確認方法は検証対象・実行形態・観測方法に応じて異なってよい（証明方法への非依存）。

### DS-167

特定形態固有の確認方法を別形態へ一律要求しない。

### DS-168

成立性の問いへの答えは確認方法に依らず同一でなければならない。

### DS-169

成立条件を確認できないことと、成立条件に違反していることを区別する（未確認と違反の区別）。

### DS-170

確認不能であることだけを根拠に違反を推定してはならない。

### DS-171

確認不能であることだけを根拠に成立確認済みとして扱ってもならない。

### DS-S029 8.3 決定論的に検出可能な不成立構造

### DS-172

§8.2の成立条件を満たさないことを、宣言の中身に依らず決定論的に検出できる例はいずれも「どんな宣言の下でも不成立を検出できない」ことが構造から証明できる。

*導出元: REQ-138, REQ-139, REQ-140, REQ-141, REQ-142, REQ-143*

*引用: 要件定義 §8.3*

### DS-173

共通契約がRust構文を要求しない。

*導出元: R-3*

*引用: R-3*

### DS-174

`static_audit` に相当する判定は独立した検査項目を新設しない。

## DS-S030 9 検証対象と Source Target

*導出元: REQ-S025*

### DS-S031 9.1 検証対象

### DS-175

すべての管理対象Testは1件以上の検証対象を宣言できなければならない。

### DS-176

実装construct（Source Target）を直接検証する実行形態ではSource Target宣言をそのまま検証対象の宣言として扱う。

### DS-177

実装construct（Source Target）を直接検証する実行形態では、同一対象の二重宣言を要求しない。

### DS-178

外部契約・境界上の振る舞いを検証する実行形態では、その契約・振る舞いを検証対象とする。

### DS-179

外部契約・境界上の振る舞いを検証する実行形態では、内部Source Targetの宣言をTest成立性の必須条件としない。

### DS-S032 9.2 Source Target の識別

### DS-180

実装コード上のimplementation constructをSource Targetとして識別可能でなければならない。

*導出元: REQ-149, REQ-150, REQ-151, REQ-152, REQ-153, REQ-154, REQ-155, REQ-156*

*引用: 要件定義 §9.2*

### DS-181

複数targetを宣言した場合も各targetを独立に識別する。

### DS-182

複数targetを宣言した場合も代表1件へ縮約しない。

### DS-183

ソースコードへの恒久ID埋め込みは必須としない。

### DS-184

共通契約が特定言語構造を必須としない。

*導出元: R-3*

*引用: R-3*

### DS-185

恒久SRC IDを使用する場合、adapter境界を越えてrepository全体で一意でなければならない。

### DS-186

同一SRC IDの複数宣言を曖昧参照として受理しない。

### DS-S033 9.3 実装 traceability

### DS-187

traceabilityの存在自体をTest成立性の条件としてはならない。

### DS-188

検証対象と実装traceabilityは別の関係として扱う。

### DS-189

検証対象と実装traceabilityは、一方から他方を推定してはならない。

*導出元: REQ-157, REQ-158, REQ-159, REQ-160, REQ-161*

*引用: 要件定義 §9.3*

## DS-S034 10. Verification Obligation

*導出元: REQ-S029, REQ-S034*

### DS-190

初回登録時の階層化を必須としない。

### DS-191

VOとTestの対応は1:1に限定せず `1:1` / `1:N` / `N:1` / `N:M` を許容する。

### DS-192

複数軸を持つVOには組合せcoverageの方針を宣言できる（各軸独立／全直積／明示列挙）。

### DS-193

複数観点を同時確認するTestの存在だけを理由に各観点を独立に証明したことにはしない。

## DS-S035 11 発見・意味判定のエスカレーションと判断記録

*導出元: REQ-S035, REQ-S046*

### DS-S036 11.2 エスカレーション

### DS-194

`UNKNOWN` はエラー処理のフォールバック先に使わない（§4.4）。

### DS-S037 11.3 判断の記録と再検証

### DS-195

判断はその時点の対象成果物・前提状態に対して、依存closure（ハッシュ）とともに判断記録へ保存する。

### DS-196

判断記録の理由・根拠・evidence note（根拠となった宣言、対象外とした範囲、具体例等）は任意（optional）とする。

### DS-197

理由が空であることだけを根拠に、その判断を無効・`UNKNOWN`・`NO_EVIDENCE`・`MISMATCH` 等として扱ってはならない。

*導出元: REQ-180, REQ-181, REQ-182, REQ-183, REQ-184, REQ-185, REQ-186, REQ-187, REQ-188, REQ-189, REQ-190, REQ-191, REQ-192*

*引用: 要件定義 §12*

### DS-198

`vtest` はbundleとの対応・対象内容ハッシュの現在一致・decision値の妥当性を検証して受理・拒否する。

### DS-199

`vtest` は受理結果を依存closureのハッシュに束縛して保存する。

### DS-200

判断記録の生成・保存の構造化プロトコルは検証状態のゲートではない。

### DS-201

判断記録の受理は当該対象の検証状態を昇格させない（§4.5）。

### DS-202

判断済みと承認済みは区別する（判断済み ≠ 承認済み）。

### DS-203

仕様・VO・Test等が変更された場合、過去の判断を現在状態へそのまま流用してはならない。

### DS-204

仕様・VO・Test等が変更された場合、現在状態に対して通常の検証を再実施する。

### DS-205

仕様・VO・Test等が変更された場合、現在状態に対して通常の検証を再実施した結果は `PASS` / `FAIL` / `MISMATCH` / `NO_EVIDENCE` / `UNKNOWN` のいずれにもなり得る。

### DS-206

変更そのものが `UNKNOWN` を生成するのではない。

## DS-S038 12. Test Registry

*導出元: REQ-S009, REQ-S036*

### DS-207

登録adapterがTestとして発見した実行可能なtest constructはすべて管理対象とする。

### DS-208

発見されたTest集合を `D`、構造上完全なmanaged Test Entity集合を `M` とする。

### DS-209

構造上完全とは、source declarationから構文上有効なTest ID・1件以上の `covers`・その他の必須metadataをTest Entityとして具体化できることをいう。

### DS-210

Discovered Testとentityの対応数は構造完全性に含めず、独立した整合性条件とする。

### DS-211

`M` はVO参照の解決とTest IDの大局的一意性を検査する前の集合とする。

### DS-212

`M` は解決不能な `covers` を持つentityやTest IDが衝突するentityも含む。

### DS-213

完全検証では、発見された各Test dについて、dに対応するmanaged Test Entityがちょうど1件存在し、managed Test Entity.coversが1件以上であり、coversの全VO参照を解決でき、Test IDが発見結果全体で一意であることを要求する（`chain_integrity`。§5.1）。

> ∀ d ∈ D:
>   d に対応する managed Test Entity がちょうど 1 件存在する
>   and managed Test Entity.covers は 1 件以上である
>   and covers の全 VO 参照を解決できる
>   and Test ID が発見結果全体で一意である

### DS-214

違反時の状態は §4.3 に従う。

### DS-215

違反時、いずれも完全検証の `PASS` として扱わない。

### DS-216

発見されたが管理宣言を持たないconstruct（`rust-cargo` では `@vtest` annotation を持たない `#[test]` 等）は診断severityとしてはwarningのままとする。

### DS-217

構造上完全なmanaged Test Entityへ対応しない事実は `chain_integrity` の非 `PASS`（`MISMATCH`/MISSING）として完全検証へ反映する。

### DS-218

診断severityと検証状態を混同しない。

### DS-219

Testの存在理由による分類（role / anchor / anchor_rationale等）と、それに基づく `covers` 件数の可変制約はv0.1では設けない。

### DS-220

すべての管理対象Testに `covers ≥ 1` を一律に要求する。

*導出元: REQ-052, REQ-053, REQ-054, REQ-055, REQ-056, REQ-057, REQ-058, REQ-193, REQ-194, REQ-195, REQ-196, REQ-197, REQ-198, REQ-199, REQ-200, REQ-201, REQ-202*

*引用: 要件定義 §4.1、§13*

### DS-221

VOへの寄与は `covers` 宣言と証拠の十分性判定だけから導出する。

## DS-S039 15 Structured Test Operation

*導出元: P-004, REQ-S039*

### DS-S040 15.2 入力検証

### DS-222

構造化入力の各項目は可能な限り受理時に検証する（対象symbol不在、Test ID不在、参照VO不在等）。

### DS-S041 15.3 編集境界

### DS-223

公式Edit操作の一回の対象は原則1Testとする。

### DS-224

公式Edit操作は暗黙に他Testを変更しない。

### DS-225

編集はadapterが特定した単一のmetadata宣言範囲とTest construct範囲に限定する。

### DS-226

source declarationが正典であるため、`covers` / `targets` の「同期漏れ」は構造的に発生しない。

*導出元: REQ-210, REQ-211, REQ-212, REQ-213, REQ-214, REQ-215, REQ-216, REQ-217*

*引用: 要件定義 §16*

### DS-S042 15.4 Form Schema

### DS-227

Form Schemaの `kind` はrepository内で大局的に一意なForm IDとする。

### DS-228

境界値・partitionの必須入力化は組込Formでは設けない。

### DS-229

境界値・partitionの必須入力化はuser-defined Form Schemaが指定できる。

*導出元: REQ-174, REQ-175, REQ-176, REQ-177, REQ-178, REQ-179, REQ-309, REQ-310, REQ-311, REQ-312, REQ-313, REQ-314, REQ-315, REQ-316, REQ-317, REQ-318, REQ-319, REQ-320, REQ-321, REQ-322, REQ-323, REQ-324, REQ-325, REQ-326, REQ-327, REQ-328, REQ-329, REQ-330, REQ-331, REQ-332*

*引用: 要件定義 §28、§11*

## DS-S043 16. 仕様入力（文書層）

*導出元: REQ-S045*

### DS-230

対象ソースコード内のdoc commentを、その対象実装自身の正当性を証明する唯一の仕様根拠として使用しない。

*導出元: REQ-233, REQ-234, REQ-235*

*引用: 要件定義 §18*

## DS-S044 17. 承認

*導出元: REQ-S018, REQ-S046*

### DS-231

判断済みと承認済みは区別する（判断済み ≠ 承認済み）。

### DS-232

未承認の判断は承認済みより弱い。

*導出元: REQ-236, REQ-237, REQ-238, REQ-239, REQ-240, REQ-241, REQ-242, REQ-243, REQ-244, REQ-245, REQ-246, REQ-247, REQ-248, REQ-249, REQ-250, REQ-251, REQ-252*

*引用: 要件定義 §19*

### DS-233

§11の `UNKNOWN` 判断も承認対象になり得る。

### DS-234

判断できることと正式承認は別段階である。

### DS-235

承認は対象自身の内容だけでなく、承認判断が依存する上流文書・上位VOの現在の依存closureへ束縛する。

### DS-236

VOの依存closureは、再帰的な上位VO・参照するdocument（およびその上位document）からなる。

### DS-237

対象またはいずれかの依存成果物が変更された承認を、現在の承認済み状態として利用してはならない。

### DS-238

変更後は現在状態に対して検証を再実施する。

### DS-239

変更後の検証結果は§4.1の5状態のいずれかに従う。

### DS-240

依存closureまたはハッシュを欠く承認を推測で有効化してはならない。

### DS-241

承認レコードは読み取り互換のため保持できるが、現在の承認済みを導出してはならない。

### DS-242

承認記録の根拠は任意（optional）に記録できる。

### DS-243

承認記録は§11の判断記録と同一entityであることを要求しない。

### DS-244

承認主体を人間に限定しない。

### DS-245

全Agentが承認権限を持つことは要求しない。

### DS-246

一般作業Agentが承認権限を持つべきとも要求しない。

### DS-247

承認は検証状態と独立の別軸である（§4.5）。

### DS-248

承認済みを理由に非 `PASS` を `PASS` へ昇格させない。

### DS-249

未承認を理由に `PASS` を降格させない。

## DS-S045 18 途中導入と既存プロジェクト対応

*導出元: R-5, REQ-S040*

### DS-S046 18.1 既存資産の可視化

### DS-250

未登録Test・欠落する宣言・未確定のVO・未実施の検査または実行を検証済みとして扱わない（状態は§4.3）。

### DS-251

`vtest init` は既存コードを変更しない。

### DS-252

`vtest verify` は正典または検証事実の欠落を対応する非 `PASS` 値として表示する。

### DS-253

`vtest verify` は部分的な登録・判断・実行状態を総合 `OK` として扱わない。

### DS-S047 18.2 導入時の責務境界

### DS-254

本システムが意味判断・候補生成を行うことを必須要件としない。

### DS-255

外部AI/Agentの能力を検証成立条件にしない。

*導出元: REQ-224, REQ-225, REQ-226, REQ-227*

*引用: 要件定義 §17.2*

## DS-S048 19. トレーサビリティと役割別 projection

*導出元: REQ-S007*

### DS-256

契約上必須と定義したリンク（`parent --relation--> child`）は必須とする。

### DS-257

任意（optional）と定義した関係（例：§9.3実装traceability）は欠落してよい。

### DS-258

存在するリンクに付す説明文・導出理由は任意とする。

### DS-259

存在するリンクに付す説明文・導出理由は空でも `chain_integrity` 違反・`MISMATCH` としない。

## DS-S049 20. フェーズゲートと進行条件

*導出元: REQ-S057*

### DS-260

検証状態と承認は独立の軸である（§4.5）。

### DS-261

本システムの責務はゲート条件が現在満たされているかの評価・提示に限る。

### DS-262

フェーズのライフサイクル管理・工程の自動遷移は責務外とする。

*導出元: REQ-288, REQ-289, REQ-290, REQ-291, REQ-292, REQ-293, REQ-294, REQ-302, REQ-303, REQ-304, REQ-305, REQ-306, REQ-307, REQ-308*

*引用: 要件定義 §26.4、§25 OOS-004*

### DS-263

「Releaseフェーズへ遷移させる」のではなく「Release gateの条件を現在満たしている」を提示する。

## DS-S050 21 テスト実行と Execution Evidence

*導出元: REQ-S019, REQ-S054*

### DS-S051 21.1 Evidence の鮮度（ハッシュ束縛による設計制約）

### DS-264

鮮度は独立検査ではなく§6のハッシュ束縛により満たす。

### DS-265

Evidenceは、記録時のTest subject内容ハッシュの一致、target参照集合の一致、各target内容ハッシュの一致、adapter IDの一致、HEAD revisionの一致、およびExecution State subjectの一致をすべて満たす場合のみ有効とする。

*導出元: REQ-115, REQ-116, REQ-117, REQ-118, REQ-119, REQ-120*

*引用: 要件定義 §6*

### DS-266

Evidenceの記録時のTest subject内容ハッシュが現在と一致する。

### DS-267

Evidenceのtarget参照集合が、現在のTestの宣言targetを解決したcanonical Source Target集合と重複なく一致する。

### DS-268

Evidenceの記録時の各target内容ハッシュが、現在解決される各implementation constructの内容ハッシュと一致する。

### DS-269

Evidenceのadapter IDが現在のTestのexecution adapterと一致する。

### DS-270

Evidenceの記録時のHEAD revisionが特定され、現在のHEAD revisionと一致する。

### DS-271

Execution State subjectが完全であり、現在再構築したExecution State subjectと一致する（dirty状態のsource、target外helper、build script、local dependency、runner / toolchain / 実行影響configの変更を含む）。

### DS-272

内容ハッシュ・Execution State subject・revision条件を満たさないEvidenceは `NO_EVIDENCE`（診断STALE）とする。

### DS-273

内容ハッシュ・Execution State subject・revision条件を満たさないEvidenceは有効な `PASS` として扱わない。

### DS-274

adapterが実行入力集合の完全性を証明できない場合は `UNKNOWN` とする。

### DS-275

部分的snapshotから現在実装への `PASS` を推測しない。

### DS-276

Evidenceが存在しても鮮度が満たされないなら、そのEvidenceから実行関連の判定を `PASS`/`FAIL` として再利用しない。

### DS-277

Evidenceが存在しても鮮度が満たされないなら、同じ鮮度・対応関係の非 `PASS` 値を保持する。

### DS-278

Evidenceが存在しない場合は実行関連を `NO_EVIDENCE`（診断NOT_EXECUTED）とする。

### DS-279

Evidence readerは、Rust実行と一意に確認できない場合は `UNKNOWN` とする。

## DS-S052 22 完全検証・集約・報告

*導出元: REQ-S053*

### DS-S053 22.1 完全検証 OK

### DS-280

完全検証におけるOKは、宣言鎖全体に対する検査（`chain_integrity` / `orphan_detection`）と、scopeに含まれる各「宣言＋コード＋証拠」の組に対する検査（`target_binding` / `oracle_presence`）がすべて `PASS` であり、テストランナーの結果を含む証拠が§6を満たす場合に限る。

### DS-281

一項目でも `FAIL` / `MISMATCH` / `NO_EVIDENCE` / `UNKNOWN` であればNGとする（fail-closed）。

*導出元: P-002, REQ-295, REQ-296*

*引用: 要件定義 §26.1、P-002*

### DS-282

利用者向け簡易出力は `OK` / `NG` の二値とする。

### DS-283

完全検証の検査集合はこの4検査に固定する。

### DS-284

完全検証の検査集合は設定によって追加・削除できない。

### DS-285

検査の部分集合を指定した実行は完全検証として表示・集約しない（§4.6）。

### DS-S054 22.2 集約

### DS-286

集約はfail-closedを基本とする。

### DS-287

子に1つでも非 `PASS` があれば親は非 `PASS` とする。

*導出元: REQ-299, REQ-300, REQ-301*

*引用: 要件定義 §26.3*

### DS-288

集約時に複数の非 `PASS` 値が混在する場合、上位に表示する代表値の優先順位は `FAIL > MISMATCH > NO_EVIDENCE > UNKNOWN` とする。

### DS-289

診断ラベルは代表値の順位に用いず、原因説明として併記する。

### DS-S055 22.3 報告

### DS-290

簡易出力は総合OK / NGとする。

### DS-291

`covers` を持つTestはVOの子として表示する。

### DS-292

adapter能力の欠落・失敗を `PASS` へ補完しない。

### DS-293

static解析またはcoverage能力がなければ該当項目は `NO_EVIDENCE`（診断NOT_CHECKED）とする。

### DS-294

runner能力がなければ実行関連は `NO_EVIDENCE`（診断NOT_EXECUTED）とする。

### DS-295

解析限界は `UNKNOWN` とする。

### DS-296

create / edit / audit / run等の明示的操作に必須の能力がなければ操作を失敗させる。

### DS-297

create / edit / audit / run等の明示的操作に必須の能力がなければファイル・判断記録・Evidenceを生成しない。

*導出元: REQ-258, REQ-259, REQ-260, REQ-261, REQ-262, REQ-263, REQ-264, REQ-265, REQ-266*

*引用: 要件定義 §21*

## DS-S056 23. スキャンと整合性検査

*導出元: REQ-S036, REQ-S050*

### DS-298

Test IDの重複（identity collision）は `MISMATCH` とする。

### DS-299

`covers` が存在しないVOを参照する場合（dangling reference）は `MISMATCH` とする。

### DS-300

Test IDを宣言するが `covers` をどのVOも参照しないTest（orphan test）は `MISMATCH` とする。

### DS-301

すべての管理対象Testに `covers ≥ 1` を一律要求する（§12）。

### DS-302

VOのparentが存在しない、または循環している場合は `MISMATCH` とする。

### DS-303

VOの `derives_from`（document参照）が存在しないdocumentを参照する場合は `MISMATCH` とする。

### DS-304

documentのderives_fromが存在しないdocumentを参照する場合（文書鎖のリンク切れ）は `MISMATCH` とする。

### DS-305

根に指定されない孤児document（`orphan_detection`）は `MISMATCH` とする。

### DS-306

Relationのfrom / toが存在しないエンティティを参照する場合は `MISMATCH` とする。

### DS-307

恒久SRC IDがadapter境界を越えて重複する場合は `MISMATCH` とする。

### DS-308

必須Test metadataの欠落は `MISMATCH` とする。

### DS-309

adapterがTestとして発見したが管理宣言を持たないconstruct（unregistered test）は診断severityをwarningとする。

### DS-310

managed Test Entityへ対応しない事実は `chain_integrity`（`MISMATCH`/MISSING）へ反映する（§12）。

### DS-311

エラーは検証結果に反映され、該当エンティティの検査を非 `PASS` にする。

### DS-312

診断severityと検証状態を混同しない。

### DS-313

content_hash照合は決定論的に解決する。

### DS-314

content_hash照合は任意形式の文書本文から参照位置の存在を構文的に推測しない。

### DS-315

参照位置の意味的妥当性・取り込み完全性は検査対象としない。

### DS-316

参照位置の意味的妥当性・取り込み完全性は必要ならエスカレーション（§11）で扱う。

## DS-S057 24 データ保存の基本方針

*導出元: REQ-S050, REQ-S058*

### DS-S058 24.2 並列編集耐性の設計原則

### DS-317

Relationレコードは不変とする。

### DS-318

record / エンティティファイルの書込みは読み手に書きかけの部分状態を観測させない。

### DS-319

並列編集耐性は「公開されたファイルは常に完全である」ことを前提とする。

### DS-320

並列編集耐性では部分書込みの検出・修復は行わない。

### DS-S059 24.3 派生情報の再構築

### DS-321

検証グラフ、逆引きインデックス、集約結果はすべて正典からの導出物である。

## DS-S060 26 インターフェース概要

*導出元: REQ-S049, REQ-S058*

### DS-S061 26.1 CLI コマンド体系

### DS-322

終了コードは `0`＝要求scopeがOK、`1`＝検証NG、`2`＝入力・adapter前提・capability等による操作拒否、`3`＝内部エラーとする。

### DS-323

フェーズゲートを指定した実行（§20）では、`0` / `1` は当該ゲートの充足・不充足を表す。

### DS-324

ゲート充足は検証状態とは別軸の評価である。

### DS-325

ゲート充足は検証状態を書き換えない。

### DS-326

出力では検証状態とゲート満否を別に提示する。

### DS-327

ゲート指定時の `0` を検証状態 `PASS` と読ませない。

### DS-328

検証状態と内部エラーは終了コードで分離する（§4.4）。

### DS-329

CIはこの終了コードのみで判定できる。

### DS-S062 26.2 MCP ツール体系

### DS-330

すべてのツールは非対話で完結する。

*引用: 要件定義 NFR-007*

## DS-S063 27. 対応範囲と adapter 境界

*導出元: R-2, R-3, REQ-S048*

### DS-331

検証契約・ID・ハッシュ・Evidence・状態・集約の概念モデルは、言語およびtest runnerに依存しない。

*導出元: R-3, REQ-258, REQ-259, REQ-260, REQ-261, REQ-262, REQ-263, REQ-264, REQ-265, REQ-266*

*引用: 要件定義 §21、R-3*

### DS-332

共通契約は特定言語の構文・構造を必須としない。

### DS-333

adapterが未登録・能力不足・解析不能の場合、検証結果を推測で `PASS` へ昇格してはならない。

## DS-S064 29. スコープ外

*導出元: REQ-S052*

### DS-334

文書層は§2.2の通りリンクとハッシュのみを扱う（OOS-001仕様書同士の品質監査）。

### DS-335

文書内容の意味的良否を検証しない（OOS-001仕様書同士の品質監査）。

### DS-336

不一致はどれを正とするか決めず状態として提示する（OOS-002修正方針決定。§4）。

*導出元: P-001*

*引用: P-001*

### DS-337

Test Edit対象外の一般編集を管理しない（OOS-003通常ソースコード編集管理。§15.3）。

### DS-338

フェーズのライフサイクル管理・工程遷移は責務外とする（OOS-004開発プロセス全体の管理。§20）。

## DS-S065 1. 実装構成

### DS-S066 1.2 主要依存クレート

*導出元: SPEC-S063*

### DS-339

`git` が利用できない場合、リビジョンは特定できず、当該 Evidence はハッシュ束縛（revision 一致）を満たさないため `target_binding` の証拠として有効な `PASS` にならない（fail-closed）（§6）。

### DS-340

`git` が利用できない場合の失効は独立検査ではなく診断ラベル `STALE` として説明する（§11.2）。

### DS-S067 1.3 内容ハッシュの定義

*導出元: SPEC-S002, SPEC-S007, SPEC-S026, SPEC-S027, SPEC-S051*

### DS-341

canonical metadataの `targets` は宣言された `TargetRef` の正規化値を束縛し、解決後のcanonical Locatorへ置換しない。

### DS-342

canonical metadataの `targets` が宣言された `TargetRef` の正規化値を束縛し、解決後のcanonical Locatorへ置換しないことにより、Testの参照方法の変更（同一Source Targetへのlocator参照からSRC ID参照への書き換え等）はTest subject hashで捕捉される（§6.1.1）。

### DS-343

検証対象は一般概念であり、このhashは検証対象をSource Targetとして実現した形態のidentity束縛であって、coreが「検証対象とは何か」をSource Targetに限定して定義するものではない（§1.3・§4.1）。

*導出元: SPEC-082, SPEC-083, SPEC-084*

*引用: 基本仕様 §9.1*

### DS-344

恒久SRC IDはhash inputの独立fieldとして束縛せず、canonical Target Reference経由でもhash inputへ入らない。

### DS-345

Evidence、検証は解決後のcanonical Source Targetのcanonical Locatorとhashへ束縛し、addressing modeごとに別subjectを作らない（§6.1）。

### DS-346

document recordの `content_hash` と実sourceが不一致ならsubject hashは現在有効な値として成立せず、`chain_integrity` の非 `PASS`（`MISMATCH`、診断 `STALE`）とする（§11.4）。

### DS-347

静的解析結果は内容ハッシュに束縛された永続subjectを持たず、hash体系に静的解析専用のsubjectを設けない。

### DS-348

format変更を構文上の意味だけから同値とみなさず、正規化後のsource bytesが変化した場合は安全側でSTALEにする。

## DS-S068 2. データディレクトリと設定

### DS-S069 2.2 `config.yaml`

*導出元: SPEC-S012, SPEC-S019, SPEC-S050, SPEC-S053*

### DS-349

`config.yaml` の各adapterの `scan` 設定の `include` はテストコード走査パスであり、省略時はワークスペース全体を対象とする。

### DS-350

`config.yaml` の各adapterの `scan` 設定の `assertion_macros` は、追加でassert相当として扱うマクロ名を指定する。

### DS-351

`config.yaml` の各adapterの `run` 設定の `coverage` は `target_binding` の動的計測方式を指定し、値は `llvm-cov` または `off` である。

### DS-352

adapter IDの重複、同一adapter内のroot重複、未知adapter、無効なadapter設定はusage error（E-CONFIG-001）とする。

### DS-353

異なるadapterが同じrootを共有することはpolyglot repositoryのために許可する。

### DS-354

coreは未知のnamespaceや値をRust設定として解釈しない。

### DS-355

`verify.full_scope` は利用者が完全検証を縮小する設定ではなく、基本仕様 §5 の固定4検査（`chain_integrity` / `orphan_detection` / `target_binding` / `oracle_presence`）を列挙するconfig invariantである。

*導出元: SPEC-053, SPEC-054, SPEC-055, SPEC-056, SPEC-057, SPEC-058, SPEC-059, SPEC-060, SPEC-061, SPEC-062, SPEC-063, SPEC-064, SPEC-065, SPEC-066, SPEC-067, SPEC-068, SPEC-069, SPEC-287, SPEC-288, SPEC-289, SPEC-290, SPEC-291, SPEC-292, SPEC-293*

*引用: 基本仕様 §5*

### DS-356

version 2では、`verify.full_scope` の重複・未知項目・欠落・余剰をE-CONFIG-001で拒否する。

### DS-357

version 1では、`verify.full_scope` のfield欠落を固定4検査として具体化し、重複または未知項目はE-CONFIG-001で拒否する。

### DS-358

旧12項目の列挙（`spec_coverage` / `test_existence` 等）は現行invariantに違反するため、versionを問わずE-CONFIG-001とし、in-memory補完で受理しない。

### DS-359

`--items` による明示的な部分集合だけを限定scopeとして扱い、項目指定を省略したCLI / MCP検証は常に固定4検査を評価する。

### DS-360

いかなる設定値も完全検証の検査を4本未満へ縮退させない（§22.1）。

*引用: 基本仕様 §4.6*

### DS-361

config読込み時に次を検査し、いずれか違反があればE-CONFIG-001（終了コード2）として設定を受理せず、検証結果を生成しない。

### DS-362

`gates` field自体の欠落と空listは「ゲート定義なし」として受理する。

### DS-363

`--gate` を指定しない実行は、`gates` field自体の欠落と空listの影響を受けない。

### DS-364

`--gate` に未定義名を指定した場合の扱いは §11.5 で定める。

### DS-365

`gates[].name` は非空文字列であり、大文字小文字を区別した完全一致で重複してはならない。重複した場合はE-CONFIG-001（終了コード2）とする。

### DS-366

`--gate <name>` の解決は `gates[].name` と同じ大文字小文字を区別した完全一致で行う（§11.5）。

### DS-367

`gates[].require` は必須とする。欠落はE-CONFIG-001（終了コード2）とする。

### DS-368

`gates[].require` の `verification` は必須とする。欠落はE-CONFIG-001（終了コード2）とする。

### DS-369

`require.verification` の値は、基本仕様 §4.1 の5状態語彙（`PASS` / `FAIL` / `MISMATCH` / `NO_EVIDENCE` / `UNKNOWN`）のいずれかと大文字小文字を区別して完全一致しなければならない。

*導出元: SPEC-046*

*引用: 基本仕様 §4.1*

### DS-370

診断ラベル（`MISSING` / `NOT_EXECUTED` / `NOT_CHECKED` / `STALE`）、`OK` / `NG`、旧12項目名、5状態の小文字表記・別綴り、list・objectなどの非文字列値を `require.verification` に指定した場合はすべてE-CONFIG-001（終了コード2）とする。

### DS-371

5状態のうち `PASS` 以外を要求する `require.verification` の定義自体は受理し、充足判定の意味は §11.5 で定める。

### DS-372

`require.approvals` は省略可能とし、省略は「要求する承認ロールなし（空集合）」として受理する。

### DS-373

`require.approvals` を指定する場合は文字列ロール名のlistとし、空文字列・重複ロール名はE-CONFIG-001（終了コード2）とする。

### DS-374

`require.approvals` のロール名が `approval_roles` に解決できない場合もE-CONFIG-001（終了コード2）とする。

*引用: 別紙A §12.3*

### DS-375

非Rust namespaceの値をcoreがRust設定として推測・書換えしてはならない。

## DS-S070 3. レコードファイルスキーマ

### DS-376

レコードの未知フィールドはエラーではなく警告とする。

### DS-377

`id` とファイル名（拡張子除く）は一致しなければならない。

### DS-S071 3.1 document レコード（`.verify/doc/DOC-*.yaml`）

*導出元: SPEC-S008, SPEC-S009, SPEC-S044*

### DS-378

document レコードの `title` fieldは任意の表示名である。

### DS-379

document レコードの `derives_from` fieldは上流documentへの導出リンクであり、0件も許容する（0件は根候補を意味する）。

### DS-380

document レコードの `derives_from` entryの `anchor` fieldは任意の上流該当箇所（節番号等）であり、空も許容し、`chain_integrity` の `MISMATCH` としない。

### DS-381

document レコードの `derives_from` entryの `note` fieldは任意の導出理由であり、空も許容し、`chain_integrity` の `MISMATCH` としない。

*導出元: SPEC-281, SPEC-282, SPEC-283*

*引用: 基本仕様 §3.4*

### DS-382

各 `derives_from` entryの `note`（導出理由・説明文）は任意であり、空でも `chain_integrity` 違反・`MISMATCH` としてはならない（§19）。

*導出元: SPEC-281, SPEC-282, SPEC-283*

*引用: 基本仕様 §3.4*

### DS-383

各 `derives_from` entryの `anchor`（参照先document内の該当箇所を指す文字列。節番号・条項番号・見出し等）は任意であり、欠落・空文字列を `chain_integrity` 違反・`MISMATCH` としてはならない（§19）。

*導出元: SPEC-281, SPEC-282, SPEC-283*

*引用: 基本仕様 §3.4*

### DS-384

`anchor` の値は不透明な文字列として保存・出力するだけであり、本システムは `anchor` を `path` の実ファイル内位置へ解決せず、実在・一意性・書式を検証しない。

### DS-385

`anchor` の内容不一致を検出する検査・診断コードは存在しない。

### DS-386

同一 `doc` を指す複数entryを `anchor` 違いで持つことを許容し、重複としない。

### DS-387

`anchor` だけの変更は `path` の実ファイルを変えないため `content_hash` を変化させないが、document subject hashを変化させるため、当該documentを上流依存closureに含む判断記録・承認は失効する（§3.5・§8.5）。

### DS-388

`derives_from` の参照先documentが存在しない場合は、文書鎖のリンク切れとして `chain_integrity` の `MISMATCH` とする。

### DS-389

`path` の実ファイルが `content_hash` と一致しなくなった場合は `chain_integrity` の `MISMATCH`（診断 `STALE`）とする（§11.4）。

### DS-390

`derives_from` が空のdocumentは根候補であり、`config.yaml` の `doc.roots` に列挙されない場合は孤児として `orphan_detection` の `MISMATCH` とする（§5.6）。

### DS-S072 3.2 VO レコード（`.verify/vo/VO-*.yaml`）

*導出元: SPEC-S009, SPEC-S030*

### DS-391

VO レコードの `derives_from` fieldは1件以上のdocumentへの直結を表す。

*導出元: SPEC-043, SPEC-044, SPEC-279, SPEC-280*

*引用: 基本仕様 §3.2*

### DS-392

VO レコードの `derives_from` entryの `anchor` fieldは任意の上流該当箇所（節番号等）であり、空も許容し、`chain_integrity` 違反・`MISMATCH` としない。

### DS-393

VO レコードの `derives_from` entryの `note` fieldは任意であり、空も許容し、`chain_integrity` 違反・`MISMATCH` としない。

### DS-394

VO レコードの `dimensions` fieldは検証軸であり、任意である（§3.2.1）。

### DS-395

VO レコードの `representative_cases` fieldは代表入力値であり、任意である。

### DS-396

VOは1件以上の `document` から `derives_from` で導出される。

### DS-397

`derives_from` の参照先documentが存在しなければ、`chain_integrity` の `MISMATCH`（dangling reference。E-SCAN-003相当はE-SCAN-012）とする（§5.4）。

### DS-398

VOの `derives_from` entryもdocumentレコードと同じく任意の `anchor`（参照先document内の該当箇所を指す不透明な文字列。節番号・条項番号・見出し等）と任意の `note` を持つ。

### DS-399

VOの `derives_from` entryの `anchor` / `note` の欠落・空文字列は `chain_integrity` 違反・`MISMATCH` としない（§19）。

*導出元: SPEC-281, SPEC-282, SPEC-283*

*引用: 基本仕様 §3.4*

### DS-400

本システムはVOの `anchor` を文書内位置へ解決せず、実在・一意性・書式を検証せず、内容不一致を検出する検査を持たない。

### DS-401

同一 `doc` を `anchor` 違いで複数entryとして持つことを許容し、重複としない。

### DS-402

`anchor` と `note` はVO subject hashの入力に含まれない（VO subject hashは `derives_from` の参照先document ID集合を束縛する）（§1.3）。

### DS-403

`anchor` だけの変更でVOの承認・判断記録は失効しない。

### DS-404

参照先document集合そのものの変更はVOの承認・判断記録を従来どおり失効させる。

### DS-405

readerは読取り互換fieldとして `status` を受理するが、実効判定とVO subject hashでは無視し、存在自体をW-STORE-001として通知する。

### DS-406

互換field値と導出値が異なる場合も導出値だけを使用する。

#### DS-S073 3.2.1 dimensions と組合せの実体化

*導出元: SPEC-S030*

### DS-407

`independent-axes` はpartitionごとに1子VOを生成する（上例では2+4=6件）。

### DS-408

`full-product` は直積ごとに1子VOを生成する（上例では8件）。

### DS-409

`explicit` は `combinations` フィールドに列挙された組合せのみを生成する。

### DS-410

生成される子VOのIDは `VO-X-<PARTITION>`（直積は `VO-X-<P1>-<P2>`）を既定とする。

### DS-411

子VO IDのsuffixはpartition値を大文字化した文字列とし、複数軸のsuffixは `dimensions` の宣言順に連結する。

### DS-412

子VO ID生成は `combinations` entry内の記述順・map key順には依存しない。

### DS-413

実体化後は通常のVOとして扱われるため、`chain_integrity` のleaf VO → Test検査は「leaf VOにcoversするTestが存在するか」だけを見ればよい。

### DS-414

`combinations` の各entryはdimension名→partition値のmapとし、`dimensions` に宣言された全軸をちょうど1回ずつ持つ。

### DS-415

上例の `explicit` 実体化は `VO-X-POSITIVE-DIV` と `VO-X-NEGATIVE-DIV` の2件を生成する。

### DS-416

`vo expand` は子VOを1件も生成せず、部分生成もしない。

### DS-417

`coverage_policy: explicit` かつ `combinations` が欠落、`null`、または空listである場合、当該VOレコードは `combinations` 不正とし、`E-SCAN-017` を報告して当該VOの `chain_integrity` を `MISMATCH` とする（§17.1）。

### DS-418

`coverage_policy: explicit` かつ `dimensions` が空である場合、当該VOレコードは `combinations` 不正とし、`E-SCAN-017` を報告して当該VOの `chain_integrity` を `MISMATCH` とする（§17.1）。

### DS-419

`combinations` が空listでないのに `coverage_policy` が `explicit` 以外（`independent-axes` / `full-product` / `null`）である場合、当該VOレコードは `combinations` 不正とし、`E-SCAN-017` を報告して当該VOの `chain_integrity` を `MISMATCH` とする（§17.1）。

### DS-420

entryが `dimensions` に宣言されていないdimension名を含む場合、当該VOレコードは `combinations` 不正とし、`E-SCAN-017` を報告して当該VOの `chain_integrity` を `MISMATCH` とする（§17.1）。

### DS-421

entryのpartition値が当該dimensionの `partitions` に列挙されていない場合、当該VOレコードは `combinations` 不正とし、`E-SCAN-017` を報告して当該VOの `chain_integrity` を `MISMATCH` とする（§17.1）。

### DS-422

entryが宣言済みdimensionのいずれかを欠く、または同じdimension名を2回以上持つ場合、当該VOレコードは `combinations` 不正とし、`E-SCAN-017` を報告して当該VOの `chain_integrity` を `MISMATCH` とする（§17.1）。

### DS-423

同一の（dimension名→partition値）対応を持つentryが2件以上ある（重複tuple）場合、当該VOレコードは `combinations` 不正とし、`E-SCAN-017` を報告して当該VOの `chain_integrity` を `MISMATCH` とする（§17.1）。

### DS-424

`combinations` の変更は当該VOの承認を失効させる（§3.5）。

### DS-S074 3.3 Relation レコード（`.verify/rel/REL-<ULID>.yaml`）

*導出元: SPEC-S006, SPEC-S009*

### DS-425

Relationレコードの `from` fieldは任意のエンティティIDである。

### DS-426

Relationレコードの `note` fieldは任意の説明文である。

### DS-427

canonical Relation IDは `REL-` と26文字のULID payloadからなる。

### DS-428

prefixed / bareの混在、ファイル名と `id` のpayload不一致、または同じpayloadの複数recordはE-SCAN-010とし、いずれかを選ばない。

### DS-429

`from` / `to` の存在はスキャン時に検査し、不在はE-SCAN-009、`chain_integrity` の `MISMATCH` とする。

### DS-S075 3.4 判断記録レコード（`.verify/decisions/<ULID>.yaml`）

*導出元: REQ-S035, SPEC-S034*

### DS-430

判断記録は actor / subject / decision / judgment_kind を必須項目とする。

### DS-431

判断記録の理由・根拠は任意とする。

### DS-432

判断記録は依存closureのハッシュに束縛される。

### DS-433

判断記録の `judgment_kind` fieldは判断型であり、必須とする（§8.1）。

### DS-434

判断記録の `supersedes` fieldは明示に置き換える旧判断記録のULID listであり、既定は空listとする（§8.5）。

### DS-435

判断記録の `dependencies` fieldは判断時点の上流依存closureであり、完全一致を要求する。

### DS-436

判断記録の `actor` fieldは誰が判断したかを表し、必須とする。

### DS-437

判断記録の `actor` の `model` fieldはagentの場合任意とする。

### DS-438

判断記録の `decision` fieldはどう判断したかを表し、必須とする。値の妥当性は §8.4 で定める。

### DS-439

判断記録の `reason` fieldは理由・根拠・evidence noteであり、任意とし、空でも無効化しない。

### DS-440

判断記録の `exclusions` fieldは対象外とした範囲であり、任意とする。

### DS-441

理由が空であることだけを根拠に、その判断を無効・`UNKNOWN`・`NO_EVIDENCE`・`MISMATCH` 等として扱ってはならない。

*導出元: REQ-180, REQ-181, REQ-182, REQ-183, REQ-184, REQ-185, REQ-186, REQ-187, REQ-188, REQ-189, REQ-190, REQ-191, REQ-192, SPEC-116, SPEC-117, SPEC-118, SPEC-119, SPEC-120, SPEC-121, SPEC-122, SPEC-333, SPEC-334, SPEC-335*

*引用: 基本仕様 §11.3, 要件定義 §12*

### DS-442

`reason` / `exclusions` はoptionalである。

### DS-443

同一対象への判断記録は複数存在してよい（再判断・多重判断）。

### DS-444

判断記録の有効性判定と実効判断の決定は §8.5 に従う。

### DS-445

実効判断は `subject` 単独ではなく `(subject, judgment_kind)` の組ごとに独立に決まり、判断型の異なる判断記録どうしは競合しない（§8.5）。

### DS-446

`judgment_kind` を欠くか §8.1 の値域外の判断記録は、履歴として保持するがいずれの `(subject, judgment_kind)` の実効判断へも寄与させず、W-STORE-003を出す。

### DS-447

`supersedes` に列挙する各ULIDは、同一 `subject` かつ同一 `judgment_kind` の既存判断記録を指さなければならない。提出時の検証は §8.4、読取り時の扱いは §8.5 に従う。

### DS-448

`supersedes` はRelationとは独立であり、`type: supersedes` のRelationレコードは実効判断の決定に用いない（§3.3）。

### DS-449

`subject` の `target` 参照は §6.1 で解決したcanonical Source Targetのcanonical Locatorとし、解決できないtargetを任意の候補で埋めない（§6.1.1）。

### DS-450

判断記録の受理は当該対象の検証状態（§4.1の5状態）を昇格させない（§8.3）。

*導出元: SPEC-116, SPEC-117, SPEC-118, SPEC-119, SPEC-120, SPEC-121, SPEC-122, SPEC-333, SPEC-334, SPEC-335*

*引用: 基本仕様 §11.3*

### DS-S076 3.5 承認レコード（`.verify/approvals/<ULID>.yaml`）

*導出元: REQ-S046, SPEC-S016, SPEC-S043, SPEC-S066*

### DS-451

承認済みを理由に非 `PASS` を `PASS` へ昇格させない。

### DS-452

未承認を理由に `PASS` を降格させない。

### DS-453

`judgment_ref` が指す判断記録が存在しない場合は、書込み時にE-APPROVAL-001として拒否する。既存レコードとして読み取った場合は当該レコードからVO / documentの実効承認も判断記録の実効承認も導出しない（W-STORE-006）。

### DS-454

判断記録ULIDを `subject` に持つ承認レコードは、書込み時にE-APPROVAL-002として拒否する。既存レコードとして読み取った場合は履歴表示だけを許可していかなる実効承認も導出せず、W-STORE-006を出す。

### DS-455

VO ID・document IDのいずれにも解決しない `subject`（Test ID、Source Target locator、Relation ID等）も、判断記録ULIDを `subject` に持つ承認レコードと同じ扱いとする。

### DS-456

いずれの種別でも対象自身は `subject_hash` で束縛するため `dependencies` へ重複して含めない。

### DS-457

document dependencyはdocument subject hashを使用するため、document recordまたは参照先sourceの変更で承認が失効する（§1.3）。

### DS-458

`approved_state` が値域外の他の値の場合、書込み時にE-APPROVAL-002として拒否する。既存レコードとして読み取った場合は履歴表示だけを許可していかなる実効承認も導出せず、W-STORE-006を出す。

### DS-459

`approved_state` の値 `approved` は、この内容で進めることを認めたこと（承認）を意味する。

### DS-460

`approved_state` の値 `rejected` は、この内容で進めることを認めないこと（却下）を意味する。

### DS-461

`approved_state` の値 `withdrawn` は、先に与えた承認を取り消したこと（承認取消）を意味する。

### DS-462

参照先を解決できない、対象が一致しない、または自己参照する `supersedes` entryを含むレコードは、書込み時にE-APPROVAL-002として拒否する。

### DS-463

既存レコードとして読み取った場合、およびsupersede関係が循環する場合は、当該レコードを実効集合へ寄与させずW-STORE-005を出す。

### DS-464

承認レコードの `supersedes` はRelationとは独立であり、`type: supersedes` のRelationレコードは実効承認の決定に用いない（§3.3）。

### DS-1588

A’(X) が空の場合、実効承認状態は `draft` とする。

### DS-465

`approved_at` / ULIDの順序、レコードの新旧、件数の多寡のいずれも、採用する承認レコードを選ぶ規則に用いてはならない。

### DS-466

検証状態（§4.1の5状態）の変化、判断記録の追加そのもの、`basis` の内容は、実効承認状態を変えない。

### DS-467

依存entryを持たない互換Approvalは読取りと履歴表示だけを許可し、現在の `approved` を導出しない。W-STORE-002を出し、対象は `draft` 相当とする。

### DS-468

承認記録は「誰が（approver）」「何を（subjectまたはjudgment reference）」「どの承認状態か（approved_state）」を必須項目として追跡可能とし、根拠は任意に記録できる。

### DS-469

承認主体は種別（`human` / `agent`）と識別子を記録する。

### DS-S077 3.6 Evidence レコード（`.verify/evidence/<ULID>.yaml`）

*導出元: SPEC-S022, SPEC-S051*

### DS-470

`result` はテストランナー（判定権威）が報告した合否をそのまま記録する（§7）。

### DS-471

本システムは合否を再判定せず、`result` を `target_binding` の証拠として消費する。

*導出元: SPEC-070, SPEC-071, SPEC-072, SPEC-073, SPEC-074, SPEC-075, SPEC-076, SPEC-077, SPEC-295, SPEC-296, SPEC-297, SPEC-298, SPEC-299, SPEC-300, SPEC-301, SPEC-302, SPEC-303*

*引用: 基本仕様 §7*

### DS-472

有効なEvidenceの `result: FAIL` は `target_binding = FAIL`（テストランナーが失敗を報告）へ至る（§11.2）。

*導出元: REQ-094, REQ-095, REQ-096, REQ-097, REQ-098, REQ-099, REQ-100, REQ-101, REQ-102, REQ-103, REQ-104*

*引用: 要件定義 §5.3*

### DS-473

`target_coverage.checked: false` では `method` と `result` をnull、`targets` を空listとし、`target_binding` の動的計測を `NO_EVIDENCE`（診断 `NOT_CHECKED`）として扱う（§11.2）。

### DS-474

repository内helperだけの変更もmanifest hashを変化させる。

### DS-475

Evidence readerは `execution_state` を欠く互換recordを履歴表示できるが、現在のEvidence freshnessを証明できないため `NO_EVIDENCE`（診断 `STALE`）とする。

### DS-476

schema違反、target entryの欠落・重複・余剰、またはaggregate resultとtarget別結果の矛盾はE-SCAN-010として扱い、そのEvidenceを有効な結果に使用しない。

### DS-477

確認不能は `UNKNOWN`、明示adapterの不一致は `MISMATCH` とし、いずれも `PASS` へ昇格しない。

## DS-S078 4. Test metadata宣言contract

### DS-S079 4.1 adapter-neutralな正規化

*導出元: REQ-S026, SPEC-S007, SPEC-S027, SPEC-S028, SPEC-S035*

### DS-478

すべての管理対象Testに `covers ≥ 1` を一律に要求する。

*導出元: REQ-052, REQ-053, REQ-054, REQ-055, REQ-056, REQ-057, REQ-058, SPEC-123, SPEC-124, SPEC-369, SPEC-370, SPEC-371, SPEC-372, SPEC-373, SPEC-374, SPEC-375, SPEC-376, SPEC-377, SPEC-378, SPEC-379, SPEC-380, SPEC-381, SPEC-382, SPEC-383, SPEC-384, SPEC-385, SPEC-386, SPEC-387, SPEC-388, SPEC-389, SPEC-390, SPEC-391, SPEC-392, SPEC-393, SPEC-394, SPEC-395, SPEC-396, SPEC-397, SPEC-398, SPEC-399, SPEC-400, SPEC-401, SPEC-402, SPEC-403, SPEC-404, SPEC-405, SPEC-406, SPEC-407*

*引用: 基本仕様 §12, 要件定義 §4.1*

### DS-479

VOへの寄与は `covers` 宣言と証拠の十分性判定だけから導出する。

### DS-480

検証対象は一般概念であり、adapter中立coreは各管理対象Testに1件以上の検証対象を要求する。

*導出元: REQ-144, REQ-145, REQ-146, REQ-147, REQ-148, SPEC-082, SPEC-083, SPEC-084*

*引用: 基本仕様 §9.1, 要件定義 §9.1*

### DS-481

検証対象は「そのTestが検証成立性を証明しようとする対象＝宣言された『何の時にどうなる』の主語」であって、実装constructに限定しない。

### DS-482

coreの `chain_integrity` は「検証対象をSource Targetとして実現し `targets ≥ 1` を宣言すること」をadapter中立の必須リンクとしない（coreのTest層必須はTest ID・`covers ≥ 1`・その他の必須metadata）（§11.1.1）。

*導出元: SPEC-057, SPEC-058, SPEC-123, SPEC-124, SPEC-369, SPEC-370, SPEC-371, SPEC-372, SPEC-373, SPEC-374, SPEC-375, SPEC-376, SPEC-377, SPEC-378, SPEC-379, SPEC-380, SPEC-381, SPEC-382, SPEC-383, SPEC-384, SPEC-385, SPEC-386, SPEC-387, SPEC-388, SPEC-389, SPEC-390, SPEC-391, SPEC-392, SPEC-393, SPEC-394, SPEC-395, SPEC-396, SPEC-397, SPEC-398, SPEC-399, SPEC-400, SPEC-401, SPEC-402, SPEC-403, SPEC-404, SPEC-405, SPEC-406, SPEC-407*

*引用: 基本仕様 §5.1, 基本仕様 §12*

### DS-483

v0.1の唯一のadapter `rust-cargo` は検証対象をSource Targetとして実現し `targets ≥ 1` を必須とする（§4.2・§4.4・§5.5）。

### DS-S080 4.2 `rust-cargo` annotation文法

*導出元: SPEC-S035, SPEC-S066*

### DS-484

`rust-cargo` の `@vtest.` 宣言表面は2種であり、表面ごとに認識する行形式が異なる。

### DS-485

Test constructのdoc comment（`///` または `/** */`）は表面1であり、test-annotation-lineを認識する。

### DS-486

Test constructではない関数itemのdoc comment（対象実装側の関数等）は表面2であり、source-target-annotation-lineを認識する。

### DS-487

test-annotation-lineの文法は `"@vtest." test-key SP value` である。

### DS-488

source-target-annotation-lineの文法は `"@vtest." source-target-key SP value` である。

### DS-489

test-keyの値域は `id` / `covers` / `target` / `intent` / `input` / `expect` / `kind` / `case` / `related` である。

### DS-490

source-target-keyの値域は `src-id` である。

### DS-491

valueは行末までのテキストとし、前後空白は除去する。

### DS-492

annotation行は1行1キーとする。

### DS-493

`covers` と `related` の値はカンマ区切りで複数指定できる。

### DS-494

`case` と `related` はキー自体を複数行書ける。

### DS-495

`case` と `related` 以外のキーの重複はエラーE-SCAN-005とする。

### DS-496

ただし `kind` がintegration系のTestに限り、`target` の複数行を許容する。

*引用: 別紙A §14.3*

### DS-497

許容された複数 `target` 内でも同じTargetRefの重複はE-SCAN-005とする。

### DS-498

綴りが異なっても解決後に同一canonical Source Targetへ到達する複数宣言（同じSource Targetへのlocator参照とSRC ID参照の併記等）も、coreが解決時にE-SCAN-005とする（§6.1.1）。

### DS-499

表面1で、`@vtest.` で始まるがtest-keyを持たない行はエラーE-SCAN-006とする（打鍵ミスの検出を優先し、警告ではなくエラーとする）。

### DS-500

表面1のE-SCAN-006は、未知キーに加え、source-target-key（`src-id`）の誤配置も含む。`src-id` は対象実装側の関数に付与すべきキーであり、Test metadataへの取り込み先を持たない。

### DS-501

表面2で、`@vtest.` で始まるがsource-target-keyを持たない行（test-keyを含む）は警告W-SCAN-105とする（§5.4）。

### DS-502

`src-id` は表面2でも反復不可であり、同一関数itemでの重複は採用すべきIDを決定できないためエラーE-SCAN-005とする。

### DS-503

`src-id` 重複時はいずれの宣言値も採用せず、当該Source TargetのSRC IDは無しとして扱う（どちらかを推測で選ばない）。

### DS-504

doc comment 内の `@vtest.` を含まない行は自由記述として無視する。

### DS-505

`@vtest.src-id` はテストではなく対象実装側の関数に付与し、任意の恒久SRC IDを宣言する。

### DS-506

scannerは `@vtest.src-id` の指定値を認識するが、付与を必須としない。

*導出元: SPEC-085, SPEC-086*

*引用: 基本仕様 §9.2*

### DS-507

表面2での打鍵ミス（`src_id` 等の未知キー）はW-SCAN-105、`src-id` の重複はE-SCAN-005で検出し、無音で無視しない（§4.2・§5.4）。

### DS-S081 4.3 `rust-cargo` locator構文

*導出元: SPEC-S028, SPEC-S066*

### DS-508

locator文法は `locator = path "::" item-path` である。

> 例：src/parser.rs::Parser::parse
>     src/lib.rs::validate_input

### DS-509

pathはプロジェクトルートからの相対パス（"/" 区切り、".rs" で終わる）である。

### DS-510

item-pathはRustアイテムパス（"::" 区切り）であり、implブロック内の関数は"型名::関数名"とする。

### DS-511

`@vtest.target` の値が `SRC-` で始まる場合はSRC ID参照として返す。

### DS-S082 4.4 宣言エラーの扱い

*導出元: SPEC-S018, SPEC-S028, SPEC-S035*

### DS-512

coreは当該Testを管理宣言欠落として `chain_integrity` の `MISMATCH`（診断 `MISSING`）とし、対応VOを推測で寄与関係へ関連付けない。

### DS-513

`rust-cargo` は検証対象をSource Targetとして実現する形態であり、追加必須metadataとして `targets ≥ 1` を要求する（§4.1・§4.2・§5.5）。

*導出元: SPEC-085, SPEC-086*

*引用: 基本仕様 §9.2*

### DS-514

coreの `id` / `covers ≥ 1` / `intent`、および `rust-cargo` の `targets ≥ 1` という必須metadataを欠く場合はE-SCAN-007とし、`ManagedTestLink::Missing`（`chain_integrity` の `MISMATCH`、診断 `MISSING`）とする。

### DS-515

`covers` 件数の可変制約（旧role/anchor由来）は設けず、すべての管理対象Testに `covers ≥ 1` を一律要求する。

*導出元: SPEC-123, SPEC-124, SPEC-369, SPEC-370, SPEC-371, SPEC-372, SPEC-373, SPEC-374, SPEC-375, SPEC-376, SPEC-377, SPEC-378, SPEC-379, SPEC-380, SPEC-381, SPEC-382, SPEC-383, SPEC-384, SPEC-385, SPEC-386, SPEC-387, SPEC-388, SPEC-389, SPEC-390, SPEC-391, SPEC-392, SPEC-393, SPEC-394, SPEC-395, SPEC-396, SPEC-397, SPEC-398, SPEC-399, SPEC-400, SPEC-401, SPEC-402, SPEC-403, SPEC-404, SPEC-405, SPEC-406, SPEC-407*

*引用: 基本仕様 §12*

## DS-S083 5. Discovery orchestration設計

### DS-S084 5.1 処理フロー

*導出元: SPEC-S004*

### DS-516

adapterが解析不能または不完全なbatchを返した場合、coreは対応する検証を `UNKNOWN` とし、Test 0件の完全なdiscoveryとして扱わない。

### DS-S085 5.2 エンティティモデル（vtest-model）

*導出元: SPEC-S007, SPEC-S012, SPEC-S017*

### DS-517

診断ラベルは検証状態と別軸である。

*導出元: REQ-092, REQ-093, SPEC-047, SPEC-048*

*引用: 基本仕様 §4.2, 要件定義 §5.2*

### DS-518

集約の代表値選択に診断ラベルを用いない（§11.3）。

*導出元: SPEC-190, SPEC-191*

*引用: 基本仕様 §22.2*

### DS-519

`TargetRef::SrcId` はadapter IDを含まないため、`SrcId` は全adapterを統合したrepositoryでglobal uniqueでなければならない。

### DS-520

collision時はE-SCAN-011とし、TargetRefを解決しない。

### DS-521

Source Target hashは常にcanonical Locatorとconstruct bytesから計算し、恒久SRC IDを独立したhash fieldとして含めない。

### DS-522

canonical Locatorは恒久SRC IDの増減で変化しないため、参照方法の違いによってSource Targetの件数、content / subject hash、Evidence上のtarget identityが分裂しない。

### DS-523

恒久SRC IDの宣言をconstruct bytesの内側へ置くadapterでは、その宣言の追加・変更・削除がconstruct bytesを変え、Source Target hashを変化させうる（§1.3）。

### DS-524

恒久SRC IDの宣言をconstruct bytesの内側へ置くadapterでSource Target hashが変化することは、sourceが実際に変化したことの帰結であり、参照方法による分裂ではない。

### DS-525

恒久SRC IDを持つSource Targetも引き続きcanonical locatorでaddressableでなければならない。

### DS-526

manifest等にある非隣接metadataも `metadata_sources` へ列挙するが、hash inputはadapter構文のraw表現ではなく `ManagedTestDraft` のcanonical logical metadataである。

### DS-527

provenance欠落はmalformed adapter outputとしてE-ADAPTER-002で拒否する。

### DS-528

明示操作に必須のcapabilityがない場合はE-ADAPTER-004で操作を中止する。

### DS-529

検証集約では、static解析 / coverage欠落は `NO_EVIDENCE`（診断 `NOT_CHECKED`）とする。

### DS-530

検証集約では、runner欠落は `NO_EVIDENCE`（診断 `NOT_EXECUTED`）とする。

### DS-531

検証集約では、解析限界は `UNKNOWN` とする。

### DS-532

変化した場合はE-EXEC-004としてEvidenceを生成しない。

### DS-533

Form `kind` はbuilt-inと `.verify/forms/` を統合したrepository全体で一意である。

### DS-534

重複kindまたは対応の不一致はE-ADAPTER-001、未知kindはE-OP-001とし、coreが名前からRust adapterを推測しない。

### DS-S086 5.4 整合性診断

*導出元: SPEC-S017, SPEC-S028*

### DS-535

E-SCAN-001はerrorであり、adapterのsource構文解析失敗（DiscoveryBatchは `Incomplete`）を意味する。

### DS-536

E-SCAN-002はerrorであり、Test ID重複（identity collision）を意味する。

### DS-537

E-SCAN-003はerrorであり、`covers` の参照先VOが存在しない（dangling reference）ことを意味する。

### DS-538

E-SCAN-004はerrorであり、`target` のロケータ／SRC IDを解決できないことを意味する。

### DS-539

E-SCAN-005はerrorであり、adapter所有の宣言で重複不可fieldが重複、または綴りの異なる複数の `target` 宣言が同一canonical Source Targetへ解決することを意味する。

### DS-540

E-SCAN-006はerrorであり、Test constructのadapter所有の宣言に未知fieldが存在することを意味する（非Test construct表面はW-SCAN-105）。

### DS-541

E-SCAN-007はerrorであり、必須metadata（core中立: id / covers ≥ 1 / intent、および当該adapterが必須とする追加metadata。`rust-cargo` では targets ≥ 1）の欠落を意味する。

### DS-542

E-SCAN-008はerrorであり、VOのparent不在または循環を意味する。

### DS-543

E-SCAN-009はerrorであり、Relationのfrom / toが不在であることを意味する。

### DS-544

E-SCAN-010はerrorであり、レコードのid / ファイル名 / schema不一致、または互換正規化後のlogical record ID重複を意味する。

### DS-545

E-SCAN-011はerrorであり、恒久SRC IDが複数adapterまたは複数Source Targetで衝突することを意味する。

### DS-546

E-SCAN-012はerrorであり、VOの `derives_from` が存在しないdocumentを参照、またはdocumentの `derives_from` が存在しないdocumentを参照する（文書鎖のリンク切れ）ことを意味する。

### DS-547

E-SCAN-016はerrorであり、根に指定されない孤児document（親documentを持たず `doc.roots` にも列挙されない）を意味する（§5.6）。

### DS-548

W-SCAN-101はwarningであり、adapterが発見したが管理宣言に対応しないTest construct（unregistered test）を意味する。

### DS-549

W-SCAN-102はwarningであり、どのVOからも参照されず、Testも参照しない孤立VOを意味する。

### DS-550

W-SCAN-103はwarningであり、`covers` を持つが対応VOがleafでない（中間VO直接参照。許容するが警告）ことを意味する。

### DS-551

W-SCAN-105はwarningであり、Test constructとして解析されない関数itemのdoc comment内の `@vtest.` 行に認識されないキーが存在することを意味する（打鍵ミス検出。`src-id` の重複はE-SCAN-005）（§4.2）。

### DS-552

W-STORE-001はwarningであり、VO recordに非正典の読取り互換field `status` が存在することを意味する（値は無視し承認から導出）。

### DS-553

W-STORE-002はwarningであり、Approvalが現在の上流依存closureを欠くか一致せず、承認として無効であることを意味する。

### DS-554

W-STORE-003はwarningであり、判断記録が `judgment_kind` を欠くか値域外で、いずれの実効判断へも寄与しないことを意味する（§8.5）。

### DS-555

W-STORE-004はwarningであり、同一 `(subject, judgment_kind)` に判断値の食い違う有効判断記録が併存し、実効判断が未確定であることを意味する（§8.5）。

### DS-556

W-STORE-005はwarningであり、判断記録または承認レコードの `supersedes` の参照先を解決できない、対象が一致しない、またはsupersede関係が循環し、当該recordが実効集合へ寄与しないことを意味する（§8.5・§3.5）。

### DS-557

W-STORE-006はwarningであり、承認レコードの `approved_state` または `subject` の種別が値域外、あるいは `judgment_ref` の参照先が存在せず、実効承認を導出しないことを意味する（§3.5）。

### DS-558

errorは該当エンティティに関わる検査を非 `PASS` にする。

### DS-559

warningは診断severityだけでは検証値を変更しない。

### DS-560

管理宣言の欠落・E-SCAN-007（必須metadata欠落）が示す `ManagedTestLink::Missing` は `chain_integrity = MISMATCH`（診断 `MISSING`）に写像する。

### DS-561

`ManagedTestLink::Multiple`、E-SCAN-002（Test ID衝突）、E-SCAN-003（解決不能なVO参照）は `chain_integrity = MISMATCH` に写像する。

### DS-562

E-SCAN-008（VO parent不在・循環）、E-SCAN-009（Relation dangling）、E-SCAN-012（文書鎖・VO derives_fromのリンク切れ）は `chain_integrity = MISMATCH` に写像する。

### DS-563

E-SCAN-016（孤児document）は `orphan_detection = MISMATCH` に写像する（§5.6）。

### DS-564

E-SCAN-011があるSRC ID参照は曖昧なため、関係するtarget解決を `MISMATCH` とし、いずれのSource Targetも選択しない。

### DS-565

候補の1件を解決結果としてEvidence・検証へ永続化しない（§6.1）。

### DS-566

衝突する恒久SRC IDを宣言した各Source Target自体は、canonical locatorで独立に具体化されたまま保持する。

### DS-S087 5.5 `rust-cargo` SourceDiscoveryAdapter

*導出元: SPEC-S063, SPEC-S066*

### DS-567

欠落はE-SCAN-007として報告する（§4.4・§5.4）。

### DS-568

したがって `rust-cargo` のTestは従来どおりSource Target宣言を要し、挙動・Eコード・fixtureは本改訂で実効的に変わらない。

### DS-569

`rust-cargo` discoveryの第4段はTest construct抽出であり、属性pathの末尾segmentが"test"である関数（`#[test]`、`#[tokio::test]` 等）を抽出する。

### DS-570

`rust-cargo` discoveryの第6段（Source Target抽出）で非Test constructのdoc comment内の `@vtest.` 行を検査し、認識されないキーからW-SCAN-105を、`src-id` の重複からE-SCAN-005を生成する（§4.2）。

### DS-S088 5.6 文書層 orphan_detection

*導出元: REQ-S010, SPEC-S019*

### DS-571

`config.yaml` の `doc.roots` に列挙されたDOC IDを根として扱い、`orphan_detection` の対象外とする（§2.2）。

### DS-572

`derives_from` が空、かつ他のどのdocumentからも `derives_from` で参照されないdocumentのうち、`doc.roots` に列挙されないものを孤児とし、E-SCAN-016（`orphan_detection = MISMATCH`）とする。

### DS-573

根に指定されたdocumentが存在しないDOC IDを参照する場合は、config invariant違反としてE-CONFIG-001とする。

## DS-S089 6. Target Reference解決

### DS-S090 6.1 adapter-neutral解決contract

*導出元: SPEC-S010, SPEC-S028*

### DS-574

解決が0件または複数候補で一意に定まらない場合はE-SCAN-004とする。

### DS-575

解決が0件または複数候補で一意に定まらない場合、推測で候補を選択しない。

### DS-576

SRC ID参照は当該恒久SRC IDを宣言したSource Targetのcanonical locatorへ解決する。

### DS-577

SRC ID参照は、同じSource Targetへのlocator参照と同一のcanonical Source Target・同一のSource Target hashへ到達する。

### DS-578

解決結果をlocator版とSrcId版の別entityへ分岐させない。

### DS-579

恒久SRC IDが複数adapterまたは複数Source Targetで衝突する場合はE-SCAN-011とする。

### DS-580

恒久SRC IDが複数adapterまたは複数Source Targetで衝突する場合、いずれのSource Targetも選択しない。

### DS-581

曖昧はfail-closedな終端状態とする。

### DS-582

曖昧な解決から代表候補を選ばない。

### DS-583

曖昧な解決について、解決済みのcanonical Source Targetを要求する後段（静的解析、Evidence、`target_coverage`、鮮度判定）へ候補を1件も引き渡さない。

### DS-584

候補は§6.3の診断表示にだけ用い、表示できることを選択の根拠にしない。

### DS-585

曖昧な解決について候補を後段へ1件も引き渡さないという禁止はTarget Referenceの解決に関するものであり、Source Targetの具体化を止めるものではない。

### DS-586

各Source Targetは自身のcanonical locatorで独立に具体化され、恒久SRC IDが衝突していても`SourceTargetDraft`ごとに1件のSource Targetとして成立する。

### DS-587

衝突が壊すのは当該恒久SRC IDによる参照の一意性だけである。

### DS-588

E-SCAN-004またはE-SCAN-011で解決できなかったtargetを、後段が任意の候補で埋めて記録・永続化することを禁ずる。

#### DS-S091 6.1.1 target identityの一方向確定

*導出元: SPEC-S028*

### DS-589

Source Target identityは「宣言されたTargetRef（Locator / SrcId）→ resolve（§6.1）→ canonical Locator」の一方向でだけ確定する。

> TestEntity.targets = 宣言されたTargetRef（Locator / SrcId） / ↓ resolve（§6.1） / Canonical Source Target = canonical Locator / ↓ / Evidence / target_coverage / 検証 = canonical Locatorをidentityとして使用

### DS-590

Evidence（§3.6、§9.4）、`target_coverage`（§10.2）、および鮮度判定（§11.2）は、解決後のcanonical Locatorをtarget identityとして記録・比較する。

### DS-591

参照側Testが宣言した`TargetRef`の綴り（SRC ID参照を含む）をこれらのidentityとして保存してはならない。

### DS-592

Testがどう宣言したか（同じSource Targetに対するLocator参照からSRC ID参照への書き換え等）の変更は、`targets`をcanonical metadataとして束縛する§1.3のTest subject hashが捕捉する。

### DS-593

Evidence側で宣言表現を保持する必要はなく、保持すれば同一Source Targetが参照方法ごとに別identityへ分裂する。

### DS-594

Testの宣言target集合は解決後のcanonical Source Target単位で一意でなければならない。

### DS-595

綴りの異なる複数の宣言が同一のcanonical Source Targetへ解決する場合は重複targetとしてE-SCAN-005とする。

### DS-S092 6.2 `rust-cargo` locator解決

*導出元: SPEC-S028*

### DS-596

`rust-cargo`のlocator解決で一意に決まらない場合（同名fnがcfg分岐で複数等）は、すべて候補として返し、解決失敗（E-SCAN-004）とする。

### DS-S093 6.3 候補提示

*導出元: SPEC-S038*

### DS-597

Structured Operationの入力検証（別紙A §14、§15）で解決に失敗した場合、coreはadapterが返した候補を共通envelopeで表示する。

*引用: 別紙A §14, 別紙A §15*

### DS-598

`rust-cargo` adapterのenum variant検証は、解決できる場合のみ検証する。

### DS-599

`rust-cargo` adapterのenum variant検証は、解決できない自由記述はそのまま受理する（best effort。拒否はしない）。

## DS-S094 7. Static Analysis orchestrationと`rust-cargo`ルール

### DS-S095 7.1 判定の原則

*導出元: P-003, SPEC-S021, SPEC-S023*

### DS-600

各ルールは`FAIL` / `UNKNOWN` / `PASS(違反なし)`のいずれかを返す。

### DS-601

決定論的に確定できる違反のみFAILとする。

### DS-602

解析の限界で確定できない場合はFAILではなくUNKNOWNとする。

### DS-603

`UNKNOWN`は意味判定できる者への判断記録エスカレーション（§8）の領分である。

*導出元: SPEC-106, SPEC-107, SPEC-108, SPEC-109, SPEC-110, SPEC-111, SPEC-112, SPEC-113, SPEC-114, SPEC-115, SPEC-116, SPEC-117, SPEC-118, SPEC-119, SPEC-120, SPEC-121, SPEC-122, SPEC-319, SPEC-320, SPEC-321, SPEC-322, SPEC-323, SPEC-324, SPEC-325, SPEC-326, SPEC-327, SPEC-328, SPEC-329, SPEC-330, SPEC-331, SPEC-332, SPEC-333, SPEC-334, SPEC-335, SPEC-336, SPEC-337, SPEC-338, SPEC-339, SPEC-340, SPEC-341, SPEC-342, SPEC-343, SPEC-344, SPEC-345, SPEC-346, SPEC-347, SPEC-348, SPEC-349, SPEC-350*

*引用: 基本仕様 §11*

### DS-604

ただしDA-002のtarget到達UNKNOWNは§7.3のruntime到達証明で解決し、判断記録へは委ねない。

### DS-605

`oracle_presence`はDA-001 / DA-003 / DA-004 / DA-005 / DA-006の合成とする。

### DS-606

`oracle_presence`は、全ルールが違反なしなら`PASS`とする。

*導出元: REQ-073, REQ-074, REQ-075, REQ-076, REQ-077, REQ-078, REQ-079, REQ-080, REQ-081, REQ-082, REQ-083, REQ-084, SPEC-066, SPEC-067, SPEC-068, SPEC-069, SPEC-290*

*引用: 基本仕様 §5.4, 要件定義 §4.4*

### DS-607

`oracle_presence`は、1つでも`FAIL`があれば`FAIL`とする。

*導出元: REQ-073, REQ-074, REQ-075, REQ-076, REQ-077, REQ-078, REQ-079, REQ-080, REQ-081, REQ-082, REQ-083, REQ-084, SPEC-066, SPEC-067, SPEC-068, SPEC-069, SPEC-290*

*引用: 基本仕様 §5.4, 要件定義 §4.4*

### DS-608

`oracle_presence`は、`FAIL`がなく`UNKNOWN`があれば`UNKNOWN`とする。

*導出元: REQ-073, REQ-074, REQ-075, REQ-076, REQ-077, REQ-078, REQ-079, REQ-080, REQ-081, REQ-082, REQ-083, REQ-084, SPEC-066, SPEC-067, SPEC-068, SPEC-069, SPEC-290*

*引用: 基本仕様 §5.4, 要件定義 §4.4*

### DS-609

`oracle_presence`に動的な昇格経路は無い。

### DS-610

静的解析は不成立の証明であり、証明の失敗は`UNKNOWN`であって、runtime証拠で`PASS`へ昇格しない。

### DS-611

`target_binding`の静的到達証明はDA-002が担う（§7.3）。

### DS-612

DA-002のtarget別verdictの`UNKNOWN`は「静的解析の到達判定境界の外にあり、静的には到達を証明できない」ことだけを表し、到達しないことを意味しない。

### DS-613

DA-002のtarget別verdictのUNKNOWNは§7.3に従い当該targetのruntime計測（§10）が実行を証明した場合に限り充足される。

> 要件定義 §4.3の2証拠源モデルは`target_binding`に固有。

*導出元: REQ-064, REQ-065, REQ-066, REQ-067, REQ-068, REQ-069, REQ-070, REQ-071, REQ-072*

*引用: 要件定義 §4.3*

### DS-614

Static Analysis capabilityがない場合は`NO_EVIDENCE`（診断`NOT_CHECKED`）とする。

### DS-615

adapterが不完全、解析限界、または解析入力集合の不完全性を報告した場合は`UNKNOWN`とし、違反なしと推測しない。

### DS-S096 7.2 `rust-cargo` ルール一覧

*導出元: SPEC-S021, SPEC-S025*

### DS-616

`rust-cargo`のassert相当の構文はDA-001〜DA-006で共通に用いる。

> assert!/assert_eq!/assert_ne!/panic! を含む標準マクロおよびconfigのassertion_macros列挙マクロ、#[should_panic]属性、.unwrap()/.expect(..)/?演算子（Result/Optionの成立検証として扱う）、Test関数がResultを返しErrを返しうる構造、の4分類の総称。

### DS-617

`rust-cargo`のassert相当の構文は`assert!` / `assert_eq!` / `assert_ne!` / `panic!`を含む標準マクロ、および`rust-cargo` configの`assertion_macros`に列挙されたマクロを含む。

### DS-618

`rust-cargo`のassert相当の構文は`#[should_panic]`属性を含む。

### DS-619

`rust-cargo`のassert相当の構文は`.unwrap()` / `.expect(..)` / `?`演算子（Result / Optionの成立検証として扱う）を含む。

### DS-620

`rust-cargo`のassert相当の構文はTest関数が`Result`を返し`Err`を返しうる構造を含む。

### DS-621

DA-001（定数アサーション）はoracle_presenceへ供給する検査であり、引数がすべてリテラル・定数式のassertを内容とし、関数内のassert相当がすべて定数アサーションであることをFAIL条件とし、定数性を確定できない式をUNKNOWNへ退避する例とする。

### DS-622

DA-002（対象未呼出）はtarget_bindingへ供給する検査であり、宣言されたtargetシンボルを呼んでいないことを内容とし、関数本体および同一ファイル内の呼出先helper（1段）を探索して呼出が存在しない、かつ他ファイルへの呼出も存在しないことをFAIL条件とし、他ファイル・他クレートの関数呼出があり間接呼出の可能性を排除できない場合をUNKNOWNへ退避する例とする。

### DS-623

DA-003（結果未検証）はoracle_presenceへ供給する検査であり、targetを呼ぶがその結果をassert相当で一切検証しないこと（照合の委譲先がある場合は§7.2.1で終端を確認する）を内容とし、target呼出結果（戻り値、および結果から派生した束縛）がassert相当に到達しない、かつ`#[should_panic]`がないことをFAIL条件とし、結果が可変参照・グローバル状態経由で検証される可能性がある場合をUNKNOWNへ退避する例とする。

### DS-624

DA-004（自己比較）はoracle_presenceへ供給する検査であり、`assert_eq!(a, b)`でaとbがトークン列として同一であることを内容とし、該当assertが存在することをFAIL条件とし、UNKNOWNへ退避する例は無い（構文的に確定）。

### DS-625

DA-005（空テスト）はoracle_presenceへ供給する検査であり、関数本体に文が存在しないことを内容とし、該当することをFAIL条件とし、UNKNOWNへ退避する例は無い。

### DS-626

DA-006（検証構文なし）はoracle_presenceへ供給する検査であり、関数内にassert相当が1つも存在しないことを内容とし、関数内にassert相当が1つも存在せず、かつ§7.2.1の照合の委譲先も同定できないことをFAIL条件とし、委譲先を同定できるが終端を確認できない場合（§7.2.1）をUNKNOWNへ退避する例とする。

### DS-627

W-DA-101（ignored）は`#[ignore]`属性を内容とし、FAILにしない警告のみであり、実行されなければ`target_binding`が診断NOT_EXECUTEDになる。

### DS-628

DA-002 / DA-003のデータフロー解析は関数内のローカル束縛の追跡（let束縛、メソッドチェーン、フィールドアクセス）までとする。

### DS-629

DA-002 / DA-003のデータフロー解析はクロージャ内・マクロ展開内はUNKNOWNとする。

### DS-630

複数target TestではDA-002 / DA-003を各targetへ個別適用する。

### DS-631

target別結果に1件でもFAILがあればrule結果をFAILとする。

### DS-632

FAILがなく1件でもUNKNOWNがあればrule結果をUNKNOWNとする。

### DS-633

全targetが違反なしの場合だけrule結果をPASSとする。

### DS-634

宣言targetへの呼出がTest本体に静的に現れない場合（subprocessを起動して別プロセスでtargetを実行する等、target呼出がsource内に存在しない）、DA-003の当該target別verdictをUNKNOWNとする。

### DS-635

呼出結果を観測できないことを「違反なし（空虚PASS）」とも「結果未到達（空虚FAIL）」とも判定しない。

### DS-636

宣言targetへの呼出がTest本体に静的に現れない場合、DA-002も同targetでUNKNOWNであり、DA-002が§7.3のruntime証明で救済されてもDA-003はUNKNOWNのまま`oracle_presence`へ寄与するため、呼出が本体に現れないTest（典型的なsubprocess E2E）はoracle_presence = PASSに到達しない。

### DS-637

target呼出はTest本体に現れるがDA-002がUNKNOWNになる場合（他ファイル・他クレートへの直接呼出で間接呼出の可能性を排除できない等）、その呼出結果がTest本体内でassert相当へ到達すればDA-003 = PASSになりうる。

### DS-638

target呼出はTest本体に現れるがDA-002がUNKNOWNになる場合のtargetは、DA-002をruntimeで救済すれば`target_binding` = PASSに到達しうる（runtime救済で実益が出る型）。

### DS-639

クロージャ・マクロ展開の内側での到達は§7.2の一般則どおりDA-002 / DA-003ともUNKNOWNとする。

#### DS-S097 7.2.1 照合の委譲先の終端（DA-003・DA-006）

### DS-640

Testの成否判定が、assert相当の構文でなく通常の関数へ委譲されている場合、その委譲先が検証閉包の中で終端しない限り、照合装置検査の成立側を確定しない。

### DS-641

assert相当の引数、または引数へ到達する束縛（§7.2のデータフロー解析の範囲内）に現れる関数呼出は当該Testの照合の委譲先とする。

> 呼出先はadapterが列挙するassert相当の構文に含まれないものに限る。

### DS-642

関数内にassert相当が1つも存在しないTest本体に現れる関数呼出は当該Testの照合の委譲先とする。

> 呼出先はadapterが列挙するassert相当の構文に含まれないものに限る。

### DS-643

委譲先`H`は、`H`を宣言targetとするTestが1件以上存在し、それらすべての`oracle_presence`が`PASS`であるとき、かつそのときに限り終端する。

### DS-644

終端の判定は covers / 宣言target のグラフの参照だけで行う。

### DS-645

終端の判定について、信頼を宣言する専用の記録・注釈・設定項目は設けない。

### DS-646

委譲先が無い（assert相当の構文だけで照合が完結する）場合、DA-003 / DA-006は従来どおり各ルールのFAIL条件で評価する。

> 判定。

### DS-647

委譲先がすべて終端する場合、DA-003 / DA-006は違反なしとする。

### DS-648

委譲先を同定できるが、それを宣言targetとするTestが0件の場合、DA-003 / DA-006は`UNKNOWN`とする。

### DS-649

委譲先を宣言targetとするTestが存在するが、その`oracle_presence`が`PASS`でない（`FAIL` / `UNKNOWN` / `NO_EVIDENCE`のいずれか）場合、DA-003 / DA-006は`UNKNOWN`とする。

### DS-650

終端の探索が循環する（`H`を宣言targetとするTestの照合が`H`自身へ、または相互に委譲される）場合、DA-003 / DA-006は`UNKNOWN`とする。

### DS-651

委譲先が他ファイル・他クレート・マクロ展開内にあり呼出先を同定できない場合、DA-003 / DA-006は`UNKNOWN`とする。

### DS-652

終端を確認できない委譲先を、違反なし（成立側）としても`FAIL`としても扱わない。

> 前者は未検証の照合装置を成立と読み替えることになり、後者は解析の限界を確定した違反と読み替えることになる（§7.1）。

### DS-653

終端の判定は同一のscan結果から算出する（§11.1の決定性）。

### DS-654

他Testの`oracle_presence`を参照するため、循環は上表のとおり`UNKNOWN`で閉じ、評価順序によって結果が変わる経路を作らない。

### DS-655

委譲先が終端したことは当該Testの`oracle_presence`を昇格させる根拠にはならず、上表の枝が示すのはDA-003 / DA-006の値だけである。

### DS-656

`oracle_presence`全体は§7.1の合成規則で決まる。

### DS-S098 7.3 target 到達の静的証明と runtime 証明の関係（target_binding）

*導出元: SPEC-S020, SPEC-S022, SPEC-S027*

### DS-657

`target_binding`は「そのTestが検証対象とする振る舞いが実際に生じ、その振る舞いを反映した観測が得られたか」を問う。

*導出元: REQ-064, REQ-065, REQ-066, REQ-067, REQ-068, REQ-069, REQ-070, REQ-071, REQ-072, SPEC-061, SPEC-062, SPEC-063, SPEC-064, SPEC-065, SPEC-289*

*引用: 基本仕様 §5.3, 要件定義 §4.3*

### DS-658

`target_binding`は静的解析（DA-002）と動的計測（§10 coverage）の2証拠源を持ち、静的に確定できなければ`UNKNOWN`とし動的証拠で昇格できる。

### DS-659

DA-002は§7.2の解析境界（関数本体および同一ファイル内helper1段。クロージャ内・マクロ展開内・他ファイル・他クレートへの呼出は§7.1 / §7.2に従いUNKNOWN）で行う静的なtarget到達証明である。

### DS-660

Testがtargetを静的解析の追えない実行境界を越えて到達させる形態はいずれもDA-002のUNKNOWNとして現れる。

### DS-661

Testがtargetを静的解析の追えない実行境界を越えて到達させる形態は、Testのkind（unit / integration）とは独立に、execution topologyによって決まる。

### DS-662

静的解析の追えない実行境界は、他ファイル・他クレートへの呼出を介した間接到達を含む。

### DS-663

静的解析の追えない実行境界は、クロージャ・マクロ展開内での到達を含む。

### DS-664

静的解析の追えない実行境界は、生成した別スレッド（in-process, thread boundary）での到達を含む。

### DS-665

静的解析の追えない実行境界は、別プロセス（subprocessを起動し、そのプロセス内でtargetを実行するprocess boundary）での到達を含む。

### DS-666

到達要件は、targetごとに、静的証明：当該targetのDA-002 verdict = PASS（§7.2の解析境界内で呼出を確認）のいずれかで充足される。

### DS-667

到達要件は、targetごとに、runtime証明：§11.2が選択した最新Evidenceが§6のハッシュ束縛（鮮度）を満たすとき、そのEvidenceの§10.2target別`target_coverage` result = PASS（`checked: true`かつ実行count > 0）のいずれかで充足される。

### DS-668

DA-002 verdictがUNKNOWN（静的に証明できない）であるtargetは、runtime証明が成立するときに限り到達要件を満たす。

### DS-669

複数target Testではtargetごとに到達要件を適用する。

### DS-670

Testの`target_binding`到達は全宣言targetの到達要件が充足された場合にのみ成立する。

### DS-671

static側は§7.2のDA-002 verdictをtargetごとに用いる。

### DS-672

DA-002 verdict = FAIL（解析境界内で到達を静的に否定）はruntime証明で覆さない。

### DS-673

runtime側は§11.2が選択した最新Evidenceだけを用いる。

### DS-674

最新Evidenceが鮮度を満たさなければruntime証明は成立せず、古いEvidenceへフォールバックしない。

> これにより同一検証内で計測が§11.2でSTALEの一方target_bindingが別EvidenceでPASSになる履歴不一致を防ぐ。

### DS-675

static側は§7.2のDA-002を再計算し、runtime側は§11.2選択Evidenceを用いて、targetごとに実効到達状態を定める。

### DS-676

静的到達とは、DA-002 verdict = PASSである状態をいう。

### DS-677

runtime到達とは、DA-002 verdict = UNKNOWNかつruntime証明成立である状態をいう。

### DS-678

未充足とは、DA-002 verdict = FAIL、またはUNKNOWNでruntime証明が成立しない状態をいう。

### DS-679

`target_binding`は、Evidenceの`result: FAIL`（テストランナーが失敗を報告）なら`FAIL`とする。

### DS-680

`target_binding`は、そうでなく全宣言targetの到達が静的到達またはruntime到達で充足されれば`PASS`とする。

### DS-681

到達未充足のtargetがあれば§11.2の写像に従い非`PASS`（動的計測count 0は`FAIL`/診断NOT_EXECUTED、計測不能・未計測は`NO_EVIDENCE`、解析限界は`UNKNOWN`）とする。

### DS-682

`target_binding`の到達要件が静的到達またはruntime到達の充足によってのみ`PASS`となる関係はfail-closedを保つ。

### DS-683

runtime証明は当該targetの`target_coverage` = PASSのときだけ成立する。

### DS-684

`target_coverage`がFAIL（count 0）・UNKNOWN（関数不見当）・NOT_CHECKED（coverage利用不能、未計測、`--fast`）のときは到達要件を満たさず、当該targetは未充足となり、`target_binding`を非`PASS`にする。

### DS-685

検証対象をSource Targetとして実現する形態（`rust-cargo`）で、宣言targetをどのtopologyでも実行しないTest（構造・契約のみをassertするTest）は静的にもruntimeにも到達を確立できず、到達要件は未充足のままとなる。

### DS-686

v0.1の唯一のadapter`rust-cargo`では検証対象をSource Targetとして宣言しないTestはE-SCAN-007（`targets ≥ 1`欠落）として`target_binding`評価の手前で`chain_integrity`の`MISMATCH`になる。

### DS-687

targetを持たないTestは本節の合成へ到達しない。

### DS-688

DA-003はこのto-runtime joinに含めない。

### DS-689

DA-003は`oracle_presence`（照合装置の存在）へ寄与するstatic data-flow判定であり（§7.2）、targetの「結果検証」を問う。

### DS-690

runtime coverageはtargetの「実行」を証明するが「結果検証」を証明しないため、coverageはDA-003を代替せず、DA-003は§7.2の意味論のまま維持する。

### DS-691

典型的なsubprocess E2E（targetの戻り値 → 子プロセスのstdout / exit code → 親プロセスのassert）では、このdata-flowはstatic analyzerから追えないためDA-003はUNKNOWNのまま残りやすい。

## DS-S099 8. 判断記録プロトコル

*導出元: REQ-S035, SPEC-S031*

### DS-692

機械が決定論で確定できない疑義は`UNKNOWN`として外部（人間または判断可能Agent）へ引き渡し、その判断を判断記録（§3.4）として追跡する。

### DS-693

判断記録の受理は当該対象の検証状態を昇格させない。

*導出元: REQ-180, REQ-181, REQ-182, REQ-183, REQ-184, REQ-185, REQ-186, REQ-187, REQ-188, REQ-189, REQ-190, REQ-191, REQ-192, SPEC-116, SPEC-117, SPEC-118, SPEC-119, SPEC-120, SPEC-121, SPEC-122, SPEC-333, SPEC-334, SPEC-335*

*引用: 基本仕様 §11.3, 要件定義 §12*

### DS-S100 8.1 バンドル生成

### DS-694

本書が定義する判断型の値は`test-semantic` / `impl-consistency` / `case-coverage`の3種であり、これ以外の値でバンドルを生成しない。

### DS-695

`test-semantic`は、subjectの値域がTest IDであり、外部へ引き渡す問いは「テストコードは、covers先VOのclaimとTest Intentが宣言する振る舞いを実際に検証しているか」である。

### DS-696

`impl-consistency`は、subjectの値域がTest IDであり、外部へ引き渡す問いは「対象実装は宣言と一致しているか」である。

### DS-697

`case-coverage`は、subjectの値域をTest IDまたはVO IDとする。

### DS-698

`case-coverage`は、subjectがTest IDのとき、外部へ引き渡す問いは「当該Testが宣言したcases集合は、covers先VOの要求入力空間を十分に代表・網羅しているか」である。

*導出元: SPEC-106, SPEC-107, SPEC-108, SPEC-109, SPEC-110, SPEC-111, SPEC-112, SPEC-113, SPEC-114, SPEC-115, SPEC-116, SPEC-117, SPEC-118, SPEC-119, SPEC-120, SPEC-121, SPEC-122, SPEC-129, SPEC-130, SPEC-131, SPEC-132, SPEC-319, SPEC-320, SPEC-321, SPEC-322, SPEC-323, SPEC-324, SPEC-325, SPEC-326, SPEC-327, SPEC-328, SPEC-329, SPEC-330, SPEC-331, SPEC-332, SPEC-333, SPEC-334, SPEC-335, SPEC-336, SPEC-337, SPEC-338, SPEC-339, SPEC-340, SPEC-341, SPEC-342, SPEC-343, SPEC-344, SPEC-345, SPEC-346, SPEC-347, SPEC-348, SPEC-349, SPEC-350*

*引用: 基本仕様 §14、§11*

### DS-699

`case-coverage`は、subjectがVO IDのとき、外部へ引き渡す問いは「当該VOをcoversするTest群のcases集合は、当該VOの要求入力空間を十分に代表・網羅しているか」である。

*導出元: SPEC-106, SPEC-107, SPEC-108, SPEC-109, SPEC-110, SPEC-111, SPEC-112, SPEC-113, SPEC-114, SPEC-115, SPEC-116, SPEC-117, SPEC-118, SPEC-119, SPEC-120, SPEC-121, SPEC-122, SPEC-129, SPEC-130, SPEC-131, SPEC-132, SPEC-319, SPEC-320, SPEC-321, SPEC-322, SPEC-323, SPEC-324, SPEC-325, SPEC-326, SPEC-327, SPEC-328, SPEC-329, SPEC-330, SPEC-331, SPEC-332, SPEC-333, SPEC-334, SPEC-335, SPEC-336, SPEC-337, SPEC-338, SPEC-339, SPEC-340, SPEC-341, SPEC-342, SPEC-343, SPEC-344, SPEC-345, SPEC-346, SPEC-347, SPEC-348, SPEC-349, SPEC-350*

*引用: 基本仕様 §14、§11*

### DS-700

`judgment_kind`と`subject`の種別の組合せがこの表にない要求ではバンドルを生成しない（別紙A §12.2のusage error、終了コード2）。

*引用: 別紙A §12.2*

### DS-701

`case-coverage`の未判断・判断結果はいずれも4検査の値へ写像せず、§11.3の集約へ寄与しない。

### DS-702

宣言targetのいずれか、または上流documentのいずれかを解決できない場合はバンドルを生成せず、候補のいずれも選択しない（§6.1）。

### DS-703

解決失敗の種別は対象不在（E-SCAN-004、document不在）を`MISMATCH`（診断`MISSING`）として当該対象の検証結果へ保持する。

### DS-704

解決失敗の種別は恒久SRC ID衝突による曖昧（E-SCAN-011）を`MISMATCH`として当該対象の検証結果へ保持する。

### DS-S101 8.3 提出スキーマ

### DS-705

判断は少なくともactor / subject / decisionを含む。

*導出元: REQ-180, REQ-181, REQ-182, REQ-183, REQ-184, REQ-185, REQ-186, REQ-187, REQ-188, REQ-189, REQ-190, REQ-191, REQ-192, SPEC-116, SPEC-117, SPEC-118, SPEC-119, SPEC-120, SPEC-121, SPEC-122, SPEC-333, SPEC-334, SPEC-335*

*引用: 基本仕様 §11.3, 要件定義 §12*

### DS-706

理由・根拠は任意（optional）とする。

*導出元: REQ-180, REQ-181, REQ-182, REQ-183, REQ-184, REQ-185, REQ-186, REQ-187, REQ-188, REQ-189, REQ-190, REQ-191, REQ-192, SPEC-116, SPEC-117, SPEC-118, SPEC-119, SPEC-120, SPEC-121, SPEC-122, SPEC-333, SPEC-334, SPEC-335*

*引用: 基本仕様 §11.3, 要件定義 §12*

### DS-707

`decision`の値集合はツールが受理する判断値（`accepted` / `rejected` / `deferred`等）とし、その妥当性を§8.4で検証する。

### DS-708

判断記録は検証状態を変更しない（§8 冒頭）。

### DS-709

`judgment_kind`は必須であり、`bundle_id`が指すバンドルの`judgment_kind`と一致しなければならない（§8.4）。

### DS-710

`supersedes`は任意であり、省略時は空listとして記録する。

### DS-711

`supersedes`に列挙する各ULIDは、同一`subject`かつ同一`judgment_kind`の既存判断記録を指さなければならない（§8.4）。

### DS-712

`reason` / `exclusions`は任意である。

### DS-713

`basis.kind`は`document` / `vo` / `test-code` / `target-code`のいずれかとする。

### DS-714

理由が空であることだけを根拠に判断を無効化しない。

*導出元: SPEC-116, SPEC-117, SPEC-118, SPEC-119, SPEC-120, SPEC-121, SPEC-122, SPEC-333, SPEC-334, SPEC-335*

*引用: 基本仕様 §11.3*

### DS-S102 8.4 提出の検証

### DS-715

`audit submit`は、bundle_idのバンドルがcacheに存在することを検証する（E-AUDIT-001）。

### DS-716

`audit submit`は、subjectがバンドルと一致することを検証する（E-AUDIT-003）。

### DS-717

`audit submit`は、judgment_kindがバンドルと一致し、§8.1の値域内であることを検証する（E-AUDIT-003）。

### DS-718

`audit submit`は、バンドル記録時の各対象の内容ハッシュが、現在のハッシュと一致することを検証する（対象が変更されていれば判断は無効。E-AUDIT-002）。

### DS-719

`audit submit`は、decisionが受理する判断値であることを検証する（E-AUDIT-004）。

### DS-720

`audit submit`は、supersedesの各ULIDが、同一subjectかつ同一judgment_kindの既存判断記録を指し、自己参照でないことを検証する（E-AUDIT-008）。

### DS-721

`subjects`に相当する対象集合はバンドル生成時の全対象の内容ハッシュを`subject_hash`と`dependencies`として記録し、依存closureのハッシュに束縛する。

### DS-722

理由（`reason` / `exclusions`）の有無を提出の受理条件にしない。

### DS-723

旧モデルのreasons / claim / basis必須検査（E-AUDIT-005）、decomposition-viewpoint検査（E-AUDIT-006）、spec / req basis検査（E-AUDIT-007）は要件定義§12「理由が空であることだけを根拠に無効扱いしない」と矛盾するため、判断記録層では課さない。

*導出元: REQ-180, REQ-181, REQ-182, REQ-183, REQ-184, REQ-185, REQ-186, REQ-187, REQ-188, REQ-189, REQ-190, REQ-191, REQ-192*

*引用: 要件定義 §12*

### DS-S103 8.5 有効性と再判断

### DS-724

判断記録が有効であるとは、judgment_kindが§8.1の値域内であり、subjectが一致し、subject_hashが現在の内容ハッシュと一致し、dependenciesが現在の上流依存closureとentity・hashとも完全一致することをいう。

> document は登録 content_hash と実ファイルの一致も要求。不一致の場合は当該 document を STALE とし、依存する判断記録も無効。

### DS-725

判断値が食い違う有効判断記録が併存する場合、機械はどれも選ばない。

### DS-726

V(subject, judgment_kind)は、「有効」を満たす判断記録の集合とする。

### DS-727

実効集合Eは、Vから、V内の他レコードのsupersedesに名指しされたものを除いた集合とする。

> supersede できるのは有効判断記録だけである。無効な判断記録の supersedes は何も除かない。

### DS-728

実効判断は、Eが空のとき未確定（UNKNOWN）とする。

### DS-729

実効判断は、Eの全レコードのdecisionが同一値であるとき、そのdecisionを実効判断値とする。

### DS-730

実効判断は、Eに2種以上のdecision値がある（競合）とき未確定（UNKNOWN）とし、W-STORE-004を出す。

### DS-731

実効判断が「未確定（`UNKNOWN`）」であることは、当該対象が§11のエスカレーション状態にとどまることを意味する。

### DS-732

実効判断が未確定（`UNKNOWN`）であり当該対象が§11のエスカレーション状態にとどまることは、§4.1の検証状態を変更せず、`UNKNOWN`に§4.2の診断ラベルを付与しない（§8 冒頭）。

### DS-733

競合は、新しい判断記録が旧判断記録を`supersedes`で明示に名指しして置き換えたときにだけ解消する。

### DS-734

判断記録の新旧（`decided_at` / ULID順）、`decision`値の優先順位（`rejected`優先等）、記録件数の多寡のいずれも解消規則に用いてはならない。

### DS-735

`supersedes`が循環する（レコード群が互いを名指ししてEが空になる）場合は未確定（`UNKNOWN`）とし、W-STORE-005を出す。

### DS-736

いずれかのレコードを推測で残さない。

### DS-737

`judgment_kind`を欠くか値域外の判断記録は、いずれの`(subject, judgment_kind)`のVにも属さず、実効判断へ寄与しない。

### DS-738

`judgment_kind`を欠くか値域外の判断記録は、履歴表示だけを許可し、W-STORE-003を出す。

### DS-739

判断記録を対象とする承認（§3.5の`judgment_ref`）は、当該判断記録が有効かつEに属する場合にだけ実効承認を導出する。

### DS-740

Eから外れた判断記録への承認は`draft`相当とする。

### DS-741

仕様・VO・Test等が変更された場合、過去の判断を現在状態へそのまま流用してはならず、現在状態に対して通常の検証（§5の4検査）を再実施する。

*導出元: REQ-180, REQ-181, REQ-182, REQ-183, REQ-184, REQ-185, REQ-186, REQ-187, REQ-188, REQ-189, REQ-190, REQ-191, REQ-192, SPEC-116, SPEC-117, SPEC-118, SPEC-119, SPEC-120, SPEC-121, SPEC-122, SPEC-333, SPEC-334, SPEC-335*

*引用: 基本仕様 §11.3, 要件定義 §12*

### DS-742

変更そのものが`UNKNOWN`を生成するのではない。

### DS-S104 8.6 参考プロンプト

### DS-743

判断の受理は検証状態を昇格させない。

## DS-S105 9. テスト実行設計

*導出元: SPEC-S022, SPEC-S051*

### DS-S106 9.1 実行対象の解決

### DS-744

`vtest run`は`--test` / `--vo` / `--all`で対象を受け取り、検証グラフからTest集合へ展開する（VO指定は部分木のcoversを辿る）。

### DS-S107 9.2 `rust-cargo` TestRunnerAdapter

### DS-745

orchestrationは`ExecutionDescriptor.adapter`をregistryで解決し、adapter不一致（E-ADAPTER-003）を拒否する。

### DS-746

明示的なrunでrunner未提供ならE-ADAPTER-004としてEvidenceを生成せず、検証集約の`target_binding`は`NO_EVIDENCE`（診断`NOT_EXECUTED`）とする。

### DS-S108 9.3 `rust-cargo` 結果のパース

### DS-747

`test <selector> ... ignored`が実行されずの場合、Evidenceは記録しない。

### DS-748

`test <selector> ... ignored`が実行されずの場合、target_bindingは診断NOT_EXECUTEDとする。

### DS-749

要求した各フィルタについて結果行が得られなかった場合、そのTestの実行は失敗（E-EXEC-002）とし、Evidenceを記録しない。

### DS-750

プロセス終了コードと結果行の集計が矛盾する場合もE-EXEC-003とする。

### DS-S109 9.4 Evidence の記録

### DS-751

Testごとに§3.6のレコードを1件生成する。

### DS-752

`revision`の取得失敗時は`commit: null`とし、このEvidenceは鮮度（§11.2）のrevision一致を満たさず`target_binding`の有効な`PASS`にならない。

### DS-753

`hashes`は欠落・重複を許可しない。

### DS-754

全宣言targetがcanonical Source Targetへ一意に解決できることをEvidence生成の前提とする。

### DS-755

1件でも「対象なし」または「曖昧」（E-SCAN-004 / E-SCAN-011）ならEvidenceを生成しない。

### DS-756

全宣言targetのうち1件でも「対象なし」または「曖昧」（E-SCAN-004 / E-SCAN-011）の場合、`target_binding`は`NO_EVIDENCE`（診断`NOT_EXECUTED`）のままとし、target解決の診断で非`PASS`を示す。

### DS-757

完全性を保証できない場合は`complete: false`とし、後続の鮮度を`PASS`にしない。

### DS-758

ビルド失敗（コンパイルエラー）の場合、対象Test群のEvidenceは記録せずE-EXEC-001を報告する。

### DS-759

`target_binding`は`NO_EVIDENCE`（診断`NOT_EXECUTED`）のままとなる。

## DS-S110 10. `rust-cargo` Target Binding 動的計測

*導出元: SPEC-S020, SPEC-S051*

### DS-S111 10.1 計測方式

### DS-760

利用不能な場合、Evidenceの`target_coverage`を`checked: false`（検証時`NO_EVIDENCE`、診断`NOT_CHECKED`）とし診断W-EXEC-101を出す（`PASS`へ変換しない）。

*導出元: SPEC-061, SPEC-062, SPEC-063, SPEC-064, SPEC-065, SPEC-289*

*引用: 基本仕様 §5.3*

### DS-761

起動される実行体をinstrument対象とし子プロセスのprofileをmergeすることを提供できない構成では境界越しtargetをUNKNOWNとする。

### DS-762

計測不能なら`target_coverage.checked: false`（`NO_EVIDENCE`/`NOT_CHECKED`）とし、能力の有無で計測結果を捏造しない。

### DS-763

Testが起動したsubprocess・spawnしたthreadの実行を宣言targetへ帰属させられる能力の実装可否は§7.3のruntime到達証明がsubprocess E2Eに及ぶかを左右するが、欠如時もfail-closedを保つ（DA-002はUNKNOWNのまま）。

### DS-764

提供されない場合は`target_coverage.checked: false`（`NO_EVIDENCE`/`NOT_CHECKED`）とする。

### DS-765

解析限界は`UNKNOWN`とし、測定済み`PASS`を推測しない。

### DS-S112 10.2 判定

### DS-766

target別判定は、count > 0なら`PASS`とする。

### DS-767

target別判定は、count == 0なら`FAIL`（診断NOT_EXECUTED）とする。

*導出元: SPEC-049*

*引用: 基本仕様 §4.3*

### DS-768

target別判定は、関数が見つからなければ`UNKNOWN`（インライン化・cfg除外等の可能性）とする。

### DS-769

Test単位集約は、FAILが1件以上あれば`FAIL`とする。

### DS-770

Test単位集約は、FAILなし、UNKNOWNが1件以上あれば`UNKNOWN`とする。

### DS-771

Test単位集約は、1件以上の全宣言targetがPASSなら`PASS`とする。

### DS-772

target別entryの欠落、重複、余分なentry、または解決後のcanonical Source Target集合との不一致を`PASS`として保存しない。

### DS-773

Evidenceの`target_coverage`へ記録する計測結果は§7.3のtarget_binding runtime証明の証拠源であり、独立の検査項目ではない。

### DS-774

Testが別プロセス（起動したsubprocess内）・別スレッド等の実行境界越しにtargetを到達させる場合も、判定は実行countに基づく。

### DS-775

providerが境界越しの実行を帰属できない場合はそのtargetを`UNKNOWN`（関数不見当扱い）とする。

### DS-776

計測自体が不能なら`target_coverage.checked: false`とする。

### DS-777

providerが境界越しの実行を帰属できない場合、または計測自体が不能な場合は、いずれも§7.3のruntime到達証明を成立させず、静的到達のUNKNOWNを`PASS`へ変換しない。

### DS-S113 10.3 実行モードの整理

### DS-778

`--fast`モードはcargo testのみとし、`target_coverage.checked: false`で記録し、検証時は`NO_EVIDENCE`（診断`NOT_CHECKED`）とする。

### DS-779

既定モード（完全検証向け）はcargo-llvm-covによるTest単位実行とする。

> 実行時間と引き換えに `target_binding` の動的証拠を得る。

## DS-S114 11. 鮮度検証と集約

### DS-S115 11.1 検査の評価地点

*導出元: SPEC-S017, SPEC-S052*

### DS-780

`chain_integrity`は評価地点をrepository scan result / DOC / VO / TESTとし、文書鎖（document derives_from・content_hash）、VOのderives_from（document 1件以上）、Testの管理宣言（Test ID・covers ≥ 1・その他の必須metadata〔intent、および当該adapterが必須とする追加metadata。rust-cargoではtargets ≥ 1〕）・covers参照解決・Test ID大局的一意性がすべて成立すれば`PASS`とする（§11.1.1）。

> 4検査（基本仕様§5）は、次の地点で評価する。表: | 検査 | 評価地点 | 評価方法 |

*導出元: SPEC-053, SPEC-054, SPEC-055, SPEC-056, SPEC-057, SPEC-058, SPEC-059, SPEC-060, SPEC-061, SPEC-062, SPEC-063, SPEC-064, SPEC-065, SPEC-066, SPEC-067, SPEC-068, SPEC-069, SPEC-287, SPEC-288, SPEC-289, SPEC-290, SPEC-291, SPEC-292, SPEC-293*

*引用: 基本仕様 §5*

### DS-781

`orphan_detection`は評価地点をDOCとし、親を持たず`doc.roots`にも列挙されないdocumentが無ければ`PASS`、あれば`MISMATCH`とする（§5.6）。

### DS-782

`target_binding`は、Evidence result FAILは`FAIL`、全宣言targetの到達が静的到達またはruntime到達で充足されれば`PASS`とする。

### DS-783

`oracle_presence`は、全PASSで`PASS`、1つでもFAILで`FAIL`、FAILなくUNKNOWNで`UNKNOWN`とする。

### DS-784

4検査の評価入力は、当該revisionのrepositoryを走査したscan結果（adapterが返すdiscovery出力と、そこからcoreが具体化したエンティティ・内容ハッシュ）、`.verify/`配下の正典ファイル集合（`config.yaml`、documentレコード、VOレコード、Relationレコード、判断記録〔`.verify/decisions/`〕、承認レコード〔`.verify/approvals/`〕、Evidenceレコード〔`.verify/evidence/`〕）、Evidence鮮度判定（§11.2）が現在のsnapshotとして再構築するExecution State subjectの入力（toolchain identity、実行結果へ影響するadapter configのcanonical projection、repository / local dependencyの入力manifest。§1.3）、および当該実行の要求scope指定（検査軸・エンティティ軸・`--gate`）に限る。

*導出元: REQ-224, REQ-225, REQ-226, REQ-227, SPEC-109, SPEC-110, SPEC-111, SPEC-112, SPEC-113, SPEC-319, SPEC-320, SPEC-321, SPEC-322, SPEC-323, SPEC-324, SPEC-325, SPEC-326, SPEC-327, SPEC-328, SPEC-329, SPEC-330, SPEC-331, SPEC-332*

*引用: 基本仕様 §11.1, 要件定義 §17.2*

### DS-785

4検査の評価入力は、当該revisionのrepositoryを走査したscan結果（adapterが返すdiscovery出力と、そこからcoreが具体化したエンティティ・内容ハッシュ）を含む。

### DS-786

4検査の評価入力は、`.verify/`配下の正典ファイル集合（`config.yaml`、documentレコード、VOレコード、Relationレコード、判断記録〔`.verify/decisions/`〕、承認レコード〔`.verify/approvals/`〕、Evidenceレコード〔`.verify/evidence/`〕）を含む。

### DS-787

4検査の評価入力は、Evidence鮮度判定（§11.2）が現在のsnapshotとして再構築するExecution State subjectの入力（toolchain identity、実行結果へ影響するadapter configのcanonical projection、repository / local dependencyの入力manifest。§1.3）を含む。

### DS-788

4検査の評価入力は、当該実行の要求scope指定（検査軸・エンティティ軸・`--gate`）を含む。

### DS-789

4検査の評価入力の集合が同一であれば、4検査の検証状態（5状態）・診断ラベル・診断コード集合・集約結果・`pending` sectionの内容・終了コードは同一でなければならない。

> 次を、それ自体として検査入力にしてはならない。

### DS-790

実行時の現在時刻・経過時間・乱数・プロセスIDを、それ自体として検査入力にしてはならない。

### DS-791

ロケール・タイムゾーン・環境変数・呼出し元の作業ディレクトリ（`--project`で解決したプロジェクトルート自体は入力に含む）を、それ自体として検査入力にしてはならない。

### DS-792

ネットワーク応答、およびLLM APIを含む外部サービスの応答を、それ自体として検査入力にしてはならない。

### DS-793

環境の変化が結果へ影響しうるのは、Execution State subjectの入力（toolchain identity・adapter config・入力manifest）を変える範囲に限る。

### DS-794

Execution State subjectの入力（toolchain identity・adapter config・入力manifest）を変える範囲で環境の変化が結果へ影響する場合の影響はEvidenceの鮮度喪失（`NO_EVIDENCE`、診断`STALE`。§11.2）として現れ、環境そのものを判定条件として読むわけではない。

### DS-795

ネットワーク応答と外部サービス応答はExecution State subjectの入力に含まれないため、例外なく検査入力にならない。

### DS-796

判断記録の受理は検証状態を昇格させない（§8.3）。

### DS-797

将来そのようなseamを評価経路へ設ける場合は、任意の判定を返す実装（正反対の判定を返す実装を含む）を差し替えても4検査の結果が変化しないことを満たさなければならない。

#### DS-S116 11.1.1 `chain_integrity` の評価

*導出元: SPEC-S018, SPEC-S027, SPEC-S035*

### DS-798

いずれか違反があれば`MISMATCH`（切れた箇所を診断ラベルで示す）とする。

### DS-799

文書層は、各`document`の`derives_from`参照先が存在すること（E-SCAN-012）を評価する。

### DS-800

文書層は、`content_hash`が現物と一致すること（不一致は診断`STALE`。§11.4）を評価する。

### DS-801

VO層は、各VOが1件以上の`document`への解決可能な`derives_from`を持つこと（不在・解決不能はE-SCAN-012）を評価する。

### DS-802

VO層は、VO parentの不在・循環をE-SCAN-008とする。

### DS-803

VO層は、`combinations`が§3.2.1の受理条件を満たすこと（違反はE-SCAN-017）を評価する。

### DS-804

Test層は、発見された各Testに対応する管理宣言（構文上有効なTest ID・1件以上の`covers`・`intent`その他の必須metadata。`targets ≥ 1`はadapter中立coreの必須リンクに含めず、当該adapterが必須とする追加metadataとして扱う〔`rust-cargo`では1件以上の`targets`〕。§4.1・基本仕様 §5.1・§9.1）がちょうど1件存在すること（欠落はE-SCAN-007、診断`MISSING`）を評価する。

*導出元: SPEC-057, SPEC-058, SPEC-082, SPEC-083, SPEC-084*

*引用: 基本仕様 §5.1・§9.1, 基本仕様 §5.1, 基本仕様 §9.1*

### DS-805

Test層は、`covers`の全VO参照を解決できること（E-SCAN-003）を評価する。

### DS-806

Test層は、Test IDが発見結果全体で一意であること（衝突はE-SCAN-002）を評価する。

### DS-807

双方向完全性は、leaf VO → Test（検証実装の存在。coversするTestが1件以上）と、発見されたTest → 宣言（管理宣言の解決）の両方向が成立して初めて成立する。

### DS-808

coversするTestの無いleaf VOは`MISMATCH`（診断`MISSING`）とする。

### DS-809

Relationのfrom / to不在はE-SCAN-009とする。

### DS-810

恒久SRC IDのadapter越え衝突はE-SCAN-011とする。

### DS-811

旧モデルの`role`に基づく`covers`可変制約・適用項目集合は設けず、すべての管理対象Testに`covers ≥ 1`を一律要求する。

*導出元: SPEC-123, SPEC-124, SPEC-369, SPEC-370, SPEC-371, SPEC-372, SPEC-373, SPEC-374, SPEC-375, SPEC-376, SPEC-377, SPEC-378, SPEC-379, SPEC-380, SPEC-381, SPEC-382, SPEC-383, SPEC-384, SPEC-385, SPEC-386, SPEC-387, SPEC-388, SPEC-389, SPEC-390, SPEC-391, SPEC-392, SPEC-393, SPEC-394, SPEC-395, SPEC-396, SPEC-397, SPEC-398, SPEC-399, SPEC-400, SPEC-401, SPEC-402, SPEC-403, SPEC-404, SPEC-405, SPEC-406, SPEC-407*

*引用: 基本仕様 §12*

### DS-812

`covers`を持たない（0件の）Testは管理宣言不整合として`chain_integrity = MISMATCH`であり、特別扱いの分岐を設けない。

### DS-S117 11.2 Evidence 鮮度判定（target_binding の証拠有効性）

*導出元: SPEC-S020*

### DS-813

対象Testの Evidenceのうち最新のものについて、evidence.hashes.test_subject == 現在のTest subject hashを検査する。

### DS-814

対象TestのEvidenceのうち最新のものについて、evidence.hashes.targetsの参照集合が、現在のTest.targetsを§6.1で解決したcanonical Locator集合と重複なく一致し、各target_constructが現在のimplementation construct hashと一致することを検査する。

### DS-815

対象TestのEvidenceのうち最新のものについて、evidence.revision.commitが非nullかつ現在のHEAD revisionと一致することを検査する。

### DS-816

対象TestのEvidenceのうち最新のものについて、evidence.execution_state.complete == trueかつ、同じschemaで現在再構築したExecution State subjectがcompleteで、hashが一致することを検査する。

### DS-817

対象TestのEvidenceのうち最新のものについて、evidence.adapterが現在のTest.execution.adapterと一致することを検査する。

> adapter欠落形は§3.6の互換条件で一意に確認できる。

### DS-818

evidence.hashes.test_subject == 現在のTest subject hashであること、evidence.hashes.targetsの参照集合が現在のTest.targetsを§6.1で解決したcanonical Locator集合と重複なく一致し各target_constructが現在のimplementation construct hashと一致すること、evidence.revision.commitが非nullかつ現在のHEAD revisionと一致すること、evidence.execution_state.complete == trueかつ同じschemaで現在再構築したExecution State subjectがcompleteでhashが一致すること、およびevidence.adapterが現在のTest.execution.adapterと一致することのすべてが成立する場合、当該Evidenceは現在の証拠として有効とする（dirty: trueでもExecution State subject一致なら有効。実行入力manifestが実体を保証する）。

### DS-819

evidence.hashes.test_subjectが現在のTest subject hashと一致しない場合、または、evidence.hashes.targetsの参照集合が現在のTest.targetsを§6.1で解決したcanonical Locator集合と重複なく一致しない場合（各target_constructが現在のimplementation construct hashと一致しない場合を含む）、`NO_EVIDENCE`（診断STALE）とする。

### DS-820

evidence.revision.commitが非nullでない、または現在のHEAD revisionと一致しない場合、`NO_EVIDENCE`（診断STALE。現在revisionに対する実行ではない）とする。

### DS-821

evidence.execution_stateのrecordが欠落している場合、またはhashが一致しない場合、`NO_EVIDENCE`（診断STALE）とする。

### DS-822

evidence.execution_state.completeがtrueでない、または現在再構築したExecution State subjectを完全に構築不能の場合、`UNKNOWN`とする。

### DS-823

Evidenceのadapterが現在のTestのexecution.adapterと明示的に不一致の場合、`MISMATCH`とする。

### DS-824

Evidenceのadapterが現在のTestのexecution.adapterと一致するかを確認不能の場合、`UNKNOWN`とする。

### DS-825

Evidenceなしの場合、`NO_EVIDENCE`（診断NOT_EXECUTED）とする。

### DS-826

Evidenceは全宣言targetが一意に解決できる場合だけ生成される（§9.4）。

### DS-827

現在の宣言targetのうち1件でもcanonical Source Targetへ一意に解決できなくなった場合、記録済み参照集合は現在のcanonical集合と一致しないため、evidence.hashes.targetsの参照集合が現在のTest.targetsを§6.1で解決したcanonical Locator集合と重複なく一致するという条件は成立せず、`target_binding`を有効な`PASS`にしない。

### DS-828

対象が存在せずE-SCAN-004となるtargetは`MISMATCH`（診断`MISSING`）として保持する（§5.4）。

### DS-829

複数候補により曖昧でE-SCAN-011となるtargetは`MISMATCH`として保持する（§5.4）。

### DS-830

有効なEvidenceが得られたとき、`result: FAIL`（テストランナーが失敗を報告）なら`target_binding`は`FAIL`とする。

*導出元: REQ-094, REQ-095, REQ-096, REQ-097, REQ-098, REQ-099, REQ-100, REQ-101, REQ-102, REQ-103, REQ-104*

*引用: 要件定義 §5.3*

### DS-831

有効なEvidenceが得られたとき、`result: PASS`かつ全宣言targetの到達要件が§7.3で充足（静的到達またはruntime到達）されれば`target_binding`は`PASS`とする。

### DS-832

有効なEvidenceが得られたとき、`result: PASS`だが到達未充足のtargetがある場合、当該targetの`target_coverage`に従い、count 0は`FAIL`（診断NOT_EXECUTED）、計測不能・未計測（`checked: false`）は`NO_EVIDENCE`（診断NOT_CHECKED）、関数不見当は`UNKNOWN`とする。

### DS-833

Evidenceが存在するが有効でない場合、`target_binding`はEvidenceを再利用せず、上表の`MISMATCH` / `NO_EVIDENCE`（STALE）/ `UNKNOWN`を保持する。

### DS-834

Evidenceが無ければ`NO_EVIDENCE`（診断`NOT_EXECUTED`）とする。

### DS-835

複数条件が非`PASS`なら根拠をすべて保持し、表示代表値は基本仕様 §22.2の優先順位で選ぶ（診断ラベルは順位に用いず併記する）。

*導出元: SPEC-190, SPEC-191*

*引用: 基本仕様 §22.2*

### DS-S118 11.3 集約アルゴリズム

*導出元: SPEC-S052*

### DS-836

項目scopeが省略された場合、aggregatorはconfig値から部分集合を組み立てず、基本仕様 §5の固定4検査を選択する。

*導出元: SPEC-053, SPEC-054, SPEC-055, SPEC-056, SPEC-057, SPEC-058, SPEC-059, SPEC-060, SPEC-061, SPEC-062, SPEC-063, SPEC-064, SPEC-065, SPEC-066, SPEC-067, SPEC-068, SPEC-069, SPEC-287, SPEC-288, SPEC-289, SPEC-290, SPEC-291, SPEC-292, SPEC-293*

*引用: 基本仕様 §5*

### DS-837

明示的な部分集合だけを限定scopeとし、その結果を完全検証として表示しない。

### DS-838

aggregateは、chain_integrity / orphan_detectionをrepository / DOC / VO / TEST構造に対して評価する。

### DS-839

aggregateは、scopeのエンティティ軸でDOC/VO/TEST部分木を選択する。

### DS-840

aggregateは、各TESTについて、scopeの検査軸に含まれるtarget_binding / oracle_presenceを評価する（含まれない検査はNO_EVIDENCE、診断NOT_CHECKED）。

### DS-841

aggregateは、各leaf VOについてcoversするTEST群の結果をfail-closedで合成する。

### DS-842

aggregateは、子VOを持つVO（親VO）について、子VOの値と、当該親VOを直接coversするTESTの値を合わせてfail-closedで合成する（直接coversするTESTが無ければ子VOの値だけを合成する）。

### DS-843

aggregateは、DOCについて下流VO部分木の合成（fail-closed）を行う。

### DS-844

総合判定は、構造検査（chain_integrity / orphan_detection）とentity treeのscope内評価がすべてPASSならOK、それ以外ならNGとする。

### DS-845

fail-closed合成は、子にFAIL/MISMATCH/NO_EVIDENCE/UNKNOWNが1つでもあれば親を非PASSとする。

### DS-846

fail-closed合成の代表値は基本仕様 §22.2の優先順位FAIL > MISMATCH > NO_EVIDENCE > UNKNOWNで選ぶ。

*導出元: SPEC-190, SPEC-191*

*引用: 基本仕様 §22.2*

### DS-847

診断ラベル（MISSING / NOT_EXECUTED / NOT_CHECKED / STALE）は順位に用いず併記する。

### DS-848

検査の表示scopeと、検査導出に必要な内部依存の評価は分離する。

### DS-849

§7.3により`target_binding`は当該TestのEvidence鮮度（§11.2）とtarget別`target_coverage`へ依存する。

### DS-850

`target_binding`が項目scopeに含まれる場合、aggregatorは§7.3のruntime到達証明の判定に必要な範囲でこれらを内部依存として評価する。

### DS-851

runtime証明に依存する`target_binding`の値は、根拠として用いたEvidence IDと当該targetの`target_coverage`結果をreportで引用し、原因を辿れる状態にする。

### DS-852

`covers`を持つTestはcovers先それぞれのVOの合成に独立に参加する。

### DS-853

「1つのTestが複数VOを検証していること」自体は許容し、各leaf VOの充足と組合せは§3.2.1の実体化されたleaf VO単位で判定する。

*導出元: SPEC-092, SPEC-093, SPEC-094, SPEC-095, SPEC-096, SPEC-097, SPEC-098, SPEC-099, SPEC-100, SPEC-101, SPEC-102, SPEC-103, SPEC-104, SPEC-105, SPEC-190, SPEC-191, SPEC-318*

*引用: 基本仕様 §10、§22.2*

### DS-854

親VOの値は、子VOの値と当該親VOを直接coversするTESTの値を合わせたfail-closed合成そのものであり、機能単位の表示のために別の合成規則・緩和規則を設けない。

### DS-855

子に1つでも非`PASS`があれば親VOは非`PASS`であり、代表値の優先順位も基本仕様 §22.2と同一とする。

*導出元: SPEC-190, SPEC-191*

*引用: 基本仕様 §22.2*

### DS-856

Testの結果が親VOへ寄与する経路は、(a) coversするleaf VO経由の伝播と、(b) 当該親VOを直接coversするTestの直接参加の2つに限る。

### DS-857

covers宣言を経由しない「機能名による束ね」（ファイルパス・モジュール名・命名規約からの推定束ね）を設けない。

### DS-858

親VOを持たないleaf VOは、それ自体が最上位の束ね単位となる。

### DS-859

DOC単位の集約は下流VO部分木の合成であり、機能単位の集約はその中間段に位置する。

### DS-S119 11.4 document 鮮度

### DS-860

スキャン時にdocumentレコードの`content_hash`と実ファイル（`path`）を比較し、不一致ならW-SCAN-104を出す。

### DS-861

当該documentを`derives_from`で参照するVO / 上位documentの鎖は、content_hash不一致として`chain_integrity = MISMATCH`（診断`STALE`）となる（§11.1.1）。

### DS-862

当該document subjectをdependencyに含む判断記録（§8.5）・承認記録（§3.5）も無効となる。

### DS-863

再登録でdocument subject hashが変化するため、以前のdependency entryを現在の承認・判断へ流用しない。

### DS-S120 11.5 フェーズゲート評価

*導出元: REQ-S057, SPEC-S050*

### DS-864

`vtest verify --gate <name>`は、指定ゲートの対象scopeについて検証を実行し、(1) 検証結果が`require.verification`を満たすか、(2) `require.approvals`の各ロールについて対象の実効承認状態（§3.5）が`approved`であるか、を評価して満否と根拠（不足している非`PASS`検査・未充足の承認ロール）を提示する。

### DS-865

`--gate <name>`は`gates[].name`との大文字小文字を区別した完全一致で解決する。

### DS-866

`--gate <name>`の解決は前方一致・部分一致・近似一致・既定ゲートへの代替は行わない。

### DS-867

一致するゲート定義が無い場合（`gates`が空、または未定義名の指定）はusage errorとしてE-CONFIG-002（終了コード2）で拒否し、スキャン・検証・ゲート評価のいずれも実行せず、検証結果・部分結果を生成しない。

### DS-868

診断には指定名と定義済みゲート名の一覧を含める。

### DS-869

検証条件の充足判定は、`require.verification`の値と、要求scopeの集約代表値との完全一致でのみ充足する。

### DS-870

集約代表値は、要求scope内で評価した全値（構造検査`chain_integrity` / `orphan_detection`と、エンティティ軸の部分木で評価した各Test / VO / DOCの検査値）を§11.3のfail-closed規則で合成した1値とする。

### DS-871

全値が`PASS`なら代表値は`PASS`（総合OKと同値）、非`PASS`が混在する場合は基本仕様 §22.2の優先順位`FAIL > MISMATCH > NO_EVIDENCE > UNKNOWN`で選ぶ。

*導出元: SPEC-190, SPEC-191*

*引用: 基本仕様 §22.2*

### DS-872

診断ラベルは充足判定に用いない。

### DS-873

5状態に順序・優劣・包含関係を設けない。

### DS-874

「要求値以上」「要求値より良い」といった比較解釈を採らず、`require.verification: UNKNOWN`は代表値が`UNKNOWN`のときだけ充足し、代表値が`PASS`でも充足しない。

### DS-875

同様に`require.verification: PASS`は代表値が`PASS`のときだけ充足する。

### DS-876

`--items`で検査軸を限定した実行では、scope外の検査が`NO_EVIDENCE`（診断`NOT_CHECKED`）として代表値の合成に参加する（§11.3、基本仕様 §4.6）。

*引用: 基本仕様 §4.6*

### DS-877

したがって限定scopeでの`require.verification: PASS`は充足せず、限定scopeの結果でゲートを充足させることはできない。

### DS-878

承認条件は検証条件と独立に評価し、`require.approvals`が空集合（省略）なら承認条件は充足とする。

### DS-879

承認未充足は検証状態を降格させず、検証の非`PASS`は承認の充足有無を変えない（基本仕様 §4.5）。

*導出元: SPEC-051, SPEC-052*

*引用: 基本仕様 §4.5*

### DS-880

ゲート全体の充足は検証条件と承認条件の両方が充足した場合に限る。

### DS-S121 11.6 役割別 projection

*導出元: REQ-S007, SPEC-S049, SPEC-S054*

### DS-881

当該親VOの代表値と、その配下の子VOごと・Testごとの内訳を同じ出力から辿れる。

### DS-882

`anchor`を持たないentryでは当該fieldを省略または`null`とし、空文字列で埋めない。

### DS-883

`anchor`の値は不透明な文字列として transport するだけで、projectionは文書内位置への解決・整合検査を行わない。

### DS-S122 11.7 判断待ち情報の構造

*導出元: SPEC-S047, SPEC-S066*

### DS-884

判断待ち情報は構造化record（report JSON内のsection）として提示する。

### DS-885

4検査のいずれにも由来しない項目（判断型に由来する項目・判断競合）では`check`を`null`とする。

### DS-886

`check`が`null`の項目は§11.3の集約へ寄与せず、いかなる検査の値も変更しない。

### DS-887

不要な項目では`judgment_kind`を`null`とする。

### DS-888

`judgment_kind: case-coverage`の項目（`kind: unknown`、`check: null`、`subject`＝対象Test ID）は、条件をすべて満たす管理対象Testごとにちょうど1件生成する。

### DS-889

判断型に由来する項目の生成条件の1つは、`covers`が1件以上あることである。

### DS-890

判断型に由来する項目の生成条件の1つは、当該Testの`cases`が1件以上ある、または解決済みのcovers先VO（レコードが存在するVO。E-SCAN-003のdangling参照を除く）のいずれかが`dimensions`を1件以上持つことである。

### DS-891

判断型に由来する項目の生成条件の1つは、`(当該Test, case-coverage)`の実効判断（§8.5）が`accepted`でないことである。

### DS-892

実効判断が未確定・`rejected`・`deferred`のいずれの場合も項目を生成し、参照した判断記録IDを`basis`に載せる。

### DS-893

判断型に由来する項目の生成条件は`case-coverage`型の項目にだけ適用する。

### DS-894

検査に由来する`kind: unknown`の項目（DA規則の解析限界等）の生成・消滅は当該検査の値だけで決まり、判断記録の有無で変わらない。

### DS-895

§8.5の実効判断が競合により未確定となった`(subject, judgment_kind)`は、`kind: unknown`、`check: null`、当該`judgment_kind`、および競合した全判断記録IDを`basis`（`kind: decision`）に持つ項目として提示する。

## DS-S123 16. 並列動作と整合性

### DS-S124 16.1 ロック不要の根拠

*導出元: SPEC-S057*

### DS-896

解析不能な中間状態はadapter discoveryのE-SCAN-001 / Incompleteとしてfail-closedに検出される（§5.1）。

### DS-S125 16.2 意味的衝突検出

*導出元: SPEC-S057*

### DS-897

ID衝突はE-SCAN-002として検出する。

### DS-898

dangling referenceはE-SCAN-003 / E-SCAN-009 / E-SCAN-012として検出する。

### DS-899

孤児documentはE-SCAN-016として検出する。

### DS-900

承認の失効は§3.5のハッシュ束縛により自動的にdraftへ遷移する。

### DS-901

判断記録・Evidenceの失効は§8.5 / §11.2のハッシュ束縛により自動的に無効（診断STALE）へ遷移する。

## DS-S126 17. 診断・終了コード体系

### DS-S127 17.1 診断コード / 診断コード表

*導出元: SPEC-S013, SPEC-S066*

### DS-902

`E-SCAN-017`はerrorであり、VOの`combinations`が不正（`coverage_policy: explicit`で欠落・空、`explicit`以外で非空、未宣言dimension・未列挙partitionの参照、宣言dimensionの欠落・重複、重複tuple。§3.2.1）である。当該VOの`chain_integrity`を`MISMATCH`とし、`vo expand`は子VOを生成しない。

> 表: | コード | 種別 | 内容 |

### DS-903

`W-SCAN-104`はwarningであり、documentレコードのcontent_hashと実ファイルの不一致である（依存判断・依存Approvalは無効、鎖はchain_integrity STALE）。

### DS-904

`E-EXEC-001`はerrorであり、テストビルド失敗である。

### DS-905

`E-EXEC-002`はerrorであり、要求したテストの結果行が得られないことである。

### DS-906

`E-EXEC-003`はerrorであり、終了コードと結果行集計の矛盾である。

### DS-907

`E-EXEC-004`はerrorであり、実行中にExecution State subjectが変化することである。

### DS-908

`W-EXEC-101`はwarningであり、カバレッジツール利用不能である（target_coverageはchecked: false、検証時NO_EVIDENCE/NOT_CHECKED）。

### DS-909

`E-AUDIT-001`はerrorであり、提出されたbundle_idが存在しないことである。

### DS-910

`E-AUDIT-002`はerrorであり、バンドル記録時のハッシュと現在のハッシュの不一致（対象が変更済）である。

### DS-911

`E-AUDIT-003`はerrorであり、subjectまたはjudgment_kindの不一致・値域外・スキーマ違反である。

### DS-912

`E-AUDIT-004`はerrorであり、decisionが受理する判断値でないことである。

### DS-913

`E-AUDIT-008`はerrorであり、supersedesの参照先が存在しない、subjectまたはjudgment_kindが一致しない、または自己参照であることである（§8.4）。

### DS-914

`E-APPROVAL-001`はerrorであり、Approval対象、`judgment_ref`の参照先、または上流依存closureを完全・currentに解決できず、recordを生成しないことである。

### DS-915

`E-APPROVAL-002`はerrorであり、`approved_state`が値域外、`subject`の種別が値域外（判断記録ULID・Test ID等）、または`supersedes`の参照先が存在しない・対象が一致しない・自己参照であることである（§3.5。recordを生成しない）。

### DS-916

`E-CONFIG-001`はerrorであり、config version、`verify.full_scope`（固定4検査）、`doc.roots`、`gates`（名前重複、`require` / `require.verification`欠落、`require.verification`が5状態語彙外、`require.approvals`の不正・未解決ロール）、config field型または登録adapterが検証する設定値が現在のconfig invariantに違反することである（未知・重複adapter IDはE-ADAPTER-001）。

### DS-917

`E-CONFIG-002`はerrorであり、呼出しがconfigに定義の無いゲート名を参照することである（`--gate` / MCPの`gate`入力。config内容自体はinvariantを満たす。検証・ゲート評価を実行せず結果を生成しない。§11.5）。

### DS-918

`E-OP-001`はerrorであり、Structured Operationの入力検証失敗（候補提示を伴う。§6.3）である。

### DS-919

`E-OP-002`はerrorであり、Edit対象Testの特定失敗である。

### DS-920

`E-OP-003`はerrorであり、Create / Editの適用後検証に失敗（再パース不能、生成された宣言がdesired stateと不一致、変更が1 Testの範囲を超える）することである。適用前の状態へロールバックし操作を中止する（別紙A §15.2・§15.4）。

*引用: 別紙A §15.2・§15.4, 別紙A §15.2, 別紙A §15.4*

### DS-921

`E-ADAPTER-001`はerrorであり、adapterが未登録、重複、またはregistryの宣言と実装が不一致であることである。

### DS-922

`E-ADAPTER-002`はerrorであり、adapterのdiscoveryまたはrunnerが確定的に失敗（Evidenceなし）することである。

### DS-923

`E-ADAPTER-003`はerrorであり、Testのexecution descriptorと選択adapterが不一致であることである。

### DS-924

`E-ADAPTER-004`はerrorであり、明示操作に必須のadapter capabilityが未提供（変更・判断・Evidenceなし）であることである。

### DS-925

`W-ADAPTER-101`はwarningであり、検証対象のadapter capabilityが未提供であることである（能力に応じNO_EVIDENCE/NOT_CHECKEDまたはNOT_EXECUTED）。

### DS-926

`W-ADAPTER-102`はwarningであり、adapterが解析限界を報告することである（該当検査はUNKNOWN）。

### DS-S128 17.2 終了コード

*導出元: SPEC-S015, SPEC-S061, SPEC-S066*

### DS-927

終了コード`0`は、要求scopeの検証結果がOK（操作コマンドでは成功）であることを意味する。

> 表: | コード | 意味 |

### DS-928

終了コード`1`は、検証結果がNGであることを意味する。

### DS-929

終了コード`2`は、操作拒否（E-OP-* / E-ADAPTER-* / E-APPROVAL-* / E-CONFIG-*、引数不正、adapter前提・capability・実行失敗、スキーマ違反の提出など。検証結果は生成しない）であることを意味する。

### DS-930

終了コード`3`は、内部エラー（ツール自体の異常）であることを意味する。

### DS-931

`--gate <name>`を指定した`vtest verify` / `vtest report`では、0と1をゲート充足で決める。

### DS-932

ゲート全体が充足（§11.5の検証条件と承認条件の両方が充足）なら0、いずれかが不充足なら1とする。

### DS-933

`require.verification`に`PASS`以外を定義したゲートでは、集約代表値が要求値と一致して充足した実行が0になり、この場合に総合がNGであることは0を妨げない。

### DS-934

ゲート名が未定義の場合はE-CONFIG-002で2とし、0 / 1を返さない。

### DS-935

`vtest scan` / `vtest doctor`では、registry・config・adapter契約の検証またはadapter呼出しがE-ADAPTER-* / E-CONFIG-*で拒否された場合は2とする。

### DS-936

`vtest scan` / `vtest doctor`では、scanが完了してrepository整合性のE-SCAN-*を報告した場合は1とする。

### DS-937

`vtest scan` / `vtest doctor`では、errorがなければ0とする。

### DS-938

同一実行に複数候補がある場合は内部エラー3、操作拒否2、検証NG1、成功0の順で優先する。

## DS-S129 12. CLI 詳細仕様

### DS-S130 12.1 共通仕様

*導出元: REQ-S009, SPEC-S012, SPEC-S013, SPEC-S055, SPEC-S078, SPEC-S096, SPEC-S106*

### DS-939

確認プロンプトを出す場合は `--yes` で抑止できる。

### DS-940

出力は既定で人間向けテキスト、`--format json` で機械可読 JSON。

### DS-941

JSON 出力は最上位に `{ "ok": bool, "data": ..., "diagnostics": [...] }` を持つ。

### DS-942

`diagnostics` の要素は `{ "code": "E-SCAN-002", "severity": "error", "message": "...", "location": ... }` である。

### DS-943

検証結果を返す `verify` / `report`（CLI の `--format json` と同名 MCP ツール）は、これに加えて最上位に `scope` を持つ（下記要求 scope の最上位表現）。

### DS-944

グローバルオプションは `--project <dir>`（プロジェクトルート。既定はカレントから `.verify/` を上方探索）を持つ。

### DS-945

グローバルオプションは `--format <text|json>` を持つ。

### DS-946

グローバルオプションは `--quiet` を持つ。

### DS-947

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

*引用: 本冊 §11.3, 基本仕様 §4.6*

### DS-948

text 出力の冒頭表示（§12.2）と同じ内容を機械可読に表したものである。

### DS-949

`scope.requested.items` は、この実行で評価した検査軸を本冊 §11.1 の検査名で列挙する。

*引用: 本冊 §11.1*

### DS-950

`--items`（MCP は `items[]`）省略時は固定4検査を 4 件すべて列挙し、空 list にしない。

### DS-951

列挙順は上記例の固定順（`chain_integrity` / `orphan_detection` / `target_binding` / `oracle_presence`）とする。

### DS-952

`scope.requested.entities` は、エンティティ軸で指定した対象を `{ "kind": "doc" | "vo" | "test", "id": ... }` の list として返す。

### DS-953

`--doc` / `--vo` / `--test` をいずれも指定しない実行では `scope.requested.entities` は空 list とし、暗黙の根エンティティで埋めない。

### DS-954

`scope.unverified_outside_scope` は、`requested.items` が 4 件未満、または `requested.entities` が空でない場合に `true`、それ以外（固定4検査 × エンティティ軸無指定）は `false` とする。

### DS-955

`true` は「要求 scope 外は未検証であり、`PASS` ではない」ことを表す。

### DS-956

scope 外・未実施の検査は集約ツリー内で `NO_EVIDENCE`（診断 `NOT_CHECKED`）として保持する。

*引用: 本冊 §11.3*

### DS-957

`verify` / `report` は `unverified_outside_scope: false` の完全検証でも `scope` を省略しない。

### DS-958

`scope` を持たない出力は限定 scope と区別できないため、完全検証の根拠として扱わない。

### DS-959

`init` / `scan` / `doc *` / `vo *` / `test *` / `audit *` / `run` など検証結果を返さないコマンドは `scope` を持たない。

### DS-960

検証結果を出力するすべてのコマンド（`verify` / `report` / `scan` の集約表示等）は、検証状態と診断ラベルを常に別軸の2列として提示する。

*導出元: SPEC-046, SPEC-047, SPEC-048*

*引用: 本冊 §5.2, 基本仕様 §4.1・§4.2*

### DS-961

JSON では検証状態を各検査ノードの `state` field へ入れる。

### DS-962

JSON では診断ラベルを各検査ノードの `diagnostic` field（0件以上）へ入れ、`state` の値には決して用いない。

### DS-963

診断ラベルを集約の代表値選択に用いず、原因説明として併記するだけとする。

*導出元: SPEC-190, SPEC-191*

*引用: 本冊 §11.3, 基本仕様 §22.2*

### DS-964

JSON envelope、adapter選択エラー、capability不足の非PASS扱いはMCPと共通である。

### DS-965

CLIだけがRust固有の既定値へフォールバックしてはならない。

### DS-966

Testを含むJSONは本冊 §5.2の `execution` を必ず返す。

*引用: 本冊 §5.2*

### DS-967

`rust-cargo` Testについてだけ、wire compatibility layerが `filter`、`package`、`test_target` を追加できる。

### DS-968

非Rust TestではRust互換fieldを省略し、空値またはdummy値を返さない。

### DS-969

Test JSONは `targets` を常に list として返す。

*引用: 本冊 §5.2*

### DS-970

`rust-cargo` は `targets ≥ 1` を必須とする。

*引用: 本冊 §4.1・§4.4*

### DS-971

targetが1件の場合だけ同値の単数互換field `target` を追加できる。

### DS-972

複数target Testでは単数fieldを省略し、先頭targetを代表値として返さない。

### DS-973

Test入力から `execution` を復元できるのは、`rust-cargo` codecに完全で相互整合するRust互換実行座標が与えられた場合だけである。

### DS-974

`execution`と互換fieldが併存する場合は一致を必須とする。

### DS-975

本 version の Test metadata は存在理由分類（旧 `role` / `anchor` / `anchor_rationale`）を持たない。

*導出元: REQ-052, REQ-053, REQ-054, REQ-055, REQ-056, REQ-057, REQ-058, SPEC-123, SPEC-124, SPEC-369, SPEC-370, SPEC-371, SPEC-372, SPEC-373, SPEC-374, SPEC-375, SPEC-376, SPEC-377, SPEC-378, SPEC-379, SPEC-380, SPEC-381, SPEC-382, SPEC-383, SPEC-384, SPEC-385, SPEC-386, SPEC-387, SPEC-388, SPEC-389, SPEC-390, SPEC-391, SPEC-392, SPEC-393, SPEC-394, SPEC-395, SPEC-396, SPEC-397, SPEC-398, SPEC-399, SPEC-400, SPEC-401, SPEC-402, SPEC-403, SPEC-404, SPEC-405, SPEC-406, SPEC-407*

*引用: 本冊 §4.1, 基本仕様 §12, 要件定義 §4.1*

### DS-976

本 version はすべての管理対象 Test に `covers ≥ 1` を一律に要求する。

*導出元: REQ-052, REQ-053, REQ-054, REQ-055, REQ-056, REQ-057, REQ-058, SPEC-123, SPEC-124, SPEC-369, SPEC-370, SPEC-371, SPEC-372, SPEC-373, SPEC-374, SPEC-375, SPEC-376, SPEC-377, SPEC-378, SPEC-379, SPEC-380, SPEC-381, SPEC-382, SPEC-383, SPEC-384, SPEC-385, SPEC-386, SPEC-387, SPEC-388, SPEC-389, SPEC-390, SPEC-391, SPEC-392, SPEC-393, SPEC-394, SPEC-395, SPEC-396, SPEC-397, SPEC-398, SPEC-399, SPEC-400, SPEC-401, SPEC-402, SPEC-403, SPEC-404, SPEC-405, SPEC-406, SPEC-407*

*引用: 本冊 §4.1, 基本仕様 §12, 要件定義 §4.1*

### DS-977

したがって CLI・MCP の入出力に role / anchor の宣言逐語 field・実効 field・既定値埋めは存在しない。

### DS-978

VO への寄与は `covers` 宣言と証拠の十分性判定だけから導出する。

### DS-979

明示操作に必須のadapter capabilityが未提供なら、`ok: false`、E-ADAPTER-004、終了コード2を返す。

### DS-980

明示操作に必須のadapter capabilityが未提供なら、create / editではファイルを変更しない。

### DS-981

明示操作に必須のadapter capabilityが未提供なら、auditでは判断記録を生成しない。

### DS-982

明示操作に必須のadapter capabilityが未提供なら、runではEvidenceを生成しない。

### DS-983

検証・reportで能力不足を観測した場合はW-ADAPTER-101と能力別の非PASS値（static / coverage 欠落は `NO_EVIDENCE`／診断 `NOT_CHECKED`、runner 欠落は `NO_EVIDENCE`／診断 `NOT_EXECUTED`、解析限界は `UNKNOWN`）を返す。

*導出元: SPEC-192, SPEC-193, SPEC-194, SPEC-195, SPEC-196*

*引用: 本冊 §5.2 末尾, 基本仕様 §22.3*

### DS-S131 12.2 `vtest init`

*導出元: SPEC-S008, SPEC-S009, SPEC-S017, SPEC-S021, SPEC-S030, SPEC-S031, SPEC-S034, SPEC-S035, SPEC-S038, SPEC-S039, SPEC-S042, SPEC-S043, SPEC-S049, SPEC-S051, SPEC-S053, SPEC-S055, SPEC-S061, SPEC-S062, SPEC-S066, SPEC-S069, SPEC-S071, SPEC-S072, SPEC-S073, SPEC-S074, SPEC-S075, SPEC-S076, SPEC-S077, SPEC-S079, SPEC-S081, SPEC-S084, SPEC-S085, SPEC-S087, SPEC-S088, SPEC-S090, SPEC-S092, SPEC-S094, SPEC-S096, SPEC-S097, SPEC-S098, SPEC-S099, SPEC-S100, SPEC-S103*

### DS-984

`config.yaml` は本冊 §2.2 の version 2 で、組込 `rust-cargo` adapter namespace を含む。

*引用: 本冊 §2.2*

### DS-985

`vtest init` の生成物には `doc/` / `vo/` / `rel/` / `forms/` / `decisions/` / `approvals/` / `evidence/` / `cache/` と `.verify/.gitignore`、組込 Form Schema（§14）を含む。

### DS-986

既存の `.verify/` があれば `vtest init` はエラー（終了コード 2）とする。

### DS-987

`vtest init` は `.verify/` を作成するだけであり、既存コードを変更しない。

*導出元: R-5, SPEC-158, SPEC-159, SPEC-160, SPEC-161, SPEC-162, SPEC-163, SPEC-414*

*引用: 基本仕様 §18.1, 要件定義 R-5*

### DS-988

`vtest init` が作成するファイル・ディレクトリは `.verify/` とその配下に限る。

### DS-989

`vtest init` は、プロジェクトルート直下の `.gitignore`・ビルド設定（`Cargo.toml` 等）・CI 設定を含め、`.verify/` の外にあるいかなるファイルも新規作成・変更・削除しない。

### DS-990

`vtest init` は既存ソースコード・既存テストコードのバイト列を変更しない。

### DS-991

`vtest init` は Test metadata 宣言（`@vtest.` 行）・annotation・doc comment を既存ソースへ挿入しない。

### DS-992

管理宣言の付与は `test create` / `test edit`（§15）と利用者自身の編集だけが行い、`vtest init` は行わない。

### DS-993

`vtest init` は既存の `.verify/` があるときは終了コード 2 で中止し、その実行でファイル・ディレクトリを 1 件も作成・変更・削除しない。

### DS-994

`vtest init` は既存 `.verify/` の内容を上書き・マージ・移動しない。

### DS-995

したがって `vtest init` の実行前後で、`.verify/` を除いた作業ツリーの内容は同一である。

### DS-996

既存プロジェクトへの後からの導入が既存資産を書き換えないことは、この不変条件で保証する。

### DS-997

整合性検査は `chain_integrity`（文書鎖・VO derives_from・Test 管理宣言）と `orphan_detection`（文書層孤児）を構成する。

*引用: 本冊 §5.6, 基本仕様 §23*

### DS-998

`vtest scan` は registry・config・adapter契約の検証または adapter 呼出しが E-ADAPTER-* / E-CONFIG-* で拒否された場合は終了コード2とし、scan結果を生成しない。

### DS-999

`vtest scan` は scan が完了し repository 整合性の E-SCAN-* 診断がある場合は終了コード1、error 診断がなければ0とする。

*引用: 本冊 §17.2*

### DS-1000

`doc` は文書種別（要件定義・基本仕様・詳細設計・API Schema 等）を区別しない。

### DS-1001

段（要件→仕様→詳細設計…）は `derives_from` リンクで表現し、種別を増やさない。

### DS-1002

旧モデルの `vtest spec` / `vtest req` は廃し、SPEC / REQ 実体層は持たない。

### DS-1003

`--derives-from` は上流 document への導出リンク（0件可＝根候補）である。

### DS-1004

各 `--derives-from` リンクに任意の `--note`（導出理由・空可・非 `MISMATCH`）を付けられる。

*導出元: SPEC-281, SPEC-282, SPEC-283*

*引用: 基本仕様 §3.4*

### DS-1005

`--anchor <text>` は直前の `--derives-from` に束縛し、参照先 document 内の該当箇所（節番号・条項番号・見出し等）を記録する。

*引用: 本冊 §3.1*

### DS-1006

`--anchor` は `--note` と同じ結合規則・同じ任意性であり、省略・空文字列は `chain_integrity` 違反にならない。

### DS-1007

`--anchor` の値は不透明な文字列として保存し、文書内位置への解決・実在確認・書式検証を行わない。

### DS-1008

`--derives-from` を伴わない `--anchor`、または 1 つの `--derives-from` に対する 2 個目以降の `--anchor` は引数不正として終了コード 2 で拒否し、レコードを書かない。

### DS-1009

`doc show` は各 `derives_from` entry の `anchor` を表示する。

### DS-1010

`--root` / `--no-root` は当該 DOC を `orphan_detection` の除外根（`config.yaml` の `doc.roots`）へ追加／除外する。

*引用: 本冊 §2.2・§5.6*

### DS-1011

根指定の追加・削除はこのフラグで管理し `doc.roots` へ反映する。

### DS-1012

`--update` は既存 DOC レコードの sha256 を現ファイルで再計算して更新する。

### DS-1013

`--update` は document subject hash が変化するため、当該 document を依存 closure に含む判断記録・承認が失効する旨を出力する。

*引用: 本冊 §3.5・§8.5・§11.4*

### DS-1014

`--update` は `--root` / `--no-root` を併せて根指定も更新できる。

### DS-1015

`doc list --tree` は `derives_from` の文書鎖を木として表示する。

### DS-1016

`doc list --roots` は現在の根集合を表示する。

### DS-1017

`doc show` は DOC の path・content_hash・derives_from・根指定・鮮度（content_hash と実ファイルの一致）・実効承認状態を表示する。

*引用: 本冊 §3.5*

### DS-1018

`derives_from` の参照先 document が存在しなければ文書鎖のリンク切れとして `chain_integrity = MISMATCH`（E-SCAN-012）とする。

### DS-1019

`path` の実ファイルが `content_hash` と一致しなくなれば `chain_integrity = MISMATCH`（診断 `STALE`）とする。

*引用: 本冊 §11.4*

### DS-1020

根に指定されず親も持たない document は孤児として `orphan_detection = MISMATCH`（E-SCAN-016）とする。

*引用: 本冊 §5.6*

### DS-1021

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

*導出元: SPEC-043, SPEC-044, SPEC-279, SPEC-280*

*引用: 本冊 §3.2, 基本仕様 §3.2*

### DS-1022

旧モデルの `--req`（REQ 参照）・`--spec` / `--section`（SPEC + 節参照）は廃し、上流参照は `--derives-from DOC-*`（任意の `--note`）へ一本化する。

### DS-1023

VO の `status`（`draft` / `approved`）は正典 field ではなく承認レコードから導出する表示値である。

*引用: 本冊 §3.2・§3.5*

### DS-1024

`status` が読取り互換 field として保存されていても値は無視し、存在自体は W-STORE-001 とする。

### DS-1025

旧 REQ の `active` / `withdrawn` 語彙は REQ 層とともに廃止する。

### DS-1026

`--doc DOC-X` は当該 document を根とする下流 VO の絞り込みである。

### DS-1027

`vo add` / `vo edit` の `--anchor <text>` は直前の `--derives-from` に束縛し、参照先 document 内の該当箇所（節番号・条項番号・見出し等）を記録する。

*引用: 本冊 §3.2*

### DS-1028

`vo add` / `vo edit` の `--anchor` は `--note` と同じ結合規則・同じ任意性であり、省略・空文字列は `chain_integrity` 違反にならず、値は不透明な文字列として保存する。

### DS-1029

`--derives-from` を伴わない `--anchor`、または 1 つの `--derives-from` に対する 2 個目以降の `--anchor` は引数不正として終了コード 2 で拒否し、レコードを書かない。

### DS-1030

`vo show` は各 `derives_from` entry の `anchor` を表示する。

### DS-1031

`anchor` は VO subject hash に入らないため、`anchor` だけを変更した `edit` は承認を失効させない。

*引用: 本冊 §3.2*

### DS-1032

`--combination` は `coverage_policy: explicit` のときに実体化する組合せ（`combinations`）を入力する。

*引用: 本冊 §3.2.1*

### DS-1033

`--combination` の1回の出現が1 tupleに対応し、`<dim>=<part>` をカンマ区切りで並べて全軸の値を与える（例：`--combination operand-sign=positive,operator=div`）。

### DS-1034

複数 tuple は `--combination` を繰り返して与える。

### DS-1035

`vo edit` の `--combination` は desired state であり、1 回以上与えたときは既存 `combinations` を与えた集合で置換する（追記しない）。

### DS-1036

`--clear-combinations` は `combinations` を空にする。

### DS-1037

`--combination` も `--clear-combinations` も与えない `vo edit` は既存 `combinations` を保持する。

### DS-1038

`--combination` の値が本冊 §3.2.1 の受理条件（`explicit` 以外での指定、未宣言 dimension、未列挙 partition、宣言 dimension の欠落・重複、重複 tuple、`explicit` かつ tuple 0 件）に違反する場合は E-SCAN-017、終了コード 2 で拒否し、レコードを書かない。

*引用: 本冊 §3.2.1*

### DS-1039

`vo add` はこの違反時に新規レコードを作成せず、`vo edit` は既存レコードを変更しない。

### DS-1040

`<dim>=<part>` の形をなさない値は引数不正として終了コード 2 で拒否する。

### DS-1041

`vo expand` は本冊 §3.2.1 の実体化（`independent-axes` / `full-product` / `explicit`）である。

*引用: 本冊 §3.2.1*

### DS-1042

`--dry-run` は生成予定の子 VO 一覧のみ表示する。

### DS-1043

`explicit` の VO は `combinations` の各 tuple につき 1 件の子 VO を、`dimensions` の宣言順に連結した suffix（`VO-X-<P1>-<P2>`）で生成する。

### DS-1044

`combinations` が本冊 §3.2.1 の受理条件に違反する VO に対しては E-SCAN-017、終了コード 2 とし、子 VO を 1 件も生成しない（部分生成しない）。

*引用: 本冊 §3.2.1*

### DS-1045

`vo approve VO-X <承認引数>` は `vtest approval create --subject-type vo --subject-id VO-X <承認引数>` の別名であり、引数・拒否条件・生成されるレコードは同一である。

### DS-1046

承認の意味論を重複して定義せず、正典は次項の `vtest approval` と本冊 §3.5 とする。

*引用: 本冊 §3.5*

### DS-1047

`vo list --status` および `vo show` が表示する承認状態は、本冊 §3.5 の実効承認導出（`approved_state` を参照し、実効集合に `rejected` / `withdrawn` が1件でも残れば `draft`）の結果であり、承認レコードの件数・新旧からは導出しない。

*引用: 本冊 §3.5*

### DS-1048

`vo edit` は実効承認が `approved` の VO に対して警告を出す（編集自体は許可し、承認はハッシュ不一致で自動失効する）。

### DS-1049

`--subject-type` と `--subject-id` は本冊 §3.5 の承認対象の値域に対応する。

*引用: 本冊 §3.5*

### DS-1050

`--subject-type vo` は `subject` に VO ID を書き込む。

### DS-1051

`--subject-type document` は `subject` に document ID を書き込む。

### DS-1052

`--subject-type judgment` は `--subject-id` に判断記録 ULID を取り、`judgment_ref` へ書き込んだうえで `subject` に当該判断記録の `subject` を、`subject_hash` / `dependencies` にその対象の現在値を記録する。

### DS-1053

方針は総称 document として登録した文書で表現するため、方針の承認・却下・取消は `--subject-type document` で記録する。

*引用: 本冊 §3.1・§3.5*

### DS-1054

`--state` は必須で、本冊 §3.5 の `approved_state`（`approved` / `rejected` / `withdrawn`）を与える。

*引用: 本冊 §3.5*

### DS-1055

`--basis` は根拠参照（任意）である。

### DS-1056

`--supersedes` は明示に置き換える旧承認レコード ID（0件以上）である。

### DS-1057

`show` は当該対象の承認レコード一覧（`approved_state`・`supersedes`・有効性）と、本冊 §3.5 の実効承認状態（`draft` / `approved`）を返す。

*引用: 本冊 §3.5*

### DS-1058

対象、`--subject-type judgment` の参照先判断記録、またはいずれかの依存 entity / document source を完全・current に解決できない場合は E-APPROVAL-001、終了コード 2 として record を追加しない。

### DS-1059

`--state` が値域外、`--subject-type` と `--subject-id` の種別が一致しない、`--supersedes` の参照先が存在しない・対象が一致しない・自己参照のいずれかであれば E-APPROVAL-002、終了コード 2 として record を追加しない。

### DS-1060

実効承認は明示の `supersedes` 関係だけで決まる。

### DS-1061

`supersedes` 関係にない複数の有効承認レコードはすべて実効集合に属し、`approved_at` / ULID の順序でどれかを「現在の承認」に選ぶことはしない。

### DS-1062

実効集合に `rejected` / `withdrawn` が 1 件でも残れば `draft` とする。

### DS-1587

取消・却下の後に再承認するには、当該レコード ID を `--supersedes` で名指しした `--state approved` を追加する。

*引用: 本冊 §3.5*

### DS-1064

承認は検証状態と独立の別軸であり、承認済みを理由に非 `PASS` を `PASS` へ昇格させない。

*導出元: SPEC-051, SPEC-052, SPEC-150, SPEC-151, SPEC-152, SPEC-153, SPEC-154, SPEC-155, SPEC-354, SPEC-355, SPEC-356, SPEC-357*

*引用: 本冊 §3.5, 基本仕様 §4.5・§17*

### DS-1065

`--dry-run` は挿入内容と挿入位置のみを表示する。

### DS-1066

回答の検証エラーは E-OP-001 として候補付きで報告する。

*引用: 本冊 §6.3*

### DS-1067

`--answers` は完全なあるべき状態を宣言する。

### DS-1068

`--set` は指定フィールドのみのあるべき値を宣言する。

### DS-1069

Test implementation の書き換えは `--body-file <path>` で adapter へ全文を与える。

> 編集の実装は §15 に定める。

### DS-1070

`test show` は Test の intent・covers・targets（宣言 target 集合）・Source Location・判断記録（§8）・Evidence（§9）の状態を表示する。

> ```text
> vtest test show TEST-X        # intent、covers、targets、位置、判断記録・Evidence 状態
> vtest test list [--vo VO-X] [--unregistered]
> vtest test query --source rust-cargo::src/parser.rs::Parser::parse   # SRC からの逆引き
> ```

### DS-1071

`test show` は role / anchor の表示・`--role` フィルタを持たない。

*引用: 本冊 §4.1*

### DS-1072

`test query` の逆引きは §11.6 の役割別 projection の基盤（VO → Tests、SRC → Tests）としても用いる。

*引用: 本冊 §5.3*

### DS-1073

静的解析は正典レコードを持たない再計算派生であり、`audit static` は正典の監査レコードを生成しない。

*導出元: P-003*

*引用: 本冊 §7.1, 基本仕様 P-003*

### DS-1074

`audit static` は `oracle_presence` へ供給する DA-001 / DA-003 / DA-004 / DA-005 / DA-006 と、`target_binding` の静的到達（DA-002）を評価する。

### DS-1075

target-scoped な DA-002 / DA-003 は宣言 target ごとの verdict を規則単位 verdict と併せて提示する。

*引用: 本冊 §3.6・§7.2*

### DS-1076

`audit static` は判断記録（§8）とは別機構であり、外部判断の記録には転用しない。

*引用: 本冊 §7*

### DS-1077

`cases` は `@vtest.case` 宣言の正規化文字列を宣言順に並べた list であり、宣言が無い Test でも空 list を明示して項目を省略しない。

### DS-1078

バンドルは派生情報であり Git 管理しない。

### DS-1079

`--kind` は判断させる UNKNOWN のエスカレーション質問のラベル（本冊 §8.1 の判断型）であって検査項目ではない。

*引用: 本冊 §8.1*

### DS-1080

`--kind` の値と `subject` 値域は本冊 §8.1 の表に従う。

*引用: 本冊 §8.1*

### DS-1081

`test-semantic` は「テストコードは VO の claim と Test Intent が宣言する振る舞いを実際に検証しているか」を意味し、`--test` のみに使う。

*引用: 本冊 §8.6*

### DS-1082

`impl-consistency` は「対象実装が宣言と一致するか」を意味し、`--test` のみに使い、上流 document を要するため §3.5 と同じ上流依存規則で document 完全集合を同梱する。

### DS-1083

`case-coverage` は「cases 集合が VO の要求入力空間を十分に代表・網羅しているか」を意味し、`--test` / `--vo` の双方に使う。

*導出元: SPEC-129, SPEC-130, SPEC-131, SPEC-132*

*引用: 本冊 §8.1, 基本仕様 §14*

### DS-1084

`--test` で `--kind` を省略した場合は `test-semantic` とする。

### DS-1085

`--vo` では `--kind case-coverage` を必須とし、`--kind` 省略および `--vo` と `test-semantic` / `impl-consistency` の組合せは usage error（終了コード 2）としてバンドルを生成しない。

### DS-1086

バンドルは選ばれた判断型を `judgment_kind` として出力し、`audit submit` はこれを判断記録へ複製する。

### DS-1087

`case-coverage` は §11 の判断対象であって基本仕様 §5 の 4 検査ではない。

*導出元: SPEC-053, SPEC-054, SPEC-055, SPEC-056, SPEC-057, SPEC-058, SPEC-059, SPEC-060, SPEC-061, SPEC-062, SPEC-063, SPEC-064, SPEC-065, SPEC-066, SPEC-067, SPEC-068, SPEC-069, SPEC-287, SPEC-288, SPEC-289, SPEC-290, SPEC-291, SPEC-292, SPEC-293*

*引用: 基本仕様 §5*

### DS-1088

`case-coverage` の未判断・判断結果はいずれの検査の値にも写像せず集約へ寄与しない。

*引用: 本冊 §8.1・§11.3*

### DS-1089

`audit submit` は本冊 §8.4 の検証（bundle_id 存在＝E-AUDIT-001、subject 一致＝E-AUDIT-003、judgment_kind 一致・値域＝E-AUDIT-003、記録時ハッシュと現在ハッシュの一致＝E-AUDIT-002、decision が受理値＝E-AUDIT-004、supersedes の参照先が同一 subject・同一 judgment_kind の既存判断記録で自己参照でない＝E-AUDIT-008）を行い、受理時に判断記録 ID（`.verify/decisions/` の ULID）を出力する。

*引用: 本冊 §8.4*

### DS-1090

判断は少なくとも actor / subject / decision / judgment_kind を含み、理由・根拠（`reason` / `exclusions`）と `supersedes` は任意である。

### DS-1091

理由が空であることだけを根拠に判断を無効化しない。

*導出元: REQ-180, REQ-181, REQ-182, REQ-183, REQ-184, REQ-185, REQ-186, REQ-187, REQ-188, REQ-189, REQ-190, REQ-191, REQ-192, SPEC-116, SPEC-117, SPEC-118, SPEC-119, SPEC-120, SPEC-121, SPEC-122, SPEC-333, SPEC-334, SPEC-335*

*引用: 本冊 §8.3, 基本仕様 §11.3, 要件定義 §12*

### DS-1092

`decision` の受理値は `accepted` / `rejected` / `deferred` 等である。

*引用: 本冊 §8.3*

### DS-1093

競合の解消は `supersedes` だけによる。

### DS-1094

同一 `(subject, judgment_kind)` に判断値の食い違う有効判断記録が併存する場合、実効判断は未確定（`UNKNOWN`）とし、W-STORE-004 を出す。

### DS-1095

機械は新旧・decision 値・件数のいずれによっても採用記録を選ばない。

### DS-1096

新しい判断記録が旧記録の ULID を `supersedes` で名指しした場合にだけ解消する。

*引用: 本冊 §8.5*

### DS-1097

未確定の事実は `verify` / `report` の判断待ち section（§12.4）へ載せる。

### DS-1098

判断記録の受理は当該対象の検証状態（5状態）を昇格させない。

*導出元: SPEC-116, SPEC-117, SPEC-118, SPEC-119, SPEC-120, SPEC-121, SPEC-122, SPEC-333, SPEC-334, SPEC-335*

*引用: 本冊 §8.3・§3.4, 基本仕様 §11.3*

### DS-1099

旧モデルの `verdict → CheckValue` 写像・reasons / basis 必須検査（E-AUDIT-005〜007）は撤去する。

*引用: 本冊 §8.4*

### DS-1100

判断記録・承認記録のいずれも検証状態を昇格・降格させない。

### DS-1101

旧モデルの `--req`（REQ 指定）は document 層の総称化により廃止し、document scope が必要な場合は VO 部分木経由で指定する。

*引用: 本冊 §9.1*

### DS-1102

`--fast` は cargo test のみで、`target_coverage` を `checked: false` として記録する。

### DS-1103

`--fast` は `target_binding` の動的証拠を採らず、検証時 `NO_EVIDENCE`／診断 `NOT_CHECKED` とする。

*引用: 本冊 §10.3*

### DS-1104

scope は2軸であり、`--items` が検査軸（4検査の部分集合）、`--doc` / `--vo` / `--test` がエンティティ軸（部分木）である。

*引用: 基本仕様 §4.6, 本冊 §11.3*

### DS-1105

旧モデルの `--spec` / `--req` は廃止し、`--req` は除去する。

### DS-1106

`--items` 省略時は常に固定4検査による完全検証を行う。

### DS-1107

`config.yaml` の `verify.full_scope` は本冊 §2.2 の invariant として事前に検証・正規化し、項目選択 knob として使用しない。

*引用: 本冊 §2.2*

### DS-1108

旧12項目の列挙は version を問わず E-CONFIG-001 とし、version 1 の field 欠落だけを固定4検査へ具体化する。

### DS-1109

`verify.full_scope` の in-memory の項目補完は行わない。

*引用: 本冊 §2.2*

### DS-1110

`--items` に4検査未満の明示的な集合を指定した場合だけ限定 scope とし、scope 外・未実施の検査は `NO_EVIDENCE`（診断 `NOT_CHECKED`）として保持し、`PASS` へ変換しない。

### DS-1111

限定 scope の結果を完全検証 OK と表示しない。

### DS-1112

いかなる設定値も完全検証を4本未満へ縮退させない。

*導出元: SPEC-189*

*引用: 基本仕様 §4.6・§22.1*

### DS-1113

scope を限定した場合、出力冒頭に要求 scope と「scope 外は未検証」の旨を必ず表示する。

### DS-1114

`--format json` では同じ内容を最上位 field `scope`（§12.1）として返し、完全検証の場合も省略しない。

### DS-1115

`--gate <name>` はフェーズゲート評価（§12.3）である。

### DS-1116

config の `gates` に同名の定義が無ければ E-CONFIG-002・終了コード 2 で拒否し、検証を実行しない。

*引用: 本冊 §11.5・§17.1*

### DS-1117

`--summary` は総合 `OK` / `NG` と非 `PASS` 件数のみを出力する。

### DS-1118

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

### DS-1119

診断ラベルは代表値の順位（基本仕様 §22.2 の `FAIL > MISMATCH > NO_EVIDENCE > UNKNOWN`）に用いず、原因説明として併記する。

*導出元: SPEC-190, SPEC-191*

*引用: 基本仕様 §22.2*

### DS-1120

`target_binding` の非 `PASS` は根拠として用いた Evidence ID と当該 target の `target_coverage` 結果を引用する。

*引用: 本冊 §11.3*

### DS-1121

`oracle_presence` の非 `PASS` は違反した DA rule と根拠 span を引用する。

*引用: 本冊 §11.3*

### DS-1122

判断記録（§8）を引用する場合は decision ID を示す。

### DS-1123

静的解析は正典レコードを持たないため監査レコード ID は引用しない。

*引用: 本冊 §7.1*

### DS-1124

`chain_integrity` は repository-level の構造検査であり、`Structural checks` 配下に表示する。

### DS-1125

`chain_integrity` は発見された各 Test の管理宣言解決と Test ID 大局的一意性をすべて評価し、未登録または不正対応の各 Test について adapter ID・source location・diagnostic code・判定値を `chain_integrity` 配下に列挙する（`MISMATCH`／診断 `MISSING`）。

*引用: 本冊 §11.1.1*

### DS-1126

covers する Test の無い leaf VO も `chain_integrity = MISMATCH`（診断 `MISSING`）として entity tree 上に示す。

### DS-1127

JSON でも同じ根拠一覧を返す。

### DS-1128

Evidence が複数 target の計測結果を持つ場合、text report は Test 単位の集約値に加えて各 target の canonical Locator・result・count を子要素として表示する。

*引用: 本冊 §6.1.1*

### DS-1129

JSON は target 別 list を欠落なく返す。

*引用: 本冊 §3.6*

### DS-1130

各行の prefix は、その行の祖先に後続兄弟があれば `│  `、なければ空白3文字を階層ごとに連結し、現在 node が途中の兄弟なら `├─ `、最後の兄弟なら `└─ ` を付けて構成する。

### DS-1131

最上位 node にも同じ途中・末尾 branch 規則を適用する。

### DS-1132

祖先 node 自身の `├─ ` / `└─ ` を子孫行へ引き継がない。

### DS-1133

`--from <node>` は任意ノード（DOC / VO / TEST / SRC）からの局所トレースの起点である。

*導出元: SPEC-172, SPEC-173, SPEC-174, SPEC-175, SPEC-176, SPEC-177, SPEC-178, SPEC-179, SPEC-180, SPEC-181, SPEC-182, SPEC-358, SPEC-359, SPEC-360, SPEC-361, SPEC-362, SPEC-363, SPEC-364*

*引用: 本冊 §11.6, 基本仕様 §19*

### DS-1134

`--direction` は上流／下流／双方である。

### DS-1135

`--depth` は連続追跡の段数である。

### DS-1136

`--view` は役割 preset（`pm`＝上位 document・VO の状態と未確定/NG、`tester`＝VO・Test・検証対象・Evidence・未実施/失敗理由、`coder`＝実装から関連 Test・VO・上流 document へのトレース）である。

### DS-1137

役割を固定 enum として本冊は仕様化せず、preset・view 体系はここに委譲される。

*導出元: SPEC-230, SPEC-231, SPEC-232, SPEC-233, SPEC-234, SPEC-235, SPEC-236, SPEC-237, SPEC-238, SPEC-239, SPEC-240, SPEC-241, SPEC-242, SPEC-243, SPEC-244, SPEC-245, SPEC-246, SPEC-247, SPEC-248, SPEC-249, SPEC-250, SPEC-251, SPEC-252, SPEC-253, SPEC-254, SPEC-255*

*引用: 本冊 §11.6, 基本仕様 §30 item21*

### DS-1138

機能単位の集約は親 VO（子 VO を持つ VO）を単位とする。

*導出元: SPEC-190, SPEC-191*

*引用: 本冊 §11.3・§11.6, 基本仕様 §22.2*

### DS-1139

`--vo <親VO>` または `--from <親VO> --direction down` は、当該親 VO の代表値（fail-closed 合成）と、その配下の子 VO ごと・Test ごとの内訳を同じツリーに返す。

### DS-1140

Feature を別エンティティとして出力せず、Feature 名・Feature ID の field を設けない。

### DS-1141

束ねの識別子は親 VO の ID とする。

### DS-1142

`--format json` の trace 出力に含まれる `derives_from` エッジ（DOC → DOC、DOC → VO）は、`anchor` と `note` を同伴する。

*導出元: SPEC-109, SPEC-110, SPEC-111, SPEC-112, SPEC-113, SPEC-319, SPEC-320, SPEC-321, SPEC-322, SPEC-323, SPEC-324, SPEC-325, SPEC-326, SPEC-327, SPEC-328, SPEC-329, SPEC-330, SPEC-331, SPEC-332*

*引用: 本冊 §11.6・§3.1・§3.2, 基本仕様 §11.1*

### DS-1143

エッジ要素は `{ "from": "DOC-REQ-001", "relation": "derives_from", "anchor": "§12.3", "note": "", "to": "VO-PARSER-UTF8-003" }` の形とする。

### DS-1144

`anchor` を持たない entry では `anchor` を省略または `null` とし空文字列で埋めない。

### DS-1145

`report --from DOC-REQ-001 --direction down --format json` は、この形式で「どの上流条項がどの概念（VO）へ対応するか」の対応ペア集合を返す。

### DS-1146

`anchor` は不透明な文字列として transport し、文書内位置への解決・整合検査を行わない。

### DS-1147

`--format json` の出力へ、未確定・要判断事項を横断的に集約した `pending` section を含める（§12.4）。

*導出元: SPEC-167, SPEC-168, SPEC-419, SPEC-420, SPEC-421, SPEC-422, SPEC-423, SPEC-424, SPEC-425, SPEC-426, SPEC-427, SPEC-428, SPEC-429, SPEC-430, SPEC-431, SPEC-432, SPEC-433, SPEC-434, SPEC-435, SPEC-436, SPEC-437, SPEC-438, SPEC-439, SPEC-440, SPEC-441, SPEC-442, SPEC-443, SPEC-444, SPEC-445, SPEC-446, SPEC-447, SPEC-448, SPEC-449, SPEC-450, SPEC-451, SPEC-452*

*引用: 本冊 §11.7, 基本仕様 §18.3*

### DS-1148

`--gate <name>` は `verify` と同じ解決規則に従い、未定義名は E-CONFIG-002・終了コード 2 で拒否する。

*引用: 本冊 §11.5*

### DS-S132 12.3 フェーズゲート評価（`verify --gate` / `report --gate`）

*導出元: REQ-S057, SPEC-S043, SPEC-S050, SPEC-S066, SPEC-S069, SPEC-S075, SPEC-S098*

### DS-1149

`config.yaml` の `gates` に、ゲート名と進行条件（`require.verification`＝要求する検証結果、`require.approvals`＝要求する承認ロール集合）を保持する。

*引用: 本冊 §2.2*

### DS-1150

`require.verification` は 5 状態語彙（`PASS` / `FAIL` / `MISMATCH` / `NO_EVIDENCE` / `UNKNOWN`）のいずれかとの完全一致でなければならず、違反は config 受理時に E-CONFIG-001（終了コード 2）とする。

### DS-1151

`require.approvals` の省略は空集合として受理する。

*引用: 本冊 §2.2*

### DS-1152

`--gate <name>`（MCP は `gate` 入力）は `gates[].name` との大文字小文字を区別した完全一致でだけ解決する。

### DS-1153

未定義名・`gates` が空の状態での指定は E-CONFIG-002、`ok: false`、終了コード 2 とし、検証もゲート評価も実行せず、`data` に部分結果を返さない。

### DS-1154

診断 message には指定名と定義済みゲート名の一覧を含め、MCP では §13.1 の `candidates` に定義済みゲート名を入れる。

### DS-1155

検証条件は `require.verification` と要求 scope の集約代表値との完全一致でのみ充足する。

*引用: 本冊 §11.5*

### DS-1156

5 状態に順序を設けず、「要求値以上」の解釈を採らない。

### DS-1157

したがって `require.verification: PASS` は代表値 `PASS` のときだけ、`require.verification: UNKNOWN` は代表値 `UNKNOWN` のときだけ充足する。

### DS-1158

`--items` で検査軸を限定した実行では scope 外検査が `NO_EVIDENCE`（診断 `NOT_CHECKED`）として代表値に参加するため、`require.verification: PASS` のゲートは限定 scope では充足しない。

### DS-1159

承認ロールの解決は本別紙が新設する最小規則である。

*導出元: SPEC-150, SPEC-151, SPEC-152, SPEC-153, SPEC-154, SPEC-155, SPEC-230, SPEC-231, SPEC-232, SPEC-233, SPEC-234, SPEC-235, SPEC-236, SPEC-237, SPEC-238, SPEC-239, SPEC-240, SPEC-241, SPEC-242, SPEC-243, SPEC-244, SPEC-245, SPEC-246, SPEC-247, SPEC-248, SPEC-249, SPEC-250, SPEC-251, SPEC-252, SPEC-253, SPEC-254, SPEC-255, SPEC-354, SPEC-355, SPEC-356, SPEC-357*

*引用: 基本仕様 §17・§30 item22*

### DS-1160

承認レコードは role field を持たないため、`config.yaml` に承認ロール → approver id 集合の対応を project 定義可能とする。

> ```yaml
> approval_roles:
>   reviewer: [reviewer-agent-01, alice]
>   owner:    [owner-human-01]
> ```

*引用: 本冊 §3.5*

### DS-1161

ロール `R` の承認が存在するとは、「本冊 §3.5 で有効な（subject_hash・依存 closure が現在一致する）対象の承認レコードのうち、`approver.id` が `approval_roles[R]` に属するものが1件以上存在する」ことをいう。

*引用: 本冊 §3.5*

### DS-1162

`gates.require.approvals` が参照するロールが `approval_roles` に無い場合は config invariant 違反として E-CONFIG-001 とする。

### DS-1163

ロール充足の判定対象は、当該 `verify` / `report` のエンティティ軸で指定した対象（`--doc` / `--vo` / `--test`。省略時は評価 scope の根エンティティ）に束縛された有効承認とする。

### DS-1164

scope 内に複数の対象がある場合は各対象について当該ロールの有効承認を要求する（fail-closed）。

### DS-1165

より細粒度の承認 authority・対象範囲はプロジェクト設定へ委譲する。

*導出元: SPEC-150, SPEC-151, SPEC-152, SPEC-153, SPEC-154, SPEC-155, SPEC-230, SPEC-231, SPEC-232, SPEC-233, SPEC-234, SPEC-235, SPEC-236, SPEC-237, SPEC-238, SPEC-239, SPEC-240, SPEC-241, SPEC-242, SPEC-243, SPEC-244, SPEC-245, SPEC-246, SPEC-247, SPEC-248, SPEC-249, SPEC-250, SPEC-251, SPEC-252, SPEC-253, SPEC-254, SPEC-255, SPEC-354, SPEC-355, SPEC-356, SPEC-357*

*引用: 基本仕様 §17・§30 item22*

### DS-1166

`vtest verify --gate <name>` は、指定ゲートの対象 scope について検証を実行し、(1) 検証結果が `require.verification`（例 `PASS`）を満たすか、(2) `require.approvals` の各ロールについて上記解決規則で有効な承認が存在するか、を評価して満否と根拠（不足している非 `PASS` 検査・未充足の承認ロール）を提示する。

### DS-1167

`report --gate` は同評価を JSON の `gate` section で返す。

### DS-1168

検証状態と承認は独立の軸であり、承認未充足は検証状態を降格させない。

*導出元: SPEC-051, SPEC-052*

*引用: 本冊 §3.5, 基本仕様 §4.5*

### DS-1169

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

### DS-1170

`verification.required` は `require.verification` の値、`verification.actual` は要求 scope の集約代表値（5 状態のいずれか）、`verification.satisfied` は両者の完全一致である。

### DS-1171

`approvals[]` は `require.approvals` の各ロールについて充足有無と未充足の対象を返し、`require.approvals` が空集合なら空 list とする。

### DS-1172

`gate.satisfied` は `verification.satisfied` と全 `approvals[].satisfied` の論理積とする。

### DS-1173

text 出力では同じ 3 項目（要求値・現在の代表値・満否）と未充足ロール・不足している非 `PASS` 検査を提示する。

### DS-1174

`--gate` を指定した実行では最上位 `ok` と終了コードをゲート充足で決める（充足 → `ok: true`・0、不充足 → `ok: false`・1、未定義ゲート名 → `ok: false`・2）。

### DS-1175

要求 scope の総合 OK / NG は集約ツリーと `gate.verification.actual` から読み取る。

*引用: 本冊 §17.2*

### DS-1176

ゲート充足は検証状態とは別軸の評価であり、検証状態を書き換えない。

### DS-1177

JSON では検証状態（集約ツリーと `gate.verification.actual`）と `gate.satisfied` を別 field として常に併記し、text 出力でも検証状態の行とゲート満否の行を分けて表示する。

### DS-1178

`--gate` 指定時の `ok: true`・終了コード 0 を検証状態 `PASS` と読ませる表示（例：検証状態の行を省略する、`PASS` の語をゲート満否に流用する）はしない。

### DS-1179

具体的なフェーズ名・承認ロール・必要承認数・権限 schema はプロジェクト設定（`config.yaml`）へ委譲する。

*導出元: SPEC-230, SPEC-231, SPEC-232, SPEC-233, SPEC-234, SPEC-235, SPEC-236, SPEC-237, SPEC-238, SPEC-239, SPEC-240, SPEC-241, SPEC-242, SPEC-243, SPEC-244, SPEC-245, SPEC-246, SPEC-247, SPEC-248, SPEC-249, SPEC-250, SPEC-251, SPEC-252, SPEC-253, SPEC-254, SPEC-255*

*引用: 基本仕様 §30 items 22-23*

### DS-S133 12.4 判断待ち情報 section（`verify` / `report` JSON）

*導出元: REQ-S043, SPEC-S047, SPEC-S066, SPEC-S100*

### DS-1180

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

### DS-1181

`kind` は `unknown`（`UNKNOWN` によるエスカレーション）/ `unregistered`（管理宣言欠落）/ `unresolved`（参照解決不能）/ `undecided`（VO 未確定）/ `pending_approval`（承認待ち）のいずれかである。

### DS-1182

`check` は関係する4検査のいずれかと現在の検証状態・診断ラベルである。

### DS-1183

4 検査のいずれにも由来しない項目（判断型に由来する項目・判断競合）では `check` を `null` とする。

### DS-1184

`check` が `null` の項目は集約へ寄与せず、いかなる検査の値も変更しない。

*引用: 本冊 §11.3*

### DS-1185

`judgment_kind` は外部判断が必要な場合の判断型（`test-semantic` / `impl-consistency` / `case-coverage`）である。

*引用: 本冊 §8.1*

### DS-1186

不要な項目では `judgment_kind` を `null` とする。

### DS-1187

`basis` は機械的に確認済みの事実（宣言鎖・検査結果・対象外とした範囲）への参照である。

### DS-1188

判断競合の項目では `basis` に競合した全判断記録 ID を `kind: decision` として列挙する。

### DS-1189

`bundle_ref` は外部判断が必要な場合の判断バンドル（§8.1）への参照であり、任意である。

## DS-S134 13. MCP ツール詳細仕様

### DS-S135 13.1 共通仕様

*導出元: SPEC-S062*

### DS-1190

各ツールの結果は CLI の `--format json` と同一の JSON 構造とする（検証状態 `state` と診断ラベル `diagnostic` の2軸を含む。§12.1）。

### DS-1191

エラーは MCP のツールエラーとして返し、`{ "code": "E-OP-001", "message": "...", "candidates": [...] }` の構造を含める。

### DS-1192

入力検証エラーには可能な限り `candidates` を含める。

*引用: 本冊 §6.3*

### DS-S136 13.2 ツール一覧

*導出元: SPEC-S062, SPEC-S075, SPEC-S087, SPEC-S094*

### DS-1193

`scan` は入力を取らず、診断一覧、エンティティ数サマリを出力する。

### DS-1194

`doc_list` / `doc_get` は `id`（get のみ）、`tree: bool`、`roots: bool` を入力とし、document レコード（木・根集合・鮮度）を出力する。

### DS-1195

`doc_upsert` は document フィールド一式（`path`、`derives_from[]`（`doc` + 任意 `anchor` + 任意 `note`）、`root: bool`、`update: bool`）を入力とし、作成・更新結果（依存判断・承認の失効警告を含む）を出力する。

### DS-1196

`approval_create` は `subject: { type: vo | document | judgment, id }`、`approver`、`state`（`approved` / `rejected` / `withdrawn`）、`basis[]`（任意）、`supersedes[]`（任意）を入力とし、承認レコード ID を出力する。

### DS-1197

`approval_withdraw` は `approval_id`、`approver`、`basis[]`（任意）を入力とし、承認レコード ID を出力する。

### DS-1198

`approval_get` は `subject: { type, id }` を入力とし、承認レコード一覧（`approved_state` / `supersedes` / 有効性）と実効承認状態（`draft` / `approved`）を出力する。

### DS-1199

`vo_list` / `vo_get` は `id`、`doc`、`status` を入力とし、VO レコード、derives_from（`doc` + 任意 `anchor` + 任意 `note`）、covers 状況、承認状態を出力する。

### DS-1200

`vo_upsert` は VO フィールド一式（`derives_from[]` 必須1件以上（`doc` + 任意 `anchor` + 任意 `note`）、`dimensions[]`、`coverage_policy`、`combinations[]`（`explicit` のとき必須1件以上。各要素は dimension 名 → partition 値の map））を入力とし、作成・更新結果（承認失効の警告含む）を出力する。

### DS-1201

`vo_expand` は `id`、`dry_run: bool` を入力とし、生成される子 VO 一覧を出力する。

### DS-1202

`vo_approve` は `id`、`approver`、`state`（必須）、`basis[]`（任意）、`supersedes[]`（任意）を入力とし、承認レコード ID を出力する。

### DS-1203

`test_query` は `vo` / `source` / `unregistered` のいずれかを入力とし、Test 一覧を出力する。

### DS-1204

`test_get` は `id` を入力とし、Test 詳細（intent、covers、targets、位置、判断記録・Evidence 状態）を出力する。

### DS-1205

`form_get` は大局的に一意な `kind` を入力とし、owner adapter を明示した Form Schema（§14）を出力する。

### DS-1206

`test_create` は `form`、`answers`（オブジェクト）、`dry_run` を入力とし、生成された Test ID、挿入位置、diff を出力する。

### DS-1207

`test_edit` は `id`、`answers` または `set`、`body`、`dry_run` を入力とし、更新結果、diff を出力する。

### DS-1208

`audit_static` は `test` または `all` を入力とし、rule 別 verdict（target-scoped な DA-002 / DA-003 は target 別 verdict を含む）と根拠 span を出力する。

*引用: 本冊 §3.6・§7.2*

### DS-1209

`audit_static` は正典レコードを生成しない。

*引用: 本冊 §7.1*

### DS-1210

`audit_bundle` は対象 ID（`test` / `vo`）、`kind`（`test-semantic` / `impl-consistency` / `case-coverage`。`test` では省略時 `test-semantic`、`vo` では `case-coverage` を必須）を入力とし、bundle_id と `judgment_kind` を含むバンドル本体（JSON）を出力する。

### DS-1211

`audit_submit` は提出 JSON（`judgment_kind` 必須、`supersedes[]` 任意）を入力とし、受理結果、判断記録 ID（`.verify/decisions/`）を出力する。

*引用: 本冊 §8.3*

### DS-1212

`audit_submit` の受理は検証状態を昇格させない。

### DS-1213

`run_tests` は `test` / `vo` / `all`、`fast: bool` を入力とし、Test ごとの結果と Evidence ID を出力する。

### DS-1214

`verify` は optional `items[]`（4検査の部分集合）、`doc` / `vo` / `test`、`gate`（任意）を入力とし（`items` 省略は固定4検査）、最上位 `scope`（§12.1）、総合 OK / NG、集約ツリー、`pending` section、`data.gate` 評価（指定時）を出力する。

### DS-1215

`report` は `verify` と同上の入力に `from` / `view` / `depth` / `direction` を加えて受け取り（`items` 省略は固定4検査）、最上位 `scope`（§12.1）、根拠付き完全レポート、projection（親 VO 起点の機能単位の束ねを含む）、`pending` section を出力する。

### DS-1216

`verify` / `report` の `gate` 入力は CLI の `--gate` と同じ解決規則に従い、config に定義の無いゲート名は E-CONFIG-002 の tool error（`candidates` に定義済みゲート名）とし、検証結果・部分結果を返さない。

*引用: 本冊 §11.5・§17.1*

### DS-1217

`gate` を指定した呼び出しの `ok` はゲート充足を表す（§12.3）。

### DS-1218

`doc_upsert` / `vo_upsert` の `derives_from[]` 各要素は `doc`（必須）、`anchor`（任意）、`note`（任意）からなる。

### DS-1219

`anchor` は参照先 document 内の該当箇所を指す不透明な文字列であり、省略・空文字列を許容し `chain_integrity` 違反にしない。

*引用: 本冊 §3.1・§3.2*

### DS-1220

`anchor` は CLI の `--anchor` と同じ値域・同じ扱いとし、文書内位置への解決・実在確認を行わない。

### DS-1221

`vo_upsert` の `combinations[]` は `combinations` を desired state として与える。

*引用: 本冊 §3.2.1*

### DS-1222

`combinations[]` の各要素は dimension 名 → partition 値の map（例 `{"operand-sign": "positive", "operator": "div"}`）で、`dimensions` に宣言された全軸をちょうど 1 回ずつ持つ。

### DS-1223

`coverage_policy` が `explicit` のときは `combinations[]` は 1 件以上を必須とし、`explicit` 以外のときは省略または空 list でなければならない。

### DS-1224

本冊 §3.2.1 の受理条件（`explicit` での欠落・空、`explicit` 以外での非空、未宣言 dimension、未列挙 partition、宣言 dimension の欠落・重複、重複 tuple、`dimensions` 空での `explicit`）に違反する入力は、`ok: false` と `{ "code": "E-SCAN-017", ... }` の tool error で拒否し、レコードを作成・更新しない。

*引用: 本冊 §3.2.1*

### DS-1225

`vo_upsert` で `combinations` を省略した更新は既存値を保持し、空 list を明示した更新は既存値を空にする。

### DS-1226

`vo_expand` は不正 `combinations` の VO に対して同じ E-SCAN-017 で拒否し、子 VO を 1 件も生成しない。

### DS-1227

`audit_static` は正典の監査レコード ID を返さない（再計算派生）。

*引用: 本冊 §7.1*

### DS-1228

`audit_submit` の受理結果は判断記録 ID であり、これは検証状態を変えない。

*引用: 本冊 §8.3*

### DS-1229

旧モデルの `spec_list` / `spec_get` / `req_list` / `req_get` / `req_upsert` は廃止し、`doc_*` へ統合した。

### DS-S137 13.3 エージェント向け利用フロー（参考）

*導出元: SPEC-S031, SPEC-S059, SPEC-S087*

### DS-1230

フォーム、監査、実行の入力に含まれる adapter namespace は opaque 値として扱い、未登録 adapter や未提供 capability を Rust 用の既定値へ暗黙変換しない。

### DS-1231

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

*導出元: SPEC-116, SPEC-117, SPEC-118, SPEC-119, SPEC-120, SPEC-121, SPEC-122, SPEC-333, SPEC-334, SPEC-335*

*引用: 本冊 §8, 基本仕様 §11.3*

## DS-S138 14. Form Schema 設計

### DS-S139 14.1 スキーマ形式（`.verify/forms/<kind>.yaml`）

*導出元: SPEC-S041, SPEC-S066*

### DS-1232

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

### DS-1233

core は `fn_name`、`.rs`、Rust 構文を Form Schema の共通 field として要求しない。

### DS-1234

`test_kind` の `regression` は Test の意図ラベル（`@vtest.kind` の値）であり、廃止された存在理由分類（role / anchor）とは別概念である。

*引用: 本冊 §4.1・§4.2*

### DS-1235

組込 Form は `role` を宣言しない。

### DS-1236

`kind` の値に regression を含む Test（`unit-regression` 等）も `kind` から存在理由分類を導出しない。

### DS-S140 14.2 検証器

*導出元: SPEC-S041, SPEC-S078, SPEC-S083*

### DS-1237

`required` を欠く回答、未知のフィールド名は E-OP-001 とする。

### DS-1238

Test ID は `--id` による明示指定がなければ、`TEST-<領域>-<連番>`（領域は covers 先 VO の ID から継承、連番は既存最大＋1）で自動採番し、結果に含めて返す。

### DS-1239

`kind` は `[a-z0-9][a-z0-9-]*` の case-sensitive 文字列で、`.verify/forms/<kind>.yaml` のファイル名と一致する repository-global な Form ID である。

### DS-S141 14.3 組込フォーム

*導出元: SPEC-S041, SPEC-S063*

### DS-1240

未提供の Structured Test capability は E-ADAPTER-004 として作成・編集を中止し、ファイルを変更しない。

### DS-1241

`rust-integration` は組込 Form であり、単一の `target` field に代えて、1件以上のロケータを持つ `targets` を必須入力として受け取る。

### DS-1242

`rust-integration` は `file` を `required:true` とする。

### DS-1243

Integration Test の配置先（test suite location）は Source Target の location とは別概念であり、targets から一意に導出できないためである。

### DS-1244

将来、Test Suite または同等の配置概念が第一級化され配置先を一意に導出できる規則が導入された場合にのみ、省略可能性を再検討する。

### DS-1245

`rust-integration` の §14.1 との差分はこの2点であり、他は同一。

### DS-1246

`rust-integration` は `targets` の全要素を入力順に個別の `@vtest.target` 行として出力する。

### DS-1247

`rust-integration` は空 list と重複 target を E-OP-001 で拒否する。

### DS-1248

`target` キーは integration 種別に限り複数行を許容する。

*引用: 本冊 §4.2の例外*

### DS-1249

先頭以外の target を `@vtest.related` へ変換しない。

### DS-S142 14.4 テスト種別ごとのフォーム拡張

*導出元: SPEC-S035, SPEC-S041, SPEC-S078*

### DS-1250

すべての管理対象 Test に `covers ≥ 1` を一律要求するため、user-defined Form も `covers` を `required: true` の `vo-ref-list` として持つ。

*導出元: SPEC-123, SPEC-124, SPEC-369, SPEC-370, SPEC-371, SPEC-372, SPEC-373, SPEC-374, SPEC-375, SPEC-376, SPEC-377, SPEC-378, SPEC-379, SPEC-380, SPEC-381, SPEC-382, SPEC-383, SPEC-384, SPEC-385, SPEC-386, SPEC-387, SPEC-388, SPEC-389, SPEC-390, SPEC-391, SPEC-392, SPEC-393, SPEC-394, SPEC-395, SPEC-396, SPEC-397, SPEC-398, SPEC-399, SPEC-400, SPEC-401, SPEC-402, SPEC-403, SPEC-404, SPEC-405, SPEC-406, SPEC-407*

*引用: 本冊 §4.1, 基本仕様 §12*

### DS-1251

本 version は role / anchor / anchor_rationale による存在理由分類・固定 Form 群を持たない。

### DS-1252

本 version は `covers` 件数の可変制約も設けない。

## DS-S143 15. Structured Test Operation adapter contract

### DS-S144 15.1 `rust-cargo` 対象の特定

*導出元: SPEC-S040*

### DS-1253

再確認で見つからない場合は E-OP-002 とする。

### DS-S161 15.2 `rust-cargo` 編集・挿入の適用

*導出元: SPEC-S039, SPEC-S040*

### DS-1589

回答自体の検証エラーは E-OP-001（候補付き）とする。

*引用: 本冊 §6.3*

## DS-S145 18. 受入契約

### DS-S146 18.1 共通条件

*導出元: SPEC-S053, SPEC-S056*

### DS-1254

要求scopeに1件でも非PASSがあれば総合結果はNGになる。

### DS-1255

scopeを限定してもscope外の値をPASSへ変更しない。

### DS-S147 18.2 共通fixture

*導出元: SPEC-S008, SPEC-S012, SPEC-S013, SPEC-S014, SPEC-S019, SPEC-S035, SPEC-S081, SPEC-S086*

### DS-1256

すべての管理対象 Test に `covers ≥ 1` を一律要求するため、`covers` を宣言しない Test は E-SCAN-007 と `chain_integrity = MISMATCH`（診断`MISSING`）になる。

*導出元: SPEC-123, SPEC-124, SPEC-369, SPEC-370, SPEC-371, SPEC-372, SPEC-373, SPEC-374, SPEC-375, SPEC-376, SPEC-377, SPEC-378, SPEC-379, SPEC-380, SPEC-381, SPEC-382, SPEC-383, SPEC-384, SPEC-385, SPEC-386, SPEC-387, SPEC-388, SPEC-389, SPEC-390, SPEC-391, SPEC-392, SPEC-393, SPEC-394, SPEC-395, SPEC-396, SPEC-397, SPEC-398, SPEC-399, SPEC-400, SPEC-401, SPEC-402, SPEC-403, SPEC-404, SPEC-405, SPEC-406, SPEC-407*

*引用: 本冊 §11.1.1, 基本仕様 §12*

### DS-1257

DA-002 / DA-003がtarget別UNKNOWNになる。

### DS-1258

runtimeの`target_coverage`のみでDA-002到達が充足される。

### DS-S148 18.3 機能別受入条件

#### DS-S149 18.3.1 discovery・record・graph と chain_integrity

*導出元: SPEC-S009, SPEC-S018, SPEC-S028, SPEC-S043, SPEC-S077, SPEC-S083, SPEC-S095*

### DS-1259

恒久SRC IDを持つSource Targetはcanonical locatorでもaddressableであり、locator参照とSRC ID参照は同一のcanonical Source Targetへ解決する。

### DS-1260

両addressing modeで同一のSource Target hashに到達し、Source Targetの件数、content / subject hash、Evidenceおよび判断記録上のtarget identityが参照方法によって分裂しない。

### DS-1261

Evidence、判断記録、`target_binding` の証拠、鮮度判定は解決後のcanonical Locatorをidentityとして記録・比較し、参照側Testが宣言した綴り（SRC ID参照を含む）を保存しない。

### DS-1262

同一のSource Targetをlocator参照するTestとSRC ID参照するTestは、Evidence上で同一のtarget identityを持つ。

### DS-1263

Testがどう宣言したかの変更（同一Source Targetに対するlocator参照からSRC ID参照への書き換え等）はTest subject hashの変化として捕捉され、Evidence側のtarget identityを変化させない。

### DS-1264

綴りの異なる複数の`target`宣言が同一のcanonical Source Targetへ解決する場合はE-SCAN-005とする。

### DS-1265

`TargetRef::SrcId`をcanonical targetとして返したadapter出力はmalformed adapter outputとして拒否する。

### DS-1266

恒久SRC IDの宣言・変更・削除でcanonical locatorは変化しない。

### DS-1267

Source Target hashは常にcanonical locatorとconstruct bytesから計算し、参照側Testの`TargetRef`綴りからは計算しない。

### DS-1268

恒久SRC IDを独立したhash fieldとしてSource Target hashのinputに含めない。

### DS-1269

恒久SRC IDの宣言をSource Target constructの内側へ置くadapter（`rust-cargo`の`@vtest.src-id` doc comment等）では、その宣言の付与・変更・削除がconstruct bytesを変えるため、Source Target hashも変化する。

### DS-1270

Source Target hashも変化することはsourceが実際に変化したことの帰結として正しい挙動であり、恒久SRC IDが独立したhash fieldであることを意味しない。

### DS-1271

Target Reference解決は解決済み / 対象なし / 曖昧を区別し、曖昧はfail-closedな終端状態とする。

### DS-1272

E-SCAN-004またはE-SCAN-011で曖昧・未解決となったtargetについて、判断記録subject、Evidence、`target_binding` の証拠のいずれも候補の1件を解決結果として記録しない。

### DS-1273

候補は診断表示にだけ用いる。

### DS-1274

判断記録subject、Evidence、`target_binding` の証拠のいずれも候補の1件を解決結果として記録しないという禁止は解決に関するものであり、Source Targetの具体化を止めない。

### DS-1275

恒久SRC IDが衝突していても、各Source Targetは自身のcanonical locatorで独立したentityとして具体化され、Source Targetの件数と各content / subject hashは衝突の有無で変化しない。

### DS-1276

衝突が壊すのは当該恒久SRC IDによる参照の一意性だけである。

### DS-1277

Test 層は、管理宣言または必須metadata（core 中立の Test ID・`covers ≥ 1`・`intent`、および当該 adapter が必須とする追加 metadata〔`rust-cargo` では `targets ≥ 1`〕）を持たないTestが1件でもあれば、W-SCAN-101またはE-SCAN-007を表示し、`ManagedTestLink::Missing`から`chain_integrity = MISMATCH`（診断 `MISSING`）を導出する。

*導出元: SPEC-057, SPEC-058*

*引用: 本冊 §11.1.1, 基本仕様 §5.1*

### DS-1278

存在しないVOを`covers`するTestは構造上完全なManaged Test Entityと`ManagedTestLink::One`のまま保持し、E-SCAN-003と`chain_integrity = MISMATCH`を導出する。

### DS-1279

`ManagedTestLink::Multiple`またはTest ID衝突（E-SCAN-002）は`chain_integrity = MISMATCH`になる。

### DS-1280

`covers` を持たない（0 件の）Testは管理宣言不整合として`chain_integrity = MISMATCH`（診断 `MISSING`）になる。

### DS-1281

役割による`covers`可変制約・特別扱いの分岐を設けず、すべての管理対象 Test に`covers ≥ 1`を一律要求する。

*導出元: SPEC-123, SPEC-124, SPEC-369, SPEC-370, SPEC-371, SPEC-372, SPEC-373, SPEC-374, SPEC-375, SPEC-376, SPEC-377, SPEC-378, SPEC-379, SPEC-380, SPEC-381, SPEC-382, SPEC-383, SPEC-384, SPEC-385, SPEC-386, SPEC-387, SPEC-388, SPEC-389, SPEC-390, SPEC-391, SPEC-392, SPEC-393, SPEC-394, SPEC-395, SPEC-396, SPEC-397, SPEC-398, SPEC-399, SPEC-400, SPEC-401, SPEC-402, SPEC-403, SPEC-404, SPEC-405, SPEC-406, SPEC-407*

*引用: 基本仕様 §12*

### DS-1282

既定を緩和して0件を受理しない。

### DS-1283

全Discovered Testが`ManagedTestLink::One`で構造上完全なentityへ1対1で対応し、Test IDが一意、各entityが`covers ≥ 1`を満たし、かつ全VO参照を解決できる場合だけTest層の`chain_integrity`が成立する。

### DS-1284

各 VO は 1 件以上の `document` への解決可能な `derives_from` を持つ。

### DS-1285

参照先 document が存在しない、または解決不能な場合は E-SCAN-012、`chain_integrity = MISMATCH`。

### DS-1286

VO parent の不在・循環は E-SCAN-008、`chain_integrity = MISMATCH`。

### DS-1287

各`document`の`derives_from`参照先が存在することを要求する（不在はE-SCAN-012、`chain_integrity = MISMATCH`）。

### DS-1288

各`document`の`content_hash`が実ファイル（`path`）と一致することを要求する（不一致はW-SCAN-104、`chain_integrity = MISMATCH`、診断`STALE`）。

### DS-1289

`covers` する Test が 1 件以上存在しない leaf VO は `chain_integrity = MISMATCH`（診断 `MISSING`）。

### DS-1290

発見された Test → 管理宣言の解決と、leaf VO → Test の両方向が成立して初めて `chain_integrity` が成立する。

### DS-1291

W-SCAN-101のwarning severityだけを理由に検証値を変更せず、Discovered Testとmanaged entityの対応事実から判定する。

### DS-1292

adapter discoveryの失敗をTest 0件の正常scanとして扱わない。

### DS-1293

解析不能・不完全なbatchは対応する検証を`UNKNOWN`とする。

### DS-1294

同じpayloadのbare / prefixed重複、混在形、ファイル名とIDの不一致はE-SCAN-010になる。

### DS-1295

Relation の from / to 不在は E-SCAN-009、`chain_integrity = MISMATCH`。

### DS-1296

読取り互換field `status`は警告（W-STORE-001）して無視する。

### DS-1297

VOの承認はVO内容hashと現在の上流依存closureへ束縛され、`document` / parent VO の内容または集合が不一致の承認を有効として扱わない。

### DS-1298

Approval作成時に対象または上流依存closureを完全・currentに解決できなければE-APPROVAL-001で拒否し、recordを生成しない。

### DS-1299

上流依存closureまたはハッシュを欠く互換Approvalを現在のapprovedへ昇格しない（W-STORE-002、VOは`draft`相当）。

### DS-1300

恒久SRC IDは全adapter統合後にrepository全体で一意である。

### DS-1301

恒久SRC IDの衝突をE-SCAN-011として拒否する。

### DS-1302

`vtest scan` / `doctor`はE-ADAPTER-* / E-CONFIG-*による操作拒否をexit 2にする。

### DS-1303

`vtest scan` / `doctor`は完了したscanのE-SCAN-*をexit 1にする。

### DS-1304

`vtest scan` / `doctor`はerrorなしをexit 0にする。

### DS-1305

`full-product` VOは宣言partitionの直積を決定論的に実体化する。

### DS-1306

`coverage_policy: explicit`と妥当な`combinations`を持つVOは、列挙されたtupleごとにちょうど1件の子VOを生成する。

*引用: 本冊 §3.2.1・§17.1*

### DS-1307

子VO IDのsuffixは`dimensions`の宣言順で連結される。

### DS-1308

同じtuple集合を記述順・map key順を変えて与えても、生成される子VO集合とIDは同一になる。

### DS-1309

`explicit`かつ`combinations`欠落を持つVOレコードはE-SCAN-017と`chain_integrity = MISMATCH`になり、`vo expand`は子VOを1件も生成しない（部分生成しない）。

### DS-1310

`explicit`かつ`combinations`空listを持つVOレコードはE-SCAN-017と`chain_integrity = MISMATCH`になり、`vo expand`は子VOを1件も生成しない（部分生成しない）。

### DS-1311

`explicit`かつ`dimensions`空を持つVOレコードはE-SCAN-017と`chain_integrity = MISMATCH`になり、`vo expand`は子VOを1件も生成しない（部分生成しない）。

### DS-1312

`independent-axes` / `full-product` / `null`かつ`combinations`非空を持つVOレコードはE-SCAN-017と`chain_integrity = MISMATCH`になり、`vo expand`は子VOを1件も生成しない（部分生成しない）。

### DS-1313

未宣言dimension名を含むtupleを持つVOレコードはE-SCAN-017と`chain_integrity = MISMATCH`になり、`vo expand`は子VOを1件も生成しない（部分生成しない）。

### DS-1314

当該dimensionの`partitions`に無いpartition値を含むtupleを持つVOレコードはE-SCAN-017と`chain_integrity = MISMATCH`になり、`vo expand`は子VOを1件も生成しない（部分生成しない）。

### DS-1315

宣言済みdimensionを欠くtupleを持つVOレコードはE-SCAN-017と`chain_integrity = MISMATCH`になり、`vo expand`は子VOを1件も生成しない（部分生成しない）。

### DS-1316

同一dimension名を2回持つtupleを持つVOレコードはE-SCAN-017と`chain_integrity = MISMATCH`になり、`vo expand`は子VOを1件も生成しない（部分生成しない）。

### DS-1317

重複tupleを持つVOレコードはE-SCAN-017と`chain_integrity = MISMATCH`になり、`vo expand`は子VOを1件も生成しない（部分生成しない）。

### DS-1318

`vo add` / `vo edit` / MCP `vo_upsert` は上記の各入力を受理時に E-SCAN-017・終了コード 2 で拒否し、レコードを作成・更新しない（拒否後に scan したエンティティ集合は操作前と同一）。

### DS-1319

`vo edit --combination` は desired state として既存 `combinations` を置換し、追記しない。

### DS-1320

`--clear-combinations` は空にする。

### DS-1321

`--combination`と`--clear-combinations`のどちらも与えない `edit` は既存 `combinations` を保持する。

### DS-1322

`combinations` だけを変更した `edit` は VO subject hash を変化させ、当該 VO の承認を失効させる。

### DS-1323

document / VO の `derives_from` entry に `anchor` を持つ状態と持たない状態の双方を読み取り、いずれも `chain_integrity` に影響しない（`anchor` の欠落・空文字列で `MISMATCH` にならない）。

*引用: 本冊 §3.1・§3.2*

### DS-1324

`anchor` の値を文書内位置へ解決せず、実在しない節番号を書いても診断を出さない。

### DS-1325

同一 `doc` を指す複数 `derives_from` entry を `anchor` 違いで保持でき、重複として拒否しない。

### DS-1326

`anchor`だけを変更したdocumentは`content_hash`（`path`の実ファイルのハッシュ）が不変のままdocument subject hashが変化する。

### DS-1327

`anchor`だけを変更したdocumentは、当該documentを上流依存closureに含む承認・判断記録を失効させる。

### DS-1328

`anchor` だけを変更した VO は VO subject hash が変化せず、当該 VO の承認が失効しない。

### DS-1329

CLI で `--derives-from` を伴わない `--anchor`、または 1 つの `--derives-from` に 2 個目の `--anchor` を与えた場合は終了コード 2 で拒否し、レコードを書かない。

### DS-1330

既存ソース・既存テストを含む fixture project で `vtest init` を実行した前後で、`.verify/` を除いた作業ツリーの全ファイルのバイト列が同一である。

*導出元: SPEC-158, SPEC-159, SPEC-160, SPEC-161, SPEC-162, SPEC-163, SPEC-414*

*引用: 別紙A §12.2, 基本仕様 §18.1*

### DS-1331

`.verify/` 外のファイルの新規作成・変更・削除が 1 件も観測されない。

### DS-1332

`init` は既存ソースへ Test metadata 宣言（`@vtest.` 行）・annotation・doc comment を挿入しない。

### DS-1333

既存 `.verify/` があるプロジェクトでの `init` は終了コード 2 で中止し、その実行でファイル・ディレクトリを 1 件も作成・変更・削除しない（既存 `.verify/` の内容も不変）。

#### DS-S150 18.3.2 orphan_detection（文書層の孤児検出）

*導出元: REQ-S010, SPEC-S019, SPEC-S081*

### DS-1334

根の除外は、`config.yaml` の `doc.roots` に列挙された DOC ID を根として扱い、`orphan_detection` の対象外とする。

### DS-1335

孤児判定は、`derives_from` が空、かつ他のどの document からも `derives_from` で参照されず、`doc.roots` にも列挙されない document を孤児とし、E-SCAN-016、`orphan_detection = MISMATCH` になる。

### DS-1336

`doc.roots` が存在しない DOC ID を参照する場合は config invariant 違反として E-CONFIG-001 とする。

### DS-1337

旧モデルの W-SCAN-102（孤立 VO）は VO 層の警告であり、文書層 `orphan_detection` とは別物として存置する。

#### DS-S151 18.3.3 決定論的静的解析（oracle_presence・target_binding 静的到達）

*導出元: SPEC-S021, SPEC-S025, SPEC-S085, SPEC-S086*

### DS-1338

DA-001〜DA-006とW-DA-101は本冊§7の判定条件に従う。

*引用: 本冊 §7*

### DS-1339

正常Testは違反なしとなり、各違反fixtureは対応ruleで非PASSになる。

### DS-1340

`oracle_presence` は DA-001 / DA-003 / DA-004 / DA-005 / DA-006 の合成とし、全ルール違反なしで `PASS` になる。

*導出元: SPEC-066, SPEC-067, SPEC-068, SPEC-069, SPEC-290*

*引用: 本冊 §7.1, 基本仕様 §5.4*

### DS-1341

1つでも `FAIL` があれば `oracle_presence` は `FAIL` になる。

### DS-1342

`FAIL` がなく `UNKNOWN` があれば `oracle_presence` は `UNKNOWN` になる。

### DS-1343

`oracle_presence` に動的な昇格経路は無く、runtime 証拠で `PASS` へ昇格しない。

### DS-1344

Test の成否判定が assert 相当の構文でなく通常の関数へ委譲されている場合において、委譲先を宣言targetとするTestが存在し、その`oracle_presence`がすべて`PASS`であるTestは、DA-003 / DA-006が違反なしとなる。

*引用: 本冊 §7.2.1*

### DS-1345

委譲先のassert相当が委譲先側にしか無いことだけを理由に`FAIL`としない。

### DS-1346

委譲先を宣言targetとするTestが0件のTestは、DA-003 / DA-006が`UNKNOWN`となる。

### DS-1347

常に真を返す照合ヘルパを呼ぶだけのTestが`oracle_presence` = `PASS`にならない。

### DS-1348

委譲先を宣言targetとするTestは存在するが、その`oracle_presence`が`PASS`でないTestは、DA-003 / DA-006が`UNKNOWN`となる。

### DS-1349

委譲先の終端が循環する（相互に照合を委譲し合う）2 Testは、いずれもDA-003 / DA-006が`UNKNOWN`となり、評価順序を変えても同じ値になる。

### DS-1350

委譲先が他ファイル・他クレート・マクロ展開内で同定できないTestは、DA-003 / DA-006が`UNKNOWN`となる。

### DS-1351

DA-002の target別verdictが`UNKNOWN`のとき、当該targetのruntime計測（§18.3.5）が実行を証明した場合に限り到達要件が充足される。

*引用: 本冊 §7.3*

### DS-1352

DA-002の target別verdictが`UNKNOWN`のとき、当該targetのruntime計測（§18.3.5）が実行を証明した場合に限り到達要件が充足されるというruntime救済は`target_binding`に固有であり、`oracle_presence`には及ばない。

### DS-1353

static audit adapterが判定へ使用したsource fragment集合の完全性を保証できない場合、当該判定はUNKNOWNとなりPASSにならない。

### DS-1354

別プロセス・別スレッド・クロージャ・他ファイル等、静的解析の到達境界を越えてtargetを実行するTestは、当該targetのtarget別DA-002 verdictがUNKNOWNになる。

*引用: 本冊 §7.3*

### DS-1355

当該targetのruntime`target_coverage`がPASS（checked: true・count > 0）ならDA-002到達要件は充足され、検証時にそのtarget別DA-002はUNKNOWN扱いにならない。

### DS-1356

呼出自体を静的に確認できないtarget（subprocess spawn等）は、DA-002だけでなくDA-003のtarget別verdictもUNKNOWNになる（空虚PASS / FAILとしない）。

*引用: 本冊 §7.3*

### DS-1357

呼出自体を静的に確認できないtargetについて、DA-003はruntimeで救済されない。

### DS-1358

したがってexit code / stdoutだけをassertするsubprocess E2Eは、当該targetのDA-002がruntimeで充足されて`target_binding = PASS`に到達しうる一方で、DA-003がUNKNOWNのまま残り`oracle_presence = UNKNOWN`となる。

### DS-1359

DA-002とDA-003の2検査が別々の値をとる場合が新モデルの識別fixtureであり、総合判定はNGになる。

### DS-1360

他ファイル・他クレートへ呼び出すが戻り値をTest本体内でassertするTestは、DA-002 UNKNOWN・DA-003 PASSとなり、runtime`target_coverage`がPASSでかつ他ルールも違反なしなら`target_binding`は到達充足、`oracle_presence = PASS`になる（runtime救済で実益が出るのはこの型）。

### DS-1361

複数targetを宣言するTestで、target Aは静的（DA-002 = PASS）、target Bはruntime（Bのtarget別`target_coverage` = PASS）でDA-002到達を充足する場合、BもTest本体内で結果をassertしDA-003 = PASSなら`oracle_presence = PASS`かつBの`target_binding`到達も充足する。

### DS-1362

Bが呼出不可視（subprocess）でDA-003 UNKNOWNなら`oracle_presence = UNKNOWN`となる。

### DS-1363

到達判定はtarget別に行い、AとBのstatic verdictを取り違えない。

### DS-1364

DA-002 verdict = FAIL（解析境界内で到達を静的に否定）は runtime 証明で覆らない。

### DS-1365

runtime証明に依存する`target_binding`の値は、§18.3.4の鮮度判定が選択した最新Evidenceが鮮度を満たすときだけ用い、無効な最新Evidenceから古い有効Evidenceへフォールバックしない。

*引用: 本冊 §11.2*

### DS-1366

無効な最新Evidenceから古い有効Evidenceへフォールバックしないことにより同一検証内で計測がSTALEの一方`target_binding`が別Evidenceで PASSになる履歴不一致を生じない。

### DS-1367

`vtest verify --items oracle_presence` / `--items target_binding` のような限定scopeでも、aggregatorは本冊§7.3のruntime到達判定に必要なEvidence鮮度・target別`target_coverage`を内部依存として評価するが、scope外の項目自体のreport valueは`NO_EVIDENCE`（診断`NOT_CHECKED`）のまま保持する。

*引用: 本冊 §7.3*

### DS-1368

同じ到達UNKNOWNのTestでも、当該targetの`target_coverage`がFAIL・UNKNOWN・NOT_CHECKED（coverage利用不能・未計測・`--fast`）なら到達要件は未充足で、当該targetのDA-002 UNKNOWNは`target_binding`の非PASS要因として残る。

### DS-1369

runtime coverageはDA-003を代替しない。

### DS-1370

結果検証はDA-003の静的判定（結果がassert相当へ到達）のまま評価し、到達がruntimeで充足されてもDA-003 UNKNOWN / FAILはそのまま`oracle_presence`へ寄与する。

### DS-1371

宣言targetをどのtopologyでも実行しない構造・契約のみのTestは、静的にもruntimeにも到達を確立できず`target_binding`の到達要件は未充足のままになる。

#### DS-S152 18.3.4 execution・Evidence（target_binding の証拠）

*導出元: SPEC-S051, SPEC-S076*

### DS-1372

選択した登録Testだけをrunnerのexact selectorで実行する。

### DS-1373

Testごとの結果、revision、hash、adapter ID、runner情報、およびExecution State subjectをEvidenceへ記録する。

### DS-1374

build failure、runner failure、必須runner capabilityの欠落、および宣言targetの解決失敗ではEvidenceを生成しない。

### DS-1375

実行前後でExecution State subjectが変化した場合はE-EXEC-004となり、Evidenceを生成しない。

### DS-1376

Evidence writerはadapter IDを必ず記録する。

### DS-1377

`test_fn` / `test_construct` / `target_fn`の互換入力は`rust-cargo` Evidenceで全canonical metadataを含むsource rangeと現在値の同一性を証明できる場合だけ受理する。

### DS-1378

Evidence readerはadapter IDを欠くrecordについて、現在のTestが `rust-cargo` で、runner kindと内容hashからRust実行を一意に確認できる場合だけ互換Evidenceとして扱う。

### DS-1379

Evidenceは全宣言targetを解決したcanonical Locatorと内容hashを重複なく保持し、参照側Testが宣言した`TargetRef`の綴り（SRC ID参照を含む）をtarget identityとして保存しない。

### DS-1380

同一Source Targetをlocator参照するTestとSRC ID参照するTestのEvidenceは、同じtarget identityと同じtarget内容hashを持つ。

### DS-1381

全宣言targetがcanonical Source Targetへ一意に解決できることをEvidence生成のpreconditionとする。

### DS-1382

1件でも対象なしまたは曖昧ならEvidenceを生成しない。

### DS-1383

部分的な`hashes.targets`を持つEvidenceを生成しない。

### DS-1384

Evidenceを生成しない場合`target_binding`は`NO_EVIDENCE`（診断`NOT_EXECUTED`）のままとなる。

### DS-1385

Evidence記録後に宣言targetのいずれかが一意に解決できなくなった場合、記録済み参照集合が現在のcanonical集合と一致しないため`NO_EVIDENCE`（診断`STALE`）になり、`target_binding`をPASSにしない。

### DS-1386

解決できなくなったtargetは、対象が存在しない場合（E-SCAN-004）は`MISMATCH`（診断`MISSING`）、複数候補により曖昧な場合（E-SCAN-011）は`MISMATCH`として保持する。

### DS-1387

両者を一括して同一の状態値にしない。

### DS-1388

canonical Test metadata、ExecutionDescriptor、Test construct、宣言target集合、いずれかのtarget内容hash、HEAD revision、またはExecution State subjectがEvidenceと異なる場合はSTALE（`NO_EVIDENCE`、診断`STALE`）になる。

### DS-1389

`revision.commit`を特定できないEvidence、および現在のHEAD revisionと一致しないEvidenceは`NO_EVIDENCE`（診断`STALE`）になり、FAILまたは有効なPASSとして扱わない。

### DS-1390

Execution State subjectはrunner / toolchain / 実行影響configと、実行可能状態を変えうるrepository / local dependency入力の完全なmanifestを束縛する。

### DS-1391

Testと宣言targetを変更せずtarget外helperだけを変更しても既存Evidenceは`NO_EVIDENCE`（診断`STALE`）になる。

### DS-1392

EvidenceがExecution State subjectを欠く互換recordなら`NO_EVIDENCE`（診断`STALE`）になり、PASSにならない。

### DS-1393

recordのsnapshotまたは現在snapshotの完全性を証明できなければ`UNKNOWN`となり、PASSにならない。

### DS-1394

Evidenceが無効（STALE / MISMATCH / UNKNOWN）なら`target_binding`へ同じ非PASSを伝播し、無効Evidenceのresultまたはcoverageを再利用しない。

### DS-1395

Evidenceなしでは`target_binding`は`NO_EVIDENCE`（診断`NOT_EXECUTED`）になる。

### DS-1396

単数互換形のEvidenceは、現在のTestがtargetをちょうど1件持つ場合だけ有効性を評価できる。

### DS-1397

複数target Testでは有効なPASSにしない。

### DS-1398

Evidenceのadapter IDがTest execution adapterと異なる場合はMISMATCHになる。

### DS-1399

有効なEvidenceについて、`result: FAIL`（テストランナーが失敗を報告）なら`target_binding`は`FAIL`になる。

*導出元: REQ-094, REQ-095, REQ-096, REQ-097, REQ-098, REQ-099, REQ-100, REQ-101, REQ-102, REQ-103, REQ-104*

*引用: 本冊 §11.2, 要件定義 §5.3*

### DS-1400

有効なEvidenceについて、`result: PASS`かつ全宣言targetの到達要件が§18.3.3 / §18.3.5で充足されれば`target_binding`は`PASS`になる。

### DS-1401

有効なEvidenceについて、`result: PASS`だが到達未充足targetがあれば、当該targetの`target_coverage`のcount 0は`target_binding`を`FAIL`（診断`NOT_EXECUTED`）にする。

### DS-1402

有効なEvidenceについて、`result: PASS`だが到達未充足targetがあれば、当該targetの`target_coverage`が計測不能・未計測（`checked: false`）は`target_binding`を`NO_EVIDENCE`（診断`NOT_CHECKED`）にする。

### DS-1403

有効なEvidenceについて、`result: PASS`だが到達未充足targetがあれば、当該targetの関数不見当は`target_binding`を`UNKNOWN`にする。

#### DS-S153 18.3.5 target_binding 動的計測（per-target）

*導出元: SPEC-S020, SPEC-S051, SPEC-S086*

### DS-1404

各宣言targetについて、計測countが1以上ならtarget別PASSになる。

### DS-1405

各宣言targetについて、計測countが0ならtarget別FAILになる。

### DS-1406

各宣言targetについて、確実に同定または計測できなければtarget別UNKNOWNになる。

### DS-1407

複数target Testの集約値は、1件でもtarget別FAILがあればFAILになる。

### DS-1408

複数target Testの集約値は、FAILがなく1件でもUNKNOWNがあればUNKNOWNになる。

### DS-1409

複数target Testの集約値は、1件以上の全宣言targetがPASSの場合だけPASSになる。

### DS-1410

target AがPASSでもtarget BがFAILまたはUNKNOWNなら、Test単位の`target_binding`をPASSにしない。

### DS-1411

`target_coverage.checked: true`のEvidenceでtarget別entryが欠落、重複、または解決後のcanonical Source Target集合と不一致ならPASSにしない。

### DS-1412

target別entryは解決後のcanonical Locatorをidentityとし、宣言側の綴りを用いない。

*引用: 本冊 §6.1.1*

### DS-1413

coverage capabilityまたは計測toolが利用できない場合は`NO_EVIDENCE`（診断`NOT_CHECKED`）となり、PASSにならない。

### DS-1414

coverage解析限界は`UNKNOWN`となり、PASSにならない。

### DS-1415

Testが別プロセス（起動したsubprocess）・別スレッドでtargetを実行する場合、coverage計測が当該境界越しの実行を宣言targetへ帰属できればtarget別PASS（count > 0）になる。

### DS-1416

target別PASS（count > 0）という結果は本冊§7.3のruntime到達証明としても機能する。

*引用: 本冊 §7.3*

### DS-1417

providerが境界越しの実行を帰属できなければtarget別UNKNOWNとなり、PASSにならない。

### DS-1418

計測不能ならTestの`target_coverage`を`checked: false`（`NO_EVIDENCE`、診断`NOT_CHECKED`）とし、PASSにならない。

#### DS-S154 18.3.6 判断記録プロトコル（非ゲート）

*導出元: REQ-S035, SPEC-S031, SPEC-S034, SPEC-S087*

### DS-1419

`vtest audit submit` の判断は少なくとも actor / subject / decision / judgment_kind を含み、理由・根拠（`reason` / `exclusions`）と `supersedes` は任意（optional）とする。

### DS-1420

submit は、bundle_id のバンドルが存在する（E-AUDIT-001）ことを順に検証し、失敗は§17のエラーコードで拒否する。

### DS-1421

submit は、subject がバンドルと一致する（E-AUDIT-003）ことを順に検証し、失敗は§17のエラーコードで拒否する。

### DS-1422

submit は、judgment_kind がバンドルと一致し値域内である（E-AUDIT-003）ことを順に検証し、失敗は§17のエラーコードで拒否する。

### DS-1423

submit は、バンドル記録時の各対象の内容ハッシュが現在と一致する（E-AUDIT-002）ことを順に検証し、失敗は§17のエラーコードで拒否する。

### DS-1424

submit は、decision が受理する判断値である（E-AUDIT-004）ことを順に検証し、失敗は§17のエラーコードで拒否する。

### DS-1425

submit は、supersedes の各 ULID が同一 subject かつ同一 judgment_kind の既存判断記録を指し自己参照でない（E-AUDIT-008）ことを順に検証し、失敗は§17のエラーコードで拒否する。

### DS-1426

理由が空であることだけを根拠に判断を無効・`UNKNOWN`・`NO_EVIDENCE`・`MISMATCH` 等として扱わない。

*導出元: REQ-180, REQ-181, REQ-182, REQ-183, REQ-184, REQ-185, REQ-186, REQ-187, REQ-188, REQ-189, REQ-190, REQ-191, REQ-192, SPEC-116, SPEC-117, SPEC-118, SPEC-119, SPEC-120, SPEC-121, SPEC-122, SPEC-333, SPEC-334, SPEC-335*

*引用: 基本仕様 §11.3, 要件定義 §12*

### DS-1427

受理された提出は判断記録として `.verify/decisions/` へ保存され、バンドル生成時の全対象の内容ハッシュを `subject_hash` と `dependencies` として記録し、依存 closure のハッシュへ束縛する。

### DS-1428

判断記録の受理は当該対象の検証状態（§4.1 の 5 状態）を昇格させない。

### DS-1429

判断記録の有効性は判定時に評価し、subject が一致し `subject_hash` が現在の内容ハッシュと一致し、`dependencies` が現在の上流依存closureとentity・hashとも完全一致する場合だけ有効とする。

### DS-1430

document は登録 content_hash と実ファイルの一致も要求し、不一致の document を STALE とし、依存する判断記録も無効とする。

*引用: 本冊 §8.5・§11.4*

### DS-1431

同一対象に有効な判断記録が複数あってよい（再判断・多重判断）。

### DS-1432

判断バンドルは Test が宣言した cases 集合を規範項目として含む。

### DS-1433

`@vtest.case` 宣言の正規化文字列を宣言順に並べた list として出力する。

*導出元: SPEC-129, SPEC-130, SPEC-131, SPEC-132*

*引用: 本冊 §8.1・§8.2, 基本仕様 §14*

### DS-1434

`@vtest.case` を持たない Test でも空 list を明示して項目を省略しない。

### DS-1435

バンドルと判断記録は判断型 `judgment_kind` をちょうど 1 件持つ。

### DS-1436

値域は `test-semantic` / `impl-consistency` / `case-coverage` であり、`subject` の値域は前 2 者が Test ID、`case-coverage` が Test ID または VO ID である。

### DS-1437

表にない組合せの要求ではバンドルを生成せず usage error（終了コード 2）とする。

*引用: 本冊 §8.1, 別紙A §12.2*

### DS-1438

`case-coverage`の未判断・判断結果はいずれの検査の値へも写像せず、集約へ寄与しない。

*引用: 本冊 §11.3*

### DS-1439

外部判断が必要な事実は判断待ち section（`check: null`、`judgment_kind: case-coverage`）としてだけ提示する。

*引用: 本冊 §8.1・§11.7*

### DS-1440

`case-coverage` の判断待ち項目は決定論的に生成する。

### DS-1441

`covers ≥ 1` かつ（`cases ≥ 1` または解決済みの covers 先 VO（レコードが存在する VO。E-SCAN-003 の dangling 参照を除く）のいずれかが `dimensions ≥ 1`）を満たす管理対象 Test ごとにちょうど 1 件生成し、`(当該 Test, case-coverage)` の実効判断が `accepted` の場合にだけ消滅する。

### DS-1442

実効判断が未確定・`rejected`・`deferred` のいずれでも項目は生成され、参照した判断記録 ID を `basis` に載せる。

### DS-1443

実効判断が `accepted` の場合にだけ消滅するという規則は `case-coverage` 型の項目にだけ適用し、検査に由来する `kind: unknown` の項目の生成・消滅は判断記録の有無で変わらない。

*引用: 本冊 §11.7*

### DS-1444

実効判断は `(subject, judgment_kind)` の組ごとに決まる。

### DS-1445

有効判断記録集合から、他の有効判断記録の `supersedes` に名指しされたものを除いた実効集合 E について、E が空なら未確定（`UNKNOWN`）とする。

### DS-1446

実効集合 E の decision 値が全て同一ならその値とする。

### DS-1447

実効集合 E に 2 種以上の decision 値があれば未確定（`UNKNOWN`）かつ W-STORE-004 とする。

*引用: 本冊 §8.5*

### DS-1448

競合は `supersedes` による明示の置き換えでだけ解消する。

### DS-1449

判断記録の新旧（`decided_at` / ULID 順）、`decision` 値の優先順位、記録件数の多寡のいずれも採用規則に用いない。

### DS-1450

競合中の対象について機械がいずれかの判断記録を採用した結果を出力しない。

### DS-1451

提出時、`supersedes` の各 ULID が同一 `subject` かつ同一 `judgment_kind` の既存判断記録を指し自己参照でないことを検証し、違反を E-AUDIT-008 で拒否する。

*引用: 本冊 §8.4*

### DS-1452

`judgment_kind` がバンドルと不一致または値域外の提出は E-AUDIT-003 で拒否する。

### DS-1453

レコード群が互いを名指しして実効集合 E が空になる場合は未確定（`UNKNOWN`）とし W-STORE-005 を出す。

*引用: 本冊 §8.5*

### DS-1454

いずれかのレコードを推測で残さない。

### DS-1455

`judgment_kind` を欠くか値域外の判断記録は履歴表示だけを許可し、いずれの実効判断へも寄与させず W-STORE-003 を出す。

*引用: 本冊 §3.4・§8.5*

### DS-1456

実効判断が未確定であることは検証状態（§4.1 の 5 状態）を変更せず、`UNKNOWN` に §4.2 の診断ラベルを付与しない。

### DS-1457

未確定の事実は判断待ち section としてだけ提示する。

*引用: 本冊 §8.5・§11.7*

### DS-1458

仕様・VO・Test 等が変更された場合、過去の判断を現在状態へそのまま流用せず、現在状態に対して §5 の 4 検査を再実施する。

*導出元: REQ-180, REQ-181, REQ-182, REQ-183, REQ-184, REQ-185, REQ-186, REQ-187, REQ-188, REQ-189, REQ-190, REQ-191, REQ-192, SPEC-116, SPEC-117, SPEC-118, SPEC-119, SPEC-120, SPEC-121, SPEC-122, SPEC-333, SPEC-334, SPEC-335*

*引用: 基本仕様 §11.3, 要件定義 §12*

### DS-1459

判断対象の target を一意に解決できない場合はバンドルを生成せず、候補のいずれも選択しない。

*引用: 本冊 §8.1*

### DS-1460

対象が存在しない場合（E-SCAN-004）は `MISMATCH`（診断 `MISSING`）、複数候補により曖昧な場合（E-SCAN-011）は `MISMATCH` とし、両者を一括して同一の状態値にしない。

#### DS-S155 18.3.7 承認と判断記録の分離

*導出元: SPEC-S016, SPEC-S034, SPEC-S043, SPEC-S074, SPEC-S075*

### DS-1461

承認済みを理由に非`PASS`（`FAIL` / `MISMATCH` / `NO_EVIDENCE` / `UNKNOWN`）を`PASS`へ昇格させず、未承認を理由に`PASS`を降格させない。

*導出元: SPEC-051, SPEC-052, SPEC-150, SPEC-151, SPEC-152, SPEC-153, SPEC-154, SPEC-155, SPEC-354, SPEC-355, SPEC-356, SPEC-357*

*引用: 基本仕様 §4.5・§17*

### DS-1462

判断受理も承認も、いずれも検証状態を昇格させない。

### DS-1463

`approved_state` の値域は `approved` / `rejected` / `withdrawn` の 3 値である。

### DS-1464

値域外の値、および値域外の `subject` 種別（判断記録 ULID・Test ID 等）は書込み時に E-APPROVAL-002 で拒否し record を生成しない。

### DS-1465

既存レコードとして読み取った場合は履歴表示だけを許可していかなる実効承認も導出せず W-STORE-006 を出す。

*引用: 本冊 §3.5*

### DS-1466

有効承認レコード集合から、他の有効承認レコードの `supersedes` に名指しされたものを除いた実効集合について、集合が空なら `draft`、`rejected` または `withdrawn` が 1 件以上残るなら `draft`、全件が `approved` なら `approved` とする。

### DS-1467

有効の条件は `approved_state` が値域内であること、対象指定が一致すること、`subject_hash` が現在の内容ハッシュと一致すること、`dependencies` が現在の上流依存closureと entity・hash とも完全一致することである。

*引用: 本冊 §3.5*

### DS-1468

承認取消・却下は実効承認を `draft` へ落とす。

### DS-1469

`approved` の承認レコードが存在しても、後から `withdrawn` または `rejected` の有効承認レコードを追加すると実効承認は `draft` になる。

### DS-1470

機械は `approved` と `rejected` / `withdrawn` のどちらかを新旧・件数で選ばない。

### DS-1471

取消・却下後の再承認は `supersedes` による。

### DS-1472

当該 `withdrawn` / `rejected` レコードの ULID を `supersedes` に名指しした `approved` レコードを追加した場合にだけ `approved` へ戻る。

### DS-1473

名指ししない `approved` の追加では `draft` のままとする。

### DS-1474

`supersedes` の参照先が存在しない・対象が一致しない・自己参照は E-APPROVAL-002、循環は W-STORE-005 とする。

*引用: 本冊 §3.5*

### DS-1475

承認対象の値域は VO ID と document ID である。

### DS-1476

判断記録の承認は `judgment_ref` によってのみ表し、判断記録 ULID を `subject` に置かない。

### DS-1477

`judgment_ref` の参照先が存在しない場合は書込み時に E-APPROVAL-001、読取り時は当該レコードから VO / document の実効承認も判断記録の実効承認も導出せず W-STORE-006 とする。

*引用: 本冊 §3.5*

### DS-1478

判断記録を対象とする実効承認は、当該判断記録が §8.5 の有効判断でありかつ実効集合 E に属する場合にだけ導出する。

### DS-1479

supersede された判断記録・競合により未確定となった判断記録への承認は `draft` 相当とする。

*引用: 本冊 §3.5・§8.5*

### DS-1480

document を対象とする承認の上流依存closureは当該 document の再帰的な上位 document（`derives_from` 先）からなり、`--subject-type document` で記録する。

### DS-1481

document 再登録（`--update`）で document subject hash が変化すると当該承認は失効する。

*引用: 本冊 §3.1・§3.5・§11.4*

### DS-1482

判断記録を対象とする承認は `--subject-type judgment` で記録し、`judgment_ref` へ判断記録 ULID を、`subject` へ当該判断記録の `subject` を書き込む。

*引用: 本冊 §3.5*

### DS-1483

判断記録 ULID を `subject` に置くレコードは生成しない。

### DS-1484

実効承認は明示の `supersedes` 関係だけで決まる。

### DS-1485

`supersedes` 関係にない複数の有効承認レコードはすべて実効集合に属し、`approved_at` / ULID の順序・レコードの新旧・件数の多寡のいずれも採用規則に用いない。

### DS-1486

`approved` と `rejected` が `supersedes` 関係なく併存する対象について、機械がどちらかに確定した結果を出力せず fail-closed に `draft` とする。

*引用: 本冊 §3.5*

### DS-1487

VO を対象とする承認の上流依存closureは、対象 VO の再帰的 parent VO、対象 VO と parent VO が `derives_from` で参照する document、および各 document の再帰的な上位 document からなる。

### DS-1488

document dependency は §1.3 の document subject hash を使用するため、document record または参照先 source の変更で承認が失効する。

*引用: 本冊 §3.5・§11.4*

### DS-1489

実効承認状態の遷移は `draft` と `approved` の 2 値の間でだけ起き、検証状態（§4.1 の 5 状態）の変化・判断記録の追加そのもの・`basis` の内容によっては遷移しない。

*引用: 本冊 §3.5*

### DS-1490

上流依存closureまたはハッシュを欠く互換 Approval は読取りと履歴表示だけを許可し、現在の `approved` を導出しない（W-STORE-002、VO は `draft` 相当）。

#### DS-S156 18.3.8 verify・report と scope

*導出元: SPEC-S017, SPEC-S053, SPEC-S054, SPEC-S055, SPEC-S069, SPEC-S094, SPEC-S096*

### DS-1491

完全検証は基本仕様 §5 の 4 検査（`chain_integrity` / `orphan_detection` / `target_binding` / `oracle_presence`）をすべて評価し、各検査の非PASSを総合NGへ反映する。

*導出元: SPEC-053, SPEC-054, SPEC-055, SPEC-056, SPEC-057, SPEC-058, SPEC-059, SPEC-060, SPEC-061, SPEC-062, SPEC-063, SPEC-064, SPEC-065, SPEC-066, SPEC-067, SPEC-068, SPEC-069, SPEC-287, SPEC-288, SPEC-289, SPEC-290, SPEC-291, SPEC-292, SPEC-293*

*引用: 基本仕様 §5*

### DS-1492

完全検証は、各検査の評価地点（DOC / VO / TEST / repository）で評価した全値がPASSの場合だけOKとする。

### DS-1493

`--items`を省略したCLI / MCP検証は常に固定4検査を評価する。

### DS-1494

version 1 configの`full_scope`欠落は固定4検査へ具体化し、version 1 / version 2 いずれでも旧12項目の列挙（`spec_coverage` / `test_existence` 等）は E-CONFIG-001 で拒否し、in-memory 補完で受理しない。

*引用: 本冊 §2.2*

### DS-1495

version 1 の重複・未知項目、version 2 の欠落・重複・未知・余剰項目も E-CONFIG-001 とし、検証結果を生成しない。

### DS-1496

4検査未満を明示した`--items`だけを限定scopeとして扱い、「完全検証」と表示しない。

### DS-1497

検査軸（4 本の部分集合）とエンティティ軸（対象とする document / VO / Test の部分木）を指定でき、限定scopeのOKは「要求scope内のOK」に限られる。

### DS-1498

いかなる設定値も完全検証の検査を 4 本未満へ縮退させない。

### DS-1499

限定scopeは要求項目だけを集約し、scope外・未実施の項目を `NO_EVIDENCE`（診断 `NOT_CHECKED`）として保持・併記する。

### DS-1500

出力には要求 scope と scope 外項目が未検証である旨を必ず併記する。

### DS-1501

完全検証でも `scope` を省略しない。

### DS-1502

検証結果を返さないコマンド（`init` / `scan` / `doc *` / `vo *` / `test *` / `audit *` / `run`）の JSON は `scope` を持たない。

### DS-1503

限定 scope の JSON 出力だけから、要求 scope と「scope 外は未検証」の旨を判定できる（`scope.unverified_outside_scope` が `true` で、scope 外検査ノードが `NO_EVIDENCE`／診断 `NOT_CHECKED`）。

### DS-1504

親 VO の値は子 VO の値と当該親 VO を直接 covers する Test の値の fail-closed 合成であり、いずれかに非 `PASS` が 1 件でもあれば親 VO は非 `PASS` になる。

### DS-1505

`--vo <親VO>` および `--from <親VO> --direction down` が親 VO の代表値と配下の子 VO・Test の内訳を同一出力で返し、出力に Feature 名・Feature ID の field を含めない。

### DS-1506

要求scope内の `FAIL`・`MISMATCH`・`NO_EVIDENCE`・`UNKNOWN` のいずれも総合PASSへ昇格しない。

### DS-1507

NO_EVIDENCE を生む入力（証拠が存在しない／証拠のハッシュが現在の対象と不一致／scope 限定により検査を実施しなかった項目）を受入で表現する。

### DS-1508

NO_EVIDENCE を生む入力は `NO_EVIDENCE`（診断は順に `NOT_EXECUTED` / `STALE` / `NOT_CHECKED`）となり `PASS` へ変換されない。

*導出元: SPEC-049*

*引用: 基本仕様 §4.3・§4.6*

### DS-1509

完全検証fixtureで4検査のそれぞれを単独で非PASSにすると総合NGになる。

### DS-1510

管理済みgraph側の他検査がすべてPASSでも、未登録Testが1件あれば`chain_integrity`により総合NGになる。

### DS-1511

集約は fail-closed とし、子に 1 つでも非 `PASS` があれば親は非 `PASS`。

### DS-1512

代表値の優先順位は `FAIL > MISMATCH > NO_EVIDENCE > UNKNOWN` とし、診断ラベル（`MISSING` / `NOT_EXECUTED` / `NOT_CHECKED` / `STALE`）は代表値の順位に用いず原因説明として併記する。

*導出元: SPEC-190, SPEC-191*

*引用: 基本仕様 §22.2, 本冊 §11.3*

### DS-1513

report は DOC → VO → Test の構造と、各非PASSの根拠（判断記録・Evidence への参照）を text / JSON で返す。

### DS-1514

`covers` を持つ Test は covers 先 VO の子ノードとして表示する。

### DS-1515

管理下にある事実と、いずれの VO へも寄与しない事実の双方を出力から確認できる。

*導出元: SPEC-192, SPEC-193, SPEC-194, SPEC-195, SPEC-196*

*引用: 基本仕様 §22.3*

### DS-1516

`covers` を持たない Test は §18.3.1 の `chain_integrity = MISMATCH` として扱い、役割別表示を設けない。

### DS-1517

同一 revision・同一 `.verify/` ファイル集合（`config.yaml`・document / VO / Relation レコード・判断記録・承認・Evidence）・同一 scope 指定に対して `verify` を繰り返し実行すると、4 検査の検証状態・診断ラベル・診断コード集合・集約結果・`pending` section・終了コードが毎回一致する。

*導出元: SPEC-109, SPEC-110, SPEC-111, SPEC-112, SPEC-113, SPEC-319, SPEC-320, SPEC-321, SPEC-322, SPEC-323, SPEC-324, SPEC-325, SPEC-326, SPEC-327, SPEC-328, SPEC-329, SPEC-330, SPEC-331, SPEC-332*

*引用: 本冊 §11.1, 基本仕様 §11.1*

### DS-1518

実行時刻・ロケール・タイムゾーン・呼出し元の作業ディレクトリを変えても、また Execution State subject の入力に影響しない環境変数を変えても、上記の出力が変化しない。

*引用: 本冊 §1.3*

### DS-1519

ネットワークを遮断した環境でも同一の出力を返す。

### DS-1520

toolchain identity・adapter config・入力 manifest を変える環境変更（`RUSTUP_TOOLCHAIN` の切替等）の影響は Evidence の鮮度喪失（`NO_EVIDENCE`、診断 `STALE`。本冊 §11.2）としてのみ現れ、環境そのものを判定条件として読む経路を持たない。

*引用: 本冊 §11.2*

### DS-1521

`vtest` は 4 検査の評価中に LLM API を含む外部サービスへ要求を出さない。

### DS-1522

外部 AI／Agent の関与は `.verify/decisions/` の判断記録ファイル経由に限られ、判断記録の受理は検証状態を昇格させない。

### DS-1523

4 検査の評価経路に、実行時に差し替え可能な意味判定 seam を持たない。

### DS-1524

`report --from DOC-X --direction down --format json` は、`derives_from` エッジごとに `from` / `relation` / `to` と当該 entry の `anchor`・`note` を返し、「どの上流条項がどの VO へ対応するか」の対応ペア集合として読める。

*導出元: SPEC-109, SPEC-110, SPEC-111, SPEC-112, SPEC-113, SPEC-319, SPEC-320, SPEC-321, SPEC-322, SPEC-323, SPEC-324, SPEC-325, SPEC-326, SPEC-327, SPEC-328, SPEC-329, SPEC-330, SPEC-331, SPEC-332*

*引用: 本冊 §11.6・§3.1・§3.2, 基本仕様 §11.1*

### DS-1525

`anchor` を持たない entry では `anchor` を省略または `null` とし、空文字列で埋めない。

#### DS-S157 18.3.9 フェーズゲート評価

*導出元: REQ-S057, SPEC-S012, SPEC-S016, SPEC-S050, SPEC-S069, SPEC-S098, SPEC-S105, SPEC-S106*

### DS-1526

`vtest verify --gate <name>` は、指定ゲートの対象 scope について検証を実行し、(1) 検証結果が `require.verification` を満たすか、(2) `require.approvals` の各ロールについて対象の有効な承認が存在するか、を評価して満否と根拠（不足している非 `PASS` 検査・未充足の承認ロール）を提示する。

### DS-1527

承認済みを理由に検証状態を昇格させない。

### DS-1528

`require.verification` の値域を config 受理時に検査する。

### DS-1529

5 状態語彙（`PASS` / `FAIL` / `MISMATCH` / `NO_EVIDENCE` / `UNKNOWN`）との完全一致は受理する。

### DS-1530

診断ラベル（`MISSING` / `NOT_EXECUTED` / `NOT_CHECKED` / `STALE`）・`OK` / `NG`・小文字表記・旧12項目名・非文字列値は E-CONFIG-001・終了コード 2 で拒否して検証結果を生成しない。

### DS-1531

`require` および `require.verification` の欠落、`gates[].name` の重複も E-CONFIG-001 とする。

*引用: 本冊 §2.2*

### DS-1532

`require.approvals` の省略と `gates` field 自体の欠落・空 list は受理する。

### DS-1533

ゲートの検証条件は `require.verification` と要求 scope の集約代表値の完全一致でのみ充足する。

### DS-1534

`require.verification` に `PASS` 以外（例 `UNKNOWN`）を定義したゲートは、代表値が同じ値のときだけ充足し、代表値が `PASS` のときは充足しない。

### DS-1535

逆に `require.verification: PASS` のゲートは代表値が非 `PASS` のとき充足しない。

### DS-1536

集約代表値は構造検査（`chain_integrity` / `orphan_detection`）を含む要求 scope 内の全評価値の fail-closed 合成であり、エンティティ軸の部分木が全 `PASS` でも構造検査が非 `PASS` なら代表値は非 `PASS` になる。

### DS-1537

`--items` で検査軸を限定した実行では scope 外検査が `NO_EVIDENCE`（診断 `NOT_CHECKED`）として代表値に参加するため、`require.verification: PASS` のゲートは限定 scope で充足しない。

### DS-1538

`require.approvals` が空集合なら `approvals` は空 list、`gate.satisfied` は `verification.satisfied` と全 `approvals[].satisfied` の論理積になる。

### DS-1539

`--gate` 指定時の最上位 `ok` と終了コードはゲート充足で決まる（充足 → `ok: true`・0、不充足 → `ok: false`・1）。

### DS-1540

`require.verification` に `PASS` 以外を定義したゲートが充足した実行は、総合が NG でも終了コード 0 になる。

### DS-1541

config の `gates` に定義の無いゲート名を `verify --gate` / `report --gate` / MCP の `gate` 入力へ指定すると、E-CONFIG-002・`ok: false`・終了コード 2 で拒否し、検証もゲート評価も実行せず部分結果を返さない。

### DS-1542

診断には指定名と定義済みゲート名の一覧を含み、MCP tool error は `candidates` に定義済みゲート名を持つ。

### DS-1543

`gates` が空・未定義の状態での指定も同じ扱いとする。

### DS-1544

ゲート名の解決は大文字小文字を区別した完全一致だけで行い、前方一致・部分一致・近似一致・既定ゲートへの代替で受理しない。

#### DS-S158 18.3.10 Structured Test Operation

*導出元: SPEC-S038, SPEC-S078*

### DS-1545

Form `kind`は`[a-z0-9][a-z0-9-]*`のcase-sensitive文字列で、built-inとuser-defined schemaを通してrepository全体で一意であり、schemaはowner `adapter` IDを別fieldで宣言する。

### DS-1546

registryのkind owner、schemaのadapter、Structured Test capabilityが一意に一致する場合だけcreate / form_getを許可する。

### DS-1547

同じkindを複数adapterが宣言する、schemaとregistry ownerが不一致、adapterが未知、またはcapabilityがない場合は操作を拒否し、ファイルを変更しない。

### DS-1548

`adapter`を欠く読取り互換Formは、登録済みStructured Test adapterのbuilt-in kind宣言またはschema compatibility matcherのうちちょうど1件だけがschemaを受理する場合に限って解決し、曖昧またはowner不在なら拒否する。

### DS-1549

matcherはschema内容から決定論的に判定し、coreは未知kindを`rust-cargo`へfallbackしない。

### DS-1550

Form Schemaの必須値と未知fieldを常に検証する。

### DS-1551

symbol、VO / Test参照、identifier、pathは選択したFormが該当fieldとvalidatorを宣言した場合だけ検証し、すべてのadapterへ一律に要求しない。

### DS-1552

create結果はscanで同じTest ID・intent・covers・targetsとして認識される。

### DS-1553

editは1 Testの拡張rangeだけを単一置換し、他Testと通常sourceを変更しない。

### DS-1554

同じdesired stateの再適用は冪等になる。

### DS-1555

Structured Test capabilityがないadapterへのcreate / editはE-ADAPTER-004となり、ファイルを変更しない。

### DS-1556

edit と同じ確認項目を create でも実施し、create 経路にだけ検証を省く分岐を設けない。

### DS-1557

挿入後の再パースが構文エラーになる fixture、挿入結果の annotation が desired state と一致しない fixture、挿入が他の Test 範囲へ及ぶ fixture のそれぞれで、create は E-OP-003・終了コード 2 になり、対象ファイルが挿入前のバイト列へ復元される。

### DS-1558

挿入によりファイルが新規作成されていた場合は不存在へ戻る。

### DS-1559

ロールバック後に scan すると、当該 create 操作が無かった場合と同一のエンティティ集合・内容ハッシュが得られる。

### DS-1560

部分適用された挿入内容・採番された Test ID・Evidence・判断記録がいずれも残らない。

### DS-1561

`create --dry-run` は挿入内容と挿入位置を提示し、ファイルを変更しない。

### DS-1562

同一 desired state からの create と、その直後の同一 desired state による edit は差分を生じない（annotation block の再生成規則が create / edit で同一。別紙A §15.3）。

*引用: 別紙A §15.3*

#### DS-S159 18.3.11 MCP interface

*導出元: SPEC-S062, SPEC-S101*

### DS-1563

別紙A（§12〜§15）が定める全 MCP tool が同じ入力に対するCLI JSONと同じdata / diagnosticsを返す。

### DS-1564

不正入力はcode / message / candidatesを持つtool errorになる。

### DS-1565

request、notification、batch、malformed transportの各入力をJSON-RPC contractどおりに処理する。

### DS-1566

MCP serverの長時間実行中もsource変更を再scanし、staleなPASSを保持しない。

#### DS-S160 18.3.12 adapter contract

*導出元: SPEC-S063, SPEC-S078, SPEC-S105*

### DS-1567

`TestEntity.content_hash`はTest constructだけでなくcanonical metadata、locationのadapter・path・opaque locator、ExecutionDescriptorを含むTest subjectへ束縛される。

### DS-1568

byte range自体は含めず、非隣接metadataだけの意味変更でもhashが変化する。

### DS-1569

registryはadapter IDの重複、宣言capabilityと実装の不一致、未登録adapterを拒否する。

### DS-1570

異なるadapterが同じrootを共有でき、同一adapter内のroot重複は拒否される。

### DS-1571

全adapterのmerge結果でTest IDのglobal uniquenessを検査する。

### DS-1572

config readerはversion 1とversion 2を受理し、読み取りだけでconfigを書き換えない。

### DS-1573

config writerと`vtest init`はversion 2のadapter namespaceを出力する。

### DS-1574

Test JSON writerは`execution`を常に出力し、`rust-cargo` Testについてだけwire codecが互換field `filter` / `package` / `test_target`を追加する。

### DS-1575

Test JSON writerは1件以上の`targets` listを常に出力し、targetが1件の場合だけ同値の単数互換field`target`を追加できる。

### DS-1576

複数targetを単数fieldへ縮約しない。

### DS-1577

synthetic TestのJSONはRust互換fieldを省略し、空値またはdummy値を出力しない。

### DS-1578

`execution`を欠くTest入力は、`rust-cargo` codecが完全で相互整合するRust互換fieldからだけdescriptorを導出する。

### DS-1579

`execution`とRust互換fieldが矛盾する入力を拒否する。

### DS-1580

明示操作に必須のcapabilityがなければE-ADAPTER-004となり、変更・判断記録・Evidenceを生成しない。

### DS-1581

検証時のstatic audit / coverage capability欠落は`NO_EVIDENCE`（診断`NOT_CHECKED`）になる。

### DS-1582

検証時のrunner欠落は`NO_EVIDENCE`（診断`NOT_EXECUTED`）になる。

### DS-1583

検証時の解析限界は`UNKNOWN`になる。

### DS-1584

synthetic adapterは`.rs`以外のsource、関数ではないTest construct、doc commentではないmetadata宣言、Rust item pathではないopaque locatorを、`vtest-model`、`vtest-scan`、`vtest-verify`の変更なしで登録・scan・verifyできる。
