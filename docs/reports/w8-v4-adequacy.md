# freeze v4 新規11 VO の adequacy 評価（PARTIAL 8・UNSUPPORTED 1・CONTRADICTED 候補 2）

wf_cdd2a414-e24（11 agents, opus/effort-high, repo read-only, contradiction duty 込み）。全 dossier: `docs/plans/dogfood-v4-adequacy.json`。

## 結果

| VO | final | 要点 |
|---|---|---|
| VO-ADAPTER-16 | PARTIAL | 正 facet は init/writer の同一 surface で compose。未証: 「flat v1 を決して吐かない」の全称負 facet（to_yaml に **latent な v1 branch が現存**、store/lib.rs:195-212 — 到達 caller 無しで VO-EXEC-14 と同格の免罪。remap 時に unreachable pin 推奨） |
| VO-ADAPTER-17 | **UNSUPPORTED** | 下記裁定 |
| VO-EXEC-15 / VO-REGISTRY-16〜20 / VO-STRUCTOP-16 | PARTIAL（7件） | sensor + 周辺が正 facet を張るが負/境界 facet に穴（各 dossier 参照） |
| VO-REGISTRY-21 | **CONTRADICTED 候補** | cluster 4 への結線を具体確認（下記） |
| VO-STRUCTOP-15 | **CONTRADICTED 候補** | 新規（下記・検証 agent 走行中） |

## 裁定

### VO-ADAPTER-17（UNSUPPORTED — ontology 維持・mapping 除去）

adequacy agent は「ordering claim は ontology over-reach」と主張したが、**規範文は実在する**（詳細設計 §5.1 L562「登録順ではなくadapter ID順にSourceDiscoveryAdapterを呼び出す」— freeze v4 時に main thread が grep 検証済み。agent はこの文を引いていない）。よって **VO は維持**。ただし agent の evidence 側の指摘は正しい: sensor（TEST-AAPI-002）は registry 列挙 helper を pin するだけで**委譲順序は検証していない**（verdict NO）し、現実装は single-adapter 固定で vacuous 適合。→ **final UNSUPPORTED**（spec-ahead-of-impl の正当な未証 VO）、covers rule（PROVES|PARTIAL のみ）により **TEST-AAPI-002 → VO-ADAPTER-17 の mapping を除去、同 test は supporting へ**。多 adapter milestone で試験可能になる。

### VO-REGISTRY-21（CONTRADICTED 候補 — cluster 4 結線確定）

結線は推定でなく具体確認された: **診断 surface は claim を守り（cross_entity_diagnostics は arity を正しく数えて E-SCAN-004 発行）、解決 surface が破る**（find_target_source の `.find()` が 2+ 一致でも先頭を解決、consumers に arity guard 無し）— この分裂そのものが cluster 4 の確定欠陥。cluster 4 の fix が本 VO を救済する。**副次の新規候補**: adapter 側 resolve_target/find_function は unique **suffix** 一致で解決（「matches it exactly」でない）＋ §907「各 subsystem が独自に candidate 列を走査して1件を選ぶ経路を持ってはならない」への違反 — 独立の per-subsystem 非対称事例として修正計画に追記価値。remap 注意: E-SCAN-004 は3意味（構文不正/未知 Cargo target/arity 失敗）に overload されており evidence 集計前に分別要。

### VO-STRUCTOP-15（CONTRADICTED 候補 — 検証中）

unit form の `file` は required:false、integration は required:true（forms.rs:42-46 vs 98-102）。8点鎖検証（dossier: `docs/plans/dogfood-vo-structop-15-dossier.json`）: **CONFIRMED（宣言成果物レベル）**。

- **決め手の判別原則**: integration の正当な差分は全て corpus のどこかで*宣言*されている（targets=§14.3 / kind 接頭辞=詳細設計 §4 L523 / kind・title=identity field）のに対し、`file` の required 非対称だけが**沈黙による逸脱** — これが NEEDS-SPEC-JUDGMENT を退けた。別紙A §14.1 は unit の `file` を `required: false` と逐語宣言、§14.3 は「他は同一」。破れは受理判定でなく**宣言成果物**（全 `vtest init` の出力 + `form_get` payload）に現れ、隔離 repro で再現済み。
- **候補の強制箇所は反証**: operations.rs:920-927 の required 検査は create 経路で**不活性**（field 順で `unique-fn-name` 検証器が先に `destination_file` を呼び遮蔽 — 反実仮想 repro: フラグを false に書換えても同一の失敗）。実際の拒否は operations_support.rs:157-160 の別機構。→ required フラグは死んだ宣言不整合 + 拒否は独立経路、という**二層の欠陥**。
- **VO claim 自体の欠陥も発見**: 「targets のみで異なる」は kind 接頭辞（`unit-*`/`integration-*` — 詳細設計 §4 L523 が宣言）の存在により修正後実装でも偽 → **claim を仕様宣言済み差分の列挙形に補正**（v4 JSON 修正済み、gate=CLAIM-CORRECTED-POST-VERIFICATION）。
- **Owner 二択（P-001）**: (a) §14.3 側に integration の `file` 必須を宣言追加 / (b) forms.rs:101 を false へ反転 + operations_support.rs に targets 対応の導出 fallback を追加（**フラグ反転だけでは通らない**ことを dossier が実証済み）。
- 副次: 実装の Form title/question が英語で別紙A の日本語宣言と文字列不一致（identity field ゆえ射程外、別件記録）。

## 数値正典への反映

- covers 提案: **183 tests / 399 mappings / auxiliary 9**（supporting 6 + anchor-none 1 + A 2）。
- 171 VO の per-VO 集計（全 wave 合算）: PROVEN 14 / PARTIAL 67+5+8=**80** / UNSUPPORTED 31+**1**=32 / CONFIRMED 39 / REFUTED 2 / NEEDS-SPEC-JUDGMENT 2 / 未検証 CONTRADICTED 候補 2（REGISTRY-21 は cluster 4 fix に随伴、STRUCTOP-15 は検証中）。
- PROVEN 0 継続の含意: 新 VO はいずれも sensor 由来の正 facet のみ — remap 適用後の通常運用（negative facet の補強）で上げていく領分。
