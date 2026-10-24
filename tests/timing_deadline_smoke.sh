#!/usr/bin/env bash
# Real native X11/egui deadline test in a disposable profile. The boundary
# semantics are also covered without sleeps by engine::tests::timed_deadline_*.
# No physical-device attribution is claimed.
set -euo pipefail
: "${DISPLAY:?Run this under disposable Xvfb}"
tmp="$(mktemp -d)"
app_pid=""
cleanup() {
  if [[ -n "$app_pid" ]]; then
    kill "$app_pid" 2>/dev/null || true
    wait "$app_pid" 2>/dev/null || true
  fi
  rm -rf "$tmp"
}
trap cleanup EXIT
export XDG_CONFIG_HOME="$tmp/config"
mkdir -p "$XDG_CONFIG_HOME"
export TYPEFINCH_QA_REPORT="$tmp/deadline.json"
# xvfb-run can return before its X11 socket is accepting clients on a
# loaded runner. Do not mistake an initial XOpenDisplayFailed for an app
# or deadline regression. This probe never touches a user display.
display_ready=0
for _ in $(seq 1 50);do
  if xdotool getdisplaygeometry >/dev/null 2>&1;then
    display_ready=1
    break
  fi
  sleep 0.1
done
if ((display_ready!=1));then
  echo "Disposable Xvfb display never accepted an X11 client" >&2
  exit 1
fi
./target/debug/typefinch >"$tmp/app.log" 2>&1 &
app_pid=$!
window=""
for _ in $(seq 1 120); do
  window="$(xdotool search --onlyvisible --name 'Typefinch' 2>/dev/null | head -n 1 || true)"
  [[ -n "$window" ]] && break
  if ! kill -0 "$app_pid" 2>/dev/null; then
    cat "$tmp/app.log" >&2
    echo "Native deadline test window exited" >&2
    exit 1
  fi
  sleep 0.1
done
if [[ -z "$window" ]]; then
  cat "$tmp/app.log" >&2
  echo "Native deadline test window not visible" >&2
  exit 1
fi
xdotool windowactivate --sync "$window" 2>/dev/null || true
xdotool windowfocus --sync "$window"
sleep 0.4
xdotool windowfocus --sync "$window"
xdotool key --clearmodifiers F5
sleep 0.3
xdotool type --clearmodifiers --delay 50 "cat"
# A disposable QA session uses the existing 2-second duration override.
for _ in $(seq 1 70); do
  [[ -s "$tmp/deadline.json" ]] && break
  if ! kill -0 "$app_pid" 2>/dev/null; then cat "$tmp/app.log" >&2; exit 1; fi
  sleep 0.1
done
if [[ ! -s "$tmp/deadline.json" ]]; then
  cat "$tmp/app.log" >&2
  echo "Timed session never completed" >&2
  exit 1
fi
# These events are deliberately sent after the persisted completion marker.
xdotool windowfocus --sync "$window"
xdotool type --clearmodifiers --delay 10 "latekeystrokes"
sleep 0.4
lua5.4 - "$tmp/deadline.json" "$XDG_CONFIG_HOME/typefinch/sessions.sqlite3" <<'LUA'
local json=require("dkjson")
local report_file=assert(io.open(assert(arg[1]),"rb"))
local report,_,err=json.decode(report_file:read("*a"))
report_file:close()
assert(not err and report.synthetic==true and report.state=="finished")
assert(report.tests==1,"exactly one native session should be saved")
assert(report.entered>=1 and report.entered<=3,
  "only the pre-deadline test input may be scored")
assert(report.elapsed_ms==2000,"elapsed time must stop exactly at timed deadline")
local dbpath=assert(arg[2])
local quote=function(s)return "'"..s:gsub("'","'\\''").."'"end
local cmd="sqlite3 -json "..quote(dbpath).." "..quote(
  "SELECT duration_ms,elapsed_ms,entered,correct,test_mode FROM sessions ORDER BY id")
local pipe=assert(io.popen(cmd,"r"))
local rows,_,db_error=json.decode(pipe:read("*a"))
local ok=pipe:close()
assert(ok and not db_error and type(rows)=="table")
assert(#rows==1,"late typing cannot create another score")
assert(rows[1].test_mode=="words")
assert(rows[1].duration_ms==2000 and rows[1].elapsed_ms==2000)
assert(rows[1].entered==report.entered and rows[1].correct==report.correct,
  "late keystrokes must not modify the SQLite score")
print("Native timed deadline PASS: exactly 2000ms, "..report.entered..
  " pre-deadline keys, no late input or extra score")
LUA
