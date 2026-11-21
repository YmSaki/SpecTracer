//! `go test` as the `go` adapter's Test Runner (DS-1904 to DS-1911, DS-1930,
//! DS-2011, DS-2012, DES-647 to DES-651, DES-677, DES-678).
//!
//! Responsibility: turn a Test's execution coordinates into a `go test`
//! command, refuse the Tests that command cannot run for sure, and read the
//! command's `-json` events as the Test's result.
//!
//! Boundary: starting the process and recording Evidence are `vtest-exec`'s;
//! the Execution State is [`crate::execution_state`]'s; reading Go source is
//! `vtest-adapter-lexical`'s. Coverage is not provided: the adapter declares no
//! Coverage capability, so the runner is never asked for it.

use std::{collections::BTreeMap, fs, path::Path};

use serde_json::Value;
use vtest_adapter_api::{
    ExecutionStateReader, RunnerCommand, RunnerOutput, RunnerTestResult, TestRunnerAdapter,
    TestRunnerError,
};
use vtest_adapter_lexical::go::test_constructs;
use vtest_model::{ExecutionDescriptor, ExecutionStateDraft};

use crate::{
    execution_state::{self, GoExecutionStateReader},
    PACKAGE_SUITE,
};

/// The runner kind Evidence records for a Test run by `go test` (DES-647).
pub const RUNNER_KIND: &str = "go-test";

/// The command that runs the tests (BD-360).
pub(crate) const PROGRAM: &str = "go";

/// The file name ending of a Go test file (DS-1904, DS-2011).
const TEST_FILE_SUFFIX: &str = "_test.go";

/// The `go` Test Runner.
///
/// `environment` is laid over the process environment for every `go` process
/// the runner starts or describes: the command's own `env` (so `vtest-exec`
/// launches it with these values) and the Execution State (DES-369), so what is
/// bound is what runs.
#[derive(Clone, Debug, Default)]
pub struct GoTestRunner {
    pub(crate) environment: BTreeMap<String, String>,
}

impl GoTestRunner {
    /// A runner that adds nothing to the process environment.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// A runner whose `go` processes see `environment` laid over the process
    /// environment.
    #[must_use]
    pub fn with_environment(environment: BTreeMap<String, String>) -> Self {
        Self { environment }
    }
}

/// A Test's execution coordinates read as the `go` adapter reads them
/// (DES-643, DES-644, DES-645).
#[derive(Clone, Copy, Debug)]
pub(crate) struct Coordinates<'a> {
    /// The module directory relative to the project root.
    pub module: &'a str,
    /// The package directory relative to the module root.
    pub suite: &'a str,
    /// The Test function's name.
    pub selector: &'a str,
}

/// Reads `execution` as coordinates. A Test without a project is one in no Go
/// module, which the runner cannot execute (DES-687, DS-1901); a descriptor of
/// any other shape is not one this adapter wrote (E-ADAPTER-003).
pub(crate) fn coordinates(
    execution: &ExecutionDescriptor,
) -> Result<Coordinates<'_>, TestRunnerError> {
    let Some(module) = execution.project.as_deref() else {
        return Err(TestRunnerError::NotExecutable {
            reason: "no go.mod above its file inside the project root, so there is no Go module \
                     to run it in (DES-687)"
                .to_owned(),
        });
    };
    let suite = execution
        .suite
        .as_ref()
        .ok_or(TestRunnerError::MissingSuite)?;
    if suite.kind != PACKAGE_SUITE {
        return Err(TestRunnerError::UnsupportedSuiteKind {
            kind: suite.kind.clone(),
        });
    }
    let name = suite
        .name
        .as_deref()
        .ok_or_else(|| TestRunnerError::MissingSuiteName {
            kind: suite.kind.clone(),
        })?;
    // `go` reads `...` in a package pattern as a wildcard, so `./a...b` also
    // matches `axb`, and the result of the Test of another package could be
    // taken for this one's (DS-1905: the Test's own package is the target). A
    // directory with `...` in its name cannot be named by a literal pattern, so
    // its Tests are not run.
    if name.contains("...") {
        return Err(TestRunnerError::NotExecutable {
            reason: format!(
                "the package directory {name} has `...` in its name, which go reads as a \
                 wildcard in a package pattern, so the one package cannot be selected by its \
                 path (DS-1905)"
            ),
        });
    }
    Ok(Coordinates {
        module,
        suite: name,
        selector: &execution.selector,
    })
}

/// DES-648: the arguments of `go test` for one Test. `-count=1` is the
/// instruction not to use a cached result (DS-1906), and `^<selector>$` selects
/// the one function by its exact name (DS-1905).
pub(crate) fn invocation_args(coordinates: &Coordinates<'_>) -> Vec<String> {
    vec![
        "test".to_owned(),
        format!("./{}", coordinates.suite),
        "-run".to_owned(),
        format!("^{}$", coordinates.selector),
        "-count=1".to_owned(),
        "-json".to_owned(),
    ]
}

/// DS-1930, DS-2011, DS-2012, DS-1904: whether the Test can be selected by its
/// name and run, read from the package directory as `go test` will read it.
///
/// Every `_test.go` file of the directory is read, whatever build constraints,
/// the adapter's include list, or `.gitignore` say about it, because `go test`
/// picks the file to build from the constraints and a same-named function in a
/// file this run does not discover could be the one whose result comes back.
/// Names that start with `_` or `.` are not read: the go command never takes
/// them into a package.
fn check_selection(root: &Path, coordinates: &Coordinates<'_>) -> Result<(), TestRunnerError> {
    let directory = root.join(coordinates.module).join(coordinates.suite);
    let shown = format!("{}/{}", coordinates.module, coordinates.suite);
    let mut unreadable = Vec::new();
    let mut named = Vec::new();

    let entries =
        fs::read_dir(&directory).map_err(|error| TestRunnerError::SelectionUndetermined {
            reason: format!("the package directory {shown} cannot be listed: {error} (DS-2012)"),
        })?;
    let mut files = Vec::new();
    for entry in entries {
        match entry {
            Ok(entry) => {
                let name = entry.file_name().to_string_lossy().into_owned();
                if name.ends_with(TEST_FILE_SUFFIX)
                    && !name.starts_with('_')
                    && !name.starts_with('.')
                    && !directory.join(&name).is_dir()
                {
                    files.push(name);
                }
            }
            Err(error) => unreadable.push(format!("an entry of {shown} ({error})")),
        }
    }
    files.sort();
    for name in files {
        let read = fs::read_to_string(directory.join(&name))
            .map_err(|error| error.to_string())
            .and_then(|text| test_constructs(&text).map_err(|error| error.message));
        match read {
            Ok(constructs) => named.extend(
                constructs
                    .into_iter()
                    .filter(|construct| construct.name == coordinates.selector)
                    .map(|construct| (name.clone(), construct)),
            ),
            Err(error) => unreadable.push(format!("{name} ({error})")),
        }
    }

    if named.len() >= 2 {
        let files = named
            .iter()
            .map(|(file, _)| file.as_str())
            .collect::<Vec<_>>()
            .join(", ");
        return Err(TestRunnerError::NotUniquelySelectable {
            reason: format!(
                "{} Test constructs are named {} in {shown} ({files}); a name selects them all \
                 (DS-1930)",
                named.len(),
                coordinates.selector
            ),
        });
    }
    if !unreadable.is_empty() {
        return Err(TestRunnerError::SelectionUndetermined {
            reason: format!(
                "whether {} is the only Test of that name in {shown} is unknown because {} cannot \
                 be read (DS-2012)",
                coordinates.selector,
                unreadable.join(", ")
            ),
        });
    }
    if let [(file, construct)] = named.as_slice() {
        if !construct.runnable_shape {
            return Err(TestRunnerError::NotExecutable {
                reason: format!(
                    "{} in {file} does not have the shape `go test` runs: no type parameters, one \
                     *testing.T parameter and no result (DS-1904)",
                    construct.name
                ),
            });
        }
    }
    Ok(())
}

/// Whether the `Package` of an event is the package `suite` names (DS-1905). The
/// import path of that package is the module path and `/` and the suite (the
/// module path alone for `.`). The module path is not known here, so what is
/// checked is that the package ends in `/` and the whole suite, which a package
/// whose directory merely resembles it (`axb` for `a...b`) does not. The pattern
/// handed to `go` is literal (see [`coordinates`]), so `go` itself selects one
/// package; this keeps the result of another package out when it does not.
fn is_selected_package(package: &str, suite: &str) -> bool {
    suite == "."
        || package
            .strip_suffix(suite)
            .is_some_and(|rest| rest.ends_with('/'))
}

/// DES-649 to DES-651, DES-677: the result of the Test named `selector` of the
/// package `suite` in the stdout of `go test -json`.
///
/// A build failure outranks every other event (DES-677). Otherwise the result
/// is the one event of the selected package whose `Test` is exactly the
/// selector and whose `Action` is `pass`, `fail` or `skip` (events of subtests
/// carry another name, events of another package another `Package`). No such
/// event is no result and more than one is an ambiguous result (DES-651). A line
/// that is not a JSON object leaves the run unclassified, and orchestration
/// records no Evidence for it.
pub(crate) fn interpret(stdout: &str, selector: &str, suite: &str) -> RunnerTestResult {
    let mut build_failed = false;
    let mut results = Vec::new();
    for line in stdout
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
    {
        let Ok(Value::Object(event)) = serde_json::from_str::<Value>(line) else {
            return RunnerTestResult::Unknown;
        };
        let action = event.get("Action").and_then(Value::as_str);
        let test = event.get("Test").and_then(Value::as_str);
        let failed_build = event
            .get("FailedBuild")
            .and_then(Value::as_str)
            .is_some_and(|package| !package.is_empty());
        if action == Some("build-fail")
            || (action == Some("fail") && test.is_none_or(str::is_empty) && failed_build)
        {
            build_failed = true;
        }
        let in_selected_package = event
            .get("Package")
            .and_then(Value::as_str)
            .is_some_and(|package| is_selected_package(package, suite));
        if test == Some(selector) && in_selected_package {
            if let Some(action @ ("pass" | "fail" | "skip")) = action {
                results.push(action.to_owned());
            }
        }
    }
    if build_failed {
        return RunnerTestResult::BuildFailed;
    }
    match results.as_slice() {
        [] => RunnerTestResult::NoResult,
        [only] => match only.as_str() {
            "pass" => RunnerTestResult::Pass,
            "fail" => RunnerTestResult::Fail,
            _ => RunnerTestResult::Ignored,
        },
        _ => RunnerTestResult::Ambiguous,
    }
}

impl TestRunnerAdapter for GoTestRunner {
    fn execution_state(
        &self,
        root: &Path,
        execution: &ExecutionDescriptor,
        head_revision: Option<&str>,
        runner_kind: &str,
    ) -> Option<ExecutionStateDraft> {
        execution_state::snapshot(self, root, execution, head_revision, runner_kind)
    }

    fn execution_state_reader(&self) -> Box<dyn ExecutionStateReader + '_> {
        Box::new(GoExecutionStateReader::new(self))
    }

    fn id(&self) -> &'static str {
        crate::ADAPTER_ID
    }

    /// `coverage` is ignored: the adapter declares no Coverage capability, so
    /// orchestration never asks for it.
    fn command(
        &self,
        root: &Path,
        execution: &ExecutionDescriptor,
        _coverage: bool,
        _coverage_output_path: Option<&Path>,
    ) -> Result<RunnerCommand, TestRunnerError> {
        let coordinates = coordinates(execution)?;
        check_selection(root, &coordinates)?;
        let args = invocation_args(&coordinates);
        Ok(RunnerCommand {
            command_line: std::iter::once(PROGRAM.to_owned())
                .chain(args.iter().cloned())
                .collect::<Vec<_>>()
                .join(" "),
            program: PROGRAM.to_owned(),
            args,
            current_dir: root.join(coordinates.module),
            env: self.environment.clone(),
            runner_kind: RUNNER_KIND.to_owned(),
        })
    }

    fn parse(&self, execution: &ExecutionDescriptor, output: RunnerOutput<'_>) -> RunnerTestResult {
        match coordinates(execution) {
            Ok(coordinates) => interpret(output.stdout, coordinates.selector, coordinates.suite),
            // A descriptor `command` would not have turned into this run.
            Err(_) => RunnerTestResult::Unknown,
        }
    }
}
