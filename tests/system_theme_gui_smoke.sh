#!/usr/bin/env bash
# Real Typefinch GUI + disposable MOCK portal only. This is not a live OS
# theme transition, a real Hyprland test, or acceptance evidence.
set -Eeuo pipefail
tmp="$(mktemp -d)"
app=''
cleanup(){
  [[ -z "$app" ]] || { kill "$app" 2>/dev/null || true; wait "$app" 2>/dev/null || true; }
  rm -rf -- "$tmp"
}
trap cleanup EXIT
mkdir -m 700 "$tmp/bin" "$tmp/config" "$tmp/data" "$tmp/cache"
export XDG_CONFIG_HOME="$tmp/config" XDG_DATA_HOME="$tmp/data"
export XDG_CACHE_HOME="$tmp/cache"
export TYPEFINCH_QA_REPORT="$tmp/qa.json"
export TYPEFINCH_QA_SYSTEM_THEME_REPORT="$tmp/theme-report.json"
export TYPEFINCH_FAKE_PORTAL_STATE="$tmp/portal-state"
printf '1\n' >"$TYPEFINCH_FAKE_PORTAL_STATE"
cat >"$tmp/bin/dbus-send" <<'MOCK'
#!/usr/bin/env bash
set -Eeuo pipefail
# Reject any unexpected interface. This fake never connects to D-Bus.
case " $* " in
  *"org.freedesktop.portal.Settings.Read"*"string:org.freedesktop.appearance"*"string:color-scheme"*) ;;
  *) exit 2;;
esac
read -r scheme <"$TYPEFINCH_FAKE_PORTAL_STATE"
case "$scheme" in 0|1|2|99) ;; *) exit 2;; esac
printf 'method return time=1 sender=:1.4 -> destination=:1.9 serial=2 reply_serial=2\n'
printf '   variant       variant          uint32 %s\n' "$scheme"
MOCK
chmod 700 "$tmp/bin/dbus-send"
export PATH="$tmp/bin:$PATH"
./target/debug/typefinch >"$tmp/app.stdout" 2>"$tmp/app.stderr" &
app=$!
wait_for_theme(){
  local expected="$1" actual="$2" got=0
  for _ in $(seq 1 110);do
    kill -0 "$app" 2>/dev/null ||
      { echo "Disposable native GUI exited before mocked portal transition" >&2;exit 1; }
    if [[ -s "$TYPEFINCH_QA_SYSTEM_THEME_REPORT" ]] &&
      jq -e --arg expected "$expected" --arg actual "$actual" '
        .synthetic==true and .state=="xdg-portal-system-theme" and
        .source=="read-only-settings-portal" and
        (.portal_dark|tostring)==$expected and
        ($actual=="any" or (.actual_dark|tostring)==$actual) and
        (.actual_dark|type)=="boolean" and
        .os_theme_transition_verified==false and
        (keys|sort)==([
           "synthetic","state","source","portal_dark","actual_dark",
           "os_theme_transition_verified"
         ]|sort)
      ' "$TYPEFINCH_QA_SYSTEM_THEME_REPORT" >/dev/null 2>&1;then
      got=1;break
    fi
    sleep 0.2
  done
  ((got==1)) || { echo "Native GUI did not follow mocked system appearance" >&2;exit 1; }
}
wait_for_theme true true
printf '2\n' >"$TYPEFINCH_FAKE_PORTAL_STATE"
wait_for_theme false false
# 0 is XDG's "no preference" and must defer to the toolkit, not force Light.
printf '0\n' >"$TYPEFINCH_FAKE_PORTAL_STATE"
wait_for_theme null any
# Re-enter a forced choice to verify unknown -> toolkit fallback is observed.
printf '1\n' >"$TYPEFINCH_FAKE_PORTAL_STATE"
wait_for_theme true true
# Unsupported portal values must NOT be interpreted as forced Dark/Light.
printf '99\n' >"$TYPEFINCH_FAKE_PORTAL_STATE"
wait_for_theme null any
echo "Mock portal native GUI System appearance transition PASS (not real Hyprland acceptance)"
