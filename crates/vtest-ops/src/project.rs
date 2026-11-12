//! The step several operations start with: a loadable config, the built-in
//! registry and a scan of the project.

use std::path::Path;

use vtest_adapter_api::AdapterRegistry;
use vtest_scan::{scan_project, ScanResult};

use crate::envelope::{config_failure, scan_failure, Failure};

/// Loads the config, composes the registry and scans `root`, in that order,
/// so a project with several problems reports the same one from the CLI and
/// from MCP: an unloadable config first (E-CONFIG-001, or E-INTERNAL-001
/// when the file cannot be read), then the registry (E-ADAPTER-001), then
/// the scan's own rejection.
pub fn load_scanned(root: &Path) -> Result<(ScanResult, AdapterRegistry), Failure> {
    vtest_store::load_config(root).map_err(|error| config_failure(&error))?;
    let registry = crate::registry::resolve_registry()?;
    let scan = scan_project(root, &registry).map_err(|error| scan_failure(&error))?;
    Ok((scan, registry))
}

#[cfg(test)]
mod tests {
    use std::{fs, path::PathBuf};

    use vtest_model::ExitCode;

    use super::load_scanned;

    fn project(name: &str) -> PathBuf {
        let root = std::env::temp_dir().join(format!(
            "vtest-ops-load-scanned-{name}-{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();
        vtest_store::init_project(&root, "load-scanned").unwrap();
        root
    }

    /// @vtest.id TEST-OPS-LOAD-SCANNED-CONFIG-FAILURE
    /// @vtest.covers VO-INTERNAL-ERROR-CODE-CLI-MCP
    /// @vtest.target crates/vtest-ops/src/project.rs::load_scanned
    /// @vtest.intent a config file that cannot be read is the E-INTERNAL-001 failure with exit 3 and a config the store rejects is the E-CONFIG-001 failure with exit 2, each with a failure envelope carrying no data, while a fresh project yields its scan and the built-in registry
    /// @vtest.case unreadable config
    /// @vtest.case rejected config
    /// @vtest.case loadable config
    #[test]
    fn load_scanned_reports_the_config_failure_before_scanning() {
        let unreadable = project("unreadable");
        let config = unreadable.join(".verify/config.yaml");
        fs::remove_file(&config).unwrap();
        fs::create_dir(&config).unwrap();
        let failure = load_scanned(&unreadable).err().expect("must be rejected");
        assert_eq!(failure.exit, ExitCode::Internal);
        assert_eq!(failure.diagnostic.code, vtest_model::INTERNAL_ERROR_CODE);
        let envelope = failure.envelope();
        assert_eq!(envelope["ok"], false);
        assert_eq!(envelope["data"], serde_json::Value::Null);
        assert_eq!(
            envelope["diagnostics"],
            serde_json::json!([{
                "code": "E-INTERNAL-001",
                "severity": "error",
                "message": failure.diagnostic.message,
            }])
        );

        let rejected = project("rejected");
        let config = rejected.join(".verify/config.yaml");
        let mut text = fs::read_to_string(&config).unwrap();
        text.push_str("not_a_config_field: 1\n");
        fs::write(&config, text).unwrap();
        let failure = load_scanned(&rejected).err().expect("must be rejected");
        assert_eq!(failure.exit, ExitCode::Usage);
        assert_eq!(failure.diagnostic.code, "E-CONFIG-001");
        assert_eq!(failure.result(), (ExitCode::Usage, failure.envelope()));

        let loadable = project("loadable");
        let (scan, registry) = load_scanned(&loadable).expect("a fresh project scans");
        assert!(scan.tests.is_empty());
        assert!(registry.get("rust-cargo").is_some());
    }
}
