//! Helpers that classify a declaration header.
//!
//! A header is the code of one statement before its body (`{`), with
//! comments, directives, and newlines removed. Each language profile decides
//! from its header whether the statement is a function definition (a Source
//! Target), a named scope that qualifies the functions inside it (a class,
//! namespace, ...), or anything else.

use crate::lexer::TokKind;

/// One significant token of a header.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct HTok<'a> {
    pub kind: TokKind,
    pub text: &'a str,
}

impl HTok<'_> {
    pub fn is_word(&self, word: &str) -> bool {
        self.kind == TokKind::Word && self.text == word
    }

    pub fn is_any_word(&self) -> bool {
        self.kind == TokKind::Word
    }

    pub fn is_punct(&self, character: char) -> bool {
        self.kind == TokKind::Punct(character)
    }

    pub fn is_open(&self, character: char) -> bool {
        self.kind == TokKind::Open(character)
    }
}

/// What a header declares.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum HeaderKind {
    /// A function definition, named by one or more qualified segments
    /// (`["Calculator", "add"]` for C++'s `Calculator::add`).
    Function(Vec<String>),
    /// A named scope whose body qualifies the functions inside it.
    Scope(Vec<String>),
    /// A named scope that applies to the rest of the enclosing body
    /// (C#'s file-scoped `namespace X;`, PHP's `namespace X;`).
    ScopeRest(Vec<String>),
    Other,
}

/// The index of the token closing the bracket opened at `open`.
pub fn matching_close(header: &[HTok<'_>], open: usize) -> Option<usize> {
    let mut depth = 0usize;
    for (index, token) in header.iter().enumerate().skip(open) {
        match token.kind {
            TokKind::Open(_) => depth += 1,
            TokKind::Close(_) => {
                depth = depth.checked_sub(1)?;
                if depth == 0 {
                    return Some(index);
                }
            }
            _ => {}
        }
    }
    None
}

/// Which leading attribute forms a language uses.
#[derive(Clone, Copy, Debug, Default)]
pub struct AttributeForms {
    /// `@Name`, `@a.b.Name(...)` (Java, Kotlin, Swift, TypeScript, PHP docs).
    pub at_sign: bool,
    /// `#[...]` (PHP 8).
    pub hash_bracket: bool,
    /// `[...]` / `[[...]]` (C#, C++).
    pub bracket: bool,
}

/// The index of the first header token after any leading attributes.
pub fn skip_attributes(header: &[HTok<'_>], forms: AttributeForms) -> usize {
    let mut index = 0;
    loop {
        let Some(token) = header.get(index) else {
            return index;
        };
        if forms.at_sign
            && token.is_punct('@')
            && header.get(index + 1).is_some_and(HTok::is_any_word)
            && !header[index + 1].is_word("interface")
        {
            index += 2;
            while header.get(index).is_some_and(|t| t.is_punct('.'))
                && header.get(index + 1).is_some_and(HTok::is_any_word)
            {
                index += 2;
            }
            if header.get(index).is_some_and(|t| t.is_open('(')) {
                match matching_close(header, index) {
                    Some(close) => index = close + 1,
                    None => return index,
                }
            }
            continue;
        }
        if forms.hash_bracket
            && token.is_punct('#')
            && header.get(index + 1).is_some_and(|t| t.is_open('['))
        {
            match matching_close(header, index + 1) {
                Some(close) => {
                    index = close + 1;
                    continue;
                }
                None => return index,
            }
        }
        if forms.bracket && token.is_open('[') {
            match matching_close(header, index) {
                Some(close) => {
                    index = close + 1;
                    continue;
                }
                None => return index,
            }
        }
        return index;
    }
}

/// The index of the first `(` that is not nested in another bracket.
pub fn first_paren(header: &[HTok<'_>], from: usize) -> Option<usize> {
    let mut depth = 0usize;
    for (index, token) in header.iter().enumerate().skip(from) {
        match token.kind {
            TokKind::Open('(') if depth == 0 => return Some(index),
            TokKind::Open(_) => depth += 1,
            TokKind::Close(_) => depth = depth.saturating_sub(1),
            _ => {}
        }
    }
    None
}

/// Whether a top-level (not bracket-nested) token in `header[from..to]`
/// satisfies `predicate`.
pub fn any_top_level(
    header: &[HTok<'_>],
    from: usize,
    to: usize,
    predicate: impl Fn(&HTok<'_>) -> bool,
) -> bool {
    let mut depth = 0usize;
    for token in header.iter().take(to).skip(from) {
        match token.kind {
            TokKind::Open(_) => depth += 1,
            TokKind::Close(_) => depth = depth.saturating_sub(1),
            _ if depth == 0 && predicate(token) => return true,
            _ => {}
        }
    }
    false
}

/// The index of the first top-level word of `words` in `header[from..to]`.
pub fn find_top_level_word(
    header: &[HTok<'_>],
    from: usize,
    to: usize,
    words: &[&str],
) -> Option<usize> {
    let mut depth = 0usize;
    for (index, token) in header.iter().enumerate().take(to).skip(from) {
        match token.kind {
            TokKind::Open(_) => depth += 1,
            TokKind::Close(_) => depth = depth.saturating_sub(1),
            TokKind::Word if depth == 0 && words.contains(&token.text) => return Some(index),
            _ => {}
        }
    }
    None
}

/// The index just past a balanced `<...>` starting at `open` (a generic
/// parameter list), or `open` itself when there is none.
pub fn skip_angle_brackets(header: &[HTok<'_>], open: usize) -> usize {
    if !header.get(open).is_some_and(|t| t.is_punct('<')) {
        return open;
    }
    let mut depth = 0usize;
    for (index, token) in header.iter().enumerate().skip(open) {
        if token.is_punct('<') {
            depth += 1;
        } else if token.is_punct('>') {
            depth -= 1;
            if depth == 0 {
                return index + 1;
            }
        }
    }
    open
}

/// The name declared immediately before the token at `paren` (usually a
/// parameter list's `(`), as qualified segments, together with the index of
/// its first token. A balanced generic list (`f<T>(`) between the name and
/// `paren` is skipped; `::`-qualified (`A::B::f`) and `.`-qualified
/// (`A.f`) names keep every segment; `operator==` is named `operator==`;
/// `~Foo` is named `~Foo`.
pub fn name_before(header: &[HTok<'_>], paren: usize) -> Option<(usize, Vec<String>)> {
    let mut end = paren;
    if end > 0 && header[end - 1].is_punct('>') {
        let mut depth = 0usize;
        let mut index = end;
        while index > 0 {
            index -= 1;
            if header[index].is_punct('>') {
                depth += 1;
            } else if header[index].is_punct('<') {
                depth -= 1;
                if depth == 0 {
                    break;
                }
            }
        }
        if depth != 0 {
            return None;
        }
        end = index;
    }
    // `operator` followed by symbol characters.
    let mut symbols = end;
    while symbols > 0 && matches!(header[symbols - 1].kind, TokKind::Punct(_)) {
        symbols -= 1;
    }
    if symbols > 0 && symbols < end && header[symbols - 1].is_word("operator") {
        let name = header[symbols - 1..end]
            .iter()
            .map(|token| token.text)
            .collect::<String>();
        return Some(qualify(header, symbols - 1, name));
    }
    // `operator()`.
    if end >= 3
        && header[end - 1].kind == TokKind::Close(')')
        && header[end - 2].is_open('(')
        && header[end - 3].is_word("operator")
    {
        return Some(qualify(header, end - 3, "operator()".to_owned()));
    }
    let word = end.checked_sub(1)?;
    if !header[word].is_any_word() {
        return None;
    }
    let mut first = word;
    let mut name = header[word].text.to_owned();
    if first > 0 && header[first - 1].is_punct('~') {
        first -= 1;
        name = format!("~{name}");
    }
    Some(qualify(header, first, name))
}

fn qualify(header: &[HTok<'_>], first: usize, name: String) -> (usize, Vec<String>) {
    let mut segments = vec![name];
    let mut index = first;
    loop {
        if index >= 3
            && header[index - 1].is_punct(':')
            && header[index - 2].is_punct(':')
            && header[index - 3].is_any_word()
        {
            segments.insert(0, header[index - 3].text.to_owned());
            index -= 3;
            continue;
        }
        break;
    }
    (index, segments)
}

/// Words that start a control statement rather than a declaration, common
/// to the C-family languages.
pub const CONTROL_KEYWORDS: &[&str] = &[
    "if",
    "else",
    "for",
    "foreach",
    "while",
    "do",
    "switch",
    "case",
    "catch",
    "try",
    "finally",
    "return",
    "throw",
    "new",
    "delete",
    "sizeof",
    "typeof",
    "using",
    "lock",
    "fixed",
    "synchronized",
    "when",
    "guard",
    "defer",
    "go",
    "select",
    "await",
    "yield",
    "assert",
];

/// Rules for languages whose functions are written `ReturnType name(...)`.
#[derive(Clone, Copy, Debug)]
pub struct CLikeRules {
    pub attributes: AttributeForms,
    /// Keywords that open a named scope (`class`, `struct`, `namespace`, ...).
    pub scope_keywords: &'static [&'static str],
    /// Keywords that open a scope applying to the rest of the body when
    /// followed by `;` instead of a body (C#'s file-scoped namespace).
    pub scope_rest_keywords: &'static [&'static str],
    /// Whether `name(...) => expression;` is a function (C#).
    pub arrow_bodies: bool,
}

/// Classifies a C-family header (C, C++, Java, C#).
pub fn classify_c_like(header: &[HTok<'_>], has_body: bool, rules: &CLikeRules) -> HeaderKind {
    let start = skip_attributes(header, rules.attributes);
    let header = &header[start..];
    let Some(first) = header.first() else {
        return HeaderKind::Other;
    };
    if first.kind == TokKind::Word && CONTROL_KEYWORDS.contains(&first.text) {
        return HeaderKind::Other;
    }
    let paren = first_paren(header, 0);
    let before_paren = paren.unwrap_or(header.len());
    if let Some(keyword) = find_top_level_word(header, 0, before_paren, rules.scope_keywords) {
        // `enum class Color`, `static class Inner<T>`: the name is the first
        // word after the keyword run that is not itself a scope keyword.
        let mut index = keyword + 1;
        while header
            .get(index)
            .is_some_and(|t| t.kind == TokKind::Word && rules.scope_keywords.contains(&t.text))
        {
            index += 1;
        }
        let mut segments = Vec::new();
        while let Some(token) = header.get(index) {
            if !token.is_any_word() {
                break;
            }
            segments.push(token.text.to_owned());
            if header.get(index + 1).is_some_and(|t| t.is_punct('.'))
                || (header.get(index + 1).is_some_and(|t| t.is_punct(':'))
                    && header.get(index + 2).is_some_and(|t| t.is_punct(':')))
            {
                index += if header[index + 1].is_punct('.') {
                    2
                } else {
                    3
                };
                continue;
            }
            break;
        }
        if segments.is_empty() {
            return HeaderKind::Other;
        }
        if has_body {
            return HeaderKind::Scope(segments);
        }
        if rules.scope_rest_keywords.contains(&header[keyword].text) {
            return HeaderKind::ScopeRest(segments);
        }
        return HeaderKind::Other;
    }
    let Some(paren) = paren else {
        return HeaderKind::Other;
    };
    let Some((name_start, segments)) = name_before(header, paren) else {
        return HeaderKind::Other;
    };
    let last = segments.last().map(String::as_str).unwrap_or_default();
    if CONTROL_KEYWORDS.contains(&last) {
        return HeaderKind::Other;
    }
    if name_start > 0 && header[name_start - 1].is_word("new") {
        return HeaderKind::Other;
    }
    // An initializer (`int x = f(1)`, `auto g = [](int) {}`) is not a
    // function definition.
    if any_top_level(header, 0, name_start, |t| t.is_punct('=')) {
        return HeaderKind::Other;
    }
    if has_body {
        return HeaderKind::Function(segments);
    }
    if rules.arrow_bodies {
        let close = matching_close(header, paren).unwrap_or(header.len());
        let arrow = header
            .windows(2)
            .skip(close)
            .any(|pair| pair[0].is_punct('=') && pair[1].is_punct('>'));
        if arrow {
            return HeaderKind::Function(segments);
        }
    }
    HeaderKind::Other
}
