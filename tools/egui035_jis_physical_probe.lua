-- T305 experimental JIS physical-position mapping for egui 0.35 + egui-winit.
-- Applied ONLY to temporary CI Cargo registry sources and a throwaway
-- checkout. Never turns OS text or an XKB label into fabricated provenance.
local egui_path,bridge_path,app_xkb=assert(arg[1]),assert(arg[2]),assert(arg[3])
assert(egui_path:find("/egui%-0%.35%.0/src/data/key%.rs$"))
assert(bridge_path:find("/egui%-winit%-0%.35%.0/src/lib%.rs$"))
assert(app_xkb=="src/xkb.rs")
local function patch(path,edits)
  local f=assert(io.open(path,"rb"));local src=assert(f:read("*a"));f:close()
  for i,e in ipairs(edits) do
    local old,new=e[1],e[2]
    local start=assert(src:find(old,1,true),path.." missing anchor "..i)
    assert(not src:find(old,start+#old,true),path.." ambiguous anchor "..i)
    src=src:sub(1,start-1)..new..src:sub(start+#old)
  end
  f=assert(io.open(path,"wb"));assert(f:write(src));f:close()
  print("Patched "..path.." ("..#edits.." anchors)")
end
patch(egui_path,{
  {[[    IntlBackslash,
    // When adding keys,]],[[    IntlBackslash,
    /// Japanese key left of the right Shift (KEY_RO, evdev 89).
    IntlRo,
    /// Japanese currency/pipe key (KEY_YEN, evdev 124).
    IntlYen,
    /// Japanese conversion controls: never inferred from printable text.
    Convert,
    NonConvert,
    KanaMode,
    /// Japanese Zenkaku/Hankaku key (evdev 85).
    Lang5,
    // When adding keys,]]},
  {[[        // International keys:
        Self::IntlBackslash,
    ];]],[[        // International keys:
        Self::IntlBackslash,
        Self::IntlRo,
        Self::IntlYen,
        Self::Convert,
        Self::NonConvert,
        Self::KanaMode,
        Self::Lang5,
    ];]]},
  {[[            "IntlBackslash" => Self::IntlBackslash,

            _ => return None,]],[[            "IntlBackslash" => Self::IntlBackslash,
            "IntlRo" => Self::IntlRo,
            "IntlYen" => Self::IntlYen,
            "Convert" => Self::Convert,
            "NonConvert" => Self::NonConvert,
            "KanaMode" => Self::KanaMode,
            "Lang5" => Self::Lang5,

            _ => return None,]]},
  {[[            Self::IntlBackslash => "IntlBackslash",
        }
    }
}]],[[            Self::IntlBackslash => "IntlBackslash",
            Self::IntlRo => "IntlRo",
            Self::IntlYen => "IntlYen",
            Self::Convert => "Convert",
            Self::NonConvert => "NonConvert",
            Self::KanaMode => "KanaMode",
            Self::Lang5 => "Lang5",
        }
    }
}]]},
  {[[        Key::IntlBackslash as usize + 1,]],[[        Key::Lang5 as usize + 1,]]}
})
patch(bridge_path,{
  {[[        KeyCode::IntlBackslash => Key::IntlBackslash,

        _ => {]],[[        KeyCode::IntlBackslash => Key::IntlBackslash,
        KeyCode::IntlRo => Key::IntlRo,
        KeyCode::IntlYen => Key::IntlYen,
        KeyCode::Convert => Key::Convert,
        KeyCode::NonConvert => Key::NonConvert,
        KeyCode::KanaMode => Key::KanaMode,
        KeyCode::Lang5 => Key::Lang5,

        _ => {]]}
})
patch(app_xkb,{
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
