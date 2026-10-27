/* eslint-disable @typescript-eslint/explicit-function-return-type */
import assert from 'node:assert/strict'
import { isIP } from 'node:net'

export const required = {
    did: 'string',
    uuid: 'string',
    os: 'string',
    osName: 'string',
    release: 'string',
    kernel: 'string',
    host: 'string',
    arch: 'string',
    uptime: 'number',
    cpu: 'string',
    gpu: 'string',
    memory: 'number',
    disk: 'number',
    mac: 'string',
    netName: 'string',
    screenWidth: 'number',
    screenHeight: 'number'
}
export const optional = {
    manufacturer: 'string',
    model: 'string',
    cores: 'number',
    freeMemory: 'number',
    ip: 'string',
    netType: 'string',
    screenScale: 'number',
    battery: 'boolean'
}

export const removed = ['hostname', 'netInterface', 'scaleFactor', 'hasBattery']

/** 校验安全断言 */
export function check(isValid, label) {
    assert.ok(Boolean(isValid), label)
}

/** 校验真实契约 */
export function validate(value) {
    check(value !== null && typeof value === 'object' && !Array.isArray(value), 'result object contract')
    const keys = Object.keys(value)
    const fields = { ...required, ...optional }
    check(removed.every(key => !(key in value)), 'removed field names absent contract')
    check(Reflect.ownKeys(value).length === keys.length, 'enumerable string keys contract')
    check(keys.every(key => Object.hasOwn(fields, key)), 'exact DeviceInfo keys contract')
    for (const [key, type] of Object.entries(required)) {
        check(Object.hasOwn(value, key) && typeof value[key] === type, `required ${key} type contract`)
        if (type === 'string' && !['gpu', 'mac', 'netName'].includes(key)) {
            check(value[key].trim().length > 0, `required ${key} nonempty contract`)
        }
    }
    for (const [key, type] of Object.entries(optional)) {
        check(!Object.hasOwn(value, key) || typeof value[key] === type, `optional ${key} type contract`)
        if (Object.hasOwn(value, key) && type === 'string') check(value[key].length > 0, `optional ${key} nonempty contract`)
    }
    for (const key of ['did', 'uuid']) {
        check(/^[a-f0-9]{64}$/.test(value[key]), `${key} SHA-256 contract`)
    }
    check(value.os === 'win' && value.kernel === 'Windows_NT', 'Windows identity contract')
    check(/^\d+\.\d+\.\d+$/.test(value.release), 'kernel release contract')
    if (Number(value.release.split('.')[2]) >= 22000 && !value.osName.includes('Server')) {
        check(!value.osName.includes('Windows 10'), 'Windows product generation contract')
    }
    check(['x64', 'arm64', 'ia32'].includes(value.arch), 'native architecture contract')
    check(Number.isFinite(value.uptime) && value.uptime >= 0, 'uptime seconds contract')
    for (const key of ['memory', 'disk', 'screenWidth', 'screenHeight', 'cores']) {
        if (value[key] !== undefined) check(Number.isSafeInteger(value[key]) && value[key] > 0, `${key} positive integer contract`)
    }
    if (value.freeMemory !== undefined) {
        check(Number.isSafeInteger(value.freeMemory) && value.freeMemory >= 0 && value.freeMemory <= value.memory, 'available memory bytes contract')
    }
    if (value.screenScale !== undefined) check(Number.isFinite(value.screenScale) && value.screenScale > 0, 'monitor scale contract')
    if (value.ip !== undefined) check(isIP(value.ip) !== 0, 'IP address contract')
    if (value.mac !== '') check(/^(?:[a-f0-9]{2}:)*[a-f0-9]{2}$/.test(value.mac), 'MAC address contract')
    if (value.netName === '') {
        check(value.mac === '' && value.ip === undefined && value.netType === undefined, 'empty selected interface contract')
    } else {
        check(typeof value.netType === 'string', 'selected interface type contract')
    }
    check(typeof JSON.stringify(value) === 'string', 'JSON serialization contract')
}
