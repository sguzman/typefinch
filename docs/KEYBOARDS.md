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

- **Implemented in source:** read-only `hyprctl -j devices` detects Hyprland devices, labels, and reported active keymap. A dropdown chooses the device *whose metadata to inspect*.
- **Not implemented:** selecting a keyboard as an exclusive input source. Ordinary Wayland `wl_keyboard` is seat-level and can aggregate multiple physical keyboards.
- **Future:** per-device event attribution only where a compositor-specific protocol or authorized opt-in evdev/libinput capture allows it. Never secretly capture keys or modify OS keyboard configuration.
- If Hyprland is unavailable, regular native typing remains available and a detection limitation is displayed.

Acceptance: hotplug two keyboards; map both with differing layouts; verify focus, shift/AltGr, symbols, dead keys, IMEs, ANSI/ISO/JIS and no double-count of physical + committed events.

[Wayland keyboard protocol](https://wayland.app/protocols/wayland) · [Hyprland devices](https://wiki.hypr.land/configuring/core/devices/) · [Hyprland keyboard-layout documentation](https://wiki.hypr.land/configuring/core/binds/keyboard-layouts/).

## XKB compiler (new implementation)

Where `xkeyboard-config` has a matching layout/variant, Typefinch can compile the installed keymap with `libxkbcommon` and remap common physical ANSI keys inside the application. This broadens *potentially emulatable* entries beyond the four built-in tables, with locale-specific symbols rather than guessed character mappings. Session-scoped XKB dead-key Compose now works for supported ANSI physical positions when the selected locale has a Compose table. This still is **not** complete emulation for physical AltGr, all Compose-key sequences, IMEs, ISO/JIS extras or per-device input attribution. An unavailable XKB preset remains recognition-only.

## Installed XKB registry discovery

At startup, Typefinch reads the local read-only xkeyboard-config rules inventory at `$XKB_CONFIG_ROOT/rules/evdev.lst` (default `/usr/share/X11/xkb/rules/evdev.lst`). It parses base layouts and named variants, deduplicates identities, and lets the user filter the installed list by name or ID alongside curated presets. Choosing an installed entry compiles its map through libxkbcommon where available. Registry discovery itself does not guarantee correct physical geometry, AltGr, Compose or IME support. A missing rules file disables extra discovery without preventing native typing.

## Session layout attribution

Typefinch records the emulated target identity for software-emulated sessions. OS-input sessions use a clearly prefixed `detected/<id>` identity only if all currently reported Hyprland keyboards agree on the active keymap and it resolves uniquely to an installed XKB entry. Otherwise Typefinch records `os:unknown` and excludes that session from layout-comparable trend charts. This is seat-level *best-effort* metadata, not proof of which physical keyboard generated a keystroke; changes after the session begins are not presently monitored. The layout context is snapshotted when a session starts.

## Refresh after hardware changes

Typefinch re-enumerates the compositor's read-only keyboard inventory when its window regains focus. The selected inspection device is preserved by exact Hyprland address when still present; otherwise selection falls back to the first available device (or none). This does not subscribe to device events while the app is unfocused, and it does not tie scored keyboard events to a selected physical device. There is no privileged global key capture.

## Personal ANSI layouts

The built-in editor creates version-1 JSON keymaps with a UUID-based `custom/<uuid>` identity, a 1–32-grapheme name, and exactly three rows of ten graphemes each for both base and Shift layers. Each physical ANSI key produces the corresponding mapped grapheme during *in-app emulation only*; Space remains Space, and Backspace remains the session's correction action. Editing is draft-based and validated before saving. Imports are explicitly selected local files (≤16 KiB); duplicate IDs get a fresh UUID and never overwrite existing mappings. Exported JSON is local and no external services are involved. This is not full ISO/JIS geometry, AltGr/Compose, or a system-level keyboard remapper.

## Read-only hotplug refresh

While Typefinch is open inside Hyprland, it refreshes the keyboard inventory on focus gain and approximately every 20 seconds while focused. Enumeration runs on a short-lived worker thread to avoid blocking typing. Keyboard selection survives reordering when the reported Hyprland address remains stable, but the selected device is only for inspection; no physical-device event interception or device attribution is implied. Outside Hyprland the application avoids background probing. Manual refresh remains available. The periodic timer does not run when the window is unfocused.

## Modifier-layer inspector

The preview exposes Base, Shift, AltGr and Shift+AltGr positions. XKB-backed layouts synthesize the right-Alt Level-3 modifier (evdev 100) for *read-only symbol inspection*, while the real input surface intentionally does not treat an undifferentiated left-Alt/right-Alt event as a trustworthy AltGr keystroke. Two-layer personal layouts have no inferred AltGr layer. Supported ANSI dead-key sequences use session-local Compose; physical AltGr, dedicated Compose-key routing, and complete ISO/JIS input remain unverified.

## Stateful Compose in scored ANSI emulation

For installed XKB layouts, a fresh libxkbcommon Compose state is associated with each scored emulation session. A dead key adds no scored character while pending. A completed sequence emits the composed UTF-8 string once; an invalid continuation drops the pending accent and passes through a printable cancelling key. Backspace cancels an uncommitted accent before deleting previously typed text. Session restart, Escape, and window-focus loss reset pending composition; user custom two-layer maps do not pretend to provide XKB Compose.

The existing native-mode committed-text path remains unchanged. If no usable locale Compose table is installed, emulation retains its previous non-composing path rather than inventing symbols. Automated coverage exercises Spanish dead acute + e, cancellation/reset, and one-grapheme scoring ([CI run 39985958620](https://github.com/sguzman/typefinch/actions/runs/39985958620)). Actual compositor AltGr attribution, ISO/JIS, IMEs and physical keyboard acceptance are still separate gates.

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
