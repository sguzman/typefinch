//! A standalone, static, script-free GitHub-compatible SVG snapshot.
use crate::{history::{self, Record}, preferences::{base_dir, Preferences, Profile,CardTheme,CardFont,CardTemplate}};
use std::{fs, path::PathBuf, time::{SystemTime, UNIX_EPOCH}};

fn xml(s: &str) -> String {
    s.replace('&',"&amp;").replace('<',"&lt;").replace('>',"&gt;")
     .replace('"',"&quot;").replace('\'',"&apos;")
}
/// Static, approved palettes. Text ink is chosen independently from accent
/// decoration for adequate contrast against both dark and light backgrounds.
#[derive(Clone,Copy)]
pub struct CardPalette {
    pub bg:&'static str,pub border:&'static str,pub heading:&'static str,
    pub emblem_bg:&'static str,pub bird:&'static str,pub wing:&'static str,
    pub eye:&'static str,pub beak:&'static str,pub name:&'static str,
    pub labels:&'static str,pub numbers:&'static str,pub footer:&'static str,
}
pub fn palette(theme:CardTheme)->CardPalette {
    match theme {
        CardTheme::Dusk=>CardPalette{
            bg:"#202645",border:"#8696F5",heading:"#BCC6FF",
            emblem_bg:"#303864",bird:"#8696F5",wing:"#BCC6FF",
            eye:"#202645",beak:"#F6C376",name:"#F1F2FF",
            labels:"#9FAADA",numbers:"#FFFFFF",footer:"#BEC7F3"},
        CardTheme::Daybreak=>CardPalette{
            bg:"#F5F6FF",border:"#526ABD",heading:"#364C92",
            emblem_bg:"#DEE4FF",bird:"#8696F5",wing:"#445BA5",
            eye:"#222D56",beak:"#A36B20",name:"#24315B",
            labels:"#495982",numbers:"#24315B",footer:"#4D5B80"},
        CardTheme::Fern=>CardPalette{
            bg:"#173C36",border:"#86D6AD",heading:"#D4FFE5",
            emblem_bg:"#225148",bird:"#74B996",wing:"#B9E8CB",
            eye:"#122D27",beak:"#F6C376",name:"#F5FFF8",
            labels:"#C5E8D7",numbers:"#FFFFFF",footer:"#C3E8D1"},
    }
}
pub fn card(profile: &Profile, layout: &str) -> String {
    card_with_style(profile,layout,CardTheme::Dusk,CardFont::Sans)
}
pub fn card_with_style(profile:&Profile,layout:&str,theme:CardTheme,font:CardFont)->String {
    let p=palette(theme);
    let name=xml(&profile.name);
    let layout=xml(layout);
    let family=font.svg_family();
    format!(r##"<svg xmlns="http://www.w3.org/2000/svg" width="550" height="205" viewBox="0 0 550 205" role="img" aria-label="Typefinch profile">
<rect width="550" height="205" rx="22" fill="{bg}"/>
<rect x="1" y="1" width="548" height="203" rx="21" fill="none" stroke="{border}" stroke-width="2"/>
<text x="28" y="44" fill="{heading}" font-size="22" font-family="{family}" font-weight="700">Typefinch</text>
<rect x="465" y="18" width="62" height="62" rx="17" fill="{emblem_bg}"/>
<ellipse cx="492" cy="51" rx="18" ry="14" fill="{bird}"/>
<circle cx="507" cy="36" r="10" fill="{bird}"/>
<circle cx="510" cy="33" r="2" fill="{eye}"/>
<path d="M517 38 L527 42 L517 46Z" fill="{beak}"/>
<path d="M470 49 Q480 35 491 45 Q484 57 470 49Z" fill="{wing}"/>
<text x="28" y="76" fill="{name_ink}" font-size="19" font-family="{family}">{name}</text>
<text x="29" y="111" fill="{labels}" font-family="{family}" font-size="13">BEST WPM</text>
<text x="29" y="148" fill="{numbers}" font-family="{family}" font-size="32" font-weight="700">{:.0}</text>
<text x="200" y="111" fill="{labels}" font-family="{family}" font-size="13">SESSIONS</text>
<text x="200" y="148" fill="{numbers}" font-family="{family}" font-size="32" font-weight="700">{}</text>
<text x="370" y="111" fill="{labels}" font-family="{family}" font-size="13">BEST ACCURACY</text>
<text x="370" y="148" fill="{numbers}" font-family="{family}" font-size="32" font-weight="700">{:.1}%</text>
<text x="29" y="184" fill="{footer}" font-family="{family}" font-size="13">Layout: {layout} · Timed-test bests, all sessions counted</text>
</svg>"##, profile.best_wpm, profile.tests, profile.best_accuracy,
    bg=p.bg,border=p.border,heading=p.heading,emblem_bg=p.emblem_bg,
    bird=p.bird,wing=p.wing,eye=p.eye,beak=p.beak,name_ink=p.name,
    labels=p.labels,numbers=p.numbers,footer=p.footer,family=family)
}
/// Two actual SVG geometry templates; both use the same owned palette and
/// exclusively static SVG text/shapes, suitable for GitHub image rendering.
pub fn card_with_template(profile:&Profile, layout:&str, theme:CardTheme,font:CardFont,
    template:CardTemplate)->String {
    if template==CardTemplate::Classic {return card_with_style(profile,layout,theme,font);}
    let p=palette(theme);
    let family=font.svg_family();
    let name=xml(&profile.name.chars().take(32).collect::<String>());
    let layout=xml(&layout.chars().take(34).collect::<String>());
    format!(r##"<svg xmlns="http://www.w3.org/2000/svg" width="550" height="145" viewBox="0 0 550 145" role="img" aria-label="Typefinch compact profile">
<rect width="550" height="145" rx="20" fill="{bg}"/>
<rect x="1" y="1" width="548" height="143" rx="19" fill="none" stroke="{border}" stroke-width="2"/>
<text x="23" y="31" fill="{heading}" font-family="{family}" font-weight="700" font-size="20">Typefinch</text>
<text x="155" y="30" fill="{name_ink}" font-family="{family}" font-size="16">{name}</text>
<rect x="500" y="11" width="34" height="34" rx="10" fill="{emblem_bg}"/>
<ellipse cx="516" cy="30" rx="10" ry="8" fill="{bird}"/>
<circle cx="521" cy="23" r="5" fill="{wing}"/>
<circle cx="523" cy="21" r="1.3" fill="{eye}"/>
<path d="M526 23 L533 26 L526 29Z" fill="{beak}"/>
<text x="23" y="66" fill="{labels}" font-family="{family}" font-size="11">BEST WPM</text>
<text x="23" y="96" fill="{numbers}" font-family="{family}" font-size="27" font-weight="700">{:.0}</text>
<text x="204" y="66" fill="{labels}" font-family="{family}" font-size="11">SESSIONS</text>
<text x="204" y="96" fill="{numbers}" font-family="{family}" font-size="27" font-weight="700">{}</text>
<text x="379" y="66" fill="{labels}" font-family="{family}" font-size="11">BEST ACCURACY</text>
<text x="379" y="96" fill="{numbers}" font-family="{family}" font-size="27" font-weight="700">{:.1}%</text>
<text x="23" y="128" fill="{footer}" font-family="{family}" font-size="11">Layout: {layout} · Timed bests · All sessions counted</text>
</svg>"##,profile.best_wpm,profile.tests,profile.best_accuracy,
        bg=p.bg,border=p.border,heading=p.heading,name_ink=p.name,emblem_bg=p.emblem_bg,
        bird=p.bird,wing=p.wing,eye=p.eye,beak=p.beak,labels=p.labels,
        numbers=p.numbers,footer=p.footer,family=family)
}
pub fn markdown_embed() -> &'static str {
    "![Typefinch typing profile](./typefinch-profile.svg)\n"
}
pub fn save_widget(profile: &Profile, layout: &str) -> Result<(PathBuf, PathBuf), String> {
    save_widget_with_style(profile,layout,CardTheme::Dusk,CardFont::Sans)
}
pub fn save_widget_with_style(profile:&Profile,layout:&str,theme:CardTheme,font:CardFont)
    -> Result<(PathBuf, PathBuf),String> {
    let svg = save_card_with_template(profile,layout,theme,font,CardTemplate::Classic)?;
    let md = svg.with_file_name("typefinch-profile.md");
    fs::write(&md, markdown_embed()).map_err(|e| e.to_string())?;
    Ok((svg, md))
}
pub fn save_widget_with_template(profile:&Profile,layout:&str,theme:CardTheme,
    font:CardFont,template:CardTemplate)->Result<(PathBuf,PathBuf),String> {
    let svg=save_card_with_template(profile,layout,theme,font,template)?;
    let md=svg.with_file_name("typefinch-profile.md");
    fs::write(&md,markdown_embed()).map_err(|e|e.to_string())?;
    Ok((svg,md))
}
/// Versioned, explicit offline export. No profile nickname, typed text,
/// credentials, or physical keyboard identifiers are part of this payload.
pub fn stats_json(profile_id:&str,records:&[Record])->Result<Vec<u8>,String> {
    serde_json::to_vec_pretty(&serde_json::json!({
        "format":"typefinch-session-history",
        "version":1,
        "profile_id":profile_id,
        "order":"newest-first",
        "sessions":records,
    })).map_err(|e|e.to_string())
}
pub fn save_stats_json(profile:&Profile)->Result<PathBuf,String> {
    let records=history::all_for_profile(&profile.id)?;
    let output=base_dir().join("exports").join("typefinch-history.json");
    if let Some(parent)=output.parent() {fs::create_dir_all(parent).map_err(|e|e.to_string())?;}
    let bytes=stats_json(&profile.id,&records)?;
    let tmp=output.with_extension("json.tmp");
    fs::write(&tmp,bytes).map_err(|e|e.to_string())?;
    fs::rename(&tmp,&output).map_err(|e|e.to_string())?;
    Ok(output)
}
/// Explicit full local backup, separate from publication-safe SVG/JSON exports.
/// These files contain profile names, settings and complete aggregate history.
pub fn save_backup(prefs:&Preferences)->Result<(PathBuf,PathBuf),String> {
    let folder=base_dir().join("exports");
    fs::create_dir_all(&folder).map_err(|e|e.to_string())?;
    let stamp=SystemTime::now().duration_since(UNIX_EPOCH)
        .map_err(|e|e.to_string())?.as_nanos();
    let stem=format!("typefinch-backup-{stamp}");
    let settings=folder.join(format!("{stem}-settings.json"));
    let database=folder.join(format!("{stem}-sessions.sqlite3"));
    let bytes=serde_json::to_vec_pretty(prefs).map_err(|e|e.to_string())?;
    // Use create_new to protect any existing file against accidental overwrite.
    use std::io::Write;
    let mut options=fs::OpenOptions::new();
    options.create_new(true).write(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let mut handle=options.open(&settings).map_err(|e|e.to_string())?;
    if let Err(e)=handle.write_all(&bytes).and_then(|_|handle.sync_all()) {
        let _=fs::remove_file(&settings);
        return Err(e.to_string());
    }
    drop(handle);
    if let Err(e)=history::snapshot_database(&database) {
        let _=fs::remove_file(&settings);
        return Err(e);
    }
    Ok((settings,database))
}
/// Render only Typefinch-generated, script-free SVG using Rust libraries.
/// Text outlines are resolved locally with the system font database.
pub fn rasterize_card(svg:&str)->Result<Vec<u8>,String> {
    let mut opts=resvg::usvg::Options::default();
    opts.fontdb_mut().load_system_fonts();
    let tree=resvg::usvg::Tree::from_str(svg,&opts).map_err(|e|e.to_string())?;
    let size=tree.size().to_int_size();
    if size.width()>4096 || size.height()>4096 {return Err("Card dimensions exceed 4096 px".into());}
    let mut pixmap=resvg::tiny_skia::Pixmap::new(size.width(),size.height())
        .ok_or("Could not allocate card pixmap")?;
    resvg::render(&tree,resvg::tiny_skia::Transform::default(),&mut pixmap.as_mut());
    pixmap.encode_png().map_err(|e|e.to_string())
}
pub fn save_card_png(profile:&Profile,layout:&str,theme:CardTheme,font:CardFont)
    ->Result<PathBuf,String> {
    save_card_png_with_template(profile,layout,theme,font,CardTemplate::Classic)
}
pub fn save_card_png_with_template(profile:&Profile,layout:&str,theme:CardTheme,font:CardFont,
    template:CardTemplate)->Result<PathBuf,String> {
    let folder=base_dir().join("exports");
    fs::create_dir_all(&folder).map_err(|e|e.to_string())?;
    let output=folder.join("typefinch-profile.png");
    let png=rasterize_card(&card_with_template(profile,layout,theme,font,template))?;
    let temp=output.with_extension("png.tmp");
    fs::write(&temp,png).map_err(|e|e.to_string())?;
    fs::rename(&temp,&output).map_err(|e|e.to_string())?;
    Ok(output)
}
pub fn save_card_with_template(profile:&Profile,layout:&str,theme:CardTheme,
    font:CardFont,template:CardTemplate)->Result<PathBuf,String> {
    let dir=base_dir().join("exports");
    fs::create_dir_all(&dir).map_err(|e|e.to_string())?;
    let output=dir.join("typefinch-profile.svg");
    fs::write(&output,card_with_template(profile,layout,theme,font,template))
        .map_err(|e|e.to_string())?;
    Ok(output)
}
pub fn save_card(profile: &Profile, layout: &str) -> Result<PathBuf,String> {
    save_card_with_style(profile,layout,CardTheme::Dusk,CardFont::Sans)
}
pub fn save_card_with_style(profile:&Profile,layout:&str,theme:CardTheme,font:CardFont)
    -> Result<PathBuf,String> {
    let dir=base_dir().join("exports");
    fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let output=dir.join("typefinch-profile.svg");
    fs::write(&output, card_with_style(profile,layout,theme,font)).map_err(|e| e.to_string())?;
    Ok(output)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn history_json_is_versioned_and_excludes_raw_text() {
        let mut session=crate::engine::Session::new_with_custom(30,crate::engine::TestMode::Custom,"a secret test sentence");
        session.begin();
        session.type_text("a");
        let rec=Record::completed("private-id","os:unknown","OS",&session);
        let serialized=String::from_utf8(stats_json("private-id",&[rec]).unwrap()).unwrap();
        let parsed:serde_json::Value=serde_json::from_str(&serialized).unwrap();
        assert_eq!(parsed["version"],1);
        assert_eq!(parsed["sessions"].as_array().unwrap().len(),1);
        assert_eq!(parsed["sessions"][0]["test_mode"],"custom");
        assert!(!serialized.contains("secret test sentence"));
        assert!(!serialized.contains("typed"));
        assert!(!serialized.contains("hardware_address"));
    }
    #[test] fn palettes_and_type_options_keep_static_svg_markup_safe() {
        let p=Profile {name:"<script>&".into(), ..Default::default()};
        for theme in CardTheme::ALL {
            for font in CardFont::ALL {
                let data=card_with_style(&p,"us(colemak)",theme,font);
                assert!(data.starts_with("<svg"));
                assert!(!data.contains("<script"));
                assert!(data.contains("&lt;script&gt;&amp;"));
                assert!(data.contains(palette(theme).bg));
                assert!(data.contains(&format!("font-family=\"{}\"",font.svg_family())));
            }
        }
        assert_ne!(palette(CardTheme::Dusk).bg,palette(CardTheme::Daybreak).bg);
    }
    #[test] fn compact_svg_has_independent_geometry_and_png_dimensions() {
        let p=Profile {name:"<Finch>&".into(),..Default::default()};
        for palette in CardTheme::ALL {
            let compact=card_with_template(&p,"us",palette,CardFont::Mono,CardTemplate::Compact);
            assert!(compact.contains("height=\"145\""));
            assert!(compact.contains("&lt;Finch&gt;&amp;"));
            assert!(!compact.contains("<script"));
            let png=rasterize_card(&compact).unwrap();
            assert_eq!(u32::from_be_bytes(png[16..20].try_into().unwrap()),550);
            assert_eq!(u32::from_be_bytes(png[20..24].try_into().unwrap()),145);
        }
    }
    #[test] fn raster_export_is_a_real_png_with_expected_dimensions() {
        let p=Profile::default();
        let png=rasterize_card(&card_with_style(&p,"us",CardTheme::Fern,CardFont::Sans)).unwrap();
        assert!(png.starts_with(&[137,80,78,71,13,10,26,10]));
        let width=u32::from_be_bytes(png[16..20].try_into().unwrap());
        let height=u32::from_be_bytes(png[20..24].try_into().unwrap());
        assert_eq!((width,height),(550,205));
        assert!(png.len()>512);
    }
    #[test] fn markdown_link_uses_local_svg_without_remote_fetch() {
        assert_eq!(markdown_embed(),"![Typefinch typing profile](./typefinch-profile.svg)\n");
        assert!(!markdown_embed().contains("http"));
    }
    #[test] fn output_escapes_untrusted_profile_text() {
        let p=Profile { name:"<script>&".into(), ..Default::default() };
        let svg=card(&p, "<svg>");
        assert!(!svg.contains("<script>"));
        assert!(svg.contains("&lt;script&gt;&amp;"));
        assert!(svg.contains("&lt;svg&gt;"));
    }
}
