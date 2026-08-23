#!/usr/bin/env node

import { run } from '../src/installer.js'

run(process.argv.slice(2)).catch((error) => {
  console.error(`create-rl: ${error.message}`)
  process.exitCode = 1
})
