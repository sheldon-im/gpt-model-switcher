use super::Status;
use switcher_core::parse_selection;
#[path="windows_switch.rs"] mod switching;
pub use switching::switch;
use windows::{core::Interface,Win32::{Foundation::{HWND,CloseHandle},System::{Com::{CoInitializeEx,CoUninitialize,CoCreateInstance,COINIT_MULTITHREADED,CLSCTX_INPROC_SERVER},Threading::{OpenProcess,QueryFullProcessImageNameW,PROCESS_QUERY_LIMITED_INFORMATION,PROCESS_NAME_WIN32},Variant::VARIANT},UI::{Accessibility::*,WindowsAndMessaging::*}}};
fn err(e:impl std::fmt::Display)->String{e.to_string()}
pub fn foreground()->Option<(HWND,String)> {unsafe{
 let hwnd=GetForegroundWindow();if hwnd.0.is_null(){return None}
 let mut pid=0;GetWindowThreadProcessId(hwnd,Some(&mut pid));
 let process=OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION,false,pid).ok()?;
 let mut buf=vec![0u16;32768];let mut len=buf.len() as u32;
 let ok=QueryFullProcessImageNameW(process,PROCESS_NAME_WIN32,windows::core::PWSTR(buf.as_mut_ptr()),&mut len).is_ok();let _=CloseHandle(process);
 if !ok{return None}let path=String::from_utf16_lossy(&buf[..len as usize]).to_lowercase();
 let allowed=path.ends_with("\\chatgpt.exe")&&(path.contains("\\windowsapps\\openai.codex_")||path.contains("\\programs\\chatgpt\\"));
 if !allowed{return None}
 let mut title=vec![0u16;1024];let n=GetWindowTextW(hwnd,&mut title);Some((hwnd,String::from_utf16_lossy(&title[..n as usize])))
}}
pub fn active()->bool{foreground().is_some()}
pub struct Session {pub ui:IUIAutomation,pub root:IUIAutomationElement,pub hwnd:HWND,pub title:String,pub started:std::time::Instant}
impl Drop for Session{fn drop(&mut self){unsafe{CoUninitialize()}}}
impl Session {
 pub fn open()->Result<Self,String>{unsafe{
  let (hwnd,title)=foreground().ok_or("ChatGPT 앱을 활성화하십시오. / Activate ChatGPT.")?;
  CoInitializeEx(None,COINIT_MULTITHREADED).ok().map_err(err)?;
  let result=(||{let ui:IUIAutomation=CoCreateInstance(&CUIAutomation8,None,CLSCTX_INPROC_SERVER).map_err(err)?;
   if let Ok(u)=ui.cast::<IUIAutomation2>(){let _=u.SetConnectionTimeout(600);let _=u.SetTransactionTimeout(600);}
   let root=ui.ElementFromHandle(hwnd).map_err(err)?;Ok(Self{ui,root,hwnd,title,started:std::time::Instant::now()})})();
  if result.is_err(){CoUninitialize()}result
 }}
 pub fn elements(&self,role:UIA_CONTROLTYPE_ID)->Result<Vec<IUIAutomationElement>,String>{unsafe{
  let condition=self.ui.CreatePropertyCondition(UIA_ControlTypePropertyId,&VARIANT::from(role.0)).map_err(err)?;
  let all=self.root.FindAll(TreeScope_Descendants,&condition).map_err(err)?;
  let count=all.Length().map_err(err)?;if count>12000{return Err("Accessibility tree too large".into())}
  let mut found=Vec::new();for i in 0..count{if let Ok(e)=all.GetElement(i){if !e.CurrentIsOffscreen().unwrap_or_default().as_bool(){found.push(e)}}}Ok(found)
 }}
 pub fn check(&self)->Result<(),String>{if self.started.elapsed()>std::time::Duration::from_secs(8){return Err("전환 제한 시간을 초과했습니다. / Switch deadline exceeded.".into())}let (h,t)=foreground().ok_or("대상 앱이 바뀌었습니다. / Target app changed.")?;if h!=self.hwnd||t!=self.title{return Err("대화 또는 창이 바뀌었습니다. / Conversation or window changed.".into())}Ok(())}
 pub fn buttons(&self)->Result<Vec<IUIAutomationElement>,String>{self.elements(UIA_ButtonControlTypeId)}
 pub fn picker(&self)->Result<IUIAutomationElement,String>{unsafe{
  let found:Vec<_>=self.buttons()?.into_iter().filter(|e|e.CurrentName().ok().is_some_and(|n|parse_selection(&n.to_string()).is_some())).collect();
  if found.len()!=1{return Err("모델·추론 버튼을 확실하게 식별하지 못했습니다. / Cannot uniquely identify model control.".into())}Ok(found[0].clone())
 }}
 pub fn read(&self)->Result<switcher_core::Selection,String>{unsafe{parse_selection(&self.picker()?.CurrentName().map_err(err)?.to_string()).ok_or("Cannot read model and reasoning".into())}}
 pub fn busy(&self)->Result<bool,String>{unsafe{Ok(self.buttons()?.iter().any(|e|e.CurrentName().ok().is_some_and(|s|matches!(s.to_string().to_lowercase().as_str(),"중지"|"중단"|"작업 중지"|"stop"|"stop generating"|"stop streaming"|"stop turn"|"현재 작업 조정"))))}}
}
pub fn inspect()->Status{match Session::open(){Ok(s)=>match s.read(){Ok(selection)=>{let busy=s.busy().unwrap_or(true);Status{active:true,available:!busy,busy,message:if busy{"응답 생성 중입니다. / Generation in progress."}else{"전환 준비 완료 / Ready to switch."}.into(),selection:Some(selection)}},Err(e)=>Status{active:true,..Status::unavailable(e)}},Err(e)=>Status::unavailable(e)}}
#[cfg(test)] mod tests{#[test]fn supported_path_is_narrow(){assert!(!super::active()||super::foreground().is_some());}}

