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

**Acceptance-based overall progress:** `███████░░░ 71%` across 41 atomic tasks. The checks, associated evidence and progress bars are recorded in [docs/progress.json](docs/progress.json) and presented as [milestones, phases and tasks](docs/ROADMAP.md). Unverified features stay below 100%. The ledger and roadmap are validated with Lua (`lua5.4 tools/roadmap.lua --check`).

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

**Word difficulty tiers:** English and Mexican Spanish Words tests support Easy, Standard and Challenge starter word lists. Standard preserves existing history IDs; alternative tiers receive their own context labels. These are transparent heuristic groupings, not validated proficiency ratings.

**Full local backups:** F8 or **Backup settings + SQLite** creates a timestamped pair under `exports/`: a settings JSON file and a consistent SQLite snapshot. The pair includes local profile names and complete aggregate history, so treat both as private files. They are not uploaded, published or automatically restored; backup creation never deletes existing data.

**Latest language and backup acceptance:** [55 passing Rust tests + native Spanish Easy session + consistent settings/SQLite snapshot](https://github.com/sguzman/typefinch/actions/runs/39895462191). Normalization, IMEs, RTL and physical Hyprland QA remain open.

**Unicode scoring v2:** Final scores now compare canonically normalized extended grapheme clusters rather than raw Unicode scalar positions. Existing version-1 history remains intact and is not silently combined with new trends. Complex IME and bidirectional input are not yet validated.

**Latest Unicode acceptance:** [59 passing Rust tests, NFC grapheme scoring and full Lua-backed X11 QA](https://github.com/sguzman/typefinch/actions/runs/39896389227). Complex IME and Hyprland hardware QA remain pending.

**History explorer:** Filter the recent-session table to the currently comparable test configuration and switch the trend between corrected/raw WPM and final/raw accuracy. Older results without recorded raw attempts display a dash, not a fabricated zero. Charts exclude incompatible scoring versions, layouts, durations and content/language tiers.

**Offline achievements:** Six initial badges are derived from saved per-profile history (first benchmark, sustained practice, precision, speed, two languages and academy completion). Legitimate thresholds prevent tiny/custom sessions from earning benchmark skill badges. Awards are recalculated locally and never uploaded; optional streaks remain a future feature.

**Daily practice goal:** Each local profile can configure a target number of qualifying timed benchmark sessions per calendar day. The progress bar derives counts and weighted accuracy from saved aggregate history using the computer's local timezone, excluding private passages and short practice runs. No calendar service, reminders, or background tracking is involved.

**Optional streak:** An opt-in checkbox shows consecutive days with substantive benchmark sessions, calculated from local dates. It is off by default and offers a one-day grace window for a day still in progress. No notifications, network service, or penalties are attached.

**Latest acceptance:** [66 Rust tests + native X11 session QA with daily targets, opt-in streak and local achievements](https://github.com/sguzman/typefinch/actions/runs/39947014970). Real Hyprland hardware acceptance remains pending.

**Card studio palettes:** Profile exports now support Dusk, Daybreak and Fern themes and sans-serif/monospace typography. Controls persist locally, and the preview updates immediately; SVG generation remains static and escapes user-controlled profile/layout text. No hosted fonts or network dependency.

**PNG card export:** F9 or the Export PNG button rasterizes the current styled SVG locally via the pure-Rust resvg renderer and writes `typefinch-profile.png` beside the SVG. No external converter, shell script, or network service is needed.

**Latest verified export:** [native Rust SVG-to-PNG, 550×205 output plus Daybreak/mono F9 graphical verification](https://github.com/sguzman/typefinch/actions/runs/39948522387). Card layout templates beyond the original format remain open.

**Card layouts:** Classic (550×205) and Compact (550×145) are distinct native-preview/SVG/PNG templates; either can use Dusk, Daybreak, Fern, sans-serif or monospace. Classic remains the persisted default for existing installs.

**Verified card templates:** [70 Rust tests and a native Compact+Daybreak+monospace SVG/PNG export](https://github.com/sguzman/typefinch/actions/runs/39949282753).

**Device refresh:** The Hyprland keyboard inventory now refreshes when Typefinch regains window focus, preserving the selected inspection device by address where possible; manual Refresh remains available. This does not grant per-device input attribution or interception.

**Ephemeral key timing:** The keyboard preview reports slower characters after at least two valid correctly typed single-key intervals (25–2000 ms). It excludes batch/composition commits and resets when starting a new session. No per-character timing or typed contents are stored or exported.

**Latest academy validation:** [73 passing tests, contrast-checked graded in-memory keyboard heatmap and timing summaries](https://github.com/sguzman/typefinch/actions/runs/39950437760). Full physical-key analytics and Hyprland hardware QA are still outstanding.

**Personal ANSI layouts:** Create and edit three ten-key rows (base and Shift) within Typefinch, using a validated v1 JSON schema. Custom keymaps are saved in local settings, selectable as software-emulated targets, and never modify the OS or capture global input. Explicit JSON import is limited to 16 KiB and refuses invalid mappings; export writes to the local `exports/` folder.

**Personal ANSI layouts and private settings:** [79 passing Rust tests plus native physical-key, SVG/PNG and input QA](https://github.com/sguzman/typefinch/actions/runs/39953436230). The v1 custom mapping editor is in-app ANSI only; physical-device routing and live Hyprland acceptance remain pending.

**Connected keyboard refresh:** A read-only, asynchronous Hyprland inventory check updates the available device list when Typefinch regains focus or about every 20 seconds while active. Selection is preserved by reported address where possible. It does not capture input from other applications or establish which keyboard delivered an event.

**Responsive QA:** [80 passing Rust tests and native X11 screenshots](https://github.com/sguzman/typefinch/actions/runs/39955283848), including 740×590 compact and 150% scaled viewports. Both screenshots were inspected to confirm the typing exercise is visible on entry. Live Hyprland/HiDPI hardware and keyboard-only navigation remain unfinished.

**Guaranteed PNG text:** The native PNG converter reuses the fonts already embedded in egui, without installing or exporting font files. PNG generation now refuses to save a card if the heading failed to render (for example on a fontless Linux CI image); the test suite checks actual glyph pixels, not only PNG dimensions.

**Linux preview distribution:** Passing CI produces a checksum-protected x86_64 tarball, a user-local installer, and one-level rollback tested in a disposable prefix. This is not an MVP release. See [installation instructions](docs/RELEASE.md).

**Verified versioned Linux preview:** [80 Rust tests, real X11 QA, optimized package and safe user-local upgrade/rollback](https://github.com/sguzman/typefinch/actions/runs/39958743350). This is a development preview; the MVP release gate is still open.

**Headless CLI:** `typefinch --version` and `typefinch --help` return immediately without opening egui. Unknown flags fail explicitly instead of launching an unexpected window. This is verified by standard CI; packaged previews built before the change retain their original CLI behavior.

**Keyboard access and reduced motion:** F1 opens a keyboard-navigation guide; F2 focuses the typing surface, Tab/Shift+Tab navigates egui controls, and Escape cancels a focused active test. A saved Reduce UI animations toggle sets egui transition time to zero without altering scored timers. System/Light/Dark remains independent and System stays the default. A full physical keyboard-only acceptance audit is still pending.

**Keyboard-accessible private import:** Custom mode also has an explicit path field and Load local file button (Enter from the path field works), so no drag-and-drop or file picker is required. Both routes share the same UTF-8/64-KiB/4,096-grapheme checks and reject non-regular files. Neither paths nor passage text are written to settings or history.

**Independent goal transfer:** The profile panel now exports a versioned private goals JSON file and accepts a local goals JSON path for import. Import modifies only the selected profile's speed/accuracy/daily targets and optional streak setting. It cannot replace profile identities, completed tests, personal records or SQLite sessions. Inputs are capped at 8 KiB and validated before applying.

**Non-destructive backup verification:** `typefinch --verify-backup SETTINGS.json SESSIONS.sqlite3` checks an F8-generated pair without changing files, opening the GUI, or touching active profiles. It verifies SQLite integrity and profile references; automatic restore is still deliberately not implemented.

**Corrupt settings safeguard:** If an existing settings file cannot be read or decoded, Typefinch preserves it byte-for-byte and clearly enters a read-only recovery state rather than creating a replacement profile on disk. Preferences writes, goals import, full backup from fallback defaults, and scored session persistence are disabled until the settings file is repaired or recovered. Missing settings on a genuine first run still bootstrap normally.

The guarded startup path also avoids opening or migrating the saved SQLite database until the settings identity is recovered, and prevents publishing fallback profile statistics or goals as if they were genuine.

**Orphaned-history safeguard:** If `settings.json` disappears but `sessions.sqlite3` still exists, startup now enters guarded recovery mode instead of assigning a new profile UUID. Original session data remains untouched while a matching settings backup is recovered.

**In-app backup inspection:** The profile studio and guarded recovery screen now accept an explicit F8 settings-backup path, infer its companion SQLite file, and report a read-only validity check. Paths and validation feedback are transient UI state; Typefinch never restores or overwrites anything from this inspection.

**Non-overwriting restore staging:** `typefinch --restore-backup-to-new-dir SETTINGS.json SESSIONS.sqlite3 NEW_DIRECTORY` validates a paired backup then copies it into a brand-new directory, with owner-only permissions on Unix. It refuses preexisting destinations and does not touch or activate the current profile. In-app recovery verification remains a separate read-only control.

**XKB modifier-layer inspector:** The keyboard preview now switches among Base, Shift, AltGr and Shift+AltGr to inspect installed layouts. Linux XKB Level-3 symbols (for example, German AltGr+E → €) are resolved with the same native keymap, while unsupported personal-map AltGr layers show an explicit unavailable glyph. Input emulation supports session-scoped XKB dead-key Compose on supported ANSI positions, but does not claim compositor-independent physical AltGr, dedicated Compose-key routing, or IME support.

**Keyboard-driven preview:** F3 cycles Base, Shift, AltGr and Shift+AltGr without changing the system keyboard or the active typing session. This is an in-app symbol inspector, not modifier-event interception.

**Stateful dead-key Compose:** XKB-backed ANSI emulation now maintains a fresh Compose state for each scored test. Spanish acute + e produces one scored `é`; a pending dead key can be canceled with Backspace, Escape or focus loss without recording an extra character. Physical AltGr and complex IME input remain unverified. [106 Rust tests and graphical CI passed](https://github.com/sguzman/typefinch/actions/runs/39985958620).

**Recovery and focus QA:** [102 passing Rust tests plus native recovery staging, corrupt/orphaned settings protection, and two-window focus-loss cancellation](https://github.com/sguzman/typefinch/actions/runs/39967868689). Real Hyprland hardware acceptance and full IME handling remain open.

**Keyboard shape explorer:** The keyboard preview now includes selectable ANSI, ISO, JIS and split **main-block illustrations** with XKB symbol inspection where installed. Unattributed ISO/JIS extras are visibly distinct from regular egui physical-key input, and a split layout does not double-count Space. This is not a physical-keyboard detector, complete numeric-keypad model, or extra-key emulation. [111 Rust tests and native CI passed for the geometry model](https://github.com/sguzman/typefinch/actions/runs/39992682545).

**Verified keyboard geometry visualization:** [CI 39997672154](https://github.com/sguzman/typefinch/actions/runs/39997672154) passed 113 Rust tests and produced native ISO/JIS/split screenshots. The main typing-block keycaps were visually inspected after correcting accumulated spacing and scroll clipping. The model is not a full 104/105/109-key hardware map and cannot route extra ISO/JIS physical-key events. F4 cycles the displayed keyboard family without changing the OS layout.
