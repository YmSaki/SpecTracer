# Repository instructions

## Authority

Product behavior is governed, in descending order, by:

1. `docs/AI並列開発向けテスト検証システム 要求・要件定義 v0.1.md`
2. `docs/AI並列開発向けテスト検証システム 基本仕様 v0.1.md`
3. `docs/AI並列開発向けテスト検証システム 詳細設計 v0.1.md`
4. `docs/AI並列開発向けテスト検証システム 詳細設計 別紙A インターフェース仕様 v0.1.md`
5. `docs/AI並列開発向けテスト検証システム 詳細設計 別紙C 受入仕様 v0.1.md`

An implementation schedule is process material, not a normative product source. Use it only when planning requires it, and never let it override the documents above. If active normative documents disagree or leave a required decision open, cite the exact sections and report the unresolved contract. Do not fill the gap from historical files, tests, implementation behavior, or Git history.

`DEVELOPMENT.md` is not a product-behavior authority. It is the separate, sole repository authority for Git flow, branch naming, tags, and shared-history policy. Follow it without restating or redefining its rules here.

## Product invariants

- Verification has exactly four checks: `chain_integrity`, `orphan_detection`, `target_binding`, and `oracle_presence`. Adding document layers, evidence sources, diagnostics, judgments, or approval rules does not add checks.
- Verification has exactly five states: `PASS`, `FAIL`, `MISMATCH`, `NO_EVIDENCE`, and `UNKNOWN`.
- `MISSING`, `NOT_EXECUTED`, `NOT_CHECKED`, and `STALE` are diagnostic labels, not verification states. Keep state, diagnostic label, diagnostic severity, operation error, aggregate OK/NG, and gate satisfaction as distinct fields and concepts.
- A limited scope never treats work outside the requested scope as `PASS`. Omitted checks represented in the aggregation tree remain `NO_EVIDENCE` with `NOT_CHECKED`. Entity scope outside the request remains explicitly unverified through the canonical scope representation. Do not present a limited result as complete verification.
- Upstream normative artifacts use the generic `document` model. Document layers are connected with `derives_from`; do not introduce document-type-specific canonical entities or schemas.
- Judgment and Approval are independent domains and may be independent entities. Recording or accepting either does not promote or demote a verification state. Judgment completion is not Approval.
- The canonical Approval CLI is `vtest approval create`, `vtest approval withdraw`, and `vtest approval show`. `vtest vo approve` is only an alias of Approval creation and must not acquire separate semantics.
- Verification state and gate satisfaction are separate axes. With `--gate`, exit code `0` and top-level `ok: true` may mean that the named gate is satisfied; they do not by themselves mean verification state `PASS`. Always expose and evaluate the actual aggregate verification state separately.
- Missing, stale, ambiguous, incomplete, unsupported, or unanalyzable inputs never become `PASS` by fallback. `UNKNOWN` is a deterministic limit, not an internal-error fallback. Operation rejection and internal failure stay outside the five-state model.
- Static or other deterministic analysis may emit `PASS` or `FAIL` only when the applicable canonical rule establishes that result. Do not infer either result from inability to prove the opposite.
- Compatibility readers and wire adapters must normalize into the current canonical model and pass every current invariant before their data can affect a result. Compatibility data must not reconstruct canonical truth, select an ambiguous candidate, weaken scope, or bypass freshness and aggregation.
- Canonical and derived data remain separate. Derivable graphs, indexes, static-analysis results, bundles, and reports must not become independent sources of truth.
- Evidence, Judgment, and Approval validity is bound to the complete current subject and dependency closure required by the canonical design. Historical records may remain readable without becoming effective current evidence.
- Language-specific discovery, analysis, editing, execution, and coverage belong behind adapter capabilities. Core domain and wire behavior must not invent language-specific defaults when an adapter or capability is absent.
- Acceptance evidence establishes current canonical conformance only when its assertions are explicitly mapped to the current canonical clauses and observables. Historical or superseded acceptance evidence may be used as regression evidence, but cannot by itself establish current conformance.
- Do not describe behavior as implemented, verified, accepted, or release-ready unless current implementation evidence demonstrates the applicable canonical contract.

## Agent operating rules

- Before changing behavior, read the applicable authority sections from requirements through acceptance criteria. Preserve upstream-to-downstream traceability; feed contradictions upstream instead of silently choosing a new rule.
- Before any review, state the review target, purpose, fixed decision axes, and exclusions. If no existing axis applies, define the appropriate axis before reviewing. Do not add, remove, or change axes during the review. Report a deficient axis separately rather than silently replacing it.
- Do not reject work for concerns outside the declared axes. Within an axis, report every material issue; the number of findings is not itself over-review.
- A material canonical or safety violation discovered outside the declared axes must be reported as an out-of-scope blocker or trigger an explicit re-scope. Do not silently ignore it or return a ready decision while it remains unresolved. Do not score it as an in-axis finding unless the review contract is explicitly re-scoped.
- Keep “changing the review axis” distinct from “finding a failure on the declared axis.”
- A gate asks whether the current phase can hand its responsibilities to the next phase correctly, not whether the artifact is flawless. State the required handoff evidence and fail closed when it is absent.
- Separate direct observations from inferences. Give file/section or command evidence for findings, and label unresolved canonical questions rather than guessing.
- Inspect the current implementation and tests, but do not treat their behavior as authority over the canonical documents. Preserve legitimate unrelated user changes.
- For verification and release decisions, use the project Skills in `.agents/skills/`. Do not recreate removed helper scripts unless a new deterministic procedure is fully derivable from the current canonical contract and current implementation, and validate any helper that is added.
- Run checks in proportion to the changed surface. The repository development gate is defined in `DEVELOPMENT.md` and `TESTING.md`; passing it does not replace canonical conformance evidence.
