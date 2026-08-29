# v0.1 Documentation Drift Audit

## Authority

Current normative baseline:

- `baseline/requirements-v0.1`
- `baseline/design-v0.1`

The normative verification model has exactly four checks:

- `chain_integrity`
- `orphan_detection`
- `target_binding`
- `oracle_presence`

Verification states are exactly:

- `PASS`
- `FAIL`
- `MISMATCH`
- `NO_EVIDENCE`
- `UNKNOWN`

`MISSING`, `NOT_EXECUTED`, `NOT_CHECKED`, and `STALE` are diagnostics, not
verification states.

## Findings

### Critical operational drift

- `AGENTS.md` still defines the obsolete 12-item model and obsolete state set.
- `.agents/skills/verify-change/SKILL.md` still validates against 12 full-scope
  items and accepts obsolete `--items impl_consistency`.
- `.agents/skills/release-check/SKILL.md` still uses the obsolete M6 11-item
  release gate and can therefore produce READY against a superseded contract.
- `.agents/skills/architecture-check/SKILL.md` contains obsolete state/diagnostic
  semantics.

### User-facing drift

- `README.md` still presents the predecessor SPEC/REQ/VO data model, obsolete
  CLI examples, obsolete verification-state set, obsolete 11-item verification
  model, and obsolete canonical `.verify` layout as current behavior.
- `TESTING.md` still presents diagnostics as verification states and describes
  limited-scope behavior using the predecessor aggregation model.

### Implementation evidence

- `tests/ACCEPTANCE.md` records real acceptance evidence for the predecessor
  implementation and MUST NOT be mechanically rewritten as evidence for the
  current four-check design.
- Until implementation is migrated, this ledger must be identified as
  implementation-baseline evidence rather than proof of conformance to the
  current normative design.

### No confirmed drift from keyword audit alone

- `CONTRIBUTING.md`
- generic sections of `ARCHITECTURE.md`

These require contextual review before modification.

## Remediation boundary

This branch updates documentation and operational guidance only.

It does not modify Rust implementation behavior or manufacture acceptance
evidence for the new design.

Implementation conformance to the current normative baseline is a subsequent
workstream.
