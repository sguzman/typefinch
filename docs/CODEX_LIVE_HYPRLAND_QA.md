# 🖥️ Typefinch: Codex live Hyprland acceptance job

**Scope:** existing T001, T401 and T902 checks. No new roadmap item.

This is a local-desktop job, not GitHub-hosted CI. The director's
connected GitHub account cannot control the user's Hyprland session.
Codex must run inside that session as the ordinary user. No root,
remote login, evdev capture, global shortcuts, new privileges, or
persistent Hyprland configuration changes.

## Preconditions enforced by Codex

1. The user has **explicitly authorized this specific live desktop QA
   session**, which sends target-window shortcuts and captures PNGs of
   the disposable Typefinch window. General development delegation
   alone is not authorization. Without it, **stop without setting
   TYPEFINCH_QA_CONSENT**.
2. The local Typefinch clone is clean in its source/test manifests and
   the full Git SHA matches the current GitHub origin/main with passing
   CI. Do not overwrite local edits; do not assume a stale build is
   acceptable.
3. The compositor is genuine, currently unlocked Hyprland, with native
   Wayland support. Run as the existing desktop user. If safety guards
   fail, do not modify those guards or retry with broader privileges.

## Authorized commands (Codex runs, not the human)

From the Typefinch repository root, after explicit authorization:

- bash tests/live_hyprland_acceptance.sh --self-test
- bash tests/hyprland_evidence_bundle_selftest.sh
- TYPEFINCH_QA_CONSENT=1 bash tests/run_live_hyprland_codex_qa.sh

Never set persistent consent in a shell profile, process supervisor,
GitHub workflow or systemd service.

The wrapper generates a new private 0700 directory under
XDG_STATE_HOME/typefinch/hyprland-qa or
~/.local/state/typefinch/hyprland-qa, containing exactly seven
0600 window-only screenshots and one 0600 redacted receipt. The
script's structural verifier checks every screenshot digest, PNG
dimensions, exact-source SHA, minimum image differences and forbidden
overclaims.

Codex must also **visually review these screenshots locally**:
check nonblank Typefinch help, legible ISO/JIS/function/keypad previews,
correct Light and Dark appearance, no clipping, unrelated windows or
obstruction. A structurally valid image alone is not visual acceptance.

Never commit, upload or paste screenshots, local paths, private
keyboard addresses, inventory JSON, log files, or profile text into
public repositories, issues, PRs, CI artifacts or conversations.
Report only a sanitized result: commit SHA, completed/failed checks,
whether visual review passed and any external blocker.

## Acceptance mapping

| Existing criterion | Candidate evidence from live session |
| --- | --- |
| T001 live screenshot | Real mapped disposable window, safe screenshot, local visual review |
| T401 connected keyboard | Read-only Hyprland and Typefinch inspector inventory parity and stability |
| T902 keyboard/input QA | Target-only compositor shortcuts and one aggregate focused OS typing attempt |
| T902 theme/layout screenshots | Seven unobscured images, F4/F10 geometry changes and app Light/Dark changes |

These are only candidate evidence until a trusted reviewer validates
the live-run provenance and visual result. **Never automatically
edit docs/progress.json or increase acceptance based on synthetic CI.**

## Explicitly not verified

T003 operating-system theme transition (only app theme toggling);
T305 physical key routing; T404 safe scoped per-device capture;
T502 real human training efficacy; T704 authorized real GitHub profile
publication; T804 comprehensive live RTL, native IME and accessibility;
T904 dependent final MVP/release acceptance.

If the user has not authorized live QA, report **blocked on local
authorization**, without asking for manual screenshot or keyboard
testing. The ordinary user must not serve as an IPC transport.

A failed/incomplete run deletes its newly created private bundle.
Previously verified bundles are never overwritten. If focus, lock,
keyboard inventory or safe screenshot geometry changes, abort rather
than refocusing someone else's window or capturing a broader region.
