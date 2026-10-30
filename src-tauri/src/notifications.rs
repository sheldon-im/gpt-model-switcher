use std::sync::atomic::{AtomicU64, Ordering};
use tauri::{AppHandle, Manager, WebviewUrl, WebviewWindowBuilder};
static SEQUENCE: AtomicU64 = AtomicU64::new(0);
static ANNOUNCEMENT_ERROR: std::sync::Mutex<Option<String>> = std::sync::Mutex::new(None);
pub fn last_error() -> Option<String> {
    ANNOUNCEMENT_ERROR.lock().ok().and_then(|e| e.clone())
}
pub fn outcome_message(outcome: &switcher_core::switching::Outcome, language: &str) -> String {
    let ko = language == "ko";
    let prefix = if !outcome.success {
        if ko {
            "전환 실패"
        } else {
            "Switch failed"
        }
    } else if outcome.already_applied {
        if ko {
            "이미 적용되어 있습니다"
        } else {
            "Already applied"
        }
    } else if ko {
        "전환 완료"
    } else {
        "Switched"
    };
    let pieces: Vec<_> = outcome.message.split(" / ").collect();
    let message = if ko {
        pieces[0]
    } else {
        pieces.last().copied().unwrap_or(&outcome.message)
    };
    let actual = if !outcome.success {
        outcome
            .actual
            .as_ref()
            .map(|s| format!(". {} {}", s.model, s.effort))
            .unwrap_or_else(|| {
                if ko {
                    ". 현재 선택값은 확인하지 못했습니다.".into()
                } else {
                    ". Current selection could not be verified.".into()
                }
            })
    } else {
        String::new()
    };
    format!("{prefix}. {message}{actual}")
}
#[derive(Debug, serde::Serialize)]
pub struct Delivery {
    pub backend: &'static str,
    pub warning: Option<String>,
}
#[cfg(any(windows, test))]
fn route_windows(
    nvda: impl FnOnce() -> Result<Option<Option<String>>, String>,
    uia: impl FnOnce() -> Result<(), String>,
) -> Result<Delivery, String> {
    match nvda()? {
        Some(warning) => Ok(Delivery {
            backend: "nvda-controller",
            warning,
        }),
        None => {
            uia()?;
            Ok(Delivery {
                backend: "windows-uia",
                warning: None,
            })
        }
    }
}
pub fn test_message(language: &str) -> &'static str {
    if language == "en" {
        "Model switcher voice feedback test."
    } else {
        "모델 전환 음성 피드백 시험입니다."
    }
}
pub fn setup(app: &AppHandle, enabled: bool) -> tauri::Result<()> {
    if !enabled || app.get_webview_window("notification").is_some() {
        return Ok(());
    }
    WebviewWindowBuilder::new(
        app,
        "notification",
        WebviewUrl::App("index.html#notification".into()),
    )
    .title("GPT Model Switcher result")
    .inner_size(400., 180.)
    .position(24., 24.)
    .decorations(false)
    .resizable(false)
    .always_on_top(true)
    .skip_taskbar(true)
    .focusable(false)
    .focused(false)
    .visible(false)
    .build()?;
    Ok(())
}
pub fn configure(app: &AppHandle, enabled: bool) {
    let handle = app.clone();
    let _ = app.run_on_main_thread(move || {
        if enabled {
            let _ = setup(&handle, true);
        } else if let Some(window) = handle.get_webview_window("notification") {
            let _ = window.hide();
        }
    });
}
pub fn announce(app: &AppHandle, message: String) {
    let app_for_call = app.clone();
    if let Err(error) = app.run_on_main_thread(move || {
        let _ = deliver(&app_for_call, &message);
    }) {
        if let Ok(mut value) = ANNOUNCEMENT_ERROR.lock() {
            *value = Some(error.to_string());
        }
    }
}
pub fn deliver(app: &AppHandle, message: &str) -> Result<Delivery, String> {
    #[cfg(windows)]
    let result = (|| {
        let resources = app.path().resource_dir().map_err(|e| e.to_string())?;
        route_windows(
            || crate::nvda::announce(&resources, message),
            || unsafe {
                use windows::{
                    core::BSTR,
                    Win32::{
                        System::Com::{CoInitializeEx, CoUninitialize, COINIT_APARTMENTTHREADED},
                        UI::Accessibility::*,
                    },
                };
                let initialized = CoInitializeEx(None, COINIT_APARTMENTTHREADED).is_ok();
                let result = (|| {
                    let window = app
                        .get_webview_window("main")
                        .ok_or("Missing announcement host")?;
                    let hwnd = window.hwnd().map_err(|e| e.to_string())?;
                    let provider = UiaHostProviderFromHwnd(hwnd).map_err(|e| e.to_string())?;
                    UiaRaiseNotificationEvent(
                        &provider,
                        NotificationKind_ActionCompleted,
                        NotificationProcessing_CurrentThenMostRecent,
                        &BSTR::from(message),
                        &BSTR::from("gpt-model-switcher"),
                    )
                    .map_err(|e| e.to_string())
                })();
                if initialized {
                    CoUninitialize()
                }
                result
            },
        )
    })();
    #[cfg(target_os = "macos")]
    let result = crate::platform::announce(message).map(|_| Delivery {
        backend: "macos-accessibility",
        warning: None,
    });
    if let Ok(mut error) = ANNOUNCEMENT_ERROR.lock() {
        *error = match &result {
            Ok(delivery) => delivery.warning.clone(),
            Err(error) => Some(error.clone()),
        };
    }
    result
}
pub fn show(app: &AppHandle) {
    let sequence = SEQUENCE.fetch_add(1, Ordering::SeqCst) + 1;
    let app = app.clone();
    let main = app.clone();
    let _=app.run_on_main_thread(move||{
    if !main.state::<crate::runtime::AppState>().settings.lock().is_ok_and(|s|s.visual_notifications){return}
    if setup(&main,true).is_err(){return}
    if let Some(w)=main.get_webview_window("notification"){
   #[cfg(windows)]unsafe{if let Ok(hwnd)=w.hwnd(){use windows::Win32::UI::WindowsAndMessaging::{ShowWindow,SW_SHOWNOACTIVATE};let _=ShowWindow(hwnd,SW_SHOWNOACTIVATE);}}
   #[cfg(target_os="macos")]unsafe{if let Ok(pointer)=w.ns_window(){let window=pointer as *mut objc2::runtime::AnyObject;let _:()=objc2::msg_send![window,orderFront:std::ptr::null_mut::<objc2::runtime::AnyObject>()];}}
 }});
    std::thread::spawn(move || {
        std::thread::sleep(std::time::Duration::from_secs(6));
        if SEQUENCE.load(Ordering::SeqCst) == sequence {
            if let Some(w) = app.get_webview_window("notification") {
                let _ = w.hide();
            }
        }
    });
}
#[cfg(test)]
mod tests {
    use super::*;
    fn selected(already_applied: bool) -> switcher_core::switching::Outcome {
        switcher_core::switching::Outcome {
            success: true,
            already_applied,
            message: "gpt-6.1-sol · high".into(),
            actual: Some(switcher_core::Selection {
                model: "gpt-6.1-sol".into(),
                effort: "high".into(),
            }),
            restored: false,
        }
    }
    #[test]
    fn same_preset_is_announced_as_already_applied_in_both_languages() {
        let result = selected(true);
        assert_eq!(
            outcome_message(&result, "ko"),
            "이미 적용되어 있습니다. gpt-6.1-sol · high"
        );
        assert_eq!(
            outcome_message(&result, "en"),
            "Already applied. gpt-6.1-sol · high"
        );
    }
    #[test]
    fn a_changed_selection_still_reports_a_completed_switch() {
        assert_eq!(
            outcome_message(&selected(false), "ko"),
            "전환 완료. gpt-6.1-sol · high"
        );
    }
    #[test]
    fn nvda_acceptance_never_sends_a_second_uia_announcement() {
        let mut fallback = 0;
        let result = route_windows(
            || Ok(Some(None)),
            || {
                fallback += 1;
                Ok(())
            },
        )
        .unwrap();
        assert_eq!(result.backend, "nvda-controller");
        assert_eq!(fallback, 0);
    }
    #[test]
    fn braille_warning_never_causes_duplicate_speech() {
        let mut fallback = 0;
        let result = route_windows(
            || Ok(Some(Some("braille warning".into()))),
            || {
                fallback += 1;
                Ok(())
            },
        )
        .unwrap();
        assert!(result.warning.is_some());
        assert_eq!(fallback, 0);
    }
    #[test]
    fn missing_nvda_uses_generic_accessibility_once() {
        let mut fallback = 0;
        assert_eq!(
            route_windows(
                || Ok(None),
                || {
                    fallback += 1;
                    Ok(())
                }
            )
            .unwrap()
            .backend,
            "windows-uia"
        );
        assert_eq!(fallback, 1);
    }
    #[test]
    fn rejected_nvda_request_surfaces_error_instead_of_double_speech() {
        assert!(route_windows(
            || Err("access denied".into()),
            || panic!("Must not blindly retry a different speech backend")
        )
        .is_err());
    }
}
