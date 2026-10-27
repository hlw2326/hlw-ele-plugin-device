import assert from 'node:assert/strict'
import { execFile } from 'node:child_process'
import { copyFile, mkdir, mkdtemp, readFile, rename, rm, writeFile } from 'node:fs/promises'
import { tmpdir } from 'node:os'
import { dirname, join } from 'node:path'
import { fileURLToPath } from 'node:url'
import { promisify } from 'node:util'
import { required, optional } from '../tests/contracts.mjs'

const execute = promisify(execFile)
const manifest = JSON.parse(await readFile(new URL('../package.json', import.meta.url), 'utf8'))
const name = manifest.name.replace('@', '').replace('/', '-')
const archive = fileURLToPath(new URL(`../target/${name}-${manifest.version}.tgz`, import.meta.url))
const directory = await mkdtemp(join(tmpdir(), 'hlw-device-package-'))
try {
    await copyFile(archive, join(directory, 'package.tgz'))
    const listing = await execute('tar', ['-tzf', 'package.tgz'], { cwd: directory, timeout: 30000, windowsHide: true })
    assert.deepEqual(listing.stdout.trim().split(/\r?\n/).sort(), [
        'package/README.md', 'package/package.json', 'package/dist/index.mjs', 'package/dist/index.cjs',
        'package/dist/index.d.mts', 'package/dist/index.d.cts', 'package/dist/native/device.win32-x64-msvc.node'
    ].sort())
    await execute('tar', ['-xzf', 'package.tgz'], { cwd: directory, timeout: 30000, windowsHide: true })
    const destination = join(directory, 'node_modules', manifest.name)
    await mkdir(dirname(destination), { recursive: true })
    await rename(join(directory, 'package'), destination)
    const packed = JSON.parse(await readFile(join(destination, 'package.json'), 'utf8'))
    for (const hook of ['preinstall', 'install', 'postinstall', 'prepare']) assert.equal(packed.scripts[hook], undefined)
    assert.equal(packed.dependencies, undefined)
    for (const extension of ['mjs', 'cjs']) {
        const entry = join(directory, `consumer.${extension}`)
        const imported = extension === 'mjs'
            ? `import { device } from ${JSON.stringify(manifest.name)}`
            : `const { device } = require(${JSON.stringify(manifest.name)})`
        await copyFile(new URL('../tests/contracts.mjs', import.meta.url), join(directory, 'contracts.mjs'))

        await writeFile(entry, `${imported}
async function run() {
    const { validate } = await import('./contracts.mjs')
    const info = device.info()
    validate(info)
    process.stdout.write(JSON.stringify({ methods: Object.keys(device), fields: Object.keys(info).length }))
}
run().catch(() => { process.exitCode = 1 })
`)
        const runtimes = [process.execPath]
        if (process.env.DEVICE_ELECTRON) runtimes.push(process.env.DEVICE_ELECTRON)
        for (const runtime of runtimes) {
            const { stdout } = await execute(runtime, [entry], {
                cwd: directory, timeout: 30000, windowsHide: true,
                env: { ...process.env, ELECTRON_RUN_AS_NODE: '1' }
            })
            const value = JSON.parse(stdout)
            assert.deepEqual(value.methods, ['info'])
            assert.ok(Number.isInteger(value.fields) && value.fields >= Object.keys(required).length && value.fields <= Object.keys(required).length + Object.keys(optional).length)
            console.log(`${extension}: packed DeviceInfo contract passed (${runtime === process.execPath ? 'Node' : 'Electron node mode'}); fields=${value.fields}`)
        }
    }
} finally {
    await rm(directory, { recursive: true, force: true, maxRetries: 10, retryDelay: 200 })
}
