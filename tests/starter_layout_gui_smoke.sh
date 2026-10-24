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
for label in dvorak colemak-dh dvorak-offline colemak-dh-offline;do
  if [[ "$label" == dvorak* ]];then
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
  if [[ "$label" == *-offline ]];then
    # Force installed-XKB compilation to fail inside Typefinch, while
    # the disposable Xvfb server keeps its known US physical keymap.
    # Verify the offline starter fallback scores identical physical keys.
    mkdir -p "$tmp/empty-xkb-root"
    export XKB_CONFIG_ROOT="$tmp/empty-xkb-root"
  fi
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
  lua5.4 - "$TYPEFINCH_QA_REPORT" "$layout" "${#codes[@]}" "$tmp/$label-source.txt" <<'LUA'
local json=require("dkjson")
local path,layout,count,source=assert(arg[1]),assert(arg[2]),assert(tonumber(arg[3])),assert(arg[4])
local f=assert(io.open(path,"rb"),"Starter XKB native session did not finish")
local result=assert(json.decode(assert(f:read("*a"))));f:close()
assert(result.synthetic and result.state=="finished"
  and result.entered==count and result.correct==count and result.accuracy==100,
  "Wrong starter layout text scored")
local db=os.getenv("XDG_CONFIG_HOME").."/typefinch/sessions.sqlite3"
local cmd=string.format("sqlite3 -json %q 'SELECT layout,input_mode,test_mode,entered,correct,attempts FROM sessions'",db)
local pipe=assert(io.popen(cmd,"r"))
local rows=assert(json.decode(assert(pipe:read("*a"))));assert(pipe:close())
assert(#rows==1 and rows[1].layout==layout and rows[1].input_mode=="Emulated"
  and rows[1].test_mode=="custom" and rows[1].entered==count and rows[1].correct==count,
  "Starter emulation SQLite identity/score incorrect")
local sf=assert(io.open(os.getenv("XDG_CONFIG_HOME").."/typefinch/settings.json","rb"))
local settings=assert(sf:read("*a"));sf:close()
local file=assert(io.open(source,"rb"))
local passage=assert(file:read("*a"));file:close()
assert(not settings:find(source,1,true),"Custom file path leaked to settings")
assert(not settings:find(passage,1,true),"Private custom passage leaked to settings")
print("Native starter ANSI XKB emulation PASS: "..layout.." "..count.."/"..count)
LUA
  stop_app
  unset XKB_CONFIG_ROOT
done
echo 'Native Dvorak and Colemak-DH Shift/edge keys PASS with installed XKB and forced offline fallback'

# Exhaustive native ANSI starter punctuation/Shift score for all four layouts.
# Verify real winit physical-key routing with XKB and the offline table.
# OS physical Right Alt attribution is separately tracked by T303.
full_codes=(49 10 11 12 13 14 15 16 17 18 19 20 21 34 35 48 51 47 59 60 61)
for layout_name in us dvorak colemak colemak-dh; do
  case "$layout_name" in
    us) layout='us' ;;
    dvorak) layout='us(dvorak)' ;;
    colemak) layout='us(colemak)' ;;
    colemak-dh) layout='us(colemak_dh)' ;;
  esac
  for offline in no yes;do
    label="all-ansi-$layout_name-$offline"
    mkdir -p "$tmp/$label"
    # Fixed independent expected symbols for the physical X11 US keycodes.
    lua5.4 - "$layout" "$tmp/$label-source.txt" <<'LUA'
local layout,path=assert(arg[1]),assert(arg[2])
local tick=string.char(96)
local ordinary=tick..[[1234567890-=[]'\;,./]]
local ordinary_shift="~"..[[!@#$%^&*()_+{}"|:<>?]]
local dvorak=tick..[[1234567890[]/=-\swvz]]
local dvorak_shift="~"..[[!@#$%^&*(){}?+_|SWVZ]]
local base,shift=ordinary,ordinary_shift
if layout=="us(dvorak)" then base,shift=dvorak,dvorak_shift end
assert(#base==21 and #shift==21,
  "Fixture must describe all 21 physical punctuation/number positions")
local f=assert(io.open(path,"wb"));assert(f:write(base,shift));assert(f:close())
LUA
    start_app "$label"
    xdotool key --clearmodifiers F11
    sleep 0.3
    cfg="$XDG_CONFIG_HOME/typefinch/settings.json"
    grep -q '"test_mode": "custom"' "$cfg" || {
      echo "Custom mode could not be selected for $label" >&2;exit 1;
    }
    stop_app
    lua5.4 - "$cfg" "$layout" <<'LUA'
local json=require("dkjson")
local path,layout=assert(arg[1]),assert(arg[2])
local f=assert(io.open(path,"rb"))
local prefs=assert(json.decode(assert(f:read("*a"))));f:close()
prefs.layout=layout
local o=assert(io.open(path,"wb"))
assert(o:write(json.encode(prefs,{indent=true})));assert(o:close())
LUA
    if [[ "$offline" == yes ]];then
      mkdir -p "$tmp/empty-xkb-root"
      export XKB_CONFIG_ROOT="$tmp/empty-xkb-root"
    fi
    export TYPEFINCH_QA_DURATION_SECONDS=15
    start_app "$label"
    xdotool key --clearmodifiers F11
    sleep 0.3
    xdotool type --clearmodifiers --delay 10 "$tmp/$label-source.txt"
    xdotool key --clearmodifiers Return
    sleep 0.3
    xdotool key --clearmodifiers F5
    sleep 0.18
    xdotool key --clearmodifiers F2
    sleep 0.18
    # A held Ctrl or ordinary Left Alt is a shortcut, not a printable
    # emulated position. Neither combination may add a scored attempt.
    # Real physical RightAlt/Level-3 detection remains a separate gate.
    xdotool key --clearmodifiers ctrl+b
    xdotool key --clearmodifiers alt+b
    xdotool key --clearmodifiers ctrl+shift+b
    for code in "${full_codes[@]}";do
      xdotool key --clearmodifiers "$code"
    done
    for code in "${full_codes[@]}";do
      xdotool key --clearmodifiers "shift+$code"
    done
    sleep 0.6
    lua5.4 - "$TYPEFINCH_QA_REPORT" "$layout" \
      "$tmp/$label-source.txt" <<'LUA'
local json=require("dkjson")
local report,layout,source=assert(arg[1]),assert(arg[2]),assert(arg[3])
local f=assert(io.open(report,"rb"),"Native full ANSI result missing")
local r=assert(json.decode(assert(f:read("*a"))));f:close()
assert(r.synthetic and r.state=="finished" and r.entered==42 and
  r.correct==42 and r.accuracy==100,
  string.format("Native full ANSI punctuation failed: %s entered=%s correct=%s",
    layout,tostring(r.entered),tostring(r.correct)))
local db=os.getenv("XDG_CONFIG_HOME").."/typefinch/sessions.sqlite3"
local cmd=string.format("sqlite3 -json %q 'SELECT layout,input_mode,test_mode,entered,correct,attempts FROM sessions'",db)
local pipe=assert(io.popen(cmd,"r"))
local rows=assert(json.decode(assert(pipe:read("*a"))));assert(pipe:close())
assert(#rows==1 and rows[1].layout==layout
  and rows[1].input_mode=="Emulated" and rows[1].test_mode=="custom"
  and rows[1].entered==42 and rows[1].correct==42 and rows[1].attempts==42,
  string.format("Full ANSI SQLite mismatch: rows=%d layout=%s expected=%s input=%s test=%s entered=%s correct=%s attempts=%s",
    #rows,tostring(rows[1] and rows[1].layout),layout,
    tostring(rows[1] and rows[1].input_mode),
    tostring(rows[1] and rows[1].test_mode),
    tostring(rows[1] and rows[1].entered),
    tostring(rows[1] and rows[1].correct),
    tostring(rows[1] and rows[1].attempts)))
local file=assert(io.open(source,"rb"));local passage=assert(file:read("*a"));file:close()
local sf=assert(io.open(os.getenv("XDG_CONFIG_HOME").."/typefinch/settings.json","rb"))
local settings=assert(sf:read("*a"));sf:close()
assert(not settings:find(passage,1,true) and not settings:find(source,1,true),
  "Native full ANSI private passage or path persisted")
print("Native complete starter number/edge Shift layers PASS: "..layout..
  " 42/42 in "..(os.getenv("XKB_CONFIG_ROOT") and "offline" or "XKB").." mode")
LUA
    import -window "$window" "typefinch-starter-full-$label.png"
    stop_app
    unset XKB_CONFIG_ROOT TYPEFINCH_QA_DURATION_SECONDS
  done
done
echo 'All four native starter ANSI number/punctuation/Shift layers PASS (XKB + offline fallback)'
