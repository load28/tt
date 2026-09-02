import assert from 'node:assert/strict'
import { readdir } from 'node:fs/promises'
import { join } from 'node:path'
import test from 'node:test'

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
