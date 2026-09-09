/* One TypeScript version, named in one place.
 *
 * ttc drives the TypeScript a project installed and looks nowhere else
 * (`src/typescript/toolchain.rs`), so a version is not a detail: what the
 * documentation tells a reader to install is what their editor and build
 * will run. Everyone therefore gets the same exact prerelease this
 * repository tests against — a user runs the compiler against the
 * TypeScript it was verified with, and an upstream nightly published
 * tonight cannot change their build tomorrow (TASK-256).
 *
 * What nothing may say is `typescript@7`: npm ranges do not match
 * prereleases, so it resolves to the 7.0 line, whose API client cannot emit
 * declarations — `ttc --types` and the editor's sidecars go quiet with no
 * hint that a version chose that. When 7.1 is released, the pin below moves
 * to `7` and these tests fail until every copy follows. */
import assert from 'node:assert/strict'
import { readFile, readdir } from 'node:fs/promises'
import { join } from 'node:path'
import test from 'node:test'

const ROOT = new URL('../../', import.meta.url)
const root = (path) => new URL(path, ROOT)

const manifest = JSON.parse(await readFile(root('package.json'), 'utf8'))
const PIN = manifest.devDependencies.typescript

/** Files that tell a reader what to install. */
const DOCUMENTS = [
  'README.md',
  'README.ko.md',
  'CONTRIBUTING.md',
  'docs/getting-started.md',
  'docs/getting-started.ko.md',
  'npm/tt-lang/README.md',
  'editors/vscode/README.md',
  'website/src/content.json',
]

/**
 * Every `typescript@<spec>` a reader would **copy** — the fenced code blocks
 * of a Markdown file, or the `code` fields of the website's content. Prose
 * is exempt on purpose: a sentence explaining why `typescript@7` is the
 * wrong thing to install has to be able to write it.
 */
function specs(text) {
  const copyable = [
    ...[...text.matchAll(/```[^\n]*\n([\s\S]*?)```/g)].map((m) => m[1]),
    ...[...text.matchAll(/"code":\s*"((?:[^"\\]|\\.)*)"/g)].map((m) => m[1]),
  ].join('\n')
  return [...copyable.matchAll(/typescript@([\w.\-^~]+)/g)].map((m) => m[1])
}

test('the repository pins an exact TypeScript version', () => {
  // A range would let two machines resolve different compilers from the same
  // lockfile-less install, which is what pinning exists to prevent.
  assert.match(PIN, /^\d+\.\d+\.\d+(-[\w.]+)?$/, `not an exact version: ${PIN}`)
})

test('the installed TypeScript is the pinned one', async () => {
  const installed = JSON.parse(
    await readFile(root('node_modules/typescript/package.json'), 'utf8'),
  )
  assert.equal(installed.version, PIN, 'run npm ci')
})

test('the pinned TypeScript can emit declarations', async () => {
  // `ttc --types` and the editor's sidecars ask the API client for
  // declaration emit; a pin without it turns those features off for everyone
  // who follows our own instructions.
  const api = await readFile(root('node_modules/typescript/dist/api/sync/api.js'), 'utf8')
  assert.ok(
    api.includes('getDeclarationEmit'),
    `${PIN} has no declaration-emit API — that arrived in TypeScript 7.1`,
  )
})

test('the project initializer scaffolds the pinned version', async () => {
  // A scaffolded project gets what the instructions say, which is what this
  // repository tests against.
  const installer = await readFile(root('packages/create-tt/src/installer.js'), 'utf8')
  const literal = installer.match(/typescript:\s*['"]([^'"]+)['"]/)
  assert.ok(literal, 'the initializer names no TypeScript version')
  assert.equal(literal[1], PIN)
})

test('every published install command names the pinned version', async () => {
  for (const document of DOCUMENTS) {
    const text = await readFile(root(document), 'utf8')
    for (const spec of specs(text)) {
      assert.equal(
        spec,
        PIN,
        `${document} tells the reader to install typescript@${spec}, ` +
          `but this repository is tested against ${PIN} ` +
          `(and a \`7\` range would resolve to 7.0, which cannot emit declarations)`,
      )
    }
  }
})

test('the compact AI contract names the pinned TypeScript install', async () => {
  const guide = await readFile(root('docs/ai/tt.md'), 'utf8')
  assert.match(guide, new RegExp(`typescript@${PIN.replaceAll('.', '\\.')}`))
  assert.doesNotMatch(guide, /so add `typescript@7`/)
})

test('the scripts print the pinned version', async () => {
  // `scripts/setup` ends by telling the reader what a consuming project
  // installs; that is a command someone copies, so it is held like any
  // other. `scripts/ci` names a *different* TypeScript on purpose — the
  // stable major the integration tests compile ttc's output with — so only
  // the 7.x line is checked here.
  for (const script of ['scripts/setup', 'scripts/doctor', 'scripts/ci']) {
    const text = await readFile(root(script), 'utf8')
    for (const spec of [...text.matchAll(/typescript@(7[\w.\-^~]*)/g)].map((m) => m[1])) {
      assert.equal(spec, PIN, `${script} prints typescript@${spec}`)
    }
  }
})

test('the generated website code blocks match their source', async () => {
  // `website/src/highlighted-sections.json` is generated from content.json;
  // a stale regeneration would show the old command on the site.
  const highlighted = await readFile(root('website/src/highlighted-sections.json'), 'utf8')
  // Tags become spaces, not nothing: the highlighter wraps each token, so
  // stripping them to empty would fuse a version with the next command.
  const plain = highlighted.replace(/<[^>]*>/g, ' ')
  for (const spec of [...plain.matchAll(/typescript@([\w.\-^~]+)/g)].map((m) => m[1])) {
    assert.equal(spec, PIN, 'run `bun run highlight` in website/')
  }
})

test('no document is missed: the list covers what mentions an install', async () => {
  // A new guide that names a version has to be held to the pin too, so the
  // list above cannot silently fall behind the docs directory.
  const guides = (await readdir(root('docs'), { withFileTypes: true }))
    .filter((entry) => entry.isFile() && entry.name.endsWith('.md'))
    .map((entry) => join('docs', entry.name))
  for (const guide of guides) {
    const text = await readFile(root(guide), 'utf8')
    if (specs(text).length === 0) continue
    assert.ok(
      DOCUMENTS.includes(guide),
      `${guide} names a TypeScript version but is not in DOCUMENTS`,
    )
  }
})
