use serde::{Serialize,Deserialize};
use switcher_core::Selection;
#[derive(Clone,Debug,Serialize,Deserialize)]
pub struct Status {pub active:bool,pub available:bool,pub busy:bool,pub message:String,pub selection:Option<Selection>}
impl Status {pub fn unavailable(message:impl Into<String>)->Self{Self{active:false,available:false,busy:false,message:message.into(),selection:None}}}
#[cfg(windows)] mod windows;
#[cfg(windows)] pub use windows::*;
#[cfg(target_os="macos")] mod macos;
#[cfg(target_os="macos")] pub use macos::*;
#[cfg(not(target_os="macos"))] pub fn accessibility(_prompt:bool)->Result<bool,String>{Ok(true)}
#[cfg(not(target_os="macos"))] pub fn open_accessibility_settings()->Result<(),String>{Ok(())}
#[cfg(not(any(windows,target_os="macos")))] pub fn inspect()->Status{Status::unavailable("Unsupported OS")}

