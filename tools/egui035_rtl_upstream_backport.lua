-- Temporary backport of the narrow upstream egui fix:
-- https://github.com/emilk/egui/pull/8698
-- https://github.com/emilk/egui/issues/8576
-- Source is MIT OR Apache-2.0 licensed; preserve its original licenses.
-- These changes are applied only to epaint/egui 0.35.0 in isolated CI.
-- No downloaded font or private text is bundled or persisted.
local epaint_path=assert(arg[1],"epaint 0.35.0 text_layout.rs required")
local egui_path=assert(arg[2],"egui 0.35.0 text_cursor_state.rs required")
assert(epaint_path:find("/epaint%-0%.35%.0/src/text/text_layout%.rs$"))
assert(egui_path:find("/egui%-0%.35%.0/src/text_selection/text_cursor_state%.rs$"))
local function patch_file(path, edits)
    local f=assert(io.open(path,"rb"));local text=assert(f:read("*a"));assert(f:close())
    for i,edit in ipairs(edits) do
        local old,new=edit[1],edit[2]
        local first=assert(text:find(old,1,true),"Missing upstream backport anchor "..i.." in "..path)
        assert(not text:find(old,first+#old,true),"Ambiguous upstream backport anchor "..i)
        text=text:sub(1,first-1)..new..text:sub(first+#old)
    end
    local out=assert(io.open(path,"wb"));assert(out:write(text));assert(out:close())
    print("Backported upstream RTL shaping fix to "..path)
end
patch_file(epaint_path,{
    {[[    glyph_buffer: &harfrust::GlyphBuffer,
    face_metrics: &StyledMetrics,]],
     [[    glyph_buffer: &harfrust::GlyphBuffer,
    direction: harfrust::Direction,
    face_metrics: &StyledMetrics,]]},
    {[[    let mut cluster_start_byte: usize = 0;
    let mut cluster_glyph_count: usize = 0;]],
     [[    let mut cluster_start_byte: usize = 0;
    let mut cluster_glyph_count: usize = 0;
    // harfrust returns visual-order clusters for an RTL run. Their byte
    // offsets decrease, so forward-only continuation padding is incorrect.
    let is_rtl=direction==harfrust::Direction::RightToLeft;
    let mut rtl_cluster_end_byte=run_text.len();]]},
    {[[            if ctx.prev_cluster.is_some() {
                emit_continuation_glyphs(
                    ctx,
                    paragraph,
                    run_text,
                    cluster_start_byte..cluster as usize,
                    cluster_glyph_count,
                    face_metrics,
                );]],
     [[            if ctx.prev_cluster.is_some() {
                let cluster_bytes=if is_rtl {
                    let bytes=cluster_start_byte..rtl_cluster_end_byte;
                    rtl_cluster_end_byte=cluster_start_byte;
                    bytes
                }else{cluster_start_byte..cluster as usize};
                emit_continuation_glyphs(
                    ctx,
                    paragraph,
                    run_text,
                    cluster_bytes,
                    cluster_glyph_count,
                    face_metrics,
                );]]},
    {[[    if ctx.prev_cluster.is_some() {
        emit_continuation_glyphs(
            ctx,
            paragraph,
            run_text,
            cluster_start_byte..run_text.len(),
            cluster_glyph_count,
            face_metrics,
        );]],
     [[    if ctx.prev_cluster.is_some() {
        let cluster_bytes=if is_rtl {
            cluster_start_byte..rtl_cluster_end_byte
        }else{cluster_start_byte..run_text.len()};
        emit_continuation_glyphs(
            ctx,
            paragraph,
            run_text,
            cluster_bytes,
            cluster_glyph_count,
            face_metrics,
        );]]},
    {[[            let glyph_buffer = shape_text(font_face, run_text, &format.coords, shape_buffer, flags);]],
     [[            let (glyph_buffer, direction) = shape_text(font_face, run_text, &format.coords, shape_buffer, flags);]]},
    {[[                &glyph_buffer,
                &face_metrics,]],
     [[                &glyph_buffer,
                direction,
                &face_metrics,]]},
    {[[) -> harfrust::GlyphBuffer {
    let font_ref = font_face.skrifa_font_ref();]],
     [[) -> (harfrust::GlyphBuffer,harfrust::Direction) {
    let font_ref = font_face.skrifa_font_ref();]]},
    {[[    buffer.guess_segment_properties();

    shaper.shape(buffer, &[])
}]],
     [[    buffer.guess_segment_properties();
    let direction=buffer.direction();
    let glyph_buffer=shaper.shape(buffer, &[]);
    (glyph_buffer,direction)
}]]}
})
patch_file(egui_path,{
    {[[    let boundary = next_word_boundary_char_index(&reversed, num_chars - ccursor.index);]],
     [[    let chars_after_cursor=num_chars-ccursor.index.min(num_chars);
    let boundary = next_word_boundary_char_index(&reversed, chars_after_cursor);]]}
})
