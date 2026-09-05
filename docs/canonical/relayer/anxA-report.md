# 別紙A（インターフェース仕様） 層仕分け結果

対象: `docs/canonical/specification.json` の `design` 配列のうち `source.doc` が 別紙A（詳細設計 別紙A インターフェース仕様 v0.1）である全438文（ID: DES-1254〜DES-1691）。
判定基準: `docs/canonical/LAYERING.md` §2（判定1→1a→2→3）を、`docs/canonical/relayer/base-report.md`（基本仕様 §765文の先行試験）の分類実績と整合させながら、節ごとに既定層を定め、文単位で例外を確定した。

## (a) 節ごとの既定の層

既定の層は節内の多数決（最頻値）で自動算出した。同数の場合はCounterの出現順で決まる。

| 節 | 既定の層 | 文数 | 例外数 |
|---|---|---:|---:|
| （文書冒頭・§0相当） | spec | 4 | 0 |
| 12.1 共通仕様 | detailed_spec | 54 | 12 |
| 12.2 `vtest init` | detailed_spec | 14 | 0 |
| 12.2 `vtest scan` | detailed_spec | 4 | 0 |
| 12.2 `vtest doctor` | detailed_spec | 2 | 0 |
| 12.2 `vtest doc add / list / show` | detailed_spec | 26 | 5 |
| 12.2 `vtest vo add / edit / list / show / expand / approve` | detailed_spec | 28 | 0 |
| 12.2 `vtest approval create / withdraw / show` | detailed_spec | 19 | 2 |
| 12.2 `vtest test create` | detailed_spec | 3 | 0 |
| 12.2 `vtest test edit` | detailed_spec | 4 | 0 |
| 12.2 `vtest test show / list / query` | detailed_spec | 3 | 0 |
| 12.2 `vtest audit static` | detailed_spec | 5 | 0 |
| 12.2 `vtest audit bundle / submit`（判断記録プロトコル） | detailed_spec | 33 | 8 |
| 12.2 `vtest run` | detailed_spec | 4 | 0 |
| 12.2 `vtest verify` | detailed_spec | 32 | 2 |
| 12.2 `vtest report` | detailed_spec | 20 | 1 |
| 12.2 `vtest mcp` | detailed_spec | 1 | 0 |
| 12.3 フェーズゲート評価（`verify --gate` / `report --gate`） | detailed_spec | 33 | 2 |
| 12.4 判断待ち情報 section（`verify` / `report` JSON） | detailed_spec | 13 | 0 |
| 13.1 共通仕様 | detailed_spec | 6 | 3 |
| 13.2 ツール一覧 | detailed_spec | 40 | 0 |
| 13.3 エージェント向け利用フロー（参考） | detailed_spec | 4 | 2 |
| 14.1 スキーマ形式（`.verify/forms/<kind>.yaml`） | detailed_spec | 5 | 1 |
| 14.2 検証器 | detailed_spec | 13 | 2 |
| 14.3 組込フォーム | detailed_spec | 13 | 1 |
| 14.4 テスト種別ごとのフォーム拡張 | detailed_spec | 10 | 2 |
| 15. Structured Test Operation adapter contract | basic_design | 4 | 0 |
| 15.1 `rust-cargo` 対象の特定 | design | 3 | 1 |
| 15.2 `rust-cargo` 編集・挿入の適用 | detailed_spec | 25 | 7 |
| 15.3 `rust-cargo` annotation blockの再生成 | detailed_spec | 9 | 0 |
| 15.4 `rust-cargo` 1 Test境界の保証 | detailed_spec | 4 | 1 |

## (b) 集計

| 層 | 件数 | 割合 |
|---|---:|---:|
| detailed_spec | 377 | 86.1% |
| spec | 34 | 7.8% |
| basic_design | 14 | 3.2% |
| design | 13 | 3.0% |
| **合計** | **438** | 100.0% |

`confidence: low` は **14件**（DES-1433, DES-1561, DES-1608, DES-1651, DES-1656, DES-1663, DES-1667, DES-1679, DES-1680, DES-1681, DES-1682, DES-1683, DES-1684, DES-1686）。30件を下回るため、規則の再定義は不要と判断した。ただし §15（Structured Test Operation adapter contract）に低確信度が集中しており、その理由は (e) で述べる系統的な境界の緊張として報告する。
`code_like: true` は **9件**（DES-1652, DES-1654, DES-1655, DES-1656, DES-1663, DES-1665, DES-1666, DES-1667, DES-1688）で、いずれも §15.1・§15.2・§15.4 の `rust-cargo` 編集適用手順に集中する。

## (c) 例外一覧（節の既定層と異なる文）

既定層と異なる層に置いた文は51件。節番号順、節内はID順。

| ID | 文（先頭50字） | 層 | 理由 | 確信度 |
|---|---|---|---|---|
| DES-1264 | 終了コードは本冊 §17.2 に従う。 | spec | 終了コードは本冊§17.2に従うという委譲。実装方法によらず真であり続ける約束。 | high |
| DES-1282 | 検証状態は5値（`PASS` / `FAIL` / `MISMATCH` / `NO_EVIDENC | spec | 検証状態は5値であるという、基本仕様§4.1で既に確定した主要事実の repeated 記述。 | high |
| DES-1284 | 診断ラベルは状態に付随する原因説明であり、`MISSING` / `NOT_EXECUTED` /  | spec | 診断ラベルの列挙という、基本仕様§4.2で既に確定した主要事実の repeated 記述。 | high |
| DES-1286 | `NO_EVIDENCE` は状態であって診断ラベルではない。 | spec | NO_EVIDENCEは状態であって診断ラベルではないという、状態と診断ラベルの区別（基本仕様§4.1・4.2）の repeated 記述。 | high |
| DES-1287 | 診断ラベルを集約の代表値選択に用いず、原因説明として併記するだけとする。 | spec | 診断ラベルを集約の代表値選択に用いないという、基本仕様§22.2の集約規則の repeated 記述。 | high |
| DES-1288 | CLIの操作は登録済みadapter registryを通じて実装を選択する。 | basic_design | CLIの操作が登録済みadapter registryを介して実装を選択するという、adapter/core分界とcomposition構造の明示。 | high |
| DES-1293 | `filter`、`package`、`test_target` は `TestEntity` の  | design | filter/package/test_targetがTestEntityのfieldでないという、core domain型のfield境界の宣言（型宣言そのもの）。 | high |
| DES-1296 | coreは `targets ≥ 1` を adapter 中立の必須件数にせず、型としては空 li | design | coreがtargets≥1を型としては要求せず空listを許容するという、core domain型の内部制約の宣言。 | high |
| DES-1297 | targetsの必須件数は adapter が定める。 | basic_design | targetsの必須件数をadapterが定めるという、adapter/core責務分界の明示。 | high |
| DES-1303 | 本 version の Test metadata は存在理由分類（旧 `role` / `anch | spec | Test metadataがrole/anchor等の存在理由分類を持たないという、基本仕様レベルで確定済みのモデル特性の repeated 記述。 | high |
| DES-1304 | 本 version はすべての管理対象 Test に `covers ≥ 1` を一律に要求する。 | spec | covers≥1を一律要求するという、基本仕様§12レベルの主要事実の repeated 記述。 | high |
| DES-1306 | VO への寄与は `covers` 宣言と証拠の十分性判定だけから導出する。 | spec | VOへの寄与はcoversと証拠の十分性判定だけから導出するという、主要な振る舞いの記述。実装方法によらず真であり続ける約束。 | high |
| DES-1332 | `doc` は上流文書を総称 `document` レコードとして管理する唯一のコマンドである。 | spec | docが上流文書を総称documentとして管理し文書種別を区別しないという、基本仕様§3.1・3.2レベルのモデル哲学の repeated 記述。 | high |
| DES-1333 | `doc` は文書種別（要件定義・基本仕様・詳細設計・API Schema 等）を区別しない。 | spec | 文書種別を区別しないという、同上モデル哲学の repeated 記述。 | high |
| DES-1334 | 段（要件→仕様→詳細設計…）は `derives_from` リンクで表現し、種別を増やさない。 | spec | 段はderives_fromで表現し種別を増やさないという、同上モデル哲学の repeated 記述。 | high |
| DES-1335 | 旧モデルの `vtest spec` / `vtest req` は廃し、SPEC / REQ 実体 | spec | 旧spec/req実体層を持たないという、モデル哲学の repeated 記述。 | high |
| DES-1354 | document の承認・却下・取消は `vtest approval` で行い、`doc` 側に承 | spec | documentの承認操作をvtest approvalに一本化しdoc側に置かないという、責務分離の哲学的記述。実装方法によらず真であり続ける約束。 | high |
| DES-1386 | 承認は特定のエンティティ型に従属しない独立の領域であり、対象種別を引数に取るこの経路が承認レコード生 | spec | 承認が対象種別に従属しない独立領域であり、この経路が唯一の正典面であるという、モデル哲学・スコープの記述。 | high |
| DES-1387 | エンティティ側の `vo approve` / `vo_approve` はこの経路への別名にすぎず | spec | vo approve/vo_approveがこの経路への別名にすぎないという、承認の唯一正典面の哲学の repeated 記述。 | high |
| DES-1420 | `audit bundle` / `submit` は本冊 §8 の判断記録プロトコルであり、意味検 | spec | audit bundle/submitが判断記録プロトコルであり意味検査でないという、定義的・カテゴリ的記述。 | high |
| DES-1421 | 本システムは宣言されていない義務・網羅漏れ・宣言と実装の意味のずれを自ら発見・裁定しない。 | spec | 本システムが宣言されていない義務等を自ら発見・裁定しないという、要件定義§12由来の哲学的約束の repeated 記述。 | high |
| DES-1422 | 本システムは機械が決定論で確定できない疑義を `UNKNOWN` として外部（人間または判断可能 A | spec | UNKNOWNを外部へ引き渡し判断記録として追跡するという、基本仕様§11由来の哲学的約束の repeated 記述。 | high |
| DES-1433 | 旧モデルの `spec-coverage`（SPEC 層依存）は復活させない。 | spec | 旧spec-coverageを復活させないという、スコープ除外の記述。 | low |
| DES-1447 | 判断記録は検査ゲートではなく、`UNKNOWN` に対する外部判断の追跡である。 | spec | 判断記録が検査ゲートでなくUNKNOWNに対する外部判断の追跡であるという、定義的記述。 | high |
| DES-1449 | 判断記録（`.verify/decisions/` の actor / subject / deci | spec | 判断記録と承認記録が別軸・別entityであるという、基本仕様§17レベルのモデル哲学の repeated 記述。 | high |
| DES-1450 | 判断済み ≠ 承認済みである。 | spec | 判断済み≠承認済みという、区別の repeated 記述。 | high |
| DES-1451 | 判断は承認なしでも記録でき、正式採用は承認の別段階である。 | spec | 判断は承認なしでも記録でき正式採用は承認の別段階であるという、定義的記述。 | high |
| DES-1458 | 検査は基本仕様 §5 の固定4検査（`chain_integrity` / `orphan_dete | spec | 検査は基本仕様§5の固定4検査のみであるという、既に確定した主要事実の repeated 記述。 | high |
| DES-1459 | 旧モデルの12項目（`spec_coverage` / `vo_decomposition` / ` | spec | 旧12項目が検査として存在しないという、4検査確定の裏面の repeated 記述。 | high |
| DES-1490 | `verify` が判定用、`report` が閲覧・提出用という役割分担とする。 | spec | verifyが判定用、reportが閲覧・提出用であるという、両コマンドの役割分担の定義的記述。 | high |
| DES-1510 | プロジェクト側が登録したフェーズ・工程・ゲートの進行条件について、現在の検証状態（5状態）と承認（§ | spec | フェーズゲート評価をMUSTとして提示できなければならないという、基本仕様§20・要件定義§26.4の要件の repeated 記述。 | high |
| DES-1511 | 本システムの責務はゲート条件が現在満たされているかの評価・提示に限り、フェーズのライフサイクル管理・ | spec | 本システムの責務がゲート条件の評価・提示に限られ、ライフサイクル管理は責務外であるという、スコープ境界の記述。 | high |
| DES-1556 | transport は stdio である。 | basic_design | transportをstdioとするという、外部から見える構成要素選択（採用technology）。 | high |
| DES-1557 | `rmcp` で実装する。 | basic_design | rmcpで実装するという、外部から見える構成要素選択（採用ライブラリ）。 | high |
| DES-1561 | 各ツール呼び出しの冒頭で mtime ベースの再スキャン判定を行う。 | design | mtimeベースの再スキャン判定という、鮮度確認の具体的内部技法（別の技法でも同じ外部保証は満たせるため実装判断）。 | low |
| DES-1602 | 各操作は、CLIとMCPで同じadapter registryを解決する。 | basic_design | CLIとMCPが同じadapter registryを解決するという、adapter/core分界とインターフェース構造の明示。 | high |
| DES-1605 | 完了確認は `verify` の4検査で行う。 | spec | 完了確認はverifyの4検査で行うという、architecture-levelの利用指針。実装方法によらず真であり続ける約束。 | high |
| DES-1608 | `test_kind` の `regression` は Test の意図ラベル（`@vtest.k | spec | test_kindのregressionが意図ラベルであり旧存在理由分類とは別概念であるという、定義的区別の記述。 | low |
| DES-1618 | `kind` は `[a-z0-9][a-z0-9-]*` の case-sensitive 文字列 | basic_design | kindの文字列形式・ファイル名一致・repository-global Form IDという、ID/命名規約の割当。 | high |
| DES-1619 | `adapter` はその Form を処理する Structured Test adapter I | basic_design | adapterがそのFormを処理するStructured Test adapter IDであるという、所有権の割当。 | high |
| DES-1624 | 組込フォームは `rust-cargo` adapter が提供する。 | basic_design | 組込フォームをrust-cargo adapterが提供するという、構成要素選択（adapter/core分界に関わる決定）。 | high |
| DES-1637 | Form Schema はユーザー定義可能とし、大局的に一意な `kind` と登録済み Struc | basic_design | Form Schemaをユーザー定義可能とし、大局的に一意なkindと登録済みadapterのadapter IDを必須とするという、Form Schemaというメカニズムの構造的宣言。 | high |
| DES-1646 | user-defined Form も `kind` と owner `adapter` ID を宣 | basic_design | user-defined Formも通常のForm Schemaでありkindの大局的一意性とowner解決規則が変わらないという、Form Schemaメカニズムの構造的一貫性の宣言。 | high |
| DES-1653 | 再確認で見つからない場合は E-OP-002 とする。 | detailed_spec | 再確認で見つからない場合はE-OP-002を返すという、外部から観測可能なエラー条件。 | high |
| DES-1654 | desired state（answers / set / body）から、あるべきアノテーションブ | design | desired stateからあるべきアノテーションブロック・シグネチャ・本体を生成するという、Edit適用手順の1ステップ（生成技法）。 | high |
| DES-1655 | 現状とあるべき状態の diff を計算する。 | design | 現状とあるべき状態のdiffを計算するという、Edit適用手順の1ステップ（差分計算技法）。 | high |
| DES-1656 | 変更を、対象テスト関数の拡張 range（doc comment 先頭〜関数末尾）の単一置換として適 | design | 変更を拡張rangeの単一置換として適用するという、Edit適用の具体的技法（別の技法でも同じ結果を得られるため実装判断）。 | low |
| DES-1663 | Form 回答（§14）から、あるべきアノテーションブロックと関数シグネチャ・本体、および挿入位置を | design | Form回答からあるべきブロック・シグネチャ・挿入位置を決定するという、Create適用手順の1ステップ（決定技法）。 | low |
| DES-1665 | 挿入前の対象ファイルの内容を保持する。 | design | 挿入前の対象ファイル内容を保持するという、Create適用手順の1ステップ（バックアップ技法）。 | high |
| DES-1666 | 対象ファイルが存在しない場合は「不存在」を挿入前の状態として保持する。 | design | 対象ファイル不存在時に「不存在」を挿入前状態として保持するという、Create適用手順の1ステップ（バックアップ技法）。 | high |
| DES-1667 | 生成した Test construct を挿入位置へ単一挿入として適用する。 | design | 生成したTest constructを挿入位置へ単一挿入として適用するという、Create適用の具体的技法。 | low |
| DES-1688 | 置換範囲が単一のテスト関数の拡張 range に限られることを、適用前（範囲計算）と適用後（他 Te | design | 範囲限定を適用前後の二重検査で確認するという、具体的な検証技法（別の技法でも同じ保証を満たせるため実装判断）。 | high |

補足: DES-1651（Test IDから編集対象を特定する）は §15.1 の既定層 `design` と一致するため上表に現れないが、`confidence: low` である（理由は (e)）。同様にDES-1679〜1687（annotation block 再生成キー順・内容保持）は §15.3 の既定層 `detailed_spec` と一致するため例外表に現れないが、うち DES-1679〜1684・1686 は `confidence: low` である。

## (d) `code_like: true` 一覧

いずれも `rust-cargo` StructuredTestAdapter の編集・挿入・境界保証（§15.1・§15.2・§15.4）における、具体的な適用技法・手順ステップである。技法を変えても同じ外部保証（構文的妥当性、他Testの非改変、ロールバックの完全性）を満たせるため、削除・実装裁量に開いた「削除候補」として残す。

| ID | 文（先頭50字） | 理由 |
|---|---|---|
| DES-1652 | スキャン結果が古い可能性があるため、編集直前に対象ファイルのみ再パースし、Test ID の位置を再 | スキャン結果が古い可能性があるため編集直前に対象ファイルのみ再パースするという、adapter内部の具体的な再確認手順（別の技法でも同じ結果を得られるため実装判断）。 |
| DES-1654 | desired state（answers / set / body）から、あるべきアノテーションブ | desired stateからあるべきアノテーションブロック・シグネチャ・本体を生成するという、Edit適用手順の1ステップ（生成技法）。 |
| DES-1655 | 現状とあるべき状態の diff を計算する。 | 現状とあるべき状態のdiffを計算するという、Edit適用手順の1ステップ（差分計算技法）。 |
| DES-1656 | 変更を、対象テスト関数の拡張 range（doc comment 先頭〜関数末尾）の単一置換として適 | 変更を拡張rangeの単一置換として適用するという、Edit適用の具体的技法（別の技法でも同じ結果を得られるため実装判断）。 |
| DES-1663 | Form 回答（§14）から、あるべきアノテーションブロックと関数シグネチャ・本体、および挿入位置を | Form回答からあるべきブロック・シグネチャ・挿入位置を決定するという、Create適用手順の1ステップ（決定技法）。 |
| DES-1665 | 挿入前の対象ファイルの内容を保持する。 | 挿入前の対象ファイル内容を保持するという、Create適用手順の1ステップ（バックアップ技法）。 |
| DES-1666 | 対象ファイルが存在しない場合は「不存在」を挿入前の状態として保持する。 | 対象ファイル不存在時に「不存在」を挿入前状態として保持するという、Create適用手順の1ステップ（バックアップ技法）。 |
| DES-1667 | 生成した Test construct を挿入位置へ単一挿入として適用する。 | 生成したTest constructを挿入位置へ単一挿入として適用するという、Create適用の具体的技法。 |
| DES-1688 | 置換範囲が単一のテスト関数の拡張 range に限られることを、適用前（範囲計算）と適用後（他 Te | 範囲限定を適用前後の二重検査で確認するという、具体的な検証技法（別の技法でも同じ保証を満たせるため実装判断）。 |

## (e) 直接観測に基づく所見

別紙Aは詳細設計本冊が確定したコマンド・ツールを具体化するHOW文書であるため、438文の86.1%が`detailed_spec`（各コマンドの引数と制約、JSON envelope fieldの外部的意味、終了コード、エラー条件、MCPツールの入出力契約）に落ちるという、依頼時の予想どおりの分布になった。`basic_design`（14件）はほぼ全件が「adapter registry を介した実装選択」「production adapter を `rust-cargo` に限定する」「Form ID・kind の命名規約」「MCPのtransportをstdio・rmcpとする」という、基本仕様 §765文の先行試験（`base-report.md`）で確立済みの2パターン（adapter/core責務分界の positive な明示、ID・技術選定の割当）に一致しており、新しいパターンは生じなかった。他方で34件の`spec`は、別紙Aが独自に主張しているのではなく、基本仕様・要件定義レベルで既に確定した事実（5状態、固定4検査、判断≠承認、UNKNOWNのエスカレーション哲学）を具体的なコマンド文脈の中で言い換えているだけの文であり、「実装方法によらず真であり続ける約束」を`detailed_spec`の精密さで再言明しても層としては`spec`に戻ることを確認した。最大の緊張は§15（Structured Test Operation adapter contract、特に§15.1・§15.2・§15.4）で生じた。ここでは「外部から検証可能な保証（他Testが変化しない、ロールバックが完全である）」と「その保証を得るための具体的技法（単一置換、diff計算、二重検査）」が同一文中に混在しており、後者は`design`+`code_like`に切り出したが、annotation再生成のキー順（§15.3、DES-1679等）のように「出力される外部artifact（ソースファイル中のannotation block）の内容そのものを固定する記述」は、技法（design）と外部契約（detailed_spec）のどちらとも解釈でき、confidence: lowとして明示的に留保した。この境界（内部技法か、外部artifactの内容規定か）は本節固有の緊張であり、LAYERING.md の規則を機械的に再適用しても解消しない可能性があるため、Owner判断が必要な場合は§15.3をまとめて再検討することを推奨する。
