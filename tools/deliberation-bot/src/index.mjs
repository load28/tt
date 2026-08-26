import { resolve } from 'node:path'

import { CodexRunner } from './codex.mjs'
import { loadConfig } from './config.mjs'
import { DeliberationEngine } from './deliberation.mjs'
import { PullRequestReviewEngine } from './review.mjs'
import { GitHubClient } from './github.mjs'
import { EventProcessor, startServer } from './server.mjs'
import { StateStore } from './state.mjs'

const configPath = resolve(process.env.TT_DELIBERATION_CONFIG ?? 'config.json')
const config = await loadConfig(configPath)
const state = new StateStore(config.server.stateDirectory)
await state.initialize()

const github = new GitHubClient(config.repository)
const codex = new CodexRunner(config.codex)
const discussionEngine = new DeliberationEngine(config, { codex, github, state })
const reviewEngine = new PullRequestReviewEngine(config, { codex, github, state })
const engine = {
  deliberate: (...arguments_) => discussionEngine.deliberate(...arguments_),
  answerMention: (...arguments_) => discussionEngine.answerMention(...arguments_),
  review: (...arguments_) => reviewEngine.review(...arguments_),
}
const processor = new EventProcessor(config, { github, engine, state })

startServer(config, processor, state)
