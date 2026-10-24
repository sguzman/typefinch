# ⏱️ Scoring units and fair comparisons

## Supported languages and the standard speed unit

Typefinch currently supports **English (`en`) and Mexican Spanish (`es-MX`)** typing corpora. For both, a displayed corrected WPM is a *standardized typing unit*, not a count of dictionary words:

`corrected WPM = correctly entered Unicode extended grapheme clusters ÷ 5 ÷ elapsed minutes`

An extended grapheme cluster is one user-perceived character. A composed `é` and the equivalent `e` followed by a combining acute accent each count as one correctly entered grapheme when canonically equivalent to the reference. Spaces and punctuation in the reference are also grapheme positions and contribute when correct. A partial or incorrect reference position does not contribute to corrected speed.

The five-grapheme normalization intentionally stays consistent across the two supported languages so the displayed units have the same mathematical definition. **It does not establish linguistic equivalence or comparative typing difficulty.** Benchmark records remain partitioned by test mode and language (for example, `words` versus `words/es-MX`), and word-difficulty variants remain separately labeled. Compare only like-for-like passages, modes, duration and language.

## Raw input versus corrected reference

`raw WPM = attempted Unicode scalar input count ÷ 5 ÷ elapsed minutes`

Raw attempts include mistakes and subsequent corrections. A decomposed accent may arrive as multiple Unicode scalar inputs but produce one final grapheme. Consequently raw WPM is a **diagnostic attempt rate**, not a directly comparable speed across input methods or scripts. Raw accuracy uses accepted attempt/mistake events; corrected accuracy uses final matching grapheme positions. These quantities deliberately have different denominators.

Pastes are excluded from scored sessions. An emulation key repeat is suppressed, while system committed-text events may depend on compositor behavior; universal per-physical-keyboard repeat attribution is **not accepted**. Window-focus loss cancels a running session without persisting a score. Expiry is checked before frame input; timed sessions reject post-deadline typing. An unfinished combining sequence cannot extend the timed cutoff. Untimed practice finishes on passage completion.

## Explicitly unaccepted language extensions

No CJK IME scoring policy or Arabic/Hebrew bidirectional caret/visual editing acceptance is implied by the shared five-grapheme convention. Those language-specific semantics remain open under the multilingual roadmap. The current application does not claim that a compositor delivers per-physical-device input events.

**Verification:** `src/engine.rs` includes canonical-equivalence, corrected-speed and raw-attempt tests; native X11 deadline, paste rejection and focus cancellation are separately verified in CI.

## Native missed-key heatmap visibility

The current session's keyboard preview paints missed reference characters with severity-aware semantic fills and displays the number of misses as text following the finger hint. This is a **reference-symbol heatmap**, not an attribution of raw physical-key events. Hover descriptions remain available. Native X11 QA creates and corrects an actual mistake, saves a screenshot of the scrolled keyboard preview, and checks a substantial number of interior error-fill pixels; color and contrast unit tests remain independent. This does not establish live Hyprland or extra-key routing.
