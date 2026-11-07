//! Framework-neutral source discovery for languages other than Rust.
//!
//! `vtest-adapter-rust` discovers Tests from Rust's `#[test]` attribute and
//! reads their declarations from doc comments. Other ecosystems have many
//! test frameworks, so these adapters do not recognize a Test by a
//! framework's syntax. Instead, **a Test construct is the construct that
//! immediately follows a comment block declaring at least one `@vtest.`
//! test-key**.
//!
//! # Responsibilities
//!
//! Each language is a [`LanguageProfile`] in its own module. It reads its
//! own syntax and returns, for one file, [`FileItems`]: every function
//! definition (a Source Target) with its name and byte range, and every
//! comment block that immediately precedes a construct, with the comment's
//! lines and the construct's byte range. How a language finds those (its
//! comments and strings, which `{` is a body, where a statement ends) is
//! decided only in its module.
//!
//! This crate root depends only on that interface. [`LexicalAdapter`] walks
//! the include paths, calls the profile for each file, and turns what it
//! returns into Drafts: it parses the `@vtest.` lines (see [`annotation`]),
//! reports their diagnostics, and builds Locators. The composition root
//! (`vtest-cli`'s `adapters.rs`) injects each profile as
//! `LexicalAdapter::new(Profile)`.
//!
//! What means the same thing in every language (pairing brackets, and
//! the rule that attaches a comment block to the construct after it) is
//! in [`shared`], for the language modules to call.
//!
//! The adapters declare only the `source_discovery` capability (DES-350).
//! Verification therefore reports `target_binding` as `NO_EVIDENCE`
//! (`NOT_EXECUTED`, `NOT_CHECKED`) and `oracle_presence` as `NO_EVIDENCE`
//! (`NOT_CHECKED`) for these Tests, and `vtest run` rejects them with
//! `E-ADAPTER-004`.
//!
//! # Locators
//!
//! A Source Target's opaque locator (BD-027) is
//! `<project-relative path>::<qualified name>`, where the qualified name
//! joins the enclosing named scopes and the function name with `::`. A
//! Test's `SourceLocation.locator` and `ExecutionDescriptor.selector` are
//! the qualified function name when the Test construct is a function whose
//! name is unique in its file, and `<first word>#<n>` (the n-th Test
//! construct of the file, from 1) otherwise.
//!
//! # Failing closed
//!
//! A file the profile cannot read is `E-SCAN-001` and contributes nothing.
//! A declared Test whose construct extent the profile cannot determine is
//! reported as Missing with `E-SCAN-001`, never as a fragment.

pub mod annotation;
pub mod c;
pub mod shared;

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

use crate::annotation::{Declaration, SourceDeclaration, TestDeclaration};

/// One language's discovery: how to read one of its files.
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

    /// Reads one file. An error means the file as a whole could not be
    /// read with confidence; it is reported as `E-SCAN-001`.
    fn extract(&self, source: &str) -> Result<FileItems, ExtractError>;
}

/// Why a file could not be read.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExtractError {
    /// The byte offset the problem was found at.
    pub offset: usize,
    pub message: String,
}

/// What a profile found in one file.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct FileItems {
    /// Every function definition, in source order.
    pub functions: Vec<Function>,
    /// Every comment block that immediately precedes a construct, in source
    /// order.
    pub annotated: Vec<Annotated>,
}

/// A function definition (a Source Target).
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Function {
    /// The qualified name, enclosing scopes first.
    pub name: Vec<String>,
    /// The byte range of the definition, from its first token (attributes
    /// and annotations included) to its last.
    pub start: usize,
    pub end: usize,
}

/// A comment block and the construct right after it.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Annotated {
    /// The byte offset where the comment block starts.
    pub comment_start: usize,
    /// The comment block's lines with the language's comment markers
    /// removed.
    pub lines: Vec<String>,
    /// The byte range of the construct, or why it could not be determined.
    pub construct: Result<(usize, usize), String>,
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

    fn run(&self, outcome: &mut DiscoveryOutcome) {
        let items = match self.profile.extract(self.source) {
            Ok(items) => items,
            Err(error) => {
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
                return;
            }
        };
        let declarations = items
            .annotated
            .iter()
            .map(|annotated| {
                let lines = annotated
                    .lines
                    .iter()
                    .filter_map(|line| annotation::parse_line(line))
                    .collect::<Vec<_>>();
                (annotated, annotation::classify(lines))
            })
            .collect::<Vec<_>>();
        let attached_at = declarations
            .iter()
            .filter_map(|(annotated, declaration)| {
                let (start, _) = annotated.construct.as_ref().ok()?;
                Some((*start, (*annotated, declaration.as_ref()?)))
            })
            .collect::<BTreeMap<_, _>>();

        let mut name_counts = BTreeMap::<String, usize>::new();
        for function in &items.functions {
            *name_counts.entry(function.name.join("::")).or_default() += 1;
        }
        let function_at = items
            .functions
            .iter()
            .map(|function| (function.start, function))
            .collect::<BTreeMap<_, _>>();

        for function in &items.functions {
            let attached = attached_at.get(&function.start);
            let start = attached.map_or(function.start, |(annotated, _)| annotated.comment_start);
            let item_path = function.name.join("::");
            let location = self.location(&item_path, start, function.end);
            let src_id = match attached {
                Some((_, Declaration::Source(declaration))) => {
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
                construct_text: self.source[start..function.end].to_owned(),
            });
        }

        let mut ordinal = 0usize;
        for (annotated, declaration) in &declarations {
            let declaration = match declaration {
                Some(Declaration::Test(declaration)) => declaration,
                Some(Declaration::Source(declaration)) => {
                    let attached_to_function = annotated
                        .construct
                        .as_ref()
                        .is_ok_and(|(start, _)| function_at.contains_key(start));
                    if !attached_to_function {
                        let end = annotated
                            .construct
                            .as_ref()
                            .map_or(annotated.comment_start, |(start, _)| *start);
                        let location = self.location("", annotated.comment_start, end);
                        self.source_diagnostics(outcome, declaration, &location);
                    }
                    continue;
                }
                None => continue,
            };
            ordinal += 1;
            let (start, end) = match &annotated.construct {
                Ok(range) => *range,
                Err(reason) => {
                    let start = annotated.comment_start;
                    let end = line_end(self.source, start);
                    let location = self.location(&format!("construct#{ordinal}"), start, end);
                    let message = format!(
                        "cannot determine the construct declared at byte {start} of {}: {reason}",
                        self.relative
                    );
                    outcome.diagnostics.push(
                        Diagnostic::error("E-SCAN-001", message).with_location(location.clone()),
                    );
                    outcome.missing_tests.push(MissingTestConstruct {
                        location,
                        construct_text: self.source[start..end].to_owned(),
                    });
                    continue;
                }
            };
            let function = function_at.get(&start);
            let locator = match function {
                Some(function) if name_counts[&function.name.join("::")] == 1 => {
                    function.name.join("::")
                }
                _ => format!("{}#{ordinal}", first_word(&self.source[start..end])),
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

/// The first identifier-like word of a construct, used to name a Test that
/// is not a uniquely named function (`test#2`, `TEST#1`).
fn first_word(text: &str) -> String {
    text.split(|c: char| !(c.is_alphanumeric() || c == '_' || c == '$'))
        .find(|word| !word.is_empty())
        .map_or_else(|| "construct".to_owned(), str::to_owned)
}

fn line_end(source: &str, start: usize) -> usize {
    source[start..]
        .find('\n')
        .map_or(source.len(), |offset| start + offset)
}

#[cfg(test)]
mod tests;
