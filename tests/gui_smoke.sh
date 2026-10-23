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
lua5.4 tests/smoke_assert.lua first "$tmp/report.json"
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
lua5.4 tests/smoke_assert.lua paste "$tmp/report.json"
# Exercise a real corrected error to separate raw from final metrics.
xdotool key --clearmodifiers F5
sleep 0.3
xdotool type --clearmodifiers --delay 32 "little finchex"
xdotool key --clearmodifiers BackSpace
xdotool type --clearmodifiers --delay 35 "s"
sleep 2.4
lua5.4 tests/smoke_assert.lua correction "$tmp/report.json"
xdotool key --clearmodifiers F6
svg="$XDG_CONFIG_HOME/typefinch/exports/typefinch-profile.svg"
md="$XDG_CONFIG_HOME/typefinch/exports/typefinch-profile.md"
for _ in $(seq 1 30); do
  if [[ -s "$svg" && -s "$md" ]]; then break; fi
  sleep 0.1
done
xmllint --noout "$svg"
lua5.4 tests/smoke_assert.lua export "$svg" "$md"

# Validate *explicit* appearance overrides on real native rendered frames.
# System-default screenshot above is still separately preserved.
kill "$app_pid" 2>/dev/null || true
wait "$app_pid" 2>/dev/null || true
app_pid=""
for theme in light dark; do
  lua5.4 tests/smoke_assert.lua set-theme "$XDG_CONFIG_HOME/typefinch/settings.json" "$theme"
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
lua5.4 tests/smoke_assert.lua theme "$light_luma" "$dark_luma"

# Complete a native untimed practice lesson; duration zero must not finish early.
lua5.4 tests/smoke_assert.lua prepare-lesson "$XDG_CONFIG_HOME/typefinch/settings.json"
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
lua5.4 tests/smoke_assert.lua lesson "$tmp/report.json"
