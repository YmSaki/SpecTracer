//! `@vtest.` declarations written in ordinary comments.
//!
//! The `rust-cargo` adapter reads the test-annotation-line grammar
//! (`"@vtest." test-key SP value`, DS-487) from Rust doc comments. The
//! lexical adapters read the same grammar from any comment of the target
//! language that immediately precedes a construct, because those languages
//! have no single doc-comment form and their test frameworks differ. Each
//! language removes its own comment markers; this module reads the lines. The key
//! sets, repetition rules, and diagnostics are the same as `rust-cargo`'s:
//!
//! - a comment block with at least one test-key line declares a Test
//!   construct (surface 1); an unknown key there is `E-SCAN-006`, a repeated
//!   non-repeatable key or a repeated identical `target` is `E-SCAN-005`
//!   (DS-1618/DS-1619, DES-229);
//! - a comment block whose `@vtest.` lines carry no test-key is a Source
//!   Target declaration (surface 2): `src-id` is recognized, any other key is
//!   `W-SCAN-105`, and a repeated `src-id` is `E-SCAN-005` with no SRC ID
//!   adopted (DS-551).

use std::collections::{BTreeMap, BTreeSet};

/// The full set of test-keys the test-annotation-line grammar recognizes.
pub const TEST_KEYS: &[&str] = &[
    "id", "covers", "target", "intent", "input", "expect", "kind", "case", "related",
];

/// The only source-target-key.
pub const SOURCE_TARGET_KEY: &str = "src-id";

/// Reads one comment line, with the language's comment markers already
/// removed, as an `@vtest.` annotation line: its key and value. A line that
/// does not start with `@vtest.` (after whitespace) is free text.
pub fn parse_line(line: &str) -> Option<(String, String)> {
    let annotation = line.trim().strip_prefix("@vtest.")?;
    let (key, value) = match annotation.find(char::is_whitespace) {
        Some(separator) => annotation.split_at(separator),
        None => (annotation, ""),
    };
    Some((key.trim().to_owned(), value.trim().to_owned()))
}

/// The normalized surface-1 declaration (DES-215's logical fields), with the
/// grammar diagnostics found while normalizing it.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct TestDeclaration {
    pub id: Option<String>,
    pub covers: Option<String>,
    pub targets: Vec<String>,
    pub intent: Option<String>,
    pub input: Option<String>,
    pub expect: Option<String>,
    pub kind: Option<String>,
    pub cases: Vec<String>,
    pub related: Vec<String>,
    /// `(code, message)` pairs; all are errors (`E-SCAN-005`/`E-SCAN-006`).
    pub diagnostics: Vec<(String, String)>,
}

/// The surface-2 declaration of a Source Target.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct SourceDeclaration {
    pub src_id: Option<String>,
    /// `(code, message)` pairs: `W-SCAN-105` warnings and `E-SCAN-005`.
    pub diagnostics: Vec<(String, String)>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Declaration {
    Test(TestDeclaration),
    Source(SourceDeclaration),
}

/// Classifies the `@vtest.` lines of one comment block. `None` when the
/// block has no `@vtest.` line at all (plain comment).
pub fn classify(lines: Vec<(String, String)>) -> Option<Declaration> {
    if lines.is_empty() {
        return None;
    }
    if lines
        .iter()
        .any(|(key, _)| TEST_KEYS.contains(&key.as_str()))
    {
        Some(Declaration::Test(parse_test(lines)))
    } else {
        Some(Declaration::Source(parse_source(lines)))
    }
}

fn parse_test(lines: Vec<(String, String)>) -> TestDeclaration {
    let mut declaration = TestDeclaration::default();
    let mut single = BTreeMap::<String, String>::new();
    for (key, value) in lines {
        if !TEST_KEYS.contains(&key.as_str()) {
            declaration.diagnostics.push((
                "E-SCAN-006".to_owned(),
                format!("unrecognized @vtest key `{key}` on a Test construct"),
            ));
            continue;
        }
        match key.as_str() {
            "case" => declaration.cases.push(value),
            "related" => declaration.related.push(value),
            "target" => declaration.targets.push(value),
            _ => {
                if single.insert(key.clone(), value).is_some() {
                    declaration.diagnostics.push((
                        "E-SCAN-005".to_owned(),
                        format!("duplicate annotation key `{key}`"),
                    ));
                }
            }
        }
    }
    let mut seen = BTreeSet::new();
    for value in &declaration.targets {
        if !seen.insert(value.as_str()) {
            declaration.diagnostics.push((
                "E-SCAN-005".to_owned(),
                format!("duplicate target `{value}`"),
            ));
        }
    }
    declaration.id = single.remove("id");
    declaration.covers = single.remove("covers");
    declaration.intent = single.remove("intent");
    declaration.input = single.remove("input");
    declaration.expect = single.remove("expect");
    declaration.kind = single.remove("kind");
    declaration
}

fn parse_source(lines: Vec<(String, String)>) -> SourceDeclaration {
    let mut declaration = SourceDeclaration::default();
    let mut declared = Vec::new();
    for (key, value) in lines {
        if key == SOURCE_TARGET_KEY {
            declared.push(value);
        } else {
            declaration.diagnostics.push((
                "W-SCAN-105".to_owned(),
                format!("unrecognized @vtest key `{key}` on a non-test construct"),
            ));
        }
    }
    match declared.len() {
        0 => {}
        1 => declaration.src_id = declared.pop().filter(|value| !value.is_empty()),
        _ => declaration.diagnostics.push((
            "E-SCAN-005".to_owned(),
            "duplicate annotation key `src-id`".to_owned(),
        )),
    }
    declaration
}

#[cfg(test)]
mod tests {
    use super::*;

    /// @vtest.id TEST-ADAPTER-LEXICAL-ANNOTATION-LINES-FROM-ANY-COMMENT
    /// @vtest.covers VO-ADAPTER-LEXICAL-NORMALIZES-DECLARATION
    /// @vtest.target crates/vtest-adapter-lexical/src/annotation.rs::parse_line
    /// @vtest.intent verifies an @vtest line yields its key and trimmed value, a key alone has an empty value, and free text is ignored
    #[test]
    fn reads_vtest_lines() {
        assert_eq!(
            parse_line(" @vtest.covers VO-1, VO-2 "),
            Some(("covers".to_owned(), "VO-1, VO-2".to_owned()))
        );
        assert_eq!(
            parse_line("@vtest.id"),
            Some(("id".to_owned(), String::new()))
        );
        assert_eq!(parse_line("see @vtest.id elsewhere"), None);
        assert_eq!(parse_line("free text"), None);
    }

    /// @vtest.id TEST-ADAPTER-LEXICAL-ANNOTATION-GRAMMAR-DIAGNOSTICS
    /// @vtest.covers VO-ADAPTER-LEXICAL-REPORTS-DECLARATION-VIOLATIONS
    /// @vtest.target crates/vtest-adapter-lexical/src/annotation.rs::classify
    /// @vtest.intent verifies unknown test keys are E-SCAN-006, repeated single keys and identical targets are E-SCAN-005, and repeatable keys accumulate
    #[test]
    fn test_surface_grammar_violations_are_reported() {
        let lines = vec![
            ("id".to_owned(), "T".to_owned()),
            ("id".to_owned(), "U".to_owned()),
            ("bogus".to_owned(), "x".to_owned()),
            ("target".to_owned(), "a".to_owned()),
            ("target".to_owned(), "a".to_owned()),
            ("case".to_owned(), "one".to_owned()),
            ("case".to_owned(), "two".to_owned()),
        ];
        let Some(Declaration::Test(declaration)) = classify(lines) else {
            panic!("a block with test keys is a Test declaration");
        };
        let codes = declaration
            .diagnostics
            .iter()
            .map(|(code, _)| code.as_str())
            .collect::<Vec<_>>();
        assert_eq!(codes, vec!["E-SCAN-005", "E-SCAN-006", "E-SCAN-005"]);
        assert_eq!(declaration.cases, vec!["one", "two"]);
    }

    /// @vtest.id TEST-ADAPTER-LEXICAL-ANNOTATION-SOURCE-SURFACE
    /// @vtest.covers VO-ADAPTER-LEXICAL-OPAQUE-LOCATOR-SYNTAX
    /// @vtest.target crates/vtest-adapter-lexical/src/annotation.rs::classify
    /// @vtest.intent verifies a block without test keys is a Source Target declaration whose src-id is adopted once, whose unknown keys warn W-SCAN-105, and whose repeated src-id adopts nothing
    #[test]
    fn source_surface_declares_src_id() {
        let Some(Declaration::Source(single)) = classify(vec![
            ("src-id".to_owned(), "SRC-ADD".to_owned()),
            ("src_id".to_owned(), "typo".to_owned()),
        ]) else {
            panic!("no test key means a Source declaration");
        };
        assert_eq!(single.src_id.as_deref(), Some("SRC-ADD"));
        assert_eq!(single.diagnostics[0].0, "W-SCAN-105");

        let Some(Declaration::Source(repeated)) = classify(vec![
            ("src-id".to_owned(), "SRC-A".to_owned()),
            ("src-id".to_owned(), "SRC-B".to_owned()),
        ]) else {
            panic!("no test key means a Source declaration");
        };
        assert_eq!(repeated.src_id, None);
        assert_eq!(repeated.diagnostics[0].0, "E-SCAN-005");
        assert_eq!(classify(Vec::new()), None);
    }
}
