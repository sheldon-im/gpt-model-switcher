// @vitest-environment jsdom
import React from 'react';import {test,expect,vi,afterEach} from 'vitest';import {render,screen,cleanup} from '@testing-library/react';import userEvent from '@testing-library/user-event';import '@testing-library/jest-dom/vitest';import {App} from './App';import {api,defaults} from './api';
afterEach(()=>{cleanup();vi.restoreAllMocks()});
test('keyboard edits and saves model preset',async()=>{vi.spyOn(api,'load').mockResolvedValue(structuredClone(defaults));vi.spyOn(api,'status').mockResolvedValue({active:false,available:false,message:'Preview'});const save=vi.spyOn(api,'save').mockResolvedValue();render(<App/>);const user=userEvent.setup();const name=await screen.findByLabelText('1 이름');await user.click(name);await user.clear(name);await user.keyboard('일상');await user.click(screen.getByRole('button',{name:'설정 저장'}));expect(save.mock.calls[0][0].presets[0].name).toBe('일상');expect(await screen.findByRole('status')).toHaveTextContent('설정을 저장했습니다.');});
test('corrupt file error cannot overwrite original',async()=>{vi.spyOn(api,'load').mockRejectedValue('Corrupt settings');vi.spyOn(api,'status').mockResolvedValue({active:false,available:false,message:'Preview'});render(<App/>);expect(await screen.findByRole('alert')).toHaveTextContent('Corrupt settings');expect(screen.queryByRole('button',{name:'설정 저장'})).not.toBeInTheDocument();});
test('results preserve settings focus and distinguish unverifiable failures',async()=>{vi.spyOn(api,'load').mockResolvedValue(structuredClone(defaults));vi.spyOn(api,'history').mockResolvedValue([{success:false,message:'Connection unavailable',restored:false}]);render(<App/>);const name=await screen.findByLabelText('1 이름');name.focus();expect(await screen.findByText('Connection unavailable',{exact:false})).toBeInTheDocument();expect(screen.getByText('확인된 선택값: 확인하지 못했습니다.')).toBeInTheDocument();expect(document.activeElement).toBe(name);});
test('permission controls request OS approval and open settings without saving presets',async()=>{
 vi.spyOn(api,'load').mockResolvedValue(structuredClone(defaults));vi.spyOn(api,'status').mockResolvedValue({active:false,available:false,message:'Permission required'});
 const access=vi.spyOn(api,'accessibility').mockResolvedValue({required:true,granted:false});
 const open=vi.spyOn(api,'openAccessibilitySettings').mockResolvedValue();const save=vi.spyOn(api,'save').mockResolvedValue();
 render(<App/>);const user=userEvent.setup();
 const request=await screen.findByRole('button',{name:'권한 다시 요청'});
 await user.click(request);expect(access).toHaveBeenCalledWith(true);
 await user.click(screen.getByRole('button',{name:'손쉬운 사용 권한 설정 열기'}));expect(open).toHaveBeenCalledOnce();expect(save).not.toHaveBeenCalled();
});
test('returning after permission approval refreshes access without moving edit focus',async()=>{
 vi.spyOn(api,'load').mockResolvedValue(structuredClone(defaults));vi.spyOn(api,'status').mockResolvedValue({active:false,available:false,message:'Ready'});
 const access=vi.spyOn(api,'accessibility').mockResolvedValue({required:true,granted:false});render(<App/>);
 await screen.findByRole('button',{name:'권한 다시 요청'});const name=screen.getByLabelText('1 이름');name.focus();
 access.mockResolvedValue({required:true,granted:true});window.dispatchEvent(new Event('focus'));
 expect(await screen.findByText('접근성 권한이 허용되었습니다.')).toBeInTheDocument();expect(screen.queryByRole('button',{name:'권한 다시 요청'})).not.toBeInTheDocument();expect(document.activeElement).toBe(name);
});
