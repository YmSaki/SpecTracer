#!/usr/bin/env python3
"""Run the deterministic SpecTracer change gate and report every result."""

import argparse
import json
import shutil
import subprocess
from pathlib import Path


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--root", type=Path, required=True)
    args = parser.parse_args()
    root = args.root.resolve()
    checks = []
    commands = [
        ["cargo", "fmt", "--all", "--", "--check"],
        ["cargo", "test", "--workspace"],
        ["cargo", "clippy", "--workspace", "--all-targets", "--", "-D", "warnings"],
    ]
    if (root / "crates/vtest-cli").is_dir():
        commands.append(["cargo", "run", "--quiet", "-p", "vtest-cli", "--", "doctor"])
    for command in commands:
        if shutil.which(command[0]) is None:
            checks.append({"command": command, "status": "NOT_EXECUTED", "reason": "cargo unavailable"})
            continue
        completed = subprocess.run(command, cwd=root, text=True, stdout=subprocess.PIPE,
                                   stderr=subprocess.STDOUT, check=False)
        checks.append({"command": command, "status": "PASS" if completed.returncode == 0 else "FAIL",
                       "exit_code": completed.returncode, "output_tail": completed.stdout[-4000:]})
    overall = "PASS" if all(item["status"] == "PASS" for item in checks) else "NOT_READY"
    print(json.dumps({"root": str(root), "overall": overall, "checks": checks}, ensure_ascii=False, indent=2))
    return 0 if overall == "PASS" else 1


if __name__ == "__main__":
    raise SystemExit(main())
