//! `swift`: Swift sources (`.swift`).
//!
//! Functions are `func` declarations (including operators), `init`, and
//! `deinit` with a body. `class`, `struct`, `enum`, `actor`, `protocol`, and
//! `extension` qualify them (an extension of `Calc` qualifies as `Calc`).
//! Attributes (`@Test`, `@Test("adds") `, `@MainActor`) are part of the
//! construct, so XCTest, Swift Testing, Quick, or any other framework is
//! discovered the same way.

use crate::{
    header::{find_top_level_word, skip_attributes, AttributeForms, HTok, HeaderKind},
    lexer::{LexicalSyntax, StringSyntax, Structure, TokKind},
    LanguageProfile, LexicalAdapter,
};

pub const ADAPTER_ID: &str = "swift";

const SYNTAX: LexicalSyntax = LexicalSyntax {
    line_comments: &["//"],
    line_comment_exclusions: &[],
    block_comments: &[("/*", "*/")],
    nested_block_comments: true,
    strings: &[
        StringSyntax::multi("\"\"\"", "\"\"\"", true),
        StringSyntax::single("\"", "\""),
    ],
    // `#if` / `#else` / `#endif` compiler-control lines.
    preprocessor: true,
    cpp_raw_strings: false,
    word_suffix_chars: &[],
    newline_terminates: true,
    structure: Structure::Braces,
};

const SCOPE_KEYWORDS: &[&str] = &["class", "struct", "enum", "actor", "protocol", "extension"];

#[derive(Clone, Copy, Debug, Default)]
pub struct Swift;

impl LanguageProfile for Swift {
    fn adapter_id(&self) -> &'static str {
        ADAPTER_ID
    }

    fn languages(&self) -> &'static [&'static str] {
        &["swift"]
    }

    fn extensions(&self) -> &'static [&'static str] {
        &["swift"]
    }

    fn excluded_dirs(&self) -> &'static [&'static str] {
        &[".build", "Pods", "Carthage", "DerivedData"]
    }

    fn syntax(&self) -> &'static LexicalSyntax {
        &SYNTAX
    }

    fn classify(&self, header: &[HTok<'_>], has_body: bool) -> HeaderKind {
        if !has_body {
            return HeaderKind::Other;
        }
        let start = skip_attributes(
            header,
            AttributeForms {
                at_sign: true,
                ..AttributeForms::default()
            },
        );
        let header = &header[start..];
        if let Some(keyword) =
            find_top_level_word(header, 0, header.len(), &["func", "init", "deinit"])
        {
            // Only modifiers (`public`, `static`, `override`, ...) may come
            // first; `x = init(...)` is an expression.
            if !header[..keyword]
                .iter()
                .all(|token| token.kind == TokKind::Word)
            {
                return HeaderKind::Other;
            }
            let word = header[keyword].text;
            if word != "func" {
                return HeaderKind::Function(vec![word.to_owned()]);
            }
            // The name is the first token after `func` plus any following
            // tokens up to the generic list or parameters, so operator
            // names such as `==` or `<` are kept whole.
            let rest = &header[keyword + 1..];
            let name = rest
                .iter()
                .take(1)
                .chain(
                    rest.iter()
                        .skip(1)
                        .take_while(|token| !token.is_open('(') && !token.is_punct('<')),
                )
                .filter(|token| !token.is_open('('))
                .map(|token| token.text)
                .collect::<String>();
            if name.is_empty() {
                return HeaderKind::Other;
            }
            return HeaderKind::Function(vec![name]);
        }
        let Some(keyword) = find_top_level_word(header, 0, header.len(), SCOPE_KEYWORDS) else {
            return HeaderKind::Other;
        };
        if !header[..keyword]
            .iter()
            .all(|token| token.kind == TokKind::Word)
        {
            return HeaderKind::Other;
        }
        let mut segments = Vec::new();
        let mut index = keyword + 1;
        while let Some(token) = header.get(index) {
            if token.kind != TokKind::Word {
                break;
            }
            segments.push(token.text.to_owned());
            if header.get(index + 1).is_some_and(|next| next.is_punct('.')) {
                index += 2;
            } else {
                break;
            }
        }
        if segments.is_empty() {
            HeaderKind::Other
        } else {
            HeaderKind::Scope(segments)
        }
    }
}

/// The `swift` adapter.
pub fn adapter() -> LexicalAdapter<Swift> {
    LexicalAdapter::new(Swift)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::discover_text;

    const SOURCE: &str = r#"import Testing

/* outer /* nested */ still a comment { */
public struct Calc {
    let base: Int

    init(base: Int = 0) { self.base = base }

    // @vtest.src-id SRC-SWIFT-ADD
    public func add(_ a: Int, _ b: Int) -> Int {
        return base + a + b
    }

    func max<T: Comparable>(_ a: T, _ b: T) -> T {
        if a > b { return a }
        return b
    }

    var doubled: Int { base * 2 }

    static func == (lhs: Calc, rhs: Calc) -> Bool { lhs.base == rhs.base }
}

extension Calc: CustomStringConvertible {
    var description: String { "Calc(\(base))" }
    func reset() -> Calc { Calc() }
}

#if DEBUG
let label = """
{ not code
"""
#endif

let handler = { (x: Int) -> Int in x }

// @vtest.id TEST-SWIFT-ADD
// @vtest.covers VO-SWIFT-ADD
// @vtest.target Sources/Calc.swift::Calc::add
// @vtest.intent adding two numbers returns their sum
@Test("adds two numbers")
func adds() {
    #expect(Calc().add(1, 2) == 3)
}
"#;

    /// @vtest.id TEST-ADAPTER-SWIFT-DISCOVERS-FUNCS-AND-TESTS
    /// @vtest.covers VO-ADAPTER-SWIFT-REGISTERS-WITHOUT-CORE-CHANGES, VO-ADAPTER-LEXICAL-OPAQUE-LOCATOR-SYNTAX
    /// @vtest.target crates/vtest-adapter-lexical/src/swift.rs::Swift::classify
    /// @vtest.intent verifies Swift funcs, generic funcs, operators, and initializers become type- and extension-qualified Source Targets while computed properties and closures do not, and an annotated Swift Testing function is a Test construct including its attribute
    #[test]
    fn discovers_swift_funcs_and_tests() {
        let outcome = discover_text(&Swift, "Sources/Calc.swift", SOURCE, "demo");
        assert!(outcome.diagnostics.is_empty(), "{:?}", outcome.diagnostics);
        let sources = outcome
            .sources
            .iter()
            .map(|source| source.locator.value.as_str())
            .collect::<Vec<_>>();
        assert_eq!(
            sources,
            vec![
                "Sources/Calc.swift::Calc::init",
                "Sources/Calc.swift::Calc::add",
                "Sources/Calc.swift::Calc::max",
                "Sources/Calc.swift::Calc::==",
                "Sources/Calc.swift::Calc::reset",
                "Sources/Calc.swift::adds",
            ]
        );
        assert_eq!(
            outcome.sources[1].src_id.as_ref().map(|id| id.as_str()),
            Some("SRC-SWIFT-ADD")
        );
        assert_eq!(outcome.tests.len(), 1);
        let test = &outcome.tests[0];
        assert_eq!(test.id.as_str(), "TEST-SWIFT-ADD");
        assert_eq!(test.location.locator, "adds");
        assert!(test
            .construct_text
            .starts_with("@Test(\"adds two numbers\")\nfunc adds() {"));
        assert!(test.construct_text.ends_with("== 3)\n}"));
    }
}
