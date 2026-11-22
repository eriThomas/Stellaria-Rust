import { StrictMode } from 'react';
import { createRoot } from 'react-dom/client';
import { LauncherApp } from './LauncherApp';

const root = document.getElementById('root');
if (root) {
  createRoot(root).render(
    <StrictMode>
      <LauncherApp />
    </StrictMode>,
  );
}
