const fs=require('node:fs');const path=require('node:path');const vscode=require('vscode');const pause=ms=>new Promise(r=>setTimeout(r,ms));
exports.run=async()=>{
const root=process.env.TT_EDITOR_TEST_WORKSPACE,results=[];
for(const [name,source] of [
 ['exhaustive-jsx','import * as React from "react";\ndeclare namespace JSX { interface IntrinsicElements { p: {children?:unknown} } }\ntype State={kind:"A"}|{kind:"B"}|{kind:"C"};declare const state:State;const view=match(state){A=><p>A</p>,B=><p>B</p>,C=><p>C</p>};'],
 ['generic-variant','export variant Box<T> { Value(value:T), Empty }\n']]){
 const file=path.join(root,name+'.ttx');fs.writeFileSync(file,source.replace('import * as React from "react";\n',''));
 const doc=await vscode.workspace.openTextDocument(file);await vscode.window.showTextDocument(doc);await pause(3000);
 const diagnostics=vscode.languages.getDiagnostics(doc.uri);results.push({name,passed:diagnostics.length===0,diagnostics});
 fs.writeFileSync(path.join(root,'..','results.json'),JSON.stringify(results,null,2));
}
};
