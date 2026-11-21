# Go adapter (`go`)

Discovers Tests and Source Targets in `.go` files. See [lexical.md](lexical.md)
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
- **Limits**: the adapter declares only `source_discovery`: there is no
  runner, no coverage, and no oracle check, so `target_binding` and
  `oracle_presence` stay `NO_EVIDENCE`, and `vtest run` rejects Go Tests with
  `E-ADAPTER-004`.

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
