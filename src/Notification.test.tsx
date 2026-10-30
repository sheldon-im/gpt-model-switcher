// @vitest-environment jsdom
import React from 'react';import {test,expect,vi,afterEach} from 'vitest';import {render,screen,cleanup} from '@testing-library/react';import '@testing-library/jest-dom/vitest';import {Notification} from './Notification';import {api} from './api';
afterEach(()=>{cleanup();vi.restoreAllMocks()});
test('already-applied visual notification is distinct and never a second live announcement',async()=>{vi.spyOn(api,'history').mockResolvedValue([{success:true,already_applied:true,message:'gpt-6.1-sol · high',restored:false}]);render(<Notification/>);const title=await screen.findByText('이미 적용됨 / Already applied');expect(title.closest('.toast')).toHaveAttribute('aria-hidden','true');expect(screen.queryByRole('status')).not.toBeInTheDocument();});
