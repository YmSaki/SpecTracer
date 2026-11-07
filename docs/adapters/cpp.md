# C++ adapter (`cpp`)

Discovers `.cc`, `.cpp`, `.cxx`, `.c++`, `.h`, `.hh`, `.hpp`, `.hxx`,
`.h++`, `.ipp`, `.inl`, and `.tpp` files (skipping `build/` and
`cmake-build-*`). See [lexical.md](lexical.md) for the declaration rules
shared by all framework-neutral adapters.

- **Source Targets**: function definitions with a body. This includes member
  functions (inline or out of line as `Calc::add`), constructors with
  initializer lists, operators (`operator==`), and templates. `namespace`
  (including `a::b`), `class`, `struct`, `union`, and `enum` qualify them:
  `src/calc.cpp::demo::math::Calc::add`. Declarations, lambdas, and
  initializers are not Source Targets.
- **Tests**: any construct right after a `@vtest.` comment. GoogleTest
  `TEST`/`TEST_F`, Catch2 `TEST_CASE`, doctest, and Boost.Test macros with a
  body all work. Such a macro block is named after its macro, so several in
  one file get the Test locator `<macro>#<n>`.
- **Limits**: overloads share a locator, so use `@vtest.src-id` to target
  one of them. Both branches of `#if` are scanned. A Test can target only
  `cpp` Source Targets: a C function scanned by the `c` adapter has another
  adapter ID.

```cpp
// @vtest.id TEST-CALC-ADD
// @vtest.covers VO-CALC-ADD
// @vtest.target src/calc.cpp::demo::Calc::add
// @vtest.intent adding two numbers returns their sum
TEST(CalcTest, Adds) { EXPECT_EQ(demo::Calc().add(1, 2), 3); }
```
