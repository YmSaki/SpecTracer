//! `cpp`: C++ sources and headers (`.cc`, `.cpp`, `.cxx`, `.c++`, `.h`,
//! `.hh`, `.hpp`, `.hxx`, `.h++`, `.ipp`, `.inl`, `.tpp`).
//!
//! A file is read in three steps, all specific to C++:
//!
//! 1. [`lex`] splits it into comments, preprocessor lines, literals (raw
//!    strings `R"x(...)x"` with their prefixes included), words (digit
//!    separators `1'000` included), brackets, `;`, and other punctuation.
//! 2. [`Reader::declaration_end`] finds where a declaration ends:
//!    - after the `{}` body of a function, namespace, or `extern "C"`
//!      block;
//!    - at the `;` after the body of a `class`, `struct`, `union`, or `enum`
//!      (which may be followed by declarators);
//!    - at the `;` of an initialized declaration, whose braces belong to the
//!      value (`int t[] = { 1 };`, `auto f = [] { };`, `int x{5};`);
//!    - for a constructor, after the body that follows its member
//!      initializer list, whose brace initializers (`x{0}`) are skipped;
//!    - for a lone macro invocation without `;` (`Q_OBJECT`), at its line
//!      break.
//! 3. [`Reader::declarations`] walks the declarations: `namespace` (nested
//!    names `a::b` included), `class`, `struct`, `union`, and `enum` bodies
//!    qualify the functions inside them, and `extern "C"` and anonymous
//!    namespace bodies are walked without a qualifier. A function
//!    definition is named by the (possibly `::`-qualified) name before its
//!    parameter list, so an out-of-line member `int Calc::f()` is
//!    `Calc::f`. Test macros with a body (`TEST(Suite, Name) { ... }`,
//!    `TEST_CASE("...") { ... }`) are functions named after the macro.

use crate::{
    shared::{attached_comments, pair_brackets, Attached, Span},
    Annotated, ExtractError, FileItems, Function, LanguageProfile, LexicalAdapter,
};

pub const ADAPTER_ID: &str = "cpp";

#[derive(Clone, Copy, Debug, Default)]
pub struct Cpp;

impl LanguageProfile for Cpp {
    fn adapter_id(&self) -> &'static str {
        ADAPTER_ID
    }

    fn languages(&self) -> &'static [&'static str] {
        &["cpp"]
    }

    fn extensions(&self) -> &'static [&'static str] {
        &[
            "cc", "cpp", "cxx", "c++", "h", "hh", "hpp", "hxx", "h++", "ipp", "inl", "tpp",
        ]
    }

    fn excluded_dirs(&self) -> &'static [&'static str] {
        &["build", "cmake-build-debug", "cmake-build-release"]
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
        reader.declarations(0, tokens.len(), &mut Vec::new(), &mut functions)?;
        Ok(FileItems {
            functions,
            annotated: reader.annotated(),
        })
    }
}

/// The `cpp` adapter.
pub fn adapter() -> LexicalAdapter<Cpp> {
    LexicalAdapter::new(Cpp)
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Kind {
    Comment,
    Directive,
    Literal,
    Word,
    Open(char),
    Close(char),
    Semi,
    Punct(char),
}

impl Kind {
    fn is_trivia(self) -> bool {
        matches!(self, Kind::Comment | Kind::Directive)
    }
}

#[derive(Clone, Copy, Debug)]
struct Tok {
    kind: Kind,
    start: usize,
    end: usize,
}

/// Splits C++ source into tokens. An unterminated comment or literal is an
/// error.
fn lex(source: &str) -> Result<Vec<Tok>, ExtractError> {
    let bytes = source.as_bytes();
    let mut tokens = Vec::new();
    let mut i = 0;
    let mut line_start = true;
    while i < bytes.len() {
        let start = i;
        let rest = &source[i..];
        if bytes[i].is_ascii_whitespace() {
            line_start |= bytes[i] == b'\n';
            i += 1;
            continue;
        }
        let at_line_start = std::mem::replace(&mut line_start, false);
        let kind = if at_line_start && bytes[i] == b'#' {
            i = directive_end(source, i);
            Kind::Directive
        } else if rest.starts_with("//") {
            i += rest.find('\n').unwrap_or(rest.len());
            Kind::Comment
        } else if let Some(body) = rest.strip_prefix("/*") {
            let close = body
                .find("*/")
                .ok_or_else(|| ExtractError::new(start, "unterminated comment"))?;
            i += close + 4;
            Kind::Comment
        } else if let Some(end) = raw_string_end(source, i) {
            i = end.ok_or_else(|| ExtractError::new(start, "unterminated raw string"))?;
            Kind::Literal
        } else if bytes[i] == b'"' || bytes[i] == b'\'' {
            i = quoted_end(source, i + 1, bytes[i])
                .ok_or_else(|| ExtractError::new(start, "unterminated literal"))?;
            Kind::Literal
        } else if bytes[i].is_ascii_alphanumeric() || bytes[i] == b'_' || !bytes[i].is_ascii() {
            let digits = bytes[i].is_ascii_digit();
            let mut end = i;
            for (offset, c) in rest.char_indices() {
                let separator = digits
                    && c == '\''
                    && rest[offset + 1..].starts_with(|n: char| n.is_ascii_alphanumeric());
                if !(c.is_alphanumeric() || c == '_' || separator) {
                    break;
                }
                end = i + offset + c.len_utf8();
            }
            i = end.max(i + 1);
            Kind::Word
        } else {
            i += 1;
            match bytes[start] {
                b'(' | b'[' | b'{' => Kind::Open(bytes[start] as char),
                b')' | b']' | b'}' => Kind::Close(bytes[start] as char),
                b';' => Kind::Semi,
                other => Kind::Punct(other as char),
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

/// When a raw string `R"delim(...)delim"` (optionally prefixed by `u8`,
/// `u`, `U`, or `L`) starts at `start`, its end, or `Some(None)` when it is
/// unterminated.
fn raw_string_end(source: &str, start: usize) -> Option<Option<usize>> {
    let rest = &source[start..];
    let after_prefix = ["u8R\"", "uR\"", "UR\"", "LR\"", "R\""]
        .iter()
        .find(|prefix| rest.starts_with(*prefix))
        .map(|prefix| start + prefix.len())?;
    if start > 0 {
        let previous = source.as_bytes()[start - 1];
        if previous.is_ascii_alphanumeric() || previous == b'_' {
            return None;
        }
    }
    let Some(paren) = source[after_prefix..].find('(') else {
        return Some(None);
    };
    let delimiter = &source[after_prefix..after_prefix + paren];
    let close = format!("){delimiter}\"");
    let body = after_prefix + paren + 1;
    Some(
        source[body..]
            .find(&close)
            .map(|offset| body + offset + close.len()),
    )
}

fn directive_end(source: &str, start: usize) -> usize {
    let bytes = source.as_bytes();
    let mut i = start;
    while i < bytes.len() {
        if bytes[i] == b'\n' && !source[..i].trim_end_matches('\r').ends_with('\\') {
            return i;
        }
        i += 1;
    }
    i
}

fn quoted_end(source: &str, from: usize, quote: u8) -> Option<usize> {
    let bytes = source.as_bytes();
    let mut i = from;
    while i < bytes.len() {
        match bytes[i] {
            b'\\' => i += 1,
            b'\n' => return None,
            byte if byte == quote => return Some(i + 1),
            _ => {}
        }
        i += 1;
    }
    None
}

const AGGREGATES: &[&str] = &["class", "struct", "union", "enum"];

/// A declaration: one past its last token, the `{` of its body, and the
/// first top-level `(` before the body.
struct Declaration {
    end: usize,
    body: Option<usize>,
    paren: Option<usize>,
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

    fn partner(&self, index: usize) -> usize {
        self.partners[index].expect("brackets are balanced")
    }

    fn touching(&self, a: usize, b: usize) -> bool {
        self.tokens[a].end == self.tokens[b].start
    }

    fn previous_significant(&self, start: usize, index: usize) -> Option<usize> {
        (start..index)
            .rev()
            .find(|&i| !self.tokens[i].kind.is_trivia())
    }

    fn next_significant(&self, from: usize) -> Option<usize> {
        (from..self.tokens.len()).find(|&i| !self.tokens[i].kind.is_trivia())
    }

    fn line_breaks(&self, before: usize, after: usize) -> usize {
        self.source[self.tokens[before].end..self.tokens[after].start]
            .matches('\n')
            .count()
    }

    /// Whether the `:` at `index` is a lone colon (not part of `::`).
    fn lone_colon(&self, index: usize) -> bool {
        let part_of = |other: usize| {
            self.kind(other) == Some(Kind::Punct(':'))
                && (self.touching(index, other) || self.touching(other, index))
        };
        !(index > 0 && part_of(index - 1)) && !part_of(index + 1)
    }

    /// The first token of the declaration at `start` after access
    /// specifiers (`public:`), which end no declaration.
    fn skip_access(&self, mut start: usize) -> usize {
        while matches!(self.kind(start), Some(Kind::Word))
            && matches!(self.text(start), "public" | "private" | "protected")
            && self.kind(start + 1) == Some(Kind::Punct(':'))
            && self.lone_colon(start + 1)
        {
            start = self
                .next_significant(start + 2)
                .unwrap_or(self.tokens.len());
        }
        start
    }

    /// Where the declaration starting at `start` ends (see the module
    /// documentation).
    fn declaration_end(&self, start: usize) -> Result<Declaration, String> {
        let mut initialized = false;
        let mut init_list = false;
        let mut aggregate = false;
        let mut body = None;
        let mut paren = None;
        let mut index = start;
        while index < self.tokens.len() {
            match self.tokens[index].kind {
                Kind::Open('{') if !initialized && body.is_none() => {
                    let close = self.partner(index);
                    let previous = self.previous_significant(start, index);
                    let after_name = previous.is_some_and(|p| {
                        self.kind(p) == Some(Kind::Word) || self.kind(p) == Some(Kind::Punct('>'))
                    });
                    let brace_init = after_name
                        && (init_list
                            || (paren.is_none()
                                && !aggregate
                                && !self.is_scope_header(start, index)));
                    if brace_init {
                        index = close + 1;
                        continue;
                    }
                    body = Some(index);
                    if aggregate && paren.is_none() {
                        index = close + 1;
                        continue;
                    }
                    let mut end = close + 1;
                    if self.kind(end) == Some(Kind::Semi) {
                        end += 1;
                    }
                    return Ok(Declaration { end, body, paren });
                }
                Kind::Open(open) => {
                    if open == '(' && body.is_none() && !initialized && !init_list {
                        paren.get_or_insert(index);
                    }
                    let close = self.partner(index);
                    if self.is_macro_line(start, close) {
                        return Ok(Declaration {
                            end: close + 1,
                            body: None,
                            paren: None,
                        });
                    }
                    index = close + 1;
                    continue;
                }
                Kind::Close(_) => break,
                Kind::Semi => {
                    return Ok(Declaration {
                        end: index + 1,
                        body,
                        paren,
                    })
                }
                Kind::Punct('=') if body.is_none() => {
                    let comparison = index.checked_sub(1).is_some_and(|p| {
                        self.touching(p, index) && matches!(self.text(p), "=" | "!" | "<" | ">")
                    }) || (self.kind(index + 1) == Some(Kind::Punct('='))
                        && self.touching(index, index + 1));
                    let operator_name = self
                        .previous_significant(start, index)
                        .is_some_and(|p| self.text(p) == "operator");
                    initialized |= !comparison && !operator_name;
                }
                Kind::Punct(':') if body.is_none() && self.lone_colon(index) => {
                    let after_params = self
                        .previous_significant(start, index)
                        .is_some_and(|p| self.kind(p) == Some(Kind::Close(')')));
                    init_list |= after_params;
                }
                Kind::Word => {
                    if AGGREGATES.contains(&self.text(index)) && paren.is_none() {
                        aggregate = true;
                    }
                    if self.is_macro_line(start, index) {
                        return Ok(Declaration {
                            end: index + 1,
                            body: None,
                            paren: None,
                        });
                    }
                }
                _ => {}
            }
            index += 1;
        }
        Err("the declaration has neither a body nor a `;`".to_owned())
    }

    /// Whether `tokens[start..index]` opens a named block: a namespace or
    /// an `extern "C"` block.
    fn is_scope_header(&self, start: usize, index: usize) -> bool {
        (start..index).any(|i| {
            self.tokens[i].kind == Kind::Word && matches!(self.text(i), "namespace" | "extern")
        })
    }

    /// Whether `tokens[start..=last]` is a lone macro invocation (`NAME` or
    /// `NAME(...)`) that ends its line without `;` or a body.
    fn is_macro_line(&self, start: usize, last: usize) -> bool {
        let shape = if last == start {
            self.tokens[start].kind == Kind::Word
        } else {
            self.tokens[start].kind == Kind::Word
                && last > start + 1
                && self.tokens[start + 1].kind == Kind::Open('(')
                && self.partner(start + 1) == last
        };
        if !shape || (last == start && AGGREGATES.contains(&self.text(start))) {
            return false;
        }
        match self.next_significant(last + 1) {
            None => true,
            Some(next) => {
                self.line_breaks(last, next) >= 1
                    && !matches!(
                        self.tokens[next].kind,
                        Kind::Open(_) | Kind::Semi | Kind::Punct(_)
                    )
                    && !(last == start && self.tokens[next].kind == Kind::Word)
            }
        }
    }

    /// Collects the function definitions among the declarations in
    /// `tokens[low..high]`, qualified by `scope`.
    fn declarations(
        &self,
        low: usize,
        high: usize,
        scope: &mut Vec<String>,
        output: &mut Vec<Function>,
    ) -> Result<(), ExtractError> {
        let mut index = low;
        while index < high {
            index = self.skip_access(index);
            if index >= high {
                break;
            }
            if self.tokens[index].kind.is_trivia() || self.tokens[index].kind == Kind::Semi {
                index += 1;
                continue;
            }
            let declaration = self
                .declaration_end(index)
                .map_err(|message| ExtractError::new(self.tokens[index].start, &message))?;
            let header_end = declaration.body.unwrap_or(declaration.end);
            match self.declares(index, header_end, &declaration) {
                Declares::Scope(name) => {
                    if let Some(open) = declaration.body {
                        let restore = scope.len();
                        scope.extend(name);
                        self.declarations(open + 1, self.partner(open), scope, output)?;
                        scope.truncate(restore);
                    }
                }
                Declares::Function(name) => {
                    let mut qualified = scope.clone();
                    qualified.extend(name);
                    output.push(Function {
                        name: qualified,
                        start: self.tokens[index].start,
                        end: self.tokens[declaration.end - 1].end,
                    });
                }
                Declares::Other => {}
            }
            index = declaration.end;
        }
        Ok(())
    }

    /// What the declaration header `tokens[start..end]` declares.
    fn declares(&self, start: usize, end: usize, declaration: &Declaration) -> Declares {
        let start = self.skip_template(start, end);
        let first_word = (start..end).find(|&i| self.tokens[i].kind == Kind::Word);
        if let Some(first) = first_word {
            match self.text(first) {
                "namespace" => {
                    let name = self.qualified_name(first + 1, end);
                    return Declares::Scope(name);
                }
                "extern" if self.kind(first + 1) == Some(Kind::Literal) => {
                    return Declares::Scope(Vec::new());
                }
                _ => {}
            }
        }
        if declaration.paren.is_none() {
            let keyword = (start..end)
                .find(|&i| self.tokens[i].kind == Kind::Word && AGGREGATES.contains(&self.text(i)));
            if let Some(keyword) = keyword {
                let mut name = keyword + 1;
                while name < end
                    && self.tokens[name].kind == Kind::Word
                    && matches!(self.text(name), "class" | "struct")
                {
                    name += 1;
                }
                let segments = self.qualified_name(name, end);
                return if segments.is_empty() {
                    Declares::Other
                } else {
                    Declares::Scope(segments)
                };
            }
            return Declares::Other;
        }
        let (Some(_), Some(paren)) = (declaration.body, declaration.paren) else {
            return Declares::Other;
        };
        // `x = f(...) {` is a value; `operator==(...)` names an operator.
        let operator = (start..paren).find(|&i| self.text(i) == "operator");
        if (start..operator.unwrap_or(paren)).any(|i| self.tokens[i].kind == Kind::Punct('=')) {
            return Declares::Other;
        }
        self.function_name(start, paren)
            .map_or(Declares::Other, Declares::Function)
    }

    /// The first token after `template <...>` prefixes.
    fn skip_template(&self, mut start: usize, end: usize) -> usize {
        while start + 1 < end
            && self.tokens[start].kind == Kind::Word
            && self.text(start) == "template"
            && self.tokens[start + 1].kind == Kind::Punct('<')
        {
            let mut depth = 0isize;
            let mut index = start + 1;
            while index < end {
                match self.tokens[index].kind {
                    Kind::Punct('<') => depth += 1,
                    Kind::Punct('>') => depth -= 1,
                    Kind::Open(_) => index = self.partner(index),
                    _ => {}
                }
                index += 1;
                if depth == 0 {
                    break;
                }
            }
            start = index;
        }
        start
    }

    /// `a::b::c` starting at `index`, as segments.
    fn qualified_name(&self, mut index: usize, end: usize) -> Vec<String> {
        let mut segments = Vec::new();
        while index < end && self.tokens[index].kind == Kind::Word {
            segments.push(self.text(index).to_owned());
            if index + 3 < end
                && self.tokens[index + 1].kind == Kind::Punct(':')
                && self.tokens[index + 2].kind == Kind::Punct(':')
            {
                index += 3;
            } else {
                break;
            }
        }
        segments
    }

    /// The name before the parameter list at `paren`, with its `::`
    /// qualifiers: `add`, `Calc::add`, `~Calc`, `operator==`,
    /// `operator()`, or `max_of` for `max_of<int>`.
    fn function_name(&self, start: usize, paren: usize) -> Option<Vec<String>> {
        // A grouped declarator (`int (*factory(void))(int)`, `T (&pick())[2]`):
        // the name is the one inside the group.
        let inner = self.next_significant(paren + 1)?;
        if matches!(self.tokens[inner].kind, Kind::Punct('*' | '&'))
            && !(paren > start && self.text(paren - 1) == "operator")
        {
            let close = self.partner(paren);
            let open = (inner..close).find(|&i| self.tokens[i].kind == Kind::Open('('))?;
            return self.function_name(inner + 1, open);
        }
        let mut name_end = paren;
        // `operator()(...)`: the first `()` is part of the name.
        if name_end > start
            && self.text(name_end - 1) == "operator"
            && self.partner(paren) == paren + 1
            && self.kind(paren + 2) == Some(Kind::Open('('))
        {
            return Some(self.qualify(start, name_end - 1, "operator()".to_owned()));
        }
        if name_end > start && self.tokens[name_end - 1].kind == Kind::Punct('>') {
            let mut depth = 0isize;
            while name_end > start {
                name_end -= 1;
                match self.tokens[name_end].kind {
                    Kind::Punct('>') => depth += 1,
                    Kind::Punct('<') => depth -= 1,
                    _ => {}
                }
                if depth == 0 {
                    break;
                }
            }
        }
        let mut symbols = name_end;
        while symbols > start && matches!(self.tokens[symbols - 1].kind, Kind::Punct(_)) {
            symbols -= 1;
        }
        if symbols > start && symbols < name_end && self.text(symbols - 1) == "operator" {
            let operator = (symbols..name_end)
                .map(|i| self.text(i))
                .collect::<String>();
            return Some(self.qualify(start, symbols - 1, format!("operator{operator}")));
        }
        let word = name_end.checked_sub(1).filter(|&w| w >= start)?;
        if self.tokens[word].kind != Kind::Word
            || matches!(
                self.text(word),
                "if" | "for" | "while" | "switch" | "return" | "sizeof" | "decltype" | "new"
            )
        {
            return None;
        }
        let mut first = word;
        let mut name = self.text(word).to_owned();
        if first > start && self.tokens[first - 1].kind == Kind::Punct('~') {
            first -= 1;
            name = format!("~{name}");
        }
        Some(self.qualify(start, first, name))
    }

    /// `name` prefixed by the `A::B::` qualifiers that end at `first`.
    fn qualify(&self, start: usize, first: usize, name: String) -> Vec<String> {
        let mut segments = vec![name];
        let mut index = first;
        while index >= start + 3
            && self.tokens[index - 1].kind == Kind::Punct(':')
            && self.tokens[index - 2].kind == Kind::Punct(':')
            && self.tokens[index - 3].kind == Kind::Word
        {
            segments.insert(0, self.text(index - 3).to_owned());
            index -= 3;
        }
        segments
    }

    /// Every comment block that starts its own line and is followed, with
    /// no blank line between, by a declaration, together with that
    /// declaration's extent. Consecutive comments separated by at most one
    /// line break form one block.
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
            if matches!(
                self.tokens[next].kind,
                Kind::Close(_) | Kind::Semi | Kind::Directive
            ) {
                continue;
            }
            let construct = self.declaration_end(next).map(|declaration| {
                (
                    self.tokens[next].start,
                    self.tokens[declaration.end - 1].end,
                )
            });
            output.push(Annotated {
                comment_start: self.tokens[first].start,
                lines: (first..=last)
                    .flat_map(|i| comment_lines(self.text(i)))
                    .collect(),
                construct,
            });
        }
        output
    }
}

enum Declares {
    Scope(Vec<String>),
    Function(Vec<String>),
    Other,
}

/// The lines of a `//` or `/* */` comment with the comment markers removed.
fn comment_lines(comment: &str) -> Vec<String> {
    if let Some(line) = comment.strip_prefix("//") {
        return vec![line.trim_start_matches(['/', '!']).to_owned()];
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

    const SOURCE: &str = r#"#include <gtest/gtest.h>

namespace demo::math {

class Calc {
public:
    Calc() : base_(0) {}
    // @vtest.src-id SRC-CPP-ADD
    int add(int a, int b) const { return a + b; }
    bool operator==(const Calc& other) const { return base_ == other.base_; }
    int declared_only(int a);
    Calc(const Calc&) = default;
private:
    int base_{1'000};
};

int Calc::declared_only(int a) { return a; }

template <class T>
T max_of(T a, T b) {
    auto pick = [](T x, T y) { return x > y ? x : y; };
    return pick(a, b);
}

const char* kRaw = R"json({"not": "code"})json";

}  // namespace demo::math

extern "C" {
int c_entry(void) { return 0; }
}

// @vtest.id TEST-CPP-ADD
// @vtest.covers VO-CPP-ADD
// @vtest.target src/calc.cpp::demo::math::Calc::add
// @vtest.intent adding two numbers returns their sum
TEST(CalcTest, Adds) {
    EXPECT_EQ(demo::math::Calc().add(1, 2), 3);
}

TEST(CalcTest, Other) {}
"#;

    /// @vtest.id TEST-ADAPTER-CPP-DISCOVERS-FUNCTIONS-AND-TESTS
    /// @vtest.covers VO-ADAPTER-CPP-REGISTERS-WITHOUT-CORE-CHANGES, VO-ADAPTER-LEXICAL-OPAQUE-LOCATOR-SYNTAX
    /// @vtest.target crates/vtest-adapter-lexical/src/cpp.rs::Cpp::extract
    /// @vtest.intent verifies C++ member, out-of-line, operator, template, and extern "C" function definitions become namespace- and class-qualified Source Targets while declarations, defaulted members, lambdas, and raw strings do not, and an annotated GoogleTest macro block is a Test construct
    #[test]
    fn discovers_cpp_functions_and_macro_tests() {
        let outcome = discover_text(&Cpp, "src/calc.cpp", SOURCE, "demo");
        assert!(outcome.diagnostics.is_empty(), "{:?}", outcome.diagnostics);
        let sources = outcome
            .sources
            .iter()
            .map(|source| source.locator.value.as_str())
            .collect::<Vec<_>>();
        assert_eq!(
            sources,
            vec![
                "src/calc.cpp::demo::math::Calc::Calc",
                "src/calc.cpp::demo::math::Calc::add",
                "src/calc.cpp::demo::math::Calc::operator==",
                "src/calc.cpp::demo::math::Calc::declared_only",
                "src/calc.cpp::demo::math::max_of",
                "src/calc.cpp::c_entry",
                "src/calc.cpp::TEST",
                "src/calc.cpp::TEST",
            ]
        );
        assert_eq!(
            outcome.sources[1].src_id.as_ref().map(|id| id.as_str()),
            Some("SRC-CPP-ADD")
        );
        assert_eq!(outcome.tests.len(), 1);
        let test = &outcome.tests[0];
        assert_eq!(test.id.as_str(), "TEST-CPP-ADD");
        assert_eq!(test.location.locator, "TEST#1");
        assert!(test.construct_text.starts_with("TEST(CalcTest, Adds) {"));
        assert!(test.construct_text.ends_with("3);\n}"));
    }

    /// @vtest.id TEST-ADAPTER-CPP-INITIALIZER-LIST-IS-NOT-THE-BODY
    /// @vtest.covers VO-ADAPTER-LEXICAL-RETURNS-SOURCE-RANGE-AND-BYTES
    /// @vtest.target crates/vtest-adapter-lexical/src/cpp.rs::Reader::declaration_end
    /// @vtest.intent verifies brace initializers in a constructor's member initializer list do not end the Test construct, so it runs to the end of the constructor body, and a change to only the end of the body changes the construct text
    #[test]
    fn initializer_list_braces_are_not_the_body() {
        let declare = "struct C {\n  // @vtest.id TEST-X\n  // @vtest.covers VO-X\n  // @vtest.intent check\n";
        let test = |body: &str| {
            let source = format!("{declare}  {body}\n  int x;\n}};\n");
            let outcome = discover_text(&Cpp, "t.cpp", &source, "demo");
            assert!(outcome.diagnostics.is_empty(), "{:?}", outcome.diagnostics);
            outcome.tests[0].construct_text.clone()
        };
        assert_eq!(test("C() : x{0} { marker(); }"), "C() : x{0} { marker(); }");
        assert_eq!(
            test("C(int a) : x{a}, y(a) { marker(); }"),
            "C(int a) : x{a}, y(a) { marker(); }"
        );
        assert_ne!(
            test("C() : x{0} { marker(); one(); }"),
            test("C() : x{0} { marker(); two(); }")
        );
    }

    /// @vtest.id TEST-ADAPTER-CPP-POINTER-RETURNING-FUNCTION-NAME
    /// @vtest.covers VO-ADAPTER-CPP-REGISTERS-WITHOUT-CORE-CHANGES
    /// @vtest.target crates/vtest-adapter-lexical/src/cpp.rs::Reader::function_name
    /// @vtest.intent verifies that a function returning a function pointer or an array reference is named by the identifier inside its grouped declarator, and that an access specifier before a method does not detach its SRC ID declaration
    #[test]
    fn grouped_declarators_and_access_specifiers() {
        let source = "int (*factory(void))(int) { return &f; }\nint (&pick())[2] { return a; }\nclass C {\npublic:\n// @vtest.src-id SRC-X\nint f(){return 1;}\n};\n";
        let outcome = discover_text(&Cpp, "fixture.cpp", source, "demo");
        assert!(outcome.diagnostics.is_empty(), "{:?}", outcome.diagnostics);
        let sources = outcome
            .sources
            .iter()
            .map(|source| {
                (
                    source.locator.value.as_str(),
                    source.src_id.as_ref().map(|id| id.as_str()),
                )
            })
            .collect::<Vec<_>>();
        assert_eq!(
            sources,
            vec![
                ("fixture.cpp::factory", None),
                ("fixture.cpp::pick", None),
                ("fixture.cpp::C::f", Some("SRC-X")),
            ]
        );
    }
}
