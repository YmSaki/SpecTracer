#!/usr/bin/env python3
"""Run the release baseline; milestone acceptance needs separate evidence."""

import argparse
import json
import subprocess
import sys
from pathlib import Path


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--root", type=Path, required=True)
    parser.add_argument("--milestone", choices=[f"M{i}" for i in range(1, 10)], required=True)
    args = parser.parse_args()
    root = args.root.resolve()
    expected = ["Cargo.toml", "tests/ACCEPTANCE.md", "crates/vtest-model", "crates/vtest-store",
                "crates/vtest-scan", "crates/vtest-cli"]
    if int(args.milestone[1:]) >= 9:
        expected.append("crates/vtest-mcp")
    presence = [{"path": path, "status": "PASS" if (root / path).exists() else "MISSING"}
                for path in expected]
    gate = Path(__file__).resolve().parents[2] / "verify-change/scripts/verify_change.py"
    run = subprocess.run([sys.executable, str(gate), "--root", str(root)], text=True,
                         stdout=subprocess.PIPE, stderr=subprocess.PIPE, check=False)
    try:
        baseline = json.loads(run.stdout)
    except json.JSONDecodeError:
        baseline = {"overall": "NOT_EXECUTED", "error": (run.stderr or run.stdout)[-4000:]}
    baseline_ok = all(item["status"] == "PASS" for item in presence) and baseline.get("overall") == "PASS"
    print(json.dumps({"milestone": args.milestone, "root": str(root), "presence": presence,
                      "baseline": baseline, "baseline_status": "PASS" if baseline_ok else "NOT_READY",
                      "acceptance_status": "NOT_CHECKED",
                      "note": "Check every applicable Annex B acceptance criterion separately before release."},
                     ensure_ascii=False, indent=2))
    return 0 if baseline_ok else 1


if __name__ == "__main__":
    raise SystemExit(main())
