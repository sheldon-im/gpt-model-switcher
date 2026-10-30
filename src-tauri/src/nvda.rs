//! NVDA's own queue and braille display, without focus events or a separate synthesizer.
use std::path::Path;
use windows::{
    core::{HSTRING, PCSTR},
    Win32::{
        Foundation::{FreeLibrary, HANDLE, HMODULE},
        System::{LibraryLoader::*, StationsAndDesktops::*},
    },
};
type Test = unsafe extern "system" fn() -> u32;
type Message = unsafe extern "system" fn(*const u16) -> u32;
struct Controller {
    module: HMODULE,
    test: Test,
    speak: Message,
    braille: Message,
}
impl Drop for Controller {
    fn drop(&mut self) {
        unsafe {
            let _ = FreeLibrary(self.module);
        }
    }
}
impl Controller {
    fn load(path: &Path) -> Result<Self, String> {
        if !path.is_absolute() {
            return Err("NVDA library path must be absolute".into());
        }
        unsafe {
            let module=LoadLibraryExW(&HSTRING::from(path.as_os_str()),None,LOAD_LIBRARY_SEARCH_DLL_LOAD_DIR|LOAD_LIBRARY_SEARCH_SYSTEM32).map_err(|e|format!("NVDA 알림 라이브러리를 읽지 못했습니다. / Cannot load NVDA notification library: {e}"))?;
            let result = (|| {
                let test = GetProcAddress(
                    module,
                    PCSTR(c"nvdaController_testIfRunning".as_ptr().cast()),
                )
                .ok_or("Missing NVDA running check")?;
                let speak =
                    GetProcAddress(module, PCSTR(c"nvdaController_speakText".as_ptr().cast()))
                        .ok_or("Missing NVDA speech entry point")?;
                let braille = GetProcAddress(
                    module,
                    PCSTR(c"nvdaController_brailleMessage".as_ptr().cast()),
                )
                .ok_or("Missing NVDA braille entry point")?;
                Ok(Self {
                    module,
                    test: std::mem::transmute::<unsafe extern "system" fn() -> isize, Test>(test),
                    speak: std::mem::transmute::<unsafe extern "system" fn() -> isize, Message>(
                        speak,
                    ),
                    braille: std::mem::transmute::<unsafe extern "system" fn() -> isize, Message>(
                        braille,
                    ),
                })
            })();
            if result.is_err() {
                let _ = FreeLibrary(module);
            }
            result
        }
    }
}
pub fn desktop_is_interactive() -> Result<(), String> {
    unsafe {
        let desktop=OpenInputDesktop(DESKTOP_CONTROL_FLAGS(0),false,DESKTOP_READOBJECTS).map_err(|_|"잠금/보안 화면에서는 안내하지 않습니다. / Cannot announce on a locked or secure desktop.")?;
        let mut name = [0u16; 256];
        let result = GetUserObjectInformationW(
            HANDLE(desktop.0),
            UOI_NAME,
            Some(name.as_mut_ptr().cast()),
            (name.len() * 2) as u32,
            None,
        );
        let _ = CloseDesktop(desktop);
        result.map_err(|e| e.to_string())?;
        let length = name
            .iter()
            .position(|c| *c == 0)
            .ok_or("Invalid desktop name")?;
        if !String::from_utf16_lossy(&name[..length]).eq_ignore_ascii_case("default") {
            return Err(
                "보안 화면에서는 안내하지 않습니다. / Secure desktop announcement blocked.".into(),
            );
        }
        Ok(())
    }
}
/// None means no NVDA server, allowing the generic accessibility path. Once
/// NVDA accepts speech, no second speech backend is invoked, even if braille fails.
pub fn announce(resource_dir: &Path, message: &str) -> Result<Option<Option<String>>, String> {
    desktop_is_interactive()?;
    if message.is_empty() || message.contains('\0') {
        return Err("Invalid announcement text".into());
    }
    let library = resource_dir.join("nvda/nvdaControllerClient.dll");
    #[cfg(debug_assertions)]
    let library = if library.exists() {
        library
    } else {
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("vendor/nvda-controller/nvdaControllerClient.dll")
    };
    let client = Controller::load(&library)?;
    let running = unsafe { (client.test)() };
    if running == 1722 {
        return Ok(None);
    }
    if running != 0 {
        return Err(format!(
            "NVDA 연결 실패 / NVDA connection failed: {running}"
        ));
    }
    deliver(
        message,
        |s| unsafe { (client.speak)(s) },
        |s| unsafe { (client.braille)(s) },
    )
    .map(Some)
}
fn deliver(
    message: &str,
    mut speak: impl FnMut(*const u16) -> u32,
    mut braille: impl FnMut(*const u16) -> u32,
) -> Result<Option<String>, String> {
    let wide: Vec<u16> = message.encode_utf16().chain(Some(0)).collect();
    let code = speak(wide.as_ptr());
    if code != 0 {
        return Err(format!(
            "NVDA 음성 안내 전달 실패 / NVDA speech request failed: {code}"
        ));
    }
    let code = braille(wide.as_ptr());
    Ok(if code == 0 {
        None
    } else {
        Some(format!(
            "NVDA 점자 안내 전달 실패 / NVDA braille request failed: {code}"
        ))
    })
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn one_speech_and_one_braille_share_the_unicode_message() {
        let mut spoken = Vec::new();
        let mut brailled = Vec::new();
        let read = |p: *const u16| unsafe {
            let mut n = 0;
            while *p.add(n) != 0 {
                n += 1
            }
            String::from_utf16_lossy(std::slice::from_raw_parts(p, n))
        };
        let warning = deliver(
            "전환 완료. Terra medium",
            |p| {
                spoken.push(read(p));
                0
            },
            |p| {
                brailled.push(read(p));
                0
            },
        )
        .unwrap();
        assert!(warning.is_none());
        assert_eq!(spoken, brailled);
        assert_eq!(spoken.len(), 1);
    }
    #[test]
    fn rejected_speech_is_not_reported_as_delivered() {
        let mut braille_calls = 0;
        assert!(deliver(
            "test",
            |_| 5,
            |_| {
                braille_calls += 1;
                0
            }
        )
        .is_err());
        assert_eq!(braille_calls, 0);
    }
    #[test]
    fn braille_failure_does_not_respeak_accepted_speech() {
        let mut speech_calls = 0;
        let warning = deliver(
            "test",
            |_| {
                speech_calls += 1;
                0
            },
            |_| 1722,
        )
        .unwrap();
        assert!(warning.is_some());
        assert_eq!(speech_calls, 1);
    }
}
