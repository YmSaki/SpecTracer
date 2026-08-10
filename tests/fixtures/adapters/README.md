# Adapter Acceptance Fixtures

These fixtures are test-adapter inputs, not canonical `.verify/` records. They
exist to prove that the core contract does not depend on Rust syntax, Cargo
coordinates, adjacent doc comments, or Rust item paths.

- `synthetic/manifest.json` describes hash-free discovery drafts and fixed
  runner observations for an in-process test adapter.
- `synthetic/source/cases.synth` contains a non-function Test construct and an
  opaque target locator.
- `synthetic/metadata/tests.json` keeps logical Test metadata non-adjacent to
  the construct.
- `mixed/collisions.json` defines repository-global Test ID and SRC ID
  collisions across `rust-cargo` and `synthetic`.
- `config/v2-rust-cargo.yaml` is the normative version 2 Rust configuration.
- `config/v2-incomplete-scope.yaml` is rejected with `E-CONFIG-001`.

The production adapter remains `rust-cargo`; `synthetic` is acceptance-only.
