//! `java`: Java sources (`.java`).
//!
//! Functions are methods and constructors with a body; `class`,
//! `interface`, `@interface`, `enum`, and `record` bodies qualify the
//! methods inside them. Annotations (`@Test`, `@ParameterizedTest`,
//! `@Timeout(5)`) before a method are part of its construct, so JUnit 4/5,
//! TestNG, or any other framework is discovered the same way.

use crate::{
    header::{classify_c_like, AttributeForms, CLikeRules, HTok, HeaderKind},
    lexer::{LexicalSyntax, StringSyntax, Structure},
    LanguageProfile, LexicalAdapter,
};

pub const ADAPTER_ID: &str = "java";

const SYNTAX: LexicalSyntax = LexicalSyntax {
    line_comments: &["//"],
    line_comment_exclusions: &[],
    block_comments: &[("/*", "*/")],
    nested_block_comments: false,
    strings: &[
        StringSyntax::multi("\"\"\"", "\"\"\"", true),
        StringSyntax::single("\"", "\""),
        StringSyntax::single("'", "'"),
    ],
    preprocessor: false,
    cpp_raw_strings: false,
    word_suffix_chars: &[],
    newline_terminates: false,
    structure: Structure::Braces,
};

const RULES: CLikeRules = CLikeRules {
    attributes: AttributeForms {
        at_sign: true,
        hash_bracket: false,
        bracket: false,
    },
    scope_keywords: &["class", "interface", "enum", "record"],
    scope_rest_keywords: &[],
    arrow_bodies: false,
};

#[derive(Clone, Copy, Debug, Default)]
pub struct Java;

impl LanguageProfile for Java {
    fn adapter_id(&self) -> &'static str {
        ADAPTER_ID
    }

    fn languages(&self) -> &'static [&'static str] {
        &["java"]
    }

    fn extensions(&self) -> &'static [&'static str] {
        &["java"]
    }

    fn excluded_dirs(&self) -> &'static [&'static str] {
        &["build", "out", ".gradle"]
    }

    fn syntax(&self) -> &'static LexicalSyntax {
        &SYNTAX
    }

    fn classify(&self, header: &[HTok<'_>], has_body: bool) -> HeaderKind {
        classify_c_like(header, has_body, &RULES)
    }
}

/// The `java` adapter.
pub fn adapter() -> LexicalAdapter<Java> {
    LexicalAdapter::new(Java)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::discover_text;

    const SOURCE: &str = r#"package demo;

import org.junit.jupiter.api.Test;

public class Calculator {
    private final Runnable hook = new Runnable() { public void run() {} };
    private static final String BRACES = """
        { not code }
        """;

    public Calculator() {}

    // @vtest.src-id SRC-JAVA-ADD
    public int add(int a, int b) { return a + b; }

    public <T extends Comparable<T>> T max(java.util.List<T> items) {
        if (items.isEmpty()) { throw new IllegalArgumentException("}"); }
        return items.get(0);
    }

    abstract static class Shape { abstract double area(); }

    @interface Marker { String value() default "x"; }

    record Point(int x, int y) {
        Point { if (x < 0) throw new IllegalArgumentException(); }
        int sum() { return x + y; }
    }
}

class CalculatorTest {
    /**
     * @vtest.id TEST-JAVA-ADD
     * @vtest.covers VO-JAVA-ADD
     * @vtest.target src/Calculator.java::Calculator::add
     * @vtest.intent adding two numbers returns their sum
     */
    @Test
    @Timeout(value = 5, unit = TimeUnit.SECONDS)
    void addsNumbers() {
        assertEquals(3, new Calculator().add(1, 2));
    }
}
"#;

    /// @vtest.id TEST-ADAPTER-JAVA-DISCOVERS-METHODS-AND-TESTS
    /// @vtest.covers VO-ADAPTER-JAVA-REGISTERS-WITHOUT-CORE-CHANGES, VO-ADAPTER-LEXICAL-OPAQUE-LOCATOR-SYNTAX
    /// @vtest.target crates/vtest-adapter-lexical/src/java.rs::Java::classify
    /// @vtest.intent verifies Java methods and constructors become class-qualified Source Targets while initializers, abstract methods, and anonymous-class fields do not, and an annotated JUnit method is a Test construct including its annotations
    #[test]
    fn discovers_java_methods_and_annotated_tests() {
        let outcome = discover_text(&Java, "src/Calculator.java", SOURCE, "demo");
        assert!(outcome.diagnostics.is_empty(), "{:?}", outcome.diagnostics);
        let sources = outcome
            .sources
            .iter()
            .map(|source| source.locator.value.as_str())
            .collect::<Vec<_>>();
        assert_eq!(
            sources,
            vec![
                "src/Calculator.java::Calculator::Calculator",
                "src/Calculator.java::Calculator::add",
                "src/Calculator.java::Calculator::max",
                "src/Calculator.java::Calculator::Point::sum",
                "src/Calculator.java::CalculatorTest::addsNumbers",
            ]
        );
        assert_eq!(
            outcome.sources[1].src_id.as_ref().map(|id| id.as_str()),
            Some("SRC-JAVA-ADD")
        );
        assert_eq!(outcome.tests.len(), 1);
        let test = &outcome.tests[0];
        assert_eq!(test.id.as_str(), "TEST-JAVA-ADD");
        assert_eq!(test.location.locator, "CalculatorTest::addsNumbers");
        assert!(test.construct_text.starts_with("@Test\n    @Timeout"));
        assert!(test.construct_text.ends_with("add(1, 2));\n    }"));
    }
}
