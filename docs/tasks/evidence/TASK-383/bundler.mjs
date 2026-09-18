import fs from 'node:fs';import path from 'node:path';import {pathToFileURL} from 'node:url';import {spawnSync} from 'node:child_process';
const repo=process.cwd();const {createServer,build}=await import(pathToFileURL(path.join(repo,'website/node_modules/vite/dist/node/index.js')));const {default:tt}=await import(pathToFileURL(path.join(repo,'integrations/unplugin/vite.js')));
const compiler=path.join(repo,'target/debug/ttc'),base=path.join(repo,'target/audit-383/bundler');fs.mkdirSync(base,{recursive:true});const results=[];const pause=ms=>new Promise(r=>setTimeout(r,ms));
function fixture(name,files){const root=path.join(base,name);fs.mkdirSync(root,{recursive:true});for(const [name,text]of Object.entries(files)){const p=path.join(root,name);fs.mkdirSync(path.dirname(p),{recursive:true});fs.writeFileSync(p,text);}return root;}
function save(r){results.push(r);fs.writeFileSync(path.join(base,'results.json'),JSON.stringify(results,null,2));}
const bundleConfig=root=>({root,configFile:false,logLevel:'silent',plugins:[tt({compiler})],build:{ssr:path.join(root,'src/main.tt'),outDir:'dist',minify:false,rollupOptions:{external:[],output:{entryFileNames:'main.mjs'}}}});
const packageRoot=fixture('workspace-package',{'package.json':'{"type":"module"}','src/main.tt':'import {value} from "@acme/domain/model.tt"; console.log(value);','node_modules/@acme/domain/package.json':'{"name":"@acme/domain","type":"module","exports":{"./model.tt":"./model.tt"}}','node_modules/@acme/domain/model.tt':'export const value=42;'});
try{await build({...bundleConfig(packageRoot),ssr:{noExternal:true}});save({name:'workspace-package-tt-import',passed:true});}catch(e){save({name:'workspace-package-tt-import',passed:false,error:e.message});}
fs.writeFileSync(path.join(packageRoot,'node_modules/@acme/domain/model.ts'),'export const value=42;');
fs.writeFileSync(path.join(packageRoot,'node_modules/@acme/domain/package.json'),'{"name":"@acme/domain","type":"module","exports":{"./model.tt":"./model.tt","./model.ts":"./model.ts"}}');
fs.writeFileSync(path.join(packageRoot,'src/main.tt'),'import {value} from "@acme/domain/model.ts"; console.log(value);');
try{await build({...bundleConfig(packageRoot),ssr:{noExternal:true}});save({name:'workspace-package-ts-control',passed:true});}catch(e){save({name:'workspace-package-ts-control',passed:false,error:e.message});}
fs.writeFileSync(path.join(packageRoot,'src/main.tt'),'import {value} from "@acme/domain/model.tt"; console.log(value);');
const watchRoot=fixture('type-only-hmr',{'package.json':'{"type":"module"}','src/model.tt':'export variant State { Ready(value:number), Empty }','src/main.tt':'import type {State} from "./model.tt"; export function render(s:State){return match(s){Ready(value)=>value,Empty=>0};}','tsconfig.json':'{"compilerOptions":{"strict":true,"module":"preserve","moduleResolution":"bundler","noEmit":true},"include":["src"]}'});
let server;
try{
 server=await createServer({root:watchRoot,configFile:false,logLevel:'silent',plugins:[tt({compiler})],server:{middlewareMode:true,hmr:false,watch:{usePolling:true,interval:100}},appType:'custom'});
 const id='/@fs/'+path.join(watchRoot,'src/main.tt.ts');const first=await server.transformRequest(id);
 await pause(500);let noticed=false;server.watcher.on('change',file=>{if(file===path.join(watchRoot,'src/model.tt'))noticed=true;});
 fs.writeFileSync(path.join(watchRoot,'src/model.tt'),'export variant State { Ready(value:number), Empty, Loading }');await pause(1000);
 let changed;try{const out=await server.transformRequest(id);changed={error:null,code:out?.code,sameCode:out?.code===first?.code};}catch(e){changed={error:e.message};}
 if(changed.code){try{const module=await import('data:text/javascript;base64,'+Buffer.from(changed.code).toString('base64'));changed.runtime=module.render({kind:'Loading'});}catch(e){changed.runtimeError=e.message;}}
 const fresh=spawnSync(compiler,['-p','--rewrite-imports','off','src/main.tt'],{cwd:watchRoot,encoding:'utf8',timeout:30000});
 save({name:'type-only-variant-hmr',noticed,watched:server.watcher.getWatched(),changed,fresh:{exit:fresh.status,stderr:fresh.stderr}});
}catch(e){save({name:'type-only-variant-hmr',error:e.message});}finally{await server?.close();}
console.log(JSON.stringify(results,null,2));
