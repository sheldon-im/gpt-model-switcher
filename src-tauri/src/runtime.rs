use crate::platform;
use std::{
    path::PathBuf,
    sync::{
        atomic::{AtomicBool, Ordering},
        Mutex,
    },
};
use switcher_core::{switching::Outcome, Preset, Settings};
use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut, ShortcutState};
pub struct AppState {
    pub settings: Mutex<Settings>,
    pub path: PathBuf,
    pub load_error: Option<String>,
    pub busy: AtomicBool,
    pub target_active: AtomicBool,
    pub bindings: Mutex<Option<String>>,
    pub history: Mutex<Vec<Outcome>>,
    gate: Mutex<()>,
}
impl AppState {
    pub fn new(path: PathBuf) -> Self {
        let loaded = Settings::load(&path);
        Self {
            settings: Mutex::new(loaded.clone().unwrap_or_default()),
            path,
            load_error: loaded.err(),
            busy: AtomicBool::new(false),
            target_active: AtomicBool::new(false),
            bindings: Mutex::new(None),
            history: Mutex::new(Vec::new()),
            gate: Mutex::new(()),
        }
    }
}
fn shortcut(p: &Preset) -> &str {
    if cfg!(target_os = "macos") {
        &p.macos_shortcut
    } else {
        &p.windows_shortcut
    }
}
pub fn sync_shortcuts(app: &AppHandle, active: bool) {
    let state = app.state::<AppState>();
    state.target_active.store(active, Ordering::SeqCst);
    let Ok(_gate) = state.gate.lock() else { return };
    let result = (|| {
        app.global_shortcut()
            .unregister_all()
            .map_err(|e| e.to_string())?;
        if state.load_error.is_some() {
            return Err("Settings file needs repair / 설정 파일을 확인하십시오.".into());
        }
        if !active {
            return Ok(());
        }
        let settings = state.settings.lock().map_err(|e| e.to_string())?.clone();
        for p in &settings.presets {
            app.global_shortcut().register(shortcut(p)).map_err(|e| {
                format!(
                    "단축키 등록 실패 / Shortcut conflict: {} ({e})",
                    shortcut(p)
                )
            })?;
        }
        Ok::<_, String>(())
    })();
    if result.is_err() {
        let _ = app.global_shortcut().unregister_all();
    }
    if let Ok(mut e) = state.bindings.lock() {
        *e = result.err();
    };
}
#[tauri::command]
pub fn get_settings(state: tauri::State<AppState>) -> Result<Settings, String> {
    if let Some(e) = &state.load_error {
        return Err(e.clone());
    }
    Ok(state.settings.lock().map_err(|e| e.to_string())?.clone())
}
#[tauri::command]
pub fn save_settings(settings: Settings, app: AppHandle) -> Result<(), String> {
    settings.validate()?;
    for p in &settings.presets {
        shortcut(p).parse::<Shortcut>().map_err(|e| e.to_string())?;
    }
    let state = app.state::<AppState>();
    if let Some(e) = &state.load_error {
        return Err(e.clone());
    }
    let visual = settings.visual_notifications;
    {
        let mut guard = state.settings.lock().map_err(|e| e.to_string())?;
        settings.save(&state.path)?;
        *guard = settings;
    }
    crate::notifications::configure(&app, visual);
    sync_shortcuts(&app, platform::active());
    Ok(())
}
#[tauri::command]
pub async fn get_status(app: AppHandle) -> Result<platform::Status, String> {
    let mut status = tauri::async_runtime::spawn_blocking(platform::inspect)
        .await
        .map_err(|e| e.to_string())?;
    let state = app.state::<AppState>();
    if let Some(error) = state.bindings.lock().map_err(|e| e.to_string())?.clone() {
        status.available = false;
        status.message = error;
    }
    if let Some(error) = crate::notifications::last_error() {
        status.message.push_str(&format!(
            " · 스크린 리더 안내 전달 실패 / Announcement delivery failed: {error}"
        ));
    }
    Ok(status)
}
#[tauri::command]
pub fn get_history(state: tauri::State<AppState>) -> Result<Vec<Outcome>, String> {
    Ok(state.history.lock().map_err(|e| e.to_string())?.clone())
}
#[derive(serde::Serialize)]
pub struct AccessibilityStatus {
    required: bool,
    granted: bool,
}
pub fn write_runtime_diagnostic(app: &AppHandle) {
    let handle = app.clone();
    tauri::async_runtime::spawn(async move {
        let result:Result<(),String>=(async{
   let access=accessibility_status(false).await?;
   let state=handle.state::<AppState>();
   let (visual,presets)={let settings=state.settings.lock().map_err(|e|e.to_string())?;
    (settings.visual_notifications,settings.presets.iter().map(|p|serde_json::json!({"id":p.id,"model":p.model,"effort":p.effort,"macosShortcut":p.macos_shortcut})).collect::<Vec<_>>())};
   let (count,latest)={let history=state.history.lock().map_err(|e|e.to_string())?;(history.len(),history.first().map(|o|o.success))};
   let windows=handle.webview_windows().into_iter().filter_map(|(label,w)|w.is_visible().ok().filter(|visible|*visible).map(|_|label)).collect::<Vec<_>>();
   let report=serde_json::json!({"version":handle.package_info().version.to_string(),"accessibilityGranted":access.granted,"visualNotifications":visual,"presets":presets,"visibleWindows":windows,"resultCount":count,"latestSuccess":latest,"settingsPath":state.path,"observedAtMs":std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_err(|e|e.to_string())?.as_millis()});
   let directory=handle.path().app_cache_dir().map_err(|e|e.to_string())?;
   write_diagnostic_report(&directory,&report)?;
   Ok(())
  }).await;
        if let Err(error) = result {
            eprintln!("Runtime diagnostic failed: {error}");
        }
    });
}
fn write_diagnostic_report(
    directory: &std::path::Path,
    report: &serde_json::Value,
) -> Result<(), String> {
    std::fs::create_dir_all(directory).map_err(|e| e.to_string())?;
    let pending = directory.join(format!(
        "runtime-diagnostic-{}.pending.json",
        uuid::Uuid::new_v4()
    ));
    let destination = directory.join("runtime-diagnostic.json");
    std::fs::write(
        &pending,
        serde_json::to_vec_pretty(report).map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())?;
    #[cfg(windows)]
    let result = unsafe {
        use windows::{
            core::HSTRING,
            Win32::Storage::FileSystem::{
                MoveFileExW, MOVEFILE_REPLACE_EXISTING, MOVEFILE_WRITE_THROUGH,
            },
        };
        MoveFileExW(
            &HSTRING::from(pending.as_os_str()),
            &HSTRING::from(destination.as_os_str()),
            MOVEFILE_REPLACE_EXISTING | MOVEFILE_WRITE_THROUGH,
        )
        .map_err(|e| e.to_string())
    };
    #[cfg(not(windows))]
    let result = std::fs::rename(&pending, destination).map_err(|e| e.to_string());
    if result.is_err() {
        let _ = std::fs::remove_file(&pending);
    }
    result
}
#[tauri::command]
pub async fn accessibility_status(prompt: bool) -> Result<AccessibilityStatus, String> {
    let granted = tauri::async_runtime::spawn_blocking(move || platform::accessibility(prompt))
        .await
        .map_err(|e| e.to_string())??;
    Ok(AccessibilityStatus {
        required: cfg!(target_os = "macos"),
        granted,
    })
}
#[tauri::command]
pub async fn open_accessibility_settings() -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(platform::open_accessibility_settings)
        .await
        .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn test_announcement(app: AppHandle) -> Result<crate::notifications::Delivery, String> {
    let message = {
        let state = app.state::<AppState>();
        let settings = state.settings.lock().map_err(|e| e.to_string())?;
        if !settings.screen_reader_notifications {
            return Err("스크린 리더 안내를 켜고 저장한 뒤 시험하십시오. / Enable and save screen reader announcements before testing.".into());
        }
        crate::notifications::test_message(&settings.language).to_string()
    };
    // Match normal outcome delivery: Cocoa announcements belong on the app's UI thread.
    let (sender, receiver) = std::sync::mpsc::sync_channel(1);
    let handle = app.clone();
    app.run_on_main_thread(move || {
        let _ = sender.send(crate::notifications::deliver(&handle, &message));
    })
    .map_err(|e| e.to_string())?;
    tauri::async_runtime::spawn_blocking(move || {
        receiver
            .recv_timeout(std::time::Duration::from_secs(5))
            .map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())??
}
pub fn publish(app: &AppHandle, outcome: Outcome) {
    let state = app.state::<AppState>();
    if let Ok(mut history) = state.history.lock() {
        history.insert(0, outcome.clone());
        history.truncate(20);
    }
    if let Ok(settings) = state.settings.lock() {
        if settings.screen_reader_notifications {
            crate::notifications::announce(
                app,
                crate::notifications::outcome_message(&outcome, &settings.language),
            );
        }
    }
    let _ = app.emit("switch-finished", outcome);
    if state.settings.lock().is_ok_and(|s| s.visual_notifications) {
        crate::notifications::show(app);
    }
}
#[tauri::command]
pub async fn apply_preset(app: AppHandle, id: String) -> Result<Outcome, String> {
    let preset = {
        let state = app.state::<AppState>();
        let settings = state.settings.lock().map_err(|e| e.to_string())?;
        settings
            .presets
            .iter()
            .find(|p| p.id == id)
            .cloned()
            .ok_or("Unknown preset")?
    };
    {
        let state = app.state::<AppState>();
        if state
            .busy
            .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
            .is_err()
        {
            return Err("이미 전환 중입니다. / A switch is already running.".into());
        }
    }
    let worker = app.clone();
    let result = tauri::async_runtime::spawn_blocking(move || {
        struct Guard<'a>(&'a AtomicBool);
        impl Drop for Guard<'_> {
            fn drop(&mut self) {
                self.0.store(false, Ordering::SeqCst)
            }
        }
        let state = worker.state::<AppState>();
        let _guard = Guard(&state.busy);
        #[cfg(target_os = "macos")]
        {
            platform::switch(&preset)
        }
        #[cfg(not(target_os = "macos"))]
        {
            match platform::wait_shortcut_release() {
                Ok(()) => platform::switch(&preset),
                Err(e) => Outcome {
                    success: false,
                    already_applied: false,
                    message: e,
                    actual: None,
                    restored: false,
                },
            }
        }
    })
    .await
    .map_err(|e| e.to_string());
    match result {
        Ok(outcome) => {
            publish(&app, outcome.clone());
            Ok(outcome)
        }
        Err(e) => {
            app.state::<AppState>().busy.store(false, Ordering::SeqCst);
            Err(e)
        }
    }
}
pub fn shortcut_plugin() -> tauri::plugin::TauriPlugin<tauri::Wry> {
    tauri_plugin_global_shortcut::Builder::new()
        .with_handler(|app, key, event| {
            #[cfg(target_os = "macos")]
            let active = app.state::<AppState>().target_active.load(Ordering::SeqCst);
            #[cfg(not(target_os = "macos"))]
            let active = platform::active();
            if event.state() != ShortcutState::Pressed || !active {
                return;
            }
            let preset = {
                let state = app.state::<AppState>();
                let Ok(settings) = state.settings.lock() else {
                    return;
                };
                settings
                    .presets
                    .iter()
                    .find(|p| shortcut(p).parse::<Shortcut>().ok().as_ref() == Some(key))
                    .map(|p| p.id.clone())
            };
            if let Some(id) = preset {
                let app = app.clone();
                tauri::async_runtime::spawn(async move {
                    let _ = apply_preset(app, id).await;
                });
            }
        })
        .build()
}
#[cfg(test)]
mod diagnostic_tests {
    use super::*;
    #[test]
    fn resident_diagnostic_can_be_replaced_without_touching_user_settings() {
        let directory = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../work")
            .join(format!("diagnostic-test-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&directory).unwrap();
        let settings = directory.join("settings.json");
        std::fs::write(&settings, b"original settings").unwrap();
        write_diagnostic_report(
            &directory,
            &serde_json::json!({"version":"first","visibleWindows":[]}),
        )
        .unwrap();
        write_diagnostic_report(&directory,&serde_json::json!({"version":"second","accessibilityGranted":true,"visibleWindows":[]})).unwrap();
        let report: serde_json::Value = serde_json::from_slice(
            &std::fs::read(directory.join("runtime-diagnostic.json")).unwrap(),
        )
        .unwrap();
        assert_eq!(report["version"], "second");
        assert_eq!(std::fs::read(&settings).unwrap(), b"original settings");
        assert_eq!(std::fs::read_dir(&directory).unwrap().count(), 2);
        std::fs::remove_file(settings).unwrap();
        std::fs::remove_file(directory.join("runtime-diagnostic.json")).unwrap();
        std::fs::remove_dir(directory).unwrap();
    }
}
