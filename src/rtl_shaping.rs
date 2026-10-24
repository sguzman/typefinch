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
            .filter(|(x,y)|x.is_finite() && y.is_finite());
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
