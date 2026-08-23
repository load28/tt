import { existsSync } from 'node:fs'
import { mkdir, readFile, readdir, writeFile } from 'node:fs/promises'
import { basename, join, relative, resolve } from 'node:path'
import { spawnSync } from 'node:child_process'

const ownManifest = JSON.parse(await readFile(new URL('../package.json', import.meta.url), 'utf8'))
const ttChannel = dependencyChannel(ownManifest.version)
const versions = {
  'tt-lang': ttChannel,
  'unplugin-tt': ttChannel,
  typescript: '^7.0.0',
  vite: '^8.0.0',
}

export function dependencyChannel(packageVersion) {
  return /-dev\./.test(packageVersion) ? 'dev' : 'latest'
}

const bundlers = {
  vite: {
    packages: ['vite'],
    configs: ['vite.config.ts', 'vite.config.js', 'vite.config.mts', 'vite.config.mjs'],
    module: 'unplugin-tt/vite',
    wrapper: 'tt.vite.config.mjs',
    commands: { dev: 'vite', build: 'vite build' },
  },
  rollup: {
    packages: ['rollup'],
    configs: ['rollup.config.ts', 'rollup.config.js', 'rollup.config.mjs'],
    module: 'unplugin-tt/rollup',
    wrapper: 'tt.rollup.config.mjs',
    commands: { dev: 'rollup --watch', build: 'rollup' },
  },
  rolldown: {
    packages: ['rolldown'],
    configs: ['rolldown.config.ts', 'rolldown.config.js', 'rolldown.config.mjs'],
    module: 'unplugin-tt/rolldown',
    wrapper: 'tt.rolldown.config.mjs',
    commands: { dev: 'rolldown --watch', build: 'rolldown' },
  },
  webpack: {
    packages: ['webpack'],
    configs: ['webpack.config.ts', 'webpack.config.js', 'webpack.config.cjs', 'webpack.config.mjs'],
    module: 'unplugin-tt/webpack',
    wrapper: 'tt.webpack.config.mjs',
    commands: { dev: 'webpack serve', build: 'webpack' },
  },
  rspack: {
    packages: ['@rspack/core'],
    configs: ['rspack.config.ts', 'rspack.config.js', 'rspack.config.mjs'],
    module: 'unplugin-tt/rspack',
    wrapper: 'tt.rspack.config.mjs',
    commands: { dev: 'rspack serve', build: 'rspack build' },
  },
  farm: {
    packages: ['@farmfe/core'],
    configs: ['farm.config.ts', 'farm.config.js', 'farm.config.mjs'],
    module: 'unplugin-tt/farm',
    wrapper: 'tt.farm.config.mjs',
    commands: { dev: 'farm start', build: 'farm build' },
  },
  esbuild: {
    packages: ['esbuild'],
    configs: [],
    module: 'unplugin-tt/esbuild',
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
      build: 'ttc --check-types src && vite build',
      check: 'ttc --check-types src',
    },
    devDependencies: {
      'tt-lang': versions['tt-lang'],
      'unplugin-tt': versions['unplugin-tt'],
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
  await writeFile(join(root, '.gitignore'), 'node_modules/\ndist/\n.tt-types/\n')
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
  devDependencies['tt-lang'] ??= versions['tt-lang']
  devDependencies.typescript ??= versions.typescript
  manifest.devDependencies = devDependencies
  manifest.scripts ??= {}
  manifest.scripts['tt:check'] ??= 'ttc --check-types src'

  const files = []
  let manualModule
  if (bundler && bundler !== 'none') {
    devDependencies['unplugin-tt'] ??= versions['unplugin-tt']
    const adapter = bundlers[bundler]
    if (!adapter) throw new Error(`unsupported bundler: ${bundler}`)
    const base = adapter.configs.find((file) => existsSync(join(root, file)))
    if (bundler === 'esbuild') {
      manualModule = adapter.module
    } else {
      await writeFile(join(root, adapter.wrapper), wrapperConfig(adapter.module, base))
      files.push(adapter.wrapper)
      manifest.scripts['tt:dev'] ??= `${adapter.commands.dev} --config ${adapter.wrapper}`
      manifest.scripts['tt:build'] ??= `ttc --check-types src && ${adapter.commands.build} --config ${adapter.wrapper}`
    }
  } else {
    manifest.scripts['tt:build'] ??= 'ttc -o .tt-build src'
  }

  await writeJson(manifestPath, manifest, indentation(source))
  return { root, packageManager, mode: 'init', bundler: bundler ?? 'none', files, manualModule }
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
  if (result.manualModule) {
    io.log(`Add tt() from ${result.manualModule} to your esbuild plugins array.`)
  }
  io.log(result.mode === 'create' ? `Run: cd ${location} && ${runScript(result.packageManager, 'dev')}` : `Run: ${runScript(result.packageManager, 'tt:check')}`)
}

function runScript(packageManager, script) {
  return packageManager === 'npm' ? `npm run ${script}` : `${packageManager} run ${script}`
}

function wrapperConfig(moduleName, base) {
  const baseImport = base ? `import base from './${base}'\n` : 'const base = {}\n'
  return `import tt from '${moduleName}'\n${baseImport}
const addRl = (config = {}) => Array.isArray(config)
  ? config.map(addRl)
  : { ...config, plugins: [tt(), ...(config.plugins ?? [])] }

export default typeof base === 'function'
  ? async (...args) => addRl(await base(...args))
  : Promise.resolve(base).then(addRl)
`
}

function tsconfig() {
  return {
    compilerOptions: {
      target: 'ES2022',
      module: 'Preserve',
      moduleResolution: 'Bundler',
      strict: true,
      noEmit: true,
      skipLibCheck: true,
    },
    include: ['src', 'vite.config.ts'],
  }
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
import tt from 'unplugin-tt/vite'

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

const starterSource = `enum Greeting {
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
  bun create tt@latest [directory] [--no-install]
  bun create tt@latest init [directory] [--bundler vite]

Options:
  --bundler <auto|none|vite|rollup|rolldown|webpack|rspack|esbuild|farm>
  --package-manager <npm|pnpm|yarn|bun>
  --registry <url>     install from an npm-compatible private/local registry
  --no-install
  -h, --help`
