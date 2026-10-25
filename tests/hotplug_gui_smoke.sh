#!/usr/bin/env bash
# Native Wayland Typefinch GUI, deliberately simulated Hyprland metadata.
# Never connects to real Hyprland, evdev, physical keyboard or user workspace.
set -Eeuo pipefail
: "${DISPLAY:?disposable Xvfb required}"
tmp="$(mktemp -d)"
wm_pid="" weston_pid="" app_pid=""
cleanup(){
  local status=$?
  if ((status!=0));then
    for log in "$tmp/app.log" "$tmp/weston.log";do
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
mkdir -p "$tmp/config" "$tmp/bin"
cat >"$tmp/bin/hyprctl" <<'SH'
#!/bin/sh
[ "${1:-}" = "-j" ] && [ "${2:-}" = devices ] || exit 2
exec cat "${TYPEFINCH_QA_HOTPLUG_FIXTURE:?}"
SH
chmod 700 "$tmp/bin/hyprctl"
export PATH="$tmp/bin:$PATH"
export XDG_RUNTIME_DIR="$tmp/runtime"
export XDG_CONFIG_HOME="$tmp/config"
export WAYLAND_DISPLAY=typefinch-hotplug-wayland
export XDG_SESSION_TYPE=wayland
export WINIT_UNIX_BACKEND=wayland
export LIBGL_ALWAYS_SOFTWARE=1
export HYPRLAND_INSTANCE_SIGNATURE=typefinch-qa-synthetic-only
export TYPEFINCH_QA_REPORT="$tmp/score.json"
export TYPEFINCH_QA_HOTPLUG_REPORT="$tmp/hotplug.json"
export TYPEFINCH_QA_HOTPLUG_FIXTURE="$tmp/devices.json"
printf '%s\n' '{"keyboards":[{"name":"Fixture A","address":"0x1","active_keymap":"English (US)"}]}' \
  >"$TYPEFINCH_QA_HOTPLUG_FIXTURE"
openbox --sm-disable >"$tmp/openbox.log" 2>&1 &
wm_pid=$!
weston --backend=x11-backend.so --renderer=pixman \
  --socket="$WAYLAND_DISPLAY" --no-config --idle-time=0 \
  --width=1024 --height=768 --log="$tmp/weston.log" \
  >"$tmp/weston.stdout" 2>&1 &
weston_pid=$!
for _ in $(seq 1 120);do
  [[ -S "$XDG_RUNTIME_DIR/$WAYLAND_DISPLAY" ]] && break
  kill -0 "$weston_pid" 2>/dev/null || {
    echo "Disposable Weston terminated prematurely" >&2;exit 1;
  }
  sleep 0.1
done
[[ -S "$XDG_RUNTIME_DIR/$WAYLAND_DISPLAY" ]] || {
  echo "Native Wayland socket unavailable" >&2;exit 1;
}
parent=""
for _ in $(seq 1 100);do
  parent="$(xdotool search --onlyvisible --pid "$weston_pid" 2>/dev/null|head -n 1||true)"
  [[ -n "$parent" ]] || parent="$(xdotool search --onlyvisible --class '[Ww]eston' 2>/dev/null|head -n 1||true)"
  [[ -n "$parent" ]] && break
  sleep 0.1
done
[[ -n "$parent" ]] || { echo "Weston X11 bridge missing" >&2;exit 1; }
./target/debug/typefinch >"$tmp/app.log" 2>&1 &
app_pid=$!
cfg="$XDG_CONFIG_HOME/typefinch/settings.json"
ready=0
for _ in $(seq 1 40);do
  kill -0 "$app_pid" 2>/dev/null || { echo "Native Typefinch quit" >&2;exit 1; }
  xdotool windowactivate --sync "$parent" 2>/dev/null || true
  xdotool windowfocus "$parent" 2>/dev/null || true
  xdotool mousemove --window "$parent" 500 340 click 1
  xdotool key --clearmodifiers F11
  sleep 0.2
  if [[ -s "$cfg" ]] && grep -q '"test_mode": "custom"' "$cfg";then
    ready=1;break
  fi
done
((ready==1)) || { echo "Native Wayland Typefinch did not accept F11" >&2;exit 1; }
# Observe compositor inspector's actual egui event loop; no names,
# addresses, key events, user passages or profile IDs are ever reported.
wait_inventory(){
  local phase="$1" count="$2" detected="$3" fresh="$4" ready=0
  for _ in $(seq 1 90);do
    if [[ -s "$TYPEFINCH_QA_HOTPLUG_REPORT" ]] \
      && lua5.4 - "$TYPEFINCH_QA_HOTPLUG_REPORT" "$count" "$detected" "$fresh" <<'LUA' >/dev/null 2>&1
local json=require("dkjson")
local f=assert(io.open(arg[1],"rb"))
local data=assert(json.decode(assert(f:read("*a"))));f:close()
assert(data.synthetic and data.inspection_only and data.state=="keyboard-inventory")
assert(data.count==tonumber(arg[2]))
assert(data.layout_detected==(arg[3]=="true"))
assert(data.fresh==(arg[4]=="true"))
assert(data.selected_index==0)
LUA
    then ready=1;break;fi
    sleep 0.15
  done
  ((ready==1)) || { echo "Native Wayland $phase state did not converge" >&2;exit 1; }
  import -window "$parent" "typefinch-hotplug-$phase.png"
  [[ -s "typefinch-hotplug-$phase.png" ]] || {
    echo "No native GUI $phase screenshot" >&2;exit 1;
  }
  echo "Native Wayland synthetic inventory $phase PASS: count=$count fresh=$fresh layout_detected=$detected"
}
wait_inventory initial 1 true true
cat >"$tmp/next.json" <<'JSON'
{"keyboards":[{"name":"Fixture A","address":"0x1","active_keymap":"English (US)"},{"name":"Fixture B","address":"0x2","active_keymap":"German"}]}
JSON
mv "$tmp/next.json" "$TYPEFINCH_QA_HOTPLUG_FIXTURE"
wait_inventory attached 2 false true
printf '%s\n' '{"keyboards":[{"name":"Fixture B","address":"0x2","active_keymap":"German"}]}' >"$tmp/next.json"
mv "$tmp/next.json" "$TYPEFINCH_QA_HOTPLUG_FIXTURE"
wait_inventory detached 1 true true
printf '%s\n' '{"keyboards":[]}' >"$tmp/next.json"
mv "$tmp/next.json" "$TYPEFINCH_QA_HOTPLUG_FIXTURE"
wait_inventory empty 0 false true
printf 'malformed-compositor-json\n' >"$tmp/next.json"
mv "$tmp/next.json" "$TYPEFINCH_QA_HOTPLUG_FIXTURE"
wait_inventory failed 0 false false
printf '%s\n' '{"keyboards":[{"name":"Fixture A","address":"0x1","active_keymap":"English (US)"}]}' >"$tmp/next.json"
mv "$tmp/next.json" "$TYPEFINCH_QA_HOTPLUG_FIXTURE"
wait_inventory recovered 1 true true
kill -0 "$app_pid" 2>/dev/null || {
  echo "Native Typefinch crashed during inspector lifecycle" >&2;exit 1;
}
echo 'Native Wayland GUI synthetic Hyprland metadata attach/detach/error/recovery PASS; physical hotplug not claimed'
