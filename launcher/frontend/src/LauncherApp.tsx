import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { useEffect, useMemo, useRef, useState } from 'react';
import { Icon } from './components/ExpressiveShape';

interface CatalogApp {
  name: string;
  path: string;
}

function tryEvalSimpleMath(input: string): string | null {
  const cleaned = input.trim();
  if (!/^[0-9+\-*/().\s]+$/.test(cleaned) || !/[+\-*/]/.test(cleaned)) return null;
  try {
    // Safe arithmetic-only evaluation (regex above strictly forbids identifiers/calls)
    const val = Function(`"use strict"; return (${cleaned});`)();
    if (typeof val === 'number' && Number.isFinite(val)) {
      return String(Math.round(val * 10000) / 10000);
    }
    return null;
  } catch {
    return null;
  }
}

export function LauncherApp() {
  const [query, setQuery] = useState('');
  const [apps, setApps] = useState<CatalogApp[]>([]);
  const [selected, setSelected] = useState(0);
  const inputRef = useRef<HTMLInputElement | null>(null);

  useEffect(() => {
    invoke<CatalogApp[]>('launcher_get_apps').then(setApps).catch(() => {});
    const unlistenPromise = listen('launcher://opened', () => {
      setQuery('');
      setSelected(0);
      inputRef.current?.focus();
    });
    inputRef.current?.focus();
    return () => {
      void unlistenPromise.then(fn => fn());
    };
  }, []);

  const mathResult = useMemo(() => tryEvalSimpleMath(query), [query]);

  const filtered = useMemo(() => {
    const q = query.trim().toLowerCase();
    if (!q) return apps.slice(0, 14);
    return apps.filter(a => a.name.toLowerCase().includes(q)).slice(0, 14);
  }, [apps, query]);

  const launch = (app: CatalogApp) => {
    void invoke('taskbar_launch_app', { path: app.path });
    void invoke('launcher_hide');
  };

  return (
    <main
      style={{
        width: '100vw',
        height: '100vh',
        background: 'rgba(20, 18, 24, 0.96)',
        borderRadius: 24,
        border: '1px solid rgba(255, 255, 255, 0.12)',
        color: '#e6e0e9',
        display: 'flex',
        flexDirection: 'column',
        padding: 16,
        gap: 12,
        boxSizing: 'border-box',
      }}
      onKeyDown={e => {
        if (e.key === 'Escape') {
          void invoke('launcher_hide');
        } else if (e.key === 'ArrowDown') {
          e.preventDefault();
          setSelected(i => Math.min(Math.max(0, filtered.length - 1), i + 1));
        } else if (e.key === 'ArrowUp') {
          e.preventDefault();
          setSelected(i => Math.max(0, i - 1));
        } else if (e.key === 'Enter' && filtered[selected]) {
          launch(filtered[selected]);
        }
      }}
    >
      <div
        style={{
          display: 'flex',
          alignItems: 'center',
          gap: 10,
          background: '#2b2930',
          borderRadius: 16,
          padding: '10px 14px',
        }}
      >
        <Icon name="search" />
        <input
          ref={inputRef}
          value={query}
          onChange={e => {
            setQuery(e.target.value);
            setSelected(0);
          }}
          placeholder="Search apps or calculate (e.g. 1920 / 1.25)…"
          style={{
            flex: 1,
            background: 'transparent',
            border: 'none',
            color: '#e6e0e9',
            fontSize: 15,
            outline: 'none',
          }}
        />
      </div>

      {mathResult !== null && (
        <div
          style={{
            padding: '10px 14px',
            borderRadius: 14,
            background: 'rgba(208, 188, 255, 0.16)',
            color: '#d0bcff',
            fontWeight: 600,
            fontSize: 16,
          }}
        >
          = {mathResult}
        </div>
      )}

      <div style={{ flex: 1, overflowY: 'auto', display: 'flex', flexDirection: 'column', gap: 4 }}>
        {filtered.map((app, index) => (
          <button
            key={app.path}
            type="button"
            onClick={() => launch(app)}
            style={{
              display: 'flex',
              alignItems: 'center',
              gap: 10,
              padding: '10px 12px',
              borderRadius: 12,
              textAlign: 'left',
              background: index === selected ? 'rgba(208, 188, 255, 0.2)' : 'transparent',
              color: '#e6e0e9',
            }}
          >
            <Icon name="apps" />
            <span>{app.name}</span>
          </button>
        ))}
      </div>
    </main>
  );
}
