import assert from 'node:assert/strict'
import { spawnSync } from 'node:child_process'
import { createHash } from 'node:crypto'
import { readFile, rm } from 'node:fs/promises'
import { createRequire } from 'node:module'
import { dirname, join, relative } from 'node:path'
import { fileURLToPath } from 'node:url'
import test from 'node:test'

const root = fileURLToPath(new URL('../', import.meta.url))
const require = createRequire(import.meta.url)

test('a loaded native destination rejects replacement without deleting previous artifacts', { timeout: 200000 }, async () => {
    const binary = join(root, 'dist', 'native', 'device.win32-x64-msvc.node')
    const declaration = join(root, 'src', 'types.d.ts')
    const before = await Promise.all([binary, declaration].map(path => readFile(path)))
    require(binary)
    const child = spawnSync(process.execPath, [join(root, 'scripts', 'native.mjs')], {
        cwd: root, encoding: 'utf8', timeout: 180000, windowsHide: true
    })
    assert.equal(child.error, undefined)
    assert.notEqual(child.status, 0, 'loaded DLL must not be silently replaced')
    const match = child.stderr.match(/Inspect staging: (.+?)\. If a destination/)
    assert.ok(match, 'failed build reports its own staging directory')
    const directory = match[1]
    assert.equal(dirname(directory), join(root, 'target'))
    assert.match(relative(join(root, 'target'), directory), /^native-[\w-]+$/)
    try {
        assert.match(child.stderr, /EACCES|EPERM|EBUSY/)
        assert.match(child.stderr, /rename/)
        const staged = await readFile(join(directory, 'output', 'device.win32-x64-msvc.node'))
        assert.ok(staged.length > 0, 'napi completes in isolation before destination replacement')
        const after = await Promise.all([binary, declaration].map(path => readFile(path)))
        for (let index = 0; index < before.length; index++) {
            assert.equal(createHash('sha256').update(after[index]).digest('hex'),
                createHash('sha256').update(before[index]).digest('hex'), 'locked build preserves published artifacts')
        }
    } finally {
        await rm(directory, { recursive: true })
    }
})
