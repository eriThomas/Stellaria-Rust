export const languages = {
  fr: { name: 'Français', locale: 'fr-FR' },
  en: { name: 'English', locale: 'en-GB' },
  es: { name: 'Español', locale: 'es-ES' },
  de: { name: 'Deutsch', locale: 'de-DE' },
  'pt-BR': { name: 'Português (Brasil)', locale: 'pt-BR' },
  it: { name: 'Italiano', locale: 'it-IT' },
  pl: { name: 'Polski', locale: 'pl-PL' },
  ru: { name: 'Русский', locale: 'ru-RU' },
  tr: { name: 'Türkçe', locale: 'tr-TR' },
  'zh-CN': { name: '简体中文', locale: 'zh-CN' },
  ja: { name: '日本語', locale: 'ja-JP' },
  ko: { name: '한국어', locale: 'ko-KR' },
};

const systemTags = () => [...(navigator.languages ?? []), navigator.language].filter(Boolean);

export function languageForTag(tag) {
  const lower = String(tag ?? '').toLowerCase();
  if (lower.startsWith('pt')) return 'pt-BR';
  if (lower.startsWith('zh')) return /^zh-(hant|tw|hk|mo)/.test(lower) ? null : 'zh-CN';
  const base = lower.split('-')[0];
  return base in languages ? base : null;
}

function systemLanguage() {
  for (const tag of systemTags()) {
    const match = languageForTag(tag);
    if (match) return match;
  }
  return 'en';
}

function chosenLanguage() {
  try {
    const value = JSON.parse(localStorage.getItem('stellaria:option:language') ?? 'null');
    return value in languages ? value : 'auto';
  } catch {
    return 'auto';
  }
}

export const language = chosenLanguage() === 'auto' ? systemLanguage() : chosenLanguage();

function localeFor(lang) {
  const system = systemTags().find(tag => languageForTag(tag) === lang);
  return system ?? languages[lang].locale;
}

export const settings = {
  locale: localeFor(language),
  defaultSource: '#7e57c2',
};

export const keys = {
  source: 'stellaria:source',
  mode: 'stellaria:mode',
  variant: 'stellaria:variant',
  scheme: 'stellaria:scheme',
  toast: 'stellaria:toast',
  autoMode: 'stellaria:auto-mode',
  dashboardTab: 'stellaria:dashboard-tab',
  idle: 'stellaria:idle',
  wallpaperSample: 'stellaria:wallpaper:sample',
  achromatic: 'stellaria:wallpaper:achromatic',
  fullscreen: 'stellaria:fullscreen',
  apps: 'stellaria:apps',
  wallpapers: 'stellaria:wallpapers',
  dashboard: monitor => `stellaria:dashboard:${monitor}`,
  focus: 'stellaria:focus',
  tasks: 'stellaria:tasks',
  taskIcons: 'stellaria:task-icons',
  pinnedTasks: 'stellaria:pinned-tasks',
  event: type => `stellaria:event:${type}`,
  popout: monitor => `stellaria:popout:${monitor}`,
  dashboardWidth: 'stellaria:dashboard:width',
  launcherQuery: 'stellaria:launcher:query',
  location: 'stellaria:location',
  weather: 'stellaria:weather',
  sysinfo: 'stellaria:sysinfo',
  popupClosed: name => `stellaria:popup:${name}:closed`,
  popupDismiss: name => `stellaria:popup:${name}:dismiss`,
  popupRequest: name => `stellaria:popup:${name}:request`,
  popupAck: name => `stellaria:popup:${name}:ack`,
  popupState: name => `stellaria:popup:${name}:state`,
  popupSpawn: name => `stellaria:popup:${name}:spawn`,
};

function migrateKeys() {
  try {
    const legacy = 'caelestia:';
    for (const key of Object.keys(localStorage)) {
      if (!key.startsWith(legacy)) continue;
      const renamed = `stellaria:${key.slice(legacy.length)}`;
      if (localStorage.getItem(renamed) === null) localStorage.setItem(renamed, localStorage.getItem(key));
      localStorage.removeItem(key);
    }
  } catch {
    return;
  }
}

migrateKeys();

export const store = {
  get(key, fallback = null) {
    try {
      const raw = localStorage.getItem(key);
      return raw === null ? fallback : JSON.parse(raw);
    } catch {
      return fallback;
    }
  },

  set(key, value) {
    try {
      localStorage.setItem(key, JSON.stringify(value));
      window.dispatchEvent(new CustomEvent('stellaria-store', { detail: key }));
    } catch (error) {
      console.error(`[stellaria] Impossible d'écrire ${key}`, error);
    }
  },

  watch(key, callback, interval = 600) {
    let last = localStorage.getItem(key);
    const check = () => {
      const current = localStorage.getItem(key);
      if (current === last) return;
      last = current;
      callback(store.get(key));
    };
    window.addEventListener('storage', event => {
      if (event.key === key) check();
    });
    window.addEventListener('stellaria-store', event => {
      if (event.detail === key) check();
    });
    setInterval(check, Math.max(interval, 2000));
  },
};

export function option(name, fallback, callback) {
  const key = `stellaria:option:${name}`;
  callback(store.get(key, fallback));
  store.watch(key, value => callback(value ?? fallback), 250);
}

export function setOption(name, value) {
  store.set(`stellaria:option:${name}`, value);
}

export function getOption(name, fallback) {
  return store.get(`stellaria:option:${name}`, fallback);
}
