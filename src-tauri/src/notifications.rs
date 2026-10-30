use std::sync::atomic::{AtomicU64, Ordering};
use tauri::{AppHandle,Manager,WebviewWindowBuilder,WebviewUrl};
static SEQUENCE:AtomicU64=AtomicU64::new(0);
pub fn setup(app:&AppHandle)->tauri::Result<()>{
 WebviewWindowBuilder::new(app,"notification",WebviewUrl::App("index.html#notification".into()))
 .title("GPT Model Switcher result").inner_size(400.,180.).position(24.,24.)
 .decorations(false).resizable(false).always_on_top(true).skip_taskbar(true)
 .focusable(false).focused(false).visible(false).build()?;
 Ok(())
}
pub fn show(app:&AppHandle){
 let sequence=SEQUENCE.fetch_add(1,Ordering::SeqCst)+1;let app=app.clone();
 let main=app.clone();let _=app.run_on_main_thread(move||{if let Some(w)=main.get_webview_window("notification"){
   #[cfg(windows)]unsafe{if let Ok(hwnd)=w.hwnd(){use windows::Win32::UI::WindowsAndMessaging::{ShowWindow,SW_SHOWNOACTIVATE};let _=ShowWindow(hwnd,SW_SHOWNOACTIVATE);}}
   #[cfg(target_os="macos")]unsafe{if let Ok(pointer)=w.ns_window(){let window=pointer as *mut objc2::runtime::AnyObject;let _:()=objc2::msg_send![window,orderFront:std::ptr::null_mut::<objc2::runtime::AnyObject>()];}}
 }});
 std::thread::spawn(move||{std::thread::sleep(std::time::Duration::from_secs(6));if SEQUENCE.load(Ordering::SeqCst)==sequence{if let Some(w)=app.get_webview_window("notification"){let _=w.hide();}}});
}
