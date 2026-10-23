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
