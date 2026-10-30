use super::Status;
use switcher_core::parse_selection;
#[path = "windows_events.rs"]
mod events;
pub use events::{wait_shortcut_release, watch};
use windows::{
    core::Interface,
    Win32::{
        Foundation::{CloseHandle, HWND},
        System::{
            Com::{
                CoCreateInstance, CoInitializeEx, CoUninitialize, CLSCTX_INPROC_SERVER,
                COINIT_MULTITHREADED,
            },
            Threading::{
                OpenProcess, QueryFullProcessImageNameW, PROCESS_NAME_WIN32,
                PROCESS_QUERY_LIMITED_INFORMATION,
            },
            Variant::VARIANT,
        },
        UI::{Accessibility::*, WindowsAndMessaging::*},
    },
};
fn err(e: impl std::fmt::Display) -> String {
    e.to_string()
}
pub fn foreground() -> Option<(HWND, String)> {
    unsafe {
        let hwnd = GetForegroundWindow();
        if hwnd.0.is_null() {
            return None;
        }
        let mut pid = 0;
        GetWindowThreadProcessId(hwnd, Some(&mut pid));
        let process = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid).ok()?;
        let mut buf = vec![0u16; 32768];
        let mut len = buf.len() as u32;
        let ok = QueryFullProcessImageNameW(
            process,
            PROCESS_NAME_WIN32,
            windows::core::PWSTR(buf.as_mut_ptr()),
            &mut len,
        )
        .is_ok();
        let _ = CloseHandle(process);
        if !ok {
            return None;
        }
        let path = String::from_utf16_lossy(&buf[..len as usize]).to_lowercase();
        let fixture = std::env::args().any(|a| a == "--fixture-target")
            && std::env::current_exe()
                .ok()
                .is_some_and(|p| p.to_string_lossy().to_lowercase() == path);
        let allowed = fixture
            || path.ends_with("\\chatgpt.exe")
                && (path.contains("\\windowsapps\\openai.codex_")
                    || path.contains("\\programs\\chatgpt\\"));
        if !allowed {
            return None;
        }
        let mut title = vec![0u16; 1024];
        let n = GetWindowTextW(hwnd, &mut title);
        let title = String::from_utf16_lossy(&title[..n as usize]);
        if fixture && !title.starts_with("GPT Model Switcher Fixture") {
            return None;
        }
        Some((hwnd, title))
    }
}
pub fn active() -> bool {
    foreground().is_some()
}
pub struct Session {
    pub ui: IUIAutomation,
    pub root: IUIAutomationElement,
    pub hwnd: HWND,
    pub title: String,
    pub document: String,
    pub started: std::time::Instant,
}
impl Drop for Session {
    fn drop(&mut self) {
        unsafe { CoUninitialize() }
    }
}
impl Session {
    pub fn local_surface(&self) -> Result<(), String> {
        unsafe {
            let permissions = self
                .buttons()?
                .iter()
                .filter(|b| {
                    b.CurrentName().ok().is_some_and(|n| {
                        matches!(n.to_string().as_str(), "권한 변경" | "Change permissions")
                    })
                })
                .count();
            let cloud = self.elements(UIA_ImageControlTypeId)?.iter().any(|e| {
                e.CurrentName().ok().is_some_and(|n| {
                    matches!(
                        n.to_string().as_str(),
                        "클라우드 채팅" | "Cloud chat" | "원격 채팅" | "Remote chat"
                    )
                })
            });
            if permissions != 1 || cloud {
                return Err("로컬 작업 대화임을 확인하지 못했습니다. / Cannot verify a local work conversation.".into());
            }
            Ok(())
        }
    }
    pub fn open() -> Result<Self, String> {
        unsafe {
            let (hwnd, title) =
                foreground().ok_or("ChatGPT 앱을 활성화하십시오. / Activate ChatGPT.")?;
            CoInitializeEx(None, COINIT_MULTITHREADED)
                .ok()
                .map_err(err)?;
            let result: Result<Self, String> = (|| {
                let ui: IUIAutomation =
                    CoCreateInstance(&CUIAutomation8, None, CLSCTX_INPROC_SERVER).map_err(err)?;
                if let Ok(u) = ui.cast::<IUIAutomation2>() {
                    let _ = u.SetConnectionTimeout(600);
                    let _ = u.SetTransactionTimeout(600);
                }
                let root = ui.ElementFromHandle(hwnd).map_err(err)?;
                let s = Self {
                    ui,
                    root,
                    hwnd,
                    title,
                    document: String::new(),
                    started: std::time::Instant::now(),
                };
                Ok(s)
            })();
            if result.is_err() {
                CoUninitialize()
            }
            // After construction, Session owns the COM reference. A failed first
            // document read must drop Session once, without a second uninitialize.
            let mut session = result?;
            session.document = session.document_name()?;
            Ok(session)
        }
    }
    pub fn elements(&self, role: UIA_CONTROLTYPE_ID) -> Result<Vec<IUIAutomationElement>, String> {
        unsafe {
            let condition = self
                .ui
                .CreatePropertyCondition(UIA_ControlTypePropertyId, &VARIANT::from(role.0))
                .map_err(err)?;
            let all = self
                .root
                .FindAll(TreeScope_Descendants, &condition)
                .map_err(err)?;
            let count = all.Length().map_err(err)?;
            if count > 12000 {
                return Err("Accessibility tree too large".into());
            }
            let mut found = Vec::new();
            for i in 0..count {
                if let Ok(e) = all.GetElement(i) {
                    if !e.CurrentIsOffscreen().unwrap_or_default().as_bool() {
                        found.push(e)
                    }
                }
            }
            Ok(found)
        }
    }
    pub fn document_name(&self) -> Result<String, String> {
        unsafe {
            Ok(self
                .elements(UIA_DocumentControlTypeId)?
                .iter()
                .filter_map(|e| e.CurrentName().ok().map(|s| s.to_string()))
                .collect::<Vec<_>>()
                .join("|"))
        }
    }
    pub fn check(&self) -> Result<(), String> {
        if self.started.elapsed() > std::time::Duration::from_secs(8) {
            return Err("전환 제한 시간을 초과했습니다. / Switch deadline exceeded.".into());
        }
        let (h, t) = foreground().ok_or("대상 앱이 바뀌었습니다. / Target app changed.")?;
        if h != self.hwnd || t != self.title || self.document_name()? != self.document {
            return Err("대화 또는 창이 바뀌었습니다. / Conversation or window changed.".into());
        }
        Ok(())
    }
    pub fn buttons(&self) -> Result<Vec<IUIAutomationElement>, String> {
        self.elements(UIA_ButtonControlTypeId)
    }
    pub fn picker(&self) -> Result<IUIAutomationElement, String> {
        unsafe {
            let found: Vec<_> = self
                .buttons()?
                .into_iter()
                .filter(|e| {
                    e.CurrentName()
                        .ok()
                        .is_some_and(|n| parse_selection(&n.to_string()).is_some())
                })
                .collect();
            if found.len() != 1 {
                return Err("모델·추론 버튼을 확실하게 식별하지 못했습니다. / Cannot uniquely identify model control.".into());
            }
            Ok(found[0].clone())
        }
    }
    pub fn read(&self) -> Result<switcher_core::Selection, String> {
        unsafe {
            parse_selection(&self.picker()?.CurrentName().map_err(err)?.to_string())
                .ok_or("Cannot read model and reasoning".into())
        }
    }
    pub fn busy(&self) -> Result<bool, String> {
        unsafe {
            Ok(self.buttons()?.iter().any(|e| {
                e.CurrentName().ok().is_some_and(|s| {
                    matches!(
                        s.to_string().to_lowercase().as_str(),
                        "중지"
                            | "중단"
                            | "작업 중지"
                            | "stop"
                            | "stop generating"
                            | "stop streaming"
                            | "stop turn"
                            | "현재 작업 조정"
                    )
                })
            }))
        }
    }
}
pub fn inspect() -> Status {
    match Session::open() {
        Ok(s) => match s.read() {
            Ok(selection) => {
                let busy = s.busy().unwrap_or(true);
                Status {
                    active: true,
                    available: !busy,
                    busy,
                    message: if busy {
                        "응답 생성 중입니다. / Generation in progress."
                    } else {
                        "전환 준비 완료 / Ready to switch."
                    }
                    .into(),
                    selection: Some(selection),
                }
            }
            Err(e) => Status {
                active: true,
                ..Status::unavailable(e)
            },
        },
        Err(e) => Status::unavailable(e),
    }
}
struct QuietWindow {
    session: Session,
    focus: IUIAutomationElement,
    id: String,
}
impl QuietWindow {
    fn new() -> Result<Self, String> {
        let session = Session::open()?;
        session.local_surface()?;
        session.read()?;
        let id = crate::desktop_ipc::resolve(&session.document)?;
        let focus = unsafe { session.ui.GetFocusedElement().map_err(err)? };
        Ok(Self { session, focus, id })
    }
    fn pid(&self) -> u32 {
        let mut pid = 0;
        unsafe {
            GetWindowThreadProcessId(self.session.hwnd, Some(&mut pid));
        }
        pid
    }
}
impl crate::quiet_switch::Context for QuietWindow {
    fn guard(&mut self) -> Result<switcher_core::Selection, String> {
        self.session.check()?;
        self.session.local_surface()?;
        if self.session.busy()? {
            return Err(
                "응답 생성 중에는 전환할 수 없습니다. / Cannot switch during generation.".into(),
            );
        }
        if crate::desktop_ipc::resolve(&self.session.document)? != self.id {
            return Err("Conversation identity changed".into());
        }
        unsafe {
            let current = self.session.ui.GetFocusedElement().map_err(err)?;
            if !self
                .session
                .ui
                .CompareElements(&self.focus, &current)
                .map_err(err)?
                .as_bool()
            {
                return Err("입력 포커스가 바뀌었습니다. / Input focus changed.".into());
            }
        }
        self.session.read()
    }
}
pub fn diagnose() -> Result<serde_json::Value, String> {
    let window = QuietWindow::new()?;
    let mut desktop = crate::desktop_ipc::Desktop::connect(window.pid())?;
    let settings = desktop.read(&window.id)?;
    Ok(
        serde_json::json!({"connection":"focus-free","selection":window.session.read()?,"settings":settings}),
    )
}
pub fn switch(p: &switcher_core::Preset) -> switcher_core::switching::Outcome {
    use crate::quiet_switch::Context;
    let result = (|| {
        let mut window = QuietWindow::new()?;
        window.guard()?;
        let mut desktop = crate::desktop_ipc::Desktop::connect(window.pid())?;
        let id = window.id.clone();
        Ok(crate::quiet_switch::apply(
            &mut window,
            &mut desktop,
            &id,
            p,
        ))
    })();
    result.unwrap_or_else(|message| switcher_core::switching::Outcome {
        success: false,
        already_applied: false,
        message,
        actual: None,
        restored: false,
    })
}
