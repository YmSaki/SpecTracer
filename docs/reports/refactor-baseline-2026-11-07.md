# リファクタリング基準（#175 R0）：`develop` `99f7252` の実測と比較手順

作成: 2026-11-07（#185。親 #175 §6・§9 R0・§11）

#175 の全子 PR が比較する「変更前」を 1 文書に固定する。コードは触っていない。
#175 §6.1〜§6.2 の値はここが出典である。本文書は調査報告であり、規範ではない（AGENTS.md 3.2）。
代表出力と fixture の所要時間は、同じディレクトリの `refactor-baseline-2026-11-07.capture.sh` で再現する（§6）。

## 1. 基準 SHA と環境

| 項目 | 値 |
|---|---|
| 基準 SHA | `develop` `99f725288ccea07f73d75f002ebeb46639d29c86`（PR #173 のマージ、2026-11-07） |
| OS | Linux x86_64、4 CPU のコンテナ |
| rustc / cargo | 1.97.0（rustc `2d8144b78` 2026-07-07、cargo `c980f4866` 2026-06-30） |
| git | 2.43.0 |
| `cargo-llvm-cov` | 未導入（`cargo llvm-cov` は `no such command`）。CI は導入している（`.github/workflows/ci.yml`） |
| `/usr/bin/time` | 無し。wall time は `date +%s.%N` の差分。最大 RSS は未計測 |
| CI（llvm-cov あり） | `99f7252` の push で成功（run 414、<https://github.com/YmSaki/SpecTracer/actions/runs/42406300827>） |

計測後に `develop` は `fd22549` へ進んだ（#183、#184。`crates/vtest-adapter-lexical/src/swift.rs`、`crates/vtest-cli/tests/mcp_cli_equivalence.rs`、`docs/adapters/swift.md` の 3 ファイル）。基準 SHA は `99f7252` のままとする。#175 §8 の行番号はこの 3 ファイルを参照しないので影響しない。§6.2 の規模では `swift.rs`（1,092 行）と CLI 統合テスト（12,369 行）がそれぞれ 63 行と 70 行増えるだけで、他は変わらない。

## 2. ゲートと既知失敗

| ゲート | 結果 |
|---|---|
| `cargo fmt --all -- --check` | 成功 |
| `cargo clippy --workspace --all-targets --locked -- -D warnings` | 成功 |
| `cargo test --workspace --locked --no-fail-fast` | 713 passed / 9 failed / 4 ignored。失敗 target は 3（§2.1） |
| `cargo run --quiet -p vtest-cli --locked -- doctor` | exit 0。W-SCAN-101（`@vtest` 注釈のない test fn）11 件 |
| CI `spec-guard` | `docs/canonical/`・`.verify/doc/` を変えない PR では対象外 |

`--no-fail-fast` を付けないと最初に失敗した test binary で止まり、後続の結果が記録されない。

### 2.1 失敗 9 件の分類

分類は「環境差」「実装不具合」「未分類」の 3 つ。9 件とも環境差で、実装不具合と未分類は 0。

| # | target | テスト | 症状 | 分類 |
|---|---|---|---|---|
| 1 | `vtest-adapter-rust --lib` | `coverage::tests::real_llvm_cov_keeps_same_line_constructs_and_type_syntax_apart` | `coverage.rs:595` で panic。`error: no such command: llvm-cov` | 環境差 |
| 2 | `vtest-adapter-rust --lib` | `coverage::tests::real_llvm_cov_measures_trait_impl_and_cfg_locators_separately` | 同上 | 環境差 |
| 3 | `evidence_e2e` | `a_covered_test_with_a_git_committed_environment_reaches_ok_true` | `ok: false`。`target_binding` が NO_EVIDENCE | 環境差 |
| 4 | `evidence_e2e` | `a_test_that_reaches_only_one_of_two_declared_targets_is_not_pass` | `assertion failed: coverage.checked` | 環境差 |
| 5 | `evidence_e2e` | `an_invalid_latest_evidence_record_does_not_fall_back_to_an_older_pass` | control が `NoEvidence`（期待 `Pass`） | 環境差 |
| 6 | `evidence_e2e` | `verify_reports_e_scan_010_for_an_aggregate_that_contradicts_its_entries` | target coverage が `None`（期待 `Some(Fail)`） | 環境差 |
| 7 | `evidence_e2e` | `only_oracle_presence_breaks_on_a_self_comparison_assertion` | `NoEvidence`（期待 `Pass`） | 環境差 |
| 8 | `execution_state_e2e` | `a_relocated_target_directory_inside_the_root_still_records_evidence` | `ok: false`。`target_binding` が NO_EVIDENCE | 環境差 |
| 9 | `execution_state_e2e` | `an_unchanged_environment_still_reaches_pass` | 同上 | 環境差 |

根拠：
- 1〜2 は実際の `cargo llvm-cov` を呼ぶテストで、コマンドが無いことが panic の文面に出ている。
- 3〜9 は `rust-cargo` adapter が coverage を取れないため `target_binding` が PASS に届かず、PASS を期待する assertion で落ちる。失敗の dump はいずれも `target_binding` だけが NO_EVIDENCE で、他 3 検査は PASS。
- 同じ SHA の CI（llvm-cov あり）は test job を含めて成功している（§1 の run 414）。

llvm-cov の無い環境で子 PR のゲートを回すときは、この 9 件を NOT_EXECUTED と書き、CI の結果を証拠にする（#175 §11）。

### 2.2 レビュー環境でだけ失敗した `execution_state_adapter` 2 件

本環境では 28 件すべて成功した（`3fb1c03`、`99f7252` とも）。失敗した環境の条件（OS、cargo、llvm-cov の有無、テストの並列度、失敗した 2 件の名前）は Owner に確認中で、判明したら本節に追記する。それまでの分類は「未分類（本環境では再現せず）」。

## 3. 性能

### 3.1 リポジトリ自身（wall time、各 3 回）

release build は `cargo build --release --locked -p vtest-cli`。`target/release` が無い状態から 60.8 s。

| コマンド | debug | release | 結果 |
|---|---|---|---|
| `vtest scan --format json` | 26.1〜27.8 s | 5.7〜6.4 s | exit 0。files 104 / tests 715 / sources 2,723 / discovered 726 |
| `vtest doctor --format json` | 25.8〜26.7 s | 5.3〜5.8 s | exit 0。diagnostics 11 |
| `vtest verify --summary --format json` | 30.3〜31.9 s | 6.6〜6.9 s | exit 1。result NG、state MISMATCH、non_pass 1,150、pending 34 |

以後の性能比較は release build で行う（debug は §4 の低速化を含むため）。許容差は数 PR 分の値が揃ってから用途で決める（#175 §4 規約 4）。

### 3.2 fixture（release、各 3 回）

§5.1 の fixture。入力が小さく、値はプロセス起動時間に近い。性能の比較はリポジトリ自身で行い、fixture は出力の比較に使う。

| fixture | `scan` | `doctor` | `verify --summary` |
|---|---|---|---|
| rust（rust-cargo、Test 2） | 4〜6 ms | 4〜6 ms | 7〜9 ms（exit 1） |
| python（lexical、Test 1） | 4〜5 ms | 4〜5 ms | 5〜7 ms（exit 1） |

### 3.3 未計測

- 最大 RSS（`/usr/bin/time` 無し）。使える環境で `/usr/bin/time -v` を足す。
- `3fb1c03` の release 値（debug のみ。#202）。
- synthetic / mixed fixture の CLI 実行（§5.4 の理由で不可）。

## 4. debug build の低速化（#202）

`3fb1c03` → `99f7252` で debug の scan が 6.7〜7.0 s → 26.1〜27.8 s、doctor が 6.6〜7.3 s → 25.8〜26.7 s、verify --summary が 9.3〜9.8 s → 30.3〜31.9 s になった。入力は files 60 → 104、sources 1,710 → 2,723 で、adapter は `rust-cargo` 1 件のまま。原因は未調査で、#202 で追う。#175 の refactor PR には混ぜない。

## 5. 代表出力

### 5.1 入力

| 入力 | 内容 | verify の状態 |
|---|---|---|
| リポジトリ自身 | `.verify/config.yaml`（adapter `rust-cargo` 1 件） | NG / MISMATCH、non_pass 1,150、pending 34 |
| fixture `rust` | `rust-cargo`。crate 1 つ、target 2（`add` / `sub`）、Test 2（`TEST-BASELINE-SUB` は oracle 無し）、VO 1、文書 ROOT 1 / R 1 | NG / FAIL、non_pass 3 |
| fixture `python` | lexical adapter `python`。Test 1、VO 1、文書は同上 | NG / NO_EVIDENCE、non_pass 2 |
| fixture `unknown-adapter` | config の adapter id が `nope` | （scan で E-CONFIG-001） |
| fixture `unreadable-config` | `.verify/config.yaml` がディレクトリ | （scan で E-INTERNAL-001） |
| `tests/fixtures/calc` | 旧 12 項目の `full_scope` を持つ既存 fixture | （scan で E-CONFIG-001、DS-1108） |

fixture はスクリプトが `vtest init` / `doc add` / `vo add` で毎回生成する（リポジトリには入れない）。

### 5.2 保存と正規化

- 生の出力はリポジトリに入れない。スクリプトが `outputs/`（stdout と stderr）と `normalized/` に再生成する。本文書には正規化後の sha256 先頭 16 桁と終了コードだけを置く。
- 正規化：ISO 8601 の時刻 → `<TIMESTAMP>`、26 文字の ULID（Evidence ID、record ID）→ `<ULID>`、fixture・出力先・リポジトリの絶対パス → `<FX>` / `<OUT>` / `<REPO>`。
- 同じバイナリで 2 回実行し、34 件すべてのハッシュと終了コードが一致した。stderr は 34 件すべて空。

### 5.3 一覧（`99f7252`、release build）

| 名前 | 入力 | コマンド | exit | sha256 先頭 16 桁 |
|---|---|---|---|---|
| `repo-scan.json` | リポジトリ自身 | `scan` `--format json` | 0 | `3fa134ba29f0bda8` |
| `repo-scan.text` | リポジトリ自身 | `scan` `--format text` | 0 | `4cd9c4ba0fd24d0d` |
| `repo-doctor.json` | リポジトリ自身 | `doctor` `--format json` | 0 | `509766b72318fb67` |
| `repo-doctor.text` | リポジトリ自身 | `doctor` `--format text` | 0 | `402dca0fd1e83a41` |
| `repo-verify-summary.json` | リポジトリ自身 | `verify --summary` `--format json` | 1 | `0453e3b43349409a` |
| `repo-verify-summary.text` | リポジトリ自身 | `verify --summary` `--format text` | 1 | `8f0748e402471cc3` |
| `repo-verify.json` | リポジトリ自身 | `verify` `--format json` | 1 | `eb35f8fa275ff96e` |
| `repo-verify.text` | リポジトリ自身 | `verify` `--format text` | 1 | `f18ad5c4f96ad658` |
| `repo-mcp-tools-list.json` | リポジトリ自身 | MCP `initialize` → `tools/list` | 0 | `fabf510d1f496370` |
| `fx-rust-scan.json` | fixture `rust` | `scan` `--format json` | 0 | `0bf10d2b3af8822b` |
| `fx-rust-scan.text` | fixture `rust` | `scan` `--format text` | 0 | `dd7d899b87fa17da` |
| `fx-rust-doctor.json` | fixture `rust` | `doctor` `--format json` | 0 | `e7fd0b8350712357` |
| `fx-rust-doctor.text` | fixture `rust` | `doctor` `--format text` | 0 | `9d854c9dff777007` |
| `fx-rust-verify-summary.json` | fixture `rust` | `verify --summary` `--format json` | 1 | `cc35ea54e9402562` |
| `fx-rust-verify-summary.text` | fixture `rust` | `verify --summary` `--format text` | 1 | `47dd61e51a89e5a3` |
| `fx-rust-verify.json` | fixture `rust` | `verify` `--format json` | 1 | `51148b069540705a` |
| `fx-rust-verify.text` | fixture `rust` | `verify` `--format text` | 1 | `994c8c60aa43e125` |
| `fx-rust-mcp-scan.json` | fixture `rust` | MCP `tools/call scan` | 0 | `d72c75a8b7fb0220` |
| `fx-rust-mcp-verify.json` | fixture `rust` | MCP `tools/call verify` | 0 | `91ebfb5b8937a709` |
| `fx-python-scan.json` | fixture `python` | `scan` `--format json` | 0 | `9ca6fb38c96c7862` |
| `fx-python-scan.text` | fixture `python` | `scan` `--format text` | 0 | `98093f8fdcc18358` |
| `fx-python-doctor.json` | fixture `python` | `doctor` `--format json` | 0 | `31a9888416ce8287` |
| `fx-python-doctor.text` | fixture `python` | `doctor` `--format text` | 0 | `9d854c9dff777007` |
| `fx-python-verify-summary.json` | fixture `python` | `verify --summary` `--format json` | 1 | `f3d5d627cb95df31` |
| `fx-python-verify-summary.text` | fixture `python` | `verify --summary` `--format text` | 1 | `e34645aae2cf6e51` |
| `fx-python-verify.json` | fixture `python` | `verify` `--format json` | 1 | `f319b8d438a09fec` |
| `fx-python-verify.text` | fixture `python` | `verify` `--format text` | 1 | `aa575a387a18b670` |
| `fx-python-mcp-scan.json` | fixture `python` | MCP `tools/call scan` | 0 | `2ed1febcc28d2fec` |
| `fx-python-mcp-verify.json` | fixture `python` | MCP `tools/call verify` | 0 | `e96a20fce093f96d` |
| `fail-config-old-full-scope.json` | `tests/fixtures/calc` | `scan --format json`（E-CONFIG-001：`full_scope` に `spec_coverage`） | 2 | `30227c53ba973416` |
| `fail-unknown-adapter.json` | fixture `unknown-adapter` | `scan --format json`（E-CONFIG-001：id `nope` 未登録） | 2 | `c095f2620e27b196` |
| `fail-unknown-test-id.json` | fixture `rust` | `run --test TEST-NOPE --format json`（E-OP-001） | 2 | `2aaf8f452ea6dc9a` |
| `fail-unreadable-config.json` | fixture `unreadable-config` | `scan --format json`（E-INTERNAL-001） | 3 | `f05ea8e85d27a179` |
| `fail-mcp-unknown-test-id.json` | fixture `rust` | MCP `tools/call run_tests` `{"test":["TEST-NOPE"]}`（E-OP-001、`isError: true`） | 0 | `d5ce300d4bc9a271` |

MCP の失敗は envelope（`isError: true`、`ok: false`、E-OP-001）で返り、プロセスの終了コードは 0。CLI と同じ `data` / `diagnostics` を持つ（DS-1563）。

### 5.4 対象外とその理由

| 対象 | 理由 | 回帰証拠 |
|---|---|---|
| synthetic / mixed fixture、E-ADAPTER-002 | `synthetic` adapter は CLI の builtin registry に無く、e2e テストが in-process で登録する | TEST-CLI-SYNTHETIC-ADAPTER-SCAN-AND-VERIFY-WITHOUT-CORE-CHANGES、TEST-CLI-MIXED-RUST-CARGO-SYNTHETIC-SCAN-MERGE、TEST-MATRIX-DISCOVERY-FAILURE-EXIT-TWO |
| E-ADAPTER-001 | registry 側の重複登録で、CLI の config からは到達しない。config の未知 id は E-CONFIG-001（DS-1663。`fail-unknown-adapter.json`） | TEST-ADAPTER-API-REGISTRY-REJECTS-DUPLICATE-ID、TEST-SCAN-UNKNOWN-ADAPTER-ID-REJECTED |
| MCP `run_tests`（fast）、`doc_*`、`approval_*`、`tests/fixtures/adapters/api-contract` | 本手順では未収録 | R3（#188）の 21 tool 同値テストと既存の受入テスト。必要になった PR が本スクリプトに足す |

## 6. 再実行と比較の手順

前提：bash、python3、sed、sha256sum、bc、git。ネットワーク不要。

1. base と head の両方を build する。
   ```sh
   git worktree add ../st-base <base-sha> && (cd ../st-base && cargo build --release --locked -p vtest-cli)
   cargo build --release --locked -p vtest-cli      # head（作業中の checkout）
   ```
2. 同じ checkout（head）を入力にして、両方のバイナリで取る。入力を同じにするのは、バイナリの差だけを見るため。
   ```sh
   docs/reports/refactor-baseline-2026-11-07.capture.sh ../st-base/target/release/vtest /tmp/cap-base .
   docs/reports/refactor-baseline-2026-11-07.capture.sh ./target/release/vtest        /tmp/cap-head .
   ```
3. 比較する。差分ゼロが原則。
   ```sh
   diff /tmp/cap-base/exit-codes.txt /tmp/cap-head/exit-codes.txt
   diff /tmp/cap-base/hashes.txt     /tmp/cap-head/hashes.txt
   diff -r /tmp/cap-base/normalized  /tmp/cap-head/normalized
   ```
4. 差分がある行は、根拠の条項 ID と一緒に PR 本文に書く（#175 §11）。`repo-*` はリポジトリ自身を読むので、ファイルを移す PR では head の checkout を入力にした値が base のバイナリでも変わる。その差はバイナリの差ではない。fixture（`fx-*`、`fail-*`）の差だけをバイナリの差として扱う。
5. 本文書の §5.3 と比べるときは、`99f7252` の checkout を入力にする（`git worktree add ../st-99f7252 99f7252` の上で手順 2 を実行）。
6. 性能はリポジトリ自身で release、各 3 回。
   ```sh
   for c in "scan --format json" "doctor --format json" "verify --summary --format json"; do
     for i in 1 2 3; do t0=$(date +%s.%N); ./target/release/vtest $c > /dev/null; t1=$(date +%s.%N); echo "$c run$i $(echo "$t1 - $t0" | bc) s"; done
   done
   ```
7. ゲートは `cargo fmt --all -- --check`、`cargo test --workspace --locked --no-fail-fast`、`cargo clippy --workspace --all-targets --locked -- -D warnings`、`cargo run --quiet -p vtest-cli --locked -- doctor`。llvm-cov が無ければ §2.1 の 9 件を NOT_EXECUTED と書く。

## 7. 残課題

- §2.2 の 2 件：失敗環境の条件を Owner に確認して追記する。
- 最大 RSS：`/usr/bin/time -v` がある環境で足す。
- 許容差（性能）：数 PR 分の値が揃ったら決める。
- #202：debug build の低速化。
- §5.4 の未収録項目：触る PR が足す。
