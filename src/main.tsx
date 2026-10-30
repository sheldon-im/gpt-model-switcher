import React from 'react';import {createRoot} from 'react-dom/client';import {App} from './App';import './style.css';
import {Notification} from './Notification';import './notification.css';
createRoot(document.getElementById('root')!).render(<React.StrictMode>{location.hash==='#notification'?<Notification/>:<App/>}</React.StrictMode>);
