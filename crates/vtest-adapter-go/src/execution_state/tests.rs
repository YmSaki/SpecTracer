//! Tests of the `go` Execution State. They start the real `go` command (to read
//! `go env`, the module file and the package list), so they fail with a message
//! when `go` is not on `PATH` instead of passing without having looked.
//!
//! Each reason a snapshot is incomplete is tested against the same project
//! without it, so a test cannot pass for another reason that also makes the
//! snapshot incomplete.

use std::{collections::BTreeMap, path::Path, process::Command};

use vtest_adapter_api::{ExecutionStateReader, TestRunnerAdapter};
use vtest_model::{ExecutionDescriptor, ExecutionStateDraft};

use super::*;
use crate::tests::{execution, project, Project};

const HEAD: &str = "0123456789abcdef0123456789abcdef01234567";
const GO_MOD: &[u8] = b"module app\n\ngo 1.21\n";
const CALC: &[u8] = b"package app\n\nfunc Add(a, b int) int { return a + b }\n";
const CALC_TEST: &[u8] =
    b"package app\n\nimport \"testing\"\n\nfunc TestCalc(t *testing.T) { _ = Add(1, 2) }\n";

fn require_go() {
    let ran = Command::new("go")
        .arg("version")
        .output()
        .ok()
        .filter(|output| output.status.success());
    assert!(
        ran.is_some(),
        "`go` must be on PATH: the tests of the go Execution State run the real Go toolchain"
    );
}

/// An environment that makes the settings of the machine irrelevant: no cgo, no
/// flags, no workspace, no toolchain switch.
fn env(extra: &[(&str, &str)]) -> BTreeMap<String, String> {
    [
        ("CGO_ENABLED", "0"),
        ("GOFLAGS", ""),
        ("GOWORK", "off"),
        ("GOTOOLCHAIN", "local"),
    ]
    .into_iter()
    .chain(extra.iter().copied())
    .map(|(name, value)| (name.to_owned(), value.to_owned()))
    .collect()
}

fn clean() -> Project {
    project(&[
        ("go.mod", GO_MOD),
        ("go.sum", b""),
        ("app/calc.go", CALC),
        ("app/calc_test.go", CALC_TEST),
    ])
}

fn here() -> ExecutionDescriptor {
    execution(Some("."), Some("app"), "TestCalc")
}

fn state(
    root: &Path,
    overlay: BTreeMap<String, String>,
    execution: &ExecutionDescriptor,
) -> (ExecutionStateDraft, Vec<String>) {
    snapshot_with_reasons(
        &GoTestRunner::with_environment(overlay),
        root,
        execution,
        Some(HEAD),
        RUNNER_KIND,
    )
    .expect("a snapshot")
}

fn assert_complete(
    root: &Path,
    overlay: BTreeMap<String, String>,
    execution: &ExecutionDescriptor,
) {
    let (draft, reasons) = state(root, overlay, execution);
    assert!(draft.complete && reasons.is_empty(), "{reasons:?}");
}

/// The snapshot is incomplete, for a reason that names `clause`.
fn assert_incomplete(
    root: &Path,
    overlay: BTreeMap<String, String>,
    execution: &ExecutionDescriptor,
    clause: &str,
) {
    let (draft, reasons) = state(root, overlay, execution);
    assert!(!draft.complete, "complete although {clause} applies");
    assert!(
        reasons.iter().any(|reason| reason.contains(clause)),
        "no reason names {clause}: {reasons:?}"
    );
}

/// DES-097, DES-623, DES-624, DES-370: a project with nothing outside its root
/// has a complete snapshot of the go schema, whose toolchain identity binds
/// what `go env` reports and whose invocation names no machine path.
/// @vtest.id TEST-ADAPTER-GO-STATE-OF-A-CLEAN-PROJECT-IS-COMPLETE
/// @vtest.covers VO-ADAPTER-GO-TOOLCHAIN-IDENTITY-IS-GO-ENV, VO-ADAPTER-GO-MANIFEST-IS-EVERY-FILE-OF-THE-PROJECT, VO-EXEC-STATE-CANONICAL-INVOCATION
/// @vtest.target crates/vtest-adapter-go/src/execution_state.rs::snapshot_with_reasons
/// @vtest.intent a project that reaches nothing outside its root has a complete go-execution-state snapshot whose toolchain identity lists GOVERSION, GOOS, GOARCH, the instruction-set setting, the cgo and compiler variables, GOFLAGS, GOEXPERIMENT, GOFIPS140, GODEBUG, GOTOOLCHAIN, GO111MODULE and GOWORK, and whose invocation holds no absolute path
#[test]
fn a_project_with_nothing_outside_its_root_has_a_complete_snapshot() {
    require_go();
    let fixture = clean();
    let (draft, reasons) = state(&fixture.root, env(&[]), &here());
    assert!(draft.complete && reasons.is_empty(), "{reasons:?}");
    assert_eq!(draft.schema(), "go-execution-state-v1");
    assert_eq!(draft.runner_kind, "go-test");
    assert_eq!(draft.head_revision.as_deref(), Some(HEAD));
    let names: Vec<&str> = draft
        .toolchain_identity
        .lines()
        .filter_map(|line| line.split_once('=').map(|(name, _)| name))
        .collect();
    let goarch = draft
        .toolchain_identity
        .lines()
        .find_map(|line| line.strip_prefix("GOARCH="))
        .expect("GOARCH is bound");
    let mut expected: Vec<&str> = BOUND_VARIABLES.to_vec();
    expected.extend(instruction_set_variable(goarch));
    expected.push("GOWORK");
    expected.sort_unstable();
    assert_eq!(names, expected);
    assert!(draft.toolchain_identity.contains("GOVERSION=go1."));
    assert!(draft.toolchain_identity.contains("CGO_ENABLED=0"));
    assert!(draft.toolchain_identity.contains("GOWORK=off"));
    let invocation = draft.invocation.to_string();
    assert!(
        !invocation.contains(&fixture.root.to_string_lossy().replace('\\', "\\\\"))
            && !invocation.contains(fixture.root.to_string_lossy().as_ref()),
        "{invocation}"
    );
    assert_eq!(draft.invocation["working_directory"], ".");
    assert_eq!(
        draft.invocation["args"],
        json!(["test", "./app", "-run", "^TestCalc$", "-count=1", "-json"])
    );
}

/// DES-623, DES-099: every regular file, byte for byte, in identity order,
/// except what is under `.git/` and `.verify/` at the root.
/// @vtest.id TEST-ADAPTER-GO-MANIFEST-LISTS-EVERY-FILE-BUT-GIT-AND-VERIFY
/// @vtest.covers VO-ADAPTER-GO-MANIFEST-IS-EVERY-FILE-OF-THE-PROJECT
/// @vtest.target crates/vtest-adapter-go/src/execution_state.rs::collect_manifest
/// @vtest.intent the manifest holds every regular file of the project with its exact bytes, including files outside any Go package and files Go ignores, sorted by path, and leaves out what is under .git and .verify at the root
#[test]
fn the_manifest_is_every_file_of_the_project_but_git_and_verify() {
    require_go();
    let binary: &[u8] = &[0, 255, 13, 10, 1];
    let fixture = project(&[
        ("go.mod", GO_MOD),
        ("go.sum", b""),
        ("app/calc.go", CALC),
        ("app/calc_test.go", CALC_TEST),
        ("app/testdata/golden.bin", binary),
        ("README.md", b"docs\n"),
        (".gitignore", b"*.log\n"),
        ("scripts/build.sh", b"#!/bin/sh\n"),
        (".git/config", b"[core]\n"),
        (".git/objects/ab/cdef", b"object"),
        (".verify/evidence/e.yaml", b"evidence"),
    ]);
    let (draft, reasons) = state(&fixture.root, env(&[]), &here());
    assert!(draft.complete, "{reasons:?}");
    let paths: Vec<&str> = draft
        .inputs
        .iter()
        .map(|input| input.root_relative_path.as_str())
        .collect();
    assert_eq!(
        paths,
        [
            ".gitignore",
            "README.md",
            "app/calc.go",
            "app/calc_test.go",
            "app/testdata/golden.bin",
            "go.mod",
            "go.sum",
            "scripts/build.sh"
        ]
    );
    let golden = draft
        .inputs
        .iter()
        .find(|input| input.root_relative_path == "app/testdata/golden.bin")
        .expect("the golden file is listed");
    assert_eq!(golden.bytes, binary);
    assert!(draft
        .inputs
        .iter()
        .all(|input| input.root_identity == "project-root" && input.kind == "file"));
}

/// DES-625, DS-474: dependencies are bound by go.sum, and any file of the
/// project, a helper outside the Test's package included, changes the subject.
/// @vtest.id TEST-ADAPTER-GO-STATE-CHANGES-WITH-GO-SUM-AND-WITH-ANY-FILE
/// @vtest.covers VO-ADAPTER-GO-STATE-BINDS-DEPENDENCIES-THROUGH-GO-SUM, VO-ADAPTER-GO-MANIFEST-IS-EVERY-FILE-OF-THE-PROJECT
/// @vtest.target crates/vtest-adapter-go/src/execution_state.rs::collect_manifest
/// @vtest.intent the snapshot of the same project is equal when read twice and differs after go.sum changes, after a helper file of another package changes, and after a file is added
#[test]
fn the_snapshot_changes_when_go_sum_or_any_file_changes() {
    require_go();
    let fixture = project(&[
        ("go.mod", GO_MOD),
        ("go.sum", b"example.com/x v1.0.0 h1:aaa=\n"),
        ("app/calc.go", CALC),
        ("app/calc_test.go", CALC_TEST),
        (
            "other/helper.go",
            b"package other\n\nfunc H() int { return 1 }\n",
        ),
    ]);
    let before = state(&fixture.root, env(&[]), &here()).0;
    assert_eq!(before, state(&fixture.root, env(&[]), &here()).0);
    for (path, bytes) in [
        ("go.sum", &b"example.com/x v1.0.0 h1:bbb=\n"[..]),
        (
            "other/helper.go",
            b"package other\n\nfunc H() int { return 2 }\n",
        ),
        ("added.txt", b"new"),
    ] {
        let changed = project(&[
            ("go.mod", GO_MOD),
            ("go.sum", b"example.com/x v1.0.0 h1:aaa=\n"),
            ("app/calc.go", CALC),
            ("app/calc_test.go", CALC_TEST),
            (
                "other/helper.go",
                b"package other\n\nfunc H() int { return 1 }\n",
            ),
            (path, bytes),
        ]);
        let after = state(&changed.root, env(&[]), &here()).0;
        assert_ne!(before.inputs, after.inputs, "{path}");
        assert!(after.complete, "{path}");
    }
}

/// DES-624: each bound `go env` value is part of the identity.
/// @vtest.id TEST-ADAPTER-GO-TOOLCHAIN-IDENTITY-FOLLOWS-EACH-BOUND-VALUE
/// @vtest.covers VO-ADAPTER-GO-TOOLCHAIN-IDENTITY-IS-GO-ENV
/// @vtest.target crates/vtest-adapter-go/src/execution_state.rs::GoTestRunner::bind_toolchain
/// @vtest.intent changing GOFLAGS, GOEXPERIMENT, GODEBUG, CGO_CFLAGS, CGO_LDFLAGS, CC, GOOS or GOARCH in the environment the go command sees changes the toolchain identity, and the same environment gives the same identity
#[test]
fn each_bound_go_env_value_changes_the_toolchain_identity() {
    require_go();
    let fixture = clean();
    let base = state(&fixture.root, env(&[]), &here()).0.toolchain_identity;
    assert_eq!(
        base,
        state(&fixture.root, env(&[]), &here()).0.toolchain_identity
    );
    for (name, value) in [
        ("GOFLAGS", "-tags=x"),
        ("GOEXPERIMENT", "loopvar"),
        ("GODEBUG", "panicnil=1"),
        ("CGO_CFLAGS", "-O3"),
        ("CGO_LDFLAGS", "-lm"),
        ("CC", "clang"),
        ("GOOS", "plan9"),
        ("GOARCH", "arm64"),
    ] {
        let changed = state(&fixture.root, env(&[(name, value)]), &here()).0;
        assert_ne!(changed.toolchain_identity, base, "{name}={value}");
        assert!(
            changed
                .toolchain_identity
                .contains(&format!("{name}={value}")),
            "{name}: {}",
            changed.toolchain_identity
        );
    }
    // GOARCH selects which instruction-set setting is bound.
    let arm64 = state(&fixture.root, env(&[("GOARCH", "arm64")]), &here()).0;
    assert!(arm64.toolchain_identity.contains("GOARM64="));
    assert!(!arm64.toolchain_identity.contains("GOAMD64="));
}

/// DES-626: the module is not inside the project root.
/// @vtest.id TEST-ADAPTER-GO-STATE-INCOMPLETE-WHEN-THE-MODULE-IS-OUTSIDE-THE-ROOT
/// @vtest.covers VO-ADAPTER-GO-STATE-INCOMPLETE-MODULE-OUTSIDE-ROOT
/// @vtest.target crates/vtest-adapter-go/src/execution_state.rs::GoTestRunner::collect_module
/// @vtest.intent a Test whose module directory is outside the project root has an incomplete snapshot, and the same Test in a module inside the root has a complete one
#[test]
fn a_module_outside_the_project_root_makes_the_snapshot_incomplete() {
    require_go();
    let fixture = project(&[
        ("proj/go.mod", GO_MOD),
        ("proj/app/a_test.go", CALC_TEST),
        ("proj/app/calc.go", CALC),
        ("elsewhere/go.mod", b"module elsewhere\n\ngo 1.21\n"),
        ("elsewhere/e/e_test.go", b"package e\n"),
    ]);
    let root = fixture.root.join("proj");
    assert_complete(&root, env(&[]), &here());
    assert_incomplete(
        &root,
        env(&[]),
        &execution(Some("../elsewhere"), Some("e"), "TestE"),
        "DES-626",
    );
}

/// DES-627: a replace with a local path outside the project root, in go.mod
/// and in go.work.
/// @vtest.id TEST-ADAPTER-GO-STATE-INCOMPLETE-WHEN-A-REPLACE-LEAVES-THE-ROOT
/// @vtest.covers VO-ADAPTER-GO-STATE-INCOMPLETE-REPLACE-OUTSIDE-ROOT
/// @vtest.target crates/vtest-adapter-go/src/execution_state.rs::check_local_replaces
/// @vtest.intent a go.mod or go.work replace whose local path is outside the project root makes the snapshot incomplete, one whose local path is inside it or that names a module version does not
#[test]
fn a_replace_to_a_local_path_outside_the_root_makes_the_snapshot_incomplete() {
    require_go();
    let with_replace = |go_mod: &str| {
        project(&[
            ("proj/go.mod", go_mod.as_bytes()),
            ("proj/app/calc.go", CALC),
            ("proj/app/calc_test.go", CALC_TEST),
            ("proj/vendored/v.go", b"package v\n"),
            ("elsewhere/go.mod", b"module example.com/x\n\ngo 1.21\n"),
        ])
    };
    let inside = with_replace("module app\n\ngo 1.21\n\nreplace example.com/x => ./vendored\nreplace example.com/y => example.com/z v1.0.0\n");
    assert_complete(&inside.root.join("proj"), env(&[]), &here());
    let outside = with_replace("module app\n\ngo 1.21\n\nreplace example.com/x => ../elsewhere\n");
    assert_incomplete(&outside.root.join("proj"), env(&[]), &here(), "DES-627");

    // The workspace file replaces too, and its replacements are read.
    let workspace = |replace: &str| {
        project(&[
            ("proj/go.mod", GO_MOD),
            (
                "proj/go.work",
                format!("go 1.21\n\nuse .\n\nreplace example.com/x => {replace}\n").as_bytes(),
            ),
            ("proj/app/calc.go", CALC),
            ("proj/app/calc_test.go", CALC_TEST),
            ("proj/vendored/v.go", b"package v\n"),
            ("elsewhere/go.mod", b"module example.com/x\n\ngo 1.21\n"),
        ])
    };
    let go_work = |fixture: &Project| {
        env(&[(
            "GOWORK",
            fixture
                .root
                .join("proj/go.work")
                .to_str()
                .expect("utf-8 path"),
        )])
    };
    let inside = workspace("./vendored");
    assert_complete(&inside.root.join("proj"), go_work(&inside), &here());
    let outside = workspace("../elsewhere");
    assert_incomplete(
        &outside.root.join("proj"),
        go_work(&outside),
        &here(),
        "DES-627",
    );
}

/// DES-628: a workspace file outside the root, or one that uses a directory
/// outside it.
/// @vtest.id TEST-ADAPTER-GO-STATE-INCOMPLETE-WHEN-THE-WORKSPACE-LEAVES-THE-ROOT
/// @vtest.covers VO-ADAPTER-GO-STATE-INCOMPLETE-GO-WORK-OUTSIDE-ROOT
/// @vtest.target crates/vtest-adapter-go/src/execution_state.rs::GoTestRunner::check_go_work
/// @vtest.intent a go.work outside the project root, or a go.work inside it that uses a directory outside it, makes the snapshot incomplete, and a go.work inside that uses only directories inside does not
#[test]
fn a_workspace_outside_the_root_makes_the_snapshot_incomplete() {
    require_go();
    let files = |work_inside: &str, work_outside: &str| {
        project(&[
            ("proj/go.mod", GO_MOD),
            ("proj/go.work", work_inside.as_bytes()),
            ("proj/app/calc.go", CALC),
            ("proj/app/calc_test.go", CALC_TEST),
            ("go.work", work_outside.as_bytes()),
            ("elsewhere/go.mod", b"module elsewhere\n\ngo 1.21\n"),
        ])
    };
    let work = |path: &Path| env(&[("GOWORK", path.to_str().expect("utf-8 path"))]);

    let fixture = files("go 1.21\n\nuse .\n", "go 1.21\n\nuse ./proj\n");
    let root = fixture.root.join("proj");
    assert_complete(&root, work(&root.join("go.work")), &here());
    assert_incomplete(
        &root,
        work(&fixture.root.join("go.work")),
        &here(),
        "DES-628",
    );

    let uses_outside = files("go 1.21\n\nuse (\n\t.\n\t../elsewhere\n)\n", "go 1.21\n");
    let root = uses_outside.root.join("proj");
    assert_incomplete(&root, work(&root.join("go.work")), &here(), "DES-628");
}

/// DES-629, DES-680: cgo in a package of the build that is not the standard
/// library, and cgo in the standard library, which the toolchain covers.
/// @vtest.id TEST-ADAPTER-GO-STATE-INCOMPLETE-WHEN-A-NON-STANDARD-PACKAGE-USES-CGO
/// @vtest.covers VO-ADAPTER-GO-STATE-INCOMPLETE-NON-STANDARD-CGO
/// @vtest.target crates/vtest-adapter-go/src/execution_state.rs::GoTestRunner::cgo_reason
/// @vtest.intent with cgo enabled, a package of the Test's build that is not in the standard library and has cgo files makes the snapshot incomplete, while the cgo of a standard library package the build imports does not
#[test]
fn cgo_in_a_non_standard_package_of_the_build_makes_the_snapshot_incomplete() {
    require_go();
    let fixture = project(&[
        ("go.mod", GO_MOD),
        ("app/calc.go", b"package app\n\nimport _ \"app/cg\"\n"),
        ("app/calc_test.go", CALC_TEST),
        (
            "cg/cg.go",
            b"package cg\n\n// int one(void) { return 1; }\nimport \"C\"\n\nfunc One() int { return int(C.one()) }\n",
        ),
        ("cg/plain.go", b"package cg\n"),
        (
            "std/std_test.go",
            b"package std\n\nimport (\n\t\"os/user\"\n\t\"testing\"\n)\n\nfunc TestStd(t *testing.T) { _, _ = user.Current() }\n",
        ),
    ]);
    // With cgo off the cg package has no cgo file in the build; `git` stands in
    // for a C compiler that answers --version, so DES-680 does not apply.
    assert_complete(&fixture.root, env(&[]), &here());
    let cgo = env(&[("CGO_ENABLED", "1"), ("CC", "git")]);
    assert_incomplete(&fixture.root, cgo.clone(), &here(), "DES-629");
    // The standard library's own cgo (os/user) is the toolchain's.
    assert_complete(
        &fixture.root,
        cgo,
        &execution(Some("."), Some("std"), "TestStd"),
    );
}

/// DES-680: the C compiler is part of the toolchain when cgo is on.
/// @vtest.id TEST-ADAPTER-GO-STATE-BINDS-THE-C-COMPILER-OF-A-CGO-BUILD
/// @vtest.covers VO-ADAPTER-GO-STATE-INCOMPLETE-CC-VERSION-UNAVAILABLE, VO-ADAPTER-GO-TOOLCHAIN-IDENTITY-IS-GO-ENV
/// @vtest.target crates/vtest-adapter-go/src/execution_state.rs::GoTestRunner::cc_version
/// @vtest.intent with CGO_ENABLED 1 the version output of CC is part of the toolchain identity, a CC that gives no version output makes the snapshot incomplete, and with CGO_ENABLED 0 CC is not run
#[test]
fn the_c_compiler_version_is_bound_when_cgo_is_on() {
    require_go();
    let fixture = clean();
    let with_cc = state(
        &fixture.root,
        env(&[("CGO_ENABLED", "1"), ("CC", "git")]),
        &here(),
    );
    assert!(with_cc.0.complete, "{:?}", with_cc.1);
    assert!(with_cc
        .0
        .toolchain_identity
        .contains("CC_VERSION=git version"));
    assert_incomplete(
        &fixture.root,
        env(&[("CGO_ENABLED", "1"), ("CC", "no-such-compiler-for-vtest")]),
        &here(),
        "DES-680",
    );
    // Off, the compiler is never run, so one that does not exist is no reason.
    assert_complete(
        &fixture.root,
        env(&[("CC", "no-such-compiler-for-vtest")]),
        &here(),
    );
}

/// DES-630: GOFLAGS that point outside the project root, and the flags that name
/// a program.
/// @vtest.id TEST-ADAPTER-GO-STATE-INCOMPLETE-WHEN-GOFLAGS-LEAVE-THE-ROOT-OR-WRAP-THE-RUN
/// @vtest.covers VO-ADAPTER-GO-STATE-INCOMPLETE-GOFLAGS-OUTSIDE-ROOT-OR-EXEC
/// @vtest.target crates/vtest-adapter-go/src/execution_state.rs::check_goflags
/// @vtest.intent GOFLAGS with -exec or -toolexec, or with -modfile, -overlay or -pgo naming a file outside the project root, makes the snapshot incomplete, and GOFLAGS with ordinary flags, -pgo=off, or a file inside the root does not
#[test]
fn goflags_that_leave_the_root_or_wrap_the_run_make_the_snapshot_incomplete() {
    require_go();
    let fixture = project(&[
        ("proj/go.mod", GO_MOD),
        ("proj/app/calc.go", CALC),
        ("proj/app/calc_test.go", CALC_TEST),
        ("proj/overlay.json", b"{}"),
        ("outside.json", b"{}"),
        ("outside.mod", GO_MOD),
    ]);
    let root = fixture.root.join("proj");
    for flags in [
        "-tags=x -count=1",
        "-pgo=off",
        "-overlay=overlay.json",
        "-ldflags=-s",
    ] {
        assert_complete(&root, env(&[("GOFLAGS", flags)]), &here());
    }
    for flags in [
        "-toolexec=echo",
        "-exec=echo",
        "-tags=x -exec=echo",
        "-overlay=../outside.json",
        "-modfile=../outside.mod",
        "-pgo=../default.pgo",
    ] {
        assert_incomplete(&root, env(&[("GOFLAGS", flags)]), &here(), "DES-630");
    }
}

/// DES-631: a build that is not a module build.
/// @vtest.id TEST-ADAPTER-GO-STATE-INCOMPLETE-IN-GOPATH-MODE
/// @vtest.covers VO-ADAPTER-GO-STATE-INCOMPLETE-GOPATH-MODE
/// @vtest.target crates/vtest-adapter-go/src/execution_state.rs::GoTestRunner::bind_toolchain
/// @vtest.intent with GO111MODULE off the build has no module file, so the snapshot is incomplete
#[test]
fn a_build_without_modules_makes_the_snapshot_incomplete() {
    require_go();
    let fixture = clean();
    assert_complete(&fixture.root, env(&[]), &here());
    assert_incomplete(
        &fixture.root,
        env(&[("GO111MODULE", "off")]),
        &here(),
        "DES-631",
    );
}

/// DES-632: a symbolic link out of the project root.
/// @vtest.id TEST-ADAPTER-GO-STATE-INCOMPLETE-WITH-A-LINK-OUT-OF-THE-ROOT
/// @vtest.covers VO-ADAPTER-GO-STATE-INCOMPLETE-SYMLINK-OUTSIDE-ROOT
/// @vtest.target crates/vtest-adapter-go/src/execution_state.rs::walk
/// @vtest.intent a symbolic link under the project root that points outside it, or that cannot be resolved, makes the snapshot incomplete, and a link that points inside does not
#[cfg(unix)]
#[test]
fn a_symbolic_link_out_of_the_root_makes_the_snapshot_incomplete() {
    use std::os::unix::fs::symlink;
    require_go();
    let fixture = project(&[
        ("proj/go.mod", GO_MOD),
        ("proj/app/calc.go", CALC),
        ("proj/app/calc_test.go", CALC_TEST),
        ("outside/secret.txt", b"x"),
    ]);
    let root = fixture.root.join("proj");
    symlink(root.join("app/calc.go"), root.join("inside-link")).expect("create a link");
    assert_complete(&root, env(&[]), &here());
    symlink(fixture.root.join("outside"), root.join("out-link")).expect("create a link");
    assert_incomplete(&root, env(&[]), &here(), "DES-632");
    fs::remove_file(root.join("out-link")).expect("remove the link");
    symlink(root.join("missing"), root.join("broken-link")).expect("create a link");
    assert_incomplete(&root, env(&[]), &here(), "DES-632");
}

/// DES-209, DES-097: without the HEAD revision, and when `go` cannot report
/// its environment, the snapshot is not complete.
/// @vtest.id TEST-ADAPTER-GO-STATE-INCOMPLETE-WITHOUT-REVISION-OR-TOOLCHAIN
/// @vtest.covers VO-EXEC-STATE-ABSENT-HEAD-COMMIT-INCOMPLETE, VO-ADAPTER-GO-TOOLCHAIN-IDENTITY-IS-GO-ENV
/// @vtest.target crates/vtest-adapter-go/src/execution_state.rs::GoTestRunner::draft
/// @vtest.intent a snapshot with no HEAD revision, or of a module directory that does not exist, is incomplete
#[test]
fn a_snapshot_without_a_revision_or_a_toolchain_is_incomplete() {
    require_go();
    let fixture = clean();
    let runner = GoTestRunner::with_environment(env(&[]));
    let (without_head, _) =
        snapshot_with_reasons(&runner, &fixture.root, &here(), None, RUNNER_KIND).expect("a draft");
    assert!(!without_head.complete);
    assert_eq!(without_head.head_revision, None);
    assert_incomplete(
        &fixture.root,
        env(&[]),
        &execution(Some("missing"), Some("app"), "TestCalc"),
        "DES-624",
    );
}

/// DES-372, DES-369: a verification reads the same snapshot a run recorded, for
/// every Test of a module, and only for the go runner kind.
/// @vtest.id TEST-ADAPTER-GO-READER-GIVES-THE-SNAPSHOT-OF-EACH-CALL
/// @vtest.covers VO-EXEC-STATE-REEVALUATION-REBUILDS-CURRENT-DTO
/// @vtest.target crates/vtest-adapter-go/src/execution_state.rs::GoExecutionStateReader::new
/// @vtest.intent the reader of one operation returns for each Test the draft a direct call returns, one per package for the cgo check, and nothing for another runner kind or a Test with no module
#[test]
fn the_reader_returns_what_a_direct_call_returns() {
    require_go();
    let fixture = project(&[
        ("go.mod", GO_MOD),
        ("app/calc.go", CALC),
        ("app/calc_test.go", CALC_TEST),
        ("app2/b_test.go", CALC_TEST),
    ]);
    let runner = GoTestRunner::with_environment(env(&[]));
    let reader = GoExecutionStateReader::new(&runner);
    for descriptor in [
        here(),
        execution(Some("."), Some("app"), "TestOther"),
        execution(Some("."), Some("app2"), "TestCalc"),
    ] {
        let direct = runner.execution_state(&fixture.root, &descriptor, Some(HEAD), RUNNER_KIND);
        let read = reader.execution_state(&fixture.root, &descriptor, Some(HEAD), RUNNER_KIND);
        assert!(direct.as_ref().is_some_and(|draft| draft.complete));
        assert_eq!(direct, read);
    }
    assert_eq!(
        reader.execution_state(&fixture.root, &here(), Some(HEAD), "cargo-test"),
        None
    );
    assert_eq!(
        reader.execution_state(
            &fixture.root,
            &execution(None, None, "TestCalc"),
            Some(HEAD),
            RUNNER_KIND
        ),
        None
    );
}

/// DES-371: what a Test writes into the project during its run shows as a
/// change between the snapshots taken around it.
/// @vtest.id TEST-ADAPTER-GO-STATE-SEES-A-FILE-A-RUN-WRITES
/// @vtest.covers VO-ADAPTER-GO-MANIFEST-IS-EVERY-FILE-OF-THE-PROJECT
/// @vtest.target crates/vtest-adapter-go/src/execution_state.rs::collect_manifest
/// @vtest.intent a file created under the project root between two snapshots, or changed in place, makes the two drafts differ
#[test]
fn a_file_written_between_two_snapshots_makes_them_differ() {
    require_go();
    let fixture = clean();
    let before = state(&fixture.root, env(&[]), &here()).0;
    fs::write(fixture.root.join("app/written-by-the-test.txt"), b"x").expect("write a file");
    let after = state(&fixture.root, env(&[]), &here()).0;
    assert_ne!(before, after);
    fs::write(fixture.root.join("app/calc.go"), b"package app\n").expect("change a file");
    assert_ne!(after, state(&fixture.root, env(&[]), &here()).0);
}

/// DES-369: the environment the command is started with is the environment the
/// snapshot binds.
/// @vtest.id TEST-ADAPTER-GO-COMMAND-ENV-AND-SNAPSHOT-AGREE
/// @vtest.covers VO-EXEC-STATE-RUNNER-ENVIRONMENT-MATCHES
/// @vtest.target crates/vtest-adapter-go/src/runner.rs::GoTestRunner::with_environment
/// @vtest.intent a runner built with an environment overlay returns a command whose env is that overlay, and the snapshot of the same Test records that overlay in its invocation and the overlaid GOFLAGS in its toolchain identity
#[test]
fn the_environment_of_the_command_is_the_environment_of_the_snapshot() {
    require_go();
    let fixture = clean();
    let overlay = env(&[("GOFLAGS", "-tags=bound")]);
    let runner = GoTestRunner::with_environment(overlay.clone());
    let command = runner
        .command(&fixture.root, &here(), false, None)
        .expect("a command");
    assert_eq!(command.env, overlay);
    let draft = runner
        .execution_state(&fixture.root, &here(), Some(HEAD), RUNNER_KIND)
        .expect("a snapshot");
    assert_eq!(draft.invocation["env"], json!(overlay));
    assert!(draft.toolchain_identity.contains("GOFLAGS=-tags=bound"));
}
