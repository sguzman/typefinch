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
    export_to_dir(goals,&base_dir().join("exports"))
}
fn export_to_dir(goals:&Goals,dir:&Path)->Result<PathBuf,String> {
    fs::create_dir_all(dir).map_err(|e|e.to_string())?;
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
    #[test] fn goal_import_can_update_only_selected_profile_targets() {
        use crate::preferences::Profile;
        let mut profiles=vec![Profile::default(),Profile::default()];
        profiles[0].name="First".into();
        profiles[1].name="Second".into();
        profiles[1].tests=37;
        profiles[1].best_wpm=132.0;
        profiles[1].best_accuracy=99.0;
        let first=profiles[0].clone();
        let second_id=profiles[1].id.clone();
        let transferred=Goals{
            target_wpm:100.0,target_accuracy:98.0,
            daily_sessions:4,show_streak:false,
        };
        // The same parse result that the UI assigns is only a Goals value;
        // no profile identity, session records or best scores are transferred.
        let decoded=parse(&encode(&transferred).unwrap()).unwrap();
        profiles[1].goals=decoded;
        assert_eq!(profiles[0].id,first.id);
        assert_eq!(profiles[0].goals,first.goals);
        assert_eq!(profiles[1].id,second_id);
        assert_eq!(profiles[1].tests,37);
        assert_eq!(profiles[1].best_wpm,132.0);
        assert_eq!(profiles[1].best_accuracy,99.0);
        assert_eq!(profiles[1].goals,transferred);
    }
    #[test] fn goal_file_import_rejects_symbolic_links() {
        #[cfg(unix)] {
            use std::os::unix::fs::symlink;
            let dir=std::env::temp_dir().join(format!("typefinch-goal-symlink-{}",uuid::Uuid::new_v4()));
            fs::create_dir(&dir).unwrap();
            let source=dir.join("goals.json");
            let link=dir.join("external.json");
            fs::write(&source,encode(&Goals::default()).unwrap()).unwrap();
            symlink(&source,&link).unwrap();
            assert!(import(&link).unwrap_err().contains("regular goal JSON"));
            assert!(import(&source).is_ok());
            fs::remove_dir_all(dir).unwrap();
        }
    }
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
    #[test] fn exported_goals_are_private_atomic_and_reimportable() {
        let dir=std::env::temp_dir().join(format!("typefinch-goal-export-{}",uuid::Uuid::new_v4()));
        let goals=Goals{target_wpm:120.0,target_accuracy:99.0,daily_sessions:5,show_streak:true};
        let first=export_to_dir(&goals,&dir).unwrap();
        assert_eq!(import(&first).unwrap(),goals);
        let second=Goals{target_wpm:25.0,target_accuracy:85.0,daily_sessions:1,show_streak:false};
        assert_eq!(export_to_dir(&second,&dir).unwrap(),first);
        assert_eq!(import(&first).unwrap(),second);
        let files=fs::read_dir(&dir).unwrap().collect::<Vec<_>>();
        assert_eq!(files.len(),1,"Atomic temp files were left behind");
        #[cfg(unix)] {
            use std::os::unix::fs::PermissionsExt;
            assert_eq!(fs::metadata(&dir).unwrap().permissions().mode()&0o777,0o700);
            assert_eq!(fs::metadata(&first).unwrap().permissions().mode()&0o777,0o600);
        }
        fs::remove_dir_all(dir).unwrap();
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
