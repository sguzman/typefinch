//! Local user-controlled configuration; no online identity.
use serde::{Deserialize, Serialize};
use crate::engine::TestMode;
use std::{fs, path::PathBuf};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all="lowercase")]
pub enum Theme { #[default] System, Light, Dark }
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Profile {
    #[serde(default = "new_profile_id")]
    pub id: String,
    pub name: String,
    pub tests: u32,
    pub best_wpm: f64,
    pub best_accuracy: f64,
}
fn new_profile_id() -> String { uuid::Uuid::new_v4().to_string() }
impl Default for Profile {
    fn default() -> Self { Self { id:new_profile_id(), name:"Finch".into(), tests:0, best_wpm:0.0, best_accuracy:0.0 } }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct Preferences {
    pub theme: Theme,
    pub test_mode: TestMode,
    pub lesson_id: String,
    pub layout: String,
    pub profiles: Vec<Profile>,
    pub active_profile: usize,
    pub duration_seconds: u64,
}
impl Default for Preferences {
    fn default() -> Self { Self { theme:Theme::System, test_mode:TestMode::Words, lesson_id:"home".into(), layout:"us".into(), profiles:vec![Profile::default()], active_profile:0, duration_seconds:30 } }
}
pub fn base_dir() -> PathBuf {
    std::env::var_os("XDG_CONFIG_HOME").map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|s| PathBuf::from(s).join(".config")))
        .unwrap_or_else(|| PathBuf::from("."))
        .join("typefinch")
}
impl Preferences {
    fn parse_and_migrate(bytes: &[u8]) -> Option<(Self,bool)> {
        let data: serde_json::Value = serde_json::from_slice(bytes).ok()?;
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
    pub fn load() -> Self {
        let path=base_dir().join("settings.json");
        let parsed=fs::read(&path).ok().and_then(|bytes| Self::parse_and_migrate(&bytes));
        match parsed {
            Some((settings, needs_write)) => {
                // UUID migrations must be persisted *before* creating history,
                // or a legacy profile gets a new UUID each launch.
                if needs_write {
                    if let Err(err)=settings.save() {
                        eprintln!("Typefinch profile ID migration not saved: {err}");
                    }
                }
                settings
            },
            None => Self::default(),
        }
    }
    pub fn save(&self) -> Result<(), String> {
        let dir=base_dir();
        fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
        let data=serde_json::to_vec_pretty(self).map_err(|e| e.to_string())?;
        let tmp=dir.join("settings.json.tmp");
        fs::write(&tmp, data).map_err(|e| e.to_string())?;
        fs::rename(&tmp, dir.join("settings.json")).map_err(|e| e.to_string())
    }
}
#[cfg(test)]
mod tests {
    use super::*;
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
    #[test] fn legacy_profile_receives_id() {
        let p: Profile = serde_json::from_str(r#"{"name":"Old","tests":3,"best_wpm":60.0,"best_accuracy":95.0}"#).unwrap();
        assert!(!p.id.is_empty());
        assert_eq!(p.name, "Old");
    }
    #[test] fn system_is_default() { assert_eq!(Preferences::default().theme, Theme::System); }
    #[test] fn settings_roundtrip() {
        let p=Preferences::default();
        let b=serde_json::to_vec(&p).unwrap();
        let q: Preferences=serde_json::from_slice(&b).unwrap();
        assert_eq!(q.layout, "us");
        assert_eq!(q.test_mode, TestMode::Words);
        assert_eq!(q.lesson_id, "home");
    }
}
