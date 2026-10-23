//! Immutable preview geometry for common physical keyboard families.
//! These are explicit *illustrations*, not detected hardware shapes. An
//! egui physical key event cannot yet prove ISO/JIS extra-key attribution.
use eframe::egui::Key;

/// Rendered size of one tenth-key geometry unit. At this scale even
/// the widest split main row stays within a typical 1100px QA window.
pub const PREVIEW_UNIT_PX:f32=3.9;

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
    pub const fn note(self) -> &'static str {
        match self {
            Self::Ansi => "Common ANSI positions (not a full numeric keypad)",
            Self::Iso => "ISO extra key (102nd) shown separately from regular backslash",
            Self::Jis => "JIS extra Ro/Yen and conversion keys; symbols depend on installed XKB",
            Self::Split => "Split illustration retains ANSI key identities; gap is visual only",
        }
    }
}

/// Units are tenths of one ordinary key width. No slot claims ownership of
/// real hardware events; `egui_key=None` must not be synthesized from another key.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Slot {
    pub name: &'static str,
    pub evdev: u32,
    pub egui_key: Option<Key>,
    pub row: u8,
    pub x: u16,
    pub width: u8,
}
fn position(name: &'static str, evdev: u32, key: Option<Key>, row:u8, x:u16, width:u8) -> Slot {
    Slot { name, evdev, egui_key:key, row, x, width }
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
            result.push(position(name,evdev,Some(key),row,offset + i as u16*10+gap,10));
        }
    }
    if shape==Shape::Iso {
        // Linux KEY_102ND (86); left of Z, below the shorter ISO left Shift.
        result.push(position("ISO 102nd",86,None,3,20,10));
    }
    if shape==Shape::Jis {
        // The Japanese-specific Linux input-event-codes.h assignments.
        result.extend([
            position("JIS Yen",124,None,0,140,10),
            position("JIS Ro",89,None,3,130,10),
            position("Muhenkan",94,None,4,20,12),
            position("Henkan",92,None,4,116,12),
            position("Kana",93,None,4,130,12),
        ]);
    }
    // The ordinary physical Space is longer on ANSI/ISO; shorter in a
    // JIS converter-key row, and shown as two *visual halves* on a split.
    if shape==Shape::Split {
        result.push(position("Space (left visual half)",57,Some(Key::Space),4,45,22));
        // A deliberately non-input display slot; never double count Space.
        result.push(position("Space (right visual half)",0,None,4,94,22));
    } else {
        let (x,width) = if shape==Shape::Jis {(45,60)} else {(45,72)};
        result.push(position("Space",57,Some(Key::Space),4,x,width));
    }
    result.sort_by_key(|slot|(slot.row,slot.x));
    result
}
#[cfg(test)]
mod tests {
    use super::*;
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
            }
            assert_eq!(slots.iter().filter(|s|s.egui_key==Some(Key::Space)).count(),1);
        }
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
        for original in a.iter().filter(|s|s.row>0 && s.row<4) {
            let moved=b.iter().find(|s|s.evdev==original.evdev).unwrap();
            assert_eq!(moved.x-original.x,if original.x >= match original.row {1=>65,2=>70,_=>80} {28} else {0});
        }
        assert_eq!(b.iter().filter(|s|s.evdev==57).count(),1);
    }
}
