import { StrictMode } from 'react';
import { createRoot } from 'react-dom/client';
import { SessionApp } from './SessionApp';

const root = document.getElementById('root');
if (root) {
  createRoot(root).render(
    <StrictMode>
      <SessionApp />
    </StrictMode>,
  );
}
