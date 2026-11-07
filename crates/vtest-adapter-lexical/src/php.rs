//! `php`: PHP sources (`.php`).
//!
//! A file is read in three steps, all specific to PHP:
//!
//! 1. [`lex`] reads the code between `<?php` (or `<?=`) and `?>` (which
//!    ends a statement like `;`); the inline HTML around it is not code.
//!    It splits the code into comments (`//`, `#`, `/* */`), string
//!    literals (`"{$a['}']}"` interpolations, heredocs, and nowdocs
//!    included), words (`$variables` and `App\Math` names included),
//!    brackets (`#[` opens an attribute), `;`, and operators.
//! 2. [`Reader::statement_end`] finds where a statement or class member
//!    ends. PHP needs no newline rule: a statement ends at its `;`, or
//!    after the `{}` body of a `function`, `class`, `interface`, `trait`,
//!    `enum`, `namespace`, or control statement (an `else`, `elseif`,
//!    `catch`, or `finally` continues it). Brackets are skipped whole, so
//!    closures, `match` arms, and anonymous classes inside an expression
//!    statement do not end it.
//! 3. [`Reader::statements`] walks the file: `namespace` (braced or
//!    statement form, `A\B` names) and `class`, `interface`, `trait`, and
//!    `enum` bodies qualify the named `function` declarations inside them.
//!    A method without a body (abstract or in an interface) and closures
//!    are not Source Targets.
//!
//! A comment block (a docblock included) attached to a statement or member
//! makes it a Test (or declares its SRC ID); its `#[...]` attributes are
//! part of the construct, so PHPUnit methods and Pest's
//! `test('adds', function () { ... });` are discovered the same way.

use crate::{
    shared::{attached_comments, pair_brackets, Attached, Span},
    Annotated, ExtractError, FileItems, Function, LanguageProfile, LexicalAdapter,
};

pub const ADAPTER_ID: &str = "php";

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

    fn extract(&self, source: &str) -> Result<FileItems, ExtractError> {
        let tokens = lex(source)?;
        let partners = pair_brackets(tokens.iter().map(|token| match token.kind {
            Kind::Open(c) | Kind::Close(c) => (token.start, Some(c)),
            _ => (token.start, None),
        }))?;
        let reader = Reader {
            source,
            tokens: &tokens,
            partners: &partners,
        };
        let mut functions = Vec::new();
        reader.statements(0, tokens.len(), &mut Vec::new(), &mut functions);
        Ok(FileItems {
            functions,
            annotated: reader.annotated(),
        })
    }
}

/// The `php` adapter.
pub fn adapter() -> LexicalAdapter<Php> {
    LexicalAdapter::new(Php)
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Kind {
    Comment,
    Literal,
    Word,
    /// `(`, `[`, `{`, or `#[` (read as `[`).
    Open(char),
    Close(char),
    /// `;` or `?>`.
    Semi,
    Op,
}

#[derive(Clone, Copy, Debug)]
struct Tok {
    kind: Kind,
    start: usize,
    end: usize,
}

fn is_word_char(byte: u8) -> bool {
    byte == b'_' || byte == b'\\' || byte.is_ascii_alphanumeric() || !byte.is_ascii()
}

/// Where the next `<?php`, `<?=`, or `<? ` opening tag after `i` ends.
fn code_start(source: &str, i: usize) -> Option<usize> {
    let mut from = i;
    while let Some(found) = source[from..].find("<?") {
        let at = from + found;
        let rest = &source[at + 2..];
        if rest.len() >= 3 && rest[..3].eq_ignore_ascii_case("php") {
            return Some(at + 5);
        }
        if rest.starts_with('=') {
            return Some(at + 3);
        }
        if rest.starts_with(|c: char| c.is_ascii_whitespace()) {
            return Some(at + 2);
        }
        from = at + 2;
    }
    None
}

/// Splits PHP source into tokens. An unterminated comment or literal is an
/// error.
fn lex(source: &str) -> Result<Vec<Tok>, ExtractError> {
    let bytes = source.as_bytes();
    let mut tokens = Vec::new();
    let Some(mut i) = code_start(source, 0) else {
        return Ok(tokens);
    };
    while i < bytes.len() {
        let start = i;
        let rest = &source[i..];
        let byte = bytes[i];
        let kind = if byte.is_ascii_whitespace() {
            i += 1;
            continue;
        } else if rest.starts_with("?>") {
            i += 2;
            tokens.push(Tok {
                kind: Kind::Semi,
                start,
                end: i,
            });
            match code_start(source, i) {
                Some(next) => i = next,
                None => break,
            }
            continue;
        } else if rest.starts_with("#[") {
            i += 2;
            Kind::Open('[')
        } else if rest.starts_with("//") || byte == b'#' {
            let line = rest.find('\n').unwrap_or(rest.len());
            i += rest[..line].find("?>").unwrap_or(line);
            Kind::Comment
        } else if let Some(body) = rest.strip_prefix("/*") {
            let close = body
                .find("*/")
                .ok_or_else(|| ExtractError::new(start, "unterminated comment"))?;
            i += close + 4;
            Kind::Comment
        } else if rest.starts_with("<<<") {
            i = heredoc_end(source, i)
                .ok_or_else(|| ExtractError::new(start, "unterminated heredoc"))?;
            Kind::Literal
        } else if matches!(byte, b'\'' | b'"' | b'`') {
            i = string_end(bytes, i)
                .ok_or_else(|| ExtractError::new(start, "unterminated string"))?;
            Kind::Literal
        } else if byte.is_ascii_digit() {
            i += 1;
            while i < bytes.len()
                && (bytes[i].is_ascii_alphanumeric()
                    || bytes[i] == b'_'
                    || (bytes[i] == b'.' && bytes.get(i + 1).is_some_and(u8::is_ascii_digit)))
            {
                i += 1;
            }
            Kind::Literal
        } else if is_word_char(byte)
            || (byte == b'$' && bytes.get(i + 1).copied().is_some_and(is_word_char))
        {
            i += 1;
            while i < bytes.len() && is_word_char(bytes[i]) {
                i += 1;
            }
            Kind::Word
        } else {
            i += 1;
            match byte {
                b'(' | b'[' | b'{' => Kind::Open(byte as char),
                b')' | b']' | b'}' => Kind::Close(byte as char),
                b';' => Kind::Semi,
                _ => Kind::Op,
            }
        };
        tokens.push(Tok {
            kind,
            start,
            end: i,
        });
    }
    Ok(tokens)
}

/// One past the end of the `'`, `"`, or `` ` `` string at `i`.
/// Interpolations (`{$a['}']}`, `${a}`) are read as code.
fn string_end(bytes: &[u8], i: usize) -> Option<usize> {
    let quote = bytes[i];
    let mut j = i + 1;
    while j < bytes.len() {
        match bytes[j] {
            b'\\' => j += 2,
            byte if byte == quote => return Some(j + 1),
            b'{' if quote != b'\'' && bytes.get(j + 1) == Some(&b'$') => {
                j = interpolation_end(bytes, j + 1)?
            }
            b'$' if quote != b'\'' && bytes.get(j + 1) == Some(&b'{') => {
                j = interpolation_end(bytes, j + 2)?
            }
            _ => j += 1,
        }
    }
    None
}

/// One past the `}` closing an interpolation whose code starts at `i`.
fn interpolation_end(bytes: &[u8], mut i: usize) -> Option<usize> {
    let mut depth = 1usize;
    while i < bytes.len() {
        match bytes[i] {
            b'\'' | b'"' => i = string_end(bytes, i)?,
            b'{' => {
                depth += 1;
                i += 1;
            }
            b'}' => {
                depth -= 1;
                i += 1;
                if depth == 0 {
                    return Some(i);
                }
            }
            _ => i += 1,
        }
    }
    None
}

/// One past the closing identifier of the heredoc or nowdoc at `i`
/// (`<<<ID`, `<<<"ID"`, `<<<'ID'`), which may be indented.
fn heredoc_end(source: &str, i: usize) -> Option<usize> {
    let header_end = i + source[i..].find('\n')?;
    let label = source[i + 3..header_end]
        .trim()
        .trim_matches(|c| c == '"' || c == '\'');
    if label.is_empty() {
        return None;
    }
    let mut line_start = header_end + 1;
    while line_start <= source.len() {
        let line_end = source[line_start..]
            .find('\n')
            .map_or(source.len(), |n| line_start + n);
        let line = &source[line_start..line_end];
        let indent = line.len() - line.trim_start().len();
        if let Some(after) = line.trim_start().strip_prefix(label) {
            if !after.starts_with(|c: char| c == '_' || c.is_ascii_alphanumeric()) {
                return Some(line_start + indent + label.len());
            }
        }
        if line_end == source.len() {
            break;
        }
        line_start = line_end + 1;
    }
    None
}

const MODIFIERS: &[&str] = &[
    "abstract",
    "final",
    "public",
    "protected",
    "private",
    "static",
    "readonly",
    "var",
];

/// Statements that end after a `{}` body.
const BODIED: &[&str] = &[
    "function",
    "class",
    "interface",
    "trait",
    "enum",
    "namespace",
    "if",
    "for",
    "foreach",
    "while",
    "do",
    "switch",
    "try",
    "declare",
    "use",
];

/// What a statement declares.
enum Declares {
    Scope(Vec<String>, usize),
    /// `namespace A\B;`: the statements after it belong to `A\B`.
    Namespace(Vec<String>),
    Function(String),
    Other,
}

struct Reader<'a> {
    source: &'a str,
    tokens: &'a [Tok],
    partners: &'a [Option<usize>],
}

impl Reader<'_> {
    fn text(&self, index: usize) -> &str {
        &self.source[self.tokens[index].start..self.tokens[index].end]
    }

    fn kind(&self, index: usize) -> Option<Kind> {
        self.tokens.get(index).map(|token| token.kind)
    }

    fn is(&self, index: usize, text: &str) -> bool {
        index < self.tokens.len()
            && self.tokens[index].kind != Kind::Comment
            && self.text(index).eq_ignore_ascii_case(text)
    }

    fn partner(&self, index: usize) -> usize {
        self.partners[index].expect("brackets are balanced")
    }

    fn next_significant(&self, from: usize) -> Option<usize> {
        (from..self.tokens.len()).find(|&i| self.tokens[i].kind != Kind::Comment)
    }

    /// The first token of the statement at `start` after its `#[...]`
    /// attributes and modifiers.
    fn keyword(&self, start: usize) -> usize {
        let mut index = start;
        loop {
            let next = match self.kind(index) {
                Some(Kind::Open('[')) if self.text(index) == "#[" => self.partner(index) + 1,
                Some(Kind::Word) if MODIFIERS.iter().any(|m| self.is(index, m)) => index + 1,
                Some(Kind::Comment) => index + 1,
                _ => return index,
            };
            index = next;
        }
    }

    /// One past the last token of the statement or member starting at
    /// `start` (see the module documentation). A `}` closing the enclosing
    /// block also ends it.
    fn statement_end(&self, start: usize) -> usize {
        if self.kind(start) == Some(Kind::Open('{')) {
            return self.partner(start) + 1;
        }
        let keyword_index = self.keyword(start);
        let keyword = match self.kind(keyword_index) {
            Some(Kind::Word) => self.text(keyword_index).to_ascii_lowercase(),
            _ => String::new(),
        };
        let bodied = BODIED.contains(&keyword.as_str());
        let mut index = start;
        while index < self.tokens.len() {
            match self.tokens[index].kind {
                Kind::Close(_) => return index,
                Kind::Semi => return index + 1,
                Kind::Open('{') if bodied => {
                    let close = self.partner(index);
                    match self.next_significant(close + 1) {
                        Some(next)
                            if ["else", "elseif", "catch", "finally"]
                                .iter()
                                .any(|k| self.is(next, k))
                                || (keyword == "do" && self.is(next, "while")) =>
                        {
                            index = next;
                        }
                        Some(next) if keyword == "use" && self.kind(next) == Some(Kind::Semi) => {
                            return next + 1
                        }
                        _ => return close + 1,
                    }
                    continue;
                }
                Kind::Op if bodied && self.text(index) == ":" && self.opens_alternative(index) => {
                    index = self.alternative_end(index, &keyword);
                    continue;
                }
                Kind::Open(_) => index = self.partner(index),
                _ => {}
            }
            index += 1;
        }
        index
    }

    /// Whether the `:` at `index` opens the alternative syntax of a
    /// control statement (`if ($x):` ... `endif;`).
    fn opens_alternative(&self, index: usize) -> bool {
        (0..index)
            .rev()
            .find(|&i| self.tokens[i].kind != Kind::Comment)
            .is_some_and(|previous| {
                self.kind(previous) == Some(Kind::Close(')')) || self.is(previous, "else")
            })
    }

    /// The `end...` word closing the alternative syntax opened at `colon`.
    fn alternative_end(&self, colon: usize, keyword: &str) -> usize {
        let closing = match keyword {
            "if" => "endif",
            "for" => "endfor",
            "foreach" => "endforeach",
            "while" => "endwhile",
            "switch" => "endswitch",
            "declare" => "enddeclare",
            _ => return colon + 1,
        };
        let mut index = colon + 1;
        while index < self.tokens.len() {
            match self.tokens[index].kind {
                Kind::Open(_) => index = self.partner(index),
                Kind::Word if self.is(index, closing) => return index + 1,
                _ => {}
            }
            index += 1;
        }
        index
    }

    /// Collects the function definitions among the statements in
    /// `tokens[low..high]`, qualified by `scope`.
    fn statements(
        &self,
        low: usize,
        high: usize,
        scope: &mut Vec<String>,
        output: &mut Vec<Function>,
    ) {
        let restore = scope.len();
        let mut index = low;
        while let Some(start) = self.next_significant(index).filter(|&s| s < high) {
            if matches!(self.tokens[start].kind, Kind::Semi | Kind::Close(_)) {
                index = start + 1;
                continue;
            }
            let end = self.statement_end(start).clamp(start + 1, high);
            match self.declares(start, end) {
                Declares::Scope(name, open) => {
                    let inner = scope.len();
                    scope.extend(name);
                    self.statements(open + 1, self.partner(open), scope, output);
                    scope.truncate(inner);
                }
                Declares::Namespace(name) => {
                    scope.truncate(restore);
                    scope.extend(name);
                }
                Declares::Function(name) => {
                    let mut qualified = scope.clone();
                    qualified.push(name);
                    output.push(Function {
                        name: qualified,
                        start: self.tokens[start].start,
                        end: self.tokens[end - 1].end,
                    });
                }
                Declares::Other => {}
            }
            index = end;
        }
        scope.truncate(restore);
    }

    /// The first top-level `{` in `tokens[from..end]`.
    fn body(&self, from: usize, end: usize) -> Option<usize> {
        let mut index = from;
        while index < end {
            match self.tokens[index].kind {
                Kind::Open('{') => return Some(index),
                Kind::Open(_) => index = self.partner(index),
                _ => {}
            }
            index += 1;
        }
        None
    }

    /// What the statement `tokens[start..end]` declares.
    fn declares(&self, start: usize, end: usize) -> Declares {
        let keyword = self.keyword(start);
        if keyword >= end || self.kind(keyword) != Some(Kind::Word) {
            return Declares::Other;
        }
        let word = self.text(keyword).to_ascii_lowercase();
        match word.as_str() {
            "function" => {
                let mut name = keyword + 1;
                if self.is(name, "&") {
                    name += 1;
                }
                if self.kind(name) == Some(Kind::Word)
                    && self.kind(name + 1) == Some(Kind::Open('('))
                    && self.body(name, end).is_some()
                {
                    Declares::Function(self.text(name).to_owned())
                } else {
                    Declares::Other
                }
            }
            "class" | "interface" | "trait" | "enum" => {
                let name = keyword + 1;
                match (self.kind(name), self.body(name, end)) {
                    (Some(Kind::Word), Some(open)) => {
                        Declares::Scope(vec![self.text(name).to_owned()], open)
                    }
                    _ => Declares::Other,
                }
            }
            "namespace" => {
                let name = keyword + 1;
                let segments = match self.kind(name) {
                    Some(Kind::Word) => self
                        .text(name)
                        .split('\\')
                        .filter(|segment| !segment.is_empty())
                        .map(str::to_owned)
                        .collect(),
                    _ => Vec::new(),
                };
                match self.body(keyword, end) {
                    Some(open) => Declares::Scope(segments, open),
                    None => Declares::Namespace(segments),
                }
            }
            _ => Declares::Other,
        }
    }

    /// Every comment block that starts a line and is followed, on the next
    /// line, by a statement or member.
    fn annotated(&self) -> Vec<Annotated> {
        let spans: Vec<Span> = self
            .tokens
            .iter()
            .map(|token| Span {
                start: token.start,
                end: token.end,
                comment: token.kind == Kind::Comment,
            })
            .collect();
        let mut output = Vec::new();
        for Attached { first, last, next } in attached_comments(self.source, &spans) {
            if matches!(self.tokens[next].kind, Kind::Close(_) | Kind::Semi) {
                continue;
            }
            let end = self.statement_end(next).max(next + 1);
            output.push(Annotated {
                comment_start: self.tokens[first].start,
                lines: (first..=last)
                    .flat_map(|i| comment_lines(self.text(i)))
                    .collect(),
                construct: Ok((self.tokens[next].start, self.tokens[end - 1].end)),
            });
        }
        output
    }
}

/// The lines of a `//`, `#`, or `/* */` comment with the comment markers
/// removed (`/**` and the leading `*` of docblock lines included).
fn comment_lines(comment: &str) -> Vec<String> {
    if let Some(line) = comment
        .strip_prefix("//")
        .or_else(|| comment.strip_prefix('#'))
    {
        return vec![line.to_owned()];
    }
    let inner = comment
        .strip_prefix("/*")
        .and_then(|rest| rest.strip_suffix("*/"))
        .unwrap_or(comment);
    inner
        .lines()
        .map(|line| {
            let line = line.trim_start();
            line.strip_prefix('*').unwrap_or(line).to_owned()
        })
        .collect()
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

    fn locators(outcome: &vtest_adapter_api::DiscoveryOutcome) -> Vec<&str> {
        outcome
            .sources
            .iter()
            .map(|source| source.locator.value.as_str())
            .collect()
    }

    /// @vtest.id TEST-ADAPTER-PHP-DISCOVERS-FUNCTIONS-AND-TESTS
    /// @vtest.covers VO-ADAPTER-PHP-REGISTERS-WITHOUT-CORE-CHANGES, VO-ADAPTER-LEXICAL-OPAQUE-LOCATOR-SYNTAX
    /// @vtest.target crates/vtest-adapter-lexical/src/php.rs::Php::extract
    /// @vtest.intent verifies PHP functions and methods become namespace- and class-qualified Source Targets while abstract methods and closures do not, hash comments are not attributes, and an attributed PHPUnit method is a Test construct including its attributes
    #[test]
    fn discovers_php_functions_and_tests() {
        let outcome = discover_text(&Php, "src/Calc.php", SOURCE, "demo");
        assert!(outcome.diagnostics.is_empty(), "{:?}", outcome.diagnostics);
        assert_eq!(
            locators(&outcome),
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

    /// @vtest.id TEST-ADAPTER-PHP-ATTRIBUTE-BRACES-ARE-NOT-THE-BODY
    /// @vtest.covers VO-ADAPTER-PHP-REGISTERS-WITHOUT-CORE-CHANGES
    /// @vtest.target crates/vtest-adapter-lexical/src/php.rs::Reader::statement_end
    /// @vtest.intent verifies that closures, match arms, and interpolations inside attributes, default values, and strings are not taken for the body, so a Test construct runs to the end of the method body and an edit to only the body's tail changes its text, and that braced namespaces qualify their functions
    #[test]
    fn attribute_braces_are_not_the_body() {
        let source = |tail: &str| {
            format!(
                "<?php\nnamespace Demo {{\nclass PickTest\n{{\n    // @vtest.id TEST-PHP-PICK\n    // @vtest.covers VO-PHP-ADD\n    // @vtest.intent the whole body is the construct\n    #[TestWith([['a' => '}}']])]\n    public function testPick(array $o = [1, 2], ?\\Closure $f = null): void\n    {{\n        $x = match ($o) {{ default => \"{{$o['}}']}}\" }};\n        {tail}\n    }}\n\n    public function after() {{}}\n}}\n}}\n"
            )
        };
        let outcome = discover_text(&Php, "tests/PickTest.php", &source("one();"), "demo");
        assert!(outcome.diagnostics.is_empty(), "{:?}", outcome.diagnostics);
        assert_eq!(
            locators(&outcome),
            vec![
                "tests/PickTest.php::Demo::PickTest::testPick",
                "tests/PickTest.php::Demo::PickTest::after",
            ]
        );
        let test = &outcome.tests[0];
        assert!(test.construct_text.starts_with("#[TestWith("));
        assert!(test.construct_text.ends_with("one();\n    }"));
        let edited = discover_text(&Php, "tests/PickTest.php", &source("two();"), "demo");
        assert_ne!(edited.tests[0].construct_text, test.construct_text);
    }

    /// @vtest.id TEST-ADAPTER-PHP-PEST-HEREDOC-AND-UNREADABLE-FILES
    /// @vtest.covers VO-ADAPTER-PHP-REGISTERS-WITHOUT-CORE-CHANGES
    /// @vtest.target crates/vtest-adapter-lexical/src/php.rs::heredoc_end
    /// @vtest.intent verifies that an annotated Pest test call is a Test construct ending at its semicolon, that heredoc text and inline HTML are not code, and that an unterminated string makes the file unreadable
    #[test]
    fn pest_heredoc_and_unreadable_files() {
        let source = "<html>{ <?php\n// @vtest.id TEST-PHP-PEST\n// @vtest.covers VO-PHP-ADD\n// @vtest.intent a Pest test is a construct\ntest('adds', function () {\n    $text = <<<EOT\n    } not code {\n    EOT;\n    expect($text)->toBeString();\n});\n?>\n</html>}\n";
        let outcome = discover_text(&Php, "tests/AddTest.php", source, "demo");
        assert!(outcome.diagnostics.is_empty(), "{:?}", outcome.diagnostics);
        assert_eq!(outcome.tests.len(), 1);
        assert!(outcome.tests[0].construct_text.starts_with("test('adds'"));
        assert!(outcome.tests[0]
            .construct_text
            .ends_with("toBeString();\n});"));

        let broken = discover_text(&Php, "src/Broken.php", "<?php\n$x = 'open;\n", "demo");
        assert_eq!(broken.diagnostics.len(), 1);
        assert_eq!(broken.diagnostics[0].code.as_str(), "E-SCAN-001");
    }

    /// @vtest.id TEST-ADAPTER-PHP-OPEN-TAG-AND-STATEMENT-NAMESPACES
    /// @vtest.covers VO-ADAPTER-PHP-REGISTERS-WITHOUT-CORE-CHANGES
    /// @vtest.target crates/vtest-adapter-lexical/src/php.rs::Reader::statements
    /// @vtest.intent verifies that the opening tag is not part of the first function, so the SRC ID comment above it attaches, and that a statement-form namespace replaces the previous one instead of nesting in it
    #[test]
    fn open_tag_and_statement_namespaces() {
        let outcome = discover_text(
            &Php,
            "fixture.php",
            "<?php\n// @vtest.src-id SRC-X\nfunction f(){return 1;}\n",
            "demo",
        );
        assert!(outcome.diagnostics.is_empty(), "{:?}", outcome.diagnostics);
        assert_eq!(
            outcome.sources[0].src_id.as_ref().map(|id| id.as_str()),
            Some("SRC-X")
        );
        let outcome = discover_text(
            &Php,
            "fixture.php",
            "<?php\nnamespace A;\nfunction a() {}\nnamespace B;\nfunction b() {}\n",
            "demo",
        );
        assert_eq!(
            locators(&outcome),
            vec!["fixture.php::A::a", "fixture.php::B::b"]
        );
    }
}
