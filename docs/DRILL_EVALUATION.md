# 🎯 Adaptive drill evaluation protocol

Typefinch offers two temporary practice generators in `src/engine.rs`: missed
reference grapheme drills and missed reference bigram drills. Both allocate
repetitions by observed in-session miss counts; the eight-pattern cap,
two-to-eight repetition rule, Unicode cluster boundaries and 256-byte source
fragment cap are **engineering behavior**, not evidence of learning.

## Current evidence and limits

- Source-local unit tests verify deterministic ranking, exact miss-position
  targeting, Unicode grapheme integrity, caps, whitespace boundaries, reset
  behavior and no storage of drill text in scores or settings.
- Native X11 tests demonstrate scoring and session lifecycle. They do **not**
  demonstrate that a person retains new typing skill, or that one drill
  outperforms ordinary practice.
- The `T502: Training efficacy tests` check in `docs/progress.json` remains
  **unaccepted** until actual comparative user-performance evidence exists.

## Implemented: same-passage paired retest (not an efficacy result)

The in-memory follow-up workflow in `src/training.rs` and `src/app.rs`
now gives a completed non-lesson typing session the option to practice its
observed difficult characters or two-grapheme pairs, then **retest the
identical original reference**. The baseline and original target remain
memory-only. Changing passage/settings/profile clears the trial; a changed
input mode, layout, duration, language, scoring target or recorded seat
provenance disqualifies the comparison. The drill itself is never treated
as a comparable post-test. The retest shows raw accuracy change in
percentage points, corrected WPM change, and any difference in attempt
counts. The underlying normal scores remain per-profile SQLite aggregate
records, but **no paired-trial identifier, passage, or character-level
error stream is saved or uploaded**. Completing a drill alone cannot
make the experiment show improvement.

This is a **descriptive self-comparison**, useful feedback but not a test
of causal efficacy. Retesting the same source introduces familiarity,
practice, repetition, regression-to-the-mean and unequal-exposure biases.
A positive, null or negative paired difference cannot pass T502's
human training-efficacy gate. The original benchmark protocol below
requires held-out transfer, an intervention control, counterbalanced
order and delayed retention.

## Proposed empirical acceptance design (not implemented)

A privacy-first evaluation should compare **adaptive targeting** against
**non-adaptive practice**, not merely compare performance on memorized drill
text. This is a proposed methodology, not a claim that any users have been
recruited or that outcomes have been measured.

1. Enroll only through a clearly labeled local opt-in. Never enable telemetry,
   read another application, capture global input, or upload typed passages.
2. Establish a baseline with identical language, input mode, reference
   difficulty, physical-layout policy and scoring version. Separately record
   a held-out passage not included in practice.
3. Randomly counterbalance adaptive versus fixed-duration, non-adaptive
   training. Balance practice time and number of opportunities, not just raw
   word counts. A within-person crossover needs order control and a washout
   interval; a parallel-group trial needs baseline stratification.
4. Measure **held-out transfer** after training and delayed retention, rather
   than treating fewer errors in the training drill as proficiency improvement.
   Record immediate and delayed measurements separately; a session that fails
   matching-input rules is not comparable.
5. Predefine primary outcome as the change in held-out missed-reference
   opportunities per eligible reference opportunity, adjusted by matched
   control. Track corrected grapheme WPM, complete final accuracy, correction
   count, raw event rate, time-on-task and confidence intervals as distinct
   secondary measures.
6. Retain only aggregate per-session counts and versioned experimental
   metadata when the participant opts in. No raw typing, key-by-key timelines,
   imported passages, source file paths or unconsented identifiers. Provide
   local export and deletion.
7. Test randomization, exclusion criteria, opportunities denominators, metric
   aggregation, duplicate-score prevention and opt-out before conducting
   evaluation. Report uncertainty and null/negative outcomes. Do not infer
   efficacy from synthetic-learner tests, screenshots or faster completion of
   the practiced text.

## Interpretation safeguards

`Session::mistakes` counts accepted incorrect scalar attempts, including
attempts later corrected. This can be greater than the count of unique
reference opportunities. Do **not** divide that counter directly by the number
of reference graphemes and call the result an error probability. An
opportunity-level numerator must be measured explicitly in any future study.

The current per-key heatmap associates mistakes with a reference symbol;
it does not identify a physical source keyboard. CJK IME, RTL editing and
cross-language speed policies have separate open acceptance gates. Results from
different input methods or scoring versions must not be pooled without a
published justification.

**Acceptance status:** methodological specification only. It advances
implementation readiness, but it does not pass T502's human training-efficacy
criterion and does not change the tracked progress percentage.
