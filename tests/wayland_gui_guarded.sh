#!/usr/bin/env bash
# Full native Wayland GUI acceptance with ONE narrowly scoped retry only when
# the external Weston 13 X11-backend process segfaults before the first
# keyboard-input assertion. Never retry product/assertion failures.
set -Eeuo pipefail
log="$(mktemp)"
trap 'rm -f "$log"' EXIT
for attempt in 1 2;do
  if xvfb-run -a -s "-screen 0 1280x900x24" bash tests/wayland_gui_smoke.sh >"$log" 2>&1;then
    cat "$log"
    if ((attempt==2));then
      echo "External Weston startup segfault recovered on full fresh native acceptance rerun."
    fi
    exit 0
  fi
  cat "$log" >&2
  if ((attempt==1)) \
    && grep -F 'Segmentation fault' "$log" >/dev/null \
    && grep -F 'weston --backend=x11-backend.so' "$log" >/dev/null \
    && grep -F 'Typefinch exited before native Wayland keyboard input' "$log" >/dev/null;then
    echo "Weston X11 backend crashed before native input. Retrying entire isolated scenario once; application assertions remain strict." >&2
  else
    echo "Native Wayland assertion failed or compositor crash outside bootstrap; not retried." >&2
    exit 1
  fi
done
exit 1
