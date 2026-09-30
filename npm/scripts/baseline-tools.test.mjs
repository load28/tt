import assert from 'node:assert/strict'
import { spawnSync } from 'node:child_process'
import { chmodSync, cpSync, existsSync, mkdirSync, readdirSync, writeFileSync } from 'node:fs'
import { join, resolve } from 'node:path'
import test from 'node:test'

import { testDir } from '../../scripts/test-dirs.cjs'

const repository = resolve(import.meta.dirname, '../..')

function scratchBaselines() {
  const root = testDir('tt-baseline-diff-')
  mkdirSync(join(root, 'scripts'))
  cpSync(join(repository, 'scripts/baseline-diff'), join(root, 'scripts/baseline-diff'))
  mkdirSync(join(root, 'tests/baselines/reference'), { recursive: true })
  mkdirSync(join(root, 'tests/baselines/local'), { recursive: true })
  writeFileSync(join(root, 'tests/baselines/reference/a case.ts'), 'old\n')
  writeFileSync(join(root, 'tests/baselines/local/a case.ts'), 'new\n')
  return root
}

function baselineDiff(root, cwd, diff) {
  return spawnSync(process.execPath, [join(root, 'scripts/baseline-diff')], {
    cwd,
    env: { ...process.env, DIFF: diff },
    encoding: 'utf8',
  })
}

test('DIFF runs from the repository root, with its own arguments, from any directory', () => {
  const root = scratchBaselines()
  for (const diff of ['diff', 'diff -u']) {
    const run = baselineDiff(root, join(root, 'tests'), diff)
    assert.equal(run.status, 0, run.stderr)
    assert.doesNotMatch(run.stderr, /No such file or directory/)
    assert.match(run.stdout, /^(< |-)old$/m, diff)
    assert.match(run.stdout, /^(> |\+)new$/m, diff)
  }
})

test('a DIFF that cannot be run is an error, not an empty diff', () => {
  const root = scratchBaselines()
  const run = baselineDiff(root, root, 'no-such-diff-tool-xyz')
  assert.notEqual(run.status, 0)
  assert.match(run.stderr, /cannot run DIFF=no-such-diff-tool-xyz/)
  assert.doesNotMatch(run.stdout, /baseline\(s\) differ/)
})

function git(cwd, ...args) {
  const result = spawnSync('git', args, { cwd, encoding: 'utf8' })
  assert.equal(result.status, 0, `git ${args.join(' ')}: ${result.stderr}`)
  return result.stdout.trim()
}

function deltaRepository(failBase) {
  const scratch = testDir('tt-diagnostic-delta-')
  const root = join(scratch, 'repo')
  for (const dir of [
    'scripts',
    'tests/fixtures/practical-diagnostics/one',
    'tests/fixtures/mixed-source-matrix',
    'tests/fixtures/mixed-source-runtime',
    'tests/baselines/reference',
    'website/src',
    'packages/create-tt/src',
  ]) {
    mkdirSync(join(root, dir), { recursive: true })
  }
  cpSync(join(repository, 'scripts/diagnostic-delta'), join(root, 'scripts/diagnostic-delta'))
  cpSync(join(repository, 'packages/create-tt/src/installer.js'), join(root, 'packages/create-tt/src/installer.js'))
  cpSync(join(repository, 'packages/create-tt/package.json'), join(root, 'packages/create-tt/package.json'))
  writeFileSync(join(root, 'website/src/content.json'), '{"topics":{}}\n')
  writeFileSync(join(root, 'tests/fixtures/practical-diagnostics/one/main.tt'), 'export const a = 1;\n')
  writeFileSync(join(root, 'tests/fixtures/mixed-source-matrix/main.tt'), 'export const a = 1;\n')
  writeFileSync(join(root, 'tests/fixtures/mixed-source-runtime/main.tt'), 'export const a = 1;\n')
  writeFileSync(join(root, 'tests/baselines/reference/case.ts'), 'base\n')
  writeFileSync(join(root, '.gitignore'), '.tt-dev/\ntarget/\n')
  git(root, 'init', '-q')
  git(root, 'add', '.')
  git(root, '-c', 'user.name=t', '-c', 'user.email=t@t', 'commit', '-q', '-m', 'base')
  git(root, 'branch', 'delta-base')
  writeFileSync(join(root, 'README.md'), 'head\n')
  git(root, 'add', '.')
  git(root, '-c', 'user.name=t', '-c', 'user.email=t@t', 'commit', '-q', '-m', 'head')

  const bin = join(scratch, 'bin')
  mkdirSync(bin)
  writeFileSync(join(bin, 'cargo'), `#!/bin/sh
case "$PWD" in
  *delta-base*) side=base; ${failBase ? 'exit 1' : ':'};;
  *) side=head;;
esac
mkdir -p target/debug
printf '#!/bin/sh\\necho "%s"\\n' "$side" > target/debug/ttc
chmod +x target/debug/ttc
`)
  chmodSync(join(bin, 'cargo'), 0o755)
  const delta = () =>
    spawnSync(process.execPath, ['scripts/diagnostic-delta', '--base', 'delta-base'], {
      cwd: root,
      env: { ...process.env, PATH: `${bin}:${process.env.PATH}` },
      encoding: 'utf8',
    })
  return { root, delta }
}

test('diagnostic-delta leaves no worktree or build behind when a build fails', { skip: process.platform === 'win32' }, () => {
  const { root, delta } = deltaRepository(true)
  const run = delta()
  assert.equal(run.status, 1, run.stdout + run.stderr)
  assert.match(run.stderr, /cargo build --bin ttc .*exited with 1/)
  assert.doesNotMatch(git(root, 'worktree', 'list'), /delta-base/)
  assert.ok(!existsSync(join(root, '.tt-dev/delta-base')))
  assert.ok(!existsSync(join(root, '.tt-dev/delta')) || readdirSync(join(root, '.tt-dev/delta')).length === 0)
})

test('diagnostic-delta counts the baselines of the working tree it builds', { skip: process.platform === 'win32' }, () => {
  const { root, delta } = deltaRepository(false)
  const unpinned = delta()
  assert.equal(unpinned.status, 1, unpinned.stdout + unpinned.stderr)
  assert.match(unpinned.stdout, /the change touches no baseline/, unpinned.stderr)
  writeFileSync(join(root, 'tests/baselines/reference/case.ts'), 'head\n')
  const pinned = delta()
  assert.equal(pinned.status, 0, pinned.stdout + pinned.stderr)
  assert.match(pinned.stdout, /1 baseline file\(s\) changed/)
  writeFileSync(join(root, 'tests/baselines/reference/new.ts'), 'head\n')
  assert.match(delta().stdout, /2 baseline file\(s\) changed/)
})
