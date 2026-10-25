#!/usr/bin/env bash
# Full native Wayland GUI acceptance with ONE narrowly scoped retry only when
# the external Weston 13 X11-backend process segfaults before the first
# verified scored native typing session, with demonstrated connection loss. Never retry product/assertion failures.
set -Eeuo pipefail
# Classify *only* the external compositor's SIGSEGV with lost Wayland
# transport before any proven completed Typefinch typing session. Keep
# this separately testable so CI cannot silently grow a broad retry policy.
weston_pre_score_crash(){
  local log="$1"
  grep -F 'Segmentation fault' "$log" >/dev/null &&
  grep -F 'weston --backend=x11-backend.so' "$log" >/dev/null &&
  ! grep -F 'Nested Weston real Wayland wl_keyboard PASS:' "$log" >/dev/null &&
  ! grep -F 'Native Weston wl_keyboard did not deliver the complete test' "$log" >/dev/null &&
  { grep -F 'Typefinch exited before native Wayland keyboard input' "$log" >/dev/null ||
    { grep -F 'Connection reset by peer' "$log" >/dev/null &&
      grep -F 'No scored native Wayland typing result' "$log" >/dev/null; }; }
}
# Tiny standalone contract test: explicitly reject after-score crashes,
# Typefinch application assertion failures and unrelated interruptions.
if [[ "${1:-}" == --self-test ]];then
  file="$(mktemp)"
  trap 'rm -f "$file"' EXIT
  printf '%s\n' 'Segmentation fault weston --backend=x11-backend.so' \
    'Typefinch exited before native Wayland keyboard input' >"$file"
  weston_pre_score_crash "$file" || exit 1
  printf '%s\n' 'Segmentation fault weston --backend=x11-backend.so' \
    'Connection reset by peer' \
    'No scored native Wayland typing result' >"$file"
  weston_pre_score_crash "$file" || exit 1
  # A successful scored session and an explicit application scoring
  # failure can NEVER be laundered into a recoverable compositor crash.
  for counterexample in \
    'Nested Weston real Wayland wl_keyboard PASS:' \
    'Native Weston wl_keyboard did not deliver the complete test';do
    printf '%s\\n' 'Segmentation fault weston --backend=x11-backend.so' \
      'Connection reset by peer' 'No scored native Wayland typing result' \
      "$counterexample" >"$file"
    if weston_pre_score_crash "$file";then exit 1;fi
  done
  printf '%s\\n' 'Native application scoring mismatch' \
    'No scored native Wayland typing result' >"$file"
  if weston_pre_score_crash "$file";then exit 1;fi
  printf '%s\\n' 'Segmentation fault weston --backend=x11-backend.so' \
    'Native input wrong: 2/3' >"$file"
  if weston_pre_score_crash "$file";then exit 1;fi
  echo "External Weston pre-score retry guard positive/negative cases PASS"
  exit 0
fi
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
  if ((attempt==1)) && weston_pre_score_crash "$log";then
    echo "Confirmed external Weston SIGSEGV before the first verified native typing result. Retrying the entire isolated GUI test once; all app assertions remain strict." >&2
  else
    echo "Native Wayland assertion failed or compositor crash outside bootstrap; not retried." >&2
    exit 1
  fi
done
exit 1
