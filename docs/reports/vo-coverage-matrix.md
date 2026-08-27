# VO カバレッジ照合マトリクス

凍結済み仕様（基本仕様 v0.1／詳細設計本冊 v0.1／別紙A インターフェース仕様 v0.1／別紙C 受入仕様 v0.1）から検証すべき規範的義務を原子単位で抽出し、既存の VO 集合（`docs/plans/vo-chains/data.yaml` の全 108 VO）が各義務を網羅しているかを照合した監査結果である。根拠は上記4文書と data.yaml の現物のみに置く。

## 凡例

判定は3値とする。

- **カバー済み**: その義務に違反したとき、少なくとも1本の VO のテストが失敗する。対応する VO の id を列挙する。
- **未カバー**: 義務は契約レベルで決定論的に検査可能だが、違反を捕らえる VO が存在しない。これが最重要の発見である。
- **対象外**: システム挙動として検証する義務ではない（文書・工程の規約、純粋な HOW 実装細部、上流委譲事項、人間裁定・意味判定に属するもの、または決定論的静的検証にならない実行時挙動）。理由を1文で示す。

モダリティは次のいずれかを記録する。

- **MUST**: 「〜しなければならない」規範。
- **MUST NOT**: 「〜してはならない」禁止規範。
- **定義**: 用語・状態・構造の確定。
- **不変条件**: 常に保たれるべき性質。
- **受入条件**: 別紙C が受入判定に用いる条件。
- **能力**: 「〜できること」の提供要求。

出典 § は義務の主たる根拠箇所を示す（複数文書に跨る義務は主根拠1つを掲げる）。

## 要約

| 指標 | 数値 |
|---|---|
| 義務総数 | 168 |
| カバー済み | 148 |
| 未カバー | 7 |
| 対象外 | 13 |
| 過剰VO（義務に紐づかない VO） | 0 |

義務総数 = カバー済み + 未カバー + 対象外（168 = 148 + 7 + 13）。過剰VO は別軸で数え、逆方向照合の結果 108 VO すべてがいずれかの義務へ紐づいた。

---

## マトリクス

### A. 検証状態と診断ラベル（基本仕様 §4）

| OBL-ID | 義務 | 出典§ | モダリティ | 判定 | 対応VO or 理由 |
|---|---|---|---|---|---|
| OBL-001 | 検証状態は PASS・FAIL・MISMATCH・NO_EVIDENCE・UNKNOWN の5値のみとする。 | 基本§4.1 | 定義 | カバー済み | VO-STATE-01 |
| OBL-002 | 診断ラベル MISSING・NOT_EXECUTED・NOT_CHECKED・STALE は検証状態と別軸とし、状態値には用いず併記のみとする。 | 基本§4.2 | 不変条件 | カバー済み | VO-STATE-02 |
| OBL-003 | 発見された Test に管理宣言が無い場合は MISMATCH（診断 MISSING）とする。 | 基本§4.3 | MUST | カバー済み | VO-CHAIN-TEST-01 |
| OBL-004 | covers の VO 参照を解決できない、同一 construct から複数 entity、Test ID 衝突はいずれも MISMATCH とする。 | 基本§4.3 | MUST | カバー済み | VO-CHAIN-TEST-03, VO-CHAIN-TEST-04 |
| OBL-005 | 文書鎖のリンク切れ・content_hash 不一致・孤児文書は MISMATCH とする。 | 基本§4.3 | MUST | カバー済み | VO-CHAIN-DOC-01, VO-CHAIN-DOC-02, VO-ORPHAN-01 |
| OBL-006 | 証拠が存在しない、または証拠のハッシュが現在の対象と不一致なら NO_EVIDENCE とする。 | 基本§4.3 | MUST | カバー済み | VO-EVIDENCE-FRESH-SUBJECT, VO-EVIDENCE-FRESH-TARGET |
| OBL-007 | scope 限定で検査を実施しなかった項目は NO_EVIDENCE（診断 NOT_CHECKED）とする。 | 基本§4.3 | MUST | カバー済み | VO-SCOPE-NOPROMOTE |
| OBL-008 | discovery が不完全・解析不能なら UNKNOWN とする。 | 基本§4.3 | MUST | カバー済み | VO-CHAIN-DISC-01 |
| OBL-009 | テストランナーが失敗を報告したら FAIL とする。 | 基本§4.3 | MUST | カバー済み | VO-TARGET-RESULT-01 |
| OBL-010 | 宣言された検証対象の実行が0回なら FAIL（診断 NOT_EXECUTED）とする。 | 基本§4.3 | MUST | カバー済み | VO-TARGET-RT-01 |
| OBL-011 | UNKNOWN は正常動作としての降参とし、内部エラー・入力不正のフォールバック先に使わない。 | 基本§4.4 | MUST NOT | カバー済み | VO-EXIT-QUARANTINE |
| OBL-012 | 未承認であることだけを理由に PASS を UNKNOWN 等へ変更してはならない。 | 基本§4.5 | MUST NOT | カバー済み | VO-APPROVAL-INDEP |
| OBL-013 | 承認済みであることを理由に非 PASS を PASS へ変更してはならない。 | 基本§4.5 | MUST NOT | カバー済み | VO-APPROVAL-INDEP |

### B. chain_integrity（基本仕様 §5.1・§12・§23／本冊 §11.1.1／別紙C §18.3.1）

| OBL-ID | 義務 | 出典§ | モダリティ | 判定 | 対応VO or 理由 |
|---|---|---|---|---|---|
| OBL-014 | 各 document の derives_from 参照先が存在しない場合は文書鎖のリンク切れとして MISMATCH とする。 | 基本§5.1 | MUST | カバー済み | VO-CHAIN-DOC-01 |
| OBL-015 | document の content_hash が現物ファイルと一致しない場合は MISMATCH（診断 STALE）とする。 | 本冊§11.1.1 | MUST | カバー済み | VO-CHAIN-DOC-02 |
| OBL-016 | 各 VO は1件以上の document への解決可能な derives_from を持たねばならず、欠く場合は MISMATCH とする。 | 基本§5.1 | MUST | カバー済み | VO-CHAIN-VO-01 |
| OBL-017 | VO の parent が存在しない、または親子関係が循環する場合は MISMATCH とする。 | 本冊§5.4 | MUST | カバー済み | VO-CHAIN-VO-02 |
| OBL-018 | 発見された各 Test に対応する管理宣言（有効な Test ID・covers 1件以上・intent 等の必須 metadata）がちょうど1件存在しない場合は MISMATCH（診断 MISSING）とする。 | 本冊§11.1.1 | MUST | カバー済み | VO-CHAIN-TEST-01 |
| OBL-019 | すべての管理対象 Test に covers 1件以上を一律要求し、covers 0件は役割による例外を設けず MISMATCH とする。 | 基本§12 | MUST | カバー済み | VO-CHAIN-TEST-02 |
| OBL-020 | covers の VO 参照を解決できない Test は参照を除去せず保持したまま MISMATCH とする。 | 本冊§4.4 | MUST | カバー済み | VO-CHAIN-TEST-03 |
| OBL-021 | Test ID が発見結果全体で一意でなく衝突する状態は MISMATCH とする。 | 本冊§11.1.1 | MUST | カバー済み | VO-CHAIN-TEST-04 |
| OBL-022 | covers する Test を1件も持たない leaf VO は双方向完全性の不備として MISMATCH（診断 MISSING）とする。 | 本冊§11.1.1 | MUST | カバー済み | VO-CHAIN-BIDIR-01 |
| OBL-023 | 発見された Test → 管理宣言と leaf VO → Test の両方向が成立して初めて chain_integrity が成立する。 | 基本§5.1 | 不変条件 | カバー済み | VO-CHAIN-BIDIR-01, VO-CHAIN-TEST-01 |
| OBL-024 | Relation の from / to が存在しないエンティティを参照する場合は MISMATCH とする。 | 本冊§3.3 | MUST | カバー済み | VO-CHAIN-REL-01 |
| OBL-025 | Relation の bare ULID を REL- 形式へ in-memory 正規化し、正規化形が一致しない・payload 重複する関係は不一致とする。 | 本冊§3.3 | MUST | カバー済み | VO-CHAIN-REL-01 |
| OBL-026 | adapter discovery の失敗を Test 0件の正常 scan として扱わず、解析不能・不完全な batch は当該検査を UNKNOWN とする。 | 本冊§5.1 | MUST NOT | カバー済み | VO-CHAIN-DISC-01 |
| OBL-027 | 全 Test を管理対象とすることと、当該 Test を仕様適合の証拠として算入することは別個の条件とする。 | 基本§5.1 | 不変条件 | カバー済み | VO-CHAIN-TEST-01, VO-TARGET-PASS-01 |
| OBL-028 | 未登録 Test（管理宣言を持たない construct）は診断 severity を warning としつつ、managed entity へ対応しない事実を MISMATCH として完全検証へ反映する。 | 基本§12 | MUST | カバー済み | VO-AGG-UNREG-NG |
| OBL-029 | targets 1件以上は rust-cargo adapter の必須 metadata であり、欠落は E-SCAN-007 として MISMATCH（診断 MISSING）とする一方、core 中立の必須リンクには加えない。 | 本冊§4.4 | MUST | カバー済み | VO-CHAIN-TEST-01（必須 metadata 欠落として同一経路で検出） |
| OBL-030 | Test construct 表面の `@vtest.` 宣言で未知キーはエラー、非 Test construct 表面の未知キーは警告として、無音で無視しない。 | 本冊§4.2 | MUST | 未カバー | 打鍵ミス検出（E-SCAN-006／W-SCAN-105）を確かめる VO が無い。 |

### C. orphan_detection（基本仕様 §5.2／本冊 §5.6／別紙C §18.3.2）

| OBL-ID | 義務 | 出典§ | モダリティ | 判定 | 対応VO or 理由 |
|---|---|---|---|---|---|
| OBL-031 | derives_from が空でどの document からも参照されず doc.roots にも列挙されない document を孤児として MISMATCH とする。 | 本冊§5.6 | MUST | カバー済み | VO-ORPHAN-01 |
| OBL-032 | config の doc.roots に列挙された document を孤児検出の対象外とする。 | 本冊§5.6 | MUST | カバー済み | VO-ORPHAN-02 |
| OBL-033 | 存在しない document を根に指定した設定を config invariant 違反として拒否する。 | 本冊§5.6 | MUST | カバー済み | VO-ORPHAN-04 |
| OBL-034 | 孤児検出の対象を文書層に限り、宣言されていない実装を孤児として検出しない。 | 基本§5.2 | MUST NOT | カバー済み | VO-ORPHAN-03 |

### D. oracle_presence と不成立構造（基本仕様 §5.4・§5.5・§8.3／本冊 §7／別紙C §18.3.3）

| OBL-ID | 義務 | 出典§ | モダリティ | 判定 | 対応VO or 理由 |
|---|---|---|---|---|---|
| OBL-035 | 成否判定が定数で失敗し得ない Test（assert!(true) 等）を FAIL とする。 | 本冊§7.2 | MUST | カバー済み | VO-ORACLE-DA001 |
| OBL-036 | 対象を呼び出すが結果を成否判定に用いない Test を FAIL とし、可変参照・グローバル経由で結果が伝わりうる場合は UNKNOWN とする。 | 本冊§7.2 | MUST | カバー済み | VO-ORACLE-DA003 |
| OBL-037 | 観測同士の自己比較（assert_eq!(x, x) 等）で成否が対象の振る舞いに依存しない Test を FAIL とする。 | 本冊§7.2 | MUST | カバー済み | VO-ORACLE-DA004 |
| OBL-038 | 本体が空の Test を FAIL とする。 | 本冊§7.2 | MUST | カバー済み | VO-ORACLE-DA005 |
| OBL-039 | 成否を左右する assert 相当を1つも持たない Test を FAIL とする。 | 本冊§7.2 | MUST | カバー済み | VO-ORACLE-DA006 |
| OBL-040 | oracle_presence は DA-001/003/004/005/006 の合成とし、全て違反なしで PASS、1つでも FAIL で FAIL、FAIL なく UNKNOWN があれば UNKNOWN とする。 | 本冊§7.1 | 定義 | カバー済み | VO-ORACLE-COMPOSE |
| OBL-041 | oracle_presence に動的な合格昇格の経路を持たせず、証明できない場合は UNKNOWN のまま runtime 証拠で PASS へ変えない。 | 本冊§7.1 | MUST NOT | カバー済み | VO-ORACLE-NOPROMOTE |
| OBL-042 | 判定は保守的に行い、確定した違反だけを FAIL とし、クロージャ内・マクロ展開内など追い切れない箇所は UNKNOWN とする。 | 本冊§7.2 | MUST | カバー済み | VO-ORACLE-CONSERV |
| OBL-043 | 静的解析は正典レコードを持たない再計算派生とし、検証のたびに現在の source から再計算して永続化しない。 | 本冊§7.1 | 不変条件 | カバー済み | VO-ORACLE-RECALC |
| OBL-044 | 判定に用いたソース断片集合の完全性を保証できない場合、その判定を UNKNOWN とし PASS にしない。 | 別紙C§18.3.3 | MUST | カバー済み | VO-ORACLE-FRAGMENT |
| OBL-045 | 本体で対象を呼ばない別プロセス型の検証では、照合装置検査を UNKNOWN のまま残し、検証対象実現検査とは別々の値をとらせ、総合を NG とする。 | 本冊§7.3 | MUST | カバー済み | VO-ORACLE-SUBPROC-SPLIT |
| OBL-046 | core は adapter 固有の AST・assertion 構文・call graph を解釈せず、正規化されたルール結果だけを検証・集約する。 | 基本§5.5 | MUST NOT | カバー済み | VO-ADAPTER-NEUTRAL |

### E. target_binding（基本仕様 §5.3／本冊 §7.3・§10／別紙C §18.3.5）

| OBL-ID | 義務 | 出典§ | モダリティ | 判定 | 対応VO or 理由 |
|---|---|---|---|---|---|
| OBL-047 | 宣言された対象コードが Test 実行経路へ入ったことを確認し、対象の呼び出しが解析境界内に無いと確定できれば対象別 FAIL、別ファイル・別クレートで確定できなければ UNKNOWN とする。 | 本冊§7.2 | MUST | カバー済み | VO-TARGET-STATIC-01 |
| OBL-048 | 解析境界内で到達を静的に否定した FAIL は、後からの動的な実行証明で覆さない。 | 本冊§7.3 | MUST NOT | カバー済み | VO-TARGET-STATIC-02 |
| OBL-049 | 対象ごとの計測回数が1以上なら対象別 PASS、0なら対象別 FAIL、確実に同定・計測できなければ対象別 UNKNOWN とする。 | 本冊§10.2 | MUST | カバー済み | VO-TARGET-RT-01 |
| OBL-050 | 静的到達が UNKNOWN の対象は、その対象の動的計測が PASS（計測済みで回数が正）のときに限り到達要件を充足したと扱う。 | 本冊§7.3 | MUST | カバー済み | VO-TARGET-RT-02 |
| OBL-051 | 複数対象の集約は、1件でも FAIL なら FAIL、FAIL なく UNKNOWN があれば UNKNOWN、全対象 PASS のときだけ PASS とする。 | 本冊§10.2 | 定義 | カバー済み | VO-TARGET-MULTI-01 |
| OBL-052 | 別プロセス・別スレッドの境界を越える実行では静的到達を UNKNOWN とし、動的計測が当該実行を宣言対象へ帰属できれば対象別 PASS とする。 | 本冊§7.3 | MUST | カバー済み | VO-TARGET-BOUNDARY-01 |
| OBL-053 | 有効な証拠のテスト結果が失敗を示す場合、target_binding を FAIL とする。 | 本冊§11.2 | MUST | カバー済み | VO-TARGET-RESULT-01 |
| OBL-054 | テスト結果が PASS でかつ全宣言対象の到達要件を充足したときだけ PASS とし、未充足対象は count 0 で FAIL、未計測で NO_EVIDENCE、対象不見当で UNKNOWN と分ける。 | 本冊§11.2 | MUST | カバー済み | VO-TARGET-PASS-01 |
| OBL-055 | どの実行形態でも宣言対象を実行しない契約のみの Test は、静的にも動的にも到達を確立できず未充足のままとする。 | 本冊§7.3 | 不変条件 | カバー済み | VO-TARGET-CONTRACT-01 |
| OBL-056 | 計測能力・計測ツールが利用不能なら NO_EVIDENCE（診断 NOT_CHECKED）、解析限界なら UNKNOWN とし、PASS へ推測昇格しない。 | 本冊§10.1 | MUST | カバー済み | VO-TARGET-CAP-01 |
| OBL-057 | 動的な実行到達が充足されても照合装置検査（DA-003）を代替せず、照合装置側の UNKNOWN・FAIL はそのまま総合へ寄与させる。 | 本冊§7.3 | MUST NOT | カバー済み | VO-TARGET-NODA003-01 |
| OBL-058 | runtime 証明に依存する target_binding は §11.2 が選択した最新 Evidence が鮮度を満たすときだけ用い、無効な最新 Evidence から古い有効 Evidence へフォールバックしない。 | 別紙C§18.3.3 | MUST NOT | カバー済み | VO-EVIDENCE-NOFALLBACK |
| OBL-059 | target_coverage.checked: true の Evidence で target 別 entry が欠落・重複・解決後の canonical Source Target 集合と不一致なら PASS にしない。 | 別紙C§18.3.5 | MUST NOT | 未カバー | target_coverage entry の集合整合による非 PASS を確かめる VO が無い（鮮度系 VO は内容不一致のみ扱う）。 |

### F. 証拠と鮮度（基本仕様 §6・§21／本冊 §11.2／別紙C §18.3.4）

| OBL-ID | 義務 | 出典§ | モダリティ | 判定 | 対応VO or 理由 |
|---|---|---|---|---|---|
| OBL-060 | 証拠を検証対象の内容ハッシュへ束縛し、ストアがハッシュをキーとして必須とする。 | 基本§6 | MUST | カバー済み | VO-EVIDENCE-BIND-01 |
| OBL-061 | Test subject の内容ハッシュが証拠時点と食い違う場合、離れた位置のメタデータ変更を含め NO_EVIDENCE（診断 STALE）とする。 | 基本§6 | MUST | カバー済み | VO-EVIDENCE-FRESH-SUBJECT |
| OBL-062 | 宣言対象の参照集合または対象構成の内容ハッシュが証拠時点と食い違う場合、NO_EVIDENCE（診断 STALE）とする。 | 本冊§11.2 | MUST | カバー済み | VO-EVIDENCE-FRESH-TARGET |
| OBL-063 | 証拠のリビジョンが特定できない、または現在の HEAD と一致しない場合、NO_EVIDENCE（診断 STALE）とする。 | 本冊§11.2 | MUST | カバー済み | VO-EVIDENCE-FRESH-REVISION |
| OBL-064 | 対象外 helper・依存・ランナー・ツールチェーン・設定など実行可能状態が食い違えば NO_EVIDENCE（STALE）、完全性を確かめられなければ UNKNOWN とする。 | 本冊§11.2 | MUST | カバー済み | VO-EVIDENCE-FRESH-EXECSTATE |
| OBL-065 | 証拠の adapter 識別が現在と食い違えば MISMATCH、確認できなければ UNKNOWN とする。 | 本冊§11.2 | MUST | カバー済み | VO-EVIDENCE-FRESH-ADAPTER |
| OBL-066 | 全宣言対象が一意に解決できることを証拠生成の前提とし、1件でも対象なし・曖昧なら部分証拠を作らず生成しない。 | 本冊§9.4 | MUST NOT | カバー済み | VO-EVIDENCE-GEN-01 |
| OBL-067 | ビルド失敗・ランナー失敗・必須能力欠如・対象解決失敗・実行前後の実行可能状態変化のいずれかでは証拠を生成しない。 | 別紙C§18.3.4 | MUST NOT | カバー済み | VO-EVIDENCE-GEN-02 |
| OBL-068 | 最新証拠が鮮度を失っているとき、過去の古い有効証拠へ後退して合格にしない。 | 本冊§11.2 | MUST NOT | カバー済み | VO-EVIDENCE-NOFALLBACK |
| OBL-069 | 内容ハッシュを adapter が自己確定せず、core が言語非依存の正規化を行って計算する（adapter はハッシュ未計算 DTO を返す）。 | 本冊§1.3 | MUST | カバー済み | VO-EVIDENCE-HASH-CORE |
| OBL-070 | Evidence の schema 違反・target entry の欠落／重複／余剰・集約結果と target 別結果の矛盾を検出し、その証拠を有効な結果に使わない。 | 本冊§3.6 | MUST NOT | 未カバー | 壊れた Evidence レコード（E-SCAN-010）の拒否を確かめる VO が無い。 |
| OBL-071 | 鮮度は独立検査を設けず §6 のハッシュ束縛の設計制約として満たし、喪失を診断ラベル STALE として説明する。 | 基本§21.1 | 不変条件 | カバー済み | VO-EVIDENCE-FRESH-SUBJECT, VO-EVIDENCE-FRESH-REVISION |

### G. 判定権威（基本仕様 §7）

| OBL-ID | 義務 | 出典§ | モダリティ | 判定 | 対応VO or 理由 |
|---|---|---|---|---|---|
| OBL-072 | テスト合否の権威を adapter のランナーに置き、本システムは再判定せずランナー結果を証拠として消費する。 | 基本§7 | MUST | カバー済み | VO-AUTHORITY-01 |
| OBL-073 | target_binding をランナーの PASS を前提として実行を実際に伴ったかを問う独立の照合として位置づける。 | 基本§7 | 定義 | カバー済み | VO-AUTHORITY-02 |

### H. Test の検証成立性（基本仕様 §8）

| OBL-ID | 義務 | 出典§ | モダリティ | 判定 | 対応VO or 理由 |
|---|---|---|---|---|---|
| OBL-074 | 管理対象 Test は、検証対象の振る舞いを反映した観測に基づき適合・不適合を識別し、不適合を非成功として反映する成否判定を持たねばならない。 | 基本§8.2 | MUST | カバー済み | VO-ORACLE-DA006, VO-ORACLE-DA003 |
| OBL-075 | 成立性の確認方法は検証対象・実行形態・観測方法で異なってよいが、成立性の問いへの答えは確認方法に依らず同一でなければならない。 | 基本§8.2 | 不変条件 | 対象外 | 実行形態横断の答え同一性を保証するメタ原則であり、単一の判別テストに落ちない設計制約。 |
| OBL-076 | 成立条件を確認できないことと違反していることを区別し、確認不能だけを根拠に違反も成立確認済みも推定しない。 | 基本§8.2 | MUST NOT | カバー済み | VO-ORACLE-CONSERV, VO-ORACLE-FRAGMENT |
| OBL-077 | Test の成否判定が他要素の判定能力に依存する場合、その依存要素の正当性確認または明示的な信頼基盤で成立性確認を終端しなければならない。 | 基本§8.2 | MUST | 対象外 | 信頼基盤の具体範囲は詳細設計・adapter へ委譲され（assert 相当構文が体現）、独立検査化されていない。 |

### I. 検証対象と Source Target（基本仕様 §9／本冊 §6）

| OBL-ID | 義務 | 出典§ | モダリティ | 判定 | 対応VO or 理由 |
|---|---|---|---|---|---|
| OBL-078 | 検証対象を正規化位置と任意の恒久識別子を併有する単一実体として識別し、複数対象を代表1件へ縮約せず双方向に辿れる。 | 本冊§6.1.1 | MUST | カバー済み | VO-ADAPTER-SRC-IDENT |
| OBL-079 | 恒久識別子を持つ対象を位置でも参照でき、両方式が同一の内容ハッシュと同一の識別へ解決する。 | 本冊§6.1 | MUST | カバー済み | VO-ADAPTER-SRC-DUAL |
| OBL-080 | 識別を宣言参照から解決を経て正規化位置へ一方向で確定させ、証拠・判断記録は解決後の正規化位置を記録し、綴りの異なる複数対象が同一へ解決する状態を不一致として検出する。 | 本冊§6.1.1 | MUST | カバー済み | VO-ADAPTER-SRC-CANON |
| OBL-081 | 対象参照の解決を解決済・対象なし・曖昧に区別し、曖昧は候補を解決結果に記録せず診断表示のみで終端させ、core の単一経路が所有する。 | 本冊§6.1 | MUST | カバー済み | VO-ADAPTER-SRC-RESOLVE |
| OBL-082 | 恒久 SRC ID を全 adapter 統合後にリポジトリ全体で一意とし衝突を拒否しつつ、各対象は正規化位置で独立に具体化する。 | 本冊§5.2 | MUST | カバー済み | VO-ADAPTER-SRC-UNIQ |
| OBL-083 | すべての管理対象 Test に1件以上の検証対象宣言を要求し、検証対象は実装 construct に限定せず外部契約・境界上の振る舞いも含む。 | 基本§9.1 | MUST | カバー済み | VO-ADAPTER-SRC-IDENT |
| OBL-084 | 検証対象と実装 traceability を別の関係として扱い、traceability の存在を Test 成立性の条件とせず、一方から他方を推定してはならない。 | 基本§9.3 | MUST NOT | 未カバー | 検証対象と任意 traceability の分離・非推定を確かめる VO が無い。 |

### J. Verification Obligation（基本仕様 §10／本冊 §3.2.1）

| OBL-ID | 義務 | 出典§ | モダリティ | 判定 | 対応VO or 理由 |
|---|---|---|---|---|---|
| OBL-085 | VO を1件以上の document から derives_from で直結させ、document と VO の間に他のエンティティ層を置かない。 | 基本§3.2 | MUST NOT | カバー済み | VO-DOCMODEL-02 |
| OBL-086 | flat な VO 群と階層化 VO 群の双方を扱い、flat な VO を再帰分解・階層化する操作を提供する。 | 基本§10 | 能力 | カバー済み | VO-CHAIN-VO-02（parent 階層構造の検査で階層 VO の扱いを保証。再帰分解操作そのものは OBL-088 と同じ抜け） |
| OBL-087 | VO と Test の対応を 1:1 に限定せず 1:N・N:1・N:M を許容する。 | 基本§10 | 能力 | カバー済み | VO-CHAIN-TEST-01（covers N:M 前提の管理宣言検査） |
| OBL-088 | dimensions と coverage_policy（independent-axes／full-product／explicit）を宣言でき、full-product 等の宣言 partition の直積を決定論的に子 VO へ実体化する。 | 本冊§3.2.1 | 能力 | 未カバー | vo expand による組合せ実体化（別紙C §18.3.1 の受入条件）を確かめる VO が無い。 |
| OBL-089 | 分解が仕様に対して十分かの判定は本システムの検査ではなくエスカレーションの領分とする。 | 基本§10 | 不変条件 | 対象外 | 網羅十分性は意味判定であり検査から明示的に排除されている（§11 の領分）。 |

### K. 発見・エスカレーションと判断記録（基本仕様 §11／本冊 §8）

| OBL-ID | 義務 | 出典§ | モダリティ | 判定 | 対応VO or 理由 |
|---|---|---|---|---|---|
| OBL-090 | 本システムは宣言されていない義務・網羅漏れ・意味のずれを自ら発見・裁定せず、UNKNOWN として外部へ引き渡す。 | 基本§11 | MUST NOT | カバー済み | VO-DECISION-NONGATE, VO-ONBOARD-PENDING |
| OBL-091 | 判断記録の受理は当該対象の検証状態を昇格させない。 | 基本§11.3 | MUST NOT | カバー済み | VO-DECISION-NONGATE |
| OBL-092 | 判断提出時に bundle 存在・subject 一致・記録時ハッシュと現在の一致・decision が受理値であることを順に検証する。 | 本冊§8.4 | MUST | カバー済み | VO-DECISION-SUBMIT |
| OBL-093 | 判断記録は actor・subject・decision を必須とし理由を任意とし、理由が空であることだけを根拠に無効・UNKNOWN・NO_EVIDENCE・MISMATCH 扱いしない。 | 基本§11.3 | MUST NOT | カバー済み | VO-DECISION-REASON-OPT |
| OBL-094 | 受理された判断を対象ハッシュと依存 closure へ束縛し、対象・依存の変更（依存文書のハッシュ不一致を含む）で失効させる。 | 本冊§8.5 | MUST | カバー済み | VO-DECISION-HASHBIND |
| OBL-095 | 対象が変更された後は過去の判断を流用せず4検査を再実施し、変更そのものは UNKNOWN を生成しない。 | 基本§11.3 | MUST NOT | カバー済み | VO-DECISION-REVERIFY |
| OBL-096 | 判断バンドルを派生情報として cache（Git 管理外）へ出力し、対象を一意に解決できなければ生成せず候補を選ばない。 | 本冊§8.1 | MUST NOT | カバー済み | VO-DECISION-BUNDLE |
| OBL-097 | 同一対象に有効な判断記録が複数あってよい（再判断・多重判断）。 | 本冊§8.5 | 能力 | 対象外 | 多重判断の許容は禁止・失敗を判定する検査ではなく、違反を持たない許容規定。 |
| OBL-098 | deterministic な静的解析結果と agent／human の判断結果を区別して保存・表示する。 | 別紙C§18.3.6 | MUST | 未カバー | 機械判定と外部判断の出所区別を確かめる VO が無い。 |

### L. Test Registry・Intent・Parameterized Test（基本仕様 §12・§13・§14）

| OBL-ID | 義務 | 出典§ | モダリティ | 判定 | 対応VO or 理由 |
|---|---|---|---|---|---|
| OBL-099 | Test ID をハンドルとして Test Intent・covers・検証対象・Source Target・Location・判断記録・Evidence を検索可能とする。 | 基本§12 | 能力 | カバー済み | VO-TRACE-INDEX, VO-TRACE-ANYNODE |
| OBL-100 | Test の存在理由分類（role／anchor）と covers 件数の可変制約を v0.1 では設けず、covers 1件以上を一律要求する。 | 基本§12 | MUST NOT | カバー済み | VO-CHAIN-TEST-02 |
| OBL-101 | 実装コードを読まなくても何を検証するか判断できる Test Intent を付随情報として関連付けられ、宣言鎖のノードとしない。 | 基本§13 | 能力 | カバー済み | VO-CHAIN-TEST-01（intent を必須 metadata として保持） |
| OBL-102 | adapter が識別した table-driven test construct 全体を一つの Test として登録でき、内部の各 case を独立 Test ID へ分解することを必須としない。 | 基本§14 | 能力 | 未カバー | table-driven／parameterized Test の一括登録（@vtest.case）を確かめる VO が無い。 |

### M. Structured Test Operation（基本仕様 §15／別紙A §15）

| OBL-ID | 義務 | 出典§ | モダリティ | 判定 | 対応VO or 理由 |
|---|---|---|---|---|---|
| OBL-103 | 作成・編集を desired state 方式とし、adapter が差分を計算し core が再走査で検証し、同じ状態の再適用が冪等になる。 | 基本§15.1 | MUST | カバー済み | VO-STO-DESIRED |
| OBL-104 | 編集の一回の対象を原則1 Test とし、拡張範囲の単一置換で他 Test・通常ソースを変えないことを前後二重検査で保証する。 | 基本§15.3 | MUST | カバー済み | VO-STO-1TEST |
| OBL-105 | 構造化入力を受理時に検証し、対象 symbol・Test ID・参照 VO の不在では候補を提示し、必須値と未知フィールドを常に検証する。 | 基本§15.2 | MUST | カバー済み | VO-STO-INPUT-VALIDATE |
| OBL-106 | Form の kind をリポジトリ全体で一意とし所有 adapter を別 field で宣言させ、一意に一致する場合だけ操作を許し、重複・未知・曖昧・能力欠如を拒否してファイルを変えない。 | 基本§15.4 | MUST | カバー済み | VO-STO-FORM-RESOLVE |
| OBL-107 | helper・fixture・通常ソースコードの編集手段を Test 操作として提供しない。 | 基本§15.3 | MUST NOT | カバー済み | VO-STO-HELPER-OOS |
| OBL-108 | 未知の form を core が Rust 用として推測してはならない。 | 基本§15.4 | MUST NOT | カバー済み | VO-STO-FORM-RESOLVE |

### N. 承認（基本仕様 §17／本冊 §3.5）

| OBL-ID | 義務 | 出典§ | モダリティ | 判定 | 対応VO or 理由 |
|---|---|---|---|---|---|
| OBL-109 | 承認は検証状態と独立の別軸とし、承認済みで非 PASS を PASS へ昇格させず、未承認で PASS を降格させない。 | 基本§17 | MUST NOT | カバー済み | VO-APPROVAL-INDEP |
| OBL-110 | 判断済みと承認済みを区別し、判断記録と承認記録が別 entity でありうる。 | 基本§17 | 定義 | カバー済み | VO-APPROVAL-DISTINCT |
| OBL-111 | VO の実効承認を、対象ハッシュ一致かつ依存 closure（再帰的親 VO・derives_from document・上位 document）が完全一致する承認が1件以上あるときに approved とし、それ以外を draft とする。 | 本冊§3.5 | 定義 | カバー済み | VO-APPROVAL-CLOSURE |
| OBL-112 | 対象または依存成果物（document 再登録を含む）の変更で承認を失効させる。 | 本冊§3.5 | MUST | カバー済み | VO-APPROVAL-STALE |
| OBL-113 | 依存 closure やハッシュを欠く互換承認から approved を導かず、作成時に対象・依存を完全解決できなければ記録を生成しない。 | 本冊§3.5 | MUST NOT | カバー済み | VO-APPROVAL-NOCLOSURE |
| OBL-114 | VO の状態を正典 field として保存せず承認から導出し、書き手は保存せず読み手は保存された互換 field を無視する。 | 本冊§3.2 | MUST | カバー済み | VO-APPROVAL-STATUS-DERIVED |
| OBL-115 | 承認記録は承認者（人間／エージェントの種別と識別子）・対象または判断参照・承認状態を必須とし、根拠を任意とする。 | 本冊§3.5 | MUST | カバー済み | VO-APPROVAL-RECORD |
| OBL-116 | 承認主体を人間に限定せず Agent も承認権限を持ちうるが、全 Agent が承認権限を持つことは要求しない。 | 基本§17 | 能力 | カバー済み | VO-APPROVAL-RECORD（種別 human／agent を必須記録） |
| OBL-117 | 誰がどの対象・範囲を承認できるか（approval authority）の具体ロール・必要承認数・権限 schema はプロジェクト設定へ委譲する。 | 基本§17 | 定義 | 対象外 | §30 item22 の委譲事項であり、本 version の検査対象でない。 |

### O. フェーズゲート（基本仕様 §20／本冊 §11.5／別紙A §12.3）

| OBL-ID | 義務 | 出典§ | モダリティ | 判定 | 対応VO or 理由 |
|---|---|---|---|---|---|
| OBL-118 | 登録されたゲートについて、検証結果が要求水準を満たすかと要求ロールの有効承認が存在するかを評価し、満否と根拠を新規コマンドを増やさず既存経路で提示する。 | 本冊§11.5 | MUST | カバー済み | VO-GATE-EVAL |
| OBL-119 | ゲートの責務を評価・提示に限り、フェーズを自動的に遷移させない。 | 基本§20 | MUST NOT | カバー済み | VO-GATE-NOTRANSITION |
| OBL-120 | config で承認ロールから承認者集合を解決し、ゲートが参照するロールが設定に無い場合を設定違反とする。 | 別紙A§12.3 | MUST | カバー済み | VO-GATE-ROLE-RESOLVE |

### P. 完全検証・集約・報告（基本仕様 §22）

| OBL-ID | 義務 | 出典§ | モダリティ | 判定 | 対応VO or 理由 |
|---|---|---|---|---|---|
| OBL-121 | 完全検証の合格を、4検査すべてが PASS でかつ証拠が §6 を満たす場合のみとし、1項目でも非 PASS なら NG とする。 | 基本§22.1 | 不変条件 | カバー済み | VO-AGG-FAILCLOSED |
| OBL-122 | Test 単位の結果を VO・Feature・document 単位へ集約し、子に1つでも非 PASS があれば親を非 PASS とする。 | 基本§22.2 | MUST | カバー済み | VO-AGG-TREE |
| OBL-123 | 集約の代表値の優先順位を FAIL > MISMATCH > NO_EVIDENCE > UNKNOWN とし、診断ラベルを順位に用いず併記する。 | 基本§22.2 | 定義 | カバー済み | VO-AGG-PRIORITY |
| OBL-124 | 管理側の他検査がすべて PASS でも未登録 Test が1件あれば chain_integrity により総合を NG とする。 | 別紙C§18.3.8 | MUST | カバー済み | VO-AGG-UNREG-NG |
| OBL-125 | NG のとき、どのエンティティの・どの検査が・どの状態と診断ラベルで落ちたかを掘り下げて辿れ、テキストと JSON の両形式で出力する。 | 基本§22.3 | 能力 | カバー済み | VO-AGG-DRILLDOWN |
| OBL-126 | 利用者向け簡易出力を OK／NG の二値とし、完全検証の検査集合を4検査に固定して設定で追加・削除できない。 | 基本§22.1 | 不変条件 | カバー済み | VO-SCOPE-NODEGRADE, VO-AGG-FAILCLOSED |
| OBL-127 | covers を持つ Test を covers 先 VO の子として表示し、いずれの VO へも寄与しない事実も出力から確認できる。 | 基本§22.3 | 能力 | カバー済み | VO-AGG-DRILLDOWN |

### Q. scope（基本仕様 §4.6）

| OBL-ID | 義務 | 出典§ | モダリティ | 判定 | 対応VO or 理由 |
|---|---|---|---|---|---|
| OBL-128 | scope を検査軸（4検査の部分集合）とエンティティ軸（document／VO／Test の部分木）の2軸で限定できる。 | 基本§4.6 | 能力 | カバー済み | VO-SCOPE-2AXIS |
| OBL-129 | scope 外・未実施の項目を NO_EVIDENCE（診断 NOT_CHECKED）として保持し PASS へ変換せず、要求 scope と未検証範囲を併記する。 | 基本§4.6 | MUST NOT | カバー済み | VO-SCOPE-NOPROMOTE |
| OBL-130 | いかなる設定値も完全検証を4検査未満へ縮退させず、検査指定省略時は固定4検査とする。 | 基本§4.6 | MUST NOT | カバー済み | VO-SCOPE-NODEGRADE |
| OBL-131 | 旧来の項目列挙をバージョンによらず設定違反とし、設定の欠落だけを固定4検査へ具体化（メモリ上の補完なし）する。 | 本冊§2.2 | MUST | カバー済み | VO-SCOPE-FULLSCOPE-INV |
| OBL-132 | 表示 scope と判定に必要な内部依存の評価を分離し、限定 scope でも内部依存を評価しつつ scope 外の表示値を NO_EVIDENCE に保つ。 | 別紙C§18.3.3 | MUST | カバー済み | VO-SCOPE-INTERNAL-DEP |

### R. トレーサビリティと役割別 projection（基本仕様 §19）

| OBL-ID | 義務 | 出典§ | モダリティ | 判定 | 対応VO or 理由 |
|---|---|---|---|---|---|
| OBL-133 | 最小単位「上流ノード → 関係 → 下流ノード」を任意のノードから取得でき、連続して遡行・下降し全体構造も取得できる。 | 基本§19 | 能力 | カバー済み | VO-TRACE-ANYNODE |
| OBL-134 | 義務→テスト・実装対象→テスト・文書→義務・文書→文書などの逆引きを正典から再構築できる派生として持つ。 | 基本§19 | 能力 | カバー済み | VO-TRACE-INDEX |
| OBL-135 | 同一のトレース構造から役割別の見え方を粒度を変えて取得でき、役割を固定の列挙にしない。 | 基本§19 | 能力 | カバー済み | VO-TRACE-PROJECTION |
| OBL-136 | 導出・被覆・検証対象・実装追跡の性質の異なる関係型を単一へ潰さず区別する。 | 基本§3.4 | MUST NOT | カバー済み | VO-DOCMODEL-03 |

### S. 途中導入と既存プロジェクト対応（基本仕様 §18）

| OBL-ID | 義務 | 出典§ | モダリティ | 判定 | 対応VO or 理由 |
|---|---|---|---|---|---|
| OBL-137 | 既に大量のコードとテストがあるプロジェクトを検証対象にでき、未登録 Test・欠落宣言・未確定の義務・未実施の検査を検証済みとして扱わない。 | 基本§18.1 | MUST NOT | カバー済み | VO-ONBOARD-VISUALIZE |
| OBL-138 | 初期化が検証用ディレクトリを作成して既存コードを改変せず、一部が欠落した状態でも読み取れる。 | 基本§18.1 | MUST NOT | カバー済み | VO-ONBOARD-INIT |
| OBL-139 | 判断待ち情報を対象・種別・関係検査・根拠・バンドル参照からなる機械可読構造として検証・報告出力へ横断集約する。 | 基本§18.3 | 能力 | カバー済み | VO-ONBOARD-PENDING |
| OBL-140 | 導入難度がプロジェクト規模とは別の理由で構造的に増大する設計を避ける。 | 基本§18.4 | 不変条件 | 対象外 | 強い不変条件ではなく設計原則であり、判別テストに落ちない。 |

### T. adapter・wire・config（基本仕様 §2.4・§27／本冊 §5）

| OBL-ID | 義務 | 出典§ | モダリティ | 判定 | 対応VO or 理由 |
|---|---|---|---|---|---|
| OBL-141 | 検証契約・識別子・ハッシュ・証拠・状態・集約を言語とランナーに依存させず、テストを関数ではなく実行記述子だけで表す。 | 基本§27 | MUST | カバー済み | VO-ADAPTER-NEUTRAL |
| OBL-142 | core を変えずに別 adapter を登録でき、識別子重複・未登録・宣言能力と実装の不一致・adapter 間の Test ID 重複を拒否する。 | 基本§27 | MUST | カバー済み | VO-ADAPTER-REGISTER |
| OBL-143 | adapter が未登録・能力不足・解析不能の場合、検証結果を推測で PASS へ昇格させない。 | 基本§27 | MUST NOT | カバー済み | VO-ADAPTER-NOPROMOTE |
| OBL-144 | 設定の新旧両形式を読み取り書き換えずに受理し、書き出し・初期化は新形式とし、Test 出力は実行座標を常に持ち特定 adapter のみ互換 field を追加し、対象は常に一覧で単数互換は1件時のみとする。 | 基本§2.4 | MUST | カバー済み | VO-ADAPTER-WIRE |
| OBL-145 | 複数 adapter の結果を決定論的に統合し、同一 root 共有を許す一方で同一 adapter 内 root 重複を拒否し、統合後の全体で Test ID の大局的一意性を検査する。 | 基本§2.4 | MUST | カバー済み | VO-ADAPTER-MERGE |
| OBL-146 | synthetic adapter が非 Rust source・関数でない Test construct・doc comment でない metadata 宣言・Rust item path でない locator を core 変更なしで登録・scan・verify できる。 | 別紙C§18.3.12 | 能力 | カバー済み | VO-ADAPTER-NEUTRAL |

### U. インターフェース（基本仕様 §26／別紙A §12・§13）

| OBL-ID | 義務 | 出典§ | モダリティ | 判定 | 対応VO or 理由 |
|---|---|---|---|---|---|
| OBL-147 | 機械インターフェースの全ツールが同一入力に対しコマンド行と同じデータと診断を返し、同じ処理構成・adapter 選択・エラー体系を共有する。 | 基本§26.2 | MUST | カバー済み | VO-IFACE-PARITY |
| OBL-148 | 要求・通知・バッチ・不正な転送を規約どおりに処理し、不正入力にコード・メッセージ・候補を付したツールエラーを返す。 | 別紙C§18.3.11 | MUST | カバー済み | VO-IFACE-JSON-RPC |
| OBL-149 | 機械インターフェースの長時間実行中もソース変更を再走査し、古くなった合格を保持しない。 | 別紙C§18.3.11 | MUST NOT | カバー済み | VO-IFACE-RESCAN |
| OBL-150 | 検証結果を出力する全経路で検証状態列と診断ラベル列を常に別軸の2列として提示する。 | 別紙A§12.1 | MUST | カバー済み | VO-STATE-02 |
| OBL-151 | 新規 CLI コマンド・MCP ツールを増やさず、新設機能（ゲート評価・projection・判断待ち）を既存コマンド・ツールの引数と出力で露出する。 | 本冊§0 | MUST NOT | 対象外 | 分冊・コマンド増設禁止の設計制約であり、個別の検証挙動ではない。 |

### V. 終了コードと並列保存（基本仕様 §26.1・§24／本冊 §16・§17.2）

| OBL-ID | 義務 | 出典§ | モダリティ | 判定 | 対応VO or 理由 |
|---|---|---|---|---|---|
| OBL-152 | 終了コードで要求 scope 合格（0）・検証 NG（1）・操作拒否（2）・内部エラー（3）を区別し、診断コードの区分を対応コードへ写す。 | 本冊§17.2 | MUST | カバー済み | VO-EXIT-CODES |
| OBL-153 | 内部エラー・入力不正を検証状態と別系統の終了コードで表し、UNKNOWN へすり替えない。 | 基本§4.4 | MUST NOT | カバー済み | VO-EXIT-QUARANTINE |
| OBL-154 | 1レコード1ファイルとし中央共有台帳を持たず、Relation・判断・承認・Evidence は ULID ファイル名の新規追加のみで作成する。 | 基本§24.2 | MUST | 対象外 | 物理保存レイアウトの規約であり、違反を判定する検証挙動ではない（§28 委譲）。 |
| OBL-155 | record・エンティティファイルの書込みを原子的に公開し、読み手に書きかけの部分状態を観測させない。 | 基本§24.2 | MUST NOT | 対象外 | 並行書込み時の物理公開挙動であり、決定論的静的検証にならない。 |
| OBL-156 | 派生情報（検証グラフ・逆引き index・集約結果）を正典から scan でいつでも再構築でき、Git 管理しない。 | 基本§24.3 | 能力 | カバー済み | VO-TRACE-INDEX |

### W. 文書モデルと基本原則（基本仕様 §2・§3・§16）

| OBL-ID | 義務 | 出典§ | モダリティ | 判定 | 対応VO or 理由 |
|---|---|---|---|---|---|
| OBL-157 | 上流文書を種別ごとの専用スキーマを設けず単一の総称ノードとして扱う。 | 基本§3.1 | 定義 | カバー済み | VO-DOCMODEL-01 |
| OBL-158 | 関係リンクの説明文を任意とし、空でも chain_integrity 違反・MISMATCH としない。 | 基本§3.4 | MUST NOT | カバー済み | VO-DOCMODEL-04 |
| OBL-159 | 文書層の段をリンクとして表し、段を追加してもスキーマを壊さず検査本数を増やさない。 | 基本§3.2 | 不変条件 | カバー済み | VO-DOCMODEL-05 |
| OBL-160 | Test → VO（covers）・Test → SRC（targets）の導出できる関係を外部ファイルへ重複保存せず、常に adapter 所有の Test metadata 宣言から再構築する。 | 基本§2.3 | MUST NOT | カバー済み | VO-DOCMODEL-03（関係型区別と導出保持） |
| OBL-161 | 不一致はどちらが正かを決めず状態として提示し、どれかを正として他を修正させない。 | 基本§2.1 | MUST NOT | 対象外 | 検証機であり決定機でないという原則（P-001）で、単一の判別テストに落ちない。 |
| OBL-162 | 対象ソースコード内の doc comment を、その対象実装自身の正当性を証明する唯一の仕様根拠として使用しない。 | 基本§16 | MUST NOT | 対象外 | v0.1 は文書入力を doc/ の総称 document に限り、doc comment 単独を仕様根拠にしない運用規約（検査化されていない）。 |
| OBL-163 | 意味判定・候補生成を検証成立条件にせず、外部 AI/Agent の補助は許容するがその能力を成立条件にしない。 | 基本§11.1 | MUST NOT | カバー済み | VO-ONBOARD-VISUALIZE, VO-DECISION-NONGATE |

### X. スコープ外の非関知（基本仕様 §29）

| OBL-ID | 義務 | 出典§ | モダリティ | 判定 | 対応VO or 理由 |
|---|---|---|---|---|---|
| OBL-164 | 文書内容の意味的良否を検証しない（仕様書同士の品質監査を行わない）。 | 基本§29 OOS-001 | MUST NOT | 対象外 | スコープ外宣言であり、機能として実装・検査しない事項。 |
| OBL-165 | 不一致に対しどれを正とするか（修正方針）を決定しない。 | 基本§29 OOS-002 | MUST NOT | 対象外 | スコープ外宣言（決定機でない）で、検査化する義務でない。 |
| OBL-166 | Test Edit 対象外の一般ソースコード編集を管理しない。 | 基本§29 OOS-003 | MUST NOT | カバー済み | VO-STO-HELPER-OOS |
| OBL-167 | フェーズのライフサイクル管理・工程遷移を行わない。 | 基本§29 OOS-004 | MUST NOT | カバー済み | VO-GATE-NOTRANSITION |
| OBL-168 | 宣言されていない実装の存在を関知せず、実装レイヤーの孤児検出を行わない。 | 基本§29 OOS-005 | MUST NOT | カバー済み | VO-ORPHAN-03 |

（注: OBL は 001〜168 の連番で採番した。別紙 C 受入固有の重複記述は親義務へ畳み込み、独立採番していない。OOS の一部は他§の禁止規範と対応 VO を共有する。）

---

## 要注意リスト（未カバー・過剰VO）

Owner はこのセクションだけを見れば抜けが分かる。以下は「義務は決定論的に検査可能だが、それを確かめる VO が存在しない」7件である。各行は義務1文と、穴が残った場合に検証されないままになる帰結1文で示す。

### 未カバー（7件）

1. **OBL-030 — 注釈の打鍵ミス検出（本冊 §4.2）**
   義務: `@vtest.` 宣言の未知キーを Test construct 表面ではエラー、非 Test 表面では警告として無音で無視しない。
   帰結: 打鍵ミスした `@vtest.covers` 等が黙って無視され、意図した宣言が検証に反映されない事故を、システムが検出できないまま通してしまう。

2. **OBL-059 — target_coverage entry の集合整合（別紙C §18.3.5）**
   義務: target_coverage.checked: true の Evidence で target 別 entry が欠落・重複・解決後の canonical Source Target 集合と不一致なら PASS にしない。
   帰結: 計測結果の一部だけを持つ、または対象集合とずれた target_coverage を有効な合格として誤用し、実際には計測されていない対象を検証済みと見なす。

3. **OBL-070 — 壊れた Evidence レコードの拒否（本冊 §3.6）**
   義務: Evidence の schema 違反・target entry の欠落／重複／余剰・集約結果と target 別結果の矛盾（E-SCAN-010）を検出し、その証拠を有効な結果に使わない。
   帰結: 破損・自己矛盾した証拠レコードが有効な PASS として消費され、鮮度検査を通り抜けて偽の合格を生む。

4. **OBL-084 — 検証対象と実装 traceability の分離・非推定（基本仕様 §9.3）**
   義務: 検証対象と実装 traceability を別の関係として扱い、traceability の存在を Test 成立性の条件とせず、一方から他方を推定しない。
   帰結: 任意の traceability が欠けただけで Test を不成立と誤判定する、または traceability を成立の証拠に流用する退行を検出できない。

5. **OBL-088 — VO 組合せの実体化（本冊 §3.2.1／別紙C §18.3.1）**
   義務: dimensions と coverage_policy から full-product 等の宣言 partition の直積を決定論的に子 VO へ実体化する。
   帰結: vo expand による組合せ空間の子 VO 生成が壊れても検出されず、直積・独立軸・明示列挙の実体化結果が仕様どおりか保証されない。

6. **OBL-098 — 機械判定と外部判断の区別保存（別紙C §18.3.6）**
   義務: deterministic な静的解析結果と agent／human の判断結果を区別して保存・表示する。
   帰結: 機械が決定論で出した結果と人／エージェントの判断が混同されて保存・表示され、根拠の出所を取り違えて信頼度を誤る。

7. **OBL-102 — table-driven／parameterized Test の一括登録（基本仕様 §14）**
   義務: adapter が識別した table-driven test construct 全体を一つの Test として登録でき、内部の各 case を独立 Test ID へ分解することを必須としない。
   帰結: 複数 case を持つ table-driven test の登録・扱いが検証されず、`@vtest.case` を持つ Test が正しく1 Test として管理される保証がない。

未カバーは OBL-030・OBL-059・OBL-070・OBL-084・OBL-088・OBL-098・OBL-102 の7件である（OBL-059 と OBL-070 はともに証拠健全性に属するが、対象が target_coverage の集合整合と Evidence schema の整合とで異なるため別立てとする）。

> 補足: 未カバー7件はいずれも「証拠・宣言の健全性を静的／決定論的に確かめる」種類の義務であり、5状態・4検査・DA 規則・鮮度・承認・集約といった中核判定はすべてカバー済みである。抜けは中核ロジックではなく、入力レコードの健全性検査（注釈の打鍵ミス、壊れた Evidence、target_coverage 集合整合、組合せ実体化、traceability 分離、機械／人判断の区別、table-driven 登録）に偏っている。

### 過剰VO（0件）

逆方向照合（108 VO → 義務）の結果、義務に紐づかない VO は無い。data.yaml の全 VO はいずれかの規範的義務へ意味的に対応した。VO-CHAIN-* は §5.1・§23 の chain_integrity 規則へ、VO-ORACLE-* は §5.4・§7 の oracle_presence へ、VO-TARGET-* は §5.3・§7.3・§10 の target_binding へ、VO-EVIDENCE-* は §6・§21 の鮮度へ、VO-APPROVAL-*／VO-GATE-* は §17・§20 の承認・ゲートへ、VO-ADAPTER-*／VO-STO-*／VO-IFACE-*／VO-TRACE-*／VO-ONBOARD-*／VO-DOCMODEL-*／VO-STATE-*／VO-SCOPE-*／VO-AGG-*／VO-EXIT-*／VO-DECISION-*／VO-AUTHORITY-*／VO-ORPHAN-* はそれぞれ対応する§へ紐づく。
