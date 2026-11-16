//! Canonical `export` operation, shared between the CLI's `run_export`
//! wrapper and `vtest-mcp`'s `export` tool handler (BD-337).
//!
//! The operation scans, builds the test specification data, decides `ok`
//! and the exit code from the scan diagnostics alone, and orders the
//! diagnostics (DES-610 to DES-612). Choosing the output format, rendering
//! Markdown and writing to stdout / stderr stay in the CLI (BD-338).

use std::path::Path;

use vtest_adapter_api::AdapterRegistry;
use vtest_model::{Diagnostic, ExitCode};
use vtest_scan::{
    scan_project,
    test_spec::{comparison_string, test_specification, TestSpecification},
};
use vtest_store::load_config;

use crate::envelope::{config_failure, envelope_json, scan_failure, Failure};

/// What one `export` call produced: the exit code, the test specification
/// data when the scan completed, and the diagnostics in DS-1703 order.
#[derive(Debug)]
pub struct ExportOutcome {
    pub exit: ExitCode,
    pub data: Option<TestSpecification>,
    pub diagnostics: Vec<Diagnostic>,
}

impl ExportOutcome {
    /// DS-1699: `ok` is true for exit code 0 only.
    pub fn ok(&self) -> bool {
        self.exit == ExitCode::Ok
    }

    /// The common JSON envelope (DS-1701); `data` is `null` when the scan
    /// did not complete (DES-610).
    pub fn envelope(&self) -> serde_json::Value {
        envelope_json(self.ok(), &self.data, &self.diagnostics)
    }

    /// An export that was rejected before a scan completed: no data and the
    /// one diagnostic of the rejection.
    pub fn rejected(failure: &Failure) -> Self {
        Self {
            exit: failure.exit,
            data: None,
            diagnostics: vec![failure.diagnostic.clone()],
        }
    }
}

pub fn execute(root: &Path, registry: &AdapterRegistry) -> ExportOutcome {
    if let Err(error) = load_config(root) {
        return ExportOutcome::rejected(&config_failure(&error));
    }
    match scan_project(root, registry) {
        Ok(scan) => {
            // DES-611: the exit code comes from the severities alone.
            let exit = if scan.has_errors() {
                ExitCode::VerificationFailed
            } else {
                ExitCode::Ok
            };
            let data = test_specification(&scan);
            let mut diagnostics = scan.diagnostics;
            sort_diagnostics(&mut diagnostics);
            ExportOutcome {
                exit,
                data: Some(data),
                diagnostics,
            }
        }
        Err(error) => ExportOutcome::rejected(&scan_failure(&error)),
    }
}

/// DS-1703: orders by `code`, the location's `adapter` / `path` / `locator`
/// (empty strings without a location), `message`, then the comparison JSON
/// of the whole diagnostic.
pub fn sort_diagnostics(diagnostics: &mut [Diagnostic]) {
    diagnostics.sort_by_cached_key(|diagnostic| {
        let location = diagnostic.location.as_deref();
        (
            diagnostic.code.clone(),
            location.map_or_else(String::new, |location| location.adapter.as_str().to_owned()),
            location.map_or_else(String::new, |location| location.path.as_str().to_owned()),
            location.map_or_else(String::new, |location| location.locator.clone()),
            diagnostic.message.clone(),
            serde_json::to_value(diagnostic)
                .map(|value| comparison_json(&value))
                .unwrap_or_default(),
        )
    });
}

/// DS-1732: object keys in byte order, no whitespace, strings escaped as
/// DS-1765 says.
fn comparison_json(value: &serde_json::Value) -> String {
    let mut out = String::new();
    write_comparison_json(value, &mut out);
    out
}

fn write_comparison_json(value: &serde_json::Value, out: &mut String) {
    match value {
        serde_json::Value::String(text) => comparison_string(text, out),
        serde_json::Value::Array(items) => {
            out.push('[');
            for (index, item) in items.iter().enumerate() {
                if index > 0 {
                    out.push(',');
                }
                write_comparison_json(item, out);
            }
            out.push(']');
        }
        serde_json::Value::Object(fields) => {
            let mut keys: Vec<&String> = fields.keys().collect();
            keys.sort_unstable();
            out.push('{');
            for (index, key) in keys.into_iter().enumerate() {
                if index > 0 {
                    out.push(',');
                }
                comparison_string(key, out);
                out.push(':');
                write_comparison_json(&fields[key], out);
            }
            out.push('}');
        }
        other => out.push_str(&other.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use std::{fs, path::PathBuf};

    use vtest_model::{AdapterId, Diagnostic, ExitCode, ProjectPath, SourceLocation, SourceRange};

    use super::{execute, sort_diagnostics};

    fn located(code: &str, adapter: &str, path: &str, locator: &str, message: &str) -> Diagnostic {
        Diagnostic::error(code, message).with_location(SourceLocation {
            adapter: AdapterId::new(adapter),
            path: ProjectPath::new(path),
            locator: locator.to_owned(),
            byte_range: SourceRange { start: 0, end: 1 },
        })
    }

    /// @vtest.id TEST-OPS-EXPORT-DIAGNOSTIC-ORDER
    /// @vtest.covers VO-OPS-EXPORT-DIAGNOSTIC-ORDER
    /// @vtest.target crates/vtest-ops/src/export.rs::sort_diagnostics
    /// @vtest.intent the diagnostics of an export are ordered by code, then adapter, path and locator with a missing location compared as empty strings, then message, then the comparison JSON of the diagnostic, whatever order they arrive in
    /// @vtest.case reversed input
    /// @vtest.case same code, location and message, different candidates
    #[test]
    fn diagnostics_are_ordered_by_code_location_message_and_json() {
        let expected = vec![
            Diagnostic::error("E-SCAN-003", "no location"),
            located("E-SCAN-003", "a", "src/b.rs", "x", "m"),
            located("E-SCAN-003", "a", "src/b.rs", "y", "m"),
            located("E-SCAN-003", "a", "src/c.rs", "a", "m"),
            located("E-SCAN-003", "b", "src/a.rs", "a", "a"),
            // Same code, location and message: the comparison JSON decides,
            // and its `candidates` key sorts before `code`.
            located("E-SCAN-003", "b", "src/a.rs", "a", "b").with_candidates(vec!["x".to_owned()]),
            located("E-SCAN-003", "b", "src/a.rs", "a", "b").with_candidates(vec!["y".to_owned()]),
            located("E-SCAN-003", "b", "src/a.rs", "a", "b"),
            Diagnostic::warning("W-SCAN-101", "a"),
        ];

        let mut reversed: Vec<Diagnostic> = expected.iter().rev().cloned().collect();
        sort_diagnostics(&mut reversed);
        assert_eq!(reversed, expected);

        let mut rotated = expected.clone();
        rotated.rotate_left(4);
        sort_diagnostics(&mut rotated);
        assert_eq!(rotated, expected);
    }

    fn project(name: &str, lib: &str) -> PathBuf {
        let root =
            std::env::temp_dir().join(format!("vtest-ops-export-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(root.join("src")).unwrap();
        vtest_store::init_project(&root, "export").unwrap();
        fs::write(
            root.join("Cargo.toml"),
            "[package]\nname = \"export_fixture\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
        )
        .unwrap();
        fs::write(root.join("src/lib.rs"), lib).unwrap();
        root
    }

    const UNMANAGED: &str = "pub fn target() {}\n\n#[cfg(test)]\nmod tests {\n    #[test]\n    fn bare() {\n        super::target();\n    }\n}\n";

    const UNRESOLVED_COVERS: &str = "pub fn target() {}\n\n#[cfg(test)]\nmod tests {\n    \
         /// @vtest.id TEST-X\n    /// @vtest.covers VO-GONE\n    \
         /// @vtest.target src/lib.rs::target\n    /// @vtest.intent fixture\n    \
         #[test]\n    fn calls_target() {\n        super::target();\n    }\n}\n";

    /// @vtest.id TEST-OPS-EXPORT-EXIT-AND-DATA
    /// @vtest.covers VO-OPS-EXPORT-EXIT-CODE-FROM-SCAN-ERRORS, VO-OPS-EXPORT-NO-DATA-WHEN-SCAN-INCOMPLETE
    /// @vtest.target crates/vtest-ops/src/export.rs::execute
    /// @vtest.intent a completed scan yields the test specification with exit code 0 when only warnings exist and exit code 1 when an error diagnostic exists, and a rejected config yields exit code 2 with no data and a `data: null` envelope
    /// @vtest.case warning only
    /// @vtest.case unresolved covers
    /// @vtest.case rejected config
    #[test]
    fn exit_code_follows_scan_errors_and_a_rejected_scan_has_no_data() {
        let registry = crate::registry::builtin_registry().unwrap();

        let warning_only = execute(&project("warning", UNMANAGED), &registry);
        assert_eq!(warning_only.exit, ExitCode::Ok);
        assert!(warning_only.ok());
        let data = warning_only.data.as_ref().expect("data");
        assert!(data.tests.is_empty());
        assert_eq!(data.unmanaged.len(), 1);
        let codes: Vec<&str> = warning_only
            .diagnostics
            .iter()
            .map(|diagnostic| diagnostic.code.as_str())
            .collect();
        assert_eq!(codes, ["W-SCAN-101"]);
        assert_eq!(warning_only.envelope()["ok"], true);

        let unresolved = execute(&project("unresolved", UNRESOLVED_COVERS), &registry);
        assert_eq!(unresolved.exit, ExitCode::VerificationFailed);
        assert!(!unresolved.ok());
        let data = unresolved.data.as_ref().expect("data");
        assert_eq!(data.tests.len(), 1);
        assert_eq!(data.unresolved.len(), 1);
        assert!(unresolved
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "E-SCAN-003"));
        let envelope = unresolved.envelope();
        assert_eq!(envelope["ok"], false);
        assert_eq!(envelope["data"]["tests"][0]["id"], "TEST-X");

        let root = project("config", UNMANAGED);
        fs::write(root.join(".verify/config.yaml"), "version: 2\nproject: [\n").unwrap();
        let rejected = execute(&root, &registry);
        assert_eq!(rejected.exit, ExitCode::Usage);
        assert!(rejected.data.is_none());
        assert_eq!(rejected.diagnostics.len(), 1);
        assert_eq!(rejected.diagnostics[0].code, "E-CONFIG-001");
        let envelope = rejected.envelope();
        assert_eq!(envelope["ok"], false);
        assert_eq!(envelope["data"], serde_json::Value::Null);
    }
}
