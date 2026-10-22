//! Curated layout names plus explicitly scoped ANSI physical-position emulation.
//! Recognizing an XKB layout name is not the same as faithfully emulating that layout.
use eframe::egui;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Mode { System, Emulated }

pub struct Layout {
    pub id: &'static str,
    pub name: &'static str,
    pub region: &'static str,
    /// ANSI unshifted physical rows, 10/10/10 positions. None = recognition only.
    pub rows: Option<[&'static str; 3]>,
}
// Non-ASCII or complex dead-key layouts must be driven through XKB/IME, not these tables.
// Not all layouts below can be physically emulated yet; the UI exposes that honestly.
pub const CATALOG: &[Layout] = &[
    Layout { id:"us", name:"US QWERTY", region:"English", rows:Some(["qwertyuiop","asdfghjkl;","zxcvbnm,./"]) },
    Layout { id:"us(dvorak)", name:"Dvorak", region:"English", rows:Some(["',.pyfgcrl","aoeuidhtns",";qjkxbmwvz"]) },
    Layout { id:"us(colemak)", name:"Colemak", region:"English", rows:Some(["qwfpgjluy;","arstdhneio","zxcvbkm,./"]) },
    Layout { id:"us(colemak_dh)", name:"Colemak-DH", region:"English", rows:Some(["qwfpbjluy;","arstgmneio","zxcdvkh,./"]) },
    Layout { id:"us(workman)", name:"Workman", region:"English", rows:None },
    Layout { id:"us(norman)", name:"Norman", region:"English", rows:None },
    Layout { id:"us(halmak)", name:"Halmak", region:"English", rows:None },
    Layout { id:"us(semimak)", name:"Semimak", region:"English", rows:None },
    Layout { id:"us(apt)", name:"APT", region:"English", rows:None },
    Layout { id:"us(canary)", name:"Canary", region:"English", rows:None },
    Layout { id:"us(engram)", name:"Engram", region:"English", rows:None },
    Layout { id:"us(nerps)", name:"Nerps", region:"English", rows:None },
    Layout { id:"gb", name:"UK", region:"English", rows:None },
    Layout { id:"de", name:"German QWERTZ", region:"German", rows:None },
    Layout { id:"de(neo)", name:"Neo 2", region:"German", rows:None },
    Layout { id:"de(adnw)", name:"AdNW", region:"German", rows:None },
    Layout { id:"fr", name:"French AZERTY", region:"French", rows:None },
    Layout { id:"fr(bepo)", name:"BÉPO", region:"French", rows:None },
    Layout { id:"es", name:"Spanish", region:"Spanish", rows:None },
    Layout { id:"latam", name:"Latin American Spanish", region:"Spanish", rows:None },
    Layout { id:"it", name:"Italian", region:"Italian", rows:None },
    Layout { id:"pt", name:"Portuguese", region:"Portuguese", rows:None },
    Layout { id:"br", name:"Brazilian ABNT2", region:"Portuguese", rows:None },
    Layout { id:"se", name:"Swedish", region:"Nordic", rows:None },
    Layout { id:"no", name:"Norwegian", region:"Nordic", rows:None },
    Layout { id:"dk", name:"Danish", region:"Nordic", rows:None },
    Layout { id:"fi", name:"Finnish", region:"Nordic", rows:None },
    Layout { id:"is", name:"Icelandic", region:"Nordic", rows:None },
    Layout { id:"nl", name:"Dutch", region:"Europe", rows:None },
    Layout { id:"be", name:"Belgian", region:"Europe", rows:None },
    Layout { id:"ch", name:"Swiss", region:"Europe", rows:None },
    Layout { id:"pl", name:"Polish", region:"Europe", rows:None },
    Layout { id:"cz", name:"Czech", region:"Europe", rows:None },
    Layout { id:"hu", name:"Hungarian", region:"Europe", rows:None },
    Layout { id:"tr", name:"Turkish Q", region:"Europe", rows:None },
    Layout { id:"tr(f)", name:"Turkish F", region:"Europe", rows:None },
    Layout { id:"ru", name:"Russian ЙЦУКЕН", region:"Cyrillic", rows:None },
    Layout { id:"ua", name:"Ukrainian", region:"Cyrillic", rows:None },
    Layout { id:"bg", name:"Bulgarian", region:"Cyrillic", rows:None },
    Layout { id:"gr", name:"Greek", region:"Greek", rows:None },
    Layout { id:"il", name:"Hebrew", region:"Hebrew", rows:None },
    Layout { id:"ara", name:"Arabic", region:"Arabic", rows:None },
    Layout { id:"ir", name:"Persian", region:"Arabic", rows:None },
    Layout { id:"in", name:"Indian", region:"Indic", rows:None },
    Layout { id:"th", name:"Thai Kedmanee", region:"Southeast Asia", rows:None },
    Layout { id:"vn", name:"Vietnamese", region:"Southeast Asia", rows:None },
    Layout { id:"jp", name:"Japanese", region:"East Asia", rows:None },
    Layout { id:"kr", name:"Korean", region:"East Asia", rows:None },
];
pub fn find(id: &str) -> Option<&'static Layout> {
    CATALOG.iter().find(|l| l.id == id)
}
pub fn get(id: &str) -> &'static Layout {
    find(id).unwrap_or(&CATALOG[0])
}
pub fn physical_index(key: egui::Key) -> Option<(usize, usize)> {
    const TOP: &str = "qwertyuiop";
    const HOME: &str = "asdfghjkl;";
    const BOTTOM: &str = "zxcvbnm,./";
    let ch = match key {
        egui::Key::Semicolon => ';', egui::Key::Comma => ',',
        egui::Key::Period => '.', egui::Key::Slash => '/',
        other => {
            let name = format!("{other:?}");
            if name.len() != 1 { return None; }
            name.chars().next()?.to_ascii_lowercase()
        }
    };
    [TOP, HOME, BOTTOM].iter().enumerate().find_map(|(row, keys)| keys.chars().position(|k| k == ch).map(|col| (row, col)))
}
pub fn emulate(key: egui::Key, id: &str, shift: bool) -> Option<char> {
    if key == egui::Key::Space { return Some(' '); }
    let (row, col) = physical_index(key)?;
    let mapped = find(id)?.rows?[row].chars().nth(col)?;
    Some(if shift { shifted(mapped) } else { mapped })
}
fn shifted(ch: char) -> char {
    match ch {
        ';'=>':', ','=>'<', '.'=>'>', '/'=>'?', '\''=>'"',
        c if c.is_ascii_alphabetic()=>c.to_ascii_uppercase(),
        c=>c,
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn catalog_has_unique_ids() {
        let mut ids = std::collections::HashSet::new();
        for layout in CATALOG { assert!(ids.insert(layout.id)); }
    }
    #[test] fn emulation_has_correct_core_examples() {
        assert_eq!(emulate(egui::Key::E, "us(colemak)", false), Some('f'));
        assert_eq!(emulate(egui::Key::Q, "us(dvorak)", false), Some('\''));
        assert_eq!(emulate(egui::Key::E, "us", true), Some('E'));
        assert_eq!(emulate(egui::Key::Space, "us(colemak_dh)", false), Some(' '));
    }
    #[test] fn unknown_layout_never_silently_emulates_us() {
        assert_eq!(emulate(egui::Key::Q, "unknown-layout", false), None);
    }
    #[test] fn recognition_is_not_false_emulation() {
        assert_eq!(emulate(egui::Key::Q, "ara", false), None);
    }
}
