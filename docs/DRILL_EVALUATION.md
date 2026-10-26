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
identical original reference**.
For baselines in **Words, Quotes, Numbers, Punctuation or Paragraphs**, the
retest intentionally becomes a **Custom** session containing exactly that
original text, not another generated benchmark. This prevents repeated
exposure from contaminating ordinary benchmark histories or personal
best comparisons. The before/after analysis still records the original
baseline context in memory and requires the same duration, language,
difficulty, profile, input mode and keyboard layout. The baseline and original target remain
memory-only. Changing passage/settings/profile clears the trial; a changed
input mode, layout, duration, language, scoring target or recorded seat
provenance disqualifies the comparison. The drill itself is never treated
as a comparable post-test. The retest shows raw accuracy change in
percentage points, corrected WPM change, and any difference in attempt
counts. The underlying normal scores remain per-profile SQLite aggregate
records, but **no paired-trial identifier, passage, or character-level
error stream is saved or uploaded**. Completing a drill alone cannot
make the experiment show improvement.

**Native GUI acceptance (synthetic only):**
[CI 40153854777](https://github.com/sguzman/typefinch/actions/runs/40153854777)
passed **235 Rust tests and the full graphical suite**. Its actual Xvfb/egui
frontend received a deliberately mistyped three-character source
(`aba` attempted as `axa`), invoked **Ctrl+Shift+K** to complete an
eight-repeat missed-character drill, then invoked **Ctrl+Shift+R** to
retype the **identical original source** accurately. The standard
per-profile SQLite database contained exactly three aggregate scores:
2/3 initial correctness, 15/15 drill correctness, 3/3 retest correctness,
with unchanged input-mode provenance. A separate synthetic-only QA report
contained just numeric before/after raw accuracy (66.7% → 100%) and
explicitly set `causal_claim: false`. The app did not persist the custom
passage, source path, generated drill or trial metadata to settings.
The 1060×760 native screenshot `typefinch-training-followup.png` was
archived and visually inspected; it renders the 66.7% → 100% change
beside a **slower** corrected WPM, and explicitly warns against inferring
causation. **None of these deliberately authored inputs came from a
real learner.** They validate feature mechanics, not efficacy.

This is a **descriptive self-comparison**, useful feedback but not a test
of causal efficacy. Retesting the same source introduces familiarity,
practice, repetition, regression-to-the-mean and unequal-exposure biases.
A positive, null or negative paired difference cannot pass T502's
human training-efficacy gate. The original benchmark protocol below
requires held-out transfer, an intervention control, counterbalanced
order and delayed retention.

## Controlled-transfer protocol primitives (engineering only)

The new in-memory `src/training.rs::ControlledTrial` state machine provides
part of the previously proposed evaluation infrastructure. It distinguishes
**adaptive** versus **non-adaptive** arms, with an equal-probability assignment
from a fresh random byte (`Arm::random()` uses the existing OS-backed UUID
generator). It rejects the exact practice passage as a held-out source. The
test sequence is **held-out baseline → different practice passage → same
held-out immediate retest → same held-out delayed retest**. It validates
identical language, difficulty, duration, input mode, profile and reported
layout/seat provenance. The delayed stage now uses a *local monotonic
clock* to require at least one real hour after the immediate retest, while
the same app process remains running. This does not establish longer-term
retention after a restart or independent participant attendance.

`control_adjusted_transfer` compares two completed, distinct-profile,
opposite-arm series only when their configured context and observed practice
opportunity counts match. It produces descriptive control-adjusted changes
in held-out missed-reference-opportunity percentage points and corrected WPM.
Tests include an exactly null difference, an unfavorable adaptive difference,
premature retests, repeated submissions, missing retention, inconsistent
input or source context, identical participant identity, unequal exposure and
balanced synthetic random-byte assignments. Missing denominators cannot be
silently treated as zero.

**Not yet implemented:** an in-app enrollment/consent UI, matched
non-adaptive passage generation, trusted time-on-task matching, cross-restart
retention scheduling, crossover/order controls, preregistered
analysis with uncertainty, real participant recruitment or any evidence of
better learning. These types are not attached to existing `FollowUp` scores
and are not exported, persisted or sent to GitHub. Pure unit tests cannot
supply the needed comparative human data; **T502 remains unaccepted**.

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

## Implemented in-memory distinct reference opportunity metric

The scored `Session` now counts **unique reference-grapheme positions
actually reached** by at least one accepted input scalar and the subset of
those positions on which a noncanonical input prefix was ever recorded.
Correcting and retrying a miss counts the original position **once**, unlike
the raw `mistakes` counter; one decomposed Unicode grapheme containing
several valid combining scalars also counts as **one** opportunity.
The counters are reset on session start, held only as bounded sets of numeric
positions, and never saved to the SQLite history, exported profile cards or
uploaded. The GUI shows the two aggregate counts and a descriptive fraction
for a finished session.

The in-memory adaptive-drill baseline and identical-reference retest display
those distinct missed-position rates alongside raw accuracy and corrected WPM.
An opt-in synthetic QA report contains only four aggregate count fields; the
native GUI test requires 1/3 missed baseline opportunities and 0/3 retest
opportunities, with no private text or reference offsets recorded. A unit test
makes the same position repeatedly mistyped, corrected, and mistyped again
and requires **1** distinct miss despite **2** incorrect scalar attempts.

**Interpretation:** This is a distinct *ever-missed reference position per
reached position* fraction, not an independent-trial probability and not a
human efficacy outcome. Timed runs may reach different numbers of reference
positions, and correction opportunities can still differ. True randomized
held-out transfer and delayed-retention experiments remain unimplemented,
and T502 is not accepted from this engineering test.

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
