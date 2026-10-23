//! Safe, explicit per-profile goal transfer. Importing goals must never modify
//! profile identity, test counts, best scores or the SQLite session database.
use crate::preferences::{base_dir,Goals};
use serde::{Deserialize,Serialize};
use std::{fs,io::{Read,Write},path::{Path,PathBuf}};
const LIMIT:usize=8192;
const FORMAT:&str="typefinch-profile-goals";
#[derive(Serialize,Deserialize)]
struct GoalSnapshot {format:String,version:u32,goals:Goals}

fn validate(goals:&Goals)->Result<(),String> {
    if !goals.target_wpm.is_finite() || !(10.0..=250.0).contains(&goals.target_wpm) {
        return Err("Goal WPM target must be between 10 and 250".into());
    }
    if !goals.target_accuracy.is_finite() || !(70.0..=100.0).contains(&goals.target_accuracy) {
        return Err("Goal accuracy target must be between 70 and 100".into());
    }
    if !(1..=10).contains(&goals.daily_sessions) {
        return Err("Daily target must be between 1 and 10 sessions".into());
    }
    Ok(())
}
pub fn encode(goals:&Goals)->Result<Vec<u8>,String> {
    validate(goals)?;
    serde_json::to_vec_pretty(&GoalSnapshot{
        format:FORMAT.into(),version:1,goals:goals.clone(),
    }).map_err(|e|e.to_string())
}
pub fn parse(bytes:&[u8])->Result<Goals,String> {
    if bytes.len()>LIMIT {return Err("Goal JSON must be at most 8 KiB".into());}
    let parsed:GoalSnapshot=serde_json::from_slice(bytes).map_err(|e|e.to_string())?;
    if parsed.format!=FORMAT || parsed.version!=1 {
        return Err("Unsupported Typefinch goal transfer format or version".into());
    }
    validate(&parsed.goals)?;
    Ok(parsed.goals)
}
pub fn import(path:&Path)->Result<Goals,String> {
    let metadata=fs::symlink_metadata(path).map_err(|e|e.to_string())?;
    if !metadata.file_type().is_file() {
        return Err("Select a regular goal JSON file, not a symbolic link or device".into());
    }
    if metadata.len()>LIMIT as u64 {return Err("Goal JSON must be at most 8 KiB".into());}
    let mut bytes=Vec::new();
    fs::File::open(path).map_err(|e|e.to_string())?.take((LIMIT+1) as u64)
        .read_to_end(&mut bytes).map_err(|e|e.to_string())?;
    parse(&bytes)
}
pub fn export(goals:&Goals)->Result<PathBuf,String> {
    let dir=base_dir().join("exports");
    fs::create_dir_all(&dir).map_err(|e|e.to_string())?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&dir,fs::Permissions::from_mode(0o700))
            .map_err(|e|e.to_string())?;
    }
    let final_path=dir.join("typefinch-goals.json");
    let temp=dir.join(format!("goals-{}.tmp",uuid::Uuid::new_v4()));
    let bytes=encode(goals)?;
    let mut options=fs::OpenOptions::new();
    options.create_new(true).write(true);
    #[cfg(unix)] {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let mut file=options.open(&temp).map_err(|e|e.to_string())?;
    let outcome=(|| -> Result<(),String> {
        file.write_all(&bytes).map_err(|e|e.to_string())?;
        file.sync_all().map_err(|e|e.to_string())?;
        fs::rename(&temp,&final_path).map_err(|e|e.to_string())?;
        Ok(())
    })();
    if outcome.is_err() {let _=fs::remove_file(&temp);}
    outcome.map(|_|final_path)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn complete_goal_roundtrip_is_idempotent_and_profile_independent() {
        let goals=Goals{target_wpm:105.0,target_accuracy:99.0,daily_sessions:4,show_streak:true};
        let bytes=encode(&goals).unwrap();
        assert_eq!(parse(&bytes).unwrap(),goals);
        let serialized=String::from_utf8(bytes).unwrap();
        assert!(!serialized.contains("profile_id") && !serialized.contains("best_wpm"));
        assert!(serialized.contains(FORMAT));
    }
    #[test] fn invalid_transfer_version_targets_and_size_are_rejected() {
        let correct=Goals::default();
        let base=String::from_utf8(encode(&correct).unwrap()).unwrap();
        assert!(parse(base.replace("typefinch-profile-goals","another-format").as_bytes()).is_err());
        assert!(parse(base.replace("\"version\": 1","\"version\": 2").as_bytes()).is_err());
        assert!(parse(base.replace("60.0","500.0").as_bytes()).is_err());
        assert!(parse(&vec![b' ';LIMIT+1]).is_err());
    }
    #[test] fn file_import_rejects_non_regular_sources_and_reads_exact_format() {
        let path=std::env::temp_dir().join(format!("typefinch-goals-test-{}.json",uuid::Uuid::new_v4()));
        fs::write(&path,encode(&Goals::default()).unwrap()).unwrap();
        assert_eq!(import(&path).unwrap(),Goals::default());
        fs::write(&path,vec![b' ';LIMIT+1]).unwrap();
        assert!(import(&path).is_err());
        fs::remove_file(&path).unwrap();
        assert!(import(&path).is_err());
        assert!(import(&std::env::temp_dir()).is_err());
    }
}
