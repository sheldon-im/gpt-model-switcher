import {useEffect,useRef,useState} from 'react';
import {api,Settings,Preset,Status} from './api';
import {validateShortcuts} from './shortcuts';
const efforts=['none','minimal','low','medium','high','xhigh','max','ultra'];
export function App(){
 const [settings,setSettings]=useState<Settings>();const [status,setStatus]=useState<Status>();
 const [error,setError]=useState('');const [notice,setNotice]=useState('');const [saving,setSaving]=useState(false);
 const addButton=useRef<HTMLButtonElement>(null);const ko=settings?.language!=='en';const t=(a:string,b:string)=>ko?a:b;
 useEffect(()=>{api.load().then(setSettings).catch(e=>setError(String(e)));api.status().then(setStatus).catch(e=>setError(String(e)));},[]);
 useEffect(()=>{document.documentElement.lang=ko?'ko':'en';},[ko]);
 const update=(id:string,patch:Partial<Preset>)=>setSettings(s=>s&&({...s,presets:s.presets.map(p=>p.id===id?{...p,...patch}:p)}));
 async function save(){if(!settings)return;setSaving(true);setError('');setNotice('');try{if(settings.presets.some(p=>!p.name.trim()||!p.model.trim()))throw new Error(t('이름과 모델을 입력하십시오.','Enter a name and model.'));validateShortcuts(settings);await api.save(settings);setNotice(t('설정을 저장했습니다.','Settings saved.'));}catch(e){setError(String(e));}finally{setSaving(false);}}
 if(!settings)return <main><h1>GPT Model Switcher</h1><p role={error?'alert':'status'}>{error||'설정을 불러오는 중입니다. / Loading settings…'}</p></main>;
 return <><a className="skip" href="#presets">{t('프리셋으로 바로 가기','Skip to presets')}</a><main>
 <header><p className="eyebrow">GPT MODEL SWITCHER</p><h1>{t('모델 전환 설정','Model switching settings')}</h1><p>{t('ChatGPT 앱에서 단축키 하나로 모델과 추론 수준을 함께 선택합니다.','Select a model and reasoning level together using a shortcut in ChatGPT.')}</p></header>
 <section aria-labelledby="state-title"><h2 id="state-title">{t('연결 상태','Connection status')}</h2><p>{status?.message||t('확인 중…','Checking…')}</p><button onClick={()=>api.status().then(setStatus).catch(e=>setError(String(e)))}>{t('상태 새로고침','Refresh status')}</button></section>
 <section aria-labelledby="preferences-title"><h2 id="preferences-title">{t('환경 설정','Preferences')}</h2><label>{t('언어','Language')}<select value={settings.language} onChange={e=>setSettings({...settings,language:e.target.value as 'ko'|'en'})}><option value="ko">한국어</option><option value="en">English</option></select></label>
 <label className="check"><input type="checkbox" checked={settings.visual_notifications} onChange={e=>setSettings({...settings,visual_notifications:e.target.checked})}/>{t('화면 알림','Visual notifications')}</label>
 <label className="check"><input type="checkbox" checked={settings.screen_reader_notifications} onChange={e=>setSettings({...settings,screen_reader_notifications:e.target.checked})}/>{t('스크린 리더 안내','Screen reader announcements')}</label></section>
 <section id="presets" aria-labelledby="presets-title"><h2 id="presets-title">{t('프리셋','Presets')}</h2><p>{t('해당 화면에서 선택할 수 있는 실제 모델 ID를 입력하십시오.','Enter the actual ID of a model available in the current view.')}</p>
 {settings.presets.map((p,i)=><fieldset key={p.id}><legend>{i+1}. {p.name||t('새 프리셋','New preset')}</legend><div className="grid">
 <label>{t('이름','Name')}<input aria-label={`${i+1} ${t('이름','Name')}`} value={p.name} onChange={e=>update(p.id,{name:e.target.value})}/></label>
 <label>{t('모델','Model')}<input aria-label={`${i+1} ${t('모델','Model')}`} value={p.model} spellCheck={false} onChange={e=>update(p.id,{model:e.target.value})}/></label>
 <label>{t('추론 수준','Reasoning level')}<select aria-label={`${i+1} ${t('추론 수준','Reasoning level')}`} value={p.effort} onChange={e=>update(p.id,{effort:e.target.value})}>{efforts.map(e=><option key={e}>{e}</option>)}</select></label>
 <label>Windows {t('단축키','shortcut')}<input aria-label={`${i+1} Windows ${t('단축키','shortcut')}`} value={p.windows_shortcut} onChange={e=>update(p.id,{windows_shortcut:e.target.value})}/></label>
 <label>macOS {t('단축키','shortcut')}<input aria-label={`${i+1} macOS ${t('단축키','shortcut')}`} value={p.macos_shortcut} onChange={e=>update(p.id,{macos_shortcut:e.target.value})}/></label>
 </div><button onClick={()=>{setSettings({...settings,presets:settings.presets.filter(x=>x.id!==p.id)});addButton.current?.focus();}}>{t('프리셋 삭제','Delete preset')} {p.name}</button></fieldset>)}
 <button ref={addButton} onClick={()=>setSettings({...settings,presets:[...settings.presets,{id:crypto.randomUUID(),name:t('새 프리셋','New preset'),model:'',effort:'medium',windows_shortcut:'',macos_shortcut:''}]})}>{t('프리셋 추가','Add preset')}</button></section>
 <footer><button className="primary" disabled={saving} onClick={save}>{saving?t('저장 중…','Saving…'):t('설정 저장','Save settings')}</button><p role="status" aria-live="polite">{notice}</p><p role="alert">{error}</p></footer>
 <section aria-labelledby="help-title"><h2 id="help-title">{t('사용 안내','How to use')}</h2><p>{t('설정을 저장한 뒤 ChatGPT로 돌아가 단축키를 누르십시오. 응답 생성 중에는 전환하지 않습니다.','Save settings, return to ChatGPT, and press the shortcut. Switching is blocked during generation.')}</p><p>{t('자동 시작은 꺼져 있습니다. 창을 닫아도 트레이 또는 메뉴 막대에서 계속 실행됩니다. 종료 메뉴로 완전히 종료할 수 있습니다.','Autostart is off. Closing the window keeps the app in the tray or menu bar. Choose Quit to exit.')}</p></section>
 </main></>;
}
