#!/usr/bin/env bash
# Full native Wayland GUI acceptance: one fresh compositor per RTL language; at
# most TWO retries for a verified external Weston pre-score SIGSEGV plus
# lost Wayland transport. Never retry product/scoring assertion failures.
set -Eeuo pipefail
# Classify *only* the external compositor's SIGSEGV with lost Wayland
# transport before any proven completed Typefinch typing session. Keep
# this separately testable so CI cannot silently grow a broad retry policy.
weston_pre_score_crash(){
  local log="$1"
  grep -F 'Segmentation fault' "$log" >/dev/null &&
  grep -F 'weston --backend=x11-backend.so' "$log" >/dev/null &&
  ! grep -F 'Nested Weston real Wayland wl_keyboard PASS:' "$log" >/dev/null &&
  ! grep -F 'Native Weston wl_keyboard Japanese RO/YEN Base/Shift PASS:' "$log" >/dev/null &&
  ! grep -F 'Native Weston wl_keyboard did not deliver the complete test' "$log" >/dev/null &&
  ! grep -F 'Native JIS physical mismatch' "$log" >/dev/null &&
  ! grep -F 'Native Wayland RTL physical-position scoring failed' "$log" >/dev/null &&
  ! grep -F 'Native Weston wl_keyboard RTL Emulated PASS:' "$log" >/dev/null &&
  ! grep -F 'Native German Wayland physical mismatch' "$log" >/dev/null &&
  ! grep -F 'Native Weston wl_keyboard German RightAlt and ISO 102nd PASS:' "$log" >/dev/null &&
  { grep -F 'Typefinch exited before native Wayland keyboard input' "$log" >/dev/null ||
    grep -F 'Nested Weston died before native input stabilization' "$log" >/dev/null ||
    { grep -F 'Connection reset by peer' "$log" >/dev/null &&
      grep -F 'No scored native Wayland typing result' "$log" >/dev/null; } ||
    { grep -F 'Wayland app exited' "$log" >/dev/null &&
      grep -F 'Connection reset by peer' "$log" >/dev/null; } ||
    { grep -F 'X Error of failed request:  BadWindow' "$log" >/dev/null &&
      grep -F 'Connection reset by peer' "$log" >/dev/null &&
      grep -F 'WinitEventLoop(ExitFailure(1))' "$log" >/dev/null; }; }
}
# Tiny standalone contract test: explicitly reject after-score crashes,
# Typefinch application assertion failures and unrelated interruptions.
fixture=tests/wayland_gui_smoke.sh
case "${1:-}" in
  --jis) fixture=tests/jis_wayland_emulation_smoke.sh ;;
  --rtl) fixture=tests/rtl_wayland_emulation_smoke.sh ;;
  --iso) fixture=tests/iso_wayland_emulation_smoke.sh ;;
  --self-test|'') ;;
  *) echo "Unsupported native Wayland fixture" >&2; exit 2 ;;
esac
if [[ "${1:-}" == --self-test ]];then
  file="$(mktemp)"
  trap 'rm -f "$file"' EXIT
  printf '%s\n' 'Segmentation fault weston --backend=x11-backend.so' \
    'Typefinch exited before native Wayland keyboard input' >"$file"
  weston_pre_score_crash "$file" || exit 1
  printf '%s\n' 'Segmentation fault weston --backend=x11-backend.so' \
    'Nested Weston died before native input stabilization' >"$file"
  weston_pre_score_crash "$file" || exit 1
  printf '%s\n' 'Segmentation fault weston --backend=x11-backend.so' \
    'Connection reset by peer' \
    'No scored native Wayland typing result' >"$file"
  weston_pre_score_crash "$file" || exit 1
  printf '%s\n' 'Segmentation fault weston --backend=x11-backend.so' \
    'Connection reset by peer' 'Wayland app exited' >"$file"
  weston_pre_score_crash "$file" || exit 1
  # A Weston X11 BadWindow SIGSEGV immediately after socket startup
  # can terminate winit without reaching the normal pre-score diagnostic.
  printf '%s\n' 'Segmentation fault weston --backend=x11-backend.so' \
    'X Error of failed request:  BadWindow' 'Connection reset by peer' \
    'WinitEventLoop(ExitFailure(1))' >"$file"
  weston_pre_score_crash "$file" || exit 1
  printf '%s\n' 'Segmentation fault weston --backend=x11-backend.so' \
    'Connection reset by peer' 'WinitEventLoop(ExitFailure(1))' >"$file"
  if weston_pre_score_crash "$file";then exit 1;fi
  printf '%s\n' 'X Error of failed request:  BadWindow' \
    'Connection reset by peer' 'WinitEventLoop(ExitFailure(1))' >"$file"
  if weston_pre_score_crash "$file";then exit 1;fi
  # A scored session or application scoring mismatch must NEVER be
  # laundered into a compositor-bootstrap retry, for either fixture.
  for counterexample in \
    'Nested Weston real Wayland wl_keyboard PASS:' \
    'Native Weston wl_keyboard Japanese RO/YEN Base/Shift PASS:' \
    'Native Weston wl_keyboard did not deliver the complete test' \
    'Native JIS physical mismatch' \
    'Native Wayland RTL physical-position scoring failed' \
    'Native Weston wl_keyboard RTL Emulated PASS:' \
    'Native German Wayland physical mismatch' \
    'Native Weston wl_keyboard German RightAlt and ISO 102nd PASS:';do
    printf '%s\n' 'Segmentation fault weston --backend=x11-backend.so' \
      'Connection reset by peer' 'No scored native Wayland typing result' \
      "$counterexample" >"$file"
    if weston_pre_score_crash "$file";then exit 1;fi
  done
  printf '%s\n' 'Native application scoring mismatch' \
    'No scored native Wayland typing result' >"$file"
  if weston_pre_score_crash "$file";then exit 1;fi
  printf '%s\n' 'Segmentation fault weston --backend=x11-backend.so' \
    'Native input wrong: 2/3' >"$file"
  if weston_pre_score_crash "$file";then exit 1;fi
  echo "External Weston pre-score retry guard positive/negative cases PASS"
  exit 0
fi
log="$(mktemp)"
trap 'rm -f "$log"' EXIT
# The RTL fixture previously reused one Weston instance for Arabic, then
# restarted the child app for Hebrew. Weston 13's X11 backend can SIGSEGV
# at that app transition after Arabic has already scored 4/4. Preserve the
# strict no-retry-after-score rule by testing each language in a fresh
# complete compositor run. Neither score can satisfy the other's gate.
segments=("")
if [[ "${1:-}" == --rtl ]];then segments=(ara il);fi
for segment in "${segments[@]}";do
  passed=0
  for attempt in 1 2 3;do
    if [[ -n "$segment" ]];then
      if xvfb-run -a -s "-screen 0 1280x900x24" bash "$fixture" "$segment" >"$log" 2>&1;then
        passed=1
      fi
    else
      if xvfb-run -a -s "-screen 0 1280x900x24" bash "$fixture" >"$log" 2>&1;then
        passed=1
      fi
    fi
    if ((passed==1));then
      cat "$log"
      if ((attempt>1));then
        echo "Confirmed external Weston pre-score SIGSEGV recovered on full fresh native acceptance rerun (attempt $attempt)."
      fi
      break
    fi
    cat "$log" >&2
    # Only retry a proven *external* Weston SIGSEGV before any scored
    # native session. Incomplete scores, product assertions, and a crash
    # after scoring remain non-retriable in each isolated language run.
    if ((attempt<3)) && weston_pre_score_crash "$log";then
      echo "Confirmed external Weston SIGSEGV before first verified native typing (attempt $attempt/3). Retrying isolated GUI test; every app assertion remains strict." >&2
      sleep 0.75
    else
      echo "Native Wayland assertion failed or compositor crash outside bootstrap; not retried." >&2
      exit 1
    fi
  done
  ((passed==1)) || exit 1
done
if [[ "${1:-}" == --rtl ]];then
  echo "Independent Arabic and Hebrew native Weston scoring PASS (one disposable compositor per language)."
fi
