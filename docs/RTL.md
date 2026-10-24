# Arabic and Hebrew input: implementation and acceptance

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

The next dedicated native X11 test also probes real mouse clicks in the
Latin and Hebrew/Arabic paragraphs of the same disposable fixture; it
is not accepted until its own end-to-end CI run verifies the expected
logical indices and visually distinct shaped caret rows.

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
