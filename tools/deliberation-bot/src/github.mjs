import { createHmac, createSign, timingSafeEqual } from 'node:crypto'

const githubApiVersion = '2022-11-28'

function base64url(value) {
  return Buffer.from(value).toString('base64url')
}

export function createAppJwt(appId, privateKey, now = Date.now()) {
  const issuedAt = Math.floor(now / 1000) - 60
  const header = base64url(JSON.stringify({ alg: 'RS256', typ: 'JWT' }))
  const payload = base64url(JSON.stringify({
    iat: issuedAt,
    exp: issuedAt + 9 * 60,
    iss: appId,
  }))
  const unsigned = `${header}.${payload}`
  const signature = createSign('RSA-SHA256').update(unsigned).sign(privateKey, 'base64url')
  return `${unsigned}.${signature}`
}

export function verifyWebhookSignature(body, signature, secret) {
  if (typeof signature !== 'string' || !signature.startsWith('sha256=')) return false
  const expected = `sha256=${createHmac('sha256', secret).update(body).digest('hex')}`
  const actualBuffer = Buffer.from(signature)
  const expectedBuffer = Buffer.from(expected)
  return actualBuffer.length === expectedBuffer.length
    && timingSafeEqual(actualBuffer, expectedBuffer)
}

export class GitHubClient {
  constructor(repository, { fetchImpl = fetch, now = Date.now } = {}) {
    this.repository = repository
    this.fetchImpl = fetchImpl
    this.now = now
    this.tokens = new Map()
  }

  async installationToken(identity) {
    const cached = this.tokens.get(identity.appId)
    if (cached && cached.expiresAt - this.now() > 5 * 60 * 1000) return cached.token

    const jwt = createAppJwt(identity.appId, identity.privateKey, this.now())
    const installation = await this.request(
      `/repos/${this.repository.owner}/${this.repository.name}/installation`,
      { token: jwt },
    )
    const access = await this.request(`/app/installations/${installation.id}/access_tokens`, {
      method: 'POST',
      token: jwt,
    })
    this.tokens.set(identity.appId, {
      token: access.token,
      expiresAt: Date.parse(access.expires_at),
    })
    return access.token
  }

  async fetchDiscussion(identity, discussionId) {
    const token = await this.installationToken(identity)
    const data = await this.graphql(token, `
      query DiscussionForDeliberation($id: ID!) {
        node(id: $id) {
          ... on Discussion {
            id
            title
            body
            url
            author { login }
            comments(last: 100) {
              nodes {
                id
                body
                createdAt
                author { login }
                replies(first: 100) {
                  nodes { id body createdAt author { login } }
                }
              }
            }
          }
        }
      }
    `, { id: discussionId })
    if (!data.node) throw new Error(`Discussion ${discussionId} was not found`)
    return data.node
  }

  async addDiscussionComment(identity, discussionId, body) {
    const token = await this.installationToken(identity)
    const data = await this.graphql(token, `
      mutation AddDeliberationComment($discussionId: ID!, $body: String!) {
        addDiscussionComment(input: { discussionId: $discussionId, body: $body }) {
          comment { id url }
        }
      }
    `, { discussionId, body })
    return data.addDiscussionComment.comment
  }

  async graphql(token, query, variables) {
    const response = await this.fetchImpl('https://api.github.com/graphql', {
      method: 'POST',
      headers: this.headers(token),
      body: JSON.stringify({ query, variables }),
    })
    const value = await response.json()
    if (!response.ok || value.errors) {
      throw new Error(`GitHub GraphQL failed: ${JSON.stringify(value.errors ?? value)}`)
    }
    return value.data
  }

  async request(path, { method = 'GET', token } = {}) {
    const response = await this.fetchImpl(`https://api.github.com${path}`, {
      method,
      headers: this.headers(token),
    })
    const value = await response.json()
    if (!response.ok) throw new Error(`GitHub API ${response.status}: ${JSON.stringify(value)}`)
    return value
  }

  headers(token) {
    return {
      accept: 'application/vnd.github+json',
      authorization: `Bearer ${token}`,
      'x-github-api-version': githubApiVersion,
      'user-agent': 'tt-deliberation-bot',
    }
  }
}
