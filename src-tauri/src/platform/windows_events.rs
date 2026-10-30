use super::*;
use std::sync::OnceLock;
static CALLBACK:OnceLock<Box<dyn Fn(bool)+Send+Sync>>=OnceLock::new();
unsafe extern "system" fn changed(_:HWINEVENTHOOK,_:u32,_:HWND,_:i32,_:i32,_:u32,_:u32){if let Some(f)=CALLBACK.get(){f(active())}}
pub fn watch(callback:impl Fn(bool)+Send+Sync+'static){let _=CALLBACK.set(Box::new(callback));std::thread::spawn(||unsafe{
 if let Some(f)=CALLBACK.get(){f(active())}
 let hook=SetWinEventHook(EVENT_SYSTEM_FOREGROUND,EVENT_SYSTEM_FOREGROUND,None,Some(changed),0,0,WINEVENT_OUTOFCONTEXT);
 if hook.0.is_null(){return}let mut msg=MSG::default();while GetMessageW(&mut msg,None,0,0).as_bool(){let _=TranslateMessage(&msg);DispatchMessageW(&msg);}let _=UnhookWinEvent(hook);
});}
pub fn wait_shortcut_release()->Result<(),String>{use windows::Win32::UI::Input::KeyboardAndMouse::*;let start=std::time::Instant::now();loop{unsafe{if [VK_CONTROL,VK_MENU,VK_SHIFT,VK_LWIN,VK_RWIN].iter().all(|v|GetAsyncKeyState(v.0 as i32)>=0){return Ok(())}}if start.elapsed()>std::time::Duration::from_secs(1){return Err("단축키에서 손을 떼고 다시 시도하십시오. / Release the shortcut keys and retry.".into())}std::thread::sleep(std::time::Duration::from_millis(20));}}
