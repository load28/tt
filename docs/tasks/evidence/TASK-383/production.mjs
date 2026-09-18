import fs from 'node:fs';import path from 'node:path';import {spawnSync} from 'node:child_process';
const root=process.cwd(),base=path.join(root,'target/audit-383/production');fs.mkdirSync(base,{recursive:true});const compiler=path.join(root,'target/debug/ttc');const results=[];
const config={compilerOptions:{strict:true,target:'ES2022',module:'preserve',moduleResolution:'bundler',jsx:'react-jsx',noEmit:true,allowImportingTsExtensions:true,lib:['ES2022','DOM','ESNext.Disposable'],skipLibCheck:true},include:['src/**/*']};
function command(exe,args,cwd){const p=spawnSync(exe,args,{cwd,encoding:'utf8',timeout:30000});return {exit:p.status,signal:p.signal,error:p.error?.message,stdout:p.stdout,stderr:p.stderr};}
function project(name,files){const dir=path.join(base,name);fs.mkdirSync(path.join(dir,'src'),{recursive:true});fs.writeFileSync(path.join(dir,'tsconfig.json'),JSON.stringify(config));for(const [file,text] of Object.entries(files)){const p=path.join(dir,'src',file);fs.mkdirSync(path.dirname(p),{recursive:true});fs.writeFileSync(p,'export {};\n'+text);}return dir;}
function audit(name,files,expected){const dir=project(name,files);const typed=command(compiler,['--check-types','src'],dir);const build=command(compiler,['src','-o','out','--no-banner'],dir);const run=build.exit===0&&expected!==undefined?command('bun',['out/main.ts'],dir):null;results.push({name,typed,build,run,expected,matches:run?run.exit===0&&run.stdout.trim()===JSON.stringify(expected):undefined});fs.writeFileSync(path.join(base,'results.json'),JSON.stringify(results,null,2));}
const service=`import * as R from '@tt/std/result';
import type { TResult } from '@tt/std';
export type User = {id: number; name: string};
export type Failure = 'missing' | 'credit';
export const events: string[] = [];
export async function findUser(id: number): Promise<TResult<User, Failure>> {
 events.push('find:'+id);await Promise.resolve();return id===0?R.Err('missing'):R.Ok({id,name:' user '+id+' '});
}
export async function charge(user: User): Promise<TResult<number, Failure>> {events.push('charge:'+user.id);await Promise.resolve();return user.id===2?R.Err('credit'):R.Ok(user.id*10);}
`;
audit('async-order-service',{'service.tt':service,'main.tt':`import * as R from '@tt/std/result';import {findUser,charge,events} from './service.tt';
async function place(id:number){
 return result {
  await using tx={async [Symbol.asyncDispose](){events.push('dispose:'+id);}};
  const user=try await findUser(id);
  const amount=try await charge(user);
  return {id:user.id,amount,name:user.name |> .trim()};
 };
}
const values=[];for(const id of [1,0,2])values.push(await place(id));console.log(JSON.stringify({values,events}));
`},{values:[{kind:'Ok',value:{id:1,amount:10,name:'user 1'}},{kind:'Err',error:'missing'},{kind:'Err',error:'credit'}],events:['find:1','charge:1','dispose:1','find:0','dispose:0','find:2','charge:2','dispose:2']});
audit('async-parallel-service',{'service.tt':service,'main.tt':`import {findUser,charge} from './service.tt';
const rows=await Promise.all([1,0,2].map(async id=>result {
 const user=try await findUser(id);const total=try await charge(user);return {id:user.id,total};
}));
console.log(JSON.stringify(rows));
`},[{kind:'Ok',value:{id:1,total:10}},{kind:'Err',error:'missing'},{kind:'Err',error:'credit'}]);
audit('transaction-finally-override',{'main.tt':`import * as R from '@tt/std/result';const events:string[]=[];
function transact(fail:boolean){return result {
 try {const value=try(fail?R.Err('rollback'):R.Ok(2));events.push('work');return value;}
 finally {events.push('cleanup');if(fail)return 7;}
};}
console.log(JSON.stringify({values:[transact(false),transact(true)],events}));
`},{values:[{kind:'Ok',value:2},{kind:'Ok',value:7}],events:['work','cleanup','cleanup']});
audit('async-short-circuit',{'main.tt':`const events:string[]=[];async function flag(v:boolean){events.push('flag');return v;}async function load(){events.push('load');return 5;}
async function run(enabled:boolean){return (await flag(enabled))&&match(await load()){5=>{await Promise.resolve();events.push('arm');return 10;},_=>0};}
console.log(JSON.stringify({values:[await run(false),await run(true)],events}));
`},{values:[false,10],events:['flag','flag','load','arm']});
audit('result-in-generator-batch',{'main.tt':`import * as R from '@tt/std/result';const events:number[]=[];
function* batches(){for(const id of [1,0,2]){yield result {const n=try(id?R.Ok(id):R.Err('skip'));events.push(n);return n*10;};}}
console.log(JSON.stringify({values:[...batches()],events}));
`},{values:[{kind:'Ok',value:10},{kind:'Err',error:'skip'},{kind:'Ok',value:20}],events:[1,2]});
const header=`type User={id:string};type Handler<T>={kind:'handler';run:(input:T)=>string};declare const flag:boolean;declare const users:User[];declare function consume<T>(x:Handler<T>):Handler<T>;declare function register<K extends string>(name:K, handler:Handler<User>):{name:K;handler:Handler<User>};\n`;
const left=`({kind:'handler',run:input=>input.id})`,right=`({kind:'handler',run:input=>input.id.toUpperCase()})`;
const contexts={
 'satisfies-registry':`const registry={get:VALUE} satisfies Record<string,Handler<User>>;`,
 'generic-call':`const h=consume<User>(VALUE);`,
 'generic-inferred-key':`const route=register('get',VALUE);const name:'get'=route.name;`,
 'promise-all':`const tasks:Promise<Handler<User>>[]= [Promise.resolve<Handler<User>>(VALUE)];`,
 'mapped-callback':`const handlers=users.map<Handler<User>>(user=>VALUE);`,
 'readonly-tuple':`const handlers:readonly [Handler<User>,Handler<User>]=[VALUE,VALUE];`,
 'computed-assignment':`const handlers:Record<string,Handler<User>>={};declare const key:string;handlers[key]=VALUE;`,
 'async-return':`async function create():Promise<Handler<User>>{return VALUE;}`,
 'closure-return':`const create:()=>Handler<User>=()=>VALUE;`,
 'nullish-context':`declare const cached:Handler<User>|undefined;const handler:Handler<User>=cached??VALUE;`,
 'nested-map':`const handlers:Handler<User>[]=users.flatMap(user=>[VALUE]);`,
 'destructured-context':`const {handler}:{handler:Handler<User>}={handler:VALUE};`,
};
for(const [name,context] of Object.entries(contexts)){
 for(const [flavor,value] of [['oracle',`(flag?${left}:${right})`],['tt',`match(flag){true=>${left},false=>${right}}`]])audit(`inference-${name}-${flavor}`,{'main.tt':header+context.replaceAll('VALUE',value)});
}
console.log(JSON.stringify(results.map(({name,typed,build,run,matches})=>({name,typeExit:typed.exit,typeError:typed.stderr,buildExit:build.exit,buildError:build.stderr,runExit:run?.exit,matches,runtimeError:run?.stderr,actual:run?.stdout})),null,2));
