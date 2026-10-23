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
The engine compares Unicode scalar positions rather than grapheme clusters, and cannot yet certify canonically equivalent composed/decomposed strings, complex IME commit behavior, shaping, or RTL rendering. Spanish passages are intended for basic committed-text entry; no claim of complete Unicode correctness. Add grapheme segmentation, normalization and IME integration tests before marking multilingual support complete.

## Future corpus governance
Store future corpora as independently versioned, attributed, redistributable data assets, with provenance/license metadata, difficulty annotations, and language-specific speed conventions. Do not claim a comprehensive language corpus from the small initial phrase set.
