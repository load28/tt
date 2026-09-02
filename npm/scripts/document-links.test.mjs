import assert from 'node:assert/strict'
import { execFileSync } from 'node:child_process'
import { existsSync, readFileSync } from 'node:fs'
import { dirname, resolve } from 'node:path'
import test from 'node:test'

const root = resolve(import.meta.dirname, '../..')

function localTarget(raw) {
  const target = raw.trim().replace(/^<|>$/g, '')
  if (/^[a-z][a-z0-9+.-]*:/i.test(target)) return null
  const [beforeFragment, fragment = ''] = target.split('#', 2)
  const path = beforeFragment.split('?', 1)[0]
  return { path: decodeURIComponent(path), fragment: decodeURIComponent(fragment) }
}

function anchors(source) {
  const found = new Set()
  const counts = new Map()
  for (const line of source.split('\n')) {
    const heading = /^(?: {0,3})#{1,6}\s+(.+?)\s*#*\s*$/.exec(line)?.[1]
    if (heading === undefined) continue
    const base = heading
      .replace(/\[([^\]]+)\]\([^)]*\)/g, '$1')
      .replace(/<[^>]+>/g, '')
      .replace(/`/g, '')
      .toLowerCase()
      .replace(/[^\p{Letter}\p{Number}\p{Mark}\s_-]/gu, '')
      .replace(/\s/g, '-')
    const count = counts.get(base) ?? 0
    counts.set(base, count + 1)
    found.add(count === 0 ? base : `${base}-${count}`)
  }
  for (const match of source.matchAll(/<(?:a\s+)?(?:id|name)=["']([^"']+)["']/gi)) {
    found.add(match[1])
  }
  return found
}

test('every tracked Markdown link names an existing local target', () => {
  const files = execFileSync('git', ['ls-files', '--', '*.md'], {
    cwd: root,
    encoding: 'utf8',
  }).trim().split('\n').filter(Boolean)
  const missing = []

  for (const file of files) {
    const source = readFileSync(resolve(root, file), 'utf8')
      .replace(/```[\s\S]*?```/g, '')
      .replace(/`+[^`\n]*`+/g, '')
    const targets = [
      ...source.matchAll(/!?\[[^\]]*\]\(([^\s)]+)(?:\s+['"][^)]*)?\)/g),
      ...source.matchAll(/^\s*\[[^\]]+\]:\s*(\S+)/gm),
    ]
    for (const match of targets) {
      const target = localTarget(match[1])
      if (target === null) continue
      const path = target.path === ''
        ? resolve(root, file)
        : target.path.startsWith('/')
          ? resolve(root, target.path.slice(1))
          : resolve(root, dirname(file), target.path)
      if (!existsSync(path)) {
        missing.push(`${file}: ${match[1]}`)
        continue
      }
      if (target.fragment !== '' && path.endsWith('.md')) {
        const targetSource = readFileSync(path, 'utf8')
        if (!anchors(targetSource).has(target.fragment.toLowerCase())) {
          missing.push(`${file}: ${match[1]} (missing heading)`)
        }
      }
    }
  }

  assert.deepEqual(missing, [])
})
