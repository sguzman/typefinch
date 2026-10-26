//! Explicitly imported local exclusion tallies for public-cohort study design.
//! These records have NO participant identifiers; separate exports may be
//! duplicates or independent participants, which this CLI cannot determine.
use crate::{pilot_analysis::read_local_bytes,training::PUBLIC_STUDY_REFERENCE_ID};
use serde::Deserialize;
use serde_json::{json,Value};
use std::{collections::HashSet,path::PathBuf};

const MAX_RECORDS:usize=200;
const CONSENT:&str="explicit local clipboard action; no network publication";
#[derive(Debug,Deserialize)]
#[serde(deny_unknown_fields)]
struct Exclusion {
    schema:u32,
    #[serde(rename="type")] kind:String,
    public_reference_cohort:String,
    app_version:String,
    randomized_arm:String,
    excluded_stage:String,
    reason:String,
    causal_claim:bool,
    consent:String,
}
impl Exclusion {
    fn check(&self)->Result<(),String> {
        if self.schema!=1 || self.kind!="typefinch-local-pilot-exclusion"
            || self.public_reference_cohort!=PUBLIC_STUDY_REFERENCE_ID
            || self.app_version.is_empty() || self.app_version.len()>32
            || !self.app_version.chars().all(|ch|
                ch.is_ascii_alphanumeric() || matches!(ch,'.'|'-'|'+'))
            || !matches!(self.randomized_arm.as_str(),"adaptive"|"non_adaptive")
            || !matches!(self.excluded_stage.as_str(),"practice"|"immediate"|"delayed")
            || !matches!(self.reason.as_str(),
                "participant_withdrew"|"protocol_invalid"|"focus_lost"|"manual_restart")
            || self.causal_claim || self.consent!=CONSENT {
            return Err("Invalid, nonpublic or incompatible local exclusion summary".into());
        }
        Ok(())
    }
}
#[derive(Default)]
struct Counts {
    submissions:usize,
    practice:usize,
    immediate:usize,
    delayed:usize,
    withdrew:usize,
    invalid:usize,
    lost_focus:usize,
    restarted:usize,
}
impl Counts {
    fn add(&mut self,record:&Exclusion) {
        self.submissions+=1;
        match record.excluded_stage.as_str() {
            "practice"=>self.practice+=1,"immediate"=>self.immediate+=1,
            "delayed"=>self.delayed+=1,_=>unreachable!("validated stage"),
        }
        match record.reason.as_str() {
            "participant_withdrew"=>self.withdrew+=1,
            "protocol_invalid"=>self.invalid+=1,
            "focus_lost"=>self.lost_focus+=1,
            "manual_restart"=>self.restarted+=1,
            _=>unreachable!("validated reason"),
        }
    }
    fn render(&self)->Value {
        json!({"submitted":self.submissions,
            "by_stage":{"practice":self.practice,"immediate":self.immediate,"delayed":self.delayed},
            "by_reason":{"participant_withdrew":self.withdrew,
                "protocol_invalid":self.invalid,"focus_lost":self.lost_focus,
                "manual_restart":self.restarted}
        })
    }
}
fn summarize(records:&[Exclusion])->Result<Value,String>{
    if !(1..=MAX_RECORDS).contains(&records.len()){
        return Err("Supply 1 to 200 explicitly selected exclusion summaries".into());
    }
    for r in records{r.check()?;}
    let version=&records[0].app_version;
    if records.iter().any(|r|&r.app_version!=version){
        return Err("Cannot pool exclusion summaries from different app versions".into());
    }
    let mut adaptive=Counts::default();
    let mut control=Counts::default();
    for r in records {
        if r.randomized_arm=="adaptive"{adaptive.add(r);}
        else{control.add(r);}
    }
    Ok(json!({
        "schema":1,
        "type":"typefinch-local-public-cohort-exclusion-tally",
        "source":"explicit-local-exclusion-files",
        "public_reference_cohort":PUBLIC_STUDY_REFERENCE_ID,
        "app_version":version,
        "total_submitted_exclusions":records.len(),
        "adaptive":adaptive.render(),
        "non_adaptive":control.render(),
        "descriptive_only":true,
        "causal_claim":false,
        "warnings":[
            "Anonymous exclusion records cannot establish participant independence or detect copied/repeated submissions",
            "A submitted exclusion tally is not evidence of a unique enrolled person or genuine randomization",
            "Participants who close the app without voluntarily copying an exclusion are not counted",
            "Completion/attrition comparisons require independent recruitment and verified denominators"
        ]
    }))
}
pub fn analyze(paths:&[PathBuf])->Result<Value,String>{
    if !(1..=MAX_RECORDS).contains(&paths.len()){
        return Err("Supply 1 to 200 explicitly selected exclusion files".into());
    }
    let mut seen=HashSet::new();
    let mut records=Vec::with_capacity(paths.len());
    for path in paths{
        let (raw,id)=read_local_bytes(path)?;
        if !seen.insert(id) {
            return Err("Repeated local exclusion file or hard-link alias".into());
        }
        let item:Exclusion=serde_json::from_slice(&raw)
            .map_err(|_|"Exclusion is not valid public-cohort schema-1 JSON")?;
        item.check()?;
        records.push(item);
    }
    summarize(&records)
}
#[cfg(test)]
mod tests {
    use super::*;
    fn record(arm:&str,stage:&str,reason:&str)->Exclusion{
        Exclusion{schema:1,kind:"typefinch-local-pilot-exclusion".into(),
            public_reference_cohort:PUBLIC_STUDY_REFERENCE_ID.into(),
            app_version:"0.1.0".into(),randomized_arm:arm.into(),
            excluded_stage:stage.into(),reason:reason.into(),
            causal_claim:false,consent:CONSENT.into()}
    }
    #[test] fn only_descriptive_arm_stage_and_reason_counts_are_emitted(){
        let rows=[
            record("adaptive","practice","participant_withdrew"),
            record("non_adaptive","immediate","focus_lost"),
            record("adaptive","delayed","protocol_invalid")
        ];
        let report=summarize(&rows).unwrap();
        assert_eq!(report["total_submitted_exclusions"],3);
        assert_eq!(report["adaptive"]["submitted"],2);
        assert_eq!(report["adaptive"]["by_stage"]["practice"],1);
        assert_eq!(report["adaptive"]["by_stage"]["delayed"],1);
        assert_eq!(report["non_adaptive"]["by_reason"]["focus_lost"],1);
        assert_eq!(report["causal_claim"],false);
        assert_eq!(report["descriptive_only"],true);
        let output=report.to_string();
        for text in ["profile-id","keyboard-address","private-phrase"]{
            assert!(!output.contains(text));
        }
    }
    #[test] fn invalid_or_private_fields_and_mixed_versions_are_refused(){
        let mut private=record("adaptive","practice","participant_withdrew");
        private.public_reference_cohort="a-secret-passage".into();
        assert!(summarize(&[private]).is_err());
        let mut invalid=record("adaptive","practice","protocol_invalid");
        invalid.excluded_stage="completed".into();
        assert!(summarize(&[invalid]).is_err());
        let mut incompatible=record("non_adaptive","delayed","focus_lost");
        incompatible.app_version="other".into();
        assert!(summarize(&[record("adaptive","practice","manual_restart"),
            incompatible]).is_err());
        let mut forged=record("adaptive","practice","manual_restart");
        forged.causal_claim=true;
        assert!(summarize(&[forged]).is_err());
        let raw=json!({"schema":1,"type":"typefinch-local-pilot-exclusion",
            "public_reference_cohort":PUBLIC_STUDY_REFERENCE_ID,
            "app_version":"0.1.0","randomized_arm":"adaptive",
            "excluded_stage":"practice","reason":"participant_withdrew",
            "causal_claim":false,"consent":CONSENT,
            "raw_passage":"DO NOT ACCEPT"});
        assert!(serde_json::from_value::<Exclusion>(raw).is_err());
    }
}
