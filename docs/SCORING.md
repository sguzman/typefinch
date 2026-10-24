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

## Pause, focus, and key-repeat policy

**Pausing is not supported for a scored benchmark**: Escape or window-focus loss cancels an active session rather than freezing and resuming its clock. A canceled session is not persisted. Restart creates a fresh attempt with reset text, event counts and monotonic timing. The application tests expiry before accepting the next GUI frame's text, preventing late keystrokes from extending a timed session.

In **software emulation**, only a new physical-key press is eligible for scoring; egui key events marked `repeat` and Ctrl, Alt or command-modified shortcut presses are ignored. Shift remains a legitimate modifier for printable characters. In **system input mode**, the application uses compositor-provided committed text and does not claim a reliable way to identify which physical keyboard produced it or whether an OS-level text repeat was intentional. Repeated committed text is subject to ordinary passage scoring, never reclassified as independent hardware events. Clipboard paste is excluded from both scoring paths.

The scoped checks include pure engine cancellation/restart tests, emulator key-repeat/modifier tests, real X11 paste suppression and competing-window focus cancellation, and timed-deadline GUI QA. They **do not prove** a universal OS keyboard autorepeat detector, hardware interception, or live Hyprland behavior.

## Provisional IME events versus committed text

The system-input scoring surface consumes ordinary compositor `Text` events and final `Ime(Commit(...))` results. IME candidate/preedit changes and enabling/disabling an IME are not scored; clipboard paste remains excluded. The focused typing surface requests an IME window/candidate rect when a scored OS-input session is running. Unit tests use synthetic Chinese characters to verify committed grapheme counts and that noncommit events do not inflate attempts. Candidate-window positioning is provisional, and complete native CJK IME integration, cross-script WPM benchmarking, and RTL shaping/caret tests remain separate acceptance gates. No physical-device identity is inferred from committed text.

## Ephemeral IME preedit and grapheme progress

While a system-input session owns focus, Typefinch visibly displays the active IME candidate as **Composing (not scored)**. Repeated preedit updates replace that ephemeral preview; only final committed text changes the typing result. The preview is bounded to 64 grapheme clusters and is cleared on IME disable/commit, restart, Escape or window-focus loss. It is never saved to the settings, score history or profile export. Untimed progress uses completed Unicode extended grapheme clusters instead of scalar character count, avoiding inflated progress from decomposed accents. Dedicated tests cover large preedit data, changing Japanese/Chinese candidate behavior and composed/decomposed completion, but do not replace native IME or RTL acceptance.
