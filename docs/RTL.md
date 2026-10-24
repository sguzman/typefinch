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
160 graphemes. This makes the preview visually useful without pretending the
editable caret is reliable.

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

## Unaccepted scope

T804 is not automatically complete because shaped screenshots are legible.
Real Arabic/Hebrew keyboard and IME composition, complex bidi paragraph
selection, arrow/caret motion **inside the editable custom-text control**,
focus changes under Hyprland, mixed punctuation and cursor overlays
positioned under live input still need end-to-end QA. The adaptive shaped reference window addresses ordinary long RTL
wrapping in native X11, but atypical glyph widths, RTL/LTR paragraph changes
and native editor-level visual caret/selection remain open.

There must be no broad claim of bidirectional editing or screen-reader
support until those paths are demonstrated. In particular, correct
appearance of a read-only reference or a nonblank native screenshot does not
prove that the UI's editable text caret or compositor IME behaves correctly.

The acceptance tracker is [progress.json](progress.json); human-readable
percentages are rendered from it. Failed acceptance evidence is kept
distinct from feature planning. No real private passage is needed in CI.
