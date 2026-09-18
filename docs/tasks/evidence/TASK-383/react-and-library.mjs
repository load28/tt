import fs from 'node:fs';import path from 'node:path';import {pathToFileURL} from 'node:url';import {spawnSync} from 'node:child_process';
const repo=process.cwd(),base=path.join(repo,'target/audit-383/apps');fs.mkdirSync(base,{recursive:true});const compiler=path.join(repo,'target/debug/ttc');const {build}=await import(pathToFileURL(path.join(repo,'website/node_modules/vite/dist/node/index.js')));const {default:tt}=await import(pathToFileURL(path.join(repo,'integrations/unplugin/vite.js')));const results=[];
function command(exe,args,cwd){const p=spawnSync(exe,args,{cwd,encoding:'utf8',timeout:30000});return {exit:p.status,stdout:p.stdout,stderr:p.stderr};}
function fixture(name,files){const dir=path.join(base,name);fs.mkdirSync(dir,{recursive:true});for(const [name,text] of Object.entries(files)){const p=path.join(dir,name);fs.mkdirSync(path.dirname(p),{recursive:true});fs.writeFileSync(p,text);}return dir;}
const config={compilerOptions:{strict:true,target:'ES2022',module:'preserve',moduleResolution:'bundler',jsx:'react-jsx',noEmit:true,allowImportingTsExtensions:true,skipLibCheck:true},include:['src']};
const models=`export variant Load<T> { Loading, Loaded(value:T), Failed(message:string) }\nexport type Item = {id:string; title:string; price:number; quantity:number};\n`;
const view=`import {useState} from 'react';import {Load,type Item} from './models.tt';
export function Cart({state,onSelect}:{state:Load<Item[]>;onSelect:(item:Item)=>void}) {
 const [filter,setFilter]=useState('');
 return <section><input value={filter} onChange={event=>setFilter(event.currentTarget.value)}/>{match(state){
 Loading=><p>Loading</p>,Failed(message)=><p role="alert">{message}</p>,Loaded(value)=>{
 const items=value.filter(item=>item.title.includes(filter));
 return <ul>{items.map(item=><li key={item.id}><button onClick={()=>onSelect(item)}>{item.title |> .trim()}</button><strong>{item.price*item.quantity}</strong></li>)}</ul>;
 }}}<footer>{match(state){Loaded(value)=>value.length,_=>0}} items</footer></section>;
}
`;
for(const [flavor,body] of [['safe',view],['summary',view.replace('item.price*item.quantity','Number(match(item.quantity){0=>0,_=>item.quantity}) + Number(match(item.price){0=>0,_=>item.price})')]]){
 const dir=fixture('react-cart-'+flavor,{'package.json':'{"type":"module"}','tsconfig.json':JSON.stringify(config),'src/models.tt':models,'src/Cart.ttx':body,'src/main.ttx':`import {renderToStaticMarkup} from 'react-dom/server';import {Cart} from './Cart.ttx';import {Load} from './models.tt';console.log(renderToStaticMarkup(<Cart state={Load.Loaded([{id:'a',title:' Apple ',price:5,quantity:2}])} onSelect={item=>item.id}/>));`});
 fs.mkdirSync(path.join(dir,'node_modules'),{recursive:true});for(const name of ['react','react-dom','@types']){const link=path.join(dir,'node_modules',name);if(!fs.existsSync(link))fs.symlinkSync(path.join(repo,'website/node_modules',name),link);}
 const typed=command(compiler,['--check-types','src'],dir);let bundled;try{await build({root:dir,configFile:false,logLevel:'silent',plugins:[tt({compiler})],build:{ssr:path.join(dir,'src/main.ttx'),outDir:'dist',minify:false,rollupOptions:{output:{entryFileNames:'main.mjs'}}}});bundled={passed:true,run:command('bun',['dist/main.mjs'],dir)};}catch(e){bundled={passed:false,error:e.message};}
 results.push({name:'react-cart-'+flavor,typed,bundled});
}
const dir=fixture('relocatable-library',{'tsconfig.json':JSON.stringify(config),'src/api.tt':`export type User={id:string};export type Handler={kind:'handler';run:(u:User)=>string};export function register(handler:Handler){return handler;}\n`,'src/main.tt':`import {register} from './api.tt';declare const flag:boolean;export const handler=register(match(flag){true=>({kind:'handler',run:user=>user.id}),false=>({kind:'handler',run:user=>user.id.toUpperCase()})});\n`});
const typed=command(compiler,['--check-types','src'],dir),built=command(compiler,['src','-o','out'],dir);let emitted='',standalone;
if(built.exit===0){emitted=fs.readFileSync(path.join(dir,'out/main.ts'),'utf8');fs.renameSync(path.join(dir,'src'),path.join(dir,'source-hidden'));fs.writeFileSync(path.join(dir,'out/tsconfig.json'),JSON.stringify({...config,include:['*.ts'],compilerOptions:{...config.compilerOptions,allowImportingTsExtensions:false}}));standalone=command(path.join(repo,'node_modules/.bin/tsc'),['--project','out/tsconfig.json'],dir);fs.renameSync(path.join(dir,'source-hidden'),path.join(dir,'src'));}
results.push({name:'relocatable-library',typed,built,emitted,standalone});
fs.writeFileSync(path.join(base,'results.json'),JSON.stringify(results,null,2));console.log(JSON.stringify(results,null,2));
