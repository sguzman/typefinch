# 🌎 Typefinch language foundation

## Currently implemented
- `en`: original project English word, quote, punctuation, and paragraph passages
- `es-MX`: original project Mexican Spanish word, quote, punctuation, and paragraph passages, including accented vowels, ñ and inverted punctuation
- Digits are the same in both starter number corpora but stored under separate language context
- Custom passages are user-provided and not automatically translated
- The five native academy lessons are presently English-only regardless of selected practice language

All included starter corpus strings were authored for Typefinch. No text was copied from a third-party corpus and no external dataset or service is necessary.

## Result identity and backward compatibility
Legacy English session identifiers remain `words`, `quotes`, `punctuation`, `paragraphs`, or `numbers` in the existing SQLite `test_mode` column. Mexican Spanish benchmark identifiers end in `/es-MX`. Locale changes are unavailable during an active test, and the session owns its language snapshot.

Comparison graphs match layout, input mode, content type plus language, duration, and scoring version. Dashboard high-water marks are still profile-wide benchmarks, not per-language records; they are displayed as overall records. Future leaderboard and publishing features must not present these as language-specific.

## Known limitations
The engine compares canonically normalized Unicode extended grapheme clusters (scoring version 2), including composed/decomposed equivalents and whole-grapheme Backspace. A separate corrected CPM (matched display graphemes/minute) is exposed alongside standardized WPM for IME-oriented custom text; it is not a physical keystroke rate or a cross-language benchmark. Synthetic CJK committed-text and IME preedit tests exist, and CI verifies installed fallback fonts for representative CJK, Arabic and Hebrew glyphs. None of these establish live Fcitx/IBus composition, bidirectional shaping/caret correctness, or complete Unicode coverage. Spanish passages are intended for basic committed-text entry; retain native IME and RTL acceptance gates.

## Active IME candidate navigation guard

[CI 40120935121](https://github.com/sguzman/typefinch/actions/runs/40120935121)
passed a regression for focused OS-input sessions: Backspace while a native
IME preedit is active no longer deletes committed, scored graphemes;
Escape dismisses the local candidate preview instead of cancelling the
scored session. Once preedit has ended, ordinary Backspace remains available.
The guard applies only to the currently focused Typefinch typing surface;
it does not capture background keys, implement an IME engine or certify
native Fcitx/IBus candidate interaction. T201 and T803 remain incomplete.

## Future corpus governance
Store future corpora as independently versioned, attributed, redistributable data assets, with provenance/license metadata, difficulty annotations, and language-specific speed conventions. Do not claim a comprehensive language corpus from the small initial phrase set.

## Starter word difficulty

Each language has an Easy, Standard and Challenge original word list. Tiers are manually curated from common/short words toward longer vocabulary and accents; they have not been calibrated using user performance data. The selection is persisted locally. The default Standard level preserves historical English `words` and Mexican Spanish `words/es-MX` identifiers; other levels append `/easy` or `/challenge`. This extra context prevents misleading cross-difficulty WPM comparisons. Tiers currently apply only to Words, not quotes or lessons.

## Canonical-equivalent scoring (version 2)

Final WPM and accuracy now compare Unicode extended grapheme clusters after canonical normalization, so `é` and `e` followed by a combining acute accent receive the same score. Backspace deletes one entire grapheme. The final incomplete combining sequence may remain open briefly to accept the accent event. Raw input attempts still count individual committed Unicode scalars, a different denominator from grapheme-based final speed. The SQLite scoring-version marker increments from 1 to 2 and legacy records remain unchanged; trend charts do not merge scoring versions. Full CJK composition, bidirectional shaping and locale-specific speed standards still require dedicated QA.
