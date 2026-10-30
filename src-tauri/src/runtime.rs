use crate::platform;
use std::{path::PathBuf,sync::{Mutex,atomic::{AtomicBool,Ordering}}};
use switcher_core::{Settings,Preset,switching::Outcome};
use tauri::{AppHandle,Manager,Emitter};
use tauri_plugin_global_shortcut::{GlobalShortcutExt,Shortcut,ShortcutState};
pub struct AppState{pub settings:Mutex<Settings>,pub path:PathBuf,pub load_error:Option<String>,pub busy:AtomicBool,pub bindings:Mutex<Option<String>>,pub history:Mutex<Vec<Outcome>>,gate:Mutex<()>}
impl AppState{pub fn new(path:PathBuf)->Self{let loaded=Settings::load(&path);Self{settings:Mutex::new(loaded.clone().unwrap_or_default()),path,load_error:loaded.err(),busy:AtomicBool::new(false),bindings:Mutex::new(None),history:Mutex::new(Vec::new()),gate:Mutex::new(())}}}
fn shortcut(p:&Preset)->&str{if cfg!(target_os="macos"){&p.macos_shortcut}else{&p.windows_shortcut}}
pub fn sync_shortcuts(app:&AppHandle,active:bool){
 let state=app.state::<AppState>();let Ok(_gate)=state.gate.lock()else{return};
 let result=(||{app.global_shortcut().unregister_all().map_err(|e|e.to_string())?;if state.load_error.is_some(){return Err("Settings file needs repair / 설정 파일을 확인하십시오.".into())}
 if !active{return Ok(())}let settings=state.settings.lock().map_err(|e|e.to_string())?.clone();
 for p in &settings.presets{app.global_shortcut().register(shortcut(p)).map_err(|e|format!("단축키 등록 실패 / Shortcut conflict: {} ({e})",shortcut(p)))?;}Ok::<_,String>(())})();
 if result.is_err(){let _=app.global_shortcut().unregister_all();}
 if let Ok(mut e)=state.bindings.lock(){*e=result.err();};
}
#[tauri::command] pub fn get_settings(state:tauri::State<AppState>)->Result<Settings,String>{if let Some(e)=&state.load_error{return Err(e.clone())}Ok(state.settings.lock().map_err(|e|e.to_string())?.clone())}
#[tauri::command] pub fn save_settings(settings:Settings,app:AppHandle)->Result<(),String>{
 settings.validate()?;for p in &settings.presets{shortcut(p).parse::<Shortcut>().map_err(|e|e.to_string())?;}
 let state=app.state::<AppState>();if let Some(e)=&state.load_error{return Err(e.clone())}
 {let mut guard=state.settings.lock().map_err(|e|e.to_string())?;settings.save(&state.path)?;*guard=settings;}
 sync_shortcuts(&app,platform::active());Ok(())
}
#[tauri::command] pub async fn get_status(app:AppHandle)->Result<platform::Status,String>{
 let mut status=tauri::async_runtime::spawn_blocking(platform::inspect).await.map_err(|e|e.to_string())?;
 let state=app.state::<AppState>();if let Some(error)=state.bindings.lock().map_err(|e|e.to_string())?.clone(){status.available=false;status.message=error;}
 Ok(status)
}
#[tauri::command] pub fn get_history(state:tauri::State<AppState>)->Result<Vec<Outcome>,String>{Ok(state.history.lock().map_err(|e|e.to_string())?.clone())}
pub fn publish(app:&AppHandle,outcome:Outcome){
 let state=app.state::<AppState>();if let Ok(mut history)=state.history.lock(){history.insert(0,outcome.clone());history.truncate(20);}
 let _=app.emit("switch-finished",outcome);
}
#[tauri::command] pub async fn apply_preset(app:AppHandle,id:String)->Result<Outcome,String>{
 let preset={let state=app.state::<AppState>();let settings=state.settings.lock().map_err(|e|e.to_string())?;settings.presets.iter().find(|p|p.id==id).cloned().ok_or("Unknown preset")?};
 {let state=app.state::<AppState>();if state.busy.compare_exchange(false,true,Ordering::SeqCst,Ordering::SeqCst).is_err(){return Err("이미 전환 중입니다. / A switch is already running.".into())}}
 let worker=app.clone();let result=tauri::async_runtime::spawn_blocking(move||{
   struct Guard<'a>(&'a AtomicBool);impl Drop for Guard<'_>{fn drop(&mut self){self.0.store(false,Ordering::SeqCst)}}
   let state=worker.state::<AppState>();let _guard=Guard(&state.busy);
   match platform::wait_shortcut_release(){Ok(())=>platform::switch(&preset),Err(e)=>Outcome{success:false,message:e,actual:None,restored:false}}
 }).await.map_err(|e|e.to_string());
 match result{Ok(outcome)=>{publish(&app,outcome.clone());Ok(outcome)},Err(e)=>{app.state::<AppState>().busy.store(false,Ordering::SeqCst);Err(e)}}
}
pub fn shortcut_plugin()->tauri::plugin::TauriPlugin<tauri::Wry>{
 tauri_plugin_global_shortcut::Builder::new().with_handler(|app,key,event|{
 if event.state()!=ShortcutState::Pressed || !platform::active(){return}
 let preset={let state=app.state::<AppState>();let Ok(settings)=state.settings.lock()else{return};settings.presets.iter().find(|p|shortcut(p).parse::<Shortcut>().ok().as_ref()==Some(key)).map(|p|p.id.clone())};
 if let Some(id)=preset{let app=app.clone();tauri::async_runtime::spawn(async move{let _=apply_preset(app,id).await;});}
 }).build()
}

