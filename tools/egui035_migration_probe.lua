-- Temporary-checkout migration probe for Typefinch T303/T305.
-- This file only runs inside the isolated GitHub Actions experiment.
-- Do not count compilation alone as physical AltGr/ISO acceptance.
local path = "src/app.rs"
local input = assert(io.open(path, "rb"))
local src = assert(input:read("*a"))
assert(input:close())
local function replace(old, new, expected)
  local start, count, chunks = 1, 0, {}
  while true do
    local at, finish = src:find(old, start, true)
    if not at then break end
    chunks[#chunks+1] = src:sub(start, at-1)
    chunks[#chunks+1] = new
    start, count = finish+1, count+1
  end
  assert(count == expected,
    string.format("egui migration anchor mismatch (%d expected, %d actual): %s",
      expected, count, old))
  chunks[#chunks+1] = src:sub(start)
  src = table.concat(chunks)
end

-- egui 0.35 receives one root Ui per frame, not a root Context.
-- Keep the preexisting Context-based input/session logic, and feed the root
-- Ui to panels. This is not a fabricated physical input source.
replace("fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {",
  "fn ui(&mut self, root_ui: &mut egui::Ui, _frame: &mut eframe::Frame) {\n        local_dummy_invalid",
  1)
replace("local_dummy_invalid",
  "let ctx_owned=root_ui.ctx().clone();\n        let ctx=&ctx_owned;", 1)

replace("egui::TopBottomPanel::top(", "egui::Panel::top(", 1)
replace("egui::SidePanel::left(", "egui::Panel::left(", 1)
replace(".default_width(295.0)", ".default_size(295.0)", 1)
replace(".min_width(255.0)", ".min_size(255.0)", 1)
replace(".max_width(370.0)", ".max_size(370.0)", 1)
replace(".show(ctx, |ui| {", ".show(root_ui, |ui| {", 3)
replace("cc.egui_ctx.style().animation_time",
  "cc.egui_ctx.style_of(cc.egui_ctx.theme()).animation_time", 1)
replace("ctx.style().visuals.panel_fill",
  "ctx.style_of(ctx.theme()).visuals.panel_fill", 3)

-- The new cursor's index is a typed CharIndex. Conversion at the
-- existing Unicode boundary functions is explicit and never byte-based.
replace(".index", ".index.0", 30)

-- egui's preedit variant changed from a tuple to a struct, including
-- preedit-range metadata. Preserve the existing rule: preedit never scores.
replace("egui::ImeEvent::Preedit(text)",
  "egui::ImeEvent::Preedit {text, ..}", 2)
replace("egui::ImeEvent::Preedit(_)",
  "egui::ImeEvent::Preedit { .. }", 1)
replace("egui::ImeEvent::Preedit(", "synthetic_preedit(", 6)
replace("ImeEvent::Preedit(", "synthetic_preedit(", 3)
replace("    use super::*;\n    fn brightness",
  "    use super::*;\n    fn synthetic_preedit(text:String)->egui::ImeEvent {\n        egui::ImeEvent::Preedit{text,active_range_chars:None}\n    }\n    fn brightness",1)

replace("rect,cursor_rect:cursor\n            }",
  "rect,cursor_rect:cursor,should_interrupt_composition:false\n            }",1)

replace("&job.text[s.byte_range.clone()]",
  "&job.text[s.byte_range.start.0..s.byte_range.end.0]",1)


-- The 0.35 text editor publishes the just-rendered selection in
-- TextEditOutput.cursor_range. Reading persisted state immediately after
-- Ui::add was returning a stale, collapsed range despite the native editor
-- visibly selecting text. Retain the full editor output for RTL projection.
replace([[let edit=ui.add_enabled(self.session.state!=State::Running,
                            egui::TextEdit::multiline(&mut self.custom_text)
                                .id(editor_id)
                                .desired_rows(4)
                                .desired_width(ui.available_width())
                                .hint_text("Edit private source text"));]],
[[let mut edit_output=None;
                        ui.add_enabled_ui(self.session.state!=State::Running, |ui| {
                            edit_output=Some(egui::TextEdit::multiline(&mut self.custom_text)
                                .id(editor_id)
                                .desired_rows(4)
                                .desired_width(ui.available_width())
                                .hint_text("Edit private source text")
                                .show(ui));
                        });
                        let edit_output=edit_output.expect("native text editor output");
                        let native_cursor_range=edit_output.cursor_range;
                        let edit=edit_output.response;]],1)

replace([[let range=if edit.has_focus(){
                            egui::TextEdit::load_state(ui.ctx(),editor_id)
                                .and_then(|state|state.cursor.char_range())
                        }else{None};]],
[[let range=if edit.has_focus(){
                            let stored=egui::TextEdit::load_state(ui.ctx(),editor_id)
                                .and_then(|state|state.cursor.char_range());
                            if shift_arrow || plain_arrow || pointer_selection {
                                stored.or(native_cursor_range)
                            }else{
                                native_cursor_range.or(stored)
                            }
                        }else{None};]],1)


-- egui 0.35's native shaped TextEdit can return a visual cursor beyond
-- the scalar length of RTL marked text (observed 29 versus 19 Arabic
-- scalars). Bound ONLY the read-only projection; never change the native
-- source cursor state, selected Unicode bytes or Undo history.
replace([[self.rtl_editor_cursor_byte=range.as_ref()
                            .and_then(|range|rtl_shaping::editor_caret_byte(]],
[[let range=range.map(|mut range|{
                            let scalars=self.custom_text.chars().count();
                            range.primary.index.0=range.primary.index.0.min(scalars);
                            range.secondary.index.0=range.secondary.index.0.min(scalars);
                            range
                        });
                        self.rtl_editor_cursor_byte=range.as_ref()
                            .and_then(|range|rtl_shaping::editor_caret_byte(]],1)

-- Disposable metadata-only diagnostic for the 0.35 native RTL regression.
-- Never print the imported source, typed text, candidate or file path.
replace([[self.rtl_editor_cursor_byte=range.as_ref()
                            .and_then(|range|rtl_shaping::editor_caret_byte(]],
[[if std::env::var_os("TYPEFINCH_QA_EDITOR_CARET_REPORT").is_some()
                            && ui.input(|i|i.events.iter().any(|event|matches!(event,
                                egui::Event::Key{key:egui::Key::A,pressed:true,modifiers,..}
                                    if modifiers.ctrl))) {
                            let indices=|range:egui::text::CCursorRange|
                                (range.primary.index.0,range.secondary.index.0);
                            let saved=egui::TextEdit::load_state(ui.ctx(),editor_id)
                                .and_then(|state|state.cursor.char_range());
                            eprintln!("egui035-qa ctrl-a: focus={} id_match={} output={:?} saved={:?} used={:?} source_chars={} source_bytes={} mapped_primary={:?} mapped_secondary={:?}",
                                edit.has_focus(),edit.id==editor_id,
                                native_cursor_range.map(indices),saved.map(indices),range.map(indices),
                                self.custom_text.chars().count(),self.custom_text.len(),
                                range.and_then(|r|rtl_shaping::editor_caret_byte(&self.custom_text,r.primary.index.0)),
                                range.and_then(|r|rtl_shaping::editor_caret_byte(&self.custom_text,r.secondary.index.0)));
                        }
                        self.rtl_editor_cursor_byte=range.as_ref()
                            .and_then(|range|rtl_shaping::editor_caret_byte(]],1)


-- egui 0.35 sometimes restores the edit-time selection as part of Undo.
-- The accepted Typefinch RTL behavior preserves restored bytes but collapses
-- the selection, exactly as in the prior native editor. Do not touch IME.
replace([[let edit=edit_output.response;
                        if edit.changed(){self.rebuild_session();}]],
[[let edit=edit_output.response;
                        if edit.changed(){self.rebuild_session();}
                        let plain_undo=ui.input(|i|i.events.iter().any(|event|
                            matches!(event,egui::Event::Key{
                                key:egui::Key::Z,pressed:true,repeat:false,modifiers,..
                            } if modifiers.ctrl && !modifiers.alt
                                && !modifiers.shift && !modifiers.command)));
                        if edit.has_focus() && plain_undo && !self.rtl_editor_preedit_active {
                            if let Some(mut state)=egui::TextEdit::load_state(ui.ctx(),editor_id){
                                if let Some(mut range)=state.cursor.char_range(){
                                    if range.primary.index!=range.secondary.index {
                                        range.secondary=range.primary;
                                        state.cursor.set_char_range(Some(range));
                                        state.store(ui.ctx(),editor_id);
                                    }
                                }
                            }
                        }]],1)
replace([[if shift_arrow || plain_arrow || pointer_selection {
                                stored.or(native_cursor_range)]],
[[if shift_arrow || plain_arrow || pointer_selection || plain_undo {
                                stored.or(native_cursor_range)]],1)


-- Temporary synthetic metadata-only Wayland diagnostic. The migrated source
-- sometimes reports the full 17-grapheme range for a visibly partial RTL
-- pointer drag. Compare original scalar counts with native galley coordinates.
replace([[let range=range.map(|mut range|{
                            let scalars=self.custom_text.chars().count();]],
[[if std::env::var_os("TYPEFINCH_QA_EDITOR_CARET_REPORT").is_some()
                            && ui.input(|i|i.events.iter().any(|event|
                                matches!(event,egui::Event::PointerButton{
                                    button:egui::PointerButton::Primary,pressed:false,..}))) {
                            let ix=|r:egui::text::CCursorRange|
                                (r.primary.index.0,r.secondary.index.0);
                            let saved=egui::TextEdit::load_state(ui.ctx(),editor_id)
                                .and_then(|st|st.cursor.char_range());
                            let row_glyphs=edit_output.galley.rows.iter()
                                .map(|row|row.row.glyphs.len()).collect::<Vec<_>>();
                            let rendered_glyphs=edit_output.galley.rows.iter()
                                .flat_map(|row|row.row.glyphs.iter().map(|glyph|glyph.chr))
                                .collect::<String>();
                            let glyph_chars=rendered_glyphs.chars().collect::<Vec<_>>();
                            let mut cursor=0usize;
                            let mut matched=Vec::new();
                            for ch in self.custom_text.chars() {
                                if let Some(rel)=glyph_chars[cursor..].iter().position(|g|*g==ch) {
                                    cursor+=rel;
                                    matched.push(cursor);
                                    cursor+=1;
                                } else {break;}
                            }
                            let coarse=|ch:char|->char {
                                if ch.is_ascii_digit(){'D'} else if ch.is_whitespace(){'_'}
                                else if matches!(ch as u32,0x0600..=0x06ff){'A'}
                                else if matches!(ch as u32,0xfb50..=0xfdff){'p'}
                                else if matches!(ch as u32,0xfe70..=0xfeff){'q'}
                                else if matches!(ch as u32,0x0590..=0x05ff){'H'}
                                else if ch.is_alphabetic(){'L'} else if ch.is_ascii_punctuation(){'P'}
                                else {'x'}
                            };
                            eprintln!("egui035-wayland-glyph-map source_pattern={} glyph_pattern={} matching_prefix={} matched_glyph_indices={:?}",
                                self.custom_text.chars().map(coarse).collect::<String>(),
                                rendered_glyphs.chars().map(coarse).collect::<String>(),
                                matched.len(),matched);
                            eprintln!("egui035-wayland-rtl-qa pointer focus={} source_chars={} source_bytes={} galley_chars={} galley_bytes={} galley_end={} rows={:?} glyphs_match_source={} output={:?} stored={:?} used={:?}",
                                edit.has_focus(),self.custom_text.chars().count(),
                                self.custom_text.len(),edit_output.galley.text().chars().count(),
                                edit_output.galley.text().len(),edit_output.galley.end().index.0,
                                row_glyphs,rendered_glyphs==self.custom_text,
                                native_cursor_range.map(ix),saved.map(ix),range.map(ix));
                        }
                        let pointer_geometry=ui.input(|i|(
                            i.pointer.primary_released(),i.pointer.press_origin(),
                            i.pointer.interact_pos()));
                        let range=if edit.has_focus() && pointer_selection && !ime_event
                            && !shift_arrow && !plain_arrow
                            && range.as_ref().is_some_and(|r|{
                                let limit=self.custom_text.chars().count();
                                r.primary.index.0>limit || r.secondary.index.0>limit
                            }) && pointer_geometry.0 {
                            if let (Some(anchor),Some(active))=(pointer_geometry.1,pointer_geometry.2) {
                                if edit_output.text_clip_rect.contains(anchor)
                                    && edit_output.text_clip_rect.contains(active) {
                                    let galley=&edit_output.galley;
                                    let mapped=egui::text::CCursorRange{
                                        primary:galley.cursor_from_pos(active-edit_output.galley_pos),
                                        secondary:galley.cursor_from_pos(anchor-edit_output.galley_pos),
                                        h_pos:None,
                                    };
                                    if std::env::var_os("TYPEFINCH_QA_EDITOR_CARET_REPORT").is_some() {
                                        eprintln!("egui035-wayland-cursor-reprojection valid={} primary={} anchor={} original={:?}",
                                            mapped.primary.index.0!=mapped.secondary.index.0,
                                            mapped.primary.index.0,mapped.secondary.index.0,
                                            range.map(|r|(r.primary.index.0,r.secondary.index.0)));
                                    }
                                    if let Some(mut native)=egui::TextEdit::load_state(ui.ctx(),editor_id) {
                                        native.cursor.set_char_range(Some(mapped));
                                        native.store(ui.ctx(),editor_id);
                                    }
                                    Some(mapped)
                                }else{range}
                            }else{range}
                        }else{range};
                        let range=range.map(|mut range|{
                            let scalars=self.custom_text.chars().count();]],1)


-- A direct regression for upstream egui issue #8576. The layout must not
-- expose a glyph cursor past the Unicode source just because it is RTL.
replace([[    #[test] fn keyboard_theme_cycle_keeps_explicit_system_light_dark_order() {]],
[[    #[test] fn egui_035_rtl_galley_cursor_count_matches_unicode_scalars() {
        let ctx=egui::Context::default();
        ctx.begin_pass(egui::RawInput::default());
        ctx.fonts_mut(|fonts| {
            for source in ["سَلَام بالعربية 123","שָׁלוֹם בעברית 123","Hello 123"] {
                let galley=fonts.layout_no_wrap(source.to_owned(),
                    egui::FontId::proportional(18.0),egui::Color32::WHITE);
                assert_eq!(galley.end().index.0,source.chars().count(),
                    "egui 0.35 shaped galley must preserve Unicode cursor extent");
                let glyphs:usize=galley.rows.iter().map(|row|row.row.glyphs.len()).sum();
                assert_eq!(glyphs,source.chars().count(),
                    "native editor may not treat multiple RTL glyphs as characters");
            }
        });
        let _=ctx.end_pass();
    }
    #[test] fn keyboard_theme_cycle_keeps_explicit_system_light_dark_order() {]],1)

local out=assert(io.open(path, "wb"))
assert(out:write(src))
assert(out:close())
print("Applied isolated egui 0.35 migration probe to "..path)
