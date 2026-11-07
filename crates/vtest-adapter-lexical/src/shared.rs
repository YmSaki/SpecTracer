//! The parts of reading a file that mean the same thing in every
//! language, for the language modules to call.
//!
//! Each language module was written on its own first, and only what then
//! turned out to be the same in meaning (not merely similar in code) is
//! here:
//!
//! - [`pair_brackets`]: which tokens are brackets is each language's
//!   lexer's decision (PHP's `#[` is a `[`); pairing `(`/`)`, `[`/`]`, and
//!   `{`/`}` once they are found does not depend on the language.
//! - [`attached_comments`]: which comment block a declaration belongs to
//!   is SpecTracer's annotation rule, not a language's grammar, so it is
//!   decided once, for every language.
//!
//! Comment syntax (`//`, `/* */`, `#`, Javadoc's leading `*`) is not here,
//! although several languages spell it the same way: it is each
//! language's grammar, and the languages agree only by coincidence.

use crate::ExtractError;

impl ExtractError {
    pub fn new(offset: usize, message: impl Into<String>) -> Self {
        Self {
            offset,
            message: message.into(),
        }
    }
}

/// Pairs the brackets among `items`, one per token in order: its byte
/// offset, and its bracket character if it is one of `([{)]}`. Returns,
/// for each token, the index of its partner. An unmatched or mismatched
/// bracket makes the file unreadable.
pub fn pair_brackets(
    items: impl IntoIterator<Item = (usize, Option<char>)>,
) -> Result<Vec<Option<usize>>, ExtractError> {
    let mut partners = Vec::new();
    let mut stack: Vec<(usize, usize, char)> = Vec::new();
    for (index, (offset, bracket)) in items.into_iter().enumerate() {
        partners.push(None);
        let Some(bracket) = bracket else {
            continue;
        };
        let open = match bracket {
            ')' => '(',
            ']' => '[',
            '}' => '{',
            _ => {
                stack.push((index, offset, bracket));
                continue;
            }
        };
        let (open_index, _, found) = stack
            .pop()
            .ok_or_else(|| ExtractError::new(offset, "unmatched closing bracket"))?;
        if found != open {
            return Err(ExtractError::new(offset, "mismatched brackets"));
        }
        partners[open_index] = Some(index);
        partners[index] = Some(open_index);
    }
    match stack.pop() {
        Some((_, offset, _)) => Err(ExtractError::new(offset, "unclosed bracket")),
        None => Ok(partners),
    }
}

/// A token's byte range, and whether it is a comment.
#[derive(Clone, Copy, Debug)]
pub struct Span {
    pub start: usize,
    pub end: usize,
    pub comment: bool,
}

/// A comment block and the first token after it, as indices into the
/// token list.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Attached {
    pub first: usize,
    pub last: usize,
    pub next: usize,
}

/// The comment blocks that are attached to the code right after them: a
/// block is consecutive comments, each starting its own line and at most
/// one line after the previous one, and the code it is attached to starts
/// on the next line (a blank line detaches it).
pub fn attached_comments(source: &str, spans: &[Span]) -> Vec<Attached> {
    let line_breaks = |before: usize, after: usize| {
        source[spans[before].end..spans[after].start]
            .matches('\n')
            .count()
    };
    let starts_line = |index: usize| {
        source[..spans[index].start]
            .rsplit('\n')
            .next()
            .is_none_or(|prefix| prefix.trim().is_empty())
    };
    let mut output = Vec::new();
    let mut index = 0;
    while index < spans.len() {
        if !spans[index].comment || !starts_line(index) {
            index += 1;
            continue;
        }
        let first = index;
        while index + 1 < spans.len()
            && spans[index + 1].comment
            && line_breaks(index, index + 1) <= 1
        {
            index += 1;
        }
        let last = index;
        index += 1;
        if index < spans.len() && line_breaks(last, index) <= 1 {
            output.push(Attached {
                first,
                last,
                next: index,
            });
        }
    }
    output
}

#[cfg(test)]
mod tests {
    use super::*;

    fn spans(source: &str, comments: &[&str], code: &[&str]) -> Vec<Span> {
        let mut spans: Vec<Span> = comments
            .iter()
            .map(|text| (text, true))
            .chain(code.iter().map(|text| (text, false)))
            .map(|(text, comment)| {
                let start = source.find(*text).expect("fixture text");
                Span {
                    start,
                    end: start + text.len(),
                    comment,
                }
            })
            .collect();
        spans.sort_by_key(|span| span.start);
        spans
    }

    /// @vtest.id TEST-ADAPTER-LEXICAL-ATTACHMENT-RULE
    /// @vtest.covers VO-ADAPTER-LEXICAL-NORMALIZES-DECLARATION
    /// @vtest.target crates/vtest-adapter-lexical/src/shared.rs::attached_comments
    /// @vtest.intent verifies that a comment block attaches to the code on the next line, that a blank line between the block and the code or inside the block detaches it, and that a comment after code on the same line starts no block
    #[test]
    fn comments_attach_to_the_next_line_only() {
        let source = "// a\n// b\nfn one\n\n// c\n\nfn two\nfn three // d\n// e\n\n// f\nfn four\n";
        let spans = spans(
            source,
            &["// a", "// b", "// c", "// d", "// e", "// f"],
            &["fn one", "fn two", "fn three", "fn four"],
        );
        let blocks = attached_comments(source, &spans);
        let texts = blocks
            .iter()
            .map(|block| {
                (
                    &source[spans[block.first].start..spans[block.last].end],
                    &source[spans[block.next].start..spans[block.next].end],
                )
            })
            .collect::<Vec<_>>();
        assert_eq!(texts, vec![("// a\n// b", "fn one"), ("// f", "fn four")]);
    }

    /// @vtest.id TEST-ADAPTER-LEXICAL-BRACKET-PAIRING
    /// @vtest.covers VO-ADAPTER-LEXICAL-RETURNS-SOURCE-RANGE-AND-BYTES
    /// @vtest.target crates/vtest-adapter-lexical/src/shared.rs::pair_brackets
    /// @vtest.intent verifies that nested brackets are paired with their partners and that an unmatched, mismatched, or unclosed bracket is an error at its offset instead of a guessed pairing
    #[test]
    fn brackets_pair_or_fail() {
        let items = [
            (0, Some('(')),
            (1, Some('[')),
            (2, None),
            (3, Some(']')),
            (4, Some(')')),
        ];
        assert_eq!(
            pair_brackets(items).unwrap(),
            vec![Some(4), Some(3), None, Some(1), Some(0)]
        );
        for (items, offset) in [
            (vec![(0, Some(')'))], 0),
            (vec![(0, Some('(')), (5, Some(']'))], 5),
            (vec![(0, Some('{')), (1, Some('('))], 1),
        ] {
            assert_eq!(pair_brackets(items).unwrap_err().offset, offset);
        }
    }
}
