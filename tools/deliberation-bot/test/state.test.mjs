import assert from 'node:assert/strict'
import { mkdtemp } from 'node:fs/promises'
import { tmpdir } from 'node:os'
import { join } from 'node:path'
import test from 'node:test'

import { StateStore } from '../src/state.mjs'

test('allows a failed delivery to be retried', async () => {
  const directory = await mkdtemp(join(tmpdir(), 'tt-deliberation-state-'))
  const state = new StateStore(directory)
  await state.initialize()

  await state.mark('delivery-1')
  assert.equal(state.has('delivery-1'), true)
  await state.forget('delivery-1')
  assert.equal(state.has('delivery-1'), false)

  const reloaded = new StateStore(directory)
  await reloaded.initialize()
  assert.equal(reloaded.has('delivery-1'), false)
})
