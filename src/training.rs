//! In-memory targeted-practice follow-up. It is an observational paired
//! measurement, NOT a causal study of typing-training efficacy.
//! Never persist a source passage, character-level errors or this trial.
use crate::{engine::{Language,OpportunityCounts,Score,Session,State,TestMode,WordDifficulty},layouts::Mode};
use std::time::Duration;
use unicode_normalization::UnicodeNormalization;
use unicode_segmentation::UnicodeSegmentation;

#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub enum Stage { Drill, Retest, Compared }

#[derive(Clone,Copy,Debug,PartialEq)]
pub struct Comparison {
    pub baseline_accuracy:f64,
    pub retest_accuracy:f64,
    pub baseline_wpm:f64,
    pub retest_wpm:f64,
    pub baseline_attempts:usize,
    pub retest_attempts:usize,
    pub baseline_opportunities:OpportunityCounts,
    pub retest_opportunities:OpportunityCounts,
}
impl Comparison {
    pub fn accuracy_change_pp(self)->f64 {self.retest_accuracy-self.baseline_accuracy}
    pub fn speed_change_wpm(self)->f64 {self.retest_wpm-self.baseline_wpm}
    pub fn same_attempt_count(self)->bool {self.baseline_attempts==self.retest_attempts}
    /// Descriptive, distinct missed-position rate delta; only computed when
    /// each source has at least one actually reached reference position.
    /// Never interpret a paired same-passage difference as causal efficacy.
    pub fn missed_opportunity_change_pp(self)->Option<f64> {
        Some(self.retest_opportunities.missed_percent()?-
            self.baseline_opportunities.missed_percent()?)
    }
}
pub struct FollowUp {
    pub reference:String,
    /// Ephemeral generated practice target; never persisted in settings/history.
    pub drill_reference:String,
    pub mode:TestMode,
    pub language:Language,
    pub difficulty:WordDifficulty,
    pub duration:Duration,
    pub profile_id:String,
    pub chosen_layout:String,
    pub recorded_layout:String,
    pub input:Mode,
    pub baseline:Score,
    pub baseline_opportunities:OpportunityCounts,
    pub stage:Stage,
    pub comparison:Option<Comparison>,
}
impl FollowUp {
    pub fn begin(session:&Session,drill:&str,profile_id:&str,chosen_layout:&str,
        recorded_layout:&str,input:Mode)->Option<Self>{
        if session.state!=State::Finished || session.mode==TestMode::Lesson
            || session.target.is_empty() || drill.is_empty() {return None;}
        let baseline=session.score();
        if baseline.entered==0 || baseline.attempts==0 {return None;}
        Some(Self {
            reference:session.target.clone(),drill_reference:drill.into(),mode:session.mode,
            language:session.language,difficulty:session.difficulty,
            duration:session.duration,profile_id:profile_id.into(),
            chosen_layout:chosen_layout.into(),recorded_layout:recorded_layout.into(),
            input,baseline,
            baseline_opportunities:session.opportunity_counts(),
            stage:Stage::Drill,comparison:None,
        })
    }
    pub fn same_context(&self,profile_id:&str,chosen_layout:&str,input:Mode)->bool{
        self.profile_id==profile_id && self.chosen_layout==chosen_layout && self.input==input
    }
    /// Check the actual identical scored reference and provenance. Neither a
    /// generated drill nor a different layout can masquerade as the retest.
    pub fn compare(&self,session:&Session,recorded_layout:&str,
        profile_id:&str,chosen_layout:&str,input:Mode)->Option<Comparison>{
        if self.stage!=Stage::Retest || session.state!=State::Finished
            || !self.same_context(profile_id,chosen_layout,input)
            || self.recorded_layout!=recorded_layout
            || self.reference!=session.target
            || session.mode!=TestMode::Custom || self.duration!=session.duration
            || self.language!=session.language || self.difficulty!=session.difficulty {
            return None;
        }
        let after=session.score();
        if after.entered==0 || after.attempts==0 {return None;}
        Some(Comparison {
            baseline_accuracy:self.baseline.raw_accuracy,
            retest_accuracy:after.raw_accuracy,
            baseline_wpm:self.baseline.wpm,
            retest_wpm:after.wpm,
            baseline_attempts:self.baseline.attempts,
            retest_attempts:after.attempts,
            baseline_opportunities:self.baseline_opportunities,
            retest_opportunities:session.opportunity_counts(),
        })
    }
}


/// A future opt-in study needs a genuine control condition and a reference
/// withheld from practice. These are *protocol mechanics*, not human efficacy
/// evidence; no participant or trial data is persisted by Typefinch.
#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub enum Arm { Adaptive, NonAdaptive }
impl Arm {
    /// Unbiased assignment from one uniformly random byte. Production callers
    /// should draw fresh entropy for each enrolled trial, not derive a group
    /// from profile names, errors or previous performance.
    pub fn from_random_byte(byte:u8)->Self {
        if byte&1==0 {Self::Adaptive}else{Self::NonAdaptive}
    }
    pub fn random()->Self { Self::from_random_byte(uuid::Uuid::new_v4().as_bytes()[0]) }
}
#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub enum TrialStage { Practice, Immediate, Delayed, Complete }

/// Generate a *length-matched* local non-adaptive comparator with identical
/// whitespace boundaries. It does not adapt to observed mistakes, and this
/// matching alone cannot equalize effort, time, language or cognitive load.
pub fn neutral_practice(adaptive:&str,held_out:&str)->Option<String> {
    if adaptive.is_empty() || held_out.is_empty() {return None;}
    let original=held_out.nfc().collect::<String>();
    for alphabet in ["asdfjkl;","qwertyuiop","1234567890"] {
        let mut index=0;
        let candidate=adaptive.graphemes(true).map(|g|{
            if g.chars().all(char::is_whitespace) {g.to_owned()}
            else {
                let value=alphabet.chars().nth(index%alphabet.len()).unwrap();
                index+=1;
                value.to_string()
            }
        }).collect::<String>();
        if !candidate.nfc().collect::<String>().contains(&original) {
            return Some(candidate);
        }
    }
    None
}

#[derive(Clone,Copy,Debug,PartialEq)]
pub struct TransferSample {
    pub opportunities:OpportunityCounts,
    pub corrected_wpm:f64,
    pub attempts:usize,
    pub elapsed_ms:u64,
}
impl TransferSample {
    fn from_session(session:&Session)->Option<Self> {
        if session.state!=State::Finished {return None;}
        let score=session.score();
        let opportunities=session.opportunity_counts();
        if score.entered==0 || score.attempts==0 ||
            opportunities.observed==0 || !score.wpm.is_finite() {return None;}
        Some(Self {opportunities,corrected_wpm:score.wpm,attempts:score.attempts,
            elapsed_ms:u64::try_from(score.elapsed.as_millis()).ok()?})
    }
}
/// This self-contained local series still cannot establish causation by itself:
/// held-out passage difficulty, recruitment, order, and practice dose require
/// independently verified comparative human evidence.
pub struct ControlledTrial {
    pub arm:Arm,
    pub stage:TrialStage,
    held_out:String,
    practice:String,
    language:Language,
    difficulty:WordDifficulty,
    duration:Duration,
    profile_id:String,
    layout:String,
    recorded_layout:String,
    input:Mode,
    pub baseline:TransferSample,
    pub practiced:Option<TransferSample>,
    pub immediate:Option<TransferSample>,
    pub delayed:Option<TransferSample>,
    // Local monotonic time, never persisted or represented as a timestamp.
    // An app restart invalidates the incomplete trial.
    immediate_at:Option<std::time::Instant>,
}
impl ControlledTrial {
    /// Baseline is an actual finished *held-out* Custom session. Its source
    /// must differ from the practice text; merely retesting trained text is
    /// the existing descriptive FollowUp, never a controlled transfer test.
    #[allow(clippy::too_many_arguments)]
    pub fn begin(baseline:&Session,practice:&str,arm:Arm,profile_id:&str,
        layout:&str,recorded_layout:&str,input:Mode)->Option<Self> {
        if baseline.mode!=TestMode::Custom || baseline.target.is_empty()
            || practice.is_empty()
            || profile_id.is_empty() || layout.is_empty() || recorded_layout.is_empty() {
            return None;
        }
        // Use the real Custom-mode normalization before deciding whether the
        // supposedly withheld passage was exposed during practice. Reject
        // canonically equivalent Unicode and a practice text containing the
        // entire held-out passage (including a normalization-equivalent copy).
        let normalized=Session::new_with_settings(baseline.duration.as_secs(),
            TestMode::Custom,practice,baseline.language,baseline.difficulty);
        let withheld=baseline.target.nfc().collect::<String>();
        let exposed=normalized.target.nfc().collect::<String>();
        if exposed.is_empty() || exposed.contains(&withheld) {return None;}
        Some(Self {
            arm,stage:TrialStage::Practice,
            held_out:baseline.target.clone(),practice:normalized.target,
            language:baseline.language,difficulty:baseline.difficulty,
            duration:baseline.duration,profile_id:profile_id.into(),
            layout:layout.into(),recorded_layout:recorded_layout.into(),input,
            baseline:TransferSample::from_session(baseline)?,
            practiced:None,immediate:None,delayed:None,immediate_at:None,
        })
    }
    /// Consent-driven clipboard export only. No participant identity,
    /// exact layout name, passages, keyboard IDs, timestamps, or raw errors.
    /// An incomplete or abandoned pilot has no summary to export.
    pub fn aggregate_export(&self)->Option<String> {
        if self.stage!=TrialStage::Complete {return None;}
        let practiced=self.practiced?;
        let immediate=self.immediate?;
        let delayed=self.delayed?;
        let summarize=|sample:TransferSample| serde_json::json!({
            "opportunities":sample.opportunities.observed,
            "ever_missed":sample.opportunities.missed,
            "corrected_wpm":sample.corrected_wpm,
            "attempts":sample.attempts,
            "elapsed_ms":sample.elapsed_ms
        });
        let record=serde_json::json!({
            "schema":1,
            "type":"typefinch-local-held-out-pilot",
            "app_version":env!("CARGO_PKG_VERSION"),
            "randomized_arm":if self.arm==Arm::Adaptive {"adaptive"}else{"non_adaptive"},
            "input_mode":if self.input==Mode::System {"os"}else{"emulated"},
            "configured_duration_seconds":self.duration.as_secs(),
            "baseline":summarize(self.baseline),
            "practice":summarize(practiced),
            "immediate":summarize(immediate),
            "delayed":summarize(delayed),
            "control_adjusted":false,
            "causal_claim":false,
            "consent":"explicit local clipboard action; no network publication"
        });
        serde_json::to_string_pretty(&record).ok()
    }
    pub fn practice_source(&self)->&str {&self.practice}
    pub fn held_out_source(&self)->&str {&self.held_out}
    pub fn same_context(&self,profile_id:&str,layout:&str,
        recorded_layout:&str,input:Mode)->bool {
        self.profile_id==profile_id && self.layout==layout
            && self.recorded_layout==recorded_layout && self.input==input
    }
    pub fn delay_ready(&self)->bool {
        self.stage==TrialStage::Delayed && self.immediate_at.is_some_and(|time|
            time.elapsed()>=Duration::from_secs(3600))
    }
    fn valid(&self,session:&Session,source:&str,profile_id:&str,layout:&str,
        recorded_layout:&str,input:Mode)->bool {
        session.state==State::Finished && session.mode==TestMode::Custom
            && session.target==source
            && self.profile_id==profile_id && self.layout==layout
            && self.recorded_layout==recorded_layout && self.input==input
            && self.language==session.language && self.difficulty==session.difficulty
            && self.duration==session.duration
    }
    pub fn record_practice(&mut self,session:&Session,profile_id:&str,
        layout:&str,recorded_layout:&str,input:Mode)->bool {
        if self.stage!=TrialStage::Practice ||
            !self.valid(session,&self.practice,profile_id,layout,recorded_layout,input) {
            return false;
        }
        let Some(sample)=TransferSample::from_session(session) else {return false;};
        self.practiced=Some(sample);
        self.stage=TrialStage::Immediate;
        true
    }
    pub fn record_immediate(&mut self,session:&Session,profile_id:&str,
        layout:&str,recorded_layout:&str,input:Mode)->bool {
        if self.stage!=TrialStage::Immediate || self.practiced.is_none() ||
            !self.valid(session,&self.held_out,profile_id,layout,recorded_layout,input) {
            return false;
        }
        let Some(sample)=TransferSample::from_session(session) else {return false;};
        self.immediate=Some(sample);
        self.immediate_at=Some(std::time::Instant::now());
        self.stage=TrialStage::Delayed;
        true
    }
    /// Use a local monotonic clock rather than trusting an elapsed duration
    /// supplied by a caller. At least one real hour must separate the
    /// immediate measurement and this retest in the *same running app*.
    pub fn record_delayed(&mut self,session:&Session,
        profile_id:&str,layout:&str,recorded_layout:&str,input:Mode)->bool {
        if self.stage!=TrialStage::Delayed || self.immediate.is_none() ||
            !self.immediate_at.is_some_and(|time|
                time.elapsed()>=Duration::from_secs(3600)) ||
            !self.valid(session,&self.held_out,profile_id,layout,recorded_layout,input) {
            return false;
        }
        let Some(sample)=TransferSample::from_session(session) else {return false;};
        self.delayed=Some(sample);
        self.stage=TrialStage::Complete;
        true
    }
}
#[derive(Clone,Copy,Debug,PartialEq)]
pub struct ControlAdjustedTransfer {
    /// Difference in immediate missed-opportunity change, in percentage
    /// points; negative means a larger descriptive reduction for adaptive.
    pub immediate_missed_pp:f64,
    pub delayed_missed_pp:f64,
    pub immediate_wpm:f64,
    pub delayed_wpm:f64,
}
/// Strictly match actual practice opportunities, language, input mode,
/// layout and duration before comparing two separately completed series.
/// No confidence interval or efficacy interpretation is available from one
/// synthetic pair, or from arbitrarily selected human volunteers.
pub fn control_adjusted_transfer(adaptive:&ControlledTrial,
    neutral:&ControlledTrial)->Option<ControlAdjustedTransfer> {
    if adaptive.arm!=Arm::Adaptive || neutral.arm!=Arm::NonAdaptive
        || adaptive.stage!=TrialStage::Complete || neutral.stage!=TrialStage::Complete
        || adaptive.profile_id==neutral.profile_id
        // A matched comparison must evaluate the SAME withheld source.
        // Distinct Custom passages can have radically different script,
        // complexity or learned familiarity despite matching language tags.
        || adaptive.held_out.nfc().collect::<String>()!=neutral.held_out.nfc().collect::<String>()
        || adaptive.language!=neutral.language || adaptive.difficulty!=neutral.difficulty
        || adaptive.duration!=neutral.duration || adaptive.input!=neutral.input
        || adaptive.layout!=neutral.layout || adaptive.recorded_layout!=neutral.recorded_layout
        || adaptive.practiced?.opportunities.observed !=
            neutral.practiced?.opportunities.observed {return None;}
    // A nominally equal-length exercise is not an equal dose. For an
    // exploratory matched pair, reject >10% differences in actual measured
    // practice time. This is a conservative predeclared engineering guard,
    // not evidence of equivalence in effort or learning.
    let practice_a=adaptive.practiced?.elapsed_ms;
    let practice_b=neutral.practiced?.elapsed_ms;
    if practice_a.abs_diff(practice_b)>practice_a.max(practice_b)/10 {
        return None;
    }
    let delta=|before:TransferSample,after:TransferSample| {
        Some(after.opportunities.missed_percent()?-before.opportunities.missed_percent()?)
    };
    let ad_im=adaptive.immediate?;
    let co_im=neutral.immediate?;
    let ad_delay=adaptive.delayed?;
    let co_delay=neutral.delayed?;
    Some(ControlAdjustedTransfer {
        immediate_missed_pp:delta(adaptive.baseline,ad_im)?-delta(neutral.baseline,co_im)?,
        delayed_missed_pp:delta(adaptive.baseline,ad_delay)?-delta(neutral.baseline,co_delay)?,
        immediate_wpm:(ad_im.corrected_wpm-adaptive.baseline.corrected_wpm)
            -(co_im.corrected_wpm-neutral.baseline.corrected_wpm),
        delayed_wpm:(ad_delay.corrected_wpm-adaptive.baseline.corrected_wpm)
            -(co_delay.corrected_wpm-neutral.baseline.corrected_wpm),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn comparison_is_only_after_same_passage_and_trained_intervention(){
        let mut baseline=Session::new_with_custom(0,TestMode::Custom,"an an");
        baseline.begin();
        baseline.type_text("xn an");
        assert_eq!(baseline.state,State::Finished);
        let mut follow=FollowUp::begin(&baseline,"n n","profile","us","us",Mode::Emulated).unwrap();
        assert_eq!(follow.stage,Stage::Drill);
        let mut after=Session::new_with_custom(0,TestMode::Custom,"an an");
        after.begin();
        after.type_text("an an");
        assert!(follow.compare(&after,"us","profile","us",Mode::Emulated).is_none());
        follow.stage=Stage::Retest;
        let measured=follow.compare(&after,"us","profile","us",Mode::Emulated).unwrap();
        assert!(measured.accuracy_change_pp()>0.0);
        assert_eq!(measured.retest_accuracy,100.0);
        assert!(measured.same_attempt_count());
        assert!(follow.compare(&after,"de","profile","us",Mode::Emulated).is_none());
        assert!(follow.compare(&after,"us","other","us",Mode::Emulated).is_none());
        assert!(follow.compare(&after,"us","profile","de",Mode::Emulated).is_none());
        assert!(follow.compare(&after,"us","profile","us",Mode::System).is_none());
    }
    #[test] fn followup_never_accepts_drill_as_the_original_passage(){
        let mut before=Session::new_with_custom(0,TestMode::Custom,"abc");
        before.begin();
        before.type_text("axc");
        let mut follow=FollowUp::begin(&before,"b b","id","us","os:unknown",Mode::System).unwrap();
        follow.stage=Stage::Retest;
        let mut drill=Session::new_with_custom(0,TestMode::Custom,"x x x");
        drill.begin();
        drill.type_text("x x x");
        assert!(follow.compare(&drill,"os:unknown","id","us",Mode::System).is_none());
        assert!(follow.compare(&before,"os:unknown","id","us",Mode::System).is_some());
        assert!(FollowUp::begin(&Session::new_with_custom(0,TestMode::Custom,"x"),
            "x","id","us","os:unknown",Mode::System).is_none());
        assert!(FollowUp::begin(&Session::new_with_custom(0,TestMode::Lesson,"x"),
            "x","id","us","os:unknown",Mode::System).is_none());
    }
    #[test] fn unicode_reference_and_changed_duration_must_not_create_false_transfer(){
        let reference="سَلَام שָׁלוֹם e\u{301}";
        let mut baseline=Session::new_with_custom(0,TestMode::Custom,reference);
        baseline.begin();
        baseline.type_text(reference);
        assert_eq!(baseline.state,State::Finished);
        let mut follow=FollowUp::begin(&baseline,"س س","private","us","os:unknown",
            Mode::System).expect("observational study baseline");
        follow.stage=Stage::Retest;
        let mut identical=Session::new_with_custom(0,TestMode::Custom,reference);
        identical.begin();
        identical.type_text(reference);
        assert!(follow.compare(&identical,"os:unknown","private","us",
            Mode::System).is_some());
        let mut timed=Session::new_with_custom(60,TestMode::Custom,reference);
        timed.begin();
        timed.type_text(reference);
        assert!(follow.compare(&timed,"os:unknown","private","us",Mode::System).is_none(),
            "different timing conditions cannot be described as comparable");
        let mut altered=Session::new_with_custom(0,TestMode::Custom,"سَلَام שָׁלוֹם e");
        altered.begin();
        altered.type_text("سَلَام שָׁלוֹם e");
        assert!(follow.compare(&altered,"os:unknown","private","us",Mode::System).is_none(),
            "a changed private passage must never match the baseline");
    }
    #[test] fn benchmark_followup_uses_identical_custom_reference_not_benchmark_ranking(){
        let mut baseline=Session::new_with_settings(
            0,TestMode::Numbers,"",Language::MexicanSpanish,WordDifficulty::Standard);
        let original=baseline.target.clone();
        baseline.begin();
        baseline.type_text(&original);
        assert_eq!(baseline.state,State::Finished);
        assert!(baseline.mode.counts_for_benchmark());
        let mut trial=FollowUp::begin(&baseline,"7 7","profile","us",
            "os:unknown",Mode::System).unwrap();
        assert_eq!(trial.mode,TestMode::Numbers);
        trial.stage=Stage::Retest;
        // The exact original source is a CUSTOM session, not another sample
        // that can silently inflate the Numbers benchmark history.
        let mut replay=Session::new_with_settings(
            0,TestMode::Custom,&trial.reference,
            Language::MexicanSpanish,WordDifficulty::Standard);
        assert_eq!(replay.target,trial.reference);
        replay.begin();
        let retest=replay.target.clone();
        replay.type_text(&retest);
        assert_eq!(replay.state,State::Finished);
        assert_eq!(replay.history_mode(),"custom");
        assert!(!replay.mode.counts_for_benchmark());
        assert!(trial.compare(&replay,"os:unknown","profile","us",
            Mode::System).is_some());
        let mut wrong_mode=Session::new_with_settings(
            0,TestMode::Numbers,"",Language::MexicanSpanish,
            WordDifficulty::Standard);
        wrong_mode.begin();
        let repeated=wrong_mode.target.clone();
        wrong_mode.type_text(&repeated);
        assert_eq!(wrong_mode.target,trial.reference);
        assert!(trial.compare(&wrong_mode,"os:unknown","profile","us",
            Mode::System).is_none(),
            "replaying an identical benchmark must not count as fresh benchmark evidence");
    }
    #[test] fn every_word_quote_number_and_paragraph_source_roundtrips_as_custom(){
        for language in Language::ALL {
            for difficulty in [
                WordDifficulty::Easy,WordDifficulty::Standard,
                WordDifficulty::Challenge,
            ] {
                for mode in [
                    TestMode::Words,TestMode::Quotes,TestMode::Numbers,
                    TestMode::Punctuation,TestMode::Paragraphs,
                ] {
                    let original=Session::new_with_settings(
                        0,mode,"",language,difficulty);
                    assert!(!original.target.is_empty());
                    let retest=Session::new_with_settings(
                        0,TestMode::Custom,&original.target,language,difficulty);
                    assert_eq!(retest.target,original.target,
                        "Original benchmark text must replay exactly as a Custom session");
                    assert!(!retest.mode.counts_for_benchmark());
                    assert_eq!(retest.history_mode(),"custom");
                }
            }
        }
    }
    #[test] fn neutral_control_is_length_and_whitespace_matched_without_error_adaptation() {
        use unicode_segmentation::UnicodeSegmentation;
        let adaptive="é q\u{301} q 🙂\nש";
        let neutral=neutral_practice(adaptive,"é q\u{301} q 🙂\nש").unwrap();
        assert_eq!(adaptive.graphemes(true).count(),neutral.graphemes(true).count());
        let a=adaptive.graphemes(true).collect::<Vec<_>>();
        let n=neutral.graphemes(true).collect::<Vec<_>>();
        for (source,control) in a.iter().zip(&n) {
            assert_eq!(source.chars().all(char::is_whitespace),
                control.chars().all(char::is_whitespace));
        }
        assert!(!neutral.contains("é q\u{301} q 🙂\nש"));
        assert_eq!(neutral_practice("aaa","asdfjkl;"),Some("asd".into()));
        assert!(neutral_practice("","aaa").is_none());
        assert!(neutral_practice("aaa","").is_none());
    }
    fn synthetic_custom(reference:&str,typed:&str)->Session {
        let mut session=Session::new_with_custom(0,TestMode::Custom,reference);
        session.begin();
        session.type_text(typed);
        assert_eq!(session.state,State::Finished);
        session
    }
    #[test] fn controlled_assignment_uses_both_uniform_random_parities() {
        let mut adaptive=0;
        let mut neutral=0;
        for sample in 0..=u8::MAX {
            match Arm::from_random_byte(sample) {
                Arm::Adaptive=>adaptive+=1,
                Arm::NonAdaptive=>neutral+=1,
            }
        }
        assert_eq!((adaptive,neutral),(128,128));
    }
    #[test] fn transfer_requires_different_held_out_text_and_actual_practice() {
        let baseline=synthetic_custom("abab","axab");
        assert!(ControlledTrial::begin(&baseline,"abab",Arm::Adaptive,
            "a","us","os:unknown",Mode::System).is_none());
        assert!(ControlledTrial::begin(&baseline,"xxababyy",Arm::Adaptive,
            "a","us","os:unknown",Mode::System).is_none(),
            "A full held-out passage embedded in practice is not withheld");
        let unicode=synthetic_custom("é","é");
        assert!(ControlledTrial::begin(&unicode,"e\u{301}",Arm::Adaptive,
            "a","us","os:unknown",Mode::System).is_none(),
            "Canonical Unicode equivalents must never count as separate texts");
        let normalized_practice=ControlledTrial::begin(&baseline,"x   y",
            Arm::NonAdaptive,"a","us","os:unknown",Mode::System).unwrap();
        assert_eq!(normalized_practice.practice,"x y",
            "Practice reference uses exactly the normalized native Custom source");
        let mut trial=ControlledTrial::begin(&baseline,"xyxy",Arm::Adaptive,
            "a","us","os:unknown",Mode::System).unwrap();
        let practiced=synthetic_custom("xyxy","xyxy");
        let immediate=synthetic_custom("abab","abab");
        assert!(!trial.record_immediate(&immediate,"a","us","os:unknown",Mode::System),
            "Retest before intervention cannot count as transfer");
        assert!(!trial.record_practice(&immediate,"a","us","os:unknown",Mode::System),
            "Replaying the held-out passage cannot pass as practice");
        assert!(!trial.record_practice(&practiced,"a","de","os:unknown",Mode::System),
            "Practice with changed layout cannot advance the trial");
        assert!(trial.record_practice(&practiced,"a","us","os:unknown",Mode::System));
        assert_eq!(trial.stage,TrialStage::Immediate);
        assert!(!trial.record_practice(&practiced,"a","us","os:unknown",Mode::System));
        assert!(!trial.record_immediate(&immediate,"b","us","os:unknown",Mode::System));
        assert!(!trial.record_immediate(&immediate,"a","us","os:changed",Mode::System));
        assert!(!trial.record_immediate(&immediate,"a","us","os:unknown",Mode::Emulated));
        assert!(trial.record_immediate(&immediate,"a","us","os:unknown",Mode::System));
        assert_eq!(trial.stage,TrialStage::Delayed);
        assert!(!trial.record_delayed(&immediate,
            "a","us","os:unknown",Mode::System),
            "An immediate repeat cannot be recorded as delayed retention");
        // Inject a previously measured monotonic instant only within this
        // isolated unit test. Production callers cannot set the clock.
        trial.immediate_at=Some(std::time::Instant::now()-Duration::from_secs(3600));
        assert!(trial.record_delayed(&immediate,
            "a","us","os:unknown",Mode::System));
        assert_eq!(trial.stage,TrialStage::Complete);
        assert!(!trial.record_delayed(&immediate,
            "a","us","os:unknown",Mode::System),
            "A second delayed observation must not silently overwrite the first");
        assert_eq!(trial.baseline.opportunities,OpportunityCounts{observed:4,missed:1});
        assert_eq!(trial.immediate.unwrap().opportunities.missed,0);
    }
    fn completed_synthetic_arm(arm:Arm,profile:&str,initial:&str,
        immediate:&str,delayed:&str,practice:&str)->ControlledTrial {
        let base=synthetic_custom("abab",initial);
        let mut trial=ControlledTrial::begin(&base,practice,arm,profile,
            "us","os:unknown",Mode::System).unwrap();
        let drill=synthetic_custom(practice,practice);
        let instant=synthetic_custom("abab",immediate);
        let later=synthetic_custom("abab",delayed);
        assert!(trial.record_practice(&drill,profile,"us","os:unknown",Mode::System));
        assert!(trial.record_immediate(&instant,profile,"us","os:unknown",Mode::System));
        trial.immediate_at=Some(std::time::Instant::now()-Duration::from_secs(3600));
        assert!(trial.record_delayed(&later,profile,
            "us","os:unknown",Mode::System));
        trial
    }
    #[test] fn controlled_contrast_reports_zero_and_unfavorable_outcomes() {
        let a=completed_synthetic_arm(Arm::Adaptive,"a","axab","abab","abab","xyxy");
        let n=completed_synthetic_arm(Arm::NonAdaptive,"b","axab","abab","abab","wxyz");
        let result=control_adjusted_transfer(&a,&n).unwrap();
        assert_eq!(result.immediate_missed_pp,0.0);
        assert_eq!(result.delayed_missed_pp,0.0);
        assert!(result.immediate_wpm.is_finite());
        let bad=completed_synthetic_arm(Arm::Adaptive,"c","axab","axab","axab","xyxy");
        let better_control=control_adjusted_transfer(&bad,&n).unwrap();
        assert_eq!(better_control.immediate_missed_pp,25.0);
        assert_eq!(better_control.delayed_missed_pp,25.0);
        assert!(control_adjusted_transfer(&n,&a).is_none(),
            "Arm labels must not be swapped or inferred from performance");
    }
    #[test] fn aggregate_export_is_opt_in_complete_only_and_excludes_text_identity() {
        let incomplete=synthetic_custom("abab","axab");
        let p=ControlledTrial::begin(&incomplete,"xyxy",Arm::Adaptive,
            "very_private_profile","my-private-layout","private-seat",Mode::System).unwrap();
        assert!(p.aggregate_export().is_none());
        let completed=completed_synthetic_arm(Arm::Adaptive,
            "private_profile","axab","abab","abab","private_practice");
        let raw=completed.aggregate_export().unwrap();
        let record:serde_json::Value=serde_json::from_str(&raw).unwrap();
        assert_eq!(record["schema"],1);
        assert_eq!(record["causal_claim"],false);
        assert_eq!(record["control_adjusted"],false);
        assert_eq!(record["randomized_arm"],"adaptive");
        assert_eq!(record["baseline"]["opportunities"],4);
        assert_eq!(record["baseline"]["ever_missed"],1);
        assert_eq!(record["delayed"]["ever_missed"],0);
        for text in ["private_profile","private_practice","abab","axab","os:unknown",
            "my-private-layout","private-seat"] {
            assert!(!raw.contains(text),"Private source leaked into de-identified export");
        }
    }
    #[test] fn matched_control_rejects_different_held_out_passages_even_with_equal_counts() {
        let adaptive=completed_synthetic_arm(Arm::Adaptive,
            "a","axab","abab","abab","xyxy");
        let baseline=synthetic_custom("wxyz","wxyx");
        let mut control=ControlledTrial::begin(&baseline,"asdf",Arm::NonAdaptive,
            "b","us","os:unknown",Mode::System).unwrap();
        let practice=synthetic_custom("asdf","asdf");
        let immediate=synthetic_custom("wxyz","wxyz");
        assert!(control.record_practice(&practice,"b","us","os:unknown",Mode::System));
        assert!(control.record_immediate(&immediate,"b","us","os:unknown",Mode::System));
        control.immediate_at=Some(std::time::Instant::now()-Duration::from_secs(3600));
        assert!(control.record_delayed(&immediate,"b","us","os:unknown",Mode::System));
        assert_eq!(adaptive.baseline.opportunities.observed,control.baseline.opportunities.observed);
        assert_eq!(adaptive.practiced.unwrap().opportunities.observed,
            control.practiced.unwrap().opportunities.observed);
        assert!(control_adjusted_transfer(&adaptive,&control).is_none(),
            "Numerically matched but different held-out texts are not comparable");
    }
    #[test] fn invalid_controlled_pair_never_makes_unsupported_comparison() {
        let a=completed_synthetic_arm(Arm::Adaptive,"a","axab","abab","abab","xyxy");
        let same_identity=completed_synthetic_arm(Arm::NonAdaptive,"a",
            "axab","abab","abab","wxyz");
        assert!(control_adjusted_transfer(&a,&same_identity).is_none(),
            "One participant is not an independent control participant");
        let mut unequal_exposure=completed_synthetic_arm(Arm::NonAdaptive,"b",
            "axab","abab","abab","wxyz");
        unequal_exposure.practiced.as_mut().unwrap().opportunities.observed+=1;
        assert!(control_adjusted_transfer(&a,&unequal_exposure).is_none(),
            "Different practice opportunities must not be hidden");
        let mut unequal_time=completed_synthetic_arm(Arm::NonAdaptive,"time",
            "axab","abab","abab","wxyz");
        unequal_time.practiced.as_mut().unwrap().elapsed_ms=12_000;
        assert!(control_adjusted_transfer(&a,&unequal_time).is_none(),
            "A paired control cannot hide materially different practice time");
        // Exercise the actual 10% boundary with realistic nonzero timings,
        // not merely the zero-duration synthetic fast path.
        let mut timed_adaptive=completed_synthetic_arm(Arm::Adaptive,"time_a",
            "axab","abab","abab","xyxy");
        timed_adaptive.practiced.as_mut().unwrap().elapsed_ms=10_000;
        let mut tolerable_time=completed_synthetic_arm(Arm::NonAdaptive,"time_ok",
            "axab","abab","abab","wxyz");
        tolerable_time.practiced.as_mut().unwrap().elapsed_ms=11_000;
        assert!(control_adjusted_transfer(&timed_adaptive,&tolerable_time).is_some());
        tolerable_time.practiced.as_mut().unwrap().elapsed_ms=11_200;
        assert!(control_adjusted_transfer(&timed_adaptive,&tolerable_time).is_none(),
            "More than 10% practice-time difference is not a matched dose");
        let mut missing_delayed=completed_synthetic_arm(Arm::NonAdaptive,"c",
            "axab","abab","abab","wxyz");
        missing_delayed.stage=TrialStage::Delayed;
        missing_delayed.delayed=None;
        assert!(control_adjusted_transfer(&a,&missing_delayed).is_none());
    }
    #[test] fn unequal_attempt_counts_are_disclosed_not_hidden(){
        let x=Comparison{baseline_accuracy:80.0,retest_accuracy:90.0,
            baseline_wpm:30.0,retest_wpm:25.0,baseline_attempts:30,retest_attempts:50,
            baseline_opportunities:OpportunityCounts{observed:20,missed:5},
            retest_opportunities:OpportunityCounts{observed:20,missed:2}};
        assert_eq!(x.accuracy_change_pp(),10.0);
        assert_eq!(x.speed_change_wpm(),-5.0);
        assert!(!x.same_attempt_count());
        assert_eq!(x.missed_opportunity_change_pp(),Some(-15.0));
        assert_eq!(Comparison{
            baseline_opportunities:OpportunityCounts::default(),..x
        }.missed_opportunity_change_pp(),None);
    }
}
