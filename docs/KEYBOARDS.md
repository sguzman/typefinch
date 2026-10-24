# 🗺️ Keyboard support and device selection

## Capability levels

1. **Recognized:** a curated layout identity in the selector, not necessarily installed in system XKB. The source catalog contains many presets from English, Spanish, German, French, other European, Cyrillic, Semitic and Asian layouts.
2. **Native OS input:** accept committed text from the compositor's layout. Keep this default for complex scripts and IME input; advanced tests remain to do.
3. **Software emulation:** interpret physical ANSI key locations through an alternate map inside Typefinch only. Four starter mappings are committed; these are provisional until complete shift/punctuation and live hardware QA.

| Layout | Identity in catalog | Starter emulation |
| --- | --- | --- |
| US QWERTY | ✅ | ✅ ANSI base layer |
| Dvorak | ✅ | ✅ ANSI base layer |
| Colemak | ✅ | ✅ ANSI base layer |
| Colemak-DH | ✅ | ✅ ANSI base layer |
| Workman, Norman, Halmak, Canary, Engram, Semimak | ✅ | XKB if installed; complete modifier/IME QA planned |
| QWERTZ, AZERTY, BÉPO, Neo 2, Turkish F | ✅ | XKB if installed; complete modifier/IME QA planned |
| Spanish, Latin American, Nordic, Cyrillic, Greek | ✅ | XKB if installed; complete modifier/IME QA planned |
| Arabic, Hebrew, Japanese, Korean and more | ✅ | Planned; IME/BiDi/geometry complexities |

The goal is full installed-XKB inventory via `xkeyboard-config` and `libxkbcommon`, plus custom layout editing, physical geometry, modifier and dead-key correctness. Do not falsely present conceptual named layouts as installed or emulatable.

## Multiple keyboards

- **Implemented in source:** read-only `hyprctl -j devices` detects Hyprland inspection rows and reported active keymaps. Other Wayland compositors expose **aggregate `wl_seat` capability metadata**, without active keymaps; genuine X11 sessions use `setxkbmap -query` for a single **configured aggregate seat**. None of these APIs attributes Typefinch typing events to physical keyboards.
- **Not implemented:** selecting a keyboard as an exclusive input source. Ordinary Wayland `wl_keyboard` is seat-level and can aggregate multiple physical keyboards.
- **Future:** per-device event attribution only where a compositor-specific protocol or authorized opt-in evdev/libinput capture allows it. Never secretly capture keys or modify OS keyboard configuration.
- If none of these inspectors is available, regular native typing remains available and the detection limitation is displayed.

Acceptance: hotplug two keyboards; map both with differing layouts; verify focus, shift/AltGr, symbols, dead keys, IMEs, ANSI/ISO/JIS and no double-count of physical + committed events.

[Wayland keyboard protocol](https://wayland.app/protocols/wayland) · [Hyprland devices](https://wiki.hypr.land/configuring/core/devices/) · [Hyprland keyboard-layout documentation](https://wiki.hypr.land/configuring/core/binds/keyboard-layouts/).

## XKB compiler (new implementation)

Where `xkeyboard-config` has a matching layout/variant, Typefinch can compile the installed keymap with `libxkbcommon` and remap common physical ANSI keys inside the application. This broadens *potentially emulatable* entries beyond the four built-in tables, with locale-specific symbols rather than guessed character mappings. Session-scoped XKB dead-key Compose now works for supported ANSI physical positions when the selected locale has a Compose table. This still is **not** complete emulation for physical AltGr, all Compose-key sequences, IMEs, ISO/JIS extras or per-device input attribution. An unavailable XKB preset remains recognition-only.

## Installed XKB registry discovery

At startup, Typefinch reads the local read-only xkeyboard-config rules inventory at `$XKB_CONFIG_ROOT/rules/evdev.lst` (default `/usr/share/X11/xkb/rules/evdev.lst`). It parses base layouts and named variants, deduplicates identities, and lets the user filter the installed list by name or ID alongside curated presets. Choosing an installed entry compiles its map through libxkbcommon where available. Registry discovery itself does not guarantee correct physical geometry, AltGr, Compose or IME support. A missing rules file disables extra discovery without preventing native typing.

## Session layout attribution

Typefinch records the emulated target identity for software-emulated sessions. OS-input sessions use a clearly prefixed `detected/<id>` identity only if a **complete, uniquely addressed** Hyprland keyboard inventory agrees on the active keymap and it resolves uniquely to an installed XKB entry. Otherwise Typefinch records `os:unknown` and excludes that session from layout-comparable trend charts. This is seat-level *best-effort* metadata, not proof of which physical keyboard generated a keystroke; the layout context is snapshotted when a session starts, then downgraded to `os:unknown` if focus changes or a later trusted inventory contradicts it. Polling is periodic, not per-event input attribution, and an unknown session is never retroactively upgraded.

## Refresh after hardware changes

Typefinch re-enumerates the compositor's read-only keyboard inventory when its window regains focus. The selected inspection device is preserved by exact Hyprland address when still present; otherwise selection falls back to the first available device (or none). This does not subscribe to device events while the app is unfocused, and it does not tie scored keyboard events to a selected physical device. There is no privileged global key capture.

## Personal ANSI layouts

The built-in editor creates version-1 JSON keymaps with a UUID-based `custom/<uuid>` identity, a 1–32-grapheme name, and exactly three rows of ten graphemes each for both base and Shift layers. Each physical ANSI key produces the corresponding mapped grapheme during *in-app emulation only*; Space remains Space, and Backspace remains the session's correction action. Editing is draft-based and validated before saving. Imports are explicitly selected local files (≤16 KiB); duplicate IDs get a fresh UUID and never overwrite existing mappings. Exported JSON is local and no external services are involved. This is not full ISO/JIS geometry, AltGr/Compose, or a system-level keyboard remapper.

## Read-only hotplug refresh

While Typefinch is open inside Hyprland or another supported Wayland session, it refreshes read-only metadata on focus gain and approximately every 20 seconds while focused. Enumeration runs on a worker thread to avoid blocking typing. Selection survives reordering when the compositor's inspection identity remains stable, but it does not grant physical-device event attribution. Genuine X11 configured-seat inspection is on-demand. Manual refresh remains available; polling stops when the window is unfocused.

## Modifier-layer inspector

The preview exposes Base, Shift, AltGr and Shift+AltGr positions. XKB-backed layouts synthesize the right-Alt Level-3 modifier (evdev 100) for *read-only symbol inspection*, while the real input surface intentionally does not treat an undifferentiated left-Alt/right-Alt event as a trustworthy AltGr keystroke. Two-layer personal layouts have no inferred AltGr layer. Supported ANSI dead-key sequences use session-local Compose; physical AltGr, dedicated Compose-key routing, and complete ISO/JIS input remain unverified.

## Stateful Compose in scored ANSI emulation

For installed XKB layouts, a fresh libxkbcommon Compose state is associated with each scored emulation session. A dead key adds no scored character while pending. A completed sequence emits the composed UTF-8 string once; an invalid continuation drops the pending accent and passes through a printable cancelling key. Backspace cancels an uncommitted accent before deleting previously typed text. Session restart, Escape, and window-focus loss reset pending composition; user custom two-layer maps do not pretend to provide XKB Compose.

The existing native-mode committed-text path remains unchanged. If no usable locale Compose table is installed, emulation retains its previous non-composing path rather than inventing symbols. Automated coverage exercises Spanish dead acute + e, cancellation/reset, and one-grapheme scoring ([CI run 39985958620](https://github.com/sguzman/typefinch/actions/runs/39985958620)). Actual compositor AltGr attribution, ISO/JIS, IMEs and physical keyboard acceptance are still separate gates.

## Native X11 Arabic/Hebrew OS-layout integration

A real X11 software-rendered GUI run under disposable Xvfb/Openbox uses
`setxkbmap` **outside Typefinch** to choose Arabic `ara` and Hebrew
`il` keymaps, then sends numeric X11 key-position events to Typefinch's
ordinary **OS input** mode. [CI 40058295306](https://github.com/sguzman/typefinch/actions/runs/40058295306)
passed both four-grapheme words with a deliberately wrong first key,
Backspace, and a corrected final score while retaining raw mistake
history. It also passed one synthetic mixed-language Custom passage
with consecutive Arabic, US Latin and Hebrew layout switches: all
13 Unicode graphemes were entered correctly, and the four archived
frames visually demonstrate progression across differently directed
script runs. The test-only 15-second fixture duration is explicitly
gated and does not change the app's normal timing behavior.
None of this attributes the input to a **physical** keyboard, measures
true human typing speed, or verifies live Hyprland/Fcitx/IBus input.

## Layout symbol regression evidence

Twenty exact XKB output expectations exercise ANSI punctuation, shifted digits, Shift/AltGr symbol levels, and non-Latin code points across US, German, French, Spanish, Latin American, Dvorak, Colemak, Russian, Greek, Hebrew and Arabic layouts. The regression suite and existing GUI checks passed in [GitHub Actions run 39986908299](https://github.com/sguzman/typefinch/actions/runs/39986908299) (107 Rust tests). These are static installed-keymap expectations, **not** proof of compositor-level physical AltGr events, per-device event attribution, ISO/JIS geometry, or full IME correctness.

## Illustrative ANSI, ISO, JIS and split previews

The preview now lets the user choose among four **explicit illustrative main typing blocks**, without inferring a keyboard's hardware shape. ANSI/ISO/JIS labels name keyboard families; the model includes representative control and modifier positions but is not a complete 104/105/109-key device, numeric keypad or comprehensive physical modifier geometry. Key slots are derived from named Linux evdev positions: ISO `KEY_102ND=86`, Japanese `KEY_RO=89`, `KEY_YEN=124`, `KEY_MUHENKAN=94`, `KEY_HENKAN=92`, and `KEY_KATAKANAHIRAGANA=93` ([Linux input-event codes](https://github.com/torvalds/linux/blob/master/include/uapi/linux/input-event-codes.h)).

The UI can inspect installed XKB symbols for extra slots by keycode without routing those slots to a guessed ordinary egui key event. Split layout is **visual separation** with exactly one scored Space identity; a second decorative space half is not a second physical input. Shape selection affects only the preview and is not a persisted hardware configuration. The underlying geometry model and duplicate/non-attribution tests passed [native CI 39992682545](https://github.com/sguzman/typefinch/actions/runs/39992682545) with 111 Rust tests; the expanded synthetic X11 geometry report passed [CI 39994019022](https://github.com/sguzman/typefinch/actions/runs/39994019022) with 112 tests. Full visual screenshot inspection and physical extra-key routing remain separate acceptance checks.

## Keyboard-shape preview acceptance

The modeled **main typing block** was visually checked against actual native X11 captures from [Actions 39997672154](https://github.com/sguzman/typefinch/actions/runs/39997672154) (113 passing Rust tests). ISO, JIS, and split rows fit the 1100×850 inspection window after disabling extra egui item spacing and reducing the geometry unit. The Japanese preview uses the Japanese XKB target during synthetic QA, with the ordinary profile unchanged. The shape selector is presentation-only; F4 cycles ANSI → ISO → JIS → Split, and F3 independently cycles inspected XKB modifier levels.

These are typed main-block illustrations, **not complete keyboards**: complete modifier/edge/control-key geometry, trusted ISO/JIS event routing, and physical Hyprland acceptance are still open. The synthetic X11 checks assert modeled slot counts and that unmappable physical keys never become guessed egui key events; the images are also archived with the CI run.

## Control-key and modifier previews

Core Backspace, Tab, Enter, Caps Lock, left/right Shift, Ctrl and Alt (plus Super on ANSI/ISO/split) are explicit labeled positions in `src/geometry.rs`. Each carries its own Linux evdev code and semantic role. The UI renders readable text captions rather than unsupported Unicode keycap symbols; modifiers and controls never enter the scored-character translation path or pretend left/right physical-event attribution. JIS-specific extra keys remain unattached to ordinary egui physical keys. This is still a **partial main-block model**; not every Japanese modifier, keypad, navigation, or hardware feature is represented.

Automated checks reject overlapping positions and duplicate evdev identities, verify printable geometry matches the XKB emulation-keycode map across all four shapes, and distinguish modifier/control positions from unassigned extra keys. [CI 40003691924](https://github.com/sguzman/typefinch/actions/runs/40003691924) passed 116 Rust tests, physical-key typing QA and native ISO/JIS/split screenshot capture. All three screenshots were visually reviewed for readable captions and no clipped positions. Full geometry, trusted extra-key routing and Hyprland hardware QA remain open.

## Spanish dead-accent preview validation

The XKB preview uses the ASCII-safe `dk` caption for a genuine dead key and `–` only for unknown/unmapped positions. The hover label names the real XKB accent; a dead key is never represented as a successfully committed single character. The isolated native graphical regression selects the installed Spanish `es` keymap, verifies the layout and geometry metadata, and captures the actual five-row preview. The [CI 40004800479](https://github.com/sguzman/typefinch/actions/runs/40004800479) run passed 116 Rust tests and native X11 checks. Its Spanish screenshot was inspected to confirm the two `dk` keycaps render legibly without missing glyphs. This is a preview-layer rendering check, not proof of physical AltGr, complex IMEs or connected Hyprland hardware.

## Function, navigation, and keypad references

Use the **Keys (F10)** selector to switch among the main typing block, function/navigation, and numeric keypad reference views. F4 continues cycling physical shape illustrations; it does not control the separate key region. The new reference regions include 26 function/navigation keys and 17 keypad keys using independently verified Linux `input-event-codes.h` numbers. They are **illustrations of standard positions**, not detected device geometry or captured key events; they never participate in scored sessions, alter the OS keyboard, or imply a connected numeric keypad exists. Key identities and captions are validated for uniqueness, non-overlap, and lack of egui character-key attribution.

[Actions 40006834967](https://github.com/sguzman/typefinch/actions/runs/40006834967) passed 118 Rust tests and software-rendered native X11 tests. F10 switched to both sections in a real graphical process; the two archived screenshots were inspected and show the labels without clipping. Full physical geometry, atypical keyboard configurations, ISO/JIS extra-key event routing, and Hyprland hardware acceptance remain open.

### Japanese top-left mode position

The JIS main-block illustration uses `KEY_ZENKAKUHANKAKU=85` in the top-left location rather than displaying a guessed ANSI grave/backtick character (`KEY_GRAVE=41`). It appears as a non-scoring `Zen` control keycap. This follows the Linux input-event code assignments and does not imply egui can attribute the actual hardware event; the keycap is preview-only. A focused test ensures JIS never silently aliases it to an ANSI printable key.

## Read-only X11 aggregate seat fallback

When the process declares a genuine `XDG_SESSION_TYPE=x11` with `DISPLAY`, Typefinch uses `setxkbmap -query` to inspect one X11 keyboard seat, even if a stale Hyprland signature remains inherited. This is intentionally reported as **"X11 aggregate seat (not a physical keyboard)"** with a stable synthetic inspection identity. The result preserves the reported layout and variant strings without pretending an X11 seat can attribute individual hardware events. There is no write to `setxkbmap`, no privilege escalation, no global keyboard capture, and no fallback through XWayland while a Wayland session is declared. Native Hyprland sessions still use their dedicated inspector. An explicit X11 session cannot be overridden by an inherited Hyprland variable.

[CI 40008974131](https://github.com/sguzman/typefinch/actions/runs/40008974131) passed 122 Rust tests, malicious/missing-layout parser cases, a separate live `Xvfb` + `setxkbmap -query` probe, and the full native graphical suite. This accepts the X11 fallback only. Generic Wayland-compositor XKB seat discovery and verified per-device attribution remain future work; if `setxkbmap` is missing or the X11 query fails, ordinary native typing remains available and the inspector reports the limitation.

## Optional device capture boundary

The current input capability is explicitly `CapturePolicy::InspectionOnly`, regardless of which keyboard row is selected. Physical-device input capture remains unsupported. Any future implementation must first satisfy the [device-input permission and threat model](DEVICE_INPUT_SECURITY.md), including an OS-scoped permission, visible opt-in, revocation, focus boundaries and no double-scoring; the document alone does not create a capture backend.

## Bounded inspection and session provenance

Typefinch opens its GUI before awaiting any compositor metadata. Startup, manual refresh, and focused Hyprland polling share a **single asynchronous inspection worker**, so repeated clicks cannot accumulate concurrent `hyprctl` processes. All inspection subprocesses have a **three-second runtime limit**, bounded stdout (256 KiB Hyprland JSON; 16 KiB X11 and Wayland seat metadata), and no shell interpolation, global keystroke hook or writable system command. A timeout, oversized response or disconnected worker clears the trusted inventory rather than leaving its previous contents marked current.

Keyboard inventory becomes **unverified** on focus loss. Previously inspected rows may remain visible for diagnosis, but their seat capability, configured X11 layout or Hyprland active-map labels now say **last reported / unverified** until a fresh successful probe. The target-adoption action remains disabled while metadata is stale or in flight. A worker launched before that focus transition cannot restore confidence in the reported keymap; after focus returns, Hyprland must supply a fresh inspection result. A running OS-input session with an earlier `detected/<id>` label downgrades to `os:unknown` when focus is lost or the inventory contradicts the old label. Likewise, a new OS-input session started before refreshed inventory arrives is recorded as `os:unknown`, not a guessed layout. Explicit software emulation retains its chosen target, and the app never upgrades an existing unknown session retroactively. Existing inspection rows can remain visible during a focus-triggered refresh, but adopting a reported/configured target is disabled until a fresh successful inventory confirms them. A scored OS-input session that finishes while an inspector refresh is unresolved still saves its typing result, but its previously detected layout label is downgraded to `os:unknown` rather than credited as revalidated. Explicitly selected software-emulated sessions are unaffected.

Hyprland reports with any unaddressed, malformed, or duplicate keyboard identity are now rejected as incomplete. Silently dropping such a row could hide a different active layout and falsely label a scored OS-input session. Rejection clears the trusted inventory and preserves the safer `os:unknown` history label; it does not prevent ordinary native typing. The stricter parser and stale-environment routing have new regression tests, but their complete CI acceptance is not yet recorded here.

[Native CI 40020631995](https://github.com/sguzman/typefinch/actions/runs/40020631995) passed 132 Rust tests, the real Xvfb aggregate-seat probe and all existing X11 graphical checks. It covers subprocess timeout, output limits, focus invalidation, fresh-vs-stale layout attribution and nonblocking keyboard inspection. **No live Hyprland hotplug or physical-device event routing is claimed by these tests.**

## Generic Wayland seat-capability fallback

In a non-Hyprland Wayland session (declared `XDG_SESSION_TYPE=wayland` and a Wayland socket), Typefinch can query the standard `wl_registry` and `wl_seat` interfaces through the Rust `wayland-client` library. It obtains **seat names and a boolean keyboard-capability flag**, not key events or active keyboard layouts. It does **not** request `wl_keyboard`, derive individual physical devices, open `/dev/input`, or change compositor settings. Hyprland metadata remains the preferred inspector when available; the X11 aggregate fallback is used only for actual X11 sessions. The read-only Wayland probe is isolated in a short-lived subprocess with the existing three-second timeout and bounded output. Selecting an aggregate seat never offers a guessed XKB emulation target, and its input provenance remains `os:unknown`.

A read-only compositor query is triggered on startup, manual refresh, and focus-aware polling; generic Wayland seats are not presented as independently attributable keyboards. `wl_seat` may be absent or report no keyboard capability, in which case the inspector returns no seat rows and ordinary native typing remains available. Wayland's descriptive seat name is not guaranteed unique. Typefinch therefore distinguishes keyboard-capable seats using their `wl_registry` global IDs, accepting duplicate names while rejecting duplicate IDs. These identifiers are valid only for the current compositor instance and inspection context; they are **not** persistent physical-device identifiers or proof of key-event attribution.

[CI 40023774467](https://github.com/sguzman/typefinch/actions/runs/40023774467) passed 136 Rust tests, six synthetic Hyprland lifecycle cases, a native headless Weston `wl_seat` protocol query, X11 seat acceptance and the full graphical suite. **Weston advertised zero seats:** that proves real protocol connectivity and safe empty-seat handling, not live keyboard-equipped seat detection. The parser independently tests capability filtering and no active-layout attribution. True generic XKB active keymap detection, physical device routing and live Hyprland hardware acceptance remain open.

[CI 40034532510](https://github.com/sguzman/typefinch/actions/runs/40034532510) passed 147 Rust tests and the complete synthetic/native GUI suite. Under disposable Xvfb, **nested Weston with input enabled advertised one actual Wayland protocol seat with keyboard capability**; the Typefinch child probe and parent parser both accepted it without requesting `wl_keyboard` or collecting key events. A second nested Weston instance launched with `--no-input` still advertised one seat, but **without keyboard capability**; Typefinch correctly returned no inspected keyboards. These are native protocol-level virtual-seat tests, not physical keyboard detection, live Hyprland hotplug, event attribution, or verified active XKB map detection.

Protocol: [Wayland core wl_seat](https://wayland.app/protocols/wayland) and [Wayland specification](https://wayland.freedesktop.org/docs/html/apa.html).

**Acceptance update, 2026-10-23:** [CI 40032741759](https://github.com/sguzman/typefinch/actions/runs/40032741759) passed 147 Rust tests, eight synthetic Wayland parent/subprocess scenarios, six synthetic Hyprland inventory scenarios, actual read-only Weston protocol connectivity (0 keyboard-capable seats), native X11 seat inspection, and full X11 graphical QA. The suite covers duplicate Wayland seat names by registry ID, malformed seat names, incomplete Hyprland inventory rejection, stale/refreshing metadata guards, and disconnected compositor failures. These are software acceptance results, **not** live keyboard-equipped Wayland, physical Hyprland hotplug or per-device typing evidence.

## Read-only keyboard inspector CLI

`typefinch --inspect-keyboards` launches no GUI and reads the same bounded, read-only compositor metadata as the keyboard panel. It outputs JSON with `schema: 1`, `capture_policy: "inspection-only"`, and `rows`. Each row has a descriptive `label`, opaque compositor-local `inspection_id`, and nullable `reported_active_keymap`, `configured_layout`, and `configured_variant`. Null means that information is not available. The X11 layout describes configured groups, **not the active input group**; standard Wayland seats have no verified active keymap, even when keyboard-capable. IDs are not physical input sources.

When no supported compositor or X11 session is available, the command emits an error on stderr, exits nonzero, and prints no success JSON. It never subscribes to `wl_keyboard`, opens raw input devices, intercepts keys, writes settings, or accesses saved profiles and sessions.

[CI 40037472601](https://github.com/sguzman/typefinch/actions/runs/40037472601) passed 151 Rust tests and end-to-end CLI checks under actual nested Weston keyboard-capable and input-disabled virtual seats, an X11/Xvfb aggregate seat, and a disconnected session. The full native graphical suite passed separately in the same run. These tests do **not** establish live physical keyboard attribution, hardware hotplug, or active generic XKB seat detection.

## Keyboard-operated custom passage import

F11 or Ctrl+O opens the ordinary Custom passage import path editor with keyboard focus. Enter loads a regular UTF-8 file through the same bounded importer as the button and drag-and-drop. The action is unavailable during a scored session or settings-recovery mode. Neither the path nor the passage is saved to preferences. [CI 40040560022](https://github.com/sguzman/typefinch/actions/runs/40040560022) passed 155 Rust tests and native X11 QA: F11 focused the real import path editor, Enter loaded a synthetic 16-grapheme passage, and scoring saved 16/16 correctly as Custom without persisting the passage/path. A separate symlink import produced no scored session. It does not establish Hyprland keyboard-only accessibility.

## Native Tab and Enter keyboard-only profile navigation

Ctrl+Shift+T cycles the actual saved appearance selection **System → Light → Dark → System** without opening settings, and is disabled while a typing session is active or settings are in recovery mode. Ctrl+N focuses the real new-profile name field without creating anything automatically. Tab moves to the adjacent **Create profile** button; Enter submits through the same production UI validation path. F1 opens the visible keyboard guide. Dedicated X11 automation uses only keyboard actions for these controls, verifies a saved new profile, and checks that guide visibility changes the rendered frame. This supports keyboard accessibility while leaving a full real Hyprland/assistive-technology audit open.

## Keyboard-operated profile goals transfer

F12 focuses the normal per-profile local goals JSON path field; Enter invokes the regular validated import and persists only the selected profile's goals. Shift+F12 explicitly exports the current goals. Neither action runs during an active typing test, and protected settings cannot be overwritten. The same buttons remain usable by mouse. [CI 40041334789](https://github.com/sguzman/typefinch/actions/runs/40041334789) passed 155 Rust tests and native X11 QA for both shortcuts: imported goals persisted only selected targets, and exported versioned JSON matched those targets while identity, test counts and personal bests remained unchanged. These controls do not inspect physical keyboard hardware or establish live Hyprland acceptance.

## Private passage and goal transfer checks

Although not keyboard hardware functions, personal goals and imported private text must not accidentally become input telemetry. [CI 40025534946](https://github.com/sguzman/typefinch/actions/runs/40025534946) validates 141 Rust tests including goals-only format isolation across two local profiles, atomic file export, negative symbolic-link imports and complete extended-grapheme preservation for large emoji passages. Existing egui UI controls are implemented, but specific native GUI clicking/typing acceptance remains open rather than being credited as complete.
