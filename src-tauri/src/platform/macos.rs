use super::Status;
use std::{process::Command,path::PathBuf};
use serde_json::Value;
fn helper()->Result<PathBuf,String>{
 let packaged=std::env::current_exe().map_err(|e|e.to_string())?.parent().ok_or("No executable directory")?.join("macos-bridge");
 if packaged.exists(){return Ok(packaged)}
 let dev=PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("native/macos-bridge-aarch64-apple-darwin");
 if dev.exists(){Ok(dev)}else{Err("macOS bridge not built. Run the macOS build instructions. / Mac 빌드 지침을 실행하십시오.".into())}
}
fn request(args:&[&str])->Result<Value,String>{
 let output=Command::new(helper()?).args(args).output().map_err(|e|e.to_string())?;
 if !output.status.success(){return Err(String::from_utf8_lossy(&output.stderr).into())}
 let v:Value=serde_json::from_slice(&output.stdout).map_err(|e|e.to_string())?;
 if let Some(e)=v.get("error").and_then(Value::as_str){Err(e.into())}else{Ok(v)}
}
pub fn active()->bool{request(&["foreground"]).ok().and_then(|v|v["active"].as_bool()).unwrap_or(false)}
pub fn inspect()->Status{request(&["inspect"]).and_then(|v|serde_json::from_value(v).map_err(|e|e.to_string())).unwrap_or_else(Status::unavailable)}
pub fn switch(_: &switcher_core::Preset)->switcher_core::switching::Outcome{switcher_core::switching::Outcome{success:false,message:"macOS switching adapter not connected".into(),actual:None,restored:false}}

