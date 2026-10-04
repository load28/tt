import { agentPrompt, mentionPrompt, moderatorPrompt, formatAgentComment, formatOutcome } from './deliberation-format.mjs'
export { formatAgentComment, formatOutcome } from './deliberation-format.mjs'

export class DeliberationEngine {
  constructor(config, { codex, github, state, logger = console }) {
    this.config = config
    this.codex = codex
    this.github = github
    this.state = state
    this.logger = logger
  }

  async deliberate(discussion, trigger) {
    const session = {
      discussionId: discussion.id,
      trigger,
      rounds: [],
      outcome: null,
    }
    let moderator = null

    for (let round = 1; round <= this.config.deliberation.maximumRounds; round += 1) {
      const responses = []
      for (const agent of this.config.agents) {
        const response = await this.codex.run(
          agentPrompt(this.config, agent, discussion, trigger, session, moderator, round),
          'agent-response.json',
        )
        responses.push({ agentId: agent.id, ...response })
        await this.github.addDiscussionComment(
          agent,
          discussion.id,
          formatAgentComment(agent, response, round),
        )
      }

      moderator = await this.codex.run(
        moderatorPrompt(this.config, discussion, session, responses, round),
        'moderator-response.json',
      )
      const unanimousAcceptance = responses.every(
        (response) => response.canAccept && response.criticalObjections.length === 0,
      )
      if (moderator.decision === 'consensus' && !unanimousAcceptance) {
        moderator.decision = 'continue'
      }
      if (round < this.config.deliberation.minimumRounds) moderator.decision = 'continue'
      if (round === this.config.deliberation.maximumRounds && moderator.decision === 'continue') {
        moderator.decision = 'human_required'
      }
      session.rounds.push({ number: round, responses, moderator })

      if (moderator.decision !== 'continue') {
        session.outcome = moderator
        await this.github.addDiscussionComment(
          this.config.controller,
          discussion.id,
          formatOutcome(moderator, round),
        )
        await this.state.recordSession(discussion.id, session)
        return session
      }
    }

    throw new Error('Deliberation finished without an outcome')
  }

  async answerMention(discussion, trigger, agents) {
    const replies = []
    for (const agent of agents) {
      const response = await this.codex.run(
        mentionPrompt(this.config, agent, discussion, trigger),
        'agent-response.json',
      )
      replies.push({ agentId: agent.id, ...response })
      await this.github.addDiscussionComment(
        agent,
        discussion.id,
        formatAgentComment(agent, response, null),
      )
    }
    return replies
  }
}
