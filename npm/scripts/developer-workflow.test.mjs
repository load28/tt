import assert from 'node:assert/strict'
import { readFile } from 'node:fs/promises'
import { resolve } from 'node:path'
import test from 'node:test'

const root = resolve(import.meta.dirname, '../..')

test('doctor inventories every tool required by the default local gate', async () => {
  const [doctor, gate] = await Promise.all([
    readFile(resolve(root, 'scripts/doctor'), 'utf8'),
    readFile(resolve(root, 'scripts/ci'), 'utf8'),
  ])

  const required = new Set([
    ...[...gate.matchAll(/^require ([a-z][a-z0-9-]*)$/gm)].map((match) => match[1]),
    ...[...gate.matchAll(/fail "([a-z][a-z0-9-]*) is not on PATH;/g)].map((match) => match[1]),
  ])

  assert.ok(required.has('bun'), 'the default gate must keep Bun explicit')
  for (const tool of required) {
    assert.match(doctor, new RegExp(`^check_command ${tool}$`, 'm'))
  }
})
