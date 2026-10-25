#!/usr/bin/env bash
# Complete real native X11 adaptive drill -> identical-source retest.
# No real learner or efficacy claim. All input is a disposable synthetic fixture.
set -Eeuo pipefail
export LIBGL_ALWAYS_SOFTWARE=1 WINIT_UNIX_BACKEND=x11 XDG_SESSION_TYPE=x11
tmp="$(mktemp -d)"
app_pid="" wm_pid=""
cleanup(){
  local status=$?
  if ((status!=0)) && [[ -s "$tmp/app.log" ]];then
    tail -n 85 "$tmp/app.log" >&2 || true
  fi
  [[ -z "$app_pid" ]] || { kill "$app_pid" 2>/dev/null || true; wait "$app_pid" 2>/dev/null || true; }
  [[ -z "$wm_pid" ]] || { kill "$wm_pid" 2>/dev/null || true; wait "$wm_pid" 2>/dev/null || true; }
  rm -rf "$tmp"
}
trap cleanup EXIT
mkdir -p "$tmp/config"
export XDG_CONFIG_HOME="$tmp/config"
export TYPEFINCH_QA_REPORT="$tmp/score.json"
export TYPEFINCH_QA_TRAINING_REPORT="$tmp/followup.json"
export TYPEFINCH_QA_DURATION_SECONDS=15
printf 'aba' >"$tmp/source.txt"
setxkbmap -layout us
openbox --sm-disable >"$tmp/wm.log" 2>&1 &
wm_pid=$!
sleep 0.25
./target/debug/typefinch >"$tmp/app.log" 2>&1 &
app_pid=$!
window=""
for _ in $(seq 1 120);do
  window="$(xdotool search --onlyvisible --pid "$app_pid" --name Typefinch 2>/dev/null|head -n 1||true)"
  [[ -n "$window" ]] && break
  kill -0 "$app_pid" 2>/dev/null || {
    echo "Typefinch exited before practice comparison QA" >&2;exit 1;
  }
  sleep 0.2
done
[[ -n "$window" ]] || { echo "No native Typefinch window" >&2;exit 1; }
xdotool windowactivate --sync "$window" 2>/dev/null || true
xdotool windowfocus "$window"
config="$XDG_CONFIG_HOME/typefinch/settings.json"
selected=0
for _ in $(seq 1 30);do
  [[ -f "$config" ]] && grep -q '"test_mode": "custom"' "$config" && { selected=1;break; }
  xdotool key --clearmodifiers F11
  sleep 0.2
done
((selected==1)) || { echo "Custom mode never activated in actual native UI" >&2;exit 1; }
xdotool type --clearmodifiers --delay 35 "$tmp/source.txt"
xdotool key --clearmodifiers Return
sleep 0.35
wait_score(){
  local tests="$1" entered="$2" correct="$3" ready=0
  for _ in $(seq 1 80);do
    if [[ -s "$TYPEFINCH_QA_REPORT" ]] \
      && lua5.4 - "$TYPEFINCH_QA_REPORT" "$tests" "$entered" "$correct" <<'LUA' >/dev/null 2>&1
local json=require("dkjson")
local f=assert(io.open(arg[1],"rb"))
local r=assert(json.decode(assert(f:read("*a"))));f:close()
assert(r.synthetic and r.state=="finished" and
  r.tests==tonumber(arg[2]) and r.entered==tonumber(arg[3])
  and r.correct==tonumber(arg[4]))
LUA
    then ready=1;break;fi
    sleep 0.15
  done
  ((ready==1)) || { echo "Failed native practice stage $tests ($entered/$correct)" >&2;exit 1; }
}
xdotool key --clearmodifiers F5
xdotool key --clearmodifiers F2
sleep 0.12
xdotool type --clearmodifiers --delay 55 axa
wait_score 1 3 2
echo 'Native practice baseline PASS: 2/3 correct, one missed middle reference'
xdotool key --clearmodifiers ctrl+shift+k
sleep 0.35
xdotool key --clearmodifiers F2
xdotool type --clearmodifiers --delay 35 "b b b b b b b b"
wait_score 2 15 15
echo 'Native adaptive missed-character drill PASS: eight target repeats scored'
xdotool key --clearmodifiers ctrl+shift+r
sleep 0.35
xdotool key --clearmodifiers F2
xdotool type --clearmodifiers --delay 55 aba
wait_score 3 3 3
# The regular score report is written immediately before the separate
# in-memory trial audit in the same GUI frame. Observe both independently.
for _ in $(seq 1 60);do
  [[ -s "$TYPEFINCH_QA_TRAINING_REPORT" ]] && break
  sleep 0.1
done
[[ -s "$TYPEFINCH_QA_TRAINING_REPORT" ]] || {
  echo "No completed in-app paired comparison despite three saved sessions" >&2
  exit 1
}
lua5.4 - "$TYPEFINCH_QA_TRAINING_REPORT" "$config" "$tmp/source.txt" <<'LUA'
local json=require("dkjson")
local file=assert(io.open(arg[1],"rb"),"No actual in-app paired retest report")
local r=assert(json.decode(assert(file:read("*a"))));file:close()
assert(r.state=="same-source-followup" and r.synthetic
  and r.causal_claim==false,"No causal-efficacy claim is authorized")
assert(math.abs(r.before_accuracy-66.6666666667)<0.01
  and r.after_accuracy==100 and r.before_attempts==3
  and r.after_attempts==3,"Paired raw accuracy or samples mismatched")
assert(r.before_reference_observed==3 and r.before_reference_missed==1
  and r.after_reference_observed==3 and r.after_reference_missed==0,
  "Distinct scored reference opportunities must not be raw key attempts")
local db=os.getenv("XDG_CONFIG_HOME").."/typefinch/sessions.sqlite3"
local command=string.format("sqlite3 -cmd '.timeout 5000' -json %q 'SELECT test_mode,input_mode,entered,correct,attempts,mistakes FROM sessions ORDER BY id'",db)
local pipe=assert(io.popen(command,"r"))
local rows=assert(json.decode(assert(pipe:read("*a"))))
assert(pipe:close())
assert(#rows==3,"Need baseline, drill, retest saved in normal local history")
assert(rows[1].test_mode=="custom" and rows[1].entered==3
  and rows[1].correct==2 and rows[1].mistakes==1)
assert(rows[2].test_mode=="custom" and rows[2].entered==15
  and rows[2].correct==15)
assert(rows[3].test_mode=="custom" and rows[3].entered==3
  and rows[3].correct==3)
for _,row in ipairs(rows) do
  assert(row.input_mode=="OS","Mode must remain identical across trial")
end
local settings=assert(io.open(arg[2],"rb"))
local body=assert(settings:read("*a"));settings:close()
local source=assert(io.open(arg[3],"rb"))
local passage=assert(source:read("*a"));source:close()
assert(not body:find(arg[3],1,true) and not body:find(passage,1,true),
  "Private fixture source or local path persisted into settings")
assert(not body:find("b b b b b b b b",1,true),
  "Generated training text leaked into persistent preferences")
print("Native identical-source follow-up PASS: 3 saved scores, baseline 66.7% -> retest 100%, same-size sample and no causal claim")
LUA
import -window "$window" typefinch-training-followup.png
[[ -s typefinch-training-followup.png ]] || {
  echo "Missing actual native follow-up screenshot" >&2;exit 1;
}
echo 'Native Typefinch paired drill/retest GUI PASS; synthetic demonstration only, not actual training efficacy'
