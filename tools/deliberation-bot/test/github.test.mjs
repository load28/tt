import assert from 'node:assert/strict'
import { generateKeyPairSync } from 'node:crypto'
import test from 'node:test'

import { createAppJwt, GitHubClient, verifyWebhookSignature } from '../src/github.mjs'

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

test('fetches PR material and posts a SHA-pinned comment review', async () => {
  const requests = []
  const responses = [
    { node_id: 'PR_kw', number: 66, title: 'title', body: null, html_url: 'url',
      user: { login: 'author' }, state: 'open', draft: false,
      base: { ref: 'main', sha: 'base' }, head: { ref: 'feature', sha: 'head' },
      additions: 2, deletions: 1, changed_files: 1 },
    [{ filename: 'src/a.rs', status: 'modified', additions: 2, deletions: 1, patch: 'diff' }],
    [],
    { id: 99, state: 'COMMENTED' },
  ]
  const client = new GitHubClient({ owner: 'load28', name: 'tt' }, {
    fetchImpl: async (url, options) => {
      requests.push({ url, options })
      const value = responses.shift()
      return { ok: true, status: 200, async json() { return value } }
    },
  })
  client.installationToken = async () => 'token'

  const pullRequest = await client.fetchPullRequest({}, 66)
  await client.addPullRequestReview({}, 66, 'head', 'review body')

  assert.equal(pullRequest.headSha, 'head')
  assert.equal(pullRequest.files[0].path, 'src/a.rs')
  const posted = requests.at(-1)
  assert.equal(posted.options.method, 'POST')
  assert.deepEqual(JSON.parse(posted.options.body), {
    commit_id: 'head', body: 'review body', event: 'COMMENT',
  })
})
