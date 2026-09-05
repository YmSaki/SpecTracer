# 基本仕様 層仕分け 試験結果（パイロット・六層版）

対象: `docs/canonical/specification.json` の `spec` 配列（基本仕様の全765文、SPEC-001〜SPEC-765）。
判定基準: `docs/canonical/LAYERING.md` §2（六層版。判定1→1a→2→3）を、節ごとの多数決で既定の層を決めたうえで文単位に適用した。

**六層への改訂について**: 当初の試験（五層版）では基本仕様と基本設計の2層で仕分けたが、Ownerの指示により基本仕様と基本設計の間に詳細仕様（`detailed_spec`）が挿入された。本書は「外から見てどう振る舞うか」（主要な機能・入出力・利用条件）を答える基本仕様（`spec`）と、「各条件で正確にどう振る舞うか」（境界値・状態ごと・優先順位・受入条件・不変条件、「入力・状態・条件が決まれば結果が一意」の精度）を答える詳細仕様（`detailed_spec`）を分けて再仕分けした。basic_design（132文）とdesign（4文）の判定は五層版から変更していない。

## (a) 節ごとの既定の層

既定の層は節内の多数決（最頻値）で自動算出した。同数の場合はCounterの出現順で決まる（恣意的だが再現可能）。

| 節 | 既定の層 | 文数 | 例外数 |
|---|---|---:|---:|
| 0. 本書の位置付け | spec | 22 | 7 |
| 1. 用語定義 | spec | 47 | 0 |
| 2.1 正典の三層構造 | spec | 12 | 3 |
| 2.2 宣言鎖と照合 | spec | 12 | 0 |
| 2.3 導出できる関係は保存しない | spec | 7 | 0 |
| 2.4 adapter 設定と wire 互換 | basic_design | 20 | 11 |
| 3.1 エンティティ種別 | spec | 23 | 8 |
| 3.2 ID 規則と関係リンク | basic_design | 21 | 13 |
| 3.3 Source Target の識別 | spec | 9 | 3 |
| 4.1 状態は 5 つ | spec | 13 | 5 |
| 4.2 診断ラベル | spec | 4 | 0 |
| 4.3 状態の割当 | detailed_spec | 9 | 1 |
| 4.4 UNKNOWN の検疫 | spec | 3 | 0 |
| 4.5 検証状態と承認の分離 | spec | 7 | 3 |
| 4.6 scope | detailed_spec | 7 | 2 |
| 5. 検査 | spec | 7 | 1 |
| 5.1 chain_integrity — 宣言鎖の完全性 | detailed_spec | 11 | 3 |
| 5.2 orphan_detection — 文書層の孤児検出 | detailed_spec | 7 | 3 |
| 5.3 target_binding — 宣言対象の振る舞いの実現 | detailed_spec | 21 | 6 |
| 5.4 oracle_presence — 照合装置の存在 | spec | 12 | 6 |
| 5.5 決定論的に検出可能な不成立構造 | detailed_spec | 12 | 6 |
| 6. 証拠 | detailed_spec | 13 | 5 |
| 7. 判定権威 | spec | 9 | 1 |
| 8.1 成立と算入の独立 | detailed_spec | 4 | 2 |
| 8.2 成立性の必要条件 | detailed_spec | 12 | 3 |
| 8.3 決定論的に検出可能な不成立構造 | spec | 7 | 2 |
| 9.1 検証対象 | detailed_spec | 8 | 3 |
| 9.2 Source Target の識別 | spec | 10 | 5 |
| 9.3 実装 traceability | spec | 8 | 2 |
| 10. Verification Obligation | spec | 18 | 3 |
| 11. 発見・意味判定のエスカレーションと判断記録 | spec | 3 | 0 |
| 11.1 データ形態の提供 | spec | 5 | 0 |
| 11.2 エスカレーション | spec | 3 | 0 |
| 11.3 判断の記録と再検証 | detailed_spec | 22 | 10 |
| 12. Test Registry | detailed_spec | 17 | 4 |
| 13. Test Intent | spec | 4 | 0 |
| 14. Parameterized / Table-Driven Test | spec | 5 | 1 |
| 15. Structured Test Operation | basic_design | 5 | 0 |
| 15.1 desired state 方式 | spec | 4 | 2 |
| 15.2 入力検証 | detailed_spec | 2 | 0 |
| 15.3 編集境界 | spec | 9 | 3 |
| 15.4 Form Schema | basic_design | 11 | 7 |
| 16. 仕様入力（文書層） | spec | 5 | 0 |
| 17. 承認 | spec | 27 | 12 |
| 18. 途中導入と既存プロジェクト対応 | spec | 2 | 0 |
| 18.1 既存資産の可視化 | basic_design | 10 | 5 |
| 18.2 導入時の責務境界 | spec | 5 | 0 |
| 18.3 判断待ち情報の構造化 | spec | 2 | 0 |
| 18.4 導入難度の規模非依存 | spec | 3 | 0 |
| 19. トレーサビリティと役割別 projection | spec | 15 | 4 |
| 20. フェーズゲートと進行条件 | spec | 8 | 0 |
| 21. テスト実行と Execution Evidence | detailed_spec | 9 | 2 |
| 21.1 Evidence の鮮度（ハッシュ束縛による設計制約） | detailed_spec | 18 | 4 |
| 22.1 完全検証 OK | detailed_spec | 7 | 3 |
| 22.2 集約 | spec | 6 | 3 |
| 22.3 報告 | spec | 13 | 6 |
| 23. スキャンと整合性検査 | detailed_spec | 22 | 7 |
| 24.1 `.verify/` ディレクトリ | basic_design | 13 | 0 |
| 24.2 並列編集耐性の設計原則 | basic_design | 17 | 8 |
| 24.3 派生情報の再構築 | spec | 4 | 1 |
| 25. 利用者別ユースケース | spec | 10 | 0 |
| 26. インターフェース概要 | basic_design | 3 | 1 |
| 26.1 CLI コマンド体系 | basic_design | 25 | 6 |
| 26.2 MCP ツール体系 | basic_design | 21 | 6 |
| 27. 対応範囲と adapter 境界 | spec | 10 | 5 |
| 28. 非機能要求への対応方針 | spec | 8 | 0 |
| 29. スコープ外 | spec | 11 | 1 |
| 30. 詳細設計へ委譲する事項 | spec | 26 | 0 |

## (b) 集計

| 層 | 件数 | 割合 |
|---|---:|---:|
| spec | 412 | 53.9% |
| detailed_spec | 217 | 28.4% |
| basic_design | 132 | 17.3% |
| design | 4 | 0.5% |
| **合計** | **765** | 100.0% |

`confidence: low` は **21件**（SPEC-011, SPEC-163, SPEC-298, SPEC-364, SPEC-435, SPEC-436, SPEC-437, SPEC-438, SPEC-439, SPEC-440, SPEC-456, SPEC-614, SPEC-649, SPEC-658, SPEC-666, SPEC-685, SPEC-708, SPEC-717, SPEC-718, SPEC-720, SPEC-739）。30件を大きく下回るため、規則の再定義は不要と判断した。うち8件（SPEC-364, 440, 614, 685, 708, 717, 718, 720）はspecとdetailed_specの境界、残り13件は五層版からの持ち越し（spec/basic_design/designの境界）。
`code_like: true` は五層版から変わらず**0件**。

## (c) 例外一覧（節の既定層と異なる文）

既定層と異なる層に置いた文は208件。節番号順、節内はID順。

| ID | 文（先頭60字） | 層 | 理由 | 確信度 |
|---|---|---|---|---|
| SPEC-010 | ツール名は `vtest` とする（バイナリ名・ディレクトリ名に使用する）。 | basic_design | ツール名（バイナリ名・ディレクトリ名）は外部から見える命名決定。 | high |
| SPEC-011 | `vtest` 本体の実装言語はRustとする。 | basic_design | 実装言語の選択は外部から見える構成要素選択（採用ツールチェーン）。別実装なら偽になる決定。 | low |
| SPEC-012 | 組込 production adapter は `rust-cargo` とする。 | basic_design | 組込production adapterをrust-cargoとする構成要素選択（adapter/core分界に関わる決定）。 | high |
| SPEC-015 | インターフェースはCLIと、AI Agent向けMCPサーバとする。 | basic_design | インターフェースをCLIとMCPの2種とする、入口の種類の宣言（LAYERING.md PM仮置き例）。 | high |
| SPEC-016 | MCPを本体とする。 | basic_design | MCPを本体とするという入口の主従関係の宣言。 | high |
| SPEC-020 | Rust固有処理は組込 `rust-cargo` adapter が所有する。 | basic_design | Rust固有処理をrust-cargo adapterが所有するという、adapterとcoreの責務分界の明示。 | high |
| SPEC-021 | CLI・MCP・検証coreはadapter registryを介して能力を選択する。 | basic_design | CLI・MCP・検証coreがadapter registryを介して能力選択するという、adapter/core分界とcomposition構造の明示。 | high |
| SPEC-071 | 宣言層は、adapter所有のTest metadata宣言、および.verify/配下のdocument / VO / | basic_design | 宣言層の正典が`.verify/`配下のどのレコードから成るかというディレクトリ/レコード構成の明示。 | high |
| SPEC-073 | 事実層は、実行結果・判断記録・承認記録からなる.verify/配下の追記型レコードファイルであり、Gitで管理される。 | basic_design | 事実層の正典が`.verify/`配下のどのレコードから成るかというディレクトリ/レコード構成の明示。 | high |
| SPEC-076 | source discovery、決定論的解析、Structured Test Operation、test runne | basic_design | discovery/解析/実行起動等をadapter capabilityとして提供するという、adapter/core責務分界の明示。 | high |
| SPEC-104 | readerは読み取りだけで正典を書き換えない。 | spec | 外から使うとどう振る舞うかという主要な機能・定義・スコープ・委譲の記述。実装方法によらず真であり続ける約束。 | high |
| SPEC-106 | adapter IDは設定内で一意でなければならない。 | detailed_spec | adapter設定の入力値制約・拒否条件を精密に規定（一意性・重複可否・未知adapter時の扱い等）。 入力値・必須項目・許容範囲についての精密な制約。詳細仕様の「入力値の範囲・形式・制約」に該当。 | high |
| SPEC-107 | 同一adapter内のroot重複も拒否する。 | detailed_spec | adapter設定の入力値制約・拒否条件を精密に規定（一意性・重複可否・未知adapter時の扱い等）。 入力値・必須項目・許容範囲についての精密な制約。詳細仕様の「入力値の範囲・形式・制約」に該当。 | high |
| SPEC-108 | 異なるadapterが同じrootを走査することは許可する。 | detailed_spec | adapter設定の入力値制約・拒否条件を精密に規定（一意性・重複可否・未知adapter時の扱い等）。 入力値・必須項目・許容範囲についての精密な制約。詳細仕様の「入力値の範囲・形式・制約」に該当。 | high |
| SPEC-109 | 未知のadapterやadapter固有設定の検証失敗は操作エラーとする。 | detailed_spec | adapter設定の入力値制約・拒否条件を精密に規定（一意性・重複可否・未知adapter時の扱い等）。 入力値・必須項目・許容範囲についての精密な制約。詳細仕様の「入力値の範囲・形式・制約」に該当。 | high |
| SPEC-110 | 未知のadapterやadapter固有設定の検証失敗時、利用可能な言語や能力を推測補完しない。 | detailed_spec | adapter設定の入力値制約・拒否条件を精密に規定（一意性・重複可否・未知adapter時の扱い等）。 入力値・必須項目・許容範囲についての精密な制約。詳細仕様の「入力値の範囲・形式・制約」に該当。 | high |
| SPEC-111 | core domainの `TestEntity` は、言語・runner非依存の `execution`（adapte | design | core domainの`TestEntity`が持つfield（execution）の宣言。PMの`SourceLocation`型宣言例と同型の内部型フィールド。 | high |
| SPEC-112 | `filter` / `package` / `test_target` は `TestEntity` のfieldでは | design | `TestEntity`のfieldでないものの宣言。内部型のフィールド境界を定義する型宣言。 | high |
| SPEC-116 | 非Rust Testでは空値・dummy値・Rust既定値を生成しない。 | detailed_spec | adapter設定の入力値制約・拒否条件を精密に規定（一意性・重複可否・未知adapter時の扱い等）。 入力値・必須項目・許容範囲についての精密な制約。詳細仕様の「入力値の範囲・形式・制約」に該当。 | high |
| SPEC-119 | 欠落・矛盾時は入力を拒否する。 | detailed_spec | adapter設定の入力値制約・拒否条件を精密に規定（一意性・重複可否・未知adapter時の扱い等）。 入力値・必須項目・許容範囲についての精密な制約。詳細仕様の「入力値の範囲・形式・制約」に該当。 | high |
| SPEC-120 | 欠落・矛盾時は推測で実行可能として扱わない。 | detailed_spec | adapter設定の入力値制約・拒否条件を精密に規定（一意性・重複可否・未知adapter時の扱い等）。 入力値・必須項目・許容範囲についての精密な制約。詳細仕様の「入力値の範囲・形式・制約」に該当。 | high |
| SPEC-121 | documentのIDは `DOC-` とし、正典は `.verify/doc/` に置く。 | basic_design | documentのIDプレフィックスと正典格納パスの割当（レコードの種類）。 | high |
| SPEC-124 | Verification ObligationのIDは `VO-` とし、正典は `.verify/vo/` に置く。 | basic_design | VOのIDプレフィックスと正典格納パスの割当。 | high |
| SPEC-127 | TestのIDは `TEST-` とし、正典はadapter所有のTest metadata宣言とする。 | basic_design | TestのIDプレフィックスと正典所在の割当。 | high |
| SPEC-129 | Source TargetはIDを持たない、または任意で `SRC-` を用い、adapter IDとopaque lo | basic_design | Source TargetのID方式（SRC-接頭辞・adapter ID+opaque locator）の割当。 | high |
| SPEC-132 | RelationのIDは `REL-`（ULID）とし、正典は `.verify/rel/` に置く。 | basic_design | RelationのIDプレフィックス（REL-/ULID）と正典格納パスの割当。 | high |
| SPEC-135 | 判断記録のIDはULIDとし、正典は `.verify/decisions/` に置く。 | basic_design | 判断記録のID方式（ULID）と正典格納パスの割当。 | high |
| SPEC-137 | 承認記録のIDはULIDとし、正典は `.verify/approvals/` に置く。 | basic_design | 承認記録のID方式（ULID）と正典格納パスの割当。 | high |
| SPEC-139 | Execution EvidenceのIDはULIDとし、正典は `.verify/evidence/` に置く。 | basic_design | Execution EvidenceのID方式（ULID）と正典格納パスの割当。 | high |
| SPEC-149 | ツールはID形式を強制せず一意性のみを強制する。 | spec | 外から使うとどう振る舞うかという主要な機能・定義・スコープ・委譲の記述。実装方法によらず真であり続ける約束。 | high |
| SPEC-150 | IDの一意性はスキャン時に全数検査する。 | spec | 外から使うとどう振る舞うかという主要な機能・定義・スコープ・委譲の記述。実装方法によらず真であり続ける約束。 | high |
| SPEC-151 | ID衝突は `chain_integrity` の非 `PASS`（`MISMATCH`）とする（§5.1、§23）。 | detailed_spec | ID衝突・空の説明文などの個別事例に対する精密な扱い（状態割当・受理可否）。 特定条件から特定の状態/結果への精密な割当。「入力・状態・条件が決まれば結果が一意に決まる」水準のためdetailed_spec。 | high |
| SPEC-152 | 任意の恒久SRC IDはadapter namespaceを持たないためrepository全体で一意とする。 | detailed_spec | ID衝突・空の説明文などの個別事例に対する精密な扱い（状態割当・受理可否）。 特定条件から特定の状態/結果への精密な割当。「入力・状態・条件が決まれば結果が一意に決まる」水準のためdetailed_spec。 | high |
| SPEC-153 | 恒久SRC IDの衝突は曖昧参照として受理しない。 | detailed_spec | ID衝突・空の説明文などの個別事例に対する精密な扱い（状態割当・受理可否）。 特定条件から特定の状態/結果への精密な割当。「入力・状態・条件が決まれば結果が一意に決まる」水準のためdetailed_spec。 | high |
| SPEC-154 | 恒久SRC IDの衝突時、どのSource Targetを指すか推測しない。 | detailed_spec | ID衝突・空の説明文などの個別事例に対する精密な扱い（状態割当・受理可否）。 特定条件から特定の状態/結果への精密な割当。「入力・状態・条件が決まれば結果が一意に決まる」水準のためdetailed_spec。 | high |
| SPEC-155 | 関係リンクは説明文・導出理由を任意（optional）で保持できる。 | spec | 外から使うとどう振る舞うかという主要な機能・定義・スコープ・委譲の記述。実装方法によらず真であり続ける約束。 | high |
| SPEC-156 | derives_from・covers・検証対象・実装traceabilityなど性質の異なる関係型は潰さず区別する。 | spec | 外から使うとどう振る舞うかという主要な機能・定義・スコープ・委譲の記述。実装方法によらず真であり続ける約束。 | high |
| SPEC-157 | 存在するリンクに付す説明文は空でもよい。 | detailed_spec | ID衝突・空の説明文などの個別事例に対する精密な扱い（状態割当・受理可否）。 特定条件から特定の状態/結果への精密な割当。「入力・状態・条件が決まれば結果が一意に決まる」水準のためdetailed_spec。 | high |
| SPEC-158 | 説明文が空であることを理由に `chain_integrity` 違反・`MISMATCH` としてはならない。 | detailed_spec | ID衝突・空の説明文などの個別事例に対する精密な扱い（状態割当・受理可否）。 特定条件から特定の状態/結果への精密な割当。「入力・状態・条件が決まれば結果が一意に決まる」水準のためdetailed_spec。 | high |
| SPEC-159 | 関係型そのものの意味論的増殖は求めない。 | spec | 外から使うとどう振る舞うかという主要な機能・定義・スコープ・委譲の記述。実装方法によらず真であり続ける約束。 | high |
| SPEC-163 | ULID payloadにより並列生成時のファイル名衝突を実用上排除する。 | spec | ULIDのpayload特性による衝突回避という説明。ID方式決定（basic_design寄り）か、性質の約束（spec）か境界が曖昧。 | low |
| SPEC-164 | 関係リンクの任意説明文・役割別projectionの保存形式・presetは詳細設計へ委譲する。 | spec | 外から使うとどう振る舞うかという主要な機能・定義・スコープ・委譲の記述。実装方法によらず真であり続ける約束。 | high |
| SPEC-168 | opaque locatorの構文と恒久SRC IDの宣言方法はadapterが定める。 | basic_design | opaque locator構文と恒久SRC ID宣言方法をadapterが定めるという、adapter/core責務分界の明示。 | high |
| SPEC-172 | Test→SRCの対応はadapter所有のTest metadata宣言から提供する。 | basic_design | Test→SRC対応をadapter所有のTest metadata宣言から提供するという、adapter/core責務分界の明示。 | high |
| SPEC-173 | SRC→Testの逆引きはスキャン結果から提供する。 | basic_design | SRC→Testの逆引きをスキャン結果（core側）から提供するという、adapter/core責務分界の明示。 | high |
| SPEC-178 | 完全検証において `PASS` はOKとする。 | detailed_spec | 5状態それぞれについて「完全検証でOKか」を精密に定める受入条件（§22.1の集約OK条件そのもの）。 検証成立性・受入の必要条件を精密に列挙。詳細仕様の「受入条件・検証可能な期待結果」に該当。 | high |
| SPEC-180 | 完全検証において `FAIL` はOKとしない。 | detailed_spec | 5状態それぞれについて「完全検証でOKか」を精密に定める受入条件（§22.1の集約OK条件そのもの）。 検証成立性・受入の必要条件を精密に列挙。詳細仕様の「受入条件・検証可能な期待結果」に該当。 | high |
| SPEC-182 | 完全検証において `MISMATCH` はOKとしない。 | detailed_spec | 5状態それぞれについて「完全検証でOKか」を精密に定める受入条件（§22.1の集約OK条件そのもの）。 検証成立性・受入の必要条件を精密に列挙。詳細仕様の「受入条件・検証可能な期待結果」に該当。 | high |
| SPEC-184 | 完全検証において `NO_EVIDENCE` はOKとしない。 | detailed_spec | 5状態それぞれについて「完全検証でOKか」を精密に定める受入条件（§22.1の集約OK条件そのもの）。 検証成立性・受入の必要条件を精密に列挙。詳細仕様の「受入条件・検証可能な期待結果」に該当。 | high |
| SPEC-186 | 完全検証において `UNKNOWN` はOKとしない。 | detailed_spec | 5状態それぞれについて「完全検証でOKか」を精密に定める受入条件（§22.1の集約OK条件そのもの）。 検証成立性・受入の必要条件を精密に列挙。詳細仕様の「受入条件・検証可能な期待結果」に該当。 | high |
| SPEC-191 | 要件定義 §5.3 の割当をそのまま採用する。 | spec | 外から使うとどう振る舞うかという主要な機能・定義・スコープ・委譲の記述。実装方法によらず真であり続ける約束。 | high |
| SPEC-206 | 技術的に `PASS` であっても未承認である状態を許容する。 | detailed_spec | 承認と検証状態の間で許される/禁じられる遷移を精密に規定する境界条件。 競合する条件間の優先順位・境界条件の精密な規定。詳細仕様の該当項目そのもの。 | high |
| SPEC-207 | 未承認であることだけを理由に `PASS` を `UNKNOWN` 等へ変更してはならない。 | detailed_spec | 承認と検証状態の間で許される/禁じられる遷移を精密に規定する境界条件。 競合する条件間の優先順位・境界条件の精密な規定。詳細仕様の該当項目そのもの。 | high |
| SPEC-208 | 承認済みであることを理由に `FAIL` / `MISMATCH` / `NO_EVIDENCE` / `UNKNOWN | detailed_spec | 承認と検証状態の間で許される/禁じられる遷移を精密に規定する境界条件。 競合する条件間の優先順位・境界条件の精密な規定。詳細仕様の該当項目そのもの。 | high |
| SPEC-210 | 検査軸は実施する検査（4本の部分集合）を指定する。 | spec | 外から使うとどう振る舞うかという主要な機能・定義・スコープ・委譲の記述。実装方法によらず真であり続ける約束。 | high |
| SPEC-211 | エンティティ軸は対象とするdocument / VO / Testの部分木を指定する。 | spec | 外から使うとどう振る舞うかという主要な機能・定義・スコープ・委譲の記述。実装方法によらず真であり続ける約束。 | high |
| SPEC-221 | 答えは検証方法・実行形態に依らず同一でなければならない。 | detailed_spec | 検査結果が証拠源・実行形態に依らず同一であるべきという精密な不変条件。 個別事例・境界条件にまで踏み込んだ不変条件。基本仕様の主要振る舞い記述を超える精度。 | high |
| SPEC-224 | chain_integrityの問いは、宣言鎖のすべてのリンクが存在し、ハッシュ照合が成立するかである。 | spec | 外から使うとどう振る舞うかという主要な機能・定義・スコープ・委譲の記述。実装方法によらず真であり続ける約束。 | high |
| SPEC-232 | どのリンクで切れたかは診断ラベルで示す。 | spec | 外から使うとどう振る舞うかという主要な機能・定義・スコープ・委譲の記述。実装方法によらず真であり続ける約束。 | high |
| SPEC-234 | すべてのTestを管理対象とすることと、当該Testを仕様適合の証拠として算入すること（§8）は別個の条件とする。 | spec | 外から使うとどう振る舞うかという主要な機能・定義・スコープ・委譲の記述。実装方法によらず真であり続ける約束。 | high |
| SPEC-235 | orphan_detectionの問いは、親を持たない `document` ノードが存在するかである。 | spec | 外から使うとどう振る舞うかという主要な機能・定義・スコープ・委譲の記述。実装方法によらず真であり続ける約束。 | high |
| SPEC-237 | 根の指定方式は `.verify/` 設定における明示的な根指定として保持する。 | basic_design | 根の指定方式を`.verify/`設定に保持するという、外部設定ファイル構成の決定。 | high |
| SPEC-238 | 根の指定の具体構文は詳細設計へ委譲する。 | spec | 外から使うとどう振る舞うかという主要な機能・定義・スコープ・委譲の記述。実装方法によらず真であり続ける約束。 | high |
| SPEC-242 | target_bindingの問いは、そのTestが検証対象とする振る舞いが実際に生じ、その振る舞いを反映した観測が得ら | spec | 外から使うとどう振る舞うかという主要な機能・定義・スコープ・委譲の記述。実装方法によらず真であり続ける約束。 | high |
| SPEC-244 | テストランナーの `PASS`/`FAIL` は判定権威（§7）の証拠として消費する。 | spec | 外から使うとどう振る舞うかという主要な機能・定義・スコープ・委譲の記述。実装方法によらず真であり続ける約束。 | high |
| SPEC-245 | target_binding検査は、その証拠が検証対象の実行を伴ったかを問う。 | spec | 外から使うとどう振る舞うかという主要な機能・定義・スコープ・委譲の記述。実装方法によらず真であり続ける約束。 | high |
| SPEC-246 | target_bindingは一つの問いに対し静的解析と動的計測の2つの証拠源を持つ。 | spec | 外から使うとどう振る舞うかという主要な機能・定義・スコープ・委譲の記述。実装方法によらず真であり続ける約束。 | high |
| SPEC-257 | 他の実行形態における確認方法は、当該形態に適した方法として詳細設計で定める。 | spec | 外から使うとどう振る舞うかという主要な機能・定義・スコープ・委譲の記述。実装方法によらず真であり続ける約束。 | high |
| SPEC-258 | 特定形態の確認方法を別形態のTestへ一律要求しない。 | spec | 外から使うとどう振る舞うかという主要な機能・定義・スコープ・委譲の記述。実装方法によらず真であり続ける約束。 | high |
| SPEC-264 | 不成立が構造から証明できる（どんな宣言の下でも不成立を検出できない＝失敗し得ない、または失敗が検証対象の振る舞いに依存し | detailed_spec | oracle_presenceの3値分岐（FAIL/成立/UNKNOWN）と境界条件を精密に規定。 特定条件から特定の状態/結果への精密な割当。「入力・状態・条件が決まれば結果が一意に決まる」水準のためdetailed_spec。 | high |
| SPEC-265 | 照合装置の存在が決定論的に確認できる場合、oracle_presence検査は成立側とする。 | detailed_spec | oracle_presenceの3値分岐（FAIL/成立/UNKNOWN）と境界条件を精密に規定。 特定条件から特定の状態/結果への精密な割当。「入力・状態・条件が決まれば結果が一意に決まる」水準のためdetailed_spec。 | high |
| SPEC-266 | 不成立が構造から証明できることと照合装置の存在が決定論的に確認できることのどちらも決定論的に言えない（解析不能等）場合、 | detailed_spec | oracle_presenceの3値分岐（FAIL/成立/UNKNOWN）と境界条件を精密に規定。 特定条件から特定の状態/結果への精密な割当。「入力・状態・条件が決まれば結果が一意に決まる」水準のためdetailed_spec。 | high |
| SPEC-268 | 静的解析は成立条件から明確に外れるTestを決定論的に検出し、外部監査へ送る前に拒否する（§8）。 | detailed_spec | oracle_presenceの3値分岐（FAIL/成立/UNKNOWN）と境界条件を精密に規定。 特定条件から特定の状態/結果への精密な割当。「入力・状態・条件が決まれば結果が一意に決まる」水準のためdetailed_spec。 | high |
| SPEC-270 | 証明の失敗は `UNKNOWN` の事由ではない。 | detailed_spec | oracle_presenceの3値分岐（FAIL/成立/UNKNOWN）と境界条件を精密に規定。 特定条件から特定の状態/結果への精密な割当。「入力・状態・条件が決まれば結果が一意に決まる」水準のためdetailed_spec。 | high |
| SPEC-273 | 答えはassertの所在・実行形態（内部construct検証か境界の振る舞い検証か）に依らず同一でなければならない。 | detailed_spec | oracle_presenceの3値分岐（FAIL/成立/UNKNOWN）と境界条件を精密に規定。 特定条件から特定の状態/結果への精密な割当。「入力・状態・条件が決まれば結果が一意に決まる」水準のためdetailed_spec。 | high |
| SPEC-275 | `rust-cargo` adapterのStatic Audit capabilityは、§8.3の不成立構造を決定論 | spec | 外から使うとどう振る舞うかという主要な機能・定義・スコープ・委譲の記述。実装方法によらず真であり続ける約束。 | high |
| SPEC-280 | 判定は保守的に行う。 | spec | 外から使うとどう振る舞うかという主要な機能・定義・スコープ・委譲の記述。実装方法によらず真であり続ける約束。 | high |
| SPEC-283 | coreはadapter固有のAST・assertion構文・call graphを解釈しない。 | basic_design | coreがadapter固有のAST・assertion構文・call graphを解釈しないという、adapter/core責務分界の明示。 | high |
| SPEC-284 | coreは正規化されたルール結果を検証・集約する。 | basic_design | coreが正規化されたルール結果のみを検証・集約するという、adapter/core責務分界の明示。 | high |
| SPEC-285 | code fragmentの具体構文はadapterの言語・runnerに従う。 | basic_design | code fragmentの具体構文はadapterの言語・runnerに従うという、adapter/core責務分界の明示。 | high |
| SPEC-286 | 共通契約がRust構文を要求しない。 | spec | 外から使うとどう振る舞うかという主要な機能・定義・スコープ・委譲の記述。実装方法によらず真であり続ける約束。 | high |
| SPEC-287 | 証拠は検証対象の内容ハッシュに束縛される。 | spec | 外から使うとどう振る舞うかという主要な機能・定義・スコープ・委譲の記述。実装方法によらず真であり続ける約束。 | high |
| SPEC-293 | 鮮度の独立検査は設けない。 | spec | 外から使うとどう振る舞うかという主要な機能・定義・スコープ・委譲の記述。実装方法によらず真であり続ける約束。 | high |
| SPEC-297 | adapterはsource range・source bytes・解析した論理metadata・実行座標をhash未計 | design | adapterが返す discovery DTO（hash未計算）という内部データ受け渡しの型宣言。rule2のDTOに直接該当。 | high |
| SPEC-298 | coreが言語非依存の正規化規則でsubject hashを計算してからTest Entityを具体化する。 | design | coreがsubject hashを計算してからTest Entityを具体化するという、内部処理の確定順序（データ流れ）。 | low |
| SPEC-299 | adapterが最終内容ハッシュを自己確定してはならない。 | basic_design | adapterが最終内容ハッシュを自己確定してはならないという、adapter/core責務分界の明示。 | high |
| SPEC-305 | `rust-cargo` adapterにおける判定権威は `cargo test` である。 | basic_design | rust-cargo adapterにおける判定権威を`cargo test`とする、外部ツール（構成要素）の採用決定。 | high |
| SPEC-311 | Testとして成立しているかの検査（§8）と、仕様適合性の証拠として算入するかの判定は独立である。 | spec | 外から使うとどう振る舞うかという主要な機能・定義・スコープ・委譲の記述。実装方法によらず真であり続ける約束。 | high |
| SPEC-312 | 全Testを管理対象とすること（`chain_integrity`）と証拠算入（成立性）は別系統とする。 | spec | 外から使うとどう振る舞うかという主要な機能・定義・スコープ・委譲の記述。実装方法によらず真であり続ける約束。 | high |
| SPEC-318 | 成立条件の確認方法は検証対象・実行形態・観測方法に応じて異なってよい（証明方法への非依存）。 | spec | 外から使うとどう振る舞うかという主要な機能・定義・スコープ・委譲の記述。実装方法によらず真であり続ける約束。 | high |
| SPEC-319 | 特定形態固有の確認方法を別形態へ一律要求しない。 | spec | 外から使うとどう振る舞うかという主要な機能・定義・スコープ・委譲の記述。実装方法によらず真であり続ける約束。 | high |
| SPEC-324 | `oracle_presence` の信頼基盤の具体的範囲（標準assert構文・framework failure s | spec | 外から使うとどう振る舞うかという主要な機能・定義・スコープ・委譲の記述。実装方法によらず真であり続ける約束。 | high |
| SPEC-326 | §8.2の成立条件を満たさないことを、宣言の中身に依らず決定論的に検出できる例はいずれも「どんな宣言の下でも不成立を検出 | detailed_spec | §8.2の不成立検出例に共通する構造的性質を精密に特徴づける。 個別事例・境界条件にまで踏み込んだ不変条件。基本仕様の主要振る舞い記述を超える精度。 | high |
| SPEC-329 | 各adapterは対応する言語・runnerの構造に対して決定論的に判定できる範囲を提供する。 | basic_design | 各adapterが対応言語・runnerの構造に対する決定論的判定範囲を提供するという、adapter/core責務分界の明示。 | high |
| SPEC-333 | 検証対象は、そのTestが検証成立性（§8）を証明しようとする対象、すなわち宣言された「何の時にどうなる」の主語である。 | spec | 外から使うとどう振る舞うかという主要な機能・定義・スコープ・委譲の記述。実装方法によらず真であり続ける約束。 | high |
| SPEC-334 | 検証対象は実装constructに限定しない。 | spec | 外から使うとどう振る舞うかという主要な機能・定義・スコープ・委譲の記述。実装方法によらず真であり続ける約束。 | high |
| SPEC-335 | 外部から観測可能な契約・境界上の振る舞いも検証対象にできる。 | spec | 外から使うとどう振る舞うかという主要な機能・定義・スコープ・委譲の記述。実装方法によらず真であり続ける約束。 | high |
| SPEC-340 | 実装コード上のimplementation constructをSource Targetとして識別可能でなければならな | detailed_spec | Source Target識別の精密な制約（一意性・複数target時の扱い）。 入力値・必須項目・許容範囲についての精密な制約。詳細仕様の「入力値の範囲・形式・制約」に該当。 | high |
| SPEC-342 | 複数targetを宣言した場合も各targetを独立に識別する。 | detailed_spec | Source Target識別の精密な制約（一意性・複数target時の扱い）。 入力値・必須項目・許容範囲についての精密な制約。詳細仕様の「入力値の範囲・形式・制約」に該当。 | high |
| SPEC-345 | 各adapterはSource Targetを一意に解決でき、同一source stateから決定論的に正規化できるTa | basic_design | 各adapterがSource Target解決・正規化を提供するという、adapter/core責務分界の明示。 | high |
| SPEC-348 | 恒久SRC IDを使用する場合、adapter境界を越えてrepository全体で一意でなければならない。 | detailed_spec | Source Target識別の精密な制約（一意性・複数target時の扱い）。 入力値・必須項目・許容範囲についての精密な制約。詳細仕様の「入力値の範囲・形式・制約」に該当。 | high |
| SPEC-349 | 同一SRC IDの複数宣言を曖昧参照として受理しない。 | detailed_spec | Source Target識別の精密な制約（一意性・複数target時の扱い）。 入力値・必須項目・許容範囲についての精密な制約。詳細仕様の「入力値の範囲・形式・制約」に該当。 | high |
| SPEC-352 | traceabilityの存在自体をTest成立性の条件としてはならない。 | detailed_spec | traceabilityがTest成立性の条件にならないことを精密に規定する境界条件。 競合する条件間の優先順位・境界条件の精密な規定。詳細仕様の該当項目そのもの。 | high |
| SPEC-355 | 検証対象と実装traceabilityは、一方から他方を推定してはならない。 | detailed_spec | traceabilityがTest成立性の条件にならないことを精密に規定する境界条件。 競合する条件間の優先順位・境界条件の精密な規定。詳細仕様の該当項目そのもの。 | high |
| SPEC-367 | VOとTestの対応は1:1に限定せず `1:1` / `1:N` / `N:1` / `N:M` を許容する。 | detailed_spec | 許容されるVO-Test対応・組合せcoverage方針を漏れなく列挙。 許容される組合せ・分類を漏れなく列挙。詳細仕様の「すべての条件」に相当。 | high |
| SPEC-371 | 複数軸を持つVOには組合せcoverageの方針を宣言できる（各軸独立／全直積／明示列挙）。 | detailed_spec | 許容されるVO-Test対応・組合せcoverage方針を漏れなく列挙。 許容される組合せ・分類を漏れなく列挙。詳細仕様の「すべての条件」に相当。 | high |
| SPEC-373 | 複数観点を同時確認するTestの存在だけを理由に各観点を独立に証明したことにはしない。 | detailed_spec | 許容されるVO-Test対応・組合せcoverage方針を漏れなく列挙。 許容される組合せ・分類を漏れなく列挙。詳細仕様の「すべての条件」に相当。 | high |
| SPEC-387 | `UNKNOWN` に対して外部（人間または判断可能Agent）が判断できる。 | spec | 外から使うとどう振る舞うかという主要な機能・定義・スコープ・委譲の記述。実装方法によらず真であり続ける約束。 | high |
| SPEC-390 | 判断記録は追跡可能とする。 | spec | 外から使うとどう振る舞うかという主要な機能・定義・スコープ・委譲の記述。実装方法によらず真であり続ける約束。 | high |
| SPEC-392 | 判断記録の理由・根拠・evidence noteは保存できる構造とする。 | spec | 外から使うとどう振る舞うかという主要な機能・定義・スコープ・委譲の記述。実装方法によらず真であり続ける約束。 | high |
| SPEC-395 | 外部の人間/Agentが判断し、判断結果（decision＋任意の理由）を提出する。 | spec | 外から使うとどう振る舞うかという主要な機能・定義・スコープ・委譲の記述。実装方法によらず真であり続ける約束。 | high |
| SPEC-398 | 判断記録の生成・保存の構造化プロトコルは検証状態のゲートではない。 | spec | 外から使うとどう振る舞うかという主要な機能・定義・スコープ・委譲の記述。実装方法によらず真であり続ける約束。 | high |
| SPEC-400 | 判断済みと承認済みは区別する（判断済み ≠ 承認済み）。 | spec | 外から使うとどう振る舞うかという主要な機能・定義・スコープ・委譲の記述。実装方法によらず真であり続ける約束。 | high |
| SPEC-401 | 判断は承認なしでも記録できる。 | spec | 外から使うとどう振る舞うかという主要な機能・定義・スコープ・委譲の記述。実装方法によらず真であり続ける約束。 | high |
| SPEC-402 | 正式採用は§17の別段階である。 | spec | 外から使うとどう振る舞うかという主要な機能・定義・スコープ・委譲の記述。実装方法によらず真であり続ける約束。 | high |
| SPEC-403 | 判断記録と承認記録は同一entityであることを要求しない（別entityでありうる）。 | spec | 外から使うとどう振る舞うかという主要な機能・定義・スコープ・委譲の記述。実装方法によらず真であり続ける約束。 | high |
| SPEC-408 | エスカレーション出力・判断記録の具体的schema、判断待ち情報（§18.3）の構造schemaと取得インターフェース、 | spec | 外から使うとどう振る舞うかという主要な機能・定義・スコープ・委譲の記述。実装方法によらず真であり続ける約束。 | high |
| SPEC-409 | 各Testは安定したTest IDによって識別可能とする。 | spec | 外から使うとどう振る舞うかという主要な機能・定義・スコープ・委譲の記述。実装方法によらず真であり続ける約束。 | high |
| SPEC-410 | Test IDをハンドルとして、Test Intent・`covers`（VO参照）・検証対象・Source Targe | spec | 外から使うとどう振る舞うかという主要な機能・定義・スコープ・委譲の記述。実装方法によらず真であり続ける約束。 | high |
| SPEC-422 | 診断severityと検証状態を混同しない。 | spec | 外から使うとどう振る舞うかという主要な機能・定義・スコープ・委譲の記述。実装方法によらず真であり続ける約束。 | high |
| SPEC-423 | Testの存在理由による分類（role / anchor / anchor_rationale等）と、それに基づく `c | spec | 外から使うとどう振る舞うかという主要な機能・定義・スコープ・委譲の記述。実装方法によらず真であり続ける約束。 | high |
| SPEC-434 | code fragmentの具体構文はadapterの言語・runnerに従う。 | basic_design | code fragmentの具体構文はadapterの言語・runnerに従うという、adapter/core責務分界の明示。 | high |
| SPEC-442 | adapterが現状との差分を計算してTest constructとmetadata宣言を更新する。 | detailed_spec | Create/Edit時のdiff計算・再スキャン検証という精密な内部確定手順。 特定条件から特定の状態/結果への精密な割当。「入力・状態・条件が決まれば結果が一意に決まる」水準のためdetailed_spec。 | high |
| SPEC-443 | coreが結果を再スキャンして検証する。 | detailed_spec | Create/Edit時のdiff計算・再スキャン検証という精密な内部確定手順。 特定条件から特定の状態/結果への精密な割当。「入力・状態・条件が決まれば結果が一意に決まる」水準のためdetailed_spec。 | high |
| SPEC-446 | 公式Edit操作の一回の対象は原則1Testとする。 | detailed_spec | 公式Edit操作の対象範囲を精密に限定する制約。 入力値・必須項目・許容範囲についての精密な制約。詳細仕様の「入力値の範囲・形式・制約」に該当。 | high |
| SPEC-447 | 公式Edit操作は暗黙に他Testを変更しない。 | detailed_spec | 公式Edit操作の対象範囲を精密に限定する制約。 入力値・必須項目・許容範囲についての精密な制約。詳細仕様の「入力値の範囲・形式・制約」に該当。 | high |
| SPEC-448 | 編集はadapterが特定した単一のmetadata宣言範囲とTest construct範囲に限定する。 | detailed_spec | 公式Edit操作の対象範囲を精密に限定する制約。 入力値・必須項目・許容範囲についての精密な制約。詳細仕様の「入力値の範囲・形式・制約」に該当。 | high |
| SPEC-456 | Rust関数単体Test用と小規模結合Test用の組込schemaを同梱する。 | spec | 組込schemaが同梱されるという内容の約束（格納先・命名の決定ではない）。 | low |
| SPEC-458 | CLI・MCPのいずれからも同一schemaを消化できる。 | spec | 外から使うとどう振る舞うかという主要な機能・定義・スコープ・委譲の記述。実装方法によらず真であり続ける約束。 | high |
| SPEC-461 | registryは `kind` からちょうど1件のStructured Test adapterへ解決できる場合だけ操 | detailed_spec | Form Schema解決・拒否条件を精密に規定。 特定条件から特定の状態/結果への精密な割当。「入力・状態・条件が決まれば結果が一意に決まる」水準のためdetailed_spec。 | high |
| SPEC-462 | registryは重複・未知adapter・未対応capability・曖昧な対応を拒否する。 | detailed_spec | Form Schema解決・拒否条件を精密に規定。 特定条件から特定の状態/結果への精密な割当。「入力・状態・条件が決まれば結果が一意に決まる」水準のためdetailed_spec。 | high |
| SPEC-463 | 未知のformをcoreがRust用として推測してはならない。 | detailed_spec | Form Schema解決・拒否条件を精密に規定。 特定条件から特定の状態/結果への精密な割当。「入力・状態・条件が決まれば結果が一意に決まる」水準のためdetailed_spec。 | high |
| SPEC-464 | 境界値・partitionの必須入力化は組込Formでは設けない。 | spec | 外から使うとどう振る舞うかという主要な機能・定義・スコープ・委譲の記述。実装方法によらず真であり続ける約束。 | high |
| SPEC-465 | 境界値・partitionの必須入力化はuser-defined Form Schemaが指定できる。 | spec | 外から使うとどう振る舞うかという主要な機能・定義・スコープ・委譲の記述。実装方法によらず真であり続ける約束。 | high |
| SPEC-478 | 承認は対象自身の内容だけでなく、承認判断が依存する上流文書・上位VOの現在の依存closureへ束縛する。 | detailed_spec | 承認の依存closure構成・必須項目・失効時の再検証・PASS昇格禁止を精密に規定（§17の中核）。 ハッシュ束縛で一致を要求する項目の精密な列挙。詳細仕様の「すべての入力条件」に相当。 | high |
| SPEC-479 | VOの依存closureは、再帰的な上位VO・参照するdocument（およびその上位document）からなる。 | detailed_spec | 承認の依存closure構成・必須項目・失効時の再検証・PASS昇格禁止を精密に規定（§17の中核）。 ハッシュ束縛で一致を要求する項目の精密な列挙。詳細仕様の「すべての入力条件」に相当。 | high |
| SPEC-480 | 対象またはいずれかの依存成果物が変更された承認を、現在の承認済み状態として利用してはならない。 | detailed_spec | 承認の依存closure構成・必須項目・失効時の再検証・PASS昇格禁止を精密に規定（§17の中核）。 ハッシュ束縛で一致を要求する項目の精密な列挙。詳細仕様の「すべての入力条件」に相当。 | high |
| SPEC-481 | 変更後は現在状態に対して検証を再実施する。 | detailed_spec | 承認の依存closure構成・必須項目・失効時の再検証・PASS昇格禁止を精密に規定（§17の中核）。 ハッシュ束縛で一致を要求する項目の精密な列挙。詳細仕様の「すべての入力条件」に相当。 | high |
| SPEC-482 | 変更後の検証結果は§4.1の5状態のいずれかに従う。 | detailed_spec | 承認の依存closure構成・必須項目・失効時の再検証・PASS昇格禁止を精密に規定（§17の中核）。 ハッシュ束縛で一致を要求する項目の精密な列挙。詳細仕様の「すべての入力条件」に相当。 | high |
| SPEC-483 | 依存closureまたはハッシュを欠く承認を推測で有効化してはならない。 | detailed_spec | 承認の依存closure構成・必須項目・失効時の再検証・PASS昇格禁止を精密に規定（§17の中核）。 ハッシュ束縛で一致を要求する項目の精密な列挙。詳細仕様の「すべての入力条件」に相当。 | high |
| SPEC-484 | 承認レコードは読み取り互換のため保持できるが、現在の承認済みを導出してはならない。 | detailed_spec | 承認の依存closure構成・必須項目・失効時の再検証・PASS昇格禁止を精密に規定（§17の中核）。 ハッシュ束縛で一致を要求する項目の精密な列挙。詳細仕様の「すべての入力条件」に相当。 | high |
| SPEC-485 | 承認記録は「誰が（approver）」「何を（subject または judgment reference）」「どの承認 | detailed_spec | 承認の依存closure構成・必須項目・失効時の再検証・PASS昇格禁止を精密に規定（§17の中核）。 ハッシュ束縛で一致を要求する項目の精密な列挙。詳細仕様の「すべての入力条件」に相当。 | high |
| SPEC-486 | 承認記録の根拠は任意（optional）に記録できる。 | detailed_spec | 承認の依存closure構成・必須項目・失効時の再検証・PASS昇格禁止を精密に規定（§17の中核）。 ハッシュ束縛で一致を要求する項目の精密な列挙。詳細仕様の「すべての入力条件」に相当。 | high |
| SPEC-492 | 承認主体は種別（`human` / `agent`）と識別子（エージェント名・モデル名等）を記録する。 | detailed_spec | 承認の依存closure構成・必須項目・失効時の再検証・PASS昇格禁止を精密に規定（§17の中核）。 ハッシュ束縛で一致を要求する項目の精密な列挙。詳細仕様の「すべての入力条件」に相当。 | high |
| SPEC-495 | 承認済みを理由に非 `PASS` を `PASS` へ昇格させない。 | detailed_spec | 承認の依存closure構成・必須項目・失効時の再検証・PASS昇格禁止を精密に規定（§17の中核）。 ハッシュ束縛で一致を要求する項目の精密な列挙。詳細仕様の「すべての入力条件」に相当。 | high |
| SPEC-496 | 未承認を理由に `PASS` を降格させない。 | detailed_spec | 承認の依存closure構成・必須項目・失効時の再検証・PASS昇格禁止を精密に規定（§17の中核）。 ハッシュ束縛で一致を要求する項目の精密な列挙。詳細仕様の「すべての入力条件」に相当。 | high |
| SPEC-500 | 既に大量のソースコードとTestが存在するプロジェクトを検証対象として扱える。 | spec | 外から使うとどう振る舞うかという主要な機能・定義・スコープ・委譲の記述。実装方法によらず真であり続ける約束。 | high |
| SPEC-501 | 既存の文書・Source・Testを読み取り、VOの存在状況・既存TestとVOの対応・Testの不足・検証成立性・宣言 | spec | 外から使うとどう振る舞うかという主要な機能・定義・スコープ・委譲の記述。実装方法によらず真であり続ける約束。 | high |
| SPEC-502 | VOが確定していない範囲を含むプロジェクトも読み取れる。 | spec | 外から使うとどう振る舞うかという主要な機能・定義・スコープ・委譲の記述。実装方法によらず真であり続ける約束。 | high |
| SPEC-503 | 未登録Test・欠落する宣言・未確定のVO・未実施の検査または実行を検証済みとして扱わない（状態は§4.3）。 | detailed_spec | 未登録/欠落/未確定/未実施のいずれも検証済み扱いしないという精密な否定規定。 特定条件から特定の状態/結果への精密な割当。「入力・状態・条件が決まれば結果が一意に決まる」水準のためdetailed_spec。 | high |
| SPEC-507 | document / VO、Test metadata宣言、判断記録、Evidenceの一部が欠ける状態も読み取り可能と | spec | 外から使うとどう振る舞うかという主要な機能・定義・スコープ・委譲の記述。実装方法によらず真であり続ける約束。 | high |
| SPEC-523 | 契約上必須と定義したリンク（`parent --relation--> child`）は必須とする。 | detailed_spec | 必須/任意リンクの区別と説明文欠落時の扱いを精密に規定。 入力値・必須項目・許容範囲についての精密な制約。詳細仕様の「入力値の範囲・形式・制約」に該当。 | high |
| SPEC-524 | 任意（optional）と定義した関係（例：§9.3実装traceability）は欠落してよい。 | detailed_spec | 必須/任意リンクの区別と説明文欠落時の扱いを精密に規定。 入力値・必須項目・許容範囲についての精密な制約。詳細仕様の「入力値の範囲・形式・制約」に該当。 | high |
| SPEC-525 | 存在するリンクに付す説明文・導出理由は任意とする。 | detailed_spec | 必須/任意リンクの区別と説明文欠落時の扱いを精密に規定。 入力値・必須項目・許容範囲についての精密な制約。詳細仕様の「入力値の範囲・形式・制約」に該当。 | high |
| SPEC-526 | 存在するリンクに付す説明文・導出理由は空でも `chain_integrity` 違反・`MISMATCH` としない。 | detailed_spec | 必須/任意リンクの区別と説明文欠落時の扱いを精密に規定。 入力値・必須項目・許容範囲についての精密な制約。詳細仕様の「入力値の範囲・形式・制約」に該当。 | high |
| SPEC-543 | `vtest run` はテストを実際に実行する。 | basic_design | `vtest run`がテストを実行するという、名指しコマンドの責務定義。 | high |
| SPEC-544 | `vtest run` は判定権威（§7）であるランナーの結果をEvidenceとして記録する。 | basic_design | `vtest run`がEvidenceを記録するという、名指しコマンドの責務定義。 | high |
| SPEC-552 | 鮮度は独立検査ではなく§6のハッシュ束縛により満たす。 | spec | 外から使うとどう振る舞うかという主要な機能・定義・スコープ・委譲の記述。実装方法によらず真であり続ける約束。 | high |
| SPEC-567 | Evidence readerはadapter IDを欠く互換recordも履歴として読み取れる。 | basic_design | Evidence readerのレガシー互換読み込み動作という、外部フォーマット互換readerの決定。 | high |
| SPEC-568 | Evidence readerは、現在のTestが `rust-cargo` で互換runner情報と内容ハッシュからR | basic_design | Evidence readerのrust-cargo限定評価条件という、外部フォーマット互換readerの決定。 | high |
| SPEC-569 | Evidence readerは、Rust実行と一意に確認できない場合は `UNKNOWN` とする。 | basic_design | Evidence readerが一意確認できない場合の扱いという、互換reader動作の一部（567/568と同一クラスタ）。 | high |
| SPEC-572 | 利用者向け簡易出力は `OK` / `NG` の二値とする。 | spec | 外から使うとどう振る舞うかという主要な機能・定義・スコープ・委譲の記述。実装方法によらず真であり続ける約束。 | high |
| SPEC-573 | 完全検証の検査集合はこの4検査に固定する。 | spec | 外から使うとどう振る舞うかという主要な機能・定義・スコープ・委譲の記述。実装方法によらず真であり続ける約束。 | high |
| SPEC-575 | 検査の部分集合を指定した実行は限定scopeである。 | spec | 外から使うとどう振る舞うかという主要な機能・定義・スコープ・委譲の記述。実装方法によらず真であり続ける約束。 | high |
| SPEC-579 | 子に1つでも非 `PASS` があれば親は非 `PASS` とする。 | detailed_spec | 集約時の非PASS優先順位を精密に規定。 競合する条件間の優先順位・境界条件の精密な規定。詳細仕様の該当項目そのもの。 | high |
| SPEC-580 | 集約時に複数の非 `PASS` 値が混在する場合、上位に表示する代表値の優先順位は `FAIL > MISMATCH > | detailed_spec | 集約時の非PASS優先順位を精密に規定。 競合する条件間の優先順位・境界条件の精密な規定。詳細仕様の該当項目そのもの。 | high |
| SPEC-581 | 診断ラベルは代表値の順位に用いず、原因説明として併記する。 | detailed_spec | 集約時の非PASS優先順位を精密に規定。 競合する条件間の優先順位・境界条件の精密な規定。詳細仕様の該当項目そのもの。 | high |
| SPEC-590 | adapter能力の欠落・失敗を `PASS` へ補完しない。 | detailed_spec | adapter能力欠落時の状態・診断ラベル・操作失敗の扱いを精密に規定。 特定条件から特定の状態/結果への精密な割当。「入力・状態・条件が決まれば結果が一意に決まる」水準のためdetailed_spec。 | high |
| SPEC-591 | static解析またはcoverage能力がなければ該当項目は `NO_EVIDENCE`（診断NOT_CHECKED） | detailed_spec | adapter能力欠落時の状態・診断ラベル・操作失敗の扱いを精密に規定。 特定条件から特定の状態/結果への精密な割当。「入力・状態・条件が決まれば結果が一意に決まる」水準のためdetailed_spec。 | high |
| SPEC-592 | runner能力がなければ実行関連は `NO_EVIDENCE`（診断NOT_EXECUTED）とする。 | detailed_spec | adapter能力欠落時の状態・診断ラベル・操作失敗の扱いを精密に規定。 特定条件から特定の状態/結果への精密な割当。「入力・状態・条件が決まれば結果が一意に決まる」水準のためdetailed_spec。 | high |
| SPEC-593 | 解析限界は `UNKNOWN` とする。 | detailed_spec | adapter能力欠落時の状態・診断ラベル・操作失敗の扱いを精密に規定。 特定条件から特定の状態/結果への精密な割当。「入力・状態・条件が決まれば結果が一意に決まる」水準のためdetailed_spec。 | high |
| SPEC-594 | create / edit / audit / run等の明示的操作に必須の能力がなければ操作を失敗させる。 | detailed_spec | adapter能力欠落時の状態・診断ラベル・操作失敗の扱いを精密に規定。 特定条件から特定の状態/結果への精密な割当。「入力・状態・条件が決まれば結果が一意に決まる」水準のためdetailed_spec。 | high |
| SPEC-595 | create / edit / audit / run等の明示的操作に必須の能力がなければファイル・判断記録・Evide | detailed_spec | adapter能力欠落時の状態・診断ラベル・操作失敗の扱いを精密に規定。 特定条件から特定の状態/結果への精密な割当。「入力・状態・条件が決まれば結果が一意に決まる」水準のためdetailed_spec。 | high |
| SPEC-596 | `vtest scan` はregistryに登録された全source discovery adapterへ委譲する。 | basic_design | `vtest scan`がregistry委譲するという、名指しコマンドの責務定義。 | high |
| SPEC-597 | `vtest scan` は統合したdiscovery結果と `.verify/` からエンティティと関係の全体グラフを | basic_design | `vtest scan`が`.verify/`とdiscovery結果からグラフを再構築するという、名指しコマンドの責務定義。 | high |
| SPEC-598 | `vtest scan` はその過程で `chain_integrity` / `orphan_detection` を | basic_design | `vtest scan`が整合性検査を行うという、名指しコマンドの責務定義。 | high |
| SPEC-613 | 診断severityと検証状態を混同しない。 | spec | 外から使うとどう振る舞うかという主要な機能・定義・スコープ・委譲の記述。実装方法によらず真であり続ける約束。 | high |
| SPEC-614 | content_hash照合は決定論的に解決する。 | spec | content_hash照合が決定論的に解決するという性質。基本仕様の主要振る舞いか精密な不変条件かの境界は低確信。 | low |
| SPEC-616 | 参照位置の意味的妥当性・取り込み完全性は検査対象としない。 | spec | 外から使うとどう振る舞うかという主要な機能・定義・スコープ・委譲の記述。実装方法によらず真であり続ける約束。 | high |
| SPEC-617 | 参照位置の意味的妥当性・取り込み完全性は必要ならエスカレーション（§11）で扱う。 | spec | 外から使うとどう振る舞うかという主要な機能・定義・スコープ・委譲の記述。実装方法によらず真であり続ける約束。 | high |
| SPEC-638 | Relationレコードは不変とする。 | spec | Relationレコードが不変であるという定義（§3.2 SPEC-133と同旨の約束、物理格納方式そのものではない）。 | high |
| SPEC-641 | マージ後の論理的不整合（ID衝突、dangling reference、承認の失効）はスキャンと整合性検査で検出する（§ | spec | マージ後の論理的不整合がスキャンで検出されるという振る舞いの約束。 | high |
| SPEC-642 | record / エンティティファイルの書込みは原子的に公開する。 | detailed_spec | 書込みの原子性・部分状態非観測という精密な技術的不変条件。 個別事例・境界条件にまで踏み込んだ不変条件。基本仕様の主要振る舞い記述を超える精度。 | high |
| SPEC-643 | record / エンティティファイルの書込みは読み手に書きかけの部分状態を観測させない。 | detailed_spec | 書込みの原子性・部分状態非観測という精密な技術的不変条件。 個別事例・境界条件にまで踏み込んだ不変条件。基本仕様の主要振る舞い記述を超える精度。 | high |
| SPEC-644 | 並列編集耐性は「公開されたファイルは常に完全である」ことを前提とする。 | detailed_spec | 書込みの原子性・部分状態非観測という精密な技術的不変条件。 個別事例・境界条件にまで踏み込んだ不変条件。基本仕様の主要振る舞い記述を超える精度。 | high |
| SPEC-645 | 並列編集耐性では部分書込みの検出・修復は行わない。 | spec | 部分書込みの検出・修復を行わないという範囲の約束。 | high |
| SPEC-646 | Test ID衝突・dangling referenceの検出、派生indexの再構築、Testと関連情報の同期を人間/ | spec | 検出・再構築・同期が記憶に依存しないことを保証するという約束。 | high |
| SPEC-647 | 具体的な物理保存方式は詳細設計へ委譲する。 | spec | 物理保存方式の詳細を詳細設計へ委譲するというメタな約束。 | high |
| SPEC-650 | `cache/` が破損・削除されても正典は影響を受けない。 | detailed_spec | cache破損・削除という境界条件下でも正典が影響されないという精密な保証。 個別事例・境界条件にまで踏み込んだ不変条件。基本仕様の主要振る舞い記述を超える精度。 | high |
| SPEC-664 | GUIは必須要件としない。 | spec | GUIを必須要件としないという、範囲を限定するだけの約束（新たな入口を宣言するものではない）。 | high |
| SPEC-665 | コマンドの完全仕様（引数・出力・終了コード）は詳細設計で定める。 | spec | コマンドの完全仕様は詳細設計で定めるというメタな委譲の約束。 | high |
| SPEC-666 | 本書ではコマンド一覧と責務を確定する。 | spec | 本書がコマンド一覧と責務を確定するという、文書の権限範囲に関するメタな宣言。 | low |
| SPEC-683 | ゲート充足は検証状態とは別軸の評価である。 | spec | ゲート充足が検証状態と別軸であるという不変条件（exit code体系そのものではない）。 | high |
| SPEC-684 | ゲート充足は検証状態を書き換えない。 | detailed_spec | ゲート充足が検証状態を書き換えないという精密な不変条件。 個別事例・境界条件にまで踏み込んだ不変条件。基本仕様の主要振る舞い記述を超える精度。 | high |
| SPEC-685 | 出力では検証状態とゲート満否を別に提示する。 | spec | 出力で検証状態とゲート満否を分けて提示するという規定。主要な出力記述か精密な出力要件かの境界は低確信。 | low |
| SPEC-689 | 終了コード体系の詳細は詳細設計へ委譲する。 | spec | 終了コード体系の詳細を詳細設計へ委譲するというメタな約束。 | high |
| SPEC-691 | MCPサーバはCLIと同一のコア機能を呼び出す。 | spec | MCPがCLIと同一のコア機能を呼ぶという、入口間の振る舞い一貫性の約束。 | high |
| SPEC-692 | ツールの完全な入出力スキーマは詳細設計で定める。 | spec | 入出力スキーマの詳細を詳細設計で定めるというメタな委譲の約束。 | high |
| SPEC-707 | すべてのツールは非対話で完結する。 | spec | すべてのツールが非対話で完結するという振る舞いの約束。 | high |
| SPEC-708 | CLIとMCPは同じadapter registry composition・JSON envelope・adapter | spec | CLIとMCPが同じregistry composition等を利用するという一貫性規定。主要な関係記述か精密な技術要件かの境界は低確信。 | low |
| SPEC-709 | MCPがCLIと異なるadapterを暗黙選択してはならない。 | detailed_spec | MCPがCLIと異なるadapterを暗黙選択してはならないという精密な否定規定。 個別事例・境界条件にまで踏み込んだ不変条件。基本仕様の主要振る舞い記述を超える精度。 | high |
| SPEC-710 | CLI command体系・MCP tool体系の詳細は詳細設計へ委譲する。 | spec | CLI/MCP体系の詳細を詳細設計へ委譲するというメタな約束。 | high |
| SPEC-712 | source discovery、決定論的解析、Structured Test Operation、test runne | basic_design | discovery/解析/実行/coverage計測をadapter能力として提供するという、adapter/core責務分界の明示（SPEC-076と同旨）。 | high |
| SPEC-714 | core verifierを変更せずに別adapterを登録できる境界を要求する。 | basic_design | core verifierを変更せずに別adapterを登録できる境界の要求という、adapter/core責務分界の明示。 | high |
| SPEC-715 | adapter追加によって共通契約・スキーマが壊れないことを設計制約とする。 | basic_design | adapter追加で共通契約・スキーマが壊れないという設計制約（adapter/core境界の性質）。 | high |
| SPEC-716 | 組込production adapterは `rust-cargo` とする。 | basic_design | 組込production adapterをrust-cargoとする構成要素選択（SPEC-012と同旨）。 | high |
| SPEC-719 | adapterが未登録・能力不足・解析不能の場合、検証結果を推測で `PASS` へ昇格してはならない。 | detailed_spec | adapter能力不足時にPASSへ推測昇格してはならないという精密なfail-closed規定。 個別事例・境界条件にまで踏み込んだ不変条件。基本仕様の主要振る舞い記述を超える精度。 | high |
| SPEC-739 | READMEに非関知宣言を一行入れる。 | basic_design | READMEへの一行追記という、具体的な外部成果物（ドキュメント）の要求。基本仕様としては異例に具体的で要再検討。 | low |

## (d) code_like 候補

`code_like: true` を付けた文は五層版と変わらず**0件**。判断根拠は五層版レポートと同一（SPEC-077の並び順、SPEC-275〜282のRust例示、SPEC-298の確定順序はいずれも「別の実装でも真であり続けるか」を満たすため`code_like`ではないと判断した）。

## (e) PM仮置き（LAYERING.md §2）の検証

六層への改訂はPM仮置きの対象（spec/basic_design/design境界）を変更していないため、五層版試験の結論を維持する。

| PMの仮置き | 本document内の対応文 | 本試験の結果 |
|---|---|---|
| 「検証状態は5つのみ」→ spec | SPEC-056, SPEC-174 ほか | 一致（spec）。5状態の存在自体は変わらずspec。ただし各状態の受入条件（完全検証でOKか、SPEC-178等）はdetailed_specへ移った。 |
| 「インターフェースはCLIとMCPの2入口」→ basic_design | SPEC-015, SPEC-016, SPEC-662, SPEC-663 | 一致（basic_design）。 |
| 「`.verify/doc/<id>.yaml`に1レコード1ファイル」→ basic_design | SPEC-121, SPEC-631〜637 | 一致（basic_design）。 |
| 「`SourceLocation`は`adapter, path, locator, byte_range`を持つ」→ design | 対応文なし（別紙A側） | 判定不能。SPEC-111・SPEC-112・SPEC-297をdesignへ移し、同型パターンで確認。 |
| 「`E-SCAN-016`はorphan_detection=MISMATCHを表す」→ basic_design | 対応文なし | 判定不能。基本仕様側の対応文（SPEC-241）は条件→状態の精密な割当としてdetailed_specへ移った。 |
| 「`rust-cargo` adapterは第6段でSource Targetを抽出する」→ design/code_like | 対応文なし | 判定不能。 |
| 別紙Cの受入条件はspec、fixtureの中身はbasic_design | 対象外 | 判定対象外。ただし本試験の結果、基本仕様内の受入条件相当の文（完全検証OK条件・成立性の必要条件等）はほぼ全てdetailed_specへ移っており、PMのこの仮置きは六層版では『受入条件はdetailed_spec』に修正が必要と考えられる（要Owner確認）。 |

## (f) 規則1・1aの帰結に関する直接観察

六層の判定1（約束か決定か）と1a（主要な振る舞いか、条件ごとの正確な振る舞いか）を765文へ適用した結果、spec 412件（53.9%）、detailed_spec 217件（28.4%）、basic_design 132件（17.3%）、design 4件（0.5%）となった。

五層版では基本仕様相当（旧spec）が629文（82.2%）残っていたが、詳細仕様を切り出すとそのうち217文（旧specの34.5%、全体の28.4%）がdetailed_specへ移り、basic_design（132文）・design（4文）を含めた残りのspecは**412文（全体の53.9%）**まで薄くなった。detailed_specへ移った217文は、条件→状態の精密な割当表（§4.3状態の割当8文、§23整合性検査15文）、検査の境界条件と多重ターゲット集約規則（§5.1〜§5.5・§8.2で計48文）、証拠・承認のハッシュ束縛と有効性条件（§6・§17・§21.1で計34文）、Evidenceの精密なフィールド列挙（§21の7文全数）、判断記録・Test Registryの必須項目と形式的定義（§11.3・§12で計25文）に集中している。残った412文のspecは、用語定義（§1、47文全数）、本書の位置付け・スコープ外・詳細設計への委譲というメタな記述（§0・§29・§30で計63文の大半）、基本仕様の主要な機能・能力の存在（VOの階層化、Structured Test Operationの4種、承認主体の非限定など）を主に含む。したがって六層に分けると、基本仕様（spec）は要求・要件の翻訳としての機能一覧・主要な振る舞い・スコープ宣言に近づき、旧仕様が抱えていた条件別の精密な振る舞い契約の大半は詳細仕様側へ移る、という結果になった。これは昨日のOwner案「基本仕様＝システム概要・システム構成・機能一覧表」に、五層版よりもはるかに近い。
