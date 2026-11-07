# JavaScript / TypeScript adapter (`javascript`)

Discovers `.js`, `.jsx`, `.mjs`, `.cjs`, `.ts`, `.tsx`, `.mts`, and `.cts`
files (skipping `node_modules/`, `dist/`, `build/`, `coverage/`, `.next/`,
`out/`). One adapter covers both languages, because their declaration syntax
is shared. See [lexical.md](lexical.md) for the declaration rules shared by
all framework-neutral adapters.

- **Source Targets**: `function` declarations (generators and `export
  default` included), class methods (including `get`/`set`), and variables or class fields initialized with an arrow
  function or a function expression (`export const add = (a, b) => a + b`).
  `class` and TypeScript `namespace`/`module` bodies qualify them:
  `src/calc.ts::add`, `src/calc.ts::Calc::sum`.
- **Tests**: any construct right after a `@vtest.` comment, typically a Jest,
  Vitest, Mocha, Jasmine, `node:test`, or Playwright call such as
  `test("adds", () => { ... })` or `it(...)`, even inside `describe`
  blocks. Such a call gets the Test locator `<callee>#<n>`, for example
  `test#1`.
- **Limits**: regular-expression literals that contain unbalanced brackets
  (`/[{]/`) can make a file unparseable, and the file is then reported as
  `E-SCAN-001`. Functions nested inside other functions or inside call
  arguments (such as helpers defined in a `describe` callback) are not Source
  Targets, and neither are methods of object literals assigned to a
  variable.

```ts
// @vtest.id TEST-CALC-ADD
// @vtest.covers VO-CALC-ADD
// @vtest.target src/calc.ts::add
// @vtest.intent adding two numbers returns their sum
test("adds", () => {
  expect(add(1, 2)).toBe(3);
});
```
