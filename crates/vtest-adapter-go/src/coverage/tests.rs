//! Tests of the `go-cover` CoverageAdapter that need no profile written by `go`:
//! the profiles here are written by hand, so each test names the one thing that
//! differs between the outcome it expects and the wrong outcome it rules out.
//! The runs that start `go test` itself are in
//! `vtest-cli/tests/go_coverage_e2e.rs`. The tests that start `go` here only read
//! the module path (`go list -m`).
//!
//! Every wrong-outcome test has a control: the same profile, with only the one
//! detail restored, is `PASS`, so a test cannot pass because everything is
//! `UNKNOWN` for another reason.

use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
};

use vtest_adapter_api::{BaselineRun, CoverageAdapter, CoverageRun};
use vtest_model::{AdapterId, Locator, TargetCoverageResult};

use super::*;
use crate::tests::{execution, project, Project};

const GO_MOD: &[u8] = b"module example.com/m\n\ngo 1.21\n";

/// `Add` and `Sub` are two functions of one file, one blank line apart.
const CALC: &str = "package app\n\nfunc Add(a, b int) int { return a + b }\n\nfunc Sub(a, b int) int { return a - b }\n";

type Measured = (TargetCoverageResult, Option<u64>);

const PASS_3: Measured = (TargetCoverageResult::Pass, Some(3));
const FAIL_0: Measured = (TargetCoverageResult::Fail, Some(0));
const UNKNOWN: Measured = (TargetCoverageResult::Unknown, None);

fn go_locator(value: &str) -> Locator {
    Locator {
        adapter: AdapterId::new("go"),
        value: value.to_owned(),
    }
}

/// The `(line, column)` of the byte `offset` of `text`, as a profile counts them:
/// lines from 1, and the column in bytes from 1.
fn position(text: &str, offset: usize) -> (usize, usize) {
    let before = &text[..offset];
    let line = before.matches('\n').count() + 1;
    let column = offset - before.rfind('\n').map_or(0, |newline| newline + 1) + 1;
    (line, column)
}

/// The profile line of a block that starts at the byte `offset` of `text`.
fn block_at(file: &str, text: &str, offset: usize, count: u64) -> String {
    let (line, column) = position(text, offset);
    format!("{file}:{line}.{column},{line}.{} 1 {count}\n", column + 1)
}

/// The profile line of the block of the body of the function that starts with
/// `signature`.
fn body_block(file: &str, text: &str, signature: &str, count: u64) -> String {
    let function = text.find(signature).expect("the function is in the text");
    let brace = function + text[function..].find('{').expect("a body");
    block_at(file, text, brace, count)
}

fn profile(rows: &[String]) -> String {
    format!("mode: atomic\n{}", rows.concat())
}

fn module(dir: &str, path: &str) -> RunModule {
    RunModule {
        dir: dir.to_owned(),
        path: path.to_owned(),
    }
}

/// The project of the module `example.com/m` at the root, with `CALC`.
fn calc_project() -> Project {
    project(&[("go.mod", GO_MOD), ("app/calc.go", CALC.as_bytes())])
}

/// Measures `targets` of `fixture` from the two profile texts, through the same
/// function `measure_run` ends in, with the module given and not read from `go`.
fn measure(
    fixture: &Project,
    run_module: &RunModule,
    selected: &str,
    baseline: &str,
    targets: &[&str],
) -> Vec<Measured> {
    let profiles = RunProfiles {
        module: run_module.clone(),
        selected: parse_profile(selected).expect("a profile"),
        baseline: parse_profile(baseline).expect("a profile"),
    };
    let mut directories = BTreeMap::new();
    targets
        .iter()
        .map(|target| {
            measure_target(
                &fixture.root,
                &profiles,
                &go_locator(target),
                &mut directories,
            )
        })
        .collect()
}

/// The file name of the module `example.com/m` in a profile line.
const CALC_FILE: &str = "example.com/m/app/calc.go";

/// The profile where `Add` ran `selected` times in the Test's run and `baseline`
/// times in the run that selects no Test, and `Sub` ran as often.
fn calc_profiles(selected: u64, baseline: u64) -> (String, String) {
    let rows = |count: u64| {
        profile(&[
            body_block(CALC_FILE, CALC, "func Add", count),
            body_block(CALC_FILE, CALC, "func Sub", 0),
        ])
    };
    (rows(selected), rows(baseline))
}

/// DES-664: a profile is read only when it is one of the mode asked for and
/// every line of it reads; a line left out could be the one that shows a target
/// ran before any Test.
/// @vtest.id TEST-ADAPTER-GO-PROFILE-IS-READ-ONLY-WHEN-EVERY-LINE-READS
/// @vtest.covers VO-ADAPTER-GO-PROFILE-FILE-IS-CONVERTED-TO-A-PROJECT-PATH, VO-ADAPTER-GO-UNCERTAIN-COUNT-IS-UNKNOWN
/// @vtest.target crates/vtest-adapter-go/src/coverage.rs::parse_profile
/// @vtest.intent a profile of mode atomic whose lines are import path, file, positions, statements and count is read with its blocks, and one of another mode, with an empty line, a position, a statement count or a count that is not a number, or a line without a field is not read at all
#[test]
fn a_profile_is_read_only_when_every_line_reads() {
    let good = "mode: atomic\nexample.com/m/app/calc.go:3.24,3.40 1 7\nexample.com/m/app/calc.go:5.15,5.16 0 0\n";
    let read = parse_profile(good).expect("a profile");
    assert_eq!(
        read.blocks,
        [
            Block {
                file: "example.com/m/app/calc.go".to_owned(),
                line: 3,
                column: 24,
                count: 7
            },
            Block {
                file: "example.com/m/app/calc.go".to_owned(),
                line: 5,
                column: 15,
                count: 0
            }
        ]
    );
    assert_eq!(
        parse_profile("mode: atomic\n"),
        Some(Profile { blocks: vec![] })
    );
    // Windows writes the same text; a carriage return is not part of a field.
    assert_eq!(
        parse_profile(&good.replace('\n', "\r\n")).map(|read| read.blocks.len()),
        Some(2)
    );
    for (name, bad) in [
        ("empty", ""),
        ("mode set", "mode: set\nexample.com/m/a.go:1.1,1.2 1 1\n"),
        ("mode count", "mode: count\nexample.com/m/a.go:1.1,1.2 1 1\n"),
        ("no mode line", "example.com/m/a.go:1.1,1.2 1 1\n"),
        ("empty line", "mode: atomic\n\nexample.com/m/a.go:1.1,1.2 1 1\n"),
        ("no count", "mode: atomic\nexample.com/m/a.go:1.1,1.2 1\n"),
        ("count not a number", "mode: atomic\nexample.com/m/a.go:1.1,1.2 1 x\n"),
        ("negative count", "mode: atomic\nexample.com/m/a.go:1.1,1.2 1 -1\n"),
        ("statements not a number", "mode: atomic\nexample.com/m/a.go:1.1,1.2 y 1\n"),
        ("no position", "mode: atomic\nexample.com/m/a.go 1 1\n"),
        ("no end", "mode: atomic\nexample.com/m/a.go:1.1 1 1\n"),
        ("column missing", "mode: atomic\nexample.com/m/a.go:1,1.2 1 1\n"),
        ("no file", "mode: atomic\n:1.1,1.2 1 1\n"),
        ("one bad line among good ones", "mode: atomic\nexample.com/m/a.go:1.1,1.2 1 1\nexample.com/m/a.go:2.1,2.2 1 oops\nexample.com/m/a.go:3.1,3.2 1 1\n"),
    ] {
        assert_eq!(parse_profile(bad), None, "{name}");
    }
}

/// DES-664: the file of a profile line is converted to a project path with the
/// module path and the module directory, on the exact `<module path>/` prefix.
/// @vtest.id TEST-ADAPTER-GO-PROFILE-FILE-IS-CONVERTED-TO-A-PROJECT-PATH
/// @vtest.covers VO-ADAPTER-GO-PROFILE-FILE-IS-CONVERTED-TO-A-PROJECT-PATH
/// @vtest.target crates/vtest-adapter-go/src/coverage.rs::RunModule::project_path
/// @vtest.intent a file under the module path of a root module is the same path in the project, one under the module path of a module in a directory is that path below the directory, and a file of another module, one whose path only begins with the module path, the module path alone, and an empty rest are no project path
#[test]
fn a_profile_file_is_converted_with_the_module_path_and_directory() {
    let root_module = module(".", "example.com/m");
    assert_eq!(
        root_module
            .project_path("example.com/m/app/calc.go")
            .as_deref(),
        Some("app/calc.go")
    );
    assert_eq!(
        root_module.project_path("example.com/m/calc.go").as_deref(),
        Some("calc.go")
    );
    let nested = module("tools/gen", "example.com/gen");
    assert_eq!(
        nested
            .project_path("example.com/gen/calc/calc.go")
            .as_deref(),
        Some("tools/gen/calc/calc.go")
    );
    // The same file name in two packages is two project paths.
    assert_eq!(
        root_module
            .project_path("example.com/m/a/util.go")
            .as_deref(),
        Some("a/util.go")
    );
    assert_eq!(
        root_module
            .project_path("example.com/m/b/util.go")
            .as_deref(),
        Some("b/util.go")
    );
    for foreign in [
        "example.com/other/app/calc.go",
        "example.com/mx/app/calc.go",
        "example.com/m",
        "example.com/m/",
        "app/calc.go",
        "",
    ] {
        assert_eq!(root_module.project_path(foreign), None, "{foreign}");
    }
}

/// DES-665, DES-666, DES-676, DES-679, DS-1406: the result of a target from what
/// the Test's run and the run that selects no Test hold for it.
/// @vtest.id TEST-ADAPTER-GO-TARGET-RESULT-FROM-THE-TWO-RUNS
/// @vtest.covers VO-ADAPTER-GO-TARGET-COUNT-IS-THE-BLOCKS-STARTING-IN-ITS-RANGE, VO-ADAPTER-GO-TARGET-WITH-NO-BLOCK-IS-UNKNOWN, VO-ADAPTER-GO-REACH-BEFORE-ANY-TEST-IS-UNKNOWN, VO-ADAPTER-GO-UNCERTAIN-COUNT-IS-UNKNOWN, VO-EXEC-TARGET-COVERAGE-COUNT-JUDGEMENT
/// @vtest.target crates/vtest-adapter-go/src/coverage.rs::judge
/// @vtest.intent a target with blocks in both runs, none run in the run that selects no Test and some in the Test's run is PASS with that count, one that did not run in the Test's run is FAIL with count 0, one that ran in the run that selects no Test, whatever the Test's run did, one with no block in either run, and one whose blocks cannot be placed are UNKNOWN with no count
#[test]
fn a_target_is_pass_only_when_it_ran_in_the_tests_run_and_not_before_any_test() {
    let blocks = |blocks: usize, count: u64| Some(RangeCount { blocks, count });
    assert_eq!(judge(blocks(2, 3), blocks(2, 0)), PASS_3);
    assert_eq!(judge(blocks(2, 0), blocks(2, 0)), FAIL_0);
    // Ran before any Test (init, a package variable, TestMain): not the Test's.
    assert_eq!(judge(blocks(2, 3), blocks(2, 1)), UNKNOWN);
    assert_eq!(judge(blocks(2, 3), blocks(2, 9)), UNKNOWN);
    // Ran only in the run that selects no Test: not FAIL either, for it ran.
    assert_eq!(judge(blocks(2, 0), blocks(2, 1)), UNKNOWN);
    // No block: the function was not found in the Test's run (DES-666), and a
    // count of 0 is not read from no block in the other run (DES-679).
    assert_eq!(judge(blocks(0, 0), blocks(2, 0)), UNKNOWN);
    assert_eq!(judge(blocks(2, 3), blocks(0, 0)), UNKNOWN);
    assert_eq!(judge(blocks(0, 0), blocks(0, 0)), UNKNOWN);
    // A block that could not be placed in either run.
    assert_eq!(judge(None, blocks(2, 0)), UNKNOWN);
    assert_eq!(judge(blocks(2, 3), None), UNKNOWN);
    assert_eq!(judge(None, None), UNKNOWN);
    // The counts add up without a wrap.
    assert_eq!(
        judge(blocks(2, u64::MAX), blocks(2, 0)),
        (TargetCoverageResult::Pass, Some(u64::MAX))
    );
}

/// DES-665: the count of a target is the sum over the blocks that start in the
/// range of its construct, the range is half open, and a block that starts just
/// outside it is not the target's.
/// @vtest.id TEST-ADAPTER-GO-BLOCK-JUST-OUTSIDE-THE-RANGE-IS-NOT-COUNTED
/// @vtest.covers VO-ADAPTER-GO-TARGET-COUNT-IS-THE-BLOCKS-STARTING-IN-ITS-RANGE
/// @vtest.target crates/vtest-adapter-go/src/coverage.rs::count_in_range
/// @vtest.intent the counts of the blocks that start at the first and at the last byte of a function add up, those of a block that starts on the byte before the function and of one that starts on the byte after it are not added, and a function whose only run block starts just outside it is FAIL with count 0 and not PASS
#[test]
fn a_block_starting_just_outside_the_range_is_not_counted() {
    let fixture = calc_project();
    let run_module = module(".", "example.com/m");
    let sub = CALC.find("func Sub").expect("Sub");
    let sub_end = CALC.len() - 1; // the line break after the closing brace
                                  // The counts of four blocks: at the first byte of `Sub`, at its closing
                                  // brace (the last byte), on the byte before it and on the byte after it.
    let rows = |first: u64, last: u64, before: u64, after: u64| {
        profile(&[
            block_at(CALC_FILE, CALC, sub, first),
            block_at(CALC_FILE, CALC, sub_end - 1, last),
            block_at(CALC_FILE, CALC, sub - 1, before),
            block_at(CALC_FILE, CALC, sub_end, after),
        ])
    };
    let measured = |selected: String| {
        measure(
            &fixture,
            &run_module,
            &selected,
            &rows(0, 0, 0, 0),
            &["app/calc.go::Sub"],
        )[0]
    };
    // The control: the blocks inside the range, and only they, are the count.
    assert_eq!(
        measured(rows(1, 2, 0, 0)),
        (TargetCoverageResult::Pass, Some(3))
    );
    assert_eq!(
        measured(rows(1, 2, 100, 1000)),
        (TargetCoverageResult::Pass, Some(3))
    );
    // The wrong outcome: all the Test ran was just outside the function.
    assert_eq!(measured(rows(0, 0, 100, 1000)), FAIL_0);
}

/// DES-664, DES-665: two packages with a file of one name hold one function of
/// one name at one place, and a Test that ran one of them reached only that one.
/// @vtest.id TEST-ADAPTER-GO-SAME-FILE-NAME-IN-TWO-PACKAGES-IS-TWO-TARGETS
/// @vtest.covers VO-ADAPTER-GO-PROFILE-FILE-IS-CONVERTED-TO-A-PROJECT-PATH, VO-ADAPTER-GO-TARGET-COUNT-IS-THE-BLOCKS-STARTING-IN-ITS-RANGE
/// @vtest.target crates/vtest-adapter-go/src/coverage.rs::measure_target
/// @vtest.intent with a util.go in package a and one in package b that have the same text, the target in a is PASS and the target in b FAIL with count 0 when only a's function ran, so the count of one file is never taken for the other, whichever order the profile lists them in
#[test]
fn the_same_file_name_in_two_packages_does_not_share_a_count() {
    let util = "package p\n\nfunc F() int { return 1 }\n";
    let fixture = project(&[
        ("go.mod", GO_MOD),
        ("a/util.go", util.as_bytes()),
        ("b/util.go", util.as_bytes()),
    ]);
    let run_module = module(".", "example.com/m");
    let a = "example.com/m/a/util.go";
    let b = "example.com/m/b/util.go";
    for (name, rows) in [
        ("a then b", [(a, 4), (b, 0)]),
        ("b then a", [(b, 0), (a, 4)]),
    ] {
        let selected = profile(
            &rows
                .iter()
                .map(|(file, count)| body_block(file, util, "func F", *count))
                .collect::<Vec<_>>(),
        );
        let baseline = profile(
            &rows
                .iter()
                .map(|(file, _)| body_block(file, util, "func F", 0))
                .collect::<Vec<_>>(),
        );
        let measured = measure(
            &fixture,
            &run_module,
            &selected,
            &baseline,
            &["a/util.go::F", "b/util.go::F"],
        );
        assert_eq!(
            measured,
            [(TargetCoverageResult::Pass, Some(4)), FAIL_0],
            "{name}"
        );
    }
}

/// DES-663, DES-684: only the packages of the module `go test` ran in were
/// measured, so a target in another module, nested or not, is not measured, and
/// a file that has the name of a file of the measured module is not that file.
/// @vtest.id TEST-ADAPTER-GO-TARGET-OUTSIDE-THE-MEASURED-MODULE-IS-UNKNOWN
/// @vtest.covers VO-ADAPTER-GO-PROFILE-FILE-IS-CONVERTED-TO-A-PROJECT-PATH, VO-ADAPTER-GO-UNCERTAIN-COUNT-IS-UNKNOWN
/// @vtest.target crates/vtest-adapter-go/src/coverage.rs::resolve_target
/// @vtest.intent a target in a module nested below the measured module, whose module path is the same as the measured module's, is UNKNOWN although the profile has a block of the same module path and file name that ran, and the same target of the measured module itself is PASS
#[test]
fn a_target_in_another_module_is_unknown_even_with_the_same_module_path() {
    let util = "package p\n\nfunc F() int { return 1 }\n";
    let fixture = project(&[
        ("go.mod", GO_MOD),
        ("p/util.go", util.as_bytes()),
        // A second module that names itself as the first does.
        ("nested/go.mod", GO_MOD),
        ("nested/p/util.go", util.as_bytes()),
        // A module whose path goes on from the first's, as a module nested in a
        // module usually does.
        ("sub/go.mod", b"module example.com/m/sub\n\ngo 1.21\n"),
        ("sub/p/util.go", util.as_bytes()),
    ]);
    let file = "example.com/m/p/util.go";
    let selected = profile(&[body_block(file, util, "func F", 5)]);
    let baseline = profile(&[body_block(file, util, "func F", 0)]);
    let measured = measure(
        &fixture,
        &module(".", "example.com/m"),
        &selected,
        &baseline,
        &["p/util.go::F", "nested/p/util.go::F"],
    );
    assert_eq!(
        measured,
        [(TargetCoverageResult::Pass, Some(5)), UNKNOWN],
        "the nested module's file is no file of the measured module"
    );
    // Run in the nested module itself, its own file is the measured one.
    let nested = measure(
        &fixture,
        &module("nested", "example.com/m"),
        &selected,
        &baseline,
        &["p/util.go::F", "nested/p/util.go::F"],
    );
    assert_eq!(nested, [UNKNOWN, (TargetCoverageResult::Pass, Some(5))]);

    // A profile that names a file of the module below it by the first module's
    // path (as a workspace that uses both modules would write it) places the
    // block at the path of that file, but the file is not in the measured
    // module: UNKNOWN, where the module check is the one thing that says so.
    let sub_file = "example.com/m/sub/p/util.go";
    let selected = profile(&[body_block(sub_file, util, "func F", 7)]);
    let baseline = profile(&[body_block(sub_file, util, "func F", 0)]);
    assert_eq!(
        measure(
            &fixture,
            &module(".", "example.com/m"),
            &selected,
            &baseline,
            &["sub/p/util.go::F"],
        ),
        [UNKNOWN]
    );
    assert_eq!(
        measure(
            &fixture,
            &module("sub", "example.com/m/sub"),
            &selected,
            &baseline,
            &["sub/p/util.go::F"],
        ),
        [(TargetCoverageResult::Pass, Some(7))],
        "the same profile of the module's own run is PASS"
    );
}

/// DES-676: whatever ran before any Test is not a Test's reach.
/// @vtest.id TEST-ADAPTER-GO-TARGET-THAT-RAN-BEFORE-ANY-TEST-IS-UNKNOWN
/// @vtest.covers VO-ADAPTER-GO-REACH-BEFORE-ANY-TEST-IS-UNKNOWN
/// @vtest.target crates/vtest-adapter-go/src/coverage.rs::measure_target
/// @vtest.intent a function that ran in the run that selects no Test is UNKNOWN whether or not the Test's run ran it more, one of the same file that did not is PASS, and a function that has a block in the Test's run and none in the other run is UNKNOWN and not read as run zero times
#[test]
fn a_target_that_ran_before_any_test_is_unknown() {
    let fixture = calc_project();
    let run_module = module(".", "example.com/m");
    let selected = profile(&[
        body_block(CALC_FILE, CALC, "func Add", 4),
        body_block(CALC_FILE, CALC, "func Sub", 4),
    ]);
    let baseline = profile(&[
        body_block(CALC_FILE, CALC, "func Add", 1),
        body_block(CALC_FILE, CALC, "func Sub", 0),
    ]);
    assert_eq!(
        measure(
            &fixture,
            &run_module,
            &selected,
            &baseline,
            &["app/calc.go::Add", "app/calc.go::Sub"]
        ),
        [UNKNOWN, (TargetCoverageResult::Pass, Some(4))]
    );
    // The other run has no block of Sub at all.
    let baseline = profile(&[body_block(CALC_FILE, CALC, "func Add", 0)]);
    assert_eq!(
        measure(
            &fixture,
            &run_module,
            &selected,
            &baseline,
            &["app/calc.go::Add", "app/calc.go::Sub"]
        ),
        [(TargetCoverageResult::Pass, Some(4)), UNKNOWN]
    );
}

/// DES-666: a target with no block in the profile is not found, and it is not
/// FAIL: nothing was measured of it.
/// @vtest.id TEST-ADAPTER-GO-TARGET-WITH-NO-BLOCK-IS-UNKNOWN
/// @vtest.covers VO-ADAPTER-GO-TARGET-WITH-NO-BLOCK-IS-UNKNOWN, VO-EXEC-COVERAGE-TARGET-NOT-LOCATED-IS-UNKNOWN
/// @vtest.target crates/vtest-adapter-go/src/coverage.rs::measure_target
/// @vtest.intent a function none of whose blocks is in the profile, because its package was not linked into the test binary or its file is not in the profile, is UNKNOWN with no count, and a function of the same file that has a block is measured
#[test]
fn a_target_with_no_block_is_unknown_not_fail() {
    let fixture = calc_project();
    let run_module = module(".", "example.com/m");
    let only_add = profile(&[body_block(CALC_FILE, CALC, "func Add", 2)]);
    let only_add_baseline = profile(&[body_block(CALC_FILE, CALC, "func Add", 0)]);
    assert_eq!(
        measure(
            &fixture,
            &run_module,
            &only_add,
            &only_add_baseline,
            &["app/calc.go::Add", "app/calc.go::Sub"]
        ),
        [(TargetCoverageResult::Pass, Some(2)), UNKNOWN]
    );
    // No block of the file at all.
    let empty = profile(&[]);
    assert_eq!(
        measure(&fixture, &run_module, &empty, &empty, &["app/calc.go::Add"]),
        [UNKNOWN]
    );
    // A profile of another file only.
    let other = profile(&[body_block(
        "example.com/m/app/other.go",
        CALC,
        "func Add",
        2,
    )]);
    assert_eq!(
        measure(&fixture, &run_module, &other, &other, &["app/calc.go::Add"]),
        [UNKNOWN]
    );
}

/// DS-1406: a target that cannot be placed for certain is UNKNOWN: a profile
/// position that is not in the file, a Locator that names no function of its
/// file, a file that is gone, a Locator of another adapter or a path out of the
/// project.
/// @vtest.id TEST-ADAPTER-GO-TARGET-THAT-CANNOT-BE-PLACED-IS-UNKNOWN
/// @vtest.covers VO-ADAPTER-GO-UNCERTAIN-COUNT-IS-UNKNOWN
/// @vtest.target crates/vtest-adapter-go/src/coverage.rs::resolve_target
/// @vtest.intent a block of the target's file at a line the file has not, at column 0, and a Locator that names no function, a missing file, a Locator of another adapter and a path with a parent component are each UNKNOWN, with the same profile giving PASS to the target that is placed
#[test]
fn a_target_that_cannot_be_placed_is_unknown() {
    let fixture = calc_project();
    let run_module = module(".", "example.com/m");
    let selected = profile(&[body_block(CALC_FILE, CALC, "func Add", 2)]);
    let baseline = profile(&[body_block(CALC_FILE, CALC, "func Add", 0)]);
    let run = |targets: &[&str]| measure(&fixture, &run_module, &selected, &baseline, targets);
    assert_eq!(
        run(&["app/calc.go::Add"]),
        [(TargetCoverageResult::Pass, Some(2))]
    );
    assert_eq!(
        run(&[
            "app/calc.go::Mul",
            "app/gone.go::Add",
            "../calc.go::Add",
            "/abs/calc.go::Add",
            "app/calc.go",
        ]),
        [UNKNOWN; 5]
    );
    let foreign = Locator {
        adapter: AdapterId::new("rust-cargo"),
        value: "app/calc.go::Add".to_owned(),
    };
    let profiles = RunProfiles {
        module: run_module.clone(),
        selected: parse_profile(&selected).expect("a profile"),
        baseline: parse_profile(&baseline).expect("a profile"),
    };
    assert_eq!(
        measure_target(&fixture.root, &profiles, &foreign, &mut BTreeMap::new()),
        UNKNOWN
    );
    // A block of the file that is not in the file, in either run.
    let beyond = format!("{selected}{CALC_FILE}:99.1,99.2 1 0\n");
    let past_the_end = measure(
        &fixture,
        &run_module,
        &beyond,
        &baseline,
        &["app/calc.go::Add"],
    );
    assert_eq!(past_the_end, [UNKNOWN]);
    let beyond_baseline = format!("{baseline}{CALC_FILE}:3.0,3.1 1 0\n");
    assert_eq!(
        measure(
            &fixture,
            &run_module,
            &selected,
            &beyond_baseline,
            &["app/calc.go::Add"]
        ),
        [UNKNOWN]
    );
}

/// A line directive renames the file of the code after it: the blocks of one file
/// would be counted for the target of another.
/// @vtest.id TEST-ADAPTER-GO-LINE-DIRECTIVE-IN-THE-PACKAGE-MAKES-TARGETS-UNKNOWN
/// @vtest.covers VO-ADAPTER-GO-UNCERTAIN-COUNT-IS-UNKNOWN
/// @vtest.target crates/vtest-adapter-go/src/coverage.rs::package_has_no_line_directive
/// @vtest.intent a package whose non-test file holds a line comment directive at the start of a line or a block comment directive anywhere gives UNKNOWN to its targets although the profile says one ran, a directive in a _test.go file or in another package does not, and the same package without the directive is PASS
#[test]
fn a_line_directive_in_the_package_makes_its_targets_unknown() {
    let rename_to_add = "package app\n\n//line calc.go:3\nfunc Elsewhere() int { return 1 }\n";
    let block_form = "package app\n\nvar _ = 1 /*line calc.go:3*/\n";
    let selected = profile(&[body_block(CALC_FILE, CALC, "func Add", 2)]);
    let baseline = profile(&[body_block(CALC_FILE, CALC, "func Add", 0)]);
    let run_module = module(".", "example.com/m");
    let outcome = |extra: &[(&str, &str)]| {
        let mut files = vec![("go.mod", GO_MOD), ("app/calc.go", CALC.as_bytes())];
        files.extend(extra.iter().map(|(path, text)| (*path, text.as_bytes())));
        let fixture = project(&files);
        measure(
            &fixture,
            &run_module,
            &selected,
            &baseline,
            &["app/calc.go::Add"],
        )[0]
    };
    let pass = (TargetCoverageResult::Pass, Some(2));
    assert_eq!(outcome(&[]), pass);
    assert_eq!(outcome(&[("app/elsewhere.go", rename_to_add)]), UNKNOWN);
    assert_eq!(outcome(&[("app/elsewhere.go", block_form)]), UNKNOWN);
    // What the directive renames is in its own package, and a test file is not
    // part of the package that is instrumented.
    assert_eq!(outcome(&[("app/elsewhere_test.go", rename_to_add)]), pass);
    assert_eq!(outcome(&[("other/elsewhere.go", rename_to_add)]), pass);
    // `//line` not at the start of a line is a comment.
    assert_eq!(
        outcome(&[(
            "app/elsewhere.go",
            "package app\n\n// not a //line calc.go:3 directive\n"
        )]),
        pass
    );
}

fn require_go() {
    let ran = Command::new("go")
        .arg("version")
        .output()
        .ok()
        .filter(|output| output.status.success());
    assert!(
        ran.is_some(),
        "`go` must be on PATH: these tests read the module path from the real go command"
    );
}

/// A measured run of `fixture`: the Test's profile `selected` and the profile of
/// the run that selects no Test `baseline`, written under the project's
/// `.verify/cache/cov` as `vtest run` writes them.
struct Run {
    fixture: Project,
    selected: PathBuf,
    baseline: PathBuf,
}

fn run_of(selected: &str, baseline: &str) -> Run {
    let fixture = calc_project();
    let cov = fixture.root.join(".verify/cache/cov");
    fs::create_dir_all(&cov).expect("create the cache directory");
    let (selected_path, baseline_path) = (cov.join("01SELECTED"), cov.join("01BASELINE"));
    fs::write(&selected_path, selected).expect("write the profile");
    fs::write(&baseline_path, baseline).expect("write the profile");
    Run {
        fixture,
        selected: selected_path,
        baseline: baseline_path,
    }
}

impl Run {
    fn measure(&self, exit_code: Option<i32>, baseline: Option<&PathBuf>) -> Vec<Measured> {
        self.measure_at(&self.selected, exit_code, baseline)
    }

    fn measure_at(
        &self,
        selected: &Path,
        exit_code: Option<i32>,
        baseline: Option<&PathBuf>,
    ) -> Vec<Measured> {
        let execution = execution(Some("."), Some("app"), "TestAdd");
        let run = CoverageRun {
            root: &self.fixture.root,
            execution: &execution,
            output_path: selected,
            baseline: baseline.map(|output_path| BaselineRun {
                output_path: output_path.as_path(),
                exit_code,
            }),
        };
        GoCoverAdapter::new()
            .measure_run(&run, &[go_locator("app/calc.go::Add")])
            .into_iter()
            .map(|measurement| (measurement.result, measurement.count))
            .collect()
    }
}

/// DES-679, DS-1406: the run that selects no Test did not end with exit code 0,
/// left no profile, or left one that cannot be read: every target is UNKNOWN,
/// and the count of that run is not read as 0.
/// @vtest.id TEST-ADAPTER-GO-UNUSABLE-NO-TEST-RUN-MAKES-TARGETS-UNKNOWN
/// @vtest.covers VO-ADAPTER-GO-UNUSABLE-NO-TEST-RUN-MAKES-TARGETS-UNKNOWN, VO-ADAPTER-GO-UNCERTAIN-COUNT-IS-UNKNOWN
/// @vtest.target crates/vtest-adapter-go/src/coverage.rs::GoCoverAdapter::measure_run
/// @vtest.intent with the Test's profile showing the target ran and the other run's profile showing it did not, the result is PASS for exit code 0, and UNKNOWN for exit code 1, for no exit code, for no baseline run, for a profile file that is not there, for an empty profile, for one of another mode, and for one with a line that cannot be read, and without a Test's profile
#[test]
fn an_unusable_run_that_selects_no_test_makes_every_target_unknown() {
    require_go();
    let (selected, baseline) = calc_profiles(3, 0);
    let run = run_of(&selected, &baseline);
    let pass = [(TargetCoverageResult::Pass, Some(3))];
    assert_eq!(run.measure(Some(0), Some(&run.baseline)), pass);
    for (name, exit_code) in [("exit 1", Some(1)), ("no exit code", None)] {
        assert_eq!(
            run.measure(exit_code, Some(&run.baseline)),
            [UNKNOWN],
            "{name}"
        );
    }
    assert_eq!(run.measure(Some(0), None), [UNKNOWN], "no baseline run");
    let gone = run.fixture.root.join(".verify/cache/cov/01GONE");
    assert_eq!(run.measure(Some(0), Some(&gone)), [UNKNOWN], "no profile");
    assert_eq!(
        run.measure_at(&gone, Some(0), Some(&run.baseline)),
        [UNKNOWN],
        "no profile of the Test's run"
    );
    for (name, text) in [
        ("empty", String::new()),
        ("mode set", baseline.replace("atomic", "set")),
        ("garbled line", format!("{baseline}garbage\n")),
    ] {
        fs::write(&run.baseline, text).expect("rewrite the profile");
        assert_eq!(
            run.measure(Some(0), Some(&run.baseline)),
            [UNKNOWN],
            "{name}"
        );
    }
    fs::write(&run.baseline, &baseline).expect("restore the profile");
    assert_eq!(run.measure(Some(0), Some(&run.baseline)), pass);
}

/// DES-673: each profile is a file of its own, and exactly the files handed in
/// are read: a profile of a stale run next to them, or the Test's profile and the
/// other run's swapped, cannot stand for them.
/// @vtest.id TEST-ADAPTER-GO-ONLY-THE-PROFILES-HANDED-IN-ARE-READ
/// @vtest.covers VO-ADAPTER-GO-PROFILES-ARE-SEPARATE-FILES-UNDER-CACHE-COV
/// @vtest.target crates/vtest-adapter-go/src/coverage.rs::GoCoverAdapter::measure_run
/// @vtest.intent a profile left in the cache directory by an earlier run, in which the target ran, is not read for a run whose own profile says it did not, the Test's profile and the other run's profile read from each other's place give the target FAIL and UNKNOWN and not PASS, and the same files in their own places give PASS
#[test]
fn only_the_profiles_handed_in_are_read() {
    require_go();
    let (ran, did_not_run) = calc_profiles(3, 0);
    let run = run_of(&did_not_run, &did_not_run);
    let stale = run.fixture.root.join(".verify/cache/cov/01STALE");
    fs::write(&stale, &ran).expect("write the stale profile");
    assert_eq!(run.measure(Some(0), Some(&run.baseline)), [FAIL_0]);
    // The directory is not searched for a profile either.
    fs::remove_file(&run.selected).expect("remove the profile");
    assert_eq!(run.measure(Some(0), Some(&run.baseline)), [UNKNOWN]);

    // Swapped: the Test's profile is the one that is all zero.
    let run = run_of(&ran, &did_not_run);
    assert_eq!(
        run.measure_at(&run.baseline, Some(0), Some(&run.selected)),
        [UNKNOWN],
        "the Test's run read from the other run's place"
    );
    assert_eq!(
        run.measure_at(&run.selected, Some(0), Some(&run.selected)),
        [UNKNOWN],
        "the same profile for both runs"
    );
    assert_eq!(
        run.measure(Some(0), Some(&run.baseline)),
        [(TargetCoverageResult::Pass, Some(3))]
    );
}

/// DES-664: the module path is the one the run's `go.mod` names, read by `go`,
/// and a project whose go.mod path differs from the profile's import paths does
/// not match.
/// @vtest.id TEST-ADAPTER-GO-MODULE-PATH-COMES-FROM-GO-LIST
/// @vtest.covers VO-ADAPTER-GO-PROFILE-FILE-IS-CONVERTED-TO-A-PROJECT-PATH
/// @vtest.target crates/vtest-adapter-go/src/coverage.rs::GoCoverAdapter::run_module
/// @vtest.intent the module of a module directory is that directory and the path its go.mod names, for a root module and for one nested below the root, a directory without a go.mod and one whose go list fails have no module, and a profile of import paths of another module path measures nothing
#[test]
fn the_module_is_read_from_go_list() {
    require_go();
    let fixture = project(&[
        ("go.mod", GO_MOD),
        ("tools/go.mod", b"module \"example.com/tools\"\n\ngo 1.21\n"),
        ("broken/go.mod", b"this is not a module file\n"),
        ("none/x.go", b"package none\n"),
    ]);
    let adapter = GoCoverAdapter::new();
    let module_of =
        |dir: &str| adapter.run_module(&fixture.root, &execution(Some(dir), Some("."), "T"));
    assert_eq!(module_of("."), Some(module(".", "example.com/m")));
    assert_eq!(
        module_of("tools"),
        Some(module("tools", "example.com/tools"))
    );
    assert_eq!(module_of("broken"), None);
    assert_eq!(module_of("gone"), None);
    // `go list -m` in a directory with no go.mod of its own reads the parent's,
    // which is not the module of that directory.
    assert_eq!(module_of("none"), None);
    assert_eq!(
        adapter.run_module(&fixture.root, &execution(None, Some("."), "T")),
        None
    );
}

/// DS-760, DS-908: whether `go` runs here decides whether coverage can be
/// measured; a command that cannot be started is unavailable with the reason the
/// adapter words, and one that runs is available.
/// @vtest.id TEST-ADAPTER-GO-COVERAGE-AVAILABILITY-IS-WHETHER-GO-RUNS
/// @vtest.covers VO-EXEC-COVERAGE-UNAVAILABLE-NOT-CHECKED
/// @vtest.target crates/vtest-adapter-go/src/coverage.rs::probe_availability
/// @vtest.intent a go command that cannot be started is unavailable coverage with the reason that target_coverage is NOT_CHECKED, a go command whose version fails is too, and the real go command is available
#[test]
fn availability_is_whether_the_go_command_runs() {
    require_go();
    let root = std::env::temp_dir();
    let environment = BTreeMap::new();
    assert_eq!(probe_availability(PROGRAM, &root, &environment), Ok(()));
    let reason = probe_availability(
        "vtest-nonexistent-go-binary-for-testing",
        &root,
        &environment,
    )
    .expect_err("a command that cannot be started is unavailable");
    assert_eq!(
        reason.to_string(),
        "`go version` cannot be run; target_coverage is NOT_CHECKED"
    );
    // A program that starts and fails: `go` run in a directory that is not there.
    assert!(
        probe_availability(PROGRAM, &root.join("vtest-no-such-directory"), &environment).is_err()
    );
}

/// DS-1406, DES-675: without the run that selects no Test, and by `measure`
/// alone, a profile cannot say what a Test reached.
/// @vtest.id TEST-ADAPTER-GO-MEASURE-WITHOUT-THE-OTHER-RUN-IS-UNKNOWN
/// @vtest.covers VO-ADAPTER-GO-MEASURES-THE-RUN-THAT-SELECTS-NO-TEST, VO-ADAPTER-GO-UNCERTAIN-COUNT-IS-UNKNOWN
/// @vtest.target crates/vtest-adapter-go/src/coverage.rs::GoCoverAdapter::measure
/// @vtest.intent measure, which reads one profile and knows no module, gives every target UNKNOWN with no count, whatever the profile holds, and names the file extension of its output as none
#[test]
fn a_profile_alone_measures_nothing() {
    let (ran, _) = calc_profiles(3, 0);
    let run = run_of(&ran, &ran);
    let adapter = GoCoverAdapter::new();
    let measured = adapter.measure(
        &run.fixture.root,
        &run.selected,
        &[
            go_locator("app/calc.go::Add"),
            go_locator("app/calc.go::Sub"),
        ],
    );
    assert_eq!(
        measured
            .iter()
            .map(|measurement| (measurement.result, measurement.count))
            .collect::<Vec<_>>(),
        [UNKNOWN; 2]
    );
    assert_eq!(adapter.output_extension(), None);
    assert_eq!(adapter.method(), "go-cover");
}
