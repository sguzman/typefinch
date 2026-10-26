#!/usr/bin/env bash
# Synthetic-only native X11 regression: a private custom multiline target
# must actually score Enter, not silently flatten imported paragraph breaks.
set -Eeuo pipefail
export LIBGL_ALWAYS_SOFTWARE=1 WINIT_UNIX_BACKEND=x11 XDG_SESSION_TYPE=x11
tmp="$(mktemp -d)"
app_pid="" wm_pid=""
cleanup(){
  status=$?
  if (( status != 0 )) && [[ -s "$tmp/app.log" ]];then tail -n 75 "$tmp/app.log" >&2;fi
  [[ -z "$app_pid" ]] || { kill "$app_pid" 2>/dev/null || true;wait "$app_pid" 2>/dev/null || true; }
  [[ -z "$wm_pid" ]] || { kill "$wm_pid" 2>/dev/null || true;wait "$wm_pid" 2>/dev/null || true; }
  rm -rf "$tmp"
}
trap cleanup EXIT
mkdir -p "$tmp/config"
export XDG_CONFIG_HOME="$tmp/config"
export TYPEFINCH_QA_REPORT="$tmp/result.json"
printf 'a\nb' > "$tmp/source.txt"
# xvfb-run may export DISPLAY before its X server is accepting
# connections. Validate the actual protocol before launching production
# winit; an XOpenDisplayFailed at this point says nothing about typing.
ready=0
for _ in $(seq 1 60);do
  if xdotool getdisplaygeometry >/dev/null 2>&1;then ready=1;break;fi
  sleep 0.1
done
((ready==1)) || { echo 'Xvfb never accepted X11 client connections' >&2;exit 1; }
openbox --sm-disable >"$tmp/wm.log" 2>&1 &
wm_pid=$!
./target/debug/typefinch >"$tmp/app.log" 2>&1 &
app_pid=$!
window=""
for _ in $(seq 1 200);do
  window="$(xdotool search --onlyvisible --name 'Typefinch' 2>/dev/null | head -n 1 || true)"
  [[ -z "$window" ]] || break
  kill -0 "$app_pid" 2>/dev/null || {
    echo 'Native multiline app exited before mapping its X11 window' >&2
    [[ ! -s "$tmp/wm.log" ]] || tail -n 30 "$tmp/wm.log" >&2
    exit 1
  }
  sleep 0.25
done
[[ -n "$window" ]] || { echo 'No native multiline window' >&2;exit 1; }
xdotool windowactivate --sync "$window" 2>/dev/null || true
xdotool windowfocus --sync "$window"
sleep 0.5
xdotool key --clearmodifiers F11
sleep 0.45
grep -q '"test_mode": "custom"' "$XDG_CONFIG_HOME/typefinch/settings.json"
xdotool type --clearmodifiers --delay 20 "$tmp/source.txt"
xdotool key --clearmodifiers Return
sleep 0.5
xdotool key --clearmodifiers F5
sleep 0.35
xdotool key --clearmodifiers F2
sleep 0.25
xdotool type --clearmodifiers 'a'
xdotool key --clearmodifiers Return
sleep 0.25
[[ ! -s "$TYPEFINCH_QA_REPORT" ]] || {
  echo 'Multiline reference completed before the final b: newline was lost' >&2; exit 1;
}
xdotool type --clearmodifiers 'b'
sleep 0.85
lua5.4 - "$TYPEFINCH_QA_REPORT" "$tmp/source.txt" <<'LUA'
local json=require("dkjson")
local path,source=assert(arg[1]),assert(arg[2])
local file=assert(io.open(path,"rb"),"Multiline native test did not complete")
local result=assert(json.decode(assert(file:read("*a"))))
file:close()
assert(result.synthetic==true and result.state=="finished","Not a finished synthetic session")
assert(result.entered==3 and result.correct==3 and result.accuracy==100,
  "Custom multiline input was flattened, duplicated or mistyped")
local sql=assert(io.popen("sqlite3 -json "..string.format("%q",os.getenv("XDG_CONFIG_HOME")..
  "/typefinch/sessions.sqlite3").." 'SELECT entered,correct,test_mode FROM sessions'","r"))
local records=assert(json.decode(assert(sql:read("*a"))))
assert(sql:close())
assert(#records==1 and records[1].test_mode=="custom" and records[1].entered==3
  and records[1].correct==3,"Native multiline SQLite score did not match")
local cfg=assert(io.open(os.getenv("XDG_CONFIG_HOME").."/typefinch/settings.json","rb"))
local body=assert(cfg:read("*a"));cfg:close()
assert(not body:find("a\\nb",1,true),"Source passage leaked into settings")
assert(not body:find(source,1,true),"Private local file path persisted")
print("Native custom multiline X11 PASS: a, Enter, b -> 3/3, no source persistence")
LUA
