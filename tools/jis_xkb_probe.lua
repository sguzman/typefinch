-- Temporary Typefinch-only JIS XKB proof. The pinned egui/egui-winit
-- vendors are already patched, so this modifies only the disposable
-- src/xkb.rs inside the isolated CI checkout; no production source changes.
local function patch(path,edits)
  local file=assert(io.open(path,"rb"));local content=assert(file:read("*a"));file:close()
  for i,e in ipairs(edits) do
    local old,new=e[1],e[2]
    local from=assert(content:find(old,1,true),"Missing JIS source anchor "..i)
    assert(not content:find(old,from+#old,true),"Ambiguous JIS anchor "..i)
    content=content:sub(1,from-1)..new..content:sub(from+#old)
  end
  file=assert(io.open(path,"wb"));assert(file:write(content));file:close()
  print("Applied pinned-vendor JIS evdev route proof in temporary "..path)
end
patch("src/xkb.rs",{
  {[[        Key::IntlBackslash=>86,
        _ => return None]],[[        Key::IntlBackslash=>86,
        Key::IntlRo=>89,Key::IntlYen=>124,
        // Conversion and mode-switching keys are NEVER scored as text.
        _ => return None]]},
  {[[    #[test] fn common_system_xkb_layouts_compile()]],[[    #[test] fn distinct_jis_ro_and_yen_xkb_positions_do_not_alias_ansi_or_controls() {
        assert_eq!(evdev_code(Key::IntlRo),Some(89));
        assert_eq!(evdev_code(Key::IntlYen),Some(124));
        assert_ne!(evdev_code(Key::IntlRo),evdev_code(Key::IntlBackslash));
        assert_ne!(evdev_code(Key::IntlYen),evdev_code(Key::Backslash));
        for control in [Key::Convert,Key::NonConvert,Key::KanaMode,Key::Lang5] {
            assert_eq!(evdev_code(control),None,
                "Japanese IME mode keys must never become printable score input");
        }
        let jp=Emulator::compile("jp(OADG109A)").expect("installed JIS109 XKB layout");
        assert_eq!(jp.translate_layer(Key::IntlRo,false,false).as_deref(),Some("\\"));
        assert_eq!(jp.translate_layer(Key::IntlRo,true,false).as_deref(),Some("_"));
        assert_eq!(jp.translate_layer(Key::IntlYen,false,false).as_deref(),Some("¥"));
        assert_eq!(jp.translate_layer(Key::IntlYen,true,false).as_deref(),Some("|"));
    }
    #[test] fn common_system_xkb_layouts_compile()]]}
})
