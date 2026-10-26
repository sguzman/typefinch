#!/usr/bin/env bash
# Compare exact acceptance percentage-point deltas; no fabricated progress.
set -Eeuo pipefail
tmp="$(mktemp -d)"
trap 'rm -rf -- "$tmp"' EXIT
lua5.4 tools/roadmap.lua --delta docs/progress.json >"$tmp/unchanged"
grep -F '(Δ +0.00 pp)' "$tmp/unchanged" >/dev/null
grep -F 'T704 ' "$tmp/unchanged" | grep -F '(Δ +0 pp)' >/dev/null
# Build a valid older ledger with one T002 acceptance check still unproven.
jq '(.milestones[].phases[].tasks[] | select(.id=="T002")) |= (
    .checks[0].passed=false
    | del(.checks[0].evidence)
    | .progress=((100*([.checks[] | select(.passed)] | length)
      / (.checks | length) + 0.5) | floor)
  )' docs/progress.json >"$tmp/previous.json"
lua5.4 tools/roadmap.lua --delta "$tmp/previous.json" >"$tmp/progress"
grep -F 'T002 ' "$tmp/progress" | grep -F '(Δ +33 pp)' >/dev/null
grep -F 'Overall:' "$tmp/progress" | grep -F '(Δ +0.80 pp)' >/dev/null
# A changed acceptance definition is not valid before/after evidence.
jq '(.milestones[].phases[].tasks[] | select(.id=="T002")
  | .checks[0].text)="Rewritten criterion"' docs/progress.json >"$tmp/redefined.json"
if lua5.4 tools/roadmap.lua --delta "$tmp/redefined.json" >"$tmp/invalid" 2>&1;then
  echo 'Redefined acceptance criterion was incorrectly comparable' >&2;exit 1
fi
grep -F 'acceptance criterion changed: T002' "$tmp/invalid" >/dev/null
echo 'Acceptance delta audit PASS: zero change, +33 task pp / +0.80 overall pp, and criteria-change refusal'
