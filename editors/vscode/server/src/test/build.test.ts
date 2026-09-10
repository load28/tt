import assert from 'node:assert/strict'
import { readFile, readdir } from 'node:fs/promises'
import { join } from 'node:path'
import test from 'node:test'

test('compile and watch begin from the same exact source projection', async () => {
  const manifest = JSON.parse(
    await readFile(join(__dirname, '../../../package.json'), 'utf8'),
  ) as { scripts: Record<string, string> }

  assert.match(manifest.scripts.compile, /^npm run clean && /)
  assert.match(manifest.scripts.watch, /^npm run clean && /)
})

test('compiled tests are an exact projection of current test sources', async () => {
  const sourceDirectory = join(__dirname, '../../src/test')
  const source = (await readdir(sourceDirectory))
    .filter((file) => file.endsWith('.test.ts'))
    .map((file) => file.replace(/\.ts$/, '.js'))
    .sort()
  const output = (await readdir(__dirname))
    .filter((file) => file.endsWith('.test.js'))
    .sort()

  assert.deepEqual(output, source)
})
