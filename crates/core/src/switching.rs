use crate::{Preset,Selection,same_model};
use serde::{Serialize,Deserialize};
#[derive(Clone,Debug,Serialize,Deserialize)]
pub struct Outcome{pub success:bool,pub message:String,pub actual:Option<Selection>,pub restored:bool}
pub trait Target {
 fn check(&mut self)->Result<(),String>;
 fn busy(&mut self)->Result<bool,String>;
 fn read(&mut self)->Result<Selection,String>;
 fn model(&mut self,value:&str)->Result<(),String>;
 fn effort(&mut self,value:&str)->Result<(),String>;
 fn finish(&mut self)->Result<(),String>;
}
pub fn apply(target:&mut impl Target,p:&Preset)->Outcome{
 let before=(||{target.check()?;if target.busy()?{return Err("응답 생성 중에는 전환할 수 없습니다. / Cannot switch during generation.".into())}target.read()})();
 let before=match before{Ok(v)=>v,Err(e)=>return Outcome{success:false,message:e,actual:None,restored:false}};
 let result=(||{if !same_model(&before.model,&p.model){target.check()?;target.model(&p.model)?;}
  let current=target.read()?;if current.effort!=p.effort{target.check()?;target.effort(&p.effort)?;}
  target.check()?;let actual=target.read()?;if !actual.matches(p){return Err("선택값 검증에 실패했습니다. / Selection verification failed.".into())}
  target.finish()?;let final_value=target.read()?;if !final_value.matches(p){return Err("포커스 복귀 후 선택값이 달라졌습니다. / Selection changed after focus restoration.".into())}Ok(final_value)})();
 match result{Ok(actual)=>Outcome{success:true,message:format!("{} · {}",actual.model,actual.effort),actual:Some(actual),restored:false},Err(e)=>{
  // No further mutation after a context change or unreadable selection.
  let current=target.check().and_then(|_|target.read());let mut restored=false;
  if let Ok(ref actual)=current{if actual!=&before&&!target.busy().unwrap_or(true){let restore=(||{target.check()?;if !same_model(&actual.model,&before.model){target.model(&before.model)?;}target.check()?;target.effort(&before.effort)?;Ok::<_,String>(())})();restored=restore.is_ok()&&target.read().is_ok_and(|s|same_model(&s.model,&before.model)&&s.effort==before.effort);}}
  let actual=if target.check().is_ok(){target.read().ok()}else{None};let _=target.finish();
  Outcome{success:false,message:e,actual,restored}
 }}
}
#[cfg(test)] mod tests{
 use super::*;use crate::Settings;
 struct Mock{value:Selection,busy:bool,changed:bool,effort_failure:bool,mutations:usize,lie:bool}
 impl Target for Mock{fn check(&mut self)->Result<(),String>{if self.changed{Err("changed".into())}else{Ok(())}}fn busy(&mut self)->Result<bool,String>{Ok(self.busy)}fn read(&mut self)->Result<Selection,String>{Ok(self.value.clone())}fn model(&mut self,v:&str)->Result<(),String>{self.mutations+=1;if !self.lie{self.value.model=v.into()}Ok(())}fn effort(&mut self,v:&str)->Result<(),String>{self.mutations+=1;if self.effort_failure&&v=="medium"{return Err("unsupported".into())}if !self.lie{self.value.effort=v.into()}Ok(())}fn finish(&mut self)->Result<(),String>{Ok(())}}
 fn mock()->Mock{Mock{value:Selection{model:"gpt-6.1-sol".into(),effort:"high".into()},busy:false,changed:false,effort_failure:false,mutations:0,lie:false}}
 #[test]fn busy_never_mutates(){let mut m=mock();m.busy=true;assert!(!apply(&mut m,&Settings::default().presets[0]).success);assert_eq!(m.mutations,0);}
 #[test]fn context_change_never_mutates(){let mut m=mock();m.changed=true;assert!(!apply(&mut m,&Settings::default().presets[0]).success);assert_eq!(m.mutations,0);}
 #[test]fn verifies_both_values(){let mut m=mock();let p=&Settings::default().presets[0];assert!(apply(&mut m,p).success);assert!(m.value.matches(p));}
 #[test]fn partial_failure_restores_original(){let mut m=mock();m.effort_failure=true;let r=apply(&mut m,&Settings::default().presets[0]);assert!(!r.success);assert!(r.restored);assert_eq!(m.value.model,"gpt-6.1-sol");}
 #[test]fn false_success_is_rejected(){let mut m=mock();m.lie=true;assert!(!apply(&mut m,&Settings::default().presets[0]).success);}
 #[test]fn already_selected_does_not_mutate(){let mut m=mock();assert!(apply(&mut m,&Settings::default().presets[1]).success);assert_eq!(m.mutations,0);}
}
