import assert from 'node:assert/strict'
import { mkdtemp, readFile, writeFile } from 'node:fs/promises'
import { join } from 'node:path'
import { tmpdir } from 'node:os'
import test from 'node:test'

import { createProject, detectBundler, initializeExisting, run } from '../src/installer.js'

test('creates a complete Vite project without installing', async () => {
  const parent = await mkdtemp(join(tmpdir(), 'create-rl-new-'))
  const root = join(parent, 'hello-rl')
  const result = await createProject({ directory: root })
  const manifest = JSON.parse(await readFile(join(root, 'package.json'), 'utf8'))
  assert.equal(result.bundler, 'vite')
  assert.equal(result.packageManager, 'bun')
  assert.equal(manifest.scripts.check, 'rlc --check-types src')
  assert.equal(manifest.devDependencies['unplugin-rl'], 'latest')
  assert.match(await readFile(join(root, 'vite.config.ts'), 'utf8'), /unplugin-rl\/vite/)
  assert.equal(await readFile(join(root, 'src/main.ts'), 'utf8'), "import './app.rl'\n")
  assert.match(await readFile(join(root, 'src/app.rl'), 'utf8'), /match \(greeting\)/)
})

test('persists a selected local registry for a new Bun project', async () => {
  const parent = await mkdtemp(join(tmpdir(), 'create-rl-registry-'))
  const root = join(parent, 'local-app')
  await createProject({ directory: root, registry: 'http://127.0.0.1:4873/' })
  assert.equal(
    await readFile(join(root, 'bunfig.toml'), 'utf8'),
    '[install]\nregistry = "http://127.0.0.1:4873/"\n',
  )
})

test('detects the existing bundler from either dependency section', () => {
  assert.equal(detectBundler({ devDependencies: { vite: '^8' } }), 'vite')
  assert.equal(detectBundler({ dependencies: { '@rspack/core': '^2' } }), 'rspack')
  assert.equal(detectBundler({ dependencies: {} }), 'none')
})

test('initializes Vite through a wrapper and preserves the user config', async () => {
  const root = await mkdtemp(join(tmpdir(), 'create-rl-init-'))
  await writeFile(join(root, 'package.json'), JSON.stringify({
    scripts: { dev: 'vite' },
    devDependencies: { vite: '^8.0.0' },
  }, null, 4))
  const original = "export default { plugins: ['user-plugin'] }\n"
  await writeFile(join(root, 'vite.config.ts'), original)

  await initializeExisting({ directory: root, bundler: 'auto' })

  const manifest = JSON.parse(await readFile(join(root, 'package.json'), 'utf8'))
  assert.equal(await readFile(join(root, 'vite.config.ts'), 'utf8'), original)
  assert.equal(manifest.scripts.dev, 'vite')
  assert.match(manifest.scripts['rl:build'], /rl\.vite\.config\.mjs/)
  assert.equal(manifest.devDependencies['rl-lang'], 'latest')
  const wrapper = await readFile(join(root, 'rl.vite.config.mjs'), 'utf8')
  assert.match(wrapper, /import base from '.\/vite\.config\.ts'/)
  assert.match(wrapper, /plugins: \[rl\(\), \.\.\.\(config\.plugins/)
})

test('keeps esbuild scripts intact and returns an explicit manual hook', async () => {
  const root = await mkdtemp(join(tmpdir(), 'create-rl-esbuild-'))
  await writeFile(join(root, 'package.json'), JSON.stringify({
    scripts: { build: 'node build.mjs' },
    devDependencies: { esbuild: '^1.0.0' },
  }))
  const result = await initializeExisting({ directory: root, bundler: 'auto' })
  const manifest = JSON.parse(await readFile(join(root, 'package.json'), 'utf8'))
  assert.equal(result.manualModule, 'unplugin-rl/esbuild')
  assert.equal(manifest.scripts.build, 'node build.mjs')
  assert.equal(manifest.scripts['rl:check'], 'rlc --check-types src')
})

test('generates a composable wrapper for every declarative bundler adapter', async () => {
  const adapters = ['vite', 'rollup', 'rolldown', 'webpack', 'rspack', 'farm']
  for (const bundler of adapters) {
    const root = await mkdtemp(join(tmpdir(), `create-rl-${bundler}-`))
    await writeFile(join(root, 'package.json'), '{"scripts":{}}\n')
    const result = await initializeExisting({ directory: root, bundler })
    const wrapper = await readFile(join(root, result.files[0]), 'utf8')
    assert.match(wrapper, new RegExp(`unplugin-rl/${bundler}`))
    assert.match(wrapper, /plugins: \[rl\(\)/)
  }
})

test('does not allow a new project to drift from the Bun and Vite baseline', async () => {
  await assert.rejects(() => run(['app', '--package-manager', 'npm']), /new projects use Bun/)
  await assert.rejects(() => run(['app', '--bundler', 'webpack']), /new projects use Vite/)
})
