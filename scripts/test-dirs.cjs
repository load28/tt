'use strict'

const fs = require('node:fs')
const os = require('node:os')
const path = require('node:path')
const { after } = require('node:test')

const repository = path.resolve(__dirname, '..')
const runs = new Map()

function run(base) {
  let dir = runs.get(base)
  if (dir === undefined) {
    fs.mkdirSync(base, { recursive: true })
    dir = fs.mkdtempSync(path.join(base, `tt-test-run-${process.pid}-`))
    runs.set(base, dir)
  }
  return dir
}

function testDir(prefix) {
  return fs.mkdtempSync(path.join(run(os.tmpdir()), prefix))
}

function repoTestDir(prefix) {
  return fs.mkdtempSync(path.join(run(path.join(repository, 'target', 'tt-tests')), prefix))
}

after(() => {
  for (const dir of runs.values()) {
    fs.rmSync(dir, { recursive: true, force: true, maxRetries: 10 })
  }
  runs.clear()
})

module.exports = { testDir, repoTestDir }
