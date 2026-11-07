# Go adapter (`go`)

Discovers `.go` files (skipping `vendor/` and `testdata/`). See
[lexical.md](lexical.md) for the declaration rules shared by all
framework-neutral adapters.

- **Source Targets**: `func` declarations with a body. A method is qualified
  by its receiver type, and type parameters are not part of the name:
  `calc/calc.go::Add` and `calc/calc.go::Calc::Add` for
  `func (c *Calc) Add(...)`. Function literals are not Source Targets.
- **Tests**: any construct right after a `@vtest.` comment. That can be a
  `go test` function (`func TestAdd(t *testing.T)`), a testify suite method,
  or a Ginkgo `It("...", func() { ... })` call, which gets the Test locator
  `It#<n>`.
- **Limits**: build tags and `//go:build` constraints are not evaluated, so
  every file is scanned.

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
