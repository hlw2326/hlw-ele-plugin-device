import type { DeviceInfo } from './types'

export type { DeviceInfo }

// 独立入口，避免被 main / preload 的类型引用带进其它进程的类型环境
declare module '@hlw-ele/types' {
    interface HlwAPI {
        device: {
            info: () => DeviceInfo
        }
    }
}

export type { HlwAPI } from '@hlw-ele/types'
