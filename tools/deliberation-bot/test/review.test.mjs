import assert from 'node:assert/strict'
import test from 'node:test'

import {
  PullRequestReviewEngine,
  buildReviewMaterial,
  formatOutcome,
} from '../src/review.mjs'
import { EventProcessor } from '../src/server.mjs'

const agents = [
  { id: 'designer', displayName: 'Designer', githubLogin: 'tt-designer[bot]', perspective: 'design' },
  { id: 'skeptic', displayName: 'Skeptic', githubLogin: 'tt-skeptic[bot]', perspective: 'risk' },
]
const config = {
  repository: { owner: 'load28', name: 'tt' },
  controller: { githubLogin: 'tt-moderator[bot]' },
  agents,
  review: { language: 'ko', minimumRounds: 2, maximumRounds: 3, maximumPatchCharacters: 1_000 },
}
const pullRequest = {
  id: 'PR_kw', number: 66, title: 'proposal', body: 'body', url: 'https://example.test/66',
  author: 'human', state: 'open', draft: false, baseRef: 'main', baseSha: 'base',
  headRef: 'feature', headSha: 'abc123', additions: 1, deletions: 0, changedFiles: 1,
  files: [{ path: 'src/a.rs', status: 'modified', additions: 1, deletions: 0, patch: '@@ +1 @@\n+new' }],
  reviews: [],
}

function reviewerResponse(agentId, canAccept = false) {
  return {
    message: `${agentId} opinion`, understoodPositions: [], findings: canAccept ? [] : [{
      severity: 'major', path: 'src/a.rs', line: 1, title: 'risk',
      evidence: 'patch evidence', recommendation: 'fix it',
    }],
    stanceChange: 'maintain', changeReason: '근거 유지',
    currentProposal: `${agentId} proposal`, agreements: [],
    criticalObjections: canAccept ? [] : ['unresolved'], canAccept,
  }
}

function moderatorResponse(decision = 'consensus') {
  return {
    decision, summary: 'agreed', proposedResolution: 'ship it',
    agreements: ['scope'], unresolved: [], nextQuestion: '',
  }
}

test('requires the minimum number of rounds before accepting consensus', async () => {
  const codex = {
    async run(_prompt, schema) {
      return schema === 'reviewer-response.json'
        ? reviewerResponse('agent', true)
        : moderatorResponse()
    },
  }
  const reviews = []
  const github = {
    async addPullRequestReview(identity, number, sha, body) {
      reviews.push({ identity, number, sha, body })
    },
  }
  const sessions = []
  const state = { async recordSession(id, session) { sessions.push({ id, session }) } }
  const engine = new PullRequestReviewEngine(config, { codex, github, state })

  const result = await engine.review(pullRequest, { action: 'opened', headSha: 'abc123' })

  assert.equal(result.rounds.length, 2)
  assert.equal(result.outcome.decision, 'consensus')
  assert.equal(reviews.length, 5)
  assert.ok(reviews.every((review) => review.sha === 'abc123'))
  assert.equal(sessions.length, 1)
  assert.match(reviews.at(-1).body, /사람의 최종 검토 대기/)
})

test('escalates to a human when the final round still requests review', async () => {
  const codex = {
    async run(_prompt, schema) {
      return schema === 'reviewer-response.json'
        ? reviewerResponse('agent')
        : moderatorResponse('continue')
    },
  }
  const engine = new PullRequestReviewEngine(config, {
    codex,
    github: { async addPullRequestReview() {} },
    state: { async recordSession() {} },
  })

  const result = await engine.review(pullRequest, { action: 'opened', headSha: 'abc123' })

  assert.equal(result.rounds.length, 3)
  assert.equal(result.outcome.decision, 'human_required')
  assert.match(formatOutcome(result.outcome, 3, 'abc123'), /사람의 최종 판단/)
})

test('does not accept moderator consensus while a reviewer has a major finding', async () => {
  let reviewerCall = 0
  const codex = {
    async run(_prompt, schema) {
      if (schema === 'reviewer-response.json') {
        reviewerCall += 1
        return reviewerResponse(`agent-${reviewerCall}`, reviewerCall > agents.length)
      }
      return moderatorResponse()
    },
  }
  const engine = new PullRequestReviewEngine(config, {
    codex,
    github: { async addPullRequestReview() {} },
    state: { async recordSession() {} },
  })

  const result = await engine.review(pullRequest, { action: 'opened', headSha: 'abc123' })

  assert.equal(result.rounds.length, 2)
  assert.equal(result.outcome.decision, 'consensus')
})

test('accepts only relevant pull request revisions from the configured repository', () => {
  const processor = new EventProcessor(config, {
    github: {}, engine: {}, state: {}, logger: { error() {} },
  })
  const base = {
    action: 'opened',
    sender: { login: 'human', type: 'User' },
    repository: { owner: { login: 'load28' }, name: 'tt' },
    pull_request: {
      node_id: 'PR_kw', number: 66, html_url: 'https://example.test/66',
      head: { sha: 'abc123' },
    },
  }
  assert.equal(processor.accept('pull_request', base).headSha, 'abc123')
  assert.equal(processor.accept('pull_request', { ...base, action: 'closed' }), null)
  assert.equal(processor.accept('discussion', base), null)
})

test('skips a queued event when the pull request head has changed', async () => {
  let reviewed = false
  const processor = new EventProcessor(config, {
    github: { async fetchPullRequest() { return { ...pullRequest, headSha: 'new-sha' } } },
    engine: { async review() { reviewed = true } },
    state: {},
    logger: { log() {}, error() {} },
  })

  const result = await processor.process({
    pullRequestId: 'PR_kw', pullRequestNumber: 66, headSha: 'old-sha',
  })

  assert.equal(result, null)
  assert.equal(reviewed, false)
})

test('omits whole patches after the configured prompt budget is exhausted', () => {
  const material = buildReviewMaterial({
    ...pullRequest,
    files: [
      { ...pullRequest.files[0], path: 'a.rs', patch: '1234' },
      { ...pullRequest.files[0], path: 'b.rs', patch: '5678' },
    ],
  }, 5)

  assert.equal(material.files[0].patch, '1234')
  assert.equal(material.files[1].patch, null)
  assert.equal(material.files[1].patchOmitted, true)
  assert.equal(material.omittedPatchCount, 1)
})
