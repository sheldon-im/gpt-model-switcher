import {expect,test} from 'vitest';import {canonicalShortcut,validateShortcuts} from './shortcuts';import {defaults} from './api';
test('detects reordered modifier aliases across presets',()=>{const s=structuredClone(defaults);s.presets[1].windows_shortcut='shift+ctrl+option+1';expect(()=>validateShortcuts(s)).toThrow('Duplicate');expect(canonicalShortcut('Command+Shift+A')).toBe('shift+super+a');});
test('rejects ambiguous or nonfunctional shortcut input',()=>{for(const s of ['A','Ctrl+Control+A','Ctrl+','Unknown+A','Alt+F25'])expect(()=>canonicalShortcut(s)).toThrow();});
