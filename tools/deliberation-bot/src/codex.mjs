import { spawn } from 'node:child_process'
import { mkdtemp, readFile, rm } from 'node:fs/promises'
import { tmpdir } from 'node:os'
import { join, resolve } from 'node:path'

const schemaDirectory = resolve(new URL('../schemas', import.meta.url).pathname)

export class CodexRunner {
  constructor(config, { spawnProcess = spawn } = {}) {
    this.config = config
    this.spawnProcess = spawnProcess
  }

  async run(prompt, schemaName) {
    const directory = await mkdtemp(join(tmpdir(), 'tt-deliberation-'))
    const outputPath = join(directory, 'response.json')
    const schemaPath = join(schemaDirectory, schemaName)
    const args = [
      'exec',
      '--ephemeral',
      '--sandbox',
      'read-only',
      '--skip-git-repo-check',
      '--output-schema',
      schemaPath,
      '--output-last-message',
      outputPath,
      '-',
    ]
    if (this.config.model) args.splice(1, 0, '--model', this.config.model)
    if (this.config.profile) args.splice(1, 0, '--profile', this.config.profile)

    try {
      await runChild(
        this.spawnProcess,
        this.config.binary,
        args,
        prompt,
        this.config.timeoutSeconds * 1000,
      )
      return JSON.parse(await readFile(outputPath, 'utf8'))
    } finally {
      await rm(directory, { recursive: true, force: true })
    }
  }
}

function runChild(spawnProcess, binary, args, input, timeoutMs) {
  return new Promise((resolvePromise, reject) => {
    const child = spawnProcess(binary, args, { stdio: ['pipe', 'pipe', 'pipe'] })
    let stderr = ''
    let stdout = ''
    const timer = setTimeout(() => {
      child.kill('SIGTERM')
      reject(new Error(`Codex timed out after ${timeoutMs}ms`))
    }, timeoutMs)

    child.stdout.on('data', (chunk) => { stdout += chunk })
    child.stderr.on('data', (chunk) => { stderr += chunk })
    child.once('error', (error) => {
      clearTimeout(timer)
      reject(error)
    })
    child.once('close', (code) => {
      clearTimeout(timer)
      if (code === 0) resolvePromise()
      else reject(new Error(`Codex exited with ${code}: ${stderr || stdout}`))
    })
    child.stdin.end(input)
  })
}
