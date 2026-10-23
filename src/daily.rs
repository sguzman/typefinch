//! Local-calendar daily goals from existing aggregate sessions.
//! A timezone is applied to each record's timestamp, so DST transitions are
//! handled by the timezone database rather than assuming a fixed UTC offset.
use crate::history::Record;
use chrono::{DateTime,NaiveDate,TimeZone,Utc};
use std::collections::HashSet;

#[derive(Debug,Clone,Copy,Default,PartialEq)]
pub struct DailySummary {
    pub sessions:usize,
    pub graphemes:i64,
    pub weighted_accuracy:f64,
}
fn qualifying(record:&Record)->bool {
    record.duration_ms>=15_000 && record.elapsed_ms>=1_000 &&
        record.entered>=20 && record.test_mode!="custom" &&
        !record.test_mode.starts_with("lesson/")
}
fn local_day<Tz:TimeZone>(record:&Record,timezone:&Tz)->Option<NaiveDate> {
    DateTime::<Utc>::from_timestamp_millis(record.finished_at_ms)
        .map(|at|at.with_timezone(timezone).date_naive())
}
/// Deliberately optional and passive. Includes yesterday as a grace day when
/// today has not yet had a qualifying session.
pub fn streak<Tz:TimeZone>(records:&[Record],today:NaiveDate,timezone:&Tz)->usize {
    let days:HashSet<_>=records.iter().filter(|r|qualifying(r))
        .filter_map(|r|local_day(r,timezone)).collect();
    let mut current=if days.contains(&today) {today} else {
        match today.pred_opt() {Some(day)=>day,None=>return 0}
    };
    let mut count=0;
    while days.contains(&current) {
        count+=1;
        let Some(previous)=current.pred_opt() else {break;};
        current=previous;
    }
    count
}
pub fn summary<Tz:TimeZone>(records:&[Record],date:NaiveDate,timezone:&Tz)->DailySummary {
    let mut result=DailySummary::default();
    let mut weighted=0.0;
    for record in records {
        if !qualifying(record) || local_day(record,timezone)!=Some(date) {continue;}
        result.sessions+=1;
        result.graphemes+=record.entered;
        weighted+=record.accuracy*record.entered as f64;
    }
    if result.graphemes>0 {
        result.weighted_accuracy=weighted/result.graphemes as f64;
    }
    result
}
#[cfg(test)]
mod tests {
    use super::*;
    use chrono::FixedOffset;
    fn row(utc:&str)->Record {
        let ms=DateTime::parse_from_rfc3339(utc).unwrap().timestamp_millis();
        Record{id:0,profile_id:"p".into(),finished_at_ms:ms,layout:"us".into(),
            input_mode:"OS".into(),test_mode:"words".into(),
            duration_ms:30000,elapsed_ms:30000,entered:100,correct:98,
            wpm:40.0,accuracy:98.0,raw_wpm:44.0,raw_accuracy:97.0,
            attempts:102,mistakes:3,corrections:2,scoring_version:2}
    }
    #[test] fn streak_uses_local_calendar_with_one_day_grace_and_no_private_sessions() {
        let mx=FixedOffset::west_opt(6*3600).unwrap();
        let today=NaiveDate::from_ymd_opt(2026,10,23).unwrap();
        let yesterday=row("2026-10-23T03:00:00Z"); // local October 22
        let prior=row("2026-10-21T18:00:00Z"); // local October 21
        let mut lesson=row("2026-10-23T19:00:00Z");
        lesson.test_mode="lesson/home".into();
        assert_eq!(streak(&[prior.clone(),yesterday.clone(),lesson],today,&mx),2);
        let today_record=row("2026-10-23T18:00:00Z");
        assert_eq!(streak(&[prior,yesterday,today_record],today,&mx),3);
        assert_eq!(streak(&[],today,&mx),0);
    }
    #[test] fn local_day_is_not_utc_day_and_filters_private_passages() {
        let mx=FixedOffset::west_opt(6*3600).unwrap();
        let day=NaiveDate::from_ymd_opt(2026,10,23).unwrap();
        let a=row("2026-10-23T03:00:00Z"); // Oct 22 at -06:00
        let mut b=row("2026-10-23T18:00:00Z"); // Oct 23 at -06:00
        b.accuracy=96.0;
        b.entered=50;
        let mut c=b.clone();
        c.test_mode="custom".into();
        let mut short=b.clone();
        short.duration_ms=2000;
        let result=summary(&[a,b,c,short],day,&mx);
        assert_eq!(result.sessions,1);
        assert_eq!(result.graphemes,50);
        assert_eq!(result.weighted_accuracy,96.0);
        assert_eq!(summary(&[],day,&mx),DailySummary::default());
    }
}