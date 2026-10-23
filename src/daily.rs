//! Local-calendar daily goals from existing aggregate sessions.
//! A timezone is applied to each record's timestamp, so DST transitions are
//! handled by the timezone database rather than assuming a fixed UTC offset.
use crate::history::Record;
use chrono::{DateTime,NaiveDate,TimeZone,Utc};

#[derive(Debug,Clone,Copy,Default,PartialEq)]
pub struct DailySummary {
    pub sessions:usize,
    pub graphemes:i64,
    pub weighted_accuracy:f64,
}
pub fn summary<Tz:TimeZone>(records:&[Record],date:NaiveDate,timezone:&Tz)->DailySummary {
    let mut result=DailySummary::default();
    let mut weighted=0.0;
    for record in records {
        if record.duration_ms<15_000 || record.elapsed_ms<1_000
            || record.entered<20 || record.test_mode=="custom"
            || record.test_mode.starts_with("lesson/") {continue;}
        let Some(at)=DateTime::<Utc>::from_timestamp_millis(record.finished_at_ms) else {continue;};
        if at.with_timezone(timezone).date_naive()!=date {continue;}
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