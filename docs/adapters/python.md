# Python adapter (`python`)

Discovers `.py` and `.pyi` files (skipping `__pycache__/`, virtual
environments, `build/`, `dist/`, and tool caches). See
[lexical.md](lexical.md) for the declaration rules shared by all
framework-neutral adapters.

- **Source Targets**: `def` and `async def` definitions, including one-line
  definitions. A decorated definition starts at its first decorator, and the
  body extends over every following line indented deeper. `class` bodies
  qualify methods: `src/calc.py::Calc::add`,
  `src/calc.py::Calc::Inner::fetch`. Functions nested inside functions are
  not Source Targets.
- **Tests**: any construct right after a `@vtest.` `#` comment, such as a
  pytest function (its `@pytest.mark...` decorators are part of the
  construct), a `unittest.TestCase` method, or a pytest class method.
- **Limits**: declarations are read from `#` comments, not from docstrings.
  Tabs count as one column, so mixed tab/space indentation can mis-nest.

```python
# @vtest.id TEST-CALC-ADD
# @vtest.covers VO-CALC-ADD
# @vtest.target src/calc.py::add
# @vtest.intent adding two numbers returns their sum
def test_add():
    assert add(1, 2) == 3
```
