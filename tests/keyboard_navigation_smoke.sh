#!/usr/bin/env bash
# End-to-end keyboard-only egui navigation; no clicks, privileged hooks or QA bypass.
set -Eeuo pipefail
export LIBGL_ALWAYS_SOFTWARE=1 WINIT_UNIX_BACKEND=x11 XDG_SESSION_TYPE=x11
tmp="$(mktemp -d)"
app_pid="" wm_pid=""
cleanup(){
  status=$?
  if (( status != 0 )) && [[ -s "$tmp/app.log" ]]; then
    echo "Keyboard navigation QA app log:" >&2
    tail -n 60 "$tmp/app.log" >&2 || true
  fi
  if [[ -n "$app_pid" ]]; then
    kill "$app_pid" 2>/dev/null || true
    wait "$app_pid" 2>/dev/null || true
  fi
  if [[ -n "$wm_pid" ]]; then
    kill "$wm_pid" 2>/dev/null || true
    wait "$wm_pid" 2>/dev/null || true
  fi
  rm -rf "$tmp"
}
trap cleanup EXIT
export XDG_CONFIG_HOME="$tmp/config"
mkdir -p "$XDG_CONFIG_HOME"
openbox --sm-disable >"$tmp/wm.log" 2>&1 &
wm_pid=$!
sleep 0.4
./target/debug/typefinch >"$tmp/app.log" 2>&1 &
app_pid=$!
window=""
for _ in $(seq 1 200);do
  window="$(xdotool search --onlyvisible --name 'Typefinch' 2>/dev/null | head -n 1 || true)"
  [[ -z "$window" ]] || break
  if ! kill -0 "$app_pid" 2>/dev/null;then cat "$tmp/app.log" >&2;exit 1;fi
  sleep 0.25
done
[[ -n "$window" ]] || { echo "Keyboard navigation UI missing" >&2; exit 1; }
xdotool windowactivate --sync "$window" 2>/dev/null || true
xdotool windowfocus --sync "$window"
sleep 0.7
import -window "$window" "$tmp/help-closed.png"
# Capturing an X11 window may disturb keyboard focus.
xdotool windowactivate --sync "$window" 2>/dev/null || true
xdotool windowfocus --sync "$window"
xdotool key --clearmodifiers F1
sleep 0.5
import -window "$window" "$tmp/help-open.png"
diff_pixels="$(compare -metric AE "$tmp/help-closed.png" "$tmp/help-open.png" null: 2>&1 || true)"
[[ "$diff_pixels" =~ ^[0-9]+$ && "$diff_pixels" -gt 300 ]] || {
  echo "F1 failed to reveal native keyboard guide: $diff_pixels pixels" >&2;exit 1;
}
xdotool windowactivate --sync "$window" 2>/dev/null || true
xdotool windowfocus --sync "$window"
xdotool key --clearmodifiers F1
sleep 0.3
# Ctrl+N focuses the actual production TextEdit, not a QA-only input path.
xdotool key --clearmodifiers ctrl+n
sleep 0.4
xdotool type --clearmodifiers --delay 28 "Keyboard QA"
sleep 0.3
xdotool key --clearmodifiers Tab
sleep 0.3
# Prove the reverse focus path reaches the text editor and returns.
xdotool key --clearmodifiers shift+Tab
sleep 0.3
xdotool type --clearmodifiers --delay 28 "2"
xdotool key --clearmodifiers Tab
sleep 0.3
xdotool key --clearmodifiers Return
sleep 0.6
settings="$XDG_CONFIG_HOME/typefinch/settings.json"
[[ -s "$settings" ]] || { echo "No profile settings written" >&2; exit 1; }
lua5.4 - "$settings" <<'LUA'
local json=require("dkjson")
local f=assert(io.open(assert(arg[1]),"rb"))
local data,_,err=json.decode(f:read("*a"));f:close()
assert(not err and type(data.profiles)=="table")
assert(#data.profiles==2,"Tab/Enter must create exactly one new profile, not bypass validation")
local n=0
for _,p in ipairs(data.profiles) do
  if p.name=="Keyboard QA2" then
    n=n+1
    assert(type(p.id)=="string" and #p.id>0)
    assert(p.tests==0 and p.best_wpm==0)
  end
end
assert(n==1,"Ctrl+N, Tab, Shift+Tab and Enter must navigate the real profile editor")
print("Native keyboard profile creation PASS: Ctrl+N, Tab, Shift+Tab, Enter")
LUA
echo "Native keyboard guide PASS: F1 altered $diff_pixels rendered pixels"
