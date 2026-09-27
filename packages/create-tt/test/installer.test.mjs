import assert from 'node:assert/strict'
import { existsSync } from 'node:fs'
import { mkdir, mkdtemp, readFile, readdir, writeFile } from 'node:fs/promises'
import { dirname, join } from 'node:path'
import { tmpdir } from 'node:os'
import { spawnSync } from 'node:child_process'
import { fileURLToPath } from 'node:url'
import test from 'node:test'

import { createProject, dependencyChannel, detectBundler, initializeExisting, parseJsonc, run, shellQuote } from '../src/installer.js'

const ownManifest = JSON.parse(await readFile(new URL('../package.json', import.meta.url), 'utf8'))
const expectedDependencyChannel = dependencyChannel(ownManifest.version)

/** What a scaffolded project installs: the exact TypeScript this repository
 * is tested against, read from its manifest rather than repeated here — a
 * user then runs the compiler against the version it was verified with
 * (TASK-256). `npm/scripts/typescript-version.test.mjs` holds the published
 * instructions to the same one. */
const repositoryManifest = JSON.parse(
  await readFile(new URL('../../../package.json', import.meta.url), 'utf8'),
)
const scaffoldedTypeScript = repositoryManifest.devDependencies.typescript

test('keeps dependencies on the installer release channel', () => {
  assert.equal(dependencyChannel('0.3.0-dev.20260826'), 'next')
  assert.equal(dependencyChannel('0.3.0-beta'), 'beta')
  assert.equal(dependencyChannel('0.3.0-beta.1'), 'beta')
  assert.equal(dependencyChannel('0.3.0-rc'), 'rc')
  assert.equal(dependencyChannel('0.3.0'), 'latest')
  assert.equal(dependencyChannel('0.0.0-dev'), 'latest')
})

test('creates a complete Vite project without installing', async () => {
  const parent = await mkdtemp(join(tmpdir(), 'create-tt-new-'))
  const root = join(parent, 'hello-tt')
  const result = await createProject({ directory: root })
  const manifest = JSON.parse(await readFile(join(root, 'package.json'), 'utf8'))
  assert.equal(result.bundler, 'vite')
  assert.equal(result.packageManager, 'bun')
  assert.equal(manifest.scripts.check, 'tsc -p tsconfig.json --runExternalCode')
  assert.equal(manifest.scripts.build, 'tsc -p tsconfig.json --runExternalCode && vite build')
  assert.equal(manifest.devDependencies['@openload28/unplugin-tt'], expectedDependencyChannel)
  assert.equal(manifest.devDependencies.typescript, scaffoldedTypeScript)
  assert.match(await readFile(join(root, 'vite.config.ts'), 'utf8'), /@openload28\/unplugin-tt\/vite/)
  assert.equal(await readFile(join(root, 'src/main.ts'), 'utf8'), "import './app.tt'\n")
  const config = JSON.parse(await readFile(join(root, 'tsconfig.json'), 'utf8'))
  assert.deepEqual(config.contentMappers, [
    { package: '@openload28/tt-lang', extensions: ['.tt', '.ttx'] },
  ])
  assert.equal(await readFile(join(root, '.gitignore'), 'utf8'), 'node_modules/\ndist/\n')
  const appSource = await readFile(join(root, 'src/app.tt'), 'utf8')
  assert.match(appSource, /variant Greeting/)
  assert.match(appSource, /match \(greeting\)/)
})

test('persists a selected local registry for a new Bun project', async () => {
  const parent = await mkdtemp(join(tmpdir(), 'create-tt-registry-'))
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
  const root = await mkdtemp(join(tmpdir(), 'create-tt-init-'))
  await writeFile(join(root, 'package.json'), JSON.stringify({
    scripts: { dev: 'vite' },
    devDependencies: { vite: '^8.0.0' },
  }, null, 4))
  const original = "export default { plugins: ['user-plugin'] }\n"
  await writeFile(join(root, 'vite.config.ts'), original)
  await writeFile(join(root, 'tsconfig.json'), JSON.stringify({ compilerOptions: { strict: true } }, null, 4))

  await initializeExisting({ directory: root, bundler: 'auto' })

  const manifest = JSON.parse(await readFile(join(root, 'package.json'), 'utf8'))
  assert.equal(await readFile(join(root, 'vite.config.ts'), 'utf8'), original)
  assert.equal(manifest.scripts.dev, 'vite')
  assert.equal(manifest.scripts['tt:check'], 'tsc -p tsconfig.tt.json --runExternalCode')
  assert.match(manifest.scripts['tt:build'], /tt\.vite\.config\.mjs/)
  assert.equal(manifest.devDependencies['@openload28/tt-lang'], expectedDependencyChannel)
  assert.equal(manifest.devDependencies.typescript, scaffoldedTypeScript)
  const wrapper = await readFile(join(root, 'tt.vite.config.mjs'), 'utf8')
  assert.match(wrapper, /import base from '.\/vite\.config\.ts'/)
  assert.match(wrapper, /plugins: \[tt\(\), \.\.\.\(config\.plugins/)
  assert.match(wrapper, /const addTt/)
  assert.doesNotMatch(wrapper, /addRl/)
  const config = JSON.parse(await readFile(join(root, 'tsconfig.tt.json'), 'utf8'))
  assert.equal(config.extends, './tsconfig.json')
  assert.deepEqual(config.compilerOptions, { noEmit: true })
  assert.equal(config.include, undefined)
  assert.deepEqual(config.contentMappers, [
    { package: '@openload28/tt-lang', extensions: ['.tt', '.ttx'] },
  ])
})

test('keeps esbuild scripts intact and returns an explicit manual hook', async () => {
  const root = await mkdtemp(join(tmpdir(), 'create-tt-esbuild-'))
  await writeFile(join(root, 'package.json'), JSON.stringify({
    scripts: { build: 'node build.mjs' },
    devDependencies: { esbuild: '^1.0.0' },
  }))
  const result = await initializeExisting({ directory: root, bundler: 'auto' })
  const manifest = JSON.parse(await readFile(join(root, 'package.json'), 'utf8'))
  assert.equal(result.manualModule, '@openload28/unplugin-tt/esbuild')
  assert.equal(manifest.scripts.build, 'node build.mjs')
  assert.equal(manifest.scripts['tt:check'], 'tsc -p tsconfig.tt.json --runExternalCode')
})

test('generates a composable wrapper for every declarative bundler adapter', async () => {
  const adapters = ['vite', 'rollup', 'rolldown', 'webpack', 'rspack', 'farm']
  for (const bundler of adapters) {
    const root = await mkdtemp(join(tmpdir(), `create-tt-${bundler}-`))
    await writeFile(join(root, 'package.json'), '{"scripts":{}}\n')
    const result = await initializeExisting({ directory: root, bundler })
    const wrapper = await readFile(join(root, result.files[0]), 'utf8')
    assert.match(wrapper, new RegExp(`@openload28/unplugin-tt/${bundler}`))
    assert.match(wrapper, /plugins: \[tt\(\)/)
    assert.doesNotMatch(wrapper, /addRl/)
  }
})

test('does not allow a new project to drift from the Bun and Vite baseline', async () => {
  await assert.rejects(() => run(['app', '--package-manager', 'npm']), /new projects use Bun/)
  await assert.rejects(() => run(['app', '--bundler', 'webpack']), /new projects use Vite/)
})


test('init preserves customized generated configs without partial writes', async () => {
  for (const file of ['tsconfig.tt.json', 'tt.vite.config.mjs']) {
    const root = await mkdtemp(join(tmpdir(), 'create-tt-conflict-'))
    const manifest = '{"devDependencies":{"vite":"^8"}}\n'
    const config = '// customized project configuration\n'
    await writeFile(join(root, 'package.json'), manifest)
    await writeFile(join(root, file), config)
    await assert.rejects(() => initializeExisting({ directory: root, bundler: 'auto' }), /refusing to overwrite existing config/)
    assert.equal(await readFile(join(root, 'package.json'), 'utf8'), manifest)
    assert.equal(await readFile(join(root, file), 'utf8'), config)
    assert.deepEqual((await readdir(root)).sort(), ['package.json', file].sort())
  }
})

test('repeated init is idempotent when generated configs are unchanged', async () => {
  const root = await mkdtemp(join(tmpdir(), 'create-tt-repeat-'))
  await writeFile(join(root, 'package.json'), '{"devDependencies":{"vite":"^8"}}\n')
  await initializeExisting({ directory: root, bundler: 'auto' })
  const before = await Promise.all(['package.json', 'tsconfig.tt.json', 'tt.vite.config.mjs'].map(file => readFile(join(root, file), 'utf8')))
  await initializeExisting({ directory: root, bundler: 'auto' })
  const after = await Promise.all(['package.json', 'tsconfig.tt.json', 'tt.vite.config.mjs'].map(file => readFile(join(root, file), 'utf8')))
  assert.deepEqual(after, before)
})

function installedTypeScript(directory) {
  const entry = join(directory, 'node_modules/typescript/lib/tsc.js')
  if (existsSync(entry)) return entry
  const parent = dirname(directory)
  return parent === directory ? undefined : installedTypeScript(parent)
}

const repositoryTypeScript = installedTypeScript(fileURLToPath(new URL('../../..', import.meta.url)))

async function viteSolutionProject() {
  const root = await mkdtemp(join(tmpdir(), 'create-tt-solution-'))
  await writeFile(join(root, 'package.json'), '{"devDependencies":{"vite":"^8.0.0"}}\n')
  await writeFile(join(root, 'tsconfig.json'), JSON.stringify({
    files: [],
    references: [{ path: './tsconfig.app.json' }, { path: './tsconfig.node.json' }],
  }, null, 2))
  await writeFile(join(root, 'tsconfig.app.json'), `{
  "compilerOptions": {
    "tsBuildInfoFile": "./node_modules/.tmp/tsconfig.app.tsbuildinfo",
    "target": "ES2022",
    "module": "ESNext",
    "skipLibCheck": true,

    /* Bundler mode */
    "moduleResolution": "bundler",
    "noEmit": true,
    // Linting
    "strict": true,
  },
  "include": ["src"]
}
`)
  await writeFile(join(root, 'tsconfig.node.json'), `{
  "compilerOptions": {
    "tsBuildInfoFile": "./node_modules/.tmp/tsconfig.node.tsbuildinfo",
    "module": "ESNext",
    "moduleResolution": "bundler",
    "skipLibCheck": true,
    "noEmit": true
  },
  "include": ["vite.config.ts"]
}
`)
  await writeFile(join(root, 'vite.config.ts'), 'export default {}\n')
  await mkdir(join(root, 'src'))
  await writeFile(join(root, 'src/main.ts'), 'export const label: string = 1\n')
  return root
}

test('init composes a solution tsconfig through its project references', async () => {
  const root = await viteSolutionProject()
  const result = await initializeExisting({ directory: root, bundler: 'auto' })
  const manifest = JSON.parse(await readFile(join(root, 'package.json'), 'utf8'))
  assert.equal(manifest.scripts['tt:check'], 'tsc -b tsconfig.tt.json --runExternalCode')
  assert.equal(manifest.scripts['tt:build'], 'tsc -b tsconfig.tt.json --runExternalCode && vite build --config tt.vite.config.mjs')
  const mapper = [{ package: '@openload28/tt-lang', extensions: ['.tt', '.ttx'] }]
  assert.deepEqual(JSON.parse(await readFile(join(root, 'tsconfig.tt.json'), 'utf8')), {
    extends: './tsconfig.json',
    compilerOptions: { noEmit: true },
    contentMappers: mapper,
    references: [{ path: './tsconfig.app.tt.json' }, { path: './tsconfig.node.tt.json' }],
  })
  for (const name of ['app', 'node']) {
    assert.deepEqual(JSON.parse(await readFile(join(root, `tsconfig.${name}.tt.json`), 'utf8')), {
      extends: `./tsconfig.${name}.json`,
      compilerOptions: { noEmit: true },
      contentMappers: mapper,
    })
  }
  assert.deepEqual(result.files.sort(), [
    'tsconfig.app.tt.json', 'tsconfig.node.tt.json', 'tsconfig.tt.json', 'tt.vite.config.mjs',
  ])
  await initializeExisting({ directory: root, bundler: 'auto' })
})

test('the generated solution check reaches the referenced sources', { skip: !repositoryTypeScript && 'the repository TypeScript is not installed' }, async () => {
  const root = await viteSolutionProject()
  await initializeExisting({ directory: root, bundler: 'auto' })
  const mapperPackage = join(root, 'node_modules/@openload28/tt-lang')
  await mkdir(mapperPackage, { recursive: true })
  await writeFile(join(mapperPackage, 'package.json'), JSON.stringify({
    name: '@openload28/tt-lang',
    version: '0.0.0',
    typescript: { contentMapper: { exec: ['ttc', '--content-mapper'] } },
  }))
  const manifest = JSON.parse(await readFile(join(root, 'package.json'), 'utf8'))
  const [, ...args] = manifest.scripts['tt:check'].split(' ')
  const check = spawnSync(process.execPath, [repositoryTypeScript, ...args], { cwd: root, encoding: 'utf8' })
  assert.notEqual(check.status, 0, check.stdout + check.stderr)
  assert.match(check.stdout, /src\/main\.ts\(1,14\): error TS2322/)
})

test('init replaces an incompatible TypeScript and reports it', async () => {
  for (const section of ['devDependencies', 'dependencies']) {
    const root = await mkdtemp(join(tmpdir(), 'create-tt-typescript-'))
    await writeFile(join(root, 'package.json'), JSON.stringify({ [section]: { typescript: '~5.8.0' } }))
    const result = await initializeExisting({ directory: root, bundler: 'none' })
    const manifest = JSON.parse(await readFile(join(root, 'package.json'), 'utf8'))
    assert.equal(manifest[section].typescript, scaffoldedTypeScript)
    assert.equal(Object.keys(manifest.devDependencies).includes('typescript'), section === 'devDependencies')
    assert.deepEqual(result.updated, [{ name: 'typescript', from: '~5.8.0', to: scaffoldedTypeScript }])
    const lines = []
    await run(['init', root, '--no-install', '--bundler', 'none'], { log: (line) => lines.push(line) })
    assert.ok(!lines.some((line) => line.startsWith('Updated typescript')), lines.join('\n'))
  }
  const root = await mkdtemp(join(tmpdir(), 'create-tt-typescript-report-'))
  await writeFile(join(root, 'package.json'), '{"devDependencies":{"typescript":"~5.8.0"}}\n')
  const lines = []
  await run(['init', root, '--no-install', '--bundler', 'none'], { log: (line) => lines.push(line) })
  assert.ok(lines.includes(`Updated typescript from ~5.8.0 to ${scaffoldedTypeScript}: tt's content mapper needs this TypeScript 7.1 build.`), lines.join('\n'))
})

test('create quotes the printed directory for a POSIX shell', async () => {
  const parent = await mkdtemp(join(tmpdir(), 'create-tt-quote-'))
  const lines = []
  const cwd = process.cwd()
  process.chdir(parent)
  try {
    await run(['ct app 2', '--no-install'], { log: (line) => lines.push(line) })
  } finally {
    process.chdir(cwd)
  }
  assert.ok(lines.includes("Run: cd 'ct app 2' && bun run dev"), lines.join('\n'))
  assert.equal(shellQuote('my-app'), 'my-app')
  assert.equal(shellQuote("it's here"), "'it'\\''s here'")
  assert.equal(shellQuote('$HOME;rm'), "'$HOME;rm'")
})

test('reads tsconfig comments and trailing commas without touching strings', () => {
  assert.deepEqual(
    parseJsonc('{\n  // line\n  "a": "x // y, }", /* block */\n  "b": [1, 2,],\n}\n'),
    { a: 'x // y, }', b: [1, 2] },
  )
})
