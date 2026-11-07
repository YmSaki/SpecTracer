# Python adapter (`python`)

Discovers `.py` and `.pyi` files (skipping `__pycache__/`, virtual
environments, `build/`, `dist/`, and tool caches). See
[lexical.md](lexical.md) for the declaration rules shared by all
framework-neutral adapters.

- **Source Targets**: `def` and `async def` definitions, including one-line
  definitions. A decorated definition starts at its first decorator, and the
  body extends over every following line indented deeper. `class` bodies
  qualify methods: `src/calc.py::Calc::add`,
  `src/calc.py::Calc::Inner::fetch`. A definition in a module- or
  class-level `if`, `try`, `with`, `for`, `while`, or `match` belongs to
  the enclosing scope, in every branch (conditions are not evaluated, so
  two branches defining `f` give two `f` Sources). Functions nested inside
  functions are not Source Targets.
- **Tests**: any construct right after a `@vtest.` `#` comment, such as a
  pytest function (its `@pytest.mark...` decorators are part of the
  construct), a `unittest.TestCase` method, or a pytest class method.
- **Where a construct ends**: at the end of its block of logical lines:
  every following logical line indented deeper (a newline inside brackets,
  after a `\`, or inside a string does not start a line), plus `elif`,
  `else`, `except`, `finally`, and `case` clauses at its own indentation
  (a `case` clause of a `match`, not a statement such as `case = 2`).
- **Limits**: declarations are read from `#` comments, not from docstrings.
  A tab advances to the next multiple of eight columns, as in Python. The
  block structure is checked as Python's parser does: a `def` without its
  name and parameter list (`def f:`), a `class` without its name, a `def`,
  `class`, `if`, ... without its `:`, a `:` with no indented block after it (a file
  cut off after `def f():`), an unexpected indent, or a dedent to no
  enclosing level makes the file unreadable (`E-SCAN-001`). f-string
  fields are read as code, comments and nested f-strings included.

```python
# @vtest.id TEST-CALC-ADD
# @vtest.covers VO-CALC-ADD
# @vtest.target src/calc.py::add
# @vtest.intent adding two numbers returns their sum
def test_add():
    assert add(1, 2) == 3
```
