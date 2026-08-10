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
- `synthetic/metadata/tests.changed.json` changes only canonical logical
  metadata for Test-subject and freshness invalidation.
- `synthetic/manifest-no-runner.json`, `manifest-incomplete-analysis.json`, and
  `manifest-discovery-failure.json` drive fail-closed capability/state mapping.
- `synthetic/target-observations.json` fixes PASS/FAIL/UNKNOWN target inputs and
  the required FAIL-dominant aggregate.
- `mixed/collisions.json` defines repository-global Test ID and SRC ID
  collisions across `rust-cargo` and `synthetic`.
- `mixed/order-a.json` and `mixed/order-b.json` contain equivalent inputs in
  opposite adapter/filesystem order.
- `forms/` fixes duplicate and ambiguous owner cases with Rust fallback
  forbidden.
- `relations/` fixes bare/prefixed v1 alias inputs and their duplicate-payload
  rejection case.
- `api-contract/` is a test-only compile contract for the normative neutral
  DTO and model field sets. It is intentionally RED until W1 creates the API.
- `config/v2-rust-cargo.yaml` is the normative version 2 Rust configuration.
- `config/v2-incomplete-scope.yaml` is rejected with `E-CONFIG-001`.

The production adapter remains `rust-cargo`; `synthetic` is acceptance-only.
