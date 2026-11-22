import * as zebar from './vendor/zebar.mjs';

// Use Zebar's supported window API instead of changing native window styles
// from the helper. The latter can look like process injection to security tools.
zebar.currentWidget().tauriWindow.setSkipTaskbar(true)
  .catch(error => console.error('[stellaria] Alt+Tab', error));

export { zebar };
export * from './src/core/store.js';
export * from './src/core/i18n.js';
export * from './src/core/monitor.js';
export * from './src/core/theme.js';
export * from './src/core/dom.js';
export * from './src/core/helper.js';
export * from './src/core/panels.js';
export * from './src/core/weather.js';
