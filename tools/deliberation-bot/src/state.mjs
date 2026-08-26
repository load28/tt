import { mkdir, readFile, rename, writeFile } from 'node:fs/promises'
import { join } from 'node:path'

export class StateStore {
  constructor(directory) {
    this.directory = directory
    this.processed = new Set()
    this.ready = false
  }

  async initialize() {
    await mkdir(this.directory, { recursive: true })
    try {
      const value = JSON.parse(await readFile(join(this.directory, 'processed.json'), 'utf8'))
      this.processed = new Set(value)
    } catch (error) {
      if (error.code !== 'ENOENT') throw error
    }
    this.ready = true
  }

  has(deliveryId) {
    this.assertReady()
    return this.processed.has(deliveryId)
  }

  async mark(deliveryId) {
    this.assertReady()
    this.processed.add(deliveryId)
    const values = [...this.processed].slice(-10_000)
    const temporary = join(this.directory, `processed-${process.pid}.tmp`)
    await writeFile(temporary, `${JSON.stringify(values, null, 2)}\n`)
    await rename(temporary, join(this.directory, 'processed.json'))
  }

  async forget(deliveryId) {
    this.assertReady()
    if (!this.processed.delete(deliveryId)) return
    const temporary = join(this.directory, `processed-${process.pid}.tmp`)
    await writeFile(temporary, `${JSON.stringify([...this.processed], null, 2)}\n`)
    await rename(temporary, join(this.directory, 'processed.json'))
  }

  async recordSession(subjectId, session) {
    this.assertReady()
    const safeId = subjectId.replaceAll(/[^a-zA-Z0-9_-]/g, '_')
    await writeFile(
      join(this.directory, `${safeId}-${Date.now()}.json`),
      `${JSON.stringify(session, null, 2)}\n`,
    )
  }

  assertReady() {
    if (!this.ready) throw new Error('StateStore.initialize() must be called first')
  }
}
