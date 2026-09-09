//! Execution State subject reconstruction (DES-097-101, DES-210-212).
//!
//! Shared by `vtest-exec` (writes a fresh Evidence record after a test run)
//! and `vtest-verify` (reconstructs the *current* subject to compare against
//! a recorded one, DS-818/DS-822) so both sides compute the identical hash
//! for the identical environment. Living here rather than in either crate
//! avoids duplicating the file-walk and hashing logic; `vtest-model` cannot
//! hold it (crate doc comment: "holds no filesystem access"), and this crate
//! already owns `.verify/`-adjacent filesystem work both callers depend on.
//!
//! DES-097 (verified present in `specification.json` this session):
//! "Execution State subject hashはdomain `vtest:execution-state:v1` を
//! 用い、adapter ID、snapshot schema ID / version、HEAD revision、runner
//! kindとcanonical invocation projection、toolchain identity、実行結果へ
//! 影響するadapter configのcanonical projection、および実行可能状態を
//! 変えうるrepository / local dependency入力の完全なmanifestを束縛する。"
//!
//! **Disclosed scope limits** (not silently narrowed):
//! - "adapter configのcanonical projection" is bound as an explicit empty
//!   marker in this slice: no adapter config knob in this repository's
//!   `config.yaml` currently affects `rust-cargo` execution results, so
//!   there is nothing yet to project. A future slice that adds such a knob
//!   must extend the field this function writes, not silently continue
//!   omitting it.
//! - DES-210's "全local path dependency root" is not walked as separate
//!   roots: this repository is a single Cargo workspace with every local
//!   path dependency nested under one filesystem root, so walking that one
//!   root already covers them. A project with local path dependencies
//!   outside the workspace root is out of this slice's scope.
//! - `stable_root_identity` (DES-100) is therefore always the fixed logical
//!   constant `"workspace-root"` rather than a per-root identity scheme —
//!   there is exactly one root to name in this slice's supported topology.

use std::{
    fs,
    path::{Component, Path, PathBuf},
    process::Command,
};

use syn::{visit::Visit, Expr, Lit, Macro};
use vtest_model::{
    encode_nested_fields, AdapterId, ExecutionState, FieldValue, SubjectDomain, SubjectHashInput,
};

/// DES-211: "`.git/`、`.verify/` のcanonical record / cache、Cargo target
/// directory等の生成物は実行入力から除外する。" Matched by bare directory
/// name at any depth — a coarser rule than the literal root-relative paths
/// the canon names, disclosed as an implementation approximation.
const EXCLUDED_DIR_NAMES: &[&str] = &[".git", ".verify", "target"];

/// The fixed logical root identity this slice's single-workspace-root
/// topology uses (DES-100's "stable root identity", see module doc).
const ROOT_IDENTITY: &str = "workspace-root";

/// Everything this crate can gather about the *current* environment without
/// itself invoking a test runner, handed to [`reconstruct_execution_state`].
/// Both `vtest-exec` (right after a run) and `vtest-verify` (at verify time,
/// never having run anything) construct one of these the same way.
pub struct ExecutionStateInputs<'a> {
    pub adapter: &'a AdapterId,
    /// DES-097 "snapshot schema ID / version" as one bound string (this
    /// slice does not further split id vs. version — no second schema
    /// version has ever existed to distinguish, disclosed).
    pub schema: &'a str,
    pub head_commit: Option<&'a str>,
    pub runner_kind: &'a str,
    /// DES-097 "canonical invocation projection" — the already-normalized
    /// command line string (`vtest-exec`'s own `command_string`/
    /// `llvm_cov_command_string`, or `vtest-verify`'s equivalent
    /// reconstruction from `Test.execution`).
    pub invocation: &'a str,
}

/// Reconstructs the current Execution State subject for `root`, or reports
/// it incomplete (DES-184/209: `complete: false`, `hash: None`) when any
/// input this slice can obtain is missing, or DES-212's escape-path check
/// cannot rule out an excluded area being read.
pub fn reconstruct_execution_state(
    root: &Path,
    inputs: ExecutionStateInputs<'_>,
) -> ExecutionState {
    let Some(head_commit) = inputs.head_commit else {
        return incomplete(inputs.schema);
    };
    let Some(toolchain) = toolchain_identity() else {
        return incomplete(inputs.schema);
    };
    if escape_risk(root).is_some() {
        // DES-212: "除外領域を…読み込む可能性を排除できない場合、snapshotを
        // 完全と報告しない。" The specific reason is not surfaced onto the
        // record itself — DES-209/184 give the record only a boolean
        // `complete` — but is available to callers that want to log it via
        // `escape_risk` directly (`vtest-verify`'s diagnostics do, see the
        // caller).
        return incomplete(inputs.schema);
    }
    let Some(entries) = collect_manifest(root) else {
        return incomplete(inputs.schema);
    };

    let manifest_field = FieldValue::Ordered(
        entries
            .into_iter()
            .map(|entry| {
                encode_nested_fields([
                    ("root", FieldValue::exact_bytes(entry.root.into_bytes())),
                    ("path", FieldValue::exact_bytes(entry.path.into_bytes())),
                    ("kind", FieldValue::exact_bytes(entry.kind.into_bytes())),
                    ("bytes", FieldValue::exact_bytes(entry.bytes)),
                ])
            })
            .collect(),
    );

    let hash = SubjectHashInput::new(SubjectDomain::ExecutionState)
        .field(
            "adapter",
            FieldValue::text_fragment(inputs.adapter.as_str()),
        )
        .field("schema", FieldValue::text_fragment(inputs.schema))
        .field("head_revision", FieldValue::text_fragment(head_commit))
        .field("runner_kind", FieldValue::text_fragment(inputs.runner_kind))
        .field("invocation", FieldValue::text_fragment(inputs.invocation))
        .field("toolchain", FieldValue::text_fragment(&toolchain))
        // DES-097's "adapter configのcanonical projection": bound as an
        // explicit empty marker per this function's module-doc disclosure.
        .field("adapter_config", FieldValue::Null)
        .field("manifest", manifest_field)
        .finish();

    ExecutionState {
        schema: inputs.schema.to_owned(),
        complete: true,
        hash: Some(hash),
    }
}

/// Returns why a current reconstruction cannot rule out an excluded-area
/// read (DES-212), or `None` if the check passes. Exposed separately from
/// [`reconstruct_execution_state`] so callers can report the reason as a
/// diagnostic rather than only the boolean `complete: false`.
pub fn escape_risk(root: &Path) -> Option<String> {
    let mut risk = None;
    visit_source_files(root, &mut |path, text| {
        if risk.is_some() {
            return;
        }
        if path.file_name().and_then(|name| name.to_str()) == Some("build.rs") {
            risk = Some(format!(
                "{} is a build script; its effects cannot be statically ruled out (DES-212)",
                path.display()
            ));
            return;
        }
        let syntax = match syn::parse_file(text) {
            Ok(file) => file,
            Err(error) => {
                risk = Some(format!(
                    "{} cannot be parsed as Rust: {error} (DES-212)",
                    path.display()
                ));
                return;
            }
        };
        let mut calls = IncludeCalls::default();
        calls.visit_file(&syntax);
        for (macro_name, argument) in calls.calls {
            // DS-212's escape check must rule out *every* occurrence of
            // each macro in the file, not only the first — a second or
            // later call whose argument resolves outside the manifest's
            // included area is exactly as much an escape risk as a first
            // one would have been. A prior version of this loop checked
            // only the first occurrence, disclosed and corrected here.
            let Some(literal) = argument else {
                risk = Some(format!(
                    "{path} calls {macro_name} with a non-literal argument, which cannot \
                         be statically resolved (DES-212)",
                    path = path.display()
                ));
                return;
            };
            let Some(parent) = path.parent() else {
                continue;
            };
            let resolved = normalize_lexically(&parent.join(&literal));
            if !path_is_under(&resolved, root) || path_is_excluded(&resolved, root) {
                risk = Some(format!(
                    "{path} resolves {macro_name}({literal:?}) outside the manifest's \
                         included area (DES-212)",
                    path = path.display()
                ));
                return;
            }
        }
    });
    risk
}

#[derive(Default)]
struct IncludeCalls {
    calls: Vec<(String, Option<String>)>,
}

impl<'ast> Visit<'ast> for IncludeCalls {
    fn visit_macro(&mut self, node: &'ast Macro) {
        let Some(name) = node.path.segments.last().map(|s| s.ident.to_string()) else {
            return;
        };
        if matches!(name.as_str(), "include" | "include_str" | "include_bytes") {
            let argument =
                syn::parse2::<Expr>(node.tokens.clone())
                    .ok()
                    .and_then(|expr| match expr {
                        Expr::Lit(expr) => match expr.lit {
                            Lit::Str(lit) => Some(lit.value()),
                            _ => None,
                        },
                        _ => None,
                    });
            self.calls.push((format!("{name}!"), argument));
        }
        syn::visit::visit_macro(self, node);
    }
}

fn incomplete(schema: &str) -> ExecutionState {
    ExecutionState {
        schema: schema.to_owned(),
        complete: false,
        hash: None,
    }
}

fn toolchain_identity() -> Option<String> {
    let rustc = Command::new("rustc")
        .args(["-Vv"])
        .output()
        .ok()
        .filter(|output| output.status.success())
        .map(|output| String::from_utf8_lossy(&output.stdout).trim().to_owned())?;
    Some(rustc)
}

struct ManifestEntry {
    root: String,
    path: String,
    kind: String,
    bytes: Vec<u8>,
}

/// DES-098/210: walks `root`, excluding `EXCLUDED_DIR_NAMES` (DES-211),
/// collecting every ordinary file as a byte-exact manifest entry. Returns
/// `None` on any I/O error rather than a partial manifest — an
/// unreadable file must not silently drop out of the bound subject.
fn collect_manifest(root: &Path) -> Option<Vec<ManifestEntry>> {
    let mut entries = Vec::new();
    let mut ok = true;
    visit_files(root, &mut |path, bytes| {
        let Some(relative) = path.strip_prefix(root).ok() else {
            ok = false;
            return;
        };
        let normalized = relative.to_string_lossy().replace('\\', "/");
        entries.push(ManifestEntry {
            root: ROOT_IDENTITY.to_owned(),
            path: normalized,
            // DES-098 "input kind": this slice has exactly one kind of
            // manifest input (an ordinary repository file); a future slice
            // that adds a distinct input kind (e.g. an environment
            // variable) extends this rather than overloading it.
            kind: "file".to_owned(),
            bytes: bytes.to_vec(),
        });
    })?;
    if !ok {
        return None;
    }
    // DES-099: "manifest entry集合は正規化identity順にencodeする".
    entries.sort_by(|a, b| (&a.root, &a.path).cmp(&(&b.root, &b.path)));
    Some(entries)
}

/// Recursively visits every ordinary file under `root` (skipping
/// [`EXCLUDED_DIR_NAMES`] *at `root`'s own top level only* — see [`walk`]'s
/// doc comment — and symlinks), calling `visit` with each file's
/// root-relative path and raw bytes. Returns `None` on any I/O error.
fn visit_files(root: &Path, visit: &mut dyn FnMut(&Path, &[u8])) -> Option<()> {
    walk(root, root, &mut |path| {
        let bytes = fs::read(path).ok()?;
        visit(path, &bytes);
        Some(())
    })
}

/// Same walk as [`visit_files`], but only over `.rs` source files, handing
/// `visit` the file's path and decoded (lossy) text — used by
/// [`escape_risk`], which only needs to grep source text, not hash bytes.
fn visit_source_files(root: &Path, visit: &mut dyn FnMut(&Path, &str)) {
    let _ = walk(root, root, &mut |path| {
        if path.extension().and_then(|ext| ext.to_str()) != Some("rs") {
            return Some(());
        }
        let text = fs::read_to_string(path).ok()?;
        visit(path, &text);
        Some(())
    });
}

/// `EXCLUDED_DIR_NAMES` (`.git`, `.verify`, `target`) is excluded only when
/// it names a **direct child of the workspace root** (`dir == root`), not
/// at any depth. DES-211 names specific root-relative generated-output
/// directories ("`.git/`、`.verify/`…、Cargo target directory"), not a bare
/// directory-name pattern; matching the name at any depth would also drop
/// a real source directory this repository or another project happens to
/// name `target` deeper in its tree (e.g. a crate's own
/// `src/target/mod.rs`) out of the manifest — under-binding the subject in
/// the direction DES-211 does not ask for. A prior version of this walk
/// matched by bare name at any depth, disclosed and corrected here.
fn walk(root: &Path, dir: &Path, visit_file: &mut dyn FnMut(&Path) -> Option<()>) -> Option<()> {
    let entries = fs::read_dir(dir).ok()?;
    for entry in entries {
        let entry = entry.ok()?;
        let path = entry.path();
        let file_type = entry.file_type().ok()?;
        if file_type.is_symlink() {
            continue;
        }
        if file_type.is_dir() {
            if dir == root
                && path
                    .file_name()
                    .and_then(|name| name.to_str())
                    .is_some_and(|name| EXCLUDED_DIR_NAMES.contains(&name))
            {
                continue;
            }
            walk(root, &path, visit_file)?;
        } else if file_type.is_file() {
            visit_file(&path)?;
        }
    }
    Some(())
}

/// Finds the first call to `macro_name` in `text` and extracts its argument:
/// `Some(Some(literal))` for a simple `"..."` string literal argument,
/// `Some(None)` when a call exists but its argument is not a simple literal
/// (concatenation, `env!`, a path expression, etc. — DES-212 cannot rule
/// out an excluded-area read through those), `None` when `macro_name` does
/// not appear at all.
/// Finds *every* call to `macro_name` in `text` and extracts each one's
/// argument (DS-212 must rule out every occurrence, not only the first —
/// see the caller). Each element is `Some(literal)` for a simple `"..."`
/// string literal argument, or `None` when that call's argument is not a
/// simple literal (concatenation, `env!`, a path expression, etc.).
#[allow(dead_code)]
fn old_find_macro_literal_arguments(text: &str, macro_name: &str) -> Vec<Option<String>> {
    let mut results = Vec::new();
    let mut search_from = 0;
    while let Some(relative) = text[search_from..].find(macro_name) {
        let start = search_from + relative;
        search_from = start + macro_name.len();
        let after = &text[search_from..];
        let trimmed = after.trim_start();
        let Some(inner) = trimmed.strip_prefix('(') else {
            continue;
        };
        let inner = inner.trim_start();
        if !inner.starts_with('"') {
            results.push(None);
            continue;
        }
        let mut chars = inner[1..].char_indices();
        let mut escaped = false;
        let mut found = None;
        for (index, ch) in &mut chars {
            if escaped {
                escaped = false;
                continue;
            }
            match ch {
                '\\' => escaped = true,
                '"' => {
                    let literal = &inner[1..1 + index];
                    let rest = inner[1 + index + 1..].trim_start();
                    found = Some(if rest.starts_with(')') {
                        Some(literal.replace("\\\"", "\""))
                    } else {
                        // Something after the string before `)`
                        // (concatenation, a second argument) — not a
                        // simple single-literal call.
                        None
                    });
                    break;
                }
                _ => {}
            }
        }
        results.push(found.unwrap_or(None));
    }
    results
}

/// Lexically resolves `.`/`..` components without touching the filesystem
/// (the target may not exist, e.g. an `include!` path under an excluded
/// directory this walk never visits).
fn normalize_lexically(path: &Path) -> PathBuf {
    let mut out = PathBuf::new();
    for component in path.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                out.pop();
            }
            other => out.push(other.as_os_str()),
        }
    }
    out
}

fn path_is_under(path: &Path, root: &Path) -> bool {
    path.starts_with(root)
}

/// Mirrors [`walk`]'s root-top-level-only exclusion rule: a resolved
/// `include!`-family path is excluded only when its **first** path
/// component under `root` is one of `EXCLUDED_DIR_NAMES`, not when any
/// deeper component happens to share that name (a prior version checked
/// every component, which would have wrongly flagged, e.g., a legitimate
/// `src/target/mod.rs` include as an escape risk).
fn path_is_excluded(path: &Path, root: &Path) -> bool {
    let Ok(relative) = path.strip_prefix(root) else {
        return true;
    };
    relative
        .components()
        .next()
        .and_then(|component| component.as_os_str().to_str())
        .is_some_and(|name| EXCLUDED_DIR_NAMES.contains(&name))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_dir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("vtest-execstate-{name}"));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).expect("create temp dir");
        dir
    }

    fn inputs<'a>(adapter: &'a AdapterId, head: &'a str) -> ExecutionStateInputs<'a> {
        ExecutionStateInputs {
            adapter,
            schema: "test-execution-state-v1",
            head_commit: Some(head),
            runner_kind: "cargo-test",
            invocation: "cargo test -p fixture -- --exact fixture::it_works",
        }
    }

    /// @vtest.id TEST-EXEC-STATE-COMPLETE-RECONSTRUCTION
    /// @vtest.covers VO-EXEC-STATE-COMPLETE-WHEN-INPUTS-RESOLVABLE
    /// @vtest.target crates/vtest-store/src/execution_state.rs::reconstruct_execution_state
    /// @vtest.intent Verifies a repository with no escape risk and a readable manifest reconstructs to complete:true with a hash present.
    #[test]
    fn a_repository_with_no_escape_risk_and_a_readable_manifest_reconstructs_complete() {
        let root = temp_dir("complete");
        fs::write(root.join("Cargo.toml"), "[package]\nname = \"fixture\"\n").expect("write");
        fs::create_dir_all(root.join(".git")).expect("mkdir .git");
        fs::write(root.join(".git").join("HEAD"), "ref: refs/heads/main\n").expect("write");
        let adapter = AdapterId::new("rust-cargo");
        let state = reconstruct_execution_state(&root, inputs(&adapter, "deadbeef"));
        assert!(state.complete, "expected a complete reconstruction");
        assert!(state.hash.is_some());
    }

    /// @vtest.id TEST-EXEC-STATE-DETERMINISTIC-HASH
    /// @vtest.covers VO-EXEC-STATE-HASH-DETERMINISTIC-FOR-IDENTICAL-INPUTS
    /// @vtest.target crates/vtest-store/src/execution_state.rs::reconstruct_execution_state
    /// @vtest.intent Verifies two reconstructions of the identical environment produce the identical hash.
    #[test]
    fn a_repository_reconstructs_the_identical_hash_across_two_calls() {
        let root = temp_dir("deterministic");
        fs::write(root.join("Cargo.toml"), "[package]\nname = \"fixture\"\n").expect("write");
        let adapter = AdapterId::new("rust-cargo");
        let first = reconstruct_execution_state(&root, inputs(&adapter, "deadbeef"));
        let second = reconstruct_execution_state(&root, inputs(&adapter, "deadbeef"));
        assert_eq!(first.hash, second.hash);
        assert!(first.complete && second.complete);
    }

    /// @vtest.id TEST-EXEC-STATE-MANIFEST-CHANGE-CHANGES-HASH
    /// @vtest.covers VO-EXEC-STATE-MANIFEST-BYTES-BOUND-TO-HASH
    /// @vtest.target crates/vtest-store/src/execution_state.rs::reconstruct_execution_state
    /// @vtest.intent Verifies rewriting a manifest file's bytes changes the reconstructed hash.
    #[test]
    fn changing_a_manifest_file_changes_the_hash() {
        let root = temp_dir("file-change");
        fs::write(root.join("Cargo.toml"), "[package]\nname = \"fixture\"\n").expect("write");
        let adapter = AdapterId::new("rust-cargo");
        let before = reconstruct_execution_state(&root, inputs(&adapter, "deadbeef"));
        fs::write(
            root.join("Cargo.toml"),
            "[package]\nname = \"fixture-renamed\"\n",
        )
        .expect("rewrite");
        let after = reconstruct_execution_state(&root, inputs(&adapter, "deadbeef"));
        assert_ne!(before.hash, after.hash);
    }

    /// DES-211 excludes the Cargo *build output* directory at the
    /// workspace root, not any directory that happens to share its name —
    /// a legitimate source directory nested deeper in the tree and named
    /// `target` (e.g. `src/target/mod.rs`) must still be bound into the
    /// manifest. A prior version of the exclusion matched by bare
    /// directory name at any depth and would have silently dropped it.
    /// @vtest.id TEST-EXEC-STATE-NESTED-TARGET-DIR-NOT-EXCLUDED
    /// @vtest.covers VO-EXEC-STATE-NESTED-TARGET-DIR-NOT-EXCLUDED
    /// @vtest.target crates/vtest-store/src/execution_state.rs::walk
    /// @vtest.intent Verifies a nested directory literally named `target` (e.g. `src/target`) is not excluded from the manifest.
    #[test]
    fn a_nested_directory_literally_named_target_is_not_excluded() {
        let root = temp_dir("nested-target-dir");
        fs::write(root.join("Cargo.toml"), "[package]\nname = \"fixture\"\n").expect("write");
        fs::create_dir_all(root.join("src").join("target")).expect("mkdir src/target");
        fs::write(
            root.join("src").join("target").join("mod.rs"),
            "pub fn compile_target() {}\n",
        )
        .expect("write src/target/mod.rs");
        let adapter = AdapterId::new("rust-cargo");
        let with_file = reconstruct_execution_state(&root, inputs(&adapter, "deadbeef"));
        fs::write(
            root.join("src").join("target").join("mod.rs"),
            "pub fn compile_target() { /* changed */ }\n",
        )
        .expect("rewrite src/target/mod.rs");
        let after_change = reconstruct_execution_state(&root, inputs(&adapter, "deadbeef"));
        assert_ne!(
            with_file.hash, after_change.hash,
            "a change inside a nested `target`-named directory must change the manifest hash \
             (it must not have been excluded)"
        );
    }

    /// DS-819 relies on this via the Test subject hash separately, but
    /// DES-097 additionally binds HEAD revision into the Execution State
    /// subject itself; a changed revision must change this hash too.
    /// @vtest.id TEST-EXEC-STATE-HEAD-REVISION-CHANGE-CHANGES-HASH
    /// @vtest.covers VO-EXEC-STATE-HEAD-REVISION-BOUND-TO-HASH
    /// @vtest.target crates/vtest-store/src/execution_state.rs::reconstruct_execution_state
    /// @vtest.intent Verifies changing the HEAD revision input changes the reconstructed hash.
    #[test]
    fn changing_head_revision_changes_the_hash() {
        let root = temp_dir("revision-change");
        fs::write(root.join("Cargo.toml"), "[package]\nname = \"fixture\"\n").expect("write");
        let adapter = AdapterId::new("rust-cargo");
        let before = reconstruct_execution_state(&root, inputs(&adapter, "deadbeef"));
        let after = reconstruct_execution_state(&root, inputs(&adapter, "cafef00d"));
        assert_ne!(before.hash, after.hash);
    }

    /// @vtest.id TEST-EXEC-STATE-ABSENT-HEAD-COMMIT-INCOMPLETE
    /// @vtest.covers VO-EXEC-STATE-ABSENT-HEAD-COMMIT-INCOMPLETE
    /// @vtest.target crates/vtest-store/src/execution_state.rs::reconstruct_execution_state
    /// @vtest.intent Verifies an absent HEAD commit input yields complete:false and hash:None rather than a fabricated hash.
    #[test]
    fn an_absent_head_commit_is_incomplete_not_a_fabricated_hash() {
        let root = temp_dir("no-head");
        fs::write(root.join("Cargo.toml"), "[package]\nname = \"fixture\"\n").expect("write");
        let adapter = AdapterId::new("rust-cargo");
        let state = reconstruct_execution_state(
            &root,
            ExecutionStateInputs {
                adapter: &adapter,
                schema: "test-execution-state-v1",
                head_commit: None,
                runner_kind: "cargo-test",
                invocation: "cargo test",
            },
        );
        assert!(!state.complete);
        assert!(state.hash.is_none());
    }

    /// @vtest.id TEST-EXEC-STATE-BUILD-SCRIPT-FORCES-INCOMPLETE
    /// @vtest.covers VO-EXEC-STATE-ESCAPE-RISK-FORCES-INCOMPLETE
    /// @vtest.target crates/vtest-store/src/execution_state.rs::escape_risk
    /// @vtest.intent Verifies a `build.rs` in the tree makes the escape risk unprovable and forces an incomplete reconstruction.
    /// DES-212: a `build.rs` in the tree makes the escape risk
    /// unprovable, so the reconstruction must not report `complete: true`.
    #[test]
    fn a_build_script_forces_an_incomplete_reconstruction() {
        let root = temp_dir("build-script");
        fs::write(root.join("Cargo.toml"), "[package]\nname = \"fixture\"\n").expect("write");
        fs::write(root.join("build.rs"), "fn main() {}\n").expect("write build.rs");
        assert!(escape_risk(&root).is_some());
        let adapter = AdapterId::new("rust-cargo");
        let state = reconstruct_execution_state(&root, inputs(&adapter, "deadbeef"));
        assert!(!state.complete);
    }

    /// @vtest.id TEST-EXEC-STATE-IN-SCOPE-INCLUDE-NOT-INCOMPLETE
    /// @vtest.covers VO-EXEC-STATE-IN-SCOPE-INCLUDE-NOT-INCOMPLETE
    /// @vtest.target crates/vtest-store/src/execution_state.rs::escape_risk
    /// @vtest.intent Verifies an `include_str!` literal resolving inside the manifest's included area does not force incompleteness.
    /// A literal `include_str!` that stays inside the manifest's own
    /// included area (not an excluded directory) does not itself force
    /// incompleteness — this is the shape this repository's own
    /// `include_str!("../tests/fixtures/...")` calls take.
    #[test]
    fn an_in_scope_include_str_literal_does_not_force_incompleteness() {
        let root = temp_dir("include-in-scope");
        fs::create_dir_all(root.join("src")).expect("mkdir src");
        fs::create_dir_all(root.join("tests").join("fixtures")).expect("mkdir fixtures");
        fs::write(root.join("Cargo.toml"), "[package]\nname = \"fixture\"\n").expect("write");
        fs::write(
            root.join("src").join("lib.rs"),
            "fn f() { let _ = include_str!(\"../tests/fixtures/sample.json\"); }\n",
        )
        .expect("write lib.rs");
        fs::write(
            root.join("tests").join("fixtures").join("sample.json"),
            "{}",
        )
        .expect("write");
        assert!(escape_risk(&root).is_none());
    }

    /// @vtest.id TEST-EXEC-STATE-OUT-OF-SCOPE-INCLUDE-FORCES-INCOMPLETE
    /// @vtest.covers VO-EXEC-STATE-ESCAPE-RISK-FORCES-INCOMPLETE
    /// @vtest.target crates/vtest-store/src/execution_state.rs::escape_risk
    /// @vtest.intent Verifies an `include_str!` literal resolving outside the manifest's included area forces incompleteness.
    /// An `include_str!` that resolves outside the manifest's included area
    /// (here, above the workspace root entirely) cannot be ruled out and
    /// must force incompleteness (DES-212).
    #[test]
    fn an_out_of_scope_include_str_literal_forces_incompleteness() {
        let root = temp_dir("include-out-of-scope").join("nested");
        fs::create_dir_all(root.join("src")).expect("mkdir src");
        fs::write(root.join("Cargo.toml"), "[package]\nname = \"fixture\"\n").expect("write");
        fs::write(
            root.join("src").join("lib.rs"),
            "fn f() { let _ = include_str!(\"../../outside.json\"); }\n",
        )
        .expect("write lib.rs");
        assert!(escape_risk(&root).is_some());
    }

    /// DS-212's escape check must not stop after the first `include_str!`
    /// occurrence in a file — a *second* call whose argument resolves
    /// outside the manifest's included area is exactly as much a risk as a
    /// first would be. A prior version of this check only inspected the
    /// first occurrence (`find_macro_literal_argument`, singular) and
    /// would have missed this.
    /// @vtest.id TEST-EXEC-STATE-SECOND-INCLUDE-OCCURRENCE-FORCES-INCOMPLETE
    /// @vtest.covers VO-EXEC-STATE-ESCAPE-RISK-FORCES-INCOMPLETE
    /// @vtest.target crates/vtest-store/src/execution_state.rs::escape_risk
    /// @vtest.intent Verifies a second `include_str!` occurrence in a file resolving out of scope forces incompleteness, not only the first occurrence.
    #[test]
    fn a_second_include_str_occurrence_pointing_out_of_scope_forces_incompleteness() {
        let root = temp_dir("include-second-out-of-scope").join("nested");
        fs::create_dir_all(root.join("src")).expect("mkdir src");
        fs::create_dir_all(root.join("tests").join("fixtures")).expect("mkdir fixtures");
        fs::write(root.join("Cargo.toml"), "[package]\nname = \"fixture\"\n").expect("write");
        fs::write(
            root.join("tests").join("fixtures").join("in-scope.json"),
            "{}",
        )
        .expect("write in-scope fixture");
        fs::write(
            root.join("src").join("lib.rs"),
            "fn f() {\n    \
                 let _ = include_str!(\"../tests/fixtures/in-scope.json\");\n    \
                 let _ = include_str!(\"../../outside.json\");\n\
             }\n",
        )
        .expect("write lib.rs");
        assert!(escape_risk(&root).is_some());
    }

    /// @vtest.id TEST-EXEC-STATE-NON-LITERAL-INCLUDE-ARG-FORCES-INCOMPLETE
    /// @vtest.covers VO-EXEC-STATE-ESCAPE-RISK-FORCES-INCOMPLETE
    /// @vtest.target crates/vtest-store/src/execution_state.rs::escape_risk
    /// @vtest.intent Verifies a non-literal `include_str!` argument, which cannot be statically resolved, forces incompleteness.
    #[test]
    fn a_non_literal_include_argument_forces_incompleteness() {
        let root = temp_dir("include-dynamic");
        fs::create_dir_all(root.join("src")).expect("mkdir src");
        fs::write(root.join("Cargo.toml"), "[package]\nname = \"fixture\"\n").expect("write");
        fs::write(
            root.join("src").join("lib.rs"),
            "fn f() { let _ = include_str!(concat!(env!(\"OUT_DIR\"), \"/x.rs\")); }\n",
        )
        .expect("write lib.rs");
        assert!(escape_risk(&root).is_some());
    }
}
