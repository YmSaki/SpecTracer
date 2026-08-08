#!/usr/bin/env python3
import argparse
import filecmp
import json
import subprocess
from pathlib import Path


MILESTONES = {
    "M1": ["crates/vtest-model", "crates/vtest-store", "crates/vtest-scan", "crates/vtest-cli", "tests/fixtures"],
    "M2": [],
    "M3": ["crates/vtest-audit"],
    "M4": ["crates/vtest-exec"],
    "M5": [],
    "M6": ["crates/vtest-verify"],
    "M7": [],
    "M8": [],
    "M9": ["crates/vtest-mcp"],
}


def run(root: Path, command: list[str]) -> dict:
    result = subprocess.run(command, cwd=root, text=True, stdout=subprocess.PIPE, stderr=subprocess.STDOUT, check=False)
    return {
        "command": command,
        "exit_code": result.returncode,
        "status": "PASS" if result.returncode == 0 else "FAIL",
        "output_tail": result.stdout.splitlines()[-30:],
    }


def distribution_sync(root: Path) -> dict:
    canonical = root / ".agents" / "skills"
    bundled = root / "plugins" / "spectracer-development" / "skills"
    mismatches = []
    for skill in ("verify-change", "architecture-check", "release-check"):
        left = canonical / skill
        right = bundled / skill
        left_files = (
            sorted(path.relative_to(left) for path in left.rglob("*") if path.is_file() and "__pycache__" not in path.parts)
            if left.exists()
            else []
        )
        right_files = (
            sorted(path.relative_to(right) for path in right.rglob("*") if path.is_file() and "__pycache__" not in path.parts)
            if right.exists()
            else []
        )
        if left_files != right_files:
            mismatches.append(f"{skill}: file set differs")
            continue
        for relative in left_files:
            if not filecmp.cmp(left / relative, right / relative, shallow=False):
                mismatches.append(f"{skill}/{relative}: content differs")
    return {"status": "PASS" if not mismatches else "FAIL", "mismatches": mismatches}


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--root", type=Path, default=Path.cwd())
    parser.add_argument("--milestone", choices=list(MILESTONES), required=True)
    args = parser.parse_args()
    root = args.root.resolve()
    target_number = int(args.milestone[1:])

    required = []
    for name, paths in MILESTONES.items():
        if int(name[1:]) <= target_number:
            required.extend(paths)
    missing = sorted(path for path in set(required) if not (root / path).exists())

    checks = []
    if (root / "Cargo.toml").exists():
        for command in (
            ["cargo", "fmt", "--all", "--", "--check"],
            ["cargo", "test", "--workspace"],
            ["cargo", "clippy", "--workspace", "--all-targets", "--", "-D", "warnings"],
        ):
            checks.append(run(root, command))
        if (root / "crates" / "vtest-cli" / "Cargo.toml").exists():
            checks.append(run(root, ["cargo", "run", "--quiet", "-p", "vtest-cli", "--", "doctor"]))

    sync = distribution_sync(root)
    ok = not missing and bool(checks) and all(check["status"] == "PASS" for check in checks) and sync["status"] == "PASS"
    status = "READY" if ok else "NOT_READY"
    print(json.dumps({"status": status, "milestone": args.milestone, "missing_paths": missing, "distribution_sync": sync, "checks": checks}, ensure_ascii=False, indent=2))
    return 0 if ok else 1


if __name__ == "__main__":
    raise SystemExit(main())
