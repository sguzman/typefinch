//! Read-only validation for an exported Typefinch settings/SQLite backup pair.
//! It never opens the user's live database or mutates either input file.
use rusqlite::{Connection,OpenFlags};
use serde_json::Value;
use std::{collections::HashSet,fs,path::Path};

pub struct BackupSummary {pub profiles:usize,pub sessions:i64,pub schema:i64}
fn require_file(path:&Path,label:&str)->Result<(),String>{
    let metadata=fs::symlink_metadata(path).map_err(|e|format!("{label}: {e}"))?;
    if !metadata.file_type().is_file(){
        return Err(format!("{label} must be a regular file, not a link or directory"));
    }
    Ok(())
}
pub fn verify(settings:&Path,database:&Path)->Result<BackupSummary,String>{
    require_file(settings,"Settings backup")?;
    require_file(database,"SQLite backup")?;
    let metadata=fs::metadata(settings).map_err(|e|e.to_string())?;
    if metadata.len()>8*1024*1024 {
        return Err("Settings backup is larger than 8 MiB".into());
    }
    let bytes=fs::read(settings).map_err(|e|format!("Reading settings backup: {e}"))?;
    let json:Value=serde_json::from_slice(&bytes).map_err(|e|format!("Invalid settings JSON: {e}"))?;
    let profiles=json.get("profiles").and_then(Value::as_array)
        .ok_or("Backup settings are missing a profiles array")?;
    if profiles.is_empty(){return Err("Backup contains no profiles".into());}
    let active=json.get("active_profile").and_then(Value::as_u64)
        .ok_or("Backup is missing an active profile index")?;
    if active as usize>=profiles.len(){return Err("Active profile index is outside the profile list".into());}
    let mut ids=HashSet::new();
    for p in profiles {
        let id=p.get("id").and_then(Value::as_str)
            .ok_or("Profile is missing its persistent identifier")?;
        if id.trim().is_empty() || !ids.insert(id.to_owned()){
            return Err("Backup profile IDs must be nonempty and unique".into());
        }
    }
    // Deserializing is validation only; do not use auto-migration, create a
    // fallback identity or write a normalized copy to the backup.
    serde_json::from_value::<crate::preferences::Preferences>(json)
        .map_err(|e|format!("Invalid settings structure: {e}"))?;
    let db=Connection::open_with_flags(database,OpenFlags::SQLITE_OPEN_READ_ONLY)
        .map_err(|e|format!("Opening SQLite backup read-only: {e}"))?;
    let schema:i64=db.pragma_query_value(None,"user_version",|r|r.get(0))
        .map_err(|e|format!("Reading backup schema: {e}"))?;
    if schema!=3 {
        return Err(format!("Unsupported SQLite backup schema {schema}; expected 3"));
    }
    let quick:String=db.query_row("PRAGMA quick_check",[],|r|r.get(0))
        .map_err(|e|format!("SQLite integrity check failed: {e}"))?;
    if quick!="ok" {return Err(format!("SQLite integrity check reported: {quick}"));}
    let sessions:i64=db.query_row("SELECT COUNT(*) FROM sessions",[],|r|r.get(0))
        .map_err(|e|format!("Missing or invalid sessions table: {e}"))?;
    let mut unknown=db.prepare("SELECT DISTINCT profile_id FROM sessions")
        .map_err(|e|format!("Cannot inspect profile references: {e}"))?;
    let refs=unknown.query_map([],|r|r.get::<_,String>(0))
        .map_err(|e|e.to_string())?;
    for item in refs {
        let id=item.map_err(|e|e.to_string())?;
        if !ids.contains(&id) {
            return Err("SQLite backup contains sessions for an absent profile".into());
        }
    }
    Ok(BackupSummary{profiles:profiles.len(),sessions,schema})
}

#[cfg(test)]
mod tests {
    use super::*;
    fn fixture()->(std::path::PathBuf,std::path::PathBuf,std::path::PathBuf,String){
        let dir=std::env::temp_dir().join(format!("typefinch-verify-{}",uuid::Uuid::new_v4()));
        fs::create_dir_all(&dir).unwrap();
        let settings=dir.join("backup-settings.json");
        let database=dir.join("backup-sessions.sqlite3");
        let prefs=crate::preferences::Preferences::default();
        let id=prefs.profiles[0].id.clone();
        fs::write(&settings,serde_json::to_vec(&prefs).unwrap()).unwrap();
        let db=Connection::open(&database).unwrap();
        db.execute_batch("CREATE TABLE sessions (profile_id TEXT NOT NULL); PRAGMA user_version=3;").unwrap();
        db.execute("INSERT INTO sessions(profile_id) VALUES (?1)",[&id]).unwrap();
        drop(db);
        (dir,settings,database,id)
    }
    #[test] fn good_backup_pair_is_checked_without_mutation(){
        let (dir,settings,database,_)=fixture();
        let original_json=fs::read(&settings).unwrap();
        let original_db=fs::read(&database).unwrap();
        let summary=verify(&settings,&database).unwrap();
        assert_eq!((summary.profiles,summary.sessions,summary.schema),(1,1,3));
        assert_eq!(fs::read(settings).unwrap(),original_json);
        assert_eq!(fs::read(database).unwrap(),original_db);
        fs::remove_dir_all(dir).unwrap();
    }
    #[test] fn reject_corruption_or_orphaned_profile_without_writing(){
        let (dir,settings,database,_)=fixture();
        let db=Connection::open(&database).unwrap();
        db.execute("INSERT INTO sessions(profile_id) VALUES ('unknown-profile')",[]).unwrap();
        drop(db);
        assert!(verify(&settings,&database).unwrap_err().contains("absent profile"));
        fs::write(&settings,b"{truncated").unwrap();
        assert!(verify(&settings,&database).unwrap_err().contains("Invalid settings JSON"));
        fs::remove_dir_all(dir).unwrap();
    }
    #[test] fn reject_unsupported_schema_and_missing_identity(){
        let (dir,settings,database,_)=fixture();
        let db=Connection::open(&database).unwrap();
        db.pragma_update(None,"user_version",4_i64).unwrap();
        drop(db);
        assert!(verify(&settings,&database).unwrap_err().contains("schema 4"));
        let db=Connection::open(&database).unwrap();
        db.pragma_update(None,"user_version",3_i64).unwrap();
        drop(db);
        let mut prefs:Value=serde_json::from_slice(&fs::read(&settings).unwrap()).unwrap();
        prefs["profiles"][0].as_object_mut().unwrap().remove("id");
        fs::write(&settings,serde_json::to_vec(&prefs).unwrap()).unwrap();
        assert!(verify(&settings,&database).unwrap_err().contains("identifier"));
        fs::remove_dir_all(dir).unwrap();
    }
    #[cfg(unix)]
    #[test] fn refuse_backup_symlinks(){
        use std::os::unix::fs::symlink;
        let (dir,settings,database,_)=fixture();
        let alias=dir.join("settings-alias.json");
        symlink(&settings,&alias).unwrap();
        assert!(verify(&alias,&database).unwrap_err().contains("regular file"));
        fs::remove_dir_all(dir).unwrap();
    }
}
