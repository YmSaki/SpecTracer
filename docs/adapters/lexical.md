# Framework-neutral language adapters

The adapters built on `vtest-adapter-lexical` discover Tests and Source Targets
in languages other than Rust. They do not depend on a test framework, so the
same rules work with pytest, JUnit, Jest, GoogleTest, or a hand-written runner.

## Declaring a Test

Write the `@vtest.` declaration in an ordinary comment directly above the
construct that is the Test. The construct can be a function, a method, or a
framework call with a body:

```js
// @vtest.id TEST-CALC-ADD
// @vtest.covers VO-CALC-ADD
// @vtest.target src/calc.js::add
// @vtest.intent adding two numbers returns their sum
test("adds", () => {
  expect(add(1, 2)).toBe(3);
});
```

- The grammar and keys are the same as for `rust-cargo`: `id`, `covers`,
  `target`, `intent` (required: `id`, `covers`, `intent`), plus `input`,
  `expect`, `kind`, and the repeatable `case`, `related`, and `target`.
- The comment block must start its own line, and at most one line break may
  separate it from the construct. A blank line ends the association.
- Attributes, annotations, and decorators between the comment and the
  declaration (`@Test`, `[Fact]`, `@pytest.mark.slow`) are part of the Test
  construct.

## Declaring a permanent SRC ID

A comment above a function whose `@vtest.` lines contain no test key declares
that function's permanent SRC ID:

```java
// @vtest.src-id SRC-CALC-ADD
int add(int a, int b) { return a + b; }
```

## Target locators

Every function definition is a Source Target with the locator
`<project-relative path>::<enclosing scopes>::<name>`, for example
`src/Calc.java::demo::Calculator::add`. Overloads share one locator, so a
Target Reference to it is ambiguous (`E-SCAN-004`). In that case, target a
permanent SRC ID instead.

## What is verified

These adapters declare only the `source_discovery` capability. Tests from them
are traced by `chain_integrity` and `orphan_detection`. `target_binding` stays
`NO_EVIDENCE` (`NOT_EXECUTED`, `NOT_CHECKED`), `oracle_presence` stays
`NO_EVIDENCE` (`NOT_CHECKED`), and `vtest run` rejects them with
`E-ADAPTER-004`. No test runner or coverage tool is assumed.

A test without a `@vtest.` declaration is not observed, because recognizing it
would require knowing the framework. A file the lexer cannot balance is
reported as `E-SCAN-001` and contributes nothing.

## Configuration

```yaml
adapters:
  - id: <adapter id>
    roots: ['.']
    scan:
      include: [src, tests]
      assertion_macros: []
    run:
      coverage: none
```

Each language adapter has its own page in this directory.

## Adding a language

A language is a `LanguageProfile` in `crates/vtest-adapter-lexical/src/<lang>.rs`:
its comment and string syntax, file extensions, excluded directories, and a
`classify` function that tells function definitions and named scopes from other
statements. The profile chooses one of the block structures this crate
provides:

| `Structure` | Bodies are delimited by | Example languages |
|---|---|---|
| `Braces` | `{ ... }` | C, C++, Java, C#, Go, JavaScript |
| `Indentation` | the indentation after a `:` line | Python |
| `KeywordEnd` | an opening keyword and its `end` | Ruby |

A language change adds only that file, one `pub mod <lang>;` line in
`src/lib.rs`, and one `registry.register(...)` line in
`crates/vtest-cli/src/adapters.rs` (the composition root, BD-007 / BD-102),
plus its tests and documentation. If a language needs something the crate does
not provide, such as another block structure or a lexer rule, change the crate
first in its own change, not in the language change.
