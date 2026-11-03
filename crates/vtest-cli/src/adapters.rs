//! Composition root: the one place `rust-cargo` (and any other built-in
//! adapter) is registered.
//!
//! BD-007「CLI・MCP・検証coreはadapter registryを介して能力を選択する」・
//! BD-102「core verifierを変更せずに別adapterを登録できる境界を要求する」
//! に従い、`vtest-scan` / `vtest-verify` / `vtest-exec`（core）は自分で
//! adapter を生成しない。`vtest-cli`・`vtest-mcp` が唯一の呼び出し元で
//! あり、`vtest-mcp` は `vtest-cli` に既に依存している（このcrateのdoc
//! comment・Cargo.toml参照）ため、ここを cli と mcp が共有する composition
//! root の唯一の場所とする — cli/mcp が `vtest-adapter-rust` へ依存する
//! こと自体は BD-118「cli / mcp が adapter-rust に依存するのは可」が
//! 許す。
//!
//! `AdapterRegistry::register` が返す `Err`（DES-351/DS-1569/DS-1663:
//! adapter IDの重複、または宣言capabilityと実装の不一致）は、この
//! composition root が唯一の登録者であるため理論上は到達しない
//! （固定1件の登録）。到達すればそれは実装のバグであり、`Result` を
//! `unimplemented!`/黙殺せず呼び出し元へ伝播させ、E-ADAPTER-001として
//! usage failure にする。

use vtest_adapter_api::{AdapterRegistrationError, AdapterRegistry};
use vtest_adapter_rust::RustCargoAdapter;

/// v0.1 の組込 production adapter一覧を登録した registry を返す
/// （基本仕様 §27「組込 production adapter は `rust-cargo` とし
/// ...`rust-cargo` 以外の production language adapter は v0.1 の
/// 提供範囲に含めない」）。
pub fn builtin_registry() -> Result<AdapterRegistry, AdapterRegistrationError> {
    let mut registry = AdapterRegistry::new();
    registry.register(Box::new(RustCargoAdapter::new()))?;
    Ok(registry)
}
