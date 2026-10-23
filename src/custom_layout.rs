//! Versioned, local-only ANSI mappings; never changes the compositor's keymap.
use eframe::egui::Key;
use serde::{Deserialize,Serialize};
use unicode_segmentation::UnicodeSegmentation;

pub const SCHEMA_VERSION:u32=1;
#[derive(Clone,Debug,PartialEq,Eq,Serialize,Deserialize)]
pub struct CustomLayout {
    pub version:u32,
    pub id:String,
    pub name:String,
    pub base:[String;3],
    pub shifted:[String;3],
}
impl CustomLayout {
    pub fn starter()->Self {
        Self {version:SCHEMA_VERSION,id:format!("custom/{}",uuid::Uuid::new_v4()),
            name:"Personal ANSI".into(),
            base:["qwertyuiop".into(),"asdfghjkl;".into(),"zxcvbnm,./".into()],
            shifted:["QWERTYUIOP".into(),"ASDFGHJKL:".into(),"ZXCVBNM<>?".into()]}
    }
    pub fn validate(&self)->Result<(),String> {
        if self.version!=SCHEMA_VERSION {return Err("Unsupported layout version".into());}
        let id=self.id.strip_prefix("custom/").ok_or("Custom ID prefix is invalid")?;
        uuid::Uuid::parse_str(id).map_err(|_|"Custom ID must be UUID")?;
        let name=self.name.trim();
        if name.is_empty() || name.graphemes(true).count()>32 || name.chars().any(char::is_control) {
            return Err("Name must be 1-32 graphemes and control-free".into());
        }
        for row in self.base.iter().chain(self.shifted.iter()) {
            if row.graphemes(true).count()!=10 {return Err("Mapping rows must have ten graphemes".into());}
            if row.chars().any(char::is_control) {return Err("Control characters are forbidden".into());}
        }
        Ok(())
    }
    pub fn translate(&self,key:Key,shift:bool)->Option<String> {
        if key==Key::Space {return Some(" ".into());}
        let (r,c)=crate::layouts::physical_index(key)?;
        let rows=if shift {&self.shifted} else {&self.base};
        rows[r].graphemes(true).nth(c).map(str::to_owned)
    }
    pub fn as_json(&self)->Result<Vec<u8>,String> {
        self.validate()?;
        serde_json::to_vec_pretty(self).map_err(|e|e.to_string())
    }
    pub fn from_json(bytes:&[u8])->Result<Self,String> {
        if bytes.len()>16_384 {return Err("Layout JSON exceeds 16 KiB".into());}
        let result:Self=serde_json::from_slice(bytes).map_err(|e|e.to_string())?;
        result.validate()?;
        Ok(result)
    }
    pub fn new_identity(&mut self){self.id=format!("custom/{}",uuid::Uuid::new_v4());}
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn starter_maps_ansi_and_roundtrips_json() {
        let p=CustomLayout::starter();
        p.validate().unwrap();
        assert_eq!(p.translate(Key::Q,false).as_deref(),Some("q"));
        assert_eq!(p.translate(Key::Q,true).as_deref(),Some("Q"));
        assert_eq!(p.translate(Key::Semicolon,true).as_deref(),Some(":"));
        assert_eq!(p.translate(Key::Space,true).as_deref(),Some(" "));
        assert_eq!(CustomLayout::from_json(&p.as_json().unwrap()).unwrap(),p);
    }
    #[test] fn untrusted_import_requires_valid_identity_and_geometry() {
        let mut p=CustomLayout::starter();
        p.version=99;assert!(p.validate().is_err());
        p.version=SCHEMA_VERSION;
        p.base[0]="invalid".into();assert!(p.validate().is_err());
        p.base[0]="qwertyuiop".into();
        p.name="".into();assert!(p.validate().is_err());
        p.name="Personal".into();
        p.id="../bad.json".into();assert!(p.validate().is_err());
        assert!(CustomLayout::from_json(&vec![0;16385]).is_err());
    }
    #[test] fn combining_graphemes_count_once_and_import_is_safe() {
        let mut p=CustomLayout::starter();
        p.base[0]="áe\u{301}rtyuiopq".into();
        p.validate().unwrap();
        assert_eq!(p.translate(Key::W,false).as_deref(),Some("e\u{301}"));
        p.new_identity();
        assert!(p.validate().is_ok());
    }
}