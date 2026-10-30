#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
use tauri::{
    menu::{Menu, MenuItem},
    tray::TrayIconBuilder,
    Manager,
};
mod desktop_ipc;
mod notifications;
mod platform;
mod quiet_switch;
mod runtime;
mod launch;
use runtime::*;
fn handle_launch(app: &tauri::AppHandle, args: &[String], initial: bool) {
    match launch::action(args, initial) {
        Ok(launch::Action::Settings) => {
            let handle = app.clone();
            let _ = app.run_on_main_thread(move || {
                if let Some(w) = handle.get_webview_window("main") {
                    let _ = w.show();
                    let _ = w.set_focus();
                }
            });
        }
        Ok(launch::Action::Apply(id)) => {
            let handle = app.clone();
            tauri::async_runtime::spawn(async move {
                if let Err(message) = apply_preset(handle.clone(), id).await {
                    publish(&handle, switcher_core::switching::Outcome {success:false,message,actual:None,restored:false});
                }
            });
        }
        Ok(launch::Action::Background) => {}
        Err(message) => publish(app, switcher_core::switching::Outcome {success:false,message,actual:None,restored:false}),
    }
}
fn main() {
    if std::env::args().any(|a| a == "--inspect") {
        println!("{}", serde_json::to_string(&platform::inspect()).unwrap());
        return;
    }
    if std::env::args().any(|a| a == "--diagnose") {
        println!("{}", serde_json::to_string(&platform::diagnose()).unwrap());
        return;
    }
    if std::env::args().any(|a| a == "--accessibility-status") {
        println!("{}", serde_json::to_string(&platform::accessibility(false)).unwrap());
        return;
    }
    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, args, _| {
            handle_launch(app, &args, false);
        }))
        .plugin(shortcut_plugin())
        .setup(|app| {
            let path = app.path().app_config_dir()?.join("settings.json");
            app.manage(AppState::new(path));
            notifications::setup(app.handle())?;
            #[cfg(target_os="macos")]
            { let _ = platform::accessibility(true); }
            handle_launch(app.handle(), &std::env::args().collect::<Vec<_>>(), true);
            if std::env::args().any(|a| a == "--fixture") {
                let w = app
                    .get_webview_window("main")
                    .ok_or("Missing main window")?;
                w.set_title("GPT Model Switcher Fixture")?;
                let base = if cfg!(debug_assertions) {
                    "http://localhost:1420/fixture.html"
                } else if cfg!(windows) {
                    "http://tauri.localhost/fixture.html"
                } else {
                    "tauri://localhost/fixture.html"
                };
                w.navigate(tauri::Url::parse(base)?)?;
            }
            let observer = app.handle().clone();
            platform::watch(move |active| {
                let handle = observer.clone();
                let _ = observer.run_on_main_thread(move || sync_shortcuts(&handle, active));
            });
            let open = MenuItem::with_id(app, "open", "설정 / Settings", true, None::<&str>)?;
            let quit = MenuItem::with_id(app, "quit", "종료 / Quit", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&open, &quit])?;
            let mut tray = TrayIconBuilder::new()
                .tooltip("GPT Model Switcher")
                .menu(&menu)
                .on_menu_event(|app, event| match event.id.as_ref() {
                    "open" => {
                        if let Some(w) = app.get_webview_window("main") {
                            let _ = w.show();
                            let _ = w.set_focus();
                        }
                    }
                    "quit" => app.exit(0),
                    _ => {}
                });
            if let Some(icon) = app.default_window_icon() {
                tray = tray.icon(icon.clone());
            }
            tray.build(app)?;
            Ok(())
        })
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                api.prevent_close();
                let _ = window.hide();
            }
        })
        .invoke_handler(tauri::generate_handler![
            get_settings,
            save_settings,
            get_status,
            get_history,
            accessibility_status,
            open_accessibility_settings,
            apply_preset
        ])
        .run(tauri::generate_context!())
        .expect("GPT Model Switcher failed to start");
}
