import assert from 'node:assert/strict'
import { readFile } from 'node:fs/promises'
import test from 'node:test'

const root = new URL('../', import.meta.url)

test('Windows x64 binary imports only Windows 10 system DLLs with no dynamic CRT', async () => {
    const binary = await readFile(new URL('dist/native/device.win32-x64-msvc.node', root))
    assert.equal(binary.readUInt16LE(0), 0x5a4d)
    const header = binary.readUInt32LE(0x3c)
    assert.equal(binary.readUInt32LE(header), 0x4550)
    assert.equal(binary.readUInt16LE(header + 4), 0x8664)
    const optional = header + 24
    assert.equal(binary.readUInt16LE(optional), 0x20b)
    const sections = optional + binary.readUInt16LE(header + 20)
    const count = binary.readUInt16LE(header + 6)
    // eslint-disable-next-line @typescript-eslint/explicit-function-return-type
    const offset = (address) => {
        for (let index = 0; index < count; index++) {
            const section = sections + index * 40
            const start = binary.readUInt32LE(section + 12)
            const size = binary.readUInt32LE(section + 16)
            if (address >= start && address < start + size) return address - start + binary.readUInt32LE(section + 20)
        }
        assert.fail('import RVA must belong to a file-backed section')
    }
    const imports = []
    const address = binary.readUInt32LE(optional + 120)
    assert.notEqual(address, 0)
    for (let entry = offset(address); binary.readUInt32LE(entry + 12) !== 0; entry += 20) {
        const start = offset(binary.readUInt32LE(entry + 12))
        const end = binary.indexOf(0, start)
        assert.ok(end > start)
        imports.push(binary.toString('ascii', start, end).toLowerCase())
    }
    assert.equal(binary.readUInt32LE(optional + 112 + 13 * 8), 0, 'no unchecked delay-loaded DLLs')
    const systems = new Set(['kernel32.dll', 'ntdll.dll', 'iphlpapi.dll', 'user32.dll', 'dxgi.dll',
        'advapi32.dll', 'oleaut32.dll', 'api-ms-win-shcore-scaling-l1-1-1.dll', 'api-ms-win-core-synch-l1-2-0.dll'])
    assert.ok(imports.length > 0)
    for (const name of imports) assert.ok(systems.has(name), `unexpected non-system dependency: ${name}`)
    const config = await readFile(new URL('.cargo/config.toml', root), 'utf8')
    assert.match(config, /\[target\.x86_64-pc-windows-msvc\]/)
    assert.match(config, /target-feature=\+crt-static/)
})
