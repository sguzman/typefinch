#!/usr/bin/env bash
# Probe the public read-only inventory CLI in isolated X11, then fail closed.
# No physical device routing, global keyboard hooks or user files involved.
set -euo pipefail
tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT
env -u DISPLAY -u WAYLAND_DISPLAY -u WAYLAND_SOCKET \
    -u HYPRLAND_INSTANCE_SIGNATURE XDG_SESSION_TYPE=tty \
    ./target/debug/typefinch --inspect-keyboards >"$tmp/unavailable.json" 2>"$tmp/unavailable.err" \
    && { echo "Inspector incorrectly succeeded without a compositor" >&2; exit 1; } || true
[[ ! -s "$tmp/unavailable.json" ]]
grep -F 'Keyboard inspection unavailable:' "$tmp/unavailable.err"
xvfb-run -a -s "-screen 0 1280x900x24" \
    env -u HYPRLAND_INSTANCE_SIGNATURE XDG_SESSION_TYPE=x11 \
    ./target/debug/typefinch --inspect-keyboards >"$tmp/x11.json"
lua5.4 - "$tmp/x11.json" <<'LUA'
local json=require("dkjson")
local f=assert(io.open(assert(arg[1]),"rb"))
local doc,_,err=json.decode(f:read("*a"));f:close()
assert(not err and doc.schema==1 and doc.capture_policy=="inspection-only")
assert(type(doc.rows)=="table" and #doc.rows==1,"X11 is an aggregate seat")
local seat=doc.rows[1]
assert(seat.inspection_id=="x11:aggregate-seat")
assert(type(seat.configured_layout)=="string" and #seat.configured_layout>0)
assert(seat.reported_active_keymap==nil or seat.reported_active_keymap==json.null,
    "setxkbmap configuration is not an observed active group")
assert(seat.key_events==nil and seat.physical_device==nil)
print("Read-only inspector CLI PASS: X11 aggregate seat, no input attribution")
LUA
