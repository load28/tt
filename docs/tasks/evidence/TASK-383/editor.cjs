const fs = require('node:fs');const path=require('node:path');const vscode=require('vscode');
const pause=ms=>new Promise(r=>setTimeout(r,ms));
exports.run=async()=>{
 const root=process.env.TT_EDITOR_TEST_WORKSPACE;const results=[];
 for(const ext of ['tt','ttx']) for(const host of ['mts','cts']){
  const provider=path.join(root,`dep-${ext}.${host}`);const consumer=path.join(root,`main-${host}.${ext}`);
  fs.writeFileSync(provider,'export const value: string = "ok";\n');fs.writeFileSync(consumer,`import { value } from "./dep-${ext}.${host}";\nexport const check: string = value;\nvalue.toUpperCase();\n`);
  const doc=await vscode.workspace.openTextDocument(consumer);await vscode.window.showTextDocument(doc);
  const pos=doc.positionAt(doc.getText().lastIndexOf('value.to'));
  let hover;for(let i=0;i<50;i++){hover=await vscode.commands.executeCommand('vscode.executeHoverProvider',doc.uri,pos);if(JSON.stringify(hover).includes('string'))break;await pause(100);}
  await pause(1200);const before=vscode.languages.getDiagnostics(doc.uri);
  fs.writeFileSync(provider,'export const value: number = 42;\n');await pause(2500);
  const after=vscode.languages.getDiagnostics(doc.uri);
  const config=path.join(root,'tsconfig.json');fs.writeFileSync(config,fs.readFileSync(config,'utf8'));
  for(let i=0;i<50;i++){if(vscode.languages.getDiagnostics(doc.uri).some(d=>String(typeof d.code==='object'?d.code.value:d.code).includes('2322')))break;await pause(100);}
  const afterConfigRefresh=vscode.languages.getDiagnostics(doc.uri);
  results.push({name:`${host} disk edit refreshes ${ext}`,passed:after.some(d=>String(typeof d.code==='object'?d.code.value:d.code).includes('2322')),hover,before,after,afterConfigRefresh});
  fs.writeFileSync(path.join(root,'..','results.json'),JSON.stringify(results,null,2));
 }
};
