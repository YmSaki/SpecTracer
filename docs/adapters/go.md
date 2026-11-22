# Go adapter (`go`)

Discovers Tests and Source Targets in `.go` files, runs the Tests with `go
test`, measures with `go test`'s own coverage whether a Test ran the
targets it declares, and reads the text of a Test to find whether it checks
the result of those targets (the static audit). See [lexical.md](lexical.md)
for the `@vtest.` line grammar it shares with the other framework-neutral
adapters. Unlike them, `go` finds a Test by its shape, not by the comment above
it, so a Test function that has no `@vtest.` declaration is still observed.

- **Files read**: every `.go` file under the adapter's `include` paths whose
  name does not start with `_` or `.`, except the files inside a directory
  named `testdata` or `vendor` or whose name starts with `_` or `.`. Files
  ignored by `.gitignore` are not read. Build constraints (`//go:build`,
  `// +build`, and `_linux` or `_amd64` in a file name) are not evaluated, so a
  file that a constraint would exclude from a build is still read.
- **Tests**: a function declared at the top level of a file whose name ends in
  `_test.go`, without a receiver, named `Test` or `Test` followed by a
  character that is not a lowercase letter (`TestAdd`, `Test_add`, `Test1`).
  Its type arguments, parameters, and results are not checked, with one
  exception: `TestMain` is a Test only when its one parameter is `*testing.T`
  (otherwise it is the main of the test binary). Nothing else is a Test: not a
  method (so not a testify suite method), not a Ginkgo `It(...)` call or a
  `t.Run` subtest, and not a `Benchmark`, `Fuzz`, or `Example` function. The
  Test is named by its function name (`TestAdd`), which is its Test locator.
- **Declaring a Test**: write the `@vtest.` lines in the function's doc
  comment, the comment block that ends on the line above the function (a blank
  line detaches it). Both `//` lines and a `/* */` block are read; in a block,
  one `*` at the start of a line is skipped. The Test's bytes are the function
  from `func` to the end of its body, without the doc comment.
- **An undeclared Test**: a Test function with no `@vtest.` line is returned
  and reported as `W-SCAN-101`. It has no managed Test, so `chain_integrity`
  stays `MISMATCH` (`MISSING`) until it is declared.
- **Source Targets**: every top-level `func` declaration with a body, methods
  included. A method is qualified by its receiver type, and type parameters
  are not part of the name: `calc/calc.go::Add` and `calc/calc.go::Calc::Add`
  for `func (c *Calc) Add(...)`. Function literals and declarations without a
  body (implemented in assembly) are not Source Targets. A Test function is
  also a Source Target. A Source Target's bytes start at its doc comment, so
  editing the comment changes its hash. Two functions with the same locator
  (several `init` functions or `_` functions in one file) are both Source
  Targets, and a `target` that names them is ambiguous (`E-SCAN-004`).
- **Declaring a permanent SRC ID**: put `@vtest.src-id SRC-...` in the doc
  comment of a function that is not a Test. On a Test function a `src-id`
  line is `E-SCAN-006`.
- **A declaration on a method**: a method is never a Test. The `@vtest.` lines
  in its doc comment are read as a Source Target declaration, so a test key
  such as `id` is `W-SCAN-105` and no Test is made, even when the method is
  named `Test...` in a `_test.go` file. A `covers` that only that method would
  have satisfied stays unlinked, so `chain_integrity` reports it `MISMATCH`.
- **Where a construct ends**: where Go itself ends a statement: at a `;`,
  or at a newline after an identifier, literal, `)`, `]`, `}`, `++`, `--`,
  or `return`/`break`/`continue`/`fallthrough`. A function's body is its
  last `{}` group, so the braces of a `struct` or `interface` result type
  (`func f() struct{ X int } {`) are not the body.
- **Unreadable files**: a file the adapter cannot read with confidence (not
  UTF-8 text, an unterminated string or comment, unbalanced brackets, a `func`
  without a name) is `E-SCAN-001` and contributes nothing, not even the
  declarations before the fault. A declared Test function without a body has
  no determinable construct, so it is reported as Missing with `E-SCAN-001`,
  not as a Test; an undeclared one is `W-SCAN-101` like any other.
- **Comments that are not read**: a `@vtest.` line in a comment inside a
  function body, or on a `type`, `var`, `const`, or `import` declaration, is
  not on a declaration surface and is ignored.
- **Running a Test**: `vtest run` starts `go test ./<package> -run ^<Name>$
  -count=1 -json` for each Test, in the directory of the Go module the Test is
  in, which is the nearest directory with a `go.mod` from the Test's file up to
  the project root. The runner kind is `go-test` (with `--fast`; see Coverage
  below for the measured run). The Test's result is the one
  `pass`, `fail` or `skip` event whose `Test` is exactly the function's name
  (events of subtests and of the package do not count): `pass` is `PASS`,
  `fail` is `FAIL`, and `skip` means the Test was not executed (no Evidence).
  The whole output is kept in the Evidence's log.
- **When no Evidence is recorded**: a package that does not build is
  `E-EXEC-001` (this outranks any other event). No result event for the Test is
  `E-EXEC-002`, and more than one is `E-EXEC-005`. A result that contradicts the
  exit status of `go test` (a Test that passes while `TestMain` exits 1 after
  `m.Run()`, or one that fails while it exits 0) is `E-EXEC-003`. A change to a
  file of the project while the Test runs is `E-EXEC-004`.
- **Tests that are not run**: a Test in no Go module (no `go.mod` between its
  file and the project root), and a Test construct `go test` does not run (type
  parameters, not exactly one `*testing.T` parameter, or a result), get the
  warning `W-EXEC-102` and no Evidence; the other Tests of the run are still run.
  If two Test constructs share a name in one package directory, including one in
  `package x_test` or in a file a build constraint leaves out, none of them is run
  (`E-EXEC-005`). Every `_test.go` file of the directory is counted, whatever the
  adapter's `include` and `.gitignore` say, except names that start with `_` or
  `.`. If one of them cannot be read, no Test of that directory is run, and no
  diagnostic is given (the canon defines no code for it, DS-2012).
- **Execution State** (`go-execution-state-v1`): every regular file under the
  project root except what is under `.git/` and `.verify/` there, byte for byte;
  the toolchain (`GOVERSION`, `GOOS`, `GOARCH` and its instruction-set setting,
  the cgo and compiler variables, `GOFLAGS`, `GOEXPERIMENT`, `GOFIPS140`,
  `GODEBUG`, `GOTOOLCHAIN`, `GO111MODULE`, `GOWORK`, and the version output of
  `CC` when cgo is on) as `go env` reports it; and the invocation. It is not
  reported complete when the module, a `go.work`, or a `replace` is outside the
  project root, when `GOFLAGS` names a file outside it or uses `-exec` or
  `-toolexec`, when the build is not a module build, when a symbolic link leaves
  the project root, when a package of the build outside the standard library
  uses cgo, or when the C compiler of a cgo build cannot report its version.
  Dependencies are bound only through `go.sum`; the module cache is not checked.
- **Coverage** (`run.coverage: go-cover`, the default mode): without `--fast`,
  `vtest run` starts the Test's `go test` with `-covermode=atomic`,
  `-coverpkg=./...` (every package of the module) and `-coverprofile`, and then
  starts `go test ./<package> -run ^$` with the same options, which selects no
  Test. Each run writes its profile to a file of its own under
  `.verify/cache/cov/` (named by a record id, with no extension). The
  Evidence's `runner.kind` is `go-test-cover` for a measured run, and its
  `target_coverage` has the method `go-cover` and one entry per declared target.
  A target's count is the sum of the counts of the profile's blocks that start
  inside the byte range of its construct (its doc comment included), with a
  profile line's file read as `<module path>/<path from the module root>` and
  turned into a project path. A target is:
  - `PASS` when blocks start in its range in both runs, none of them ran in the
    run that selects no Test, and some ran in the Test's run (the count is the
    Test's);
  - `FAIL` (`NOT_EXECUTED`) when they did not run in the Test's run (count 0);
  - `UNKNOWN`, with no count, when it cannot be told for certain: a block of it
    ran in the run that selects no Test (an `init` function, a package variable,
    `TestMain`: that is not a Test's reach), it has no block in a profile (its
    package is not linked into the test binary, or it is in another module, even
    a nested one reached with `replace`: only the packages of the module `go
    test` ran in are measured), the run that selects no Test did not exit with
    code 0 or left no profile, a profile is not of mode `atomic` or has a line
    that cannot be read, a block's position is not in the file, the module path
    cannot be read from `go list -m`, or a non-test `.go` file of the target's
    package holds a line directive (`//line` at the start of a line, or
    `/*line`), which renames the blocks of one file onto another.
  The Test's own result still decides first: a failing Test is `FAIL` whatever
  it reached. A target that only a process the Test started ran has no count (go
  does not merge the profiles of child processes), so it is `FAIL`, not `PASS`.
  The run that selects no Test is run once per measured Test, with the Test's
  Execution State watched around both runs (a file it writes is `E-EXEC-004`).
  Its Execution State is of the runner kind `go-test-cover`, whose invocation
  holds the cover options and the arguments of the run that selects no Test,
  with markers for the profiles.
- **`--fast`, `run.coverage: off` and the config**: `--fast` and `run.coverage:
  off` run the plain `go test` command (runner kind `go-test`) and record
  `target_coverage` as not checked, so `target_binding` is `NO_EVIDENCE`
  (`NOT_CHECKED`); `off` gives no warning, being the project's own setting.
  `run.coverage` of a `go` entry of `config.yaml` is `go-cover` or `off`; any
  other value is `E-CONFIG-001` when the project is scanned. If `go version`
  cannot be run, coverage is unavailable: `W-EXEC-101` and a `target_coverage`
  that is not checked.
- **Static audit** (`oracle_presence`): the adapter reads the text of the Test
  function, and the package clause and imports of its file, with a Go reader of
  its own that runs in process, so `vtest verify` starts no process and does not
  need the Go toolchain. It provides DA-001 (constant assertion), DA-003 (the
  target's result is not checked), DA-004 (a value compared with itself), DA-005
  (empty Test) and DA-006 (no assertion), which `oracle_presence` composes: any
  `FAIL` is `FAIL`, else any `UNKNOWN` is `UNKNOWN`, else `PASS`.
  - **What counts as an assertion**: a call of `Error`, `Errorf`, `Fatal`,
    `Fatalf`, `Fail` or `FailNow` on a variable declared `*testing.T` or
    `testing.TB` (a parameter of the Test or of a function literal, or `var t
    *testing.T`; a copy `u := t` is one too), a call of `panic`, and a call spelled
    as an entry of `scan.assertion_macros` (`assert.Equal`, the callee name with
    dots; the argument of type `testing.T` is left out). `t.Skip`, `return`,
    `os.Exit` and `runtime.Goexit` are not failures.
  - **What decides an assertion**: for a failure report or `panic`, the
    conditions of the `if` and `else if` branches that enclose it and the earlier
    ones of the same chain, the tag and the `case` expressions of the `switch`
    clauses up to its own (all of them when a `default` is among them), the
    condition or range expression of an enclosing `for`, and the conditions of an
    earlier `return`, `break`, `continue`, `t.Skip` or `os.Exit` that may leave
    before it. For an `assertion_macros` call, its arguments.
  - **`PASS` needs a derived value shown to decide an assertion**: a value from
    the call of a declared target (a function, or a method matched by its name),
    followed through `:=` and `var` bindings, assignments, method chains, fields,
    branches and loops (a value that differs between paths is unknown), and
    through the body of a `t.Run` call's function literal as the rest of the Test.
    A conversion to a built-in type, `len`, `cap`, `reflect.DeepEqual`,
    `errors.Is` and `bytes.Equal` pass a value on; a composite literal holding the
    result is read only as a whole (`reflect.DeepEqual(s, want)`, `p == want`).
  - **`FAIL`** when the Test has no assertion and calls nothing that could
    check, when every assertion has no deciding position or only literal ones
    (`if false { t.Fatal() }`, `if x != 1 || true`), when no value derived from
    the result decides any assertion (`_ = Add(1, 2)`, a reassigned result, a
    `t.Skip` or `return` as the only way out), when a condition compares a value
    with itself with `==` or `!=` (`got != got`), and for an empty body.
  - **`UNKNOWN`** wherever the audit cannot tell, never `PASS`: a text that does
    not parse (or nests deeper than 1000 levels, or needs more than 2 000 000
    statements to be walked); a call of the target, or an assertion, only in a
    `go` or `defer` statement, a function literal that is not a `t.Run` subtest,
    or a function the Test calls (`helper(t)`, a function value passed to `t.Run`);
    a derived value that is passed to a function or method (a log call included),
    is the receiver of a method, has its address taken, is assigned to another
    variable, a field, an element or a package variable, is sent on a channel, or is
    referred to by a function literal; a branch that assigns it on one path only; a
    function other than a conversion or comparison function among what decides an
    assertion or in a binding it reads (a delegation target, DS-641): the audit
    never confirms that one terminates, so a check in a helper is `UNKNOWN` even
    when the helper checks; a `panic` in a body that has a `defer` that may recover;
    an exit before an assertion from inside a loop, by a label or `goto`, a `select`, a
    `fallthrough` (the next clause is entered by its own case or by the clause before);
    code that never runs (`if false`, after a `return`); a call that cannot be tied
    to the target (a local function named like it, a function of another package, a
    method of another type for a function target, a package that is not imported, a
    bare call in an `x_test` package); a call of the target that has a receiver or
    an argument that is not a literal, or an assertion whose deciding positions
    read a call or a package-level name, when no value derived from the result
    decides an assertion (the effect of the target may be checked through the
    argument, the receiver or the state it changes, so the result not reaching an
    assertion is no proof; a local bound or assigned from a call or from a name that
    is not a local one counts as such a name); a subtest that may call `Parallel`
    (its body runs after the parent returns, so it is not read as the rest of the
    Test: one that calls or keeps `t.Parallel`, or hands its `T` to a function); a
    `switch` on the type of a derived value; a pointer to a composite literal that
    holds the result; a target declared by a permanent SRC ID; a Test that takes its
    `T` through another name for `testing`.
  - **Not provided**: DA-002 (the static reach of a target). `target_reach` keeps the
    default of the adapter contract, which proves no reach, so `target_binding`
    rests on `go-cover` alone.
- **Limits**: the adapter declares `source_discovery`, `static_analysis`,
  `test_runner` and `coverage`. A Go Test is `PASS` as a whole when its
  `target_binding` (the target ran under the Test, measured by `go-cover`) and
  its `oracle_presence` (the audit finds a check that the target's result
  decides) are both `PASS`, with `chain_integrity` and `orphan_detection`. A Test
  that is `PASS` for `target_binding` has run its target under `go test`'s
  coverage, which says the code ran, not that its result was checked; the audit
  says there is a check, not that the check is right. The audit reads names where
  a clause names them (`testing.T`, `panic`, `true`, `false`, `len`, a
  conversion's type, `reflect.DeepEqual`, an `assertion_macros` spelling), so
  these remain false `PASS` shapes: a type of the project that has the name `T`
  and `Fatal` but records no failure, a package-level `panic`, `true` or `len`
  that replaces the built-in, a method of another type with the name of a method
  target, a derived value in a message argument of an `assertion_macros` call,
  a value compared with a copy of itself (`b := a; a != b`) or by
  `reflect.DeepEqual(a, a)`, a condition that is constant through a named constant
  or an operation other than `||` / `&&` with a literal, a loop over a table
  that is empty at run time, a slice, a method value or a pointer receiver that
  keeps the address of a variable before the result is assigned to it and a call
  that then writes through it (`keep(buf[:]); buf = Fill(); overwrite()`; only the
  forms `&x`, a bound field, element, slice or method value, and a range over the
  variable are followed), and a qualified call `pkg.Name` whose import path ends
  with the target's directory, which is taken for the target's package (the audit
  has no module path to compare). A Test that logs the derived value before checking
  it (`t.Logf("%d", got)`) is `UNKNOWN`: passing it to a method makes it unknown.
- **Known false `PASS` of `target_binding`**: a target is told apart from code
  outside the Test only by comparing the Test's run with the run that selects no
  Test. Code that starts before any Test and reaches the target only while a
  Test runs is counted as the Test's, so the target is `PASS` though the Test
  never called it. Two examples: a goroutine started by an `init` function
  that calls the target later, and a `TestMain` that calls it only when
  `-test.run` is not `^$`. The aggregate of a Go Test can be `PASS`, so this false
  `PASS` of `target_binding` is no longer held back by `oracle_presence` for a
  Test whose check is found in the text but whose call of the target does not
  run (a call under a condition that is false at run time).

```go
// @vtest.id TEST-CALC-ADD
// @vtest.covers VO-CALC-ADD
// @vtest.target calc/calc.go::Add
// @vtest.intent adding two numbers returns their sum
func TestAdd(t *testing.T) {
	if Add(1, 2) != 3 {
		t.Fatal("wrong sum")
	}
}
```
