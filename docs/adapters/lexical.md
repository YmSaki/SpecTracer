# Framework-neutral language adapters

The adapters built on `vtest-adapter-lexical` discover Tests and Source Targets
in languages other than Rust. They do not depend on a test framework, so the
same rules work with pytest, JUnit, Jest, GoogleTest, or a hand-written runner.

> **Status: experimental.** These adapters are merged ahead of the canonical
> scope as an experimental implementation. REQ-264, SPEC-218, SPEC-361, and
> SPEC-458 still list `rust-cargo` as the only production language adapter in
> v0.1, and this page does not change that scope. Until the canonical
> documents are revised, treat their behavior as subject to change.

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
would require knowing the framework. A file the adapter cannot read with
confidence (an unterminated string, unbalanced brackets, a missing `end`) is
reported as `E-SCAN-001` and contributes nothing. A declared Test whose extent
cannot be determined is reported as Missing with `E-SCAN-001`; it is never
returned as a fragment of its construct.

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

A language is a `LanguageProfile` in `crates/vtest-adapter-lexical/src/<lang>.rs`.
Its `extract` function reads one file by that language's own rules and returns
`FileItems`:

- every function definition (a Source Target): its qualified name and byte
  range;
- every comment block attached to a construct: the comment's lines without the
  comment markers, and the construct's byte range, or why the range could not
  be determined.

The crate root does the rest for every language: it walks the include paths,
parses the `@vtest.` lines, reports their diagnostics, and builds Drafts and
Locators. The composition root (`crates/vtest-ops/src/registry.rs`, BD-007 /
BD-102) injects each profile as `LexicalAdapter::new(Profile)`.

Each language reads its comments and strings and decides where a declaration
ends by its own grammar, written for that language alone:

| Language | Where a declaration ends |
|---|---|
| C, C++, C#, Java | after the `{}` body, or at `;` |
| Go | at a newline where Go inserts a semicolon |
| Kotlin | at a newline that ends the expression, unless the next line continues it (`else`, `.`, `?:`) |
| Swift | at a newline, judged by operator whitespace (`x!` ends a line, `a +` does not) |
| JavaScript, TypeScript | after the body, or where automatic semicolon insertion ends it |
| PHP | at `;`, or after the body |
| Python | at the end of the indented block of logical lines |
| Ruby | at the `end` paired with its keyword |

Only what means the same thing in every language is shared, in
`src/shared.rs`: pairing brackets once the lexer has found them, and the rule
that attaches a comment block to the construct on the next line (SpecTracer's
annotation rule). Comment syntax is not shared even where languages spell it
the same way, because it is each language's grammar.

A language change adds only its file, one `pub mod <lang>;` line in
`src/lib.rs`, one `registry.register(...)` line in the composition root, its
tests, and its documentation. If a language needs something shared that the
crate does not provide, change the crate first in its own change, not in the
language change.
