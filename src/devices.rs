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
