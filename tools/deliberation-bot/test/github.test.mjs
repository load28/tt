import assert from 'node:assert/strict'
import { generateKeyPairSync } from 'node:crypto'
import test from 'node:test'

import { createAppJwt, verifyWebhookSignature } from '../src/github.mjs'

test('verifies GitHub sha256 webhook signatures', () => {
  const body = Buffer.from('{"action":"created"}')
  assert.equal(
    verifyWebhookSignature(
      body,
      'sha256=0031e94255b70a79704e0356204543768c078ca4f48b3ccc547edef03f4f338a',
      'secret',
    ),
    true,
  )
  assert.equal(verifyWebhookSignature(body, 'sha256=bad', 'secret'), false)
})

test('creates a three-part RS256 GitHub App JWT', () => {
  const { privateKey } = generateKeyPairSync('rsa', { modulusLength: 2048 })
  const jwt = createAppJwt('123', privateKey.export({ type: 'pkcs8', format: 'pem' }), 1_800_000)
  assert.equal(jwt.split('.').length, 3)
  const payload = JSON.parse(Buffer.from(jwt.split('.')[1], 'base64url'))
  assert.equal(payload.iss, '123')
  assert.ok(payload.exp > payload.iat)
})
