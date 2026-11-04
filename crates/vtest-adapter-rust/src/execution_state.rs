//! Rust/Cargo input collection. Core validates and hashes these DTOs.
//! The supported snapshot schema covers one workspace root. External local
//! dependencies, build scripts, procedural macros and unresolved includes
//! report incompleteness.
use proc_macro2::{Delimiter, TokenStream, TokenTree};
use std::{
    collections::BTreeMap,
    fs,
    path::{Component, Path, PathBuf},
    process::Command,
};
use syn::{visit::Visit, Attribute, Expr, Lit, Macro, Meta};
use vtest_model::{CanonicalProjection, ExecutionInputDraft, ExecutionStateDraft};
/// DES-211: `.git/` and `.verify/` are excluded at the workspace root. The
/// Cargo target directory is not a fixed name: it is resolved from the
/// effective environment and Cargo configuration ([`ExecutionContext`]).
const EXCLUDED_ROOT_ENTRIES: &[&str] = &[".git", ".verify"];
const ROOT_IDENTITY: &str = "workspace-root";
const INCLUDE_MACROS: &[&str] = &["include", "include_str", "include_bytes"];

/// The process environment a runner command inherits. Keys and values are
/// kept as UTF-8; a variable that is not valid Unicode is recorded in
/// [`Environment::undecodable`] so the snapshot can refuse completeness for
/// an execution-affecting variable it cannot bind (DES-208).
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct Environment {
    pub vars: BTreeMap<String, String>,
    pub undecodable: Vec<String>,
}

impl Environment {
    /// The environment of the current process, which `vtest-exec` passes on
    /// unchanged to the runner command (the command only overlays its own
    /// `env` map, which is folded in by [`Environment::with_overlay`]).
    pub fn current() -> Self {
        let mut environment = Self::default();
        for (key, value) in std::env::vars_os() {
            match (key.into_string(), value.into_string()) {
                (Ok(key), Ok(value)) => {
                    environment.vars.insert(key, value);
                }
                (Ok(key), Err(_)) => environment.undecodable.push(key),
                (Err(key), _) => environment
                    .undecodable
                    .push(key.to_string_lossy().into_owned()),
            }
        }
        environment.undecodable.sort();
        environment
    }

    pub fn from_vars<I, K, V>(vars: I) -> Self
    where
        I: IntoIterator<Item = (K, V)>,
        K: Into<String>,
        V: Into<String>,
    {
        Self {
            vars: vars
                .into_iter()
                .map(|(key, value)| (key.into(), value.into()))
                .collect(),
            undecodable: Vec::new(),
        }
    }

    /// The command's own `env` entries override inherited ones, exactly as
    /// `std::process::Command::env` does when `vtest-exec` launches it.
    pub fn with_overlay(mut self, overlay: &BTreeMap<String, String>) -> Self {
        for (key, value) in overlay {
            self.undecodable.retain(|name| name != key);
            self.vars.insert(key.clone(), value.clone());
        }
        self
    }

    fn get(&self, key: &str) -> Option<&str> {
        self.vars.get(key).map(String::as_str)
    }
}

/// DES-097/208/369: the variables Cargo, rustc, rustup and the libtest
/// harness read as configuration all carry the `CARGO` or `RUST` prefix
/// (`RUSTFLAGS`, `CARGO_ENCODED_RUSTFLAGS`, `RUSTC`, `RUSTC_WRAPPER`,
/// `RUSTUP_TOOLCHAIN`, `CARGO_TARGET_DIR`, `CARGO_PROFILE_*`,
/// `CARGO_TARGET_<triple>_RUSTFLAGS`, `RUST_TEST_THREADS`, ...). Binding the
/// whole prefix family, rather than a list of names known today, keeps a
/// newly introduced Cargo setting bound instead of silently unbound.
///
/// Two groups are left out, each for a reason that holds for every value:
/// - per-crate values Cargo itself sets for the compiled crate and the test
///   process (`CARGO`, `CARGO_PKG_*`, `CARGO_MANIFEST_DIR`,
///   `CARGO_MANIFEST_PATH`, `CARGO_CRATE_NAME`). The inner `cargo test`
///   overwrites any inherited value, so an outer value never reaches the
///   execution;
/// - credentials (`*TOKEN*`, `*CREDENTIAL*`, `*SECRET*`), which authenticate
///   registry access and do not change what is built or run.
fn affects_execution(name: &str) -> bool {
    if !(name.starts_with("CARGO") || name.starts_with("RUST")) {
        return false;
    }
    let overwritten_by_cargo = name == "CARGO"
        || name.starts_with("CARGO_PKG_")
        || matches!(
            name,
            "CARGO_MANIFEST_DIR" | "CARGO_MANIFEST_PATH" | "CARGO_CRATE_NAME"
        );
    let credential = ["TOKEN", "CREDENTIAL", "SECRET"]
        .iter()
        .any(|word| name.contains(word));
    !overwritten_by_cargo && !credential
}

/// The effective execution context of one snapshot: the bound environment,
/// the Cargo configuration files Cargo reads for a command started in `root`,
/// and the generated-output directories DES-211 excludes from the manifest.
#[derive(Clone, Debug)]
pub struct ExecutionContext {
    environment: Environment,
    /// Configuration files in Cargo's precedence order (highest first).
    config_files: Vec<ConfigFile>,
    /// Absolute, lexically normalized generated-output directories.
    output_dirs: Vec<PathBuf>,
    /// Reasons this context cannot be enumerated completely (DES-208).
    incomplete: Vec<String>,
}

#[derive(Clone, Debug)]
struct ConfigFile {
    /// Logical location: a root-relative path for files inside `root`, or
    /// `ancestor-<n>/...` / `cargo-home/...` for files outside it, so the
    /// projection carries no machine path (DES-370).
    logical: String,
    /// The directory that contains the `.cargo` directory (or `CARGO_HOME`);
    /// Cargo resolves relative paths in the file against it.
    base: PathBuf,
    inside_root: bool,
    text: String,
}

impl ExecutionContext {
    /// Resolves the context for a runner command started in `root` with
    /// `environment` (DES-369: the same environment the command inherits).
    pub fn resolve(root: &Path, environment: Environment, runner_kind: &str) -> Self {
        let mut incomplete = Vec::new();
        for name in &environment.undecodable {
            if affects_execution(name) {
                incomplete.push(format!(
                    "environment variable {name} is not valid Unicode and cannot be bound \
                     (DES-208)"
                ));
            }
        }
        let config_files = cargo_config_files(root, &environment, &mut incomplete);
        let target_dir = cargo_target_dir(root, &environment, &config_files, &mut incomplete);
        let mut output_dirs = vec![absolute(&target_dir)];
        // `cargo llvm-cov` builds into its own target directory, which a
        // user may move with CARGO_LLVM_COV_TARGET_DIR.
        if runner_kind == "cargo-llvm-cov" {
            if let Some(dir) = environment.get("CARGO_LLVM_COV_TARGET_DIR") {
                output_dirs.push(absolute(&root.join(dir)));
            }
        }
        let absolute_root = absolute(root);
        for dir in &output_dirs {
            if absolute_root.starts_with(dir) {
                incomplete.push(format!(
                    "the Cargo output directory {} contains the workspace root, so generated \
                     files cannot be separated from inputs (DES-211)",
                    dir.display()
                ));
            }
        }
        Self {
            environment,
            config_files,
            output_dirs,
            incomplete,
        }
    }

    /// The directories DES-211 excludes as Cargo generated output.
    pub fn output_dirs(&self) -> &[PathBuf] {
        &self.output_dirs
    }

    /// DES-097: the canonical projection of the execution-affecting
    /// configuration — the bound environment variables and the Cargo
    /// configuration files outside `root` (files inside `root` are bound as
    /// manifest entries already).
    fn effective_config(&self) -> CanonicalProjection {
        let environment: serde_json::Map<String, serde_json::Value> = self
            .environment
            .vars
            .iter()
            .filter(|(name, _)| affects_execution(name))
            .map(|(name, value)| (name.clone(), serde_json::Value::String(value.clone())))
            .collect();
        let config: Vec<serde_json::Value> = self
            .config_files
            .iter()
            .filter(|file| !file.inside_root)
            .map(|file| serde_json::json!({"location": file.logical, "text": file.text}))
            .collect();
        serde_json::json!({"environment": environment, "cargo_config": config})
    }
}

/// Cargo reads `.cargo/config.toml` (and the legacy `.cargo/config`) from
/// the current directory and every ancestor, then from `CARGO_HOME`; a
/// deeper file takes precedence. An unreadable or non-UTF-8 file makes the
/// context incomplete rather than silently unbound.
fn cargo_config_files(
    root: &Path,
    environment: &Environment,
    incomplete: &mut Vec<String>,
) -> Vec<ConfigFile> {
    let mut files = Vec::new();
    let mut read = |path: PathBuf, logical: String, base: PathBuf, inside_root: bool| {
        if !path.is_file() {
            return;
        }
        match fs::read_to_string(&path) {
            Ok(text) => files.push(ConfigFile {
                logical,
                base,
                inside_root,
                text,
            }),
            Err(error) => incomplete.push(format!(
                "Cargo configuration {} cannot be read: {error} (DES-208)",
                path.display()
            )),
        }
    };
    let absolute_root = absolute(root);
    for (depth, dir) in absolute_root.ancestors().enumerate() {
        for name in ["config.toml", "config"] {
            let path = dir.join(".cargo").join(name);
            let logical = if depth == 0 {
                format!(".cargo/{name}")
            } else {
                format!("ancestor-{depth}/.cargo/{name}")
            };
            read(path, logical, dir.to_owned(), depth == 0);
        }
    }
    let cargo_home = environment
        .get("CARGO_HOME")
        .map(|home| absolute(&root.join(home)))
        .or_else(|| {
            environment
                .get("HOME")
                .or_else(|| environment.get("USERPROFILE"))
                .map(|home| absolute(&PathBuf::from(home).join(".cargo")))
        });
    if let Some(home) = cargo_home {
        // A CARGO_HOME that is one of the `.cargo` directories visited above
        // is the same file; read it once.
        let already_read = absolute(root)
            .ancestors()
            .any(|dir| dir.join(".cargo") == home);
        if !already_read {
            for name in ["config.toml", "config"] {
                read(
                    home.join(name),
                    format!("cargo-home/{name}"),
                    home.clone(),
                    false,
                );
            }
        }
    }
    files
}

/// DES-211: the Cargo target directory Cargo actually uses — the
/// `CARGO_TARGET_DIR` / `CARGO_BUILD_TARGET_DIR` environment variable, else
/// the highest-precedence `build.target-dir` configuration value, else
/// `target/` in the workspace root.
fn cargo_target_dir(
    root: &Path,
    environment: &Environment,
    config_files: &[ConfigFile],
    incomplete: &mut Vec<String>,
) -> PathBuf {
    for name in ["CARGO_TARGET_DIR", "CARGO_BUILD_TARGET_DIR"] {
        if let Some(dir) = environment.get(name).filter(|dir| !dir.is_empty()) {
            return normalize_lexically(&root.join(dir));
        }
    }
    for file in config_files {
        let value: toml::Value = match toml::from_str(&file.text) {
            Ok(value) => value,
            Err(error) => {
                incomplete.push(format!(
                    "Cargo configuration {} cannot be parsed: {error} (DES-208)",
                    file.logical
                ));
                continue;
            }
        };
        let Some(dir) = value.get("build").and_then(|build| build.get("target-dir")) else {
            continue;
        };
        match dir.as_str() {
            Some(dir) => return normalize_lexically(&file.base.join(dir)),
            None => incomplete.push(format!(
                "Cargo configuration {} has a non-string build.target-dir (DES-208)",
                file.logical
            )),
        }
    }
    root.join("target")
}

pub fn snapshot(
    root: &Path,
    head: Option<&str>,
    runner_kind: &str,
    invocation: CanonicalProjection,
) -> ExecutionStateDraft {
    snapshot_in(root, head, runner_kind, invocation, Environment::current())
}

/// [`snapshot`] for an explicit environment: the one the runner command
/// inherits (DES-369). Tests pass a fixed environment instead of mutating
/// the shared process environment.
pub fn snapshot_in(
    root: &Path,
    head: Option<&str>,
    runner_kind: &str,
    invocation: CanonicalProjection,
    environment: Environment,
) -> ExecutionStateDraft {
    let context = ExecutionContext::resolve(root, environment, runner_kind);
    let toolchain = toolchain_identity(root, &context.environment);
    let entries = collect_manifest(root, &context);
    let complete = head.is_some()
        && toolchain.is_some()
        && entries.is_some()
        && context.incomplete.is_empty()
        && escape_risk_in(root, &context).is_none()
        && local_dependencies_are_in_scope(root, &context);
    ExecutionStateDraft {
        schema_id: "rust-cargo-execution-state".into(),
        schema_version: "1".into(),
        complete,
        head_revision: head.map(str::to_owned),
        runner_kind: runner_kind.into(),
        invocation,
        toolchain_identity: toolchain.unwrap_or_default(),
        effective_config: context.effective_config(),
        inputs: entries.unwrap_or_default(),
    }
}

// DES-210/212: this schema cannot claim completeness for a path dependency
// outside the collected root. Inspect dependency path declarations in manifests.
fn local_dependencies_are_in_scope(root: &Path, context: &ExecutionContext) -> bool {
    fn paths_in_scope(
        value: &toml::Value,
        parent: &Path,
        root: &Path,
        context: &ExecutionContext,
    ) -> bool {
        match value {
            toml::Value::Table(table) => table.iter().all(|(key, value)| {
                if key == "path" {
                    value.as_str().is_some_and(|path| {
                        let path = normalize_lexically(&parent.join(path));
                        path_is_under(&path, root) && !path_is_excluded(&path, root, context)
                    })
                } else {
                    paths_in_scope(value, parent, root, context)
                }
            }),
            toml::Value::Array(values) => values
                .iter()
                .all(|v| paths_in_scope(v, parent, root, context)),
            _ => true,
        }
    }
    let mut valid = true;
    let walked = walk(root, root, context, &mut |path| {
        if path.file_name().and_then(|n| n.to_str()) == Some("Cargo.toml") {
            let text = fs::read_to_string(path).ok()?;
            let manifest: toml::Value = toml::from_str(&text).ok()?;
            valid &= paths_in_scope(&manifest, path.parent()?, root, context);
        }
        Some(())
    });
    valid && walked.is_some()
}

/// DES-212 escape check with the context of the current process environment.
pub fn escape_risk(root: &Path) -> Option<String> {
    let context = ExecutionContext::resolve(root, Environment::current(), "cargo-test");
    escape_risk_in(root, &context)
}

/// DES-210/212: code that runs at build time with arbitrary file access —
/// a build script or a procedural macro of a workspace package — can read an
/// excluded area, so its presence rules out a complete snapshot. Cargo
/// selects the build script from `package.build` (a path, `true` for
/// `build.rs`, `false` for none) and otherwise from a `build.rs` file beside
/// the manifest; every `build.rs` file is treated as a build script so a
/// file that Cargo would pick up is never missed.
fn manifest_escape_risk(path: &Path, manifest: &toml::Value) -> Option<String> {
    let package = manifest.get("package");
    match package.and_then(|package| package.get("build")) {
        Some(toml::Value::String(script)) => {
            return Some(format!(
                "{} declares build script {script:?}; its effects cannot be statically ruled \
                 out (DES-212)",
                path.display()
            ))
        }
        Some(toml::Value::Boolean(true)) => {
            return Some(format!(
                "{} declares a build script; its effects cannot be statically ruled out \
                 (DES-212)",
                path.display()
            ))
        }
        Some(toml::Value::Boolean(false)) | None => {}
        Some(_) => {
            return Some(format!(
                "{} has a package.build value Cargo does not accept; the build script \
                 cannot be resolved (DES-212)",
                path.display()
            ))
        }
    }
    let lib = manifest.get("lib");
    let proc_macro = ["proc-macro", "proc_macro"].iter().any(|key| {
        lib.and_then(|lib| lib.get(*key))
            .and_then(toml::Value::as_bool)
            .unwrap_or(false)
    });
    proc_macro.then(|| {
        format!(
            "{} is a procedural macro crate; its compile-time effects cannot be statically \
             ruled out (DES-212)",
            path.display()
        )
    })
}

pub fn escape_risk_in(root: &Path, context: &ExecutionContext) -> Option<String> {
    let mut risk = None;
    let walked = walk(root, root, context, &mut |path| {
        if risk.is_some() {
            return Some(());
        }
        let name = path.file_name().and_then(|name| name.to_str());
        if name == Some("build.rs") {
            risk = Some(format!(
                "{} is a build script; its effects cannot be statically ruled out (DES-212)",
                path.display()
            ));
            return Some(());
        }
        if name == Some("Cargo.toml") {
            let text = fs::read_to_string(path).ok()?;
            risk = match toml::from_str::<toml::Value>(&text) {
                Ok(manifest) => manifest_escape_risk(path, &manifest),
                Err(error) => Some(format!(
                    "{} cannot be parsed as a Cargo manifest: {error} (DES-212)",
                    path.display()
                )),
            };
            return Some(());
        }
        if path.extension().and_then(|ext| ext.to_str()) != Some("rs") {
            return Some(());
        }
        let text = fs::read_to_string(path).ok()?;
        risk = source_escape_risk(root, context, path, &text);
        Some(())
    });
    risk.or_else(|| {
        walked
            .is_none()
            .then(|| "source inputs could not be read completely".to_owned())
    })
}

fn source_escape_risk(
    root: &Path,
    context: &ExecutionContext,
    path: &Path,
    text: &str,
) -> Option<String> {
    let syntax = match syn::parse_file(text) {
        Ok(file) => file,
        Err(error) => {
            return Some(format!(
                "{} cannot be parsed as Rust: {error} (DES-212)",
                path.display()
            ));
        }
    };
    let mut calls = IncludeCalls::default();
    calls.visit_file(&syntax);
    if let Some(reason) = calls.unresolvable {
        return Some(format!("{}: {reason} (DES-212)", path.display()));
    }
    let parent = path.parent()?;
    for (macro_name, argument) in calls.calls {
        // DS-212's escape check must rule out *every* occurrence of each
        // macro in the file, not only the first — a second or later call
        // whose argument resolves outside the manifest's included area is
        // exactly as much an escape risk as a first one would have been.
        let Some(literal) = argument else {
            return Some(format!(
                "{path} calls {macro_name} with a non-literal argument, which cannot be \
                 statically resolved (DES-212)",
                path = path.display()
            ));
        };
        let resolved = normalize_lexically(&parent.join(&literal));
        if !path_is_under(&resolved, root) || path_is_excluded(&resolved, root, context) {
            return Some(format!(
                "{path} resolves {macro_name}({literal:?}) outside the manifest's included \
                 area (DES-212)",
                path = path.display()
            ));
        }
    }
    None
}

/// Collects every `include!`-family call and `#[path]` module path in a file.
///
/// `syn` leaves the tokens of a macro invocation (`concat!(..)`,
/// `assert_eq!(..)`, a `macro_rules!` body, ...) and of a list attribute
/// (`#[cfg_attr(.., doc = ..)]`) unparsed, so a visitor over the syntax tree
/// alone never sees an `include_str!` nested in them. Those token streams are
/// scanned token by token as well. A macro called through a `macro_rules!`
/// metavariable (`$m!(..)`, which may be handed `include_str`) cannot be
/// resolved statically and is reported in `unresolvable` (DES-212).
///
/// A `path` attribute is found wherever rustc can apply it: directly
/// (`#[path = ".."]`), inside `cfg_attr` at any nesting depth
/// (`#[cfg_attr(unix, path = "..")]`), and in an attribute spelled out in
/// macro tokens (`macro_rules! m { () => { #[path = ".."] mod x; } }`). A
/// `path` value or a whole attribute that is a `macro_rules!` metavariable
/// (`#[path = $p]`, `#[$a] mod x;`) cannot be resolved statically and is
/// reported in `unresolvable`.
#[derive(Default)]
struct IncludeCalls {
    calls: Vec<(String, Option<String>)>,
    unresolvable: Option<String>,
    /// Whether the macro tokens being scanned declare a file module
    /// (`mod x;` or `mod $x;`). Only then can a metavariable attribute
    /// (`#[$a]`) be a `path` that loads a file; the common
    /// `$(#[$meta])* struct ..` doc pass-through cannot.
    file_module_in_macro: bool,
}

/// Whether `tokens`, at any group depth, declare a module loaded from a file:
/// `mod name;` or `mod $name;` (a `mod name { .. }` is inline and loads none).
fn declares_file_module(tokens: TokenStream) -> bool {
    let tokens: Vec<TokenTree> = tokens.into_iter().collect();
    tokens.iter().enumerate().any(|(index, token)| match token {
        TokenTree::Group(group) => declares_file_module(group.stream()),
        TokenTree::Ident(ident) if ident == "mod" => {
            let semicolon = |offset: usize| {
                matches!(tokens.get(index + offset), Some(TokenTree::Punct(p)) if p.as_char() == ';')
            };
            match tokens.get(index + 1) {
                Some(TokenTree::Ident(_)) => semicolon(2),
                Some(TokenTree::Punct(p)) if p.as_char() == '$' => {
                    matches!(tokens.get(index + 2), Some(TokenTree::Ident(_))) && semicolon(3)
                }
                _ => false,
            }
        }
        _ => false,
    })
}

impl IncludeCalls {
    /// Records the module path an attribute's content (the tokens between
    /// `#[` and `]`) can apply: `path = ".."` itself, or any `path` entry
    /// inside a (possibly nested) `cfg_attr(predicate, attr, ..)`. The
    /// `cfg` predicate is not evaluated: a configuration that selects the
    /// path is possible, so its target counts as read.
    fn scan_attribute(&mut self, tokens: &[TokenTree]) {
        match tokens {
            [TokenTree::Ident(ident), TokenTree::Punct(eq), value @ ..]
                if ident == "path" && eq.as_char() == '=' =>
            {
                match value {
                    [TokenTree::Punct(dollar), ..] if dollar.as_char() == '$' => {
                        self.unresolvable.get_or_insert_with(|| {
                            "a #[path] value is a macro_rules! metavariable, so the module \
                             file it loads cannot be resolved"
                                .to_owned()
                        });
                    }
                    _ => {
                        let argument = match value {
                            [literal @ TokenTree::Literal(_)] => {
                                syn::parse2::<syn::LitStr>(literal.clone().into())
                                    .ok()
                                    .map(|literal| literal.value())
                            }
                            _ => None,
                        };
                        self.calls.push(("#[path]".to_owned(), argument));
                    }
                }
            }
            [TokenTree::Ident(ident), TokenTree::Group(group)]
                if ident == "cfg_attr" && group.delimiter() == Delimiter::Parenthesis =>
            {
                self.scan_cfg_attr(group.stream());
            }
            [TokenTree::Punct(dollar), ..]
                if dollar.as_char() == '$' && self.file_module_in_macro =>
            {
                self.unresolvable.get_or_insert_with(|| {
                    "an attribute is a macro_rules! metavariable (`#[$a]`), so whether it \
                     sets a module path cannot be resolved"
                        .to_owned()
                });
            }
            _ => {}
        }
    }

    /// `cfg_attr(predicate, attr1, attr2, ..)`: the first entry is the
    /// predicate; every later entry is an attribute.
    fn scan_cfg_attr(&mut self, body: TokenStream) {
        let inner: Vec<TokenTree> = body.into_iter().collect();
        for attribute in inner
            .split(|token| matches!(token, TokenTree::Punct(p) if p.as_char() == ','))
            .skip(1)
        {
            self.scan_attribute(attribute);
        }
    }

    fn scan_tokens(&mut self, tokens: TokenStream) {
        let tokens: Vec<TokenTree> = tokens.into_iter().collect();
        for (index, token) in tokens.iter().enumerate() {
            match token {
                TokenTree::Punct(pound) if pound.as_char() == '#' => {
                    // An attribute written in macro tokens: `#[..]` or `#![..]`.
                    let group = match (tokens.get(index + 1), tokens.get(index + 2)) {
                        (Some(TokenTree::Group(group)), _) => Some(group),
                        (Some(TokenTree::Punct(bang)), Some(TokenTree::Group(group)))
                            if bang.as_char() == '!' =>
                        {
                            Some(group)
                        }
                        _ => None,
                    };
                    if let Some(group) = group.filter(|g| g.delimiter() == Delimiter::Bracket) {
                        let content: Vec<TokenTree> = group.stream().into_iter().collect();
                        self.scan_attribute(&content);
                    }
                }
                TokenTree::Group(group) => self.scan_tokens(group.stream()),
                TokenTree::Ident(ident) => {
                    let name = ident.to_string();
                    if !INCLUDE_MACROS.contains(&name.as_str()) {
                        continue;
                    }
                    let bang = matches!(
                        tokens.get(index + 1),
                        Some(TokenTree::Punct(punct)) if punct.as_char() == '!'
                    );
                    if !bang {
                        // A bare identifier (a variable named `include`, or a
                        // macro name handed to another macro) reads nothing by
                        // itself; a local macro that calls a handed-in name
                        // does so through `$m!`, reported below.
                        continue;
                    }
                    match tokens.get(index + 2) {
                        Some(TokenTree::Group(group)) if group.delimiter() != Delimiter::None => {
                            let argument = syn::parse2::<syn::LitStr>(group.stream())
                                .ok()
                                .map(|literal| literal.value());
                            self.calls.push((format!("{name}!"), argument));
                        }
                        _ => {
                            self.unresolvable.get_or_insert_with(|| {
                                format!(
                                    "`{name}!` appears in macro tokens without a delimited \
                                     argument, so what it reads cannot be resolved"
                                )
                            });
                        }
                    }
                }
                TokenTree::Punct(punct) if punct.as_char() == '$' => {
                    let metavariable_call = matches!(
                        (tokens.get(index + 1), tokens.get(index + 2)),
                        (Some(TokenTree::Ident(_)), Some(TokenTree::Punct(bang)))
                            if bang.as_char() == '!'
                    );
                    if metavariable_call {
                        self.unresolvable.get_or_insert_with(|| {
                            "a macro is called through a macro_rules! metavariable, so \
                             whether it reads a file cannot be resolved"
                                .to_owned()
                        });
                    }
                }
                _ => {}
            }
        }
    }
}

impl<'ast> Visit<'ast> for IncludeCalls {
    fn visit_macro(&mut self, node: &'ast Macro) {
        let Some(name) = node.path.segments.last().map(|s| s.ident.to_string()) else {
            return;
        };
        if INCLUDE_MACROS.contains(&name.as_str()) {
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
        } else {
            let outer = self.file_module_in_macro;
            self.file_module_in_macro = outer || declares_file_module(node.tokens.clone());
            self.scan_tokens(node.tokens.clone());
            self.file_module_in_macro = outer;
        }
        syn::visit::visit_macro(self, node);
    }

    fn visit_meta(&mut self, node: &'ast Meta) {
        if let Meta::List(list) = node {
            self.scan_tokens(list.tokens.clone());
        }
        syn::visit::visit_meta(self, node);
    }

    fn visit_attribute(&mut self, node: &'ast Attribute) {
        // `#[path = ".."] mod m;` loads a module file from that path, and so
        // does `#[cfg_attr(cfg, path = "..")] mod m;` when `cfg` holds. Every
        // attribute is checked, not only those syn attaches to a `mod` item:
        // a `path` that applies to nothing loads nothing, so reporting it
        // only errs toward incompleteness.
        match &node.meta {
            Meta::NameValue(meta) if meta.path.is_ident("path") => {
                let argument = match &meta.value {
                    Expr::Lit(expr) => match &expr.lit {
                        Lit::Str(lit) => Some(lit.value()),
                        _ => None,
                    },
                    _ => None,
                };
                self.calls.push(("#[path]".to_owned(), argument));
            }
            Meta::List(list) if list.path.is_ident("cfg_attr") => {
                self.scan_cfg_attr(list.tokens.clone());
            }
            _ => {}
        }
        syn::visit::visit_attribute(self, node);
    }
}

/// DES-097/369: the toolchain identity of the toolchain the runner command
/// actually selects — `rustc` (or `$RUSTC`) and `cargo` started in `root`
/// with the command's environment, so a `rust-toolchain` file or a
/// `RUSTUP_TOOLCHAIN` override is reflected.
fn toolchain_identity(root: &Path, environment: &Environment) -> Option<String> {
    let version = |program: &str, args: &[&str]| {
        Command::new(program)
            .args(args)
            .current_dir(root)
            .env_clear()
            .envs(&environment.vars)
            .output()
            .ok()
            .filter(|output| output.status.success())
            .map(|output| String::from_utf8_lossy(&output.stdout).trim().to_owned())
    };
    let rustc = version(environment.get("RUSTC").unwrap_or("rustc"), &["-Vv"])?;
    let cargo = version("cargo", &["-V"])?;
    Some(format!("{rustc}\n{cargo}"))
}

/// DES-098/210: walks `root`, excluding generated output (DES-211),
/// collecting every ordinary file as a byte-exact manifest entry. Returns
/// `None` on any I/O error rather than a partial manifest — an
/// unreadable file must not silently drop out of the bound subject.
fn collect_manifest(root: &Path, context: &ExecutionContext) -> Option<Vec<ExecutionInputDraft>> {
    let mut entries = Vec::new();
    let mut ok = true;
    walk(root, root, context, &mut |path| {
        let bytes = fs::read(path).ok()?;
        let Some(relative) = path.strip_prefix(root).ok() else {
            ok = false;
            return Some(());
        };
        let normalized = relative.to_string_lossy().replace('\\', "/");
        entries.push(ExecutionInputDraft {
            root_identity: ROOT_IDENTITY.to_owned(),
            root_relative_path: normalized,
            // DES-098 "input kind": this slice has exactly one kind of
            // manifest input (an ordinary repository file); the environment
            // and out-of-root configuration are bound through
            // `effective_config` instead.
            kind: "file".to_owned(),
            bytes,
        });
        Some(())
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

/// Visits every ordinary file under `root`, skipping generated output and
/// failing on symlinks. Returns `None` on any I/O error.
///
/// DES-211 names specific generated-output locations ("`.git/`、`.verify/`…、
/// Cargo target directory"), not a bare directory-name pattern. `.git` and
/// `.verify` are excluded only as **direct children of the workspace root**,
/// and the Cargo target directory only at the location Cargo resolves for
/// this context ([`ExecutionContext::output_dirs`]). Matching a name at any
/// depth would also drop a real source directory named `target` deeper in
/// the tree (e.g. `src/target/mod.rs`) — under-binding the subject in the
/// direction DES-211 does not ask for — and matching only the fixed name
/// `target` would bind the generated files of a relocated target directory
/// (`CARGO_TARGET_DIR=out`) as inputs, so every build changes the subject.
fn walk(
    root: &Path,
    dir: &Path,
    context: &ExecutionContext,
    visit_file: &mut dyn FnMut(&Path) -> Option<()>,
) -> Option<()> {
    let entries = fs::read_dir(dir).ok()?;
    for entry in entries {
        let entry = entry.ok()?;
        let path = entry.path();
        // Git worktrees use a .git file rather than a directory. Excluding
        // the logical generated entry also keeps its machine path unbound.
        if path_is_excluded(&path, root, context) {
            continue;
        }
        let file_type = entry.file_type().ok()?;
        if file_type.is_symlink() {
            return None;
        }
        if file_type.is_dir() {
            walk(root, &path, context, visit_file)?;
        } else if file_type.is_file() {
            visit_file(&path)?;
        }
    }
    Some(())
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

/// `path` made absolute against the current directory and lexically
/// normalized, so paths built from a relative root, an absolute
/// `CARGO_TARGET_DIR`, or `..` components compare consistently.
fn absolute(path: &Path) -> PathBuf {
    if path.is_absolute() {
        normalize_lexically(path)
    } else {
        normalize_lexically(&std::env::current_dir().unwrap_or_default().join(path))
    }
}

fn path_is_under(path: &Path, root: &Path) -> bool {
    absolute(path).starts_with(absolute(root))
}

/// Mirrors [`walk`]'s exclusion rule for a resolved path: excluded when its
/// first component under `root` is `.git` or `.verify`, or when it lies in a
/// resolved Cargo output directory. A deeper component that merely shares one
/// of those names (a legitimate `src/target/mod.rs`) is not excluded.
fn path_is_excluded(path: &Path, root: &Path, context: &ExecutionContext) -> bool {
    let path = absolute(path);
    let Ok(relative) = path.strip_prefix(absolute(root)) else {
        return true;
    };
    let root_entry = relative
        .components()
        .next()
        .and_then(|component| component.as_os_str().to_str())
        .is_some_and(|name| EXCLUDED_ROOT_ENTRIES.contains(&name));
    root_entry || context.output_dirs.iter().any(|dir| path.starts_with(dir))
}
