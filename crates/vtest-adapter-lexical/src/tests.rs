use std::{
    env, fs,
    sync::atomic::{AtomicU64, Ordering},
};

use super::*;
use crate::{
    header::{classify_c_like, AttributeForms, CLikeRules},
    lexer::StringSyntax,
};

/// A minimal C-family profile used only by these tests.
struct Toy;

const TOY_SYNTAX: LexicalSyntax = LexicalSyntax {
    line_comments: &["//"],
    line_comment_exclusions: &[],
    block_comments: &[("/*", "*/")],
    nested_block_comments: false,
    strings: &[
        StringSyntax::single("\"", "\""),
        StringSyntax::single("'", "'"),
    ],
    preprocessor: false,
    cpp_raw_strings: false,
    word_suffix_chars: &[],
    newline_terminates: false,
    structure: Structure::Braces,
};

const TOY_RULES: CLikeRules = CLikeRules {
    attributes: AttributeForms {
        at_sign: true,
        hash_bracket: false,
        bracket: false,
    },
    scope_keywords: &["class", "namespace"],
    scope_rest_keywords: &["namespace"],
    arrow_bodies: false,
};

impl LanguageProfile for Toy {
    fn adapter_id(&self) -> &'static str {
        "toy"
    }

    fn languages(&self) -> &'static [&'static str] {
        &["toy"]
    }

    fn extensions(&self) -> &'static [&'static str] {
        &["toy"]
    }

    fn excluded_dirs(&self) -> &'static [&'static str] {
        &["vendor"]
    }

    fn syntax(&self) -> &'static LexicalSyntax {
        &TOY_SYNTAX
    }

    fn classify(&self, header: &[HTok<'_>], has_body: bool) -> HeaderKind {
        classify_c_like(header, has_body, &TOY_RULES)
    }
}

static NEXT: AtomicU64 = AtomicU64::new(0);

fn discover(files: &[(&str, &str)]) -> DiscoveryOutcome {
    let root = env::temp_dir().join(format!(
        "vtest-adapter-lexical-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    let _ = fs::remove_dir_all(&root);
    for (path, text) in files {
        let full = root.join(path);
        fs::create_dir_all(full.parent().expect("parent")).expect("create fixture dir");
        fs::write(full, text).expect("write fixture");
    }
    let outcome = LexicalAdapter::new(Toy)
        .discover(
            &root,
            "toy-project",
            &AdapterScanConfig {
                include_paths: vec![PathBuf::from(".")],
            },
        )
        .expect("discovery succeeds");
    let _ = fs::remove_dir_all(&root);
    outcome
}

const CALCULATOR: &str = r#"namespace demo;

class Calculator {
    /** Adds two numbers.
     * @vtest.src-id SRC-TOY-ADD
     */
    int add(int a, int b) { return a + b; }

    int sub(int a, int b) {
        if (a > b) { return a - b; }
        return "}".length;
    }
}

class CalculatorTest {
    // @vtest.id TEST-TOY-ADD
    // @vtest.covers VO-TOY-ADD, VO-TOY-MATH
    // @vtest.target src/calc.toy::demo::Calculator::sub
    // @vtest.target SRC-TOY-ADD
    // @vtest.intent adds
    @Test(timeout = 5)
    void addsNumbers() { assertEquals(3, add(1, 2)); }
}

/* @vtest.id TEST-TOY-CALL-STYLE */
// @vtest.covers VO-TOY-ADD
// @vtest.intent a framework call with a closure is a Test construct too
test("adds", () => {
    expect(add(1, 2)).toBe(3);
});
"#;

/// @vtest.id TEST-ADAPTER-LEXICAL-DISCOVERS-ANNOTATED-CONSTRUCTS
/// @vtest.covers VO-ADAPTER-LEXICAL-NORMALIZES-DECLARATION, VO-ADAPTER-LEXICAL-OPAQUE-LOCATOR-SYNTAX
/// @vtest.target crates/vtest-adapter-lexical/src/lib.rs::LexicalAdapter::discover
/// @vtest.intent verifies an annotated method and an annotated framework call are both Test constructs with normalized metadata, and every function is a Source Target with a scope-qualified locator and its declared src-id
#[test]
fn discovers_annotated_tests_and_qualified_sources() {
    let outcome = discover(&[("src/calc.toy", CALCULATOR)]);
    assert!(outcome.diagnostics.is_empty(), "{:?}", outcome.diagnostics);
    assert_eq!(outcome.files_scanned, 1);

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
            ("src/calc.toy::demo::Calculator::add", Some("SRC-TOY-ADD")),
            ("src/calc.toy::demo::Calculator::sub", None),
            ("src/calc.toy::demo::CalculatorTest::addsNumbers", None),
        ]
    );

    assert_eq!(outcome.tests.len(), 2);
    let method = &outcome.tests[0];
    assert_eq!(method.id.as_str(), "TEST-TOY-ADD");
    assert_eq!(
        method.covers.iter().map(VoId::as_str).collect::<Vec<_>>(),
        vec!["VO-TOY-ADD", "VO-TOY-MATH"]
    );
    assert_eq!(
        method.targets,
        vec![
            TargetRef::Locator(Locator {
                adapter: AdapterId::new("toy"),
                value: "src/calc.toy::demo::Calculator::sub".to_owned(),
            }),
            TargetRef::SrcId(SrcId::new("SRC-TOY-ADD")),
        ]
    );
    assert_eq!(method.location.locator, "demo::CalculatorTest::addsNumbers");
    assert_eq!(
        method.execution.selector,
        "demo::CalculatorTest::addsNumbers"
    );
    assert_eq!(method.execution.project.as_deref(), Some("toy-project"));
    assert!(method.construct_text.starts_with("@Test(timeout = 5)"));
    assert!(!method.construct_text.contains("@vtest"));

    let call = &outcome.tests[1];
    assert_eq!(call.id.as_str(), "TEST-TOY-CALL-STYLE");
    assert_eq!(call.location.locator, "test#2");
    assert!(call.construct_text.starts_with("test(\"adds\""));
    assert!(call.construct_text.ends_with("});"));
}

/// @vtest.id TEST-ADAPTER-LEXICAL-RANGES-MATCH-CURRENT-BYTES
/// @vtest.covers VO-ADAPTER-LEXICAL-RETURNS-SOURCE-RANGE-AND-BYTES
/// @vtest.target crates/vtest-adapter-lexical/src/lib.rs::LexicalAdapter::discover
/// @vtest.intent verifies every returned Test and Source Target byte range selects exactly its construct bytes in the current file, with the Source Target including its comment block
#[test]
fn every_range_selects_its_construct_bytes() {
    let outcome = discover(&[("src/calc.toy", CALCULATOR)]);
    let slice = |location: &SourceLocation| {
        &CALCULATOR[location.byte_range.start as usize..location.byte_range.end as usize]
    };
    for test in &outcome.tests {
        assert_eq!(slice(&test.location), test.construct_text);
    }
    for source in &outcome.sources {
        assert_eq!(slice(&source.location), source.construct_text);
    }
    assert!(outcome.sources[0]
        .construct_text
        .starts_with("/** Adds two numbers."));
    assert!(outcome.sources[1]
        .construct_text
        .ends_with("return \"}\".length;\n    }"));
}

/// @vtest.id TEST-ADAPTER-LEXICAL-MALFORMED-DECLARATION-IS-MISSING
/// @vtest.covers VO-ADAPTER-LEXICAL-UNPARSEABLE-DECLARATION-MISSING, VO-ADAPTER-LEXICAL-REPORTS-DECLARATION-VIOLATIONS
/// @vtest.target crates/vtest-adapter-lexical/src/lib.rs::LexicalAdapter::discover
/// @vtest.intent verifies declarations with a grammar error or a missing required key are returned as Missing Test constructs with E-SCAN-006 or E-SCAN-007 and no Test draft
#[test]
fn malformed_declarations_are_missing_tests_with_diagnostics() {
    let outcome = discover(&[(
        "t.toy",
        "// @vtest.id TEST-A\n// @vtest.covers VO-A\n// @vtest.intnet typo\nvoid a() {}\n\n\
         // @vtest.id TEST-B\n// @vtest.intent no covers\nvoid b() {}\n",
    )]);
    assert!(outcome.tests.is_empty());
    assert_eq!(outcome.missing_tests.len(), 2);
    let codes = outcome
        .diagnostics
        .iter()
        .map(|diagnostic| diagnostic.code.as_str())
        .collect::<Vec<_>>();
    assert_eq!(codes, vec!["E-SCAN-006", "E-SCAN-007"]);
    assert_eq!(outcome.missing_tests[0].construct_text, "void a() {}");
}

/// @vtest.id TEST-ADAPTER-LEXICAL-BLANK-LINE-DETACHES
/// @vtest.covers VO-ADAPTER-LEXICAL-NORMALIZES-DECLARATION
/// @vtest.target crates/vtest-adapter-lexical/src/lib.rs::LexicalAdapter::discover
/// @vtest.intent verifies a blank line between a declaration comment and the next construct leaves the construct undeclared
#[test]
fn a_blank_line_ends_the_association() {
    let outcome = discover(&[(
        "t.toy",
        "// @vtest.id TEST-A\n// @vtest.covers VO-A\n// @vtest.intent x\n\nvoid a() {}\n",
    )]);
    assert!(outcome.tests.is_empty());
    assert!(outcome.missing_tests.is_empty());
    assert_eq!(outcome.sources.len(), 1);
}

/// @vtest.id TEST-ADAPTER-LEXICAL-UNBALANCED-FILE-E-SCAN-001
/// @vtest.covers VO-ADAPTER-LEXICAL-RETURNS-SOURCE-RANGE-AND-BYTES
/// @vtest.target crates/vtest-adapter-lexical/src/lib.rs::LexicalAdapter::discover
/// @vtest.intent verifies a file whose brackets do not balance yields E-SCAN-001 and no guessed constructs, without stopping discovery of other files or descending into excluded directories
#[test]
fn unbalanced_file_is_e_scan_001_and_contributes_nothing() {
    let outcome = discover(&[
        (
            "a.toy",
            "// @vtest.id T\n// @vtest.covers VO\n// @vtest.intent x\nvoid a() {\n",
        ),
        ("b.toy", "void b() {}\n"),
        ("vendor/c.toy", "void c() {}\n"),
    ]);
    assert_eq!(outcome.files_scanned, 2);
    assert_eq!(outcome.diagnostics.len(), 1);
    assert_eq!(outcome.diagnostics[0].code, "E-SCAN-001");
    assert!(outcome.tests.is_empty());
    assert_eq!(outcome.sources.len(), 1);
    assert_eq!(outcome.sources[0].locator.value, "b.toy::b");
}

/// @vtest.id TEST-ADAPTER-LEXICAL-DECLARES-DISCOVERY-ONLY
/// @vtest.covers VO-ADAPTER-LEXICAL-DECLARES-DISCOVERY-ONLY
/// @vtest.target crates/vtest-adapter-lexical/src/lib.rs::LexicalAdapter
/// @vtest.intent verifies a lexical adapter declares its id, languages, config namespace, and only the source_discovery capability, and registers without a capability mismatch
#[test]
fn declares_only_source_discovery() {
    let adapter = LexicalAdapter::new(Toy);
    let descriptor = adapter.descriptor();
    assert_eq!(descriptor.id, "toy");
    assert_eq!(descriptor.languages, vec!["toy"]);
    assert_eq!(descriptor.config_namespace, "toy");
    assert_eq!(descriptor.capabilities, vec![Capability::SourceDiscovery]);
    let mut registry = vtest_adapter_api::AdapterRegistry::new();
    registry
        .register(Box::new(adapter))
        .expect("declaration matches implementation");
    assert!(registry.test_runner("toy").is_none());
    assert!(registry.coverage("toy").is_none());
    assert!(registry.static_analysis("toy").is_none());
}

/// A profile for the non-brace structures: `def name` is a function and
/// `class Name` a scope, as in Python and Ruby.
struct Keyworded {
    syntax: &'static LexicalSyntax,
}

const INDENT_SYNTAX: LexicalSyntax = LexicalSyntax {
    line_comments: &["#"],
    line_comment_exclusions: &[],
    block_comments: &[],
    nested_block_comments: false,
    strings: &[StringSyntax::single("\"", "\"")],
    preprocessor: false,
    cpp_raw_strings: false,
    word_suffix_chars: &[],
    newline_terminates: true,
    structure: Structure::Indentation,
};

const KEYWORD_SYNTAX: LexicalSyntax = LexicalSyntax {
    word_suffix_chars: &['?', '!'],
    structure: Structure::KeywordEnd,
    ..INDENT_SYNTAX
};

impl LanguageProfile for Keyworded {
    fn adapter_id(&self) -> &'static str {
        "keyworded"
    }

    fn languages(&self) -> &'static [&'static str] {
        &["keyworded"]
    }

    fn extensions(&self) -> &'static [&'static str] {
        &["kw"]
    }

    fn syntax(&self) -> &'static LexicalSyntax {
        self.syntax
    }

    fn classify(&self, header: &[HTok<'_>], _has_body: bool) -> HeaderKind {
        match header {
            [keyword, name, ..] if name.kind == lexer::TokKind::Word => {
                if keyword.is_word("def") {
                    HeaderKind::Function(vec![name.text.to_owned()])
                } else if keyword.is_word("class") {
                    HeaderKind::Scope(vec![name.text.to_owned()])
                } else {
                    HeaderKind::Other
                }
            }
            _ => HeaderKind::Other,
        }
    }
}

fn structure_summary(syntax: &'static LexicalSyntax, source: &str) -> (Vec<String>, Vec<String>) {
    let outcome = discover_text(&Keyworded { syntax }, "a.kw", source, "p");
    assert!(outcome.diagnostics.is_empty(), "{:?}", outcome.diagnostics);
    let sources = outcome
        .sources
        .iter()
        .map(|source| source.locator.value.clone())
        .collect();
    let tests = outcome
        .tests
        .iter()
        .map(|test| test.construct_text.clone())
        .collect();
    (sources, tests)
}

/// @vtest.id TEST-ADAPTER-LEXICAL-INDENTATION-EXTENTS
/// @vtest.covers VO-ADAPTER-LEXICAL-RETURNS-SOURCE-RANGE-AND-BYTES
/// @vtest.target crates/vtest-adapter-lexical/src/indent.rs::construct_end
/// @vtest.target crates/vtest-adapter-lexical/src/indent.rs::functions
/// @vtest.intent verifies that under the indentation structure a construct ends where its indented body ends, a decorator belongs to it, and a class qualifies the functions it indents
#[test]
fn indentation_structure_bounds_constructs_by_indent() {
    let source = "class Calc:\n    def add(self, a):\n        return a\n\n# @vtest.id TEST-KW-ADD\n# @vtest.covers VO-KW-ADD\n# @vtest.intent adds\n@mark\ndef test_add():\n    assert add(1)\n\n    assert add(2)\nvalue = 1\n";
    let (sources, tests) = structure_summary(&INDENT_SYNTAX, source);
    assert_eq!(sources, vec!["a.kw::Calc::add", "a.kw::test_add"]);
    assert_eq!(
        tests,
        vec!["@mark\ndef test_add():\n    assert add(1)\n\n    assert add(2)"]
    );
}

/// @vtest.id TEST-ADAPTER-LEXICAL-KEYWORD-END-EXTENTS
/// @vtest.covers VO-ADAPTER-LEXICAL-RETURNS-SOURCE-RANGE-AND-BYTES
/// @vtest.target crates/vtest-adapter-lexical/src/keyword.rs::construct_end
/// @vtest.target crates/vtest-adapter-lexical/src/keyword.rs::functions
/// @vtest.intent verifies that under the keyword/end structure a construct ends at the end that closes it, nested blocks do not end it early, a word suffix such as `?` stays part of a name but `!=` stays an operator, and a class qualifies its functions
#[test]
fn keyword_end_structure_pairs_openers_with_end() {
    let source = "class Calc\n  def empty?(a)\n    a != 0\n  end\nend\n\n# @vtest.id TEST-KW-ADD\n# @vtest.covers VO-KW-ADD\n# @vtest.intent adds\ndef test_add\n  [1].each do |x|\n    if x then x end\n  end\nend\n";
    let (sources, tests) = structure_summary(&KEYWORD_SYNTAX, source);
    assert_eq!(sources, vec!["a.kw::Calc::empty?", "a.kw::test_add"]);
    assert_eq!(
        tests,
        vec!["def test_add\n  [1].each do |x|\n    if x then x end\n  end\nend"]
    );
}
