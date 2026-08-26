import { readFile } from 'node:fs/promises'
import { dirname, resolve } from 'node:path'

function requiredString(value, label) {
  if (typeof value !== 'string' || value.trim() === '') {
    throw new Error(`${label} must be a non-empty string`)
  }
  return value
}

function positiveInteger(value, label) {
  if (!Number.isInteger(value) || value < 1) {
    throw new Error(`${label} must be a positive integer`)
  }
  return value
}

export async function loadConfig(path, environment = process.env) {
  const absolutePath = resolve(path)
  const baseDirectory = dirname(absolutePath)
  const raw = JSON.parse(await readFile(absolutePath, 'utf8'))
  const agents = raw.agents ?? []
  if (!Array.isArray(agents) || agents.length < 2) {
    throw new Error('agents must contain at least two entries')
  }

  const minimumRounds = positiveInteger(raw.deliberation?.minimumRounds, 'minimumRounds')
  const maximumRounds = positiveInteger(raw.deliberation?.maximumRounds, 'maximumRounds')
  if (minimumRounds > maximumRounds) {
    throw new Error('minimumRounds cannot exceed maximumRounds')
  }

  const identities = [raw.controller, ...agents]
  for (const [index, identity] of identities.entries()) {
    const label = index === 0 ? 'controller' : `agents[${index - 1}]`
    requiredString(identity?.githubLogin, `${label}.githubLogin`)
    if (!identity?.appId && !identity?.appIdEnv) {
      throw new Error(`${label} must provide appId or appIdEnv`)
    }
    if (!identity?.privateKeyFile && !identity?.privateKeyEnv) {
      throw new Error(`${label} must provide privateKeyFile or privateKeyEnv`)
    }
  }

  if (!raw.server?.webhookSecretFile && !raw.server?.webhookSecretEnv) {
    throw new Error('server must provide webhookSecretFile or webhookSecretEnv')
  }

  const webhookSecret = raw.server.webhookSecretFile
    ? (await readFile(resolve(baseDirectory, raw.server.webhookSecretFile), 'utf8')).trim()
    : requiredString(
      environment[raw.server.webhookSecretEnv],
      `${raw.server.webhookSecretEnv} environment variable`,
    )
  return {
    ...raw,
    sourcePath: absolutePath,
    server: {
      host: raw.server.host ?? '127.0.0.1',
      port: positiveInteger(raw.server.port, 'server.port'),
      webhookPath: raw.server.webhookPath ?? '/github/webhook',
      stateDirectory: resolve(baseDirectory, raw.server.stateDirectory ?? 'state'),
      webhookSecret,
    },
    repository: {
      owner: requiredString(raw.repository?.owner, 'repository.owner'),
      name: requiredString(raw.repository?.name, 'repository.name'),
    },
    codex: {
      binary: raw.codex?.binary ?? 'codex',
      model: raw.codex?.model ?? '',
      profile: raw.codex?.profile ?? '',
      timeoutSeconds: positiveInteger(raw.codex?.timeoutSeconds ?? 300, 'codex.timeoutSeconds'),
    },
    deliberation: {
      language: raw.deliberation.language ?? 'ko',
      minimumRounds,
      maximumRounds,
    },
    controller: await materializeIdentity(raw.controller, environment, baseDirectory),
    agents: await Promise.all(agents.map(async (agent) => ({
      ...(await materializeIdentity(agent, environment, baseDirectory)),
      id: requiredString(agent.id, 'agent.id'),
      displayName: requiredString(agent.displayName, 'agent.displayName'),
      perspective: requiredString(agent.perspective, 'agent.perspective'),
    }))),
  }
}

async function materializeIdentity(identity, environment, baseDirectory) {
  const privateKey = identity.privateKeyFile
    ? await readFile(resolve(baseDirectory, identity.privateKeyFile), 'utf8')
    : requiredString(
      environment[identity.privateKeyEnv],
      `${identity.privateKeyEnv} environment variable`,
    ).replaceAll('\\n', '\n')
  return {
    ...identity,
    appId: requiredString(
      identity.appId ?? environment[identity.appIdEnv],
      identity.appId ? 'appId' : `${identity.appIdEnv} environment variable`,
    ),
    privateKey,
  }
}
