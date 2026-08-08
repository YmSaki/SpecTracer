#!/usr/bin/env python3
import json
import re
import sys


PATH_LINE = re.compile(r"^\*\*\* (Add|Update|Delete) File: (.+)$", re.MULTILINE)
APPEND_ONLY = re.compile(r"(^|/)\.verify/(approvals|audits|evidence)/", re.IGNORECASE)
DERIVED = re.compile(r"(^|/)\.verify/cache(/|$)", re.IGNORECASE)


def deny(reason: str) -> None:
    print(
        json.dumps(
            {
                "hookSpecificOutput": {
                    "hookEventName": "PreToolUse",
                    "permissionDecision": "deny",
                    "permissionDecisionReason": reason,
                }
            },
            ensure_ascii=False,
        )
    )


def main() -> int:
    payload = json.load(sys.stdin)
    tool_input = payload.get("tool_input") or {}
    patch = tool_input.get("command", "") if isinstance(tool_input, dict) else ""

    for action, raw_path in PATH_LINE.findall(patch):
        path = raw_path.strip().replace("\\", "/")
        if DERIVED.search(path):
            deny(f"{path} is derived data. Rebuild .verify/cache instead of editing it.")
            return 0
        if action in {"Update", "Delete"} and APPEND_ONLY.search(path):
            deny(f"{path} is append-only evidence. Add a new vtest record instead of mutating history.")
            return 0

    print("{}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
