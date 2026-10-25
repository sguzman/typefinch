#!/usr/bin/env bash
# T305: JIS physical RO/YEN and non-scoring conversion-key experimental probe.
# Run in an isolated Xvfb CI seat. Does NOT certify actual hardware,
# device-specific identity, Wayland, or production Hyprland support.
set -Eeuo pipefail
export LIBGL_ALWAYS_SOFTWARE=1 WINIT_UNIX_BACKEND=x11 XDG_SESSION_TYPE=x11
export TYPEFINCH_QA_XKB_COMPOSE=1 TYPEFINCH_QA_DURATION_SECONDS=15
tmp="$(mktemp -d)"
app='' wm='' window=''
cleanup(){
  local code=$?
  if ((code!=0));then
    [[ ! -s "$tmp/app.log" ]] || tail -n 70 "$tmp/app.log" >&2
  fi
  [[ -z "$app" ]] || { kill "$app" 2>/dev/null || true; wait "$app" 2>/dev/null || true; }
  [[ -z "$wm" ]] || { kill "$wm" 2>/dev/null || true; wait "$wm" 2>/dev/null || true; }
  rm -rf "$tmp"
}
trap cleanup EXIT
export XDG_CONFIG_HOME="$tmp/config"
export TYPEFINCH_QA_REPORT="$tmp/result.json" TYPEFINCH_QA_START_REPORT="$tmp/started.json"
mkdir -p "$XDG_CONFIG_HOME"
# Host XKB is US; Japanese characters MUST originate from Typefinch's
# Japanese software-emulated physical layout rather than committed OS text.
setxkbmap -layout us -model pc105
openbox --sm-disable >"$tmp/wm.log" 2>&1 &
wm=$!
sleep 0.4
start_app(){
  ./target/debug/typefinch >"$tmp/app.log" 2>&1 &
  app=$!
  window=''
  for _ in $(seq 1 120);do
    window="$(xdotool search --onlyvisible --pid "$app" --name Typefinch 2>/dev/null | head -n1 || true)"
    [[ -z "$window" ]] || break
    kill -0 "$app" 2>/dev/null || { echo "Migrated emulation app exited" >&2;exit 1; }
    sleep 0.25
  done
  [[ -n "$window" ]] || { echo "Migrated emulation window absent" >&2;exit 1; }
  xdotool windowactivate --sync "$window" 2>/dev/null || true
  xdotool windowfocus --sync "$window"
  sleep 0.35
}
stop_app(){
  kill "$app" 2>/dev/null || true
  wait "$app" 2>/dev/null || true
  app=''
  sleep 0.2
}
# Generate a disposable local settings identity using the real UI.
start_app
for _ in $(seq 1 30);do
  if [[ -f "$XDG_CONFIG_HOME/typefinch/settings.json" ]] &&
     grep -q '"test_mode": "custom"' "$XDG_CONFIG_HOME/typefinch/settings.json";then
    break
  fi
  xdotool key --clearmodifiers F11
  sleep 0.2
done
grep -q '"test_mode": "custom"' "$XDG_CONFIG_HOME/typefinch/settings.json" ||
 { echo "Custom mode not selected" >&2;exit 1; }
stop_app
lua5.4 - "$XDG_CONFIG_HOME/typefinch/settings.json" <<'LUA'
local json=require("dkjson")
local path=assert(arg[1])
local f=assert(io.open(path,"rb"));local prefs=assert(json.decode(f:read("*a")));f:close()
prefs.layout="jp(OADG109A)"
local out=assert(io.open(path,"wb"))
assert(out:write(json.encode(prefs,{indent=true})));assert(out:close())
LUA

printf '%s' '\_¥|' > "$tmp/source.txt"
start_app
# Native file import; no fixture content passed as an app command-line arg.
xdotool key --clearmodifiers F11
sleep 0.4
xdotool type --clearmodifiers --delay 22 "$tmp/source.txt"
xdotool key --clearmodifiers Return
sleep 0.6
xdotool key --clearmodifiers F5
sleep 0.3
xdotool key --clearmodifiers F2
sleep 0.3

# Negative controls: Japanese IME-mode keys are real physical positions,
# but may never manufacture a scored symbol without a text commit.
# Linux evdev 85/92/93/94 become X11 93/100/101/102.
xdotool key --clearmodifiers 93
xdotool key --clearmodifiers 100
xdotool key --clearmodifiers 101
xdotool key --clearmodifiers 102
sleep 0.16

# RO (KEY_RO evdev 89 = X11 97) Base and Shift.
xdotool key --clearmodifiers 97
sleep 0.17
xdotool key --clearmodifiers shift+97
sleep 0.17
# YEN (KEY_YEN evdev 124 = X11 132) Base and Shift.
xdotool key --clearmodifiers 132
sleep 0.17
xdotool key --clearmodifiers shift+132
sleep 0.7

lua5.4 - "$TYPEFINCH_QA_REPORT" "$tmp/source.txt" <<'LUA'
local json=require("dkjson")
local result_path,source=assert(arg[1]),assert(arg[2])
local file=assert(io.open(result_path,"rb"),"Physical Level-3/ISO fixture did not finish")
local result=assert(json.decode(file:read("*a")));file:close()
assert(result.synthetic and result.state=="finished" and result.entered==4
  and result.correct==4 and result.accuracy==100,
  string.format("Native JIS physical mismatch state=%s entered=%s correct=%s accuracy=%s",
    tostring(result.state),tostring(result.entered),tostring(result.correct),
    tostring(result.accuracy)))
local database=os.getenv("XDG_CONFIG_HOME").."/typefinch/sessions.sqlite3"
local cmd=string.format("sqlite3 -json %q 'SELECT layout,input_mode,test_mode,entered,correct,attempts FROM sessions'",database)
local p=assert(io.popen(cmd,"r"))
local rows=assert(json.decode(p:read("*a")));assert(p:close())
assert(#rows==1 and rows[1].layout=="jp(OADG109A)" and rows[1].input_mode=="Emulated"
  and rows[1].test_mode=="custom" and rows[1].entered==4
  and rows[1].correct==4 and rows[1].attempts==4,
  "Real Japanese RO/YEN physical positions or persisted session identity invalid")
local conf=assert(io.open(os.getenv("XDG_CONFIG_HOME").."/typefinch/settings.json","rb"))
local settings=conf:read("*a");conf:close()
local ff=assert(io.open(source,"rb"));local text=ff:read("*a");ff:close()
assert(not settings:find(source,1,true) and not settings:find(text,1,true),
  "Disposable Unicode test fixture or its path leaked to settings")
print("Native Japanese physical RO/YEN Base/Shift PASS: 4/4; non-printing IME controls unscored")
LUA
