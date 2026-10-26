#!/usr/bin/env bash
# Offline STRUCTURAL verification of private live-Hyprland QA evidence.
# Does not contact a compositor or GitHub, prove physical-device attribution,
# perform visual inspection, or alter the project acceptance ledger.
set -Eeuo pipefail
refuse(){ echo "Hyprland evidence refused: $1" >&2;return 1; }
verify_bundle(){
  local dir="$1" receipt mode count name img width height format dims="" sha expected
  [[ -d "$dir" && ! -L "$dir" ]] || { refuse "not a private directory";return 1; }
  mode="$(stat -c '%a' -- "$dir")" || return 1
  [[ "$mode" =~ ^[0-7]{3,4}$ ]] || { refuse "ambiguous permissions";return 1; }
  (( (8#$mode & 0077)==0 )) || { refuse "directory is accessible to other users";return 1; }
  receipt="$dir/typefinch-hyprland-receipt.json"
  [[ -f "$receipt" && ! -L "$receipt" && "$(stat -c '%a' -- "$receipt")" == 600 ]] ||
    { refuse "private receipt missing";return 1; }
  jq -e '
    type=="object" and (keys|sort)==([
      "schema","kind","state","compositor","source_commit","clean_source_tree",
      "keyboard_inspection","connected_keyboard_count","focused_os_input_verified",
      "window_only_capture","help_changed_pixels","light_dark_changed_pixels",
      "illustrated_geometry","theme_modes","checks",
      "system_theme_transition_verified","physical_keyboard_routing_verified",
      "per_device_attribution_verified","screenshots"
    ]|sort) and
    .schema==1 and .kind=="typefinch-local-hyprland-qa" and
    .state=="passed" and .compositor=="real-hyprland" and
    (.source_commit|type=="string" and test("^[0-9a-f]{40}$")) and
    .clean_source_tree==true and .keyboard_inspection=="read-only" and
    (.connected_keyboard_count|type=="number" and .>0 and .<=128 and .==floor) and
    .focused_os_input_verified==true and .window_only_capture==true and
    (.help_changed_pixels|type=="number" and .>2000 and .==floor) and
    (.light_dark_changed_pixels|type=="number" and .>2000 and .==floor) and
    .illustrated_geometry==["ISO","JIS","navigation","keypad"] and
    .theme_modes==["system","light","dark","system"] and
    .system_theme_transition_verified==false and
    .physical_keyboard_routing_verified==false and
    .per_device_attribution_verified==false and
    (.checks|type=="object" and (keys|sort)==([
       "T001_live_window_screenshot","T401_live_connected_keyboard_inspection",
       "T902_targeted_keyboard_input","T902_theme_layout_screenshots"
     ]|sort) and all(.[]; .==true)) and
    (.screenshots|type=="object" and (keys|sort)==
       (["help","iso","jis","navigation","keypad","light","dark"]|sort) and
       all(.[]; type=="string" and test("^[a-f0-9]{64}$")))
  ' "$receipt" >/dev/null 2>&1 ||
    { refuse "invalid, overclaiming, or nonredacted receipt";return 1; }
  count="$(find "$dir" -mindepth 1 -maxdepth 1 | wc -l)"
  [[ "$count" -eq 8 ]] || { refuse "unexpected or missing files";return 1; }
  for name in help iso jis navigation keypad light dark;do
    img="$dir/typefinch-hyprland-$name.png"
    [[ -f "$img" && ! -L "$img" && "$(stat -c '%a' -- "$img")" == 600 ]] ||
      { refuse "screenshot missing or unsafe";return 1; }
    [[ "$(stat -c '%s' -- "$img")" -gt 100 &&
       "$(stat -c '%s' -- "$img")" -le 33554432 ]] ||
      { refuse "invalid image size";return 1; }
    read -r format width height <<<"$(identify -ping -format '%m %w %h' "$img" 2>/dev/null)" ||
      { refuse "unreadable image";return 1; }
    [[ "$format" == PNG && "$width" =~ ^[0-9]+$ && "$height" =~ ^[0-9]+$ ]] ||
      { refuse "not a single PNG";return 1; }
    (( width>=400 && width<=8192 && height>=300 && height<=8192 )) ||
      { refuse "invalid screenshot dimensions";return 1; }
    if [[ -n "$dims" && "$dims" != "$width $height" ]];then
      refuse "window dimensions changed";return 1
    fi
    dims="$width $height"
    sha="$(sha256sum -- "$img" | cut -d ' ' -f1)"
    expected="$(jq -er --arg key "$name" '.screenshots[$key]' "$receipt")" ||
      { refuse "missing digest";return 1; }
    [[ "$sha" == "$expected" ]] ||
      { refuse "image was changed after the run";return 1; }
  done
  echo "PRIVATE_HYPRLAND_EVIDENCE_STRUCTURALLY_VALID"
  jq -c '{source_commit,connected_keyboard_count,checks,
    unverified:{system_theme_transition:.system_theme_transition_verified,
      physical_key_routing:.physical_keyboard_routing_verified,
      per_device_attribution:.per_device_attribution_verified},
    review_required:true
  }' "$receipt"
}
[[ "$#" == 1 ]] || { echo "Usage: bash tests/verify_hyprland_evidence.sh PRIVATE_BUNDLE_DIRECTORY" >&2;exit 2; }
verify_bundle "$1"
