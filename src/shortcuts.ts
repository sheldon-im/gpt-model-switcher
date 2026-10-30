import type {Settings} from './api';
export function canonicalShortcut(value:string):string{
 const parts=value.split('+').map(x=>x.trim().toLowerCase());
 const key=parts.pop()||'';const aliases:Record<string,string>={ctrl:'control',control:'control',alt:'alt',option:'alt',shift:'shift',super:'super',command:'super',meta:'super'};
 const mods=parts.map(x=>aliases[x]);
 if(!mods.length||mods.some(x=>!x)||new Set(mods).size!==mods.length||!/^([a-z0-9]|f([1-9]|1[0-9]|2[0-4]))$/.test(key))throw new Error('단축키 형식: Control+Alt+Shift+1 / Shortcut format: Control+Alt+Shift+1');
 return [...mods.sort(),key].join('+');
}
export function validateShortcuts(s:Settings){for(const os of ['windows_shortcut','macos_shortcut'] as const){const seen=new Set();for(const p of s.presets){const k=canonicalShortcut(p[os]);if(seen.has(k))throw new Error('중복된 단축키입니다. / Duplicate shortcut.');seen.add(k);}}}
