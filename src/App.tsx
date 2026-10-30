import {useEffect,useRef,useState} from 'react';
import {api,Settings,Preset,Status,Outcome,AccessibilityStatus} from './api';
import {validateShortcuts} from './shortcuts';
import {outcomeTitle} from './outcome';
const efforts=['none','minimal','low','medium','high','xhigh','max','ultra'];
export function App(){
 const [settings,setSettings]=useState<Settings>();const [status,setStatus]=useState<Status>();
 const [history,setHistory]=useState<Outcome[]>([]);
 const [access,setAccess]=useState<AccessibilityStatus>();
 const [error,setError]=useState('');const [notice,setNotice]=useState('');const [saving,setSaving]=useState(false);
 const [testing,setTesting]=useState(false);const [testInfo,setTestInfo]=useState('');
 const addButton=useRef<HTMLButtonElement>(null);const ko=settings?.language!=='en';const t=(a:string,b:string)=>ko?a:b;
 useEffect(()=>{api.load().then(setSettings).catch(e=>setError(String(e)));api.status().then(setStatus).catch(e=>setError(String(e)));},[]);
 useEffect(()=>{document.documentElement.lang=ko?'ko':'en';},[ko]);
 useEffect(()=>{
  let disposed=false;
  const refresh=()=>api.accessibility().then(a=>{if(!disposed)setAccess(a)}).catch(e=>{if(!disposed)setError(String(e))});
  refresh();const timer=window.setInterval(refresh,2000);window.addEventListener('focus',refresh);
  return()=>{disposed=true;window.clearInterval(timer);window.removeEventListener('focus',refresh)};
 },[]);
 useEffect(()=>{let disposed=false;let stop:(()=>void)|undefined;api.history().then(h=>{if(!disposed)setHistory(h)}).catch(e=>setError(String(e)));api.subscribe(outcome=>setHistory(h=>[outcome,...h].slice(0,20))).then(unlisten=>{if(disposed)unlisten();else stop=unlisten;});return()=>{disposed=true;stop?.();};},[]);
 const update=(id:string,patch:Partial<Preset>)=>setSettings(s=>s&&({...s,presets:s.presets.map(p=>p.id===id?{...p,...patch}:p)}));
 async function save(){if(!settings)return;setSaving(true);setError('');setNotice('');try{if(settings.presets.some(p=>!p.name.trim()||!p.model.trim()))throw new Error(t('이름과 모델을 입력하십시오.','Enter a name and model.'));validateShortcuts(settings);await api.save(settings);setNotice(t('설정을 저장했습니다.','Settings saved.'));}catch(e){setError(String(e));}finally{setSaving(false);}}
 if(!settings)return <main><h1>GPT Model Switcher</h1><p role={error?'alert':'status'}>{error||'설정을 불러오는 중입니다. / Loading settings…'}</p></main>;
 return <><a className="skip" href="#presets">{t('프리셋으로 바로 가기','Skip to presets')}</a><main>
 <header><p className="eyebrow">GPT MODEL SWITCHER</p><h1>{t('모델 전환 설정','Model switching settings')}</h1><p>{t('ChatGPT 앱에서 단축키 하나로 모델과 추론 수준을 함께 선택합니다.','Select a model and reasoning level together using a shortcut in ChatGPT.')}</p></header>
 {access?.required&&<section aria-labelledby="access-title"><h2 id="access-title">{t('접근성 권한','Accessibility permission')}</h2>
 <p role="status">{access.granted?t('접근성 권한이 허용되었습니다.','Accessibility access is granted.'):t('macOS 권한 요청을 확인하고 시스템 설정에서 GPT Model Switcher를 허용하십시오. 허용 후 자동으로 다시 확인합니다.','Review the macOS request and allow GPT Model Switcher in System Settings. Access is checked again automatically.')}</p>
 {!access.granted&&<><button onClick={()=>api.accessibility(true).then(setAccess).catch(e=>setError(String(e)))}>{t('권한 다시 요청','Request permission again')}</button><button onClick={()=>api.openAccessibilitySettings().catch(e=>setError(String(e)))}>{t('손쉬운 사용 권한 설정 열기','Open Accessibility settings')}</button></>}
 </section>}
 <section aria-labelledby="state-title"><h2 id="state-title">{t('연결 상태','Connection status')}</h2><p>{status?.message||t('확인 중…','Checking…')}</p><button onClick={()=>api.status().then(setStatus).catch(e=>setError(String(e)))}>{t('상태 새로고침','Refresh status')}</button></section>
 <section aria-labelledby="preferences-title"><h2 id="preferences-title">{t('환경 설정','Preferences')}</h2><label>{t('언어','Language')}<select value={settings.language} onChange={e=>setSettings({...settings,language:e.target.value as 'ko'|'en'})}><option value="ko">한국어</option><option value="en">English</option></select></label>
 <label className="check"><input type="checkbox" checked={settings.visual_notifications} onChange={e=>setSettings({...settings,visual_notifications:e.target.checked})}/>{t('화면 알림','Visual notifications')}</label>
 <label className="check"><input type="checkbox" checked={settings.screen_reader_notifications} onChange={e=>setSettings({...settings,screen_reader_notifications:e.target.checked})}/>{t('스크린 리더 안내','Screen reader announcements')}</label><button disabled={testing||!settings.screen_reader_notifications} onClick={async()=>{setTesting(true);setTestInfo('');setError('');try{const result=await api.testAnnouncement();setTestInfo(result.warning||t('시험 안내를 보냈습니다. 실제로 들리는지 확인하십시오.','Test announcement sent. Check that you can hear it.'));}catch(e){setError(String(e));}finally{setTesting(false);}}}>{t('음성 안내 시험','Test voice feedback')}</button><p>{t('설정을 저장한 뒤 시험하십시오. 모델을 바꾸지 않고 짧은 안내를 한 번 보냅니다.','Save settings before testing. Sends one short announcement without changing the model.')}</p><p>{testInfo}</p></section>
 <section id="presets" aria-labelledby="presets-title"><h2 id="presets-title">{t('프리셋','Presets')}</h2><p>{t('앱 모델 목록에 있는 정확한 모델 ID를 입력하십시오. 포커스 없는 연결을 지원하는 로컬 대화에서 사용할 수 있습니다.','Enter an exact model ID from the app catalog. Requires a local conversation supporting the focus-free connection.')}</p>
 {settings.presets.map((p,i)=><fieldset key={p.id}><legend>{i+1}. {p.name||t('새 프리셋','New preset')}</legend><div className="grid">
 <label>{t('이름','Name')}<input aria-label={`${i+1} ${t('이름','Name')}`} value={p.name} onChange={e=>update(p.id,{name:e.target.value})}/></label>
 <label>{t('모델','Model')}<input aria-label={`${i+1} ${t('모델','Model')}`} value={p.model} spellCheck={false} onChange={e=>update(p.id,{model:e.target.value})}/></label>
 <label>{t('추론 수준','Reasoning level')}<select aria-label={`${i+1} ${t('추론 수준','Reasoning level')}`} value={p.effort} onChange={e=>update(p.id,{effort:e.target.value})}>{efforts.map(e=><option key={e}>{e}</option>)}</select></label>
 <label>Windows {t('단축키','shortcut')}<input aria-label={`${i+1} Windows ${t('단축키','shortcut')}`} value={p.windows_shortcut} onChange={e=>update(p.id,{windows_shortcut:e.target.value})}/></label>
 <label>macOS {t('단축키','shortcut')}<input aria-label={`${i+1} macOS ${t('단축키','shortcut')}`} value={p.macos_shortcut} onChange={e=>update(p.id,{macos_shortcut:e.target.value})}/></label>
 </div><button onClick={()=>{setSettings({...settings,presets:settings.presets.filter(x=>x.id!==p.id)});addButton.current?.focus();}}>{t('프리셋 삭제','Delete preset')} {p.name}</button></fieldset>)}
 <button ref={addButton} onClick={()=>setSettings({...settings,presets:[...settings.presets,{id:crypto.randomUUID(),name:t('새 프리셋','New preset'),model:'',effort:'medium',windows_shortcut:'',macos_shortcut:''}]})}>{t('프리셋 추가','Add preset')}</button></section>
 <footer><button className="primary" disabled={saving} onClick={save}>{saving?t('저장 중…','Saving…'):t('설정 저장','Save settings')}</button><p role="status" aria-live="polite">{notice}</p><p role="alert">{error}</p></footer>
 <section aria-labelledby="history-title"><h2 id="history-title">{t('최근 전환 결과','Recent switch results')}</h2><p>{t('이번 실행의 최근 20개 결과입니다. 전환 중 입력 포커스를 옮기지 않습니다.','The latest 20 results from this session. Switching keeps input focus in place.')}</p>{history.length===0?<p>{t('아직 결과가 없습니다.','No results yet.')}</p>:<ol>{history.map((h,i)=><li key={i}><strong>{outcomeTitle(h,settings.language)}</strong>: {h.message}<p>{t('확인된 선택값','Verified selection')}: {h.actual?`${h.actual.model} · ${h.actual.effort}`:t('확인하지 못했습니다.','Could not be verified.')}{h.restored&&` · ${t('원래 설정 복구 확인','Original settings restored')}`}</p></li>)}</ol>}</section>
 <section aria-labelledby="help-title"><h2 id="help-title">{t('사용 안내','How to use')}</h2><p>{t('설정을 저장한 뒤 ChatGPT로 돌아가 단축키를 누르십시오. 응답 생성 중에는 전환하지 않습니다. 클라우드 채팅·동일 제목의 대화·연결 미지원 화면에서는 중단하며 메뉴 조작으로 재시도하지 않습니다.','Save settings, return to ChatGPT, and press the shortcut. Generation, cloud chats, ambiguous titles and unsupported connections block switching; menus are never used as a fallback.')}</p><p>{t('자동 시작은 꺼져 있습니다. 창을 닫아도 트레이 또는 메뉴 막대에서 계속 실행됩니다. 종료 메뉴로 완전히 종료할 수 있습니다.','Autostart is off. Closing the window keeps the app in the tray or menu bar. Choose Quit to exit.')}</p></section>
 </main></>;
}
