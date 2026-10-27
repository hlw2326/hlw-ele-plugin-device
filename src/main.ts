import { ipcMain } from 'electron'
import { device } from './index'
import { IPC } from './constants'

/**
 * 初始化设备：注册 hlw:device:*
 */
export function initDevice(): void {
    ipcMain.on(IPC.info, (event) => {
        event.returnValue = device.info()
    })
}
