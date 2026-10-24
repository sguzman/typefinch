# ⏱️ Scoring units and fair comparisons

## Supported languages and the standard speed unit

Typefinch currently supports **English (`en`) and Mexican Spanish (`es-MX`)** typing corpora. For both, a displayed corrected WPM is a *standardized typing unit*, not a count of dictionary words:

`corrected WPM = correctly entered Unicode extended grapheme clusters ÷ 5 ÷ elapsed minutes`

An extended grapheme cluster is one user-perceived character. A composed `é` and the equivalent `e` followed by a combining acute accent each count as one correctly entered grapheme when canonically equivalent to the reference. Spaces and punctuation in the reference are also grapheme positions and contribute when correct. A partial or incorrect reference position does not contribute to corrected speed.

The five-grapheme normalization intentionally stays consistent across the two supported languages so the displayed units have the same mathematical definition. **It does not establish linguistic equivalence or comparative typing difficulty.** Benchmark records remain partitioned by test mode and language (for example, `words` versus `words/es-MX`), and word-difficulty variants remain separately labeled. Compare only like-for-like passages, modes, duration and language.

## CJK-friendly committed-text speed convention

Typefinch also displays **corrected CPM**, defined as the count of correctly
entered *final Unicode extended grapheme clusters* per elapsed minute:

`corrected CPM = correctly matched reference graphemes ÷ elapsed minutes`

It is useful for Chinese, Japanese and Korean passages entered through an IME:
candidate selection and variable romanization length do **not** turn one
displayed character into a fixed number of physical keypresses. The metric
also includes correctly matched spaces and punctuation, consistent with the
existing final-grapheme scorer. It is not a count of keyboard strokes, a
linguistic word rate, or an estimate of IME composition effort.

Mathematically, in the **same session**, standardized WPM is corrected CPM
divided by five. That arithmetic does **not** imply cross-language proficiency
equivalence. CPM and WPM should be compared only within similar passages,
scoring versions, input methods and language contexts. Typefinch does not yet
ship a certified CJK corpus or native Fcitx/IBus acceptance, and custom CJK
practice is not a public or cross-language benchmark. When a custom passage
contains Han, kana or Hangul, the typing HUD emphasizes corrected CPM and
retains standardized WPM as a secondary diagnostic. English/Spanish passages
keep their familiar prominent WPM rather than acquiring a redundant label.
This is a bounded *script display heuristic*, not language identification,
IME detection or cross-language score equivalence.

Pure Rust tests cover Chinese corrections, Japanese kana, Korean Hangul,
canonical accents, zero-duration handling and the exact five-unit
relationship. Native CJK IME and bidirectional shaping remain separately
unaccepted.

## Raw input versus corrected reference

`raw WPM = attempted Unicode scalar input count ÷ 5 ÷ elapsed minutes`

Raw attempts include mistakes and subsequent corrections. A decomposed accent may arrive as multiple Unicode scalar inputs but produce one final grapheme. Consequently raw WPM is a **diagnostic attempt rate**, not a directly comparable speed across input methods or scripts. Raw accuracy uses accepted attempt/mistake events; corrected accuracy uses final matching grapheme positions. These quantities deliberately have different denominators.

Pastes are excluded from scored sessions. An emulation key repeat is suppressed, while system committed-text events may depend on compositor behavior; universal per-physical-keyboard repeat attribution is **not accepted**. Window-focus loss cancels a running session without persisting a score. Expiry is checked before frame input; timed sessions reject post-deadline typing. An unfinished combining sequence cannot extend the timed cutoff. Untimed practice finishes on passage completion.

## Streaming Unicode composition boundary tests

Scoring now determines whether an arriving Unicode scalar extends the
previous grapheme using actual Unicode extended-grapheme segmentation,
rather than guessing from a short list of combining codepoints. It
handles Korean decomposed Hangul Jamo, a third regional-indicator symbol
after an already paired flag, and split emoji ZWJ sequences without
inventing extra character positions. Raw scalar attempts are still
diagnostic and separate from final corrected-grapheme accuracy.

Canonical composition also tolerates a still-incomplete sequence of
the **exact expected combining marks** arriving in a different
canonical-class order. Once the mark count is complete, genuine
non-equivalent equal-class ordering remains an error. The regression
suite verifies both the valid intermediate sequence and the negative
case. This does not demonstrate any live Fcitx/IBus integration or
hardware keystroke attribution. [CI 40118806315](https://github.com/sguzman/typefinch/actions/runs/40118806315)
passed the complete source and native X11 suite with these tests.

## Explicitly unaccepted language extensions

The corrected CPM policy specifies display-text accounting only. Arabic/Hebrew bidirectional caret/visual editing acceptance, CJK IME behavior in live compositors and cross-script proficiency calibration remain open under the multilingual roadmap. The current application does not claim that a compositor delivers per-physical-device input events.

**Verification:** `src/engine.rs` includes canonical-equivalence, corrected-speed and raw-attempt tests; native X11 deadline, paste rejection and focus cancellation are separately verified in CI.

## Native missed-key heatmap visibility

The current session's keyboard preview paints missed reference characters with severity-aware semantic fills and displays the number of misses as text following the finger hint. This is a **reference-symbol heatmap**, not an attribution of raw physical-key events. Hover descriptions remain available. Native X11 QA creates and corrects an actual mistake, saves a screenshot of the scrolled keyboard preview, and checks a substantial number of interior error-fill pixels; color and contrast unit tests remain independent. This does not establish live Hyprland or extra-key routing.

## Pause, focus, and key-repeat policy

**Pausing is not supported for a scored benchmark**: Escape or window-focus loss cancels an active session rather than freezing and resuming its clock. A canceled session is not persisted. Restart creates a fresh attempt with reset text, event counts and monotonic timing. The application tests expiry before accepting the next GUI frame's text, preventing late keystrokes from extending a timed session.

In **software emulation**, only a new physical-key press is eligible for scoring; egui key events marked `repeat` and Ctrl, Alt or command-modified shortcut presses are ignored. Shift remains a legitimate modifier for printable characters. In **system input mode**, the application uses compositor-provided committed text and does not claim a reliable way to identify which physical keyboard produced it or whether an OS-level text repeat was intentional. Repeated committed text is subject to ordinary passage scoring, never reclassified as independent hardware events. Clipboard paste is excluded from both scoring paths.

The scoped checks include pure engine cancellation/restart tests, emulator key-repeat/modifier tests, real X11 paste suppression and competing-window focus cancellation, and timed-deadline GUI QA. They **do not prove** a universal OS keyboard autorepeat detector, hardware interception, or live Hyprland behavior.

## Provisional IME events versus committed text

The system-input scoring surface consumes ordinary compositor `Text` events and final `Ime(Commit(...))` results. IME candidate/preedit changes and enabling/disabling an IME are not scored; clipboard paste remains excluded. The focused typing surface requests an IME window/candidate rect when a scored OS-input session is running. Unit tests use synthetic Chinese characters to verify committed grapheme counts and that noncommit events do not inflate attempts. Candidate-window positioning is provisional, and complete native CJK IME integration, cross-script WPM benchmarking, and RTL shaping/caret tests remain separate acceptance gates. No physical-device identity is inferred from committed text.

## Same-frame duplicate IME/text events

In OS input mode, some platform adapters may represent one finalized string
as both `Ime(Commit(text))` and `Text(text)` in the same egui frame.
Typefinch now treats an immediately corresponding pair of **opposite event
kinds with identical text** as one committed input. Two ordinary Text events,
two separate IME commits, or a later same-string event remain independent.
The guard is per-frame only, never depends on a timer or a physical device,
and stores at most 16 KiB of candidate text temporarily in memory.
It cannot infer duplicates emitted across frames or distinguish all
compositor IME implementations. Tests cover both pair orderings in Chinese
and Hebrew and valid repeated inputs. The complete
[CI 40058295306](https://github.com/sguzman/typefinch/actions/runs/40058295306)
passed those unit tests plus native virtual-X11 GUI regression; **real
Fcitx/IBus integration remains pending**.

## Multiline private custom reference input

Custom passages preserve meaningful hard paragraph boundaries after CRLF/CR
normalization, collapsing horizontal whitespace and dropping blank-only
lines. The scorer treats LF as one extended grapheme. A focused Enter
keypress commits it **only when the next expected custom reference grapheme
is LF**; it never injects an unrelated control key into an ordinary test.
A provisional IME candidate must not be confirmed as a scored newline.
Duplicate Enter/Text newline representations inside one GUI frame are
suppressed. This does not change ordinary benchmark and lesson whitespace
normalization, and source passages remain in memory only.

Native X11 import-and-scoring evidence in
[CI 40055443243](https://github.com/sguzman/typefinch/actions/runs/40055443243)
demonstrates `a`, Enter, `b` as three correctly scored graphemes in a
single Custom record, without source text/path persistence. Separate
[CI 40055821789](https://github.com/sguzman/typefinch/actions/runs/40055821789)
tests score four Arabic and four Hebrew graphemes from virtual physical
key-position events under installed-XKB *software emulation*. These tests do
not establish native CJK/RTL IME acceptance, real-device identity, or live
Wayland/Hyprland keyboard input.

## Ephemeral IME preedit and grapheme progress

While a system-input session owns focus, Typefinch visibly displays the active IME candidate as **Composing (not scored)**. Repeated preedit updates replace that ephemeral preview; only final committed text changes the typing result. The preview is bounded to 64 grapheme clusters and 4 KiB of UTF-8 data (including unusually long combining sequences) and is cleared on IME disable/commit, restart, Escape or window-focus loss. It is never saved to the settings, score history or profile export. Untimed progress uses completed Unicode extended grapheme clusters instead of scalar character count, avoiding inflated progress from decomposed accents. Dedicated tests cover large preedit data, changing Japanese/Chinese candidate behavior and composed/decomposed completion, but do not replace native IME or RTL acceptance.

## Optional installed Unicode glyph fallbacks

Typefinch keeps egui's preferred proportional and monospace font families as primary. When available, it appends system-installed Noto Sans CJK, Arabic and Hebrew font data as **fallbacks only**. No font files are bundled, downloaded, shared or persisted, and each file is read only from the trusted system font directory with an explicit size cap. The settings panel reports which fallbacks were found. Native CI installs the freely distributable OS Noto packages and checks actual egui glyph coverage for representative CJK, Arabic and Hebrew characters in both font families. Glyph availability alone does **not** establish right-to-left shaping, bidirectional caret movement, IME candidate selection or full script coverage.
