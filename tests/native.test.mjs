/* eslint-disable @typescript-eslint/explicit-function-return-type */
import { AssertionError } from 'node:assert'
import { createRequire } from 'node:module'
import { totalmem, uptime, release, hostname, networkInterfaces } from 'node:os'
import { setTimeout } from 'node:timers/promises'
import test from 'node:test'
import { check, validate, required, optional, removed } from './contracts.mjs'

const require = createRequire(import.meta.url)

safeTest('contract validator rejects extras, missing required fields and invalid optional values', async () => {
    const { device } = await import('@hlw-ele/plugin-device')
    const value = structuredClone(device.info())
    for (const key of Object.keys(required)) {
        const invalid = { ...value }
        delete invalid[key]
        let isRejected = false
        try {
            validate(invalid)
        } catch {
            isRejected = true
        }
        check(isRejected, 'missing required field rejection')
    }
    for (const key of Object.keys(optional)) {
        const invalid = { ...value, [key]: null }
        let isRejected = false
        try {
            validate(invalid)
        } catch {
            isRejected = true
        }
        check(isRejected, 'invalid optional field rejection')
    }
    for (const key of [...removed, 'deviceId', 'smbiosUuid', 'issues', 'info', 'system', 'graphics', 'volumes']) {
        let isRejected = false
        try {
            validate({ ...value, [key]: [] })
        } catch {
            isRejected = true
        }
        check(isRejected, 'extra field rejection')
    }
    for (const key of ['did', 'uuid']) {
        for (const invalid of ['', 'A'.repeat(64), '0'.repeat(63), '00010203-0405-0607-0809-0a0b0c0d0e0f', null, undefined]) {
            let isRejected = false
            try {
                validate({ ...value, [key]: invalid })
            } catch {
                isRejected = true
            }
            check(isRejected, 'invalid identity hash rejection')
        }
    }
    for (const key of Object.keys(optional)) delete value[key]
    if (value.netName !== '') value.netType = 'ifType:999'
    validate(value)
})

function safeTest(name, run) {
    test(name, { timeout: 30000 }, async () => {
        try {
            await run()
        } catch (error) {
            if (error instanceof AssertionError) throw error
            // 隔离设备错误
            // eslint-disable-next-line preserve-caught-error
            throw new Error('Native integration failed; device details suppressed')
        }
    })
}

for (const mode of ['ESM', 'CJS', 'native']) {
    safeTest(`${mode} exposes only synchronous info with the complete flat contract`, async () => {
        let api
        if (mode === 'ESM') api = (await import('@hlw-ele/plugin-device')).device
        if (mode === 'CJS') api = require('@hlw-ele/plugin-device').device
        if (mode === 'native') api = require('../dist/native/device.win32-x64-msvc.node')
        check(Object.keys(api).join(',') === 'info', 'single collector export contract')
        check(typeof api.info === 'function' && api.info.length === 0, 'zero argument contract')
        validate(api.info())
    })
}

safeTest('native binding collects fresh memory and uptime independently of facade caching', async () => {
    const device = require('../dist/native/device.win32-x64-msvc.node')
    const before = uptime()
    const first = device.info()
    await setTimeout(40)
    const second = device.info()
    const after = uptime()
    for (const value of [first, second]) {
        validate(value)
        check(value.memory === totalmem(), 'OS memory byte contract')
        check(value.release === release(), 'real kernel version contract')
        check(value.host === hostname(), 'real hostname contract')
        const interfaces = networkInterfaces()
        const hasMac = Object.values(interfaces).some(addresses => addresses?.some(item =>
            !item.internal && item.mac !== '' && !/^(?:00:)*00$|^(?:ff:)*ff$/i.test(item.mac)
        ))
        if (hasMac) check(value.mac !== '', 'active MAC interface selection contract')
        if (value.ip !== undefined) {
            const addresses = networkInterfaces()[value.netName]
            check(addresses?.some(item => item.address === value.ip || `${item.address}%${item.scopeid}` === value.ip), 'same interface IP contract')
            if (value.mac !== '') check(addresses?.some(item => item.mac === value.mac), 'same interface MAC contract')
        }
    }
    check(first.uptime >= before - 1 && second.uptime <= after + 1, 'OS uptime seconds contract')
    check(second.uptime > first.uptime, 'fresh collection contract')
})

safeTest('facade snapshot reads return independent complete objects', async () => {
    const { device } = await import('@hlw-ele/plugin-device')
    const first = device.info()
    const second = device.info()
    validate(first)
    validate(second)
    check(first !== second, 'independent result contract')
    check(JSON.stringify(first) === JSON.stringify(second), 'cached snapshot contract')
    const did = second.did
    const uuid = second.uuid
    first.did = 'isolation marker'
    first.uuid = 'isolation marker'
    check(second.did === did && second.uuid === uuid, 'concurrent isolation contract')
    const next = device.info()
    validate(next)
    check(next.did === did && next.uuid === uuid, 'subsequent isolation contract')
    check(JSON.stringify(next) === JSON.stringify(second), 'subsequent snapshot contract')
})
