#!/usr/bin/env python3
import json
import subprocess
import sys
from pathlib import Path


def run(root: Path, command: list[str]) -> tuple[bool, str]:
    completed = subprocess.run(
        command,
        cwd=root,
        text=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.STDOUT,
        check=False,
    )
    tail = "\n".join(completed.stdout.splitlines()[-20:])
    label = " ".join(command)
    return completed.returncode == 0, f"$ {label}\n{tail}".strip()


def main() -> int:
    payload = json.load(sys.stdin)
    if payload.get("stop_hook_active"):
        print(json.dumps({"continue": True}))
        return 0

    root = Path(payload.get("cwd", ".")).resolve()
    if not (root / "Cargo.toml").exists():
        print(json.dumps({"continue": True}))
        return 0

    checks = [
        ["cargo", "fmt", "--all", "--", "--check"],
        ["cargo", "test", "--workspace"],
    ]
    if (root / "crates" / "vtest-cli" / "Cargo.toml").exists():
        checks.append(["cargo", "run", "--quiet", "-p", "vtest-cli", "--", "doctor"])

    failures = []
    for command in checks:
        ok, output = run(root, command)
        if not ok:
            failures.append(output)

    if failures:
        reason = "SpecTracer validation failed. Fix or explicitly report these non-PASS gates:\n\n" + "\n\n".join(failures)
        print(json.dumps({"decision": "block", "reason": reason}, ensure_ascii=False))
    else:
        print(json.dumps({"continue": True}))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
