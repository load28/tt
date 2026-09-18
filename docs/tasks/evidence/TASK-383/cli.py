from pathlib import Path
import subprocess,json,time
root=Path.cwd(); base=root/'target/audit-383'; ttc=str(root/'target/debug/ttc');base.mkdir(parents=True,exist_ok=True); findings=[]
def run(args,cwd):
 p=subprocess.run([ttc,*args],cwd=cwd,text=True,capture_output=True,timeout=40);return {'args':args,'exit':p.returncode,'stdout':p.stdout,'stderr':p.stderr}
def project(name):
 p=base/name;p.mkdir(exist_ok=True);(p/'tsconfig.json').write_text(json.dumps({'compilerOptions':{'strict':True,'target':'ES2022','module':'preserve','moduleResolution':'bundler','noEmit':True,'allowImportingTsExtensions':True},'include':['src/**/*']}));(p/'src').mkdir(exist_ok=True);return p
p=project('sidecar');
for folder,typ,val in [('a','number','1'),('b','string','"hello"')]:
 d=p/'src'/folder;d.mkdir(exist_ok=True);(d/'model.tt').write_text(f'export const value: {typ} = {val};\n')
r=run(['--types','src','-o','types'],p);r['files']={str(f.relative_to(p)):f.read_text() for f in (p/'types').rglob('*.ts')};findings.append({'case':'relative-sidecar-collision',**r})
p=project('overwrite');(p/'src/a.tt').write_text('export const value = 1;\n');(p/'src/a.ts').write_text('// authored source\nexport const original = 2;\n');r=run(['src/a.tt'],p);r['after']=(p/'src/a.ts').read_text();findings.append({'case':'overwrite-existing-ts',**r})
def watch(name,change,args=['--check-types','--watch','src']):
 p=project(name);(p/'src/dep.ts').write_text('export const value: string = "ok";\n');(p/'src/main.tt').write_text('import { value } from "./dep";\nconst check: string = value;\n');log=p/'watch.log'
 with log.open('w') as out:
  proc=subprocess.Popen([ttc,*args],cwd=p,stdout=out,stderr=out,text=True)
  try:
   deadline=time.time()+30
   while 'Ctrl-C' not in log.read_text() and time.time()<deadline:time.sleep(.1)
   before=log.read_text();change(p);time.sleep(2);after=log.read_text()
  finally:proc.terminate();proc.wait(timeout=5)
 findings.append({'case':name,'before':before,'after':after,'fresh':run(['--check-types','src'],p)})
watch('watch-ts-dependency',lambda p:(p/'src/dep.ts').write_text('export const value: number = 1;\n'))
watch('watch-config',lambda p:(p/'tsconfig.json').write_text('{"compilerOptions":{"noUnusedLocals":true,"module":"preserve","moduleResolution":"bundler"},"include":["src/**/*"]}'))
(base/'probes.json').write_text(json.dumps(findings,indent=2));print(json.dumps(findings,indent=2))
