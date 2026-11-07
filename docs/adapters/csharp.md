# C# adapter (`csharp`)

Discovers `.cs` files (skipping `bin/` and `obj/`). See
[lexical.md](lexical.md) for the declaration rules shared by all
framework-neutral adapters.

- **Source Targets**: methods, constructors, and operators with a block body
  or an expression body (`=> expr;`). Block-scoped and file-scoped
  `namespace`s and `class`, `struct`, `interface`, `record`, and `enum`
  qualify them: `src/Calc.cs::Demo::Calc::Add`. Properties, accessors, local
  functions, and positional records without a body are not Source Targets.
- **Tests**: any construct right after a `@vtest.` comment, typically an
  xUnit, NUnit, or MSTest method. Its attributes (`[Fact]`,
  `[Theory, InlineData(1, 2)]`) are part of the Test construct. `///` XML
  doc comments work. Free text and XML tags in them are ignored.
- **Limits**: overloads share a locator, so use `@vtest.src-id` to target one
  of them. Preprocessor lines (`#if`, `#region`) are skipped as directives,
  and both branches of an `#if` are scanned.

```csharp
public class CalcTests
{
    // @vtest.id TEST-CALC-ADD
    // @vtest.covers VO-CALC-ADD
    // @vtest.target src/Calc.cs::Demo::Calc::Add
    // @vtest.intent adding two numbers returns their sum
    [Fact]
    public void Adds() => Assert.Equal(3, Calc.Add(1, 2));
}
```
