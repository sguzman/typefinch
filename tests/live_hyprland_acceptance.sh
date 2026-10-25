#!/usr/bin/env bash
# Explicitly opted-in local QA, never CI-run against a real desktop.
set -Eeuo pipefail
if [[ "${1:-}" == --self-test ]];then
  command -v jq >/dev/null
  fixture='[{"pid":21,"at":[1,2],"size":[3,4]},{"pid":42,"at":[18,27],"size":[800,600]}]'
  rect="$(jq -nr --argjson rows "$fixture" --argjson pid 42 \
    '$rows | first(.[] | select(.pid==$pid) | "\(.at[0]),\(.at[1]) \(.size[0])x\(.size[1])")')"
  [[ "$rect" == "18,27 800x600" ]] || exit 1
  echo "Hyprland QA parser self-test PASS (no compositor accessed)"
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
# Disposable profile: never open the user's existing settings/database.
cargo build --locked >/dev/null
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
hyprctl dispatch sendshortcut ", F1, pid:$app" >/dev/null
sleep 0.8
grim -g "$rect" "$tmp/after.png"
[[ -s "$tmp/after.png" ]] || exit 1
pixels="$(compare -metric AE "$tmp/before.png" "$tmp/after.png" null: 2>&1 || true)"
[[ "$pixels" =~ ^[0-9]+$ && "$pixels" -gt 2000 ]] ||
  { echo "Real compositor did not display F1 guide: pixels=$pixels" >&2;exit 1; }
# Local opt-in screenshot only, in an explicit preexisting evidence directory.
if [[ -n "${TYPEFINCH_QA_OUTPUT_DIR:-}" ]];then
  out="$TYPEFINCH_QA_OUTPUT_DIR"
  [[ -d "$out" && ! -L "$out" && ! -e "$out/typefinch-hyprland-help.png" ]] ||
    { echo "Unsafe or occupied evidence destination" >&2;exit 1; }
  install -m 600 "$tmp/after.png" "$out/typefinch-hyprland-help.png"
fi
jq -n --argjson pixels "$pixels" '{state:"passed",compositor:"real-hyprland",
  profile:"disposable",input:"F1 compositor-dispatched to Typefinch only",
  physical_keyboard_verified:false,per_device_attribution_verified:false,
  changed_pixels:$pixels}'
