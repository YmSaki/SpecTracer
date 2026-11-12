//! Form registry (別紙A §14, DES-489): merges the built-in Forms that
//! Structured Test adapters declare (DES-373) with the user-defined schemas
//! in `.verify/forms/`, and resolves one `kind` to its schema and its single
//! owner adapter.
//!
//! - A `kind` is repository-global (DS-533, DS-1545). Two adapters declaring
//!   the same built-in kind is E-ADAPTER-001 (DS-534).
//! - A schema's `adapter` must name a registered adapter with the Structured
//!   Test capability, and must agree with the built-in owner of the same kind
//!   (DS-1546, DS-1547).
//! - A read-compatible schema without `adapter` resolves only when exactly
//!   one Structured Test adapter accepts it, through its built-in kind
//!   declaration or its compatibility matcher (DS-1548, DES-490).
//! - An unknown kind is E-OP-001 (DS-534). Core never falls back to
//!   `rust-cargo` by name (DS-1549).

use std::{collections::BTreeSet, path::Path};

use vtest_adapter_api::AdapterRegistry;
use vtest_model::Diagnostic;
use vtest_store::{
    is_valid_form_kind, parse_form_schema, read_form_schema_text, FormSchema, VerifyLayout,
};

/// A Form `kind` resolved to its schema and owner adapter.
#[derive(Clone, Debug)]
pub struct ResolvedForm {
    pub schema: FormSchema,
    /// The owner adapter ID (DS-1205: `form_get` states it explicitly).
    pub adapter: String,
}

/// Every Form kind the registry's Structured Test adapters declare as
/// built-in, with the declaring adapter, in adapter ID order.
fn built_in_owners<'a>(registry: &'a AdapterRegistry, kind: &str) -> Vec<(&'a str, &'static str)> {
    structured_ids(registry)
        .into_iter()
        .filter_map(|id| {
            let adapter = registry.structured_test(id)?;
            adapter
                .built_in_forms()
                .iter()
                .find(|form| form.kind == kind)
                .map(|form| (id, form.schema))
        })
        .collect()
}

/// Structured Test adapter IDs in ID order, so resolution never depends on
/// registration order (DES-231).
fn structured_ids(registry: &AdapterRegistry) -> Vec<&str> {
    let mut ids = registry.structured_test_ids();
    ids.sort_unstable();
    ids
}

/// Every Form kind known to the registry or present in `.verify/forms/`,
/// for the candidates of an unknown-kind error.
fn known_kinds(root: &Path, registry: &AdapterRegistry) -> Vec<String> {
    let mut kinds = BTreeSet::new();
    for id in registry.structured_test_ids() {
        if let Some(adapter) = registry.structured_test(id) {
            kinds.extend(
                adapter
                    .built_in_forms()
                    .iter()
                    .map(|form| form.kind.to_owned()),
            );
        }
    }
    if let Ok(entries) = std::fs::read_dir(VerifyLayout::new(root).forms_dir()) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|value| value.to_str()) == Some("yaml") {
                if let Some(stem) = path.file_stem().and_then(|value| value.to_str()) {
                    kinds.insert(stem.to_owned());
                }
            }
        }
    }
    kinds.into_iter().collect()
}

fn invalid_schema(kind: &str, error: impl std::fmt::Display) -> Diagnostic {
    Diagnostic::error(
        "E-OP-001",
        format!("Form `{kind}` schema is invalid: {error}"),
    )
}

/// Resolves `kind` to its schema and single owner adapter, or the rejection
/// DS-1547 requires. Nothing is written either way.
pub fn resolve_form(
    root: &Path,
    registry: &AdapterRegistry,
    kind: &str,
) -> Result<ResolvedForm, Diagnostic> {
    if !is_valid_form_kind(kind) {
        return Err(Diagnostic::error(
            "E-OP-001",
            format!("Form kind `{kind}` must match [a-z0-9][a-z0-9-]* (DS-1545)"),
        ));
    }
    let owners = built_in_owners(registry, kind);
    if owners.len() > 1 {
        return Err(Diagnostic::error(
            "E-ADAPTER-001",
            format!(
                "Form kind `{kind}` is declared by more than one adapter; refusing to pick one \
                 (DS-534, DS-1547)"
            ),
        )
        .with_candidates(
            owners
                .iter()
                .map(|(id, _)| (*id).to_owned())
                .collect::<Vec<_>>(),
        ));
    }
    let built_in = owners.first().copied();

    let layout = VerifyLayout::new(root);
    let user_text = read_form_schema_text(&layout, kind)
        .map_err(|error| Diagnostic::error("E-OP-001", error.to_string()))?;
    let Some(text) = user_text else {
        // No user-defined schema: only a built-in declaration can supply it.
        let Some((adapter, schema_text)) = built_in else {
            return Err(
                Diagnostic::error("E-OP-001", format!("unknown Form kind `{kind}`"))
                    .with_candidates(known_kinds(root, registry)),
            );
        };
        let schema = parse_form_schema(schema_text).map_err(|error| invalid_schema(kind, error))?;
        return owned(kind, schema, adapter, built_in);
    };

    let schema = parse_form_schema(&text).map_err(|error| invalid_schema(kind, error))?;
    if schema.kind != kind {
        return Err(Diagnostic::error(
            "E-OP-001",
            format!(
                "`.verify/forms/{kind}.yaml` declares kind `{}`; the file name must match the \
                 kind (DS-1239)",
                schema.kind
            ),
        ));
    }
    match schema.adapter.clone() {
        Some(adapter) => {
            if registry.get(&adapter).is_none() {
                return Err(Diagnostic::error(
                    "E-ADAPTER-001",
                    format!("Form `{kind}` names unknown adapter `{adapter}` (DS-1547)"),
                )
                .with_candidates(registry.ids().map(str::to_owned)));
            }
            if registry.structured_test(&adapter).is_none() {
                return Err(Diagnostic::error(
                    "E-ADAPTER-004",
                    format!(
                        "Form `{kind}` names adapter `{adapter}`, which has no Structured Test \
                         capability (DS-1555)"
                    ),
                ));
            }
            owned(kind, schema, &adapter, built_in)
        }
        None => {
            // DS-1548 / DES-490: exactly one Structured Test adapter must
            // accept the schema, by its built-in kind or its matcher.
            let accepting: Vec<&str> = structured_ids(registry)
                .into_iter()
                .filter(|id| {
                    registry.structured_test(id).is_some_and(|adapter| {
                        adapter
                            .built_in_forms()
                            .iter()
                            .any(|form| form.kind == kind)
                            || adapter.accepts_form_schema(&text)
                    })
                })
                .collect();
            match accepting.as_slice() {
                [adapter] => owned(kind, schema, adapter, built_in),
                [] => Err(Diagnostic::error(
                    "E-OP-001",
                    format!(
                        "Form `{kind}` declares no `adapter` and no registered adapter accepts \
                         it (DS-1548)"
                    ),
                )),
                many => Err(Diagnostic::error(
                    "E-ADAPTER-001",
                    format!(
                        "Form `{kind}` declares no `adapter` and several adapters accept it; \
                         refusing to pick one (DS-1548)"
                    ),
                )
                .with_candidates(many.iter().map(|id| (*id).to_owned()).collect::<Vec<_>>())),
            }
        }
    }
}

/// The final DS-1546 check: the owner has the Structured Test capability and
/// agrees with the built-in owner of the same kind, if any.
fn owned(
    kind: &str,
    schema: FormSchema,
    adapter: &str,
    built_in: Option<(&str, &'static str)>,
) -> Result<ResolvedForm, Diagnostic> {
    if let Some((built_in_owner, _)) = built_in {
        if built_in_owner != adapter {
            return Err(Diagnostic::error(
                "E-ADAPTER-001",
                format!(
                    "Form `{kind}` names adapter `{adapter}`, but `{built_in_owner}` owns that \
                     built-in kind (DS-1547)"
                ),
            ));
        }
    }
    if let Some(declared) = &schema.adapter {
        if declared != adapter {
            return Err(Diagnostic::error(
                "E-ADAPTER-001",
                format!("Form `{kind}` declares adapter `{declared}`, not `{adapter}` (DS-1547)"),
            ));
        }
    }
    Ok(ResolvedForm {
        schema,
        adapter: adapter.to_owned(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use vtest_adapter_api::{
        Adapter, AdapterDescriptor, BuiltInForm, Capability, StructuredTestAdapter,
    };
    use vtest_adapter_rust::RustCargoAdapter;

    const FAKE_FORM: &str = "kind: rust-unit-function\nadapter: fake\ntitle: Fake\nfields:\n  - name: target\n    question: Target?\n    type: symbol\n    required: true\ntemplate: |\n  body {target}\n";

    /// A second Structured Test adapter that declares `rust-unit-function`
    /// too, or (`declares: false`) has the capability with no built-in Form.
    struct Fake {
        id: &'static str,
        declares: bool,
        structured: bool,
    }

    impl Adapter for Fake {
        fn descriptor(&self) -> AdapterDescriptor {
            AdapterDescriptor {
                id: self.id.to_owned(),
                languages: vec!["fake".to_owned()],
                capabilities: if self.structured {
                    vec![Capability::StructuredTest]
                } else {
                    Vec::new()
                },
                config_namespace: self.id.to_owned(),
            }
        }
        fn as_structured_test(&self) -> Option<&dyn StructuredTestAdapter> {
            self.structured
                .then_some(self as &dyn StructuredTestAdapter)
        }
    }

    impl StructuredTestAdapter for Fake {
        fn check_test_construct(&self, _: &str) -> Result<(), String> {
            Ok(())
        }
        fn check_source_file(&self, _: &str) -> Result<(), String> {
            Ok(())
        }
        fn check_body(&self, _: &str) -> Result<(), String> {
            Ok(())
        }
        fn is_identifier(&self, _: &str) -> bool {
            true
        }
        fn body_range(&self, construct: &str) -> Result<std::ops::Range<usize>, String> {
            Ok(0..construct.len())
        }
        fn enum_variants(&self, _: &str, _: &str) -> Option<Vec<String>> {
            None
        }
        fn language(&self) -> &'static str {
            "Fake"
        }
        fn source_extension(&self) -> &'static str {
            "fake"
        }
        fn source_files(&self, _: &Path) -> Vec<std::path::PathBuf> {
            Vec::new()
        }
        fn parse_locator(&self, _: &str) -> Option<vtest_adapter_api::ParsedLocator> {
            None
        }
        fn built_in_forms(&self) -> &'static [BuiltInForm] {
            if self.declares {
                &[BuiltInForm {
                    kind: "rust-unit-function",
                    schema: FAKE_FORM,
                }]
            } else {
                &[]
            }
        }
    }

    fn registry(extra: Option<Fake>) -> AdapterRegistry {
        let mut registry = AdapterRegistry::new();
        registry
            .register(Box::new(RustCargoAdapter::new()))
            .unwrap();
        if let Some(extra) = extra {
            registry.register(Box::new(extra)).unwrap();
        }
        registry
    }

    fn root(name: &str, forms: &[(&str, &str)]) -> std::path::PathBuf {
        static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let root = std::env::temp_dir().join(format!(
            "vtest-scan-forms-{name}-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
        ));
        let forms_dir = root.join(".verify/forms");
        std::fs::create_dir_all(&forms_dir).unwrap();
        for (kind, text) in forms {
            std::fs::write(forms_dir.join(format!("{kind}.yaml")), text).unwrap();
        }
        root
    }

    fn rust_unit_form() -> &'static str {
        vtest_adapter_rust::structured_test::RUST_UNIT_FUNCTION_FORM
    }

    /// @vtest.id TEST-SCAN-FORM-RESOLVES-UNIQUE-OWNER
    /// @vtest.covers VO-FORM-KIND-UNIQUE-OWNER-DECLARED, VO-FORM-ADAPTERLESS-SCHEMA-ONE-OWNER
    /// @vtest.target crates/vtest-scan/src/forms.rs::resolve_form
    /// @vtest.intent a kind resolves to its schema and single owner adapter, from the adapter's built-in declaration, from a user schema naming its adapter, or from an adapter-less schema exactly one adapter accepts
    /// @vtest.case built-in declaration only
    /// @vtest.case user schema with `adapter`
    /// @vtest.case adapter-less user schema
    #[test]
    fn a_kind_resolves_to_its_schema_and_single_owner() {
        let built_in = resolve_form(
            &root("built-in", &[]),
            &registry(None),
            "rust-unit-function",
        )
        .expect("built-in kind resolves");
        assert_eq!(built_in.adapter, "rust-cargo");
        assert_eq!(built_in.schema.kind, "rust-unit-function");

        let declared = resolve_form(
            &root("declared", &[("rust-unit-function", rust_unit_form())]),
            &registry(None),
            "rust-unit-function",
        )
        .expect("user schema naming its owner resolves");
        assert_eq!(declared.adapter, "rust-cargo");

        let legacy = rust_unit_form().replace("adapter: rust-cargo\n", "");
        let adapterless = resolve_form(
            &root("adapterless", &[("rust-unit-function", &legacy)]),
            &registry(Some(Fake {
                id: "fake",
                declares: false,
                structured: true,
            })),
            "rust-unit-function",
        )
        .expect("exactly one adapter accepts the adapter-less schema");
        assert_eq!(adapterless.adapter, "rust-cargo");
        assert_eq!(adapterless.schema.adapter, None);
    }

    /// @vtest.id TEST-SCAN-FORM-REJECTS-AMBIGUOUS-OR-MISMATCHED-OWNER
    /// @vtest.covers VO-FORM-OWNER-CONFLICT-REJECTED, VO-FORM-UNKNOWN-KIND-E-OP-001
    /// @vtest.target crates/vtest-scan/src/forms.rs::resolve_form
    /// @vtest.intent a kind declared by two adapters, a schema naming an unknown adapter, an adapter without Structured Test, or another adapter than the built-in owner is rejected, and an unknown kind is E-OP-001 with candidates
    /// @vtest.case same built-in kind from two adapters
    /// @vtest.case schema names an unknown adapter
    /// @vtest.case schema names an adapter without Structured Test
    /// @vtest.case schema adapter differs from the built-in owner
    /// @vtest.case unknown kind
    #[test]
    fn conflicting_or_unknown_forms_are_rejected() {
        let duplicate = resolve_form(
            &root("duplicate", &[]),
            &registry(Some(Fake {
                id: "fake",
                declares: true,
                structured: true,
            })),
            "rust-unit-function",
        )
        .unwrap_err();
        assert_eq!(duplicate.code, "E-ADAPTER-001");
        assert_eq!(duplicate.candidates, ["fake", "rust-cargo"]);

        let unknown_adapter = rust_unit_form().replace("adapter: rust-cargo", "adapter: python");
        let error = resolve_form(
            &root(
                "unknown-adapter",
                &[("rust-unit-function", &unknown_adapter)],
            ),
            &registry(None),
            "rust-unit-function",
        )
        .unwrap_err();
        assert_eq!(error.code, "E-ADAPTER-001");

        let no_capability = rust_unit_form().replace("adapter: rust-cargo", "adapter: plain");
        let error = resolve_form(
            &root("no-capability", &[("rust-unit-function", &no_capability)]),
            &registry(Some(Fake {
                id: "plain",
                declares: false,
                structured: false,
            })),
            "rust-unit-function",
        )
        .unwrap_err();
        assert_eq!(error.code, "E-ADAPTER-004");

        let mismatch = rust_unit_form().replace("adapter: rust-cargo", "adapter: fake");
        let error = resolve_form(
            &root("mismatch", &[("rust-unit-function", &mismatch)]),
            &registry(Some(Fake {
                id: "fake",
                declares: false,
                structured: true,
            })),
            "rust-unit-function",
        )
        .unwrap_err();
        assert_eq!(error.code, "E-ADAPTER-001");

        let error = resolve_form(&root("unknown", &[]), &registry(None), "rust-unit").unwrap_err();
        assert_eq!(error.code, "E-OP-001");
        assert!(error.candidates.contains(&"rust-unit-function".to_owned()));
    }
}
