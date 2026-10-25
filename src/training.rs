//! In-memory targeted-practice follow-up. It is an observational paired
//! measurement, NOT a causal study of typing-training efficacy.
//! Never persist a source passage, character-level errors or this trial.
use crate::{engine::{Language,Score,Session,State,TestMode,WordDifficulty},layouts::Mode};
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
}
impl Comparison {
    pub fn accuracy_change_pp(self)->f64 {self.retest_accuracy-self.baseline_accuracy}
    pub fn speed_change_wpm(self)->f64 {self.retest_wpm-self.baseline_wpm}
    pub fn same_attempt_count(self)->bool {self.baseline_attempts==self.retest_attempts}
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
            input,baseline,stage:Stage::Drill,comparison:None,
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
            || self.mode!=session.mode || self.duration!=session.duration
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
    #[test] fn unequal_attempt_counts_are_disclosed_not_hidden(){
        let x=Comparison{baseline_accuracy:80.0,retest_accuracy:90.0,
            baseline_wpm:30.0,retest_wpm:25.0,baseline_attempts:30,retest_attempts:50};
        assert_eq!(x.accuracy_change_pp(),10.0);
        assert_eq!(x.speed_change_wpm(),-5.0);
        assert!(!x.same_attempt_count());
    }
}
