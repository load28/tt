import { access, mkdir, writeFile } from 'node:fs/promises'
import { homedir } from 'node:os'
import { dirname, join, resolve } from 'node:path'
import { fileURLToPath } from 'node:url'

const toolDirectory = resolve(dirname(fileURLToPath(import.meta.url)), '..')
const repositoryRoot = resolve(toolDirectory, '../..')
const runtimeDirectory = join(repositoryRoot, '.deliberation-bot')
const launchAgentsDirectory = join(homedir(), 'Library', 'LaunchAgents')
const nodeBinary = process.execPath
const path = [
  join(homedir(), '.local', 'bin'),
  join(homedir(), '.asdf', 'shims'),
  '/opt/homebrew/bin',
  '/usr/local/bin',
  '/usr/bin',
  '/bin',
].join(':')

await access(join(runtimeDirectory, 'config.json'))
await mkdir(launchAgentsDirectory, { recursive: true })
await mkdir(runtimeDirectory, { recursive: true })

const services = [
  {
    label: 'dev.load28.tt-deliberation-bot',
    arguments: [nodeBinary, join(toolDirectory, 'src', 'index.mjs')],
    environment: {
      PATH: path,
      TT_DELIBERATION_CONFIG: join(runtimeDirectory, 'config.json'),
    },
    stdout: join(runtimeDirectory, 'bot.log'),
    stderr: join(runtimeDirectory, 'bot-error.log'),
  },
]

for (const service of services) {
  const target = join(launchAgentsDirectory, `${service.label}.plist`)
  await writeFile(target, plist(service), { mode: 0o600 })
  process.stdout.write(`${target}\n`)
}

function plist(service) {
  const argumentsXml = service.arguments.map((value) => `      <string>${escapeXml(value)}</string>`).join('\n')
  const environmentXml = Object.entries(service.environment)
    .map(([key, value]) => `      <key>${escapeXml(key)}</key>\n      <string>${escapeXml(value)}</string>`)
    .join('\n')
  return `<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>Label</key>
  <string>${escapeXml(service.label)}</string>
  <key>ProgramArguments</key>
  <array>
${argumentsXml}
  </array>
  <key>EnvironmentVariables</key>
  <dict>
${environmentXml}
  </dict>
  <key>WorkingDirectory</key>
  <string>${escapeXml(toolDirectory)}</string>
  <key>RunAtLoad</key>
  <true/>
  <key>KeepAlive</key>
  <true/>
  <key>ThrottleInterval</key>
  <integer>10</integer>
  <key>StandardOutPath</key>
  <string>${escapeXml(service.stdout)}</string>
  <key>StandardErrorPath</key>
  <string>${escapeXml(service.stderr)}</string>
</dict>
</plist>
`
}

function escapeXml(value) {
  return value
    .replaceAll('&', '&amp;')
    .replaceAll('<', '&lt;')
    .replaceAll('>', '&gt;')
    .replaceAll('"', '&quot;')
    .replaceAll("'", '&apos;')
}
