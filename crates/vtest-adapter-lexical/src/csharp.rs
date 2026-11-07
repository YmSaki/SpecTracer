//! `csharp`: C# sources (`.cs`).
//!
//! Functions are methods, constructors, and operators with a block body or
//! an expression body (`=> expr;`). `namespace` (block or file-scoped),
//! `class`, `struct`, `interface`, `record`, and `enum` qualify them.
//! Attributes (`[Fact]`, `[Theory, InlineData(1, 2)]`, `[TestMethod]`)
//! before a method are part of its construct, so xUnit, NUnit, MSTest, or
//! any other framework is discovered the same way.

use crate::{
    header::{classify_c_like, AttributeForms, CLikeRules, HTok, HeaderKind},
    lexer::{LexicalSyntax, StringSyntax, Structure},
    LanguageProfile, LexicalAdapter,
};

pub const ADAPTER_ID: &str = "csharp";

const SYNTAX: LexicalSyntax = LexicalSyntax {
    line_comments: &["//"],
    line_comment_exclusions: &[],
    block_comments: &[("/*", "*/")],
    nested_block_comments: false,
    strings: &[
        StringSyntax::multi("\"\"\"", "\"\"\"", false),
        StringSyntax::multi("@\"", "\"", false),
        StringSyntax::single("\"", "\""),
        StringSyntax::single("'", "'"),
    ],
    preprocessor: true,
    cpp_raw_strings: false,
    word_suffix_chars: &[],
    newline_terminates: false,
    structure: Structure::Braces,
};

const RULES: CLikeRules = CLikeRules {
    attributes: AttributeForms {
        at_sign: false,
        hash_bracket: false,
        bracket: true,
    },
    scope_keywords: &[
        "namespace",
        "class",
        "struct",
        "interface",
        "record",
        "enum",
    ],
    scope_rest_keywords: &["namespace"],
    arrow_bodies: true,
};

#[derive(Clone, Copy, Debug, Default)]
pub struct CSharp;

impl LanguageProfile for CSharp {
    fn adapter_id(&self) -> &'static str {
        ADAPTER_ID
    }

    fn languages(&self) -> &'static [&'static str] {
        &["csharp"]
    }

    fn extensions(&self) -> &'static [&'static str] {
        &["cs"]
    }

    fn excluded_dirs(&self) -> &'static [&'static str] {
        &["bin", "obj"]
    }

    fn syntax(&self) -> &'static LexicalSyntax {
        &SYNTAX
    }

    fn classify(&self, header: &[HTok<'_>], has_body: bool) -> HeaderKind {
        classify_c_like(header, has_body, &RULES)
    }
}

/// The `csharp` adapter.
pub fn adapter() -> LexicalAdapter<CSharp> {
    LexicalAdapter::new(CSharp)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::discover_text;

    const SOURCE: &str = r#"using Xunit;

namespace Demo.Math;

#region helpers
public static class Calculator
{
    private const string Braces = @"{ ""not code"" }";

    // @vtest.src-id SRC-CS-ADD
    public static int Add(int a, int b)
    {
        return a + b;
    }

    public static int Twice(int a) => a * 2;

    public static T Max<T>(T a, T b) where T : System.IComparable<T>
    {
        if (a.CompareTo(b) > 0) { return a; }
        return b;
    }

    public static int Count { get { return 1; } }

    public record Point(int X, int Y);
}
#endregion

public class CalculatorTests
{
    /// <summary>Adds.</summary>
    /// @vtest.id TEST-CS-ADD
    /// @vtest.covers VO-CS-ADD
    /// @vtest.target src/Calculator.cs::Demo::Math::Calculator::Add
    /// @vtest.intent adding two numbers returns their sum
    [Theory]
    [InlineData(1, 2, 3)]
    public void AddsNumbers(int a, int b, int expected)
    {
        Assert.Equal(expected, Calculator.Add(a, b));
    }
}
"#;

    /// @vtest.id TEST-ADAPTER-CSHARP-DISCOVERS-METHODS-AND-TESTS
    /// @vtest.covers VO-ADAPTER-CSHARP-REGISTERS-WITHOUT-CORE-CHANGES, VO-ADAPTER-LEXICAL-OPAQUE-LOCATOR-SYNTAX
    /// @vtest.target crates/vtest-adapter-lexical/src/csharp.rs::CSharp::classify
    /// @vtest.intent verifies C# block-bodied and expression-bodied methods become namespace- and class-qualified Source Targets while properties and positional records do not, and an attributed xUnit method is a Test construct including its attributes
    #[test]
    fn discovers_csharp_methods_and_attributed_tests() {
        let outcome = discover_text(&CSharp, "src/Calculator.cs", SOURCE, "demo");
        assert!(outcome.diagnostics.is_empty(), "{:?}", outcome.diagnostics);
        let sources = outcome
            .sources
            .iter()
            .map(|source| source.locator.value.as_str())
            .collect::<Vec<_>>();
        assert_eq!(
            sources,
            vec![
                "src/Calculator.cs::Demo::Math::Calculator::Add",
                "src/Calculator.cs::Demo::Math::Calculator::Twice",
                "src/Calculator.cs::Demo::Math::Calculator::Max",
                "src/Calculator.cs::Demo::Math::CalculatorTests::AddsNumbers",
            ]
        );
        assert_eq!(
            outcome.sources[0].src_id.as_ref().map(|id| id.as_str()),
            Some("SRC-CS-ADD")
        );
        assert_eq!(outcome.tests.len(), 1);
        let test = &outcome.tests[0];
        assert_eq!(test.id.as_str(), "TEST-CS-ADD");
        assert!(test
            .construct_text
            .starts_with("[Theory]\n    [InlineData(1, 2, 3)]"));
        assert!(test
            .construct_text
            .ends_with("Calculator.Add(a, b));\n    }"));
    }
}
