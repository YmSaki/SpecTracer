//! `php`: PHP sources (`.php`).
//!
//! Functions are named `function` declarations with a body: free functions
//! and methods. `namespace` (braced or statement form, `A\B` names) and
//! `class`, `interface`, `trait`, and `enum` qualify them. `#[...]`
//! attributes and docblock annotations are part of the construct, so
//! PHPUnit methods, Pest's `test('adds', function () { ... })`, Codeception,
//! or any other framework are discovered the same way.

use crate::{
    header::{find_top_level_word, skip_attributes, AttributeForms, HTok, HeaderKind},
    lexer::{LexicalSyntax, StringSyntax, Structure, TokKind},
    LanguageProfile, LexicalAdapter,
};

pub const ADAPTER_ID: &str = "php";

const SYNTAX: LexicalSyntax = LexicalSyntax {
    line_comments: &["//", "#"],
    line_comment_exclusions: &["#["],
    block_comments: &[("/*", "*/")],
    nested_block_comments: false,
    strings: &[
        StringSyntax::multi("\"", "\"", true),
        StringSyntax::multi("'", "'", true),
    ],
    preprocessor: false,
    cpp_raw_strings: false,
    word_suffix_chars: &[],
    newline_terminates: false,
    structure: Structure::Braces,
};

const SCOPE_KEYWORDS: &[&str] = &["class", "interface", "trait", "enum"];

#[derive(Clone, Copy, Debug, Default)]
pub struct Php;

impl LanguageProfile for Php {
    fn adapter_id(&self) -> &'static str {
        ADAPTER_ID
    }

    fn languages(&self) -> &'static [&'static str] {
        &["php"]
    }

    fn extensions(&self) -> &'static [&'static str] {
        &["php"]
    }

    fn excluded_dirs(&self) -> &'static [&'static str] {
        &["vendor"]
    }

    fn syntax(&self) -> &'static LexicalSyntax {
        &SYNTAX
    }

    fn classify(&self, header: &[HTok<'_>], has_body: bool) -> HeaderKind {
        let start = skip_attributes(
            header,
            AttributeForms {
                hash_bracket: true,
                ..AttributeForms::default()
            },
        );
        let mut header = &header[start..];
        // The `<?php` open tag starts the first statement of a file.
        if header.len() >= 3
            && header[0].is_punct('<')
            && header[1].is_punct('?')
            && header[2].is_word("php")
        {
            header = &header[3..];
        }
        if header
            .first()
            .is_some_and(|token| token.is_word("namespace"))
        {
            let segments = header[1..]
                .iter()
                .filter(|token| token.kind == TokKind::Word)
                .map(|token| token.text.to_owned())
                .collect::<Vec<_>>();
            return match (segments.is_empty(), has_body) {
                (true, _) => HeaderKind::Other,
                (false, true) => HeaderKind::Scope(segments),
                (false, false) => HeaderKind::ScopeRest(segments),
            };
        }
        if let Some(function) = find_top_level_word(header, 0, header.len(), &["function"]) {
            // Only modifiers may precede `function`; `$f = function () {}`
            // and `test('x', function () {})` are expressions.
            if !has_body
                || !header[..function]
                    .iter()
                    .all(|token| token.kind == TokKind::Word)
            {
                return HeaderKind::Other;
            }
            let mut index = function + 1;
            if header.get(index).is_some_and(|token| token.is_punct('&')) {
                index += 1;
            }
            return match header.get(index) {
                Some(name) if name.kind == TokKind::Word => {
                    HeaderKind::Function(vec![name.text.to_owned()])
                }
                _ => HeaderKind::Other,
            };
        }
        let Some(keyword) = find_top_level_word(header, 0, header.len(), SCOPE_KEYWORDS) else {
            return HeaderKind::Other;
        };
        let modifiers_only = header[..keyword]
            .iter()
            .all(|token| token.kind == TokKind::Word);
        match header.get(keyword + 1) {
            Some(name) if name.kind == TokKind::Word && has_body && modifiers_only => {
                HeaderKind::Scope(vec![name.text.to_owned()])
            }
            _ => HeaderKind::Other,
        }
    }
}

/// The `php` adapter.
pub fn adapter() -> LexicalAdapter<Php> {
    LexicalAdapter::new(Php)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::discover_text;

    const SOURCE: &str = r#"<?php namespace App\Math;

declare(strict_types=1);

use PHPUnit\Framework\TestCase;

# a hash comment with a brace {
final class Calc
{
    private string $label = "{ not
code }";

    // @vtest.src-id SRC-PHP-ADD
    public function add(int $a, int $b): int
    {
        return $a + $b;
    }

    abstract protected function undefined(): int;

    public static function &shared(): array { static $x = []; return $x; }
}

function helper(callable $f = null) {
    $g = function () use ($f) { return $f; };
    return fn($x) => $x;
}

class CalcTest extends TestCase
{
    /**
     * @vtest.id TEST-PHP-ADD
     * @vtest.covers VO-PHP-ADD
     * @vtest.target src/Calc.php::App::Math::Calc::add
     * @vtest.intent adding two numbers returns their sum
     */
    #[Test]
    #[DataProvider('numbers')]
    public function testAdd(): void
    {
        $this->assertSame(3, (new Calc())->add(1, 2));
    }
}
"#;

    /// @vtest.id TEST-ADAPTER-PHP-DISCOVERS-FUNCTIONS-AND-TESTS
    /// @vtest.covers VO-ADAPTER-PHP-REGISTERS-WITHOUT-CORE-CHANGES, VO-ADAPTER-LEXICAL-OPAQUE-LOCATOR-SYNTAX
    /// @vtest.target crates/vtest-adapter-lexical/src/php.rs::Php::classify
    /// @vtest.intent verifies PHP functions and methods become namespace- and class-qualified Source Targets while abstract methods and closures do not, hash comments are not attributes, and an attributed PHPUnit method is a Test construct including its attributes
    #[test]
    fn discovers_php_functions_and_tests() {
        let outcome = discover_text(&Php, "src/Calc.php", SOURCE, "demo");
        assert!(outcome.diagnostics.is_empty(), "{:?}", outcome.diagnostics);
        let sources = outcome
            .sources
            .iter()
            .map(|source| source.locator.value.as_str())
            .collect::<Vec<_>>();
        assert_eq!(
            sources,
            vec![
                "src/Calc.php::App::Math::Calc::add",
                "src/Calc.php::App::Math::Calc::shared",
                "src/Calc.php::App::Math::helper",
                "src/Calc.php::App::Math::CalcTest::testAdd",
            ]
        );
        assert_eq!(
            outcome.sources[0].src_id.as_ref().map(|id| id.as_str()),
            Some("SRC-PHP-ADD")
        );
        assert_eq!(outcome.tests.len(), 1);
        let test = &outcome.tests[0];
        assert_eq!(test.id.as_str(), "TEST-PHP-ADD");
        assert!(test
            .construct_text
            .starts_with("#[Test]\n    #[DataProvider('numbers')]"));
        assert!(test.construct_text.ends_with("add(1, 2));\n    }"));
    }
}
