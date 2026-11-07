# Swift adapter (`swift`)

Discovers `.swift` files (skipping `.build/`, `Pods/`, `Carthage/`,
`DerivedData/`). See [lexical.md](lexical.md) for the declaration rules
shared by all framework-neutral adapters.

- **Source Targets**: `func` declarations (operators included, such as
  `==`), `init`, and `deinit` with a body. `class`, `struct`, `enum`,
  `actor`, `protocol`, and `extension` qualify them, and an extension
  qualifies with the extended type's name: `Sources/Calc.swift::Calc::add`.
  Computed properties, accessors, and closures are not Source Targets.
- **Tests**: any construct right after a `@vtest.` comment. That can be an
  XCTest method, a Swift Testing `@Test` function (the attribute is part of
  the construct), or a Quick `it("...") { ... }` block.
- **Where a construct ends**: after the `{}` body of a `func`, `init`,
  `deinit`, or type, or a property's accessor block; otherwise at a `;` or a
  newline, judged by Swift's operator whitespace (a line ending with `x!` is
  complete, one ending with `a +` is not; a next line starting with a spaced
  operator or `.member` continues it). A function's body is the first `{`
  after its parameter list, so a default argument closure
  (`func f(x: () -> Int = { 1 })`) is not the body.
- **Limits**: overloads (including several `init`s) share a locator, so use
  `@vtest.src-id` to target one of them. `#if` lines are skipped, and every
  branch is scanned.

```swift
// @vtest.id TEST-CALC-ADD
// @vtest.covers VO-CALC-ADD
// @vtest.target Sources/Calc.swift::Calc::add
// @vtest.intent adding two numbers returns their sum
@Test func adds() {
    #expect(Calc().add(1, 2) == 3)
}
```
