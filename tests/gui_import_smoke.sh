#!/usr/bin/env bash
# Real keyboard-operated egui custom-file path import under disposable Xvfb.
# The fixture has no user data; no privileged keyboard input or interception.
set -Eeuo pipefail
export LIBGL_ALWAYS_SOFTWARE=1 WINIT_UNIX_BACKEND=x11 XDG_SESSION_TYPE=x11
tmp="$(mktemp -d)"
app_pid="" wm_pid=""
cleanup() {
  status=$?
  if (( status != 0 )) && [[ -f "$tmp/app.log" ]]; then
    echo "Native import QA failed; app diagnostics:" >&2
    tail -n 100 "$tmp/app.log" >&2 || true
  fi
  [[ -z "$app_pid" ]] || { kill "$app_pid" 2>/dev/null || true; wait "$app_pid" 2>/dev/null || true; }
  [[ -z "$wm_pid" ]] || { kill "$wm_pid" 2>/dev/null || true; wait "$wm_pid" 2>/dev/null || true; }
  rm -rf "$tmp"
}
trap cleanup EXIT
openbox --sm-disable >"$tmp/wm.log" 2>&1 &
wm_pid=$!
sleep 0.4
start_app() {
  ./target/debug/typefinch >"$tmp/app.log" 2>&1 &
  app_pid=$!
  window=""
  for _ in $(seq 1 200); do
    window="$(xdotool search --onlyvisible --name 'Typefinch' 2>/dev/null | head -n 1 || true)"
    [[ -z "$window" ]] || break
    if ! kill -0 "$app_pid" 2>/dev/null; then
      echo "Typefinch exited before import GUI appeared" >&2; cat "$tmp/app.log" >&2; exit 1
    fi
    sleep 0.25
  done
  [[ -n "$window" ]] || { echo "No Typefinch import window" >&2; exit 1; }
  xdotool windowactivate --sync "$window" 2>/dev/null || true
  xdotool windowfocus --sync "$window"
  sleep 0.5
}
stop_app() {
  kill "$app_pid" 2>/dev/null || true
  wait "$app_pid" 2>/dev/null || true
  app_pid=""
  sleep 0.2
}
keyboard_import() {
  local path="$1"
  xdotool key --clearmodifiers ctrl+o
  sleep 0.6
  xdotool type --clearmodifiers --delay 20 "$path"
  xdotool key --clearmodifiers Return
  sleep 0.5
  xdotool key --clearmodifiers F5
  sleep 0.4
  # Explicitly assert typed-event focus; the import field may still own
  # editor focus after Return in some native egui/X11 frame sequences.
  xdotool key --clearmodifiers F2
  sleep 0.3
  xdotool type --clearmodifiers --delay 35 "cat dog sun moon"
  sleep 2.5
}
mkdir -p "$tmp/good" "$tmp/rejected"
printf '%s' 'cat dog sun moon' >"$tmp/passage.txt"
ln -s "$tmp/passage.txt" "$tmp/link.txt"
export XDG_CONFIG_HOME="$tmp/good"
export TYPEFINCH_QA_REPORT="$tmp/good/report.json"
start_app
keyboard_import "$tmp/passage.txt"
lua5.4 tests/smoke_assert.lua native-file-import \
  "$tmp/good/report.json" "$tmp/good/typefinch/settings.json" "$tmp/passage.txt"
stop_app
# A rejected symbolic link in a new profile must not create a scored result.
export XDG_CONFIG_HOME="$tmp/rejected"
export TYPEFINCH_QA_REPORT="$tmp/rejected/report.json"
start_app
keyboard_import "$tmp/link.txt"
[[ ! -s "$tmp/rejected/report.json" ]] || { echo "Rejected import created a score" >&2; exit 1; }
db="$tmp/rejected/typefinch/sessions.sqlite3"
if [[ -f "$db" ]]; then
  [[ "$(sqlite3 "$db" 'SELECT COUNT(*) FROM sessions')" = 0 ]] || {
    echo "Rejected import persisted a session" >&2; exit 1;
  }
fi
grep -q '"test_mode": "custom"' "$tmp/rejected/typefinch/settings.json" || {
  echo "Ctrl+O failed to switch to Custom mode" >&2; exit 1;
}
echo "Native custom file symlink refusal PASS: no scored session"
