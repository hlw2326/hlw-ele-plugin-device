import { readFile, writeFile } from 'node:fs/promises'

const path = process.argv[2] ?? new URL('../src/types.d.ts', import.meta.url)
const source = await readFile(path, 'utf8')
const formatted = source.replace(/\/\*\*([\s\S]*?)\*\//g, (_, block) => {
    const lines = block.split(/\r?\n/).map(line => line.trim().replace(/^\*\s?/, '')).filter(Boolean)
    return `/**\n${lines.map(line => ` * ${line}`).join('\n')}\n */`
})
await writeFile(path, formatted)
