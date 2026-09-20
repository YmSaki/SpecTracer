//! Acceptance coverage for 別紙C §18.3.8 verify・report と scope
//! (DES-548/549/550).
//!
//! DES-548's wire shape (`scope.requested.items` / `scope.requested.
//! entities` / `scope.unverified_outside_scope`) is asserted here literally
//! against `ops::verify::execute`'s JSON `data` — `vtest_verify::
//! ScopeReport` was renamed to this exact shape (previously
//! `requested_checks`/`entity`/`outside_scope_is_unverified`) so the
//! CLI/MCP wire output matches the spec's own field names rather than the
//! test asserting around a mismatch.
//!
//! DES-549 (tree-drawing branch symbols) and DES-550 (mutation-testing
//! seam) are not covered here — DES-550 in particular calls for a stub-
//! injection harness this closure-slice does not have.

use std::path::PathBuf;

use vtest_cli::ops;
use vtest_model::ExitCode;
use vtest_store::init_project;

fn registry() -> vtest_adapter_api::AdapterRegistry {
    vtest_cli::adapters::builtin_registry().expect("builtin registry must register cleanly")
}

fn temp_root(name: &str) -> PathBuf {
    // A nanosecond-timestamp suffix alone collides under parallel test
    // execution on Windows' coarser clock resolution -- see
    // `vtest-scan`'s `fixture()` doc comment for the confirmed root cause
    // of a previously-unconfirmed flaky failure elsewhere in this
    // workspace.
    static COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let sequence = COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let root = std::env::temp_dir().join(format!(
        "vtest-cli-acceptance-18-3-8-{name}-{}-{sequence}",
        std::process::id()
    ));
    std::fs::create_dir_all(&root).expect("create fixture root");
    root
}

/// DES-548's "`--items` 省略時は固定4検査を4件すべて列挙" and
/// "エンティティ軸無指定は空 list" halves, asserted against the literal
/// `scope.requested.items` / `scope.requested.entities` field names.
/// @vtest.id TEST-ACCEPTANCE-18-3-8-OMITTED-ITEMS
/// @vtest.covers VO-VERIFY-SCOPE-REQUESTED-WIRE-SHAPE
/// @vtest.target crates/vtest-cli/src/ops/verify.rs::execute
/// @vtest.intent omitting --items and the entity axis lists all four checks and an empty entities list in scope.requested
#[test]
fn omitted_items_and_entity_report_the_full_axis_in_scope_requested() {
    let root = temp_root("omitted-items");
    init_project(&root, "acceptance-18-3-8-fixture").expect("init .verify/ layout");
    let (exit, data, _diagnostics) =
        ops::verify::execute(&root, &[], None, None, None, None, false, &registry())
            .expect("verify must run to completion on an empty (non-scan-error) project");
    assert_eq!(
        exit,
        ExitCode::VerificationFailed,
        "an empty project is never a complete PASS"
    );
    assert_eq!(
        data.scope.requested.items,
        vec![
            vtest_model::VerificationCheck::ChainIntegrity,
            vtest_model::VerificationCheck::OrphanDetection,
            vtest_model::VerificationCheck::TargetBinding,
            vtest_model::VerificationCheck::OraclePresence,
        ],
        "DES-548: omitting --items must list all four checks in scope.requested.items"
    );
    assert!(
        data.scope.requested.entities.is_empty(),
        "DES-548: no entity axis given must mean scope.requested.entities is an empty list"
    );
    assert!(
        !data.scope.unverified_outside_scope,
        "no entity axis was requested, so nothing is outside the requested scope"
    );
}

/// DES-548's "エンティティ軸指定ありで `true`" half: a `--vo` scope
/// populates `scope.requested.entities` with exactly that entity's id and
/// sets `scope.unverified_outside_scope`.
/// @vtest.id TEST-ACCEPTANCE-18-3-8-ENTITY-SCOPE
/// @vtest.covers VO-VERIFY-SCOPE-REQUESTED-WIRE-SHAPE
/// @vtest.target crates/vtest-cli/src/ops/verify.rs::execute
/// @vtest.intent an entity-limited scope populates scope.requested.entities and sets unverified_outside_scope
#[test]
fn an_entity_scope_populates_requested_entities_and_marks_outside_scope_unverified() {
    let root = temp_root("entity-scope");
    init_project(&root, "acceptance-18-3-8-fixture").expect("init .verify/ layout");
    let (_exit, data, _diagnostics) = ops::verify::execute(
        &root,
        &[],
        None,
        Some("VO-DOES-NOT-EXIST".to_owned()),
        None,
        None,
        false,
        &registry(),
    )
    .expect("verify must run to completion even when the named VO does not exist");
    assert_eq!(
        data.scope.requested.entities,
        vec!["VO-DOES-NOT-EXIST".to_owned()],
        "DES-548: an entity-limited scope must name that entity in scope.requested.entities"
    );
    assert!(
        data.scope.unverified_outside_scope,
        "DES-548: an entity axis given must mean scope.unverified_outside_scope is true"
    );
}
