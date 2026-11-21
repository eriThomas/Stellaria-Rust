import {
  Hct,
  MaterialDynamicColors,
  SchemeContent,
  SchemeExpressive,
  SchemeFidelity,
  SchemeFruitSalad,
  SchemeMonochrome,
  SchemeNeutral,
  SchemeRainbow,
  SchemeTonalSpot,
  SchemeVibrant,
  argbFromHex,
  argbFromRgb,
  hexFromArgb,
  sourceColorFromImage,
} from '../../vendor/material-color-utilities.mjs';
import { getOption, keys, option, setOption, settings, store } from './store.js';
import { t, translateDocument } from './i18n.js';
import { clampBorderSize } from './customizer.js';

export const variants = {
  tonalspot: { label: 'Tonal Spot', scheme: SchemeTonalSpot },
  vibrant: { label: 'Vibrant', scheme: SchemeVibrant },
  expressive: { label: t('Expressif'), scheme: SchemeExpressive },
  fidelity: { label: t('Fidélité'), scheme: SchemeFidelity },
  content: { label: t('Contenu'), scheme: SchemeContent },
  fruitsalad: { label: t('Salade de fruits'), scheme: SchemeFruitSalad },
  rainbow: { label: t('Arc-en-ciel'), scheme: SchemeRainbow },
  neutral: { label: t('Neutre'), scheme: SchemeNeutral },
  monochrome: { label: 'Monochrome', scheme: SchemeMonochrome },
};

const roleNames = [
  'primary', 'onPrimary', 'primaryContainer', 'onPrimaryContainer',
  'secondary', 'onSecondary', 'secondaryContainer', 'onSecondaryContainer',
  'tertiary', 'onTertiary', 'tertiaryContainer', 'onTertiaryContainer',
  'error', 'onError', 'errorContainer', 'onErrorContainer',
  'surface', 'surfaceDim', 'surfaceBright',
  'surfaceContainerLowest', 'surfaceContainerLow', 'surfaceContainer',
  'surfaceContainerHigh', 'surfaceContainerHighest',
  'onSurface', 'onSurfaceVariant', 'outline', 'outlineVariant',
  'inverseSurface', 'inverseOnSurface', 'inversePrimary', 'shadow', 'scrim',
];

const kebab = name => name.replace(/[A-Z]/g, letter => `-${letter.toLowerCase()}`);

let schemeBundle = null;
const schemeBundleReady = fetch(new URL('../../schemes.json', import.meta.url))
  .then(response => (response.ok ? response.json() : null))
  .then(bundle => {
    schemeBundle = bundle;
    return bundle;
  })
  .catch(() => null);

export function loadSchemes() {
  return schemeBundleReady;
}

function staticScheme(selection = store.get(keys.scheme, null), dark = store.get(keys.mode, 'dark') !== 'light') {
  const flavour = selection && schemeBundle?.[selection.name]?.[selection.flavour];
  if (!flavour) return null;
  const preferred = dark ? 'dark' : 'light';
  const mode = flavour[preferred] ? preferred : Object.keys(flavour)[0];
  return { mode, roles: flavour[mode] };
}

function activeVariant() {
  if (getOption('smartScheme', true) && store.get(keys.achromatic, false)) return variants.monochrome;
  return variants[store.get(keys.variant, 'tonalspot')] ?? variants.tonalspot;
}

export function applyTheme() {
  const style = document.documentElement.style;
  const fixed = staticScheme();
  if (fixed) {
    for (const role of roleNames) {
      if (fixed.roles[role]) style.setProperty(`--m3-${kebab(role)}`, fixed.roles[role]);
    }
    document.documentElement.dataset.mode = fixed.mode;
    return;
  }

  const source = store.get(keys.source, argbFromHex(settings.defaultSource));
  const dark = store.get(keys.mode, 'dark') !== 'light';
  const scheme = new (activeVariant().scheme)(Hct.fromInt(source), dark, 0);

  for (const role of roleNames) {
    const color = MaterialDynamicColors[role];
    if (color) style.setProperty(`--m3-${kebab(role)}`, hexFromArgb(color.getArgb(scheme)));
  }
  document.documentElement.dataset.mode = dark ? 'dark' : 'light';
}
export function windowsPalette(primaryHex) {
  const primary = Hct.fromInt(argbFromHex(primaryHex));
  const chroma = Math.max(primary.chroma, 36);
  return [90, 76, 62, 48, 40, 28, 16].map(tone => hexFromArgb(Hct.from(primary.hue, chroma, tone).toInt()));
}

const requiredFonts = [['24px "Material Symbols Rounded"', 'home'], ['16px "Google Sans Flex"', 'A']];
const fontRetryKey = 'stellaria:font-retry';

async function ensureFonts() {
  try {
    await document.fonts.ready;
    const results = await Promise.allSettled(requiredFonts.map(([font, text]) => document.fonts.load(font, text)));
    const failed = results.some(result => result.status === 'rejected' || result.value.length === 0 || result.value.some(face => face.status !== 'loaded'));
    if (!failed) {
      sessionStorage.removeItem(fontRetryKey);
      return;
    }
    const attempt = Number(sessionStorage.getItem(fontRetryKey) ?? '0') + 1;
    if (attempt > 3) return;
    sessionStorage.setItem(fontRetryKey, String(attempt));
    setTimeout(() => location.reload(), 1500 * attempt);
  } catch (error) {
    console.error('[stellaria] Polices', error);
  }
}

export function initTheme() {
  if (document.body) translateDocument();
  else document.addEventListener('DOMContentLoaded', () => translateDocument(), { once: true });
  if (document.readyState === 'complete') ensureFonts();
  else window.addEventListener('load', () => ensureFonts(), { once: true });
  applyTheme();
  schemeBundleReady.then(applyTheme);
  for (const key of [keys.source, keys.mode, keys.variant, keys.scheme, keys.achromatic, 'stellaria:option:smartScheme']) store.watch(key, applyTheme);
  if (getOption('transparency', false)) setOption('transparency', false);
  option('gameMode', false, enabled => document.documentElement.classList.toggle('game-mode', Boolean(enabled)));
  option('borderSize', 0, value => {
    const size = clampBorderSize(value);
    document.documentElement.style.setProperty('--border', `${size}px`);
    document.documentElement.classList.toggle('borderless', size === 0);
  });
  installIconTint();
  option('tintIcons', true, enabled => document.documentElement.classList.toggle('tint-icons', Boolean(enabled)));
}

function installIconTint() {
  if (document.getElementById('stellaria-tint')) return;
  const ns = 'http://www.w3.org/2000/svg';
  const make = (tag, attributes) => {
    const node = document.createElementNS(ns, tag);
    for (const [name, value] of Object.entries(attributes)) node.setAttribute(name, value);
    return node;
  };
  const host = make('svg', { width: '0', height: '0', 'aria-hidden': 'true', style: 'position:absolute' });
  const filter = make('filter', { id: 'stellaria-tint', 'color-interpolation-filters': 'sRGB' });
  filter.append(
    make('feColorMatrix', { type: 'saturate', values: '0', result: 'gray' }),
    make('feFlood', { class: 'tint-flood', result: 'color' }),
    make('feBlend', { in: 'color', in2: 'gray', mode: 'color', result: 'tinted' }),
    make('feComposite', { in: 'tinted', in2: 'SourceAlpha', operator: 'in' }),
  );
  host.append(filter);
  const attach = () => document.body.append(host);
  if (document.body) attach();
  else document.addEventListener('DOMContentLoaded', attach, { once: true });
}

export async function analyzeImage(base64) {
  const image = new Image();
  image.src = `data:image/png;base64,${base64}`;
  await image.decode();
  const canvas = document.createElement('canvas');
  canvas.width = Math.min(160, image.width);
  canvas.height = Math.max(1, Math.round(image.height * canvas.width / image.width));
  const context = canvas.getContext('2d', { willReadFrequently: true });
  context.drawImage(image, 0, 0, canvas.width, canvas.height);
  const pixels = context.getImageData(0, 0, canvas.width, canvas.height).data;
  let colourful = 0;
  for (let i = 0; i < pixels.length; i += 4) {
    if (Hct.fromInt(argbFromRgb(pixels[i], pixels[i + 1], pixels[i + 2])).chroma > 16) colourful++;
  }
  return {
    source: await sourceColorFromImage(image),
    achromatic: colourful / (pixels.length / 4) < 0.01,
  };
}
