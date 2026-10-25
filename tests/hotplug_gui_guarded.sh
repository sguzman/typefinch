#!/usr/bin/env bash
# Typefinch T404: narrowly scoped external Weston crash recovery around the
# disposable native hotplug fixture. Product assertions are never retried.
set -Eeuo pipefail
weston_bootstrap_crash(){
    local log="$1"
    # Weston 13's X11-backend may SIGSEGV before the first completed
    # inventory check, severing the app's Wayland socket.
    grep -E 'Segmentation fault.*weston --backend=x11-backend.so' "$log" >/dev/null &&
    { grep -F 'Connection reset by peer' "$log" >/dev/null ||
      grep -F 'Disposable Weston terminated prematurely' "$log" >/dev/null; } &&
    ! grep -F 'Native Wayland synthetic inventory initial PASS' "$log" >/dev/null &&
    ! grep -F 'Native Wayland GUI synthetic Hyprland metadata attach/detach/error/recovery PASS' "$log" >/dev/null &&
    ! grep -E 'Native Wayland (initial|attached|detached|empty|failed|recovered) state did not converge' "$log" >/dev/null
}
if [[ "${1:-}" == --self-test ]];then
    file="$(mktemp)"
    trap 'rm -f "$file"' EXIT
    printf '%s\n' 'Segmentation fault weston --backend=x11-backend.so' \
      'Io error: Connection reset by peer' > "$file"
    weston_bootstrap_crash "$file"
    printf '%s\n' 'Segmentation fault weston --backend=x11-backend.so' \
      'Native Wayland synthetic inventory initial PASS' \
      'Io error: Connection reset by peer' > "$file"
    if weston_bootstrap_crash "$file";then exit 1;fi
    printf '%s\n' 'Native Wayland initial state did not converge' \
      'Io error: Connection reset by peer' > "$file"
    if weston_bootstrap_crash "$file";then exit 1;fi
    printf '%s\n' 'Segmentation fault weston --backend=x11-backend.so' \
      'Native Wayland attached state did not converge' \
      'Io error: Connection reset by peer' > "$file"
    if weston_bootstrap_crash "$file";then exit 1;fi
    printf '%s\n' 'Segmentation fault weston --backend=x11-backend.so' \
      'Native Wayland GUI synthetic Hyprland metadata attach/detach/error/recovery PASS' \
      'Io error: Connection reset by peer' > "$file"
    if weston_bootstrap_crash "$file";then exit 1;fi
    echo 'External Weston hotplug bootstrap-only retry classifier PASS'
    exit 0
fi
log="$(mktemp)"
trap 'rm -f "$log"' EXIT
for attempt in 1 2;do
    if xvfb-run -a -s "-screen 0 1280x900x24" bash tests/hotplug_gui_smoke.sh >"$log" 2>&1;then
        cat "$log"
        [[ "$attempt" == 1 ]] || echo 'External Weston pre-inventory crash recovered after complete fresh hotplug acceptance rerun'
        exit 0
    fi
    cat "$log" >&2
    if ((attempt==1)) && weston_bootstrap_crash "$log";then
        echo 'Verified external Weston X11-backend SIGSEGV before any inventory acceptance; retrying once on a fresh compositor' >&2
        sleep 0.75
    else
        echo 'Hotplug application assertion or non-bootstrap compositor failure: no retry' >&2
        exit 1
    fi
done
exit 1
