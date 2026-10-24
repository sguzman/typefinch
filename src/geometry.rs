//! Immutable preview geometry for common physical keyboard families.
//! These are explicit *illustrations*, not detected hardware shapes. An
//! egui physical key event cannot yet prove ISO/JIS extra-key attribution.
use eframe::egui::Key;

/// Rendered size of one tenth-key geometry unit. At this scale even
/// the widest split main row stays within a typical 1100px QA window.
pub const PREVIEW_UNIT_PX:f32=3.8;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Shape { Ansi, Iso, Jis, Split }
impl Shape {
    pub const ALL: [Self;4] = [Self::Ansi, Self::Iso, Self::Jis, Self::Split];
    pub const fn label(self) -> &'static str {
        match self {
            Self::Ansi => "ANSI main block",
            Self::Iso => "ISO main block",
            Self::Jis => "JIS main block",
            Self::Split => "Split main block",
        }
    }
    pub const fn next(self) -> Self {
        match self {
            Self::Ansi=>Self::Iso,
            Self::Iso=>Self::Jis,
            Self::Jis=>Self::Split,
            Self::Split=>Self::Ansi,
        }
    }
    pub const fn note(self) -> &'static str {
        match self {
            Self::Ansi => "Common ANSI positions (not a full numeric keypad)",
            Self::Iso => "ISO extra key (102nd) shown separately from regular backslash",
            Self::Jis => "JIS extra Ro/Yen and conversion keys; symbols depend on installed XKB",
            Self::Split => "Split illustration retains ANSI key identities; gap is visual only",
        }
    }
}

/// Which physical-key *reference* region is illustrated. Non-main
/// sections have no scoring or compositor event attribution.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Section { Typing, Navigation, Keypad }
impl Section {
    pub const ALL:[Self;3]=[Self::Typing,Self::Navigation,Self::Keypad];
    pub const fn label(self)->&'static str {
        match self {Self::Typing=>"Typing block",Self::Navigation=>"Function / navigation",
                    Self::Keypad=>"Numeric keypad"}
    }
    pub const fn next(self)->Self {
        match self {Self::Typing=>Self::Navigation,Self::Navigation=>Self::Keypad,
                    Self::Keypad=>Self::Typing}
    }
}
/// Units are tenths of one ordinary key width. No slot claims ownership of
/// real hardware events; `egui_key=None` must not be synthesized from another key.
/// Presentation semantics, never inferred from XKB modifier symbols.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SlotKind { Character, Extra, Control, Modifier, Decoration }
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Slot {
    pub name: &'static str,
    pub evdev: u32,
    pub egui_key: Option<Key>,
    pub row: u8,
    pub x: u16,
    pub width: u8,
    pub kind: SlotKind,
    pub caption: &'static str,
}
fn position(name: &'static str, evdev: u32, key: Option<Key>, row:u8, x:u16, width:u8) -> Slot {
    let kind=if evdev==0 {SlotKind::Decoration}
        else if key.is_some() {SlotKind::Character} else {SlotKind::Extra};
    Slot { name, evdev, egui_key:key, row, x, width, kind, caption:"" }
}
/// Visible fallback labels for dedicated national input keys that have no
/// printable XKB symbol. These are not interpreted as scored key events.
fn named_extra(name:&'static str, caption:&'static str, evdev:u32,
    row:u8, x:u16, width:u8)->Slot {
    let mut slot=position(name,evdev,None,row,x,width);
    slot.caption=caption;
    slot
}
fn special(name:&'static str, caption:&'static str, evdev:u32,
    row:u8, x:u16, width:u8, modifier:bool)->Slot {
    let mut slot=position(name,evdev,None,row,x,width);
    slot.kind=if modifier {SlotKind::Modifier} else {SlotKind::Control};
    slot.caption=caption;
    slot
}
const NUMBER: &[(Key,u32,&str)] = &[
    (Key::Backtick,41,"Grave"),
    (Key::Num1,2,"1"), (Key::Num2,3,"2"), (Key::Num3,4,"3"),
    (Key::Num4,5,"4"), (Key::Num5,6,"5"), (Key::Num6,7,"6"),
    (Key::Num7,8,"7"), (Key::Num8,9,"8"), (Key::Num9,10,"9"),
    (Key::Num0,11,"0"), (Key::Minus,12,"Minus"), (Key::Equals,13,"Equals"),
];
const UPPER: &[(Key,u32,&str)] = &[
    (Key::Q,16,"Q"), (Key::W,17,"W"), (Key::E,18,"E"),
    (Key::R,19,"R"), (Key::T,20,"T"), (Key::Y,21,"Y"),
    (Key::U,22,"U"), (Key::I,23,"I"), (Key::O,24,"O"),
    (Key::P,25,"P"), (Key::OpenBracket,26,"Left bracket"),
    (Key::CloseBracket,27,"Right bracket"), (Key::Backslash,43,"Backslash"),
];
const HOME: &[(Key,u32,&str)] = &[
    (Key::A,30,"A"), (Key::S,31,"S"), (Key::D,32,"D"),
    (Key::F,33,"F"), (Key::G,34,"G"), (Key::H,35,"H"),
    (Key::J,36,"J"), (Key::K,37,"K"), (Key::L,38,"L"),
    (Key::Semicolon,39,"Semicolon"), (Key::Quote,40,"Quote"),
];
const LOWER: &[(Key,u32,&str)] = &[
    (Key::Z,44,"Z"), (Key::X,45,"X"), (Key::C,46,"C"),
    (Key::V,47,"V"), (Key::B,48,"B"), (Key::N,49,"N"),
    (Key::M,50,"M"), (Key::Comma,51,"Comma"),
    (Key::Period,52,"Period"), (Key::Slash,53,"Slash"),
];
pub fn slots(shape:Shape) -> Vec<Slot> {
    let mut result=Vec::with_capacity(60);
    for (row, offset, keys) in [
        (0,0,NUMBER),(1,15,UPPER),(2,20,HOME),(3,30,LOWER),
    ] {
        for (i, &(key,evdev,name)) in keys.iter().enumerate() {
            let gap = if shape==Shape::Split && row>0 && i>=5 {28} else {0};
            let x=offset + i as u16*10+gap;
            if shape==Shape::Jis && evdev==41 {
                // Japanese 109-key references use KEY_ZENKAKUHANKAKU (85)
                // in the top-left ANSI-Grave position. It is a mode key,
                // never a guessed printable backtick or scored text event.
                result.push(special("Zenkaku/Hankaku","Zen",85,row,x,10,false));
            } else {
                result.push(position(name,evdev,Some(key),row,x,10));
            }
        }
    }
    if shape==Shape::Iso {
        // Linux KEY_102ND (86); left of Z, below the shorter ISO left Shift.
        result.push(named_extra("ISO 102nd","ISO",86,3,20,10));
    }
    if shape==Shape::Jis {
        // The Japanese-specific Linux input-event-codes.h assignments.
        result.extend([
            named_extra("JIS Yen","Yen",124,0,140,10),
            named_extra("JIS Ro","Ro",89,3,130,10),
            named_extra("Muhenkan","Non",94,4,35,12),
            named_extra("Henkan","Conv",92,4,109,12),
            named_extra("Kana","Kana",93,4,122,12),
        ]);
    }
    // Controls and modifiers are illustrated physical slots only. They are
    // never translated into scored text or left/right-specific key events.
    let jis=shape==Shape::Jis;
    let split=shape==Shape::Split;
    result.extend([
        special("Backspace","Bksp",14,0,if jis {150} else {130},20,false),
        special("Tab","Tab",15,1,0,15,false),
        special("Caps Lock","Caps",58,2,0,20,true),
        special("Enter","Enter",28,2,if shape==Shape::Split {158} else {130},
            if jis {25} else if split {20} else {22},false),
        special("Left Shift","Shift",42,3,0,if shape==Shape::Iso {20} else {30},true),
        special("Right Shift","Shift",54,3,
            if split {158} else if jis {140} else {130},
            if split {18} else {22},true),
        special("Left Ctrl","Ctrl",29,4,0,12,true),
        special("Left Alt","Alt",56,4,if jis {24} else {29},if jis {10} else {13},true),
        special("Right Alt (AltGr)","RAlt",100,4,if jis {135} else {119},
            if jis {10} else {12},true),
        special("Right Ctrl","Ctrl",97,4,if jis {168} else {158},
            if jis {10} else {12},true),
    ]);
    // JIS 109 (unlike older 106) has both Super keys and an application
    // Menu key. The complete bottom row is a physical *reference*, not an
    // attempt to synthesize OS shortcuts or conversion-key input events.
    result.extend([
        special("Left Super","◆",125,4,if jis {13} else {14},
            if jis {10} else {12},true),
        special("Right Super","◆",126,4,if jis {146} else {132},
            if jis {10} else {12},true),
        special("Menu","Menu",127,4,if jis {157} else {145},
            if jis {10} else {12},false),
    ]);
    // The ordinary physical Space is longer on ANSI/ISO; shorter in a
    // JIS converter-key row, and shown as two *visual halves* on a split.
    if shape==Shape::Split {
        result.push(position("Space (left visual half)",57,Some(Key::Space),4,45,22));
        // A deliberately non-input display slot; never double count Space.
        result.push(position("Space (right visual half)",0,None,4,94,22));
    } else {
        let (x,width) = if shape==Shape::Jis {(48,60)} else {(45,72)};
        result.push(position("Space",57,Some(Key::Space),4,x,width));
    }
    result.sort_by_key(|slot|(slot.row,slot.x));
    result
}
/// Standard Linux evdev reference positions; these keys are never
/// attributed to a connected keyboard, synthesized, or scored. Main typing
/// geometry is deliberately unchanged for the existing QA corpus.
pub fn section_slots(shape:Shape,section:Section)->Vec<Slot> {
    if section==Section::Typing {return slots(shape);}
    let mut out=Vec::with_capacity(30);
    match section {
        Section::Typing=>unreachable!(),
        Section::Navigation=>{
            out.push(special("Escape","Esc",1,0,0,12,false));
            for (index,evdev) in [59,60,61,62,63,64,65,66,67,68,87,88].into_iter().enumerate() {
                let x=20+index as u16*11+if index>=8 {12} else if index>=4 {6} else {0};
                let name=match index {
                    0=>"F1",1=>"F2",2=>"F3",3=>"F4",4=>"F5",5=>"F6",
                    6=>"F7",7=>"F8",8=>"F9",9=>"F10",10=>"F11",_=>"F12",
                };
                out.push(special(name,name,evdev,0,x,10,false));
            }
            for (name,caption,evdev,row,x) in [
                ("Print Screen","Prt",99,1,0),
                ("Scroll Lock","ScrLk",70,1,14),
                ("Pause","Pause",119,1,28),
                ("Insert","Ins",110,1,70),
                ("Home","Home",102,1,82),
                ("Page Up","PgUp",104,1,94),
                ("Delete","Del",111,2,70),
                ("End","End",107,2,82),
                ("Page Down","PgDn",109,2,94),
                ("Up arrow","Up",103,3,82),
                ("Left arrow","Left",105,4,70),
                ("Down arrow","Down",108,4,82),
                ("Right arrow","Right",106,4,94),
            ] {out.push(special(name,caption,evdev,row,x,11,false));}
        },
        Section::Keypad=>{
            for (name,caption,evdev,row,x,width) in [
                ("Num Lock","Num",69,0,0,12),
                ("Keypad divide","/",98,0,13,12),
                ("Keypad multiply","*",55,0,26,12),
                ("Keypad minus","-",74,0,39,12),
                ("Keypad 7","7",71,1,0,12),
                ("Keypad 8","8",72,1,13,12),
                ("Keypad 9","9",73,1,26,12),
                ("Keypad plus","+",78,1,39,12),
                ("Keypad 4","4",75,2,0,12),
                ("Keypad 5","5",76,2,13,12),
                ("Keypad 6","6",77,2,26,12),
                ("Keypad 1","1",79,3,0,12),
                ("Keypad 2","2",80,3,13,12),
                ("Keypad 3","3",81,3,26,12),
                ("Keypad Enter","Enter",96,3,39,12),
                ("Keypad 0","0",82,4,0,25),
                ("Keypad decimal",".",83,4,26,12),
            ] {out.push(special(name,caption,evdev,row,x,width,false));}
        }
    }
    out.sort_by_key(|slot|(slot.row,slot.x));
    out
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn reference_sections_have_real_codes_but_never_claim_event_capture() {
        use std::collections::HashSet;
        assert_eq!(Section::Typing.next(),Section::Navigation);
        assert_eq!(Section::Navigation.next(),Section::Keypad);
        assert_eq!(Section::Keypad.next(),Section::Typing);
        for shape in Shape::ALL {
            for (section,expected_count) in [(Section::Navigation,26),(Section::Keypad,17)] {
                let slots=section_slots(shape,section);
                assert_eq!(slots.len(),expected_count);
                let mut seen=HashSet::new();
                for slot in &slots {
                    assert_eq!(slot.kind,SlotKind::Control);
                    assert!(slot.egui_key.is_none());
                    assert!(seen.insert(slot.evdev),"duplicate key {} in {section:?}",slot.name);
                    assert!(!slot.caption.is_empty());
                }
                for row in 0..=4 {
                    let keys:Vec<_>=slots.iter().filter(|s|s.row==row).collect();
                    for pair in keys.windows(2) {
                        assert!(pair[0].x+u16::from(pair[0].width)<=pair[1].x,
                            "{section:?}: {} overlaps {}",pair[0].name,pair[1].name);
                    }
                }
            }
        }
        assert_eq!(section_slots(Shape::Iso,Section::Typing),slots(Shape::Iso));
    }
    #[test] fn extension_has_stable_linux_evdev_codes() {
        let nav=section_slots(Shape::Ansi,Section::Navigation);
        for (name,code) in [("Escape",1),("F1",59),("F10",68),("F11",87),
                            ("F12",88),("Up arrow",103),("Delete",111)] {
            assert_eq!(nav.iter().find(|s|s.name==name).unwrap().evdev,code);
        }
        let keypad=section_slots(Shape::Ansi,Section::Keypad);
        for (name,code) in [("Num Lock",69),("Keypad 0",82),("Keypad decimal",83),
                            ("Keypad Enter",96),("Keypad divide",98)] {
            assert_eq!(keypad.iter().find(|s|s.name==name).unwrap().evdev,code);
        }
    }
    #[test] fn geometry_shortcut_cycle_never_changes_input_keymap() {
        let mut shape=Shape::Ansi;
        for expected in [Shape::Iso,Shape::Jis,Shape::Split,Shape::Ansi] {
            shape=shape.next();
            assert_eq!(shape,expected);
        }
    }
    #[test] fn main_block_preview_row_fits_native_qa_center_pane() {
        // This is a model bound, not a replacement for a rendered screenshot.
        for shape in Shape::ALL {
            let max_units=slots(shape).iter().map(|s|s.x + u16::from(s.width)).max().unwrap();
            assert!((max_units as f32 * PREVIEW_UNIT_PX) <= 690.0,
                "{shape:?} exceeds native QA geometry space");
        }
    }
    #[test] fn every_geometry_has_unique_real_evdev_positions() {
        for shape in Shape::ALL {
            let slots=slots(shape);
            let mut codes=std::collections::HashSet::new();
            for slot in &slots {
                assert!(slot.width>0);
                assert!(slot.x<200);
                if slot.evdev!=0 {
                    assert!(codes.insert(slot.evdev), "duplicate evdev {} in {:?}",slot.evdev,shape);
                }
                if slot.kind!=SlotKind::Character {
                    assert!(slot.egui_key.is_none(), "non-printable key event: {}",slot.name);
                }
                if slot.kind==SlotKind::Decoration {assert_eq!(slot.evdev,0);}
            }
            for row in 0..=4 {
                let keys:Vec<_>=slots.iter().filter(|s|s.row==row).collect();
                for pair in keys.windows(2) {
                    assert!(pair[0].x+pair[0].width as u16<=pair[1].x,
                        "{shape:?}: {} overlaps {}",pair[0].name,pair[1].name);
                }
            }
            assert_eq!(slots.iter().filter(|s|s.egui_key==Some(Key::Space)).count(),1);
        }
    }
    #[test] fn full_ansi_iso_jis_and_split_reference_inventories_are_exact() {
        use std::collections::HashSet;
        // Count *unique physical Linux key codes*, never graphical halves,
        // never independently numbered control rows or imagined hotplug
        // devices. ANSI 104 / ISO 105 / JIS 109 include the navigation and
        // numeric key blocks shown through the preview's section selector.
        for (shape,count) in [
            (Shape::Ansi,104_usize),(Shape::Iso,105),
            (Shape::Jis,109),(Shape::Split,104)
        ] {
            let sections=[
                section_slots(shape,Section::Typing),
                section_slots(shape,Section::Navigation),
                section_slots(shape,Section::Keypad),
            ];
            let mut physical=HashSet::new();
            let mut rendered=0;
            let mut decorative=0;
            for slots in &sections {
                rendered+=slots.len();
                for slot in slots {
                    assert!(slot.width>0 && !slot.name.is_empty());
                    if slot.evdev==0 {
                        decorative+=1;
                        assert_eq!(slot.kind,SlotKind::Decoration);
                        assert!(slot.egui_key.is_none());
                    }else{
                        assert!(physical.insert(slot.evdev),
                            "{shape:?}: duplicate physical evdev {} ({})",
                            slot.evdev,slot.name);
                        if slot.kind!=SlotKind::Character {
                            assert!(slot.egui_key.is_none(),
                                "reference control or national key emits text: {}",
                                slot.name);
                        }
                    }
                }
            }
            assert_eq!(physical.len(),count,
                "{shape:?}: incomplete family key inventory");
            assert_eq!(rendered,count+decorative);
            assert_eq!(decorative,if shape==Shape::Split {1}else{0},
                "split Space halves must never be two physical keys");
            for evdev in [
                1,14,15,28,29,42,54,56,58,97,100,125,126,127,
                57,69,82,83,96,98,99,119
            ] {
                assert!(physical.contains(&evdev),
                    "{shape:?}: missing function, control, modifier or keypad code {evdev}");
            }
            match shape {
                Shape::Ansi|Shape::Split=>{
                    assert!(physical.contains(&41));
                    assert!(!physical.contains(&86));
                    assert!(!physical.contains(&85));
                },
                Shape::Iso=>{
                    assert!(physical.contains(&41));
                    assert!(physical.contains(&86));
                },
                Shape::Jis=>{
                    assert!(!physical.contains(&41));
                    for evdev in [85,89,92,93,94,124] {
                        assert!(physical.contains(&evdev),
                            "JIS 109 special evdev {evdev} missing");
                    }
                },
            }
        }
    }
    #[test] fn modifier_and_control_positions_are_explicit_not_scored() {
        for shape in Shape::ALL {
            let slots=slots(shape);
            for code in [14,15,28,29,42,54,56,58,97,100] {
                let slot=slots.iter().find(|s|s.evdev==code).expect("control position missing");
                assert!(matches!(slot.kind,SlotKind::Control|SlotKind::Modifier));
                assert!(slot.egui_key.is_none());
                assert!(!slot.caption.is_empty());
            }
            for code in [125,126,127] {
                assert_eq!(slots.iter().filter(|s|s.evdev==code).count(),1,
                    "all illustrated 104/105/109-key blocks retain Super/Menu keys");
            }
            assert_eq!(slots.iter().filter(|s|s.kind==SlotKind::Extra).count(),
                match shape { Shape::Iso=>1,Shape::Jis=>5,_=>0 });
        }
    }
    #[test] fn jis_109_bottom_row_includes_windows_menu_and_conversion_keys() {
        let slots=slots(Shape::Jis);
        let order=[29,125,56,94,57,92,93,100,126,127,97];
        let positions:Vec<u32>=slots.iter().filter(|s|s.row==4)
            .map(|s|s.evdev).collect();
        assert_eq!(positions,order,
            "JIS 109 bottom-row geometry follows Ctrl/Win/Alt/NonConvert/Space/Convert/Kana/Alt/Win/Menu/Ctrl");
        for code in [125,126,127,92,93,94] {
            assert!(slots.iter().any(|s|s.evdev==code && s.egui_key.is_none()));
        }
        assert_eq!(slots.iter().filter(|s|s.egui_key==Some(Key::Space)).count(),1);
    }
    #[test] fn iso_and_jis_extra_keys_have_honest_readable_fallback_labels(){
        for (shape,specs) in [
            (Shape::Iso,vec![(86,"ISO")]),
            (Shape::Jis,vec![(124,"Yen"),(89,"Ro"),(94,"Non"),(92,"Conv"),(93,"Kana")]),
        ] {
            let slots=slots(shape);
            for (evdev,caption) in specs {
                let key=slots.iter().find(|s|s.evdev==evdev).unwrap();
                assert_eq!(key.kind,SlotKind::Extra);
                assert_eq!(key.caption,caption);
                assert!(key.egui_key.is_none(),"national edge keys cannot fabricate key events");
            }
        }
        for shape in Shape::ALL {
            let right_alt=slots(shape).into_iter().find(|s|s.evdev==100).unwrap();
            assert_eq!(right_alt.caption,"RAlt",
                "right Alt position is shown without claiming an AltGr event");
        }
    }
    #[test] fn jis_conversion_keys_have_visible_fallback_labels_without_inventing_input() {
        let slots=slots(Shape::Jis);
        for (name,caption) in [("Muhenkan","Non"),("Henkan","Conv"),("Kana","Kana")] {
            let slot=slots.iter().find(|s|s.name==name).unwrap();
            assert_eq!(slot.kind,SlotKind::Extra);
            assert_eq!(slot.caption,caption);
            assert!(slot.egui_key.is_none());
        }
    }
    #[test] fn japanese_top_left_is_zenkaku_not_fake_ansi_grave() {
        let jis=slots(Shape::Jis);
        let zen=jis.iter().find(|s|s.evdev==85).expect("missing Zenkaku/Hankaku");
        assert_eq!(zen.kind,SlotKind::Control);
        assert_eq!(zen.caption,"Zen");
        assert!(zen.egui_key.is_none());
        assert_eq!(zen.row,0);
        assert_eq!(zen.x,0);
        assert!(!jis.iter().any(|s|s.evdev==41));
        let ansi=slots(Shape::Ansi);
        assert_eq!(ansi.iter().find(|s|s.evdev==41).unwrap().egui_key,Some(Key::Backtick));
    }
    #[test] fn iso_and_jis_extras_do_not_fake_egui_key_events() {
        let ansi=slots(Shape::Ansi);
        let iso=slots(Shape::Iso);
        let jis=slots(Shape::Jis);
        assert_eq!(iso.len(),ansi.len()+1);
        let extra=iso.iter().find(|slot|slot.evdev==86).unwrap();
        assert_eq!(extra.name,"ISO 102nd");
        assert_eq!(extra.egui_key,None);
        for code in [89,92,93,94,124] {
            let slot=jis.iter().find(|slot|slot.evdev==code).unwrap();
            assert!(slot.egui_key.is_none());
            assert!(ansi.iter().all(|s|s.evdev!=code));
        }
    }
    #[test] fn split_is_preview_spacing_not_doubled_input() {
        let a=slots(Shape::Ansi);
        let b=slots(Shape::Split);
        for original in a.iter().filter(|s|s.kind==SlotKind::Character && s.row>0 && s.row<4) {
            let moved=b.iter().find(|s|s.evdev==original.evdev).unwrap();
            assert_eq!(moved.x-original.x,if original.x >= match original.row {1=>65,2=>70,_=>80} {28} else {0});
        }
        assert_eq!(b.iter().filter(|s|s.evdev==57).count(),1);
    }
}
