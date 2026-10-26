#!/usr/bin/env bash
# Native X11 proof of the experimental local pilot stages using ONLY fixed
# disposable synthetic input. No participant, real device or efficacy result.
set -Eeuo pipefail
export LIBGL_ALWAYS_SOFTWARE=1 WINIT_UNIX_BACKEND=x11 XDG_SESSION_TYPE=x11
tmp="$(mktemp -d)"
app_pid="" wm_pid=""
cleanup(){
  local status=$?
  if ((status!=0)) && [[ -s "$tmp/app.log" ]];then
    tail -n 80 "$tmp/app.log" >&2 || true
  fi
  [[ -z "$app_pid" ]] || { kill "$app_pid" 2>/dev/null || true; wait "$app_pid" 2>/dev/null || true; }
  [[ -z "$wm_pid" ]] || { kill "$wm_pid" 2>/dev/null || true; wait "$wm_pid" 2>/dev/null || true; }
  rm -rf "$tmp"
}
trap cleanup EXIT
mkdir -p "$tmp/config"
export XDG_CONFIG_HOME="$tmp/config"
export TYPEFINCH_QA_REPORT="$tmp/score.json"
export TYPEFINCH_QA_PILOT_REPORT="$tmp/pilot.json"
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
  kill -0 "$app_pid" 2>/dev/null || { echo "Native pilot app exited" >&2;exit 1; }
  sleep 0.2
done
[[ -n "$window" ]] || { echo "No native pilot window" >&2;exit 1; }
xdotool windowactivate --sync "$window" 2>/dev/null || true
xdotool windowfocus "$window"
config="$XDG_CONFIG_HOME/typefinch/settings.json"
selected=0
for _ in $(seq 1 30);do
  [[ -f "$config" ]] && grep -q '"test_mode": "custom"' "$config" && { selected=1;break; }
  xdotool key --clearmodifiers F11
  sleep 0.2
done
((selected==1)) || { echo "Native Custom selection failed" >&2;exit 1; }
xdotool type --clearmodifiers --delay 35 "$tmp/source.txt"
xdotool key --clearmodifiers Return
sleep 0.35
wait_score(){
  local tests="$1" entered="$2" correct="$3" ok=0
  for _ in $(seq 1 100);do
    if [[ -s "$TYPEFINCH_QA_REPORT" ]] && jq -e \
      --argjson t "$tests" --argjson n "$entered" --argjson c "$correct" \
      '.synthetic==true and .state=="finished" and .tests==$t and .entered==$n and .correct==$c' \
      "$TYPEFINCH_QA_REPORT" >/dev/null 2>&1;then ok=1;break;fi
    sleep 0.12
  done
  ((ok==1)) || { echo "Native pilot scored stage $tests ($entered/$correct) failed" >&2;exit 1; }
}
wait_pilot(){
  local expected="$1" ok=0
  for _ in $(seq 1 100);do
    if [[ -s "$TYPEFINCH_QA_PILOT_REPORT" ]] && jq -e --arg stage "$expected" \
      '.synthetic==true and .source=="fixed-disposable-fixture" and
        .stage==$stage and .causal_claim==false and
        (.arm=="adaptive" or .arm=="non_adaptive")' \
      "$TYPEFINCH_QA_PILOT_REPORT" >/dev/null 2>&1;then ok=1;break;fi
    sleep 0.12
  done
  ((ok==1)) || { echo "Native pilot did not reach stage $expected" >&2;exit 1; }
}
xdotool key --clearmodifiers F5
xdotool key --clearmodifiers F2
sleep 0.1
xdotool type --clearmodifiers --delay 55 axa
wait_score 1 3 2
[[ ! -e "$TYPEFINCH_QA_PILOT_REPORT" ]] || {
  echo "Unconsented synthetic pilot report existed before opt-in" >&2;exit 1;
}
# QA-only guarded Ctrl+Shift+P exercises the same consent and start method.
xdotool key --clearmodifiers ctrl+shift+p
wait_pilot practice
arm="$(jq -r '.arm' "$TYPEFINCH_QA_PILOT_REPORT")"
if [[ "$arm" == adaptive ]];then
  passage="b b b b b b b b"
else
  passage="a s d f j k l ;"
fi
xdotool key --clearmodifiers F2
sleep 0.12
xdotool type --clearmodifiers --delay 42 "$passage"
wait_score 2 15 15
wait_pilot immediate
xdotool key --clearmodifiers ctrl+shift+i
sleep 0.3
xdotool key --clearmodifiers F2
xdotool type --clearmodifiers --delay 55 aba
wait_score 3 3 3
wait_pilot delayed
# This QA does not fake an hour. The real delayed retest must stay LOCKED.
xdotool key --clearmodifiers ctrl+shift+d
sleep 0.3
jq -e '.stage=="delayed" and .causal_claim==false' "$TYPEFINCH_QA_PILOT_REPORT" >/dev/null
sqlite3 -cmd '.timeout 5000' -json "$XDG_CONFIG_HOME/typefinch/sessions.sqlite3" \
  "SELECT test_mode,input_mode,entered,correct FROM sessions ORDER BY id;" >"$tmp/rows.json"
jq -e 'length==3 and
  all(.[]; .test_mode=="custom" and .input_mode=="OS") and
  .[0].entered==3 and .[0].correct==2 and
  .[1].entered==15 and .[1].correct==15 and
  .[2].entered==3 and .[2].correct==3' "$tmp/rows.json" >/dev/null
! grep -q 'a s d f j k l ;' "$config"
! grep -q 'b b b b b b b b' "$config"
! grep -q 'aba' "$config"
echo "Native local held-out pilot PASS: baseline, randomized practice, immediate transfer, one-hour gate locked; no causal or private-data claim"
