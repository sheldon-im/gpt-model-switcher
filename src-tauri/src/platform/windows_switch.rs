use super::*;
use switcher_core::{normalized,Preset,switching::{Target,Outcome,apply}};
use windows::Win32::UI::Input::KeyboardAndMouse::{SendInput,INPUT,INPUT_0,INPUT_KEYBOARD,KEYBDINPUT,VK_ESCAPE,KEYEVENTF_KEYUP};
use windows::Win32::UI::Input::KeyboardAndMouse::{VIRTUAL_KEY,VK_LEFT,VK_RIGHT,VK_RETURN};
impl Session {
 fn key(&self,key:VIRTUAL_KEY)->Result<(),String>{self.check()?;unsafe{let inputs=[INPUT{r#type:INPUT_KEYBOARD,Anonymous:INPUT_0{ki:KEYBDINPUT{wVk:key,..Default::default()}}},INPUT{r#type:INPUT_KEYBOARD,Anonymous:INPUT_0{ki:KEYBDINPUT{wVk:key,dwFlags:KEYEVENTF_KEYUP,..Default::default()}}}];if SendInput(&inputs,std::mem::size_of::<INPUT>() as i32)!=2{return Err("Key input failed".into())}Ok(())}}
 fn adjust_power(&self,effort:&str)->Result<(),String>{
  self.open_picker()?;let initial=self.read()?;let rank=|e:&str|switcher_core::EFFORTS.iter().position(|v|*v==e).ok_or("Unknown effort");
  let target=rank(effort)?;for _ in 0..8{let before=self.read()?;if before.effort==effort{return self.key(VK_RETURN)}
   if !switcher_core::same_model(&before.model,&initial.model){return Err("Power control changed model; restoring selection / 파워가 모델을 변경하여 복구합니다.".into())}
   let control=self.find_choice(&["파워","Power","Reasoning effort","추론 수준"] )?.ok_or("No accessible reasoning control / 추론 컨트롤을 찾지 못했습니다.")?;
   unsafe{self.check()?;control.SetFocus().map_err(err)?;if !self.ui.CompareElements(&control,&self.ui.GetFocusedElement().map_err(err)?).map_err(err)?.as_bool(){return Err("Power control focus verification failed".into())}}
   self.key(if rank(&before.effort)?<target{VK_RIGHT}else{VK_LEFT})?;
   self.wait_for(||Ok(Session::read(self).is_ok_and(|s|s!=before)))?;
  }Err("Requested effort is not available / 요청한 추론 수준을 선택할 수 없습니다.".into())
 }
 fn invoke(&self,e:&IUIAutomationElement)->Result<(),String>{self.check()?;unsafe{
  if !e.CurrentIsEnabled().map_err(err)?.as_bool(){return Err("Selection is disabled / 선택할 수 없는 항목입니다.".into())}
  if let Ok(p)=e.GetCurrentPatternAs::<IUIAutomationInvokePattern>(UIA_InvokePatternId){p.Invoke().map_err(err)}
  else if let Ok(p)=e.GetCurrentPatternAs::<IUIAutomationSelectionItemPattern>(UIA_SelectionItemPatternId){p.Select().map_err(err)}
  else if let Ok(p)=e.GetCurrentPatternAs::<IUIAutomationExpandCollapsePattern>(UIA_ExpandCollapsePatternId){p.Expand().map_err(err)}
  else {Err("No supported accessibility action".into())}
 }}
 fn overlays(&self)->Result<Vec<IUIAutomationElement>,String>{unsafe{
  let mut v=self.elements(UIA_MenuControlTypeId)?;
  for role in [UIA_ListControlTypeId,UIA_PaneControlTypeId,UIA_WindowControlTypeId] {
   for e in self.elements(role)? {if e.CurrentAriaRole().ok().is_some_and(|s|matches!(s.to_string().as_str(),"listbox"|"dialog")){v.push(e)}}
  } Ok(v)
 }}
 fn choices(&self)->Result<Vec<IUIAutomationElement>,String>{unsafe{
  let mut result=Vec::new();let cond=self.ui.CreateTrueCondition().map_err(err)?;
  for overlay in self.overlays()?{let elements=overlay.FindAll(TreeScope_Descendants,&cond).map_err(err)?;for i in 0..elements.Length().map_err(err)?{
   let e=elements.GetElement(i).map_err(err)?;let role=e.CurrentControlType().map_err(err)?;
   if [UIA_MenuItemControlTypeId,UIA_ListItemControlTypeId,UIA_RadioButtonControlTypeId,UIA_ButtonControlTypeId].contains(&role)&&!e.CurrentIsOffscreen().unwrap_or_default().as_bool(){result.push(e)}
  }}Ok(result)
 }}
 fn find_choice(&self,names:&[&str])->Result<Option<IUIAutomationElement>,String>{unsafe{
  let matches:Vec<_>=self.choices()?.into_iter().filter(|e|e.CurrentName().ok().is_some_and(|name|names.iter().any(|n|normalized(name.to_string().lines().next().unwrap_or(""))==normalized(n)))).collect();
  // Nested overlays can expose the same element more than once.
  if matches.is_empty(){return Ok(None)}let first=&matches[0];for other in matches.iter().skip(1){if !self.ui.CompareElements(first,other).map_err(err)?.as_bool(){return Err("Ambiguous menu selection / 메뉴 항목이 중복됩니다.".into())}}Ok(Some(first.clone()))
 }}
 fn open_picker(&self)->Result<(),String>{
  self.check()?;if self.overlays()?.is_empty(){self.invoke(&self.picker()?)?;}
  self.wait_for(||Ok(!self.overlays()?.is_empty()))?;
  Ok(())
 }
 fn wait_for(&self,mut condition:impl FnMut()->Result<bool,String>)->Result<(),String>{let end=std::time::Instant::now()+std::time::Duration::from_secs(2);
  loop{self.check()?;if condition()?{return Ok(())}if std::time::Instant::now()>=end{return Err("메뉴 확인 시간이 초과되었습니다. / Menu timeout.".into())}std::thread::sleep(std::time::Duration::from_millis(60));}
 }
 fn choose(&self,names:&[&str],submenus:&[&str])->Result<(),String>{
  self.open_picker()?;
  if let Some(e)=self.find_choice(names)?{return self.invoke(&e)}
  for submenu in submenus{if let Some(e)=self.find_choice(&[submenu])?{self.invoke(&e)?;let _=self.wait_for(||Ok(self.find_choice(names)?.is_some()));if let Some(choice)=self.find_choice(names)?{return self.invoke(&choice)}}}
  Err(format!("현재 화면에서 선택할 수 없습니다. / Not available in this view: {}",names[0]))
 }
 pub fn menu_names(&self)->Result<Vec<String>,String>{self.open_picker()?;unsafe{Ok(self.choices()?.iter().filter_map(|e|e.CurrentName().ok().map(|s|s.to_string())).collect())}}
 pub fn submenu_names(&self,aliases:&[&str])->Result<Vec<String>,String>{self.open_picker()?;let e=self.find_choice(aliases)?.ok_or("Submenu missing")?;self.invoke(&e)?;std::thread::sleep(std::time::Duration::from_millis(200));unsafe{Ok(self.choices()?.iter().filter_map(|e|e.CurrentName().ok().map(|s|s.to_string())).collect())}}
 pub fn submenu_dump(&self)->Result<Vec<String>,String>{self.open_picker()?;if let Some(e)=self.find_choice(&["파워","Power"])?{self.invoke(&e)?;}std::thread::sleep(std::time::Duration::from_millis(150));unsafe{let cond=self.ui.CreateTrueCondition().map_err(err)?;let mut lines=Vec::new();for overlay in self.overlays()?{let all=overlay.FindAll(TreeScope_Descendants,&cond).map_err(err)?;for i in 0..all.Length().map_err(err)?{let e=all.GetElement(i).map_err(err)?;lines.push(format!("{:?} | {} | {}",e.CurrentControlType().ok(),e.CurrentAriaRole().unwrap_or_default(),e.CurrentName().unwrap_or_default()));}}Ok(lines)}}
 pub fn current_efforts_dump(&self)->Result<Vec<String>,String>{let current=self.read()?;self.choose(&[&current.model],&["모델 선택","Choose model","Select model"])?;self.submenu_dump()}
 fn escape(&self)->Result<(),String>{self.check()?;unsafe{
  let inputs=[INPUT{r#type:INPUT_KEYBOARD,Anonymous:INPUT_0{ki:KEYBDINPUT{wVk:VK_ESCAPE,..Default::default()}}},INPUT{r#type:INPUT_KEYBOARD,Anonymous:INPUT_0{ki:KEYBDINPUT{wVk:VK_ESCAPE,dwFlags:KEYEVENTF_KEYUP,..Default::default()}}}];
  if SendInput(&inputs,std::mem::size_of::<INPUT>() as i32)!=2{return Err("Could not dismiss menu".into())}Ok(())
 }}
}
impl Target for Session{
 fn check(&mut self)->Result<(),String>{Session::check(self)}
 fn busy(&mut self)->Result<bool,String>{Session::busy(self)}
 fn read(&mut self)->Result<switcher_core::Selection,String>{Session::read(self)}
 fn model(&mut self,v:&str)->Result<(),String>{
  self.choose(&[v],&["모델 선택","Choose model","Select model","Advanced","고급","Models","모델","More models","다른 모델"])?;
  self.wait_for(||Ok(Session::read(self).is_ok_and(|s|switcher_core::same_model(&s.model,v))))
 }
 fn effort(&mut self,v:&str)->Result<(),String>{
  let names=match v{"medium"=>vec!["Medium","중간"],"high"=>vec!["High","높음"],"low"=>vec!["Low","낮음"],"xhigh"=>vec!["Extra High","xhigh","매우 높음"],"none"=>vec!["None","없음"],"max"=>vec!["Max","최대"],"ultra"=>vec!["Ultra"],"minimal"=>vec!["Minimal"],_=>return Err("Unsupported effort".into())};
  self.open_picker()?;
  if let Some(item)=self.find_choice(&names)?{self.invoke(&item)?;}else{self.adjust_power(v)?;}
  self.wait_for(||Ok(Session::read(self).is_ok_and(|s|s.effort==v)))
 }
 fn finish(&mut self)->Result<(),String>{self.check()?;if !self.overlays()?.is_empty(){self.escape()?}
  unsafe{let edits=self.elements(UIA_EditControlTypeId)?;let composers:Vec<_>=edits.into_iter().filter(|e|e.CurrentName().ok().is_some_and(|n|{let s=n.to_string().to_lowercase();s.contains("chatgpt")||s.contains("message")||s.contains("메시지")||s.contains("질문")})).collect();if composers.len()!=1{return Err("Cannot uniquely restore composer focus / 입력창 포커스 복귀를 확인할 수 없습니다.".into())}self.check()?;composers[0].SetFocus().map_err(err)}
 }
}
pub fn switch(p:&Preset)->Outcome{match Session::open(){Ok(mut s)=>apply(&mut s,p),Err(e)=>Outcome{success:false,message:e,actual:None,restored:false}}}

