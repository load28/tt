#!/usr/bin/env node

import { run } from '../src/installer.js'

run(process.argv.slice(2)).catch((error) => {
  console.error(`@load28/create-tt: ${error.message}`)
  process.exitCode = 1
})
