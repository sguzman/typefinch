//! A standalone, static, script-free GitHub-compatible SVG snapshot.
use crate::{history::{self, Record}, preferences::{base_dir, Profile}};
use std::{fs, path::PathBuf};

fn xml(s: &str) -> String {
    s.replace('&',"&amp;").replace('<',"&lt;").replace('>',"&gt;")
     .replace('"',"&quot;").replace('\'',"&apos;")
}
pub fn card(profile: &Profile, layout: &str) -> String {
    let name=xml(&profile.name);
    let layout=xml(layout);
    format!(r##"<svg xmlns="http://www.w3.org/2000/svg" width="550" height="205" viewBox="0 0 550 205" role="img" aria-label="Typefinch profile">
<rect width="550" height="205" rx="22" fill="#202645"/>
<rect x="1" y="1" width="548" height="203" rx="21" fill="none" stroke="#8696F5" stroke-width="2"/>
<text x="28" y="44" fill="#BCC6FF" font-size="22" font-family="sans-serif" font-weight="700">Typefinch</text>
<rect x="465" y="18" width="62" height="62" rx="17" fill="#303864"/>
<ellipse cx="492" cy="51" rx="18" ry="14" fill="#8696F5"/>
<circle cx="507" cy="36" r="10" fill="#8696F5"/>
<circle cx="510" cy="33" r="2" fill="#202645"/>
<path d="M517 38 L527 42 L517 46Z" fill="#F6C376"/>
<path d="M470 49 Q480 35 491 45 Q484 57 470 49Z" fill="#BCC6FF"/>
<text x="28" y="76" fill="#F1F2FF" font-size="19" font-family="sans-serif">{name}</text>
<text x="29" y="111" fill="#9FAADA" font-family="sans-serif" font-size="13">BEST WPM</text>
<text x="29" y="148" fill="#FFFFFF" font-family="sans-serif" font-size="32" font-weight="700">{:.0}</text>
<text x="200" y="111" fill="#9FAADA" font-family="sans-serif" font-size="13">TESTS</text>
<text x="200" y="148" fill="#FFFFFF" font-family="sans-serif" font-size="32" font-weight="700">{}</text>
<text x="370" y="111" fill="#9FAADA" font-family="sans-serif" font-size="13">BEST ACCURACY</text>
<text x="370" y="148" fill="#FFFFFF" font-family="sans-serif" font-size="32" font-weight="700">{:.1}%</text>
<text x="29" y="184" fill="#BEC7F3" font-family="sans-serif" font-size="13">Layout: {layout} · Benchmark records only</text>
</svg>"##, profile.best_wpm, profile.tests, profile.best_accuracy)
}
pub fn markdown_embed() -> &'static str {
    "![Typefinch typing profile](./typefinch-profile.svg)\n"
}
pub fn save_widget(profile: &Profile, layout: &str) -> Result<(PathBuf, PathBuf), String> {
    let svg = save_card(profile, layout)?;
    let md = svg.with_file_name("typefinch-profile.md");
    fs::write(&md, markdown_embed()).map_err(|e| e.to_string())?;
    Ok((svg, md))
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
pub fn save_card(profile: &Profile, layout: &str) -> Result<PathBuf,String> {
    let dir=base_dir().join("exports");
    fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let output=dir.join("typefinch-profile.svg");
    fs::write(&output, card(profile,layout)).map_err(|e| e.to_string())?;
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
