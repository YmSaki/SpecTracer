# 詳細設計 本冊 §6–§19 層仕分け結果（det2）

対象: `docs/canonical/specification.json` の `design` 配列のうち、`source.doc` が詳細設計 本冊で `source.lines[0] >= 949` の全文（DES-701〜DES-1253、553文）。本冊の§6（Target Reference解決）・§7（Static Analysis orchestrationと`rust-cargo`ルール）・§8（判断記録プロトコル）・§9（テスト実行設計）・§10（`rust-cargo` Target Binding 動的計測）・§11（鮮度検証と集約）・§16（並列動作と整合性）・§17（診断・終了コード体系）・§19（実装選択と提供範囲）にあたる。

判定基準: `docs/canonical/LAYERING.md` §2（判定1→1a→2→3、六層版）。553文を5グループ（担当範囲: §6/§9/§10、§7、§8/§16、§11、§17/§19）に分けて並列に仕分けたのち、本書で節番号順に統合した。統合にあたり文の再判定は行っていない（各グループの判定結果をそのまま採用）。§17については課題文が与えた個別指針（「コードの外部的意味＝detailed_spec、内部伝播規則＝design」）を各グループ共通の道具として適用した。

## (a) 節ごとの既定の層

| 節 | 既定の層 | 文数 | 例外数 | 理由 |
|---|---|---:|---:|---|
| 6.1 adapter-neutral解決contract | detailed_spec | 26 | 11 | 大半が「特定条件→エラーコード/振る舞い」の精密な約束。例外はadapter/core分界（basic_design）と型・データフロー詳細（design）。 |
| 6.1.1 target identityの一方向確定 | detailed_spec | 7 | 0 | identity解決の一方向性・一貫性という外部観測可能な精密な不変条件のみで構成。 |
| 6.2 `rust-cargo` locator解決 | design | 4 | 0 | rust-cargo固有の解決アルゴリズム契約。うち3文は番号付き手順（description中の1.2.3.）でcode_like。 |
| 6.3 候補提示 | detailed_spec | 6 | 3 | 失敗条件下の候補表示という精密な振る舞いが中心。候補構築順序の2文とAST検索の1文はdesign（うち2文code_like）。 |
| 7.（導入） | detailed_spec | 5 | 3 | 証拠源→検査の対応づけが中心で精密寄りだが、内訳はdetailed_spec 2・spec 2・basic_design 1の同数。出現順で先着のdetailed_specを既定とした。 |
| 7.1 判定の原則 | detailed_spec | 23 | 7 | FAIL/UNKNOWN/PASSの状態割当・優先順位・合成規則が精密に列挙される一方、StaticAnalysisAdapterとの入出力契約とcore/adapter境界がdesign/basic_designの塊で例外化。 |
| 7.2 `rust-cargo` ルール一覧（本文） | detailed_spec | 19 | 2 | assert相当構文の分類・データフロー境界・集約規則はすべて精密な条件→結果の規定。 |
| 7.2 `rust-cargo` ルール一覧 / ルール表 | detailed_spec | 7 | 0 | DA-001〜006・W-DA-101の内容・FAIL条件・UNKNOWN退避例を定める検査条件表そのもの。 |
| 7.2.1 照合の委譲先の終端（本文） | detailed_spec | 11 | 0 | 委譲先の定義・終端の必要十分条件・循環時の扱いなど、いずれも精密な条件規定。 |
| 7.2.1 照合の委譲先の終端 / 判定表 | detailed_spec | 6 | 0 | 委譲先の状態ごとにDA-003・DA-006の値を定める決定表。 |
| 7.3 target到達の静的証明とruntime証明の関係 | detailed_spec | 40 | 5 | 到達要件・静的/runtime到達の定義・target_binding状態割当が精密に列挙される。 |
| 8. 判断記録プロトコル（冒頭） | spec | 4 | 0 | 判断記録が検証状態のゲートでないという一般的な領域境界の約束が中心。 |
| 8.1 バンドル生成 | design | 30 | 16 | バンドルが持つ情報一式（対象VO・Test Intent・cases集合・対象実装等）をfieldとして列挙する記述が最多。 |
| 8.2 バンドル JSON スキーマ（例） | design | 1 | 0 | JSONスキーマのfield列挙（`SourceLocation`型宣言と同型）。 |
| 8.3 提出スキーマ | detailed_spec | 13 | 5 | 必須/任意field・値域・整合性条件など受理条件に関わる精密な規定が最多。 |
| 8.4 提出の検証 | detailed_spec | 12 | 4 | 各検証項目をE-AUDIT-*エラーコードへ精密に対応付ける記述が最多。 |
| 8.5 有効性と再判断 | detailed_spec | 27 | 10 | 有効性の定義・実効判断(V→E→効果値)の解決規則という精密な条件→結果対応が最多。 |
| 8.6 参考プロンプト | spec | 3 | 0 | 判断記録の一般的な領域境界（ゲートでない・昇格させない）の再確認。 |
| 9.1 実行対象の解決 | detailed_spec | 2 | 1 | `--test`/`--vo`/`--all`の展開規則が中心。`--req`廃止はCLI構成の決定でbasic_design。 |
| 9.2 `rust-cargo` TestRunnerAdapter | design | 11 | 3 | TestEntity各フィールドのCargo意味への解釈というadapter自身の契約が中心。 |
| 9.3 `rust-cargo` 結果のパース | design | 10 | 4 | stdout文字列の意味付け（出力形式依存でcode_like）が中心。 |
| 9.4 Evidence の記録 | detailed_spec | 14 | 3 | Evidence生成条件・状態割当という精密な規則が中心。 |
| 10.1 計測方式 | detailed_spec | 13 | 7 | 計測不能時の状態割当が中心。技術選定・capability境界はbasic_design、手法/コマンドはdesign。 |
| 10.2 判定 | detailed_spec | 18 | 6 | count→PASS/FAIL/UNKNOWNの割当・集約優先順位が中心。 |
| 10.3 実行モードの整理 | detailed_spec | 3 | 1 | モード別の精密な振る舞いが中心。 |
| 11.1 検査の評価地点 | detailed_spec | 30 | 12 | 4検査それぞれのPASS/MISMATCH等の条件をフィールド単位・エラーコード単位で精密に規定する文が過半。 |
| 11.1.1 `chain_integrity` の評価 | detailed_spec | 17 | 2 | 文書層/VO層/Test層ごとの評価条件とE-SCAN-*コードの対応を精密に列挙する文が大半。 |
| 11.2 Evidence 鮮度判定 | detailed_spec | 23 | 0 | ハッシュ等値・commit一致・execution_state完全性などの条件と状態の対応を精密に規定する文のみ。 |
| 11.3 集約アルゴリズム | detailed_spec | 32 | 9 | leaf VO/親VO/DOCの合成規則・優先順位・OK/NG条件など集約契約の精密な規定が中心。 |
| 11.4 document 鮮度 | detailed_spec | 5 | 0 | content_hash不一致時の警告コード・状態伝播・失効条件を精密に規定する文のみ。 |
| 11.5 フェーズゲート評価 | detailed_spec | 25 | 9 | `--gate`解決・充足判定・優先順位などの精密な条件規定が中心。 |
| 11.6 役割別 projection | spec | 15 | 9 | 役割別preset提供・trace可能性など機能レベルの約束が最頻。 |
| 11.7 判断待ち情報の構造 | design | 21 | 12 | `subject`/`kind`/`check`等の出力レコードのフィールド型・値域宣言が最頻（detailed_specと9件で同数、初出順の慣例によりdesignを既定とした）。 |
| 16.（冒頭） | spec | 1 | 0 | 別紙Aへの委譲というメタな約束。 |
| 16.1 ロック不要の根拠 | basic_design | 10 | 3 | 書込み操作の分類・原子的公開という並行処理機構の構造決定が最多（基本仕様§24.2の具体化）。 |
| 16.2 意味的衝突検出 | detailed_spec | 6 | 1 | 各不整合種別をE-SCAN-*エラーコードへ対応付ける記述が最多。 |
| 17.1 診断コード（診断コード表含む） | detailed_spec | 27 | 2 | 表の各行は「コードXはerror/warningであり条件Yを意味し結果Zとする」という、コードの外部的意味の精密な条件→結果対応。 |
| 17.2 終了コード | detailed_spec | 15 | 3 | 終了コード0/1/2/3・ゲート充足・優先順位の各文は条件と終了コードを精密に対応付ける。 |
| 19. 実装選択と提供範囲 | spec | 11 | 0 | 「実装が選択できる」「提供範囲外とする」はどちらも実装方法によらず真であり続ける約束（基本仕様§29・§30と同型）。 |

## (b) 集計

| 層 | 件数 | 割合 |
|---|---:|---:|
| spec | 90 | 16.3% |
| detailed_spec | 349 | 63.1% |
| basic_design | 35 | 6.3% |
| design | 79 | 14.3% |
| **合計** | **553** | 100.0% |

`confidence: low` は **35件**（30件はやや上回るが40件は下回るため規則の再定義は不要と判断した）。内訳: §6/§9/§10グループ10件（DES-704, 705, 710, 946, 955, 967, 969, 996, 1006, 1008）、§7グループ8件（DES-744, 745, 746, 748, 790, 802, 803, 833）、§8/§16グループ7件（DES-874, 895, 901, 935, 937, 1189, 1191）、§11グループ5件（DES-1089, 1153, 1159, 1164, 1168）、§17/§19グループ5件（DES-1238, 1243, 1244, 1245, 1246）。

`code_like: true` は **14件**、すべて§6.2〜6.3・§9.2〜9.4・§10.1の担当グループに集中し、他4グループ（§7・§8/§16・§11・§17/§19、計442文）は0件だった。

### low-confidence の共通パターン（規則の下で決まらない境界、再定義提案ではなく記録）

- **「実装者が別の作り方を選んでも真」という判定1の litmus と、「基本仕様＝システム概要」案との緊張**（DES-746, 790, 833 = 正典レコードを持たず検証のたびに再計算する、の3変奏）。基本仕様パイロット（`base-report.md` §24.3）の先例に倣いspecとしたが、LAYERING.md §2末尾（L50）が明言する未解決の緊張そのものの再現。
- **「実装が選択できる／委譲する」という文型**（DES-1243〜1246, 1238）。ルール1の litmus を機械的に当てると約束（spec）側に落ちるが、1aには「実装裁量の委譲」を受け止める区分がなく、`LAYERING-reference.md`は同種項目を詳細設計（design）の構成要素として明示している。基本仕様§30（既存パイロットで26/26 spec）との構造的類似からspecとしたが、これは類推であり規則自体が「委譲の対象が次の文書層か実装コードか」を区別していないことに由来する。
- **判定範囲の不変条件か内部依存先を縛る決定か**（DES-802, 803 = 終端判定がcovers/宣言targetグラフのみを参照する、専用の信頼記録を設けない）。外部から観測可能な判定範囲の約束（detailed_spec）とも、判定機構の内部依存先を縛る決定（design）とも読める。
- **フィールド値表現・内部データ構造選択がdetailed_specかdesignか**（DES-1089, 1153, 1159, 1164, 1168、および DES-895, 901, 935, 937）。「既存機構への対応付け」「フィールド値の省略/null表現」のような細部は、外部的に観測可能な振る舞いの約束と部品の内部契約のどちらにも読める。

## (c) 例外一覧（節の既定層と異なる文）

節番号順、節内はID順。各グループが個別に作成した例外一覧を統合したもので、統合にあたり再判定は行っていない。

### §6（Target Reference解決）

| ID | 文（先頭60字） | 層 | 理由 | 確信度 |
|---|---|---|---|---|
| DES-701 | coreは`TargetRef::Locator.adapter`をregistryで解決する。 | basic_design | adapter選択の構成メカニズム（registry）に関する構造決定。 | high |
| DES-702 | coreは、opaque locatorの解釈を該当する`SourceDiscoveryAdapter`へ委譲する。 | basic_design | core/adapterの責務分界。 | high |
| DES-703 | adapterは正規化されたTarget Reference、Source Location、source rangeを返す。 | design | adapter返却値の構成要素を列挙するインターフェース契約。 | high |
| DES-704 | coreは、返却されたadapter IDとTarget Referenceの一致、source rangeの範囲…を検証する。 | design | coreの事後条件契約（約束か決定かの境界が微妙）。 | low |
| DES-705 | coreは§1.3のSource Target hashを計算する。 | design | core内データ所有権の記述で境界主張ではない。 | low |
| DES-706 | coreはopaque locatorの内部構文は解釈しない。 | basic_design | core/adapter分界の正典的表現。 | high |
| DES-709 | SRC ID参照はcoreが統合済みSRC索引で一意性を検査する。 | basic_design | 統合索引インフラを介した検査のcore側割当。 | high |
| DES-710 | SRC ID参照は対応するadapterのSource Locationとsource rangeを使用する。 | design | adapter側フィールドを用いるデータフロー詳細。 | low |
| DES-716 | 解決結果は「解決済み」「対象なし」「曖昧」の3状態を区別する。 | spec | 状態の個数を定める語彙宣言（基本仕様§4.1と同型）。 | high |
| DES-724 | Target Referenceの解決はcoreの単一経路が所有し、静的解析、実行、Evidence writer…が消費する。 | basic_design | 単一経路所有・他部品は消費者という構造上の責務分界。 | high |
| DES-725 | 各subsystemが独自にcandidate列を走査して1件を選ぶ経路を持ってはならない。 | basic_design | 重複経路の禁止という構造決定。 | high |
| DES-734 | `rust-cargo`のlocator`path::item-path`の解決は、§5.5で構築したSRC索引を用いる。 | design | adapter固有の解決アルゴリズム契約。 | high |
| DES-739 | `rust-cargo` adapterは、item-pathの末尾セグメント一致（別パスの同名関数）の順で候補を構築する。 | design | 候補構築の優先順位を定める番号付き手順の一部（code_like）。 | high |
| DES-740 | `rust-cargo` adapterは、編集距離2以内の近似名の順で候補を構築する。 | design | 同じ手順の2番目のステップ（code_like）。 | high |
| DES-741 | `rust-cargo` adapterのenum variant検証（`expect`の値が`ParseError::InvalidUtf8`…）はAST検索を用いる。 | design | AST検索を用いるadapter固有の技術契約。 | high |

### §7（Static Analysis orchestrationと`rust-cargo`ルール）

| ID | 文（先頭60字） | 層 | 理由 | 確信度 |
|---|---|---|---|---|
| DES-746 | 決定論的解析結果は正典レコードを持たず、検証のたびに現在のsource / configから再計算する派生情報である。 | spec | 正典レコードを持たず再計算するという一般約束。base-report §24.3先例に倣いspec。 | low |
| DES-747 | `vtest audit static`は要求時に解析を起動し、結果をstdoutと`cache/`へ出力する。 | basic_design | `cache/`という具体パスへの出力先固定＝永続化方式の決定。 | high |
| DES-748 | `vtest audit static`は判断記録（§8）とは別機構であり、外部判断の記録には転用しない。 | spec | 別機構混同を禁じる主要挙動の宣言（部品境界とも読める境界事例）。 | low |
| DES-763 | `vtest-audit`は`TestEntity.execution.adapter`をregistryで解決する。 | design | vtest-audit自身が依存するインターフェース（registry）の契約。 | high |
| DES-764 | `vtest-audit`は、Test、全Target Reference、各source range、cur…をadapterへ渡す。 | design | StaticAnalysisAdapterへ渡す入力データの型契約。 | high |
| DES-765 | adapterはrule ID、verdict、根拠span、解析限界を返す。 | design | adapter戻り値の型契約。 | high |
| DES-766 | target-scopedなDA-002 / DA-003については、宣言targetごとのverdictを畳み込む前の形で返す。 | design | 畳み込み前のデータ粒度契約（手順順序ではなくデータ形状）。 | high |
| DES-767 | target-scopedなDA-002 / DA-003の集合を全宣言targetと過不足なく1対1に対応させる。 | design | interfaceデータの完全性についての契約。 | high |
| DES-768 | coreはadapter ID、source location・現在bytesとの対応、決定論的encodingを検証し集約する。 | design | coreが能動的に行う検証・集約という自コンポーネント契約。 | high |
| DES-769 | adapter固有のASTやassertion構文をcoreは解釈しない。 | basic_design | adapterとcoreの責務分界そのもの（LAYERING.md §2の例示に合致）。 | high |
| DES-790 | 静的解析は再計算派生であるため、target別verdictと規則単位verdictは検証のたびに再計算する。 | spec | 746と同型の再計算約束。base-report §24.3先例に倣いspec。 | low |
| DES-797 | ルールごとの判定結果と根拠（該当スパン）は`vtest audit static`の出力および`cache/`へ記録する。 | basic_design | 747と同型の`cache/`出力先固定＝永続化方式の決定。 | high |
| DES-802 | 終端の判定はcovers / 宣言targetのグラフの参照だけで行う。 | detailed_spec | 判定範囲の不変条件。約束/決定の境界が割れる境界事例。 | low |
| DES-803 | 終端の判定について、信頼を宣言する専用の記録・注釈・設定項目は設けない。 | detailed_spec | 専用記録を設けない禁止の不変条件。802と同様の境界事例。 | low |
| DES-833 | `target_binding`項目値は検証時に算出する。 | spec | 746・790と同型の再計算約束。base-report §24.3先例に倣いspec。 | low |
| DES-844 | 本節の到達要件は検証対象をSource Targetとして実現する形態に限定する。 | spec | 本節の適用範囲を限定する宣言（base-report §27・§29相当）。 | high |
| DES-846 | 検証対象をSource Targetとして宣言しない他の実行形態の確認方法は下位仕様・後続版へ委譲する。 | spec | 確認方法を下位仕様・後続版へ委譲する範囲宣言（base-report §30相当）。 | high |
| DES-847 | 本節のtarget実行到達規則を普遍規則として適用しない。 | spec | 844・846と同型の適用範囲宣言。 | high |
| DES-854 | 本節はprocess boundaryによってDA-002到達が恒久UNKNOWNになる問題だけを解消する。 | spec | 844・846・847と同型の適用範囲宣言。 | high |

### §8（判断記録プロトコル）・§16（並列動作と整合性）

| ID | 文（先頭60字） | 層 | 理由 | 確信度 |
|---|---|---|---|---|
| DES-859 | `vtest audit bundle`は判断対象ごとに情報をJSONとして`cache/bundles/<ULID>`へ出力する。 | basic_design | バンドル出力先パスという永続化方式の決定。 | high |
| DES-860 | バンドルは派生情報でありGit管理しない。 | basic_design | バンドルの版管理方針という永続化方針の決定。 | high |
| DES-861 | 提出結果の検証に必要な情報は判断記録へ複製されるため、バンドル自体の永続化は不要である。 | basic_design | 永続化方針の帰結。 | high |
| DES-874 | 本書が定義する判断型の値は`test-semantic`/`impl-consistency`/`case-coverage`の3種である。 | spec | 判断型は3種のみという網羅的定義（「検証状態は5つのみ」型の約束と同型）。 | low |
| DES-875 | `test-semantic`は、subjectの値域がTest IDであり、外部へ引き渡す問いが定まっている。 | spec | 問いの定義という主要な機能定義（chain_integrity等と同型）。 | high |
| DES-876 | `impl-consistency`は、subjectの値域がTest IDであり、外部へ引き渡す問いが定まっている。 | spec | 問いの定義という主要な機能定義。 | high |
| DES-877 | `case-coverage`は、subjectの値域をTest IDまたはVO IDとする。 | spec | subject値域の定義という主要な機能定義。 | high |
| DES-878 | `case-coverage`は、subjectがTest IDのとき、外部へ引き渡す問いが定まっている。 | spec | 問いの定義。 | high |
| DES-879 | `case-coverage`は、subjectがVO IDのとき、外部へ引き渡す問いが定まっている。 | spec | 問いの定義。 | high |
| DES-880 | `judgment_kind`と`subject`の種別の組合せがこの表にない要求ではバンドルを生成しない。 | detailed_spec | 無効組合せを別紙Aのusage error・終了コード2へ精密に対応付ける。 | high |
| DES-881 | `case-coverage`は§11の判断対象であって§5の4検査ではない。 | spec | 領域境界の約束。 | high |
| DES-882 | `case-coverage`の未判断・判断結果はいずれも4検査の値へ写像せず、§11.3の集約へ寄与しない。 | spec | 領域境界の約束。 | high |
| DES-883 | 外部判断が必要な事実は§11.7の判断待ち情報として提示する。 | spec | 一般的な振る舞い。 | high |
| DES-886 | 宣言targetまたは上流documentを解決できない場合はバンドルを生成せず候補も選択しない。 | detailed_spec | 解決不能時にバンドル非生成・非選択という境界条件の精密な規定。 | high |
| DES-887 | 解決失敗の種別（対象不在、E-SCAN-004・document不在）を`MISMATCH`（診断`MISSING`）とする。 | detailed_spec | エラーコード→状態・診断ラベルの精密な対応。 | high |
| DES-888 | 解決失敗の種別（恒久SRC ID衝突による曖昧、E-SCAN-011）を`MISMATCH`として保持する。 | detailed_spec | エラーコード→状態の精密な対応。 | high |
| DES-890 | `vtest audit submit --file result.json`で提出する。 | design | サブコマンドのインターフェース契約（フラグ名の決定）。 | high |
| DES-893 | 提出スキーマは`bundle_id`・`subject`・`judgment_kind`・`supersedes`・`decision`等をfieldとする。 | design | 提出スキーマのfield列挙という型宣言。 | high |
| DES-895 | 旧モデルの`verdict → CheckValue`写像（PASS/FAIL/COMPLETE/INCOMPLETE検査）を撤去する。 | design | 旧写像経路の撤去というコンポーネント固有の挙動変更。 | low |
| DES-896 | 判断記録は検証状態を変更しない（§8冒頭）。 | spec | 一般的な領域境界の約束（§8冒頭の再掲）。 | high |
| DES-902 | 理由が空であることだけを根拠に判断を無効化しない。 | spec | 一般原則（基本仕様§11.3の直接引用）。 | high |
| DES-903 | `audit submit`は、検証に失敗した場合は§17のエラーコードで拒否する。 | basic_design | 名指しコマンドの責務定義。 | high |
| DES-910 | 受理された提出は判断記録（§3.4）として`.verify/decisions/`へ保存される。 | basic_design | ディレクトリ・レコード構成の決定。 | high |
| DES-913 | 旧モデルのreasons/claim/basis必須検査（E-AUDIT-005）等を撤去する。 | design | 旧検査群の撤去というコンポーネント固有の契約変更。 | high |
| DES-914 | 旧モデルの必須検査は要件定義§12「理由が空であることだけを根拠に無効扱いしない」に反する。 | spec | 上位引用に基づく一般的な振る舞いの約束。 | high |
| DES-915 | 判断記録の有効性は判定時に評価する。 | spec | 一般的な評価タイミングの約束。 | high |
| DES-917 | 対象は`(subject, judgment_kind)`の組であり、組ごとに独立に評価する。 | spec | 一般的な定義。 | high |
| DES-926 | 未確定である事実は§11.7の判断待ち情報として提示する。 | spec | 一般的な振る舞い。 | high |
| DES-935 | 同一対象に有効な判断記録が複数あってよい（再判断・多重判断）。 | spec | 一般的な許容範囲。 | low |
| DES-936 | 回数はツールとして制限しない（運用ポリシー）。 | spec | 一般的な運用ポリシー。 | high |
| DES-937 | 仕様等が変更された場合、過去の判断を現在状態へそのまま流用してはならない。 | spec | 一般原則（要件定義§12直接引用）。 | low |
| DES-938 | 現在状態に対して通常の検証（§5の4検査）を再実施した結果は5状態いずれかになる。 | spec | 5状態いずれにもなり得るという一般的な定義。 | high |
| DES-939 | 変更そのものが`UNKNOWN`を生成するのではない。 | spec | 一般的な否定的明確化。 | high |
| DES-940 | 判断済みと承認済みは区別する（判断済み≠承認済み）。 | spec | 一般的な定義（SPEC-400と同一文言）。 | high |
| DES-941 | 判断は承認なしでも記録でき、正式採用は§3.5の承認の別段階である。 | spec | 一般的な約束（SPEC-401/402と同型）。 | high |
| DES-1190 | すべての判定は「その時点の正典の読み取り」に基づき、正典が変われば次回のscan / verifyが差分を反映する。 | spec | 外部から観測できる一般的な整合性の約束。 | high |
| DES-1192 | 原子的公開の対象は`.verify/`配下のrecord・エンティティファイルであり書きかけ状態を正典に見せない。 | detailed_spec | 原子的公開の対象範囲・可視化方式・部分状態非観測という精密な不変条件。 | high |
| DES-1194 | 解析不能な中間状態はadapter discoveryのE-SCAN-001 / Incompleteとしてfail-closeする。 | detailed_spec | エラーコードへの精密な対応。 | high |
| DES-1195 | `vtest doctor`は、同じTest IDの重複、covers先VOの欠落、承認済VOの内容不一致等を検出する。 | basic_design | 名指しコマンドの責務定義。 | high |

### §9（テスト実行設計）・§10（`rust-cargo` Target Binding 動的計測）

| ID | 文（先頭60字） | 層 | 理由 | 確信度 |
|---|---|---|---|---|
| DES-946 | 旧モデルの`--req`（REQ指定）はdocument層の総称化により廃止し代替経路を設ける。 | basic_design | CLIインターフェース構成の決定（フラグ廃止と代替経路）。 | low |
| DES-947〜951 | `TestEntity.execution`の各field（project/suite.kind/suite.name/selector等）の解釈。 | design | adapter自身のフィールド解釈契約。 | high |
| DES-952 | `TestEntity`へCargo固有fieldを戻してはならない。 | design | core型の境界を定める型契約（既出SPEC-112と同型）。 | high |
| DES-953 | 実行は（project, suite）で分けたbatchとし、libtestの`--exact` flagと複数selectorを用いる。 | design | 具体コマンドライン（`cargo test -p ... --exact ...`）を固定（code_like）。 | high |
| DES-954 | 実行対象の解釈とcommand生成は`TestRunnerAdapter`が所有する。 | basic_design | 責務分界（所有権の割当）。 | high |
| DES-955 | orchestrationは`ExecutionDescriptor.adapter`をregistryで解決し、adapter不一致を検出する。 | detailed_spec | E-ADAPTER-003の外部的意味が主内容（registry言及との複合文）。 | low |
| DES-956 | 明示的なrunでrunner未提供ならE-ADAPTER-004としてEvidenceを生成せずエラーとする。 | detailed_spec | エラーコード・検証状態・診断の精密な割当。 | high |
| DES-957 | `--exact`は後続の全フィルタへ適用されるフラグであり、各フィルタは完全一致で解釈される。 | design | フラグ解釈契約というadapter自身の契約。 | high |
| DES-958 | stdoutのパースはstable toolchainの標準出力形式のみに依存する。 | design | adapter自身のパース前提契約。 | high |
| DES-959〜962 | `running N tests`/`... ok`/`... FAILED`/`... ignored`という出力の意味付け。 | design | 出力文字列に依存する意味付け（形式が変われば偽になる、code_like）。 | high |
| DES-967 | stdout / stderrの全文は`cache/logs/<ULID>.log`へ保存し、Evidenceの`log_ref`が参照する。 | design | Evidenceのデータ参照契約（構造決定かの境界が微妙）。 | low |
| DES-969 | `revision`は実行直前に`git rev-parse HEAD`と`git status --porcelain`で取得する。 | design | 具体コマンドを固定した記述（code_like）。 | high |
| DES-971 | `hashes`は、実行直前のdiscovery結果からTest subject hash等を構成する。 | design | `hashes`フィールド構成という型定義相当の記述。 | high |
| DES-978 | `execution_state`は、実行直前にrunner adapterが返すsnapshot schemaに基づく。 | design | execution_state hashの入力源という型構成要素の定義。 | high |
| DES-982 | `rust-cargo` CoverageAdapterは`cargo-llvm-cov`を使用する。 | basic_design | 技術選定。 | high |
| DES-983 | 起動時に`cargo llvm-cov --version`で利用可否を確認し、利用不能なら計測しない。 | design | 具体コマンドを用いた確認手順の固定（code_like）。 | high |
| DES-985 | カバレッジをTest単位で対象関数へ帰属させるため、計測時はTestを1件ずつ実行する。 | design | 計測手法に関するadapter自身の技術契約。 | high |
| DES-986 | Testが起動したsubprocess・spawnしたthreadの実行を宣言targetへ帰属させられるかはcapability依存。 | basic_design | capability境界の割当。 | high |
| DES-987 | subprocess内の実行を計測するには起動される実行体もinstrument対象とする。 | design | capability実現手段の技術的説明。 | high |
| DES-991 | 計測コマンドは`cargo llvm-cov test -p <project> --lib --json …`である。 | design | 具体コマンドラインを固定した記述（code_like）。 | high |
| DES-992 | coverageは独立した`CoverageAdapter` capabilityとして扱う。 | basic_design | capability分離という構造決定。 | high |
| DES-995 | 出力JSON（llvm-cov export形式）の`data[].functions[]`からTestの対象関数を検索する。 | design | 具体JSONスキーマ経路を固定した記述（code_like）。 | high |
| DES-996 | 一致条件は、demangle済み関数名の末尾がlocatorのitem-pathと一致することである。 | design | adapter固有のマッチング基準（技術契約か精密な振る舞いかの境界が微妙）。 | low |
| DES-997 | ジェネリック関数は複数インスタンスが現れるため、同じtargetに対応するcountを合算する。 | design | adapter固有の算出規則。 | high |
| DES-1004 | 各targetのcanonical Locator（§6.1.1）・result・countとTest単位集約結果を記録する。 | design | target_coverageレコードの構成という型定義相当の記述。 | high |
| DES-1006 | Evidenceの`target_coverage`は§7.3のtarget_binding runtime証明の証拠源である。 | detailed_spec | 独立検査でないという位置付けの精密な明確化（spec寄りとの境界が微妙）。 | low |
| DES-1008 | coverage providerは当該境界越しの実行を宣言targetへ帰属させなければならない。 | design | capability提供者自身の技術契約（capability境界の主張との境界が微妙）。 | low |
| DES-1012 | coverage providerが境界越しの実行を宣言targetへ帰属させられるかはadapterのcoverage capability依存。 | basic_design | capability境界の割当（DES-986と同型）。 | high |
| DES-1013 | `vtest run`は2モードを持つ。 | spec | モードの個数を定める語彙宣言。 | high |

### §11（鮮度検証と集約）

| ID | 文（先頭60字） | 層 | 理由 | 確信度 |
|---|---|---|---|---|
| DES-1018 | `target_binding`は評価地点をTESTとし、§7.3の合成による。 | spec | 評価地点の宣言のみで条件は§7.3へ委譲、確定させていない。 | high |
| DES-1020 | `target_binding`の未充足は§11.2の写像に従う。 | spec | 詳細を§11.2へ委譲する記述で条件自体は確定しない。 | high |
| DES-1021 | `oracle_presence`は評価地点をTESTとし、§7.1の合成による。 | spec | 評価地点の宣言のみで詳細は§7.1へ委譲する。 | high |
| DES-1035 | 本システムは意味判定・候補生成を外部の判定器へ委ねるseamを4検査の評価経路に持たない。 | spec | 機能の不在を述べる主要な振る舞いの約束。 | high |
| DES-1036 | 外部AI／Agentは判断記録（§8）の著者として`.verify/decisions/`へ記録を残す経路でのみ関与する。 | spec | 外部AIの関与経路を限定する主要な振る舞いの約束。 | high |
| DES-1039 | 完全検証の検査集合はこの4検査に固定し、設定で追加・削除できない。 | spec | 「検証状態は5つのみ」と同型の主要な振る舞いの約束。 | high |
| DES-1040 | 旧モデルの12項目（`spec_coverage`等）は検査として存在しない。 | spec | 検査集合に関する主要な振る舞いの約束。 | high |
| DES-1041 | `test_existence` / `test_traceability`は`chain_integrity`へ統合した。 | spec | 旧項目の統合先を述べる主要な振る舞いの約束。 | high |
| DES-1042 | `static_audit`は`oracle_presence`と`target_binding`の静的到達へ分割した。 | spec | 旧項目の分割先を述べる主要な振る舞いの約束。 | high |
| DES-1043 | `test_execution` / `target_execution` / `runtime_result`は`target_binding`の証拠へ吸収した。 | spec | 旧項目の吸収先を述べる主要な振る舞いの約束。 | high |
| DES-1044 | `evidence_validity`は独立検査を廃し、鮮度喪失を診断ラベル`STALE`として説明した。 | spec | 旧項目の廃止を述べる主要な振る舞いの約束。 | high |
| DES-1045 | `spec_coverage`等は検査から除去し、疑義は`UNKNOWN`としてエスカレーションとした。 | spec | 旧項目の除去を述べる主要な振る舞いの約束。 | high |
| DES-1046 | `chain_integrity`は宣言鎖のすべてのリンクが存在し、ハッシュ照合が成立するかを問う。 | spec | 検査が問う内容の記述（基本仕様の問いの記述と同型）。 | high |
| DES-1060 | すべてのTestを管理対象とすることと、証拠として算入することは別個の条件とする。 | spec | 基本仕様の同型記述（SPEC-234相当）と一致する主要な振る舞いの約束。 | high |
| DES-1087 | `verify.full_scope`はconfig読込み時に§2.2のinvariantとして検証・正規化済みでなければならない。 | design | 特定処理段階に紐づく内部前提条件（部品の契約）。 | high |
| DES-1089 | aggregateは、scanによりグラフ構築する。 | design | 内部データ構造・処理経路の選択。別実装がグラフを介さない場合に偽になりうる。 | low |
| DES-1100 | 利用者向け簡易出力は`OK` / `NG`の二値とする。 | spec | 出力値の種類に関する主要な振る舞いの約束。 | high |
| DES-1101 | 詳細出力は任意ノードからの局所／経路／全体トレースに沿ったツリー表示とする。 | spec | 機能レベルの出力能力の約束。 | high |
| DES-1102 | 人間向けテキストと機械可読JSONの両方を出力できる。 | spec | 基本仕様§22.3と同型の機能レベルの約束。 | high |
| DES-1109 | 基本仕様§22.2の「Feature単位」は親VOを単位として実現する。 | design | 抽象概念を既存の型（親VO）で実現する対応付けの決定。 | high |
| DES-1110 | Featureを独立のエンティティ種別・レコードファイル・ID体系として設けない。 | basic_design | 正典データモデル構造（エンティティ種別・ディレクトリ）の決定。 | high |
| DES-1114 | covers宣言を経由しない「機能名による束ね」を設けない。 | spec | 機能の不在を述べる主要な振る舞いの約束。 | high |
| DES-1117 | 機能単位の表示経路は§11.6のprojectionで露出し、新規コマンド・ツールを増やさない。 | basic_design | インターフェース面（CLI/MCP surface）の構造決定。 | high |
| DES-1123 | プロジェクト側が登録したゲートの進行条件について、評価・提示できなければならない（MUST）。 | spec | 主要能力の約束（MUST要件）。 | high |
| DES-1124 | 検証状態と承認は独立の軸であり、ゲートは両者の組合せを進行条件にできる。 | spec | 基本仕様§4.5と同型の基本原則の約束。 | high |
| DES-1125 | ゲート定義は`config.yaml`の`gates`にゲート名と進行条件を保持する。 | basic_design | 外部設定ファイル構造の決定。 | high |
| DES-1135 | 5状態に順序・優劣・包含関係を設けない。 | spec | 基本原則の約束。 | high |
| DES-1143 | 本システムの責務はゲート条件が現在満たされているかの評価・提示に限る。 | spec | スコープ限定の主要な振る舞いの約束。 | high |
| DES-1144 | フェーズのライフサイクル管理・工程の自動遷移は責務外とする（§29 OOS-004）。 | spec | スコープ外宣言。 | high |
| DES-1145 | 「Releaseフェーズへ遷移させる」のではなく「条件を現在満たしている」を提示する。 | spec | フレーミングの確認（主要な振る舞いの約束）。 | high |
| DES-1146 | 新規CLIコマンド・MCPツールを増やさず、既存の`--gate`引数と出力で露出する。 | basic_design | インターフェース面の構造決定。 | high |
| DES-1147 | 具体的なフェーズ名・承認ロール等は別紙Aへ委譲する。 | spec | 委譲宣言（主要な振る舞いの約束）。 | high |
| DES-1153 | 親VOを起点とする下流方向のprojectionが、機能単位の集約を提示する経路である。 | design | 既存機構への対応付けの決定。 | low |
| DES-1154 | 親VOの代表値と、その配下の子VOごと・Testごとの内訳を同じ出力から辿れる。 | detailed_spec | 出力内容の精密な規定。 | high |
| DES-1155 | Feature名・Feature IDの別fieldを出力に設けず、識別子は親VOのIDとする。 | design | 出力schemaのフィールド決定。 | high |
| DES-1156 | 新規コマンド・ツールを増やさず、既存の`report`/`test query`で露出する。 | basic_design | インターフェース面の構造決定。 | high |
| DES-1157 | 逆引きインデックスをprojectionの基盤とする。 | design | 内部データ構造・処理経路の決定。 | high |
| DES-1158 | projectionが出力する`derives_from`エッジには`anchor`を常に同伴させる。 | design | 出力エッジのフィールド構成の決定。 | high |
| DES-1159 | `anchor`を持たないentryでは当該fieldを省略または`null`とする。 | design | フィールド値表現の決定。 | low |
| DES-1161 | `anchor`の値は不透明な文字列としてtransportするだけで、解決・整合検査を行わない。 | detailed_spec | 責務境界の精密な規定。 | high |
| DES-1162 | 対応ペア取得のために新規コマンド・ツールを設けない。 | basic_design | インターフェース面の構造決定。 | high |
| DES-1163 | 未確定事項等を機械可読な構造として保持・取得可能とする。 | spec | 機能レベルの提供能力の約束。 | high |
| DES-1169 | `check`が`null`の項目は§11.3の集約へ寄与せず、検査の値を変更しない。 | detailed_spec | 精密な振る舞い規定（集約への非寄与）。 | high |
| DES-1174 | `judgment_kind: case-coverage`の項目は対象Testごとにちょうど1件生成する。 | detailed_spec | 生成数の精密な規定。 | high |
| DES-1175 | 生成条件の1つは、`covers`が1件以上あることである。 | detailed_spec | 生成条件の精密な規定。 | high |
| DES-1176 | 生成条件の1つは、`cases`が1件以上あるか、covers先VOが`dimensions`を1件以上持つことである。 | detailed_spec | 生成条件の精密な規定。 | high |
| DES-1177 | 生成条件の1つは、実効判断が`accepted`でないことである。 | detailed_spec | 生成条件の精密な規定。 | high |
| DES-1178 | 実効判断が未確定・`rejected`・`deferred`のいずれでも項目を生成する。 | detailed_spec | 精密な条件と出力要件の規定。 | high |
| DES-1179 | 判断型に由来する生成条件は`case-coverage`型の項目にだけ適用する。 | detailed_spec | スコープ限定の精密な規定。 | high |
| DES-1180 | 検査由来の`kind: unknown`項目の生成・消滅は検査値だけで決まる。 | detailed_spec | 独立性の精密な規定。 | high |
| DES-1181 | 実効判断が競合により未確定となった項目の内容を規定する。 | detailed_spec | 精密な条件と出力内容の規定。 | high |
| DES-1182 | 新規コマンド・ツールを増やさず、既存のJSON出力に判断待ちsectionを含める。 | basic_design | インターフェース面の構造決定。 | high |
| DES-1183 | UNKNOWNだけでなく未確定・要判断事項を横断的に集約する。 | spec | 機能レベルの提供能力の約束。 | high |

### §17（診断・終了コード体系）

（§16は例外なし。DES-1189・DES-1191は§16.1の既定層basic_designと一致しており、低確信度ではあるが例外ではないため(c)には含まない。§8.5のDES-901も同様に既定層detailed_specと一致する低確信度例であり(c)には含まない。）

| ID | 文（先頭60字） | 層 | 理由 | 確信度 |
|---|---|---|---|---|
| DES-1201 | 診断コードは§5.4のスキャン診断に加えて定義する。 | spec | この節が診断コードの定義範囲を宣言するスコープ記述であり、実装方法によらず真である約束。 | high |
| DES-1227 | 旧モデルの意味監査提出検査（E-AUDIT-005/006/007）は判断記録層への転用に伴い撤去する。 | spec | 旧検査の撤去という外部から観測可能なスコープ変更を述べる約束。 | high |
| DES-1235 | 要求scopeの総合OK/NGはJSONとtextの集約出力から読み取れる。 | spec | 主要な出力の存在を述べる記述で、条件→コードの精密な対応を含まない。 | high |
| DES-1237 | 終了コードは診断severityだけでなく操作段階で決める。 | spec | 終了コード決定の要因を述べる一般原則で、特定条件と特定コードを一意に対応付けていない。 | high |
| DES-1242 | 検証状態と内部エラーは終了コードで分離する。 | spec | 基本仕様§4.4・§26.1を引用した一般原則の再述で、特定条件→特定コードの精密な対応ではない。 | high |

### §19（実装選択と提供範囲）

例外なし（11文すべて既定層のspecに一致）。

## (d) code_like 一覧

`code_like: true` は全553文中**14件**、すべて§6.2〜6.3・§9.2〜9.4・§10.1担当グループの範囲に集中する（§7・§8・§11・§16・§17・§19からは0件）。

| ID | 文（先頭80字） | なぜ code_like か |
|---|---|---|
| DES-735 | `rust-cargo`のlocator解決は、pathが索引に存在するかを確認する。 | descriptionが「1. path存在確認 / 2. item-path一致確認 / 3. 曖昧時候補返却」と番号付き手順を記述しており、ステップ1に相当。 |
| DES-736 | `rust-cargo`のlocator解決は、path内でitem-pathが一致するfn / impl fnが存在するかを確認する。 | 同じ番号付き手順のステップ2。別の順序・粒度で実装すれば文言と食い違う。 |
| DES-737 | `rust-cargo`のlocator解決で一意に決まらない場合は、すべて候補として返し曖昧を報告する。 | 同じ番号付き手順のステップ3（分岐後の処理）。 |
| DES-739 | `rust-cargo` adapterは、item-pathの末尾セグメント一致（別パスの同名関数）の順で候補を構築する。 | descriptionが「1. 末尾セグメント一致 / 2. 編集距離2以内」と優先順位を番号付けしており、順序が変われば文言が偽になる。 |
| DES-740 | `rust-cargo` adapterは、編集距離2以内の近似名の順で候補を構築する。 | 同じ優先順位手順の2番目のステップ。 |
| DES-953 | 実行は（project, suite）で分けたbatchとし、libtestの`--exact` flagと複数selectorを用いる。 | descriptionに`cargo test -p <project> --lib -- --exact <selector1> <selector2> ...`という具体コマンドラインが記載され、統合テストの`--test`分岐まで固定。 |
| DES-959 | `running N tests`という出力は実行対象数の確認を意味する。 | cargo testの特定出力文字列に対する意味付けで、出力形式（バージョン・toolchain）が変われば文言が偽になる。 |
| DES-960 | `test <selector> ... ok`という出力はPASSを意味する。 | 同上、特定出力文字列への意味付け。 |
| DES-961 | `test <selector> ... FAILED`という出力はFAILを意味する。 | 同上。 |
| DES-962 | `test <selector> ... ignored`という出力は実行されずを意味する。 | 同上。 |
| DES-969 | `revision`は実行直前に`git rev-parse HEAD`と`git status --porcelain`で取得する。 | 具体コマンド名を固定しており、別API（libgit2等）で同等の値を得る実装では文言が偽になる。 |
| DES-983 | 起動時に`cargo llvm-cov --version`で利用可否を確認し、利用不能なら計測しない。 | 具体コマンド（`cargo llvm-cov --version`）による確認手順を固定。 |
| DES-991 | 計測コマンドは`cargo llvm-cov test -p <project> --lib --json --output-path…`である。 | 完全なコマンドライン（フラグ・出力パス形式含む）を逐語的に固定。 |
| DES-995 | 出力JSON（llvm-cov export形式）の`data[].functions[]`から、Testが宣言する各対象関数を検索する。 | llvm-cov exportの具体的なJSONパスを固定しており、フォーマット変更・別ツールでは文言が偽になる。 |

§7の担当グループは、個別ルール群（DA-001〜006・W-DA-101）がcode_likeの温床になりやすいと想定して精査したが、全文が「入力条件→verdict」の宣言的表として書かれており該当なしと判定した（判断が割れかけた候補と棄却理由はグループ2の元レポートに記録）。§11の担当グループも同様に§11.2/§11.3の列挙構造を精査したが、いずれも計算順序を入れ替えても成立する宣言的定義であり該当なしとした（DES-1089のみ内部データ構造選択としてdesignに置いたが、順序・ループ・分岐は固定していないためcode_likeは付けていない）。

## (e) 直接観察（`design` に残った文について、件数のみ）

`design`（79件、14.3%）は5グループの担当範囲全体に一様に分布してはおらず、§8.1〜8.2（バンドル生成、31件）と§6.2〜10.3（Target Reference解決からTarget Binding動的計測まで、37件）の2箇所に厚く、§7（6件）・§11（17件）・§16/§17/§19（0件）は薄い。

内容で分けると3種類に大別できる。第一に、成果物・レコードが持つfield一式の型宣言（`SourceLocation`型宣言と同型の粒度）で、§8.1のバンドル構成9項目（DES-863〜871相当）、§8.2のJSONスキーマ、§9のTestEntity各field解釈（DES-947〜952, 957, 958, 971, 978）、§11.6〜11.7の出力レコードのフィールド型・値域宣言（12件）がこれにあたる。第二に、adapter・providerなど部品自身の技術契約（技術選定そのものではなく、選定済み技術の使い方の契約）で、§9〜10のcargo/llvm-cov実行・パース・計測契約や、§7の`StaticAnalysisAdapter`入出力契約（DES-763〜768、§7のdesign 6件はすべてここに集中）が該当する。第三に、cargo/llvm-covの具体コマンドライン・出力文字列・JSONパスを逐語的に固定した記述で、これが14件のcode_like全件と一致し、§9.2〜10.1に集中する。§8.1のdesign（30件）は成果物のfield列挙が中心で、受理・拒否条件を伴う§8.3の詳細仕様群とは区別された。§11のdesign（17件）は出力フィールド型・値域宣言（12件）と既存機構への対応付け決定（2件）・内部データ構造選択（3件）に分かれ、§16/§17/§19では該当する内容自体が現れなかった（§17は「コードの外部的意味」がすべてdetailed_specに落ち、内部伝播を名指しする文が本冊の記述粒度には存在しなかったため）。
