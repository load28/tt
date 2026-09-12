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

// Exercise native diagnostics without activating the tt language server. The
// ordinary editor matrix covers the same lifecycle with both servers active.
exports.run = async () => {
  const root = process.env.TT_EDITOR_TEST_WORKSPACE;
  const results = [];
  await vscode.extensions.getExtension('TypeScriptTeam.native-preview').activate();
  for (const consumerKind of ['ts', 'tsx']) {
    for (const providerKind of ['ts', 'tsx']) {
      const name = `${providerKind} -> ${consumerKind}: discard refreshes the newly active consumer`;
      try {
        const stem = `${providerKind}-${consumerKind}`;
        const providerPath = path.join(root, `provider-${stem}.${providerKind}`);
        const consumerPath = path.join(root, `consumer-${stem}.${consumerKind}`);
        const original = 'export const value: string = "disk";\n';
        const source = `import { value } from "./provider-${stem}.${providerKind}";\nexport const result: string = value;\nvalue.toUpperCase();\n`;
        fs.writeFileSync(providerPath, original);
        fs.writeFileSync(consumerPath, source);
        const consumer = await vscode.workspace.openTextDocument(consumerPath);
        await vscode.window.showTextDocument(consumer, { viewColumn: vscode.ViewColumn.One, preview: false });
        // Keep a real unsaved consumer overlay through every dependency edit.
        await replace(consumer, source + '// unsaved consumer\n');
        const consumerVersion = consumer.version;
        const provider = await vscode.workspace.openTextDocument(providerPath);
        await vscode.window.showTextDocument(provider, { viewColumn: vscode.ViewColumn.Two, preview: false });
        const expectError = () => eventually('native dependency error', () => {
          assert.ok(errors(consumer).some(d => d.source === 'ts' && String(d.code) === '2322'), JSON.stringify(errors(consumer)));
        });
        await replace(provider, 'export const value: number = 42;\n');
        await expectError();
        await replace(provider, original);
        await eventually('dependency correction', () => assert.deepEqual(errors(consumer), []));
        await replace(provider, 'export const value: number = 42;\n');
        await expectError();
        await vscode.commands.executeCommand('workbench.action.revertAndCloseActiveEditor');
        assert.equal(provider.getText(), original);
        assert.equal(provider.isDirty, false);
        assert.equal(vscode.window.activeTextEditor?.document.uri.toString(), consumer.uri.toString());
        // No edit, save, completion request, or extra focus action is allowed
        // to refresh the consumer on behalf of the diagnostic lifecycle.
        await eventually('discarded dependency clears', () => assert.deepEqual(errors(consumer), []));
        assert.equal(consumer.version, consumerVersion);
        assert.equal(consumer.isDirty, true);
        assert.equal(fs.readFileSync(consumerPath, 'utf8'), source);
        results.push({ name, passed: true });
      } catch (error) {
        results.push({ name, passed: false, error: error.stack });
      }
      fs.writeFileSync(path.join(root, '..', 'results.json'), JSON.stringify(results, null, 2));
    }
  }
  assert.equal(results.filter(result => !result.passed).length, 0, JSON.stringify(results));
};
