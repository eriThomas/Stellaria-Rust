export function bindTransientTaskReset(client, storage, taskKey, render) {
  return client.onConnection(connected => {
    if (connected) return;
    storage.set(taskKey, []);
    render();
  });
}

export function keepWidgetOutOfTaskSwitcher(widget) {
  return widget.tauriWindow.setSkipTaskbar(true);
}
