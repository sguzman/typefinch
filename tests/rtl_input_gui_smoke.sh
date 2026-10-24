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
  [[ -z "$app_pid" ]] || { kill "$app_pid" 2>/dev/null || true; wait "$app_pid" 2>/dev/null || true; }
  [[ -z "$wm_pid" ]] || { kill "$wm_pid" 2>/dev/null || true; wait "$wm_pid" 2>/dev/null || true; }
  if [[ -n "${GITHUB_WORKSPACE:-}" ]];then
    for image in "$tmp"/rtl-native-mixed-*.png;do
      [[ ! -s "$image" ]] || cp "$image" "$GITHUB_WORKSPACE/"
    done
  fi
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
  [[ -n "$window" ]] || { echo "No RTL XKB window" >&2; exit 1; }
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
# Verify ordinary OS-input scoring with genuinely changed X11 keyboard layout
# and numeric raw X11 keycodes (evdev + 8). xdotool key 38 sends the
# currently mapped X key position, not a requested Latin/RTL keysym.
# The file-path importer runs under US XKB first to avoid synthetic text
# remapping the installed RTL keyboard layer behind this test's back.
for layout in ara il;do
  setxkbmap -layout us
  start_app "os-$layout"
  xdotool key --clearmodifiers F11
  sleep 0.45
  xdotool type --clearmodifiers --delay 25 "$tmp/$layout-passage.txt"
  xdotool key --clearmodifiers Return
  sleep 0.55
  if [[ "$layout" == ara ]];then expected=arabic; codes=(39 42 43 46); wrong=38;
  else expected=hebrew; codes=(38 45 30 32); wrong=39;fi
  lua5.4 - "$TYPEFINCH_QA_IMPORT_REPORT" "$expected" <<'LUA'
local json=require("dkjson")
local f=assert(io.open(assert(arg[1]),"rb"),"No native OS RTL import report")
local r=assert(json.decode(assert(f:read("*a"))));f:close()
assert(r.synthetic and r.script==arg[2] and r.graphemes==4,
  "Native OS RTL fixture identity mismatch")
LUA
  setxkbmap -layout "$layout"
  setxkbmap -query | grep -Eq "^layout:[[:space:]]+$layout$" || {
    echo "X11 native OS layout was not applied: $layout" >&2; exit 1;
  }
  xdotool windowfocus --sync "$window"
  xdotool key --clearmodifiers F5
  sleep 0.35
  xdotool key --clearmodifiers F2
  sleep 0.25
  # The first typed grapheme is intentionally wrong, then corrected.
  # Neither a wrong key nor its Backspace should complete this word.
  xdotool key --clearmodifiers "$wrong"
  sleep 0.20
  [[ ! -s "$TYPEFINCH_QA_REPORT" ]] || {
    echo "Wrong RTL key completed the native OS session" >&2;exit 1;
  }
  xdotool key --clearmodifiers BackSpace
  sleep 0.16
  for code in "${codes[@]}";do
    xdotool key --clearmodifiers "$code"
    sleep 0.08
  done
  sleep 0.85
  lua5.4 - "$TYPEFINCH_QA_REPORT" <<'LUA'
local json=require("dkjson")
local f=assert(io.open(assert(arg[1]),"rb"),"Native OS RTL score missing")
local r=assert(json.decode(assert(f:read("*a"))));f:close()
assert(r.synthetic and r.state=="finished" and r.entered==4
  and r.correct==4 and r.accuracy==100,"Native OS RTL layout failed to score")
local path=os.getenv("XDG_CONFIG_HOME").."/typefinch/sessions.sqlite3"
local sql=string.format("sqlite3 -json %q 'SELECT input_mode,test_mode,entered,correct,attempts,mistakes,corrections FROM sessions'",path)
local pipe=assert(io.popen(sql,"r"))
local rows=assert(json.decode(assert(pipe:read("*a"))))
assert(pipe:close())
assert(#rows==1 and rows[1].input_mode=="OS" and rows[1].test_mode=="custom"
  and rows[1].entered==4 and rows[1].correct==4
  and rows[1].attempts==5 and rows[1].mistakes==1 and rows[1].corrections==1,
  "Native OS-layout RTL mistake/correction metadata wrong")
print("Native X11 OS-layout RTL typing PASS: 4/4 after one corrected error, raw mistake retained")
LUA
  stop_app
done
# The isolated mixed-script screenshot scenario needs time to capture four
# frames. Other native deadline QA retains the default synthetic 2 seconds.
export TYPEFINCH_QA_DURATION_SECONDS=15
# A single real scored custom passage spans three genuine OS XKB layouts.
# Language switches come from isolated Xvfb test setup, never Typefinch.
# The native reference remains in logical Unicode order while COSMIC shapes
# the adjacent Arabic/Latin/Hebrew words in the correct visual directions.
setxkbmap -layout us
printf '%s' 'سلام abc שלום' >"$tmp/mixed-native-passage.txt"
start_app "mixed-native"
xdotool key --clearmodifiers F11
sleep 0.45
xdotool type --clearmodifiers --delay 25 "$tmp/mixed-native-passage.txt"
xdotool key --clearmodifiers Return
sleep 0.5
lua5.4 - "$TYPEFINCH_QA_IMPORT_REPORT" <<'LUA'
local json=require("dkjson")
local f=assert(io.open(assert(arg[1]),"rb"),"No mixed RTL import audit")
local x=assert(json.decode(assert(f:read("*a"))));f:close()
assert(x.synthetic and x.state=="custom-import"
  and x.script=="mixed" and x.graphemes==13,
  "Real mixed Arabic/Latin/Hebrew reference was not loaded")
LUA
xdotool windowfocus --sync "$window"
xdotool key --clearmodifiers F5
sleep 0.3
xdotool key --clearmodifiers F2
sleep 0.25
import -window "$window" "$tmp/rtl-native-mixed-start.png"
setxkbmap -layout ara
for code in 39 42 43 46;do
  xdotool key --clearmodifiers "$code"
  sleep 0.07
done
xdotool key --clearmodifiers 65
sleep 0.18
import -window "$window" "$tmp/rtl-native-mixed-arabic.png"
setxkbmap -layout us
for code in 38 56 54;do
  xdotool key --clearmodifiers "$code"
  sleep 0.07
done
xdotool key --clearmodifiers 65
sleep 0.18
import -window "$window" "$tmp/rtl-native-mixed-latin.png"
setxkbmap -layout il
for code in 38 45 30 32;do
  xdotool key --clearmodifiers "$code"
  sleep 0.07
done
sleep 0.75
import -window "$window" "$tmp/rtl-native-mixed-hebrew.png"
for pair in start:arabic arabic:latin latin:hebrew;do
  earlier="${pair%%:*}" later="${pair##*:}"
  pixels="$(compare -metric AE "$tmp/rtl-native-mixed-$earlier.png" "$tmp/rtl-native-mixed-$later.png" null: 2>&1 || true)"
  [[ "$pixels" =~ ^[0-9]+$ && "$pixels" -gt 100 ]] || {
    echo "Native mixed RTL visual transition absent: $pair ($pixels pixels)" >&2
    exit 1
  }
done
lua5.4 - "$TYPEFINCH_QA_REPORT" <<'LUA'
local json=require("dkjson")
local f=assert(io.open(assert(arg[1]),"rb"),"Mixed native X11 score missing")
local r=assert(json.decode(assert(f:read("*a"))));f:close()
assert(r.synthetic and r.state=="finished"
  and r.entered==13 and r.correct==13 and r.accuracy==100,
  string.format("Native mixed RTL score wrong: entered=%s correct=%s elapsed_ms=%s",
    tostring(r.entered),tostring(r.correct),tostring(r.elapsed_ms)))
local path=os.getenv("XDG_CONFIG_HOME").."/typefinch/sessions.sqlite3"
local q=string.format("sqlite3 -json %q 'SELECT input_mode,test_mode,entered,correct FROM sessions'",path)
local p=assert(io.popen(q,"r"))
local rows=assert(json.decode(assert(p:read("*a"))));assert(p:close())
assert(#rows==1 and rows[1].input_mode=="OS"
  and rows[1].test_mode=="custom" and rows[1].entered==13
  and rows[1].correct==13,"Native mixed passage SQLite provenance wrong")
print("Native mixed XKB input PASS: Arabic -> US Latin -> Hebrew, 13/13 OS graphemes")
LUA
stop_app
setxkbmap -layout us
echo 'Arabic/Hebrew emulated, native OS and mixed-direction virtual key-position scoring PASS; IME/physical Hyprland still open'
