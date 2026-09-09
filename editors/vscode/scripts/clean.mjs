import { rm } from 'node:fs/promises'
import { dirname, join, resolve } from 'node:path'
import { fileURLToPath } from 'node:url'

const root = resolve(dirname(fileURLToPath(import.meta.url)), '..')

// TypeScript build mode does not delete JavaScript for source files that no
// longer exist. Remove only the two declared output trees and their build
// metadata so every compile is an exact projection of the current sources.
for (const project of ['client', 'server']) {
  await rm(join(root, project, 'out'), { recursive: true, force: true })
  await rm(join(root, project, 'tsconfig.tsbuildinfo'), { force: true })
}
