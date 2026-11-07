# Ruby adapter (`ruby`)

Discovers `.rb` and `.rake` files (skipping `vendor/`, `.bundle/`, `tmp/`,
`log/`, `coverage/`). See [lexical.md](lexical.md) for the declaration
rules shared by all framework-neutral adapters.

- **Source Targets**: `def` methods, including predicate and bang methods
  (`empty?`, `save!`), singleton methods (`self.create`), setters
  (`value=`, also `def value= v`), operators (`==`, `[]`, and unary `+@` and
  `-@`), `private def`, and endless methods
  (`def name = expr`). `class` and `module` (including `A::B`) qualify
  them: `lib/calc.rb::Demo::Calc::add`.
- **Tests**: any construct right after a `@vtest.` `#` comment. That can be
  a Minitest or test-unit method, or an RSpec example such as
  `it "adds" do ... end` with the Test locator `it#<n>`, even inside
  `describe` blocks.
- **Body structure**: `def`, `class`, `module`, `do`, `begin`, `case`, and
  statement-leading `if`/`unless`/`while`/`until`/`for` open a body that
  `end` closes. Modifier forms (`return x if y`), a loop's own `do`, and
  keywords used as method names or symbols (`range.end`, `:end`, `class:`)
  do not.
- **Where a construct ends**: at the `end` paired with the keyword it
  opens, then at the end of its line unless the line ends with an operator
  or the next line starts with `.method`; the bodies of heredocs opened on
  that line (`<<~TEXT` ... `TEXT`) are part of it. `end` inside heredocs,
  `%w[]` literals, interpolations, and regular expressions is text. After a
  method name, `<<word` is a heredoc only with an upper-case label (`a <<b`
  may be a shift); elsewhere any label opens one (`text = <<text`).
- **Limits**: a missing or extra `end`, an unterminated heredoc, string,
  percent literal, or regular expression, or a file that ends while an
  expression is still awaited (`def f =`, `a +`), makes the file
  unreadable (`E-SCAN-001`).

```ruby
# @vtest.id TEST-CALC-ADD
# @vtest.covers VO-CALC-ADD
# @vtest.target lib/calc.rb::Calc::add
# @vtest.intent adding two numbers returns their sum
it "adds" do
  expect(Calc.new.add(1, 2)).to eq(3)
end
```
