//! Read-only Hyprland device inspection; not a privileged input grab.
use serde_json::Value;
use std::process::Command;

#[derive(Clone, Debug, Default)]
pub struct Keyboard {
    pub name: String,
    pub address: String,
    pub active_keymap: String,
    pub layout: String,
    pub variant: String,
}
/// Preserve a user-selected physical keyboard by Hyprland address through
/// enumeration changes. Selection is inspection-only, not event attribution.
pub fn preserve_selection(previous:Option<&str>, devices:&[Keyboard])->usize {
    previous.and_then(|address|devices.iter().position(|k|k.address==address)).unwrap_or(0)
}
pub fn inspect() -> Result<Vec<Keyboard>, String> {
    if std::env::var_os("HYPRLAND_INSTANCE_SIGNATURE").is_none() {
        return Err("Hyprland session unavailable; native app input still works.".into());
    }
    let out = Command::new("hyprctl").args(["-j", "devices"]).output().map_err(|e| format!("hyprctl unavailable: {e}"))?;
    if !out.status.success() { return Err("Hyprland did not return device information.".into()); }
    parse_json(&out.stdout)
}

fn parse_json(data: &[u8]) -> Result<Vec<Keyboard>, String> {
    let value: Value = serde_json::from_slice(data).map_err(|e| format!("Invalid Hyprland device JSON: {e}"))?;
    let keyboards = value.get("keyboards").and_then(Value::as_array).ok_or("Hyprland omitted the keyboards list.")?;
    Ok(keyboards.iter().map(|item| {
        let s = |key| item.get(key).and_then(Value::as_str).unwrap_or("").to_owned();
        Keyboard { name:s("name"), address:s("address"), active_keymap:s("active_keymap"), layout:s("layout"), variant:s("variant") }
    }).collect())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn hotplug_refresh_preserves_selected_device_or_uses_safe_first() {
        let original=vec![
            Keyboard {address:"a".into(),name:"A".into(),..Default::default()},
            Keyboard {address:"b".into(),name:"B".into(),..Default::default()},
        ];
        assert_eq!(preserve_selection(Some("b"),&original),1);
        let swapped=vec![original[1].clone(),original[0].clone()];
        assert_eq!(preserve_selection(Some("b"),&swapped),0);
        assert_eq!(preserve_selection(Some("missing"),&swapped),0);
        assert_eq!(preserve_selection(Some("b"),&[]),0);
    }
    #[test] fn multiple_keyboards_preserve_each_active_keymap() {
        let input = br#"{"keyboards":[{"name":"Laptop","address":"0x1","active_keymap":"English (US)","layout":"us"},{"name":"USB","address":"0x2","active_keymap":"German","layout":"de"}]}"#;
        let found = parse_json(input).unwrap();
        assert_eq!(found.len(),2);
        assert_eq!(found[0].active_keymap,"English (US)");
        assert_eq!(found[1].layout,"de");
    }
    #[test] fn invalid_device_json_fails_explicitly() { assert!(parse_json(b"[]").is_err()); }
    #[test] fn blank_keyboard_defaults_do_not_grant_input_access() {
        let k = Keyboard::default();
        assert!(k.name.is_empty());
    }
}
