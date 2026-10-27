/* eslint-disable @typescript-eslint/explicit-function-return-type */
import assert from 'node:assert/strict'
import { readFile } from 'node:fs/promises'
import { createRequire } from 'node:module'
import { fileURLToPath } from 'node:url'
import { runInNewContext } from 'node:vm'
import test from 'node:test'

const filename = fileURLToPath(new URL('../dist/index.cjs', import.meta.url))
const source = await readFile(filename, 'utf8')
const require = createRequire(import.meta.url)

function createFacade(load) {
    const exports = {}
    runInNewContext(source, {
        exports,
        __filename: filename,
        process: { platform: 'win32', arch: 'x64' },
        structuredClone,
        require: name => name === 'node:module' ? { createRequire: () => load } : require(name)
    })
    return exports.device
}

test('facade collects once and reuses the snapshot', () => {
    let loads = 0
    let calls = 0
    const device = createFacade(() => {
        loads += 1
        return { info: () => ({ uptime: ++calls }) }
    })
    assert.deepEqual(device.info(), { uptime: 1 })
    assert.deepEqual(device.info(), { uptime: 1 })
    assert.equal(loads, 1)
    assert.equal(calls, 1)
})

test('facade retries after a collection failure', () => {
    let calls = 0
    const error = new Error('Synthetic collection failure')
    const device = createFacade(() => ({
        info: () => {
            if (++calls === 1) throw error
            return { uptime: 2 }
        }
    }))
    assert.throws(() => device.info(), error)
    assert.deepEqual(device.info(), { uptime: 2 })
})

test('facade retries after a load failure', () => {
    let loads = 0
    const error = new Error('Synthetic load failure')
    const device = createFacade(() => {
        if (++loads === 1) throw error
        return { info: () => ({ uptime: 1 }) }
    })
    assert.throws(() => device.info(), error)
    assert.deepEqual(device.info(), { uptime: 1 })
    assert.equal(loads, 2)
})

test('facade clones isolate callers and preserve optional fields', () => {
    const snapshot = { uptime: 1, freeMemory: 100, battery: false }
    const device = createFacade(() => ({ info: () => snapshot }))
    const first = device.info()
    first.uptime = 999
    delete first.freeMemory
    first.battery = true
    assert.deepEqual(device.info(), snapshot)
    const second = device.info()
    assert.deepEqual(second, snapshot)
    assert.notEqual(second, snapshot)
    second.uptime = 888
    assert.deepEqual(device.info(), snapshot)
})
