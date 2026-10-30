#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
use switcher_core::Settings;
use tauri::{Manager,menu::{Menu,MenuItem},tray::TrayIconBuilder};
mod platform;
mod runtime;
mod desktop_ipc;
use runtime::*;
fn main(){
    if std::env::args().any(|a|a=="--inspect"){println!("{}",serde_json::to_string(&platform::inspect()).unwrap());return;}
    #[cfg(windows)]
    if std::env::args().any(|a|a=="--inspect-menu"){let mut s=platform::Session::open().expect("Activate ChatGPT");let result=s.menu_names();let _=switcher_core::switching::Target::finish(&mut s);println!("{}",serde_json::to_string(&result).unwrap());return;}
    #[cfg(windows)]
    if std::env::args().any(|a|a=="--inspect-current-efforts"){let mut s=platform::Session::open().expect("Activate ChatGPT");let result=s.current_efforts_dump();let _=switcher_core::switching::Target::finish(&mut s);println!("{}",serde_json::to_string(&result).unwrap());return;}
    #[cfg(windows)]
    if let Some(sub)=std::env::args().find_map(|a|a.strip_prefix("--inspect-submenu=").map(str::to_owned)){let mut s=platform::Session::open().expect("Activate ChatGPT");let result=if sub=="model"{s.submenu_names(&["모델 선택","Choose model","Select model"])}else{s.submenu_dump()};let _=switcher_core::switching::Target::finish(&mut s);println!("{}",serde_json::to_string(&result).unwrap());return;}
    if let Some(id)=std::env::args().find_map(|a|a.strip_prefix("--apply=").map(str::to_owned)){let settings=Settings::default();if let Some(p)=settings.presets.iter().find(|p|p.id==id){println!("{}",serde_json::to_string(&platform::switch(p)).unwrap());}return;}
    tauri::Builder::default().plugin(tauri_plugin_single_instance::init(|app,_,_|{if let Some(w)=app.get_webview_window("main"){let _=w.show();let _=w.set_focus();}})).plugin(shortcut_plugin()).setup(|app|{
        let path=app.path().app_config_dir()?.join("settings.json");
        app.manage(AppState::new(path));
        if std::env::args().any(|a|a=="--fixture"){let w=app.get_webview_window("main").ok_or("Missing main window")?;w.set_title("GPT Model Switcher Fixture")?;let base=if cfg!(debug_assertions){"http://localhost:1420/fixture.html"}else if cfg!(windows){"http://tauri.localhost/fixture.html"}else{"tauri://localhost/fixture.html"};w.navigate(tauri::Url::parse(base)?)?;}
        let observer=app.handle().clone();platform::watch(move|active|{let handle=observer.clone();let _=observer.run_on_main_thread(move||sync_shortcuts(&handle,active));});
        let open=MenuItem::with_id(app,"open","설정 / Settings",true,None::<&str>)?;
        let quit=MenuItem::with_id(app,"quit","종료 / Quit",true,None::<&str>)?;
        let menu=Menu::with_items(app,&[&open,&quit])?;
        let mut tray=TrayIconBuilder::new().tooltip("GPT Model Switcher").menu(&menu).on_menu_event(|app,event|match event.id.as_ref(){"open"=>{if let Some(w)=app.get_webview_window("main"){let _=w.show();let _=w.set_focus();}},"quit"=>app.exit(0),_=>{}});
        if let Some(icon)=app.default_window_icon(){tray=tray.icon(icon.clone());}tray.build(app)?;Ok(())
    }).on_window_event(|window,event|{if let tauri::WindowEvent::CloseRequested{api,..}=event{api.prevent_close();let _=window.hide();}})
    .invoke_handler(tauri::generate_handler![get_settings,save_settings,get_status,get_history,apply_preset]).run(tauri::generate_context!()).expect("GPT Model Switcher failed to start");
}
