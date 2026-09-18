import { mkdir, mkdtemp, writeFile } from 'node:fs/promises'
import { tmpdir } from 'node:os'
import { join, resolve } from 'node:path'
import { spawnSync } from 'node:child_process'

const compiler = resolve('target/debug/ttc')
const root = await mkdtemp(resolve('target/task384/runtime-final-'))
const runtime = spawnSync(compiler, ['--emit-std', 'runtime', '--no-banner'], { encoding: 'utf8' })
const runtimeRoot = join(root, 'node_modules/@tt/runtime')
await mkdir(runtimeRoot, { recursive: true })
await writeFile(join(runtimeRoot, 'package.json'), '{"name":"@tt/runtime","type":"module","exports":"./index.ts"}\n')
await writeFile(join(runtimeRoot, 'index.ts'), runtime.stdout)

await mkdir(join(root, 'tt'), { recursive: true })
await writeFile(join(root, 'tt/runtime.ts'), runtime.stdout)

const prelude = `/** @jsx h */
/** @jsxFrag Fragment */
const trace: string[] = [];
const side = (label: string, value: number) => (trace.push(label), value);
const R = { Ok: <T,>(value: T) => ({ kind: "Ok" as const, value }) };
variant E { A(value: number), B }
const take = (...values: unknown[]) => values;
const receiver = { m: (...values: unknown[]) => values };
class Box { constructor(...values: unknown[]) {} }
const h = (tag: unknown, props: unknown, ...children: unknown[]) => ({ tag, props, children });
const Fragment = Symbol("Fragment");
`
const expressions = {
  plain: label => `side("${label}", 1)`,
  pipe: label => `(side("${label}", 1) |> (x => x + 1))`,
  optional: label => `({ value: side("${label}", 1) } |> ?.value)`,
  tagMatch: label => `match (E.A(side("${label}", 1))) { A(value) => value, B => 0 }`,
  literalMatch: label => `match (side("${label}", 1)) { 1 => 1, _ => 0 }`,
  isMatch: label => `match (new Error(String(side("${label}", 1)))) { is Error { message } => message.length, _ => 0 }`,
  result: label => `result { const value = try R.Ok(side("${label}", 1)); return value; }`,
  flow: label => `(flow |> ((x: number) => x + 1))(side("${label}", 1))`,
  ifLetOwner: label => `(() => { if let A(value) = E.A(side("${label}", 1)) { return value; } return 0; })()`,
  letElseOwner: label => `(() => { const A(value) = E.A(side("${label}", 1)) else { return 0; }; return value; })()`,
  tryOwner: label => `(() => { const value = try R.Ok(side("${label}", 1)); return R.Ok(value); })()`,
}
const hosts = {
  array: (a,b) => `[${a},${b}]`, args: (a,b) => `take(${a},${b})`, object: (a,b) => `({a:${a},b:${b}})`,
  template: (a,b) => `\`${'${'}${a}}:${'${'}${b}}\``, sequence: (a,b) => `(${a},${b})`, binary: (a,b) => `Number(${a})+Number(${b})`,
  computed: (a,b) => `({[String(${a})]:${b}})`, optionalCall: (a,b) => `receiver?.m(${a},${b})`, construct: (a,b) => `new Box(${a},${b})`,
  logical: (a,b) => `(${a})&&(${b})`, conditional: (a,b) => `true?(${a}):(${b})`,
}
const jsxHosts = { attributes:(a,b)=>`<main a={${a}} b={${b}}/>`, children:(a,b)=>`<main>{${a}}{${b}}</main>`, attrChild:(a,b)=>`<main a={${a}}>{${b}}</main>`, siblings:(a,b)=>`<><i>{${a}}</i><b>{${b}}</b></>` }

const shard = Number(process.env.TT_AUDIT_SHARD ?? 0)
const shards = Number(process.env.TT_AUDIT_SHARDS ?? 1)
let cases = 0
let rejected = 0
const rejectedExamples = []
let compiled = 0
const wrong = []
for (const [kind, table] of [['ts', hosts], ['tsx', jsxHosts]]) for (const [hostName, host] of Object.entries(table)) for (const [leftName,left] of Object.entries(expressions)) for (const [rightName,right] of Object.entries(expressions)) {
  if (cases++ % shards !== shard) continue
  if (process.env.TT_AUDIT_KIND && process.env.TT_AUDIT_KIND !== kind) continue
  const ext = kind === 'tsx' ? 'ttx' : 'tt'
  const sourcePath = join(root, `case.${ext}`)
  await writeFile(sourcePath, `${prelude}\nconst answer=${host(left('left'),right('right'))};\nconsole.log(JSON.stringify(trace));\n`)
  const built = spawnSync(compiler, ['-p','--no-banner',sourcePath], { encoding:'utf8', timeout: 15000 })
  if (built.status !== 0) {
    rejected++
    if (!built.stderr.includes('error[match-placement]')) rejectedExamples.push({kind,hostName,leftName,rightName,error:built.stderr.slice(0,500)})
    continue
  }
  compiled++
  const outputPath = join(root, `case.${kind === 'tsx' ? 'tsx' : 'ts'}`)
  await writeFile(outputPath, built.stdout)
  const ran = spawnSync('bun',['--jsx-runtime','classic','--jsx-factory','h','--jsx-fragment','Fragment',outputPath],{encoding:'utf8',cwd:root})
  const expected = hostName === 'conditional' ? ['left'] : ['left','right']
  let actual
  try { actual=JSON.parse(ran.stdout.trim()) } catch { actual=null }
  if (ran.status !== 0 || JSON.stringify(actual)!==JSON.stringify(expected)) wrong.push({kind,hostName,leftName,rightName,status:ran.status,expected,actual,stderr:ran.stderr.trim().slice(0,300)})
}
console.log(JSON.stringify({compiled,rejected,rejectedExamples,wrong:wrong.length,examples:wrong},null,2))
