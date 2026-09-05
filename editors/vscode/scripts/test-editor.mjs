import { spawn } from 'node:child_process'
import { mkdtempSync, mkdirSync, readFileSync, symlinkSync, writeFileSync } from 'node:fs'
import { dirname, join, resolve } from 'node:path'
import { fileURLToPath } from 'node:url'

const extension = resolve(dirname(fileURLToPath(import.meta.url)), '..')
const repo = resolve(extension, '../..')
const base = join(repo, 'target', 'editor-tests')
mkdirSync(base, { recursive: true })
const run = mkdtempSync(join(base, 'run-'))
const workspace = join(run, 'workspace')
const nativeExtension = process.env.VSCODE_TYPESCRIPT_EXTENSION
mkdirSync(join(workspace, '.vscode'), { recursive: true })
writeFileSync(join(workspace, '.vscode', 'settings.json'), JSON.stringify({
  'tt.compilerPath': join(repo, 'target', 'debug', process.platform === 'win32' ? 'ttc.exe' : 'ttc'),
  'tt.sidecar': 'off',
  'security.workspace.trust.enabled': false,
  'js/ts.experimental.useTsgo': true,
  'typescript.experimental.useTsgo': true,
}))
writeFileSync(join(workspace, 'tsconfig.json'), JSON.stringify({
  compilerOptions: { strict: true, target: 'ES2022', module: 'preserve', moduleResolution: 'bundler', jsx: 'preserve', noEmit: true, allowImportingTsExtensions: true },
  include: ['**/*'],
  ...(nativeExtension ? { contentMappers: [{ package: '@openload28/tt-lang', extensions: ['.tt', '.ttx'] }] } : {}),
}))
mkdirSync(join(workspace, 'node_modules', '@openload28'), { recursive: true })
symlinkSync(join(repo, 'npm', 'tt-lang'), join(workspace, 'node_modules', '@openload28', 'tt-lang'), 'junction')
console.log(`Editor test artifacts: ${run}`)
const child = spawn(process.env.VSCODE_EXECUTABLE || 'code', [
  '--new-window', '--skip-welcome', '--skip-release-notes', '--disable-workspace-trust',
  '--user-data-dir', join(run, 'profile'), '--extensions-dir', join(run, 'extensions'),
  `--extensionDevelopmentPath=${extension}`,
  ...(nativeExtension ? [`--extensionDevelopmentPath=${resolve(nativeExtension)}`] : []),
  `--extensionTestsPath=${join(extension, 'test', 'editor.cjs')}`,
  '--wait', workspace,
], { stdio: 'inherit', env: { ...process.env, TTC_BINARY: join(repo, 'target', 'debug', process.platform === 'win32' ? 'ttc.exe' : 'ttc'), TT_EDITOR_TEST_WORKSPACE: workspace } })
child.on('error', error => { console.error(error); process.exitCode = 1 })
child.on('exit', code => {
  try {
    const results = JSON.parse(readFileSync(join(run, 'results.json'), 'utf8'))
    console.log(JSON.stringify(results, null, 2))
    process.exitCode = code === 0 && results.length === (nativeExtension ? 64 : 32) && results.every(result => result.passed) ? 0 : 1
  } catch (error) {
    console.error('Extension host did not produce a complete test report:', error.message)
    process.exitCode = 1
  }
})
