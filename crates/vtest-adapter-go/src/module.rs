//! The Go module a Test belongs to (DES-684) and the execution coordinates
//! built from it (DES-643, DES-644, DES-645, DES-687).
//!
//! Responsibility: decide, from where a Test's file is, which directory `go
//! test` is started in and which package it is given.
//!
//! Boundary: this module only looks for `go.mod` files. It does not run `go`,
//! read a `go.mod`, or decide whether a Test can be executed; the runner does.

use std::path::Path;

use vtest_model::{AdapterId, ExecutionDescriptor, TestSuite};

use crate::{ADAPTER_ID, PACKAGE_SUITE};

/// The file that marks the root of a Go module.
const GO_MOD: &str = "go.mod";

/// The execution coordinates of the Test named `selector` in the project
/// relative file `file` (DES-645).
///
/// DES-684: the module is the nearest directory with a `go.mod`, from the
/// directory of the file up to the project root and no further. `project` is
/// that directory relative to the project root, `.` for the root itself
/// (DES-643), and the suite is the package (DES-644): the directory of the file
/// relative to the module root. A file with no `go.mod` above it has no module
/// to run in (DES-687), so it gets neither a project nor a suite and the runner
/// refuses it.
pub(crate) fn execution_for(root: &Path, file: &str, selector: &str) -> ExecutionDescriptor {
    let mut directory: Vec<&str> = file.split('/').collect();
    directory.pop();
    let module_depth = (0..=directory.len()).rev().find(|depth| {
        root.join(directory[..*depth].join("/"))
            .join(GO_MOD)
            .is_file()
    });
    let (project, suite) = match module_depth {
        Some(depth) => (
            Some(relative(&directory[..depth])),
            Some(TestSuite {
                kind: PACKAGE_SUITE.to_owned(),
                name: Some(relative(&directory[depth..])),
            }),
        ),
        None => (None, None),
    };
    ExecutionDescriptor {
        adapter: AdapterId::new(ADAPTER_ID),
        project,
        suite,
        selector: selector.to_owned(),
    }
}

/// `components` as a `/`-separated relative path, `.` when there are none.
fn relative(components: &[&str]) -> String {
    if components.is_empty() {
        ".".to_owned()
    } else {
        components.join("/")
    }
}
