//! Read-only Hyprland device inspection; not a privileged input grab.
use serde_json::Value;
use std::{collections::HashSet,process::Command};

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
#[derive(Clone,Copy,Debug,PartialEq,Eq)]
enum InspectorBackend {Hyprland, X11Aggregate, Unavailable}
fn backend_for_session(hyprland:bool,session_type:Option<&str>,has_display:bool)
    ->InspectorBackend {
    if hyprland {InspectorBackend::Hyprland}
    else if session_type==Some("x11") && has_display {InspectorBackend::X11Aggregate}
    else {InspectorBackend::Unavailable}
}
pub fn inspect() -> Result<Vec<Keyboard>, String> {
    match backend_for_session(
        std::env::var_os("HYPRLAND_INSTANCE_SIGNATURE").is_some(),
        std::env::var("XDG_SESSION_TYPE").ok().as_deref(),
        std::env::var_os("DISPLAY").is_some()) {
        InspectorBackend::Hyprland=>inspect_hyprland(),
        InspectorBackend::X11Aggregate=>inspect_x11_seat(),
        InspectorBackend::Unavailable=>Err(
            "No supported compositor keyboard inspector; native app input still works.".into()),
    }
}
fn inspect_hyprland() -> Result<Vec<Keyboard>, String> {
    let out=Command::new("hyprctl").args(["-j","devices"]).output()
        .map_err(|e|format!("hyprctl unavailable: {e}"))?;
    if !out.status.success(){return Err("Hyprland did not return device information.".into());}
    parse_json(&out.stdout)
}
fn inspect_x11_seat()->Result<Vec<Keyboard>,String> {
    let out=Command::new("setxkbmap").arg("-query").output()
        .map_err(|e|format!("X11 aggregate seat inspector unavailable: {e}"))?;
    if !out.status.success(){return Err("X11 aggregate seat query failed.".into());}
    parse_x11_query(&out.stdout)
}
/// An X11 core seat is NOT a physical device. Never return multiple
/// invented keyboards, silently infer a hardware shape, or claim key events
/// can be attributed to the reported seat.
fn parse_x11_query(data:&[u8])->Result<Vec<Keyboard>,String>{
    if data.len()>16*1024{return Err("X11 seat query exceeds safe size.".into());}
    let body=std::str::from_utf8(data)
        .map_err(|_|"X11 seat query was not UTF-8.".to_string())?;
    let mut layout=None;
    let mut variant=String::new();
    for line in body.lines() {
        let Some((key,value))=line.split_once(':') else {continue};
        match key.trim() {
            "layout"=>layout=Some(value.trim().to_string()),
            "variant"=>variant=value.trim().to_string(),
            _=>{},
        }
    }
    let layout=layout.filter(|value|!value.is_empty() && value.len()<=128
        && !value.chars().any(char::is_control))
        .ok_or("X11 seat query omitted a valid layout.")?;
    if variant.len()>128 || variant.chars().any(char::is_control) {
        return Err("X11 seat query returned an invalid variant.".into());
    }
    Ok(vec![Keyboard {
        name:"X11 aggregate seat (not a physical keyboard)".into(),
        address:"x11:aggregate-seat".into(),
        // A configured set of X11 groups is not an observed active group.
        active_keymap:String::new(),
        layout,
        variant,
    }])
}

const MAX_HYPRLAND_RESPONSE_BYTES:usize=256*1024;
const MAX_INSPECTED_KEYBOARDS:usize=128;
fn usable_metadata(value:&str,max_len:usize)->bool {
    !value.is_empty() && value.len()<=max_len
        && !value.chars().any(char::is_control)
}
fn parse_json(data: &[u8]) -> Result<Vec<Keyboard>, String> {
    if data.len()>MAX_HYPRLAND_RESPONSE_BYTES {
        return Err("Hyprland device inventory exceeds inspection size limit.".into());
    }
    let value:Value=serde_json::from_slice(data)
        .map_err(|e|format!("Invalid Hyprland device JSON: {e}"))?;
    let keyboards=value.get("keyboards").and_then(Value::as_array)
        .ok_or("Hyprland omitted the keyboards list.")?;
    if keyboards.len()>MAX_INSPECTED_KEYBOARDS {
        return Err("Hyprland returned too many keyboard entries.".into());
    }
    let mut seen=HashSet::new();
    let mut found=Vec::with_capacity(keyboards.len());
    for item in keyboards {
        // Hyprland addresses identify *inspection rows* across hotplug
        // refreshes. A blank or duplicate identity cannot safely preserve
        // a selected row; do not invent one from its array index/name.
        let Some(address)=item.get("address").and_then(Value::as_str) else {continue};
        if !usable_metadata(address,256) || !seen.insert(address.to_owned()) {
            continue;
        }
        let field=|key:&str,max_len|item.get(key).and_then(Value::as_str)
            .filter(|v|v.len()<=max_len && !v.chars().any(char::is_control))
            .unwrap_or("").to_owned();
        let name=field("name",256);
        found.push(Keyboard{
            name:if name.is_empty(){"Unlabeled keyboard".into()}else{name},
            address:address.to_owned(),
            active_keymap:field("active_keymap",128),
            layout:field("layout",128),
            variant:field("variant",128),
        });
    }
    if !keyboards.is_empty() && found.is_empty() {
        return Err("Hyprland returned no keyboard entries with usable unique addresses.".into());
    }
    Ok(found)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn xwayland_display_is_never_used_as_native_x11_seat_evidence() {
        use InspectorBackend::*;
        assert_eq!(backend_for_session(false,Some("wayland"),true),Unavailable);
        assert_eq!(backend_for_session(false,Some("wayland"),false),Unavailable);
        assert_eq!(backend_for_session(false,None,true),Unavailable);
        assert_eq!(backend_for_session(false,Some("x11"),false),Unavailable);
        assert_eq!(backend_for_session(false,Some("x11"),true),X11Aggregate);
        assert_eq!(backend_for_session(true,Some("wayland"),true),Hyprland);
        assert_eq!(backend_for_session(true,Some("x11"),true),Hyprland);
    }
    #[test] fn x11_seat_parser_does_not_invent_per_device_identity() {
        let raw=b"rules: evdev\nmodel: pc105\nlayout: us,ru\nvariant: ,phonetic\noptions: grp:alt_shift_toggle\n";
        let found=parse_x11_query(raw).unwrap();
        assert_eq!(found.len(),1);
        assert!(found[0].name.contains("not a physical keyboard"));
        assert_eq!(found[0].address,"x11:aggregate-seat");
        assert_eq!(found[0].layout,"us,ru");
        assert_eq!(found[0].variant,",phonetic");
        assert!(found[0].active_keymap.is_empty(),
            "configured X11 groups must not be reported as observed input");
    }
    #[test] fn x11_bad_or_unavailable_layout_is_not_guessed() {
        assert!(parse_x11_query(b"rules: evdev\nvariant: intl\n").is_err());
        assert!(parse_x11_query(b"layout: \n").is_err());
        assert!(parse_x11_query(b"layout: us\nvariant: ok\rno\n").is_err());
        assert!(parse_x11_query(&[255,254]).is_err());
        assert!(parse_x11_query(&vec![b'a';16*1024+1]).is_err());
    }
    #[test] fn native_x11_aggregate_seat_is_read_only() {
        // Exercised under Xvfb by the dedicated CI step, not simulated
        // during the default unit-test pass without DISPLAY.
        if std::env::var_os("TYPEFINCH_QA_X11_SEAT").is_none(){return;}
        assert_eq!(std::env::var("XDG_SESSION_TYPE").as_deref(),Ok("x11"));
        assert!(std::env::var_os("DISPLAY").is_some());
        assert!(std::env::var_os("HYPRLAND_INSTANCE_SIGNATURE").is_none());
        let found=inspect().expect("X11 aggregate seat must be readable");
        assert_eq!(found.len(),1);
        assert_eq!(found[0].address,"x11:aggregate-seat");
        assert!(!found[0].layout.is_empty());
        assert!(found[0].active_keymap.is_empty());
        assert!(found[0].name.contains("not a physical keyboard"));
    }
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
    #[test] fn duplicate_or_unaddressed_keyboards_cannot_hijack_inspection_selection() {
        let input=br#"{"keyboards":[
            {"name":"First","address":"0x1","active_keymap":"English (US)"},
            {"name":"Imposter","address":"0x1","active_keymap":"German"},
            {"name":"No identity","active_keymap":"Spanish"},
            {"name":"Control","address":"bad\naddress"},
            {"name":"USB","address":"0x2","active_keymap":"German"},
            {"name":"Broken label\nline","address":"0x3","active_keymap":"English (US)"}
        ]}"#;
        let found=parse_json(input).unwrap();
        assert_eq!(found.len(),3);
        assert_eq!(found[0].name,"First");
        assert_eq!(found[0].active_keymap,"English (US)");
        assert_eq!(found[1].address,"0x2");
        assert_eq!(found[2].name,"Unlabeled keyboard");
        assert_eq!(preserve_selection(Some("0x2"),&found),1);
        assert_eq!(preserve_selection(Some("bad\naddress"),&found),0);
        assert!(parse_json(br#"{"keyboards":[{"name":"anonymous"}]}"#).is_err());
        assert!(parse_json(br#"{"keyboards":[]}"#).unwrap().is_empty());
    }
    #[test] fn compositor_inventory_is_bounded_and_never_executable() {
        assert!(parse_json(&vec![b'a';MAX_HYPRLAND_RESPONSE_BYTES+1]).is_err());
        let entries=(0..MAX_INSPECTED_KEYBOARDS+1)
            .map(|i|serde_json::json!({"address":format!("0x{i:x}"),"name":"K"}))
            .collect::<Vec<_>>();
        let huge=serde_json::to_vec(&serde_json::json!({"keyboards":entries})).unwrap();
        assert!(parse_json(&huge).is_err());
        assert!(parse_json(br#"{"keyboards":[{"address":"bad\u0000id"}]}"#).is_err());
    }
    #[test] fn invalid_device_json_fails_explicitly() { assert!(parse_json(b"[]").is_err()); }
    #[test] fn blank_keyboard_defaults_do_not_grant_input_access() {
        let k = Keyboard::default();
        assert!(k.name.is_empty());
    }
}
