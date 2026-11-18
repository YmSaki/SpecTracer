# PHP adapter (`php`)

Discovers `.php` files (skipping `vendor/`). See [lexical.md](lexical.md)
for the declaration rules shared by all framework-neutral adapters.

- **Source Targets**: named `function` declarations with a body, both free
  functions and methods. `namespace` (statement form `namespace A\B;` or
  braced) and `class`, `interface`, `trait`, and `enum` qualify them:
  `src/Calc.php::App::Math::Calc::add`. A named function declared inside a
  control statement (`if (!function_exists('f')) { function f() {} }`, a
  `switch` case, alternative syntax) or inside another function body belongs
  to the namespace, so it is named `src/a.php::App::f`. Abstract methods,
  closures, and arrow functions are not Source Targets.
- **Tests**: any construct right after a `@vtest.` comment. That can be a
  PHPUnit method, including its `#[Test]` attributes, or a Pest
  `test('adds', function () { ... });` call with the Test locator `test#<n>`.
  `@vtest.` lines are read from docblocks, `//` comments, and `#` comments.
- **Where a construct ends**: at its `;`, or after the `{}` body of a
  `function`, type, `namespace`, or control statement (`else`, `catch`, and
  `finally` continue it). A control statement's body can also be a single
  statement (`if ($a) one(); else two();`, `do one(); while ($a);`) or
  alternative syntax (`if ($a): ... endif;`). Closures, `match` arms, and attributes (`#[...]`)
  inside a statement do not end it. Inline HTML outside `<?php ... ?>` is
  not code.
- **Limits**: an unterminated string, heredoc, or nowdoc, or brackets that
  do not balance in the code between `<?php` and `?>`, or a file that ends
  inside a statement (`function f()`, `if ($a)`, a call without its `;`),
  make the file unreadable (`E-SCAN-001`). A statement that the `}` of its
  block cuts off the same way is not read as a whole construct either.

```php
final class CalcTest extends TestCase
{
    // @vtest.id TEST-CALC-ADD
    // @vtest.covers VO-CALC-ADD
    // @vtest.target src/Calc.php::App::Calc::add
    // @vtest.intent adding two numbers returns their sum
    public function testAdd(): void
    {
        $this->assertSame(3, (new Calc())->add(1, 2));
    }
}
```
