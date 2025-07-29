import { contextBridge, ipcRenderer } from 'web';

contextBridge.exposeInMainWorld('electronAPI', {
	openDirectory: async () => await ipcRenderer.invoke('dialog:openDirectory')
});
