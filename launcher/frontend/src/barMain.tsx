import { StrictMode } from 'react';
import { createRoot } from 'react-dom/client';
import { BarApp } from './BarApp';

const params = new URLSearchParams(window.location.search);
const monitorLabel = params.get('monitor') ?? 'bar-primary';

const root = document.getElementById('root');
if (root) {
  createRoot(root).render(
    <StrictMode>
      <BarApp monitorLabel={monitorLabel} />
    </StrictMode>,
  );
}
