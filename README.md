# Typefinch 🐦⌨️

**Find your rhythm.** A native, local-first Rust + egui typing laboratory with profiles, layouts, practice, statistics and adorable GitHub-ready profile cards.

![Typefinch soft periwinkle project color](assets/typefinch-color.svg)

**Project color:** `#8696F5` · `oklch(70.123% 0.14003 274.97)` ([decision](docs/COLOR.md)). **Theme:** System by default, or explicit ☀ Light / 🌙 Dark.

> **Early development preview:** not yet a validated MVP. Code is committed, but compilation, native input behavior and real Hyprland QA must pass before claiming readiness.

## Currently implemented in source

- ⌨️ English word, quote, number and punctuation tests: 15, 30, 60 and 120 seconds; WPM, accuracy and corrections.
- 🐦 Multiple local profiles, versioned SQLite session history, and saved high-water statistics without online accounts.
- 🌗 System/Light/Dark egui appearance and persisted preference.
- 🗺️ Curated keyboard-layout catalog plus installed XKB layout and variant discovery with four initial ANSI fallback emulation tables: QWERTY, Dvorak, Colemak and Colemak-DH. Installed XKB layout/variant identities can also be compiled for provisional in-app ANSI-position emulation; unavailable identities remain recognition-only. AltGr/dead-key/IME and non-ANSI behavior still need QA.
- 🔌 Read-only Hyprland device discovery with a chooser for device metadata. **Not yet a per-keyboard typing input filter.**
- 🎨 Static XML-escaped SVG GitHub-card exporter.

## Progress and development

**Acceptance-based overall progress:** `███░░░░░░░ 30%` across 41 atomic tasks. The checks, associated evidence and progress bars are recorded in [docs/progress.json](docs/progress.json) and presented as [milestones, phases and tasks](docs/ROADMAP.md). Unverified features stay below 100%.

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
