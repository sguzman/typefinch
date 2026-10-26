#!/usr/bin/env bash
# Local Codex/agent entrypoint. Run ONLY with prior explicit desktop-QA
# authorization, inside the user's unlocked Hyprland session. No CI/live
# fallback, no global input capture, no profile reuse, no network upload.
set -Eeuo pipefail
[[ "$#" == 0 ]] || { echo "No arguments accepted" >&2;exit 2; }
[[ "$(printenv TYPEFINCH_QA_CONSENT 2>/dev/null || true)" == 1 ]] ||
  { echo "Explicit TYPEFINCH_QA_CONSENT=1 required" >&2;exit 2; }
[[ -n "$(printenv HYPRLAND_INSTANCE_SIGNATURE 2>/dev/null || true)" ]] ||
  { echo "Live Hyprland desktop required; no virtual fallback" >&2;exit 2; }
[[ -f Cargo.toml && -f rust-toolchain.toml ]] ||
  { echo "Run from Typefinch root" >&2;exit 2; }
umask 077
root="$(realpath -- "$(git rev-parse --show-toplevel)")"
state_dir="$(printenv XDG_STATE_HOME 2>/dev/null || true)"
[[ -n "$state_dir" ]] || state_dir="$HOME/.local/state"
base="$(realpath -m -- "$state_dir/typefinch/hyprland-qa")"
[[ "$base" != "$root" && "$base" != "$root/"* ]] ||
  { echo "Private screenshot evidence cannot live inside the repository" >&2;exit 2; }
mkdir -p -m 700 -- "$base"
[[ -d "$base" && ! -L "$base" ]] ||
  { echo "Invalid local evidence parent" >&2;exit 2; }
chmod 700 -- "$base"
bundle="$(mktemp -d "$base/live.XXXXXXXX")"
status=1
cleanup(){
  if [[ "$status" != 0 ]];then
    # The bundle was just created under a strictly private parent. A failed
    # run must not leave incomplete/ambiguous screenshots to be accepted.
    rm -rf -- "$bundle"
  fi
}
trap cleanup EXIT
export TYPEFINCH_QA_OUTPUT_DIR="$bundle"
result="$(bash tests/live_hyprland_acceptance.sh)"
bash tests/verify_hyprland_evidence.sh "$bundle" >/dev/null
# Avoid writing a second file into the strict eight-file evidence bundle.
# Report a bundle path locally to the Codex operator, never into GitHub.
printf '%s\n' "$result"
printf 'PRIVATE_EVIDENCE_DIRECTORY=%s\n' "$bundle"
printf '%s\n' "LIVE_BUNDLE_REQUIRES_VISUAL_AND_PROVENANCE_REVIEW"
status=0
