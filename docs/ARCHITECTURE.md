# 🦀 Typefinch architecture

Typefinch is a **single standalone Rust application**, not a monorepo, with a native egui/eframe interface and an offline-first data model. It does not require network access, accounts on a remote service, or system-wide keyboard hooks.

## Module map

| Module | Responsibility |
| --- | --- |
| `app` | UI state, focus, navigation, profiles, settings, on-screen typing feedback |
| `engine` | Monotonic-session clock and deterministic WPM/accuracy calculations |
| `layouts` | Named preset catalog and four provisional ANSI fallback mappings |
| `xkb` | Compile installed system XKB keymaps and translate supported physical positions |
| `devices` | Read-only Hyprland keyboard listing and active-keymap metadata |
| `preferences` | System/Light/Dark theme, local profiles and settings JSON |
| `history` | Versioned SQLite sessions, profile isolation, stored scoring rules |
| `export` | Static escaped SVG profile card |
| `qa` | Explicit synthetic GUI-run assertions; inactive in ordinary use |

## Session contract

A session starts on F5 or Start and accepts input while the app window and typing surface own focus. Escape cancels. Window focus loss cancels without saving. Completed timed sessions with no entered characters are excluded from records. Ordinary input consumes compositor-committed text; software emulation uses physical-key positions, never modifies the global keyboard layout, and does not claim to identify which physical keyboard generated an ordinary Wayland event.

The current scoring version is `1`: correct-position Unicode *scalar values* divided by five per elapsed minute for WPM, and correctly positioned characters divided by entered characters for accuracy. Empty entry is 0% accurate. This is not yet grapheme-aware or IME-certified. Raw WPM, corrected-entry policy and per-language denominators are roadmap features.

## Persistence

`$XDG_CONFIG_HOME/typefinch/settings.json` (or `~/.config/typefinch/settings.json`) stores local profiles, UUID identities, layout and theme preferences. It is written through a temporary file and rename.

`$XDG_CONFIG_HOME/typefinch/sessions.sqlite3` stores records with a versioned schema (`PRAGMA user_version=2`). Columns include UUID-scoped profile ID, completion timestamp, layout, input mode, content type, test and elapsed duration, entered/correct counts, WPM, accuracy, corrections and scoring version. It stores **no raw text**, keystroke contents, profile nickname or keyboard hardware address. Database schema and migration paths have unit tests; backup/retention and recovery are still work in progress.

SVG export is explicit and writes to the local `exports/typefinch-profile.svg`; the user decides whether to publish it. Never put local settings or session databases into Git history.

## Keyboard capabilities

The static catalog is recognition-oriented. When the operating system provides the selected XKB symbols, `libxkbcommon` compiles their character mapping for a known subset of physical ANSI keys. AltGr, dead/compose sequences, ISO/JIS extras, complex IMEs and trusted per-device attribution are not complete. The device selector only inspects Hyprland metadata. See [KEYBOARDS.md](KEYBOARDS.md).

## QA boundary

GitHub Actions runs Cargo checks and unit tests, then builds a native Linux executable and launches it under Xvfb + Openbox + software OpenGL. `tests/gui_smoke.sh` injects synthetic typing with `xdotool` and asserts results through a temporary explicitly opt-in `TYPEFINCH_QA_REPORT` file plus the local SQLite record, then uploads a screenshot. This gives reproducible *X11 graphical* evidence, not real Hyprland hardware acceptance.

The [acceptance ledger](progress.json) is the only basis for percentages and its Markdown presentation must be regenerated from it.
