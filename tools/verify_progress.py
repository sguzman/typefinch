#!/usr/bin/env python3
"""Check the machine-readable acceptance ledger and rendered progress claims."""
import json
from pathlib import Path
root = Path(__file__).resolve().parents[1]
data = json.loads((root / "docs/progress.json").read_text())
seen = set()
for milestone in data["milestones"]:
    for phase in milestone["phases"]:
        for task in phase["tasks"]:
            assert task["id"] not in seen, task["id"]
            seen.add(task["id"])
            checks = task["checks"]
            assert checks, task["id"]
            for check in checks:
                assert not check["passed"] or check.get("evidence"), task["id"]
            actual = int(100 * sum(c["passed"] for c in checks) / len(checks) + .5)
            assert actual == task["progress"], (task["id"], actual, task["progress"])
print(f"Verified {len(seen)} tasks; all acceptance evidence and percentages are internally consistent.")
