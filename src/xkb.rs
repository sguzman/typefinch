//! Read-only XKB keymap compilation: use system layouts for in-app emulation.
//! This is not privileged input access and never changes the compositor keymap.
use eframe::egui::Key;
use xkbcommon::xkb;

pub struct Emulator { keymap: xkb::Keymap }

impl Emulator {
    pub fn compile(identity: &str) -> Result<Self, String> {
        // Named variants from the static catalog use layout(variant) syntax.
        let (layout, variant) = if let Some((main, remainder)) = identity.split_once('(') {
            (main, remainder.strip_suffix(')').ok_or("Invalid XKB variant")?)
        } else {
            (identity, "")
        };
        let context = xkb::Context::new(xkb::CONTEXT_NO_FLAGS);
        let keymap = xkb::Keymap::new_from_names(
            &context, "", "pc105", layout, variant, Some(String::new()), xkb::COMPILE_NO_FLAGS
        ).ok_or_else(|| format!("XKB layout {identity} is unavailable"))?;
        Ok(Self { keymap })
    }
    /// Translate a physical key position. Does not capture the keyboard globally.
    /// Modifier state is constructed per key; dead-key/Compose sequences are future work.
    pub fn translate(&self, physical: Key, shift: bool) -> Option<String> {
        let evdev = evdev_code(physical)?;
        let mut state = xkb::State::new(&self.keymap);
        if shift { state.update_key(xkb::Keycode::new(42 + 8), xkb::KEY_DOWN); }
        let result = state.key_get_utf8(xkb::Keycode::new(evdev + 8));
        if result.is_empty() || result.chars().any(char::is_control) { None } else { Some(result) }
    }
}
/// Common ANSI key positions; ISO/JIS extra key positions have separate roadmap gates.
/// Linux evdev keycodes + 8 are XKB keycodes.
fn evdev_code(key: Key) -> Option<u32> {
    const ROWS: [[u32;10];3] = [
        [16,17,18,19,20,21,22,23,24,25],
        [30,31,32,33,34,35,36,37,38,39],
        [44,45,46,47,48,49,50,51,52,53],
    ];
    if let Some((r,c)) = crate::layouts::physical_index(key) { return Some(ROWS[r][c]); }
    Some(match key {
        Key::Space => 57,
        Key::Num1=>2,Key::Num2=>3,Key::Num3=>4,Key::Num4=>5,Key::Num5=>6,
        Key::Num6=>7,Key::Num7=>8,Key::Num8=>9,Key::Num9=>10,Key::Num0=>11,
        Key::Minus=>12,Key::Equals=>13,Key::OpenBracket=>26,Key::CloseBracket=>27,
        Key::Quote=>40,Key::Backtick=>41,Key::Backslash=>43,
        _ => return None
    })
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn common_system_xkb_layouts_compile() {
        for id in ["us", "de", "fr", "es", "latam", "ru", "gr", "ara", "il"] {
            assert!(Emulator::compile(id).is_ok(), "{id} did not compile");
        }
    }
    #[test] fn compiled_variants_have_expected_letters() {
        let us=Emulator::compile("us").unwrap();
        let de=Emulator::compile("de").unwrap();
        assert_eq!(us.translate(Key::Y,false).as_deref(),Some("y"));
        assert_eq!(us.translate(Key::Y,true).as_deref(),Some("Y"));
        assert_eq!(de.translate(Key::Y,false).as_deref(),Some("z"));
        let colemak=Emulator::compile("us(colemak)").unwrap();
        assert_eq!(colemak.translate(Key::E,false).as_deref(),Some("f"));
    }
    #[test] fn not_installed_layout_fails_explicitly() {
        assert!(Emulator::compile("zz_no_such_layout").is_err());
    }
}
