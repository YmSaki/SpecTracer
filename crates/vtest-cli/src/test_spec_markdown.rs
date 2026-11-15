//! The Markdown document of `vtest export --format md`.
//!
//! Boundary: the only input is the test specification data, so the document
//! shows the same content in the same order as the JSON `data` (BD-333,
//! DS-1744) and cannot reach Evidence or verification results (BD-336). The
//! function reads nothing else and cannot fail (BD-335, DES-608). Choosing
//! the output format and writing to stdout / stderr stay in the CLI entry
//! (BD-338).
//!
//! Clauses: BD-333, BD-342, DES-608, DS-1744 to DS-1763.

use std::collections::BTreeMap;

use vtest_scan::test_spec::{
    identifier_encoding, SpecReference, SpecTargetRef, SpecTest, TestSpecification,
};

/// DS-1747: Markdown encoding of a value. Every ASCII punctuation character
/// gets a `\` in front, LF becomes `<br>`, the other C0 controls and U+007F
/// become a decimal character reference, and leading and trailing spaces
/// become `&#32;`. The original value can be recovered from the result.
pub fn markdown_encoding(value: &str) -> String {
    let leading = value.len() - value.trim_start_matches(' ').len();
    let trailing_start = value.trim_end_matches(' ').len().max(leading);
    let mut encoded = String::with_capacity(value.len());
    for (index, character) in value.char_indices() {
        match character {
            ' ' if index < leading || index >= trailing_start => encoded.push_str("&#32;"),
            '\n' => encoded.push_str("<br>"),
            control if control < '\u{20}' || control == '\u{7F}' => {
                encoded.push_str(&format!("&#{};", u32::from(control)));
            }
            punctuation if punctuation.is_ascii_punctuation() => {
                encoded.push('\\');
                encoded.push(punctuation);
            }
            other => encoded.push(other),
        }
    }
    encoded
}

/// DS-1753.
fn target_reference(reference: &SpecTargetRef) -> String {
    match reference {
        SpecTargetRef::Locator { adapter, value } => format!(
            "locator {}::{}",
            markdown_encoding(adapter),
            markdown_encoding(value)
        ),
        SpecTargetRef::SrcId { id } => format!("src_id {}", markdown_encoding(id)),
    }
}

fn test_link(id: &str, key: &str) -> String {
    format!("[{}](#test-{key})", markdown_encoding(id))
}

/// The lines of the document being written. All blank lines come from
/// [`Document::heading`], so the document has no others (DS-1749).
struct Document {
    lines: Vec<String>,
}

impl Document {
    /// DS-1749: one blank line before every heading except the first line,
    /// and one after every heading. Two headings in a row share one.
    fn heading(&mut self, line: String) {
        if !self.lines.is_empty() && self.lines.last().is_some_and(|last| !last.is_empty()) {
            self.lines.push(String::new());
        }
        self.lines.push(line);
        self.lines.push(String::new());
    }

    fn line(&mut self, line: String) {
        self.lines.push(line);
    }

    /// DS-1750.
    fn none(&mut self) {
        self.lines.push("None.".to_owned());
    }

    /// DS-1745: LF line ends, the last line included.
    fn finish(self) -> String {
        let mut text = self.lines.join("\n");
        text.push('\n');
        text
    }
}

fn test_item(document: &mut Document, test: &SpecTest) {
    document.heading(format!(
        "### <a id=\"test-{}\"></a>{}",
        test.key,
        markdown_encoding(&test.id)
    ));
    // DS-1758: a scalar that was not declared is written as `(none)`.
    let scalar = |name: &str, value: Option<&str>| match value {
        Some(value) => format!("- {name}: {}", markdown_encoding(value)),
        None => format!("- {name}: (none)"),
    };
    // DS-1759: an empty list is `(none)`, otherwise one indented line each.
    let list = |document: &mut Document, name: &str, items: Vec<String>| {
        if items.is_empty() {
            document.line(format!("- {name}: (none)"));
        } else {
            document.line(format!("- {name}:"));
            for item in items {
                document.line(format!("  - {item}"));
            }
        }
    };
    document.line(scalar("intent", Some(test.intent.as_str())));
    list(
        document,
        "covers",
        test.covers
            .iter()
            .map(|cover| {
                format!(
                    "[{}](#vo-{}): {}",
                    markdown_encoding(&cover.vo),
                    identifier_encoding(&cover.vo),
                    cover
                        .claim
                        .as_deref()
                        .map_or_else(|| "(unresolved)".to_owned(), markdown_encoding)
                )
            })
            .collect(),
    );
    list(
        document,
        "targets",
        test.targets
            .iter()
            .map(|target| {
                let mut line = target_reference(&target.reference);
                if !target.resolved {
                    line.push_str(" (unresolved)");
                }
                line
            })
            .collect(),
    );
    document.line(scalar("input", test.input.as_deref()));
    document.line(scalar("expect", test.expect.as_deref()));
    document.line(scalar("kind", test.kind.as_deref()));
    list(
        document,
        "cases",
        test.cases
            .iter()
            .map(|case| markdown_encoding(case))
            .collect(),
    );
    list(
        document,
        "related",
        test.related
            .iter()
            .map(|related| match &related.test {
                Some(key) => test_link(&related.id, key),
                None => format!("{} (unresolved)", markdown_encoding(&related.id)),
            })
            .collect(),
    );
    document.line(scalar("adapter", Some(test.location.adapter.as_str())));
    document.line(scalar("path", Some(test.location.path.as_str())));
    document.line(scalar("locator", Some(test.location.locator.as_str())));
}

/// Renders the test specification data as the Markdown document (DES-608).
pub fn render_test_specification(data: &TestSpecification) -> String {
    let ids_by_key = data
        .tests
        .iter()
        .map(|test| (test.key.as_str(), test.id.as_str()))
        .collect::<BTreeMap<_, _>>();
    // A key always names an element of `data.tests`; the fallback only keeps
    // this function total for data built by hand.
    let link = |key: &str| test_link(ids_by_key.get(key).copied().unwrap_or(key), key);

    let mut document = Document { lines: Vec::new() };
    // DS-1748: all six headings, in this order, also for empty sections.
    document.heading("# Test Specification".to_owned());

    document.heading("## Unmanaged tests".to_owned());
    if data.unmanaged.is_empty() {
        document.none();
    }
    for entry in &data.unmanaged {
        // DS-1751.
        document.line(format!(
            "- {}: {}: {} ({})",
            markdown_encoding(&entry.location.adapter),
            markdown_encoding(&entry.location.path),
            markdown_encoding(&entry.location.locator),
            entry
                .codes
                .iter()
                .map(|code| markdown_encoding(code))
                .collect::<Vec<_>>()
                .join(", ")
        ));
    }

    document.heading("## Unresolved references".to_owned());
    if data.unresolved.is_empty() {
        document.none();
    }
    for entry in &data.unresolved {
        // DS-1752.
        let reference = match &entry.reference {
            SpecReference::Id(id) => markdown_encoding(id),
            SpecReference::Target(target) => target_reference(target),
        };
        document.line(format!(
            "- {} {}: {reference} ({})",
            link(&entry.test),
            entry.field.as_str(),
            entry
                .code
                .as_deref()
                .map_or_else(|| "no diagnostic".to_owned(), markdown_encoding)
        ));
    }

    document.heading("## Tests by VO".to_owned());
    if data.by_vo.is_empty() {
        document.none();
    }
    for entry in &data.by_vo {
        // DS-1754 / DS-1755.
        document.heading(format!(
            "### <a id=\"vo-{}\"></a>{}",
            identifier_encoding(&entry.vo),
            markdown_encoding(&entry.vo)
        ));
        document.line(format!(
            "claim: {}",
            entry
                .claim
                .as_deref()
                .map_or_else(|| "(unresolved)".to_owned(), markdown_encoding)
        ));
        for key in &entry.tests {
            document.line(format!("- {}", link(key)));
        }
    }

    document.heading("## Tests by path".to_owned());
    if data.by_path.is_empty() {
        document.none();
    }
    for entry in &data.by_path {
        // DS-1756.
        document.heading(format!(
            "### {}: {}",
            markdown_encoding(&entry.adapter),
            markdown_encoding(&entry.path)
        ));
        for key in &entry.tests {
            document.line(format!("- {}", link(key)));
        }
    }

    document.heading("## Tests".to_owned());
    if data.tests.is_empty() {
        document.none();
    }
    for test in &data.tests {
        // DS-1757.
        test_item(&mut document, test);
    }

    document.finish()
}

#[cfg(test)]
mod tests {
    use super::*;
    use vtest_scan::test_spec::{
        SpecCover, SpecLocation, SpecPathIndex, SpecReferenceField, SpecRelated, SpecTarget,
        SpecUnmanaged, SpecUnresolved, SpecVoIndex,
    };

    /// The inverse of DS-1747, written independently of `markdown_encoding`.
    fn decode(encoded: &str) -> String {
        let mut decoded = String::new();
        let mut rest = encoded;
        while let Some(character) = rest.chars().next() {
            if let Some(after) = rest.strip_prefix("<br>") {
                decoded.push('\n');
                rest = after;
            } else if let Some(after) = rest.strip_prefix("&#") {
                let (number, after) = after.split_once(';').expect("reference ends with ;");
                decoded.push(char::from_u32(number.parse().expect("decimal")).expect("char"));
                rest = after;
            } else if let Some(after) = rest.strip_prefix('\\') {
                let escaped = after.chars().next().expect("escaped character");
                decoded.push(escaped);
                rest = &after[escaped.len_utf8()..];
            } else {
                decoded.push(character);
                rest = &rest[character.len_utf8()..];
            }
        }
        decoded
    }

    /// @vtest.id TEST-CLI-TEST-SPEC-MARKDOWN-ENCODING-ESCAPES-EVERY-ASCII-SYMBOL
    /// @vtest.covers VO-CLI-TEST-SPEC-MARKDOWN-ENCODING
    /// @vtest.target crates/vtest-cli/src/test_spec_markdown.rs::markdown_encoding
    /// @vtest.intent each of the 32 ASCII symbols gets a backslash in front and keeps its place, while letters, digits, inner spaces and non-ASCII characters are written as they are (DS-1747)
    #[test]
    fn every_ascii_symbol_gets_a_backslash_and_other_characters_stay() {
        let symbols = "!\"#$%&'()*+,-./:;<=>?@[\\]^_`{|}~";
        assert_eq!(symbols.chars().count(), 32);

        let expected = symbols
            .chars()
            .flat_map(|symbol| ['\\', symbol])
            .collect::<String>();

        assert_eq!(markdown_encoding(symbols), expected);
        assert_eq!(markdown_encoding("aZ09 é 日本"), "aZ09 é 日本");
    }

    /// @vtest.id TEST-CLI-TEST-SPEC-MARKDOWN-ENCODING-ROUND-TRIPS
    /// @vtest.covers VO-CLI-TEST-SPEC-MARKDOWN-ENCODING
    /// @vtest.target crates/vtest-cli/src/test_spec_markdown.rs::markdown_encoding
    /// @vtest.intent LF becomes <br>, the other C0 controls and U+007F become decimal references, leading and trailing spaces become &#32;, the result has no line break, and undoing the replacements gives back the value byte for byte (DS-1747)
    #[test]
    fn controls_and_edge_spaces_are_replaced_and_the_value_can_be_recovered() {
        assert_eq!(
            markdown_encoding("  a b\n\tc\r\u{0}\u{7F}  "),
            "&#32;&#32;a b<br>&#9;c&#13;&#0;&#127;&#32;&#32;"
        );
        assert_eq!(markdown_encoding("   "), "&#32;&#32;&#32;");
        assert_eq!(markdown_encoding(""), "");

        let value = " # <br> &#32; \\ *x* [y](z)\n- item\n\n    code\u{1F}\u{7F} 日本語 `|` ";
        let encoded = markdown_encoding(value);

        assert!(!encoded.contains('\n'));
        assert_eq!(decode(&encoded), value);
    }

    fn location(path: &str, locator: &str) -> SpecLocation {
        SpecLocation {
            adapter: "rust-cargo".to_owned(),
            path: path.to_owned(),
            locator: locator.to_owned(),
        }
    }

    fn test_item(key: &str, id: &str) -> SpecTest {
        SpecTest {
            key: key.to_owned(),
            id: id.to_owned(),
            intent: "intent".to_owned(),
            covers: Vec::new(),
            targets: Vec::new(),
            input: None,
            expect: None,
            kind: None,
            cases: Vec::new(),
            related: Vec::new(),
            location: location("src/a.rs", "tests::a"),
        }
    }

    fn empty() -> TestSpecification {
        TestSpecification {
            tests: Vec::new(),
            unmanaged: Vec::new(),
            unresolved: Vec::new(),
            by_vo: Vec::new(),
            by_path: Vec::new(),
        }
    }

    /// @vtest.id TEST-CLI-TEST-SPEC-MARKDOWN-EMPTY-DATA-KEEPS-ALL-HEADINGS
    /// @vtest.covers VO-CLI-TEST-SPEC-MARKDOWN-HEADINGS
    /// @vtest.target crates/vtest-cli/src/test_spec_markdown.rs::render_test_specification
    /// @vtest.intent empty data renders the six headings in the fixed order, each followed by a blank line and None., with one blank line before each heading but the first and a final LF (DS-1745, DS-1748, DS-1749, DS-1750)
    #[test]
    fn empty_data_renders_all_six_headings_with_none() {
        assert_eq!(
            render_test_specification(&empty()),
            "# Test Specification\n\
             \n\
             ## Unmanaged tests\n\
             \n\
             None.\n\
             \n\
             ## Unresolved references\n\
             \n\
             None.\n\
             \n\
             ## Tests by VO\n\
             \n\
             None.\n\
             \n\
             ## Tests by path\n\
             \n\
             None.\n\
             \n\
             ## Tests\n\
             \n\
             None.\n"
        );
    }

    fn full() -> TestSpecification {
        let mut first = test_item("TEST-A", "TEST-A");
        first.covers = vec![
            SpecCover {
                vo: "VO A".to_owned(),
                claim: Some("claim *A*".to_owned()),
            },
            SpecCover {
                vo: "VO-GONE".to_owned(),
                claim: None,
            },
        ];
        first.targets = vec![
            SpecTarget {
                reference: SpecTargetRef::Locator {
                    adapter: "rust-cargo".to_owned(),
                    value: "src/a.rs::f".to_owned(),
                },
                resolved: true,
            },
            SpecTarget {
                reference: SpecTargetRef::SrcId {
                    id: "SRC-1".to_owned(),
                },
                resolved: false,
            },
        ];
        first.input = Some("(none)".to_owned());
        first.cases = vec!["one".to_owned(), "two".to_owned()];
        first.related = vec![
            SpecRelated {
                id: "TEST-DUP".to_owned(),
                test: None,
            },
            SpecRelated {
                id: "TEST-B".to_owned(),
                test: Some("TEST-B~1".to_owned()),
            },
        ];
        TestSpecification {
            tests: vec![first, test_item("TEST-B~1", "TEST-B")],
            unmanaged: vec![SpecUnmanaged {
                location: location("src/a.rs", "tests::plain"),
                codes: vec!["E-SCAN-006".to_owned(), "E-SCAN-007".to_owned()],
            }],
            unresolved: vec![
                SpecUnresolved {
                    test: "TEST-A".to_owned(),
                    field: SpecReferenceField::Covers,
                    reference: SpecReference::Id("VO-GONE".to_owned()),
                    code: Some("E-SCAN-003".to_owned()),
                },
                SpecUnresolved {
                    test: "TEST-A".to_owned(),
                    field: SpecReferenceField::Related,
                    reference: SpecReference::Id("TEST-NOWHERE".to_owned()),
                    code: None,
                },
                SpecUnresolved {
                    test: "TEST-B~1".to_owned(),
                    field: SpecReferenceField::Target,
                    reference: SpecReference::Target(SpecTargetRef::SrcId {
                        id: "SRC-1".to_owned(),
                    }),
                    code: Some("E-SCAN-004".to_owned()),
                },
            ],
            by_vo: vec![
                SpecVoIndex {
                    vo: "VO A".to_owned(),
                    claim: Some("claim *A*".to_owned()),
                    tests: vec!["TEST-A".to_owned()],
                },
                SpecVoIndex {
                    vo: "VO-GONE".to_owned(),
                    claim: None,
                    tests: vec!["TEST-A".to_owned()],
                },
            ],
            by_path: vec![SpecPathIndex {
                adapter: "rust-cargo".to_owned(),
                path: "src/a.rs".to_owned(),
                tests: vec!["TEST-A".to_owned(), "TEST-B~1".to_owned()],
            }],
        }
    }

    /// @vtest.id TEST-CLI-TEST-SPEC-MARKDOWN-RENDERS-EVERY-LINE-FORM
    /// @vtest.covers VO-CLI-TEST-SPEC-MARKDOWN-SAME-CONTENT-AS-DATA
    /// @vtest.target crates/vtest-cli/src/test_spec_markdown.rs::render_test_specification
    /// @vtest.intent the document shows every element of the data in data order with the line forms of each section: unmanaged and unresolved lines, VO and path indexes with anchors and links, and Test items with their eleven lines, (none), (unresolved) and (no diagnostic) markers (DS-1744, DS-1751 to DS-1763)
    #[test]
    fn the_document_shows_every_element_in_its_line_form() {
        let expected = r#"# Test Specification

## Unmanaged tests

- rust\-cargo: src\/a\.rs: tests\:\:plain (E\-SCAN\-006, E\-SCAN\-007)

## Unresolved references

- [TEST\-A](#test-TEST-A) covers: VO\-GONE (E\-SCAN\-003)
- [TEST\-A](#test-TEST-A) related: TEST\-NOWHERE (no diagnostic)
- [TEST\-B](#test-TEST-B~1) target: src_id SRC\-1 (E\-SCAN\-004)

## Tests by VO

### <a id="vo-VO_20A"></a>VO A

claim: claim \*A\*
- [TEST\-A](#test-TEST-A)

### <a id="vo-VO-GONE"></a>VO\-GONE

claim: (unresolved)
- [TEST\-A](#test-TEST-A)

## Tests by path

### rust\-cargo: src\/a\.rs

- [TEST\-A](#test-TEST-A)
- [TEST\-B](#test-TEST-B~1)

## Tests

### <a id="test-TEST-A"></a>TEST\-A

- intent: intent
- covers:
  - [VO A](#vo-VO_20A): claim \*A\*
  - [VO\-GONE](#vo-VO-GONE): (unresolved)
- targets:
  - locator rust\-cargo::src\/a\.rs\:\:f
  - src_id SRC\-1 (unresolved)
- input: \(none\)
- expect: (none)
- kind: (none)
- cases:
  - one
  - two
- related:
  - TEST\-DUP (unresolved)
  - [TEST\-B](#test-TEST-B~1)
- adapter: rust\-cargo
- path: src\/a\.rs
- locator: tests\:\:a

### <a id="test-TEST-B~1"></a>TEST\-B

- intent: intent
- covers: (none)
- targets: (none)
- input: (none)
- expect: (none)
- kind: (none)
- cases: (none)
- related: (none)
- adapter: rust\-cargo
- path: src\/a\.rs
- locator: tests\:\:a
"#;

        assert_eq!(render_test_specification(&full()), expected);
    }

    /// @vtest.id TEST-CLI-TEST-SPEC-MARKDOWN-VALUES-DO-NOT-CHANGE-STRUCTURE
    /// @vtest.covers VO-CLI-TEST-SPEC-MARKDOWN-VALUES-KEEP-STRUCTURE
    /// @vtest.target crates/vtest-cli/src/test_spec_markdown.rs::render_test_specification
    /// @vtest.intent replacing every declared value with text full of Markdown syntax and line breaks leaves the number of lines, the number of heading lines and the blank lines of the document unchanged (SPEC-481)
    #[test]
    fn hostile_values_leave_the_line_structure_unchanged() {
        let plain = full();
        let hostile_text = "# h\n| a | b |\n`c` <d> *e* [f](g)\n\n- h\n    i";
        let mut hostile = full();
        for test in &mut hostile.tests {
            test.id = hostile_text.to_owned();
            test.intent = hostile_text.to_owned();
            test.expect = Some(hostile_text.to_owned());
            test.location.path = hostile_text.to_owned();
            test.location.locator = hostile_text.to_owned();
            for case in &mut test.cases {
                *case = hostile_text.to_owned();
            }
            for cover in &mut test.covers {
                cover.vo = hostile_text.to_owned();
                cover.claim = cover.claim.as_ref().map(|_| hostile_text.to_owned());
            }
            for related in &mut test.related {
                related.id = hostile_text.to_owned();
            }
        }
        for entry in &mut hostile.unmanaged {
            entry.location.locator = hostile_text.to_owned();
        }
        for entry in &mut hostile.by_vo {
            entry.vo = hostile_text.to_owned();
            entry.claim = Some(hostile_text.to_owned());
        }
        for entry in &mut hostile.by_path {
            entry.path = hostile_text.to_owned();
        }
        let mut plain_with_expect = plain;
        for test in &mut plain_with_expect.tests {
            test.expect = Some("expect".to_owned());
        }
        for entry in &mut plain_with_expect.by_vo {
            entry.claim = Some("claim".to_owned());
        }

        let shape = |document: &str| {
            document
                .lines()
                .map(|line| {
                    if line.is_empty() {
                        "blank"
                    } else if line.starts_with('#') {
                        "heading"
                    } else {
                        "text"
                    }
                })
                .collect::<Vec<_>>()
        };

        assert_eq!(
            shape(&render_test_specification(&hostile)),
            shape(&render_test_specification(&plain_with_expect))
        );
    }
}
