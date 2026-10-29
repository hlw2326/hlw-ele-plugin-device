import { defineConfig } from 'tsdown'

export default defineConfig({
    entry: ['src/index.ts', 'src/main.ts', 'src/preload.ts', 'src/augment.ts'],
    format: ['esm', 'cjs'],
    platform: 'node',
    deps: { neverBundle: ['electron', '@hlw-ele/core', '@hlw-ele/types'] },
    dts: true,
    clean: false
})
