# 🦀 Typefinch architecture

Typefinch is a **single standalone Rust application**, not a monorepo, with a native egui/eframe interface and an offline-first data model. It does not require network access, accounts on a remote service, or system-wide keyboard hooks.

## Module map

| Module | Responsibility |
| --- | --- |
| `app` | UI state, focus, navigation, profiles, settings, on-screen typing feedback |
| `engine` | Monotonic-session clock and deterministic WPM/accuracy calculations |
| `layouts` | Named preset catalog and four provisional ANSI fallback mappings |
| `xkb` | Compile installed system XKB keymaps, translate supported physical positions, and apply session-scoped dead-key Compose |
| `devices` | Read-only Hyprland keyboard listing or a guarded aggregate X11 seat query; never key capture |
| `preferences` | System/Light/Dark theme, local profiles and settings JSON |
| `history` | Versioned SQLite sessions, profile isolation, stored scoring rules |
| `export` | Static SVG/Markdown cards, JSON history export and explicit private backups |
| `qa` | Explicit synthetic GUI-run assertions; inactive in ordinary use |

## Session contract

A session starts on F5 or Start and accepts input while the app window and typing surface own focus. Escape cancels. Window focus loss cancels without saving. Completed timed sessions with no entered characters are excluded from records. Ordinary input consumes compositor-committed text; software emulation uses physical-key positions, never modifies the global keyboard layout, and does not claim to identify which physical keyboard generated an ordinary Wayland event.

The current scoring version is `2`: final WPM and accuracy compare Unicode extended grapheme clusters after NFC canonical normalization. WPM divides correctly positioned graphemes by five per elapsed minute; accuracy divides correct graphemes by entered graphemes, with 0% for an empty entry. Backspace removes an entire grapheme. A short grace period permits a final accent to arrive in a separate committed-text event. Raw WPM and accuracy instead count input scalar attempts, so raw and final metrics intentionally have different denominators. Older version-1 records remain intact and never merge into version-2 trends. Complex IME, RTL and locale-specific speed conventions still require validation.

## Persistence

`$XDG_CONFIG_HOME/typefinch/settings.json` (or `~/.config/typefinch/settings.json`) stores local profiles, UUID identities, layout and theme preferences. It is written through a temporary file and rename.

`$XDG_CONFIG_HOME/typefinch/sessions.sqlite3` stores records with a versioned schema (`PRAGMA user_version=3`). Columns include UUID-scoped profile ID, completion timestamp, layout, input mode, content type, test and elapsed duration, entered/correct counts, WPM, raw WPM, final and raw accuracy, total attempts, mistakes, corrections and scoring version. It stores **no raw text**, keystroke contents, profile nickname or keyboard hardware address. Schema migrations, unbounded history exports and consistent SQLite snapshots have tests. F8 creates an explicit settings/SQLite backup pair; automatic restore and retention controls remain unfinished.

SVG export is explicit and writes to the local `exports/typefinch-profile.svg`; the user decides whether to publish it. Never put local settings or session databases into Git history.

## Keyboard capabilities

The static catalog is recognition-oriented. When the operating system provides the selected XKB symbols, `libxkbcommon` compiles their character mapping for a known subset of physical ANSI keys. Installed XKB layouts now support session-local dead-key Compose for recognized ANSI positions when a locale table is available. Physical AltGr, dedicated Compose-key routing, ISO/JIS extras, complex IMEs and trusted per-device attribution are not complete. The device selector inspects Hyprland device metadata or one explicitly aggregate X11 seat, never per-device key input. See [KEYBOARDS.md](KEYBOARDS.md).

## QA boundary

GitHub Actions validates the progress ledger with Lua, runs Cargo checks and unit tests, then builds a native Linux executable and launches it under Xvfb + Openbox + software OpenGL. `tests/gui_smoke.sh` injects synthetic typing with `xdotool` and asserts results through a temporary explicitly opt-in `TYPEFINCH_QA_REPORT` file plus the local SQLite record, then uploads a screenshot. This gives reproducible *X11 graphical* evidence, not real Hyprland hardware acceptance.

The [acceptance ledger](progress.json) is the only basis for percentages and its Markdown presentation must be regenerated from it.

## Lesson mastery

A finished untimed lesson is recorded as an attempt. Only records with nonzero entry, full final-position correctness, 100% final accuracy and zero prescribed duration count toward course completion. Corrected mistakes affect raw metrics without disqualifying an otherwise correct passage. Incomplete or incorrect lesson records remain in history but never inflate the course progress count.

## Language-scoped tests

The initial locale catalog contains `en` and `es-MX`. Both have original word, quote, punctuation and paragraph passages, while numbers share a language-neutral corpus. Word tests have Easy, Standard and Challenge heuristic tiers. Historical English Standard identifiers remain stable; Spanish modes are suffixed `/es-MX` and nonstandard word tiers append `/easy` or `/challenge`. Comparisons are segmented by this content identity, layout, input method, duration and scoring version. Academy lessons remain English-only, and custom text is not translated. See [LANGUAGES.md](LANGUAGES.md).

## Settings recovery safety

A missing settings.json creates a normal writable first-run profile. An existing invalid/unreadable settings file instead produces a visible read-only recovery state: the original bytes are not changed, and the app blocks preferences saves, goal imports, full backups of fallback defaults and insertion of new scored sessions. Migration-generated profile IDs must successfully persist before history is recorded. Recovery must preserve both settings and the corresponding SQLite database; automatic destructive restore is deliberately not implemented.

The guarded state does not open or initialize the existing SQLite database at startup and blocks exports that would misleadingly represent newly generated fallback identities. This avoids both identity orphaning and unintended schema migration before a valid settings file has been recovered.

## Preview-only physical geometry

The `geometry` module defines non-authoritative ANSI/ISO/JIS/split main typing-block positions in fractional-key units, with independent Linux evdev codes for ISO/JIS extras. The preview can inspect their XKB symbols but does not synthesize key events or claim connected-device geometry. A second split Space is only visual. The module is unit tested and does not change session persistence or input routing. See [KEYBOARDS.md](KEYBOARDS.md).

## Safe keyboard-inspector lifecycle

The GUI initializes without waiting for `hyprctl` or `setxkbmap`. A single-flight worker reads bounded compositor metadata and requests a repaint upon completion. The inspector process is terminated on timeout, reports no raw input stream, and keeps no persistent hardware identifiers. Focus loss invalidates active OS-input provenance and any pending result issued before the focus transition; a subsequent focused inspection is required before attributing a new OS-input session to a recognized compositor layout. See [KEYBOARDS.md](KEYBOARDS.md) and [CI 40020631995](https://github.com/sguzman/typefinch/actions/runs/40020631995).

## Generic Wayland seat metadata

`src/wayland_seat.rs` uses the read-only core `wl_registry`/`wl_seat` protocol to enumerate aggregate seats and keyboard capability. It deliberately binds no `wl_keyboard` object and observes no key presses or keymap content. `src/devices.rs` chooses this fallback only on a declared Wayland session without Hyprland, parses the small JSON result from an internal bounded subprocess, and displays seats as **aggregate capability references**, never individual physical devices or detected XKB layouts. The helper exists in the Typefinch executable itself, without a background daemon. See [CI 40023774467](https://github.com/sguzman/typefinch/actions/runs/40023774467) for a successful Weston protocol probe with zero advertised seats.
