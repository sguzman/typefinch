//! Read-only, optional font fallback discovery. User font preferences win.
//! Fonts remain installed and maintained by the host Linux system; we do not
//! vendor or distribute font files and never traverse private directories.
use eframe::egui::{self,FontData,FontDefinitions,FontFamily};
use std::{fs,path::{Path,PathBuf},sync::Arc};

const MAX_FONT_BYTES:u64=40*1024*1024;
const SYSTEM_FONT_DIRS:&[&str]=&[
    "/usr/share/fonts", "/usr/local/share/fonts",
];
#[derive(Clone,Copy,Debug,Default)]
pub struct Fallbacks {pub cjk:bool,pub arabic:bool,pub hebrew:bool}
struct Candidate {label:&'static str,needle:&'static str,filenames:&'static [&'static str]}

const CANDIDATES:[Candidate;3]=[
    Candidate {label:"typefinch-noto-cjk",needle:"NotoSansCJK-Regular",
        filenames:&["NotoSansCJK-Regular.ttc","NotoSansCJKsc-Regular.otf",
            "NotoSansCJKsc-Regular.ttf"]},
    Candidate {label:"typefinch-noto-arabic",needle:"NotoSansArabic",
        filenames:&["NotoSansArabic-Regular.ttf"]},
    Candidate {label:"typefinch-noto-hebrew",needle:"NotoSansHebrew",
        filenames:&["NotoSansHebrew-Regular.ttf"]},
];
fn trusted_font(path:&Path)->bool {
    let Ok(real)=path.canonicalize() else{return false;};
    SYSTEM_FONT_DIRS.iter().any(|root|real.starts_with(root))
        && matches!(real.extension().and_then(|ext|ext.to_str()),
            Some("ttf"|"ttc"|"otf"))
        && fs::metadata(real).is_ok_and(|metadata|
            metadata.is_file() && metadata.len()>100 && metadata.len()<=MAX_FONT_BYTES)
}
fn candidate_paths(candidate:&Candidate)->Vec<PathBuf> {
    // Conventional Arch and Debian font layouts plus their versioned/static
    // filenames. Do not recurse outside the system-managed font roots.
    let directories=[
        "/usr/share/fonts/opentype/noto",
        "/usr/share/fonts/truetype/noto",
        "/usr/share/fonts/noto",
        "/usr/share/fonts/noto-cjk",
        "/usr/share/fonts/OTF",
        "/usr/share/fonts/TTF",
        "/usr/local/share/fonts",
    ];
    let mut paths=Vec::new();
    for directory in directories {
        let root=Path::new(directory);
        for name in candidate.filenames {
            paths.push(root.join(name));
        }
        if let Ok(entries)=fs::read_dir(root) {
            let mut matched=entries.flatten().filter_map(|entry|{
                let file=entry.file_name();
                let name=file.to_string_lossy();
                if name.starts_with(candidate.needle)
                    && matches!(entry.path().extension().and_then(|e|e.to_str()),
                        Some("ttf"|"ttc"|"otf")) {
                    Some(entry.path())
                } else {None}
            }).take(50).collect::<Vec<_>>();
            matched.sort();
            paths.extend(matched);
        }
    }
    paths
}
fn load_first(candidate:&Candidate)->Option<Vec<u8>> {
    candidate_paths(candidate).into_iter()
        .find(|path|trusted_font(path))
        .and_then(|path|fs::read(path).ok())
        .filter(|bytes|bytes.len()>=4
            && matches!(&bytes[..4], b"ttcf"|b"OTTO"|b"true"|b"\0\x01\0\0"))
}
pub fn install(ctx:&egui::Context)->Fallbacks {
    let mut defs=FontDefinitions::default();
    let mut available=Fallbacks::default();
    for (index,candidate) in CANDIDATES.iter().enumerate() {
        if let Some(bytes)=load_first(candidate) {
            let name=candidate.label.to_owned();
            defs.font_data.insert(name.clone(),Arc::new(FontData::from_owned(bytes)));
            for family in [FontFamily::Proportional,FontFamily::Monospace] {
                defs.families.entry(family).or_default().push(name.clone());
            }
            match index {
                0=>available.cjk=true,1=>available.arabic=true,2=>available.hebrew=true,
                _=>unreachable!()
            }
        }
    }
    if available.cjk || available.arabic || available.hebrew {ctx.set_fonts(defs);}
    available
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn rejects_private_and_invalid_system_fonts() {
        assert!(!trusted_font(Path::new("/etc/passwd")));
        assert!(!trusted_font(Path::new("/dev/null")));
        assert!(!trusted_font(Path::new("/tmp/not-installed.ttf")));
    }
    #[test] fn system_noto_fallbacks_render_required_glyphs() {
        if std::env::var_os("TYPEFINCH_QA_UNICODE_FONTS").is_none(){return;}
        let ctx=egui::Context::default();
        let status=install(&ctx);
        assert!(status.cjk && status.arabic && status.hebrew,
            "CI must provide CJK, Arabic and Hebrew system Noto packages: {status:?}");
        ctx.begin_pass(egui::RawInput::default());
        for family in [FontFamily::Proportional,FontFamily::Monospace] {
            let font=egui::FontId::new(18.0,family);
            ctx.fonts_mut(|fonts|{
                for glyph in ['你','好','漢','م','س','א','ש'] {
                    assert!(fonts.has_glyph(&font,glyph),
                        "installed fallback cannot render {glyph} for {font:?}");
                }
            });
        }
        let _=ctx.end_pass();
    }
}
