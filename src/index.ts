import { createRequire } from 'node:module'
import type { DeviceInfo } from './types'

const require = createRequire(import.meta.url)
let binding: typeof import('./types') | undefined
let cached: DeviceInfo | undefined

function load(): typeof import('./types') {
    if (process.platform !== 'win32' || process.arch !== 'x64') {
        throw new Error('@hlw-ele/plugin-device currently supports Windows x64 only')
    }
    binding ??= require('./native/device.win32-x64-msvc.node')
    return binding!
}

/** 设备采集接口 */
export const device = Object.freeze({
    /** 获取完整信息 */
    info(): DeviceInfo {
        cached ??= load().info()
        return structuredClone(cached)
    }
})

export type { DeviceInfo } from './types'
