#!/usr/bin/env bash
# Explicitly opted-in local QA, never CI-run against a real desktop.
set -Eeuo pipefail
if [[ "${1:-}" == --self-test ]];then
  command -v jq >/dev/null
  fixture='[{"pid":21,"at":[1,2],"size":[3,4]},{"pid":42,"at":[18,27],"size":[800,600]}]'
  rect="$(jq -nr --argjson rows "$fixture" --argjson pid 42 \
    '$rows | first(.[] | select(.pid==$pid) | "\(.at[0]),\(.at[1]) \(.size[0])x\(.size[1])")')"
  [[ "$rect" == "18,27 800x600" ]] || exit 1
  # Test-only synthetic metadata: no compositor is contacted and no actual
  # device identifiers or typed material are ever embedded in the fixture.
  inventory='{"keyboards":[{"name":"synthetic"}]}'
  inspection='{"schema":1,"capture_policy":"inspection-only","input_attribution":"aggregate seat; no per-device key-event attribution","rows":[{}]}'
  jq -en --argjson d "$inspection" '$d.schema==1 and
    $d.capture_policy=="inspection-only" and
    $d.input_attribution=="aggregate seat; no per-device key-event attribution"
    and ($d.rows|type=="array")' >/dev/null
  [[ "$(jq -n --argjson d "$inventory" '$d.keyboards|length')" ==
     "$(jq -n --argjson d "$inspection" '$d.rows|length')" ]] || exit 1
  wrong='{"schema":1,"capture_policy":"global-evdev","input_attribution":"physical","rows":[{}]}'
  if jq -en --argjson d "$wrong" '$d.capture_policy=="inspection-only"
    and $d.input_attribution=="aggregate seat; no per-device key-event attribution"' >/dev/null;then
    echo "Unsafe global-capture claim accepted by QA validator" >&2
    exit 1
  fi
  echo "Hyprland QA metadata and rectangle self-tests PASS (no compositor accessed)"
  exit 0
fi
[[ "$#" -eq 0 ]] || exit 2
[[ "${TYPEFINCH_QA_CONSENT:-}" == 1 ]] || { echo "Explicit QA opt-in required" >&2;exit 2; }
[[ -n "${HYPRLAND_INSTANCE_SIGNATURE:-}" ]] || { echo "Live Hyprland required" >&2;exit 2; }
for cmd in cargo hyprctl jq grim compare;do
  command -v "$cmd" >/dev/null || { echo "Missing QA dependency: $cmd" >&2;exit 2; }
done
[[ -f Cargo.toml && -f rust-toolchain.toml ]] || { echo "Run from Typefinch root" >&2;exit 2; }
tmp="$(mktemp -d)"
app='' previous='' workspace=''
cleanup(){
  result=$?
  set +e
  [[ -z "$app" ]] || { kill "$app" 2>/dev/null; wait "$app" 2>/dev/null; }
  if [[ -n "$previous" && "$previous" != null ]];then
    hyprctl dispatch focuswindow "pid:$previous" >/dev/null 2>&1
  elif [[ -n "$workspace" && "$workspace" != null ]];then
    hyprctl dispatch workspace "$workspace" >/dev/null 2>&1
  fi
  rm -rf -- "$tmp"
  exit "$result"
}
trap cleanup EXIT
previous="$(hyprctl -j activewindow | jq -r '.pid // empty')"
workspace="$(hyprctl -j activeworkspace | jq -r '.id // empty')"
mkdir -m 700 "$tmp/config" "$tmp/data" "$tmp/cache"
export XDG_CONFIG_HOME="$tmp/config" XDG_DATA_HOME="$tmp/data" XDG_CACHE_HOME="$tmp/cache"
export XDG_SESSION_TYPE=wayland WINIT_UNIX_BACKEND=wayland
# Ephemeral structured F4/F10 audit; never save characters or hardware IDs.
export TYPEFINCH_QA_REPORT="$tmp/geometry.json" TYPEFINCH_QA_PREVIEW_SHAPE=1
# Disposable profile: never open the user's existing settings/database.
cargo build --locked >/dev/null
# Real, read-only compositor inventory parity. Keep names and opaque addresses
# exclusively in the throwaway QA directory; report counts only.
./target/debug/typefinch --inspect-keyboards >"$tmp/typefinch-devices.json"
hyprctl -j devices >"$tmp/hyprland-devices.json"
jq -e '.schema==1 and .capture_policy=="inspection-only" and
  .input_attribution=="aggregate seat; no per-device key-event attribution"
  and (.rows | type=="array")' "$tmp/typefinch-devices.json" >/dev/null ||
  { echo "Typefinch live keyboard inspection is not read-only" >&2;exit 1; }
observed="$(jq -r '.rows | length' "$tmp/typefinch-devices.json")"
connected="$(jq -r '.keyboards | length' "$tmp/hyprland-devices.json")"
[[ "$observed" =~ ^[0-9]+$ && "$connected" =~ ^[0-9]+$ &&
   "$connected" -gt 0 && "$observed" == "$connected" ]] ||
  { echo "Actual Hyprland keyboard inventory parity failed" >&2;exit 1; }
./target/debug/typefinch >"$tmp/app.log" 2>&1 &
app=$!
found=0
for _ in $(seq 1 100);do
  kill -0 "$app" 2>/dev/null || { echo "QA app exited" >&2;exit 1; }
  if hyprctl -j clients | jq -e --argjson pid "$app" 'any(.[]; .pid==$pid and .mapped==true)' >/dev/null;then
    found=1;break
  fi
  sleep 0.2
done
((found==1)) || { echo "Typefinch did not open in Hyprland" >&2;exit 1; }
hyprctl dispatch focuswindow "pid:$app" >/dev/null
for _ in $(seq 1 30);do
  [[ "$(hyprctl -j activewindow | jq -r '.pid // empty')" == "$app" ]] && break
  sleep 0.2
done
[[ "$(hyprctl -j activewindow | jq -r '.pid // empty')" == "$app" ]] ||
  { echo "QA window never gained focus" >&2;exit 1; }
clients="$(hyprctl -j clients)"
rect="$(jq -nr --argjson rows "$clients" --argjson pid "$app" \
  '$rows | first(.[] | select(.pid==$pid) | "\(.at[0]),\(.at[1]) \(.size[0])x\(.size[1])")')"
[[ "$rect" =~ ^-?[0-9]+,-?[0-9]+\ [1-9][0-9]*x[1-9][0-9]*$ ]] ||
  { echo "Invalid foreground window geometry" >&2;exit 1; }
grim -g "$rect" "$tmp/before.png"
[[ -s "$tmp/before.png" ]] || exit 1
# Events target only the Typefinch foreground window.
send_key(){
  [[ "$(hyprctl -j activewindow | jq -r '.pid // empty')" == "$app" ]] ||
    { echo "Foreground QA focus lost; refusing input" >&2;exit 1; }
  hyprctl dispatch sendshortcut "$1, pid:$app" >/dev/null
  sleep 0.45
}
snapshot(){
  grim -g "$rect" "$tmp/$1.png"
  [[ -s "$tmp/$1.png" ]] || { echo "Missing $1 native screenshot" >&2;exit 1; }
}
pixel_delta(){
  local changed
  changed="$(compare -metric AE "$tmp/$1.png" "$tmp/$2.png" null: 2>&1 || true)"
  [[ "$changed" =~ ^[0-9]+$ && "$changed" -gt "$3" ]] ||
    { echo "No significant $1-to-$2 GUI change ($changed pixels)" >&2;exit 1; }
  echo "$changed"
}
geometry(){
  [[ -s "$TYPEFINCH_QA_REPORT" ]] ||
    { echo "No F4/F10 geometry report from actual GUI" >&2;exit 1; }
  jq -e --arg shape "$1" --arg section "$2" \
    '.synthetic==true and .state=="geometry-preview" and
     .shape==$shape and .section==$section and .slots>0' \
    "$TYPEFINCH_QA_REPORT" >/dev/null ||
    { echo "Incorrect F4/F10 preview state" >&2;exit 1; }
  rm -f "$TYPEFINCH_QA_REPORT"
}
theme(){
  local expected="$1"
  for _ in $(seq 1 20);do
    [[ -f "$XDG_CONFIG_HOME/typefinch/settings.json" ]] &&
      jq -e --arg expected "$expected" '.theme==$expected' \
        "$XDG_CONFIG_HOME/typefinch/settings.json" >/dev/null 2>&1 && return 0
    sleep 0.1
  done
  echo "Theme shortcut did not persist $expected in disposable settings" >&2
  exit 1
}
# F1: actual native Wayland focused help overlay.
send_key ", F1"
snapshot help
pixels="$(pixel_delta before help 2000)"
send_key ", F1"
# F4/F10: geometry reports from production app plus window-only captures.
send_key ", F4"
geometry "ISO main block" "Typing block"
snapshot iso
send_key ", F4"
geometry "JIS main block" "Typing block"
snapshot jis
send_key ", F10"
geometry "JIS main block" "Function / navigation"
snapshot navigation
send_key ", F10"
geometry "JIS main block" "Numeric keypad"
snapshot keypad
# App-owned System → Light → Dark → System, not an OS theme transition.
send_key "CTRL SHIFT, T"
theme light
snapshot light
send_key "CTRL SHIFT, T"
theme dark
snapshot dark
theme_pixels="$(pixel_delta light dark 2000)"
send_key "CTRL SHIFT, T"
theme system
# Opt-in local evidence only. Preflight all targets; never overwrite.
if [[ -n "${TYPEFINCH_QA_OUTPUT_DIR:-}" ]];then
  out="$TYPEFINCH_QA_OUTPUT_DIR"
  [[ -d "$out" && ! -L "$out" ]] ||
    { echo "Invalid local QA evidence destination" >&2;exit 1; }
  for name in help iso jis navigation keypad light dark;do
    [[ ! -e "$out/typefinch-hyprland-$name.png" &&
       ! -L "$out/typefinch-hyprland-$name.png" ]] ||
      { echo "Refusing to overwrite existing QA evidence" >&2;exit 1; }
  done
  for name in help iso jis navigation keypad light dark;do
    install -m 600 "$tmp/$name.png" "$out/typefinch-hyprland-$name.png"
  done
fi
jq -n --argjson help_pixels "$pixels" --argjson theme_pixels "$theme_pixels" \
  --argjson connected "$connected" \
  '{state:"passed",compositor:"real-hyprland",profile:"disposable",
    actual_keyboard_inspection_rows:$connected,
    input:"targeted compositor shortcuts F1 F4 F10 Ctrl+Shift+T",
    help_changed_pixels:$help_pixels,light_dark_changed_pixels:$theme_pixels,
    illustrated_geometry:["ISO","JIS","navigation","keypad"],
    app_theme_cycle:["system","light","dark","system"],
    system_theme_transition_verified:false,
    physical_keyboard_verified:false,per_device_attribution_verified:false}'
