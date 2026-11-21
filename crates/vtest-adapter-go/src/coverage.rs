//! `go-cover`: the `go` CoverageAdapter (BD-354, DS-1879, DS-1926, DES-663 to
//! DES-667, DES-673, DES-675, DES-676, DES-679).
//!
//! Responsibility: read the coverage profiles of one measured Test run (the
//! Test's own run and the run that selects no Test) and say, for each declared
//! target, whether the Test reached it: `PASS` with a count above 0, `FAIL`
//! with a count of 0, `UNKNOWN` when that cannot be told for certain (DS-1406,
//! DS-1414). A target is `PASS` only when its source range holds blocks that ran
//! in the Test's run, the same blocks are in the run that selects no Test, and
//! none of them ran there: whatever runs before any Test (`init`, a package
//! variable, `TestMain`) cannot be told from a Test's reach (DES-676).
//!
//! Every uncertainty reads as `UNKNOWN` and never as a count of 0 or as a
//! missing block with no count: an unusable run that selects no Test (DES-679),
//! a profile or a profile line that cannot be read, a module that cannot be
//! named, a target outside the measured module, a position that does not fall in
//! the file, and a package whose files hold a line directive (which renames the
//! blocks of a file, so a block of one file could stand for another's).
//!
//! Boundary: starting `go` for the measured runs and recording `target_coverage`
//! are `vtest-exec`'s, building the commands is [`crate::runner`]'s, and reading
//! Go source is `vtest-adapter-lexical`'s: the range of a target is the one its
//! discovery gave it (`source_constructs`). This module only reads the profiles
//! it is handed, the source of the targets, and the `go.mod` that names the
//! module.

use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::{Component, Path},
    process::Command,
};

use serde_json::Value;
use vtest_adapter_api::{
    CoverageAdapter, CoverageRun, CoverageTargetMeasurement, CoverageUnavailable,
};
use vtest_adapter_lexical::go::source_constructs;
use vtest_model::{ExecutionDescriptor, Locator, TargetCoverageResult};

use crate::{execution_state::comparable, module, runner::PROGRAM, ADAPTER_ID};

/// DES-667: the `method` of the `target_coverage` this adapter records.
pub const COVERAGE_METHOD: &str = "go-cover";

/// DES-663: the first line of a profile written under `-covermode=atomic`. A
/// profile of another mode is not one this adapter asked for.
const PROFILE_MODE_LINE: &str = "mode: atomic";

/// What Go calls a line directive, which renames the file and the line of what
/// follows: `//line ` at the start of a line, or `/*line ` anywhere.
const LINE_DIRECTIVE_AT_LINE_START: &str = "//line ";
const LINE_DIRECTIVE_COMMENT: &str = "/*line ";

/// The `go-cover` CoverageAdapter.
///
/// `environment` is the one the runner starts `go` under: the module path is read
/// by the same `go` the measured runs used.
#[derive(Clone, Debug, Default)]
pub struct GoCoverAdapter {
    environment: BTreeMap<String, String>,
}

impl GoCoverAdapter {
    /// An adapter that adds nothing to the process environment.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// An adapter whose `go` processes see `environment` laid over the process
    /// environment.
    #[must_use]
    pub fn with_environment(environment: BTreeMap<String, String>) -> Self {
        Self { environment }
    }
}

impl CoverageAdapter for GoCoverAdapter {
    fn id(&self) -> &'static str {
        ADAPTER_ID
    }

    fn method(&self) -> &'static str {
        COVERAGE_METHOD
    }

    /// Whether `go` runs here. `go test` measures coverage by itself (BD-354), so
    /// there is no other tool to ask for.
    fn availability(&self, root: &Path) -> Result<(), CoverageUnavailable> {
        probe_availability(PROGRAM, root, &self.environment)
    }

    /// DES-675, DES-679: a profile alone cannot say what a Test reached, because
    /// what ran before any Test is in it too. Without the run that selects no
    /// Test, and without the Test's module, every target is `UNKNOWN`.
    fn measure(
        &self,
        _root: &Path,
        _coverage_output_path: &Path,
        targets: &[Locator],
    ) -> Vec<CoverageTargetMeasurement> {
        all_unknown(targets)
    }

    /// DES-673: a profile is a file of its own, named by the record id alone.
    fn output_extension(&self) -> Option<&'static str> {
        None
    }

    fn measure_run(
        &self,
        run: &CoverageRun<'_>,
        targets: &[Locator],
    ) -> Vec<CoverageTargetMeasurement> {
        let Some(profiles) = self.read_run(run) else {
            return all_unknown(targets);
        };
        let mut directories = BTreeMap::new();
        targets
            .iter()
            .map(|target| {
                let (result, count) = measure_target(run.root, &profiles, target, &mut directories);
                CoverageTargetMeasurement {
                    target: target.clone(),
                    result,
                    count,
                }
            })
            .collect()
    }
}

/// `availability` with the program a parameter, so a test can name one that cannot
/// be started (production always passes `go`).
fn probe_availability(
    program: &str,
    root: &Path,
    environment: &BTreeMap<String, String>,
) -> Result<(), CoverageUnavailable> {
    Command::new(program)
        .current_dir(root)
        .envs(environment)
        .arg("version")
        .output()
        .ok()
        .filter(|output| output.status.success())
        .map(|_| ())
        .ok_or_else(|| {
            CoverageUnavailable::new("`go version` cannot be run; target_coverage is NOT_CHECKED")
        })
}

/// What a measured run needs before any target can be judged: the module the
/// Test ran in, and the two profiles that were written there.
struct RunProfiles {
    module: RunModule,
    selected: Profile,
    baseline: Profile,
}

impl GoCoverAdapter {
    /// The profiles of `run`, or `None` when any of them cannot be relied on:
    /// there is no run that selects no Test, it did not end with exit code 0
    /// (DES-679), a profile is missing, is not a profile of the mode asked for or
    /// has a line that cannot be read (DS-1406), or the module cannot be named.
    fn read_run(&self, run: &CoverageRun<'_>) -> Option<RunProfiles> {
        let baseline = run.baseline?;
        if baseline.exit_code != Some(0) {
            return None;
        }
        let selected = parse_profile(&fs::read_to_string(run.output_path).ok()?)?;
        let baseline = parse_profile(&fs::read_to_string(baseline.output_path).ok()?)?;
        let module = self.run_module(run.root, run.execution)?;
        Some(RunProfiles {
            module,
            selected,
            baseline,
        })
    }

    /// The module `go test` ran in (DES-643): its directory in the project, and
    /// its path as `go list -m` names it, which is what an import path in a
    /// profile starts with (DES-664). In a workspace `go list -m` lists every
    /// module of it, so the one whose directory is the run's is taken.
    fn run_module(&self, root: &Path, execution: &ExecutionDescriptor) -> Option<RunModule> {
        let dir = execution.project.as_deref()?;
        let directory = root.join(dir);
        let output = Command::new(PROGRAM)
            .current_dir(&directory)
            .envs(&self.environment)
            .args(["list", "-m", "-json"])
            .output()
            .ok()
            .filter(|output| output.status.success())?;
        let listing = String::from_utf8(output.stdout).ok()?;
        let wanted = comparable(&directory);
        let mut named = None;
        for item in serde_json::Deserializer::from_str(&listing).into_iter::<Value>() {
            let item = item.ok()?;
            let path = item.get("Path").and_then(Value::as_str)?;
            let here = item.get("Dir").and_then(Value::as_str)?;
            if comparable(Path::new(here)) == wanted {
                // Two modules in one directory would be a listing that cannot be
                // trusted.
                if named.replace(path.to_owned()).is_some() {
                    return None;
                }
            }
        }
        Some(RunModule {
            dir: dir.to_owned(),
            path: named.filter(|path| !path.is_empty())?,
        })
    }
}

/// The module a measured run was started in.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct RunModule {
    /// The module directory relative to the project root, `.` for the root.
    pub(crate) dir: String,
    /// The module path in its `go.mod`.
    pub(crate) path: String,
}

impl RunModule {
    /// DES-664: the project-relative path of the file a profile line names. The
    /// import path of a file is the module path, `/`, and the file's path from
    /// the module root, so the file is the rest after exactly `<module path>/`
    /// (a longer name that merely begins with the module path is another
    /// module's), placed under the module's directory. `None` for a file of
    /// another module.
    pub(crate) fn project_path(&self, profile_file: &str) -> Option<String> {
        let inside = profile_file
            .strip_prefix(&self.path)?
            .strip_prefix('/')
            .filter(|inside| !inside.is_empty())?;
        Some(if self.dir == "." {
            inside.to_owned()
        } else {
            format!("{}/{inside}", self.dir)
        })
    }
}

/// One block of a coverage profile (DES-664): where it starts and how many times
/// it ran.
#[derive(Clone, Debug, Eq, PartialEq)]
struct Block {
    /// The first field of the line, `<import path>/<file>`.
    file: String,
    line: u64,
    column: u64,
    count: u64,
}

/// A coverage profile that was read completely.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
struct Profile {
    blocks: Vec<Block>,
}

/// Reads a profile written under `-covermode=atomic` (DES-663). Every line is
/// read or the profile is not: a line left out could be the one that shows a
/// target ran before any Test (DS-1406).
fn parse_profile(text: &str) -> Option<Profile> {
    let mut lines = text.lines();
    if lines.next()? != PROFILE_MODE_LINE {
        return None;
    }
    lines
        .map(parse_block)
        .collect::<Option<Vec<_>>>()
        .map(|blocks| Profile { blocks })
}

/// DES-664: `<import path>/<file>:<start line>.<start column>,<end line>.<end
/// column> <number of statements> <count>`.
fn parse_block(line: &str) -> Option<Block> {
    let (rest, count) = line.rsplit_once(' ')?;
    let (rest, statements) = rest.rsplit_once(' ')?;
    let (file, span) = rest.rsplit_once(':')?;
    let (start, end) = span.split_once(',')?;
    let (line, column) = position(start)?;
    position(end)?;
    statements.parse::<u64>().ok()?;
    if file.is_empty() {
        return None;
    }
    Some(Block {
        file: file.to_owned(),
        line,
        column,
        count: count.parse().ok()?,
    })
}

fn position(text: &str) -> Option<(u64, u64)> {
    let (line, column) = text.split_once('.')?;
    Some((line.parse().ok()?, column.parse().ok()?))
}

/// A target's place in its source file: the byte range of its construct and
/// where each line of the file starts.
struct TargetSource {
    /// The target's project-relative file.
    path: String,
    start: usize,
    end: usize,
    line_starts: Vec<usize>,
}

impl TargetSource {
    /// The byte offset a block's start position stands for (the column counts
    /// bytes from 1), or `None` when the position is not in the file: a line
    /// the file does not have, or column 0.
    fn offset(&self, line: u64, column: u64) -> Option<usize> {
        let line_start = *self
            .line_starts
            .get(usize::try_from(line.checked_sub(1)?).ok()?)?;
        Some(line_start + usize::try_from(column.checked_sub(1)?).ok()?)
    }
}

/// DS-1406: every target `UNKNOWN`, with no count (DES-189).
fn all_unknown(targets: &[Locator]) -> Vec<CoverageTargetMeasurement> {
    targets
        .iter()
        .map(|target| unknown(target.clone()))
        .collect()
}

fn unknown(target: Locator) -> CoverageTargetMeasurement {
    CoverageTargetMeasurement {
        target,
        result: TargetCoverageResult::Unknown,
        count: None,
    }
}

/// The measurement of one declared target, as `(result, count)` (DES-665,
/// DES-666, DES-676).
fn measure_target(
    root: &Path,
    profiles: &RunProfiles,
    target: &Locator,
    directories: &mut BTreeMap<String, bool>,
) -> (TargetCoverageResult, Option<u64>) {
    let Some(source) = resolve_target(root, &profiles.module, target, directories) else {
        return (TargetCoverageResult::Unknown, None);
    };
    judge(
        count_in_range(&profiles.module, &profiles.selected, &source),
        count_in_range(&profiles.module, &profiles.baseline, &source),
    )
}

/// What a profile holds for a target: how many blocks start in its source range
/// (DES-665) and their counts added up. `None` when a block of the target's file
/// has a position that is not in the file, because then no count of that file
/// can be placed.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct RangeCount {
    pub(crate) blocks: usize,
    pub(crate) count: u64,
}

fn count_in_range(
    module: &RunModule,
    profile: &Profile,
    source: &TargetSource,
) -> Option<RangeCount> {
    let mut found = RangeCount {
        blocks: 0,
        count: 0,
    };
    for block in &profile.blocks {
        if module.project_path(&block.file).as_deref() != Some(source.path.as_str()) {
            continue;
        }
        let offset = source.offset(block.line, block.column)?;
        // The range is half open: the byte after the construct is not in it.
        if (source.start..source.end).contains(&offset) {
            found.blocks += 1;
            found.count = found.count.saturating_add(block.count);
        }
    }
    Some(found)
}

/// DES-665, DES-666, DES-676: the result of one target from what the Test's run
/// and the run that selects no Test hold for it.
///
/// A mistake in the run that selects no Test can only make a target `PASS` that
/// should not be, so every doubt about it is `UNKNOWN`: the target has no block
/// there, or the blocks are not placed. `PASS` needs a count above 0 in the
/// Test's run and none at all in the other.
pub(crate) fn judge(
    selected: Option<RangeCount>,
    baseline: Option<RangeCount>,
) -> (TargetCoverageResult, Option<u64>) {
    let (Some(selected), Some(baseline)) = (selected, baseline) else {
        return (TargetCoverageResult::Unknown, None);
    };
    // DES-666: no block of the target in the Test's run is a function that was
    // not found. DES-679, DS-1406: no block of it in the other run is a count
    // that is not known, not a count of 0.
    if selected.blocks == 0 || baseline.blocks == 0 {
        return (TargetCoverageResult::Unknown, None);
    }
    // DES-676: it ran before any Test.
    if baseline.count >= 1 {
        return (TargetCoverageResult::Unknown, None);
    }
    if selected.count > 0 {
        (TargetCoverageResult::Pass, Some(selected.count))
    } else {
        (TargetCoverageResult::Fail, Some(0))
    }
}

/// Places `target` in its file with the range its discovery gave it, or `None`
/// when it cannot be placed for certain: it is not a Go Locator of this
/// project, its file is not in the measured module (only the packages of that
/// module were measured, DES-663), it cannot be read as the one construct of
/// that Locator, or its package holds a line directive.
fn resolve_target(
    root: &Path,
    module: &RunModule,
    target: &Locator,
    directories: &mut BTreeMap<String, bool>,
) -> Option<TargetSource> {
    if target.adapter.as_str() != ADAPTER_ID {
        return None;
    }
    let (path, _) = target.value.split_once("::")?;
    if !Path::new(path)
        .components()
        .all(|component| matches!(component, Component::Normal(_)))
    {
        return None;
    }
    // The nearest `go.mod` of the file decides its module, as for a Test
    // (DES-684): a file of a nested module is in no package of this one.
    if module::module_of(root, path).as_deref() != Some(module.dir.as_str()) {
        return None;
    }
    let (directory, _) = path.rsplit_once('/').unwrap_or((".", path));
    let free = *directories
        .entry(directory.to_owned())
        .or_insert_with(|| package_has_no_line_directive(&root.join(directory)));
    if !free {
        return None;
    }
    let text = fs::read_to_string(root.join(path)).ok()?;
    let mut found = source_constructs(path, &text)
        .ok()?
        .into_iter()
        .filter(|construct| construct.locator == target.value);
    let (Some(construct), None) = (found.next(), found.next()) else {
        return None;
    };
    let line_starts = std::iter::once(0)
        .chain(
            text.bytes()
                .enumerate()
                .filter(|&(_, byte)| byte == b'\n')
                .map(|(index, _)| index + 1),
        )
        .collect();
    Some(TargetSource {
        path: path.to_owned(),
        start: construct.start,
        end: construct.end,
        line_starts,
    })
}

/// Whether no `.go` file of the package directory `directory` that `go` builds
/// into the package (not a `_test.go` file) holds a line directive.
///
/// A line directive renames the file and the line of the code after it, and the
/// profile then names a file of the same package that code is not in: the blocks
/// of one file would be counted for the target of another. Every file is read,
/// whatever its build constraints say, because which of them are built is not
/// known here. A directory or a file that cannot be read is not free.
fn package_has_no_line_directive(directory: &Path) -> bool {
    let Ok(entries) = fs::read_dir(directory) else {
        return false;
    };
    let mut names = BTreeSet::new();
    for entry in entries {
        let Ok(entry) = entry else {
            return false;
        };
        let name = entry.file_name().to_string_lossy().into_owned();
        if name.ends_with(".go") && !name.ends_with("_test.go") {
            names.insert(name);
        }
    }
    names.iter().all(|name| {
        fs::read_to_string(directory.join(name)).is_ok_and(|text| {
            !text.contains(LINE_DIRECTIVE_COMMENT)
                && !text
                    .lines()
                    .any(|line| line.starts_with(LINE_DIRECTIVE_AT_LINE_START))
        })
    })
}

#[cfg(test)]
mod tests;
