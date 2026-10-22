//! Discovery of system-installed XKB layout and variant labels.
//! The xkeyboard-config rules file is data, not executable code.
use std::{collections::HashSet, fs, path::PathBuf};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InstalledLayout {
    pub id: String,
    pub name: String,
}
#[derive(Clone, Copy, PartialEq, Eq)]
enum Section { None, Layout, Variant }

pub fn parse_rules(input: &str) -> Vec<InstalledLayout> {
    let mut section = Section::None;
    let mut results = Vec::new();
    let mut seen = HashSet::new();
    for line in input.lines() {
        let line = line.trim();
        if line.starts_with('!') {
            section = match line.trim_start_matches('!').trim().split_whitespace().next() {
                Some("layout") => Section::Layout,
                Some("variant") => Section::Variant,
                _ => Section::None,
            };
            continue;
        }
        if line.is_empty() || line.starts_with('#') { continue; }
        let Some((code, label)) = line.split_once(char::is_whitespace) else { continue };
        let label = label.trim();
        let (id, display) = match section {
            Section::Layout => (code.to_string(),label.to_string()),
            Section::Variant => match label.split_once(':') {
                Some((base, description)) if !base.trim().is_empty() =>
                    (format!("{}({code})",base.trim()),description.trim().to_string()),
                _ => continue
            },
            Section::None => continue,
        };
        if !display.is_empty() && seen.insert(id.clone()) {
            results.push(InstalledLayout { id, name: display });
        }
    }
    results.sort_by(|a,b| a.name.to_lowercase().cmp(&b.name.to_lowercase()).then(a.id.cmp(&b.id)));
    results
}
/// Resolve a compositor-reported display name only when exactly one installed
/// XKB entry matches it; never guess between similarly named variants.
pub fn resolve_active<'a>(layouts: &'a [InstalledLayout], active: &str) -> Option<&'a InstalledLayout> {
    let normalized = |name: &str| name.split_whitespace().collect::<Vec<_>>().join(" ").to_lowercase();
    let label = normalized(active);
    if label.is_empty() { return None; }
    let mut hits = layouts.iter().filter(|l| normalized(&l.name) == label);
    let first = hits.next()?;
    if hits.next().is_some() { None } else { Some(first) }
}
pub fn discover() -> Result<Vec<InstalledLayout>, String> {
    let root = std::env::var_os("XKB_CONFIG_ROOT")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("/usr/share/X11/xkb"));
    let file = root.join("rules/evdev.lst");
    let source = fs::read_to_string(&file).map_err(|e| format!("XKB layout registry {}: {e}",file.display()))?;
    let layouts = parse_rules(&source);
    if layouts.is_empty() { Err("XKB registry contains no layouts".to_owned()) } else { Ok(layouts) }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn parse_layouts_variants_and_skip_other_sections() {
        let sample = "! model\n pc105 Generic\n! layout\n us English (US)\n de German\n! variant\n colemak us: English (Colemak)\n nodeadkeys de: German (no dead keys)\n! option\n abc ignore\n";
        let x = parse_rules(sample);
        assert_eq!(x.len(),4);
        assert!(x.iter().any(|t| t.id=="us" && t.name=="English (US)"));
        assert!(x.iter().any(|t| t.id=="de(nodeadkeys)" && t.name=="German (no dead keys)"));
        assert!(x.iter().any(|t| t.id=="us(colemak)"));
    }
    #[test] fn active_map_detection_requires_unique_name() {
        let data = vec![
            InstalledLayout { id:"us".into(),name:"English (US)".into() },
            InstalledLayout { id:"de".into(),name:"German".into() }
        ];
        assert_eq!(resolve_active(&data,"  english   (US) ").unwrap().id,"us");
        assert!(resolve_active(&data,"Swedish").is_none());
        let mut dup = data;
        dup.push(InstalledLayout { id:"us(custom)".into(),name:"English (US)".into() });
        assert!(resolve_active(&dup,"English (US)").is_none());
    }
    #[test] fn duplicate_ids_are_not_offered() {
        let input = "! layout\n us English (US)\n us Duplicate\n";
        let x=parse_rules(input);
        assert_eq!(x.len(),1);
        assert_eq!(x[0].name,"English (US)");
    }
    #[test] fn installed_registry_has_us_if_present() {
        if let Ok(found)=discover() {
            assert!(found.iter().any(|l|l.id=="us"));
        }
    }
}
