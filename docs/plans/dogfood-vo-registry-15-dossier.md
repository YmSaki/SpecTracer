# VO-REGISTRY-15 — NEEDS-SPEC-JUDGMENT

## 8点鎖

1. **VO claim**: 「A declaration key the adapter's field contract does not admit — unrecognized inside `@vtest.` (E-SCAN-006) or a repeat of a non-repeatable key (E-SCAN-005) — is diagnosed at error severity rather than downgraded to a warning or ignored as free text, leaving the affected entity's check items non-PASS.」refs = 詳細設計 §4.2/§4.4/§5.4, 基本仕様 §16。
2. **Normative source = THE FORK**（下記「二読」）。§4.2 L513 が節の対象を「テスト関数直前の doc comment」に限定する一方、§5.4 の E-SCAN-005/006 行は「adapter所有の宣言」とだけ書き、§4.2 L526 は `@vtest.src-id` を非テスト関数に置く。両読みとも文面で支持される。
3. **Design mechanism**: §4.2 の key 集合（L517-518, `src-id` を含む）＋ L524「`@vtest.` で始まるが未知のキーを持つ行はエラー E-SCAN-006（打鍵ミスの検出を優先し、警告ではなくエラーとする）」。§5.5 step 5「doc属性を§4.2の文法でparseする」/ step 6「すべてのfn / impl fnを…`@vtest.src-id`認識に使用する」。
4. **Concrete condition**: production fn に `/// @vtest.src_id SRC-X`（underscore の打鍵ミス。admitted は `src-id`）。
5. **Implementation path**: `crates/vtest-adapter-rust/src/discovery.rs`
   - :116-118 `KNOWN` に `"src-id"`、:131-137 未知キー→ `__unknown_key__` sentinel、:140-143 重複→ `__duplicate_key__`
   - :947-956 SourceTargetDraft を push（`src_id: parse_src_id(attrs)`）
   - **:958-960 `if !is_test_function(attrs) { return; }`**
   - :971-982（E-SCAN-005/006 発火）と :1032 は**すべてこの return の後**。リポジトリ全体で発火点はこの3箇所のみ。
   - :167-171 `parse_src_id` は `values.get("src-id")` だけを読み、`__parse_error__` を捨てる。
6. **Expected**（broad reading）: E-SCAN-006 error。
7. **Actual**: 診断ゼロ・`ok:true`・exit 0、`src_id: None` で Source Target 登録。
8. **System-level**: 後段 gate なし。`doctor` は `scan` の alias（同一出力・exit 0）。`verify` は `crates/vtest-verify/src/lib.rs:584-589` で「scan に error 診断が1件でもあれば vo_decomposition=FAIL」と畳むが、**この経路は error が存在しないので発火しない**。唯一の観測経路は「誰かが `SRC-X` を参照した時」で、その場合 E-SCAN-004 が**打鍵ミスした production 関数ではなく test の locus**で出る（case3）。未参照なら完全な無音。

## 二読（どちらも normative text で支持される）

- **Narrow（test-scoped）**: §4.2 L513「テスト関数直前の doc comment（`///` または `/** */`）内の行を対象とする。」／§4.4 は全体が Test construct 前提（「該当Test constructをDiscovered Testとして返し」）／基本仕様 §16「adapterごとの**Test metadata**宣言構文とパースエラーの扱い」／基本仕様 §3.3「恒久IDは必須ではなく、**指定された場合だけ**adapterが認識する」。→ 文法は全関数で parse されるが、**診断義務**は Test 処理に付く。現実装はこの読みに一致。
- **Broad（declaration-scoped）**: §5.4「E-SCAN-006 | error | adapter所有の宣言に未知fieldが存在」（test 限定語なし。E-SCAN-005 も同様。対して E-SCAN-007 は自身の行で「必須metadata（id / covers / targets / intent）の欠落」と Test 固有に書き分けられている）／§4.2 L518 が `src-id` を key 集合に含め、L526「`@vtest.src-id` はテストではなく対象実装側の関数に付与し」＋「付与・変更・削除は Source Target hash を変化させる」— strict narrow だと L526 が到達不能になる／L524 の rationale が「打鍵ミスの検出」そのもの。

補強（narrow 寄り）: claim 末尾「leaving the affected entity's check items non-PASS」は、未参照の production 関数には locus を持たない（Source Target は check item を持たない）。claim の書き手は test surface を想定していた可能性が高い。

## Repro（隔離 temp、リポジトリには一切実行せず）

`cargo build -p vtest-cli`（repo、target/ のみ書込）→ `mktemp -d` 配下に `tests/fixtures/calc/m1/base` を `target/` 除外でコピー、cwd を temp 内に固定。fixture は Git Bash heredoc（LF）。

```
===== case1 CONTROL  test fn + `/// @vtest.src_id SRC-X` =====
NG  summary: {"files":2,"sources":2,"tests":0}
error E-SCAN-006: unknown @vtest key `src_id`      exit=1
  scan --format json: {"code":"E-SCAN-006","severity":"error", location tests/registered.rs}
===== case2 COUNTEREXAMPLE  production fn + 同一 typo key =====
OK  summary: {"files":2,"sources":2,"tests":1}     exit=0
  diagnostics: []      src/lib.rs::known  src_id= None
  verify: vo_decomposition PASS / test_existence PASS（typo は一切現れない）
  doctor: OK exit=0
===== case3 typo'd src_id + test が `@vtest.target SRC-X` を参照 =====
NG  error E-SCAN-004: test `TEST-M1-CLEAN` target cannot be resolved   exit=1（locus は test）
===== case3b POSITIVE CONTROL  正しい `@vtest.src-id SRC-X` + 同参照 =====
OK  exit=0（解決成功）
===== case4 unknown key と duplicate key を同時に持つ test =====
NG  error E-SCAN-006: unknown @vtest key `typo`   ← E-SCAN-005 は出ない  exit=1
===== case5 typo test + 別の clean test が同じ VO を covers =====
NG  E-SCAN-006 / verify: test_existence PASS, vo_decomposition FAIL, VO FAIL
```

## Secondary verdicts

- **(a) else-if swallowing — CONFIRMED**。discovery.rs:149-153 が `if unknown … else if duplicate`。case4 で unknown/duplicate 同居宣言から E-SCAN-006 のみ。E-SCAN-005 は恒久的に握り潰される（sentinel が単一 slot なので、同種複数も1件目のみ）。
- **(b) 「この family に非PASS check-item の locus がない」— REFUTED（ただし別欠陥あり）**。locus は存在する: `vtest-verify/src/lib.rs:584-589` が `scan.diagnostics.iter().any(Diagnostic::is_error)` で**どの entity の error かを問わず**全 VO の `vo_decomposition` を FAIL にする（case1/3/4/5 で発火）。ただしこれは §5.4「error は**該当エンティティに関わる**チェック項目を非PASSにする」の entity 帰属を持たない blanket fold であり、別個の指摘に値する。offending Test が early return で消える事実自体は CONFIRMED（case1 で `tests:0`、verify に `test:` 行が出ない）。production 関数 surface では error が存在しないので (b) の locus も当然発火しない。
