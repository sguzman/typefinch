#!/usr/bin/env python3
"""Regenerate roadmap from the authoritative Typefinch progress ledger."""
import json
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
LEDGER = ROOT / "docs" / "progress.json"
OUTPUT = ROOT / "docs" / "ROADMAP.md"

def rnd(value):
    return int(value + .5)

def mean(tasks):
    return rnd(sum(t["progress"] for t in tasks) / (len(tasks) or 1))

def bar(value):
    n = min(10, rnd(value / 10))
    return "█" * n + "░" * (10 - n)

def render(data):
    all_tasks = [t for m in data["milestones"] for p in m["phases"] for t in p["tasks"]]
    lookup = {t["id"]: t for t in all_tasks}
    lines = [
        "# 🗺️ Typefinch roadmap", "",
        "**Source of truth:** [progress.json](progress.json). Percentages are evidence-backed fractions of atomic acceptance checks, averaged into phase/milestone totals. No unverified task reaches 100%.", "",
        f"**Overall:** `{bar(mean(all_tasks))} {mean(all_tasks)}%` across {len(all_tasks)} tasks. This is tracked progress, not a claim of runtime readiness.", "",
    ]
    for milestone in data["milestones"]:
        tasks = [t for phase in milestone["phases"] for t in phase["tasks"]]
        n = mean(tasks)
        lines += [f"## {milestone['emoji']} {milestone['id']} — {milestone['name']} — `{bar(n)} {n}%`", "", f"**Acceptance gate:** {milestone['gate']}", ""]
        for phase in milestone["phases"]:
            n = mean(phase["tasks"])
            lines += [f"### {phase['emoji']} {phase['id']} — {phase['name']} — `{bar(n)} {n}%`", ""]
            for task in phase["tasks"]:
                n = task["progress"]
                lines.append(f"- {task['emoji']} **{task['id']}** {task['name']} — `{bar(n)} {n}%`")
                for check in task["checks"]:
                    mark = "x" if check["passed"] else " "
                    suffix = " — " + check["evidence"] if check["passed"] else ""
                    lines.append(f"  - [{mark}] {check['text']}{suffix}")
            lines.append("")
        lines += ["**Feature inventory** (computed from associated task progress):", ""]
        for feature in milestone["features"]:
            n = mean([lookup[key] for key in feature["task_ids"]])
            lines.append(f"- {feature['emoji']} {feature['name']} — `{bar(n)} {n}%`")
        lines.append("")
    return "\n".join(lines) + "\n"

if __name__ == "__main__":
    content = render(json.loads(LEDGER.read_text(encoding="utf-8")))
    if "--check" in sys.argv:
        if not OUTPUT.exists() or OUTPUT.read_text(encoding="utf-8") != content:
            raise SystemExit("ROADMAP.md is stale; run python3 tools/render_roadmap.py")
        print("Roadmap equals its machine-readable ledger")
    else:
        OUTPUT.write_text(content, encoding="utf-8")
