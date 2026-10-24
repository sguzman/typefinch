#!/usr/bin/env bash
# Real Fcitx5/XIM Pinyin, Anthy and Hangul scoring under disposable X11.
# This is not production Wayland/Hyprland or physical keyboard acceptance.
set -Eeuo pipefail
export LIBGL_ALWAYS_SOFTWARE=1 WINIT_UNIX_BACKEND=x11 XDG_SESSION_TYPE=x11
export XMODIFIERS=@im=fcitx GTK_IM_MODULE=fcitx QT_IM_MODULE=fcitx
export LC_ALL=C.UTF-8
# CLI variant is allowlisted: never interpolate arbitrary input into a shell
# command, config file or filename. Each invocation has isolated XDG/DBus.
case "${1:-pinyin}" in
  pinyin) engine=pinyin; passage='你'; keys=ni; commit_key=space ;;
  anthy) engine=anthy; passage='あ'; keys=a; commit_key=Return ;;
  hangul) engine=hangul; passage='한'; keys=gks; commit_key=space ;;
  *) echo 'Unknown synthetic IME fixture' >&2; exit 2 ;;
esac
tmp="$(mktemp -d)"
app='' wm=''
cleanup(){
  local code=$?
  if ((code!=0));then
    echo "Native Fcitx ${engine} probe failed" >&2
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
cat >"$XDG_CONFIG_HOME/fcitx5/profile" <<PROFILE
[Groups/0]
Name=Default
Default Layout=us
DefaultIM=${engine}

[Groups/0/Items/0]
Name=keyboard-us
Layout=

[Groups/0/Items/1]
Name=${engine}
Layout=us

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
# The daemon has no focused input context at startup, so readiness is
# based on the actual engine addon-load log, not an early -s/-n response.
for _ in $(seq 1 80);do
  grep -Eq "Loaded addon ${engine}$" "$tmp/fcitx.log" && break
  sleep 0.25
done
grep -Eq "Loaded addon ${engine}$" "$tmp/fcitx.log" || {
  echo "Fcitx ${engine} addon never finished loading" >&2;exit 1;
}
setxkbmap -layout us
printf '%s' "$passage" >"$tmp/${engine}.txt"
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
# Keep the source-path editor strictly Latin: turning on an IME during
# import would convert the temporary ASCII filesystem path to candidates.
fcitx5-remote -c
xdotool key --clearmodifiers F11
sleep 0.45
grep -q '"test_mode": "custom"' "$XDG_CONFIG_HOME/typefinch/settings.json" || {
  echo "Custom source editor not activated" >&2;exit 1;
}
fcitx5-remote -c
xdotool type --clearmodifiers --delay 35 "$tmp/${engine}.txt"
xdotool key --clearmodifiers Return
sleep 0.55
lua5.4 - "$TYPEFINCH_QA_IMPORT_REPORT" <<'LUA'
local json=require('dkjson')
local f=assert(io.open(assert(arg[1]),'rb'),'Missing synthetic IME source import')
local r=assert(json.decode(assert(f:read('*a'))));f:close()
assert(r.synthetic and r.state=='custom-import' and r.graphemes==1,
  'Expected exactly one imported Unicode grapheme')
LUA
xdotool windowfocus --sync "$window"
xdotool key --clearmodifiers F5
sleep 0.30
xdotool key --clearmodifiers F2
sleep 0.30
fcitx5-remote -s "$engine"
fcitx5-remote -o
sleep 0.35
[[ "$(fcitx5-remote -n)" == "$engine" ]] || {
  echo "Fcitx engine not selected while focused" >&2;exit 1;
}
[[ "$(fcitx5-remote)" == '2' ]] || {
  echo "Fcitx not active on focused native text surface" >&2;exit 1;
}
# Romanized key events go through the *actual* selected IME, not a
# synthetic egui ImeEvent. Preedit must not finish the scored session.
xdotool type --clearmodifiers --delay 125 "$keys"
sleep 0.4
[[ ! -s "$TYPEFINCH_QA_REPORT" ]] || {
  echo "Preedit incorrectly completed a typing session" >&2;exit 1;
}
xdotool key --clearmodifiers "$commit_key"
sleep 0.85
lua5.4 - "$TYPEFINCH_QA_REPORT" <<'LUA'
local json=require('dkjson')
local f=assert(io.open(assert(arg[1]),'rb'),'No Fcitx candidate commit score')
local r=assert(json.decode(assert(f:read('*a'))));f:close()
assert(r.synthetic and r.state=='finished' and r.correct==1 and r.entered==1
  and r.accuracy==100,
  'Fcitx candidate failed one-grapheme scoring')
local path=os.getenv('XDG_CONFIG_HOME')..'/typefinch/sessions.sqlite3'
local cmd=string.format('sqlite3 -json %q %q',path,
  'SELECT input_mode,test_mode,attempts,entered,correct FROM sessions')
local pipe=assert(io.popen(cmd,'r'))
local rows=assert(json.decode(assert(pipe:read('*a'))));assert(pipe:close())
assert(#rows==1 and rows[1].input_mode=='OS' and rows[1].test_mode=='custom'
  and rows[1].attempts==1 and rows[1].entered==1 and rows[1].correct==1,
  'Fcitx session persistence/provenance incorrect')
print('Native XIM Fcitx5 candidate commit PASS: 1/1 correct')
LUA
