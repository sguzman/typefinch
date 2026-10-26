# 🔐 Per-device input permissions and threat model

**Status: design-only acceptance gate.** Typefinch currently has no per-device keyboard-event capture backend. The `CapturePolicy::InspectionOnly` mode is hard-coded and visible in the UI. The Hyprland and native X11 inspectors only read keymap/device metadata; they do not intercept keyboard input. This document authorizes no new access or permissions.

## What is at risk

A physical keyboard-event stream may contain passwords, authentication codes, private messages, clipboard-related shortcuts, and data typed into **other applications**. Capturing that stream is materially more privileged than receiving text and key events from Typefinch's own focused GUI. Hyprland addresses and configured XKB layouts are **inventory metadata**, not evidence of which physical device emitted a key event.

Threats include accidental observation while Typefinch is unfocused, stale permission surviving a restart, confused-device selection after hotplug, coalescing two sources into double-counted attempts, and exposing raw keys, device identifiers, or private text in logs, exports, CI artifacts, or a public repository.

## Required boundary for a possible future backend

1. **Deny by default.** Existing native typing and the read-only keyboard inspector must operate without additional permissions. The UI must say `unsupported` when trustworthy scoped capture is unavailable; it must never silently substitute XWayland, X11 configured layouts, or guessed evdev codes as device attribution.
2. **Explicit, informed, reversible consent.** Any future scoped backend must explain what is observed, the exact named device and scope, and how to stop it. Consent is a deliberate in-app action for that session only, never implicit from selecting a metadata row. Revocation and focus loss must disable capture immediately; restarting does not reinstate consent.
3. **OS-enforceable least privilege.** Prefer a compositor-mediated mechanism that supplies **only** key events for the Typefinch-focused surface and grants truthful physical-device attribution. Ordinary Wayland seat events do not identify physical keyboards. If no properly scoped compositor capability is available, remain inspection-only. Do not elevate the Typefinch UI to root, request ambient access to `/dev/input`, install a background key logger or daemon, or make a global evdev reader the default.
4. **No secondary typing stream.** A permitted physical-key path must be explicitly selected as the session input source or remain preview-only. Never score both the native committed-text path and a physical-device stream for the same event. Dead keys, Compose, Shift, AltGr and IME commit semantics need separate physical-device regression tests before scoring is enabled.
5. **Data minimization.** Only aggregate session statistics may be persisted by default. Do not retain raw keycodes, scan-code sequences, per-key timing traces, hardware addresses, typed phrases, or other applications' keystrokes. Error reports, QA fixtures and exported profile cards must remain free of real input text and private device identifiers.
6. **Safe failure.** Denial, revocation, unplugging, permission errors, changed keymaps or lost focus immediately revert to the existing native seat-level typing path without silently attributing input to another keyboard. A change mid-session must not upgrade an unknown provenance to a supposedly verified device.

## 2026 compositor capability review: why existing APIs are insufficient

The tempting available interfaces do **not** currently meet the required
compositor-enforced combination of **focused Typefinch surface + one named
physical keyboard**:

- [Hyprland input-capture permission](https://wiki.hypr.land/configuring/core/advanced-configuration/permissions/)
  is explicitly described as access to **all compositor input** (keyboard,
  pointer and touch), subject to a permission prompt. Consent alone does not
  reduce its scope to one keyboard or the currently focused Typefinch surface.
  Do not request this permission as a workaround for missing per-device input.
- [XDG InputCapture portal](https://flatpak.github.io/xdg-desktop-portal/docs/doc-org.freedesktop.portal.InputCapture.html)
  delegates activation to the compositor; the standardized trigger is an
  *edge-of-screen pointer barrier*, not application focus or a selected
  hardware keyboard. Its libei stream may capture input after activation,
  but this is **not** proof of per-device attribution or a focused-window
  grant. Do not equate a portal consent dialog with scoped capture.
- [Hyprland per-device binds](https://wiki.hypr.land/configuring/core/binds/devices/per-device/)
  restrict which device can activate a **compositor-wide keybind**. They
  are not an ordinary GUI's focus-restricted, revocable event subscription.
  Never install temporary global keybinds to harvest character input,
  even if filtered by device or guarded by a client-side focus check.
- [XDG RemoteDesktop portal](https://flatpak.github.io/xdg-desktop-portal/docs/doc-org.freedesktop.portal.RemoteDesktop.html)
  authorizes remote **injection** of keyboard/pointer events; it is not
  a per-device event-provenance reader.

**Implementation decision:** keep `CapturePolicy::InspectionOnly` hard-coded
and retain the compositor's normal focused `wl_keyboard` or committed-text
input path. A future backend requires a documented, actually deployed
compositor capability with enforced scope and truthful device attribution,
plus explicit consent, revocation, focus loss and multiple-keyboard testing.
Merely discovering an input-capture global, a portal, a device name or an
EIS connection must never enable independent scoring. This review is
architectural evidence, **not** acceptance of T404's backend or hardware test.

## Acceptance and negative tests before any backend ships

A future implementation needs an explicit permission UX, audited privilege scope, and evidence from the actual target compositor (Hyprland first). Required negative tests: consent absent; Typefinch unfocused; consent revoked; restarted app; device unplugged or replaced; multiple keyboards with different layouts; password input in another application; IME/Compose and AltGr; and no double-scored physical plus committed-text event.

CI should prove that disabling or withholding the permission still leaves the existing GUI and local history functional. Live hardware tests must distinguish actual compositor-granted device events from synthetic X11 events and static XKB symbol queries. No CI-only mock or permission-design document counts as proof of a working physical capture backend.

**Current scope:** the permission and threat model can be accepted independently. Backend implementation and the no-global-interception hardware verification remain **unaccepted**.
