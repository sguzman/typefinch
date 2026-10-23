#!/usr/bin/env bash
# Native egui keyboard-only goal transfer; synthetic files only, no user data.
set -Eeuo pipefail
export LIBGL_ALWAYS_SOFTWARE=1 WINIT_UNIX_BACKEND=x11 XDG_SESSION_TYPE=x11
tmp="$(mktemp -d)"
app_pid="" wm_pid=""
cleanup() {
  status=$?
  if ((status!=0)) && [[ -f "$tmp/app.log" ]]; then
    tail -n 80 "$tmp/app.log" >&2 || true
  fi
  [[ -z "$app_pid" ]] || { kill "$app_pid" 2>/dev/null || true; wait "$app_pid" 2>/dev/null || true; }
  [[ -z "$wm_pid" ]] || { kill "$wm_pid" 2>/dev/null || true; wait "$wm_pid" 2>/dev/null || true; }
  rm -rf "$tmp"
}
trap cleanup EXIT
mkdir -p "$tmp/config"
export XDG_CONFIG_HOME="$tmp/config"
# The only input is this hard-coded, synthetic goal transfer fixture.
cat >"$tmp/incoming.json" <<'JSON'
{"format":"typefinch-profile-goals","version":1,"goals":{"target_wpm":88.0,"target_accuracy":96.0,"daily_sessions":4,"show_streak":true}}
JSON
openbox --sm-disable >"$tmp/wm.log" 2>&1 &
wm_pid=$!
sleep 0.4
./target/debug/typefinch >"$tmp/app.log" 2>&1 &
app_pid=$!
window=""
for _ in $(seq 1 200); do
  window="$(xdotool search --onlyvisible --name 'Typefinch' 2>/dev/null | head -n 1 || true)"
  [[ -z "$window" ]] || break
  if ! kill -0 "$app_pid" 2>/dev/null; then
    echo "Goal transfer GUI exited before opening" >&2; cat "$tmp/app.log" >&2; exit 1
  fi
  sleep 0.25
done
[[ -n "$window" ]] || { echo "No goal transfer window" >&2; exit 1; }
xdotool windowactivate --sync "$window" 2>/dev/null || true
xdotool windowfocus --sync "$window"
sleep 0.5
xdotool key --clearmodifiers F12
sleep 0.5
xdotool type --clearmodifiers --delay 20 "$tmp/incoming.json"
xdotool key --clearmodifiers Return
sleep 0.6
lua5.4 tests/goals_gui_assert.lua imported "$XDG_CONFIG_HOME/typefinch/settings.json" "$tmp/incoming.json"
xdotool key --clearmodifiers shift+F12
for _ in $(seq 1 40); do
  [[ -s "$XDG_CONFIG_HOME/typefinch/exports/typefinch-goals.json" ]] && break
  sleep 0.1
done
lua5.4 tests/goals_gui_assert.lua exported "$XDG_CONFIG_HOME/typefinch/exports/typefinch-goals.json" "$tmp/incoming.json"
echo "Native goal transfer PASS: keyboard import/export and per-profile persistence"
