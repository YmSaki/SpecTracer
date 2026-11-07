# Kotlin adapter (`kotlin`)

Discovers `.kt` and `.kts` files (skipping `build/`, `out/`, `.gradle/`).
See [lexical.md](lexical.md) for the declaration rules shared by all
framework-neutral adapters.

- **Source Targets**: `fun` declarations with a block body or an expression
  body (`fun f() = expr`). Extension functions are named without their
  receiver, and backtick names keep their text without the backticks.
  `class`, `interface`, `object`, and `companion object` (`Companion` when
  unnamed) qualify them: `src/Calc.kt::Calc::add`,
  `src/Calc.kt::Calc::Companion::create`. Abstract funs and members of
  anonymous objects are not Source Targets.
- **Tests**: any construct right after a `@vtest.` comment. That can be a
  JUnit or kotlin.test method (with `@Test` as part of the construct) or a
  Kotest/Spek block such as `"adds" { ... }` or `test("adds") { ... }`.
- **Limits**: overloads share a locator, so use `@vtest.src-id` to target
  one of them. A declaration header split before its `{` onto a line that
  does not start with `{`, `.`, `:`, `=`, or `?` is read as ending at the
  line break.

```kotlin
class CalcTest {
    // @vtest.id TEST-CALC-ADD
    // @vtest.covers VO-CALC-ADD
    // @vtest.target src/Calc.kt::Calc::add
    // @vtest.intent adding two numbers returns their sum
    @Test
    fun `adds two numbers`() = assertEquals(3, Calc().add(1, 2))
}
```
