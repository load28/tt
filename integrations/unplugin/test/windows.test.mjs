import assert from 'node:assert/strict'
import { execFile } from 'node:child_process'
import { chmod, writeFile } from 'node:fs/promises'
import { join } from 'node:path'
import test from 'node:test'

import { testDir } from '../../../scripts/test-dirs.cjs'
import { pathToFileURL } from 'node:url'
import { promisify } from 'node:util'

const run = promisify(execFile)
const plugin = pathToFileURL(new URL('../index.js', import.meta.url).pathname).href

const win32Path = `
import path from "node:path";
const w = path.win32;
export default w;
export const { resolve, dirname, isAbsolute, join, sep, relative, basename, extname, normalize } = w;
`

const hooks = `
export async function resolve(specifier, context, next) {
  if (specifier === "node:path" && context.parentURL === ${JSON.stringify(plugin)}) {
    return { url: "data:text/javascript," + encodeURIComponent(${JSON.stringify(win32Path)}), shortCircuit: true };
  }
  return next(specifier, context);
}
`

const compiler = `#!/usr/bin/env node
import { createInterface } from "node:readline";
const args = process.argv.slice(2);
if (args[0] === "--emit-std") {
  process.stdout.write("export const module = " + JSON.stringify(args[1]) + ";\\n");
} else {
  createInterface({ input: process.stdin }).on("line", (line) => {
    const { id, method, params } = JSON.parse(line);
    const result = method === "dependencies"
      ? { paths: [params.path, "C:\\\\proj\\\\src\\\\model.tt"] }
      : { code: "export const compiled = true;\\n", messages: [] };
    process.stdout.write(JSON.stringify({ id, result }) + "\\n");
  });
}
`

const scenario = (compilerPath) => `
import { register } from "node:module";
register("data:text/javascript," + encodeURIComponent(${JSON.stringify(hooks)}));
process.cwd = () => "C:\\\\proj";
const { unpluginFactory } = await import(${JSON.stringify(plugin)});
const normalizePath = (id) => id.replace(/\\\\/g, "/");
const plugin = unpluginFactory({ compiler: ${JSON.stringify(compilerPath)} });
const context = { addWatchFile() {}, error(message) { throw new Error(message); } };
const types = normalizePath(plugin.resolveId("@tt/std", "C:/proj/src/main.ts"));
const option = normalizePath(plugin.resolveId("./option.js", types));
const std = await plugin.load.call(context, option);
const entry = normalizePath(plugin.resolveId("./main.tt", "C:\\\\proj\\\\src\\\\index.ts"));
await plugin.load.call(context, entry);
const invalidated = [];
const graph = { getModuleById: (id) => (id === entry ? { id } : undefined), invalidateModule: (module) => invalidated.push(module.id) };
plugin.vite.configureServer({ environments: { client: { moduleGraph: graph } } });
plugin.watchChange("C:/proj/src/model.tt");
console.log(JSON.stringify({ types, option, std: std?.code ?? null, resolvedAgain: plugin.resolveId(types), invalidated }));
`

test('standard module ids and dependency paths survive Vite path normalization on Windows', async () => {
  const root = testDir('unplugin-tt-win32-')
  const fake = join(root, 'ttc.mjs')
  await writeFile(fake, compiler)
  await chmod(fake, 0o755)
  const { stdout } = await run(process.execPath, ['--input-type=module', '-e', scenario(fake)])
  const result = JSON.parse(stdout)
  assert.equal(result.option, 'virtual:unplugin-tt/std/option.ts')
  assert.equal(result.resolvedAgain, result.types)
  assert.equal(result.std, 'export const module = "option";\n')
  assert.deepEqual(result.invalidated, [result.invalidated[0]])
  assert.match(result.invalidated[0], /^C:\/proj\/src\/main\.tt/)
})
