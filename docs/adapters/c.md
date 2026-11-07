# C adapter (`c`)

Discovers `.c` and `.h` files (skipping `build/`). See
[lexical.md](lexical.md) for the declaration rules shared by all
framework-neutral adapters.

- **Source Targets**: function definitions with a body, including headers
  split over several lines (`static char *\ncopy(...)\n{`). The locator is
  `<path>::<name>`, for example `src/calc.c::add`. Prototypes, `struct`
  definitions, initializers, and `#define` macros are not Source Targets.
- **Tests**: any construct right after a `@vtest.` comment, such as a Unity
  or CMocka test function or a Check `START_TEST(name) { ... }` block. A
  macro block is named after its macro (`START_TEST`); when several share a
  name in one file, the Test locator falls back to `<first word>#<n>`.
- **Where a construct ends**: after a function's `{}` body, or at the `;`
  of a declaration (initializers and `struct`/`union`/`enum` bodies
  included). A macro invocation on a line of its own (`END_TEST`) ends at
  its line.
- **Limits**: preprocessor lines are skipped, and both branches of an `#if`
  are scanned. A function whose name comes from a macro expansion is named
  after the macro. A C++ adapter (`cpp`) also claims `.h`, so configure the
  one that matches the project.

```c
// @vtest.id TEST-CALC-ADD
// @vtest.covers VO-CALC-ADD
// @vtest.target src/calc.c::add
// @vtest.intent adding two numbers returns their sum
void test_add(void) { TEST_ASSERT_EQUAL(3, add(1, 2)); }
```
