# Arabic and Hebrew: current scope and acceptance

Typefinch operates on **Unicode logical grapheme order** for scoring and
reference feedback. A mistake is matched to the reference position rather
than reversing the typed string to approximate visual RTL order. Adjacent
equally colored graphemes share an egui text run so the unentered part of an
Arabic word is not needlessly split into individually formatted letters.

Automated unit tests check Arabic marks, Hebrew niqqud, mixed Latin/RTL
passages, cursor highlighting, windowed reference positions, canonical
equivalence, and preserved string order. Native X11 automation imports
author-authored synthetic Arabic and Hebrew passages with the ordinary
keyboard-operated Custom file importer, captures separate screenshots, and
asserts that the actual egui window changes. CI checks availability of OS
Noto glyph fallbacks. The fixtures contain no private texts, device events,
or account identifiers.

**Not yet accepted:** complete Arabic contextual glyph shaping, proper
Unicode Bidirectional Algorithm reordering across mixed paragraphs, visual
selection/caret positioning, edit controls in RTL, or real RTL IME/compositor
entry. egui 0.33.3 text/layout has limitations in complex bidirectional
paragraphs. Do not claim that styling contiguous logical runs alone solves
OpenType shaping or UAX #9. End-to-end RTL success requires rendering and
input tests against suitable shaped output, not only nonblank screenshots.
Text remains in logical order; avoid naïve grapheme reversal because it
breaks numbers, punctuation, diacritics and mixed scripts.

Future architecture: evaluate a pure-Rust Unicode BiDi + OpenType shaping
stack for this focused reference widget, maintain mapping from logical
grapheme ranges to visual glyph clusters and correct cursor rectangle, then
verify native Arabic/Hebrew rendering, selection, IME, accessibility, and
highlighting in an actual Wayland compositor. Never route untrusted physical
device events or collect private keyboard input as a testing shortcut.

The T804 shaping/caret and RTL acceptance checks remain pending until
that larger work is complete.
