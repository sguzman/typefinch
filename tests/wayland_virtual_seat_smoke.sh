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
./target/debug/typefinch --inspect-keyboards >"$tmp/seat-inventory.json"
lua5.4 - "$tmp/seat-inventory.json" <<'LUA'
local json=require("dkjson")
local f=assert(io.open(assert(arg[1]),"rb"))
local doc,_,err=json.decode(f:read("*a"));f:close()
assert(not err and doc.schema==1 and doc.capture_policy=="inspection-only")
assert(#doc.rows>0,"virtual keyboard-capable seat must be visible in public CLI")
for _,row in ipairs(doc.rows) do
  assert(row.inspection_id:match("^wayland:seat:%d+$"))
  assert(row.reported_active_keymap==nil or row.reported_active_keymap==json.null)
  assert(row.key_events==nil and row.physical_device==nil)
end
print("Wayland inspector CLI PASS: read-only virtual seat metadata, no keymap attribution")
LUA
TYPEFINCH_QA_WAYLAND_INSPECTOR="$PWD/target/debug/typefinch" \
  TYPEFINCH_QA_REQUIRE_KEYBOARD_SEAT=1 \
  cargo test devices::tests::native_wayland_child_inspector_is_bounded_and_unattributed -- --nocapture

# The same real X11-backed compositor can advertise a seat WITHOUT keyboard
# capability. The inspector must not invent a keyboard from that seat/global.
kill "$weston_pid" 2>/dev/null || true
wait "$weston_pid" 2>/dev/null || true
weston_pid=""
export WAYLAND_DISPLAY=typefinch-virtual-no-input
weston --backend=x11-backend.so --renderer=pixman --no-input \
  --socket="$WAYLAND_DISPLAY" --no-config --idle-time=0 \
  --width=800 --height=600 --log="$tmp/weston-no-input.log" \
  >"$tmp/weston-no-input.stdout" 2>&1 &
weston_pid=$!
for _ in $(seq 1 100); do
  [[ -S "$XDG_RUNTIME_DIR/$WAYLAND_DISPLAY" ]] && break
  if ! kill -0 "$weston_pid" 2>/dev/null; then
    echo "No-input Weston exited before creating socket" >&2
    cat "$tmp/weston-no-input.stdout" "$tmp/weston-no-input.log" >&2
    exit 1
  fi
  sleep 0.1
done
[[ -S "$XDG_RUNTIME_DIR/$WAYLAND_DISPLAY" ]] || {
  echo "No-input Weston Wayland socket not created" >&2
  cat "$tmp/weston-no-input.stdout" "$tmp/weston-no-input.log" >&2
  exit 1
}
./target/debug/typefinch --_probe_wayland_seats >"$tmp/no-input-seats.json"
lua5.4 - "$tmp/no-input-seats.json" <<'LUA'
local json=require("dkjson")
local f=assert(io.open(assert(arg[1]),"rb"))
local seats,_,err=json.decode(f:read("*a"))
f:close()
assert(not err and type(seats)=="table")
assert(#seats>0,"No-input Weston should still advertise its aggregate seat")
for _,seat in ipairs(seats) do
  assert(type(seat.id)=="number" and type(seat.name)=="string")
  assert(seat.keyboard==false,"Input-disabled Weston must not advertise keyboard capability")
  assert(seat.active_keymap==nil and seat.physical_device==nil)
end
print("Native nested Weston no-input PASS: "..#seats.." seat(s) with no keyboard capability")
LUA
./target/debug/typefinch --inspect-keyboards >"$tmp/no-input-inventory.json"
lua5.4 - "$tmp/no-input-inventory.json" <<'LUA'
local json=require("dkjson")
local f=assert(io.open(assert(arg[1]),"rb"))
local doc,_,err=json.decode(f:read("*a"));f:close()
assert(not err and doc.schema==1 and doc.capture_policy=="inspection-only")
assert(type(doc.rows)=="table" and #doc.rows==0,
  "a no-input Wayland seat must not be presented as a keyboard")
print("Wayland no-input inspector CLI PASS: empty keyboard inventory")
LUA
TYPEFINCH_QA_WAYLAND_INSPECTOR="$PWD/target/debug/typefinch" \
  TYPEFINCH_QA_EXPECT_NO_KEYBOARD_SEAT=1 \
  cargo test devices::tests::native_wayland_child_inspector_is_bounded_and_unattributed -- --nocapture
