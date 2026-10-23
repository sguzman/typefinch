# 🗺️ Typefinch roadmap

**Source of truth:** [progress.json](progress.json). Percentages are evidence-backed fractions of atomic acceptance checks, averaged into phase/milestone totals. No unverified task reaches 100%.

**Overall:** `███████░░░ 74%` across 41 tasks. This is tracked progress, not a claim of runtime readiness.

## 🐣 M0 — Native typing MVP — `████████░░ 77%`

**Acceptance gate:** A working native session, reliable scoring, local profiles, and live Hyprland acceptance.

### 🧱 P0 — Foundation and visual identity — `███████░░░ 69%`

- 🦀 **T001** Rust/egui application — `███████░░░ 67%`
  - [x] Native shell committed — Cargo.toml; src/main.rs
  - [x] CI native build passes — GitHub Actions run 39867817749: cargo check succeeded
  - [ ] Live Hyprland window screenshot
- 🎨 **T002** Perceptual OKLCH project color — `███████░░░ 67%`
  - [x] Color specified — docs/COLOR.md
  - [x] README swatch authored — assets/typefinch-color.svg
  - [ ] Contrast and accessibility audit
- 🌗 **T003** System, Light and Dark modes — `████████░░ 75%`
  - [x] Three-way selector coded — src/app.rs
  - [x] Preference persistence coded — src/preferences.rs
  - [x] Explicit Light and Dark rendered screenshots pass graphical CI — GitHub Actions 39872686067: light luminance 0.955 vs dark 0.125
  - [ ] Hyprland system theme transition verified
- 🧭 **T004** Navigation and responsive egui shell — `███████░░░ 67%`
  - [x] Main panels and controls coded — src/app.rs
  - [ ] Keyboard navigation audit
  - [x] HiDPI/small-window screenshots — CI 39955283848: 740×590 narrow and 150% DPI 1279×880 screenshots; both inspected for immediate, legible typing surface

### 🐦 P1 — Profiles and persistence — `█████████░ 89%`

- 👤 **T101** Multiple local profiles — `██████████ 100%`
  - [x] Create/select profile UI coded — src/app.rs
  - [x] JSON persistence coded — src/preferences.rs
  - [x] Recovery, collisions, migrations and actual runtime testing — src/preferences.rs: stable profile UUID migrations, missing/corrupt identity guards; src/backup_verify.rs: validated isolated recovery; native X11 recovery and rollback tests, Actions 39967868689
- 🗄️ **T102** SQLite test history — `██████████ 100%`
  - [x] Schema and migrations — src/history.rs: schema user_version=1, idempotent migration; Actions 39870213935
  - [x] Persist each complete test — src/history.rs and synthetic X11 session SQLite assertion; Actions 39870213935
  - [x] Retention and backup tests — src/history.rs: >100-session retention query, VACUUM INTO snapshot test and native F8 backup QA, CI 39895462191
- 🎯 **T103** Per-profile goals and settings — `███████░░░ 67%`
  - [x] Settings and goals UI — src/app.rs: independent editable WPM/accuracy goals and progress bars; Actions 39888678045
  - [x] Per-profile setting persistence — src/preferences.rs: per-profile goals and migration defaults; Actions 39888678045
  - [ ] Export/import

### ⌨️ P2 — Tests and core scoring — `████████░░ 77%`

- ⚡ **T201** Typing input and session lifecycle — `████████░░ 75%`
  - [x] Input handlers implemented — src/app.rs
  - [x] Start/cancel/restart coded — src/engine.rs
  - [x] End-to-end text/paste/focus QA — tests/gui_smoke.sh: real typed/paste prevention and two-window focus-loss cancellation, no SQLite row; Actions 39967868689
  - [ ] IME support
- 📏 **T202** WPM and accuracy calculations — `██████████ 100%`
  - [x] Pure scoring function implemented — src/engine.rs
  - [x] Scoring tests authored — src/engine.rs
  - [x] CI scoring tests pass — GitHub Actions run 39867817749: all 11 Rust unit tests passed
- ⏱️ **T203** Timed tests — `████████░░ 75%`
  - [x] Four durations coded — src/app.rs
  - [x] Monotonic timer coded — src/engine.rs
  - [ ] Live timing edge-case QA
  - [x] Session history persisted — Xvfb session stored in SQLite during GUI smoke; Actions 39870213935
- 📚 **T204** Word, quote, punctuation and custom tests — `████████░░ 83%`
  - [x] Starter English corpus — src/engine.rs
  - [x] Quote passages — src/engine.rs quotes corpus; GitHub Actions 39872686067
  - [x] Dedicated paragraph test content — src/engine.rs: paragraph content; GitHub Actions 39880764327: 39 tests passed
  - [x] Punctuation and numbers — src/engine.rs provides distinct numeric and punctuation corpora; GitHub Actions 39872686067
  - [x] Editable private custom passages — src/app.rs, src/engine.rs: in-memory custom editor and bounded text; Actions 39873831520 and 39880764327
  - [ ] Import local text files into custom passages
- 🧮 **T205** Scoring taxonomy and fair comparisons — `█████░░░░░ 50%`
  - [x] Raw vs corrected WPM — src/engine.rs, src/history.rs: raw WPM, attempts and corrected metrics; Actions 39880764327 X11 corrected-error QA
  - [ ] Per-language speed definitions
  - [ ] Pause/focus/repeat rules
  - [x] Anti-cheat tests — Real X11 Ctrl+V injection excluded from scored session; GitHub Actions 39872686067

**Feature inventory** (computed from associated task progress):

- ⌨️ Typing tests — `████████░░ 77%`
- 🐦 Local identity — `█████████░ 89%`
- 🌗 Appearance — `███████░░░ 70%`

## 🗺️ M1 — Keyboard layout laboratory — `███████░░░ 72%`

**Acceptance gate:** Broad XKB-backed layout emulation and honest multi-device handling.

### 🧬 P3 — Recognition, emulation and geometry — `████████░░ 83%`

- 📖 **T301** Known layout catalog — `██████████ 100%`
  - [x] Curated identifiers and names coded — src/layouts.rs
  - [x] Discover installed XKB variants — src/registry.rs; GitHub Actions 39872686067 discovered XKB layouts and passed 30 tests
  - [x] Normalize identities and availability — src/registry.rs and src/app.rs: deduplicated variants, unique active-map resolution, unavailable-map fallback; Actions 39880764327
- 🔀 **T302** Starter software emulation — `████████░░ 75%`
  - [x] Four initial ANSI row mappings coded — src/layouts.rs
  - [x] Physical key event handling coded — src/app.rs
  - [x] Native mapping QA — src/app.rs physical-key emulation and custom-layout event path exercised in GitHub Actions run 39953436230: 79 Rust tests and native X11 graphical suite passed
  - [ ] Full modifiers and punctuation layers
- 🌐 **T303** libxkbcommon-backed full emulation — `████████░░ 75%`
  - [x] Compile installed keymaps — src/xkb.rs; GitHub Actions 39868627924 successfully compiled nine installed XKB layouts and passed mapping tests
  - [x] Session-local XKB Compose for ANSI dead keys and one-grapheme scoring — src/xkb.rs and src/app.rs: session-local XKB Compose, reset on restart/Backspace/cancel/focus loss; GitHub Actions 39985958620: 106 Rust tests and native X11 graphical suite passed
  - [ ] Shift/AltGr/dead-key handling
  - [x] Layout regression corpus — src/xkb.rs: 20 golden symbol cases across US, German, French, Spanish, Latin American, Dvorak, Colemak, Russian, Greek, Hebrew and Arabic, including Shift, AltGr inspection and Unicode; GitHub Actions 39986908299: 107 Rust tests and native X11 graphical suite passed
- 🛠️ **T304** User-editable custom layouts — `██████████ 100%`
  - [x] Editor and geometry — src/custom_layout.rs ANSI base/shift mapping with egui editor, validation; GitHub Actions run 39953436230: 79 Rust tests and native X11 graphical suite passed
  - [x] Versioned schema — src/custom_layout.rs v1 UUID-identified serde JSON schema, Unicode grapheme validation, malformed-input unit tests; GitHub Actions run 39953436230: 79 Rust tests and native X11 graphical suite passed
  - [x] Import/export and tests — src/app.rs bounded JSON file import/export, collision renaming, per-profile persistence and synthetic custom-keyboard input; GitHub Actions run 39953436230: 79 Rust tests and native X11 graphical suite passed
- 🌍 **T305** ANSI, ISO, JIS and split geometries — `███████░░░ 67%`
  - [x] Geometry definitions — src/geometry.rs: typed ANSI/ISO/JIS/split main-block slot models, verified Linux evdev extra-key codes, uniqueness and non-attribution tests; GitHub Actions 39992682545: 111 Rust tests and native X11 smoke passed. This is preview geometry, not complete hardware routing.
  - [ ] Physical key routing
  - [x] Per-key rendering for modeled main typing-block positions — src/app.rs and src/geometry.rs: XKB key symbols, named dead-key positions, extra-key placeholders and four layouts; 1100×850 ISO/JIS/split native screenshots inspected from Actions 39997672154 (113 Rust tests + X11 smoke pass), including full five-row visibility and unclipped split final key. Main-block preview only.
  - [x] Core control/modifier keycaps and XKB position parity — src/geometry.rs, src/app.rs and src/xkb.rs: typed Backspace/Tab/Enter/Caps/Shift/Ctrl/Alt/Super keycaps, geometry nonoverlap/non-attribution tests, 48 printable slot-to-XKB code parity checks and native ISO/JIS/split captures visually inspected; CI 40003691924: 116 Rust tests and X11 graphical suite passed
  - [x] Function/navigation and numeric keypad reference views — src/geometry.rs, src/app.rs and tests/gui_smoke.sh: 26 function/navigation and 17 keypad Linux evdev reference keys, non-overlap/uniqueness/non-attribution tests and F10 native selection; Actions 40006834967 passed 118 Rust tests and X11 GUI assertions, both archived screenshots visually inspected
  - [ ] Complete control/modifier and edge-key geometry

### 🔌 P4 — Keyboard detection and selection — `██████░░░░ 59%`

- 🔍 **T401** Hyprland device inspection — `███████░░░ 67%`
  - [x] Read-only hyprctl query coded — src/devices.rs
  - [x] Device and active map display coded — src/app.rs
  - [ ] Live connected-keyboard test
- 🎛️ **T402** Multiple connected keyboards — `███████░░░ 67%`
  - [x] Read-only device chooser coded — src/app.rs
  - [ ] Hotplug support
  - [x] Reliable per-device event attribution or explicit unsupported mode — src/devices.rs CapturePolicy::InspectionOnly and src/app.rs: visible no-per-device typing mode, no physical event stream; unit test with two separate keyboard inspection identities, CI 40014109451: complete Rust and native X11 suites passed
- 🪟 **T403** Generic compositor fallback — `███████░░░ 67%`
  - [x] Safe Hyprland-unavailable state coded — src/devices.rs
  - [ ] Generic XKB seat detection
  - [x] X11 fallback — src/devices.rs: gated read-only setxkbmap -query on genuine X11 sessions, one clearly labeled aggregate seat rather than invented devices; CI 40008974131: 122 Rust tests and dedicated live Xvfb seat probe, plus graphical suite passed
- 🔐 **T404** Optional authorized per-device capture — `███░░░░░░░ 33%`
  - [x] Permission and threat model — docs/DEVICE_INPUT_SECURITY.md: threat boundary, explicit permission requirements, revocation, focus scoping, no global evdev/root capture, no double-scoring and negative acceptance tests; design-only, no capture implementation
  - [ ] Opt-in backend
  - [ ] No global interception verification

**Feature inventory** (computed from associated task progress):

- 🗺️ Layout recognition — `████████░░ 78%`
- 🔀 Software layout emulation — `████████░░ 79%`
- 🔌 Multi-keyboard selection — `█████░░░░░ 50%`

## 🎓 M2 — Typing academy — `█████████░ 92%`

**Acceptance gate:** Adaptive lessons, weak-key drills, and course progression.

### 🧠 P5 — Practice and learning — `█████████░ 92%`

- 📚 **T501** Courses and lessons — `██████████ 100%`
  - [x] Lesson content schema — src/lesson.rs: five original licensed-by-author lesson definitions; Actions 39888678045
  - [x] Progression UI — src/app.rs: lesson picker, completion labels and course progress; Actions 39888678045
  - [x] Course completion tests — src/history.rs: full-accuracy mastery rules and profile isolation; GitHub Actions 39888678045, 46 Rust tests and native untimed lesson
- 🎯 **T502** Character and n-gram drills — `███████░░░ 67%`
  - [x] Per-key timing metrics — src/engine.rs bounded ephemeral single-key timing and deterministic interval tests; src/app.rs slowest-key feedback; CI 39950437760 (73 Rust tests)
  - [x] Adaptive drill generation — src/engine.rs: ephemeral missed-key drill generation and deterministic tests; Actions 39896389227
  - [ ] Training efficacy tests
- 🖐️ **T503** Finger assignments and visual guidance — `██████████ 100%`
  - [x] Finger models — src/layouts.rs and src/app.rs: tested ANSI positional finger assignments and keycap guide; Actions 39896389227
  - [x] Heatmaps — src/app.rs graded severity keycap heatmap with per-character hover counts; native X11 graphical CI 39950437760
  - [x] Accessibility tests — src/app.rs tests for WCAG >=4.5 error ink on each light/dark heatmap severity; CI 39950437760
- 🌱 **T504** Daily targets and accuracy-first practice — `██████████ 100%`
  - [x] Goal engine — src/preferences.rs and src/app.rs: user-configured benchmark targets and progress calculations; Actions 39888678045
  - [x] Challenge UI — src/app.rs: editable per-profile daily benchmark slider, progress bar and optional streak control; CI 39947014970
  - [x] Performance summaries — src/daily.rs: weighted-accuracy and local-calendar benchmark summaries with timezone tests; CI 39947014970

**Feature inventory** (computed from associated task progress):

- 🎓 Structured curriculum — `██████████ 100%`
- 🧠 Adaptive exercises — `████████░░ 84%`

## 📊 M3 — Statistics and achievements — `█████████░ 92%`

**Acceptance gate:** Reproducible longitudinal statistics and achievements.

### 📈 P6 — Analytics and progression — `█████████░ 92%`

- 🏅 **T601** Records by layout and test mode — `██████████ 100%`
  - [x] Basic profile bests tracked — src/app.rs
  - [x] Segmented history — src/history.rs: context-specific comparisons and per-profile recent history; Actions 39888678045
  - [x] Versioned scoring policies — src/history.rs: SCORING_VERSION 2 preserves v1 data and excludes cross-version trends; Actions 39896389227
- 📉 **T602** Progress charts and consistency — `██████████ 100%`
  - [x] Timeseries store — src/history.rs: timestamped SQLite session records and comparable context histories; Actions 39880764327
  - [x] Graphs and filters — src/history.rs selectable metric summaries and context filter; src/app.rs 4-metric chart/table UI; CI 39947014970 (66 Rust tests and X11 smoke)
  - [x] Consistency formulas — src/history.rs: mean, best and population standard deviation with unit test; Actions 39888678045
- 🔥 **T603** Key timing and error heatmaps — `███████░░░ 67%`
  - [x] Event recording — src/engine.rs bounded in-memory key/error interval recording, privacy-safe aggregates; GitHub Actions run 39953436230: 79 Rust tests and native X11 graphical suite passed
  - [x] Aggregations — src/engine.rs deterministic missed-key counts and per-key timing averages; GitHub Actions run 39953436230: 79 Rust tests and native X11 graphical suite passed
  - [ ] Visual heatmaps
- 🏆 **T604** Achievements and streaks — `██████████ 100%`
  - [x] Rules and earned records — src/achievements.rs: six deterministic, profile-scoped earned badges with qualification and anti-gaming unit tests; CI 39947014970
  - [x] Celebration UI — src/app.rs: award celebration message and achievement status panel; CI 39947014970 native executable
  - [x] Optional streaks — src/daily.rs: opt-in local-day streak with grace and timezone tests; src/preferences.rs default disabled; CI 39947014970

**Feature inventory** (computed from associated task progress):

- 📊 Statistics — `█████████░ 89%`
- 🏆 Achievements — `██████████ 100%`

## 🎨 M4 — Profile card studio — `███████░░░ 69%`

**Acceptance gate:** Accessible exportable SVG and Markdown cards with a friendly finch identity.

### 🐤 P7 — Widgets and publishing — `███████░░░ 69%`

- 🪄 **T701** GitHub-safe SVG card — `████████░░ 75%`
  - [x] SVG generator coded — src/export.rs
  - [x] XML escaping and test authored — src/export.rs
  - [ ] Rendered validation on GitHub
  - [x] Filesystem export test passes — GitHub Actions run 39873211637: F6 native GUI writes valid SVG and paired Markdown, parsed by XML validator
- 🎛️ **T702** Widget designer — `██████████ 100%`
  - [x] Template system — src/export.rs Classic 550×205 and Compact 550×145 versioned templates, live egui preview and native PNG/SVG export; CI 39949282753 (70 tests and X11 verification)
  - [x] Color and type controls — src/preferences.rs card theme/font settings; src/export.rs static palettes and escaped SVG; native Daybreak/mono export CI 39948044402
  - [x] Live preview — src/app.rs immediate native egui palette/typography preview; Rust tests + graphical native CI 39948522387
- 📝 **T703** Markdown, PNG, JSON exports — `██████████ 100%`
  - [x] README Markdown — src/export.rs produces paired local Markdown/SVG and validates the embed; GitHub Actions 39872686067
  - [x] PNG renderer — src/export.rs resvg/tiny-skia native PNG raster at 550×205, unit test plus F9 X11 GUI and Lua PNG validation; CI 39948522387
  - [x] Privacy-aware data exports — src/export.rs, src/history.rs: entire profile's versioned JSON aggregate export; Actions 39888678045 native F7 export QA
- 🔄 **T704** Optional GitHub profile updates — `░░░░░░░░░░ 0%`
  - [ ] Explicit opt-in
  - [ ] Safe credentials
  - [ ] Offline failure handling

**Feature inventory** (computed from associated task progress):

- 🐤 GitHub widget — `█████████░ 92%`
- 🔄 Publishing automation — `░░░░░░░░░░ 0%`

## 🌎 M5 — Multilingual typing — `█████░░░░░ 50%`

**Acceptance gate:** Correct Unicode, IME composition, RTL and per-language corpora.

### 🗣️ P8 — Languages and input methods — `█████░░░░░ 50%`

- 🧾 **T801** Language packs and corpora — `██████████ 100%`
  - [x] English/Spanish data — src/engine.rs: English and Mexican Spanish corpora, CI 39895462191
  - [x] Corpus provenance and license — docs/LANGUAGES.md: original Typefinch-authored passages with provenance/license description, CI 39895462191
  - [x] Difficulty grading — src/engine.rs: heuristic Easy/Standard/Challenge word tiers and history isolation, CI 39895462191
- 🔤 **T802** Grapheme and normalization scoring — `██████████ 100%`
  - [x] Grapheme segmentation — src/engine.rs and src/app.rs: unicode-segmentation counts/display/Backspace; Actions 39896389227 (59 tests)
  - [x] Normalization rules — src/engine.rs: NFC canonical comparison, combining-accent completion; Actions 39896389227
  - [x] Regression tests — src/engine.rs: decomposed accent, joined emoji, flag and whole-grapheme Backspace regressions; Actions 39896389227
- ㊙️ **T803** CJK IME composition — `░░░░░░░░░░ 0%`
  - [ ] Committed text semantics
  - [ ] CJK integration tests
  - [ ] Speed conventions
- 🪶 **T804** Arabic and Hebrew bidirectional input — `░░░░░░░░░░ 0%`
  - [ ] Shaping and caret
  - [ ] Error highlighting
  - [ ] RTL tests

**Feature inventory** (computed from associated task progress):

- 🌏 Languages — `███░░░░░░░ 33%`
- 🔤 Unicode correctness — `██████████ 100%`

## 📦 M6 — Release quality — `██████░░░░ 58%`

**Acceptance gate:** Automated regression, live QA and reproducible Linux packages.

### 🧪 P9 — Acceptance and release — `██████░░░░ 58%`

- 🧪 **T901** CI and acceptance-ledger validation — `██████████ 100%`
  - [x] Workflow authored — .github/workflows/ci.yml
  - [x] CI build/test green — GitHub Actions run 39867817749: cargo check and cargo test passed
  - [x] Roadmap drift verified — GitHub Actions 39868627924: render_roadmap.py --check passed
- 🪟 **T902** Live Hyprland automation — `███░░░░░░░ 33%`
  - [x] Headless run — Xvfb + Openbox + Mesa launched native egui app, typed 33 chars and captured screenshot; Actions 39870050679
  - [ ] Keyboard/input QA
  - [ ] Theme/layout screenshots
- ♿ **T903** Accessibility review — `███░░░░░░░ 33%`
  - [ ] Keyboard-only audit
  - [x] Contrast/reduced motion — src/app.rs: WCAG-oriented contrast palette tests and reduce-motion timing tests; native light/dark screenshot QA, Actions 39967868689
  - [ ] Unicode fonts
- 🚀 **T904** Reproducible Linux release — `███████░░░ 67%`
  - [x] Versioned packages — GitHub Actions 39958743350: optimized Linux x86_64 build, deterministic allowlisted tar.gz + SHA-256, install/upgrade/rollback and unmanaged-executable refusal
  - [x] Install/rollback tests — GitHub Actions 39958743350: optimized Linux x86_64 build, deterministic allowlisted tar.gz + SHA-256, install/upgrade/rollback and unmanaged-executable refusal
  - [ ] MVP gate accepted

**Feature inventory** (computed from associated task progress):

- 🧪 Automated QA — `███████░░░ 67%`
- 🚀 Release readiness — `█████░░░░░ 50%`

