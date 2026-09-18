import { mkdtemp, writeFile } from 'node:fs/promises'
import { tmpdir } from 'node:os'
import { join, resolve } from 'node:path'
import { spawnSync } from 'node:child_process'

const compiler = resolve('target/debug/ttc')
const root = await mkdtemp(resolve('target/audit-383/composition-'))
const prelude = `import * as R from "@tt/std/result";
import * as O from "@tt/std/option";
variant E { A(value: number), B }
const take = (...values: unknown[]) => values;
const receiver = { m: (...values: unknown[]) => values };
class Box { constructor(...values: unknown[]) {} }
`

const expressions = {
  plain: n => `${n}`,
  pipe: n => `(${n} |> (x => x + 1))`,
  optional: n => `({ value: ${n} } |> ?.value)`,
  tagMatch: n => `match (E.A(${n})) { A(value) => value, B => 0 }`,
  literalMatch: n => `match (${n}) { 1 => 1, _ => 0 }`,
  isMatch: n => `match (new Error(String(${n}))) { is Error { message } => message.length, _ => 0 }`,
  result: n => `result { const value = try R.Ok(${n}); return value; }`,
  flow: n => `(flow |> ((x: number) => x + 1))(${n})`,
  ifLetOwner: n => `(() => { if let A(value) = E.A(${n}) { return value; } return 0; })()`,
  letElseOwner: n => `(() => { const A(value) = E.A(${n}) else { return 0; }; return value; })()`,
  tryOwner: n => `(() => { const value = try R.Ok(${n}); return R.Ok(value); })()`,
}

const hosts = {
  array: (a, b) => `[${a}, ${b}]`,
  args: (a, b) => `take(${a}, ${b})`,
  object: (a, b) => `({ first: ${a}, second: ${b} })`,
  template: (a, b) => `\`${'${'}${a}}:${'${'}${b}}\``,
  sequence: (a, b) => `(${a}, ${b})`,
  binary: (a, b) => `Number(${a}) + Number(${b})`,
  computed: (a, b) => `({ [String(${a})]: ${b} })`,
  optionalCall: (a, b) => `receiver?.m(${a}, ${b})`,
  construct: (a, b) => `new Box(${a}, ${b})`,
  logical: (a, b) => `(${a}) && (${b})`,
  conditional: (a, b) => `true ? (${a}) : (${b})`,
}

const jsxHosts = {
  attributes: (a, b) => `<main data-a={${a}} data-b={${b}} />`,
  children: (a, b) => `<main>{${a}}{${b}}</main>`,
  attrChild: (a, b) => `<main data-a={${a}}>{${b}}</main>`,
  siblings: (a, b) => `<> <i>{${a}}</i> <b>{${b}}</b> </>`,
}

const failures = []
async function check(kind, hostName, leftName, rightName, source) {
  const extension = kind === 'tsx' ? 'ttx' : 'tt'
  const file = join(root, `${kind}-${hostName}-${leftName}-${rightName}.${extension}`)
  await writeFile(file, `${prelude}\nexport const answer = ${source};\n`)
  const out = spawnSync(compiler, ['-p', '--no-banner', file], { encoding: 'utf8' })
  if (out.status !== 0) failures.push({ kind, hostName, leftName, rightName, status: out.status, stderr: out.stderr.trim().split('\n').slice(0, 3).join(' | ') })
}

for (const [hostName, host] of Object.entries(hosts)) {
  for (const [leftName, left] of Object.entries(expressions)) {
    for (const [rightName, right] of Object.entries(expressions)) {
      await check('ts', hostName, leftName, rightName, host(left(1), right(1)))
    }
  }
}
for (const [hostName, host] of Object.entries(jsxHosts)) {
  for (const [leftName, left] of Object.entries(expressions)) {
    for (const [rightName, right] of Object.entries(expressions)) {
      await check('tsx', hostName, leftName, rightName, host(left(1), right(1)))
    }
  }
}

const grouped = Map.groupBy(failures, f => f.stderr)
console.log(JSON.stringify({ cases: Object.keys(hosts).length * Object.keys(expressions).length ** 2 + Object.keys(jsxHosts).length * Object.keys(expressions).length ** 2, failures: failures.length, groups: [...grouped].map(([error, entries]) => ({ count: entries.length, error, examples: entries.slice(0, 8).map(({stderr, ...x}) => x) })) }, null, 2))
