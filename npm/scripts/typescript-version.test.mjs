/* Which TypeScript, for whom.
 *
 * ttc drives the TypeScript a project installed and looks nowhere else
 * (`src/typescript/toolchain.rs`), so a version is not a detail: what the
 * documentation tells a reader to install is what their editor and build
 * will run. Two audiences want different things from that, and this file is
 * where the difference is stated rather than left to whoever edits next
 * (TASK-256):
 *
 * - **This repository** pins an exact nightly. CI has to compare the same
 *   program from one week to the next, which a moving version cannot do.
 * - **Published instructions** say `typescript@next`. An exact nightly
 *   written into a README is correct on the day it is written and stale
 *   after it, and nobody is going to bump seven documents every night.
 *
 * What both must avoid is `typescript@7`: npm ranges do not match
 * prereleases, so it resolves to the 7.0 line, whose API client cannot emit
 * declarations — `ttc --types` and the editor's sidecars go quiet with no
 * hint that a version chose that. */
import assert from 'node:assert/strict'
import { readFile, readdir } from 'node:fs/promises'
import { join } from 'node:path'
import test from 'node:test'

const ROOT = new URL('../../', import.meta.url)
const root = (path) => new URL(path, ROOT)

const manifest = JSON.parse(await readFile(root('package.json'), 'utf8'))
const PIN = manifest.devDependencies.typescript

/** The spec published instructions must use — a tag, so it does not go
 * stale, and one that resolves to the 7.1 line. */
const PUBLISHED = 'next'

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

test('the project initializer scaffolds the published spec', async () => {
  // A scaffolded project is a user's project: it gets what the instructions
  // say, not the nightly this repository happened to pin.
  const installer = await readFile(root('packages/create-tt/src/installer.js'), 'utf8')
  const literal = installer.match(/typescript:\s*['"]([^'"]+)['"]/)
  assert.ok(literal, 'the initializer names no TypeScript version')
  assert.equal(literal[1], PUBLISHED)
})

test('every published install command uses the tag, never a range or a nightly', async () => {
  for (const document of DOCUMENTS) {
    const text = await readFile(root(document), 'utf8')
    for (const spec of specs(text)) {
      assert.equal(
        spec,
        PUBLISHED,
        `${document} tells the reader to install typescript@${spec}; ` +
          `published instructions use typescript@${PUBLISHED} ` +
          `(a range resolves to 7.0, an exact nightly goes stale)`,
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
    assert.equal(spec, PUBLISHED, 'run `bun run highlight` in website/')
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
