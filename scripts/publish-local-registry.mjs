#!/usr/bin/env bun

import { cp, mkdtemp, readFile, writeFile } from 'node:fs/promises'
import { arch, platform, tmpdir } from 'node:os'
import { dirname, join, resolve } from 'node:path'
import { spawnSync } from 'node:child_process'
import { fileURLToPath } from 'node:url'

const root = resolve(dirname(fileURLToPath(import.meta.url)), '..')
const registry = registryUrl(process.argv[2] ?? 'http://127.0.0.1:4873/')
const cargo = await readFile(join(root, 'Cargo.toml'), 'utf8')
const baseVersion = cargo.match(/^version\s*=\s*"([^"]+)"/m)?.[1]
if (!baseVersion) fail('Cargo.toml has no package version')
const stamp = Date.now()
const compilerVersion = `${baseVersion}-local.${stamp}`
const packageKey = platformKey()
const executable = platform() === 'win32' ? 'rlc.exe' : 'rlc'
const binary = join(root, 'target', 'release', executable)
const staging = await mkdtemp(join(tmpdir(), 'rl-local-registry-'))
const packages = join(staging, 'packages')

await requireRegistry(registry)
run('cargo', ['build', '--release'], root)
run('node', [
  join(root, 'npm/scripts/make-platform-package.mjs'),
  packageKey,
  binary,
  packages,
  compilerVersion,
], root)

const rlLang = join(packages, 'rl-lang')
await cp(join(root, 'npm/rl-lang'), rlLang, {
  recursive: true,
  filter: (source) => !source.endsWith('rl-dev.local.json'),
})
await updateManifest(rlLang, (manifest) => {
  manifest.version = compilerVersion
  manifest.optionalDependencies = { [`rl-lang-${packageKey}`]: compilerVersion }
})

const unplugin = join(packages, 'unplugin-rl')
await cp(join(root, 'integrations/unplugin'), unplugin, {
  recursive: true,
  filter: (source) => !source.endsWith('node_modules'),
})
const unpluginBase = JSON.parse(await readFile(join(unplugin, 'package.json'), 'utf8')).version
await updateManifest(unplugin, (manifest) => {
  manifest.version = `${unpluginBase}-local.${stamp}`
})

const initializer = join(packages, 'create-rl')
await cp(join(root, 'packages/create-rl'), initializer, { recursive: true })
await updateManifest(initializer, (manifest) => {
  manifest.version = compilerVersion
})

for (const directory of [join(packages, `rl-lang-${packageKey}`), rlLang, unplugin, initializer]) {
  run('bun', ['publish', '--registry', registry, '--tag', 'latest'], directory, {
    BUN_CONFIG_REGISTRY: registry,
  })
}

console.log(`\nPublished local RL toolchain ${compilerVersion} to ${registry}`)
console.log(`BUN_CONFIG_REGISTRY=${registry} bunx create-rl@latest my-app --registry ${registry}`)

function run(command, args, cwd, environment = {}) {
  const result = spawnSync(command, args, {
    cwd,
    stdio: 'inherit',
    env: { ...process.env, ...environment },
  })
  if (result.error) throw result.error
  if (result.status !== 0) fail(`${command} exited with status ${result.status}`)
}

async function updateManifest(directory, update) {
  const path = join(directory, 'package.json')
  const manifest = JSON.parse(await readFile(path, 'utf8'))
  update(manifest)
  await writeFile(path, `${JSON.stringify(manifest, null, 2)}\n`)
}

async function requireRegistry(url) {
  try {
    const response = await fetch(new URL('/-/ping', url))
    if (response.ok) return
  } catch {}
  fail(`local registry is not reachable at ${url}\nstart it with: bunx verdaccio@6 --config scripts/verdaccio.local.yaml --listen 127.0.0.1:4873`)
}

function platformKey() {
  const os = { linux: 'linux', darwin: 'darwin', win32: 'win32' }[platform()]
  const cpu = { x64: 'x64', arm64: 'arm64' }[arch()]
  if (!os || !cpu || (os === 'win32' && cpu !== 'x64')) {
    fail(`no prebuilt rl-lang package target for ${platform()}-${arch()}`)
  }
  return `${os}-${cpu}`
}

function registryUrl(value) {
  const url = new URL(value)
  if (!['http:', 'https:'].includes(url.protocol)) fail('registry URL must use http or https')
  return url.href
}

function fail(message) {
  console.error(`publish-local-registry: ${message}`)
  process.exit(1)
}
