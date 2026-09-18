import fs from 'node:fs';import path from 'node:path';import {spawnSync} from 'node:child_process';
const repo=process.cwd(),base=path.join(repo,'target/audit-383/include-patterns-confirmed'),compiler=path.join(repo,'target/release/ttc');fs.mkdirSync(base,{recursive:true});const results=[];
for(const ext of ['tt','ttx']){
 const dir=path.join(base,ext);fs.mkdirSync(path.join(dir,'src'),{recursive:true});fs.writeFileSync(path.join(dir,'src/main.'+ext),'export const count: number = "wrong";');
 const config={compilerOptions:{strict:true,module:'preserve',moduleResolution:'bundler',noEmit:true,jsx:'preserve'}};
 const run=(exe,args)=>{const p=spawnSync(exe,args,{cwd:dir,encoding:'utf8',timeout:30000,env:{...process.env,TTC_BINARY:compiler}});return {exit:p.status,stdout:p.stdout,stderr:p.stderr};};
 for(const include of [['src'],['src/**/*.'+ext],['src/main.'+ext],['src/**/*.'+(ext==='tt'?'ts':'tsx')]]){
  fs.writeFileSync(path.join(dir,'tsconfig.json'),JSON.stringify({...config,include}));results.push({ext,include,result:run(compiler,['--check-types','src/main.'+ext])});
 }
 const packageDir=path.join(dir,'node_modules/@openload28');fs.mkdirSync(packageDir,{recursive:true});if(!fs.existsSync(path.join(packageDir,'tt-lang')))fs.symlinkSync(path.join(repo,'npm/tt-lang'),path.join(packageDir,'tt-lang'));
 fs.writeFileSync(path.join(dir,'tsconfig.native.json'),JSON.stringify({...config,include:['src/**/*.'+ext],contentMappers:[{package:'@openload28/tt-lang',extensions:['.tt','.ttx']}]}));results.push({ext,nativeControl:run(path.join(repo,'node_modules/.bin/tsc'),['--project','tsconfig.native.json','--runExternalCode'])});
}
fs.writeFileSync(path.join(base,'results.json'),JSON.stringify(results,null,2));console.log(JSON.stringify(results,null,2));
