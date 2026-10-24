const { contextBridge, ipcRenderer } = require('electron');

const listen = (channel) => (cb) => ipcRenderer.on(channel, (_e, data) => cb(data));

contextBridge.exposeInMainWorld('arc', {
  send: (action, arg) => ipcRenderer.send('ui', action, arg),
  onState: listen('state'),
  onMetrics: listen('metrics'),
  onToast: listen('toast'),
  onPalette: listen('palette:open'),
});
