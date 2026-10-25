-- Isolated egui 0.35 T303/T305 physical-key routing proof.
-- Applied after egui035_migration_probe.lua in a temporary CI checkout.
-- No global keyboard interception, saved source text, or production dependency change.
local function edit(path,changes)
    local file=assert(io.open(path,"rb"))
    local src=assert(file:read("*a")); assert(file:close())
    for _,change in ipairs(changes) do
        local old,new=change[1],change[2]
        local at=assert(src:find(old,1,true),"Missing unique physical-key probe anchor: "..path.." "..old:sub(1,80))
        assert(not src:find(old,at+#old,true),"Ambiguous physical-key probe anchor: "..path.." "..old:sub(1,80))
        src=src:sub(1,at-1)..new..src:sub(at+#old)
    end
    local out=assert(io.open(path,"wb"))
    assert(out:write(src));assert(out:close())
end
edit("src/xkb.rs",{
    {[[Key::Quote=>40,Key::Backtick=>41,Key::Backslash=>43,]],
     [[Key::Quote=>40,Key::Backtick=>41,Key::Backslash=>43,
        Key::IntlBackslash=>86,]]},
    {[[    #[test] fn common_system_xkb_layouts_compile()]],
     [[    #[test] fn physical_iso_102nd_key_has_its_own_xkb_position() {
        let de=Emulator::compile("de").expect("installed German ISO layout");
        assert_eq!(evdev_code(Key::IntlBackslash),Some(86));
        assert_eq!(de.translate_layer(Key::IntlBackslash,false,false).as_deref(),Some("<"));
        assert_eq!(de.translate_layer(Key::IntlBackslash,true,false).as_deref(),Some(">"));
        assert_ne!(evdev_code(Key::IntlBackslash),evdev_code(Key::Backslash));
    }
    #[test] fn common_system_xkb_layouts_compile()]]}
})
edit("src/app.rs",{
    {[[    ime_preedit:String,
    installed_layouts:]],
     [[    ime_preedit:String,
    emulated_right_alt_held:bool,
    emulated_left_alt_held:bool,
    installed_layouts:]]},
    {[[composition:None, ime_preedit:String::new(), installed_layouts,]],
     [[composition:None, ime_preedit:String::new(),
             emulated_right_alt_held:false, emulated_left_alt_held:false, installed_layouts,]]},
    {[[fn pressed_key_modifiers(events:&[egui::Event],desired:egui::Key)->Option<egui::Modifiers> {]],
     [[/// An AltGr layer is enabled only by an expressly observed in-focus
/// physical right-Alt press. Modifiers.alt alone is NOT AltGr; left Alt,
/// Ctrl, command and auto-repeat do not score. No device is attributed.
fn emulated_level3_key_accepted(repeat:bool,modifiers:egui::Modifiers,
    right_alt:bool,left_alt:bool)->bool {
    !repeat && right_alt && !left_alt && !modifiers.ctrl && !modifiers.command
}
fn pressed_key_modifiers(events:&[egui::Event],desired:egui::Key)->Option<egui::Modifiers> {]]},
    {[[        self.ime_preedit.clear();
        if self.session.target.is_empty() {]],
     [[        self.ime_preedit.clear();
        self.emulated_right_alt_held=false;
        self.emulated_left_alt_held=false;
        if self.session.target.is_empty() {]]},
    {[[            self.ime_preedit.clear();
            if let Some(compose) = self.composition.as_mut() { compose.reset(); }]],
     [[            self.ime_preedit.clear();
            self.emulated_right_alt_held=false;
            self.emulated_left_alt_held=false;
            if let Some(compose) = self.composition.as_mut() { compose.reset(); }]]},
    {[[            match event {
                egui::Event::Key { key: egui::Key::Escape, pressed: true, .. } => {]],
     [[            match event {
                egui::Event::Key {key,physical_key,pressed,..}
                    if self.mode==Mode::Emulated
                    && (key==egui::Key::AltRight
                        || physical_key==Some(egui::Key::AltRight))=>{
                    self.emulated_right_alt_held=pressed;
                }
                egui::Event::Key {key,physical_key,pressed,..}
                    if self.mode==Mode::Emulated
                    && (key==egui::Key::AltLeft
                        || physical_key==Some(egui::Key::AltLeft))=>{
                    self.emulated_left_alt_held=pressed;
                }
                egui::Event::Key { key: egui::Key::Escape, pressed: true, .. } => {]]},
    {[[if self.mode == Mode::Emulated && emulated_key_accepted(repeat,modifiers) =>]],
     [[if self.mode == Mode::Emulated
                     && ((!self.emulated_left_alt_held && !self.emulated_right_alt_held
                         && emulated_key_accepted(repeat,modifiers))
                      || emulated_level3_key_accepted(repeat,modifiers,
                             self.emulated_right_alt_held,self.emulated_left_alt_held)) =>]]},
    {[[                    if let Some(physical) = physical_key.or(if key == egui::Key::Space { Some(key) } else { None }) {
                        let mapped = if let Some(personal)=self.prefs.custom_layouts.iter()
                            .find(|l|l.id==self.prefs.layout) {
                            personal.translate(physical,modifiers.shift)
                        } else if let Some(emulator) = &self.emulator {
                            if let Some(compose) = self.composition.as_mut() {
                                emulator.translate_composed(physical, modifiers.shift, compose)
                            } else {
                                emulator.translate(physical, modifiers.shift)
                            }
                        } else {
                            layouts::emulate(physical, &self.prefs.layout, modifiers.shift).map(|c|c.to_string())
                        };]],
     [[                    if let Some(physical) = physical_key.or(if key == egui::Key::Space { Some(key) } else { None }) {
                        let alt_gr=self.emulated_right_alt_held && !self.emulated_left_alt_held;
                        let mapped = if let Some(personal)=self.prefs.custom_layouts.iter()
                            .find(|l|l.id==self.prefs.layout) {
                            if alt_gr {None} else {personal.translate(physical,modifiers.shift)}
                        } else if let Some(emulator) = &self.emulator {
                            if let Some(compose) = self.composition.as_mut() {
                                emulator.translate_composed_layer(physical,modifiers.shift,alt_gr,compose)
                            } else {
                                emulator.translate_layer(physical,modifiers.shift,alt_gr)
                            }
                        } else if alt_gr {
                            None
                        } else {
                            layouts::emulate(physical, &self.prefs.layout, modifiers.shift).map(|c|c.to_string())
                        };]]},
    {[[    #[test] fn emulated_key_repeat_and_modifier_shortcuts_are_not_scored() {]],
     [[    #[test] fn explicit_right_alt_gates_emulated_level_three_without_left_alt_leak() {
        let plain=egui::Modifiers::default();
        let alt=egui::Modifiers{alt:true,..plain};
        assert!(!emulated_level3_key_accepted(false,alt,false,false));
        assert!(!emulated_level3_key_accepted(false,alt,true,true));
        assert!(emulated_level3_key_accepted(false,alt,true,false));
        assert!(emulated_level3_key_accepted(false,plain,true,false),
            "right Alt event itself, not a possibly collapsed modifier flag, authorizes Level 3");
        assert!(!emulated_level3_key_accepted(true,alt,true,false));
        assert!(!emulated_level3_key_accepted(false,
            egui::Modifiers{ctrl:true,..alt},true,false));
        assert!(!emulated_level3_key_accepted(false,
            egui::Modifiers{command:true,..alt},true,false));
        let de=Emulator::compile("de").unwrap();
        let mut state=de.compose_state().unwrap();
        let mut session=Session::new_with_custom(30,TestMode::Custom,"e€@e");
        session.begin();
        for (key,right_alt) in [
            (egui::Key::E,false),(egui::Key::E,true),
            (egui::Key::Q,true),(egui::Key::E,false),
        ] {
            if let Some(text)=de.translate_composed_layer(key,false,right_alt,&mut state){
                session.type_text(&text);
            }
        }
        assert_eq!(session.typed,"e€@e");
        assert_eq!(session.attempts,4);
        assert_eq!(session.score().accuracy,100.0);
    }
    #[test] fn emulated_key_repeat_and_modifier_shortcuts_are_not_scored() {]]}
})
print("Isolated egui 0.35 physical AltGr + ISO input probe patched");
