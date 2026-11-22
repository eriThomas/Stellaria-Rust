import type { DashboardSettings } from '../types/ipc';

interface Rgb {
  r: number;
  g: number;
  b: number;
}

function parseHex(hex: string): Rgb {
  const clean = hex.replace('#', '');
  if (clean.length !== 6) return { r: 126, g: 87, b: 194 };
  const n = Number.parseInt(clean, 16);
  if (Number.isNaN(n)) return { r: 126, g: 87, b: 194 };
  return { r: (n >> 16) & 255, g: (n >> 8) & 255, b: n & 255 };
}

function rgbToHsl({ r, g, b }: Rgb): [number, number, number] {
  const rn = r / 255;
  const gn = g / 255;
  const bn = b / 255;
  const max = Math.max(rn, gn, bn);
  const min = Math.min(rn, gn, bn);
  const l = (max + min) / 2;
  if (max === min) return [0, 0, l * 100];
  const d = max - min;
  const s = l > 0.5 ? d / (2 - max - min) : d / (max + min);
  let h = 0;
  if (max === rn) h = ((gn - bn) / d + (gn < bn ? 6 : 0)) * 60;
  else if (max === gn) h = ((bn - rn) / d + 2) * 60;
  else h = ((rn - gn) / d + 4) * 60;
  return [Math.round(h), Math.round(s * 100), Math.round(l * 100)];
}

function hsl(h: number, s: number, l: number): string {
  const hue = ((h % 360) + 360) % 360;
  return `hsl(${hue} ${Math.max(0, Math.min(100, s))}% ${Math.max(0, Math.min(100, l))}%)`;
}

export function applyMaterialTheme(settings: DashboardSettings): void {
  const root = document.documentElement;
  const [h, s] = rgbToHsl(parseHex(settings.sourceColor));
  const dark = settings.themeMode === 'dark';
  const sat = settings.themeVariant === 'monochrome' ? 0 : Math.max(24, Math.min(82, s));

  const tokens: Record<string, string> = dark
    ? {
        '--m3-primary': hsl(h, sat, 78),
        '--m3-on-primary': hsl(h, sat, 18),
        '--m3-primary-container': hsl(h, sat * 0.7, 28),
        '--m3-on-primary-container': hsl(h, sat, 92),
        '--m3-secondary': hsl(h + 24, sat * 0.65, 76),
        '--m3-tertiary': hsl(h - 36, sat * 0.72, 78),
        '--m3-error': '#f2b8b5',
        '--m3-surface': hsl(h, Math.min(14, sat * 0.25), 9),
        '--m3-surface-container-low': hsl(h, Math.min(14, sat * 0.25), 12),
        '--m3-surface-container': hsl(h, Math.min(16, sat * 0.28), 15),
        '--m3-surface-container-high': hsl(h, Math.min(16, sat * 0.3), 19),
        '--m3-on-surface': '#ece6f0',
        '--m3-on-surface-variant': '#cac4d0',
        '--m3-outline': '#938f99',
        '--m3-outline-variant': '#49454f',
      }
    : {
        '--m3-primary': hsl(h, sat, 40),
        '--m3-on-primary': '#ffffff',
        '--m3-primary-container': hsl(h, sat * 0.85, 90),
        '--m3-on-primary-container': hsl(h, sat, 16),
        '--m3-secondary': hsl(h + 24, sat * 0.6, 42),
        '--m3-tertiary': hsl(h - 36, sat * 0.65, 42),
        '--m3-error': '#b3261e',
        '--m3-surface': hsl(h, Math.min(18, sat * 0.25), 97),
        '--m3-surface-container-low': hsl(h, Math.min(18, sat * 0.25), 94),
        '--m3-surface-container': hsl(h, Math.min(20, sat * 0.28), 91),
        '--m3-surface-container-high': hsl(h, Math.min(20, sat * 0.3), 87),
        '--m3-on-surface': '#1d1b20',
        '--m3-on-surface-variant': '#49454f',
        '--m3-outline': '#79747e',
        '--m3-outline-variant': '#cac4d0',
      };

  for (const [name, value] of Object.entries(tokens)) {
    root.style.setProperty(name, value);
  }
  root.dataset.mode = settings.themeMode;
  root.style.setProperty('--border', `${settings.borderSize}px`);
  root.classList.toggle('reduced-motion', settings.reducedMotion || settings.gameMode);
  root.classList.toggle('game-mode', settings.gameMode);
}
