import type { Provider } from '@hlw-ele/core'
import { device as api } from './index'
import { IPC } from './constants'

/**
 * 设备插件：注册 hlw:device:info（同步通道）
 */
export const device: Provider = {
    name: 'device',
    optional: true,
    setup(kernel) {
        kernel.handleSync(IPC.info, () => {
            return api.info()
        })
    }
}
