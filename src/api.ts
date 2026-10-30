import {invoke,isTauri} from '@tauri-apps/api/core';
import {listen} from '@tauri-apps/api/event';
export interface Preset {id:string;name:string;model:string;effort:string;windows_shortcut:string;macos_shortcut:string}
export interface Settings {version:number;language:'ko'|'en';visual_notifications:boolean;screen_reader_notifications:boolean;presets:Preset[]}
export interface Status {active:boolean;available:boolean;message:string;selection?:{model:string;effort:string};busy?:boolean}
export interface Outcome {success:boolean;already_applied?:boolean;message:string;actual?:{model:string;effort:string};restored:boolean}
export const defaults:Settings={version:1,language:'ko',visual_notifications:true,screen_reader_notifications:true,presets:[{id:'daily',name:'일상용',model:'gpt-6.1-terra',effort:'medium',windows_shortcut:'Control+Alt+Shift+1',macos_shortcut:'Super+Alt+Shift+1'},{id:'coding',name:'코딩용',model:'gpt-6.1-sol',effort:'high',windows_shortcut:'Control+Alt+Shift+2',macos_shortcut:'Super+Alt+Shift+2'}]};
// Browser preview never controls another app.
export const api={
load:async():Promise<Settings>=>isTauri()?invoke('get_settings'):JSON.parse(localStorage.getItem('gpt-switcher-preview')||JSON.stringify(defaults)),
save:async(settings:Settings):Promise<void>=>{if(isTauri())return invoke('save_settings',{settings});localStorage.setItem('gpt-switcher-preview',JSON.stringify(settings));},
status:async():Promise<Status>=>isTauri()?invoke('get_status'):{active:false,available:false,message:'Browser preview · 실제 모델 전환은 데스크톱 앱에서 가능합니다.'},
history:async():Promise<Outcome[]>=>isTauri()?invoke('get_history'):[],
testAnnouncement:async():Promise<{backend:string;warning?:string}>=>{if(!isTauri())throw new Error('안내 시험은 데스크톱 앱에서 실행하십시오. / Run the announcement test in the desktop app.');return invoke('test_announcement');},
subscribe:async(callback:(outcome:Outcome)=>void):Promise<()=>void>=>isTauri()?listen<Outcome>('switch-finished',event=>callback(event.payload)):()=>{}
};
