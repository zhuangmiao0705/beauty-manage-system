import { build } from 'esbuild'
import { mkdtemp, rm } from 'node:fs/promises'
import { tmpdir } from 'node:os'
import { join } from 'node:path'
import { spawnSync } from 'node:child_process'

const directory = await mkdtemp(join(tmpdir(), 'salon-tests-'))
try {
  const output = join(directory, 'commission.test.cjs')
  await build({
    entryPoints: ['tests/commission.test.ts'],
    outfile: output,
    bundle: true,
    platform: 'node',
    format: 'cjs',
    logLevel: 'warning'
  })
  const result = spawnSync(process.execPath, ['--test', output], { stdio: 'inherit' })
  process.exitCode = result.status ?? 1
} finally {
  await rm(directory, { recursive: true, force: true })
}
