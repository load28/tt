import assert from 'node:assert/strict'
import test from 'node:test'

import { DeliberationEngine, formatOutcome } from '../src/deliberation.mjs'
import { EventProcessor, mentionedAgents } from '../src/server.mjs'

const agents = [
  { id: 'designer', displayName: 'Designer', githubLogin: 'tt-designer[bot]', perspective: 'design' },
  { id: 'skeptic', displayName: 'Skeptic', githubLogin: 'tt-skeptic[bot]', perspective: 'risk' },
]
const config = {
  repository: { owner: 'load28', name: 'tt' },
  controller: { githubLogin: 'tt-moderator[bot]' },
  agents,
  deliberation: { language: 'ko', minimumRounds: 2, maximumRounds: 3 },
}
const discussion = {
  id: 'D_kw', title: 'proposal', body: 'body', comments: { nodes: [] },
}

function agentResponse(agentId, canAccept = false) {
  return {
    message: `${agentId} opinion`, understoodPositions: [], stanceChange: 'maintain',
    changeReason: '근거 유지', currentProposal: `${agentId} proposal`, agreements: [],
    criticalObjections: canAccept ? [] : ['unresolved'], canAccept,
  }
}

test('requires the minimum number of rounds before accepting consensus', async () => {
  const calls = []
  const codex = {
    async run(_prompt, schema) {
      if (schema === 'agent-response.json') return agentResponse(`agent-${calls.length}`, true)
      calls.push('moderator')
      return {
        decision: 'consensus', summary: 'agreed', proposedResolution: 'ship it',
        agreements: ['scope'], unresolved: [], nextQuestion: '',
      }
    },
  }
  const comments = []
  const github = { async addDiscussionComment(identity, _id, body) { comments.push([identity, body]) } }
  const sessions = []
  const state = { async recordSession(_id, session) { sessions.push(session) } }
  const engine = new DeliberationEngine(config, { codex, github, state })

  const result = await engine.deliberate(discussion, { source: 'discussion' })

  assert.equal(result.rounds.length, 2)
  assert.equal(result.outcome.decision, 'consensus')
  assert.equal(comments.length, 5)
  assert.equal(sessions.length, 1)
})

test('escalates to a human when the final round still requests discussion', async () => {
  const codex = {
    async run(_prompt, schema) {
      if (schema === 'agent-response.json') return agentResponse('agent')
      return {
        decision: 'continue', summary: 'split', proposedResolution: 'options',
        agreements: [], unresolved: ['priority'], nextQuestion: 'which priority?',
      }
    },
  }
  const github = { async addDiscussionComment() {} }
  const state = { async recordSession() {} }
  const engine = new DeliberationEngine(config, { codex, github, state })

  const result = await engine.deliberate(discussion, { source: 'discussion' })

  assert.equal(result.rounds.length, 3)
  assert.equal(result.outcome.decision, 'human_required')
  assert.match(formatOutcome(result.outcome, 3), /사람의 최종 판단/)
})

test('does not accept moderator consensus while an agent has a critical objection', async () => {
  let agentCall = 0
  const codex = {
    async run(_prompt, schema) {
      if (schema === 'agent-response.json') {
        agentCall += 1
        return agentResponse(`agent-${agentCall}`, agentCall > 2)
      }
      return {
        decision: 'consensus', summary: 'agreed', proposedResolution: 'ship it',
        agreements: ['scope'], unresolved: [], nextQuestion: 'resolve objection',
      }
    },
  }
  const engine = new DeliberationEngine(config, {
    codex,
    github: { async addDiscussionComment() {} },
    state: { async recordSession() {} },
  })

  const result = await engine.deliberate(discussion, { source: 'discussion' })

  assert.equal(result.rounds.length, 2)
  assert.equal(result.outcome.decision, 'consensus')
})

test('routes an explicit bot mention only to that perspective', () => {
  assert.deepEqual(mentionedAgents('검토해 주세요 @tt-skeptic', agents), ['skeptic'])
  assert.deepEqual(mentionedAgents('@tt-skeptic-extra는 다른 이름입니다', agents), [])
})

test('accepts human Discussion events and ignores bot comments', () => {
  const processor = new EventProcessor(config, {
    github: {}, engine: {}, state: {}, logger: { error() {} },
  })
  const base = {
    action: 'created',
    repository: { owner: { login: 'load28' }, name: 'tt' },
    discussion: { node_id: 'D_kw', user: { login: 'human', type: 'User' }, body: 'hello' },
  }
  assert.equal(processor.accept('discussion', base).discussionId, 'D_kw')
  assert.equal(processor.accept('discussion_comment', {
    ...base,
    comment: { body: 'bot output', user: { login: 'tt-skeptic[bot]', type: 'Bot' } },
  }), null)
})
