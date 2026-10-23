#!/usr/bin/env bash
# Headless native Wayland protocol acceptance; never a hardware keyboard QA.
set -euo pipefail
tmp="$(mktemp -d)"
weston_pid=""
trap 'if [ -n "$weston_pid" ]; then kill "$weston_pid" 2>/dev/null || true; fi; rm -rf "$tmp"' EXIT
mkdir -m 700 "$tmp/runtime"
export XDG_RUNTIME_DIR="$tmp/runtime"
export WAYLAND_DISPLAY=typefinch-wayland-test
export XDG_SESSION_TYPE=wayland
unset HYPRLAND_INSTANCE_SIGNATURE DISPLAY
weston --backend=headless-backend.so --socket="$WAYLAND_DISPLAY" --idle-time=0 --no-config --log="$tmp/weston.log" >"$tmp/weston.stdout" 2>&1 &
weston_pid=$!
for _ in $(seq 1 100); do
  [[ -S "$XDG_RUNTIME_DIR/$WAYLAND_DISPLAY" ]] && break
  if ! kill -0 "$weston_pid" 2>/dev/null; then cat "$tmp/weston.log"; exit 1; fi
  sleep 0.1
done
[[ -S "$XDG_RUNTIME_DIR/$WAYLAND_DISPLAY" ]] || { cat "$tmp/weston.log"; exit 1; }
./target/debug/typefinch --_probe_wayland_seats >"$tmp/seats.json"
lua5.4 - "$tmp/seats.json" <<'LUA'
local json=require("dkjson")
local f=assert(io.open(assert(arg[1]),"rb"))
local seats,_,err=json.decode(f:read("*a"))
f:close()
assert(not err and type(seats)=="table")
for _,seat in ipairs(seats) do
  assert(type(seat.id)=="number" and type(seat.name)=="string")
  assert(type(seat.keyboard)=="boolean")
  assert(seat.active_keymap==nil and seat.physical_device==nil)
end
print("Native Wayland protocol PASS: "..#seats.." aggregate seat globals; no key events or device attribution")
LUA
