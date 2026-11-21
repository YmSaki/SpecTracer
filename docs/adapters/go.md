# Go adapter (`go`)

Discovers Tests and Source Targets in `.go` files and runs the Tests with `go
test`. See [lexical.md](lexical.md)
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
  the project root. The runner kind is `go-test`. The Test's result is the one
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
- **Limits**: the adapter declares `source_discovery` and `test_runner`. There
  is no coverage and no oracle check, so `target_binding` and `oracle_presence`
  stay `NO_EVIDENCE` (`NOT_CHECKED`) for a Go Test, whatever its Evidence says.
  `vtest run` records the Evidence, and a run without `--fast` reports
  `W-EXEC-101` because no coverage is measured.

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
