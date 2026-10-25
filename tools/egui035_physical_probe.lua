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
    {[[if let Some(physical) = physical_key.or(if key == egui::Key::Space { Some(key) } else { None }) {
                        let mapped]],
     [[if let Some(physical) = physical_key.or(if key == egui::Key::Space { Some(key) } else { None }) {
                        let alt_gr=self.emulated_right_alt_held && !self.emulated_left_alt_held;
                        let mapped]]},
    {[[personal.translate(physical,modifiers.shift)]],
     [[if alt_gr {None} else {personal.translate(physical,modifiers.shift)}]]},
    {[[emulator.translate_composed(physical, modifiers.shift, compose)]],
     [[emulator.translate_composed_layer(physical,modifiers.shift,alt_gr,compose)]]},
    {[[emulator.translate(physical, modifiers.shift)]],
     [[emulator.translate_layer(physical,modifiers.shift,alt_gr)]]},
    {[[layouts::emulate(physical, &self.prefs.layout, modifiers.shift).map(|c| c.to_string())]],
     [[if alt_gr {None} else {layouts::emulate(physical, &self.prefs.layout, modifiers.shift).map(|c| c.to_string())}]]},
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
-- Diagnostic output contains only physical key names, modifier booleans and
-- result lengths, never passages, translated symbols or private IME text.
edit("src/app.rs",{
    {[[self.emulated_right_alt_held=pressed;]],
     [[self.emulated_right_alt_held=pressed;
                    if std::env::var_os("TYPEFINCH_QA_XKB_COMPOSE").is_some()
                        && std::env::var_os("TYPEFINCH_QA_REPORT").is_some() {
                        eprintln!("egui035-key-qa right_alt pressed={pressed}");
                    }]]},
    {[[self.emulated_left_alt_held=pressed;]],
     [[self.emulated_left_alt_held=pressed;
                    if std::env::var_os("TYPEFINCH_QA_XKB_COMPOSE").is_some()
                        && std::env::var_os("TYPEFINCH_QA_REPORT").is_some() {
                        eprintln!("egui035-key-qa left_alt pressed={pressed}");
                    }]]},
    {[[if let Some(text) = mapped { self.session.type_text(&text); }]],
     [[if std::env::var_os("TYPEFINCH_QA_XKB_COMPOSE").is_some()
                            && std::env::var_os("TYPEFINCH_QA_REPORT").is_some() {
                            eprintln!("egui035-key-qa physical={physical:?} level3={alt_gr} shift={} mapped_chars={} attempts_before={}",
                                modifiers.shift,mapped.as_ref().map(|s|s.chars().count()).unwrap_or(0),self.session.attempts);
                        }
                        if let Some(text) = mapped { self.session.type_text(&text); }]]}
})


-- Include rejected key events, not just accepted scoring events. This
-- distinguishes a compositor/XIM omission from our own modifier gate.
-- Scope is synthetic physical-key metadata only, never input text.
edit("src/app.rs",{
    {[[        for event in events {
            match event {]],
     [[        for event in events {
            if std::env::var_os("TYPEFINCH_QA_XKB_COMPOSE").is_some()
                && std::env::var_os("TYPEFINCH_QA_REPORT").is_some() {
                if let egui::Event::Key{key,physical_key,pressed,repeat,modifiers}= &event {
                    if matches!(physical_key,
                        Some(egui::Key::E|egui::Key::Q|egui::Key::AltRight
                            |egui::Key::AltLeft|egui::Key::IntlBackslash)) {
                        eprintln!("egui035-key-raw key={key:?} physical={physical_key:?} pressed={pressed} repeat={repeat} alt={} ctrl={} shift={} command={} held_right={} held_left={}",
                            modifiers.alt,modifiers.ctrl,modifiers.shift,modifiers.command,
                            self.emulated_right_alt_held,self.emulated_left_alt_held);
                    }
                }
            }
            match event {]]}
})

print("Isolated egui 0.35 physical AltGr + ISO input probe patched");
