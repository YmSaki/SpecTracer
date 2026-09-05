# 上流整合性監査 13 箇所の正典ノード処置案（2026-09-06）

`reports/upstream-traceability-audit-2026-09-04.md` §5 が挙げた 13 箇所 / 8 主題 / **29 ノード id** について、`docs/canonical/specification.json` を一次資料として逐語確認し、ノードごとの処置を**提案**する。

**本書は提案であり、決定ではない。** `docs/` も `specification.json` も `relations/retired-ids.json` も本書の作成では一切変更していない。

---

## 凡例

### 処置ラベル

| ラベル | 意味 |
|---|---|
| **DELETE** | 当該文が下流で発明した規範であり、上流の明文が既に必要なことをすべて言っている。ノードを退役させる。 |
| **REWRITE** | 当該文に上流の規範を書き写す。**転記のみ**で、上流に無い内容は足さない。置換後の逐語文を本書に示す。 |
| **OWNER-DECISION** | 演繹で閉じない。二つの読みと、なぜ閉じないかを示す。 |
| **対象外** | 監査 §5 が「該当する文はこの中の1つ」とした範囲に含まれるが、指摘された規範を運んでいない無関係な隣接文。処置しない。 |

### 層と上流方向

正典 7 層の順序は `root` → `request` → `require` → `spec` → `detailed_spec` → `basic_design` → `design`。接頭辞は `ROOT-` / `R-`・`P-` / `REQ-` / `SPEC-` / `DS-` / `BD-` / `DES-`。

**重要な注意（層の仕分けの影響）**: 監査 md は上流を元文書名（要件定義・基本仕様）で書いているが、正典 json は文ごとに層を振り直している。そのため元「基本仕様」の文のいくつかは `detailed_spec`（DS-）に落ちている。本書は上流を**正典の層**で示す。主題 B は、この仕分けの結果として上流と下流が同一層（detailed_spec）に並ぶ箇所がある。該当箇所で明示する。

### 逐語引用について

`statement` 欄はすべて specification.json からの逐語である。バッククォート・記号を含めて原文どおりに写した。

---

## A. 複数 Source Target の宣言を integration 系に限る

### 上流（逐語）

| id | 層 | statement |
|---|---|---|
| `REQ-150` | require | 1 つの Test は 1 件以上の Source Target を宣言できる。 |
| `REQ-151` | require | 複数 target を宣言した場合も、各 target を独立に識別し、代表 1 件へ縮約してはならない。 |
| `SPEC-085` | spec | 1つのTestは1件以上のSource Targetを宣言できる。 |

条件は付いていない。`kind` にも実行形態にも言及がない。

### 処置案

| node id | statement（逐語） | 処置 | 上流 | 置換文 / 理由 |
|---|---|---|---|---|
| `DS-494` | `` `case` と `related` はキー自体を複数行書ける。`` | **REWRITE** | REQ-150 / SPEC-085 | 置換文: ``  `case`・`related`・`target` はキー自体を複数行書ける。 `` |
| `DS-495` | `` `case` と `related` 以外のキーの重複はエラーE-SCAN-005とする。 `` | **REWRITE** | REQ-150 / SPEC-085 | 置換文: `` `case`・`related`・`target` 以外のキーの重複はエラーE-SCAN-005とする。 ``（例外を列挙している文なので、DS-494 の書き換えだけでは効かない） |
| `DS-496` | ただし `kind` がintegration系のTestに限り、`target` の複数行を許容する。 | **DELETE** | REQ-150 / SPEC-085 | 制限そのものが detailed_spec の発明。上流は無条件で許可している。 |
| `DS-497` | 許容された複数 `target` 内でも同じTargetRefの重複はE-SCAN-005とする。 | **REWRITE** | REQ-151 | 置換文: `複数 `target` 内でも同じTargetRefの重複はE-SCAN-005とする。`（「許容された」＝ DS-496 の条件付き許可への参照を落とすだけ） |
| `DS-498` | 綴りが異なっても解決後に同一canonical Source Targetへ到達する複数宣言（同じSource Targetへのlocator参照とSRC ID参照の併記等）も、coreが解決時にE-SCAN-005とする（§6.1.1）。 | 対象外 | — | 同一 Source Target への二重宣言の話であり、複数 target の可否とは別。 |
| `DS-1247` | `` `rust-integration` は空 list と重複 target を E-OP-001 で拒否する。 `` | 対象外 | — | Form の入力検証。件数制限ではない。 |
| `DS-1248` | `` `target` キーは integration 種別に限り複数行を許容する。 `` | **DELETE** | REQ-150 / SPEC-085 | DS-496 と同文の別紙A 側。 |

### DS-494 / DS-495 を REWRITE にする理由（DS-496 単独削除は壊れる）

DS-495 は「`case` と `related` **以外**のキーの重複はエラー E-SCAN-005」と、例外を逐語で列挙している。DS-496 はその例外に `target` を条件付きで追加する文である。DS-496 だけを消すと、複数 target を宣言したすべての Test が DS-495 の文面によって E-SCAN-005 になる。上流 REQ-150 が無条件に許した宣言が、削除の副作用で全面禁止に変わる。

DS-494（複数行書けるキーの一覧）を書き換えるだけでは足りない。DS-495 は自分で例外集合を書いているので、両方に `target` を加える転記が要る。**DS-494 と DS-495 の2文はセットで処置する。**

### 事実（規範ではない）

- **循環引用**: DS-496 の `cites` は `["別紙A §14.3"]`、DS-1248 の `cites` は `["本冊 §4.2の例外"]`。この制限の唯一の根拠は互いである。上流を指す cites は無い。
- **「integration系」に定義が無い**: 正典 json 全体で `kind` の integration 系という区分を定義した文は無い。近いのは `DS-661`「Testがtargetを静的解析の追えない実行境界を越えて到達させる形態は、Testのkind（unit / integration）とは独立に、execution topologyによって決まる。」で、これは kind と形態が独立だと述べている。`DES-434`/`DES-435` の `suite.kind`（lib / bin / integration）は `ExecutionDescriptor` の別 field であり `@vtest.kind` ではない。
- **仕様内部で閉じた矛盾**: 別紙A §14.1 の組込 Form テンプレートは `@vtest.kind unit-{test_kind}` を出力する（別紙A L524、ノードは `DS-1232` の schema 本文内）。`DS-1245`「`rust-integration` の §14.1 との差分はこの2点であり、他は同一。」であり、その2点は `file` 必須（DS-1242）と `targets` 入力（DS-1241）だけ。したがって `rust-integration` Form は `kind` を `unit-*` で出力しつつ `DS-1246`「`targets` の全要素を入力順に個別の `@vtest.target` 行として出力する」。仕様が用意した Form の出力を DS-496 が拒否する。
- **実装**: `crates/vtest-scan/src/lib.rs:1186-1195` が `kind` の `starts_with("integration")` で分岐し、それ以外で複数 `target` を E-SCAN-005 にしている。この制限は実装に入っている（事実であり、規範の根拠ではない）。

---

## B. rust-cargo の全 Test に `targets ≥ 1` を一律必須にする

### 上流（逐語）

| id | 層 | statement |
|---|---|---|
| `REQ-147` | require | 実装 construct（Source Target）を直接検証する実行形態では、Source Target 宣言をそのまま検証対象の宣言として扱い、同一対象の二重宣言を要求しない。 |
| `REQ-148` | require | 外部契約・境界上の振る舞いを検証する実行形態では、その契約または振る舞いを検証対象とし、内部 Source Target の宣言を Test 成立性の必須条件としない。 |
| `REQ-071` | require | 特定の実行形態の確認方法を、別の実行形態の Test へ一律に要求してはならない。 |
| `REQ-263` | require | 組込 production adapter は `rust-cargo` とし、Rust・Rust function unit test・小規模な integration test を対象とする。 |
| `REQ-055` | require | Test 層では、発見された各 Test に対応する管理宣言（構文上有効な Test ID・1 件以上の `covers`・その他の必須 metadata）がちょうど 1 件存在し、`covers` の全 VO 参照を解決でき、Test ID が発見結果全体で一意であることを要求する。 |
| `DS-178` | detailed_spec | 外部契約・境界上の振る舞いを検証する実行形態では、その契約・振る舞いを検証対象とする。 |
| `DS-179` | detailed_spec | 外部契約・境界上の振る舞いを検証する実行形態では、内部Source Targetの宣言をTest成立性の必須条件としない。 |

上流は**実行形態**で免除を切っている。指摘対象は **adapter 単位**で一律必須にしている。

**層の注意**: DS-178 / DS-179 は元「基本仕様 L355」だが正典では `detailed_spec` に入った。DS-567 / DS-568 も `detailed_spec` である。この主題は同一層内の矛盾として現れる。層をまたぐ根拠は require 層の REQ-147 / REQ-148 / REQ-071 が持っている。

### 処置案

| node id | statement（逐語） | 処置 | 上流 | 置換文 / 理由 |
|---|---|---|---|---|
| `DES-344` | `` `SourceDiscoveryAdapter` はadapterがTestとして認識した全Discovered Test draftを返す。 `` | 対象外 | — | 返却範囲の記述。必須 metadata に触れていない。 |
| `DES-345` | `` `ManagedTestDraftLink::One` は、構文上有効なTest IDと必須metadata（core中立の `covers ≥ 1` / `intent`、および当該adapterが必須とする追加metadata。`rust-cargo` では `targets ≥ 1`）をdraftとして具体化できる場合に設定する（§4.1・§4.4）。 `` | **REWRITE** | REQ-055 | 置換文: `` `ManagedTestDraftLink::One` は、構文上有効なTest IDと必須metadata（core中立の `covers ≥ 1` / `intent`、および当該adapterが必須とする追加metadata）をdraftとして具体化できる場合に設定する（§4.1・§4.4）。 ``（`` 。`rust-cargo` では `targets ≥ 1` `` の一句だけを落とす。残りは REQ-055 の転記として正当） |
| `DS-567` | 欠落はE-SCAN-007として報告する（§4.4・§5.4）。 | **DELETE** | REQ-148 / DS-179 | 「欠落」の主語は DES-390 が課した `targets ≥ 1`。規範が消えれば主語が消える。下記の注記を参照。 |
| `DS-568` | したがって `rust-cargo` のTestは従来どおりSource Target宣言を要し、挙動・Eコード・fixtureは本改訂で実効的に変わらない。 | **DELETE** | REQ-148 / DS-179 | 規範の言い直し＋改訂履歴の散文。上流に根拠が無いうえ、「従来どおり」「本改訂で実効的に変わらない」は現状維持の宣言であって仕様ではない。 |
| `BD-200` | `` `vtest-scan` はこれらのRust固有処理を実行しない。 `` | 対象外 | — | core / adapter 境界の記述。REQ-259 / REQ-260 に根拠がある。 |
| `BD-201` | 各管理対象Testに1件以上のSource Target（`targets ≥ 1`）を必須とすることはadapter層に属し、core中立の `chain_integrity` 必須リンクではない（§4.1・§11.1.1）。 | **DELETE** | REQ-055 / REQ-148 | 「その必須要求がどの層に属するか」を述べた配置の文。必須要求そのものが消えれば主語が無くなる。運んでいる境界の事実（core 中立の必須リンクに含まれない）は REQ-055 の列挙に `targets` が無いことの裏返しであり、新規の内容ではない。 |
| `DES-389` | `` `rust-cargo` adapterは§5.1の `DiscoveryBatch` を構築する。 `` | 対象外 | — | 構築責務の記述。 |
| `DES-390` | 当該adapterは検証対象をSource Targetとして実現する形態であり、各管理対象Testに1件以上のSource Target（`targets ≥ 1`）を必須とする。 | **DELETE** | REQ-148 / REQ-071 / REQ-263 | 本主題の主箇所。上流が実行形態で切った免除を adapter 単位の一律必須へ置き換えている。 |
| `BD-269` | fixture は、`rust-cargo` で `targets` を宣言しない Test（E-SCAN-007、`chain_integrity = MISMATCH`、診断`MISSING`）を表現できる。<br>description: `` `targets ≥ 1`は`rust-cargo` adapterの必須metadata。 `` | **DELETE** | REQ-148 / DS-179 | statement と description の両方を退役。受入 fixture が発明された規範を固定している。 |

### DS-567 の削除が依存しているもの（注記）

DS-567 を消しても E-SCAN-007 という診断コードは失われない。定義は `DS-541`「E-SCAN-007はerrorであり、必須metadata（core中立: id / covers ≥ 1 / intent、および当該adapterが必須とする追加metadata。`rust-cargo` では targets ≥ 1）の欠落を意味する。」が持っている。

ただし **DS-541 自身が本主題の同型候補**である（後述の未確認候補表に含む）。DS-567 の削除が寄りかかっているのは DS-541 の**総称部分**（「必須metadata…の欠落」）であって、末尾の「`rust-cargo` では targets ≥ 1」ではない。DS-541 を処置する際は、この総称部分を残す必要がある。

### DES-390 の追加の問題（REQ-263 との衝突）

DES-390 の前段「当該adapterは検証対象をSource Targetとして実現する形態であり」は、`rust-cargo` を REQ-147 の形態**だけ**の adapter だと断定している。REQ-263 は「`rust-cargo` とし、Rust・Rust function unit test・**小規模な integration test** を対象とする」と言っている。integration test は REQ-148 が免除した形態を含みうる。前段の断定そのものが上流と衝突する。

### 削除後に残る問い（**未決の開示項目**。処置ではない）

上流 §4.3（REQ-S011）を全項読んだ結果を示す。

| id | statement |
|---|---|
| `REQ-067` | 静的に確定できなければ `UNKNOWN` とし、動的証拠で昇格できる。 |
| `REQ-068` | 実装 construct（Source Target）を検証対象とする実行形態では、宣言された対象コードが実際に Test 実行経路へ入ったことをこの確認方法とする。 |
| `REQ-070` | 他の実行形態における確認方法は、§8 条項 3 に従い、当該形態に適した方法として下位仕様で定める。 |

上流は「PASS へのフォールバックは起きない」（REQ-067、および REQ-265）までは閉じている。しかし**境界検証形態の rust-cargo Test について `target_binding` の確認方法が何か**は REQ-070 が下位仕様へ委譲したまま、detailed_spec がそれを書く代わりに一律必須を書いた。DES-390 らを削除すると、この委譲が空のまま残る。

これは**処置の可否を左右しない**（一律必須は上流違反なので削除は独立に成立する）。上流が委譲した穴として別途起票すべき項目として開示する。

### 同型の未確認候補（監査 §5 の 13 箇所の外。**未確認**として列挙のみ）

監査 §2 の主題 B が「反証役が同型として列挙」とした 詳細設計 L620 / L856 / L873 / L1418 / L1454 のノード id、および本書の作成中に `E-SCAN-007` / `targets ≥ 1` の grep で見つかった同文を挙げる。**上流照合はしていない。** 13 ノードだけを退役させても、これらが同じ規範を言い直したまま残る。

| 元行 | node id | 層 | statement 抜粋 |
|---|---|---|---|
| L620 | `DES-243` | design | `TestEntity.targets` の件数はadapterが定める（`rust-cargo` は `targets ≥ 1` を必須とする）（§4.1・§4.4）。 |
| L620 | `DES-244` | design | coreは `targets ≥ 1` を中立必須にせず、`TestEntity.targets` の型としては空を許容する。 |
| L856 | `DES-384` | design | 検証グラフのエッジ `TEST → SRC` は `targets` であり、検証対象をSource Targetとして実現する形態、1:N（`rust-cargo` では `targets ≥ 1`）である（§4.1）。 |
| L873 | `DS-541` | detailed_spec | E-SCAN-007はerrorであり、必須metadata（core中立: id / covers ≥ 1 / intent、および当該adapterが必須とする追加metadata。`rust-cargo` では targets ≥ 1）の欠落を意味する。 |
| L1418 | `DS-780` | detailed_spec | `chain_integrity`は評価地点を…Testの管理宣言（…その他の必須metadata〔intent、および当該adapterが必須とする追加metadata。rust-cargoではtargets ≥ 1〕）… |
| L1454 | `DS-804` | detailed_spec | Test層は、発見された各Testに対応する管理宣言（…`targets ≥ 1`はadapter中立coreの必須リンクに含めず、当該adapterが必須とする追加metadataとして扱う〔`rust-cargo`では1件以上の`targets`〕…）… |
| （追加発見） | `DS-514` | detailed_spec | coreの `id` / `covers ≥ 1` / `intent`、および `rust-cargo` の `targets ≥ 1` という必須metadataを欠く場合はE-SCAN-007とし… |
| （追加発見） | `DS-686` | detailed_spec | v0.1の唯一のadapter`rust-cargo`では検証対象をSource Targetとして宣言しないTestはE-SCAN-007（`targets ≥ 1`欠落）として`target_binding`評価の手前で`chain_integrity`の`MISMATCH`になる。 |
| （追加発見） | `DS-1277` | detailed_spec | Test 層は、管理宣言または必須metadata（…`rust-cargo` では `targets ≥ 1`〕）を持たないTestが1件でもあれば… |
| （追加発見） | `DES-225` | design | E-SCAN-007はadapterが報告する構文・必須metadata診断であり、`targets ≥ 1` は `rust-cargo` の必須metadataとしてこの経路で検出される（core中立の必須リンクへは加えない）（§11.1.1）。 |

`DS-686` は特に強い。上流 REQ-148 が免除した Test を名指しで `MISMATCH` にすると書いている。

### 実装の事実（規範ではない）

**主題 B の規則は現行コードに実装されている。** これは事実の報告であり、規範の根拠ではない。

- `crates/vtest-scan/src/lib.rs:1178` — `@vtest.target` が無い Test に `kind` を見ずに E-SCAN-007 を出す。
- `crates/vtest-scan/src/lib.rs:2174` — その挙動を固定する assert。
- `crates/vtest-cli/tests/m1_acceptance.rs:325` — 受入テストが `("E-SCAN-007", "tests/diagnostics.rs")` を期待する。

上流を正とするなら、この実装と受入テストは規範側の修正後に追随する対象になる。実装がこうなっていることは仕様を維持する理由にならない。

---

## C. 孤児判定に第二条件を AND で追加

### 上流（逐語）

| id | 層 | statement |
|---|---|---|
| `REQ-059` | require | `orphan_detection` の問いは、親を持たない `document` ノードが存在するか、である。 |
| `SPEC-291` | spec | `orphan_detection` は文書層の孤児検出であり、親（上流document）を持たない `document` ノードが存在するかを問う。 |

一条件のみ。

### 処置案

| node id | statement（逐語） | 処置 | 上流 | 置換文 |
|---|---|---|---|---|
| `DS-572` | `` `derives_from` が空、かつ他のどのdocumentからも `derives_from` で参照されないdocumentのうち、`doc.roots` に列挙されないものを孤児とし、E-SCAN-016（`orphan_detection = MISMATCH`）とする。 `` | **REWRITE** | REQ-059 / SPEC-291 | 置換文: `` `derives_from` が空のdocumentのうち、`doc.roots` に列挙されないものを孤児とし、E-SCAN-016（`orphan_detection = MISMATCH`）とする。 `` |
| `DS-1335` | 孤児判定は、`derives_from` が空、かつ他のどの document からも `derives_from` で参照されず、`doc.roots` にも列挙されない document を孤児とし、E-SCAN-016、`orphan_detection = MISMATCH` になる。 | **REWRITE** | REQ-059 / SPEC-291 | 置換文: `孤児判定は、`derives_from` が空で `doc.roots` にも列挙されない document を孤児とし、E-SCAN-016、`orphan_detection = MISMATCH` になる。` |

### DELETE ではなく REWRITE にする理由

DS-572 は 詳細設計 §5.6 の定義本体であり、他の4文がここを参照している。

| id | statement |
|---|---|
| `DS-390` | `derives_from` が空のdocumentは根候補であり、`config.yaml` の `doc.roots` に列挙されない場合は孤児として `orphan_detection` の `MISMATCH` とする（§5.6）。 |
| `DS-547` | E-SCAN-016はerrorであり、根に指定されない孤児document（親documentを持たず `doc.roots` にも列挙されない）を意味する（§5.6）。 |
| `DS-563` | E-SCAN-016（孤児document）は `orphan_detection = MISMATCH` に写像する（§5.6）。 |
| `DS-781` | `orphan_detection`は評価地点をDOCとし、親を持たず`doc.roots`にも列挙されないdocumentが無ければ`PASS`、あれば`MISMATCH`とする（§5.6）。 |

これら4文はすべて**一条件**である。DS-572 を消すと参照先が消える。第二条件だけを落とす転記が最小の修正になる。

DS-1335（別紙C）は BD-286「fixture は、文書鎖の状態として `derives_from` が空かつ根に列挙されない孤児 document（E-SCAN-016、`orphan_detection = MISMATCH`）を表現できる。」が既に一条件で fixture を規定しているため、DELETE でも成立する。**REWRITE を推す**（受入仕様の判定文をそのまま残すほうが、fixture 側だけに定義が残る状態より読み手に安全）。

### 第二条件が漏らす具体例

`derives_from` が空で、`doc.roots` にも列挙されていない document `D` があり、別の document `E` が `derives_from: [D]` を宣言している場合。

- 上流の一条件: `D` は親を持たない → 孤児 → `MISMATCH`。
- 第二条件つき: `D` は `E` から参照されている → 孤児から除外 → 検出されない。

`D` は**宣言されていない根**である。文書鎖の頂点が `doc.roots` の宣言なしに存在する状態を、第二条件は通してしまう。検出を狭める方向の追加であり、fail-closed に反する。

出所として監査が挙げた `docs/plans/vo-proposal.md:331`（VO-ORPHAN-01）は plan 文書であり、規範ではない。

---

## D. Evidence の adapter 不一致だけを `MISMATCH` に割り当てる

### 上流（逐語）

| id | 層 | statement |
|---|---|---|
| `REQ-099` | require | 証拠が存在しない、または証拠のハッシュが現在の対象と不一致の場合、状態は `NO_EVIDENCE`（診断ラベルは STALE 等）となる。 |
| `REQ-120` | require | 鮮度の独立検査は設けず、鮮度喪失は診断ラベル（STALE）として説明する。 |
| `SPEC-427` | spec | 鮮度喪失の独立検査（旧`evidence_validity`）は設けず、鮮度は基本仕様§6のハッシュ束縛により満たし、喪失を診断ラベル`STALE`として説明する。 |
| `ROOT-032` | root | STALE は検証状態機械から除外。ハッシュ不一致で参照が外れた証拠に対する **診断ラベル** に降格。検証結果としては証拠なしと同じ NO。 |

### 処置案

| node id | statement（逐語） | 処置 | 上流 | 置換文 |
|---|---|---|---|---|
| `DS-823` | Evidenceのadapterが現在のTestのexecution.adapterと明示的に不一致の場合、`MISMATCH`とする。 | **REWRITE** | REQ-099 / SPEC-427 | 置換文: `Evidenceのadapterが現在のTestのexecution.adapterと明示的に不一致の場合、`NO_EVIDENCE`（診断STALE）とする。` |
| `DS-477` | 確認不能は `UNKNOWN`、明示adapterの不一致は `MISMATCH` とし、いずれも `PASS` へ昇格しない。 | **REWRITE** | REQ-099 | 置換文: `` 確認不能は `UNKNOWN`、明示adapterの不一致は `NO_EVIDENCE`（診断STALE）とし、いずれも `PASS` へ昇格しない。 `` |
| `DS-1398` | EvidenceのadapterがTest execution adapterと異なる場合はMISMATCHになる。 | **REWRITE** | REQ-099 | 置換文: `EvidenceのadapterがTest execution adapterと異なる場合は`NO_EVIDENCE`（診断`STALE`）になる。` |

### 兄弟条件との比較（同じ節の逐語）

`DS-S117`（11.2 Evidence 鮮度判定）は5条件を**並列**に列挙している。

| id | 条件 | 割当 |
|---|---|---|
| `DS-819` | test_subject hash 不一致 / targets 参照集合の不一致 | `NO_EVIDENCE`（診断STALE） |
| `DS-820` | revision.commit が null / HEAD 不一致 | `NO_EVIDENCE`（診断STALE） |
| `DS-821` | execution_state record 欠落 / hash 不一致 | `NO_EVIDENCE`（診断STALE） |
| `DS-822` | execution_state.complete が true でない / 再構築不能 | `UNKNOWN` |
| **`DS-823`** | **adapter 明示的不一致** | **`MISMATCH`** |
| `DS-824` | adapter 一致を確認不能 | `UNKNOWN` |

DS-818 は5条件すべての成立を有効性の定義としている。5番目だけが別経路へ分岐する根拠は上流に無い。別紙C 側も兄弟条件を `NO_EVIDENCE` にしている（`DS-1385`「Evidence記録後に宣言targetのいずれかが一意に解決できなくなった場合、記録済み参照集合が現在のcanonical集合と一致しないため`NO_EVIDENCE`（診断`STALE`）になり、`target_binding`をPASSにしない。」、`DS-1388`「canonical Test metadata、ExecutionDescriptor、Test construct、…がEvidenceと異なる場合はSTALE（`NO_EVIDENCE`、診断`STALE`）になる。」）。

### DELETE ではなく REWRITE を推す理由（および DELETE を採る場合の帰結）

adapter は既に subject hash に束縛されている。

| id | statement |
|---|---|
| `DES-260` | `ExecutionDescriptor` の `adapter` fieldは `AdapterId` 型である。 |
| `DES-077` | Test subject hashはdomain `vtest:test-subject:v1` を用い、adapter ID、Test ID、全canonical metadata、Source Locationのadapter・project-relative path・opaque locator、ExecutionDescriptor、および正規化したTest construct bytesを束縛する。 |

`Test.execution.adapter` が変われば `test_subject` hash が変わり、DS-819 が既に `NO_EVIDENCE` を出す。したがって DS-823 の条件は実質 DS-819 に包含されており、**DELETE でも上流の割当が失われない**。

それでも REWRITE を推すのは、DS-823 を単独削除すると DS-817「対象TestのEvidenceのうち最新のものについて、evidence.adapterが現在のTest.execution.adapterと一致することを検査する。」が**結果の記述を持たない検査**として残るためである。結果が書かれていない検査は、実装が結果を発明する場所になる。転記で `NO_EVIDENCE` を書けば、上流の割当を保ったまま兄弟条件と同型になる。

**DELETE を選ぶ場合**は DS-817 と DS-824 も同時に処置しないと穴が残る。この選択は Owner の裁量に属する。本書は REWRITE を提案し、DELETE を代替として開示する。

---

## E. 編集失敗時のロールバック義務

### 上流の探索結果

**require 層にも spec 層にも、公式編集経路が適用の途中で失敗したときのファイル状態を定める文は無い。** 8主題のうち、require / spec に上流の錨が無いのはこの主題だけである。

最も近い明文は次の2つ。いずれも**事前**（能力が無ければ何も生成しない）か、**目的**（事故を低減する）の記述であって、適用済み変更を戻す事後の義務ではない。

| id | 層 | statement |
|---|---|---|
| `REQ-217` | require | 公式経路の提供によって誤編集・更新忘れ・複数 Test 同時変更の事故を低減し、直接編集による不整合も検証で検出可能であることが望ましい。 |
| `SPEC-143` | spec | 公式経路の提供により誤編集・更新忘れ・複数Test同時変更の事故を低減する。 |
| `DS-297` | detailed_spec | create / edit / audit / run等の明示的操作に必須の能力がなければファイル・判断記録・Evidenceを生成しない。 |

`DS-980`「明示操作に必須のadapter capabilityが未提供なら、create / editではファイルを変更しない。」も**事前**の条件である。

### 処置案

| node id | statement（逐語） | 処置 | 上流 | 理由 |
|---|---|---|---|---|
| `DS-920` | `` `E-OP-003`はerrorであり、Create / Editの適用後検証に失敗（再パース不能、生成された宣言がdesired stateと不一致、変更が1 Testの範囲を超える）することである。適用前の状態へロールバックし操作を中止する（別紙A §15.2・§15.4）。 `` | **OWNER-DECISION** | 明文なし（REQ-217 / SPEC-143 からの導出は可能） | 下記の二読み |

### 二つの読みと、なぜ演繹で閉じないか

**読み1（保持し、導出であると記録する）**
ロールバックは REQ-217 / SPEC-143 の「公式経路によって事故を低減する」を実現する HOW である。適用後検証に失敗した編集を書きかけのまま残すと、公式経路を通ったほうが直接編集より状態が悪くなり、REQ-217 の目的が反転する。この読みでは DS-920 を保持し、`derived_from: ["REQ-217"]` を付けて**導出であることを明示**する。

**読み2（削除する）**
CLAUDE.local.md §2 の判別に従えば、「適用前の状態へ戻す義務」は規範（こうあるべき）であって事実ではない。規範は上位の仕事であり、上位に明文が無いなら detailed_spec の発明である。「答えはたいてい消す」に従えば削除。

**閉じない理由**: REQ-217 は「〜であることが**望ましい**」であり、義務を課していない。望ましさから義務は演繹できない。一方、削除すると「公式編集経路は部分適用された状態を残しうる」という安全性の欠落が生じ、これは上流が明示的に許したものでもない。**上流が沈黙している**のであって、上流が許可も禁止もしていない。どちらの読みも上流と矛盾しない。したがって Owner の裁定が要る。

**推奨**: 読み1（保持＋`derived_from` の明示）。8主題のうち、削除が安全性を失わせるのはこの主題だけである。ただしこれは推奨であって決定ではない。

### 同一義務を運ぶ他ノード（**いずれの読みを採っても同時に処置が要る**）

| id | 層 | statement |
|---|---|---|
| `DES-503` | design | 挿入後の再パース検証とロールバックは Edit と同一の規則で Create にも適用する。 |
| `DES-516` | design | ロールバック後は、当該操作より前と同じソーステキストが観測できなければならない。 |
| `DES-517` | design | 部分適用された挿入内容を残さない。 |
| `DES-520` | design | ロールバック後の再スキャンで、当該操作が無かった場合と同一のエンティティ集合・内容ハッシュが得られる。 |
| `DS-1559` | detailed_spec | ロールバック後に scan すると、当該 create 操作が無かった場合と同一のエンティティ集合・内容ハッシュが得られる。 |
| `DS-1560` | detailed_spec | 部分適用された挿入内容・採番された Test ID・Evidence・判断記録がいずれも残らない。 |

DS-920 だけを処置すると、義務は6ノードに残ったまま診断コードの定義だけが消える。

---

## F. Test ID 重複を操作エラーにする

### 上流（逐語）

| id | 層 | statement |
|---|---|---|
| `REQ-097` | require | Test ID 衝突が生じる場合、状態は `MISMATCH` となる。 |
| `REQ-199` | require | `M` は VO 参照の解決と Test ID の大局的一意性を検査する前の集合とし、解決不能な `covers` を持つ entity や、他の entity と Test ID が衝突する entity も含む。 |
| `REQ-055` | require | Test 層では、…Test ID が発見結果全体で一意であることを要求する。 |

REQ-199 は、Test ID が衝突する entity を**含んだまま** `M` を構成すると明示している。衝突は scan を中止させる操作エラーではなく、集合に載ったうえで `MISMATCH` になる。

### 処置案

| node id | statement（逐語） | 処置 | 上流 | 置換文 / 理由 |
|---|---|---|---|---|
| `DS-022` | registryの重複ID、未登録adapter、adapter間のTest ID重複は操作エラーとする。 | **REWRITE** | REQ-097 / REQ-199 | 置換文: `registryの重複ID、未登録adapterは操作エラーとする。`（`adapter間のTest ID重複` の一項のみ削除） |
| `DS-023` | registryの重複ID、未登録adapter、adapter間のTest ID重複は空のscanとして成功扱いしない。 | **REWRITE** | REQ-097 / REQ-199 | 置換文: `registryの重複ID、未登録adapterは空のscanとして成功扱いしない。`（既存の `derived_from: ["REQ-258"…"REQ-266"]` と `cites: ["要件定義 §21"]` はそのまま維持） |
| `BD-016` | source discovery、決定論的解析、Structured Test Operation、test runner起動、coverage計測はadapter capabilityとして提供する。 | 対象外 | — | 同じ元行にあるだけの隣接文。REQ-259 の転記。 |
| `DES-009` | adapterが返す導出結果はregistryでmergeし、adapter ID・path・Test IDの順に正規化する。 | 対象外 | — | 同じ元行にあるだけの隣接文。正規化順の記述。 |

同節の `DS-024`「どれかを正として他を修正させることはしない。」も対象外（隣接文）。

### DELETE ではなく REWRITE の理由

「registry の重複 ID」「未登録 adapter」の2項は要件定義 §5.4 の委譲の範囲内で正当であり、監査も「前2項目は…委譲内で正当」と確認している。文を丸ごと消すとその2項も失われる。第3項だけを落とす転記が最小になる。

### 基本仕様内部の矛盾（事実）

同じ基本仕様が2箇所で DS-022 / DS-023 と食い違っている。

| id | 元文書 | statement |
|---|---|---|
| `DS-212` | 基本仕様 §12 | `M` は解決不能な `covers` を持つentityやTest IDが衝突するentityも含む。 |
| `DS-298` | 基本仕様 §23（`DS-S056`） | Test IDの重複（identity collision）は `MISMATCH` とする。 |

DS-022 / DS-023 は自層の他の文とも矛盾している。

### 詳細設計は DS-022 に従っていない（事実）

| id | statement |
|---|---|
| `DS-536` | E-SCAN-002はerrorであり、Test ID重複（identity collision）を意味する。 |
| `DS-561` | `ManagedTestLink::Multiple`、E-SCAN-002（Test ID衝突）、E-SCAN-003（解決不能なVO参照）は `chain_integrity = MISMATCH` に写像する。 |
| `DS-1279` | `ManagedTestLink::Multiple`またはTest ID衝突（E-SCAN-002）は`chain_integrity = MISMATCH`になる。 |

E-SCAN-002 は `MISMATCH` であり、操作エラー（終了コード2）ではない。DS-022 を REWRITE すればこの不一致も解消する。

---

## G. ID 一意性を DOC / VO に一般化

### 上流（逐語）

要件定義が一意性を課しているのは Test ID と SRC ID だけである。

| id | 層 | statement |
|---|---|---|
| `REQ-055` | require | Test 層では、…Test ID が発見結果全体で一意であることを要求する。 |
| `REQ-155` | require | 恒久 SRC ID を使用する場合、その ID は adapter 境界を越えて repository 全体で一意でなければならない。 |

**DOC ID / VO ID の一意性を課した文は require 層に無い。**

基本仕様 §23（正典では `DS-S056`、DS-298〜DS-316）の `MISMATCH` 列挙にも DOC / VO の ID 重複は無い。列挙されているのは `DS-298` の「Test IDの重複」だけである。

### 処置案

DS-053 / DS-054 が属する `DS-S010`（3.2 ID 規則と関係リンク）の主語は `SPEC-043`「DOC / VO / TESTのIDは利用者（人間またはAI）が命名する。」であり、DOC / VO / TEST の3種すべてである。

| node id | statement（逐語） | 処置 | 上流 | 理由 / 置換文 |
|---|---|---|---|---|
| `DS-053` | IDの一意性はスキャン時に全数検査する。 | **DELETE** | REQ-055 | Test ID の全数検査は `DS-806`「Test層は、Test IDが発見結果全体で一意であること（衝突はE-SCAN-002）を評価する。」と `DS-897`「ID衝突はE-SCAN-002として検出する。」が既に持っている。DOC / VO へ一般化した部分は上流に根拠が無い。 |
| `DS-054` | ID衝突は `chain_integrity` の非 `PASS`（`MISMATCH`）とする（§5.1、§23）。 | **DELETE** | REQ-097 | `DS-298` / `DS-561` / `DS-1279` が Test ID 衝突 → `MISMATCH` を持っている。DOC / VO へ一般化した部分は上流に根拠が無い。 |

**代替（DELETE を採らない場合）**: 主語を Test ID へ限定する REWRITE も成立する。
- DS-053 置換文: `Test IDの一意性はスキャン時に全数検査する。`
- DS-054 置換文: `` Test ID衝突は `chain_integrity` の非 `PASS`（`MISMATCH`）とする（§5.1、§23）。 ``

**DELETE を推す**。§5.1 / §23 側に同じ規範が既にあり、§3.2 に重複して置く必要が無い。

### 監査が拾わなかった同型の隣接文

| id | statement |
|---|---|
| `DS-052` | ツールはID形式を強制せず一意性のみを強制する。 |

DS-052 も `SPEC-043` の DOC / VO / TEST を主語に持つ節にあり、DOC / VO の一意性強制を含意する。DS-053 / DS-054 を処置するなら同時に検討すべき。**監査 §5 の 13 箇所に含まれないため、本書では処置を提案しない。**

### DOC / VO の ID 重複は既に record 層が持っている（事実）

DOC / VO の ID 重複は到達可能であり（`id` field とファイル名は別、互換 reader の正規化経路もある）、その事象は既に record 層の診断が所有している。

| id | statement |
|---|---|
| `DS-544` | E-SCAN-010はerrorであり、レコードのid / ファイル名 / schema不一致、または互換正規化後のlogical record ID重複を意味する。 |

DS-054 は同じ事象に別の帰結（`chain_integrity` の `MISMATCH`）を割り当てている。上流に根拠が無いだけでなく、既存の割当と重複かつ不一致である。DELETE を推す根拠はこの点でも強まる。

---

## H. 編集の適用を「単一置換」に限定

### 上流（逐語）

| id | 層 | statement |
|---|---|---|
| `REQ-260` | require | 共通契約は特定言語の構文・構造を必須としない。 |
| `SPEC-135` | spec | Edit Testは、Test IDを編集ハンドルとして、adapterが識別する対象Testのmetadata宣言およびTest constructを更新する。 |
| `DS-225` | detailed_spec | 編集はadapterが特定した単一のmetadata宣言範囲とTest construct範囲に限定する。 |

上流は **metadata 宣言範囲と Test construct 範囲の2つ**を許容している。「1回の置換で適用する」とは言っていない。

**層の注意**: DS-225 は元「基本仕様 L462」だが正典では `detailed_spec`。指摘対象 BD-257 は `basic_design` なので、DS-225 は正典の層順でも上流にあたる。

### 処置案

| node id | statement（逐語） | 処置 | 上流 | 置換文 |
|---|---|---|---|---|
| `BD-257` | orchestration は Test ID と adapter ID で対象を一意に選択し、adapter が返す拡張範囲を単一置換として適用する。 | **REWRITE** | DS-225 / SPEC-135 / REQ-260 | 置換文: `orchestration は Test ID と adapter ID で対象を一意に選択し、adapter が特定した metadata 宣言範囲と Test construct 範囲へ適用する。` |

### 理由

BD-257 は**全 adapter に課される一般 orchestration 契約**である。`rust-cargo` では metadata 宣言（doc comment）と Test construct（関数）が隣接するので単一置換で足りるが、その隣接は Rust の構文的性質であって共通契約の性質ではない。REQ-260「共通契約は特定言語の構文・構造を必須としない」に反する。

adapter 固有の単一置換を述べた文はそのまま残してよい（削除も書き換えも不要）。

| id | 層 | statement |
|---|---|---|
| `DS-1553` | detailed_spec | editは1 Testの拡張rangeだけを単一置換し、他Testと通常sourceを変更しない。 |
| `DES-498` | design | 変更を、対象テスト関数の拡張 range（doc comment 先頭〜関数末尾）の単一置換として適用する。 |

DES-498 は「対象テスト関数の拡張 range（doc comment 先頭〜関数末尾）」と明示的に Rust 固有であり、`rust-cargo` の実現方法として正当。DS-1553 は別紙C の受入項目で、対象を `rust-cargo` として読める。**問題は BD-257 が言語非依存の契約層に置かれていることだけ**である。

---

## 集計

### 監査 §5 が名指しした 29 ノードの処置

| 処置 | 件数 | node id |
|---|---|---|
| **DELETE** | 9 | `DS-496`、`DS-1248`、`DS-567`、`DS-568`、`BD-201`、`DES-390`、`BD-269`、`DS-053`、`DS-054` |
| **REWRITE** | 12 | `DS-494`、`DS-495`、`DS-497`、`DES-345`、`DS-572`、`DS-1335`、`DS-823`、`DS-477`、`DS-1398`、`DS-022`、`DS-023`、`BD-257` |
| **OWNER-DECISION** | 1 | `DS-920` |
| **対象外**（無関係な隣接文） | 7 | `DS-498`、`DS-1247`、`DES-344`、`BD-200`、`DES-389`、`BD-016`、`DES-009` |
| **合計** | **29** | |

主題別の内訳: A = DELETE 2 / REWRITE 3 / 対象外 2、B = DELETE 5 / REWRITE 1 / 対象外 3、C = REWRITE 2、D = REWRITE 3、E = OWNER-DECISION 1、F = REWRITE 2 / 対象外 2、G = DELETE 2、H = REWRITE 1。

### 退役する id（DELETE 9件）

`relations/retired-ids.json` は `{"old_id", "new_id", "reason"?}` の配列である。移動先を持たない退役をこの形式でどう表すかは既存エントリに前例が無い（既存はすべて `new_id` を持つ id 付け替え）。**表現形式は Owner / 正典管理側の決定事項**として開示する。

```
DS-496    A  詳細設計 §4.2   integration 系限定の複数 target 許容
DS-1248   A  別紙A §14.3     同上（別紙A 側）
DS-567    B  詳細設計 §5.5   targets 欠落の E-SCAN-007 報告
DS-568    B  詳細設計 §5.5   「従来どおり…実効的に変わらない」
BD-201    B  詳細設計 §5.5   targets ≥ 1 必須の層帰属
DES-390   B  詳細設計 §5.5   rust-cargo 全 Test へ targets ≥ 1 必須
BD-269    B  別紙C §18.2     同 fixture（description 含む）
DS-053    G  基本仕様 §3.2   ID 一意性の全数検査（DOC / VO / TEST）
DS-054    G  基本仕様 §3.2   ID 衝突 → chain_integrity MISMATCH（同）
```

### derived_from への影響

**削除対象9ノードのいずれかを `derived_from` で指しているノードは存在しない**（全 3,854 ノードを走査）。

ただしこれは「削除して安全」を意味しない。正典の item レベルの `derived_from` は現状きわめて疎である。

| 層 / 種別 | derived_from を持つ / 総数 |
|---|---|
| detailed_spec / item | 142 / 1,586 |
| basic_design / item | 39 / 311 |
| design / item | 12 / 561 |
| spec / item | 75 / 461 |
| require / item | 16 / 337 |

節レベルの辺は存在するが、指しているのは**親節**（`DS-S080`、`DS-S087`、`DS-S117` など）であり、これらの節は item を1つ削除しても残る。したがって節の辺も切れない。

**実際の依存は `derived_from` ではなく本文の相互参照で成立している。** 本書が各主題で挙げた「同型の未確認候補」「同一義務を運ぶ他ノード」がそれである。id グラフだけを見て安全と判断してはならない。

### 処置を提案していないが同じ規範を運ぶノード（未確認・要追跡）

13 箇所の外にあるため本書は処置を提案しない。**これらを残したまま 29 ノードだけを処置すると、規範は生き残る。**

| 主題 | node id |
|---|---|
| B | `DS-514`、`DS-541`、`DS-686`、`DS-780`、`DS-804`、`DS-1277`、`DES-225`、`DES-243`、`DES-244`、`DES-384` |
| E | `DES-503`、`DES-516`、`DES-517`、`DES-520`、`DS-1559`、`DS-1560` |
| G | `DS-052` |

### 実装への波及（事実の報告。規範の根拠ではない）

| 主題 | 実装箇所 |
|---|---|
| A | `crates/vtest-scan/src/lib.rs:1186-1195`（`kind.starts_with("integration")` 分岐、それ以外は E-SCAN-005） |
| B | `crates/vtest-scan/src/lib.rs:1178`（`@vtest.target` 欠落で無条件に E-SCAN-007）、`crates/vtest-scan/src/lib.rs:2174`（assert）、`crates/vtest-cli/tests/m1_acceptance.rs:325`（受入期待） |

**主題 B の発明された規則は現行コードの E-SCAN-007 として実装されている。** これは事実であって、仕様を維持する理由ではない。上流を正とするなら、実装と受入テストが規範側の修正に追随する。

---

## 本書の限界（開示）

- 上流照合は監査 §2 が挙げた上流文を一次資料（specification.json）で逐語確認し、加えて各主題の周辺を grep で掃いた。**網羅的な逆方向監査（上流にあるのに下流に無い＝欠落）はしていない。**
- 「同型の未確認候補」に挙げたノードは上流照合をしていない。同文・同義に見えるという観察のみである。
- 主題 B の「削除後に残る問い」（境界検証形態の rust-cargo Test に対する `target_binding` の確認方法）は、上流 REQ-070 が下位仕様へ委譲したまま埋まっていない。処置の可否とは独立の未決事項として開示する。
- 置換文は上流の逐語からの転記として書いた。表記（バッククォート・読点）は当該節の既存文体に合わせたが、**採否と最終文言は Owner の判断による**。
