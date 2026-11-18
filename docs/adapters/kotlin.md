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
- **Where a construct ends**: at a `;`, or at a newline after a token that
  can end an expression, unless the next line continues it (`.`, `?.`,
  `?:`, `&&`, `||`, `as`, `else`, `catch`, `finally`). So an expression body
  such as `= if (x) { 1 } else { 2 }` runs to its last branch. A newline
  never ends an annotation line (`@Test`) or a `fun`/`class` header still
  waiting for its `{`, `=`, `:`, or `where`.
- **Limits**: overloads share a locator, so use `@vtest.src-id` to target
  one of them.

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
