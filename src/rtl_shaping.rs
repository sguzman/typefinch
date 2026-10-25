//! A bounded pure-Rust shaping and bidi renderer for visual RTL reference
//! passages. Scoring remains in src/engine.rs at *logical* grapheme positions.
//! Glyphs and screenshots are never saved except in explicit synthetic CI.
//! This renderer does not subscribe to any keyboard input or mutate a layout.
use cosmic_text::{Attrs,Buffer,Color,Cursor,FontSystem,Metrics,Shaping,SwashCache};
use eframe::egui::{self,Color32};
use unicode_segmentation::UnicodeSegmentation;

/// Script-triggered preview only, not a language classifier. Unicode
/// text order remains logical; COSMIC Text handles visual bidi placement.
pub fn contains_rtl(text:&str)->bool {
    text.chars().any(|ch|matches!(ch as u32,
        0x0590..=0x08FF | 0xFB1D..=0xFDFF | 0xFE70..=0xFEFF
        | 0x10E60..=0x10E7F | 0x1EE00..=0x1EEFF))
}
/// Keep at most approximately four to five visual lines of reference near
/// the current logical cursor. This prevents long RTL passages from placing
/// the caret in a clipped texture at narrow window widths.
pub fn preview_window_limit(width:usize)->usize {
    (width.max(128)/14).saturating_mul(4).clamp(40,230)
}
/// Approximate visual row occupancy. Source text is still shaped by COSMIC,
/// not repositioned or reversed with this conservative sizing heuristic.
pub fn preview_height(text:&str,width:usize)->usize {
    let columns=(width.max(128)/12).max(1);
    let rows=text.split('\n').map(|line|{
        line.graphemes(true).count().max(1).div_ceil(columns)
    }).sum::<usize>().max(1);
    rows.saturating_mul(30).saturating_add(18).clamp(102,240)
}
/// Map egui's scalar-character cursor into a valid UTF-8 grapheme boundary.
/// A cursor inside Arabic marks, Hebrew niqqud, or joined emoji is snapped
/// to the start of that cluster rather than fabricated between glyphs.
pub fn editor_caret_byte(text:&str,char_index:usize)->Option<usize>{
    let byte=text.char_indices().nth(char_index).map(|(offset,_)|offset)
        .or_else(||(char_index==text.chars().count()).then_some(text.len()))?;
    if byte==text.len(){return Some(byte);}
    text.grapheme_indices(true).take_while(|(offset,_)|*offset<=byte)
        .last().map(|(offset,_)|offset).or(Some(0))
}
/// Split only on logical selection boundaries. COSMIC still performs
/// the actual bidirectional shaping; no source reversal, text mutation,
/// inferred cursor direction or IME state is involved.
pub fn selection_spans(text:&str,selected:Option<(usize,usize)>,
    regular:Color32,highlight:Color32)->Vec<(String,Color32)> {
    let Some((start,end))=selected else{return vec![(text.into(),regular)];};
    if start>=end || end>text.len()
        || !text.is_char_boundary(start) || !text.is_char_boundary(end)
        || !text.grapheme_indices(true).any(|(offset,_)|offset==start)
        || (end!=text.len() && !text.grapheme_indices(true).any(|(offset,_)|offset==end)) {
        return vec![(text.into(),regular)];
    }
    let mut result=Vec::with_capacity(3);
    if start>0 {result.push((text[..start].into(),regular));}
    result.push((text[start..end].into(),highlight));
    if end<text.len(){result.push((text[end..].into(),regular));}
    result
}
/// Align a keyboard selection endpoint with a complete Unicode grapheme.
/// The ordinary egui TextEdit uses scalar-character cursor indices, which
/// can land inside Arabic harakat, Hebrew niqqud, or joined emoji. Only
/// Shift+Arrow selections use this helper; neither source text nor caret
/// positioning for IME composition is rewritten.
/// Arrow-only movement through marked graphemes: resolve a native scalar
/// cursor step to the next or previous *complete* extended grapheme.
pub fn snap_moved_caret(text:&str,active:usize,previous:usize)->Option<usize>{
    if active==previous {return None;}
    let total=text.chars().count();
    if active>total || previous>total {return None;}
    let mut start=0;
    for grapheme in text.graphemes(true) {
        let end=start+grapheme.chars().count();
        if start<active && active<end {
            return Some(if active>previous {end} else {start});
        }
        start=end;
    }
    None
}
pub fn snap_partial_selection(text:&str,active:usize,anchor:usize,previous:usize)->Option<usize>{
    if active==anchor || anchor>text.chars().count() {return None;}
    snap_moved_caret(text,active,previous)
}
/// When a native pointer drag starts or ends inside a marked grapheme,
/// expand both selection endpoints to complete logical clusters. This is
/// applied only to a non-collapsed mouse selection, not to IME preedit,
/// keyboard navigation, caret clicks, or scoring input.
pub fn snap_drag_selection(text:&str,active:usize,anchor:usize)
    ->Option<(usize,usize)> {
    let total=text.chars().count();
    if active==anchor || active>total || anchor>total {return None;}
    let low=active.min(anchor);
    let high=active.max(anchor);
    let mut scalar=0;
    let mut first=0;
    let mut last=total;
    for grapheme in text.graphemes(true) {
        let next=scalar+grapheme.chars().count();
        if scalar<=low && low<next {first=scalar;}
        if scalar<high && high<=next {last=next;}
        scalar=next;
    }
    let corrected=if active<anchor {(first,last)}else{(last,first)};
    (corrected!=(active,anchor)).then_some(corrected)
}
pub struct ShapedImage {
    pub image:egui::ColorImage,
    /// Visual caret origin in texture-local points/pixels; no physical
    /// keyboard attribution or IME event subscription is involved.
    pub caret:Option<(f32,f32)>,
}
pub struct Renderer {
    fonts:FontSystem,
    cache:SwashCache,
}
impl Renderer {
    pub fn new()->Self {
        Self{fonts:FontSystem::new(),cache:SwashCache::new()}
    }
    /// Rasterize shaped logical spans, preserving Unicode bidi placement.
    /// Transparent output layers onto the existing native egui surface.
    /// Resource caps forbid allocating arbitrarily large passages or images.
    pub fn render(&mut self,spans:&[(String,Color32)],width:usize,height:usize)
        ->Option<egui::ColorImage> {
        self.render_with_caret(spans,width,height,None).map(|result|result.image)
    }
    /// The cursor index refers to a byte boundary of the *logical visible*
    /// reference text. COSMIC Text maps it through BiDi layout into visual
    /// coordinates. Caller owns the focus and IME policy.
    pub fn render_with_caret(&mut self,spans:&[(String,Color32)],
        width:usize,height:usize,cursor_byte:Option<usize>)
        ->Option<ShapedImage> {
        if width==0 || height==0 || width>2048 || height>256
            || spans.len()>512
            || spans.iter().map(|(text,_)|text.len()).sum::<usize>()>16_384 {
            return None;
        }
        let mut pixels=vec![0_u8;width.checked_mul(height)?.checked_mul(4)?];
        let mut buffer=Buffer::new(&mut self.fonts,Metrics::new(21.0,30.0));
        let mut borrowed=buffer.borrow_with(&mut self.fonts);
        borrowed.set_size(Some(width as f32),Some(height as f32));
        borrowed.set_rich_text(
            spans.iter().map(|(text,color)|(
                text.as_str(),
                Attrs::new().color(Color::rgb(color.r(),color.g(),color.b()))
            )),
            &Attrs::new(),Shaping::Advanced,None,
        );
        let visible=spans.iter().map(|(text,_)|text.as_str()).collect::<String>();
        borrowed.shape_until_scroll(false);
        // Byte offsets are logical, but COSMIC cursors are (line, line-local
        // byte offsets). Reject a cursor inside a codepoint or grapheme.
        // A multiline pasted Hebrew or Arabic reference must not position
        // the caret in the first line or bisect a combining mark.
        let caret=cursor_byte.filter(|index|*index<=visible.len()
            && visible.is_char_boundary(*index)
            && (*index==visible.len() ||
                visible.grapheme_indices(true).any(|(start,_)|start==*index)))
            .and_then(|index|{
                let before=&visible[..index];
                let line=before.bytes().filter(|b|*b==b'\n').count();
                let column=before.rsplit_once('\n')
                    .map(|(_,tail)|tail.len()).unwrap_or(before.len());
                borrowed.cursor_position(&Cursor::new(line,column))
            })
            .filter(|(x,y)|x.is_finite() && y.is_finite()
                && *x>=0.0 && *y>=0.0
                && *x<=width as f32 && *y<height as f32);
        borrowed.draw(&mut self.cache,Color::rgb(255,255,255),|x,y,w,h,color|{
            let (red,green,blue,alpha)=color.as_rgba_tuple();
            if alpha==0{return;}
            for yy in 0..h.min(256) {
                let cy=i64::from(y)+i64::from(yy);
                if cy<0 || cy>=height as i64 {continue;}
                for xx in 0..w.min(2048) {
                    let cx=i64::from(x)+i64::from(xx);
                    if cx<0 || cx>=width as i64 {continue;}
                    let index=((cy as usize)*width+cx as usize)*4;
                    let old_a=u32::from(pixels[index+3]);
                    let new_a=u32::from(alpha);
                    let alpha_out=new_a+(old_a*(255-new_a)+127)/255;
                    if alpha_out==0 {continue;}
                    for (offset,channel) in [red,green,blue].into_iter().enumerate(){
                        let previous=u32::from(pixels[index+offset]);
                        let weighted=u32::from(channel)*new_a
                            +(previous*old_a*(255-new_a)+127)/255;
                        pixels[index+offset]=((weighted+alpha_out/2)/alpha_out).min(255) as u8;
                    }
                    pixels[index+3]=alpha_out.min(255) as u8;
                }
            }
        });
        Some(ShapedImage{
            image:egui::ColorImage::from_rgba_unmultiplied([width,height],&pixels),
            caret,
        })
    }
    /// Map a visual point in a shaped RTL/mixed-script preview back to a
    /// *logical UTF-8 grapheme boundary*. Uses COSMIC Text's actual BiDi
    /// layout and cluster-aware hit testing, never reverses input or
    /// fabricates key events. This helper is not an editable widget and
    /// does not change the native text/IME caret until explicitly integrated.
    pub fn hit_test_byte(&mut self,text:&str,width:usize,height:usize,
        x:f32,y:f32)->Option<usize> {
        if width==0 || height==0 || width>2048 || height>256
            || text.len()>16_384 || !x.is_finite() || !y.is_finite()
            || x<0.0 || y<0.0 || x>width as f32 || y>height as f32 {
            return None;
        }
        let mut buffer=Buffer::new(&mut self.fonts,Metrics::new(21.0,30.0));
        let mut borrowed=buffer.borrow_with(&mut self.fonts);
        borrowed.set_size(Some(width as f32),Some(height as f32));
        borrowed.set_text(text,&Attrs::new(),Shaping::Advanced,None);
        let cursor=borrowed.hit(x,y)?;
        // COSMIC's Cursor.index is line-relative BYTE position. Build
        // the same line-relative offset model used by render_with_caret.
        let mut start=0_usize;
        let mut line=cursor.line;
        for piece in text.split('\n') {
            if line==0 {
                if cursor.index>piece.len() || !piece.is_char_boundary(cursor.index){
                    return None;
                }
                let byte=start.checked_add(cursor.index)?;
                if byte==text.len(){return Some(byte);}
                return text.grapheme_indices(true)
                    .take_while(|(at,_)|*at<=byte)
                    .last().map(|(at,_)|at)
                    .or_else(||(byte==0).then_some(0));
            }
            start=start.checked_add(piece.len())?.checked_add(1)?;
            line-=1;
        }
        None
    }
    /// Read-only diagnostics from the actual shaping library. A line's base
    /// direction is not inferred from font availability or string reversal.
    pub fn paragraph_directions(&mut self,text:&str)->Vec<bool> {
        let mut buffer=Buffer::new(&mut self.fonts,Metrics::new(21.0,30.0));
        let mut borrowed=buffer.borrow_with(&mut self.fonts);
        borrowed.set_size(Some(800.0),Some(120.0));
        borrowed.set_text(text,&Attrs::new(),Shaping::Advanced,None);
        borrowed.layout_runs().map(|run|run.rtl).collect()
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn rtl_selection_spans_preserve_logical_graphemes_and_boundaries(){
        let text="سلام abc שָׁלוֹם";
        let start="سلام ".len();
        let end="سلام abc".len();
        let regular=Color32::WHITE;
        let highlight=Color32::from_rgb(130,200,240);
        let spans=selection_spans(text,Some((start,end)),regular,highlight);
        assert_eq!(spans.len(),3);
        assert_eq!(spans[0],("سلام ".into(),regular));
        assert_eq!(spans[1],("abc".into(),highlight));
        assert_eq!(spans[2],(" שָׁלוֹם".into(),regular));
        assert_eq!(spans.iter().map(|(part,_)|part.as_str()).collect::<String>(),text);
        let renderer=&mut Renderer::new();
        let neutral=renderer.render(&[(text.into(),regular)],640,102).unwrap();
        let selected=renderer.render(&spans,640,102).unwrap();
        assert!(neutral.pixels.iter().zip(&selected.pixels)
            .filter(|(a,b)|a!=b).count()>10);
        // Unaligned selection inside a niqqud cluster must fail closed.
        let bad="سلام abc ש".len();
        assert_eq!(selection_spans(text,Some((bad,end.max(bad+1))),regular,highlight),
            vec![(text.into(),regular)]);
        assert_eq!(selection_spans(text,Some((0,text.len()+2)),regular,highlight),
            vec![(text.into(),regular)]);
    }
    #[test] fn pointer_drag_never_selects_partial_marked_or_joined_graphemes(){
        let arabic="سَلَام";
        assert_eq!(snap_drag_selection(arabic,1,3),Some((0,4)));
        assert_eq!(snap_drag_selection(arabic,3,1),Some((4,0)),
            "reverse drag must preserve its active endpoint direction");
        assert_eq!(snap_drag_selection(arabic,2,4),None,
            "already complete grapheme range must remain unchanged");
        let hebrew="שָׁלוֹם";
        assert_eq!(snap_drag_selection(hebrew,1,4),Some((0,4)));
        assert_eq!(snap_drag_selection(hebrew,4,1),Some((4,0)));
        let emoji="👩‍💻";
        assert_eq!(snap_drag_selection(emoji,1,2),Some((0,3)),
            "a joined emoji cannot be partially selected by a mouse drag");
        assert_eq!(snap_drag_selection(emoji,2,1),Some((3,0)));
        assert_eq!(snap_drag_selection(emoji,2,2),None,
            "ordinary mouse caret clicks are not rewritten");
        assert_eq!(snap_drag_selection(emoji,3,0),None);
        assert_eq!(snap_drag_selection(emoji,4,0),None);
    }
    #[test] fn all_mixed_script_pointer_drags_snap_to_exact_unicode_boundaries(){
        // Exhaustively check every ordered scalar-cursor pair rather than
        // assuming a particular visual direction or one Arabic/Hebrew mark.
        // These fixtures also cover flags, ZWJ sequences, decomposed Latin,
        // Indic ligatures and adjacent RTL/LTR punctuation.
        for text in [
            "سَلَام 123 שָׁלוֹם",
            "👩‍💻 شَ שָׁ e\u{301} 🇲🇽",
            "English (שָׁלוֹם) العربية!",
            "क्‍ष مَرْحَبًا 👨‍👩‍👧‍👦",
        ] {
            let total=text.chars().count();
            let starts=text.grapheme_indices(true).map(|(offset,_)|
                text[..offset].chars().count()).chain(std::iter::once(total))
                .collect::<std::collections::HashSet<_>>();
            for anchor in 0..=total {
                for active in 0..=total {
                    if active==anchor {
                        assert_eq!(snap_drag_selection(text,active,anchor),None);
                        continue;
                    }
                    let (fixed_active,fixed_anchor)=
                        snap_drag_selection(text,active,anchor)
                        .unwrap_or((active,anchor));
                    assert!(starts.contains(&fixed_active),
                        "active cursor split Unicode grapheme: {text:?} ({active},{anchor})");
                    assert!(starts.contains(&fixed_anchor),
                        "anchor cursor split Unicode grapheme: {text:?} ({active},{anchor})");
                    let original_low=active.min(anchor);
                    let original_high=active.max(anchor);
                    assert!(fixed_active.min(fixed_anchor)<=original_low
                        && fixed_active.max(fixed_anchor)>=original_high,
                        "normalization must never shrink an actual pointer selection");
                    assert_eq!(fixed_active<fixed_anchor,active<anchor,
                        "normalization must preserve selection direction");
                }
            }
            assert_eq!(snap_drag_selection(text,total+1,0),None);
            assert_eq!(snap_drag_selection(text,0,total+1),None);
        }
    }
    #[test] fn cosmic_visual_hit_testing_returns_only_logical_cluster_boundaries(){
        let mut shaper=Renderer::new();
        for text in [
            "שָׁלוֹם abc مرحبا 123",
            "سَلَام\nHebrew שָׁלוֹם\n👩‍💻 e\u{301}",
        ] {
            let boundaries=text.grapheme_indices(true).map(|(idx,_)|idx)
                .chain(std::iter::once(text.len()))
                .collect::<std::collections::HashSet<_>>();
            let mut hits=0;
            for y in [2.0,15.0,34.0,62.0,91.0] {
                for x in [2.0,34.0,84.0,180.0,290.0] {
                    if let Some(byte)=shaper.hit_test_byte(text,320,140,x,y){
                        hits+=1;
                        assert!(byte<=text.len() && boundaries.contains(&byte),
                            "COSMIC pointer hit bisected an actual grapheme: {text:?} x={x} y={y} at={byte}");
                        assert!(text.is_char_boundary(byte));
                    }
                }
            }
            assert!(hits>4,"visual BiDi hit tester never returned valid cursor boundaries");
            for (x,y) in [
                (-1.0,1.0),(1.0,-1.0),
                (f32::NAN,1.0),(1.0,f32::NAN),
                (f32::INFINITY,1.0),(1.0,f32::NEG_INFINITY),
                (321.0,1.0),(1.0,141.0),
            ] {
                assert_eq!(shaper.hit_test_byte(text,320,140,x,y),None);
            }
            assert_eq!(shaper.hit_test_byte(text,0,140,1.0,1.0),None);
            assert_eq!(shaper.hit_test_byte(text,320,0,1.0,1.0),None);
            assert_eq!(shaper.hit_test_byte(text,2049,140,1.0,1.0),None);
            assert_eq!(shaper.hit_test_byte(text,320,257,1.0,1.0),None);
        }
    }
    #[test] fn keyboard_selection_expands_and_contracts_complete_clusters(){
        let hebrew="שָׁלוֹם";
        assert_eq!(snap_partial_selection(hebrew,1,0,0),Some(3));
        assert_eq!(snap_partial_selection(hebrew,2,0,1),Some(3));
        assert_eq!(snap_partial_selection(hebrew,2,0,3),Some(0),
            "Shift+Arrow reversed must clear the selected first cluster");
        assert_eq!(snap_partial_selection(hebrew,2,3,3),Some(0));
        assert_eq!(snap_partial_selection(hebrew,3,0,0),None);
        let arabic="سَلَام";
        assert_eq!(snap_partial_selection(arabic,1,0,0),Some(2));
        assert_eq!(snap_partial_selection(arabic,1,0,2),Some(0));
        let emoji="👩‍💻 is one";
        assert_eq!(snap_partial_selection(emoji,1,0,0),Some(3));
        assert_eq!(snap_partial_selection(emoji,2,0,1),Some(3));
        assert_eq!(snap_partial_selection(emoji,2,0,3),Some(0));
        assert_eq!(snap_partial_selection(emoji,0,0,0),None);
        assert_eq!(snap_partial_selection(emoji,99,0,0),None);
        assert_eq!(snap_partial_selection(emoji,1,99,0),None);
        assert_eq!(snap_partial_selection("abc",1,0,0),None);
    }
    #[test] fn plain_arrow_navigation_skips_combining_mark_interiors(){
        let hebrew="שָׁלוֹם";
        assert_eq!(snap_moved_caret(hebrew,1,0),Some(3));
        assert_eq!(snap_moved_caret(hebrew,2,3),Some(0));
        assert_eq!(snap_moved_caret(hebrew,3,0),None);
        assert_eq!(snap_moved_caret(hebrew,0,3),None);
        assert_eq!(snap_moved_caret(hebrew,2,2),None);
        let arabic="سَلَام";
        assert_eq!(snap_moved_caret(arabic,1,0),Some(2));
        assert_eq!(snap_moved_caret(arabic,1,2),Some(0));
        let emoji="👩‍💻";
        assert_eq!(snap_moved_caret(emoji,1,0),Some(3));
        assert_eq!(snap_moved_caret(emoji,2,3),Some(0));
        assert_eq!(snap_moved_caret(emoji,99,0),None);
        assert_eq!(snap_moved_caret("abc",1,0),None);
    }
    #[test] fn egui_scalar_editor_caret_snaps_to_unicode_graphemes(){
        let text="שָׁלוֹם 🌍👩‍💻 سلام";
        assert_eq!(editor_caret_byte(text,0),Some(0));
        assert_eq!(editor_caret_byte(text,1),Some(0),
            "niqqud after Hebrew base must not create a caret");
        assert_eq!(editor_caret_byte(text,2),Some(0));
        assert_eq!(editor_caret_byte(text,3),Some("שָׁ".len()));
        let at_emoji=text.chars().position(|ch|ch=='👩').unwrap();
        let start=editor_caret_byte(text,at_emoji).unwrap();
        assert_eq!(editor_caret_byte(text,at_emoji+1),Some(start));
        assert_eq!(editor_caret_byte(text,at_emoji+2),Some(start));
        assert_eq!(editor_caret_byte(text,text.chars().count()),Some(text.len()));
        assert_eq!(editor_caret_byte(text,text.chars().count()+1),None);
        assert_eq!(editor_caret_byte("",0),Some(0));
    }
    #[test] fn narrow_viewport_keeps_rtl_shaped_caret_in_bounded_frames(){
        let wide=preview_window_limit(900);
        let narrow=preview_window_limit(175);
        assert!(wide>narrow);
        assert!(wide<=230 && narrow>=40);
        let short="שלום 123";
        assert_eq!(preview_height(short,800),102);
        let long="שלום ".repeat(45);
        let wide_height=preview_height(&long,900);
        let narrow_height=preview_height(&long,175);
        assert!(narrow_height>=wide_height);
        assert!(narrow_height<=240);
        assert!(preview_height("שלום\nשלום",175)>=102);
    }
    #[test] fn rtl_detection_uses_script_not_unrelated_cjk_or_latin_text(){
        for passage in ["سَلَام","שָׁלוֹם","English سلام","فارسی"] {
            assert!(contains_rtl(passage));
        }
        for passage in ["English text","你好世界","きょう","한글","🙂"] {
            assert!(!contains_rtl(passage));
        }
    }
    #[test] fn independent_bidi_shaper_recognizes_rtl_base_direction(){
        let mut renderer=Renderer::new();
        for passage in ["سَلَام بالعربية","שָׁלוֹם בעברית"] {
            let direction=renderer.paragraph_directions(passage);
            assert!(!direction.is_empty());
            assert!(direction.iter().all(|rtl|*rtl),
                "native shaping must establish RTL base direction");
        }
        assert!(renderer.paragraph_directions("English reference").iter()
            .all(|rtl|!*rtl));
    }
    #[test] fn native_bidi_cursor_maps_logical_byte_offset_to_visual_position(){
        let mut renderer=Renderer::new();
        for passage in ["سلام","שלום"] {
            let style=vec![(passage.to_owned(),Color32::WHITE)];
            let initial=renderer.render_with_caret(&style,640,96,Some(0)).unwrap();
            let next=renderer.render_with_caret(&style,640,96,
                Some(passage.chars().next().unwrap().len_utf8())).unwrap();
            let first=initial.caret.expect("initial caret must be placed by shaper");
            let second=next.caret.expect("following caret must have a visual position");
            assert!(first.0>=0.0 && first.0<=640.0 && first.1>=0.0 && first.1<=96.0);
            assert!(second.0>=0.0 && second.0<=640.0 && second.1>=0.0 && second.1<=96.0);
            assert!((first.0-second.0).abs()>1.0,
                "advancing logical RTL text must move the visual caret");
            assert!(renderer.render_with_caret(&style,640,96,Some(passage.len()+1))
                .unwrap().caret.is_none(),"out-of-bounds caret is not guessed");
        }
    }
    #[test] fn multiline_rtl_caret_uses_line_relative_grapheme_boundary(){
        let mut renderer=Renderer::new();
        let content="سلام\nשָׁלוֹם";
        let parts=vec![(content.to_owned(),Color32::WHITE)];
        let first=renderer.render_with_caret(&parts,640,128,Some(0))
            .unwrap().caret.expect("first line cursor");
        let second=renderer.render_with_caret(&parts,640,128,
            Some("سلام\n".len())).unwrap().caret.expect("second line cursor");
        assert!(second.1>first.1+12.0,
            "second logical paragraph's caret must be on a later visual row");
        let mark_inside="سلام\nש".len()+1;
        assert!(renderer.render_with_caret(&parts,640,128,Some(mark_inside))
            .unwrap().caret.is_none(),"reject byte boundary inside UTF-8 scalar");
        let grapheme="שָׁ";
        let not_cluster_boundary="سلام\nש".len();
        assert!(not_cluster_boundary<"سلام\n".len()+grapheme.len());
        assert!(renderer.render_with_caret(&parts,640,128,Some(not_cluster_boundary))
            .unwrap().caret.is_none(),"reject caret between base and niqqud");
    }
    #[test] fn mixed_direction_paragraphs_keep_logical_carets_across_line_changes(){
        let mut renderer=Renderer::new();
        // Deliberately use an RTL base line, an LTR base line, and a third
        // RTL line. Numerals and parentheses must not cause source reversal.
        let passage="سَلَام (ABC 123), שָׁלוֹם!\nEnglish XYZ 42\nשלום 123";
        let spans=vec![(passage.to_owned(),Color32::WHITE)];
        let lines=renderer.paragraph_directions(passage);
        assert!(lines.iter().any(|rtl|*rtl),"RTL line missing");
        assert!(lines.iter().any(|rtl|!*rtl),"LTR line missing");
        let starts=[0,"سَلَام (ABC 123), שָׁלוֹם!\n".len(),
            "سَلَام (ABC 123), שָׁלוֹם!\nEnglish XYZ 42\n".len()];
        let positions=starts.into_iter().map(|cursor|{
            renderer.render_with_caret(&spans,760,150,Some(cursor))
                .unwrap().caret.expect("grapheme boundary must map to a caret")
        }).collect::<Vec<_>>();
        assert!(positions[1].1>positions[0].1+12.0,"LTR paragraph caret in RTL line");
        assert!(positions[2].1>positions[1].1+12.0,"final RTL caret in LTR line");
        for (x,y) in positions {
            assert!(x>=0.0 && x<=760.0 && y>=0.0 && y<150.0);
        }
        // A niqqud combining cluster is one cursor step, even in mixed text.
        let cluster_start="سَلَام (ABC 123), ".len();
        let inside_cluster=cluster_start+"ש".len();
        assert!(renderer.render_with_caret(&spans,760,150,Some(inside_cluster))
            .unwrap().caret.is_none(),"mixed BiDi cursor must not split niqqud");
    }
    #[test] fn mixed_direction_error_raster_tracks_logical_arabic_prefix(){
        let mut renderer=Renderer::new();
        let neutral=renderer.render(&[
            ("سلام English 42, שלום!".into(),Color32::WHITE)
        ],760,108).unwrap();
        let mistake=renderer.render(&[
            ("س".into(),Color32::from_rgb(220,55,55)),
            ("لام English 42, שלום!".into(),Color32::WHITE)
        ],760,108).unwrap();
        let changed=neutral.pixels.iter().zip(&mistake.pixels)
            .filter(|(a,b)|a!=b).count();
        assert!(changed>8 && changed<760*108,
            "mixed RTL error must change the shaped ink, not reorder the source");
    }
    #[test] fn styled_rtl_mistake_changes_raster_ink_without_reversing_source(){
        let mut renderer=Renderer::new();
        let text="سلام";
        let neutral=renderer.render(&[(text.to_owned(),Color32::from_rgb(160,160,160))],
            560,96).unwrap();
        let highlighted=renderer.render(&[
            ("س".to_owned(),Color32::from_rgb(220,50,65)),
            ("لام".to_owned(),Color32::from_rgb(160,160,160)),
        ],560,96).unwrap();
        assert_eq!(neutral.size,highlighted.size);
        let differences=neutral.pixels.iter().zip(&highlighted.pixels)
            .filter(|(a,b)|a!=b).count();
        assert!(differences>12,
            "actual shaped raster must respond to reference error coloring");
        assert!(differences<560*96,"render should retain transparent padding");
    }
    #[test] fn shaped_rtl_output_is_bounded_nonblank_and_transparent(){
        let mut renderer=Renderer::new();
        let spans=vec![("سَلَام بالعربية".to_owned(),Color32::from_rgb(245,245,245))];
        let result=renderer.render(&spans,640,96).unwrap();
        assert_eq!(result.size,[640,96]);
        let ink=result.pixels.iter().filter(|pixel|pixel.a()>0).count();
        assert!(ink>50,"system-installed font must draw shaped Arabic glyphs");
        assert!(ink<640*96,"renderer background must remain transparent");
        assert!(renderer.render(&spans,4096,96).is_none());
        assert!(renderer.render(&[("x".repeat(16_385),Color32::WHITE)],640,96).is_none());
    }
}
