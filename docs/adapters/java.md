# Java adapter (`java`)

Discovers `.java` files (skipping `build/`, `out/`, `.gradle/`). See
[lexical.md](lexical.md) for the declaration rules shared by all
framework-neutral adapters.

- **Source Targets**: methods and constructors with a body. `class`,
  `interface`, `@interface`, `enum`, and `record` bodies qualify them:
  `src/Calc.java::Calc::add`, `src/Calc.java::Outer::Inner::run`.
  Abstract and interface methods without a body, field initializers, and
  methods of anonymous classes are not Source Targets.
- **Tests**: any construct right after a `@vtest.` comment, typically a JUnit
  or TestNG method. Its annotations (`@Test`, `@ParameterizedTest`,
  `@Timeout(...)`) are part of the Test construct. Javadoc (`/** ... */`) and
  line comments both work.
- **Where a construct ends**: after the `{}` body of a type, method, or
  constructor, or at its `;`. Brackets in annotations and parameter lists
  (`@Tag(values = {"a"})`) and a field's initializer (`= {1, 2}`) are not a
  body.
- **Limits**: the package is not part of the locator, because the path
  already identifies the file. Overloaded methods share a locator; use
  `@vtest.src-id` to target one of them.

```java
class CalcTest {
    // @vtest.id TEST-CALC-ADD
    // @vtest.covers VO-CALC-ADD
    // @vtest.target src/Calc.java::Calc::add
    // @vtest.intent adding two numbers returns their sum
    @Test
    void adds() { assertEquals(3, new Calc().add(1, 2)); }
}
```
