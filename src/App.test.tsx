// @vitest-environment jsdom
import React from 'react';import {test,expect,vi,afterEach} from 'vitest';import {render,screen,cleanup} from '@testing-library/react';import userEvent from '@testing-library/user-event';import '@testing-library/jest-dom/vitest';import {App} from './App';import {api,defaults} from './api';
afterEach(()=>{cleanup();vi.restoreAllMocks()});
test('keyboard edits and saves model preset',async()=>{vi.spyOn(api,'load').mockResolvedValue(structuredClone(defaults));vi.spyOn(api,'status').mockResolvedValue({active:false,available:false,message:'Preview'});const save=vi.spyOn(api,'save').mockResolvedValue();render(<App/>);const user=userEvent.setup();const name=await screen.findByLabelText('1 이름');await user.click(name);await user.clear(name);await user.keyboard('일상');await user.click(screen.getByRole('button',{name:'설정 저장'}));expect(save.mock.calls[0][0].presets[0].name).toBe('일상');expect(await screen.findByRole('status')).toHaveTextContent('설정을 저장했습니다.');});
test('corrupt file error cannot overwrite original',async()=>{vi.spyOn(api,'load').mockRejectedValue('Corrupt settings');vi.spyOn(api,'status').mockResolvedValue({active:false,available:false,message:'Preview'});render(<App/>);expect(await screen.findByRole('alert')).toHaveTextContent('Corrupt settings');expect(screen.queryByRole('button',{name:'설정 저장'})).not.toBeInTheDocument();});
test('results preserve settings focus and distinguish unverifiable failures',async()=>{vi.spyOn(api,'load').mockResolvedValue(structuredClone(defaults));vi.spyOn(api,'history').mockResolvedValue([{success:false,message:'Connection unavailable',restored:false}]);render(<App/>);const name=await screen.findByLabelText('1 이름');name.focus();expect(await screen.findByText('Connection unavailable',{exact:false})).toBeInTheDocument();expect(screen.getByText('확인된 선택값: 확인하지 못했습니다.')).toBeInTheDocument();expect(document.activeElement).toBe(name);});
test('voice test requests feedback once without saving or moving focus',async()=>{vi.spyOn(api,'load').mockResolvedValue(structuredClone(defaults));const send=vi.spyOn(api,'testAnnouncement').mockResolvedValue({backend:'nvda-controller'});const save=vi.spyOn(api,'save');render(<App/>);const button=await screen.findByRole('button',{name:'음성 안내 시험'});await userEvent.click(button);expect(send).toHaveBeenCalledTimes(1);expect(save).not.toHaveBeenCalled();expect(document.activeElement).toBe(button);expect(await screen.findByText('시험 안내를 보냈습니다. 실제로 들리는지 확인하십시오.')).not.toHaveAttribute('role','status');});
test('disabled announcements cannot be voice tested',async()=>{vi.spyOn(api,'load').mockResolvedValue({...structuredClone(defaults),screen_reader_notifications:false});const send=vi.spyOn(api,'testAnnouncement');render(<App/>);const button=await screen.findByRole('button',{name:'음성 안내 시험'});expect(button).toBeDisabled();await userEvent.click(button);expect(send).not.toHaveBeenCalled();});
test('already-applied result is distinct in history and keeps editor focus',async()=>{vi.spyOn(api,'load').mockResolvedValue(structuredClone(defaults));vi.spyOn(api,'history').mockResolvedValue([{success:true,already_applied:true,message:'gpt-6.1-sol · high',actual:{model:'gpt-6.1-sol',effort:'high'},restored:false}]);render(<App/>);const name=await screen.findByLabelText('1 이름');name.focus();expect(await screen.findByText('이미 적용됨')).toBeInTheDocument();expect(screen.queryByText('전환 완료')).not.toBeInTheDocument();expect(document.activeElement).toBe(name);});

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
