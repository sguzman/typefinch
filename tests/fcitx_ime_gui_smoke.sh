#!/usr/bin/env bash
# Experimental real Fcitx5/XIM Pinyin commit under disposable X11.
# This is not production Wayland/Hyprland or physical keyboard acceptance.
set -Eeuo pipefail
export LIBGL_ALWAYS_SOFTWARE=1 WINIT_UNIX_BACKEND=x11 XDG_SESSION_TYPE=x11
export XMODIFIERS=@im=fcitx GTK_IM_MODULE=fcitx QT_IM_MODULE=fcitx
export LC_ALL=C.UTF-8
tmp="$(mktemp -d)"
app='' wm=''
cleanup(){
  local code=$?
  if ((code!=0));then
    echo "Native Fcitx Pinyin probe failed" >&2
    [[ ! -f "$tmp/fcitx.log" ]] || tail -n 90 "$tmp/fcitx.log" >&2
    [[ ! -f "$tmp/app.log" ]] || tail -n 90 "$tmp/app.log" >&2
  fi
  [[ -z "$app" ]] || { kill "$app" 2>/dev/null || true; wait "$app" 2>/dev/null || true; }
  [[ -z "$wm" ]] || { kill "$wm" 2>/dev/null || true; wait "$wm" 2>/dev/null || true; }
  fcitx5-remote -e >/dev/null 2>&1 || true
  rm -rf "$tmp"
}
trap cleanup EXIT
export XDG_CONFIG_HOME="$tmp/config" XDG_DATA_HOME="$tmp/data"
export TYPEFINCH_QA_REPORT="$tmp/score.json" TYPEFINCH_QA_IMPORT_REPORT="$tmp/import.json"
mkdir -p "$XDG_CONFIG_HOME/fcitx5" "$XDG_DATA_HOME"
cat >"$XDG_CONFIG_HOME/fcitx5/profile" <<'PROFILE'
[Groups/0]
Name=Default
Default Layout=us
DefaultIM=pinyin

[Groups/0/Items/0]
Name=keyboard-us
Layout=

[Groups/0/Items/1]
Name=pinyin
Layout=

[GroupOrder]
0=Default
PROFILE
cat >"$XDG_CONFIG_HOME/fcitx5/config" <<'CONFIG'
[Behavior]
ActiveByDefault=False
CONFIG
fcitx5 -d >"$tmp/fcitx.log" 2>&1
# Fcitx 5.1.x (Ubuntu 24.04) does not support remote -l. Probe its
# documented running-state and selected-method APIs instead.
for _ in $(seq 1 80);do
  fcitx5-remote --check >/dev/null 2>&1 && break
  sleep 0.25
done
fcitx5-remote --check >/dev/null 2>&1 || {
  echo "Fcitx5 DBus controller unavailable" >&2; exit 1;
}
# Fcitx advertises its D-Bus controller before asynchronously loading
# the installed Pinyin dictionary/addon. Retry selection until ready.
for _ in $(seq 1 80);do
  fcitx5-remote -s pinyin >/dev/null 2>&1 || true
  [[ "$(fcitx5-remote -n 2>/dev/null || true)" == pinyin ]] && break
  sleep 0.25
done
[[ "$(fcitx5-remote -n)" == pinyin ]] || {
  echo "Installed Pinyin engine did not become selected after startup" >&2;exit 1;
}
setxkbmap -layout us
printf '%s' '你' >"$tmp/chinese.txt"
openbox --sm-disable >"$tmp/wm.log" 2>&1 &
wm=$!
./target/debug/typefinch >"$tmp/app.log" 2>&1 &
app=$!
window=''
for _ in $(seq 1 140);do
  window="$(xdotool search --onlyvisible --name Typefinch 2>/dev/null | head -n 1 || true)"
  [[ -z "$window" ]] || break
  kill -0 "$app" 2>/dev/null || { echo "Native IME application exited" >&2;exit 1; }
  sleep 0.25
done
[[ -n "$window" ]] || { echo "Native IME window missing" >&2; exit 1; }
xdotool windowactivate --sync "$window" 2>/dev/null || true
xdotool windowfocus --sync "$window"
sleep 0.45
# Keep the source-path editor strictly Latin: enabling Pinyin before
# importing a synthetic UTF-8 file would turn the path into IME candidates.
fcitx5-remote -c
xdotool key --clearmodifiers F11
sleep 0.45
grep -q '"test_mode": "custom"' "$XDG_CONFIG_HOME/typefinch/settings.json" || {
  echo "Custom source editor not activated" >&2;exit 1;
}
fcitx5-remote -c
xdotool type --clearmodifiers --delay 35 "$tmp/chinese.txt"
xdotool key --clearmodifiers Return
sleep 0.55
lua5.4 - "$TYPEFINCH_QA_IMPORT_REPORT" <<'LUA'
local json=require('dkjson')
local f=assert(io.open(assert(arg[1]),'rb'),'Missing Chinese source import')
local r=assert(json.decode(assert(f:read('*a'))));f:close()
assert(r.synthetic and r.state=='custom-import' and r.graphemes==1,
  'Expected exactly one imported Chinese grapheme')
LUA
xdotool windowfocus --sync "$window"
xdotool key --clearmodifiers F5
sleep 0.30
xdotool key --clearmodifiers F2
sleep 0.30
fcitx5-remote -s pinyin
fcitx5-remote -o
sleep 0.35
[[ "$(fcitx5-remote -n)" == 'pinyin' ]] || {
  echo "Fcitx engine not selected while focused" >&2;exit 1;
}
[[ "$(fcitx5-remote)" == '2' ]] || {
  echo "Fcitx not active on focused native text surface" >&2;exit 1;
}
# Latin key events become one real Pinyin IME candidate. Their preedit
# must not score or be persisted as an attempt before Space commits 你.
xdotool type --clearmodifiers --delay 125 'ni'
sleep 0.4
[[ ! -s "$TYPEFINCH_QA_REPORT" ]] || {
  echo "Preedit incorrectly completed a typing session" >&2;exit 1;
}
xdotool key --clearmodifiers space
sleep 0.85
lua5.4 - "$TYPEFINCH_QA_REPORT" <<'LUA'
local json=require('dkjson')
local f=assert(io.open(assert(arg[1]),'rb'),'No Fcitx candidate commit score')
local r=assert(json.decode(assert(f:read('*a'))));f:close()
assert(r.synthetic and r.state=='finished' and r.correct==1 and r.entered==1
  and r.attempts==1 and r.accuracy==100,
  'Fcitx candidate failed one-grapheme scoring')
local path=os.getenv('XDG_CONFIG_HOME')..'/typefinch/sessions.sqlite3'
local cmd=string.format('sqlite3 -json %q %q',path,
  'SELECT input_mode,test_mode,attempts,entered,correct FROM sessions')
local pipe=assert(io.popen(cmd,'r'))
local rows=assert(json.decode(assert(pipe:read('*a'))));assert(pipe:close())
assert(#rows==1 and rows[1].input_mode=='OS' and rows[1].test_mode=='custom'
  and rows[1].attempts==1 and rows[1].entered==1 and rows[1].correct==1,
  'Fcitx session persistence/provenance incorrect')
print('Native XIM Fcitx5 Pinyin commit PASS: ni -> 你; 1/1 correct')
LUA
