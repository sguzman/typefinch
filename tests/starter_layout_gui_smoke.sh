#!/usr/bin/env bash
# Real native GUI scoring using physical X11 keycodes and Typefinch's
# installed-XKB software emulation. This validates the starter layouts'
# punctuation, Shift, and Colemak-DH bottom row, not physical hardware.
set -Eeuo pipefail
export LIBGL_ALWAYS_SOFTWARE=1 WINIT_UNIX_BACKEND=x11 XDG_SESSION_TYPE=x11
export TYPEFINCH_QA_XKB_COMPOSE=1
tmp="$(mktemp -d)"
app_pid="" wm_pid="" window=""
cleanup() {
  status=$?
  if ((status!=0));then
    for log in "$tmp"/*/app.log;do
      [[ ! -s "$log" ]] || tail -n 70 "$log" >&2 || true
    done
  fi
  [[ -z "$app_pid" ]] || { kill "$app_pid" 2>/dev/null || true; wait "$app_pid" 2>/dev/null || true; }
  [[ -z "$wm_pid" ]] || { kill "$wm_pid" 2>/dev/null || true; wait "$wm_pid" 2>/dev/null || true; }
  rm -rf "$tmp"
}
trap cleanup EXIT
setxkbmap -layout us
openbox --sm-disable >"$tmp/wm.log" 2>&1 &
wm_pid=$!
sleep 0.4
start_app(){
  local label="$1"
  mkdir -p "$tmp/$label/config"
  export XDG_CONFIG_HOME="$tmp/$label/config"
  export TYPEFINCH_QA_REPORT="$tmp/$label/result.json"
  ./target/debug/typefinch >"$tmp/$label/app.log" 2>&1 &
  app_pid=$!
  window=""
  for _ in $(seq 1 200);do
    window="$(xdotool search --onlyvisible --name 'Typefinch' 2>/dev/null|head -n1||true)"
    [[ -z "$window" ]] || break
    kill -0 "$app_pid" 2>/dev/null || { echo "Starter emulation app exited" >&2;exit 1; }
    sleep 0.25
  done
  [[ -n "$window" ]] || { echo "Starter emulation window missing" >&2;exit 1; }
  xdotool windowactivate --sync "$window" 2>/dev/null || true
  xdotool windowfocus --sync "$window"
  sleep 0.35
}
stop_app(){
  kill "$app_pid" 2>/dev/null || true
  wait "$app_pid" 2>/dev/null || true
  app_pid=""
  sleep 0.2
}
for label in dvorak colemak-dh;do
  if [[ "$label" == dvorak ]];then
    layout='us(dvorak)'
    passage='[]{}_?'
    codes=(20 21 shift+20 shift+21 shift+48 shift+34)
  else
    layout='us(colemak_dh)'
    passage='xcdvz'
    codes=(52 53 54 55 56)
  fi
  printf '%s' "$passage" >"$tmp/$label-source.txt"
  start_app "$label"
  xdotool key --clearmodifiers F11
  sleep 0.45
  cfg="$XDG_CONFIG_HOME/typefinch/settings.json"
  grep -q '"test_mode": "custom"' "$cfg" || {
    echo "F11 failed to select Custom mode for $label" >&2;exit 1;
  }
  stop_app
  lua5.4 - "$cfg" "$layout" <<'LUA'
local json=require("dkjson")
local path,layout=assert(arg[1]),assert(arg[2])
local f=assert(io.open(path,"rb"))
local preferences=assert(json.decode(assert(f:read("*a"))));f:close()
preferences.layout=layout
local out=assert(io.open(path,"wb"))
assert(out:write(json.encode(preferences,{indent=true})));assert(out:close())
LUA
  start_app "$label"
  xdotool key --clearmodifiers F11
  sleep 0.45
  xdotool type --clearmodifiers --delay 20 "$tmp/$label-source.txt"
  xdotool key --clearmodifiers Return
  sleep 0.5
  xdotool key --clearmodifiers F5
  sleep 0.25
  xdotool key --clearmodifiers F2
  sleep 0.25
  for code in "${codes[@]}";do
    xdotool key --clearmodifiers "$code"
    sleep 0.07
  done
  sleep 0.8
  lua5.4 - "$TYPEFINCH_QA_REPORT" "$layout" "${#codes[@]}" <<'LUA'
local json=require("dkjson")
local path,layout,count=assert(arg[1]),assert(arg[2]),assert(tonumber(arg[3]))
local f=assert(io.open(path,"rb"),"Starter XKB native session did not finish")
local result=assert(json.decode(assert(f:read("*a"))));f:close()
assert(result.synthetic and result.state=="finished"
  and result.entered==count and result.correct==count and result.accuracy==100,
  "Wrong starter layout text scored")
local db=os.getenv("XDG_CONFIG_HOME").."/typefinch/sessions.sqlite3"
local cmd=string.format("sqlite3 -json %q 'SELECT layout,input_mode,test_mode,entered,correct FROM sessions'",db)
local pipe=assert(io.popen(cmd,"r"))
local rows=assert(json.decode(assert(pipe:read("*a"))));assert(pipe:close())
assert(#rows==1 and rows[1].layout==layout and rows[1].input_mode=="Emulated"
  and rows[1].test_mode=="custom" and rows[1].entered==count and rows[1].correct==count,
  "Starter emulation SQLite identity/score incorrect")
local sf=assert(io.open(os.getenv("XDG_CONFIG_HOME").."/typefinch/settings.json","rb"))
local settings=assert(sf:read("*a"));sf:close()
assert(not settings:find(assert(arg[1]):gsub("result.json","source.txt"),1,true),
  "Unexpected fixture path in settings")
print("Native starter ANSI XKB emulation PASS: "..layout.." "..count.."/"..count)
LUA
  stop_app
done
echo 'Dvorak shifted punctuation and Colemak-DH bottom row native scoring PASS'
