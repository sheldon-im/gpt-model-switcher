import {useEffect,useState} from 'react';
import {api,Outcome} from './api';
export function Notification(){
 const [outcome,setOutcome]=useState<Outcome>();
 useEffect(()=>{let disposed=false;let stop:(()=>void)|undefined;api.history().then(h=>{if(!disposed)setOutcome(h[0]);});api.subscribe(setOutcome).then(u=>{if(disposed)u();else stop=u;});return()=>{disposed=true;stop?.();};},[]);
 // Native accessibility announcements handle speech once. This visual window never takes focus.
 return <div className="toast" aria-hidden="true"><strong>{outcome?.success?'전환 완료 / Switched':'전환 결과 / Switch result'}</strong><p>{outcome?.message}</p>{outcome&&!outcome.success&&<small>{outcome.actual?`${outcome.actual.model} · ${outcome.actual.effort}`:'선택값 확인 불가 / Selection unverified'}</small>}</div>;
}
