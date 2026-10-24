#!/usr/bin/env bash
# Native X11 rendering smoke for synthetic Arabic and Hebrew custom passages.
# Tests actual shaped idle/active X11 frames, but not physical RTL keyboard
# typing, live compositor IMEs or complete keyboard editor caret behavior.
set -Eeuo pipefail
export LIBGL_ALWAYS_SOFTWARE=1 WINIT_UNIX_BACKEND=x11 XDG_SESSION_TYPE=x11
tmp="$(mktemp -d)"
app_pid="" wm_pid=""
cleanup(){
  status=$?
  if (( status != 0 )) && [[ -s "$tmp/app.log" ]]; then
    tail -n 60 "$tmp/app.log" >&2 || true
  fi
  if [[ -n "$app_pid" ]]; then
    kill "$app_pid" 2>/dev/null || true
    wait "$app_pid" 2>/dev/null || true
  fi
  if [[ -n "$wm_pid" ]]; then
    kill "$wm_pid" 2>/dev/null || true
    wait "$wm_pid" 2>/dev/null || true
  fi
  if [[ -n "${GITHUB_WORKSPACE:-}" ]]; then
    for img in "$tmp"/rtl-*.png;do
      [[ ! -s "$img" ]] || cp "$img" "$GITHUB_WORKSPACE/"
    done
  fi
  rm -rf "$tmp"
}
trap cleanup EXIT
export XDG_CONFIG_HOME="$tmp/config"
mkdir -p "$XDG_CONFIG_HOME"
printf '%s' 'سَلَام بالعربية 123' > "$tmp/arabic.txt"
printf '%s' 'שָׁלוֹם בעברית 123' > "$tmp/hebrew.txt"
openbox --sm-disable > "$tmp/wm.log" 2>&1 &
wm_pid=$!
sleep 0.4
./target/debug/typefinch > "$tmp/app.log" 2>&1 &
app_pid=$!
window=""
for _ in $(seq 1 200);do
  window="$(xdotool search --onlyvisible --name 'Typefinch' 2>/dev/null | head -n 1 || true)"
  [[ -z "$window" ]] || break
  if ! kill -0 "$app_pid" 2>/dev/null; then cat "$tmp/app.log" >&2;exit 1;fi
  sleep 0.25
done
[[ -n "$window" ]] || { echo 'No native RTL QA window' >&2;exit 1; }
focus_window(){
  xdotool windowactivate --sync "$window" 2>/dev/null || true
  xdotool windowfocus --sync "$window"
  sleep 0.25
}
focus_window
import -window "$window" "$tmp/rtl-baseline.png"
for name in arabic hebrew;do
  focus_window
  xdotool key --clearmodifiers F11
  sleep 0.45
  xdotool key --clearmodifiers ctrl+a
  xdotool type --clearmodifiers --delay 25 "$tmp/$name.txt"
  xdotool key --clearmodifiers Return
  sleep 0.7
  grep -q '"test_mode": "custom"' "$XDG_CONFIG_HOME/typefinch/settings.json" || {
    echo "RTL fixture did not reach Custom mode" >&2; exit 1;
  }
  focus_window
  import -window "$window" "$tmp/rtl-$name.png"
  [[ -s "$tmp/rtl-$name.png" ]] || { echo "No $name screenshot" >&2;exit 1; }
  delta="$(compare -metric AE "$tmp/rtl-baseline.png" "$tmp/rtl-$name.png" null: 2>&1 || true)"
  [[ "$delta" =~ ^[0-9]+$ && "$delta" -gt 1500 ]] || {
    echo "RTL $name fixture did not alter rendered frame: $delta" >&2;exit 1;
  }
  echo "Native $name fixture rendered: $delta changed pixels (not BiDi QA)"
  # The real app enters active typing mode with F5; capture COSMIC Text's
  # shaped caret position with no synthetic private text or physical key
  # attribution. Immediately cancel so the next fixture can import safely.
  focus_window
  xdotool key --clearmodifiers F5
  sleep 0.3
  xdotool key --clearmodifiers F2
  sleep 0.2
  import -window "$window" "$tmp/rtl-$name-running.png"
  [[ -s "$tmp/rtl-$name-running.png" ]] || {
    echo "No active RTL caret screenshot: $name" >&2;exit 1;
  }
  active_delta="$(compare -metric AE "$tmp/rtl-$name.png" "$tmp/rtl-$name-running.png" null: 2>&1 || true)"
  [[ "$active_delta" =~ ^[0-9]+$ && "$active_delta" -gt 100 ]] || {
    echo "RTL active frame never changed: $name / $active_delta" >&2;exit 1;
  }
  xdotool key --clearmodifiers Escape
  sleep 0.25
  echo "Native $name active typing frame captured: $active_delta changed pixels"
done
different="$(compare -metric AE "$tmp/rtl-arabic.png" "$tmp/rtl-hebrew.png" null: 2>&1 || true)"
[[ "$different" =~ ^[0-9]+$ && "$different" -gt 400 ]] || {
  echo "Arabic/Hebrew fixture screenshots indistinguishable: $different" >&2;exit 1;
}
echo 'RTL native shaped idle/active frames PASS (not hardware or live IME acceptance)'
