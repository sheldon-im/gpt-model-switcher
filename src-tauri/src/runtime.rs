use crate::platform;
use std::{path::PathBuf,sync::{Mutex,atomic::{AtomicBool,Ordering}}};
use switcher_core::{Settings,Preset,switching::Outcome};
use tauri::{AppHandle,Manager,Emitter};
use tauri_plugin_global_shortcut::{GlobalShortcutExt,Shortcut,ShortcutState};
pub struct AppState{pub settings:Mutex<Settings>,pub path:PathBuf,pub load_error:Option<String>,pub busy:AtomicBool,pub target_active:AtomicBool,pub bindings:Mutex<Option<String>>,pub history:Mutex<Vec<Outcome>>,gate:Mutex<()>}
impl AppState{pub fn new(path:PathBuf)->Self{let loaded=Settings::load(&path);Self{settings:Mutex::new(loaded.clone().unwrap_or_default()),path,load_error:loaded.err(),busy:AtomicBool::new(false),target_active:AtomicBool::new(false),bindings:Mutex::new(None),history:Mutex::new(Vec::new()),gate:Mutex::new(())}}}
fn shortcut(p:&Preset)->&str{if cfg!(target_os="macos"){&p.macos_shortcut}else{&p.windows_shortcut}}
pub fn sync_shortcuts(app:&AppHandle,active:bool){
 let state=app.state::<AppState>();state.target_active.store(active,Ordering::SeqCst);let Ok(_gate)=state.gate.lock()else{return};
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
 let visual=settings.visual_notifications;
 {let mut guard=state.settings.lock().map_err(|e|e.to_string())?;settings.save(&state.path)?;*guard=settings;}
 crate::notifications::configure(&app,visual);
 sync_shortcuts(&app,platform::active());Ok(())
}
#[tauri::command] pub async fn get_status(app:AppHandle)->Result<platform::Status,String>{
 let mut status=tauri::async_runtime::spawn_blocking(platform::inspect).await.map_err(|e|e.to_string())?;
 let state=app.state::<AppState>();if let Some(error)=state.bindings.lock().map_err(|e|e.to_string())?.clone(){status.available=false;status.message=error;}
 if let Some(error)=crate::notifications::last_error(){status.message.push_str(&format!(" · 스크린 리더 안내 전달 실패 / Announcement delivery failed: {error}"));}
 Ok(status)
}
#[tauri::command] pub fn get_history(state:tauri::State<AppState>)->Result<Vec<Outcome>,String>{Ok(state.history.lock().map_err(|e|e.to_string())?.clone())}
#[derive(serde::Serialize)]
pub struct AccessibilityStatus { required: bool, granted: bool }
pub fn write_runtime_diagnostic(app:&AppHandle){
 let handle=app.clone();tauri::async_runtime::spawn(async move{
  let result:Result<(),String>=(async{
   let access=accessibility_status(false).await?;
   let state=handle.state::<AppState>();
   let (visual,presets)={let settings=state.settings.lock().map_err(|e|e.to_string())?;
    (settings.visual_notifications,settings.presets.iter().map(|p|serde_json::json!({"id":p.id,"model":p.model,"effort":p.effort,"macosShortcut":p.macos_shortcut})).collect::<Vec<_>>())};
   let (count,latest)={let history=state.history.lock().map_err(|e|e.to_string())?;(history.len(),history.first().map(|o|o.success))};
   let windows=handle.webview_windows().into_iter().filter_map(|(label,w)|w.is_visible().ok().filter(|visible|*visible).map(|_|label)).collect::<Vec<_>>();
   let report=serde_json::json!({"version":handle.package_info().version.to_string(),"accessibilityGranted":access.granted,"visualNotifications":visual,"presets":presets,"visibleWindows":windows,"resultCount":count,"latestSuccess":latest,"settingsPath":state.path,"observedAtMs":std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_err(|e|e.to_string())?.as_millis()});
   let directory=handle.path().app_cache_dir().map_err(|e|e.to_string())?;
   std::fs::create_dir_all(&directory).map_err(|e|e.to_string())?;
   let pending=directory.join("runtime-diagnostic.pending.json");
   std::fs::write(&pending,serde_json::to_vec_pretty(&report).map_err(|e|e.to_string())?).map_err(|e|e.to_string())?;
   std::fs::rename(pending,directory.join("runtime-diagnostic.json")).map_err(|e|e.to_string())?;
   Ok(())
  }).await;
  if let Err(error)=result {eprintln!("Runtime diagnostic failed: {error}");}
 });
}
#[tauri::command] pub async fn accessibility_status(prompt:bool)->Result<AccessibilityStatus,String>{
 let granted=tauri::async_runtime::spawn_blocking(move||platform::accessibility(prompt)).await.map_err(|e|e.to_string())??;
 Ok(AccessibilityStatus{required:cfg!(target_os="macos"),granted})
}
#[tauri::command] pub async fn open_accessibility_settings()->Result<(),String>{
 tauri::async_runtime::spawn_blocking(platform::open_accessibility_settings).await.map_err(|e|e.to_string())?
}
pub fn publish(app:&AppHandle,outcome:Outcome){
 let state=app.state::<AppState>();if let Ok(mut history)=state.history.lock(){history.insert(0,outcome.clone());history.truncate(20);}
 if let Ok(settings)=state.settings.lock(){if settings.screen_reader_notifications{let ko=settings.language=="ko";let prefix=match(outcome.success,ko){(true,true)=>"전환 완료",(false,true)=>"전환 실패",(true,false)=>"Switched",(false,false)=>"Switch failed"};let pieces:Vec<_>=outcome.message.split(" / ").collect();let message=if ko{pieces[0]}else{pieces.last().copied().unwrap_or(&outcome.message)};let actual=if !outcome.success{outcome.actual.as_ref().map(|s|format!(". {} {}",s.model,s.effort)).unwrap_or_else(||if ko{". 현재 선택값은 확인하지 못했습니다.".into()}else{". Current selection could not be verified.".into()})}else{String::new()};crate::notifications::announce(app,format!("{prefix}. {message}{actual}"));}}
 let _=app.emit("switch-finished",outcome);
 if state.settings.lock().is_ok_and(|s|s.visual_notifications){crate::notifications::show(app);}
}
#[tauri::command] pub async fn apply_preset(app:AppHandle,id:String)->Result<Outcome,String>{
 let preset={let state=app.state::<AppState>();let settings=state.settings.lock().map_err(|e|e.to_string())?;settings.presets.iter().find(|p|p.id==id).cloned().ok_or("Unknown preset")?};
 {let state=app.state::<AppState>();if state.busy.compare_exchange(false,true,Ordering::SeqCst,Ordering::SeqCst).is_err(){return Err("이미 전환 중입니다. / A switch is already running.".into())}}
 let worker=app.clone();let result=tauri::async_runtime::spawn_blocking(move||{
   struct Guard<'a>(&'a AtomicBool);impl Drop for Guard<'_>{fn drop(&mut self){self.0.store(false,Ordering::SeqCst)}}
   let state=worker.state::<AppState>();let _guard=Guard(&state.busy);
   // macOS uses only AX reads and conditional IPC; no key events are sent.
   // Holding the shortcut modifiers must not veto this focus-free operation.
   #[cfg(target_os="macos")] { platform::switch(&preset) }
   #[cfg(not(target_os="macos"))]
   { match platform::wait_shortcut_release(){Ok(())=>platform::switch(&preset),Err(e)=>Outcome{success:false,message:e,actual:None,restored:false}} }
 }).await.map_err(|e|e.to_string());
 match result{Ok(outcome)=>{publish(&app,outcome.clone());Ok(outcome)},Err(e)=>{app.state::<AppState>().busy.store(false,Ordering::SeqCst);Err(e)}}
}
pub fn shortcut_plugin()->tauri::plugin::TauriPlugin<tauri::Wry>{
 tauri_plugin_global_shortcut::Builder::new().with_handler(|app,key,event|{
 if event.state()!=ShortcutState::Pressed || !app.state::<AppState>().target_active.load(Ordering::SeqCst){return}
 let preset={let state=app.state::<AppState>();let Ok(settings)=state.settings.lock()else{return};settings.presets.iter().find(|p|shortcut(p).parse::<Shortcut>().ok().as_ref()==Some(key)).map(|p|p.id.clone())};
 if let Some(id)=preset{let app=app.clone();tauri::async_runtime::spawn(async move{let _=apply_preset(app,id).await;});}
 }).build()
}

