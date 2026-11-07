use std::{
    env, fs,
    sync::atomic::{AtomicU64, Ordering},
};

use super::*;

/// A line-oriented language used only by these tests, so that they exercise
/// the crate root through [`LanguageProfile`] and nothing else:
///
/// - `// text` is a comment line; consecutive comment lines form a block,
///   and the next line (with no blank line between) is its construct;
/// - `scope NAME {` ... `}` qualifies the functions inside it;
/// - `fn NAME ...` is a one-line function definition;
/// - a construct line `??` cannot be read (its extent is unknown);
/// - a line containing `!!` makes the whole file unreadable.
struct Toy;

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

    fn extract(&self, source: &str) -> Result<FileItems, ExtractError> {
        let mut items = FileItems::default();
        let mut scope = Vec::new();
        let mut comment: Option<(usize, Vec<String>)> = None;
        let mut offset = 0;
        for raw in source.split_inclusive('\n') {
            let start = offset;
            offset += raw.len();
            let line = raw.trim_end_matches('\n');
            let indent = line.len() - line.trim_start().len();
            let text = line.trim();
            if text.contains("!!") {
                return Err(ExtractError {
                    offset: start,
                    message: "unreadable line".to_owned(),
                });
            }
            if let Some(body) = text.strip_prefix("//") {
                let block = comment.get_or_insert((start + indent, Vec::new()));
                block.1.push(body.to_owned());
                continue;
            }
            let construct = (start + indent, start + line.len());
            if let Some((comment_start, lines)) = comment.take() {
                if !text.is_empty() && text != "}" {
                    items.annotated.push(Annotated {
                        comment_start,
                        lines,
                        construct: if text == "??" {
                            Err("unknown extent".to_owned())
                        } else {
                            Ok(construct)
                        },
                    });
                }
            }
            if let Some(name) = text.strip_prefix("scope ") {
                scope.push(name.trim_end_matches('{').trim().to_owned());
            } else if text == "}" {
                scope.pop();
            } else if let Some(rest) = text.strip_prefix("fn ") {
                let mut name = scope.clone();
                name.push(
                    rest.split_whitespace()
                        .next()
                        .unwrap_or_default()
                        .to_owned(),
                );
                items.functions.push(Function {
                    name,
                    start: construct.0,
                    end: construct.1,
                });
            }
        }
        Ok(items)
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

const CALCULATOR: &str = "scope Calculator {
  // Adds two numbers.
  // @vtest.src-id SRC-TOY-ADD
  fn add a b
  fn sub a b
}
scope CalculatorTest {
  // @vtest.id TEST-TOY-ADD
  // @vtest.covers VO-TOY-ADD, VO-TOY-MATH
  // @vtest.target src/calc.toy::Calculator::sub
  // @vtest.target SRC-TOY-ADD
  // @vtest.intent adds
  fn addsNumbers check add 1 2
}
// @vtest.id TEST-TOY-CALL-STYLE
// @vtest.covers VO-TOY-ADD
// @vtest.intent a construct that is not a function is a Test construct too
test adds
";

/// @vtest.id TEST-ADAPTER-LEXICAL-DISCOVERS-ANNOTATED-CONSTRUCTS
/// @vtest.covers VO-ADAPTER-LEXICAL-NORMALIZES-DECLARATION, VO-ADAPTER-LEXICAL-OPAQUE-LOCATOR-SYNTAX
/// @vtest.target crates/vtest-adapter-lexical/src/lib.rs::LexicalAdapter::discover
/// @vtest.intent verifies an annotated function and an annotated non-function construct are both Test constructs with normalized metadata, and every function the profile returns is a Source Target with a scope-qualified locator and its declared src-id
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
            ("src/calc.toy::Calculator::add", Some("SRC-TOY-ADD")),
            ("src/calc.toy::Calculator::sub", None),
            ("src/calc.toy::CalculatorTest::addsNumbers", None),
        ]
    );

    assert_eq!(outcome.tests.len(), 2);
    let function = &outcome.tests[0];
    assert_eq!(function.id.as_str(), "TEST-TOY-ADD");
    assert_eq!(
        function.covers.iter().map(VoId::as_str).collect::<Vec<_>>(),
        vec!["VO-TOY-ADD", "VO-TOY-MATH"]
    );
    assert_eq!(
        function.targets,
        vec![
            TargetRef::Locator(Locator {
                adapter: AdapterId::new("toy"),
                value: "src/calc.toy::Calculator::sub".to_owned(),
            }),
            TargetRef::SrcId(SrcId::new("SRC-TOY-ADD")),
        ]
    );
    assert_eq!(function.location.locator, "CalculatorTest::addsNumbers");
    assert_eq!(function.execution.selector, "CalculatorTest::addsNumbers");
    assert_eq!(function.execution.project.as_deref(), Some("toy-project"));
    assert_eq!(function.construct_text, "fn addsNumbers check add 1 2");

    let other = &outcome.tests[1];
    assert_eq!(other.id.as_str(), "TEST-TOY-CALL-STYLE");
    assert_eq!(other.location.locator, "test#2");
    assert_eq!(other.construct_text, "test adds");
}

/// @vtest.id TEST-ADAPTER-LEXICAL-RANGES-MATCH-CURRENT-BYTES
/// @vtest.covers VO-ADAPTER-LEXICAL-RETURNS-SOURCE-RANGE-AND-BYTES
/// @vtest.target crates/vtest-adapter-lexical/src/lib.rs::LexicalAdapter::discover
/// @vtest.intent verifies every returned Test and Source Target byte range selects exactly its construct bytes in the current file, with a declared Source Target including its comment block
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
    assert_eq!(
        outcome.sources[0].construct_text,
        "// Adds two numbers.\n  // @vtest.src-id SRC-TOY-ADD\n  fn add a b"
    );
}

/// @vtest.id TEST-ADAPTER-LEXICAL-MALFORMED-DECLARATION-IS-MISSING
/// @vtest.covers VO-ADAPTER-LEXICAL-UNPARSEABLE-DECLARATION-MISSING, VO-ADAPTER-LEXICAL-REPORTS-DECLARATION-VIOLATIONS
/// @vtest.target crates/vtest-adapter-lexical/src/lib.rs::LexicalAdapter::discover
/// @vtest.intent verifies declarations with a grammar error or a missing required key are returned as Missing Test constructs with E-SCAN-006 or E-SCAN-007 and no Test draft
#[test]
fn malformed_declarations_are_missing_tests_with_diagnostics() {
    let outcome = discover(&[(
        "t.toy",
        "// @vtest.id TEST-A\n// @vtest.covers VO-A\n// @vtest.intnet typo\nfn a\n\n\
         // @vtest.id TEST-B\n// @vtest.intent no covers\nfn b\n",
    )]);
    assert!(outcome.tests.is_empty());
    assert_eq!(outcome.missing_tests.len(), 2);
    let codes = outcome
        .diagnostics
        .iter()
        .map(|diagnostic| diagnostic.code.as_str())
        .collect::<Vec<_>>();
    assert_eq!(codes, vec!["E-SCAN-006", "E-SCAN-007"]);
    assert_eq!(outcome.missing_tests[0].construct_text, "fn a");
}

/// @vtest.id TEST-ADAPTER-LEXICAL-UNKNOWN-EXTENT-IS-MISSING
/// @vtest.covers VO-ADAPTER-LEXICAL-UNPARSEABLE-DECLARATION-MISSING, VO-ADAPTER-LEXICAL-RETURNS-SOURCE-RANGE-AND-BYTES
/// @vtest.target crates/vtest-adapter-lexical/src/lib.rs::LexicalAdapter::discover
/// @vtest.intent verifies a declared Test whose construct extent the profile cannot determine is a Missing Test construct with E-SCAN-001 and never a Test draft over a fragment, while other Tests of the file are still discovered
#[test]
fn an_unknown_extent_is_missing_not_a_fragment() {
    let outcome = discover(&[(
        "t.toy",
        "// @vtest.id TEST-A\n// @vtest.covers VO-A\n// @vtest.intent a\n??\n\n\
         // @vtest.id TEST-B\n// @vtest.covers VO-B\n// @vtest.intent b\nfn b\n",
    )]);
    assert_eq!(
        outcome
            .tests
            .iter()
            .map(|t| t.id.as_str())
            .collect::<Vec<_>>(),
        vec!["TEST-B"]
    );
    assert_eq!(outcome.missing_tests.len(), 1);
    assert_eq!(outcome.diagnostics.len(), 1);
    assert_eq!(outcome.diagnostics[0].code, "E-SCAN-001");
}

/// @vtest.id TEST-ADAPTER-LEXICAL-UNBALANCED-FILE-E-SCAN-001
/// @vtest.covers VO-ADAPTER-LEXICAL-RETURNS-SOURCE-RANGE-AND-BYTES
/// @vtest.target crates/vtest-adapter-lexical/src/lib.rs::LexicalAdapter::discover
/// @vtest.intent verifies a file the profile cannot read yields E-SCAN-001 and no guessed constructs, without stopping discovery of other files or descending into excluded directories
#[test]
fn unreadable_file_is_e_scan_001_and_contributes_nothing() {
    let outcome = discover(&[
        (
            "a.toy",
            "// @vtest.id T\n// @vtest.covers VO\n// @vtest.intent x\nfn a !!\n",
        ),
        ("b.toy", "fn b\n"),
        ("vendor/c.toy", "fn c\n"),
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
