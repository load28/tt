const assert = require('node:assert/strict');
const fs = require('node:fs');
const path = require('node:path');
const vscode = require('vscode');

const errors = doc => vscode.languages.getDiagnostics(doc.uri).filter(d => d.severity === vscode.DiagnosticSeverity.Error);
async function eventually(label, check) {
  const deadline = Date.now() + 15000;
  let last;
  do {
    try { await check(); return; } catch (error) { last = error; }
    await new Promise(resolve => setTimeout(resolve, 100));
  } while (Date.now() < deadline);
  throw new Error(`${label}: ${last?.message}`);
}
async function replace(doc, text) {
  const edit = new vscode.WorkspaceEdit();
  edit.replace(doc.uri, new vscode.Range(doc.positionAt(0), doc.positionAt(doc.getText().length)), text);
  assert.equal(await vscode.workspace.applyEdit(edit), true);
}
exports.run = async () => {
  const root = process.env.TT_EDITOR_TEST_WORKSPACE;
  const results = [];
  const check = async (name, run) => {
    try { await run(); results.push({ name, passed: true }); }
    catch (error) { results.push({ name, passed: false, error: error.stack }); }
    fs.writeFileSync(path.join(root, '..', 'results.json'), JSON.stringify(results, null, 2));
  };
  const open = async (name, text) => {
    const file = path.join(root, name);
    fs.writeFileSync(file, text);
    const doc = await vscode.workspace.openTextDocument(file);
    await vscode.window.showTextDocument(doc, { preview: false });
    return doc;
  };
  const native = await vscode.extensions.getExtension('TypeScriptTeam.native-preview').activate();
  await check('native exposes explicit content-mapper feature ownership', () => {
    assert.equal(native.contentMapperFeatureOwnership, true);
  });
  await check('inferred native projects receive the registered mapper manifest', async () => {
    await vscode.extensions.getExtension('tt-lang.tt-language').activate();
    const directory = path.join(root, '..', 'inferred');
    fs.mkdirSync(directory, { recursive: true });
    fs.writeFileSync(path.join(directory, 'model.tt'), 'export variant Status { Ready(value: string), Empty }\n');
    const source = 'import { Status } from "./model.tt";\nStatus.Ready("hello");\n';
    const doc = await open('../inferred/consumer.ts', source);
    await eventually('inferred-project variant completion', async () => {
      const list = await vscode.commands.executeCommand('vscode.executeCompletionItemProvider', doc.uri, doc.positionAt(source.lastIndexOf('Status.') + 7));
      assert.ok(list?.items.some(item => (item.label.label || item.label) === 'Ready'));
    });
  });
  for (const extension of ['tt', 'ttx']) {
    await check(`${extension}: one provider retains incomplete-arm inference and diagnostic transitions`, async () => {
      const source = 'export {};\nvariant User { Admin(name: string), Guest }\ndeclare const user: User;\nconst greeting = match (user) { Admin(name) => name, Guest => "guest" };\nconst message: number = greeting;\n';
      const doc = await open(`typing.${extension}`, source);
      await eventually('one type error', () => {
        assert.equal(errors(doc).length, 1, JSON.stringify(errors(doc)));
        assert.equal(errors(doc)[0].source, 'ttc');
        assert.equal(String(errors(doc)[0].code), 'ts2322');
      });
      const hovers = await vscode.commands.executeCommand('vscode.executeHoverProvider', doc.uri, doc.positionAt(source.indexOf('greeting')));
      assert.equal(hovers.length, 1, JSON.stringify(hovers));
      assert.match(hovers[0].contents.map(c => c.value || c).join('\n'), /string/);
      const partial = source.replace('Guest => "guest"', 'Gue').replace('=> name,', '=> name.,');
      await replace(doc, partial);
      await eventually('one String completion', async () => {
        const list = await vscode.commands.executeCommand('vscode.executeCompletionItemProvider', doc.uri, doc.positionAt(partial.indexOf('name.,') + 5));
        const matches = list.items.filter(item => (item.label.label || item.label) === 'toUpperCase');
        assert.equal(matches.length, 1);
      });
      await replace(doc, source.replace('message: number', 'message: string'));
      await eventually('type error clears', () => assert.deepEqual(errors(doc), []));
    });
  }
  await check('late ownership preserves native consumer synchronization and release restores UI', async () => {
    const registration = native.registerContentMappers('tt-test.native', [{ extensions: ['.owned'] }]);
    let external;
    try {
      const provider = await open('provider.owned', 'export const value: number = "wrong";\n');
      await eventually('native provider error before delegation', () => assert.equal(errors(provider).length, 1, JSON.stringify(vscode.languages.getDiagnostics())));
      const consumer = await open('consumer.ts', 'import { value } from "./provider.owned";\nconst result: string = value;\n');
      await eventually('native provider and consumer errors', () => {
        assert.equal(errors(provider).length, 1);
        assert.equal(errors(consumer).length, 1, JSON.stringify(vscode.languages.getDiagnostics()));
        assert.equal(errors(provider)[0].source, 'ts');
      });
      external = native.registerContentMappers('tt-test.external', [{ extensions: ['.owned'], languageFeatures: 'external' }]);
      await eventually('late delegation removes native UI', () => assert.deepEqual(errors(provider), []));
      await replace(provider, 'export const value: string = 123;\n');
      await eventually('unsaved provider changes still reach native consumer', () => assert.deepEqual(errors(consumer), []));
      external.dispose();
      external = undefined;
      await vscode.window.showTextDocument(provider);
      await eventually('lease release restores native diagnostics', () => {
        assert.equal(errors(provider).length, 1);
        assert.equal(errors(provider)[0].source, 'ts');
      });
    } finally {
      external?.dispose();
      registration.dispose();
    }
  });
  if (results.some(result => !result.passed)) throw new Error('Ownership tests failed; see results.json');
};
