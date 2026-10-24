#!/usr/bin/env bash
# Genuine nested Wayland GUI typing: Xvfb virtual hardware -> Weston X11
# backend -> Wayland wl_keyboard -> the production Typefinch egui surface.
# No physical hardware, Hyprland, privileges or global key interception.
set -Eeuo pipefail
: "${DISPLAY:?A disposable Xvfb DISPLAY is required}"
tmp="$(mktemp -d)"
weston_pid="" app_pid="" wm_pid=""
cleanup(){
  status=$?
  if ((status!=0));then
    for log in "$tmp/app.log" "$tmp/weston.log" "$tmp/weston.stdout";do
      [[ ! -s "$log" ]] || tail -n 75 "$log" >&2 || true
    done
  fi
  [[ -z "$app_pid" ]] || { kill "$app_pid" 2>/dev/null || true; wait "$app_pid" 2>/dev/null || true; }
  [[ -z "$weston_pid" ]] || { kill "$weston_pid" 2>/dev/null || true; wait "$weston_pid" 2>/dev/null || true; }
  [[ -z "$wm_pid" ]] || { kill "$wm_pid" 2>/dev/null || true; wait "$wm_pid" 2>/dev/null || true; }
  rm -rf "$tmp"
}
trap cleanup EXIT
mkdir -m 700 "$tmp/runtime"
mkdir -p "$tmp/config"
export XDG_RUNTIME_DIR="$tmp/runtime"
export XDG_CONFIG_HOME="$tmp/config"
export WAYLAND_DISPLAY=typefinch-wayland-input
export XDG_SESSION_TYPE=wayland
export WINIT_UNIX_BACKEND=wayland
export LIBGL_ALWAYS_SOFTWARE=1
export TYPEFINCH_QA_REPORT="$tmp/score.json"
unset HYPRLAND_INSTANCE_SIGNATURE WAYLAND_SOCKET
printf '%s' 'abc' > "$tmp/source.txt"
openbox --sm-disable >"$tmp/wm.log" 2>&1 &
wm_pid=$!
weston --backend=x11-backend.so --renderer=pixman --socket="$WAYLAND_DISPLAY" \
  --no-config --idle-time=0 --width=1024 --height=768 --log="$tmp/weston.log" \
  >"$tmp/weston.stdout" 2>&1 &
weston_pid=$!
for _ in $(seq 1 120);do
  [[ -S "$XDG_RUNTIME_DIR/$WAYLAND_DISPLAY" ]] && break
  kill -0 "$weston_pid" 2>/dev/null || {
    echo "Nested Weston exited before Wayland socket" >&2;exit 1;
  }
  sleep 0.1
done
[[ -S "$XDG_RUNTIME_DIR/$WAYLAND_DISPLAY" ]] || {
  echo "Nested Weston Wayland socket not ready" >&2;exit 1;
}
parent=""
for _ in $(seq 1 100);do
  # Weston/X11 has differing title/class conventions across distributions.
  # Match its own PID first so Openbox, the Xvfb root, or another window
  # can never be mistaken for the compositor input bridge.
  parent="$(xdotool search --onlyvisible --pid "$weston_pid" 2>/dev/null|head -n 1||true)"
  if [[ -z "$parent" ]];then
    parent="$(xdotool search --onlyvisible --class '[Ww]eston' 2>/dev/null|head -n 1||true)"
  fi
  if [[ -z "$parent" ]];then
    parent="$(xdotool search --onlyvisible --name '[Ww]eston' 2>/dev/null|head -n 1||true)"
  fi
  [[ -n "$parent" ]] && break
  sleep 0.1
done
if [[ -z "$parent" ]];then
  echo "No Weston parent X11 window by PID/class/title" >&2
  for id in $(xdotool search --onlyvisible --name '.' 2>/dev/null||true);do
    printf 'Synthetic X11 window %s title=' "$id" >&2
    xdotool getwindowname "$id" >&2 || true
    printf ' class=' >&2
    xdotool getwindowclassname "$id" >&2 || true
  done
  exit 1
fi
./target/debug/typefinch >"$tmp/app.log" 2>&1 &
app_pid=$!
cfg="$XDG_CONFIG_HOME/typefinch/settings.json"
# The Wayland client can become live after Weston's X11 bridge is already
# focusable. A fixed sleep makes F11 a timing lottery on fast CI runners.
# Require the app's persisted response before injecting any private text.
custom_selected=0
for _ in $(seq 1 30);do
  kill -0 "$app_pid" 2>/dev/null || {
    echo "Typefinch exited before native Wayland keyboard input" >&2;exit 1;
  }
  xdotool windowactivate --sync "$parent" 2>/dev/null || true
  xdotool windowfocus "$parent" 2>/dev/null || true
  # Focus the *inner* Wayland surface, not merely the outer X11 window.
  xdotool mousemove --window "$parent" 500 340 click 1
  xdotool key --clearmodifiers F11
  sleep 0.3
  if [[ -f "$cfg" ]] && grep -q '"test_mode": "custom"' "$cfg";then
    custom_selected=1
    break
  fi
done
if ((custom_selected!=1));then
  # The image contains only a disposable synthetic fixture. Retain it for
  # failed CI diagnosis; never include a real profile or passage.
  import -window "$parent" typefinch-wayland-virtual.png 2>/dev/null || true
  echo "Wayland F11 did not select Custom after 30 focused attempts" >&2
  exit 1
fi
xdotool type --clearmodifiers --delay 25 "$tmp/source.txt"
xdotool key --clearmodifiers Return
sleep 0.55
xdotool key --clearmodifiers F5
sleep 0.3
xdotool key --clearmodifiers F2
sleep 0.2
xdotool type --clearmodifiers --delay 40 abc
sleep 0.8
lua5.4 - "$TYPEFINCH_QA_REPORT" "$tmp/source.txt" "$cfg" <<'LUA'
local json=require("dkjson")
local report,source,settings=assert(arg[1]),assert(arg[2]),assert(arg[3])
local f=assert(io.open(report,"rb"),"No scored native Wayland typing result")
local r=assert(json.decode(assert(f:read("*a"))));f:close()
assert(r.synthetic and r.state=="finished" and r.entered==3
  and r.correct==3 and r.accuracy==100,
  "Native Weston wl_keyboard did not deliver the complete test")
local db=os.getenv("XDG_CONFIG_HOME").."/typefinch/sessions.sqlite3"
local cmd=string.format("sqlite3 -json %q 'SELECT test_mode,input_mode,layout,entered,correct FROM sessions'",db)
local p=assert(io.popen(cmd,"r"))
local rows=assert(json.decode(assert(p:read("*a"))));assert(p:close())
assert(#rows==1 and rows[1].test_mode=="custom" and rows[1].input_mode=="OS"
  and rows[1].layout=="os:unknown" and rows[1].entered==3 and rows[1].correct==3,
  "Native Wayland provenance or score must remain aggregate/unattributed")
local cf=assert(io.open(settings,"rb"))
local body=assert(cf:read("*a"));cf:close()
local sf=assert(io.open(source,"rb"))
local passage=assert(sf:read("*a"));sf:close()
assert(not body:find(source,1,true),"Local source path persisted")
assert(not body:find(passage,1,true),"Private source text persisted")
print("Nested Weston real Wayland wl_keyboard PASS: 3/3 OS-input graphemes, unattributed seat, no text persistence")
LUA
import -window "$parent" typefinch-wayland-virtual.png
echo "Nested Weston production Wayland GUI typed and saved one clean Custom session"
