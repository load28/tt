/* One TypeScript version, named in one place.
 *
 * ttc drives the TypeScript a project installed and looks nowhere else
 * (`src/typescript/toolchain.rs`), so what the documentation tells a user to
 * install *is* what their editor and build will run. The version this
 * repository tests against therefore has to be the version every install
 * command names — a document that says `typescript@7` while the repository
 * pins a 7.1 prerelease hands users a compiler that cannot emit declarations
 * and leaves them debugging a difference nobody declared (TASK-256).
 *
 * The pin lives in the repository's package.json. This test holds the
 * scaffolder and every published install command to it. */
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

test('the project initializer installs the pinned version', async () => {
  const installer = await readFile(root('packages/create-tt/src/installer.js'), 'utf8')
  const literal = installer.match(/typescript:\s*['"]([^'"]+)['"]/)
  assert.ok(literal, 'the initializer names no TypeScript version')
  assert.equal(literal[1], PIN)
})

test('every install command names the pinned version', async () => {
  for (const document of DOCUMENTS) {
    const text = await readFile(root(document), 'utf8')
    for (const spec of specs(text)) {
      assert.equal(
        spec,
        PIN,
        `${document} tells the reader to install typescript@${spec}, ` +
          `but this repository pins ${PIN}`,
      )
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
