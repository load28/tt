const assert = require('node:assert/strict');
const fs = require('node:fs');
const path = require('node:path');
const vscode = require('vscode');

const pause = ms => new Promise(resolve => setTimeout(resolve, ms));
async function eventually(label, predicate) {
  const deadline = Date.now() + 12000;
  let last;
  do {
    try { await predicate(); return; } catch (error) { last = error; }
    await pause(100);
  } while (Date.now() < deadline);
  throw new Error(`${label}: ${last?.message}`);
}
const errors = doc => vscode.languages.getDiagnostics(doc.uri).filter(d => d.severity === vscode.DiagnosticSeverity.Error);
const hasCode = (doc, code) => errors(doc).some(d => String(typeof d.code === 'object' ? d.code.value : d.code).replace(/^ts/, '') === String(code));

exports.run = async () => {
  const root = process.env.TT_EDITOR_TEST_WORKSPACE;
  const results = [];
  async function check(name, action) {
    try { await action(); results.push({ name, passed: true }); }
    catch (error) { results.push({ name, passed: false, error: error.stack }); }
    fs.writeFileSync(path.join(root, '..', 'results.json'), JSON.stringify(results, null, 2));
  }
  async function open(name, source) {
    const file = path.join(root, name);
    fs.writeFileSync(file, source);
    const doc = await vscode.workspace.openTextDocument(file);
    await vscode.window.showTextDocument(doc);
    return doc;
  }
  async function completion(doc, label) {
    await eventually(`completion ${label}`, async () => {
      const position = doc.positionAt(doc.getText().lastIndexOf('value.') + 6);
      const list = await vscode.commands.executeCommand('vscode.executeCompletionItemProvider', doc.uri, position);
      assert.ok(list?.items.some(item => (typeof item.label === 'string' ? item.label : item.label.label) === label));
    });
  }
  for (const consumer of ['tt', 'ttx']) {
    for (const provider of ['tt', 'ttx', 'ts', 'tsx']) {
      await check(`external ${provider} change updates ${consumer}`, async () => {
        const dependency = path.join(root, `disk-${consumer}-${provider}.${provider}`);
        fs.writeFileSync(dependency, 'export const value: string = "disk";\n');
        const doc = await open(`use-${consumer}-${provider}.${consumer}`, `import { value } from './${path.basename(dependency)}';\nconst result: string = value;\nvalue.toUpperCase();\n`);
        await completion(doc, 'toUpperCase');
        // The dependency is never opened as an editor buffer.
        fs.writeFileSync(dependency, 'export const value: number = 42;\n');
        await eventually('external type mismatch', () => assert.ok(hasCode(doc, 2322), JSON.stringify(errors(doc))));
        await completion(doc, 'toFixed');
        fs.writeFileSync(dependency, 'export const value: string = "restored";\n');
        await eventually('external repair', () => assert.equal(errors(doc).length, 0));
      });
    }
    await check(`module create/delete/recreate updates ${consumer}`, async () => {
      const dependency = path.join(root, `new-${consumer}.ts`);
      const doc = await open(`new-user.${consumer}`, `import { value } from './${path.basename(dependency)}';\nconst result: string = value;\nvalue.toUpperCase();\n`);
      await eventually('missing module', () => assert.ok(hasCode(doc, 2307), JSON.stringify(errors(doc))));
      fs.writeFileSync(dependency, 'export const value: string = "created";\n');
      await eventually('created module', () => assert.equal(errors(doc).length, 0));
      await completion(doc, 'toUpperCase');
      fs.unlinkSync(dependency);
      await eventually('deleted module', () => assert.ok(hasCode(doc, 2307), JSON.stringify(errors(doc))));
      fs.writeFileSync(dependency, 'export const value: string = "recreated";\n');
      await eventually('recreated module', () => assert.equal(errors(doc).length, 0));
    });
  }
  await check('tsconfig edits refresh diagnostics without losing unsaved text', async () => {
    const configPath = path.join(root, 'tsconfig.json');
    const config = JSON.parse(fs.readFileSync(configPath, 'utf8'));
    config.compilerOptions.noImplicitAny = false;
    fs.writeFileSync(configPath, JSON.stringify(config));
    const doc = await open('config.tt', 'export const value = "hello";\nvalue.toUpperCase();\n');
    await completion(doc, 'toUpperCase');
    const edit = new vscode.WorkspaceEdit();
    edit.insert(doc.uri, doc.positionAt(doc.getText().length), 'export function identity(input) { return input; }\n');
    assert.equal(await vscode.workspace.applyEdit(edit), true);
    const text = doc.getText();
    config.compilerOptions.noImplicitAny = true;
    fs.writeFileSync(configPath, JSON.stringify(config));
    await eventually('new config error', () => assert.ok(hasCode(doc, 7006), JSON.stringify(errors(doc))));
    config.compilerOptions.noImplicitAny = false;
    fs.writeFileSync(configPath, JSON.stringify(config));
    await eventually('relaxed config clears error', () => assert.equal(errors(doc).length, 0));
    assert.equal(doc.getText(), text);
    assert.equal(doc.isDirty, true);
  });
  assert.equal(results.filter(result => !result.passed).length, 0, JSON.stringify(results));
};
