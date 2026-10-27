import { execFileSync } from 'node:child_process'
import { copyFile, mkdir, mkdtemp, rename, rm } from 'node:fs/promises'
import { createRequire } from 'node:module'
import { dirname, join, relative } from 'node:path'
import { fileURLToPath } from 'node:url'

const root = fileURLToPath(new URL('../', import.meta.url))
const require = createRequire(import.meta.url)
await mkdir(join(root, 'target'), { recursive: true })
const directory = await mkdtemp(join(root, 'target', 'native-'))
try {
    await copyFile(join(root, 'package.json'), join(directory, 'package.json'))
    execFileSync(process.execPath, [join(dirname(require.resolve('@napi-rs/cli/package.json')), 'dist', 'cli.js'), 'build',
        '--platform', '--release', '--no-js', '--target', 'x86_64-pc-windows-msvc',
        '--package-json-path', relative(root, join(directory, 'package.json')),
        '--output-dir', join(directory, 'output'), '--dts', 'types.d.ts'],
    { cwd: root, stdio: 'inherit', timeout: 180000 })
    execFileSync(process.execPath, [join(root, 'scripts', 'declarations.mjs'), join(directory, 'output', 'types.d.ts')],
        { cwd: root, stdio: 'inherit', timeout: 10000 })
    await mkdir(join(root, 'dist', 'native'), { recursive: true })
    await rename(join(directory, 'output', 'device.win32-x64-msvc.node'), join(root, 'dist', 'native', 'device.win32-x64-msvc.node'))
    await rename(join(directory, 'output', 'types.d.ts'), join(root, 'src', 'types.d.ts'))
} catch (error) {
    console.error(`Native build/publish failed; no destination is deleted before replacement. Inspect staging: ${directory}. If a destination is locked, close its owner and retry.`)
    throw error
}
await rm(directory, { recursive: true })
