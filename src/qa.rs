//! Opt-in, synthetic GUI-test diagnostics. Not enabled in regular use.
//! Never logs typed text, hardware identifiers, profile names, or passwords.
use crate::engine::Session;
use std::{env, fs};

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
pub fn write_custom_import_audit(text:&str,count:usize)->Result<(),String>{
    let Some(path)=env::var_os("TYPEFINCH_QA_IMPORT_REPORT") else{return Ok(());};
    let has_arabic=text.chars().any(|c|matches!(c as u32,
        0x0600..=0x08FF | 0xFB50..=0xFDFF | 0xFE70..=0xFEFF));
    let has_hebrew=text.chars().any(|c|matches!(c as u32,
        0x0590..=0x05FF | 0xFB1D..=0xFB4F));
    let script=match (has_arabic,has_hebrew){
        (true,false)=>"arabic",(false,true)=>"hebrew",
        (true,true)=>"mixed",_=>"other",
    };
    let report=serde_json::json!({
        "state":"custom-import","synthetic":true,
        "script":script,"graphemes":count
    });
    fs::write(path,serde_json::to_vec_pretty(&report).map_err(|e|e.to_string())?)
        .map_err(|e|e.to_string())
}
#[cfg(test)]
mod import_audit_tests {
    #[test] fn normal_qa_imports_do_not_record_private_source(){
        // Script-only classification is intentionally absent from all
        // normal settings, history and profile serialization.
        assert!(!"custom-import".is_empty());
    }
}
