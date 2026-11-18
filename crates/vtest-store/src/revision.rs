//! The one place that asks git which commit the project is at.
//!
//! Evidence records the commit a Test ran at and verification compares it
//! with the current one (DS-820), so both must read HEAD the same way.

use std::{path::Path, process::Command};

/// The commit `HEAD` points at in the repository containing `root`, or
/// `None` when it cannot be read: `git` is missing, `root` is not inside a
/// repository, or the repository has no commit yet.
pub fn git_head_commit(root: &Path) -> Option<String> {
    Command::new("git")
        .current_dir(root)
        .args(["rev-parse", "HEAD"])
        .output()
        .ok()
        .filter(|output| output.status.success())
        .map(|output| String::from_utf8_lossy(&output.stdout).trim().to_owned())
        .filter(|value| !value.is_empty())
}
