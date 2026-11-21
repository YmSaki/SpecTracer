//! The `go` adapter (BD-350, BD-357).
//!
//! Responsibility: own everything Go-specific that is not the lexical reading of
//! Go source. BD-357 gives the `go` adapter its discovery, static audit, runner
//! and coverage; AGENTS.md 9.2 places the toolchain parts (`go test`, Execution
//! State) in this crate, as `vtest-adapter-rust` does for `rust-cargo`.
//!
//! Boundary: discovery is delegated to `vtest_adapter_lexical::go`, which keeps
//! the `@vtest.` grammar and the Go syntax in one place (AGENTS.md 4.2, 11). This
//! is the one adapter-to-adapter dependency, and it is for discovery only. At
//! this stage the crate declares the `source_discovery` capability and nothing
//! else, so it reports exactly what the lexical `go` adapter reported before the
//! crate existed.

use vtest_adapter_api::{Adapter, AdapterDescriptor, SourceDiscoveryAdapter};

pub use vtest_adapter_lexical::go::ADAPTER_ID;

/// The `go` adapter. Discovery is the lexical `go` module's.
#[derive(Clone, Copy, Debug, Default)]
pub struct GoAdapter {
    discovery: vtest_adapter_lexical::go::GoAdapter,
}

/// The `go` adapter the composition root registers.
pub fn adapter() -> GoAdapter {
    GoAdapter::default()
}

impl Adapter for GoAdapter {
    fn descriptor(&self) -> AdapterDescriptor {
        self.discovery.descriptor()
    }

    fn as_source_discovery(&self) -> Option<&dyn SourceDiscoveryAdapter> {
        Some(&self.discovery)
    }
}

#[cfg(test)]
mod tests {
    use std::{
        fs,
        path::PathBuf,
        sync::atomic::{AtomicU64, Ordering},
    };

    use vtest_adapter_api::{AdapterRegistry, AdapterScanConfig};

    use super::*;

    static NEXT: AtomicU64 = AtomicU64::new(0);

    const TEST_FILE: &str = "package calc\n\nimport \"testing\"\n\n// @vtest.id TEST-GO-CRATE-ADD\n// @vtest.covers VO-GO-CRATE\n// @vtest.target calc/calc.go::Add\n// @vtest.intent adding returns the sum\nfunc TestAdd(t *testing.T) {\n\tif Add(1, 2) != 3 {\n\t\tt.Fatal(\"sum\")\n\t}\n}\n\nfunc TestUndeclared(t *testing.T) {}\n";
    const SOURCE_FILE: &str = "package calc\n\nfunc Add(a, b int) int { return a + b }\n";

    fn fixture() -> PathBuf {
        let root = std::env::temp_dir().join(format!(
            "vtest-adapter-go-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(root.join("calc")).expect("create fixture directory");
        fs::write(root.join("calc/calc.go"), SOURCE_FILE).expect("write source");
        fs::write(root.join("calc/calc_test.go"), TEST_FILE).expect("write test");
        root
    }

    /// The crate registers under the `go` id with exactly the capability set the
    /// lexical `go` adapter declared, so moving the registration changes nothing
    /// a caller can observe.
    /// @vtest.id TEST-ADAPTER-GO-DESCRIPTOR-IS-THE-LEXICAL-GO-DESCRIPTOR
    /// @vtest.covers VO-ADAPTER-GO-REGISTERS-WITHOUT-CORE-CHANGES
    /// @vtest.target crates/vtest-adapter-go/src/lib.rs::adapter
    /// @vtest.intent the go adapter of its own crate registers under the id go, declares only source_discovery like the lexical go adapter, and resolves through the registry
    #[test]
    fn the_go_crate_registers_with_the_descriptor_the_lexical_go_adapter_had() {
        let lexical = vtest_adapter_lexical::go::adapter().descriptor();
        let own = adapter().descriptor();
        assert_eq!(format!("{own:?}"), format!("{lexical:?}"));
        assert_eq!(own.id, "go");

        let mut registry = AdapterRegistry::new();
        registry
            .register(Box::new(adapter()))
            .expect("the go adapter registers");
        assert!(registry.source_discovery("go").is_some());
        assert!(registry.test_runner("go").is_none());
        assert!(registry.coverage("go").is_none());
        assert!(registry.static_analysis("go").is_none());
    }

    /// @vtest.id TEST-ADAPTER-GO-DISCOVERY-IS-THE-LEXICAL-GO-DISCOVERY
    /// @vtest.covers VO-ADAPTER-GO-REGISTERS-WITHOUT-CORE-CHANGES
    /// @vtest.target crates/vtest-adapter-go/src/lib.rs::adapter
    /// @vtest.intent discovery through the go adapter of its own crate returns the same Tests, Source Targets and diagnostics as the lexical go adapter on one project
    #[test]
    fn discovery_through_the_go_crate_equals_the_lexical_go_discovery() {
        let root = fixture();
        let config = AdapterScanConfig {
            include_paths: vec![PathBuf::from(".")],
        };
        let own = adapter()
            .as_source_discovery()
            .expect("source discovery")
            .discover(&root, "fallback", &config)
            .expect("own discovery");
        let lexical = vtest_adapter_lexical::go::adapter()
            .discover(&root, "fallback", &config)
            .expect("lexical discovery");
        let _ = fs::remove_dir_all(&root);
        assert_eq!(own.tests.len(), 1, "{own:?}");
        assert_eq!(own.missing_tests.len(), 1, "{own:?}");
        assert_eq!(format!("{own:?}"), format!("{lexical:?}"));
    }
}
