#!/usr/bin/env bash
# Synthetic X11 physical-position Key events through installed XKB emulation.
# Tests Arabic and Hebrew committed characters, without claiming real
# hardware, compositor seat identity, an IME or live Wayland acceptance.
set -Eeuo pipefail
export LIBGL_ALWAYS_SOFTWARE=1 WINIT_UNIX_BACKEND=x11 XDG_SESSION_TYPE=x11
tmp="$(mktemp -d)"
app_pid="" wm_pid="" window=""
cleanup(){
  status=$?
  if (( status != 0 ));then
    for log in "$tmp"/*/app.log;do
      [[ ! -s "$log" ]] || tail -n 65 "$log" >&2 || true
    done
  fi
  [[ -z "$app_pid" ]] || {kill "$app_pid" 2>/dev/null || true;wait "$app_pid" 2>/dev/null || true;}
  [[ -z "$wm_pid" ]] || {kill "$wm_pid" 2>/dev/null || true;wait "$wm_pid" 2>/dev/null || true;}
  rm -rf "$tmp"
}
trap cleanup EXIT
openbox --sm-disable >"$tmp/wm.log" 2>&1 &
wm_pid=$!
sleep 0.4
start_app(){
  local layout="$1"
  mkdir -p "$tmp/$layout/config"
  export XDG_CONFIG_HOME="$tmp/$layout/config"
  export TYPEFINCH_QA_REPORT="$tmp/$layout/report.json"
  export TYPEFINCH_QA_IMPORT_REPORT="$tmp/$layout/import.json"
  ./target/debug/typefinch >"$tmp/$layout/app.log" 2>&1 &
  app_pid=$!
  window=""
  for _ in $(seq 1 200);do
    window="$(xdotool search --onlyvisible --name 'Typefinch' 2>/dev/null|head -n 1||true)"
    [[ -z "$window" ]] || break
    kill -0 "$app_pid" 2>/dev/null || { echo "Typefinch RTL XKB app exited" >&2;exit 1; }
    sleep 0.25
  done
  [[ -n "$window" ]] || {echo "No RTL XKB window" >&2;exit 1;}
  xdotool windowactivate --sync "$window" 2>/dev/null || true
  xdotool windowfocus --sync "$window"
  sleep 0.35
}
stop_app(){
  kill "$app_pid" 2>/dev/null || true
  wait "$app_pid" 2>/dev/null || true
  app_pid="" window=""
  sleep 0.25
}
for layout in ara il;do
  if [[ "$layout" == ara ]];then
    expected=arabic
    keys=(s g h l)
    printf '%s' 'سلام' >"$tmp/$layout-passage.txt"
  else
    expected=hebrew
    keys=(a k u o)
    printf '%s' 'שלום' >"$tmp/$layout-passage.txt"
  fi
  start_app "$layout"
  # Native F11 creates a real, versioned settings file and chooses Custom.
  xdotool key --clearmodifiers F11
  sleep 0.4
  cfg="$XDG_CONFIG_HOME/typefinch/settings.json"
  grep -q '"test_mode": "custom"' "$cfg" || {
    echo "Initial RTL configuration not saved" >&2;exit 1;
  }
  stop_app
  # Explicit synthetic QA configuration, not a bypass of the key-event
  # translation path. The production executable compiles this real XKB map.
  lua5.4 - "$cfg" "$layout" <<'LUA'
local json=require("dkjson")
local path,layout=assert(arg[1]),assert(arg[2])
local fd=assert(io.open(path,"rb"))
local prefs=assert(json.decode(assert(fd:read("*a"))));fd:close()
prefs.layout=layout
local output=assert(io.open(path,"wb"))
assert(output:write(json.encode(prefs,{indent=true})));assert(output:close())
LUA
  export TYPEFINCH_QA_XKB_COMPOSE=1
  start_app "$layout"
  xdotool key --clearmodifiers F11
  sleep 0.45
  xdotool type --clearmodifiers --delay 25 "$tmp/$layout-passage.txt"
  xdotool key --clearmodifiers Return
  sleep 0.55
  lua5.4 - "$TYPEFINCH_QA_IMPORT_REPORT" "$expected" <<'LUA'
local json=require("dkjson")
local f=assert(io.open(assert(arg[1]),"rb"),"No explicit synthetic import evidence")
local data=assert(json.decode(assert(f:read("*a"))));f:close()
assert(data.synthetic and data.state=="custom-import" and data.script==arg[2]
  and data.graphemes==4,"RTL XKB fixture identity mismatch")
LUA
  xdotool windowfocus --sync "$window"
  xdotool key --clearmodifiers F5
  sleep 0.3
  xdotool key --clearmodifiers F2
  sleep 0.25
  for key in "${keys[@]}";do
    xdotool key --clearmodifiers "$key"
    sleep 0.08
  done
  sleep 0.8
  lua5.4 - "$TYPEFINCH_QA_REPORT" "$layout" <<'LUA'
local json=require("dkjson")
local report,layout=assert(arg[1]),assert(arg[2])
local f=assert(io.open(report,"rb"),"No scored RTL native XKB output")
local r=assert(json.decode(assert(f:read("*a"))));f:close()
assert(r.synthetic and r.state=="finished" and r.entered==4 and r.correct==4
  and r.accuracy==100,"Physical-position XKB event did not score RTL character")
local path=os.getenv("XDG_CONFIG_HOME").."/typefinch/sessions.sqlite3"
local sql=string.format("sqlite3 -json %q 'SELECT layout,input_mode,test_mode,entered,correct FROM sessions'",path)
local pipe=assert(io.popen(sql,"r"))
local rows=assert(json.decode(assert(pipe:read("*a"))))
assert(pipe:close())
assert(#rows==1 and rows[1].layout==layout and rows[1].input_mode=="Emulated"
  and rows[1].test_mode=="custom" and rows[1].entered==4 and rows[1].correct==4,
  "RTL XKB scored SQLite record did not come from expected emulation")
print("Native X11 RTL XKB key events PASS: "..layout..", 4/4 correct, Emulated")
LUA
  stop_app
  unset TYPEFINCH_QA_XKB_COMPOSE
done
echo 'Arabic + Hebrew X11 XKB emulation scored real virtual key-position events; live Wayland, IME and physical-device acceptance remain open'
