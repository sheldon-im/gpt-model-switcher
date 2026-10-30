use serde::{Deserialize, Serialize};
pub mod switching;
use std::{collections::HashSet, fs, path::Path};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct Preset {
    pub id: String,
    pub name: String,
    pub model: String,
    pub effort: String,
    pub windows_shortcut: String,
    pub macos_shortcut: String,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct Settings {
    pub version: u32,
    pub language: String,
    pub visual_notifications: bool,
    pub screen_reader_notifications: bool,
    pub presets: Vec<Preset>,
}
impl Default for Settings {
    fn default() -> Self {
        Self { version: 1, language: "ko".into(), visual_notifications: true,
            screen_reader_notifications: true,
            presets: [("daily", "일상용", "gpt-6.1-terra", "medium"),
                      ("coding", "코딩용", "gpt-6.1-sol", "high")].into_iter().enumerate()
                .map(|(i,(id,name,model,effort))| Preset { id:id.into(),name:name.into(),model:model.into(),effort:effort.into(),
                    windows_shortcut:format!("Control+Alt+Shift+{}",i+1),macos_shortcut:format!("Super+Alt+Shift+{}",i+1) }).collect() }
    }
}
pub const EFFORTS: &[&str] = &["none","minimal","low","medium","high","xhigh","max","ultra"];
pub fn normalized(s: &str) -> String { s.chars().filter(|c|c.is_alphanumeric()).flat_map(char::to_lowercase).collect() }
pub fn same_model(a:&str,b:&str)->bool { normalized(a)==normalized(b) }
pub fn shortcut_key(s:&str)->Result<String,String> {
    let parts:Vec<_>=s.split('+').map(str::trim).collect();
    if parts.len()<2 || parts.iter().any(|s|s.is_empty()) { return Err("단축키는 보조 키와 일반 키를 함께 지정하십시오. / Use a modifier and a key.".into()); }
    let mut modifiers=Vec::new();
    for p in &parts[..parts.len()-1] {
        let m=match p.to_ascii_lowercase().as_str(){"control"|"ctrl"=>"control","alt"|"option"=>"alt","shift"=>"shift","super"|"command"|"meta"=>"super",_=>return Err(format!("Unknown modifier: {p}"))};
        if modifiers.contains(&m){return Err("Duplicate modifier".into())} modifiers.push(m);
    }
    modifiers.sort();
    let key=parts.last().unwrap().to_ascii_lowercase();
    let valid=key.len()==1 && key.chars().all(|c|c.is_ascii_alphanumeric()) || key.strip_prefix('f').and_then(|n|n.parse::<u8>().ok()).is_some_and(|n|(1..=24).contains(&n));
    if !valid {return Err("지원 키: A–Z, 0–9, F1–F24 / Supported keys: A–Z, 0–9, F1–F24".into())}
    Ok(format!("{}+{}",modifiers.join("+"),key))
}
impl Settings {
    pub fn validate(&self)->Result<(),String> {
        if self.version!=1 {return Err("Unsupported settings version; original file preserved".into())}
        if !["ko","en"].contains(&self.language.as_str()){return Err("Unsupported language".into())}
        let (mut ids,mut win,mut mac)=(HashSet::new(),HashSet::new(),HashSet::new());
        for p in &self.presets {
            if p.id.is_empty() || p.name.trim().is_empty() || p.model.trim().is_empty() || !ids.insert(&p.id) {return Err("Invalid or duplicate preset / 프리셋 이름·모델·ID를 확인하십시오.".into())}
            if !EFFORTS.contains(&p.effort.as_str()){return Err("Unsupported effort".into())}
            if !win.insert(shortcut_key(&p.windows_shortcut)?) || !mac.insert(shortcut_key(&p.macos_shortcut)?){return Err("Duplicate shortcut / 중복된 단축키입니다.".into())}
        } Ok(())
    }
    pub fn load(path:&Path)->Result<Self,String>{
        if !path.exists(){return Ok(Self::default())}
        let s:Self=serde_json::from_slice(&fs::read(path).map_err(|e|e.to_string())?).map_err(|e|format!("Settings could not be read; original preserved: {e}"))?;
        s.validate()?; Ok(s)
    }
    pub fn save(&self,path:&Path)->Result<(),String>{
        self.validate()?;
        if let Some(parent)=path.parent(){fs::create_dir_all(parent).map_err(|e|e.to_string())?}
        let tmp=path.with_extension("json.tmp");
        fs::write(&tmp,serde_json::to_vec_pretty(self).map_err(|e|e.to_string())?).map_err(|e|e.to_string())?;
        // Preserve a recoverable copy on platforms where rename cannot replace an existing file.
        let backup=path.with_extension("json.bak");
        if path.exists(){fs::copy(path,&backup).map_err(|e|e.to_string())?;fs::remove_file(path).map_err(|e|e.to_string())?;}
        if let Err(e)=fs::rename(&tmp,path){if backup.exists(){let _=fs::copy(&backup,path);}return Err(e.to_string())} Ok(())
    }
}

#[derive(Clone,Debug,Serialize,Deserialize,PartialEq,Eq)]
pub struct Selection {pub model:String,pub effort:String}
impl Selection {pub fn matches(&self,p:&Preset)->bool{same_model(&self.model,&p.model)&&self.effort==p.effort}}
pub fn parse_selection(label:&str)->Option<Selection>{
    let aliases=[("extra high","xhigh"),("중간","medium"),("높음","high"),("낮음","low"),("최대","max"),("medium","medium"),("minimal","minimal"),("xhigh","xhigh"),("ultra","ultra"),("high","high"),("none","none"),("low","low"),("max","max")];
    let lower=label.to_lowercase();
    for (suffix,effort) in aliases {if lower.ends_with(suffix){let model=label[..label.len()-suffix.len()].trim();if model.to_lowercase().starts_with("gpt"){return Some(Selection{model:model.into(),effort:effort.into()})}}} None
}

#[cfg(test)] mod tests {
    use super::*;
    #[test] fn defaults_roundtrip(){let d=tempfile::tempdir().unwrap();let p=d.path().join("settings.json");let s=Settings::default();s.save(&p).unwrap();assert_eq!(s,Settings::load(&p).unwrap());}
    #[test] fn corrupt_file_preserved(){let d=tempfile::tempdir().unwrap();let p=d.path().join("settings.json");fs::write(&p,b"broken").unwrap();assert!(Settings::load(&p).is_err());assert_eq!(fs::read(&p).unwrap(),b"broken");}
    #[test] fn future_version_preserved(){let mut s=Settings::default();s.version=2;assert!(s.validate().is_err());}
    #[test] fn duplicate_alias_shortcuts_rejected(){let mut s=Settings::default();s.presets[1].windows_shortcut="Shift+Ctrl+Alt+1".into();assert!(s.validate().is_err());}
    #[test] fn non_modifier_and_duplicate_modifiers_rejected(){assert!(shortcut_key("A+B").is_err());assert!(shortcut_key("Ctrl+Control+A").is_err());assert!(shortcut_key("Shift+").is_err());}
    #[test] fn labels_match_exact_models(){let s=parse_selection("GPT-6.1 Sol High").unwrap();assert!(s.matches(&Settings::default().presets[1]));assert!(!same_model("gpt-6.1-sol-extra","gpt-6.1-sol"));assert_eq!(parse_selection("GPT-6 Astra Extra High").unwrap().effort,"xhigh");}
}
