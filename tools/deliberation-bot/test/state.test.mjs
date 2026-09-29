import assert from 'node:assert/strict'
import test from 'node:test'

import { testDir } from '../../../scripts/test-dirs.cjs'

import { StateStore } from '../src/state.mjs'

test('allows a failed delivery to be retried', async () => {
  const directory = testDir('tt-deliberation-state-')
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
