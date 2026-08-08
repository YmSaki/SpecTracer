#!/usr/bin/env python3
import json
import sys


def main() -> int:
    payload = json.load(sys.stdin)
    context = (
        "SpecTracer source precedence is requirements -> basic specification -> detailed design. "
        "Preserve fail-closed results, canonical/derived separation, and M1-M9 order. "
        "The project vtest MCP entry remains disabled until M9 is implemented."
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
