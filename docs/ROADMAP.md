# 🗺️ Typefinch roadmap

**Source of truth:** [progress.json](progress.json). Percentages are evidence-backed fractions of atomic acceptance checks, averaged into phase/milestone totals. No unverified task reaches 100%.

**Overall:** `████░░░░░░ 44%` across 41 tasks. This is tracked progress, not a claim of runtime readiness.

## 🐣 M0 — Native typing MVP — `███████░░░ 70%`

**Acceptance gate:** A working native session, reliable scoring, local profiles, and live Hyprland acceptance.

### 🧱 P0 — Foundation and visual identity — `██████░░░░ 61%`

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
- 🧭 **T004** Navigation and responsive egui shell — `███░░░░░░░ 33%`
  - [x] Main panels and controls coded — src/app.rs
  - [ ] Keyboard navigation audit
  - [ ] HiDPI/small-window screenshots

### 🐦 P1 — Profiles and persistence — `████████░░ 78%`

- 👤 **T101** Multiple local profiles — `███████░░░ 67%`
  - [x] Create/select profile UI coded — src/app.rs
  - [x] JSON persistence coded — src/preferences.rs
  - [ ] Recovery, collisions, migrations and actual runtime testing
- 🗄️ **T102** SQLite test history — `██████████ 100%`
  - [x] Schema and migrations — src/history.rs: schema user_version=1, idempotent migration; Actions 39870213935
  - [x] Persist each complete test — src/history.rs and synthetic X11 session SQLite assertion; Actions 39870213935
  - [x] Retention and backup tests — src/history.rs: >100-session retention query, VACUUM INTO snapshot test and native F8 backup QA, CI 39895462191
- 🎯 **T103** Per-profile goals and settings — `███████░░░ 67%`
  - [x] Settings and goals UI — src/app.rs: independent editable WPM/accuracy goals and progress bars; Actions 39888678045
  - [x] Per-profile setting persistence — src/preferences.rs: per-profile goals and migration defaults; Actions 39888678045
  - [ ] Export/import

### ⌨️ P2 — Tests and core scoring — `███████░░░ 72%`

- ⚡ **T201** Typing input and session lifecycle — `█████░░░░░ 50%`
  - [x] Input handlers implemented — src/app.rs
  - [x] Start/cancel/restart coded — src/engine.rs
  - [ ] End-to-end text/paste/focus QA
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

- ⌨️ Typing tests — `███████░░░ 72%`
- 🐦 Local identity — `████████░░ 78%`
- 🌗 Appearance — `██████░░░░ 58%`

## 🗺️ M1 — Keyboard layout laboratory — `████░░░░░░ 35%`

**Acceptance gate:** Broad XKB-backed layout emulation and honest multi-device handling.

### 🧬 P3 — Recognition, emulation and geometry — `████░░░░░░ 37%`

- 📖 **T301** Known layout catalog — `██████████ 100%`
  - [x] Curated identifiers and names coded — src/layouts.rs
  - [x] Discover installed XKB variants — src/registry.rs; GitHub Actions 39872686067 discovered XKB layouts and passed 30 tests
  - [x] Normalize identities and availability — src/registry.rs and src/app.rs: deduplicated variants, unique active-map resolution, unavailable-map fallback; Actions 39880764327
- 🔀 **T302** Starter software emulation — `█████░░░░░ 50%`
  - [x] Four initial ANSI row mappings coded — src/layouts.rs
  - [x] Physical key event handling coded — src/app.rs
  - [ ] Native mapping QA
  - [ ] Full modifiers and punctuation layers
- 🌐 **T303** libxkbcommon-backed full emulation — `███░░░░░░░ 33%`
  - [x] Compile installed keymaps — src/xkb.rs; GitHub Actions 39868627924 successfully compiled nine installed XKB layouts and passed mapping tests
  - [ ] Shift/AltGr/dead-key handling
  - [ ] Layout regression corpus
- 🛠️ **T304** User-editable custom layouts — `░░░░░░░░░░ 0%`
  - [ ] Editor and geometry
  - [ ] Versioned schema
  - [ ] Import/export and tests
- 🌍 **T305** ANSI, ISO, JIS and split geometries — `░░░░░░░░░░ 0%`
  - [ ] Geometry definitions
  - [ ] Physical key routing
  - [ ] Per-key rendering

### 🔌 P4 — Keyboard detection and selection — `███░░░░░░░ 33%`

- 🔍 **T401** Hyprland device inspection — `███████░░░ 67%`
  - [x] Read-only hyprctl query coded — src/devices.rs
  - [x] Device and active map display coded — src/app.rs
  - [ ] Live connected-keyboard test
- 🎛️ **T402** Multiple connected keyboards — `███░░░░░░░ 33%`
  - [x] Read-only device chooser coded — src/app.rs
  - [ ] Hotplug support
  - [ ] Reliable per-device event attribution or explicit unsupported mode
- 🪟 **T403** Generic compositor fallback — `███░░░░░░░ 33%`
  - [x] Safe Hyprland-unavailable state coded — src/devices.rs
  - [ ] Generic XKB seat detection
  - [ ] X11 fallback
- 🔐 **T404** Optional authorized per-device capture — `░░░░░░░░░░ 0%`
  - [ ] Permission and threat model
  - [ ] Opt-in backend
  - [ ] No global interception verification

**Feature inventory** (computed from associated task progress):

- 🗺️ Layout recognition — `███████░░░ 67%`
- 🔀 Software layout emulation — `██░░░░░░░░ 21%`
- 🔌 Multi-keyboard selection — `██░░░░░░░░ 17%`

## 🎓 M2 — Typing academy — `███░░░░░░░ 33%`

**Acceptance gate:** Adaptive lessons, weak-key drills, and course progression.

### 🧠 P5 — Practice and learning — `███░░░░░░░ 33%`

- 📚 **T501** Courses and lessons — `██████████ 100%`
  - [x] Lesson content schema — src/lesson.rs: five original licensed-by-author lesson definitions; Actions 39888678045
  - [x] Progression UI — src/app.rs: lesson picker, completion labels and course progress; Actions 39888678045
  - [x] Course completion tests — src/history.rs: full-accuracy mastery rules and profile isolation; GitHub Actions 39888678045, 46 Rust tests and native untimed lesson
- 🎯 **T502** Character and n-gram drills — `░░░░░░░░░░ 0%`
  - [ ] Per-key timing metrics
  - [ ] Adaptive drill generation
  - [ ] Training efficacy tests
- 🖐️ **T503** Finger assignments and visual guidance — `░░░░░░░░░░ 0%`
  - [ ] Finger models
  - [ ] Heatmaps
  - [ ] Accessibility tests
- 🌱 **T504** Daily targets and accuracy-first practice — `███░░░░░░░ 33%`
  - [x] Goal engine — src/preferences.rs and src/app.rs: user-configured benchmark targets and progress calculations; Actions 39888678045
  - [ ] Challenge UI
  - [ ] Performance summaries

**Feature inventory** (computed from associated task progress):

- 🎓 Structured curriculum — `███████░░░ 67%`
- 🧠 Adaptive exercises — `░░░░░░░░░░ 0%`

## 📊 M3 — Statistics and achievements — `███░░░░░░░ 34%`

**Acceptance gate:** Reproducible longitudinal statistics and achievements.

### 📈 P6 — Analytics and progression — `███░░░░░░░ 34%`

- 🏅 **T601** Records by layout and test mode — `███████░░░ 67%`
  - [x] Basic profile bests tracked — src/app.rs
  - [x] Segmented history — src/history.rs: context-specific comparisons and per-profile recent history; Actions 39888678045
  - [ ] Versioned scoring policies
- 📉 **T602** Progress charts and consistency — `███████░░░ 67%`
  - [x] Timeseries store — src/history.rs: timestamped SQLite session records and comparable context histories; Actions 39880764327
  - [ ] Graphs and filters
  - [x] Consistency formulas — src/history.rs: mean, best and population standard deviation with unit test; Actions 39888678045
- 🔥 **T603** Key timing and error heatmaps — `░░░░░░░░░░ 0%`
  - [ ] Event recording
  - [ ] Aggregations
  - [ ] Visual heatmaps
- 🏆 **T604** Achievements and streaks — `░░░░░░░░░░ 0%`
  - [ ] Rules and earned records
  - [ ] Celebration UI
  - [ ] Optional streaks

**Feature inventory** (computed from associated task progress):

- 📊 Statistics — `█████░░░░░ 45%`
- 🏆 Achievements — `░░░░░░░░░░ 0%`

## 🎨 M4 — Profile card studio — `████░░░░░░ 36%`

**Acceptance gate:** Accessible exportable SVG and Markdown cards with a friendly finch identity.

### 🐤 P7 — Widgets and publishing — `████░░░░░░ 36%`

- 🪄 **T701** GitHub-safe SVG card — `████████░░ 75%`
  - [x] SVG generator coded — src/export.rs
  - [x] XML escaping and test authored — src/export.rs
  - [ ] Rendered validation on GitHub
  - [x] Filesystem export test passes — GitHub Actions run 39873211637: F6 native GUI writes valid SVG and paired Markdown, parsed by XML validator
- 🎛️ **T702** Widget designer — `░░░░░░░░░░ 0%`
  - [ ] Template system
  - [ ] Color and type controls
  - [ ] Live preview
- 📝 **T703** Markdown, PNG, JSON exports — `███████░░░ 67%`
  - [x] README Markdown — src/export.rs produces paired local Markdown/SVG and validates the embed; GitHub Actions 39872686067
  - [ ] PNG renderer
  - [x] Privacy-aware data exports — src/export.rs, src/history.rs: entire profile's versioned JSON aggregate export; Actions 39888678045 native F7 export QA
- 🔄 **T704** Optional GitHub profile updates — `░░░░░░░░░░ 0%`
  - [ ] Explicit opt-in
  - [ ] Safe credentials
  - [ ] Offline failure handling

**Feature inventory** (computed from associated task progress):

- 🐤 GitHub widget — `█████░░░░░ 47%`
- 🔄 Publishing automation — `░░░░░░░░░░ 0%`

## 🌎 M5 — Multilingual typing — `███░░░░░░░ 25%`

**Acceptance gate:** Correct Unicode, IME composition, RTL and per-language corpora.

### 🗣️ P8 — Languages and input methods — `███░░░░░░░ 25%`

- 🧾 **T801** Language packs and corpora — `██████████ 100%`
  - [x] English/Spanish data — src/engine.rs: English and Mexican Spanish corpora, CI 39895462191
  - [x] Corpus provenance and license — docs/LANGUAGES.md: original Typefinch-authored passages with provenance/license description, CI 39895462191
  - [x] Difficulty grading — src/engine.rs: heuristic Easy/Standard/Challenge word tiers and history isolation, CI 39895462191
- 🔤 **T802** Grapheme and normalization scoring — `░░░░░░░░░░ 0%`
  - [ ] Grapheme segmentation
  - [ ] Normalization rules
  - [ ] Regression tests
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
- 🔤 Unicode correctness — `░░░░░░░░░░ 0%`

## 📦 M6 — Release quality — `███░░░░░░░ 33%`

**Acceptance gate:** Automated regression, live QA and reproducible Linux packages.

### 🧪 P9 — Acceptance and release — `███░░░░░░░ 33%`

- 🧪 **T901** CI and acceptance-ledger validation — `██████████ 100%`
  - [x] Workflow authored — .github/workflows/ci.yml
  - [x] CI build/test green — GitHub Actions run 39867817749: cargo check and cargo test passed
  - [x] Roadmap drift verified — GitHub Actions 39868627924: render_roadmap.py --check passed
- 🪟 **T902** Live Hyprland automation — `███░░░░░░░ 33%`
  - [x] Headless run — Xvfb + Openbox + Mesa launched native egui app, typed 33 chars and captured screenshot; Actions 39870050679
  - [ ] Keyboard/input QA
  - [ ] Theme/layout screenshots
- ♿ **T903** Accessibility review — `░░░░░░░░░░ 0%`
  - [ ] Keyboard-only audit
  - [ ] Contrast/reduced motion
  - [ ] Unicode fonts
- 🚀 **T904** Reproducible Linux release — `░░░░░░░░░░ 0%`
  - [ ] Versioned packages
  - [ ] Install/rollback tests
  - [ ] MVP gate accepted

**Feature inventory** (computed from associated task progress):

- 🧪 Automated QA — `███████░░░ 67%`
- 🚀 Release readiness — `░░░░░░░░░░ 0%`

