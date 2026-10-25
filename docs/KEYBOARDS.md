# 🗺️ Keyboard support and device selection

## Capability levels

1. **Recognized:** a curated layout identity in the selector, not necessarily installed in system XKB. The source catalog contains many presets from English, Spanish, German, French, other European, Cyrillic, Semitic and Asian layouts.
2. **Native OS input:** accept committed text from the compositor's layout. Keep this default for complex scripts and IME input; advanced tests remain to do.
3. **Software emulation:** interpret key positions through an alternate map inside Typefinch only. Four starter layouts have verified ANSI Base/Shift and outer punctuation without installed XKB. With production egui 0.35, a distinct physical RightAlt event routes XKB Level 3 and the ISO 102nd position is scoreable in virtual X11. JIS RO/YEN physical-position emulation has native X11 and nested Weston Wayland fixtures; other national keys, device-specific attribution, compositor variations and live Hyprland hardware QA remain pending.

| Layout | Identity in catalog | Starter emulation |
| --- | --- | --- |
| US QWERTY | ✅ | ✅ ANSI Base/Shift + edge punctuation |
| Dvorak | ✅ | ✅ ANSI Base/Shift + edge punctuation |
| Colemak | ✅ | ✅ ANSI Base/Shift + edge punctuation |
| Colemak-DH | ✅ | ✅ ANSI Base/Shift + edge punctuation |
| Workman, Norman, Halmak, Canary, Engram, Semimak | ✅ | XKB if installed; complete modifier/IME QA planned |
| QWERTZ, AZERTY, BÉPO, Neo 2, Turkish F | ✅ | XKB if installed; complete modifier/IME QA planned |
| Spanish, Latin American, Nordic, Cyrillic, Greek | ✅ | XKB if installed; complete modifier/IME QA planned |
| Arabic, Hebrew, Japanese, Korean and more | ✅ | Planned; IME/BiDi/geometry complexities |

The goal is full installed-XKB inventory via `xkeyboard-config` and `libxkbcommon`, plus custom layout editing, physical geometry, modifier and dead-key correctness. Do not falsely present conceptual named layouts as installed or emulatable.

## Production egui 0.35 physical-key routing — virtual X11

[Production CI 40235788506](https://github.com/sguzman/typefinch/actions/runs/40235788506) passed the complete locked native suite. Its physical-key fixture runs the actual Typefinch GUI on an isolated Xvfb/X11 seat with **host XKB set to US** and in-app emulation set to **German**. Real XTest key positions scored **6/6** expected graphemes: ordinary E, RightAlt+E (€), RightAlt+Q (@), ordinary E, ISO 102nd Base (<), and ISO 102nd Shift (>). A physical LeftAlt+E control did not add an attempt. SQLite reported the correct layout, Emulated input mode, and six correct attempts. Private source text and its path were absent from settings.

This is not the older OS-committed-text AltGr test: RightAlt has a distinct egui 0.35 event, not a guessed `Modifiers.alt` flag. The app synthesizes the XKB modifier only after observing RightAlt, and refuses to treat LeftAlt as AltGr. The ISO extra key maps to Linux evdev **86** / X11 **94**. Source and tests are in `src/app.rs`, `src/xkb.rs` and `tests/egui035_physical_emulation_smoke.sh`; the latter is a permanent production CI gate. An expanded
[production run 40236809646](https://github.com/sguzman/typefinch/actions/runs/40236809646)
passed an **8/8** physical-position sequence including RightAlt+ISO (`|`)
and German Shift+RightAlt+S (`ẞ`); LeftAlt remained non-scoring.
This accepts T303's scored XKB modifier and Compose criterion for its
specified virtual input paths, not T305's broader JIS or hardware routing.

The egui upgrade also exposed [upstream RTL glyph-index bug #8576](https://github.com/emilk/egui/issues/8576). Typefinch pins the narrowly patched third-party `epaint 0.35.0` source under `third_party/epaint-0.35.0` ([provenance and licenses](../third_party/epaint-0.35.0/UPSTREAM.md)). The locked native tests verify glyph/cursor extent and Arabic/Hebrew editing/Undo in X11 and nested Weston. No installed Cargo cache is patched during a production build.

**Scope:** this establishes host-independent German app emulation of the tested physical positions *inside a virtual X11 seat*, not a device-identified keyboard, arbitrary ISO/JIS regional keys, other compositor-specific modifier delivery, a privileged keyboard hook, or live Hyprland acceptance. The broader T305 and hardware gates remain open.

## Multiple keyboards

- **Implemented in source:** read-only `hyprctl -j devices` detects Hyprland inspection rows and reported active keymaps. Other Wayland compositors expose **aggregate `wl_seat` capability metadata and configured XKB keymap group names**, not the active group; genuine X11 sessions use `setxkbmap -query` for a single **configured aggregate seat**. None of these APIs attributes Typefinch typing events to physical keyboards.
- **Not implemented:** selecting a keyboard as an exclusive input source. Ordinary Wayland `wl_keyboard` is seat-level and can aggregate multiple physical keyboards.
- **Future:** per-device event attribution only where a compositor-specific protocol or authorized opt-in evdev/libinput capture allows it. Never secretly capture keys or modify OS keyboard configuration.
- If none of these inspectors is available, regular native typing remains available and the detection limitation is displayed.

Acceptance: hotplug two keyboards; map both with differing layouts; verify focus, shift/AltGr, symbols, dead keys, IMEs, ANSI/ISO/JIS and no double-count of physical + committed events.

[Wayland keyboard protocol](https://wayland.app/protocols/wayland) · [Hyprland devices](https://wiki.hypr.land/configuring/core/devices/) · [Hyprland keyboard-layout documentation](https://wiki.hypr.land/configuring/core/binds/keyboard-layouts/).

## Native JIS 109 and Menu-key reference geometry acceptance

[CI 40121775319](https://github.com/sguzman/typefinch/actions/runs/40121775319)
passed Rust source tests and the full native X11/Wayland GUI regression suite.
The keyboard preview now retains physical-reference Left/Right Super and Menu
positions on its JIS 109 model, and Menu on the ANSI, ISO and split main
blocks. The JIS bottom row models Ctrl, Super, Alt, Muhenkan, Space, Henkan,
Kana, Alt, Super, Menu, Ctrl in explicit physical order; no modifier or
conversion key is synthesized into scored input. The three Japanese
conversion keys have readable `Non`, `Conv` and `Kana` fallback labels
when installed XKB provides no printable character.

Archived `typefinch-geometry-jis.png`, `typefinch-geometry-iso.png` and
`typefinch-geometry-split.png` from that successful CI run were opened and
visually inspected: keys are visible within the native preview, with no
bottom-row clipping. The Lua native GUI fixture checks the revised slot,
modifier and control counts, including shape cycling and Spanish-ISO
selection. These are **illustrated, unscored layout references**, not
physical ISO/JIS event routing or complete modifier/IME acceptance.
T305's remaining acceptance checks stay open.

## Nested Weston native Wayland GUI acceptance

[CI 40119656114](https://github.com/sguzman/typefinch/actions/runs/40119656114)
passed the complete native source and GUI regression suite, including
a new **actual Wayland GUI** fixture. Disposable Xvfb virtual keyboard
events were routed through Weston 13's X11 backend into the production
Typefinch egui surface over Wayland. The normal keyboard-only F11 import,
F5 start and F2 typing-focus path scored the synthetic `abc` passage
as **3/3 correct** in about 0.6 seconds. SQLite recorded one Custom
result with `input_mode=OS` and **`layout=os:unknown`**, since a
standard Wayland aggregate seat does not expose the active XKB keymap.
Neither the imported passage nor the temporary local file path was
persisted in settings.

The archived `typefinch-wayland-virtual.png` image was opened and
visually inspected. It shows the actual completed native Wayland
Typefinch window, correctly sized controls, green completed reference,
a 100% progress bar and no double-scored input. The first two
integration attempts failed solely while locating Weston's outer X11
window. The accepted fixture finds that window by compositor PID,
class or title before sending input, and retains diagnostics if that
discovery fails. **This is Weston/Xvfb virtual-input acceptance only:**
it does not establish live Hyprland compatibility, a physical keyboard
identity, generic active-XKB-seat detection or a native IME candidate
overlay. T001, T403 and T902 remain below 100%.

## Starter ANSI Base/Shift and offline fallback

[CI 40118806315](https://github.com/sguzman/typefinch/actions/runs/40118806315)
passed the complete Rust and native X11 GUI suite, including 48 physical
ANSI positions (three ten-key letter rows, 17 edge keys, Space) at both
Base and Shift levels for each of **US QWERTY, Dvorak, Colemak and
Colemak-DH**, compared against installed libxkbcommon keymaps. The
previous Colemak-DH bottom-row transposition was corrected; Dvorak's
number-row-adjacent and punctuation key differences now retain their
actual shifted/unshifted symbols.

An independent native GUI regression uses physical X11 keycodes through
Typefinch's own **software emulation** and requires Dvorak `[]{}_?`
(6/6) plus Colemak-DH `xcdvz` (5/5). It repeats both fixtures with
`XKB_CONFIG_ROOT` deliberately pointing to an empty directory,
forcing the compiled-XKB path to be unavailable and exercising the
explicit starter-table fallback. All four scored records identify
`input_mode=Emulated`; source passages and local paths remain out of
saved settings. This is reproducible virtual X11 QA, not a claim about
physical keyboard identity, AltGr attribution, compositor-specific
modifier routing or complete ISO/JIS layouts. T302 and T303 therefore
remain partially unaccepted.

## XKB compiler (new implementation)

Where `xkeyboard-config` has a matching layout/variant, Typefinch can compile the installed keymap with `libxkbcommon` and remap common physical ANSI keys inside the application. This broadens *potentially emulatable* entries beyond the four built-in tables, with locale-specific symbols rather than guessed character mappings. Session-scoped XKB dead-key Compose now works for supported ANSI physical positions when the selected locale has a Compose table. This still is **not** complete emulation for all physical modifier/Compose sequences, IMEs, national keys beyond the tested ISO 102nd and JIS RO/YEN positions, or per-device attribution. An unavailable XKB preset remains recognition-only.

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

## Native X11 Shift and AltGr input verification

[CI 40114330744](https://github.com/sguzman/typefinch/actions/runs/40114330744)
passed a **real Xvfb/egui system-input** fixture under the installed German
XKB layout. The disposable OS seat received `Shift+1`, Space and
`ISO_Level3_Shift+E` (Right Alt/AltGr), and Typefinch scored the resulting
`! €` as exactly **3/3 correct committed graphemes** with three attempts.
The independent SQLite assertion verified `input_mode=OS` and
`test_mode=custom`; no fixture passage/path was persisted in settings.
This adds an actual virtual-X11 compositor-input regression beyond the
static AltGr symbol-inspection tests.

A second disposable process in [CI 40114996096](https://github.com/sguzman/typefinch/actions/runs/40114996096)
set the actual X11 keyboard layout to Spanish (`es`), sent physical
XKB keycodes for **dead acute, then E**, and verified that ordinary
OS-input mode committed exactly **one `é` grapheme with one scored
attempt**, never scoring the uncommitted accent by itself. The SQLite
record used `input_mode=OS` and `test_mode=custom`. The complete
graphical regression suite passed. This is standard X11 dead-key
composition, not a live Fcitx/IBus CJK IME or the app's full software
AltGr/Compose feature gate.

**Historical boundary of these older OS-input fixtures:** they did not test
Typefinch's in-app LeftAlt/RightAlt distinction, Shift+AltGr layers,
real keyboard hardware, production Hyprland or generic IME composition.
A later native in-app emulation run now covers the former two under virtual X11.
T303's app-local XKB Shift/AltGr/Compose criterion is accepted with the later production egui 0.35 virtual-X11 evidence; see the production section above. Real Hyprland/IME and per-device input remain separate open gates.

## Illustrative ANSI, ISO, JIS and split previews

The preview now lets the user choose among four **explicit illustrative main typing blocks**, without inferring a keyboard's hardware shape. ANSI/ISO/JIS labels name keyboard families. Together with the F10 function/navigation and keypad reference views, the sections model all distinct standard **104 ANSI / 105 ISO / 109 JIS** Linux keycode positions (104 for the split ANSI illustration). These are separate illustrations, not an assembled, detected physical keyboard, and do not imply event attribution. Key slots are derived from named Linux evdev positions: ISO `KEY_102ND=86`, Japanese `KEY_RO=89`, `KEY_YEN=124`, `KEY_MUHENKAN=94`, `KEY_HENKAN=92`, and `KEY_KATAKANAHIRAGANA=93` ([Linux input-event codes](https://github.com/torvalds/linux/blob/master/include/uapi/linux/input-event-codes.h)).

The UI can inspect installed XKB symbols for extra slots by keycode without guessing input from a visually labeled keycap. Production egui 0.35 now routes the ISO 102nd event as `IntlBackslash` and JIS RO/YEN as `IntlRo`/`IntlYen` through pinned egui/winit bridge sources. [Production CI 40242423798](https://github.com/sguzman/typefinch/actions/runs/40242423798) passed 254 Rust tests and its genuine Xvfb/XTest JIS fixture: RO/YEN Base/Shift scored 4/4 under a US host layout with Japanese app emulation, while Zenkaku/Hankaku, conversion, non-conversion and Kana controls produced no scored text. [CI 40242737495](https://github.com/sguzman/typefinch/actions/runs/40242737495) repeated these native checks and passed optimized packaging with pinned, licensed egui/winit bridge sources. [CI 40245001521](https://github.com/sguzman/typefinch/actions/runs/40245001521) added the corresponding **real nested-Weston `wl_keyboard` path**: JIS RO/YEN Base/Shift scored 4/4 with Japanese app emulation, conversion/mode controls stayed unscored, and 254 Rust tests plus the full native GUI suite passed. The new JIS Wayland runner uses the existing narrowly tested external-Weston pre-score SIGSEGV retry guard; scoring assertions are never retried as compositor crashes. Both fixtures use disposable virtual input, not live connected Japanese keyboard/Hyprland or per-device proof. Split layout is **visual separation** with exactly one scored Space identity; a second decorative space half is not a second physical input. Shape selection affects only the preview and is not a persisted hardware configuration. The underlying geometry model and duplicate/non-attribution tests passed [native CI 39992682545](https://github.com/sguzman/typefinch/actions/runs/39992682545) with 111 Rust tests; the expanded synthetic X11 geometry report passed [CI 39994019022](https://github.com/sguzman/typefinch/actions/runs/39994019022) with 112 tests. Full visual screenshot inspection and physical extra-key routing remain separate acceptance checks.

## Keyboard-shape preview acceptance

The modeled **main typing block** was visually checked against actual native X11 captures from [Actions 39997672154](https://github.com/sguzman/typefinch/actions/runs/39997672154) (113 passing Rust tests). ISO, JIS, and split rows fit the 1100×850 inspection window after disabling extra egui item spacing and reducing the geometry unit. The Japanese preview uses the Japanese XKB target during synthetic QA, with the ordinary profile unchanged. The shape selector is presentation-only; F4 cycles ANSI → ISO → JIS → Split, and F3 independently cycles inspected XKB modifier levels.

The typing blocks are only one of three illustrated regions; full standard keyboard-family **reference inventories** are verified separately across typing, function/navigation and keypad views. Connected physical keyboard shapes, trusted ISO/JIS event routing and real Hyprland acceptance remain open. The synthetic X11 checks assert modeled slot counts and that unmappable physical keys never become guessed egui key events; the images are also archived with the CI run.

## Control-key and modifier previews

Core Backspace, Tab, Enter, Caps Lock, left/right Shift, Ctrl and Alt (plus Super on ANSI/ISO/split) are explicit labeled positions in `src/geometry.rs`. Each carries its own Linux evdev code and semantic role. The UI renders readable text captions rather than unsupported Unicode keycap symbols; modifiers and controls never enter the scored-character translation path or pretend left/right physical-event attribution. JIS RO/YEN and conversion/mode keys now have distinct egui physical identities, without claiming actual connected Japanese hardware attribution. The main-block drawing is separate from F10's navigation/keypad views; the complete standard illustrative JIS 109 inventory is now covered across those sections, but no Japanese hardware event routing is claimed.

Automated checks reject overlapping positions and duplicate evdev identities, verify printable geometry matches the XKB emulation-keycode map across all four shapes, and distinguish modifier/control positions from unassigned extra keys. [CI 40003691924](https://github.com/sguzman/typefinch/actions/runs/40003691924) passed 116 Rust tests, physical-key typing QA and native ISO/JIS/split screenshot capture. All three screenshots were visually reviewed for readable captions and no clipped positions. The standard full-family **reference inventory** is now accepted, while trusted extra-key routing, exotic/nonstandard shapes and Hyprland hardware QA remain open.

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

In a non-Hyprland Wayland session (declared `XDG_SESSION_TYPE=wayland` and a Wayland socket), Typefinch queries the standard `wl_registry`/`wl_seat` interfaces through Rust `wayland-client`. It obtains **seat names and keyboard-capability flags**. A separate short-lived **unfocused** internal inspector also requests `wl_keyboard` solely to receive the offered XKB keymap descriptor and extract its human-readable group names. It does **not** create a Wayland surface, read/log/score key events, derive individual physical devices, open `/dev/input` or change compositor settings. It ignores every non-keymap keyboard event rather than treating it as input. The previous capability-only `--_probe_wayland_seats` diagnostic still never binds `wl_keyboard`. Hyprland metadata remains preferred when available, and X11 aggregate fallback is used only in X11 sessions. The inspector is bounded by a three-second subprocess deadline, at most 16 XKB groups per map and four MiB of descriptor data; only bounded seat/group labels leave the child. Configured groups are **not the active group**, are not valid physical input attribution, cannot become an emulation target via inferred layout, and all generic Wayland OS-input scores remain `os:unknown`.

A read-only compositor query is triggered on startup, manual refresh, and focus-aware polling; generic Wayland seats are not presented as independently attributable keyboards. `wl_seat` may be absent or report no keyboard capability, in which case the inspector returns no seat rows and ordinary native typing remains available. Wayland's descriptive seat name is not guaranteed unique. Typefinch therefore distinguishes keyboard-capable seats using their `wl_registry` global IDs, accepting duplicate names while rejecting duplicate IDs. These identifiers are valid only for the current compositor instance and inspection context; they are **not** persistent physical-device identifiers or proof of key-event attribution.

[CI 40023774467](https://github.com/sguzman/typefinch/actions/runs/40023774467) passed 136 Rust tests, six synthetic Hyprland lifecycle cases, a native headless Weston `wl_seat` protocol query, X11 seat acceptance and the full graphical suite. **Weston advertised zero seats:** that proves real protocol connectivity and safe empty-seat handling, not live keyboard-equipped seat detection. The parser independently tests capability filtering and no active-layout attribution. True generic XKB active keymap detection, physical device routing and live Hyprland hardware acceptance remain open.

[CI 40034532510](https://github.com/sguzman/typefinch/actions/runs/40034532510) passed 147 Rust tests and the complete synthetic/native GUI suite. Under disposable Xvfb, **nested Weston with input enabled advertised one actual Wayland protocol seat with keyboard capability**; the Typefinch child probe and parent parser both accepted it without requesting `wl_keyboard` or collecting key events. A second nested Weston instance launched with `--no-input` still advertised one seat, but **without keyboard capability**; Typefinch correctly returned no inspected keyboards. These are native protocol-level virtual-seat tests, not physical keyboard detection, live Hyprland hotplug, event attribution, or verified active XKB map detection.

Protocol: [Wayland core wl_seat](https://wayland.app/protocols/wayland) and [Wayland specification](https://wayland.freedesktop.org/docs/html/apa.html).

**Acceptance update, 2026-10-23:** [CI 40032741759](https://github.com/sguzman/typefinch/actions/runs/40032741759) passed 147 Rust tests, eight synthetic Wayland parent/subprocess scenarios, six synthetic Hyprland inventory scenarios, actual read-only Weston protocol connectivity (0 keyboard-capable seats), native X11 seat inspection, and full X11 graphical QA. The suite covers duplicate Wayland seat names by registry ID, malformed seat names, incomplete Hyprland inventory rejection, stale/refreshing metadata guards, and disconnected compositor failures. These are software acceptance results, **not** live keyboard-equipped Wayland, physical Hyprland hotplug or per-device typing evidence.

## Read-only keyboard inspector CLI

`typefinch --inspect-keyboards` launches no GUI and reads the same bounded, read-only compositor metadata as the keyboard panel. It outputs JSON with `schema: 1`, `capture_policy: "inspection-only"`, and `rows`. Each row has a descriptive `label`, opaque compositor-local `inspection_id`, nullable `reported_active_keymap`, `configured_layout`, `configured_variant`, and a `seat_xkb_groups` array (the offered keymap groups; empty for other inspector backends). Null means that information is not available. The X11 layout describes configured groups, **not the active input group**; standard Wayland seats have no verified active keymap, even when keyboard-capable. IDs are not physical input sources.

When no supported compositor or X11 session is available, the command emits an error on stderr, exits nonzero, and prints no success JSON. It never creates a focused Wayland surface, reads raw input devices, records typing events, writes settings, or accesses saved profiles and sessions. Generic Wayland inspection briefly binds an unfocused `wl_keyboard` for the keymap descriptor only; this is **not** an active keyboard-event collector.

[CI 40037472601](https://github.com/sguzman/typefinch/actions/runs/40037472601) passed 151 Rust tests and end-to-end CLI checks under actual nested Weston keyboard-capable and input-disabled virtual seats, an X11/Xvfb aggregate seat, and a disconnected session. The full native graphical suite passed separately in the same run. These tests do **not** establish live physical keyboard attribution, hardware hotplug, or active generic XKB seat detection.

## Keyboard-operated custom passage import

F11 or Ctrl+O opens the ordinary Custom passage import path editor with keyboard focus. Enter loads a regular UTF-8 file through the same bounded importer as the button and drag-and-drop. The action is unavailable during a scored session or settings-recovery mode. Neither the path nor the passage is saved to preferences. [CI 40040560022](https://github.com/sguzman/typefinch/actions/runs/40040560022) passed 155 Rust tests and native X11 QA: F11 focused the real import path editor, Enter loaded a synthetic 16-grapheme passage, and scoring saved 16/16 correctly as Custom without persisting the passage/path. A separate symlink import produced no scored session. It does not establish Hyprland keyboard-only accessibility.

## Native Tab and Enter keyboard-only profile navigation

Ctrl+Shift+T cycles the actual saved appearance selection **System → Light → Dark → System** without opening settings, and is disabled while a typing session is active or settings are in recovery mode. Ctrl+N focuses the real new-profile name field without creating anything automatically. Tab moves to the adjacent **Create profile** button; Enter submits through the same production UI validation path. F1 opens the visible keyboard guide. Dedicated X11 automation uses only keyboard actions for these controls, verifies a saved new profile, and checks that guide visibility changes the rendered frame. This supports keyboard accessibility while leaving a full real Hyprland/assistive-technology audit open.

## Keyboard-operated profile goals transfer

F12 focuses the normal per-profile local goals JSON path field; Enter invokes the regular validated import and persists only the selected profile's goals. Shift+F12 explicitly exports the current goals. Neither action runs during an active typing test, and protected settings cannot be overwritten. The same buttons remain usable by mouse. [CI 40041334789](https://github.com/sguzman/typefinch/actions/runs/40041334789) passed 155 Rust tests and native X11 QA for both shortcuts: imported goals persisted only selected targets, and exported versioned JSON matched those targets while identity, test counts and personal bests remained unchanged. These controls do not inspect physical keyboard hardware or establish live Hyprland acceptance.

## Private passage and goal transfer checks

Although not keyboard hardware functions, personal goals and imported private text must not accidentally become input telemetry. [CI 40025534946](https://github.com/sguzman/typefinch/actions/runs/40025534946) validates 141 Rust tests including goals-only format isolation across two local profiles, atomic file export, negative symbolic-link imports and complete extended-grapheme preservation for large emoji passages. Existing egui UI controls are implemented, but specific native GUI clicking/typing acceptance remains open rather than being credited as complete.

## Full family reference-inventory acceptance

[GitHub Actions 40134673256](https://github.com/sguzman/typefinch/actions/runs/40134673256) passed **224 Rust tests** and the complete native X11 graphical suite. The new `full_ansi_iso_jis_and_split_reference_inventories_are_exact` regression checks Linux evdev code uniqueness over the **typing**, **function/navigation** and **keypad** sections, with **104 distinct ANSI**, **105 ISO**, **109 JIS**, and **104 split ANSI** physical-code *reference slots*. The split second Space drawing carries code zero and cannot double-count a real key. ISO 102nd and JIS Ro/Yen have readable fallback captions, and right Alt is distinguishable from left Alt without claiming reliable physical AltGr input. A second regression rejects fake printable egui events for those national edge keys. Archived 1100×850 native ISO, JIS and split screenshots were independently opened and inspected for layout and readable keycaps; previous F10 navigation/keypad screenshots were inspected in CI 40006834967. The acceptance is a **software-rendered reference model**, not physical device shape identification or actual ISO/JIS event routing.

## Hosted headless Hyprland compositor QA boundary

Two deliberately isolated experimental GitHub Actions runs tried to open the
actual Typefinch Wayland GUI under real Hyprland in an Ubuntu 26.04 container:
[40138733735](https://github.com/sguzman/typefinch/actions/runs/40138733735)
(stand-alone headless software renderer) and
[40139025545](https://github.com/sguzman/typefinch/actions/runs/40139025545)
(nested behind a working software-rendered Weston 14 parent). Ubuntu provided
Hyprland 0.53.3, but both stopped **before** Typefinch started because
Hyprland/Aquamarine threw `CBackend::create() failed` without a usable
GPU/render allocator. Weston itself initialized the headless output in the
second attempt. Neither run demonstrates a failing typing or rendering path
inside Typefinch. No screenshots were produced and **no T001/T902 or hardware
acceptance is credited**.

The isolated opt-in workflow `.github/workflows/hyprland-headless.yml` is
kept as a documented reproducible probe, not a required passing CI gate.
Do not keep repeating it on the same GPU-less hosted runner. A next attempt
needs a functioning renderer (or an authorized graphical Hyprland test runner)
and must preserve independent screenshots, keyboard and focus evidence without
touching the user's active workspace, global input devices or private data.

## Explicit XKB Level-3 Compose path (verified in isolated Rust tests)

Commit [`a2094fc`](https://github.com/sguzman/typefinch/commit/a2094fc82b6bea8e2afc5a3de82a2a691fa0fcf1)
adds `Emulator::translate_composed_layer` for an **explicitly supplied**
Level-3 modifier state, while the existing scored egui input path retains
its conservative non-Level-3 wrapper. [CI 40141102036](https://github.com/sguzman/typefinch/actions/runs/40141102036)
passed all 230 Rust tests and the full graphical suite. The scoped source
regression verifies German E/Q Level-3 euro/at output and a 4/4-correct
Unicode scoring sequence. This is XKB-symbol and explicit-modifier
translation acceptance, not physical AltGr input acceptance.

The application does **not** infer Right Alt from egui's generic `alt`
flag, which may represent Left Alt or AltGr depending on the compositor.
Reliable physical Right-Alt/AltGr event attribution, scored native in-app
modifier routing and T303's full Shift/AltGr/dead-key acceptance remain open.

## Four complete English starter ANSI base/Shift layers — accepted

[GitHub Actions 40143940280](https://github.com/sguzman/typefinch/actions/runs/40143940280)
passed **230 Rust tests and the complete native graphical suite** on
[`99acb6c`](https://github.com/sguzman/typefinch/commit/99acb6c7278f399051fc638e8d8b9a61579822cc).
The native X11 test exercises actual physical-position key events into the
unmodified Typefinch GUI, with one complete 21-key number/edge punctuation
sequence at Base and again at Shift, for **42/42 correct scored graphemes**
in each of eight isolated cases: US QWERTY, Dvorak, Colemak and Colemak-DH,
each with libxkbcommon and with its keymap library intentionally unavailable
(forced offline built-in tables). It also presses Ctrl, Left Alt and
Ctrl+Shift shortcuts ahead of scoring and requires unchanged final attempts,
correctness, per-session SQLite input provenance and no private passage/path
persistence. An existing independent Rust parity test checks every modeled
ANSI base/Shift key, 48 positions per starter map, against installed XKB.

Eight screenshots are archived in the `typefinch-gui-smoke` artifact under
`typefinch-starter-full-*.png`. Representative US, Dvorak, Colemak and
Colemak-DH captures were opened and inspected: native score 42/42, full
progress bar, active input target identity and matching keyboard preview.
The synthetic fixtures contain no personal typing or hardware identifiers.

**Boundary:** This accepts the four **starter software** layouts' explicit
ANSI Base/Shift and punctuation behavior and suppression of ordinary
shortcut-modified letters. It is **not** physical Right Alt/AltGr
attribution, Caps Lock behavior on real keyboards, arbitrary ISO/JIS
national-edge routing or a real Hyprland device/IME test. Those independent
T303/T305/hardware criteria remain open. There is no global key interception.

## Native Wayland compositor-inventory hotplug lifecycle — accepted (synthetic metadata)

[CI 40146202012](https://github.com/sguzman/typefinch/actions/runs/40146202012)
passed **230 Rust tests and the entire native GUI suite** at
[`4fdfc36`](https://github.com/sguzman/typefinch/commit/4fdfc36f291a072f70b75f9f67cf129422a82c88).
The production Typefinch egui UI remained open as a **real Wayland client**
under a disposable Weston/Xvfb compositor. Only its `hyprctl -j devices`
metadata provider was replaced by a synthetic, fixed-argument test fixture.
The same background-inspector poll and recovery path as production (normally
20 seconds; 300 ms strictly within the explicit synthetic QA process)
observed six successive states without manual interaction:

- One reported keyboard with a unique recognized XKB map: fresh, known.
- A second keyboard with a conflicting map: two fresh entries, no trustworthy
  single active layout attribution.
- Removal of the first keyboard: one fresh recognized map.
- All keyboards removed: empty inventory, unknown layout.
- Malformed compositor JSON: cleared/unverified inventory, no stale trust.
- Healthy keyboard reappears: fresh inventory, known unique map.

The real UI reported only counts, selected index, freshness and a boolean
indicating whether a unique installed layout matched; it did **not** log
addresses, keycodes, raw events or typed text. Six screenshots of the live
1024x768 disposable GUI are archived as `typefinch-hotplug-*.png`.
The attached-state image was manually inspected to confirm the real app
remained rendered; inventory transitions are proven by QA metadata from its
actual asynchronous GUI state, not by screenshot text or simulated editor
logic. Unit tests independently verify selected inspection identity remains
stable through reordering and resets safely after removal.

**Scope:** T402's software **hotplug support** is accepted; this is *not*
evidence of a real physical keyboard being attached or removed under Hyprland,
per-device keyboard attribution, an opt-in key-event capture backend or real
Hyprland visual QA. T401, T404 and T902 remain explicitly unaccepted.

## Generic Wayland XKB keymap group discovery — accepted

[GitHub Actions 40148518400](https://github.com/sguzman/typefinch/actions/runs/40148518400)
passed **231 Rust tests and the complete native graphical suite** on
[`52f69cd`](https://github.com/sguzman/typefinch/commit/52f69cdbb8cf9ffc1845cdf00f3c9c36fd9ed354).
Under isolated **nested Weston/Xvfb**, the new
`typefinch --_probe_wayland_xkb_seats` executable queried a
keyboard-capable virtual `wl_seat` and returned the compositor's actual
offered **XKB keymap group labels**. The ordinary
`typefinch --inspect-keyboards` CLI independently exposed the same group's
names in the `seat_xkb_groups` field, while
`reported_active_keymap` remained null. Synthetic tests retained their
eight malformed, duplicate, timeout and disconnected-compositor scenarios.
The legacy `--_probe_wayland_seats` capability-only mode still never requests
`wl_keyboard`, and the no-input virtual Weston seat still produces zero
inspected keyboard rows.

The keymap inspector uses a separate short-lived Wayland connection with
**no input surface**, accepts only an XKB text-v1 keymap descriptor,
compiles a bounded four-MiB-or-less keymap in the child, validates at most
16 group names (max 128 bytes each), and returns only seat/group metadata.
Every non-keymap keyboard protocol event is ignored without inspecting or
storing it. The parent enforces its independent three-second deadline and
16-KiB response size limit. Invalid keymaps fail closed and do not preserve
stale trusted metadata.

This satisfies **generic XKB seat *keymap* detection**, NOT active-group
detection. Wayland's independent unfocused metadata client cannot establish
the active group or which physical keyboard generated typed events.
No `detected/*` label is inferred for generic Wayland typing: scored OS input
remains `os:unknown`. Real physical-device attribution, physical Hyprland
hotplug, and optional compositor-authorized per-device capture remain open.

## Hosted experimental Hyprland limitations (October 25, 2026)

The experimental `.github/workflows/hyprland-headless.yml` tried to
launch real **Hyprland 0.53.3** in an isolated Ubuntu 26.04 container,
rather than accepting synthetic `hyprctl` fixtures as evidence.

- [Experimental run 40203842796](https://github.com/sguzman/typefinch/actions/runs/40203842796)
  failed before opening Typefinch: Aquamarine could not establish a backend
  with a Pixman-only Weston parent.
- [Experimental run 40204104287](https://github.com/sguzman/typefinch/actions/runs/40204104287)
  failed even with Mesa llvmpipe and a Weston GL parent. Aquamarine found
  no usable DRM/KMS device, could not open `/dev/dri/card1` and also
  encountered a parent compositor `wl_compositor` protocol version mismatch
  (advertised v5, requested v6). The log reports `CBackend::create() failed`.

Both are **external runner/compositor initialization failures**. Neither
execution reached the Typefinch application, therefore neither proves a
Typefinch defect nor passes its live Hyprland gates.

A distinct [labwc software-GLES experiment 40244600862](https://github.com/sguzman/typefinch/actions/runs/40244600862)
likewise stopped before Hyprland or Typefinch opened: wlroots 0.19/labwc 0.9.3
reported `drmGetDevices2 failed` and `Cannot create GLES2 renderer: no DRM FD
available`. The alternate headless-parent attempt was reverted, and the
workflow remains manually dispatchable only. No repeated GPU-less hosted
attempt is warranted by this evidence. The experimental
workflow remains manually dispatchable but is not a required on-push gate
on hosted runners known to lack the necessary allocator/device/protocol
support. Further virtual validation needs a compatible render-capable host;
real connected keyboards, physical-device attribution, native compositor
IME handling and system theme transitions still need appropriately
authorized live acceptance. Standard X11 and nested Weston CI stay separate.

## 🖥️ Opt-in local Hyprland acceptance handoff

`tests/live_hyprland_acceptance.sh` prepares the existing T001/T902 live-foreground QA for an authorized **local Codex agent**, not a human doing manual QA. It requires an actual Hyprland session and explicit `TYPEFINCH_QA_CONSENT=1`; it never runs automatically on a local machine. The script compiles the locked app, launches with disposable XDG state, proves its window PID/focus through real Hyprland IPC, and dispatches F1 only to that window. It compares two window-bounded local screenshots and restores prior focus. Evidence is deleted by default; an optional already-existing `TYPEFINCH_QA_OUTPUT_DIR` keeps a screenshot locally, never uploads it. The CI `--self-test` covers only pure JSON parsing and does not claim live acceptance.

This does **not** demonstrate a physical keyboard event, per-device attribution, actual system-theme transition, arbitrary IME behavior or global input capture. Those gates remain open until a local agent executes and independently verifies them. No root, `/dev/input` or unconsented access is required.
