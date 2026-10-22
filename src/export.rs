//! A standalone, static, script-free GitHub-compatible SVG snapshot.
use crate::preferences::{base_dir, Profile};
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
<text x="28" y="44" fill="#BCC6FF" font-size="22" font-family="sans-serif" font-weight="700">⌨ Typefinch</text>
<text x="28" y="76" fill="#F1F2FF" font-size="19" font-family="sans-serif">{name}</text>
<text x="29" y="111" fill="#9FAADA" font-family="sans-serif" font-size="13">BEST WPM</text>
<text x="29" y="148" fill="#FFFFFF" font-family="sans-serif" font-size="32" font-weight="700">{:.0}</text>
<text x="200" y="111" fill="#9FAADA" font-family="sans-serif" font-size="13">TESTS</text>
<text x="200" y="148" fill="#FFFFFF" font-family="sans-serif" font-size="32" font-weight="700">{}</text>
<text x="370" y="111" fill="#9FAADA" font-family="sans-serif" font-size="13">BEST ACCURACY</text>
<text x="370" y="148" fill="#FFFFFF" font-family="sans-serif" font-size="32" font-weight="700">{:.1}%</text>
<text x="29" y="184" fill="#BEC7F3" font-family="sans-serif" font-size="13">Layout: {layout} · Local profile snapshot</text>
</svg>"##, profile.best_wpm, profile.tests, profile.best_accuracy)
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
    #[test] fn output_escapes_untrusted_profile_text() {
        let p=Profile { name:"<script>&".into(), ..Default::default() };
        let svg=card(&p, "<svg>");
        assert!(!svg.contains("<script>"));
        assert!(svg.contains("&lt;script&gt;&amp;"));
        assert!(svg.contains("&lt;svg&gt;"));
    }
}
