//! The `go` Execution State: the hash-free snapshot of everything that can
//! change what `go test` does for one Test (DES-097, DES-207, DES-369,
//! DES-623 to DES-632, DES-680).
//!
//! Responsibility: list the project's files byte for byte, read the toolchain
//! the command will use, and say whether that list is complete. Completeness is
//! claimed only when every input that can change the result is bound
//! (DES-208); a snapshot that cannot say so is reported incomplete, which keeps
//! its Evidence from standing as a current `PASS` (DES-209).
//!
//! Boundary: hashing and the comparison of snapshots are core's (DES-101,
//! DES-371). The module cache is bound only through `go.sum` (DES-625).

use std::{
    cell::RefCell,
    collections::BTreeMap,
    fs,
    path::{Component, Path, PathBuf},
    process::Command,
    rc::Rc,
};

use serde_json::{json, Value};
use vtest_adapter_api::ExecutionStateReader;
use vtest_model::{ExecutionInputDraft, ExecutionStateDraft};

use crate::runner::{
    baseline_args, coordinates, invocation_args, measured_args, Coordinates, GoTestRunner, RunMode,
    BASELINE_PROFILE_MARKER, PROFILE_MARKER, PROGRAM,
};

/// DES-623 names the schema `go-execution-state-v1`.
const SCHEMA_ID: &str = "go-execution-state";
const SCHEMA_VERSION: &str = "1";

/// The stable root identity of every manifest entry (DES-100): the project
/// root, never its path on this machine.
const ROOT_IDENTITY: &str = "project-root";

/// DES-623: the entries of the project root the manifest leaves out.
const EXCLUDED_ROOT_ENTRIES: &[&str] = &[".git", ".verify"];

/// DES-624: the values of `go env` the toolchain identity binds, besides the
/// instruction-set setting of the target architecture and `GOWORK`.
const BOUND_VARIABLES: &[&str] = &[
    "GOVERSION",
    "GOOS",
    "GOARCH",
    "CGO_ENABLED",
    "CC",
    "CXX",
    "AR",
    "PKG_CONFIG",
    "CGO_CFLAGS",
    "CGO_CPPFLAGS",
    "CGO_CXXFLAGS",
    "CGO_FFLAGS",
    "CGO_LDFLAGS",
    "GOFLAGS",
    "GOEXPERIMENT",
    "GOFIPS140",
    "GODEBUG",
    "GOTOOLCHAIN",
    "GO111MODULE",
];

/// Every instruction-set variable `go env` has. DES-624 binds the one that
/// belongs to `GOARCH` (`GOAMD64` on amd64); see [`instruction_set_variable`].
const INSTRUCTION_SET_VARIABLES: &[&str] = &[
    "GOAMD64",
    "GO386",
    "GOARM",
    "GOARM64",
    "GOMIPS",
    "GOMIPS64",
    "GOPPC64",
    "GORISCV64",
    "GOWASM",
];

/// The `go` flags that take a file or a directory (DES-630). `-modfile`,
/// `-overlay` and `-pgo` are the clause's; the others are the same kind of flag
/// in `go help build` and `go help testflag`.
const PATH_FLAGS: &[&str] = &[
    "modfile",
    "overlay",
    "pgo",
    "pkgdir",
    "o",
    "coverprofile",
    "cpuprofile",
    "memprofile",
    "blockprofile",
    "mutexprofile",
    "trace",
    "outputdir",
    "fuzzcachedir",
];

/// DES-624: the setting that goes with `goarch`, or none for an architecture
/// `go` has no instruction-set setting for. DES-624 names amd64 as the
/// example; the others are the variables `go help environment` lists per
/// architecture.
fn instruction_set_variable(goarch: &str) -> Option<&'static str> {
    match goarch {
        "amd64" => Some("GOAMD64"),
        "386" => Some("GO386"),
        "arm" => Some("GOARM"),
        "arm64" => Some("GOARM64"),
        "mips" | "mipsle" => Some("GOMIPS"),
        "mips64" | "mips64le" => Some("GOMIPS64"),
        "ppc64" | "ppc64le" => Some("GOPPC64"),
        "riscv64" => Some("GORISCV64"),
        "wasm" => Some("GOWASM"),
        _ => None,
    }
}

/// What a snapshot reads once per module: the toolchain, the files of the
/// project, and the reasons the snapshot cannot be called complete.
#[derive(Debug)]
pub(crate) struct ModuleInputs {
    toolchain: Option<String>,
    manifest: Option<Vec<ExecutionInputDraft>>,
    incomplete: Vec<String>,
}

/// The coordinates of `execution` and the mode it is run in, when this runner and
/// `runner_kind` are the ones that describe it. The mode is read from the runner
/// kind alone, which is all the Evidence of a run keeps (DES-097).
fn described<'a>(
    execution: &'a vtest_model::ExecutionDescriptor,
    runner_kind: &str,
) -> Option<(Coordinates<'a>, RunMode)> {
    let mode = RunMode::of_kind(runner_kind)?;
    Some((coordinates(execution).ok()?, mode))
}

/// The snapshot of `execution`, and why it is incomplete when it is
/// (DES-207): `None` when `runner_kind` or the descriptor is not one this
/// runner describes.
pub(crate) fn snapshot_with_reasons(
    runner: &GoTestRunner,
    root: &Path,
    execution: &vtest_model::ExecutionDescriptor,
    head_revision: Option<&str>,
    runner_kind: &str,
) -> Option<(ExecutionStateDraft, Vec<String>)> {
    let (coordinates, mode) = described(execution, runner_kind)?;
    let inputs = runner.collect_module(root, coordinates.module);
    let cgo = runner.cgo_reason(root, &coordinates);
    Some(runner.draft(&coordinates, mode, head_revision, &inputs, cgo))
}

/// [`snapshot_with_reasons`] without the reasons.
pub(crate) fn snapshot(
    runner: &GoTestRunner,
    root: &Path,
    execution: &vtest_model::ExecutionDescriptor,
    head_revision: Option<&str>,
    runner_kind: &str,
) -> Option<ExecutionStateDraft> {
    snapshot_with_reasons(runner, root, execution, head_revision, runner_kind)
        .map(|(draft, _)| draft)
}

/// DES-369, DES-370: the canonical invocation of the commands a Test is run
/// with in `mode`, as they are started: no machine path (the working directory is
/// the module's place in the project, and the profiles are stood for by
/// markers), and, for a measured run, both the Test's own command and the run
/// that selects no Test (DES-675).
fn invocation(
    coordinates: &Coordinates<'_>,
    mode: RunMode,
    environment: &BTreeMap<String, String>,
) -> Value {
    let mut invocation = json!({
        "program": PROGRAM,
        "args": match mode {
            RunMode::Plain => invocation_args(coordinates),
            RunMode::Measured => measured_args(coordinates, PROFILE_MARKER),
        },
        "working_directory": coordinates.module,
        "env": environment,
    });
    if mode == RunMode::Measured {
        invocation["baseline_args"] = json!(baseline_args(coordinates, BASELINE_PROFILE_MARKER));
    }
    invocation
}

/// The reader of one operation that checks many Tests against a project it does
/// not change (`vtest verify`): the files and the toolchain of a module are read
/// once, and the packages of a build once per package (DES-372).
pub(crate) struct GoExecutionStateReader<'a> {
    runner: &'a GoTestRunner,
    modules: RefCell<BTreeMap<(PathBuf, String), Rc<ModuleInputs>>>,
    cgo: RefCell<BTreeMap<(PathBuf, String, String), Option<String>>>,
}

impl<'a> GoExecutionStateReader<'a> {
    pub(crate) fn new(runner: &'a GoTestRunner) -> Self {
        Self {
            runner,
            modules: RefCell::new(BTreeMap::new()),
            cgo: RefCell::new(BTreeMap::new()),
        }
    }
}

impl ExecutionStateReader for GoExecutionStateReader<'_> {
    fn execution_state(
        &self,
        root: &Path,
        execution: &vtest_model::ExecutionDescriptor,
        head_revision: Option<&str>,
        runner_kind: &str,
    ) -> Option<ExecutionStateDraft> {
        let (coordinates, mode) = described(execution, runner_kind)?;
        let module_key = (root.to_owned(), coordinates.module.to_owned());
        let inputs = Rc::clone(
            self.modules
                .borrow_mut()
                .entry(module_key.clone())
                .or_insert_with(|| Rc::new(self.runner.collect_module(root, coordinates.module))),
        );
        let cgo = self
            .cgo
            .borrow_mut()
            .entry((module_key.0, module_key.1, coordinates.suite.to_owned()))
            .or_insert_with(|| self.runner.cgo_reason(root, &coordinates))
            .clone();
        Some(
            self.runner
                .draft(&coordinates, mode, head_revision, &inputs, cgo)
                .0,
        )
    }
}

impl GoTestRunner {
    /// Reads what every Test of the module `module` shares.
    fn collect_module(&self, root: &Path, module: &str) -> ModuleInputs {
        let mut incomplete = Vec::new();
        let root_comparable = comparable(root);
        let module_dir = root.join(module);
        if !comparable(&module_dir).starts_with(&root_comparable) {
            incomplete.push(format!(
                "the Go module {module} is outside the project root (DES-626)"
            ));
        }
        let toolchain = if let Some(env) = self.go_env(&module_dir) {
            Some(self.bind_toolchain(&env, &module_dir, &root_comparable, &mut incomplete))
        } else {
            incomplete.push(format!(
                "`go env` cannot be read in {module}, so the toolchain is unknown (DES-624)"
            ));
            None
        };
        let manifest = collect_manifest(root, &root_comparable, &mut incomplete);
        if manifest.is_none() {
            incomplete
                .push("the files under the project root cannot all be read (DES-623)".to_owned());
        }
        ModuleInputs {
            toolchain,
            manifest,
            incomplete,
        }
    }

    /// DES-629: a reason when a package of the Test's build that is not part of
    /// the standard library uses cgo, or when the packages cannot be listed.
    ///
    /// `-buildvcs=false` keeps `go list` from asking the version control system
    /// about the checkout, which can fail (a repository owned by another user, a
    /// `.git` that is not a repository) without that having anything to do with
    /// which packages the build holds.
    fn cgo_reason(&self, root: &Path, coordinates: &Coordinates<'_>) -> Option<String> {
        let package = format!("./{}", coordinates.suite);
        let listing = self.go(
            &root.join(coordinates.module),
            &[
                "list",
                "-buildvcs=false",
                "-deps",
                "-test",
                "-json=ImportPath,Standard,CgoFiles",
                &package,
            ],
        );
        let Some(listing) = listing else {
            return Some(format!(
                "the packages of the build of {package} cannot be listed, so cgo cannot be ruled \
                 out (DES-629)"
            ));
        };
        for item in serde_json::Deserializer::from_str(&listing).into_iter::<Value>() {
            let Ok(item) = item else {
                return Some("the package listing cannot be read (DES-629)".to_owned());
            };
            let standard = item.get("Standard").and_then(Value::as_bool) == Some(true);
            let uses_cgo = item
                .get("CgoFiles")
                .and_then(Value::as_array)
                .is_some_and(|files| !files.is_empty());
            if uses_cgo && !standard {
                let name = item
                    .get("ImportPath")
                    .and_then(Value::as_str)
                    .unwrap_or("a package");
                return Some(format!(
                    "{name} is not in the standard library and uses cgo (DES-629)"
                ));
            }
        }
        None
    }

    /// The draft for `coordinates` run in `mode`, and why it is incomplete when
    /// it is.
    fn draft(
        &self,
        coordinates: &Coordinates<'_>,
        mode: RunMode,
        head_revision: Option<&str>,
        inputs: &ModuleInputs,
        cgo: Option<String>,
    ) -> (ExecutionStateDraft, Vec<String>) {
        let mut reasons = inputs.incomplete.clone();
        reasons.extend(cgo);
        if head_revision.is_none() {
            reasons.push("the HEAD revision is unknown (DES-097)".to_owned());
        }
        let complete =
            reasons.is_empty() && inputs.toolchain.is_some() && inputs.manifest.is_some();
        let draft = ExecutionStateDraft {
            schema_id: SCHEMA_ID.to_owned(),
            schema_version: SCHEMA_VERSION.to_owned(),
            complete,
            head_revision: head_revision.map(str::to_owned),
            runner_kind: mode.kind().to_owned(),
            invocation: invocation(coordinates, mode, &self.environment),
            toolchain_identity: inputs.toolchain.clone().unwrap_or_default(),
            // No adapter configuration changes what `go test` does; what does is
            // the environment, which the toolchain identity binds (DES-624).
            effective_config: json!({}),
            inputs: inputs.manifest.clone().unwrap_or_default(),
        };
        (draft, reasons)
    }

    /// Runs `go args` in `dir` under the runner's environment and returns its
    /// stdout, or `None` when it did not run, failed, or printed non-UTF-8.
    fn go(&self, dir: &Path, args: &[&str]) -> Option<String> {
        let output = Command::new(PROGRAM)
            .current_dir(dir)
            .envs(&self.environment)
            .args(args)
            .output()
            .ok()?;
        if !output.status.success() {
            return None;
        }
        String::from_utf8(output.stdout).ok()
    }

    /// `go args` whose stdout is one JSON value.
    fn go_json(&self, dir: &Path, args: &[&str]) -> Option<Value> {
        serde_json::from_str(&self.go(dir, args)?).ok()
    }

    /// The effective values of the variables DES-624 binds, as `go env` reports
    /// them in `dir` (`go env -w`, `GOENV` and `$GOROOT/go.env` included), plus
    /// `GOWORK` and `GOMOD`.
    fn go_env(&self, dir: &Path) -> Option<BTreeMap<String, String>> {
        let mut args = vec!["env", "-json"];
        args.extend(BOUND_VARIABLES);
        args.extend(INSTRUCTION_SET_VARIABLES);
        args.extend(["GOWORK", "GOMOD"]);
        let Value::Object(values) = self.go_json(dir, &args)? else {
            return None;
        };
        values
            .into_iter()
            .map(|(name, value)| Some((name, value.as_str()?.to_owned())))
            .collect()
    }

    /// DES-624: the toolchain identity, and with it the checks of what the
    /// toolchain and the module file reach outside the project root
    /// (DES-626 to DES-631, DES-680), whose findings go to `incomplete`.
    fn bind_toolchain(
        &self,
        env: &BTreeMap<String, String>,
        module_dir: &Path,
        root: &Path,
        incomplete: &mut Vec<String>,
    ) -> String {
        let value = |name: &str| env.get(name).map_or("", String::as_str);

        // DES-631 and DES-626: the build is a module build, and its go.mod is
        // inside the project root.
        let go_mod = value("GOMOD");
        if go_mod.is_empty() || go_mod == "/dev/null" || go_mod.eq_ignore_ascii_case("NUL") {
            incomplete.push(
                "the build is not a module build (GOPATH mode), so its inputs are not listed \
                 (DES-631)"
                    .to_owned(),
            );
        } else if !comparable(Path::new(go_mod)).starts_with(root) {
            incomplete.push(format!(
                "the go.mod of the build, {go_mod}, is outside the project root (DES-626)"
            ));
        }

        let modfile = check_goflags(
            &split_goflags(value("GOFLAGS")),
            module_dir,
            root,
            incomplete,
        );
        self.check_go_mod(
            modfile.as_deref().unwrap_or("go.mod"),
            module_dir,
            root,
            incomplete,
        );
        let go_work = self.check_go_work(value("GOWORK"), root, incomplete);

        let mut lines: Vec<String> = BOUND_VARIABLES
            .iter()
            .map(|name| format!("{name}={}", value(name)))
            .collect();
        if let Some(variable) = instruction_set_variable(value("GOARCH")) {
            lines.push(format!("{variable}={}", value(variable)));
        }
        lines.push(format!("GOWORK={go_work}"));
        // DES-624, DES-680: the C compiler of a cgo build is part of the
        // toolchain; when its version cannot be had, the toolchain is not known.
        if value("CGO_ENABLED") == "1" {
            match self.cc_version(value("CC"), module_dir) {
                Some(version) => lines.push(format!("CC_VERSION={version}")),
                None => incomplete.push(format!(
                    "CGO_ENABLED is 1 and `{} --version` gave no output, so the C compiler is \
                     unknown (DES-680)",
                    value("CC")
                )),
            }
        }
        lines.sort();
        lines.join("\n")
    }

    /// The output of `<cc> --version`, or `None` when it cannot be run.
    fn cc_version(&self, cc: &str, dir: &Path) -> Option<String> {
        let mut words = cc.split_whitespace();
        let program = words.next()?;
        let output = Command::new(program)
            .args(words)
            .arg("--version")
            .current_dir(dir)
            .envs(&self.environment)
            .output()
            .ok()?;
        if !output.status.success() {
            return None;
        }
        let text = String::from_utf8(output.stdout).ok()?;
        let text = text.trim();
        (!text.is_empty()).then(|| text.to_owned())
    }

    /// DES-627: a `replace` of the module file that names a local path outside
    /// the project root.
    fn check_go_mod(
        &self,
        modfile: &str,
        module_dir: &Path,
        root: &Path,
        incomplete: &mut Vec<String>,
    ) {
        match self.go_json(module_dir, &["mod", "edit", "-json", modfile]) {
            Some(parsed) => check_local_replaces(&parsed, module_dir, root, modfile, incomplete),
            None => incomplete.push(format!(
                "{modfile} cannot be read as a module file, so its replacements are unknown \
                 (DES-627)"
            )),
        }
    }

    /// DES-627, DES-628: the workspace file `go_work` as `go env GOWORK` names
    /// it. Returns its place for the identity: the project-relative path, or
    /// `off` / nothing as `go` reports them.
    fn check_go_work(&self, go_work: &str, root: &Path, incomplete: &mut Vec<String>) -> String {
        if go_work.is_empty() || go_work == "off" {
            return go_work.to_owned();
        }
        let file = comparable(Path::new(go_work));
        let Ok(relative) = file.strip_prefix(root) else {
            incomplete.push(format!(
                "the workspace file {go_work} is outside the project root (DES-628)"
            ));
            return "<outside the project root>".to_owned();
        };
        let place = relative.to_string_lossy().replace('\\', "/");
        let directory = file.parent().unwrap_or(&file).to_owned();
        match self.go_json(&directory, &["work", "edit", "-json", go_work]) {
            Some(parsed) => {
                for used in parsed
                    .get("Use")
                    .and_then(Value::as_array)
                    .into_iter()
                    .flatten()
                {
                    let disk_path = used.get("DiskPath").and_then(Value::as_str).unwrap_or("");
                    if disk_path.is_empty()
                        || !comparable(&directory.join(disk_path)).starts_with(root)
                    {
                        incomplete.push(format!(
                            "the workspace uses `{disk_path}`, which is outside the project root \
                             (DES-628)"
                        ));
                    }
                }
                check_local_replaces(&parsed, &directory, root, &place, incomplete);
            }
            None => incomplete.push(format!(
                "{go_work} cannot be read as a workspace file, so its directories are unknown \
                 (DES-628)"
            )),
        }
        place
    }
}

/// DES-630: `GOFLAGS` flags that take a file or a directory outside the
/// project root, and `-exec` and `-toolexec`, which name a program that can
/// change the build and the run and cannot be listed. Returns the last
/// `-modfile`, which is the module file `go` reads.
fn check_goflags(
    flags: &[String],
    module_dir: &Path,
    root: &Path,
    incomplete: &mut Vec<String>,
) -> Option<String> {
    let mut modfile = None;
    for flag in flags {
        let body = flag.trim_start_matches('-');
        let (name, value) = match body.split_once('=') {
            Some((name, value)) => (name, Some(value)),
            None => (body, None),
        };
        if matches!(name, "exec" | "toolexec") {
            incomplete.push(format!(
                "GOFLAGS has -{name}, whose program can change the build and the run and cannot \
                 be listed (DES-630)"
            ));
        }
        let Some(value) = value.filter(|_| PATH_FLAGS.contains(&name)) else {
            continue;
        };
        // `-pgo` is `auto`, `off`, or a profile.
        if name == "pgo" && matches!(value, "auto" | "off") {
            continue;
        }
        if !comparable(&module_dir.join(value)).starts_with(root) {
            incomplete.push(format!(
                "GOFLAGS has -{name}={value}, outside the project root (DES-630)"
            ));
        }
        if name == "modfile" {
            modfile = Some(value.to_owned());
        }
    }
    modfile
}

/// DES-627: every `replace` in `parsed` (the JSON of `go mod edit` or `go work
/// edit`) whose right side is a local path, which `go` writes without a
/// version, must stay inside the project root.
fn check_local_replaces(
    parsed: &Value,
    directory: &Path,
    root: &Path,
    from: &str,
    incomplete: &mut Vec<String>,
) {
    for replace in parsed
        .get("Replace")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
    {
        let new = replace.get("New");
        let version = new
            .and_then(|new| new.get("Version"))
            .and_then(Value::as_str)
            .unwrap_or("");
        if !version.is_empty() {
            continue;
        }
        let path = new
            .and_then(|new| new.get("Path"))
            .and_then(Value::as_str)
            .unwrap_or("");
        if path.is_empty() || !comparable(&directory.join(path)).starts_with(root) {
            incomplete.push(format!(
                "{from} replaces a module with the local path `{path}`, outside the project root \
                 (DES-627)"
            ));
        }
    }
}

/// `GOFLAGS` split into its flags: on white space, with single or double quotes
/// around a flag allowed and no unescaping inside them, as `go` splits it.
fn split_goflags(value: &str) -> Vec<String> {
    let mut flags = Vec::new();
    let mut current = String::new();
    let mut started = false;
    let mut quote = None;
    for character in value.chars() {
        match quote {
            Some(open) if character == open => quote = None,
            Some(_) => current.push(character),
            None if character == '\'' || character == '"' => {
                quote = Some(character);
                started = true;
            }
            None if character.is_whitespace() => {
                if started || !current.is_empty() {
                    flags.push(std::mem::take(&mut current));
                    started = false;
                }
            }
            None => current.push(character),
        }
    }
    if started || !current.is_empty() {
        flags.push(current);
    }
    flags
}

/// DES-623: every regular file under the project root, byte for byte, except
/// what is under `.git/` and `.verify/` at the root. A symbolic link is not a
/// regular file: one that leaves the project root (or cannot be resolved) makes
/// the snapshot incomplete (DES-632), and one that stays inside it names files
/// the walk lists under their own paths. `None` when any entry cannot be read,
/// because an unreadable file must not silently drop out of the subject.
fn collect_manifest(
    root: &Path,
    root_comparable: &Path,
    incomplete: &mut Vec<String>,
) -> Option<Vec<ExecutionInputDraft>> {
    let mut entries = Vec::new();
    walk(root, root, root_comparable, &mut entries, incomplete)?;
    // DES-099: the manifest is in normalized identity order.
    entries.sort_by(|a, b| a.root_relative_path.cmp(&b.root_relative_path));
    Some(entries)
}

fn walk(
    root: &Path,
    directory: &Path,
    root_comparable: &Path,
    entries: &mut Vec<ExecutionInputDraft>,
    incomplete: &mut Vec<String>,
) -> Option<()> {
    for entry in fs::read_dir(directory).ok()? {
        let entry = entry.ok()?;
        let path = entry.path();
        if directory == root
            && EXCLUDED_ROOT_ENTRIES
                .iter()
                .any(|name| entry.file_name() == *name)
        {
            continue;
        }
        let file_type = entry.file_type().ok()?;
        if file_type.is_symlink() {
            match fs::canonicalize(&path).map(strip_verbatim) {
                Ok(target) if target.starts_with(root_comparable) => {}
                Ok(target) => incomplete.push(format!(
                    "{} is a symbolic link to {}, outside the project root (DES-632)",
                    path.display(),
                    target.display()
                )),
                Err(error) => incomplete.push(format!(
                    "{} is a symbolic link that cannot be resolved: {error} (DES-632)",
                    path.display()
                )),
            }
        } else if file_type.is_dir() {
            walk(root, &path, root_comparable, entries, incomplete)?;
        } else if file_type.is_file() {
            let relative = path.strip_prefix(root).ok()?;
            entries.push(ExecutionInputDraft {
                root_identity: ROOT_IDENTITY.to_owned(),
                root_relative_path: relative.to_string_lossy().replace('\\', "/"),
                // DES-098: the one kind of manifest input is a file.
                kind: "file".to_owned(),
                bytes: fs::read(&path).ok()?,
            });
        }
    }
    Some(())
}

/// `path` in a form two paths can be compared in: resolved when it exists,
/// otherwise made absolute and cleaned of `.` and `..`, and without the
/// verbatim prefix Windows adds when it resolves.
pub(crate) fn comparable(path: &Path) -> PathBuf {
    match fs::canonicalize(path) {
        Ok(resolved) => strip_verbatim(resolved),
        Err(_) => lexical(&std::path::absolute(path).unwrap_or_else(|_| path.to_owned())),
    }
}

/// `path` without `.` components, with `..` applied to what precedes it.
fn lexical(path: &Path) -> PathBuf {
    let mut cleaned = PathBuf::new();
    for component in path.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                cleaned.pop();
            }
            other => cleaned.push(other.as_os_str()),
        }
    }
    cleaned
}

/// `\\?\C:\x` as `C:\x`: the prefix Windows puts on a resolved path, which a
/// path from `go` does not have.
fn strip_verbatim(path: PathBuf) -> PathBuf {
    let text = path.to_string_lossy();
    match text.strip_prefix(r"\\?\") {
        Some(rest) if rest.chars().nth(1) == Some(':') => PathBuf::from(rest),
        _ => path,
    }
}

#[cfg(test)]
mod tests;
