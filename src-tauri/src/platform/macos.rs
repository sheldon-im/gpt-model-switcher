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
pub fn wait_shortcut_release()->Result<(),String>{request(&["release"]).map(|_|())}
pub fn watch(callback:impl Fn(bool)+Send+Sync+'static){std::thread::spawn(move||{use std::io::{BufRead,BufReader};let Ok(path)=helper()else{callback(false);return};let Ok(mut child)=Command::new(path).arg("watch").stdout(std::process::Stdio::piped()).stderr(std::process::Stdio::null()).spawn()else{callback(false);return};if let Some(out)=child.stdout.take(){for line in BufReader::new(out).lines().map_while(Result::ok){if let Ok(v)=serde_json::from_str::<Value>(&line){if let Some(active)=v["active"].as_bool(){callback(active)}}}}let _=child.wait();callback(false);});}
pub fn inspect()->Status{request(&["inspect"]).and_then(|v|serde_json::from_value(v).map_err(|e|e.to_string())).unwrap_or_else(Status::unavailable)}
use switcher_core::{Preset,Selection,switching::{Target,Outcome,apply}};
struct MacTarget{context:String}
impl MacTarget{fn new()->Result<Self,String>{let value=request(&["inspect"])?;let mut context=value["context"].clone();context["deadline"]=serde_json::json!(std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_err(|e|e.to_string())?.as_secs_f64()+8.0);Ok(Self{context:serde_json::to_string(&context).map_err(|e|e.to_string())?})}}
impl Target for MacTarget{
 fn check(&mut self)->Result<(),String>{request(&["check",&self.context]).map(|_|())}
 fn busy(&mut self)->Result<bool,String>{request(&["inspect"]).and_then(|v|v["busy"].as_bool().ok_or("Missing generation state".into()))}
 fn read(&mut self)->Result<Selection,String>{self.check()?;request(&["inspect"]).and_then(|v|serde_json::from_value(v["selection"].clone()).map_err(|e|e.to_string()))}
 fn model(&mut self,v:&str)->Result<(),String>{request(&["model",v,&self.context]).map(|_|())}
 fn effort(&mut self,v:&str)->Result<(),String>{request(&["effort",v,&self.context]).map(|_|())}
 fn finish(&mut self)->Result<(),String>{request(&["finish",&self.context]).map(|_|())}
}
pub fn switch(p:&Preset)->Outcome{match MacTarget::new(){Ok(mut t)=>apply(&mut t,p),Err(e)=>Outcome{success:false,message:e,actual:None,restored:false}}}


