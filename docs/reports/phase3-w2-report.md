# Phase 3 / W2 状態報告

Current Phase: Phase 3
Current Wave: W2（store / config v2 / compatibility）
Branch: `feature/adapter-separation-alpha2-implementation`
Commits: `fad5d36`, `976fb14`, `cbdcfd8`, `40edadb`, `744c982`, `34382c3`（baseline `575ea72` / 前HEAD `88211c4`）

---

## 0. 引き継ぎ状況

codexは W1 まで commit 済み（`543c122` 〜 `88211c4`）、W2 は未コミットの作業ツリーとして残されていた。
handoff メモは存在しない（`.codex/` は設定のみ）。本作業はその W2 を継続し、残項目を完了させた。

引き継いだ未コミット分の provenance は当ブランチの作業ツリーであり、`rescue/invalid-alpha2` /
`rescue/pre-alpha2-local-wip` には一切触れていない（checkout / cherry-pick / merge / reset / delete いずれも無し）。
**これは前提であって証明ではない** — codexの未コミット作業が Owner 承認済みであることは検証できない。

---

## 1. Completion Criteria（上位計画 W2 完了条件 7項目）

| ID | Criterion | Evidence | Actual | Result |
|---|---|---|---|---|
| AC-1 | v1 configを無変更で読める | `version_one_without_full_scope_uses_fixed_twelve_without_rewrite` | PASS、config.yaml 無変更 | PASS |
| AC-2 | v2 initとround-tripが決定論的 | `init_writes_v2_adapter_namespace_and_form_owner` | PASS | PASS |
| AC-3 | v1 11項目 full_scope → 固定12 / v2の不完全・重複・unknown・extra scope → E-CONFIG-001 | `default_verify_evaluates_the_fixed_twelve_items`、`version_two_config_is_accepted_and_incomplete_scope_is_rejected`、`version_two_duplicate_full_scope_is_e_config_001`、`version_two_full_scope_rejects_unknown_and_extra_items` | 4件PASS（unknown項目・13個目のextra項目とも拒否） | PASS |
| AC-4 | Relation writerが`REL-<ULID>`、整合bare ULIDのみin-memory正規化、重複はfail-closed | `duplicate_bare_and_prefixed_relation_payload_is_rejected`、`relation_round_trip_requires_a_valid_immutable_record`、`relation_id_aliases_cannot_duplicate_one_ulid_payload` | 3件PASS | PASS |
| AC-5 | unknown / duplicate adapterがfail-closed | `unknown_adapter_rejects_scan_without_result`、`duplicate_adapter_id_rejects_scan_without_result`、`zero_adapter_registry_is_fail_closed` | 3件PASS | PASS |
| AC-6 | Rust adapter descriptorとcapability宣言が取得できる | `rust_cargo_registration()`（`vtest-adapter-rust`）、CLI `built_in_registry()` 経由でscanが解決 | PASS | PASS |
| AC-7 | `rust-cargo` codecがv1互換fieldを`execution`と損失なくround-tripし矛盾を拒否 | `version_one_compatibility_round_trips_losslessly`、`compatibility_contradictions_and_partial_fields_are_rejected` | 2件PASS | PASS |

## 2. Completion Criteria（本計画 §7 実装リストのうち上記に含まれないもの）

| ID | Criterion | Evidence | Actual | Result |
|---|---|---|---|---|
| AC-8 | adapter namespace / roots validation / duplicate root detection | `init_writes_v2_adapter_namespace_and_form_owner`、`adapter_roots_must_be_present_unique_and_project_relative` | 重複root（`crates` と `crates/`）・traversal（`../outside`）・roots空 の3経路とも拒否 | PASS |
| AC-9 | duplicate payload detection | `relation_id_aliases_cannot_duplicate_one_ulid_payload`（両ファイルとも不採用でE-SCAN-010） | PASS | PASS |
| AC-10 | VO status derivation（writerは保存しない／互換fieldは警告して無視） | `the_compatibility_status_field_never_reaches_the_record_subject`、doctor実行でW-STORE-001が実データに発火 | 実装＋PASS | PASS |
| AC-11 | Approval upstream dependency closure | AF-027 `specification_dependency_change_invalidates_vo_approval`、store単体6件 | RED→PASS | PASS |
| AC-12 | E-APPROVAL-001（closure未解決ならrecordを生成しない） | `approve_refuses_an_unresolvable_upstream_closure_without_writing_a_record` | exit 2、approval 0件 | PASS |
| AC-13 | dependencies欠落の互換Approvalを承認へ昇格しない（W-STORE-002） | `compatibility_approval_without_a_closure_is_never_approved` | PASS | PASS |
| AC-14 | Rust互換JSONのwire codec隔離 | `vtest-adapter-rust::RustCargoCodec`、CLI `ScanData::from_result` がregistry経由でencode | 実装 | PASS |

**Overall: PASS（W2固有条件は全件成立）**

---

## 3. Commands executed

```text
cargo fmt --all -- --check                                → OK
cargo clippy --workspace --all-targets -- -D warnings     → OK（警告ゼロ）
cargo test --workspace --no-fail-fast                     → 後述（既知REDのみ）
cargo run --quiet -p vtest-cli -- doctor                  → exit 0
```

## 4. Test 結果と既知RED

### 4.1 adapter acceptance（frozen）

`575ea72` freeze基準: **25 passed / 13 failed**（前HEAD `88211c4` では 15/21。AF-041 / AF-042 追加により母数が38へ）。
残る13件は全て後続waveが所有する RED であり、W2 の所有ではない。内訳は **W4所有3件 / W5所有10件**（§7 の G-4・G-5・G-7 と一致）。

- W4所有(3): `static_audit_ignores_run_only_config_changes`、`specification_only_change_stales_impl_consistency`、`impl_consistency_fail_maps_to_mismatch`
- W5所有(10) — うち Evidence 関連9件: `evidence_contains_neutral_subjects_and_complete_execution_state`、`evidence_without_execution_state_is_compatibility_stale`、`evidence_without_revision_commit_is_stale`、`execution_state_mutation_reports_e_exec_004_without_evidence`、`head_change_without_test_or_target_change_stales_evidence`、`incomplete_current_execution_snapshot_is_unknown`、`local_dependency_change_stales_evidence`、`multi_target_evidence_keeps_target_specific_results`、`target_external_helper_change_stales_evidence`
- W5所有(10) — 残り1件: `orchestration_crates_have_no_direct_rust_analysis_dependencies`。`vtest-scan`(syn)・`vtest-audit`(syn/quote)・`vtest-exec`(rustc-demangle)の3crateを同時に検査するため、W3・W4・W5が全て完了するまでGREENにならない

W2所有だったAF-027は本作業でRED→PASS。

### 4.2 crate単体テスト

全て green: adapter-api 3+12 / adapter-rust 2 / audit 12 / cli 6 / exec 6 / model 8 / scan 16 / store 26 / verify 11。

### 4.3 milestone acceptance（M1〜M9）

`88211c4` 時点で既に16件RED（W1のneutral model化により、消費側の更新は W3〜W6 が所有）。
本作業後は19件RED。**差分3件は全て contract 更新に伴うもので、回帰ではない**（後述 §5）。

| suite | baseline `88211c4` | 現在 | 差分 |
|---|---|---|---|
| m1 | 3 failed | 4 failed | +1 |
| m2 | 2 failed | 4 failed | +2 |
| m3 | 1 | 1 | 0 |
| m4 | 3 | 3 | 0 |
| m5 | 0 | 0 | 0 |
| m6 | 2 | 2 | 0 |
| m7 | 3 | 3 | 0 |
| m8 | 1 | 1 | 0 |
| m9 | 1 | 1 | 0 |

baseline は作業ツリーを一切変更せず、scratchpad への clone（`88211c4` checkout）で実測した。

---

## 5. Owner決定の適用結果

Owner判断（決定1〜4）を受けて U-1〜U-4 を確定・処理した。

### 決定1 — M1/M2 の3テスト: `REVALIDATION_REQUIRED / owner=W3`

| test | 旧contract主張 | 現行contract根拠 |
|---|---|---|
| `m1_invalid_project_config_is_a_json_internal_error` | 不正configは exit 3 / E-CORE-001 | Annex C §18.3.1「E-ADAPTER-* / E-CONFIG-*による操作拒否をexit 2」 |
| `m2_approval_rejects_a_missing_basis_before_mutating_the_vo` | canonical VO recordが`status`を持つ | 詳細設計 §3.5「VOの実効`status`は…canonical VO recordへ保存しない」 |
| `m2_vo_edit_invalidates_approval_and_returns_to_effective_draft` | 保存された`status`が`draft` | 同上 |

**状態: REVALIDATION_REQUIRED / owner=W3。W2では変更しない。**
これら3件および baseline から続く16件を含む M1〜M9 の既存PASSは、W3がPASSさせるまで
**現行contractのPASS evidenceとして使用しない。**

### 決定2 — Acceptance Freeze の欠落2件: 解消

`tests/ACCEPTANCE.md` に AF-041 / AF-042 を追加（commit `40edadb`）。
Specification clause / criterion / observable / minimal counterexample / fixture /
expected result / baseline classification / owning wave の全項目を
「### AF-041 / AF-042 freeze record」節に記載した。

baseline classification は推定ではなく実測: `075313a`（production tree が baseline `575ea72`
と同一。freeze commitはtestとfixtureのみを変更）で両テストを実行し、

- AF-041: canonical VO recordが `status: 'draft'` を保存 → **FROZEN_RED**
- AF-042: closureを持たないApprovalから `approved` を導出、`W-STORE-002` はコードベースに存在せず → **FROZEN_RED**

現行treeでは両方PASS（owning wave = W2）。

なお AF-042 の freeze record 文言は、freeze commit `40edadb` が aggregate 化 `744c982` より先行したため、
決定3適用前の `subject_hash` 定義（VOファイルのcontent hash）で記述されている。fixture構成の記述としては
現在も文字どおり正しいが、「current」の語義は決定3で変わった。criterionの判別力は現在 `W-STORE-002`
observable が担っており、テストは aggregate 化後も PASS する（closure欠落は `subject_hash` 照合と無関係に
発火するため）。frozen ledger の文言更新の要否は Owner 判断とする。

### 決定3 — `subject_hash` = 上流closureのcanonical aggregate hash: 実装済み

`Approval.subject_hash` は VOファイルのraw content hashではなく、

```text
VO record subject（§1.3 record-subject、互換field status を除外）
+ 上流closure（parent VO / REQ / 再帰parent REQ / SPEC subject）
        ↓ canonical order（kind, id 昇順）
        ↓ domain `vtest:approval-subject:v1`、length-prefixed
Approval.subject_hash
```

とした（`hash_approval_subject`）。各leafは§1.3のprimitive（`hash_record_subject` /
`hash_spec_subject`）を使う。`dependencies` fieldは§3.5の記録要求と診断の判別子として保持する。

判別の設計: recorded closure == current closure かつ aggregate不一致 → **VO自身の編集によるsupersede**
（§3.5「編集は承認をハッシュ不一致で自動失効させる」の正常経路なのでW-STORE-002を出さない）。
recorded ≠ current → `ClosureChanged`、closure欠落 → `ClosureAbsent`、いずれもW-STORE-002。

**正規仕様への反映が必要な差分（記録）:**

| # | 差分 | 対象 |
|---|---|---|
| D-1 | `subject_hash` を「対象VOの内容ハッシュ」から「上流closureを含むcanonical aggregate hash」へ | 詳細設計 §3.5 |
| D-2 | VO leafに raw file hash ではなく §1.3 record subject を使う（互換field `status` に不感応） | 詳細設計 §3.5 / §1.3 |
| D-3 | 新domain `vtest:approval-subject:v1` の追加 | 詳細設計 §1.3 |

これらはOwner決定による仕様明確化であり、実装だけで閉じない。独立spec PR経路での反映が必要。

### 決定4 — SPEC record hash と SPEC source hash の分離: 実装済み

- `hash_specification_source()` を source hash の単一定義とした。従来は `spec add` が `from_text`
  （LF正規化＋行末空白除去）、W-SCAN-104 が `from_bytes`（正規化なし）で不一致だった。
- `SpecRecord.sha256` の型を `SpecSourceHash` にした。registration時のsnapshotであることを型が表明し、
  record content hash が期待される位置では型検査で弾かれる。
- freshness / closure は current source を再計算して束縛する。source mismatch は非PASS
  （closure解決失敗 → approve時 E-APPROVAL-001、導出時 draft）。

**W4への宿題（決定4の未適用箇所）**: `crates/vtest-cli/src/lib.rs` の semantic audit bundle 組立ては、
判定subjectに `SpecRecord.sha256`（登録時snapshot）をそのまま使っている。これは決定4が禁じた
false-PASS経路そのものだが、semantic audit bundleはW4所有のためW2では意味を変更していない。
型分離によりこの2箇所は `registered_snapshot()` という明示的な呼び出しとしてgrep可能にし、
コメントでW4の義務を明記した。

---

## 6. Changed files

```text
crates/vtest-store/src/approval.rs   （新規）closure解決・VO status導出・単体5件
crates/vtest-store/src/records.rs    ApprovalDependency / ApprovalRecord.dependencies / VoRecord.status → Option
crates/vtest-store/src/lib.rs        approval module 公開
crates/vtest-cli/src/lib.rs          approve前のclosure解決とE-APPROVAL-001、effective_vo_status委譲、単体1件
crates/vtest-scan/src/lib.rs         W-STORE-001を存在判定へ、W-STORE-002追加、status必須解除、alias単体テスト更新
crates/vtest-model/src/lib.rs        hash_approval_subject / hash_specification_source / SpecSourceHash
tests/ACCEPTANCE.md                  AF-041 / AF-042 とfreeze record
crates/vtest-cli/tests/adapter_acceptance.rs  AF-041 / AF-042 のCLI観測テスト
```

（上記に加え、引き継いだ codex の未コミット W2 分：`Cargo.toml` / `Cargo.lock` /
`crates/vtest-adapter-rust/**` / `vtest-adapter-api` / `vtest-audit` / `vtest-cli/Cargo.toml` /
`vtest-scan/src/operations.rs` / `vtest-store/Cargo.toml` / `vtest-store/src/forms.rs` を同一commitへ含む）

---

## 7. ゲート発効フェーズと deadlock 検証

「W2で解決できるか、できないならどのフェーズから有効なゲートか」を全項目について確定した。
判定基準は **そのゲートを満たすために必要な実装が、所有Waveの内側で完結するか** である。

| # | 項目 | W2で解決可 | ゲート発効 | 所有Waveの内側で充足可能か |
|---|---|---|---|---|
| G-1 | AF-041 / AF-042（VO status writer、Approval closure） | **可** | W2（発効済み・PASS） | — |
| G-2 | AF-027 Approval upstream closure | **可** | W2（発効済み・PASS） | — |
| G-3 | M1 / M2 / M8 acceptance 全PASS | 不可 | **W3完了時** | 可。RED原因は診断の`location.file`→neutral `SourceLocation`改称（W1由来）、config拒否のexit code、保存`status`への依存の3種で、いずれもW3の所有範囲内で更新できる |
| G-4 | M3 / M5 acceptance | 不可 | **W4完了時** | 可 |
| G-5 | M4 / M7 acceptance | 不可 | **W5完了時** | 可 |
| G-6 | M6 / M9 acceptance | 不可 | **W6完了時** | 可 |
| G-7 | AF-035 forbidden dependency | 不可 | **W5完了時** | 可。ただし単一Waveでは不可 — `vtest-scan`(W3)・`vtest-audit`(W4)・`vtest-exec`(W5)の3crateを同時に検査するため、W3単独ではGREENにならない |
| G-8 | 決定4のW4宿題（bundleが登録時snapshotを判定subjectに使用） | 不可 | **W4完了時** | 可。semantic audit bundleはW4所有 |
| G-9 | D-1〜D-3 の正規仕様反映 | 不可（実装工程では閉じない） | **W8着手前** | 可。ただし agent の作業ではなく、独立spec PR と Owner merge が必要（本計画 §15） |
| G-10 | W8 dogfood の「Approval current」 | 可（機構）／要運用 | **W8** | 可。後述 P-1 の前提充足が必要 |

### deadlock 検証

**検出して解消した1件（W2所有）**

`vtest-verify` に承認判定の二重実装 `vo_is_approved` が残っており、`approval.subject_hash` を
VOファイルのraw content hashと比較していた。決定3で `subject_hash` が集約hashになったため、この関数は
**恒久的に false** を返す状態だった。`vo_coverage` は semantic audit がPASSでも承認が無ければ `MISSING` へ
落とすため、**12項目PASSがW8まで到達不能**になる deadlock だった。承認導出を `derive_vo_status` /
`current_approval_subject` の単一実装へ集約して解消した（commit `34382c3`）。

到達可能性の証拠: `specification_source_change_invalidates_the_recorded_closure`（store単体）が
canonical approve 経路で `derive_vo_status(..).approved == true` を先に表明しており、`vtest-verify` は
同じ関数を呼ぶため、承認到達性は構成上保証される。CLI単体
`approval_is_derived_and_edit_makes_it_draft` も spec→req→vo→approve の全経路で `approved` を確認する。

**未実装前提をゲートにしていないことの確認**

- G-3〜G-8 はいずれも「後続Waveが自分の所有範囲で実装すればGREENになる」ものであり、W2の成果物が
  それらの充足を妨げない。G-7 のみ単一Waveでは充足できないため、所有をW3ではなく**W5**へ訂正した。
- E-APPROVAL-001 / closure解決は W4〜W6 の機構（static audit、Evidence、semantic audit）に依存しない。
  SPEC / REQ / VO record と Specification source だけで解決するため、W2の実装だけで承認は成立する。
- `--basis` は任意引数であり、Audit record（W4）が存在しなくても approve は成立する。

### 運用前提（コードのゲートではないが W8 を止めうるもの）

**P-1**: 本リポジトリ自身の `.verify` には stale な SPEC が存在する（`doctor` が
`W-SCAN-104: SPEC SPEC-DOGFOOD-M3 hash is stale` を出力）。決定4により Specification freshness は
current source へ束縛されるため、この SPEC に依存する VO は closure を解決できず approve できない。
これは実装の欠落ではなく登録データの陳腐化であり、`vtest spec add --update` で解消する。
**充足責任の所在**: Owner の指示による運用作業であり、agent の実装工程ではない。W8 の dogfood 開始前に
実施が必要。**W2 では canonical data を変更しない**（本計画 §4 の保護対象であり、
`.codex/hooks/protect_canonical_data.py` の保護対象でもある）。

---

## 7. Next phase allowed

**NO**

W2固有の完了条件は全件成立し、Owner決定1〜4も適用済み。ただしOwnerが承認したのはこの4件の処理であって
Phase 4 の着手ではない。本計画 §1 により Phase 完了は Owner への報告と明示的な続行指示を要する。
Phase 4（W3: Rust discovery / Structured Test Operation）へは着手していない。
push / PR も行っていない。

D-1〜D-3 は正規仕様への反映が必要な差分として未処理のまま残る（独立spec PR経路）。
