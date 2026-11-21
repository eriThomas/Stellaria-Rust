export const customizerDefaults = Object.freeze({
  borderSize: 0,
  showActiveWindow: false,
  showTasksFromAllMonitors: true,
  launcherWinKey: false,
  groupTasks: true,
  taskPreviews: true,
  minimizeActiveTaskOnClick: false,
});

export function clampBorderSize(value) {
  return Math.max(0, Math.min(40, Number(value) || 0));
}
