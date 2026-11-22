import * as zebar from './vendor/zebar.mjs';
import { keepWidgetOutOfTaskSwitcher } from './src/core/runtime-recovery.js';

// Use Zebar's supported window API instead of changing native window styles
// from the helper. The latter can look like process injection to security tools.
keepWidgetOutOfTaskSwitcher(zebar.currentWidget())
  .catch(error => console.error('[stellaria] Alt+Tab', error));

export { zebar };
export * from './src/core/store.js';
export * from './src/core/i18n.js';
export * from './src/core/monitor.js';
export * from './src/core/theme.js';
export * from './src/core/dom.js';
export * from './src/core/helper.js';
export * from './src/core/panels.js';
export * from './src/core/runtime-recovery.js';
export * from './src/core/weather.js';
