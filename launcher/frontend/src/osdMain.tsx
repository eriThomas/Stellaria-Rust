import { StrictMode } from 'react';
import { createRoot } from 'react-dom/client';
import { OsdApp } from './OsdApp';

createRoot(document.getElementById('root')!).render(
  <StrictMode>
    <OsdApp />
  </StrictMode>,
);
