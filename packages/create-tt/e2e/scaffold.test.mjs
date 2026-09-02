import assert from 'node:assert/strict'
import { access, mkdir, mkdtemp, readFile, rm, writeFile } from 'node:fs/promises'
import { constants } from 'node:fs'
import { tmpdir } from 'node:os'
import { dirname, join, resolve } from 'node:path'
import { fileURLToPath } from 'node:url'
import { spawnSync } from 'node:child_process'
import test from 'node:test'

import { createProject } from '../src/installer.js'

const repositoryRoot = resolve(dirname(fileURLToPath(import.meta.url)), '../../..')
const compiler = process.env.TTC_BINARY ?? join(
  repositoryRoot,
  'target',
  'debug',
  process.platform === 'win32' ? 'ttc.exe' : 'ttc',
)

function run(command, args, cwd, temporaryDirectory) {
  const result = spawnSync(command, args, {
    cwd,
    encoding: 'utf8',
    env: {
      ...process.env,
      TTC_BINARY: compiler,
      TMPDIR: temporaryDirectory,
      BUN_INSTALL_CACHE_DIR: join(temporaryDirectory, 'bun-cache'),
    },
  })
  assert.equal(
    result.status,
    0,
    [
      `${command} ${args.join(' ')} exited with ${result.status}`,
      result.stdout,
      result.stderr,
    ].filter(Boolean).join('\n'),
  )
}

test('a freshly resolved Bun scaffold type-checks and builds', { timeout: 180_000 }, async () => {
  await access(compiler, constants.X_OK)
  const parent = await mkdtemp(join(tmpdir(), 'create-tt-e2e-'))
  const root = join(parent, 'app')

  try {
    await createProject({ directory: root })

    // Exercise the unpublished code in this checkout while leaving Vite and
    // its native transitive dependencies to a real fresh registry resolution.
    const manifestPath = join(root, 'package.json')
    const manifest = JSON.parse(await readFile(manifestPath, 'utf8'))
    manifest.devDependencies['@openload28/tt-lang'] = `file:${join(repositoryRoot, 'npm/tt-lang')}`
    manifest.devDependencies['@openload28/unplugin-tt'] = `file:${join(repositoryRoot, 'integrations/unplugin')}`
    await writeFile(manifestPath, `${JSON.stringify(manifest, null, 2)}\n`)

    const temporaryDirectory = join(root, '.tmp')
    await mkdir(temporaryDirectory)
    run('bun', ['install'], root, temporaryDirectory)
    await access(join(root, 'bun.lock'), constants.R_OK)
    run('bun', ['run', 'check'], root, temporaryDirectory)
    run('bun', ['run', 'build'], root, temporaryDirectory)

    await assert.rejects(access(join(root, '.tt-types'), constants.F_OK))
  } finally {
    await rm(parent, { recursive: true, force: true })
  }
})
