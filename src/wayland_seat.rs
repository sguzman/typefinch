//! Read-only native Wayland wl_seat capability enumeration.
//! Never requests wl_keyboard, maps keys, or attributes physical devices.
use serde::{Deserialize,Serialize};
use std::collections::BTreeMap;
use wayland_client::{
    protocol::{wl_registry,wl_seat},
    Connection,Dispatch,QueueHandle,WEnum,
};

#[derive(Clone,Debug,Serialize,Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Seat {
    pub id:u32,
    pub name:String,
    pub keyboard:bool,
}
const MAX_SEATS:usize=128;
#[derive(Default)]
struct State {
    seats:BTreeMap<u32,Seat>,
    overflow:bool,
}
impl State {
    fn add_seat(&mut self,id:u32)->bool {
        if self.seats.contains_key(&id) {return false;}
        if self.seats.len()>=MAX_SEATS {
            self.overflow=true;
            return false;
        }
        self.seats.insert(id,Seat{
            id,name:format!("unnamed-{id}"),keyboard:false
        });
        true
    }
}
impl Dispatch<wl_registry::WlRegistry,()> for State {
    fn event(state:&mut Self,registry:&wl_registry::WlRegistry,
        event:wl_registry::Event,_:&(),_:&Connection,qh:&QueueHandle<Self>) {
        match event {
            wl_registry::Event::Global{name,interface,version}
                if interface=="wl_seat" && version>=1=>{
                // An oversized registry must fail closed, never silently
                // return the first N seats as though the list were complete.
                if state.add_seat(name) {
                    // Version 2 exposes compositor-assigned seat names.
                    // Version 1 exposes capability only, never physical identity.
                    registry.bind::<wl_seat::WlSeat,_,_>(name,version.min(2),qh,name);
                }
            },
            wl_registry::Event::GlobalRemove{name}=>{
                state.seats.remove(&name);
            },
            _=>{},
        }
    }
}
impl Dispatch<wl_seat::WlSeat,u32> for State {
    fn event(state:&mut Self,_:&wl_seat::WlSeat,
        event:wl_seat::Event,id:&u32,_:&Connection,_:&QueueHandle<Self>) {
        let Some(seat)=state.seats.get_mut(id) else{return;};
        match event {
            wl_seat::Event::Capabilities{capabilities}=>{
                seat.keyboard=matches!(capabilities,
                    WEnum::Value(c) if c.contains(wl_seat::Capability::Keyboard));
            },
            wl_seat::Event::Name{name}=>{
                if !name.is_empty() && name.len()<=128
                    && !name.chars().any(char::is_control) {
                    seat.name=name;
                }
            },
            _=>{},
        }
    }
}
/// Called only by a short-lived internal subprocess with a strict parent
/// timeout. Never binds wl_keyboard or subscribes to input events.
pub fn read_only_seats()->Result<Vec<Seat>,String>{
    let conn=Connection::connect_to_env()
        .map_err(|e|format!("Wayland compositor connection unavailable: {e}"))?;
    let mut queue=conn.new_event_queue::<State>();
    let qh=queue.handle();
    let _registry=conn.display().get_registry(&qh,());
    let mut state=State::default();
    queue.roundtrip(&mut state)
        .map_err(|e|format!("Wayland registry inspection failed: {e}"))?;
    queue.roundtrip(&mut state)
        .map_err(|e|format!("Wayland seat capability inspection failed: {e}"))?;
    if state.overflow {
        return Err("Wayland advertised more than 128 seat globals; refusing truncated inventory.".into());
    }
    Ok(state.seats.into_values().collect())
}
#[cfg(test)]
mod tests {
    use super::*;
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
