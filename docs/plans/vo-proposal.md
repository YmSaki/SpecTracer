# SpecTracer v0.1 VO セット

Owner 裁定（2026-08-28、104 件）を反映した検証義務（Verification Obligation, VO）セット。凍結スペック5文書（要求・要件定義 / 基本仕様 / 詳細設計 本冊 / 別紙A / 別紙C）だけを根拠とする（別紙B=実装計画は obligation 源にしない）。

規模: 領域（親 VO）25 本 / leaf VO 199 本。

## 凡例

- **leaf VO**: 1 件 = 独立に検証可能な 1 命題。直下テストの入力と期待出力が一意に決まる粒度で書く。
- **親 VO**: 集約ノード。covers を持たず、子 leaf の fail-closed 合成だけを値とする（詳細設計 本冊 §11.3）。
- **derives_from**: obligation を最も具体・検証可能に述べる**最下流の単一 document** へ張る。上流の該当箇所は「上流」欄に併記する。
- **DOC 命名**: `DOC-REQ`=要求・要件定義 / `DOC-BASIC`=基本仕様 / `DOC-DETAIL`=詳細設計 本冊 / `DOC-ANNEX-A`=別紙A / `DOC-ANNEX-C`=別紙C。
- **ID 規約**: `VO-<領域>-<小領域>-<連番>`。裁定「分割」で置き換えられた VO の旧 ID は廃止し、後継に新 ID を採番する。裁定に触れられていない VO の ID は変更しない。
- **テスト欄**: 「入力 = … → 期待 = …」を 1 行以上。行が複数ある leaf は、同一の判定機構を異なる入力で行使するものに限る。判定機構が異なるものは別 leaf に分けた。

## 方針

- **薄さ**: 説明を読んだ時点でテストの入力と期待が一意に決まることを合格条件とする。「総合的に」「適切に」「該当する」「要件を満たす」等の未定義語を命題に置かない。
- **粒度規則**: 独立した判定機構ごとに 1 leaf とする（例: 判断記録提出の 4 拒否条件は 4 leaf）。判定機構が 1 つで不正入力の種類だけが複数のものは 1 leaf にまとめ、テスト欄を複数行にする（例: `combinations` 受理条件違反 7 種は E-SCAN-017 の 1 leaf）。
- **判断の分離**: 意味一致・網羅十分性・役割ごとの見え方の中身・設計上の構造制約は VO にせず、基本仕様 §11 の判断記録へ移送する（「§11 へ移送した条項」節）。
- **4 軸の独立**: 検証状態（5 状態）／判断（判断記録）／承認（承認レコード）／ゲート充足は互いに独立した軸であり、一方が他方を書き換える VO を置かない。
- **追記型と現在値**: 事実レコードは追記型であり、「現在採用されるレコード」は明示の `supersedes` 関係だけで決まる。時刻・ULID 順・件数を採用規則に用いない。曖昧は fail-closed。

---

## 裁定 → VO 照合表（104 行）

「VO ID」列は当該裁定で生成・変更・削除した leaf VO。「対象外」は `—`。

| # | ruling id | 実効裁定 | VO ID | 備考 |
|---|---|---|---|---|
| 1 | UNCOV-c4b2183f | 追加 | VO-CHAIN-DECL-01 / VO-CHAIN-DECL-02 | 表面ごとに判定機構が異なる（error / warning）ため 2 leaf |
| 2 | UNCOV-c1e239b4 | 統合 | VO-TARGET-MULTI-01 | entry 過不足・canonical 対象集合不一致のケースを追加 |
| 3 | UNCOV-097963f1 | 追加 | VO-EVIDENCE-INTEGRITY-01 | |
| 4 | UNCOV-f3f04b0d | 追加 | VO-DECISION-PROV-01 | |
| 5 | UNCOV-9497ddc9 | 追加 | VO-TESTMODEL-CASES-01 | |
| 6 | UNCOV-3bac8f5f | 統合 | VO-CHAIN-TEST-04 | 管理宣言が複数対応（`ManagedTestLink::Multiple`）のケースを追加 |
| 7 | UNCOV-62527a0e | 追加 | VO-ORACLE-DA003-MULTI-01 | |
| 8 | UNCOV-065c6547 | 対象外 | — | 運用規約に留める |
| 9 | UNCOV-32c8ed52 | 統合 | VO-TARGET-CAP-01 | 意図的 scope 省略のケースを追加 |
| 10 | UNCOV-0d23262d | 追加 | VO-ORACLE-TRUENEG-01 | |
| 11 | UNCOV-ad6cdacb | 追加 | VO-VOMODEL-DIM-01 / VO-VOMODEL-EXPAND-01 / -02 / -03 | 組合せ方針 3 種は実体化規則が異なるため別 leaf |
| 12 | UNCOV-91c13116 | 追加 | VO-VOMODEL-HIER-01 | 再帰分解操作は VO-VOMODEL-EXPAND-01..03 が担う |
| 13 | UNCOV-5b5aa758 | 追加 | VO-VOMODEL-MULT-01 | |
| 14 | UNCOV-16dc1bdf | 追加 | VO-INVARIANT-FORM-01 | assert 所在軸に限定して起草。実行形態軸の扱いは ESCALATE-1 |
| 15 | UNCOV-2ce2683a | 追加 | VO-ADAPTER-TARGETREQ-01 / -02 | core 中立側と rust-cargo 側で判定機構が異なる |
| 16 | UNCOV-09b10bc1 | 追加 | VO-TARGET-RESOLVE-DIAG-01 | |
| 17 | UNCOV-647db2de | 追加 | VO-ADAPTER-SRC-CANON-04 | |
| 18 | UNCOV-0b4ef718 | 追加 | VO-ORACLE-IGNORE-01 / VO-TARGET-IGNORE-01 | 供給先検査が異なるため 2 leaf |
| 19 | UNCOV-f6001d8f | 追加 | VO-EVIDENCE-SELECT-01 | |
| 20 | UNCOV-8197a7f5 | 統合 | VO-CHAIN-TEST-04 | DOC ID / VO ID の logical ID 重複（E-SCAN-010）へ拡張 |
| 21 | UNCOV-55bd70d0 | 統合 | VO-CHAIN-REL-01 | bare ULID 受理・非書き換えの肯定側を追加 |
| 22 | UNCOV-7e5cf4ef | 追加 | VO-SCOPE-OUTPUT-01 / VO-SCOPE-OUTPUT-02 | |
| 23 | UNCOV-21634c58 | 追加 | VO-DETERM-CANON-01 | |
| 24 | UNCOV-c8c2e644 | 追加 | VO-TRACE-ANCHOR-01 | 宣言鎖・検査結果の提示は VO-AGG-DRILLDOWN、対象外範囲は VO-SCOPE-OUTPUT-01 が担う |
| 25 | UNCOV-766c4eb4 | 追加 | VO-DETERM-VERIFY-01 | Owner 裁定により決定性命題として起草 |
| 26 | UNCOV-74a67f9f | 追加 | VO-TESTMODEL-INTENT-01 | |
| 27 | UNCOV-d39ecd31 | 追加 | VO-TESTMODEL-INTENT-02 | |
| 28 | UNCOV-6d385b46 | 追加 | VO-STO-PARITY-01 | |
| 29 | UNCOV-4ad0b24f | 追加 | VO-INVARIANT-SEVERITY-01 | |
| 30 | UNCOV-95224104 | 追加 | VO-INVARIANT-SCANSCOPE-01 | |
| 31 | UNCOV-d5020280 | 追加 | VO-STORE-ATOMIC-01 | |
| 32 | UNCOV-f26145db | 追加 | VO-STORE-RELIMMUT-01 | |
| 33 | UNCOV-bc1c3cc8 | 追加 | VO-STORE-APPEND-01 | |
| 34 | UNCOV-24270b70 | 追加 | VO-STORE-GITIGNORE-01 | |
| 35 | UNCOV-0a32dc96 | 追加 | VO-IFACE-NONINTERACTIVE-01 | |
| 36 | UNCOV-c162ba7a | 追加 | VO-EVIDENCE-DIRTY-01 | |
| 37 | UNCOV-03ed7443 | 追加 | VO-ORACLE-TERM-01 / VO-ORACLE-TERM-02 | derives_from は暫定（ESCALATE-2） |
| 38 | UNCOV-15fa5eb1 | 追加 | VO-ORACLE-TERM-03 | |
| 39 | UNCOV-326eb448 | 対象外 | — | 網羅十分性は §11 の領分 |
| 40 | UNCOV-2ab24ec0 | 対象外 | — | 多重判断の許容そのものは検査にしない。競合の扱いは #77 |
| 41 | UNCOV-766f9055 | 対象外 | — | |
| 42 | UNCOV-c8028652 | 対象外 | VO-GATE-EVAL（条項削除） | 旧 VO-GATE-EVAL の「新規 cmd/tool を増やさず既存 verify/report で露出」句を削除 |
| 43 | UNCOV-bd43d724 | 対象外 | — | |
| 44 | UNCOV-31f27bcf | 対象外 | — | OOS-001 |
| 45 | UNCOV-9afb95b8 | 対象外 | — | OOS-002 |
| 46 | WEAK-d55ac5a0 | 統合 | VO-DOCMODEL-03 | 任意性・相互推定禁止・target 非計上のケースを追加 |
| 47 | WEAK-f0cba1eb | 統合 | VO-TARGET-PASS-01 / VO-AGG-FAILCLOSED-01 | 瑕疵なし入力の真陽性 PASS ケースを追加 |
| 48 | WEAK-d6ee8e43 | 統合 | VO-TARGET-PASS-01 / VO-TARGET-RT-01 | 証拠皆無ケースと count 0 の FAIL との区別を追加 |
| 49 | WEAK-73843f36 | 統合 | VO-TRACE-INDEX | Evidence からの関係生成・修復禁止ケースを追加 |
| 50 | WEAK-6f4ffbf7 | 統合 | VO-CHAIN-TEST-01 | Intent の非ノード性ケースを追加（ESCALATE-3） |
| 51 | WEAK-a491cb0b | 統合 | VO-ADAPTER-NOPROMOTE | NOT_CHECKED 値と解析限界 UNKNOWN の区別を追加 |
| 52 | WEAK-5a1be1fe | 統合 | VO-ORACLE-CONSERV | DA-001 定数性未確定トリガを追加 |
| 53 | WEAK-2d5d9a7d | 統合 | VO-ADAPTER-SRC-DUAL | ハッシュ入力構成のケースを追加 |
| 54 | WEAK-47e80c88 | 統合 | VO-AGG-DRILLDOWN / VO-ONBOARD-FAILCLOSED-01 | 非寄与 Test の表示完全性を追加 |
| 55 | WEAK-33661e6b | 統合 | VO-CHAIN-REL-01 | 正規化形の衝突（E-SCAN-010）を追加 |
| 56 | WEAK-ea96cd6f | 統合 | VO-SCOPE-2AXIS / VO-SCOPE-NOPROMOTE | 限定 scope を完全検証として表示しないケースを追加 |
| 57 | WEAK-23bbace5 | 統合 | VO-ADAPTER-WIRE-04 | 分割後継へ接続。非 Rust の空値・dummy 生成禁止を追加 |
| 58 | WEAK-59c77d7b | 統合 | VO-APPROVAL-INDEP | 降格禁止方向のテストを追加 |
| 59 | WEAK-4e0ba018 | 統合 | VO-EVIDENCE-FRESH-SUBJECT | 実行座標のみ変更の経路を追加 |
| 60 | WEAK-a78c0cf4 | 統合 | VO-AUTHORITY-01 | FAIL 側の証拠消費を追加 |
| 61 | WEAK-eaa44487 | 統合 | VO-AUTHORITY-01 | rust-cargo 具体（cargo test の ok / FAILED 行）を追加 |
| 62 | WEAK-ad403b4e | 統合 → 新 leaf | VO-DECISION-BUNDLE-03 | 統合先 VO-DECISION-BUNDLE は #101 で分割済み。内容完全性はいずれの後継とも判定機構が異なるため後継 leaf を採番 |
| 63 | WEAK-ab178d4e | 統合 → 新 leaf | VO-ONBOARD-PARTIAL-01 | 統合先 VO-ONBOARD-VISUALIZE は #99 で分割済み。読取り堅牢性を独立 leaf として明示化 |
| 64 | WEAK-6d7bb36b | 統合 | VO-DOCMODEL-04 | note の保存・往復（肯定側）を追加 |
| 65 | WEAK-1e1e7212 | 統合 | VO-TRACE-ANYNODE | 下降・全体構造・非 Test 起点を追加 |
| 66 | WEAK-79a0056a | 統合 | VO-CHAIN-TEST-01 / VO-AGG-UNREG-NG | 警告重大度と chain 反映の同時成立を追加 |
| 67 | WEAK-a665014e | 統合 | VO-IFACE-PARITY-03 | 分割後継へ接続。拒否入力の負経路・暗黙フォールバック禁止を追加 |
| 68 | FEAT-f35e635e | 追加 | VO-ORACLE-TERM-01 / VO-ORACLE-TERM-02 | #37 と同一命題の設計側。derives_from 暫定（ESCALATE-2） |
| 69 | FEAT-66517617 | 統合 | VO-TRACE-ANCHOR-01 | #24 と同一 leaf。anchor 付き derives_from エッジの同伴として実現 |
| 70 | FEAT-e87340f3 | 追加 | VO-VOMODEL-COMB-01 / VO-VOMODEL-COMB-02 | 受理条件と入力経路で判定機構が異なる |
| 71 | FEAT-82c5f3c2 | 対象外 | — | #8 と同一裁定 |
| 72 | FEAT-9e603d25 | 追加 | VO-APPROVAL-SUBJECT-01 / VO-APPROVAL-SUBJECT-02 | |
| 73 | FEAT-c141aa53 | 追加 | VO-GATE-NAME-01 | |
| 74 | FEAT-6ec47bd0 | 追加 | VO-AGG-PARENT-01 / VO-AGG-PARENT-02 | |
| 75 | FEAT-c134e8a6 | 追加 | VO-SCOPE-OUTPUT-01 / VO-SCOPE-OUTPUT-02 | #22 と同一 leaf 対 |
| 76 | FEAT-0c90f367 | 対象外 | — | 後続版へ委譲済み |
| 77 | FEAT-9b4e5d68 | 追加 | VO-DECISION-EFFECTIVE-01 / VO-DECISION-EFFECTIVE-02 | |
| 78 | FEAT-01bd4ddd | 追加 | VO-DECISION-CASECOV-01 | |
| 79 | FEAT-d36760ad | 追加 | VO-STO-ROLLBACK-01 | |
| 80 | FEAT-7d03b44e | 追加 | VO-APPROVAL-STATE-01 / -02 / -03 | 値域・実効集合の畳み込み・supersedes で判定機構が異なる |
| 81 | FEAT-6c8f99f9 | 追加 | VO-DECISION-BUNDLE-03 | #62 と同一 leaf（項目列の 1 項目として行使） |
| 82 | FEAT-478da9f9 | 追加 | VO-ONBOARD-NOMOD-01 | 旧 VO-ONBOARD-INIT の非改変句は本 leaf へ移し、VO-ONBOARD-INIT は生成物の命題に絞る |
| 83 | FEAT-d18e8d78 | 追加 | VO-GATE-REQVER-01 / VO-GATE-REQVER-02 | |
| 84 | FEAT-30470802 | 対象外 | — | §30 item22 の委譲事項 |
| 85 | EXCESS-8eb6b2f1 | 統合 | VO-EVIDENCE-GEN-01 | 曖昧ケースの期待状態を MISMATCH へ修正し、誤った NO_EVIDENCE 規定を削除 |
| 86 | EXTRACT-542a640e | 追加 | VO-DELIV-README-01 | |
| 87 | EXTRACT-217a84b7 | 対象外 | — | 文書運用プロセスの規範 |
| 88 | FATVO-625d4c8d | 分割 | 削除: VO-ADAPTER-WIRE ／ 生成: VO-ADAPTER-WIRE-01..05 | |
| 89 | FATVO-b84378e8 | 分割 | 削除: VO-ADAPTER-REGISTER ／ 生成: VO-ADAPTER-REGISTER-01..05 | |
| 90 | FATVO-79525dc3 | 分割 | 削除: VO-ADAPTER-MERGE ／ 生成: VO-ADAPTER-MERGE-01..04 | |
| 91 | FATVO-1c8fcdd3 | 分割 | 削除: VO-IFACE-PARITY ／ 生成: VO-IFACE-PARITY-01..04 | |
| 92 | FATVO-3542951b | 分割 | 削除: VO-IFACE-JSON-RPC ／ 生成: VO-IFACE-RPC-01..04 | |
| 93 | FATVO-1c1e9fdb | 分割 | 削除: VO-APPROVAL-STATUS-DERIVED ／ 生成: VO-APPROVAL-STATUS-01..03 | |
| 94 | FATVO-6b2b8e52 | 分割 | 削除: VO-APPROVAL-NOCLOSURE ／ 生成: VO-APPROVAL-COMPAT-01 / VO-APPROVAL-CREATE-01 | |
| 95 | FATVO-44619eef | 分割 | 削除: VO-ADAPTER-SRC-RESOLVE ／ 生成: VO-ADAPTER-SRC-RESOLVE-01 / -02 ／ 移送: TRANSFER-1 | |
| 96 | FATVO-45837267 | 分割 | 削除: VO-ADAPTER-SRC-CANON ／ 生成: VO-ADAPTER-SRC-CANON-01..03 | |
| 97 | FATVO-32c42934 | 分割 | 削除: VO-ADAPTER-SRC-IDENT ／ 生成: VO-ADAPTER-SRC-IDENT-01 / -02 | |
| 98 | FATVO-90c77c2e | 分割 | 削除: VO-ADAPTER-SRC-UNIQ ／ 生成: VO-ADAPTER-SRC-UNIQ-01 / -02 | |
| 99 | FATVO-799af9f6 | 分割 | 削除: VO-ONBOARD-VISUALIZE ／ 生成: VO-ONBOARD-FAILCLOSED-01 / VO-ONBOARD-CAPACITY-01 ／ 移送: TRANSFER-2 | |
| 100 | FATVO-62d677c8 | 分割 | 削除: VO-DECISION-SUBMIT ／ 生成: VO-DECISION-SUBMIT-01..04 | |
| 101 | FATVO-5c87a685 | 分割 | 削除: VO-DECISION-BUNDLE ／ 生成: VO-DECISION-BUNDLE-01 / -02 | |
| 102 | FATVO-2be19fa9 | 分割 | 削除: VO-AGG-FAILCLOSED ／ 生成: VO-AGG-FAILCLOSED-01 | 「かつ証拠が要件を満たす」句を削除し 4 検査の連言に一致させた |
| 103 | FATVO-70f1bf0d | 分割 | 削除: VO-TRACE-PROJECTION ／ 生成: VO-TRACE-PROJECTION-01 / -02 ／ 移送: TRANSFER-3 | |
| 104 | FATVO-65e9fa02 | 分割 | 削除: VO-STO-1TEST ／ 生成: VO-STO-1TEST-01 ／ 移送: TRANSFER-4 | 「原則」を外し厳密に 1 Test へ固定 |

---

## §11 へ移送した条項

VO にせず基本仕様 §11 の判断記録（judgment）として扱う条項。

| id | 移送した条項 | 出所 | 判断として扱う理由 |
|---|---|---|---|
| TRANSFER-1 | Source Target 解決を core の単一経路が所有すること | 旧 VO-ADAPTER-SRC-RESOLVE | 入出力で確定できない構造制約であり、判別テストが定まらない |
| TRANSFER-2 | 未登録テスト・欠落宣言・未確定の義務・未実施の検査の各区分 → 状態値の対応表 | 旧 VO-ONBOARD-VISUALIZE | 各区分にどの状態値を割り当てるかが上流に列挙されておらず、肯定側の期待出力が一意でない |
| TRANSFER-3 | 役割 → 見え方（参照対象・粒度）の具体的対応 | 旧 VO-TRACE-PROJECTION | 詳細設計 本冊 §11.6 が preset・モード体系を別紙A・プロジェクト設定へ委譲しており、期待射影を役割ごとに判断で確定させる |
| TRANSFER-4 | 1 回の Structured Edit が複数 Test にまたがってよい例外条件 | 旧 VO-STO-1TEST | 例外条件が上流に列挙されておらず、合否が一意でない |

---

## 領域一覧（親 VO 25 本）

| 領域 | 対象 | leaf 数 |
|---|---|---|
| DOCMODEL | 総称 document・宣言鎖のモデル | 5 |
| VOMODEL | VO の表現能力（dimensions・実体化・階層・多重度） | 8 |
| TESTMODEL | Test Entity のモデル（cases・Test Intent） | 3 |
| CHAIN | 検査①`chain_integrity` | 13 |
| ORPHAN | 検査②`orphan_detection` | 4 |
| TARGET | 検査③`target_binding` | 13 |
| ORACLE | 検査④`oracle_presence` | 17 |
| STATE | 5 状態と 4 診断ラベルの 2 軸 | 2 |
| EVIDENCE | Evidence の生成 precondition・内部整合・ハッシュ束縛・鮮度 | 13 |
| AUTHORITY | 合否判定権威（runner） | 2 |
| DECISION | 判断記録（非ゲート）・バンドル・実効判断 | 15 |
| APPROVAL | 承認（検証状態と独立の軸） | 15 |
| GATE | フェーズゲート評価（提示のみ） | 6 |
| SCOPE | scope 2 軸と scope 出力 | 7 |
| AGG | fail-closed 集約と機能単位の束ね | 7 |
| ADAPTER | adapter 境界・registry・wire 互換・Source Target identity | 29 |
| STORE | 保存規約（原子的公開・追記型・Git 管理境界） | 4 |
| EXIT | UNKNOWN 検疫と終了コード | 2 |
| ONBOARD | 途中導入（既存資産・判断待ち） | 6 |
| TRACE | トレーサビリティと projection | 5 |
| STO | Structured Test Operation | 7 |
| IFACE | MCP = CLI 同一性・非対話性 | 10 |
| INVARIANT | 領域横断の不変条件 | 3 |
| DETERM | 決定性（verify・正規化） | 2 |
| DELIV | 成果物受入 | 1 |
| 計 | | 199 |

---

## VO-DOCMODEL（総称 document・宣言鎖のモデル）

- **VO-DOCMODEL-01** — document は `id` / `path` / `content_hash` / `derives_from` を持つ単一の総称ノード型であり、文書種別ごとの専用スキーマ・専用ディレクトリを持たない。
  - derives_from: DOC-DETAIL §3.1 ／ 上流: BASIC §3.1・REQ §3.2
  - 入力 = 要求文書・基本仕様・詳細設計を `doc add` で登録した `.verify/doc/` → 期待 = 3 件とも同一 schema の `DOC-*.yaml` として読め、`spec/` `req/` 等の種別別ディレクトリが生成されない
- **VO-DOCMODEL-02** — VO は 1 件以上の document へ `derives_from` で直結して導出され、VO と document の間に他のエンティティ層を置かない。
  - derives_from: DOC-DETAIL §3.2 ／ 上流: BASIC §3.2
  - 入力 = `vo add --id VO-X --derives-from DOC-BASIC-001` → 期待 = VO レコードが `derives_from: [{doc: DOC-BASIC-001}]` を持ち、`requirements` / `spec_refs` field を持たない
- **VO-DOCMODEL-03** — 検証対象（`targets`）と実装 traceability は別の関係型であり、一方から他方を推定せず、実装 traceability の有無は Test 成立性の条件にならない。
  - derives_from: DOC-BASIC §9.3 ／ 上流: REQ §3.4・BASIC §19
  - 入力 = 実装 traceability を宣言しない Test → 期待 = その欠落だけでは `chain_integrity` を非 PASS にしない
  - 入力 = 実装 traceability だけを宣言し `targets` を宣言しない Test（rust-cargo） → 期待 = `targets` を traceability から補完せず E-SCAN-007（必須 metadata 欠落）
  - 入力 = 実装 traceability を持つ Test → 期待 = `target_binding` / `target_coverage` の対象集合に当該 traceability 先が計上されない
- **VO-DOCMODEL-04** — リンクに付す説明文（`note`）は任意であり、保存・往復して取り出せる一方、欠落・空文字列は `chain_integrity` 違反にも `MISMATCH` にもならない。
  - derives_from: DOC-DETAIL §3.2 ／ 上流: BASIC §3.4・§19
  - 入力 = `note` を空にした `derives_from` entry → 期待 = `chain_integrity` = PASS、診断なし
  - 入力 = `--note "根拠: §8.2条項2"` を付けた `derives_from` entry → 期待 = 保存され `vo show` が同一文字列を返す
- **VO-DOCMODEL-05** — 文書層の段は document 間のリンクで表し、段を増やしてもレコード schema が変わらず検査項目が増えない。
  - derives_from: DOC-REQ §3.2 ／ 上流: REQ §3.3
  - 入力 = DOC-A → DOC-B → DOC-C の 3 段と、DOC-A → DOC-B の 2 段 → 期待 = 双方とも同一の document schema で表現でき、評価される検査は固定 4 検査のまま

---

## VO-VOMODEL（VO の表現能力）

- **VO-VOMODEL-DIM-01** — VO は検証軸（`dimensions`）と各軸の partition を宣言でき、宣言の有無で VO レコードの受理可否が変わらない。
  - derives_from: DOC-DETAIL §3.2.1 ／ 上流: BASIC §10
  - 入力 = `--dimension operand-sign=positive,negative --dimension operator=add,sub` → 期待 = 2 軸 × それぞれ 2 / 2 partition が VO レコードへ保存され、`vo show` が同一構造を返す
  - 入力 = `dimensions` を持たない VO → 期待 = `chain_integrity` = PASS（`dimensions` の欠落を違反にしない）
- **VO-VOMODEL-EXPAND-01** — `coverage_policy: independent-axes` の `vo expand` は、各軸の partition ごとに子 VO を 1 件ずつ実体化する。
  - derives_from: DOC-DETAIL §3.2.1 ／ 上流: BASIC §10
  - 入力 = 軸 `operand-sign`(2 partition) と `operator`(4 partition) を持つ VO-X に `vo expand` → 期待 = 子 VO 6 件（2 + 4）が `VO-X-<PARTITION>` の ID で生成される
- **VO-VOMODEL-EXPAND-02** — `coverage_policy: full-product` の `vo expand` は直積ごとに子 VO を 1 件実体化し、suffix を `dimensions` の宣言順に連結する。
  - derives_from: DOC-DETAIL §3.2.1 ／ 上流: BASIC §10
  - 入力 = 同上の VO-X（`full-product`）に `vo expand` → 期待 = 子 VO 8 件、ID は `VO-X-POSITIVE-ADD` 等で軸の宣言順に連結される
- **VO-VOMODEL-EXPAND-03** — `coverage_policy: explicit` の `vo expand` は `combinations` に列挙された tuple のみを実体化し、entry 内の記述順・map key 順に依存しない。
  - derives_from: DOC-DETAIL §3.2.1 ／ 上流: BASIC §10
  - 入力 = `combinations` が `{operand-sign: positive, operator: div}` と `{operator: div, operand-sign: negative}` の 2 件 → 期待 = 子 VO は `VO-X-POSITIVE-DIV` と `VO-X-NEGATIVE-DIV` の 2 件のみ
- **VO-VOMODEL-COMB-01** — `combinations` が受理条件に違反する VO は E-SCAN-017 とし、当該 VO の `chain_integrity` を `MISMATCH` とし、`vo expand` は子 VO を 1 件も生成しない（部分生成しない）。
  - derives_from: DOC-DETAIL §3.2.1・§17.1 ／ 上流: BASIC §10
  - 入力 = `coverage_policy: explicit` かつ `combinations` が空 list → 期待 = E-SCAN-017、`chain_integrity` = MISMATCH、子 VO 0 件
  - 入力 = `coverage_policy: full-product` かつ `combinations` が非空 → 期待 = 同上
  - 入力 = entry が未宣言の dimension 名を含む → 期待 = 同上
  - 入力 = entry の partition 値が当該 dimension の `partitions` に無い → 期待 = 同上
  - 入力 = entry が宣言済み dimension を 1 つ欠く → 期待 = 同上
  - 入力 = 同一 dimension 名を 2 回持つ entry → 期待 = 同上
  - 入力 = 同一（dimension 名 → partition 値）対応の entry が 2 件 → 期待 = 同上
- **VO-VOMODEL-COMB-02** — `combinations` は CLI の `--combination` と MCP の `vo_upsert.combinations[]` から入力でき、desired state として既存値を置換する。
  - derives_from: DOC-ANNEX-A §12.2 ／ 上流: BASIC §10・DETAIL §3.2.1
  - 入力 = `vo edit VO-X --combination operand-sign=positive,operator=div` を既存 2 tuple の VO へ適用 → 期待 = `combinations` が与えた 1 tuple で置換される（追記されない）
  - 入力 = `vo edit VO-X --clear-combinations` → 期待 = `combinations` が空になる
  - 入力 = `--combination` も `--clear-combinations` も与えない `vo edit` → 期待 = 既存 `combinations` が保持される
- **VO-VOMODEL-HIER-01** — VO は `parent` により階層構造を持て、階層化された VO 群と `parent: null` の flat な VO 群が同一の検証グラフ上で共存する。
  - derives_from: DOC-DETAIL §3.2 ／ 上流: BASIC §10
  - 入力 = `parent: null` の VO と `parent: VO-X` の VO を含む `.verify/vo/` → 期待 = 双方が scan で読め、`chain_integrity` = PASS、`vo list --tree` が階層と最上位を区別して返す
  - 入力 = flat な VO-Y に `vo edit VO-Y --parent VO-X` → 期待 = VO-Y が VO-X の子として集約に参加する
- **VO-VOMODEL-MULT-01** — VO と Test の対応は 1:1 に限らず、1:N・N:1・N:M を受理する。
  - derives_from: DOC-DETAIL §11.3 ／ 上流: BASIC §10
  - 入力 = 1 つの leaf VO を 2 つの Test が covers → 期待 = 双方が当該 VO の合成に参加し、`chain_integrity` = PASS
  - 入力 = 1 つの Test が 2 つの leaf VO を covers → 期待 = 当該 Test の結果が両 VO の合成へ独立に参加し、`chain_integrity` = PASS

---

## VO-TESTMODEL（Test Entity のモデル）

- **VO-TESTMODEL-CASES-01** — adapter が識別した table-driven / parameterized の Test construct 全体を 1 件の Test として登録し、内部の各 case を独立した Test ID へ分解することを必須とせず `cases` 集合として保持する。
  - derives_from: DOC-DETAIL §4.1・§5.2 ／ 上流: BASIC §14・DETAIL §4.2
  - 入力 = `@vtest.case` を 3 行持つ 1 つの table-driven Test construct → 期待 = Test Entity は 1 件、`cases` の要素数は 3、Test ID は 1 個だけ発番される
- **VO-TESTMODEL-INTENT-01** — `input` / `expect` を宣言しない Test を、その欠如だけを理由に構造不完全・`MISMATCH` としない。
  - derives_from: DOC-DETAIL §4.1・§4.4 ／ 上流: BASIC §13
  - 入力 = `id` / `covers` / `intent` / `target` を持ち `input` / `expect` を持たない rust-cargo Test → 期待 = `chain_integrity` = PASS、E-SCAN-007 を出さない
- **VO-TESTMODEL-INTENT-02** — Test に関連付けた `intent` / `input` / `expect` の値を保持し、`test show` の出力と判断バンドルの双方から実装コードを読まずに取得できる。
  - derives_from: DOC-DETAIL §8.1 ／ 上流: BASIC §13・ANNEX-A §12.2
  - 入力 = `intent` / `input` / `expect` を宣言した Test に `test show` → 期待 = 3 値が宣言どおりの文字列として出力に現れる
  - 入力 = 同 Test に `audit bundle` → 期待 = バンドル JSON に同じ 3 値が収録される

---

## VO-CHAIN（検査①`chain_integrity`）

主な derives_from: DOC-ANNEX-C §18.3.1 ／ 上流: DETAIL §11.1.1・§5.4

- **VO-CHAIN-DOC-01** — document の `derives_from` が存在しない document を参照する場合、E-SCAN-012 とし `chain_integrity` = `MISMATCH` とする。
  - derives_from: DOC-DETAIL §5.4 ／ 上流: BASIC §3.2・ANNEX-C §18.3.1
  - 入力 = `derives_from: [DOC-NOPE]`（DOC-NOPE 未登録）の document → 期待 = E-SCAN-012、`chain_integrity` = MISMATCH、終了コード 1
- **VO-CHAIN-DOC-02** — document レコードの `content_hash` が実ファイルと一致しない場合、W-SCAN-104 を出し、当該 document を参照する鎖の `chain_integrity` を `MISMATCH`（診断 `STALE`）とする。
  - derives_from: DOC-DETAIL §11.4 ／ 上流: BASIC §3.2
  - 入力 = 登録後に実ファイルを 1 バイト変更した document → 期待 = W-SCAN-104、当該 document を参照する VO の `chain_integrity` = MISMATCH（診断 STALE）
- **VO-CHAIN-VO-01** — VO は解決可能な `derives_from`（document）を 1 件以上持ち、参照先が存在しなければ E-SCAN-012 とし `chain_integrity` = `MISMATCH` とする。
  - derives_from: DOC-DETAIL §3.2・§5.4 ／ 上流: BASIC §3.2
  - 入力 = `derives_from: [DOC-NOPE]` の VO → 期待 = E-SCAN-012、`chain_integrity` = MISMATCH
- **VO-CHAIN-VO-02** — VO の `parent` が存在しない、または parent 関係が循環する場合、E-SCAN-008 とし `chain_integrity` = `MISMATCH` とする。
  - derives_from: DOC-DETAIL §5.4 ／ 上流: BASIC §10
  - 入力 = `parent: VO-NOPE`（未登録）の VO → 期待 = E-SCAN-008、`chain_integrity` = MISMATCH
  - 入力 = VO-A の parent が VO-B、VO-B の parent が VO-A → 期待 = E-SCAN-008、`chain_integrity` = MISMATCH
- **VO-CHAIN-TEST-01** — 発見された Test construct に対応する管理宣言がちょうど 1 件あり、core 中立の必須 metadata（Test ID・`covers` 1 件以上・`intent`）と当該 adapter の必須 metadata（rust-cargo は `targets` 1 件以上）を満たすことを要求する。欠落は E-SCAN-007 または W-SCAN-101 とし `chain_integrity` = `MISMATCH`（診断 `MISSING`）とする。
  - derives_from: DOC-DETAIL §4.4・§5.4 ／ 上流: BASIC §5.1・§12・ANNEX-C §18.3.1
  - 入力 = `@vtest.covers` を持たない `#[test]` 関数 → 期待 = E-SCAN-007、`chain_integrity` = MISMATCH（診断 MISSING）
  - 入力 = `@vtest.` 宣言を一切持たない `#[test]` 関数 → 期待 = W-SCAN-101（warning 重大度）と同時に `chain_integrity` = MISMATCH（診断 MISSING）が成立する（重大度が検証状態を打ち消さない）
  - 入力 = `intent` を持ち `input` / `expect` を持たず、Test Intent を独立レコードとして持たない Test → 期待 = 検証グラフに Intent ノードが現れず、`chain_integrity` = PASS（Intent の非ノード性）
- **VO-CHAIN-TEST-02** — すべての管理対象 Test に `covers` 1 件以上を一律に要求し、Test の役割・種別による可変制約を設けない。
  - derives_from: DOC-DETAIL §4.1 ／ 上流: BASIC §12・REQ §4.1
  - 入力 = `kind: integration` の Test で `covers` 0 件 → 期待 = E-SCAN-007、`chain_integrity` = MISMATCH（診断 MISSING）。`kind` による緩和が起きない
- **VO-CHAIN-TEST-03** — `covers` が参照するすべての VO が解決でき、解決できない場合は E-SCAN-003 とし `chain_integrity` = `MISMATCH` とする。このとき Test Entity と `ManagedTestLink::One` を除去しない。
  - derives_from: DOC-DETAIL §4.4・§5.4 ／ 上流: BASIC §12
  - 入力 = `covers: VO-NOPE`（未登録）を持つ Test → 期待 = E-SCAN-003、`chain_integrity` = MISMATCH、かつ当該 Test が entity 一覧に残る
- **VO-CHAIN-TEST-04** — 正典レコードの ID 一意性を全数検査し、Test ID の衝突は E-SCAN-002、document / VO の logical record ID の重複は E-SCAN-010 として `chain_integrity` = `MISMATCH` とする。1 つの実行 construct に管理宣言が 2 件以上対応する状態も `MISMATCH` とする。
  - derives_from: DOC-DETAIL §5.4 ／ 上流: BASIC §3.2・§5.1・§12・DETAIL §11.1.1
  - 入力 = 同一 Test ID を宣言する 2 つの Test construct → 期待 = E-SCAN-002、`chain_integrity` = MISMATCH
  - 入力 = 同一 DOC ID を持つ 2 つの `doc/*.yaml` → 期待 = E-SCAN-010、`chain_integrity` = MISMATCH
  - 入力 = 同一 VO ID を持つ 2 つの `vo/*.yaml` → 期待 = E-SCAN-010、`chain_integrity` = MISMATCH
  - 入力 = 1 つの `#[test]` 関数に対し管理宣言が 2 件対応する状態（`ManagedTestLink::Multiple`） → 期待 = `chain_integrity` = MISMATCH（二重定義を見逃さない）
- **VO-CHAIN-BIDIR-01** — covers する Test が 1 件も無い leaf VO を `chain_integrity` = `MISMATCH`（診断 `MISSING`）とする。
  - derives_from: DOC-ANNEX-C §18.3.1 ／ 上流: BASIC §12・DETAIL §11.1.1
  - 入力 = 子 VO を持たず covers する Test も無い VO → 期待 = `chain_integrity` = MISMATCH（診断 MISSING）
- **VO-CHAIN-REL-01** — Relation レコードの端点が存在しない場合は E-SCAN-009、識別子の正規化形が衝突する場合は E-SCAN-010 とし、いずれも `chain_integrity` = `MISMATCH` とする。接頭辞の無い bare ULID の Relation は version 1 互換入力として受理し、読み取りだけでファイルを書き換えない。
  - derives_from: DOC-DETAIL §3.3・§5.4 ／ 上流: BASIC §3.2・ANNEX-C §18.3.1
  - 入力 = `from: TEST-NOPE` の Relation → 期待 = E-SCAN-009、`chain_integrity` = MISMATCH
  - 入力 = bare ULID の Relation と `REL-` 付き同 ULID の Relation が併存 → 期待 = E-SCAN-010、`chain_integrity` = MISMATCH
  - 入力 = ファイル名と record ID が一致しない Relation → 期待 = E-SCAN-010、`chain_integrity` = MISMATCH
  - 入力 = bare ULID の Relation 1 件のみ → 期待 = `REL-` 付き正規形へ in-memory 正規化して受理し、`chain_integrity` = PASS。scan 後もファイルのバイト列が不変
- **VO-CHAIN-DISC-01** — adapter の discovery が失敗した batch（`Incomplete`）を「Test 0 件の正常な scan」として扱わず、E-SCAN-001 とし当該範囲を `UNKNOWN` とする。
  - derives_from: DOC-DETAIL §5.1・§5.4 ／ 上流: BASIC §27
  - 入力 = 構文解析不能な `.rs` を含むリポジトリ → 期待 = E-SCAN-001、batch が `Incomplete`、当該範囲の検査値が `UNKNOWN`（PASS にならない）
- **VO-CHAIN-DECL-01** — Test construct の doc comment 内で `@vtest.` で始まるが test-key を持たない行を E-SCAN-006 とし、無音で無視しない。
  - derives_from: DOC-DETAIL §4.2・§5.4 ／ 上流: BASIC §5.1
  - 入力 = `@vtest.cover VO-X`（`covers` の打鍵ミス）を持つ Test construct → 期待 = E-SCAN-006（error）、`chain_integrity` = MISMATCH
  - 入力 = Test construct の doc comment に `@vtest.src-id SRC-A`（表面の誤配置） → 期待 = E-SCAN-006
- **VO-CHAIN-DECL-02** — Test construct ではない関数 item の doc comment 内で `@vtest.` で始まるが source-target-key を持たない行を W-SCAN-105 とし、無音で無視しない。
  - derives_from: DOC-DETAIL §4.2・§5.4 ／ 上流: BASIC §5.1
  - 入力 = 対象実装側の関数に `@vtest.src_id SRC-A`（打鍵ミス） → 期待 = W-SCAN-105（warning）。当該関数の Source Target は SRC ID 無しとして保持され、検証値は W-SCAN-105 だけでは変わらない
  - 入力 = 対象実装側の関数に `@vtest.src-id` が 2 行 → 期待 = E-SCAN-005、いずれの宣言値も採用せず当該 Source Target の SRC ID は無しとする

---

## VO-ORPHAN（検査②`orphan_detection`）

- **VO-ORPHAN-01** — `derives_from` が空で、かつ他のどの document からも参照されず、`doc.roots` にも列挙されない document を E-SCAN-016 とし `orphan_detection` = `MISMATCH` とする。
  - derives_from: DOC-DETAIL §5.6 ／ 上流: BASIC §5.2・REQ §4.2
  - 入力 = 親も参照元も無く `doc.roots` に無い document → 期待 = E-SCAN-016、`orphan_detection` = MISMATCH
- **VO-ORPHAN-02** — `doc.roots` に列挙された document を根として `orphan_detection` の対象外とする。
  - derives_from: DOC-DETAIL §5.6 ／ 上流: BASIC §5.2
  - 入力 = 親を持たないが `doc.roots` に列挙された document → 期待 = E-SCAN-016 を出さず `orphan_detection` = PASS
- **VO-ORPHAN-03** — `orphan_detection` の対象は文書層のみとし、宣言されていない実装の検出を行わない。
  - derives_from: DOC-DETAIL §5.6 ／ 上流: REQ R-2・BASIC §29（OOS-005）
  - 入力 = どの Test の `targets` からも参照されない実装関数 → 期待 = `orphan_detection` = PASS（当該関数は孤児として報告されない）
- **VO-ORPHAN-04** — `doc.roots` が存在しない DOC ID を参照する場合、config invariant 違反として E-CONFIG-001 とする。
  - derives_from: DOC-DETAIL §5.6 ／ 上流: BASIC §5.2
  - 入力 = `doc.roots: [DOC-NOPE]`（未登録） → 期待 = E-CONFIG-001、終了コード 2

---

## VO-TARGET（検査③`target_binding`）

- **VO-TARGET-STATIC-01** — DA-002 は関数本体および同一ファイル内の呼出先 helper 1 段を探索し、そこに宣言 target の呼出が無く他ファイルへの呼出も無い場合に当該 target の verdict を `FAIL`、他ファイル・他クレートへの呼出があり間接呼出を排除できない場合を `UNKNOWN` とする。
  - derives_from: DOC-DETAIL §7.2 ／ 上流: BASIC §5.3・ANNEX-C §18.3.3
  - 入力 = target を一度も呼ばず同一ファイル内 helper も呼ばない Test → 期待 = DA-002 target 別 verdict = FAIL
  - 入力 = 他ファイルの関数を呼び、その先で target が呼ばれうる Test → 期待 = DA-002 target 別 verdict = UNKNOWN
- **VO-TARGET-STATIC-02** — DA-002 verdict = `FAIL` の target は runtime 証明で覆らない。
  - derives_from: DOC-DETAIL §7.3 ／ 上流: REQ §4.3
  - 入力 = DA-002 = FAIL の target について `target_coverage` result = PASS（count > 0）の鮮度有効 Evidence → 期待 = 当該 target の到達要件は未充足のまま、`target_binding` は非 PASS
- **VO-TARGET-RT-01** — runtime 計測の target 別判定は、実行 count 1 以上を `PASS`、count 0 を `FAIL`（診断 `NOT_EXECUTED`）、対象関数を同定できない場合を `UNKNOWN` とする。実行証拠が 1 件も存在しない場合は `NO_EVIDENCE`（診断 `NOT_EXECUTED`）とし、count 0 の `FAIL` と区別する。
  - derives_from: DOC-DETAIL §10.2 ／ 上流: ANNEX-C §18.3.5・§18.3.4
  - 入力 = `target_coverage.targets[i].count: 3` → 期待 = 当該 target result = PASS
  - 入力 = `target_coverage.targets[i].count: 0` → 期待 = 当該 target result = FAIL、診断 NOT_EXECUTED
  - 入力 = coverage 出力に当該関数が現れない → 期待 = 当該 target result = UNKNOWN、count = null
  - 入力 = 当該 Test の Evidence が 0 件（ビルド失敗で未実行を含む） → 期待 = `target_binding` = NO_EVIDENCE（診断 NOT_EXECUTED）。count 0 の FAIL とは別値
- **VO-TARGET-RT-02** — DA-002 verdict = `UNKNOWN` の target は、鮮度有効な最新 Evidence の `target_coverage` が `checked: true` かつ当該 target result = `PASS`（count > 0）のときに限り §7.3 の到達要件（静的証明または runtime 証明のいずれか）を充足する。
  - derives_from: DOC-DETAIL §7.3 ／ 上流: REQ §4.3
  - 入力 = DA-002 = UNKNOWN、`checked: true`、当該 target count = 2 の鮮度有効 Evidence → 期待 = 当該 target は runtime 到達として充足
  - 入力 = DA-002 = UNKNOWN、`checked: false` → 期待 = 未充足、`target_binding` = NO_EVIDENCE（診断 NOT_CHECKED）
- **VO-TARGET-MULTI-01** — 複数 target Test の `target_binding` は target ごとの到達要件を全宣言 target について評価し、全充足のときだけ `PASS` とする。`target_coverage.checked: true` の Evidence でも、`targets` entry が欠落・重複している、または解決後の canonical Source Target 集合と一致しない場合は合格にしない。
  - derives_from: DOC-DETAIL §7.3・§3.6 ／ 上流: ANNEX-C §18.3.5
  - 入力 = 宣言 target 3 件、うち 1 件が count 0 → 期待 = `target_binding` = FAIL（診断 NOT_EXECUTED）
  - 入力 = 宣言 target 3 件、`target_coverage.targets` が 2 件（entry 欠落） → 期待 = E-SCAN-010、当該 Evidence を有効な結果に使用せず `target_binding` は PASS にならない
  - 入力 = `target_coverage.targets` に同一 canonical Locator が 2 件（重複） → 期待 = 同上
  - 入力 = `target_coverage.targets` に宣言 target 集合に無い canonical Locator が 1 件（余剰） → 期待 = 同上
- **VO-TARGET-BOUNDARY-01** — subprocess・別スレッド等の実行境界を越えた到達は DA-002 の `UNKNOWN` として現れ、coverage が当該 target へ実行を帰属できる場合に限り target 別に `PASS` となる。判定は Test の `kind` ではなく execution topology で決まる。
  - derives_from: DOC-DETAIL §7.3 ／ 上流: ANNEX-C §18.2・§18.3.5
  - 入力 = 別スレッドで target を呼ぶ `kind: unit` の Test、coverage が当該 target count > 0 を計測 → 期待 = DA-002 = UNKNOWN、当該 target は runtime 到達として充足
  - 入力 = 同じ topology で `kind: integration` と宣言した Test → 期待 = `kind` 違いで判定が変わらない
- **VO-TARGET-RESULT-01** — 鮮度有効な Evidence の `result: FAIL`（ランナーが失敗を報告）は `target_binding` = `FAIL` とする。
  - derives_from: DOC-DETAIL §11.2・§7.3 ／ 上流: REQ §5.3
  - 入力 = `result: FAIL` かつ全 target の coverage count > 0 の鮮度有効 Evidence → 期待 = `target_binding` = FAIL（到達充足に優先する）
- **VO-TARGET-PASS-01** — `result: PASS` かつ全宣言 target の到達が充足された場合に `target_binding` = `PASS` とし、未充足の target があるときは原因に応じて count 0 → `FAIL`（診断 `NOT_EXECUTED`）、`checked: false` → `NO_EVIDENCE`（診断 `NOT_CHECKED`）、対象関数を同定できない → `UNKNOWN` とする。
  - derives_from: DOC-DETAIL §11.2・§7.3 ／ 上流: ANNEX-C §18.3.4
  - 入力 = 4 検査に違反が無く、`result: PASS`・`checked: true`・全 target count > 0・全ハッシュ一致・`execution_state.complete: true` の瑕疵なし入力 → 期待 = `target_binding` = PASS（真陽性。すべてを保守側へ降格しない）
  - 入力 = `result: PASS`、1 target が count 0 → 期待 = `target_binding` = FAIL（診断 NOT_EXECUTED）
  - 入力 = `result: PASS`、`checked: false` → 期待 = `target_binding` = NO_EVIDENCE（診断 NOT_CHECKED）
  - 入力 = `result: PASS`、当該 Test の Evidence が 0 件 → 期待 = `target_binding` = NO_EVIDENCE（診断 NOT_EXECUTED）。count 0 の FAIL と区別される
- **VO-TARGET-CONTRACT-01** — 宣言 target をどの topology でも実行しない Test（構造・契約のみを assert する Test）は、静的にも runtime にも到達を確立できず、到達要件が未充足のままとなる。
  - derives_from: DOC-DETAIL §7.3 ／ 上流: ANNEX-C §18.3.3・BASIC §5.3
  - 入力 = target を呼ばず型・定数の構造のみを assert する rust-cargo Test（`targets` 1 件宣言） → 期待 = DA-002 = FAIL、`target_binding` は非 PASS
- **VO-TARGET-CAP-01** — coverage の capability・ツールが利用できない、または `--items` / `--fast` で意図的に省略された検査は `NO_EVIDENCE`（診断 `NOT_CHECKED`）として保持し `PASS` へ変換しない。adapter が解析限界を報告した場合は `UNKNOWN` とする。
  - derives_from: DOC-ANNEX-C §18.3.5 ／ 上流: BASIC §5.3・§22.3・ANNEX-C §18.3.3
  - 入力 = coverage ツール不在（W-EXEC-101） → 期待 = `target_binding` = NO_EVIDENCE（診断 NOT_CHECKED）
  - 入力 = `verify --items chain_integrity`（`target_binding` を意図的に省略） → 期待 = `target_binding` = NO_EVIDENCE（診断 NOT_CHECKED）。PASS へ変換されない
  - 入力 = adapter が解析限界を報告（W-ADAPTER-102） → 期待 = 当該検査 = UNKNOWN（NOT_CHECKED とは別値）
- **VO-TARGET-NODA003-01** — runtime coverage は DA-003 を代替せず、DA-003 の `UNKNOWN` / `FAIL` はそのまま `oracle_presence` へ寄与する。
  - derives_from: DOC-DETAIL §7.3 ／ 上流: DETAIL §7.2
  - 入力 = DA-003 = UNKNOWN、当該 target の coverage count > 0 → 期待 = `oracle_presence` = UNKNOWN のまま（coverage による昇格が起きない）
- **VO-TARGET-RESOLVE-DIAG-01** — 宣言 target が解決できないとき、対象が実在しない場合（診断 `MISSING`）と複数候補で曖昧な場合とを、同一の状態値・診断へ潰さず別々に区別したうえで、いずれも `MISMATCH` とする。
  - derives_from: DOC-DETAIL §5.4・§11.2 ／ 上流: ANNEX-C §18.3.4・§18.3.6
  - 入力 = `@vtest.target src/nope.rs::missing_fn`（実在しない locator） → 期待 = E-SCAN-004、`MISMATCH`、診断 MISSING
  - 入力 = 複数の Source Target へ解決しうる SRC ID 参照（E-SCAN-011 の衝突） → 期待 = `MISMATCH`、診断は MISSING ではない別の曖昧診断。候補の 1 件を解決結果として選ばない
- **VO-TARGET-IGNORE-01** — `#[ignore]` により実行対象から除外された Test は、実行されなかった帰結として `target_binding` に診断 `NOT_EXECUTED` が付く。
  - derives_from: DOC-DETAIL §7.2（W-DA-101） ／ 上流: ANNEX-C §18.3.3
  - 入力 = `#[ignore]` を持つ管理対象 Test に `run` → 期待 = `target_binding` が非 PASS で診断 NOT_EXECUTED を持つ

---

## VO-ORACLE（検査④`oracle_presence`）

- **VO-ORACLE-DA001** — 関数内の assert 相当がすべて定数アサーション（引数がすべてリテラル・定数式）である場合、DA-001 を `FAIL` とする。
  - derives_from: DOC-DETAIL §7.2 ／ 上流: BASIC §8.2
  - 入力 = 本体が `assert!(true);` だけの Test → 期待 = DA-001 = FAIL、`oracle_presence` = FAIL
- **VO-ORACLE-DA003** — target を呼ぶがその呼出結果（戻り値および結果から派生した束縛）が assert 相当に到達せず `#[should_panic]` も無い場合、DA-003 を `FAIL` とする。
  - derives_from: DOC-DETAIL §7.2 ／ 上流: BASIC §8.2
  - 入力 = `let _ = parse(input);` のみで結果を検証しない Test → 期待 = DA-003 = FAIL、`oracle_presence` = FAIL
- **VO-ORACLE-DA004** — `assert_eq!(a, b)` の a と b がトークン列として同一である assert が存在する場合、DA-004 を `FAIL` とする。
  - derives_from: DOC-DETAIL §7.2 ／ 上流: BASIC §8.2
  - 入力 = `assert_eq!(x, x);` を含む Test → 期待 = DA-004 = FAIL、`oracle_presence` = FAIL
- **VO-ORACLE-DA005** — 関数本体に文が 1 つも存在しない場合、DA-005 を `FAIL` とする。
  - derives_from: DOC-DETAIL §7.2 ／ 上流: BASIC §8.2
  - 入力 = 本体が空の `#[test]` 関数 → 期待 = DA-005 = FAIL、`oracle_presence` = FAIL
- **VO-ORACLE-DA006** — 関数内に assert 相当が 1 つも存在しない場合、DA-006 を `FAIL` とする。
  - derives_from: DOC-DETAIL §7.2 ／ 上流: BASIC §8.2
  - 入力 = target を呼ぶだけで assert 相当を持たない Test → 期待 = DA-006 = FAIL、`oracle_presence` = FAIL
- **VO-ORACLE-COMPOSE** — `oracle_presence` は DA-001 / DA-003 / DA-004 / DA-005 / DA-006 の合成とし、全ルール違反なしなら `PASS`、1 つでも `FAIL` があれば `FAIL`、`FAIL` が無く `UNKNOWN` があれば `UNKNOWN` とする。
  - derives_from: DOC-DETAIL §7.1 ／ 上流: BASIC §5.4・REQ §4.4
  - 入力 = DA-004 = FAIL、他は違反なし → 期待 = `oracle_presence` = FAIL
  - 入力 = DA-003 = UNKNOWN、他は違反なし → 期待 = `oracle_presence` = UNKNOWN
  - 入力 = 全ルール違反なし → 期待 = `oracle_presence` = PASS
- **VO-ORACLE-NOPROMOTE** — `oracle_presence` に runtime 昇格経路は無く、静的な証明の失敗は `UNKNOWN` のままで runtime 証拠により `PASS` にならない。
  - derives_from: DOC-DETAIL §7.1 ／ 上流: REQ §4.4
  - 入力 = `oracle_presence` = UNKNOWN の Test に `result: PASS` かつ全 target count > 0 の鮮度有効 Evidence → 期待 = `oracle_presence` = UNKNOWN のまま
- **VO-ORACLE-CONSERV** — 決定論的に確定できる違反のみ `FAIL` とし、確定できない場合は `FAIL` ではなく `UNKNOWN` へ退避する。
  - derives_from: DOC-DETAIL §7.1・§7.2 ／ 上流: BASIC §8.2
  - 入力 = クロージャ内でのみ target 結果が assert へ到達する Test → 期待 = DA-003 = UNKNOWN（FAIL にしない）
  - 入力 = マクロ展開内でのみ assert 相当が現れる Test → 期待 = DA-006 = UNKNOWN
  - 入力 = assert 相当の引数の定数性を静的に確定できない式（実行時に決まる値を含む定数畳み込み候補） → 期待 = DA-001 = UNKNOWN（FAIL にしない）
- **VO-ORACLE-SUBPROC-SPLIT** — 宣言 target への呼出が Test 本体に静的に現れない場合、DA-003 の当該 target 別 verdict を `UNKNOWN` とし、DA-002 が runtime 証明で救済されても `oracle_presence` は `UNKNOWN` のままとする。
  - derives_from: DOC-DETAIL §7.2・§7.3 ／ 上流: ANNEX-C §18.3.3
  - 入力 = subprocess を起動して target を実行し親プロセスで stdout を assert する Test、当該 target の coverage count > 0 → 期待 = `target_binding` = PASS になりうる一方 `oracle_presence` = UNKNOWN、総合は非 PASS
- **VO-ORACLE-RECALC** — 静的解析の結果は正典レコードを持たない再計算派生であり、監査レコードとして永続化しない。
  - derives_from: DOC-DETAIL §7.1 ／ 上流: BASIC P-003
  - 入力 = `audit static` を実行した後の `.verify/` → 期待 = `.verify/audits/` 等の正典ディレクトリが生成されず、出力は stdout と `cache/` のみ。`audit_static` は正典レコード ID を返さない
- **VO-ORACLE-FRAGMENT** — 解析対象の source fragment の完全性を保証できない場合、当該ルールを `UNKNOWN` とし違反なしと推測しない。
  - derives_from: DOC-DETAIL §7.1 ／ 上流: ANNEX-C §18.3.3
  - 入力 = adapter が解析入力集合の不完全性を報告した Test → 期待 = 当該ルール = UNKNOWN、`oracle_presence` = UNKNOWN
- **VO-ORACLE-DA003-MULTI-01** — 複数の宣言 target を持つ Test では DA-003 を target ごとに個別適用し、target 別 verdict 集合を全宣言 target と過不足なく 1 対 1 に対応させ、1 件でも `FAIL` があれば規則結果を `FAIL`、`FAIL` が無く `UNKNOWN` があれば `UNKNOWN`、全 target が違反なしのときだけ `PASS` へ畳み込む。
  - derives_from: DOC-DETAIL §7.2・§7.1 ／ 上流: BASIC §8.2
  - 入力 = 宣言 target 3 件、うち 1 件の結果のみ assert へ到達しない → 期待 = target 別 verdict 3 件、規則結果 DA-003 = FAIL
  - 入力 = 宣言 target 3 件、1 件が UNKNOWN・2 件が違反なし → 期待 = 規則結果 DA-003 = UNKNOWN
  - 入力 = 宣言 target 3 件すべて違反なし → 期待 = target 別 verdict 3 件、規則結果 DA-003 = PASS
- **VO-ORACLE-TRUENEG-01** — 違反構造を持たない正常な Test について DA-001 / DA-003 / DA-004 / DA-005 / DA-006 がいずれも違反なしを返し、`.unwrap()` / `.expect(..)` / `?` 演算子 / `#[should_panic]` / `Result` の `Err` 返しを assert 相当として認める。
  - derives_from: DOC-DETAIL §7.2 ／ 上流: ANNEX-C §18.3.3
  - 入力 = target を呼びその戻り値を `assert_eq!` で検証する Test → 期待 = 5 ルールすべて違反なし、`oracle_presence` = PASS
  - 入力 = target の戻り値に `.unwrap()` を適用するだけの Test → 期待 = assert 相当と認め DA-006 違反なし、`oracle_presence` = PASS
  - 入力 = `#[should_panic]` を持ち target を呼ぶ Test → 期待 = DA-003 / DA-006 とも違反なし
  - 入力 = `Result` を返し `?` で伝播する Test → 期待 = DA-006 違反なし
- **VO-ORACLE-TERM-01** — assert 相当の引数が標準 assert 集合に属さない helper 呼出である Test は、当該 helper が別の Test の宣言 target であり、その Test の `oracle_presence` が `PASS` であるときに限り照合装置の成立側を確認済みとし、そうでなければ `UNKNOWN` とする。
  - derives_from: DOC-DETAIL §7.2（暫定。ESCALATE-2） ／ 上流: BASIC §8.2条項2・§5.4
  - 入力 = 判定を helper `assert_valid(..)` へ委譲する Test A、helper が Test B の宣言 target であり Test B の `oracle_presence` = PASS → 期待 = Test A の `oracle_presence` = PASS
  - 入力 = 同じ Test A、helper をいずれの Test も target として宣言していない → 期待 = Test A の `oracle_presence` = UNKNOWN（FAIL にしない）
  - 入力 = 同じ Test A、helper が Test B の target だが Test B の `oracle_presence` = UNKNOWN → 期待 = Test A の `oracle_presence` = UNKNOWN
- **VO-ORACLE-TERM-02** — helper への委譲関係が循環する場合、`oracle_presence` を `UNKNOWN` とし、循環中のいずれかを推測で成立とみなさない。
  - derives_from: DOC-DETAIL §7.2（暫定。ESCALATE-2） ／ 上流: BASIC §8.2条項2
  - 入力 = Test A の判定 helper が Test B の target、Test B の判定 helper が Test A の target → 期待 = 双方とも `oracle_presence` = UNKNOWN
- **VO-ORACLE-TERM-03** — 標準 assert 相当のみ、または `rust-cargo` config の `assertion_macros` に列挙されたマクロのみを照合装置として持つ Test を、DA-006 および `oracle_presence` で不合格にしない。
  - derives_from: DOC-DETAIL §7.2 ／ 上流: BASIC §8.2
  - 入力 = `assert_eq!` のみで判定する Test → 期待 = DA-006 違反なし、`oracle_presence` = PASS
  - 入力 = `assertion_macros: [my_assert]` を config に列挙し `my_assert!` のみで判定する Test → 期待 = DA-006 違反なし、`oracle_presence` = PASS
  - 入力 = 同じ Test で `assertion_macros` の列挙を config から外す → 期待 = `my_assert!` は標準 assert 集合に属さず VO-ORACLE-TERM-01 の委譲判定へ回る
- **VO-ORACLE-IGNORE-01** — `#[ignore]` 属性は W-DA-101 の警告のみとし、`oracle_presence` を `FAIL` にしない。
  - derives_from: DOC-DETAIL §7.2 ／ 上流: ANNEX-C §18.3.3
  - 入力 = `#[ignore]` を持ち assert 相当を備えた Test → 期待 = W-DA-101（warning）、`oracle_presence` = PASS

---

## VO-STATE（5 状態と診断ラベルの 2 軸）

- **VO-STATE-01** — 検証状態は `PASS` / `FAIL` / `MISMATCH` / `NO_EVIDENCE` / `UNKNOWN` の 5 値のみとする。
  - derives_from: DOC-ANNEX-A §12.1 ／ 上流: BASIC §4.1・ANNEX-C §18.2
  - 入力 = `verify --format json` の任意の検査ノード → 期待 = `state` field の値が 5 値のいずれかであり、`MISSING` / `NOT_EXECUTED` / `NOT_CHECKED` / `STALE` が `state` に現れない
- **VO-STATE-02** — 診断ラベル `MISSING` / `NOT_EXECUTED` / `NOT_CHECKED` / `STALE` は検証状態と別軸であり、`state` の値に用いず `diagnostic` field へ併記する。
  - derives_from: DOC-ANNEX-A §12.1 ／ 上流: BASIC §4.2
  - 入力 = 管理宣言を欠く Test → 期待 = `state: MISMATCH`、`diagnostic: [MISSING]` の 2 列として出力される
  - 入力 = 集約の代表値選択 → 期待 = 診断ラベルが優先順位に用いられない

---

## VO-EVIDENCE（Evidence の生成・整合・鮮度）

- **VO-EVIDENCE-BIND-01** — Evidence は検証対象の内容ハッシュに束縛し、Evidence ストアはハッシュキーを必須とする。
  - derives_from: DOC-DETAIL §3.6 ／ 上流: REQ §6・BASIC §6
  - 入力 = `hashes.test_subject` を持たない Evidence を書き込む要求 → 期待 = writer が拒否し record を生成しない
- **VO-EVIDENCE-FRESH-SUBJECT** — `hashes.test_subject` が現在の Test subject hash と一致しない Evidence を `NO_EVIDENCE`（診断 `STALE`）とする。Test construct 本体を変えない metadata 変更、および実行座標（`ExecutionDescriptor`）のみの変更でも一致は失われる。
  - derives_from: DOC-DETAIL §11.2 ／ 上流: BASIC §6・DETAIL §1.3
  - 入力 = Test の `intent` 行だけを書き換えた後の既存 Evidence → 期待 = `target_binding` = NO_EVIDENCE（診断 STALE）
  - 入力 = Test の実行座標（adapter / project / suite / selector）だけを変更した後の既存 Evidence → 期待 = NO_EVIDENCE（診断 STALE）
- **VO-EVIDENCE-FRESH-TARGET** — `hashes.targets` の canonical Locator 集合または `target_construct` hash が現在値と一致しない Evidence を `NO_EVIDENCE`（診断 `STALE`）とする。
  - derives_from: DOC-DETAIL §11.2 ／ 上流: BASIC §6
  - 入力 = target 実装の本体を変更した後の既存 Evidence → 期待 = NO_EVIDENCE（診断 STALE）
- **VO-EVIDENCE-FRESH-REVISION** — `revision.commit` が存在しない、または現在の HEAD と一致しない Evidence を `NO_EVIDENCE`（診断 `STALE`）とする。
  - derives_from: DOC-DETAIL §11.2 ／ 上流: BASIC §6
  - 入力 = 記録後に別 commit へ進めたリポジトリの既存 Evidence → 期待 = NO_EVIDENCE（診断 STALE）
- **VO-EVIDENCE-FRESH-EXECSTATE** — `execution_state` の subject hash が現在再構築したものと一致しない Evidence を `NO_EVIDENCE`（診断 `STALE`）とし、`complete: false`（snapshot 不完全）は `UNKNOWN` とする。
  - derives_from: DOC-ANNEX-C §18.3.4 ／ 上流: DETAIL §1.3・§3.6
  - 入力 = repository 内 helper だけを変更した後の既存 Evidence → 期待 = NO_EVIDENCE（診断 STALE）
  - 入力 = `execution_state.complete: false`・`hash: null` の Evidence → 期待 = 現在の有効な PASS 証拠に使用せず、当該検査 = UNKNOWN
  - 入力 = `execution_state` を持たない互換 Evidence → 期待 = 履歴表示は可、`NO_EVIDENCE`（診断 STALE）
- **VO-EVIDENCE-FRESH-ADAPTER** — Evidence の `adapter` が現在の Test の adapter と明示的に不一致なら `MISMATCH`、互換 Evidence で adapter を一意に確認できない場合は `UNKNOWN` とし、いずれも `PASS` へ昇格しない。
  - derives_from: DOC-DETAIL §3.6 ／ 上流: BASIC §27
  - 入力 = `adapter: other-runner` の Evidence と `rust-cargo` の現在 Test → 期待 = MISMATCH
  - 入力 = `adapter` 欠落の互換 Evidence で runner kind・内容ハッシュから Rust 実行を一意に確認できない → 期待 = UNKNOWN
- **VO-EVIDENCE-GEN-01** — Evidence の生成 precondition は全宣言 target が一意に解決済みであることとし、1 件でも対象なし・曖昧があれば Evidence を生成せず、部分 targets の Evidence も作らない。宣言 target が複数候補へ曖昧化した状態の `target_binding` は `MISMATCH` とする。
  - derives_from: DOC-DETAIL §9.4・§11.2 ／ 上流: ANNEX-C §18.3.4・§18.3.6
  - 入力 = 宣言 target 2 件のうち 1 件が複数候補へ曖昧化（E-SCAN-011） → 期待 = Evidence を生成せず、`target_binding` = MISMATCH（`NO_EVIDENCE` にしない）
  - 入力 = 宣言 target 2 件のうち 1 件が実在しない → 期待 = Evidence を生成せず、`target_binding` = MISMATCH（診断 MISSING）
- **VO-EVIDENCE-GEN-02** — ビルド失敗・ランナー失敗・capability 欠落・target 解決失敗・実行前後の Execution State subject 変化（E-EXEC-004）のいずれかがあるとき Evidence を生成しない。
  - derives_from: DOC-DETAIL §9.4・§17.1 ／ 上流: ANNEX-C §18.3.4
  - 入力 = テストビルド失敗（E-EXEC-001） → 期待 = Evidence 0 件、終了コード 2
  - 入力 = 実行中に Execution State subject が変化（E-EXEC-004） → 期待 = Evidence 0 件
- **VO-EVIDENCE-NOFALLBACK** — 最新 Evidence が鮮度を満たさないとき、より古い鮮度有効な Evidence へフォールバックしない。
  - derives_from: DOC-DETAIL §7.3・§11.2 ／ 上流: BASIC §6
  - 入力 = 新しい STALE Evidence と、それより古い鮮度有効 Evidence が併存 → 期待 = `target_binding` = NO_EVIDENCE（診断 STALE）。古い Evidence で PASS にしない
- **VO-EVIDENCE-HASH-CORE** — 内容ハッシュは adapter が自己確定せず、core が言語非依存の正規化で計算する（adapter はハッシュ未計算の DTO を返す）。
  - derives_from: DOC-DETAIL §1.3・§5.1 ／ 上流: BASIC §6
  - 入力 = adapter が hash field を埋めた DiscoveryBatch → 期待 = core が当該値を採用せず自ら計算した hash を用いる
- **VO-EVIDENCE-INTEGRITY-01** — Evidence の schema 違反、`target` entry の欠落・重複・余剰、および集約結果と target 別結果の矛盾を E-SCAN-010 として検出し、当該 Evidence を有効な結果として使用しない。
  - derives_from: DOC-DETAIL §3.6・§10.2 ／ 上流: ANNEX-C §18.3.4
  - 入力 = `target_coverage.result: PASS` だが `targets[i].result: FAIL` を含む Evidence → 期待 = E-SCAN-010、当該 Evidence を消費せず `target_binding` は PASS にならない
  - 入力 = schema にない field 型で書かれた Evidence → 期待 = E-SCAN-010、同上
- **VO-EVIDENCE-DIRTY-01** — `revision.dirty: true` であっても `execution_state` subject が現在再構築したものと一致する Evidence は鮮度有効として合格側へ通す。
  - derives_from: DOC-DETAIL §11.2 ／ 上流: ANNEX-C §18.3.4
  - 入力 = `revision.dirty: true`、`execution_state.complete: true` かつ hash が現在値と一致、全ハッシュ一致 → 期待 = 鮮度有効。dirty だけを理由に NO_EVIDENCE（STALE）にしない
- **VO-EVIDENCE-SELECT-01** — 選択した登録 Test だけをランナーの exact selector で実行し、選択外の Test を実行しない。
  - derives_from: DOC-ANNEX-C §18.3.4 ／ 上流: DETAIL §9.1・§9.2
  - 入力 = 同一モジュールに 2 つの Test があり片方だけを `run --test TEST-A` → 期待 = runner コマンドが `-- --exact` で TEST-A のみを指定し、TEST-B の Evidence が生成されない

---

## VO-AUTHORITY（合否判定権威）

- **VO-AUTHORITY-01** — 合否の判定権威は adapter の runner であり、本システムは合否を再判定せず runner が報告した `result` をそのまま証拠として消費する。`rust-cargo` では `cargo test` の ok / FAILED 行が権威となる。
  - derives_from: DOC-DETAIL §3.6・§9.3 ／ 上流: REQ §7・BASIC §7
  - 入力 = runner が ok を報告した Test → 期待 = Evidence `result: PASS` として記録され、独自の合否判定が代入されない
  - 入力 = runner が FAILED を報告した Test → 期待 = Evidence `result: FAIL` としてそのまま取り込み、`target_binding` = FAIL（再判定しない）
  - 入力 = `cargo test` 出力の ok 行と exit code が矛盾 → 期待 = E-EXEC-003 とし、独自の合否を代入しない
- **VO-AUTHORITY-02** — `target_binding` は runner の `result` を前提としたうえで「宣言対象の実行を伴ったか」を問う独立した照合であり、runner の合否を上書きしない。
  - derives_from: DOC-DETAIL §7.3 ／ 上流: BASIC §7
  - 入力 = `result: PASS` かつ全 target count 0 → 期待 = `target_binding` = FAIL（診断 NOT_EXECUTED）。Evidence の `result` は PASS のまま記録される

---

## VO-DECISION（判断記録・バンドル・実効判断）

主な derives_from: DOC-DETAIL §8 ／ 上流: BASIC §11・ANNEX-C §18.3.6

- **VO-DECISION-NONGATE** — 判断記録の受理は検証状態（5 状態）を昇格させない。
  - derives_from: DOC-DETAIL §8.5・§8.6 ／ 上流: BASIC §11.3
  - 入力 = `oracle_presence` = UNKNOWN の Test に `decision: accepted` の判断記録を提出 → 期待 = 受理されるが `oracle_presence` = UNKNOWN のまま
- **VO-DECISION-REASON-OPT** — 判断記録は `actor` / `subject` / `decision` を必須とし理由を任意とする。理由が空であることを根拠に無効・`UNKNOWN`・`NO_EVIDENCE`・`MISMATCH` として扱わない。
  - derives_from: DOC-DETAIL §8.3・§17.1 ／ 上流: REQ §12・BASIC §11.3
  - 入力 = `claim` / `basis` を空にした判断記録の提出 → 期待 = 受理され、E-AUDIT-005 / 006 / 007 に相当する拒否が起きない
- **VO-DECISION-HASHBIND** — 受理した判断記録は `subject_hash` と依存 closure に束縛され、対象または依存の変更で無効となる。依存 document の `content_hash` が実ファイルと一致しない場合も当該判断記録は無効となる。
  - derives_from: DOC-DETAIL §8.5 ／ 上流: BASIC §11.3
  - 入力 = 判断記録の受理後に対象 Test を変更 → 期待 = 当該判断記録は無効（有効判断集合 V に属さない）
  - 入力 = 依存 document の実ファイルを変更（W-SCAN-104） → 期待 = 当該 document を STALE とし、依存する判断記録も無効
- **VO-DECISION-REVERIFY** — 仕様・VO・Test が変更された場合、過去の判断を現在状態へ流用せず 4 検査を再実施し、その結果は 5 状態のいずれにもなりうる。変更そのものが `UNKNOWN` を生成しない。
  - derives_from: DOC-DETAIL §8.5 ／ 上流: REQ §12・BASIC §11.3
  - 入力 = `accepted` の判断記録を持つ Test の本体を変更し `verify` → 期待 = 4 検査が再計算され、結果が変更前の判断値から導出されない
- **VO-DECISION-SUBMIT-01** — 提出された `bundle_id` が存在しない判断記録提出を E-AUDIT-001 として拒否する。
  - derives_from: DOC-DETAIL §8.4・§17.1 ／ 上流: ANNEX-C §18.3.6
  - 入力 = 存在しない `bundle_id` を持つ提出（他 3 条件は充足） → 期待 = E-AUDIT-001、終了コード 2、判断記録を生成しない
- **VO-DECISION-SUBMIT-02** — 提出の `subject` または `judgment_kind` がバンドルと一致しない、値域外、または schema 違反である提出を E-AUDIT-003 として拒否する。
  - derives_from: DOC-DETAIL §8.4・§17.1 ／ 上流: ANNEX-C §18.3.6
  - 入力 = バンドルの subject と異なる subject を書いた提出（他 3 条件は充足） → 期待 = E-AUDIT-003、判断記録を生成しない
- **VO-DECISION-SUBMIT-03** — バンドル記録時のハッシュと現在のハッシュが一致しない提出を E-AUDIT-002 として拒否する。
  - derives_from: DOC-DETAIL §8.4・§17.1 ／ 上流: ANNEX-C §18.3.6
  - 入力 = バンドル生成後に対象 Test を変更してから提出（他 3 条件は充足） → 期待 = E-AUDIT-002、判断記録を生成しない
- **VO-DECISION-SUBMIT-04** — `decision` が受理する判断値でない提出を E-AUDIT-004 として拒否する。
  - derives_from: DOC-DETAIL §8.4・§17.1 ／ 上流: ANNEX-C §18.3.6
  - 入力 = `decision: maybe`（値域外）の提出（他 3 条件は充足） → 期待 = E-AUDIT-004、判断記録を生成しない
- **VO-DECISION-BUNDLE-01** — 判断バンドルは派生情報として `cache/bundles/` へ出力し、Git 管理対象外とする。
  - derives_from: DOC-DETAIL §8.1・§2.1 ／ 上流: BASIC §24.1
  - 入力 = `audit bundle` の実行 → 期待 = 生成物が `.verify/cache/bundles/<id>.json` に置かれ、`.verify/.gitignore` の `cache/` により Git 管理対象外となる
- **VO-DECISION-BUNDLE-02** — 対象を一意に解決できない場合はバンドルを生成せず、候補の 1 件を選択しない。
  - derives_from: DOC-DETAIL §8.1 ／ 上流: ANNEX-C §18.3.6
  - 入力 = 宣言 target が複数候補へ曖昧化した Test に `audit bundle` → 期待 = バンドルを生成せず、候補を解決結果として採用しない
- **VO-DECISION-BUNDLE-03** — 生成された判断バンドルは規範的項目列（対象 VO・Test Intent・Test の `cases`・テストコード・対象実装・関連テスト・既知 partition・過去判断・対象の内容ハッシュ・リビジョン）を実際に含む。
  - derives_from: DOC-DETAIL §8.1・§11.3 ／ 上流: BASIC §11.3・§14
  - 入力 = `cases` を 3 件持ち過去判断 1 件を持つ Test に `audit bundle` → 期待 = バンドル JSON に上記 10 項目がすべて現れ、`cases` 3 件と過去判断 1 件が構造化された値として収録される
- **VO-DECISION-PROV-01** — 決定論的な静的解析の結果と、エージェントまたは人間による判断の結果とを、保存場所と出力の双方で区別する。
  - derives_from: DOC-DETAIL §7.1・§8.1 ／ 上流: ANNEX-C §18.3.6
  - 入力 = `audit static` と `audit submit` を実行した後の `.verify/` → 期待 = 静的解析結果は `cache/` にのみ置かれ正典レコードを持たず、判断は `.verify/decisions/<ULID>.yaml` に置かれる
  - 入力 = `report --format json` → 期待 = 静的解析由来の項目（`basis.kind: da-rule`）と判断由来の項目（`basis.kind: decision`）が別の出所として区別できる
- **VO-DECISION-EFFECTIVE-01** — 同一 `(subject, judgment_kind)` に判断値の食い違う有効判断記録が `supersedes` 関係なく併存する場合、実効判断を未確定（`UNKNOWN`）とし W-STORE-004 を出す。新旧・`decision` 値の優先順位・件数の多寡を解消規則に用いない。
  - derives_from: DOC-DETAIL §8.5 ／ 上流: BASIC §11
  - 入力 = 同一対象・同一 `judgment_kind` に `accepted` と `rejected` の有効判断記録が併存（`supersedes` なし） → 期待 = 実効判断 = 未確定（UNKNOWN）、W-STORE-004。どちらも採用されない
  - 入力 = 同上で `rejected` の方が新しい ULID → 期待 = 同じく未確定（最新勝ちを採らない）
- **VO-DECISION-EFFECTIVE-02** — 判断の競合は、新しい判断記録が旧判断記録を `supersedes` で明示に名指ししたときにだけ解消する。`supersedes` が循環して実効集合が空になる場合は未確定（`UNKNOWN`）とし W-STORE-005 を出す。
  - derives_from: DOC-DETAIL §8.5 ／ 上流: BASIC §11
  - 入力 = `accepted` の判断記録が既存 `rejected` の ULID を `supersedes` に名指し → 期待 = 実効判断 = accepted
  - 入力 = 2 件の判断記録が互いを `supersedes` に名指し → 期待 = 実効集合が空、未確定（UNKNOWN）、W-STORE-005。いずれかを推測で残さない
  - 入力 = 無効な判断記録の `supersedes` が有効判断記録を名指し → 期待 = 何も除外されない
- **VO-DECISION-CASECOV-01** — `judgment_kind: case-coverage` の判断待ち項目を、covers 1 件以上・`cases` 1 件以上または解決済み covers 先 VO が `dimensions` 1 件以上・実効判断が `accepted` でない、の 3 条件をすべて満たす管理対象 Test ごとにちょうど 1 件生成し、`check: null` として集約へ寄与させない。
  - derives_from: DOC-DETAIL §11.7 ／ 上流: BASIC §14・ANNEX-A §12.4
  - 入力 = covers 1 件・`cases` 2 件・当該対象の実効判断が未確定の Test → 期待 = `pending` に `judgment_kind: case-coverage`・`check: null` の項目が 1 件生成され、検査値は変わらない
  - 入力 = 同 Test で実効判断が `accepted` → 期待 = 当該項目が生成されない
  - 入力 = 同 Test で実効判断が `deferred` → 期待 = 項目が生成され、参照した判断記録 ID が `basis` に載る
  - 入力 = `cases` 0 件かつ covers 先 VO が `dimensions` を持たない Test → 期待 = 当該項目が生成されない

---

## VO-APPROVAL（承認）

主な derives_from: DOC-DETAIL §3.5 ／ 上流: BASIC §4.5・§17・ANNEX-C §18.3.7

- **VO-APPROVAL-INDEP** — 承認は検証状態と独立の別軸であり、承認済みを理由に非 `PASS` を `PASS` へ昇格させず、未承認を理由に `PASS` を降格させない。
  - derives_from: DOC-DETAIL §3.5 ／ 上流: BASIC §4.5・§17
  - 入力 = `oracle_presence` = FAIL の Test を covers する VO に実効承認 `approved` → 期待 = 検証状態は FAIL のまま
  - 入力 = 4 検査すべて PASS の VO に承認レコードが 1 件も無い → 期待 = 検証状態は PASS のまま（`UNKNOWN` 等へ降格しない）。実効承認だけが `draft`
- **VO-APPROVAL-DISTINCT** — 判断済みと承認済みを区別し、承認レコードが判断記録と同一 entity であることを要求しない。
  - derives_from: DOC-DETAIL §8.5・§3.5 ／ 上流: BASIC §11.3・§17
  - 入力 = 実効判断が `accepted` で承認レコードが 0 件の VO → 期待 = 実効承認 = draft（判断が承認を導出しない）
- **VO-APPROVAL-CLOSURE** — VO の実効承認は、`subject_hash` の一致に加え、依存 closure（再帰的な parent VO・`derives_from` 先 document・各 document の再帰的な上位 document）が entity・hash とも完全一致する承認レコードからのみ導出する。
  - derives_from: DOC-DETAIL §3.5 ／ 上流: BASIC §17
  - 入力 = parent VO の hash だけが承認時と異なる承認レコード → 期待 = 有効承認集合に属さず、実効承認 = draft
  - 入力 = `dependencies` に余剰 entry を含む承認レコード → 期待 = 完全一致を満たさず draft
- **VO-APPROVAL-STALE** — 対象または依存成果物の変更（document の再登録を含む）により承認が失効する。
  - derives_from: DOC-DETAIL §3.5・§11.4 ／ 上流: BASIC §17
  - 入力 = 実効承認 `approved` の VO の `claim` を変更 → 期待 = `subject_hash` 不一致で実効承認 = draft
  - 入力 = 依存 document を `doc add --update` で再登録 → 期待 = document subject hash が変化し、実効承認 = draft
- **VO-APPROVAL-RECORD** — 承認レコードは `approver`（種別 `human` / `agent` と識別子）、`subject` または `judgment_ref`、`approved_state` を必須項目として持ち、根拠（`basis`）は任意とする。
  - derives_from: DOC-DETAIL §3.5 ／ 上流: BASIC §17
  - 入力 = `approver.kind` を欠く承認作成要求 → 期待 = record を生成しない
  - 入力 = `basis` を空にした承認作成要求 → 期待 = 受理され record が生成される
- **VO-APPROVAL-COMPAT-01** — 依存 closure またはハッシュを欠く互換 Approval から `approved` を導出せず、W-STORE-002 を出して対象を `draft` 相当とする。
  - derives_from: DOC-DETAIL §3.5 ／ 上流: BASIC §17
  - 入力 = `dependencies` を持たない旧形式の承認レコードのみが存在する VO → 期待 = W-STORE-002、実効承認 = draft、読取りと履歴表示のみ可
- **VO-APPROVAL-CREATE-01** — 承認作成時に対象・`judgment_ref` の参照先・依存 entity / document source を完全かつ現在の値として解決できない場合、E-APPROVAL-001 として record を生成しない。
  - derives_from: DOC-DETAIL §3.5 ／ 上流: ANNEX-A §12.2
  - 入力 = 存在しない VO ID を `--subject-id` に与えた `approval create` → 期待 = E-APPROVAL-001、終了コード 2、`.verify/approvals/` にファイルが増えない
  - 入力 = `--subject-type judgment` で存在しない判断記録 ULID を与える → 期待 = E-APPROVAL-001、record 非生成
- **VO-APPROVAL-STATUS-01** — VO の `status` は承認レコードから導出する表示値であり、正典 field ではない。
  - derives_from: DOC-DETAIL §3.2 ／ 上流: BASIC §17
  - 入力 = 実効承認 `approved` の VO に `vo show` → 期待 = `status: approved` が導出値として表示され、VO レコードファイルに `status` が存在しない
- **VO-APPROVAL-STATUS-02** — reader は保存された互換 field `status` を実効判定と VO subject hash の双方で無視し、その存在を W-STORE-001 として通知する。
  - derives_from: DOC-DETAIL §3.2 ／ 上流: BASIC §17
  - 入力 = `status: approved` を保存した VO レコードで実効承認が draft → 期待 = W-STORE-001、表示される status は draft（保存値を採用しない）
- **VO-APPROVAL-STATUS-03** — canonical writer は VO レコードへ `status` を保存しない。
  - derives_from: DOC-DETAIL §3.2 ／ 上流: BASIC §17
  - 入力 = `vo add` / `vo edit` で作成・更新した VO レコード → 期待 = 出力ファイルに `status` key が存在しない
- **VO-APPROVAL-SUBJECT-01** — 承認対象は `vo`（`subject` に VO ID）・`document`（`subject` に document ID）・`judgment`（`judgment_ref` に判断記録 ULID）の 3 種のみとし、判断記録 ULID を `subject` に置いた承認レコード、および VO / document のいずれにも解決しない `subject` を持つ承認レコードを E-APPROVAL-002 として拒否する。
  - derives_from: DOC-DETAIL §3.5 ／ 上流: BASIC §17
  - 入力 = `subject` に判断記録 ULID を書いた承認作成要求 → 期待 = E-APPROVAL-002、record 非生成
  - 入力 = `subject` に Test ID を書いた既存承認レコード → 期待 = 履歴表示のみ許可、いかなる実効承認も導出せず W-STORE-006
  - 入力 = `--subject-type document` で方針文書を対象とする承認作成 → 期待 = 受理され、document ID が `subject` に書かれる
- **VO-APPROVAL-SUBJECT-02** — 判断記録を対象とする承認は、当該判断記録が有効判断であり、かつ §8.5 の実効集合 E に属する場合にのみ実効承認を導出する。参照先の判断記録が存在しない場合は E-APPROVAL-001（書込み時）／W-STORE-006（既存読取り時）とする。
  - derives_from: DOC-DETAIL §3.5・§8.5 ／ 上流: BASIC §17
  - 入力 = 実効集合 E に属する判断記録への `approved` 承認 → 期待 = 当該判断記録の実効承認 = approved
  - 入力 = 他レコードに `supersedes` された判断記録への `approved` 承認 → 期待 = 実効承認 = draft 相当
  - 入力 = `judgment_ref` の参照先が存在しない既存承認レコード → 期待 = W-STORE-006、VO / document の実効承認も判断記録の実効承認も導出しない
- **VO-APPROVAL-STATE-01** — `approved_state` は `approved` / `rejected` / `withdrawn` の 3 値のみを受理し、値域外は書込み時 E-APPROVAL-002、既存読取り時は W-STORE-006 として実効承認を導出しない。
  - derives_from: DOC-DETAIL §3.5 ／ 上流: BASIC §17
  - 入力 = `--state pending` の `approval create` → 期待 = E-APPROVAL-002、終了コード 2、record 非生成
  - 入力 = `approved_state: pending` の既存承認レコード → 期待 = 履歴表示のみ、W-STORE-006、実効承認 = draft
- **VO-APPROVAL-STATE-02** — 実効承認は、実効集合の全レコードが `approved` のときだけ `approved` とし、実効集合に `rejected` または `withdrawn` が 1 件でも残れば `draft` とする。`approved_at` / ULID の順序・件数の多寡を採用規則に用いない。
  - derives_from: DOC-DETAIL §3.5 ／ 上流: BASIC §17
  - 入力 = `approved` と `rejected` が `supersedes` 関係なく併存 → 期待 = 実効承認 = draft（fail-closed。どちらも採らない）
  - 入力 = `withdrawn` より後の ULID を持つ `approved` が `supersedes` を持たない → 期待 = 実効承認 = draft（最新勝ちにしない）
  - 入力 = 実効集合が `approved` 2 件のみ → 期待 = 実効承認 = approved
- **VO-APPROVAL-STATE-03** — 実効集合からの除外は `supersedes` による明示の名指しだけで起き、参照先を解決できない・対象が一致しない・自己参照する `supersedes` は書込み時 E-APPROVAL-002、supersede 関係が循環する場合は W-STORE-005 として当該レコードを実効集合へ寄与させない。
  - derives_from: DOC-DETAIL §3.5 ／ 上流: BASIC §17
  - 入力 = `withdrawn` レコードの ULID を `--supersedes` に名指しした `approved` の追加 → 期待 = 実効承認 = approved
  - 入力 = 自己の ULID を `supersedes` に含む承認作成要求 → 期待 = E-APPROVAL-002、record 非生成
  - 入力 = 承認レコード 2 件が互いを `supersedes` に名指し → 期待 = W-STORE-005、双方とも実効集合へ寄与せず実効承認 = draft

---

## VO-GATE（フェーズゲート評価）

主な derives_from: DOC-DETAIL §11.5 ／ 上流: BASIC §20・REQ §26.4・ANNEX-C §18.3.9

- **VO-GATE-EVAL** — `verify --gate <name>` は、(1) 要求 scope の集約代表値が `require.verification` を満たすか、(2) `require.approvals` の各ロールについて対象の実効承認状態が `approved` であるかを評価し、満否と根拠（不足している非 `PASS` 検査・未充足の承認ロール）を提示する。ゲート全体の充足は両条件が充足した場合に限る。
  - derives_from: DOC-DETAIL §11.5 ／ 上流: ANNEX-C §18.3.9・ANNEX-A §12.3
  - 入力 = 集約代表値 = PASS、`require.approvals: [reviewer]` のロール承認が存在 → 期待 = `gate.satisfied: true`、`ok: true`、終了コード 0
  - 入力 = 集約代表値 = MISMATCH、承認は充足 → 期待 = `gate.verification.satisfied: false`、`gate.satisfied: false`、不足している非 PASS 検査が根拠として提示される
  - 入力 = 集約代表値 = PASS、`reviewer` ロールの有効承認なし → 期待 = `approvals[0].satisfied: false` と `missing_subjects` が提示され `gate.satisfied: false`
- **VO-GATE-NOTRANSITION** — ゲートの責務は条件充足の評価・提示に限り、フェーズの自動遷移・ライフサイクル管理を行わない。ゲート充足は検証状態を書き換えない。
  - derives_from: DOC-DETAIL §11.5 ／ 上流: BASIC §20・§29（OOS-004）・REQ §26.4
  - 入力 = `gate.satisfied: true` の `verify --gate release` → 期待 = config・レコードにフェーズ状態が書き込まれず、検証状態（集約ツリーと `gate.verification.actual`）が `gate.satisfied` と別 field として併記される
  - 入力 = `require.verification: UNKNOWN` のゲートが充足して `ok: true`・終了コード 0 → 期待 = 検証状態の行が省略されず、`PASS` の語がゲート満否に流用されない
- **VO-GATE-ROLE-RESOLVE** — 承認ロールは `config.yaml` の `approval_roles` でロール → approver id 集合として解決し、`gates.require.approvals` が参照するロールが `approval_roles` に無い場合を E-CONFIG-001 とする。
  - derives_from: DOC-ANNEX-A §12.3 ／ 上流: BASIC §17・§30
  - 入力 = `approval_roles.reviewer: [alice]` と `approver.id: alice` の有効承認 → 期待 = `reviewer` ロールの承認が存在すると判定される
  - 入力 = `gates[].require.approvals: [owner]` かつ `approval_roles` に `owner` が無い → 期待 = E-CONFIG-001、終了コード 2
- **VO-GATE-NAME-01** — `--gate <name>` は `gates[].name` との大文字小文字を区別した完全一致でのみ解決し、未定義名および `gates` が空の状態での指定を E-CONFIG-002（終了コード 2）とし、スキャン・検証・ゲート評価のいずれも実行せず部分結果を返さない。
  - derives_from: DOC-DETAIL §11.5・§17.1 ／ 上流: ANNEX-A §12.3
  - 入力 = `gates` に `release` のみ定義し `--gate Release` を指定 → 期待 = E-CONFIG-002、`ok: false`、終了コード 2、`data` に部分結果なし
  - 入力 = `gates` が空の状態で `--gate release` → 期待 = E-CONFIG-002、同上
  - 入力 = 未定義名の指定 → 期待 = 診断 message に指定名と定義済みゲート名の一覧が含まれ、MCP では `candidates` に定義済みゲート名が入る
- **VO-GATE-REQVER-01** — `require.verification` は 5 状態語彙のいずれかとの完全一致でなければならず、違反を config 受理時に E-CONFIG-001（終了コード 2）とする。
  - derives_from: DOC-ANNEX-A §12.3 ／ 上流: BASIC §20・DETAIL §2.2
  - 入力 = `require.verification: OK`（5 状態語彙外） → 期待 = E-CONFIG-001、終了コード 2
  - 入力 = `require.verification` を欠くゲート定義 → 期待 = E-CONFIG-001
- **VO-GATE-REQVER-02** — ゲートの検証条件は `require.verification` と要求 scope の集約代表値との完全一致でのみ充足し、5 状態に順序・優劣・包含関係を設けない。
  - derives_from: DOC-DETAIL §11.5 ／ 上流: ANNEX-A §12.3
  - 入力 = `require.verification: UNKNOWN`、集約代表値 = PASS → 期待 = 充足しない（「要求値以上」の解釈を採らない）
  - 入力 = `require.verification: PASS`、集約代表値 = PASS → 期待 = 充足する
  - 入力 = `require.verification: PASS` かつ `--items chain_integrity` の限定 scope → 期待 = scope 外検査が NO_EVIDENCE として代表値に参加するため充足しない

---

## VO-SCOPE（scope 2 軸と scope 出力）

主な derives_from: DOC-ANNEX-C §18.3.8 ／ 上流: BASIC §4.6

- **VO-SCOPE-2AXIS** — scope は検査軸（4 検査の部分集合）とエンティティ軸（DOC / VO / Test の部分木）の 2 軸で限定でき、4 検査未満を明示した実行を限定スコープとして扱い完全検証として表示しない。
  - derives_from: DOC-DETAIL §11.3 ／ 上流: BASIC §4.6・ANNEX-C §18.3.8
  - 入力 = `verify --items chain_integrity,orphan_detection` → 期待 = 2 検査のみ評価、`scope.unverified_outside_scope: true`、完全検証の表示をしない
  - 入力 = `verify --vo VO-X` → 期待 = VO-X 部分木のみ評価、`scope.requested.entities` に当該 VO が現れる
- **VO-SCOPE-NOPROMOTE** — scope 外・未実施の検査は `NO_EVIDENCE`（診断 `NOT_CHECKED`）として保持し `PASS` へ変換せず、要求 scope と scope 外が未検証である旨を併記する。
  - derives_from: DOC-DETAIL §11.3 ／ 上流: BASIC §4.6・ANNEX-C §18.3.8
  - 入力 = `verify --items chain_integrity` → 期待 = `target_binding` / `oracle_presence` が NO_EVIDENCE（診断 NOT_CHECKED）として集約ツリーに残り、PASS にならない
  - 入力 = 同実行の text 出力 → 期待 = 要求 scope と scope 外未検証の旨が冒頭に併記され、完全検証と読めない
- **VO-SCOPE-NODEGRADE** — いかなる設定値も完全検証を 4 検査未満へ縮退させず、`--items` 省略時は固定 4 検査を選択する。
  - derives_from: DOC-DETAIL §11.3・§2.2 ／ 上流: BASIC §5
  - 入力 = `verify`（`--items` 省略） → 期待 = `scope.requested.items` が 4 件すべてを固定順で列挙し、config 値から部分集合を組み立てない
- **VO-SCOPE-FULLSCOPE-INV** — 旧 12 項目を列挙した `verify.full_scope` は config version を問わず E-CONFIG-001 とし、version 1 で当該 field を欠く場合のみ固定 4 検査へ具体化する（in-memory 補完を行わない）。
  - derives_from: DOC-DETAIL §2.2 ／ 上流: BASIC §5
  - 入力 = `verify.full_scope` に旧 12 項目を列挙した config（version 2） → 期待 = E-CONFIG-001、終了コード 2
  - 入力 = version 1 で `verify.full_scope` を欠く config → 期待 = 固定 4 検査として受理される
- **VO-SCOPE-INTERNAL-DEP** — 検査の表示 scope と、検査導出に必要な内部依存の評価を分離する。
  - derives_from: DOC-DETAIL §11.3 ／ 上流: ANNEX-C §18.3.3
  - 入力 = `verify --items target_binding` → 期待 = Evidence 鮮度と `target_coverage` を内部依存として評価しつつ、`scope.requested.items` は `target_binding` 1 件のみを列挙する
- **VO-SCOPE-OUTPUT-01** — `verify` / `report` の JSON は最上位に `scope` を常に持ち、`scope.requested.items` は `--items` 省略時に固定 4 検査を固定順で 4 件列挙し、`scope.requested.entities` はエンティティ軸未指定のとき空 list とする。
  - derives_from: DOC-ANNEX-A §12.1 ／ 上流: BASIC §4.6・DETAIL §11.3
  - 入力 = `verify --format json`（引数なし） → 期待 = `scope.requested.items` が `chain_integrity` / `orphan_detection` / `target_binding` / `oracle_presence` の順で 4 件、`entities` が空 list
  - 入力 = 完全検証の `verify --format json` → 期待 = `scope` が省略されない
  - 入力 = `init` / `scan` / `run` の JSON 出力 → 期待 = `scope` を持たない
- **VO-SCOPE-OUTPUT-02** — `scope.unverified_outside_scope` は `requested.items` が 4 件未満、または `requested.entities` が空でない場合に `true`、固定 4 検査かつエンティティ軸未指定のとき `false` とする。
  - derives_from: DOC-ANNEX-A §12.1 ／ 上流: BASIC §4.6
  - 入力 = `verify --items chain_integrity --format json` → 期待 = `unverified_outside_scope: true`
  - 入力 = `verify --vo VO-X --format json`（items 省略） → 期待 = `unverified_outside_scope: true`
  - 入力 = `verify --format json`（引数なし） → 期待 = `unverified_outside_scope: false`

---

## VO-AGG（集約と機能単位の束ね）

主な derives_from: DOC-DETAIL §11.3 ／ 上流: BASIC §22

- **VO-AGG-FAILCLOSED-01** — 完全検証の総合 OK は固定 4 検査がすべて `PASS` の場合のみとし、1 項目でも非 `PASS` があれば NG とする。
  - derives_from: DOC-DETAIL §11.3 ／ 上流: BASIC §22・REQ §26.1
  - 入力 = 4 検査すべて PASS の瑕疵なしリポジトリ → 期待 = 総合 OK、終了コード 0（真陽性。すべてを保守側へ降格しない）
  - 入力 = `orphan_detection` のみ MISMATCH で他 3 検査 PASS → 期待 = 総合 NG、終了コード 1
- **VO-AGG-TREE** — Test 単位の結果を VO・DOC 単位へ fail-closed で集約し、子に 1 つでも非 `PASS` があれば親を非 `PASS` とする。
  - derives_from: DOC-DETAIL §11.3 ／ 上流: BASIC §22
  - 入力 = leaf VO を covers する 2 Test のうち 1 件が FAIL → 期待 = 当該 leaf VO が非 PASS、その上位 DOC も非 PASS
- **VO-AGG-PRIORITY** — 代表値の優先順位を `FAIL` > `MISMATCH` > `NO_EVIDENCE` > `UNKNOWN` とし、診断ラベルを順位に用いず併記する。
  - derives_from: DOC-DETAIL §11.3 ／ 上流: BASIC §22.2
  - 入力 = 子の値が FAIL と MISMATCH と UNKNOWN → 期待 = 親の代表値 = FAIL
  - 入力 = 子の値が NO_EVIDENCE（診断 STALE）と UNKNOWN → 期待 = 親の代表値 = NO_EVIDENCE、診断 STALE は併記され順位に影響しない
- **VO-AGG-UNREG-NG** — 他の検査がすべて `PASS` でも、管理宣言を持たない未登録 Test が 1 件あれば `chain_integrity` により総合 NG とする。この事実は W-SCAN-101 の警告重大度と同時に成立する。
  - derives_from: DOC-ANNEX-C §18.3.8 ／ 上流: DETAIL §5.4・§11.1.1
  - 入力 = 未登録 Test 1 件、他は全 PASS → 期待 = W-SCAN-101（warning）と `chain_integrity` = MISMATCH（診断 MISSING）が同時に成立し、総合 NG・終了コード 1
- **VO-AGG-DRILLDOWN** — NG のとき、どのエンティティ・どの検査・どの状態・どの診断ラベルで落ちたかを掘り下げられ、非 `PASS` の根拠（判断記録・Evidence への参照）を辿れる。covers を持つ Test は covers 先 VO の子として表示し、いずれの VO へも寄与しない Test の事実も出力から確認できる。
  - derives_from: DOC-DETAIL §11.3 ／ 上流: BASIC §22.3・ANNEX-C §18.3.8
  - 入力 = 総合 NG の `verify --format json` → 期待 = 非 PASS ノードごとに entity ID・検査名・`state`・`diagnostic` と根拠 Evidence ID が辿れる
  - 入力 = 同実行の text 出力 → 期待 = 同じ内容が人間向けに提示される
  - 入力 = covers を持つ Test と、どの VO へも寄与しない Test が併存 → 期待 = 前者は covers 先 VO の子として、後者は非寄与の事実として、いずれも出力に現れる
- **VO-AGG-PARENT-01** — 機能単位の集約は親 VO を単位として実現し、Feature を独立のエンティティ種別・レコードファイル・ID 体系・宣言 field として設けず `.verify/` に Feature 用ディレクトリを置かない。
  - derives_from: DOC-DETAIL §11.3 ／ 上流: BASIC §22.2・§3.1
  - 入力 = 子 VO を 3 件持つ親 VO-X → 期待 = 機能単位の束ねが VO-X の ID で識別され、`.verify/` に feature ディレクトリ・Feature ID が現れない
  - 入力 = 親 VO を持たない leaf VO → 期待 = それ自体が最上位の束ね単位となる
- **VO-AGG-PARENT-02** — Test の結果が親 VO へ寄与する経路は、covers する leaf VO 経由の伝播と、当該親 VO を直接 covers する Test の直接参加の 2 つに限り、ファイルパス・モジュール名・命名規約からの推定束ねを設けない。
  - derives_from: DOC-DETAIL §11.3 ／ 上流: BASIC §22.2
  - 入力 = 親 VO-X の子 VO を covers する Test → 期待 = 子 VO 経由で VO-X の合成に参加する
  - 入力 = 親 VO-X を直接 covers する Test → 期待 = VO-X の合成へ直接参加する（W-SCAN-103 の警告は出るが寄与は成立する）
  - 入力 = VO-X と同名のモジュールに置かれ covers を宣言しない Test → 期待 = VO-X の合成に参加しない

---

## VO-ADAPTER（adapter 境界・registry・wire 互換・Source Target identity）

主な derives_from: DOC-ANNEX-C §18.3.12 ／ 上流: BASIC §27

- **VO-ADAPTER-NEUTRAL** — 検証契約・ID・ハッシュ・Evidence・状態・集約は language / runner 非依存であり、Test Entity は言語の関数ではなく `ExecutionDescriptor` だけを実行座標として持つ。
  - derives_from: DOC-DETAIL §5.2 ／ 上流: BASIC §27・§2.4
  - 入力 = 非 Rust adapter の Test を含む scan → 期待 = 5 状態・4 検査・集約規則が同一に適用され、Test JSON が `execution` を持ち Rust 固有 field を持たない
- **VO-ADAPTER-NOPROMOTE** — adapter の未登録・能力不足・解析不能を `PASS` へ昇格させず、static / coverage capability の欠如は `NO_EVIDENCE`（診断 `NOT_CHECKED`）、adapter が報告した解析限界は `UNKNOWN` として区別する。
  - derives_from: DOC-ANNEX-A §12.1 ／ 上流: BASIC §22.3・§27・ANNEX-C §18.3.3
  - 入力 = static analysis capability を持たない adapter の Test → 期待 = `oracle_presence` = NO_EVIDENCE（診断 NOT_CHECKED）、W-ADAPTER-101
  - 入力 = adapter が解析限界を報告（W-ADAPTER-102） → 期待 = 当該検査 = UNKNOWN（NOT_CHECKED とは別値）
  - 入力 = runner capability を持たない adapter の Test → 期待 = NO_EVIDENCE（診断 NOT_EXECUTED）
- **VO-ADAPTER-TARGETREQ-01** — core は `targets` 1 件以上を adapter 中立の必須リンクとせず、型として空の target list を許容する。
  - derives_from: DOC-DETAIL §4.1 ／ 上流: BASIC §9.1・ANNEX-C §18.3.1
  - 入力 = `targets` を空 list とする非 rust-cargo adapter の Test（core 中立必須 metadata は充足） → 期待 = core の `chain_integrity` = PASS（targets 0 件を core が MISMATCH にしない）
  - 入力 = 同 Test の Test JSON → 期待 = `targets` が空 list として返る（単数互換 field を持たない）
- **VO-ADAPTER-TARGETREQ-02** — `rust-cargo` adapter は `targets` 1 件以上を当該 adapter の必須 metadata として要求し、欠落を E-SCAN-007（`chain_integrity` = `MISMATCH`、診断 `MISSING`）として報告する。
  - derives_from: DOC-DETAIL §4.4・§5.5 ／ 上流: BASIC §9.2・ANNEX-C §18.3.12
  - 入力 = `@vtest.target` を持たない rust-cargo の管理対象 Test → 期待 = E-SCAN-007、`chain_integrity` = MISMATCH（診断 MISSING）
- **VO-ADAPTER-REGISTER-01** — core を変更せずに別の adapter を registry へ登録でき、登録後にその adapter の Test が発見・検証される。
  - derives_from: DOC-ANNEX-C §18.3.12 ／ 上流: BASIC §27
  - 入力 = 一意な adapter ID と宣言 capability を持つ adapter を registry へ登録 → 期待 = 登録が成功し、当該 adapter の Test が scan 結果に現れる
- **VO-ADAPTER-REGISTER-02** — registry に同一 adapter ID が重複して登録される状態を E-ADAPTER-001 として拒否する。
  - derives_from: DOC-DETAIL §17.1 ／ 上流: BASIC §27
  - 入力 = 既登録の adapter ID と同じ ID を持つ adapter の登録（他条件は充足） → 期待 = E-ADAPTER-001、終了コード 2
- **VO-ADAPTER-REGISTER-03** — 未登録の adapter を指定する操作を E-ADAPTER-001 として拒否する。
  - derives_from: DOC-DETAIL §17.1 ／ 上流: BASIC §27
  - 入力 = config に未登録の adapter ID を指定した実行 → 期待 = E-ADAPTER-001、終了コード 2、結果を生成しない
- **VO-ADAPTER-REGISTER-04** — registry の宣言 capability と実装が一致しない adapter を E-ADAPTER-001 として拒否する。
  - derives_from: DOC-DETAIL §17.1 ／ 上流: BASIC §27
  - 入力 = coverage capability を宣言するが実装を持たない adapter の登録 → 期待 = E-ADAPTER-001、終了コード 2
- **VO-ADAPTER-REGISTER-05** — 異なる adapter が発見した Test の ID が重複する状態を E-SCAN-002 とし `chain_integrity` = `MISMATCH` とする。
  - derives_from: DOC-DETAIL §5.4 ／ 上流: BASIC §27・ANNEX-C §18.3.12
  - 入力 = adapter A と adapter B が同一 Test ID を報告 → 期待 = E-SCAN-002、`chain_integrity` = MISMATCH
- **VO-ADAPTER-MERGE-01** — 複数 adapter の discovery 結果の統合は決定論的であり、adapter の実行順序を変えても同一の統合結果を返す。
  - derives_from: DOC-ANNEX-C §18.3.12 ／ 上流: BASIC §27
  - 入力 = adapter A → B の順と B → A の順で同一リポジトリを scan → 期待 = エンティティ集合・内容ハッシュ・診断の順序が完全に一致する
- **VO-ADAPTER-MERGE-02** — 異なる adapter が同一の root を共有する構成を受理する。
  - derives_from: DOC-ANNEX-C §18.3.12 ／ 上流: BASIC §27
  - 入力 = adapter A と adapter B が同一ディレクトリを root として宣言 → 期待 = 受理され、双方の Test が統合結果に現れる
- **VO-ADAPTER-MERGE-03** — 同一 adapter 内で root が重複する構成を拒否する。
  - derives_from: DOC-ANNEX-C §18.3.12 ／ 上流: BASIC §27
  - 入力 = 単一 adapter の config が同一 root を 2 回列挙 → 期待 = E-CONFIG-001、終了コード 2
- **VO-ADAPTER-MERGE-04** — 全 adapter の統合後の集合に対して Test ID の大局的一意性を検査する。
  - derives_from: DOC-ANNEX-C §18.3.12 ／ 上流: DETAIL §4.4・§5.4
  - 入力 = 各 adapter 内では一意だが統合後に重複する Test ID → 期待 = E-SCAN-002、`chain_integrity` = MISMATCH（adapter 単位の検査で見逃さない）
- **VO-ADAPTER-WIRE-01** — reader は config version 1 の入力を受理し、読み取りだけでファイルを書き換えない。
  - derives_from: DOC-DETAIL §2.2 ／ 上流: BASIC §2.4
  - 入力 = version 1 の `config.yaml` に対する `scan` → 期待 = 受理され、実行後の `config.yaml` のバイト列が不変
- **VO-ADAPTER-WIRE-02** — writer および `init` は config version 2 を出力する。
  - derives_from: DOC-ANNEX-A §12.2 ／ 上流: BASIC §2.4・DETAIL §2.2
  - 入力 = `vtest init` → 期待 = 生成された `config.yaml` の version が 2 であり、組込 `rust-cargo` adapter namespace を含む
- **VO-ADAPTER-WIRE-03** — Test を含む JSON は本冊 §5.2 の `execution`（実行座標）を常に返す。
  - derives_from: DOC-ANNEX-A §12.1 ／ 上流: BASIC §2.4
  - 入力 = 任意の adapter の Test に `test show --format json` → 期待 = `execution` が常に存在する
- **VO-ADAPTER-WIRE-04** — version 1 互換 field は `rust-cargo` の Test にだけ追加し、非 Rust Test では省略して空値・dummy 値・Rust 既定値を生成しない。
  - derives_from: DOC-ANNEX-A §12.1 ／ 上流: BASIC §2.4
  - 入力 = `rust-cargo` Test の JSON → 期待 = `filter` / `package` / `test_target` が `execution` と整合する値で追加される
  - 入力 = 非 Rust adapter の Test の JSON → 期待 = `filter` / `package` / `test_target` の key 自体が存在しない（空文字列・null・既定値で埋めない）
- **VO-ADAPTER-WIRE-05** — Test JSON は `targets` を常に list として返し、単数互換 field `target` は target がちょうど 1 件のときにだけ追加する。
  - derives_from: DOC-ANNEX-A §12.1 ／ 上流: BASIC §2.4
  - 入力 = target 1 件の Test → 期待 = `targets` が 1 要素 list、`target` が同値で追加される
  - 入力 = target 3 件の Test → 期待 = `targets` が 3 要素 list、`target` field が存在しない（先頭を代表値として返さない）
- **VO-ADAPTER-SRC-IDENT-01** — 複数の Source Target を独立に保持し、代表 1 件へ縮約しない。
  - derives_from: DOC-DETAIL §5.2 ／ 上流: BASIC §9.2・ANNEX-C §18.3.1
  - 入力 = target を 3 件宣言した Test → 期待 = Source Target 3 件が独立に保持され、Evidence の `hashes.targets` も 3 件になる
- **VO-ADAPTER-SRC-IDENT-02** — Test と Source Target を双方向に辿れる。
  - derives_from: DOC-DETAIL §5.3 ／ 上流: BASIC §3.3・§9.2
  - 入力 = `test query --source src/parser.rs::Parser::parse` → 期待 = 当該 Source Target を宣言する Test の一覧が返る
  - 入力 = `test show TEST-A` → 期待 = 当該 Test が宣言する Source Target の一覧が返る
- **VO-ADAPTER-SRC-DUAL** — 恒久 SRC ID を持つ Source Target は locator でも SRC ID でも addressable であり、両モードで同一の hash・同一の identity へ解決する。内容ハッシュは canonical locator と construct bytes だけから計算し、恒久 SRC ID を独立したハッシュ入力 field として加えない。
  - derives_from: DOC-ANNEX-C §18.3.1 ／ 上流: DETAIL §1.3・§6.1.1
  - 入力 = 同一関数を locator で参照する Test と SRC ID で参照する Test → 期待 = 双方が同一 canonical Source Target・同一 content hash へ解決する
  - 入力 = construct 内側の `@vtest.src-id` 宣言行を変更 → 期待 = construct bytes の変化を経由して hash が変わる（SRC ID を独立のハッシュ入力にしない）
- **VO-ADAPTER-SRC-CANON-01** — Source Target の identity は宣言された `TargetRef` → 解決 → canonical Locator の一方向で確定し、逆方向の書き戻しを行わない。
  - derives_from: DOC-DETAIL §6.1.1 ／ 上流: ANNEX-C §18.3.1
  - 入力 = SRC ID 参照を持つ Test の scan → 期待 = 解決結果が canonical Locator となり、宣言側の綴りが canonical へ書き換えられない
- **VO-ADAPTER-SRC-CANON-02** — Evidence と判断記録は解決後の canonical Locator を記録し、参照側が宣言した `TargetRef` の綴りを identity として記録しない。
  - derives_from: DOC-DETAIL §3.6・§6.1.1 ／ 上流: ANNEX-C §18.3.1
  - 入力 = SRC ID 参照で target を宣言した Test の `run` → 期待 = Evidence の `hashes.targets[].target` が canonical Locator 文字列であり `SRC-` で始まらない
- **VO-ADAPTER-SRC-CANON-03** — 綴りの異なる複数の target 宣言が同一の canonical Source Target へ解決する状態を E-SCAN-005 として検出する。
  - derives_from: DOC-DETAIL §4.2・§5.4 ／ 上流: ANNEX-C §18.3.1
  - 入力 = 同一関数を locator と SRC ID の両方で宣言した Test → 期待 = E-SCAN-005、`chain_integrity` = MISMATCH
- **VO-ADAPTER-SRC-CANON-04** — canonical な解決対象として恒久 SRC ID を返した adapter 出力を不正な adapter 出力として拒否する。
  - derives_from: DOC-ANNEX-C §18.3.1 ／ 上流: DETAIL §6.1.1
  - 入力 = 解決結果に `SRC-` で始まる値を canonical target として返す adapter → 期待 = 拒否され、当該解決結果を Evidence・検証へ採用しない
- **VO-ADAPTER-SRC-RESOLVE-01** — target 解決の結果を「解決済み」「対象なし」「曖昧」の 3 分類として区別する。
  - derives_from: DOC-DETAIL §6.1・§6.3 ／ 上流: ANNEX-C §18.3.4
  - 入力 = 実在する一意な locator → 期待 = 解決済み
  - 入力 = 実在しない locator → 期待 = 対象なし（E-SCAN-004、診断 MISSING）
  - 入力 = 複数候補へ解決しうる SRC ID 参照 → 期待 = 曖昧（対象なしとは別分類）
- **VO-ADAPTER-SRC-RESOLVE-02** — 曖昧な解決は fail-closed に終端し、候補を解決結果として記録せず診断表示のみを行う。
  - derives_from: DOC-DETAIL §5.4・§6.1 ／ 上流: ANNEX-C §18.3.4
  - 入力 = 衝突する SRC ID 参照（E-SCAN-011） → 期待 = 当該 target 解決を MISMATCH とし、候補の 1 件を Evidence・検証へ永続化せず、候補一覧は診断としてのみ表示される
- **VO-ADAPTER-SRC-UNIQ-01** — 恒久 SRC ID は全 adapter 統合後のリポジトリ全体で一意であり、衝突を E-SCAN-011 として拒否する。
  - derives_from: DOC-DETAIL §5.4 ／ 上流: ANNEX-C §18.3.1
  - 入力 = 異なる 2 つの関数が同一の `@vtest.src-id` を宣言 → 期待 = E-SCAN-011
  - 入力 = 異なる adapter の 2 つの Source Target が同一 SRC ID を宣言 → 期待 = E-SCAN-011
- **VO-ADAPTER-SRC-UNIQ-02** — SRC ID が衝突しても、衝突した各 Source Target は canonical Locator で独立に具体化されたまま保持される。
  - derives_from: DOC-DETAIL §5.4 ／ 上流: ANNEX-C §18.3.1
  - 入力 = SRC ID が衝突する 2 つの Source Target → 期待 = E-SCAN-011 と同時に、両 Source Target が各々の canonical Locator で entity 一覧に存在し、locator 参照からは一意に解決できる

---

## VO-STORE（保存規約）

- **VO-STORE-ATOMIC-01** — `.verify/` 配下のレコード・エンティティファイルの書き込みは完全な内容が単一の操作で可視になる方式（同一ファイルシステム内の temp 書込み＋rename 等）で公開し、書きかけ状態・一時ファイル残渣を正典ディレクトリの読み手に観測させない。
  - derives_from: DOC-DETAIL §16.1 ／ 上流: BASIC §24.2
  - 入力 = レコード書込みの各中間時点で正典ディレクトリを列挙 → 期待 = 対象ファイルは書込み前の完全な内容か書込み後の完全な内容のいずれかとしてのみ観測され、部分内容のファイル・一時ファイルが列挙されない
- **VO-STORE-RELIMMUT-01** — 外部 Relation レコードは不変であり、同一 ULID のレコードの内容ハッシュが変化した状態を `MISMATCH` とする。
  - derives_from: DOC-DETAIL §3.3・§16.1 ／ 上流: BASIC §24.2
  - 入力 = 既存 `REL-<ULID>.yaml` の内容を編集 → 期待 = 同一 ULID の内容ハッシュ変化として `MISMATCH`。変更は旧レコード削除＋新レコード追加で表現する
- **VO-STORE-APPEND-01** — 判断記録・承認記録・Evidence は ULID をファイル名とする新規ファイル追加のみで作成し、既存ファイルの編集を伴わない。同一 ULID のレコードの内容ハッシュが変化した状態を `MISMATCH` とする。
  - derives_from: DOC-DETAIL §16.1 ／ 上流: BASIC §24.2
  - 入力 = `audit submit` / `approval create` / `run` の実行前後の `.verify/decisions/` `.verify/approvals/` `.verify/evidence/` → 期待 = 新規 ULID ファイルが 1 件増え、既存ファイルのバイト列が 1 件も変化しない
  - 入力 = 既存の判断記録ファイルの内容を編集 → 期待 = 同一 ULID の内容ハッシュ変化として `MISMATCH`
- **VO-STORE-GITIGNORE-01** — `vtest init` は `.verify/.gitignore` を生成して `cache/` を Git 管理対象外とし、`cache/` 以外の正典・事実レコードを Git 管理対象とする。
  - derives_from: DOC-DETAIL §2.1 ／ 上流: BASIC §24.1
  - 入力 = `vtest init` 後の `.verify/` → 期待 = `.verify/.gitignore` が存在し `cache/` を除外する。`doc/` `vo/` `rel/` `forms/` `decisions/` `approvals/` `evidence/` は Git 管理対象として残る

---

## VO-EXIT（UNKNOWN 検疫と終了コード）

- **VO-EXIT-QUARANTINE** — `UNKNOWN` は正常動作としての降参であり、内部エラー・入力不正の fallback 先として用いず、それらは終了コードで別系統として扱う。
  - derives_from: DOC-DETAIL §17.2 ／ 上流: REQ §5.4・BASIC §4.4
  - 入力 = config invariant 違反（E-CONFIG-001） → 期待 = 終了コード 2、検証結果を生成しない（検査値を UNKNOWN にして 0 / 1 を返さない）
  - 入力 = ツール自体の異常 → 期待 = 終了コード 3
- **VO-EXIT-CODES** — 終了コードは 0（要求 scope の検証結果 OK）／1（検証 NG）／2（操作拒否）／3（内部エラー）とし、同一実行に複数候補があるときは 3 → 2 → 1 → 0 の順で優先する。
  - derives_from: DOC-DETAIL §17.2 ／ 上流: BASIC §26.1・ANNEX-A §12.1
  - 入力 = E-ADAPTER-* または E-CONFIG-* で拒否された `scan` → 期待 = 終了コード 2、scan 結果を生成しない
  - 入力 = scan が完了し E-SCAN-* を報告 → 期待 = 終了コード 1
  - 入力 = error 診断なしの `scan` → 期待 = 終了コード 0
  - 入力 = `--gate` 充足の `verify` → 期待 = 終了コード 0（総合が NG でも `require.verification` と一致していれば 0 を妨げない）

---

## VO-ONBOARD（途中導入）

主な derives_from: DOC-BASIC §18

- **VO-ONBOARD-FAILCLOSED-01** — 未登録テスト・欠落宣言・未確定の義務・未実施の検査のいずれも検証済みとして扱わない。
  - derives_from: DOC-BASIC §18.3 ／ 上流: REQ §17・DETAIL §11.7
  - 入力 = 未登録 Test を含むリポジトリ → 期待 = 当該事実が出力に現れ、総合が OK にならない
  - 入力 = 管理宣言が欠落した Test → 期待 = `chain_integrity` が非 PASS
  - 入力 = covers する Test の無い leaf VO → 期待 = `chain_integrity` が非 PASS
  - 入力 = `--items` で省略した検査 → 期待 = NO_EVIDENCE（診断 NOT_CHECKED）として保持され PASS にならない
  - 入力 = どの VO へも寄与しない Test → 期待 = 非寄与の事実が出力から確認でき、検証済みとして扱われない
- **VO-ONBOARD-CAPACITY-01** — 既に大量のコードと Test を持つプロジェクトを検証対象として登録・スキャンできる。
  - derives_from: DOC-BASIC §18.1 ／ 上流: REQ §17
  - 入力 = 既存 Test を多数含むリポジトリに `vtest init` と `scan` → 期待 = スキャンが完了し、全 Test construct が Discovered Test として列挙される
- **VO-ONBOARD-INIT** — `vtest init` は `.verify/` 一式（`doc/` `vo/` `rel/` `forms/` `decisions/` `approvals/` `evidence/` `cache/`、`config.yaml` 雛形、`.verify/.gitignore`、組込 Form Schema）を生成する。
  - derives_from: DOC-ANNEX-A §12.2 ／ 上流: DETAIL §2.1・BASIC §24.1
  - 入力 = `.verify/` の無いプロジェクトで `vtest init` → 期待 = 上記の生成物がすべて存在する
  - 入力 = 既存 `.verify/` があるプロジェクトで `vtest init` → 期待 = 終了コード 2 で中止する
- **VO-ONBOARD-NOMOD-01** — `vtest init` は `.verify/` とその配下だけを作成し、`.verify/` の外にあるいかなるファイルも新規作成・変更・削除せず、既存ソース・既存テストコードのバイト列を変更しない。中止した実行はファイルを 1 件も作成・変更・削除しない。
  - derives_from: DOC-ANNEX-A §12.2 ／ 上流: BASIC §18.1・REQ R-5
  - 入力 = 既存ソースを含むプロジェクトで `vtest init` → 期待 = `.verify/` を除いた作業ツリーの内容が実行前と同一（ルート直下の `.gitignore`・`Cargo.toml`・CI 設定を含む）
  - 入力 = 既存ソースに `@vtest.` 行が無い状態で `vtest init` → 期待 = 既存ソースへ管理宣言が挿入されない
  - 入力 = 既存 `.verify/` があるプロジェクトで `vtest init` → 期待 = 終了コード 2 で中止し、その実行でファイル・ディレクトリを 1 件も作成・変更・削除しない
- **VO-ONBOARD-PARTIAL-01** — 判断記録・Evidence が一部欠落した既存プロジェクトの状態を、ハードエラーにせず読み取り、対応する非 `PASS` 状態として提示する。
  - derives_from: DOC-BASIC §18.1 ／ 上流: DETAIL §11.2・§8.5
  - 入力 = Evidence が 0 件の Test を含むリポジトリに `verify` → 期待 = 例外・中断なく完走し、`target_binding` = NO_EVIDENCE（診断 NOT_EXECUTED）として提示される
  - 入力 = 判断記録を持たない `UNKNOWN` の Test → 期待 = 完走し、判断待ち情報として提示される
- **VO-ONBOARD-PENDING** — 判断待ち情報を機械可読な構造（`subject` / `kind` / `check` / `judgment_kind` / `basis` / `bundle_ref`）として `verify` / `report` の JSON へ横断的に集約する。`check: null` の項目は集約へ寄与せずいかなる検査の値も変更しない。
  - derives_from: DOC-DETAIL §11.7 ／ 上流: ANNEX-A §12.4・BASIC §18.3
  - 入力 = UNKNOWN の検査と判断競合が併存するリポジトリの `verify --format json` → 期待 = `pending` section に両方が項目として現れ、6 項目の構造を持つ
  - 入力 = `check: null` の項目が 1 件ある状態 → 期待 = いずれの検査値も当該項目の有無で変わらない

---

## VO-TRACE（トレーサビリティと projection）

主な derives_from: DOC-DETAIL §11.6

- **VO-TRACE-ANYNODE** — 最小単位「上流ノード → 関係 → 下流ノード」を任意のノード（DOC / VO / TEST / SRC）から取得でき、上流方向・下流方向へ連続して辿れ、プロジェクト全体構造も取得できる。常に全チェーンを表示することは求めない。
  - derives_from: DOC-DETAIL §11.6 ／ 上流: REQ §3.4・NFR-003・BASIC §19
  - 入力 = Test ノードを起点とした上流トレース → 期待 = covers 先 VO とその derives_from 先 document が連続して辿れる
  - 入力 = DOC ノードを起点とした下降トレース → 期待 = 下流 VO と Test が連続して辿れる
  - 入力 = VO ノード / SRC ノードを起点としたトレース → 期待 = いずれも起点として受理される
  - 入力 = 起点を指定しない全体構造の取得 → 期待 = プロジェクト全体のトレーサビリティ構造が返る
- **VO-TRACE-INDEX** — 逆引きインデックス（VO → Tests / SRC → Tests / DOC → VOs / DOC → DOCs）を正典レコードから再構築する派生情報とし、Evidence に含まれる target 参照から TEST → SRC の関係を生成・修復しない。
  - derives_from: DOC-DETAIL §5.3・§3.6 ／ 上流: REQ P-003・BASIC §2.3・NFR-004
  - 入力 = インデックスを削除した状態での `verify` → 期待 = 正典レコードから再構築され結果が変わらない
  - 入力 = Test の `@vtest.target` 宣言を削除し、当該 target を含む既存 Evidence が残っている状態 → 期待 = TEST → SRC の関係が Evidence から生成・修復されず、当該 Test の targets が 0 件として扱われる
- **VO-TRACE-PROJECTION-01** — 役割の集合は固定 enum に縛られず可変であり、本冊が役割名を列挙・固定しない。
  - derives_from: DOC-DETAIL §11.6 ／ 上流: BASIC §19・§30
  - 入力 = 未知の役割名を `report --view` に指定 → 期待 = 役割名が固定 enum として拒否されず、preset 定義の有無だけで解決される
- **VO-TRACE-PROJECTION-02** — projection は同一のトレーサビリティ構造から、preset の定義に従って参照対象・関係・集約粒度を変えた出力を返す。親 VO を起点とする下降 projection は当該親 VO の代表値と配下の子 VO・Test ごとの内訳を同じ出力から辿れるようにし、Feature 名・Feature ID の別 field を設けない。
  - derives_from: DOC-DETAIL §11.6 ／ 上流: BASIC §19・DETAIL §11.3
  - 入力 = 親 VO-X を起点とする下降 projection → 期待 = VO-X の代表値と、子 VO ごと・Test ごとの内訳が同一出力から辿れ、束ねの識別子が VO-X の ID である
  - 入力 = 同一リポジトリに対する 2 つの異なる preset → 期待 = 同一の正典構造から粒度の異なる出力が返り、正典レコードが変化しない
  - 注: 各役割にどの参照対象・どの粒度を対応させるかは TRANSFER-3 として §11 判断記録へ移送済み
- **VO-TRACE-ANCHOR-01** — projection が出力する `derives_from` エッジ（DOC → DOC、DOC → VO）に当該 entry の `anchor` を常に同伴させ、`anchor` を持たない entry では当該 field を省略または `null` とし空文字列で埋めない。これにより上流条項と VO の対応ペアが構造化出力として取得できる。
  - derives_from: DOC-DETAIL §11.6 ／ 上流: BASIC §11.1・DETAIL §3.2
  - 入力 = `anchor: "§8.2条項2"` を持つ `derives_from` entry の projection → 期待 = 出力エッジに同一文字列の `anchor` が同伴する
  - 入力 = `anchor` を持たない `derives_from` entry の projection → 期待 = 当該 field が省略または `null` であり、空文字列で埋められない
  - 入力 = 同一 doc を異なる `anchor` で 2 entry 持つ VO → 期待 = 2 本のエッジとして出力され、重複として畳まれない

---

## VO-STO（Structured Test Operation）

主な derives_from: DOC-ANNEX-A §15

- **VO-STO-DESIRED** — Create / Edit は desired state 方式とし、adapter が差分を計算し core が再スキャンで検証する。同一の desired state の再適用は冪等である。
  - derives_from: DOC-ANNEX-A §15.2・§15.3 ／ 上流: BASIC §15.1・ANNEX-C §18.3.10
  - 入力 = 同一 desired state での `test edit` を 2 回連続実行 → 期待 = 2 回目で差分が生じない
  - 入力 = `test create` 直後に同じ desired state で `test edit` → 期待 = 差分が生じない
- **VO-STO-1TEST-01** — 1 回の Edit の対象は厳密に 1 つの Test とし、拡張 range の単一置換で他の Test・helper・fixture・通常ソースを変更しない。範囲計算（適用前）と他 Test のハッシュ不変確認（適用後）の二重で検査する。
  - derives_from: DOC-ANNEX-A §15.4 ／ 上流: REQ §16・BASIC §15.3
  - 入力 = 同一ファイルに 3 つの Test があり 1 つを `test edit` → 期待 = 対象 Test 以外の 2 件のソーステキストが不変、helper・通常コードも不変
  - 入力 = 置換範囲が他 Test を含むと適用前に判明する編集 → 期待 = E-OP-003 で中止し、ファイルを変更しない
  - 注: 1 回の編集が複数 Test にまたがってよい例外条件は TRANSFER-4 として §11 判断記録へ移送済み
- **VO-STO-INPUT-VALIDATE** — 構造化入力を受理時に検証し、symbol・Test ID・参照 VO が存在しない場合は候補を提示して E-OP-001 とする。Form の必須値と未知 field を常に検証する。
  - derives_from: DOC-ANNEX-A §15.2・§14.2 ／ 上流: DETAIL §6.3・§17.1
  - 入力 = 存在しない VO ID を covers に指定した `test create` → 期待 = E-OP-001（候補付き）、終了コード 2、ファイル非改変
  - 入力 = Form の必須回答を欠く `test create` → 期待 = E-OP-001、ファイル非改変
  - 入力 = Form に宣言されていない field を含む回答 → 期待 = E-OP-001、ファイル非改変
- **VO-STO-FORM-RESOLVE** — Form の `kind` はリポジトリ全体で一意であり、owner adapter が field を宣言する。1 件に解決できる場合にだけ `test create` / `form_get` を許可し、重複・未知・曖昧・capability 不足は拒否してファイルを変更しない。
  - derives_from: DOC-ANNEX-A §14.1・§14.3 ／ 上流: BASIC §15.4
  - 入力 = 同一 `kind` を 2 つの adapter が宣言 → 期待 = 拒否、ファイル非改変
  - 入力 = 未知の `kind` を指定 → 期待 = 拒否、ファイル非改変
  - 入力 = 一意に解決できる `kind` → 期待 = owner adapter を明示した Form Schema が返る
- **VO-STO-HELPER-OOS** — helper・fixture・通常ソースコードの編集手段を提供しない。
  - derives_from: DOC-ANNEX-A §15.4 ／ 上流: REQ OOS-003・BASIC §15.3
  - 入力 = helper 関数を対象とする編集要求 → 期待 = 当該操作の入口が存在せず、Test 以外のソースが Structured Operation で変更されない
- **VO-STO-ROLLBACK-01** — Create の適用後に再パース検証を行い、失敗した場合は適用前の状態へ復元して E-OP-003 とする。部分適用された挿入内容を残さず、Test ID の採番・Evidence・判断記録を含む副産物を 1 つも残さない。
  - derives_from: DOC-ANNEX-A §15.2 ／ 上流: BASIC §15.1・DETAIL §17.1
  - 入力 = 挿入後に構文的に妥当でなくなる `test create` → 期待 = E-OP-003、適用前のソーステキストが復元される
  - 入力 = 挿入によりファイルが新規作成されたうえで検証に失敗 → 期待 = 当該ファイルが不存在へ戻る
  - 入力 = E-OP-003 で中止した `test create` の後に `scan` → 期待 = 操作が無かった場合と同一のエンティティ集合・内容ハッシュが得られる
- **VO-STO-PARITY-01** — Structured Test Operation は CLI と MCP のどちらの入口からでも同一の Form Schema と desired state 入力を消化し、同等の結果（生成 Test・診断・拒否）を返す。
  - derives_from: DOC-ANNEX-A §13.1・§13.3 ／ 上流: BASIC §15.4
  - 入力 = 同一の Form 回答を `test create` と MCP `test_create` に与える → 期待 = 生成される Test ID・挿入位置・annotation block が一致する
  - 入力 = 同一の不正回答を双方に与える → 期待 = 同一の診断コード（E-OP-001）と候補が返り、いずれもファイルを変更しない

---

## VO-IFACE（MCP = CLI 同一性・非対話性）

主な derives_from: DOC-ANNEX-C §18.3.11 ／ 上流: BASIC §26.2

- **VO-IFACE-PARITY-01** — 同一の入力に対して MCP ツールと CLI が同じ `data` と `diagnostics` を返す。
  - derives_from: DOC-ANNEX-C §18.3.11 ／ 上流: BASIC §26.2・ANNEX-A §13.1
  - 入力 = 同一リポジトリに対する `verify --format json` と MCP `verify` → 期待 = `data` と `diagnostics` が一致する
- **VO-IFACE-PARITY-02** — CLI と MCP は同一の registry composition を用いる。
  - derives_from: DOC-ANNEX-A §12.1 ／ 上流: BASIC §26.2
  - 入力 = adapter を 1 件追加登録した状態での CLI `scan` と MCP `scan` → 期待 = 双方が同一の adapter 集合を用い、発見される Test 集合が一致する
- **VO-IFACE-PARITY-03** — CLI と MCP は同一の adapter 選択規則を用い、adapter 選択が拒否される入力に対して同一のエラーと診断を返す。MCP 側が異なる adapter を暗黙に選択しない。
  - derives_from: DOC-ANNEX-A §12.1・§13.1 ／ 上流: BASIC §26.2
  - 入力 = 未登録 adapter を指定した CLI 実行と MCP 呼出 → 期待 = 双方が E-ADAPTER-001 を返す（MCP が別 adapter へフォールバックしない）
  - 入力 = capability 不足の adapter を明示指定した CLI 実行と MCP 呼出 → 期待 = 双方が E-ADAPTER-004、ファイル・Evidence・判断記録を生成しない
- **VO-IFACE-PARITY-04** — CLI と MCP は同一の JSON envelope とエラー体系を共有し、CLI だけが Rust 固有の既定値へフォールバックしない。
  - derives_from: DOC-ANNEX-A §12.1 ／ 上流: BASIC §26.2
  - 入力 = 同一のエラー入力に対する CLI と MCP → 期待 = 同一の `code` を持つ診断が返り、`{ok, data, diagnostics}` の構造が一致する
  - 入力 = 非 Rust Test を含む CLI 出力 → 期待 = Rust 固有 field の既定値が補われない
- **VO-IFACE-RPC-01** — JSON-RPC の request 入力を規約どおりに処理し、`id` に対応する応答を返す。
  - derives_from: DOC-ANNEX-A §13.1 ／ 上流: BASIC §26.2
  - 入力 = `id` を持つ正当な request → 期待 = 同一 `id` を持つ応答が 1 件返る
- **VO-IFACE-RPC-02** — JSON-RPC の notification 入力を規約どおりに処理し、応答を返さない。
  - derives_from: DOC-ANNEX-A §13.1 ／ 上流: BASIC §26.2
  - 入力 = `id` を持たない notification → 期待 = 応答が返らず、処理は実行される
- **VO-IFACE-RPC-03** — JSON-RPC の batch 入力を規約どおりに処理し、request 要素の応答だけを配列で返す。
  - derives_from: DOC-ANNEX-A §13.1 ／ 上流: BASIC §26.2
  - 入力 = request 2 件と notification 1 件からなる batch → 期待 = 応答配列の要素数が 2 で、notification の応答が含まれない
- **VO-IFACE-RPC-04** — 不正な転送・不正入力に対し、`code` / `message` / `candidates` を伴う tool error を返す。
  - derives_from: DOC-ANNEX-A §13.1 ／ 上流: BASIC §26.2
  - 入力 = 解析不能な JSON → 期待 = `ok: false` と `code` を持つ tool error
  - 入力 = 存在しない VO ID を与えた `vo_get` → 期待 = `code` と `candidates` を伴う tool error
- **VO-IFACE-RESCAN** — MCP の長時間実行中もソース変更を再スキャンし、stale な `PASS` を保持しない。
  - derives_from: DOC-ANNEX-A §13.1 ／ 上流: BASIC §26.2
  - 入力 = MCP セッション中にソースを変更してから `verify` を再実行 → 期待 = 変更が反映され、変更前の PASS が保持されない
- **VO-IFACE-NONINTERACTIVE-01** — CLI と MCP のすべての操作が対話的な確認入力の待受けなしに完結する。
  - derives_from: DOC-ANNEX-A §12.1 ／ 上流: BASIC §26.2・ANNEX-A §13.1
  - 入力 = 標準入力を閉じた状態での各 CLI コマンド → 期待 = 入力待ちでブロックせず終了コードを返す
  - 入力 = 確認を伴う操作に `--yes` を与えない実行 → 期待 = プロンプトで停止せず完結する

---

## VO-INVARIANT（領域横断の不変条件）

- **VO-INVARIANT-FORM-01** — 同一の静的事実（宣言 target への呼出が Test 本体に現れ、その結果が assert 相当へ到達する）を持ち、assert の所在だけが異なる 2 つの Test は、4 検査すべてで同一の状態値になる。assert の所在ごとの緩い判定規則を設けない。
  - derives_from: DOC-DETAIL §7.2（暫定。ESCALATE-1・ESCALATE-2） ／ 上流: BASIC §5・§5.4・§8.2
  - 入力 = Test A が本体で直接 `assert_eq!` する。Test B は同一の target 呼出結果を helper へ渡し、その helper は別 Test の宣言 target で `oracle_presence` = PASS → 期待 = A と B の `chain_integrity` / `orphan_detection` / `target_binding` / `oracle_presence` の 4 値がすべて一致する
  - 注: 実行形態（process / thread boundary）軸の不変条件は詳細設計 §7.3 と両立しないため本 VO の範囲外とした（ESCALATE-1）
- **VO-INVARIANT-SEVERITY-01** — 警告重大度だけの診断は、対象検査の検証値を降格も昇格もさせない。
  - derives_from: DOC-DETAIL §5.4 ／ 上流: DETAIL §23
  - 入力 = 孤立 VO の警告（W-SCAN-102）のみが存在するリポジトリ → 期待 = 4 検査の値が警告の有無で変わらず、総合 OK が保たれる
  - 入力 = 中間 VO への直接参照の警告（W-SCAN-103）のみ → 期待 = 同上
  - 入力 = 非 Test construct 表面の未知キー警告（W-SCAN-105）のみ → 期待 = 同上
- **VO-INVARIANT-SCANSCOPE-01** — スキャンは自由記述の文書本文から参照位置を構文推測せず、参照の意味的妥当性・取り込み完全性を検査しない。document の鮮度は記録済み `content_hash` と実ファイルの照合、およびレコード水準の参照存在だけを根拠とする。
  - derives_from: DOC-DETAIL §3.1・§11.4 ／ 上流: DETAIL §23
  - 入力 = 本文に他文書への言及を含むが `derives_from` に記載しない document → 期待 = 本文の言及を参照として拾わず、`chain_integrity` = PASS
  - 入力 = 本文と `derives_from` の内容が意味的に食い違う document → 期待 = 意味の食い違いを検出せず、`content_hash` が一致する限り非 PASS にしない

---

## VO-DETERM（決定性）

- **VO-DETERM-VERIFY-01** — `verify` の入力は scan 結果・Evidence・判断記録のファイル集合だけであり、同一入力に対して同一の出力を返す。検証成立条件を外部 AI / Agent の能力へ依存させない。
  - derives_from: DOC-BASIC §11.1 ／ 上流: BASIC P-003・DETAIL §7.1
  - 入力 = 同一のリポジトリ状態に対する `verify --format json` の 2 回連続実行 → 期待 = `ok` / `scope` / `data` / `diagnostics` が完全に一致する（実行時刻等の非決定 field を含まない）
  - 入力 = ネットワークを遮断した環境での `verify` → 期待 = 結果が変わらず完走する
- **VO-DETERM-CANON-01** — adapter の Target Reference 正規化は、同一の source state に対して反復的に同一の canonical Target Reference を生成する。
  - derives_from: DOC-DETAIL §9.2・§6.1 ／ 上流: BASIC §27
  - 入力 = 同一リポジトリに対する `scan` の 2 回連続実行 → 期待 = 各 Test の canonical Target Reference 集合と Source Target の内容ハッシュが完全に一致する

---

## VO-DELIV（成果物受入）

- **VO-DELIV-README-01** — README に、宣言された義務の裏付けの検証のみで出荷し宣言されていない実装の存在を関知しない旨の非関知宣言を記載する。
  - derives_from: DOC-BASIC §29（OOS-005） ／ 上流: REQ R-2
  - 入力 = リポジトリの README → 期待 = 未宣言の実装を検出しない旨の宣言が 1 箇所以上存在する
  - 注: 本 VO の検証はリポジトリ内容の照合であり、vtest の実行時挙動の検証ではない（ESCALATE-4）

---

## 既存 VO レコードの処遇

`.verify/vo/VO-DOGFOOD-M3-STATIC-AUDIT.yaml`（claim: "Static rules bind the declared target and result flow without promoting ambiguity to PASS"）:

- **再ターゲット**: covering test `TEST-DOGFOOD-M3-TARGET-RULES`（`classify_target_call`）は DA-002 の target 解決を行使するため、**VO-TARGET-STATIC-01** へ再ターゲットする。claim が "result flow" にも触れる部分は **VO-ORACLE-DA003** の領分であり、当該 Test が両者を行使するなら covers を 2 件宣言する（VO-VOMODEL-MULT-01 が許容する N:M）。
- **schema 移行**: 現ファイルは旧モデル（`requirements` / `spec_refs` / `status`）で書かれている。新 schema（`derives_from` / `parent` / `claim` / `dimensions` / `coverage_policy` / `combinations`）へ書き換える。`status` は VO-APPROVAL-STATUS-02 が W-STORE-001 を誘発する非正典 field として扱う。
- **実装側の追随**: `crates/vtest-audit/src/lib.rs` の `@vtest.covers` を新 VO ID へ追随させる（実装 PR の範囲）。

---

## ESCALATE（VO として起草しきれなかった裁定）

| id | 裁定 | 起草した範囲 | 未解決の点 |
|---|---|---|---|
| ESCALATE-1 | UNCOV-16dc1bdf（形態非依存の判定一致） | assert の所在（本体直書き / helper 委譲）だけが異なる 2 Test で 4 検査値が一致することを VO-INVARIANT-FORM-01 とした | Owner 裁定の文言「実行形態に依らず同一」を実行形態（process / thread boundary）軸へそのまま適用すると、詳細設計 本冊 §7.3 と両立しない。§7.3 は、呼出が Test 本体に現れない subprocess E2E で DA-003 が `UNKNOWN` のまま残り `oracle_presence` = PASS に到達しないことを明記している。したがって「同一入力事実を持つ形態違いの 2 Test は 4 検査値が一致する」を実行形態軸で普遍主張として立てると、§7.3 に対する反証テストが直ちに書ける。実行形態軸まで不変条件とするなら §7.3 の改訂が先に必要であり、Owner の裁定を要する |
| ESCALATE-2 | UNCOV-03ed7443 / FEAT-f35e635e（照合装置のグラフ終端） | Owner 裁定の意味論をそのまま VO-ORACLE-TERM-01 / -02 とし、VO-ORACLE-TERM-03 で正方向を固定した | 当該裁定に対応する詳細設計の改訂（本冊 §7.2 の DA-003 / DA-006 と標準 assert 集合、別紙C §18.3.3 の oracle 行）が本ブランチにまだ入っていない。`derives_from` を暫定的に本冊 §7.2 としているため、改訂のマージ後に節参照の照合が要る。現行 §7.2 の DA-006 は「関数内に assert 相当が 1 つも存在しない」を `FAIL` とするだけであり、helper 委譲の終端規則を持たない。VO-INVARIANT-FORM-01 も同じ改訂に依存する |
| ESCALATE-3 | WEAK-6f4ffbf7（Test Intent の非ノード性） | 「Intent が独立ノードとして検証グラフに現れず、`input` / `expect` の欠落が `chain_integrity` を非 PASS にしない」を VO-CHAIN-TEST-01 のケースとして起草した | 裁定の obligation は「Test Intent の有無や欠落を宣言鎖検査の成否として扱わない」だが、詳細設計 本冊 §4.4 は `intent` を core 中立の必須 metadata とし、欠落を E-SCAN-007（`chain_integrity` = MISMATCH）としている。`intent` field の欠落まで「宣言鎖の成否に用いない」と読むと §4.4 と直接矛盾するため、ノード性と `input` / `expect` の任意性に限定して起草した。この限定の可否は Owner の裁定を要する |
| ESCALATE-4 | EXTRACT-542a640e（README の非関知宣言） | VO-DELIV-README-01 として leaf 化した | 本 VO の検証対象は vtest の実行時挙動ではなくリポジトリの文書内容であり、他の 198 leaf と検証手段の種類が異なる。VO セットの中に置くか、リリース時のチェックリスト項目として VO の外へ出すかは Owner の裁定を要する。leaf として残した場合、covers する Test を rust-cargo Test として書くには README を Source Target として宣言する必要があり、`targets ≥ 1` の要求（VO-ADAPTER-TARGETREQ-02）と整合しない |
