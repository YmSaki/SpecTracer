# Owner 判断キュー — 判断ブリーフ（2026-08-17）

## ★裁定結果（2026-08-17 Owner 回答）

1. **REGISTRY-15 = broad 採用**。目的（打鍵ミス検出）からして production 関数表面も含む。非テスト表面は error でなくてもよい — 「未定義キーを検出した」旨の **warning** を出す設計で可。
2. **STATAUDIT-01 = helper 追跡採用**。Owner の枠組み: 「helper に委任する = helper が正常にジャッジできるかをテストすることと同値」→ DA-006 も同一ファイル helper 1段を追跡してよい。視界外は UNKNOWN 退避。
3. **STRUCTOP-15 = 推奨どおり実装対称化**（file required:false + 全 targets 同一ファイルなら導出・跨るなら明示要求）。
4. **原子性 = 推奨どおり追補**（一文 + VO 新設）。
5. **black-box = 「ブラックボックスで考える」**: white-box 規則の緩和ではなく、black-box 視点から監査モデルを構想する（検証対象 = 境界契約、oracle = 境界観測）。設計着手は remap 後。
6. **Evidence Set = 第一級化は将来必要というのが当然**（Owner 同意）。remap は現状意味論で通し、post-remap の spec PR 候補として⑤と並ぶ。
7. **STORE-020/021 = ④追補の帰結として昇格**（削除しない。新 VO へ covers、remap 時に適用）。

→ ①②③④ は spec 変更を伴う。spec-only PR（develop 起点）として一括起案する。①③は実装変更も伴う（spec merge 後）。

---

7件。各項目 = 何の話か / 確定している事実 / 選択肢 / いつまでに要るか / 推奨。
一次資料: dogfood-vo-registry-15-dossier.md, dogfood-contradiction-verification.json (VO-STATAUDIT-01), dogfood-vo-structop-15-dossier.json, dogfood-agap-rederivation.json (STORE-020/021), w8-dogfood-findings.md (問題2)。

---

## 1. VO-REGISTRY-15 — `@vtest.` 打鍵ミス検出の適用範囲（仕様の二読）

**何の話か**: `@vtest.` で始まる未知キー（打鍵ミス）はエラー E-SCAN-006 になる — この規則が「テスト関数の doc comment 限定」か「adapter 所有宣言すべて（production 関数上の `@vtest.src-id` 含む）」か、仕様が二読可能。

**事実**:
- narrow 読み: §4.2 L513 が節の対象を「テスト関数直前の doc comment」と書く。現実装はこれ。
- broad 読み: §5.4 の E-SCAN-006 行は「adapter所有の宣言に未知field」とだけ書き（test 限定語なし）、§4.2 L526 は `@vtest.src-id` を**非テスト関数**（対象実装側）に置く。L524 の rationale は「打鍵ミスの検出を優先し、警告ではなくエラーとする」。
- 実測: production 関数に `/// @vtest.src_id SRC-X`（underscore の typo）→ **診断ゼロ・exit 0・src_id は None で登録**。誰かが SRC-X を参照して初めて E-SCAN-004 が「typo した production 関数ではなく参照側 test の位置」で出る。未参照なら完全な無音。
- 付随欠陥だった E-SCAN-005 の else-if 握り潰しは wave 1 で修正済み。

**選択肢**:
- (A) broad を正とする → 実装修正（discovery の early return 前に診断発火を移す。小規模）
- (B) narrow を正とする → 仕様修正（§5.4 に「Test metadata に限る」を明記し、L526 の src-id 表面は「typo 無検出」を仕様として受容）

**期限**: remap 前でなくてよい。ただし SRC ID を本格運用する前が望ましい（typo が無音で通る）。

**推奨: (A) broad**。L524 の rationale（打鍵ミス検出）は場所を問わず成立し、L526 が src-id を production 関数に明示的に置く以上、その表面だけ typo 無音は規則の意図と矛盾する。(B) は「src-id の typo は検出しない」を仕様に明記することになり、防御しにくい。

---

## 2. VO-STATAUDIT-01 — DA-006「退避例: なし」と保守的判定原則の緊張（仕様の二読）

**何の話か**: assert を同一ファイルの helper 関数へ委譲したテスト（本体に assert 構文ゼロ）を DA-006（検証構文なし）が FAIL にする。上位原則「決定論的に確定できる違反のみ FAIL、確定できないものは UNKNOWN として意味監査へ」（基本仕様 §7.2 L370、詳細設計 §7.1 L959、別紙C L94）と、DA-006 の行「UNKNOWNへ退避する例: **なし**」（詳細設計 §7.2 L985）が衝突。

**事実**:
- 実装は Pass/Fail の2値で Unknown 腕が存在しない。helper を追わない。
- 一方 DA-002/DA-003 は「同一ファイル内の呼出先 helper 1段」を追う実装が既にある（解析装置は存在する）。
- 実害: TEST-CLI-022/023/060 の3件がこの形（helper 内に assert 実在、全数確認済み）で誤 FAIL 相当。
- FAIL したテストは既定で意味監査 bundle から除外される（§7.2 L996）→ 原則が指定する「UNKNOWN → 意味監査へ送る」経路が閉じ、誤 FAIL は下流で回収されない。
- DA-006 は基本仕様の最小規則列挙（DA-001..005 相当）に**入っていない** — 詳細設計から下でのみ存在。文書優先順位は 要件定義 > 基本仕様 > 詳細設計。

**選択肢**:
- (A) 原則優先: DA-006 も同一ファイル helper 1段を追い、視界外は UNKNOWN 退避に改訂（詳細設計 §7.2 の行を修正）
- (B) 行優先: 現行維持。委譲テストは FAIL が正 — テスト本体に assert を書く規律をテスト側に課す

**期限**: remap 前が望ましい（3テストの adequacy 評価に直結）。

**推奨: (A)**。要件定義 §11 の違反クラスは意味的定義（「明らかに意味のないテスト」）で、helper 委譲テストは意味的には検証している。追跡境界は DA-002/003 と同一の「同一ファイル1段」に固定し、非決定性を避ける。

---

## 3. VO-STRUCTOP-15 — rust-integration Form の `file` field（双方向どちらに合わせるか）

**何の話か**: 別紙A §14.3 は「rust-integration は単一 `target` に代えて複数ロケータ `targets` を必須とする。**他は同一**」と宣言。しかし実装 schema は `file` が unit=required:false / integration=**required:true** で非同一。宣言 artifact レベルの矛盾は CONFIRMED（init が書く forms yaml と form_get の応答の両方で再現）。

**事実**:
- unit は `file` 省略時に target から追加先ファイルを導出して成功する。
- integration で `file` 省略 → E-OP-001 拒否。
- 補足: required flag は create 経路では不活性（false にしても同じ拒否が出る別実装経路がある）— wave 4 で operations が adapter routing に変わったため、この不活性側の現状は要再確認。

**選択肢**:
- (A) 仕様側を改訂: §14.3 に「integration は `file` 必須」と宣言を追加（最小変更）
- (B) 実装側を対称化: `file` を required:false にし、全 targets が同一ファイルに収まる場合は導出、跨る場合のみ明示要求（「他は同一」を最大限保つ。導出規則の設計が少し要る）

**期限**: 急がない。remap 非依存。

**推奨: (B)**。integration テストの targets が単一ファイルに収まるケースは多く、unit との対称性（「他は同一」）は annex の明示意図。ただし (A) でも矛盾は解消するので、仕様文を増やしたくなければ (A) で可。

---

## 4. 書込み原子性の spec gap — 追補するか

**何の話か**: TEST-STORE-020/021（record file の temp-file 経由の原子的公開・書きかけ残渣なし、を検証）が仕様のどこにも anchor できない。上流再導出の結論: 正規の並行耐性連鎖（要件定義 §29 → 基本仕様 §5.2 → 詳細設計 §16.1）は「ファイル素集合性 + append-only ULID + Git merge」で完結しており、torn read / temp file / atomic replacement に**一切言及なし**。原子性への言及は非規範の実装スケジュールにのみ存在。

**決めること**: record 書込みの原子性（部分状態が reader から観測されないこと）を仕様義務として追補するか、仕様沈黙のままにするか。

**判断材料**:
- 並列 AI 開発が製品前提。ファイル名衝突は ULID 設計で起きないが、**reader（scan/verify）が書きかけ file を読む torn read** は並行実行で設計上起きうる。
- torn read の帰結は「YAML parse 失敗 or 部分 record」— まさに今回別途見つけた「parse 時 silent drop」類の入力を発生させる側。fail-closed 製品として、入力面の保証を書く価値はある。
- 追補は一文で足りる（「record file の公開は原子的でなければならない」）。VO 1件新設で STORE-020/021 が verification テストに昇格。

**期限**: 急がない。remap 非依存（⑦の帰結にだけ影響）。

**推奨: 追補する**。コスト極小で、torn read は理論上でなく運用上の実シナリオ。

---

## 5. black-box topology — 契約テストの監査モデル（最大の未決仕様事項）

**何の話か**: suite の 84 件は subprocess/black-box 契約テスト（`Command::new(vtest)` / MCP stdio 駆動）。target 関数は**子プロセス内**で動くため、white-box 規則 DA-002（対象呼出）/ DA-003（結果検証）は構造的に UNKNOWN → これらのテストは all-12-PASS に**永遠に到達できない**。現モデルに「契約（CLI/MCP インターフェース）を対象とする black-box テスト」の第一級表現が無い（target は必ず Rust シンボル）。

**事実**:
- これは DA-003 が厳しすぎる問題ではなく**分解のミスマッチ**（あなたの当初仮説どおり、dogfood で裏付け済み）: black-box テストの検証対象は内部シンボルでなく境界契約であり、その oracle は境界 assert（runtime_result）が担っている。
- kind（integration）と topology（in-process/subprocess）は別概念 — kind では区別できない。
- role 仕様とも別軸: black-box の**verification** テストは role=verification のまま。topology の問題。

**選択肢**:
- (A) 仕様拡張: 契約 target 種別（CLI コマンド / MCP method 等）を導入し、black-box 用の到達性・oracle 規則を定義（role 仕様 PR と同規模の spec 工事）
- (B) 運用ルール: black-box は境界 assert + runtime_result で評価し、static_audit UNKNOWN を「期待される正常値」として文書化（丸めない。thesis 整合）
- (C) 現状維持: UNKNOWN のまま risk acceptance

**期限**: **remap には不要**（UNKNOWN のままでも covers は張れる。NOT_VERIFIED を丸めない thesis 通り）。「84件を all-PASS に到達させたい」なら要決定。

**推奨: 方向は (A)、時期は remap 後**。dogfood の実測が「境界契約こそ仕様義務の表面」を示しており、(B)/(C) は 84 件を永久に二級のまま置く。ただし設計が重いので、remap 完走を先に。

---

## 6. Evidence Set 第一級 — VO を複数テストで証明する合成の表現

**何の話か**: 1 つの VO を複数テストの evidence の**合成**で証明する場合、「この VO はこの evidence 集合で十分」という判定を担う first-class record が存在しない。現状の確定意味論（remap 前提）: covers = 対応宣言、十分性 = vo-coverage 監査の理由文に書く。第一級 record の不在は finding として記録済み。

**決めること**: vo-coverage 監査の理由文での表現を続けるか、Evidence Set record（VO → 構成テスト集合 + facet 割当て）を新設するか。

**判断材料**:
- adequacy 分析（88 partial VO の facet 合成）の経験上、理由文 free text では **facet 網羅の機械検証ができない**（再計算が人手/エージェント頼み）。第一級化すれば adequacy を check item にできる。
- ただし schema・監査・hash 束縛の全面追加 = role 仕様 PR 級の仕様工事。
- remap は現状意味論で安全に通せる（設計済み）。

**期限**: 急がない。remap 非依存。

**推奨: 保留継続（現状意味論で remap を通し、post-remap の spec PR 候補として⑤と並べて優先順位判断）**。

---

## 7. TEST-STORE-020/021 の削除可否 — ④の従属判断

**何の話か**: 原子的公開を検証する2テスト。covers は空。あなたは以前「削除に傾く」だった。

**選択肢**（④の決定で分岐）:
- ④で原子性を追補する → **削除しない**。新 VO に covers を張り verification テストへ昇格
- ④で追補しない → **削除不要になった**: role 仕様が merge されたので `@vtest.role supporting` + covers なしが合法に管理できる（「削除 or 偽 covers」の二択はもう存在しない）。原子的公開実装への regression 検出力は残る
- それでも消したい → 削除（失うのは上記の検出力のみ）

**推奨: ④に従属させる**。④追補なら昇格、④沈黙なら supporting で保持。単独で削除する理由は role 仕様 merge 後は無くなっている。

---

## まとめ表

| # | 件名 | 決めること | 期限 | 推奨 |
|---|---|---|---|---|
| 1 | REGISTRY-15 | E-SCAN-006 の範囲: broad/narrow | SRC ID 本格運用前 | broad（実装修正） |
| 2 | STATAUDIT-01 | DA-006 helper 1段追跡 or 現行 FAIL | remap 前望ましい | helper 1段 + UNKNOWN 退避 |
| 3 | STRUCTOP-15 | file 必須を仕様化 or 実装対称化 | 急がない | 実装対称化（導出） |
| 4 | 原子性 gap | 仕様追補 or 沈黙 | 急がない | 追補（一文） |
| 5 | black-box | 契約 target 種別の仕様化 | remap 後でよい | 方向は仕様化、時期は remap 後 |
| 6 | Evidence Set | 第一級 record 新設 or 監査理由文 | 急がない | 保留継続 |
| 7 | STORE-020/021 | 削除/supporting/昇格 | remap 前 | ④に従属（削除不要） |
