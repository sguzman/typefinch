#!/usr/bin/env bash
# CI-only synthetic compositor metadata, not physical Hyprland/hotplug QA.
set -euo pipefail
tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT
mkdir -p "$tmp/bin"
cat > "$tmp/bin/hyprctl" <<'SH'
#!/bin/sh
[ "${1:-}" = "-j" ] && [ "${2:-}" = "devices" ] || exit 2
if [ "${TYPEFINCH_QA_HYPRLAND_CASE:-}" = "hung" ]; then
  exec sleep 9
fi
exec cat "${TYPEFINCH_QA_HYPRLAND_FIXTURE:?}"
SH
chmod 755 "$tmp/bin/hyprctl"
export PATH="$tmp/bin:$PATH"
export HYPRLAND_INSTANCE_SIGNATURE=typefinch-qa-only
export XDG_SESSION_TYPE=wayland
export TYPEFINCH_QA_HYPRLAND_FIXTURE="$tmp/devices.json"
unset DISPLAY
for scenario in initial reordered unplugged duplicates invalid hung; do
  export TYPEFINCH_QA_HYPRLAND_CASE="$scenario"
  case "$scenario" in
    initial) cat > "$TYPEFINCH_QA_HYPRLAND_FIXTURE" <<'JSON'
{"keyboards":[{"name":"Primary","address":"0x1","active_keymap":"English (US)"},{"name":"Secondary","address":"0x2","active_keymap":"German"}]}
JSON
      ;;
    reordered) cat > "$TYPEFINCH_QA_HYPRLAND_FIXTURE" <<'JSON'
{"keyboards":[{"name":"Secondary","address":"0x2","active_keymap":"German"},{"name":"Primary","address":"0x1","active_keymap":"English (US)"}]}
JSON
      ;;
    unplugged) cat > "$TYPEFINCH_QA_HYPRLAND_FIXTURE" <<'JSON'
{"keyboards":[{"name":"Primary","address":"0x1","active_keymap":"English (US)"}]}
JSON
      ;;
    duplicates) cat > "$TYPEFINCH_QA_HYPRLAND_FIXTURE" <<'JSON'
{"keyboards":[{"name":"Primary","address":"0x1","active_keymap":"English (US)"},{"name":"Impostor","address":"0x1","active_keymap":"German"},{"name":"Anonymous","active_keymap":"German"}]}
JSON
      ;;
    invalid) printf 'not-json\n' > "$TYPEFINCH_QA_HYPRLAND_FIXTURE" ;;
    hung) printf '{"keyboards":[]}\n' > "$TYPEFINCH_QA_HYPRLAND_FIXTURE" ;;
  esac
  cargo test devices::tests::synthetic_hyprland_inventory_lifecycle_requires_explicit_fixture -- --nocapture
  echo "Synthetic Hyprland inventory $scenario PASS (not a physical keyboard test)"
done
