# 別紙C（受入仕様）層仕分け 結果報告

対象: `docs/canonical/specification.json` の `design` 配列中、`source.doc` が別紙C（詳細設計 別紙C 受入仕様 v0.1.md）である文（DES-1692〜DES-2154、463文）。

判定基準: `docs/canonical/LAYERING.md` §2（判定1→1a→2→3）。節ごとにデフォルト層を決め、文単位で例外を当てた。

## (a) 節ごとのデフォルト層

| 節 | デフォルト層 | 文数 | 例外数 |
|---|---|---:|---:|
| 18.1 共通条件 | 詳細設計(design) | 7 | 4 |
| 18.2 共通fixture | 基本設計(basic_design) | 41 | 7 |
| 18.3.1 discovery・record・graph と chain_integrity | 詳細仕様(detailed_spec) | 28 | 11 |
| 18.3.1 discovery・record・graph と chain_integrity / chain_integrity（宣言鎖の完全性） | 詳細仕様(detailed_spec) | 34 | 5 |
| 18.3.1 discovery・record・graph と chain_integrity / VOの`combinations`（`coverage_policy: explicit`） | 詳細仕様(detailed_spec) | 17 | 0 |
| 18.3.1 discovery・record・graph と chain_integrity / `derives_from` の `anchor` | 詳細仕様(detailed_spec) | 7 | 0 |
| 18.3.1 discovery・record・graph と chain_integrity / `vtest init` の非改変不変条件 | 詳細仕様(detailed_spec) | 4 | 0 |
| 18.3.2 orphan_detection（文書層の孤児検出） | 詳細仕様(detailed_spec) | 7 | 4 |
| 18.3.3 決定論的静的解析（oracle_presence・target_binding 静的到達） | 詳細仕様(detailed_spec) | 38 | 6 |
| 18.3.4 execution・Evidence（target_binding の証拠） | 詳細仕様(detailed_spec) | 35 | 3 |
| 18.3.5 target_binding 動的計測（per-target） | 詳細仕様(detailed_spec) | 17 | 2 |
| 18.3.6 判断記録プロトコル（非ゲート） | 詳細仕様(detailed_spec) | 53 | 13 |
| 18.3.7 承認と判断記録の分離 | 詳細仕様(detailed_spec) | 44 | 16 |
| 18.3.8 verify・report と scope | 詳細仕様(detailed_spec) | 31 | 6 |
| 18.3.8 verify・report と scope / 判定の決定性 | 詳細仕様(detailed_spec) | 8 | 1 |
| 18.3.8 verify・report と scope / 上流該当箇所の同伴 | 詳細仕様(detailed_spec) | 3 | 1 |
| 18.3.9 フェーズゲート評価 | 詳細仕様(detailed_spec) | 29 | 10 |
| 18.3.10 Structured Test Operation | 詳細仕様(detailed_spec) | 11 | 1 |
| 18.3.10 Structured Test Operation / Create の挿入後検証とロールバック | 詳細仕様(detailed_spec) | 8 | 0 |
| 18.3.11 MCP interface | 詳細仕様(detailed_spec) | 4 | 0 |
| 18.3.12 adapter contract | 詳細仕様(detailed_spec) | 28 | 9 |
| 18.4 提供範囲外 | 基本仕様(spec) | 9 | 0 |

## (b) 総計

| 層 | 件数 |
|---|---:|
| 基本仕様(spec) | 51 |
| 詳細仕様(detailed_spec) | 323 |
| 基本設計(basic_design) | 55 |
| 詳細設計(design) | 34 |
| 合計 | 463 |

`confidence: low`: 8 件。`code_like: true`: 5 件。

## (c) 例外（節デフォルトと異なる層に置いた文）

### 18.1 共通条件（デフォルト: 詳細設計(design)）

| id | 文 | 層 | 理由 | confidence |
|---|---|---|---|---|
| DES-1694 | 検証結果はfail-closedである。 | 基本仕様(spec) | fail-closedであるという、別の作り方でも真であり続ける粗粒度の約束。 | high |
| DES-1695 | 要求scopeに1件でも非PASSがあれば総合結果はNGになる。 | 詳細仕様(detailed_spec) | scope内1件でも非PASSならNGという条件付き結果。 | high |
| DES-1696 | scopeを限定してもscope外の値をPASSへ変更しない。 | 詳細仕様(detailed_spec) | scope限定時にscope外値を昇格しないという条件付き不変条件。 | high |
| DES-1697 | CLIとMCPは同じcore処理、adapter registry、JSON envelope、診断codeを使用する。 | 基本設計(basic_design) | CLIとMCPが同一core・registry・envelope・診断codeを共有するという構造上の決定。 | high |

### 18.2 共通fixture（デフォルト: 基本設計(basic_design)）

| id | 文 | 層 | 理由 | confidence |
|---|---|---|---|---|
| DES-1707 | すべての管理対象 Test に `covers ≥ 1` を一律要求するため、`covers` を宣言しない Test … | 詳細仕様(detailed_spec) | coversを持たないTestの結果（E-SCAN-007・MISMATCH）という条件付き結果。 | high |
| DES-1716 | 状態は5つのみとする。 | 基本仕様(spec) | 状態は5つのみという粗粒度の一般原則（基本仕様で確立済みの再掲）。 | high |
| DES-1718 | 診断ラベルは検証状態と別軸の原因説明である。 | 基本仕様(spec) | 診断ラベルと検証状態の関係を述べる粗粒度の概念規定。 | high |
| DES-1719 | 診断ラベルは状態値ではない。 | 基本仕様(spec) | 診断ラベルは状態値でないという粗粒度の概念規定。 | high |
| DES-1726 | DA-002 / DA-003がtarget別UNKNOWNになる。 | 詳細仕様(detailed_spec) | 特定fixture条件下でのDA-002/DA-003の値という条件付き結果。 | high |
| DES-1727 | runtimeの`target_coverage`のみでDA-002到達が充足される。 | 詳細仕様(detailed_spec) | runtimeのtarget_coverageのみでDA-002到達が充足されるという条件付き結果。 | high |
| DES-1738 | synthetic adapterは配布対象のproduction language adapterではない。 | 基本仕様(spec) | synthetic adapterの位置付けを述べる粗粒度のスコープ規定。 | high |

### 18.3.1 discovery・record・graph と chain_integrity（デフォルト: 詳細仕様(detailed_spec)）

| id | 文 | 層 | 理由 | confidence |
|---|---|---|---|---|
| DES-1740 | source discovery adapterは全Discovered Test draft、ManagedTestD… | 詳細設計(design) | adapter/core内部のDTO・型の契約であり、外部からは観測できない決定。 | high |
| DES-1741 | coreは出力を検証してTest subject / Source Target hashを計算してからManaged … | 詳細設計(design) | coreが検証→hash計算→具体化という処理順序で行うという内部手続き順の記述。 | high |
| DES-1742 | Source Targetはcanonical locatorと任意の恒久SRC IDを併有する単一のentityである… | 詳細設計(design) | adapter/core内部のDTO・型の契約であり、外部からは観測できない決定。 | high |
| DES-1743 | adapterは同一constructをlocator版とSrcId版の2 draftへ複製せず、恒久SRC IDを`S… | 詳細設計(design) | adapter/core内部のDTO・型の契約であり、外部からは観測できない決定。 | high |
| DES-1746 | Source Target identityは「宣言された`TargetRef` → 解決 → canonical Lo… | 基本設計(basic_design) | Target Reference解決の方向・単一所有経路という構造・責務境界の決定。 | high |
| DES-1751 | `SourceTargetDraft.target`は必ず`TargetRef::Locator`である。 | 詳細設計(design) | adapter/core内部のDTO・型の契約であり、外部からは観測できない決定。 | high |
| DES-1752 | `TargetRef::SrcId`をcanonical targetとして返したadapter出力はmalformed… | 詳細設計(design) | adapter出力のmalformed判定という内部契約（design/detailed_spec境界が曖昧）。 | low |
| DES-1758 | SRC ID参照はcoreの統合済みSRC索引から、その恒久SRC IDを宣言したSource Targetのcanon… | 詳細設計(design) | core内部のSRC索引という具体的な実現機構の記述。 | high |
| DES-1765 | Target Reference解決はcoreの単一経路が所有する。 | 基本設計(basic_design) | Target Reference解決の方向・単一所有経路という構造・責務境界の決定。 | high |
| DES-1766 | discovery、静的解析、実行、Evidence writer、検証集約が独自にcandidate列を走査して1件を… | 基本設計(basic_design) | Target Reference解決の方向・単一所有経路という構造・責務境界の決定。 | high |
| DES-1767 | adapter所有のmetadata宣言、ID、target、VO参照、record schema、Relationの違… | 基本仕様(spec) | 違反検出全般を述べる粗粒度の一般能力の約束。 | low |

### 18.3.1 discovery・record・graph と chain_integrity / chain_integrity（宣言鎖の完全性）（デフォルト: 詳細仕様(detailed_spec)）

| id | 文 | 層 | 理由 | confidence |
|---|---|---|---|---|
| DES-1770 | 診断ラベルを二重定義しない。 | 基本仕様(spec) | 診断ラベル二重定義禁止／document種別を区別しない、という粗粒度の一般原則。 | high |
| DES-1781 | document 種別を区別せず、要件定義・基本仕様・詳細設計・API Schema 等をすべて総称 document … | 基本仕様(spec) | 診断ラベル二重定義禁止／document種別を区別しない、という粗粒度の一般原則。 | high |
| DES-1787 | Relation writerは`REL-<ULID>`だけを生成する。 | 詳細設計(design) | Relation writer/readerのID生成・正規化という型・schemaレベルの内部契約。 | high |
| DES-1788 | readerはファイル名とrecord IDが同じbare ULIDのversion 1互換Relationを読み取り、… | 詳細設計(design) | Relation writer/readerのID生成・正規化という型・schemaレベルの内部契約。 | high |
| DES-1791 | VO writerは`status`を保存せず、実効値をApprovalから導出する。 | 詳細設計(design) | VO writerがstatusを保存しないというwriterの内部契約（型・フィールドレベル）。 | high |

### 18.3.2 orphan_detection（文書層の孤児検出）（デフォルト: 詳細仕様(detailed_spec)）

| id | 文 | 層 | 理由 | confidence |
|---|---|---|---|---|
| DES-1830 | `orphan_detection` は文書層のみを対象とし、親（上流 document）を持たない `document… | 基本仕様(spec) | orphan_detectionの対象範囲を述べる粗粒度の定義（基本仕様の再掲）。 | high |
| DES-1831 | 実装レイヤーの孤児検出（宣言されていない実装の検出）は行わない。 | 基本仕様(spec) | orphan_detectionの対象範囲を述べる粗粒度の定義（基本仕様の再掲）。 | high |
| DES-1833 | 根指定の追加・削除は `vtest doc` コマンドの引数で管理する。 | 基本設計(basic_design) | 根指定の管理をvtest docコマンドに委ねるという構造上の決定。 | high |
| DES-1836 | 旧モデルの W-SCAN-102（孤立 VO）は VO 層の警告であり、文書層 `orphan_detection` と… | 基本仕様(spec) | 旧モデルとの関係を述べる粗粒度の整理。 | low |

### 18.3.3 決定論的静的解析（oracle_presence・target_binding 静的到達）（デフォルト: 詳細仕様(detailed_spec)）

| id | 文 | 層 | 理由 | confidence |
|---|---|---|---|---|
| DES-1837 | DA-001〜DA-006とW-DA-101は本冊§7の判定条件に従う。 | 基本仕様(spec) | 本冊§7への準拠を指し示す粗粒度の参照。 | low |
| DES-1838 | 静的解析は正典レコードを持たない再計算派生であり、検証のたびに現在のsource / configから再計算する。 | 基本設計(basic_design) | 静的解析結果を正典レコードとせず毎回再計算するという永続化方式の決定。 | high |
| DES-1839 | 確定違反だけをFAILとし、解析限界をUNKNOWNとして保持する。 | 基本仕様(spec) | 確定違反のみFAIL・解析限界はUNKNOWNという粗粒度の一般原則。 | high |
| DES-1840 | 正常Testは違反なしとなり、各違反fixtureは対応ruleで非PASSになる。 | 詳細設計(design) | 受入fixtureと判定ruleの対応関係という受入スイート内部の構成の記述。 | high |
| DES-1852 | 信頼を宣言する専用の注釈・設定項目・レコードを新設せず、covers / 宣言targetのグラフだけで上記の各値が決ま… | 基本仕様(spec) | 信頼専用の注釈・レコードを新設しないという粗粒度の一般原則。 | high |
| DES-1869 | 表示scopeと内部依存評価を分離する。 | 基本設計(basic_design) | 表示scopeと内部依存評価を分離するという責務境界の決定。 | high |

### 18.3.4 execution・Evidence（target_binding の証拠）（デフォルト: 詳細仕様(detailed_spec)）

| id | 文 | 層 | 理由 | confidence |
|---|---|---|---|---|
| DES-1880 | Evidence writerは中立fieldの`hashes.test_subject`と`hashes.target… | 詳細設計(design) | Evidence writerが出力するfield名（hashes.test_subject等）という型・schemaレベルの契約。 | high |
| DES-1900 | 旧モデルの`test_execution` / `runtime_result` / `target_execution… | 基本仕様(spec) | 旧モデルの検査項目撤去という粗粒度の移行整理（基本仕様が定める4検査の再掲）。 | high |
| DES-1901 | 鮮度喪失の独立検査（旧`evidence_validity`）は設けず、鮮度は基本仕様§6のハッシュ束縛により満たし、喪… | 基本仕様(spec) | 旧モデルの検査項目撤去という粗粒度の移行整理（基本仕様が定める4検査の再掲）。 | high |

### 18.3.5 target_binding 動的計測（per-target）（デフォルト: 詳細仕様(detailed_spec)）

| id | 文 | 層 | 理由 | confidence |
|---|---|---|---|---|
| DES-1925 | `target_coverage` は `target_binding` の動的計測結果であり独立の検査項目ではない。 | 基本仕様(spec) | target_coverageが独立検査でないという粗粒度の一般原則（基本仕様4検査の再掲）。 | high |
| DES-1926 | 旧モデルの`target_execution`検査項目は撤去し、計測事実だけをEvidenceの`target_cove… | 基本仕様(spec) | target_coverageが独立検査でないという粗粒度の一般原則（基本仕様4検査の再掲）。 | high |

### 18.3.6 判断記録プロトコル（非ゲート）（デフォルト: 詳細仕様(detailed_spec)）

| id | 文 | 層 | 理由 | confidence |
|---|---|---|---|---|
| DES-1927 | `vtest audit bundle` は判断対象ごとに、判断に必要な情報（対象 VO と claim、Test In… | 基本設計(basic_design) | バンドルの出力先パス（cache/bundles/）という永続化方式の構造上の決定。 | high |
| DES-1928 | バンドルは派生情報であり Git 管理しない。 | 基本設計(basic_design) | バンドルを派生情報としてGit管理しないという永続化方針の決定。 | low |
| DES-1929 | `vtest audit submit` の判断は少なくとも actor / subject / decision / … | 詳細設計(design) | submitの必須/任意fieldという型レベルの契約。 | high |
| DES-1937 | 旧モデルの reasons / claim / basis 必須検査（E-AUDIT-005）、decompositio… | 基本仕様(spec) | 判断記録プロトコルの位置付け・旧モデルとの関係を述べる粗粒度の概念規定。 | high |
| DES-1940 | 判断記録プロトコルは検証状態のゲートではなく、`UNKNOWN` に対する外部判断の追跡である。 | 基本仕様(spec) | 判断記録プロトコルの位置付け・旧モデルとの関係を述べる粗粒度の概念規定。 | high |
| DES-1941 | 旧モデルの `verdict → CheckValue` 写像（`impl_consistency = MISMATCH… | 基本仕様(spec) | 判断記録プロトコルの位置付け・旧モデルとの関係を述べる粗粒度の概念規定。 | high |
| DES-1942 | 旧モデルの意味監査 bundle 種別（spec-coverage / test-semantic / vo-cover… | 基本仕様(spec) | 判断記録プロトコルの位置付け・旧モデルとの関係を述べる粗粒度の概念規定。 | high |
| DES-1943 | `spec_coverage` / `vo_decomposition` / `vo_coverage` / `impl… | 基本仕様(spec) | 判断記録プロトコルの位置付け・旧モデルとの関係を述べる粗粒度の概念規定。 | high |
| DES-1944 | deterministic 結果（§18.3.3 の静的解析）と agent / human の判断結果を区別して保存・… | 基本設計(basic_design) | 決定論的結果とagent/humanの判断結果を区別して保存・表示するという責務分離の決定。 | high |
| DES-1952 | 値域は `test-semantic` / `impl-consistency` / `case-coverage` で… | 詳細設計(design) | judgment_kind/subjectの値域という型レベルの契約。 | high |
| DES-1954 | `case-coverage` は §11 の判断対象であって基本仕様 §5 の 4 検査ではない。 | 基本仕様(spec) | 判断記録プロトコルの位置付け・旧モデルとの関係を述べる粗粒度の概念規定。 | high |
| DES-1976 | §5 の 4 検査を再実施した結果は `PASS` / `FAIL` / `MISMATCH` / `NO_EVIDEN… | 基本仕様(spec) | 判断記録プロトコルの位置付け・旧モデルとの関係を述べる粗粒度の概念規定。 | high |
| DES-1977 | 変更そのものが `UNKNOWN` を生成するのではない。 | 基本仕様(spec) | 判断記録プロトコルの位置付け・旧モデルとの関係を述べる粗粒度の概念規定。 | high |

### 18.3.7 承認と判断記録の分離（デフォルト: 詳細仕様(detailed_spec)）

| id | 文 | 層 | 理由 | confidence |
|---|---|---|---|---|
| DES-1980 | 判断済みと承認済みを区別する（判断済み ≠ 承認済み）。 | 基本仕様(spec) | 承認と判断記録の独立性という粗粒度の概念規定（4軸独立の再掲）または委譲先の指示。 | high |
| DES-1981 | 判断記録と承認記録は同一 entity であることを要求せず、別 entity でありうる。 | 基本仕様(spec) | 承認と判断記録の独立性という粗粒度の概念規定（4軸独立の再掲）または委譲先の指示。 | high |
| DES-1982 | 判断は承認なしでも記録でき、正式採用は承認の別段階である。 | 基本仕様(spec) | 承認と判断記録の独立性という粗粒度の概念規定（4軸独立の再掲）または委譲先の指示。 | high |
| DES-1983 | 承認は検証状態と独立の別軸である。 | 基本仕様(spec) | 承認と判断記録の独立性という粗粒度の概念規定（4軸独立の再掲）または委譲先の指示。 | high |
| DES-1986 | `approved_state` の値域は `approved` / `rejected` / `withdrawn` … | 詳細設計(design) | approved_state等の値域・承認主体の記録fieldという型レベルの契約。 | high |
| DES-1989 | 実効承認の導出は `approved_state` を参照する。 | 詳細設計(design) | 実効承認導出がapproved_state fieldを参照するという内部契約。 | high |
| DES-1999 | 承認対象の値域は VO ID と document ID である。 | 詳細設計(design) | approved_state等の値域・承認主体の記録fieldという型レベルの契約。 | high |
| DES-2004 | 承認レコード生成の正典面は対象種別を引数に取る単一の経路である。 | 基本設計(basic_design) | 承認レコード生成・CLI/MCPコマンド体系（approval create/withdraw/show）という構造上の決定。 | high |
| DES-2005 | `vtest approval create --subject-type <vo\|document\|judgmen… | 基本設計(basic_design) | 承認レコード生成・CLI/MCPコマンド体系（approval create/withdraw/show）という構造上の決定。 | high |
| DES-2006 | 対象種別ごとに別の承認規則・別の承認コマンドを設けない。 | 基本設計(basic_design) | 承認レコード生成・CLI/MCPコマンド体系（approval create/withdraw/show）という構造上の決定。 | high |
| DES-2007 | `vtest approval withdraw <approval-id>` / `approval_withdraw… | 基本設計(basic_design) | 承認レコード生成・CLI/MCPコマンド体系（approval create/withdraw/show）という構造上の決定。 | high |
| DES-2008 | `vtest approval show` / `approval_get` は当該対象の承認レコード一覧と実効承認状態… | 基本設計(basic_design) | 承認レコード生成・CLI/MCPコマンド体系（approval create/withdraw/show）という構造上の決定。 | high |
| DES-2009 | 方針は総称 document として登録した文書で表現し、専用のエンティティ型を設けない。 | 基本仕様(spec) | 承認と判断記録の独立性という粗粒度の概念規定（4軸独立の再掲）または委譲先の指示。 | high |
| DES-2021 | 承認主体は種別（`human` / `agent`）と識別子を記録する。 | 詳細設計(design) | approved_state等の値域・承認主体の記録fieldという型レベルの契約。 | high |
| DES-2022 | 承認権限（approval authority）・承認ロール・必要承認数・権限 schema はプロジェクト設定と別紙A… | 基本仕様(spec) | 承認と判断記録の独立性という粗粒度の概念規定（4軸独立の再掲）または委譲先の指示。 | high |
| DES-2023 | 承認 workflow の状態遷移と `approved_state` の値域は本冊 §3.5 に定める。 | 基本仕様(spec) | 承認と判断記録の独立性という粗粒度の概念規定（4軸独立の再掲）または委譲先の指示。 | high |

### 18.3.8 verify・report と scope（デフォルト: 詳細仕様(detailed_spec)）

| id | 文 | 層 | 理由 | confidence |
|---|---|---|---|---|
| DES-2030 | scope は 2 軸で限定できる。 | 基本設計(basic_design) | scopeが検査軸・エンティティ軸の2軸で構成されるというデータモデル構造の決定。 | high |
| DES-2035 | `verify` / `report` の JSON（CLI・MCP）は最上位に `scope` を返し、`scope.… | 詳細設計(design) | JSON出力のfield構成という型・schemaレベルの契約。 | high |
| DES-2039 | 機能単位の集約は親 VO（子 VO を持つ VO）を単位とし、Feature を別エンティティ・別レコード・別 ID と… | 基本仕様(spec) | Featureを別エンティティとして設けない、旧構造をdocument化で再導出するという粗粒度の概念規定。 | high |
| DES-2043 | NO_EVIDENCE を生む入力（証拠が存在しない／証拠のハッシュが現在の対象と不一致／scope 限定により検査を実… | 基本設計(basic_design) | NO_EVIDENCEを生む入力を受入fixtureで表現するという受入スイート構成の決定。 | low |
| DES-2050 | 旧モデルの SPEC → REQ → VO → Test 構造は総称 document 化により DOC → VO → … | 基本仕様(spec) | Featureを別エンティティとして設けない、旧構造をdocument化で再導出するという粗粒度の概念規定。 | high |
| DES-2054 | text treeのancestor continuation、middle child、last childを一意なb… | 詳細設計(design) | text tree描画記号という表示アルゴリズムレベルの具体的な記述。 | high |

### 18.3.8 verify・report と scope / 判定の決定性（デフォルト: 詳細仕様(detailed_spec)）

| id | 文 | 層 | 理由 | confidence |
|---|---|---|---|---|
| DES-2062 | 評価経路へそのような seam を導入する変更を行う場合は、正反対の判定を返す stub を注入しても 4 検査の結果が… | 詳細設計(design) | 評価経路へseamを導入する変更時のstub注入確認という受入スイート自身の検証手続き。 | high |

### 18.3.8 verify・report と scope / 上流該当箇所の同伴（デフォルト: 詳細仕様(detailed_spec)）

| id | 文 | 層 | 理由 | confidence |
|---|---|---|---|---|
| DES-2065 | 「どの上流条項がどの VO へ対応するか」の対応ペアの取得に新規 CLI コマンド・MCP ツールを用いない（既存の `… | 基本仕様(spec) | 対応ペア取得に新規コマンドを用いないという粗粒度の一般原則。 | high |

### 18.3.9 フェーズゲート評価（デフォルト: 詳細仕様(detailed_spec)）

| id | 文 | 層 | 理由 | confidence |
|---|---|---|---|---|
| DES-2066 | プロジェクト側が登録したフェーズ・工程・ゲートの進行条件について、現在の検証状態（§4.1 の 5 状態）と承認（§18… | 基本仕様(spec) | ゲート機能の位置付け・責務範囲・委譲先を述べる粗粒度の概念規定。 | high |
| DES-2067 | ゲート定義は `config.yaml` の `gates` に、ゲート名と進行条件（`require.verifica… | 詳細設計(design) | config.yamlのgatesスキーマ／--gate指定時のJSON field構成という型・schemaレベルの契約。 | high |
| DES-2069 | 条件充足・不足の両方を fixture で確認する。 | 基本設計(basic_design) | 満足・不足双方をfixtureで確認するという受入スイート構成の決定。 | high |
| DES-2070 | 検証状態と承認は独立の軸であり、ゲートは両者の組合せを進行条件にできる。 | 基本仕様(spec) | ゲート機能の位置付け・責務範囲・委譲先を述べる粗粒度の概念規定。 | high |
| DES-2080 | 順序・包含解釈による充足を認めない fixture を持つ。 | 基本設計(basic_design) | 満足・不足双方をfixtureで確認するという受入スイート構成の決定。 | high |
| DES-2083 | `--gate` を指定した `verify` / `report` の JSON は `data.gate` に `n… | 詳細設計(design) | config.yamlのgatesスキーマ／--gate指定時のJSON field構成という型・schemaレベルの契約。 | high |
| DES-2091 | 責務はゲート条件が現在満たされているかの評価・提示に限る。 | 基本仕様(spec) | ゲート機能の位置付け・責務範囲・委譲先を述べる粗粒度の概念規定。 | high |
| DES-2092 | フェーズのライフサイクル管理・工程の自動遷移は責務外とする。 | 基本仕様(spec) | ゲート機能の位置付け・責務範囲・委譲先を述べる粗粒度の概念規定。 | high |
| DES-2093 | 新規 CLI コマンド・MCP ツールを増やさず、既存の `vtest verify` の `--gate` 引数と出力… | 基本仕様(spec) | ゲート機能の位置付け・責務範囲・委譲先を述べる粗粒度の概念規定。 | high |
| DES-2094 | 具体的なフェーズ名・承認ロール・必要承認数はプロジェクト設定と別紙A へ委譲する。 | 基本仕様(spec) | ゲート機能の位置付け・責務範囲・委譲先を述べる粗粒度の概念規定。 | high |

### 18.3.10 Structured Test Operation（デフォルト: 詳細仕様(detailed_spec)）

| id | 文 | 層 | 理由 | confidence |
|---|---|---|---|---|
| DES-2095 | Form `kind`は`[a-z0-9][a-z0-9-]*`のcase-sensitive文字列で、built-in… | 詳細設計(design) | Form kindの文字列形式という型レベルの契約。 | high |

### 18.3.12 adapter contract（デフォルト: 詳細仕様(detailed_spec)）

| id | 文 | 層 | 理由 | confidence |
|---|---|---|---|---|
| DES-2118 | `vtest-adapter-api`は言語・runner非依存であり、Cargo、Rust parser、llvm-c… | 詳細設計(design) | vtest-adapter-api / vtest-modelの型定義（フィールドと意味）という型レベルの契約。 | high |
| DES-2119 | `vtest-model::TestEntity`はTestを関数として表現せず、adapter所有のTest cons… | 詳細設計(design) | vtest-adapter-api / vtest-modelの型定義（フィールドと意味）という型レベルの契約。 | high |
| DES-2120 | `TargetRef::Locator`はadapter IDとadapter所有のopaque locatorを保持す… | 詳細設計(design) | vtest-adapter-api / vtest-modelの型定義（フィールドと意味）という型レベルの契約。 | high |
| DES-2121 | `SourceLocation`はadapter ID、project-relative path、opaque loc… | 詳細設計(design) | vtest-adapter-api / vtest-modelの型定義（フィールドと意味）という型レベルの契約。 | high |
| DES-2122 | `TargetRef::Locator`と`SourceLocation`のどちらもRust module path、関… | 詳細設計(design) | vtest-adapter-api / vtest-modelの型定義（フィールドと意味）という型レベルの契約。 | high |
| DES-2123 | `vtest-model::TestEntity`は`ExecutionDescriptor`だけを実行座標として持ち、… | 詳細設計(design) | vtest-adapter-api / vtest-modelの型定義（フィールドと意味）という型レベルの契約。 | high |
| DES-2126 | `SourceDiscoveryAdapter`はhash未計算DTOを返し、coreがDTO検証・hash計算・dom… | 詳細設計(design) | coreがDTO検証・hash計算・domain entity具体化をこの順で行うという内部処理順の記述。 | high |
| DES-2127 | `rust-cargo` adapterはRust discovery、static audit、Structured … | 基本設計(basic_design) | rust-cargo adapterの責務境界（何を所有するか）という構造上の決定。 | high |
| DES-2128 | `vtest-scan`はadapter discoveryの委譲・出力検証・決定論的統合・core record整合性… | 基本設計(basic_design) | vtest-scanがsyn::parse_file等のRust固有APIを所有しないという責務境界（言語固有の記述を含む）。 | high |

## (d) code_like: true の文

| id | 文 | 層 | 理由 |
|---|---|---|---|
| DES-1693 | Rust workspaceの受入テストは`cargo test --workspace`で実行できる。 | 詳細設計(design) | 受入テストの実行手段としてcargo test --workspaceを名指す、Rust/cargo固有の記述。 |
| DES-1741 | coreは出力を検証してTest subject / Source Target hashを計算してからManaged Test Entity、ManagedTestLink、Source Targetを具体化する。 | 詳細設計(design) | coreが検証→hash計算→具体化という処理順序で行うという内部手続き順の記述。 |
| DES-2054 | text treeのancestor continuation、middle child、last childを一意なbranch記号で描画する。 | 詳細設計(design) | text tree描画記号という表示アルゴリズムレベルの具体的な記述。 |
| DES-2126 | `SourceDiscoveryAdapter`はhash未計算DTOを返し、coreがDTO検証・hash計算・domain entity具体化をこの順で行う。 | 詳細設計(design) | coreがDTO検証・hash計算・domain entity具体化をこの順で行うという内部処理順の記述。 |
| DES-2128 | `vtest-scan`はadapter discoveryの委譲・出力検証・決定論的統合・core record整合性を所有し、`*.rs`列挙、`syn::parse_file`、`#[test]`抽出、doc comment parseを所有しない。 | 基本設計(basic_design) | vtest-scanがsyn::parse_file等のRust固有APIを所有しないという責務境界（言語固有の記述を含む）。 |

## confidence: low の文（一覧）

| id | 文 | 層 | 理由 |
|---|---|---|---|
| DES-1752 | `TargetRef::SrcId`をcanonical targetとして返したadapter出力はmalformed adapter outputとして拒否する。 | 詳細設計(design) | adapter出力のmalformed判定という内部契約（design/detailed_spec境界が曖昧）。 |
| DES-1757 | Source Target hashも変化することはsourceが実際に変化したことの帰結として正しい挙動であり、恒久SRC IDが独立したhash fieldであることを意味しない。 | 詳細仕様(detailed_spec) | hash変化の帰結を説明する補足文で、単独の受入条件としては論証的。 |
| DES-1767 | adapter所有のmetadata宣言、ID、target、VO参照、record schema、Relationの違反を対応診断codeで検出する。 | 基本仕様(spec) | 違反検出全般を述べる粗粒度の一般能力の約束。 |
| DES-1836 | 旧モデルの W-SCAN-102（孤立 VO）は VO 層の警告であり、文書層 `orphan_detection` とは別物として存置する。 | 基本仕様(spec) | 旧モデルとの関係を述べる粗粒度の整理。 |
| DES-1837 | DA-001〜DA-006とW-DA-101は本冊§7の判定条件に従う。 | 基本仕様(spec) | 本冊§7への準拠を指し示す粗粒度の参照。 |
| DES-1928 | バンドルは派生情報であり Git 管理しない。 | 基本設計(basic_design) | バンドルを派生情報としてGit管理しないという永続化方針の決定。 |
| DES-2043 | NO_EVIDENCE を生む入力（証拠が存在しない／証拠のハッシュが現在の対象と不一致／scope 限定により検査を実施しなかった項目）を受入で表現する。 | 基本設計(basic_design) | NO_EVIDENCEを生む入力を受入fixtureで表現するという受入スイート構成の決定。 |
| DES-2049 | report は DOC → VO → Test の構造と、各非PASSの根拠（判断記録・Evidence への参照）を text / JSON で返す。 | 詳細仕様(detailed_spec) | reportの出力内容（DOC→VO→Test構造と非PASS根拠）を述べるが、構造規定（basic_design）とも読める。 |

## (e) 直接観察

「受入条件・検証可能な期待結果 → 詳細仕様」という Owner 参照文書の原則は、実測でも支持された。463文中323文（約70%）が detailed_spec に置かれ、大半は「特定の入力・状態・条件が決まればE-SCAN-*/E-*/W-*コードと5状態・4診断ラベルの値が一意に決まる」という条件文の形をしていた。一方で、22節すべてが detailed_spec 一色になったわけではない。§18.2（共通fixture、41文）は「fixtureは…を表現できる」という受入プロジェクトの構成物定義が支配的で basic_design（34文）が多数を占め、§18.3.1冒頭（discovery・record・graph、28文）と§18.3.12（adapter contract、28文）は型・field・値域の宣言（designがそれぞれ7文）が目立った。§18.1（共通条件）と§18.3.8内「判定の決定性」の一部は、受入スイート自身の運用（cargo test --workspaceでの実行、stub注入によるseam不在確認）を述べており design に置いた。`code_like: true`は5文のみで、いずれも特定言語・特定コマンド・特定描画アルゴリズムを名指す文（cargo test --workspace、core内部の処理順、text tree描画記号、Rust固有API名）に限られ、大部分の文は実装非依存の条件文だった。`confidence: low`は8文で、旧モデルとの関係を説明する移行注記や、単一の受入条件として独立して読みにくい補足説明文に集中した。
