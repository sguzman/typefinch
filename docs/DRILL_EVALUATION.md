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
generator). It rejects exposing the complete original reference inside
the practice passage. The sequence is **practice-disjoint baseline →
different practice passage → same-baseline immediate retest → same-baseline
delayed retest**. Since the learner types the reference at baseline,
this is NOT never-seen novel-text transfer; familiarity and retesting
effects remain possible in both arms. It validates
identical language, difficulty, duration, input mode, profile and reported
layout/seat provenance. A *process-local monotonic clock* requires the
"immediate" retest to **finish within ten minutes after practice**. Delayed
retention must finish **between one and 24 real hours after the immediate
retest** in that same running process. After either deadline the pilot
cannot proceed, and there is no timestamp reconstruction across restarts.
These windows are an engineering protocol, not proof of attendance or
causal learning.

`control_adjusted_transfer` compares two completed, distinct-profile,
opposite-arm series only when their practice-disjoint reference passage
matches canonically, configured context matches, and observed practice
opportunity counts match. Their actual measured practice time must differ
by no more than 10% of the longer observed duration. This conservative
engineering matching heuristic is not evidence of equal effort. It
produces descriptive control-adjusted differences in repeated-baseline
missed-reference-opportunity percentage points and corrected WPM.
Tests include an exactly null difference, an unfavorable adaptive difference,
premature retests, repeated submissions, missing retention, inconsistent
input or source context, identical participant identity, unequal exposure and
balanced synthetic random-byte assignments. Missing denominators cannot be
silently treated as zero.

**Randomization/eligibility safeguard:** the two in-memory candidate
interventions are constructed and validated before selecting a random arm.
The native synthetic GUI test still forces either arm only in an isolated
disposable QA session. No arm-specific failed enrollment is silently
counted as a successful randomized assignment. This establishes a
mechanical protection against differential arm attrition, not balanced
real-world recruitment or causal effectiveness.

**Still missing:** real human participant
recruitment, controlled time-on-task and effort equivalence, novel-reference
transfer, cross-restart
retention scheduling, counterbalanced crossover/order effects, preregistered
uncertainty-aware analysis, and empirical evidence of learning. This pilot
remains separate from the ordinary `FollowUp`; its de-identified aggregates
can be copied locally only after completion and an explicit user click, not
automatically persisted or sent to GitHub. **T502 remains unaccepted**.

## Implemented local opt-in pilot workflow (not efficacy evidence)

After finishing a Custom passage with observed missed characters, the
native Typefinch interface now offers a **default-off consent checkbox** and
a **Start randomized transfer pilot** action. **Before** arm assignment, the
app checks that BOTH the adaptive drill and the fixed non-adaptive comparison
are valid for the SAME baseline. If either overlaps the complete baseline,
is empty/invalid, or equals the other practice passage, neither arm is
assigned. This prevents arm-specific rejection after randomization.
The fixed comparator matches displayed grapheme and whitespace counts but
does **not** establish equal practice time, effort, difficulty, fluency or a
controlled population. Its fixed QWERTY alphabet is currently appropriate
only for Latin-script content, including accented Mexican Spanish; the pilot
therefore declines Arabic, Hebrew, CJK, emoji and mixed-script sources until
a genuinely comparable control is implemented. Ordinary Unicode typing and
drill features are unaffected.

After a valid practice score is saved, the app permits one **immediate
held-out retest** of the *original* source. After that score is saved, it
unlocks the **delayed held-out retest** only after one real hour and only
until the 24-hour cutoff on the process-local monotonic clock. The
immediate retest has a ten-minute completion deadline after practice.
Quitting the app discards the study.
Aborting, restarting, editing the source, losing focus, changing profile,
input mode, or keyboard provenance invalidates the active study. No global
input capture or device-level keystroke attribution is added.

A completed pilot can display aggregate missed-opportunity counts and
descriptive differences from its own baseline. **Only an explicit Copy
de-identified pilot aggregates click** copies a JSON document to the local
clipboard. This includes an arm label, fixed schema and app version, OS
versus emulated input category, configured duration, and baseline/practice/
immediate/delayed aggregate opportunities, ever-missed counts, attempts,
elapsed milliseconds and corrected WPM. It excludes raw text, per-character
errors, profile identifiers, configured layout strings, keyboard addresses,
timestamps and personal file paths. A discarded/incomplete study has no
record to export. Data are neither uploaded nor automatically persisted.

The version-4 export adds an optional, exact public reference cohort ID.
Clicking **Load public study baseline** in the native typing UI loads the
published `typefinch.en.standard.unlimited.v1` reference and fixes
English, Standard difficulty, Custom mode and unlimited test duration
locally. Participants may decline or type their own passages as before.
No pilot enrollment or transmission occurs when the public source is
loaded. The same ordinary baseline errors and explicit per-pilot opt-in
criteria still apply.

Only byte-exact matches to this **already public** reference under the
specified settings produce the `public_reference_cohort` identifier.
All other passages export `null`: Typefinch deliberately does **not** hash,
fingerprint, pseudonymize or otherwise leak private Custom passages through
a stable identifier, since even cryptographic hashes of short private texts
are susceptible to dictionary attacks. The reference itself remains absent
from the JSON and no keyboard address/profile ID is exported. A shared
cohort proves only common public source/settings, **not** comparable keyboard
layout, representative participant recruitment, equal training dose,
independent trials, randomization balance, or causal efficacy.

The version-4 export retains the full reference-opportunity count,
`coverage_policy="all_reference_positions_reached"`, predeclared
immediate/retention windows, and the actual monotonic delayed interval
(`actual_retention_gap_ms`). The local pilot rejects a timed stage if any
reference position was never reached,
although its normal local aggregate score still remains valid. This prevents
partial passage selection from falsely improving an observed error fraction,
but also excludes slow/noncompleting participants from completed-pilot
summaries. Such attrition can bias a study and **must be reported**, not hidden
or interpreted as improved training efficacy. Previous schema-1 exports do not assert full-reference coverage;
schema-2 lacks the bounded timing window; schema-3 cannot establish a
common public reference cohort. Do not silently pool their measurements
with schema-4 observations. Even schema-4 alone does not establish real
comparative efficacy, and private-passage results cannot be matched
without separately authorized contextual evidence.

The export explicitly marks `causal_claim=false` and
`control_adjusted=false`. It cannot be treated as a controlled result
without external enrollment, comparable participants and rigorous analysis.
Unit tests verify control-practice length/whitespace matching, randomized
assignment parity, protocol integrity, delayed measurement gate and
private-data omission. **Native GUI end-to-end pilot QA and empirical
comparative human data remain separate acceptance requirements.** No
change is made to the T502 progress check.

## Read-only public-cohort comparison (descriptive only)

The built-in offline CLI accepts **only explicitly selected local aggregate
exports**, never profile directories or a live settings/database connection:
`typefinch --compare-public-pilots arm-a.json arm-b.json [more.json ...]`.
It requires 2–200 schema-4 records with the exact published
`typefinch.en.standard.unlimited.v1` reference cohort, matching app
version and input-mode category, complete reached-position counts, valid
non-negative finite numeric fields, reported retention within the agreed
window and both randomized arms represented. It rejects unknown JSON
fields (including raw passages), mismatched cohorts, partial/older exports,
symbolic links, devices, directories, oversized inputs and repeated
file identities (including hard-linked aliases of the same inode). A file
is read only through an explicit pathname; no network access or automatic
discovery is performed. Error messages do not echo private paths or data.
Distinct files could nevertheless originate from the SAME participant or
repeated submission; source-file identity deduplication does not prove
human independence.

The JSON result contains only **arm counts, mean baseline error rates,
mean immediate/delayed changes, practice opportunities/timing and
difference-of-means summaries**. It explicitly sets `causal_claim=false`;
confidence intervals, significance tests and claims of better human
training are deliberately absent. If participants had different
layouts, repeated enrollments, differential noncompletion or biased
recruitment, these local outputs cannot independently detect or repair
that. Without permissioned, independently vetted human enrollment data,
neither the comparison CLI nor the public challenge constitutes T502
efficacy evidence. The already published passage is the only source
with a stable cohort ID; private passages cannot be matched by a hidden
hash or accidental export.

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
