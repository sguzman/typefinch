//! Short-lived, bounded Wayland seat inspection. No surfaces, input focus,
//! global keyboard interception, physical attribution or raw event logging.
use serde::{Deserialize,Serialize};
use std::{collections::BTreeMap,fs::File,io::Read,os::fd::OwnedFd};
use wayland_client::{
    protocol::{wl_keyboard,wl_registry,wl_seat},
    Connection,Dispatch,QueueHandle,WEnum,
};
use xkbcommon::xkb;

#[derive(Clone,Debug,Serialize,Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Seat {
    pub id:u32,
    pub name:String,
    pub keyboard:bool,
}
/// The compositor's keymap GROUPS, not an observed active group, typing source
/// or identity of any physical keyboard. Missing keymaps remain empty.
#[derive(Clone,Debug,Serialize,Deserialize)]
#[serde(deny_unknown_fields)]
pub struct XkbSeat {
    pub id:u32,
    pub name:String,
    pub keyboard:bool,
    #[serde(default)]
    pub xkb_groups:Vec<String>,
}
const MAX_SEATS:usize=128;
const MAX_XKB_SEATS:usize=8;
const MAX_XKB_KEYMAP_BYTES:u32=4*1024*1024;
const MAX_XKB_GROUPS:u32=16;

#[derive(Default)]
struct State {
    seats:BTreeMap<u32,Seat>,
    keyboards:BTreeMap<u32,wl_keyboard::WlKeyboard>,
    keymaps:BTreeMap<u32,Vec<String>>,
    probe_xkb:bool,
    overflow:bool,
    invalid_name:bool,
    invalid_keymap:bool,
}
impl State {
    fn add_seat(&mut self,id:u32)->bool {
        if self.seats.contains_key(&id) {return false;}
        if self.seats.len()>=MAX_SEATS {
            self.overflow=true;
            return false;
        }
        self.seats.insert(id,Seat {
            id,name:format!("unnamed-{id}"),keyboard:false
        });
        true
    }
    fn update_capability(&mut self,id:u32,keyboard:bool){
        if let Some(seat)=self.seats.get_mut(&id){seat.keyboard=keyboard;}
    }
    fn update_name(&mut self,id:u32,name:&str){
        if !self.seats.contains_key(&id){return;}
        if name.is_empty() || name.len()>128 || name.chars().any(char::is_control) {
            self.invalid_name=true;
            return;
        }
        if let Some(seat)=self.seats.get_mut(&id){seat.name=name.into();}
    }
}
/// Cap bytes before parsing, require an entire null-terminated valid XKB map,
/// and return only bounded human-readable group labels. Never expose raw map.
fn parse_xkb_keymap(fd:OwnedFd,size:u32)->Result<Vec<String>,String>{
    if !(2..=MAX_XKB_KEYMAP_BYTES).contains(&size){
        return Err("Wayland XKB keymap size invalid or excessive.".into());
    }
    let mut bytes=Vec::new();
    File::from(fd).take(u64::from(size)+1).read_to_end(&mut bytes)
        .map_err(|_|"Wayland XKB keymap read failed.")?;
    if bytes.len()!=size as usize || bytes.pop()!=Some(0){
        return Err("Wayland XKB keymap truncated or missing terminator.".into());
    }
    let source=String::from_utf8(bytes)
        .map_err(|_|"Wayland XKB keymap is not UTF-8.")?;
    let ctx=xkb::Context::new(xkb::CONTEXT_NO_FLAGS);
    let map=xkb::Keymap::new_from_string(
        &ctx,source,xkb::KEYMAP_FORMAT_TEXT_V1,xkb::COMPILE_NO_FLAGS)
        .ok_or("Wayland XKB keymap compilation failed.")?;
    let groups=map.num_layouts();
    if groups==0 || groups>MAX_XKB_GROUPS {
        return Err("Wayland XKB keymap group count invalid.".into());
    }
    (0..groups).map(|idx|{
        let name=map.layout_get_name(idx);
        if name.is_empty() || name.len()>128 || name.chars().any(char::is_control) {
            Err("Wayland XKB group name invalid.".into())
        } else {Ok(name.to_owned())}
    }).collect()
}
impl Dispatch<wl_registry::WlRegistry,()> for State {
    fn event(state:&mut Self,registry:&wl_registry::WlRegistry,
        event:wl_registry::Event,_:&(),_:&Connection,qh:&QueueHandle<Self>) {
        match event {
            wl_registry::Event::Global{name,interface,version}
                if interface=="wl_seat" && version>=1=>{
                if state.add_seat(name){
                    // Bind v2 for compositor seat names; no physical device ID.
                    registry.bind::<wl_seat::WlSeat,_,_>(name,version.min(2),qh,name);
                }
            },
            wl_registry::Event::GlobalRemove{name}=>{
                state.seats.remove(&name);
                state.keyboards.remove(&name);
                state.keymaps.remove(&name);
            },
            _=>{},
        }
    }
}
impl Dispatch<wl_seat::WlSeat,u32> for State {
    fn event(state:&mut Self,seat:&wl_seat::WlSeat,
        event:wl_seat::Event,id:&u32,_:&Connection,qh:&QueueHandle<Self>) {
        match event {
            wl_seat::Event::Capabilities{capabilities}=>{
                let keyboard=matches!(capabilities,
                    WEnum::Value(c) if c.contains(wl_seat::Capability::Keyboard));
                state.update_capability(*id,keyboard);
                if keyboard && state.probe_xkb && !state.keyboards.contains_key(id){
                    if state.keyboards.len()>=MAX_XKB_SEATS {
                        state.invalid_keymap=true;
                    } else {
                        state.keyboards.insert(*id,seat.get_keyboard(qh,*id));
                    }
                }
                if !keyboard {
                    state.keyboards.remove(id);
                    state.keymaps.remove(id);
                }
            },
            wl_seat::Event::Name{name}=>state.update_name(*id,&name),
            _=>{},
        }
    }
}
// A separate, unfocused client has no Wayland keyboard focus. It requests
// wl_keyboard only to receive the compositor-provided keymap descriptor.
// Ignore every key/enter/leave/modifier/repeat event without inspecting it.
impl Dispatch<wl_keyboard::WlKeyboard,u32> for State {
    fn event(state:&mut Self,_:&wl_keyboard::WlKeyboard,
        event:wl_keyboard::Event,id:&u32,_:&Connection,_:&QueueHandle<Self>) {
        if let wl_keyboard::Event::Keymap{format,fd,size}=event {
            if format!=wl_keyboard::KeymapFormat::XkbV1.into(){
                state.invalid_keymap=true;
                return;
            }
            match parse_xkb_keymap(fd,size){
                Ok(groups)=>{state.keymaps.insert(*id,groups);},
                Err(_)=>{state.invalid_keymap=true;},
            }
        }
    }
}
fn inspect_seats(with_xkb:bool)->Result<State,String>{
    let conn=Connection::connect_to_env()
        .map_err(|e|format!("Wayland compositor connection unavailable: {e}"))?;
    let mut queue=conn.new_event_queue::<State>();
    let qh=queue.handle();
    let _registry=conn.display().get_registry(&qh,());
    let mut state=State{probe_xkb:with_xkb,..Default::default()};
    queue.roundtrip(&mut state)
        .map_err(|e|format!("Wayland registry inspection failed: {e}"))?;
    queue.roundtrip(&mut state)
        .map_err(|e|format!("Wayland seat capability inspection failed: {e}"))?;
    if with_xkb {
        queue.roundtrip(&mut state)
            .map_err(|e|format!("Wayland XKB keymap inspection failed: {e}"))?;
    }
    if state.overflow {
        return Err("Wayland advertised more than 128 seat globals; refusing truncated inventory.".into());
    }
    if state.invalid_name {
        return Err("Wayland advertised malformed seat names; refusing identity substitution.".into());
    }
    if state.invalid_keymap {
        return Err("Wayland XKB seat keymap validation failed; refusing partial inventory.".into());
    }
    Ok(state)
}
/// Legacy capability-only probe: never requests wl_keyboard.
pub fn read_only_seats()->Result<Vec<Seat>,String>{
    Ok(inspect_seats(false)?.seats.into_values().collect())
}
/// Generic XKB group introspection. The child process exits immediately
/// after these three roundtrips. It never receives global keyboard input.
pub fn read_xkb_seats()->Result<Vec<XkbSeat>,String>{
    let state=inspect_seats(true)?;
    let groups=state.keymaps;
    Ok(state.seats.into_values().map(|seat|XkbSeat{
        id:seat.id,name:seat.name,keyboard:seat.keyboard,
        xkb_groups:groups.get(&seat.id).cloned().unwrap_or_default(),
    }).collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn seat_name_and_capability_changes_do_not_imply_a_key_event_stream() {
        let mut state=State::default();
        assert!(state.add_seat(7));
        state.update_capability(7,true);
        state.update_name(7,"seat0");
        assert_eq!(state.seats.get(&7).unwrap().name,"seat0");
        assert!(state.seats.get(&7).unwrap().keyboard);
        state.update_name(7,"bad\nname");
        assert_eq!(state.seats.get(&7).unwrap().name,"seat0");
        assert!(state.invalid_name,"bad name must invalidate inventory");
        state.update_name(7,"");
        assert_eq!(state.seats.get(&7).unwrap().name,"seat0");
        state.update_capability(7,false);
        assert!(!state.seats.get(&7).unwrap().keyboard);
        assert_eq!(state.seats.get(&7).unwrap().id,7);
        assert!(!state.overflow);
        assert!(state.invalid_name);
    }
    #[test] fn unnamed_v1_seat_is_distinct_from_invalid_name_event() {
        let mut state=State::default();
        assert!(state.add_seat(10));
        assert_eq!(state.seats.get(&10).unwrap().name,"unnamed-10");
        assert!(!state.invalid_name,"absence of wl_seat v2 name is legal");
        state.update_name(999,"bad\nname");
        assert!(!state.invalid_name,"unbound or removed seat is ignored");
        state.update_name(10,&"a".repeat(129));
        assert!(state.invalid_name,"oversized names must fail closed");
    }
    #[test] fn oversized_wayland_registry_fails_closed() {
        let mut state=State::default();
        for id in 0..MAX_SEATS as u32 {
            assert!(state.add_seat(id));
        }
        assert_eq!(state.seats.len(),MAX_SEATS);
        assert!(!state.add_seat(0),"duplicate global must not rebind a seat");
        assert!(!state.add_seat(MAX_SEATS as u32));
        assert!(state.overflow,"a partial seat inventory must be rejected");
        assert_eq!(state.seats.len(),MAX_SEATS);
    }
    #[test] fn wayland_seat_metadata_is_not_an_input_stream() {
        let payload=Seat{id:7,name:"seat0".into(),keyboard:true};
        let raw=serde_json::to_vec(&payload).unwrap();
        let decoded:Seat=serde_json::from_slice(&raw).unwrap();
        assert_eq!(decoded.id,7);
        assert!(decoded.keyboard);
        assert_eq!(decoded.name,"seat0");
        assert!(serde_json::from_slice::<Seat>(
            br#"{"id":7,"name":"seat0","keyboard":true,"private_keys":[1,2]}"#
        ).is_err());
    }
}
