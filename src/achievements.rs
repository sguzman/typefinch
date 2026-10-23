//! Offline achievements derived from aggregate SQLite session records.
//! Badge computation is deterministic and does not persist user-entered text.
use crate::history::{Record,SCORING_VERSION};
use std::collections::HashSet;

#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub struct Achievement {
    pub id:&'static str,
    pub emoji:&'static str,
    pub title:&'static str,
    pub reason:&'static str,
}
pub const FIRST:Achievement=Achievement{id:"first-flight",emoji:"🐣",title:"First flight",reason:"Complete one substantial timed benchmark"};
pub const PERSISTENT:Achievement=Achievement{id:"steady-wings",emoji:"🪶",title:"Steady wings",reason:"Complete 10 substantial timed benchmarks"};
pub const PRECISE:Achievement=Achievement{id:"precision",emoji:"🎯",title:"Precision landing",reason:"Reach 98% accuracy on at least 60 graphemes"};
pub const FAST:Achievement=Achievement{id:"fast-wings",emoji:"⚡",title:"Fast wings",reason:"Reach 80 WPM with 95% accuracy on at least 100 graphemes"};
pub const LANGUAGES:Achievement=Achievement{id:"two-tongues",emoji:"🌎",title:"Two tongues",reason:"Complete English and Mexican Spanish benchmark sessions"};
pub const ACADEMY:Achievement=Achievement{id:"academy",emoji:"🎓",title:"Academy graduate",reason:"Master all five starter lessons with 100% final accuracy"};
pub const ALL:[Achievement;6]=[FIRST,PERSISTENT,PRECISE,FAST,LANGUAGES,ACADEMY];

fn benchmark(r:&Record)->bool {
    r.duration_ms>=15_000 && r.entered>=20 && r.elapsed_ms>=1_000
        && !r.test_mode.starts_with("lesson/") && r.test_mode!="custom"
}
fn language(r:&Record)->&'static str {
    if r.test_mode.split('/').any(|part|part=="es-MX") {"es-MX"} else {"en"}
}
pub fn earned(records:&[Record])->Vec<Achievement> {
    let mut standard=0_usize;
    let mut precise=false;
    let mut fast=false;
    let mut used=HashSet::new();
    let mut lessons=HashSet::new();
    for r in records {
        if crate::history::mastered_lesson(r) && r.scoring_version==SCORING_VERSION {
            if let Some(id)=r.test_mode.strip_prefix("lesson/") {
                lessons.insert(id);
            }
        }
        if !benchmark(r) {continue;}
        standard+=1;
        used.insert(language(r));
        if r.scoring_version==SCORING_VERSION {
            precise |= r.entered>=60 && r.accuracy>=98.0;
            fast |= r.entered>=100 && r.wpm>=80.0 && r.accuracy>=95.0;
        }
    }
    ALL.into_iter().filter(|a|match a.id {
        "first-flight"=>standard>=1,
        "steady-wings"=>standard>=10,
        "precision"=>precise,
        "fast-wings"=>fast,
        "two-tongues"=>used.len()>=2,
        "academy"=>["home","top","bottom","pairs","symbols"].into_iter().all(|id|lessons.contains(id)),
        _=>false,
    }).collect()
}
#[cfg(test)]
mod tests {
    use super::*;
    fn sample()->Record {
        Record {
            id:1,profile_id:"p".into(),finished_at_ms:1,layout:"us".into(),
            input_mode:"Emulated".into(),test_mode:"words".into(),duration_ms:30000,
            elapsed_ms:30000,entered:120,correct:119,wpm:85.0,accuracy:99.2,
            raw_wpm:90.0,raw_accuracy:95.0,attempts:125,mistakes:6,
            corrections:2,scoring_version:SCORING_VERSION,
        }
    }
    #[test] fn badges_are_recomputed_from_qualified_records() {
        let r=sample();
        let initial_awards=earned(&[r.clone()]);
        assert!(initial_awards.contains(&FIRST) && initial_awards.contains(&PRECISE) && initial_awards.contains(&FAST));
        assert!(!initial_awards.contains(&PERSISTENT) && !initial_awards.contains(&LANGUAGES));
        let mut more=vec![r;10];
        more[1].test_mode="words/es-MX/easy".into();
        let awards=earned(&more);
        assert!(awards.contains(&PERSISTENT) && awards.contains(&LANGUAGES));
    }
    #[test] fn tiny_custom_and_legacy_sessions_cannot_game_skill_awards() {
        let mut r=sample();
        r.test_mode="custom".into();
        assert!(earned(&[r.clone()]).is_empty());
        r.test_mode="words".into();
        r.duration_ms=2000;
        assert!(earned(&[r.clone()]).is_empty());
        r.duration_ms=30000;
        r.scoring_version=1;
        let awards=earned(&[r]);
        assert!(awards.contains(&FIRST));
        assert!(!awards.contains(&PRECISE) && !awards.contains(&FAST));
    }
    #[test] fn lesson_badge_requires_five_distinct_mastered_passages() {
        let mut data=Vec::new();
        for name in ["home","top","bottom","pairs","symbols"] {
            let mut item=sample();
            item.test_mode=format!("lesson/{name}");
            item.duration_ms=0;
            item.entered=50;
            item.correct=50;
            item.accuracy=100.0;
            data.push(item);
        }
        assert!(earned(&data).contains(&ACADEMY));
        data[4].correct=49;
        data[4].accuracy=98.0;
        assert!(!earned(&data).contains(&ACADEMY));
    }
}
