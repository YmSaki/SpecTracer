#!/usr/bin/env python3
import argparse
import json
import subprocess
from pathlib import Path


def execute(root: Path, command: list[str]) -> dict:
    result = subprocess.run(
        command,
        cwd=root,
        text=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.STDOUT,
        check=False,
    )
    return {
        "command": command,
        "exit_code": result.returncode,
        "status": "PASS" if result.returncode == 0 else "FAIL",
        "output_tail": result.stdout.splitlines()[-30:],
    }


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--root", type=Path, default=Path.cwd())
    args = parser.parse_args()
    root = args.root.resolve()

    if not (root / "Cargo.toml").exists():
        report = {"ok": False, "status": "NOT_EXECUTED", "reason": "Cargo.toml is not implemented yet", "checks": []}
        print(json.dumps(report, ensure_ascii=False, indent=2))
        return 1

    commands = [
        ["cargo", "fmt", "--all", "--", "--check"],
        ["cargo", "test", "--workspace"],
        ["cargo", "clippy", "--workspace", "--all-targets", "--", "-D", "warnings"],
    ]
    if (root / "crates" / "vtest-cli" / "Cargo.toml").exists():
        commands.append(["cargo", "run", "--quiet", "-p", "vtest-cli", "--", "doctor"])

    checks = [execute(root, command) for command in commands]
    ok = all(check["status"] == "PASS" for check in checks)
    print(json.dumps({"ok": ok, "status": "PASS" if ok else "FAIL", "checks": checks}, ensure_ascii=False, indent=2))
    return 0 if ok else 1


if __name__ == "__main__":
    raise SystemExit(main())
