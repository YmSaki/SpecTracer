//! Golden test of the test specification core (#68): a fixed `rust-cargo`
//! project is scanned through the built-in registry, the data comes from
//! `vtest_scan::test_specification`, and the Markdown document from
//! `vtest_cli::test_spec_markdown`. The CLI and MCP entries are not part of
//! this test.
//!
//! To regenerate the golden files after an intended change, run this test
//! with `VTEST_UPDATE_GOLDEN=1` and review the diff.

mod support;

use std::{env, fs, path::PathBuf};

use vtest_cli::test_spec_markdown::render_test_specification;
use vtest_scan::{scan_project, test_specification, ScanResult};

const CARGO_TOML: &str = "[package]\nname = \"fixture\"\nversion = \"0.1.0\"\nedition = \"2021\"\n";

const LIB_RS: &str = r#"/// @vtest.src-id SRC-PARSE
pub fn parse() {}

/// @vtest.src-id SRC-TWICE
pub fn render() {}

/// @vtest.src-id SRC-TWICE
pub fn other() {}

#[cfg(test)]
mod tests {
    /// @vtest.id TEST-PARSE
    /// @vtest.covers VO-PARSE, VO-GONE
    /// @vtest.target SRC-PARSE
    /// @vtest.target src/lib.rs::missing
    /// @vtest.intent parse accepts `a|b` and *c*
    /// @vtest.input a # b
    /// @vtest.expect <ok>
    /// @vtest.kind unit-normal
    /// @vtest.case first
    /// @vtest.case second [x](y)
    /// @vtest.related TEST-RENDER, TEST-NOWHERE, TEST-TWICE
    #[test]
    fn parse_accepts() {
        super::parse();
    }

    /// @vtest.id TEST-RENDER
    /// @vtest.covers VO-RENDER
    /// @vtest.target src/lib.rs::render
    /// @vtest.target SRC-TWICE
    /// @vtest.intent render returns
    #[test]
    fn render_returns() {
        super::render();
    }

    /// @vtest.id TEST-TWICE
    /// @vtest.covers VO-PARSE
    /// @vtest.intent first of two
    #[test]
    fn twice_one() {}

    /// @vtest.id TEST-TWICE
    /// @vtest.covers VO-PARSE
    /// @vtest.intent second of two
    #[test]
    fn twice_two() {}

    #[test]
    fn no_declaration() {}

    /// @vtest.id TEST-INCOMPLETE
    /// @vtest.intent has no covers
    #[test]
    fn incomplete_declaration() {}
}
"#;

fn scan_fixture() -> (support::Fixture, ScanResult) {
    let fixture = support::fixture(
        "rust-cargo",
        &[("Cargo.toml", CARGO_TOML), ("src/lib.rs", LIB_RS)],
        &["VO-PARSE", "VO-RENDER"],
    );
    let registry = vtest_ops::registry::builtin_registry().expect("builtin registry");
    let scan = scan_project(fixture.path(), &registry).expect("scan completes");
    (fixture, scan)
}

fn golden(name: &str, actual: &str) {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/golden/test_spec_core")
        .join(name);
    if env::var_os("VTEST_UPDATE_GOLDEN").is_some() {
        fs::write(&path, actual).expect("write golden file");
    }
    let expected = fs::read_to_string(&path).expect("read golden file");
    assert_eq!(actual, expected, "{name} differs from the golden file");
}

/// @vtest.id TEST-CLI-TEST-SPEC-CORE-MATCHES-GOLDEN-FOR-FIXED-PROJECT
/// @vtest.covers VO-SCAN-TEST-SPEC-BASIS-FROM-SCAN-VALUES
/// @vtest.target crates/vtest-scan/src/test_spec.rs::test_specification
/// @vtest.target crates/vtest-cli/src/test_spec_markdown.rs::render_test_specification
/// @vtest.intent for a fixed rust-cargo project with managed Tests, a shared Test ID, a Test without a declaration, an incomplete declaration, a SRC ID declared twice and unresolved covers, target and related references, the data built from the scan and the Markdown document built from that data equal the golden files (DES-600, DES-601)
#[test]
fn the_core_output_of_a_fixed_project_matches_the_golden_files() {
    let (_fixture, scan) = scan_fixture();

    let data = test_specification(&scan);
    let mut json = serde_json::to_string_pretty(&data).expect("serialize data");
    json.push('\n');

    golden("data.json", &json);
    golden("document.md", &render_test_specification(&data));
}

/// @vtest.id TEST-CLI-TEST-SPEC-CORE-SAME-OUTPUT-FOR-REPEATED-AND-REORDERED-SCANS
/// @vtest.covers VO-CLI-TEST-SPEC-NO-ENVIRONMENT-DEPENDENT-VALUES
/// @vtest.target crates/vtest-scan/src/test_spec.rs::test_specification
/// @vtest.target crates/vtest-cli/src/test_spec_markdown.rs::render_test_specification
/// @vtest.intent two scans of the same project in different temporary directories, and the second scan with its Tests, Discovered Tests and basis lists reversed, give byte-identical data and byte-identical Markdown, so neither holds an absolute path or another value of the run (DS-1738)
#[test]
fn repeated_and_reordered_scans_give_identical_output() {
    let (_first_fixture, first) = scan_fixture();
    let (_second_fixture, mut second) = scan_fixture();
    second.tests.reverse();
    second.discovered.reverse();
    second.test_spec_basis.construct_codes.reverse();
    second.test_spec_basis.references.reverse();

    let first_data = test_specification(&first);
    let second_data = test_specification(&second);

    assert_eq!(
        serde_json::to_string(&first_data).expect("serialize data"),
        serde_json::to_string(&second_data).expect("serialize data")
    );
    assert_eq!(
        render_test_specification(&first_data),
        render_test_specification(&second_data)
    );
    assert!(!first_data.tests.is_empty());
}
