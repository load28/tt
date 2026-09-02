import assert from 'node:assert/strict'
import { mkdtemp, rm, writeFile } from 'node:fs/promises'
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
  assert.deepEqual(ttContext.watched, [tt])

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
