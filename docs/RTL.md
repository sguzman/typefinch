# Arabic and Hebrew input: implementation and acceptance

## Production egui 0.35 shaping correction — verified October 25, 2026

[Full production CI 40235788506](https://github.com/sguzman/typefinch/actions/runs/40235788506) passed the native Arabic/Hebrew selection, byte-exact source Undo, X11 and nested Weston Wayland regressions after upgrading to egui 0.35. Upstream [egui bug #8576](https://github.com/emilk/egui/issues/8576) generated more shaped RTL glyph slots than Unicode source scalars and broke pointer-cursor indices. The fix now lives in the pinned, licensed [epaint 0.35.0 path dependency](../third_party/epaint-0.35.0/UPSTREAM.md) and is included in the reproducible Cargo.lock. No character-proportion guess or downgraded selection assertions were accepted.

**Boundary:** these tests establish the documented synthetic X11 and nested Wayland scenarios; they do not establish arbitrary complex BiDi editing, real Hyprland/focus/IME candidate overlays, or physical-keyboard attribution. T804's broader RTL acceptance remains open.

**Native Wayland RTL physical-position emulation — verified October 26, 2026.**
[Production CI 40247820451](https://github.com/sguzman/typefinch/actions/runs/40247820451)
passed **254 Rust tests** and the full native GUI suite. The new
`tests/rtl_wayland_emulation_smoke.sh` exercised the production egui/winit
Wayland `wl_keyboard` route under disposable nested Weston with a **US**
virtual host keymap while Typefinch independently compiled installed Arabic
(`ara`) and Hebrew (`il`) XKB software-emulation layouts. Each real native
Wayland fixture scored **4/4** expected Unicode graphemes at 100% accuracy,
saved exactly one aggregate Custom/Emulated SQLite record with the matching
layout, and verified that the reference text and source path were absent
from preferences. The companion JIS native Wayland regression also
passed **4/4**, with mode/conversion controls unscored.

The new cases use synthetic XTest input passed through the X11 Weston
parent to a genuine child Wayland surface. They do **not** verify
connected Arabic/Hebrew hardware, Hyprland seat or physical-device
identity, Fcitx/IBus composition or candidate selection, the breadth of
Unicode BiDi editing, or real user skill. The **T804 RTL tests gate
remains unaccepted at 67%** until that broader acceptance evidence exists.




## What works in source

Typefinch scores input against the reference in **logical Unicode extended
grapheme order**, including Arabic vowel marks, Hebrew niqqud and mixed-script
text. It does not reverse a string in order to imitate right-to-left layout.

For passages containing Arabic/Hebrew script ranges, the typing reference
widget now delegates text layout and glyph rasterization to the pure-Rust
`cosmic-text` engine. This supplies OpenType shaping and Unicode bidirectional
layout. Adjacent reference graphemes with the same feedback color form one
logical style run; colored missed graphemes remain tied to the correct
reference index before the shaper maps them into visual glyphs.

When a test is running, the text shaper maps the logical reference grapheme
boundary into a **visual caret position**. The native typing widget draws the
caret at the result and offers that rectangle to egui's IME output. The
conversion accounts for multiple lines and rejects byte offsets inside
UTF-8 scalars and combining-grapheme clusters. A missing or invalid
coordinate never triggers fabricated physical keyboard events.

The custom-passages editor remains a regular egui multiline control so that
its existing keyboard and clipboard editing behavior is preserved. Because
egui's complex-script editor selection is not fully accepted, Typefinch
additionally displays a separate, read-only COSMIC Text preview of up to
160 graphemes. When the raw editor owns focus, the preview window follows its
primary character cursor, snaps interior Hebrew/Arabic combining marks and
joined emoji to valid grapheme boundaries, and shows a shaped visual caret.
**This does not replace or certify the native editable selection/IME model.**

All source text, scoring and preview textures are local and in-memory.
No keyboard hooks, telemetry, unrequested files, public statistics or
privileged input-monitoring APIs are involved.

## Evidence

- CI [40052914361](https://github.com/sguzman/typefinch/actions/runs/40052914361)
  passed the integrated shaped-reference tests and native Arabic/Hebrew
  screenshot fixtures. Visual review showed correctly ordered, joined RTL
  reference words, diacritics and numeric suffixes; the old egui reference
  path visibly reversed that content.
- CI [40053309856](https://github.com/sguzman/typefinch/actions/runs/40053309856)
  passed logical-to-visual cursor unit tests and the complete native X11
  suite. It did not yet independently screenshot the caret during live typing.
- CI [40054099986](https://github.com/sguzman/typefinch/actions/runs/40054099986)
  passed **193 Rust tests** and the complete native graphical suite.
  The corrected native fixture runs Arabic and Hebrew in **separate disposable
  app processes** with separate XDG settings. A synthetic-only script-class
  and grapheme-count audit must agree with the fixture before any screenshots
  are accepted. Both independent RTL reference screenshots, running caret
  images and deliberately incorrect ASCII-key error images were inspected.
  The corresponding reference glyph is highlighted in red and text shapes
  right-to-left in both languages.
- An earlier screenshot test falsely reused the Arabic passage for its
  "Hebrew" case. That evidence was explicitly rejected, the fixture was
  isolated, and the new run passed. No raw passage content is written to
  the test-only import audit.
- Multiline caret, grapheme boundaries, transparent bounded textures and
  actual error-colored raster differences have synthetic Rust coverage.
  These results are **not** live RTL keyboard typing, compositor IME,
  editor-selection or physical Hyprland acceptance.

## Responsive wrapping evidence

CI [40054677101](https://github.com/sguzman/typefinch/actions/runs/40054677101)
passed the complete native regression suite with a third, independently
imported long Hebrew fixture. `src/rtl_shaping.rs` bounds the grapheme
reference window to the available width and varies the transparent texture
height with estimated line occupancy. The native image
`rtl-hebrew-long-error.png` was inspected: connected Hebrew words wrap
into four right-to-left lines, the first logical error is red, the shaped
caret remains visible and the UI reports the bounded reference window
(1–184 of 247 graphemes). The read-only editor preview likewise wraps;
the sidebar itself remains scrollable. The renderer caps image dimensions
and never reverses the source passage.

This is **bounded, cursor-centered visual pagination**, not a promise that
all reference characters appear simultaneously or that the multiline
editable text control has a correct BiDi selection model. Pathological
very-wide glyph clusters, mixed paragraph direction changes and native
IME caret movement still need dedicated end-to-end testing.

## Custom multiline paragraphs and scored virtual XKB input

Custom passages now retain **meaningful hard line breaks** instead of flattening
all paragraphs into spaces. CRLF and CR are normalized to LF; runs of
horizontal whitespace are collapsed, and empty-only lines are omitted.
An Enter keypress is scored against a retained LF only while the focused
typing session expects that grapheme. Provisional IME candidates never count
as Enter; a duplicate platform text event in the same frame is suppressed.
Other control characters remain unscored. Ordinary benchmark/lesson content
keeps its previous whitespace-normalization policy.

[CI 40055443243](https://github.com/sguzman/typefinch/actions/runs/40055443243)
passed both pure-Rust multiline normalization/scoring tests and real X11
**F11 import → F5 start → F2 typing focus → `a`, Enter, `b`**:
three correct graphemes were persisted once in a Custom SQLite record, with
neither the fixture source nor file path saved to settings. The complete
native GUI suite also passed. The independent mixed-script RTL screenshot
`rtl-mixed-error.png` from this run was visually inspected: Arabic-base,
English-base, and Hebrew/Arabic mixed paragraphs now appear on three
separate lines in the active scored reference, with a red first logical
mistake. Earlier screenshots showed an erroneously flattened single line.

A second native scenario now uses **actual X11 virtual key-position events**
routed through installed XKB Arabic (`ara`) and Hebrew (`il`) software
emulation. It requires a complete four-grapheme logical word from each,
rather than counting ASCII `x` as a script keystroke, and checks the
finished score and layout/input-mode metadata in SQLite. The four-character
key map is independently asserted in Rust. The dedicated native GUI
step passed in [CI 40055821789](https://github.com/sguzman/typefinch/actions/runs/40055821789).
These are emulated X11 input tests, **not** Fcitx/IBus composition, physical
device attribution or production Hyprland acceptance.

## Native editor-caret and mixed-layout integration

[CI 40058295306](https://github.com/sguzman/typefinch/actions/runs/40058295306)
passed the complete source and native GUI suite. A keyboard-only `Ctrl+E`
shortcut opens the ordinary writable custom-passage editor (no new
privileged input API). Its separate COSMIC Text preview now follows the
native egui primary cursor and keeps up to 160 graphemes around that
position. A synthetic-only metadata audit asserts that `Ctrl+Home`
and `Ctrl+End` move the logical cursor **0 → 17** for the Arabic fixture
and **0 → 15** for Hebrew, with a positioned shaped caret in the visible
window. The source text, selection and file path are not recorded by this
audit. Independent native screenshots were inspected: the shaped caret
moves visibly through Arabic/Hebrew words with an ASCII numeric suffix.
Selection within the raw egui editor is still not accepted.

The same passing run actually switched the isolated **X11 OS** keyboard
layout `ara` → `us` → `il` *during a single scored Custom session*.
The synthetic passage contained Arabic, Latin and Hebrew characters.
It finished at **13/13 correct graphemes**, with `input_mode=OS` in the
disposable SQLite history. The four archived screenshots
`rtl-native-mixed-start.png`, `rtl-native-mixed-arabic.png`,
`rtl-native-mixed-latin.png` and `rtl-native-mixed-hebrew.png`
were inspected. They show properly joined, bidirectionally ordered text
and the scored progression **0 → 5 → 9 → 13**; the Hebrew image shows a
finished, fully green passage. Earlier isolated native tests additionally
deliberately mistyped and corrected one Arabic/Hebrew key; the final
4/4-correct record retained one raw mistake and one Backspace correction.

A real QA failure was retained as evidence: the original four-screen
capture ran against the synthetic **2-second** timed fixture, which
correctly stopped at 10/13 characters. A new, explicitly gated
`TYPEFINCH_QA_DURATION_SECONDS=15` setting extends only this
disposable screenshot fixture. All ordinary app durations and existing
2-second deadline regression tests remain unchanged. The corrected
CI run passed. This is **virtual X11 OS layout** and software-XKB QA,
not physical Hyprland keyboard hardware or Fcitx/IBus integration.

## Shaped selection follows the actual native editor

[CI 40113278026](https://github.com/sguzman/typefinch/actions/runs/40113278026)
passed Rust Unicode selection-span tests and native X11 `Ctrl+A` selection
in the **real egui source editor**. The separate COSMIC Text preview now
colors only the logically selected extended grapheme clusters, using
grapheme-aligned UTF-8 boundaries. No source text is reordered, copied into
test logs or persisted. The synthetic-only GUI report records the count of
selected graphemes, not the passage, selected substring or clipboard.

The native fixture asserted **17/17 Arabic** and **15/15 Hebrew** selected
graphemes after `Ctrl+A` and archived
`rtl-arabic-editor-selection.png` and
`rtl-hebrew-editor-selection.png`. Both images were visually inspected:
the shaped preview visibly changes the selected glyph colors, separately
from the editable source field. [CI 40113785730](https://github.com/sguzman/typefinch/actions/runs/40113785730)
then passed a stronger real GUI regression comparing crops of **only the
shaped preview**, so selecting text exclusively in the raw editor cannot
falsely satisfy visual acceptance.

This is selected-glyph **feedback** in a read-only rendered preview, not
a replacement text editor. Native bidirectional editor hit-testing,
selection motion across mixed RTL/LTR runs, arbitrary partial selections,
platform IME preedit overlays and physical Hyprland keyboard QA remain
unaccepted. T804 retains its existing acceptance percentage.

## Native first-script-grapheme selection acceptance

[CI 40127907188](https://github.com/sguzman/typefinch/actions/runs/40127907188)
passed the complete Rust and native graphical suite, including
keyboard-operated **partial selection of the first actual Arabic and Hebrew
script grapheme** in the editable egui source field. The source is disposable
test text with combining vowel marks and later Latin digits. The fixture
requires exactly one *logical extended grapheme* to be selected and its
cursor at logical index 1, rather than accepting selection of the trailing
ASCII digit. The independent COSMIC Text preview then changes **191 pixels
for Arabic and 224 for Hebrew** relative to the same unselected starting
caret. Both resulting `rtl-*-editor-partial.png` screenshots were opened
and visually inspected: the correct initial right-to-left script glyph is
highlighted in the shaped preview.

The earlier [CI 40126877549](https://github.com/sguzman/typefinch/actions/runs/40126877549)
passed a weaker partial-selection test that only selected the final numeral;
that result was **not** used as first-script-grapheme acceptance.
[CI 40127603470](https://github.com/sguzman/typefinch/actions/runs/40127603470)
found that one native Shift+Arrow step could stop *inside* an Arabic marked
grapheme, yielding zero complete selected graphemes in the shaped preview.
That earlier test advanced a bounded number of scalar cursor positions
before accepting a whole grapheme. A later application correction in
[CI 40128888077](https://github.com/sguzman/typefinch/actions/runs/40128888077)
now snaps a **single native Shift+Arrow** to the complete marked cluster.
The test passed **221 Rust tests** and confirmed the first Arabic and Hebrew
script graphemes in the actual native editor and shaped preview, changing
191 and 224 preview pixels respectively.

[CI 40129503350](https://github.com/sguzman/typefinch/actions/runs/40129503350)
also passed 221 Rust tests and the full native GUI suite after correcting
the **opposite-direction Shift+Arrow** path. The earlier helper chose a
grapheme boundary based only on the selection anchor, potentially trapping
reverse navigation at the same selected cluster. The corrected helper uses
the active cursor position *before and after* the native key event, preserves
the native selection anchor and horizontal position, and snaps in the actual
movement direction. Both real Arabic and Hebrew sessions selected exactly
one initial grapheme and then retracted to zero selected graphemes with one
opposite Shift+Arrow. Native focus and IME events are not intercepted.

[CI 40129044449](https://github.com/sguzman/typefinch/actions/runs/40129044449)
additionally exercised the actual editable control on three *mixed*
Arabic, Latin and Hebrew paragraphs. Moving from Ctrl+Home to Ctrl+End
preserved the logical cursor bounds (0 to 54 graphemes), with the separate
shaped visual caret moving from vertical pixel 0 to 60. Both archived
full-frame mixed-editor screenshots were inspected: the caret appears in
the first and last paragraph respectively, without text loss or a
selection. This is **specific X11 input evidence**, not proof of arbitrary
mixed-direction mouse hit-testing or physical Wayland correctness.

[CI 40130276882](https://github.com/sguzman/typefinch/actions/runs/40130276882)
also accepted ordinary **unshifted** Arrow navigation. The run passed
222 Rust tests and the native GUI suite. One arrow advances through the
entire first marked Arabic/Hebrew grapheme, and the opposite arrow returns
to the anchor, without a selection (0→1→0 in both languages). The native
IME, shortcut, mouse-selection and text contents are not rewritten by this
endpoint adjustment.

T804 **Shaping and caret** advances to accepted for these X11
reference/editor pathways, while **RTL tests** remain unaccepted for
arbitrary mixed-run editing, candidate overlays, hardware focus, and
physical Wayland/Hyprland sessions. The project's unrounded aggregate
moved from 86.61% to 87.44%; the published whole-number average remains
87% by the ledger's established rounding rule.

## Mixed-script keyboard selection and visible native source hit-testing

[CI 40131452479](https://github.com/sguzman/typefinch/actions/runs/40131452479)
passed 222 Rust tests and the full native X11 GUI suite. The ordinary
multiline source editor extended its selection from **22 to 37** logical
graphemes with two Shift+Down events and retracted to **22** with Shift+Up.
The separate COSMIC Text shaped preview cursor visited visual rows
**0 → 30 → 60 pixels** while its selected-span color changed independently
of the raw editor. All three synthetic-only screenshots were inspected:
the Arabic-containing, Latin, and mixed Hebrew/Arabic lines remain legible
and selection feedback follows the logical range. No text was stored.

[CI 40132170740](https://github.com/sguzman/typefinch/actions/runs/40132170740)
passed all 222 Rust tests and the complete native GUI suite, including
**actual mouse hit-testing** of the ordinary editable RTL source control.
Disposable fixed-window clicks on the Latin paragraph and then the mixed
Hebrew/Arabic paragraph moved the logical caret from grapheme **23 to 38**
and its independent shaped preview from visual row **30 to 60 pixels**.
The two archived screenshots were opened and visually inspected: the
native source caret and the separate shaped caret appear on the expected
different paragraphs, without adding a selection or modifying the source.
This accepts those two X11 mouse-hit-test points, not arbitrary BiDi
hit-testing, drag selection, physical keyboards or Wayland IMEs.

## Native grapheme editing, pointer dragging and virtual Wayland

[CI 40132824689](https://github.com/sguzman/typefinch/actions/runs/40132824689)
passed 222 Rust tests and the native GUI suite, including real
**selection → Backspace → Undo** of the first marked grapheme in the
ordinary Arabic and Hebrew source editors. The separate shaped audit
measured grapheme counts **17 → 16 → 17** (Arabic) and
**15 → 14 → 15** (Hebrew), preserving a single whole-grapheme
selection and an unselected logical anchor after deletion. This validates
the tested deletion/Undo sequence, not every edit-history transition.

[CI 40133438367](https://github.com/sguzman/typefinch/actions/runs/40133438367)
also passed 222 Rust tests and the complete native X11 GUI suite. Real
pointer down/move/up gestures selected **15 logical graphemes** between
the Latin paragraph and the mixed Hebrew/Arabic paragraph in both
directions; the shaped caret reached rows **60 and 30 pixels**.
Both archived full-frame drag screenshots were inspected: the raw
multiline editor and separate shaped preview show the expected selected
middle paragraph and differing active-caret locations. These are two
specific bidirectional pointer ranges, not a general guarantee for every
visual or logical edit position.

[CI 40133580017](https://github.com/sguzman/typefinch/actions/runs/40133580017)
passed the **entire native CI suite (222 Rust tests)** and new authentic
Wayland/wl_keyboard input checks. A disposable Weston compositor receives
synthetic hardware keyboard events from its X11 parent and delivers
them to the unmodified Typefinch Wayland surface. Both imported
Arabic and Hebrew marked-text fixtures passed single-grapheme
**Shift+Arrow selection and opposite-direction retraction**
(0 → 1 → 0), with no source passage or local path saved to settings.
The two archived Weston screenshots were opened and inspected; they
show script-specific selection in the native editable field and the
separate shaped preview. This is **virtual Wayland**, not physical
Hyprland, real Fcitx/IBus candidate interaction or multi-device hardware
routing.

An earlier [CI 40133277469](https://github.com/sguzman/typefinch/actions/runs/40133277469)
passed the Arabic Wayland case but failed when Weston 13's X11 backend
crashed during rapid client teardown before the Hebrew case. The accepted
fixture reuses one live Wayland client for both sequential imports,
instead of misclassifying that compositor lifecycle crash as a typing
engine result.

## Unaccepted scope

T804 is not automatically complete because shaped screenshots are legible.
Real Arabic/Hebrew **physical** keyboard and IME composition, complete
bidirectional selection in the editable control, focus changes under
Hyprland, atypical punctuation and production IME cursor overlays still
need end-to-end QA. The adaptive shaped reference window addresses ordinary long RTL
wrapping in native X11, but atypical glyph widths, RTL/LTR paragraph changes
and native editor-level selection/IME behavior remain open.

There must be no broad claim of bidirectional editing or screen-reader
support until those paths are demonstrated. In particular, correct
appearance of a read-only reference or a nonblank native screenshot does not
prove that the UI's editable text caret or compositor IME behaves correctly.

The acceptance tracker is [progress.json](progress.json); human-readable
percentages are rendered from it. Failed acceptance evidence is kept
distinct from feature planning. No real private passage is needed in CI.

## Native Weston/Wayland RTL editing and source-isolation acceptance

[CI 40141102036](https://github.com/sguzman/typefinch/actions/runs/40141102036)
passed **230 Rust tests and the entire native GUI suite** on
[`f025be5`](https://github.com/sguzman/typefinch/commit/f025be5f3d35533da6a9989507e64e71a6bb274a).
Actual egui TextEdit input through a nested Weston Wayland session verified
independent marked-grapheme selection, deletion, and Undo:

- Arabic: 17 → 16 → 17 graphemes after selection, Backspace, and Undo.
- Hebrew: 15 → 14 → 15 graphemes under the same operations.
- Whole-range Ctrl+Shift+End / Ctrl+Shift+Home: 0 → 17 → 0 Arabic,
  and 0 → 15 → 0 Hebrew selected graphemes.

The pair of archived `typefinch-wayland-rtl-*-whole.png` screenshots was
opened and visually inspected; the actual editable field and separately shaped
preview both depict the full selection. The fixture additionally requires a
pixel-difference assertion against an unselected baseline, without recording
source text or local input paths in machine-readable reports.

This test found a genuine old-editor-history bug in failed
[CI 40140798070](https://github.com/sguzman/typefinch/actions/runs/40140798070):
after importing the Hebrew passage, Undo incorrectly restored the
**previous Arabic document's length** (15 → 14 → 17). Typefinch now resets
the egui TextEdit undo history and stale cursor state whenever a new private
file replaces the passage or a generated practice drill replaces the editor
source. The passing rerun verifies the corrected cross-import lifecycle
without restarting the Wayland client, which previously caused Weston teardown
failures.

The same passing native GUI suite exercised mixed Arabic/Latin/Hebrew pointer
drags in both directions (15 selected graphemes, active caret rows 60/30).
Production pointer-drag ranges now expand partial Arabic vowel marks, Hebrew
niqqud or joined emoji to complete Unicode grapheme boundaries, with an
independent Rust regression. Actual per-pointer marked-glyph hit-testing remains
more limited than complete arbitrary BiDi editor acceptance.

**Scope boundary:** virtual Weston/Xvfb input only. T804's broader RTL-tests
acceptance check remains open for production Hyprland hardware,
native Wayland Fcitx/IBus candidate overlays, arbitrary mixed-script editor
hit-testing and end-to-end IME behavior. No physical-keyboard or full BiDi
acceptance percentage is claimed from this run.

## Native Wayland pointer dragging and clickable COSMIC preview

[CI 40154992897](https://github.com/sguzman/typefinch/actions/runs/40154992897)
passed **235 Rust tests and the full native GUI suite**, including actual
Weston `wl_pointer` forward and backward drags within the *editable*
Arabic and Hebrew marked-script source fields. Selection contained
**9/9 logical graphemes for Arabic** and **11/11 for Hebrew** in both
directions. The native editor source counts remained unchanged; screenshot
pairs (`typefinch-wayland-rtl-*-pointer-{forward,backward}.png`) were opened
and visually inspected. The raw egui selection and separate COSMIC shaped
preview both reflected the selection. This checks two specific same-line
visual ranges in a virtual Wayland compositor, **not arbitrary BiDi
mouse hit-testing** or physical Hyprland.

[CI 40155868325](https://github.com/sguzman/typefinch/actions/runs/40155868325)
then passed **237 Rust tests and the complete native graphical suite**.
`Renderer::hit_test_byte` uses COSMIC Text's shaped bidirectional
`Buffer::hit` method with bounded dimensions, source bytes and pointer
coordinates. It converts a visual click into a logical UTF-8 byte offset,
then snaps to a whole Unicode extended grapheme boundary without reversing
the passage. Unit tests exhaustively exercise selection endpoint pairs for
marked Hebrew, Arabic, mixed Latin/RTL text, Indic conjuncts, joined emoji,
regional-indicator flags and decomposed accents; the hit-test tests check
that valid visual points return genuine grapheme boundaries and invalid
coordinates are rejected.

The native GUI now allows an explicit **click on the separate shaped
preview to position the ordinary source TextEdit caret**. Under the same
disposable Weston session, the real editor's logical cursor moved
**0 → 12 graphemes (Arabic)** and **0 → 9 (Hebrew)** after a normal mouse
click on the shaped glyphs, with zero selected graphemes and the full
original source preserved. `typefinch-wayland-rtl-*-preview-click.png`
screenshots were archived and opened. The shaped preview is still
**not an editable text surface** and does not own keyboard or IME events;
it hands a validated logical caret to the existing editor. No typed text,
source file path, clipboard content or device identity is collected in
normal use, and CI's numeric audit excludes source text.

**Acceptance boundary:** The two tests verify limited native Weston
pointer selection and click-to-caret interactions. They do not establish
correctness of arbitrary mixed-direction selection, native Fcitx/IBus
candidate overlays, physical Hyprland keyboard input, or full assistive
technology integration. T804's broader RTL-tests gate remains **open**
and its tracked completion remains unchanged.


## Native X11 exact-source recovery after mixed-direction editing

[CI 40202172590](https://github.com/sguzman/typefinch/actions/runs/40202172590)
passed **243 Rust tests** and the complete native GUI suite. Its real
Xvfb/egui editor test dragged a selection between Latin and mixed Hebrew/Arabic
paragraphs, replaced the selected range with one ASCII grapheme, then undid
the replacement. It independently asserted the changed selection length
and compared the **entire recovered UTF-8 source byte for byte** against the
original disposable synthetic fixture using the native TextEdit's Ctrl+A/C
and X11 clipboard. This closes a real testing blind spot: equal-length
substitutions or damaged combining marks could previously pass a check
that looked only at grapheme counts.

[CI 40202618058](https://github.com/sguzman/typefinch/actions/runs/40202618058)
extended that byte-exact native editor verification to the individual
Arabic-with-harakat and Hebrew-with-niqqud fixtures after selecting,
Backspacing and undoing one marked grapheme. The native RTL GUI stage
passed; any additional workflow-stage verdict is tracked in GitHub Actions.
The clipboard is read only in isolated synthetic X11 CI; test output
contains no copied passage bytes or private file paths.

**Scope:** This demonstrates specific native X11 editor operations, not
arbitrary bidirectional editing in production, native Wayland Fcitx/IBus
candidate handling, physical Hyprland keyboards or per-device input routing.
T804's broad RTL-tests acceptance check remains open, with no added
percentage credited for these narrower regressions.

## Native shaped-preview drag selection and text recovery

The shaped RTL preview now supports a deliberate **pointer drag** as well as
click-to-caret. It remains noneditable and never owns typed input, clipboard
or IME composition: COSMIC Text hit-tests each visual pointer endpoint into
a logical UTF-8 byte boundary, validates a complete extended grapheme
(including Arabic harakat, Hebrew niqqud and joined glyphs), and sets the
source egui TextEdit's actual selection only when the mouse is released.
This avoids switching keyboard focus halfway through a drag. Native
Wayland click-to-caret remains independently tested.

**Native X11 acceptance evidence:**

- [CI 40205912651](https://github.com/sguzman/typefinch/actions/runs/40205912651)
  passed 244 Rust tests and the complete graphical suite with genuine
  Xvfb pointer selection inside the shaped preview. The source editor
  reported a real, partial Unicode selection and the separate preview
  visibly changed.
- [CI 40206738200](https://github.com/sguzman/typefinch/actions/runs/40206738200)
  passed the same 244 Rust tests and complete graphical suite. For Arabic
  with marks, forward/reverse gestures selected 12 graphemes, moving the
  primary native caret **4 → 16**; for Hebrew with niqqud, the same range
  selected 12 graphemes with caret **3 → 15**. Both source and preview were
  screenshot-tested. No source mutation was needed to create a selection.
- [CI 40207316038](https://github.com/sguzman/typefinch/actions/runs/40207316038)
  passed 244 Rust tests and the full native GUI suite. After reversing the
  visual gesture, the selected Arabic and Hebrew ranges were actually
  replaced with a single Latin grapheme through the source TextEdit, then
  Undo recovered the complete original synthetic UTF-8 passage **byte for
  byte**, not just the same grapheme count.

**Native nested Wayland acceptance evidence:**

- [CI 40206752996](https://github.com/sguzman/typefinch/actions/runs/40206752996)
  passed 244 Rust tests and the complete native GUI suite. Genuine nested
  Weston `wl_pointer` drags across the shaped preview selected **14 Arabic**
  and **10 Hebrew** graphemes in the real editable TextEdit, with unchanged
  total source length and visible selection differences in the shaped image.
  Both `typefinch-wayland-rtl-*-preview-drag-{before,after}.png` screenshots
  were opened and inspected: the native editor displays an actual highlighted
  range and the independently shaped preview changes the corresponding
  connected glyph colors. Existing actual Wayland click-to-caret, typed
  insertion and Undo checks continued to pass.

The Unicode tests exhaustively check ordered pairs of valid grapheme
boundaries and reject code-point offsets inside diacritic/ZWJ clusters or
beyond the source buffer. CI artifacts and QA reports are synthetic;
production input is not logged or captured globally.

**Acceptance boundary:** This is demonstrated on virtual X11 and nested
Weston Wayland, not a physical Hyprland session or arbitrary complex
bidirectional drag topology. Real Fcitx/IBus candidate overlays, every
caret/selection path, physical keyboard attribution and assistive technology
remain outside these tests. T804's overall RTL acceptance check therefore
remains **open at 67%**. The project-wide 90.46% shown in this earlier
evidence snapshot preceded the separately accepted T303 modifier work; the
current total is in [the roadmap](ROADMAP.md). These narrower regressions do
not justify claiming general-purpose production RTL editing is complete.

## Focus loss, bounded drag release and mixed visual rows

- [CI 40209863359](https://github.com/sguzman/typefinch/actions/runs/40209863359)
  passed 245 Rust tests and complete GUI regression. A native X11
  pointer drag released beyond the shaped preview's right image edge
  retained a bounded editable selection of **10 marked Arabic** or
  **7 marked Hebrew** graphemes. Release is clamped to a finite,
  resource-bounded image point; press must originate inside the preview.
- [CI 40211286846](https://github.com/sguzman/typefinch/actions/runs/40211286846)
  passed the complete suite after adding focus-loss and source-replacement
  cancellation. An unfinished preview drag does not retain a stale
  selection anchor across a lost app focus or a newly loaded private passage.
- [CI 40211634328](https://github.com/sguzman/typefinch/actions/runs/40211634328)
  passed **246 Rust tests** and the complete graphical suite. Its new
  narrow wrapping test asserts that lower visual rows of a single long
  mixed RTL paragraph resolve into later *logical* byte offsets without
  splitting marked graphemes. The native X11 GUI then dragged across
  the actual shaped preview from a mixed RTL line into a Latin line:
  the editable source reported **18 selected graphemes**, active caret
  index **28**, and visible selection paint. Replacing that range with
  a Latin grapheme and Undo recovered the exact UTF-8 source bytes.

These tests establish bounded and focus-safe drag interactions over the
specified synthetic X11 and nested Weston scenarios. They do not certify
hardware Hyprland behavior, arbitrary multiline BiDi interactions, native
Fcitx/IBus candidate positioning, or every accessibility pathway. T804's
broader RTL-test gate remains open at 67%; the project-wide percentage
reported at the time was unchanged, but the current total is tracked in
[the evidence-backed roadmap](ROADMAP.md).

## Native source-anchored Shift-click

The shaped, **noneditable** Arabic/Hebrew preview now accepts Shift+click
to extend an existing selection in the actual egui multiline TextEdit.
It reuses the original logical selection anchor (including an anchor from
a prior keyboard or pointer selection), snaps both endpoints to whole
Unicode graphemes and preserves reverse selection orientation. Plain
click collapses the selection; drag selection remains independently
supported. The source TextEdit still owns all typed input, Undo and IME
events, with no global event subscription.

[Full CI 40217555758](https://github.com/sguzman/typefinch/actions/runs/40217555758)
passed **247 Rust tests**, the complete nested Weston native Wayland GUI
suite, and all native X11 tests:

- **Wayland:** real `wl_pointer` Shift+click on Arabic extended then
  retracted a source selection **16 → 2 graphemes**; Hebrew **12 → 2**.
  A subsequent ordinary click collapsed the range. The existing
  native pointer drag, IME-preedit guard and source Undo regressions
  continued to pass.
- **X11:** real XTest Shift+click on marked Arabic extended then
  retracted selection **16 → 4 graphemes**; marked Hebrew **7 → 3**.
  Ordinary click collapsed both selections.
- **Actual edit and recovery:** the source editor replaced a
  Shift+click-selected Arabic range of **10/17** graphemes with one
  Latin grapheme, leaving **8** total. Native Undo restored **17**;
  a UTF-8 clipboard comparison proved **byte-exact recovery** of
  all original marks. The Hebrew case selected **7/15**, replacement
  left **9**, Undo restored **15** and the same exact-source check passed.
- **Pure unit checks:** direct whole-grapheme click mapping rejects
  invalid offsets, plain click collapses, and repeated forward/reverse
  Shift+click preserves the original source anchor across both scripts.

The earlier failed X11 test asserted partial selection at a Hebrew
visual coordinate that actually selected all 15 graphemes; an archived
screenshot confirmed full native selection. Another discarded test
replaced nearly all Arabic glyphs, causing the app correctly to exit
RTL-preview mode, so its RTL-only numeric audit stopped updating.
The accepted test now chooses previously measured partial-range
positions and separately retains near-complete-selection coverage.
Neither failure was marked as acceptance.

The implementation also cancels transient RTL preedit ownership on
application focus loss, and the preview instructions now mention
Shift+click. This remains evidence for the specified virtual X11
and nested Wayland behavior, **not** full native Hyprland RTL/IME
acceptance. T804 stays **67%** and overall acceptance as reported in the current roadmap until
the broader remaining gates can genuinely be verified.

## Native Wayland ordinary OS-input Arabic/Hebrew scoring (October 26, 2026)

[Full CI 40340041814](https://github.com/sguzman/typefinch/actions/runs/40340041814)
passed **264 Rust tests and the complete graphical suite**, including
the strict RTL Wayland acceptance runner. Four independent Xvfb/Weston
compositor processes were required rather than restarting clients after
a completed session.

| Input path | Arabic | Hebrew |
|---|---|---|
| Native Wayland physical-position **software emulation** | 4/4 after one corrected error; separate 5/5 hard-Newline passage | 4/4 after one corrected error; separate 5/5 hard-Newline passage |
| Native Wayland compositor-translated **OS input** | 4/4 after one corrected error | 4/4 after one corrected error |

The new **OS-input** case runs the actual Wayland child with software
emulation disabled. It first loads a disposable custom reference under the
X11 parent's US keyboard map, then applies the target Arabic or Hebrew
XKB map to the disposable Xvfb parent and uses numeric physical X11
keycodes through the nested Weston keyboard seat. The Weston keyboard
configuration is explicit for each separate language process. Each
result must be a real finished typing session, retain one mistake and
one correction across five attempts, and persist **OS** input-mode
provenance in isolated SQLite. The source and import path must not
appear in preferences. These checks do not replace the independent
**Emulated** scores or the hard-Newline regressions.

**Failures remain visible:** CI
[40337331095](https://github.com/sguzman/typefinch/actions/runs/40337331095)
failed because an unconfigured Weston instance did not deliver expected
OS-mode text. CI
[40338576636](https://github.com/sguzman/typefinch/actions/runs/40338576636)
still failed with an explicit Weston keymap and Alt+Shift alone. The
guarded runner correctly declined to retry these app-scoring failures.
The accepted configuration additionally changed the isolated parent X11
layout and sent physical keycodes. No production keymap was changed.

**Acceptance boundary:** This tests ordinary compositor-translated RTL input
on a nested virtual Wayland seat, not physical Hyprland keyboards, scoped
per-device capture, Fcitx/IBus candidate overlays, arbitrary mixed-direction
editing or full accessibility integration. T804's broad RTL test gate
remains open; do not claim real hardware or comprehensive RTL acceptance.

## Native Wayland mixed-script OS input and atomic caret QA (October 26, 2026)

[Full CI 40346017191](https://github.com/sguzman/typefinch/actions/runs/40346017191)
passed **265 Rust tests and the complete graphical suite**. The RTL
acceptance runner now exercises **five independently
isolated Weston compositor cases**: Arabic Emulated, Hebrew Emulated, Arabic
OS-input, Hebrew OS-input, and a mixed Arabic/Latin/Hebrew OS-input case.
The Emulated cases each retain their earlier separate four-grapheme,
mistake-correction and hard-newline five-grapheme sessions.

The new **mixed OS-input** case imports a synthetic private Custom reference
of exactly 13 extended graphemes, then scores the actual focused native
Wayland input session in logical order through Arabic -> US Latin -> Hebrew.
For each run it changes both the disposable parent Xvfb XKB layout and the
nested Weston keyboard group, with numeric X11 physical-key input. The
native Typefinch software emulator is disabled. The single completed
session must persist 13/13 correct graphemes, exactly 14 attempts including
one corrected mistake, and correct **OS** input-mode provenance in SQLite.
Neither the reference text nor import path may enter preferences. The
strict runner does not retry scoring assertion failures as compositor
bootstrap failures.

**QA-read race corrected:** In [commit 9fa0228](https://github.com/sguzman/typefinch/commit/9fa02284d4f40aa5d75b4f2b8fa7a7b2db31b984),
the synthetic-only RTL editor caret audit switched from in-place
truncate-and-write to private same-directory staging plus atomic rename.
The previous CI
[40344362111](https://github.com/sguzman/typefinch/actions/runs/40344362111)
had failed an unrelated existing mixed-script X11 selection check because
it copied an incomplete JSON snapshot between GUI frames. A concurrent
writer/reader Rust regression now requires complete JSON on every read,
and verifies no symlinked external file is modified. In the successful run
the full original RTL X11 selection/edit/Undo assertions remained enabled
and passed; no retry or weaker selection check was introduced.

**Remaining boundary:** The test is synthetic input through a genuine nested
Wayland compositor, not a connected physical Hyprland keyboard, a native
Wayland IME candidate overlay, or unrestricted bidirectional editing.
It provides additional evidence within T804 without passing T804's
broader RTL acceptance check.
