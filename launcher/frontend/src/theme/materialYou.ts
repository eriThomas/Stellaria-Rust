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
  return `hsl(${hue} ${Math.max(0, Math.min(100, Math.round(s)))}% ${Math.max(0, Math.min(100, Math.round(l)))}%)`;
}

export function applyMaterialTheme(settings: DashboardSettings): void {
  const root = document.documentElement;
  const [h, s] = rgbToHsl(parseHex(settings.sourceColor));
  const dark = settings.themeMode === 'dark';
  const sat = settings.themeVariant === 'monochrome' ? 0 : Math.max(28, Math.min(82, s));
  const neutralSat = Math.min(12, sat * 0.22);

  const tokens: Record<string, string> = dark
    ? {
        '--m3-primary': hsl(h, sat, 66),
        '--m3-on-primary': hsl(h, sat, 14),
        '--m3-primary-container': hsl(h, sat * 0.55, 22),
        '--m3-on-primary-container': hsl(h, sat, 86),
        '--m3-secondary': hsl(h + 10, sat * 0.52, 64),
        '--m3-on-secondary': hsl(h + 10, sat * 0.5, 16),
        '--m3-secondary-container': hsl(h + 10, sat * 0.35, 21),
        '--m3-on-secondary-container': hsl(h + 10, sat * 0.45, 86),
        '--m3-tertiary': hsl(h - 28, sat * 0.62, 68),
        '--m3-on-tertiary': hsl(h - 28, sat * 0.5, 16),
        '--m3-tertiary-container': hsl(h - 28, sat * 0.4, 24),
        '--m3-on-tertiary-container': hsl(h - 28, sat * 0.55, 88),
        '--m3-error': '#ffb4ab',
        '--m3-on-error': '#690005',
        '--m3-error-container': '#93000a',
        '--m3-on-error-container': '#ffdad6',
        '--m3-surface': hsl(h, neutralSat, 10),
        '--m3-surface-dim': hsl(h, neutralSat, 8),
        '--m3-surface-bright': hsl(h, neutralSat, 24),
        '--m3-surface-container-lowest': hsl(h, neutralSat, 7),
        '--m3-surface-container-low': hsl(h, neutralSat, 12),
        '--m3-surface-container': hsl(h, neutralSat, 14),
        '--m3-surface-container-high': hsl(h, neutralSat, 18),
        '--m3-surface-container-highest': hsl(h, neutralSat, 22),
        '--m3-on-surface': '#e6e1e5',
        '--m3-on-surface-variant': '#c4c7c5',
        '--m3-outline': '#8e918f',
        '--m3-outline-variant': '#444746',
        '--m3-inverse-surface': '#e6e1e5',
        '--m3-inverse-on-surface': '#313033',
        '--m3-inverse-primary': hsl(h, sat, 40),
        '--m3-shadow': '#000000',
        '--m3-scrim': '#000000',
      }
    : {
        '--m3-primary': hsl(h, sat, 36),
        '--m3-on-primary': '#ffffff',
        '--m3-primary-container': hsl(h, sat * 0.75, 88),
        '--m3-on-primary-container': hsl(h, sat, 14),
        '--m3-secondary': hsl(h + 10, sat * 0.48, 38),
        '--m3-on-secondary': '#ffffff',
        '--m3-secondary-container': hsl(h + 10, sat * 0.55, 88),
        '--m3-on-secondary-container': hsl(h + 10, sat * 0.5, 16),
        '--m3-tertiary': hsl(h - 28, sat * 0.55, 38),
        '--m3-on-tertiary': '#ffffff',
        '--m3-tertiary-container': hsl(h - 28, sat * 0.65, 88),
        '--m3-on-tertiary-container': hsl(h - 28, sat * 0.55, 16),
        '--m3-error': '#ba1a1a',
        '--m3-on-error': '#ffffff',
        '--m3-error-container': '#ffdad6',
        '--m3-on-error-container': '#410002',
        '--m3-surface': hsl(h, neutralSat, 97),
        '--m3-surface-dim': hsl(h, neutralSat, 88),
        '--m3-surface-bright': hsl(h, neutralSat, 99),
        '--m3-surface-container-lowest': '#ffffff',
        '--m3-surface-container-low': hsl(h, neutralSat, 95),
        '--m3-surface-container': hsl(h, neutralSat, 92),
        '--m3-surface-container-high': hsl(h, neutralSat, 89),
        '--m3-surface-container-highest': hsl(h, neutralSat, 86),
        '--m3-on-surface': '#1c1b1f',
        '--m3-on-surface-variant': '#444746',
        '--m3-outline': '#747775',
        '--m3-outline-variant': '#c4c7c5',
        '--m3-inverse-surface': '#313033',
        '--m3-inverse-on-surface': '#f4eff4',
        '--m3-inverse-primary': hsl(h, sat, 72),
        '--m3-shadow': '#000000',
        '--m3-scrim': '#000000',
      };

  for (const [name, value] of Object.entries(tokens)) {
    root.style.setProperty(name, value);
  }
  root.dataset.mode = settings.themeMode;
  root.style.setProperty('--border', `${settings.borderSize}px`);
  root.classList.toggle('borderless', settings.borderSize === 0);
  root.classList.toggle('game-mode', settings.gameMode);
  root.classList.toggle('reduced-motion', settings.reducedMotion || settings.gameMode);
}
