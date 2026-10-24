# 🗺️ Typefinch roadmap

**Source of truth:** [progress.json](progress.json). Percentages are evidence-backed fractions of atomic acceptance checks, averaged into phase/milestone totals. No unverified task reaches 100%.

**Overall:** `█████████░ 85%` across 41 tasks. This is tracked progress, not a claim of runtime readiness.

## 🐣 M0 — Native typing MVP — `█████████░ 93%`

**Acceptance gate:** A working native session, reliable scoring, local profiles, and live Hyprland acceptance.

### 🧱 P0 — Foundation and visual identity — `█████████░ 86%`

- 🦀 **T001** Rust/egui application — `███████░░░ 67%`
  - [x] Native shell committed — Cargo.toml; src/main.rs
  - [x] CI native build passes — GitHub Actions run 39867817749: cargo check succeeded
  - [ ] Live Hyprland window screenshot
- 🎨 **T002** Perceptual OKLCH project color — `██████████ 100%`
  - [x] Color specified — docs/COLOR.md
  - [x] README swatch authored — assets/typefinch-color.svg
  - [x] Contrast and accessibility audit — tools/color_audit.lua and docs/COLOR.md: automated WCAG relative-luminance checks for the README swatch label and light/dark accent/selection text. CI 40039184099: five AA normal-text pairings (5.40, 6.27, 7.55, 6.27, 5.40:1), white-on-brand excluded (2.73:1), 153 Rust tests and native X11 suite green. This scoped brand audit does not accept keyboard-only navigation or Unicode-font QA.
- 🌗 **T003** System, Light and Dark modes — `████████░░ 75%`
  - [x] Three-way selector coded — src/app.rs
  - [x] Preference persistence coded — src/preferences.rs
  - [x] Explicit Light and Dark rendered screenshots pass graphical CI — GitHub Actions 39872686067: light luminance 0.955 vs dark 0.125
  - [ ] Hyprland system theme transition verified
- 🧭 **T004** Navigation and responsive egui shell — `██████████ 100%`
  - [x] Main panels and controls coded — src/app.rs
  - [x] Keyboard navigation audit — src/app.rs Ctrl+N focuses native egui profile name TextEdit without bypassing Create profile validation; tests/keyboard_navigation_smoke.sh uses only X11 key events for Ctrl+N, Tab, Shift+Tab and Enter and captures F1 guide visibility. GitHub Actions 40047048736 passed 168 Rust tests, native profile creation (one new identity and zero score history), F1 guide screenshot changed 107294 pixels, full X11 GUI, compositor and Unicode font QA. This completes shell navigation audit, not live Hyprland or assistive-technology acceptance.
  - [x] HiDPI/small-window screenshots — CI 39955283848: 740×590 narrow and 150% DPI 1279×880 screenshots; both inspected for immediate, legible typing surface

### 🐦 P1 — Profiles and persistence — `██████████ 100%`

- 👤 **T101** Multiple local profiles — `██████████ 100%`
  - [x] Create/select profile UI coded — src/app.rs
  - [x] JSON persistence coded — src/preferences.rs
  - [x] Recovery, collisions, migrations and actual runtime testing — src/preferences.rs: stable profile UUID migrations, missing/corrupt identity guards; src/backup_verify.rs: validated isolated recovery; native X11 recovery and rollback tests, Actions 39967868689
- 🗄️ **T102** SQLite test history — `██████████ 100%`
  - [x] Schema and migrations — src/history.rs: schema user_version=1, idempotent migration; Actions 39870213935
  - [x] Persist each complete test — src/history.rs and synthetic X11 session SQLite assertion; Actions 39870213935
  - [x] Retention and backup tests — src/history.rs: >100-session retention query, VACUUM INTO snapshot test and native F8 backup QA, CI 39895462191
- 🎯 **T103** Per-profile goals and settings — `██████████ 100%`
  - [x] Settings and goals UI — src/app.rs: independent editable WPM/accuracy goals and progress bars; Actions 39888678045
  - [x] Per-profile setting persistence — src/preferences.rs: per-profile goals and migration defaults; Actions 39888678045
  - [x] Safe per-profile goal JSON export/import logic — src/goal_transfer.rs and src/app.rs: bounded/versioned goals-only JSON transfer, atomic 0600 private export, regular-file/symlink rejection, active-profile assignment with failed-save rollback; CI 40025534946: 141 Rust tests and full native graphical suite pass
  - [x] Native GUI goal import/export interaction and persistence — src/app.rs F12 keyboard focus to ordinary goal-import path, Enter validated import, and Shift+F12 explicit JSON export; tests/goals_gui_smoke.sh and tests/goals_gui_assert.lua exercise real native X11 egui interactions, versioned goal transfer, private on-disk settings and unchanged profile identity/test/score fields. GitHub Actions 40041334789: 155 Rust tests, goal imported/exported PASS, custom passage and full GUI QA passed. Not physical Hyprland acceptance.

### ⌨️ P2 — Tests and core scoring — `██████████ 95%`

- ⚡ **T201** Typing input and session lifecycle — `████████░░ 75%`
  - [x] Input handlers implemented — src/app.rs
  - [x] Start/cancel/restart coded — src/engine.rs
  - [x] End-to-end text/paste/focus QA — tests/gui_smoke.sh: real typed/paste prevention and two-window focus-loss cancellation, no SQLite row; Actions 39967868689
  - [ ] IME support
- 📏 **T202** WPM and accuracy calculations — `██████████ 100%`
  - [x] Pure scoring function implemented — src/engine.rs
  - [x] Scoring tests authored — src/engine.rs
  - [x] CI scoring tests pass — GitHub Actions run 39867817749: all 11 Rust unit tests passed
- ⏱️ **T203** Timed tests — `██████████ 100%`
  - [x] Four durations coded — src/app.rs
  - [x] Monotonic timer coded — src/engine.rs
  - [x] Live timing edge-case QA — src/engine.rs and src/app.rs: exact monotonic deadline checked before delayed GUI key processing; focus-loss cancel wins; incomplete grapheme grace never crosses timed cutoff. CI 40038837832 passed 153 Rust tests, native X11 2-second deadline (exact 2000 ms, three pre-deadline keys, late input excluded from SQLite), and full GUI/Wayland/X11 suites.
  - [x] Session history persisted — Xvfb session stored in SQLite during GUI smoke; Actions 39870213935
- 📚 **T204** Word, quote, punctuation and custom tests — `██████████ 100%`
  - [x] Starter English corpus — src/engine.rs
  - [x] Quote passages — src/engine.rs quotes corpus; GitHub Actions 39872686067
  - [x] Dedicated paragraph test content — src/engine.rs: paragraph content; GitHub Actions 39880764327: 39 tests passed
  - [x] Punctuation and numbers — src/engine.rs provides distinct numeric and punctuation corpora; GitHub Actions 39872686067
  - [x] Editable private custom passages — src/app.rs, src/engine.rs: in-memory custom editor and bounded text; Actions 39873831520 and 39880764327
  - [x] Bounded local UTF-8 file import into custom passages — src/engine.rs and src/app.rs: explicit Load local file and drag/drop into in-memory custom passage, 64 KiB/4096 grapheme cap, rejects symlinks/devices/controls and invalid UTF-8; Unicode ZWJ cluster and file fixture QA in CI 40025534946, 141 Rust tests and graphical suite pass
  - [x] Native GUI local-file import interaction and isolation — src/app.rs F11 keyboard navigation and normal local file-path importer; tests/gui_import_smoke.sh plus tests/smoke_assert.lua use real X11 egui focus, Enter and F5/F2 scoring in disposable profiles. CI 40040560022: 155 Rust tests and native GUI suite passed; Custom session 16/16 correct, no passage/path in persisted settings, independent symlink attempt produced no scored record. No live Hyprland or physical-device attribution implied.
- 🧮 **T205** Scoring taxonomy and fair comparisons — `██████████ 100%`
  - [x] Raw vs corrected WPM — src/engine.rs, src/history.rs: raw WPM, attempts and corrected metrics; Actions 39880764327 X11 corrected-error QA
  - [x] Per-language speed definitions — docs/SCORING.md and src/engine.rs: both supported languages use five correctly matched extended grapheme clusters per standardized corrected WPM unit, with explicit comparability limits; test english_and_mexican_spanish_share_explicit_five_grapheme_wpm_units and native X11 QA green in GitHub Actions 40039799888. No CJK/RTL equivalence or active keyboard attribution claimed.
  - [x] Pause/focus/repeat rules — src/app.rs emulated_key_accepted and src/engine.rs canceled-session state semantics; docs/SCORING.md explicitly defines no pause/resume for benchmarks, Escape/focus loss cancellation, fresh restarts, emulated key-repeat and Ctrl/Alt/command filtering, Shift, and honest system committed-text limits. GitHub Actions 40043811020 passed 158 Rust tests including dedicated cancellation/restart and repeat/modifier regressions, native X11 two-window focus cancellation, paste exclusion, exact timed boundary, goal/custom import, heatmap and full graphical QA. No universal physical-repeat detection claimed.
  - [x] Anti-cheat tests — Real X11 Ctrl+V injection excluded from scored session; GitHub Actions 39872686067

**Feature inventory** (computed from associated task progress):

- ⌨️ Typing tests — `██████████ 95%`
- 🐦 Local identity — `██████████ 100%`
- 🌗 Appearance — `█████████░ 92%`

## 🗺️ M1 — Keyboard layout laboratory — `███████░░░ 73%`

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

### 🔌 P4 — Keyboard detection and selection — `██████░░░░ 61%`

- 🔍 **T401** Hyprland device inspection — `███████░░░ 67%`
  - [x] Read-only hyprctl query coded — src/devices.rs
  - [x] Device and active map display coded — src/app.rs
  - [ ] Live connected-keyboard test
- 🎛️ **T402** Multiple connected keyboards — `███████░░░ 67%`
  - [x] Read-only device chooser coded — src/app.rs
  - [ ] Hotplug support
  - [x] Reliable per-device event attribution or explicit unsupported mode — src/devices.rs CapturePolicy::InspectionOnly and src/app.rs: visible no-per-device typing mode, no physical event stream; unit test with two separate keyboard inspection identities, CI 40014109451: complete Rust and native X11 suites passed
- 🪟 **T403** Generic compositor fallback — `████████░░ 75%`
  - [x] Safe Hyprland-unavailable state coded — src/devices.rs
  - [x] Read-only standard Wayland seat capability query — src/wayland_seat.rs, src/devices.rs, src/main.rs and tests/wayland_virtual_seat_smoke.sh: read-only wl_registry/wl_seat capability and optional terminal JSON snapshot, with no wl_keyboard, physical event stream, or device attribution. CI 40037472601 passed 151 Rust tests, six synthetic Hyprland and eight synthetic Wayland scenarios, native headless Weston (0 seats), nested Weston/Xvfb (one keyboard-capable virtual seat and one disabled seat correctly excluded), disconnected/X11/Wayland CLI checks, and full native X11 graphical QA. Active generic XKB identification, real Hyprland hotplug and physical input attribution remain unverified.
  - [ ] Generic XKB seat detection
  - [x] X11 fallback — src/devices.rs: gated read-only setxkbmap -query on genuine X11 sessions, one clearly labeled aggregate seat rather than invented devices; CI 40008974131: 122 Rust tests and dedicated live Xvfb seat probe, plus graphical suite passed
- 🔐 **T404** Optional authorized per-device capture — `███░░░░░░░ 33%`
  - [x] Permission and threat model — docs/DEVICE_INPUT_SECURITY.md: threat boundary, explicit permission requirements, revocation, focus scoping, no global evdev/root capture, no double-scoring and negative acceptance tests; design-only, no capture implementation
  - [ ] Opt-in backend
  - [ ] No global interception verification

**Feature inventory** (computed from associated task progress):

- 🗺️ Layout recognition — `████████░░ 81%`
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
  - [x] Adaptive drill generation — src/engine.rs: deterministic, session-local severity-weighted Unicode grapheme and error-position-specific bigram drills; eight distinct patterns, two-to-eight repetitions, 256-byte bounded source fragments, no whitespace-spanning pairs and no persisted typed text. Engine tests cover Unicode combining and ZWJ clusters, repeated letters in different contexts, memory/reset bounds, and targeted ranking. GitHub Actions 40047696761 passed 173 Rust tests, app build, complete native X11 graphical and compositor suites. T502 training efficacy remains unverified.
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

## 📊 M3 — Statistics and achievements — `██████████ 100%`

**Acceptance gate:** Reproducible longitudinal statistics and achievements.

### 📈 P6 — Analytics and progression — `██████████ 100%`

- 🏅 **T601** Records by layout and test mode — `██████████ 100%`
  - [x] Basic profile bests tracked — src/app.rs
  - [x] Segmented history — src/history.rs: context-specific comparisons and per-profile recent history; Actions 39888678045
  - [x] Versioned scoring policies — src/history.rs: SCORING_VERSION 2 preserves v1 data and excludes cross-version trends; Actions 39896389227
- 📉 **T602** Progress charts and consistency — `██████████ 100%`
  - [x] Timeseries store — src/history.rs: timestamped SQLite session records and comparable context histories; Actions 39880764327
  - [x] Graphs and filters — src/history.rs selectable metric summaries and context filter; src/app.rs 4-metric chart/table UI; CI 39947014970 (66 Rust tests and X11 smoke)
  - [x] Consistency formulas — src/history.rs: mean, best and population standard deviation with unit test; Actions 39888678045
- 🔥 **T603** Key timing and error heatmaps — `██████████ 100%`
  - [x] Event recording — src/engine.rs bounded in-memory key/error interval recording, privacy-safe aggregates; GitHub Actions run 39953436230: 79 Rust tests and native X11 graphical suite passed
  - [x] Aggregations — src/engine.rs deterministic missed-key counts and per-key timing averages; GitHub Actions run 39953436230: 79 Rust tests and native X11 graphical suite passed
  - [x] Visual heatmaps — src/app.rs visual severity heatmap with text miss count on each keycap, plus key_heat_fill light/dark contrast tests; tests/gui_smoke.sh renders a real corrected-error session, captures typefinch-heatmap.png, and counts 3731 native X11 screenshot pixels painted with semantic miss colors. GitHub Actions 40043194648 passed 156 Rust tests, full GUI QA, and native heatmap screenshot verification. Reference-symbol feedback only, not physical-device attribution or live Hyprland hardware.
- 🏆 **T604** Achievements and streaks — `██████████ 100%`
  - [x] Rules and earned records — src/achievements.rs: six deterministic, profile-scoped earned badges with qualification and anti-gaming unit tests; CI 39947014970
  - [x] Celebration UI — src/app.rs: award celebration message and achievement status panel; CI 39947014970 native executable
  - [x] Optional streaks — src/daily.rs: opt-in local-day streak with grace and timezone tests; src/preferences.rs default disabled; CI 39947014970

**Feature inventory** (computed from associated task progress):

- 📊 Statistics — `██████████ 100%`
- 🏆 Achievements — `██████████ 100%`

## 🎨 M4 — Profile card studio — `█████████░ 94%`

**Acceptance gate:** Accessible exportable SVG and Markdown cards with a friendly finch identity.

### 🐤 P7 — Widgets and publishing — `█████████░ 94%`

- 🪄 **T701** GitHub-safe SVG card — `██████████ 100%`
  - [x] SVG generator coded — src/export.rs
  - [x] XML escaping and test authored — src/export.rs
  - [x] Rendered validation on GitHub — assets/typefinch-card-demo.svg is an author-owned synthetic, script-free 550x145 compact card embedded in README. Native CI 40051764866 test tests/github_card_browser_smoke.sh fetched exact SHA from raw.githubusercontent.com, byte-compared against repo source, and loaded the public SVG in headless Chromium: decoded natural dimensions 550x145; browser screenshot typefinch-github-card-browser.png inspected and shows legible bird, labels, numerals and footer. Not real user data and not auto-upload of private profiles.
  - [x] Filesystem export test passes — GitHub Actions run 39873211637: F6 native GUI writes valid SVG and paired Markdown, parsed by XML validator
- 🎛️ **T702** Widget designer — `██████████ 100%`
  - [x] Template system — src/export.rs Classic 550×205 and Compact 550×145 versioned templates, live egui preview and native PNG/SVG export; CI 39949282753 (70 tests and X11 verification)
  - [x] Color and type controls — src/preferences.rs card theme/font settings; src/export.rs static palettes and escaped SVG; native Daybreak/mono export CI 39948044402
  - [x] Live preview — src/app.rs immediate native egui palette/typography preview; Rust tests + graphical native CI 39948522387
- 📝 **T703** Markdown, PNG, JSON exports — `██████████ 100%`
  - [x] README Markdown — src/export.rs produces paired local Markdown/SVG and validates the embed; GitHub Actions 39872686067
  - [x] PNG renderer — src/export.rs resvg/tiny-skia native PNG raster at 550×205, unit test plus F9 X11 GUI and Lua PNG validation; CI 39948522387
  - [x] Privacy-aware data exports — src/export.rs, src/history.rs: entire profile's versioned JSON aggregate export; Actions 39888678045 native F7 export QA
- 🔄 **T704** Optional GitHub profile updates — `████████░░ 75%`
  - [x] Explicit opt-in — src/app.rs optional collapsible public publishing panel defaults off; valid GitHub repo name, fresh explicit public-data checkbox and enabled user click required; consent resets when card or destination changes, single-flight nonblocking worker, no automatic publication. src/publish.rs mock API tests deny missing consent without any API call. CI 40052478666 full green.
  - [x] Safe credentials — src/publish.rs uses existing authenticated GitHub CLI via fixed non-shell argv, GH_PROMPT_DISABLED=1 and bounded timeout; no tokens accepted or stored. Permissions.push verified; target restricted to fixed repository-root typefinch-profile.svg; unrelated and unverifiable remote files refused; existing owned SHA required for safe replacement. Pure mock and native CI 40052478666 green; no user account action performed.
  - [x] Offline failure handling — src/publish.rs mocked network/auth errors fail closed, corrupt/missing/foreign existing card refused, upload failure and wrong response path not reported as confirmed success. UI has single in-flight worker and no deferred upload queue. CI 40052478666 passed unit and full native graphical suite.
  - [ ] Real authorized GitHub publication and verified result

**Feature inventory** (computed from associated task progress):

- 🐤 GitHub widget — `██████████ 100%`
- 🔄 Publishing automation — `████████░░ 75%`

## 🌎 M5 — Multilingual typing — `███████░░░ 67%`

**Acceptance gate:** Correct Unicode, IME composition, RTL and per-language corpora.

### 🗣️ P8 — Languages and input methods — `███████░░░ 67%`

- 🧾 **T801** Language packs and corpora — `██████████ 100%`
  - [x] English/Spanish data — src/engine.rs: English and Mexican Spanish corpora, CI 39895462191
  - [x] Corpus provenance and license — docs/LANGUAGES.md: original Typefinch-authored passages with provenance/license description, CI 39895462191
  - [x] Difficulty grading — src/engine.rs: heuristic Easy/Standard/Challenge word tiers and history isolation, CI 39895462191
- 🔤 **T802** Grapheme and normalization scoring — `██████████ 100%`
  - [x] Grapheme segmentation — src/engine.rs and src/app.rs: unicode-segmentation counts/display/Backspace; Actions 39896389227 (59 tests)
  - [x] Normalization rules — src/engine.rs: NFC canonical comparison, combining-accent completion; Actions 39896389227
  - [x] Regression tests — src/engine.rs: decomposed accent, joined emoji, flag and whole-grapheme Backspace regressions; Actions 39896389227
- ㊙️ **T803** CJK IME composition — `███████░░░ 67%`
  - [x] Committed text semantics — src/app.rs score_system_committed_event accepts system-mode egui::Event::Text and ImeEvent::Commit, excludes noncommitted IME events and Paste, and advertises a focused IME candidate rectangle; synthetic 你好世界 custom passage test validates four graphemes and no scoring of enable/disable/paste. GitHub Actions 40044338842 passed 159 Rust tests including ime_commits_score_once_while_noncommitted_events_do_not plus native X11 GUI suite. No live CJK/Fcitx/IBus integration, cross-script speed policy or RTL acceptance claimed.
  - [ ] CJK integration tests
  - [x] Speed conventions — src/engine.rs Score.cpm counts correctly matched final Unicode grapheme clusters per elapsed minute, independently of raw IME keystrokes; tests cover corrected Chinese, Japanese kana, Korean Hangul, canonical accents and zero-duration bounds. src/app.rs displays corrected CPM separately from standardized WPM; docs/SCORING.md explicitly rejects cross-language equivalence and physical-key attribution. GitHub Actions 40048186051 passed 174 Rust tests and the complete native X11 graphical/compositor suite. Actual Fcitx/IBus CJK integration and RTL acceptance remain open.
- 🪶 **T804** Arabic and Hebrew bidirectional input — `░░░░░░░░░░ 0%`
  - [ ] Shaping and caret
  - [ ] Error highlighting
  - [ ] RTL tests

**Feature inventory** (computed from associated task progress):

- 🌏 Languages — `██████░░░░ 56%`
- 🔤 Unicode correctness — `██████████ 100%`

## 📦 M6 — Release quality — `████████░░ 75%`

**Acceptance gate:** Automated regression, live QA and reproducible Linux packages.

### 🧪 P9 — Acceptance and release — `████████░░ 75%`

- 🧪 **T901** CI and acceptance-ledger validation — `██████████ 100%`
  - [x] Workflow authored — .github/workflows/ci.yml
  - [x] CI build/test green — GitHub Actions run 39867817749: cargo check and cargo test passed
  - [x] Roadmap drift verified — GitHub Actions 39868627924: render_roadmap.py --check passed
- 🪟 **T902** Live Hyprland automation — `███░░░░░░░ 33%`
  - [x] Headless run — Xvfb + Openbox + Mesa launched native egui app, typed 33 chars and captured screenshot; Actions 39870050679
  - [ ] Keyboard/input QA
  - [ ] Theme/layout screenshots
- ♿ **T903** Accessibility review — `██████████ 100%`
  - [x] Keyboard-only audit — src/app.rs native keyboard controls include Ctrl+N profile editor focus, Tab/Shift+Tab/Enter validated creation, Ctrl+Shift+T System/Light/Dark cycle, F1 guide, F2/F5 typing, F3/F4/F10 preview, F6–F9 exports and F11/F12 local imports. tests/keyboard_navigation_smoke.sh and existing GUI/import/goal tests drive real X11 egui controls: CI 40047465930 passed 171 Rust tests, keyboard-created profile, F1 107868-pixel guide delta, saved system/light/dark/system sequence and complete native X11 smoke. Scope is core keyboard-only workflows, not assistive screen reader, actual Hyprland keyboard or RTL caret testing.
  - [x] Contrast/reduced motion — src/app.rs: WCAG-oriented contrast palette tests and reduce-motion timing tests; native light/dark screenshot QA, Actions 39967868689
  - [x] Unicode fonts — src/fonts.rs: read-only installed system Noto CJK/Arabic/Hebrew fallbacks with size/type guards and egui font glyph coverage; GitHub Actions 40046042312 green, including dedicated TYPEFINCH_QA_UNICODE_FONTS test, compilation and full X11 QA. Glyph availability only: native shaping, RTL caret, and real IME acceptance remain open.
- 🚀 **T904** Reproducible Linux release — `███████░░░ 67%`
  - [x] Versioned packages — GitHub Actions 39958743350: optimized Linux x86_64 build, deterministic allowlisted tar.gz + SHA-256, install/upgrade/rollback and unmanaged-executable refusal
  - [x] Install/rollback tests — GitHub Actions 39958743350: optimized Linux x86_64 build, deterministic allowlisted tar.gz + SHA-256, install/upgrade/rollback and unmanaged-executable refusal
  - [ ] MVP gate accepted

**Feature inventory** (computed from associated task progress):

- 🧪 Automated QA — `███████░░░ 67%`
- 🚀 Release readiness — `████████░░ 84%`

