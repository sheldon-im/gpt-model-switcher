import type {Outcome} from './api';
export function outcomeTitle(outcome:Outcome,language:'ko'|'en'){
 if(!outcome.success)return language==='ko'?'전환 실패':'Switch failed';
 if(outcome.already_applied)return language==='ko'?'이미 적용됨':'Already applied';
 return language==='ko'?'전환 완료':'Switched';
}
