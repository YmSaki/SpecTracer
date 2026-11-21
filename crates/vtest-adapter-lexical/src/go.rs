//! `go`: Go sources (`.go`), read by the rules of DS-2001 to DS-2010.
//!
//! `go` is the one adapter of this crate that does not take a Test from the
//! comment above a construct. A Go Test construct is a shape: a top-level,
//! receiverless function named `Test...` in a `_test.go` file (DS-2002). It is
//! discovered whether or not it is declared; one without a `@vtest.`
//! declaration is returned as a Missing construct with `W-SCAN-101` (DS-2006).
//! Reading "the construct after a `@vtest.` comment" would leave an undeclared
//! Test construct unobserved. `go` therefore implements
//! [`SourceDiscoveryAdapter`] itself and takes from the crate root only what
//! means the same in every language: the include walk, the Test draft builder,
//! and the `@vtest.` line grammar.
//!
//! A file is read in three steps, all specific to Go:
//!
//! 1. [`lex`] splits it into comments, literals (interpreted and raw
//!    strings, runes, numbers), words, brackets, `;`, and operators.
//! 2. [`Reader::statement_end`] finds where a declaration ends, using Go's
//!    own rule: at a `;`, or at a newline that follows an identifier, a
//!    literal, `return`, `break`, `continue`, `fallthrough`, `++`, `--`, `)`,
//!    `]`, or `}` (the places where Go inserts a semicolon), counted at the
//!    statement's own bracket depth. Brackets are skipped whole, so a newline
//!    inside a body, a parameter list, or a `struct { ... }` type does not end
//!    anything.
//! 3. [`Reader::functions`] reads the top-level `func` declarations. A
//!    function's body is its last `{}` group, unless that group belongs to a
//!    `struct` or `interface` result type (`func f() struct{}` has no body).
//!    A method is qualified by its receiver's type name. A declaration whose
//!    name cannot be read makes the file unreadable (DS-2010).
//!
//! Only a top-level `func` declaration has a `@vtest.` surface (DS-2004): its
//! doc comment, which is the comment block that ends on the line above it. On
//! a Test construct that is surface 1 (test-keys), on any other function or
//! method it is surface 2 (`src-id`). A comment anywhere else is not read.
//!
//! Build constraints (`//go:build`, `// +build`, `_<GOOS>`, `_<GOARCH>`) are
//! not evaluated (DS-2001).

use std::{
    collections::BTreeMap,
    path::{Component, Path},
};

use vtest_adapter_api::{
    Adapter, AdapterDescriptor, AdapterScanConfig, Capability, DiscoveryError, DiscoveryOutcome,
    MissingTestConstruct, SourceDiscoveryAdapter, SourceDraft,
};
use vtest_model::{
    AdapterId, Diagnostic, Locator, ProjectPath, SourceLocation, SourceRange, SrcId,
};

use crate::{
    annotation::{parse_line, parse_source, parse_test},
    discover_files, push_source_diagnostics, push_test, push_unreadable,
    shared::{attached_comments, pair_brackets, Attached, Span},
    ExtractError,
};

pub const ADAPTER_ID: &str = "go";

/// Directories the walk never enters, besides every name starting with `_` or
/// `.`: the go command does not take the packages of `testdata`, and the
/// packages in `vendor` are dependencies (DS-2001).
const EXCLUDED_DIRS: &[&str] = &["testdata", "vendor"];

/// The file name ending of a Go test file (DS-2002).
const TEST_FILE_SUFFIX: &str = "_test.go";

/// The `go` adapter: `source_discovery` only.
#[derive(Clone, Copy, Debug, Default)]
pub struct GoAdapter;

/// The `go` adapter.
pub fn adapter() -> GoAdapter {
    GoAdapter
}

impl Adapter for GoAdapter {
    fn descriptor(&self) -> AdapterDescriptor {
        AdapterDescriptor {
            id: ADAPTER_ID.to_owned(),
            languages: vec!["go".to_owned()],
            capabilities: vec![Capability::SourceDiscovery],
            config_namespace: ADAPTER_ID.to_owned(),
        }
    }

    fn as_source_discovery(&self) -> Option<&dyn SourceDiscoveryAdapter> {
        Some(self)
    }
}

impl SourceDiscoveryAdapter for GoAdapter {
    fn id(&self) -> &'static str {
        ADAPTER_ID
    }

    fn discover(
        &self,
        root: &Path,
        fallback_package: &str,
        config: &AdapterScanConfig,
    ) -> Result<DiscoveryOutcome, DiscoveryError> {
        discover_files(
            ADAPTER_ID,
            &|path| is_read(root, path),
            skipped_dir,
            root,
            config,
            |relative, source, outcome| {
                discover_source(relative, source, fallback_package, outcome);
            },
        )
    }
}

/// Whether a name is one the go command never takes a package or file from
/// (DS-2001).
fn is_ignored_name(name: &str) -> bool {
    name.starts_with('_') || name.starts_with('.')
}

/// Whether the walk stays out of the directory `name`. The project root is
/// never asked: it is wherever the project is, and its name says nothing about
/// the code in it.
fn skipped_dir(name: &str) -> bool {
    is_ignored_name(name) || EXCLUDED_DIRS.contains(&name)
}

/// Whether `path` is a `.go` file `go` reads (DS-2001): its name does not
/// start with `_` or `.`, and none of the directories between the project
/// root and it is skipped.
fn is_read(root: &Path, path: &Path) -> bool {
    let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
        return false;
    };
    if !name.ends_with(".go") || is_ignored_name(name) {
        return false;
    }
    let Ok(relative) = path.strip_prefix(root) else {
        return true;
    };
    !relative
        .parent()
        .into_iter()
        .flat_map(Path::components)
        .filter_map(|component| match component {
            Component::Normal(name) => name.to_str(),
            _ => None,
        })
        .any(skipped_dir)
}

/// Discovers one `.go` file at the project-relative path `relative`. A file
/// that cannot be read with confidence is `E-SCAN-001` and contributes
/// nothing else (DS-2010), so what is found is added only after the whole file
/// has been read.
fn discover_source(relative: &str, source: &str, package: &str, outcome: &mut DiscoveryOutcome) {
    let functions = match read_functions(source) {
        Ok(functions) => functions,
        Err(error) => {
            push_unreadable(outcome, ADAPTER_ID, relative, source.len(), &error);
            return;
        }
    };
    let in_test_file = relative.ends_with(TEST_FILE_SUFFIX);
    for function in functions {
        let is_test = function.is_test_construct(in_test_file);
        let declaration_start = function
            .doc
            .as_ref()
            .map_or(function.start, |doc| doc.start);
        let lines = function
            .doc
            .iter()
            .flat_map(|doc| &doc.lines)
            .filter_map(|line| parse_line(line))
            .collect::<Vec<_>>();
        let item = match &function.receiver {
            Some(receiver) => format!("{receiver}::{}", function.name),
            None => function.name.clone(),
        };
        let item_location = location(relative, &item, declaration_start, function.end);

        let mut src_id = None;
        if is_test {
            push_test_construct(outcome, relative, package, source, &function, lines);
        } else if !lines.is_empty() {
            let declaration = parse_source(lines);
            push_source_diagnostics(outcome, &declaration, &item_location);
            src_id = declaration.src_id;
        }
        if function.has_body {
            outcome.sources.push(SourceDraft {
                locator: Locator {
                    adapter: AdapterId::new(ADAPTER_ID),
                    value: format!("{relative}::{item}"),
                },
                src_id: src_id.map(SrcId::new),
                location: item_location,
                construct_text: source[declaration_start..function.end].to_owned(),
            });
        }
    }
}

/// Reports a Test construct: its Test draft when it is declared completely,
/// otherwise a Missing construct with the diagnostics that say why (DES-223).
/// The construct is the function from `func` to the end of its body, without
/// its doc comment (DES-681); its locator is the function's name (DES-683).
fn push_test_construct(
    outcome: &mut DiscoveryOutcome,
    relative: &str,
    package: &str,
    source: &str,
    function: &Func,
    lines: Vec<(String, String)>,
) {
    let location = location(relative, &function.name, function.start, function.end);
    let construct = &source[function.start..function.end];
    let (diagnostic, codes) = if lines.is_empty() {
        let message = format!("test function `{}` has no @vtest annotation", function.name);
        (Diagnostic::warning("W-SCAN-101", message), "W-SCAN-101")
    } else if !function.has_body {
        // The construct runs to the end of the body (DES-681); without one its
        // extent is not determinable, so a declared one is not returned as a
        // Test (DS-2010).
        let message = format!(
            "cannot determine the construct of test function `{}`: it has no body",
            function.name
        );
        (Diagnostic::error("E-SCAN-001", message), "E-SCAN-001")
    } else {
        let declaration = parse_test(lines);
        push_test(
            outcome,
            ADAPTER_ID,
            package,
            &declaration,
            location,
            construct,
        );
        return;
    };
    outcome
        .diagnostics
        .push(diagnostic.with_location(location.clone()));
    outcome.missing_tests.push(MissingTestConstruct {
        location,
        construct_text: construct.to_owned(),
        codes: vec![codes.to_owned()],
    });
}

fn location(relative: &str, locator: &str, start: usize, end: usize) -> SourceLocation {
    SourceLocation {
        adapter: AdapterId::new(ADAPTER_ID),
        path: ProjectPath::new(relative),
        locator: locator.to_owned(),
        byte_range: SourceRange {
            start: start as u64,
            end: end as u64,
        },
    }
}

/// A top-level `func` declaration.
#[derive(Debug)]
struct Func {
    /// The receiver's type name without `*` and type arguments; `None` for a
    /// function (DS-2008).
    receiver: Option<String>,
    name: String,
    /// The byte range from `func` to the last token (the closing `}` of the
    /// body when there is one); the doc comment is not in it.
    start: usize,
    end: usize,
    has_body: bool,
    /// Whether its parameters are one `*T` or `*<identifier>.T` (DES-685).
    takes_one_testing_t: bool,
    doc: Option<Doc>,
}

/// The comment block that ends on the line above a declaration (DS-2004).
#[derive(Debug)]
struct Doc {
    start: usize,
    /// Its lines with the comment markers removed (DS-2005).
    lines: Vec<String>,
}

impl Func {
    /// DS-2002: in a `_test.go` file, a function without a receiver whose
    /// name is `Test` or `Test` followed by a character that is not a
    /// lowercase letter, whatever its type arguments, parameters, and results
    /// are. `TestMain` is a Test construct only when its one parameter is a
    /// `*T` (otherwise it is the `*testing.M` main of the test binary).
    fn is_test_construct(&self, in_test_file: bool) -> bool {
        in_test_file
            && self.receiver.is_none()
            && self.name.strip_prefix("Test").is_some_and(|rest| {
                rest.chars()
                    .next()
                    .is_none_or(|first| !first.is_lowercase())
            })
            && (self.name != "TestMain" || self.takes_one_testing_t)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Kind {
    Comment,
    Literal,
    Word,
    Open(char),
    Close(char),
    Semi,
    /// `++` or `--`, after which Go inserts a semicolon at a newline.
    Step,
    Punct(char),
}

#[derive(Clone, Copy, Debug)]
struct Tok {
    kind: Kind,
    start: usize,
    end: usize,
}

/// Splits Go source into tokens. An unterminated comment or literal is an
/// error.
fn lex(source: &str) -> Result<Vec<Tok>, ExtractError> {
    let bytes = source.as_bytes();
    let mut tokens = Vec::new();
    let mut i = 0;
    while i < bytes.len() {
        let start = i;
        let rest = &source[i..];
        let byte = bytes[i];
        let kind = if byte.is_ascii_whitespace() {
            i += 1;
            continue;
        } else if rest.starts_with("//") {
            i += rest.find('\n').unwrap_or(rest.len());
            Kind::Comment
        } else if let Some(body) = rest.strip_prefix("/*") {
            let close = body
                .find("*/")
                .ok_or_else(|| ExtractError::new(start, "unterminated comment"))?;
            i += close + 4;
            Kind::Comment
        } else if byte == b'`' {
            let close = rest[1..]
                .find('`')
                .ok_or_else(|| ExtractError::new(start, "unterminated raw string"))?;
            i += close + 2;
            Kind::Literal
        } else if byte == b'"' || byte == b'\'' {
            i = quoted_end(bytes, i + 1, byte)
                .ok_or_else(|| ExtractError::new(start, "unterminated literal"))?;
            Kind::Literal
        } else if byte.is_ascii_digit()
            || (byte == b'.' && bytes.get(i + 1).is_some_and(u8::is_ascii_digit))
        {
            i += 1;
            while i < bytes.len()
                && (bytes[i].is_ascii_alphanumeric() || matches!(bytes[i], b'_' | b'.'))
            {
                i += 1;
            }
            Kind::Literal
        } else if byte == b'_' || byte.is_ascii_alphabetic() || !byte.is_ascii() {
            while i < bytes.len()
                && (bytes[i] == b'_' || bytes[i].is_ascii_alphanumeric() || !bytes[i].is_ascii())
            {
                i += 1;
            }
            Kind::Word
        } else if rest.starts_with("++") || rest.starts_with("--") {
            i += 2;
            Kind::Step
        } else {
            i += 1;
            match byte {
                b'(' | b'[' | b'{' => Kind::Open(byte as char),
                b')' | b']' | b'}' => Kind::Close(byte as char),
                b';' => Kind::Semi,
                _ => Kind::Punct(byte as char),
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

/// One past the closing `quote` of a string or rune whose contents start at
/// `i`. Interpreted strings and runes cannot span lines.
fn quoted_end(bytes: &[u8], mut i: usize, quote: u8) -> Option<usize> {
    while i < bytes.len() {
        match bytes[i] {
            b'\\' => i += 2,
            b'\n' => return None,
            byte if byte == quote => return Some(i + 1),
            _ => i += 1,
        }
    }
    None
}

/// Reads the top-level `func` declarations of one file, or says why the file
/// cannot be read.
fn read_functions(source: &str) -> Result<Vec<Func>, ExtractError> {
    let tokens = lex(source)?;
    let partners = pair_brackets(tokens.iter().map(|token| match token.kind {
        Kind::Open(c) | Kind::Close(c) => (token.start, Some(c)),
        _ => (token.start, None),
    }))?;
    Reader {
        source,
        tokens: &tokens,
        partners: &partners,
    }
    .functions()
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

    fn next_significant(&self, from: usize) -> Option<usize> {
        (from..self.tokens.len()).find(|&i| self.tokens[i].kind != Kind::Comment)
    }

    fn line_breaks(&self, before: usize, after: usize) -> usize {
        self.source[self.tokens[before].end..self.tokens[after].start]
            .matches('\n')
            .count()
    }

    /// Whether Go inserts a semicolon at a newline after `tokens[index]`.
    fn ends_line(&self, index: usize) -> bool {
        match self.tokens[index].kind {
            Kind::Literal | Kind::Step | Kind::Close(_) => true,
            Kind::Word => {
                !KEYWORDS.contains(&self.text(index))
                    || matches!(
                        self.text(index),
                        "break" | "continue" | "fallthrough" | "return"
                    )
            }
            _ => false,
        }
    }

    /// One past the last token of the declaration or statement starting at
    /// `start` (see the module documentation). A `}` or `)` closing the
    /// enclosing block also ends it.
    fn statement_end(&self, start: usize) -> usize {
        let mut index = start;
        while index < self.tokens.len() {
            let last = match self.tokens[index].kind {
                Kind::Comment => {
                    index += 1;
                    continue;
                }
                Kind::Close(_) => return index,
                Kind::Semi => return index + 1,
                Kind::Open(_) => self.partner(index),
                _ => index,
            };
            match self.next_significant(last + 1) {
                None => return last + 1,
                Some(next) if self.line_breaks(last, next) > 0 && self.ends_line(last) => {
                    return last + 1
                }
                Some(next) => index = next,
            }
        }
        index
    }

    /// Every top-level `func` declaration, in source order, with its doc
    /// comment.
    fn functions(&self) -> Result<Vec<Func>, ExtractError> {
        let spans: Vec<Span> = self
            .tokens
            .iter()
            .map(|token| Span {
                start: token.start,
                end: token.end,
                comment: token.kind == Kind::Comment,
            })
            .collect();
        let mut docs = attached_comments(self.source, &spans)
            .into_iter()
            .map(|Attached { first, last, next }| {
                let doc = Doc {
                    start: self.tokens[first].start,
                    lines: (first..=last)
                        .flat_map(|i| comment_lines(self.text(i)))
                        .collect(),
                };
                (next, doc)
            })
            .collect::<BTreeMap<_, _>>();
        let mut output = Vec::new();
        let mut index = 0;
        while let Some(start) = self.next_significant(index) {
            let end = self.statement_end(start).max(start + 1);
            if self.kind(start) == Some(Kind::Word) && self.text(start) == "func" {
                // An explicit `;` after the body (`func f() {};`) is not part
                // of the function.
                let mut last = end - 1;
                while last > start && self.kind(last) == Some(Kind::Semi) {
                    last -= 1;
                }
                output.push(self.function(start, last, docs.remove(&start))?);
            }
            index = end;
        }
        Ok(output)
    }

    /// The `func` declaration `tokens[start..=last]`.
    fn function(&self, start: usize, last: usize, doc: Option<Doc>) -> Result<Func, ExtractError> {
        let unreadable = |message: &str| ExtractError::new(self.tokens[start].start, message);
        let mut name = start + 1;
        let receiver = if self.kind(name) == Some(Kind::Open('(')) {
            let close = self.partner(name);
            let receiver = self
                .receiver_type(name, close)
                .ok_or_else(|| unreadable("method without a receiver type"))?;
            name = close + 1;
            Some(receiver)
        } else {
            None
        };
        if self.kind(name) != Some(Kind::Word) || name > last {
            return Err(unreadable("func declaration without a name"));
        }
        let has_body = self.kind(last) == Some(Kind::Close('}')) && {
            let body = self.partner(last);
            body > name
                && !(self.kind(body - 1) == Some(Kind::Word)
                    && matches!(self.text(body - 1), "struct" | "interface"))
        };
        Ok(Func {
            receiver,
            name: self.text(name).to_owned(),
            start: self.tokens[start].start,
            end: self.tokens[last].end,
            has_body,
            takes_one_testing_t: self.takes_one_testing_t(name),
            doc,
        })
    }

    /// The type name in the receiver `(c *Calc)`, `(l List[T])`, or
    /// `(Calc)`: its last word outside type arguments.
    fn receiver_type(&self, open: usize, close: usize) -> Option<String> {
        let mut index = open + 1;
        let mut found = None;
        while index < close {
            match self.tokens[index].kind {
                Kind::Word => found = Some(index),
                Kind::Open(_) => index = self.partner(index),
                _ => {}
            }
            index += 1;
        }
        found.map(|index| self.text(index).to_owned())
    }

    /// Whether the parameters of the function named by `tokens[name]` are
    /// exactly one parameter whose type is `*T` or `*<identifier>.T`, named or
    /// not (DES-685). The identifier is not checked to be the `testing`
    /// package: go does not either when it builds the test main.
    fn takes_one_testing_t(&self, name: usize) -> bool {
        let mut open = name + 1;
        if self.kind(open) == Some(Kind::Open('[')) {
            open = self.partner(open) + 1;
        }
        if self.kind(open) != Some(Kind::Open('(')) {
            return false;
        }
        let close = self.partner(open);
        let mut shape = Vec::new();
        for index in open + 1..close {
            match self.tokens[index].kind {
                Kind::Comment => {}
                Kind::Open(_) | Kind::Close(_) => return false,
                kind => shape.push((kind, self.text(index))),
            }
        }
        if shape
            .last()
            .is_some_and(|(kind, _)| *kind == Kind::Punct(','))
        {
            shape.pop();
        }
        if let [(Kind::Word, _), (Kind::Punct('*'), _), ..] = shape.as_slice() {
            shape.remove(0);
        }
        matches!(
            shape.as_slice(),
            [(Kind::Punct('*'), _), (Kind::Word, "T")]
                | [
                    (Kind::Punct('*'), _),
                    (Kind::Word, _),
                    (Kind::Punct('.'), _),
                    (Kind::Word, "T")
                ]
        )
    }
}

/// Go's keywords. Only `break`, `continue`, `fallthrough`, and `return` end
/// a line.
const KEYWORDS: &[&str] = &[
    "break",
    "case",
    "chan",
    "const",
    "continue",
    "default",
    "defer",
    "else",
    "fallthrough",
    "for",
    "func",
    "go",
    "goto",
    "if",
    "import",
    "interface",
    "map",
    "package",
    "range",
    "return",
    "select",
    "struct",
    "switch",
    "type",
    "var",
];

/// The lines of a `//` or `/* */` comment with the comment markers removed
/// (DS-2005).
fn comment_lines(comment: &str) -> Vec<String> {
    if let Some(line) = comment.strip_prefix("//") {
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
    use std::{
        env, fs,
        path::PathBuf,
        process,
        sync::atomic::{AtomicU64, Ordering},
    };

    use super::*;

    fn discover_text(relative: &str, source: &str) -> DiscoveryOutcome {
        let mut outcome = DiscoveryOutcome {
            files_scanned: 1,
            tests: Vec::new(),
            missing_tests: Vec::new(),
            sources: Vec::new(),
            diagnostics: Vec::new(),
        };
        discover_source(relative, source, "demo", &mut outcome);
        outcome
    }

    fn source_locators(outcome: &DiscoveryOutcome) -> Vec<&str> {
        outcome
            .sources
            .iter()
            .map(|source| source.locator.value.as_str())
            .collect()
    }

    fn diagnostic_codes(outcome: &DiscoveryOutcome) -> Vec<&str> {
        outcome
            .diagnostics
            .iter()
            .map(|diagnostic| diagnostic.code.as_str())
            .collect()
    }

    /// The Test names of the Missing constructs and the drafts, each in order.
    fn construct_names(outcome: &DiscoveryOutcome) -> (Vec<&str>, Vec<&str>) {
        (
            outcome
                .missing_tests
                .iter()
                .map(|missing| missing.location.locator.as_str())
                .collect(),
            outcome
                .tests
                .iter()
                .map(|test| test.location.locator.as_str())
                .collect(),
        )
    }

    static NEXT: AtomicU64 = AtomicU64::new(0);

    /// Writes `files` below a project root named `root_name` and discovers the
    /// paths `includes` (relative to that root) through the adapter itself.
    fn discover_dir(
        root_name: &str,
        files: &[(&str, &[u8])],
        includes: &[&str],
    ) -> DiscoveryOutcome {
        let parent = env::temp_dir().join(format!(
            "vtest-go-discovery-{}-{}",
            process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        let root = parent.join(root_name);
        for (path, bytes) in files {
            let full = root.join(path);
            fs::create_dir_all(full.parent().expect("parent")).expect("create fixture dir");
            fs::write(full, bytes).expect("write fixture");
        }
        let outcome = adapter()
            .discover(
                &root,
                "demo",
                &AdapterScanConfig {
                    include_paths: includes.iter().map(PathBuf::from).collect(),
                },
            )
            .expect("discovery succeeds");
        let _ = fs::remove_dir_all(&parent);
        outcome
    }

    const SOURCE: &str = r#"package calc

import "testing"

const raw = `{ not code`

type Calc struct{ base int }

// Add returns the sum.
// @vtest.src-id SRC-GO-ADD
func (c *Calc) Add(a, b int) int { return c.base + a + b }

func Max[T int | float64](a, b T) T {
	if a > b {
		return a
	}
	return b
}

func (l List[T]) Len() int { return len(l.items) }

var handler = func() {}

func assembly(x int) int

// @vtest.id TEST-GO-ADD
// @vtest.covers VO-GO-ADD
// @vtest.target calc/calc_test.go::Calc::Add
// @vtest.intent adding two numbers returns their sum
func TestAdd(t *testing.T) {
	if got := (&Calc{}).Add(1, 2); got != 3 {
		t.Fatalf("got %d", got)
	}
}
"#;

    /// @vtest.id TEST-ADAPTER-GO-DISCOVERS-FUNCS-AND-TESTS
    /// @vtest.covers VO-ADAPTER-GO-REGISTERS-WITHOUT-CORE-CHANGES, VO-ADAPTER-LEXICAL-OPAQUE-LOCATOR-SYNTAX, VO-ADAPTER-LEXICAL-RETURNS-SOURCE-RANGE-AND-BYTES
    /// @vtest.target crates/vtest-adapter-lexical/src/go.rs::discover_source
    /// @vtest.intent verifies Go functions with a body, generic functions, and methods qualified by their receiver type become Source Targets whose bytes start at their doc comment, while function literals, bodyless declarations, and types do not, and that a declared Test function is a Test construct located by its name whose bytes start at func and exclude its doc comment
    #[test]
    fn discovers_go_funcs_and_tests() {
        let outcome = discover_text("calc/calc_test.go", SOURCE);
        assert!(outcome.diagnostics.is_empty(), "{:?}", outcome.diagnostics);
        assert_eq!(
            source_locators(&outcome),
            vec![
                "calc/calc_test.go::Calc::Add",
                "calc/calc_test.go::Max",
                "calc/calc_test.go::List::Len",
                "calc/calc_test.go::TestAdd",
            ]
        );
        let add = &outcome.sources[0];
        assert_eq!(
            add.src_id.as_ref().map(|id| id.as_str()),
            Some("SRC-GO-ADD")
        );
        assert!(add
            .construct_text
            .starts_with("// Add returns the sum.\n// @vtest.src-id"));
        assert_eq!(
            &SOURCE[usize::try_from(add.location.byte_range.start).unwrap()
                ..usize::try_from(add.location.byte_range.end).unwrap()],
            add.construct_text
        );
        assert!(outcome.sources[3]
            .construct_text
            .starts_with("// @vtest.id TEST-GO-ADD"));
        assert_eq!(outcome.sources[3].src_id, None);

        assert_eq!(outcome.tests.len(), 1);
        assert!(outcome.missing_tests.is_empty());
        let test = &outcome.tests[0];
        assert_eq!(test.id.as_str(), "TEST-GO-ADD");
        assert_eq!(test.location.locator, "TestAdd");
        assert_eq!(test.execution.selector, "TestAdd");
        assert!(test
            .construct_text
            .starts_with("func TestAdd(t *testing.T) {"));
        assert!(test.construct_text.ends_with("got)\n\t}\n}"));
    }

    /// @vtest.id TEST-ADAPTER-GO-TEST-CONSTRUCT-IS-A-SHAPE
    /// @vtest.covers VO-SCAN-UNREGISTERED-TEST-W-SCAN-101
    /// @vtest.target crates/vtest-adapter-lexical/src/go.rs::discover_source
    /// @vtest.intent verifies that in a _test.go file a top-level receiverless function named Test or Test followed by a character that is not a lowercase letter is a Test construct whatever its type arguments, parameters, and results, and that a method, a lowercase continuation, Benchmark, Fuzz, Example, and a *testing.M TestMain are not, and that the same names in a file not ending in _test.go are not
    #[test]
    fn test_construct_is_decided_by_its_shape() {
        let source = "package calc\n\nimport \"testing\"\n\n\
            func Test(t *testing.T) {}\n\
            func TestUpper(t *testing.T) {}\n\
            func Test1(t *testing.T) {}\n\
            func Test_snake(t *testing.T) {}\n\
            func Test\u{c4}(t *testing.T) {}\n\
            func Test\u{65e5}\u{672c}(t *testing.T) {}\n\
            func TestGeneric[T any](t *testing.T) {}\n\
            func TestWrongShape() int { return 1 }\n\
            func Testlower(t *testing.T) {}\n\
            func testLower(t *testing.T) {}\n\
            func BenchmarkX(b *testing.B) {}\n\
            func FuzzX(f *testing.F) {}\n\
            func ExampleX() {}\n\
            func (s *Suite) TestMethod(t *testing.T) {}\n\
            func TestMain(m *testing.M) {}\n";
        let outcome = discover_text("calc/calc_test.go", source);
        assert_eq!(
            construct_names(&outcome),
            (
                vec![
                    "Test",
                    "TestUpper",
                    "Test1",
                    "Test_snake",
                    "Test\u{c4}",
                    "Test\u{65e5}\u{672c}",
                    "TestGeneric",
                    "TestWrongShape",
                ],
                vec![]
            )
        );
        assert!(diagnostic_codes(&outcome)
            .iter()
            .all(|code| *code == "W-SCAN-101"));
        assert_eq!(outcome.diagnostics.len(), 8);

        let elsewhere = discover_text("calc/calc.go", source);
        assert_eq!(construct_names(&elsewhere), (vec![], vec![]));
        assert!(
            elsewhere.diagnostics.is_empty(),
            "{:?}",
            elsewhere.diagnostics
        );
    }

    /// @vtest.id TEST-ADAPTER-GO-TESTMAIN-IS-A-TEST-ONLY-FOR-TESTING-T
    /// @vtest.covers VO-SCAN-UNREGISTERED-TEST-W-SCAN-101
    /// @vtest.target crates/vtest-adapter-lexical/src/go.rs::Reader::takes_one_testing_t
    /// @vtest.intent verifies that TestMain is a Test construct exactly when its parameters are one *T or *identifier.T, named or not, with or without a trailing comma, and is not for *testing.M, no parameter, two parameters, a value or a deeper-qualified type
    #[test]
    fn testmain_is_a_test_construct_only_with_one_pointer_to_t() {
        for (parameters, is_test) in [
            ("t *testing.T", true),
            ("t *tt.T", true),
            ("*testing.T", true),
            ("_ *testing.T", true),
            ("t *T", true),
            ("t *testing.T,", true),
            ("t /* the test */ *testing.T", true),
            ("m *testing.M", false),
            ("", false),
            ("t *testing.T, x int", false),
            ("a, b *testing.T", false),
            ("t testing.T", false),
            ("t *a.b.T", false),
            ("t ...*testing.T", false),
            ("t []*testing.T", false),
        ] {
            let source = format!("package calc\n\nfunc TestMain({parameters}) {{}}\n");
            let outcome = discover_text("calc/main_test.go", &source);
            assert_eq!(
                outcome.missing_tests.len(),
                usize::from(is_test),
                "TestMain({parameters})"
            );
        }
        let generic = discover_text(
            "calc/main_test.go",
            "package calc\n\nfunc TestMain[T any](t *testing.T) {}\n",
        );
        assert_eq!(generic.missing_tests.len(), 1);
    }

    /// @vtest.id TEST-ADAPTER-GO-UNDECLARED-TEST-IS-RETURNED-WITH-W-SCAN-101
    /// @vtest.covers VO-SCAN-UNREGISTERED-TEST-W-SCAN-101, VO-SCAN-DISCOVERED-SET-RETAINS-UNMANAGED-CONSTRUCTS
    /// @vtest.target crates/vtest-adapter-lexical/src/go.rs::push_test_construct
    /// @vtest.intent verifies that a Test construct with no @vtest. line, with no comment or with a comment without one or with a declaration separated by a blank line, is returned as a Missing construct that carries W-SCAN-101, located by its name over the function from func to the end of its body, and that no Test draft and no other diagnostic is produced
    #[test]
    fn undeclared_test_is_returned_with_w_scan_101() {
        let source = "package calc\n\n\
            func TestBare(t *testing.T) {\n\tcheck(t)\n}\n\n\
            // TestDocumented checks the sum.\n\
            func TestDocumented(t *testing.T) {}\n\n\
            // @vtest.id TEST-DETACHED\n\
            // @vtest.covers VO-GO-ADD\n\
            // @vtest.intent separated by a blank line\n\n\
            func TestDetached(t *testing.T) {}\n";
        let outcome = discover_text("calc/calc_test.go", source);
        assert_eq!(
            construct_names(&outcome),
            (vec!["TestBare", "TestDocumented", "TestDetached"], vec![])
        );
        assert_eq!(diagnostic_codes(&outcome), vec!["W-SCAN-101"; 3]);
        assert!(outcome
            .diagnostics
            .iter()
            .all(|diagnostic| !diagnostic.is_error()));
        assert!(outcome
            .missing_tests
            .iter()
            .all(|missing| missing.codes == vec!["W-SCAN-101".to_owned()]));
        assert_eq!(
            outcome.missing_tests[0].construct_text,
            "func TestBare(t *testing.T) {\n\tcheck(t)\n}"
        );
        // The Test function is a Source Target too, with its doc comment.
        assert_eq!(
            source_locators(&outcome),
            vec![
                "calc/calc_test.go::TestBare",
                "calc/calc_test.go::TestDocumented",
                "calc/calc_test.go::TestDetached"
            ]
        );
        assert!(outcome.sources[1]
            .construct_text
            .starts_with("// TestDocumented checks the sum.\nfunc TestDocumented"));
    }

    /// @vtest.id TEST-ADAPTER-GO-METHOD-DECLARATION-IS-W-SCAN-105-AND-NO-TEST
    /// @vtest.covers VO-SCAN-NON-TEST-UNKNOWN-KEY-W-SCAN-105
    /// @vtest.target crates/vtest-adapter-lexical/src/go.rs::discover_source
    /// @vtest.intent verifies that a method carrying a complete Test declaration, even in a _test.go file and named Test..., is not a Test construct: each of its test-key lines is W-SCAN-105, no Test draft and no Missing construct and no error is produced, and the method is still a Source Target qualified by its receiver
    #[test]
    fn declaration_on_a_method_is_w_scan_105_and_no_test() {
        let source = "package calc\n\n\
            // @vtest.id TEST-SUITE-ADD\n\
            // @vtest.covers VO-GO-ADD\n\
            // @vtest.intent a suite method is not a Test\n\
            func (s *Suite) TestAdd(t *testing.T) {}\n";
        let outcome = discover_text("calc/suite_test.go", source);
        assert_eq!(construct_names(&outcome), (vec![], vec![]));
        assert_eq!(diagnostic_codes(&outcome), vec!["W-SCAN-105"; 3]);
        assert!(outcome
            .diagnostics
            .iter()
            .all(|diagnostic| !diagnostic.is_error()));
        assert_eq!(
            source_locators(&outcome),
            vec!["calc/suite_test.go::Suite::TestAdd"]
        );
        assert_eq!(outcome.sources[0].src_id, None);
    }

    /// @vtest.id TEST-ADAPTER-GO-SOURCE-SURFACE-READS-SRC-ID
    /// @vtest.covers VO-SCAN-SRC-ID-DECLARED-ON-IMPLEMENTATION, VO-SCAN-NON-TEST-UNKNOWN-KEY-W-SCAN-105
    /// @vtest.target crates/vtest-adapter-lexical/src/go.rs::discover_source
    /// @vtest.intent verifies that the doc comment of a function that is not a Test construct adopts a single src-id, adopts none and reports E-SCAN-005 for a repeated one, reports W-SCAN-105 for any other key, and that a Test-named function in a file not ending in _test.go is on this surface and no Test
    #[test]
    fn source_surface_reads_src_id_and_reports_other_keys() {
        let source = "package calc\n\n\
            // @vtest.src-id SRC-ONE\n\
            func One() {}\n\n\
            // @vtest.src-id SRC-A\n\
            // @vtest.src-id SRC-B\n\
            func Two() {}\n\n\
            // @vtest.src_id SRC-TYPO\n\
            func Three() {}\n\n\
            // @vtest.id TEST-NOT-IN-A-TEST-FILE\n\
            func TestElsewhere(t *testing.T) {}\n";
        let outcome = discover_text("calc/calc.go", source);
        assert_eq!(construct_names(&outcome), (vec![], vec![]));
        assert_eq!(
            diagnostic_codes(&outcome),
            vec!["E-SCAN-005", "W-SCAN-105", "W-SCAN-105"]
        );
        let src_ids = outcome
            .sources
            .iter()
            .map(|source| source.src_id.as_ref().map(|id| id.as_str()))
            .collect::<Vec<_>>();
        assert_eq!(src_ids, vec![Some("SRC-ONE"), None, None, None]);
    }

    /// @vtest.id TEST-ADAPTER-GO-TEST-SURFACE-GRAMMAR
    /// @vtest.covers VO-SCAN-UNKNOWN-ANNOTATION-KEY-E-SCAN-006, VO-ADAPTER-LEXICAL-REPORTS-DECLARATION-VIOLATIONS
    /// @vtest.target crates/vtest-adapter-lexical/src/go.rs::push_test_construct
    /// @vtest.intent verifies that on a Test construct a src-id line is E-SCAN-006, a repeated id is E-SCAN-005, a missing intent is E-SCAN-007, each making a Missing construct with its own code and no Test draft, and that a /* */ comment is read line by line without its leading stars, with repeated case and target lines and comma-separated covers
    #[test]
    fn test_surface_grammar_and_comment_forms() {
        let source = "package calc\n\n\
            // @vtest.src-id SRC-MISPLACED\n\
            func TestSrcOnTest(t *testing.T) {}\n\n\
            // @vtest.id TEST-A\n\
            // @vtest.id TEST-B\n\
            // @vtest.covers VO-GO-ADD\n\
            // @vtest.intent twice\n\
            func TestDuplicateId(t *testing.T) {}\n\n\
            // @vtest.id TEST-NO-INTENT\n\
            // @vtest.covers VO-GO-ADD\n\
            func TestNoIntent(t *testing.T) {}\n\n\
            /*\n\
             * @vtest.id TEST-BLOCK\n\
             * @vtest.covers VO-GO-ADD, VO-GO-MAX\n\
             * @vtest.target calc/calc.go::Add\n\
             * @vtest.target calc/calc.go::Max\n\
             * @vtest.case first\n\
             * @vtest.case second\n\
             * @vtest.intent a block comment is read line by line\n\
             */\n\
            func TestBlock(t *testing.T) {}\n";
        let outcome = discover_text("calc/calc_test.go", source);
        assert_eq!(
            construct_names(&outcome),
            (
                vec!["TestSrcOnTest", "TestDuplicateId", "TestNoIntent"],
                vec!["TestBlock"]
            )
        );
        let missing_codes = outcome
            .missing_tests
            .iter()
            .map(|missing| missing.codes.clone())
            .collect::<Vec<_>>();
        assert_eq!(
            missing_codes,
            vec![
                vec!["E-SCAN-006".to_owned()],
                vec!["E-SCAN-005".to_owned()],
                vec!["E-SCAN-007".to_owned()]
            ]
        );
        let test = &outcome.tests[0];
        assert_eq!(test.id.as_str(), "TEST-BLOCK");
        assert_eq!(test.covers.len(), 2);
        assert_eq!(test.targets.len(), 2);
        assert_eq!(test.cases, vec!["first", "second"]);
    }

    /// @vtest.id TEST-ADAPTER-GO-ONLY-FUNC-DECLARATIONS-ARE-SURFACES
    /// @vtest.covers VO-ADAPTER-LEXICAL-NORMALIZES-DECLARATION
    /// @vtest.target crates/vtest-adapter-lexical/src/go.rs::Reader::functions
    /// @vtest.intent verifies that a @vtest. declaration on a statement inside a function body (a subtest call), on a type, var, or const declaration, or one that is not attached to a func declaration yields no Test, no Source Target declaration, and no diagnostic, and that a doc comment attaches only when it ends on the line above the declaration
    #[test]
    fn only_func_declarations_carry_a_declaration() {
        let source = "package calc\n\n\
            func TestAll(t *testing.T) {\n\
            \t// @vtest.id TEST-GO-SUB\n\
            \t// @vtest.covers VO-GO-ADD\n\
            \t// @vtest.intent a subtest is not a Test construct\n\
            \tt.Run(\"adds\", func(t *testing.T) {\n\t\tcheck(t)\n\t})\n\
            }\n\n\
            // @vtest.id TEST-ON-TYPE\n\
            type Suite struct{}\n\n\
            // @vtest.src-id SRC-ON-VAR\n\
            var handler = 1\n\n\
            // @vtest.src-id SRC-DETACHED\n\n\
            func Detached() {}\n";
        let outcome = discover_text("calc/all_test.go", source);
        assert_eq!(construct_names(&outcome), (vec!["TestAll"], vec![]));
        assert_eq!(diagnostic_codes(&outcome), vec!["W-SCAN-101"]);
        let detached = outcome
            .sources
            .iter()
            .find(|source| source.locator.value.ends_with("::Detached"))
            .expect("Detached is a Source Target");
        assert_eq!(detached.src_id, None);
        assert!(detached.construct_text.starts_with("func Detached"));
    }

    /// @vtest.id TEST-ADAPTER-GO-STRUCT-RESULT-IS-NOT-THE-BODY
    /// @vtest.covers VO-ADAPTER-GO-REGISTERS-WITHOUT-CORE-CHANGES
    /// @vtest.target crates/vtest-adapter-lexical/src/go.rs::Reader::statement_end
    /// @vtest.intent verifies that the braces of a struct or interface result type are not taken for the function body, so a Test construct runs to the end of the body and an edit to only the body's tail changes its text, and that a function whose last braces are a struct result type has no body
    #[test]
    fn struct_result_braces_are_not_the_body() {
        let source = |tail: &str| {
            format!(
                "package calc\n\n// @vtest.id TEST-GO-STRUCT\n// @vtest.covers VO-GO-ADD\n// @vtest.intent the whole body is the construct\nfunc TestPair(t *testing.T) struct {{\n\tX int\n}} {{\n\tmarker()\n\t{tail}\n\treturn struct{{ X int }}{{1}}\n}}\n\nfunc declared() struct{{}}\n"
            )
        };
        let outcome = discover_text("calc/pair_test.go", &source("one()"));
        assert!(outcome.diagnostics.is_empty(), "{:?}", outcome.diagnostics);
        assert!(outcome.missing_tests.is_empty());
        assert_eq!(
            source_locators(&outcome),
            vec!["calc/pair_test.go::TestPair"]
        );
        let test = &outcome.tests[0];
        assert!(test.construct_text.starts_with("func TestPair("));
        assert!(test
            .construct_text
            .ends_with("one()\n\treturn struct{ X int }{1}\n}"));
        let edited = discover_text("calc/pair_test.go", &source("two()"));
        assert_ne!(edited.tests[0].construct_text, test.construct_text);
    }

    /// @vtest.id TEST-ADAPTER-GO-EXPLICIT-SEMICOLON
    /// @vtest.covers VO-ADAPTER-LEXICAL-NORMALIZES-DECLARATION
    /// @vtest.target crates/vtest-adapter-lexical/src/go.rs::Reader::function
    /// @vtest.intent verifies a function followed by an explicit semicolon is found with the same name and SRC ID as one ended by a newline
    #[test]
    fn explicit_semicolon_after_a_body() {
        for source in [
            "package p\n// @vtest.src-id SRC-GO-F\nfunc f() {}\n",
            "package p;\n// @vtest.src-id SRC-GO-F\nfunc f() {};\n",
            "package p\n// @vtest.src-id SRC-GO-F\nfunc f() {}; func g() {}\n",
        ] {
            let outcome = discover_text("p/p.go", source);
            assert!(outcome.diagnostics.is_empty(), "{:?}", outcome.diagnostics);
            assert_eq!(outcome.sources[0].locator.value, "p/p.go::f", "{source}");
            let range = &outcome.sources[0].location.byte_range;
            assert!(
                source[usize::try_from(range.start).unwrap()..usize::try_from(range.end).unwrap()]
                    .ends_with("{}"),
                "{source}"
            );
            assert_eq!(
                outcome.sources[0].src_id.as_ref().map(|id| id.as_str()),
                Some("SRC-GO-F"),
                "{source}"
            );
        }
    }

    /// @vtest.id TEST-ADAPTER-GO-DUPLICATE-LOCATORS-ARE-ALL-RETURNED
    /// @vtest.covers VO-SCAN-TARGET-UNRESOLVABLE-E-SCAN-004
    /// @vtest.target crates/vtest-adapter-lexical/src/go.rs::discover_source
    /// @vtest.intent verifies that two init functions and two functions named _ in one file are all returned as Source Targets with the same locator, so that core resolves a Target Reference to either one as ambiguous instead of the adapter picking one
    #[test]
    fn duplicate_locators_are_all_returned() {
        let outcome = discover_text(
            "p/p.go",
            "package p\n\nfunc init() {}\nfunc init() {}\nfunc _() {}\nfunc _() {}\nfunc once() {}\n",
        );
        assert!(outcome.diagnostics.is_empty(), "{:?}", outcome.diagnostics);
        assert_eq!(
            source_locators(&outcome),
            vec![
                "p/p.go::init",
                "p/p.go::init",
                "p/p.go::_",
                "p/p.go::_",
                "p/p.go::once"
            ]
        );
    }

    /// @vtest.id TEST-ADAPTER-GO-UNREADABLE-FILE-IS-E-SCAN-001
    /// @vtest.covers VO-ADAPTER-LEXICAL-UNPARSEABLE-DECLARATION-MISSING
    /// @vtest.target crates/vtest-adapter-lexical/src/go.rs::discover_source
    /// @vtest.intent verifies that a file with an unterminated raw string, an unclosed bracket, or a func without a name is E-SCAN-001 and returns no Test, no Missing construct, and no Source Target even when valid declarations precede the fault, and that a declared Test function without a body is a Missing construct with E-SCAN-001 instead of a Test while an undeclared one is W-SCAN-101
    #[test]
    fn unreadable_files_are_e_scan_001_and_contribute_nothing() {
        let valid = "// @vtest.id TEST-GO-OK\n// @vtest.covers VO-GO-ADD\n// @vtest.intent valid\nfunc TestOk(t *testing.T) {}\n\nfunc Helper() {}\n\n";
        for (name, fault) in [
            ("raw string", "const raw = `open\n"),
            ("bracket", "func Broken() {\n"),
            ("name", "func () {}\n"),
            ("comment", "/* open\n"),
            ("literal", "var s = \"open\n"),
        ] {
            let outcome = discover_text(
                "calc/broken_test.go",
                &format!("package calc\n\n{valid}{fault}"),
            );
            assert_eq!(diagnostic_codes(&outcome), vec!["E-SCAN-001"], "{name}");
            assert!(outcome.tests.is_empty(), "{name}");
            assert!(outcome.missing_tests.is_empty(), "{name}");
            assert!(outcome.sources.is_empty(), "{name}");
        }

        let bodyless = discover_text(
            "calc/calc_test.go",
            "package calc\n\n// @vtest.id TEST-GO-NOBODY\n// @vtest.covers VO-GO-ADD\n// @vtest.intent the extent is unknown\nfunc TestNoBody(t *testing.T)\n",
        );
        assert_eq!(diagnostic_codes(&bodyless), vec!["E-SCAN-001"]);
        assert!(bodyless.tests.is_empty());
        assert_eq!(bodyless.missing_tests.len(), 1);
        assert_eq!(bodyless.missing_tests[0].location.locator, "TestNoBody");
        assert_eq!(
            bodyless.missing_tests[0].codes,
            vec!["E-SCAN-001".to_owned()]
        );
        assert!(bodyless.sources.is_empty());

        let undeclared = discover_text(
            "calc/calc_test.go",
            "package calc\n\nfunc TestNoBody(t *testing.T)\n",
        );
        assert_eq!(diagnostic_codes(&undeclared), vec!["W-SCAN-101"]);
        assert_eq!(undeclared.missing_tests.len(), 1);
        assert_eq!(
            undeclared.missing_tests[0].codes,
            vec!["W-SCAN-101".to_owned()]
        );
    }

    const BROKEN: &[u8] = b"package x\n\nconst raw = `open\n";
    const GOOD: &[u8] = b"package x\n\nfunc TestGood(t *testing.T) {}\n";

    /// @vtest.id TEST-ADAPTER-GO-WALK-RULES
    /// @vtest.covers VO-ADAPTER-GO-REGISTERS-WITHOUT-CORE-CHANGES
    /// @vtest.target crates/vtest-adapter-lexical/src/go.rs::GoAdapter::discover
    /// @vtest.intent verifies that files whose names start with _ or ., and every file inside a directory named testdata or vendor or starting with _ or ., are not read (an unreadable file there gives no E-SCAN-001), that files with build constraints in their text or GOOS and GOARCH suffixes in their name and files in a directory named target are read, and that a project root whose own name starts with . is still walked
    #[test]
    fn walk_reads_every_go_file_except_the_ignored_names() {
        let files: &[(&str, &[u8])] = &[
            ("calc/good_test.go", GOOD),
            (
                "calc/constrained_linux_amd64_test.go",
                b"//go:build ignore\n// +build ignore\n\npackage x\n\nfunc TestConstrained(t *testing.T) {}\n",
            ),
            ("target/good_test.go", GOOD),
            ("calc/_underscore_test.go", BROKEN),
            ("calc/.hidden_test.go", BROKEN),
            ("calc/testdata/broken_test.go", BROKEN),
            ("testdata/broken_test.go", BROKEN),
            ("vendor/dep/broken_test.go", BROKEN),
            ("calc/vendor/broken_test.go", BROKEN),
            ("calc/_dir/broken_test.go", BROKEN),
            ("calc/.dir/broken_test.go", BROKEN),
            ("calc/notes.txt", BROKEN),
        ];
        for root_name in ["project", ".project"] {
            let outcome = discover_dir(root_name, files, &["."]);
            assert!(
                outcome.diagnostics.iter().all(|d| d.code == "W-SCAN-101"),
                "{root_name}: {:?}",
                outcome.diagnostics
            );
            let mut names = outcome
                .sources
                .iter()
                .map(|source| source.locator.value.as_str())
                .collect::<Vec<_>>();
            names.sort_unstable();
            assert_eq!(
                names,
                vec![
                    "calc/constrained_linux_amd64_test.go::TestConstrained",
                    "calc/good_test.go::TestGood",
                    "target/good_test.go::TestGood",
                ],
                "{root_name}"
            );
            assert_eq!(outcome.files_scanned, 3, "{root_name}");
        }

        for include in [
            "calc/_underscore_test.go",
            "calc/.hidden_test.go",
            "calc/testdata/broken_test.go",
            "vendor/dep/broken_test.go",
            "calc/_dir",
            "vendor",
        ] {
            let outcome = discover_dir("project", files, &[include]);
            assert_eq!(outcome.files_scanned, 0, "{include}");
            assert!(outcome.diagnostics.is_empty(), "{include}");
        }
        let explicit = discover_dir("project", files, &["calc/good_test.go"]);
        assert_eq!(explicit.files_scanned, 1);
    }

    /// @vtest.id TEST-ADAPTER-GO-UNREADABLE-FILE-ON-DISK-IS-E-SCAN-001
    /// @vtest.covers VO-ADAPTER-LEXICAL-UNPARSEABLE-DECLARATION-MISSING
    /// @vtest.target crates/vtest-adapter-lexical/src/go.rs::GoAdapter::discover
    /// @vtest.intent verifies that through the adapter a file that is not UTF-8 text and a file with an unterminated raw string are each E-SCAN-001 without hiding the Tests of the readable files beside them
    #[test]
    fn unreadable_files_on_disk_do_not_hide_readable_ones() {
        let outcome = discover_dir(
            "project",
            &[
                ("calc/good_test.go", GOOD),
                ("calc/binary_test.go", b"package x\n\xff\xfe\n"),
                ("calc/open_test.go", BROKEN),
            ],
            &["."],
        );
        // The files are read in path order: binary, good, open.
        assert_eq!(
            diagnostic_codes(&outcome),
            vec!["E-SCAN-001", "W-SCAN-101", "E-SCAN-001"]
        );
        assert_eq!(outcome.missing_tests.len(), 1);
        assert_eq!(
            outcome.missing_tests[0].location.path.as_str(),
            "calc/good_test.go"
        );
        assert_eq!(outcome.files_scanned, 3);
    }
}
