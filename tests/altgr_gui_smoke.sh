#!/usr/bin/env bash
# Synthetic Xvfb X11 keyboard input, NOT physical Hyprland or an app-local
# trusted RightAlt modifier. Verify compositor committed text is scored once.
set -Eeuo pipefail
export LIBGL_ALWAYS_SOFTWARE=1 WINIT_UNIX_BACKEND=x11 XDG_SESSION_TYPE=x11
tmp="$(mktemp -d)"
pid="" wm=""
cleanup(){
  status=$?
  if (( status != 0 )) && [[ -s "$tmp/app.log" ]];then
    tail -n 85 "$tmp/app.log" >&2 || true
  fi
  [[ -z "$pid" ]] || { kill "$pid" 2>/dev/null || true; wait "$pid" 2>/dev/null || true; }
  [[ -z "$wm" ]] || { kill "$wm" 2>/dev/null || true; wait "$wm" 2>/dev/null || true; }
  rm -rf "$tmp"
}
trap cleanup EXIT
mkdir -p "$tmp/config"
export XDG_CONFIG_HOME="$tmp/config"
export TYPEFINCH_QA_REPORT="$tmp/score.json"
export TYPEFINCH_QA_IMPORT_REPORT="$tmp/import.json"
setxkbmap -layout us
printf '%s' '! €' > "$tmp/shift-altgr.txt"
openbox --sm-disable >"$tmp/wm.log" 2>&1 &
wm=$!
./target/debug/typefinch >"$tmp/app.log" 2>&1 &
pid=$!
window=""
for _ in $(seq 1 200);do
  window="$(xdotool search --onlyvisible --name 'Typefinch' 2>/dev/null | head -n 1 || true)"
  [[ -z "$window" ]] || break
  kill -0 "$pid" 2>/dev/null || { echo 'Native AltGr app exited' >&2;exit 1; }
  sleep 0.25
done
[[ -n "$window" ]] || { echo 'Native AltGr GUI missing' >&2;exit 1; }
xdotool windowactivate --sync "$window" 2>/dev/null || true
xdotool windowfocus --sync "$window"
sleep 0.4
xdotool key --clearmodifiers F11
sleep 0.4
grep -q '"test_mode": "custom"' "$XDG_CONFIG_HOME/typefinch/settings.json"
xdotool type --clearmodifiers --delay 20 "$tmp/shift-altgr.txt"
xdotool key --clearmodifiers Return
sleep 0.45
lua5.4 - "$TYPEFINCH_QA_IMPORT_REPORT" <<'LUA'
local json=require("dkjson")
local f=assert(io.open(assert(arg[1]),"rb"),"Missing synthetic import audit")
local r=assert(json.decode(assert(f:read("*a"))));f:close()
assert(r.synthetic and r.state=="custom-import" and r.script=="other"
  and r.graphemes==3,"Native Shift/AltGr fixture not imported")
LUA
# Change ONLY the disposable X11 seat. The app remains in OS input mode.
setxkbmap -layout de
setxkbmap -query | grep -Eq '^layout:[[:space:]]+de$' || {
  echo 'Native German X11 layout was not applied' >&2;exit 1;
}
xdotool windowfocus --sync "$window"
xdotool key --clearmodifiers F5
sleep 0.25
xdotool key --clearmodifiers F2
sleep 0.2
xdotool key --clearmodifiers shift+1
xdotool key --clearmodifiers space
xdotool key --clearmodifiers ISO_Level3_Shift+e
sleep 0.65
lua5.4 - "$TYPEFINCH_QA_REPORT" <<'LUA'
local json=require("dkjson")
local f=assert(io.open(assert(arg[1]),"rb"),"No finished AltGr score")
local r=assert(json.decode(assert(f:read("*a"))));f:close()
assert(r.synthetic and r.state=="finished"
  and r.entered==3 and r.correct==3 and r.accuracy==100,
  string.format("Native OS Shift/AltGr failed entered=%s correct=%s state=%s",
    tostring(r.entered),tostring(r.correct),tostring(r.state)))
local dir=os.getenv("XDG_CONFIG_HOME")
local cmd=string.format("sqlite3 -json %q 'SELECT input_mode,test_mode,entered,correct,attempts FROM sessions'",
  dir.."/typefinch/sessions.sqlite3")
local pipe=assert(io.popen(cmd,"r"))
local rows=assert(json.decode(assert(pipe:read("*a"))));assert(pipe:close())
assert(#rows==1 and rows[1].input_mode=="OS" and rows[1].test_mode=="custom"
  and rows[1].entered==3 and rows[1].correct==3 and rows[1].attempts==3,
  "Native X11 modifier session score or provenance wrong")
local prefs=assert(io.open(dir.."/typefinch/settings.json","rb"))
local saved=assert(prefs:read("*a"));prefs:close()
assert(not saved:find("shift-altgr.txt",1,true),"Private fixture path persisted")
assert(not saved:find("! €",1,true),"Private custom text persisted")
print("Native X11 OS Shift+1 and RightAlt+E PASS: !, space, euro; 3/3 correct")
LUA
