import { invoke } from '@tauri-apps/api/core';
import { useEffect, useState } from 'react';
import { Icon } from './ExpressiveShape';

interface TodoItem {
  id: number;
  text: string;
  done: boolean;
}

type ToolCategory = 'productivity' | 'wellness';

const NOTES_KEY = 'stellaria-shell:tools:notes';
const TODOS_KEY = 'stellaria-shell:tools:todos';
const WATER_KEY = 'stellaria-shell:tools:water';

function loadJson<T>(key: string, fallback: T): T {
  try {
    const raw = localStorage.getItem(key);
    return raw ? (JSON.parse(raw) as T) : fallback;
  } catch {
    return fallback;
  }
}

export function ToolsTab() {
  const [category, setCategory] = useState<ToolCategory>('productivity');
  const [secondsLeft, setSecondsLeft] = useState(25 * 60);
  const [running, setRunning] = useState(false);
  const [notes, setNotes] = useState(() => loadJson<string>(NOTES_KEY, ''));
  const [todos, setTodos] = useState<TodoItem[]>(() => loadJson<TodoItem[]>(TODOS_KEY, []));
  const [todoInput, setTodoInput] = useState('');
  const [waterMl, setWaterMl] = useState(() => loadJson<number>(WATER_KEY, 0));
  const [keepAwake, setKeepAwake] = useState(false);

  useEffect(() => {
    if (!running) return;
    const timer = window.setInterval(() => {
      setSecondsLeft(s => {
        if (s <= 1) {
          setRunning(false);
          return 0;
        }
        return s - 1;
      });
    }, 1000);
    return () => window.clearInterval(timer);
  }, [running]);

  const updateNotes = (value: string) => {
    setNotes(value);
    try {
      localStorage.setItem(NOTES_KEY, JSON.stringify(value));
    } catch {}
  };

  const saveTodos = (next: TodoItem[]) => {
    setTodos(next);
    try {
      localStorage.setItem(TODOS_KEY, JSON.stringify(next));
    } catch {}
  };

  const addTodo = () => {
    const text = todoInput.trim();
    if (!text) return;
    saveTodos([...todos, { id: Date.now(), text, done: false }]);
    setTodoInput('');
  };

  const addWater = (delta: number) => {
    const next = Math.max(0, waterMl + delta);
    setWaterMl(next);
    try {
      localStorage.setItem(WATER_KEY, JSON.stringify(next));
    } catch {}
  };

  const toggleKeepAwake = () => {
    const next = !keepAwake;
    setKeepAwake(next);
    void invoke('shell_set_keep_awake', { enabled: next }).catch(() => {});
  };

  const mm = String(Math.floor(secondsLeft / 60)).padStart(2, '0');
  const ss = String(secondsLeft % 60).padStart(2, '0');

  return (
    <section
      className="page"
      data-page="tools"
      data-testid="tab-panel-tools"
      style={{ width: 920, display: 'flex', flexDirection: 'column', gap: 12 }}
    >
      <div style={{ display: 'flex', gap: 8 }}>
        <button
          type="button"
          className={`icon-button state-layer${category === 'productivity' ? ' checked' : ''}`}
          style={{ padding: '0 14px', display: 'flex', gap: 6, height: 34 }}
          onClick={() => setCategory('productivity')}
        >
          <Icon name="task_alt" />
          <span>Focus & Productivity</span>
        </button>
        <button
          type="button"
          className={`icon-button state-layer${category === 'wellness' ? ' checked' : ''}`}
          style={{ padding: '0 14px', display: 'flex', gap: 6, height: 34 }}
          onClick={() => setCategory('wellness')}
        >
          <Icon name="self_improvement" />
          <span>System & Wellness</span>
        </button>
      </div>

      {category === 'productivity' ? (
        <div style={{ display: 'grid', gridTemplateColumns: '240px 1fr 1fr', gap: 12 }}>
          <article className="card" style={{ borderRadius: 24, padding: 14, display: 'flex', flexDirection: 'column', gap: 10 }}>
            <div style={{ display: 'flex', alignItems: 'center', gap: 6, color: 'var(--m3-primary)', fontWeight: 600 }}>
              <Icon name="timer" />
              <span>Pomodoro</span>
            </div>
            <div style={{ fontSize: 36, fontWeight: 700, textAlign: 'center', margin: '12px 0' }}>
              {mm}:{ss}
            </div>
            <div style={{ display: 'flex', justifyContent: 'center', gap: 6 }}>
              <button type="button" className="icon-button state-layer fill" onClick={() => setRunning(r => !r)}>
                <Icon name={running ? 'pause' : 'play_arrow'} filled />
              </button>
              <button
                type="button"
                className="icon-button state-layer"
                onClick={() => {
                  setRunning(false);
                  setSecondsLeft(25 * 60);
                }}
              >
                <Icon name="restart_alt" />
              </button>
            </div>
          </article>

          <article className="card" style={{ borderRadius: 24, padding: 14, display: 'flex', flexDirection: 'column', gap: 8 }}>
            <div style={{ display: 'flex', alignItems: 'center', gap: 6, color: 'var(--m3-secondary)', fontWeight: 600 }}>
              <Icon name="edit_note" />
              <span>Scratchpad</span>
            </div>
            <textarea
              value={notes}
              onChange={e => updateNotes(e.target.value)}
              placeholder="Quick notes…"
              style={{
                flex: 1,
                minHeight: 210,
                background: 'var(--m3-surface-container-low)',
                color: 'var(--m3-on-surface)',
                border: 'none',
                borderRadius: 14,
                padding: 10,
                resize: 'none',
                fontFamily: 'inherit',
                fontSize: 13,
                outline: 'none',
              }}
            />
          </article>

          <article className="card" style={{ borderRadius: 24, padding: 14, display: 'flex', flexDirection: 'column', gap: 8 }}>
            <div style={{ display: 'flex', alignItems: 'center', gap: 6, color: 'var(--m3-primary)', fontWeight: 600 }}>
              <Icon name="checklist" />
              <span>To-Do</span>
            </div>
            <div style={{ display: 'flex', gap: 6 }}>
              <input
                value={todoInput}
                onChange={e => setTodoInput(e.target.value)}
                onKeyDown={e => e.key === 'Enter' && addTodo()}
                placeholder="Add task…"
                style={{
                  flex: 1,
                  background: 'var(--m3-surface-container-low)',
                  borderRadius: 12,
                  padding: '8px 10px',
                  border: 'none',
                  color: 'var(--m3-on-surface)',
                }}
              />
              <button type="button" className="icon-button state-layer" onClick={addTodo}>
                <Icon name="add" />
              </button>
            </div>
            <div style={{ flex: 1, overflowY: 'auto', display: 'flex', flexDirection: 'column', gap: 6, maxHeight: 180 }}>
              {todos.map(item => (
                <div key={item.id} style={{ display: 'flex', alignItems: 'center', gap: 6, fontSize: 13 }}>
                  <input
                    type="checkbox"
                    checked={item.done}
                    onChange={() =>
                      saveTodos(todos.map(t => (t.id === item.id ? { ...t, done: !t.done } : t)))
                    }
                  />
                  <span style={{ flex: 1, textDecoration: item.done ? 'line-through' : 'none', opacity: item.done ? 0.55 : 1 }}>
                    {item.text}
                  </span>
                  <button type="button" onClick={() => saveTodos(todos.filter(t => t.id !== item.id))}>
                    <Icon name="close" />
                  </button>
                </div>
              ))}
            </div>
          </article>
        </div>
      ) : (
        <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr', gap: 12 }}>
          <article className="card" style={{ borderRadius: 24, padding: 16, display: 'flex', flexDirection: 'column', gap: 10 }}>
            <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
              <span style={{ display: 'flex', alignItems: 'center', gap: 6, color: 'var(--m3-tertiary)', fontWeight: 600 }}>
                <Icon name="water_drop" /> Water Tracker
              </span>
              <strong>{waterMl} / 2000 ml</strong>
            </div>
            <div style={{ display: 'flex', gap: 8 }}>
              <button type="button" className="icon-button state-layer fill" onClick={() => addWater(250)}>
                +250 ml
              </button>
              <button type="button" className="icon-button state-layer" onClick={() => addWater(-waterMl)}>
                <Icon name="restart_alt" />
              </button>
            </div>
          </article>

          <article className="card" style={{ borderRadius: 24, padding: 16, display: 'flex', flexDirection: 'column', gap: 10 }}>
            <div style={{ display: 'flex', alignItems: 'center', gap: 6, color: 'var(--m3-primary)', fontWeight: 600 }}>
              <Icon name="coffee" />
              <span>Keep Awake</span>
            </div>
            <p style={{ margin: 0, fontSize: 12.5, opacity: 0.78 }}>
              Prevents display sleep while active using native Win32 power requests.
            </p>
            <button
              type="button"
              className={`icon-button state-layer${keepAwake ? ' checked' : ''}`}
              onClick={toggleKeepAwake}
              style={{ width: '100%', padding: '0 14px', display: 'flex', gap: 6, justifyContent: 'center' }}
            >
              <Icon name="coffee" />
              <span>{keepAwake ? 'Keep Awake: ON' : 'Keep Awake: OFF'}</span>
            </button>
          </article>
        </div>
      )}
    </section>
  );
}
