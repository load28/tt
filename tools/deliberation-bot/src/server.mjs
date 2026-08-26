import { createServer } from 'node:http'

import { verifyWebhookSignature } from './github.mjs'

export class EventProcessor {
  constructor(config, { github, engine, state, logger = console }) {
    this.config = config
    this.github = github
    this.engine = engine
    this.state = state
    this.logger = logger
    this.queues = new Map()
  }

  accept(eventName, payload) {
    if (payload.action !== 'created') return null
    if (eventName !== 'discussion' && eventName !== 'discussion_comment') return null
    const discussion = payload.discussion
    if (!discussion?.node_id) return null
    const repository = payload.repository
    if (
      repository?.owner?.login !== this.config.repository.owner
      || repository?.name !== this.config.repository.name
    ) return null

    const actor = payload.comment?.user ?? discussion.user
    if (actor?.type === 'Bot') return null
    const body = payload.comment?.body ?? discussion.body ?? ''
    return {
      discussionId: discussion.node_id,
      source: eventName,
      actor: actor?.login ?? 'unknown',
      body,
      url: payload.comment?.html_url ?? discussion.html_url,
      mentions: mentionedAgents(body, this.config.agents),
    }
  }

  enqueue(trigger) {
    const previous = this.queues.get(trigger.discussionId) ?? Promise.resolve()
    const current = previous
      .catch(() => {})
      .then(() => this.process(trigger))
      .finally(() => {
        if (this.queues.get(trigger.discussionId) === current) {
          this.queues.delete(trigger.discussionId)
        }
      })
    this.queues.set(trigger.discussionId, current)
    return current
  }

  async process(trigger) {
    const discussion = await this.github.fetchDiscussion(
      this.config.controller,
      trigger.discussionId,
    )
    if (trigger.mentions.length > 0) {
      const agents = this.config.agents.filter((agent) => trigger.mentions.includes(agent.id))
      return this.engine.answerMention(discussion, trigger, agents)
    }
    return this.engine.deliberate(discussion, trigger)
  }
}

export function mentionedAgents(body, agents) {
  return agents
    .filter((agent) => {
      const login = agent.githubLogin.replace(/\[bot\]$/i, '').toLowerCase()
      const escaped = login.replaceAll(/[.*+?^${}()|[\]\\]/g, '\\$&')
      return new RegExp(`@${escaped}(?![a-zA-Z0-9-])`, 'i').test(body)
    })
    .map((agent) => agent.id)
}

export function startServer(config, processor, state, logger = console) {
  const server = createServer(async (request, response) => {
    if (request.method === 'GET' && request.url === '/healthz') {
      response.writeHead(200, { 'content-type': 'application/json' })
      response.end('{"status":"ok"}\n')
      return
    }
    if (request.method !== 'POST' || request.url !== config.server.webhookPath) {
      response.writeHead(404).end()
      return
    }

    try {
      const body = await readBody(request, 2 * 1024 * 1024)
      if (!verifyWebhookSignature(
        body,
        request.headers['x-hub-signature-256'],
        config.server.webhookSecret,
      )) {
        response.writeHead(401).end('invalid signature\n')
        return
      }

      const deliveryId = request.headers['x-github-delivery']
      if (!deliveryId || state.has(deliveryId)) {
        response.writeHead(202).end('ignored\n')
        return
      }
      const payload = JSON.parse(body.toString('utf8'))
      const trigger = processor.accept(request.headers['x-github-event'], payload)
      await state.mark(deliveryId)
      if (trigger) {
        processor.enqueue(trigger).catch(async (error) => {
          await state.forget(deliveryId)
          logger.error('deliberation failed', error)
        })
      }
      response.writeHead(202).end(trigger ? 'accepted\n' : 'ignored\n')
    } catch (error) {
      logger.error('webhook failed', error)
      response.writeHead(error.code === 'BODY_TOO_LARGE' ? 413 : 400).end('bad request\n')
    }
  })

  server.listen(config.server.port, config.server.host, () => {
    logger.log(
      `TT deliberation bot listening on http://${config.server.host}:${config.server.port}${config.server.webhookPath}`,
    )
  })
  return server
}

async function readBody(request, limit) {
  const chunks = []
  let size = 0
  for await (const chunk of request) {
    size += chunk.length
    if (size > limit) {
      const error = new Error('request body is too large')
      error.code = 'BODY_TOO_LARGE'
      throw error
    }
    chunks.push(chunk)
  }
  return Buffer.concat(chunks)
}
