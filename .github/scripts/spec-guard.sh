#!/usr/bin/env bash
# Enforces the specification-change path rules in DEVELOPMENT.md
# ("Git Flow" -> "Specification changes"). Used by the `spec-guard` CI job.
#
# Environment:
#   HEAD_REF  head branch name of the pull request
#   BASE_SHA  base commit of the pull request
#   HEAD_SHA  head commit of the pull request
set -euo pipefail

: "${HEAD_REF:?HEAD_REF is required}"
: "${BASE_SHA:?BASE_SHA is required}"
: "${HEAD_SHA:?HEAD_SHA is required}"

# Spec paths. docs/canonical/export/ is derived and carries no authority.
SPEC_PATHS=(docs/canonical .verify/doc ':(exclude)docs/canonical/export')
# Code and tests: a spec/* branch must not change them.
IMPL_PATHS=(crates tests)

changed() { git diff --name-only "${BASE_SHA}...${HEAD_SHA}" -- "$@"; }

status=0

if [[ "${HEAD_REF}" == spec/* ]]; then
  impl_changes="$(changed "${IMPL_PATHS[@]}")"
  if [[ -n "${impl_changes}" ]]; then
    echo "::error title=spec-guard::A spec/* branch must not change crates/ or tests/. Implementation starts after the spec PR is merged."
    echo "${impl_changes}"
    status=1
  fi
else
  spec_changes="$(changed "${SPEC_PATHS[@]}")"
  if [[ -n "${spec_changes}" ]]; then
    echo "::error title=spec-guard::Only spec/* branches may change the specification (docs/canonical/**, .verify/doc/**). Branch '${HEAD_REF}' changes:"
    echo "${spec_changes}"
    status=1
  fi
fi

exit "${status}"
