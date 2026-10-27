import { ipcRenderer } from 'electron'
import { IPC } from './constants'
import type { DeviceInfo } from './types'

/**
 * 设备
 */
export const device = {

    /**
     * 获取设备信息，同步返回
     */
    info: (): DeviceInfo => {
        return ipcRenderer.sendSync(IPC.info)
    }

}
