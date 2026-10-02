import assert from 'node:assert/strict'
import { existsSync } from 'node:fs'
import { mkdir, readFile, readdir, symlink, writeFile } from 'node:fs/promises'
import { dirname, join } from 'node:path'
import { spawnSync } from 'node:child_process'
import { fileURLToPath } from 'node:url'
import test from 'node:test'

import { testDir } from '../../../scripts/test-dirs.cjs'

import { createProject, dependencyChannel, detectBundler, initializeExisting, packageName, parseJsonc, run, shellQuote } from '../src/installer.js'

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
  const parent = testDir('create-tt-new-')
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
  const parent = testDir('create-tt-registry-')
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
  const root = testDir('create-tt-init-')
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
  const root = testDir('create-tt-esbuild-')
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
    const root = testDir(`create-tt-${bundler}-`)
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
    const root = testDir('create-tt-conflict-')
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
  const root = testDir('create-tt-repeat-')
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
  const root = testDir('create-tt-solution-')
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

test('init keeps a reference that leaves the project through a symlink as written', async () => {
  const outside = testDir('create-tt-outside-')
  await writeFile(join(outside, 'tsconfig.json'), '{"compilerOptions":{"strict":true}}\n')
  const root = testDir('create-tt-linked-')
  await writeFile(join(root, 'package.json'), '{}\n')
  await mkdir(join(root, 'packages'))
  await symlink(outside, join(root, 'packages/app'), 'dir')
  await writeFile(join(root, 'tsconfig.json'), JSON.stringify({ files: [], references: [{ path: './packages/app' }] }))
  const result = await initializeExisting({ directory: root, bundler: 'none' })
  assert.equal(existsSync(join(outside, 'tsconfig.tt.json')), false)
  assert.deepEqual(JSON.parse(await readFile(join(root, 'tsconfig.tt.json'), 'utf8')).references, [{ path: './packages/app' }])
  assert.deepEqual(result.files, ['tsconfig.tt.json'])
})

test('init refuses a root tsconfig that resolves outside the project and writes nothing', async () => {
  const outside = testDir('create-tt-outside-root-')
  await writeFile(join(outside, 'tsconfig.json'), '{"compilerOptions":{"strict":true}}\n')
  const root = testDir('create-tt-linked-root-')
  await writeFile(join(root, 'package.json'), '{}\n')
  await symlink(join(outside, 'tsconfig.json'), join(root, 'tsconfig.json'))
  await assert.rejects(initializeExisting({ directory: root, bundler: 'none' }), /resolves outside the project/)
  assert.deepEqual((await readdir(outside)).sort(), ['tsconfig.json'])
  assert.deepEqual(await readFile(join(root, 'package.json'), 'utf8'), '{}\n')
})

test('init refuses to write a generated file through a symlink that leaves the project', async () => {
  const outside = testDir('create-tt-outside-file-')
  const root = testDir('create-tt-linked-file-')
  await writeFile(join(root, 'package.json'), '{}\n')
  await writeFile(join(root, 'tsconfig.json'), '{"compilerOptions":{"strict":true}}\n')
  await symlink(join(outside, 'target.json'), join(root, 'tsconfig.tt.json'))
  await assert.rejects(initializeExisting({ directory: root, bundler: 'none' }), /resolves outside the project/)
  assert.deepEqual(await readdir(outside), [])
})

test('init refuses to update a package.json that resolves outside the project and writes nothing', async () => {
  const outside = testDir('create-tt-outside-manifest-')
  await writeFile(join(outside, 'package.json'), '{}\n')
  const root = testDir('create-tt-linked-manifest-')
  await symlink(join(outside, 'package.json'), join(root, 'package.json'))
  await assert.rejects(initializeExisting({ directory: root, bundler: 'none' }), /resolves outside the project/)
  assert.equal(await readFile(join(outside, 'package.json'), 'utf8'), '{}\n')
  assert.deepEqual(await readdir(root), ['package.json'])
})

test('init treats a directory whose name begins with two dots as inside the project', async () => {
  const root = testDir('create-tt-dotted-')
  await writeFile(join(root, 'package.json'), '{}\n')
  await mkdir(join(root, '..cache'))
  await writeFile(join(root, '..cache/tsconfig.json'), '{"compilerOptions":{"strict":true}}\n')
  await writeFile(join(root, 'tsconfig.json'), JSON.stringify({ files: [], references: [{ path: './..cache' }] }))
  await initializeExisting({ directory: root, bundler: 'none' })
  assert.deepEqual(JSON.parse(await readFile(join(root, 'tsconfig.tt.json'), 'utf8')).references, [{ path: './..cache/tsconfig.tt.json' }])
  assert.equal(existsSync(join(root, '..cache/tsconfig.tt.json')), true)
})

test('init visits a config reached through two paths once', async () => {
  const root = testDir('create-tt-aliased-')
  await writeFile(join(root, 'package.json'), '{}\n')
  await mkdir(join(root, 'app'))
  await writeFile(join(root, 'app/tsconfig.json'), '{"compilerOptions":{"strict":true}}\n')
  await symlink(join(root, 'app'), join(root, 'alias'), 'dir')
  await writeFile(join(root, 'tsconfig.json'), JSON.stringify({ files: [], references: [{ path: './app' }, { path: './alias' }] }))
  const result = await initializeExisting({ directory: root, bundler: 'none' })
  assert.deepEqual(JSON.parse(await readFile(join(root, 'tsconfig.tt.json'), 'utf8')).references, [
    { path: './app/tsconfig.tt.json' },
    { path: './app/tsconfig.tt.json' },
  ])
  assert.deepEqual(result.files.sort(), ['app/tsconfig.tt.json', 'tsconfig.tt.json'])
})

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

async function referencedProjectGraph() {
  const root = testDir('create-tt-references-')
  await writeFile(join(root, 'package.json'), '{}\n')
  await writeFile(join(root, 'tsconfig.json'), JSON.stringify({ files: [], references: [{ path: './a' }, { path: './b' }] }))
  for (const [name, config, source] of [
    ['a', { compilerOptions: { composite: true, strict: true, outDir: 'dist', tsBuildInfoFile: 'dist/a.tsbuildinfo', types: [] }, include: ['src'] }, 'export const a = 1\n'],
    ['b', { compilerOptions: { composite: true, strict: true, outDir: 'dist', types: [] }, include: ['src'], references: [{ path: '../a' }] }, "import { a } from '../../a/src/a.js'\nexport const b: string = a\n"],
  ]) {
    await mkdir(join(root, name, 'src'), { recursive: true })
    await writeFile(join(root, name, 'tsconfig.json'), JSON.stringify(config))
    await writeFile(join(root, name, 'src', `${name}.ts`), source)
  }
  return root
}

test('init lets a project another project references emit declarations into a cache', async () => {
  const root = await referencedProjectGraph()
  await initializeExisting({ directory: root, bundler: 'none' })
  const mapper = [{ package: '@openload28/tt-lang', extensions: ['.tt', '.ttx'] }]
  assert.deepEqual(JSON.parse(await readFile(join(root, 'a/tsconfig.tt.json'), 'utf8')), {
    extends: './tsconfig.json',
    compilerOptions: {
      noEmit: false,
      emitDeclarationOnly: true,
      outDir: '../node_modules/.cache/tt/a',
      declarationDir: '../node_modules/.cache/tt/a',
      tsBuildInfoFile: '../node_modules/.cache/tt/a/tsconfig.tt.tsbuildinfo',
    },
    contentMappers: mapper,
  })
  assert.deepEqual(JSON.parse(await readFile(join(root, 'b/tsconfig.tt.json'), 'utf8')), {
    extends: './tsconfig.json',
    compilerOptions: { noEmit: true },
    contentMappers: mapper,
    references: [{ path: '../a/tsconfig.tt.json' }],
  })
  await initializeExisting({ directory: root, bundler: 'none' })
})

test('the generated graph builds when a referenced project is referenced by another', { skip: !repositoryTypeScript && 'the repository TypeScript is not installed' }, async () => {
  const root = await referencedProjectGraph()
  await initializeExisting({ directory: root, bundler: 'none' })
  const mapperPackage = join(root, 'node_modules/@openload28/tt-lang')
  await mkdir(mapperPackage, { recursive: true })
  await writeFile(join(mapperPackage, 'package.json'), JSON.stringify({
    name: '@openload28/tt-lang',
    version: '0.0.0',
    typescript: { contentMapper: { exec: ['ttc', '--content-mapper'] } },
  }))
  const manifest = JSON.parse(await readFile(join(root, 'package.json'), 'utf8'))
  const [, ...args] = manifest.scripts['tt:check'].split(' ')
  assert.deepEqual(args, ['-b', 'tsconfig.tt.json', '--runExternalCode'])
  const check = () => spawnSync(process.execPath, [repositoryTypeScript, ...args], { cwd: root, encoding: 'utf8' })
  const failed = check()
  assert.notEqual(failed.status, 0, failed.stdout + failed.stderr)
  assert.doesNotMatch(failed.stdout, /TS6310/)
  assert.match(failed.stdout, /b\/src\/b\.ts\(2,14\): error TS2322/)
  await writeFile(join(root, 'b/src/b.ts'), "import { a } from '../../a/src/a.js'\nexport const b: number = a\n")
  const passed = check()
  assert.equal(passed.status, 0, passed.stdout + passed.stderr)
  assert.equal(existsSync(join(root, 'a/dist')), false)
})

const repositoryCompiler = fileURLToPath(new URL(`../../../target/debug/${process.platform === 'win32' ? 'ttc.exe' : 'ttc'}`, import.meta.url))

test('init derives tt:build from the source roots the configuration includes', async () => {
  const graph = await referencedProjectGraph()
  await initializeExisting({ directory: graph, bundler: 'none' })
  const script = (root) => readFile(join(root, 'package.json'), 'utf8').then((text) => JSON.parse(text).scripts['tt:build'])
  assert.equal(await script(graph), 'ttc -o .tt-build/a/src a/src && ttc -o .tt-build/b/src b/src')

  const inherited = testDir('create-tt-build-extends-')
  await writeFile(join(inherited, 'package.json'), '{}\n')
  await writeFile(join(inherited, 'tsconfig.base.json'), '{ "include": ["lib/**/*"] }\n')
  await writeFile(join(inherited, 'tsconfig.json'), '{ "extends": "./tsconfig.base.json", "compilerOptions": { "strict": true } }\n')
  await mkdir(join(inherited, 'lib'))
  await initializeExisting({ directory: inherited, bundler: 'none' })
  assert.equal(await script(inherited), 'ttc -o .tt-build lib')

  const unconfigured = testDir('create-tt-build-default-')
  await writeFile(join(unconfigured, 'package.json'), '{}\n')
  await mkdir(join(unconfigured, 'src'))
  await initializeExisting({ directory: unconfigured, bundler: 'none' })
  assert.equal(await script(unconfigured), 'ttc -o .tt-build src')
})

test('a derived multi-root tt:build keeps imports between the roots', { skip: !existsSync(repositoryCompiler) && 'the repository compiler is not built' }, async () => {
  const root = await referencedProjectGraph()
  await writeFile(join(root, 'a/src/t.tt'), 'export const t = 1 |> ((x: number) => x)\n')
  await writeFile(join(root, 'b/src/u.tt'), "import { t } from '../../a/src/t.tt'\nexport const u = t\n")
  await initializeExisting({ directory: root, bundler: 'none' })
  const manifest = JSON.parse(await readFile(join(root, 'package.json'), 'utf8'))
  for (const command of manifest.scripts['tt:build'].split(' && ')) {
    const [, ...args] = command.split(' ')
    const built = spawnSync(repositoryCompiler, args, { cwd: root, encoding: 'utf8' })
    assert.equal(built.status, 0, built.stderr)
  }
  assert.match(await readFile(join(root, '.tt-build/b/src/u.ts'), 'utf8'), /from '\.\.\/\.\.\/a\/src\/t\.js'/)
  assert.equal(existsSync(join(root, '.tt-build/a/src/t.ts')), true)
})

test('init replaces an incompatible TypeScript and reports it', async () => {
  for (const section of ['devDependencies', 'dependencies']) {
    const root = testDir('create-tt-typescript-')
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
  const root = testDir('create-tt-typescript-report-')
  await writeFile(join(root, 'package.json'), '{"devDependencies":{"typescript":"~5.8.0"}}\n')
  const lines = []
  await run(['init', root, '--no-install', '--bundler', 'none'], { log: (line) => lines.push(line) })
  assert.ok(lines.includes(`Updated typescript from ~5.8.0 to ${scaffoldedTypeScript}: tt's content mapper needs this TypeScript 7.1 build.`), lines.join('\n'))
})

test('create quotes the printed directory for a POSIX shell', async () => {
  const parent = testDir('create-tt-quote-')
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

test('names a project with a name npm accepts for a new package', async () => {
  const cases = [
    ['hello-tt', 'hello-tt'],
    ['Hello World', 'hello-world'],
    ['.hidden', 'hidden'],
    ['_private', 'private'],
    ['-.-_x', 'x'],
    ['node_modules', 'my-tt-app'],
    ['favicon.ico', 'my-tt-app'],
    ['http', 'my-tt-app'],
    ['...', 'my-tt-app'],
    ['a'.repeat(300), 'a'.repeat(214)],
  ]
  for (const [directory, name] of cases) assert.equal(packageName(directory), name, directory)
  const parent = testDir('create-tt-names-')
  const root = join(parent, '.dotted')
  await createProject({ directory: root })
  const manifest = JSON.parse(await readFile(join(root, 'package.json'), 'utf8'))
  assert.equal(manifest.name, 'dotted')
})

test('takes every argument after -- as the directory', async () => {
  const parent = testDir('create-tt-dashdash-')
  const cwd = process.cwd()
  process.chdir(parent)
  try {
    await run(['--no-install', '--', '-dash'], { log() {} })
    await assert.rejects(() => run(['--', 'one', 'two']), /unknown argument: two/)
  } finally {
    process.chdir(cwd)
  }
  const manifest = JSON.parse(await readFile(join(parent, '-dash', 'package.json'), 'utf8'))
  assert.equal(manifest.name, 'dash')
})
