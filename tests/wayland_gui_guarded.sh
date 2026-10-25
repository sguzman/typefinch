#!/usr/bin/env bash
# Full native Wayland GUI acceptance with ONE narrowly scoped retry only when
# the external Weston 13 X11-backend process segfaults before the first
# verified scored native typing session, with demonstrated connection loss. Never retry product/assertion failures.
set -Eeuo pipefail
log="$(mktemp)"
trap 'rm -f "$log"' EXIT
for attempt in 1 2;do
  if xvfb-run -a -s "-screen 0 1280x900x24" bash tests/wayland_gui_smoke.sh >"$log" 2>&1;then
    cat "$log"
    if ((attempt==2));then
      echo "Confirmed external Weston pre-score SIGSEGV recovered on full fresh native acceptance rerun."
    fi
    exit 0
  fi
  cat "$log" >&2
  # Weston sometimes segfaults *after* it advertises a socket and
  # receives the first synthetic keyboard events, but before the first
  # source-scoring report. Detect the external Weston SIGSEGV, verified
  # loss of the Wayland connection, and absence of *any* successful scored
  # session. Do NOT retry failed product assertions, incomplete scores,
  # or a crash after a successfully completed native typing session.
  if ((attempt==1)) \
    && grep -F 'Segmentation fault' "$log" >/dev/null \
    && grep -F 'weston --backend=x11-backend.so' "$log" >/dev/null \
    && ! grep -F 'Nested Weston real Wayland wl_keyboard PASS:' "$log" >/dev/null \
    && { grep -F 'Typefinch exited before native Wayland keyboard input' "$log" >/dev/null \
       || { grep -F 'Connection reset by peer' "$log" >/dev/null \
            && grep -F 'No scored native Wayland typing result' "$log" >/dev/null; }; };then
    echo "Confirmed external Weston SIGSEGV before the first verified native typing result. Retrying the entire isolated GUI test once; all app assertions remain strict." >&2
  else
    echo "Native Wayland assertion failed or compositor crash outside bootstrap; not retried." >&2
    exit 1
  fi
done
exit 1
