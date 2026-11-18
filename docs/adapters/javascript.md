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
- **Where a construct ends**: a `function`, `class`, method, `namespace`, or
  `if`/`for`/`try` statement ends after its `{}` body (`else`, `catch`, and
  `finally` continue it); anything else at a `;` or where automatic
  semicolon insertion ends it. A `{` after `:`, `|`, `&`, `=>`, `<`, or
  `extends` is a TypeScript type, so `function f(): { x: number } {` takes
  the second brace as the body. Template literals, regular expressions, and
  JSX elements are read as literals.
- **Limits**: a `/` is read as a regular expression only where an
  expression can start, so a regular expression after `)` (rare outside
  `if (x) /re/.test(y)`) is read as division; if its brackets then do not
  balance, the file is reported as `E-SCAN-001`. After `}`, which may close
  a block or an object literal, a `/` is read as a regular expression only
  if one ends on that line, and otherwise as division. A file that ends
  inside a statement (`const f = () =>`, `const f = function ()`, an
  unterminated regular expression, or a template `${...}` whose brackets do
  not balance) is `E-SCAN-001` too, and so is a statement that the `}` of
  its block cuts off the same way (`broken = () =>` before a class's `}`);
  a function declaration without a body there is read as an overload or
  ambient signature. Functions nested inside other functions or inside call
  arguments (such as helpers defined in a `describe` callback) are not Source
  Targets, and neither are methods of object literals assigned to a
  variable. In an initializer, `name<...>` followed by `(` or a template
  literal is read as a generic call, as TypeScript reads it, in `.js` files
  too, so in `const f = () => a < b, g = c > (d)` the two declarators are
  read as one.

```ts
// @vtest.id TEST-CALC-ADD
// @vtest.covers VO-CALC-ADD
// @vtest.target src/calc.ts::add
// @vtest.intent adding two numbers returns their sum
test("adds", () => {
  expect(add(1, 2)).toBe(3);
});
```
