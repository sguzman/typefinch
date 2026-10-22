# 🗺️ Typefinch roadmap

**Source of truth:** [progress.json](progress.json). Percentages are evidence-backed fractions of atomic acceptance checks, averaged into phase/milestone totals. No unverified task reaches 100%.

**Overall:** `██░░░░░░░░ 22%` across 41 tasks. This is tracked progress, not a claim of runtime readiness.

## 🐣 M0 — Native typing MVP — `████░░░░░░ 44%`

**Acceptance gate:** A working native session, reliable scoring, local profiles, and live Hyprland acceptance.

### 🧱 P0 — Foundation and visual identity — `██████░░░░ 59%`

- 🦀 **T001** Rust/egui application — `███████░░░ 67%`
  - [x] Native shell committed — Cargo.toml; src/main.rs
  - [x] CI native build passes — GitHub Actions run 39867817749: cargo check succeeded
  - [ ] Live Hyprland window screenshot
- 🎨 **T002** Perceptual OKLCH project color — `███████░░░ 67%`
  - [x] Color specified — docs/COLOR.md
  - [x] README swatch authored — assets/typefinch-color.svg
  - [ ] Contrast and accessibility audit
- 🌗 **T003** System, Light and Dark modes — `███████░░░ 67%`
  - [x] Three-way selector coded — src/app.rs
  - [x] Preference persistence coded — src/preferences.rs
  - [ ] Hyprland system theme transition verified
- 🧭 **T004** Navigation and responsive egui shell — `███░░░░░░░ 33%`
  - [x] Main panels and controls coded — src/app.rs
  - [ ] Keyboard navigation audit
  - [ ] HiDPI/small-window screenshots

### 🐦 P1 — Profiles and persistence — `██░░░░░░░░ 22%`

- 👤 **T101** Multiple local profiles — `███████░░░ 67%`
  - [x] Create/select profile UI coded — src/app.rs
  - [x] JSON persistence coded — src/preferences.rs
  - [ ] Recovery, collisions, migrations and actual runtime testing
- 🗄️ **T102** SQLite test history — `░░░░░░░░░░ 0%`
  - [ ] Schema and migrations
  - [ ] Persist each complete test
  - [ ] Retention and backup tests
- 🎯 **T103** Per-profile goals and settings — `░░░░░░░░░░ 0%`
  - [ ] Settings and goals UI
  - [ ] Per-profile setting persistence
  - [ ] Export/import

### ⌨️ P2 — Tests and core scoring — `█████░░░░░ 45%`

- ⚡ **T201** Typing input and session lifecycle — `█████░░░░░ 50%`
  - [x] Input handlers implemented — src/app.rs
  - [x] Start/cancel/restart coded — src/engine.rs
  - [ ] End-to-end text/paste/focus QA
  - [ ] IME support
- 📏 **T202** WPM and accuracy calculations — `██████████ 100%`
  - [x] Pure scoring function implemented — src/engine.rs
  - [x] Scoring tests authored — src/engine.rs
  - [x] CI scoring tests pass — GitHub Actions run 39867817749: all 11 Rust unit tests passed
- ⏱️ **T203** Timed tests — `█████░░░░░ 50%`
  - [x] Four durations coded — src/app.rs
  - [x] Monotonic timer coded — src/engine.rs
  - [ ] Live timing edge-case QA
  - [ ] Session history persisted
- 📚 **T204** Word, quote, punctuation and custom tests — `███░░░░░░░ 25%`
  - [x] Starter English corpus — src/engine.rs
  - [ ] Quotes and paragraphs
  - [ ] Punctuation and numbers
  - [ ] Custom/imported text
- 🧮 **T205** Scoring taxonomy and fair comparisons — `░░░░░░░░░░ 0%`
  - [ ] Raw vs corrected WPM
  - [ ] Per-language speed definitions
  - [ ] Pause/focus/repeat rules
  - [ ] Anti-cheat tests

**Feature inventory** (computed from associated task progress):

- ⌨️ Typing tests — `█████░░░░░ 45%`
- 🐦 Local identity — `██░░░░░░░░ 22%`
- 🌗 Appearance — `██████░░░░ 56%`

## 🗺️ M1 — Keyboard layout laboratory — `██░░░░░░░░ 24%`

**Acceptance gate:** Broad XKB-backed layout emulation and honest multi-device handling.

### 🧬 P3 — Recognition, emulation and geometry — `██░░░░░░░░ 17%`

- 📖 **T301** Known layout catalog — `███░░░░░░░ 33%`
  - [x] Curated identifiers and names coded — src/layouts.rs
  - [ ] Discover installed XKB variants
  - [ ] Normalize identities and availability
- 🔀 **T302** Starter software emulation — `█████░░░░░ 50%`
  - [x] Four initial ANSI row mappings coded — src/layouts.rs
  - [x] Physical key event handling coded — src/app.rs
  - [ ] Native mapping QA
  - [ ] Full modifiers and punctuation layers
- 🌐 **T303** libxkbcommon-backed full emulation — `░░░░░░░░░░ 0%`
  - [ ] Compile installed keymaps
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

- 🗺️ Layout recognition — `████░░░░░░ 44%`
- 🔀 Software layout emulation — `█░░░░░░░░░ 13%`
- 🔌 Multi-keyboard selection — `██░░░░░░░░ 17%`

## 🎓 M2 — Typing academy — `░░░░░░░░░░ 0%`

**Acceptance gate:** Adaptive lessons, weak-key drills, and course progression.

### 🧠 P5 — Practice and learning — `░░░░░░░░░░ 0%`

- 📚 **T501** Courses and lessons — `░░░░░░░░░░ 0%`
  - [ ] Lesson content schema
  - [ ] Progression UI
  - [ ] Course completion tests
- 🎯 **T502** Character and n-gram drills — `░░░░░░░░░░ 0%`
  - [ ] Per-key timing metrics
  - [ ] Adaptive drill generation
  - [ ] Training efficacy tests
- 🖐️ **T503** Finger assignments and visual guidance — `░░░░░░░░░░ 0%`
  - [ ] Finger models
  - [ ] Heatmaps
  - [ ] Accessibility tests
- 🌱 **T504** Daily targets and accuracy-first practice — `░░░░░░░░░░ 0%`
  - [ ] Goal engine
  - [ ] Challenge UI
  - [ ] Performance summaries

**Feature inventory** (computed from associated task progress):

- 🎓 Structured curriculum — `░░░░░░░░░░ 0%`
- 🧠 Adaptive exercises — `░░░░░░░░░░ 0%`

## 📊 M3 — Statistics and achievements — `█░░░░░░░░░ 8%`

**Acceptance gate:** Reproducible longitudinal statistics and achievements.

### 📈 P6 — Analytics and progression — `█░░░░░░░░░ 8%`

- 🏅 **T601** Records by layout and test mode — `███░░░░░░░ 33%`
  - [x] Basic profile bests tracked — src/app.rs
  - [ ] Segmented history
  - [ ] Versioned scoring policies
- 📉 **T602** Progress charts and consistency — `░░░░░░░░░░ 0%`
  - [ ] Timeseries store
  - [ ] Graphs and filters
  - [ ] Consistency formulas
- 🔥 **T603** Key timing and error heatmaps — `░░░░░░░░░░ 0%`
  - [ ] Event recording
  - [ ] Aggregations
  - [ ] Visual heatmaps
- 🏆 **T604** Achievements and streaks — `░░░░░░░░░░ 0%`
  - [ ] Rules and earned records
  - [ ] Celebration UI
  - [ ] Optional streaks

**Feature inventory** (computed from associated task progress):

- 📊 Statistics — `█░░░░░░░░░ 11%`
- 🏆 Achievements — `░░░░░░░░░░ 0%`

## 🎨 M4 — Profile card studio — `█░░░░░░░░░ 13%`

**Acceptance gate:** Accessible exportable SVG and Markdown cards with a friendly finch identity.

### 🐤 P7 — Widgets and publishing — `█░░░░░░░░░ 13%`

- 🪄 **T701** GitHub-safe SVG card — `█████░░░░░ 50%`
  - [x] SVG generator coded — src/export.rs
  - [x] XML escaping and test authored — src/export.rs
  - [ ] Rendered validation on GitHub
  - [ ] Filesystem export test passes
- 🎛️ **T702** Widget designer — `░░░░░░░░░░ 0%`
  - [ ] Template system
  - [ ] Color and type controls
  - [ ] Live preview
- 📝 **T703** Markdown, PNG, JSON exports — `░░░░░░░░░░ 0%`
  - [ ] README Markdown
  - [ ] PNG renderer
  - [ ] Privacy-aware data exports
- 🔄 **T704** Optional GitHub profile updates — `░░░░░░░░░░ 0%`
  - [ ] Explicit opt-in
  - [ ] Safe credentials
  - [ ] Offline failure handling

**Feature inventory** (computed from associated task progress):

- 🐤 GitHub widget — `██░░░░░░░░ 17%`
- 🔄 Publishing automation — `░░░░░░░░░░ 0%`

## 🌎 M5 — Multilingual typing — `░░░░░░░░░░ 0%`

**Acceptance gate:** Correct Unicode, IME composition, RTL and per-language corpora.

### 🗣️ P8 — Languages and input methods — `░░░░░░░░░░ 0%`

- 🧾 **T801** Language packs and corpora — `░░░░░░░░░░ 0%`
  - [ ] English/Spanish data
  - [ ] Corpus provenance and license
  - [ ] Difficulty grading
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

- 🌏 Languages — `░░░░░░░░░░ 0%`
- 🔤 Unicode correctness — `░░░░░░░░░░ 0%`

## 📦 M6 — Release quality — `██░░░░░░░░ 17%`

**Acceptance gate:** Automated regression, live QA and reproducible Linux packages.

### 🧪 P9 — Acceptance and release — `██░░░░░░░░ 17%`

- 🧪 **T901** CI and acceptance-ledger validation — `███████░░░ 67%`
  - [x] Workflow authored — .github/workflows/ci.yml
  - [x] CI build/test green — GitHub Actions run 39867817749: cargo check and cargo test passed
  - [ ] Roadmap drift verified
- 🪟 **T902** Live Hyprland automation — `░░░░░░░░░░ 0%`
  - [ ] Headless run
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

- 🧪 Automated QA — `███░░░░░░░ 34%`
- 🚀 Release readiness — `░░░░░░░░░░ 0%`

