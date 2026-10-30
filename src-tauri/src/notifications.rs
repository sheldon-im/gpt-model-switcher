use std::sync::atomic::{AtomicU64, Ordering};
use tauri::{AppHandle,Manager,WebviewWindowBuilder,WebviewUrl};
static SEQUENCE:AtomicU64=AtomicU64::new(0);
static ANNOUNCEMENT_ERROR:std::sync::Mutex<Option<String>>=std::sync::Mutex::new(None);
pub fn last_error()->Option<String>{ANNOUNCEMENT_ERROR.lock().ok().and_then(|e|e.clone())}
pub fn setup(app:&AppHandle,enabled:bool)->tauri::Result<()>{
 if !enabled || app.get_webview_window("notification").is_some(){return Ok(())}
 WebviewWindowBuilder::new(app,"notification",WebviewUrl::App("index.html#notification".into()))
 .title("GPT Model Switcher result").inner_size(400.,180.).position(24.,24.)
 .decorations(false).resizable(false).always_on_top(true).skip_taskbar(true)
 .focusable(false).focused(false).visible(false).build()?;
 Ok(())
}
pub fn configure(app:&AppHandle,enabled:bool){
 let handle=app.clone();let _=app.run_on_main_thread(move||{
  if enabled {let _=setup(&handle,true);}
  else if let Some(window)=handle.get_webview_window("notification"){let _=window.hide();}
 });
}
pub fn announce(app:&AppHandle,message:String){
 #[cfg(windows)]let app_for_call=app.clone();let _=app.run_on_main_thread(move||{
   #[cfg(windows)]let result=(||unsafe{
     use windows::{core::BSTR,Win32::{System::Com::{CoInitializeEx,CoUninitialize,COINIT_APARTMENTTHREADED},UI::Accessibility::*}};
     let initialized=CoInitializeEx(None,COINIT_APARTMENTTHREADED).is_ok();
     let result=(||{let window=app_for_call.get_webview_window("main").ok_or("Missing announcement host")?;let hwnd=window.hwnd().map_err(|e|e.to_string())?;
       let provider=UiaHostProviderFromHwnd(hwnd).map_err(|e|e.to_string())?;
       UiaRaiseNotificationEvent(&provider,NotificationKind_ActionCompleted,NotificationProcessing_CurrentThenMostRecent,&BSTR::from(message),&BSTR::from("gpt-model-switcher")).map_err(|e|e.to_string())})();
     if initialized{CoUninitialize()}result
   })();
   #[cfg(target_os="macos")]let result=crate::platform::announce(&message);
   if let Ok(mut error)=ANNOUNCEMENT_ERROR.lock(){*error=result.err();}
 });
}
pub fn show(app:&AppHandle){
 let sequence=SEQUENCE.fetch_add(1,Ordering::SeqCst)+1;let app=app.clone();
 let main=app.clone();let _=app.run_on_main_thread(move||{
 if setup(&main,true).is_err(){return}
 if let Some(w)=main.get_webview_window("notification"){
   #[cfg(windows)]unsafe{if let Ok(hwnd)=w.hwnd(){use windows::Win32::UI::WindowsAndMessaging::{ShowWindow,SW_SHOWNOACTIVATE};let _=ShowWindow(hwnd,SW_SHOWNOACTIVATE);}}
   #[cfg(target_os="macos")]unsafe{if let Ok(pointer)=w.ns_window(){let window=pointer as *mut objc2::runtime::AnyObject;let _:()=objc2::msg_send![window,orderFront:std::ptr::null_mut::<objc2::runtime::AnyObject>()];}}
 }});
 std::thread::spawn(move||{std::thread::sleep(std::time::Duration::from_secs(6));if SEQUENCE.load(Ordering::SeqCst)==sequence{if let Some(w)=app.get_webview_window("notification"){let _=w.hide();}}});
}
