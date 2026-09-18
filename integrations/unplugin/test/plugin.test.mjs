import assert from 'node:assert/strict'
import { mkdtemp, realpath, rm, writeFile } from 'node:fs/promises'
import { tmpdir } from 'node:os'
import { dirname, join } from 'node:path'
import test from 'node:test'

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

test('the shared hooks resolve and compile tt, ttx, and standard modules', async (t) => {
  assert.ok(compiler, 'TTC_BINARY must name the compiler under test')
  const root = await mkdtemp(join(tmpdir(), 'unplugin-tt-'))
  t.after(() => rm(root, { recursive: true, force: true }))
  const importer = join(root, 'entry.ts')
  const tt = join(root, 'shape.tt')
  const ttx = join(root, 'view.ttx')
  await writeFile(tt, 'export variant Shape { Circle(radius: number), Point }\n')
  await writeFile(ttx, 'export const View = () => <main>tt</main>;\n')

  const plugin = unpluginFactory({ compiler, sourcemap: true })
  const ttId = plugin.resolveId('./shape.tt', importer)
  const ttxId = plugin.resolveId('./view.ttx', importer)
  assert.equal(ttId, `${tt}.ts`)
  assert.equal(ttxId, `${ttx}.tsx`)

  const ttContext = context()
  const compiledTt = await plugin.load.call(ttContext, ttId)
  assert.match(compiledTt.code, /export type Shape/)
  assert.equal(compiledTt.map.sources[0], 'shape.tt')
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

test('sourcemap false is a working public option and diagnostics reach the host', async (t) => {
  assert.ok(compiler, 'TTC_BINARY must name the compiler under test')
  const root = await mkdtemp(join(tmpdir(), 'unplugin-tt-errors-'))
  t.after(() => rm(root, { recursive: true, force: true }))
  const file = join(root, 'bad.tt')
  await writeFile(
    file,
    'variant State { Ready, Empty }\ndeclare const state: State;\nexport const value = match (state) { Ready => 1 };\n',
  )

  const plugin = unpluginFactory({ compiler, sourcemap: false })
  await assert.rejects(
    plugin.load.call(context(), `${file}.ts`),
    /error\[match-not-exhaustive\]/,
  )

  await writeFile(file, 'export const value = 1;\n')
  const compiled = await plugin.load.call(context(), `${file}.ts`)
  assert.equal(compiled.map, null)
  assert.doesNotMatch(compiled.code, /sourceMappingURL/)
})

test('bare tt specifiers use host package exports and preserve external decisions', async () => {
  const plugin = unpluginFactory({ compiler })
  const requests = []
  const host = { async resolve(...args) { requests.push(args); return { id: '/workspace/packages/domain/model.tt', meta: { host: true } } } }
  const result = await plugin.resolveId.call(host, '@acme/domain/model.tt', '/workspace/app/main.tt.ts')
  assert.equal(result.id, '/workspace/packages/domain/model.tt.ts')
  assert.deepEqual(result.meta, { host: true })
  assert.deepEqual(requests, [['@acme/domain/model.tt', '/workspace/app/main.tt.ts', { skipSelf: true }]])
  const external = { id: '@acme/domain/model.tt', external: true }
  assert.equal(await plugin.resolveId.call({ resolve: async () => external }, external.id, '/app/main.tt.ts'), external)
  const javascript = { id: '/workspace/packages/domain/model.js' }
  assert.equal(await plugin.resolveId.call({ resolve: async () => javascript }, external.id, '/app/main.tt.ts'), javascript)
})

test('type-only dependencies invalidate cached modules even with HMR disabled', async t => {
  const root = await mkdtemp(join(tmpdir(), 'unplugin-tt-watch-'))
  t.after(() => rm(root, { recursive: true, force: true }))
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
  plugin.vite.configureServer({ config: { server: { hmr: false } }, environments: { client: { moduleGraph: graph } } })
  await writeFile(model, 'export variant State { Ready(value: number), Empty, Loading }')
  plugin.watchChange(dependency)
  assert.deepEqual(invalidated, [module])
  await assert.rejects(() => plugin.load.call(context(), id), /missing.*Loading/)
})
