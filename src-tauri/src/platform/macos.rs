use super::Status;
use serde_json::Value;
use std::ffi::{CStr, CString};
unsafe extern "C" {
    fn switcher_macos_request(input: *const libc::c_char) -> *mut libc::c_char;
    fn switcher_macos_free(pointer: *mut libc::c_char);
    fn switcher_macos_watch(callback: extern "C" fn(u8));
}
fn request(args: &[&str]) -> Result<Value, String> {
    let input = CString::new(serde_json::to_string(args).map_err(|e| e.to_string())?)
        .map_err(|e| e.to_string())?;
    let text = unsafe {
        let pointer = switcher_macos_request(input.as_ptr());
        if pointer.is_null() {
            return Err("macOS bridge returned no response".into());
        }
        let text = CStr::from_ptr(pointer).to_string_lossy().into_owned();
        switcher_macos_free(pointer);
        text
    };
    let value: Value = serde_json::from_str(&text).map_err(|e| e.to_string())?;
    if let Some(error) = value.get("error").and_then(Value::as_str) {
        Err(error.into())
    } else {
        Ok(value)
    }
}
pub fn announce(message: &str) -> Result<(), String> {
    request(&["announce", message]).map(|_| ())
}
pub fn accessibility(prompt: bool) -> Result<bool, String> {
    let value = request(&[if prompt { "request-access" } else { "access" }])?;
    value["granted"]
        .as_bool()
        .ok_or("Missing accessibility permission status".into())
}
pub fn open_accessibility_settings() -> Result<(), String> {
    request(&["open-access-settings"]).map(|_| ())
}
pub fn active() -> bool {
    request(&["foreground"])
        .ok()
        .and_then(|v| v["active"].as_bool())
        .unwrap_or(false)
}
type ForegroundCallback = Box<dyn Fn(bool) + Send + Sync>;
static FOREGROUND_CALLBACK: std::sync::OnceLock<ForegroundCallback> = std::sync::OnceLock::new();
extern "C" fn foreground_changed(active: u8) {
    if let Some(callback) = FOREGROUND_CALLBACK.get() {
        callback(active != 0)
    }
}
pub fn watch(callback: impl Fn(bool) + Send + Sync + 'static) {
    if FOREGROUND_CALLBACK.set(Box::new(callback)).is_ok() {
        // Called by Tauri setup on its main thread; no child process owns AX access.
        unsafe { switcher_macos_watch(foreground_changed) };
    }
}
pub fn inspect() -> Status {
    request(&["inspect"])
        .and_then(|v| serde_json::from_value(v).map_err(|e| e.to_string()))
        .unwrap_or_else(Status::unavailable)
}
use switcher_core::{switching::Outcome, Preset, Selection};
struct MacWindow {
    context: String,
    id: String,
    title: String,
    pid: u32,
}
impl MacWindow {
    fn new() -> Result<Self, String> {
        let v = request(&["inspect"])?;
        let title = v["context"]["document"]
            .as_str()
            .ok_or("Missing conversation title")?
            .to_string();
        let id = crate::desktop_ipc::resolve(&title)?;
        let pid = v["context"]["pid"]
            .as_u64()
            .ok_or("Missing target process")? as u32;
        let mut context = v["context"].clone();
        context["deadline"] = serde_json::json!(
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map_err(|e| e.to_string())?
                .as_secs_f64()
                + 8.0
        );
        Ok(Self {
            context: context.to_string(),
            id,
            title,
            pid,
        })
    }
}
impl crate::quiet_switch::Context for MacWindow {
    fn guard(&mut self) -> Result<Selection, String> {
        request(&["check", &self.context])?;
        if crate::desktop_ipc::resolve(&self.title)? != self.id {
            return Err("Conversation identity changed".into());
        }
        let v = request(&["inspect"])?;
        serde_json::from_value(v["selection"].clone()).map_err(|e| e.to_string())
    }
}
pub fn diagnose() -> Result<Value, String> {
    let w = MacWindow::new()?;
    let mut d = crate::desktop_ipc::Desktop::connect(w.pid)?;
    d.read(&w.id)
}
pub fn switch(p: &Preset) -> Outcome {
    use crate::quiet_switch::Context;
    let result = (|| {
        let mut w = MacWindow::new()?;
        w.guard()?;
        let mut d = crate::desktop_ipc::Desktop::connect(w.pid)?;
        let id = w.id.clone();
        Ok(crate::quiet_switch::apply(&mut w, &mut d, &id, p))
    })();
    result.unwrap_or_else(|message| Outcome {
        success: false,
        already_applied: false,
        message,
        actual: None,
        restored: false,
    })
}

#[cfg(test)]
mod bridge_tests {
    use super::*;
    #[test]
    fn linked_bridge_roundtrip_and_malformed_context_are_safe() {
        assert!(request(&["access"]).unwrap()["granted"].is_boolean());
        assert!(request(&["check", "{}"])
            .unwrap_err()
            .contains("Invalid or expired target context"));
        assert!(request(&["check"])
            .unwrap_err()
            .contains("Missing target context"));
        assert!(request(&["unknown"])
            .unwrap_err()
            .contains("Unknown bridge command"));
    }
}
