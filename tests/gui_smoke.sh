#!/usr/bin/env bash
# X11 / software Mesa smoke test. This is NOT a real Hyprland acceptance pass.
set -Eeuo pipefail
export LIBGL_ALWAYS_SOFTWARE=1
export WINIT_UNIX_BACKEND=x11
export RUST_BACKTRACE=1
tmp="$(mktemp -d)"
app_pid=""
wm_pid=""
secondary_pid=""
cleanup() {
  if [[ -n "$secondary_pid" ]]; then
    kill "$secondary_pid" 2>/dev/null || true
    wait "$secondary_pid" 2>/dev/null || true
  fi
  if [[ -n "$app_pid" ]]; then kill "$app_pid" 2>/dev/null || true; wait "$app_pid" 2>/dev/null || true; fi
  if [[ -n "$wm_pid" ]]; then kill "$wm_pid" 2>/dev/null || true; wait "$wm_pid" 2>/dev/null || true; fi
  if [[ -n "${GITHUB_WORKSPACE:-}" ]]; then
    for img in typefinch-window.png typefinch-light.png typefinch-dark.png typefinch-narrow.png typefinch-hidpi.png typefinch-recovery.png; do
      if [[ -f "$tmp/$img" ]]; then cp "$tmp/$img" "$GITHUB_WORKSPACE/$img"; fi
    done
  fi
  if [[ -n "${GITHUB_WORKSPACE:-}" ]]; then
    for file in "$XDG_CONFIG_HOME/typefinch/exports/typefinch-profile.svg" "$XDG_CONFIG_HOME/typefinch/exports/typefinch-profile.md" "$XDG_CONFIG_HOME/typefinch/exports/typefinch-profile.png"; do
      if [[ -s "$file" ]]; then cp "$file" "$GITHUB_WORKSPACE/"; fi
    done
  fi
  if [[ -n "${GITHUB_WORKSPACE:-}" ]] && [[ -f "$XDG_CONFIG_HOME/typefinch/exports/typefinch-history.json" ]]; then
    cp "$XDG_CONFIG_HOME/typefinch/exports/typefinch-history.json" "$GITHUB_WORKSPACE/"
  fi
  if [[ -n "${GITHUB_WORKSPACE:-}" ]] && [[ -f "$tmp/app.log" ]]; then
    cp "$tmp/app.log" "$GITHUB_WORKSPACE/typefinch-gui-smoke.log"
  fi
  if [[ -n "${GITHUB_WORKSPACE:-}" ]] && [[ -f "$tmp/window-manager.log" ]]; then
    cp "$tmp/window-manager.log" "$GITHUB_WORKSPACE/typefinch-window-manager.log"
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
for _ in $(seq 1 200); do
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
  echo "Typefinch window did not become visible after 50 seconds"
  xdotool search --name 'Typefinch' || true
  xwininfo -root -tree | head -n 80 || true
  cat "$tmp/window-manager.log" || true
  cat "$tmp/app.log"
  exit 1
fi
if ! xdotool windowactivate --sync "$window" 2>/dev/null; then
  echo "EWMH activation unavailable; forcing X11 keyboard focus"
fi
xdotool windowfocus --sync "$window"
# The native window can become visible before its first egui input frame.
sleep 0.4
# Window focus can move after ImageMagick captures and restarts.
# Explicit refocus prevents a missed F5 from silently corrupting QA.
xdotool windowactivate --sync "$window" 2>/dev/null || true
xdotool windowfocus --sync "$window"
sleep 0.3
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
# Window focus can move after ImageMagick captures and restarts.
# Explicit refocus prevents a missed F5 from silently corrupting QA.
xdotool windowactivate --sync "$window" 2>/dev/null || true
xdotool windowfocus --sync "$window"
sleep 0.3
xdotool key --clearmodifiers F5
sleep 0.3
xdotool type --clearmodifiers --delay 35 "little finches"
printf "PASTE SHOULD NEVER CHANGE A SCORED SESSION" | xclip -selection clipboard
xdotool key --clearmodifiers ctrl+v
sleep 2.4
lua5.4 tests/smoke_assert.lua paste "$tmp/report.json"
# Exercise a real corrected error to separate raw from final metrics.
# Window focus can move after ImageMagick captures and restarts.
# Explicit refocus prevents a missed F5 from silently corrupting QA.
xdotool windowactivate --sync "$window" 2>/dev/null || true
xdotool windowfocus --sync "$window"
sleep 0.3
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
xdotool key --clearmodifiers F7
stats="$XDG_CONFIG_HOME/typefinch/exports/typefinch-history.json"
for _ in $(seq 1 30); do
  [[ -s "$stats" ]] && break
  sleep 0.1
done
lua5.4 tests/smoke_assert.lua stats "$stats"

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
  for _ in $(seq 1 200); do
    window="$(xdotool search --onlyvisible --name 'Typefinch' 2>/dev/null | head -n 1 || true)"
    [[ -n "$window" ]] && break
    if ! kill -0 "$app_pid" 2>/dev/null; then cat "$tmp/app-$theme.log"; exit 1; fi
    sleep 0.25
  done
  [[ -n "$window" ]] || { echo "No $theme window"; exit 1; }
  if ! xdotool windowactivate --sync "$window" 2>/dev/null; then
  echo "EWMH activation unavailable; forcing X11 keyboard focus"
fi
xdotool windowfocus --sync "$window"
# The native window can become visible before its first egui input frame.
sleep 0.4
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
for _ in $(seq 1 200); do
  window="$(xdotool search --onlyvisible --name 'Typefinch' 2>/dev/null | head -n 1 || true)"
  [[ -n "$window" ]] && break
  if ! kill -0 "$app_pid" 2>/dev/null; then cat "$tmp/app-lesson.log"; exit 1; fi
  sleep 0.25
done
[[ -n "$window" ]] || { echo "No lesson window"; exit 1; }
if ! xdotool windowactivate --sync "$window" 2>/dev/null; then
  echo "EWMH activation unavailable; forcing X11 keyboard focus"
fi
xdotool windowfocus --sync "$window"
# The native window can become visible before its first egui input frame.
sleep 0.4
# Window focus can move after ImageMagick captures and restarts.
# Explicit refocus prevents a missed F5 from silently corrupting QA.
xdotool windowactivate --sync "$window" 2>/dev/null || true
xdotool windowfocus --sync "$window"
sleep 0.3
xdotool key --clearmodifiers F5
sleep 0.3
xdotool type --clearmodifiers --delay 27 "asdf jkl; asdf jkl; fj dk sl a; sad flask; jill asks; asdf jkl;"
sleep 0.7
lua5.4 tests/smoke_assert.lua lesson "$tmp/report.json"

# Verify the running native GUI honors a persisted Mexican Spanish Easy
# language selection and never merges it with existing English history.
kill "$app_pid" 2>/dev/null || true
wait "$app_pid" 2>/dev/null || true
app_pid=""
lua5.4 tests/smoke_assert.lua prepare-spanish "$XDG_CONFIG_HOME/typefinch/settings.json"
rm -f "$tmp/report.json"
./target/debug/typefinch >"$tmp/app-spanish.log" 2>&1 &
app_pid=$!
window=""
for _ in $(seq 1 200); do
  window="$(xdotool search --onlyvisible --name 'Typefinch' 2>/dev/null | head -n 1 || true)"
  [[ -n "$window" ]] && break
  if ! kill -0 "$app_pid" 2>/dev/null; then cat "$tmp/app-spanish.log"; exit 1; fi
  sleep 0.25
done
[[ -n "$window" ]] || { echo "No Spanish test window"; exit 1; }
xdotool windowactivate --sync "$window" 2>/dev/null || true
xdotool windowfocus --sync "$window"
sleep 0.4
# Window focus can move after ImageMagick captures and restarts.
# Explicit refocus prevents a missed F5 from silently corrupting QA.
xdotool windowactivate --sync "$window" 2>/dev/null || true
xdotool windowfocus --sync "$window"
sleep 0.3
xdotool key --clearmodifiers F5
sleep 0.3
xdotool type --clearmodifiers --delay 35 "casa mesa gato perro"
sleep 2.4
lua5.4 tests/smoke_assert.lua spanish "$tmp/report.json"

# An explicitly requested F8 backup must preserve every profile record.
# This test uses only synthetic disposable profile settings.
xdotool key --clearmodifiers F8
for _ in $(seq 1 40); do
  shopt -s nullglob
  backups=("$XDG_CONFIG_HOME/typefinch/exports"/typefinch-backup-*-sessions.sqlite3)
  [[ ${#backups[@]} -gt 0 ]] && break
  sleep 0.1
done
[[ ${#backups[@]} -eq 1 ]] || { echo "Full backup was not created"; exit 1; }
backup_db="${backups[0]}"
backup_settings="${backup_db%-sessions.sqlite3}-settings.json"
[[ -s "$backup_settings" ]] || { echo "Full backup settings are missing"; exit 1; }
lua5.4 tests/smoke_assert.lua backup "$backup_db" "$backup_settings"
./target/debug/typefinch --verify-backup "$backup_settings" "$backup_db"
# Rejected inputs must not be changed by the validator.
cp "$backup_settings" "$tmp/bad-backup-settings.json"
printf 'not-json' >"$tmp/bad-backup-settings.json"
if ./target/debug/typefinch --verify-backup "$tmp/bad-backup-settings.json" "$backup_db"; then
  echo "Backup validator accepted malformed settings" >&2
  exit 1
fi
echo "Read-only backup verification PASS"
mkdir -p "$tmp/restore-staging"
restored="$tmp/restore-staging/typefinch"
./target/debug/typefinch --restore-backup-to-new-dir "$backup_settings" "$backup_db" "$restored"
cmp "$backup_settings" "$restored/settings.json"
cmp "$backup_db" "$restored/sessions.sqlite3"
if ./target/debug/typefinch --restore-backup-to-new-dir "$backup_settings" "$backup_db" "$restored"; then
  echo "Restore staging unexpectedly overwrote existing destination"
  exit 1
fi
echo "Safe restore-to-new-directory PASS"

# Native SVG profile designer: verify selected palette and font are honored.
kill "$app_pid" 2>/dev/null || true
wait "$app_pid" 2>/dev/null || true
app_pid=""
lua5.4 tests/smoke_assert.lua prepare-card-theme "$XDG_CONFIG_HOME/typefinch/settings.json"
./target/debug/typefinch >"$tmp/app-card.log" 2>&1 &
app_pid=$!
window=""
for _ in $(seq 1 200); do
  window="$(xdotool search --onlyvisible --name 'Typefinch' 2>/dev/null | head -n 1 || true)"
  [[ -n "$window" ]] && break
  if ! kill -0 "$app_pid" 2>/dev/null; then cat "$tmp/app-card.log"; exit 1; fi
  sleep 0.25
done
[[ -n "$window" ]] || { echo "No profile-card window"; exit 1; }
xdotool windowactivate --sync "$window" 2>/dev/null || true
xdotool windowfocus --sync "$window"
sleep 0.4
xdotool key --clearmodifiers F6
for _ in $(seq 1 40); do
  [[ -s "$XDG_CONFIG_HOME/typefinch/exports/typefinch-profile.svg" ]] && break
  sleep 0.1
done
xmllint --noout "$XDG_CONFIG_HOME/typefinch/exports/typefinch-profile.svg"
lua5.4 tests/smoke_assert.lua card-theme "$XDG_CONFIG_HOME/typefinch/exports/typefinch-profile.svg"

xdotool key --clearmodifiers F9
png="$XDG_CONFIG_HOME/typefinch/exports/typefinch-profile.png"
for _ in $(seq 1 100); do
  [[ -s "$png" ]] && break
  sleep 0.1
done
lua5.4 tests/smoke_assert.lua png "$png"

# Distinguish actual in-app personal-layout emulation from OS text entry.
kill "$app_pid" 2>/dev/null || true
wait "$app_pid" 2>/dev/null || true
app_pid=""
lua5.4 tests/smoke_assert.lua prepare-personal-keymap "$XDG_CONFIG_HOME/typefinch/settings.json"
rm -f "$tmp/report.json"
./target/debug/typefinch >"$tmp/app-personal.log" 2>&1 &
app_pid=$!
window=""
for _ in $(seq 1 200); do
  window="$(xdotool search --onlyvisible --name 'Typefinch' 2>/dev/null | head -n 1 || true)"
  [[ -n "$window" ]] && break
  if ! kill -0 "$app_pid" 2>/dev/null; then cat "$tmp/app-personal.log"; exit 1; fi
  sleep 0.25
done
[[ -n "$window" ]] || { echo "No personal layout window"; exit 1; }
xdotool windowactivate --sync "$window" 2>/dev/null || true
xdotool windowfocus --sync "$window"
sleep 0.4
# Window focus can move after ImageMagick captures and restarts.
# Explicit refocus prevents a missed F5 from silently corrupting QA.
xdotool windowactivate --sync "$window" 2>/dev/null || true
xdotool windowfocus --sync "$window"
sleep 0.3
xdotool key --clearmodifiers F5
sleep 0.3
xdotool type --clearmodifiers --delay 29 "cat dog sun moon"
sleep 2.4
lua5.4 tests/smoke_assert.lua personal-keymap "$tmp/report.json"

# Real X11 physical-key integration for installed Spanish XKB + Compose.
# Synthetic opt-in mode is scoped to this disposable QA process; no user
# input mode is modified or persisted. The default X11 keymap remains US,
# so apostrophe is the physical ANSI key mapped to Spanish dead acute.
kill "$app_pid" 2>/dev/null || true
wait "$app_pid" 2>/dev/null || true
app_pid=""
lua5.4 tests/smoke_assert.lua prepare-xkb-compose "$XDG_CONFIG_HOME/typefinch/settings.json"
rm -f "$tmp/report.json"
TYPEFINCH_QA_XKB_COMPOSE=1 ./target/debug/typefinch >"$tmp/app-xkb-compose.log" 2>&1 &
app_pid=$!
window=""
for _ in $(seq 1 200); do
  window="$(xdotool search --onlyvisible --name 'Typefinch' 2>/dev/null | head -n 1 || true)"
  [[ -n "$window" ]] && break
  if ! kill -0 "$app_pid" 2>/dev/null; then cat "$tmp/app-xkb-compose.log"; exit 1; fi
  sleep 0.25
done
[[ -n "$window" ]] || { echo "No XKB Compose test window" >&2; exit 1; }
xdotool windowactivate --sync "$window" 2>/dev/null || true
xdotool windowfocus --sync "$window"
sleep 0.4
xdotool windowactivate --sync "$window" 2>/dev/null || true
xdotool windowfocus --sync "$window"
sleep 0.3
xdotool key --clearmodifiers F5
sleep 0.3
xdotool type --clearmodifiers --delay 40 "Con pr"
xdotool key --clearmodifiers apostrophe
xdotool type --clearmodifiers --delay 40 "actica"
sleep 2.4
lua5.4 tests/smoke_assert.lua xkb-compose "$tmp/report.json"

# Responsive screenshots from the actual graphical binary, not mocks.
# Retain captures as CI artifacts for visual inspection; dimensions alone do
# not establish that every text label is unclipped.
xdotool windowsize "$window" 740 590
sleep 0.6
import -window "$window" "$tmp/typefinch-narrow.png"
[[ -s "$tmp/typefinch-narrow.png" ]] || { echo "Missing narrow viewport screenshot"; exit 1; }
identify -format 'Narrow native viewport: %wx%h pixels\n' "$tmp/typefinch-narrow.png"

kill "$app_pid" 2>/dev/null || true
wait "$app_pid" 2>/dev/null || true
app_pid=""
# Winit's X11 DPI override provides an isolated 150% environment for visual QA;
# the screenshot itself is still a physical-pixel recording.
WINIT_X11_SCALE_FACTOR=1.5 ./target/debug/typefinch >"$tmp/app-hidpi.log" 2>&1 &
app_pid=$!
window=""
for _ in $(seq 1 200); do
  window="$(xdotool search --onlyvisible --name 'Typefinch' 2>/dev/null | head -n 1 || true)"
  [[ -n "$window" ]] && break
  if ! kill -0 "$app_pid" 2>/dev/null; then cat "$tmp/app-hidpi.log"; exit 1; fi
  sleep 0.25
done
[[ -n "$window" ]] || { echo "No HiDPI window"; exit 1; }
xdotool windowactivate --sync "$window" 2>/dev/null || true
xdotool windowfocus --sync "$window"
sleep 0.6
import -window "$window" "$tmp/typefinch-hidpi.png"
[[ -s "$tmp/typefinch-hidpi.png" ]] || { echo "Missing 150% scaled viewport screenshot"; exit 1; }
identify -format '150% X11 DPI override viewport: %wx%h pixels\n' "$tmp/typefinch-hidpi.png"


# Existing corrupt settings must never be silently replaced by a new default
# profile. A native X11 session must refuse to persist scored input or backups.
kill "$app_pid" 2>/dev/null || true
wait "$app_pid" 2>/dev/null || true
app_pid=""
recover_root="$tmp/recovery-config"
mkdir -p "$recover_root/typefinch"
printf '%s' '{"incomplete":' >"$recover_root/typefinch/settings.json"
cp "$recover_root/typefinch/settings.json" "$tmp/original-invalid-settings.json"
rm -f "$tmp/recovery-report.json"
XDG_CONFIG_HOME="$recover_root" TYPEFINCH_QA_REPORT="$tmp/recovery-report.json" \
  ./target/debug/typefinch >"$tmp/app-recovery.log" 2>&1 &
app_pid=$!
window=""
for _ in $(seq 1 200); do
  window="$(xdotool search --onlyvisible --name 'Typefinch' 2>/dev/null | head -n 1 || true)"
  [[ -n "$window" ]] && break
  if ! kill -0 "$app_pid" 2>/dev/null; then cat "$tmp/app-recovery.log"; exit 1; fi
  sleep 0.25
done
[[ -n "$window" ]] || { echo "No guarded-settings window"; exit 1; }
xdotool windowactivate --sync "$window" 2>/dev/null || true
xdotool windowfocus --sync "$window"
sleep 0.4
import -window "$window" "$tmp/typefinch-recovery.png"
# Image capture can disturb the X11 focus; prove the guarded session is
# actually started instead of accepting a false-positive empty database.
xdotool windowactivate --sync "$window" 2>/dev/null || true
xdotool windowfocus --sync "$window"
sleep 0.3
xdotool key --clearmodifiers F5
sleep 0.3
xdotool type --clearmodifiers --delay 35 "little finches practice"
sleep 2.5
xdotool key --clearmodifiers F8
sleep 0.5
lua5.4 tests/smoke_assert.lua recovery \
  "$recover_root/typefinch/settings.json" "$tmp/original-invalid-settings.json" \
  "$recover_root/typefinch/sessions.sqlite3" "$tmp/recovery-report.json"
if compgen -G "$recover_root/typefinch/exports/typefinch-backup-*" >/dev/null; then
  echo "Corrupt-settings fallback must not create a misleading full backup"
  exit 1
fi

# A missing settings.json is *not* a new profile when an older SQLite file
# survives. Verify using the native GUI, not only isolated Rust unit tests.
kill "$app_pid" 2>/dev/null || true
wait "$app_pid" 2>/dev/null || true
app_pid=""
orphan_root="$tmp/orphan-config"
mkdir -p "$orphan_root/typefinch"
cp "$backup_db" "$orphan_root/typefinch/sessions.sqlite3"
cp "$orphan_root/typefinch/sessions.sqlite3" "$tmp/orphan-original.sqlite3"
XDG_CONFIG_HOME="$orphan_root" TYPEFINCH_QA_REPORT="$tmp/orphan-report.json" \
  ./target/debug/typefinch >"$tmp/app-orphan.log" 2>&1 &
app_pid=$!
window=""
for _ in $(seq 1 200); do
  window="$(xdotool search --onlyvisible --name 'Typefinch' 2>/dev/null | head -n 1 || true)"
  [[ -n "$window" ]] && break
  if ! kill -0 "$app_pid" 2>/dev/null; then cat "$tmp/app-orphan.log"; exit 1; fi
  sleep 0.25
done
[[ -n "$window" ]] || { echo "No orphan-history recovery window"; exit 1; }
xdotool windowactivate --sync "$window" 2>/dev/null || true
xdotool windowfocus --sync "$window"
sleep 0.4
xdotool key --clearmodifiers F5
sleep 0.3
xdotool type --clearmodifiers --delay 35 "little finches practice"
sleep 2.5
xdotool key --clearmodifiers F8
sleep 0.3
lua5.4 tests/smoke_assert.lua orphan "$tmp/orphan-report.json"
[[ ! -e "$orphan_root/typefinch/settings.json" ]] || { echo "Recreated missing identity!"; exit 1; }
cmp "$orphan_root/typefinch/sessions.sqlite3" "$tmp/orphan-original.sqlite3"
if compgen -G "$orphan_root/typefinch/exports/typefinch-backup-*" >/dev/null; then
  echo "Orphan-history guard must not generate fallback profile backup"
  exit 1
fi

# Two real windows: a focused test must cancel as soon as another app
# takes foreground focus, never recording a score or paste.
kill "$app_pid" 2>/dev/null || true
wait "$app_pid" 2>/dev/null || true
app_pid=""
focus_root="$tmp/focus-primary"
secondary_root="$tmp/focus-secondary"
mkdir -p "$focus_root" "$secondary_root"
XDG_CONFIG_HOME="$focus_root" TYPEFINCH_QA_REPORT="$tmp/focus-report.json" \
  ./target/debug/typefinch >"$tmp/app-focus-primary.log" 2>&1 &
app_pid=$!
primary_window=""
for _ in $(seq 1 200); do
  primary_window="$(xdotool search --onlyvisible --name 'Typefinch' 2>/dev/null | head -n 1 || true)"
  [[ -n "$primary_window" ]] && break
  if ! kill -0 "$app_pid" 2>/dev/null; then cat "$tmp/app-focus-primary.log"; exit 1; fi
  sleep 0.25
done
[[ -n "$primary_window" ]] || { echo "No focus-test primary window"; exit 1; }
XDG_CONFIG_HOME="$secondary_root" ./target/debug/typefinch >"$tmp/app-focus-secondary.log" 2>&1 &
secondary_pid=$!
secondary_window=""
for _ in $(seq 1 200); do
  secondary_window="$(xdotool search --onlyvisible --name 'Typefinch' 2>/dev/null | grep -vx "$primary_window" | head -n 1 || true)"
  [[ -n "$secondary_window" ]] && break
  if ! kill -0 "$secondary_pid" 2>/dev/null; then cat "$tmp/app-focus-secondary.log"; exit 1; fi
  sleep 0.25
done
[[ -n "$secondary_window" ]] || { echo "No focus-test competing window"; exit 1; }
xdotool windowactivate --sync "$primary_window" 2>/dev/null || true
xdotool windowfocus --sync "$primary_window"
sleep 0.3
xdotool key --clearmodifiers F5
sleep 0.2
xdotool type --clearmodifiers --delay 20 "lit"
xdotool windowactivate --sync "$secondary_window" 2>/dev/null || true
xdotool windowfocus --sync "$secondary_window"
sleep 0.8
XDG_CONFIG_HOME="$focus_root" lua5.4 tests/smoke_assert.lua focus-loss "$tmp/focus-report.json"
