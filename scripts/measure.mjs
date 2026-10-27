/* eslint-disable @typescript-eslint/explicit-function-return-type */
import { performance } from 'node:perf_hooks'
import { validate } from '../tests/contracts.mjs'

async function measure(run) {
    const times = []
    for (let index = 0; index < 15; index += 1) {
        const start = performance.now()
        const value = await run()
        times.push(performance.now() - start)
        validate(value)
    }
    const sorted = times.sort((left, right) => left - right)
    return {
        count: sorted.length,
        p50Ms: sorted[Math.ceil(sorted.length * 0.5) - 1],
        p95Ms: sorted[Math.ceil(sorted.length * 0.95) - 1]
    }
}

async function main() {
    const start = performance.now()
    const { device } = await import('@hlw-ele/plugin-device')
    const imported = performance.now()
    const value = device.info()
    const first = performance.now()
    validate(value)
    if (process.argv[2] === 'cold') {
        process.stdout.write(JSON.stringify({
            importMs: imported - start,
            firstInfoMs: first - imported,
            totalMs: first - start,
            fields: Object.keys(value).length
        }))
        return
    }
    process.stdout.write(JSON.stringify({ cloneSnapshot: await measure(() => device.info()) }))
}

main().catch(() => {
    process.exitCode = 1
})
