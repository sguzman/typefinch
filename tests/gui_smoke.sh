#!/usr/bin/env bash
# X11 / software Mesa smoke test. This is NOT a real Hyprland acceptance pass.
set -Eeuo pipefail
export LIBGL_ALWAYS_SOFTWARE=1
export WINIT_UNIX_BACKEND=x11
export RUST_BACKTRACE=1
tmp="$(mktemp -d)"
app_pid=""
wm_pid=""
cleanup() {
  if [[ -n "$app_pid" ]]; then kill "$app_pid" 2>/dev/null || true; wait "$app_pid" 2>/dev/null || true; fi
  if [[ -n "$wm_pid" ]]; then kill "$wm_pid" 2>/dev/null || true; wait "$wm_pid" 2>/dev/null || true; fi
  if [[ -n "${GITHUB_WORKSPACE:-}" ]]; then
    for img in typefinch-window.png typefinch-light.png typefinch-dark.png; do
      if [[ -f "$tmp/$img" ]]; then cp "$tmp/$img" "$GITHUB_WORKSPACE/$img"; fi
    done
  fi
  if [[ -n "${GITHUB_WORKSPACE:-}" ]]; then
    for file in "$XDG_CONFIG_HOME/typefinch/exports/typefinch-profile.svg" "$XDG_CONFIG_HOME/typefinch/exports/typefinch-profile.md"; do
      if [[ -s "$file" ]]; then cp "$file" "$GITHUB_WORKSPACE/"; fi
    done
  fi
  if [[ -n "${GITHUB_WORKSPACE:-}" ]] && [[ -f "$tmp/app.log" ]]; then
    cp "$tmp/app.log" "$GITHUB_WORKSPACE/typefinch-gui-smoke.log"
  fi
  rm -rf "$tmp"
}
trap cleanup EXIT
export XDG_CONFIG_HOME="$tmp/config"
export TYPEFINCH_QA_REPORT="$tmp/report.json"
openbox --sm-disable >"$tmp/window-manager.log" 2>&1 &
wm_pid=$!
sleep 0.5
./target/debug/typefinch >"$tmp/app.log" 2>&1 &
app_pid=$!
window=""
for _ in $(seq 1 60); do
  window="$(xdotool search --onlyvisible --name 'Typefinch' 2>/dev/null | head -n 1 || true)"
  if [[ -n "$window" ]]; then break; fi
  if ! kill -0 "$app_pid" 2>/dev/null; then
    echo "Typefinch exited before its window appeared"
    cat "$tmp/app.log"
    exit 1
  fi
  sleep 0.25
done
if [[ -z "$window" ]]; then
  echo "Typefinch window did not appear"
  cat "$tmp/app.log"
  exit 1
fi
xdotool windowactivate --sync "$window"
xdotool key --clearmodifiers F5
sleep 0.4
xdotool type --clearmodifiers --delay 55 "little finches practice every day"
sleep 2.6
if command -v import >/dev/null; then
  import -window "$window" "$tmp/typefinch-window.png"
fi
if [[ ! -s "$tmp/report.json" ]]; then
  echo "No completed scored session after synthetic typing"
  cat "$tmp/app.log"
  exit 1
fi
python3 - "$tmp/report.json" <<'PY'
import json,sys
r=json.load(open(sys.argv[1]))
assert r["synthetic"] and r["state"]=="finished",r
assert r["tests"] == 1, r
assert r["entered"] >= 8, r
assert r["correct"] >= 5, r
assert r["wpm"] > 0.0, r
assert 1500 <= r["elapsed_ms"] <= 2500, r
import sqlite3,os
dbpath=os.path.join(os.environ["XDG_CONFIG_HOME"],"typefinch","sessions.sqlite3")
with sqlite3.connect(dbpath) as db:
    rows=db.execute("SELECT entered,correct,scoring_version,test_mode FROM sessions").fetchall()
assert len(rows)==1 and rows[0][0]>=8 and rows[0][2]==1 and rows[0][3]=="words",rows
print(f"X11 GUI smoke PASS: {r['entered']} entered, {r['correct']} correct, {r['wpm']:.1f} WPM")
PY
if [[ ! -s "$tmp/typefinch-window.png" ]]; then
  echo "GUI screenshot missing"; exit 1
fi
echo "Native X11 window and synthetic typing session both verified."

# A second session exercises real clipboard paste suppression, not just unit logic.
xdotool key --clearmodifiers F5
sleep 0.3
xdotool type --clearmodifiers --delay 35 "little finches"
printf "PASTE SHOULD NEVER CHANGE A SCORED SESSION" | xclip -selection clipboard
xdotool key --clearmodifiers ctrl+v
sleep 2.4
python3 - "$tmp/report.json" <<'PY'
import json,os,sqlite3,sys
r=json.load(open(sys.argv[1]))
assert r["tests"]==2,r
assert r["entered"]==len("little finches"),r
assert r["correct"]==len("little finches"),r
with sqlite3.connect(os.path.join(os.environ["XDG_CONFIG_HOME"],"typefinch","sessions.sqlite3")) as db:
    rows=db.execute("SELECT entered,correct FROM sessions ORDER BY id").fetchall()
assert len(rows)==2 and rows[1]==(len("little finches"),len("little finches")),rows
print("Clipboard paste suppression PASS: synthetic paste was not counted")
PY
# Exercise a real corrected error to separate raw from final metrics.
xdotool key --clearmodifiers F5
sleep 0.3
xdotool type --clearmodifiers --delay 32 "little finchex"
xdotool key --clearmodifiers BackSpace
xdotool type --clearmodifiers --delay 35 "s"
sleep 2.4
python3 - "$tmp/report.json" <<'PY'
import json,os,sqlite3,sys
r=json.load(open(sys.argv[1]))
assert r["state"]=="finished" and r["tests"]==3,r
with sqlite3.connect(os.path.join(os.environ["XDG_CONFIG_HOME"],"typefinch","sessions.sqlite3")) as db:
    version=db.execute("PRAGMA user_version").fetchone()[0]
    rows=db.execute("SELECT entered, correct, attempts, mistakes, corrections, accuracy, raw_accuracy, wpm, raw_wpm FROM sessions ORDER BY id").fetchall()
assert version==3,version
assert len(rows)==3,rows
entered,correct,attempts,mistakes,corrections,accuracy,raw_accuracy,wpm,raw_wpm=rows[2]
assert entered==correct==len("little finches"),rows[2]
assert attempts==entered+1 and mistakes==1 and corrections==1,rows[2]
assert accuracy==100.0 and 0<raw_accuracy<100 and raw_wpm>wpm,rows[2]
print("Corrected-error QA PASS: raw mistakes survive backspacing; SQLite v3 round-trips both scores")
PY
xdotool key --clearmodifiers F6
svg="$XDG_CONFIG_HOME/typefinch/exports/typefinch-profile.svg"
md="$XDG_CONFIG_HOME/typefinch/exports/typefinch-profile.md"
for _ in $(seq 1 30); do
  if [[ -s "$svg" && -s "$md" ]]; then break; fi
  sleep 0.1
done
python3 - "$svg" "$md" <<'PY'
from pathlib import Path
from xml.etree import ElementTree
import sys
svg,md=map(Path,sys.argv[1:3])
root=ElementTree.parse(svg).getroot()
assert root.tag.endswith("svg"),root.tag
assert "<script" not in svg.read_text(encoding="utf-8")
assert "Finch" in svg.read_text(encoding="utf-8")
content=md.read_text(encoding="utf-8")
assert content.strip()=="![Typefinch typing profile](./typefinch-profile.svg)",content
print("SVG/Markdown export PASS: well-formed local static card, correct relative image embed")
PY

# Validate *explicit* appearance overrides on real native rendered frames.
# System-default screenshot above is still separately preserved.
kill "$app_pid" 2>/dev/null || true
wait "$app_pid" 2>/dev/null || true
app_pid=""
for theme in light dark; do
  python3 - "$XDG_CONFIG_HOME/typefinch/settings.json" "$theme" <<'PY'
import json,sys
path=sys.argv[1]
with open(path,encoding="utf-8") as f: data=json.load(f)
data["theme"]=sys.argv[2]
with open(path,"w",encoding="utf-8") as f: json.dump(data,f)
PY
  ./target/debug/typefinch >"$tmp/app-$theme.log" 2>&1 &
  app_pid=$!
  window=""
  for _ in $(seq 1 60); do
    window="$(xdotool search --onlyvisible --name 'Typefinch' 2>/dev/null | head -n 1 || true)"
    [[ -n "$window" ]] && break
    if ! kill -0 "$app_pid" 2>/dev/null; then cat "$tmp/app-$theme.log"; exit 1; fi
    sleep 0.25
  done
  [[ -n "$window" ]] || { echo "No $theme window"; exit 1; }
  xdotool windowactivate --sync "$window"
  sleep 0.5
  import -window "$window" "$tmp/typefinch-$theme.png"
  [[ -s "$tmp/typefinch-$theme.png" ]] || { echo "Missing $theme screenshot"; exit 1; }
  kill "$app_pid" 2>/dev/null || true
  wait "$app_pid" 2>/dev/null || true
  app_pid=""
done
light_luma="$(identify -format '%[fx:mean]' "$tmp/typefinch-light.png")"
dark_luma="$(identify -format '%[fx:mean]' "$tmp/typefinch-dark.png")"
python3 - "$light_luma" "$dark_luma" <<'PY'
import sys
light,dark=map(float,sys.argv[1:3])
assert light > dark + 0.25,(light,dark)
print(f"Theme render PASS: light luminance={light:.3f}, dark={dark:.3f}")
PY

# Complete a native untimed practice lesson; duration zero must not finish early.
python3 - "$XDG_CONFIG_HOME/typefinch/settings.json" <<'PY'
import json,sys
path=sys.argv[1]
with open(path,encoding="utf-8") as f: data=json.load(f)
data["test_mode"]="lesson"
data["lesson_id"]="home"
data["theme"]="dark"
with open(path,"w",encoding="utf-8") as f: json.dump(data,f)
PY
rm -f "$tmp/report.json"
./target/debug/typefinch >"$tmp/app-lesson.log" 2>&1 &
app_pid=$!
window=""
for _ in $(seq 1 60); do
  window="$(xdotool search --onlyvisible --name 'Typefinch' 2>/dev/null | head -n 1 || true)"
  [[ -n "$window" ]] && break
  if ! kill -0 "$app_pid" 2>/dev/null; then cat "$tmp/app-lesson.log"; exit 1; fi
  sleep 0.25
done
[[ -n "$window" ]] || { echo "No lesson window"; exit 1; }
xdotool windowactivate --sync "$window"
xdotool key --clearmodifiers F5
sleep 0.3
xdotool type --clearmodifiers --delay 27 "asdf jkl; asdf jkl; fj dk sl a; sad flask; jill asks; asdf jkl;"
sleep 0.7
python3 - "$tmp/report.json" <<'PY'
import json,os,sqlite3,sys
report=json.load(open(sys.argv[1]))
assert report["state"]=="finished" and report["tests"]==4,report
expected="asdf jkl; asdf jkl; fj dk sl a; sad flask; jill asks; asdf jkl;"
with sqlite3.connect(os.path.join(os.environ["XDG_CONFIG_HOME"],"typefinch","sessions.sqlite3")) as db:
    row=db.execute("SELECT test_mode,duration_ms,elapsed_ms,entered,correct FROM sessions ORDER BY id DESC LIMIT 1").fetchone()
assert row is not None
mode,duration,elapsed,entered,correct=row
assert mode=="lesson/home" and duration==0 and elapsed>0,row
assert entered==correct==len(expected),row
print("Untimed practice PASS: native typing finished on last lesson character and persisted with distinct tag")
PY
