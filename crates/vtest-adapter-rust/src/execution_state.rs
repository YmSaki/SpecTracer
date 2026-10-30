//! Rust/Cargo input collection. Core validates and hashes these DTOs.
//! The supported snapshot schema covers one workspace root. External local
//! dependencies, build scripts and unresolved includes report incompleteness.
use std::{
    fs,
    path::{Component, Path, PathBuf},
    process::Command,
};
use syn::{visit::Visit, Expr, Lit, Macro};
use vtest_model::{CanonicalProjection, ExecutionInputDraft, ExecutionStateDraft};
const EXCLUDED_DIR_NAMES: &[&str] = &[".git", ".verify", "target"];
const ROOT_IDENTITY: &str = "workspace-root";

pub fn snapshot(
    root: &Path,
    head: Option<&str>,
    runner_kind: &str,
    invocation: CanonicalProjection,
) -> ExecutionStateDraft {
    let toolchain = toolchain_identity();
    let entries = collect_manifest(root);
    let complete = head.is_some()
        && toolchain.is_some()
        && entries.is_some()
        && escape_risk(root).is_none()
        && local_dependencies_are_in_scope(root);
    ExecutionStateDraft {
        schema_id: "rust-cargo-execution-state".into(),
        schema_version: "1".into(),
        complete,
        head_revision: head.map(str::to_owned),
        runner_kind: runner_kind.into(),
        invocation,
        toolchain_identity: toolchain.unwrap_or_default(),
        effective_config: serde_json::Value::Null,
        inputs: entries.unwrap_or_default(),
    }
}

// DES-210/212: this schema cannot claim completeness for a path dependency
// outside the collected root. Inspect dependency path declarations in manifests.
fn local_dependencies_are_in_scope(root: &Path) -> bool {
    fn paths_in_scope(value: &toml::Value, parent: &Path, root: &Path) -> bool {
        match value {
            toml::Value::Table(table) => table.iter().all(|(key, value)| {
                if key == "path" {
                    value.as_str().is_some_and(|path| {
                        let path = normalize_lexically(&parent.join(path));
                        path_is_under(&path, root) && !path_is_excluded(&path, root)
                    })
                } else {
                    paths_in_scope(value, parent, root)
                }
            }),
            toml::Value::Array(values) => values.iter().all(|v| paths_in_scope(v, parent, root)),
            _ => true,
        }
    }
    let mut valid = true;
    let walked = walk(root, root, &mut |path| {
        if path.file_name().and_then(|n| n.to_str()) == Some("Cargo.toml") {
            let text = fs::read_to_string(path).ok()?;
            let manifest: toml::Value = toml::from_str(&text).ok()?;
            valid &= paths_in_scope(&manifest, path.parent()?, root);
        }
        Some(())
    });
    valid && walked.is_some()
}

pub fn escape_risk(root: &Path) -> Option<String> {
    let mut risk = None;
    let walked = visit_source_files(root, &mut |path, text| {
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
    risk.or_else(|| {
        walked
            .is_none()
            .then(|| "source inputs could not be read completely".to_owned())
    })
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

fn toolchain_identity() -> Option<String> {
    let rustc = Command::new("rustc")
        .args(["-Vv"])
        .output()
        .ok()
        .filter(|output| output.status.success())
        .map(|output| String::from_utf8_lossy(&output.stdout).trim().to_owned())?;
    Some(rustc)
}

/// DES-098/210: walks `root`, excluding `EXCLUDED_DIR_NAMES` (DES-211),
/// collecting every ordinary file as a byte-exact manifest entry. Returns
/// `None` on any I/O error rather than a partial manifest — an
/// unreadable file must not silently drop out of the bound subject.
fn collect_manifest(root: &Path) -> Option<Vec<ExecutionInputDraft>> {
    let mut entries = Vec::new();
    let mut ok = true;
    visit_files(root, &mut |path, bytes| {
        let Some(relative) = path.strip_prefix(root).ok() else {
            ok = false;
            return;
        };
        let normalized = relative.to_string_lossy().replace('\\', "/");
        entries.push(ExecutionInputDraft {
            root_identity: ROOT_IDENTITY.to_owned(),
            root_relative_path: normalized,
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
    entries.sort_by(|a, b| {
        (&a.root_identity, &a.root_relative_path).cmp(&(&b.root_identity, &b.root_relative_path))
    });
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
fn visit_source_files(root: &Path, visit: &mut dyn FnMut(&Path, &str)) -> Option<()> {
    walk(root, root, &mut |path| {
        if path.extension().and_then(|ext| ext.to_str()) != Some("rs") {
            return Some(());
        }
        let text = fs::read_to_string(path).ok()?;
        visit(path, &text);
        Some(())
    })
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
        // Git worktrees use a .git file rather than a directory. Excluding
        // the logical generated entry also keeps its machine path unbound.
        if dir == root
            && path
                .file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| EXCLUDED_DIR_NAMES.contains(&name))
        {
            continue;
        }
        let file_type = entry.file_type().ok()?;
        if file_type.is_symlink() {
            return None;
        }
        if file_type.is_dir() {
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
