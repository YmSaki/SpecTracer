//! Composition root: the one place the built-in adapters are instantiated
//! and registered.
//!
//! The core crates (`vtest-scan`, `vtest-exec`, `vtest-verify`) never create
//! an adapter themselves; they receive this registry and select capabilities
//! through it (BD-007), so another adapter can be registered without
//! changing the core verifier (BD-102). `vtest-cli` and `vtest-mcp` both
//! obtain the registry here, which is why they use the same one (BD-100).
//!
//! `AdapterRegistry::register` rejects a duplicate adapter id and a declared
//! capability the implementation does not provide (DES-351, DS-1569,
//! DS-1663). With a fixed list that is an implementation bug, and it is
//! still reported, as E-ADAPTER-001, instead of being ignored.

use vtest_adapter_api::{AdapterRegistrationError, AdapterRegistry};
use vtest_adapter_rust::RustCargoAdapter;

use crate::envelope::Failure;

/// v0.1 の組込 production adapter一覧を登録した registry を返す
/// （基本仕様 §27「組込 production adapter は `rust-cargo` とし
/// ...`rust-cargo` 以外の production language adapter は v0.1 の
/// 提供範囲に含めない」）。
pub fn builtin_registry() -> Result<AdapterRegistry, AdapterRegistrationError> {
    let mut registry = AdapterRegistry::new();
    registry.register(Box::new(RustCargoAdapter::new()))?;
    registry.register(Box::new(vtest_adapter_lexical::c::adapter()))?;
    registry.register(Box::new(vtest_adapter_lexical::cpp::adapter()))?;
    registry.register(Box::new(vtest_adapter_lexical::csharp::adapter()))?;
    registry.register(Box::new(vtest_adapter_lexical::go::adapter()))?;
    registry.register(Box::new(vtest_adapter_lexical::java::adapter()))?;
    registry.register(Box::new(vtest_adapter_lexical::javascript::adapter()))?;
    registry.register(Box::new(vtest_adapter_lexical::kotlin::adapter()))?;
    registry.register(Box::new(vtest_adapter_lexical::php::adapter()))?;
    registry.register(Box::new(vtest_adapter_lexical::python::adapter()))?;
    registry.register(Box::new(vtest_adapter_lexical::ruby::adapter()))?;
    registry.register(Box::new(vtest_adapter_lexical::swift::adapter()))?;
    Ok(registry)
}

/// The built-in registry, or the E-ADAPTER-001 usage failure both
/// entrypoints report when it cannot be composed.
pub fn resolve_registry() -> Result<AdapterRegistry, Failure> {
    builtin_registry().map_err(|error| Failure::usage("E-ADAPTER-001", error.to_string()))
}
