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
fn new_profile_id() -> String { uuid::Uuid::new_v4().to_string() }
impl Default for Profile {
    fn default() -> Self { Self { id:new_profile_id(), name:"Finch".into(), tests:0, best_wpm:0.0, best_accuracy:0.0, goals:Goals::default() } }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct Preferences {
    pub theme: Theme,
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
    fn default() -> Self { Self { theme:Theme::System, card_theme:CardTheme::Dusk, card_font:CardFont::Sans, card_template:CardTemplate::Classic, test_mode:TestMode::Words, language:Language::English, word_difficulty:WordDifficulty::Standard, lesson_id:"home".into(), layout:"us".into(), custom_layouts:Vec::new(), profiles:vec![Profile::default()], active_profile:0, duration_seconds:30 } }
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
