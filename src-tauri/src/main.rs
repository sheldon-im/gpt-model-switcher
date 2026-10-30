#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
use std::{path::PathBuf,sync::Mutex};
use switcher_core::Settings;
use tauri::{Manager,menu::{Menu,MenuItem},tray::TrayIconBuilder};
mod platform;
struct AppState{settings:Mutex<Settings>,path:PathBuf}
#[tauri::command] fn get_settings(state:tauri::State<AppState>)->Result<Settings,String>{Ok(state.settings.lock().map_err(|e|e.to_string())?.clone())}
#[tauri::command] fn save_settings(settings:Settings,state:tauri::State<AppState>)->Result<(),String>{let mut guard=state.settings.lock().map_err(|e|e.to_string())?;settings.save(&state.path)?;*guard=settings;Ok(())}
#[tauri::command] async fn get_status()->Result<platform::Status,String>{tauri::async_runtime::spawn_blocking(platform::inspect).await.map_err(|e|e.to_string())}
fn main(){
    if std::env::args().any(|a|a=="--inspect"){println!("{}",serde_json::to_string(&platform::inspect()).unwrap());return;}
    tauri::Builder::default().setup(|app|{
        let path=app.path().app_config_dir()?.join("settings.json");
        let settings=Settings::load(&path).map_err(std::io::Error::other)?;
        app.manage(AppState{settings:Mutex::new(settings),path});
        let open=MenuItem::with_id(app,"open","설정 / Settings",true,None::<&str>)?;
        let quit=MenuItem::with_id(app,"quit","종료 / Quit",true,None::<&str>)?;
        let menu=Menu::with_items(app,&[&open,&quit])?;
        let mut tray=TrayIconBuilder::new().tooltip("GPT Model Switcher").menu(&menu).on_menu_event(|app,event|match event.id.as_ref(){"open"=>{if let Some(w)=app.get_webview_window("main"){let _=w.show();let _=w.set_focus();}},"quit"=>app.exit(0),_=>{}});
        if let Some(icon)=app.default_window_icon(){tray=tray.icon(icon.clone());}tray.build(app)?;Ok(())
    }).on_window_event(|window,event|{if let tauri::WindowEvent::CloseRequested{api,..}=event{api.prevent_close();let _=window.hide();}})
    .invoke_handler(tauri::generate_handler![get_settings,save_settings,get_status]).run(tauri::generate_context!()).expect("GPT Model Switcher failed to start");
}
