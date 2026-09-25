#!/usr/bin/env node
'use strict'

const { runCli } = require('./index.js')

try {
  runCli(['burin-map', ...process.argv.slice(2)])
} catch (e) {
  console.error(e && e.message ? e.message : e)
  process.exit(1)
}
