# Test Specification

## Unmanaged tests

- rust\-cargo: src\/lib\.rs: tests\:\:incomplete\_declaration (E\-SCAN\-007)
- rust\-cargo: src\/lib\.rs: tests\:\:no\_declaration (W\-SCAN\-101)

## Unresolved references

- [TEST\-PARSE](#test-TEST-PARSE) covers: VO\-GONE (E\-SCAN\-003)
- [TEST\-PARSE](#test-TEST-PARSE) related: TEST\-NOWHERE (no diagnostic)
- [TEST\-PARSE](#test-TEST-PARSE) related: TEST\-TWICE (E\-SCAN\-002)
- [TEST\-PARSE](#test-TEST-PARSE) target: locator rust\-cargo::src\/lib\.rs\:\:missing (E\-SCAN\-004)
- [TEST\-RENDER](#test-TEST-RENDER) target: src_id SRC\-TWICE (E\-SCAN\-011)

## Tests by VO

### <a id="vo-VO-GONE"></a>VO\-GONE

claim: (unresolved)
- [TEST\-PARSE](#test-TEST-PARSE)

### <a id="vo-VO-PARSE"></a>VO\-PARSE

claim: fixture claim
- [TEST\-PARSE](#test-TEST-PARSE)
- [TEST\-TWICE](#test-TEST-TWICE~1)
- [TEST\-TWICE](#test-TEST-TWICE~2)

### <a id="vo-VO-RENDER"></a>VO\-RENDER

claim: fixture claim
- [TEST\-RENDER](#test-TEST-RENDER)

## Tests by path

### rust\-cargo: src\/lib\.rs

- [TEST\-PARSE](#test-TEST-PARSE)
- [TEST\-RENDER](#test-TEST-RENDER)
- [TEST\-TWICE](#test-TEST-TWICE~1)
- [TEST\-TWICE](#test-TEST-TWICE~2)

## Tests

### <a id="test-TEST-PARSE"></a>TEST\-PARSE

- intent: parse accepts \`a\|b\` and \*c\*
- covers:
  - [VO\-PARSE](#vo-VO-PARSE): fixture claim
  - [VO\-GONE](#vo-VO-GONE): (unresolved)
- targets:
  - src_id SRC\-PARSE
  - locator rust\-cargo::src\/lib\.rs\:\:missing (unresolved)
- input: a \# b
- expect: \<ok\>
- kind: unit\-normal
- cases:
  - first
  - second \[x\]\(y\)
- related:
  - [TEST\-RENDER](#test-TEST-RENDER)
  - TEST\-NOWHERE (unresolved)
  - TEST\-TWICE (unresolved)
- adapter: rust\-cargo
- path: src\/lib\.rs
- locator: tests\:\:parse\_accepts

### <a id="test-TEST-RENDER"></a>TEST\-RENDER

- intent: render returns
- covers:
  - [VO\-RENDER](#vo-VO-RENDER): fixture claim
- targets:
  - locator rust\-cargo::src\/lib\.rs\:\:render
  - src_id SRC\-TWICE (unresolved)
- input: (none)
- expect: (none)
- kind: (none)
- cases: (none)
- related: (none)
- adapter: rust\-cargo
- path: src\/lib\.rs
- locator: tests\:\:render\_returns

### <a id="test-TEST-TWICE~1"></a>TEST\-TWICE

- intent: first of two
- covers:
  - [VO\-PARSE](#vo-VO-PARSE): fixture claim
- targets: (none)
- input: (none)
- expect: (none)
- kind: (none)
- cases: (none)
- related: (none)
- adapter: rust\-cargo
- path: src\/lib\.rs
- locator: tests\:\:twice\_one

### <a id="test-TEST-TWICE~2"></a>TEST\-TWICE

- intent: second of two
- covers:
  - [VO\-PARSE](#vo-VO-PARSE): fixture claim
- targets: (none)
- input: (none)
- expect: (none)
- kind: (none)
- cases: (none)
- related: (none)
- adapter: rust\-cargo
- path: src\/lib\.rs
- locator: tests\:\:twice\_two
