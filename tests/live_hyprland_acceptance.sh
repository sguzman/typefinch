#!/usr/bin/env bash
# Explicitly opted-in local QA, never CI-run against a real desktop.
set -Eeuo pipefail
# Only capture or send QA shortcuts while the compositor reports an
# unlocked session. A focused Typefinch window can remain mapped behind a
# lock screen; window/overlap checks alone do not rule out lock overlays.
# Hyprland 0.55's read-only 'hyprctl -j locked' returns {"locked":bool}.
# Unsupported, malformed, unavailable or ambiguous results fail CLOSED.
session_is_unlocked(){
  # Slurp exactly one JSON document; ordinary jq -e could accept a second
  # "unlocked" object after an earlier contradictory locked record.
  jq -es 'length == 1 and (.[0] | type) == "object" and .[0].locked == false' \
    "$1" >/dev/null 2>&1
}
# Compare the read-only inventory itself, not only its count.
inventory_parity(){
  jq -en --slurpfile own "$1" --slurpfile host "$2" '
    def reported: if type=="string" and length>0 then . else null end;
    ($own[0].rows // null) as $rows |
    ($host[0].keyboards // null) as $keys |
    if (($rows|type)!="array" or ($keys|type)!="array") then false else
      ($rows|length) as $n |
      ($n>0) and ($n<=128) and (($keys|length)==$n) and
      (([$rows[]|.inspection_id]|unique|length)==$n) and
      (([$keys[]|.address]|unique|length)==$n) and
      (([$rows[] | {
        id:.inspection_id, name:.label, keymap:.reported_active_keymap,
        layout:.configured_layout, variant:.configured_variant
      }] | sort_by(.id)) ==
      ([$keys[] | {
        id:.address,
        name:(if (.name|type)=="string" and (.name|length)>0
          then .name else "Unlabeled keyboard" end),
        keymap:(.active_keymap|reported),
        layout:(.layout|reported),
        variant:(.variant|reported)
      }] | sort_by(.id)))
    end
  ' >/dev/null
}
# A connected keyboard unplugged, replaced or remapped halfway through
# native GUI QA must invalidate the starting inventory evidence. Compare
# normalized rows in local disposable files; ordering is not identity.
inventory_stable(){
  jq -en --slurpfile before "$1" --slurpfile after "$2" '
    def canonical: [.rows[] | {
      inspection_id,label,reported_active_keymap,
      configured_layout,configured_variant
    }] | sort_by(.inspection_id);
    ($before[0] // null) as $first |
    ($after[0] // null) as $last |
    if (($first.rows|type)!="array" or ($last.rows|type)!="array")
    then false
    else (
      $first.schema==1 and $last.schema==1 and
      $first.capture_policy=="inspection-only" and
      $last.capture_policy=="inspection-only" and
      $first.input_attribution=="aggregate seat; no per-device key-event attribution" and
      $last.input_attribution=="aggregate seat; no per-device key-event attribution" and
      ($first.rows|length)>0 and
      all(($first.rows + $last.rows)[];(
        has("key_events")|not) and (has("physical_device")|not) and
        (has("raw_keys")|not)) and
      ($first|canonical)==($last|canonical)
    )
    end
  ' >/dev/null
}
# Fail closed when a visible same-workspace Hyprland client intersects the
# Typefinch capture rectangle. This does not detect layer-shell overlays.
window_is_clear(){
  jq -en --slurpfile data "$1" --argjson pid "$2" --argjson ws "$3" '
    def valid_rect:
      (.at|type)=="array" and (.size|type)=="array" and
      (.at|length)==2 and (.size|length)==2 and
      ([.at[],.size[]] | all(.[];type=="number" and .==floor)) and
      .size[0]>0 and .size[1]>0;
    ($data[0] // null) as $clients |
    if ($clients|type)!="array" then false else
      ([$clients[] | select(.pid==$pid and .mapped==true)]) as $apps |
      if ($apps|length)!=1 then false else
        ($apps[0]) as $target |
        ([$clients[] | select(.pid!=$pid and .mapped==true and .hidden!=true
          and .workspace.id==$ws)]) as $others |
        ($target.workspace.id==$ws and ($target|valid_rect) and
         (all($others[];valid_rect)) and
         ([$others[] | select(
            .at[0]<($target.at[0]+$target.size[0]) and
            (.at[0]+.size[0])>$target.at[0] and
            .at[1]<($target.at[1]+$target.size[1]) and
            (.at[1]+.size[1])>$target.at[1])]|length)==0)
      end
    end
  ' >/dev/null
}
# Require the current focused surface to retain the same on-screen
# rectangle and workspace as the screenshot captured at startup. A moved or
# resized window must never yield stale window-only screenshot evidence.
window_geometry_matches(){
  jq -en --slurpfile data "$1" --argjson pid "$2" --arg expected "$3" \
    --argjson workspace "$4" '
    ($data[0] // null) as $window |
    if ($window|type)!="object" then false else
      ($window.pid==$pid and $window.mapped==true and
       $window.workspace.id==$workspace and
       ($window.at|type)=="array" and
       ($window.size|type)=="array" and
       ($window.at|length)==2 and ($window.size|length)==2 and
       ([$window.at[], $window.size[]] | all(.[]; type=="number" and .==floor)) and
       $window.size[0]>0 and $window.size[1]>0 and
       ("\($window.at[0]),\($window.at[1]) \($window.size[0])x\($window.size[1])"==$expected))
    end
  ' >/dev/null
}
# Hyprland's Lua API uses structured dispatch expressions; older
# versions use the legacy dispatcher vocabulary. Only fixed QA arguments.
lua_focus_pid(){ printf 'hl.dsp.focus({ window = "pid:%s" })' "$1"; }
lua_focus_workspace(){ printf 'hl.dsp.focus({ workspace = "%s" })' "$1"; }
lua_shortcut(){
  printf 'hl.dsp.send_shortcut({ mods = "%s", key = "%s", window = "pid:%s" })' \
    "$1" "$2" "$3"
}
hypr_focus_pid(){
  [[ "$1" =~ ^[1-9][0-9]*$ ]] || return 2
  if [[ "$hypr_mode" == lua ]];then
    hyprctl dispatch "$(lua_focus_pid "$1")"
  else
    hyprctl dispatch focuswindow "pid:$1"
  fi
}
hypr_focus_workspace(){
  [[ "$1" =~ ^[1-9][0-9]*$ ]] || return 2
  if [[ "$hypr_mode" == lua ]];then
    hyprctl dispatch "$(lua_focus_workspace "$1")"
  else
    hyprctl dispatch workspace "$1"
  fi
}
# Do not steal focus from another application during QA cleanup. If the
# user has deliberately switched away, preserve their active foreground.
focus_is_ours(){ [[ "$1" =~ ^[1-9][0-9]*$ && "$1" == "$2" ]]; }
# Atomic no-overwrite publication, including when the source temp tree
# and explicitly chosen evidence directory are on different filesystems.
# mktemp creates a private file in the destination's own filesystem;
# the subsequent hardlink fails atomically for existing files or symlinks.
qa_stage=''
publish_evidence(){
  local src="$1" dst="$2" dir="$3"
  qa_stage="$(mktemp "$dir/.typefinch-qa-stage.XXXXXXXX")" || return 1
  if ! install -m 600 "$src" "$qa_stage";then
    rm -f -- "$qa_stage"; qa_stage='';return 1
  fi
  if ! ln -- "$qa_stage" "$dst";then
    rm -f -- "$qa_stage"; qa_stage='';return 1
  fi
  rm -f -- "$qa_stage"
  qa_stage=''
}
if [[ "${1:-}" == --self-test ]];then
  command -v jq >/dev/null
  [[ "$(lua_focus_pid 42)" == 'hl.dsp.focus({ window = "pid:42" })' ]] || exit 1
  [[ "$(lua_focus_workspace 3)" == 'hl.dsp.focus({ workspace = "3" })' ]] || exit 1
  [[ "$(lua_shortcut 'CTRL + SHIFT' T 42)" == 'hl.dsp.send_shortcut({ mods = "CTRL + SHIFT", key = "T", window = "pid:42" })' ]] || exit 1
  [[ "$(lua_shortcut '' F1 42)" == 'hl.dsp.send_shortcut({ mods = "", key = "F1", window = "pid:42" })' ]] || exit 1
  hypr_mode=lua
  if hypr_focus_pid 0 >/dev/null 2>&1;then echo 'Invalid target PID accepted' >&2;exit 1;fi
  focus_is_ours 42 42 || exit 1
  if focus_is_ours 42 21 || focus_is_ours 42 '' || focus_is_ours '' '';then
    echo 'Other foreground windows were incorrectly marked as QA-owned' >&2
    exit 1
  fi
  # Probe succeeds only for the actual no-op dispatcher, not merely an
  # installed Hyprland version or an available Lua REPL.
  hyprctl(){
    if [[ "$*" == "dispatch hl.dsp.no_op()" ]];then printf 'ok\n';return 0;fi
    printf 'error\n';return 1
  }
  probe="$(hyprctl dispatch 'hl.dsp.no_op()')" || exit 1
  [[ "$probe" == ok ]] || exit 1
  if hyprctl dispatch 'hl.dsp.focus({ workspace = "3" })' >/dev/null 2>&1;then
    echo "Fixture falsely allowed non-no-op dispatch" >&2;exit 1
  fi
  unset -f hyprctl
  fixture='[{"pid":21,"at":[1,2],"size":[3,4]},{"pid":42,"at":[18,27],"size":[800,600]}]'
  rect="$(jq -nr --argjson rows "$fixture" --argjson pid 42 \
    '$rows | first(.[] | select(.pid==$pid) | "\(.at[0]),\(.at[1]) \(.size[0])x\(.size[1])")')"
  [[ "$rect" == "18,27 800x600" ]] || exit 1
  # Test-only synthetic metadata: no compositor is contacted and no actual
  # device identifiers or typed material are ever embedded in the fixture.
  inventory='{"keyboards":[{"address":"sim:01","name":"Virtual Alpha","active_keymap":"English (US)","layout":"us","variant":""},{"address":"sim:02","name":"Virtual Beta","active_keymap":"German","layout":"de","variant":""}]}'
  inspection='{"schema":1,"capture_policy":"inspection-only","input_attribution":"aggregate seat; no per-device key-event attribution","rows":[{"inspection_id":"sim:02","label":"Virtual Beta","reported_active_keymap":"German","configured_layout":"de","configured_variant":null},{"inspection_id":"sim:01","label":"Virtual Alpha","reported_active_keymap":"English (US)","configured_layout":"us","configured_variant":null}]}'
  jq -en --argjson d "$inspection" '$d.schema==1 and
    $d.capture_policy=="inspection-only" and
    $d.input_attribution=="aggregate seat; no per-device key-event attribution"
    and ($d.rows|type=="array")' >/dev/null
  inventory_count="$(jq -n --argjson d "$inventory" '$d.keyboards|length')"
  inspection_count="$(jq -n --argjson d "$inspection" '$d.rows|length')"
  [[ "$inventory_count" == "$inspection_count" ]] || exit 1
  # Equal-count mismatches must not certify read-only device parity.
  fixture_dir="$(mktemp -d)"
  trap 'rm -rf -- "$fixture_dir"' EXIT
  # Session lock state is a hard safety gate, not a screenshot hint.
  printf '%s' '{"locked":false}' >"$fixture_dir/unlocked.json"
  session_is_unlocked "$fixture_dir/unlocked.json" || {
    echo "Unlocked compositor status rejected" >&2;exit 1;
  }
  for payload in '{"locked":true}' '{}' '{"locked":"false"}' 'null' '[]' 'not-json';do
    printf '%s' "$payload" >"$fixture_dir/unsafe-lock.json"
    if session_is_unlocked "$fixture_dir/unsafe-lock.json";then
      echo "Unsafe/ambiguous lock status enabled desktop capture" >&2;exit 1
    fi
  done
  printf '%s\n' '{"locked":true}' '{"locked":false}' >"$fixture_dir/unsafe-lock.json"
  if session_is_unlocked "$fixture_dir/unsafe-lock.json";then
    echo "Multiple contradictory compositor lock records accepted" >&2;exit 1
  fi
  printf '%s' "$inventory" >"$fixture_dir/hyprland.json"
  printf '%s' "$inspection" >"$fixture_dir/typefinch.json"
  inventory_parity "$fixture_dir/typefinch.json" "$fixture_dir/hyprland.json" || exit 1
  jq '.keyboards |= reverse' "$fixture_dir/hyprland.json" >"$fixture_dir/reordered.json"
  inventory_parity "$fixture_dir/typefinch.json" "$fixture_dir/reordered.json" || exit 1
  jq '.rows |= reverse' "$fixture_dir/typefinch.json" >"$fixture_dir/inspector-reordered.json"
  inventory_stable "$fixture_dir/typefinch.json" "$fixture_dir/inspector-reordered.json" || exit 1
  jq '.rows[0].configured_layout="changed"' "$fixture_dir/typefinch.json" >"$fixture_dir/inspector-drift.json"
  if inventory_stable "$fixture_dir/typefinch.json" "$fixture_dir/inspector-drift.json";then
    echo "Keyboard layout changed mid-QA without invalidating evidence" >&2;exit 1
  fi
  jq '.capture_policy="global-evdev"' "$fixture_dir/typefinch.json" >"$fixture_dir/unsafe-policy.json"
  if inventory_stable "$fixture_dir/typefinch.json" "$fixture_dir/unsafe-policy.json";then
    echo "Unsafe final input capture policy accepted as inspection-only" >&2;exit 1
  fi
  jq '.rows[0].physical_device="claimed"' "$fixture_dir/typefinch.json" >"$fixture_dir/unsafe-provenance.json"
  if inventory_stable "$fixture_dir/typefinch.json" "$fixture_dir/unsafe-provenance.json";then
    echo "Unverified physical-device field accepted in metadata-only QA" >&2;exit 1
  fi
  for change in '.keyboards[0].address="sim:99"' \
    '.keyboards[0].active_keymap="German"' \
    '.keyboards[1].address="sim:01"' \
    '.keyboards[0].name="A different keyboard"';do
    jq "$change" "$fixture_dir/hyprland.json" >"$fixture_dir/mismatch.json"
    if inventory_parity "$fixture_dir/typefinch.json" "$fixture_dir/mismatch.json";then
      echo "Equal-count but mismatched inventory passed QA" >&2
      exit 1
    fi
  done
  # Verify actual no-clobber semantics (including a dangling symlink)
  # using text-only fixtures. No screenshot or compositor access occurs.
  printf '%s' 'a' >"$fixture_dir/evidence-source"
  publish_evidence "$fixture_dir/evidence-source" "$fixture_dir/result" "$fixture_dir" || exit 1
  [[ "$(cat "$fixture_dir/result")" == a ]] || exit 1
  printf '%s' 'b' >"$fixture_dir/evidence-source"
  if publish_evidence "$fixture_dir/evidence-source" "$fixture_dir/result" "$fixture_dir";then
    echo "Existing QA artifact overwritten" >&2;exit 1
  fi
  [[ "$(cat "$fixture_dir/result")" == a ]] || exit 1
  ln -s "$fixture_dir/absent" "$fixture_dir/symlink-result"
  if publish_evidence "$fixture_dir/evidence-source" "$fixture_dir/symlink-result" "$fixture_dir";then
    echo "QA artifact destination symlink overwritten" >&2;exit 1
  fi
  [[ -L "$fixture_dir/symlink-result" && ! -e "$fixture_dir/absent" ]] || exit 1
  printf '%s' '{"pid":42,"mapped":true,"workspace":{"id":3},"at":[18,27],"size":[800,600]}' >"$fixture_dir/active.json"
  window_geometry_matches "$fixture_dir/active.json" 42 '18,27 800x600' 3 || exit 1
  for mutation in '.pid=21' '.workspace.id=4' '.at[0]=19' '.size[0]=799' '.mapped=false';do
    jq "$mutation" "$fixture_dir/active.json" >"$fixture_dir/moved.json"
    if window_geometry_matches "$fixture_dir/moved.json" 42 '18,27 800x600' 3;then
      echo "Changed window geometry or identity passed screenshot guard" >&2;exit 1
    fi
  done
  # Unrelated workspaces cannot obscure Typefinch; same-workspace overlap
  # must fail even when the native compositor screenshot tool succeeds.
  printf '%s' '[{"pid":42,"mapped":true,"hidden":false,"workspace":{"id":3},"at":[18,27],"size":[800,600]},{"pid":21,"mapped":true,"hidden":false,"workspace":{"id":4},"at":[25,35],"size":[80,80]}]' >"$fixture_dir/clients.json"
  window_is_clear "$fixture_dir/clients.json" 42 3 || exit 1
  jq '.[1].workspace.id=3' "$fixture_dir/clients.json" >"$fixture_dir/occluded.json"
  if window_is_clear "$fixture_dir/occluded.json" 42 3;then
    echo "Overlapping GUI accepted as window-only evidence" >&2;exit 1
  fi
  jq '.[1].at=[900,900] | .[1].workspace.id=3' \
    "$fixture_dir/clients.json" >"$fixture_dir/nonoverlap.json"
  window_is_clear "$fixture_dir/nonoverlap.json" 42 3 || exit 1
  jq '.[1].workspace.id=3 | .[1].at=null' \
    "$fixture_dir/clients.json" >"$fixture_dir/uninspectable.json"
  if window_is_clear "$fixture_dir/uninspectable.json" 42 3;then
    echo "Unreadable competing window rectangle passed QA" >&2;exit 1
  fi
  jq '.[0].workspace.id=4' "$fixture_dir/clients.json" >"$fixture_dir/target-moved.json"
  if window_is_clear "$fixture_dir/target-moved.json" 42 3;then
    echo "QA target on another workspace passed capture guard" >&2;exit 1
  fi
  jq '.[1].pid=42' "$fixture_dir/clients.json" >"$fixture_dir/ambiguous.json"
  if window_is_clear "$fixture_dir/ambiguous.json" 42 3;then
    echo "Ambiguous target window accepted for screenshot" >&2;exit 1
  fi
  input_meta='{"state":"focused-input","synthetic":true,"input_mode":"OS","attempts":1,"entered":1}'
  jq -en --argjson x "$input_meta" '$x.state=="focused-input"
    and $x.synthetic and $x.input_mode=="OS" and $x.attempts==1
    and $x.entered==1 and ($x|has("text")|not) and ($x|has("key")|not)' >/dev/null
  wrong='{"schema":1,"capture_policy":"global-evdev","input_attribution":"physical","rows":[{}]}'
  if jq -en --argjson d "$wrong" '$d.capture_policy=="inspection-only"
    and $d.input_attribution=="aggregate seat; no per-device key-event attribution"' >/dev/null;then
    echo "Unsafe global-capture claim accepted by QA validator" >&2
    exit 1
  fi
  echo "Hyprland QA identity/map stability, focused-input schema, consented-focus, no-clobber evidence, desktop-lock denial, fixed-geometry and screenshot-occlusion self-tests PASS (no compositor accessed)"
  exit 0
fi
[[ "$#" -eq 0 ]] || exit 2
[[ "${TYPEFINCH_QA_CONSENT:-}" == 1 ]] || { echo "Explicit QA opt-in required" >&2;exit 2; }
[[ -n "${HYPRLAND_INSTANCE_SIGNATURE:-}" ]] || { echo "Live Hyprland required" >&2;exit 2; }
for cmd in cargo hyprctl jq grim compare;do
  command -v "$cmd" >/dev/null || { echo "Missing QA dependency: $cmd" >&2;exit 2; }
done
[[ -f Cargo.toml && -f rust-toolchain.toml ]] || { echo "Run from Typefinch root" >&2;exit 2; }
# A no-op dispatcher probes the *active configuration's* Lua dispatch
# interface. Hyprland 0.55 can run Lua OR legacy configs, independently of
# compositor version. No keyboard, focus, workspace or settings are changed.
# A legacy fallback can only focus the disposable app's explicit PID;
# the script verifies focus again before any QA shortcut.
hypr_mode=legacy
if probe="$(hyprctl dispatch 'hl.dsp.no_op()' 2>/dev/null)" &&
   [[ "$probe" == ok ]];then
  hypr_mode=lua
fi
tmp="$(mktemp -d)"
app='' previous='' workspace=''
assert_session_unlocked(){
  hyprctl -j locked >"$tmp/session-lock.json" ||
    { echo "Hyprland lock state unavailable; refusing QA input/capture" >&2;exit 1; }
  session_is_unlocked "$tmp/session-lock.json" ||
    { echo "Locked or ambiguous desktop state; refusing QA input/capture" >&2;exit 1; }
}
cleanup(){
  result=$?
  set +e
  local foreground restore=0
  foreground="$(hyprctl -j activewindow 2>/dev/null | jq -r '.pid // empty' 2>/dev/null)"
  [[ -z "$qa_stage" ]] || rm -f -- "$qa_stage"
  [[ -z "$app" ]] || { kill "$app" 2>/dev/null; wait "$app" 2>/dev/null; }
  # A user may have switched to another window while automated QA was
  # running. Never seize their new focus when tearing down our own app.
  if focus_is_ours "$app" "$foreground";then
    if [[ "$previous" =~ ^[1-9][0-9]*$ ]] &&
      hypr_focus_pid "$previous" >/dev/null 2>&1 &&
      [[ "$(hyprctl -j activewindow 2>/dev/null | jq -r '.pid // empty' 2>/dev/null)" == "$previous" ]];then
      restore=1
    fi
    if ((restore==0)) && [[ "$workspace" =~ ^[1-9][0-9]*$ ]];then
      hypr_focus_workspace "$workspace" >/dev/null 2>&1
    fi
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
export TYPEFINCH_QA_INPUT_REPORT="$tmp/focused-input.json"
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
inventory_parity "$tmp/typefinch-devices.json" "$tmp/hyprland-devices.json" ||
  { echo "Actual Hyprland keyboard identities or keymaps diverged" >&2;exit 1; }
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
hypr_focus_pid "$app" >/dev/null
for _ in $(seq 1 30);do
  [[ "$(hyprctl -j activewindow | jq -r '.pid // empty')" == "$app" ]] && break
  sleep 0.2
done
[[ "$(hyprctl -j activewindow | jq -r '.pid // empty')" == "$app" ]] ||
  { echo "QA window never gained focus" >&2;exit 1; }
# Use the focused window's own geometry. Never pass other clients'
# names/titles in an argv argument while resolving the capture rectangle.
hyprctl -j activewindow >"$tmp/active-window.json"
rect="$(jq -r --argjson pid "$app" \
  'select(.pid==$pid) | "\(.at[0]),\(.at[1]) \(.size[0])x\(.size[1])"' \
  "$tmp/active-window.json")"
[[ "$rect" =~ ^-?[0-9]+,-?[0-9]+\ [1-9][0-9]*x[1-9][0-9]*$ ]] ||
  { echo "Invalid foreground window geometry" >&2;exit 1; }
# Refuse screenshots if focus changed or another visible client overlaps.
# Layer-shell overlays are not in hyprctl clients; evidence remains local.
assert_capture_clear(){
  assert_session_unlocked
  hyprctl -j activewindow >"$tmp/current-window.json"
  [[ "$(jq -r '.pid // empty' "$tmp/current-window.json")" == "$app" ]] ||
    { echo "Cannot capture unfocused Typefinch window" >&2;exit 1; }
  local active_ws
  active_ws="$(hyprctl -j activeworkspace | jq -r '.id // empty')"
  [[ "$active_ws" =~ ^-?[0-9]+$ ]] ||
    { echo "Hyprland workspace identity unavailable" >&2;exit 1; }
  window_geometry_matches "$tmp/current-window.json" "$app" "$rect" "$active_ws" ||
    { echo "Typefinch screenshot rectangle moved or became ambiguous" >&2;exit 1; }
  hyprctl -j clients >"$tmp/clients.json"
  window_is_clear "$tmp/clients.json" "$app" "$active_ws" ||
    { echo "Other window overlaps Typefinch; screenshot refused" >&2;exit 1; }
}
assert_capture_clear
grim -g "$rect" "$tmp/before.png"
[[ -s "$tmp/before.png" ]] || exit 1
assert_capture_clear
# Events target only the Typefinch foreground window.
send_key(){
  local mods="$1" key="$2" legacy_mods=''
  case "$mods:$key" in
    ':F1'|':F4'|':F10'|':F5'|':F2'|':A'|'CTRL + SHIFT:T') ;;
    *) echo "Non-QA shortcut refused" >&2;exit 2 ;;
  esac
  assert_session_unlocked
  [[ "$(hyprctl -j activewindow | jq -r '.pid // empty')" == "$app" ]] ||
    { echo "Foreground QA focus lost; refusing input" >&2;exit 1; }
  if [[ "$hypr_mode" == lua ]];then
    hyprctl dispatch "$(lua_shortcut "$mods" "$key" "$app")" >/dev/null
  else
    [[ "$mods" != 'CTRL + SHIFT' ]] || legacy_mods='CTRL SHIFT'
    hyprctl dispatch sendshortcut "$legacy_mods, $key, pid:$app" >/dev/null
  fi
  sleep 0.45
}
snapshot(){
  assert_capture_clear
  grim -g "$rect" "$tmp/$1.png"
  [[ -s "$tmp/$1.png" ]] || { echo "Missing $1 native screenshot" >&2;exit 1; }
  assert_capture_clear
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
send_key "" F1
snapshot help
pixels="$(pixel_delta before help 2000)"
send_key "" F1
# F4/F10: geometry reports from production app plus window-only captures.
send_key "" F4
geometry "ISO main block" "Typing block"
snapshot iso
send_key "" F4
geometry "JIS main block" "Typing block"
snapshot jis
send_key "" F10
geometry "JIS main block" "Function / navigation"
snapshot navigation
send_key "" F10
geometry "JIS main block" "Numeric keypad"
snapshot keypad
# App-owned System → Light → Dark → System, not an OS theme transition.
send_key "CTRL + SHIFT" T
theme light
snapshot light
send_key "CTRL + SHIFT" T
theme dark
snapshot dark
theme_pixels="$(pixel_delta light dark 2000)"
send_key "CTRL + SHIFT" T
theme system
# Native compositor target-only keyboard input: start an ordinary typing
# session and send exactly one letter to its focused typing widget. The
# app audits numeric attempts, not source text or physical device identity.
send_key "" F5
send_key "" F2
[[ ! -e "$TYPEFINCH_QA_INPUT_REPORT" ]] ||
  { echo "Focused input audit predated QA key event" >&2;exit 1; }
send_key "" A
received=0
for _ in $(seq 1 30);do
  if [[ -s "$TYPEFINCH_QA_INPUT_REPORT" ]] &&
     jq -e '.synthetic==true and .state=="focused-input" and
       .input_mode=="OS" and .attempts==1 and .entered==1' \
       "$TYPEFINCH_QA_INPUT_REPORT" >/dev/null 2>&1;then
    received=1;break
  fi
  sleep 0.15
done
((received==1)) ||
  { echo "Focused Wayland shortcut failed to count one typed attempt" >&2;exit 1; }
# The live keyboard inspector is read-only. Snapshot it again after all
# compositor shortcuts and reject any hotplug, layout or identity drift.
./target/debug/typefinch --inspect-keyboards >"$tmp/typefinch-devices-final.json"
hyprctl -j devices >"$tmp/hyprland-devices-final.json"
inventory_parity "$tmp/typefinch-devices-final.json" "$tmp/hyprland-devices-final.json" ||
  { echo "Final Hyprland keyboard inventory parity failed" >&2;exit 1; }
inventory_stable "$tmp/typefinch-devices.json" "$tmp/typefinch-devices-final.json" ||
  { echo "Keyboard inventory or active map changed during Hyprland QA" >&2;exit 1; }
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
  # No local screenshot files may be published after a lock transition.
  assert_session_unlocked
  for name in help iso jis navigation keypad light dark;do
    publish_evidence "$tmp/$name.png" "$out/typefinch-hyprland-$name.png" "$out" ||
      { echo "Cannot atomically publish local QA evidence without replacing a file" >&2;exit 1; }
  done
fi
jq -n --argjson help_pixels "$pixels" --argjson theme_pixels "$theme_pixels" \
  --argjson connected "$connected" \
  '{state:"passed",compositor:"real-hyprland",profile:"disposable",
    actual_keyboard_inspection_rows:$connected,
    input:"targeted compositor shortcuts F1 F4 F10 Ctrl+Shift+T F5 F2 A",
    synthetic_focused_typing_attempt_verified:true,
    help_changed_pixels:$help_pixels,light_dark_changed_pixels:$theme_pixels,
    illustrated_geometry:["ISO","JIS","navigation","keypad"],
    app_theme_cycle:["system","light","dark","system"],
    system_theme_transition_verified:false,
    physical_keyboard_verified:false,per_device_attribution_verified:false}'
