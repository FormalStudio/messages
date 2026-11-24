import { expect, test } from 'bun:test'
import { LAUNCH_JXA } from './refresher'

// osacompile only parses, so this checks the inline launcher script without launching Find My.
test.skipIf(process.platform !== 'darwin')('the Find My launcher script compiles as JXA', () => {
  const out = Bun.spawnSync(['osacompile', '-l', 'JavaScript', '-o', '/dev/null', '-e', LAUNCH_JXA])
  expect(out.stderr.toString()).toBe('')
  expect(out.exitCode).toBe(0)
})
