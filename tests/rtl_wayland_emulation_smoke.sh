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
weston --backend=x11-backend.so --renderer=pixman --socket="$WAYLAND_DISPLAY" \
  --no-config --idle-time=0 --width=1024 --height=768 \
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
for layout in ara il;do
  mkdir -p "$tmp/$layout/config"
  export XDG_CONFIG_HOME="$tmp/$layout/config"
  export TYPEFINCH_QA_REPORT="$tmp/$layout/report.json"
  export TYPEFINCH_QA_IMPORT_REPORT="$tmp/$layout/import.json"
  if [[ "$layout" == ara ]];then
    expected=arabic passage='سلام' keys='s g h l'
  else
    expected=hebrew passage='שלום' keys='a k u o'
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
p.layout=layout
local out=assert(io.open(path,"wb"))
assert(out:write(json.encode(p,{indent=true})));assert(out:close())
LUA
  start_app
  xdotool key --clearmodifiers F11
  sleep 0.4
  xdotool type --clearmodifiers --delay 22 "$tmp/$layout/source.txt"
  xdotool key --clearmodifiers Return
  sleep 0.6
  lua5.4 - "$TYPEFINCH_QA_IMPORT_REPORT" "$expected" <<'LUA'
local json=require("dkjson")
local f=assert(io.open(assert(arg[1]),"rb"),"Native Wayland RTL import missing")
local p=assert(json.decode(f:read("*a")));f:close()
assert(p.state=="custom-import" and p.synthetic and p.script==arg[2] and p.graphemes==4,
  "Native Wayland imported the wrong RTL reference")
LUA
  xdotool windowfocus "$parent" 2>/dev/null || true
  xdotool key --clearmodifiers F5
  sleep 0.3
  xdotool key --clearmodifiers F2
  sleep 0.3
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
  "sqlite3 -json %q 'SELECT layout,input_mode,test_mode,entered,correct,attempts FROM sessions'",db),"r"))
local rows=assert(json.decode(pipe:read("*a")));assert(pipe:close())
assert(#rows==1 and rows[1].layout==layout and rows[1].input_mode=="Emulated"
  and rows[1].test_mode=="custom" and rows[1].entered==4 and rows[1].correct==4
  and rows[1].attempts==4,"Native Wayland RTL score provenance mismatch")
local cf=assert(io.open(os.getenv("XDG_CONFIG_HOME").."/typefinch/settings.json","rb"))
local cfg=cf:read("*a");cf:close()
local sf=assert(io.open(source,"rb"));local target=sf:read("*a");sf:close()
assert(not cfg:find(target,1,true) and not cfg:find(source,1,true),
  "RTL reference or its path leaked into persistent preferences")
print("Native Weston wl_keyboard RTL Emulated PASS: "..layout..", 4/4")
LUA
  stop_app
done
echo "Native nested Wayland Arabic/Hebrew emulation PASS; no IME or hardware claims"
