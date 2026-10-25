//! In-memory targeted-practice follow-up. It is an observational paired
//! measurement, NOT a causal study of typing-training efficacy.
//! Never persist a source passage, character-level errors or this trial.
use crate::{engine::{Language,OpportunityCounts,Score,Session,State,TestMode,WordDifficulty},layouts::Mode};
use std::time::Duration;

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
