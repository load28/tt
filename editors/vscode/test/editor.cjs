const assert = require('node:assert/strict');
const fs = require('node:fs');
const path = require('node:path');
const vscode = require('vscode');

const pause = ms => new Promise(resolve => setTimeout(resolve, ms));
async function eventually(label, check) {
  const deadline = Date.now() + 15000;
  let last;
  do {
    try { await check(); return; } catch (error) { last = error; }
    await pause(100);
  } while (Date.now() < deadline);
  throw new Error(`${label}: ${last?.message}`);
}
async function replace(doc, text) {
  const edit = new vscode.WorkspaceEdit();
  edit.replace(doc.uri, new vscode.Range(doc.positionAt(0), doc.positionAt(doc.getText().length)), text);
  assert.equal(await vscode.workspace.applyEdit(edit), true);
}
function errors(doc) {
  return vscode.languages.getDiagnostics(doc.uri).filter(d => d.severity === vscode.DiagnosticSeverity.Error);
}
function isTypeMismatch(diagnostic) {
  const code = typeof diagnostic.code === 'object' ? diagnostic.code.value : diagnostic.code;
  return String(code).replace(/^ts/, '') === '2322';
}
exports.run = async () => {
  const root = process.env.TT_EDITOR_TEST_WORKSPACE;
  assert.ok(root);
  const results = [];
  const check = async (name, run) => {
    try { await run(); results.push({ name, passed: true }); }
    catch (error) { results.push({ name, passed: false, error: error.stack }); }
    fs.writeFileSync(path.join(root, '..', 'results.json'), JSON.stringify(results, null, 2));
    console.log(JSON.stringify(results.at(-1)));
  };
  for (const ext of (process.env.VSCODE_TYPESCRIPT_EXTENSION ? ['tt', 'ttx', 'ts', 'tsx'] : ['tt', 'ttx'])) {
    for (const providerExt of ['tt', 'ttx', 'ts', 'tsx']) {
    const label = `${providerExt} -> ${ext}`;
    const providerPath = path.join(root, `provider-${ext}-${providerExt}.${providerExt}`);
    fs.writeFileSync(providerPath, 'export const value: string = "hello";\n');
    const file = path.join(root, `consumer-${ext}-${providerExt}.${ext}`);
    const ttSyntax = ext === 'tt' || ext === 'ttx'
      ? 'variant Status { Ready, Empty }\ndeclare const status: Status;\nconst label = match (status) { Ready => "ready", Empty => "empty" };\n' : '';
    const jsx = ext === 'ttx' || ext === 'tsx'
      ? 'declare global { namespace JSX { interface IntrinsicElements { main: { children?: unknown } } } }\nconst view = <main>{value}</main>;\n' : '';
    const source = `import { value } from "./provider-${ext}-${providerExt}.${providerExt}";\n${ttSyntax}${jsx}const result: string = value;\nvalue.toUpperCase();\n`;
    fs.writeFileSync(file, source);
    const doc = await vscode.workspace.openTextDocument(file);
    await vscode.window.showTextDocument(doc);
    await check(`${label}: activation and completion`, async () => {
      assert.equal(doc.languageId, ({ ts: 'typescript', tsx: 'typescriptreact' })[ext] || ext);
      await eventually('completion', async () => {
        const list = await vscode.commands.executeCommand('vscode.executeCompletionItemProvider', doc.uri, doc.positionAt(source.indexOf('value.to') + 6));
        assert.ok(list?.items.some(item => (typeof item.label === 'string' ? item.label : item.label.label) === 'toUpperCase'));
      });
      const incomplete = source.replace('value.toUpperCase();', 'value.');
      await replace(doc, incomplete);
      await eventually('incomplete-buffer completion', async () => {
        const list = await vscode.commands.executeCommand('vscode.executeCompletionItemProvider', doc.uri, doc.positionAt(incomplete.lastIndexOf('value.') + 6));
        assert.ok(list?.items.some(item => (typeof item.label === 'string' ? item.label : item.label.label) === 'toUpperCase'));
      });
      await replace(doc, source);
    });
    await check(`${label}: cross-file definition and rename`, async () => {
      const position = doc.positionAt(source.lastIndexOf('value.to'));
      await eventually('definition', async () => {
        const locations = await vscode.commands.executeCommand('vscode.executeDefinitionProvider', doc.uri, position);
        assert.ok(locations?.some(location => (location.uri || location.targetUri).fsPath === providerPath), JSON.stringify(locations));
      });
      const edit = await vscode.commands.executeCommand('vscode.executeDocumentRenameProvider', doc.uri, position, 'renamedValue');
      assert.ok(edit?.entries().some(([uri]) => uri.fsPath === file), JSON.stringify(edit?.entries()));
      const originals = [];
      for (const [uri] of edit.entries()) {
        assert.ok(uri.fsPath === file || uri.fsPath === providerPath, `rename leaked a generated path: ${uri}`);
        const target = await vscode.workspace.openTextDocument(uri);
        originals.push([target, target.getText()]);
      }
      assert.equal(await vscode.workspace.applyEdit(edit), true);
      assert.ok(doc.getText().includes('renamedValue.toUpperCase()'));
      for (const [target, text] of originals) await replace(target, text);
    });
    await check(`${label}: introduce and clear unsaved error`, async () => {
      await replace(doc, source.replace('result: string', 'result: number'));
      await eventually('type error', () => {
        const diagnostic = errors(doc).find(isTypeMismatch);
        assert.ok(diagnostic, JSON.stringify(errors(doc)));
        assert.equal(doc.getText(diagnostic.range), ext === 'ts' || ext === 'tsx' ? 'result' : 'value');
      });
      await replace(doc, source);
      await eventually('error clears', () => assert.equal(errors(doc).length, 0));
    });
    await check(`${label}: dependency edits refresh untouched consumer`, async () => {
      const provider = await vscode.workspace.openTextDocument(providerPath);
      await vscode.window.showTextDocument(provider, vscode.ViewColumn.Beside);
      await replace(provider, 'export const value: number = 42;\n');
      await eventually('dependent type error', () => assert.ok(errors(doc).some(isTypeMismatch), JSON.stringify(errors(doc))));
      await eventually('dependent completion', async () => {
        const list = await vscode.commands.executeCommand('vscode.executeCompletionItemProvider', doc.uri, doc.positionAt(source.lastIndexOf('value.to') + 6));
        const labels = list?.items.map(item => typeof item.label === 'string' ? item.label : item.label.label) || [];
        assert.ok(labels.includes('toFixed') && !labels.includes('toUpperCase'), JSON.stringify(labels));
      });
      await replace(provider, 'export const value: string = "hello";\n');
      await eventually('dependent error clears', () => assert.equal(errors(doc).length, 0));
      await replace(provider, 'export const value: number = 42;\n');
      await eventually('dependent error returns', () => assert.ok(errors(doc).some(isTypeMismatch)));
      await vscode.window.showTextDocument(provider);
      await vscode.commands.executeCommand('workbench.action.revertAndCloseActiveEditor');
      await eventually('discarded dependency error clears', () => assert.equal(errors(doc).length, 0));
    });
    }
  }
  assert.equal(results.filter(r => !r.passed).length, 0, JSON.stringify(results));
};
