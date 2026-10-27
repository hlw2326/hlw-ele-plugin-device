/* eslint-disable @typescript-eslint/explicit-function-return-type */
import { execFile } from 'node:child_process'
import { performance } from 'node:perf_hooks'
import { fileURLToPath } from 'node:url'
import { promisify } from 'node:util'

const execute = promisify(execFile)
const worker = fileURLToPath(new URL('./measure.mjs', import.meta.url))

async function collect(mode) {
    const { stdout } = await execute(process.execPath, [worker, mode], {
        timeout: 30000,
        killSignal: 'SIGKILL',
        windowsHide: true,
        maxBuffer: 64 * 1024
    })
    return JSON.parse(stdout)
}

function summarize(values) {
    const sorted = [...values].sort((left, right) => left - right)
    return {
        count: sorted.length,
        p50Ms: sorted[Math.ceil(sorted.length * 0.5) - 1],
        p95Ms: sorted[Math.ceil(sorted.length * 0.95) - 1]
    }
}

async function main() {
    const samples = []
    for (let index = 0; index < 5; index += 1) {
        const start = performance.now()
        const value = await collect('cold')
        samples.push({ ...value, processMs: performance.now() - start })
    }
    const results = {}
    for (const key of ['importMs', 'firstInfoMs', 'totalMs', 'processMs']) {
        results[key] = summarize(samples.map(sample => sample[key]))
    }
    results.fieldCounts = samples.map(sample => sample.fields)
    results.cachedSnapshot = await collect('cached')
    console.log(JSON.stringify(results, null, 4))
}

main().catch(() => {
    console.error('Benchmark failed or exceeded the 30000 ms child timeout; details suppressed')
    process.exitCode = 1
})
