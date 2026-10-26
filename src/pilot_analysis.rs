//! Read-only, opt-in comparison of explicit local Typefinch study exports.
//! This does NOT establish independent participants, valid randomization,
//! equal training dose, or human efficacy. No network or implicit file search.
use crate::training::{PUBLIC_STUDY_REFERENCE,PUBLIC_STUDY_REFERENCE_ID};
use serde::{Deserialize,Serialize};
use serde_json::{json,Value};
use std::{collections::HashSet,fs::OpenOptions,io::Read,
    os::unix::fs::{MetadataExt,OpenOptionsExt},path::PathBuf};
use unicode_segmentation::UnicodeSegmentation;

const MAX_FILE_BYTES:u64=32_768;
const MAX_RECORDS:usize=200;
const EXPORT_CONSENT:&str="explicit local clipboard action; no network publication";

#[derive(Debug,Deserialize,Serialize)]
#[serde(deny_unknown_fields)]
struct Measurement {
    opportunities:usize,
    ever_missed:usize,
    corrected_wpm:f64,
    attempts:usize,
    elapsed_ms:u64,
}
impl Measurement {
    fn check(&self,reference:Option<usize>)->Result<(),String> {
        if self.opportunities==0 || self.ever_missed>self.opportunities
            || self.attempts<self.opportunities
            || reference.is_some_and(|r|r!=self.opportunities)
            || self.elapsed_ms==0 || self.elapsed_ms>86_400_000
            || !self.corrected_wpm.is_finite()
            || !(0.0..=1000.0).contains(&self.corrected_wpm) {
            return Err("Invalid or incomplete aggregate measurement".into());
        }
        Ok(())
    }
    fn missed_pp(&self)->f64 {
        self.ever_missed as f64*100.0/self.opportunities as f64
    }
}
#[derive(Debug,Deserialize,Serialize)]
#[serde(deny_unknown_fields)]
struct Pilot {
    schema:u32,
    #[serde(rename="type")] kind:String,
    app_version:String,
    randomized_arm:String,
    input_mode:String,
    configured_duration_seconds:u64,
    baseline:Measurement,
    practice:Measurement,
    immediate:Measurement,
    delayed:Measurement,
    control_adjusted:bool,
    causal_claim:bool,
    consent:String,
    reference_opportunities:usize,
    coverage_policy:String,
    public_reference_cohort:Option<String>,
    immediate_deadline_seconds:u64,
    retention_window_seconds:[u64;2],
    actual_retention_gap_ms:u64,
}
impl Pilot {
    fn check(&self)->Result<(),String> {
        let expected=PUBLIC_STUDY_REFERENCE.graphemes(true).count();
        if self.schema!=4 || self.kind!="typefinch-local-held-out-pilot"
            || self.app_version.is_empty() || self.app_version.len()>32
            || !self.app_version.chars().all(|ch|
                ch.is_ascii_alphanumeric() || matches!(ch,'.'|'-'|'+'))
            || !matches!(self.randomized_arm.as_str(),"adaptive"|"non_adaptive")
            || !matches!(self.input_mode.as_str(),"os"|"emulated")
            || self.configured_duration_seconds!=0
            || self.reference_opportunities!=expected
            || self.public_reference_cohort.as_deref()!=Some(PUBLIC_STUDY_REFERENCE_ID)
            || self.coverage_policy!="all_reference_positions_reached"
            || self.immediate_deadline_seconds!=600
            || self.retention_window_seconds!=[3600,86400]
            || !(3_600_000..=86_400_000).contains(&self.actual_retention_gap_ms)
            || self.control_adjusted || self.causal_claim
            || self.consent!=EXPORT_CONSENT {
            return Err("Incompatible, nonpublic, or unverifiable study export".into());
        }
        self.baseline.check(Some(expected))?;
        self.practice.check(None)?;
        self.immediate.check(Some(expected))?;
        self.delayed.check(Some(expected))?;
        Ok(())
    }
}
/// Strict fixed-size no-follow local reader shared by both completed
/// aggregate comparison and public-cohort exclusion summaries.
pub(crate) fn read_local_bytes(path:&PathBuf)->Result<(Vec<u8>,(u64,u64)),String> {
    // A caller must provide every input path. Never follow symlinks, read a
    // directory/device, or print file content/path in diagnostic errors.
    let mut file=OpenOptions::new().read(true)
        .custom_flags(libc::O_NOFOLLOW|libc::O_NONBLOCK)
        .open(path).map_err(|_|"Cannot open a study aggregate regular file")?;
    let meta=file.metadata().map_err(|_|"Cannot inspect study aggregate file")?;
    if !meta.is_file() || meta.len()>MAX_FILE_BYTES {
        return Err("Expected a regular study aggregate file of at most 32 KiB".into());
    }
    let identity=(meta.dev(),meta.ino());
    let mut data=Vec::new();
    file.take(MAX_FILE_BYTES+1).read_to_end(&mut data)
        .map_err(|_|"Cannot read local study aggregate file")?;
    if data.len() as u64>MAX_FILE_BYTES {
        return Err("Study aggregate file exceeds 32 KiB".into());
    }
    Ok((data,identity))
}
fn read_local(path:&PathBuf)->Result<(Pilot,(u64,u64)),String> {
    let (data,identity)=read_local_bytes(path)?;
    let item: Pilot=serde_json::from_slice(&data)
        .map_err(|_|"Study aggregate is not valid schema-4 JSON")?;
    item.check()?;
    Ok((item,identity))
}
fn mean(values:&mut [f64])->f64 {
    values.sort_by(f64::total_cmp);
    values.iter().sum::<f64>()/values.len() as f64
}
fn arm_summary(group:&[&Pilot])->Value {
    let calc=|f:&dyn Fn(&Pilot)->f64| {
        let mut values=group.iter().map(|x|f(x)).collect::<Vec<_>>();
        mean(&mut values)
    };
    json!({
        "n":group.len(),
        "baseline_missed_pp":calc(&|p|p.baseline.missed_pp()),
        "immediate_change_pp":calc(&|p|p.immediate.missed_pp()-p.baseline.missed_pp()),
        "delayed_change_pp":calc(&|p|p.delayed.missed_pp()-p.baseline.missed_pp()),
        "immediate_wpm_change":calc(&|p|p.immediate.corrected_wpm-p.baseline.corrected_wpm),
        "delayed_wpm_change":calc(&|p|p.delayed.corrected_wpm-p.baseline.corrected_wpm),
        "practice_opportunities_mean":calc(&|p|p.practice.opportunities as f64),
        "practice_elapsed_ms_mean":calc(&|p|p.practice.elapsed_ms as f64),
    })
}
fn difference(a:&Value,b:&Value,key:&str)->f64 {
    a[key].as_f64().unwrap()-b[key].as_f64().unwrap()
}
fn summarize(records:&[Pilot])->Result<Value,String> {
    if !(2..=MAX_RECORDS).contains(&records.len()){
        return Err("Supply 2 to 200 explicitly selected aggregate files".into());
    }
    for record in records {record.check()?;}
    let first=&records[0];
    if records.iter().any(|x|
        x.app_version!=first.app_version || x.input_mode!=first.input_mode) {
        return Err("Cannot pool different application versions or input modes".into());
    }
    let adaptive=records.iter().filter(|x|x.randomized_arm=="adaptive")
        .collect::<Vec<_>>();
    let neutral=records.iter().filter(|x|x.randomized_arm=="non_adaptive")
        .collect::<Vec<_>>();
    if adaptive.is_empty()||neutral.is_empty() {
        return Err("At least one valid aggregate from each randomized arm is required".into());
    }
    let a=arm_summary(&adaptive);
    let n=arm_summary(&neutral);
    // Compute contrasts BEFORE consuming either summary as a JSON value.
    let immediate_missed=difference(&a,&n,"immediate_change_pp");
    let delayed_missed=difference(&a,&n,"delayed_change_pp");
    let immediate_wpm=difference(&a,&n,"immediate_wpm_change");
    let delayed_wpm=difference(&a,&n,"delayed_wpm_change");
    Ok(json!({
        "schema":1,
        "type":"typefinch-local-descriptive-cohort-comparison",
        "source":"explicit-local-aggregate-files",
        "public_reference_cohort":PUBLIC_STUDY_REFERENCE_ID,
        "app_version":first.app_version,
        "input_mode":first.input_mode,
        "total_exported_records":records.len(),
        "adaptive":a,
        "non_adaptive":n,
        "difference_of_mean_immediate_missed_change_pp":immediate_missed,
        "difference_of_mean_delayed_missed_change_pp":delayed_missed,
        "difference_of_mean_immediate_wpm_change":immediate_wpm,
        "difference_of_mean_delayed_wpm_change":delayed_wpm,
        "descriptive_only":true,
        "causal_claim":false,
        "warnings":[
            "Participant independence, duplicate enrollment, real randomization, physical keyboard layout and legitimate human performance cannot be verified from de-identified exports",
            "Public passage is reused at baseline and retests, so familiarity and selection effects remain",
            "Completed exports exclude dropouts; differential attrition and practice difficulty may bias comparisons",
            "No preregistered confidence intervals or causal efficacy conclusion is produced"
        ]
    }))
}
pub fn analyze(paths:&[PathBuf])->Result<Value,String> {
    if !(2..=MAX_RECORDS).contains(&paths.len()){
        return Err("Supply 2 to 200 explicitly selected aggregate files".into());
    }
    let mut identities=HashSet::new();
    // A copied or differently formatted version of the exact same exported
    // aggregates must not be treated as another independent observation.
    // Canonical bytes remain in process memory only, never persisted or sent.
    let mut canonical_records=HashSet::new();
    let mut records=Vec::with_capacity(paths.len());
    for path in paths {
        let (record,identity)=read_local(path)?;
        if !identities.insert(identity) {
            // Reject exact same opened inode, even through a hard link.
            // Do not print paths, hashes, or participant-level data.
            return Err("Repeated input file; each aggregate may be supplied only once".into());
        }
        let canonical=serde_json::to_string(&record)
            .map_err(|_|"Cannot normalize local aggregate record")?;
        if !canonical_records.insert(canonical) {
            return Err("Duplicate aggregate observations cannot be counted twice".into());
        }
        records.push(record);
    }
    summarize(&records)
}
#[cfg(test)]
mod tests {
    use super::*;
    fn sample(arm:&str,baseline_missed:usize,immediate_missed:usize,delayed_missed:usize)->Pilot {
        let make=|missed:usize| Measurement{
            opportunities:PUBLIC_STUDY_REFERENCE.graphemes(true).count(),
            ever_missed:missed,corrected_wpm:50.0,attempts:120,elapsed_ms:22_000,
        };
        Pilot{
            schema:4,kind:"typefinch-local-held-out-pilot".into(),
            app_version:"0.1.0".into(),randomized_arm:arm.into(),
            input_mode:"os".into(),configured_duration_seconds:0,
            baseline:make(baseline_missed),
            practice:Measurement {opportunities:15,ever_missed:0,corrected_wpm:40.0,
                attempts:15,elapsed_ms:20_000},
            immediate:make(immediate_missed),delayed:make(delayed_missed),
            control_adjusted:false,causal_claim:false,consent:EXPORT_CONSENT.into(),
            reference_opportunities:PUBLIC_STUDY_REFERENCE.graphemes(true).count(),
            coverage_policy:"all_reference_positions_reached".into(),
            public_reference_cohort:Some(PUBLIC_STUDY_REFERENCE_ID.into()),
            immediate_deadline_seconds:600,retention_window_seconds:[3600,86400],
            actual_retention_gap_ms:3_600_000,
        }
    }
    #[test] fn comparing_public_aggregates_discloses_only_group_statistics() {
        let cases=[sample("adaptive",5,1,2),sample("non_adaptive",5,4,5)];
        let result=summarize(&cases).unwrap();
        let opportunity=PUBLIC_STUDY_REFERENCE.graphemes(true).count() as f64;
        let contrast=(-3.0)*100.0/opportunity;
        assert!((result["difference_of_mean_immediate_missed_change_pp"]
            .as_f64().unwrap()-contrast).abs()<1e-8);
        assert!((result["difference_of_mean_delayed_missed_change_pp"]
            .as_f64().unwrap()-contrast).abs()<1e-8);
        assert_eq!(result["adaptive"]["n"],1);
        assert_eq!(result["non_adaptive"]["n"],1);
        assert_eq!(result["causal_claim"],false);
        assert!(result["warnings"].as_array().unwrap().len()>=4);
        let rendered=result.to_string();
        assert!(!rendered.contains(PUBLIC_STUDY_REFERENCE));
        assert!(!rendered.contains("private-profile"));
    }
    #[test] fn invalid_or_incompatible_comparisons_never_claim_efficacy() {
        let a=sample("adaptive",4,2,2);
        let n=sample("non_adaptive",4,4,4);
        assert!(summarize(&[sample("adaptive",4,2,2)]).is_err());
        assert!(summarize(&[sample("adaptive",4,2,2),sample("adaptive",4,2,2)]).is_err());
        let mut old=sample("non_adaptive",4,4,4);
        old.schema=3;
        assert!(summarize(&[a,n,old]).is_err());
        let mut leaked=sample("adaptive",4,2,2);
        leaked.public_reference_cohort=None;
        assert!(summarize(&[leaked,sample("non_adaptive",4,4,4)]).is_err());
        let mut malicious=sample("adaptive",4,2,2);
        malicious.app_version="a\\nprivate/path".into();
        assert!(summarize(&[malicious,sample("non_adaptive",4,4,4)]).is_err(),
            "Untrusted input metadata may not become echoed private output");
        let mut changed=sample("non_adaptive",4,4,4);
        changed.app_version="0.2.0".into();
        assert!(summarize(&[sample("adaptive",4,2,2),changed]).is_err());
        let mut impossible=sample("adaptive",4,2,2);
        impossible.immediate.ever_missed=impossible.immediate.opportunities+1;
        assert!(summarize(&[impossible,sample("non_adaptive",4,4,4)]).is_err());
        let mut timed=sample("adaptive",4,2,2);
        timed.actual_retention_gap_ms=86_400_001;
        assert!(summarize(&[timed,sample("non_adaptive",4,4,4)]).is_err());
    }
    #[test] fn local_reader_never_follows_a_symlink_or_reads_a_directory() {
        use std::os::unix::fs::symlink;
        let directory=std::env::temp_dir().join(format!("typefinch-pilot-{}",
            uuid::Uuid::new_v4()));
        std::fs::create_dir(&directory).unwrap();
        let file=directory.join("data.json");
        std::fs::write(&file,br#"{"raw_passage":"private"}"#).unwrap();
        let link=directory.join("link.json");
        symlink(&file,&link).unwrap();
        let symlink_error=read_local(&link).unwrap_err();
        assert!(!symlink_error.contains("private"));
        assert!(!symlink_error.contains(&directory.to_string_lossy().to_string()));
        assert!(read_local(&directory).is_err());
        std::fs::remove_dir_all(&directory).unwrap();
    }
    #[test] fn raw_passage_field_is_rejected_by_strict_schema() {
        let clean=sample("adaptive",4,2,2);
        let mut value=serde_json::json!({
            "schema":clean.schema,"type":clean.kind,"app_version":clean.app_version,
            "randomized_arm":clean.randomized_arm,"input_mode":clean.input_mode,
            "configured_duration_seconds":0,
            "reference_opportunities":clean.reference_opportunities,
            "coverage_policy":clean.coverage_policy,
            "public_reference_cohort":clean.public_reference_cohort,
            "immediate_deadline_seconds":600,"retention_window_seconds":[3600,86400],
            "actual_retention_gap_ms":3600000,"control_adjusted":false,
            "causal_claim":false,"consent":clean.consent,
            "baseline":{"opportunities":clean.baseline.opportunities,"ever_missed":4,"corrected_wpm":50.0,"attempts":120,"elapsed_ms":22000},
            "practice":{"opportunities":15,"ever_missed":0,"corrected_wpm":40.0,"attempts":15,"elapsed_ms":20000},
            "immediate":{"opportunities":clean.immediate.opportunities,"ever_missed":2,"corrected_wpm":50.0,"attempts":120,"elapsed_ms":22000},
            "delayed":{"opportunities":clean.delayed.opportunities,"ever_missed":2,"corrected_wpm":50.0,"attempts":120,"elapsed_ms":22000},
        });
        let parsed: Pilot=serde_json::from_value(value.clone()).unwrap();
        assert!(parsed.check().is_ok());
        value["raw_passage"]=json!("do not ingest my private text");
        assert!(serde_json::from_value::<Pilot>(value).is_err());
    }
}
