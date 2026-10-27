import assert from 'node:assert/strict'
import { access, readFile } from 'node:fs/promises'
import { fileURLToPath } from 'node:url'
import { normalize } from 'node:path'
import test from 'node:test'
import ts from 'typescript'
import { required, optional, removed } from './contracts.mjs'

const root = new URL('../', import.meta.url)
const manifest = JSON.parse(await readFile(new URL('package.json', root), 'utf8'))

test('published exports contain the package entries and their artifacts', async () => {
    assert.equal(manifest.name, '@hlw-ele/plugin-device')
    assert.deepEqual(Object.keys(manifest.exports), ['.', './main', './preload', './types'])
    assert.ok(manifest.files.includes('dist'))
    for (const hook of ['preinstall', 'install', 'postinstall', 'prepare']) assert.equal(manifest.scripts[hook], undefined)
    assert.equal(manifest.dependencies, undefined)
    assert.deepEqual(manifest.os, ['win32'])
    assert.deepEqual(manifest.cpu, ['x64'])
    for (const entry of Object.values(manifest.exports)) {
        if (!entry.import) continue
        for (const mode of ['import', 'require']) {
            await access(new URL(entry[mode].default, root))
            await access(new URL(entry[mode].types, root))
        }
    }
    await access(new URL('dist/native/device.win32-x64-msvc.node', root))
    const module = await import('@hlw-ele/plugin-device')
    assert.deepEqual(Object.keys(module), ['device'])
})

for (const extension of ['mts', 'cts']) {
    test(`published ${extension} declarations enforce exact flat keys and optionality`, () => {
        const filename = fileURLToPath(new URL(`tests/contract.${extension}`, root))
        const fields = Object.entries({ ...required, ...optional }).map(([key, type]) =>
            `${key}${Object.hasOwn(optional, key) ? '?' : ''}: ${type}`).join(';')
        const source = `
import { device, type DeviceInfo } from '@hlw-ele/plugin-device'
// @ts-expect-error 移除旧有类型
import type { Info, SystemInfo, CpuInfo, MemoryInfo, IdentityInfo, FirmwareInfo, GpuInfo, VolumeInfo, NetworkInfo, NetworkState, PowerState, Issue } from '@hlw-ele/plugin-device'
// @ts-expect-error 隐藏原生方法
import { info as nativeInfo } from '@hlw-ele/plugin-device'
${removed.map(key => `// @ts-expect-error 禁止旧有字段\nvoid ({} as DeviceInfo).${key}`).join('\n')}
type Expected = { ${fields} }
type Equal<Left, Right> = (<Value>() => Value extends Left ? 1 : 2) extends (<Value>() => Value extends Right ? 1 : 2) ? true : false
const keys: Equal<keyof DeviceInfo, keyof Expected> = true
const schema: Equal<DeviceInfo, Expected> = true
const method: Equal<keyof typeof device, 'info'> = true
const parameters: Equal<Parameters<typeof device.info>, []> = true
const returned: Equal<ReturnType<typeof device.info>, DeviceInfo> = true
const info: DeviceInfo = device.info()
// @ts-expect-error 禁止额外参数
void device.info({ isRefresh: true })
// @ts-expect-error 禁止额外参数
void device.info(undefined)
// @ts-expect-error 移除旧有方法
void device.getHardware()
// @ts-expect-error 禁止结果嵌套
void info.system
// @ts-expect-error 移除错误集合
void info.issues
// @ts-expect-error 移除旧有标识
void info.deviceId
const did: string = info.did
const uuid: string = info.uuid
// @ts-expect-error 标识必须存在
const missingDid: DeviceInfo = { ...info, did: undefined }
// @ts-expect-error 标识必须存在
const missingUuid: DeviceInfo = { ...info, uuid: undefined }
void [did, uuid, missingDid, missingUuid]
void [info, keys, schema, method, parameters, returned]
`
        const options = {
            module: ts.ModuleKind.NodeNext,
            moduleResolution: ts.ModuleResolutionKind.NodeNext,
            target: ts.ScriptTarget.ES2022,
            strict: true,
            exactOptionalPropertyTypes: true,
            noEmit: true,
            types: ['node']
        }
        const host = ts.createCompilerHost(options)
        const getSourceFile = host.getSourceFile.bind(host)
        host.getSourceFile = (path, language, onError, isNew) => normalize(path) === normalize(filename)
            ? ts.createSourceFile(path, source, language, true)
            : getSourceFile(path, language, onError, isNew)
        const program = ts.createProgram([filename], options, host)
        const diagnostics = ts.getPreEmitDiagnostics(program)
        assert.equal(diagnostics.length, 0, `published type contract diagnostic codes: ${diagnostics.map(item => item.code).join(',')}`)
    })
}

test('Rust-generated fields retain concise multiline JSDoc without nested public types', async () => {
    const declaration = await readFile(new URL('src/types.d.ts', root), 'utf8')
    const source = ts.createSourceFile('types.d.ts', declaration, ts.ScriptTarget.Latest, true)
    const interfaces = source.statements.filter(ts.isInterfaceDeclaration)
    assert.deepEqual(interfaces.map(item => item.name.text), ['DeviceInfo'])
    const members = interfaces[0].members
    assert.equal(members.length, Object.keys(required).length + Object.keys(optional).length)
    assert.ok(members.every(member => !removed.includes(member.name.getText(source))), 'removed field names absent from generated declarations')
    for (const member of members) {
        const docs = member.jsDoc
        assert.equal(docs?.length, 1, 'one field documentation block')
        assert.match(docs[0].comment, /^[\u4e00-\u9fff]{4,6}$/, 'concise Chinese field documentation')
        assert.match(docs[0].getText(source), /^\/\*\*\r?\n/, 'multiline field documentation')
    }
})
