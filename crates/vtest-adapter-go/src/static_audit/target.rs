//! A declared target and the file it is called from, as the static audit reads
//! them (DS-622, DS-1970, DS-1979).
//!
//! Responsibility: read a Go target locator (`<path>::<Name>` or
//! `<path>::<Receiver>::<Name>`, BD-350) and the package and import header of
//! the Test's file, and say whether a call is a call of the target. Boundary:
//! the input of the audit holds no `go.mod`, so a call that cannot be tied to
//! the target's file from these (a qualified call through an import whose
//! package is not the target's directory, a same-named method on another
//! type) is `Ambiguous`, never a target call, and the audit then reports
//! `UNKNOWN` (DS-615, DS-1680).

use crate::syntax::{parse_file_header, FileHeader};

/// A declared target of a Test.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct TargetSpec {
    /// The final segment of the locator.
    pub(crate) symbol: String,
    /// The locator names a method (`<path>::<Receiver>::<Name>`).
    pub(crate) is_method: bool,
    /// The project-relative directory of the target's file; empty at the
    /// project root.
    pub(crate) dir: String,
}

impl TargetSpec {
    /// The target a `go` locator value names, or `None` for a value that is
    /// not one.
    pub(crate) fn from_locator(value: &str) -> Option<Self> {
        let parts: Vec<&str> = value.split("::").collect();
        let (path, symbol) = (parts.first()?, parts.last()?);
        if parts.len() < 2 || symbol.is_empty() {
            return None;
        }
        Some(Self {
            symbol: (*symbol).to_owned(),
            is_method: parts.len() >= 3,
            dir: parent_dir(path),
        })
    }
}

fn parent_dir(path: &str) -> String {
    path.rsplit_once('/')
        .map(|(dir, _)| dir.to_owned())
        .unwrap_or_default()
}

/// Whether a call is of a declared target.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum TargetMatch {
    No,
    Definite,
    /// Possibly the target: the name matches and the file does not settle it.
    Ambiguous,
}

/// What an import name stands for in the Test's file.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum ImportLookup<'a> {
    Path(&'a str),
    NotImported,
    /// The header was not read, or a dot import hides the names.
    Unknown,
}

struct ImportBinding {
    name: String,
    path: String,
}

/// The package clause and imports of the Test's file, and its directory.
#[derive(Debug)]
pub(crate) struct FileInfo {
    test_dir: String,
    package: Option<String>,
    imports: Option<Vec<ImportBinding>>,
}

impl std::fmt::Debug for ImportBinding {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} {:?}", self.name, self.path)
    }
}

/// The name an import binds: its explicit name, or the last element of its
/// path (the one before it for a major version suffix such as `v2`).
fn default_name(path: &str) -> String {
    let mut elements = path.rsplit('/');
    let last = elements.next().unwrap_or(path);
    let is_version =
        last.len() > 1 && last.starts_with('v') && last[1..].chars().all(|c| c.is_ascii_digit());
    if is_version {
        if let Some(previous) = elements.next() {
            return previous.to_owned();
        }
    }
    last.to_owned()
}

impl FileInfo {
    /// `test_path` is the project-relative path of the Test's file; `file_text`
    /// is the whole file, when it was readable as text.
    pub(crate) fn new(test_path: &str, file_text: Option<&str>) -> Self {
        let header: Option<FileHeader> = file_text.and_then(|text| parse_file_header(text).ok());
        let (package, imports) = match header {
            Some(header) => {
                let dot_import = header
                    .imports
                    .iter()
                    .any(|import| import.name.as_deref() == Some("."));
                let imports = if dot_import {
                    None
                } else {
                    Some(
                        header
                            .imports
                            .into_iter()
                            .filter(|import| import.name.as_deref() != Some("_"))
                            .map(|import| ImportBinding {
                                name: import
                                    .name
                                    .clone()
                                    .unwrap_or_else(|| default_name(&import.path)),
                                path: import.path,
                            })
                            .collect(),
                    )
                };
                (Some(header.package), imports)
            }
            None => (None, None),
        };
        Self {
            test_dir: parent_dir(test_path),
            package,
            imports,
        }
    }

    /// Whether the file is of an external test package (`package x_test`);
    /// `None` when the package clause was not read.
    pub(crate) fn is_external_test(&self) -> Option<bool> {
        self.package.as_ref().map(|name| name.ends_with("_test"))
    }

    pub(crate) fn import(&self, name: &str) -> ImportLookup<'_> {
        match &self.imports {
            None => ImportLookup::Unknown,
            Some(imports) => imports
                .iter()
                .find(|import| import.name == name)
                .map_or(ImportLookup::NotImported, |import| {
                    ImportLookup::Path(&import.path)
                }),
        }
    }

    /// A bare call `Name(..)` of a function target: the same package as the
    /// Test's file.
    pub(crate) fn bare_call_matches(&self, target: &TargetSpec) -> TargetMatch {
        if target.dir != self.test_dir {
            return TargetMatch::No;
        }
        match self.is_external_test() {
            Some(false) => TargetMatch::Definite,
            Some(true) => TargetMatch::No,
            None => TargetMatch::Ambiguous,
        }
    }

    /// A qualified call `q.Name(..)` of a function target through the import
    /// name `q`.
    pub(crate) fn qualified_call_matches(
        &self,
        qualifier: &str,
        target: &TargetSpec,
    ) -> TargetMatch {
        match self.import(qualifier) {
            ImportLookup::Path(path) => {
                if target.dir.is_empty() {
                    // The root package: the import path alone does not say.
                    TargetMatch::Ambiguous
                } else if path == target.dir || path.ends_with(&format!("/{}", target.dir)) {
                    TargetMatch::Definite
                } else {
                    TargetMatch::No
                }
            }
            ImportLookup::NotImported | ImportLookup::Unknown => TargetMatch::Ambiguous,
        }
    }
}
