# Typefinch 🐦⌨️

**Find your rhythm.** A native, local-first Rust + egui typing laboratory with profiles, layouts, practice, statistics and adorable GitHub-ready profile cards.

![Typefinch soft periwinkle project color](assets/typefinch-color.svg)

**Project color:** `#8696F5` · `oklch(70.123% 0.14003 274.97)` ([decision](docs/COLOR.md)). **Theme:** System by default, or explicit ☀ Light / 🌙 Dark.

> **Early development preview:** not yet a validated MVP. Code is committed, but compilation, native input behavior and real Hyprland QA must pass before claiming readiness.

## Currently implemented in source

- ⌨️ English and Mexican Spanish word, quote, punctuation and paragraph tests (plus locale-neutral numbers and private custom passages): 15, 30, 60 and 120 seconds; WPM, accuracy and corrections.
- 🐦 Multiple local profiles, versioned SQLite session history, and saved high-water statistics without online accounts.
- 🌗 System/Light/Dark egui appearance and persisted preference.
- 🗺️ Curated keyboard-layout catalog plus installed XKB layout and variant discovery with four initial ANSI fallback emulation tables: QWERTY, Dvorak, Colemak and Colemak-DH. Installed XKB layout/variant identities can also be compiled for provisional in-app ANSI-position emulation; unavailable identities remain recognition-only. AltGr/dead-key/IME and non-ANSI behavior still need QA.
- 🔌 Read-only Hyprland device discovery with a chooser for device metadata. **Not yet a per-keyboard typing input filter.**
- 🎨 Static XML-escaped SVG GitHub-card exporter.

## Progress and development

**Acceptance-based overall progress:** `████░░░░░░ 40%` across 41 atomic tasks. The checks, associated evidence and progress bars are recorded in [docs/progress.json](docs/progress.json) and presented as [milestones, phases and tasks](docs/ROADMAP.md). Unverified features stay below 100%. The ledger and roadmap are validated with Lua (`lua5.4 tools/roadmap.lua --check`).

Linux (especially EndeavourOS + Hyprland) is the priority runtime.

```sh
cargo run --release
cargo test --all-targets
```

Settings: `$XDG_CONFIG_HOME/typefinch/settings.json` (or `~/.config/typefinch/settings.json`). SVG snapshots: adjacent `exports/` folder. No system-wide layout changes, background keylogging or login server.

[🦀 Architecture](docs/ARCHITECTURE.md) · [🔐 Privacy](docs/PRIVACY.md) · [🗺️ Keyboard support and honest limitations](docs/KEYBOARDS.md) · [🎨 Color](docs/COLOR.md) · [📋 GitHub tracking](docs/ISSUES.md).

Public sources must never include private workstation paths, user-supplied venting, secrets, personal typed text or profiles. License: MIT.

## Native GUI validation

GitHub Actions runs a real X11/virtual-display smoke test using Openbox and software Mesa. It focuses the typing canvas, injects synthetic keys, checks WPM and the saved SQLite record, and uploads a screenshot. This verifies the native application under **X11**, not production Hyprland or physical multi-keyboard routing. CI evidence: [successful X11 and SQLite run](https://github.com/sguzman/typefinch/actions/runs/39870213935).

**More acceptance evidence:** [30 passing Rust tests, paste rejection, Light/Dark screenshots](https://github.com/sguzman/typefinch/actions/runs/39872686067). These are X11 tests; real Hyprland acceptance remains pending.

**Keyboard shortcuts:** F5 starts or restarts a test; F6 exports the local SVG/Markdown GitHub profile card; F7 exports all statistics for the active profile as local JSON. Both are application-local actions.

**Export acceptance:** [F6 native SVG + Markdown integration passed in CI](https://github.com/sguzman/typefinch/actions/runs/39873211637).

**Latest acceptance evidence:** [39 Rust tests + native X11 corrected-error, paste, theme and export QA](https://github.com/sguzman/typefinch/actions/runs/39880764327). Hyprland physical-device QA remains pending.

## 🪶 Practice lessons

Five starter lessons (home row, top row, bottom row, common pairs, symbols) run **untimed** until the whole passage is typed. Completion is saved locally with a `lesson/<id>` content label, but lesson sessions are not pooled with timed tests in comparable-score charts. The lesson passages are original project text. This is an initial curriculum scaffold, not yet adaptive practice.

**Lesson mastery rule:** a passage earns the Completed label only with 100% final character accuracy; a fully typed but inaccurate attempt remains saved for review without advancing the course. Corrected mistakes still count in raw accuracy.

**Per-profile goals:** Each local identity now keeps independent speed and accuracy targets, with live progress bars. Goals are editable without account synchronization and older settings retain safe defaults.

**Latest full verification:** [46 Rust tests and a Lua-asserted X11 graphical suite, including JSON history export](https://github.com/sguzman/typefinch/actions/runs/39888678045). The SVG/profile widget remains manually published; Hyprland hardware QA is pending.

**Private custom text import:** While Custom text is selected, drag a local UTF-8 text/Markdown file onto the window. Imports are bounded at 64 KiB, retain at most 4,096 characters in memory and are discarded on exit. Importing never logs or persists the passage.

**Adaptive practice preview:** After a session with mistakes, choose **Practice missed keys** to start a generated custom drill concentrating on frequently missed reference characters. The drill is deterministic for that session, remains in memory, and does not affect benchmark records. Historical per-key statistics and adaptive efficacy evaluation remain future work.

**Finger-position guide:** The layout preview now includes conventional ANSI left/right finger labels by physical key position, independent of its selected XKB symbol map. These are hints, not personalized ergonomic guidance or a replacement for future per-key heatmaps.

The ANSI keycap preview now highlights mistakes from the current session in red and provides per-symbol hover counts. These are in-memory practice hints only; no per-character errors are written to disk.

## 🌎 Language foundation

Word tests support original English and Mexican Spanish word sets, selectable inside Test setup. Results are tagged separately (English `words`, Spanish `words/es-MX`) for comparable history. This does not yet mean fully localized quotes, lesson curricula, Unicode grapheme normalization, AltGr/Compose, or IME certification. The OS-input mode is recommended for accented Spanish characters.

**Language safety:** English benchmark IDs remain unchanged for existing histories; Mexican Spanish benchmark modes use distinct `/es-MX` identifiers. Spanish word, quote, punctuation and paragraph passages are original project text. Numbers are shared across languages but tracked under separate test contexts. English-only starter lessons and user-provided custom text are not translated automatically. Unicode grapheme normalization and complex input methods remain unverified.
