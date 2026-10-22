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
| Workman, Norman, Halmak, Canary, Engram, Semimak | ✅ | Planned |
| QWERTZ, AZERTY, BÉPO, Neo 2, Turkish F | ✅ | Planned |
| Spanish, Latin American, Nordic, Cyrillic, Greek | ✅ | Planned |
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

Where `xkeyboard-config` has a matching layout/variant, Typefinch can compile the installed keymap with `libxkbcommon` and remap common physical ANSI keys inside the application. This broadens *potentially emulatable* entries beyond the four built-in tables, with locale-specific symbols rather than guessed character mappings. This still is **not** complete emulation for AltGr, dead-key Compose, IMEs, ISO/JIS physical extras or per-device input attribution. An unavailable XKB preset remains recognition-only.
