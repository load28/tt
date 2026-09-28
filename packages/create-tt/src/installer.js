import { existsSync } from 'node:fs'
import { lstat, mkdir, readFile, readdir, readlink, realpath, writeFile } from 'node:fs/promises'
import { basename, dirname, isAbsolute, join, relative, resolve, sep } from 'node:path'
import { spawnSync } from 'node:child_process'

const ownManifest = JSON.parse(await readFile(new URL('../package.json', import.meta.url), 'utf8'))
const ttChannel = dependencyChannel(ownManifest.version)
const versions = {
  '@openload28/tt-lang': ttChannel,
  '@openload28/unplugin-tt': ttChannel,
  vite: '^8.0.0',
  // ttc drives the TypeScript the project installs and looks nowhere else,
  // so a scaffolded project has to install one — the exact version this
  // repository tests against, which is the whole point: a user runs the
  // compiler against the TypeScript it was verified with, and an upstream
  // nightly cannot change their build overnight. Not `7`: declaration
  // output (`ttc --types`, the editor's `.tt.d.ts` sidecars) needs the emit
  // API that arrived in 7.1, and `7` resolves to the 7.0 line because
  // ranges do not match prereleases (TASK-256). When 7.1 is released this
  // moves to `7` everywhere at once, held by
  // npm/scripts/typescript-version.test.mjs.
  typescript: "7.1.0-dev.20260826.1",
}

export function dependencyChannel(packageVersion) {
  if (/-dev\./.test(packageVersion)) return 'next'
  if (/-beta(?:\.|$)/.test(packageVersion)) return 'beta'
  if (/-rc(?:\.|$)/.test(packageVersion)) return 'rc'
  return 'latest'
}

const bundlers = {
  vite: {
    packages: ['vite'],
    configs: ['vite.config.ts', 'vite.config.js', 'vite.config.mts', 'vite.config.mjs'],
    module: '@openload28/unplugin-tt/vite',
    wrapper: 'tt.vite.config.mjs',
    commands: { dev: 'vite', build: 'vite build' },
  },
  rollup: {
    packages: ['rollup'],
    configs: ['rollup.config.ts', 'rollup.config.js', 'rollup.config.mjs'],
    module: '@openload28/unplugin-tt/rollup',
    wrapper: 'tt.rollup.config.mjs',
    commands: { dev: 'rollup --watch', build: 'rollup' },
  },
  rolldown: {
    packages: ['rolldown'],
    configs: ['rolldown.config.ts', 'rolldown.config.js', 'rolldown.config.mjs'],
    module: '@openload28/unplugin-tt/rolldown',
    wrapper: 'tt.rolldown.config.mjs',
    commands: { dev: 'rolldown --watch', build: 'rolldown' },
  },
  webpack: {
    packages: ['webpack'],
    configs: ['webpack.config.ts', 'webpack.config.js', 'webpack.config.cjs', 'webpack.config.mjs'],
    module: '@openload28/unplugin-tt/webpack',
    wrapper: 'tt.webpack.config.mjs',
    commands: { dev: 'webpack serve', build: 'webpack' },
  },
  rspack: {
    packages: ['@rspack/core'],
    configs: ['rspack.config.ts', 'rspack.config.js', 'rspack.config.mjs'],
    module: '@openload28/unplugin-tt/rspack',
    wrapper: 'tt.rspack.config.mjs',
    commands: { dev: 'rspack serve', build: 'rspack build' },
  },
  farm: {
    packages: ['@farmfe/core'],
    configs: ['farm.config.ts', 'farm.config.js', 'farm.config.mjs'],
    module: '@openload28/unplugin-tt/farm',
    wrapper: 'tt.farm.config.mjs',
    commands: { dev: 'farm start', build: 'farm build' },
  },
  esbuild: {
    packages: ['esbuild'],
    configs: [],
    module: '@openload28/unplugin-tt/esbuild',
  },
}

export async function run(argv, io = console) {
  const options = parseArguments(argv)
  if (options.help) {
    io.log(help)
    return
  }
  const result = options.command === 'init'
    ? await initializeExisting(options)
    : await createProject(options)

  if (options.install) installDependencies(result.root, result.packageManager, options.registry)
  printResult(result, io)
}

export async function createProject(options) {
  const root = resolve(options.directory ?? 'my-tt-app')
  if (existsSync(root) && (await readdir(root)).length > 0) {
    throw new Error(`target directory is not empty: ${root}`)
  }
  await mkdir(join(root, 'src'), { recursive: true })
  const packageManager = options.packageManager ?? 'bun'
  const name = packageName(basename(root))
  const manifest = {
    name,
    private: true,
    version: '0.0.0',
    type: 'module',
    scripts: {
      dev: 'vite',
      build: 'tsc -p tsconfig.json --runExternalCode && vite build',
      check: 'tsc -p tsconfig.json --runExternalCode',
    },
    devDependencies: {
      '@openload28/tt-lang': versions['@openload28/tt-lang'],
      '@openload28/unplugin-tt': versions['@openload28/unplugin-tt'],
      typescript: versions.typescript,
      vite: versions.vite,
    },
  }
  await writeJson(join(root, 'package.json'), manifest)
  await writeJson(join(root, 'tsconfig.json'), tsconfig())
  await writeFile(join(root, 'vite.config.ts'), viteConfig)
  await writeFile(join(root, 'index.html'), indexHtml)
  await writeFile(join(root, 'src/main.ts'), "import './app.tt'\n")
  await writeFile(join(root, 'src/app.tt'), starterSource)
  await writeFile(join(root, '.gitignore'), 'node_modules/\ndist/\n')
  if (options.registry) {
    await writeFile(join(root, 'bunfig.toml'), `[install]\nregistry = ${JSON.stringify(options.registry)}\n`)
  }
  return { root, packageManager, mode: 'create', bundler: 'vite', files: ['src/main.ts', 'src/app.tt', 'vite.config.ts'] }
}

export async function initializeExisting(options) {
  const root = resolve(options.directory ?? '.')
  const manifestPath = join(root, 'package.json')
  if (!existsSync(manifestPath)) throw new Error(`package.json not found in ${root}`)

  const source = await readFile(manifestPath, 'utf8')
  const manifest = JSON.parse(source)
  const packageManager = options.packageManager ?? detectPackageManager(root, manifest)
  const bundler = options.bundler === 'auto' ? detectBundler(manifest) : options.bundler
  const devDependencies = manifest.devDependencies ?? {}
  devDependencies['@openload28/tt-lang'] ??= versions['@openload28/tt-lang']
  const updated = []
  const typescriptSection = manifest.dependencies?.typescript !== undefined
    ? manifest.dependencies
    : devDependencies
  const declaredTypeScript = typescriptSection.typescript
  if (declaredTypeScript !== versions.typescript) {
    typescriptSection.typescript = versions.typescript
    if (declaredTypeScript !== undefined) {
      updated.push({ name: 'typescript', from: declaredTypeScript, to: versions.typescript })
    }
  }
  manifest.devDependencies = devDependencies
  manifest.scripts ??= {}
  const typeConfig = 'tsconfig.tt.json'
  const generated = []
  const realRoot = await realpath(root)
  const references = existsSync(join(root, 'tsconfig.json'))
    ? await typeConfigGraph(realRoot, await projectConfig(realRoot, join(root, 'tsconfig.json')), generated)
    : (generated.push([typeConfig, `${JSON.stringify(tsconfig(), null, '  ')}\n`]), false)
  const typeCheck = `tsc ${references ? '-b' : '-p'} ${typeConfig} --runExternalCode`
  manifest.scripts['tt:check'] ??= typeCheck

  const files = []
  let manualModule
  if (bundler && bundler !== 'none') {
    devDependencies['@openload28/unplugin-tt'] ??= versions['@openload28/unplugin-tt']
    const adapter = bundlers[bundler]
    if (!adapter) throw new Error(`unsupported bundler: ${bundler}`)
    const base = adapter.configs.find((file) => existsSync(join(root, file)))
    if (bundler === 'esbuild') {
      manualModule = adapter.module
    } else {
      generated.push([adapter.wrapper, wrapperConfig(adapter.module, base)])
      files.push(adapter.wrapper)
      manifest.scripts['tt:dev'] ??= `${adapter.commands.dev} --config ${adapter.wrapper}`
      manifest.scripts['tt:build'] ??= `${typeCheck} && ${adapter.commands.build} --config ${adapter.wrapper}`
    }
  } else {
    manifest.scripts['tt:build'] ??= 'ttc -o .tt-build src'
  }

  // Validate the complete output set before changing any project files.
  // Identical generated files make repeated init safe; customized files
  // require an explicit user decision outside the initializer.
  for (const [file, content] of generated) {
    const path = join(root, file)
    await assertInsideProject(realRoot, path)
    if (existsSync(path) && await readFile(path, 'utf8') !== content) {
      throw new Error(`refusing to overwrite existing config: ${path}`)
    }
  }
  for (const [file, content] of generated) await writeFile(join(root, file), content)
  await writeJson(manifestPath, manifest, indentation(source))
  files.push(...generated.map(([file]) => file).filter((file) => !files.includes(file)))
  return { root, packageManager, mode: 'init', bundler: bundler ?? 'none', files, manualModule, updated }
}

async function typeConfigGraph(root, configPath, generated, visited = new Set()) {
  visited.add(configPath)
  let config
  try {
    config = parseJsonc(await readFile(configPath, 'utf8'))
  } catch (error) {
    throw new Error(`cannot read ${configPath}: ${error.message}`)
  }
  const directory = dirname(configPath)
  const counterpart = configPath.replace(/\.json$/, '') + '.tt.json'
  const slot = generated.push(undefined) - 1
  const content = tsconfig(`./${basename(configPath)}`)
  const hasReferences = Array.isArray(config.references)
  if (hasReferences) {
    const references = []
    for (const reference of config.references) {
      const lexical = typeof reference?.path === 'string' && referencedConfig(directory, reference.path)
      const target = lexical && existsSync(lexical) && await realpath(lexical)
      if (!target || !insideRoot(root, target)) {
        references.push(reference)
        continue
      }
      if (!visited.has(target)) await typeConfigGraph(root, target, generated, visited)
      const referenced = target.replace(/\.json$/, '') + '.tt.json'
      references.push({ ...reference, path: relativePath(directory, referenced) })
    }
    content.references = references
  }
  generated[slot] = [relative(root, counterpart), `${JSON.stringify(content, null, '  ')}\n`]
  return hasReferences
}

async function projectConfig(root, path) {
  const config = await realpath(path)
  if (!insideRoot(root, config)) throw new Error(`refusing to follow ${path}: it resolves outside the project to ${config}`)
  return config
}

async function assertInsideProject(root, path) {
  const target = await writtenPath(path, new Set())
  if (!insideRoot(root, target)) throw new Error(`refusing to write ${path}: it resolves outside the project to ${target}`)
}

async function writtenPath(path, seen) {
  if (seen.has(path)) throw new Error(`refusing to write ${path}: its symbolic links form a cycle`)
  seen.add(path)
  const entry = await lstat(path).catch(() => null)
  if (entry?.isSymbolicLink()) return writtenPath(resolve(dirname(path), await readlink(path)), seen)
  return entry ? realpath(path) : join(await realpath(dirname(path)), basename(path))
}

function referencedConfig(directory, path) {
  const target = resolve(directory, path)
  return target.endsWith('.json') ? target : join(target, 'tsconfig.json')
}

function insideRoot(root, path) {
  const fromRoot = relative(root, path)
  return fromRoot !== '' && fromRoot !== '..' && !fromRoot.startsWith(`..${sep}`) && !isAbsolute(fromRoot)
}

function relativePath(from, to) {
  const path = relative(from, to).split('\\').join('/')
  return path === '..' || path.startsWith('../') ? path : `./${path}`
}

export function parseJsonc(source) {
  let text = ''
  let pendingComma = false
  let index = 0
  while (index < source.length) {
    const char = source[index]
    if (source.startsWith('//', index)) {
      while (index < source.length && source[index] !== '\n') index += 1
      continue
    }
    if (source.startsWith('/*', index)) {
      const end = source.indexOf('*/', index + 2)
      index = end === -1 ? source.length : end + 2
      continue
    }
    if (/\s/.test(char)) {
      text += char
      index += 1
      continue
    }
    if (pendingComma && char !== '}' && char !== ']') text += ','
    pendingComma = false
    if (char === ',') {
      pendingComma = true
      index += 1
    } else if (char === '"') {
      let end = index + 1
      while (end < source.length && source[end] !== '"') end += source[end] === '\\' ? 2 : 1
      text += source.slice(index, end + 1)
      index = end + 1
    } else {
      text += char
      index += 1
    }
  }
  return JSON.parse(text)
}

export function detectBundler(manifest) {
  const dependencies = { ...manifest.dependencies, ...manifest.devDependencies }
  return Object.entries(bundlers).find(([, adapter]) =>
    adapter.packages.some((name) => name in dependencies))?.[0] ?? 'none'
}

function parseArguments(argv) {
  const args = [...argv]
  const options = {
    command: args[0] === 'init' ? args.shift() : 'create',
    install: true,
    bundler: 'auto',
  }
  while (args.length) {
    const arg = args.shift()
    if (arg === '--help' || arg === '-h') options.help = true
    else if (arg === '--no-install') options.install = false
    else if (arg === '--bundler') options.bundler = requiredValue(arg, args)
    else if (arg === '--package-manager') options.packageManager = requiredValue(arg, args)
    else if (arg === '--registry') options.registry = registryUrl(requiredValue(arg, args))
    else if (!arg.startsWith('-') && !options.directory) options.directory = arg
    else throw new Error(`unknown argument: ${arg}`)
  }
  if (!['auto', 'none', ...Object.keys(bundlers)].includes(options.bundler)) {
    throw new Error(`unsupported bundler: ${options.bundler}`)
  }
  if (options.packageManager && !['npm', 'pnpm', 'yarn', 'bun'].includes(options.packageManager)) {
    throw new Error(`unsupported package manager: ${options.packageManager}`)
  }
  if (options.command === 'create' && options.packageManager && options.packageManager !== 'bun') {
    throw new Error('new projects use Bun; --package-manager only applies to init')
  }
  if (options.command === 'create' && !['auto', 'vite'].includes(options.bundler)) {
    throw new Error('new projects use Vite; --bundler only selects vite for create')
  }
  return options
}

function requiredValue(flag, args) {
  const value = args.shift()
  if (!value || value.startsWith('-')) throw new Error(`${flag} needs a value`)
  return value
}

function detectPackageManager(root, manifest = {}) {
  const declared = typeof manifest.packageManager === 'string'
    ? manifest.packageManager.split('@')[0]
    : undefined
  if (['npm', 'pnpm', 'yarn', 'bun'].includes(declared)) return declared
  const locks = [['pnpm-lock.yaml', 'pnpm'], ['yarn.lock', 'yarn'], ['bun.lock', 'bun'], ['bun.lockb', 'bun']]
  return locks.find(([file]) => existsSync(join(root, file)))?.[1] ?? 'npm'
}

function installDependencies(root, packageManager, registry) {
  const args = ['install']
  if (registry) args.push('--registry', registry)
  const result = spawnSync(packageManager, args, { cwd: root, stdio: 'inherit' })
  if (result.error) throw result.error
  if (result.status !== 0) throw new Error(`${packageManager} install exited with status ${result.status}`)
}

function registryUrl(value) {
  let url
  try {
    url = new URL(value)
  } catch {
    throw new Error(`invalid registry URL: ${value}`)
  }
  if (!['http:', 'https:'].includes(url.protocol)) throw new Error('registry URL must use http or https')
  if (url.username || url.password) throw new Error('pass registry credentials through Bun configuration or environment variables')
  return url.href
}

function printResult(result, io) {
  const location = relative(process.cwd(), result.root) || '.'
  io.log(result.mode === 'create' ? `Created a tt project in ${location}.` : `Added tt to ${location}.`)
  if (result.files.length) io.log(`Generated: ${result.files.join(', ')}`)
  for (const { name, from, to } of result.updated ?? []) {
    io.log(`Updated ${name} from ${from} to ${to}: tt's content mapper needs this TypeScript 7.1 build.`)
  }
  if (result.manualModule) {
    io.log(`Add tt() from ${result.manualModule} to your esbuild plugins array.`)
  }
  io.log(result.mode === 'create' ? `Run: cd ${shellQuote(location)} && ${runScript(result.packageManager, 'dev')}` : `Run: ${runScript(result.packageManager, 'tt:check')}`)
}

export function shellQuote(word) {
  return /^[A-Za-z0-9_@%+=:,.\/-]+$/.test(word) ? word : `'${word.replaceAll("'", "'\\''")}'`
}

function runScript(packageManager, script) {
  return packageManager === 'npm' ? `npm run ${script}` : `${packageManager} run ${script}`
}

function wrapperConfig(moduleName, base) {
  const baseImport = base ? `import base from './${base}'\n` : 'const base = {}\n'
  return `import tt from '${moduleName}'\n${baseImport}
const addTt = (config = {}) => Array.isArray(config)
  ? config.map(addTt)
  : { ...config, plugins: [tt(), ...(config.plugins ?? [])] }

export default typeof base === 'function'
  ? async (...args) => addTt(await base(...args))
  : Promise.resolve(base).then(addTt)
`
}

function tsconfig(extendsConfig) {
  const config = {
    ...(extendsConfig
      ? { extends: extendsConfig, compilerOptions: { noEmit: true } }
      : {
          compilerOptions: {
            target: 'ES2022',
            module: 'Preserve',
            moduleResolution: 'Bundler',
            strict: true,
            noEmit: true,
            skipLibCheck: true,
          },
          include: ['src', 'vite.config.ts'],
        }),
    contentMappers: [
      { package: '@openload28/tt-lang', extensions: ['.tt', '.ttx'] },
    ],
  }
  return config
}

function packageName(value) {
  const normalized = value.toLowerCase().replace(/[^a-z0-9._-]+/g, '-').replace(/^-+|-+$/g, '')
  return normalized || 'my-tt-app'
}

function indentation(source) {
  return source.match(/\n([ \t]+)\S/)?.[1] ?? '  '
}

async function writeJson(path, value, space = '  ') {
  await writeFile(path, `${JSON.stringify(value, null, space)}\n`)
}

const viteConfig = `import { defineConfig } from 'vite'
import tt from '@openload28/unplugin-tt/vite'

export default defineConfig({
  plugins: [tt()],
})
`

const indexHtml = `<!doctype html>
<html lang="en">
  <head><meta charset="UTF-8"><meta name="viewport" content="width=device-width, initial-scale=1.0"><title>tt app</title></head>
  <body><main id="app"></main><script type="module" src="/src/main.ts"></script></body>
</html>
`

const starterSource = `variant Greeting {
  Hello(name: string),
  Welcome,
}

const greeting = Greeting.Hello('tt')
document.querySelector<HTMLDivElement>('#app')!.textContent = match (greeting) {
  Hello(name) => \`Hello, \${name}!\`,
  Welcome => 'Welcome!',
}
`

const help = `Create a new tt project or add tt to an existing TypeScript project.

Usage:
  bunx @openload28/create-tt@next [directory] [--no-install]
  bunx @openload28/create-tt@next init [directory] [--bundler vite]

Options:
  --bundler <auto|none|vite|rollup|rolldown|webpack|rspack|esbuild|farm>
  --package-manager <npm|pnpm|yarn|bun>
  --registry <url>     install from an npm-compatible private/local registry
  --no-install
  -h, --help`
