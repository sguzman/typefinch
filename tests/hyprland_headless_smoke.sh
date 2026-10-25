#!/usr/bin/env bash
# Genuine headless Hyprland compositor, synthetic output only; this neither
# reads host input devices nor proves connected hardware/Hyprland hotplug.
set -euo pipefail
tmp="$(mktemp -d)"
compositor=''
parent=''
app=''
cleanup(){
  local code=$?
  set +e
  [[ -z "$app" ]] || { kill "$app" 2>/dev/null; wait "$app" 2>/dev/null; }
  [[ -z "$compositor" ]] || { kill "$compositor" 2>/dev/null; wait "$compositor" 2>/dev/null; }
  [[ -z "$parent" ]] || { kill "$parent" 2>/dev/null; wait "$parent" 2>/dev/null; }
  if ((code!=0));then
    echo "Headless Hyprland failed" >&2
    for log in "$tmp"/cache/hyprland/hyprlandCrashReport*.txt;do
      [[ ! -f "$log" ]] || tail -n 180 "$log" >&2
    done
    [[ ! -f "$tmp/hyprland.log" ]] || tail -n 100 "$tmp/hyprland.log" >&2
    [[ ! -f "$tmp/weston.log" ]] || tail -n 50 "$tmp/weston.log" >&2
    for log in "$XDG_RUNTIME_DIR"/hypr/*/hyprland.log;do
      [[ ! -f "$log" ]] || tail -n 110 "$log" >&2
    done
    [[ ! -f "$tmp/typefinch.log" ]] || tail -n 50 "$tmp/typefinch.log" >&2
  fi
  rm -rf "$tmp" || true
  exit "$code"
}
trap cleanup EXIT
mkdir -m 700 "$tmp/runtime" "$tmp/config" "$tmp/data" "$tmp/cache"
export XDG_RUNTIME_DIR="$tmp/runtime" XDG_CONFIG_HOME="$tmp/config"
export XDG_DATA_HOME="$tmp/data" XDG_CACHE_HOME="$tmp/cache"
export XDG_SESSION_TYPE=wayland XDG_CURRENT_DESKTOP=Hyprland
export LIBGL_ALWAYS_SOFTWARE=1 MESA_LOADER_DRIVER_OVERRIDE=llvmpipe
export HYPRLAND_NO_SD_NOTIFY=1 HYPRLAND_TRACE=1 AQ_TRACE=1
unset DISPLAY WAYLAND_SOCKET WAYLAND_DISPLAY HYPRLAND_INSTANCE_SIGNATURE
# Current Aquamarine cannot create a GPU-less stand-alone backend. The
# disposable software-rendered parent Weston supplies a legitimate Wayland
# allocator/surface; Hyprland itself still owns the real child compositor.
# Aquamarine's nested Wayland backend needs a dmabuf-capable EGL parent;
# the Pixman-only Weston parent lacks the required allocation protocols.
# Exercise Weston software GLES via llvmpipe instead of pretending a
# Pixman-only Wayland socket is an acceptable Hyprland host.
weston --backend=headless-backend.so --renderer=gl --socket=parent-wayland \
  --no-config --idle-time=0 --width=1280 --height=900 \
  --log="$tmp/weston.log" >"$tmp/weston-stdout.log" 2>&1 &
parent=$!
for _ in $(seq 1 100);do
  [[ -S "$XDG_RUNTIME_DIR/parent-wayland" ]] && break
  kill -0 "$parent" 2>/dev/null || { echo "Parent Weston exited" >&2;exit 1; }
  sleep 0.2
done
[[ -S "$XDG_RUNTIME_DIR/parent-wayland" ]] || { echo "Parent Weston socket missing" >&2;exit 1; }
export WAYLAND_DISPLAY=parent-wayland
cat >"$tmp/hyprland.conf" <<'CONF'
monitor = ,1280x900@60,auto,1
misc {
    disable_hyprland_logo = true
    disable_splash_rendering = true
}
general {
    gaps_in = 0
    gaps_out = 0
    border_size = 1
}
CONF
Hyprland --config "$tmp/hyprland.conf" >"$tmp/hyprland.log" 2>&1 &
compositor=$!
instance=''
for _ in $(seq 1 100);do
  kill -0 "$compositor" 2>/dev/null || { echo "Hyprland exited on startup" >&2;exit 1; }
  for f in "$XDG_RUNTIME_DIR"/hypr/*/.socket.sock;do
    if [[ -S "$f" ]];then instance="$(basename "$(dirname "$f")")";break;fi
  done
  [[ -z "$instance" ]] || break
  sleep 0.2
done
[[ -n "$instance" ]] || { echo "Hyprland IPC missing" >&2;exit 1; }
export HYPRLAND_INSTANCE_SIGNATURE="$instance"
hyprctl -j version >"$tmp/version.json"
jq -e 'type=="object" and (.branch!=null or .tag!=null or .commit!=null)' "$tmp/version.json" >/dev/null
hyprctl -j monitors >"$tmp/monitors.json"
if [[ "$(jq 'length' "$tmp/monitors.json")" == 0 ]];then
  hyprctl output create headless TypefinchCI
  sleep 0.8
  hyprctl -j monitors >"$tmp/monitors.json"
fi
jq -e 'type=="array" and length>0' "$tmp/monitors.json" >/dev/null
for socket in "$XDG_RUNTIME_DIR"/wayland-*;do
  if [[ -S "$socket" ]];then export WAYLAND_DISPLAY="$(basename "$socket")";break;fi
done
: "${WAYLAND_DISPLAY:?Hyprland native Wayland socket missing}"
test -x /workspace/target/debug/typefinch
env -u DISPLAY ./target/debug/typefinch >"$tmp/typefinch.log" 2>&1 &
app=$!
found=0
for _ in $(seq 1 100);do
  kill -0 "$app" 2>/dev/null || { echo "Typefinch crashed" >&2;exit 1; }
  hyprctl -j clients >"$tmp/clients.json" || true
  if jq -e 'any(.[]; .title=="Typefinch")' "$tmp/clients.json" >/dev/null 2>&1;then
    found=1;break
  fi
  sleep 0.2
done
((found==1)) || { echo "Typefinch Wayland client not registered" >&2;exit 1; }
jq -n --slurpfile version "$tmp/version.json" \
  --slurpfile monitors "$tmp/monitors.json" \
  --slurpfile clients "$tmp/clients.json" \
  '{compositor:"Hyprland",virtual:true,physical_keyboard:false,version:$version[0],monitors:$monitors[0],clients:($clients[0]|map({title,class,workspace}))}' >typefinch-hyprland-inspection.json
# A rendered window is not evidence that Hyprland delivers keyboard events.
# Route a synthetic F1 through Hyprland's native sendshortcut dispatcher to the
# actual Typefinch Wayland window; never access /dev/input or global key streams.
hyprctl dispatch focuswindow "pid:$app" >"$tmp/focus.log"
for _ in $(seq 1 30);do
  active_pid="$(hyprctl -j activewindow | jq -r '.pid // empty')"
  [[ "$active_pid" == "$app" ]] && break
  sleep 0.2
done
[[ "${active_pid:-}" == "$app" ]] || {
  echo "Typefinch did not become the active native Hyprland window" >&2
  exit 1
}
sleep 0.6
grim "$tmp/before-shortcut.png"
[[ -s "$tmp/before-shortcut.png" ]] || {
  echo "Hyprland baseline screenshot missing" >&2; exit 1;
}
# F1 toggles Typefinch's built-in keyboard help panel in the real GUI.
hyprctl dispatch sendshortcut ", F1, pid:$app" >"$tmp/shortcut.log"
sleep 0.7
grim typefinch-hyprland-headless.png
file typefinch-hyprland-headless.png
[[ -s typefinch-hyprland-headless.png ]]
# A changed screenshot alone can be a blinking caret. Demand a substantial
# pixel difference consistent with the entire F1 help panel being rendered.
pixels="$(compare -metric AE "$tmp/before-shortcut.png"   typefinch-hyprland-headless.png null: 2>&1 || true)"
[[ "$pixels" =~ ^[0-9]+$ && "$pixels" -gt 2000 ]] || {
  echo "Native Hyprland F1 did not visibly open Typefinch help: pixels=$pixels" >&2
  exit 1
}
echo "Native Hyprland Wayland GUI and F1 keyboard shortcut PASS ($pixels differing pixels)"
echo "Virtual compositor only; physical keyboard, IME and hotplug remain unaccepted"
