#!/usr/bin/env bash
# T804: Arabic/Hebrew physical-position emulation on real nested wl_keyboard.
# Disposable virtual Xvfb/Weston, never a real keyboard/IME/Hyprland gate.
set -Eeuo pipefail
export LIBGL_ALWAYS_SOFTWARE=1 WINIT_UNIX_BACKEND=wayland XDG_SESSION_TYPE=wayland
export WAYLAND_DISPLAY=typefinch-rtl-wayland
export TYPEFINCH_QA_XKB_COMPOSE=1 TYPEFINCH_QA_DURATION_SECONDS=15
tmp="$(mktemp -d)"
app='' wm='' weston='' parent=''
cleanup(){
  code=$?
  set +e
  if ((code!=0));then
    for log in "$tmp"/*/app.log;do
      [[ ! -s "$log" ]] || tail -n 85 "$log" >&2
    done
    [[ ! -s "$tmp/weston.log" ]] || tail -n 90 "$tmp/weston.log" >&2
  fi
  [[ -z "$app" ]] || { kill "$app" 2>/dev/null;wait "$app" 2>/dev/null; }
  [[ -z "$weston" ]] || { kill "$weston" 2>/dev/null;wait "$weston" 2>/dev/null; }
  [[ -z "$wm" ]] || { kill "$wm" 2>/dev/null;wait "$wm" 2>/dev/null; }
  rm -rf -- "$tmp"
  exit "$code"
}
trap cleanup EXIT
mkdir -m 700 "$tmp/runtime"
export XDG_RUNTIME_DIR="$tmp/runtime"
unset HYPRLAND_INSTANCE_SIGNATURE WAYLAND_SOCKET
setxkbmap -layout us -model pc105
openbox --sm-disable >"$tmp/wm.log" 2>&1 &
wm=$!
# Preserve emulated regression and give OS input its own clean Weston.
# An explicit Weston keyboard config is required; environment defaults
# are not a reliable X11-backend keymap configuration.
native_os_case=""
case "${1:-both}" in ara-os) native_os_case=ara ;; il-os) native_os_case=il ;; mixed-os) native_os_case=mixed ;; esac
weston_options=(--no-config)
if [[ -n "$native_os_case" ]];then
  unset TYPEFINCH_QA_XKB_COMPOSE
  if [[ "$native_os_case" == mixed ]];then
    keymap_groups=us,ara,il
  else
    keymap_groups="us,$native_os_case"
  fi
  printf '[keyboard]\nkeymap_layout=%s\nkeymap_options=grp:alt_shift_toggle\n' \
    "$keymap_groups" >"$tmp/weston.ini"
  weston_options=(--config="$tmp/weston.ini")
fi
weston --backend=x11-backend.so --renderer=pixman --socket="$WAYLAND_DISPLAY" \
  "${weston_options[@]}" --idle-time=0 --width=1024 --height=768 \
  --log="$tmp/weston.log" >"$tmp/weston-out.log" 2>&1 &
weston=$!
for _ in $(seq 1 120);do
  [[ -S "$XDG_RUNTIME_DIR/$WAYLAND_DISPLAY" ]] && break
  kill -0 "$weston" 2>/dev/null || { echo "Nested Weston exited" >&2;exit 1; }
  sleep 0.1
done
[[ -S "$XDG_RUNTIME_DIR/$WAYLAND_DISPLAY" ]] ||
  { echo "Native Wayland socket missing" >&2;exit 1; }
for _ in $(seq 1 100);do
  parent="$(xdotool search --onlyvisible --pid "$weston" 2>/dev/null | head -n1 || true)"
  [[ -n "$parent" ]] || parent="$(xdotool search --onlyvisible --class '[Ww]eston' 2>/dev/null | head -n1 || true)"
  [[ -z "$parent" ]] || break
  sleep 0.1
done
[[ -n "$parent" ]] || { echo "No X11-to-Wayland input bridge" >&2;exit 1; }
start_app(){
  ./target/debug/typefinch >"$tmp/$layout/app.log" 2>&1 &
  app=$!
  sleep 0.75
  kill -0 "$app" 2>/dev/null || { echo "Wayland app exited" >&2;exit 1; }
  xdotool windowactivate --sync "$parent" 2>/dev/null || true
  xdotool windowfocus "$parent" 2>/dev/null || true
  xdotool mousemove --window "$parent" 500 340 click 1
  sleep 0.65
}
stop_app(){
  kill "$app" 2>/dev/null || true
  wait "$app" 2>/dev/null || true
  app=''
  sleep 0.2
}
# The guarded CI runs each script in a fresh Weston/Xvfb compositor.
# Direct standalone invocation without an argument preserves the original
# two-layout developer smoke behavior, but is not the CI acceptance path.
case "${1:-both}" in
  ara) layouts=(ara) ;;
  il) layouts=(il) ;;
  ara-os) layouts=(ara) ;;
  il-os) layouts=(il) ;;
  mixed-os) layouts=(mixed) ;;
  both) layouts=(ara il) ;;
  *) echo 'Unsupported native Wayland RTL fixture layout' >&2;exit 2 ;;
esac
remaining=${#layouts[@]}
for layout in "${layouts[@]}";do
  mkdir -p "$tmp/$layout/config"
  export XDG_CONFIG_HOME="$tmp/$layout/config"
  export TYPEFINCH_QA_REPORT="$tmp/$layout/report.json"
  export TYPEFINCH_QA_IMPORT_REPORT="$tmp/$layout/import.json"
  if [[ "$layout" == mixed ]];then
    expected=mixed passage='سلام abc שלום' keys='' wrong=''
  elif [[ "$layout" == ara ]];then
    expected=arabic passage='سلام' keys='s g h l' wrong='a'
  else
    expected=hebrew passage='שלום' keys='a k u o' wrong='s'
  fi
  printf '%s' "$passage" >"$tmp/$layout/source.txt"
  start_app
  cfg="$XDG_CONFIG_HOME/typefinch/settings.json"
  for _ in $(seq 1 30);do
    [[ -s "$cfg" ]] && grep -q '"test_mode": "custom"' "$cfg" && break
    xdotool windowfocus "$parent" 2>/dev/null || true
    xdotool mousemove --window "$parent" 500 340 click 1
    xdotool key --clearmodifiers F11
    sleep 0.3
  done
  [[ -s "$cfg" ]] && grep -q '"test_mode": "custom"' "$cfg" ||
    { echo "Custom mode unavailable in native Wayland" >&2;exit 1; }
  stop_app
  lua5.4 - "$cfg" "$layout" <<'LUA'
local json=require("dkjson")
local path,layout=assert(arg[1]),assert(arg[2])
local f=assert(io.open(path,"rb"));local p=assert(json.decode(f:read("*a")));f:close()
p.layout=layout=="mixed" and "us" or layout
local out=assert(io.open(path,"wb"))
assert(out:write(json.encode(p,{indent=true})));assert(out:close())
LUA
  start_app
  xdotool key --clearmodifiers F11
  sleep 0.4
  xdotool type --clearmodifiers --delay 22 "$tmp/$layout/source.txt"
  xdotool key --clearmodifiers Return
  sleep 0.6
  lua5.4 - "$TYPEFINCH_QA_IMPORT_REPORT" "$expected" "$layout" <<'LUA'
local json=require("dkjson")
local f=assert(io.open(assert(arg[1]),"rb"),"Native Wayland RTL import missing")
local p=assert(json.decode(f:read("*a")));f:close()
assert(p.state=="custom-import" and p.synthetic and p.script==arg[2]
  and p.graphemes==(arg[3]=="mixed" and 13 or 4),
  "Native Wayland imported the wrong RTL reference")
LUA
  if [[ "$native_os_case" == mixed ]];then
    # One native Wayland scored Custom session spans three XKB groups.
    # Both Xvfb's parent layout and Weston's keyboard group are changed.
    # No emulator, injected IME commit, physical device or global capture.
    xdotool windowfocus "$parent" 2>/dev/null || true
    xdotool key --clearmodifiers F5
    sleep 0.25
    xdotool key --clearmodifiers F2
    sleep 0.25
    setxkbmap -layout ara
    xdotool key --clearmodifiers Alt_L+Shift_L
    sleep 0.20
    xdotool key --clearmodifiers 38
    sleep 0.15
    [[ ! -s "$TYPEFINCH_QA_REPORT" ]] ||
      { echo "Mixed Wayland RTL wrong key completed a session" >&2;exit 1; }
    xdotool key --clearmodifiers BackSpace
    sleep 0.12
    for code in 39 42 43 46;do
      xdotool key --clearmodifiers "$code"
      sleep 0.08
    done
    xdotool key --clearmodifiers 65
    sleep 0.10
    [[ ! -s "$TYPEFINCH_QA_REPORT" ]] ||
      { echo "Arabic segment prematurely finished mixed RTL passage" >&2;exit 1; }
    # ara -> il -> us. Keep the same focused child Wayland session.
    setxkbmap -layout us
    xdotool key --clearmodifiers Alt_L+Shift_L
    xdotool key --clearmodifiers Alt_L+Shift_L
    sleep 0.20
    for code in 38 56 54;do
      xdotool key --clearmodifiers "$code"
      sleep 0.08
    done
    xdotool key --clearmodifiers 65
    sleep 0.10
    [[ ! -s "$TYPEFINCH_QA_REPORT" ]] ||
      { echo "Latin segment prematurely finished mixed RTL passage" >&2;exit 1; }
    # us -> ara -> il. Third script, same native Wayland surface.
    setxkbmap -layout il
    xdotool key --clearmodifiers Alt_L+Shift_L
    xdotool key --clearmodifiers Alt_L+Shift_L
    sleep 0.20
    for code in 38 45 30 32;do
      xdotool key --clearmodifiers "$code"
      sleep 0.08
    done
    sleep 0.80
    lua5.4 - "$TYPEFINCH_QA_REPORT" "$tmp/$layout/source.txt" <<'LUA'
local json=require("dkjson")
local f=assert(io.open(arg[1],"rb"),"Missing mixed RTL native Wayland OS score")
local r=assert(json.decode(f:read("*a")));f:close()
assert(r.synthetic and r.state=="finished" and r.entered==13
  and r.correct==13 and r.accuracy==100,
  string.format("Native mixed RTL Wayland OS scoring mismatch: %s/%s state=%s",
    tostring(r.correct),tostring(r.entered),tostring(r.state)))
local path=os.getenv("XDG_CONFIG_HOME").."/typefinch/sessions.sqlite3"
local pipe=assert(io.popen(string.format(
  "sqlite3 -json %q 'SELECT input_mode,test_mode,entered,correct,attempts,mistakes,corrections FROM sessions'",path),"r"))
local rows=assert(json.decode(pipe:read("*a")));assert(pipe:close())
assert(#rows==1 and rows[1].input_mode=="OS" and rows[1].test_mode=="custom"
  and rows[1].entered==13 and rows[1].correct==13
  and rows[1].attempts==14 and rows[1].mistakes==1
  and rows[1].corrections==1,
  "Mixed RTL Wayland OS SQLite score provenance or mistake invalid")
local cf=assert(io.open(os.getenv("XDG_CONFIG_HOME").."/typefinch/settings.json","rb"))
local prefs=cf:read("*a");cf:close()
local sf=assert(io.open(arg[2],"rb"));local passage=sf:read("*a");sf:close()
assert(not prefs:find(passage,1,true) and not prefs:find(arg[2],1,true),
  "Mixed RTL source passage or import path persisted into preferences")
print("Native Weston RTL mixed OS-input PASS: Arabic -> Latin -> Hebrew, 13/13 after corrected mistake")
LUA
    kill -0 "$weston" 2>/dev/null ||
      { echo "Nested Weston exited after mixed RTL Wayland score" >&2;exit 1; }
    continue
  fi
  if [[ -n "$native_os_case" ]];then
    # X11-backed Weston may inherit the parent XKB mapping.
    # Change only this disposable Xvfb display; emit raw numeric positions.
    if [[ "$layout" == ara ]];then
      physical_codes=(39 42 43 46) wrong_code=38
    else
      physical_codes=(38 45 30 32) wrong_code=39
    fi
    setxkbmap -layout "$layout"
    setxkbmap -query | grep -Eq "^layout:[[:space:]]+$layout$" ||
      { echo "Disposable parent XKB layout did not change" >&2;exit 1; }
    sleep 0.4
    xdotool key --clearmodifiers Alt_L+Shift_L
    sleep 0.35
    xdotool key --clearmodifiers F5
    sleep 0.30
    xdotool key --clearmodifiers F2
    sleep 0.25
    xdotool key --clearmodifiers "$wrong_code"
    sleep 0.16
    [[ ! -s "$TYPEFINCH_QA_REPORT" ]] ||
      { echo "Wrong Wayland OS RTL key unexpectedly completed a session" >&2;exit 1; }
    xdotool key --clearmodifiers BackSpace
    sleep 0.15
    for key in "${physical_codes[@]}";do
      xdotool key --clearmodifiers "$key"
      sleep 0.12
    done
    sleep 0.8
    lua5.4 - "$TYPEFINCH_QA_REPORT" "$layout" "$tmp/$layout/source.txt" <<'LUA'
local json=require("dkjson")
local f=assert(io.open(arg[1],"rb"),"Native RTL Wayland OS score missing")
local r=assert(json.decode(f:read("*a")));f:close()
assert(r.synthetic and r.state=="finished" and r.entered==4
  and r.correct==4 and r.accuracy==100,
  string.format("Native Wayland OS RTL scoring failed: entered=%s correct=%s accuracy=%s state=%s",
    tostring(r.entered),tostring(r.correct),tostring(r.accuracy),tostring(r.state)))
local db=os.getenv("XDG_CONFIG_HOME").."/typefinch/sessions.sqlite3"
local cmd=string.format(
  "sqlite3 -json %q 'SELECT input_mode,test_mode,entered,correct,attempts,mistakes,corrections FROM sessions'",db)
local pipe=assert(io.popen(cmd,"r"))
local rows=assert(json.decode(pipe:read("*a")));assert(pipe:close())
assert(#rows==1 and rows[1].input_mode=="OS" and rows[1].test_mode=="custom"
  and rows[1].entered==4 and rows[1].correct==4 and rows[1].attempts==5
  and rows[1].mistakes==1 and rows[1].corrections==1,
  "Native RTL Wayland OS-input SQLite provenance or correction mismatch")
local cf=assert(io.open(os.getenv("XDG_CONFIG_HOME").."/typefinch/settings.json","rb"))
local prefs=cf:read("*a");cf:close()
local sf=assert(io.open(arg[3],"rb"));local passage=sf:read("*a");sf:close()
assert(not prefs:find(passage,1,true) and not prefs:find(arg[3],1,true),
  "RTL OS-input passage or source path persisted in preferences")
print("Native Weston RTL OS-input XKB group PASS: "..arg[2]..
  ", 4/4 with corrected error; no IME or physical keyboard claims")
LUA
    kill -0 "$weston" 2>/dev/null ||
      { echo "Native Weston exited after RTL OS scoring" >&2;exit 1; }
    continue
  fi
  xdotool windowfocus "$parent" 2>/dev/null || true
  xdotool key --clearmodifiers F5
  sleep 0.3
  xdotool key --clearmodifiers F2
  sleep 0.3
  # Prove correction on the actual child Wayland wl_keyboard path, not
  # just a clean four-key sequence. The wrong physical position must
  # remain a raw mistake after BackSpace and the completed correct word.
  xdotool key --clearmodifiers "$wrong"
  sleep 0.18
  [[ ! -s "$TYPEFINCH_QA_REPORT" ]] ||
    { echo "Wrong native Wayland RTL key completed the session" >&2;exit 1; }
  xdotool key --clearmodifiers BackSpace
  sleep 0.18
  for key in $keys;do
    xdotool key --clearmodifiers "$key"
    sleep 0.10
  done
  sleep 0.7
  lua5.4 - "$TYPEFINCH_QA_REPORT" "$layout" "$tmp/$layout/source.txt" <<'LUA'
local json=require("dkjson")
local report,layout,source=assert(arg[1]),assert(arg[2]),assert(arg[3])
local f=assert(io.open(report,"rb"),"No native Wayland RTL score")
local r=assert(json.decode(f:read("*a")));f:close()
assert(r.synthetic and r.state=="finished" and r.entered==4 and r.correct==4
  and r.accuracy==100,"Native Wayland RTL physical-position scoring failed")
local db=os.getenv("XDG_CONFIG_HOME").."/typefinch/sessions.sqlite3"
local pipe=assert(io.popen(string.format(
  "sqlite3 -json %q 'SELECT layout,input_mode,test_mode,entered,correct,attempts,mistakes,corrections FROM sessions'",db),"r"))
local rows=assert(json.decode(pipe:read("*a")));assert(pipe:close())
assert(#rows==1 and rows[1].layout==layout and rows[1].input_mode=="Emulated"
  and rows[1].test_mode=="custom" and rows[1].entered==4 and rows[1].correct==4
  and rows[1].attempts==5 and rows[1].mistakes==1 and rows[1].corrections==1,
  "Native Wayland RTL correction or score provenance mismatch")
local cf=assert(io.open(os.getenv("XDG_CONFIG_HOME").."/typefinch/settings.json","rb"))
local cfg=cf:read("*a");cf:close()
local sf=assert(io.open(source,"rb"));local target=sf:read("*a");sf:close()
assert(not cfg:find(target,1,true) and not cfg:find(source,1,true),
  "RTL reference or its path leaked into persistent preferences")
print("Native Weston wl_keyboard RTL Emulated PASS: "..layout..", 4/4 after one corrected mistake")
LUA
  # A second session on the same native Wayland surface must score an
  # actual hard newline through Return, not only four single-line letters.
  if [[ "$layout" == ara ]];then
    printf 'سل\nام' >"$tmp/$layout/multiline.txt"
    multiline_keys=(s g Return h l)
  else
    printf 'של\nום' >"$tmp/$layout/multiline.txt"
    multiline_keys=(a k Return u o)
  fi
  xdotool windowfocus "$parent" 2>/dev/null || true
  xdotool key --clearmodifiers F11
  sleep 0.35
  # F11 focuses the existing path field; select its previous filename
  # rather than appending the new import path to it.
  xdotool key --clearmodifiers ctrl+a
  sleep 0.1
  xdotool type --clearmodifiers --delay 22 "$tmp/$layout/multiline.txt"
  xdotool key --clearmodifiers Return
  sleep 0.6
  lua5.4 - "$TYPEFINCH_QA_IMPORT_REPORT" "$expected" <<'LUA'
local json=require("dkjson")
local f=assert(io.open(arg[1],"rb"),"Native RTL multiline import missing")
local p=assert(json.decode(f:read("*a")));f:close()
assert(p.synthetic and p.state=="custom-import" and p.script==arg[2]
  and p.graphemes==5,"RTL multiline import did not retain hard line break")
LUA
  rm -f -- "$TYPEFINCH_QA_REPORT"
  xdotool key --clearmodifiers F5
  sleep 0.3
  xdotool key --clearmodifiers F2
  sleep 0.3
  for key in "${multiline_keys[@]}";do
    xdotool key --clearmodifiers "$key"
    sleep 0.12
  done
  sleep 0.75
  lua5.4 - "$TYPEFINCH_QA_REPORT" "$layout" "$tmp/$layout/multiline.txt" <<'LUA'
local json=require("dkjson")
local report,layout,source=assert(arg[1]),assert(arg[2]),assert(arg[3])
local f=assert(io.open(report,"rb"),"No multiline Wayland RTL score")
local r=assert(json.decode(f:read("*a")));f:close()
assert(r.synthetic and r.state=="finished" and r.entered==5
  and r.correct==5 and r.accuracy==100,
  "Native Wayland RTL newline scoring failed")
local db=os.getenv("XDG_CONFIG_HOME").."/typefinch/sessions.sqlite3"
local p=assert(io.popen(string.format(
  "sqlite3 -json %q 'SELECT layout,input_mode,test_mode,entered,correct,attempts,mistakes,corrections FROM sessions ORDER BY id'",db),"r"))
local rows=assert(json.decode(p:read("*a")));assert(p:close())
assert(#rows==2 and rows[1].entered==4 and rows[1].correct==4
  and rows[1].mistakes==1 and rows[1].corrections==1,
  "Prior corrected RTL score changed after second import")
assert(rows[2].layout==layout and rows[2].input_mode=="Emulated"
  and rows[2].test_mode=="custom" and rows[2].entered==5
  and rows[2].correct==5 and rows[2].attempts==5
  and rows[2].mistakes==0 and rows[2].corrections==0,
  "Multiline RTL score provenance mismatch")
local pf=assert(io.open(os.getenv("XDG_CONFIG_HOME").."/typefinch/settings.json","rb"))
local prefs=pf:read("*a");pf:close()
local sf=assert(io.open(source,"rb"));local content=sf:read("*a");sf:close()
assert(not prefs:find(content,1,true) and not prefs:find(source,1,true),
  "RTL multiline source or local path leaked into persistent preferences")
print("Native RTL Wayland multiline Enter PASS: "..layout..", 5/5; two isolated scores")
LUA
  # Keep the final scored child surface alive through the acceptance
  # verdict. Weston 13 can SIGSEGV during rapid child-client teardown;
  # cleanup owns the orderly final app/compositor shutdown. Only a
  # standalone two-layout developer smoke needs a restart between cases.
  remaining=$((remaining-1))
  if ((remaining>0));then stop_app;fi
  # A crash while a scored session is still expected to be live is never
  # classified as a retriable pre-score bootstrap failure.
  kill -0 "$weston" 2>/dev/null ||
    { echo "Nested Weston exited after native RTL scoring" >&2;exit 1; }
done
if [[ -n "$native_os_case" ]];then
  echo "Native nested Wayland ${1} OS-input PASS; no IME or physical hardware claims"
elif [[ "${1:-both}" == both ]];then
  echo "Native nested Wayland Arabic/Hebrew emulation PASS; no IME or hardware claims"
else
  echo "Native nested Wayland ${1} emulation PASS; no IME or hardware claims"
fi
