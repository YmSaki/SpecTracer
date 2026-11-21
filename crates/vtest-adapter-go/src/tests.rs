//! Tests of the `go` adapter that need no `go` command: its declaration, the
//! coordinates it gives a Test, the Tests its runner refuses, and the reading
//! of `go test -json` output. The output used here was captured from go 1.26.4;
//! the runs that start `go` itself are in `vtest-cli/tests/go_runner_e2e.rs` and
//! `execution_state/tests.rs`.

use std::{
    fs,
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
};

use vtest_adapter_api::{
    AdapterRegistry, AdapterScanConfig, RunnerCommand, RunnerTestResult, TestRunnerError,
};
use vtest_model::{AdapterId, ExecutionDescriptor, TestSuite};

use super::*;
use crate::runner::{coordinates, interpret, invocation_args};

static NEXT: AtomicU64 = AtomicU64::new(0);

/// A project on disk, removed when dropped.
pub(crate) struct Project {
    pub(crate) root: PathBuf,
}

impl Drop for Project {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

/// A project directory holding `files` (paths relative to its root).
pub(crate) fn project(files: &[(&str, &[u8])]) -> Project {
    let root = std::env::temp_dir().join(format!(
        "vtest-adapter-go-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    let _ = fs::remove_dir_all(&root);
    for (path, bytes) in files {
        let full = root.join(path);
        fs::create_dir_all(full.parent().expect("parent")).expect("create fixture directory");
        fs::write(full, bytes).expect("write fixture file");
    }
    Project { root }
}

pub(crate) fn execution(
    module: Option<&str>,
    suite: Option<&str>,
    selector: &str,
) -> ExecutionDescriptor {
    ExecutionDescriptor {
        adapter: AdapterId::new("go"),
        project: module.map(str::to_owned),
        suite: suite.map(|name| TestSuite {
            kind: "package".to_owned(),
            name: Some(name.to_owned()),
        }),
        selector: selector.to_owned(),
    }
}

fn discover(project: &Project) -> DiscoveryOutcome {
    adapter()
        .discover(
            &project.root,
            "fallback",
            &AdapterScanConfig {
                include_paths: vec![PathBuf::from(".")],
            },
        )
        .expect("discovery succeeds")
}

fn declared(id: &str, name: &str) -> String {
    format!(
        "// @vtest.id {id}\n// @vtest.covers VO-GO\n// @vtest.intent {name} runs\nfunc {name}(t *testing.T) {{}}\n"
    )
}

fn test_file(body: &str) -> Vec<u8> {
    format!("package app\n\nimport \"testing\"\n\n{body}").into_bytes()
}

fn command(
    project: &Project,
    execution: &ExecutionDescriptor,
) -> Result<RunnerCommand, TestRunnerError> {
    GoTestRunner::new().command(&project.root, execution, false, None)
}

/// A real `go test -json` stream, as captured, for `selector`.
mod captured {
    pub const PASS: &str = r#"{"Time":"2026-11-21T17:47:05.1767099+09:00","Action":"start","Package":"app/app"}
{"Time":"2026-11-21T17:47:05.6187328+09:00","Action":"run","Package":"app/app","Test":"TestPass"}
{"Time":"2026-11-21T17:47:05.6187328+09:00","Action":"output","Package":"app/app","Test":"TestPass","Output":"=== RUN   TestPass\n"}
{"Time":"2026-11-21T17:47:05.6192564+09:00","Action":"output","Package":"app/app","Test":"TestPass","Output":"--- PASS: TestPass (0.00s)\n"}
{"Time":"2026-11-21T17:47:05.6192564+09:00","Action":"pass","Package":"app/app","Test":"TestPass","Elapsed":0}
{"Time":"2026-11-21T17:47:05.6192564+09:00","Action":"output","Package":"app/app","Output":"PASS\n"}
{"Time":"2026-11-21T17:47:05.6446788+09:00","Action":"output","Package":"app/app","Output":"ok  \tapp/app\t0.468s\n"}
{"Time":"2026-11-21T17:47:05.6446788+09:00","Action":"pass","Package":"app/app","Elapsed":0.468}
"#;
    pub const FAIL: &str = r#"{"Time":"2026-11-21T17:47:06.1869885+09:00","Action":"start","Package":"app/app"}
{"Time":"2026-11-21T17:47:06.6550121+09:00","Action":"run","Package":"app/app","Test":"TestFail"}
{"Time":"2026-11-21T17:47:06.6550121+09:00","Action":"output","Package":"app/app","Test":"TestFail","Output":"    calc_test.go:14: boom\n"}
{"Time":"2026-11-21T17:47:06.6550121+09:00","Action":"fail","Package":"app/app","Test":"TestFail","Elapsed":0}
{"Time":"2026-11-21T17:47:06.6550121+09:00","Action":"output","Package":"app/app","Output":"FAIL\n"}
{"Time":"2026-11-21T17:47:06.6807659+09:00","Action":"fail","Package":"app/app","Elapsed":0.494}
"#;
    pub const SKIP: &str = r#"{"Time":"2026-11-21T17:47:07.2180663+09:00","Action":"start","Package":"app/app"}
{"Time":"2026-11-21T17:47:07.680209+09:00","Action":"run","Package":"app/app","Test":"TestSkip"}
{"Time":"2026-11-21T17:47:07.6807686+09:00","Action":"skip","Package":"app/app","Test":"TestSkip","Elapsed":0}
{"Time":"2026-11-21T17:47:07.7058892+09:00","Action":"pass","Package":"app/app","Elapsed":0.488}
"#;
    /// `TestSub` runs two subtests, the second of which fails.
    pub const SUBTESTS: &str = r#"{"Time":"2026-11-21T17:47:08.293497+09:00","Action":"start","Package":"app/app"}
{"Time":"2026-11-21T17:47:08.725034+09:00","Action":"run","Package":"app/app","Test":"TestSub"}
{"Time":"2026-11-21T17:47:08.725034+09:00","Action":"run","Package":"app/app","Test":"TestSub/a"}
{"Time":"2026-11-21T17:47:08.725034+09:00","Action":"pass","Package":"app/app","Test":"TestSub/a","Elapsed":0}
{"Time":"2026-11-21T17:47:08.725034+09:00","Action":"run","Package":"app/app","Test":"TestSub/b"}
{"Time":"2026-11-21T17:47:08.725034+09:00","Action":"fail","Package":"app/app","Test":"TestSub/b","Elapsed":0}
{"Time":"2026-11-21T17:47:08.725034+09:00","Action":"fail","Package":"app/app","Test":"TestSub","Elapsed":0}
{"Time":"2026-11-21T17:47:08.7504387+09:00","Action":"fail","Package":"app/app","Elapsed":0.457}
"#;
    /// A selector that matches no function.
    pub const NONE: &str = r#"{"Time":"2026-11-21T17:47:09.2671905+09:00","Action":"start","Package":"app/app"}
{"Time":"2026-11-21T17:47:09.7179771+09:00","Action":"output","Package":"app/app","Output":"testing: warning: no tests to run\n"}
{"Time":"2026-11-21T17:47:09.7424648+09:00","Action":"output","Package":"app/app","Output":"ok  \tapp/app\t0.475s [no tests to run]\n"}
{"Time":"2026-11-21T17:47:09.7424648+09:00","Action":"pass","Package":"app/app","Elapsed":0.475}
"#;
    /// A package that does not compile (go 1.26.4 prints both the build-fail
    /// event and a package `fail` with `FailedBuild`).
    pub const BUILD_FAILED: &str = r##"{"ImportPath":"app/bfail [app/bfail.test]","Action":"build-output","Output":"# app/bfail [app/bfail.test]\n"}
{"ImportPath":"app/bfail [app/bfail.test]","Action":"build-output","Output":"bfail\\a_test.go:5:40: cannot use \"s\" (untyped string constant) as int value in variable declaration\n"}
{"ImportPath":"app/bfail [app/bfail.test]","Action":"build-fail"}
{"Time":"2026-11-21T17:47:25.8209874+09:00","Action":"start","Package":"app/bfail"}
{"Time":"2026-11-21T17:47:25.8215054+09:00","Action":"output","Package":"app/bfail","Output":"FAIL\tapp/bfail [build failed]\n"}
{"Time":"2026-11-21T17:47:25.8215054+09:00","Action":"fail","Package":"app/bfail","Elapsed":0.001,"FailedBuild":"app/bfail [app/bfail.test]"}
"##;
    /// Two Tests named `TestDup` in `package dupx` and `package dupx_test`.
    pub const SAME_NAME_TWICE: &str = r#"{"Time":"2026-11-21T17:47:26.8950289+09:00","Action":"start","Package":"app/dupx"}
{"Time":"2026-11-21T17:47:27.3495416+09:00","Action":"run","Package":"app/dupx","Test":"TestDup"}
{"Time":"2026-11-21T17:47:27.3500859+09:00","Action":"pass","Package":"app/dupx","Test":"TestDup","Elapsed":0}
{"Time":"2026-11-21T17:47:27.3500859+09:00","Action":"run","Package":"app/dupx","Test":"TestDup"}
{"Time":"2026-11-21T17:47:27.3500859+09:00","Action":"fail","Package":"app/dupx","Test":"TestDup","Elapsed":0}
{"Time":"2026-11-21T17:47:27.374381+09:00","Action":"fail","Package":"app/dupx","Elapsed":0.479}
"#;
}

/// DES-350, DES-621: the adapter declares discovery and the runner, and no
/// capability it does not implement; the registry resolves the runner.
/// @vtest.id TEST-ADAPTER-GO-DECLARES-DISCOVERY-AND-RUNNER
/// @vtest.covers VO-ADAPTER-LEXICAL-DECLARES-DISCOVERY-ONLY, VO-ADAPTER-GO-REGISTERS-WITHOUT-CORE-CHANGES
/// @vtest.target crates/vtest-adapter-go/src/lib.rs::adapter
/// @vtest.intent the go adapter declares its id, its language, its config namespace, source_discovery and test_runner and neither coverage nor static analysis, registers, and resolves its runner by the id go
#[test]
fn the_go_adapter_declares_discovery_and_the_runner_only() {
    let descriptor = adapter().descriptor();
    assert_eq!(descriptor.id, "go");
    assert_eq!(descriptor.languages, vec!["go".to_owned()]);
    assert_eq!(descriptor.config_namespace, "go");
    assert_eq!(
        descriptor.capabilities,
        vec![Capability::SourceDiscovery, Capability::TestRunner]
    );
    let mut registry = AdapterRegistry::new();
    registry
        .register(Box::new(adapter()))
        .expect("a consistent declaration registers");
    assert!(registry.source_discovery("go").is_some());
    assert_eq!(
        registry.test_runner("go").map(|runner| runner.id()),
        Some("go")
    );
    assert!(registry.coverage("go").is_none());
    assert!(registry.static_analysis("go").is_none());
}

/// DES-643, DES-644, DES-645: a Test in the module root's package.
/// @vtest.id TEST-ADAPTER-GO-COORDINATES-OF-A-ROOT-MODULE-TEST
/// @vtest.covers VO-ADAPTER-GO-EXECUTION-PROJECT-IS-THE-MODULE, VO-ADAPTER-GO-EXECUTION-SUITE-IS-THE-PACKAGE, VO-ADAPTER-GO-EXECUTION-SELECTOR-IS-THE-TEST-NAME
/// @vtest.target crates/vtest-adapter-go/src/module.rs::execution_for
/// @vtest.intent a declared Go Test whose go.mod is at the project root gets the project dot, the suite kind package named by its directory below the module root, and its function name as selector
#[test]
fn a_test_gets_the_module_the_package_and_its_name_as_coordinates() {
    let fixture = project(&[
        ("go.mod", b"module app\n\ngo 1.21\n"),
        (
            "app/calc_test.go",
            &test_file(&declared("TEST-GO-A", "TestAdd")),
        ),
        (
            "root_test.go",
            &test_file(&declared("TEST-GO-B", "TestRoot")),
        ),
    ]);
    let outcome = discover(&fixture);
    let by_id = |id: &str| {
        outcome
            .tests
            .iter()
            .find(|test| test.id.as_str() == id)
            .expect("the Test is discovered")
    };
    let nested = &by_id("TEST-GO-A").execution;
    assert_eq!(nested.adapter.as_str(), "go");
    assert_eq!(nested.project.as_deref(), Some("."));
    let suite = nested.suite.as_ref().expect("a suite");
    assert_eq!(
        (suite.kind.as_str(), suite.name.as_deref()),
        ("package", Some("app"))
    );
    assert_eq!(nested.selector, "TestAdd");

    let root = &by_id("TEST-GO-B").execution;
    assert_eq!(root.project.as_deref(), Some("."));
    assert_eq!(
        root.suite.as_ref().and_then(|suite| suite.name.as_deref()),
        Some(".")
    );
    assert_eq!(root.selector, "TestRoot");
}

/// DES-684: the module is the nearest go.mod, not the outermost.
/// @vtest.id TEST-ADAPTER-GO-MODULE-IS-THE-NEAREST-GO-MOD
/// @vtest.covers VO-ADAPTER-GO-MODULE-IS-THE-NEAREST-GO-MOD, VO-ADAPTER-GO-EXECUTION-PROJECT-IS-THE-MODULE
/// @vtest.target crates/vtest-adapter-go/src/module.rs::execution_for
/// @vtest.intent a Test below a nested go.mod belongs to that module, named by its path from the project root, and its suite is the directory below the nested module
#[test]
fn the_module_is_the_nearest_go_mod_from_the_test_file_up_to_the_project_root() {
    let fixture = project(&[
        ("go.mod", b"module outer\n\ngo 1.21\n"),
        ("services/api/go.mod", b"module api\n\ngo 1.21\n"),
        (
            "services/api/handlers/h_test.go",
            &test_file(&declared("TEST-GO-NEST", "TestHandler")),
        ),
        (
            "services/other/o_test.go",
            &test_file(&declared("TEST-GO-OUTER", "TestOther")),
        ),
    ]);
    let outcome = discover(&fixture);
    let execution = |id: &str| {
        outcome
            .tests
            .iter()
            .find(|test| test.id.as_str() == id)
            .expect("the Test is discovered")
            .execution
            .clone()
    };
    let nested = execution("TEST-GO-NEST");
    assert_eq!(nested.project.as_deref(), Some("services/api"));
    assert_eq!(
        nested.suite.and_then(|suite| suite.name).as_deref(),
        Some("handlers")
    );
    let outer = execution("TEST-GO-OUTER");
    assert_eq!(outer.project.as_deref(), Some("."));
    assert_eq!(
        outer.suite.and_then(|suite| suite.name).as_deref(),
        Some("services/other")
    );
}

/// DES-687: no go.mod up to the project root leaves the Test without a module.
/// @vtest.id TEST-ADAPTER-GO-NO-GO-MOD-MEANS-NO-MODULE
/// @vtest.covers VO-ADAPTER-GO-TEST-WITHOUT-GO-MOD-IS-NOT-EXECUTED
/// @vtest.target crates/vtest-adapter-go/src/module.rs::execution_for
/// @vtest.intent a Go Test with no go.mod between its file and the project root is discovered with neither project nor suite, and the runner refuses it as a Test construct it cannot execute
#[test]
fn a_test_with_no_go_mod_has_no_module_and_the_runner_refuses_it() {
    let fixture = project(&[(
        "lonely/l_test.go",
        &test_file(&declared("TEST-GO-LONELY", "TestLonely")),
    )]);
    let outcome = discover(&fixture);
    let execution = &outcome.tests[0].execution;
    assert_eq!(
        (execution.project.clone(), execution.suite.clone()),
        (None, None)
    );
    assert_eq!(execution.selector, "TestLonely");
    match command(&fixture, execution) {
        Err(error @ TestRunnerError::NotExecutable { .. }) => {
            assert!(error.refuses_one_test());
            assert!(error.to_string().contains("DES-687"), "{error}");
        }
        other => panic!("expected NotExecutable, got {other:?}"),
    }
}

/// DES-647, DES-648, DS-1905, DS-1906.
/// @vtest.id TEST-ADAPTER-GO-COMMAND-IS-GO-TEST-ONE-NAME-NO-CACHE
/// @vtest.covers VO-ADAPTER-GO-RUNNER-KIND-IS-GO-TEST, VO-ADAPTER-GO-RUNS-ONE-TEST-PER-INVOCATION, VO-ADAPTER-GO-SELECTS-ONE-TEST-BY-EXACT-NAME, VO-ADAPTER-GO-RUNS-WITHOUT-THE-RESULT-CACHE, VO-ADAPTER-GO-FAST-MODE-HAS-NO-COVERAGE
/// @vtest.target crates/vtest-adapter-go/src/runner.rs::invocation_args
/// @vtest.intent the go command runs go test for the one package, selects the function by the exact-name pattern, turns the result cache off, asks for json, starts in the module root, is of kind go-test, and is the same whether or not coverage is asked for
#[test]
fn the_command_is_go_test_for_one_package_by_exact_name_without_the_cache() {
    let fixture = project(&[
        ("services/api/go.mod", b"module api\n\ngo 1.21\n"),
        (
            "services/api/h/h_test.go",
            &test_file(&declared("TEST-GO-CMD", "TestCmd")),
        ),
    ]);
    let execution = execution(Some("services/api"), Some("h"), "TestCmd");
    let spec = command(&fixture, &execution).expect("a command");
    assert_eq!(spec.program, "go");
    assert_eq!(
        spec.args,
        ["test", "./h", "-run", "^TestCmd$", "-count=1", "-json"]
    );
    assert_eq!(
        spec.command_line,
        "go test ./h -run ^TestCmd$ -count=1 -json"
    );
    assert_eq!(spec.current_dir, fixture.root.join("services/api"));
    assert_eq!(spec.runner_kind, "go-test");
    assert!(spec.env.is_empty());
    // No coverage capability is declared, so the flag is never set; if it were,
    // the command would not change.
    let with_coverage = GoTestRunner::new()
        .command(&fixture.root, &execution, true, Some(Path::new("c.out")))
        .expect("a command");
    assert_eq!(with_coverage, spec);
    // The module root as a package is `./.`.
    let root_package = coordinates(&self::execution(Some("."), Some("."), "TestX"))
        .map(|c| invocation_args(&c))
        .expect("coordinates");
    assert_eq!(root_package[1], "./.");
}

/// An execution descriptor that is not the go adapter's stops the run
/// (E-ADAPTER-003), unlike the refusal of one Test.
/// @vtest.id TEST-ADAPTER-GO-FOREIGN-DESCRIPTOR-IS-NOT-A-REFUSAL
/// @vtest.covers VO-ADAPTER-GO-EXECUTION-SUITE-IS-THE-PACKAGE, VO-EXEC-RUN-TESTS-DESCRIPTOR-MISMATCH-E-ADAPTER-003
/// @vtest.target crates/vtest-adapter-go/src/runner.rs::coordinates
/// @vtest.intent a descriptor with a project and no suite, a suite of another kind, or a package suite with no name is an error that stops the run, not a refusal of one Test
#[test]
fn a_descriptor_of_another_shape_is_an_error_not_a_refusal() {
    let fixture = project(&[("go.mod", b"module app\n")]);
    let mut no_suite = execution(Some("."), None, "TestX");
    no_suite.suite = None;
    let mut library = execution(Some("."), Some("."), "TestX");
    library.suite.as_mut().expect("suite").kind = "lib".to_owned();
    let mut unnamed = execution(Some("."), Some("."), "TestX");
    unnamed.suite.as_mut().expect("suite").name = None;
    for (descriptor, expected) in [
        (no_suite, TestRunnerError::MissingSuite),
        (
            library,
            TestRunnerError::UnsupportedSuiteKind {
                kind: "lib".to_owned(),
            },
        ),
        (
            unnamed,
            TestRunnerError::MissingSuiteName {
                kind: "package".to_owned(),
            },
        ),
    ] {
        let error = command(&fixture, &descriptor).expect_err("a descriptor error");
        assert_eq!(error, expected);
        assert!(!error.refuses_one_test());
    }
}

fn refusal(files: &[(&str, &[u8])], selector: &str) -> Option<TestRunnerError> {
    let mut all = vec![("go.mod", &b"module app\n\ngo 1.21\n"[..])];
    all.extend_from_slice(files);
    let fixture = project(&all);
    command(&fixture, &execution(Some("."), Some("app"), selector)).err()
}

/// DS-1930, DS-2011: a name two Test constructs share selects both.
/// @vtest.id TEST-ADAPTER-GO-SAME-NAME-IN-ANY-TEST-FILE-IS-NOT-UNIQUE
/// @vtest.covers VO-ADAPTER-GO-SAME-NAME-TESTS-RECORD-NO-EVIDENCE, VO-ADAPTER-GO-SAME-NAME-COUNT-READS-EVERY-TEST-FILE
/// @vtest.target crates/vtest-adapter-go/src/runner.rs::check_selection
/// @vtest.intent two Test constructs with one name in one package directory make the Test not uniquely selectable, whether they are in package x and x_test, or one is in a file a build constraint leaves out, and every _test.go counts even when discovery would not read it
#[test]
fn a_name_that_two_test_constructs_share_is_not_uniquely_selectable() {
    let plain = test_file("func TestDup(t *testing.T) {}\n");
    let external = b"package app_test\n\nimport \"testing\"\n\nfunc TestDup(t *testing.T) {}\n";
    let constrained = format!(
        "//go:build ignore_me\n\n{}",
        String::from_utf8(plain.clone()).expect("utf-8")
    )
    .into_bytes();
    for (name, second) in [
        ("package x and x_test", &external[..]),
        ("a file a build constraint leaves out", &constrained[..]),
    ] {
        let files: [(&str, &[u8]); 2] = [("app/a_test.go", &plain), ("app/b_test.go", second)];
        match refusal(&files, "TestDup") {
            Some(error @ TestRunnerError::NotUniquelySelectable { .. }) => {
                assert!(error.refuses_one_test(), "{name}");
                let text = error.to_string();
                assert!(
                    text.contains("a_test.go") && text.contains("b_test.go"),
                    "{text}"
                );
            }
            other => panic!("{name}: expected NotUniquelySelectable, got {other:?}"),
        }
    }
    // A name of its own is uniquely selectable.
    assert_eq!(refusal(&[("app/a_test.go", &plain)], "TestDup"), None);
}

/// DS-2011: only `_test.go` files of the package directory whose names do not
/// start with `_` or `.` are counted.
/// @vtest.id TEST-ADAPTER-GO-ONLY-THE-FILES-GO-BUILDS-ARE-COUNTED
/// @vtest.covers VO-ADAPTER-GO-SAME-NAME-COUNT-READS-EVERY-TEST-FILE
/// @vtest.target crates/vtest-adapter-go/src/runner.rs::check_selection
/// @vtest.intent a function of the same name in a file whose name starts with an underscore or a dot, in a file not ending in _test.go, or in another directory is not counted
#[test]
fn files_the_go_command_never_builds_are_not_counted() {
    let one = test_file("func TestOnce(t *testing.T) {}\n");
    let same = test_file("func TestOnce(t *testing.T) {}\n");
    let files: [(&str, &[u8]); 5] = [
        ("app/a_test.go", &one),
        ("app/_hidden_test.go", &same),
        ("app/.dot_test.go", &same),
        ("app/helper.go", b"package app\n\nfunc TestOnce() {}\n"),
        ("app/sub/s_test.go", &same),
    ];
    assert_eq!(refusal(&files, "TestOnce"), None);
}

/// DS-2012: a file that cannot be read leaves the count unknown.
/// @vtest.id TEST-ADAPTER-GO-UNREADABLE-FILE-LEAVES-THE-COUNT-UNKNOWN
/// @vtest.covers VO-ADAPTER-GO-UNREADABLE-COUNTED-FILE-RECORDS-NO-EVIDENCE
/// @vtest.target crates/vtest-adapter-go/src/runner.rs::check_selection
/// @vtest.intent a _test.go file that is not UTF-8 text or whose syntax cannot be determined makes the selection undetermined, with no diagnostic code, and a duplicate that is already certain is still E-EXEC-005
#[test]
fn an_unreadable_test_file_leaves_the_selection_undetermined() {
    let good = test_file("func TestGood(t *testing.T) {}\n");
    let not_text: &[u8] = &[0xff, 0xfe, 0x00, b'x'];
    let unterminated: &[u8] = b"package app\n\n/* never closed\nfunc TestGood(t *testing.T) {}\n";
    for (name, bad) in [
        ("invalid UTF-8", not_text),
        ("unterminated comment", unterminated),
    ] {
        let files: [(&str, &[u8]); 2] = [("app/a_test.go", &good), ("app/b_test.go", bad)];
        match refusal(&files, "TestGood") {
            Some(error @ TestRunnerError::SelectionUndetermined { .. }) => {
                assert!(error.refuses_one_test(), "{name}");
                assert!(error.to_string().contains("b_test.go"), "{error}");
            }
            other => panic!("{name}: expected SelectionUndetermined, got {other:?}"),
        }
    }
    let twice: [(&str, &[u8]); 3] = [
        ("app/a_test.go", &good),
        ("app/c_test.go", &good),
        ("app/b_test.go", not_text),
    ];
    assert!(matches!(
        refusal(&twice, "TestGood"),
        Some(TestRunnerError::NotUniquelySelectable { .. })
    ));
    // The directory itself cannot be listed.
    assert!(matches!(
        refusal(&[], "TestGood"),
        Some(TestRunnerError::SelectionUndetermined { .. })
    ));
}

/// DS-1904, DES-685: the shape `go test` runs.
/// @vtest.id TEST-ADAPTER-GO-ONLY-THE-RUNNABLE-SHAPE-IS-EXECUTED
/// @vtest.covers VO-ADAPTER-GO-RUNNABLE-TEST-CONSTRUCT-SHAPE, VO-ADAPTER-GO-UNEXECUTABLE-TEST-RECORDS-NO-EVIDENCE
/// @vtest.target crates/vtest-adapter-go/src/runner.rs::check_selection
/// @vtest.intent a Test construct with type parameters, a second parameter, no parameter, or a result is refused as not executable, and one taking *testing.T, with the package under another name or ending in a comment, is run
#[test]
fn a_construct_of_the_wrong_shape_is_refused_as_not_executable() {
    for (name, source) in [
        (
            "type parameters",
            "func TestShape[T any](t *testing.T) {}\n",
        ),
        (
            "a second parameter",
            "func TestShape(t *testing.T, n int) {}\n",
        ),
        ("no parameter", "func TestShape() {}\n"),
        (
            "a result",
            "func TestShape(t *testing.T) int { return 1 }\n",
        ),
        (
            "a result in parentheses",
            "func TestShape(t *testing.T) (err error) { return }\n",
        ),
        ("the wrong type", "func TestShape(t *testing.B) {}\n"),
    ] {
        let files: [(&str, &[u8]); 1] = [("app/s_test.go", &test_file(source))];
        match refusal(&files, "TestShape") {
            Some(error @ TestRunnerError::NotExecutable { .. }) => {
                assert!(error.refuses_one_test(), "{name}");
                assert!(error.to_string().contains("DS-1904"), "{name}: {error}");
            }
            other => panic!("{name}: expected NotExecutable, got {other:?}"),
        }
    }
    for source in [
        "func TestShape(t *testing.T) {}\n",
        "func TestShape(tt *testing.T) { /* done */ }\n",
        "func TestShape(*testing.T) {}\n",
    ] {
        let files: [(&str, &[u8]); 1] = [("app/s_test.go", &test_file(source))];
        assert_eq!(refusal(&files, "TestShape"), None, "{source}");
    }
}

/// DES-649, DES-650, DS-1907, DS-1908.
/// @vtest.id TEST-ADAPTER-GO-RESULT-IS-THE-EXACT-NAME-EVENT
/// @vtest.covers VO-ADAPTER-GO-RESULT-IS-THE-EXACT-NAME-EVENT, VO-ADAPTER-GO-ACTION-MEANS-PASS-FAIL-OR-SKIPPED, VO-ADAPTER-GO-RESULT-IS-READ-FROM-GO-TEST, VO-ADAPTER-GO-SKIP-IS-NOT-EXECUTED
/// @vtest.target crates/vtest-adapter-go/src/runner.rs::interpret
/// @vtest.intent in go test json output the one pass, fail or skip event of the exact test name is PASS, FAIL or not executed, a subtest's events and the package's events are not the Test's result, and the parent of a failed subtest is FAIL
#[test]
fn the_result_is_the_one_event_of_the_exact_name() {
    assert_eq!(
        interpret(captured::PASS, "TestPass", "app"),
        RunnerTestResult::Pass
    );
    assert_eq!(
        interpret(captured::FAIL, "TestFail", "app"),
        RunnerTestResult::Fail
    );
    assert_eq!(
        interpret(captured::SKIP, "TestSkip", "app"),
        RunnerTestResult::Ignored
    );
    // The parent's own event decides; the passing subtest and the package
    // event do not.
    assert_eq!(
        interpret(captured::SUBTESTS, "TestSub", "app"),
        RunnerTestResult::Fail
    );
    // A name that only a subtest or another Test has is not the Test's result.
    assert_eq!(
        interpret(captured::SUBTESTS, "TestSub/a", "app"),
        RunnerTestResult::Pass
    );
    assert_eq!(
        interpret(captured::PASS, "TestPas", "app"),
        RunnerTestResult::NoResult
    );
    assert_eq!(
        interpret(captured::PASS, "TestPasss", "app"),
        RunnerTestResult::NoResult
    );
    // The package line passed, the named Test is not in the stream.
    assert_eq!(
        interpret(captured::FAIL, "TestPass", "app"),
        RunnerTestResult::NoResult
    );
}

/// DES-651, DS-1909, DS-1930, DS-1931.
/// @vtest.id TEST-ADAPTER-GO-NO-RESULT-AND-SEVERAL-RESULTS
/// @vtest.covers VO-ADAPTER-GO-NO-OR-SEVERAL-RESULT-EVENTS, VO-ADAPTER-GO-NO-RESULT-IS-E-EXEC-002, VO-ADAPTER-GO-SAME-NAME-TESTS-RECORD-NO-EVIDENCE
/// @vtest.target crates/vtest-adapter-go/src/runner.rs::interpret
/// @vtest.intent go test json output with no result event for the name, though the package passed, is no result, and with two result events for the name is an ambiguous result
#[test]
fn no_result_and_several_results_are_told_apart() {
    assert_eq!(
        interpret(captured::NONE, "TestNothing", "app"),
        RunnerTestResult::NoResult
    );
    assert_eq!(
        interpret(captured::SAME_NAME_TWICE, "TestDup", "dupx"),
        RunnerTestResult::Ambiguous
    );
}

/// DES-677, DS-1910.
/// @vtest.id TEST-ADAPTER-GO-BUILD-FAILURE-OUTRANKS-THE-EVENTS
/// @vtest.covers VO-ADAPTER-GO-BUILD-FAILURE-OUTRANKS-RESULT-EVENTS, VO-ADAPTER-GO-BUILD-FAILURE-IS-E-EXEC-001
/// @vtest.target crates/vtest-adapter-go/src/runner.rs::interpret
/// @vtest.intent a build-fail event, or a package fail event that carries FailedBuild, is a build failure, whatever result events for the name the stream also holds, and a package fail without FailedBuild is not
#[test]
fn a_build_failure_outranks_every_other_event() {
    assert_eq!(
        interpret(captured::BUILD_FAILED, "TestB", "bfail"),
        RunnerTestResult::BuildFailed
    );
    // Both markers are enough alone, and a Test's pass event does not outrank
    // them.
    let build_fail_only = r#"{"ImportPath":"app [app.test]","Action":"build-fail"}"#;
    let failed_build_only =
        r#"{"Action":"fail","Package":"app","Elapsed":0,"FailedBuild":"app [app.test]"}"#;
    let pass = r#"{"Action":"pass","Package":"app","Test":"TestB","Elapsed":0}"#;
    for stream in [build_fail_only, failed_build_only] {
        assert_eq!(
            interpret(stream, "TestB", "."),
            RunnerTestResult::BuildFailed
        );
        assert_eq!(
            interpret(&format!("{pass}\n{stream}\n"), "TestB", "."),
            RunnerTestResult::BuildFailed
        );
        assert_eq!(
            interpret(&format!("{stream}\n{pass}\n"), "TestB", "."),
            RunnerTestResult::BuildFailed
        );
    }
    // A package `fail` of an ordinary failing Test has no FailedBuild.
    assert_eq!(
        interpret(captured::FAIL, "TestFail", "app"),
        RunnerTestResult::Fail
    );
}

/// Output that is not `go test -json` leaves the run unclassified, so no
/// Evidence is recorded for it.
/// @vtest.id TEST-ADAPTER-GO-OUTPUT-THAT-IS-NOT-JSON-IS-NOT-CLASSIFIED
/// @vtest.covers VO-ADAPTER-GO-RESULT-IS-READ-FROM-GO-TEST
/// @vtest.target crates/vtest-adapter-go/src/runner.rs::interpret
/// @vtest.intent a line that is not a json object anywhere in the output makes the result unknown even when the name has a pass event
#[test]
fn output_that_is_not_json_is_not_classified() {
    let noisy = format!("go: downloading example.com/x v1.0.0\n{}", captured::PASS);
    assert_eq!(
        interpret(&noisy, "TestPass", "app"),
        RunnerTestResult::Unknown
    );
    assert_eq!(
        interpret("[1,2]\n", "TestPass", "app"),
        RunnerTestResult::Unknown
    );
    assert_eq!(interpret("", "TestPass", "app"), RunnerTestResult::NoResult);
}

/// DS-1905: a directory whose name holds `...` cannot be named by a literal
/// package pattern, because go reads `...` as a wildcard.
/// @vtest.id TEST-ADAPTER-GO-WILDCARD-IN-A-PACKAGE-DIRECTORY-IS-NOT-RUN
/// @vtest.covers VO-ADAPTER-GO-SELECTS-ONE-TEST-BY-EXACT-NAME, VO-ADAPTER-GO-UNEXECUTABLE-TEST-RECORDS-NO-EVIDENCE
/// @vtest.target crates/vtest-adapter-go/src/runner.rs::coordinates
/// @vtest.intent a Test whose package directory has three dots in its name is refused as not executable, so no go test pattern that matches other directories is started, while a directory with one or two dots is run
#[test]
fn a_package_directory_with_a_wildcard_in_its_name_is_not_run() {
    let fixture = project(&[
        ("go.mod", b"module app\n\ngo 1.21\n"),
        (
            "a...b/a_test.go",
            &test_file("func TestAdd(t *testing.T) {}\n"),
        ),
        (
            "a.b/a_test.go",
            &test_file("func TestAdd(t *testing.T) {}\n"),
        ),
        (
            "a..b/a_test.go",
            &test_file("func TestAdd(t *testing.T) {}\n"),
        ),
    ]);
    match command(&fixture, &execution(Some("."), Some("a...b"), "TestAdd")) {
        Err(error @ TestRunnerError::NotExecutable { .. }) => {
            assert!(error.refuses_one_test());
            assert!(error.to_string().contains("DS-1905"), "{error}");
        }
        other => panic!("expected NotExecutable, got {other:?}"),
    }
    for suite in ["a.b", "a..b"] {
        let spec = command(&fixture, &execution(Some("."), Some(suite), "TestAdd"))
            .expect("a command for a literal pattern");
        assert_eq!(spec.args[1], format!("./{suite}"));
    }
    // The module directory is the working directory, not a pattern.
    let nested = project(&[
        ("x...y/go.mod", b"module nested\n\ngo 1.21\n"),
        (
            "x...y/a_test.go",
            &test_file("func TestAdd(t *testing.T) {}\n"),
        ),
    ]);
    assert!(command(&nested, &execution(Some("x...y"), Some("."), "TestAdd")).is_ok());
}

/// DS-1905: only the events of the selected package are the Test's result.
/// @vtest.id TEST-ADAPTER-GO-EVENTS-OF-ANOTHER-PACKAGE-ARE-NOT-THE-RESULT
/// @vtest.covers VO-ADAPTER-GO-SELECTS-ONE-TEST-BY-EXACT-NAME, VO-ADAPTER-GO-RESULT-IS-THE-EXACT-NAME-EVENT
/// @vtest.target crates/vtest-adapter-go/src/runner.rs::interpret
/// @vtest.intent a pass event of the same Test name from package app/axb is no result for the Test of package a...b, and so is one from a package whose path only ends like the suite, or one with no Package, while the event of app/a...b is its PASS
#[test]
fn the_events_of_another_package_are_not_the_result() {
    let pass_of = |package: &str| {
        format!("{{\"Action\":\"pass\",\"Package\":\"{package}\",\"Test\":\"TestAdd\",\"Elapsed\":0}}\n")
    };
    assert_eq!(
        interpret(&pass_of("app/axb"), "TestAdd", "a...b"),
        RunnerTestResult::NoResult
    );
    assert_eq!(
        interpret(&pass_of("app/xa...b"), "TestAdd", "a...b"),
        RunnerTestResult::NoResult
    );
    assert_eq!(
        interpret(
            "{\"Action\":\"pass\",\"Test\":\"TestAdd\"}\n",
            "TestAdd",
            "a...b"
        ),
        RunnerTestResult::NoResult
    );
    assert_eq!(
        interpret(&pass_of("app/a...b"), "TestAdd", "a...b"),
        RunnerTestResult::Pass
    );
    // A nested package is the suite's own only with its whole path, and the
    // module root package (`.`) is the module path itself.
    assert_eq!(
        interpret(&pass_of("app/x/y"), "TestAdd", "x/y"),
        RunnerTestResult::Pass
    );
    assert_eq!(
        interpret(&pass_of("app/y"), "TestAdd", "x/y"),
        RunnerTestResult::NoResult
    );
    assert_eq!(
        interpret(&pass_of("app"), "TestAdd", "."),
        RunnerTestResult::Pass
    );
    // Another package's result next to the selected one's is not a second one.
    let both = format!("{}{}", pass_of("app/axb"), pass_of("app/a...b"));
    assert_eq!(interpret(&both, "TestAdd", "a...b"), RunnerTestResult::Pass);
}
