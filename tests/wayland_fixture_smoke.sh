#!/usr/bin/env bash
# Synthetic Wayland metadata fixture. Tests the real bounded child-process path,
# not live keyboard-equipped Wayland hardware or key-event capture.
set -euo pipefail
tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT
cat > "$tmp/seat-probe" <<'SH'
#!/bin/sh
[ "${1:-}" = "--_probe_wayland_xkb_seats" ] || exit 2
case "${TYPEFINCH_QA_WAYLAND_CASE:-}" in
  two_seats)
    printf '%s\n' '[{"id":10,"name":"shared","keyboard":true},{"id":11,"name":"shared","keyboard":true}]'
    ;;
  reordered)
    printf '%s\n' '[{"id":11,"name":"shared","keyboard":true},{"id":10,"name":"shared","keyboard":true}]'
    ;;
  no_keyboard)
    printf '%s\n' '[{"id":10,"name":"pointer-only","keyboard":false}]'
    ;;
  duplicate_id)
    printf '%s\n' '[{"id":10,"name":"first","keyboard":true},{"id":10,"name":"second","keyboard":true}]'
    ;;
  malformed) printf 'not json\n' ;;
  oversized) head -c 20000 /dev/zero | tr '\000' a ;;
  command_failed) exit 17 ;;
  hung) exec sleep 9 ;;
  *) exit 2 ;;
esac
SH
chmod 755 "$tmp/seat-probe"
export TYPEFINCH_QA_WAYLAND_FIXTURE="$tmp/seat-probe"
for scenario in two_seats reordered no_keyboard duplicate_id malformed oversized command_failed hung; do
  export TYPEFINCH_QA_WAYLAND_CASE="$scenario"
  cargo test devices::tests::synthetic_wayland_probe_subprocess_is_bounded_and_rejects_untrusted_output -- --nocapture
  echo "Synthetic Wayland subprocess $scenario PASS (not hardware acceptance)"
done
