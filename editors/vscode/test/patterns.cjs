const assert = require('node:assert/strict');
const fs = require('node:fs');
const path = require('node:path');
const vscode = require('vscode');

exports.run = async () => {
  const root = process.env.TT_EDITOR_TEST_WORKSPACE;
  const results = [];
  fs.writeFileSync(path.join(root, 'model.tt'), 'export variant User { Admin(name: string, level: number), Guest }\n');
  const prefix = 'import { User } from "./model.tt";\nvariant Other { Wrong }\ndeclare const user: User;\n';
  for (const extension of ['tt', 'ttx']) {
    const file = path.join(root, `patterns.${extension}`);
    fs.writeFileSync(file, prefix);
    const doc = await vscode.workspace.openTextDocument(file);
    await vscode.window.showTextDocument(doc, { preview: false });
    const complete = async (pattern, expected, trigger) => {
      const offset = prefix.length + pattern.indexOf('#');
      const text = prefix + pattern.replace('#', '');
      const edit = new vscode.WorkspaceEdit();
      edit.replace(doc.uri, new vscode.Range(doc.positionAt(0), doc.positionAt(doc.getText().length)), text);
      assert.equal(await vscode.workspace.applyEdit(edit), true);
      const list = await vscode.commands.executeCommand('vscode.executeCompletionItemProvider', doc.uri, doc.positionAt(offset), trigger);
      const labels = list?.items.map(item => typeof item.label === 'string' ? item.label : item.label.label) ?? [];
      for (const label of expected) assert.ok(labels.includes(label), `${pattern}: missing ${label}; ${JSON.stringify(labels)}`);
      return labels;
    };
    const check = async (name, run) => {
      try { await run(); results.push({ name: `${extension}: ${name}`, passed: true }); }
      catch (error) { results.push({ name: `${extension}: ${name}`, passed: false, error: error.stack }); }
      fs.writeFileSync(path.join(root, '..', 'results.json'), JSON.stringify(results, null, 2));
    };
    await check('first arm retains imported cases at every prefix', async () => {
      await complete('const r = match (user) {# };', ['Admin', 'Guest'], '{');
      for (const part of ['', 'A', 'Ad', 'Admin', 'G', 'Gu', 'Guest']) {
        await complete(`const r = match (user) { ${part}# };`, ['Admin', 'Guest']);
      }
    });
    await check('following arms survive wildcard and incomplete siblings', async () => {
      await complete('const r = match (user) { Admin(name) => name,# };', ['Admin', 'Guest'], ',');
      for (const sibling of ['_ => "other"', 'Unfin', 'Guest => "guest"']) {
        const labels = await complete(`const r = match (user) { Admin(name) => name, Gu#, ${sibling} };`, ['Admin', 'Guest']);
        assert.ok(!labels.includes('Wrong'));
      }
    });
    await check('payload fields appear on delimiters and prefixes', async () => {
      await complete('const r = match (user) { Admin(#) => "x" };', ['name', 'level'], '(');
      await complete('const r = match (user) { Admin(name,#) => name };', ['name', 'level'], ',');
      for (const part of ['n', 'na', 'name', 'name, l', 'name, le']) {
        await complete(`const r = match (user) { Admin(${part}#) => "x" };`, ['name', 'level']);
      }
    });
    await check('tuple slots complete without treating expression calls as slots', async () => {
      for (const part of ['(', '(A', '(Admin(name), ', '(Admin(name), G']) {
        await complete(`const r = match (user, user) { ${part}# };`, ['Admin', 'Guest']);
      }
      const outside = await complete('const object = {# };', [], '{');
      // VS Code may add word-based suggestions independently of tt.
      assert.ok(!outside.includes('Admin') && !outside.includes('Guest'));
    });
  }
  assert.equal(results.filter(result => !result.passed).length, 0, JSON.stringify(results));
};
