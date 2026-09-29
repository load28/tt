import assert from 'node:assert/strict'
import { execFile } from 'node:child_process'
import { chmod, readFile, writeFile, mkdir } from 'node:fs/promises'
import { join } from 'node:path'
import test from 'node:test'
import { promisify } from 'node:util'

import { testDir } from '../../../scripts/test-dirs.cjs'

import { unpluginFactory } from '../index.js'

const run = promisify(execFile)
const compiler = process.env.TTC_BINARY

function context() {
  return { addWatchFile() {}, error(message) { throw new Error(message) } }
}

/** A compiler that logs each start and exit of the real one it runs. */
async function loggingCompiler(root) {
  const log = join(root, 'starts.log')
  const wrapper = join(root, 'ttc.mjs')
  await writeFile(wrapper, `#!/usr/bin/env node
import { spawn } from "node:child_process";
import { appendFileSync } from "node:fs";
const args = process.argv.slice(2);
appendFileSync(${JSON.stringify(log)}, "start " + args.join(" ") + "\\n");
spawn(${JSON.stringify(compiler)}, args, { stdio: "inherit" }).on("exit", (code) => {
  appendFileSync(${JSON.stringify(log)}, "exit\\n");
  process.exit(code ?? 1);
});
`)
  await chmod(wrapper, 0o755)
  return { wrapper, log: async () => (await readFile(log, 'utf8').catch(() => '')).split('\n').filter(Boolean) }
}

async function until(condition) {
  for (let tries = 0; tries < 400 && !(await condition()); tries++) await new Promise((done) => setTimeout(done, 25))
  assert.ok(await condition(), 'the condition never held')
}

test('one server compiles every module to exactly what ttc -p prints', async () => {
  assert.ok(compiler, 'TTC_BINARY must name the compiler under test')
  const root = testDir('unplugin-tt-server-')
  const src = join(root, 'src')
  await mkdir(src)
  await writeFile(join(root, 'tsconfig.json'), JSON.stringify({ compilerOptions: { strict: true, target: 'esnext', module: 'esnext', moduleResolution: 'bundler', noEmit: true, jsx: 'preserve' }, include: ['src'] }))
  const files = []
  for (let i = 0; i < 3; i++) {
    const file = join(src, `m${i}.tt`)
    const previous = i === 0 ? '' : `import { State${i - 1} } from "./m${i - 1}.tt";\nexport const before = (s: State${i - 1}) => match (s) { Ready(value) => value, Empty => 0 };\n`
    await writeFile(file, `${previous}export variant State${i} { Ready(value: number), Empty }\nexport const values = (s: State${i}) => match (s) { Ready(value) => [value], Empty => [] };\n`)
    files.push(file)
  }
  const view = join(src, 'view.ttx')
  await writeFile(view, 'variant Tone { Loud, Quiet }\r\nexport const view = (tone: Tone) => <main>{match (tone) { Loud => <b>!</b>, Quiet => null }}</main>;\r\n')
  files.push(view)

  const { wrapper, log } = await loggingCompiler(root)
  const plain = unpluginFactory({ compiler: wrapper, sourcemap: false })
  const mapped = unpluginFactory({ compiler: wrapper, sourcemap: true })
  const loaded = await Promise.all(files.map((file) => plain.load.call(context(), plain.resolveId(file))))
  const withMaps = await Promise.all(files.map((file) => mapped.load.call(context(), mapped.resolveId(file))))

  for (const [index, file] of files.entries()) {
    const { stdout } = await run(compiler, ['-p', '--rewrite-imports', 'off', file])
    assert.equal(loaded[index].code, stdout, file)
    const inline = await run(compiler, ['-p', '--rewrite-imports', 'off', '--source-map', 'inline', file])
    assert.ok(inline.stdout.startsWith(withMaps[index].code), file)
    assert.match(inline.stdout.slice(withMaps[index].code.length), /^\/\/# sourceMappingURL=data:/, file)
    assert.deepEqual(withMaps[index].map.sources, [file])
  }
  assert.deepEqual((await log()).filter((line) => line.startsWith('start')), ['start --server', 'start --server'])

  mapped.closeBundle.call({ meta: { watchMode: true } })
  plain.closeBundle.call({ meta: { watchMode: false } })
  await until(async () => (await log()).filter((line) => line === 'exit').length === 1)
  mapped.closeWatcher()
  await until(async () => (await log()).filter((line) => line === 'exit').length === 2)
})

test('a server that stops is started once more, then reported', async () => {
  const root = testDir('unplugin-tt-server-crash-')
  const file = join(root, 'main.tt')
  await writeFile(file, 'export const a = 1;\n')
  const marker = join(root, 'crashed')
  const fake = join(root, 'ttc.mjs')
  await writeFile(fake, `#!/usr/bin/env node
import { existsSync, writeFileSync } from "node:fs";
import { createInterface } from "node:readline";
const always = process.env.TT_TEST_ALWAYS_CRASH === "1";
createInterface({ input: process.stdin }).on("line", (line) => {
  if (always || !existsSync(${JSON.stringify(marker)})) {
    writeFileSync(${JSON.stringify(marker)}, "");
    process.stderr.write("thread 'main' has overflowed its stack\\n");
    process.exit(134);
  }
  const { id, method, params } = JSON.parse(line);
  const result = method === "dependencies" ? { paths: [params.path] } : { code: "export const a = 1;\\n", messages: [] };
  process.stdout.write(JSON.stringify({ id, result }) + "\\n");
});
`)
  await chmod(fake, 0o755)

  const recovered = await unpluginFactory({ compiler: fake, sourcemap: false }).load.call(context(), `${file}?lang.ts`)
  assert.equal(recovered.code, 'export const a = 1;\n')

  process.env.TT_TEST_ALWAYS_CRASH = '1'
  try {
    await assert.rejects(
      unpluginFactory({ compiler: fake }).load.call(context(), `${file}?lang.ts`),
      /stopped twice.*exited with code 134[\s\S]*overflowed its stack/,
    )
  } finally {
    delete process.env.TT_TEST_ALWAYS_CRASH
  }
})

test('a compiler that cannot start is reported, not replaced', async () => {
  const root = testDir('unplugin-tt-server-missing-')
  const file = join(root, 'main.tt')
  await writeFile(file, 'export const a = 1;\n')
  await assert.rejects(
    unpluginFactory({ compiler: join(root, 'no-such-ttc') }).load.call(context(), `${file}?lang.ts`),
    /cannot run .*no-such-ttc/,
  )
})
