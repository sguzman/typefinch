#!/usr/bin/env bash
# X11 / software Mesa smoke test. This is NOT a real Hyprland acceptance pass.
set -Eeuo pipefail
export LIBGL_ALWAYS_SOFTWARE=1
export WINIT_UNIX_BACKEND=x11
export RUST_BACKTRACE=1
tmp="$(mktemp -d)"
app_pid=""
cleanup() {
  if [[ -n "$app_pid" ]]; then kill "$app_pid" 2>/dev/null || true; wait "$app_pid" 2>/dev/null || true; fi
  if [[ -n "${GITHUB_WORKSPACE:-}" ]] && [[ -f "$tmp/typefinch-window.png" ]]; then
    cp "$tmp/typefinch-window.png" "$GITHUB_WORKSPACE/typefinch-gui-smoke.png"
  fi
  if [[ -n "${GITHUB_WORKSPACE:-}" ]] && [[ -f "$tmp/app.log" ]]; then
    cp "$tmp/app.log" "$GITHUB_WORKSPACE/typefinch-gui-smoke.log"
  fi
  rm -rf "$tmp"
}
trap cleanup EXIT
export XDG_CONFIG_HOME="$tmp/config"
export TYPEFINCH_QA_REPORT="$tmp/report.json"
./target/debug/typefinch >"$tmp/app.log" 2>&1 &
app_pid=$!
window=""
for _ in $(seq 1 60); do
  window="$(xdotool search --onlyvisible --name 'Typefinch' 2>/dev/null | head -n 1 || true)"
  if [[ -n "$window" ]]; then break; fi
  if ! kill -0 "$app_pid" 2>/dev/null; then
    echo "Typefinch exited before its window appeared"
    cat "$tmp/app.log"
    exit 1
  fi
  sleep 0.25
done
if [[ -z "$window" ]]; then
  echo "Typefinch window did not appear"
  cat "$tmp/app.log"
  exit 1
fi
xdotool windowactivate --sync "$window"
xdotool key --clearmodifiers F5
sleep 0.4
xdotool type --clearmodifiers --delay 55 "little finches practice every day"
sleep 2.6
if command -v import >/dev/null; then
  import -window "$window" "$tmp/typefinch-window.png"
fi
if [[ ! -s "$tmp/report.json" ]]; then
  echo "No completed scored session after synthetic typing"
  cat "$tmp/app.log"
  exit 1
fi
python3 - "$tmp/report.json" <<'PY'
import json,sys
r=json.load(open(sys.argv[1]))
assert r["synthetic"] and r["state"]=="finished",r
assert r["tests"] == 1, r
assert r["entered"] >= 8, r
assert r["correct"] >= 5, r
assert r["wpm"] > 0.0, r
assert 1500 <= r["elapsed_ms"] <= 2500, r
print(f"X11 GUI smoke PASS: {r['entered']} entered, {r['correct']} correct, {r['wpm']:.1f} WPM")
PY
if [[ ! -s "$tmp/typefinch-window.png" ]]; then
  echo "GUI screenshot missing"; exit 1
fi
echo "Native X11 window and synthetic typing session both verified."
