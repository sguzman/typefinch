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
    let value: Value = serde_json::from_slice(&out.stdout).map_err(|e| format!("Invalid Hyprland device JSON: {e}"))?;
    let keyboards = value.get("keyboards").and_then(Value::as_array).ok_or("Hyprland omitted the keyboards list.")?;
    Ok(keyboards.iter().map(|item| {
        let s = |key| item.get(key).and_then(Value::as_str).unwrap_or("").to_owned();
        Keyboard { name:s("name"), address:s("address"), active_keymap:s("active_keymap"), layout:s("layout"), variant:s("variant") }
    }).collect())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn blank_keyboard_defaults_do_not_grant_input_access() {
        let k = Keyboard::default();
        assert!(k.name.is_empty());
    }
}
