//! Local user-controlled configuration; no online identity.
use serde::{Deserialize, Serialize};
use crate::{engine::{TestMode,Language,WordDifficulty},custom_layout::CustomLayout};
use std::{fs, path::PathBuf};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all="lowercase")]
pub enum Theme { #[default] System, Light, Dark }
#[derive(Clone,Copy,Debug,Default,PartialEq,Eq,Serialize,Deserialize)]
#[serde(rename_all="lowercase")]
pub enum CardTheme {#[default] Dusk, Daybreak, Fern}
impl CardTheme {
    pub const ALL:[Self;3]=[Self::Dusk,Self::Daybreak,Self::Fern];
    pub fn label(self)->&'static str {
        match self {Self::Dusk=>"🌙 Dusk",Self::Daybreak=>"☀ Daybreak",Self::Fern=>"🌿 Fern"}
    }
}
#[derive(Clone,Copy,Debug,Default,PartialEq,Eq,Serialize,Deserialize)]
#[serde(rename_all="lowercase")]
pub enum CardTemplate {#[default] Classic, Compact}
impl CardTemplate {
    pub const ALL:[Self;2]=[Self::Classic,Self::Compact];
    pub fn label(self)->&'static str {match self {Self::Classic=>"Classic · 550×205",Self::Compact=>"Compact · 550×145"}}
}
#[derive(Clone,Copy,Debug,Default,PartialEq,Eq,Serialize,Deserialize)]
#[serde(rename_all="lowercase")]
pub enum CardFont {#[default] Sans, Mono}
impl CardFont {
    pub const ALL:[Self;2]=[Self::Sans,Self::Mono];
    pub fn label(self)->&'static str {match self {Self::Sans=>"Sans-serif",Self::Mono=>"Monospace"}}
    pub fn svg_family(self)->&'static str {match self {Self::Sans=>"sans-serif",Self::Mono=>"monospace"}}
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Goals {
    pub target_wpm: f64,
    pub target_accuracy: f64,
    #[serde(default = "default_daily_target")]
    pub daily_sessions:u32,
    #[serde(default)]
    pub show_streak:bool,
}
fn default_daily_target()->u32 {3}
impl Default for Goals {
    fn default() -> Self { Self { target_wpm:60.0, target_accuracy:95.0, daily_sessions:default_daily_target(), show_streak:false } }
}
impl Goals {
    pub fn wpm_progress(&self, best:f64)->f32 {
        (best.max(0.0)/self.target_wpm.max(1.0)).clamp(0.0,1.0) as f32
    }
    pub fn accuracy_progress(&self, best:f64)->f32 {
        (best.max(0.0)/self.target_accuracy.max(1.0)).clamp(0.0,1.0) as f32
    }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Profile {
    #[serde(default = "new_profile_id")]
    pub id: String,
    pub name: String,
    pub tests: u32,
    pub best_wpm: f64,
    pub best_accuracy: f64,
    #[serde(default)]
    pub goals: Goals,
}
impl Profile {
    /// Call only after the corresponding session was successfully committed
    /// to local SQLite. Private exercises cannot set benchmark personal bests.
    pub fn record_saved_score(&mut self,mode:TestMode,score:&crate::engine::Score) {
        self.tests=self.tests.saturating_add(1);
        if mode.counts_for_benchmark() {
            self.best_wpm=self.best_wpm.max(score.wpm);
            self.best_accuracy=self.best_accuracy.max(score.accuracy);
        }
    }
}
fn new_profile_id() -> String { uuid::Uuid::new_v4().to_string() }
impl Default for Profile {
    fn default() -> Self { Self { id:new_profile_id(), name:"Finch".into(), tests:0, best_wpm:0.0, best_accuracy:0.0, goals:Goals::default() } }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct Preferences {
    pub theme: Theme,
    pub reduce_motion:bool,
    pub card_theme:CardTheme,
    pub card_font:CardFont,
    pub card_template:CardTemplate,
    pub test_mode: TestMode,
    pub language: Language,
    pub word_difficulty: WordDifficulty,
    pub lesson_id: String,
    pub layout: String,
    pub custom_layouts:Vec<CustomLayout>,
    pub profiles: Vec<Profile>,
    pub active_profile: usize,
    pub duration_seconds: u64,
}
impl Default for Preferences {
    fn default() -> Self { Self { theme:Theme::System, reduce_motion:false, card_theme:CardTheme::Dusk, card_font:CardFont::Sans, card_template:CardTemplate::Classic, test_mode:TestMode::Words, language:Language::English, word_difficulty:WordDifficulty::Standard, lesson_id:"home".into(), layout:"us".into(), custom_layouts:Vec::new(), profiles:vec![Profile::default()], active_profile:0, duration_seconds:30 } }
}
pub fn base_dir() -> PathBuf {
    std::env::var_os("XDG_CONFIG_HOME").map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|s| PathBuf::from(s).join(".config")))
        .unwrap_or_else(|| PathBuf::from("."))
        .join("typefinch")
}
pub struct SettingsLoad {
    pub prefs:Preferences,
    /// True means the existing source must not be overwritten by a default.
    pub read_only:bool,
    pub warning:Option<String>,
}
impl Preferences {
    fn parse_and_migrate(bytes: &[u8]) -> Option<(Self,bool)> {
        let data: serde_json::Value = serde_json::from_slice(bytes).ok()?;
        // Existing identity-bearing settings cannot be recreated implicitly
        // from serde defaults. '{}' must not generate a fresh UUID and orphan
        // a real profile's SQLite sessions.
        if !data.get("profiles").is_some_and(serde_json::Value::is_array)
            || data.get("active_profile").is_none() {
            return None;
        }
        let mut p: Self = serde_json::from_value(data.clone()).ok()?;
        if p.profiles.is_empty() || p.active_profile >= p.profiles.len() { return None; }
        let originally_missing = data.get("profiles").and_then(|v|v.as_array())
            .is_some_and(|arr|arr.iter().any(|value|value.get("id").and_then(|s|s.as_str()).is_none()));
        let mut changed = originally_missing;
        let mut ids=std::collections::HashSet::new();
        for profile in &mut p.profiles {
            if profile.id.trim().is_empty() || !ids.insert(profile.id.clone()) {
                profile.id = new_profile_id();
                ids.insert(profile.id.clone());
                changed = true;
            }
        }
        Some((p, changed))
    }
    pub fn load_guarded()->SettingsLoad {
        Self::load_at(&base_dir().join("settings.json"))
    }
    fn load_at(path:&std::path::Path)->SettingsLoad {
        let unsafe_to_write=|warning:&str|SettingsLoad{
            prefs:Self::default(),read_only:true,warning:Some(warning.to_owned())
        };
        match fs::symlink_metadata(path) {
            Err(e) if e.kind()==std::io::ErrorKind::NotFound=>{
                // An existing history database without its identity-bearing
                // settings is NOT a first run. Never replace the missing
                // identity with a fresh UUID and orphan all saved sessions.
                let database=path.parent().unwrap_or(std::path::Path::new("."))
                    .join("sessions.sqlite3");
                match fs::symlink_metadata(&database) {
                    Ok(_)=>return unsafe_to_write(
                        "Settings are missing but an existing sessions database was found. Nothing was replaced; restore the matching settings backup before recording."),
                    Err(ref e) if e.kind()==std::io::ErrorKind::NotFound=>{},
                    Err(_)=>return unsafe_to_write(
                        "Settings are missing and the existing sessions database cannot be inspected. No identity or history is replaced."),
                }
                return SettingsLoad{prefs:Self::default(),read_only:false,warning:None};
            },
            Err(_)=>return unsafe_to_write(
                "Could not inspect the settings path. No data is replaced; settings and session recording are disabled."),
            Ok(metadata) if !metadata.file_type().is_file()=>
                return unsafe_to_write(
                    "Existing settings path is not a regular file (or is a symbolic link). The original is preserved; settings and session recording are disabled."),
            Ok(_)=>{},
        }
        match fs::read(path) {
            Err(_)=>unsafe_to_write(
                "Existing settings could not be read. The original is unchanged; settings and session recording are disabled to protect it."),
            Ok(bytes)=>match Self::parse_and_migrate(&bytes) {
                None=>unsafe_to_write(
                    "Existing settings could not be decoded or validated. The original file is untouched; settings and session recording are disabled until it is repaired or restored from backup."),
                Some((settings,needs_write))=>{
                    // Persist new identifiers BEFORE any sessions use those IDs.
                    if needs_write {
                        let dir=path.parent().unwrap_or(std::path::Path::new("."));
                        if settings.save_to_dir(dir).is_err() {
                            return SettingsLoad {
                                prefs:settings,read_only:true,
                                warning:Some("Profile identifier migration could not be saved. Existing settings are preserved; settings and session recording are disabled.".into()),
                            };
                        }
                    }
                    SettingsLoad{prefs:settings,read_only:false,warning:None}
                }
            }
        }
    }
    pub fn save(&self) -> Result<(), String> {
        self.save_to_dir(&base_dir())
    }
    /// A private, atomic same-directory replacement. This also upgrades the
    /// permissions of older installs whose JSON files used the system umask.
    fn save_to_dir(&self,dir:&std::path::Path)->Result<(),String> {
        use std::io::Write;
        fs::create_dir_all(dir).map_err(|e|e.to_string())?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(dir,fs::Permissions::from_mode(0o700))
                .map_err(|e|format!("Cannot restrict settings directory: {e}"))?;
        }
        let bytes=serde_json::to_vec_pretty(self).map_err(|e|e.to_string())?;
        let temp=dir.join(format!("settings-{}.tmp",uuid::Uuid::new_v4()));
        let mut options=fs::OpenOptions::new();
        options.create_new(true).write(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }
        let mut file=options.open(&temp).map_err(|e|e.to_string())?;
        let result=(||->Result<(),String>{
            file.write_all(&bytes).map_err(|e|e.to_string())?;
            file.sync_all().map_err(|e|e.to_string())?;
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                fs::set_permissions(&temp,fs::Permissions::from_mode(0o600))
                    .map_err(|e|e.to_string())?;
            }
            fs::rename(&temp,dir.join("settings.json")).map_err(|e|e.to_string())?;
            Ok(())
        })();
        if result.is_err() {let _=fs::remove_file(&temp);}
        result
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn corrupt_existing_settings_never_get_silently_replaced() {
        let dir=std::env::temp_dir().join(format!("typefinch-corrupt-settings-{}",uuid::Uuid::new_v4()));
        fs::create_dir_all(&dir).unwrap();
        let settings=dir.join("settings.json");
        let corrupt=b"{broken json \xff and truncated";
        fs::write(&settings,corrupt).unwrap();
        let result=Preferences::load_at(&settings);
        assert!(result.read_only);
        assert!(result.warning.as_ref().is_some_and(|m|m.contains("unchanged")||m.contains("untouched")));
        assert_eq!(fs::read(&settings).unwrap(),corrupt);
        assert_eq!(fs::read_dir(&dir).unwrap().count(),1);
        fs::remove_dir_all(&dir).unwrap();
    }
    #[cfg(unix)]
    #[test] fn dangling_settings_symlink_must_not_count_as_first_run() {
        use std::os::unix::fs::symlink;
        let dir=std::env::temp_dir().join(format!("typefinch-symlink-{}",uuid::Uuid::new_v4()));
        fs::create_dir_all(&dir).unwrap();
        let link=dir.join("settings.json");
        symlink("missing-target", &link).unwrap();
        let result=Preferences::load_at(&link);
        assert!(result.read_only);
        assert!(result.warning.unwrap().contains("symbolic link"));
        assert!(fs::symlink_metadata(&link).unwrap().file_type().is_symlink());
        fs::remove_dir_all(dir).unwrap();
    }
    #[test] fn existing_settings_with_missing_identity_are_not_new_first_runs() {
        for bad in [b"{}".as_slice(),b"{\"theme\":\"dark\"}".as_slice(),
            b"{\"profiles\":[{\"name\":\"Finch\",\"tests\":1,\"best_wpm\":15.0,\"best_accuracy\":85.0}]}".as_slice()] {
            assert!(Preferences::parse_and_migrate(bad).is_none());
        }
    }
    #[test] fn missing_settings_with_existing_database_must_enter_recovery_mode() {
        let dir=std::env::temp_dir().join(format!("typefinch-orphan-history-{}",uuid::Uuid::new_v4()));
        fs::create_dir_all(&dir).unwrap();
        let settings=dir.join("settings.json");
        let database=dir.join("sessions.sqlite3");
        let sentinel=b"existing history must never be touched";
        fs::write(&database,sentinel).unwrap();
        let loaded=Preferences::load_at(&settings);
        assert!(loaded.read_only);
        assert!(loaded.warning.unwrap().contains("existing sessions database"));
        assert!(!settings.exists());
        assert_eq!(fs::read(&database).unwrap(),sentinel);
        fs::remove_dir_all(dir).unwrap();
    }
    #[test] fn first_run_is_writable_but_empty_existing_profiles_are_guarded() {
        let dir=std::env::temp_dir().join(format!("typefinch-guard-{}",uuid::Uuid::new_v4()));
        fs::create_dir_all(&dir).unwrap();
        let settings=dir.join("settings.json");
        let first=Preferences::load_at(&settings);
        assert!(!first.read_only && first.warning.is_none());
        fs::write(&settings,br#"{"profiles":[]}"#).unwrap();
        let bad=Preferences::load_at(&settings);
        assert!(bad.read_only);
        assert_eq!(fs::read(&settings).unwrap(),br#"{"profiles":[]}"#);
        fs::remove_dir_all(&dir).unwrap();
    }
    #[test] fn atomic_settings_save_is_private_and_leaves_no_temp_files() {
        let dir=std::env::temp_dir().join(format!("typefinch-prefs-{}",uuid::Uuid::new_v4()));
        let mut prefs=Preferences::default();
        prefs.profiles[0].name="Local account".into();
        prefs.save_to_dir(&dir).unwrap();
        prefs.profiles[0].name="Updated local account".into();
        prefs.save_to_dir(&dir).unwrap();
        let files=fs::read_dir(&dir).unwrap()
            .map(|entry|entry.unwrap().file_name().to_string_lossy().into_owned())
            .collect::<Vec<_>>();
        assert_eq!(files,vec!["settings.json"]);
        let bytes=fs::read(dir.join("settings.json")).unwrap();
        let parsed:Preferences=serde_json::from_slice(&bytes).unwrap();
        assert_eq!(parsed.profiles[0].name,"Updated local account");
        #[cfg(unix)] {
            use std::os::unix::fs::PermissionsExt;
            assert_eq!(fs::metadata(&dir).unwrap().permissions().mode()&0o777,0o700);
            assert_eq!(fs::metadata(dir.join("settings.json")).unwrap().permissions().mode()&0o777,0o600);
        }
        fs::remove_dir_all(dir).unwrap();
    }
    #[test] fn saved_practice_sessions_never_inflate_benchmark_bests() {
        let mut profile=Profile::default();
        let clean=crate::engine::score("abcdefghij","abcdefghij",
            std::time::Duration::from_secs(30));
        profile.record_saved_score(TestMode::Lesson,&clean);
        profile.record_saved_score(TestMode::Custom,&clean);
        assert_eq!(profile.tests,2);
        assert_eq!(profile.best_wpm,0.0);
        profile.record_saved_score(TestMode::Words,&clean);
        assert_eq!(profile.tests,3);
        assert!(profile.best_wpm>0.0);
        assert_eq!(profile.best_accuracy,100.0);
    }
    #[test] fn legacy_profile_ids_are_migrated_and_stable_when_serialized() {
        let old = br#"{"theme":"system","layout":"us","profiles":[
            {"name":"Old","tests":3,"best_wpm":60.0,"best_accuracy":95.0}
        ],"active_profile":0,"duration_seconds":30}"#;
        let (p,migrated)=Preferences::parse_and_migrate(old).unwrap();
        assert!(migrated);
        let stable=p.profiles[0].id.clone();
        assert!(!stable.is_empty());
        let bytes=serde_json::to_vec(&p).unwrap();
        let (again,migrated_again)=Preferences::parse_and_migrate(&bytes).unwrap();
        assert!(!migrated_again);
        assert_eq!(again.profiles[0].id,stable);
    }
    #[test] fn duplicate_and_empty_ids_are_repaired_without_changing_valid_ids() {
        let mut p=Preferences::default();
        let known=p.profiles[0].id.clone();
        p.profiles.push(Profile { id:known.clone(),name:"Duplicate".into(),..Default::default() });
        p.profiles.push(Profile { id:String::new(),name:"Blank".into(),..Default::default() });
        let raw=serde_json::to_vec(&p).unwrap();
        let (repaired,migrated)=Preferences::parse_and_migrate(&raw).unwrap();
        assert!(migrated);
        assert_eq!(repaired.profiles[0].id,known);
        assert_ne!(repaired.profiles[1].id, known);
        assert!(!repaired.profiles[2].id.is_empty());
        assert_ne!(repaired.profiles[1].id,repaired.profiles[2].id);
    }
    #[test] fn independent_goals_survive_profile_serialization() {
        let mut p=Preferences::default();
        let other=Profile { name:"Other".into(), ..Default::default() };
        p.profiles.push(other);
        p.profiles[1].goals.target_wpm=110.0;
        let bytes=serde_json::to_vec(&p).unwrap();
        let loaded:Preferences=serde_json::from_slice(&bytes).unwrap();
        assert_eq!(loaded.profiles[0].goals.target_wpm,60.0);
        assert_eq!(loaded.profiles[1].goals.target_wpm,110.0);
        assert_eq!(loaded.profiles[1].goals.target_accuracy,95.0);
        assert_eq!(loaded.profiles[1].goals.daily_sessions,3);
        assert!(!loaded.profiles[1].goals.show_streak);
        assert_eq!(loaded.profiles[0].goals.wpm_progress(30.0),0.5);
        assert_eq!(loaded.profiles[1].goals.wpm_progress(120.0),1.0);
    }
    #[test] fn legacy_profile_receives_id() {
        let p: Profile = serde_json::from_str(r#"{"name":"Old","tests":3,"best_wpm":60.0,"best_accuracy":95.0}"#).unwrap();
        assert!(!p.id.is_empty());
        assert_eq!(p.name, "Old");
        assert_eq!(p.goals,Goals::default());
    }
    #[test] fn old_settings_default_to_standard_word_difficulty() {
        let mut data=serde_json::to_value(Preferences::default()).unwrap();
        data.as_object_mut().unwrap().remove("word_difficulty");
        let loaded:Preferences=serde_json::from_value(data).unwrap();
        assert_eq!(loaded.word_difficulty,WordDifficulty::Standard);
    }
    #[test] fn old_profile_goals_keep_a_safe_daily_default() {
        let old=r#"{"target_wpm":70,"target_accuracy":96}"#;
        let goals:Goals=serde_json::from_str(old).unwrap();
        assert_eq!(goals.daily_sessions,3);
        assert!(!goals.show_streak);
    }
    #[test] fn older_settings_default_to_animation_enabled() {
        let mut data=serde_json::to_value(Preferences::default()).unwrap();
        data.as_object_mut().unwrap().remove("reduce_motion");
        let old:Preferences=serde_json::from_value(data).unwrap();
        assert!(!old.reduce_motion);
    }
    #[test] fn older_settings_migrate_to_default_card_style() {
        let mut data=serde_json::to_value(Preferences::default()).unwrap();
        data.as_object_mut().unwrap().remove("card_theme");
        data.as_object_mut().unwrap().remove("card_font");
        data.as_object_mut().unwrap().remove("card_template");
        let older:Preferences=serde_json::from_value(data).unwrap();
        assert_eq!(older.card_theme,CardTheme::Dusk);
        assert_eq!(older.card_font,CardFont::Sans);
        assert_eq!(older.card_template,CardTemplate::Classic);
    }
    #[test] fn personal_layouts_survive_settings_roundtrip_and_default_empty() {
        let mut p=Preferences::default();
        assert!(p.custom_layouts.is_empty());
        let custom=CustomLayout::starter();
        p.layout=custom.id.clone();
        p.custom_layouts.push(custom.clone());
        let encoded=serde_json::to_vec(&p).unwrap();
        let q:Preferences=serde_json::from_slice(&encoded).unwrap();
        assert_eq!(q.custom_layouts,vec![custom]);
        assert_eq!(q.layout,p.layout);
    }
    #[test] fn system_is_default() { assert_eq!(Preferences::default().theme, Theme::System); }
    #[test] fn settings_roundtrip() {
        let p=Preferences::default();
        let b=serde_json::to_vec(&p).unwrap();
        let q: Preferences=serde_json::from_slice(&b).unwrap();
        assert_eq!(q.layout, "us");
        assert_eq!(q.card_theme,CardTheme::Dusk);
        assert_eq!(q.card_font,CardFont::Sans);
        assert_eq!(q.card_template,CardTemplate::Classic);
        assert_eq!(q.test_mode, TestMode::Words);
        assert_eq!(q.language, Language::English);
        assert_eq!(q.word_difficulty, WordDifficulty::Standard);
        assert_eq!(q.lesson_id, "home");
    }
}
