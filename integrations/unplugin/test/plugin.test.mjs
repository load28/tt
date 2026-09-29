import assert from 'node:assert/strict'
import { existsSync } from 'node:fs'
import { chmod, realpath, writeFile } from 'node:fs/promises'
import { dirname, isAbsolute, join } from 'node:path'
import test from 'node:test'

import { testDir } from '../../../scripts/test-dirs.cjs'

import {
  esbuildPlugin,
  farmPlugin,
  rolldownPlugin,
  rollupPlugin,
  rspackPlugin,
  unpluginFactory,
  vitePlugin,
  webpackPlugin,
} from '../index.js'

const compiler = process.env.TTC_BINARY

function context() {
  const watched = []
  return {
    watched,
    addWatchFile(file) { watched.push(file) },
    error(message) { throw new Error(message) },
  }
}

test('every published adapter is constructible from the shared plugin', () => {
  for (const adapter of [
    vitePlugin,
    rollupPlugin,
    rolldownPlugin,
    webpackPlugin,
    rspackPlugin,
    esbuildPlugin,
    farmPlugin,
  ]) {
    assert.equal(typeof adapter, 'function')
    assert.ok(adapter({ compiler }), 'adapter returned no plugin')
  }
})

test('the shared hooks resolve and compile tt, ttx, and standard modules', async () => {
  assert.ok(compiler, 'TTC_BINARY must name the compiler under test')
  const root = testDir('unplugin-tt-')
  const importer = join(root, 'entry.ts')
  const tt = join(root, 'shape.tt')
  const ttx = join(root, 'view.ttx')
  await writeFile(tt, 'export variant Shape { Circle(radius: number), Point }\n')
  await writeFile(ttx, 'export const View = () => <main>tt</main>;\n')

  const plugin = unpluginFactory({ compiler, sourcemap: true })
  const ttId = plugin.resolveId('./shape.tt', importer)
  const ttxId = plugin.resolveId('./view.ttx', importer)
  assert.equal(ttId, `${tt}?lang.ts`)
  assert.equal(ttxId, `${ttx}?lang.tsx`)

  const ttContext = context()
  const compiledTt = await plugin.load.call(ttContext, ttId)
  assert.match(compiledTt.code, /export type Shape/)
  assert.deepEqual(compiledTt.map.sources, [tt])
  assert.equal(compiledTt.map.sourceRoot, undefined)
  assert.ok(ttContext.watched.includes(tt))
  assert.ok(ttContext.watched.includes(await realpath(ttx)))

  const compiledTtx = await plugin.load.call(context(), ttxId)
  assert.match(compiledTtx.code, /<main>tt<\/main>/)
  assert.equal(plugin.esbuild.loader('', ttxId), 'tsx')
  assert.equal(plugin.esbuild.loader('', ttId), 'ts')

  const stdId = plugin.resolveId('@tt/std/result')
  const std = await plugin.load.call(context(), stdId)
  assert.match(std.code, /export const Ok/)
  assert.equal(std.map, null)
})

test('entry and root-relative tt specifiers resolve like the host resolves them', async (t) => {
  const root = testDir('unplugin-tt-entry-')
  const plugin = unpluginFactory({ compiler })
  const previous = process.cwd()
  process.chdir(root)
  t.after(() => process.chdir(previous))
  assert.equal(plugin.resolveId('./src/main.tt'), `${join(await realpath(root), 'src/main.tt')}?lang.ts`)

  const requests = []
  const host = { async resolve(...args) { requests.push(args); return { id: join(root, 'src/main.tt') } } }
  const resolved = await plugin.resolveId.call(host, '/src/main.tt', join(root, 'index.html'))
  assert.equal(resolved.id, `${join(root, 'src/main.tt')}?lang.ts`)
  assert.deepEqual(requests, [['/src/main.tt', join(root, 'index.html'), { skipSelf: true }]])
})

test('a CRLF source keeps its line endings and hands its map to the host', async () => {
  assert.ok(compiler, 'TTC_BINARY must name the compiler under test')
  const root = testDir('unplugin-tt-crlf-')
  const file = join(root, 'crlf.tt')
  await writeFile(file, 'variant V { A, B }\r\nexport const f = (v: V) => match (v) { A => 1, B => 2 };\r\n')

  const plugin = unpluginFactory({ compiler, sourcemap: true })
  const compiled = await plugin.load.call(context(), `${file}?lang.ts`)
  assert.ok(compiled.map, 'the inline map was not detached')
  assert.equal(compiled.map.sources[0], file)
  assert.doesNotMatch(compiled.code, /sourceMappingURL/)
  assert.match(compiled.code, /\r\n$/)
})

test('source map sources are anchored to the compiled file, honouring sourceRoot', async () => {
  const root = testDir('unplugin-tt-map-')
  const file = join(root, 'src', 'lib.tt')
  const fake = join(root, 'ttc.mjs')
  const map = { version: 3, sourceRoot: '../shared', sources: ['lib.tt', null], names: [], mappings: 'AAAA' }
  await writeFile(fake, `#!/usr/bin/env node
import { createInterface } from "node:readline";
createInterface({ input: process.stdin }).on("line", (line) => {
  const { id, method, params } = JSON.parse(line);
  const result = method === "dependencies"
    ? { files: [params.path], directories: [] }
    : { code: "export const a = 1;\\n//# sourceMappingURL=data:application/json;charset=utf-8;base64," + ${JSON.stringify(Buffer.from(JSON.stringify(map)).toString('base64'))} + "\\n", messages: [] };
  process.stdout.write(JSON.stringify({ id, result }) + "\\n");
});
`)
  await chmod(fake, 0o755)
  const plugin = unpluginFactory({ compiler: fake })
  const compiled = await plugin.load.call(context(), `${file}?lang.ts`)
  assert.equal(compiled.code, 'export const a = 1;\n')
  assert.deepEqual(compiled.map.sources, [join(root, 'shared', 'lib.tt'), null])
  assert.equal('sourceRoot' in compiled.map, false)
})

test('sourcemap false is a working public option and diagnostics reach the host', async () => {
  assert.ok(compiler, 'TTC_BINARY must name the compiler under test')
  const root = testDir('unplugin-tt-errors-')
  const file = join(root, 'bad.tt')
  await writeFile(
    file,
    'variant State { Ready, Empty }\ndeclare const state: State;\nexport const value = match (state) { Ready => 1 };\n',
  )

  const plugin = unpluginFactory({ compiler, sourcemap: false })
  await assert.rejects(
    plugin.load.call(context(), `${file}?lang.ts`),
    /error\[match-not-exhaustive\]/,
  )

  await writeFile(file, 'export const value = 1;\n')
  const compiled = await plugin.load.call(context(), `${file}?lang.ts`)
  assert.equal(compiled.map, null)
  assert.doesNotMatch(compiled.code, /sourceMappingURL/)
})

test('bare tt specifiers use host package exports and preserve external decisions', async () => {
  const plugin = unpluginFactory({ compiler })
  const requests = []
  const host = { async resolve(...args) { requests.push(args); return { id: '/workspace/packages/domain/model.tt', meta: { host: true } } } }
  const result = await plugin.resolveId.call(host, '@acme/domain/model.tt', '/workspace/app/main.tt?lang.ts')
  assert.equal(result.id, '/workspace/packages/domain/model.tt?lang.ts')
  assert.deepEqual(result.meta, { host: true })
  assert.deepEqual(requests, [['@acme/domain/model.tt', '/workspace/app/main.tt?lang.ts', { skipSelf: true }]])
  const external = { id: '@acme/domain/model.tt', external: true }
  assert.equal(await plugin.resolveId.call({ resolve: async () => external }, external.id, '/app/main.tt?lang.ts'), external)
  const javascript = { id: '/workspace/packages/domain/model.js' }
  assert.equal(await plugin.resolveId.call({ resolve: async () => javascript }, external.id, '/app/main.tt?lang.ts'), javascript)
})

test('query-suffixed tt imports keep their query and the real file before it', async () => {
  assert.ok(compiler, 'TTC_BINARY must name the compiler under test')
  const root = testDir('unplugin-tt-query-')
  const worker = join(root, 'worker.tt')
  const importer = join(root, 'main.tt')
  await writeFile(worker, 'variant M { Ping, Pong }\nconst m: M = M.Ping;\nexport const reply = match (m) { Ping => "ping", Pong => "pong" };\n')

  const plugin = unpluginFactory({ compiler })
  const workerFile = plugin.resolveId('./worker.tt?worker_file&type=module', `${importer}?lang.ts`)
  assert.equal(workerFile, `${worker}?worker_file&type=module&lang.ts`)
  assert.equal(plugin.resolveId(workerFile), workerFile)
  assert.equal(plugin.resolveId(`${worker}?lang.ts`), `${worker}?lang.ts`)
  const compiled = await plugin.load.call(context(), workerFile)
  assert.match(compiled.code, /const reply/)
  assert.equal(plugin.esbuild.loader('', workerFile), 'ts')
  assert.ok(plugin.esbuild.onLoadFilter.test(workerFile))

  for (const query of ['?worker', '?sharedworker', '?worker&inline', '?worker&url', '?raw', '?url', '?import&raw']) {
    assert.equal(plugin.resolveId(`./worker.tt${query}`, importer), null, query)
    assert.equal(await plugin.load.call(context(), `${worker}${query}`), null, query)
  }

  const requests = []
  const host = { async resolve(...args) { requests.push(args); return { id: worker } } }
  const served = await plugin.resolveId.call(host, '/src/worker.tt?worker_file&type=module', undefined)
  assert.equal(served.id, `${worker}?worker_file&type=module&lang.ts`)
  assert.deepEqual(requests, [['/src/worker.tt', undefined, { skipSelf: true }]])
})

test('the Vite dependency scanner reads tt modules from files that exist', async () => {
  assert.ok(compiler, 'TTC_BINARY must name the compiler under test')
  const root = testDir('unplugin-tt-scan-')
  const source = join(root, 'lib.tt')
  const view = join(root, 'view.ttx')
  await writeFile(source, 'import dep from "scan-dep";\nimport { Some } from "@tt/std/option";\nexport const value: number = dep.answer;\nexport const some = Some(value);\n')
  await writeFile(view, 'export const View = () => <main>tt</main>;\n')

  const plugin = unpluginFactory({ compiler })
  const id = plugin.resolveId('./lib.tt', join(root, 'main.ts'))
  assert.ok(existsSync(id.replace(/[?#][\s\S]*$/, '')), 'the scanner reads the id without its query from disk')
  assert.equal(isAbsolute(plugin.resolveId('@tt/std/option')), false, 'the scanner externalizes non-absolute ids')

  const esbuildConfig = plugin.vite.config.call(undefined)
  assert.deepEqual(esbuildConfig.optimizeDeps.extensions, ['.tt', '.ttx'])
  const loads = []
  esbuildConfig.optimizeDeps.esbuildOptions.plugins[0].setup({ onLoad: (options, callback) => loads.push({ options, callback }) })
  assert.equal(loads.length, 1)
  assert.ok(loads[0].options.filter.test(source))
  assert.ok(loads[0].options.filter.test(id))
  const scanned = await loads[0].callback({ path: source })
  assert.equal(scanned.loader, 'ts')
  assert.equal(scanned.resolveDir, root)
  assert.match(scanned.contents, /from "scan-dep"/)
  assert.match(scanned.contents, /from "@tt\/std\/option"/)
  assert.equal((await loads[0].callback({ path: view })).loader, 'tsx')

  const rolldownConfig = plugin.vite.config.call({ meta: { rolldownVersion: '1.0.0' } })
  assert.equal(rolldownConfig.optimizeDeps.esbuildOptions, undefined)
  const [rolldownScan] = rolldownConfig.optimizeDeps.rolldownOptions.plugins
  const loaded = await rolldownScan.load(id)
  assert.equal(loaded.moduleType, 'ts')
  assert.match(loaded.code, /from "scan-dep"/)
  assert.equal(await rolldownScan.load(join(root, 'main.ts')), null)
})

test('type-only dependencies invalidate cached modules even with HMR disabled', async t => {
  const root = testDir('unplugin-tt-watch-')
  const source = join(root, 'main.tt')
  const model = join(root, 'model.tt')
  await writeFile(model, 'export variant State { Ready(value: number), Empty }')
  await writeFile(source, 'import type {State} from "./model.tt"; export function render(state:State){return match(state){Ready(value)=>value,Empty=>0};}')
  const plugin = unpluginFactory({ compiler })
  const id = plugin.resolveId(source)
  const loadContext = context()
  await plugin.load.call(loadContext, id)
  const dependency = await realpath(model)
  assert.ok(loadContext.watched.includes(dependency))
  const module = { id }
  const invalidated = []
  const graph = { getModuleById: key => key === id ? module : undefined, invalidateModule: module => invalidated.push(module) }
  plugin.vite.configureServer({ config: { root, server: { hmr: false } }, watcher: { add() {} }, environments: { client: { moduleGraph: graph } } })
  await writeFile(model, 'export variant State { Ready(value: number), Empty, Loading }')
  plugin.watchChange(dependency)
  assert.deepEqual(invalidated, [module])
  await assert.rejects(() => plugin.load.call(context(), id), /missing.*Loading/)
})

/**
 * A compiler whose `dependencies` answer names `files` beside the module's
 * own file and `directories`, and which compiles every module to one line.
 */
async function dependencyCompiler(root, files, directories) {
  const fake = join(root, 'ttc.mjs')
  await writeFile(fake, `#!/usr/bin/env node
import { createInterface } from "node:readline";
createInterface({ input: process.stdin }).on("line", (line) => {
  const { id, method, params } = JSON.parse(line);
  const result = method === "dependencies"
    ? { files: [params.path, ...${JSON.stringify(files)}], directories: ${JSON.stringify(directories)} }
    : { code: "export const a = 1;\\n", messages: [] };
  process.stdout.write(JSON.stringify({ id, result }) + "\\n");
});
`)
  await chmod(fake, 0o755)
  return fake
}

test('dependency files and directories are registered as each host defines them', async () => {
  const root = testDir('unplugin-tt-dependency-kinds-')
  const file = join(root, 'src', 'main.tt')
  const model = join(root, 'src', 'model.ts')
  const outside = join(dirname(root), 'shared', 'types.ts')
  const listed = join(root, 'src')
  const fake = await dependencyCompiler(root, [model, outside], [root, listed])
  const id = `${file}?lang.ts`

  // Rollup, Rolldown, Vite's build and Farm: `addWatchFile` takes both.
  const rollup = context()
  await unpluginFactory({ compiler: fake }, { framework: 'rollup' }).load.call(rollup, id)
  assert.deepEqual(rollup.watched, [file, model, outside, root, listed])

  // webpack and Rspack: a directory is a context dependency.
  for (const framework of ['webpack', 'rspack']) {
    const contextDependencies = []
    const webpack = {
      ...context(),
      getNativeBuildContext: () => ({ framework, loaderContext: { addContextDependency: (directory) => contextDependencies.push(directory) } }),
    }
    await unpluginFactory({ compiler: fake }, { framework }).load.call(webpack, id)
    assert.deepEqual(webpack.watched, [file, model, outside], framework)
    assert.deepEqual(contextDependencies, [root, listed], framework)
  }

  // Vite's dev server resolves what `addWatchFile` names as an import of
  // the module, so a dependency goes to its watcher, and only when the
  // watcher does not already cover it.
  const added = []
  const plugin = unpluginFactory({ compiler: fake }, { framework: 'vite' })
  plugin.vite.configureServer({ config: { root }, watcher: { add: (paths) => added.push(...paths) } })
  const dev = context()
  await plugin.load.call(dev, id)
  assert.deepEqual(dev.watched, [file])
  assert.deepEqual(added, [outside])
})

test('an entry added to or removed from a listed directory invalidates the module', async () => {
  const root = testDir('unplugin-tt-dependency-directories-')
  const file = join(root, 'src', 'main.tt')
  const listed = join(root, 'src')
  const fake = await dependencyCompiler(root, [], [listed])
  const plugin = unpluginFactory({ compiler: fake }, { framework: 'vite' })
  const invalidated = []
  const module = { id: `${file}?lang.ts` }
  const graph = { getModuleById: (key) => (key === module.id ? module : undefined), invalidateModule: (found) => invalidated.push(found) }
  plugin.vite.configureServer({ config: { root }, watcher: { add() {} }, environments: { client: { moduleGraph: graph } } })
  await plugin.load.call(context(), module.id)

  plugin.watchChange(join(listed, 'other.ts'), { event: 'update' })
  plugin.watchChange(join(listed, 'nested', 'added.ts'), { event: 'create' })
  assert.deepEqual(invalidated, [])
  plugin.watchChange(join(listed, 'added.ts'), { event: 'create' })
  plugin.watchChange(join(listed, 'removed.ts'), { event: 'delete' })
  assert.deepEqual(invalidated, [module, module])
})
