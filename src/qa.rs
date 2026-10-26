//! Opt-in, synthetic GUI-test diagnostics. Not enabled in regular use.
//! Never logs typed text, hardware identifiers, profile names, or passwords.
use crate::engine::Session;
use std::{env, fs};

/// This synthetic-only signal drives the native GUI pilot smoke fixture.
/// Never emit reference/practice text, profile IDs, devices, or a real study.
/// The two QA environment variables and fixed disposable source must all match.
pub fn write_synthetic_pilot_audit(held_out:&str,
    arm:crate::training::Arm,stage:crate::training::TrialStage)->Result<(),String> {
    if held_out!="aba" || env::var_os("TYPEFINCH_QA_REPORT").is_none() {return Ok(());}
    let Some(path)=env::var_os("TYPEFINCH_QA_PILOT_REPORT") else{return Ok(());};
    let arm=match arm {
        crate::training::Arm::Adaptive=>"adaptive",
        crate::training::Arm::NonAdaptive=>"non_adaptive",
    };
    let stage=match stage {
        crate::training::TrialStage::Practice=>"practice",
        crate::training::TrialStage::Immediate=>"immediate",
        crate::training::TrialStage::Delayed=>"delayed",
        crate::training::TrialStage::Complete=>"complete",
    };
    let bytes=serde_json::to_vec_pretty(&serde_json::json!({
        "synthetic":true,"source":"fixed-disposable-fixture",
        "arm":arm,"stage":stage,"causal_claim":false
    })).map_err(|e|e.to_string())?;
    fs::write(path,bytes).map_err(|e|e.to_string())
}
pub fn write_smoke_report(session: &Session, completed_tests: u32) -> Result<(), String> {
    let Some(path) = env::var_os("TYPEFINCH_QA_REPORT") else { return Ok(()); };
    let score = session.score();
    let report = serde_json::json!({
        "state":"finished",
        "elapsed_ms": score.elapsed.as_millis(),
        "entered": score.entered,
        "correct": score.correct,
        "accuracy": score.accuracy,
        "wpm": score.wpm,
        "tests": completed_tests,
        "synthetic": true
    });
    fs::write(path, serde_json::to_vec_pretty(&report).map_err(|e| e.to_string())?)
        .map_err(|e| e.to_string())
}

/// One scored, focused, app-targeted event for explicit local QA. This
/// reports only aggregate counts; never the key, text, layout address or
/// profile identity. No output exists outside opt-in disposable QA.
fn focused_input_payload(session:&Session,input_mode:&str)->serde_json::Value {
    serde_json::json!({
        "state":"focused-input",
        "synthetic":true,
        "input_mode":input_mode,
        "attempts":session.attempts,
        "entered":session.score().entered,
    })
}
/// On every later GUI frame with exactly one attempted key, leave the
/// first evidence immutable. create_new also rejects a preexisting symlink.
fn write_focused_input_once(path:&std::path::Path,bytes:&[u8])->Result<(),String>{
    use std::io::{ErrorKind,Write};
    let mut file=match fs::OpenOptions::new().write(true).create_new(true).open(path){
        Ok(file)=>file,
        Err(error) if error.kind()==ErrorKind::AlreadyExists=>return Ok(()),
        Err(error)=>return Err(error.to_string()),
    };
    file.write_all(bytes).map_err(|e|e.to_string())
}
pub fn write_focused_input_audit(session:&Session,input_mode:&str)->Result<(),String>{
    if env::var_os("TYPEFINCH_QA_REPORT").is_none()
        || session.state!=crate::engine::State::Running
        || session.attempts!=1 {return Ok(());}
    let Some(path)=env::var_os("TYPEFINCH_QA_INPUT_REPORT") else{return Ok(());};
    let data=focused_input_payload(session,input_mode);
    let bytes=serde_json::to_vec_pretty(&data).map_err(|e|e.to_string())?;
    write_focused_input_once(std::path::Path::new(&path),&bytes)
}
#[cfg(test)]
mod focused_input_tests {
    #[test] fn first_audit_is_immutable_and_preexisting_symlink_is_untouched(){
        use super::write_focused_input_once;
        let dir=std::env::temp_dir().join(format!("typefinch-focus-{}",uuid::Uuid::new_v4()));
        std::fs::create_dir(&dir).expect("isolated private QA fixture");
        let report=dir.join("report.json");
        write_focused_input_once(&report,b"first").unwrap();
        write_focused_input_once(&report,b"second").unwrap();
        assert_eq!(std::fs::read(&report).unwrap(),b"first");
        #[cfg(unix)]{
            use std::os::unix::fs::symlink;
            let target=dir.join("target.json");
            let link=dir.join("link.json");
            std::fs::write(&target,b"keep").unwrap();
            symlink(&target,&link).unwrap();
            write_focused_input_once(&link,b"change").unwrap();
            assert_eq!(std::fs::read(&target).unwrap(),b"keep");
            std::fs::remove_file(link).unwrap();
            std::fs::remove_file(target).unwrap();
        }
        std::fs::remove_file(report).unwrap();
        std::fs::remove_dir(dir).unwrap();
    }
    #[test] fn audit_contains_only_aggregates_not_private_source_or_keystroke(){
        use crate::engine::{Session,TestMode};
        let mut session=Session::new_with_custom(15,TestMode::Custom,"SECRET-QA-REFERENCE");
        session.begin();
        session.type_text("z");
        let info=super::focused_input_payload(&session,"OS");
        assert_eq!(info["state"],"focused-input");
        assert_eq!(info["attempts"],1);
        assert_eq!(info["entered"],1);
        assert_eq!(info["input_mode"],"OS");
        assert!(info.get("key").is_none());
        assert!(info.get("text").is_none());
        assert!(!info.to_string().contains("SECRET-QA-REFERENCE"));
    }
}

/// Test-only marker proving that a focused timed session actually reached the
/// persistence guard. No typed text or profile identities are included.
pub fn write_guarded_report()->Result<(),String> {
    let Some(path)=env::var_os("TYPEFINCH_QA_REPORT") else {return Ok(());};
    let report=serde_json::json!({"state":"blocked","synthetic":true});
    fs::write(path,serde_json::to_vec_pretty(&report).map_err(|e|e.to_string())?)
        .map_err(|e|e.to_string())
}

/// Marker for an actual foreground test canceled by losing window focus.
/// Only the explicit synthetic QA process can write this metadata-only file.
pub fn write_focus_lost_report()->Result<(),String> {
    let Some(path)=env::var_os("TYPEFINCH_QA_REPORT") else {return Ok(());};
    let data=serde_json::json!({"state":"canceled-focus","synthetic":true});
    fs::write(path,serde_json::to_vec_pretty(&data).map_err(|e|e.to_string())?)
        .map_err(|e|e.to_string())
}

/// Synthetic-only QA metadata for visual keyboard geometry snapshots.
pub fn write_geometry_report(
    name:&str, target:&str, section:&str, slots:&[crate::geometry::Slot]
)->Result<(),String>{
    let Some(path)=env::var_os("TYPEFINCH_QA_REPORT") else {return Ok(());};
    let report=serde_json::json!({
        "state":"geometry-preview",
        "synthetic":true,
        "shape":name,
        "target":target,
        "section":section,
        "slots":slots.len(),
        "unattributed":slots.iter().filter(|s|
            s.kind==crate::geometry::SlotKind::Extra).count(),
        "controls":slots.iter().filter(|s|
            s.kind==crate::geometry::SlotKind::Control).count(),
        "modifiers":slots.iter().filter(|s|
            s.kind==crate::geometry::SlotKind::Modifier).count(),
        "visual_only":slots.iter().filter(|s|s.evdev==0).count(),
    });
    fs::write(path,serde_json::to_vec_pretty(&report).map_err(|e|e.to_string())?)
        .map_err(|e|e.to_string())
}

/// Only a disposable QA fixture with this explicit environment variable
/// writes synthetic import evidence. Never persist the source text, file
/// path, typed characters, hardware IDs or a user profile identifier.
fn synthetic_script_class(text:&str)->&'static str {
    let has_arabic=text.chars().any(|c|matches!(c as u32,
        0x0600..=0x08FF | 0xFB50..=0xFDFF | 0xFE70..=0xFEFF));
    let has_hebrew=text.chars().any(|c|matches!(c as u32,
        0x0590..=0x05FF | 0xFB1D..=0xFB4F));
    match (has_arabic,has_hebrew){
        (true,false)=>"arabic",(false,true)=>"hebrew",
        (true,true)=>"mixed",_=>"other",
    }
}
pub fn write_custom_import_audit(text:&str,count:usize)->Result<(),String>{
    let Some(path)=env::var_os("TYPEFINCH_QA_IMPORT_REPORT") else{return Ok(());};
    let report=serde_json::json!({
        "state":"custom-import","synthetic":true,
        "script":synthetic_script_class(text),"graphemes":count
    });
    fs::write(path,serde_json::to_vec_pretty(&report).map_err(|e|e.to_string())?)
        .map_err(|e| e.to_string())
}
// The shaped editor republishes a numeric caret snapshot on many GUI
// frames. Truncate-and-write can expose empty or partial JSON to a real
// compositor test copying the file between frames. Publish a COMPLETE
// same-directory private file through atomic rename instead. Neither bytes
// nor source paths are logged, and this is only used for explicit QA.
fn write_atomic_editor_audit(path:&std::path::Path,bytes:&[u8])->Result<(),String>{
    use std::{io::Write,os::unix::fs::OpenOptionsExt,
        sync::atomic::{AtomicU64,Ordering}};
    static NEXT_REPORT:AtomicU64=AtomicU64::new(0);
    let mut name=path.file_name().ok_or("QA report path needs a file name")?
        .to_os_string();
    name.push(format!(".typefinch-{}-{}.tmp",std::process::id(),
        NEXT_REPORT.fetch_add(1,Ordering::Relaxed)));
    let stage=path.with_file_name(name);
    let mut file=fs::OpenOptions::new().write(true).create_new(true)
        .mode(0o600).open(&stage).map_err(|e|e.to_string())?;
    let result=(||->std::io::Result<()>{
        file.write_all(bytes)?;
        file.flush()?;
        drop(file);
        fs::rename(&stage,path)?;
        Ok(())
    })();
    if result.is_err(){let _=fs::remove_file(&stage);}
    result.map_err(|e|e.to_string())
}
#[cfg(test)]
mod atomic_editor_audit_tests {
    use super::*;
    #[test]
    fn repeated_caret_snapshots_never_expose_partial_json_to_concurrent_reader(){
        let dir=std::env::temp_dir().join(format!("typefinch-rtl-audit-{}",
            uuid::Uuid::new_v4()));
        fs::create_dir(&dir).unwrap();
        let path=dir.join("caret.json");
        let first:&[u8]=br#"{"state":"rtl-editor-caret","index":0}"#;
        let second:&[u8]=br#"{"state":"rtl-editor-caret","index":123,"reserved":"abcdefghijabcdefghijabcdefghijabcdefghij"}"#;
        write_atomic_editor_audit(&path,first).unwrap();
        let writer_path=path.clone();
        let writer=std::thread::spawn(move||{
            for i in 0..500 {
                write_atomic_editor_audit(&writer_path,
                    if i%2==0 {second} else {first}).unwrap();
                if i%4==0 {std::thread::yield_now();}
            }
        });
        for _ in 0..1500 {
            let data=fs::read(&path).unwrap();
            let object:serde_json::Value=serde_json::from_slice(&data)
                .expect("A live QA reader must never observe truncated JSON");
            assert_eq!(object["state"],"rtl-editor-caret");
            assert!(data.as_slice()==first || data.as_slice()==second);
            std::thread::yield_now();
        }
        writer.join().unwrap();
        // Rename must replace a symlink itself, never follow its target.
        let external=dir.join("external");
        fs::write(&external,b"protected").unwrap();
        std::os::unix::fs::symlink(&external,&path).unwrap_err();
        fs::remove_file(&path).unwrap();
        std::os::unix::fs::symlink(&external,&path).unwrap();
        write_atomic_editor_audit(&path,first).unwrap();
        assert_eq!(fs::read(&external).unwrap(),b"protected");
        assert_eq!(fs::read(&path).unwrap(),first);
        assert!(fs::symlink_metadata(&path).unwrap().file_type().is_file());
        assert_eq!(fs::read_dir(&dir).unwrap().count(),2,
            "Temporary QA stages must not accumulate");
        fs::remove_dir_all(&dir).unwrap();
    }
}

/// Synthetic, explicitly opted-in evidence of the *actual native editor*
/// caret being projected into the separate shaped preview. No source text,
/// profile, path, script, key identity or raw selection is recorded.
pub fn write_rtl_editor_caret_audit(index:usize,total:usize,
    first:usize,last:usize,caret:Option<(f32,f32)>,
    selected_graphemes:usize)->Result<(),String>{
    let Some(path)=env::var_os("TYPEFINCH_QA_EDITOR_CARET_REPORT") else{return Ok(());};
    let report=serde_json::json!({
        "state":"rtl-editor-caret","synthetic":true,
        "index":index,"total":total,"visible_start":first,
        "visible_end":last,
        "caret_positioned":caret.is_some(),
        // Synthetic-only rendered caret geometry, not source text or
        // selection contents. Check true cross-paragraph BiDi movement.
        "caret_y_px":caret.map(|(_,y)|y.round() as i32),
        "selected_graphemes":selected_graphemes,
    });
    write_atomic_editor_audit(std::path::Path::new(&path),
        &serde_json::to_vec_pretty(&report).map_err(|e|e.to_string())?)
}
#[cfg(test)]
mod import_audit_tests {
    #[test] fn fixture_labels_distinguish_arabic_hebrew_and_mixed(){
        use super::synthetic_script_class;
        assert_eq!(synthetic_script_class("سَلَام بالعربية 123"),"arabic");
        assert_eq!(synthetic_script_class("שָׁלוֹם בעברית 123"),"hebrew");
        assert_eq!(synthetic_script_class("שלום سلام"),"mixed");
        assert_eq!(synthetic_script_class("abc 123"),"other");
    }
}

/// Test-only observer for the inspector's actual native GUI lifecycle.
/// Writes no compositor names, keyboard identifiers, layouts, source text,
/// window titles, raw key events or profile metadata.
pub fn write_hotplug_audit(count:usize,selected:usize,fresh:bool,
    uniform_known_layout:bool)->Result<(),String>{
    if env::var_os("TYPEFINCH_QA_REPORT").is_none(){return Ok(());}
    let Some(path)=env::var_os("TYPEFINCH_QA_HOTPLUG_REPORT") else{return Ok(());};
    let report=serde_json::json!({
        "state":"keyboard-inventory","synthetic":true,
        "count":count,"selected_index":selected,
        "fresh":fresh,"layout_detected":uniform_known_layout,
        "inspection_only":true,
    });
    fs::write(path,serde_json::to_vec_pretty(&report).map_err(|e|e.to_string())?)
        .map_err(|e|e.to_string())
}


/// Opt-in synthetic GUI evidence for a real in-app drill and same-source
/// retest. Numeric aggregates only; never record a passage or identity.
pub fn write_training_followup_audit(c:crate::training::Comparison)->Result<(),String>{
    if env::var_os("TYPEFINCH_QA_REPORT").is_none(){return Ok(());}
    let Some(path)=env::var_os("TYPEFINCH_QA_TRAINING_REPORT") else{return Ok(());};
    let data=serde_json::json!({
        "state":"same-source-followup","synthetic":true,
        "before_accuracy":c.baseline_accuracy,
        "after_accuracy":c.retest_accuracy,
        "before_wpm":c.baseline_wpm,"after_wpm":c.retest_wpm,
        "before_attempts":c.baseline_attempts,
        "after_attempts":c.retest_attempts,
        "before_reference_observed":c.baseline_opportunities.observed,
        "before_reference_missed":c.baseline_opportunities.missed,
        "after_reference_observed":c.retest_opportunities.observed,
        "after_reference_missed":c.retest_opportunities.missed,
        "causal_claim":false
    });
    fs::write(path,serde_json::to_vec_pretty(&data).map_err(|e|e.to_string())?)
        .map_err(|e|e.to_string())
}

/// Synthetic-only indication that the actual native typing session started
/// and owns the expected Unicode reference length. No source text, keycodes,
/// device identity, typed events or file path is included in the payload.
pub fn write_test_started_audit(session:&Session)->Result<(),String>{
    if env::var_os("TYPEFINCH_QA_REPORT").is_none(){return Ok(());}
    let Some(path)=env::var_os("TYPEFINCH_QA_START_REPORT") else{return Ok(());};
    use unicode_segmentation::UnicodeSegmentation;
    let data=serde_json::json!({
        "state":"running","synthetic":true,
        "expected_graphemes":session.target.graphemes(true).count(),
    });
    fs::write(path,serde_json::to_vec_pretty(&data).map_err(|e|e.to_string())?)
        .map_err(|e|e.to_string())
}
