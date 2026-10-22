//! Local user-controlled configuration; no online identity.
use serde::{Deserialize, Serialize};
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
    pub layout: String,
    pub profiles: Vec<Profile>,
    pub active_profile: usize,
    pub duration_seconds: u64,
}
impl Default for Preferences {
    fn default() -> Self { Self { theme:Theme::System, layout:"us".into(), profiles:vec![Profile::default()], active_profile:0, duration_seconds:30 } }
}
pub fn base_dir() -> PathBuf {
    std::env::var_os("XDG_CONFIG_HOME").map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|s| PathBuf::from(s).join(".config")))
        .unwrap_or_else(|| PathBuf::from("."))
        .join("typefinch")
}
impl Preferences {
    pub fn load() -> Self {
        fs::read(base_dir().join("settings.json")).ok()
            .and_then(|b| serde_json::from_slice::<Self>(&b).ok())
            .filter(|p| !p.profiles.is_empty() && p.active_profile < p.profiles.len())
            .unwrap_or_default()
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
    }
}
