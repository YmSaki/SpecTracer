//! Framework-neutral source discovery for languages other than Rust.
//!
//! `vtest-adapter-rust` discovers Tests from Rust's `#[test]` attribute and
//! reads their declarations from doc comments. Other ecosystems have many
//! test frameworks (pytest and unittest, JUnit and TestNG, Jest and Vitest
//! and Mocha, GoogleTest and Catch2, ...), so this crate does not recognize a
//! Test by a framework's syntax. Instead, **a Test construct is the construct
//! that immediately follows a comment block declaring at least one
//! `@vtest.` test-key**. The same rule finds a JUnit method, a Go
//! `func TestX`, a `test("...", () => { ... })` call, and a GoogleTest
//! `TEST(Suite, Name) { ... }` block.
//!
//! Each language is a [`LanguageProfile`]: its lexical syntax, the file
//! extensions it owns, and how it tells a function definition (a Source
//! Target) and a named scope from other statements. [`LexicalAdapter`]
//! turns a profile into a registered adapter that declares only the
//! `source_discovery` capability (DES-350). It declares no test runner,
//! coverage, or static analysis, because those depend on the test framework
//! and tools a project chooses; verification therefore reports
//! `target_binding` as `NO_EVIDENCE` (`NOT_EXECUTED`, `NOT_CHECKED`) and
//! `oracle_presence` as `NO_EVIDENCE` (`NOT_CHECKED`) for these Tests, and
//! `vtest run` rejects them with `E-ADAPTER-004`, as for the synthetic
//! adapter.
//!
//! # Declarations
//!
//! The grammar is `rust-cargo`'s test-annotation-line (DS-487), read from
//! the comment block that immediately precedes a construct (see
//! [`annotation`]). "Immediately" means that the comment block starts its
//! own line, consecutive comments in it are separated by at most one line
//! break, and at most one line break separates the block from the
//! construct: a blank line ends the association. Attributes, annotations,
//! and decorators between the comment and the declaration (`@Test`,
//! `[Fact]`, `@pytest.mark.slow`) belong to the construct.
//!
//! # Locators
//!
//! A Source Target's opaque locator (BD-027) is
//! `<project-relative path>::<qualified name>`, where the qualified name
//! joins the enclosing named scopes (class, namespace, ...) and the function
//! name with `::` — for example `src/calc.py::Calculator::add`. Overloads
//! share a locator; a Target Reference to one is then ambiguous and the core
//! reports `E-SCAN-004`, so declare a permanent `@vtest.src-id` on the one
//! meant and target that ID instead.
//!
//! A Test's `SourceLocation.locator` and `ExecutionDescriptor.selector` are
//! the qualified function name when the Test construct is a function whose
//! name is unique in its file, and `<first word>#<n>` (the n-th Test
//! construct of the file, from 1) otherwise.
//!
//! # Limits
//!
//! - A test that carries no `@vtest.` declaration is not observed at all,
//!   because recognizing it would require knowing the framework; no
//!   `W-SCAN-101` is emitted and no `ManagedTestLink::Missing` is produced
//!   for it. A declared Test whose declaration is malformed *is* observed
//!   and reported as Missing with its diagnostics (DES-223).
//! - The lexer does not understand every literal form of every language
//!   (JavaScript regular expression literals, Ruby `%w()` literals, heredocs
//!   with brackets). A file whose brackets do not balance after lexing is
//!   reported as `E-SCAN-001` and contributes nothing, rather than being
//!   guessed at.

pub mod annotation;
pub mod braces;
pub mod header;
pub mod javascript;
pub mod lexer;

use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
};

use ignore::WalkBuilder;
use vtest_adapter_api::{
    Adapter, AdapterDescriptor, AdapterScanConfig, Capability, DiscoveryError, DiscoveryOutcome,
    MissingTestConstruct, SourceDiscoveryAdapter, SourceDraft, TestDraft,
};
use vtest_model::{
    AdapterId, Diagnostic, ExecutionDescriptor, Locator, ProjectPath, SourceLocation, SourceRange,
    SrcId, TargetRef, TestId, VoId,
};

use crate::{
    annotation::{Declaration, SourceDeclaration, TestDeclaration},
    braces::FunctionSite,
    header::{HTok, HeaderKind},
    lexer::{LexError, LexicalSyntax, Structure, Tok, TokKind},
};

/// One language's discovery rules.
pub trait LanguageProfile {
    /// The adapter ID (`config.yaml` `adapters[].id`, `Locator.adapter`).
    fn adapter_id(&self) -> &'static str;

    /// The languages this adapter declares (DES-350).
    fn languages(&self) -> &'static [&'static str];

    /// File extensions (without the dot) this adapter discovers.
    fn extensions(&self) -> &'static [&'static str];

    /// Directory names never descended into, in addition to `.git` and
    /// `target` (dependency and build output directories such as
    /// `node_modules`).
    fn excluded_dirs(&self) -> &'static [&'static str] {
        &[]
    }

    fn syntax(&self) -> &'static LexicalSyntax;

    /// Classifies one declaration header (see [`header`]). `has_body` tells
    /// whether the statement continues with a `{ ... }` body.
    fn classify(&self, header: &[HTok<'_>], has_body: bool) -> HeaderKind;
}

/// A [`LanguageProfile`] registered as an adapter with the
/// `source_discovery` capability only.
pub struct LexicalAdapter<P> {
    profile: P,
}

impl<P: LanguageProfile> LexicalAdapter<P> {
    pub fn new(profile: P) -> Self {
        Self { profile }
    }

    pub fn profile(&self) -> &P {
        &self.profile
    }
}

impl<P: LanguageProfile> Adapter for LexicalAdapter<P> {
    fn descriptor(&self) -> AdapterDescriptor {
        AdapterDescriptor {
            id: self.profile.adapter_id().to_owned(),
            languages: self
                .profile
                .languages()
                .iter()
                .map(|language| (*language).to_owned())
                .collect(),
            capabilities: vec![Capability::SourceDiscovery],
            config_namespace: self.profile.adapter_id().to_owned(),
        }
    }

    fn as_source_discovery(&self) -> Option<&dyn SourceDiscoveryAdapter> {
        Some(self)
    }
}

impl<P: LanguageProfile> SourceDiscoveryAdapter for LexicalAdapter<P> {
    fn id(&self) -> &'static str {
        self.profile.adapter_id()
    }

    fn discover(
        &self,
        root: &Path,
        fallback_package: &str,
        config: &AdapterScanConfig,
    ) -> Result<DiscoveryOutcome, DiscoveryError> {
        let mut paths = Vec::new();
        for include_path in &config.include_paths {
            let full = root.join(include_path);
            collect_files(root, &full, &self.profile, &mut paths).map_err(|error| {
                DiscoveryError {
                    path: full.clone(),
                    message: error.to_string(),
                }
            })?;
        }
        paths.sort();
        paths.dedup();

        let mut outcome = DiscoveryOutcome {
            files_scanned: paths.len(),
            tests: Vec::new(),
            missing_tests: Vec::new(),
            sources: Vec::new(),
            diagnostics: Vec::new(),
        };
        for path in &paths {
            let relative = path
                .strip_prefix(root)
                .unwrap_or(path)
                .to_string_lossy()
                .replace('\\', "/");
            match fs::read_to_string(path) {
                Ok(source) => FileScan::new(&self.profile, &relative, &source, fallback_package)
                    .run(&mut outcome),
                Err(error) => outcome.diagnostics.push(
                    Diagnostic::error(
                        "E-SCAN-001",
                        format!("failed to read {}: {error}", path.display()),
                    )
                    .with_location(file_location(
                        self.profile.adapter_id(),
                        &relative,
                        0,
                    )),
                ),
            }
        }
        Ok(outcome)
    }
}

/// Discovers the text of one file as if it were at the project-relative
/// path `relative`, without touching the file system. `vtest scan` goes
/// through [`SourceDiscoveryAdapter::discover`]; this entry point lets a
/// language profile's tests check its rules on inline sources.
pub fn discover_text(
    profile: &dyn LanguageProfile,
    relative: &str,
    source: &str,
    package: &str,
) -> DiscoveryOutcome {
    let mut outcome = DiscoveryOutcome {
        files_scanned: 1,
        tests: Vec::new(),
        missing_tests: Vec::new(),
        sources: Vec::new(),
        diagnostics: Vec::new(),
    };
    FileScan::new(profile, relative, source, package).run(&mut outcome);
    outcome
}

fn collect_files(
    project_root: &Path,
    path: &Path,
    profile: &dyn LanguageProfile,
    output: &mut Vec<PathBuf>,
) -> Result<(), ignore::Error> {
    if !path.exists() {
        return Ok(());
    }
    let wanted = |candidate: &Path| {
        candidate
            .extension()
            .and_then(|value| value.to_str())
            .is_some_and(|extension| profile.extensions().contains(&extension))
    };
    if path.is_file() {
        if wanted(path) {
            output.push(path.to_owned());
        }
        return Ok(());
    }
    let include_root = path.to_owned();
    let excluded = profile.excluded_dirs();
    let mut builder = WalkBuilder::new(project_root);
    builder
        .standard_filters(false)
        .hidden(false)
        .parents(false)
        .ignore(false)
        .git_ignore(true)
        .git_global(false)
        .git_exclude(false)
        .require_git(false)
        .follow_links(false)
        .filter_entry(move |entry| {
            let name = entry.file_name().to_str().unwrap_or_default();
            let is_dir = entry.file_type().is_some_and(|kind| kind.is_dir());
            !(is_dir && (name == ".git" || name == "target" || excluded.contains(&name)))
                && (include_root.starts_with(entry.path())
                    || entry.path().starts_with(&include_root))
        });
    for entry in builder.build() {
        let entry = entry?;
        if entry.file_type().is_some_and(|kind| kind.is_file()) && wanted(entry.path()) {
            output.push(entry.into_path());
        }
    }
    Ok(())
}

fn file_location(adapter: &str, relative: &str, length: usize) -> SourceLocation {
    SourceLocation {
        adapter: AdapterId::new(adapter),
        path: ProjectPath::new(relative),
        locator: relative.rsplit('/').next().unwrap_or(relative).to_owned(),
        byte_range: SourceRange {
            start: 0,
            end: length as u64,
        },
    }
}

/// A comment block and the token it is attached to.
struct CommentBlock {
    start_byte: usize,
    attached: usize,
    declaration: Option<Declaration>,
}

struct FileScan<'a> {
    profile: &'a dyn LanguageProfile,
    relative: &'a str,
    source: &'a str,
    package: &'a str,
}

impl<'a> FileScan<'a> {
    fn new(
        profile: &'a dyn LanguageProfile,
        relative: &'a str,
        source: &'a str,
        package: &'a str,
    ) -> Self {
        Self {
            profile,
            relative,
            source,
            package,
        }
    }

    fn adapter(&self) -> AdapterId {
        AdapterId::new(self.profile.adapter_id())
    }

    fn location(&self, locator: &str, start: usize, end: usize) -> SourceLocation {
        SourceLocation {
            adapter: self.adapter(),
            path: ProjectPath::new(self.relative),
            locator: locator.to_owned(),
            byte_range: SourceRange {
                start: start as u64,
                end: end as u64,
            },
        }
    }

    fn lex_failure(&self, outcome: &mut DiscoveryOutcome, error: LexError) {
        outcome.diagnostics.push(
            Diagnostic::error(
                "E-SCAN-001",
                format!(
                    "failed to parse {} at byte {}: {}",
                    self.relative, error.offset, error.message
                ),
            )
            .with_location(file_location(
                self.profile.adapter_id(),
                self.relative,
                self.source.len(),
            )),
        );
    }

    fn run(&self, outcome: &mut DiscoveryOutcome) {
        let syntax = self.profile.syntax();
        let tokens = match lexer::lex(self.source, syntax) {
            Ok(tokens) => tokens,
            Err(error) => return self.lex_failure(outcome, error),
        };
        let (functions, extent): (Vec<FunctionSite>, Box<dyn Fn(usize) -> usize>) = match syntax
            .structure
        {
            Structure::Braces => {
                let partners = match braces::matching(&tokens) {
                    Ok(partners) => partners,
                    Err(error) => return self.lex_failure(outcome, error),
                };
                let classify =
                    |header: &[HTok<'_>], has_body: bool| self.profile.classify(header, has_body);
                let functions =
                    braces::functions(&tokens, self.source, syntax, &partners, &classify);
                let tokens_for_extent = tokens.clone();
                let extent = move |start: usize| {
                    braces::construct_end(
                        &tokens_for_extent,
                        syntax,
                        start,
                        tokens_for_extent.len(),
                    )
                };
                (functions, Box::new(extent))
            }
        };
        let blocks = comment_blocks(&tokens, self.source, syntax);
        let block_at = blocks
            .iter()
            .map(|block| (block.attached, block))
            .collect::<BTreeMap<_, _>>();

        let mut name_counts = BTreeMap::<String, usize>::new();
        for function in &functions {
            *name_counts.entry(function.name.join("::")).or_default() += 1;
        }
        let function_at = functions
            .iter()
            .map(|function| (function.start_tok, function))
            .collect::<BTreeMap<_, _>>();

        for function in &functions {
            let block = block_at.get(&function.start_tok);
            let start = block.map_or(tokens[function.start_tok].start, |block| block.start_byte);
            let end = tokens[function.end_tok - 1].end;
            let item_path = function.name.join("::");
            let location = self.location(&item_path, start, end);
            let src_id = match block.and_then(|block| block.declaration.as_ref()) {
                Some(Declaration::Source(declaration)) => {
                    self.source_diagnostics(outcome, declaration, &location);
                    declaration.src_id.clone().map(SrcId::new)
                }
                _ => None,
            };
            outcome.sources.push(SourceDraft {
                locator: Locator {
                    adapter: self.adapter(),
                    value: format!("{}::{item_path}", self.relative),
                },
                src_id,
                location,
                construct_text: self.source[start..end].to_owned(),
            });
        }

        let mut ordinal = 0usize;
        for block in &blocks {
            let declaration = match &block.declaration {
                Some(Declaration::Test(declaration)) => declaration,
                Some(Declaration::Source(declaration)) => {
                    if !function_at.contains_key(&block.attached) {
                        let start = tokens[block.attached].start;
                        let location = self.location("", block.start_byte, start);
                        self.source_diagnostics(outcome, declaration, &location);
                    }
                    continue;
                }
                None => continue,
            };
            ordinal += 1;
            let function = function_at.get(&block.attached);
            let end_tok = function.map_or_else(|| extent(block.attached), |f| f.end_tok);
            if end_tok <= block.attached {
                continue;
            }
            let start = tokens[block.attached].start;
            let end = tokens[end_tok - 1].end;
            let locator = match function {
                Some(function) if name_counts[&function.name.join("::")] == 1 => {
                    function.name.join("::")
                }
                _ => format!(
                    "{}#{ordinal}",
                    first_word(&tokens[block.attached..end_tok], self.source)
                ),
            };
            let location = self.location(&locator, start, end);
            self.test(outcome, declaration, location, &self.source[start..end]);
        }
    }

    fn source_diagnostics(
        &self,
        outcome: &mut DiscoveryOutcome,
        declaration: &SourceDeclaration,
        location: &SourceLocation,
    ) {
        for (code, message) in &declaration.diagnostics {
            let diagnostic = if code.starts_with('E') {
                Diagnostic::error(code, message)
            } else {
                Diagnostic::warning(code, message)
            };
            outcome
                .diagnostics
                .push(diagnostic.with_location(location.clone()));
        }
    }

    /// Builds the Test draft, or reports the construct as Missing with the
    /// diagnostics that prevent it (DES-223, DS-1666).
    fn test(
        &self,
        outcome: &mut DiscoveryOutcome,
        declaration: &TestDeclaration,
        location: SourceLocation,
        construct: &str,
    ) {
        let missing = |outcome: &mut DiscoveryOutcome, diagnostics: Vec<Diagnostic>| {
            for diagnostic in diagnostics {
                outcome
                    .diagnostics
                    .push(diagnostic.with_location(location.clone()));
            }
            outcome.missing_tests.push(MissingTestConstruct {
                location: location.clone(),
                construct_text: construct.to_owned(),
            });
        };
        let name = &location.locator;
        if !declaration.diagnostics.is_empty() {
            let diagnostics = declaration
                .diagnostics
                .iter()
                .map(|(code, message)| Diagnostic::error(code, message))
                .collect();
            return missing(outcome, diagnostics);
        }
        let Some(id) = declaration.id.as_deref().filter(|value| !value.is_empty()) else {
            return missing(
                outcome,
                vec![Diagnostic::error(
                    "E-SCAN-007",
                    format!("test `{name}` is missing required @vtest.id"),
                )],
            );
        };
        let covers = declaration
            .covers
            .as_deref()
            .unwrap_or_default()
            .split(',')
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(VoId::new)
            .collect::<Vec<_>>();
        if covers.is_empty() {
            return missing(
                outcome,
                vec![Diagnostic::error(
                    "E-SCAN-007",
                    format!("test `{id}` has no VO in @vtest.covers"),
                )],
            );
        }
        let Some(intent) = declaration
            .intent
            .as_deref()
            .filter(|value| !value.is_empty())
        else {
            return missing(
                outcome,
                vec![Diagnostic::error(
                    "E-SCAN-007",
                    format!("test `{id}` is missing required @vtest.intent"),
                )],
            );
        };
        let targets = declaration
            .targets
            .iter()
            .map(|value| match value.strip_prefix("SRC-") {
                Some(_) => TargetRef::SrcId(SrcId::new(value.clone())),
                None => TargetRef::Locator(Locator {
                    adapter: self.adapter(),
                    value: value.clone(),
                }),
            })
            .collect();
        let related = declaration
            .related
            .iter()
            .flat_map(|value| value.split(','))
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(TestId::new)
            .collect();
        outcome.tests.push(TestDraft {
            id: TestId::new(id),
            covers,
            targets,
            intent: intent.to_owned(),
            input: declaration.input.clone(),
            expect: declaration.expect.clone(),
            kind: declaration.kind.clone(),
            cases: declaration.cases.clone(),
            related,
            execution: ExecutionDescriptor {
                adapter: self.adapter(),
                project: Some(self.package.to_owned()),
                suite: None,
                selector: location.locator.clone(),
            },
            location,
            construct_text: construct.to_owned(),
        });
    }
}

fn first_word(tokens: &[Tok], source: &str) -> String {
    tokens
        .iter()
        .find(|token| token.kind == TokKind::Word)
        .map_or_else(
            || "construct".to_owned(),
            |token| token.text(source).to_owned(),
        )
}

/// Groups comments into blocks and attaches each block to the construct
/// that immediately follows it (see the crate documentation).
fn comment_blocks(tokens: &[Tok], source: &str, syntax: &LexicalSyntax) -> Vec<CommentBlock> {
    let closers = syntax
        .block_comments
        .iter()
        .map(|(_, close)| *close)
        .collect::<Vec<_>>();
    let mut blocks = Vec::new();
    let mut index = 0usize;
    while index < tokens.len() {
        let token = tokens[index];
        if token.kind != TokKind::Comment || !starts_line(source, token.start) {
            index += 1;
            continue;
        }
        let first = index;
        let mut last = index;
        loop {
            let (newlines, next) = skip_newlines(tokens, last + 1);
            if newlines <= 1 && next < tokens.len() && tokens[next].kind == TokKind::Comment {
                last = next;
            } else {
                break;
            }
        }
        index = last + 1;
        let (newlines, next) = skip_newlines(tokens, last + 1);
        if newlines > 1 || next >= tokens.len() {
            continue;
        }
        if matches!(
            tokens[next].kind,
            TokKind::Comment | TokKind::Directive | TokKind::Close(_) | TokKind::Semi
        ) {
            continue;
        }
        let lines = tokens[first..=last]
            .iter()
            .filter(|token| token.kind == TokKind::Comment)
            .flat_map(|token| annotation::annotation_lines(token.text(source), &closers))
            .collect::<Vec<_>>();
        blocks.push(CommentBlock {
            start_byte: tokens[first].start,
            attached: next,
            declaration: annotation::classify(lines),
        });
    }
    blocks
}

fn skip_newlines(tokens: &[Tok], mut index: usize) -> (usize, usize) {
    let mut newlines = 0usize;
    while index < tokens.len() && tokens[index].kind == TokKind::Newline {
        newlines += 1;
        index += 1;
    }
    (newlines, index)
}

fn starts_line(source: &str, start: usize) -> bool {
    source[..start]
        .rsplit('\n')
        .next()
        .is_none_or(|prefix| prefix.trim().is_empty())
}

#[cfg(test)]
mod tests;
