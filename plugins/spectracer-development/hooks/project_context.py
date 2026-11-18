#!/usr/bin/env python3
import json
import sys


def main() -> int:
    payload = json.load(sys.stdin)
    context = (
        "SpecTracer product behavior follows the current v0.1 requirements, basic specification, "
        "detailed design, and Annexes A/C; DEVELOPMENT.md governs process only. "
        "Verification uses exactly four checks (chain_integrity, orphan_detection, target_binding, "
        "oracle_presence) and five states (PASS, FAIL, MISMATCH, NO_EVIDENCE, UNKNOWN). "
        "MISSING, NOT_EXECUTED, NOT_CHECKED, and STALE are diagnostics, not states. "
        "Keep Judgment, Approval, verification state, and gate satisfaction separate, and fail "
        "closed when current conformance evidence is insufficient."
    )
    print(
        json.dumps(
            {
                "hookSpecificOutput": {
                    "hookEventName": "SessionStart",
                    "additionalContext": context,
                }
            },
            ensure_ascii=False,
        )
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
