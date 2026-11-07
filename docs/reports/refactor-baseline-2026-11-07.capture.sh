#!/usr/bin/env bash
# docs/reports/refactor-baseline-2026-11-07.md（#175 R0、#185）の「代表出力」と「fixture の所要時間」を再現する。
#
# 使い方:
#   docs/reports/refactor-baseline-2026-11-07.capture.sh <vtest バイナリ> <出力先> [リポジトリ root]
#     リポジトリ root を省くと、このスクリプトを含む git リポジトリの root を使う。
#
# 出力先に書くもの:
#   fixtures/            スクリプトが生成する隔離プロジェクト（rust-cargo、python、失敗経路 2 つ）
#   outputs/             生の stdout（<名前>）と stderr（<名前>.stderr）
#   normalized/          時刻・ULID・パスをマスクした stdout
#   exit-codes.txt       <名前> exit=<終了コード>
#   hashes.txt           normalized/ の sha256 先頭 16 桁と <名前>
#   timing-fixtures.txt  fixture での scan / doctor / verify --summary の wall time（各 3 回）
#
# 比較（base と head のバイナリで別々の出力先に実行してから）:
#   diff a/exit-codes.txt b/exit-codes.txt
#   diff a/hashes.txt b/hashes.txt
#   diff -r a/normalized b/normalized
#
# 依存: bash、python3、sed、sha256sum、bc、git。ネットワークは使わない。リポジトリの .verify/ には書かない。
set -u
if [ $# -lt 2 ]; then echo "usage: $0 <vtest-binary> <out-dir> [repo-root]" >&2; exit 64; fi
BIN=$(readlink -f "$1")
mkdir -p "$2"; OUT=$(readlink -f "$2")
REPO=$(readlink -f "${3:-$(git -C "$(dirname "$0")" rev-parse --show-toplevel)}")
FX="$OUT/fixtures"
rm -rf "$OUT/outputs" "$OUT/normalized" "$FX"
mkdir -p "$OUT/outputs" "$OUT/normalized" "$FX"
: > "$OUT/exit-codes.txt"; : > "$OUT/timing-fixtures.txt"; : > "$OUT/hashes.txt"

# 実行ごとに変わる値をマスクする: ISO 8601 の時刻、ULID（Evidence ID など）、fixture / 出力先 / リポジトリの絶対パス。
normalize() {
  sed -E -e 's/[0-9]{4}-[0-9]{2}-[0-9]{2}T[0-9]{2}:[0-9]{2}:[0-9]{2}(\.[0-9]+)?(Z|[+-][0-9]{2}:[0-9]{2})/<TIMESTAMP>/g' \
         -e 's/[0-9A-HJKMNP-TV-Z]{26}/<ULID>/g' \
         -e "s#$FX#<FX>#g" -e "s#$OUT#<OUT>#g" -e "s#$REPO#<REPO>#g"
}
# run <名前> <作業ディレクトリ> <vtest の引数...>
run() {
  local name=$1 cwd=$2; shift 2
  ( cd "$cwd" && "$BIN" "$@" ) > "$OUT/outputs/$name" 2> "$OUT/outputs/$name.stderr"
  echo "$name exit=$?" >> "$OUT/exit-codes.txt"
}
# mcp <名前> <project root> <JSON-RPC の行...>
mcp() {
  local name=$1 root=$2; shift 2
  printf '%s\n' "$@" | "$BIN" --project "$root" mcp > "$OUT/outputs/$name" 2> "$OUT/outputs/$name.stderr"
  echo "$name exit=$?" >> "$OUT/exit-codes.txt"
}

# ---- fixture ---------------------------------------------------------------
make_doc() { cat > "$1" <<'JSON'
{"schema_version":"0.1",
 "root":[{"id":"ROOT-001","statement":"fixture root","source":{"doc":"root.md","heading":"fixture","lines":[1,1]}}],
 "request":[{"id":"R-001","statement":"fixture requirement","derives_from":["ROOT-001"],"source":{"doc":"req.md","heading":"fixture","lines":[1,1]}}],
 "require":[],"spec":[],"detailed_spec":[],"basic_design":[],"design":[]}
JSON
}
# init_fixture <dir> <adapter id> <scan.include の項目...>
init_fixture() {
  local dir=$1 adapter=$2; shift 2
  mkdir -p "$dir"
  "$BIN" init --project "$dir" --name baseline-fixture --quiet
  {
    printf 'version: 2\nproject:\n  name: baseline-fixture\nadapters:\n  - id: %s\n    roots:\n      - %s\n    scan:\n      include:\n' "$adapter" "'.'"
    for inc in "$@"; do printf '        - %s\n' "$inc"; done
    printf '      assertion_macros: []\n    run:\n      coverage: none\nverify:\n  full_scope:\n    - chain_integrity\n    - orphan_detection\n    - target_binding\n    - oracle_presence\n'
  } > "$dir/.verify/config.yaml"
  make_doc "$dir/fixture-doc.json"
  "$BIN" doc add --project "$dir" --id fixture --path fixture-doc.json --format json > /dev/null
  "$BIN" vo --project "$dir" --quiet add --id VO-FIXTURE-ONE --claim "fixture claim" --derives-from R-001
}

# rust-cargo: crate 1 つ、target 2 つ、Test 2 つ（1 つは oracle を持たない）
init_fixture "$FX/rust" rust-cargo src tests
mkdir -p "$FX/rust/src" "$FX/rust/tests"
printf '[package]\nname = "baseline"\nversion = "0.1.0"\nedition = "2021"\n\n[workspace]\n' > "$FX/rust/Cargo.toml"
printf 'pub fn add(a: i32, b: i32) -> i32 { a + b }\npub fn sub(a: i32, b: i32) -> i32 { a - b }\n' > "$FX/rust/src/lib.rs"
cat > "$FX/rust/tests/calc.rs" <<'RS'
/// @vtest.id TEST-BASELINE-ADD
/// @vtest.covers VO-FIXTURE-ONE
/// @vtest.target src/lib.rs::add
/// @vtest.intent adding two numbers returns their sum
#[test]
fn add_works() { assert_eq!(baseline::add(1, 2), 3); }

/// @vtest.id TEST-BASELINE-SUB
/// @vtest.covers VO-FIXTURE-ONE
/// @vtest.target src/lib.rs::sub
/// @vtest.intent subtracting without an oracle
#[test]
fn sub_runs() { let _ = baseline::sub(3, 1); }
RS

# python: 字句 adapter 1 つ、Test 1 つ
init_fixture "$FX/python" python src
mkdir -p "$FX/python/src"
cat > "$FX/python/src/calc.py" <<'PY'
def add(a, b):
    return a + b


# @vtest.id TEST-BASELINE-PY-ADD
# @vtest.covers VO-FIXTURE-ONE
# @vtest.target src/calc.py::add
# @vtest.intent adding two numbers returns their sum
def test_add():
    assert add(1, 2) == 3
PY

# 失敗経路: 未登録の adapter id、読めない config（config.yaml がディレクトリ）
init_fixture "$FX/unknown-adapter" nope src
init_fixture "$FX/unreadable-config" rust-cargo src
rm "$FX/unreadable-config/.verify/config.yaml"; mkdir "$FX/unreadable-config/.verify/config.yaml"

# ---- 代表出力: リポジトリ自身 ----------------------------------------------
for fmt in json text; do
  run "repo-scan.$fmt"            "$REPO" scan --format $fmt
  run "repo-doctor.$fmt"          "$REPO" doctor --format $fmt
  run "repo-verify-summary.$fmt"  "$REPO" verify --summary --format $fmt
  run "repo-verify.$fmt"          "$REPO" verify --format $fmt
done
mcp "repo-mcp-tools-list.json" "$REPO" \
  '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}' \
  '{"jsonrpc":"2.0","id":2,"method":"tools/list","params":{}}'

# ---- 代表出力: fixture --------------------------------------------------------
for fx in rust python; do
  for fmt in json text; do
    run "fx-$fx-scan.$fmt"           "$FX/$fx" scan --format $fmt
    run "fx-$fx-doctor.$fmt"         "$FX/$fx" doctor --format $fmt
    run "fx-$fx-verify-summary.$fmt" "$FX/$fx" verify --summary --format $fmt
    run "fx-$fx-verify.$fmt"         "$FX/$fx" verify --format $fmt
  done
  mcp "fx-$fx-mcp-scan.json" "$FX/$fx" \
    '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}' \
    '{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"scan","arguments":{}}}'
  mcp "fx-$fx-mcp-verify.json" "$FX/$fx" \
    '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}' \
    '{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"verify","arguments":{}}}'
done

# ---- 失敗経路 -------------------------------------------------------------------
run "fail-config-old-full-scope.json" "$REPO/tests/fixtures/calc" scan --format json        # E-CONFIG-001（旧 full_scope）、exit 2
run "fail-unknown-adapter.json"       "$FX/unknown-adapter"       scan --format json        # E-CONFIG-001（未登録 id）、exit 2
run "fail-unknown-test-id.json"       "$FX/rust"                  run --test TEST-NOPE --format json   # E-OP-001、exit 2
run "fail-unreadable-config.json"     "$FX/unreadable-config"     scan --format json        # E-INTERNAL-001、exit 3
mcp "fail-mcp-unknown-test-id.json" "$FX/rust" \
  '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}' \
  '{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"run_tests","arguments":{"test":["TEST-NOPE"]}}}'

# ---- 正規化とハッシュ -----------------------------------------------------------
for f in "$OUT"/outputs/*; do
  case "$f" in *.stderr) continue;; esac
  name=$(basename "$f")
  normalize < "$f" > "$OUT/normalized/$name"
  printf '%s  %s\n' "$(sha256sum < "$OUT/normalized/$name" | cut -c1-16)" "$name" >> "$OUT/hashes.txt"
done

# ---- fixture の所要時間（各 3 回） ----------------------------------------------------
for fx in rust python; do
  for c in "scan --format json" "doctor --format json" "verify --summary --format json"; do
    for i in 1 2 3; do
      t0=$(date +%s.%N); ( cd "$FX/$fx" && "$BIN" $c > /dev/null 2>&1 ); ec=$?; t1=$(date +%s.%N)
      printf '%s %s run%d %.3f s exit=%d\n' "$fx" "${c%% *}" "$i" "$(echo "$t1 - $t0" | bc)" "$ec" >> "$OUT/timing-fixtures.txt"
    done
  done
done
echo "done: $OUT"
