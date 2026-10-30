use super::Status;
use serde_json::Value;
use std::{path::PathBuf, process::Command};
fn helper() -> Result<PathBuf, String> {
    let packaged = std::env::current_exe()
        .map_err(|e| e.to_string())?
        .parent()
        .ok_or("No executable directory")?
        .join("macos-bridge");
    if packaged.exists() {
        return Ok(packaged);
    }
    let dev =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("native/macos-bridge-aarch64-apple-darwin");
    if dev.exists() {
        Ok(dev)
    } else {
        Err("macOS bridge not built. Run the macOS build instructions. / Mac 빌드 지침을 실행하십시오.".into())
    }
}
fn request(args: &[&str]) -> Result<Value, String> {
    let output = Command::new(helper()?)
        .args(args)
        .output()
        .map_err(|e| e.to_string())?;
    if !output.status.success() {
        return Err(String::from_utf8_lossy(&output.stderr).into());
    }
    let v: Value = serde_json::from_slice(&output.stdout).map_err(|e| e.to_string())?;
    if let Some(e) = v.get("error").and_then(Value::as_str) {
        Err(e.into())
    } else {
        Ok(v)
    }
}
pub fn announce(message:&str)->Result<(),String>{request(&["announce",message]).map(|_|())}
pub fn accessibility(prompt: bool) -> Result<bool, String> {
    let value = request(&[if prompt { "request-access" } else { "access" }])?;
    value["granted"].as_bool().ok_or("Missing accessibility permission status".into())
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
pub fn wait_shortcut_release() -> Result<(), String> {
    request(&["release"]).map(|_| ())
}
pub fn watch(callback: impl Fn(bool) + Send + Sync + 'static) {
    std::thread::spawn(move || {
        use std::io::{BufRead, BufReader};
        let Ok(path) = helper() else {
            callback(false);
            return;
        };
        let Ok(mut child) = Command::new(path)
            .arg("watch")
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::null())
            .spawn()
        else {
            callback(false);
            return;
        };
        if let Some(out) = child.stdout.take() {
            for line in BufReader::new(out).lines().map_while(Result::ok) {
                if let Ok(v) = serde_json::from_str::<Value>(&line) {
                    if let Some(active) = v["active"].as_bool() {
                        callback(active)
                    }
                }
            }
        }
        let _ = child.wait();
        callback(false);
    });
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
        message,
        actual: None,
        restored: false,
    })
}
