#!/usr/bin/env bash
# A real Wayland wl_seat with keyboard capability, driven by nested Weston X11.
# Xvfb provides virtual input; this is NEVER physical/Hyprland hardware acceptance.
set -euo pipefail
: "${DISPLAY:?This smoke test requires a disposable Xvfb display}"
tmp="$(mktemp -d)"
weston_pid=""
trap 'if [[ -n "$weston_pid" ]]; then kill "$weston_pid" 2>/dev/null || true; wait "$weston_pid" 2>/dev/null || true; fi; rm -rf "$tmp"' EXIT
mkdir -m 700 "$tmp/runtime"
export XDG_RUNTIME_DIR="$tmp/runtime"
export WAYLAND_DISPLAY=typefinch-virtual-seat
export XDG_SESSION_TYPE=wayland
unset HYPRLAND_INSTANCE_SIGNATURE WAYLAND_SOCKET
weston --backend=x11-backend.so --renderer=pixman --socket="$WAYLAND_DISPLAY" --no-config \
  --idle-time=0 --width=800 --height=600 --log="$tmp/weston.log" \
  >"$tmp/weston.stdout" 2>&1 &
weston_pid=$!
for _ in $(seq 1 100); do
  [[ -S "$XDG_RUNTIME_DIR/$WAYLAND_DISPLAY" ]] && break
  if ! kill -0 "$weston_pid" 2>/dev/null; then
    echo "Nested Weston exited before creating socket" >&2
    cat "$tmp/weston.stdout" "$tmp/weston.log" >&2
    exit 1
  fi
  sleep 0.1
done
[[ -S "$XDG_RUNTIME_DIR/$WAYLAND_DISPLAY" ]] || {
  echo "Nested Weston Wayland socket not created" >&2
  cat "$tmp/weston.stdout" "$tmp/weston.log" >&2
  exit 1
}
# Validate real wl_registry/wl_seat replies from Weston; never bind wl_keyboard.
./target/debug/typefinch --_probe_wayland_seats >"$tmp/seats.json"
lua5.4 - "$tmp/seats.json" <<'LUA'
local json=require("dkjson")
local f=assert(io.open(assert(arg[1]),"rb"))
local seats,_,err=json.decode(f:read("*a"))
f:close()
assert(not err and type(seats)=="table")
local active=0
local ids={}
for _,seat in ipairs(seats) do
  assert(type(seat.id)=="number" and type(seat.name)=="string")
  assert(type(seat.keyboard)=="boolean")
  assert(seat.active_keymap==nil and seat.physical_device==nil and seat.key_events==nil)
  assert(not ids[seat.id],"each Wayland registry global must have a unique ID")
  ids[seat.id]=true
  if seat.keyboard then active=active+1 end
end
assert(active>0,"Nested Weston did not advertise a keyboard-capable virtual seat")
print("Native nested Weston PASS: "..active.." virtual keyboard-capable aggregate seat(s); no key capture")
LUA
TYPEFINCH_QA_WAYLAND_INSPECTOR="$PWD/target/debug/typefinch" \
  TYPEFINCH_QA_REQUIRE_KEYBOARD_SEAT=1 \
  cargo test devices::tests::native_wayland_child_inspector_is_bounded_and_unattributed -- --nocapture
