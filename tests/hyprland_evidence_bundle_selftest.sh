#!/usr/bin/env bash
# Completely synthetic private bundle. No real compositor, user profile,
# physical keyboard, screenshots from a desktop, or human evidence.
set -Eeuo pipefail
tmp="$(mktemp -d)"
trap 'rm -rf -- "$tmp"' EXIT
dir="$tmp/private-bundle"
mkdir -m 700 "$dir"
for name in help iso jis navigation keypad light dark;do
  convert -size 800x600 xc:lightgray "$dir/typefinch-hyprland-$name.png"
  chmod 600 "$dir/typefinch-hyprland-$name.png"
done
hashes="$(
  for name in help iso jis navigation keypad light dark;do
    printf '%s\t%s\n' "$name" \
      "$(sha256sum "$dir/typefinch-hyprland-$name.png" | cut -d ' ' -f1)"
  done | jq -Rn '[inputs|split("\t")|{key:.[0],value:.[1]}]|from_entries'
)"
sha="$(printf '%040d' 0)"
jq -n --arg sha "$sha" --argjson images "$hashes" '{
  schema:1,kind:"typefinch-local-hyprland-qa",state:"passed",
  compositor:"real-hyprland",source_commit:$sha,clean_source_tree:true,
  keyboard_inspection:"read-only",connected_keyboard_count:2,
  focused_os_input_verified:true,window_only_capture:true,
  help_changed_pixels:4000,light_dark_changed_pixels:5000,
  illustrated_geometry:["ISO","JIS","navigation","keypad"],
  theme_modes:["system","light","dark","system"],
  system_theme_transition_verified:false,
  physical_keyboard_routing_verified:false,
  per_device_attribution_verified:false,
  checks:{T001_live_window_screenshot:true,
    T401_live_connected_keyboard_inspection:true,
    T902_targeted_keyboard_input:true,T902_theme_layout_screenshots:true},
  screenshots:$images
}' >"$dir/typefinch-hyprland-receipt.json"
chmod 600 "$dir/typefinch-hyprland-receipt.json"
original="$tmp/original"
cp "$dir/typefinch-hyprland-receipt.json" "$original"
cp "$dir/typefinch-hyprland-help.png" "$tmp/help-original.png"
bash tests/verify_hyprland_evidence.sh "$dir" >"$tmp/safe-result"
grep -q 'PRIVATE_HYPRLAND_EVIDENCE_STRUCTURALLY_VALID' "$tmp/safe-result"
refused(){
  if bash tests/verify_hyprland_evidence.sh "$dir" >"$tmp/bad-out" 2>"$tmp/bad-err";then
    echo "Tampered, overclaiming or unsafe bundle was accepted" >&2;exit 1
  fi
  if grep -Fq "$tmp/" "$tmp/bad-err";then
    echo "Private bundle pathname was echoed in an error" >&2;exit 1
  fi
}
for mutation in \
  '.system_theme_transition_verified=true' \
  '.physical_keyboard_routing_verified=true' \
  '.per_device_attribution_verified=true' \
  '.checks.T902_targeted_keyboard_input=false' \
  '.connected_keyboard_count=0' \
  '.keyboard_names=["private user device"]' \
  '.screenshots.help="0000"';do
  jq "$mutation" "$original" >"$dir/typefinch-hyprland-receipt.json"
  refused
done
cp "$original" "$dir/typefinch-hyprland-receipt.json"
chmod 755 "$dir"
refused
chmod 700 "$dir"
printf 'tampered' >>"$dir/typefinch-hyprland-help.png"
refused
# Restore exact original bytes. PNG encoders may embed variable metadata.
cp "$tmp/help-original.png" "$dir/typefinch-hyprland-help.png"
chmod 600 "$dir/typefinch-hyprland-help.png"
bash tests/verify_hyprland_evidence.sh "$dir" >"$tmp/safe-result"
ln -s "$tmp/private-image" "$dir/extra"
refused
rm "$dir/extra"
rm "$dir/typefinch-hyprland-help.png"
ln -s "$tmp/private-image" "$dir/typefinch-hyprland-help.png"
refused
echo "Private Hyprland bundle SHA-256/schema/privacy/screenshot tamper self-test PASS (no compositor accessed)"
