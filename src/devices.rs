//! Read-only Hyprland device inspection; not a privileged input grab.
use serde_json::Value;
use std::{
    collections::HashSet,
    io::Read,
    process::{Command,Stdio},
    sync::mpsc,
    thread,
    time::{Duration,Instant},
};

/// Input originates from the compositor's aggregate seat. This application
/// only inspects physical keyboard metadata; no per-device stream, grant,
/// privileged input backend, or independent event filter exists.
#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub enum CapturePolicy { InspectionOnly }
pub const CAPTURE_POLICY:CapturePolicy=CapturePolicy::InspectionOnly;
impl CapturePolicy {
    pub const fn explanation(self)->&'static str {
        match self {
            Self::InspectionOnly=>
                "Per-device typing: unsupported. Selection inspects metadata only; scored input remains seat-level.",
        }
    }
}
#[derive(Clone, Debug, Default)]
pub struct Keyboard {
    pub name: String,
    pub address: String,
    pub active_keymap: String,
    pub layout: String,
    pub variant: String,
}
/// Preserve selection by opaque inspection identity through enumeration
/// changes. Neither a Hyprland address nor Wayland registry ID attributes events.
pub fn preserve_selection(previous:Option<&str>, devices:&[Keyboard])->usize {
    previous.and_then(|address|devices.iter().position(|k|k.address==address)).unwrap_or(0)
}
#[derive(Clone,Copy,Debug,PartialEq,Eq)]
enum InspectorBackend {Hyprland, X11Aggregate, WaylandAggregate, Unavailable}
fn backend_for_session(hyprland:bool,session_type:Option<&str>,
    has_x11_display:bool,has_wayland_socket:bool)->InspectorBackend {
    if hyprland {InspectorBackend::Hyprland}
    else if session_type==Some("x11") && has_x11_display {InspectorBackend::X11Aggregate}
    else if session_type==Some("wayland") && has_wayland_socket {InspectorBackend::WaylandAggregate}
    else {InspectorBackend::Unavailable}
}
pub fn inspect() -> Result<Vec<Keyboard>, String> {
    match backend_for_session(
        std::env::var_os("HYPRLAND_INSTANCE_SIGNATURE").is_some(),
        std::env::var("XDG_SESSION_TYPE").ok().as_deref(),
        std::env::var_os("DISPLAY").is_some(),
        std::env::var_os("WAYLAND_DISPLAY").is_some()
            || std::env::var_os("WAYLAND_SOCKET").is_some()) {
        InspectorBackend::Hyprland=>inspect_hyprland(),
        InspectorBackend::X11Aggregate=>inspect_x11_seat(),
        InspectorBackend::WaylandAggregate=>inspect_wayland_seats(),
        InspectorBackend::Unavailable=>Err(
            "No supported compositor keyboard inspector; native app input still works.".into()),
    }
}
/// A metadata query must not hold the only GUI inspector worker forever.
/// Both tools are invoked with constant, non-shell arguments. Limit elapsed
/// time and bytes *before* parsing, and never persist their raw output.
const INSPECT_TIMEOUT:Duration=Duration::from_secs(3);
const X11_QUERY_MAX_BYTES:usize=16*1024;
const HYPRLAND_QUERY_MAX_BYTES:usize=256*1024;
fn bounded_query(program:&str,args:&[&str],limit:usize,timeout:Duration)
    ->Result<Vec<u8>,String> {
    let mut child=Command::new(program)
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|e|format!("{program} unavailable: {e}"))?;
    let pipe=child.stdout.take().ok_or("Inspector stdout pipe unavailable.")?;
    let (send,receive)=mpsc::channel();
    thread::spawn(move||{
        let mut buf=Vec::new();
        // The extra byte distinguishes an exact-size response from an
        // overlarge one; never allocate an unbounded compositor response.
        let result=pipe.take((limit+1) as u64).read_to_end(&mut buf);
        let _=send.send(result.map(|_|buf));
    });
    let started=Instant::now();
    let status=loop {
        match child.try_wait() {
            Ok(Some(status))=>break status,
            Ok(None) if started.elapsed()<timeout=>thread::sleep(Duration::from_millis(10)),
            Ok(None)=>{
                let _=child.kill();
                let _=child.wait();
                return Err(format!("{program} inspection timed out."));
            }
            Err(err)=>{
                let _=child.kill();
                let _=child.wait();
                return Err(format!("{program} inspection failed: {err}"));
            }
        }
    };
    let remaining=timeout.saturating_sub(started.elapsed());
    let bytes=receive.recv_timeout(remaining)
        .map_err(|_|format!("{program} inspection output was not completed in time."))?
        .map_err(|e|format!("{program} inspection output failed: {e}"))?;
    if bytes.len()>limit {return Err(format!("{program} inspection output exceeded safe size."));}
    if !status.success(){return Err(format!("{program} inspection command failed."));}
    Ok(bytes)
}
fn inspect_hyprland() -> Result<Vec<Keyboard>, String> {
    let raw=bounded_query("hyprctl",&["-j","devices"],
        HYPRLAND_QUERY_MAX_BYTES,INSPECT_TIMEOUT)?;
    parse_json(&raw)
}
fn inspect_x11_seat()->Result<Vec<Keyboard>,String> {
    let raw=bounded_query("setxkbmap",&["-query"],
        X11_QUERY_MAX_BYTES,INSPECT_TIMEOUT)?;
    parse_x11_query(&raw)
}
fn inspect_wayland_seats()->Result<Vec<Keyboard>,String> {
    // Query standard wl_registry/wl_seat only, in a bounded, short-lived
    // copy of this executable. A stalled compositor cannot strand the
    // only inspector worker, and wl_keyboard is never requested.
    let path=std::env::current_exe()
        .map_err(|e|format!("Cannot locate Wayland seat inspector: {e}"))?;
    let exe=path.to_str().ok_or("Wayland inspector executable path is not UTF-8.")?;
    inspect_wayland_seats_from_executable(exe)
}
// The same parent-side bounded subprocess and parser can be exercised in
// native Weston CI using the built production binary (not the Rust test harness).
fn inspect_wayland_seats_from_executable(exe:&str)->Result<Vec<Keyboard>,String> {
    let raw=bounded_query(exe,&["--_probe_wayland_seats"],
        X11_QUERY_MAX_BYTES,INSPECT_TIMEOUT)?;
    parse_wayland_seats(&raw)
}
fn parse_wayland_seats(raw:&[u8])->Result<Vec<Keyboard>,String>{
    if raw.len()>X11_QUERY_MAX_BYTES {
        return Err("Wayland seat response exceeds inspection size limit.".into());
    }
    let seats:Vec<crate::wayland_seat::Seat>=serde_json::from_slice(raw)
        .map_err(|e|format!("Invalid Wayland seat metadata: {e}"))?;
    if seats.len()>MAX_INSPECTED_KEYBOARDS {
        return Err("Wayland returned too many seat entries.".into());
    }
    let mut identities=HashSet::new();
    let mut found=Vec::new();
    for seat in seats {
        // A protocol name is descriptive but not guaranteed unique. Use
        // wl_registry global IDs to distinguish seats even if names collide.
        // These identities are compositor-local, never hardware identities.
        if !usable_metadata(&seat.name,128) || !identities.insert(seat.id) {
            return Err("Wayland returned an invalid name or duplicate registry seat ID.".into());
        }
        if !seat.keyboard {continue;}
        // wl_seat groups input devices, never physical keyboards. Neither
        // wl_seat names nor capabilities reveal the active XKB keymap.
        found.push(Keyboard {
            name:format!("Wayland seat {} (registry {}, not a physical keyboard)",seat.name,seat.id),
            address:format!("wayland:seat:{}",seat.id),
            active_keymap:String::new(),
            layout:String::new(),
            variant:String::new(),
        });
    }
    Ok(found)
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

const MAX_HYPRLAND_RESPONSE_BYTES:usize=HYPRLAND_QUERY_MAX_BYTES;
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
    #[test] fn metadata_subprocesses_are_bounded_and_do_not_require_a_shell_for_runtime_use() {
        // The production path uses constant executable/argument pairs,
        // never a user-controlled shell string. CI's shell is used here
        // only to exercise a small, controllable test child.
        let text=bounded_query("sh",&["-c","printf 'layout: us\\n'"],
            128,Duration::from_secs(2)).unwrap();
        assert_eq!(text,b"layout: us\n");
        let err=bounded_query("sh",&["-c","head -c 1024 /dev/zero"],
            128,Duration::from_secs(2)).unwrap_err();
        assert!(err.contains("safe size")||err.contains("command failed"));
        let started=Instant::now();
        let err=bounded_query("sh",&["-c","exec sleep 2"],
            128,Duration::from_millis(75)).unwrap_err();
        assert!(err.contains("timed out"));
        assert!(started.elapsed()<Duration::from_secs(1),
            "a blocked inspector must never block the UI or future refreshes");
    }
    #[test] fn per_device_input_is_an_explicit_unsupported_mode() {
        assert_eq!(CAPTURE_POLICY,CapturePolicy::InspectionOnly);
        assert!(CAPTURE_POLICY.explanation().contains("unsupported"));
        let input=br#"{"keyboards":[
            {"name":"First","address":"0xa","active_keymap":"English (US)"},
            {"name":"Second","address":"0xb","active_keymap":"German"}]}"#;
        let found=parse_json(input).unwrap();
        assert_eq!(found.len(),2);
        // These are two *inspection rows*, not two event streams. No
        // selector or keyboard address grants input-device attribution.
        assert_ne!(found[0].address,found[1].address);
        assert_eq!(CAPTURE_POLICY,CapturePolicy::InspectionOnly);
    }
    #[test] fn xwayland_display_is_never_used_as_native_x11_seat_evidence() {
        use InspectorBackend::*;
        assert_eq!(backend_for_session(false,Some("wayland"),true,false),Unavailable);
        assert_eq!(backend_for_session(false,Some("wayland"),true,true),WaylandAggregate);
        assert_eq!(backend_for_session(false,Some("wayland"),false,true),WaylandAggregate);
        assert_eq!(backend_for_session(false,None,true,true),Unavailable);
        assert_eq!(backend_for_session(false,Some("x11"),false,true),Unavailable);
        assert_eq!(backend_for_session(false,Some("x11"),true,true),X11Aggregate);
        assert_eq!(backend_for_session(true,Some("wayland"),true,true),Hyprland);
        assert_eq!(backend_for_session(true,Some("x11"),true,true),Hyprland);
    }
    #[test] fn wayland_seat_protocol_metadata_never_invents_input_layouts() {
        let payload=serde_json::to_vec(&vec![
            crate::wayland_seat::Seat{id:10,name:"seat0".into(),keyboard:true},
            crate::wayland_seat::Seat{id:11,name:"pointer".into(),keyboard:false},
        ]).unwrap();
        let devices=parse_wayland_seats(&payload).unwrap();
        assert_eq!(devices.len(),1);
        assert_eq!(devices[0].address,"wayland:seat:10");
        assert!(devices[0].name.contains("not a physical keyboard"));
        assert!(devices[0].active_keymap.is_empty());
        assert!(devices[0].layout.is_empty());
        assert!(devices[0].variant.is_empty());
        assert!(parse_wayland_seats(b"not json").is_err());
        assert!(parse_wayland_seats(&vec![b'a';X11_QUERY_MAX_BYTES+1]).is_err());
        // Distinct seats are allowed to have the same descriptive name.
        let same_name=serde_json::to_vec(&vec![
            crate::wayland_seat::Seat{id:1,name:"seat0".into(),keyboard:true},
            crate::wayland_seat::Seat{id:2,name:"seat0".into(),keyboard:true},
        ]).unwrap();
        let distinct=parse_wayland_seats(&same_name).unwrap();
        assert_eq!(distinct.len(),2);
        assert_ne!(distinct[0].address,distinct[1].address);
        assert!(distinct.iter().all(|seat|seat.active_keymap.is_empty()));
        assert_eq!(preserve_selection(Some("wayland:seat:2"),&distinct),1);
        let duplicate_id=serde_json::to_vec(&vec![
            crate::wayland_seat::Seat{id:1,name:"seat0".into(),keyboard:true},
            crate::wayland_seat::Seat{id:1,name:"seat1".into(),keyboard:true},
        ]).unwrap();
        assert!(parse_wayland_seats(&duplicate_id).is_err());
    }
    #[test] fn wayland_seat_capability_transitions_do_not_preserve_stale_selection() {
        let first=serde_json::to_vec(&vec![
            crate::wayland_seat::Seat{id:10,name:"seat0".into(),keyboard:true},
            crate::wayland_seat::Seat{id:11,name:"seat1".into(),keyboard:false},
        ]).unwrap();
        let before=parse_wayland_seats(&first).unwrap();
        assert_eq!(before.len(),1);
        assert_eq!(before[0].address,"wayland:seat:10");
        let next=serde_json::to_vec(&vec![
            crate::wayland_seat::Seat{id:10,name:"seat0".into(),keyboard:false},
            crate::wayland_seat::Seat{id:11,name:"seat1".into(),keyboard:true},
        ]).unwrap();
        let after=parse_wayland_seats(&next).unwrap();
        assert_eq!(after.len(),1);
        assert_eq!(after[0].address,"wayland:seat:11");
        assert_eq!(preserve_selection(Some("wayland:seat:10"),&after),0);
        assert!(after[0].active_keymap.is_empty());
    }
    #[test] fn native_wayland_child_inspector_is_bounded_and_unattributed() {
        // The CI Weston compositor has no attached hardware keyboard.
        // This checks the actual production probe binary and parent parser,
        // not seat-capability acceptance with physical keyboard hardware.
        let Ok(exe)=std::env::var("TYPEFINCH_QA_WAYLAND_INSPECTOR") else {return;};
        assert_eq!(std::env::var("XDG_SESSION_TYPE").as_deref(),Ok("wayland"));
        let devices=inspect_wayland_seats_from_executable(&exe).unwrap();
        assert!(devices.iter().all(|seat|
            seat.address.starts_with("wayland:seat:")
            && seat.active_keymap.is_empty()
            && seat.layout.is_empty()
            && seat.variant.is_empty()));
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
    #[test] fn synthetic_hyprland_inventory_lifecycle_requires_explicit_fixture() {
        let Ok(scenario)=std::env::var("TYPEFINCH_QA_HYPRLAND_CASE") else {return;};
        assert_eq!(std::env::var_os("HYPRLAND_INSTANCE_SIGNATURE").as_deref(),
            Some(std::ffi::OsStr::new("typefinch-qa-only")));
        assert_eq!(std::env::var("XDG_SESSION_TYPE").as_deref(),Ok("wayland"));
        // A separate disposable CI executable named hyprctl supplies data
        // through the real bounded subprocess path. This is NOT physical
        // Hyprland/hardware acceptance.
        let started=Instant::now();
        let result=inspect();
        match scenario.as_str() {
            "initial"|"reordered"=>{
                let found=result.unwrap();
                assert_eq!(found.len(),2);
                let names=found.iter().map(|k|k.address.as_str()).collect::<Vec<_>>();
                assert_eq!(names,
                    if scenario=="initial"{vec!["0x1","0x2"]}else{vec!["0x2","0x1"]});
                assert_eq!(preserve_selection(Some("0x2"),&found),
                    if scenario=="initial"{1}else{0});
                assert_eq!(found.iter().find(|k|k.address=="0x2").unwrap().active_keymap,
                    "German");
            },
            "unplugged"=>{
                let found=result.unwrap();
                assert_eq!(found.len(),1);
                assert_eq!(found[0].address,"0x1");
                assert_eq!(preserve_selection(Some("0x2"),&found),0);
            },
            "duplicates"=>{
                let found=result.unwrap();
                assert_eq!(found.len(),1);
                assert_eq!(found[0].active_keymap,"English (US)");
                assert_eq!(found[0].address,"0x1");
            },
            "invalid"=>{
                assert!(result.unwrap_err().contains("Invalid Hyprland device JSON"));
            },
            "hung"=>{
                assert!(result.unwrap_err().contains("timed out"));
                assert!(started.elapsed()<Duration::from_secs(5));
            },
            _=>panic!("unrecognized synthetic Hyprland QA scenario"),
        }
        assert!(started.elapsed()<Duration::from_secs(5));
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
